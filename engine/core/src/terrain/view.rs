// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Terrain-view height snapshots, cell indices and plain row submissions
//! (terrain.md §15.5.8, §15.5.12.1, §15.5.13.4 and §15.5.15). These implement
//! specified parts of the view, not a complete renderer or native host bridge.

// SPEC-GAP: complete cache bytes and auxiliary passes still require the §15.5.15.4 observations (TR47); no independent view is registered.

mod indices;
mod rows;

pub use indices::{cell_triangle_indices, TerrainViewIndices};
pub use rows::{PlainViewRow, ViewRowCall, ViewVertexBuffer};

use super::{msg, Post, Terrain, VertRect};
use crate::x87::r24;

/// Invalid input to a view height copy. No part of either cache is changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewHeightError {
    GridChanged,
    InvalidRectangle,
}

/// The y coordinates at byte +4 of the terrain and water draw vertices.
/// Owns its storage; it neither borrows the simulation nor updates it.
#[derive(Clone, Debug)]
pub struct TerrainViewHeights {
    cells_x: u32,
    cells_z: u32,
    terrain: Vec<f32>,
    water: Vec<f32>,
}

impl TerrainViewHeights {
    /// Init copies the entire inclusive vertex rectangle, terrain first and
    /// water second (§15.5.12.1). Construct a new cache when changing cities.
    pub fn new(source: &Terrain) -> Self {
        let count = source.vertex_count() as usize;
        let mut cache = Self {
            cells_x: source.cells_x(),
            cells_z: source.cells_z(),
            terrain: vec![0.0; count],
            water: vec![0.0; count],
        };
        let whole = VertRect::new(0, 0, cache.cells_x as i32, cache.cells_z as i32);
        cache.copy(source, whole, false);
        cache.copy(source, whole, true);
        cache
    }

    /// Rows are stored with stride cellsX + 1, including the last vertex row
    /// and column. Reading these slices does not refresh the snapshot.
    pub fn terrain(&self) -> &[f32] {
        &self.terrain
    }

    pub fn water(&self) -> &[f32] {
        &self.water
    }

    /// Consume a heights post, returning true when handled. Other messages
    /// are left for the rest of the view. data3/data4 do not affect the copy.
    pub fn handle_post(&mut self, source: &Terrain, post: &Post) -> Result<bool, ViewHeightError> {
        let water = match post.msg {
            msg::TERRAIN_CHANGED => false,
            msg::WATER_CHANGED => true,
            _ => return Ok(false),
        };
        // SPEC-GAP: invalid rectangles or a changed source grid are rejected atomically instead of performing the native unchecked copy.
        if source.cells_x() != self.cells_x || source.cells_z() != self.cells_z {
            return Err(ViewHeightError::GridChanged);
        }
        let rect = VertRect::new(
            (post.data1 >> 16) as i32,
            (post.data2 >> 16) as i32,
            (post.data1 & 0xFFFF) as i32,
            (post.data2 & 0xFFFF) as i32,
        );
        if rect.x0 > rect.x1 || rect.z0 > rect.z1 || rect.x1 > self.cells_x as i32 || rect.z1 > self.cells_z as i32 {
            return Err(ViewHeightError::InvalidRectangle);
        }
        self.copy(source, rect, water);
        Ok(true)
    }

    fn copy(&mut self, source: &Terrain, rect: VertRect, water: bool) {
        let stride = self.cells_x as usize + 1;
        let sea = source.sea_level();
        for z in rect.z0..=rect.z1 {
            for x in rect.x0..=rect.x1 {
                let i = z as usize * stride + x as usize;
                let altitude = source.altitude_at_index(i as u32);
                if water {
                    let depth = source.water_depth_at_index(i as u32);
                    self.water[i] = if depth > 0.0 {
                        r24(f64::from(altitude) + f64::from(depth)) as f32
                    } else {
                        sea
                    };
                } else {
                    // The terrain copy preserves the original f32 bits.
                    self.terrain[i] = altitude;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::TerrainTuning;

    fn source() -> Terrain {
        Terrain::from_altitudes(
            2,
            1,
            &[240.0, 250.0, 260.0, 270.0, 280.0, 290.0],
            250.0,
            TerrainTuning::SHIPPED,
        )
        .unwrap()
    }

    fn post(msg: u32, rect: VertRect) -> Post {
        let (data1, data2) = rect.packed();
        Post {
            msg,
            data1,
            data2,
            data3: u32::MAX,
            data4: u32::MAX,
        }
    }

    #[test]
    fn init_copies_borders_and_water_surface_not_depth() {
        let mut t = source();
        t.set_water_depth(1, 0, 5.0);
        t.set_water_depth(0, 1, -3.0);
        t.set_water_depth(2, 1, 12.0);
        let cache = TerrainViewHeights::new(&t);
        assert_eq!(cache.terrain(), &[240.0, 250.0, 260.0, 270.0, 280.0, 290.0]);
        assert_eq!(cache.water(), &[250.0, 255.0, 250.0, 250.0, 250.0, 302.0]);
    }

    #[test]
    fn messages_refresh_independent_snapshots_only_inside_inclusive_rect() {
        let mut t = source();
        let mut cache = TerrainViewHeights::new(&t);
        let old_water = cache.water().to_vec();
        // Both mutations remain invisible until a corresponding message arrives.
        for z in 0..=1 {
            for x in 0..=2 {
                t.set_raw_altitude(x, z, 300.0 + (z * 3 + x) as f32);
                t.set_water_depth(x, z, 20.0);
            }
        }
        assert_eq!(cache.terrain()[5], 290.0);
        assert_eq!(cache.water(), old_water);
        let rect = VertRect::new(1, 0, 2, 1);
        assert_eq!(cache.handle_post(&t, &post(msg::TERRAIN_CHANGED, rect)), Ok(true));
        assert_eq!(cache.terrain(), &[240.0, 301.0, 302.0, 270.0, 304.0, 305.0]);
        assert_eq!(cache.water(), old_water);
        assert_eq!(cache.handle_post(&t, &post(msg::WATER_CHANGED, rect)), Ok(true));
        assert_eq!(cache.water(), &[250.0, 321.0, 322.0, 250.0, 324.0, 325.0]);
    }

    #[test]
    fn water_post_reads_live_altitude_without_refreshing_terrain_snapshot() {
        let mut t = source();
        let mut cache = TerrainViewHeights::new(&t);
        t.set_raw_altitude(2, 1, 310.0);
        t.set_water_depth(2, 1, 2.0);
        assert_eq!(
            cache.handle_post(&t, &post(msg::WATER_CHANGED, VertRect::point(2, 1))),
            Ok(true)
        );
        assert_eq!(cache.terrain()[5], 290.0);
        assert_eq!(cache.water()[5], 312.0);
    }

    #[test]
    fn water_post_reads_live_sea_level_only_for_nonpositive_depths() {
        let mut t = source();
        let mut cache = TerrainViewHeights::new(&t);
        for (x, depth) in [0.0, -0.0, -3.0].into_iter().enumerate() {
            t.set_water_depth(x as i32, 0, depth);
            t.set_water_depth(x as i32, 1, (x + 1) as f32);
        }
        t.set_sea_level_raw(275.0);
        let whole = VertRect::new(0, 0, 2, 1);
        // The view subscribes to the height messages, not SeaLevelChanged.
        assert_eq!(cache.handle_post(&t, &post(msg::SEA_LEVEL_CHANGED, whole)), Ok(false));
        assert_eq!(cache.water(), &[250.0; 6]);
        assert_eq!(cache.handle_post(&t, &post(msg::WATER_CHANGED, whole)), Ok(true));
        assert_eq!(cache.water(), &[275.0, 275.0, 275.0, 271.0, 282.0, 293.0]);
        assert_eq!(cache.terrain(), &[240.0, 250.0, 260.0, 270.0, 280.0, 290.0]);
    }

    #[test]
    fn terrain_copy_preserves_float_bits_and_water_addition_rounds_once() {
        let mut t = source();
        let payload = 0x7FC1_2345;
        t.set_raw_altitude(0, 0, f32::from_bits(payload));
        t.set_water_depth(0, 0, 0.0);
        t.set_raw_altitude(2, 1, -0.0);
        t.set_raw_altitude(1, 1, 16_777_216.0);
        t.set_water_depth(1, 1, 1.0);
        let cache = TerrainViewHeights::new(&t);
        assert_eq!(cache.terrain()[0].to_bits(), payload);
        assert_eq!(cache.water()[0], 250.0);
        assert_eq!(cache.terrain()[5].to_bits(), (-0.0_f32).to_bits());
        assert_eq!(cache.water()[4], 16_777_216.0);
    }

    #[test]
    fn invalid_posts_never_clamp_or_partially_overwrite() {
        let mut t = source();
        let mut cache = TerrainViewHeights::new(&t);
        let before_terrain = cache.terrain().to_vec();
        let before_water = cache.water().to_vec();
        t.set_raw_altitude(0, 0, 999.0);
        for rect in [
            VertRect::new(0, 0, 3, 1),
            VertRect::new(0, 0, 2, 2),
            VertRect::new(2, 0, 1, 1),
            VertRect::new(0, 1, 2, 0),
            VertRect::new(-1, 0, 1, 1),
        ] {
            for msg in [msg::TERRAIN_CHANGED, msg::WATER_CHANGED] {
                assert_eq!(
                    cache.handle_post(&t, &post(msg, rect)),
                    Err(ViewHeightError::InvalidRectangle)
                );
            }
        }
        let other = Terrain::create(1, 2, TerrainTuning::SHIPPED).unwrap();
        assert_eq!(
            cache.handle_post(&other, &post(msg::TERRAIN_CHANGED, VertRect::point(0, 0))),
            Err(ViewHeightError::GridChanged)
        );
        assert_eq!(cache.terrain(), before_terrain);
        assert_eq!(cache.water(), before_water);
        assert_eq!(
            cache.handle_post(&other, &post(msg::TEXTURE_REDISPLAY, VertRect::point(0, 0))),
            Ok(false)
        );
    }
}
