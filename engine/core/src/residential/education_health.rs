// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The education and health half of `cSC4ResidentialSimulator`
//! (`docs/specs/residential-education-health.md`, "REH"):
//!
//! - service-building recognition, the budget registration and the
//!   health-affecting pending list (REH §4.1, residential.md §5.8);
//! - funding, efficiency and base staff efficiency (§4.2);
//! - the ordinance effect lists, modifiers and decay products (§4.3);
//! - the coverage maps of states 5 / 7 (§4.4) and the grading of states
//!   6 / 8 (§4.5) with its EQ/HQ boost (§4.6) and auto-budget (§4.10);
//! - the monthly strike check of state 0 and the strike chance (§4.9);
//! - the query data, ratings, totals, capacity estimate, coverage geometry
//!   and ideal funding (§4.10, §4.11).
//!
//! The frame these run in — the state machine, the grids, the lists, the
//! department blocks and the save record — is `simulator.rs` / `record.rs`.
//! Everything outside the class (the budget simulator and its departments
//! and line items, the occupants with their properties and lots, the
//! ordinance simulator, the automata controllers) is reached through
//! [`EducationHealthHost`](super::EducationHealthHost). No `RefCell` borrow is
//! held across a host call.
//!
//! Every float step is one float32 rounding in the order REH gives
//! (foundations §18), and every "× 0.01" is [`pct`] (the float32 `0.01f`,
//! RS1). "u32→float" of a value with the top bit set adds 2³² with one
//! rounding ([`u32_to_float`]).
//!
//! # The answered questions (EH1 to EH13, `docs/specs/QUESTIONS.md`)
//!
//! The lane's literal readings 1 to 13 were answered from the exe and REH now
//! states each one. The code follows REH and marks each place `EHn`:
//!
//! 1. EH1 (§4.2): the efficiency ratings and the base staff efficiency call
//!    slot 20 with (a, b) = (0, 0) first, then test the staff item.
//! 2. EH2 (§4.2): slot 19 ends with the dirty flag set in every case but a
//!    null department, including when both percentages are set.
//! 3. EH3 (§4.1): after `ChangeBudgetItemLine`, the call's own copy of the
//!    item has the building type as its line, and every later step uses it.
//! 4. EH4 (§4.1): on removal with no line item the original crashes;
//!    *[choice]* the count step and `RemoveLineItem` are skipped.
//! 5. EH5 (§4.1, §4.5): a building without a capacity property gets a stack
//!    leftover as its effective capacity, and grading uses the radius
//!    property ID as its capacity; *[choice]* 0 in both places.
//! 6. EH6 (§4.5): the tract list and the boost use the lot centre, else
//!    `GetPosition`; "served" takes the building's own tract from
//!    `GetPosition` alone, so the two can differ.
//! 7. EH7 (§4.10, §4.11): slot 28 is the school if the building has the
//!    school capacity, else the hospital; slot 58 tries the school query, then
//!    the hospital query; slot 45's "hospital wins" covers only the radius,
//!    the kind and the ordinance list (the school flag stays set).
//! 8. EH8 (§4.11): slot 45 returns AL = 1 whenever the kind is found; no
//!    radius reads as 0 and no qualifying point gives dist 0. *[choice]* A
//!    distance property with fewer than two values (no points; the original
//!    leaves dist unwritten) also gives 0.
//! 9. EH9 (§4.10): slot 58 tests the served **property** itself.
//! 10. EH10 (§4.11): both queries read capacity, effective capacity, served
//!     and grade, in that order; the hospital query then requires the staff
//!     item. A failed query leaves the fields it read (served raw).
//! 11. EH11 (§4.11): the ratings' record has stack leftovers; *[choice]* it
//!     starts at 0.
//! 12. EH12 (§4.11): the totals reset the caller's +0x14..+0x28 first, then
//!     count and add **every** building that has one of the groups, whatever
//!     its query returned. AQ1: their own record's +0x14..+0x24 are stack
//!     leftovers in the original until a query writes them; *[choice]* they
//!     start at 0.
//! 13. EH13 (§4.6): with n = 0, eff and pop are used unscaled; population,
//!     coverage and target are read at the same (x, z), the age grid at cell
//!     (x × S_t, z × S_t). *[choice]* A tract outside a smaller source grid
//!     reads as 0.
//!
//! # Choices (`[choice]`) where the original crashes or hangs
//!
//! - A chunk size of 0, or a group with no chunk entry (the chunk lists are
//!   empty without the exemplar), gives units 0 instead of a divide fault.
//! - A staff divisor of 0 gives 0 instead of a divide fault.
//! - `EstimateIdealFunding` stops stepping when `f ± 0.01` no longer changes
//!   `f` (a hostile curve or limit would loop forever in the original).
//! - Null list entries never reach these passes: `Read` drops them (RS12).

use std::rc::Rc;

use crate::curve::ResponseCurve;
use crate::grid::{TractGeometry, TractGrid};
use crate::property::PropertyHolder;
use crate::props;
use crate::variant::{mem, Elem, Variant};
use crate::x87::{ftol32, rhe, u32_to_float};

use super::simulator::{min_cap, mul_i, mul_u32, ResidentialSim};
use super::tuning::{pct, PERCENT};
use super::{group, QueryData, MAX_QUOTIENT, PROP_AUTO_BUDGET};

/// The building's strike start day, `0` when not striking (u32, REH §2).
pub const PROP_STRIKE_DAY: u32 = 0xEAA6_9F83;
/// The served population of a building not graded yet (REH §2, §9).
pub const NOT_GRADED: u32 = 0xFFFF_FFFF;
/// The display flag a created line item gets (REH §4.1).
pub const LINE_ITEM_DISPLAY_FLAG: u32 = 2;
/// The two health-affecting properties (residential.md §5.8), both > 0.
pub const PROP_HEALTH_AFFECTING: [u32; 2] = [0x6A19_F6B5, 0xAA19_F6EA];
/// The automata controller groups of a strike (REH §4.9).
pub const STRIKING_EDUCATION: &str = "striking_education";
pub const STRIKING_HEALTH: &str = "striking_health";
/// The code default of every building and ordinance curve (REH §4.3, §4.5).
pub const FLAT_CURVE: [(f32, f32); 2] = [(0.0, 100.0), (100.0, 100.0)];

/// The curve (0,100)(100,100).
pub fn flat_curve() -> ResponseCurve {
    ResponseCurve::from_points(&FLAT_CURVE)
}

/// The property and purpose IDs of one service (REH §2, §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Service {
    pub school: bool,
    /// Budget purposes: staff, and bus / ambulance (§4.2).
    pub staff_purpose: u32,
    pub vehicle_purpose: u32,
    /// The occupant groups that make a building without a staff item a
    /// service building (§4.1).
    pub recognition_groups: [u32; 2],
    /// Building properties (§4.13).
    pub capacity: u32,
    pub radius: u32,
    pub effect_multiplier: u32,
    pub age_curve: u32,
    pub wealth_curve: u32,
    pub distance_curve: u32,
    pub ordinance_kind: u32,
    /// Per-building results (§2).
    pub served: u32,
    pub effective_capacity: u32,
    pub grade: u32,
    pub staff_ratio: u32,
    /// The "can strike" group (§4.9).
    pub strike_group: u32,
    /// The group lists' IDs in coverage-grid order (§4.1).
    pub groups: &'static [u32],
    /// The ordinance effect properties (§4.3).
    pub ord_radius: u32,
    pub ord_distance_curve: u32,
    pub ord_effect: u32,
    pub ord_capacity: u32,
    pub ord_age_curve: u32,
    pub ord_decay: u32,
}

pub const SCHOOL: Service = Service {
    school: true,
    staff_purpose: 0xEA56_54B6,
    vehicle_purpose: 0x4A56_54BA,
    recognition_groups: [0x1503, 0x151B],
    capacity: 0x8922_041B,
    radius: 0x691B_42B3,
    effect_multiplier: 0xA92A_E446,
    age_curve: 0xC929_9662,
    wealth_curve: 0xC92D_7EC0,
    distance_curve: 0xE921_A936,
    ordinance_kind: 0xA92D_8E40,
    served: 0x0922_0161,
    effective_capacity: 0xE924_4B2E,
    grade: 0x2924_53C5,
    staff_ratio: 0x2A97_6C20,
    strike_group: 0x1A03,
    groups: &group::SCHOOL,
    ord_radius: 0xA91B_3AF4,
    ord_distance_curve: 0xA91B_3AFA,
    ord_effect: 0xA92D_9D7A,
    ord_capacity: 0x892D_9D02,
    ord_age_curve: 0xC91B_3B02,
    ord_decay: 0x692E_F65A,
};

pub const HOSPITAL: Service = Service {
    school: false,
    staff_purpose: 0xCA56_5486,
    vehicle_purpose: 0xEA56_549E,
    recognition_groups: [0x1507, 0x151C],
    capacity: 0x6922_0415,
    radius: 0x091B_42A2,
    effect_multiplier: 0xC929_9659,
    age_curve: 0xA92A_E3BD,
    wealth_curve: 0xA92D_9FCD,
    distance_curve: 0x2921_A9CC,
    ordinance_kind: 0x292D_8F9B,
    served: 0xC922_015C,
    effective_capacity: 0x4924_4B21,
    grade: 0x0924_53CB,
    staff_ratio: 0xAA97_6BE7,
    strike_group: 0x1A02,
    groups: &group::HOSPITAL,
    ord_radius: 0x491B_3AD5,
    ord_distance_curve: 0x891B_3AE6,
    ord_effect: 0xE91B_3AEE,
    ord_capacity: 0x092D_909B,
    ord_age_curve: 0xE92D_9DB4,
    ord_decay: 0xC92D_9C7A,
};

impl Service {
    pub fn of(school: bool) -> &'static Service {
        if school {
            &SCHOOL
        } else {
            &HOSPITAL
        }
    }
}

/// `GetBudgetItemForPurpose`'s item (budget.md §2: `{departmentID, lineItem,
/// purpose, cost, index, localFundingPercentage}`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BudgetItem {
    pub department: u32,
    /// 0 = "use the building type" (budget.md §12).
    pub line: u32,
    pub index: u32,
    pub cost: i64,
    /// A fraction (1.0 = 100 %).
    pub local_funding: f32,
}

/// One ordinance effect element (REH §4.3).
#[derive(Clone, Debug, PartialEq)]
pub struct OrdinanceEffect {
    /// Radius factor (× 0.01), default 1.0: m1.
    pub radius: f32,
    pub distance_curve: ResponseCurve,
    /// Effect factor, default 1.0: m3.
    pub effect: f32,
    /// Capacity factor, default 1.0: m2.
    pub capacity: f32,
    pub age_curve: ResponseCurve,
    /// Decay factor, default 1.0 (§4.7 via §7.1).
    pub decay: f32,
    /// The group filter: **always empty** (§4.3), so the element affects every
    /// building of its service.
    pub groups: Vec<u32>,
}

impl OrdinanceEffect {
    /// One service's element of an ordinance's misc properties, or `None`
    /// when none of the six properties is present (§4.3). Both curves are
    /// rebuilt from (0,100)(100,100) and then loaded when present.
    pub fn read(h: &PropertyHolder, svc: &Service) -> Option<OrdinanceEffect> {
        let factor = |id: u32| props::f32_of(h, id).map(pct);
        let radius = factor(svc.ord_radius);
        let distance_curve = props::curve(h, svc.ord_distance_curve);
        let effect = factor(svc.ord_effect);
        let capacity = factor(svc.ord_capacity);
        let age_curve = props::curve(h, svc.ord_age_curve);
        let decay = factor(svc.ord_decay);
        let present = radius.is_some()
            || distance_curve.is_some()
            || effect.is_some()
            || capacity.is_some()
            || age_curve.is_some()
            || decay.is_some();
        present.then(|| OrdinanceEffect {
            radius: radius.unwrap_or(1.0),
            distance_curve: distance_curve.unwrap_or_else(flat_curve),
            effect: effect.unwrap_or(1.0),
            capacity: capacity.unwrap_or(1.0),
            age_curve: age_curve.unwrap_or_else(flat_curve),
            decay: decay.unwrap_or(1.0),
            groups: Vec::new(),
        })
    }
}

/// `GetOrdinanceModifiers(list)` (§4.3): `(m1, m2, m3)` = Π radius, Π
/// capacity, Π effect factor, each from 1.0 in list order (float32).
pub fn ordinance_modifiers(list: &[OrdinanceEffect]) -> (f32, f32, f32) {
    let mut m = (1.0f32, 1.0f32, 1.0f32);
    for e in list {
        m.0 *= e.radius;
        m.1 *= e.capacity;
        m.2 *= e.effect;
    }
    m
}

/// Π decay factor from 1.0 in list order (§4.3).
pub fn decay_product(list: &[OrdinanceEffect]) -> f32 {
    list.iter().fold(1.0f32, |p, e| p * e.decay)
}

/// The first numeric element of a property value, as the property getters
/// read it (strings and empty values are absent).
fn first_elem(v: &Variant) -> Option<Elem> {
    match v {
        Variant::Scalar(e) => Some(*e),
        Variant::Array { elem, items } if *elem != mem::RZ_CHAR && *elem != 0 => items.first().copied(),
        _ => None,
    }
}

fn elem_f32(e: Elem) -> f32 {
    match e {
        Elem::F32(x) => x,
        other => other.as_f64() as f32,
    }
}

/// A curve property the loader turns into a curve with **no** points
/// (foundations §4.1): a scalar or an array of fewer than two numbers.
fn too_few_values(v: &Variant) -> bool {
    match v {
        Variant::Scalar(_) => true,
        Variant::Array { elem, items } if *elem != mem::RZ_CHAR && *elem != 0 => items.len() < 2,
        _ => false,
    }
}

/// `float(a) / float(b)` of two u32 values, one rounding.
fn ratio_u32(a: u32, b: u32) -> f32 {
    (u32_to_float(a as i32) / u32_to_float(b as i32)) as f32
}

/// `float(units) / float(n)` (§4.5, §4.6).
fn per_unit(units: u32, n: u8) -> f32 {
    (u32_to_float(units as i32) / f64::from(n)) as f32
}

/// `capacity / chunk` (u32); [choice] 0 for a chunk of 0.
fn units_of(capacity: u32, chunk: u32) -> u32 {
    capacity.checked_div(chunk).unwrap_or(0)
}

/// `ceil(a / d)` (u32); [choice] 0 for a divisor of 0.
fn ceil_div(a: u32, d: u32) -> u32 {
    match d {
        0 => 0,
        d => a / d + u32::from(a % d != 0),
    }
}

/// `trunc(float(capacity) × f × m)` as the u32 `_ftol` result (§4.5).
fn scaled_capacity(capacity: u32, f: f32, m: f32) -> u32 {
    ftol32(f64::from(mul_u32(capacity as i32, f) * m)) as u32
}

/// The tract range of `GetTractsInRadius` / `AddRadiusEffectWithCurves`
/// (§4.4, §4.6): `trunc((p ∓ r) × inv)`, the lower bound raised to 0 and the
/// upper lowered to `count − 1` with an unsigned or a signed compare.
fn tract_range(p: f32, r: f32, inv: f32, count: i32, unsigned_upper: bool) -> (i32, i32) {
    let mut lo = ftol32(f64::from((p - r) * inv));
    let mut hi = ftol32(f64::from((p + r) * inv));
    if lo < 0 {
        lo = 0;
    }
    let over = if unsigned_upper {
        (hi as u32) >= (count as u32)
    } else {
        hi >= count
    };
    if over {
        hi = count.wrapping_sub(1);
    }
    (lo, hi)
}

/// `GetTractsInRadius(px, pz, r, w, countX, countZ)` (REH §4.4, `0x005E5200`):
/// the tracts whose centre lies within r of (px, pz), x-major. The centres
/// start at `x0 × w + w × 0.5` and advance by `+= w` in float32; a tract is
/// in iff `dz² + dx² ≤ r²` (dx² stored first). The upper bounds use an
/// unsigned compare, so a negative one becomes `count − 1`.
pub fn tracts_in_radius(px: f32, pz: f32, r: f32, w: f32, count_x: i32, count_z: i32) -> Vec<(i32, i32)> {
    let inv = 1.0 / w;
    let (x0, x1) = tract_range(px, r, inv, count_x, true);
    let (z0, z1) = tract_range(pz, r, inv, count_z, true);
    let half = w * 0.5;
    let r2 = r * r;
    let mut out = Vec::new();
    let mut cx = mul_i(i64::from(x0), w) + half;
    for x in x0..=x1 {
        let mut cz = mul_i(i64::from(z0), w) + half;
        for z in z0..=z1 {
            let dx = cx - px;
            let dz = cz - pz;
            let dx2 = dx * dx;
            if dz * dz + dx2 <= r2 {
                out.push((x, z));
            }
            cz += w;
        }
        cx += w;
    }
    out
}

/// The arguments of `AddRadiusEffectWithCurves` (REH §4.6, `0x005E48F0`).
pub struct RadiusEffect<'a> {
    /// The average age grid; `None` skips the age curves.
    pub age: Option<&'a dyn TractGrid<u8>>,
    /// The float grid that receives the effect (an EQ or HQ grid).
    pub target: &'a dyn TractGrid<f32>,
    /// listB: evaluated at the tract's average age.
    pub age_curves: &'a [&'a ResponseCurve],
    pub x: f32,
    pub z: f32,
    pub radius: f32,
    /// listA: evaluated at the 0..1 distance fraction.
    pub distance_curves: &'a [&'a ResponseCurve],
    pub value: f32,
    pub max: f32,
    pub units: u32,
    pub coverage: Option<&'a dyn TractGrid<u8>>,
    pub population: Option<&'a dyn TractGrid<u16>>,
}

/// `AddRadiusEffectWithCurves` (REH §4.6): adds `value` shaped by the curves
/// to every populated tract within the radius of the target grid, capped at
/// `max`, and accumulates `total`.
///
/// - The bounds are those of [`tracts_in_radius`] with a **signed** upper
///   compare.
/// - *[quirk]* A tract with no population is skipped **without** advancing
///   the z centre, so every later tract of that column uses a centre one
///   tract too far back.
/// - `xd = 1 − (r − d) / r` is a 0..1 fraction *[compat]*.
pub fn add_radius_effect_with_curves(e: &RadiusEffect<'_>, total: &mut f32) {
    let g = e.target.geometry();
    let w = g.width_x;
    let inv = 1.0 / w;
    let (x0, x1) = tract_range(e.x, e.radius, inv, g.count_x, false);
    let (z0, z1) = tract_range(e.z, e.radius, inv, g.count_z, false);
    let half = w * 0.5;
    let r = e.radius;
    let s_t = g.tract_size;
    let mut cx = mul_i(i64::from(x0), w) + half;
    for x in x0..=x1 {
        let mut cz = mul_i(i64::from(z0), w) + half;
        for z in z0..=z1 {
            let p = e.population.map_or(0, |g| g.get(x, z));
            if p == 0 {
                // [quirk] the z centre is not advanced on this path.
                continue;
            }
            let dx = cx - e.x;
            let dz = cz - e.z;
            let d = (dz * dz + dx * dx).sqrt();
            if d <= r {
                let mut mult = 1.0f32;
                let xd = 1.0 - (r - d) / r;
                for c in e.distance_curves {
                    mult = pct(mult * c.eval(xd));
                }
                if let Some(age) = e.age {
                    let a = f32::from(age.get_cell_value(x.wrapping_mul(s_t), z.wrapping_mul(s_t)));
                    for c in e.age_curves {
                        mult = pct(mult * c.eval(a));
                    }
                }
                let mut eff = mult * e.value;
                let n = e.coverage.map_or(0, |c| c.get(x, z));
                // EH13: n = 0 leaves eff and pop' = pop unscaled.
                let mut pop2 = u32::from(p);
                if n != 0 {
                    let k = per_unit(e.units, n);
                    eff *= k;
                    pop2 = ftol32(f64::from(mul_i(i64::from(p), k))) as u32;
                }
                *total += mul_i(i64::from(pop2 & 0xFFFF), eff);
                e.target.set(x, z, min_cap(e.max, e.target.get(x, z) + eff));
            }
            cz += w;
        }
        cx += w;
    }
}

/// What one service's coverage and grading states read from the state.
struct Frame {
    svc: &'static Service,
    lists: Vec<Vec<Option<u64>>>,
    coverage: Vec<Option<Rc<dyn TractGrid<u8>>>>,
    chunks: Vec<u32>,
    grade_grid: Option<Rc<dyn TractGrid<f32>>>,
    quotients: [Option<Rc<dyn TractGrid<f32>>>; 3],
    population: [Option<Rc<dyn TractGrid<u16>>>; 3],
    age: Option<Rc<dyn TractGrid<u8>>>,
    slop: f32,
    auto_budget: bool,
}

impl Frame {
    /// The chunk size of group i ([choice] 0 when the list is short).
    fn chunk(&self, i: usize) -> u32 {
        self.chunks.get(i).copied().unwrap_or(0)
    }
}

/// One building's grading inputs shared by `GetPopulationServed` and the
/// boost (REH §4.5, §4.6).
struct Grading<'a> {
    occ: u64,
    /// r' = (e2 × m1) × radius.
    r: f32,
    units: u32,
    c1: &'a ResponseCurve,
    c2: &'a ResponseCurve,
    c3: &'a ResponseCurve,
    effects: &'a [OrdinanceEffect],
    coverage: Option<&'a dyn TractGrid<u8>>,
}

impl ResidentialSim {
    // ---- occupant properties ----

    fn has_prop(&self, occ: u64, id: u32) -> bool {
        self.host.occupant_property(occ, id).is_some()
    }

    fn prop_u32(&self, occ: u64, id: u32) -> Option<u32> {
        self.host
            .occupant_property(occ, id)
            .as_ref()
            .and_then(first_elem)
            .map(|e| e.as_i64() as u32)
    }

    fn prop_u32s(&self, occ: u64, id: u32) -> Vec<u32> {
        self.host
            .occupant_property(occ, id)
            .map(|v| match v {
                Variant::Array { elem, .. } if elem == mem::RZ_CHAR || elem == 0 => Vec::new(),
                v => v.items().iter().map(|e| e.as_i64() as u32).collect(),
            })
            .unwrap_or_default()
    }

    fn prop_f32(&self, occ: u64, id: u32) -> Option<f32> {
        self.host
            .occupant_property(occ, id)
            .as_ref()
            .and_then(first_elem)
            .map(elem_f32)
    }

    /// A building curve: rebuilt from (0,100)(100,100), then loaded when the
    /// property is present ("default" pattern, §4.5).
    fn prop_curve(&self, occ: u64, id: u32) -> ResponseCurve {
        self.host
            .occupant_property(occ, id)
            .as_ref()
            .and_then(props::curve_of_variant)
            .unwrap_or_else(flat_curve)
    }

    fn strike_day(&self, occ: u64) -> u32 {
        self.prop_u32(occ, PROP_STRIKE_DAY).unwrap_or(0)
    }

    /// The lot's centre (`GetOccupantLot` → `GetCenter`), else the occupant's
    /// `GetPosition`: the tract list (§4.4) and the boost centre (§4.6), EH6.
    /// "Served" (§4.5) uses `GetPosition` alone.
    fn centre(&self, occ: u64) -> (f32, f32) {
        match self.host.occupant_lot_center(occ) {
            Some(c) => c,
            None => self.host.occupant_position(occ),
        }
    }

    /// The service of a building by its capacity property, as slot 45 picks
    /// it (§4.11): school if it has `0x8922041B`, then hospital if it has
    /// `0x69220415` (the hospital wins when both).
    pub fn occupant_service(&self, occ: u64) -> Option<&'static Service> {
        self.coverage_kind(occ).1
    }

    /// Slot 45's two tests (§4.11, EH7): the school test sets the school flag,
    /// and the hospital test, run after it, picks the hospital's radius, kind
    /// and ordinance list **without** clearing the flag *[quirk]*. Returns
    /// `(school flag, service)`.
    fn coverage_kind(&self, occ: u64) -> (bool, Option<&'static Service>) {
        let school = self.has_prop(occ, SCHOOL.capacity);
        let hospital = self.has_prop(occ, HOSPITAL.capacity);
        let svc = if hospital {
            Some(&HOSPITAL)
        } else if school {
            Some(&SCHOOL)
        } else {
            None
        };
        (school, svc)
    }

    // ---- ordinance effects (§4.3) ----

    /// `GetOrdinanceEffects(healthList, schoolList)`: one element per service
    /// for each ordinance that is on, in ascending ID order, appended to the
    /// lists. State 0 clears them first (`clear`); `Init` does not. RG7
    /// (answered): the original reads them only in `Init` and in state 0;
    /// between two state 0s the lists stay as read.
    pub(super) fn read_ordinance_effects(&self, clear: bool) {
        let h = &*self.host;
        let mut school = Vec::new();
        let mut health = Vec::new();
        for id in h.ordinances_on() {
            let Some(p) = h.ordinance_properties(id) else {
                continue;
            };
            if let Some(e) = OrdinanceEffect::read(&p, &HOSPITAL) {
                health.push(e);
            }
            if let Some(e) = OrdinanceEffect::read(&p, &SCHOOL) {
                school.push(e);
            }
        }
        let mut s = self.s.borrow_mut();
        if clear {
            s.school_effects.clear();
            s.health_effects.clear();
        }
        s.school_effects.extend(school);
        s.health_effects.extend(health);
    }

    /// `(eduProd, healthProd)`: the decay products of the two lists (§4.3),
    /// which scale the EQ and HQ decays when the population tuning is read.
    pub fn ordinance_decay_products(&self) -> (f32, f32) {
        let s = self.s.borrow();
        (decay_product(&s.school_effects), decay_product(&s.health_effects))
    }

    /// `GetOrdinanceEffectListForOccupant(occ, kindProperty, list)`: the
    /// elements whose group filter is empty or meets the building's u32-array
    /// property `kind` (§4.3). The filter is never filled, so this is the
    /// whole list *[compat]*.
    fn ordinance_effects_for(&self, occ: u64, svc: &Service, kind: u32) -> Vec<OrdinanceEffect> {
        let list = {
            let s = self.s.borrow();
            if svc.school {
                s.school_effects.clone()
            } else {
                s.health_effects.clone()
            }
        };
        if list.iter().all(|e| e.groups.is_empty()) {
            return list;
        }
        let kinds = self.prop_u32s(occ, kind);
        list.into_iter()
            .filter(|e| e.groups.is_empty() || e.groups.iter().any(|g| kinds.contains(g)))
            .collect()
    }

    // ---- funding and efficiency (§4.2) ----

    /// Slot 20 `GetOccupantFundingPercentages(occ, &a, &b, isSchool)`: no staff
    /// item → 1.0 / 1.0; a null staff department leaves both unchanged; b is
    /// 1.0 without a vehicle item and unchanged with a null vehicle department.
    /// RG10 (answered; the bracket "1.0" differed): 1.0 only for a missing
    /// item, never for a missing department. A department whose funding the
    /// host cannot read (`None`) is treated as missing too: that output (and,
    /// for the staff department, b) keeps the caller's value. [choice] The
    /// original always reads a present department's funding; `None` exists
    /// only for a host that cannot.
    pub fn get_occupant_funding(&self, occ: u64, a: &mut f32, b: &mut f32, is_school: bool) {
        let h = &*self.host;
        let svc = Service::of(is_school);
        let Some(item) = h.budget_item(occ, svc.staff_purpose) else {
            *a = 1.0;
            *b = 1.0;
            return;
        };
        let Some(dept) = h.department_budget(item.department) else {
            return;
        };
        *a = if h.is_locally_funded(dept, occ, svc.staff_purpose) {
            item.local_funding
        } else {
            match h.department_funding(dept) {
                Some(f) => f,
                None => return,
            }
        };
        match h.budget_item(occ, svc.vehicle_purpose) {
            None => *b = 1.0,
            Some(v) => {
                if let Some(vd) = h.department_budget(v.department) {
                    if h.is_locally_funded(vd, occ, svc.vehicle_purpose) {
                        *b = v.local_funding;
                    } else if let Some(f) = h.department_funding(vd) {
                        *b = f;
                    }
                }
            }
        }
    }

    /// Slot 20 with the caller's values starting at 1.0 (a convenience).
    pub fn occupant_funding(&self, occ: u64, is_school: bool) -> (f32, f32) {
        let (mut a, mut b) = (1.0, 1.0);
        self.get_occupant_funding(occ, &mut a, &mut b, is_school);
        (a, b)
    }

    /// Slot 19 `SetOccupantFundingPercentages(occ, staff, vehicle, isSchool)`:
    /// `SetLocalFundingPercent` for the staff, then the vehicle purpose. A
    /// missing item ends the call with the dirty flag set, a null department
    /// without it, and setting both also sets it (EH2). A replacement returns
    /// true.
    pub fn set_occupant_funding(&self, occ: u64, staff: f32, vehicle: f32, is_school: bool) -> bool {
        let h = &*self.host;
        let svc = Service::of(is_school);
        for (purpose, x) in [(svc.staff_purpose, staff), (svc.vehicle_purpose, vehicle)] {
            let Some(item) = h.budget_item(occ, purpose) else {
                self.s.borrow_mut().dirty = true;
                return true;
            };
            let Some(dept) = h.department_budget(item.department) else {
                return true;
            };
            h.set_local_funding(dept, occ, x, purpose);
        }
        self.s.borrow_mut().dirty = true;
        true
    }

    /// `(funding → efficiency curve, funding → coverage curve, strike
    /// efficiency)` of a service, evaluated without holding a borrow across a
    /// host call.
    fn with_service_tuning<R>(&self, school: bool, f: impl FnOnce(&ResponseCurve, &ResponseCurve, f32) -> R) -> R {
        let s = self.s.borrow();
        let t = &s.simulator_tuning;
        if school {
            f(
                &t.school_funding_curve,
                &t.school_coverage_curve,
                s.education.strike_efficiency,
            )
        } else {
            f(
                &t.health_funding_curve,
                &t.health_coverage_curve,
                s.health.strike_efficiency,
            )
        }
    }

    /// `GetOccupantEfficiencyRatings(occ, &e1, &e2, isSchool, ignoreStrike)`
    /// (§4.2): `(1, 1)` without a staff item; otherwise e1 = a × strike
    /// efficiency for a striking building (unless ignored), else
    /// `effCurve(a × 100) × 0.01`, and e2 = `coverageCurve(b × 100) × 0.01`.
    pub fn efficiency_ratings(&self, occ: u64, is_school: bool, ignore_strike: bool) -> (f32, f32) {
        let svc = Service::of(is_school);
        // EH1: (a, b) start at (0, 0), and slot 20 is always called, before
        // the staff-item test.
        let (mut a, mut b) = (0.0f32, 0.0f32);
        self.get_occupant_funding(occ, &mut a, &mut b, is_school);
        if self.host.budget_item(occ, svc.staff_purpose).is_none() {
            return (1.0, 1.0);
        }
        let striking = !ignore_strike && self.strike_day(occ) != 0;
        self.with_service_tuning(is_school, |eff, cov, strike_eff| {
            let e1 = if striking {
                a * strike_eff
            } else {
                pct(eff.eval(a * 100.0))
            };
            (e1, pct(cov.eval(b * 100.0)))
        })
    }

    /// `GetBaseStaffEfficiency(occ, isSchool)` (§4.2): `effCurve(a × 100) ×
    /// 0.01` with a staff item, else a (1.0, set by slot 20); strikes are
    /// ignored.
    pub fn base_staff_efficiency(&self, occ: u64, is_school: bool) -> f32 {
        let svc = Service::of(is_school);
        // EH1: slot 20 first, from (0, 0).
        let (mut a, mut b) = (0.0f32, 0.0f32);
        self.get_occupant_funding(occ, &mut a, &mut b, is_school);
        if self.host.budget_item(occ, svc.staff_purpose).is_none() {
            return a;
        }
        self.with_service_tuning(is_school, |eff, _, _| pct(eff.eval(a * 100.0)))
    }

    // ---- buildings (§4.1, residential.md §5.8) ----

    /// `DoBuildingChanged(b, added)` (REH §4.1): the hospital test, then the
    /// school test (a null department aborts the whole call), then the
    /// health-affecting test (residential.md §5.8), then the slot-11 dirty
    /// test.
    pub fn do_building_changed(&self, b: u64, added: bool) {
        if !self.service_building_changed(b, added, &HOSPITAL) {
            return;
        }
        if !self.service_building_changed(b, added, &SCHOOL) {
            return;
        }
        self.health_affecting_changed(b, added);
        if self.host.building_slot11(b) == 1 {
            self.s.borrow_mut().dirty = true;
        }
    }

    /// One service's part of `DoBuildingChanged`; false when a null department
    /// aborts the call.
    fn service_building_changed(&self, b: u64, added: bool, svc: &Service) -> bool {
        let h = &*self.host;
        let item = h.budget_item(b, svc.staff_purpose);
        let is_service = item.is_some() || svc.recognition_groups.iter().any(|&g| h.is_occupant_group(b, g));
        if !is_service {
            return true;
        }
        if let Some(item) = item {
            let mut line = item.line;
            if line == 0 {
                // EH3: the call's copy of the item has the building type as
                // its line from here on.
                line = h.building_type(b);
                h.change_budget_item_line(b, item.index, line);
            }
            let Some(dept) = h.department_budget(item.department) else {
                return false;
            };
            let line_item = if added {
                let li = match h.line_item(dept, line) {
                    Some(li) => Some(li),
                    None => {
                        let li = h.create_line_item_for_building_type(dept, line, true);
                        if let Some(li) = li {
                            h.set_line_item_display_flag(li, LINE_ITEM_DISPLAY_FLAG, true);
                        }
                        li
                    }
                };
                h.add_locally_funded(dept, b, svc.staff_purpose);
                li
            } else {
                h.remove_locally_funded(dept, b, svc.staff_purpose);
                // EH4: the original uses the line item unchecked (a crash
                // without one); [choice] no count step and no RemoveLineItem.
                h.line_item(dept, line)
            };
            if let Some(li) = line_item {
                let n = h.line_item_count(li).wrapping_add(if added { 1 } else { -1 });
                h.set_line_item_count(li, n);
                if !added && n == 0 {
                    h.remove_line_item(dept, line);
                }
            }
            if let Some(v) = h.budget_item(b, svc.vehicle_purpose) {
                let Some(vd) = h.department_budget(v.department) else {
                    return false;
                };
                if added {
                    h.add_locally_funded(vd, b, svc.vehicle_purpose);
                } else {
                    h.remove_locally_funded(vd, b, svc.vehicle_purpose);
                }
            }
        }
        if added {
            h.set_occupant_u32(b, svc.served, NOT_GRADED);
            h.set_occupant_f32(b, svc.grade, 1.0);
            h.set_occupant_f32(b, svc.staff_ratio, 1.0);
            // EH5: the original writes a stack leftover here; [choice] 0.
            let capacity = self.prop_u32(b, svc.capacity).unwrap_or(0);
            h.set_occupant_u32(b, svc.effective_capacity, capacity);
            h.add_ref(b);
            let mut s = self.s.borrow_mut();
            let pending = if svc.school {
                &mut s.lists.pending_school
            } else {
                &mut s.lists.pending_hospital
            };
            pending.push(Some(b));
            s.dirty = true;
        } else {
            {
                let mut s = self.s.borrow_mut();
                let l = &mut s.lists;
                let (groups, pending) = if svc.school {
                    (&mut l.school_groups[..], &mut l.pending_school)
                } else {
                    (&mut l.hospital_groups[..], &mut l.pending_hospital)
                };
                let target = groups.iter_mut().find(|g| g.contains(&Some(b))).unwrap_or(pending);
                if let Some(i) = target.iter().position(|&o| o == Some(b)) {
                    target.remove(i);
                }
            }
            h.release(b);
            self.s.borrow_mut().dirty = true;
        }
        true
    }

    /// residential.md §5.8: a building with both health-affecting properties
    /// above 0.0 (read as floats; NaN fails) is appended to the pending list
    /// ("the other list") with `AddRef` when added; when removed it is erased from
    /// the health-affecting list, else from the pending list, and released if
    /// it was erased. Dirty is set either way.
    fn health_affecting_changed(&self, b: u64, added: bool) {
        let [p, q] = PROP_HEALTH_AFFECTING.map(|id| self.prop_f32(b, id));
        let affecting = matches!((p, q), (Some(x), Some(y)) if x > 0.0 && y > 0.0);
        if !affecting {
            return;
        }
        if added {
            self.host.add_ref(b);
            let mut s = self.s.borrow_mut();
            s.lists.other.push(Some(b));
            s.dirty = true;
            return;
        }
        let erased = {
            let mut s = self.s.borrow_mut();
            let l = &mut s.lists;
            let erase = |list: &mut Vec<Option<u64>>| match list.iter().position(|&o| o == Some(b)) {
                Some(i) => {
                    list.remove(i);
                    true
                }
                None => false,
            };
            erase(&mut l.health_affecting) || erase(&mut l.other)
        };
        if erased {
            self.host.release(b);
        }
        self.s.borrow_mut().dirty = true;
    }

    // ---- state 0 (§3, §4.9) ----

    /// State 0's education and health part (REH §3): reset both grade
    /// totals, the monthly strike check for schools then hospitals, then
    /// clear and re-read the ordinance effects. The pending lists follow
    /// (`simulator.rs`).
    pub(super) fn education_health_state_0(&self) {
        {
            let mut s = self.s.borrow_mut();
            s.school_grade_total = 0.0;
            s.hospital_grade_total = 0.0;
        }
        self.monthly_strike_check(&SCHOOL);
        self.monthly_strike_check(&HOSPITAL);
        self.read_ordinance_effects(true);
    }

    fn group_lists(&self, svc: &Service) -> Vec<Vec<Option<u64>>> {
        let s = self.s.borrow();
        if svc.school {
            s.lists.school_groups.to_vec()
        } else {
            s.lists.hospital_groups.to_vec()
        }
    }

    /// `(strike flag, start, duration, min grade to end, message type)`.
    fn strike_block(&self, svc: &Service) -> (bool, i32, i32, f32, u32) {
        let s = self.s.borrow();
        let b = if svc.school { &s.education } else { &s.health };
        (b.strike_flag != 0, b.strike_start, b.strike_duration, b.min_grade, b.id)
    }

    /// `DoMonthlyStrikeCheck(dept, groupLists, …, strikeGroup)` (§4.9).
    /// *[quirk]* `changed` is sticky: after the first change every striking
    /// building calls `BeginStrike`, a no-op while the flag is set.
    fn monthly_strike_check(&self, svc: &Service) {
        let mut changed = false;
        let mut count = 0u32;
        for occ in self.group_lists(svc).into_iter().flatten().flatten() {
            if !self.host.is_occupant_group(occ, svc.strike_group) {
                continue;
            }
            changed |= self.update_striking_state(occ, svc);
            if self.strike_day(occ) > 0 {
                count += 1;
                if changed {
                    self.begin_strike(svc, occ);
                }
            }
        }
        if changed && self.strike_block(svc).0 && count == 0 {
            self.end_strike(svc.school);
        }
    }

    /// `UpdateStrikingState(occ, dept, staffProp, gradeProp)` (§4.9): true
    /// when the building starts or ends a strike.
    fn update_striking_state(&self, occ: u64, svc: &Service) -> bool {
        let h = &*self.host;
        let s0 = self.strike_day(occ);
        let today = h.sim_day_number();
        let (_, _, duration, min_grade, msg_type) = self.strike_block(svc);
        let started = s0 == 0;
        let new = if !started {
            // Ends after `duration` **days** (u32 compare) *[quirk]*, or when
            // the staff ratio (default 1.0) reaches "min grade" *[quirk]*; a
            // NaN ratio keeps the strike.
            let ends = today.wrapping_sub(s0) > duration as u32
                || self.prop_f32(occ, svc.staff_ratio).unwrap_or(1.0) >= min_grade;
            if !ends {
                return false;
            }
            0
        } else {
            let g = self.prop_f32(occ, svc.grade).unwrap_or(0.0);
            let p = {
                let s = self.s.borrow();
                let t = &s.simulator_tuning;
                let curve = if svc.school {
                    &t.school_strike_curve
                } else {
                    &t.health_strike_curve
                };
                curve.eval(g * 100.0)
            };
            // Exactly one draw per eligible non-striking building, even when
            // p = 0; p is a percentage (no 0.01).
            let r = self.rng.borrow_mut().range_f64(0.0, 100.0);
            if r >= f64::from(p) {
                return false;
            }
            today
        };
        h.set_occupant_u32(occ, PROP_STRIKE_DAY, new);
        let name = if msg_type == super::msg::HEALTH_STRIKE {
            STRIKING_HEALTH
        } else {
            STRIKING_EDUCATION
        };
        if started {
            h.add_controllers(name, occ);
        } else {
            h.remove_controllers(name, occ);
        }
        true
    }

    /// `BeginStrike(dept, occ)` (§4.9): false while the flag is set;
    /// otherwise flag := 1, start := today and the strike message (void1 =
    /// the occupant, data2 = 1) is posted.
    fn begin_strike(&self, svc: &Service, occ: u64) -> bool {
        let today = self.host.sim_day_number();
        let begun = {
            let mut s = self.s.borrow_mut();
            let b = if svc.school { &mut s.education } else { &mut s.health };
            b.begin_strike(today).then_some(b.id)
        };
        match begun {
            Some(msg_type) => {
                self.host.post_strike_message(msg_type, Some(occ), 1, 0);
                true
            }
            None => false,
        }
    }

    /// Slots 10 / 11 when not on strike: `p := (c + p) − c × p` over the group
    /// lists (§4.9).
    pub(super) fn combined_strike_chance(&self, svc: &Service) -> f32 {
        let mut p = 0.0f32;
        for list in self.group_lists(svc) {
            let c = self.list_strike_chance(&list, svc);
            p = (c + p) - c * p;
        }
        p
    }

    /// `GetChanceOfStrike(list)` (§4.9). *[quirk]* A building outside the
    /// strike group reuses the previous c (0 before the first member), and g
    /// keeps the previous grade when the property is missing.
    fn list_strike_chance(&self, list: &[Option<u64>], svc: &Service) -> f32 {
        let (mut q, mut g, mut c) = (0.0f32, 0.0f32, 0.0f32);
        for &occ in list.iter().flatten() {
            if self.host.is_occupant_group(occ, svc.strike_group) {
                if let Some(x) = self.prop_f32(occ, svc.grade) {
                    g = x;
                }
                let s = self.s.borrow();
                let t = &s.simulator_tuning;
                let curve = if svc.school {
                    &t.school_strike_curve
                } else {
                    &t.health_strike_curve
                };
                c = pct(curve.eval(g * 100.0));
            }
            q = (q + c) - q * c;
        }
        q
    }

    // ---- coverage and grading (§4.4–§4.6, §4.10) ----

    fn frame(&self, school: bool) -> Frame {
        let s = self.s.borrow();
        let g = &s.grids;
        let t = &s.simulator_tuning;
        Frame {
            svc: Service::of(school),
            lists: if school {
                s.lists.school_groups.to_vec()
            } else {
                s.lists.hospital_groups.to_vec()
            },
            coverage: if school {
                g.school_coverage.to_vec()
            } else {
                g.hospital_coverage.to_vec()
            },
            chunks: if school {
                t.school_chunks.clone()
            } else {
                t.hospital_chunks.clone()
            },
            grade_grid: if school {
                g.school_grade.clone()
            } else {
                g.hospital_grade.clone()
            },
            quotients: if school { g.eq.clone() } else { g.hq.clone() },
            population: g.population.clone(),
            age: g.age.clone(),
            slop: t.coverage_slop,
            auto_budget: if school {
                s.school_auto_budget
            } else {
                s.hospital_auto_budget
            },
        }
    }

    /// States 5 / 7 (REH §3, §4.4): reset the served counter, then
    /// `UpdateCoverageMap` for every group in order.
    pub(super) fn coverage_state(&self, school: bool) {
        let f = self.frame(school);
        let mut served = 0i32;
        for i in 0..f.lists.len() {
            self.update_coverage_map(&f, i, &mut served);
        }
        let mut s = self.s.borrow_mut();
        if school {
            s.school_covered = served;
        } else {
            s.hospital_covered = served;
        }
    }

    /// `UpdateCoverageMap(service, i)` (§4.4).
    fn update_coverage_map(&self, f: &Frame, i: usize, served: &mut i32) {
        let Some(cov) = f.coverage.get(i).cloned().flatten() else {
            return;
        };
        cov.fill(0);
        let chunk = f.chunk(i);
        for &occ in f.lists[i].iter().flatten() {
            let (_, e2) = self.efficiency_ratings(occ, f.svc.school, false);
            let effects = self.ordinance_effects_for(occ, f.svc, f.svc.ordinance_kind);
            let mut m = e2;
            for e in &effects {
                m *= e.radius;
            }
            self.mark_building_coverage(f, i, &*cov, occ, m, chunk, served);
        }
    }

    /// `MarkBuildingCoverage` (§4.4). *[quirk]* The served test ignores the
    /// units, so a building with units 0 still counts the tract, again for
    /// every such building; the saturation tests the low 16 bits of the sum.
    #[allow(clippy::too_many_arguments)]
    fn mark_building_coverage(
        &self,
        f: &Frame,
        i: usize,
        cov: &dyn TractGrid<u8>,
        occ: u64,
        m: f32,
        chunk: u32,
        served: &mut i32,
    ) {
        let (Some(radius), Some(capacity)) = (self.prop_f32(occ, f.svc.radius), self.prop_u32(occ, f.svc.capacity))
        else {
            return;
        };
        let r = radius * m;
        if r <= 0.0 {
            return;
        }
        let units = units_of(capacity, chunk);
        for (x, z) in self.covered_tracts(occ, r, cov.geometry(), f.slop) {
            let old = cov.get(x, z);
            let sum = u32::from(old).wrapping_add(units);
            cov.set(x, z, if sum & 0xFFFF > 255 { 255 } else { (sum & 0xFF) as u8 });
            let lower = f.coverage[..i].iter().flatten().any(|g| g.get(x, z) != 0);
            if old == 0 && !lower {
                for p in f.population.iter().flatten() {
                    *served = served.wrapping_add(i32::from(p.get(x, z)));
                }
            }
        }
    }

    /// `GetCoveredTractList(occ, r, grid)` (§4.4).
    fn covered_tracts(&self, occ: u64, r: f32, g: TractGeometry, slop: f32) -> Vec<(i32, i32)> {
        let (px, pz) = self.centre(occ);
        tracts_in_radius(px, pz, r * slop, g.width_x, g.count_x, g.count_z)
    }

    /// States 6 / 8 (REH §3): grade grid := 0, then for every group and
    /// building in list order the auto-budget step (§4.10) and
    /// `ComputeOccupantGrade` (§4.5) with max 200.
    pub(super) fn grading_state(&self, school: bool) {
        let f = self.frame(school);
        if let Some(g) = &f.grade_grid {
            g.fill(0.0);
        }
        for i in 0..f.lists.len() {
            for &occ in f.lists[i].iter().flatten() {
                if f.auto_budget || self.host.occupant_bool_property(occ, PROP_AUTO_BUDGET) == Some(true) {
                    self.auto_budget(occ, school);
                }
                self.compute_occupant_grade(&f, i, occ);
            }
        }
    }

    /// The auto-budget step (§4.10): only the staff percentage changes.
    fn auto_budget(&self, occ: u64, school: bool) {
        let (mut a, mut b) = (0.0f32, 0.0f32);
        self.get_occupant_funding(occ, &mut a, &mut b, school);
        if let Some(f) = self.estimate_ideal_funding(occ) {
            self.set_occupant_funding(occ, f, b, school);
        }
    }

    /// `ComputeOccupantGrade` (§4.5) with its grade-grid blend and the EQ/HQ
    /// boost (§4.6).
    fn compute_occupant_grade(&self, f: &Frame, i: usize, occ: u64) {
        let h = &*self.host;
        let svc = f.svc;
        let Some(radius) = self.prop_f32(occ, svc.radius) else {
            h.set_occupant_u32(occ, svc.served, 0);
            h.set_occupant_u32(occ, svc.effective_capacity, 0);
            h.set_occupant_f32(occ, svc.grade, 0.0);
            return;
        };
        // EH5: the original grades a missing capacity as the radius
        // property's ID (a stack leftover); [choice] 0.
        let capacity = self.prop_u32(occ, svc.capacity).unwrap_or(0);
        let (e1, e2) = self.efficiency_ratings(occ, svc.school, false);
        let base = self.base_staff_efficiency(occ, svc.school);
        let c1 = self.prop_curve(occ, svc.age_curve);
        let c2 = self.prop_curve(occ, svc.wealth_curve);
        let c3 = self.prop_curve(occ, svc.distance_curve);
        let effects = self.ordinance_effects_for(occ, svc, svc.ordinance_kind);
        let (m1, m2, m3) = ordinance_modifiers(&effects);
        let r = (e2 * m1) * radius;
        let tracts = match &f.population[0] {
            Some(p) => self.covered_tracts(occ, r, p.geometry(), f.slop),
            None => Vec::new(),
        };
        let cov = f.coverage.get(i).cloned().flatten();
        let g = Grading {
            occ,
            r,
            units: units_of(capacity, f.chunk(i)),
            c1: &c1,
            c2: &c2,
            c3: &c3,
            effects: &effects,
            coverage: cov.as_deref(),
        };
        let served = self.population_served(f, &g, &tracts);
        let ratio = |cap: u32| {
            if cap == 0 {
                0.0
            } else if served == 0 {
                1.0
            } else {
                min_cap(1.0, ratio_u32(cap, served))
            }
        };
        let staff = ratio(scaled_capacity(capacity, base, m2));
        let eff_cap = scaled_capacity(capacity, e1, m2);
        let grade = ratio(eff_cap);
        h.set_occupant_f32(occ, svc.staff_ratio, staff);
        h.set_occupant_f32(occ, svc.grade, grade);
        h.set_occupant_u32(occ, svc.effective_capacity, eff_cap);
        h.set_occupant_u32(occ, svc.served, served);

        // The grade grid *[quirk]*: the previous value is multiplied by n − 1
        // rather than averaged, and n = 0 gives t / 0 (NaN for t = 0, kept by
        // the min).
        if let (Some(gg), Some(cov)) = (&f.grade_grid, &cov) {
            let t = mul_u32(g.units as i32, grade);
            for &(x, z) in &tracts {
                let n = cov.get(x, z);
                let prev = if n > 1 {
                    mul_i(i64::from(n - 1), gg.get(x, z))
                } else {
                    0.0
                };
                gg.set(x, z, min_cap(1.0, prev + t / f32::from(n)));
            }
        }

        if r > 0.0 {
            self.add_boost(f, &g, grade, m3);
        }
    }

    /// `GetPopulationServed` (§4.5): Σ over the listed covered tracts of the
    /// building's share of each wealth's population, shaped by the age,
    /// wealth and distance curves; rounded half to even (`rhe`).
    /// *[quirk]* dx is `float(u32(tx − x))` and the distance in tracts is
    /// divided by r' in metres.
    fn population_served(&self, f: &Frame, g: &Grading<'_>, tracts: &[(i32, i32)]) -> u32 {
        if g.r <= 0.0 {
            return 0;
        }
        // EH6 *[compat]*: the building's own tract from its `GetPosition`
        // alone, with no lot lookup (the tract list used the lot centre).
        let (px, pz) = self.host.occupant_position(g.occ);
        let (tx, tz) = f.population[0]
            .as_ref()
            .map_or((0, 0), |p| p.geometry().position_to_tract(px, pz));
        let wealth: [f32; 3] = std::array::from_fn(|w| pct(g.c2.eval(w as f32)));
        let mut sum = 0.0f32;
        for &(x, z) in tracts {
            let n = g.coverage.map_or(0, |c| c.get(x, z));
            if n == 0 {
                continue;
            }
            let dx = u32_to_float(tx.wrapping_sub(x)) as f32;
            let dzi = tz.wrapping_sub(z);
            let dz2 = u32_to_float(dzi.wrapping_mul(dzi)) as f32;
            let d = (dz2 + dx * dx).sqrt();
            let xd = (d / g.r) * 100.0;
            let age = f32::from(f.age.as_ref().map_or(0, |a| a.get(x, z)));
            let mut md = pct(g.c3.eval(xd));
            let mut ma = pct(g.c1.eval(age));
            for e in g.effects {
                md = pct(e.distance_curve.eval(xd) * md);
                ma = pct(e.age_curve.eval(age) * ma);
            }
            let k = per_unit(g.units, n);
            for (w, c2) in wealth.iter().enumerate() {
                let pop = f.population[w].as_ref().map_or(0, |p| p.get(x, z));
                let share = mul_i(i64::from(pop), k);
                sum += ((c2 * share) * ma) * md;
            }
        }
        rhe(sum) as u32
    }

    /// The EQ (schools) or HQ (hospitals) boost at the end of grading (§4.6).
    /// *[quirk]* The curve lists filter with the school kind property even for
    /// hospitals; the filter is empty, so this changes nothing.
    fn add_boost(&self, f: &Frame, g: &Grading<'_>, grade: f32, m3: f32) {
        let svc = f.svc;
        let mult = self.prop_f32(g.occ, svc.effect_multiplier).unwrap_or(1.0);
        let (px, pz) = self.centre(g.occ);
        let curve_effects = self.ordinance_effects_for(g.occ, svc, SCHOOL.ordinance_kind);
        let list_a: Vec<&ResponseCurve> = curve_effects.iter().map(|e| &e.distance_curve).chain([g.c3]).collect();
        let list_b: Vec<&ResponseCurve> = curve_effects.iter().map(|e| &e.age_curve).chain([g.c1]).collect();
        let mut total = {
            let s = self.s.borrow();
            if svc.school {
                s.school_grade_total
            } else {
                s.hospital_grade_total
            }
        };
        for w in 0..3 {
            let Some(target) = &f.quotients[w] else {
                continue;
            };
            let value = ((grade * pct(g.c2.eval(w as f32))) * mult) * m3;
            let e = RadiusEffect {
                age: f.age.as_deref(),
                target: &**target,
                age_curves: &list_b,
                x: px,
                z: pz,
                radius: g.r * f.slop,
                distance_curves: &list_a,
                value,
                max: MAX_QUOTIENT as f32,
                units: g.units,
                coverage: g.coverage,
                population: f.population[w].as_deref(),
            };
            add_radius_effect_with_curves(&e, &mut total);
        }
        let mut s = self.s.borrow_mut();
        if svc.school {
            s.school_grade_total = total;
        } else {
            s.hospital_grade_total = total;
        }
    }

    // ---- ideal funding (§4.10) ----

    /// Slot 58 `EstimateIdealFunding(occ, &f)` (§4.10): the first point of the
    /// grid r ± k × 0.01 where the efficiency reaches r = served / (capacity ×
    /// m2), clamped to [0, max funding]; `None` (AL = 0) when neither query
    /// succeeds or the building is not graded yet.
    pub fn estimate_ideal_funding(&self, occ: u64) -> Option<f32> {
        // EH7 *[compat]*: the school query first, then the hospital query.
        let mut q = QueryData::default();
        let svc = if self.query_data_into(true, occ, &mut q) {
            &SCHOOL
        } else if self.query_data_into(false, occ, &mut q) {
            &HOSPITAL
        } else {
            return None;
        };
        // EH9: the served property itself (read into a value preset to
        // 0xFFFFFFFF, so a missing one fails too), not the record's +0x24.
        if self.prop_u32(occ, svc.served).unwrap_or(NOT_GRADED) == NOT_GRADED {
            return None;
        }
        let effects = self.ordinance_effects_for(occ, svc, svc.ordinance_kind);
        let (_, m2, _) = ordinance_modifiers(&effects);
        let r = ratio_u32(q.served, ftol32(f64::from(mul_u32(q.capacity as i32, m2))) as u32);
        let (curve, limit) = {
            let s = self.s.borrow();
            if svc.school {
                (s.simulator_tuning.school_funding_curve.clone(), s.education.max_funding)
            } else {
                (s.simulator_tuning.health_funding_curve.clone(), s.health.max_funding)
            }
        };
        let v_of = |f: f32| pct(curve.eval(f * 100.0));
        let mut f = r;
        let mut v = v_of(f);
        while v > r {
            let next = f - PERCENT;
            if next == f {
                break; // [choice] the original would loop forever.
            }
            f = next;
            v = v_of(f);
            if f <= 0.0 {
                f = 0.0;
                break;
            }
        }
        while v < r {
            let next = f + PERCENT;
            if next == f {
                break; // [choice]
            }
            f = next;
            v = v_of(f);
            if f > limit {
                f = limit;
                break;
            }
        }
        Some(f)
    }

    // ---- queries (§4.11) ----

    /// The query record of slots 26 / 27 (§4.11, EH10): capacity, effective
    /// capacity and served (raw) are read straight into the record, then the
    /// grade; the hospital query then requires its staff item. The first
    /// missing one returns false (AL = 0) and leaves the fields read before
    /// it. Only a successful query writes +0x1C..+0x38 and maps served
    /// `0xFFFFFFFF` to 0.
    pub fn query_data_into(&self, school: bool, occ: u64, q: &mut QueryData) -> bool {
        let h = &*self.host;
        let svc = Service::of(school);
        let divisor = {
            let s = self.s.borrow();
            if school {
                s.simulator_tuning.school_staff_divisor
            } else {
                s.simulator_tuning.hospital_staff_divisor
            }
        };
        let Some(capacity) = self.prop_u32(occ, svc.capacity) else {
            return false;
        };
        q.capacity = capacity;
        let Some(eff) = self.prop_u32(occ, svc.effective_capacity) else {
            return false;
        };
        q.effective_capacity = eff;
        let Some(served) = self.prop_u32(occ, svc.served) else {
            return false;
        };
        q.served = served; // raw until the query succeeds
        let Some(grade) = self.prop_f32(occ, svc.grade) else {
            return false;
        };
        let staff = match h.budget_item(occ, svc.staff_purpose) {
            Some(i) => (i.local_funding, i.cost),
            None if school => (1.0, 0),
            None => return false,
        };
        let vehicle = match h.budget_item(occ, svc.vehicle_purpose) {
            Some(i) => (i.local_funding, i.cost),
            None => (1.0, 0),
        };
        q.capacity_per_staff = capacity.checked_div(divisor).unwrap_or(0);
        // *[quirk]* the school rounds up, the hospital down.
        q.effective_per_staff = if school {
            ceil_div(eff, divisor)
        } else {
            eff.checked_div(divisor).unwrap_or(0)
        };
        if served == NOT_GRADED {
            q.served = 0;
        }
        q.grade = grade;
        (q.staff_funding, q.staff_cost) = staff;
        (q.vehicle_funding, q.vehicle_cost) = vehicle;
        true
    }

    /// Slots 26 / 27 on a fresh record; `None` = AL 0.
    pub fn school_query_data(&self, occ: u64) -> Option<QueryData> {
        let mut q = QueryData::default();
        self.query_data_into(true, occ, &mut q).then_some(q)
    }
    pub fn hospital_query_data(&self, occ: u64) -> Option<QueryData> {
        let mut q = QueryData::default();
        self.query_data_into(false, occ, &mut q).then_some(q)
    }

    /// Slots 12 / 13 (hidden pointer): Σ effective capacity over the group
    /// lists ÷ the population, clamped to 1.0; 0 when the sum is 0, 1.0 when
    /// the population is 0. *[quirk]* The query record is not reset between
    /// buildings, so a failed query adds the previous building's value again.
    /// EH11: the original's record starts with stack leftovers; *[choice]* 0.
    pub fn system_rating(&self, school: bool) -> f32 {
        let mut q = QueryData::default();
        let mut sum = 0u32;
        for occ in self.group_lists(Service::of(school)).into_iter().flatten().flatten() {
            self.query_data_into(school, occ, &mut q);
            sum = sum.wrapping_add(q.effective_capacity);
        }
        let pop = self.get_population();
        if sum == 0 {
            0.0
        } else if pop == 0 {
            1.0
        } else {
            min_cap(1.0, (u32_to_float(sum as i32) / f64::from(pop)) as f32)
        }
    }
    pub fn school_system_rating(&self) -> f32 {
        self.system_rating(true)
    }
    pub fn health_system_rating(&self) -> f32 {
        self.system_rating(false)
    }

    /// Slots 14 / 15 (§4.11, EH12): the caller's +0x14..+0x24 := 0 and +0x28
    /// := 0.0 first (+0x2C..+0x38 are not touched). Then every building of
    /// every group list, in list order, that is in at least one of `groups`
    /// (`IsOccupantGroup`, per building) runs the service's query into one
    /// record, **whatever its result**, and adds the record's +0x14..+0x24
    /// (u32, wrapping) and +0x28 (float32) to the totals. Returns the number
    /// of buildings counted (EAX). A failed query adds what it read plus the
    /// previous building's other fields. The original crashes on a null
    /// `groups` *[quirk]*; a slice cannot be null.
    pub fn system_totals(&self, school: bool, groups: &[u32], totals: &mut QueryData) -> u32 {
        let svc = Service::of(school);
        totals.capacity = 0;
        totals.effective_capacity = 0;
        totals.capacity_per_staff = 0;
        totals.effective_per_staff = 0;
        totals.served = 0;
        totals.grade = 0.0;
        // [choice] (§4.11, AQ1, recommended): the original sets up its record
        // once with only +0x10, +0x28, +0x2C and +0x34, so +0x14..+0x24 hold
        // stack leftovers until a query writes them (a farmer's market
        // counted first adds two leftover words at +0x1C and +0x20).
        // FreeCity4 starts them at 0, as EH11's ratings record.
        let mut q = QueryData::default();
        let mut count = 0u32;
        for occ in self.group_lists(svc).into_iter().flatten().flatten() {
            if !groups.iter().any(|&g| self.host.is_occupant_group(occ, g)) {
                continue;
            }
            self.query_data_into(school, occ, &mut q); // the result is ignored
            totals.capacity = totals.capacity.wrapping_add(q.capacity);
            totals.effective_capacity = totals.effective_capacity.wrapping_add(q.effective_capacity);
            totals.capacity_per_staff = totals.capacity_per_staff.wrapping_add(q.capacity_per_staff);
            totals.effective_per_staff = totals.effective_per_staff.wrapping_add(q.effective_per_staff);
            totals.served = totals.served.wrapping_add(q.served);
            totals.grade += q.grade;
            count = count.wrapping_add(1);
        }
        count
    }
    pub fn school_system_totals(&self, groups: &[u32], totals: &mut QueryData) -> u32 {
        self.system_totals(true, groups, totals)
    }
    pub fn hospital_system_totals(&self, groups: &[u32], totals: &mut QueryData) -> u32 {
        self.system_totals(false, groups, totals)
    }

    /// Slot 28 `EstimateCurrentOccupantCapacity(occ, &cap, &staff)` (§4.11):
    /// cap = trunc((capacity × e1) × m2) with e1 ignoring strikes, staff =
    /// ceil(cap / **school** divisor) for both services *[quirk]*.
    pub fn estimate_occupant_capacity(&self, occ: u64) -> (u32, u32) {
        // EH7 *[compat]*: the school if the building has the school capacity,
        // else the hospital, with no test that its capacity exists (a missing
        // one reads as 0, so cap = staff = 0).
        let svc = if self.has_prop(occ, SCHOOL.capacity) {
            &SCHOOL
        } else {
            &HOSPITAL
        };
        let capacity = self.prop_u32(occ, svc.capacity).unwrap_or(0);
        let (e1, _) = self.efficiency_ratings(occ, svc.school, true);
        let effects = self.ordinance_effects_for(occ, svc, svc.ordinance_kind);
        let (_, m2, _) = ordinance_modifiers(&effects);
        let cap = scaled_capacity(capacity, e1, m2);
        let divisor = self.s.borrow().simulator_tuning.school_staff_divisor;
        (cap, ceil_div(cap, divisor))
    }

    /// Slot 45 `GetOccupantCoverage(occ, level, &dist, &radius)` (§4.11):
    /// `(distance at level, radius)`; `None` (AL = 0) for a building that is
    /// neither a school nor a hospital. dist walks the distance curve's
    /// points from the second on, keeping the last one above `level × 100`,
    /// and interpolates at the first one at or below it; *[quirk]* the
    /// `x_prev` term has no 0.01 factor. EH8: AL = 1 whenever the kind is
    /// found; no radius reads as 0; no qualifying point gives dist 0.
    pub fn occupant_coverage(&self, occ: u64, level: f32) -> Option<(f32, f32)> {
        // EH7 *[quirk]*: the hospital wins the radius, kind and ordinance
        // list; the efficiency ratings and the distance curve follow the
        // school flag.
        let (school_flag, svc) = self.coverage_kind(occ);
        let svc = svc?;
        let (_, e2) = self.efficiency_ratings(occ, school_flag, false);
        let effects = self.ordinance_effects_for(occ, svc, svc.ordinance_kind);
        let (m1, _, _) = ordinance_modifiers(&effects);
        let radius = (e2 * m1) * self.prop_f32(occ, svc.radius).unwrap_or(0.0);
        let l = level * 100.0;
        let curve = Service::of(school_flag).distance_curve;
        let points = match self.host.occupant_property(occ, curve) {
            // EH8: no points; the original leaves dist unwritten, [choice] 0.
            Some(v) if too_few_values(&v) => Vec::new(),
            Some(v) => props::curve_of_variant(&v).unwrap_or_else(flat_curve).points(),
            None => flat_curve().points(),
        };
        let mut dist = 0.0f32;
        if let Some((&first, rest)) = points.split_first() {
            let mut prev = first;
            for &(x, y) in rest {
                if y > l {
                    prev = (x, y);
                    continue;
                }
                let (xp, yp) = prev;
                dist = (((yp - l) / (yp - y)) * pct(x - xp)) * radius + xp * radius;
                break;
            }
        }
        Some((dist, radius))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REH test 3: the tract list of `GetTractsInRadius`.
    #[test]
    fn tract_list() {
        assert_eq!(tracts_in_radius(32.0, 32.0, 60.0, 64.0, 8, 8), vec![(0, 0)]);
        assert_eq!(
            tracts_in_radius(32.0, 32.0, 100.0, 64.0, 8, 8),
            vec![(0, 0), (0, 1), (1, 0), (1, 1)]
        );
        // x0 = trunc(−1.71875) = −1, raised to 0.
        assert_eq!(tract_range(-10.0, 100.0, 1.0 / 64.0, 8, true), (0, 1));
        assert_eq!(tracts_in_radius(-10.0, 32.0, 100.0, 64.0, 8, 8), vec![(0, 0), (0, 1)]);
        // The unsigned upper compare: a negative upper bound becomes count − 1;
        // the signed one keeps it.
        assert_eq!(tract_range(-200.0, 10.0, 1.0 / 64.0, 8, true), (0, 7));
        assert_eq!(tract_range(-200.0, 10.0, 1.0 / 64.0, 8, false), (0, -2));
    }

    #[test]
    fn helpers() {
        assert_eq!(ceil_div(990, 30), 33);
        assert_eq!(ceil_div(991, 30), 34);
        assert_eq!(ceil_div(5, 0), 0);
        assert_eq!(units_of(1500, 500), 3);
        assert_eq!(units_of(400, 500), 0);
        assert_eq!(units_of(400, 0), 0);
        assert_eq!(ratio_u32(1000, 1250), 0.8);
        assert_eq!(per_unit(5, 2), 2.5);
    }
}
