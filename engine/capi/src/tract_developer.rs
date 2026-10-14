// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The tract developer's C ABI (fc4.h "tract developer",
//! docs/specs/tract-developer.md §1–§4, §6, §7).
//!
//! The host implements [`fc4_tract_host`]: every `TractDeveloperHost` method
//! (§8: the growth developer, the BDS, the lot developer, the lot manager,
//! lots, lot configurations and their manager, the city grids and services).
//! The plugin's `cISC4TractDeveloper` (with its message-target and
//! serializable sub-objects) forwards its slots to the `fc4_tract_*`
//! functions. The BDS, the growth developers, the lot developer and the lot
//! manager stay whoever the city runs: the host reaches them only through
//! their interfaces.
//!
//! Tokens are opaque host identities: growth developers, lots, lot
//! configurations, configuration filters and network tiles. 0 is the null
//! object. Tokens the host hands out are **borrowed** and must stay valid for
//! the whole top-level call that received them (the core never AddRefs or
//! releases, `fc4_core::tract_developer` "Reference counts"); a host keeps
//! the references its lists gave it until that call returns. Ordering by
//! token is the core's pointer order (§2.2, §9.5 item 1): a host that hands
//! out the game's object addresses gets the original's order.
//!
//! # Statics
//!
//! The class-wide statics of §9.1 (the file-local `cRZRandom`, the C
//! `rand()` state and the two random cell iterators) are one object per
//! process (per thread that creates objects; the game creates city components
//! on its main thread), made by the first [`fc4_tract_create`] with the
//! session seed (foundations §6.3: `derive_seed(S, 0x4C09C113)`, §4.22
//! *[choice]*). Later objects share them. The C `rand()` is the core's own
//! `CrtRand` at state 1 (foundations §12.6), not the game's.
//!
//! # Threads and re-entrancy
//!
//! The core is single-threaded. The plugin serialises every call on one object
//! with its own lock. A call made while the same object is already inside the
//! core (a host callback that comes back, which the core forbids) is refused
//! with the documented fallback and counted (`fc4_tract_state::reentrant`).
//!
//! # Shadow mode
//!
//! [`fc4_tract_create_shadow`] makes a core whose host table reads the game
//! but **never writes it**: services, agents, the message server, construction,
//! abandonment, reoccupation, lot creation, deletion, facing and state
//! changes, demolition, terrain redisplay, construction resources and the
//! configuration filters are swallowed and counted; `SetPopulation` calls are
//! recorded (for `fc4_tract_shadow_update_population`). A shadow core has its
//! own statics (never the replacement's).

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::SaveObject;
use fc4_core::tract_developer::{
    answers_iid, configuration_filter_includes, BuildingProfile, Cap, CellRect, Cheat, ConstructionProfile,
    GrowthProfile, Outcome, SharedStatics, TractDeveloper, TractDeveloperHost, TractDeveloperStatics,
    CLSID_TRACT_DEVELOPER, HIST_COLUMNS, HIST_ROWS,
};

use crate::budget::fc4_token_callback;
use crate::demand::fc4_u32_callback;
use crate::guard;
use crate::hostgrid::{collect_tokens, fc4_cell_rect, take_props};
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// The "Building Development" tuning exemplar (§4.21): type, group, instance.
pub const FC4_TRACT_TUNING_TYPE: u32 = 0x6534_284A;
pub const FC4_TRACT_TUNING_GROUP: u32 = 0xE7E2_C2DB;
pub const FC4_TRACT_TUNING_INSTANCE: u32 = 0xE8DA_7677;

/// `dev_cap`'s `which` (growth developer slots 19–22, §4.2).
pub const FC4_TRACT_CAP_POWER: u32 = 0;
pub const FC4_TRACT_CAP_WATER: u32 = 1;
pub const FC4_TRACT_CAP_FIRE: u32 = 2;
pub const FC4_TRACT_CAP_PARK: u32 = 3;

/// `cheat`'s `which` (BDS slots 54–57).
pub const FC4_TRACT_CHEAT_IGNORE_PLUMBING: u32 = 0;
pub const FC4_TRACT_CHEAT_IGNORE_POWER: u32 = 1;
pub const FC4_TRACT_CHEAT_IGNORE_CAPS: u32 = 2;
pub const FC4_TRACT_CHEAT_DEBUG_GROWTH: u32 = 3;

/// The most bits `cfg_required_roads` may report (the core reads four).
pub const FC4_TRACT_MAX_ROAD_BITS: usize = 32;

/// `GrowthProfile` as the slots read it (§7.1): the growth developer, the
/// tract and the stage (only its low byte is used, by `Grow`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct fc4_tract_profile {
    pub developer: u64,
    pub x: i32,
    pub z: i32,
    pub stage: i32,
    pub reserved: i32,
}

impl From<&fc4_tract_profile> for GrowthProfile {
    fn from(p: &fc4_tract_profile) -> Self {
        GrowthProfile {
            developer: p.developer,
            x: p.x,
            z: p.z,
            stage: p.stage,
        }
    }
}

/// `ConstructionProfile` (growth-developer §6.2): what `GetConstructionInfo`
/// fills (the fields after `land_value_max` are not read by the tract
/// developer).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct fc4_tract_construction {
    pub zone: i32,
    pub unknown2: i32,
    pub wealth: u32,
    pub purpose: u32,
    pub land_value_min: u8,
    pub land_value_max: u8,
    pub reserved: [u8; 2],
    pub building_type: u32,
    pub x: i32,
    pub z: i32,
}

/// The `BuildingProfile` fields the tract developer reads (docs/abi
/// layouts.md "Building profile": purpose +0x00, wealth +0x04, capacity
/// +0x08). An unknown type reads as zeros.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct fc4_tract_building_profile {
    pub purpose: u32,
    pub wealth: u32,
    pub capacity: u16,
    pub reserved: u16,
}

pub type fc4_tract_style_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, group: u32, fraction: f32)>;
/// One `SetPopulation(devType, n)` the shadow core asked for.
pub type fc4_tract_population_callback =
    Option<unsafe extern "C" fn(ctx: *mut c_void, lot: u64, dev_type: u32, population: u16)>;

/// Everything the tract developer asks of the game (fc4.h `fc4_tract_host`).
/// Missing callbacks answer neutrally (0, false, empty, none).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_tract_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // ---- lifecycle (§1.3, §1.4) ----
    pub acquire_services: Option<unsafe extern "C" fn(U) -> bool>,
    pub add_agent: Option<unsafe extern "C" fn(U, u32, u32) -> bool>,
    pub acquire_message_server: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_message_server: Option<unsafe extern "C" fn(U)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub release_services: Option<unsafe extern "C" fn(U)>,
    /// A NEW `fc4_props` with the tuning exemplar's properties, or NULL.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    // ---- growth developer ----
    pub dev_target_zone: Option<unsafe extern "C" fn(U, u64) -> i32>,
    pub dev_subtype: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub dev_purpose: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub dev_wealth: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub dev_construction_info: Option<unsafe extern "C" fn(U, u64, *mut fc4_tract_construction)>,
    pub dev_desirability_tract_size: Option<unsafe extern "C" fn(U, u64) -> i32>,
    pub dev_desirability_shift: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub dev_desirability_value: Option<unsafe extern "C" fn(U, u64, i32, i32) -> i16>,
    pub dev_capacity_value: Option<unsafe extern "C" fn(U, u64, i32, i32) -> u32>,
    /// False without a demand node; else `*out` = cISC4Demand slot 12.
    pub dev_source_demand: Option<unsafe extern "C" fn(U, u64, *mut f32) -> bool>,
    pub dev_cap: Option<unsafe extern "C" fn(U, u64, u32) -> u8>,
    pub dev_monopoly_use: Option<unsafe extern "C" fn(U, u64) -> i32>,
    /// The style map's entries in ascending key order.
    pub dev_style_percentages: Option<unsafe extern "C" fn(U, u64, fc4_tract_style_callback, *mut c_void)>,
    // ---- BDS ----
    pub building_profile: Option<unsafe extern "C" fn(U, u32, *mut fc4_tract_building_profile)>,
    /// The types of `GetBuildingTypesByFamily(set, id)` in the order whose
    /// insertion into a 193-bucket STLport `hash_set` reproduces the set.
    pub building_types_by_family: Option<unsafe extern "C" fn(U, u32, fc4_u32_callback, *mut c_void)>,
    pub building_in_occupant_group: Option<unsafe extern "C" fn(U, u32, u32) -> bool>,
    pub building_is_monopoly: Option<unsafe extern "C" fn(U, u32) -> bool>,
    /// False when the property is absent; else its values through the callback.
    pub building_field_ids: Option<unsafe extern "C" fn(U, u32, fc4_u32_callback, *mut c_void) -> bool>,
    pub developer_for: Option<unsafe extern "C" fn(U, u32, u32) -> u64>,
    pub claim_construction_resources: Option<unsafe extern "C" fn(U, u16) -> bool>,
    pub release_construction_resources: Option<unsafe extern "C" fn(U, u16)>,
    pub cheat: Option<unsafe extern "C" fn(U, u32) -> bool>,
    // ---- lot developer ----
    pub needs_anchor_building: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, u32) -> bool>,
    pub start_construction: Option<unsafe extern "C" fn(U, u64, u64, u32, bool) -> bool>,
    pub abandon_building: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub reoccupy_building: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub area_slope: Option<unsafe extern "C" fn(U, *const fc4_cell_rect) -> f32>,
    // ---- lot manager ----
    pub get_lot: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    pub lots_in_cell_rect: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, fc4_token_callback, *mut c_void)>,
    pub lots_surrounding_lot: Option<unsafe extern "C" fn(U, u64, u32, fc4_token_callback, *mut c_void)>,
    pub lots_surrounding_rect:
        Option<unsafe extern "C" fn(U, *const fc4_cell_rect, u32, fc4_token_callback, *mut c_void)>,
    pub can_create_lot: Option<unsafe extern "C" fn(U, i32, i32, i32, i32) -> i32>,
    pub create_lot_if_possible: Option<unsafe extern "C" fn(U, i32, i32, i32, i32, i32) -> u64>,
    pub delete_lot: Option<unsafe extern "C" fn(U, u64) -> bool>,
    // ---- lots ----
    pub lot_bounding_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub lot_slope: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub lot_location: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    pub lot_size: Option<unsafe extern "C" fn(U, u64, *mut u32, *mut u32)>,
    pub lot_facing: Option<unsafe extern "C" fn(U, u64) -> i32>,
    pub lot_set_facing: Option<unsafe extern "C" fn(U, u64, i32)>,
    pub lot_has_road_access: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_has_organic_road_access: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub lot_zone_type: Option<unsafe extern "C" fn(U, u64) -> i32>,
    pub lot_state: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_set_state: Option<unsafe extern "C" fn(U, u64, u32, u32, u32)>,
    pub lot_is_livable: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_has_capacity: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_days_since_state_change: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_occupant_wealth: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_is_historical: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_building_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_capacity: Option<unsafe extern "C" fn(U, u64, u32, bool) -> u16>,
    pub lot_available_capacity: Option<unsafe extern "C" fn(U, u64, u32, bool) -> u16>,
    pub lot_set_population: Option<unsafe extern "C" fn(U, u64, u32, u16)>,
    /// The growth stage of the lot's configuration, or -1 without one.
    pub lot_configuration_stage: Option<unsafe extern "C" fn(U, u64) -> i32>,
    // ---- lot configurations ----
    pub cfg_size: Option<unsafe extern "C" fn(U, u64, *mut u32, *mut u32)>,
    pub cfg_min_slope: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub cfg_max_slope: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub cfg_growth_stage: Option<unsafe extern "C" fn(U, u64) -> u8>,
    pub cfg_min_capacity: Option<unsafe extern "C" fn(U, u64) -> u16>,
    pub cfg_max_capacity: Option<unsafe extern "C" fn(U, u64) -> u16>,
    pub cfg_compatible_zone: Option<unsafe extern "C" fn(U, u64, i32) -> bool>,
    pub cfg_compatible_wealth: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub cfg_compatible_purpose: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub cfg_compatible_style: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    /// Writes up to `cap` bytes (0 / 1, configuration frame) and returns the
    /// bit count (only `min(count, cap)` are read).
    pub cfg_required_roads: Option<unsafe extern "C" fn(U, u64, *mut u8, usize) -> usize>,
    /// False when there is no primary building object; else its object-ID
    /// list through the callback.
    pub cfg_primary_building_ids: Option<unsafe extern "C" fn(U, u64, fc4_u32_callback, *mut c_void) -> bool>,
    // ---- lot configuration manager ----
    pub get_lot_configuration: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub register_configuration_filter: Option<unsafe extern "C" fn(U, u32, u32) -> u64>,
    pub unregister_configuration_filter: Option<unsafe extern "C" fn(U, u64)>,
    pub configurations_by_filter: Option<unsafe extern "C" fn(U, u64, fc4_token_callback, *mut c_void)>,
    // ---- city grids and services ----
    pub zone_at: Option<unsafe extern "C" fn(U, i32, i32) -> i32>,
    pub land_value: Option<unsafe extern "C" fn(U, i32, i32) -> u8>,
    pub powered_value: Option<unsafe extern "C" fn(U, i32, i32) -> i32>,
    pub is_cell_watered: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub fire_coverage: Option<unsafe extern "C" fn(U, i32, i32) -> u8>,
    pub park_value: Option<unsafe extern "C" fn(U, i32, i32) -> i16>,
    pub road_access_value: Option<unsafe extern "C" fn(U, i32, i32) -> i32>,
    pub city_cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    pub find_network_tile: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    pub network_is_of_type: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub demolish_lot: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub redisplay_terrain: Option<unsafe extern "C" fn(U, *const fc4_cell_rect)>,
}

/// What one `fc4_tract_grow` did (fc4.h `fc4_tract_grow_result`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct fc4_tract_grow_result {
    pub struct_size: u32,
    /// The return value (§4.3): the capacity started (may be negative).
    pub built: i32,
    pub out_flag: u8,
    pub reserved: [u8; 3],
    /// Codes appended to the outcome list.
    pub outcome_count: u32,
    /// The first 16 of them.
    pub outcomes: [u32; 16],
    /// Candidates scored (§4.14), takeover candidates included.
    pub scored: u32,
    /// `desired` / `desiredMax` (§4.2).
    pub desired: i32,
    pub desired_max: i32,
    /// The last configuration and building `PickBuilding` chose (0: none).
    pub reserved1: u32,
    pub chosen_cfg: u64,
    pub chosen_building: u32,
    pub reserved2: u32,
}

/// The style state, flags and counters (fc4.h `fc4_tract_state`). Counters
/// run from creation.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct fc4_tract_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub configured: u8,
    /// `IsUsingAllStylesAtOnce` (slot 11).
    pub all_styles: u8,
    pub reserved: u8,
    /// `GetCurrentStyle` (slot 8).
    pub current_style: u32,
    pub current_style_index: i32,
    pub years_since_style_change: i32,
    /// `GetYearsBetweenStyleChanges` (slot 13).
    pub years_between_style_changes: i32,
    pub active_style_count: u32,
    /// The first 16 active styles.
    pub active_styles: [u32; 16],
    pub filters: u32,
    pub grow_calls: u32,
    pub decline_calls: u32,
    pub update_population_calls: u32,
    pub reserved1: u32,
    /// Sums of `Grow` and `Decline` results.
    pub built_total: i64,
    pub declined_total: i64,
    /// Outcome codes appended to `Grow`'s lists, by code (§4.1).
    pub outcomes: [u32; 28],
    pub scored: u32,
    /// Host writes the core asked for (calls, whatever the host answered).
    pub constructions: u32,
    pub reoccupations: u32,
    pub abandonments: u32,
    pub lots_created: u32,
    pub lots_deleted: u32,
    pub demolitions: u32,
    pub populations_set: u32,
    pub resource_claims: u32,
    /// Calls refused because the object was already inside the core.
    pub reentrant: u32,
    /// Shadow: game writes swallowed.
    pub swallowed: u32,
    pub reserved2: u32,
}

/// The tuning (§4.21, fc4.h `fc4_tract_tuning`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct fc4_tract_tuning {
    pub struct_size: u32,
    pub overbuild: f32,
    pub reoccupy: [f32; 2],
    pub redevelop_size: f32,
    pub demolition_costs: [f32; 2],
    pub new_lot_costs: [f32; 2],
    pub occupancy_variation: f32,
    pub min_vacancy: [f32; 2],
    pub kick_out: u8,
    pub reserved: [u8; 3],
    pub available_style_count: u32,
    pub available_styles: [u32; 16],
}

const _: () = {
    assert!(std::mem::size_of::<fc4_tract_profile>() == 24);
    assert!(std::mem::size_of::<fc4_tract_construction>() == 32);
    assert!(std::mem::size_of::<fc4_tract_building_profile>() == 12);
    assert!(std::mem::size_of::<fc4_tract_grow_result>() == 112);
    assert!(std::mem::size_of::<fc4_tract_state>() == 288);
    assert!(std::mem::size_of::<fc4_tract_tuning>() == 120);
    assert!(std::mem::size_of::<fc4_tract_host>() == if cfg!(target_pointer_width = "32") { 368 } else { 736 });
};

// ---- host adapter -----------------------------------------------------------------------------------

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.c.$f { Some(f) => unsafe { f($s.c.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

unsafe extern "C" fn push_u32(ctx: *mut c_void, v: u32) {
    if let Some(out) = (ctx as *mut Vec<u32>).as_mut() {
        out.push(v);
    }
}

unsafe extern "C" fn push_style(ctx: *mut c_void, group: u32, fraction: f32) {
    if let Some(out) = (ctx as *mut Vec<(u32, f32)>).as_mut() {
        out.push((group, fraction));
    }
}

fn collect_u32(f: impl FnOnce(fc4_u32_callback, *mut c_void)) -> Vec<u32> {
    let mut v: Vec<u32> = Vec::new();
    f(Some(push_u32), &mut v as *mut Vec<u32> as *mut c_void);
    v
}

/// Counters the host keeps (`fc4_tract_state`).
#[derive(Clone, Copy, Debug, Default)]
struct Counters {
    constructions: u32,
    reoccupations: u32,
    abandonments: u32,
    lots_created: u32,
    lots_deleted: u32,
    demolitions: u32,
    populations_set: u32,
    resource_claims: u32,
    swallowed: u32,
}

pub(crate) struct CHost {
    c: fc4_tract_host,
    /// Shadow mode (see the module notes).
    shadow: bool,
    counters: Cell<Counters>,
    /// Shadow: the `SetPopulation` calls of the current call.
    populations: RefCell<Vec<(u64, u32, u16)>>,
}

impl CHost {
    pub(crate) fn new(c: fc4_tract_host, shadow: bool) -> CHost {
        CHost {
            c,
            shadow,
            counters: Cell::new(Counters::default()),
            populations: RefCell::new(Vec::new()),
        }
    }

    fn note(&self, f: impl FnOnce(&mut Counters)) {
        let mut t = self.counters.get();
        f(&mut t);
        self.counters.set(t);
    }

    /// True (and counted) when the write is swallowed (shadow mode).
    fn swallow(&self) -> bool {
        if self.shadow {
            self.note(|t| t.swallowed = t.swallowed.wrapping_add(1));
        }
        self.shadow
    }

    fn rect_of(&self, f: impl FnOnce(*mut fc4_cell_rect)) -> CellRect {
        let mut r = fc4_cell_rect::default();
        f(&mut r);
        r.into()
    }
}

impl TractDeveloperHost for CHost {
    // ---- lifecycle ----
    fn acquire_services(&self) -> bool {
        if self.swallow() {
            return true;
        }
        cb!(self, acquire_services, false)
    }
    fn add_agent(&self, level: u32, message: u32) -> bool {
        if self.swallow() {
            return true;
        }
        cb!(self, add_agent, false, level, message)
    }
    fn acquire_message_server(&self) -> bool {
        if self.swallow() {
            return true;
        }
        cb!(self, acquire_message_server, false)
    }
    fn release_message_server(&self) {
        if self.swallow() {
            return;
        }
        cb!(self, release_message_server, ())
    }
    fn remove_agent(&self) {
        if self.swallow() {
            return;
        }
        cb!(self, remove_agent, ())
    }
    fn release_services(&self) {
        if self.swallow() {
            return;
        }
        cb!(self, release_services, ())
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }

    // ---- growth developer ----
    fn dev_target_zone(&self, dev: u64) -> i32 {
        cb!(self, dev_target_zone, 0, dev)
    }
    fn dev_subtype(&self, dev: u64) -> u32 {
        cb!(self, dev_subtype, 0, dev)
    }
    fn dev_purpose(&self, dev: u64) -> u32 {
        cb!(self, dev_purpose, 0, dev)
    }
    fn dev_wealth(&self, dev: u64) -> u32 {
        cb!(self, dev_wealth, 0, dev)
    }
    fn dev_construction_info(&self, dev: u64) -> ConstructionProfile {
        let mut o = fc4_tract_construction::default();
        cb!(self, dev_construction_info, (), dev, &mut o);
        ConstructionProfile {
            zone: o.zone,
            unknown2: o.unknown2,
            wealth: o.wealth,
            purpose: o.purpose,
            land_value_min: o.land_value_min,
            land_value_max: o.land_value_max,
            building_type: o.building_type,
            x: o.x,
            z: o.z,
        }
    }
    fn dev_desirability_tract_size(&self, dev: u64) -> i32 {
        cb!(self, dev_desirability_tract_size, 0, dev)
    }
    fn dev_desirability_shift(&self, dev: u64) -> u32 {
        cb!(self, dev_desirability_shift, 0, dev)
    }
    fn dev_desirability_value(&self, dev: u64, tx: i32, tz: i32) -> i16 {
        cb!(self, dev_desirability_value, 0, dev, tx, tz)
    }
    fn dev_capacity_value(&self, dev: u64, tx: i32, tz: i32) -> u32 {
        cb!(self, dev_capacity_value, 0, dev, tx, tz)
    }
    fn dev_source_demand(&self, dev: u64) -> Option<f32> {
        let mut v = 0.0f32;
        cb!(self, dev_source_demand, false, dev, &mut v).then_some(v)
    }
    fn dev_cap(&self, dev: u64, cap: Cap) -> u8 {
        let which = match cap {
            Cap::Power => FC4_TRACT_CAP_POWER,
            Cap::Water => FC4_TRACT_CAP_WATER,
            Cap::Fire => FC4_TRACT_CAP_FIRE,
            Cap::Park => FC4_TRACT_CAP_PARK,
        };
        cb!(self, dev_cap, 0, dev, which)
    }
    fn dev_monopoly_use(&self, dev: u64) -> i32 {
        cb!(self, dev_monopoly_use, 0, dev)
    }
    fn dev_style_percentages(&self, dev: u64) -> Vec<(u32, f32)> {
        let mut v: Vec<(u32, f32)> = Vec::new();
        cb!(
            self,
            dev_style_percentages,
            (),
            dev,
            Some(push_style),
            &mut v as *mut Vec<(u32, f32)> as *mut c_void
        );
        v
    }

    // ---- BDS ----
    fn building_profile(&self, building: u32) -> BuildingProfile {
        let mut o = fc4_tract_building_profile::default();
        cb!(self, building_profile, (), building, &mut o);
        BuildingProfile {
            // The core keeps bytes (values above 255 do not occur: 0..9 and 0..3).
            purpose: o.purpose.min(255) as u8,
            wealth: o.wealth.min(255) as u8,
            capacity: o.capacity,
            ..BuildingProfile::default()
        }
    }
    fn building_types_by_family(&self, family: u32) -> Vec<u32> {
        collect_u32(|f, ctx| cb!(self, building_types_by_family, (), family, f, ctx))
    }
    fn building_in_occupant_group(&self, building: u32, group: u32) -> bool {
        cb!(self, building_in_occupant_group, false, building, group)
    }
    fn building_is_monopoly(&self, building: u32) -> bool {
        cb!(self, building_is_monopoly, false, building)
    }
    fn building_field_ids(&self, building: u32) -> Option<Vec<u32>> {
        let mut found = false;
        let v = collect_u32(|f, ctx| found = cb!(self, building_field_ids, false, building, f, ctx));
        found.then_some(v)
    }
    fn developer_for(&self, purpose: u32, wealth: u32) -> Option<u64> {
        opt(cb!(self, developer_for, 0, purpose, wealth))
    }
    fn claim_construction_resources(&self, amount: u16) -> bool {
        self.note(|t| t.resource_claims = t.resource_claims.wrapping_add(1));
        if self.swallow() {
            return false;
        }
        cb!(self, claim_construction_resources, false, amount)
    }
    fn release_construction_resources(&self, amount: u16) {
        if self.swallow() {
            return;
        }
        cb!(self, release_construction_resources, (), amount)
    }
    fn cheat(&self, cheat: Cheat) -> bool {
        let which = match cheat {
            Cheat::IgnorePlumbing => FC4_TRACT_CHEAT_IGNORE_PLUMBING,
            Cheat::IgnorePower => FC4_TRACT_CHEAT_IGNORE_POWER,
            Cheat::IgnoreCaps => FC4_TRACT_CHEAT_IGNORE_CAPS,
            Cheat::DebugGrowth => FC4_TRACT_CHEAT_DEBUG_GROWTH,
        };
        cb!(self, cheat, false, which)
    }

    // ---- lot developer ----
    fn needs_anchor_building(&self, rect: CellRect, purpose: u32) -> bool {
        let r = fc4_cell_rect::from(rect);
        cb!(self, needs_anchor_building, false, &r, purpose)
    }
    fn start_construction(&self, lot: u64, cfg: u64, building: u32, flag: bool) -> bool {
        self.note(|t| t.constructions = t.constructions.wrapping_add(1));
        if self.swallow() {
            return false;
        }
        cb!(self, start_construction, false, lot, cfg, building, flag)
    }
    fn abandon_building(&self, lot: u64, reason: u32) -> bool {
        self.note(|t| t.abandonments = t.abandonments.wrapping_add(1));
        if self.swallow() {
            return false;
        }
        cb!(self, abandon_building, false, lot, reason)
    }
    fn reoccupy_building(&self, lot: u64, dev_type: u32) -> bool {
        self.note(|t| t.reoccupations = t.reoccupations.wrapping_add(1));
        if self.swallow() {
            return false;
        }
        cb!(self, reoccupy_building, false, lot, dev_type)
    }
    fn area_slope(&self, rect: CellRect) -> f32 {
        let r = fc4_cell_rect::from(rect);
        cb!(self, area_slope, 0.0, &r)
    }

    // ---- lot manager ----
    fn get_lot(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, get_lot, 0, x, z))
    }
    fn lots_in_cell_rect(&self, rect: CellRect) -> Vec<u64> {
        let r = fc4_cell_rect::from(rect);
        collect_tokens(|f, ctx| cb!(self, lots_in_cell_rect, (), &r, f, ctx))
    }
    fn lots_surrounding_lot(&self, lot: u64, flags: u32) -> Vec<u64> {
        collect_tokens(|f, ctx| cb!(self, lots_surrounding_lot, (), lot, flags, f, ctx))
    }
    fn lots_surrounding_rect(&self, rect: CellRect, flags: u32) -> Vec<u64> {
        let r = fc4_cell_rect::from(rect);
        collect_tokens(|f, ctx| cb!(self, lots_surrounding_rect, (), &r, flags, f, ctx))
    }
    fn can_create_lot(&self, x: i32, z: i32, w: i32, h: i32) -> i32 {
        cb!(self, can_create_lot, 0, x, z, w, h)
    }
    fn create_lot_if_possible(&self, x: i32, z: i32, w: i32, h: i32, facing: i32) -> Option<u64> {
        self.note(|t| t.lots_created = t.lots_created.wrapping_add(1));
        if self.swallow() {
            return None;
        }
        opt(cb!(self, create_lot_if_possible, 0, x, z, w, h, facing))
    }
    fn delete_lot(&self, lot: u64) -> bool {
        self.note(|t| t.lots_deleted = t.lots_deleted.wrapping_add(1));
        if self.swallow() {
            return false;
        }
        cb!(self, delete_lot, false, lot)
    }

    // ---- lots ----
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.rect_of(|r| cb!(self, lot_bounding_rect, (), lot, r))
    }
    fn lot_slope(&self, lot: u64) -> f32 {
        cb!(self, lot_slope, 0.0, lot)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        let (mut x, mut z) = (0i32, 0i32);
        cb!(self, lot_location, (), lot, &mut x, &mut z);
        (x, z)
    }
    fn lot_size(&self, lot: u64) -> (u32, u32) {
        let (mut w, mut h) = (0u32, 0u32);
        cb!(self, lot_size, (), lot, &mut w, &mut h);
        (w, h)
    }
    fn lot_facing(&self, lot: u64) -> i32 {
        cb!(self, lot_facing, 0, lot)
    }
    fn lot_set_facing(&self, lot: u64, facing: i32) {
        if self.swallow() {
            return;
        }
        cb!(self, lot_set_facing, (), lot, facing)
    }
    fn lot_has_road_access(&self, lot: u64) -> bool {
        cb!(self, lot_has_road_access, false, lot)
    }
    fn lot_has_organic_road_access(&self, lot: u64, purpose: u32) -> bool {
        cb!(self, lot_has_organic_road_access, false, lot, purpose)
    }
    fn lot_zone_type(&self, lot: u64) -> i32 {
        cb!(self, lot_zone_type, 0, lot)
    }
    fn lot_state(&self, lot: u64) -> u32 {
        cb!(self, lot_state, 0, lot)
    }
    fn lot_set_state(&self, lot: u64, state: u32, wealth: u32, reason: u32) {
        if self.swallow() {
            return;
        }
        cb!(self, lot_set_state, (), lot, state, wealth, reason)
    }
    fn lot_is_livable(&self, lot: u64) -> bool {
        cb!(self, lot_is_livable, false, lot)
    }
    fn lot_has_capacity(&self, lot: u64) -> bool {
        cb!(self, lot_has_capacity, false, lot)
    }
    fn lot_days_since_state_change(&self, lot: u64) -> u32 {
        cb!(self, lot_days_since_state_change, 0, lot)
    }
    fn lot_occupant_wealth(&self, lot: u64) -> u32 {
        cb!(self, lot_occupant_wealth, 0, lot)
    }
    fn lot_is_historical(&self, lot: u64) -> bool {
        cb!(self, lot_is_historical, false, lot)
    }
    fn lot_building_type(&self, lot: u64) -> u32 {
        cb!(self, lot_building_type, 0, lot)
    }
    fn lot_capacity(&self, lot: u64, dev_type: u32, flag: bool) -> u16 {
        cb!(self, lot_capacity, 0, lot, dev_type, flag)
    }
    fn lot_available_capacity(&self, lot: u64, dev_type: u32, flag: bool) -> u16 {
        cb!(self, lot_available_capacity, 0, lot, dev_type, flag)
    }
    fn lot_set_population(&self, lot: u64, dev_type: u32, population: u16) {
        self.note(|t| t.populations_set = t.populations_set.wrapping_add(1));
        if self.shadow {
            self.populations.borrow_mut().push((lot, dev_type, population));
            self.note(|t| t.swallowed = t.swallowed.wrapping_add(1));
            return;
        }
        cb!(self, lot_set_population, (), lot, dev_type, population)
    }
    fn lot_configuration_stage(&self, lot: u64) -> Option<u8> {
        let s = cb!(self, lot_configuration_stage, -1, lot);
        (s >= 0).then_some(s.min(255) as u8)
    }

    // ---- lot configurations ----
    fn cfg_size(&self, cfg: u64) -> (u32, u32) {
        let (mut w, mut h) = (0u32, 0u32);
        cb!(self, cfg_size, (), cfg, &mut w, &mut h);
        (w, h)
    }
    fn cfg_min_slope(&self, cfg: u64) -> f32 {
        cb!(self, cfg_min_slope, 0.0, cfg)
    }
    fn cfg_max_slope(&self, cfg: u64) -> f32 {
        cb!(self, cfg_max_slope, 0.0, cfg)
    }
    fn cfg_growth_stage(&self, cfg: u64) -> u8 {
        cb!(self, cfg_growth_stage, 0, cfg)
    }
    fn cfg_min_capacity(&self, cfg: u64) -> u16 {
        cb!(self, cfg_min_capacity, 0, cfg)
    }
    fn cfg_max_capacity(&self, cfg: u64) -> u16 {
        cb!(self, cfg_max_capacity, 0, cfg)
    }
    fn cfg_compatible_zone(&self, cfg: u64, zone: i32) -> bool {
        cb!(self, cfg_compatible_zone, false, cfg, zone)
    }
    fn cfg_compatible_wealth(&self, cfg: u64, wealth: u32) -> bool {
        cb!(self, cfg_compatible_wealth, false, cfg, wealth)
    }
    fn cfg_compatible_purpose(&self, cfg: u64, purpose: u32) -> bool {
        cb!(self, cfg_compatible_purpose, false, cfg, purpose)
    }
    fn cfg_compatible_style(&self, cfg: u64, style: u32) -> bool {
        cb!(self, cfg_compatible_style, false, cfg, style)
    }
    fn cfg_required_roads(&self, cfg: u64) -> Vec<bool> {
        let mut bits = [0u8; FC4_TRACT_MAX_ROAD_BITS];
        let n = cb!(self, cfg_required_roads, 0, cfg, bits.as_mut_ptr(), bits.len());
        bits[..n.min(bits.len())].iter().map(|&b| b != 0).collect()
    }
    fn cfg_primary_building_ids(&self, cfg: u64) -> Option<Vec<u32>> {
        let mut found = false;
        let v = collect_u32(|f, ctx| found = cb!(self, cfg_primary_building_ids, false, cfg, f, ctx));
        found.then_some(v)
    }

    // ---- lot configuration manager ----
    fn get_lot_configuration(&self, id: u32) -> Option<u64> {
        opt(cb!(self, get_lot_configuration, 0, id))
    }
    fn register_configuration_filter(&self, wealth: u32, purpose: u32) -> u64 {
        // A shadow core never grows, so it never lists configurations.
        if self.swallow() {
            return 0;
        }
        cb!(self, register_configuration_filter, 0, wealth, purpose)
    }
    fn unregister_configuration_filter(&self, filter: u64) {
        if self.swallow() {
            return;
        }
        cb!(self, unregister_configuration_filter, (), filter)
    }
    fn configurations_by_filter(&self, filter: u64) -> Vec<u64> {
        if self.shadow {
            return Vec::new();
        }
        collect_tokens(|f, ctx| cb!(self, configurations_by_filter, (), filter, f, ctx))
    }

    // ---- city grids and services ----
    fn zone_at(&self, x: i32, z: i32) -> i32 {
        cb!(self, zone_at, 0, x, z)
    }
    fn land_value(&self, x: i32, z: i32) -> u8 {
        cb!(self, land_value, 0, x, z)
    }
    fn powered_value(&self, x: i32, z: i32) -> i32 {
        cb!(self, powered_value, 0, x, z)
    }
    fn is_cell_watered(&self, x: i32, z: i32) -> bool {
        cb!(self, is_cell_watered, false, x, z)
    }
    fn fire_coverage(&self, x: i32, z: i32) -> u8 {
        cb!(self, fire_coverage, 0, x, z)
    }
    fn park_value(&self, x: i32, z: i32) -> i16 {
        cb!(self, park_value, 0, x, z)
    }
    fn road_access_value(&self, x: i32, z: i32) -> i32 {
        cb!(self, road_access_value, 0, x, z)
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0i32, 0i32);
        cb!(self, city_cell_counts, (), &mut x, &mut z);
        (x, z)
    }
    fn find_network_tile(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, find_network_tile, 0, x, z))
    }
    fn network_is_of_type(&self, tile: u64, network_type: u32) -> bool {
        cb!(self, network_is_of_type, false, tile, network_type)
    }
    fn demolish_lot(&self, lot: u64) -> bool {
        self.note(|t| t.demolitions = t.demolitions.wrapping_add(1));
        if self.swallow() {
            return false;
        }
        cb!(self, demolish_lot, false, lot)
    }
    fn redisplay_terrain(&self, rect: CellRect) {
        if self.swallow() {
            return;
        }
        let r = fc4_cell_rect::from(rect);
        cb!(self, redisplay_terrain, (), &r)
    }
}

// ---- process-wide statics -----------------------------------------------------------------------------

thread_local! {
    /// The class-wide statics of §9.1 for this process (see the module notes).
    static SHARED: RefCell<Option<SharedStatics>> = const { RefCell::new(None) };
}

/// The process-wide statics, created (and the file-local generator seeded
/// from `session_seed`) on first use.
fn shared(session_seed: u32) -> SharedStatics {
    SHARED.with(|s| {
        s.borrow_mut()
            .get_or_insert_with(|| {
                // derive_seed never returns 0xFFFFFFFF, so the clock values
                // are never read (foundations §6.3).
                let seeds = FixedSeeds {
                    wall_clock: session_seed,
                    process_clock: session_seed,
                };
                TractDeveloperStatics::from_session(session_seed, &seeds)
            })
            .clone()
    })
}

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_tract`).
#[repr(C)]
pub struct fc4_tract {
    _private: [u8; 0],
}

/// Running totals of the slot calls (`fc4_tract_state`).
#[derive(Clone, Copy, Debug, Default)]
struct Totals {
    grow_calls: u32,
    decline_calls: u32,
    update_population_calls: u32,
    built_total: i64,
    declined_total: i64,
    outcomes: [u32; 28],
    scored: u32,
    reentrant: u32,
}

struct Handle {
    host: Rc<CHost>,
    dev: RefCell<TractDeveloper>,
    totals: Cell<Totals>,
}

impl Handle {
    fn new(host: Rc<CHost>, statics: SharedStatics) -> Handle {
        let dev = TractDeveloper::new(host.clone(), statics);
        Handle {
            host,
            dev: RefCell::new(dev),
            totals: Cell::new(Totals::default()),
        }
    }

    fn note(&self, f: impl FnOnce(&mut Totals)) {
        let mut t = self.totals.get();
        f(&mut t);
        self.totals.set(t);
    }
}

/// Runs `f` on the core; a call while the core is already in use (a
/// re-entrant call) gets `fallback` and is counted.
fn dev<R>(p: *const fc4_tract, fallback: R, f: impl FnOnce(&mut TractDeveloper, &Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_tract*.
    let Some(h) = (unsafe { (p as *const Handle).as_ref() }) else {
        return fallback;
    };
    let Ok(mut d) = h.dev.try_borrow_mut() else {
        h.note(|t| t.reentrant = t.reentrant.wrapping_add(1));
        return fallback;
    };
    guard(fallback, || f(&mut d, h))
}

pub(crate) fn host_ok(host: *const fc4_tract_host) -> Option<fc4_tract_host> {
    // SAFETY: documented precondition: NULL or a valid host table.
    let h = unsafe { host.as_ref() }?;
    ((h.struct_size as usize) >= std::mem::size_of::<fc4_tract_host>()).then_some(*h)
}

/// `QueryInterface(iid)` (§1.1): whether the tract developer answers the IID
/// (`1`, `0xA818ED1B`, `0x452294AA`, `0xE4FDA3D4`).
#[no_mangle]
pub extern "C" fn fc4_tract_query_interface(iid: u32) -> bool {
    answers_iid(iid)
}

/// The class ID (`GetGZCLSID`, §1.1): `0x2990C142`.
#[no_mangle]
pub extern "C" fn fc4_tract_class_id() -> u32 {
    CLSID_TRACT_DEVELOPER
}

/// The configuration filter's `IsLotConfigurationIncluded(cfg)` (§4.6, §7.4)
/// for `(wealth, purpose)`, through `host`'s configuration reads. Needs no
/// object: the plugin's filter calls it while the core is inside
/// `GetLotConfigurationsByFilter`.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_filter_includes(
    host: *const fc4_tract_host,
    cfg: u64,
    wealth: u32,
    purpose: u32,
) -> bool {
    guard(false, || {
        let Some(h) = host_ok(host) else { return false };
        configuration_filter_includes(&CHost::new(h, false), cfg, wealth, purpose)
    })
}

fn create(host: *const fc4_tract_host, shadow: bool, session_seed: u32) -> *mut fc4_tract {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            return std::ptr::null_mut();
        };
        let statics = if shadow {
            // Never the replacement's statics; a shadow core draws nothing
            // from them (no Grow, no Decline).
            TractDeveloperStatics::from_session(
                0,
                &FixedSeeds {
                    wall_clock: 1,
                    process_clock: 1,
                },
            )
        } else {
            shared(session_seed)
        };
        Box::into_raw(Box::new(Handle::new(Rc::new(CHost::new(h, shadow)), statics))) as *mut fc4_tract
    })
}

/// Creates a tract developer (the constructor's state, §2.1). The host table
/// is copied. The first object of the process seeds the shared statics from
/// `session_seed` (see the module notes); later objects share them. NULL
/// when the table is incomplete.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_create(host: *const fc4_tract_host, session_seed: u32) -> *mut fc4_tract {
    create(host, false, session_seed)
}

/// Creates a tract developer in shadow mode (see the module notes).
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_create_shadow(host: *const fc4_tract_host) -> *mut fc4_tract {
    create(host, true, 0)
}

/// Destroys the object. Nothing is released through the host (a filter still
/// registered belongs to the host: call `fc4_tract_shutdown` first).
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_destroy(p: *mut fc4_tract) {
    if p.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(p as *mut Handle)));
}

/// `Init` (§1.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_init(p: *mut fc4_tract) -> bool {
    dev(p, false, |d, _| d.init())
}

/// `Shutdown` (§1.4): always true. The style state is kept.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_shutdown(p: *mut fc4_tract) -> bool {
    dev(p, true, |d, _| d.shutdown())
}

/// `DoMessage` (§3): Year (`0x66956817`) runs `NewYear`; always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_do_message(p: *mut fc4_tract, msg_type: u32) -> bool {
    dev(p, true, |d, _| d.do_message(msg_type))
}

/// Slot 5 `Grow(profile, outcomes&, outFlag&)` (§4.3): returns `built`.
/// Every code the original appends to the outcome list goes to `outcome` (in
/// order, after the core returns: nothing reads the list during the call);
/// `out_flag` (may be NULL) receives `outFlag`, `out` (may be NULL) the
/// details.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_grow(
    p: *mut fc4_tract,
    profile: *const fc4_tract_profile,
    outcome: fc4_u32_callback,
    ctx: *mut c_void,
    out_flag: *mut bool,
    out: *mut fc4_tract_grow_result,
) -> i32 {
    let Some(pr) = profile.as_ref() else { return 0 };
    let gp = GrowthProfile::from(pr);
    let r = dev(p, None, |d, h| {
        let mut codes: Vec<Outcome> = Vec::new();
        let mut flag = false;
        let built = d.grow(&gp, &mut codes, &mut flag);
        let scored = d.last_scored();
        h.note(|t| {
            t.grow_calls = t.grow_calls.wrapping_add(1);
            t.built_total = t.built_total.wrapping_add(i64::from(built));
            t.scored = t.scored.wrapping_add(scored);
            for c in &codes {
                if let Some(n) = t.outcomes.get_mut(c.code() as usize) {
                    *n = n.wrapping_add(1);
                }
            }
        });
        let (desired, desired_max) = d.desired();
        let (cfg, building) = d.chosen();
        Some((
            built,
            flag,
            codes,
            scored,
            desired,
            desired_max,
            cfg.unwrap_or(0),
            building,
        ))
    });
    let Some((built, flag, codes, scored, desired, desired_max, cfg, building)) = r else {
        return 0;
    };
    if let Some(f) = outcome {
        for c in &codes {
            f(ctx, c.code());
        }
    }
    if let Some(o) = out_flag.as_mut() {
        *o = flag;
    }
    if let Some(o) = out.as_mut() {
        let mut first = [0u32; 16];
        for (slot, c) in first.iter_mut().zip(codes.iter()) {
            *slot = c.code();
        }
        *o = fc4_tract_grow_result {
            struct_size: std::mem::size_of::<fc4_tract_grow_result>() as u32,
            built,
            out_flag: u8::from(flag),
            outcome_count: codes.len() as u32,
            outcomes: first,
            scored,
            desired,
            desired_max,
            chosen_cfg: cfg,
            chosen_building: building,
            ..Default::default()
        };
    }
    built
}

/// Slot 6 `Decline(profile)` (§4.19): the capacity abandoned.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_decline(p: *mut fc4_tract, profile: *const fc4_tract_profile) -> i32 {
    let Some(pr) = profile.as_ref() else { return 0 };
    let gp = GrowthProfile::from(pr);
    dev(p, 0, |d, h| {
        let sum = d.decline(&gp);
        h.note(|t| {
            t.decline_calls = t.decline_calls.wrapping_add(1);
            t.declined_total = t.declined_total.wrapping_add(i64::from(sum));
        });
        sum
    })
}

/// Slot 7 `UpdatePopulation(profile)` (§4.18). The slot returns 0 (§7.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_update_population(p: *mut fc4_tract, profile: *const fc4_tract_profile) {
    let Some(pr) = profile.as_ref() else { return };
    let gp = GrowthProfile::from(pr);
    dev(p, (), |d, h| {
        d.update_population(&gp);
        h.note(|t| t.update_population_calls = t.update_population_calls.wrapping_add(1));
    })
}

/// Slot 8 `GetCurrentStyle` (§4.20).
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_current_style(p: *const fc4_tract) -> u32 {
    dev(p, 0, |d, _| d.current_style())
}

/// Slot 9 `GetActiveStyles` (§4.20): writes up to `cap` styles to `out` (may
/// be NULL) and returns how many there are.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_active_styles(p: *const fc4_tract, out: *mut u32, cap: usize) -> usize {
    dev(p, 0, |d, _| {
        let v = d.active_styles();
        if !out.is_null() {
            let n = v.len().min(cap);
            std::ptr::copy_nonoverlapping(v.as_ptr(), out, n);
        }
        v.len()
    })
}

/// Slot 10 `SetActiveStyles(v)` (§4.20). `v` may be NULL when `n` is 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_set_active_styles(p: *mut fc4_tract, v: *const u32, n: usize) {
    let styles: Vec<u32> = if v.is_null() || n == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(v, n).to_vec()
    };
    dev(p, (), |d, _| d.set_active_styles(&styles))
}

/// Slot 11 `IsUsingAllStylesAtOnce`.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_is_using_all_styles(p: *const fc4_tract) -> bool {
    dev(p, false, |d, _| d.is_using_all_styles_at_once())
}

/// Slot 12 `UseAllStylesAtOnce(f)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_use_all_styles(p: *mut fc4_tract, all: bool) {
    dev(p, (), |d, _| d.use_all_styles_at_once(all))
}

/// Slot 13 `GetYearsBetweenStyleChanges`.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_years_between_style_changes(p: *const fc4_tract) -> i32 {
    dev(p, 0, |d, _| d.years_between_style_changes())
}

/// Slot 14 `SetYearsBetweenStyleChanges(n)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_set_years_between_style_changes(p: *mut fc4_tract, years: i32) {
    dev(p, (), |d, _| d.set_years_between_style_changes(years))
}

/// The debug histogram of §4.1 (off by default; it is reset by every `Grow`).
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_set_histogram(p: *mut fc4_tract, on: bool) {
    dev(p, (), |d, _| d.set_histogram_enabled(on))
}

/// The histogram of the last `Grow`: 13 rows of 28 counters, row-major, into
/// `out` (at least `13 * 28` entries). False for a NULL buffer.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_histogram(p: *const fc4_tract, out: *mut u32, cap: usize) -> bool {
    if out.is_null() || cap < HIST_ROWS * HIST_COLUMNS {
        return false;
    }
    dev(p, false, |d, _| {
        let o = std::slice::from_raw_parts_mut(out, HIST_ROWS * HIST_COLUMNS);
        for (r, row) in d.histogram().iter().enumerate() {
            o[r * HIST_COLUMNS..(r + 1) * HIST_COLUMNS].copy_from_slice(row);
        }
        true
    })
}

/// The style state, flags and counters. False for NULL arguments or while
/// the core is in use (a re-entrant call).
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_get_state(p: *const fc4_tract, out: *mut fc4_tract_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    dev(p, false, |d, h| {
        let t = h.totals.get();
        let c = h.host.counters.get();
        let styles = d.active_styles();
        let mut first = [0u32; 16];
        for (slot, s) in first.iter_mut().zip(styles.iter()) {
            *slot = *s;
        }
        *o = fc4_tract_state {
            struct_size: std::mem::size_of::<fc4_tract_state>() as u32,
            initialized: u8::from(d.is_initialised()),
            configured: u8::from(d.is_configured()),
            all_styles: u8::from(d.is_using_all_styles_at_once()),
            reserved: 0,
            current_style: d.current_style(),
            current_style_index: d.current_style_index(),
            years_since_style_change: d.years_since_style_change(),
            years_between_style_changes: d.years_between_style_changes(),
            active_style_count: styles.len() as u32,
            active_styles: first,
            filters: d.filters().len() as u32,
            grow_calls: t.grow_calls,
            decline_calls: t.decline_calls,
            update_population_calls: t.update_population_calls,
            reserved1: 0,
            built_total: t.built_total,
            declined_total: t.declined_total,
            outcomes: t.outcomes,
            scored: t.scored,
            constructions: c.constructions,
            reoccupations: c.reoccupations,
            abandonments: c.abandonments,
            lots_created: c.lots_created,
            lots_deleted: c.lots_deleted,
            demolitions: c.demolitions,
            populations_set: c.populations_set,
            resource_claims: c.resource_claims,
            reentrant: t.reentrant,
            swallowed: c.swallowed,
            reserved2: 0,
        };
        true
    })
}

/// The tuning (§4.21) as the object holds it.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_get_tuning(p: *const fc4_tract, out: *mut fc4_tract_tuning) -> bool {
    let Some(o) = out.as_mut() else { return false };
    dev(p, false, |d, _| {
        let tu = d.tuning();
        let mut first = [0u32; 16];
        for (slot, s) in first.iter_mut().zip(tu.available_styles.iter()) {
            *slot = *s;
        }
        *o = fc4_tract_tuning {
            struct_size: std::mem::size_of::<fc4_tract_tuning>() as u32,
            overbuild: tu.overbuild,
            reoccupy: tu.reoccupy,
            redevelop_size: tu.redevelop_size_threshold,
            demolition_costs: tu.demolition,
            new_lot_costs: tu.new_lot,
            occupancy_variation: tu.occupancy_variation,
            min_vacancy: tu.min_vacancy,
            kick_out: u8::from(tu.kick_out),
            reserved: [0; 3],
            available_style_count: tu.available_styles.len() as u32,
            available_styles: first,
        };
        true
    })
}

/// `ReadTuning` (§4.21) and its style fix-ups, without the rest of `Init`
/// (the shadow: what `Init` does to a loaded record).
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_read_tuning(p: *mut fc4_tract) {
    dev(p, (), |d, _| d.read_tuning())
}

/// `Write` (§6).
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_write(p: *const fc4_tract, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    dev(p, false, |d, _| d.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§6). Runs before `Init`: nothing is sized from the city here.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_read(p: *mut fc4_tract, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    dev(p, false, |d, _| {
        d.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}

// ---- shadow probes (tooling) -------------------------------------------------------------------------

/// Shadow: `UpdatePopulation(profile)` (§4.18) on a shadow core; every
/// `SetPopulation(devType, n)` it asks for goes to `cb` instead of the game.
/// Returns the calls made, -1 when `p` is not a shadow core.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_shadow_update_population(
    p: *mut fc4_tract,
    profile: *const fc4_tract_profile,
    cb: fc4_tract_population_callback,
    ctx: *mut c_void,
) -> i32 {
    let Some(pr) = profile.as_ref() else { return -1 };
    let gp = GrowthProfile::from(pr);
    let calls = dev(p, None, |d, h| {
        if !h.host.shadow {
            return None;
        }
        h.host.populations.borrow_mut().clear();
        d.update_population(&gp);
        Some(std::mem::take(&mut *h.host.populations.borrow_mut()))
    });
    let Some(calls) = calls else { return -1 };
    if let Some(f) = cb {
        for &(lot, dev_type, n) in &calls {
            f(ctx, lot, dev_type, n);
        }
    }
    calls.len() as i32
}

/// Shadow: `Grow`'s setup (§4.3 steps 1–2) for `profile` with `relaxed` as
/// given, for the probes below (reads only). False when `p` is not a shadow
/// core.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_probe_setup(
    p: *mut fc4_tract,
    profile: *const fc4_tract_profile,
    relaxed: bool,
) -> bool {
    let Some(pr) = profile.as_ref() else { return false };
    let gp = GrowthProfile::from(pr);
    dev(p, false, |d, h| {
        if !h.host.shadow {
            return false;
        }
        d.probe_setup(&gp, relaxed);
        true
    })
}

/// Shadow: `IsLotConfigurationSuitable(cfg)` (§4.7) after
/// `fc4_tract_probe_setup` (not counted). -1 when `p` is not a shadow core.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_probe_configuration_suitable(p: *const fc4_tract, cfg: u64) -> i32 {
    dev(p, -1, |d, h| {
        if !h.host.shadow {
            return -1;
        }
        d.probe_configuration_suitable(cfg) as i32
    })
}

/// Shadow: `IsLotSuitable(lot, false)` (§4.8) after `fc4_tract_probe_setup`.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_probe_lot_suitable(p: *const fc4_tract, lot: u64) -> i32 {
    dev(p, -1, |d, h| {
        if !h.host.shadow {
            return -1;
        }
        d.probe_lot_suitable(lot) as i32
    })
}

/// Shadow: `IsBuildingCompatible(b, c)` (§4.17) with the capacity window
/// `[lo, hi]`, after `fc4_tract_probe_setup`.
#[no_mangle]
pub unsafe extern "C" fn fc4_tract_probe_building_compatible(
    p: *const fc4_tract,
    building: u32,
    lo: i32,
    hi: i32,
) -> i32 {
    dev(p, -1, |d, h| {
        if !h.host.shadow {
            return -1;
        }
        d.probe_building_compatible(building, lo, hi) as i32
    })
}
