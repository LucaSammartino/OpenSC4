// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The tuning of §7: the population values read at `Init` and at the first
//! state-4 call of each cycle (§7.1), the simulator values read at `Init`
//! when not initialised (§7.2), and the two per-call curves (§7.3). Property
//! names are those of `docs/specs/data-formats.md`.

use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::props;
use crate::stream::ResKey;
use crate::variant::{mem, Elem, Variant};
use crate::x87::r24;

use super::DepartmentBlock;

/// The one exemplar (§7): type `0x6534284A`, group `0xE7E2C2DB`, instance
/// `0x69189C84`.
pub const TUNING_KEY: ResKey = ResKey {
    type_id: 0x6534_284A,
    group: 0xE7E2_C2DB,
    instance: 0x6918_9C84,
};

/// Tuning property IDs (§7.1–§7.3).
pub mod id {
    // ---- population tuning (§7.1) ----
    pub const EQ_DECAY: u32 = 0xC929_C433;
    pub const HQ_DECAY: u32 = 0x0929_C439;
    pub const LOW_AGE: u32 = 0xC929_C652;
    pub const MEDIUM_AGE: u32 = 0x0929_CC30;
    pub const HIGH_AGE: u32 = 0x0929_CC36;
    pub const AGE_RESET: u32 = 0xCA02_7307;
    pub const EQ_RESET: u32 = 0xCA02_7325;
    pub const HQ_RESET: u32 = 0xCA02_7328;
    pub const LOW_EQ: u32 = 0xC929_CC58;
    pub const MEDIUM_EQ: u32 = 0x0929_CC65;
    pub const HIGH_EQ: u32 = 0x2929_CC6B;
    pub const LOW_HQ: u32 = 0xC929_CC8D;
    pub const MEDIUM_HQ: u32 = 0x6929_CC92;
    pub const HIGH_HQ: u32 = 0x4929_CC97;
    pub const DEATHS_DOOR_HQ: u32 = 0x6AB7_0DA7;
    pub const DEATHS_DOOR_HEIGHT: u32 = 0x6AB7_1654;
    pub const DEATHS_DOOR_DELTA: u32 = 0x4AB7_1659;
    pub const DEATHS_DOOR_CHANCE: u32 = 0x4AB7_0DAC;
    pub const AIR_CURVE: u32 = 0x6941_B022;
    pub const WATER_CURVE: u32 = 0xA941_B028;
    pub const GARBAGE_CURVE: u32 = 0xC941_B02E;

    // ---- simulator tuning (§7.2), health then education ----
    pub const HEALTH_MIN_FUNDING: u32 = 0xE918_946F;
    pub const HEALTH_FUNDING_CURVE: u32 = 0x2918_949E;
    pub const HEALTH_COVERAGE_CURVE: u32 = 0x0A53_B2F9;
    pub const HEALTH_INIT_MAX_FUNDING: u32 = 0xA918_9479;
    pub const HEALTH_STRIKE_EFFICIENCY: u32 = 0x8918_9482;
    pub const HEALTH_STRIKE_DURATION: u32 = 0x8918_948B;
    pub const HEALTH_MIN_GRADE: u32 = 0x8918_9495;
    pub const HEALTH_STRIKE_CURVE: u32 = 0x6918_94A4;
    pub const SCHOOL_MIN_FUNDING: u32 = 0xC918_94A8;
    pub const SCHOOL_FUNDING_CURVE: u32 = 0x6918_94C6;
    pub const SCHOOL_COVERAGE_CURVE: u32 = 0xEA53_B30F;
    pub const SCHOOL_INIT_MAX_FUNDING: u32 = 0x8918_94AD;
    pub const SCHOOL_STRIKE_EFFICIENCY: u32 = 0xE918_94B3;
    pub const SCHOOL_STRIKE_DURATION: u32 = 0xA918_94BC;
    pub const SCHOOL_MIN_GRADE: u32 = 0x6918_94C0;
    pub const SCHOOL_STRIKE_CURVE: u32 = 0xC918_94CB;
    /// `0xEA540621 + g`, g = 0..8: "Avg. Age to 1-10 … 81-90 Age Group".
    pub const AGE_GROUP_CURVES: u32 = 0xEA54_0621;
    pub const SCHOOL_STAFF_DIVISOR: u32 = 0x8A89_1421;
    pub const HOSPITAL_STAFF_DIVISOR: u32 = 0x8A89_142A;
    pub const COVERAGE_SLOP: u32 = 0x6A9F_6EDD;
    pub const MAX_EQ_BOOST: u32 = 0x4AA8_4161;
    pub const MAX_HQ_BOOST: u32 = 0x2AA8_4164;
    pub const SCHOOL_CHUNKS: u32 = 0xEC43_826B;
    pub const HOSPITAL_CHUNKS: u32 = 0xAC43_826D;

    // ---- per-call curves (§7.3) ----
    pub const HQ_TO_LIFE_EXPECTANCY: u32 = 0x6941_A67F;
    pub const LE_TO_WORKFORCE: u32 = 0x4941_A92B;
}

/// L1, the code default of the funding and age-group curves (§7.2).
const L1: &[(f32, f32)] = &[(0.0, 0.0), (100.0, 100.0), (120.0, 110.0)];
/// L2, the code default of the coverage and strike-response curves (§7.2).
const L2: &[(f32, f32)] = &[(0.0, 20.0), (100.0, 100.0)];
/// The pollution curves' code default (§7.1).
const POLLUTION_DEFAULT: &[(f32, f32)] = &[(0.0, 0.0), (32000.0, 100.0)];
/// The life-expectancy curve's code default (§7.3).
const LE_DEFAULT: &[(f32, f32)] = &[(0.0, 0.0), (200.0, 200.0)];
/// The workforce curve's code default (§7.3).
const WORKFORCE_DEFAULT: &[(f32, f32)] = &[(0.0, 0.0), (200.0, 100.0)];

/// The nine age groups of §5.6.
pub const AGE_GROUPS: usize = 9;

/// The class's one `0.01` constant (RS1, foundations §18, residential.md
/// §7.1): the float32 `0.01f` = `0x3C23D70A` = 0.0099999998. The "% × 0.01"
/// / SC4Percentage property readers (§7.1, §7.2), the `× 0.01f` of §5.6 step
/// 4, slots 30/31 (§6.4) and the version < 5 / < 6 record conversions (§9.2)
/// all multiply by it, through [`pct`] only.
///
/// Shipped percentages therefore become 80 → 0.79999995 (`0x3F4CCCCC`, an
/// exact tie rounded to even), 120 → 1.1999999 (`0x3F999999`), 20 →
/// 0.19999999 (`0x3E4CCCCC`), while 25, 50, 90 and 100 give 0.25, 0.5, 0.9f
/// and 1.0. Because each product is rounded to float32 before any `_ftol`,
/// `1200 × 0.01f` is 12.0, not 11.99999973 (§5.6 step 4, §15 test 10).
pub const PERCENT: f32 = 0.01;

/// `x × 0.01f`: the exact product of the two floats (exact in `f64`)
/// rounded once to float32 ([`crate::x87::r24`]).
pub fn pct(x: f32) -> f32 {
    r24(f64::from(x) * f64::from(PERCENT)) as f32
}

/// The population tuning T of §7.1, read at `Init` and at the first state-4
/// call of each cycle.
#[derive(Clone, Debug, PartialEq)]
pub struct PopulationTuning {
    pub eq_decay: f32,
    pub hq_decay: f32,
    /// The newcomers' ages by wealth (§5.5 step 6), low / medium / high.
    pub new_age: [i32; 3],
    pub reset_age: i32,
    pub eq_reset: f32,
    pub hq_reset: f32,
    /// The newcomers' EQ bonus by wealth (REH §4.7 step 4).
    pub new_eq: [f32; 3],
    /// The newcomers' HQ bonus by wealth. *[quirk]* No code default; 0.0
    /// [choice] (residential.md §7.1).
    pub new_hq: [f32; 3],
    pub deaths_door_hq: f32,
    /// The buzzards effect's height and its ±delta (§5.5 step 2).
    pub effect_height: f32,
    pub effect_delta: f32,
    /// The buzzards chance, a percentage compared with a 0..100 draw.
    pub effect_chance: f32,
    /// Air, water, garbage (§5.5 step 1).
    pub pollution_curves: [ResponseCurve; 3],
}

impl Default for PopulationTuning {
    /// The code defaults of §7.1, set unconditionally before the exemplar is
    /// read.
    fn default() -> PopulationTuning {
        PopulationTuning {
            eq_decay: 1.0,
            hq_decay: 1.0,
            new_age: [25, 30, 35],
            reset_age: 40,
            eq_reset: 0.7,
            hq_reset: 0.7,
            new_eq: [4.0, 6.0, 8.0],
            new_hq: [0.0, 0.0, 0.0],
            deaths_door_hq: 0.0,
            effect_height: 0.0,
            effect_delta: 0.0,
            effect_chance: 0.0,
            // The constructed curves are empty (§7.1 "keep"); `read` rebuilds
            // them from the code default only when the exemplar exists.
            pollution_curves: Default::default(),
        }
    }
}

impl PopulationTuning {
    /// §7.1: the code defaults are set unconditionally, the exemplar is read,
    /// and finally the two decays are multiplied by the ordinance products
    /// (float32). `edu_prod` / `health_prod` are Π of the decay multipliers
    /// in the education and health ordinance-effect lists (REH §4.3), 1.0 for
    /// empty lists.
    ///
    /// The three pollution curves follow foundations §4.1's two patterns:
    /// rebuilt from the code default **only when the exemplar exists** and
    /// then loaded by key, so a missing exemplar leaves `previous` ("keep";
    /// empty on the first read, and an empty curve evaluates to 0.0).
    pub fn read(
        exemplar: Option<&PropertyHolder>,
        edu_prod: f32,
        health_prod: f32,
        previous: &[ResponseCurve; 3],
    ) -> PopulationTuning {
        let mut t = PopulationTuning {
            pollution_curves: previous.clone(),
            ..PopulationTuning::default()
        };
        if let Some(h) = exemplar {
            let f = |id: u32, v: &mut f32| {
                if let Some(x) = props::f32_of(h, id) {
                    *v = x;
                }
            };
            let i = |id: u32, v: &mut i32| {
                if let Some(x) = props::i64_of(h, id) {
                    *v = x as i32;
                }
            };
            let p = |id: u32, v: &mut f32| {
                if let Some(x) = props::f32_of(h, id) {
                    *v = pct(x);
                }
            };
            f(id::EQ_DECAY, &mut t.eq_decay);
            f(id::HQ_DECAY, &mut t.hq_decay);
            i(id::LOW_AGE, &mut t.new_age[0]);
            i(id::MEDIUM_AGE, &mut t.new_age[1]);
            i(id::HIGH_AGE, &mut t.new_age[2]);
            i(id::AGE_RESET, &mut t.reset_age);
            p(id::EQ_RESET, &mut t.eq_reset);
            p(id::HQ_RESET, &mut t.hq_reset);
            f(id::LOW_EQ, &mut t.new_eq[0]);
            f(id::MEDIUM_EQ, &mut t.new_eq[1]);
            f(id::HIGH_EQ, &mut t.new_eq[2]);
            f(id::LOW_HQ, &mut t.new_hq[0]);
            f(id::MEDIUM_HQ, &mut t.new_hq[1]);
            f(id::HIGH_HQ, &mut t.new_hq[2]);
            f(id::DEATHS_DOOR_HQ, &mut t.deaths_door_hq);
            f(id::DEATHS_DOOR_HEIGHT, &mut t.effect_height);
            f(id::DEATHS_DOOR_DELTA, &mut t.effect_delta);
            f(id::DEATHS_DOOR_CHANCE, &mut t.effect_chance);
            // "default" pattern with the exemplar present.
            for (curve, prop) in t
                .pollution_curves
                .iter_mut()
                .zip([id::AIR_CURVE, id::WATER_CURVE, id::GARBAGE_CURVE])
            {
                *curve = ResponseCurve::from_points(POLLUTION_DEFAULT);
                if let Some(c) = props::curve(h, prop) {
                    *curve = c;
                }
            }
        }
        t.eq_decay = (f64::from(t.eq_decay) * f64::from(edu_prod)) as f32;
        t.hq_decay = (f64::from(t.hq_decay) * f64::from(health_prod)) as f32;
        t
    }
}

/// The simulator tuning of §7.2. The two department blocks' scalars are read
/// into the caller's blocks (they are persisted separately, §9.1); this holds
/// the curves and the values only the education/health half reads.
#[derive(Clone, Debug, PartialEq)]
pub struct SimulatorTuning {
    /// Health funding → efficiency (`0x2918949E`), L1 default.
    pub health_funding_curve: ResponseCurve,
    /// Health funding → coverage (`0x0A53B2F9`), L2 default.
    pub health_coverage_curve: ResponseCurve,
    /// Health strike response (`0x691894A4`), L2 default.
    pub health_strike_curve: ResponseCurve,
    pub school_funding_curve: ResponseCurve,
    pub school_coverage_curve: ResponseCurve,
    pub school_strike_curve: ResponseCurve,
    /// `0xEA540621 + g`, g = 0..8 (§5.6), L1 default.
    pub age_group_curves: [ResponseCurve; AGE_GROUPS],
    /// Set only when the exemplar exists (§7.2 *[compat]*); the "this branch"
    /// defaults are used otherwise [choice].
    pub school_staff_divisor: u32,
    pub hospital_staff_divisor: u32,
    pub coverage_slop: f32,
    /// Loaded but never read (REH §4.13 *[quirk]*).
    pub max_eq_boost: f32,
    pub max_hq_boost: f32,
    pub school_chunks: Vec<u32>,
    pub hospital_chunks: Vec<u32>,
}

impl Default for SimulatorTuning {
    /// The code defaults of §7.2, set unconditionally before the exemplar is
    /// read.
    fn default() -> SimulatorTuning {
        let l1 = || ResponseCurve::from_points(L1);
        let l2 = || ResponseCurve::from_points(L2);
        SimulatorTuning {
            health_funding_curve: l1(),
            health_coverage_curve: l2(),
            health_strike_curve: l2(),
            school_funding_curve: l1(),
            school_coverage_curve: l2(),
            school_strike_curve: l2(),
            age_group_curves: std::array::from_fn(|_| l1()),
            school_staff_divisor: 30,
            hospital_staff_divisor: 30,
            coverage_slop: 1.1,
            max_eq_boost: 10.0,
            max_hq_boost: 10.0,
            school_chunks: Vec::new(),
            hospital_chunks: Vec::new(),
        }
    }
}

impl SimulatorTuning {
    /// §7.2: the code defaults are set first, unconditionally — including the
    /// two department blocks' scalars, their strike flags and their strike
    /// start days, so strikes do not survive save and load (§9.3 *[quirk]*) —
    /// and the properties are read **only if the exemplar exists**. Curves
    /// use the "default" pattern (rebuilt from the code point list, then
    /// loaded by key).
    ///
    /// The department references (`department1`, `department2`) and the block
    /// `id` are kept: the defaults overwrite only the scalars the table lists.
    pub fn read(
        exemplar: Option<&PropertyHolder>,
        health: &mut DepartmentBlock,
        education: &mut DepartmentBlock,
    ) -> SimulatorTuning {
        let mut t = SimulatorTuning::default();
        // Unconditional: both blocks back to their code defaults, keeping
        // their references and their message-type IDs.
        for block in [&mut *health, &mut *education] {
            let id = block.id;
            let (d1, d2) = (block.department1, block.department2);
            *block = DepartmentBlock {
                id,
                department1: d1,
                department2: d2,
                ..DepartmentBlock::code_defaults(id)
            };
        }
        let Some(h) = exemplar else { return t };
        let curve = |v: &mut ResponseCurve, prop: u32| {
            *v = ResponseCurve::from_points(
                if matches!(
                    prop,
                    id::HEALTH_COVERAGE_CURVE
                        | id::HEALTH_STRIKE_CURVE
                        | id::SCHOOL_COVERAGE_CURVE
                        | id::SCHOOL_STRIKE_CURVE
                ) {
                    L2
                } else {
                    L1
                },
            );
            if let Some(c) = props::curve(h, prop) {
                *v = c;
            }
        };
        let block = |b: &mut DepartmentBlock,
                     min_funding: u32,
                     init_max: u32,
                     efficiency: u32,
                     duration: u32,
                     min_grade: u32| {
            if let Some(x) = props::f32_of(h, min_funding) {
                b.f0 = pct(x);
            }
            if let Some(v) = props::f32s(h, init_max) {
                if v.len() >= 2 {
                    b.init_funding = pct(v[0]);
                    b.max_funding = pct(v[1]);
                }
            }
            if let Some(x) = props::f32_of(h, efficiency) {
                b.strike_efficiency = pct(x);
            }
            if let Some(x) = props::i64_of(h, duration) {
                b.strike_duration = x as i32;
            }
            if let Some(x) = props::f32_of(h, min_grade) {
                b.min_grade = pct(x);
            }
        };
        block(
            health,
            id::HEALTH_MIN_FUNDING,
            id::HEALTH_INIT_MAX_FUNDING,
            id::HEALTH_STRIKE_EFFICIENCY,
            id::HEALTH_STRIKE_DURATION,
            id::HEALTH_MIN_GRADE,
        );
        block(
            education,
            id::SCHOOL_MIN_FUNDING,
            id::SCHOOL_INIT_MAX_FUNDING,
            id::SCHOOL_STRIKE_EFFICIENCY,
            id::SCHOOL_STRIKE_DURATION,
            id::SCHOOL_MIN_GRADE,
        );
        curve(&mut t.health_funding_curve, id::HEALTH_FUNDING_CURVE);
        curve(&mut t.health_coverage_curve, id::HEALTH_COVERAGE_CURVE);
        curve(&mut t.health_strike_curve, id::HEALTH_STRIKE_CURVE);
        curve(&mut t.school_funding_curve, id::SCHOOL_FUNDING_CURVE);
        curve(&mut t.school_coverage_curve, id::SCHOOL_COVERAGE_CURVE);
        curve(&mut t.school_strike_curve, id::SCHOOL_STRIKE_CURVE);
        for (g, c) in t.age_group_curves.iter_mut().enumerate() {
            curve(c, id::AGE_GROUP_CURVES + g as u32);
        }
        // Set only when the exemplar exists (§7.2 *[compat]*).
        if let Some(x) = props::u32_of(h, id::SCHOOL_STAFF_DIVISOR) {
            t.school_staff_divisor = x;
        }
        if let Some(x) = props::u32_of(h, id::HOSPITAL_STAFF_DIVISOR) {
            t.hospital_staff_divisor = x;
        }
        if let Some(x) = props::f32_of(h, id::COVERAGE_SLOP) {
            t.coverage_slop = x;
        }
        if let Some(x) = props::f32_of(h, id::MAX_EQ_BOOST) {
            t.max_eq_boost = x;
        }
        if let Some(x) = props::f32_of(h, id::MAX_HQ_BOOST) {
            t.max_hq_boost = x;
        }
        if let Some(v) = props::u32s(h, id::SCHOOL_CHUNKS) {
            t.school_chunks = v;
        }
        if let Some(v) = props::u32s(h, id::HOSPITAL_CHUNKS) {
            t.hospital_chunks = v;
        }
        t
    }
}

/// §7.3 / §6.4: the life-expectancy curve, a fresh temporary curve per call
/// with the "default" pattern. `None` when the exemplar is missing, which
/// makes slots 29 and 34 return 0.
pub fn le_curve(exemplar: Option<&PropertyHolder>) -> Option<ResponseCurve> {
    let h = exemplar?;
    Some(props::curve(h, id::HQ_TO_LIFE_EXPECTANCY).unwrap_or_else(|| ResponseCurve::from_points(LE_DEFAULT)))
}

/// §7.3 / §6.4: the workforce curve, a fresh temporary curve per call with
/// the "default" pattern. `None` when the exemplar is missing, which makes
/// slots 30 and 31 write the static 0.0 *[inferred]*.
pub fn workforce_curve(exemplar: Option<&PropertyHolder>) -> Option<ResponseCurve> {
    let h = exemplar?;
    Some(props::curve(h, id::LE_TO_WORKFORCE).unwrap_or_else(|| ResponseCurve::from_points(WORKFORCE_DEFAULT)))
}

/// §6.5 `GetAverageNewAgeByWealth`: the code defaults `{25, 30, 25}`
/// (*[quirk]* the third should be 35), overwritten with the three age
/// properties read as `uint8` when the exemplar has them.
pub fn new_ages_by_wealth(exemplar: Option<&PropertyHolder>) -> [u8; 3] {
    let mut ages = [25u8, 30, 25];
    if let Some(h) = exemplar {
        for (prop, slot) in [id::LOW_AGE, id::MEDIUM_AGE, id::HIGH_AGE]
            .into_iter()
            .zip(ages.iter_mut())
        {
            if let Some(x) = props::i64_of(h, prop) {
                *slot = x as u8;
            }
        }
    }
    ages
}

/// The shipped residential exemplar (`data-formats.md`), for hosts and tests
/// without the game's data files.
pub fn shipped_tuning() -> PropertyHolder {
    let mut h = PropertyHolder::new();
    let f = |v: f32| Variant::Scalar(Elem::F32(v));
    let u = |v: u32| Variant::Scalar(Elem::U32(v));
    let curve = |v: &[f32]| Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect());

    // ---- population tuning (§7.1) ----
    h.add(id::EQ_DECAY, f(0.2));
    h.add(id::HQ_DECAY, f(0.2));
    h.add(id::LOW_AGE, u(25));
    h.add(id::MEDIUM_AGE, u(30));
    h.add(id::HIGH_AGE, u(35));
    h.add(id::AGE_RESET, u(40));
    h.add(id::EQ_RESET, f(80.0));
    h.add(id::HQ_RESET, f(80.0));
    h.add(id::LOW_EQ, u(20));
    h.add(id::MEDIUM_EQ, u(40));
    h.add(id::HIGH_EQ, u(60));
    h.add(id::LOW_HQ, u(20));
    h.add(id::MEDIUM_HQ, u(40));
    h.add(id::HIGH_HQ, u(60));
    h.add(id::DEATHS_DOOR_HQ, f(2.0));
    h.add(id::DEATHS_DOOR_HEIGHT, f(50.0));
    h.add(id::DEATHS_DOOR_DELTA, f(10.0));
    h.add(id::DEATHS_DOOR_CHANCE, f(1.5));
    h.add(id::AIR_CURVE, curve(&[0.0, 0.0, 512.0, 0.1, 1024.0, 50.0]));
    h.add(id::WATER_CURVE, curve(&[0.0, 0.0, 512.0, 0.1, 1024.0, 50.0]));
    h.add(id::GARBAGE_CURVE, curve(&[0.0, 0.0, 512.0, 0.2, 1024.0, 20.0]));

    // ---- simulator tuning (§7.2); the two OBS min-funding properties are
    // absent from the shipped exemplar ----
    h.add(id::HEALTH_FUNDING_CURVE, curve(&[0.0, 0.0, 100.0, 100.0, 120.0, 110.0]));
    h.add(id::HEALTH_COVERAGE_CURVE, curve(&[0.0, 50.0, 100.0, 100.0]));
    h.add(id::HEALTH_INIT_MAX_FUNDING, curve(&[25.0, 120.0]));
    h.add(id::HEALTH_STRIKE_EFFICIENCY, f(20.0));
    h.add(id::HEALTH_STRIKE_DURATION, u(500));
    h.add(id::HEALTH_MIN_GRADE, f(90.0));
    h.add(id::HEALTH_STRIKE_CURVE, curve(&[0.0, 100.0, 80.0, 0.0]));
    h.add(id::SCHOOL_FUNDING_CURVE, curve(&[0.0, 0.0, 100.0, 100.0, 120.0, 110.0]));
    h.add(
        id::SCHOOL_COVERAGE_CURVE,
        curve(&[0.0, 50.0, 100.0, 100.0, 120.0, 110.0]),
    );
    h.add(id::SCHOOL_INIT_MAX_FUNDING, curve(&[25.0, 120.0]));
    h.add(id::SCHOOL_STRIKE_EFFICIENCY, f(20.0));
    h.add(id::SCHOOL_STRIKE_DURATION, u(500));
    h.add(id::SCHOOL_MIN_GRADE, f(90.0));
    h.add(id::SCHOOL_STRIKE_CURVE, curve(&[0.0, 100.0, 80.0, 0.0]));
    // The nine age-group curves, in group order.
    let groups: [&[f32]; AGE_GROUPS] = [
        &[20.0, 24.0, 30.0, 6.0, 40.0, 9.0, 50.0, 4.0, 60.0, 0.0],
        &[20.0, 28.0, 30.0, 24.0, 40.0, 12.0, 50.0, 6.0, 60.0, 0.0],
        &[20.0, 31.0, 30.0, 27.0, 40.0, 12.0, 50.0, 6.0, 60.0, 0.0],
        &[20.0, 12.0, 30.0, 20.0, 40.0, 13.0, 50.0, 8.0, 60.0, 3.0],
        &[20.0, 5.0, 30.0, 12.0, 40.0, 23.0, 50.0, 19.0, 60.0, 13.0],
        &[20.0, 0.0, 30.0, 7.0, 40.0, 14.0, 50.0, 27.0, 60.0, 39.0],
        &[20.0, 0.0, 30.0, 4.0, 40.0, 11.0, 50.0, 20.0, 60.0, 31.0],
        &[20.0, 0.0, 30.0, 0.0, 40.0, 5.0, 50.0, 8.0, 60.0, 11.0],
        &[20.0, 0.0, 30.0, 0.0, 40.0, 1.0, 50.0, 2.0, 60.0, 3.0],
    ];
    for (g, points) in groups.iter().enumerate() {
        h.add(id::AGE_GROUP_CURVES + g as u32, curve(points));
    }
    h.add(id::SCHOOL_STAFF_DIVISOR, u(30));
    h.add(id::HOSPITAL_STAFF_DIVISOR, u(30));
    h.add(id::COVERAGE_SLOP, f(1.1));
    h.add(id::MAX_EQ_BOOST, f(1.3));
    h.add(id::MAX_HQ_BOOST, f(1.1));
    h.add(
        id::SCHOOL_CHUNKS,
        Variant::array(
            mem::UINT32,
            [500u32, 750, 1000, 1000, 10000, 300, 1200]
                .into_iter()
                .map(Elem::U32)
                .collect(),
        ),
    );
    h.add(
        id::HOSPITAL_CHUNKS,
        Variant::array(mem::UINT32, [500u32, 500, 2000].into_iter().map(Elem::U32).collect()),
    );

    // ---- per-call curves (§7.3) ----
    h.add(
        id::HQ_TO_LIFE_EXPECTANCY,
        curve(&[0.0, 50.0, 20.0, 55.0, 40.0, 60.0, 60.0, 65.0, 200.0, 100.0]),
    );
    h.add(id::LE_TO_WORKFORCE, curve(&[50.0, 40.0, 100.0, 60.0]));
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> [ResponseCurve; 3] {
        Default::default()
    }

    #[test]
    fn population_defaults_then_shipped() {
        let d = PopulationTuning::read(None, 1.0, 1.0, &empty());
        assert_eq!((d.eq_decay, d.hq_decay), (1.0, 1.0));
        assert_eq!(d.new_age, [25, 30, 35]);
        assert_eq!((d.eq_reset, d.hq_reset), (0.7, 0.7));
        assert_eq!(d.new_eq, [4.0, 6.0, 8.0]);
        // [quirk] no code default for the HQ bonuses: 0.0 [choice].
        assert_eq!(d.new_hq, [0.0, 0.0, 0.0]);
        assert_eq!(d.deaths_door_hq, 0.0);
        // No exemplar on the first read: the pollution curves stay empty
        // ("keep") and evaluate to 0.0.
        assert!(d.pollution_curves.iter().all(|c| c.is_empty()));
        assert_eq!(d.pollution_curves[0].eval(32000.0), 0.0);

        let e = shipped_tuning();
        let t = PopulationTuning::read(Some(&e), 1.0, 1.0, &empty());
        assert_eq!((t.eq_decay, t.hq_decay), (0.2, 0.2));
        assert_eq!(t.reset_age, 40);
        assert_eq!(t.new_eq, [20.0, 40.0, 60.0]);
        assert_eq!(t.new_hq, [20.0, 40.0, 60.0]);
        assert_eq!((t.deaths_door_hq, t.effect_chance), (2.0, 1.5));
        assert_eq!((t.effect_height, t.effect_delta), (50.0, 10.0));
        // §15 test 24 / RS1: "% × 0.01f" — 80 × 0.01f is an exact tie that
        // rounds to the even float 0.79999995 (`0x3F4CCCCC`), not 0.8f.
        assert_eq!(t.eq_reset, pct(80.0));
        assert_eq!(t.eq_reset.to_bits(), 0x3F4C_CCCC);
        assert_eq!(t.hq_reset.to_bits(), 0x3F4C_CCCC);
        // The product is rounded to float32 before any truncation (§5.6
        // step 4): 1200 × 0.01f is 12.0, not 11.99999973.
        assert_eq!(pct(1200.0), 12.0);
        assert_eq!(PERCENT.to_bits(), 0x3C23_D70A);
        for (x, bits) in [
            (25.0, 0x3E80_0000),
            (50.0, 0x3F00_0000),
            (90.0, 0x3F66_6666),
            (100.0, 0x3F80_0000),
        ] {
            assert_eq!(pct(x).to_bits(), bits, "{x}");
        }
        // Absent properties keep the code defaults (0.7 = 0x3F333333).
        assert_eq!(d.eq_reset.to_bits(), 0x3F33_3333);
        // The ordinance products scale the decays in float32.
        let o = PopulationTuning::read(Some(&e), 1.1, 0.5, &empty());
        assert_eq!((o.eq_decay, o.hq_decay), (0.2 * 1.1f32, 0.1));
    }

    /// §7.1's "keep" pattern: a missing exemplar leaves the previous curves,
    /// a present one rebuilds from the code default first.
    #[test]
    fn pollution_curves_keep_without_exemplar() {
        let mut h = PropertyHolder::new();
        h.add(
            id::AIR_CURVE,
            Variant::array(mem::FLOAT32, vec![Elem::F32(0.0), Elem::F32(7.0)]),
        );
        let t = PopulationTuning::read(Some(&h), 1.0, 1.0, &empty());
        assert_eq!(t.pollution_curves[0].eval(100.0), 7.0);
        // The other two were rebuilt from the code default, not kept.
        assert_eq!(t.pollution_curves[1].eval(32000.0), 100.0);
        // Without the exemplar all three stay as they were.
        let k = PopulationTuning::read(None, 1.0, 1.0, &t.pollution_curves);
        assert_eq!(k.pollution_curves, t.pollution_curves);
        assert_eq!(k.pollution_curves[0].eval(100.0), 7.0);
        assert_eq!(k.pollution_curves[2].eval(32000.0), 100.0);
    }

    /// §7.2: the defaults are unconditional, so a loaded department block's
    /// strike flag and start day are cleared (§9.3 *[quirk]*, §15 test 21).
    #[test]
    fn simulator_read_resets_strike_state() {
        let mut health = DepartmentBlock {
            strike_flag: 1,
            strike_start: 1234,
            strike_duration: 500,
            max_funding: 3.0,
            department1: Some(7),
            department2: Some(8),
            ..DepartmentBlock::code_defaults(0x077A_CCD2)
        };
        let mut education = DepartmentBlock::code_defaults(0x077A_CCD0);
        let t = SimulatorTuning::read(None, &mut health, &mut education);
        assert_eq!((health.strike_flag, health.strike_start), (0, 0));
        assert_eq!(health.strike_duration, 10);
        assert_eq!(health.max_funding, 1.2);
        // References and the message-type ID survive the reset.
        assert_eq!((health.department1, health.department2), (Some(7), Some(8)));
        assert_eq!(health.id, 0x077A_CCD2);
        assert_eq!(education.id, 0x077A_CCD0);
        // Without the exemplar the "this branch" defaults stand [choice].
        assert_eq!((t.school_staff_divisor, t.hospital_staff_divisor), (30, 30));
        assert_eq!(t.coverage_slop, 1.1);
        assert!(t.school_chunks.is_empty());

        let e = shipped_tuning();
        let t = SimulatorTuning::read(Some(&e), &mut health, &mut education);
        assert_eq!(health.strike_duration, 500);
        // §15 test 24 / RS1: × 0.01f, one float32 rounding each.
        assert_eq!(health.max_funding.to_bits(), 0x3F99_9999); // 120 → 1.1999999
        assert_eq!(health.strike_efficiency.to_bits(), 0x3E4C_CCCC); // 20 → 0.19999999
        assert_eq!(health.init_funding, 0.25);
        assert_eq!(health.min_grade, 0.9); // 90 → 0.9f
        assert_eq!(
            (education.max_funding, education.strike_efficiency),
            (health.max_funding, health.strike_efficiency)
        );
        assert_eq!(health.f0, 0.6); // OBS property absent from the exemplar
        assert_eq!(t.school_chunks, vec![500, 750, 1000, 1000, 10000, 300, 1200]);
        assert_eq!(t.hospital_chunks, vec![500, 500, 2000]);
        assert_eq!(t.school_coverage_curve.eval(0.0), 50.0);
        assert_eq!(t.health_strike_curve.eval(0.0), 100.0);
        assert_eq!(t.age_group_curves.len(), AGE_GROUPS);
        assert_eq!(t.age_group_curves[0].eval(30.0), 6.0);
        assert_eq!(t.age_group_curves[5].eval(60.0), 39.0);
    }

    /// §15 tests 15, 16, 17: the per-call curves and the wealth ages.
    #[test]
    fn per_call_curves() {
        let e = shipped_tuning();
        assert_eq!(le_curve(None), None);
        assert_eq!(workforce_curve(None), None);
        let le = le_curve(Some(&e)).unwrap();
        assert_eq!(le.eval(100.0), 75.0);
        assert_eq!(le.eval(22.0), 55.5);
        assert_eq!(le.eval(0.0), 50.0);
        let w = workforce_curve(Some(&e)).unwrap();
        assert_eq!(w.eval(75.0), 50.0);
        assert_eq!(w.eval(61.0), 44.4);
        // Without the exemplar the code defaults stand (§7.3 "default").
        let mut h = PropertyHolder::new();
        h.add(id::EQ_DECAY, Variant::Scalar(Elem::F32(0.2)));
        assert_eq!(le_curve(Some(&h)).unwrap().eval(100.0), 100.0);
        assert_eq!(workforce_curve(Some(&h)).unwrap().eval(100.0), 50.0);

        // §6.5: {25, 30, 25} without the exemplar, the shipped ages with it.
        assert_eq!(new_ages_by_wealth(None), [25, 30, 25]);
        assert_eq!(new_ages_by_wealth(Some(&e)), [25, 30, 35]);
    }
}
