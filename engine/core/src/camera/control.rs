// SPDX-License-Identifier: GPL-3.0-or-later
use super::geometry::*;
use super::S3dCamera;
use crate::stream::{InStream, OutStream};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_8};
use std::sync::{Arc, RwLock, RwLockWriteGuard};

pub const CAMERA_CONTROL_CLASS_ID: u32 = 0xC9C6_28EC;
pub type CameraHandle = Arc<RwLock<S3dCamera>>;

/// Stable, fallibly allocated control storage for the §2.1 lifetime contract.
/// Rust storage replaces the native heap/layout; a borrowed CameraControl never
/// frees itself. A destroyed object cannot be accessed again through this owner.
/// Dropping storage drops Rust handles but never calls the camera's Release.
#[derive(Debug)]
pub struct CameraControlAllocation {
    storage: Vec<CameraControl>,
    destroyed: bool,
}

impl CameraControlAllocation {
    pub fn try_create() -> Result<Self, std::collections::TryReserveError> {
        Self::try_create_with(|storage| storage.try_reserve_exact(1))
    }
    fn try_create_with(
        reserve: impl FnOnce(&mut Vec<CameraControl>) -> Result<(), std::collections::TryReserveError>,
    ) -> Result<Self, std::collections::TryReserveError> {
        let mut storage = Vec::new();
        // Construction and AddRef must not occur on allocation failure.
        reserve(&mut storage)?;
        storage.push(CameraControl::new());
        Ok(Self {
            storage,
            destroyed: false,
        })
    }
    pub fn control(&self) -> &CameraControl {
        assert!(!self.destroyed, "camera control has been destroyed");
        &self.storage[0]
    }
    pub fn control_mut(&mut self) -> &mut CameraControl {
        assert!(!self.destroyed, "camera control has been destroyed");
        &mut self.storage[0]
    }
    pub fn release(&mut self) -> u32 {
        self.release_with(|_, _| {})
    }
    fn release_with(&mut self, before_destructor: impl FnOnce(&CameraControl, u32)) -> u32 {
        let control = self.control_mut();
        if control.references > 1 {
            control.references -= 1;
            control.references
        } else {
            // Slot 6 observes the original 0/1 count, not a decremented ledger.
            before_destructor(control, 1);
            self.deleting_destructor(1);
            0
        }
    }
    /// The returned address is an identity token only; it may already be freed.
    /// Flags other than bit 0 have no effect. No camera Detach/Release occurs.
    pub fn deleting_destructor(&mut self, flags: u32) -> *const CameraControl {
        let control = self.control_mut();
        let address = control as *const CameraControl;
        drop(std::mem::take(&mut control.bounds));
        self.destroyed = true;
        if flags & 1 != 0 {
            drop(std::mem::take(&mut self.storage));
        }
        address
    }
}
const CELL_PIXELS: [f32; 5] = [8.0, 16.0, 32.0, 73.0, 146.0];
const PITCH_DEGREES: [f32; 5] = [30.0, 35.0, 40.0, 45.0, 45.0];
const SCROLL_STEPS: [f32; 5] = [100.0, 50.0, 25.0, 12.5, 6.25];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ZoomRotation {
    pub zoom: u32,
    pub rotation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ViewMessageKind {
    CameraUpdated = 0x4A9F_2FFE,
    ViewZRChange1 = 0xA6B7_9600,
    ViewZRChange2 = 0xA6B7_9601,
    ViewZRChange3 = 0xA6B7_9602,
    ViewTChange1 = 0xA6B7_9610,
    ViewTChange2 = 0xA6B7_9611,
    ViewTChange3 = 0xA6B7_9612,
    PostViewInit = 0xA6B7_9620,
    PostViewRefresh = 0xA6B7_9621,
}

/// Diagnostic snapshot taken at send time, not a native message delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewMessage {
    pub kind: ViewMessageKind,
    pub previous: ZoomRotation,
    pub new: ZoomRotation,
}

/// One synchronous message object, reused across each post-change sequence.
/// Payload borrows are resolved against the live control at observation time.
#[derive(Debug)]
pub struct LiveViewMessage {
    pub kind: ViewMessageKind,
}

#[derive(Debug)]
pub enum ViewMessageData<'a> {
    Control(&'a CameraControl),
    Pair(&'a ZoomRotation),
}

impl LiveViewMessage {
    pub fn data1<'a>(&self, control: &'a CameraControl) -> ViewMessageData<'a> {
        if self.kind == ViewMessageKind::CameraUpdated {
            ViewMessageData::Control(control)
        } else {
            ViewMessageData::Pair(&control.previous)
        }
    }
    pub fn data2<'a>(&self, control: &'a CameraControl) -> Option<&'a ZoomRotation> {
        (self.kind != ViewMessageKind::CameraUpdated).then_some(&control.new)
    }
}

/// A synchronous host send, not a queue. The mutable control permits reentry;
/// no camera lock is held while recipients run. Hosts resolve live payloads
/// using data1/data2; references must not escape the callback.
pub trait ViewMessageServer: std::fmt::Debug + Send + Sync {
    fn send(&self, control: &mut CameraControl, message: &LiveViewMessage);
}

/// Only the documented foreign terrain-view query is supplied by the host.
pub trait TerrainView {
    fn ray_cast(&mut self, origin: Vec3, direction: Vec3) -> Option<Vec3>;
}

/// Foreign view-utilities inputs/notification, never a camera owner result.
/// Called without a camera lock; the callback can read the computed camera.
pub trait CameraViewUtilities: std::fmt::Debug + Send + Sync {
    /// Corner indices 0, 1, 2, 3, as consumed by §2.5.
    fn city_corners(&self, rotation: u32, city_size: Vec2) -> [Vec3; 4];
    fn update_camera_position(&self, control: &CameraControl);
}

#[derive(Debug)]
pub struct CameraControl {
    initialised: bool,
    loaded: bool,
    references: u32,
    camera: Option<CameraHandle>,
    plane_origin: Vec3,
    corner: Vec3,
    eye: Vec3,
    target: Vec3,
    velocity: Vec3,
    direction: Vec3,
    axes: [Vec3; 2],
    plane: [f32; 4],
    eye_plane: Vec2,
    eye_pixels: Vec2,
    previous_eye_points: [f32; 4],
    tweak: Vec2,
    magnification: f32,
    previous: ZoomRotation,
    current: ZoomRotation,
    new: ZoomRotation,
    yaw: f32,
    pitch: f32,
    near: f32,
    far: f32,
    viewport: [i32; 2],
    metres_per_pixel: f32,
    pixels_per_metre: f32,
    offset: Vec3,
    bounds_in_use: bool,
    bounds: Vec<Vec2>,
    city_size: Vec2,
    messages: Vec<ViewMessage>,
    message_server: Option<Arc<dyn ViewMessageServer>>,
    view_utilities: Option<Arc<dyn CameraViewUtilities>>,
}

impl Default for CameraControl {
    fn default() -> Self {
        Self {
            initialised: false,
            loaded: false,
            references: 0,
            camera: None,
            plane_origin: [0.0; 3],
            corner: [0.0; 3],
            eye: [0.0; 3],
            target: [0.0; 3],
            velocity: [0.0; 3],
            direction: [0.0; 3],
            axes: [[0.0; 3], [0.0, 1.0, 0.0]],
            plane: [0.0; 4],
            eye_plane: [0.0; 2],
            eye_pixels: [0.0; 2],
            previous_eye_points: [0.0; 4],
            tweak: [0.0; 2],
            magnification: 1.0,
            previous: ZoomRotation::default(),
            current: ZoomRotation::default(),
            new: ZoomRotation::default(),
            yaw: -FRAC_PI_8,
            pitch: PITCH_DEGREES[0].to_radians(),
            near: 0.0,
            far: 0.0,
            viewport: [800, 600],
            metres_per_pixel: 1.0,
            pixels_per_metre: 1.0,
            offset: [0.0; 3],
            bounds_in_use: true,
            bounds: Vec::new(),
            city_size: [4096.0, 4096.0],
            messages: Vec::new(),
            message_server: None,
            view_utilities: None,
        }
    }
}

fn write_camera(camera: &CameraHandle) -> RwLockWriteGuard<'_, S3dCamera> {
    camera.write().unwrap_or_else(|poison| poison.into_inner())
}

impl CameraControl {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn class_id(&self) -> u32 {
        CAMERA_CONTROL_CLASS_ID
    }
    pub fn reference_count(&self) -> u32 {
        self.references
    }
    /// Literal shared AddRef ledger (§2.1); allocation life is owned by
    /// CameraControlAllocation, never by a borrowed control.
    pub fn add_ref(&mut self) -> u32 {
        self.references = self.references.wrapping_add(1);
        self.references
    }
    /// Supported IIDs require a writable output cell. Rejected IIDs leave
    /// even an absent output untouched. All interfaces use this same address.
    pub fn query_interface(&mut self, iid: u32, output: Option<&mut *const Self>) -> bool {
        self.query_interface_with(iid, output, |control, _| {
            control.add_ref();
        })
    }
    fn query_interface_with(
        &mut self,
        iid: u32,
        output: Option<&mut *const Self>,
        add_ref: impl FnOnce(&mut Self, &*const Self),
    ) -> bool {
        if !matches!(iid, 1 | CAMERA_CONTROL_CLASS_ID | 0xE4FD_A3D4) {
            return false;
        }
        let output = output.expect("supported camera-control IID requires an output cell");
        *output = self as *const Self;
        add_ref(self, output);
        true
    }
    pub fn initialised(&self) -> bool {
        self.initialised
    }
    pub fn loaded(&self) -> bool {
        self.loaded
    }
    pub fn camera(&self) -> Option<CameraHandle> {
        self.camera.clone()
    }
    pub fn view_target(&self) -> Vec3 {
        self.target
    }
    pub fn view_target_velocity(&self) -> Vec3 {
        self.velocity
    }
    pub fn eye_position(&self) -> Vec3 {
        self.eye
    }
    pub fn eye_offset(&self) -> Vec3 {
        self.offset
    }
    pub fn rotation_corner(&self) -> Vec3 {
        self.corner
    }
    pub fn rotation_corner_into(&self, rotation: u32, output: &mut Vec3) {
        output[1] = 0.0;
        match rotation {
            0 => {
                output[0] = 0.0;
                output[2] = 0.0;
            }
            1 => {
                output[0] = self.city_size[0];
                output[2] = 0.0;
            }
            2 => {
                output[0] = self.city_size[0];
                output[2] = self.city_size[1];
            }
            3 => {
                output[0] = 0.0;
                output[2] = self.city_size[1];
            }
            _ => {}
        }
    }
    pub fn camera_plane_origin(&self) -> Vec3 {
        self.plane_origin
    }
    pub fn camera_plane(&self) -> [f32; 4] {
        self.plane
    }
    pub fn camera_plane_axes(&self) -> [Vec3; 2] {
        self.axes
    }
    pub fn eye_camera_plane_point(&self) -> Vec2 {
        self.eye_plane
    }
    pub fn eye_camera_plane_pixels(&self) -> Vec2 {
        self.eye_pixels
    }
    pub fn previous_eye_camera_plane_points(&self) -> [f32; 4] {
        self.previous_eye_points
    }
    /// Compatibility name retained for callers; §2.3 caches the eye, not target.
    pub fn target_camera_plane_point(&self) -> Vec2 {
        self.eye_camera_plane_point()
    }
    pub fn view_direction(&self) -> Vec3 {
        self.direction
    }
    pub fn zoom(&self) -> u32 {
        self.current.zoom
    }
    pub fn rotation(&self) -> u32 {
        self.current.rotation
    }
    pub fn previous_zoom_and_rotation(&self) -> ZoomRotation {
        self.previous
    }
    pub fn new_zoom_and_rotation(&self) -> ZoomRotation {
        self.new
    }
    pub fn custom_magnification(&self) -> f32 {
        self.magnification
    }
    pub fn virtual_zoom(&self) -> u32 {
        // CA4: retain bounded arithmetic even for malformed saved indices.
        self.current.zoom.wrapping_add(u32::from(self.magnification > 1.5))
    }
    pub fn zoom_limits(&self) -> (u32, u32, f32) {
        (0, 4, 5.0)
    }
    pub fn distances_to_clip_planes(&self) -> Vec2 {
        [self.near, self.far]
    }
    pub fn viewport_size(&self) -> [i32; 2] {
        self.viewport
    }
    pub fn city_size(&self) -> Vec2 {
        self.city_size
    }
    pub fn metres_per_pixel(&self) -> f32 {
        self.metres_per_pixel
    }
    pub fn pixels_per_metre(&self) -> f32 {
        self.pixels_per_metre
    }
    pub fn pitch(&self) -> f32 {
        self.pitch
    }
    pub fn yaw(&self) -> f32 {
        self.yaw
    }
    pub fn position_tweak(&self) -> Vec2 {
        self.tweak
    }
    pub fn scroll_bounds_in_use(&self) -> bool {
        self.bounds_in_use
    }
    pub fn scroll_bounds(&self) -> &[Vec2] {
        &self.bounds
    }
    pub fn scroll_step(&self) -> f32 {
        // Safety policy only: native malformed-save lookup is unchecked and
        // has no deterministic out-of-array contract (CA4, §2.6).
        SCROLL_STEPS[self.current.zoom.min(4) as usize]
    }
    pub fn messages(&self) -> &[ViewMessage] {
        &self.messages
    }
    pub fn take_messages(&mut self) -> Vec<ViewMessage> {
        std::mem::take(&mut self.messages)
    }

    pub fn send_post_view_change_message(&mut self, kind: ViewMessageKind) {
        self.send_message(&LiveViewMessage { kind });
    }
    pub fn send_pre_view_change(&mut self, kind: u32) {
        if kind == 2 {
            self.send_post_view_change_message(ViewMessageKind::ViewZRChange1);
        }
    }
    pub fn send_post_view_change(&mut self, kind: u32) {
        match kind {
            0 => self.send_post_view_change_message(ViewMessageKind::PostViewInit),
            2 => self.send_sequence(&[
                ViewMessageKind::CameraUpdated,
                ViewMessageKind::ViewZRChange2,
                ViewMessageKind::ViewZRChange3,
            ]),
            3 => self.send_target_changed(),
            4 => self.send_post_view_change_message(ViewMessageKind::PostViewRefresh),
            _ => {}
        }
    }
    /// §2.7: every 32-bit kind is accepted; only absence of a camera fails.
    pub fn update(&mut self, kind: u32) -> bool {
        if self.camera.is_none() {
            return false;
        }
        self.send_pre_view_change(kind);
        if kind != 3 {
            self.update_camera_zoom_and_rotation_params();
        }
        self.update_camera_position_params();
        if let Some(view) = self.view_utilities.clone() {
            view.update_camera_position(self);
            if self.bounds.is_empty() || matches!(kind, 1 | 2) {
                self.update_scroll_bounds();
            }
        }
        self.send_post_view_change(kind);
        if kind == 2 {
            self.clamp_target_to_scroll_bounds();
        }
        true
    }
    pub fn set_message_server(&mut self, server: Option<Arc<dyn ViewMessageServer>>) {
        self.message_server = server;
    }
    pub fn set_view_utilities(&mut self, utilities: Option<Arc<dyn CameraViewUtilities>>) {
        self.view_utilities = utilities;
    }
    fn send_message(&mut self, message: &LiveViewMessage) {
        self.messages.push(ViewMessage {
            kind: message.kind,
            previous: self.previous,
            new: self.new,
        });
        if let Some(server) = self.message_server.clone() {
            server.send(self, message);
        }
    }
    fn send_sequence(&mut self, kinds: &[ViewMessageKind]) {
        let mut message = LiveViewMessage {
            kind: ViewMessageKind::CameraUpdated,
        };
        for &kind in kinds {
            message.kind = kind;
            self.send_message(&message);
        }
    }
    fn send_target_changed(&mut self) {
        self.send_sequence(&[
            ViewMessageKind::CameraUpdated,
            ViewMessageKind::ViewTChange2,
            ViewMessageKind::ViewTChange3,
        ]);
    }

    /// The foreign view supplies a camera identity, never an owner-method
    /// result. Attach to the core-owned camera and derive all state here.
    pub fn init(&mut self, target: Vec3, camera: impl Into<Option<CameraHandle>>) -> bool {
        if self.initialised {
            return true;
        }
        if self.loaded {
            self.new = self.current;
        } else {
            self.target = target;
        }
        if let Some(camera) = camera.into() {
            self.attach_to_camera(camera);
            self.update(0);
        }
        self.initialised = true;
        true
    }
    pub fn attach_to_camera(&mut self, camera: CameraHandle) {
        if self.camera.as_ref().is_some_and(|old| Arc::ptr_eq(old, &camera)) {
            return;
        }
        {
            let mut c = write_camera(&camera);
            c.add_ref();
        }
        self.detach_from_camera();
        {
            let mut c = write_camera(&camera);
            c.set_viewport(self.viewport[0] as f32, self.viewport[1] as f32);
        }
        self.camera = Some(camera);
        if self.initialised {
            self.update(0);
        }
    }
    pub fn detach_from_camera(&mut self) {
        if let Some(camera) = self.camera.take() {
            write_camera(&camera).release();
        }
    }
    // Adapter convenience for external Detach-before-Release teardown.
    // The native destructor does not detach; Init remains retained.
    pub fn shutdown(&mut self) {
        self.detach_from_camera();
    }

    pub fn update_camera_zoom_and_rotation_params(&mut self) {
        // Safety policy, not native fidelity: contain malformed-save table
        // lookups. Native out-of-array values are not a valid-state contract.
        let zoom = self.current.zoom.min(4) as usize;
        let pixels = CELL_PIXELS[zoom] * self.magnification;
        let cell_span = 16.0 * (FRAC_PI_8.sin() + FRAC_PI_8.cos());
        self.metres_per_pixel = cell_span / pixels;
        self.pixels_per_metre = 1.0 / self.metres_per_pixel;
        self.yaw = (self.current.rotation as f32 + 1.0) * FRAC_PI_2 - FRAC_PI_8;
        self.pitch = PITCH_DEGREES[zoom].to_radians();
        let d = 8192.0 / self.pitch.cos();
        let w = self.city_size[0].max(4096.0);
        let h = self.city_size[1].max(4096.0);
        self.near = (d - (w * w + h * h).sqrt() / self.pitch.cos()) * 0.95;
        self.far = d * 1.5;
        let a = self.viewport[0] as f32 * self.metres_per_pixel * 0.5;
        let c = self.viewport[1] as f32 * self.metres_per_pixel * 0.5;
        if let Some(camera) = &self.camera {
            write_camera(camera).set_ortho(-a, a, -c, c, self.near, self.far);
        }
        self.offset = [
            self.yaw.cos() * 8192.0,
            self.pitch.tan() * 8192.0,
            self.yaw.sin() * 8192.0,
        ];
    }
    pub fn update_camera_position_params(&mut self) {
        let mut corner = self.corner;
        self.rotation_corner_into(self.current.rotation, &mut corner);
        self.corner = corner;
        self.plane_origin = add(self.corner, self.offset);
        let length2 = dot(self.offset, self.offset);
        let k = if length2 == 0.0 {
            0.0
        } else {
            1.0 - dot(sub(self.target, self.corner), self.offset) / length2
        };
        self.eye = add(self.target, mul(self.offset, k));
        if let Some(camera) = &self.camera {
            let mut c = write_camera(camera);
            c.set_position(self.eye);
            c.set_look_at(self.target, [0.0, 1.0, 0.0]);
            c.set_pixel_offset(0.0, 0.0);
            if let Some(p) = c.project(self.corner) {
                c.set_pixel_offset(p[0] - snap(p[0]) + self.tweak[0], -(p[1] - snap(p[1])) + self.tweak[1]);
            }
            let view = c.view_xform();
            self.axes = [row(&view.matrix, 0), row(&view.matrix, 1)];
            self.direction = mul(row(&view.matrix, 2), -1.0);
        }
        self.plane = [
            self.direction[0],
            self.direction[1],
            self.direction[2],
            -dot(self.direction, self.eye),
        ];
        if let Some(camera) = &self.camera {
            self.previous_eye_points = [
                self.eye_plane[0],
                self.eye_plane[1],
                self.eye_pixels[0],
                self.eye_pixels[1],
            ];
            self.eye_plane = self.convert_world_point_to_camera_plane_point(self.eye);
            let offset = write_camera(camera).pixel_offset();
            self.eye_pixels = [
                self.pixels_per_metre * self.eye_plane[0] + offset[0],
                self.pixels_per_metre * self.eye_plane[1] + offset[1],
            ];
        }
    }
    pub fn convert_world_point_to_camera_plane_point(&self, point: Vec3) -> Vec2 {
        let p = sub(point, self.plane_origin);
        // No numeric contract without a camera (§2.4); never use stale copies.
        let c = self
            .camera
            .as_ref()
            .expect("plane conversion requires an attached camera");
        let c = c.read().unwrap_or_else(|poison| poison.into_inner());
        let view = c.view_xform();
        [
            dot(p, row(&view.matrix, 0)) * view.scale,
            dot(p, row(&view.matrix, 1)) * view.scale,
        ]
    }
    pub fn convert_camera_plane_point_to_world_point(&self, point: Vec2) -> Vec3 {
        let c = self
            .camera
            .as_ref()
            .expect("plane conversion requires an attached camera");
        let c = c.read().unwrap_or_else(|poison| poison.into_inner());
        let view = c.view_xform();
        let reciprocal = 1.0 / view.scale;
        let r = row(&view.matrix, 0);
        let u = row(&view.matrix, 1);
        let b = row(&view.matrix, 2);
        // The zero-depth terms must participate even for nonfinite back rows.
        std::array::from_fn(|i| self.plane_origin[i] + (point[0] * r[i] + point[1] * u[i] + 0.0 * b[i]) * reciprocal)
    }

    /// Documented foreign city-corner inputs (view-utilities §4.2), including
    /// the rectangular-city corner-2/rotation-1 quirk. These are not results
    /// of either owner being implemented in this module.
    fn city_corners(&self) -> [Vec3; 4] {
        let [w, d] = self.city_size;
        let pairs = match self.current.rotation {
            0 => [[0.0, d], [0.0, 0.0], [w, 0.0], [w, d]],
            1 => [[0.0, 0.0], [w, 0.0], [w, w], [0.0, d]],
            2 => [[w, 0.0], [w, d], [0.0, d], [0.0, 0.0]],
            3 => [[w, d], [0.0, d], [0.0, 0.0], [w, 0.0]],
            _ => [[0.0; 2]; 4],
        };
        pairs.map(|p| [p[0], 0.0, p[1]])
    }
    pub fn update_scroll_bounds(&mut self) {
        if !self.bounds_in_use {
            return;
        }
        let corners = self.view_utilities.as_ref().map_or_else(
            || self.city_corners(),
            |view| view.city_corners(self.current.rotation, self.city_size),
        );
        self.update_scroll_bounds_with_corners(corners);
    }
    /// Allows a foreign view to supply its four documented city corners;
    /// camera-plane conversion and polygon construction remain owner logic.
    pub fn update_scroll_bounds_with_corners(&mut self, corners: [Vec3; 4]) {
        if !self.bounds_in_use {
            return;
        }
        let [b, a, c, d] = corners.map(|p| self.convert_world_point_to_camera_plane_point(p));
        // CA1 asks only for the interpretation: preserve this exact formula.
        let lift = (self.offset[1] * 0.48) / self.axes[1][1] - self.metres_per_pixel * self.viewport[1] as f32 * 0.5;
        self.bounds = vec![
            [a[0], a[1] + lift],
            [b[0] - 500.0, b[1] + lift],
            [b[0] - 500.0, b[1]],
            [d[0], d[1]],
            [c[0] + 500.0, c[1]],
            [c[0] + 500.0, c[1] + lift],
        ];
    }
    pub fn set_scroll_bounds_in_use(&mut self, enabled: bool) {
        self.bounds_in_use = enabled;
        self.update_scroll_bounds();
    }
    /// §2.5: cut at the earliest outward crossing, then slide along its edge.
    pub fn clamp_scroll_to_bounds(&self, from: Vec2, to: Vec2) -> Vec2 {
        clamp_polygon(&self.bounds, self.bounds_in_use, from, to)
    }
    pub fn clamp_target_to_scroll_bounds(&mut self) {
        let q = self.convert_world_point_to_camera_plane_point(self.target);
        let from = self.convert_world_point_to_camera_plane_point(self.corner);
        let clamped = self.clamp_scroll_to_bounds(from, q);
        if (clamped[0] - q[0]).abs() < 0.0001 && (clamped[1] - q[1]).abs() < 0.0001 {
            return;
        }
        let world = self.convert_camera_plane_point_to_world_point(clamped);
        if let Some(hit) = intersect_with_ground_plane(self.direction, world) {
            self.target = hit;
            self.velocity = [0.0; 3];
        }
        if self.camera.is_some() {
            self.update_camera_position_params();
            self.send_target_changed();
        }
    }

    pub fn set_zoom(&mut self, mut zoom: f32) -> bool {
        let m = if zoom > 4.0 && zoom < 5.0 {
            let m = 2.0_f64.powf(f64::from(zoom - 4.0)) as f32;
            zoom = 4.0;
            m
        } else {
            1.0
        };
        // CA4: NaN is refused rather than converted to an integer index.
        if !(0.0..=4.0).contains(&zoom) {
            return false;
        }
        let z = zoom.trunc() as u32;
        if zoom == self.current.zoom as f32 && m == self.magnification {
            return false;
        }
        self.set_custom_magnification(m);
        self.change_zoom_rotation(ZoomRotation {
            zoom: z,
            rotation: self.current.rotation,
        })
    }
    pub fn set_zoom_and_rotation(&mut self, zoom: u32, rotation: u32) -> bool {
        if zoom > 4 || rotation > 3 {
            return false;
        }
        let pair = ZoomRotation { zoom, rotation };
        self.set_custom_magnification(1.0);
        if pair == self.current {
            return false;
        }
        self.change_zoom_rotation(pair)
    }
    pub fn set_rotation(&mut self, rotation: u32) -> bool {
        if rotation > 3 {
            return true;
        }
        if rotation == self.current.rotation {
            return true;
        }
        self.change_zoom_rotation(ZoomRotation {
            zoom: self.current.zoom,
            rotation,
        })
    }
    pub fn rotate_left(&mut self) -> bool {
        self.set_rotation(((self.current.rotation.wrapping_add(1) as i32) % 4) as u32)
    }
    pub fn rotate_right(&mut self) -> bool {
        self.set_rotation(((self.current.rotation.wrapping_add(3) as i32) % 4) as u32)
    }
    pub fn zoom_in(&mut self) -> bool {
        let v = self.virtual_zoom() as i32;
        if f64::from(v) >= 5.0 {
            return false;
        }
        self.set_zoom(v.wrapping_add(1) as f32)
    }
    pub fn zoom_out(&mut self) -> bool {
        let v = self.virtual_zoom() as i32;
        if v <= 0 {
            return false;
        }
        self.set_zoom(v.wrapping_sub(1) as f32)
    }
    fn change_zoom_rotation(&mut self, pair: ZoomRotation) -> bool {
        self.previous = self.current;
        self.current = pair;
        self.new = pair;
        self.update(2)
    }
    pub fn set_custom_magnification(&mut self, m: f32) {
        if m != self.magnification && self.magnification > 0.0 && self.magnification < 1000.0 {
            self.magnification = m;
            self.update(4);
        }
    }
    pub fn set_view_target(&mut self, target: Vec3) -> bool {
        if !target.iter().all(|v| v.is_finite() && v.abs() <= 100_000.0) {
            return false;
        }
        self.target = target;
        self.velocity = [0.0; 3];
        self.update(3);
        true
    }
    pub fn set_view_target_velocity(&mut self, velocity: Vec3) {
        if velocity.iter().all(|v| !v.is_nan()) {
            self.velocity = velocity;
        }
    }
    pub fn scroll_by_viewport_angle(&mut self, angle: f32, amount: f32, terrain: Option<&mut dyn TerrainView>) -> bool {
        if self.camera.is_none() {
            return false;
        }
        let from = self.convert_world_point_to_camera_plane_point(self.target);
        let step = self.scroll_step();
        let to = [
            from[0] + angle.cos() * amount * step,
            from[1] - angle.sin() * amount * step,
        ];
        let to = self.clamp_scroll_to_bounds(from, to);
        self.target = self.convert_camera_plane_point_to_world_point(to);
        let terrain_hit = terrain.and_then(|t| t.ray_cast(self.target, self.direction));
        if let Some(hit) = terrain_hit.or_else(|| intersect_with_ground_plane(self.direction, self.target)) {
            self.target = hit;
        }
        self.update(3);
        true
    }
    pub fn set_camera_position_tweak_add(&mut self, x: f32, y: f32) {
        if self.tweak != [x, y] {
            self.tweak = [x, y];
            self.update(5);
        }
    }
    pub fn set_city_size(&mut self, width: f32, depth: f32) {
        self.city_size = [width, depth];
        if self.initialised {
            self.bounds.clear();
            self.update(5);
        }
    }
    pub fn set_viewport_size(&mut self, width: i32, height: i32) {
        self.viewport = [width, height];
        if let Some(camera) = &self.camera {
            write_camera(camera).set_viewport(width as f32, height as f32);
        }
        // CA5: §2.6 promises the viewport update only, no implicit recompute.
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(1);
        for v in self.target {
            out.put_f32(v);
        }
        for pair in [self.previous, self.current, self.new] {
            out.put_u32(pair.zoom);
            out.put_u32(pair.rotation);
        }
        out.error() == 0
    }
    pub fn read(&mut self, inp: &mut dyn InStream) -> bool {
        self.loaded = true;
        if inp.get_u16() != Some(1) {
            return false;
        }
        // Mutate each destination immediately and attempt every primitive.
        // InStream's None leaves that destination untouched; stream errors
        // determine success, with no camera-level rollback or validation.
        for v in &mut self.target {
            if let Some(value) = inp.get_f32() {
                *v = value;
            }
        }
        for pair in [&mut self.previous, &mut self.current, &mut self.new] {
            if let Some(zoom) = inp.get_u32() {
                pair.zoom = zoom;
            }
            if let Some(rotation) = inp.get_u32() {
                pair.rotation = rotation;
            }
        }
        inp.error() == 0
    }
}

fn dot2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

pub(super) fn clamp_polygon(polygon: &[Vec2], enabled: bool, original: Vec2, to: Vec2) -> Vec2 {
    clamp_polygon_observed(polygon, enabled, original, to, |_| {})
}

// The observer counts completed edge projections, allowing §6.1's exhaustion
// discriminator to prove it reaches the budget instead of an earlier exit.
fn clamp_polygon_observed(
    polygon: &[Vec2],
    enabled: bool,
    original: Vec2,
    to: Vec2,
    mut projected: impl FnMut(usize),
) -> Vec2 {
    if !enabled || polygon.is_empty() {
        return to;
    }
    let n = polygon.len();
    let mut start = original;
    let mut end = to;
    let original_direction = [
        f64::from(to[0]) - f64::from(original[0]),
        f64::from(to[1]) - f64::from(original[1]),
    ];
    let mut negative = false;
    let mut nonnegative = false;
    for _ in 0..n {
        let mut best = 1.0_f32;
        let mut first = None;
        for i in 0..n {
            let a = polygon[(i + n - 1) % n].map(f64::from);
            let b = polygon[i].map(f64::from);
            let v = [a[0] - b[0], a[1] - b[1]];
            let length = dot2(v, v).sqrt();
            // §2.5 specifies the double threshold before the float length
            // store, followed by normalization using that stored length.
            if length < 1e-8_f64 {
                continue;
            }
            let length = f64::from(length as f32);
            let normal = [v[1] / length, -v[0] / length];
            let distance = |p: Vec2| dot2(normal, p.map(f64::from)) - dot2(normal, a);
            let end_side = distance(end) as f32;
            let start_side = distance(start);
            if end_side > 0.0 || start_side <= f64::from(end_side) {
                continue;
            }
            let fraction = start_side / (start_side - f64::from(end_side));
            if fraction < f64::from(best) {
                best = fraction as f32;
                first = Some((i, [-v[0], -v[1]], length));
            }
        }
        let Some((edge_index, edge, length)) = first else {
            return end;
        };
        let fraction = if best >= 0.001_f32 {
            f64::from(best) - f64::from(0.0001_f32)
        } else {
            0.0
        };
        let remaining: Vec2 =
            std::array::from_fn(|j| ((f64::from(end[j]) - f64::from(start[j])) * (1.0 - fraction)) as f32);
        let intersection: Vec2 = std::array::from_fn(|j| end[j] - remaining[j]);
        if dot2(original_direction, edge) < 0.0 {
            negative = true;
        } else {
            nonnegative = true;
        }
        if negative && nonnegative {
            return intersection;
        }
        let unit = [edge[0] / length, edge[1] / length];
        let along = dot2(remaining.map(f64::from), unit);
        start = intersection;
        end = std::array::from_fn(|j| (f64::from(intersection[j]) + unit[j] * along) as f32);
        projected(edge_index);
    }
    original
}

#[cfg(test)]
#[path = "closure_tests.rs"]
mod closure_tests;

#[cfg(test)]
#[path = "resolved_tests.rs"]
mod resolved_tests;

#[cfg(test)]
mod ground_failure_contracts {
    use super::*;
    fn fixture() -> CameraControl {
        CameraControl {
            camera: Some(Arc::new(RwLock::new(S3dCamera::new()))),
            direction: [1.0, 0.0, 0.0],
            velocity: [1.0, 2.0, 3.0],
            ..CameraControl::new()
        }
    }
    fn assert_updates(c: &CameraControl) {
        assert_eq!(
            c.messages.iter().map(|m| m.kind).collect::<Vec<_>>(),
            [
                ViewMessageKind::CameraUpdated,
                ViewMessageKind::ViewTChange2,
                ViewMessageKind::ViewTChange3
            ]
        );
        assert_eq!(c.velocity, [1.0, 2.0, 3.0]);
    }
    #[test]
    fn angle_scroll_stores_plane_point_before_failed_ground_and_still_notifies() {
        struct Miss;
        impl TerrainView for Miss {
            fn ray_cast(&mut self, origin: Vec3, direction: Vec3) -> Option<Vec3> {
                assert_eq!(origin, [7.0, 8.0, 9.0]);
                assert_eq!(direction, [1.0, 0.0, 0.0]);
                None
            }
        }
        let mut c = fixture();
        // §6.1 defined inverse result (7,8,9), parallel ground ray.
        c.plane_origin = [7.0, 8.0, 9.0];
        c.target = c.plane_origin;
        assert!(c.scroll_by_viewport_angle(0.0, 0.0, Some(&mut Miss)));
        assert_eq!(c.target, [7.0, 8.0, 9.0]);
        assert_updates(&c);
    }
    #[test]
    fn target_clamp_uses_corner_start_retains_old_target_after_failed_ground() {
        let mut c = fixture();
        c.axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        c.corner = [5.0, 5.0, 0.0];
        c.target = [15.0, 5.0, 9.0];
        c.bounds = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        c.clamp_target_to_scroll_bounds();
        assert_eq!(c.target, [15.0, 5.0, 9.0]);
        assert_updates(&c);
    }
}
