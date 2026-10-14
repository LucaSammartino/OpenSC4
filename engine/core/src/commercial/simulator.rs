// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The commercial simulator (§2.1, §3, §4, §8.1).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::grid::TractGrid;
use crate::residential::{distance_to_convergence, seed_proximity};
use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::{
    developer, instance, msg, state as st, CommercialHost, CommercialMessage, AGENT_FLAGS, AGENT_LEVELS, AGENT_NAME,
    CLSID_COMMERCIAL_SIMULATOR, DEMAND_NODES, IID_COMMERCIAL_SIMULATOR, IID_MESSAGE_TARGET2, IID_SERIALIZABLE,
    IID_SIM_GRID, IID_UNKNOWN, OCCUPANT_TYPE_BUILDING, PURPOSE_OFFICE, RECORD_VERSION, TRACT_SIZE, WEALTH_CO2,
    WEALTH_CO3,
};

/// `QueryInterface` (§1): the commercial simulator answers its own IID, `1`,
/// `cIGZMessageTarget2` and `cIGZSerializable`.
pub fn query_interface(iid: u32) -> bool {
    matches!(
        iid,
        IID_UNKNOWN | IID_COMMERCIAL_SIMULATOR | IID_MESSAGE_TARGET2 | IID_SERIALIZABLE
    )
}

/// Every field of §2.1 (saved or not).
#[derive(Clone)]
pub struct CommercialState {
    pub initialized: bool,
    pub ref_count: u32,
    /// `Release` destroyed the object (§2.3).
    pub destroyed: bool,
    /// The write-only static `0x00B48614` (§4.1 step 1).
    pub static_flag: bool,
    pub city_held: bool,
    pub bds_held: bool,
    pub message_server_held: bool,
    /// Constructor 1 (§2.1).
    pub dirty: bool,
    /// 0..3, constructor 3 (§4.3).
    pub state: i32,
    /// The demand nodes `0x3000` / `0x3800`, held, never read (§2.1).
    pub demand_nodes: [Option<u64>; 2],
    /// The grids' tract size (§2.1): 1.
    pub tract_size: u8,
    /// The CO$$ proximity grid (instance `0x89D5B788`).
    pub co2: Option<Rc<dyn TractGrid<u8>>>,
    /// The CO$$$ proximity grid (instance `0x89D5B6EF`).
    pub co3: Option<Rc<dyn TractGrid<u8>>>,
}

impl Default for CommercialState {
    /// The constructor's values (§2.1).
    fn default() -> CommercialState {
        CommercialState {
            initialized: false,
            ref_count: 0,
            destroyed: false,
            static_flag: false,
            city_held: false,
            bds_held: false,
            message_server_held: false,
            dirty: true,
            state: st::IDLE,
            demand_nodes: [None, None],
            tract_size: TRACT_SIZE as u8,
            co2: None,
            co3: None,
        }
    }
}

impl CommercialState {
    /// The saved flag byte (§8.1): `dirty || state ≠ 3`.
    pub fn flag(&self) -> bool {
        self.dirty || self.state != st::IDLE
    }
}

/// The commercial simulator.
pub struct CommercialSim {
    host: Rc<dyn CommercialHost>,
    token: Cell<u64>,
    s: RefCell<CommercialState>,
}

impl CommercialSim {
    /// A constructed (not initialised) simulator, reference count 0.
    pub fn new(host: Rc<dyn CommercialHost>, token: u64) -> Rc<CommercialSim> {
        Rc::new(CommercialSim {
            host,
            token: Cell::new(token),
            s: RefCell::new(CommercialState::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    /// A copy of the state (tests, diagnostics).
    pub fn snapshot(&self) -> CommercialState {
        self.s.borrow().clone()
    }

    /// Changes the state directly (tests, vectors, a host restoring state).
    pub fn with_state<R>(&self, f: impl FnOnce(&mut CommercialState) -> R) -> R {
        let mut st = self.s.borrow().clone();
        let r = f(&mut st);
        *self.s.borrow_mut() = st;
        r
    }

    // ---- identity (§2.3) ----

    /// Slot 1 `AddRef`.
    pub fn add_ref(&self) -> u32 {
        let mut s = self.s.borrow_mut();
        s.ref_count = s.ref_count.wrapping_add(1);
        s.ref_count
    }

    /// Slot 2 `Release` (§2.3 *[compat]*): above 1 it decrements and returns
    /// the new count; at 1 or less it calls `Shutdown`, destroys the object
    /// (here: marks it destroyed) and returns 0 without decrementing first.
    pub fn release(&self) -> u32 {
        let c = self.s.borrow().ref_count;
        if c > 1 {
            self.s.borrow_mut().ref_count = c - 1;
            return c - 1;
        }
        self.shutdown();
        self.s.borrow_mut().destroyed = true;
        0
    }

    // ---- lifecycle (§4.1, §4.2) ----

    /// Slot 3 `Init` (§4.1). False when the demand simulator or either demand
    /// node is missing; the grids are then not created.
    pub fn init(&self) -> bool {
        let h = &*self.host;
        {
            let mut s = self.s.borrow_mut();
            if s.initialized {
                return true;
            }
            // Step 1.
            s.initialized = true;
            s.static_flag = true;
        }
        // Step 2: the four agents, in order.
        for level in AGENT_LEVELS {
            h.add_agent(level, AGENT_NAME, AGENT_FLAGS);
        }
        // Step 3.
        if !self.s.borrow().message_server_held && h.acquire_message_server() {
            self.s.borrow_mut().message_server_held = true;
            for m in msg::SUBSCRIPTIONS {
                h.subscribe(m);
            }
        }
        // Step 4.
        if !self.s.borrow().city_held && h.acquire_city() {
            self.s.borrow_mut().city_held = true;
        }
        if !self.s.borrow().bds_held && h.acquire_bds() {
            self.s.borrow_mut().bds_held = true;
        }
        // Step 5 [compat]: each node found is held; the first missing one
        // ends `Init` (short-circuit: 0x3800 is not asked for when 0x3000 is
        // missing).
        if !h.has_demand_simulator() {
            return false;
        }
        for (i, id) in DEMAND_NODES.into_iter().enumerate() {
            let held = self.s.borrow().demand_nodes[i];
            if held.is_some() {
                continue;
            }
            match h.demand_node(id) {
                Some(node) => self.s.borrow_mut().demand_nodes[i] = Some(node),
                None => return false,
            }
        }
        // Step 6: CO$$$ first, then CO$$; a grid loaded from the save is kept.
        if self.s.borrow().co3.is_none() {
            let g = h.create_grid(instance::CO3, TRACT_SIZE);
            self.s.borrow_mut().co3 = g;
        }
        if self.s.borrow().co2.is_none() {
            let g = h.create_grid(instance::CO2, TRACT_SIZE);
            self.s.borrow_mut().co2 = g;
        }
        true
    }

    /// Slot 4 `Shutdown` (§4.2). Returns true.
    pub fn shutdown(&self) -> bool {
        let h = &*self.host;
        let s = {
            let mut s = self.s.borrow_mut();
            if !s.initialized {
                return true;
            }
            s.initialized = false;
            let taken = s.clone();
            s.co2 = None;
            s.co3 = None;
            s.city_held = false;
            s.bds_held = false;
            s.message_server_held = false;
            s.demand_nodes = [None, None];
            taken
        };
        // Shutdown and release both grids (the release is dropping them).
        for g in [&s.co2, &s.co3].into_iter().flatten() {
            h.shutdown_grid(g);
        }
        drop(s.co2);
        drop(s.co3);
        if s.city_held {
            h.release_city();
        }
        if s.bds_held {
            h.release_bds();
        }
        if s.message_server_held {
            for m in msg::SUBSCRIPTIONS {
                h.unsubscribe(m);
            }
            h.release_message_server();
        }
        for node in s.demand_nodes.into_iter().flatten() {
            h.release_demand_node(node);
        }
        h.remove_agent();
        true
    }

    // ---- messages (§3) ----

    /// `DoMessage` (§3): the idle and day agents run one `DoIdle` step; the
    /// one-shot and month agents do nothing; an occupant insert or removal
    /// of a building whose profile purpose is 3 (office) marks the maps
    /// dirty, with no preview-occupant test *[quirk]*. Returns true.
    pub fn do_message(&self, m: &CommercialMessage) -> bool {
        match m.msg_type {
            msg::SIM_IDLE | msg::SIM_NEW_DAY => self.do_idle(),
            msg::OCCUPANT_INSERTED | msg::OCCUPANT_REMOVED if m.data3 == OCCUPANT_TYPE_BUILDING => {
                if let Some(occ) = m.void1 {
                    if self.host.building_purpose(occ) == Some(PURPOSE_OFFICE) {
                        self.s.borrow_mut().dirty = true;
                    }
                }
            }
            _ => {}
        }
        true
    }

    /// `DoIdle` (§4.3): one step of the four-state machine. State 0 falls
    /// through to state 1 in the same call. [choice] A state outside 0..3
    /// (impossible: the state is never saved) does nothing.
    pub fn do_idle(&self) {
        let state = self.s.borrow().state;
        match state {
            st::IDLE => {
                let mut s = self.s.borrow_mut();
                if s.dirty {
                    s.dirty = false;
                    s.state = st::START;
                }
            }
            st::START | st::REBUILD_CO2 => {
                self.s.borrow_mut().state = st::REBUILD_CO2;
                self.rebuild(WEALTH_CO2);
                self.s.borrow_mut().state = st::REBUILD_CO3;
            }
            st::REBUILD_CO3 => {
                self.rebuild(WEALTH_CO3);
                self.s.borrow_mut().state = st::IDLE;
            }
            _ => {}
        }
    }

    /// One map rebuild (§4.4 *[compat]*): the building-map key of
    /// (office, `wealth`), then the shared seed-and-converge transform run
    /// directly on the proximity grid (no temporary, no copy).
    pub fn rebuild(&self, wealth: u32) {
        let grid = {
            let s = self.s.borrow();
            if wealth == WEALTH_CO3 {
                s.co3.clone()
            } else {
                s.co2.clone()
            }
        };
        let key = self.host.building_map_key(PURPOSE_OFFICE, wealth);
        let map = self.host.building_map();
        let Some(grid) = grid else {
            return;
        };
        // *[compat]* The shared seed helper (residential.md §5.3 step 1,
        // RG24) writes nothing when the map is null, not even the edge; the
        // passes of the transform still run [question: literal reading of
        // helper 0x005E5130, seed then passes].
        if let Some(m) = map {
            seed_proximity(&*grid, &*m, key);
        }
        distance_to_convergence(&*grid, grid.count_x(), grid.count_z());
    }

    // ---- the interface (§4.6) ----

    /// Slot 5 `GetProximityMap(developerType)`: `0x3320` → CO$$, `0x3330` →
    /// CO$$$, anything else → none. No `AddRef`.
    pub fn proximity_map(&self, developer_type: u32) -> Option<Rc<dyn TractGrid<u8>>> {
        let s = self.s.borrow();
        match developer_type {
            developer::CO2 => s.co2.clone(),
            developer::CO3 => s.co3.clone(),
            _ => None,
        }
    }

    /// Parses the §8.1 record: (dirty, CO$$, CO$$$).
    fn parse(&self, inp: &mut dyn InStream) -> Option<(bool, ResolvedGrid, ResolvedGrid)> {
        if inp.get_u16()? != RECORD_VERSION {
            return None;
        }
        let flag = inp.get_u8()?;
        let r2 = inp.get_ref_as(IID_SIM_GRID)?;
        let g2 = self.resolve(&r2);
        let r3 = inp.get_ref_as(IID_SIM_GRID)?;
        let g3 = self.resolve(&r3);
        if inp.error() != 0 {
            return None;
        }
        Some((flag != 0, g2, g3))
    }

    fn resolve(&self, r: &RefValue) -> ResolvedGrid {
        if r.is_null() {
            None
        } else {
            self.host.resolve_grid(r)
        }
    }
}

type ResolvedGrid = Option<Rc<dyn TractGrid<u8>>>;

fn grid_ref(g: &ResolvedGrid) -> RefValue {
    g.as_ref().map_or(RefValue::Null, |g| g.save_ref())
}

impl SaveObject for CommercialSim {
    fn class_id(&self) -> u32 {
        CLSID_COMMERCIAL_SIMULATOR
    }

    /// The §8.1 record: version 1, the flag byte, the two grid references.
    /// True unless the stream reports an error.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let (flag, co2, co3) = {
            let s = self.s.borrow();
            (s.flag(), s.co2.clone(), s.co3.clone())
        };
        out.put_u16(RECORD_VERSION);
        out.put_bool(flag);
        out.put_ref(&grid_ref(&co2));
        out.put_ref(&grid_ref(&co3));
        out.error() == 0
    }

    /// The §8.1 record: false unless the version is exactly 1. `dirty :=
    /// flag ≠ 0`; `state` keeps its value (the constructor's 3), so a city
    /// saved mid-rebuild restarts the rebuild at the first `DoIdle`.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        match self.parse(inp) {
            Some((dirty, co2, co3)) => {
                let old = {
                    let mut s = self.s.borrow_mut();
                    s.dirty = dirty;
                    (std::mem::replace(&mut s.co2, co2), std::mem::replace(&mut s.co3, co3))
                };
                // [choice] (§8.1): the old grids are released.
                drop(old);
                true
            }
            None => false,
        }
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
