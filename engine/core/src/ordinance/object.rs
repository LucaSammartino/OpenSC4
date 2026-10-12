// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The standard ordinance (`cSC4OrdinanceSimple`) and its subclasses (§3).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::property::ExemplarHolder;
use crate::props;
use crate::stream::{InStream, OutStream, RefValue, ResKey, SaveObject};
use crate::variant::Variant;

use super::{prop, HostExemplars, Ordinance, OrdinanceHost, CASINO_BUILDING_TYPE, CASINO_INPUT_CONTROL};

/// Record version written (§9.2).
pub const ORDINANCE_RECORD_VERSION: u16 = 4;

/// LegalizeGambling's ID: version 1 records derive `isIncomeOrdinance` from it.
const GAMBLING_ID: u32 = 0xA0D0_7129;

/// The behaviour variants of §3.6. Every other shipped class is `Plain`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrdinanceKind {
    Plain,
    FreeClinics,
    JuniorSportsLeague,
    LandfillGasRecovery,
    LegalizeGambling,
    NeighborhoodWatch,
    ProReadingCampaign,
    TouristPromotion,
    TrashPresort,
}

impl OrdinanceKind {
    /// The 21 standard classes (§3.6), with their behaviour.
    pub const CLASSES: [(u32, OrdinanceKind); 21] = [
        (0x8BD4_FC74, OrdinanceKind::Plain), // cSC4OrdinanceSimple
        (0xA2BF_1DDC, OrdinanceKind::Plain), // CarpoolIncentive
        (0xA2BF_1E43, OrdinanceKind::Plain), // CleanAir
        (0x00D0_723D, OrdinanceKind::Plain), // CommunityCPRTraining
        (0x40D0_7236, OrdinanceKind::FreeClinics),
        (0x815B_4CEF, OrdinanceKind::JuniorSportsLeague),
        (0x22F6_E80C, OrdinanceKind::LandfillGasRecovery),
        (0xA0D0_7129, OrdinanceKind::LegalizeGambling),
        (0x62BF_1DAA, OrdinanceKind::Plain), // MandatorySmokeDetectors
        (0x62BF_1DB9, OrdinanceKind::NeighborhoodWatch),
        (0xE0D0_722E, OrdinanceKind::Plain), // NuclearFreeZone
        (0x22F6_E81B, OrdinanceKind::Plain), // PaperReductionAct
        (0x82B9_999B, OrdinanceKind::Plain), // PowerConservation
        (0xE0D0_7233, OrdinanceKind::ProReadingCampaign),
        (0xA2BF_1DE5, OrdinanceKind::Plain), // ShuttleService
        (0xC2F6_E81F, OrdinanceKind::Plain), // TireRecycling
        (0x62F6_E7CF, OrdinanceKind::TouristPromotion),
        (0x42BF_1E18, OrdinanceKind::TrashPresort),
        (0xC2BF_1E04, OrdinanceKind::Plain), // VehicleEmissionStandard
        (0x02BF_1DFA, OrdinanceKind::Plain), // WaterConservation
        (0xC2BF_1DC5, OrdinanceKind::Plain), // YouthCurfew
    ];

    /// The behaviour of a standard class, or `None` for an unknown class ID.
    pub fn of_class(class_id: u32) -> Option<OrdinanceKind> {
        Self::CLASSES.iter().find(|c| c.0 == class_id).map(|c| c.1)
    }
}

/// Departments and line items the subclasses read (§3.6).
mod budget_ids {
    pub const HEALTH_DEPT: u32 = 0x0918_8F42;
    pub const FREE_CLINICS_LINE: u32 = 0x0311_0000;
    pub const JUNIOR_SPORTS_DEPT: u32 = 0x8A42_60A6;
    pub const EDUCATION_DEPT: u32 = 0x0918_8F4C;
    pub const PRO_READING_LINES: [u32; 2] = [0x0319_0000, 0x0034_A000];
    pub const TOURISM_DEPT: u32 = 0x8A41_6368;
    pub const SANITATION_DEPT: u32 = 0x6A42_A8F5;
}

/// The ordinance's state (§3.1).
#[derive(Clone, Debug, Default)]
pub struct OrdinanceData {
    pub initialized: bool,
    pub key: Option<ResKey>,
    pub id: u32,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub year_first_available: u32,
    pub chance_availability: f32,
    pub enactment_income: i64,
    pub retracment_income: i64,
    pub monthly_constant_income: i64,
    pub monthly_income_factor: f32,
    pub advisor_id: u32,
    pub is_income_ordinance: bool,
    pub available: bool,
    pub on: bool,
    pub enabled: bool,
    pub adjusted: i64,
    pub misc: ExemplarHolder,
}

/// `cSC4OrdinanceSimple` and its subclasses: the class ID selects the
/// overrides of §3.6.
pub struct StandardOrdinance {
    host: Rc<dyn OrdinanceHost>,
    token: Cell<u64>,
    class_id: u32,
    kind: OrdinanceKind,
    d: RefCell<OrdinanceData>,
}

/// `trunc_s64(x)` (Rust casts saturate; NaN gives 0).
fn trunc_s64(x: f32) -> i64 {
    x as i64
}

fn hex_name(id: u32) -> Vec<u8> {
    format!("0x{id:08x}").into_bytes()
}

impl StandardOrdinance {
    /// A new ordinance of a standard class (§3.6), as its class factory
    /// creates it; `None` for a class ID that is not a standard ordinance.
    pub fn new(host: Rc<dyn OrdinanceHost>, class_id: u32, token: u64) -> Option<Rc<StandardOrdinance>> {
        let kind = OrdinanceKind::of_class(class_id)?;
        Some(Rc::new(StandardOrdinance {
            host,
            token: Cell::new(token),
            class_id,
            kind,
            d: RefCell::new(OrdinanceData::default()),
        }))
    }

    pub fn class_id(&self) -> u32 {
        self.class_id
    }

    pub fn kind(&self) -> OrdinanceKind {
        self.kind
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn snapshot(&self) -> OrdinanceData {
        self.d.borrow().clone()
    }

    /// The misc-properties holder (`GetMiscProperties()`).
    pub fn misc_properties(&self) -> ExemplarHolder {
        self.d.borrow().misc.clone()
    }

    /// `SetKey(key)` (concrete class only, §3.5).
    pub fn set_key(&self, key: ResKey) {
        self.d.borrow_mut().key = Some(key);
    }

    pub fn key(&self) -> Option<ResKey> {
        self.d.borrow().key
    }

    /// `FromData(key)` (§3.2).
    pub fn from_data(&self, key: ResKey) -> bool {
        let Some(ex) = self.host.exemplar(key) else {
            return false;
        };
        let mut d = self.d.borrow_mut();
        d.key = Some(key);
        d.misc.parent = Some((key, ex.clone()));
        let h = &*ex;
        if let Some(v) = props::u32_of(h, prop::ID) {
            d.id = v;
        }
        if let Some(v) = props::u32_of(h, prop::YEAR_FIRST_AVAILABLE) {
            d.year_first_available = v;
        }
        if let Some(v) = props::f32_of(h, prop::CHANCE_AVAILABILITY) {
            d.chance_availability = v;
        }
        if let Some(v) = props::i64_of(h, prop::ENACTMENT_INCOME) {
            d.enactment_income = v;
        }
        if let Some(v) = props::i64_of(h, prop::RETRACMENT_INCOME) {
            d.retracment_income = v;
        }
        if let Some(v) = props::i64_of(h, prop::MONTHLY_CONSTANT_INCOME) {
            d.monthly_constant_income = v;
        }
        if let Some(v) = props::f32_of(h, prop::MONTHLY_INCOME_FACTOR) {
            d.monthly_income_factor = v;
        }
        if let Some(v) = props::u32_of(h, prop::ADVISOR_ID) {
            d.advisor_id = v;
        }
        if let Some(v) = props::bool_of(h, prop::IS_INCOME_ORDINANCE) {
            d.is_income_ordinance = v;
        }
        let id = d.id;
        d.name = self.text(h, prop::NAME_KEY, prop::NAME, std::mem::take(&mut d.name), id);
        d.description = self.text(
            h,
            prop::DESCRIPTION_KEY,
            prop::DESCRIPTION,
            std::mem::take(&mut d.description),
            id,
        );
        true
    }

    /// §3.2 steps 4–5: LTEXT, then the string property, then `0x%08x`.
    fn text(
        &self,
        h: &crate::property::PropertyHolder,
        key_prop: u32,
        str_prop: u32,
        current: Vec<u8>,
        id: u32,
    ) -> Vec<u8> {
        let mut text = current;
        if let Some(s) = self.ltext_of(props::u32s(h, key_prop)) {
            text = s;
        }
        if text.is_empty() {
            text = props::string_of(h, str_prop).unwrap_or_else(|| hex_name(id));
        }
        text
    }

    /// The LTEXT string named by a three-word key property whose first word
    /// is non-zero.
    fn ltext_of(&self, words: Option<Vec<u32>>) -> Option<Vec<u8>> {
        let w = words?;
        if w.len() < 3 || w[0] == 0 {
            return None;
        }
        self.host.ltext(ResKey {
            type_id: w[0],
            group: w[1],
            instance: w[2],
        })
    }

    /// The "monthly base" of §3.3 / §3.6 as a float, or `None` when the
    /// result is the constant alone.
    fn monthly_base(&self) -> Option<f32> {
        use budget_ids::*;
        let h = &self.host;
        let dept_total = |dept| h.department_total_expenses(dept).unwrap_or(0);
        let line = |dept, line| h.line_item_full_expenses(dept, line).unwrap_or(0);
        Some(match self.kind {
            OrdinanceKind::LegalizeGambling => return None,
            OrdinanceKind::FreeClinics => line(HEALTH_DEPT, FREE_CLINICS_LINE) as f32,
            OrdinanceKind::JuniorSportsLeague => dept_total(JUNIOR_SPORTS_DEPT) as f32,
            OrdinanceKind::LandfillGasRecovery => h.landfill_capacity() as f32,
            OrdinanceKind::ProReadingCampaign => {
                if h.department_total_expenses(EDUCATION_DEPT).is_none() {
                    0.0
                } else {
                    let [a, b] = PRO_READING_LINES;
                    line(EDUCATION_DEPT, a).wrapping_add(line(EDUCATION_DEPT, b)) as f32
                }
            }
            OrdinanceKind::TouristPromotion => dept_total(TOURISM_DEPT) as f32,
            OrdinanceKind::TrashPresort => {
                if !h.has_budget_simulator() {
                    return None;
                }
                dept_total(SANITATION_DEPT) as f32
            }
            OrdinanceKind::Plain | OrdinanceKind::NeighborhoodWatch => h.residential_population() as f32,
        })
    }

    /// The subclass condition added to the standard test (§3.6).
    fn extra_condition(&self) -> bool {
        let h = &self.host;
        match self.kind {
            OrdinanceKind::JuniorSportsLeague => {
                h.department_total_expenses(budget_ids::JUNIOR_SPORTS_DEPT).unwrap_or(0) != 0
            }
            OrdinanceKind::LandfillGasRecovery => h.landfill_capacity() != 0 && h.power_plant_count(3) > 0,
            OrdinanceKind::NeighborhoodWatch => h.police_station_count() != 0,
            OrdinanceKind::TrashPresort => h.recycling_center_effect() * 100.0 > 0.0,
            _ => true,
        }
    }

    /// LegalizeGambling's `SetOn(false)` side effects (§3.6).
    fn gambling_off(&self) {
        let h = &self.host;
        let lots = h.lots_with_building_type(CASINO_BUILDING_TYPE);
        h.demolish_lots(&lots);
        h.hide_conditional_building(CASINO_BUILDING_TYPE);
        h.remove_view_input_control(CASINO_INPUT_CONTROL);
    }
}

impl Ordinance for StandardOrdinance {
    fn token(&self) -> u64 {
        self.token.get()
    }

    /// `Init()`: sets initialized and enabled, then `FromData(key)`.
    fn init(&self) -> bool {
        let key = {
            let mut d = self.d.borrow_mut();
            if d.initialized {
                return true;
            }
            d.initialized = true;
            d.enabled = true;
            d.key
        };
        if let Some(k) = key {
            self.from_data(k);
        }
        true
    }

    fn shutdown(&self) -> bool {
        let mut d = self.d.borrow_mut();
        if d.initialized {
            d.initialized = false;
            d.enabled = false;
        }
        true
    }

    /// §3.3 / §3.6, in single precision, truncated toward zero.
    fn current_monthly_income(&self) -> i64 {
        let (constant, factor) = {
            let d = self.d.borrow();
            (d.monthly_constant_income, d.monthly_income_factor)
        };
        match self.monthly_base() {
            None => constant,
            Some(base) => trunc_s64(constant as f32 + base * factor),
        }
    }

    fn id(&self) -> u32 {
        self.d.borrow().id
    }
    fn name(&self) -> Vec<u8> {
        self.d.borrow().name.clone()
    }
    fn description(&self) -> Vec<u8> {
        self.d.borrow().description.clone()
    }
    fn year_first_available(&self) -> u32 {
        self.d.borrow().year_first_available
    }
    fn chance_availability(&self) -> f32 {
        self.d.borrow().chance_availability
    }
    fn enactment_income(&self) -> i64 {
        self.d.borrow().enactment_income
    }
    fn retracment_income(&self) -> i64 {
        self.d.borrow().retracment_income
    }
    fn monthly_constant_income(&self) -> i64 {
        self.d.borrow().monthly_constant_income
    }
    fn monthly_income_factor(&self) -> f32 {
        self.d.borrow().monthly_income_factor
    }
    fn advisor_id(&self) -> u32 {
        self.d.borrow().advisor_id
    }
    fn misc_property(&self, id: u32) -> Option<Variant> {
        self.d.borrow().misc.get(id).cloned()
    }
    fn is_available(&self) -> bool {
        self.d.borrow().available
    }
    fn is_on(&self) -> bool {
        let d = self.d.borrow();
        d.available && d.on
    }
    fn is_enabled(&self) -> bool {
        self.d.borrow().enabled
    }
    fn monthly_adjusted_income(&self) -> i64 {
        self.d.borrow().adjusted
    }

    /// §3.4 plus the subclass condition.
    fn check_conditions(&self) -> bool {
        let (enabled, year) = {
            let d = self.d.borrow();
            (d.enabled, d.year_first_available)
        };
        if !enabled {
            return false;
        }
        let Some(now) = self.host.sim_year() else { return false };
        now >= year && self.extra_condition()
    }

    fn is_income_ordinance(&self) -> bool {
        self.d.borrow().is_income_ordinance
    }

    fn simulate(&self) -> bool {
        let v = self.current_monthly_income();
        self.d.borrow_mut().adjusted = v;
        true
    }

    fn set_available(&self, available: bool) -> bool {
        self.force_available(available);
        self.d.borrow_mut().adjusted = 0;
        true
    }

    /// `SetOn(b)` with the preview adjustment of §3.5.
    fn set_on(&self, on: bool) -> bool {
        {
            let mut d = self.d.borrow_mut();
            if on != d.on {
                if on {
                    d.adjusted = trunc_s64((d.enactment_income as f32).abs() + d.adjusted as f32);
                } else {
                    d.adjusted = trunc_s64(d.adjusted as f32 - (d.retracment_income as f32).abs());
                }
                d.on = on;
            }
        }
        if !on && self.kind == OrdinanceKind::LegalizeGambling {
            self.gambling_off();
        }
        true
    }

    fn set_enabled(&self, enabled: bool) -> bool {
        self.d.borrow_mut().enabled = enabled;
        true
    }
    fn force_available(&self, available: bool) -> bool {
        self.d.borrow_mut().available = available;
        true
    }
    fn force_on(&self, on: bool) -> bool {
        self.d.borrow_mut().on = on;
        true
    }
    fn force_monthly_adjusted_income(&self, v: i64) -> bool {
        self.d.borrow_mut().adjusted = v;
        true
    }

    fn save_ref(self: Rc<Self>) -> RefValue {
        RefValue::Object(self)
    }
}

// ---- persistence (§9.2) ----------------------------------------------------------------------------

impl SaveObject for StandardOrdinance {
    fn class_id(&self) -> u32 {
        self.class_id
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        if out.error() != 0 {
            return false;
        }
        let d = self.d.borrow();
        out.put_u16(ORDINANCE_RECORD_VERSION);
        out.put_bool(d.initialized);
        out.put_u32(d.id);
        out.put_str(&d.name);
        out.put_str(&d.description);
        out.put_u32(d.year_first_available);
        out.put_f32(d.chance_availability);
        out.put_i64(d.enactment_income);
        out.put_i64(d.retracment_income);
        out.put_i64(d.monthly_constant_income);
        out.put_f32(d.monthly_income_factor);
        out.put_u32(d.advisor_id);
        out.put_bool(d.available);
        out.put_bool(d.on);
        out.put_bool(d.enabled);
        out.put_i64(d.adjusted);
        out.put_bool(d.is_income_ordinance);
        if !d.misc.write(out) {
            return false;
        }
        out.error() == 0
    }

    /// Any version reads (§9.2); exemplar-derived fields are then refreshed
    /// from the current exemplar when it exists.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.error() != 0 {
            return false;
        }
        let parsed = (|| -> Option<OrdinanceData> {
            let version = inp.get_u16()?;
            let mut n = OrdinanceData {
                initialized: inp.get_bool()?,
                id: inp.get_u32()?,
                name: inp.get_str()?,
                description: inp.get_str()?,
                year_first_available: inp.get_u32()?,
                ..OrdinanceData::default()
            };
            let chance = inp.get_f32()?;
            n.chance_availability = if version < 4 { chance * 0.01 } else { chance };
            n.enactment_income = inp.get_i64()?;
            n.retracment_income = inp.get_i64()?;
            n.monthly_constant_income = inp.get_i64()?;
            n.monthly_income_factor = inp.get_f32()?;
            n.advisor_id = inp.get_u32()?;
            n.available = inp.get_bool()?;
            n.on = inp.get_bool()?;
            n.enabled = inp.get_bool()?;
            n.adjusted = inp.get_i64()?;
            n.is_income_ordinance = if version < 2 {
                n.id == GAMBLING_ID
            } else {
                inp.get_bool()?
            };
            Some(n)
        })();
        let Some(mut n) = parsed else { return false };
        let mut misc = ExemplarHolder::default();
        if !misc.read(inp, &HostExemplars(&*self.host)) {
            return false;
        }
        // LTEXT overrides from the holder (own properties, then the parent).
        let word_list = |id| match misc.get(id) {
            Some(v) => {
                let mut h = crate::property::PropertyHolder::new();
                h.add(id, v.clone());
                props::u32s(&h, id)
            }
            None => None,
        };
        if let Some(s) = self.ltext_of(word_list(prop::NAME_KEY)) {
            if !s.is_empty() {
                n.name = s;
            }
        }
        if let Some(s) = self.ltext_of(word_list(prop::DESCRIPTION_KEY)) {
            if !s.is_empty() {
                n.description = s;
            }
        }
        n.key = misc.parent_key();
        n.misc = misc;
        let key = n.key;
        *self.d.borrow_mut() = n;
        if let Some(k) = key {
            self.from_data(k);
        }
        inp.error() == 0
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
