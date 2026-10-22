// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned SC4ViewUtilities algorithms, from docs/specs/view-utilities.md §§1–7,9,11.
//!
//! The 50-body accounting and remaining numeric limits are in README.md.
//! Camera snapshots and camera eye rays are typed external camera inputs; no
//! SC4ViewUtilities result, geometry test, picking or frustum computation is
//! delegated to a host. Getters use ordinary shared references, never RefCell.

mod frustum;
mod math;
mod picking;

use std::sync::OnceLock;

pub use frustum::{clip_polygon_to_polygon_2d, Frustum, GroundLine, IndeterminateClipIntersection, Plane, Projection};
pub use math::{
    cross_in_storage, get_ray_intersection_with_ground_plane, max_of_four_floats, min_of_four_floats, point_array_aabb,
    ray_intersects_ground_plane, ray_intersects_plane, Aabb, Pixel, PixelRect, Ray, Vec3,
};
pub use picking::{
    decode_indices, mesh_pick, mesh_pick_checked, model_pick, model_pick_checked, primitive_vertex_indices,
    ray_intersects_triangle, triangle_indices, triangle_pick, InvalidMeshReference, Mesh, Model, ModelInstance, Pick,
    PrimitiveGroup, Transform, TriangleIterator,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewVolume {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

/// Raw camera/control data, copied into the owner during a camera update.
/// The clip distances are distinct from the camera's view-volume distances.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraState {
    pub position: Vec3,
    pub look_at: Vec3,
    pub zoom: u32,
    pub rotation: i32,
    pub viewport_width: i32,
    pub viewport_height: i32,
    pub near_clip_distance: f32,
    pub far_clip_distance: f32,
    pub metres_per_pixel: f32,
    pub pixels_per_metre: f32,
    pub volume: ViewVolume,
    pub projection: Projection,
}

/// Only the camera's external eye-ray query (§4.1). Implementations must not
/// call the original SC4ViewUtilities. The owner normalizes the direction and
/// computes ground intersections and all downstream algorithms itself.
pub trait CameraRayProvider {
    fn eye_ray(&self, x: f32, y: f32) -> Option<Ray>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewAngles {
    pub yaw: f32,
    pub pitch: f32,
    pub ground_pitch: f32,
}

pub fn get_camera_view_angles_for_rotation(rotation: i32, zoom: u32) -> Option<ViewAngles> {
    // Checked adapter: native out-of-table reads have no portable value.
    let pitch = f32::from_bits(*[0x3f06_0a92, 0x3f1c_61aa, 0x3f32_b8c2, 0x3f49_0fdb, 0x3f49_0fdb].get(zoom as usize)?);
    let offset = f64::from(f32::from_bits(0xbec9_0fdb));
    Some(ViewAngles {
        // At 64-bit x87 precision this product/add is exact before the store:
        // quarter-turn = 0xC90FDB * 2^-23, offset = -quarter-turn/4.
        // Integer accumulation avoids an unproved f64 double-rounding shortcut.
        yaw: ((i64::from(rotation) * 4 - 1) * 0x00c9_0fdb) as f32 * (1.0 / 33_554_432.0),
        pitch,
        // Native FSIN/FCOS/FPTAN/FPATAN have no portable libm bit guarantee.
        ground_pitch: (f64::from(pitch).tan() / offset.cos()).atan() as f32,
    })
}

pub fn get_object_y_rotation_angle(rotation: i32) -> f32 {
    (-(i64::from(rotation) * 0x00c9_0fdb)) as f32 * (1.0 / 8_388_608.0)
}

pub fn get_city_width_from_ortho_view(width: f32, zoom: u32) -> Option<f32> {
    get_camera_view_angles_for_rotation(0, zoom)?;
    let a = f64::from(f32::from_bits(0x3ec9_0fdb));
    Some((f64::from(width) * (a.cos() + a.sin())) as f32)
}

pub fn rotate_vector_by_view_rotation(v: &mut Vec3, rotation: i32) {
    let (x, z) = (v.x, v.z);
    match rotation % 4 {
        1 => {
            v.x = -z;
            v.z = x;
        }
        2 => {
            v.x = -x;
            v.z = -z;
        }
        3 => {
            v.x = z;
            v.z = -x;
        }
        _ => {}
    }
}

pub fn get_vector_for_edge_into(edge: i32, out: &mut Vec3) {
    out.y = 0.0;
    let (x, z) = match edge % 4 {
        0 => (0.0, -1.0),
        1 => (1.0, 0.0),
        2 => (0.0, 1.0),
        3 => (-1.0, 0.0),
        _ => return,
    };
    out.x = x;
    out.z = z;
}

/// Value-returning adapter uses zero for the otherwise preserved x/z words.
pub fn get_vector_for_edge(edge: i32) -> Vec3 {
    let mut out = Vec3::ZERO;
    get_vector_for_edge_into(edge, &mut out);
    out
}

pub fn round_down_to_nearest_block_boundary(value: f32, block: i32) -> Option<i32> {
    round_block(value, block, false)
}
pub fn round_up_to_nearest_block_boundary(value: f32, block: i32) -> Option<i32> {
    round_block(value, block, true)
}
fn round_block(value: f32, block: i32, up: bool) -> Option<i32> {
    // Masked invalid _ftol2 conversion gives low word zero. Checked division
    // exposes native IDIV faults as None, including the second adjusted divide.
    let n = crate::x87::ftol32(f64::from(value));
    let remainder = n.checked_rem(block)?;
    let adjusted = if remainder != 0 && up && n >= 0 {
        n.wrapping_add(block)
    } else if remainder != 0 && !up && n <= 0 {
        n.wrapping_sub(block)
    } else {
        n
    };
    Some(adjusted.checked_div(block)?.wrapping_mul(block))
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VisibleArea {
    pub inner: Aabb,
    pub outer: Aabb,
}

/// All persistent state is owned; camera update/draw mutation requires &mut self.
/// Data read from another thread can be exposed as shared immutable snapshots.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewUtilities {
    renderer_present: bool,
    camera_present: bool,
    valid: bool,
    city: (f32, f32),
    camera: CameraState,
    angles: ViewAngles,
    rotation_yaw: f32,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
    sin_negative_yaw: f32,
    cos_negative_yaw: f32,
    sin_pitch: f32,
    cos_pitch: f32,
    scales: Vec3,
    inverse_scales: Vec3,
    view: Frustum,
    draw: Frustum,
    draw_narrowed: bool,
    draw_rect: PixelRect,
    viewport_ground: OnceLock<Option<[Vec3; 4]>>,
    visible_area: OnceLock<Option<VisibleArea>>,
    unused_constructor_fields: [f32; 3],
}

impl Default for ViewUtilities {
    fn default() -> Self {
        Self {
            renderer_present: false,
            camera_present: false,
            valid: false,
            city: (0.0, 0.0),
            camera: CameraState::default(),
            angles: ViewAngles::default(),
            rotation_yaw: 0.0,
            forward: Vec3::ZERO,
            right: Vec3::ZERO,
            up: Vec3::ZERO,
            sin_negative_yaw: 0.0,
            cos_negative_yaw: 0.0,
            sin_pitch: 0.0,
            cos_pitch: 0.0,
            scales: Vec3::ZERO,
            inverse_scales: Vec3::ZERO,
            view: Frustum::default(),
            draw: Frustum::default(),
            draw_narrowed: false,
            draw_rect: PixelRect::default(),
            viewport_ground: OnceLock::new(),
            visible_area: OnceLock::new(),
            unused_constructor_fields: [1.0, 1.0, 0.3],
        }
    }
}

impl ViewUtilities {
    pub fn new() -> Self {
        Self::default()
    }

    /// Rust input setters, not attributed native methods (VU4). The renderer
    /// supplies city metres before the first update; construction stays zero.
    pub fn set_renderer_present(&mut self, present: bool) {
        self.renderer_present = present;
    }
    pub fn set_city_size(&mut self, width: f32, depth: f32) {
        self.city = (width, depth);
    }
    pub fn is_valid(&self) -> bool {
        self.valid
    }
    pub fn camera(&self) -> &CameraState {
        &self.camera
    }
    pub fn angles(&self) -> ViewAngles {
        self.angles
    }
    pub fn basis(&self) -> (Vec3, Vec3, Vec3) {
        (self.forward, self.right, self.up)
    }
    pub fn scales(&self) -> (Vec3, Vec3) {
        (self.scales, self.inverse_scales)
    }
    pub fn view_frustum(&self) -> Frustum {
        let mut view = self.view;
        if let Some(Some(points)) = self.viewport_ground.get() {
            view.ground_points = *points;
        }
        view
    }
    pub fn draw_frustum(&self) -> &Frustum {
        &self.draw
    }
    pub fn draw_rect(&self) -> PixelRect {
        self.draw_rect
    }
    pub fn draw_is_narrowed(&self) -> bool {
        self.draw_narrowed
    }
    pub fn get_zoom_and_rotation(&self) -> (u32, i32) {
        (self.camera.zoom, self.camera.rotation)
    }
    pub fn viewport_units_to_world_units(&self, value: f32) -> f32 {
        value * self.camera.metres_per_pixel
    }

    pub fn update_camera_position(&mut self, camera: Option<&CameraState>) -> bool {
        self.visible_area.take();
        self.viewport_ground.take();
        self.valid = false;
        self.camera_present = camera.is_some();
        if !self.renderer_present {
            return false;
        }
        let Some(camera) = camera else { return false };
        let Some(angles) = get_camera_view_angles_for_rotation(camera.rotation, camera.zoom) else {
            return false;
        };
        self.camera = *camera;
        let v = camera.volume;
        let depth_scale = 1.0 / (f64::from(v.far) - f64::from(v.near));
        self.scales = Vec3::new(
            (f64::from(camera.viewport_width) / (f64::from(v.right) - f64::from(v.left))) as f32,
            (f64::from(camera.viewport_height) / (f64::from(v.top) - f64::from(v.bottom))) as f32,
            depth_scale as f32,
        );
        self.inverse_scales = Vec3::new(1.0 / self.scales.x, 1.0 / self.scales.y, (1.0 / depth_scale) as f32);
        self.angles = angles;
        self.rotation_yaw = -get_object_y_rotation_angle(camera.rotation);
        self.sin_negative_yaw = (-f64::from(angles.yaw)).sin() as f32;
        self.cos_negative_yaw = (-f64::from(angles.yaw)).cos() as f32;
        self.sin_pitch = f64::from(angles.pitch).sin() as f32;
        self.cos_pitch = f64::from(angles.pitch).cos() as f32;
        self.forward = (camera.look_at - camera.position).normalized();
        let right = self.forward.cross(Vec3::new(0.0, 1.0, 0.0));
        self.up = right.cross(self.forward).normalized();
        self.right = right.normalized();
        self.view = Frustum::build(
            camera.position,
            (self.forward, self.right, self.up),
            v,
            camera.projection,
            self.city,
            &self.view.corners,
        );
        self.valid = true;
        true
    }

    pub fn project_point_to_fixed_view_coordinates(&self, p: Vec3) -> Vec3 {
        let x = p.x * self.cos_negative_yaw - p.z * self.sin_negative_yaw;
        let z = p.x * self.sin_negative_yaw + p.z * self.cos_negative_yaw;
        Vec3::new(
            x,
            p.y * self.cos_pitch - z * self.sin_pitch,
            p.y * self.sin_pitch + z * self.cos_pitch,
        )
    }
    pub fn project_point_to_view_coordinates(&self, p: Vec3) -> Vec3 {
        self.project_point_to_fixed_view_coordinates(p - self.camera.position)
    }
    pub fn project_point_from_view_coordinates_to_viewport_coordinates(&self, p: Vec3) -> Vec3 {
        Vec3::new(
            (p.x - self.camera.volume.left) * self.scales.x,
            (self.camera.volume.top - p.y) * self.scales.y,
            (p.z + self.camera.volume.near) * self.scales.z,
        )
    }
    pub fn project_point_to_viewport_coordinates(&self, p: Vec3) -> Vec3 {
        self.project_point_from_view_coordinates_to_viewport_coordinates(self.project_point_to_view_coordinates(p))
    }
    pub fn project_point_to_viewport_coordinates_2d(&self, p: Vec3) -> Vec3 {
        let v = self.project_point_to_view_coordinates(p);
        let mut out = self.project_point_from_view_coordinates_to_viewport_coordinates(v);
        out.z = v.z;
        out
    }
    pub fn project_point_to_viewport_z(&self, p: Vec3) -> f32 {
        (self.project_point_to_view_coordinates(p).z + self.camera.volume.near) * self.scales.z
    }
    pub fn unproject_point_from_view_coordinates_to_world_coordinates(&self, p: Vec3) -> Vec3 {
        let y = p.y * self.cos_pitch + p.z * self.sin_pitch;
        let z = -p.y * self.sin_pitch + p.z * self.cos_pitch;
        self.camera.position
            + Vec3::new(
                p.x * self.cos_negative_yaw + z * self.sin_negative_yaw,
                y,
                -p.x * self.sin_negative_yaw + z * self.cos_negative_yaw,
            )
    }
    pub fn unproject_point_from_viewport_coordinates_to_world_coordinates(&self, p: Vec3) -> Vec3 {
        self.unproject_point_from_view_coordinates_to_world_coordinates(Vec3::new(
            p.x * self.inverse_scales.x + self.camera.volume.left,
            self.camera.volume.top - p.y * self.inverse_scales.y,
            p.z * self.inverse_scales.z - self.camera.volume.near,
        ))
    }
    pub fn project_point_from_viewport_coordinates_to_fixed_view_coordinates(&self, p: Vec3) -> Vec3 {
        self.project_point_to_fixed_view_coordinates(
            self.unproject_point_from_viewport_coordinates_to_world_coordinates(p),
        )
    }
    pub fn is_point_in_view_frustum(&self, p: Vec3) -> bool {
        let p = self.project_point_to_view_coordinates(p);
        let v = self.camera.volume;
        v.left <= p.x && p.x <= v.right && v.bottom <= p.y && p.y <= v.top && v.near < -p.z && -p.z < v.far
    }
    pub fn is_bounding_box_in_view_frustum(&self, bounds: Aabb) -> bool {
        self.view.contains_box(bounds)
    }
    pub fn is_bounding_box_in_draw_frustum(&self, bounds: Aabb) -> bool {
        self.draw.contains_box(bounds)
    }
    pub fn get_frustum_aabb_city_clipped(&self, view_frustum: bool) -> Option<Aabb> {
        if view_frustum {
            self.view.city_clipped_bounds
        } else {
            self.draw.city_clipped_bounds
        }
    }

    /// Invalid rotation leaves the caller's rectangle untouched (§4).
    pub fn project_bounding_box_to_viewport_rectangle(&self, b: Aabb, out: &mut PixelRect) -> bool {
        let corner = |x, y, z| {
            Vec3::new(
                if x { b.max.x } else { b.min.x },
                if y { b.max.y } else { b.min.y },
                if z { b.max.z } else { b.min.z },
            )
        };
        let choices = match self.camera.rotation {
            0 => [
                (false, true, true),
                (false, true, false),
                (true, true, false),
                (true, false, true),
            ],
            1 => [
                (false, true, false),
                (true, true, false),
                (true, true, true),
                (false, false, true),
            ],
            2 => [
                (true, true, false),
                (true, true, true),
                (false, true, true),
                (false, false, false),
            ],
            3 => [
                (true, true, true),
                (false, true, true),
                (false, true, false),
                (true, false, false),
            ],
            _ => return false,
        };
        let p = choices.map(|(x, y, z)| self.project_point_to_viewport_coordinates_2d(corner(x, y, z)));
        // §4.4 float-bit bias conversion, including its out-of-range results.
        *out = PixelRect::new(
            crate::x87::rhe(p[0].x - 0.5),
            crate::x87::rhe(p[1].y - 0.5),
            crate::x87::rhe(p[2].x),
            crate::x87::rhe(p[3].y),
        );
        true
    }

    pub fn viewport_position_to_ray(
        &self,
        camera: Option<&dyn CameraRayProvider>,
        x: f32,
        y: f32,
        out: &mut Ray,
    ) -> bool {
        if !self.renderer_present || !self.camera_present {
            return false;
        }
        let Some(ray) = camera.and_then(|camera| camera.eye_ray(x, y)) else {
            return false;
        };
        *out = Ray {
            origin: ray.origin,
            direction: ray.direction.normalized(),
        };
        true
    }
    pub fn get_viewport_point_intersection_with_ground_plane(
        &self,
        camera: Option<&dyn CameraRayProvider>,
        x: f32,
        y: f32,
        out: &mut Vec3,
    ) -> bool {
        // Checked portable boundary: native failure consumes uninitialized stack
        // words. Preserve out and return false; no successful coordinate invented.
        let mut ray = Ray::default();
        if !self.viewport_position_to_ray(camera, x, y, &mut ray) {
            return false;
        }
        *out = get_ray_intersection_with_ground_plane(ray);
        true
    }
    pub fn get_viewport_rect_ground_corners(
        &self,
        camera: Option<&dyn CameraRayProvider>,
        rect: PixelRect,
    ) -> Option<[Vec3; 4]> {
        let a = rect.top_left;
        let b = rect.bottom_right;
        let mut points = [Vec3::ZERO; 4];
        for (i, (x, y)) in [(a.x, a.y), (b.x, a.y), (a.x, b.y), (b.x, b.y)].into_iter().enumerate() {
            let mut p = Vec3::ZERO;
            if !self.get_viewport_point_intersection_with_ground_plane(camera, x as f32, y as f32, &mut p) {
                return None;
            }
            points[i] = Vec3::new(p.x, 0.0, p.z);
        }
        Some(points)
    }
    pub fn get_city_corner_position(&self, corner: u32, out: &mut Vec3) -> bool {
        out.y = 0.0;
        let (w, d) = self.city;
        let rows = [
            [(0.0, d), (0.0, 0.0), (w, 0.0), (w, d)],
            [(0.0, 0.0), (w, 0.0), (w, d), (0.0, d)],
            [(w, 0.0), (w, w), (0.0, d), (0.0, 0.0)],
            [(w, d), (0.0, d), (0.0, 0.0), (w, 0.0)],
        ];
        let Some(row) = rows.get(corner as usize) else {
            return false;
        };
        let Some(&(x, z)) = row.get(self.camera.rotation as usize) else {
            return false;
        };
        out.x = x;
        out.z = z;
        true
    }
    pub fn get_visible_edges(&self) -> (i32, i32) {
        (
            self.camera.rotation.wrapping_add(1) % 4,
            self.camera.rotation.wrapping_add(2) % 4,
        )
    }
    pub fn clip_position_to_city_2d(&self, x: &mut f32, z: &mut f32) {
        let clip = |v: f32, size: f32| {
            if v < 0.0 {
                0.0
            } else if v >= size {
                size - 1.0
            } else {
                v
            }
        };
        *x = clip(*x, self.city.0);
        *z = clip(*z, self.city.1);
    }
    pub fn clip_world_position_to_city_position(&self, p: &mut Vec3) {
        self.clip_position_to_city_2d(&mut p.x, &mut p.z);
    }

    pub fn build_draw_frustum_data_from_viewport_rect(
        &mut self,
        rect: PixelRect,
        camera: Option<&dyn CameraRayProvider>,
    ) -> bool {
        self.visible_area.take();
        self.viewport_ground.take();
        self.draw_rect = rect;
        let full = PixelRect::new(0, 0, self.camera.viewport_width, self.camera.viewport_height);
        if rect == full {
            self.draw_narrowed = false;
            let previous_ground = self.draw.ground_points;
            self.draw = self.view;
            self.draw.ground_points = previous_ground;
            let points = *self
                .viewport_ground
                .get_or_init(|| self.get_viewport_rect_ground_corners(camera, full));
            let Some(points) = points else { return false };
            self.draw.ground_points = points;
            return true;
        }
        // §5 null renderer faults natively; null camera retains previous draw
        // flag/geometry. Checked policy rejects both after the entry stores.
        if !self.renderer_present || !self.camera_present || camera.is_none() {
            return false;
        }
        self.draw_narrowed = true;
        let c = rect.center();
        let factor = self.camera.metres_per_pixel;
        let mut volume = self.camera.volume;
        if rect.top_left.x != 0 || rect.bottom_right.x != self.camera.viewport_width {
            volume.left = rect.top_left.x.wrapping_sub(c.x) as f32 * factor;
            volume.right = rect.bottom_right.x.wrapping_sub(c.x) as f32 * factor;
        }
        if rect.top_left.y != 0 || rect.bottom_right.y != self.camera.viewport_height {
            volume.top = c.y.wrapping_sub(rect.top_left.y) as f32 * factor;
            volume.bottom = c.y.wrapping_sub(rect.bottom_right.y) as f32 * factor;
        }
        let Some(points) = self.get_viewport_rect_ground_corners(camera, rect) else {
            return false;
        };
        let position = self
            .unproject_point_from_viewport_coordinates_to_world_coordinates(Vec3::new(c.x as f32, c.y as f32, 0.0))
            - self.forward * volume.near;
        self.draw = Frustum::build(
            position,
            (self.forward, self.right, self.up),
            volume,
            self.camera.projection,
            self.city,
            &self.draw.corners,
        );
        self.draw.ground_points = points;
        true
    }

    pub fn get_ground_plane_visible_area_rectangle(
        &self,
        camera: Option<&dyn CameraRayProvider>,
    ) -> Option<VisibleArea> {
        *self.visible_area.get_or_init(|| {
            let full = PixelRect::new(0, 0, self.camera.viewport_width, self.camera.viewport_height);
            let p = self.get_viewport_rect_ground_corners(camera, full)?;
            // §6 explicitly says viewport corners, not dirty-rectangle corners.
            let inner = Aabb::from_points(&[(p[0] + p[1]) * 0.5, (p[2] + p[3]) * 0.5]);
            let bounds = Aabb {
                min: Vec3::new(
                    min_of_four_floats(p.map(|p| p.x)),
                    0.0,
                    min_of_four_floats(p.map(|p| p.z)),
                ),
                max: Vec3::new(
                    max_of_four_floats(p.map(|p| p.x)),
                    0.0,
                    max_of_four_floats(p.map(|p| p.z)),
                ),
            };
            let outer = Aabb {
                min: Vec3::new(
                    round_down_to_nearest_block_boundary(bounds.min.x, 64)? as f32,
                    0.0,
                    round_down_to_nearest_block_boundary(bounds.min.z, 64)? as f32,
                ),
                max: Vec3::new(
                    round_up_to_nearest_block_boundary(bounds.max.x, 64)? as f32,
                    0.0,
                    round_up_to_nearest_block_boundary(bounds.max.z, 64)? as f32,
                ),
            };
            let area = VisibleArea { inner, outer };
            Some(area)
        })
    }
}

#[cfg(test)]
mod tests;
