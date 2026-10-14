// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4ProtectionSimulator` and its two classes: state (§3), lifecycle
//! (§4), messages and grid updates (§5), strikes and budget hooks (§6),
//! effects (§7), the dispatch source (§8), police jails (§9) and the save
//! records (§11).
//!
//! No `RefCell` borrow is held across a host callback.

use std::any::Any;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::rc::Rc;

use crate::curve::ResponseCurve;
use crate::grid::{add_radius_effect, CellIterator, GridProvider, TractGrid, IID_SIM_GRID};
use crate::rng::CrtRand;
use crate::stream::{InStream, OutStream, RefValue, SaveObject};
use crate::variant::{Elem, Variant};

use super::algo::{self, pct, times_hundred, trunc};
use super::{
    history, msg, prop, BudgetItem, Effect, ObjRef, ProtectionHost, ProtectionKind, ProtectionMessage,
    ProtectionRandom, ProtectionTuning, AGENT_FLAGS, AGENT_LEVELS, AGENT_NAME, AUTOMATA_D, AUTOMATA_FIRE,
    AUTOMATA_POLICE, AUTOMATA_POLICE_DISASTER, DEPARTMENT_GROUP, DEPARTMENT_NAME_GROUP, IID_DEPARTMENT_BUDGET,
    IID_DISPATCH_SOURCE, JAILBREAK_GROUP, JAIL_DEPARTMENT, JAIL_DEPARTMENT_NAME, JAIL_PURPOSE, OCCUPANT_TYPE_BUILDING,
    TRACT_SIZE, TUNING_GROUP,
};

/// The base record version written and the highest accepted (§11).
pub const BASE_RECORD_VERSION: u16 = 3;
/// The fire record version (§11).
pub const FIRE_RECORD_VERSION: u16 = 3;
/// The police record version (§11).
pub const POLICE_RECORD_VERSION: u16 = 8;

/// The coverage-cycle counters (§5.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CycleCounters {
    pub zoned: i32,
    pub sum: i32,
    pub non_zero: i32,
}

/// The two outputs of `GetOccupantCoverage` (§7.3).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CoverageOutputs {
    pub cover_radius: f32,
    pub station_radius: f32,
}

/// The police jail state (§3, §9).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JailState {
    pub jail_department: Option<ObjRef>,
    /// The locally-funded-jail list (refs, saved).
    pub funded_jails: Vec<ObjRef>,
    /// The jail list (refs, saved).
    pub jails: Vec<ObjRef>,
    /// The jailbreak list (refs, not saved).
    pub jailbreaks: Vec<ObjRef>,
    pub capacity: i32,
    pub population: i32,
    pub jailed: i32,
    pub arrests_last: i32,
    pub arrests_this: i32,
    pub unarrested_this: i32,
    pub crimes_last: i32,
    pub crimes_this: i32,
}

/// The simulator's state (§3).
#[derive(Clone, Debug, PartialEq)]
pub struct ProtectionData {
    pub initialized: bool,
    pub ref_count: u32,
    /// Set by the final `Release` (the plugin then drops the object).
    pub destroyed: bool,
    /// Set by `Read` (not otherwise used).
    pub loaded: bool,
    pub department: Option<ObjRef>,
    pub purpose: u32,
    pub stations: Vec<ObjRef>,
    pub iterator: CellIterator,
    pub tracts_per_day: i32,
    pub factor: f32,
    pub counters: CycleCounters,
    pub avg: f32,
    pub pct: f32,
    pub strike_flag: bool,
    pub strike_date: u32,
    pub next_call: u32,
    pub tuning: ProtectionTuning,
    /// The fire plane / police helicopter dispatch source (§8.4).
    pub air_source: Option<ObjRef>,
    /// Police only (§9); stays empty in a fire simulator.
    pub jail: JailState,
}

impl ProtectionData {
    pub fn new(kind: ProtectionKind) -> ProtectionData {
        ProtectionData {
            initialized: false,
            ref_count: 0,
            destroyed: false,
            loaded: false,
            department: None,
            purpose: 0,
            stations: Vec::new(),
            iterator: CellIterator::constructed(),
            tracts_per_day: 0,
            factor: 1.0,
            counters: CycleCounters::default(),
            avg: 0.0,
            pct: 0.0,
            strike_flag: false,
            strike_date: 0,
            next_call: 1,
            tuning: ProtectionTuning::code_defaults(kind),
            air_source: None,
            jail: JailState::default(),
        }
    }
}

/// The first element of an occupant property as a signed integer (floats
/// truncated); `None` when absent or empty.
fn first_i64(v: &Variant) -> Option<i64> {
    v.items().first().map(Elem::as_i64)
}

fn first_f32(v: &Variant) -> Option<f32> {
    v.items().first().map(|e| match *e {
        Elem::F32(x) => x,
        other => other.as_f64() as f32,
    })
}

fn host_ref(o: &Option<ObjRef>) -> RefValue {
    o.map_or(RefValue::Null, |o| RefValue::Host {
        token: o.token,
        class_id: o.class_id,
    })
}

fn as_obj(r: RefValue) -> Option<ObjRef> {
    match r {
        RefValue::Host { token, class_id } => Some(ObjRef { token, class_id }),
        _ => None,
    }
}

/// `curve(x × 100.0f) × 0.01f`, each product one `f32` rounding (§6.2).
fn curve_pct(curve: &ResponseCurve, x: f32) -> f32 {
    pct(curve.eval(times_hundred(x)))
}

/// What a base record read gives (§11).
struct BaseRecord {
    department: Option<ObjRef>,
    iterator: CellIterator,
    adjusted: Option<Rc<dyn TractGrid<u8>>>,
    unadjusted: Option<Rc<dyn TractGrid<i16>>>,
    stations: Vec<ObjRef>,
    counters: CycleCounters,
    avg: f32,
    pct: f32,
    strike_flag: bool,
    strike_date: u32,
    next_call: u32,
}

/// The police fields of the police record (§11).
struct PoliceRecord {
    jail_department: Option<ObjRef>,
    funded_jails: Vec<ObjRef>,
    jails: Vec<ObjRef>,
    values: [i32; 8],
}

/// One fire or police simulator object.
pub struct ProtectionSim {
    host: Rc<dyn ProtectionHost>,
    kind: ProtectionKind,
    token: Cell<u64>,
    d: RefCell<ProtectionData>,
    adjusted: RefCell<Option<Rc<dyn TractGrid<u8>>>>,
    unadjusted: RefCell<Option<Rc<dyn TractGrid<i16>>>>,
    rng: RefCell<Box<dyn ProtectionRandom>>,
    crt: RefCell<CrtRand>,
}

impl ProtectionSim {
    /// A constructed (not initialised) simulator. `rng` is the object's own
    /// `cRZRandom` (§10); the iterator's `rand()` starts at seed 1.
    pub fn new(
        host: Rc<dyn ProtectionHost>,
        kind: ProtectionKind,
        token: u64,
        rng: Box<dyn ProtectionRandom>,
    ) -> Rc<ProtectionSim> {
        Rc::new(ProtectionSim {
            host,
            kind,
            token: Cell::new(token),
            d: RefCell::new(ProtectionData::new(kind)),
            adjusted: RefCell::new(None),
            unadjusted: RefCell::new(None),
            rng: RefCell::new(rng),
            crt: RefCell::new(CrtRand::default()),
        })
    }

    pub fn kind(&self) -> ProtectionKind {
        self.kind
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn data(&self) -> Ref<'_, ProtectionData> {
        self.d.borrow()
    }

    /// Mutable state, for hosts and tests that set up a scenario.
    pub fn data_mut(&self) -> RefMut<'_, ProtectionData> {
        self.d.borrow_mut()
    }

    /// The iterator's C `rand()` state.
    pub fn crt(&self) -> RefMut<'_, CrtRand> {
        self.crt.borrow_mut()
    }

    /// The adjusted (`uint8`) coverage grid: fire slot 5
    /// `GetFireProtectionGrid`, police slot 5 `GetPolicePowerGrid` (`+0x14`,
    /// no `AddRef`).
    pub fn coverage_grid(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.adjusted.borrow().clone()
    }

    /// The crime simulator's police slot 5 (`+0x14`).
    pub fn police_grid(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.coverage_grid()
    }

    /// The unadjusted (`int16`) grid.
    pub fn unadjusted_grid(&self) -> Option<Rc<dyn TractGrid<i16>>> {
        self.unadjusted.borrow().clone()
    }

    /// Replaces the grids (hosts and tests; `Init` creates them when absent).
    pub fn set_grids(&self, adjusted: Option<Rc<dyn TractGrid<u8>>>, unadjusted: Option<Rc<dyn TractGrid<i16>>>) {
        *self.adjusted.borrow_mut() = adjusted;
        *self.unadjusted.borrow_mut() = unadjusted;
    }

    fn is_police(&self) -> bool {
        self.kind == ProtectionKind::Police
    }

    // ---- reference count (§1) ------------------------------------------------------------------------

    /// `AddRef`: the new count.
    pub fn add_ref(&self) -> u32 {
        let mut d = self.d.borrow_mut();
        d.ref_count = d.ref_count.wrapping_add(1);
        d.ref_count
    }

    /// `Release`: with a count ≤ 1 calls `Shutdown`, marks the object
    /// destroyed and returns 0; else decrements and returns the new count.
    pub fn release(&self) -> u32 {
        let count = self.d.borrow().ref_count;
        if count <= 1 {
            self.shutdown();
            let mut d = self.d.borrow_mut();
            d.ref_count = 0;
            d.destroyed = true;
            0
        } else {
            let mut d = self.d.borrow_mut();
            d.ref_count = count - 1;
            d.ref_count
        }
    }

    // ---- lifecycle (§4) ------------------------------------------------------------------------------

    /// `Init` (§4.1); always true.
    pub fn init(&self) -> bool {
        if self.d.borrow().initialized {
            return true;
        }
        self.d.borrow_mut().initialized = true;
        let p = self.kind.params();
        match self.kind {
            ProtectionKind::Fire => {
                self.d.borrow_mut().purpose = p.purpose;
                self.read_tuning();
                self.base_init();
                self.ensure_department(false);
            }
            ProtectionKind::Police => {
                self.read_tuning();
                self.base_init();
                self.d.borrow_mut().purpose = p.purpose;
                self.ensure_department(false);
                self.ensure_department(true);
                self.host.subscribe(&[msg::CRIME]);
            }
        }
        if let Some(g) = self.coverage_grid() {
            self.host.set_adjusted_instance(&g, p.adjusted_instance);
        }
        if let Some(g) = self.unadjusted_grid() {
            self.host.set_unadjusted_instance(&g, p.unadjusted_instance);
        }
        if self.d.borrow().air_source.is_none() {
            let source = self.host.create_air_source(self.kind);
            self.d.borrow_mut().air_source = source;
        }
        let source = self.d.borrow().air_source;
        if let Some(s) = source {
            self.host.air_source_init(s.token);
        }
        true
    }

    fn ensure_department(&self, jail: bool) {
        let present = {
            let d = self.d.borrow();
            if jail {
                d.jail.jail_department.is_some()
            } else {
                d.department.is_some()
            }
        };
        if present {
            return;
        }
        let p = self.kind.params();
        let max = self.d.borrow().tuning.max_funding;
        let (id, name) = if jail {
            (JAIL_DEPARTMENT, JAIL_DEPARTMENT_NAME)
        } else {
            (p.department, p.department_name)
        };
        let dept = self
            .host
            .create_department(id, DEPARTMENT_GROUP, (DEPARTMENT_NAME_GROUP, name), max);
        let mut d = self.d.borrow_mut();
        if jail {
            d.jail.jail_department = dept;
        } else {
            d.department = dept;
        }
    }

    /// Base init (§4.1).
    fn base_init(&self) {
        let p = self.kind.params();
        self.host.acquire_services(self.kind);
        if self.adjusted.borrow().is_none() {
            let g = GridProvider::<u8>::create_grid(&*self.host, p.adjusted_instance, TRACT_SIZE);
            *self.adjusted.borrow_mut() = g;
        }
        if self.unadjusted.borrow().is_none() {
            let g = GridProvider::<i16>::create_grid(&*self.host, p.unadjusted_instance, TRACT_SIZE);
            *self.unadjusted.borrow_mut() = g;
        }
        let (cx, cz) = self.coverage_grid().map_or((0, 0), |g| (g.count_x(), g.count_z()));
        {
            let mut d = self.d.borrow_mut();
            d.iterator
                .set_rect(0, 0, cx.saturating_sub(1).max(0), cz.saturating_sub(1).max(0));
        }
        self.host.add_agent(AGENT_NAME, &AGENT_LEVELS, AGENT_FLAGS);
        self.host.subscribe(&msg::SUBSCRIPTIONS);
        self.host.register_dispatch_source();
        let tracts = i64::from(cx) * i64::from(cz) / i64::from(p.period);
        self.d.borrow_mut().tracts_per_day = tracts.clamp(0, i64::from(i32::MAX)) as i32;
    }

    /// `Shutdown` (§4.2); always true.
    pub fn shutdown(&self) -> bool {
        if !self.d.borrow().initialized {
            return true;
        }
        self.d.borrow_mut().initialized = false;
        let source = self.d.borrow_mut().air_source.take();
        if let Some(s) = source {
            self.host.air_source_shutdown(s.token);
            self.host.release(s.token);
        }
        if self.is_police() {
            self.host.unsubscribe(&[msg::CRIME]);
        }
        self.base_shutdown();
        if self.is_police() {
            let (funded, jails, breaks) = {
                let mut d = self.d.borrow_mut();
                (
                    std::mem::take(&mut d.jail.funded_jails),
                    std::mem::take(&mut d.jail.jails),
                    std::mem::take(&mut d.jail.jailbreaks),
                )
            };
            for o in jails.iter().chain(funded.iter()).chain(breaks.iter()) {
                self.host.release(o.token);
            }
        }
        let (dept, jail_dept) = {
            let mut d = self.d.borrow_mut();
            (d.department.take(), d.jail.jail_department.take())
        };
        for o in dept.iter().chain(jail_dept.iter()) {
            self.host.release(o.token);
        }
        true
    }

    /// The base shutdown (`ProtectionShutdown`, §4.2, traced in full by PR15):
    /// dispatch unregister, the station list, the nine unsubscriptions (in
    /// their own order, [`msg::UNSUBSCRIPTIONS`]), `RemoveAgent`, the adjusted
    /// then the unadjusted grid, and last the held services. The services
    /// (message server, zone manager, budget simulator, city: one host call)
    /// are therefore released **before** the class releases its jail lists
    /// and departments (PR15). The original releases the message server right
    /// after the unsubscriptions; it is the application's service, so moving
    /// that release to step 6 with the others is not observable.
    fn base_shutdown(&self) {
        self.host.unregister_dispatch_source();
        let stations = std::mem::take(&mut self.d.borrow_mut().stations);
        for s in &stations {
            self.host.release(s.token);
        }
        self.host.unsubscribe(&msg::UNSUBSCRIPTIONS);
        self.host.remove_agent();
        let adjusted = self.adjusted.borrow_mut().take();
        if let Some(g) = adjusted {
            self.host.shutdown_adjusted(&g);
        }
        let unadjusted = self.unadjusted.borrow_mut().take();
        if let Some(g) = unadjusted {
            self.host.shutdown_unadjusted(&g);
        }
        self.host.release_services(self.kind);
    }

    pub fn is_initialized(&self) -> bool {
        self.d.borrow().initialized
    }

    /// Reads the tuning (§6.7).
    pub fn read_tuning(&self) {
        let exemplar = self.host.tuning(self.kind);
        let t = ProtectionTuning::read(self.kind, exemplar.as_deref());
        self.d.borrow_mut().tuning = t;
    }

    pub fn tuning(&self) -> ProtectionTuning {
        self.d.borrow().tuning.clone()
    }

    // ---- messages (§5) -------------------------------------------------------------------------------

    /// `DoMessage` (message-target slot 3): always true.
    pub fn handle_message(&self, m: ProtectionMessage) -> bool {
        let p = self.kind.params();
        match m.msg_type {
            msg::SIM_NEW_DAY => self.do_daily(),
            msg::SIM_NEW_MONTH => self.do_monthly(),
            msg::OCCUPANT_INSERTED | msg::OCCUPANT_REMOVED | msg::OCCUPANT_STATE_CHANGED => {
                if let Some(o) = m.occupant {
                    if self.host.occupant_property(o.token, prop::PREVIEW_OCCUPANT).is_none() {
                        match m.msg_type {
                            msg::OCCUPANT_INSERTED => self.occupant_inserted(o),
                            msg::OCCUPANT_REMOVED => self.occupant_removed(o),
                            _ => self.occupant_changed(o),
                        }
                    }
                }
            }
            msg::FUNDING_CHANGED => {
                let (dept, has_stations, jail_dept) = {
                    let d = self.d.borrow();
                    (d.department, !d.stations.is_empty(), d.jail.jail_department)
                };
                if let Some(dept) = dept {
                    if has_stations && self.host.department_id(dept.token) == m.data2 {
                        self.update_unadjusted();
                    }
                }
                if self.is_police() {
                    if let Some(j) = jail_dept {
                        if self.host.department_id(j.token) == m.data2 {
                            self.recalculate();
                        }
                    }
                }
            }
            msg::DISPATCH_ACTIVE | msg::DISPATCH_INACTIVE => {
                if m.data1 == p.dispatch_type {
                    if let Some(info) = self.host.dispatch_info(m.data2) {
                        // §5 (PR14, `DoDispatchActivated`): the source
                        // station's dispatch effect (the record's +0x20, as
                        // UpdateUnadjusted reads it), scaled by its
                        // efficiency (§7.2). The original ignores the AL and
                        // uses a stack leftover as the radius when it fails
                        // [quirk]; [choice] FreeCity4 skips the update then.
                        let (ok, e) = self.dispatch_effect(info.station);
                        if ok {
                            self.update_both_grids_by_position(info.x, info.z, e.radius);
                        }
                    }
                }
            }
            msg::DISPATCH_END => {
                if m.data1 == p.dispatch_type {
                    if let Some(info) = self.host.dispatch_info(m.data2) {
                        self.revoke_dispatch_from_station(info.station, info.call_id);
                    }
                }
            }
            msg::RELOAD_TUNING => {
                if m.data1 == TUNING_GROUP && m.data2 == p.tuning_instance {
                    self.read_tuning();
                }
            }
            msg::CRIME if self.is_police() => self.crime_event(m.unknown),
            _ => {}
        }
        true
    }

    // ---- occupant properties -------------------------------------------------------------------------

    fn prop_s32(&self, o: u64, id: u32) -> Option<i32> {
        self.host
            .occupant_property(o, id)
            .and_then(|v| first_i64(&v))
            .map(|v| v as i32)
    }

    fn prop_u32(&self, o: u64, id: u32) -> Option<u32> {
        self.host
            .occupant_property(o, id)
            .and_then(|v| first_i64(&v))
            .map(|v| v as u32)
    }

    fn prop_f32(&self, o: u64, id: u32) -> Option<f32> {
        self.host.occupant_property(o, id).and_then(|v| first_f32(&v))
    }

    fn has_prop(&self, o: u64, id: u32) -> bool {
        self.host.occupant_property(o, id).is_some()
    }

    // ---- stations (§5.2, §9.4) -----------------------------------------------------------------------

    fn find_station(&self, token: u64) -> Option<usize> {
        self.d.borrow().stations.iter().position(|s| s.token == token)
    }

    /// `DoOccupantInserted` (message-target slot 7).
    pub fn occupant_inserted(&self, o: ObjRef) {
        self.base_inserted(o);
        if !self.is_police() {
            return;
        }
        if self.is_station(o.token) {
            self.host.post(msg::POLICE_STATION_CHANGED, None);
        }
        if self.is_jail(o.token) {
            self.host.add_ref(o.token);
            self.d.borrow_mut().jail.jails.push(o);
            if let Some(dept) = self.item_department(o.token, JAIL_PURPOSE) {
                self.host.add_ref(o.token);
                self.d.borrow_mut().jail.funded_jails.push(o);
                self.host.add_locally_funded(dept, o.token, JAIL_PURPOSE);
            }
            self.recalculate();
        }
    }

    /// The department of the occupant's budget item for `purpose`, when the
    /// item exists and `GetDepartmentBudget(item.departmentID)` is not null
    /// (§5.2 step 4, §6.1, §9.4).
    fn item_department(&self, o: u64, purpose: u32) -> Option<u64> {
        self.host.budget_item(o, purpose).and_then(|i| i.department)
    }

    fn base_inserted(&self, o: ObjRef) {
        if self.host.occupant_type(o.token) != OCCUPANT_TYPE_BUILDING {
            return;
        }
        let (ok, e) = self.station_effect(o.token);
        if !ok {
            return;
        }
        let p = self.kind.params();
        self.host.add_ref(o.token);
        self.d.borrow_mut().stations.push(o);
        // [choice] n = 0 when the count is absent (the original's local is
        // uninitialised).
        let mut n = 0;
        self.dispatches_at_station_into(o.token, None, None, &mut n);
        self.set_station_availability_flags(o.token, algo::unit_mask(n));
        let others: Vec<u64> = {
            let d = self.d.borrow();
            d.stations
                .iter()
                .filter(|s| s.token != o.token)
                .map(|s| s.token)
                .collect()
        };
        // §8.2 step 2: the two locals carry over between stations, so one
        // without the properties re-marks the previous range ([quirk]).
        let (mut base, mut count) = (0u32, 0u32);
        let used: Vec<(u32, u32)> = others
            .into_iter()
            .map(|s| {
                self.dispatches_at_station_into(s, None, Some(&mut base), &mut count);
                (base, count)
            })
            .collect();
        let next_call = self.d.borrow().next_call;
        let base = algo::choose_base_call_number(&used, next_call, n);
        self.set_station_base_call_number(o.token, base);
        {
            let mut d = self.d.borrow_mut();
            d.next_call = d.next_call.max(base.saturating_add(n));
        }
        if let Some(dept) = self.item_department(o.token, p.purpose) {
            self.host.add_locally_funded(dept, o.token, p.purpose);
        }
        self.update_both_grids_by_position(e.x, e.z, e.radius);
    }

    /// `DoOccupantRemoved` (message-target slot 8).
    pub fn occupant_removed(&self, o: ObjRef) {
        if !self.is_police() {
            self.base_removed(o);
            return;
        }
        // §9.4 Remove (`0x006E1570`): only buildings.
        if self.host.occupant_type(o.token) != OCCUPANT_TYPE_BUILDING {
            return;
        }
        // Step 1 (PR8): the first matching entry of the locally-funded-jail
        // list, with no other condition.
        let funded = {
            let mut d = self.d.borrow_mut();
            let i = d.jail.funded_jails.iter().position(|j| j.token == o.token);
            i.map(|i| d.jail.funded_jails.remove(i))
        };
        if let Some(j) = funded {
            self.host.release(j.token);
        }
        // Step 2.
        let jail = {
            let mut d = self.d.borrow_mut();
            let i = d.jail.jails.iter().position(|j| j.token == o.token);
            i.map(|i| d.jail.jails.remove(i))
        };
        if let Some(j) = jail {
            self.host.release(j.token);
            // [choice] skipped when the item's department is missing (the
            // original dereferences null).
            if let Some(dept) = self.item_department(o.token, JAIL_PURPOSE) {
                self.host.remove_locally_funded(dept, o.token, JAIL_PURPOSE);
            }
            self.recalculate();
        }
        // Step 3.
        self.base_removed(o);
        if self.is_station(o.token) {
            self.host.post(msg::POLICE_STATION_CHANGED, None);
        }
    }

    fn base_removed(&self, o: ObjRef) {
        if self.host.occupant_type(o.token) != OCCUPANT_TYPE_BUILDING {
            return;
        }
        let Some(i) = self.find_station(o.token) else { return };
        let s = self.d.borrow_mut().stations.remove(i);
        let p = self.kind.params();
        if let Some(dept) = self.item_department(o.token, p.purpose) {
            self.host.remove_locally_funded(dept, o.token, p.purpose);
        }
        let (_, e) = self.station_effect(o.token);
        self.update_both_grids_by_position(e.x, e.z, e.radius);
        self.host.release(s.token);
    }

    /// `DoOccupantChanged` (message-target slot 9).
    pub fn occupant_changed(&self, o: ObjRef) {
        if self.host.occupant_type(o.token) == OCCUPANT_TYPE_BUILDING && self.find_station(o.token).is_some() {
            let (_, e) = self.station_effect(o.token);
            self.update_both_grids_by_position(e.x, e.z, e.radius);
        }
        if self.is_police() && self.is_station(o.token) {
            self.host.post(msg::POLICE_STATION_CHANGED, None);
        }
    }

    /// Message-target slot 10.
    pub fn set_station_availability_flags(&self, o: u64, flags: u32) {
        self.host.set_occupant_u32(o, self.kind.params().mask_prop, flags);
    }

    /// Message-target slot 15.
    pub fn set_station_base_call_number(&self, o: u64, base: u32) {
        self.host.set_occupant_u32(o, self.kind.params().base_prop, base);
    }

    /// Message-target slot 13.
    pub fn dispatch_type(&self) -> u32 {
        self.kind.params().dispatch_type
    }

    // ---- grids (§5.1–§5.4) ---------------------------------------------------------------------------

    fn cone(g: &dyn TractGrid<i16>, e: &Effect) {
        let geom = g.geometry();
        add_radius_effect::<i16>(
            g,
            geom.count_x,
            geom.count_z,
            trunc(e.x * geom.inv_width_x),
            trunc(e.z * geom.inv_width_z),
            e.radius * geom.inv_width_x,
            e.centre as f32,
            e.outer as f32,
        );
    }

    /// `UpdateUnadjusted` (§5.1).
    pub fn update_unadjusted(&self) {
        let Some(g) = self.unadjusted_grid() else { return };
        g.fill(0);
        let stations = self.d.borrow().stations.clone();
        for s in &stations {
            // [choice] (§5.1): a station whose effect fails is skipped (the
            // original draws the shared record's stale values).
            let (ok, e) = self.station_effect(s.token);
            if ok {
                Self::cone(&*g, &e);
            }
        }
        let t = self.kind.params().dispatch_type;
        let n = self.host.dispatch_count(t);
        let ids = self.host.dispatch_ids(t);
        for &id in ids.iter().take(n as usize) {
            if let Some(info) = self.host.dispatch_info(id) {
                if info.active {
                    // §5.1 / §7.2 (PR2, PR14): the record's +0x20, the source
                    // station; the effect is scaled by its efficiency. [question]
                    // §5.1 does not say what this loop does with a failed
                    // effect: its AL is ignored here and the record, zeroed per
                    // dispatch [choice], then holds centre 0, which adds 0.
                    let (_, mut e) = self.dispatch_effect(info.station);
                    e.x = info.x;
                    e.z = info.z;
                    Self::cone(&*g, &e);
                }
            }
        }
    }

    /// `UpdateBothGridsByPosition` (§5.2).
    pub fn update_both_grids_by_position(&self, x: f32, z: f32, radius: f32) {
        let adjusted = self.coverage_grid();
        let unadjusted = self.unadjusted_grid();
        let (Some(adj), Some(unadj)) = (adjusted, unadjusted) else {
            return;
        };
        let geom = adj.geometry();
        let (tx, tz) = geom.position_to_tract(x, z);
        let r = i64::from(trunc(radius * geom.inv_width_x));
        self.update_unadjusted();
        let factor = self.d.borrow().factor;
        let x0 = (i64::from(tx) - r).max(0);
        let x1 = (i64::from(tx) + r).min(i64::from(geom.count_x) - 1);
        let z0 = (i64::from(tz) - r).max(0);
        let z1 = (i64::from(tz) + r).min(i64::from(geom.count_z) - 1);
        for x in x0..=x1 {
            for z in z0..=z1 {
                let (x, z) = (x as i32, z as i32);
                adj.set(x, z, algo::adjusted_value(unadj.get(x, z), factor));
            }
        }
    }

    /// `DoDaily` (message-target slot 5): factor := 1, then the daily step
    /// (§5.3).
    pub fn do_daily(&self) {
        self.d.borrow_mut().factor = 1.0;
        let adjusted = self.coverage_grid();
        let unadjusted = self.unadjusted_grid();
        let (Some(adj), Some(unadj)) = (adjusted, unadjusted) else {
            return;
        };
        let geom = adj.geometry();
        // §5.3: S is the simulator's own tract-size constant, not the grid's.
        let s = i64::from(TRACT_SIZE);
        let (n, factor) = {
            let d = self.d.borrow();
            (d.tracts_per_day, d.factor)
        };
        // [choice] at most one full pass per day (tracts per day is always
        // smaller unless a host sets it).
        let mut n = i64::from(n).min(geom.len() as i64);
        let mut empty = 0;
        while n > 0 {
            let next = self.d.borrow_mut().iterator.next_cell();
            if let Some((tx, tz)) = next {
                empty = 0;
                let v = algo::adjusted_value(unadj.get(tx, tz), factor);
                adj.set(tx, tz, v);
                if self.tract_zoned(tx, tz, s) {
                    let mut d = self.d.borrow_mut();
                    let c = &mut d.counters;
                    c.zoned = c.zoned.wrapping_add(1);
                    c.sum = c.sum.wrapping_add(i32::from(v));
                    if v != 0 {
                        c.non_zero = c.non_zero.wrapping_add(1);
                    }
                }
                n -= 1;
            } else {
                empty += 1;
                if empty > 1 {
                    break;
                }
                {
                    let mut d = self.d.borrow_mut();
                    let c = d.counters;
                    let (avg, pct) = if c.zoned > 0 {
                        (
                            algo::div_ii(f64::from(c.sum), f64::from(c.zoned)),
                            algo::div_ii(f64::from(c.non_zero), f64::from(c.zoned)),
                        )
                    } else {
                        (0.0, 0.0)
                    };
                    d.avg = avg;
                    d.pct = pct;
                    d.counters = CycleCounters::default();
                }
                self.end_of_cycle();
                let mut crt = self.crt.borrow_mut();
                self.d.borrow_mut().iterator.reset(true, &mut crt);
            }
        }
    }

    /// Whether any zone-grid cell of tract (tx, tz) is zoned (§5.3).
    /// [choice] cells outside the zone grid are skipped: negative ones here,
    /// those past its end by the host (which reads them as 0).
    fn tract_zoned(&self, tx: i32, tz: i32, s: i64) -> bool {
        let (x0, z0) = (i64::from(tx) * s, i64::from(tz) * s);
        let cells = 0..=i64::from(i32::MAX);
        for x in x0..x0 + s {
            for z in z0..z0 + s {
                if !cells.contains(&x) || !cells.contains(&z) {
                    continue;
                }
                if self.host.zone_value(x as i32, z as i32) != 0 {
                    return true;
                }
            }
        }
        false
    }

    /// `EndOfCycle` (§5.4, message-target slot 17).
    pub fn end_of_cycle(&self) {
        let p = self.kind.params();
        let (pct_v, avg) = {
            let d = self.d.borrow();
            (d.pct, d.avg)
        };
        // §5.4: float32 constants 100.0f and 0.5f, one rounding per
        // operation, `trunc` = `_ftol`.
        let pct100 = times_hundred(pct_v);
        self.host.record_history(p.history_pct, trunc(pct100));
        self.host.record_history(p.history_avg, trunc(avg));
        if self.kind == ProtectionKind::Fire {
            let police = self.host.police_percent_coverage().unwrap_or(0.0);
            let police100 = times_hundred(police);
            let safety = (pct100 + police100) * 0.5f32;
            self.host.record_history(history::SAFETY, trunc(safety));
        }
        self.host.post(p.msg_cycle, None);
    }

    /// Message-target slot 16: a no-op in both classes.
    pub fn process_cell(&self, _x: i32, _z: i32) {}

    // ---- formulas (§6) -------------------------------------------------------------------------------

    /// `fundingOf(s, purpose)` (§6.1): no item → 0; the item's department
    /// missing → 1.0; locally funded in that department → the item's local
    /// funding; else the department's funding.
    pub fn funding_of(&self, o: u64, purpose: u32) -> f32 {
        self.host.budget_item(o, purpose).map_or(0.0, |i| self.item_funding(&i))
    }

    fn item_funding(&self, i: &BudgetItem) -> f32 {
        match i.department {
            None => 1.0,
            Some(_) if i.locally_funded => i.local_funding,
            Some(dept) => self.host.department_funding(dept),
        }
    }

    /// `jailFunding(j)` (§9.1, `0x006E06E0`): §6.1 with the jail purpose,
    /// except that no jail item gives 1.0.
    pub fn jail_funding(&self, o: u64) -> f32 {
        self.host
            .budget_item(o, JAIL_PURPOSE)
            .map_or(1.0, |i| self.item_funding(&i))
    }

    /// `GetStationEfficiency(s)` (§6.2; fire slot 10, police slot 25).
    pub fn station_efficiency(&self, o: u64) -> f32 {
        let (curve, no_road, strike) = {
            let d = self.d.borrow();
            (
                d.tuning.funding_curve.clone(),
                d.tuning.no_road_multiplier,
                d.tuning.strike_efficiency,
            )
        };
        let purpose = self.kind.params().purpose;
        let mut e = curve_pct(&curve, self.funding_of(o, purpose));
        if self.kind == ProtectionKind::Fire && self.host.lot_road_access(o) == Some(false) {
            e *= no_road;
        }
        if self.is_station_on_strike(o) {
            e *= strike;
        }
        e
    }

    /// The local funding fraction of §6.3 (1.0 without an item).
    fn strike_funding(&self, o: u64) -> f32 {
        self.host
            .budget_item(o, self.kind.params().purpose)
            .map_or(1.0, |i| i.local_funding)
    }

    /// `IsStationOnStrike(s)` (fire slot 18, police slot 17).
    pub fn is_station_on_strike(&self, o: u64) -> bool {
        self.prop_u32(o, prop::STRIKE_START).is_some_and(|v| v != 0)
    }

    /// `UpdateStrikingState(s)` (§6.3): whether the state changed.
    pub fn update_striking_state(&self, o: u64) -> bool {
        let now = self.host.sim_day();
        let f = self.strike_funding(o);
        let (max_duration, min_end, curve) = {
            let d = self.d.borrow();
            (
                d.tuning.max_strike_duration,
                d.tuning.min_funding_to_end,
                d.tuning.strike_curve.clone(),
            )
        };
        let group = self.kind.params().strike_group;
        match self.prop_u32(o, prop::STRIKE_START).filter(|&v| v != 0) {
            Some(start) => {
                if now.wrapping_sub(start) > max_duration as u32 || f >= min_end {
                    self.host.set_occupant_u32(o, prop::STRIKE_START, 0);
                    self.host.remove_controllers(group, o);
                    true
                } else {
                    false
                }
            }
            None => {
                let input = times_hundred(f);
                let start = match self.kind {
                    ProtectionKind::Fire => {
                        let draw = self.rng.borrow_mut().range_f64(0.0, 100.0);
                        draw < f64::from(curve.eval(input))
                    }
                    ProtectionKind::Police => {
                        let draw = self.rng.borrow_mut().uniform_f64();
                        draw < f64::from(pct(curve.eval(input)))
                    }
                };
                if start {
                    self.host.set_occupant_u32(o, prop::STRIKE_START, now);
                    self.host.add_controllers(group, o);
                }
                start
            }
        }
    }

    /// `GetChanceOfStrike` (§6.4; fire slot 12, police slot 11).
    pub fn chance_of_strike(&self) -> f32 {
        let (flag, stations, curve) = {
            let d = self.d.borrow();
            (d.strike_flag, d.stations.clone(), d.tuning.strike_curve.clone())
        };
        if flag {
            return 1.0;
        }
        let chances: Vec<f32> = stations
            .iter()
            .map(|s| curve_pct(&curve, self.strike_funding(s.token)))
            .collect();
        algo::combine_strike_chance(&chances)
    }

    /// The monthly strike pass (§6.5).
    pub fn strike_pass(&self) {
        let stations = self.d.borrow().stations.clone();
        let mut changed = false;
        let mut striking = 0;
        for s in &stations {
            let c = self.update_striking_state(s.token);
            changed = changed || c;
            if self.is_station_on_strike(s.token) {
                striking += 1;
                if changed {
                    self.begin_strike(s.token);
                }
            }
        }
        if changed {
            if striking == 0 && self.is_on_strike() {
                self.end_strike();
            }
            self.update_unadjusted();
        }
    }

    /// `BeginStrike(s)` (fire slot 13, police slot 12).
    pub fn begin_strike(&self, o: u64) -> bool {
        if self.d.borrow().strike_flag {
            return false;
        }
        let now = self.host.sim_day();
        {
            let mut d = self.d.borrow_mut();
            d.strike_flag = true;
            d.strike_date = now;
        }
        self.host.post(self.kind.params().msg_strike_begin, Some(o));
        true
    }

    /// `EndStrike()` (fire slot 14, police slot 13).
    pub fn end_strike(&self) -> bool {
        if !self.d.borrow().strike_flag {
            return false;
        }
        {
            let mut d = self.d.borrow_mut();
            d.strike_flag = false;
            d.strike_date = 0;
        }
        self.host.post(self.kind.params().msg_strike_end, None);
        true
    }

    /// `IsOnStrike` (fire slot 11, police slot 10).
    pub fn is_on_strike(&self) -> bool {
        self.d.borrow().strike_flag
    }

    /// `GetStrikeStartDate`.
    pub fn strike_start_date(&self) -> u32 {
        self.d.borrow().strike_date
    }

    /// `GetStrikeLength`: now − date, or `0xFFFFFFFF` when date = 0.
    pub fn strike_length(&self) -> u32 {
        let date = self.d.borrow().strike_date;
        if date == 0 {
            u32::MAX
        } else {
            self.host.sim_day().wrapping_sub(date)
        }
    }

    /// `GetStrikingStations(list&)`: appends the striking stations (no
    /// clear; police: nothing unless the city strike flag is set) and
    /// returns the number appended.
    pub fn striking_stations(&self, out: &mut Vec<ObjRef>) -> i32 {
        if self.is_police() && !self.is_on_strike() {
            return 0;
        }
        let stations = self.d.borrow().stations.clone();
        let before = out.len();
        out.extend(stations.into_iter().filter(|s| self.is_station_on_strike(s.token)));
        (out.len() - before) as i32
    }

    /// `DoMonthly` (message-target slot 6; police §9.5).
    pub fn do_monthly(&self) {
        if !self.is_police() {
            self.strike_pass();
            return;
        }
        let rate = self.d.borrow().tuning.rehab_rate;
        let k = {
            let mut d = self.d.borrow_mut();
            let (jailed, k) = algo::rehabilitation(d.jail.jailed, rate);
            d.jail.jailed = jailed;
            k
        };
        if k != 0 {
            self.recalculate();
        }
        self.strike_pass();
        let jails = self.d.borrow().jail.jails.clone();
        for j in &jails {
            for id in [prop::JAIL_CAPACITY, prop::JAIL_POPULATION, prop::JAIL_OVERCAPACITY_COST] {
                let _ = self.host.occupant_property(j.token, id);
            }
        }
        let (breaks, end) = {
            let d = self.d.borrow();
            (d.jail.jailbreaks.clone(), d.tuning.end_jailbreak_capacity)
        };
        for j in &breaks {
            if algo::jailbreak_ends(self.jailed_percentage(), end) {
                self.end_jailbreak(j.token);
            }
        }
        for j in &jails {
            let chance = self.chance_of_jailbreak(j.token);
            if algo::jailbreak_starts(chance, || self.rng.borrow_mut().uniform_f64()) {
                self.begin_jailbreak(*j);
            }
        }
    }

    // ---- budget hooks (§6.6) -------------------------------------------------------------------------

    /// `GetStationFunding(s, cost*, p*)` (fire slot 8, police slot 7):
    /// (low 32 bits of the item cost, local funding), or `None` (false).
    pub fn station_funding(&self, o: u64) -> Option<(i32, f32)> {
        self.host
            .budget_item(o, self.kind.params().purpose)
            .map(|i| (i.cost as i32, i.local_funding))
    }

    /// `SetStationFunding(s, p)` (fire slot 9, police slot 8): the item's
    /// department (`GetDepartmentBudget(item.departmentID)`), not the
    /// simulator's own (§6.6); false without an item or that department.
    pub fn set_station_funding(&self, o: u64, p: f32) -> bool {
        let purpose = self.kind.params().purpose;
        let Some(dept) = self.item_department(o, purpose) else {
            return false;
        };
        self.host.set_local_funding(dept, o, p, purpose);
        true
    }

    /// `ProtectionGetStationFunding(s, float*)` (message-target slot 11).
    pub fn protection_station_funding(&self, o: u64) -> Option<f32> {
        self.host
            .budget_item(o, self.kind.params().purpose)
            .map(|i| i.local_funding)
    }

    /// `GetDepartmentBudget` (slot 6, no `AddRef`).
    pub fn department(&self) -> Option<ObjRef> {
        self.d.borrow().department
    }

    /// `GetJailDepartmentBudget` (police slot 9).
    pub fn jail_department(&self) -> Option<ObjRef> {
        self.d.borrow().jail.jail_department
    }

    // ---- effects and coverage (§7) -------------------------------------------------------------------

    /// `GetStationEffect(s)` on a zeroed record (every FreeCity4 caller):
    /// (AL, effect). An absent police outer or radius therefore reads as 0
    /// ([choice] of §7.1).
    pub fn station_effect(&self, o: u64) -> (bool, Effect) {
        let mut e = Effect::default();
        let ok = self.station_effect_into(o, &mut e);
        (ok, e)
    }

    /// `GetStationEffect(s, record&)` (§7.1, message-target slot 14, PR2) on
    /// the caller's record; the AL result.
    ///
    /// - Fire: centre, outer and radius := 0; all three are read; position
    ///   and original centre are written; fails when the centre or the
    ///   radius is absent, leaving the raw values unscaled.
    /// - Police: centre := 0 and read; a zero or absent centre fails at once
    ///   (every other field keeps the caller's value). Otherwise an absent
    ///   outer or radius keeps the caller's value and the call succeeds.
    pub fn station_effect_into(&self, o: u64, e: &mut Effect) -> bool {
        let p = self.kind.params();
        match self.kind {
            ProtectionKind::Fire => {
                let centre = self.prop_s32(o, p.effect_props[0]);
                let outer = self.prop_s32(o, p.effect_props[1]);
                let radius = self.prop_f32(o, p.effect_props[2]);
                e.centre = centre.unwrap_or(0);
                e.outer = outer.unwrap_or(0);
                e.radius = radius.unwrap_or(0.0);
                self.effect_position(o, e);
                if centre.is_none() || radius.is_none() {
                    return false;
                }
            }
            ProtectionKind::Police => {
                e.centre = self.prop_s32(o, p.effect_props[0]).unwrap_or(0);
                if e.centre == 0 {
                    return false;
                }
                if let Some(v) = self.prop_s32(o, p.effect_props[1]) {
                    e.outer = v;
                }
                if let Some(v) = self.prop_f32(o, p.effect_props[2]) {
                    e.radius = v;
                }
                self.effect_position(o, e);
            }
        }
        let eff = self.effect_efficiency(o);
        *e = algo::scale_effect(*e, eff);
        true
    }

    /// Writes the occupant's position (x, z) and original centre := centre.
    fn effect_position(&self, o: u64, e: &mut Effect) {
        let pos = self.host.occupant_position(o);
        e.x = pos[0];
        e.z = pos[2];
        e.original_centre = e.centre;
    }

    /// The efficiency a station effect is scaled by (§7.1).
    fn effect_efficiency(&self, o: u64) -> f32 {
        if self.host.is_prop_occupant(o) {
            let dept = self.d.borrow().department;
            let funding = dept.map_or(1.0, |d| self.host.department_funding(d.token));
            let curve = self.d.borrow().tuning.funding_curve.clone();
            curve_pct(&curve, funding)
        } else {
            self.station_efficiency(o)
        }
    }

    /// `GetDispatchEffect(station)` (§7.2, message-target slot 12) on a
    /// zeroed record ([choice]: every FreeCity4 caller starts from 0, so an
    /// absent outer or radius reads as 0); see [`Self::dispatch_effect_into`].
    pub fn dispatch_effect(&self, station: u64) -> (bool, Effect) {
        let mut e = Effect::default();
        let ok = self.dispatch_effect_into(station, &mut e);
        (ok, e)
    }

    /// `GetDispatchEffect(station, record&)` (§7.2, message-target slot 12)
    /// on the caller's record; the AL result. The properties are the
    /// dispatch's source station's (PR2; the shipped data has them on the
    /// station exemplars only). *[compat]* (PR14, corrected §7.2):
    ///
    /// 1. x := z := 0 and centre := 0, then the centre is read;
    /// 2. an absent or zero centre fails (AL = 0): outer, radius and the
    ///    original centre keep the caller's values;
    /// 3. outer, then radius: an absent one keeps the caller's value;
    /// 4. original centre := centre;
    /// 5. the effect is **scaled** by `GetStationEfficiency(station)` (§6.2:
    ///    funding, road access and strikes) through `AdjustProtectionEffect`
    ///    (§7.1);
    /// 6. AL = 1.
    pub fn dispatch_effect_into(&self, station: u64, e: &mut Effect) -> bool {
        let p = self.kind.params();
        e.x = 0.0;
        e.z = 0.0;
        e.centre = self.prop_s32(station, p.dispatch_effect_props[0]).unwrap_or(0);
        if e.centre == 0 {
            return false;
        }
        if let Some(v) = self.prop_s32(station, p.dispatch_effect_props[1]) {
            e.outer = v;
        }
        if let Some(v) = self.prop_f32(station, p.dispatch_effect_props[2]) {
            e.radius = v;
        }
        e.original_centre = e.centre;
        let eff = self.station_efficiency(station);
        *e = algo::scale_effect(*e, eff);
        true
    }

    /// `GetOccupantCoverage(s, p, cover&, station&)` (§7.3; fire slot 21,
    /// police slot 20). Fire zeroes both outputs first; police leaves them
    /// untouched when the effect fails.
    pub fn occupant_coverage(&self, o: u64, p: f32, out: &mut CoverageOutputs) -> bool {
        if self.kind == ProtectionKind::Fire {
            *out = CoverageOutputs::default();
        }
        let (ok, e) = self.station_effect(o);
        if !ok {
            return false;
        }
        let (cover, station) = algo::occupant_coverage(&e, p);
        out.cover_radius = cover;
        out.station_radius = station;
        true
    }

    /// `GetStationDispatchRadius(s)`.
    pub fn station_dispatch_radius(&self, o: u64) -> f32 {
        match self.dispatch_effect(o) {
            (true, e) => e.radius,
            _ => 0.0,
        }
    }

    /// `GetAverageCoverage` (ST0).
    pub fn average_coverage(&self) -> f32 {
        self.d.borrow().avg
    }

    /// `GetPercentCoverage` (the value behind the returned pointer).
    pub fn percent_coverage(&self) -> f32 {
        self.d.borrow().pct
    }

    /// `IsFireStation` / `IsPoliceStation`: has the radius property.
    pub fn is_station(&self, o: u64) -> bool {
        o != 0 && self.has_prop(o, self.kind.params().effect_props[2])
    }

    /// `GetFireStations` / `GetPoliceStations` (clears and copies; the
    /// plugin `AddRef`s the copies).
    pub fn stations(&self) -> Vec<ObjRef> {
        self.d.borrow().stations.clone()
    }

    pub fn station_count(&self) -> u32 {
        self.d.borrow().stations.len() as u32
    }

    // ---- dispatch source (§8) ------------------------------------------------------------------------

    /// Dispatch-source slot 3.
    pub fn can_handle_dispatch_type(&self, t: u32) -> bool {
        t == self.kind.params().dispatch_type
    }

    /// `GetDispatchesAtStation(s, flags*, base*, count*)` (slot 7): `None`
    /// when the count, or a requested flags/base, is absent; values not
    /// requested are 0. Flags are masked to `count` units.
    pub fn dispatches_at_station(&self, o: u64, want_flags: bool, want_base: bool) -> Option<(u32, u32, u32)> {
        let (mut flags, mut base, mut count) = (0, 0, 0);
        let ok = self.dispatches_at_station_into(
            o,
            want_flags.then_some(&mut flags),
            want_base.then_some(&mut base),
            &mut count,
        );
        ok.then_some((flags, base, count))
    }

    /// `GetDispatchesAtStation(s, flags*, base*, count*)` (§8.1) on the
    /// caller's outputs: each requested output is written as soon as its
    /// property is read (order count, flags, base); an absent property
    /// leaves its output unchanged and ends the call with false.
    pub fn dispatches_at_station_into(
        &self,
        o: u64,
        flags: Option<&mut u32>,
        base: Option<&mut u32>,
        count: &mut u32,
    ) -> bool {
        let p = self.kind.params();
        let Some(c) = self.prop_u32(o, p.unit_count_prop) else {
            return false;
        };
        *count = c;
        if let Some(f) = flags {
            let Some(v) = self.prop_u32(o, p.mask_prop) else {
                return false;
            };
            *f = v & algo::unit_mask(c);
        }
        if let Some(b) = base {
            let Some(v) = self.prop_u32(o, p.base_prop) else {
                return false;
            };
            *b = v;
        }
        true
    }

    /// `DispatchFromStation(s, id*)` (slot 9): the call ID, or `None` (AL =
    /// 0).
    pub fn dispatch_from_station(&self, o: u64, id: u32) -> Option<u32> {
        let (flags, base, count) = self.dispatches_at_station(o, true, true)?;
        let (mask, call) = algo::dispatch_from_station_mask(flags, base, count, id)?;
        self.set_station_availability_flags(o, mask);
        Some(call)
    }

    /// `RevokeDispatchFromStation(s, id)` (slot 10).
    pub fn revoke_dispatch_from_station(&self, o: u64, id: u32) -> bool {
        let Some((flags, base, count)) = self.dispatches_at_station(o, true, true) else {
            return false;
        };
        match algo::revoke_dispatch_mask(flags, base, count, id) {
            Some(mask) => {
                self.set_station_availability_flags(o, mask);
                true
            }
            None => false,
        }
    }

    /// `DispatchFromNearestStation(x, z, id*)` (slot 8): the chosen station
    /// (EAX) and `*id` (0 unless the dispatch succeeded).
    pub fn dispatch_from_nearest_station(&self, x: f32, z: f32) -> Option<(ObjRef, u32)> {
        let stations = self.d.borrow().stations.clone();
        let mut best: Option<(ObjRef, f32)> = None;
        for s in stations {
            let free = self
                .dispatches_at_station(s.token, true, false)
                .is_some_and(|(f, _, _)| f != 0);
            if !free || self.host.lot_road_access(s.token) == Some(false) {
                continue;
            }
            let pos = self.host.occupant_position(s.token);
            let (dx, dz) = (pos[0] - x, pos[2] - z);
            let dist = (dx * dx + dz * dz).sqrt();
            if best.map_or(true, |(_, b)| dist < b) && !dist.is_nan() {
                best = Some((s, dist));
            }
        }
        let (s, _) = best?;
        Some((s, self.dispatch_from_station(s.token, 0).unwrap_or(0)))
    }

    /// `CanDispatch(x, z, allBusy*)` (slot 4): `None` = false, else
    /// `allBusy`.
    pub fn can_dispatch(&self, x: f32, z: f32) -> Option<bool> {
        let total = self.total_dispatch_count();
        if total <= 0 {
            return None;
        }
        let below = self.host.sea_level() < self.host.altitude(x, z);
        if !below {
            return None;
        }
        let busy = self.host.dispatch_count(self.kind.params().dispatch_type);
        Some(i64::from(busy) >= i64::from(total))
    }

    /// `GetTotalDispatchCount` (fire slot 25, police slot 26).
    pub fn total_dispatch_count(&self) -> i32 {
        let p = self.kind.params();
        let stations = self.d.borrow().stations.clone();
        stations.iter().fold(0i32, |acc, s| {
            acc.wrapping_add(self.prop_u32(s.token, p.unit_count_prop).unwrap_or(0) as i32)
        })
    }

    /// `GetMaxPossibleDispatchCount` (dispatch-source slot 5, §7.3, PR1): one
    /// `count` local starting at 0 and never reset, so a station without the
    /// count property adds the previous station's count ([quirk]); signed
    /// 32-bit sum.
    pub fn max_possible_dispatch_count(&self) -> i32 {
        let stations = self.d.borrow().stations.clone();
        let mut count = 0u32;
        stations.iter().fold(0i32, |total, s| {
            self.dispatches_at_station_into(s.token, None, None, &mut count);
            total.wrapping_add(count as i32)
        })
    }

    /// `GetDispatchAutomataProperties` (slot 11): (`*b`, `*c`, `*d`).
    pub fn dispatch_automata_properties(&self) -> (u32, [f32; 3], u32) {
        let b = match self.kind {
            ProtectionKind::Fire => AUTOMATA_FIRE,
            ProtectionKind::Police if self.host.disaster_ongoing() => AUTOMATA_POLICE_DISASTER,
            ProtectionKind::Police => AUTOMATA_POLICE,
        };
        (b, [0.0; 3], AUTOMATA_D)
    }

    // ---- police jails (§9) ---------------------------------------------------------------------------

    /// `IsJail(occ)` (slot 33).
    pub fn is_jail(&self, o: u64) -> bool {
        self.is_police() && o != 0 && self.has_prop(o, prop::JAIL_CAPACITY)
    }

    /// `GetJailCapacity(occ)` (slot 28); an absent capacity reads as 0.
    pub fn jail_capacity_of(&self, o: u64) -> u32 {
        let curve = self.d.borrow().tuning.jail_curve.clone();
        let eff = curve_pct(&curve, self.jail_funding(o));
        algo::jail_capacity(self.prop_u32(o, prop::JAIL_CAPACITY).unwrap_or(0), eff)
    }

    /// `Recalculate` (§9.1): capacity ≤ 0 → ratio 0; jailed > capacity →
    /// 1; else `f32(jailed / capacity)`.
    pub fn recalculate(&self) {
        let jails = self.d.borrow().jail.jails.clone();
        let caps: Vec<u32> = jails.iter().map(|j| self.jail_capacity_of(j.token)).collect();
        let capacity = caps.iter().fold(0u32, |a, &c| a.wrapping_add(c)) as i32;
        let jailed = self.d.borrow().jail.jailed;
        let ratio = if capacity <= 0 {
            0.0
        } else if jailed > capacity {
            1.0
        } else {
            algo::div_ii(f64::from(jailed), f64::from(capacity))
        };
        let mut population = 0u32;
        for (j, &c) in jails.iter().zip(&caps) {
            let v = algo::ratio_share(c, ratio);
            self.host.set_occupant_u32(j.token, prop::JAIL_POPULATION, v);
            population = population.wrapping_add(v);
        }
        let mut d = self.d.borrow_mut();
        d.jail.capacity = capacity;
        d.jail.population = population as i32;
    }

    /// `GetJailedPercentage()` (slot 36).
    pub fn jailed_percentage(&self) -> f32 {
        let d = self.d.borrow();
        algo::jailed_percentage(d.jail.jailed, d.jail.capacity)
    }

    /// `GetJailCapacity()` (slot 29).
    pub fn jail_capacity(&self) -> u32 {
        self.d.borrow().jail.capacity as u32
    }

    /// `GetJailPopulation` (slot 30, `+0x78`; crime §6.6).
    pub fn jail_population(&self) -> u32 {
        self.d.borrow().jail.population as u32
    }

    /// `GetCriminalCount` (slot 31): the jailed count.
    pub fn criminal_count(&self) -> u32 {
        self.d.borrow().jail.jailed as u32
    }

    /// `GetArrestCount` (slot 34): last month's arrests.
    pub fn arrest_count(&self) -> u32 {
        self.d.borrow().jail.arrests_last as u32
    }

    /// `GetJailCount` (slot 35).
    pub fn jail_count(&self) -> u32 {
        self.d.borrow().jail.jails.len() as u32
    }

    /// `GetJails(list&)` (slot 32).
    pub fn jails(&self) -> Vec<ObjRef> {
        self.d.borrow().jail.jails.clone()
    }

    /// `GetJailbreakBuildings(list&)` (slot 42): 0 without touching the list
    /// when empty, else clears and copies.
    pub fn jailbreak_buildings(&self, out: &mut Vec<ObjRef>) -> i32 {
        let breaks = self.d.borrow().jail.jailbreaks.clone();
        if breaks.is_empty() {
            return 0;
        }
        *out = breaks;
        out.len() as i32
    }

    /// `IsInJailbreak(j)` (slot 39).
    pub fn is_in_jailbreak(&self, o: u64) -> bool {
        self.d.borrow().jail.jailbreaks.iter().any(|j| j.token == o)
    }

    /// `GetChanceOfJailbreak(j)` (§9.5).
    pub fn chance_of_jailbreak(&self, o: u64) -> f32 {
        let (population, curve) = {
            let d = self.d.borrow();
            (d.jail.population, d.tuning.jailbreak_curve.clone())
        };
        if !self.is_jail(o) || population <= 0 || self.is_in_jailbreak(o) {
            return 0.0;
        }
        curve_pct(&curve, self.jailed_percentage())
    }

    /// `BeginJailbreak(j)` (slot 40): always true.
    pub fn begin_jailbreak(&self, j: ObjRef) -> bool {
        self.host.add_ref(j.token);
        self.d.borrow_mut().jail.jailbreaks.push(j);
        let (lo, hi) = self.d.borrow().tuning.escapee_range;
        let r = self.rng.borrow_mut().range_f64(f64::from(lo), f64::from(hi));
        let escaped = algo::escapees(self.jail_capacity_of(j.token), r);
        {
            let mut d = self.d.borrow_mut();
            d.jail.jailed = d.jail.jailed.wrapping_sub(escaped).max(0);
        }
        self.recalculate();
        self.host.post(msg::JAILBREAK_BEGIN, Some(j.token));
        self.host.add_controllers(JAILBREAK_GROUP, j.token);
        true
    }

    /// `EndJailbreak(j)` (slot 41): AL = 0 always.
    pub fn end_jailbreak(&self, o: u64) -> bool {
        if !self.is_jail(o) {
            return false;
        }
        let Some(i) = self.d.borrow().jail.jailbreaks.iter().position(|j| j.token == o) else {
            return false;
        };
        self.host.remove_controllers(JAILBREAK_GROUP, o);
        self.host.post(msg::JAILBREAK_END, Some(o));
        let j = self.d.borrow_mut().jail.jailbreaks.remove(i);
        self.host.release(j.token);
        false
    }

    /// `GetJailCurrentUpkeepCost(j)` (slot 37):
    /// `trunc(float(cost) × localFunding)` as int64 (`_ftol`: a NaN or
    /// out-of-range product gives `i64::MIN`); 0 without an item.
    pub fn jail_upkeep_cost(&self, o: u64) -> i64 {
        let Some(item) = self.host.budget_item(o, JAIL_PURPOSE) else {
            return 0;
        };
        let v = ((item.cost as f64) * f64::from(item.local_funding)) as f32;
        crate::x87::ftol(f64::from(v))
    }

    /// The crime message (§9.2).
    pub fn crime_event(&self, unknown: Option<u64>) {
        let Some(u) = unknown else { return };
        let Some((x, z)) = self.host.lot_location(u) else {
            return;
        };
        let v = self.coverage_grid().map_or(0, |g| g.get_cell_value(x, z));
        let (px, pz) = self.host.cell_center_to_position(x, z);
        let stations = self.d.borrow().stations.clone();
        let mut best: Option<(u64, f32)> = None;
        for s in &stations {
            let pos = self.host.occupant_position(s.token);
            let (dx, dz) = (pos[0] - px, pos[2] - pz);
            let dist = (dx * dx + dz * dz).sqrt();
            if !dist.is_nan() && best.map_or(true, |(_, b)| dist < b) {
                best = Some((s.token, dist));
            }
        }
        let threshold = self.d.borrow().tuning.arrest_threshold;
        let arrest = i32::from(v) > threshold;
        {
            let mut d = self.d.borrow_mut();
            if arrest {
                d.jail.arrests_this = d.jail.arrests_this.wrapping_add(1);
            } else {
                d.jail.unarrested_this = d.jail.unarrested_this.wrapping_add(1);
            }
            d.jail.crimes_this = d.jail.crimes_this.wrapping_add(1);
        }
        if let Some((s, _)) = best {
            let mut stats = self.station_stats(s);
            if arrest {
                stats[3] = stats[3].wrapping_add(1);
            }
            stats[2] = stats[2].wrapping_add(1);
            self.host.set_occupant_u32s(s, prop::STATION_STATS, &stats);
        }
    }

    /// The station statistics vector (§9.2, PR12): four zeros when absent; a
    /// vector that is read keeps its length and extra elements, except that
    /// a shorter one is padded with zeros to four ([choice]). Always at
    /// least four elements.
    pub fn station_stats(&self, o: u64) -> Vec<u32> {
        let mut stats: Vec<u32> = self
            .host
            .occupant_property(o, prop::STATION_STATS)
            .map_or_else(Vec::new, |v| v.items().iter().map(|e| e.as_i64() as u32).collect());
        if stats.len() < 4 {
            stats.resize(4, 0);
        }
        stats
    }

    /// `GetPoliceStationArrestCount(s)` (slot 43): element 1.
    pub fn station_arrest_count(&self, o: u64) -> u32 {
        self.station_stats(o)[1]
    }

    /// `GetPoliceStationCrimeCount(s)` (slot 44): element 0.
    pub fn station_crime_count(&self, o: u64) -> u32 {
        self.station_stats(o)[0]
    }

    /// `ProcessMonthlyCrimeStats()` (slot 38, `+0x98`; crime §6.4).
    /// [choice] returns true.
    pub fn process_monthly_crime_stats(&self) -> bool {
        let arrests = self.d.borrow().jail.arrests_this;
        if arrests > 0 {
            {
                let mut d = self.d.borrow_mut();
                d.jail.jailed = d.jail.jailed.wrapping_add(arrests).max(0);
            }
            self.recalculate();
        }
        let (crimes, arrests, stations) = {
            let mut d = self.d.borrow_mut();
            d.jail.arrests_last = d.jail.arrests_this;
            d.jail.arrests_this = 0;
            d.jail.crimes_last = d.jail.crimes_this;
            d.jail.crimes_this = 0;
            (d.jail.crimes_last, d.jail.arrests_last, d.stations.clone())
        };
        for s in &stations {
            // §9.2 monthly shift: [1] := [3], [0] := [2], [3] := [2] := 0;
            // extra elements are written back unchanged.
            let mut v = self.station_stats(s.token);
            v[1] = v[3];
            v[0] = v[2];
            v[3] = 0;
            v[2] = 0;
            self.host.set_occupant_u32s(s.token, prop::STATION_STATS, &v);
        }
        self.host.record_history(history::CRIMES, crimes);
        self.host.record_history(history::ARRESTS, arrests);
        true
    }

    /// `ResetUnjailedCriminals()` (slot 27, `+0x6C`; §9.3): the released
    /// count.
    pub fn reset_unjailed_criminals(&self) -> i32 {
        let mut d = self.d.borrow_mut();
        let overflow = d.tuning.overflow_factor;
        let (released, jailed) = algo::reset_unjailed(d.jail.capacity, d.jail.jailed, d.jail.unarrested_this, overflow);
        d.jail.unarrested_this = 0;
        d.jail.jailed = jailed;
        released
    }

    // ---- persistence (§11) ---------------------------------------------------------------------------

    fn write_base(&self, out: &mut dyn OutStream) -> bool {
        let d = self.d.borrow();
        out.put_u16(BASE_RECORD_VERSION);
        out.put_ref(&host_ref(&d.department));
        d.iterator.write(out);
        let adj = self.adjusted.borrow().as_ref().map_or(RefValue::Null, |g| g.save_ref());
        let unadj = self
            .unadjusted
            .borrow()
            .as_ref()
            .map_or(RefValue::Null, |g| g.save_ref());
        out.put_ref(&adj);
        out.put_ref(&unadj);
        write_list(out, &d.stations);
        out.put_i32(d.counters.zoned);
        out.put_i32(d.counters.sum);
        out.put_i32(d.counters.non_zero);
        out.put_f32(d.avg);
        out.put_f32(d.pct);
        out.put_u8(u8::from(d.strike_flag));
        out.put_u32(d.strike_date);
        out.put_u32(d.next_call);
        out.error() == 0
    }

    /// Reads a list of references (u32 count + refs); host references are
    /// collected into `list` (owned by the caller on failure).
    fn read_list(inp: &mut dyn InStream, list: &mut Vec<ObjRef>) -> Option<()> {
        let n = inp.get_u32()?;
        for _ in 0..n {
            if let Some(o) = as_obj(inp.get_ref()?) {
                list.push(o);
            }
        }
        Some(())
    }

    fn read_base_record(&self, inp: &mut dyn InStream, owned: &mut Vec<ObjRef>) -> Option<BaseRecord> {
        let version = inp.get_u16()?;
        if version > BASE_RECORD_VERSION {
            return None;
        }
        let department = as_obj(inp.get_ref_as(IID_DEPARTMENT_BUDGET)?);
        if let Some(d) = department {
            owned.push(d);
        }
        let iterator = CellIterator::read(inp)?;
        let adj = inp.get_ref_as(IID_SIM_GRID)?;
        let adjusted = if adj.is_null() {
            None
        } else {
            GridProvider::<u8>::resolve_grid(&*self.host, &adj)
        };
        let unadj = inp.get_ref_as(IID_SIM_GRID)?;
        let unadjusted = if unadj.is_null() {
            None
        } else {
            GridProvider::<i16>::resolve_grid(&*self.host, &unadj)
        };
        let start = owned.len();
        Self::read_list(inp, owned)?;
        let stations = owned[start..].to_vec();
        let counters = CycleCounters {
            zoned: inp.get_i32()?,
            sum: inp.get_i32()?,
            non_zero: inp.get_i32()?,
        };
        let avg = inp.get_f32()?;
        let mut pct_v = inp.get_f32()?;
        let strike_flag = inp.get_u8()? != 0;
        let strike_date = inp.get_u32()?;
        let next_call = inp.get_u32()?;
        if inp.error() != 0 {
            return None;
        }
        if version < 3 {
            pct_v = pct(pct_v);
        }
        Some(BaseRecord {
            department,
            iterator,
            adjusted,
            unadjusted,
            stations,
            counters,
            avg,
            pct: pct_v,
            strike_flag,
            strike_date,
            next_call,
        })
    }

    /// The base `Read` (§11): commits only a complete record.
    fn read_base(&self, inp: &mut dyn InStream) -> bool {
        let mut owned = Vec::new();
        let Some(r) = self.read_base_record(inp, &mut owned) else {
            for o in owned {
                self.host.release(o.token);
            }
            return false;
        };
        let (old_dept, old_stations) = {
            let mut d = self.d.borrow_mut();
            let old = (d.department.take(), std::mem::take(&mut d.stations));
            d.department = r.department;
            d.iterator = r.iterator;
            d.stations = r.stations;
            d.counters = r.counters;
            d.avg = r.avg;
            d.pct = r.pct;
            d.strike_flag = r.strike_flag;
            d.strike_date = r.strike_date;
            d.next_call = r.next_call;
            old
        };
        for o in old_dept.iter().chain(old_stations.iter()) {
            self.host.release(o.token);
        }
        let old_adj = std::mem::replace(&mut *self.adjusted.borrow_mut(), r.adjusted.clone());
        if let Some(g) = old_adj {
            if !r.adjusted.as_ref().is_some_and(|n| Rc::ptr_eq(n, &g)) {
                self.host.shutdown_adjusted(&g);
            }
        }
        let old_unadj = std::mem::replace(&mut *self.unadjusted.borrow_mut(), r.unadjusted.clone());
        if let Some(g) = old_unadj {
            if !r.unadjusted.as_ref().is_some_and(|n| Rc::ptr_eq(n, &g)) {
                self.host.shutdown_unadjusted(&g);
            }
        }
        inp.error() == 0
    }

    fn read_police_record(&self, inp: &mut dyn InStream, owned: &mut Vec<ObjRef>) -> Option<PoliceRecord> {
        let jail_department = as_obj(inp.get_ref_as(IID_DEPARTMENT_BUDGET)?);
        if let Some(d) = jail_department {
            owned.push(d);
        }
        let start = owned.len();
        Self::read_list(inp, owned)?;
        let funded_jails = owned[start..].to_vec();
        let start = owned.len();
        Self::read_list(inp, owned)?;
        let jails = owned[start..].to_vec();
        let mut values = [0i32; 8];
        for v in &mut values {
            *v = inp.get_i32()?;
        }
        if inp.error() != 0 {
            return None;
        }
        Some(PoliceRecord {
            jail_department,
            funded_jails,
            jails,
            values,
        })
    }

    fn read_police(&self, inp: &mut dyn InStream, version: u16) {
        let mut owned = Vec::new();
        let Some(r) = self.read_police_record(inp, &mut owned) else {
            for o in owned {
                self.host.release(o.token);
            }
            return;
        };
        let mut released = Vec::new();
        {
            let mut d = self.d.borrow_mut();
            let j = &mut d.jail;
            released.extend(j.jail_department.take());
            released.append(&mut j.funded_jails);
            released.append(&mut j.jails);
            j.jail_department = r.jail_department;
            j.funded_jails = r.funded_jails;
            j.jails = r.jails;
            let [capacity, population, jailed, arrests_last, arrests_this, unarrested, crimes_last, crimes_this] =
                r.values;
            j.capacity = capacity;
            j.population = population;
            j.jailed = jailed;
            j.arrests_last = arrests_last;
            j.arrests_this = arrests_this;
            j.unarrested_this = unarrested;
            j.crimes_last = crimes_last;
            j.crimes_this = crimes_this;
        }
        for o in released {
            self.host.release(o.token);
        }
        if version > 7 {
            self.read_air_source(inp);
        }
    }

    /// The air dispatch source ref (§11, PR13): the current source is
    /// released first (pointer cleared, then `Release`); a failed reference
    /// leaves none (foundations §2.7).
    fn read_air_source(&self, inp: &mut dyn InStream) {
        let old = self.d.borrow_mut().air_source.take();
        if let Some(o) = old {
            self.host.release(o.token);
        }
        let source = inp.get_ref_as(IID_DISPATCH_SOURCE).and_then(as_obj);
        self.d.borrow_mut().air_source = source;
    }
}

fn write_list(out: &mut dyn OutStream, list: &[ObjRef]) {
    out.put_u32(list.len() as u32);
    for o in list {
        out.put_ref(&host_ref(&Some(*o)));
    }
}

impl SaveObject for ProtectionSim {
    fn class_id(&self) -> u32 {
        self.kind.params().clsid
    }

    /// The fire (v3) or police (v8) record (§11); returns `!GetError()`.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        if out.error() != 0 {
            return false;
        }
        match self.kind {
            ProtectionKind::Fire => {
                out.put_u16(FIRE_RECORD_VERSION);
                self.write_base(out);
                out.put_ref(&host_ref(&self.d.borrow().air_source));
            }
            ProtectionKind::Police => {
                out.put_u16(POLICE_RECORD_VERSION);
                if self.write_base(out) {
                    let d = self.d.borrow();
                    let j = &d.jail;
                    out.put_ref(&host_ref(&j.jail_department));
                    write_list(out, &j.funded_jails);
                    write_list(out, &j.jails);
                    for v in [
                        j.capacity,
                        j.population,
                        j.jailed,
                        j.arrests_last,
                        j.arrests_this,
                        j.unarrested_this,
                        j.crimes_last,
                        j.crimes_this,
                    ] {
                        out.put_i32(v);
                    }
                    out.put_ref(&host_ref(&d.air_source));
                }
            }
        }
        out.error() == 0
    }

    /// The fire or police `Read` (§11).
    fn read(&self, inp: &mut dyn InStream) -> bool {
        let Some(version) = inp.get_u16() else { return false };
        match self.kind {
            ProtectionKind::Fire => {
                if version > FIRE_RECORD_VERSION {
                    return false;
                }
                self.d.borrow_mut().loaded = true;
                let _ = self.read_base(inp);
                if version > 2 {
                    self.read_air_source(inp);
                }
            }
            ProtectionKind::Police => {
                if version > POLICE_RECORD_VERSION {
                    return false;
                }
                if self.read_base(inp) {
                    self.read_police(inp, version);
                }
                self.d.borrow_mut().loaded = true;
            }
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
