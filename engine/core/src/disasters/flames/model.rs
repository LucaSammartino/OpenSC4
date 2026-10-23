// SPDX-License-Identifier: GPL-3.0-or-later
use super::numeric;
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
    fn new(vertices: [Vec3; 3], previous_area: f32, transform: Xform) -> Result<Self, PlacementError> {
        let (normal, cumulative_area) = numeric::triangle(vertices, previous_area)?;
        let (wall, side) = classify(normal);
        Ok(Self {
            vertices,
            normal,
            cumulative_area,
            wall,
            side,
            transform,
        })
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
            let range = mesh
                .first_primitive()
                .ok_or(PlacementError::Unsupported(Unsupported::UnsafeIndex))?;
            let count = range
                .end
                .checked_sub(range.start)
                .filter(|count| u32::try_from(*count).is_ok())
                .ok_or(PlacementError::Unsupported(Unsupported::UnsafeIndex))?;
            // FF3: ceil(k/3), NOT chunks_exact: a partial final triple reads
            // trailing physical indices if they exist, otherwise an error.
            for first in (0..count).step_by(3) {
                let mut vertices = [[0.0; 3]; 3];
                for (offset, vertex) in vertices.iter_mut().enumerate() {
                    let physical = first
                        .checked_add(offset)
                        .ok_or(PlacementError::Unsupported(Unsupported::UnsafeIndex))?;
                    let index = mesh
                        .index(physical)
                        .ok_or(PlacementError::Unsupported(Unsupported::UnsafeIndex))?;
                    if u16::try_from(index).is_err() {
                        return Err(PlacementError::Unsupported(Unsupported::UnsafeVertex));
                    }
                    *vertex = mesh
                        .vertex(index)
                        .ok_or(PlacementError::Unsupported(Unsupported::UnsafeVertex))?;
                }
                let triangle = Triangle::new(vertices, total, *transform)?;
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
        let r = numeric::range(random.uniform_f64(), 0.0, numeric::input(total)?)?;
        let triangle = triangles
            .iter()
            // Unordered would stop natively too. Nonfinite geometry/draws are
            // rejected by the qualified numeric boundary before reaching here.
            .find(|triangle| {
                !matches!(
                    f64::from(triangle.cumulative_area).partial_cmp(&r),
                    Some(std::cmp::Ordering::Less)
                )
            })
            .ok_or(PlacementError::Unsupported(Unsupported::SelectionOnePast))?;
        let u = random.uniform_f64();
        let second = random.uniform_f64();
        points.push(ModelPoint {
            position: numeric::place(triangle.vertices, triangle.normal, &triangle.transform, u, second)?,
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
    fn all_ordered_less_is_unsupported_after_only_the_area_draw() {
        let seeds = FixedSeeds {
            wall_clock: 1,
            process_clock: 0,
        };
        let mut random = RzRandom::new(1, &seeds);
        let mut triangle = Triangle::new([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]], 0., Xform::default()).unwrap();
        triangle.cumulative_area = 0.;
        assert_eq!(
            sample(1, &[triangle], 2., &mut random),
            Err(PlacementError::Unsupported(Unsupported::SelectionOnePast))
        );
        // Independent recurrence seed1 * 0x278dde6d; no u/v draw or retry.
        assert_eq!(random.mul_state(), 0x278dde6d);
    }

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
        )
        .unwrap();
        first.cumulative_area = 1.0;
        let mut second = Triangle::new(first.vertices, 1.0, Xform::default()).unwrap();
        second.transform.translation = [100.0, 0.0, 0.0];
        let triangles = [first, second];
        let point = sample(1, &triangles, 2.0, &mut random).unwrap()[0];
        assert!(
            point.position[0] < 2.0,
            "an equal-area boundary belongs to the first triangle"
        );
    }
}
