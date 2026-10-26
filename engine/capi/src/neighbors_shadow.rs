// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The neighbours shadow (fc4.h "neighbours shadow"): our core's month on a
//! snapshot of the ORIGINAL simulator's inputs, without touching the game.
//!
//! The plugin reads, through the game's interfaces only, what the original
//! simulator's month reads: its connections and deals, the regional cities
//! around the current city (position, size, established, budget, income,
//! the five commodity statistics), their connections, the city's utility
//! figures, the landfill usage, the date and the tuning exemplar. It builds a
//! shadow with [`fc4_neighbors_shadow_create`], which is an ordinary
//! `fc4_neighbors*` whose host is the snapshot: every `fc4_neighbors_*` query
//! works on it, and `fc4_neighbors_do_message` runs the month.
//!
//! The snapshot host answers the region from the snapshot and keeps every
//! write in it (an overlay): statistics, budgets, the L lists of
//! `SetLocalDeals`. Status reports are counted, not posted; a **sent** report
//! is delivered to the shadow simulator at once, as the message server does
//! (§5.3), so the retry report of §7.2 is reproduced.
//!
//! `UpdateImportExport` (foundations §11.3) sums the O and the L lists of a
//! city. O is not readable through any interface. *[choice]* The shadow
//! takes O's contribution as what the city's stored imported/exported values
//! hold beyond the contribution of the L list the original simulator last
//! pushed (its deal list as it is now): `base = stored − L(before)`, and
//! afterwards `value = base + L(now)`. This is exact while the stored values
//! are the result of the original's last push of the same list.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use fc4_core::neighbors::{
    msg, CityHandle, CityId, ConnectionState, DealData, ForeignConnection, NeighborConnection, NeighborDeal,
    NeighborsHost, NeighborsMessage, NeighborsSimulator, Stat, UtilityFigures,
};
use fc4_core::property::PropertyHolder;

use crate::guard;
use crate::neighbors::{
    fc4_neighbor_connection_state, fc4_neighbor_deal_data, fc4_neighbors, fc4_neighbors_figures, handle_ref,
    shadow_handle,
};
use crate::props::{fc4_props, props_ref};

/// A regional city of the snapshot (fc4.h `fc4_neighbors_shadow_city`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_neighbors_shadow_city {
    pub token: u64,
    pub pos_x: i32,
    pub pos_z: i32,
    pub size_x: i32,
    pub size_z: i32,
    pub established: u8,
    pub reserved: [u8; 7],
    pub budget: f64,
    pub income: f64,
    /// `[stat][commodity − 1]`; stat 0 exported, 1 imported, 2 produced, 3
    /// demanded, 4 cost per unit.
    pub stats: [[f64; 3]; 5],
}

/// A connection of the snapshot (fc4.h `fc4_neighbors_shadow_conn`): held by
/// regional city `city` (the current city's are the original simulator's
/// list, in order).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_neighbors_shadow_conn {
    pub city: u64,
    pub token: u64,
    pub state: fc4_neighbor_connection_state,
}

/// The snapshot (fc4.h `fc4_neighbors_shadow_input`). Everything is copied.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_neighbors_shadow_input {
    pub struct_size: u32,
    pub today: u32,
    pub count_x: i32,
    pub count_z: i32,
    pub current_city: u64,
    pub city_object: u64,
    pub landfill_usage: u32,
    pub landfill_capacity: u32,
    /// The original's state bytes as the shim read them (neighbors.md §15.4
    /// NG7): ready `+0x0C` (ConnectionsReady seen), uiDisabled `+0x3E`,
    /// dealsDirty `+0x3C`, statsDirty `+0x3D`, busy `+0x3F`.
    pub ready: u8,
    pub ui_disabled: u8,
    pub two_curves: u8,
    /// ABI 45 (0 before: the earlier fixed values).
    pub deals_dirty: u8,
    pub stats_dirty: u8,
    pub busy: u8,
    pub reserved: [u8; 2],
    pub figures: fc4_neighbors_figures,
    /// The tuning exemplar's properties (borrowed; copied), NULL = missing.
    pub tuning: *const fc4_props,
    pub cities: *const fc4_neighbors_shadow_city,
    pub city_count: usize,
    pub conns: *const fc4_neighbors_shadow_conn,
    pub conn_count: usize,
    /// The original simulator's deals (`FindDealsAll`), in order.
    pub deals: *const fc4_neighbor_deal_data,
    pub deal_count: usize,
}

/// What the shadow's month did instead of acting (fc4.h
/// `fc4_neighbors_shadow_counters`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_neighbors_shadow_counters {
    pub posts: u32,
    pub sends: u32,
    pub retries: u32,
    pub connection_messages: u32,
    pub set_local_deals: u32,
    pub update_import_export: u32,
    pub history: u32,
    pub reserved: u32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_neighbors_shadow_city>() == 168);
    assert!(std::mem::size_of::<fc4_neighbors_shadow_conn>() == 40);
    assert!(
        std::mem::size_of::<fc4_neighbors_shadow_input>() == if cfg!(target_pointer_width = "32") { 208 } else { 232 }
    );
    assert!(std::mem::size_of::<fc4_neighbors_shadow_counters>() == 32);
};

fn stat_index(s: Stat) -> usize {
    match s {
        Stat::Exported => 0,
        Stat::Imported => 1,
        Stat::Produced => 2,
        Stat::Demanded => 3,
        Stat::CostPerUnit => 4,
    }
}

/// The O-list part of a city's imported (`[0]`) and exported (`[1]`), per
/// commodity.
type Base = [[f64; 3]; 2];

/// The snapshot host.
pub(crate) struct ShadowHost {
    today: u32,
    counts: (i32, i32),
    current: Option<CityId>,
    city_object: Option<CityHandle>,
    landfill: (u32, u32),
    figures: UtilityFigures,
    tuning: Option<Rc<PropertyHolder>>,
    cities: RefCell<Vec<fc4_neighbors_shadow_city>>,
    conns: Vec<fc4_neighbors_shadow_conn>,
    /// The current city's connections (the simulator's list), live.
    own: RefCell<HashMap<u64, ConnectionState>>,
    base: RefCell<HashMap<CityId, Base>>,
    local: RefCell<HashMap<CityId, Vec<Rc<NeighborDeal>>>>,
    sim: RefCell<Weak<NeighborsSimulator>>,
    counters: Cell<fc4_neighbors_shadow_counters>,
}

impl ShadowHost {
    fn city_index(&self, city: CityId) -> Option<usize> {
        self.cities.borrow().iter().position(|c| c.token == city)
    }

    fn with_city<R>(&self, city: CityId, fallback: R, f: impl FnOnce(&mut fc4_neighbors_shadow_city) -> R) -> R {
        let Some(i) = self.city_index(city) else {
            return fallback;
        };
        f(&mut self.cities.borrow_mut()[i])
    }

    fn lookup(&self, rx: i32, rz: i32) -> Option<CityId> {
        self.cities
            .borrow()
            .iter()
            .find(|c| rx >= c.pos_x && rx < c.pos_x + c.size_x && rz >= c.pos_z && rz < c.pos_z + c.size_z)
            .map(|c| c.token)
    }

    fn count(&self, f: impl FnOnce(&mut fc4_neighbors_shadow_counters)) {
        let mut c = self.counters.get();
        f(&mut c);
        self.counters.set(c);
    }

    /// foundations §11.3 `extract`: whether deal `d` belongs to city `this`'s
    /// L list (`None` = a region cell without a city: removed).
    fn claims(&self, this: CityId, d: &DealData) -> Option<bool> {
        let a = self.lookup(d.city_side.0, d.city_side.1)?;
        let b = self.lookup(d.other_side.0, d.other_side.1)?;
        let cur = self.current?;
        Some(if this == cur {
            a == cur || b == cur
        } else {
            (a == cur && b == this) || (a == this && b == cur)
        })
    }

    /// One deal's part of `UpdateImportExport` for city `this` (foundations
    /// §11.3): `(commodity index, imported, exported)`.
    fn contribution(&self, this: CityId, d: &DealData) -> Option<(usize, f64, f64)> {
        const SKIP: u32 = 0xBC;
        let other = self.lookup(d.other_side.0, d.other_side.1)?;
        if d.flags & SKIP != 0 {
            return None;
        }
        let c = fc4_core::neighbors::deal_type::commodity(d.deal_type)? as usize - 1;
        let amount = f64::from(d.amount);
        let buy = d.deal_type & 1 == 0;
        let to_this = other == this;
        Some(match (buy, to_this) {
            (true, true) | (false, false) => (c, 0.0, amount),
            (true, false) | (false, true) => (c, amount, 0.0),
        })
    }

    fn sum(&self, this: CityId, deals: &[DealData]) -> Base {
        let mut s: Base = [[0.0; 3]; 2];
        for d in deals {
            if let Some((c, imp, exp)) = self.contribution(this, d) {
                s[0][c] += imp;
                s[1][c] += exp;
            }
        }
        s
    }

    /// `base = stored − L(before)` for every city of the snapshot.
    fn init_base(&self, before: &[DealData]) {
        let tokens: Vec<CityId> = self.cities.borrow().iter().map(|c| c.token).collect();
        for city in tokens {
            let l: Vec<DealData> = before
                .iter()
                .filter(|d| self.claims(city, d) == Some(true))
                .cloned()
                .collect();
            let part = self.sum(city, &l);
            let stored = self.with_city(city, [[0.0; 3]; 2], |c| [c.stats[1], c.stats[0]]);
            let mut b: Base = [[0.0; 3]; 2];
            for k in 0..2 {
                for c in 0..3 {
                    b[k][c] = stored[k][c] - part[k][c];
                }
            }
            self.base.borrow_mut().insert(city, b);
        }
    }

    fn import_export(&self, city: CityId) {
        let base = self.base.borrow().get(&city).copied().unwrap_or([[0.0; 3]; 2]);
        let l: Vec<DealData> = self
            .local
            .borrow()
            .get(&city)
            .map(|v| v.iter().map(|d| d.data()).collect())
            .unwrap_or_default();
        let part = self.sum(city, &l);
        self.with_city(city, (), |c| {
            for k in 0..3 {
                c.stats[1][k] = base[0][k] + part[0][k];
                c.stats[0][k] = base[1][k] + part[1][k];
            }
        });
    }
}

impl NeighborsHost for ShadowHost {
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.clone()
    }
    fn add_agent(&self, _level: u32, _name: &str, _flags: u32) {}
    fn remove_agents(&self) {}
    fn subscribe(&self, _msg_type: u32) {}
    fn unsubscribe(&self, _msg_type: u32) {}
    fn cell_counts(&self) -> (i32, i32) {
        self.counts
    }
    fn today(&self) -> u32 {
        self.today
    }
    fn utility_figures(&self) -> UtilityFigures {
        self.figures
    }
    fn landfill_usage(&self) -> (u32, u32) {
        self.landfill
    }
    fn send_history(&self, _series: u32, _value: i32) {
        self.count(|c| c.history += 1);
    }
    fn current_city(&self) -> Option<CityId> {
        self.current
    }
    fn current_city_object(&self) -> Option<CityHandle> {
        self.city_object
    }
    fn city_at(&self, rx: i32, rz: i32) -> Option<CityId> {
        self.lookup(rx, rz)
    }
    fn city_position(&self, city: CityId) -> (i32, i32) {
        self.with_city(city, (0, 0), |c| (c.pos_x, c.pos_z))
    }
    fn city_size(&self, city: CityId) -> (i32, i32) {
        self.with_city(city, (0, 0), |c| (c.size_x, c.size_z))
    }
    fn city_established(&self, city: CityId) -> bool {
        self.with_city(city, false, |c| c.established != 0)
    }
    fn city_stat(&self, city: CityId, stat: Stat, commodity: i32) -> f64 {
        if !(1..=3).contains(&commodity) {
            return 0.0;
        }
        self.with_city(city, 0.0, |c| c.stats[stat_index(stat)][commodity as usize - 1])
    }
    fn set_city_stat(&self, city: CityId, stat: Stat, commodity: i32, value: f64) {
        if !(1..=3).contains(&commodity) {
            return;
        }
        self.with_city(city, (), |c| c.stats[stat_index(stat)][commodity as usize - 1] = value)
    }
    fn city_budget(&self, city: CityId) -> f64 {
        self.with_city(city, 0.0, |c| c.budget)
    }
    fn set_city_budget(&self, city: CityId, value: f64) {
        self.with_city(city, (), |c| c.budget = value)
    }
    fn set_city_income(&self, city: CityId, value: f64) {
        self.with_city(city, (), |c| c.income = value)
    }
    fn city_find_connection(&self, city: CityId, conn_type: u32, x: i32, z: i32) -> Option<ForeignConnection> {
        if Some(city) == self.current {
            let own = self.own.borrow();
            return self.conns.iter().filter(|c| c.city == city).find_map(|c| {
                let s = own.get(&c.token)?;
                (s.conn_type == conn_type && s.cell == (x, z)).then_some(ForeignConnection {
                    token: c.token,
                    active: s.active,
                })
            });
        }
        self.conns
            .iter()
            .find(|c| c.city == city && c.state.conn_type == conn_type && (c.state.x, c.state.z) == (x, z))
            .map(|c| ForeignConnection {
                token: c.token,
                active: c.state.active != 0,
            })
    }
    fn city_connections(&self, city: CityId) -> Vec<(u32, (i32, i32))> {
        self.conns
            .iter()
            .filter(|c| c.city == city)
            .map(|c| (c.state.conn_type, (c.state.other_x, c.state.other_z)))
            .collect()
    }
    fn change_symmetric_connection(&self, _city: CityId, _conn: &Rc<NeighborConnection>, _add: bool) {}
    fn reload_city(&self, _city: CityId) {}
    fn update_local_deals(&self, _city: CityId) {}
    fn local_deals(&self, _city: CityId) -> Vec<DealData> {
        Vec::new()
    }
    fn set_local_deals(&self, city: CityId, deals: &mut Vec<Rc<NeighborDeal>>) {
        self.count(|c| c.set_local_deals += 1);
        let mut l = Vec::new();
        deals.retain(|d| match self.claims(city, &d.data()) {
            None => false,
            Some(claimed) => {
                if claimed {
                    l.push(d.clone());
                }
                true
            }
        });
        self.local.borrow_mut().insert(city, l);
        // SetLocalDeals runs UpdateImportExport itself (foundations §11.3).
        self.import_export(city);
    }
    fn update_import_export(&self, city: CityId) {
        self.count(|c| c.update_import_export += 1);
        self.import_export(city);
    }
    fn post_status(&self, deal: Option<&Rc<NeighborDeal>>, status: u32) {
        self.count(|c| {
            c.posts += 1;
            if deal.is_none() && status == fc4_core::neighbors::status::RETRY {
                c.retries += 1;
            }
        });
    }
    fn send_status(&self, _deal: &Rc<NeighborDeal>, status: u32) {
        self.count(|c| c.sends += 1);
        let sim = self.sim.borrow().upgrade();
        if let Some(sim) = sim {
            sim.do_message(&NeighborsMessage {
                msg_type: msg::DEAL_STATUS,
                data1: status,
                ..NeighborsMessage::default()
            });
        }
    }
    fn post_connection_message(&self, _msg_type: u32, _conn_type: u32, _x: i32, _z: i32) {
        self.count(|c| c.connection_messages += 1);
    }
    fn cell_center(&self, x: i32, z: i32) -> (f32, f32) {
        (x as f32 * 16.0 + 8.0, z as f32 * 16.0 + 8.0)
    }
    fn marker_altitude(&self, _px: f32, _pz: f32) -> f32 {
        0.0
    }
    fn max_surface_level(&self, _x: i32, _z: i32) -> f32 {
        0.0
    }
    fn add_marker_prop(&self, _position: [f32; 3], _orientation: i32, _bool_props: &[u32]) -> Option<u64> {
        None
    }
    fn marker_props(&self, _x: i32, _z: i32) -> Vec<u64> {
        Vec::new()
    }
    fn remove_prop(&self, _prop: u64) {}
    fn move_prop(&self, _prop: u64, _position: [f32; 3]) {}
    fn occupant_type(&self, _occupant: u64) -> u32 {
        0
    }
    fn occupant_networks(&self, _occupant: u64) -> Vec<u32> {
        Vec::new()
    }
    fn occupant_cell(&self, _occupant: u64) -> (i32, i32) {
        (0, 0)
    }
    fn power_pole_flag(&self, _occupant: u64) -> bool {
        false
    }
    fn foreign_connection(&self, token: u64) -> Option<ConnectionState> {
        self.own.borrow().get(&token).copied()
    }
    fn set_foreign_connection_flags(&self, token: u64, visited: Option<bool>, active: Option<bool>) {
        if let Some(s) = self.own.borrow_mut().get_mut(&token) {
            if let Some(v) = visited {
                s.visited = v;
            }
            if let Some(a) = active {
                s.active = a;
            }
        }
    }
}

/// SAFETY: NULL (with `n == 0`) or valid for `n` elements.
unsafe fn slice<'a, T>(p: *const T, n: usize) -> &'a [T] {
    if p.is_null() || n == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(p, n)
    }
}

/// A shadow on `input` (copied): an `fc4_neighbors*` (destroy it with
/// `fc4_neighbors_destroy`) whose simulator holds the original's connections
/// (as foreign connections with the snapshot's state) and deals (in order),
/// with the tuning loaded (`Init` on the snapshot), ConnectionsReady and the
/// UI flag as given. NULL for a bad input.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_shadow_create(input: *const fc4_neighbors_shadow_input) -> *mut fc4_neighbors {
    guard(std::ptr::null_mut(), || {
        let Some(i) = input.as_ref() else {
            return std::ptr::null_mut();
        };
        if (i.struct_size as usize) < std::mem::size_of::<fc4_neighbors_shadow_input>() {
            return std::ptr::null_mut();
        }
        let cities = slice(i.cities, i.city_count).to_vec();
        let conns = slice(i.conns, i.conn_count).to_vec();
        let deals: Vec<DealData> = slice(i.deals, i.deal_count).iter().map(|d| d.to_core()).collect();
        let current = (i.current_city != 0).then_some(i.current_city);
        let own: HashMap<u64, ConnectionState> = conns
            .iter()
            .filter(|c| Some(c.city) == current)
            .map(|c| (c.token, c.state.to_core()))
            .collect();
        let sh = Rc::new(ShadowHost {
            today: i.today,
            counts: (i.count_x, i.count_z),
            current,
            city_object: (i.city_object != 0).then_some(i.city_object),
            landfill: (i.landfill_usage, i.landfill_capacity),
            figures: i.figures.to_core(),
            tuning: props_ref(i.tuning).cloned().map(Rc::new),
            cities: RefCell::new(cities),
            conns: conns.clone(),
            own: RefCell::new(own),
            base: RefCell::new(HashMap::new()),
            local: RefCell::new(HashMap::new()),
            sim: RefCell::new(Weak::new()),
            counters: Cell::new(fc4_neighbors_shadow_counters::default()),
        });
        sh.init_base(&deals);
        let host: Rc<dyn NeighborsHost> = sh.clone();
        let sim = NeighborsSimulator::new(host.clone(), 0, 0);
        *sh.sim.borrow_mut() = Rc::downgrade(&sim);
        sim.set_two_curves(i.two_curves != 0);
        for c in conns.iter().filter(|c| Some(c.city) == current) {
            sim.adopt_connection(NeighborConnection::foreign(c.token, &host));
        }
        // Init loads the tuning and the price curve; its push only sets the
        // overlay's L lists, which the month's first push replaces.
        sim.init();
        // Init pushes an empty L list before the captured deals are adopted.
        // That is setup, not part of the original's month (which may be busy
        // and do nothing). Keep the original's pre-delivery city statistics.
        *sh.cities.borrow_mut() = slice(i.cities, i.city_count).to_vec();
        for d in deals {
            sim.adopt_deal(&NeighborDeal::from_data(d));
        }
        // NG7 (neighbors.md §15.4): the original's own bytes, not assumed.
        sim.set_ready(i.ready != 0);
        sim.set_ui_disable_flag(i.ui_disabled != 0);
        sim.set_dirty(i.deals_dirty != 0, i.stats_dirty != 0);
        sim.set_busy(i.busy != 0);
        sh.counters.set(fc4_neighbors_shadow_counters::default());
        shadow_handle(host, sim, sh)
    })
}

/// A city of the shadow's overlay (its values after the shadow's writes):
/// false for an unknown city or a handle that is not a shadow.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_shadow_city_get(
    p: *const fc4_neighbors,
    city: u64,
    out: *mut fc4_neighbors_shadow_city,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let Some(h) = handle_ref(p) else { return false };
    guard(false, || {
        let Some(sh) = &h.shadow else { return false };
        match sh.city_index(city) {
            Some(i) => {
                *o = sh.cities.borrow()[i];
                true
            }
            None => false,
        }
    })
}

/// The shadow's counters since it was created.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_shadow_get_counters(
    p: *const fc4_neighbors,
    out: *mut fc4_neighbors_shadow_counters,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let Some(h) = handle_ref(p) else { return false };
    guard(false, || match &h.shadow {
        Some(sh) => {
            *o = sh.counters.get();
            true
        }
        None => false,
    })
}
