// SPDX-License-Identifier: GPL-3.0-or-later
//! Visibility, model picking and the clipped terrain polyline (render3d §11).
use super::{ContainerServices, GridElement, Handle, References, Render3d, WorldBox};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ray {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickRecord {
    pub ray: Ray,
    pub screen: [i32; 2],
    pub filter: Option<Handle>,
    pub max_distance: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainTarget {
    Base(Handle),
    Extended(Handle),
}

pub trait PickingServices: References + ContainerServices {
    fn bounding_box_in_frustum(&mut self, bounds: WorldBox, view: bool) -> bool;
    fn eye_ray(&mut self, camera: Handle, screen: [f32; 2], out: &mut Ray) -> bool;
    /// QI for 0x6771477D; a successful result owns one interface reference.
    fn query_terrain_view(&mut self, view: Handle) -> Option<Handle>;
    fn terrain_hit(
        &mut self,
        target: TerrainTarget,
        ray: Ray,
        context: Option<Handle>,
        out: &mut [f32; 3],
        extra: u32,
    ) -> bool;
    fn ground_plane_hit(&mut self, ray: Ray, out: &mut [f32; 3]) -> bool;
    fn terrain_size(&mut self, terrain: Handle) -> [i32; 2];
    /// Model slot 3 returns a borrowed object, not an acquired reference.
    fn model_object(&mut self, model: Handle) -> Option<Handle>;
    /// QI for 0x26BB767E; a successful result owns one interface reference.
    fn query_filter_object(&mut self, object: Handle) -> Option<Handle>;
    fn filter_accepts(&mut self, filter: Handle, object: Handle) -> bool;
    fn model_hit_test(&mut self, model: Handle, record: &mut PickRecord) -> bool;
}

fn normalized(mut ray: Ray) -> Ray {
    let [x, y, z] = ray.direction.map(f64::from);
    // SPEC-GAP: f64 approximates the specified extended-precision vector operations.
    let reciprocal = 1.0 / (y * y + x * x + z * z).sqrt();
    ray.direction = [x, y, z].map(|v| (v * reciprocal) as f32);
    ray
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| (a as f64 - b as f64).powi(2))
        .sum::<f64>()
        .sqrt() as f32
}

impl Render3d {
    pub fn is_model_visible(&self, host: &mut impl PickingServices, model: Handle, frustum_kind: i32) -> bool {
        let mut bounds = WorldBox::default();
        host.model_world_box(model, &mut bounds);
        host.bounding_box_in_frustum(bounds, frustum_kind == 2)
    }

    pub fn is_grid_element_visible(
        &self,
        host: &mut impl PickingServices,
        element: &GridElement,
        frustum_kind: i32,
    ) -> bool {
        self.is_model_visible(host, element.model, frustum_kind)
    }

    pub fn pick_terrain_ray(
        &self,
        host: &mut impl PickingServices,
        ray: Ray,
        out: &mut [f32; 3],
        fallback: bool,
        extra: u32,
    ) -> bool {
        let Some(view) = self.objects.terrain_view else {
            return false;
        };
        let acquired = host.query_terrain_view(view);
        let target = acquired.map_or(TerrainTarget::Base(view), TerrainTarget::Extended);
        let forwarded_extra = if acquired.is_some() { extra } else { 0 };
        let mut hit = host.terrain_hit(target, ray, self.context, out, forwarded_extra);
        if !hit {
            let mut raised = ray;
            raised.origin[1] += 4.0;
            hit = host.terrain_hit(target, raised, self.context, out, forwarded_extra);
        }
        if !hit && fallback {
            hit = host.ground_plane_hit(ray, out);
        }
        if let Some(acquired) = acquired {
            host.release(acquired);
        }
        hit
    }

    pub fn pick_terrain(
        &self,
        host: &mut impl PickingServices,
        screen: [i32; 2],
        out: &mut [f32; 3],
        fallback: bool,
        extra: u32,
    ) -> bool {
        let Some(camera) = self.camera.filter(|_| self.objects.terrain_view.is_some()) else {
            return false;
        };
        let mut ray = Ray::default();
        if !host.eye_ray(camera, screen.map(|v| v as f32), &mut ray) {
            return false;
        }
        self.pick_terrain_ray(host, normalized(ray), out, fallback, extra)
    }

    pub fn pick(
        &mut self,
        host: &mut impl PickingServices,
        screen: [i32; 2],
        filter: Option<Handle>,
        out: &mut Option<Handle>,
    ) -> bool {
        let Some(camera) = self.camera else {
            return false;
        };
        let mut ray = Ray::default();
        host.eye_ray(camera, screen.map(|v| v as f32), &mut ray);
        self.pick_ray(host, screen, ray, filter, out)
    }

    pub fn pick_ray(
        &mut self,
        host: &mut impl PickingServices,
        screen: [i32; 2],
        mut ray: Ray,
        filter: Option<Handle>,
        out: &mut Option<Handle>,
    ) -> bool {
        *out = None;
        let mut candidates = Vec::new();
        let rect = self.viewport_ray_container_rect(host, screen[0], screen[1]);
        if let Some(grid) = self.grid.as_mut() {
            let cells = grid.cells_for_rect(rect);
            // SPEC-GAP: cell-list enumeration uses the grid's row/column traversal order.
            for y in cells.top.max(0)..=cells.bottom.min(grid.side() - 1) {
                for x in cells.left.max(0)..=cells.right.min(grid.side() - 1) {
                    for model in grid.quadrant(x, y).1.to_vec() {
                        let element = grid.element_mut(model).unwrap();
                        if element.flags & 2 == 0 {
                            element.flags |= 2;
                            candidates.push(model);
                        }
                    }
                }
            }
        }
        let mut max_distance = f32::MAX;
        if let Some(view) = self.objects.terrain_view {
            let mut hit = [0.0; 3];
            if host.terrain_hit(TerrainTarget::Base(view), ray, self.context, &mut hit, 0) {
                max_distance = distance(hit, ray.origin);
            } else {
                ray.origin[1] += 4.0;
                if host.terrain_hit(TerrainTarget::Base(view), ray, self.context, &mut hit, 0) {
                    max_distance = distance(hit, ray.origin);
                }
                ray.origin[1] -= 4.0;
            }
        }
        let mut record = PickRecord {
            ray,
            screen,
            filter,
            max_distance,
        };
        let mut result = false;
        for model in candidates {
            let accepted = if let Some(filter) = filter {
                if let Some(acquired) = host.model_object(model).and_then(|o| host.query_filter_object(o)) {
                    let accepts = host.filter_accepts(filter, acquired);
                    host.release(acquired);
                    accepts
                } else {
                    false
                }
            } else {
                true
            };
            if accepted && host.model_hit_test(model, &mut record) {
                result = true;
                if let Some(old) = out.take() {
                    host.release(old);
                }
                *out = Some(model);
                host.add_ref(model);
            }
            self.grid.as_mut().unwrap().element_mut(model).unwrap().flags &= !2;
        }
        result
    }

    pub fn pick_terrain_always(
        &self,
        host: &mut impl PickingServices,
        screen: [i32; 2],
        out: &mut [f32; 3],
        extra: u32,
    ) -> bool {
        let Some(camera) = self.camera.filter(|_| self.objects.terrain_view.is_some()) else {
            return false;
        };
        let mut ray = Ray::default();
        if !host.eye_ray(camera, screen.map(|v| v as f32), &mut ray) {
            return false;
        }
        self.pick_terrain_always_ray(host, normalized(ray), out, extra)
    }

    pub fn pick_terrain_always_ray(
        &self,
        host: &mut impl PickingServices,
        ray: Ray,
        out: &mut [f32; 3],
        extra: u32,
    ) -> bool {
        let Some(terrain) = self.objects.terrain else {
            return false;
        };
        if self.pick_terrain_ray(host, ray, out, false, extra) {
            return true;
        }
        if ray.direction[1] as f64 > -1e-5 {
            return false;
        }
        let points = terrain_polyline(ray, host.terrain_size(terrain));
        for index in 0..points.len() - 1 {
            let start = points[index];
            let end = points[index + 1];
            let segment = Ray {
                origin: [start[0], start[1], start[2]],
                direction: std::array::from_fn(|i| end[i] - start[i]),
            };
            let last = index == points.len() - 2;
            if !self.pick_terrain_ray(host, segment, out, last, extra) {
                continue;
            }
            let beyond = (0..3)
                .map(|i| (out[i] as f64 - end[i] as f64) * segment.direction[i] as f64)
                .sum::<f64>();
            if last || beyond <= 0.0 {
                let t = (out[1] as f64 - ray.origin[1] as f64) / ray.direction[1] as f64;
                for i in [0, 2] {
                    out[i] = (t * ray.direction[i] as f64 + ray.origin[i] as f64) as f32;
                }
                return true;
            }
        }
        false
    }
}

fn plane_distance(p: [f32; 4], plane: [f32; 4]) -> f32 {
    let [x, y, z, w] = p.map(f64::from);
    let [a, b, c, d] = plane.map(f64::from);
    (((w * d + z * c) + y * b) + x * a) as f32
}

fn project_plane(p: &mut [f32; 4], plane: [f32; 4]) {
    let [x, y, z, w] = p.map(f64::from);
    let [a, b, c, d] = plane.map(f64::from);
    let s = (((a * x + c * z) + b * y) + w * 0.0) + d;
    for (value, normal) in p[..3].iter_mut().zip(plane) {
        *value = (*value as f64 - normal as f64 * s) as f32;
    }
}

fn crossing(first: [f32; 4], second: [f32; 4], d0: f32, d1: f32) -> [f32; 4] {
    let t = d0 as f64 / (d0 as f64 - d1 as f64);
    std::array::from_fn(|i| (first[i] as f64 * (1.0 - t) + second[i] as f64 * t) as f32)
}

pub(super) fn terrain_polyline(ray: Ray, [width, depth]: [i32; 2]) -> Vec<[f32; 4]> {
    let t = ray.origin[1] as f64 / ray.direction[1] as f64;
    let mut points = vec![
        [ray.origin[0], ray.origin[1], ray.origin[2], 1.0],
        [
            (ray.origin[0] as f64 - ray.direction[0] as f64 * t) as f32,
            (ray.origin[1] as f64 - ray.direction[1] as f64 * t) as f32,
            (ray.origin[2] as f64 - ray.direction[2] as f64 * t) as f32,
            1.0,
        ],
    ];
    let planes = [
        [1.0, 0.0, 0.0, -0.01_f32],
        [0.0, 0.0, 1.0, -0.01_f32],
        [-1.0, 0.0, 0.0, (16.0 * width as f64 - 0.01_f32 as f64) as f32],
        [0.0, 0.0, -1.0, (16.0 * depth as f64 - 0.01_f32 as f64) as f32],
    ];
    for plane in planes {
        let mut index = 0;
        while index + 1 < points.len() {
            let first = points[index];
            let second = points[index + 1];
            let d0 = plane_distance(first, plane);
            let d1 = plane_distance(second, plane);
            if d0 >= 0.0 {
                if d1 < 0.0 {
                    points.insert(index + 1, crossing(first, second, d0, d1));
                    index += 1;
                }
            } else {
                project_plane(&mut points[index], plane);
                if d1 > 0.0 {
                    points.insert(index + 1, crossing(first, second, d0, d1));
                    break;
                }
            }
            index += 1;
        }
        let last = points.last_mut().unwrap();
        if plane_distance(*last, plane) < 0.0 {
            project_plane(last, plane);
        }
    }
    points
}
