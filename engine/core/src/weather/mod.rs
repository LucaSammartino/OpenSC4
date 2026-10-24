// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The weather subsystem (`docs/specs/weather.md`): the weather simulator
//! (`cISC4WeatherSimulator`, class `0x2990C1BC`). Section references (`§n`)
//! point at that spec.
//!
//! # Structure
//!
//! - [`WeatherSim`]: the state of §3, the per-message behaviour of §5, the
//!   algorithms of §6, the lifecycle of §8, the tuning of §9 and the save
//!   record of §10.
//! - [`CellIterator`]: the shared "prime-step" random cell iterator (§6.7).
//! - [`WeatherHost`]: everything the simulator reads from the city, the
//!   terrain, the region and the simulator clock, plus message posting.
//! - [`WeatherRandom`]: the simulator's saved `cRZRandom` (§7), behind a
//!   trait so tests can script it ([`WeatherRng`] is the real one).
//!
//! # Choices made where the spec is open (`[choice]`)
//!
//! - Cell counts are clamped to `0 ..= MAX_CELLS` (1024) per axis; every
//!   grid access is bounds-checked, so a corrupt save or a lying host cannot
//!   make the simulator index out of range. Grids whose shape does not match
//!   the counts read as missing cells (0) and are not written.
//! - `PositionToCell` returns 0 (§4).
//! - `SetManualMoistureAddition` also clamps the unclamped upper bound to the
//!   grid (§6.4.1).
//! - `SimulateMoisture` supports only `cityCellsPerCell = 1` (§6.5). The
//!   height filter follows §6.5 step 3 exactly (z pass into a buffer, its
//!   high-z mirror using countX [quirk]; x pass `× 0.0625`; all `f32`).
//!   [choice] "in place" for the x pass is read as using the pass-1 values
//!   of the neighbours (a separable filter), not values already updated in
//!   the same pass. The blur mirrors each axis at its own size. The work
//!   buffer is `i32`; the result is clamped to 0..255 when it is written.
//!   An empty rectangle changes nothing but still posts MoistureMapUpdated.
//! - The water-rectangle scan (§6.6) implements the intended algorithm: the
//!   run of a growing rectangle is trimmed at its ends on each new row and
//!   the rectangle keeps the narrowest run.
//! - The iterator start uses a per-simulator C-runtime `rand()` seeded with 1
//!   (§7); it is not saved. With an empty rectangle `reset` still makes its
//!   one `rand()` call and starts at 0. The iterator's saved state byte is
//!   1 = running (also never reset) or 2 = exhausted (§10, [compat]).
//! - Endless loops of the original are bounded: the wind-angle draw (§6.2)
//!   gives up after 100 draws (then `s = 0`), a weekly pass walks at most
//!   `countX × countZ` cells (Init never sets `cellsPerWeek` higher; only a
//!   hostile save read before Init could) and stops after a second iterator
//!   wrap without progress, and `DoCompleteSimulation` runs at most
//!   `cells / cellsPerWeek + 3` weekly passes (one when `cellsPerWeek` is 0).
//! - Without a terrain, altitude reads 0, normals (0, 1, 0) and no cell has
//!   water (the host decides); `DoMonthlySimulation(1)` skips the water scan
//!   and `UpdateMoisture` does nothing (§5.3, §4).
//! - §5.1, §6.1, §6.2, §6.3 and §6.5 run in `f32` in the spec's operation
//!   order (one rounding per operation, foundations §18; the generator's
//!   doubles are 24-bit, foundations §6.1). The daily fraction uses the
//!   binary64 constant before rounding; daily
//!   normalization uses reciprocal products and moisture rescales the
//!   measured direction, as specified by WE1 in §5.1.1–2 and §6.5.1.
//!   The cosine of §6.1 is the
//!   `double` cosine of the rounded angle (x87 `fcos` is not affected by the
//!   precision control); it can differ from the original only within a
//!   `double` error of a `float32` rounding boundary. The windSpread tuning
//!   (`tan(deg × π/360) × 0.5`, §8.1) still runs in `f64` and is stored as
//!   `f32` (the spec gives no operation order for it).
//! - The saved Gaussian cache word keeps the last cached value when the
//!   "has cached" flag is 0, so write → read → write is byte-identical.
//! - `ambientMoisture` starts at 0 before the first `Init` (spec: "—").
//! - The byte-grid writer writes `rows × cols` and fails on non-square grids;
//!   the reader leaves a grid empty when either count is ≥ 1,000,000 (§10).
//! - A version 0/1 record's legacy block is read and discarded (§10).

mod simulator;

pub use crate::grid::{CellIterator, IteratorState, PRIME_TABLE};
pub use simulator::{shipped_tuning, Grid, WeatherData, WeatherSim, MAX_CELLS, WEATHER_RECORD_VERSION};

use std::rc::Rc;

use crate::property::PropertyHolder;
use crate::rng::{RzRandom, SeedSource, CLOCK_SEED};
use crate::stream::ResKey;

/// Class and interface IDs (§1).
pub const CLSID_WEATHER_SIMULATOR: u32 = 0x2990_C1BC;
pub const IID_WEATHER_SIMULATOR: u32 = 0xC6C6_6245;

/// Agent registration (§5): one agent at these levels, flags 2.
pub const AGENT_NAME: &str = "cSC4WeatherSimulator";
pub const AGENT_LEVELS: [u32; 3] = [6, 5, 4];
pub const AGENT_FLAGS: u32 = 2;

/// The tuning exemplar (§9).
pub const TUNING_KEY: ResKey = ResKey {
    type_id: 0x6534_284A,
    group: 0x7A4A_8458,
    instance: 0x1A2F_DB6B,
};

/// Messages (§5, §6).
pub mod msg {
    pub const SIM_NEW_DAY: u32 = 0x6695_6814;
    pub const SIM_NEW_WEEK: u32 = 0x6695_6815;
    pub const SIM_NEW_MONTH: u32 = 0x6695_6816;
    pub const POST_CITY_INIT: u32 = 0x26D3_1EC1;
    /// Posted: data2 = the new season.
    pub const SEASON_CHANGED: u32 = 0x6716_A4E5;
    pub const WEATHER_CYCLE_COMPLETED: u32 = 0x6716_A4E6;
    pub const MOISTURE_MAP_UPDATED: u32 = 0x09B6_D58E;
}

/// Tuning properties (§9).
pub mod prop {
    pub const SUBDIV_X: u32 = 0xA760_7D70;
    pub const SUBDIV_Z: u32 = 0xA760_7D71;
    pub const BLEND_RADIUS: u32 = 0xA760_7D72;
    pub const BASE_TEMP: u32 = 0xA760_7D73;
    pub const TEMP_AMP_A: u32 = 0xA760_7D74;
    pub const TEMP_AMP_B: u32 = 0xA760_7D75;
    pub const SEA_RADIUS: u32 = 0xA760_7D76;
    pub const SEA_WEIGHT: u32 = 0xA760_7D77;
    pub const HEIGHT_FACTOR: u32 = 0xA760_7D78;
    pub const WIND_FACTOR: u32 = 0xA760_7D79;
    pub const BASE_MOISTURE: u32 = 0xA760_7D7A;
    pub const MOIST_AMP_A: u32 = 0xA760_7D7B;
    pub const MOIST_AMP_B: u32 = 0xA760_7D7C;
    pub const SLOPE: u32 = 0xA760_7D7D;
    pub const HIGH_MOISTURE: u32 = 0xA760_7D7E;
    pub const CURVATURE_THRESHOLD: u32 = 0xA760_7D7F;
    pub const TUNING_80: u32 = 0xA760_7D80;
    pub const TUNING_81: u32 = 0xA760_7D81;
    pub const DEFAULT_WIND_SPEED: u32 = 0xA760_7D86;
    pub const DEFAULT_WIND_DIR_X: u32 = 0xA760_7D87;
    pub const DEFAULT_WIND_DIR_Z: u32 = 0xA760_7D88;
    pub const WIND_SPREAD_DEGREES: u32 = 0xA760_7D89;
}

/// Everything the weather simulator reads from, or does to, the rest of the
/// game. Coordinates are terrain vertex / cell indices unless named world.
pub trait WeatherHost {
    // ---- city ----
    /// `cISC4City::SizeX()` / `SizeZ()` in world units.
    fn city_size(&self) -> (f32, f32);
    /// `cISC4City::CellWidthX()` / `CellWidthZ()` (16 in the game).
    fn city_cell_width(&self) -> (f32, f32);
    /// `cISC4City::GetWorldHemisphere()` (1 = no season shift).
    fn world_hemisphere(&self) -> i32;
    /// The calendar month (1..12) of the simulator's current date.
    fn sim_month(&self) -> i32;
    /// `cISC4RegionalCity::GetWaterPrefs()` (lo, hi), or `None` without a
    /// region (defaults 60, 100).
    fn water_prefs(&self) -> Option<(u8, u8)>;
    /// The "Weather Tuning Parameters" exemplar ([`TUNING_KEY`]), if loaded.
    fn tuning(&self) -> Option<Rc<PropertyHolder>>;

    // ---- terrain ----
    /// Whether the city has a terrain (`cISTETerrain`).
    fn terrain_available(&self) -> bool;
    /// `GetAltitudeAtVertex(x, z)`.
    fn altitude_at_vertex(&self, x: i32, z: i32) -> f32;
    /// `GetVertexNormal(x, z)` as (x, y, z).
    fn vertex_normal(&self, x: i32, z: i32) -> [f32; 3];
    /// `CellHasWater(x, z)`.
    fn cell_has_water(&self, x: i32, z: i32) -> bool;
    /// `GetAltitude(worldX, worldZ)`.
    fn altitude(&self, world_x: f32, world_z: f32) -> f32;
    /// `GetSeaLevel()`.
    fn sea_level(&self) -> f32;

    // ---- messages ----
    /// Posts a message with void1 = the weather simulator.
    fn post(&self, msg_type: u32, data2: u32);
}

/// The saved generator state (§10).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RngState {
    pub lcg: u32,
    pub mul: u32,
    pub has_cached_gaussian: i32,
    pub cached_gaussian: f64,
}

/// The draws the weather makes from its `cRZRandom` (§7).
pub trait WeatherRandom {
    /// `RandomDoubleRangeUniform(a, b)`.
    fn range_f64(&mut self, a: f64, b: f64) -> f64;
    /// `RandomDoubleGaussian()`.
    fn gaussian(&mut self) -> f64;
    /// `RandomSint32RangeGaussianFast(lo, hi)`.
    fn range_i32_gaussian_fast(&mut self, lo: i32, hi: i32) -> i32;
    fn state(&self) -> RngState;
    fn set_state(&mut self, s: RngState);
}

/// The real generator: a `cRZRandom` plus the saved Gaussian cache word.
pub struct WeatherRng {
    rz: RzRandom,
    last_cached: f64,
}

impl WeatherRng {
    /// Seeded from the wall clock (`Seed(0xFFFFFFFF)`, §3).
    pub fn new(seeds: &dyn SeedSource) -> WeatherRng {
        WeatherRng {
            rz: RzRandom::new(CLOCK_SEED, seeds),
            last_cached: 0.0,
        }
    }

    pub fn with_seed(seed: u32, seeds: &dyn SeedSource) -> WeatherRng {
        WeatherRng {
            rz: RzRandom::new(seed, seeds),
            last_cached: 0.0,
        }
    }

    /// Seeded from the session seed (foundations §6.3 *[choice]*: a member
    /// seeded from the clock takes `derive(GZCLSID)`, `0x2990C1BC`). A
    /// loaded record's saved generator state replaces it as before (§10).
    pub fn from_session(session: u32) -> WeatherRng {
        let key = crate::seeding::keys::WEATHER;
        WeatherRng::with_seed(
            crate::seeding::seed(session, key),
            &crate::seeding::clocks(session, key),
        )
    }
}

impl WeatherRandom for WeatherRng {
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        self.rz.range_f64(a, b)
    }
    fn gaussian(&mut self) -> f64 {
        let g = self.rz.gaussian();
        if let Some(c) = self.rz.cached_gaussian() {
            self.last_cached = c;
        }
        g
    }
    fn range_i32_gaussian_fast(&mut self, lo: i32, hi: i32) -> i32 {
        self.rz.range_i32_gaussian_fast(lo, hi)
    }
    fn state(&self) -> RngState {
        let cached = self.rz.cached_gaussian();
        RngState {
            lcg: self.rz.lcg_state(),
            mul: self.rz.mul_state(),
            has_cached_gaussian: i32::from(cached.is_some()),
            cached_gaussian: cached.unwrap_or(self.last_cached),
        }
    }
    fn set_state(&mut self, s: RngState) {
        let cached = (s.has_cached_gaussian != 0).then_some(s.cached_gaussian);
        self.rz.set_state(s.lcg, s.mul, cached);
        self.last_cached = s.cached_gaussian;
    }
}
