// SPDX-License-Identifier: GPL-3.0-or-later
use crate::camera::geometry::{add, cross, dot, mul, sub, transform};
use crate::camera::Vec3;
use crate::network_occupants::Xform;
use crate::rng::{RzRandom, SeedSource, CLOCK_SEED};
use std::ops::Range;

/// Raw foreign mesh access. Adapters can borrow formats::s3d data or a game
/// interface. They do not enumerate triangles or choose flame points.
pub trait Mesh {
    /// First primitive's reported range. Only its length is used: FF3 ignores
    /// the first-index offset. `index` exposes the entire readable backing,
    /// including complete triples extending past this reported count.
    fn first_primitive(&self) -> Option<Range<usize>>;
    fn index(&self, physical_index: usize) -> Option<usize>;
    fn vertex(&self, vertex_index: usize) -> Option<Vec3>;
}

/// Snapshot of the occupant's instance list / mesh-container interface query.
/// Borrowed data carries no invented AddRef/Release operations (FF2).
pub enum ModelInstance<'a> {
    Other,
    MeshContainer {
        transform: Xform,
        meshes: &'a [&'a dyn Mesh],
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelPoint {
    pub position: Vec3,
    pub wall: bool,
    pub side: u32,
}

/// Portable diagnostics, not newly invented native return values. FF2/FF3:
/// malformed mesh data, count overflow and initial-stage failures are unpublished.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementError {
    /// Outside the qualified portable domain, never a native false/fallback.
    Unsupported(Unsupported),
    MissingFirstPrimitive,
    MissingIndex,
    MissingVertex,
    NoSelectableTriangle,
    CountOverflow,
    AlreadyPopulated,
    UnpublishedInitialStage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    UnsafeIndex,
    UnsafeVertex,
    Numeric,
    DegenerateNormal,
    SelectionOnePast,
    HardwareTrig,
    EffectTable,
}

struct Triangle {
    vertices: [Vec3; 3],
    normal: Vec3,
    cumulative_area: f32,
    wall: bool,
    side: u32,
    transform: Xform,
}

/// §2's exact comparisons, including the equality boundaries and z wall flag.
pub(super) fn classify(normal: Vec3) -> (bool, u32) {
    let (x, z) = (normal[0], normal[2]);
    let side = if x.abs() >= z.abs() && x >= z {
        0
    } else if z.abs() > x.abs() && x >= z {
        1
    } else if x.abs() >= z.abs() && x < z {
        2
    } else {
        3
    };
    (z > 0.0, side)
}

impl Triangle {
    fn new(vertices: [Vec3; 3], previous_area: f32, transform: Xform) -> Self {
        let area_cross = cross(sub(vertices[0], vertices[1]), sub(vertices[2], vertices[1]));
        let twice_area = dot(area_cross, area_cross).sqrt();
        let normal_cross = cross(sub(vertices[1], vertices[0]), sub(vertices[2], vertices[0]));
        let length = dot(normal_cross, normal_cross).sqrt();
        // FF3: no invented degenerate-face filter or zero-normal fallback.
        let normal = normal_cross.map(|v| v / length);
        let (wall, side) = classify(normal);
        Self {
            vertices,
            normal,
            cumulative_area: previous_area + twice_area,
            wall,
            side,
            transform,
        }
    }
}

/// §2. None is the published false (no instances or no triangles). On success
/// there are exactly count entries, even when count=0. Each call constructs a
/// fresh clock-seeded generator; it never consumes the fire's persistent RNG.
pub fn get_points_on_model(
    count: u32,
    instances: &[ModelInstance<'_>],
    seeds: &dyn SeedSource,
) -> Result<Option<Vec<ModelPoint>>, PlacementError> {
    let mut triangles = Vec::new();
    let mut total = 0.0_f32;
    for instance in instances {
        let ModelInstance::MeshContainer { transform, meshes } = instance else {
            continue;
        };
        for mesh in *meshes {
            let range = mesh.first_primitive().ok_or(PlacementError::MissingFirstPrimitive)?;
            // FF3: ceil(k/3), NOT chunks_exact: a partial final triple reads
            // trailing physical indices if they exist, otherwise an error.
            for first in (0..range.len()).step_by(3) {
                let mut vertices = [[0.0; 3]; 3];
                for (offset, vertex) in vertices.iter_mut().enumerate() {
                    let physical = first.checked_add(offset)
                        .ok_or(PlacementError::Unsupported(Unsupported::UnsafeIndex))?;
                    let index = mesh.index(physical)
                        .ok_or(PlacementError::Unsupported(Unsupported::UnsafeIndex))?;
                    *vertex = mesh.vertex(index)
                        .ok_or(PlacementError::Unsupported(Unsupported::UnsafeVertex))?;
                }
                let triangle = Triangle::new(vertices, total, *transform);
                total = triangle.cumulative_area;
                triangles.push(triangle);
            }
        }
    }
    if triangles.is_empty() {
        return Ok(None);
    }
    let mut random = RzRandom::new(CLOCK_SEED, seeds);
    sample(count, &triangles, total, &mut random).map(Some)
}

fn sample(
    count: u32,
    triangles: &[Triangle],
    total: f32,
    random: &mut RzRandom,
) -> Result<Vec<ModelPoint>, PlacementError> {
    let mut points = Vec::new();
    for _ in 0..count {
        let r = random.range_f64(0.0, f64::from(total));
        let triangle = triangles
            .iter()
            .find(|triangle| f64::from(triangle.cumulative_area) >= r)
            .ok_or(PlacementError::NoSelectableTriangle)?;
        let u = random.uniform_f64() as f32;
        let v = random.range_f64(0.0, f64::from(1.0_f32 - u)) as f32;
        let w = (1.0_f32 - u) - v;
        let [a, b, c] = triangle.vertices;
        let local = add(add(add(mul(a, u), mul(b, v)), mul(c, w)), triangle.normal);
        points.push(ModelPoint {
            position: transform(&triangle.transform, local),
            wall: triangle.wall,
            side: triangle.side,
        });
    }
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::FixedSeeds;

    #[test]
    fn cumulative_boundary_and_zero_draw_choose_first_ge_not_next() {
        let seeds = FixedSeeds {
            wall_clock: 1,
            process_clock: 0,
        };
        let mut random = RzRandom::new(1, &seeds);
        // Zero state with zero process clock gives exactly 0.5, so r=1 of 2.
        random.set_state(1, 0, None);
        let mut first = Triangle::new(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            0.0,
            Xform::default(),
        );
        first.cumulative_area = 1.0;
        let mut second = Triangle::new(first.vertices, 1.0, Xform::default());
        second.transform.translation = [100.0, 0.0, 0.0];
        let triangles = [first, second];
        let point = sample(1, &triangles, 2.0, &mut random).unwrap()[0];
        assert!(
            point.position[0] < 2.0,
            "an equal-area boundary belongs to the first triangle"
        );
    }
}
