// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Ordered directional trace, water-ripples §1.2. Flag meanings remain WR2.
use super::{Cell, Error, Point, TerrainProvider, WaterRipple};
use std::collections::VecDeque;

const DIRECTIONS: [(i32, i32); 8] = [(-1, 0), (-1, -1), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1)];

fn neighbour(c: Cell, d: usize, counts: [u32; 2]) -> Option<Cell> {
    let (dx, dz) = DIRECTIONS[d];
    let c = Cell {
        x: c.x.wrapping_add(dx as u32),
        z: c.z.wrapping_add(dz as u32),
    };
    (c.x <= counts[0] && c.z <= counts[1]).then_some(c)
}

/// Mutates only visit bit 1. The caller clears visits globally after all seeds.
pub fn trace(terrain: &dyn TerrainProvider, seed: Cell) -> Result<Vec<u32>, Error> {
    let counts = terrain.cell_counts();
    super::vertex_index(seed, counts)?;
    let stride = counts[0] + 1;
    let mut indices = VecDeque::from([seed.z * stride + seed.x]);
    terrain.set_vertex_flags(seed, terrain.vertex_flags(seed) as u16 | 1)?;
    let water = terrain.water_level();
    for forward in [true, false] {
        let mut current = seed;
        let mut excluded = None;
        for k in 0..8 {
            let d = if forward { k } else { 7 - k };
            if let Some(c) = neighbour(seed, d, counts) {
                if terrain.vertex_height(c)? < water {
                    excluded = Some(d);
                    break;
                }
            }
        }
        let Some(mut excluded) = excluded else {
            continue;
        };
        'walk: loop {
            for k in 1..8 {
                let d = if forward {
                    (excluded + k) & 7
                } else {
                    (excluded + 8 - k) & 7
                };
                let Some(c) = neighbour(current, d, counts) else {
                    continue;
                };
                let flags = terrain.vertex_flags(c);
                if WaterRipple::eligible_flags(flags) {
                    terrain.set_vertex_flags(c, flags as u16 | 1)?;
                    let index = c.z * stride + c.x;
                    if forward {
                        indices.push_back(index);
                    } else {
                        indices.push_front(index);
                    }
                    current = c;
                    excluded = (d + 4) & 7;
                    continue 'walk;
                }
                if terrain.vertex_height(c)? > water {
                    break 'walk;
                }
            }
            break;
        }
    }
    Ok(indices.into_iter().collect())
}

/// Refreshes flags, consumes short traces too, then clears bit 1 everywhere.
/// Appending versus replacement is an owner/caller decision, not made here.
pub fn detect(terrain: &dyn TerrainProvider, minimum: u32) -> Result<Vec<Vec<Point>>, Error> {
    let counts = super::dimensions(terrain)?;
    terrain.refresh_shore_flags()?;
    let stride = counts[0] + 1;
    let mut shores = Vec::new();
    for z in 0..=counts[1] {
        for x in 0..=counts[0] {
            let c = Cell { x, z };
            if WaterRipple::eligible_flags(terrain.vertex_flags(c)) {
                let indices = trace(terrain, c)?;
                if indices.len() as u32 > minimum {
                    shores.push(
                        indices
                            .into_iter()
                            .map(|i| {
                                // Valid indices are nonnegative signed32 in the specified domain.
                                let i = i as i32;
                                Point::new((i % stride as i32) as f32 * 16.0, (i / stride as i32) as f32 * 16.0)
                            })
                            .collect(),
                    );
                }
            }
        }
    }
    for z in 0..=counts[1] {
        for x in 0..=counts[0] {
            let c = Cell { x, z };
            terrain.set_vertex_flags(c, terrain.vertex_flags(c) as u16 & !1)?;
        }
    }
    Ok(shores)
}
