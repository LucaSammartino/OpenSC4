// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network manager (§2.1, §3.1, §4.1, §6.1, §7).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::grid::CellRect;
use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::config::{corner_mismatch, terrain_rect};
use super::flags::flag;
use super::{
    msg, rect_is_empty, terrain_kind, NetworkManagerHost, NetworkMessage, CLSID_NETWORK_MANAGER, EMPTY_RECT,
    IID_NETWORK_OCCUPANT, MANAGER_RECORD_VERSION, TOOL_COUNT,
};

/// The destroy groups of §7.2 (tables `0x00AA7B40` / `0x00AA7B34`): (mask,
/// tool type).
pub const DESTROY_GROUPS: [(u32, u32); 3] = [(flag::SURFACE, 0), (flag::PIPE, 4), (flag::SUBWAY, 7)];

/// Every field of §2.1 the simulation sees.
#[derive(Clone, Debug)]
pub struct NetworkManagerState {
    pub initialized: bool,
    pub ref_count: u32,
    /// `Release` destroyed the object.
    pub destroyed: bool,
    /// Apply the UK path remapping (§7.11).
    pub uk_paths: bool,
    /// A long terrain operation is running (§7.6).
    pub long_terrain_op: bool,
    /// The accumulated terrain-change area.
    pub dirty: CellRect,
    /// The globally managed pieces, each held (`None`: a null reference read
    /// from a save).
    pub managed: Vec<Option<u64>>,
    /// Occupants queued for removal (§7.3), each held.
    pub pending: Vec<u64>,
    pub queueing: bool,
    /// Pieces that asked for the retrofit (§7.10), each held.
    pub retrofit: Vec<u64>,
    /// Construction crews (§7.12), each held.
    pub crews: Vec<u64>,
    /// The cached tools, one per network type, each held.
    pub tools: [Option<u64>; TOOL_COUNT],
}

impl Default for NetworkManagerState {
    /// The constructor's values (`0x00613330`).
    fn default() -> NetworkManagerState {
        NetworkManagerState {
            initialized: false,
            ref_count: 0,
            destroyed: false,
            uk_paths: false,
            long_terrain_op: false,
            dirty: EMPTY_RECT,
            managed: Vec::new(),
            pending: Vec::new(),
            queueing: false,
            retrofit: Vec::new(),
            crews: Vec::new(),
            tools: [None; TOOL_COUNT],
        }
    }
}

/// The network manager.
pub struct NetworkManager {
    host: Rc<dyn NetworkManagerHost>,
    token: Cell<u64>,
    s: RefCell<NetworkManagerState>,
}

impl NetworkManager {
    /// A constructed (not initialised) manager, reference count 0.
    pub fn new(host: Rc<dyn NetworkManagerHost>, token: u64) -> Rc<NetworkManager> {
        Rc::new(NetworkManager {
            host,
            token: Cell::new(token),
            s: RefCell::new(NetworkManagerState::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn snapshot(&self) -> NetworkManagerState {
        self.s.borrow().clone()
    }

    pub fn with_state<R>(&self, f: impl FnOnce(&mut NetworkManagerState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }

    // ---- identity (§1) ----

    pub fn add_ref(&self) -> u32 {
        let mut s = self.s.borrow_mut();
        s.ref_count = s.ref_count.wrapping_add(1);
        s.ref_count
    }

    /// `Release` (§1): above 1 it decrements; at 1 or below the object is
    /// deleted (here: marked) and 0 is returned.
    pub fn release(&self) -> u32 {
        let mut s = self.s.borrow_mut();
        if s.ref_count > 1 {
            s.ref_count -= 1;
            return s.ref_count;
        }
        s.ref_count = 0;
        s.destroyed = true;
        0
    }

    // ---- lifecycle (§3.1) ----

    /// Slot 3 `Init` (§3.1). Returns true.
    pub fn init(&self) -> bool {
        let h = &*self.host;
        {
            let mut s = self.s.borrow_mut();
            if s.initialized {
                return true;
            }
            s.initialized = true;
        }
        let uk = h.uk_paths();
        self.s.borrow_mut().uk_paths = uk;
        for m in msg::MANAGER_SUBSCRIPTIONS {
            h.subscribe(m);
        }
        self.s.borrow_mut().long_terrain_op = false;
        for t in 0..TOOL_COUNT {
            let tool = h.create_tool(t as u32);
            // Held: the creation reference is the cache's.
            let old = std::mem::replace(&mut self.s.borrow_mut().tools[t], tool);
            if let Some(o) = old {
                h.release(o);
            }
        }
        true
    }

    /// Slot 4 `Shutdown` (§3.1). `pending`, `crews` and `dirty` are left as
    /// they are. Returns true.
    pub fn shutdown(&self) -> bool {
        let h = &*self.host;
        let (managed, retrofit, tools) = {
            let mut s = self.s.borrow_mut();
            if !s.initialized {
                return true;
            }
            s.initialized = false;
            (
                std::mem::take(&mut s.managed),
                std::mem::take(&mut s.retrofit),
                std::mem::replace(&mut s.tools, [None; TOOL_COUNT]),
            )
        };
        for t in managed.into_iter().flatten() {
            h.release(t);
        }
        for t in retrofit {
            h.release(t);
        }
        // [inferred] Nulling a cached tool releases the cache's reference.
        for t in tools.into_iter().flatten() {
            h.release(t);
        }
        for m in msg::MANAGER_SUBSCRIPTIONS {
            h.unsubscribe(m);
        }
        true
    }

    // ---- messages (§6.1) ----

    /// `DoMessage` (`0x00613950`, §6.1). Returns true.
    pub fn do_message(&self, m: &NetworkMessage) -> bool {
        match m.msg_type {
            msg::POST_CITY_INIT => {
                self.run_retrofit();
                if self.host.is_night() {
                    self.light_managed(true);
                }
            }
            msg::PRE_CITY_SHUTDOWN => self.pre_city_shutdown(),
            msg::OCCUPANT_INSERTED => {
                if let Some(o) = m.void1 {
                    self.occupant_inserted(o);
                }
            }
            msg::OCCUPANT_REMOVED => {
                if let Some(o) = m.void1 {
                    self.occupant_removed(o);
                }
            }
            msg::DUSK => self.light_managed(true),
            msg::DAWN => self.light_managed(false),
            msg::VIEW_CHANGED => {
                if m.view_old != m.view_new {
                    self.host.view_rotation_changed();
                }
            }
            msg::WATER_DISPLAY_CHANGED => self.water_display_changed(),
            msg::LONG_TERRAIN_BEGIN => {
                let mut s = self.s.borrow_mut();
                s.dirty = EMPTY_RECT;
                s.long_terrain_op = true;
            }
            msg::TERRAIN_CHANGED => self.terrain_changed(m.data1, m.data2, m.data4),
            msg::LONG_TERRAIN_END => self.underground_refit(),
            _ => {}
        }
        true
    }

    /// `0x99EF1142` (§6.1): a globally managed piece is appended with the
    /// reference of the query (no duplicate check).
    pub fn occupant_inserted(&self, occupant: u64) {
        let h = &*self.host;
        let Some(n) = h.query_network_occupant(occupant) else {
            return;
        };
        if h.is_managed_globally(n) {
            self.s.borrow_mut().managed.push(Some(n));
        } else {
            h.release(n);
        }
    }

    /// `0x99EF1143` (§6.1): the first entry of a globally managed piece is
    /// released and replaced by the last entry (swap-and-pop).
    pub fn occupant_removed(&self, occupant: u64) {
        let h = &*self.host;
        let Some(n) = h.query_network_occupant(occupant) else {
            return;
        };
        if h.is_managed_globally(n) {
            let found = {
                let mut s = self.s.borrow_mut();
                match s.managed.iter().position(|e| *e == Some(n)) {
                    Some(i) => {
                        s.managed.swap_remove(i);
                        true
                    }
                    None => false,
                }
            };
            if found {
                h.release(n);
            }
        }
        h.release(n);
    }

    /// Dusk / dawn / PostCityInit (§6.1): `SetLit` on every managed piece
    /// that `CanBeLit`.
    fn light_managed(&self, lit: bool) {
        let h = &*self.host;
        let managed = self.s.borrow().managed.clone();
        for n in managed.into_iter().flatten() {
            if h.can_be_lit(n) {
                h.set_lit(n, lit);
            }
        }
    }

    /// `0x6AB97795` (§6.1): post `0x99EF1145` for every network occupant
    /// with network flag `0x40000` and occupant flag 4.
    fn water_display_changed(&self) {
        let h = &*self.host;
        for n in h.network_occupants() {
            if h.network_flags(n) & flag::UNKNOWN_40000 == flag::UNKNOWN_40000 && h.occupant_flag_set(n, 4) {
                h.post_refresh(n);
            }
        }
    }

    /// `0x26D31EC2` PreCityShutdown (§6.1, §7.12): slot 5 of the first crew
    /// while crews remain (at most the starting count of times [choice]);
    /// then every tool is dropped.
    pub fn pre_city_shutdown(&self) {
        let h = &*self.host;
        let n = self.s.borrow().crews.len();
        for _ in 0..n {
            let Some(&c) = self.s.borrow().crews.first() else {
                break;
            };
            h.crew_shutdown(c);
        }
        let tools = std::mem::replace(&mut self.s.borrow_mut().tools, [None; TOOL_COUNT]);
        for t in tools.into_iter().flatten() {
            h.release(t);
        }
    }

    // ---- tools (§7.1) ----

    /// Slot 5 `GetNetworkTool(type, unique)` (§7.1): the cached tool
    /// (borrowed), or a new one with the caller's reference.
    pub fn get_network_tool(&self, network_type: u32, unique: bool) -> Option<u64> {
        if network_type as usize >= TOOL_COUNT {
            return None;
        }
        if unique {
            self.host.create_tool(network_type)
        } else {
            self.s.borrow().tools[network_type as usize]
        }
    }

    // ---- removal (§7.2–§7.4) ----

    /// Slot 6 `DestroyNetworkOccupants(list)` (§7.2).
    pub fn destroy_network_occupants(&self, list: &[u64]) {
        let h = &*self.host;
        if self.s.borrow().queueing {
            for &o in list {
                h.add_ref(o);
                self.s.borrow_mut().pending.push(o);
            }
            return;
        }
        for (mask, tool_type) in DESTROY_GROUPS {
            let mut collected = Vec::new();
            for &o in list {
                if h.network_flags(o) & mask != 0 {
                    h.add_ref(o);
                    collected.push(o);
                }
            }
            if collected.is_empty() {
                continue;
            }
            // [choice] A null cached tool skips the group.
            if let Some(t) = self.get_network_tool(tool_type, false) {
                h.add_ref(t);
                h.tool_init(t);
                h.tool_destroy_occupants(t, &collected);
                h.tool_shutdown(t);
                h.release(t);
            }
            // [quirk] The collected references are not released.
        }
    }

    /// Slot 7 `QueueRemovals(on)` (§7.3).
    pub fn queue_removals(&self, on: bool) {
        let h = &*self.host;
        let pending = {
            let mut s = self.s.borrow_mut();
            if on != s.queueing {
                s.queueing = on;
            }
            if on || s.pending.is_empty() {
                return;
            }
            std::mem::take(&mut s.pending)
        };
        // [choice] Ordered by creation number (stable for equal numbers).
        let mut keyed: Vec<(u64, u64)> = pending.into_iter().map(|o| (h.creation_number(o), o)).collect();
        keyed.sort_by_key(|&(k, _)| k);
        let mut list: Vec<u64> = Vec::with_capacity(keyed.len());
        for (_, o) in keyed {
            if list.last() == Some(&o) {
                h.release(o);
            } else {
                list.push(o);
            }
        }
        self.destroy_network_occupants(&list);
        // Releasing the list releases its elements.
        for o in list {
            h.release(o);
        }
    }

    /// Slot 8 `RemoveOrReplaceOccupant(tool, occ, &replaced)` (§7.4): the
    /// result and whether `replaced` was set to true (it is not written
    /// otherwise).
    pub fn remove_or_replace_occupant(&self, tool: u64, occupant: Option<u64>) -> (bool, bool) {
        let h = &*self.host;
        let Some(o) = occupant else {
            return (false, false);
        };
        if h.occupant_flags(o) & 1 == 0 || !h.is_network_occupant(o) {
            return (false, false);
        }
        if h.network_flags(o) & flag::INTERSECTION != 0 {
            let (x, z) = h.occupied_cell(o);
            let info = h.tool_cell_info(tool, ((z as u32) << 16).wrapping_add(x as u32));
            if h.remove_occupant(o) {
                h.tool_reinsert_piece(tool, info);
                return (true, true);
            }
            return (false, false);
        }
        (h.remove_occupant(o), false)
    }

    // ---- small slots (§7.5) ----

    /// Slot 16 `RegisterConstructionCrew(c)`.
    pub fn register_construction_crew(&self, crew: u64) {
        self.host.add_ref(crew);
        self.s.borrow_mut().crews.push(crew);
    }

    /// Slot 17 `UnregisterConstructionCrew(c)`: find, erase, release.
    pub fn unregister_construction_crew(&self, crew: u64) {
        let found = {
            let mut s = self.s.borrow_mut();
            match s.crews.iter().position(|&c| c == crew) {
                Some(i) => {
                    s.crews.remove(i);
                    true
                }
                None => false,
            }
        };
        if found {
            self.host.release(crew);
        }
    }

    /// Slot 23 `AddToRetrofitList(n)`.
    pub fn add_to_retrofit_list(&self, n: u64) {
        self.host.add_ref(n);
        self.s.borrow_mut().retrofit.push(n);
    }

    /// `Retrofit` (§7.10): each entry still asking for it goes to the host
    /// (presentation); then the list is emptied and every entry released
    /// [question NM7].
    pub fn run_retrofit(&self) {
        let h = &*self.host;
        let list = std::mem::take(&mut self.s.borrow_mut().retrofit);
        for &n in &list {
            if h.check_for_retrofit(n) {
                h.retrofit(n);
            }
        }
        for n in list {
            h.release(n);
        }
    }

    // ---- terrain (§7.6) ----

    /// Whether any corner of the occupant's cell mismatches the terrain
    /// (`0x006105F0`).
    pub fn corners_mismatch(&self, n: u64) -> bool {
        let h = &*self.host;
        let (cx, cz) = h.occupied_cell(n);
        let x0 = cx.wrapping_mul(16) as f32;
        let z0 = cz.wrapping_mul(16) as f32;
        for (x, z) in [(x0, z0), (x0 + 16.0, z0), (x0, z0 + 16.0), (x0 + 16.0, z0 + 16.0)] {
            let d = h.occupant_altitude(n, x, z) - h.terrain_altitude(x, z);
            if corner_mismatch(d) {
                return true;
            }
        }
        false
    }

    /// `0x278E6003` (§7.6).
    pub fn terrain_changed(&self, data1: u32, data2: u32, kind: u32) {
        let h = &*self.host;
        let rect = terrain_rect(data1, data2, h.city_size());
        let long = {
            let mut s = self.s.borrow_mut();
            if s.long_terrain_op {
                let d = s.dirty;
                s.dirty = CellRect {
                    x1: d.x1.min(rect.x1),
                    z1: d.z1.min(rect.z1),
                    x2: d.x2.max(rect.x2),
                    z2: d.z2.max(rect.z2),
                };
            } else {
                s.dirty = rect;
            }
            s.long_terrain_op
        };
        if kind != terrain_kind::LEVELLING {
            let mut kept = Vec::new();
            for n in h.occupants_in_rect(rect, flag::ALL_TYPES) {
                if kind == terrain_kind::KEEP_ALL {
                    continue;
                }
                if kind != terrain_kind::UNDERGROUND && (h.is_of_type(n, 4) || h.is_of_type(n, 7)) {
                    continue;
                }
                if !self.corners_mismatch(n) {
                    continue;
                }
                if kind == terrain_kind::SKIP_80000 && h.network_flags(n) & flag::UNKNOWN_80000 == flag::UNKNOWN_80000 {
                    continue;
                }
                kept.push(n);
            }
            self.destroy_network_occupants(&kept);
        }
        if !long {
            self.underground_refit();
        }
    }

    /// `0x6A3B0AFD` and the tail of §7.6 step 4: the underground refit.
    pub fn underground_refit(&self) {
        let h = &*self.host;
        let dirty = self.s.borrow().dirty;
        if !rect_is_empty(&dirty) {
            let mut pipes: (Vec<(i32, i32)>, Vec<u64>) = (Vec::new(), Vec::new());
            let mut subways: (Vec<(i32, i32)>, Vec<u64>) = (Vec::new(), Vec::new());
            for n in h.occupants_in_rect(dirty, flag::PIPE_AND_SUBWAY) {
                let group = if h.is_of_type(n, 4) {
                    &mut pipes
                } else if h.is_of_type(n, 7) {
                    &mut subways
                } else {
                    continue;
                };
                h.add_ref(n);
                group.0.push(h.occupied_cell(n));
                group.1.push(n);
            }
            for (tool_type, (cells, list)) in [(4u32, &pipes), (7u32, &subways)] {
                // [choice] A null cached tool skips the kind. [question NM9]
                // Step 2 has no "if any" test (unlike §7.2), so both kinds
                // run even with an empty list.
                if let Some(t) = self.get_network_tool(tool_type, false) {
                    h.tool_init(t);
                    h.tool_update_underground_heights(t, cells, list, tool_type);
                    h.tool_shutdown(t);
                }
            }
            for n in pipes.1.into_iter().chain(subways.1) {
                h.release(n);
            }
            self.s.borrow_mut().dirty = EMPTY_RECT;
        }
        self.s.borrow_mut().long_terrain_op = false;
    }

    // ---- the record (§4.1) ----

    fn read_record(&self, inp: &mut dyn InStream) -> bool {
        if inp.error() != 0 {
            return false;
        }
        if inp.get_u16() != Some(MANAGER_RECORD_VERSION) {
            return false;
        }
        let Some(count) = inp.get_u32() else {
            return false;
        };
        for _ in 0..count {
            let Some(r) = inp.get_ref_as(IID_NETWORK_OCCUPANT) else {
                return false;
            };
            let entry = if r.is_null() {
                // [question NM6] A null reference reads as a null entry.
                None
            } else {
                match self.host.resolve_ref(&r, IID_NETWORK_OCCUPANT) {
                    Some(t) => Some(t),
                    None => return false,
                }
            };
            // Appended: the vector is not cleared first *[quirk]*.
            self.s.borrow_mut().managed.push(entry);
        }
        inp.error() == 0
    }
}

impl SaveObject for NetworkManager {
    fn class_id(&self) -> u32 {
        CLSID_NETWORK_MANAGER
    }

    /// §4.1 `Write` (`0x006111E0`): version 1, the count and `managed` in
    /// vector order.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let managed = self.s.borrow().managed.clone();
        out.put_u16(MANAGER_RECORD_VERSION);
        out.put_u32(managed.len() as u32);
        for e in managed {
            out.put_ref(&e.map_or(RefValue::Null, |t| self.host.object_ref(t)));
        }
        out.error() == 0
    }

    /// §4.1 `Read` (`0x006124C0`): appends to `managed`.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        self.read_record(inp)
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
