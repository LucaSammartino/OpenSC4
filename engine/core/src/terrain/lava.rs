// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The volcano lava's flow (terrain.md §11, `cSTEVolcanoLava`): the start
//! rectangles, `DoLavaFlow`, `SetThingsOnFire` and the burning draws. The
//! lava is not saved (§11.2). Rendering (decals, steam, frames) is out of
//! scope.

use crate::x87::{ftol32, r24};

use super::field::{Terrain, VertRect};
use super::kind;

/// The lava tuning (§11.1, exemplar `0x6534284A / 0x88CD66E9 / 5`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LavaTuning {
    /// `0x69987E61` × 1e-6: the maximum flow rate per microsecond.
    pub max_flow_rate: f32,
    /// `0x69987E62`: the level difference for the full rate.
    pub full_rate_difference: f32,
    /// `0x69987E63`: solidified lava → terrain height (shipped 0).
    pub height_factor: f32,
    /// `0x69987E68`: the solidification multiplier over water.
    pub water_multiplier: f32,
    /// `0x69987E69`: the ignition level.
    pub ignite_level: f32,
    /// `0x69987E6A`: the burn level.
    pub burn_level: f32,
    /// `0x69987E6C`: the solidification threshold.
    pub solidify_threshold: f32,
}

impl LavaTuning {
    pub const SHIPPED: LavaTuning = LavaTuning {
        max_flow_rate: 4e-6,
        full_rate_difference: 12.0,
        height_factor: 0.0,
        water_multiplier: 2.0,
        ignite_level: 1.5,
        burn_level: 3.0,
        solidify_threshold: 3.0,
    };
}

impl Default for LavaTuning {
    fn default() -> Self {
        LavaTuning::SHIPPED
    }
}

/// The lava's half-width of the burn rectangle (§11.2).
pub const BURN_HALF_WIDTH: i32 = 7;
/// Cells drawn per tick by `BurnSurroundingStuff`: `(7 × 7) / 6` (§11.4).
pub const BURN_DRAWS_PER_TICK: u32 = 8;
/// The edge outflow factor (§11.3 step 3.1).
const EDGE_OUTFLOW: f64 = 1e-7;
/// The flow below this level decays instead of solidifying (§11.3 step 3.3).
const DECAY_LEVEL: f32 = 0.1;

/// What the lava asks of the fire side (§11.4, §11.5).
pub trait LavaHost {
    /// The flammability simulator's `IsCellFlammable` (slot 6).
    fn is_cell_flammable(&mut self, x: i32, z: i32) -> bool;
    /// Fire interface slot 5: `IgniteCell(x, z, false, true, 0)`.
    fn ignite_cell(&mut self, x: i32, z: i32);
    /// Fire interface slot 7: `QuickBurnCell(x, z)`.
    fn quick_burn_cell(&mut self, x: i32, z: i32);
    /// Fire interface slot 9: `SetCause(3)`.
    fn set_cause(&mut self, cause: u32);
    /// The burn rectangle's random iterator: the next cell, or `None` when
    /// it is exhausted (the host resets it; that draw is lost, §11.4).
    fn next_burn_cell(&mut self) -> Option<(i32, i32)>;
}

/// The eight neighbours `(dx, dz)` in the order of the terrain's offsets
/// (`+0x84..+0xA0`, [`Terrain::neighbor_offsets`], TR4): W, NW, N, NE, E,
/// SE, S, SW (north is z − 1). The even k are the edges, the odd k the
/// diagonals.
pub const NEIGHBOURS: [(i32, i32); 8] = [(-1, 0), (-1, -1), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1)];

/// The lava of one eruption (§11.2).
#[derive(Clone, Debug, PartialEq)]
pub struct Lava {
    pub tuning: LavaTuning,
    /// One level per terrain vertex.
    pub levels: Vec<f32>,
    pub source: VertRect,
    pub active: VertRect,
    /// The burn rectangle (cells).
    pub burn: VertRect,
    pub production_rate: f32,
    pub solidification_rate: f32,
}

/// The `SetThingsOnFire` decision (§11.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireMode {
    None,
    Ignite,
    Burn,
}

impl Lava {
    /// `InitializeVolcanoLava` then the start at `(x, z)` metres with
    /// `radius` metres (§11.2): `(x, z)` clamped to `[0, size − 1]`, the
    /// source vertex `(floor(x / 16), floor(z / 16))`, the source rectangle
    /// of half-width `trunc(radius / 16)` clamped to the vertices, the
    /// active rectangle the same, and the burn rectangle of half-width 7
    /// clipped to the cells. Rates 0, levels 0.
    pub fn start(terrain: &Terrain, x: f32, z: f32, radius: f32, tuning: LavaTuning) -> Lava {
        let clamp = |v: f32, size: f32| {
            let hi = r24(f64::from(size) - 1.0) as f32;
            if v < 0.0 {
                0.0
            } else if v > hi {
                hi
            } else {
                v
            }
        };
        let x = clamp(x, terrain.size_x());
        let z = clamp(z, terrain.size_z());
        let sx = r24(f64::from(x) / 16.0).floor() as i32;
        let sz = r24(f64::from(z) / 16.0).floor() as i32;
        let hw = ftol32(r24(f64::from(radius) / 16.0));
        let (cx, cz) = (terrain.cells_x() as i32, terrain.cells_z() as i32);
        let source = VertRect::new((sx - hw).max(0), (sz - hw).max(0), (sx + hw).min(cx), (sz + hw).min(cz));
        let burn = VertRect::new(
            (sx - BURN_HALF_WIDTH).max(0),
            (sz - BURN_HALF_WIDTH).max(0),
            (sx + BURN_HALF_WIDTH).min(cx - 1),
            (sz + BURN_HALF_WIDTH).min(cz - 1),
        );
        Lava {
            tuning,
            levels: vec![0.0; terrain.vertex_count() as usize],
            source,
            active: source,
            burn,
            production_rate: 0.0,
            solidification_rate: 0.0,
        }
    }

    fn at(terrain: &Terrain, x: i32, z: i32) -> usize {
        x as usize + z as usize * terrain.verts_x() as usize
    }

    /// The lava level at a vertex (0 out of bounds).
    pub fn level(&self, terrain: &Terrain, x: i32, z: i32) -> f32 {
        if terrain.vertex_in_bounds(x, z) {
            self.levels[Self::at(terrain, x, z)]
        } else {
            0.0
        }
    }

    pub fn set_level(&mut self, terrain: &Terrain, x: i32, z: i32, v: f32) {
        if terrain.vertex_in_bounds(x, z) {
            self.levels[Self::at(terrain, x, z)] = v;
        }
    }

    fn surface(&self, terrain: &Terrain, x: i32, z: i32) -> f64 {
        let i = Self::at(terrain, x, z);
        r24(f64::from(terrain.altitudes()[i]) + f64::from(self.levels[i]))
    }

    /// `GetNeighborWithLowestLavaLevel` (`0x0075DA90`, §11.3, TR4): the
    /// neighbour index k (0..7 of [`NEIGHBOURS`]) or `None` when no neighbour
    /// is lower. `surf := r24(lava + alt)`, `best := 0`. The edges W, N, E, S
    /// (k = 0, 2, 4, 6) that the `GetNeighbors` mask allows win on `drop >
    /// best` (strict: the first wins a tie, a NaN never wins); then `best :=
    /// r24(√2 × best)` and the diagonals NW, NE, SE, SW (k = 1, 3, 5, 7)
    /// compete against it the same way, a winning diagonal setting `best` to
    /// its own unscaled drop.
    pub fn lowest_neighbour_k(&self, terrain: &Terrain, x: i32, z: i32) -> Option<usize> {
        if !terrain.vertex_in_bounds(x, z) {
            return None;
        }
        let me = self.surface(terrain, x, z);
        let mask = terrain.neighbor_mask(x, z);
        let mut best = 0.0f64;
        let mut result = None;
        let mut scan = |k: usize, best: &mut f64| {
            if mask & (1 << k) == 0 {
                return;
            }
            let (dx, dz) = NEIGHBOURS[k];
            let (nx, nz) = (x + dx, z + dz);
            if !terrain.vertex_in_bounds(nx, nz) {
                return;
            }
            let drop = r24(me - self.surface(terrain, nx, nz));
            if drop > *best {
                *best = drop;
                result = Some(k);
            }
        };
        for k in [0, 2, 4, 6] {
            scan(k, &mut best);
        }
        best = r24(f64::from(std::f32::consts::SQRT_2) * best);
        for k in [1, 3, 5, 7] {
            scan(k, &mut best);
        }
        result
    }

    /// The vertex of [`Lava::lowest_neighbour_k`].
    pub fn lowest_neighbour(&self, terrain: &Terrain, x: i32, z: i32) -> Option<(i32, i32)> {
        self.lowest_neighbour_k(terrain, x, z).map(|k| {
            let (dx, dz) = NEIGHBOURS[k];
            (x + dx, z + dz)
        })
    }

    /// The amount one vertex passes on (§11.3 step 3.1) and the neighbour it
    /// goes to (`None` = it leaves the map over the edge, or no flow).
    pub fn flow_amount(&self, terrain: &Terrain, x: i32, z: i32, dt: f32) -> (f32, Option<(i32, i32)>) {
        let lava = f64::from(self.level(terrain, x, z));
        let (cx, cz) = (terrain.cells_x() as i32, terrain.cells_z() as i32);
        if x == 0 || x == cx || z == 0 || z == cz {
            let a = r24(r24(f64::from(dt) * EDGE_OUTFLOW) * lava);
            return (a as f32, None);
        }
        let Some((nx, nz)) = self.lowest_neighbour(terrain, x, z) else {
            return (0.0, None);
        };
        let max_rate = f64::from(self.tuning.max_flow_rate);
        let full = f64::from(self.tuning.full_rate_difference);
        let mut h = r24(r24(self.surface(terrain, x, z) - self.surface(terrain, nx, nz)) * 0.5);
        let rate = if h < full {
            r24(r24(max_rate / full) * h)
        } else {
            max_rate
        };
        if lava < h {
            h = lava;
        }
        let mut amount = r24(r24(f64::from(dt) * h) * rate);
        if amount > h {
            amount = h;
        }
        (amount as f32, Some((nx, nz)))
    }

    /// `SetThingsOnFire`'s mode (§11.5): ignite when the level crosses the
    /// ignition level, burn when it crosses the burn level.
    pub fn fire_mode(&self, old: f32, amount: f32) -> FireMode {
        let new = r24(f64::from(old) + f64::from(amount)) as f32;
        if old < self.tuning.ignite_level && self.tuning.ignite_level <= new {
            FireMode::Ignite
        } else if old < self.tuning.burn_level && self.tuning.burn_level <= new {
            FireMode::Burn
        } else {
            FireMode::None
        }
    }

    /// `SetThingsOnFire(vertex, oldLava, amount)` (`0x0075DE60`, §11.5): the
    /// flammable cells of `(x − 1 … x) × (z − 1 … z)`, x outer and z inner
    /// (`(x − 1, z − 1)`, `(x − 1, z)`, `(x, z − 1)`, `(x, z)`), each ignited
    /// or quick-burned and then `SetCause(3)` (TR5): 0 to 4 `SetCause` per
    /// call. A cell outside the city is not flammable (slot 6), so it is
    /// skipped without asking the host.
    pub fn set_things_on_fire(
        &self,
        terrain: &Terrain,
        x: i32,
        z: i32,
        old: f32,
        amount: f32,
        host: &mut dyn LavaHost,
    ) -> FireMode {
        let mode = self.fire_mode(old, amount);
        if mode == FireMode::None {
            return mode;
        }
        for cx in x - 1..=x {
            for cz in z - 1..=z {
                if !terrain.cell_in_bounds(cx, cz) || !host.is_cell_flammable(cx, cz) {
                    continue;
                }
                match mode {
                    FireMode::Ignite => host.ignite_cell(cx, cz),
                    _ => host.quick_burn_cell(cx, cz),
                }
                host.set_cause(3);
            }
        }
        mode
    }

    /// `DoLavaFlow(dt)` (`0x0075F4A0`, §11.3), dt in microseconds. Changes
    /// the terrain only through `RaiseVertex(…, factor × f × oldLava)` and
    /// ends with `RedisplayTerrain(1, 1, touched, 3)` when a vertex
    /// solidified.
    pub fn flow(&mut self, terrain: &mut Terrain, dt: f32, host: &mut dyn LavaHost) {
        let dtf = f64::from(dt);
        // Step 1.
        let mut solid = r24(dtf * f64::from(self.solidification_rate));
        let mut solid_water = r24(solid * f64::from(self.tuning.water_multiplier));
        let solidifying = solid > 0.0;
        solid = solid.min(1.0);
        solid_water = solid_water.min(1.0);
        // Step 2.
        let add = r24(dtf * f64::from(self.production_rate));
        for z in self.source.z0..=self.source.z1 {
            for x in self.source.x0..=self.source.x1 {
                if terrain.vertex_in_bounds(x, z) {
                    let i = Self::at(terrain, x, z);
                    self.levels[i] = r24(f64::from(self.levels[i]) + add) as f32;
                    self.active.include(x, z);
                }
            }
        }
        // Step 3.
        let mut touched: Option<VertRect> = None;
        let a = self.active;
        for z in a.z0..=a.z1 {
            for x in a.x0..=a.x1 {
                if !terrain.vertex_in_bounds(x, z) {
                    continue;
                }
                let i = Self::at(terrain, x, z);
                if self.levels[i] <= 0.0 {
                    continue;
                }
                let (amount, to) = self.flow_amount(terrain, x, z, dt);
                if amount > 0.0 {
                    if let Some((nx, nz)) = to {
                        let j = Self::at(terrain, nx, nz);
                        let old = self.levels[j];
                        self.set_things_on_fire(terrain, nx, nz, old, amount, host);
                        self.levels[j] = r24(f64::from(old) + f64::from(amount)) as f32;
                    }
                    self.levels[i] = r24(f64::from(self.levels[i]) - f64::from(amount)) as f32;
                }
                if solidifying {
                    let lava = self.levels[i];
                    if lava >= self.tuning.solidify_threshold {
                        let f = if terrain.water_depth_at_index(i as u32) > 0.0 {
                            solid_water
                        } else {
                            solid
                        };
                        self.levels[i] = r24(f64::from(lava) - r24(f * f64::from(lava))) as f32;
                        let raise = r24(r24(f64::from(self.tuning.height_factor) * f) * f64::from(lava));
                        terrain.raise_vertex(i as u32, raise as f32);
                        match touched.as_mut() {
                            Some(r) => r.include(x, z),
                            None => touched = Some(VertRect::point(x, z)),
                        }
                    } else if lava >= DECAY_LEVEL {
                        self.levels[i] = r24(f64::from(lava) * 0.9) as f32;
                    } else {
                        self.levels[i] = 0.0;
                    }
                }
            }
        }
        // Step 4. [choice] The grown rectangle is clamped to the vertices.
        let (cx, cz) = (terrain.cells_x() as i32, terrain.cells_z() as i32);
        self.active = VertRect::new(
            (a.x0 - 1).max(0),
            (a.z0 - 1).max(0),
            (a.x1 + 1).min(cx),
            (a.z1 + 1).min(cz),
        );
        // Step 5.
        if let Some(t) = touched {
            terrain.redisplay_terrain(true, true, t, kind::DISASTER);
        }
    }

    /// `BurnSurroundingStuff` (`0x0075F340`, §11.4): while the production
    /// rate is positive, 8 draws from the burn iterator; each flammable cell
    /// drawn is ignited. An exhausted draw is lost.
    pub fn burn_surrounding(&self, host: &mut dyn LavaHost) {
        if self.production_rate <= 0.0 {
            return;
        }
        for _ in 0..BURN_DRAWS_PER_TICK {
            if let Some((x, z)) = host.next_burn_cell() {
                if host.is_cell_flammable(x, z) {
                    host.ignite_cell(x, z);
                }
            }
        }
    }
}
