// SPDX-License-Identifier: GPL-3.0-or-later
use super::{ray_intersects_ground_plane, Aabb, Ray, Vec3, ViewVolume};

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
    pub city_clipped_bounds: Option<Aabb>,
    pub volume: ViewVolume,
    pub planes: [Plane; 6],
    pub reduced_planes: [Vec3; 6],
    pub ground_lines: [GroundLine; 2],
    pub ground_points: [Vec3; 4],
}

impl Frustum {
    pub(crate) fn build(
        position: Vec3,
        basis: (Vec3, Vec3, Vec3),
        volume: ViewVolume,
        projection: Projection,
        city: (f32, f32),
        previous: &[Vec3; 8],
    ) -> Self {
        let (forward, right, up) = basis;
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
            // §3.1 native quirk: expand previous far corners, not current near.
            let ratio = volume.far / volume.near;
            for i in 0..4 {
                corners[i + 4] = position + (previous[i + 4] - position) * ratio;
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
            reduced_planes: planes.map(|p| Vec3::new(p.normal.x, p.normal.z, p.offset)),
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

pub(crate) fn city_clipped_bounds(corners: &[Vec3; 8], (width, depth): (f32, f32)) -> Option<Aabb> {
    let mut a = Vec3::ZERO;
    let mut b = Vec3::ZERO;
    ray_intersects_ground_plane(
        Ray {
            origin: corners[0],
            direction: corners[4] - corners[0],
        },
        &mut a,
    );
    ray_intersects_ground_plane(
        Ray {
            origin: corners[3],
            direction: corners[7] - corners[3],
        },
        &mut b,
    );
    let polygon = clip_polygon_to_polygon_2d(
        &[corners[3], corners[0], a, b],
        &[
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, depth),
            Vec3::new(width, 0.0, depth),
            Vec3::new(width, 0.0, 0.0),
        ],
        10,
    )
    .ok()?;
    let mut bounds = Aabb::from_points(&polygon);
    bounds.min.y = 0.0;
    bounds.max.y = corners[3].y;
    Some(bounds)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndeterminateClipIntersection;

/// §7.2 ordered clipping in x/z. No deduplication or winding repair. The native
/// unchecked near-parallel intersection is exposed as an error, not stale data.
pub fn clip_polygon_to_polygon_2d(
    subject: &[Vec3],
    clip: &[Vec3],
    capacity: usize,
) -> Result<Vec<Vec3>, IndeterminateClipIntersection> {
    let mut polygon = subject.to_vec();
    let Some(&last_clip) = clip.last() else {
        return Ok(polygon);
    };
    let mut edge_start = last_clip;
    for &edge_end in clip {
        let input = std::mem::take(&mut polygon);
        let Some(&last) = input.last() else { break };
        let edge = edge_end - edge_start;
        let side = |p: Vec3| (p.x - edge_start.x) * edge.z - (p.z - edge_start.z) * edge.x;
        let mut previous = last;
        for current in input {
            let previous_inside = side(previous) >= 0.0;
            let current_inside = side(current) >= 0.0;
            if previous_inside != current_inside {
                let delta = current - previous;
                let determinant = delta.x * edge.z - delta.z * edge.x;
                if determinant.abs() < super::math::GEOMETRY_TOLERANCE {
                    return Err(IndeterminateClipIntersection);
                }
                let t = -side(previous) / determinant;
                let point = previous + delta * t;
                if polygon.len() < capacity.min(40) {
                    polygon.push(Vec3::new(point.x, 0.0, point.z));
                }
            }
            if current_inside && polygon.len() < capacity.min(40) {
                polygon.push(current);
            }
            previous = current;
        }
        edge_start = edge_end;
    }
    Ok(polygon)
}
