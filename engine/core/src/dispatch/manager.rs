// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The dispatch manager (§2–§4, §7).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::{
    dispatch_type, msg, state, vehicle_view_kind, DispatchHost, DispatchInfo, DispatchMessage, CLSID_DISPATCH_MANAGER,
    IID_AUTOMATON, IID_DISPATCH_MANAGER, IID_MESSAGE_TARGET2, IID_OCCUPANT, IID_PYLON, IID_SERIALIZABLE, IID_UNKNOWN,
    RECORD_VERSION, TRACK_KIND0_GROUPS, UI_STATE_ACTIVE, UI_STATE_IDLE,
};

/// `QueryInterface` (§1): the sub-object offset (`1` and `0x694028EC` at
/// +0, `cIGZMessageTarget2` at +4, `cIGZSerializable` at +8), else `None`.
pub fn query_interface(iid: u32) -> Option<u32> {
    match iid {
        IID_UNKNOWN | IID_DISPATCH_MANAGER => Some(0),
        IID_MESSAGE_TARGET2 => Some(4),
        IID_SERIALIZABLE => Some(8),
        _ => None,
    }
}

/// Every field of §2.
#[derive(Clone, Debug)]
pub struct DispatchState {
    pub initialized: bool,
    pub ref_count: u32,
    /// `Release` destroyed the object.
    pub destroyed: bool,
    /// Set by `Read` (before the version check).
    pub loaded: bool,
    pub message_server_held: bool,
    pub city_held: bool,
    /// The dispatch list, in list order.
    pub records: Vec<DispatchInfo>,
    /// The last dispatch ID handed out.
    pub next_id: u32,
    pub vehicle_view: Option<u64>,
    /// The pylon the player is dragging (held).
    pub active_pylon: Option<u64>,
    /// One per free unit of the open tool's type (held; `None`: a null
    /// reference read from a record).
    pub available_pylons: Vec<Option<u64>>,
    /// The type whose tool is open; 8 = none.
    pub tool_type: u32,
    /// Registered dispatch sources (held).
    pub sources: Vec<u64>,
}

impl Default for DispatchState {
    /// The constructor's values (§2).
    fn default() -> DispatchState {
        DispatchState {
            initialized: false,
            ref_count: 0,
            destroyed: false,
            loaded: false,
            message_server_held: false,
            city_held: false,
            records: Vec::new(),
            next_id: 0,
            vehicle_view: None,
            active_pylon: None,
            available_pylons: Vec::new(),
            tool_type: dispatch_type::NONE,
            sources: Vec::new(),
        }
    }
}

/// The dispatch manager.
pub struct DispatchManager {
    host: Rc<dyn DispatchHost>,
    token: Cell<u64>,
    s: RefCell<DispatchState>,
}

/// What `Read` parsed (§7), applied only when complete.
struct Parsed {
    records: Vec<DispatchInfo>,
    next_id: u32,
    active: Option<u64>,
    pylons: Vec<Option<u64>>,
    tool_type: u32,
}

impl DispatchManager {
    /// A constructed (not initialised) manager, reference count 0.
    pub fn new(host: Rc<dyn DispatchHost>, token: u64) -> Rc<DispatchManager> {
        Rc::new(DispatchManager {
            host,
            token: Cell::new(token),
            s: RefCell::new(DispatchState::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn snapshot(&self) -> DispatchState {
        self.s.borrow().clone()
    }

    pub fn with_state<R>(&self, f: impl FnOnce(&mut DispatchState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }

    // ---- identity (§1, §8) ----

    pub fn add_ref(&self) -> u32 {
        let mut s = self.s.borrow_mut();
        s.ref_count = s.ref_count.wrapping_add(1);
        s.ref_count
    }

    /// Slot 2 `Release`: above 1 it decrements; otherwise `Shutdown`, the
    /// destructor (the records' references and the sources are released)
    /// and 0.
    pub fn release(&self) -> u32 {
        let c = self.s.borrow().ref_count;
        if c > 1 {
            self.s.borrow_mut().ref_count = c - 1;
            return c - 1;
        }
        self.shutdown();
        self.destroy();
        0
    }

    fn destroy(&self) {
        let h = &*self.host;
        let (records, sources) = {
            let mut s = self.s.borrow_mut();
            s.destroyed = true;
            (std::mem::take(&mut s.records), std::mem::take(&mut s.sources))
        };
        for r in records {
            for t in [r.station, r.automaton, r.pylon].into_iter().flatten() {
                h.release(t);
            }
        }
        for t in sources {
            h.release(t);
        }
    }

    // ---- lifecycle (§4.1) ----

    /// Slot 3 `Init` (§4.1). Returns true.
    pub fn init(&self) -> bool {
        let h = &*self.host;
        {
            let mut s = self.s.borrow_mut();
            if s.initialized {
                return true;
            }
            s.initialized = true;
        }
        if !self.s.borrow().message_server_held && h.acquire_message_server() {
            self.s.borrow_mut().message_server_held = true;
            for m in msg::SUBSCRIPTIONS {
                h.subscribe(m);
            }
        }
        if self.s.borrow().vehicle_view.is_none() {
            let v = h.create_vehicle_view();
            self.s.borrow_mut().vehicle_view = v;
        }
        if !self.s.borrow().city_held && h.acquire_city() {
            self.s.borrow_mut().city_held = true;
        }
        if self.s.borrow().loaded {
            self.s.borrow_mut().tool_type = dispatch_type::NONE;
            self.set_active_pylon(None);
            self.clear_available_pylons();
            let n = self.s.borrow().records.len();
            for i in 0..n {
                let r = self.s.borrow().records[i];
                if let (Some(a), Some(kind)) = (r.automaton, vehicle_view_kind(r.dispatch_type)) {
                    let handle = h.track_vehicle(a, kind);
                    self.s.borrow_mut().records[i].view_handle = handle;
                }
            }
        }
        true
    }

    /// Slot 4 `Shutdown` (§4.1). Records are kept. Returns true.
    pub fn shutdown(&self) -> bool {
        let h = &*self.host;
        if !self.s.borrow().initialized {
            return true;
        }
        if std::mem::take(&mut self.s.borrow_mut().message_server_held) {
            for m in msg::SUBSCRIPTIONS {
                h.unsubscribe(m);
            }
            h.release_message_server();
        }
        if let Some(p) = self.s.borrow_mut().active_pylon.take() {
            h.pylon_shutdown(p);
            h.release(p);
        }
        self.clear_available_pylons();
        if let Some(v) = self.s.borrow_mut().vehicle_view.take() {
            h.destroy_vehicle_view(v);
        }
        if std::mem::take(&mut self.s.borrow_mut().city_held) {
            h.release_city();
        }
        self.s.borrow_mut().initialized = false;
        true
    }

    // ---- sources (§4.6, slots 13–15) ----

    /// Slot 13: holds a reference; no duplicate check.
    pub fn register_dispatch_source(&self, source: u64) {
        self.host.add_ref(source);
        self.s.borrow_mut().sources.push(source);
    }

    /// Slot 14: the first match is removed and released.
    pub fn unregister_dispatch_source(&self, source: u64) {
        let removed = {
            let mut s = self.s.borrow_mut();
            match s.sources.iter().position(|&t| t == source) {
                Some(i) => {
                    s.sources.remove(i);
                    true
                }
                None => false,
            }
        };
        if removed {
            self.host.release(source);
        }
    }

    /// Slot 15: the first registered source whose `CanHandleDispatchType`
    /// is true (borrowed).
    pub fn get_dispatch_source(&self, t: u32) -> Option<u64> {
        let sources = self.s.borrow().sources.clone();
        sources.into_iter().find(|&src| self.host.source_can_handle(src, t))
    }

    // ---- queries (§4.6) ----

    /// Slot 5 `CanDispatch(type, x, z)`.
    pub fn can_dispatch(&self, t: u32, x: f32, z: f32) -> bool {
        match self.get_dispatch_source(t) {
            Some(src) => self.host.source_can_dispatch(src, x, z).0,
            None => false,
        }
    }

    /// Slot 10 `GetDispatchInfo(id)`: a copy (no references taken).
    pub fn get_dispatch_info(&self, id: u32) -> Option<DispatchInfo> {
        self.s.borrow().records.iter().find(|r| r.id == id).copied()
    }

    /// Slot 11: records of that type.
    pub fn get_dispatch_count(&self, t: u32) -> u32 {
        self.s.borrow().records.iter().filter(|r| r.dispatch_type == t).count() as u32
    }

    /// Slot 12: the IDs of that type in list order; it stops when the
    /// number written equals `n`, checked after each write [quirk] (`n = 0`
    /// = no limit).
    pub fn get_dispatch_id_array(&self, t: u32, n: u32) -> Vec<u32> {
        let mut out = Vec::new();
        for r in self.s.borrow().records.iter().filter(|r| r.dispatch_type == t) {
            out.push(r.id);
            if out.len() as u32 == n {
                break;
            }
        }
        out
    }

    /// Slot 18 (borrowed).
    pub fn get_active_pylon(&self) -> Option<u64> {
        self.s.borrow().active_pylon
    }

    /// Slot 20: the vehicle of the record that holds `p` (borrowed).
    pub fn get_automaton_from_pylon(&self, p: u64) -> Option<u64> {
        self.s
            .borrow()
            .records
            .iter()
            .find(|r| r.pylon == Some(p))
            .and_then(|r| r.automaton)
    }

    /// Slot 21: the free units summed over the source's stations.
    pub fn get_available_pylon_count(&self, t: u32) -> u32 {
        match self.get_dispatch_source(t) {
            Some(src) => self
                .host
                .source_stations(src)
                .iter()
                .map(|s| s.free_units())
                .fold(0u32, u32::wrapping_add),
            None => 0,
        }
    }

    /// Slot 22 (borrowed).
    pub fn get_dispatch_vehicle_view(&self) -> Option<u64> {
        self.s.borrow().vehicle_view
    }

    fn index_of_id(&self, id: u32) -> Option<usize> {
        self.s.borrow().records.iter().position(|r| r.id == id)
    }

    // ---- Dispatch (§4.2) ----

    /// Slot 6 `Dispatch(type, x, z)`: the dispatch ID, 0 on failure.
    pub fn dispatch(&self, t: u32, x: f32, z: f32) -> u32 {
        let h = &*self.host;
        // 1.
        let Some(src) = self.get_dispatch_source(t) else {
            return 0;
        };
        let (can, all_busy) = h.source_can_dispatch(src, x, z);
        if !can {
            return 0;
        }
        // 2.
        let pylon = self.s.borrow().active_pylon;
        let existing = pylon.and_then(|p| self.s.borrow().records.iter().position(|r| r.pylon == Some(p)));
        let id = match (pylon, existing) {
            (None, _) if all_busy => return 0,
            (Some(_), Some(i)) => {
                let y = h.altitude(x, z);
                let mut s = self.s.borrow_mut();
                let r = &mut s.records[i];
                r.target = (x, y, z);
                r.id
            }
            _ => {
                // 3. [quirk] The ID is consumed even if the dispatch fails.
                let id = {
                    let mut s = self.s.borrow_mut();
                    s.next_id = s.next_id.wrapping_add(1);
                    s.next_id
                };
                let y = h.altitude(x, z);
                let mut rec = DispatchInfo {
                    dispatch_type: t,
                    id,
                    target: (x, y, z),
                    ..DispatchInfo::default()
                };
                // 4.
                if !self.init_dispatch(&mut rec, src, pylon) {
                    return 0;
                }
                self.s.borrow_mut().records.push(rec);
                id
            }
        };
        // 5.
        if self.begin_dispatch(id, src) {
            id
        } else {
            self.revoke_dispatch_by_id(id);
            0
        }
    }

    /// `InitDispatch(record)` (§4.2 step 4).
    fn init_dispatch(&self, rec: &mut DispatchInfo, src: u64, pylon: Option<u64>) -> bool {
        let h = &*self.host;
        let (x, _, z) = rec.target;
        match pylon {
            None => {
                let Some((station, call_id)) = h.source_dispatch_from_nearest_station(src, x, z) else {
                    return false;
                };
                h.add_ref(station);
                rec.station = Some(station);
                rec.call_id = call_id;
                if rec.dispatch_type == self.s.borrow().tool_type {
                    // The available pylon with that call number moves to the
                    // record (the list's reference goes with it).
                    let avail = self.s.borrow().available_pylons.clone();
                    let found = avail
                        .iter()
                        .position(|e| e.is_some_and(|p| h.pylon_call_number(p) == call_id));
                    if let Some(i) = found {
                        rec.pylon = self.s.borrow_mut().available_pylons.remove(i);
                    }
                } else if let Some(p) = h.create_pylon() {
                    // [quirk] Created, given the station, never stored: leaked.
                    h.pylon_set_source(p, station);
                }
            }
            Some(p) => {
                let station = h.pylon_source(p);
                if let Some(st) = station {
                    h.add_ref(st);
                }
                rec.station = station;
                h.add_ref(p);
                rec.pylon = Some(p);
                let mut call_id = h.pylon_call_number(p) & 0xFFFF;
                // The result is ignored.
                h.source_dispatch_from_station(src, station, &mut call_id);
                rec.call_id = call_id;
                let removed = {
                    let mut s = self.s.borrow_mut();
                    match s.available_pylons.iter().position(|&e| e == Some(p)) {
                        Some(i) => {
                            s.available_pylons.remove(i);
                            true
                        }
                        None => false,
                    }
                };
                if removed {
                    h.release(p);
                }
            }
        }
        if let Some(p) = rec.pylon {
            let (x, y, z) = rec.target;
            h.pylon_move_to(p, x, y, z);
            h.pylon_set_dispatch_type(p, rec.dispatch_type);
        }
        true
    }

    /// `BeginDispatch(record)` (§4.2 step 5).
    fn begin_dispatch(&self, id: u32, src: u64) -> bool {
        let h = &*self.host;
        let Some(i) = self.index_of_id(id) else {
            return false;
        };
        let rec = self.s.borrow().records[i];
        let Some(a) = h.begin_vehicle(src, &rec) else {
            return false;
        };
        let mut handle = rec.view_handle;
        // DP9: only a new vehicle is tracked (kind 0 in occupant group 0x4202
        // or 0x4303, else 1); a re-targeted one keeps the record's handle.
        if rec.automaton.is_none() {
            let kind = if TRACK_KIND0_GROUPS.iter().any(|&g| h.vehicle_in_group(a, g)) {
                0
            } else {
                1
            };
            handle = h.track_vehicle(a, kind);
        }
        if let Some(i) = self.index_of_id(id) {
            let mut s = self.s.borrow_mut();
            s.records[i].automaton = Some(a);
            s.records[i].view_handle = handle;
        }
        h.post(msg::BEGIN, rec.dispatch_type, id);
        true
    }

    // ---- states (§4.3) ----

    /// `UpdateDispatchState(record, state)` (§4.3).
    pub fn update_dispatch_state(&self, id: u32, new_state: u32) {
        let h = &*self.host;
        let Some(i) = self.index_of_id(id) else { return };
        let working = new_state == state::WORKING;
        let (changed, rec) = {
            let mut s = self.s.borrow_mut();
            let r = &mut s.records[i];
            let changed = working != r.active;
            if changed {
                r.active = working;
            }
            (changed, *r)
        };
        if changed {
            h.post(
                if working { msg::ACTIVE } else { msg::INACTIVE },
                rec.dispatch_type,
                rec.id,
            );
        }
        if let Some(p) = rec.pylon {
            h.pylon_set_dispatch_state(p, new_state);
        }
    }

    // ---- ending and revoking (§4.4) ----

    /// `EndDispatch(record)` (§4.4): the record's references are released
    /// and cleared; the record stays in the list.
    fn end_dispatch(&self, i: usize) -> bool {
        let h = &*self.host;
        // [quirk] No inactive message.
        let rec = {
            let mut s = self.s.borrow_mut();
            s.records[i].active = false;
            s.records[i]
        };
        let Some(src) = self.get_dispatch_source(rec.dispatch_type) else {
            return false;
        };
        h.source_revoke_dispatch_from_station(src, rec.station, rec.call_id);
        if let Some(p) = rec.pylon {
            if self.s.borrow().active_pylon == Some(p) {
                self.set_active_pylon(None);
            }
            if rec.dispatch_type == self.s.borrow().tool_type {
                self.return_pylon_to_station(Some(src), p);
            } else {
                h.pylon_shutdown(p);
            }
            h.release(p);
        }
        if rec.view_handle != 0 {
            h.untrack_vehicle(rec.view_handle);
        }
        if let Some(a) = rec.automaton {
            h.release(a);
        }
        if let Some(st) = rec.station {
            h.release(st);
        }
        if let Some(j) = self.index_of_id(rec.id) {
            let mut s = self.s.borrow_mut();
            let r = &mut s.records[j];
            r.pylon = None;
            r.automaton = None;
            r.station = None;
            r.view_handle = 0;
        }
        h.post(msg::END, rec.dispatch_type, rec.id);
        true
    }

    /// Slot 7 `RevokeDispatchByID(id)`: `EndDispatch`, then the record is
    /// erased; false when no record has the ID.
    pub fn revoke_dispatch_by_id(&self, id: u32) -> bool {
        let Some(i) = self.index_of_id(id) else {
            return false;
        };
        let r = self.end_dispatch(i);
        if let Some(j) = self.index_of_id(id) {
            self.s.borrow_mut().records.remove(j);
        }
        r
    }

    /// Slot 8 `RevokeDispatchByPylon(p)`.
    pub fn revoke_dispatch_by_pylon(&self, p: u64) -> bool {
        let h = &*self.host;
        let id = self
            .s
            .borrow()
            .records
            .iter()
            .find(|r| r.pylon == Some(p))
            .map(|r| r.id);
        let r = match id {
            Some(id) => self.revoke_dispatch_by_id(id),
            None => false,
        };
        // DP8: in every case; with no source for the type it does nothing.
        self.return_pylon_to_station(self.get_dispatch_source(h.pylon_dispatch_type(p)), p);
        r
    }

    /// Slot 9 `RevokeAllDispatches(type)`: [quirk] every record, whatever its
    /// type. Returns true.
    pub fn revoke_all_dispatches(&self, _t: u32) -> bool {
        let ids: Vec<u32> = self.s.borrow().records.iter().map(|r| r.id).collect();
        for id in ids {
            self.revoke_dispatch_by_id(id);
        }
        true
    }

    /// `ReturnPylonToStation(source, p)` (§4.4): nothing without a source.
    fn return_pylon_to_station(&self, source: Option<u64>, p: u64) {
        let h = &*self.host;
        let Some(src) = source else { return };
        let Some(station) = h.pylon_source(p) else {
            // DP6 [choice]: shut down; the caller's release follows (the
            // original releases here too, an over-release).
            h.pylon_shutdown(p);
            return;
        };
        // The states only when the station's unit state was available; the
        // pylon joins the available list either way.
        if h.pylon_return_to_station(src, p, station) {
            h.pylon_set_dispatch_state(p, state::IDLE);
            h.pylon_set_ui_state(p, UI_STATE_IDLE);
        }
        let listed = self.s.borrow().available_pylons.contains(&Some(p));
        if !listed {
            h.add_ref(p);
            self.s.borrow_mut().available_pylons.push(Some(p));
        }
    }

    // ---- the dispatch tool (§4.5) ----

    /// [question DP4] Every available pylon is shut down and released.
    fn clear_available_pylons(&self) {
        let h = &*self.host;
        let list = std::mem::take(&mut self.s.borrow_mut().available_pylons);
        for p in list.into_iter().flatten() {
            h.pylon_shutdown(p);
            h.release(p);
        }
    }

    /// Slot 16 `BeginDispatchTool(type)`.
    pub fn begin_dispatch_tool(&self, t: u32) -> bool {
        let h = &*self.host;
        if t == self.s.borrow().tool_type {
            return true;
        }
        let Some(src) = self.get_dispatch_source(t) else {
            return false;
        };
        self.s.borrow_mut().tool_type = t;
        self.clear_available_pylons();
        for st in h.source_stations(src) {
            for i in 0..st.count.min(32) {
                if st.mask & (1 << i) == 0 {
                    continue;
                }
                let Some(p) = h.create_pylon() else { continue };
                h.pylon_set_call_number(p, st.base.wrapping_add(i));
                h.pylon_set_dispatch_state(p, state::IDLE);
                h.pylon_set_dispatch_type(p, t);
                h.pylon_set_ui_state(p, UI_STATE_IDLE);
                h.pylon_set_source(p, st.station);
                h.pylon_place_at_unit(p, st.station, i, st.count);
                self.s.borrow_mut().available_pylons.push(Some(p));
            }
        }
        true
    }

    /// Slot 17 `EndDispatchTool(type)`.
    pub fn end_dispatch_tool(&self, t: u32) -> bool {
        if t != self.s.borrow().tool_type {
            return false;
        }
        self.clear_available_pylons();
        self.s.borrow_mut().tool_type = dispatch_type::NONE;
        true
    }

    /// Slot 19 `SetActivePylon(p)`: when it changes, the old pylon gets UI
    /// state 1 and is released; the new one is held and gets UI state 2.
    pub fn set_active_pylon(&self, p: Option<u64>) {
        let h = &*self.host;
        let old = self.s.borrow().active_pylon;
        if old == p {
            return;
        }
        if let Some(o) = old {
            h.pylon_set_ui_state(o, UI_STATE_IDLE);
            h.release(o);
        }
        if let Some(n) = p {
            h.add_ref(n);
            h.pylon_set_ui_state(n, UI_STATE_ACTIVE);
        }
        self.s.borrow_mut().active_pylon = p;
    }

    // ---- messages (§3) ----

    /// `DoMessage` (§3).
    pub fn do_message(&self, m: &DispatchMessage) -> bool {
        match m.msg_type {
            msg::REMOVE_OCCUPANT => {
                if let Some(o) = m.object {
                    self.station_removed(o);
                }
                true
            }
            msg::DISASTER_PERIOD_ENDED => {
                let vehicles: Vec<u64> = self.s.borrow().records.iter().filter_map(|r| r.automaton).collect();
                for a in vehicles {
                    self.host.automaton_go_home(a);
                }
                true
            }
            msg::VIEW_CHANGED => {
                let v = self.s.borrow().vehicle_view;
                self.host.refresh_vehicle_view(v);
                true
            }
            msg::AUTOMATON_EVENT => self.automaton_event(m.object, m.data2, m.data3),
            _ => true,
        }
    }

    /// A vehicle's `0x09D92FC3` event (§3).
    pub fn automaton_event(&self, automaton: Option<u64>, code: u32, data3: u32) -> bool {
        if code != msg::EVENT_GONE && code != msg::EVENT_STATE {
            return false;
        }
        let Some(a) = automaton else { return false };
        let Some(id) = self
            .s
            .borrow()
            .records
            .iter()
            .find(|r| r.automaton == Some(a))
            .map(|r| r.id)
        else {
            return true;
        };
        if code == msg::EVENT_GONE {
            self.revoke_dispatch_by_id(id);
        } else {
            self.update_dispatch_state(id, data3);
            if data3 == state::IDLE {
                self.revoke_dispatch_by_id(id);
            }
        }
        true
    }

    /// `0x99EF1143` (§3): [quirk] the station's records are dropped without
    /// `EndDispatch` (no message, no unit returned); then its available
    /// pylons go.
    pub fn station_removed(&self, o: u64) {
        let h = &*self.host;
        loop {
            let found = self.s.borrow().records.iter().position(|r| r.station == Some(o));
            let Some(i) = found else { break };
            let rec = self.s.borrow_mut().records.remove(i);
            if rec.view_handle != 0 {
                h.untrack_vehicle(rec.view_handle);
            }
            // DP7: despawned and released.
            if let Some(a) = rec.automaton {
                h.automaton_despawn(a);
                h.release(a);
            }
            if let Some(p) = rec.pylon {
                if self.s.borrow().active_pylon == Some(p) {
                    self.set_active_pylon(None);
                }
                h.pylon_shutdown(p);
                h.release(p);
            }
            h.release(o);
        }
        let avail = self.s.borrow().available_pylons.clone();
        for p in avail.into_iter().flatten() {
            if h.pylon_source(p) != Some(o) {
                continue;
            }
            // DP7: shut down, released, then cleared from active.
            h.pylon_shutdown(p);
            h.release(p);
            if self.s.borrow().active_pylon == Some(p) {
                self.set_active_pylon(None);
            }
            let mut s = self.s.borrow_mut();
            if let Some(i) = s.available_pylons.iter().position(|&e| e == Some(p)) {
                s.available_pylons.remove(i);
            }
        }
    }

    // ---- the record (§7) ----

    fn get_ref(&self, inp: &mut dyn InStream, iid: u32, acquired: &mut Vec<u64>) -> Option<Option<u64>> {
        let r = inp.get_ref_as(iid)?;
        if r.is_null() {
            return Some(None);
        }
        let t = self.host.resolve_ref(&r, iid);
        if let Some(t) = t {
            acquired.push(t);
        }
        Some(t)
    }

    fn parse(&self, inp: &mut dyn InStream, acquired: &mut Vec<u64>) -> Option<Parsed> {
        if inp.get_u16()? != RECORD_VERSION {
            return None;
        }
        let n = inp.get_u32()?;
        let mut records = Vec::new();
        for _ in 0..n {
            let dispatch_type = inp.get_u32()?;
            let id = inp.get_u32()?;
            let call_id = inp.get_u32()?;
            let active = inp.get_u8()? != 0;
            let target = (inp.get_f32()?, inp.get_f32()?, inp.get_f32()?);
            let station = self.get_ref(inp, IID_OCCUPANT, acquired)?;
            let automaton = self.get_ref(inp, IID_AUTOMATON, acquired)?;
            let pylon = self.get_ref(inp, IID_PYLON, acquired)?;
            records.push(DispatchInfo {
                dispatch_type,
                id,
                call_id,
                view_handle: 0,
                active,
                target,
                station,
                automaton,
                pylon,
            });
        }
        let next_id = inp.get_u32()?;
        let active = self.get_ref(inp, IID_PYLON, acquired)?;
        let m = inp.get_u32()?;
        let mut pylons = Vec::new();
        for _ in 0..m {
            pylons.push(self.get_ref(inp, IID_PYLON, acquired)?);
        }
        let tool_type = inp.get_u32()?;
        if inp.error() != 0 {
            return None;
        }
        Some(Parsed {
            records,
            next_id,
            active,
            pylons,
            tool_type,
        })
    }
}

impl SaveObject for DispatchManager {
    fn class_id(&self) -> u32 {
        CLSID_DISPATCH_MANAGER
    }

    /// §7. True unless the stream reports an error.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let h = &*self.host;
        let s = self.s.borrow().clone();
        let r = |t: Option<u64>| t.map_or(RefValue::Null, |t| h.object_ref(t));
        out.put_u16(RECORD_VERSION);
        out.put_u32(s.records.len() as u32);
        for rec in &s.records {
            out.put_u32(rec.dispatch_type);
            out.put_u32(rec.id);
            out.put_u32(rec.call_id);
            out.put_u8(u8::from(rec.active));
            out.put_f32(rec.target.0);
            out.put_f32(rec.target.1);
            out.put_f32(rec.target.2);
            out.put_ref(&r(rec.station));
            out.put_ref(&r(rec.automaton));
            out.put_ref(&r(rec.pylon));
        }
        out.put_u32(s.next_id);
        out.put_ref(&r(s.active_pylon));
        out.put_u32(s.available_pylons.len() as u32);
        for &p in &s.available_pylons {
            out.put_ref(&r(p));
        }
        out.put_u32(s.tool_type);
        out.error() == 0
    }

    /// §7: `loaded` is set first; a version other than 2 fails; the records
    /// are appended and the pylons pushed. [choice] Atomic otherwise.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        self.s.borrow_mut().loaded = true;
        let mut acquired = Vec::new();
        match self.parse(inp, &mut acquired) {
            Some(p) => {
                let mut s = self.s.borrow_mut();
                s.records.extend(p.records);
                s.next_id = p.next_id;
                s.active_pylon = p.active;
                s.available_pylons.extend(p.pylons);
                s.tool_type = p.tool_type;
                true
            }
            None => {
                for t in acquired {
                    self.host.release(t);
                }
                false
            }
        }
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
