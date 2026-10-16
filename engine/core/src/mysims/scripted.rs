// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! A scripted [`MySimHost`]: every answer is a field the test sets, every
//! action goes to a log. Used by the unit tests, the conformance runner and
//! the C ABI.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use crate::stream::RefValue;

use super::{Building, Dispatch, Field, Grid, JobDestination, Lot, MySimHost, MySimPost};

/// The class ID of a host object reference.
pub const HOST_CLASS: u32 = 0x1234_5678;

/// One logged host action.
#[derive(Clone, Debug, PartialEq)]
pub enum MsCall {
    CreateTable(i32),
    Activate(i32),
    AgentLua(bool),
    Post(MySimPost),
    Dispatch(i32, String),
    Trigger(String, [f32; 3]),
    Subscribe(u32, bool),
}

/// The scripted host.
#[derive(Default)]
pub struct ScriptedMySimHost {
    pub tables: RefCell<BTreeSet<i32>>,
    /// No Lua state: `create_table` fails.
    pub no_lua: Cell<bool>,
    pub fields: RefCell<BTreeMap<(i32, String), Field>>,
    pub tuning: RefCell<BTreeMap<String, f64>>,
    pub ui_blocked: Cell<bool>,
    pub cells: Cell<(u32, u32)>,
    pub disaster_ongoing: Cell<bool>,
    pub disaster_locations: RefCell<Vec<(u32, (i32, i32))>>,
    pub buildings: RefCell<BTreeMap<u64, Building>>,
    pub lots: RefCell<BTreeMap<u64, Lot>>,
    /// Positions of non-building occupants (fires).
    pub positions: RefCell<BTreeMap<u64, [f32; 3]>>,
    /// `buildings_in_box` / `all_buildings` fail.
    pub no_query: Cell<bool>,
    /// Lots of `lots_in_rect` (filtered by cell).
    pub rect_lots: RefCell<Vec<Lot>>,
    pub special: RefCell<Vec<u64>>,
    pub schools: RefCell<Vec<u64>>,
    pub hospitals: RefCell<Vec<u64>>,
    pub grades: RefCell<BTreeMap<u64, f32>>,
    pub job_titles: RefCell<BTreeMap<u64, Vec<u8>>>,
    pub landmark_funding: RefCell<BTreeMap<u64, f32>>,
    pub average_age: RefCell<BTreeMap<(i32, i32), i32>>,
    pub eq: Cell<Option<f32>>,
    pub hq: Cell<Option<f32>>,
    pub life_expectancy: Cell<Option<u8>>,
    pub new_age: Cell<Option<u8>>,
    pub grids: RefCell<BTreeMap<(Grid, i32, i32), i32>>,
    /// Grid values at any position.
    pub grids_at: RefCell<BTreeMap<Grid, i32>>,
    pub coverage: RefCell<BTreeMap<bool, i32>>,
    pub powered: RefCell<BTreeMap<u64, bool>>,
    pub watered: RefCell<BTreeMap<u64, bool>>,
    pub stations: RefCell<BTreeMap<bool, Vec<(u64, f32)>>>,
    pub travel_ratio: Cell<Option<f32>>,
    pub no_trip_simulators: Cell<bool>,
    pub dispatch: RefCell<BTreeMap<i32, Dispatch>>,
    /// `find_new_job` answers, in order; false when empty.
    pub find_job: RefCell<VecDeque<bool>>,
    /// The destination `find_new_job` true sets (reported by `job_destination`).
    pub job_destination: RefCell<Option<JobDestination>>,
    pub change_job: Cell<bool>,
    pub commute: Cell<Option<f32>>,
    pub neighbor_names: RefCell<BTreeMap<(i32, i32), Vec<u8>>>,
    pub refs: RefCell<BTreeMap<u64, i32>>,
    pub subscribed: RefCell<BTreeSet<u32>>,
    pub log: RefCell<Vec<MsCall>>,
}

impl ScriptedMySimHost {
    pub fn new() -> Rc<ScriptedMySimHost> {
        let h = ScriptedMySimHost::default();
        h.cells.set((64, 64));
        Rc::new(h)
    }
    pub fn take_log(&self) -> Vec<MsCall> {
        std::mem::take(&mut self.log.borrow_mut())
    }
    fn push(&self, c: MsCall) {
        self.log.borrow_mut().push(c);
    }
    /// A Lua field of MySim `sim`.
    pub fn field(&self, sim: i32, name: &str) -> Option<Field> {
        self.fields.borrow().get(&(sim, name.to_string())).cloned()
    }
    /// A number field.
    pub fn num(&self, sim: i32, name: &str) -> Option<f64> {
        match self.field(sim, name) {
            Some(Field::Num(n)) => Some(n),
            _ => None,
        }
    }
    /// Adds a building with a lot.
    pub fn add_building(&self, token: u64, b: Building, lot: Option<Lot>) {
        self.buildings.borrow_mut().insert(token, b);
        if let Some(l) = lot {
            self.lots.borrow_mut().insert(token, l);
        }
    }
}

impl MySimHost for ScriptedMySimHost {
    fn create_table(&self, sim: i32) -> bool {
        if self.no_lua.get() {
            return false;
        }
        self.push(MsCall::CreateTable(sim));
        self.tables.borrow_mut().insert(sim);
        true
    }
    fn set_field(&self, sim: i32, name: &str, v: Field) {
        self.fields.borrow_mut().insert((sim, name.to_string()), v);
    }
    fn activate_lua(&self, sim: i32) -> bool {
        self.push(MsCall::Activate(sim));
        !self.no_lua.get()
    }
    fn tuning(&self, key: &str) -> Option<f64> {
        self.tuning.borrow().get(key).copied()
    }
    fn ui_blocked(&self) -> bool {
        self.ui_blocked.get()
    }
    fn register_agent_lua(&self, on: bool) {
        self.push(MsCall::AgentLua(on));
    }
    fn cell_counts(&self) -> (u32, u32) {
        self.cells.get()
    }
    fn cell_center(&self, cx: i32, cz: i32) -> (f32, f32) {
        (cx as f32 * 16.0 + 8.0, cz as f32 * 16.0 + 8.0)
    }
    fn disaster_ongoing(&self) -> bool {
        self.disaster_ongoing.get()
    }
    fn disaster_locations(&self) -> Vec<(u32, (i32, i32))> {
        self.disaster_locations.borrow().clone()
    }
    fn building(&self, b: u64) -> Option<Building> {
        self.buildings.borrow().get(&b).cloned()
    }
    fn lot(&self, b: u64) -> Option<Lot> {
        self.lots.borrow().get(&b).cloned()
    }
    fn occupant_position(&self, o: u64) -> Option<[f32; 3]> {
        if let Some(b) = self.buildings.borrow().get(&o) {
            return Some(b.position);
        }
        self.positions.borrow().get(&o).copied()
    }
    fn buildings_in_box(&self, x0: i32, z0: i32, x1: i32, z1: i32) -> Option<Vec<u64>> {
        if self.no_query.get() {
            return None;
        }
        let lots = self.lots.borrow();
        Some(
            self.buildings
                .borrow()
                .keys()
                .copied()
                .filter(|b| {
                    lots.get(b)
                        .map_or(true, |l| (x0..=x1).contains(&l.cell.0) && (z0..=z1).contains(&l.cell.1))
                })
                .collect(),
        )
    }
    fn all_buildings(&self) -> Option<Vec<u64>> {
        if self.no_query.get() {
            return None;
        }
        Some(self.buildings.borrow().keys().copied().collect())
    }
    fn lots_in_rect(&self, x0: i32, z0: i32, x1: i32, z1: i32) -> Vec<Lot> {
        self.rect_lots
            .borrow()
            .iter()
            .filter(|l| (x0..=x1).contains(&l.cell.0) && (z0..=z1).contains(&l.cell.1))
            .cloned()
            .collect()
    }
    fn special_buildings(&self) -> Vec<u64> {
        self.special.borrow().clone()
    }
    fn education_buildings(&self, hospitals: bool) -> Vec<u64> {
        if hospitals {
            self.hospitals.borrow().clone()
        } else {
            self.schools.borrow().clone()
        }
    }
    fn grade(&self, b: u64, _hospital: bool) -> Option<f32> {
        self.grades.borrow().get(&b).copied()
    }
    fn job_title(&self, b: u64) -> Vec<u8> {
        self.job_titles.borrow().get(&b).cloned().unwrap_or_default()
    }
    fn landmark_funding(&self, b: u64) -> Option<f32> {
        self.landmark_funding.borrow().get(&b).copied()
    }
    fn average_age(&self, x: i32, z: i32) -> Option<i32> {
        self.average_age.borrow().get(&(x, z)).copied()
    }
    fn eq_by_wealth(&self, _x: i32, _z: i32, _wealth: u8) -> Option<f32> {
        self.eq.get()
    }
    fn hq_by_wealth(&self, _x: i32, _z: i32, _wealth: u8) -> Option<f32> {
        self.hq.get()
    }
    fn life_expectancy(&self, _x: i32, _z: i32) -> Option<u8> {
        self.life_expectancy.get()
    }
    fn new_age_by_wealth(&self, _wealth: u8) -> Option<u8> {
        self.new_age.get()
    }
    fn grid(&self, g: Grid, x: i32, z: i32) -> Option<i32> {
        self.grids.borrow().get(&(g, x, z)).copied()
    }
    fn grid_at(&self, g: Grid, _x: f32, _z: f32) -> Option<i32> {
        self.grids_at.borrow().get(&g).copied()
    }
    fn coverage(&self, police: bool, _x0: i32, _z0: i32, _w: i32, _h: i32) -> Option<i32> {
        self.coverage.borrow().get(&police).copied()
    }
    fn powered(&self, b: u64) -> Option<bool> {
        self.powered.borrow().get(&b).copied()
    }
    fn watered(&self, b: u64) -> Option<bool> {
        self.watered.borrow().get(&b).copied()
    }
    fn stations(&self, police: bool) -> Option<Vec<(u64, f32)>> {
        self.stations.borrow().get(&police).cloned()
    }
    fn travel_time_ratio(&self, _x: i32, _z: i32, _travel_type: u32) -> Option<f32> {
        self.travel_ratio.get()
    }
    fn trip_simulators(&self) -> bool {
        !self.no_trip_simulators.get()
    }
    fn dispatch(&self, sim: i32) -> Option<Dispatch> {
        self.dispatch.borrow().get(&sim).cloned()
    }
    fn find_new_job(&self, _sim: i32) -> bool {
        self.find_job.borrow_mut().pop_front().unwrap_or(false)
    }
    fn change_job(&self, _sim: i32, b: u64) -> bool {
        if self.change_job.get() {
            *self.job_destination.borrow_mut() = Some(JobDestination::Lot {
                building: b,
                cell: self.lots.borrow().get(&b).map_or((0, 0), |l| l.cell),
            });
        }
        self.change_job.get()
    }
    fn job_destination(&self, _sim: i32) -> Option<JobDestination> {
        *self.job_destination.borrow()
    }
    fn commute_time(&self, _sim: i32) -> Option<f32> {
        self.commute.get()
    }
    fn neighbor_city_name(&self, cell: (i32, i32)) -> Option<Vec<u8>> {
        self.neighbor_names.borrow().get(&cell).cloned()
    }
    fn dispatch_command(&self, sim: i32, what: &str) {
        self.push(MsCall::Dispatch(sim, what.to_string()));
    }
    fn post(&self, m: &MySimPost) {
        self.push(MsCall::Post(*m));
    }
    fn automata_trigger(&self, name: &str, position: [f32; 3]) {
        self.push(MsCall::Trigger(name.to_string(), position));
    }
    fn subscribe(&self, msg_type: u32, on: bool) {
        if on {
            self.subscribed.borrow_mut().insert(msg_type);
        } else {
            self.subscribed.borrow_mut().remove(&msg_type);
        }
        self.push(MsCall::Subscribe(msg_type, on));
    }
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
    }
    fn object_ref(&self, token: u64) -> RefValue {
        RefValue::Host {
            token,
            class_id: HOST_CLASS,
        }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } => Some(*token),
            _ => None,
        }
    }
}
