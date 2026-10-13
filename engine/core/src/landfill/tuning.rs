// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The landfill developer's tuning (§4.1).
//!
//! The values are **process-wide statics** in the original: a second developer
//! object would overwrite them with the same values. FreeCity4 holds them per
//! developer *[choice]*; a missing property leaves the value unchanged (§4.1),
//! which is what makes the two readings agree for one city.

use crate::property::PropertyHolder;
use crate::props;
use crate::stream::ResKey;
use crate::variant::{mem, Elem, Variant};

/// The tuning exemplar's key (§4.1): "LandfillDeveloper".
pub const TUNING_KEY: ResKey = ResKey {
    type_id: 0x6534_284A,
    group: 0x67CD_5FA1,
    instance: 0x6A28_06AF,
};

/// Tuning property IDs (§4.1).
pub mod prop {
    /// `kLandfillDecayRate` (float).
    pub const DECAY_RATE: u32 = 0x4A28_0510;
    /// `kLevelToleranceThreshold` (float).
    pub const LEVEL_TOLERANCE: u32 = 0x4A28_0513;
    /// `kMaxGarbageToTakePerMonth` (uint32).
    pub const MAX_PER_MONTH: u32 = 0x4A67_7E5B;
    /// `kMaxGarbageToTakePerMonthPerTile` (uint32).
    pub const MAX_PER_MONTH_PER_TILE: u32 = 0x4A67_7E5C;
    /// `kMaxGarbageToDecayPerMonthPerZone` (uint32), applied **per heap**
    /// *[quirk]*.
    pub const MAX_DECAY_PER_ZONE: u32 = 0x4A67_7E5E;
    /// `kDefaultLandfillTileCost` (sint64).
    pub const DEFAULT_TILE_COST: u32 = 0x4A67_7E5F;
    /// `kCostPerCellHeightChange`: stored as `sint64`, converted to a float by
    /// the variant (§4.1).
    pub const COST_PER_CELL_HEIGHT_CHANGE: u32 = 0x2AA0_0FE7;
    /// `kbMoveWithTerrain` (bool).
    pub const MOVE_WITH_TERRAIN: u32 = 0x4A67_7E5D;
}

/// The tuning in effect (§4.1). Every value keeps its previous one when its
/// property is absent; no response curve is loaded (foundations §4.1 does not
/// apply).
#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    /// `0x4A280510`, shipped 0.07 (§4.7.3).
    pub decay_rate: f32,
    /// `0x4A280513`, shipped 5.0 (§4.3).
    pub level_tolerance: f32,
    /// `0x4A677E5B`, shipped 327 680 000 (§4.7.1).
    pub max_per_month: u32,
    /// `0x4A677E5C`, shipped 5000 (§4.7.2).
    pub max_per_month_per_tile: u32,
    /// `0x4A677E5E`, shipped 1500 (§4.7.3), applied per heap *[quirk]*.
    pub max_decay_per_zone: u32,
    /// `0x4A677E5F`, shipped 5 (§4.9.1).
    pub default_tile_cost: i64,
    /// `0x2AA00FE7`, shipped 4 (§4.3).
    pub cost_per_cell_height_change: f32,
    /// `0x4A677E5D`, shipped true (§4.5.2).
    pub move_with_terrain: bool,
}

impl Default for Tuning {
    /// The code defaults (§4.1 "Code default" column).
    fn default() -> Tuning {
        Tuning {
            decay_rate: 0.1,
            level_tolerance: 1.0,
            max_per_month: 40000,
            max_per_month_per_tile: 5000,
            max_decay_per_zone: 3000,
            default_tile_cost: 0,
            cost_per_cell_height_change: 1.0,
            move_with_terrain: true,
        }
    }
}

impl Tuning {
    /// Reads the exemplar once per object on the first `Init` (§4.1) through
    /// `cSCPropertyHelp::GetPropertyValue`. A missing property leaves the
    /// value unchanged. With no exemplar nothing is read.
    pub fn read(&mut self, h: Option<&PropertyHolder>) {
        let Some(h) = h else { return };
        if let Some(v) = props::f32_of(h, prop::DECAY_RATE) {
            self.decay_rate = v;
        }
        if let Some(v) = props::f32_of(h, prop::LEVEL_TOLERANCE) {
            self.level_tolerance = v;
        }
        if let Some(v) = props::u32_of(h, prop::MAX_PER_MONTH) {
            self.max_per_month = v;
        }
        if let Some(v) = props::u32_of(h, prop::MAX_PER_MONTH_PER_TILE) {
            self.max_per_month_per_tile = v;
        }
        if let Some(v) = props::u32_of(h, prop::MAX_DECAY_PER_ZONE) {
            self.max_decay_per_zone = v;
        }
        if let Some(v) = props::i64_of(h, prop::DEFAULT_TILE_COST) {
            self.default_tile_cost = v;
        }
        if let Some(v) = props::f32_of(h, prop::COST_PER_CELL_HEIGHT_CHANGE) {
            self.cost_per_cell_height_change = v;
        }
        if let Some(v) = props::bool_of(h, prop::MOVE_WITH_TERRAIN) {
            self.move_with_terrain = v;
        }
    }
}

/// The shipped "LandfillDeveloper" exemplar's values (§4.1 "Shipped" column),
/// for hosts and tests without the game's data files.
pub fn shipped_tuning() -> PropertyHolder {
    let mut h = PropertyHolder::new();
    h.add(prop::DECAY_RATE, Variant::Scalar(Elem::F32(0.07)));
    h.add(prop::LEVEL_TOLERANCE, Variant::Scalar(Elem::F32(5.0)));
    h.add(prop::MAX_PER_MONTH, Variant::Scalar(Elem::U32(327_680_000)));
    h.add(prop::MAX_PER_MONTH_PER_TILE, Variant::Scalar(Elem::U32(5000)));
    h.add(prop::MAX_DECAY_PER_ZONE, Variant::Scalar(Elem::U32(1500)));
    h.add(prop::DEFAULT_TILE_COST, Variant::Scalar(Elem::I64(5)));
    // §4.1: stored as an s64 and converted to 4.0 by the variant.
    h.add(
        prop::COST_PER_CELL_HEIGHT_CHANGE,
        Variant::Scalar(Elem::I64(4)),
    );
    h.add(prop::MOVE_WITH_TERRAIN, Variant::Scalar(Elem::Bool(true)));
    h
}

/// The shipped tile costs (§4.1 table): the first value of property
/// [`super::PROP_ITEM_COST`] of each tile exemplar.
pub fn shipped_tile_costs() -> [(u32, i64); 6] {
    use super::tile;
    [
        (tile::EMPTY, 1),
        (tile::CORNER, 10),
        (tile::EDGE, 20),
        (tile::DIAGONAL, 20),
        (tile::THREE, 30),
        (tile::INTERIOR, 40),
    ]
}

/// A tile exemplar's holder carrying its shipped cost (§4.1): what the host
/// returns for a tile so that [`super::LandfillHost::occupant_item_cost`] can
/// price maintenance (§4.9.1).
pub fn shipped_tile_exemplar(tile_id: u32) -> Option<PropertyHolder> {
    let cost = shipped_tile_costs().into_iter().find(|(t, _)| *t == tile_id)?.1;
    let mut h = PropertyHolder::new();
    h.add(
        super::PROP_ITEM_COST,
        Variant::array(mem::SINT64, vec![Elem::I64(cost)]),
    );
    Some(h)
}
