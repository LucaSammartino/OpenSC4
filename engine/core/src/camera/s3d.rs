// SPDX-License-Identifier: GPL-3.0-or-later
use super::geometry::*;
use crate::network_occupants::xform::{Xform, IDENTITY_MATRIX, XFORM_MATRIX, XFORM_SCALE, XFORM_TRANSLATION};

pub const CAMERA_CLASS_ID: u32 = 0xE9C6_262A;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewVolume {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

#[derive(Clone, Debug)]
pub struct S3dCamera {
    references: u32,
    viewport: Vec2,
    depth_size: f32,
    subview: Viewport,
    subview_in_use: bool,
    local: Xform,
    view: Xform,
    projection: Mat4,
    inverse_projection: Mat4,
    target: Vec3,
    up: Vec3,
    projection_state: u32,
    projection_type: u32,
    volume: ViewVolume,
    pixel_offset: Vec2,
    depth_bias: f32,
}

impl Default for S3dCamera {
    fn default() -> Self {
        let mut camera = Self {
            references: 0,
            viewport: [100.0, 100.0],
            depth_size: 65_536.0,
            subview: Viewport {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            subview_in_use: false,
            local: Xform::default(),
            view: Xform::default(),
            projection: [[0.0; 4]; 4],
            inverse_projection: [[0.0; 4]; 4],
            target: [0.0; 3],
            up: [0.0; 3],
            projection_state: 1,
            projection_type: 0,
            volume: ViewVolume {
                left: -1.0,
                right: 1.0,
                bottom: -1.0,
                top: 1.0,
                near: 1.0,
                far: 2.0,
            },
            pixel_offset: [0.0; 2],
            depth_bias: 0.0,
        };
        camera.update_projection();
        camera
    }
}

impl S3dCamera {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn class_id(&self) -> u32 {
        CAMERA_CLASS_ID
    }
    /// The pure-core reference ledger; Rust/Arc owns allocation/destruction.
    pub fn add_ref(&mut self) -> u32 {
        self.references = self.references.wrapping_add(1);
        self.references
    }
    pub fn query_interface(&mut self, iid: u32) -> bool {
        if iid == 1 || iid == CAMERA_CLASS_ID {
            self.add_ref();
            true
        } else {
            false
        }
    }
    pub fn release(&mut self) -> u32 {
        self.references = self.references.saturating_sub(1);
        self.references
    }
    pub fn reference_count(&self) -> u32 {
        self.references
    }
    pub fn viewport(&self) -> Vec2 {
        self.viewport
    }
    pub fn subview(&self) -> Viewport {
        self.subview
    }
    pub fn subview_in_use(&self) -> bool {
        self.subview_in_use
    }
    pub fn depth_size(&self) -> f32 {
        self.depth_size
    }
    pub fn depth_bias(&self) -> f32 {
        self.depth_bias
    }
    pub fn pixel_offset(&self) -> Vec2 {
        self.pixel_offset
    }
    pub fn position(&self) -> Vec3 {
        self.local.translation
    }
    pub fn look_at(&self) -> Vec3 {
        self.target
    }
    pub fn up(&self) -> Vec3 {
        self.up
    }
    pub fn projection_type(&self) -> u32 {
        self.projection_type
    }
    pub fn view_state(&self) -> u8 {
        self.local.counter
    }
    pub fn projection_state(&self) -> u32 {
        self.projection_state
    }
    pub fn local_xform(&self) -> &Xform {
        &self.local
    }
    pub fn view_xform(&self) -> &Xform {
        &self.view
    }
    pub fn projection_matrix(&self) -> &Mat4 {
        &self.projection
    }
    pub fn inverse_projection_matrix(&self) -> &Mat4 {
        &self.inverse_projection
    }
    pub fn raw_view_volume(&self) -> ViewVolume {
        self.volume
    }

    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport = [width, height];
        self.subview = Viewport {
            x: 0.0,
            y: 0.0,
            width,
            height,
        };
        self.subview_in_use = false;
        self.changed_projection(2);
    }
    pub fn set_subview(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.subview = Viewport { x, y, width, height };
        self.subview_in_use = true;
        self.changed_projection(1);
    }
    pub fn reset_subview(&mut self) {
        self.subview = Viewport {
            x: 0.0,
            y: 0.0,
            width: self.viewport[0],
            height: self.viewport[1],
        };
        self.subview_in_use = false;
        self.changed_projection(1);
    }
    pub fn set_depth_size(&mut self, size: f32) {
        self.depth_size = size;
        self.changed_projection(1);
    }
    pub fn set_depth_bias(&mut self, bias: f32) {
        self.depth_bias = bias;
        self.changed_projection(1);
    }
    pub fn set_pixel_offset(&mut self, dx: f32, dy: f32) {
        self.pixel_offset = [dx, dy];
        self.changed_projection(1);
    }
    pub fn set_ortho(&mut self, l: f32, r: f32, b: f32, t: f32, n: f32, f: f32) {
        self.set_volume(
            1,
            ViewVolume {
                left: l,
                right: r,
                bottom: b,
                top: t,
                near: n,
                far: f,
            },
        );
    }
    /// Implementation-only extension to evaluate the documented type-2 state.
    /// CA3 remains blocked: no native setter, signature or counter contract
    /// is identified. This entry must never be bound as a native camera slot.
    pub fn set_frustum(&mut self, l: f32, r: f32, b: f32, t: f32, n: f32, f: f32) {
        self.set_volume(
            2,
            ViewVolume {
                left: l,
                right: r,
                bottom: b,
                top: t,
                near: n,
                far: f,
            },
        );
    }
    fn set_volume(&mut self, kind: u32, volume: ViewVolume) {
        self.projection_type = kind;
        self.volume = volume;
        self.changed_projection(1);
    }
    pub fn view_volume(&self) -> ViewVolume {
        let v = self.volume;
        let dx = (v.right - v.left) / self.viewport[0] * self.pixel_offset[0];
        let dy = (v.top - v.bottom) / self.viewport[1] * self.pixel_offset[1];
        ViewVolume {
            left: v.left + dx,
            right: v.right + dx,
            bottom: v.bottom - dy,
            top: v.top - dy,
            ..v
        }
    }
    pub fn set_position(&mut self, position: Vec3) {
        self.local.translation = position;
        self.local.mask |= XFORM_TRANSLATION;
        self.local.counter = self.local.counter.wrapping_add(1);
        self.update_view_xform();
    }
    pub fn set_look_at(&mut self, target: Vec3, up: Vec3) {
        let back = normalize(sub(self.local.translation, target));
        let right = normalize(cross(up, back));
        let true_up = cross(back, right);
        self.local.matrix = [
            right[0], true_up[0], back[0], right[1], true_up[1], back[1], right[2], true_up[2], back[2],
        ];
        self.local.mask |= XFORM_MATRIX;
        self.local.counter = self.local.counter.wrapping_add(2);
        self.target = target;
        self.up = up;
        self.update_view_xform();
    }
    /// Safe replacement for mutating LocalXform directly. The caller supplies
    /// the existing transform's counter as well as its parts.
    pub fn set_local_xform(&mut self, transform: Xform) {
        self.local = transform;
        self.update_view_xform();
    }
    pub fn reset_xforms(&mut self) {
        if self.local.mask != 0 {
            self.local.reset();
        }
        if self.view.mask != 0 {
            self.view.reset();
        }
        self.update_view_xform();
    }
    pub fn update_view_xform(&mut self) {
        if self.view.counter == self.local.counter {
            return;
        }
        let matrix = if self.local.mask & XFORM_MATRIX != 0 {
            transpose(&self.local.matrix)
        } else {
            IDENTITY_MATRIX
        };
        let scale = if self.local.mask & XFORM_SCALE != 0 {
            1.0 / self.local.scale
        } else {
            1.0
        };
        let translation = mul(rotate(&matrix, self.local.translation), -scale);
        self.view = Xform {
            counter: self.local.counter,
            mask: self.local.mask,
            matrix,
            translation,
            scale,
        };
    }
    fn changed_projection(&mut self, count: u32) {
        self.projection_state = self.projection_state.wrapping_add(count);
        self.update_projection();
    }
    /// Eager caches allow readers on another thread without interior mutation.
    // Preserve §1.4's (subview expression) + old offset operand order.
    #[allow(clippy::assign_op_pattern)]
    pub fn update_projection(&mut self) {
        let v = self.volume;
        let mut sx = 1.0 / (v.right - v.left);
        let mut sy = 1.0 / (v.top - v.bottom);
        let sz = 1.0 / (v.far - v.near);
        let px = 1.0 / self.subview.width;
        let py = 1.0 / self.subview.height;
        let mut ox = -2.0 * px * self.pixel_offset[0];
        let mut oy = -2.0 * py * self.pixel_offset[1];
        let beta = -(self.depth_bias / self.depth_size);
        if self.subview_in_use {
            sx *= px * self.viewport[0];
            sy *= py * self.viewport[1];
            ox = (px * self.viewport[0] - 1.0) - 2.0 * px * self.subview.x + ox;
            oy = (py * self.viewport[1] - 1.0) - 2.0 * py * self.subview.y + oy;
        }
        let mut m = [[0.0; 4]; 4];
        let mut inv = [[0.0; 4]; 4];
        if self.projection_type == 2 {
            m[0][0] = 2.0 * v.near * sx;
            m[0][2] = (v.left + v.right) * sx - ox;
            m[1][1] = 2.0 * v.near * sy;
            m[1][2] = (v.bottom + v.top) * sy - oy;
            m[2][2] = -(v.near + v.far) * sz - beta;
            m[2][3] = -2.0 * v.near * v.far * sz;
            m[3][2] = -1.0;
            inv[0][0] = 1.0 / m[0][0];
            inv[0][3] = m[0][2] / m[0][0];
            inv[1][1] = 1.0 / m[1][1];
            inv[1][3] = m[1][2] / m[1][1];
            inv[2][3] = -1.0;
            inv[3][2] = 1.0 / m[2][3];
            inv[3][3] = m[2][2] / m[2][3];
        } else {
            m[0][0] = 2.0 * sx;
            m[0][3] = ox - (v.left + v.right) * sx;
            m[1][1] = 2.0 * sy;
            m[1][3] = oy - (v.bottom + v.top) * sy;
            m[2][2] = -2.0 * sz;
            m[2][3] = beta - (v.near + v.far) * sz;
            m[3][3] = 1.0;
            for i in 0..3 {
                inv[i][i] = 1.0 / m[i][i];
                inv[i][3] = -m[i][3] / m[i][i];
            }
            inv[3][3] = 1.0;
        }
        self.projection = m;
        self.inverse_projection = inv;
    }
    pub fn project(&self, world: Vec3) -> Option<Vec3> {
        let v = transform(&self.view, world);
        let ndc = divide(apply(&self.projection, homogeneous(v)))?;
        Some([
            (ndc[0] + 1.0) * self.subview.width / 2.0 + self.subview.x,
            (1.0 - ndc[1]) * self.subview.height / 2.0 + self.subview.y,
            ndc[2],
        ])
    }
    pub fn unproject(&self, screen: Vec3) -> Option<Vec3> {
        let ndc = [
            2.0 * (screen[0] - self.subview.x) / self.subview.width - 1.0,
            2.0 * (self.subview.y + self.subview.height - screen[1]) / self.subview.height - 1.0,
            screen[2],
        ];
        let v = divide(apply(&self.inverse_projection, homogeneous(ndc)))?;
        let p = sub(v, self.view.translation).map(|x| x / self.view.scale);
        Some(rotate(&transpose(&self.view.matrix), p))
    }
    pub fn eye_ray(&self, screen: Vec2) -> Option<Ray> {
        if self.projection_type == 2 {
            let nx = 2.0 * screen[0] / self.viewport[0] - 1.0;
            let ny = 2.0 * (self.viewport[1] - screen[1]) / self.viewport[1] - 1.0;
            Some(Ray {
                origin: self.position(),
                direction: mul(rotate(&self.local.matrix, [nx, ny, -self.volume.near]), -1.0),
            })
        } else {
            let mut d = sub(self.target, self.position());
            let q = self.unproject([screen[0], screen[1], 0.0])?;
            let length2 = dot(d, d);
            if length2 > 0.0 {
                d = mul(d, self.volume.far / length2.sqrt());
            }
            Some(Ray {
                origin: sub(q, d),
                direction: d,
            })
        }
    }
    pub fn eye_rays(&self, points: &[Vec2]) -> Vec<Option<Ray>> {
        points.iter().map(|&p| self.eye_ray(p)).collect()
    }
}
