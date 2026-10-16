// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `UpdateTerrainHeights` (slot 57, `0x00644530`, §9.7): the grading.

use super::satisfier::VertexHtConstraintSatisfier;
use super::GradingTerrain;
use crate::terrain::VertRect;
use crate::x87::r24;

/// The grading threshold (float `0x00A867A4`, §9.7).
pub const GRADING_THRESHOLD: f32 = 0.001;
/// The `RedisplayTerrain` `data4` kind of the network tool's grading
/// (terrain.md §8.2).
pub const GRADING_REDISPLAY_KIND: u32 = 4;

/// The grading's result: the changed vertex ids in solution order and the
/// redisplayed vertex rectangle (`None` when nothing changed).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Grading {
    pub changed: Vec<i32>,
    pub rect: Option<VertRect>,
}

/// §9.7: writes the terrain satisfier's solution into the terrain.
pub fn update_terrain_heights(ter: &VertexHtConstraintSatisfier, t: &mut dyn GradingTerrain) -> Grading {
    let (cx, cz) = t.cells();
    let stride = cx as i32 + 1;
    let verts_z = cz as i32 + 1;
    // 1. Clear the output vector.
    let mut out = Grading::default();
    let mut rect: Option<VertRect> = None;
    // 2. Each solved vertex in solution order.
    for &(v, h) in ter.solution() {
        let a = t.vertex_altitude(v);
        let d = r24(f64::from(h) - f64::from(a)) as f32;
        if d.abs() > GRADING_THRESHOLD {
            t.set_altitude_at_vertex(v, h);
            out.changed.push(v);
            // [quirk] z divides by the Z vertex count, not the stride.
            let (x, z) = (v.rem_euclid(stride), v.div_euclid(verts_z));
            match &mut rect {
                None => rect = Some(VertRect::point(x, z)),
                Some(r) => r.include(x, z),
            }
        }
    }
    // 3. Grow by 1 (not below 0; the max side only while below the cell
    // count).
    if let Some(mut r) = rect {
        if r.x0 > 0 {
            r.x0 -= 1;
        }
        if r.z0 > 0 {
            r.z0 -= 1;
        }
        if r.x1 < cx as i32 {
            r.x1 += 1;
        }
        if r.z1 < cz as i32 {
            r.z1 += 1;
        }
        // 4. Redisplay inside a long terrain operation.
        t.begin_long_terrain_operation();
        t.redisplay_terrain(r, GRADING_REDISPLAY_KIND);
        t.end_long_terrain_operation();
        out.rect = Some(r);
    }
    out
}
