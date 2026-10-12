// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cISC4OrdinanceSimulator` (§4–§9).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::props;
use crate::stream::{InStream, OutStream, RefValue, SaveObject};
use crate::variant::{mem, Elem, Variant};

use super::{msg, Ordinance, OrdinanceHost, StandardOrdinance, CLSID_ORDINANCE_SIMULATOR, ORDINANCE_EXEMPLAR_TYPE};

/// Record version written and accepted (§9.1).
pub const SIM_RECORD_VERSION: u16 = 4;

#[derive(Default)]
struct SimData {
    initialized: bool,
    loaded_from_save: bool,
    ordinances: BTreeMap<u32, Rc<dyn Ordinance>>,
    monthly_expense: i64,
    monthly_income: i64,
    pending: i64,
}

/// A copy of the observable simulator state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrdinanceSnapshot {
    pub initialized: bool,
    pub loaded_from_save: bool,
    /// Map keys, ascending.
    pub keys: Vec<u32>,
    pub monthly_expense: i64,
    pub monthly_income: i64,
    pub pending: i64,
}

/// The ordinance simulator.
pub struct OrdinanceSim {
    host: Rc<dyn OrdinanceHost>,
    token: Cell<u64>,
    d: RefCell<SimData>,
}

impl OrdinanceSim {
    pub fn new(host: Rc<dyn OrdinanceHost>, token: u64) -> Rc<OrdinanceSim> {
        Rc::new(OrdinanceSim {
            host,
            token: Cell::new(token),
            d: RefCell::new(SimData::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn host(&self) -> &Rc<dyn OrdinanceHost> {
        &self.host
    }

    pub fn snapshot(&self) -> OrdinanceSnapshot {
        let d = self.d.borrow();
        OrdinanceSnapshot {
            initialized: d.initialized,
            loaded_from_save: d.loaded_from_save,
            keys: d.ordinances.keys().copied().collect(),
            monthly_expense: d.monthly_expense,
            monthly_income: d.monthly_income,
            pending: d.pending,
        }
    }

    fn find(&self, id: u32) -> Option<Rc<dyn Ordinance>> {
        self.d.borrow().ordinances.get(&id).cloned()
    }

    fn entries(&self) -> Vec<(u32, Rc<dyn Ordinance>)> {
        self.d
            .borrow()
            .ordinances
            .iter()
            .map(|(k, o)| (*k, o.clone()))
            .collect()
    }

    // ---- lifecycle (§8) ---------------------------------------------------------------------------------

    /// `Init()` (core part: registering the month agent is the shim's).
    pub fn init(&self) -> bool {
        let loaded = {
            let mut d = self.d.borrow_mut();
            if d.initialized {
                return true;
            }
            d.initialized = true;
            d.loaded_from_save
        };
        if loaded {
            self.rekey_by_id();
        } else {
            {
                let mut d = self.d.borrow_mut();
                d.monthly_expense = 0;
                d.monthly_income = 0;
                d.pending = 0;
            }
            self.load_standard_ordinances();
        }
        self.d.borrow_mut().loaded_from_save = false;
        true
    }

    /// `Shutdown()` (core part): the map is kept.
    pub fn shutdown(&self) -> bool {
        self.d.borrow_mut().initialized = false;
        true
    }

    pub fn is_initialized(&self) -> bool {
        self.d.borrow().initialized
    }

    /// §8.1: one ordinance per type-14 exemplar whose instance has a class.
    fn load_standard_ordinances(&self) {
        for key in self.host.exemplar_keys() {
            if key.type_id != props::EXEMPLAR_TYPE {
                continue;
            }
            let Some(ex) = self.host.exemplar(key) else { continue };
            if !props::has_exemplar_type(&ex, ORDINANCE_EXEMPLAR_TYPE) {
                continue;
            }
            let Some(o) = self.host.create_ordinance(key.instance) else {
                continue;
            };
            o.set_key(key);
            o.init();
            let id = o.id();
            let inserted = match self.d.borrow_mut().ordinances.entry(id) {
                Entry::Occupied(_) => false,
                Entry::Vacant(e) => {
                    e.insert(o.clone() as Rc<dyn Ordinance>);
                    true
                }
            };
            if !inserted {
                self.host.release(o.token());
            }
        }
    }

    /// [choice] After a load, entries keyed by class ID move to their
    /// `GetID()` (first wins; ID 0 keeps the class key).
    fn rekey_by_id(&self) {
        let old = std::mem::take(&mut self.d.borrow_mut().ordinances);
        let mut map: BTreeMap<u32, Rc<dyn Ordinance>> = BTreeMap::new();
        let mut leftovers = Vec::new();
        for (k, o) in old {
            let id = o.id();
            let key = if id == 0 { k } else { id };
            match map.entry(key) {
                Entry::Occupied(_) => leftovers.push((k, o)),
                Entry::Vacant(e) => {
                    e.insert(o);
                }
            }
        }
        for (k, o) in leftovers {
            match map.entry(k) {
                Entry::Occupied(_) => self.host.release(o.token()),
                Entry::Vacant(e) => {
                    e.insert(o);
                }
            }
        }
        self.d.borrow_mut().ordinances = map;
    }

    // ---- interface (§4.2) -------------------------------------------------------------------------------

    /// `AddOrdinance(o)`.
    pub fn add_ordinance(&self, o: Rc<dyn Ordinance>) -> bool {
        self.remove_ordinance(&*o);
        let id = o.id();
        self.host.add_ref(o.token());
        let prev = self.d.borrow_mut().ordinances.insert(id, o.clone());
        if let Some(p) = prev {
            self.host.release(p.token());
        }
        if o.is_available() {
            o.set_available(false);
            self.set_ordinance_available(id, true);
        }
        if o.is_on() {
            o.set_on(false);
            self.set_ordinance_on(id, true);
        }
        if !o.is_available() && o.check_conditions() {
            self.set_ordinance_available(id, true);
        }
        true
    }

    /// `RemoveOrdinance(o)`: repeals and withdraws the stored entry with the
    /// same ID, then erases it.
    pub fn remove_ordinance(&self, o: &dyn Ordinance) -> bool {
        let id = o.id();
        let Some(stored) = self.find(id) else { return false };
        if stored.is_on() {
            self.set_ordinance_on(id, false);
        }
        if stored.is_available() {
            self.set_ordinance_available(id, false);
        }
        let removed = self.d.borrow_mut().ordinances.remove(&id);
        if let Some(r) = removed {
            self.host.release(r.token());
        }
        true
    }

    pub fn is_ordinance_available(&self, id: u32) -> bool {
        self.find(id).is_some_and(|o| o.is_available())
    }

    pub fn is_ordinance_on(&self, id: u32) -> bool {
        self.find(id).is_some_and(|o| o.is_on())
    }

    pub fn is_ordinance_available_but_off(&self, id: u32) -> bool {
        self.find(id).is_some_and(|o| o.is_available() && !o.is_on())
    }

    /// `SetOrdinanceAvailable(id, b)` (§4.3).
    pub fn set_ordinance_available(&self, id: u32, available: bool) -> bool {
        let Some(o) = self.find(id) else { return false };
        if o.is_available() != available {
            if !available && o.is_on() {
                self.set_ordinance_on(id, false);
            }
            o.set_available(available);
            self.host
                .post(msg::AVAILABILITY_CHANGED, id, u32::from(o.is_available()));
            if available && o.is_on() {
                o.set_on(false);
                self.set_ordinance_on(id, true);
            }
        }
        true
    }

    /// `SetOrdinanceOn(id, b)` (§4.4).
    pub fn set_ordinance_on(&self, id: u32, on: bool) -> bool {
        let Some(o) = self.find(id) else { return false };
        if o.is_on() != on {
            o.set_on(on);
            let amount = if on {
                o.enactment_income()
            } else {
                o.retracment_income()
            };
            {
                let mut d = self.d.borrow_mut();
                d.pending = d.pending.wrapping_add(amount);
            }
            self.host.post(msg::ORDINANCE_UPDATED, id, u32::from(o.is_on()));
        }
        true
    }

    fn ids_where(&self, capacity: usize, pred: impl Fn(&dyn Ordinance) -> bool) -> (Vec<u32>, u32) {
        let mut out = Vec::new();
        let mut total = 0u32;
        for (k, o) in self.entries() {
            if pred(&*o) {
                total = total.wrapping_add(1);
                if out.len() < capacity {
                    out.push(k);
                }
            }
        }
        (out, total)
    }

    pub fn available_count(&self) -> u32 {
        self.ids_where(0, |o| o.is_available()).1
    }

    /// `GetOrdinanceAvailableArray(out, &n)`: (IDs written, total available).
    pub fn available_array(&self, capacity: usize) -> (Vec<u32>, u32) {
        self.ids_where(capacity, |o| o.is_available())
    }

    pub fn on_count(&self) -> u32 {
        self.ids_where(0, |o| o.is_on()).1
    }

    /// `GetOrdinanceOnArray(out, &n)`: (IDs written, total on).
    pub fn on_array(&self, capacity: usize) -> (Vec<u32>, u32) {
        self.ids_where(capacity, |o| o.is_on())
    }

    pub fn is_ordinance_enabled(&self, id: u32) -> bool {
        self.find(id).is_some_and(|o| o.is_enabled())
    }

    pub fn set_ordinance_enabled(&self, id: u32, enabled: bool) -> bool {
        match self.find(id) {
            Some(o) => {
                o.set_enabled(enabled);
                true
            }
            None => false,
        }
    }

    /// `GetOrdinanceIDArray(out, &n)`: (IDs written, number of entries).
    pub fn id_array(&self, capacity: usize) -> (Vec<u32>, u32) {
        self.ids_where(capacity, |_| true)
    }

    pub fn ordinance_id_by_name(&self, name: &[u8]) -> u32 {
        self.ordinance_by_name(name).map_or(0, |o| o.id())
    }

    /// `GetOrdinanceByID(id)` (no reference added).
    pub fn ordinance_by_id(&self, id: u32) -> Option<Rc<dyn Ordinance>> {
        self.find(id)
    }

    /// `GetOrdinanceByName(name)`: the first in key order, ASCII case ignored.
    pub fn ordinance_by_name(&self, name: &[u8]) -> Option<Rc<dyn Ordinance>> {
        self.entries()
            .into_iter()
            .map(|(_, o)| o)
            .find(|o| o.name().eq_ignore_ascii_case(name))
    }

    pub fn monthly_expense(&self) -> i64 {
        self.d.borrow().monthly_expense
    }

    pub fn monthly_income(&self) -> i64 {
        self.d.borrow().monthly_income
    }

    /// `GetCumulativeFactor(prop, index)` (§6.1), in `f32`.
    pub fn cumulative_factor(&self, prop: u32, index: u32) -> f32 {
        let mut f = 1.0f32;
        for (_, o) in self.entries() {
            if !o.is_on() {
                continue;
            }
            let Some(v) = o.misc_property(prop) else { continue };
            f *= match v {
                Variant::Scalar(Elem::F32(x)) => x,
                Variant::Array { elem, items } if elem == mem::FLOAT32 => match items.get(index as usize) {
                    Some(Elem::F32(x)) => *x,
                    _ => 1.0,
                },
                _ => 1.0,
            };
        }
        f
    }

    /// `GetCumulativeOffset(prop, index)` (§6.1): a 32-bit sum, returned
    /// sign-extended ([choice]).
    pub fn cumulative_offset(&self, prop: u32, index: u32) -> i64 {
        let mut s = 0i32;
        for (_, o) in self.entries() {
            if !o.is_on() {
                continue;
            }
            let Some(v) = o.misc_property(prop) else { continue };
            let add = match v {
                Variant::Scalar(Elem::U32(x)) => x as i32,
                Variant::Scalar(Elem::I32(x)) => x,
                Variant::Array { elem, items } if elem == mem::UINT32 || elem == mem::SINT32 => {
                    match items.get(index as usize) {
                        Some(Elem::U32(x)) => *x as i32,
                        Some(Elem::I32(x)) => *x,
                        _ => 0,
                    }
                }
                _ => 0,
            };
            s = s.wrapping_add(add);
        }
        i64::from(s)
    }

    // ---- monthly behaviour (§5) ----------------------------------------------------------------------

    /// `DoMessage`: SimNewMonth and SimBegin; always true.
    pub fn handle_message(&self, msg_type: u32) -> bool {
        match msg_type {
            msg::SIM_NEW_MONTH => self.do_monthly_simulation(),
            msg::SIM_BEGIN => self.simulation_begin(),
            _ => {}
        }
        true
    }

    fn book(d: &mut SimData, a: i64) {
        if a <= 0 {
            d.monthly_expense = d.monthly_expense.wrapping_sub(a);
        } else {
            d.monthly_income = d.monthly_income.wrapping_add(a);
        }
    }

    fn update_availability(&self, id: u32, o: &dyn Ordinance) {
        if !o.is_available() {
            if o.check_conditions() {
                self.set_ordinance_available(id, true);
            }
        } else if !o.check_conditions() {
            self.set_ordinance_available(id, false);
        }
    }

    /// `DoMonthlySimulation` (§5).
    pub fn do_monthly_simulation(&self) {
        {
            let mut d = self.d.borrow_mut();
            d.monthly_expense = 0;
            d.monthly_income = 0;
            let p = d.pending;
            Self::book(&mut d, p);
            d.pending = 0;
        }
        for (id, o) in self.entries() {
            if o.is_on() && o.simulate() {
                let a = o.monthly_adjusted_income();
                Self::book(&mut self.d.borrow_mut(), a);
            }
            self.update_availability(id, &*o);
        }
        self.host.post(msg::MONTHLY_DONE, 0, 0);
    }

    /// `SimulationBegin` (§5): availability first, then the current adjusted
    /// amounts, without `pending` and without the closing message.
    pub fn simulation_begin(&self) {
        {
            let mut d = self.d.borrow_mut();
            d.monthly_expense = 0;
            d.monthly_income = 0;
        }
        for (id, o) in self.entries() {
            self.update_availability(id, &*o);
            if o.is_on() {
                let a = o.monthly_adjusted_income();
                Self::book(&mut self.d.borrow_mut(), a);
            }
        }
    }
}

impl Drop for OrdinanceSim {
    fn drop(&mut self) {
        for (_, o) in std::mem::take(&mut self.d.get_mut().ordinances) {
            self.host.release(o.token());
        }
    }
}

// ---- persistence (§9.1) --------------------------------------------------------------------------------

impl SaveObject for OrdinanceSim {
    fn class_id(&self) -> u32 {
        CLSID_ORDINANCE_SIMULATOR
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        if out.error() != 0 {
            return false;
        }
        let entries = self.entries();
        let d = self.d.borrow();
        out.put_u16(SIM_RECORD_VERSION);
        out.put_i64(d.monthly_expense);
        out.put_i64(d.monthly_income);
        out.put_u32(entries.len() as u32);
        for (_, o) in entries {
            out.put_ref(&o.save_ref());
        }
        out.put_i64(d.pending);
        out.error() == 0
    }

    fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.error() != 0 {
            return false;
        }
        let Some(version) = inp.get_u16() else { return false };
        if version != SIM_RECORD_VERSION {
            return false;
        }
        let (Some(expense), Some(income), Some(count)) = (inp.get_i64(), inp.get_i64(), inp.get_u32()) else {
            return false;
        };
        let old = std::mem::take(&mut self.d.borrow_mut().ordinances);
        for (_, o) in old {
            self.host.release(o.token());
        }
        for _ in 0..count {
            let Some(r) = inp.get_ref() else { break };
            let (o, class): (Rc<dyn Ordinance>, u32) = match r {
                RefValue::Object(obj) => {
                    let class = obj.class_id();
                    match obj.as_any_rc().downcast::<StandardOrdinance>() {
                        Ok(s) => {
                            self.host.add_ref(s.token()); // the map's reference
                            (s as Rc<dyn Ordinance>, class)
                        }
                        Err(_) => continue,
                    }
                }
                RefValue::Host { token, class_id } => match self.host.foreign_ordinance(token) {
                    Some(o) => (o, class_id),
                    None => {
                        self.host.release(token);
                        continue;
                    }
                },
                RefValue::Null => continue,
            };
            // Keyed by class ID, as the original; Init re-keys by GetID().
            let prev = self.d.borrow_mut().ordinances.insert(class, o);
            if let Some(p) = prev {
                self.host.release(p.token());
            }
        }
        let Some(pending) = inp.get_i64() else { return false };
        let mut d = self.d.borrow_mut();
        d.monthly_expense = expense;
        d.monthly_income = income;
        d.pending = pending;
        d.loaded_from_save = true;
        inp.error() == 0
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ordinance::{prop, CASINO_BUILDING_TYPE, ORDINANCE_EXEMPLAR_GROUP};
    use crate::property::PropertyHolder;
    use crate::props::{exemplar_key, PROP_EXEMPLAR_TYPE};
    use crate::stream::{ByteReader, ByteWriter, RefResolver, ResKey, SaveRegistry};
    use std::rc::Weak;

    const CPR: u32 = 0x00D0_723D;
    const GAMBLING: u32 = 0xA0D0_7129;
    const PRESORT: u32 = 0x42BF_1E18;
    const WATCH: u32 = 0x62BF_1DB9;
    const SMOKE: u32 = 0x62BF_1DAA;

    /// A minimal city for the ordinance simulator.
    #[derive(Default)]
    struct Fake {
        me: RefCell<Weak<Fake>>,
        exemplars: RefCell<Vec<(ResKey, Rc<PropertyHolder>)>>,
        posts: RefCell<Vec<(u32, u32, u32)>>,
        refs: RefCell<BTreeMap<u64, i64>>,
        next: Cell<u64>,
        year: Cell<Option<u32>>,
        population: Cell<i32>,
        no_budget: Cell<bool>,
        police: Cell<u32>,
        lots: RefCell<Vec<(u64, u32)>>,
        demolished: RefCell<Vec<u64>>,
        hidden: Cell<u32>,
    }

    impl Fake {
        fn new() -> Rc<Fake> {
            let f = Rc::new(Fake::default());
            f.year.set(Some(2000));
            *f.me.borrow_mut() = Rc::downgrade(&f);
            f
        }
        fn rc(&self) -> Rc<Fake> {
            self.me.borrow().upgrade().unwrap()
        }
        fn token(&self) -> u64 {
            self.next.set(self.next.get() + 1);
            self.refs.borrow_mut().insert(self.next.get(), 1);
            self.next.get()
        }
        fn make(&self, class: u32) -> Option<Rc<StandardOrdinance>> {
            StandardOrdinance::new(self.rc() as Rc<dyn OrdinanceHost>, class, self.token())
        }
        /// An ordinance exemplar: (enact, repeal, constant, factor) plus
        /// extra properties.
        fn exemplar(&self, instance: u32, money: (i32, i32, i32, f32), extra: &[(u32, Variant)]) {
            let mut h = PropertyHolder::new();
            h.add(PROP_EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(14)));
            h.add(prop::ID, Variant::Scalar(Elem::U32(instance)));
            h.add(prop::ENACTMENT_INCOME, Variant::Scalar(Elem::I32(money.0)));
            h.add(prop::RETRACMENT_INCOME, Variant::Scalar(Elem::I32(money.1)));
            h.add(prop::MONTHLY_CONSTANT_INCOME, Variant::Scalar(Elem::I32(money.2)));
            h.add(prop::MONTHLY_INCOME_FACTOR, Variant::Scalar(Elem::F32(money.3)));
            for (id, v) in extra {
                h.add(*id, v.clone());
            }
            self.exemplars
                .borrow_mut()
                .push((exemplar_key(ORDINANCE_EXEMPLAR_GROUP, instance), Rc::new(h)));
        }
        fn take_posts(&self) -> Vec<(u32, u32, u32)> {
            std::mem::take(&mut *self.posts.borrow_mut())
        }
    }

    impl OrdinanceHost for Fake {
        fn exemplar_keys(&self) -> Vec<ResKey> {
            self.exemplars.borrow().iter().map(|e| e.0).collect()
        }
        fn exemplar(&self, key: ResKey) -> Option<Rc<PropertyHolder>> {
            self.exemplars.borrow().iter().find(|e| e.0 == key).map(|e| e.1.clone())
        }
        fn ltext(&self, _key: ResKey) -> Option<Vec<u8>> {
            None
        }
        fn create_ordinance(&self, class_id: u32) -> Option<Rc<StandardOrdinance>> {
            self.make(class_id)
        }
        fn add_ref(&self, token: u64) {
            *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
        }
        fn release(&self, token: u64) {
            *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
        }
        fn foreign_ordinance(&self, _token: u64) -> Option<Rc<dyn Ordinance>> {
            None
        }
        fn post(&self, msg_type: u32, data2: u32, data3: u32) {
            self.posts.borrow_mut().push((msg_type, data2, data3));
        }
        fn sim_year(&self) -> Option<u32> {
            self.year.get()
        }
        fn residential_population(&self) -> i32 {
            self.population.get()
        }
        fn has_budget_simulator(&self) -> bool {
            !self.no_budget.get()
        }
        fn department_total_expenses(&self, _dept: u32) -> Option<i64> {
            None
        }
        fn line_item_full_expenses(&self, _dept: u32, _line: u32) -> Option<i64> {
            None
        }
        fn landfill_capacity(&self) -> u32 {
            0
        }
        fn power_plant_count(&self, _power_type: i32) -> i32 {
            0
        }
        fn police_station_count(&self) -> u32 {
            self.police.get()
        }
        fn recycling_center_effect(&self) -> f32 {
            0.0
        }
        fn lots_with_building_type(&self, building_type: u32) -> Vec<u64> {
            self.lots
                .borrow()
                .iter()
                .filter(|l| l.1 == building_type)
                .map(|l| l.0)
                .collect()
        }
        fn demolish_lots(&self, lots: &[u64]) {
            self.demolished.borrow_mut().extend_from_slice(lots);
            self.lots.borrow_mut().retain(|l| !lots.contains(&l.0));
        }
        fn hide_conditional_building(&self, building_type: u32) {
            self.hidden.set(building_type);
        }
        fn remove_view_input_control(&self, _control_id: u32) {}
    }

    fn sim(f: &Rc<Fake>) -> Rc<OrdinanceSim> {
        let s = OrdinanceSim::new(f.clone() as Rc<dyn OrdinanceHost>, 1);
        assert!(s.init());
        s
    }

    #[test]
    fn monthly_amount_is_f32_truncated() {
        let f = Fake::new();
        f.exemplar(CPR, (-10, -10, -50, -0.005), &[]);
        f.exemplar(GAMBLING, (0, -20, 100, 1.0), &[]);
        f.exemplar(PRESORT, (-20, -10, -20, -0.003), &[]);
        f.exemplar(SMOKE, (0, 0, 0, -0.001), &[]);
        let s = sim(&f);
        f.population.set(20_000);
        assert_eq!(s.ordinance_by_id(CPR).unwrap().current_monthly_income(), -150);
        assert_eq!(s.ordinance_by_id(GAMBLING).unwrap().current_monthly_income(), 100);
        // A missing sanitation department is a base of 0; no budget simulator: the constant.
        assert_eq!(s.ordinance_by_id(PRESORT).unwrap().current_monthly_income(), -20);
        f.no_budget.set(true);
        assert_eq!(s.ordinance_by_id(PRESORT).unwrap().current_monthly_income(), -20);
        // Truncation toward zero: -1.5 -> -1.
        f.population.set(1_500);
        assert_eq!(s.ordinance_by_id(SMOKE).unwrap().current_monthly_income(), -1);
    }

    #[test]
    fn loading_needs_type_14_and_a_class_and_keeps_the_first_id() {
        let f = Fake::new();
        f.exemplar(CPR, (-10, -10, -50, -0.005), &[]);
        f.exemplar(0x1234_5678, (0, 0, 0, 0.0), &[]); // no class
        f.exemplar(
            0xA2BF_1DDC,
            (0, 0, 0, 0.0),
            &[(PROP_EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(5)))],
        );
        // A second exemplar (other group) with the same instance and ID.
        let dup = f.exemplars.borrow()[0].1.clone();
        f.exemplars.borrow_mut().push((exemplar_key(0x1111_2222, CPR), dup));
        let s = sim(&f);
        assert_eq!(s.snapshot().keys, vec![CPR]);
        let o = s.ordinance_by_id(CPR).unwrap();
        assert!(o.is_enabled() && !o.is_available());
        assert_eq!(o.name(), b"0x00d0723d".to_vec());
        // The duplicate was created and released.
        let released = f.refs.borrow().values().filter(|&&n| n == 0).count();
        assert_eq!(released, 1);
    }

    #[test]
    fn unavailable_enactment_charges_every_call() {
        let f = Fake::new();
        f.exemplar(CPR, (-10, -10, -50, 0.0), &[]);
        let s = sim(&f);
        assert!(s.set_ordinance_on(CPR, true));
        assert!(s.set_ordinance_on(CPR, true));
        assert_eq!(s.snapshot().pending, -20);
        assert_eq!(
            f.take_posts(),
            vec![(msg::ORDINANCE_UPDATED, CPR, 0), (msg::ORDINANCE_UPDATED, CPR, 0)]
        );
        s.do_monthly_simulation();
        assert_eq!(s.monthly_expense(), 20);
        assert_eq!(s.snapshot().pending, -10); // re-enacted when it became available
        assert!(s.is_ordinance_on(CPR));
    }

    #[test]
    fn add_ordinance_enacts_twice_and_remove_repeals() {
        let f = Fake::new();
        f.exemplar(CPR, (-10, -10, -50, 0.0), &[]);
        let s = OrdinanceSim::new(f.clone() as Rc<dyn OrdinanceHost>, 1);
        let o = f.make(CPR).unwrap();
        o.set_key(exemplar_key(ORDINANCE_EXEMPLAR_GROUP, CPR));
        o.init();
        o.force_available(true);
        o.force_on(true);
        let token = o.token();
        assert!(s.add_ordinance(o.clone()));
        assert_eq!(s.snapshot().pending, -20);
        assert_eq!(f.take_posts().len(), 3);
        assert_eq!(f.refs.borrow()[&token], 2);
        assert!(s.remove_ordinance(&*o));
        assert_eq!(s.snapshot().pending, -30);
        assert_eq!(f.refs.borrow()[&token], 1);
        assert!(!s.remove_ordinance(&*o));
    }

    #[test]
    fn month_end_books_and_withdraws() {
        let f = Fake::new();
        f.police.set(1);
        f.exemplar(WATCH, (-10, -10, -10, 0.0), &[]);
        let s = sim(&f);
        s.do_monthly_simulation();
        assert!(s.is_ordinance_available(WATCH));
        s.set_ordinance_on(WATCH, true);
        f.take_posts();
        f.police.set(0);
        s.do_monthly_simulation();
        let snap = s.snapshot();
        assert_eq!((snap.monthly_expense, snap.monthly_income, snap.pending), (20, 0, -10));
        assert_eq!(
            f.take_posts(),
            vec![
                (msg::ORDINANCE_UPDATED, WATCH, 0),
                (msg::AVAILABILITY_CHANGED, WATCH, 0),
                (msg::MONTHLY_DONE, 0, 0)
            ]
        );
    }

    #[test]
    fn gambling_off_demolishes_casinos_every_time() {
        let f = Fake::new();
        f.exemplar(GAMBLING, (0, -20, 100, 1.0), &[]);
        f.lots.borrow_mut().extend([(1, CASINO_BUILDING_TYPE), (2, 7)]);
        let s = sim(&f);
        let g = s.ordinance_by_id(GAMBLING).unwrap();
        g.set_on(false); // already off: the side effects still run
        assert_eq!(*f.demolished.borrow(), vec![1]);
        assert_eq!(f.hidden.get(), CASINO_BUILDING_TYPE);
        assert_eq!(f.lots.borrow().len(), 1);
    }

    #[test]
    fn cumulative_queries() {
        let f = Fake::new();
        f.exemplar(
            CPR,
            (0, 0, 0, 0.0),
            &[
                (0x28ED_0380, Variant::Scalar(Elem::F32(0.95))),
                (0xAA5B_8407, Variant::Scalar(Elem::I32(i32::MAX))),
            ],
        );
        f.exemplar(
            GAMBLING,
            (0, 0, 0, 0.0),
            &[
                (0x28ED_0380, Variant::Scalar(Elem::F32(1.2))),
                (0xAA5B_8407, Variant::Scalar(Elem::U32(1))),
                (0x2A65_4200, Variant::array(mem::FLOAT32, vec![Elem::F32(0.5)])),
            ],
        );
        let s = sim(&f);
        s.do_monthly_simulation();
        assert_eq!(s.cumulative_factor(0x28ED_0380, 0), 1.0);
        s.set_ordinance_on(CPR, true);
        s.set_ordinance_on(GAMBLING, true);
        assert_eq!(s.cumulative_factor(0x28ED_0380, 0), 0.95f32 * 1.2f32);
        assert_eq!(s.cumulative_factor(0x2A65_4200, 0), 0.5);
        assert_eq!(s.cumulative_factor(0x2A65_4200, 1), 1.0);
        assert_eq!(s.cumulative_offset(0xAA5B_8407, 0), i64::from(i32::MIN));
    }

    struct Resolver(Vec<Rc<StandardOrdinance>>);
    impl RefResolver for Resolver {
        fn resolve(&mut self, _class_id: u32, object_id: u32) -> Option<RefValue> {
            self.0
                .get((object_id as usize).wrapping_sub(1))
                .map(|n| RefValue::Object(n.clone() as Rc<dyn SaveObject>))
        }
    }

    #[test]
    fn save_round_trip_and_versions() {
        let f = Fake::new();
        f.exemplar(CPR, (-10, -10, -50, -0.005), &[]);
        f.exemplar(GAMBLING, (0, -20, 100, 1.0), &[]);
        let s = sim(&f);
        s.do_monthly_simulation();
        s.set_ordinance_on(GAMBLING, true);
        s.do_monthly_simulation();
        s.set_ordinance_on(CPR, true);
        let write = |s: &OrdinanceSim| {
            let mut w = ByteWriter::with_registry(Rc::new(RefCell::new(SaveRegistry::new())));
            assert!(s.write(&mut w));
            w.into_bytes()
        };
        let bytes = write(&s);
        assert_eq!(&bytes[..2], &[4, 0]);
        // The ordinances as the store would load them (object IDs 1, 2 in key order).
        let objs: Vec<Rc<StandardOrdinance>> = s
            .snapshot()
            .keys
            .iter()
            .map(|&k| {
                let src = s
                    .ordinance_by_id(k)
                    .unwrap()
                    .save_ref()
                    .object::<StandardOrdinance>()
                    .unwrap();
                let mut w = ByteWriter::new();
                assert!(src.write(&mut w));
                let rec = w.into_bytes();
                let n = f.make(k).unwrap();
                assert!(n.read(&mut ByteReader::new(&rec)));
                let mut again = ByteWriter::new();
                assert!(n.write(&mut again));
                assert_eq!(again.into_bytes(), rec);
                n
            })
            .collect();
        let loaded = OrdinanceSim::new(f.clone() as Rc<dyn OrdinanceHost>, 9);
        let mut res = Resolver(objs);
        assert!(loaded.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
        assert_eq!(write(&loaded), bytes);
        assert!(loaded.init());
        let want = OrdinanceSnapshot {
            initialized: true,
            loaded_from_save: false,
            ..s.snapshot()
        };
        assert_eq!(loaded.snapshot(), want);
        // Versions 3 and 5 are rejected without marking the simulator loaded.
        for v in [3u8, 5] {
            let mut other_bytes = bytes.clone();
            other_bytes[0] = v;
            let other = OrdinanceSim::new(f.clone() as Rc<dyn OrdinanceHost>, 10);
            assert!(!other.read(&mut ByteReader::new(&other_bytes)));
            assert!(!other.snapshot().loaded_from_save);
        }
    }

    #[test]
    fn hostile_inputs_do_not_panic() {
        let f = Fake::new();
        f.exemplar(CPR, (i32::MIN, i32::MAX, i32::MIN, f32::NAN), &[]);
        f.exemplar(GAMBLING, (0, 0, i32::MAX, f32::INFINITY), &[]);
        f.population.set(i32::MAX);
        let s = sim(&f);
        for _ in 0..3 {
            s.do_monthly_simulation();
            for id in [CPR, GAMBLING] {
                s.set_ordinance_on(id, true);
                s.set_ordinance_on(id, false);
                s.set_ordinance_on(id, true);
            }
            s.simulation_begin();
        }
        let o = s.ordinance_by_id(CPR).unwrap();
        o.force_monthly_adjusted_income(i64::MIN);
        o.set_on(false);
        o.set_on(true);
        let _ = s.cumulative_factor(0, u32::MAX);
        let _ = s.cumulative_offset(0, u32::MAX);
        let _ = s.on_array(usize::MAX);
        // Truncated and garbage records.
        let n = f.make(CPR).unwrap();
        for len in 0..40u32 {
            let junk: Vec<u8> = (0..len).map(|i| i.wrapping_mul(37) as u8).collect();
            let _ = n.read(&mut ByteReader::new(&junk));
            let _ = s.read(&mut ByteReader::new(&junk));
        }
    }
}
