// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The Lua `game` variables of the advisor classes (§7.3–§7.8) and the
//! Safety messages (§7.6).
//!
//! Every value comes from [`AdvisorHost::value`] / [`AdvisorHost::list`];
//! a source the host cannot answer leaves its variable unwritten [choice].
//! `update_variables` ends with the class's occupant variables (CityPlanning
//! as the spec states; the other classes [choice]) and, for Utility, the
//! neighbour-deal variables [choice].

use std::rc::Rc;

use crate::x87::r24;

use super::advisor::Advisor;
use super::subject::AdviceSubject;
use super::{
    disaster, event, msg, pct100, AdvisorHost, AdvisorKind, AdvisorMessage, Item, ListKind, ListSrc, Src, SrcKind, Val,
    ITEM_LOT_EXISTS, ITEM_ROAD_ACCESS,
};

use SrcKind as K;

/// `FLT_MAX`, the empty-list minimum (§7.5).
const FLT_MAX: f32 = f32::MAX;
/// The GA_CRIME history type (§7.6).
pub const HISTORY_GA_CRIME: u32 = 0xAA66_2693;

/// Departments (§7.3–§7.8).
pub mod dept {
    pub const PARKS: u32 = 0x8A42_60A6;
    pub const LANDMARKS: u32 = 0x8A41_6368;
    pub const HEALTH: u32 = 0x0918_8F42;
    pub const EDUCATION: u32 = 0x0918_8F4C;
    pub const FIRE: u32 = 0x28F5_5A9F;
    pub const POLICE: u32 = 0xA296_3983;
    pub const WATER: u32 = 0xAA40_14B4;
    pub const POLLUTION: u32 = 0x6A42_A8F5;
}

/// Occupant groups of the HQ lists (§7.5).
pub mod groups {
    pub const HOSPITAL: u32 = 0x1513;
    pub const CLINIC: u32 = 0x1512;
    pub const ELEMENTARY: u32 = 0x150F;
    pub const HIGH: u32 = 0x1510;
    pub const COLLEGE: u32 = 0x1504;
    pub const PRIVATE_SCHOOL: u32 = 0x1514;
    pub const MUSEUM: u32 = 0x1506;
    pub const LIBRARY: u32 = 0x1505;
    pub const ALL_HEALTH: [u32; 2] = [HOSPITAL, CLINIC];
    pub const ALL_SCHOOLS: [u32; 4] = [ELEMENTARY, HIGH, PRIVATE_SCHOOL, COLLEGE];
}

/// The deal types (neighbors.md), by variable suffix (§7.8).
pub const DEAL_TYPES: [(&str, u32); 6] = [
    ("buy_water", 0x4773_FFE2),
    ("sell_water", 0x4773_FFE3),
    ("buy_power", 0x4773_FFE0),
    ("sell_power", 0x4773_FFE1),
    ("export_garbage", 0x4773_FFE4),
    ("import_garbage", 0x4773_FFE5),
];

/// Neighbour connection types.
pub mod conn {
    pub const RAIL: u32 = 1;
    pub const ROAD: u32 = 2;
    pub const POWER_POLE: u32 = 4;
    pub const PIPE: u32 = 5;
    pub const AVENUE: u32 = 8;
}

struct Ctx<'a> {
    adv: &'a Rc<Advisor>,
    h: Rc<dyn AdvisorHost>,
}

impl Ctx<'_> {
    fn new(adv: &Rc<Advisor>) -> Ctx<'_> {
        Ctx { adv, h: adv.host() }
    }
    fn v(&self, k: SrcKind, a: u32, b: u32) -> Option<Val> {
        self.h.value(&Src::ab(k, a, b))
    }
    fn list(&self, kind: ListKind, a: u32, groups: &[u32]) -> Option<Vec<Item>> {
        self.h.list(&ListSrc {
            kind: Some(kind),
            a,
            groups: groups.to_vec(),
        })
    }
    fn set(&self, name: &str, x: f64) {
        self.h.set_global(name, x);
    }
    fn local(&self, name: &str, x: f64, s: Option<Rc<AdviceSubject>>) {
        self.h.set_local(name, x, s.as_ref());
    }
    /// A subject at a cell, or null.
    fn cell(&self, c: Option<(f32, f32)>) -> Option<Rc<AdviceSubject>> {
        c.map(|(x, z)| {
            let s = AdviceSubject::new(&self.adv.shared);
            s.set_location(x, z);
            s
        })
    }
    /// A subject holding an object, or null for 0.
    fn obj(&self, o: u64) -> Option<Rc<AdviceSubject>> {
        (o != 0).then(|| {
            let s = AdviceSubject::new(&self.adv.shared);
            s.set_object(Some(o));
            s
        })
    }
    fn int(&self, name: &str, k: SrcKind, a: u32) {
        if let Some(v) = self.v(k, a, 0) {
            self.set(name, v.i as f64);
        }
    }
    fn int32(&self, name: &str, k: SrcKind, a: u32) {
        if let Some(v) = self.v(k, a, 0) {
            self.set(name, f64::from(v.i as i32));
        }
    }
    fn u32v(&self, name: &str, k: SrcKind, a: u32) {
        if let Some(v) = self.v(k, a, 0) {
            self.set(name, f64::from(v.i as u32));
        }
    }
    fn u8v(&self, name: &str, k: SrcKind, a: u32) {
        if let Some(v) = self.v(k, a, 0) {
            self.set(name, f64::from(v.i as u8));
        }
    }
    fn float(&self, name: &str, k: SrcKind, a: u32, b: u32) {
        if let Some(v) = self.v(k, a, b) {
            self.set(name, f64::from(v.f32()));
        }
    }
    fn pct(&self, name: &str, k: SrcKind, a: u32, b: u32) {
        if let Some(v) = self.v(k, a, b) {
            self.set(name, pct100(v.f32()));
        }
    }
    fn funding(&self, name: &str, department: u32) {
        self.pct(name, K::DepartmentFunding, department, 0);
    }
}

/// `f32(a × 100)` rounded once (×100).
fn x100(v: f32) -> f32 {
    r24(f64::from(v) * 100.0) as f32
}

/// Class slot 9 `update_variables`.
pub(super) fn update_variables(adv: &Rc<Advisor>) {
    match adv.kind() {
        AdvisorKind::CityPlanning => city_planning(adv),
        AdvisorKind::Finance => finance(adv),
        AdvisorKind::Hq => hq(adv),
        AdvisorKind::Safety => safety(adv),
        AdvisorKind::Traffic => traffic(adv),
        AdvisorKind::Utility => {
            utility(adv);
            deal_variables(adv);
            return;
        }
        // The base and CitySituations have no variables [inferred].
        _ => return,
    }
    occupant_variables(adv);
}

/// The occupant variables (UpdateSelf, §5.5).
pub(super) fn occupant_variables(adv: &Rc<Advisor>) {
    match adv.kind() {
        AdvisorKind::CityPlanning => city_planning_occupants(adv),
        AdvisorKind::Hq => hq_occupants(adv),
        AdvisorKind::Safety => safety_occupants(adv),
        AdvisorKind::Traffic => traffic_occupants(adv),
        AdvisorKind::Utility => utility_occupants(adv),
        _ => {}
    }
}

// ---- CityPlanning (§7.3) ----

/// The region argument of the demand reads.
pub const REGION: u32 = 0x20000;

fn city_planning(adv: &Rc<Advisor>) {
    let c = Ctx::new(adv);
    if let Some(v) = c.v(K::MayorRating, 0, 0) {
        c.set("ga_mayor_rating", f64::from(v.i as i8));
    }
    for (name, k) in [
        ("l_mayor_rating_l", K::MinMayorRating),
        ("l_mayor_rating_h", K::MaxMayorRating),
    ] {
        if let Some(v) = c.v(k, 0, 0) {
            c.local(name, f64::from(v.i as i8), c.cell(v.cell));
        }
    }
    // Populations: city (b = 0) then region (b = 0x20000).
    for (scope, b) in [("city", 0u32), ("region", REGION)] {
        let r = c.v(K::DemandSupply, 0x1001, b).map(|v| v.f32());
        let cc = c.v(K::DemandSupply, 0x3001, b).map(|v| v.f32());
        let i = c.v(K::DemandSupply, 0x4001, b).map(|v| v.f32());
        for (n, v) in [("r", r), ("c", cc), ("i", i)] {
            if let Some(v) = v {
                c.set(&format!("g_{scope}_{n}_population"), f64::from(v));
            }
        }
        if let (Some(r), Some(cc), Some(i)) = (r, cc, i) {
            c.set(&format!("g_{scope}_rci_population"), f64::from((i + cc) + r));
        }
        c.float(&format!("g_{scope}_workforce_population"), K::DemandSupply, 0x2100, b);
    }
    for (n, node) in [("r1", 0x1011), ("r2", 0x1021), ("r3", 0x1031)] {
        c.float(&format!("g_city_{n}_population"), K::DemandSupply, node, 0);
    }
    for (n, t) in [
        ("ir", 0x4101),
        ("id", 0x4201),
        ("im", 0x4301),
        ("iht", 0x4401),
        ("co3", 0x3331),
        ("co2", 0x3321),
        ("cs1", 0x3111),
        ("cs2", 0x3121),
        ("cs3", 0x3131),
    ] {
        c.u32v(&format!("g_city_{n}_population"), K::JobsByCensus, t);
    }
    // Region nodes [question: GetDemand(node, 0x20000)].
    for (n, node) in [
        ("r1", 0x1010),
        ("r2", 0x1020),
        ("r3", 0x1030),
        ("co2", 0x3320),
        ("co3", 0x3330),
    ] {
        c.float(&format!("g_{n}_demand"), K::DemandValue, node, REGION);
    }
    c.float("g_workforce_demand", K::DemandValue, 0x2000, REGION);
    for (n, node) in [
        ("r1", 0x1010),
        ("r2", 0x1020),
        ("r3", 0x1030),
        ("co2", 0x3320),
        ("co3", 0x3330),
        ("id", 0x4200),
        ("im", 0x4300),
        ("iht", 0x4400),
        ("workforce", 0x2000),
    ] {
        c.float(&format!("g_{n}_active_demand"), K::DemandActive, node, REGION);
    }
    for (n, node) in [
        ("id", 0x4200),
        ("im", 0x4300),
        ("iht", 0x4400),
        ("co2", 0x3320),
        ("co3", 0x3330),
        ("r1", 0x1010),
        ("r2", 0x1020),
        ("r3", 0x1030),
    ] {
        c.pct(&format!("g_current_{n}_cap"), K::DemandCap, node, REGION);
    }
    c.float("g_tax_rate_neutral", K::NeutralTaxRate, 0, 0);
    let zones = [
        "g_num_rzone_ld_tiles",
        "g_num_rzone_md_tiles",
        "g_num_rzone_hd_tiles",
        "g_num_czone_ld_tiles",
        "g_num_czone_md_tiles",
        "g_num_czone_hd_tiles",
        "g_num_izone_r_tiles",
        "g_num_izone_l_tiles",
        "g_num_izone_h_tiles",
    ];
    for (t, name) in zones.iter().enumerate() {
        c.int32(name, K::ZoneCount, t as u32 + 1);
    }
    c.u32v("g_num_rail_neighbors", K::ConnectionsOfType, conn::RAIL);
    c.u32v("g_num_road_neighbors", K::ConnectionsOfType, conn::ROAD);
    c.u32v("g_num_avenue_neighbors", K::ConnectionsOfType, conn::AVENUE);
    c.u32v("g_num_cities_connected", K::NeighborCities, 0);
    c.u32v("g_num_cities_adjacent", K::NeighborCities, 1);
    c.u32v("g_num_cities_connected_indirectly", K::NeighborCities, 2);
    if let Some(v) = c.v(K::CityHasSea, 0, 0) {
        c.set("g_city_has_sea", if v.i != 0 { 1.0 } else { 0.0 });
    }
    // Extrapolated demand: groups r, c, i.
    let rows: [(&str, u32, usize); 12] = [
        ("r1", 0x1010, 0),
        ("r2", 0x1020, 0),
        ("r3", 0x1030, 0),
        ("cs1", 0x3110, 1),
        ("cs2", 0x3120, 1),
        ("cs3", 0x3130, 1),
        ("co2", 0x3320, 1),
        ("co3", 0x3330, 1),
        ("ir", 0x4100, 2),
        ("id", 0x4200, 2),
        ("im", 0x4300, 2),
        ("ih", 0x4400, 2),
    ];
    let mut sums = [0i32; 3];
    let mut have = [false; 3];
    for (n, t, g) in rows {
        let (Some(e), Some(p)) = (c.v(K::ExtrapolatedPopulation, t, 0), c.v(K::RegionPopulation, t, 0)) else {
            continue;
        };
        let d = (e.i as i32).wrapping_sub(p.i as i32);
        c.set(&format!("g_{n}_demand_extrap"), f64::from(d));
        sums[g] = sums[g].wrapping_add(d);
        have[g] = true;
    }
    for (g, n) in ["r", "c", "i"].iter().enumerate() {
        if have[g] {
            c.set(&format!("g_{n}_demand_extrap"), f64::from(sums[g]));
        }
    }
    if let (Some(x), Some(z)) = (c.v(K::CitySizeX, 0, 0), c.v(K::CitySizeZ, 0, 0)) {
        c.set("g_city_tile_size", f64::from(x.f32() * z.f32()));
    }
    c.int32("g_water_tile_count", K::WaterTileCount, 0);
}

fn city_planning_occupants(adv: &Rc<Advisor>) {
    let c = Ctx::new(adv);
    c.funding("g_parks_funding_p", dept::PARKS);
    c.funding("g_landmarks_funding_p", dept::LANDMARKS);
    let s = adv.state();
    c.set("g_num_buildings", f64::from(s.building_count));
    c.set("g_num_parks", f64::from(s.park_count));
    c.set("g_num_recreation", f64::from(s.recreation_count));
}

// ---- Finance (§7.4) ----

/// The tax-rate suffixes, `GetTaxRate(0..11)`.
pub const TAX_RATES: [&str; 12] = [
    "r_low",
    "r_med",
    "r_high",
    "cs_low",
    "cs_med",
    "cs_high",
    "co_med",
    "co_high",
    "i_resource",
    "i_dirty",
    "i_manufacturing",
    "i_hightech",
];

fn finance(adv: &Rc<Advisor>) {
    let c = Ctx::new(adv);
    c.int("g_funds", K::TotalFunds, 0);
    c.int("g_income_monthly", K::IncomeMonthly, 0);
    c.int("g_expense_monthly", K::ExpenseMonthly, 0);
    for (i, n) in TAX_RATES.iter().enumerate() {
        c.float(&format!("g_tax_rate_{n}"), K::TaxRate, i as u32, 0);
    }
    c.int("g_tax_income", K::TaxIncome, 0);
    c.int("g_borrowed", K::Borrowed, 0);
    c.int("g_borrowing_limit", K::BorrowingLimit, 0);
    c.int("g_bond_payments_monthly", K::BondPaymentsMonthly, 0);
    c.int("g_nd_income", K::DealIncome, 0);
    c.int("g_nd_expense", K::DealExpense, 0);
}

// ---- HQ (§7.5) ----

fn hq(adv: &Rc<Advisor>) {
    let c = Ctx::new(adv);
    c.funding("g_health_funding_p", dept::HEALTH);
    c.funding("g_education_funding_p", dept::EDUCATION);
    c.u8v("ga_life_exp", K::GlobalLifeExpectancy, 0);
    c.float("ga_health", K::GlobalHealth, 0, 0);
    c.float("ga_education", K::GlobalEducation, 0, 0);
    // The life-expectancy tract values repeat the HQ ones [quirk].
    for (names, k) in [
        (["l_tract_hq_l", "l_tract_life_exp_l"], K::HqMin),
        (["l_tract_hq_h", "l_tract_life_exp_h"], K::HqMax),
    ] {
        if let Some(v) = c.v(k, 0, 0) {
            for n in names {
                c.local(n, f64::from(v.f32()), c.cell(v.cell));
            }
        }
    }
    for (n, k) in [("l_tract_eq_l", K::EqMin), ("l_tract_eq_h", K::EqMax)] {
        if let Some(v) = c.v(k, 0, 0) {
            c.local(n, f64::from(v.f32()), c.cell(v.cell));
        }
    }
    // Uninitialised in the original [quirk]; 0 [choice].
    c.set("g_num_treated_patients", 0.0);
    c.set("g_health_capacity", 0.0);
}

/// The minimum funding (×100) and grade of a building list, walked back to
/// front with a strict `<` from `FLT_MAX` (§7.5).
fn list_minima(c: &Ctx<'_>, name: &str, items: &[Item]) {
    let mut min_f = FLT_MAX;
    let mut obj_f = 0u64;
    let mut min_g = FLT_MAX;
    let mut obj_g = 0u64;
    for it in items.iter().rev() {
        if it.f0 < min_f {
            min_f = it.f0;
            obj_f = it.object;
        }
        let g = x100(it.f1);
        if g < min_g {
            min_g = g;
            obj_g = it.object;
        }
    }
    c.local(&format!("l_{name}_funding_l"), pct100(min_f), c.obj(obj_f));
    // [choice] The grade's subject is the building with the lowest grade.
    c.local(&format!("l_{name}_grade_l"), f64::from(min_g), c.obj(obj_g));
}

/// `f32(f32(served / f32(population + 1)) × 100)`.
fn coverage(t: &Val) -> f64 {
    let served = t.n as u32 as f32;
    let pop = (t.i as i32).wrapping_add(1) as f32;
    f64::from(x100(served / pop))
}

/// `count > 0 ? f32(f32(Σgrade / count) × 100) : 0`.
fn grade(sum: f32, count: f32) -> f64 {
    if count > 0.0 {
        f64::from(x100(sum / count))
    } else {
        0.0
    }
}

fn hq_occupants(adv: &Rc<Advisor>) {
    let c = Ctx::new(adv);
    let hospitals = c.list(ListKind::Hospitals, 0, &[groups::HOSPITAL]);
    let clinics = c.list(ListKind::Hospitals, 0, &[groups::CLINIC]);
    let schools = c.list(ListKind::Schools, 0, &groups::ALL_SCHOOLS);
    let museums = c.list(ListKind::Schools, 0, &[groups::MUSEUM]);
    let libraries = c.list(ListKind::Schools, 0, &[groups::LIBRARY]);
    for (n, l) in [
        ("hospital", &hospitals),
        ("clinic", &clinics),
        ("school", &schools),
        ("museum", &museums),
        ("library", &libraries),
    ] {
        if let Some(items) = l {
            let plural = match n {
                "library" => "libraries".to_string(),
                _ => format!("{n}s"),
            };
            c.set(&format!("g_num_{plural}"), items.len() as u32 as f64);
            list_minima(&c, n, items);
        }
    }
    if let Some(t) = c.v(K::HealthTotals, 0, 0) {
        c.set("g_health_coverage_p", coverage(&t));
        if let (Some(h), Some(cl)) = (&hospitals, &clinics) {
            let hcount = (h.len() as u32).wrapping_add(cl.len() as u32) as f32;
            c.set("ga_health_grade", grade(t.f32(), hcount));
        }
    }
    if let Some(t) = c.v(K::SchoolTotals, 0, 0) {
        c.set("ga_school_grade", grade(t.f32(), t.m as u32 as f32));
        c.set("g_education_coverage_p", coverage(&t));
    }
    for (n, g) in [("museum", groups::MUSEUM), ("library", groups::LIBRARY)] {
        if let Some(t) = c.v(K::SchoolTotals, g, 0) {
            c.set(&format!("ga_{n}_grade"), grade(t.f32(), t.m as u32 as f32));
            c.set(&format!("g_num_{n}_served"), f64::from(t.n as u32));
        }
    }
    if let Some(s) = &schools {
        // The all-schools count again [quirk].
        c.set("g_num_educational_buildings", s.len() as u32 as f64);
    }
    for (n, g) in [
        ("g_num_elem_schools", groups::ELEMENTARY),
        ("g_num_high_schools", groups::HIGH),
        ("g_num_colleges", groups::COLLEGE),
    ] {
        if let Some(l) = c.list(ListKind::Schools, 0, &[g]) {
            c.set(n, l.len() as u32 as f64);
        }
    }
}

// ---- Safety (§7.6) ----

fn safety(adv: &Rc<Advisor>) {
    let c = Ctx::new(adv);
    // The fire department is fetched first and discarded.
    c.pct("g_fire_strike_chance", K::FireStrikeChance, 0, 0);
    c.int32("g_population", K::Population, 0);
    // v, x, z zeroed once; the crime hotspot reuses them [quirk].
    let mut hot = (0i64, Some((0.0f32, 0.0f32)));
    if let Some(v) = c.v(K::FlammabilityHotspot, 0, 0) {
        hot = (v.i, v.cell.or(hot.1));
        c.local("l_flammability_h", f64::from(hot.0 as i32), c.cell(hot.1));
    }
    if let Some(v) = c.v(K::AverageCrime, 0, 0) {
        let crime = (v.i as u8) as i32;
        c.set("ga_crime", f64::from(crime));
        c.h.record_history(HISTORY_GA_CRIME, crime);
    }
    // No hotspot (no answer): the flammability values are repeated [quirk].
    if let Some(v) = c.v(K::CrimeHotspot, 0, 0) {
        hot = (v.i, v.cell.or(hot.1));
    }
    c.local("l_crime_h", f64::from(hot.0 as i32), c.cell(hot.1));
    c.int32("g_inmate_count", K::JailPopulation, 0);
    c.int32("g_criminal_count", K::JailedCount, 0);
    c.u32v("g_crime_commited_count", K::MonthlyCrimeCount, 0);
    c.int32("g_arrest_count", K::ArrestCount, 0);
    c.pct("g_police_strike_chance", K::PoliceStrikeChance, 0, 0);
    c.pct("g_school_strike_chance", K::SchoolStrikeChance, 0, 0);
    c.pct("g_health_strike_chance", K::HealthStrikeChance, 0, 0);
}

/// The station minimum (front to back, strict `<` from 100.0, ×100) and the
/// first station without road access (§7.6).
fn stations(c: &Ctx<'_>, funding_name: &str, roads_name: &str, items: &[Item]) {
    let mut min = 100.0f32;
    let mut obj = 0u64;
    for it in items {
        if it.f0 < min {
            min = it.f0;
            obj = it.object;
        }
    }
    c.local(funding_name, pct100(min), c.obj(obj));
    match items
        .iter()
        .find(|it| it.flags & ITEM_LOT_EXISTS != 0 && it.flags & ITEM_ROAD_ACCESS == 0)
    {
        Some(it) => c.local(roads_name, 1.0, c.obj(it.object)),
        None => c.local(roads_name, 0.0, None),
    }
}

fn safety_occupants(adv: &Rc<Advisor>) {
    let c = Ctx::new(adv);
    c.funding("g_fire_funding_p", dept::FIRE);
    c.funding("g_police_funding_p", dept::POLICE);
    c.u32v("g_fire_station_count", K::FireStationCount, 0);
    c.u32v("g_police_station_count", K::PoliceStationCount, 0);
    c.u32v("g_jail_count", K::JailCount, 0);
    c.u8v("ga_flammability", K::AverageFlammability, 0);
    c.float("ga_fire_coverage", K::FireCoverage, 0, 0);
    c.pct("g_fire_coverage_p", K::FireCoveragePercent, 0, 0);
    c.float("ga_police_coverage", K::PoliceCoverage, 0, 0);
    c.pct("g_police_coverage_p", K::PoliceCoveragePercent, 0, 0);
    c.int32("g_jail_capacity", K::JailCapacity, 0);
    if let Some(l) = c.list(ListKind::FireStations, 0, &[]) {
        stations(&c, "l_fire_funding_p", "l_fire_station_no_roads", &l);
    }
    if let Some(l) = c.list(ListKind::PoliceStations, 0, &[]) {
        stations(&c, "l_police_funding_p", "l_police_no_roads", &l);
    }
}

/// The Safety messages (§5.5, §7.6). The strike messages' data words are
/// read as "data1 ≠ 0 = begins" [question Q4].
pub(super) fn safety_message(adv: &Rc<Advisor>, m: &AdvisorMessage) {
    let c = Ctx::new(adv);
    let sys = adv.shared.sys();
    let push = |ev: u32, s: Option<Rc<AdviceSubject>>| {
        if let Some(sys) = &sys {
            sys.push_event(ev, s);
        }
    };
    let pair = |var: &str, on: bool, begin: u32, end: u32| {
        c.set(var, if on { 1.0 } else { 0.0 });
        push(if on { begin } else { end }, None);
    };
    let cell_of = |a: u32, b: u32| c.cell(Some((a as i32 as f32, b as i32 as f32)));
    match m.msg_type {
        msg::HEALTH_STRIKE => pair(
            "g_health_strike",
            m.data1 != 0,
            event::HEALTH_STRIKE_BEGIN,
            event::HEALTH_STRIKE_END,
        ),
        msg::SCHOOL_STRIKE => pair(
            "g_school_strike",
            m.data1 != 0,
            event::SCHOOL_STRIKE_BEGIN,
            event::SCHOOL_STRIKE_END,
        ),
        msg::JAILBREAK_BEGIN | msg::JAILBREAK_END => pair(
            "g_jailbreak",
            m.msg_type == msg::JAILBREAK_BEGIN,
            event::JAILBREAK_BEGIN,
            event::JAILBREAK_END,
        ),
        msg::FIRE_STRIKE_BEGIN | msg::FIRE_STRIKE_END => pair(
            "g_fire_strike",
            m.msg_type == msg::FIRE_STRIKE_BEGIN,
            event::FIRE_STRIKE_BEGIN,
            event::FIRE_STRIKE_END,
        ),
        msg::POLICE_STRIKE_BEGIN | msg::POLICE_STRIKE_END => pair(
            "g_police_strike",
            m.msg_type == msg::POLICE_STRIKE_BEGIN,
            event::POLICE_STRIKE_BEGIN,
            event::POLICE_STRIKE_END,
        ),
        msg::TRANSIT_STRIKE_BEGIN | msg::TRANSIT_STRIKE_END => pair(
            "g_transit_strike",
            m.msg_type == msg::TRANSIT_STRIKE_BEGIN,
            msg::TRANSIT_STRIKE_BEGIN,
            msg::TRANSIT_STRIKE_END,
        ),
        msg::NUCLEAR_EXPLOSION => push(msg::NUCLEAR_EXPLOSION, None),
        msg::ZOO_ESCAPE => push(event::ZOO_ESCAPE, None),
        msg::PIPE_BURST => {
            if !adv.state().user_disaster {
                push(event::PIPE_BURST, cell_of(m.data1, m.data2));
            }
        }
        msg::ROUTE_BLOCKED => {
            if m.data2 > 2 {
                match m.data1 {
                    0 => push(event::ROUTE_BLOCKED_0, None),
                    1 => push(event::ROUTE_BLOCKED_1, None),
                    _ => {}
                }
            }
        }
        msg::DISASTER_STARTED => {
            c.set("g_disasters_in_progress", 1.0);
            // The type is data4 (mysims.md §3.2).
            let t = m.data4;
            let fire = t == disaster::FIRE;
            let riot = t == disaster::RIOT;
            let mut user = !(fire || riot || t == disaster::PIPE_BURST);
            if fire {
                c.set("g_disasters_fire_started", 1.0);
                c.set("g_disasters_fire_ended", 0.0);
                let cause = m
                    .unknown
                    .and_then(|o| {
                        c.h.value(&Src {
                            object: o,
                            ..Src::new(K::DisasterFireCause)
                        })
                    })
                    .map(|v| v.i as i32);
                if let Some(cause) = cause {
                    if matches!(cause, 1 | 3 | 4) {
                        user = true;
                    }
                    for (n, k) in [("arson", 2), ("effects", 4), ("flammability", 5)] {
                        c.set(
                            &format!("g_disasters_fire_cause_{n}"),
                            if cause == k { 1.0 } else { 0.0 },
                        );
                    }
                }
            }
            if riot {
                c.set("g_disasters_riot_started", 1.0);
                c.set("g_disasters_riot_ended", 0.0);
            }
            adv.with_state(|s| {
                s.fire_active |= fire;
                s.riot_active |= riot;
                s.user_disaster = user;
            });
            let ev = if user {
                event::USER_DISASTER_BASE ^ t
            } else {
                event::DISASTER_BASE ^ t
            };
            push(ev, cell_of(m.data2, m.data3));
            if !user {
                push(event::DISASTER_NOT_USER, None);
            }
        }
        msg::DISASTER_ENDED => {
            c.set("g_disasters_in_progress", 0.0);
            c.u32v("g_disasters_damage_building_count", K::DisasterDamageBuildings, 0);
            c.int("g_disasters_damage_cost", K::DisasterDamageCost, 0);
            c.pct(
                "g_disasters_police_dispatch_effectiveness",
                K::DisasterPoliceEffectiveness,
                0,
                0,
            );
            c.pct(
                "g_disasters_fire_dispatch_effectiveness",
                K::DisasterFireEffectiveness,
                0,
                0,
            );
            let (fire, riot) = adv.with_state(|s| {
                let r = (s.fire_active, s.riot_active);
                s.fire_active = false;
                s.riot_active = false;
                s.user_disaster = false;
                r
            });
            if fire {
                c.set("g_disasters_fire_ended", 1.0);
            }
            if riot {
                c.set("g_disasters_riot_ended", 1.0);
            }
            push(event::DISASTER_ENDED, None);
        }
        _ => {}
    }
}

// ---- Traffic (§7.7) ----

/// The congestion networks, `a` 0..7.
pub const NETWORKS: [&str; 8] = [
    "road", "rail", "street", "avenue", "subway", "elevated", "monorail", "bridge",
];
/// The station kinds, `a` 0..4.
pub const STATION_KINDS: [&str; 5] = ["bus", "train", "subway", "elevated", "monorail"];
/// The tile-count networks, `a` 0..9.
pub const TILE_NETWORKS: [&str; 10] = [
    "road",
    "rail",
    "subway",
    "highway",
    "street",
    "avenue",
    "elevated_rail",
    "monorail",
    "dirtroad",
    "groundhighway",
];

fn traffic(adv: &Rc<Advisor>) {
    let c = Ctx::new(adv);
    for (a, n) in NETWORKS.iter().enumerate() {
        let a = a as u32;
        let name = format!("ga_{n}_congestion");
        c.pct(&name, K::CongestionAverage, a, 0);
        if *n == "rail" {
            // Written twice [quirk].
            c.pct(&name, K::CongestionAverage, a, 0);
        }
        c.u32v(&format!("g_{n}_congestion_tile_count"), K::CongestionTiles, a);
        if let Some(v) = c.v(K::CongestionWorst, a, 0) {
            c.local(&format!("l_{n}_congestion_h"), pct100(v.f32()), c.cell(v.cell));
        }
    }
    for (a, n) in STATION_KINDS.iter().enumerate() {
        let a = a as u32;
        let twice = matches!(*n, "elevated" | "monorail");
        for _ in 0..if twice { 2 } else { 1 } {
            c.pct(&format!("ga_{n}_station_utilization_p"), K::StationUtilization, a, 0);
            if let Some(v) = c.v(K::StationUtilizationLow, a, 0) {
                let t = x100(v.f32()).trunc();
                c.local(&format!("l_{n}_station_utilization_pl"), f64::from(t), c.cell(v.cell));
            }
        }
    }
    c.pct("g_road_funding_p", K::TransportFunding, 0, 0);
    c.pct("g_masstransit_funding_p", K::TransportFunding, 1, 0);
    if let Some(v) = c.v(K::TransitStrike, 0, 0) {
        c.set("g_transit_strike", if v.i != 0 { 1.0 } else { 0.0 });
    }
    c.u8v("ga_freight_trip_length", K::FreightTripLength, 0);
    c.pct("ga_airport_efficiency", K::PortEfficiency, 0, 0);
    c.pct("ga_seaport_efficiency", K::PortEfficiency, 1, 0);
    for (a, n) in ["airport", "seaport"].iter().enumerate() {
        if let Some(lots) = c.list(ListKind::PortLots, a as u32, &[]) {
            let name = format!("l_{n}_no_road_access");
            match lots.iter().rev().find(|l| l.flags & ITEM_ROAD_ACCESS == 0) {
                Some(l) => c.local(&name, 1.0, c.cell(Some((l.x, l.z)))),
                None => c.local(&name, 0.0, None),
            }
        }
    }
    // The connection maxima; the avenue pass keeps the road location
    // [quirk].
    let mut loc: Option<(f32, f32)> = None;
    for (t, n) in [(conn::ROAD, "road"), (conn::AVENUE, "avenue")] {
        let Some(conns) = c.list(ListKind::Connections, t, &[]) else {
            continue;
        };
        let mut max = 0i64;
        for it in &conns {
            let cell = (it.x, it.z);
            if let Some(v) = c.v(K::CongestionAt, it.x as i32 as u32, it.z as i32 as u32) {
                if v.i > max {
                    max = v.i;
                    loc = Some(cell);
                }
            }
        }
        c.local(&format!("l_{n}_connection_congestion_h"), max as f64, c.cell(loc));
    }
    c.u32v("g_rail_neighbor_connection_count", K::ConnectionsOfType, conn::RAIL);
}

/// Port size and stage masks (§7.7).
pub const PORT_SIZES_ALL: u32 = 0xE;
pub const PORT_STAGES_ALL: u32 = 0x1F;

fn traffic_occupants(adv: &Rc<Advisor>) {
    let c = Ctx::new(adv);
    for (a, n) in STATION_KINDS.iter().enumerate() {
        // [question] The station count names are inferred.
        c.u32v(&format!("g_{n}_station_count"), K::StationCount, a as u32);
    }
    c.u32v("g_train_freight_depot_count", K::FreightDepotCount, 0);
    c.u32v("g_num_rapidtransit_transitions", K::RapidTransitTransitions, 0);
    c.u32v("g_pothole_count", K::PotholeCount, 0);
    for (a, n) in TILE_NETWORKS.iter().enumerate() {
        c.u32v(&format!("g_{n}_tile_count"), K::NetworkTiles, a as u32);
    }
    let ports = |port: u32, name: &str, sizes: &[(&str, u32)]| {
        let num = |size: u32, stage: u32| c.v(K::NumPorts, port, size | stage << 8).map(|v| v.i as u32 as f64);
        if let Some(v) = num(PORT_SIZES_ALL, PORT_STAGES_ALL) {
            c.set(&format!("g_{name}_count"), v);
        }
        for (s, mask) in sizes {
            if let Some(v) = num(*mask, PORT_STAGES_ALL) {
                c.set(&format!("g_{s}_{name}_count"), v);
            }
        }
        for (i, stage) in [1u32, 2, 4].into_iter().enumerate() {
            if let Some(v) = num(PORT_SIZES_ALL, stage) {
                c.set(&format!("g_{name}_s{}_count", i + 1), v);
            }
        }
    };
    ports(0, "airport", &[("small", 2), ("medium", 4), ("large", 8)]);
    // [question] The seaport size mask is taken as all sizes.
    ports(1, "seaport", &[]);
    c.u32v("g_seaport_volume", K::SeaportVolume, 0);
    c.set("g_car_zot_count", adv.state().zots.len() as f64);
}

// ---- Utility (§7.8) ----

fn utility(adv: &Rc<Advisor>) {
    let c = Ctx::new(adv);
    c.pct("g_power_funding_p", K::PowerFunding, 0, 0);
    c.int32("g_power_production_capacity", K::PowerProduction, 0);
    c.int32("g_power_consumed", K::PowerConsumed, 0);
    c.int32("g_overworked_power_plant_count", K::OverworkedPlants, 0);
    c.int32("g_unpowered_building_count", K::UnpoweredBuildings, 0);
    if let Some(v) = c.v(K::PowerStrike, 0, 0) {
        c.set("g_power_strike", if v.i != 0 { 1.0 } else { 0.0 });
    }
    c.pct("g_power_strike_chance", K::PowerStrikeChance, 0, 0);
    let dbl = |name: &str, k: SrcKind, a: u32| {
        if let Some(v) = c.v(k, a, 0) {
            c.set(name, v.f);
        }
    };
    dbl("g_power_imported", K::RegionImported, 1);
    dbl("g_power_exported", K::RegionExported, 1);
    c.funding("g_water_funding_p", dept::WATER);
    c.int32("g_water_production_capacity", K::WaterProduction, 0);
    c.int32("g_water_consumed", K::WaterConsumed, 0);
    dbl("g_water_imported", K::RegionImported, 2);
    dbl("g_water_exported", K::RegionExported, 2);
    c.funding("g_pollution_funding_p", dept::POLLUTION);
    for (n, k) in [
        ("g_incinerator_capacity_daily", K::IncineratorCapacity),
        ("g_waste_to_energy_capacity_daily", K::WasteToEnergyCapacity),
    ] {
        if let Some(v) = c.v(k, 0, 0) {
            // Monthly × 30, int32 wrapping [quirk].
            c.set(n, f64::from((v.i as i32).wrapping_mul(30) as u32));
        }
    }
    c.pct("g_recycling_center_effect_p", K::RecyclingEffect, 0, 0);
    for (n, k) in [
        ("ga_air_pollution", K::AirPollution),
        ("ga_water_pollution", K::WaterPollution),
        ("ga_garbage_pollution", K::GarbagePollution),
    ] {
        if let Some(v) = c.v(k, 0, 0) {
            c.set(n, f64::from((v.i as i32).max(0)));
        }
    }
    c.int32("g_uncollected_garbage", K::UncollectedGarbage, 0);
    if let Some(v) = c.v(K::LandfillUsage, 0, 0) {
        let used = v.i as u32;
        let cap = v.n as u32;
        c.set("g_landfill_capacity", f64::from(cap));
        c.set("g_available_landfill_capacity", f64::from(cap.wrapping_sub(used)));
    }
    c.int("g_garbage_produced", K::GarbageProduced, 0);
    c.int("g_garbage_recycled", K::GarbageRecycled, 0);
    c.int("g_garbage_to_energy", K::GarbageToEnergy, 0);
    c.int("g_garbage_to_landfill", K::GarbageToLandfill, 0);
    // The slots are swapped on purpose (neighbors.md §8.2).
    dbl("g_garbage_imported", K::RegionExported, 3);
    dbl("g_garbage_exported", K::RegionImported, 3);
    for (n, k) in [
        ("l_air_pollution_h", K::WorstAirPollution),
        ("l_water_pollution_h", K::WorstWaterPollution),
        ("l_uncollected_garbage_h", K::WorstUncollectedGarbage),
    ] {
        if let Some(v) = c.v(k, 0, 0) {
            c.local(n, f64::from((v.i as i32).max(0)), c.cell(v.cell));
        }
    }
}

/// Power plant types by variable suffix (§7.8).
pub const PLANT_TYPES: [(&str, u32); 8] = [
    ("coal", 1),
    ("gas", 3),
    ("oil", 6),
    ("wind", 9),
    ("solar", 7),
    ("nuclear", 5),
    ("waste", 8),
    ("fusion", 2),
];

fn utility_occupants(adv: &Rc<Advisor>) {
    let c = Ctx::new(adv);
    c.int32("g_power_plant_count", K::PowerPlantCount, 0);
    for (n, t) in PLANT_TYPES {
        c.int32(&format!("g_power_plant_count_{n}"), K::PowerPlantsOfType, t);
    }
    c.int32("g_aged_power_plant_count", K::AgedPowerPlants, 0);
    c.int32("g_power_pole_count", K::PowerPoleCount, 0);
    // [question] The funding is ×100, the age an int.
    let extremes = |k: SrcKind, funding: &str, age: &str| {
        if let Some(v) = c.v(k, 0, 0) {
            c.local(funding, pct100(v.f32()), c.obj(v.object));
            c.local(age, f64::from(v.i as i32), c.obj(v.object2));
        }
    };
    extremes(K::PowerPlantExtremes, "l_power_plant_funding_pl", "l_power_plant_age_h");
    c.int32("g_water_source_count", K::WaterSourceCount, 0);
    c.int32("g_water_pipe_count", K::WaterPipeCount, 0);
    c.int32("g_water_distressed_pipe_count", K::DistressedPipes, 0);
    c.int32("g_water_burst_pipe_count", K::BurstPipes, 0);
    c.int32("g_watered_building_count", K::WateredBuildings, 0);
    c.int32("g_unwatered_building_count", K::UnwateredBuildings, 0);
    c.int32("g_water_pollution_pump_shutdown_count", K::PumpShutdowns, 0);
    extremes(
        K::WaterBuildingExtremes,
        "l_water_building_funding_pl",
        "l_water_building_age_h",
    );
    c.int32("g_incinerator_count", K::IncineratorCount, 0);
    c.int32("g_waste_to_energy_building_count", K::WasteToEnergyCount, 0);
    c.int32("g_water_treatment_plant_count", K::WaterTreatmentCount, 0);
    c.int32("g_recycling_center_count", K::RecyclingCenterCount, 0);
}

/// The neighbour-deal variables (§7.8). `g_nd_can_<x>` := 1 when a
/// proposal of that deal type exists, else 0 [question].
pub(super) fn deal_variables(adv: &Rc<Advisor>) {
    let c = Ctx::new(adv);
    for (n, t) in DEAL_TYPES {
        c.u32v(&format!("g_nd_count_{n}"), K::DealCount, t);
    }
    if let Some(all) = c.list(ListKind::Connections, 0, &[]) {
        let has = |f: &dyn Fn(u32) -> bool| if all.iter().any(|i| f(i.kind)) { 1.0 } else { 0.0 };
        c.set("g_nd_connection_present_water", has(&|k| k == conn::PIPE));
        c.set("g_nd_connection_present_power", has(&|k| k == conn::POWER_POLE));
        c.set(
            "g_nd_connection_present_garbage",
            has(&|k| k == conn::RAIL || k == conn::ROAD),
        );
    }
    if let Some(props) = c.list(ListKind::DealProposals, 0, &[]) {
        for n in [
            "buy_water",
            "buy_power",
            "export_garbage",
            "sell_water",
            "sell_power",
            "import_garbage",
        ] {
            let t = DEAL_TYPES.iter().find(|(m, _)| *m == n).map(|x| x.1).unwrap_or(0);
            let any = props.iter().any(|p| p.kind == t);
            c.set(&format!("g_nd_can_{n}"), if any { 1.0 } else { 0.0 });
        }
    }
}
