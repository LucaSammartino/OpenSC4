// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The shipped records the simulation uses and the disaster mapping, as
//! data (effects.md §8.4, §9.4, §10). The install sweep
//! (`formats/tests/effects_directory.rs`) checks them against the installed
//! effect directory.
//!
//! The game records keep only the fields of §9.4; the effect-map shape of
//! records 37–39 and 43 (render) is not in the spec and is left 0.

use super::directory::{game_flag, BrushRecord, ComponentRecord, EffectDescription, EffectDirectory, GameRecord};
use super::{component_type, msg};

/// Terrain brushes (terrain.md §9.1).
pub const CRATER_BRUSH: u32 = 0x8A33_E17A;
pub const VOLCANO_BRUSH: u32 = 0xCA31_3035;
pub const EROSION_BRUSH: u32 = 0x6A4B_AAFA;
pub const SMOOTHING_BRUSH: u32 = 0xEA31_3BB9;

/// Every shipped target height (§8.4).
pub const TARGET_HEIGHT: f32 = 100.0;

const fn brush(id: u32, rate: f32, length: f32, one_shot: bool, level: u32, strength: f32, radius: f32) -> BrushRecord {
    BrushRecord {
        version: 0,
        brush_id: id,
        rate,
        length,
        one_shot,
        level,
        strength: (strength, strength),
        radius: (radius, radius),
        target_height: TARGET_HEIGHT,
    }
}

/// The 25 brush records of §8.4, in file order.
pub const BRUSHES: [BrushRecord; 25] = [
    brush(CRATER_BRUSH, 1.0, 0.0, true, 2, 4.0, 0.65),
    brush(CRATER_BRUSH, 2.0, 0.5, false, 3, 3.0, 0.4),
    brush(VOLCANO_BRUSH, 30.0, 3.0, false, 1, 1.0, 0.5),
    brush(VOLCANO_BRUSH, 40.0, 4.0, false, 1, 1.0, 0.5),
    brush(VOLCANO_BRUSH, 45.0, 3.5, false, 1, 1.0, 0.6),
    brush(VOLCANO_BRUSH, 50.0, 3.5, false, 1, 1.0, 0.6),
    brush(VOLCANO_BRUSH, 55.0, 3.0, false, 1, 1.0, 0.8),
    brush(VOLCANO_BRUSH, 65.0, 3.0, false, 1, 1.0, 0.8),
    brush(VOLCANO_BRUSH, 65.0, 3.0, false, 1, 1.0, 1.0),
    brush(VOLCANO_BRUSH, 80.0, 3.5, false, 1, 1.0, 1.0),
    brush(VOLCANO_BRUSH, 130.0, 2.0, false, 1, 1.0, 1.3),
    brush(CRATER_BRUSH, 2.0, 0.0, true, 3, 4.0, 0.5),
    brush(CRATER_BRUSH, 2.0, 0.0, true, 3, 6.0, 0.7),
    brush(CRATER_BRUSH, 2.0, 0.0, true, 3, 3.0, 0.8),
    brush(CRATER_BRUSH, 2.0, 0.0, true, 3, 2.0, 0.6),
    brush(CRATER_BRUSH, 2.0, 0.0, true, 3, 5.0, 0.9),
    brush(CRATER_BRUSH, 2.0, 0.0, true, 3, 4.0, 0.5),
    brush(CRATER_BRUSH, 2.0, 0.0, true, 3, 6.0, 0.8),
    brush(CRATER_BRUSH, 2.0, 0.0, true, 3, 3.0, 1.0),
    brush(CRATER_BRUSH, 2.0, 0.0, true, 3, 2.0, 0.6),
    brush(CRATER_BRUSH, 2.0, 0.0, true, 3, 5.0, 0.7),
    brush(CRATER_BRUSH, 2.0, 0.0, true, 3, 4.0, 0.5),
    brush(EROSION_BRUSH, 1.0, 0.0, true, 2, 1.0, 1.0),
    brush(SMOOTHING_BRUSH, 1.0, 0.0, true, 2, 1.0, 1.0),
    brush(EROSION_BRUSH, 1.0, 0.0, true, 4, 0.3, 0.25),
];

/// Demolition flags of §9.4 (demolition.md §4.9): the cause byte, and the
/// explode bits `0x1300`.
pub const DEMOLISH_CAUSE_1: u32 = 1;
pub const DEMOLISH_CAUSE_2: u32 = 2;
pub const DEMOLISH_EXPLODE_1: u32 = 0x1301;
pub const DEMOLISH_EXPLODE_2: u32 = 0x1302;

/// The number of shipped game records.
pub const GAME_RECORD_COUNT: usize = 57;

#[derive(Clone, Copy, Default)]
struct G {
    flags: u32,
    demolish: i32,
    dflags: u32,
    max_size: f32,
    burn: i32,
    message: u32,
    duration: f32,
    toxic: i32,
    extinguish: i32,
}

const G0: G = G {
    flags: 0,
    demolish: 0,
    dflags: 1,
    max_size: 0.0,
    burn: 0,
    message: 0,
    duration: 0.0,
    toxic: 0,
    extinguish: 0,
};

const fn burn(r: i32) -> G {
    G { burn: r, ..G0 }
}

const fn demolish(flags: u32, r: i32, dflags: u32, burn: i32) -> G {
    G {
        flags,
        demolish: r,
        dflags,
        burn,
        ..G0
    }
}

const fn hidden_pause(seconds: f32) -> G {
    G {
        flags: game_flag::PAUSE_SIM_HIDDEN,
        duration: seconds,
        ..G0
    }
}

const fn message(t: u32) -> G {
    G { message: t, ..G0 }
}

const NN: u32 = game_flag::NO_NETWORKS;
const NNF: u32 = game_flag::NO_NETWORKS | game_flag::NO_FLORA;

/// The simulation fields of the 57 game records (§9.4).
const GAMES: [G; GAME_RECORD_COUNT] = [
    burn(1),                                // 0 debriscorefx_terrain
    demolish(game_flag::SINGLE, 10, 1, 0),  // 1 mushroomcloud
    burn(15),                               // 2 mushroomcloud
    hidden_pause(5.0),                      // 3 mushroomcloud
    hidden_pause(5.0),                      // 4 powerplant_explosion
    demolish(0, 2, 1, 0),                   // 5 powerplant_explosion
    burn(6),                                // 6 powerplant_explosion
    burn(1),                                // 7 electrical_fire_starter_terrain
    burn(1),                                // 8 lava_fallout_burn_terrain
    demolish(0, 5, DEMOLISH_CAUSE_2, 10),   // 9 volcanocreator1a
    demolish(0, 5, DEMOLISH_CAUSE_2, 10),   // 10 volcanocreator1b
    demolish(0, 5, DEMOLISH_CAUSE_2, 10),   // 11 volcanocreator2a
    demolish(0, 5, DEMOLISH_CAUSE_2, 10),   // 12 volcanocreator2b
    demolish(0, 8, DEMOLISH_CAUSE_2, 12),   // 13 volcanocreator3a
    demolish(0, 8, DEMOLISH_CAUSE_2, 12),   // 14 volcanocreator3b
    demolish(0, 8, DEMOLISH_CAUSE_2, 12),   // 15 volcanocreator4a
    demolish(0, 10, DEMOLISH_CAUSE_2, 12),  // 16 volcanocreator4b
    demolish(0, 10, DEMOLISH_CAUSE_2, 14),  // 17 volcanocreator5
    demolish(NN, 1, DEMOLISH_EXPLODE_1, 0), // 18 robot_walkfx_slave
    message(msg::EFFECT_LOCATION),          // 19 robot_walkfx_slave
    demolish(NN, 1, DEMOLISH_EXPLODE_1, 0), // 20 shooting_platesfx_slave
    demolish(NN, 1, DEMOLISH_EXPLODE_1, 0), // 21 robot_firefx_slave
    demolish(NN, 1, DEMOLISH_EXPLODE_1, 2), // 22 robot_landfx_slave
    message(msg::EFFECT_LOCATION),          // 23 meteor_stuff_slave
    demolish(0, 6, DEMOLISH_CAUSE_2, 10),   // 24 meteor_stuff_terrain
    demolish(0, 6, DEMOLISH_CAUSE_2, 10),   // 25 meteor_stuff_water
    G {
        flags: NN,
        demolish: 4,
        dflags: DEMOLISH_EXPLODE_2,
        max_size: 48.0,
        message: msg::TORNADO_LOCATION,
        ..G0
    }, // 26 tornadofx_slave
    demolish(NNF, 2, DEMOLISH_EXPLODE_2, 0), // 27 earthquakefire_terrain
    demolish(NNF, 1, DEMOLISH_EXPLODE_2, 0), // 28 quaker
    burn(2),                                // 29 lightningstrike_trigger
    hidden_pause(5.0),                      // 30 sparkles_on_demolish
    hidden_pause(15.0),                     // 31 obliterate_finale
    demolish(NNF, 1, DEMOLISH_EXPLODE_1, 0), // 32 lasercrater_a
    demolish(0, 2, 1, 0),                   // 33 biglasercrater_a
    demolish(NN, 1, DEMOLISH_EXPLODE_1, 2), // 34 mouthfirefx_slave
    demolish(NN, 1, DEMOLISH_EXPLODE_1, 0), // 35 tsaurus_walkfx_slave
    message(msg::EFFECT_LOCATION),          // 36 tsaurus_walkfx_slave
    G0,                                     // 37 runwaysmall (effect map)
    G0,                                     // 38 runwaymedium (effect map)
    G0,                                     // 39 runwaylarge (effect map)
    burn(2),                                // 40 rocket_explodefx_terrain
    burn(1),                                // 41 rocket_explodefx2_terrain
    burn(1),                                // 42 rocket_explodefx3_terrain
    G0,                                     // 43 whalefx_slave (effect map)
    demolish(0, 1, 1, 0),                   // 44 (no effect uses it)
    burn(1),                                // 45 molotov_flames
    hidden_pause(5.0),                      // 46 smlmushroomcloud
    demolish(NN, 1, DEMOLISH_EXPLODE_2, 2), // 47 tankfire_terrain
    G {
        extinguish: 10000,
        ..G0
    }, // 48 water1drop_slave
    G {
        extinguish: 10000,
        ..G0
    }, // 49 water1drop_terrain
    G {
        extinguish: 10000,
        ..G0
    }, // 50 waterdrop
    demolish(0, 2, DEMOLISH_EXPLODE_2, 2),  // 51 helifire_terrain
    demolish(0, 2, DEMOLISH_EXPLODE_2, 2),  // 52 jetfire_terrain
    demolish(NN, 1, DEMOLISH_EXPLODE_2, 2), // 53 ufolasercrater_a
    demolish(NN, 5, DEMOLISH_EXPLODE_2, 2), // 54 whalefire_terrain
    G { toxic: 20, ..G0 },                  // 55 p_train_toxic_slave
    G { extinguish: 4, ..G0 },              // 56 collisions_active
];

/// The records 37–39 and 43 carry only an effect map (render).
pub const MAP_ONLY_GAME_RECORDS: [usize; 4] = [37, 38, 39, 43];

/// Game record `i` of §9.4 (the constructor's defaults for every field the
/// table does not give); `None` past 56.
pub fn game_record(i: usize) -> Option<GameRecord> {
    let g = GAMES.get(i)?;
    Some(GameRecord {
        flags: g.flags,
        demolish_radius: g.demolish,
        demolish_flags: g.dflags,
        max_demolish_size: g.max_size,
        burn_radius: g.burn,
        message_type: g.message,
        duration: g.duration,
        toxic: g.toxic,
        extinguish: g.extinguish,
        ..GameRecord::default()
    })
}

/// All 57 game records of §9.4.
pub fn game_records() -> Vec<GameRecord> {
    (0..GAME_RECORD_COUNT).filter_map(game_record).collect()
}

/// Whether two game records agree on the simulation fields of §9.4 (flags,
/// demolition radius, flags and size limits, burn, message type, duration,
/// toxic, extinguish).
pub fn same_simulation_fields(a: &GameRecord, b: &GameRecord) -> bool {
    a.flags == b.flags
        && a.demolish_radius == b.demolish_radius
        && (a.demolish_radius == 0 || a.demolish_flags == b.demolish_flags)
        && a.min_demolish_size == b.min_demolish_size
        && a.max_demolish_size == b.max_demolish_size
        && a.burn_radius == b.burn_radius
        && a.message_type == b.message_type
        && a.duration == b.duration
        && a.toxic == b.toxic
        && a.extinguish == b.extinguish
}

// ---- the volcano (§10.1) ----------------------------------------------------------------------------

/// The effect the volcano creates (§10.1).
pub const VOLCANO_CONTROL: &str = "volcano_control";
/// Its description index and priority in the shipped directory.
pub const VOLCANO_CONTROL_INDEX: u32 = 367;
pub const VOLCANO_CONTROL_PRIORITY: u32 = 1;
/// The end message the volcano sets (data1 = the volcano instance).
pub const VOLCANO_END_MESSAGE: u32 = msg::VOLCANO_EFFECT_END;
/// The select group of the nine creators.
pub const VOLCANO_SELECT_GROUP: u16 = 1;
/// The attractor component (`volcano_disaster`, attractor 18, flag 1).
pub const VOLCANO_ATTRACTOR: (&str, u32) = ("volcano_disaster", 18);

/// A volcano creator: its name, brush record, game record and select
/// weight in `volcano_control`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VolcanoCreator {
    pub name: &'static str,
    pub brush: usize,
    pub game: usize,
    pub weight: u16,
}

/// The nine creators, in `volcano_control`'s component order (§10.1).
pub const VOLCANO_CREATORS: [VolcanoCreator; 9] = [
    VolcanoCreator {
        name: "volcanocreator1a",
        brush: 2,
        game: 9,
        weight: 7281,
    },
    VolcanoCreator {
        name: "volcanocreator1b",
        brush: 3,
        game: 10,
        weight: 7281,
    },
    VolcanoCreator {
        name: "volcanocreator2a",
        brush: 4,
        game: 11,
        weight: 7281,
    },
    VolcanoCreator {
        name: "volcanocreator2b",
        brush: 5,
        game: 12,
        weight: 7281,
    },
    VolcanoCreator {
        name: "volcanocreator3a",
        brush: 6,
        game: 13,
        weight: 7281,
    },
    VolcanoCreator {
        name: "volcanocreator3b",
        brush: 7,
        game: 14,
        weight: 7281,
    },
    VolcanoCreator {
        name: "volcanocreator4a",
        brush: 8,
        game: 15,
        weight: 7281,
    },
    VolcanoCreator {
        name: "volcanocreator4b",
        brush: 9,
        game: 16,
        weight: 7281,
    },
    VolcanoCreator {
        name: "volcanocreator5",
        brush: 10,
        game: 17,
        weight: 7287,
    },
];

/// The member a select draw `r` picks among `weights` (§5.3): the first
/// whose running sum is `≥ r`, 0-based; `None` when the sum stays below.
pub fn select_member(weights: &[u16], r: u32) -> Option<usize> {
    let mut sum = 0u32;
    weights.iter().position(|&w| {
        sum += u32::from(w);
        sum >= r
    })
}

// ---- the effect disasters (§10.2) ---------------------------------------------------------------------

/// The layer's managers whose menu entries only place `N_control` (§10.2).
pub const LAYER_MANAGER_TYPES: [u32; 3] = [0xC4C9_0997, 0x4296_3812, 0x4A13_32E8];
/// The parameter index an effect-disaster instance forwards (itself) with.
pub const INSTANCE_PARAMETER: u32 = 13;
/// `cISC4DisasterManagerEffect` (§11.3).
pub const IID_DISASTER_MANAGER_EFFECT: u32 = 0xCA7B_3F3C;

/// The menu tool's effects for the exemplar name `N` (§10.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolEffects {
    pub trigger: String,
    pub button_trigger: String,
    pub button: String,
    /// Tool `+0x98`.
    pub single: String,
    /// Tool `+0x94`: the effect the disaster starts.
    pub control: String,
    /// Tool `+0x9C`.
    pub active: String,
    pub inactive: String,
}

impl ToolEffects {
    pub fn of(n: &str) -> ToolEffects {
        ToolEffects {
            trigger: format!("{n}_trigger"),
            button_trigger: format!("{n}_button_trigger"),
            button: format!("{n}_button"),
            single: format!("{n}_single"),
            control: format!("{n}_control"),
            active: format!("{n}_active"),
            inactive: format!("{n}_inactive"),
        }
    }
}

/// A shipped control effect (§10.2): the menu entry, `N_control` and its
/// description index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlEffect {
    pub menu: &'static str,
    pub control: &'static str,
    pub index: u32,
}

/// The shipped control effects (all priority 1). Lightning has none.
pub const CONTROL_EFFECTS: [ControlEffect; 5] = [
    ControlEffect {
        menu: "Meteors",
        control: "meteorstrike_control",
        index: 383,
    },
    ControlEffect {
        menu: "Tornado",
        control: "tornado_control",
        index: 400,
    },
    ControlEffect {
        menu: "RobotAttack",
        control: "robot_attack_control",
        index: 382,
    },
    ControlEffect {
        menu: "Truckasaurus",
        control: "tsaurus_attack_control",
        index: 463,
    },
    ControlEffect {
        menu: "UFOs",
        control: "ufo_control",
        index: 453,
    },
];

/// The game records with a location message (§10.2).
pub const LOCATION_GAME_RECORDS: [usize; 3] = [19, 23, 36];

/// The meteor craters' select group (`meteor_stuff_terrain`: brush records
/// 11–15, weights 13107 each) and the lightning bolts' (weights 9362 × 6,
/// 9363).
pub const METEOR_CRATER_WEIGHTS: [u16; 5] = [13107; 5];
pub const LIGHTNING_BOLT_WEIGHTS: [u16; 7] = [9362, 9362, 9362, 9362, 9362, 9362, 9363];

/// The meteor crater effects and their brush records (§8.4).
pub const METEOR_CRATERS: [(&str, usize); 5] = [
    ("meteorcrater_a", 11),
    ("meteorcrater_b", 12),
    ("meteorcrater_c", 13),
    ("meteorcrater_d", 14),
    ("meteorcrater_e", 15),
];

/// A directory with the shipped brush and game records and the effects of
/// the volcano and the meteor craters as §10 describes them (the creators'
/// render components left out): the nine creators (indices 0–8),
/// `volcano_control` (9, with its attractor component, flag 1) and the five
/// craters (10–14). For tests and vectors.
pub fn volcano_directory() -> EffectDirectory {
    let mut effects = Vec::new();
    let mut names = Vec::new();
    for c in VOLCANO_CREATORS {
        let mut g = ComponentRecord::new(component_type::GAME, c.game as u32);
        g.flags = 1;
        effects.push(EffectDescription::new(
            1,
            vec![ComponentRecord::new(component_type::BRUSH, c.brush as u32), g],
        ));
        names.push((c.name.to_owned(), (effects.len() - 1) as u32));
    }
    let mut comps: Vec<ComponentRecord> = VOLCANO_CREATORS
        .iter()
        .map(|c| ComponentRecord {
            select_group: VOLCANO_SELECT_GROUP,
            select_weight: c.weight,
            ..ComponentRecord::meta(c.name)
        })
        .collect();
    let mut attractor = ComponentRecord::new(component_type::ATTRACTOR, VOLCANO_ATTRACTOR.1);
    attractor.flags = 1;
    comps.push(attractor);
    effects.push(EffectDescription::new(VOLCANO_CONTROL_PRIORITY, comps));
    names.push((VOLCANO_CONTROL.to_owned(), (effects.len() - 1) as u32));
    for (name, b) in METEOR_CRATERS {
        effects.push(EffectDescription::new(
            1,
            vec![ComponentRecord::new(component_type::BRUSH, b as u32)],
        ));
        names.push((name.to_owned(), (effects.len() - 1) as u32));
    }
    EffectDirectory::from_parts(BRUSHES.to_vec(), game_records(), effects, names)
}
