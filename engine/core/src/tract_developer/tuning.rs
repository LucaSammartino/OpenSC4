// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The tract developer's tuning (`ReadTuning`, §4.21).

use crate::property::PropertyHolder;
use crate::props;
use crate::variant::{mem, Elem, Variant};

/// Tuning property IDs (§4.21), in the "Building Development" exemplar.
pub mod prop {
    pub const OVERBUILD: u32 = 0x47E2_C500;
    pub const REOCCUPY_THRESHOLDS: u32 = 0x47E2_C501;
    pub const REDEVELOP_SIZE_THRESHOLD: u32 = 0x47E2_C502;
    pub const DEMOLITION_COSTS: u32 = 0x47E2_C510;
    pub const NEW_LOT_COSTS: u32 = 0x47E2_C511;
    pub const OCCUPANCY_VARIATION: u32 = 0x47E2_C530;
    pub const MIN_VACANCY_RANGE: u32 = 0x47E2_C531;
    pub const KICK_OUT_LOWER_WEALTH: u32 = 0x47E2_C540;
    pub const AVAILABLE_STYLES: u32 = 0x47E2_C550;
    pub const YEARS_BETWEEN_STYLES: u32 = 0x47E2_C551;
}

/// The tuning exemplar key (§4.21): `{type, group, instance}`.
pub const TUNING_KEY: (u32, u32, u32) = (0x6534_284A, 0xE7E2_C2DB, 0xE8DA_7677);

/// The tuning values (§4.21). Floats are float32 values.
#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    pub overbuild: f32,
    pub reoccupy: [f32; 2],
    pub redevelop_size_threshold: f32,
    pub demolition: [f32; 2],
    pub new_lot: [f32; 2],
    pub occupancy_variation: f32,
    pub min_vacancy: [f32; 2],
    pub kick_out: bool,
    pub available_styles: Vec<u32>,
}

impl Tuning {
    /// The values the constructor leaves (§4.21 "constructor" column).
    pub fn constructor() -> Tuning {
        Tuning {
            min_vacancy: [0.1, 0.1],
            ..Tuning::code_defaults()
        }
    }

    /// The reader's code defaults (§4.21).
    pub fn code_defaults() -> Tuning {
        Tuning {
            overbuild: 1.5,
            reoccupy: [0.0, 1.2],
            redevelop_size_threshold: 1.5,
            demolition: [1.0, 0.2],
            new_lot: [10.0, 5.0],
            occupancy_variation: 0.2,
            min_vacancy: [0.01, 0.1],
            kick_out: true,
            available_styles: Vec::new(),
        }
    }

    /// `ReadTuning` (§4.21): every value gets its reader default first,
    /// then the property when present; a pair takes the property's first
    /// two floats. *[choice]* A pair property with fewer than two values
    /// (the original reads past its end) keeps the default for the missing
    /// elements. The style list is cleared first, then filled when present.
    /// `years_between` is the simulator's field: when it is 0 it becomes 40,
    /// then the property when present (§2.1); otherwise it is not read.
    pub fn read(&mut self, h: Option<&PropertyHolder>, years_between: &mut i32) {
        *self = Tuning::code_defaults();
        if let Some(h) = h {
            let d = Tuning::code_defaults();
            let pair = |id: u32, def: [f32; 2]| -> [f32; 2] {
                match props::f32s(h, id) {
                    Some(v) => [
                        v.first().copied().unwrap_or(def[0]),
                        v.get(1).copied().unwrap_or(def[1]),
                    ],
                    None => def,
                }
            };
            self.overbuild = props::f32_of(h, prop::OVERBUILD).unwrap_or(d.overbuild);
            self.reoccupy = pair(prop::REOCCUPY_THRESHOLDS, d.reoccupy);
            self.redevelop_size_threshold =
                props::f32_of(h, prop::REDEVELOP_SIZE_THRESHOLD).unwrap_or(d.redevelop_size_threshold);
            self.demolition = pair(prop::DEMOLITION_COSTS, d.demolition);
            self.new_lot = pair(prop::NEW_LOT_COSTS, d.new_lot);
            self.occupancy_variation = props::f32_of(h, prop::OCCUPANCY_VARIATION).unwrap_or(d.occupancy_variation);
            self.min_vacancy = pair(prop::MIN_VACANCY_RANGE, d.min_vacancy);
            self.kick_out = props::bool_of(h, prop::KICK_OUT_LOWER_WEALTH).unwrap_or(d.kick_out);
            if let Some(v) = props::u32s(h, prop::AVAILABLE_STYLES) {
                self.available_styles = v;
            }
        }
        if *years_between == 0 {
            *years_between = 40;
            if let Some(v) = h.and_then(|h| props::i64_of(h, prop::YEARS_BETWEEN_STYLES)) {
                *years_between = v as i32;
            }
        }
    }

    /// The shipped exemplar (§4.21 "Shipped" column), for hosts and tests
    /// without the game's data files.
    pub fn shipped_exemplar() -> PropertyHolder {
        let mut h = PropertyHolder::new();
        let f = |v: f32| Variant::Scalar(Elem::F32(v));
        let f2 = |a: f32, b: f32| Variant::array(mem::FLOAT32, vec![Elem::F32(a), Elem::F32(b)]);
        h.add(prop::OVERBUILD, f(1.5));
        h.add(prop::REOCCUPY_THRESHOLDS, f2(0.0, 1.2));
        h.add(prop::REDEVELOP_SIZE_THRESHOLD, f(1.5));
        h.add(prop::DEMOLITION_COSTS, f2(1.0, 0.2));
        h.add(prop::NEW_LOT_COSTS, f2(1.0, 1.0));
        h.add(prop::OCCUPANCY_VARIATION, f(0.0));
        h.add(prop::MIN_VACANCY_RANGE, f2(0.01, 0.1));
        h.add(prop::KICK_OUT_LOWER_WEALTH, Variant::Scalar(Elem::Bool(true)));
        h.add(
            prop::AVAILABLE_STYLES,
            Variant::array(
                mem::UINT32,
                vec![
                    Elem::U32(0x2000),
                    Elem::U32(0x2001),
                    Elem::U32(0x2002),
                    Elem::U32(0x2003),
                ],
            ),
        );
        h.add(prop::YEARS_BETWEEN_STYLES, Variant::Scalar(Elem::I32(5)));
        h
    }
}
