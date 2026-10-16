// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The lot developer's tuning (`ReadSimulatorTunableValues`, §3.5).

use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::props;
use crate::variant::{mem, Elem, Variant};

/// Tuning property IDs (§3.5), in the "Building Development" exemplar.
pub mod prop {
    pub const STAY_NEW_TIME: u32 = 0x47E2_C400;
    pub const OCCUPANCY_THRESHOLDS: u32 = 0x47E2_C401;
    pub const MAX_CELL_ALTITUDE_DELTA_FOR_FLATNESS: u32 = 0xE900_EA24;
    pub const MAX_SLOPE_FOR_FLATNESS: u32 = 0xE900_EA25;
    pub const DEFAULT_LOT_FOUNDATION_TYPE: u32 = 0xE900_EA26;
    pub const DEFAULT_BUILDING_FOUNDATION_TYPE: u32 = 0x6916_516B;
    pub const ATTEMPT_LOT_LEVELLING: u32 = 0xE900_EA27;
    pub const ATTEMPT_INTERNAL_LOT_LEVELLING: u32 = 0xE900_EA28;
    pub const AUTO_LEVEL_INTERNAL_ALTITUDE_DELTA: u32 = 0xE900_EA29;
    pub const MAX_AREA_SLOPE_FOR_LEVELLING: u32 = 0xE900_EA2A;
    pub const NIGHT_UPDATE_INTERVAL: u32 = 0xE900_EA2B;
    pub const NIGHT_UPDATE_COUNT: u32 = 0xE900_EA2C;
    pub const COST_VS_SLOPE: u32 = 0x2A2D_7824;
}

/// The tuning exemplar key (§3.5): `{type, group, instance}`.
pub const TUNING_KEY: (u32, u32, u32) = (0x6534_284A, 0xE7E2_C2DB, 0xE8DA_7677);

/// The default foundation type (`0xE900EA26` and `0x6916516B` code default).
pub const DEFAULT_FOUNDATION_TYPE: u32 = 0x890B_7312;

/// The tuning values kept in the tuning block (§3.5). The night interval
/// and count are written straight into the simulator's accumulator and
/// batch size (§2.2), not kept here.
#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    /// `0x47E2C400` (days, TestCondition).
    pub stay_new_time: u16,
    /// `0x47E2C401`: [0] abandon (desirability), [1] distress, [2] renew.
    pub thresholds: [f32; 3],
    pub max_cell_altitude_delta_for_flatness: f32,
    pub max_slope_for_flatness: f32,
    pub default_lot_foundation_type: u32,
    pub default_building_foundation_type: u32,
    pub attempt_lot_levelling: bool,
    pub attempt_internal_lot_levelling: bool,
    /// Read but never used (§3.5 quirk 11).
    pub auto_level_internal_altitude_delta: f32,
    pub max_area_slope_for_levelling: f32,
    /// `0x2A2D7824` (keep pattern; empty until loaded).
    pub cost_curve: ResponseCurve,
}

impl Default for Tuning {
    /// The constructor's values (§3.5 "code default").
    fn default() -> Tuning {
        Tuning {
            stay_new_time: 90,
            thresholds: [0.2, 0.4, 0.6],
            max_cell_altitude_delta_for_flatness: 1.0,
            max_slope_for_flatness: 1.0,
            default_lot_foundation_type: DEFAULT_FOUNDATION_TYPE,
            default_building_foundation_type: DEFAULT_FOUNDATION_TYPE,
            attempt_lot_levelling: true,
            attempt_internal_lot_levelling: true,
            auto_level_internal_altitude_delta: 0.0,
            max_area_slope_for_levelling: 0.0,
            cost_curve: ResponseCurve::default(),
        }
    }
}

impl Tuning {
    /// `ReadSimulatorTunableValues` (§3.5). With no exemplar nothing is
    /// read. Otherwise the three thresholds are first re-set to 0.2, 0.4,
    /// 0.6; every reader leaves its member unchanged when the property is
    /// absent. *[choice]* A threshold array shorter than three values keeps
    /// the default for the missing elements (the original reads past its
    /// end). The night interval goes into `night_acc` and the night count
    /// into `night_batch` (§2.2, quirk 10).
    pub fn read(&mut self, h: Option<&PropertyHolder>, night_acc: &mut u32, night_batch: &mut u32) {
        let Some(h) = h else { return };
        if let Some(v) = props::i64_of(h, prop::STAY_NEW_TIME) {
            self.stay_new_time = v as u16;
        }
        self.thresholds = [0.2, 0.4, 0.6];
        if let Some(v) = props::f32s(h, prop::OCCUPANCY_THRESHOLDS) {
            for (t, x) in self.thresholds.iter_mut().zip(v) {
                *t = x;
            }
        }
        let f = |id: u32, m: &mut f32| {
            if let Some(v) = props::f32_of(h, id) {
                *m = v;
            }
        };
        f(
            prop::MAX_CELL_ALTITUDE_DELTA_FOR_FLATNESS,
            &mut self.max_cell_altitude_delta_for_flatness,
        );
        f(prop::MAX_SLOPE_FOR_FLATNESS, &mut self.max_slope_for_flatness);
        if let Some(v) = props::u32_of(h, prop::DEFAULT_LOT_FOUNDATION_TYPE) {
            self.default_lot_foundation_type = v;
        }
        if let Some(v) = props::u32_of(h, prop::DEFAULT_BUILDING_FOUNDATION_TYPE) {
            self.default_building_foundation_type = v;
        }
        if let Some(v) = props::bool_of(h, prop::ATTEMPT_LOT_LEVELLING) {
            self.attempt_lot_levelling = v;
        }
        if let Some(v) = props::bool_of(h, prop::ATTEMPT_INTERNAL_LOT_LEVELLING) {
            self.attempt_internal_lot_levelling = v;
        }
        f(
            prop::AUTO_LEVEL_INTERNAL_ALTITUDE_DELTA,
            &mut self.auto_level_internal_altitude_delta,
        );
        f(
            prop::MAX_AREA_SLOPE_FOR_LEVELLING,
            &mut self.max_area_slope_for_levelling,
        );
        if let Some(v) = props::u32_of(h, prop::NIGHT_UPDATE_INTERVAL) {
            *night_acc = v;
        }
        if let Some(v) = props::u32_of(h, prop::NIGHT_UPDATE_COUNT) {
            *night_batch = v;
        }
        if let Some(c) = props::curve(h, prop::COST_VS_SLOPE) {
            self.cost_curve = c;
        }
    }

    /// The shipped exemplar's values (§3.5 "Shipped" column), for hosts and
    /// tests without the game's data files.
    pub fn shipped_exemplar() -> PropertyHolder {
        let mut h = PropertyHolder::new();
        let f = |v: f32| Variant::Scalar(Elem::F32(v));
        let u = |v: u32| Variant::Scalar(Elem::U32(v));
        let b = |v: bool| Variant::Scalar(Elem::Bool(v));
        h.add(prop::STAY_NEW_TIME, Variant::Scalar(Elem::U16(90)));
        h.add(
            prop::OCCUPANCY_THRESHOLDS,
            Variant::array(mem::FLOAT32, vec![Elem::F32(0.3), Elem::F32(0.45), Elem::F32(0.5)]),
        );
        h.add(prop::MAX_CELL_ALTITUDE_DELTA_FOR_FLATNESS, f(1.0));
        h.add(prop::MAX_SLOPE_FOR_FLATNESS, f(1.0));
        h.add(prop::DEFAULT_LOT_FOUNDATION_TYPE, u(DEFAULT_FOUNDATION_TYPE));
        h.add(prop::ATTEMPT_LOT_LEVELLING, b(true));
        h.add(prop::ATTEMPT_INTERNAL_LOT_LEVELLING, b(true));
        h.add(prop::AUTO_LEVEL_INTERNAL_ALTITUDE_DELTA, f(20.0));
        h.add(prop::MAX_AREA_SLOPE_FOR_LEVELLING, f(40.0));
        h.add(prop::NIGHT_UPDATE_INTERVAL, u(100));
        h.add(prop::NIGHT_UPDATE_COUNT, u(10));
        let curve = [
            0.0, 0.0, 0.555_556, 10.0, 1.111_11, 20.0, 1.666_67, 40.0, 2.222_22, 80.0, 2.777_78, 170.0, 3.333_33,
            350.0, 3.888_89, 720.0, 4.444_45, 1470.0, 5.0, 3000.0,
        ];
        h.add(
            prop::COST_VS_SLOPE,
            Variant::array(mem::FLOAT32, curve.iter().map(|&v| Elem::F32(v)).collect()),
        );
        h
    }
}
