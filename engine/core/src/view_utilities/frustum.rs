// SPDX-License-Identifier: GPL-3.0-or-later
use super::{get_ray_intersection_with_ground_plane, Aabb, Ray, Vec3, ViewVolume};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub offset: f32,
}

impl Plane {
    fn through(normal: Vec3, point: Vec3) -> Self {
        Self {
            normal,
            offset: -normal.dot(point),
        }
    }
    pub fn evaluate(self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.offset
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GroundLine {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl GroundLine {
    pub fn evaluate(self, point: Vec3) -> f32 {
        (point.x - self.origin.x) * self.direction.z - (point.z - self.origin.z) * self.direction.x
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Projection {
    #[default]
    Orthographic,
    Perspective,
}

/// Owned, immutable-to-read result of §§3,5. Planes: right, left, bottom,
/// top, near, far. Ground points are TL,TR,BL,BR.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Frustum {
    pub corners: [Vec3; 8],
    pub bounds: Aabb,
    pub city_clipped_bounds: Aabb,
    pub volume: ViewVolume,
    pub planes: [Plane; 6],
    pub ground_lines: [GroundLine; 2],
    pub ground_points: [Vec3; 4],
}

impl Frustum {
    pub(crate) fn build(
        position: Vec3,
        forward: Vec3,
        right: Vec3,
        up: Vec3,
        volume: ViewVolume,
        projection: Projection,
        city: (f32, f32),
    ) -> Self {
        let n = position + forward * volume.near;
        let f = position + forward * volume.far;
        let h = right * ((volume.right - volume.left) / 2.0);
        let v = up * ((volume.top - volume.bottom) / 2.0);
        let mut corners = [
            n + h + v,
            n + h - v,
            n - h - v,
            n - h + v,
            f + h + v,
            f + h - v,
            f - h - v,
            f - h + v,
        ];
        if projection == Projection::Perspective {
            // Explicit spec §3.1 choice: derive from current near corners, never
            // compound stale far corners over successive camera updates.
            let ratio = volume.far / volume.near;
            for i in 0..4 {
                corners[i + 4] = position + (corners[i] - position) * ratio;
            }
        }
        let planes = [
            Plane::through(right, corners[3]),
            Plane::through(-right, corners[0]),
            Plane::through(up, corners[2]),
            Plane::through(-up, corners[3]),
            Plane::through(forward, corners[3]),
            Plane::through(-forward, corners[7]),
        ];
        let ground_lines = [
            GroundLine {
                origin: corners[3],
                direction: corners[7] - corners[3],
            },
            GroundLine {
                origin: corners[0],
                direction: corners[4] - corners[0],
            },
        ];
        Self {
            corners,
            bounds: Aabb::from_points(&corners),
            volume,
            planes,
            ground_lines,
            city_clipped_bounds: city_clipped_bounds(&corners, city),
            ground_points: [Vec3::ZERO; 4],
        }
    }

    /// §3.4 strict four-side test; intentionally no near/far-plane tests.
    pub fn contains_box(&self, bounds: Aabb) -> bool {
        let a = self.ground_lines[0];
        let b = self.ground_lines[1];
        a.evaluate(bounds.support(Vec3::new(-a.direction.z, 0.0, a.direction.x))) < 0.0
            && b.evaluate(bounds.support(Vec3::new(b.direction.z, 0.0, -b.direction.x))) > 0.0
            && self.planes[2].evaluate(bounds.support(self.planes[2].normal)) > 0.0
            && self.planes[3].evaluate(bounds.support(self.planes[3].normal)) > 0.0
    }
}

fn city_clipped_bounds(corners: &[Vec3; 8], (width, depth): (f32, f32)) -> Aabb {
    let a = get_ray_intersection_with_ground_plane(Ray {
        origin: corners[0],
        direction: corners[4] - corners[0],
    });
    let b = get_ray_intersection_with_ground_plane(Ray {
        origin: corners[3],
        direction: corners[7] - corners[3],
    });
    // VU6: the dependency's clipping arithmetic/ordering is not specified.
    // Literal convex quadrilateral perimeter 0,3,ground(3),ground(0), clipped
    // inclusively against x>=0,x<=W,z>=0,z<=D by Sutherland-Hodgman.
    let mut polygon = vec![corners[0], corners[3], b, a];
    for (axis, edge, keep_greater) in [(0, 0.0, true), (0, width, false), (2, 0.0, true), (2, depth, false)] {
        let input = std::mem::take(&mut polygon);
        let Some(&last) = input.last() else { break };
        let coordinate = |p: Vec3| if axis == 0 { p.x } else { p.z };
        let inside = |p: Vec3| {
            if keep_greater {
                coordinate(p) >= edge
            } else {
                coordinate(p) <= edge
            }
        };
        let mut previous = last;
        for current in input {
            if inside(previous) != inside(current) {
                let t = (edge - coordinate(previous)) / (coordinate(current) - coordinate(previous));
                let mut intersection = previous + (current - previous) * t;
                if axis == 0 {
                    intersection.x = edge
                } else {
                    intersection.z = edge
                }
                polygon.push(intersection);
            }
            if inside(current) {
                polygon.push(current);
            }
            previous = current;
        }
    }
    let mut bounds = Aabb::from_points(&polygon);
    bounds.min.y = 0.0;
    bounds.max.y = corners[3].y;
    bounds
}
