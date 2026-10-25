// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The neighbours simulator's C ABI (fc4.h "neighbours",
//! docs/specs/neighbors.md).
//!
//! The host implements [`fc4_neighbors_host`] (every `NeighborsHost` method of
//! the core, in the trait's order); the plugin's `cISC4NeighborsSimulator`
//! forwards its slots to the `fc4_neighbors_*` functions, its deals
//! (`cISC4NeighborDeal`) to `fc4_neighbor_deal_*` and its connections
//! (`cISC4NeighborConnection`) to `fc4_neighbor_connection_*`.
//!
//! Tokens are opaque host identities: regional cities (`cISC4RegionalCity*`),
//! the city object (`cISC4City*`), occupants, props, connections and deals. 0
//! is the null object.
//!
//! Objects (§15.2). A connection or deal of ours is a core object behind a
//! handle (`fc4_neighbor_connection*`, `fc4_neighbor_deal*`, each a strong
//! reference); the host's GZCOM object owns its handle and destroys it when
//! it dies. The core asks for the object when it first needs one
//! (`connection_object` for a new connection, `deal_object` when a deal is
//! first handed to the host) and keeps its token. A foreign connection (an
//! original object accepted on load) is read and written through the
//! `foreign_connection` callbacks. A foreign deal (a caller's object handed
//! to `AddDeal`, NG4) is kept as the listed deal: the host holds one
//! reference for the core, the core reads it through `foreign_deal`, writes
//! and drives it through `foreign_deal_call`, and lets it go with
//! `release_deal` (`fc4_neighbors_add_foreign_deal`). Foreign deals the
//! region hands over on load still reach the core as data (`local_deals`).
//!
//! Re-entrancy: the core holds no borrow across a host call and is meant to
//! be re-entered (a status report **sent** by `SetAmount` reaches the
//! simulator synchronously, §5.3, and a running deal simulation answers it
//! with the retry report, §7.2). So, unlike the plumbing glue, a message
//! delivered during another call is handled at once.

use std::cell::Cell;
use std::ffi::{c_char, c_void};
use std::rc::Rc;

use fc4_core::neighbors::{
    CityHandle, CityId, ConnectionRequest, ConnectionState, DealCall, DealData, ForeignConnection, NeighborConnection,
    NeighborDeal, NeighborsHost, NeighborsMessage, NeighborsSimulator, Stat, UtilityFigures, CLSID_NEIGHBOR_CONNECTION,
};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::SaveObject;
use fc4_core::variant::{mem, Variant};

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::{native_variant, take_props, NativeVariant};
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

// ---- data -----------------------------------------------------------------------------------------

/// A deal's state (fc4.h `fc4_neighbor_deal_data`, §5.1; the property holder
/// is not part of it).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_neighbor_deal_data {
    pub deal_type: u32,
    pub other_x: i32,
    pub other_z: i32,
    pub city_x: i32,
    pub city_z: i32,
    pub amount: i32,
    pub inc_amount: i32,
    pub inc_amount2: i32,
    pub last_amount: i32,
    pub last_inc_amount: i32,
    pub amount_used: i32,
    pub flags: u32,
    pub cost: i64,
    pub start_date: u32,
    pub expiration_date: u32,
    pub current_date: u32,
    pub suspension_date: u32,
}

impl fc4_neighbor_deal_data {
    pub(crate) fn of(d: &DealData) -> fc4_neighbor_deal_data {
        fc4_neighbor_deal_data {
            deal_type: d.deal_type,
            other_x: d.other_side.0,
            other_z: d.other_side.1,
            city_x: d.city_side.0,
            city_z: d.city_side.1,
            amount: d.amount,
            inc_amount: d.inc_amount,
            inc_amount2: d.inc_amount2,
            last_amount: d.last_amount,
            last_inc_amount: d.last_inc_amount,
            amount_used: d.amount_used,
            flags: d.flags,
            cost: d.cost,
            start_date: d.start_date,
            expiration_date: d.expiration_date,
            current_date: d.current_date,
            suspension_date: d.suspension_date,
        }
    }

    pub(crate) fn to_core(self) -> DealData {
        DealData {
            deal_type: self.deal_type,
            other_side: (self.other_x, self.other_z),
            city_side: (self.city_x, self.city_z),
            amount: self.amount,
            inc_amount: self.inc_amount,
            inc_amount2: self.inc_amount2,
            last_amount: self.last_amount,
            last_inc_amount: self.last_inc_amount,
            amount_used: self.amount_used,
            cost: self.cost,
            start_date: self.start_date,
            expiration_date: self.expiration_date,
            current_date: self.current_date,
            suspension_date: self.suspension_date,
            flags: self.flags,
            props: PropertyHolder::new(),
        }
    }
}

/// A connection's state (fc4.h `fc4_neighbor_connection_state`, §4.1).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_neighbor_connection_state {
    pub conn_type: u32,
    pub x: i32,
    pub z: i32,
    pub other_x: i32,
    pub other_z: i32,
    pub visited: u8,
    pub active: u8,
    pub reserved: [u8; 2],
}

impl fc4_neighbor_connection_state {
    pub(crate) fn of(s: &ConnectionState) -> fc4_neighbor_connection_state {
        fc4_neighbor_connection_state {
            conn_type: s.conn_type,
            x: s.cell.0,
            z: s.cell.1,
            other_x: s.other_region_cell.0,
            other_z: s.other_region_cell.1,
            visited: u8::from(s.visited),
            active: u8::from(s.active),
            reserved: [0; 2],
        }
    }

    pub(crate) fn to_core(self) -> ConnectionState {
        ConnectionState {
            conn_type: self.conn_type,
            cell: (self.x, self.z),
            other_region_cell: (self.other_x, self.other_z),
            visited: self.visited != 0,
            active: self.active != 0,
        }
    }
}

/// The figures `UpdateCommodityStats` reads (fc4.h `fc4_neighbors_figures`,
/// §8.2). A department is missing when its `has_*` byte is 0.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_neighbors_figures {
    pub total_funds: f64,
    pub monthly_income: f64,
    pub power_production: f64,
    pub power_requirement: f64,
    pub water_capacity: f64,
    pub water_demand: f64,
    pub garbage_recycled: f64,
    pub waste_to_energy_capacity: f64,
    pub incinerator_capacity: f64,
    pub garbage_abducted: f64,
    pub garbage_produced: f64,
    pub has_power_department: u8,
    pub has_water_department: u8,
    pub has_sanitation_department: u8,
    pub reserved: [u8; 5],
    pub power_department_expenses: f64,
    pub water_department_expenses: f64,
    pub sanitation_department_expenses: f64,
    pub waste_to_energy_expenses: f64,
}

impl fc4_neighbors_figures {
    pub(crate) fn to_core(self) -> UtilityFigures {
        let opt = |has: u8, v: f64| (has != 0).then_some(v);
        UtilityFigures {
            total_funds: self.total_funds,
            monthly_income: self.monthly_income,
            power_production: self.power_production,
            power_requirement: self.power_requirement,
            water_capacity: self.water_capacity,
            water_demand: self.water_demand,
            garbage_recycled: self.garbage_recycled,
            waste_to_energy_capacity: self.waste_to_energy_capacity,
            incinerator_capacity: self.incinerator_capacity,
            garbage_abducted: self.garbage_abducted,
            garbage_produced: self.garbage_produced,
            power_department_expenses: opt(self.has_power_department, self.power_department_expenses),
            water_department_expenses: opt(self.has_water_department, self.water_department_expenses),
            sanitation_department_expenses: opt(self.has_sanitation_department, self.sanitation_department_expenses),
            waste_to_energy_expenses: self.waste_to_energy_expenses,
        }
    }
}

/// A message for `fc4_neighbors_do_message` (fc4.h `fc4_neighbors_message`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_neighbors_message {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    /// `GetVoid1()` as a `cISC4Occupant` (PreDemolishNetworkOccupant,
    /// RemoveOccupant); 0 = none.
    pub occupant: u64,
}

/// Diagnostics (fc4.h `fc4_neighbors_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_neighbors_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub ready: u8,
    pub deals_dirty: u8,
    pub stats_dirty: u8,
    pub ui_disabled: u8,
    pub busy: u8,
    pub reserved0: [u8; 2],
    pub connections: u32,
    pub foreign_connections: u32,
    pub deals: u32,
    pub reserved1: u32,
    /// `GetTotalMonthlyNeighborDealIncome` / `...Expense` (§10.1).
    pub income: i64,
    pub expense: i64,
}

/// One `(type, other-side region cell)` of a regional city's connections.
pub type fc4_neighbor_link_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, conn_type: u32, rx: i32, rz: i32)>;
/// One deal of the region (`GetLocalDeals`), as data.
pub type fc4_neighbor_deal_data_callback =
    Option<unsafe extern "C" fn(ctx: *mut c_void, data: *const fc4_neighbor_deal_data)>;
/// One property of a connection's holder, in the game's native variant
/// layout (as `fc4_props_set_typed` takes it).
pub type fc4_neighbor_property_callback = Option<
    unsafe extern "C" fn(ctx: *mut c_void, id: u32, mem_type: u16, is_array: bool, data: *const c_void, count: usize),
>;

/// Everything the neighbours simulator asks of the game (fc4.h
/// `fc4_neighbors_host`; one callback per `NeighborsHost` method, in the
/// trait's order).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_neighbors_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // lifecycle
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const c_char, usize, u32)>,
    pub remove_agents: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    // city
    pub cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    pub today: Option<unsafe extern "C" fn(U) -> u32>,
    pub utility_figures: Option<unsafe extern "C" fn(U, *mut fc4_neighbors_figures)>,
    pub landfill_usage: Option<unsafe extern "C" fn(U, *mut u32, *mut u32)>,
    pub send_history: Option<unsafe extern "C" fn(U, u32, i32)>,
    // region
    pub current_city: Option<unsafe extern "C" fn(U) -> u64>,
    pub current_city_object: Option<unsafe extern "C" fn(U) -> u64>,
    pub city_at: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    pub city_position: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    pub city_size: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    pub city_established: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// `stat`: 0 exported, 1 imported, 2 produced, 3 demanded, 4 cost per unit.
    pub city_stat: Option<unsafe extern "C" fn(U, u64, u32, i32) -> f64>,
    pub set_city_stat: Option<unsafe extern "C" fn(U, u64, u32, i32, f64)>,
    pub city_budget: Option<unsafe extern "C" fn(U, u64) -> f64>,
    pub set_city_budget: Option<unsafe extern "C" fn(U, u64, f64)>,
    pub set_city_income: Option<unsafe extern "C" fn(U, u64, f64)>,
    /// `city.FindConnection(type, x, z)`: the connection (borrowed) and its
    /// `GetIsActive()`; 0 = none.
    pub city_find_connection: Option<unsafe extern "C" fn(U, u64, u32, i32, i32, *mut bool) -> u64>,
    pub city_connections: Option<unsafe extern "C" fn(U, u64, fc4_neighbor_link_callback, *mut c_void)>,
    pub change_symmetric_connection: Option<unsafe extern "C" fn(U, u64, u64, bool)>,
    pub reload_city: Option<unsafe extern "C" fn(U, u64)>,
    pub update_local_deals: Option<unsafe extern "C" fn(U, u64)>,
    pub local_deals: Option<unsafe extern "C" fn(U, u64, fc4_neighbor_deal_data_callback, *mut c_void)>,
    /// `SetLocalDeals(list)` with the deal objects `deals[0..count]`; the host
    /// sets `kept[i]` to 0 for every deal the region removed from the list.
    pub set_local_deals: Option<unsafe extern "C" fn(U, u64, *const u64, usize, *mut u8)>,
    pub update_import_export: Option<unsafe extern "C" fn(U, u64)>,
    // messages
    /// A status report (§5.3); deal 0 for the retry report.
    pub post_status: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub send_status: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub post_connection_message: Option<unsafe extern "C" fn(U, u32, u32, i32, i32)>,
    // markers
    pub cell_center: Option<unsafe extern "C" fn(U, i32, i32, *mut f32, *mut f32)>,
    pub marker_altitude: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    pub max_surface_level: Option<unsafe extern "C" fn(U, i32, i32) -> f32>,
    /// `position` is 3 floats; `bool_props[0..count]` are set true. Returns
    /// the prop (0 = none).
    pub add_marker_prop: Option<unsafe extern "C" fn(U, *const f32, i32, *const u32, usize) -> u64>,
    pub marker_props: Option<unsafe extern "C" fn(U, i32, i32, fc4_token_callback, *mut c_void)>,
    pub remove_prop: Option<unsafe extern "C" fn(U, u64)>,
    pub move_prop: Option<unsafe extern "C" fn(U, u64, *const f32)>,
    // occupants
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// The network type numbers, one per callback (as tokens).
    pub occupant_networks: Option<unsafe extern "C" fn(U, u64, fc4_token_callback, *mut c_void)>,
    pub occupant_cell: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    pub power_pole_flag: Option<unsafe extern "C" fn(U, u64) -> bool>,
    // objects (§15.2)
    /// The GZCOM object for a new connection of ours: the host takes the NEW
    /// handle (it destroys it when the object dies) and returns the object's
    /// token, holding one reference for the simulator's list; 0 = none (the
    /// core destroys the handle).
    pub connection_object: Option<unsafe extern "C" fn(U, *mut fc4_neighbor_connection) -> u64>,
    /// The GZCOM object for a deal of ours, made the first time the deal is
    /// handed to the host: the host takes the NEW handle and returns the
    /// object's token (no reference is held for the core); 0 = none.
    pub deal_object: Option<unsafe extern "C" fn(U, *mut fc4_neighbor_deal) -> u64>,
    pub foreign_connection: Option<unsafe extern "C" fn(U, u64, *mut fc4_neighbor_connection_state) -> bool>,
    /// `SetVisitedThisCycle` / `SetIsActive` on a foreign connection: -1 =
    /// leave, 0 = false, 1 = true.
    pub set_foreign_connection_flags: Option<unsafe extern "C" fn(U, u64, i32, i32)>,
    pub repair_foreign_connection: Option<unsafe extern "C" fn(U, u64, u64)>,
    /// The simulator's list let go of a connection: release its reference.
    pub release_connection: Option<unsafe extern "C" fn(U, u64)>,
    // foreign deals (NG4; ABI 45, appended)
    /// A foreign deal's state, read through its interface; false = unreadable.
    pub foreign_deal: Option<unsafe extern "C" fn(U, u64, *mut fc4_neighbor_deal_data) -> bool>,
    /// A call on a foreign deal (`FC4_NEIGHBOR_DEAL_CALL_*`): the setters
    /// 0-11 numbered as in `fc4_neighbor_deal_set_field` (`a`, `b`), 12 =
    /// `SetAmount(a)`, 13 = `UpdateCost`, 14 = `UpdateState`, 15 = `Cancel`,
    /// 16 = `Activate`, 17 = `SetFlags(a, b != 0)`. Returns the call's bool
    /// (`Activate`, `SetFlags`); true for the void ones.
    pub foreign_deal_call: Option<unsafe extern "C" fn(U, u64, u32, i64, i64) -> bool>,
    /// The core let go of a foreign deal: release the reference
    /// `fc4_neighbors_add_foreign_deal` was given.
    pub release_deal: Option<unsafe extern "C" fn(U, u64)>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_neighbor_deal_data>() == 72);
    assert!(std::mem::size_of::<fc4_neighbor_connection_state>() == 24);
    assert!(std::mem::size_of::<fc4_neighbors_figures>() == 128);
    assert!(std::mem::size_of::<fc4_neighbors_message>() == 24);
    assert!(std::mem::size_of::<fc4_neighbors_state>() == 48);
    assert!(std::mem::size_of::<fc4_neighbors_host>() == if cfg!(target_pointer_width = "32") { 216 } else { 432 });
};

// ---- object handles -------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_neighbor_deal`): a strong reference to a core
/// deal and the token of the host object that owns the handle.
#[repr(C)]
pub struct fc4_neighbor_deal {
    _private: [u8; 0],
}

/// Opaque handle (fc4.h `fc4_neighbor_connection`).
#[repr(C)]
pub struct fc4_neighbor_connection {
    _private: [u8; 0],
}

pub(crate) struct DealHandle {
    pub(crate) deal: Rc<NeighborDeal>,
    token: Cell<u64>,
}

pub(crate) struct ConnHandle {
    pub(crate) conn: Rc<NeighborConnection>,
    token: Cell<u64>,
}

pub(crate) fn new_deal_handle(deal: Rc<NeighborDeal>) -> *mut fc4_neighbor_deal {
    Box::into_raw(Box::new(DealHandle {
        deal,
        token: Cell::new(0),
    })) as *mut fc4_neighbor_deal
}

fn new_conn_handle(conn: Rc<NeighborConnection>) -> *mut fc4_neighbor_connection {
    Box::into_raw(Box::new(ConnHandle {
        conn,
        token: Cell::new(0),
    })) as *mut fc4_neighbor_connection
}

/// SAFETY: NULL or a live handle.
pub(crate) unsafe fn deal_ref<'a>(h: *const fc4_neighbor_deal) -> Option<&'a DealHandle> {
    (h as *const DealHandle).as_ref()
}

/// SAFETY: NULL or a live handle.
unsafe fn conn_ref<'a>(h: *const fc4_neighbor_connection) -> Option<&'a ConnHandle> {
    (h as *const ConnHandle).as_ref()
}

// ---- host adapter ---------------------------------------------------------------------------------

struct CHost(fc4_neighbors_host);

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.0.$f { Some(f) => unsafe { f($s.0.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

fn stat_index(s: Stat) -> u32 {
    match s {
        Stat::Exported => 0,
        Stat::Imported => 1,
        Stat::Produced => 2,
        Stat::Demanded => 3,
        Stat::CostPerUnit => 4,
    }
}

unsafe extern "C" fn push_link(ctx: *mut c_void, t: u32, rx: i32, rz: i32) {
    if let Some(v) = (ctx as *mut Vec<(u32, (i32, i32))>).as_mut() {
        v.push((t, (rx, rz)));
    }
}

unsafe extern "C" fn push_deal_data(ctx: *mut c_void, d: *const fc4_neighbor_deal_data) {
    if let (Some(v), Some(d)) = ((ctx as *mut Vec<DealData>).as_mut(), d.as_ref()) {
        v.push(d.to_core());
    }
}

unsafe extern "C" fn push_u64(ctx: *mut c_void, t: u64) {
    if let Some(v) = (ctx as *mut Vec<u64>).as_mut() {
        v.push(t);
    }
}

impl CHost {
    /// The host object of a deal of ours, made on first use.
    fn deal_token(&self, d: &Rc<NeighborDeal>) -> u64 {
        if d.token() != 0 {
            return d.token();
        }
        let Some(f) = self.0.deal_object else { return 0 };
        let h = new_deal_handle(d.clone());
        // SAFETY: valid callback; the host takes the handle.
        let t = unsafe { f(self.0.user, h) };
        if t == 0 {
            // SAFETY: the host refused the handle, so it is still ours.
            unsafe { fc4_neighbor_deal_destroy(h) };
            return 0;
        }
        // SAFETY: the host keeps the handle alive while its object lives.
        if let Some(dh) = unsafe { deal_ref(h) } {
            dh.token.set(t);
        }
        d.set_token(t);
        t
    }
}

impl NeighborsHost for CHost {
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        cb!(
            self,
            add_agent,
            (),
            level,
            name.as_ptr() as *const c_char,
            name.len(),
            flags
        )
    }
    fn remove_agents(&self) {
        cb!(self, remove_agents, ())
    }
    fn subscribe(&self, msg_type: u32) {
        cb!(self, subscribe, (), msg_type)
    }
    fn unsubscribe(&self, msg_type: u32) {
        cb!(self, unsubscribe, (), msg_type)
    }
    fn cell_counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, cell_counts, (), &mut x, &mut z);
        (x, z)
    }
    fn today(&self) -> u32 {
        cb!(self, today, 0)
    }
    fn utility_figures(&self) -> UtilityFigures {
        let mut f = fc4_neighbors_figures::default();
        cb!(self, utility_figures, (), &mut f);
        f.to_core()
    }
    fn landfill_usage(&self) -> (u32, u32) {
        let (mut u, mut c) = (0, 0);
        cb!(self, landfill_usage, (), &mut u, &mut c);
        (u, c)
    }
    fn send_history(&self, series: u32, value: i32) {
        cb!(self, send_history, (), series, value)
    }
    fn current_city(&self) -> Option<CityId> {
        opt(cb!(self, current_city, 0))
    }
    fn current_city_object(&self) -> Option<CityHandle> {
        opt(cb!(self, current_city_object, 0))
    }
    fn city_at(&self, rx: i32, rz: i32) -> Option<CityId> {
        opt(cb!(self, city_at, 0, rx, rz))
    }
    fn city_position(&self, city: CityId) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, city_position, (), city, &mut x, &mut z);
        (x, z)
    }
    fn city_size(&self, city: CityId) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, city_size, (), city, &mut x, &mut z);
        (x, z)
    }
    fn city_established(&self, city: CityId) -> bool {
        cb!(self, city_established, false, city)
    }
    fn city_stat(&self, city: CityId, stat: Stat, commodity: i32) -> f64 {
        cb!(self, city_stat, 0.0, city, stat_index(stat), commodity)
    }
    fn set_city_stat(&self, city: CityId, stat: Stat, commodity: i32, value: f64) {
        cb!(self, set_city_stat, (), city, stat_index(stat), commodity, value)
    }
    fn city_budget(&self, city: CityId) -> f64 {
        cb!(self, city_budget, 0.0, city)
    }
    fn set_city_budget(&self, city: CityId, value: f64) {
        cb!(self, set_city_budget, (), city, value)
    }
    fn set_city_income(&self, city: CityId, value: f64) {
        cb!(self, set_city_income, (), city, value)
    }
    fn city_find_connection(&self, city: CityId, conn_type: u32, x: i32, z: i32) -> Option<ForeignConnection> {
        let mut active = false;
        let t = cb!(self, city_find_connection, 0, city, conn_type, x, z, &mut active);
        opt(t).map(|token| ForeignConnection { token, active })
    }
    fn city_connections(&self, city: CityId) -> Vec<(u32, (i32, i32))> {
        let mut v: Vec<(u32, (i32, i32))> = Vec::new();
        cb!(
            self,
            city_connections,
            (),
            city,
            Some(push_link),
            &mut v as *mut Vec<(u32, (i32, i32))> as *mut c_void
        );
        v
    }
    fn change_symmetric_connection(&self, city: CityId, conn: &Rc<NeighborConnection>, add: bool) {
        cb!(self, change_symmetric_connection, (), city, conn.token(), add)
    }
    fn reload_city(&self, city: CityId) {
        cb!(self, reload_city, (), city)
    }
    fn update_local_deals(&self, city: CityId) {
        cb!(self, update_local_deals, (), city)
    }
    fn local_deals(&self, city: CityId) -> Vec<DealData> {
        let mut v: Vec<DealData> = Vec::new();
        cb!(
            self,
            local_deals,
            (),
            city,
            Some(push_deal_data),
            &mut v as *mut Vec<DealData> as *mut c_void
        );
        v
    }
    fn set_local_deals(&self, city: CityId, deals: &mut Vec<Rc<NeighborDeal>>) {
        let tokens: Vec<u64> = deals.iter().map(|d| self.deal_token(d)).collect();
        let mut kept = vec![1u8; deals.len()];
        cb!(
            self,
            set_local_deals,
            (),
            city,
            tokens.as_ptr(),
            tokens.len(),
            kept.as_mut_ptr()
        );
        let mut i = 0;
        deals.retain(|_| {
            let k = kept[i] != 0;
            i += 1;
            k
        });
    }
    fn update_import_export(&self, city: CityId) {
        cb!(self, update_import_export, (), city)
    }
    fn post_status(&self, deal: Option<&Rc<NeighborDeal>>, status: u32) {
        let t = deal.map_or(0, |d| self.deal_token(d));
        cb!(self, post_status, (), t, status)
    }
    fn send_status(&self, deal: &Rc<NeighborDeal>, status: u32) {
        let t = self.deal_token(deal);
        cb!(self, send_status, (), t, status)
    }
    fn post_connection_message(&self, msg_type: u32, conn_type: u32, x: i32, z: i32) {
        cb!(self, post_connection_message, (), msg_type, conn_type, x, z)
    }
    fn cell_center(&self, x: i32, z: i32) -> (f32, f32) {
        let (mut px, mut pz) = (0.0f32, 0.0f32);
        cb!(self, cell_center, (), x, z, &mut px, &mut pz);
        (px, pz)
    }
    fn marker_altitude(&self, px: f32, pz: f32) -> f32 {
        cb!(self, marker_altitude, 0.0, px, pz)
    }
    fn max_surface_level(&self, x: i32, z: i32) -> f32 {
        cb!(self, max_surface_level, 0.0, x, z)
    }
    fn add_marker_prop(&self, position: [f32; 3], orientation: i32, bool_props: &[u32]) -> Option<u64> {
        opt(cb!(
            self,
            add_marker_prop,
            0,
            position.as_ptr(),
            orientation,
            bool_props.as_ptr(),
            bool_props.len()
        ))
    }
    fn marker_props(&self, x: i32, z: i32) -> Vec<u64> {
        let mut v: Vec<u64> = Vec::new();
        cb!(
            self,
            marker_props,
            (),
            x,
            z,
            Some(push_u64),
            &mut v as *mut Vec<u64> as *mut c_void
        );
        v
    }
    fn remove_prop(&self, prop: u64) {
        cb!(self, remove_prop, (), prop)
    }
    fn move_prop(&self, prop: u64, position: [f32; 3]) {
        cb!(self, move_prop, (), prop, position.as_ptr())
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        cb!(self, occupant_type, 0, occupant)
    }
    fn occupant_networks(&self, occupant: u64) -> Vec<u32> {
        let mut v: Vec<u64> = Vec::new();
        cb!(
            self,
            occupant_networks,
            (),
            occupant,
            Some(push_u64),
            &mut v as *mut Vec<u64> as *mut c_void
        );
        v.into_iter().map(|n| n as u32).collect()
    }
    fn occupant_cell(&self, occupant: u64) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, occupant_cell, (), occupant, &mut x, &mut z);
        (x, z)
    }
    fn power_pole_flag(&self, occupant: u64) -> bool {
        cb!(self, power_pole_flag, false, occupant)
    }
    fn connection_object(&self, conn: &Rc<NeighborConnection>) -> u64 {
        let Some(f) = self.0.connection_object else { return 0 };
        let h = new_conn_handle(conn.clone());
        // SAFETY: valid callback; the host takes the handle.
        let t = unsafe { f(self.0.user, h) };
        if t == 0 {
            // SAFETY: refused, so still ours.
            unsafe { fc4_neighbor_connection_destroy(h) };
            return 0;
        }
        // SAFETY: the host keeps the handle alive while its object lives.
        if let Some(ch) = unsafe { conn_ref(h) } {
            ch.token.set(t);
        }
        t
    }
    fn foreign_connection(&self, token: u64) -> Option<ConnectionState> {
        let mut s = fc4_neighbor_connection_state::default();
        cb!(self, foreign_connection, false, token, &mut s).then(|| s.to_core())
    }
    fn set_foreign_connection_flags(&self, token: u64, visited: Option<bool>, active: Option<bool>) {
        let v = visited.map_or(-1, i32::from);
        let a = active.map_or(-1, i32::from);
        cb!(self, set_foreign_connection_flags, (), token, v, a)
    }
    fn repair_foreign_connection(&self, token: u64, city: CityId) {
        cb!(self, repair_foreign_connection, (), token, city)
    }
    fn release_connection(&self, token: u64) {
        cb!(self, release_connection, (), token)
    }
    fn foreign_deal(&self, token: u64) -> Option<DealData> {
        let mut d = fc4_neighbor_deal_data::default();
        cb!(self, foreign_deal, false, token, &mut d).then(|| d.to_core())
    }
    fn foreign_deal_call(&self, token: u64, call: DealCall) -> bool {
        let (op, a, b) = deal_call_op(call);
        cb!(self, foreign_deal_call, false, token, op, a, b)
    }
    fn release_deal(&self, token: u64) {
        cb!(self, release_deal, (), token)
    }
}

/// A call on a foreign deal as the C op, `a`, `b` (`foreign_deal_call`).
pub(crate) fn deal_call_op(c: DealCall) -> (u32, i64, i64) {
    match c {
        DealCall::SetType(t) => (0, i64::from(t), 0),
        DealCall::SetOtherSide(x, z) => (1, i64::from(x), i64::from(z)),
        DealCall::SetCitySide(x, z) => (2, i64::from(x), i64::from(z)),
        DealCall::SetLastAmount(v) => (3, i64::from(v), 0),
        DealCall::SetLastIncAmount(v) => (4, i64::from(v), 0),
        DealCall::SetIncAmount(v) => (5, i64::from(v), 0),
        DealCall::SetIncAmount2(v) => (6, i64::from(v), 0),
        DealCall::SetAmountUsed(v) => (7, i64::from(v), 0),
        DealCall::SetStartDate(d) => (8, i64::from(d), 0),
        DealCall::SetExpirationDate(d) => (9, i64::from(d), 0),
        DealCall::SetCurrentDate(d) => (10, i64::from(d), 0),
        DealCall::SetSuspensionDate(d) => (11, i64::from(d), 0),
        DealCall::SetAmount(v) => (12, i64::from(v), 0),
        DealCall::UpdateCost => (13, 0, 0),
        DealCall::UpdateState => (14, 0, 0),
        DealCall::Cancel => (15, 0, 0),
        DealCall::Activate => (16, 0, 0),
        DealCall::SetFlags(f, on) => (17, i64::from(f), i64::from(on)),
    }
}

// ---- the simulator handle -------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_neighbors`).
#[repr(C)]
pub struct fc4_neighbors {
    _private: [u8; 0],
}

pub(crate) struct Handle {
    pub(crate) host: Rc<dyn NeighborsHost>,
    /// The C table behind `host` (none for the shadow).
    chost: Option<Rc<CHost>>,
    pub(crate) sim: Rc<NeighborsSimulator>,
    /// The shadow's snapshot host (neighbors_shadow.rs), when this handle is
    /// a shadow.
    pub(crate) shadow: Option<Rc<crate::neighbors_shadow::ShadowHost>>,
}

pub(crate) fn into_handle(h: Handle) -> *mut fc4_neighbors {
    Box::into_raw(Box::new(h)) as *mut fc4_neighbors
}

/// A handle for the shadow: its own host, no C table.
pub(crate) fn shadow_handle(
    host: Rc<dyn NeighborsHost>,
    sim: Rc<NeighborsSimulator>,
    shadow: Rc<crate::neighbors_shadow::ShadowHost>,
) -> *mut fc4_neighbors {
    into_handle(Handle {
        host,
        chost: None,
        sim,
        shadow: Some(shadow),
    })
}

/// SAFETY: NULL or a live handle.
pub(crate) unsafe fn handle_ref<'a>(p: *const fc4_neighbors) -> Option<&'a Handle> {
    (p as *const Handle).as_ref()
}

/// Runs `f` on the simulator; the fallback for NULL or a panic.
fn sim<R>(p: *const fc4_neighbors, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_neighbors*.
    let Some(h) = (unsafe { handle_ref(p) }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

/// The host object of a deal (made on first use) for a handle whose host is
/// the C table; the shadow's deals have no host objects (0).
fn deal_token_of(h: &Handle, d: &Rc<NeighborDeal>) -> u64 {
    match &h.chost {
        Some(c) => c.deal_token(d),
        None => d.token(),
    }
}

fn emit(cb: fc4_token_callback, ctx: *mut c_void, tokens: &[u64]) -> usize {
    if let Some(f) = cb {
        for &t in tokens {
            // SAFETY: valid callback (documented precondition).
            unsafe { f(ctx, t) };
        }
    }
    tokens.len()
}

fn tokens_of(cs: &[Rc<NeighborConnection>]) -> Vec<u64> {
    cs.iter().map(|c| c.token()).collect()
}

/// Creates the core object (§6.1). The host table is copied. `token` is the
/// host identity of the simulator; `session` the session seed (foundations
/// §6.3) for the proposal generator.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_create(
    host: *const fc4_neighbors_host,
    token: u64,
    session: u32,
) -> *mut fc4_neighbors {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_neighbors_host>() => {
            let c = Rc::new(CHost(*h));
            let host: Rc<dyn NeighborsHost> = c.clone();
            let sim = NeighborsSimulator::new(host.clone(), token, session);
            into_handle(Handle {
                host,
                chost: Some(c),
                sim,
                shadow: None,
            })
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys the core object. The connection references a loaded object that
/// was never initialised still holds are released.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_destroy(p: *mut fc4_neighbors) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let b = Box::from_raw(p as *mut Handle);
        if !b.sim.state().initialized {
            for c in b.sim.take_connections() {
                if c.token() != 0 {
                    b.host.release_connection(c.token());
                }
            }
        }
        drop(b);
    });
}

/// The two-curve switch (§8.3 *[choice]*), off by default.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_set_two_curves(p: *mut fc4_neighbors, on: bool) {
    sim(p, (), |h| h.sim.set_two_curves(on))
}

/// `Init` (§14.1): true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_init(p: *mut fc4_neighbors) -> bool {
    sim(p, false, |h| h.sim.init())
}

/// `Shutdown` (§14.2): true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_shutdown(p: *mut fc4_neighbors) -> bool {
    sim(p, false, |h| h.sim.shutdown())
}

/// `DoMessage` (§11.1): true for a live object. Handled at once, also while
/// another call is running (see the module docs).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_do_message(p: *mut fc4_neighbors, m: *const fc4_neighbors_message) -> bool {
    let Some(m) = m.as_ref() else { return true };
    let msg = NeighborsMessage {
        msg_type: m.msg_type,
        data1: m.data1,
        data2: m.data2,
        data3: m.data3,
        occupant: opt(m.occupant),
    };
    sim(p, false, |h| h.sim.do_message(&msg))
}

/// Diagnostics.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_get_state(p: *const fc4_neighbors, out: *mut fc4_neighbors_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    sim(p, false, |h| {
        let s = h.sim.state();
        let foreign = h.sim.all_connections().iter().filter(|c| c.is_foreign()).count();
        *o = fc4_neighbors_state {
            struct_size: std::mem::size_of::<fc4_neighbors_state>() as u32,
            initialized: u8::from(s.initialized),
            ready: u8::from(s.ready),
            deals_dirty: u8::from(s.deals_dirty),
            stats_dirty: u8::from(s.stats_dirty),
            ui_disabled: u8::from(s.ui_disabled),
            busy: u8::from(s.busy),
            reserved0: [0; 2],
            connections: s.connections as u32,
            foreign_connections: foreign as u32,
            deals: s.deals as u32,
            reserved1: 0,
            income: h.sim.total_monthly_income(),
            expense: h.sim.total_monthly_expense(),
        };
        true
    })
}

/// `Write` (§13): connections go out as `fc4_ref`s (our objects with their
/// host token; foreign ones as the host objects).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_write(p: *const fc4_neighbors, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    sim(p, false, |h| h.sim.write(&mut COut(o)))
}

/// Our own objects never come back from a stream: no factory is registered
/// for the connection class (§15.2), so every reference is a host object.
fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§13): false for a version other than 2. Each connection reference
/// is accepted (a host object becomes a foreign connection whose reference
/// the list keeps).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_read(p: *mut fc4_neighbors, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    sim(p, false, |h| {
        h.sim.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}

// ---- connections (slots 5-23) ----

/// Slot 5 `RequestCreateConnection` (§6.2 *[choice]*): 0 when the connection
/// exists (the original returns false), 1 when the host must ask the player;
/// `neighbour` gets the established neighbour whose name goes into the text
/// (0 = the default name).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_request_create_connection(
    p: *mut fc4_neighbors,
    t: u32,
    x: i32,
    z: i32,
    neighbour: *mut u64,
) -> i32 {
    let r = sim(p, ConnectionRequest::Exists, |h| {
        h.sim.request_create_connection(t, x, z)
    });
    match r {
        ConnectionRequest::Exists => 0,
        ConnectionRequest::Confirm { neighbour: n } => {
            if let Some(o) = neighbour.as_mut() {
                *o = n.unwrap_or(0);
            }
            1
        }
    }
}

/// Slot 6 `CreateConnection` (§6.3): the new connection's token (borrowed;
/// the list holds the reference), 0 when one exists.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_create_connection(
    p: *mut fc4_neighbors,
    t: u32,
    x: i32,
    z: i32,
    flip: bool,
) -> u64 {
    sim(p, 0, |h| {
        h.sim.create_connection(t, x, z, flip).map_or(0, |c| c.token())
    })
}

/// Slot 7 `DeleteConnection`.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_delete_connection(p: *mut fc4_neighbors, t: u32, x: i32, z: i32) {
    sim(p, (), |h| h.sim.delete_connection(t, x, z))
}

/// Slot 8 `DeleteOccupantConnection` (§6.5).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_delete_occupant_connection(p: *mut fc4_neighbors, occupant: u64) {
    sim(p, (), |h| h.sim.delete_occupant_connection(occupant))
}

/// Slot 9 `DeleteAllConnectionsOfType`.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_delete_all_connections_of_type(p: *mut fc4_neighbors, t: u32) {
    sim(p, (), |h| h.sim.delete_all_connections_of_type(t))
}

/// Slot 10 `HasConnection`.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_has_connection(p: *const fc4_neighbors, t: u32, x: i32, z: i32) -> bool {
    sim(p, false, |h| h.sim.has_connection(t, x, z))
}

/// Slot 11 `FindConnection`: the token (borrowed), 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_find_connection(p: *const fc4_neighbors, t: u32, x: i32, z: i32) -> u64 {
    sim(p, 0, |h| h.sim.find_connection(t, x, z).map_or(0, |c| c.token()))
}

/// Slot 12 `FindCityOnOtherSideOfConnection(x, z)`: the regional city
/// (borrowed), 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_find_city_on_other_side_of_connection(
    p: *const fc4_neighbors,
    x: i32,
    z: i32,
) -> u64 {
    sim(p, 0, |h| h.sim.find_city_on_other_side_of_connection(x, z).unwrap_or(0))
}

/// Slots 13/14 `FindConnectionOnOtherSide(t, x, z)` (slot 13 passes its
/// argument's type and cell): the other city's connection (borrowed) and its
/// `GetIsActive()`; 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_find_connection_on_other_side(
    p: *const fc4_neighbors,
    t: u32,
    x: i32,
    z: i32,
    active: *mut bool,
) -> u64 {
    let r = sim(p, None, |h| h.sim.find_connection_on_other_side(t, x, z));
    if let Some(a) = active.as_mut() {
        *a = r.is_some_and(|f| f.active);
    }
    r.map_or(0, |f| f.token)
}

/// The geometry of §2.2 for connection cell `(x, z)` with the current
/// regional city: the neighbouring region cell (false without a current
/// regional city).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_neighbouring_region_cell(
    p: *const fc4_neighbors,
    x: i32,
    z: i32,
    rx: *mut i32,
    rz: *mut i32,
) -> bool {
    let Some(c) = sim(p, None, |h| h.sim.neighbouring_region_cell(x, z)) else {
        return false;
    };
    if let Some(o) = rx.as_mut() {
        *o = c.0;
    }
    if let Some(o) = rz.as_mut() {
        *o = c.1;
    }
    true
}

/// Slot 15 `FindUnvisitedConnection(t, city)`: the token, 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_find_unvisited_connection(p: *const fc4_neighbors, t: u32, city: u64) -> u64 {
    sim(p, 0, |h| {
        h.sim.find_unvisited_connection(t, city).map_or(0, |c| c.token())
    })
}

/// Slots 16-19: `which` 0 = `GetAllConnections`, 1 = `GetConnectionsOfType(t)`,
/// 2 = `GetConnectionsOfTypeToCity(t, city)`, 3 = `GetConnectionsToCity(city)`.
/// The tokens are borrowed (the list holds the references). Returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_connections(
    p: *const fc4_neighbors,
    which: u32,
    t: u32,
    city: u64,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> usize {
    let tokens = sim(p, Vec::new(), |h| {
        let cs = match which {
            0 => h.sim.all_connections(),
            1 => h.sim.connections_of_type(t),
            2 => h.sim.connections_of_type_to_city(t, city),
            3 => h.sim.connections_to_city(city),
            _ => Vec::new(),
        };
        tokens_of(&cs)
    });
    emit(cb, ctx, &tokens)
}

/// Slots 20-22: `which` 0 = `GetAdjacentCities`, 1 = `GetConnectedCities`,
/// 2 = `GetAllConnectedCities`. Regional-city tokens (borrowed). Returns the
/// count.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_cities(
    p: *const fc4_neighbors,
    which: u32,
    include_unestablished: bool,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> usize {
    let cities = sim(p, Vec::new(), |h| match which {
        0 => h.sim.adjacent_cities(include_unestablished),
        1 => h.sim.connected_cities(include_unestablished),
        2 => h.sim.all_connected_cities(include_unestablished),
        _ => Vec::new(),
    });
    emit(cb, ctx, &cities)
}

/// Slot 23 `FindConnectionSuitableForDeal(type, city)` (§6.6): the token, 0 =
/// none.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_find_connection_suitable_for_deal(
    p: *const fc4_neighbors,
    deal_type: u32,
    city: u64,
) -> u64 {
    sim(p, 0, |h| {
        h.sim
            .find_connection_suitable_for_deal(deal_type, city)
            .map_or(0, |c| c.token())
    })
}

// ---- deals (slots 24-37) ----

/// Slot 24 `ProposeDeal(type, city, &out)`: the deal's host object (made
/// now; the caller AddRefs it for `out`), 0 = none (no suitable connection).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_propose_deal(p: *mut fc4_neighbors, deal_type: u32, city: u64) -> u64 {
    sim(p, 0, |h| {
        h.sim.propose_deal(deal_type, city).map_or(0, |d| deal_token_of(h, &d))
    })
}

/// Slot 25 `ProposeDeals(list, add)` (§7.1): the proposals' host objects (the
/// caller AddRefs each it appends). Returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_propose_deals(
    p: *mut fc4_neighbors,
    add: bool,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> usize {
    let tokens = sim(p, Vec::new(), |h| {
        h.sim
            .propose_deals(add)
            .iter()
            .map(|d| deal_token_of(h, d))
            .collect::<Vec<u64>>()
    });
    emit(cb, ctx, &tokens)
}

/// Slot 26 `AddDeal(deal)` for a deal of ours: true.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_add_deal(p: *mut fc4_neighbors, deal: *const fc4_neighbor_deal) -> bool {
    let Some(d) = deal_ref(deal) else { return false };
    let d = d.deal.clone();
    sim(p, false, |h| h.sim.add_deal(&d))
}

/// Slot 26 `AddDeal(deal)` for a foreign deal, the old way: a deal of ours
/// with the foreign deal's state (`Copy`, §5.4) is added instead. Returns its
/// host object's token (0 if the host made none). Superseded for `AddDeal` by
/// `fc4_neighbors_add_foreign_deal` (NG4); kept for ABI compatibility.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_add_deal_data(
    p: *mut fc4_neighbors,
    data: *const fc4_neighbor_deal_data,
) -> u64 {
    let Some(data) = data.as_ref() else { return 0 };
    let data = data.to_core();
    sim(p, 0, |h| {
        let d = h.sim.new_deal();
        d.copy_from(&data);
        h.sim.add_deal(&d);
        deal_token_of(h, &d)
    })
}

/// Slot 26 `AddDeal(deal)` for a foreign deal (§15.4 NG4): the caller's
/// object itself becomes the listed deal. The host has taken one reference
/// for the core (`token`); the core releases it through `release_deal` when
/// it lets the deal go, and on failure (false) the reference is still the
/// host's. Silent is cleared through `foreign_deal_call` (`SetFlags`) and the
/// deal is simulated through the same calls.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_add_foreign_deal(p: *mut fc4_neighbors, token: u64) -> bool {
    if token == 0 {
        return false;
    }
    sim(p, false, |h| {
        if h.chost.is_none() {
            return false;
        }
        h.sim.add_foreign_deal(token)
    })
}

/// Slot 27 `SetUIDisableFlag`.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_set_ui_disable_flag(p: *mut fc4_neighbors, on: bool) {
    sim(p, (), |h| h.sim.set_ui_disable_flag(on))
}

/// Slots 28-30: `which` 0 = `FindDealsAll`, 1 = `FindDeals(type)` (slot 30),
/// 2 = `FindDeals(type, city)` (slot 29). The deals' host objects (made on
/// first use; the caller AddRefs each it appends). Returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_find_deals(
    p: *mut fc4_neighbors,
    which: u32,
    deal_type: u32,
    city: u64,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> usize {
    let tokens = sim(p, Vec::new(), |h| {
        let ds = match which {
            0 => h.sim.find_deals_all(),
            1 => h.sim.find_deals(deal_type),
            2 => h.sim.find_deals_with(deal_type, city),
            _ => Vec::new(),
        };
        ds.iter().map(|d| deal_token_of(h, d)).collect::<Vec<u64>>()
    });
    emit(cb, ctx, &tokens)
}

/// The deals' states, in list order (the shadow and the month line).
/// Returns the count; `out[0..cap]` is filled.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_deal_data(
    p: *const fc4_neighbors,
    out: *mut fc4_neighbor_deal_data,
    cap: usize,
) -> usize {
    let all = sim(p, Vec::new(), |h| {
        h.sim
            .find_deals_all()
            .iter()
            .map(|d| fc4_neighbor_deal_data::of(&d.data()))
            .collect::<Vec<_>>()
    });
    if !out.is_null() {
        for (i, d) in all.iter().take(cap).enumerate() {
            *out.add(i) = *d;
        }
    }
    all.len()
}

/// Whether the simulator's list holds this deal (the host keeps the deal's
/// object alive while it does).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_holds_deal(p: *const fc4_neighbors, deal: *const fc4_neighbor_deal) -> bool {
    let Some(d) = deal_ref(deal) else { return false };
    let d = d.deal.clone();
    sim(p, false, |h| h.sim.find_deals_all().iter().any(|x| Rc::ptr_eq(x, &d)))
}

/// Links a deal of ours to this simulator ("the current city's neighbours
/// simulator", §5.4) without adding it to the list.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_claim_deal(p: *mut fc4_neighbors, deal: *const fc4_neighbor_deal) {
    let Some(d) = deal_ref(deal) else { return };
    let d = d.deal.clone();
    sim(p, (), |h| h.sim.claim_deal(&d))
}

/// Slot 31 `MarkDealsDirty`.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_mark_deals_dirty(p: *mut fc4_neighbors) {
    sim(p, (), |h| h.sim.mark_deals_dirty())
}

/// Slot 32 `GetTotalMonthlyNeighborDealIncome` (§10.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_total_income(p: *const fc4_neighbors) -> i64 {
    sim(p, 0, |h| h.sim.total_monthly_income())
}

/// Slot 33 `GetTotalMonthlyNeighborDealExpense` (§10.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_total_expense(p: *const fc4_neighbors) -> i64 {
    sim(p, 0, |h| h.sim.total_monthly_expense())
}

/// Slot 34 `UpdateCommodityStats(city)` (§8.2): `city` is the city object.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_update_commodity_stats(p: *mut fc4_neighbors, city: u64) {
    sim(p, (), |h| h.sim.update_commodity_stats(opt(city)))
}

/// Slot 35 `GetDealCost(type, amount, city)` (§8.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_deal_cost(
    p: *const fc4_neighbors,
    deal_type: u32,
    amount: f64,
    city: u64,
) -> f64 {
    sim(p, 0.0, |h| h.sim.get_deal_cost(deal_type, amount, opt(city)))
}

/// Slot 36 `GetDealMaxAmount(type, city)` (§8.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbors_deal_max_amount(p: *const fc4_neighbors, deal_type: u32, city: u64) -> f64 {
    sim(p, 0.0, |h| h.sim.get_deal_max_amount(deal_type, opt(city)))
}

// ---- deals (cISC4NeighborDeal) ---------------------------------------------------------------------

fn with_deal<R>(h: *const fc4_neighbor_deal, fallback: R, f: impl FnOnce(&Rc<NeighborDeal>) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live handle.
    let Some(d) = (unsafe { deal_ref(h) }) else {
        return fallback;
    };
    guard(fallback, || f(&d.deal))
}

/// A new deal (Proposed + Silent, §5.1), linked to no simulator; the caller
/// owns the handle.
#[no_mangle]
pub extern "C" fn fc4_neighbor_deal_create() -> *mut fc4_neighbor_deal {
    guard(std::ptr::null_mut(), || new_deal_handle(NeighborDeal::new()))
}

/// Destroys a handle. If the deal's token is this handle's object, the deal
/// forgets it (a later use makes a new object).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_destroy(h: *mut fc4_neighbor_deal) {
    if h.is_null() {
        return;
    }
    guard((), || {
        let b = Box::from_raw(h as *mut DealHandle);
        let t = b.token.get();
        if t != 0 && b.deal.token() == t {
            b.deal.set_token(0);
        }
        drop(b);
    });
}

/// The host object that owns this handle (for a handle the host made itself).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_set_token(h: *mut fc4_neighbor_deal, token: u64) {
    let Some(d) = deal_ref(h) else { return };
    guard((), || {
        d.token.set(token);
        d.deal.set_token(token);
    })
}

/// The deal's state.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_get(h: *const fc4_neighbor_deal, out: *mut fc4_neighbor_deal_data) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_deal(h, false, |d| {
        *o = fc4_neighbor_deal_data::of(&d.data());
        true
    })
}

/// Plain setters (§5.4): `field` 0 = type, 1 = other side (a, b), 2 = city
/// side (a, b), 3 = lastAmount, 4 = lastIncAmount, 5 = incAmount, 6 =
/// incAmount2, 7 = amountUsed, 8-11 = start, expiration, current and
/// suspension date (Julian day `a`).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_set_field(h: *mut fc4_neighbor_deal, field: u32, a: i32, b: i32) {
    with_deal(h, (), |d| match field {
        0 => d.set_type(a as u32),
        1 => d.set_other_side(a, b),
        2 => d.set_city_side(a, b),
        3 => d.set_last_amount(a),
        4 => d.set_last_inc_amount(a),
        5 => d.set_inc_amount(a),
        6 => d.set_inc_amount2(a),
        7 => d.set_amount_used(a),
        8 => d.set_start_date(a as u32),
        9 => d.set_expiration_date(a as u32),
        10 => d.set_current_date(a as u32),
        11 => d.set_suspension_date(a as u32),
        _ => {}
    })
}

/// `SetAmount(a)` (§5.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_set_amount(h: *mut fc4_neighbor_deal, a: i32) {
    with_deal(h, (), |d| d.set_amount(a))
}

/// `UpdateCost()` (§5.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_update_cost(h: *mut fc4_neighbor_deal) {
    with_deal(h, (), |d| d.update_cost())
}

/// `Copy(src)` (§5.4): `src` read through its interface by the host.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_copy(h: *mut fc4_neighbor_deal, src: *const fc4_neighbor_deal_data) {
    let Some(s) = src.as_ref() else { return };
    let s = s.to_core();
    with_deal(h, (), |d| d.copy_from(&s))
}

/// `UpdateState()` (§7.3 step 1).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_update_state(h: *mut fc4_neighbor_deal) {
    with_deal(h, (), |d| d.update_state())
}

/// `Cancel()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_cancel(h: *mut fc4_neighbor_deal) {
    with_deal(h, (), |d| d.cancel())
}

/// `Activate()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_activate(h: *mut fc4_neighbor_deal) -> bool {
    with_deal(h, false, |d| d.activate())
}

/// `SetFlags(f, on)`: true.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_set_flags(h: *mut fc4_neighbor_deal, f: u32, on: bool) -> bool {
    with_deal(h, false, |d| d.set_flags(f, on))
}

/// `Clone(&out, silent)` (§5.5): a NEW handle for the clone (the caller wraps
/// it in an object and sets its token).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_clone(h: *const fc4_neighbor_deal, silent: bool) -> *mut fc4_neighbor_deal {
    with_deal(h, std::ptr::null_mut(), |d| new_deal_handle(d.clone_deal(silent)))
}

/// `Write` (§5.6).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_write(h: *const fc4_neighbor_deal, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_deal(h, false, |d| d.write(&mut COut(o)))
}

/// `Read` (§5.6).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_deal_read(h: *mut fc4_neighbor_deal, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_deal(h, false, |d| {
        d.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}

// ---- connections (cISC4NeighborConnection) -------------------------------------------------------

fn with_conn<R>(h: *const fc4_neighbor_connection, fallback: R, f: impl FnOnce(&Rc<NeighborConnection>) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live handle.
    let Some(c) = (unsafe { conn_ref(h) }) else {
        return fallback;
    };
    guard(fallback, || f(&c.conn))
}

/// A new connection (§4.1) linked to no simulator; the caller owns the
/// handle (tests and the shadow's checks).
#[no_mangle]
pub extern "C" fn fc4_neighbor_connection_create() -> *mut fc4_neighbor_connection {
    guard(std::ptr::null_mut(), || new_conn_handle(NeighborConnection::new()))
}

/// Destroys a handle.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_connection_destroy(h: *mut fc4_neighbor_connection) {
    if h.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(h as *mut ConnHandle)));
}

/// The connection's state.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_connection_get(
    h: *const fc4_neighbor_connection,
    out: *mut fc4_neighbor_connection_state,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_conn(h, false, |c| {
        *o = fc4_neighbor_connection_state::of(&c.state());
        true
    })
}

/// Setters: `field` 0 = type (a), 1 = cell (a, b), 2 = other-side region cell
/// (a, b), 3 = visitedThisCycle (a != 0), 4 = active (a != 0).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_connection_set_field(
    h: *mut fc4_neighbor_connection,
    field: u32,
    a: i32,
    b: i32,
) {
    with_conn(h, (), |c| match field {
        0 => c.set_type(a as u32),
        1 => c.set_cell(a, b),
        2 => c.set_other_region_cell(a, b),
        3 => c.set_visited_this_cycle(a != 0),
        4 => c.set_active(a != 0),
        _ => {}
    })
}

/// `GetConnectionInnerCell` (§4.2) in a city of `count_x × count_z` cells.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_connection_inner_cell(
    h: *const fc4_neighbor_connection,
    count_x: i32,
    count_z: i32,
    x: *mut i32,
    z: *mut i32,
) -> bool {
    let Some(c) = with_conn(h, None, |c| c.inner_cell((count_x, count_z))) else {
        return false;
    };
    if let Some(o) = x.as_mut() {
        *o = c.0;
    }
    if let Some(o) = z.as_mut() {
        *o = c.1;
    }
    true
}

/// `Repair(R)` (§4.2): `R` at region cell `(pos_x, pos_z)` of size
/// `size_x × size_z`, in a city of `count_x × count_z` cells.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_connection_repair(
    h: *mut fc4_neighbor_connection,
    count_x: i32,
    count_z: i32,
    pos_x: i32,
    pos_z: i32,
    size_x: i32,
    size_z: i32,
) {
    with_conn(h, (), |c| {
        c.repair((count_x, count_z), (pos_x, pos_z), (size_x, size_z))
    })
}

/// Replaces the connection's property holder with a copy of `props` (the
/// host's live holder, before a `Write`).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_connection_set_props(h: *mut fc4_neighbor_connection, props: *const fc4_props) {
    let p = crate::props::props_ref(props).cloned().unwrap_or_default();
    with_conn(h, (), |c| c.set_properties(p))
}

/// A connection property in the game's native layout, arrays included (the
/// shared `native_variant` delivers scalars only: its type check sees the
/// array flag). The void array (raw bytes, NG37) comes as memory type 0 with
/// `is_array`, one byte per element. `None` for empty, pointer and object
/// values.
pub(crate) fn connection_property(v: &Variant) -> Option<NativeVariant> {
    let Variant::Array { elem, items } = v else {
        return native_variant(v);
    };
    if *elem == mem::EMPTY {
        return Some(NativeVariant {
            mem_type: mem::EMPTY,
            is_array: true,
            count: items.len(),
            bytes: items.iter().map(|e| e.as_i64() as u8).collect(),
        });
    }
    if !(mem::BOOL..=mem::RZ_CHAR).contains(elem) {
        return None;
    }
    let mut bytes = Vec::new();
    for e in items {
        bytes.extend(native_variant(&Variant::Scalar(*e))?.bytes);
    }
    Some(NativeVariant {
        mem_type: *elem,
        is_array: true,
        count: items.len(),
        bytes,
    })
}

/// The connection's properties, one callback each in the holder's order (the
/// host's live holder, after a `Read`), arrays included (the game writes its
/// trip capacity, a u16 array, there: foundations §10.2). Types the game
/// helpers do not take from the core (empty, pointers, objects) are left out.
/// Returns the count delivered.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_connection_props(
    h: *const fc4_neighbor_connection,
    cb: fc4_neighbor_property_callback,
    ctx: *mut c_void,
) -> usize {
    let props = with_conn(h, PropertyHolder::new(), |c| c.properties());
    let Some(f) = cb else { return 0 };
    let mut n = 0;
    for id in props.ids() {
        let Some(v) = props.get(id) else { continue };
        if let Some(nv) = connection_property(v) {
            f(
                ctx,
                id,
                nv.mem_type,
                nv.is_array,
                nv.bytes.as_ptr() as *const c_void,
                nv.count,
            );
            n += 1;
        }
    }
    n
}

/// `Write` (§4.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_connection_write(
    h: *const fc4_neighbor_connection,
    out: *const fc4_ostream,
) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_conn(h, false, |c| c.write(&mut COut(o)))
}

/// `Read` (§4.3): false unless the version is 1.
#[no_mangle]
pub unsafe extern "C" fn fc4_neighbor_connection_read(
    h: *mut fc4_neighbor_connection,
    inp: *const fc4_istream,
) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_conn(h, false, |c| {
        c.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}

/// The class ID our connection objects write (`GetGZCLSID`, §4.3).
#[no_mangle]
pub extern "C" fn fc4_neighbor_connection_class_id() -> u32 {
    CLSID_NEIGHBOR_CONNECTION
}
