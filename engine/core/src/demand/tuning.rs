// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Demand tuning (§11.1), the nation's economy contract (§8.2, §11.2) and the
//! demand-exemplar schema (§4.1).

use std::collections::BTreeMap;

use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::props;
use crate::stream::ResKey;

/// Demand Simulator exemplar `0x6534284A / 0xE7E2C2DB / 0x4A1F3CE6` (§11.1).
pub const TUNING_KEY: ResKey = props::exemplar_key(0xE7E2_C2DB, 0x4A1F_3CE6);
/// Nation exemplar `0x6534284A / 0xE7E2C2DB / 0x4A1F6115` (§11.2).
pub const NATION_KEY: ResKey = props::exemplar_key(0xE7E2_C2DB, 0x4A1F_6115);

mod p {
    pub const NEUTRAL_TAX_CURVE: u32 = 0x4A1F_38B5;
    pub const TAX_MODIFIER_CURVE: u32 = 0x4A1F_38B6;
    pub const TAX_PENALTY_DAYS: u32 = 0x4A1F_38B7;
    pub const RELIEF_BY_NETWORK: u32 = 0x4A1F_38C5;
    pub const RELIEF_AIR_C: u32 = 0x4A1F_38C6;
    /// Labelled "Commercial … Seaports", feeds industrial relief [compat].
    pub const RELIEF_SEA_I: u32 = 0x4A1F_38C7;
    pub const RELIEF_PER_TRIP: u32 = 0x4A1F_38C8;
    pub const RELIEF_AIR_I: u32 = 0x4A1F_38C9;
    /// Labelled "Industrial … Seaports", feeds commercial relief [compat].
    pub const RELIEF_SEA_C: u32 = 0x4A1F_38CA;
    pub const SALARY: u32 = 0x8A63_F2A1;

    pub const NATION_IDS: u32 = 0x4A1F_629F;
    pub const NATION_CURVE_BASE: u32 = 0x4A1F_62A0;
    pub const NATION_DIFFICULTY: u32 = 0x4A1F_6300;

    pub const EX_KEY: u32 = 0x0000_0021;
    pub const EX_MIN: u32 = 0x47BB_3F10;
    pub const EX_MAX: u32 = 0x47BB_3F11;
    pub const EX_INITIAL_SUPPLY: u32 = 0x47BB_3F20;
    pub const EX_INITIAL_DEMAND: u32 = 0x47BB_3F21;
    pub const EX_DRIVES: u32 = 0x47BB_3F30;
    pub const EX_SATISFIES: u32 = 0x47BB_3F31;
    pub const EX_CONTRIBUTES: u32 = 0x47BB_3F32;
    pub const EX_CAPS: u32 = 0x47BB_3F41;
    pub const EX_SUMS: u32 = 0x47BB_3F50;
    pub const EX_REGION_USE: u32 = 0x47BB_3F60;
}

/// Property IDs of the demand exemplars (§4.1), for callers that build them.
pub mod exemplar_prop {
    pub use super::p::{
        EX_CAPS as CAPS, EX_CONTRIBUTES as CONTRIBUTES, EX_DRIVES as DRIVES, EX_INITIAL_DEMAND as INITIAL_DEMAND,
        EX_INITIAL_SUPPLY as INITIAL_SUPPLY, EX_KEY as KEY, EX_MAX as MAX, EX_MIN as MIN, EX_REGION_USE as REGION_USE,
        EX_SATISFIES as SATISFIES, EX_SUMS as SUMS,
    };
}

/// The demand simulator's tuning (§11.1).
#[derive(Clone, Debug, PartialEq)]
pub struct DemandTuning {
    /// Neutral tax rate vs. population (none = absent).
    pub neutral_tax_curve: Option<ResponseCurve>,
    /// Tax modifier vs. rate variance (none = absent).
    pub tax_modifier_curve: Option<ResponseCurve>,
    pub tax_penalty_days: u32,
    /// Indexed by network type (§8.3).
    pub relief_by_network: [f32; 13],
    pub relief_air_c: f32,
    pub relief_sea_i: f32,
    pub relief_per_trip: f32,
    pub relief_air_i: f32,
    pub relief_sea_c: f32,
    pub salary: [u32; 12],
}

impl Default for DemandTuning {
    /// The code defaults of §11.1.
    fn default() -> Self {
        DemandTuning {
            neutral_tax_curve: None,
            tax_modifier_curve: None,
            tax_penalty_days: 90,
            relief_by_network: [0.0; 13],
            relief_air_c: 1.0,
            relief_sea_i: 2.0,
            relief_per_trip: 20.0,
            relief_air_i: 0.1,
            relief_sea_c: 20.0,
            salary: [100; 12],
        }
    }
}

/// Overwrites the leading entries of `dst` with those present.
/// [choice] missing entries keep the default (§11.1).
fn overlay<T: Copy, const N: usize>(dst: &mut [T; N], values: Option<Vec<T>>) {
    if let Some(v) = values {
        for (d, s) in dst.iter_mut().zip(v) {
            *d = s;
        }
    }
}

impl DemandTuning {
    /// Reads the tuning exemplar over the code defaults. An absent curve
    /// property removes the curve.
    pub fn from_holder(h: &PropertyHolder) -> DemandTuning {
        let curve = |id| props::f32s(h, id).and_then(|v| ResponseCurve::from_flat(&v));
        let mut t = DemandTuning {
            neutral_tax_curve: curve(p::NEUTRAL_TAX_CURVE),
            tax_modifier_curve: curve(p::TAX_MODIFIER_CURVE),
            ..DemandTuning::default()
        };
        if let Some(v) = props::u32_of(h, p::TAX_PENALTY_DAYS) {
            t.tax_penalty_days = v;
        }
        overlay(&mut t.relief_by_network, props::f32s(h, p::RELIEF_BY_NETWORK));
        let scalars: [(u32, &mut f32); 5] = [
            (p::RELIEF_AIR_C, &mut t.relief_air_c),
            (p::RELIEF_SEA_I, &mut t.relief_sea_i),
            (p::RELIEF_PER_TRIP, &mut t.relief_per_trip),
            (p::RELIEF_AIR_I, &mut t.relief_air_i),
            (p::RELIEF_SEA_C, &mut t.relief_sea_c),
        ];
        for (id, dst) in scalars {
            if let Some(v) = props::f32_of(h, id) {
                *dst = v;
            }
        }
        overlay(&mut t.salary, props::u32s(h, p::SALARY));
        t
    }

    /// The tuning from the exemplar, or the code defaults when it is missing
    /// ([choice], §11.1).
    pub fn from_exemplar(h: Option<&PropertyHolder>) -> DemandTuning {
        h.map(DemandTuning::from_holder).unwrap_or_default()
    }
}

/// The nation's economy-modifier contract (§8.2, §11.2). The nation belongs
/// to the application; this helper implements the contract for a host that
/// has no other nation object.
#[derive(Clone, Debug, PartialEq)]
pub struct NationEconomy {
    /// Curve per demand ID; ID 0 = the default curve.
    pub curves: BTreeMap<u32, ResponseCurve>,
    /// Indexed by `GetDifficultyLevel()`.
    pub difficulty: [f32; 3],
}

impl Default for NationEconomy {
    /// No curves, no difficulty modifier ([choice]: the spec gives no code
    /// defaults for the nation exemplar).
    fn default() -> Self {
        NationEconomy {
            curves: BTreeMap::new(),
            difficulty: [0.0; 3],
        }
    }
}

impl NationEconomy {
    pub fn from_holder(h: &PropertyHolder) -> NationEconomy {
        let mut n = NationEconomy::default();
        for (i, id) in props::u32s(h, p::NATION_IDS)
            .unwrap_or_default()
            .into_iter()
            .enumerate()
        {
            let prop = p::NATION_CURVE_BASE.wrapping_add(i as u32);
            if let Some(c) = props::f32s(h, prop).and_then(|v| ResponseCurve::from_flat(&v)) {
                n.curves.insert(id, c);
            }
        }
        overlay(&mut n.difficulty, props::f32s(h, p::NATION_DIFFICULTY));
        n
    }

    /// `GetEconomyModifier(demandID)` for `months` since the city start and a
    /// difficulty level, with a city (see [`NationEconomy::modifier_with_city`]).
    pub fn modifier(&self, demand_id: u32, months: i32, difficulty: u32) -> f32 {
        self.modifier_with_city(demand_id, months, Some(difficulty))
    }

    /// `GetEconomyModifier(demandID)` (nation.md §2). The month count is
    /// converted to float as an **unsigned** value, so a negative count
    /// evaluates the curve at `months + 2^32` *[quirk]*; the curve value is
    /// rounded to float; `difficulty[level]` is added only when a city exists
    /// (`Some(level)`). [choice] An unknown difficulty uses the last entry;
    /// no curve at all contributes 0.
    pub fn modifier_with_city(&self, demand_id: u32, months: i32, city_difficulty: Option<u32>) -> f32 {
        let curve = self.curves.get(&demand_id).or_else(|| self.curves.get(&0));
        let c = curve.map_or(0.0, |c| c.eval(months as u32 as f32));
        match city_difficulty {
            Some(d) => c + self.difficulty[(d as usize).min(2)],
            None => c,
        }
    }

    /// Months between two (year, month) dates (§8.2).
    pub fn months_between(start: (i32, i32), now: (i32, i32)) -> i32 {
        (now.0 - start.0).wrapping_mul(12).wrapping_add(now.1 - start.1)
    }
}

/// One demand exemplar (§4.1), decoded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DemandExemplar {
    /// The node key: property `0x21`, else the instance ID.
    pub key: u32,
    pub min: Option<i32>,
    pub max: Option<i32>,
    pub initial_supply: Option<i32>,
    pub initial_demand: Option<i32>,
    /// (target key, percent) pairs.
    pub drives: Vec<(u32, u32)>,
    pub satisfies: Vec<(u32, u32)>,
    pub contributes: Vec<(u32, u32)>,
    pub caps: Vec<u32>,
    pub sums: Vec<u32>,
    /// 0 if absent.
    pub region_use: u32,
}

impl DemandExemplar {
    /// Decodes a demand exemplar whose resource instance is `instance`.
    pub fn from_holder(instance: u32, h: &PropertyHolder) -> DemandExemplar {
        let i32_of = |id| props::i64_of(h, id).map(|v| v as i32);
        DemandExemplar {
            key: props::u32_of(h, p::EX_KEY).unwrap_or(instance),
            min: i32_of(p::EX_MIN),
            max: i32_of(p::EX_MAX),
            initial_supply: i32_of(p::EX_INITIAL_SUPPLY),
            initial_demand: i32_of(p::EX_INITIAL_DEMAND),
            drives: props::u32_pairs(h, p::EX_DRIVES),
            satisfies: props::u32_pairs(h, p::EX_SATISFIES),
            contributes: props::u32_pairs(h, p::EX_CONTRIBUTES),
            caps: props::u32s(h, p::EX_CAPS).unwrap_or_default(),
            sums: props::u32s(h, p::EX_SUMS).unwrap_or_default(),
            region_use: props::u32_of(h, p::EX_REGION_USE).unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variant::{mem, Elem, Variant};

    fn floats(v: &[f32]) -> Variant {
        Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect())
    }

    #[test]
    fn defaults_and_overrides() {
        let t = DemandTuning::from_exemplar(None);
        assert_eq!(t, DemandTuning::default());
        assert_eq!(t.salary, [100; 12]);
        let mut h = PropertyHolder::new();
        h.add(p::TAX_MODIFIER_CURVE, floats(&[0.2, 0.3, 1.0, 0.0, 5.0, -3.5]));
        h.add(p::RELIEF_BY_NETWORK, floats(&[10000.0, 10000.0, 100000.0]));
        h.add(p::RELIEF_SEA_C, Variant::Scalar(Elem::F32(7.0)));
        h.add(p::SALARY, Variant::array(mem::UINT32, vec![Elem::U32(17000)]));
        let t = DemandTuning::from_holder(&h);
        let c = t.tax_modifier_curve.unwrap();
        assert_eq!(c.eval(2.0), -0.875);
        assert_eq!(t.relief_by_network[2], 100000.0);
        assert_eq!(t.relief_by_network[3], 0.0);
        assert_eq!(t.relief_sea_c, 7.0);
        assert_eq!(t.salary[0], 17000);
        assert_eq!(t.salary[1], 100);
        assert!(t.neutral_tax_curve.is_none());
    }

    #[test]
    fn nation_contract() {
        let mut h = PropertyHolder::new();
        h.add(p::NATION_IDS, Variant::array(mem::UINT32, vec![Elem::U32(0)]));
        h.add(p::NATION_CURVE_BASE, floats(&[0.0, 0.0]));
        h.add(p::NATION_DIFFICULTY, floats(&[0.06, 0.03, 0.01]));
        let n = NationEconomy::from_holder(&h);
        assert_eq!(n.modifier(0x1010, 5, 0), 0.06);
        assert_eq!(n.modifier(0x4100, 100, 2), 0.01);
        assert_eq!(NationEconomy::months_between((2000, 11), (2001, 2)), 3);
    }

    /// nation.md §5: the difficulty is added only with a city, and a month
    /// count before the start is read unsigned (far past the last point).
    #[test]
    fn nation_md_conformance() {
        let mut h = PropertyHolder::new();
        h.add(p::NATION_IDS, Variant::array(mem::UINT32, vec![Elem::U32(0)]));
        h.add(p::NATION_DIFFICULTY, floats(&[0.06, 0.03, 0.01]));
        h.add(p::NATION_CURVE_BASE, floats(&[0.0, 0.0, 120.0, 0.0]));
        let n = NationEconomy::from_holder(&h);
        for (d, want) in [(0, 0.06f32), (1, 0.03), (2, 0.01)] {
            assert_eq!(n.modifier_with_city(0x1010, 5, Some(d)), want);
        }
        assert_eq!(n.modifier_with_city(0x1010, 5, None), 0.0);
        let before = NationEconomy::months_between((2000, 2), (2000, 1));
        assert_eq!(before, -1);
        assert_eq!(n.modifier_with_city(0x1010, before, Some(0)), 0.06);
        // A rising curve shows the unsigned read: −1 lands past the last
        // point (value 1), not before the first (value 0).
        let mut h2 = PropertyHolder::new();
        h2.add(p::NATION_IDS, Variant::array(mem::UINT32, vec![Elem::U32(0)]));
        h2.add(p::NATION_CURVE_BASE, floats(&[0.0, 0.0, 120.0, 1.0]));
        let n2 = NationEconomy::from_holder(&h2);
        assert_eq!(n2.modifier_with_city(7, -1, None), 1.0);
        assert_eq!(n2.modifier_with_city(7, 0, None), 0.0);
    }

    #[test]
    fn exemplar_schema() {
        let mut h = PropertyHolder::new();
        h.add(p::EX_MIN, Variant::Scalar(Elem::I32(-6000)));
        h.add(
            p::EX_DRIVES,
            Variant::array(mem::UINT32, vec![Elem::U32(0x2010), Elem::U32(68)]),
        );
        let e = DemandExemplar::from_holder(0x3121, &h);
        assert_eq!(e.key, 0x3121);
        assert_eq!(e.min, Some(-6000));
        assert_eq!(e.drives, [(0x2010, 68)]);
        h.add(p::EX_KEY, Variant::Scalar(Elem::U32(0x99)));
        assert_eq!(DemandExemplar::from_holder(0x3121, &h).key, 0x99);
    }
}
