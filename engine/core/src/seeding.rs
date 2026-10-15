// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The session seed (`docs/specs/foundations.md` §6.3 *[choice]*).
//!
//! The host gives the core one 32-bit session seed `S` (`fc4.h`: every
//! create function that owns a generator takes it). Every generator's seed is
//! `derive(K) = S XOR K`, with 0 and `0xFFFFFFFF` (the values that make the
//! original fall back to the clock, §6.1) moved by XOR 1 ([`derive_seed`]).
//! `K` depends on the generator's kind of §6.2 ([`SeedKey`]); [`keys`] lists
//! the `K` of every generator a merged core owns, and every core seeds
//! through [`seed`] / [`clocks`] with one of them.
//!
//! Not covered here, by §6.3: coordinate-seeded temporaries keep their
//! coordinate seed ([`coordinate_seed`] only moves the tract developer's
//! (0, 0) case off 0), and the C runtime `rand()` ([`crate::rng::CrtRand`])
//! stays unseeded (state 1, §12.6).

use crate::rng::{derive_seed, file_seed, FixedSeeds};

/// Which `K` a generator uses (foundations §6.3, by the kinds of §6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SeedKey {
    /// A file-local generator seeded once at static initialisation with the
    /// clock XOR a constant: `K` = that constant (§6.3 table,
    /// [`file_seed`]).
    FileLocal(u32),
    /// A member generator seeded from the clock (`Seed(0xFFFFFFFF)` at
    /// construction, or the clock at `Init`): `K` = the owning class's
    /// GZCLSID, XOR the instance's creation index (from 0) for a class with
    /// several instances (MySims).
    Member { clsid: u32, instance: u32 },
    /// A member seeded with a constant whose zero multiplicative state later
    /// reseeds from the clock (§6.1): that reseed uses `K` = the owning
    /// class's GZCLSID (flammability).
    ZeroStateReseed(u32),
    /// A temporary generator seeded from the clock: `K` = the owner's
    /// GZCLSID XOR `n`, the number of earlier creations at that site by the
    /// owning object since its construction (not saved; QUESTIONS CB14).
    Temporary { clsid: u32, n: u32 },
}

impl SeedKey {
    /// A member generator of a class with one instance.
    pub const fn member(clsid: u32) -> SeedKey {
        SeedKey::Member { clsid, instance: 0 }
    }

    /// The constant `K` of foundations §6.3.
    pub const fn k(self) -> u32 {
        match self {
            SeedKey::FileLocal(k) => k,
            SeedKey::Member { clsid, instance } => clsid ^ instance,
            SeedKey::ZeroStateReseed(clsid) => clsid,
            SeedKey::Temporary { clsid, n } => clsid ^ n,
        }
    }
}

/// The seed of the generator `key` in the session `session`:
/// `derive(K)` of foundations §6.3. Never 0 and never `0xFFFFFFFF`.
pub fn seed(session: u32, key: SeedKey) -> u32 {
    derive_seed(session, key.k())
}

/// The clock values to build the generator `key` with: both are
/// [`seed`]`(session, key)`. A generator seeded with a derived seed never
/// reads them (the seed is neither `0xFFFFFFFF` nor 0, and its
/// multiplicative state stays odd-multiplied and non-zero); a generator
/// seeded with a constant 0 reads the process clock once, and §6.3 makes
/// that reseed `derive(GZCLSID)` ([`SeedKey::ZeroStateReseed`]).
pub fn clocks(session: u32, key: SeedKey) -> FixedSeeds {
    let s = seed(session, key);
    FixedSeeds {
        wall_clock: s,
        process_clock: s,
    }
}

/// A coordinate-seeded temporary's seed (§6.3): unchanged, except that the
/// one case whose seed is 0 (tract developer `UpdatePopulation` for tract
/// (0, 0), whose multiplicative path the original reseeds from the clock)
/// uses 1 *[choice]* (tract-developer.md §4.18).
pub fn coordinate_seed(seed: u32) -> u32 {
    if seed == 0 {
        1
    } else {
        seed
    }
}

/// The `K` of every generator a merged core owns (foundations §6.2, §6.3).
///
/// Generators the merged cores do not draw from are left out: the power
/// simulator's and the zone manager's members (never drawn, power.md,
/// zones.md), the traffic network map's (no draws in traffic-network.md).
/// Cores still in flight that own generators (§6.2, §6.3 table) adopt the
/// helper when their glue lands: prop developer (member, × 28), city
/// situation manager (member), advisors and MySims (members, MySims per
/// instance), the `cSC4Lot`, wealth requester, network lot manager, network
/// construction crew, construction / base-texture / retaining-wall / prop /
/// fire occupants, automata, region creation and advice file-local
/// generators. The disaster layer's core is merged (its glue is not): its
/// generators are listed here.
pub mod keys {
    use super::{file_seed, SeedKey};

    // ---- file-local generators (the §6.3 table) --------------------------

    /// Building development simulator (`0x00B485F0`, `cRZRandom`).
    pub const BUILDING_DEVELOPMENT: SeedKey = SeedKey::FileLocal(file_seed::BUILDING_DEVELOPMENT);
    /// Growth developers (`0x00B49080`, `cRZRandom`, shared).
    pub const GROWTH_DEVELOPER: SeedKey = SeedKey::FileLocal(file_seed::GROWTH_DEVELOPER);
    /// Tract developers (`0x00B4AFD8`, `cRZRandom`, shared).
    pub const TRACT_DEVELOPER: SeedKey = SeedKey::FileLocal(file_seed::TRACT_DEVELOPER);
    /// Lot developer (`0x00B49AF0`, `cRZRandom`).
    pub const LOT_DEVELOPER: SeedKey = SeedKey::FileLocal(file_seed::LOT_DEVELOPER);
    /// Zone developer (`0x00B4B398`, `cRZRandom`).
    pub const ZONE_DEVELOPER: SeedKey = SeedKey::FileLocal(file_seed::ZONE_DEVELOPER);
    /// Budget simulator `DoMessage` (`0x00B47B78`, `cRZRandomSuper`).
    pub const BUDGET: SeedKey = SeedKey::FileLocal(file_seed::BUDGET);
    /// Crime simulator (`0x00B48630`, `cRZRandomSuper`, process-wide).
    pub const CRIME: SeedKey = SeedKey::FileLocal(file_seed::CRIME);
    /// Pollution simulator visual effects (`0x00B49BD0`, `cRZRandomSuper`).
    pub const POLLUTION_EFFECTS: SeedKey = SeedKey::FileLocal(file_seed::POLLUTION);
    /// Earthquake disaster (`0x00B22AE8`, `cRZRandom`; disasters.md).
    pub const EARTHQUAKE: SeedKey = SeedKey::FileLocal(file_seed::EARTHQUAKE);
    /// Pipe-burst disaster (`0x00B22B60`, `cRZRandom`; disasters.md).
    pub const PIPE_BURST: SeedKey = SeedKey::FileLocal(file_seed::PIPE_BURST);
    /// Riot disaster (`0x00B22B80`, `cRZRandom`; disasters.md).
    pub const RIOT: SeedKey = SeedKey::FileLocal(file_seed::RIOT);

    // ---- member generators seeded from the clock (K = GZCLSID) -----------

    /// Residential simulator (`cRZRandom`, wall clock at construction).
    pub const RESIDENTIAL: SeedKey = SeedKey::member(crate::residential::CLSID_RESIDENTIAL_SIMULATOR);
    /// Aura simulator (`cRZRandom`, wall clock at construction; aura.md §9).
    pub const AURA: SeedKey = SeedKey::member(crate::aura::CLSID_AURA_SIMULATOR);
    /// Weather simulator (`cRZRandom`, wall clock at construction).
    pub const WEATHER: SeedKey = SeedKey::member(crate::weather::CLSID_WEATHER_SIMULATOR);
    /// Flora simulator (`cRZRandom`, wall clock at construction).
    pub const FLORA: SeedKey = SeedKey::member(crate::flora::CLSID_FLORA_SIMULATOR);
    /// Fire protection simulator (`cRZRandom`, wall clock at construction;
    /// GZCLSID `0x2990C05A`, protection.md §1).
    pub const FIRE_PROTECTION: SeedKey =
        SeedKey::member(crate::protection::ProtectionKind::Fire.params().clsid);
    /// Police simulator (`cRZRandom`, wall clock at construction; GZCLSID
    /// `0x6990C04F`, protection.md §1).
    pub const POLICE: SeedKey = SeedKey::member(crate::protection::ProtectionKind::Police.params().clsid);
    /// Plumbing simulator (`cRZRandom`, wall clock at `Init`).
    pub const PLUMBING: SeedKey = SeedKey::member(crate::plumbing::CLSID_PLUMBING_SIMULATOR);
    /// Traffic simulator (`cRZRandomSuper`, wall clock at `Init`).
    pub const TRAFFIC_SIMULATOR: SeedKey = SeedKey::member(crate::traffic_simulator::RNG_K);
    /// Park manager (`cRZRandomSuper`, wall clock at construction; parks.md
    /// §5).
    pub const PARK_MANAGER: SeedKey = SeedKey::member(crate::parks::CLSID_PARK_MANAGER);
    /// The disaster fire instance's own `cRZRandom` (flammability.md §13.1;
    /// K = the fire instance class `0x69C1718F`, disasters lane choice, QUESTIONS DS1).
    pub const DISASTER_FIRE: SeedKey = SeedKey::member(crate::disasters::instance_class::FIRE);

    // ---- members seeded with a constant -----------------------------------

    /// Flammability simulator: its `cRZRandom` is seeded with 0 and keeps
    /// it; the double path's clock reseed uses `derive(GZCLSID)`.
    pub const FLAMMABILITY_RESEED: SeedKey =
        SeedKey::ZeroStateReseed(crate::flammability::CLSID_FLAMMABILITY_SIMULATOR);

    // ---- temporaries seeded from the clock (K = GZCLSID XOR n) -----------

    /// The civic building simulator's monthly temporary, the `n`-th made by
    /// this simulator object (civic-buildings.md §5).
    pub const fn civic_monthly(n: u32) -> SeedKey {
        SeedKey::Temporary {
            clsid: crate::civic::CLSID_CIVIC_BUILDING_SIMULATOR,
            n,
        }
    }

    /// The neighbours simulator's deal-proposal generator (process-wide and
    /// lazy in the original), the `n`-th made by this simulator object
    /// (neighbors.md; one per object, so `n` = 0).
    pub const fn neighbors_proposals(n: u32) -> SeedKey {
        SeedKey::Temporary {
            clsid: crate::neighbors::CLSID_NEIGHBORS_SIMULATOR,
            n,
        }
    }

    /// The argument-less pipe burst's throwaway generator (disasters.md §8),
    /// the `n`-th made by this layer object; `K` = the pipe-burst manager
    /// class `0x4A132FDF` XOR `n`.
    pub const fn pipe_burst_start(n: u32) -> SeedKey {
        SeedKey::Temporary {
            clsid: crate::disasters::seed::PIPE_BURST_MANAGER,
            n,
        }
    }

    /// A growth developer's decline-key temporary whose coordinate seed is
    /// `0xFFFFFFFF` (growth-developer.md §7 *[quirk]*: `Seed` then reads the
    /// clock), the `n`-th such clock read by this developer object.
    /// [choice] (QUESTIONS RN2) §6.3 says coordinate-seeded temporaries are
    /// unchanged; this one case reads the clock, so it is treated as a
    /// clock-seeded temporary of the owning class.
    pub const fn growth_decline_clock(n: u32) -> SeedKey {
        SeedKey::Temporary {
            clsid: crate::growth_developer::CLSID_GROWTH_DEVELOPER,
            n,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::{RzRandom, RzRandomSuper};

    const S: u32 = 0x1234_5678;

    /// `derive(K)` for a key whose expected `K` is `k`: the key's constant,
    /// the derived seed, the fix-up and the clock values.
    fn check(key: SeedKey, k: u32) {
        assert_eq!(key.k(), k, "{key:?}");
        assert_eq!(seed(S, key), S ^ k, "{key:?}");
        // S = K gives 0, S = !K gives 0xFFFFFFFF: both move by XOR 1.
        assert_eq!(seed(k, key), 1, "{key:?}");
        assert_eq!(seed(!k, key), 0xFFFF_FFFE, "{key:?}");
        let c = clocks(S, key);
        assert_eq!((c.wall_clock, c.process_clock), (S ^ k, S ^ k), "{key:?}");
    }

    #[test]
    fn derive_fix_up() {
        assert_eq!(derive_seed(5, 5), 1);
        assert_eq!(derive_seed(0xFFFF_FFFF, 0), 0xFFFF_FFFE);
        assert_eq!(derive_seed(0, 0), 1);
        assert_eq!(derive_seed(0xA5A5_0000, 0x0000_5A5A), 0xA5A5_5A5A);
    }

    #[test]
    fn k_earthquake() {
        check(keys::EARTHQUAKE, 0x6C09_C1AB);
    }

    #[test]
    fn k_pipe_burst() {
        check(keys::PIPE_BURST, 0xEC09_C1D1);
    }

    #[test]
    fn k_riot() {
        check(keys::RIOT, 0x4C09_C187);
    }

    #[test]
    fn k_disaster_fire() {
        check(keys::DISASTER_FIRE, 0x69C1_718F);
    }

    #[test]
    fn k_pipe_burst_start() {
        check(keys::pipe_burst_start(0), 0x4A13_2FDF);
        check(keys::pipe_burst_start(3), 0x4A13_2FDC);
    }

    #[test]
    fn k_building_development() {
        check(keys::BUILDING_DEVELOPMENT, 0x0C09_C0B8);
    }

    #[test]
    fn k_growth_developer() {
        check(keys::GROWTH_DEVELOPER, 0xAC09_C0E8);
    }

    #[test]
    fn k_tract_developer() {
        check(keys::TRACT_DEVELOPER, 0x4C09_C113);
    }

    #[test]
    fn k_lot_developer() {
        check(keys::LOT_DEVELOPER, 0x6C09_C0F4);
    }

    #[test]
    fn k_zone_developer() {
        check(keys::ZONE_DEVELOPER, 0x8C09_C68A);
    }

    #[test]
    fn k_budget() {
        check(keys::BUDGET, 0x6C09_C3D7);
    }

    #[test]
    fn k_crime() {
        check(keys::CRIME, 0x6C09_C3F6);
    }

    #[test]
    fn k_pollution_effects() {
        check(keys::POLLUTION_EFFECTS, 0x4C09_C4D3);
    }

    #[test]
    fn k_residential() {
        check(keys::RESIDENTIAL, 0x4990_C013);
    }

    #[test]
    fn k_aura() {
        check(keys::AURA, 0x0990_BDD8);
    }

    #[test]
    fn k_weather() {
        check(keys::WEATHER, 0x2990_C1BC);
    }

    #[test]
    fn k_flora() {
        check(keys::FLORA, 0xC990_C034);
    }

    #[test]
    fn k_fire_protection() {
        check(keys::FIRE_PROTECTION, 0x2990_C05A);
    }

    #[test]
    fn k_police() {
        check(keys::POLICE, 0x6990_C04F);
    }

    #[test]
    fn k_plumbing() {
        check(keys::PLUMBING, 0x0990_C075);
    }

    #[test]
    fn k_traffic_simulator() {
        check(keys::TRAFFIC_SIMULATOR, 0x6990_C1AA);
    }

    #[test]
    fn k_park_manager() {
        check(keys::PARK_MANAGER, 0xAA3F_E6BC);
    }

    #[test]
    fn k_flammability_reseed() {
        check(keys::FLAMMABILITY_RESEED, 0xE990_C045);
    }

    #[test]
    fn k_civic_monthly() {
        check(keys::civic_monthly(0), 0x0A41_3481);
        check(keys::civic_monthly(1), 0x0A41_3480);
        check(keys::civic_monthly(6), 0x0A41_3487);
    }

    #[test]
    fn k_neighbors_proposals() {
        check(keys::neighbors_proposals(0), 0xC990_C0A4);
        check(keys::neighbors_proposals(3), 0xC990_C0A7);
    }

    #[test]
    fn k_growth_decline_clock() {
        check(keys::growth_decline_clock(0), 0x0990_C138);
        check(keys::growth_decline_clock(2), 0x0990_C13A);
    }

    #[test]
    fn k_member_instances() {
        // MySims-style classes: GZCLSID XOR the creation index.
        let k = SeedKey::Member {
            clsid: 0xAB00_0000,
            instance: 5,
        };
        check(k, 0xAB00_0005);
        assert_eq!(SeedKey::member(0x10), SeedKey::Member { clsid: 0x10, instance: 0 });
    }

    #[test]
    fn coordinate_seeds() {
        assert_eq!(coordinate_seed(0), 1);
        assert_eq!(coordinate_seed(0x0001_0002 << 4), 0x0010_0020);
        assert_eq!(coordinate_seed(0xFFFF_FFFF), 0xFFFF_FFFF);
    }

    /// A derived generator never reads the clock values: built with any
    /// other clock values it draws the same.
    #[test]
    fn derived_generators_ignore_the_clock() {
        for key in [keys::AURA, keys::CRIME, keys::civic_monthly(3)] {
            let s = seed(S, key);
            let odd = FixedSeeds {
                wall_clock: 7,
                process_clock: 9,
            };
            let mut a = RzRandom::new(s, &clocks(S, key));
            let mut b = RzRandom::new(s, &odd);
            for _ in 0..8 {
                assert_eq!(a.next_raw(), b.next_raw());
                assert_eq!(a.uniform_f64().to_bits(), b.uniform_f64().to_bits());
            }
            let mut c = RzRandomSuper::new(s, &clocks(S, key));
            let mut d = RzRandomSuper::new(s, &odd);
            assert_eq!(c.next_u32(), d.next_u32());
        }
    }

    /// Two sessions with the same `S` give every generator the same seed;
    /// a different `S` (bit 1, RN1) changes them.
    #[test]
    fn same_session_same_seeds() {
        let all = [
            keys::BUILDING_DEVELOPMENT,
            keys::GROWTH_DEVELOPER,
            keys::TRACT_DEVELOPER,
            keys::LOT_DEVELOPER,
            keys::ZONE_DEVELOPER,
            keys::BUDGET,
            keys::CRIME,
            keys::POLLUTION_EFFECTS,
            keys::RESIDENTIAL,
            keys::AURA,
            keys::WEATHER,
            keys::FLORA,
            keys::FIRE_PROTECTION,
            keys::POLICE,
            keys::PLUMBING,
            keys::TRAFFIC_SIMULATOR,
            keys::PARK_MANAGER,
            keys::FLAMMABILITY_RESEED,
            keys::EARTHQUAKE,
            keys::PIPE_BURST,
            keys::RIOT,
            keys::DISASTER_FIRE,
        ];
        for key in all {
            assert_eq!(seed(S, key), seed(S, key));
            assert_ne!(seed(S, key), seed(S ^ 2, key));
        }
        // Every K is distinct, so no two generators share a stream.
        let mut ks: Vec<u32> = all.iter().map(|k| k.k()).collect();
        ks.sort_unstable();
        ks.dedup();
        assert_eq!(ks.len(), all.len());
    }
}
