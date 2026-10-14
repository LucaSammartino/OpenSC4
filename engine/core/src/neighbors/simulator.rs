// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The neighbours simulator, class `0xC990C0A4` (§6–§14).

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use super::connection::NeighborConnection;
use super::deal::{DealData, NeighborDeal};
use super::pricing::{deal_cost, deal_max_amount, CommodityStats, PriceInputs};
use super::tuning::{curve_of, prop, Tuning};
use super::{
    conn_type, deal_type, flag, history, inner_cell, msg, neighbour_connection_cell, neighbouring_region_cell,
    occupant, side_of, status, CityHandle, CityId, ForeignConnection, NeighborsHost, NeighborsMessage, Side, Stat,
    AGENT_FLAGS, AGENT_LEVELS, AGENT_NAME, CLSID_NEIGHBORS_SIMULATOR, CLSID_NEIGHBOR_CONNECTION,
    ELEVATED_MARKER_HEIGHT, IID_MESSAGE_TARGET2, IID_NEIGHBORS_SIMULATOR, IID_NEIGHBOR_CONNECTION, IID_SERIALIZABLE,
    IID_UNKNOWN, PROPOSAL_SEED_KEY,
};
use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::rng::{derive_seed, FixedSeeds, RzRandom};
use crate::stream::{InStream, OutStream, RefValue, SaveObject};

/// The record version (§13).
pub const SIMULATOR_RECORD_VERSION: u16 = 2;

/// The cost per unit of a commodity whose department is missing (§8.2).
pub const MISSING_DEPARTMENT_COST: f64 = 1e-5;
/// `k := months + 0.000107` (§7.3).
pub const LANDFILL_HORIZON_EPSILON: f64 = 0.000107;
/// The term range's upper extension (§7.1).
pub const TERM_EPSILON: f64 = 0.001;
/// Days per term year (§7.1).
pub const DAYS_PER_YEAR: u32 = 365;
/// The proposals' default maximum term (§12: 5 in proposals).
pub const PROPOSAL_DEFAULT_MAX_TERM: f32 = 5.0;

/// `QueryInterface` (§1).
pub fn answers_iid(iid: u32) -> bool {
    matches!(
        iid,
        IID_UNKNOWN | IID_NEIGHBORS_SIMULATOR | IID_MESSAGE_TARGET2 | IID_SERIALIZABLE
    )
}

/// `RequestCreateConnection`'s answer (§6.2 *[choice]*).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionRequest {
    /// `HasConnection` was true: the original returns false.
    Exists,
    /// The shim shows the yes/no dialog; `neighbour` is the established
    /// neighbour whose name goes into the text (else the default name).
    Confirm { neighbour: Option<CityId> },
}

/// A snapshot of the simulator's flags (§6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimState {
    pub initialized: bool,
    pub ready: bool,
    pub deals_dirty: bool,
    pub stats_dirty: bool,
    pub ui_disabled: bool,
    pub busy: bool,
    pub connections: usize,
    pub deals: usize,
}

/// A cell or region-cell pair `(x, z)`.
type Cell2 = (i32, i32);

/// `cSC4NeighborsSimulator`.
pub struct NeighborsSimulator {
    host: Rc<dyn NeighborsHost>,
    me: RefCell<Weak<NeighborsSimulator>>,
    token: Cell<u64>,
    session: u32,
    initialized: Cell<bool>,
    ready: Cell<bool>,
    connections: RefCell<Vec<Rc<NeighborConnection>>>,
    deals: RefCell<Vec<Rc<NeighborDeal>>>,
    tuning_holder: RefCell<Option<Rc<PropertyHolder>>>,
    tuning: RefCell<Tuning>,
    /// The one curve object (§8.3); empty = no curve.
    curve: RefCell<ResponseCurve>,
    /// The two-curve switch (§8.3 *[choice]*, off by default): the scarcity
    /// curve kept apart from the garbage curve.
    two_curves: Cell<bool>,
    price_curve: RefCell<ResponseCurve>,
    garbage_curve: RefCell<ResponseCurve>,
    deals_dirty: Cell<bool>,
    stats_dirty: Cell<bool>,
    ui_disabled: Cell<bool>,
    busy: Cell<bool>,
    /// The proposal generator, created on first use (§7.1).
    rng: RefCell<Option<RzRandom>>,
}

fn trunc_i32(v: f64) -> i32 {
    if v.is_finite() && v > -2_147_483_649.0 && v < 2_147_483_648.0 {
        v.trunc() as i32
    } else {
        i32::MIN
    }
}

impl NeighborsSimulator {
    /// A simulator in its initial state (§6.1). `session` is the host's
    /// session seed (foundations §6.3).
    pub fn new(host: Rc<dyn NeighborsHost>, token: u64, session: u32) -> Rc<NeighborsSimulator> {
        let s = Rc::new(NeighborsSimulator {
            host,
            me: RefCell::new(Weak::new()),
            token: Cell::new(token),
            session,
            initialized: Cell::new(false),
            ready: Cell::new(false),
            connections: RefCell::new(Vec::new()),
            deals: RefCell::new(Vec::new()),
            tuning_holder: RefCell::new(None),
            tuning: RefCell::new(Tuning::default()),
            curve: RefCell::new(ResponseCurve::default()),
            two_curves: Cell::new(false),
            price_curve: RefCell::new(ResponseCurve::default()),
            garbage_curve: RefCell::new(ResponseCurve::default()),
            deals_dirty: Cell::new(true),
            stats_dirty: Cell::new(false),
            ui_disabled: Cell::new(false),
            busy: Cell::new(false),
            rng: RefCell::new(None),
        });
        *s.me.borrow_mut() = Rc::downgrade(&s);
        s
    }

    fn rc(&self) -> Rc<NeighborsSimulator> {
        self.me.borrow().upgrade().expect("simulator alive")
    }

    pub fn host(&self) -> &Rc<dyn NeighborsHost> {
        &self.host
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn state(&self) -> SimState {
        SimState {
            initialized: self.initialized.get(),
            ready: self.ready.get(),
            deals_dirty: self.deals_dirty.get(),
            stats_dirty: self.stats_dirty.get(),
            ui_disabled: self.ui_disabled.get(),
            busy: self.busy.get(),
            connections: self.connections.borrow().len(),
            deals: self.deals.borrow().len(),
        }
    }

    /// The tuning values in use.
    pub fn tuning(&self) -> Tuning {
        self.tuning.borrow().clone()
    }

    /// `GetTuningExemplar()`.
    pub fn tuning_exemplar(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning_holder.borrow().clone()
    }

    /// Loads the tuning (Init step 1, or a test).
    pub fn set_tuning(&self, h: Option<Rc<PropertyHolder>>) {
        *self.tuning.borrow_mut() = Tuning::read(h.as_deref());
        *self.tuning_holder.borrow_mut() = h;
    }

    /// The two-curve switch (§8.3 *[choice]*); off by default.
    pub fn set_two_curves(&self, on: bool) {
        self.two_curves.set(on);
    }

    /// The curve `GetDealCost` evaluates (§8.3).
    pub fn deal_price_curve(&self) -> ResponseCurve {
        if self.two_curves.get() {
            self.price_curve.borrow().clone()
        } else {
            self.curve.borrow().clone()
        }
    }

    /// The test hook for `ready` (ConnectionsReady).
    pub fn set_ready(&self, on: bool) {
        self.ready.set(on);
    }

    pub fn set_dirty(&self, deals: bool, stats: bool) {
        self.deals_dirty.set(deals);
        self.stats_dirty.set(stats);
    }

    // ---- lifecycle (§14) ----

    /// `Init()` (§14.1). Returns true.
    pub fn init(&self) -> bool {
        if self.initialized.get() {
            return true;
        }
        // 1.
        self.initialized.set(true);
        if self.tuning_holder.borrow().is_none() {
            self.set_tuning(self.host.tuning());
        }
        let holder = self.tuning_holder.borrow().clone();
        if let Some(c) = curve_of(holder.as_deref(), prop::DEAL_PRICE_CURVE) {
            *self.curve.borrow_mut() = c.clone();
            *self.price_curve.borrow_mut() = c;
        }
        // 2.
        for level in AGENT_LEVELS {
            self.host.add_agent(level, AGENT_NAME, AGENT_FLAGS);
        }
        // 3.
        for m in msg::SUBSCRIPTIONS {
            self.host.subscribe(m);
        }
        // 4. A foreign connection (§15.2) repairs itself through its
        // interface.
        if let Some(r) = self.host.current_city() {
            let counts = self.host.cell_counts();
            let (pos, size) = (self.host.city_position(r), self.host.city_size(r));
            for c in self.connections.borrow().clone() {
                if c.is_foreign() {
                    self.host.repair_foreign_connection(c.token(), r);
                } else {
                    c.repair(counts, pos, size);
                }
            }
        }
        // 5.
        for c in self.adjacent_cities(false) {
            self.host.reload_city(c);
        }
        // *[compat]* `ReloadCity` replaces the region's city object: no city
        // from before the reloads is used again. The adjacent cities (step 7)
        // and `R` (step 6) are asked for again.
        let adjacent = self.adjacent_cities(false);
        // 6.
        let current = self.host.current_city();
        if let Some(r) = current {
            self.host.update_local_deals(r);
        }
        self.deals.borrow_mut().clear();
        // 7. In reverse list order; each city's deals from the back.
        let today = self.host.today();
        for &c in adjacent.iter().rev() {
            if Some(c) == current {
                continue;
            }
            self.host.update_local_deals(c);
            for d in self.host.local_deals(c).iter().rev() {
                let e = self.clone_deal_data(d, false);
                if e.other_city().is_some() {
                    e.set_flags(flag::SILENT, false);
                    self.deals.borrow_mut().push(e.clone());
                    let shift = today.wrapping_sub(e.current_date());
                    e.set_start_date(e.start_date().wrapping_add(shift));
                    e.set_expiration_date(e.expiration_date().wrapping_add(shift));
                    e.set_current_date(e.current_date().wrapping_add(shift));
                    e.set_suspension_date(e.suspension_date().wrapping_add(shift));
                }
            }
        }
        // 8.
        self.push(true, true);
        true
    }

    /// `Shutdown()` (§14.2). Returns true.
    pub fn shutdown(&self) -> bool {
        if !self.initialized.get() {
            return true;
        }
        self.initialized.set(false);
        for m in msg::SUBSCRIPTIONS {
            self.host.unsubscribe(m);
        }
        self.host.remove_agents();
        let doomed = std::mem::take(&mut *self.connections.borrow_mut());
        self.release_connections(&doomed);
        self.deals.borrow_mut().clear();
        *self.tuning_holder.borrow_mut() = None;
        true
    }

    /// The references the list held to `doomed` (now out of the list) are
    /// released (ours and foreign alike, §15.2).
    fn release_connections(&self, doomed: &[Rc<NeighborConnection>]) {
        for c in doomed {
            if c.token() != 0 {
                self.host.release_connection(c.token());
            }
        }
    }

    /// Appends a connection to the list without any effect (the shadow's
    /// snapshot of the original's list, and tests).
    pub fn adopt_connection(&self, c: Rc<NeighborConnection>) {
        self.connections.borrow_mut().push(c);
    }

    /// Takes the connection list out of the simulator (the handle's
    /// destructor, for an object that was read but never initialised: the
    /// references the list holds are the caller's to release).
    pub fn take_connections(&self) -> Vec<Rc<NeighborConnection>> {
        std::mem::take(&mut *self.connections.borrow_mut())
    }

    // ---- messages (§11.1) ----

    /// `DoMessage`. Returns true.
    pub fn do_message(&self, m: &NeighborsMessage) -> bool {
        match m.msg_type {
            msg::SIM_NEW_MONTH => self.simulate_deals(),
            msg::SIM_NEW_DAY | msg::SIM_IDLE => self.push(self.deals_dirty.get(), self.stats_dirty.get()),
            msg::DEAL_STATUS => {
                if matches!(m.data1, status::CHANGED | status::RETRY) {
                    self.stats_dirty.set(true);
                } else {
                    self.deals_dirty.set(true);
                }
                self.simulate_deals();
            }
            msg::CONNECTIONS_READY => self.ready.set(true),
            msg::TERRAIN_HEIGHTS_CHANGED => self.terrain_heights_changed(m.data1, m.data2),
            msg::PRE_DEMOLISH_NETWORK_OCCUPANT => {
                if let Some(o) = m.occupant {
                    self.delete_occupant_connection(o);
                }
            }
            msg::REMOVE_OCCUPANT => {
                if let Some(o) = m.occupant {
                    if self.host.occupant_type(o) == occupant::POWER_POLE && !self.host.power_pole_flag(o) {
                        self.delete_occupant_connection(o);
                    }
                }
            }
            msg::CITY_ESTABLISHED | msg::CITY_OBLITERATED => {
                self.deals.borrow_mut().clear();
                self.push(true, true);
            }
            _ => {}
        }
        true
    }

    /// TerrainHeightsChanged (§11.1): the rectangle `x0 = (data1 >> 16) − 1`,
    /// `x1 = data1 & 0xFFFF`, `z0 = (data2 >> 16) − 1`, `z1 = data2 & 0xFFFF`,
    /// clamped to the city; the markers of every connection whose inner cell
    /// lies in it move to the cell centre at `GetMaxSurfaceLevel`.
    fn terrain_heights_changed(&self, data1: u32, data2: u32) {
        let (cx, cz) = self.host.cell_counts();
        let x0 = (((data1 >> 16) as i32) - 1).max(0);
        let x1 = ((data1 & 0xFFFF) as i32).min(cx - 1);
        let z0 = (((data2 >> 16) as i32) - 1).max(0);
        let z1 = ((data2 & 0xFFFF) as i32).min(cz - 1);
        for c in self.connections.borrow().clone() {
            let Some((ix, iz)) = c.inner_cell((cx, cz)) else {
                continue;
            };
            if ix < x0 || ix > x1 || iz < z0 || iz > z1 {
                continue;
            }
            let (px, pz) = self.host.cell_center(ix, iz);
            let mut y = self.host.max_surface_level(ix, iz);
            if conn_type::is_elevated(c.conn_type()) {
                y += ELEVATED_MARKER_HEIGHT;
            }
            for p in self.host.marker_props(ix, iz) {
                self.host.move_prop(p, [px, y, pz]);
            }
        }
    }

    // ---- geometry through the region (§2.2) ----

    fn current_geometry(&self) -> Option<(CityId, Cell2, Cell2)> {
        let r = self.host.current_city()?;
        Some((r, self.host.city_position(r), self.host.city_size(r)))
    }

    /// The neighbouring region cell of connection cell `(x, z)` with the
    /// current regional city (§2.2).
    pub fn neighbouring_region_cell(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        let (_, pos, size) = self.current_geometry()?;
        let (cx, cz) = self.host.cell_counts();
        Some(neighbouring_region_cell(side_of(x, z, cx, cz), x, z, pos, size))
    }

    /// `FindCityOnOtherSideOfConnection(x, z)` (§6.2).
    pub fn find_city_on_other_side_of_connection(&self, x: i32, z: i32) -> Option<CityId> {
        let (rx, rz) = self.neighbouring_region_cell(x, z)?;
        self.host.city_at(rx, rz)
    }

    /// The neighbour's connection cell of `(x, z)` (§2.2) and the neighbour.
    /// `None` without a current regional record or a city at the region
    /// cell. An interior cell finds our own regional city and `(0, 0)` (NB6).
    pub fn neighbour_cell(&self, x: i32, z: i32) -> Option<(CityId, (i32, i32))> {
        let (cx, cz) = self.host.cell_counts();
        let side = side_of(x, z, cx, cz);
        let (rx, rz) = self.neighbouring_region_cell(x, z)?;
        let n = self.host.city_at(rx, rz)?;
        let cell = neighbour_connection_cell(side, x, z, (rx, rz), self.host.city_position(n), self.host.city_size(n));
        Some((n, cell))
    }

    /// `FindConnectionOnOtherSide(t, x, z)` (§6.2).
    pub fn find_connection_on_other_side(&self, t: u32, x: i32, z: i32) -> Option<ForeignConnection> {
        let (n, (nx, nz)) = self.neighbour_cell(x, z)?;
        self.host.city_find_connection(n, t, nx, nz)
    }

    /// `GetCityOnOtherSideOfConnection()` of one of our connections.
    pub fn connection_city(&self, c: &NeighborConnection) -> Option<CityId> {
        let (rx, rz) = c.other_region_cell();
        self.host.city_at(rx, rz)
    }

    // ---- connections (§6.2–§6.6) ----

    /// `RequestCreateConnection(type, x, z, flag)` (§6.2 *[choice]*).
    pub fn request_create_connection(&self, t: u32, x: i32, z: i32) -> ConnectionRequest {
        if self.has_connection(t, x, z) {
            return ConnectionRequest::Exists;
        }
        let neighbour = self
            .find_city_on_other_side_of_connection(x, z)
            .filter(|&c| self.host.city_established(c));
        ConnectionRequest::Confirm { neighbour }
    }

    /// `CreateConnection(type, x, z, flag)` (§6.3). *[choice]* Returns the new
    /// connection, or null when one exists.
    pub fn create_connection(&self, t: u32, x: i32, z: i32, flip: bool) -> Option<Rc<NeighborConnection>> {
        // 1.
        if self.has_connection(t, x, z) {
            return None;
        }
        // 2.
        let c = NeighborConnection::new();
        self.connections.borrow_mut().push(c.clone());
        c.set_type(t);
        c.set_cell(x, z);
        if let Some((rx, rz)) = self.neighbouring_region_cell(x, z) {
            c.set_other_region_cell(rx, rz);
        }
        // The host object of our connection (§15.2), before the region sees it.
        c.set_token(self.host.connection_object(&c));
        if let Some(n) = self.connection_city(&c) {
            self.host.change_symmetric_connection(n, &c, true);
        }
        // 3.
        if t != conn_type::FERRY {
            let (cx, cz) = self.host.cell_counts();
            let (ix, iz) = inner_cell(x, z, cx, cz);
            let (px, pz) = self.host.cell_center(ix, iz);
            // NB3: the height is taken at the inner cell's centre, the point
            // the prop is placed at; the connection cell is not used.
            let mut y = self.host.marker_altitude(px, pz);
            if conn_type::is_elevated(t) {
                y += ELEVATED_MARKER_HEIGHT;
            }
            let orientation = marker_orientation(side_of(x, z, cx, cz), flip);
            let mut props = vec![occupant::MARKER_FLAG];
            match t {
                conn_type::PIPE => props.extend([occupant::MARKER_UNDERGROUND, occupant::MARKER_PIPE]),
                conn_type::SUBWAY => props.extend([occupant::MARKER_UNDERGROUND, occupant::MARKER_SUBWAY]),
                _ => {}
            }
            self.host.add_marker_prop([px, y, pz], orientation, &props);
        }
        // 4.
        self.host.post_connection_message(msg::CONNECTION_CREATED, t, x, z);
        Some(c)
    }

    /// Deleting a connection (§6.4); the caller removes it from the list.
    fn delete_connection_object(&self, c: &Rc<NeighborConnection>) {
        if let Some(n) = self.connection_city(c) {
            self.host.change_symmetric_connection(n, c, false);
        }
        let (x, z) = c.cell();
        let (cx, cz) = self.host.cell_counts();
        let (ix, iz) = inner_cell(x, z, cx, cz);
        for p in self.host.marker_props(ix, iz) {
            self.host.remove_prop(p);
        }
        self.host
            .post_connection_message(msg::CONNECTION_DELETED, c.conn_type(), x, z);
    }

    /// `DeleteConnection(type, x, z)`.
    pub fn delete_connection(&self, t: u32, x: i32, z: i32) {
        let Some(c) = self.find_connection(t, x, z) else { return };
        self.delete_connection_object(&c);
        self.connections.borrow_mut().retain(|o| !Rc::ptr_eq(o, &c));
        self.release_connections(std::slice::from_ref(&c));
    }

    /// `DeleteAllConnectionsOfType(t)`.
    pub fn delete_all_connections_of_type(&self, t: u32) {
        let doomed = self.connections_of_type(t);
        for c in &doomed {
            self.delete_connection_object(c);
        }
        self.connections
            .borrow_mut()
            .retain(|o| !doomed.iter().any(|d| Rc::ptr_eq(d, o)));
        self.release_connections(&doomed);
    }

    /// `DeleteOccupantConnection(occ)` (§6.5).
    pub fn delete_occupant_connection(&self, occ: u64) {
        let kind = self.host.occupant_type(occ);
        let t = if kind == occupant::NETWORK {
            let carried = self.host.occupant_networks(occ);
            // The later entry of the list wins.
            const MAP: [(u32, u32); 10] = [
                (4, conn_type::PIPE),
                (11, conn_type::DIRT_ROAD),
                (0, conn_type::ROAD),
                (6, conn_type::AVENUE),
                (12, conn_type::GROUND_HIGHWAY),
                (2, conn_type::HIGHWAY),
                (7, conn_type::SUBWAY),
                (8, conn_type::LIGHT_RAIL),
                (9, conn_type::MONORAIL),
                (1, conn_type::RAIL),
            ];
            let mut found = None;
            for (net, ct) in MAP {
                if carried.contains(&net) {
                    found = Some(ct);
                }
            }
            match found {
                Some(t) => t,
                None => return,
            }
        } else if kind == occupant::POWER_POLE {
            conn_type::POWER_POLE
        } else {
            return;
        };
        let (cx, cz) = self.host.occupant_cell(occ);
        let (count_x, count_z) = self.host.cell_counts();
        if cx == 0 {
            self.delete_connection(t, -1, cz);
        } else if cx == count_x - 1 {
            self.delete_connection(t, count_x, cz);
        }
        if cz == 0 {
            self.delete_connection(t, cx, -1);
        } else if cz == count_z - 1 {
            self.delete_connection(t, cx, count_z);
        }
    }

    /// `HasConnection(t, x, z)`.
    pub fn has_connection(&self, t: u32, x: i32, z: i32) -> bool {
        self.find_connection(t, x, z).is_some()
    }

    /// `FindConnection(t, x, z)`: the first match.
    pub fn find_connection(&self, t: u32, x: i32, z: i32) -> Option<Rc<NeighborConnection>> {
        self.connections
            .borrow()
            .iter()
            .find(|c| c.conn_type() == t && c.cell() == (x, z))
            .cloned()
    }

    /// `FindConnectionOnOtherSide(conn)`.
    pub fn find_connection_on_other_side_of(&self, c: &NeighborConnection) -> Option<ForeignConnection> {
        let (x, z) = c.cell();
        self.find_connection_on_other_side(c.conn_type(), x, z)
    }

    /// `FindUnvisitedConnection(t, city)`.
    pub fn find_unvisited_connection(&self, t: u32, city: CityId) -> Option<Rc<NeighborConnection>> {
        self.connections_of_type_to_city(t, city)
            .into_iter()
            .find(|c| !c.visited_this_cycle())
    }

    /// `GetAllConnections`.
    pub fn all_connections(&self) -> Vec<Rc<NeighborConnection>> {
        self.connections.borrow().clone()
    }

    /// `GetConnectionsOfType(t)`.
    pub fn connections_of_type(&self, t: u32) -> Vec<Rc<NeighborConnection>> {
        self.all_connections()
            .into_iter()
            .filter(|c| c.conn_type() == t)
            .collect()
    }

    /// `GetConnectionsOfTypeToCity(t, city)`.
    pub fn connections_of_type_to_city(&self, t: u32, city: CityId) -> Vec<Rc<NeighborConnection>> {
        self.all_connections()
            .into_iter()
            .filter(|c| c.conn_type() == t && self.connection_city(c) == Some(city))
            .collect()
    }

    /// `GetConnectionsToCity(city)`.
    pub fn connections_to_city(&self, city: CityId) -> Vec<Rc<NeighborConnection>> {
        self.all_connections()
            .into_iter()
            .filter(|c| self.connection_city(c) == Some(city))
            .collect()
    }

    /// `GetAdjacentCities(list, includeUnestablished)` (§9.1). *[choice]*
    /// (NB14) Ordered by region position (x, then z): the original collects
    /// into a `std::set` of pointers (`0x006D01E0`), ordered by heap address,
    /// which cannot be reproduced.
    pub fn adjacent_cities(&self, include_unestablished: bool) -> Vec<CityId> {
        let Some((_, (px, pz), (sx, sz))) = self.current_geometry() else {
            return Vec::new();
        };
        let mut cells = Vec::new();
        for x in px..px + sx {
            cells.push((x, pz - 1));
            cells.push((x, pz + sz));
        }
        for z in pz..pz + sz {
            cells.push((px - 1, z));
            cells.push((px + sx, z));
        }
        let mut out: Vec<CityId> = Vec::new();
        for (x, z) in cells {
            if let Some(c) = self.host.city_at(x, z) {
                if !out.contains(&c) && (include_unestablished || self.host.city_established(c)) {
                    out.push(c);
                }
            }
        }
        out.sort_by_key(|&c| self.host.city_position(c));
        out
    }

    /// `GetConnectedCities(list, includeUnestablished)` (§6.2).
    pub fn connected_cities(&self, include_unestablished: bool) -> Vec<CityId> {
        let mut out = Vec::new();
        for c in self.all_connections() {
            if !conn_type::is_transport(c.conn_type()) {
                continue;
            }
            if let Some(n) = self.connection_city(&c) {
                if (include_unestablished || self.host.city_established(n)) && !out.contains(&n) {
                    out.push(n);
                }
            }
        }
        out
    }

    /// `GetAllConnectedCities(list, includeUnestablished)` (§6.2): breadth
    /// first over the region from the current city.
    pub fn all_connected_cities(&self, include_unestablished: bool) -> Vec<CityId> {
        let Some(r) = self.host.current_city() else {
            return Vec::new();
        };
        let mut list = vec![r];
        let mut i = 0;
        while i < list.len() {
            let city = list[i];
            let conns: Vec<(u32, (i32, i32))> = if city == r {
                self.all_connections()
                    .iter()
                    .map(|c| (c.conn_type(), c.other_region_cell()))
                    .collect()
            } else {
                self.host.city_connections(city)
            };
            for (t, (x, z)) in conns {
                if !conn_type::is_transport(t) {
                    continue;
                }
                if let Some(n) = self.host.city_at(x, z) {
                    if !list.contains(&n) {
                        list.push(n);
                    }
                }
            }
            i += 1;
        }
        list.retain(|&c| c != r && (include_unestablished || self.host.city_established(c)));
        list
    }

    /// `FindConnectionSuitableForDeal(type, city)` (§6.6).
    pub fn find_connection_suitable_for_deal(&self, t: u32, city: CityId) -> Option<Rc<NeighborConnection>> {
        match t {
            deal_type::BUY_GARBAGE | deal_type::SELL_GARBAGE => {
                for ct in conn_type::GARBAGE_SEARCH {
                    if let Some(c) = self.connections_of_type_to_city(ct, city).into_iter().next() {
                        return Some(c);
                    }
                }
                None
            }
            deal_type::BUY_POWER | deal_type::SELL_POWER | deal_type::BUY_WATER | deal_type::SELL_WATER => {
                let ct = if deal_type::commodity(t) == Some(1) {
                    conn_type::POWER_POLE
                } else {
                    conn_type::PIPE
                };
                let sell = deal_type::is_sell(t);
                self.connections_of_type_to_city(ct, city).into_iter().find(|c| {
                    if sell {
                        c.is_active()
                    } else {
                        self.find_connection_on_other_side_of(c).is_some_and(|o| o.active)
                    }
                })
            }
            _ => None,
        }
    }

    // ---- deals ----

    /// A new deal linked to this simulator.
    pub fn new_deal(&self) -> Rc<NeighborDeal> {
        let d = NeighborDeal::new();
        d.attach(&self.rc());
        d
    }

    /// Links a deal to this simulator without adding it to the list: "the
    /// current city's neighbours simulator" of §5.4, for a deal of ours that
    /// outlived the simulator that made it (the region keeps references
    /// across city loads, foundations §11.4).
    pub fn claim_deal(&self, d: &Rc<NeighborDeal>) {
        d.attach(&self.rc());
    }

    /// Links a deal to this simulator and appends it to the list, without a
    /// push (test and load helper).
    pub fn adopt_deal(&self, d: &Rc<NeighborDeal>) {
        d.attach(&self.rc());
        self.deals.borrow_mut().push(d.clone());
    }

    /// The deal list (`FindDealsAll`).
    pub fn find_deals_all(&self) -> Vec<Rc<NeighborDeal>> {
        self.deals.borrow().clone()
    }

    /// `FindDeals(type)`.
    pub fn find_deals(&self, t: u32) -> Vec<Rc<NeighborDeal>> {
        self.find_deals_all()
            .into_iter()
            .filter(|d| d.deal_type() == t)
            .collect()
    }

    /// `FindDeals(type, city)`: the deals of that type whose other-side city
    /// is `city`.
    pub fn find_deals_with(&self, t: u32, city: CityId) -> Vec<Rc<NeighborDeal>> {
        self.find_deals_all()
            .into_iter()
            .filter(|d| {
                let (x, z) = d.other_side();
                d.deal_type() == t && self.host.city_at(x, z) == Some(city)
            })
            .collect()
    }

    /// `MarkDealsDirty()`.
    pub fn mark_deals_dirty(&self) {
        self.deals_dirty.set(true);
    }

    /// `SetUIDisableFlag(b)`.
    pub fn set_ui_disable_flag(&self, on: bool) {
        self.ui_disabled.set(on);
    }

    /// `AddDeal(deal)` (§6.2): Silent off, append, push both. Returns true.
    pub fn add_deal(&self, d: &Rc<NeighborDeal>) -> bool {
        d.attach(&self.rc());
        d.set_flags(flag::SILENT, false);
        self.deals.borrow_mut().push(d.clone());
        self.push(true, true);
        true
    }

    /// `GetTotalMonthlyNeighborDealExpense()` (§10.1).
    pub fn total_monthly_expense(&self) -> i64 {
        self.total(deal_type::is_buy)
    }

    /// `GetTotalMonthlyNeighborDealIncome()` (§10.1).
    pub fn total_monthly_income(&self) -> i64 {
        self.total(deal_type::is_sell)
    }

    fn total(&self, side: fn(u32) -> bool) -> i64 {
        self.deals
            .borrow()
            .iter()
            .filter(|d| side(d.deal_type()) && !d.check_flags_or(flag::TOTALS_EXCLUDED))
            .fold(0i64, |s, d| s.wrapping_add(d.cost()))
    }

    /// `Clone(&out, silent)` of a deal's state (§5.5), seen from the current
    /// city.
    pub fn clone_deal_data(&self, src: &DealData, silent: bool) -> Rc<NeighborDeal> {
        let e = self.new_deal();
        e.copy_from(src);
        e.set_flags(flag::SILENT, silent);
        let (cx, cz) = e.city_side();
        let (ox, oz) = e.other_side();
        let city_side = self.host.city_at(cx, cz);
        let other_side = self.host.city_at(ox, oz);
        match (city_side, other_side) {
            (Some(a), Some(_)) => {
                if Some(a) != self.host.current_city() {
                    e.set_city_side(ox, oz);
                    e.set_other_side(cx, cz);
                    e.set_type(deal_type::toggled(e.deal_type()));
                }
            }
            _ => {
                e.set_flags(flag::CANCELLED, true);
            }
        }
        e
    }

    /// `UpdateState()` (§7.3 step 1). *[choice]* (NB15) Without a current
    /// regional city the budget test (`cost > budget`) is skipped; the
    /// original dereferences the missing city for a buy deal and crashes
    /// (`0x006CF386`). Sell deals never use it.
    pub(crate) fn update_deal_state(&self, d: &Rc<NeighborDeal>) {
        let r = self.host.current_city();
        let t = d.deal_type();
        if deal_type::is_buy(t) && r.is_some_and(|r| self.host.city_budget(r) < d.cost() as f64) {
            d.set_flags(flag::CANNOT_AFFORD, true);
            return;
        }
        let suitable = d
            .other_city()
            .and_then(|c| self.find_connection_suitable_for_deal(t, c))
            .is_some();
        if !suitable {
            d.set_flags(flag::NO_CONNECTION, true);
        }
    }

    // ---- proposals (§7.1) ----

    /// `ProposeDeal(type, city, &out)`: the deal when a suitable connection
    /// exists.
    pub fn propose_deal(&self, t: u32, city: CityId) -> Option<Rc<NeighborDeal>> {
        let d = self.new_deal();
        let c = self.find_connection_suitable_for_deal(t, city)?;
        let (rx, rz) = c.other_region_cell();
        d.set_other_side(rx, rz);
        if let Some((_, (px, pz), _)) = self.current_geometry() {
            d.set_city_side(px, pz);
        }
        d.set_type(t);
        d.set_amount(0);
        Some(d)
    }

    fn proposal_rng_range(&self, lo: f64, hi: f64) -> f64 {
        let mut slot = self.rng.borrow_mut();
        let rng = slot.get_or_insert_with(|| {
            let s = derive_seed(self.session, PROPOSAL_SEED_KEY);
            RzRandom::new(
                s,
                &FixedSeeds {
                    wall_clock: s,
                    process_clock: s,
                },
            )
        });
        rng.range_f64(lo, hi)
    }

    /// The proposal generator's states `(lcg, mul)`, if created.
    pub fn proposal_rng_state(&self) -> Option<(u32, u32)> {
        self.rng.borrow().as_ref().map(|r| (r.lcg_state(), r.mul_state()))
    }

    /// `ProposeDeals(list, add)` (§7.1): the proposals.
    pub fn propose_deals(&self, add: bool) -> Vec<Rc<NeighborDeal>> {
        let mut list = Vec::new();
        if !self.ready.get() {
            return list;
        }
        self.update_commodity_stats(self.host.current_city_object());
        let Some(r) = self.host.current_city() else { return list };
        let t = self.tuning();
        let min_budget = f64::from(t.proposal_min_budget);
        let cost_factor = f64::from(t.proposal_cost_factor);
        for c in self.adjacent_cities(false) {
            for (buy, sell) in [
                (deal_type::BUY_POWER, deal_type::SELL_POWER),
                (deal_type::BUY_WATER, deal_type::SELL_WATER),
                (deal_type::BUY_GARBAGE, deal_type::SELL_GARBAGE),
            ] {
                let base = f64::from(t.base_amount(buy));
                for (dt, payer) in [(buy, r), (sell, c)] {
                    if self.find_connection_suitable_for_deal(dt, c).is_none()
                        || !self.find_deals_with(dt, c).is_empty()
                    {
                        continue;
                    }
                    if self.get_deal_max_amount(dt, Some(c)).min(base) < base {
                        continue;
                    }
                    let cost = self.get_deal_cost(dt, base, Some(c));
                    let budget = self.host.city_budget(payer);
                    if min_budget < budget && cost < budget && cost_factor * cost < budget {
                        if let Some(d) = self.propose(dt, c, add, &t) {
                            list.push(d);
                        }
                    }
                }
            }
        }
        if !list.is_empty() && add {
            self.deals_dirty.set(true);
            self.push(true, true);
        }
        list
    }

    /// `Propose(type, C)` of §7.1.
    fn propose(&self, t: u32, c: CityId, add: bool, tuning: &Tuning) -> Option<Rc<NeighborDeal>> {
        let d = self.propose_deal(t, c)?;
        if add {
            d.set_flags(flag::SILENT, false);
            self.deals.borrow_mut().push(d.clone());
        }
        d.set_amount(0);
        let lo = f64::from(tuning.min_term_years);
        let hi = f64::from(tuning.max_term_years.unwrap_or(PROPOSAL_DEFAULT_MAX_TERM));
        let y = self.proposal_rng_range(lo, lo.max(hi) + TERM_EPSILON).floor();
        let today = self.host.today();
        d.set_current_date(today);
        d.set_start_date(today);
        let days = (f64::from(DAYS_PER_YEAR) * y) as i64;
        d.set_expiration_date(today.wrapping_add(days as u32));
        Some(d)
    }

    // ---- pricing (§8) ----

    fn stats_of(&self, city: CityId, commodity: i32) -> CommodityStats {
        CommodityStats {
            demanded: self.host.city_stat(city, Stat::Demanded, commodity),
            exported: self.host.city_stat(city, Stat::Exported, commodity),
            produced: self.host.city_stat(city, Stat::Produced, commodity),
            imported: self.host.city_stat(city, Stat::Imported, commodity),
            cost_per_unit: self.host.city_stat(city, Stat::CostPerUnit, commodity),
        }
    }

    /// The buyer, the seller and the price inputs of a deal type with `C`.
    fn price_setup(&self, t: u32, c: CityId) -> Option<(CommodityStats, CommodityStats, PriceInputs)> {
        let commodity = deal_type::commodity(t)?;
        let r = self.host.current_city()?;
        let (us, them) = (self.stats_of(r, commodity), self.stats_of(c, commodity));
        let (buyer, seller) = if deal_type::is_buy(t) { (us, them) } else { (them, us) };
        let tuning = self.tuning();
        let existing = self
            .find_deals_with(t, c)
            .first()
            .map_or(0.0, |d| f64::from(d.amount()));
        Some((
            buyer,
            seller,
            PriceInputs {
                base: f64::from(tuning.base_amount(t)),
                scale: f64::from(tuning.amount_scale),
                min_markup: tuning.min_markup,
                max_markup: tuning.max_markup,
                existing,
            },
        ))
    }

    /// `GetDealCost(type, amount, C)` (§8.1). *[choice]* 0 for unknown types
    /// and for a null `C`.
    pub fn get_deal_cost(&self, t: u32, amount: f64, c: Option<CityId>) -> f64 {
        let Some((buyer, seller, p)) = c.and_then(|c| self.price_setup(t, c)) else {
            return 0.0;
        };
        let curve = self.deal_price_curve();
        deal_cost(&buyer, &seller, amount, &p, &|x| curve.eval(x))
    }

    /// `GetDealMaxAmount(type, C)` (§8.1): 0 when `C` is null.
    pub fn get_deal_max_amount(&self, t: u32, c: Option<CityId>) -> f64 {
        let Some((buyer, seller, p)) = c.and_then(|c| self.price_setup(t, c)) else {
            return 0.0;
        };
        deal_max_amount(&buyer, &seller, &p)
    }

    fn free_landfill(&self) -> f64 {
        let (usage, capacity) = self.host.landfill_usage();
        f64::from(capacity.wrapping_sub(usage))
    }

    /// `UpdateCommodityStats(city)` (§8.2): only for the current city.
    ///
    /// The subject is the city object (`cISC4City`), compared by identity
    /// with the current one (NB12, `0x006D2A2C`–`0x006D2A46`): null or any
    /// other handle, our own regional city's included, does nothing. `R` is
    /// then the current regional city; *[choice]* without one this returns
    /// at once (the original does not null-check it and crashes,
    /// `0x006D2A6C`).
    pub fn update_commodity_stats(&self, city: Option<CityHandle>) {
        let Some(c) = city else { return };
        if self.host.current_city_object() != Some(c) {
            return;
        }
        let Some(r) = self.host.current_city() else { return };
        // 1.
        let f = self.host.utility_figures();
        self.host.set_city_budget(r, f.total_funds);
        self.host.set_city_income(r, f.monthly_income);
        // 2.
        let (mut p, mut w, mut g) = (0i64, 0i64, 0i64);
        for d in self.deals.borrow().iter() {
            match d.deal_type() {
                deal_type::BUY_POWER => p = p.wrapping_add(d.cost()),
                deal_type::BUY_WATER => w = w.wrapping_add(d.cost()),
                deal_type::BUY_GARBAGE => g = g.wrapping_add(d.cost()),
                _ => {}
            }
        }
        // 3.
        self.push(true, true);
        // 4.
        let tuning = self.tuning();
        for c in 1..=3 {
            let imp = self.host.city_stat(r, Stat::Imported, c);
            let exp = self.host.city_stat(r, Stat::Exported, c);
            let (produced, demanded, cpu) = match c {
                1 => {
                    let produced = f.power_production;
                    let cpu = f
                        .power_department_expenses
                        .map_or(MISSING_DEPARTMENT_COST, |e| (p as f64 + e) / (produced + imp + 1.0));
                    (produced, f.power_requirement, cpu)
                }
                2 => {
                    let produced = f.water_capacity;
                    let cpu = f
                        .water_department_expenses
                        .map_or(MISSING_DEPARTMENT_COST, |e| (w as f64 + e) / (produced + imp + 1.0));
                    (produced, f.water_demand, cpu)
                }
                _ => {
                    // [choice] (NB13) A missing 0xEA46934F makes the
                    // free-landfill term 0 (§12): the original divides by an
                    // uninitialized stack float (0x006D2AE3), so its result
                    // is undefined. A present value divides literally (0
                    // gives +inf, or NaN with no free capacity).
                    let landfill = match tuning.max_term_years {
                        None => 0.0,
                        Some(years) => self.free_landfill() / (12.0 * f64::from(years)),
                    };
                    let produced = f.garbage_recycled
                        + f.waste_to_energy_capacity
                        + f.incinerator_capacity
                        + landfill
                        + f.garbage_abducted;
                    let cpu = match f.sanitation_department_expenses {
                        None => MISSING_DEPARTMENT_COST,
                        Some(e) => {
                            self.load_garbage_curve();
                            let x = (f.waste_to_energy_expenses + g as f64 + e) / (produced + imp + 1.0);
                            f64::from(self.garbage_cost_curve().eval(x as f32))
                        }
                    };
                    (produced, f.garbage_produced, cpu)
                }
            };
            self.host.set_city_stat(r, Stat::Produced, c, produced);
            self.host.set_city_stat(r, Stat::Demanded, c, demanded);
            self.host.set_city_stat(r, Stat::Exported, c, exp);
            self.host.set_city_stat(r, Stat::Imported, c, imp);
            self.host.set_city_stat(r, Stat::CostPerUnit, c, cpu);
            let series: &[(u32, f64)] = match c {
                1 => &[(history::POWER_IMPORTED, imp), (history::POWER_EXPORTED, exp)],
                2 => &[(history::WATER_IMPORTED, imp), (history::WATER_EXPORTED, exp)],
                _ => &[
                    (history::GARBAGE_EXPORTED, exp),
                    (history::GARBAGE_IMPORTED, imp),
                    (history::GARBAGE_DEMANDED, demanded),
                    (history::GARBAGE_PRODUCED, produced),
                ],
            };
            for &(s, v) in series {
                self.host.send_history(s, trunc_i32(v));
            }
        }
    }

    /// §8.3: the garbage curve `0x4B61FFC9` goes into the one curve object
    /// (or, with two curves, into its own) when the property is present.
    fn load_garbage_curve(&self) {
        let holder = self.tuning_holder.borrow().clone();
        if let Some(c) = curve_of(holder.as_deref(), prop::GARBAGE_COST_CURVE) {
            if self.two_curves.get() {
                *self.garbage_curve.borrow_mut() = c;
            } else {
                *self.curve.borrow_mut() = c;
            }
        }
    }

    fn garbage_cost_curve(&self) -> ResponseCurve {
        if self.two_curves.get() {
            self.garbage_curve.borrow().clone()
        } else {
            self.curve.borrow().clone()
        }
    }

    // ---- region push (§9.2) ----

    /// `update_deals_in_regional_cities(deals, stats)` (§9.2): for each
    /// adjacent established city (in reverse list order, foundations §11.4
    /// step 3), then the current regional city: `SetLocalDeals(list)` and/or
    /// `UpdateImportExport()`; then the pushed flags are cleared.
    pub fn push(&self, deals: bool, stats: bool) {
        if !deals && !stats {
            return;
        }
        let mut cities: Vec<CityId> = self.adjacent_cities(false).into_iter().rev().collect();
        if let Some(r) = self.host.current_city() {
            cities.push(r);
        }
        for c in cities {
            if deals {
                // The region may prune the list it is given (foundations
                // §11.3 [quirk]): the region gets a copy (no borrow is held
                // across the call) and the deals it removed leave ours.
                let mut list = self.find_deals_all();
                let before = list.len();
                self.host.set_local_deals(c, &mut list);
                if list.len() != before {
                    self.deals
                        .borrow_mut()
                        .retain(|d| list.iter().any(|x| Rc::ptr_eq(x, d)));
                }
            }
            if stats {
                self.host.update_import_export(c);
            }
        }
        if deals {
            self.deals_dirty.set(false);
        }
        if stats {
            self.stats_dirty.set(false);
        }
    }

    // ---- the deal simulation (§7.2, §7.3) ----

    /// `simulate_deals` (§7.3).
    pub fn simulate_deals(&self) {
        if !self.ready.get() {
            return;
        }
        if self.busy.get() {
            self.host.post_status(None, status::RETRY);
            return;
        }
        self.busy.set(true);
        let city = self.host.current_city();
        self.update_commodity_stats(self.host.current_city_object());
        if !self.ui_disabled.get() {
            self.run_deals(city);
            self.update_commodity_stats(self.host.current_city_object());
        }
        self.push(self.deals_dirty.get(), self.stats_dirty.get());
        self.busy.set(false);
    }

    fn run_deals(&self, r: Option<CityId>) {
        let tuning = self.tuning();
        // The deal simulation's default for 0xEA46934F is 0 (§12).
        let months = 12.0 * f64::from(tuning.max_term_years.unwrap_or(0.0));
        let k = months + LANDFILL_HORIZON_EPSILON;
        let free = self.free_landfill();
        let mut export_budget = r.map_or(0.0, |r| self.host.city_stat(r, Stat::Exported, 3));
        let today = self.host.today();
        let grace = f64::from(tuning.grace_days);
        let probation = f64::from(tuning.probation_days);
        let mut removed: Vec<Rc<NeighborDeal>> = Vec::new();
        for d in self.find_deals_all() {
            if d.is_silent() {
                continue;
            }
            // 1.
            self.update_deal_state(&d);
            // 2.
            if d.deal_type() == deal_type::SELL_GARBAGE {
                if let Some(r) = r {
                    let have = self.host.city_stat(r, Stat::Imported, 3)
                        + self.host.city_stat(r, Stat::Produced, 3)
                        + free * (1.0 - 1.0 / k);
                    if have < export_budget {
                        export_budget -= f64::from(d.amount());
                        d.set_flags(flag::NO_LANDFILL, true);
                    }
                }
            }
            // 3.
            if !d.check_flags_or(flag::BLOCKING) {
                if d.check_flags_or(flag::PROPOSED) {
                    if d.activate() {
                        d.set_current_date(today);
                        d.report(status::ACTIVATED);
                        self.stats_dirty.set(true);
                    }
                } else if d.check_flags_or(flag::ACTIVE) {
                    d.set_current_date(today);
                    let t = f64::from(today);
                    let suspension = f64::from(d.suspension_date());
                    if d.check_flags_or(flag::GRACE) {
                        if t >= suspension + grace {
                            d.set_flags(flag::GRACE, false);
                            d.set_flags(flag::PROBATION, true);
                        }
                    } else if d.check_flags_or(flag::PROBATION) {
                        if t >= suspension + grace + probation {
                            d.set_flags(flag::PROBATION, false);
                        }
                    } else if d.check_flags_or(flag::EXPIRED) {
                        d.set_flags(flag::EXPIRED, false);
                        let payer = if deal_type::is_sell(d.deal_type()) {
                            d.other_city()
                        } else {
                            r
                        };
                        d.update_cost();
                        // [choice] (NB16) A missing payer cannot afford.
                        // The original does not null-check it (0x006D13A5)
                        // but cannot reach the case; the core can, through
                        // the NB15 choice.
                        let affordable = payer.is_some_and(|p| self.host.city_budget(p) >= d.cost() as f64);
                        if !affordable {
                            d.set_flags(flag::CANNOT_AFFORD, true);
                        } else {
                            let span = d.expiration_date().wrapping_sub(d.start_date());
                            d.set_current_date(today);
                            d.set_start_date(today);
                            d.set_expiration_date(today.wrapping_add(span));
                            d.report(status::RENEWED);
                        }
                    } else if today >= d.expiration_date() {
                        d.set_flags(flag::EXPIRED, true);
                        d.report(status::EXPIRED);
                    }
                } else {
                    d.cancel();
                }
            } else {
                let removal = if d.check_flags_or(flag::CANCELLED) {
                    Some(status::CANCELLED)
                } else if !d.check_flags_or(flag::PROBATION) {
                    // The first strike.
                    d.set_flags(flag::GRACE, true);
                    d.set_flags(flag::FAILURES, false);
                    d.set_suspension_date(today);
                    None
                } else if d.check_flags_or(flag::NO_CONNECTION) {
                    Some(status::NO_CONNECTION)
                } else if d.check_flags_or(flag::CANNOT_AFFORD) {
                    Some(status::CANNOT_AFFORD)
                } else if d.check_flags_or(flag::NO_LANDFILL) {
                    Some(status::NO_LANDFILL)
                } else {
                    None
                };
                if let Some(s) = removal {
                    d.report(s);
                    d.set_amount(0);
                    removed.push(d.clone());
                }
            }
        }
        if !removed.is_empty() {
            self.deals_dirty.set(true);
            self.deals
                .borrow_mut()
                .retain(|d| !removed.iter().any(|x| Rc::ptr_eq(x, d)));
        }
    }

    // ---- persistence (§13) ----

    /// §13.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        let conns = self.all_connections();
        out.put_u16(SIMULATOR_RECORD_VERSION);
        out.put_u32(conns.len() as u32);
        for c in conns {
            // A foreign connection (§15.2) is written by its own object.
            let r = if c.is_foreign() {
                RefValue::Host {
                    token: c.token(),
                    class_id: CLSID_NEIGHBOR_CONNECTION,
                }
            } else {
                RefValue::Object(c as Rc<dyn SaveObject>)
            };
            out.put_ref(&r);
        }
        out.error() == 0
    }

    /// §13: fails unless `version == 2`; each reference resolved as a
    /// connection is appended (the list is not cleared first). Failing
    /// references (wrong interface, unknown class) are skipped, as in the
    /// original. *[choice]* (NB7) A null reference is skipped too; the
    /// original appends it and then releases it unchecked (`0x006CFEC5`),
    /// crashing inside `Read`. A version other than 2 fails right after the
    /// `u16`, and the store ignores the result (NB11).
    ///
    /// A host object (§15.2: the original connection class, when it is not
    /// replaced) is accepted as a foreign connection; the reference the
    /// stream handed over is the list's. Its state is not read here: the
    /// object may still be loading (foundations §2.7, `loadNow = false`).
    pub fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.get_u16() != Some(SIMULATOR_RECORD_VERSION) {
            return false;
        }
        let n = inp.get_u32().unwrap_or(0);
        for _ in 0..n {
            if inp.error() != 0 {
                break;
            }
            let Some(r) = inp.get_ref_as(IID_NEIGHBOR_CONNECTION) else {
                break;
            };
            match r {
                RefValue::Host { token, .. } if token != 0 => {
                    let c = NeighborConnection::foreign(token, &self.host);
                    self.connections.borrow_mut().push(c);
                }
                r => {
                    if let Some(c) = r.object::<NeighborConnection>() {
                        self.connections.borrow_mut().push(c);
                    }
                }
            }
        }
        inp.error() == 0
    }
}

/// The marker orientation by side (§6.3): `flag = false` West 3, North 0,
/// East 1, South 2; `flag = true` the opposite (1, 2, 3, 0). *[choice]* 0
/// for an interior cell.
pub fn marker_orientation(side: Side, flip: bool) -> i32 {
    match (side, flip) {
        (Side::West, false) => 3,
        (Side::North, false) => 0,
        (Side::East, false) => 1,
        (Side::South, false) => 2,
        (Side::West, true) => 1,
        (Side::North, true) => 2,
        (Side::East, true) => 3,
        (Side::South, true) => 0,
        (Side::Interior, _) => 0,
    }
}

impl SaveObject for NeighborsSimulator {
    fn class_id(&self) -> u32 {
        CLSID_NEIGHBORS_SIMULATOR
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        NeighborsSimulator::write(self, out)
    }

    fn read(&self, inp: &mut dyn InStream) -> bool {
        NeighborsSimulator::read(self, inp)
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn std::any::Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
