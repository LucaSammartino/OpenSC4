// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/power.md`: a scripted city implementing
//! `PowerHost` (occupants with their properties, footprints, poles, lines
//! and network pieces; lots; zones; budget departments, line items and
//! budget items; neighbour deals and connections; the ordinance factor;
//! the 3D view's input control) and ops that drive the power simulator.
//! See README.md ("Power ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::{Rc, Weak};

use fc4_core::grid::{create_for_clsid, Bitmap, CellRect, CityDims, GridProvider, SimGrid, TractGrid};
use fc4_core::power::{
    msg, query_interface, shipped_tuning, BudgetItem, Deal, LineInfo, PoleInfo, Posted, PowerHost, PowerInterface,
    PowerMessage, PowerSim, PowerState, CLSID_POWER_SIMULATOR, IID_DEPARTMENT_BUDGET, IID_OCCUPANT, IID_POWER_POLE,
    SLOT_TABLE,
};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::CrtRand;
use fc4_core::store::{load_store, save_store, LoadRegistry, Package};
use fc4_core::stream::{ByteReader, ByteWriter, InStream, OutStream, RefValue, SaveObject, SaveRegistry};
use fc4_core::variant::{Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

/// Test-only class IDs of the host objects a save refers to.
const OCCUPANT_CLASS: u32 = 0x0CC0_0001;
const DEPARTMENT_CLASS: u32 = 0x0CC0_0002;

/// Tokens of objects the world creates (departments, line items).
const FIRST_TOKEN: u64 = 0x1_0000;

#[derive(Clone, Default)]
struct Occ {
    t: u32,
    props: PropertyHolder,
    pos: [f32; 3],
    bbox: ([f32; 3], [f32; 3]),
    cells: CellRect,
    lot: Option<u64>,
    building_type: Option<u32>,
    /// QI to the type's interface (pole, line, network) succeeds.
    qi: bool,
    cell: (i32, i32),
    ignored: bool,
    preview: bool,
    flags: u32,
    line_poles: Option<(u64, u64)>,
    in_manager: bool,
    distressed: bool,
    budget: Option<BudgetItem>,
}

#[derive(Clone, Copy, Default)]
struct Lot {
    rect: CellRect,
    state: u32,
    needs_power: bool,
    has_power: bool,
}

#[derive(Clone, Default)]
struct Dept {
    token: u64,
    funding: f32,
    budget_funding: i64,
    /// line → line item token.
    lines: BTreeMap<u32, u64>,
}

#[derive(Clone, Copy, Default)]
struct LineItem {
    secondary: i32,
    expenses: i64,
}

#[derive(Clone, Copy, Default)]
struct DealS {
    t: u32,
    city: u64,
    amount: i32,
    used: i32,
    flags: u32,
}

#[derive(Clone, Copy, Default)]
struct Conn {
    city: u64,
    cell: (i32, i32),
    visited: bool,
    active: bool,
    other: Option<u64>,
}

/// One saved city plus the host objects its records refer to.
#[derive(Clone)]
struct SavedCity {
    pkg: Package,
    root: Vec<u8>,
    hosts: HashMap<u32, u64>,
}

#[derive(Default)]
pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<Rc<PowerSim>>>,
    next_token: Cell<u64>,
    saves: RefCell<BTreeMap<String, SavedCity>>,
    cells: Cell<(i32, i32)>,
    widths: Cell<(f32, f32)>,
    factor: Cell<f32>,
    sim_time: Cell<u32>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    lots: RefCell<BTreeMap<u64, Lot>>,
    zones: RefCell<BTreeMap<(i32, i32), u32>>,
    depts: RefCell<BTreeMap<u32, Dept>>,
    department_ok: Cell<bool>,
    line_items: RefCell<BTreeMap<u64, LineItem>>,
    deals: RefCell<Vec<(u64, DealS)>>,
    conns: RefCell<Vec<(u64, Conn)>>,
    view_control: Cell<Option<u32>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    grids: RefCell<Vec<Rc<SimGrid<u8>>>>,
    crt_mark: Cell<CrtRand>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            next_token: Cell::new(FIRST_TOKEN),
            cells: Cell::new((256, 256)),
            widths: Cell::new((16.0, 16.0)),
            factor: Cell::new(1.0),
            department_ok: Cell::new(true),
            tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
            ..Default::default()
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn sim(&self) -> R<Rc<PowerSim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no power simulator (power_create)".into())
    }

    fn token(&self) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        t
    }

    fn occ<T>(&self, occ: u64, f: impl FnOnce(&Occ) -> T) -> Option<T> {
        self.occs.borrow().get(&occ).map(f)
    }

    fn dept_by_token(&self, token: u64) -> Option<u32> {
        self.depts
            .borrow()
            .iter()
            .find(|(_, d)| d.token == token)
            .map(|(&id, _)| id)
    }

    fn deal_mut(&self, deal: u64, f: impl FnOnce(&mut DealS)) {
        if let Some((_, d)) = self.deals.borrow_mut().iter_mut().find(|(t, _)| *t == deal) {
            f(d);
        }
    }

    fn conn<T>(&self, conn: u64, f: impl FnOnce(&mut Conn) -> T) -> Option<T> {
        self.conns
            .borrow_mut()
            .iter_mut()
            .find(|(t, _)| *t == conn)
            .map(|(_, c)| f(c))
    }

    fn line_item<T>(&self, item: u64, f: impl FnOnce(&mut LineItem) -> T) -> Option<T> {
        self.line_items.borrow_mut().get_mut(&item).map(f)
    }

    fn make_line_item(&self, dept: u64, line: u32) -> Option<u64> {
        let id = self.dept_by_token(dept)?;
        let t = self.token();
        self.line_items.borrow_mut().insert(t, LineItem::default());
        self.depts.borrow_mut().get_mut(&id)?.lines.insert(line, t);
        Some(t)
    }

    /// The (department ID, line) of a line item token.
    fn line_of(&self, item: u64) -> Json {
        for (id, d) in self.depts.borrow().iter() {
            for (&line, &t) in &d.lines {
                if t == item {
                    return Json::Array(vec![hx(*id), ji(line)]);
                }
            }
        }
        Json::Null
    }

    fn dept_json(&self, dept: u64) -> Json {
        self.dept_by_token(dept).map_or(Json::Null, hx)
    }
}

impl GridProvider<u8> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        let (x, z) = self.cells.get();
        let (wx, wz) = self.widths.get();
        let city = CityDims {
            cells_x: x,
            cells_z: z,
            cell_width_x: wx,
            cell_width_z: wz,
        };
        self.events.push(
            "create_grid",
            vec![("instance", hx(instance)), ("size", ji(tract_size))],
        );
        let g = SimGrid::<u8>::create(&city, instance, tract_size);
        self.grids.borrow_mut().push(g.clone());
        Some(g as Rc<dyn TractGrid<u8>>)
    }
}

/// An ID as an exact `"0x%08X"` string (numbers compare with a relative
/// tolerance, which would let neighbouring IDs match).
fn hx(v: u32) -> Json {
    Json::Str(format!("0x{v:08X}"))
}

fn rect_json(r: CellRect) -> Json {
    Json::Array(vec![ji(r.x1), ji(r.z1), ji(r.x2), ji(r.z2)])
}

impl PowerHost for World {
    fn cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn cell_widths(&self) -> (f32, f32) {
        self.widths.get()
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn cumulative_factor(&self, id: u32, index: u32) -> f32 {
        self.events.push("factor", vec![("id", hx(id)), ("index", ji(index))]);
        self.factor.get()
    }
    fn sim_time(&self) -> u32 {
        self.sim_time.get()
    }
    fn acquire_services(&self) {
        self.events.push("acquire_services", vec![]);
    }
    fn release_services(&self) {
        self.events.push("release_services", vec![]);
    }

    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.events.push(
            "add_agent",
            vec![
                ("level", ji(level)),
                ("name", Json::Str(name.into())),
                ("flags", ji(flags)),
            ],
        );
    }
    fn remove_agent(&self) {
        self.events.push("remove_agent", vec![]);
    }
    fn subscribe(&self, msg_types: &[u32]) {
        let t = msg_types.iter().map(|&m| hx(m)).collect();
        self.events.push("subscribe", vec![("types", Json::Array(t))]);
    }
    fn unsubscribe(&self, msg_types: &[u32]) {
        let t = msg_types.iter().map(|&m| hx(m)).collect();
        self.events.push("unsubscribe", vec![("types", Json::Array(t))]);
    }
    fn post(&self, m: &Posted) {
        let data = m.data.iter().map(|&d| ji(d)).collect();
        self.events.push(
            "post",
            vec![
                ("type", hx(m.msg_type)),
                ("data", Json::Array(data)),
                ("unknown", m.unknown.map_or(Json::Null, |u| ji(u as i64))),
            ],
        );
    }
    fn record_history(&self, series: u32, value: i32) {
        self.events
            .push("history", vec![("series", hx(series)), ("value", ji(value))]);
    }
    fn set_history_queue_length(&self, series: u32, length: u32) {
        self.events
            .push("history_queue", vec![("series", hx(series)), ("length", ji(length))]);
    }

    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
    }
    fn object_ref(&self, token: u64) -> RefValue {
        let class_id = if self.occs.borrow().contains_key(&token) {
            OCCUPANT_CLASS
        } else {
            DEPARTMENT_CLASS
        };
        RefValue::Host { token, class_id }
    }
    fn resolve_ref(&self, r: &RefValue, iid: u32) -> Option<u64> {
        let RefValue::Host { token, class_id } = r else {
            return None;
        };
        let ok = match (*class_id, iid) {
            (OCCUPANT_CLASS, IID_OCCUPANT) => true,
            (OCCUPANT_CLASS, IID_POWER_POLE) => self.occ(*token, |o| o.qi && o.t == 0x2890_D4DE) == Some(true),
            (DEPARTMENT_CLASS, IID_DEPARTMENT_BUDGET) => true,
            _ => false,
        };
        if ok {
            Some(*token)
        } else {
            self.release(*token);
            None
        }
    }

    fn occupant_type(&self, occ: u64) -> u32 {
        self.occ(occ, |o| o.t).unwrap_or(0)
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.occs.borrow().get(&occ)?.props.get(id).cloned()
    }
    fn set_occupant_property(&self, occ: u64, id: u32, value: Variant) {
        let shown = match &value {
            Variant::Scalar(Elem::F32(v)) => jf(*v),
            Variant::Scalar(e) => ji(e.as_i64()),
            _ => Json::Null,
        };
        self.events.push(
            "set_property",
            vec![("occupant", ji(occ as i64)), ("id", hx(id)), ("value", shown)],
        );
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.props.add(id, value);
        }
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        self.occ(occ, |o| o.pos).unwrap_or_default()
    }
    fn occupant_bounding_box(&self, occ: u64) -> ([f32; 3], [f32; 3]) {
        self.occ(occ, |o| o.bbox).unwrap_or_default()
    }
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect {
        self.occ(occ, |o| o.cells).unwrap_or_default()
    }
    fn occupant_in_manager(&self, occ: u64) -> bool {
        self.occ(occ, |o| o.in_manager).unwrap_or(false)
    }
    fn building_type(&self, occ: u64) -> Option<u32> {
        self.occ(occ, |o| o.building_type).flatten()
    }
    fn power_pole(&self, occ: u64) -> Option<PoleInfo> {
        self.occ(occ, |o| {
            (o.qi && o.t == 0x2890_D4DE).then_some(PoleInfo {
                ignored: o.ignored,
                cell: o.cell,
            })
        })
        .flatten()
    }
    fn break_all_connections(&self, pole: u64, flag: bool) {
        self.events.push(
            "break_connections",
            vec![("pole", ji(pole as i64)), ("flag", Json::Bool(flag))],
        );
    }
    fn pole_is_distressed(&self, pole: u64) -> bool {
        self.occ(pole, |o| o.distressed).unwrap_or(false)
    }
    fn set_pole_distressed(&self, pole: u64, on: bool) {
        self.events
            .push("distressed", vec![("pole", ji(pole as i64)), ("on", Json::Bool(on))]);
        if let Some(o) = self.occs.borrow_mut().get_mut(&pole) {
            o.distressed = on;
        }
    }
    fn power_line(&self, occ: u64) -> Option<LineInfo> {
        self.occ(occ, |o| {
            (o.qi && o.t == 0x0990_B2EA).then_some(LineInfo {
                preview: o.preview,
                cell: o.cell,
            })
        })
        .flatten()
    }
    fn line_connected_poles(&self, line: u64) -> Option<(u64, u64)> {
        self.occ(line, |o| o.line_poles).flatten()
    }
    fn remove_pole_connection(&self, pole: u64, line: u64) {
        self.events.push(
            "remove_connection",
            vec![("pole", ji(pole as i64)), ("line", ji(line as i64))],
        );
    }
    fn network_occupant_cell(&self, occ: u64) -> Option<(i32, i32)> {
        self.occ(occ, |o| (o.qi && o.t == 0xC772_BF98).then_some(o.cell))
            .flatten()
    }
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool {
        self.occ(occ, |o| o.flags & flag != 0).unwrap_or(false)
    }
    fn network_flags(&self, occ: u64) -> u32 {
        self.occ(occ, |o| o.flags).unwrap_or(0)
    }
    fn occupants_in_cells(&self, rect: CellRect) -> Vec<u64> {
        self.events.push("occupants_in_cells", vec![("rect", rect_json(rect))]);
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| {
                let c = match o.t {
                    0x2890_D4DE | 0x0990_B2EA | 0xC772_BF98 => CellRect::new(o.cell.0, o.cell.1, o.cell.0, o.cell.1),
                    _ => o.cells,
                };
                o.in_manager && c.x1 <= rect.x2 && c.x2 >= rect.x1 && c.z1 <= rect.z2 && c.z2 >= rect.z1
            })
            .map(|(&id, _)| id)
            .collect()
    }
    fn remove_occupant(&self, occ: u64) {
        self.events.push("remove_occupant", vec![("occupant", ji(occ as i64))]);
    }

    fn zone_type(&self, x: i32, z: i32) -> u32 {
        self.zones.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn power_line_tool_zone_change(&self, rect: CellRect, data3: u32) {
        self.events.push(
            "tool_zone_change",
            vec![("rect", rect_json(rect)), ("data3", ji(data3))],
        );
    }

    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, l)| l.rect.x1 <= x && x <= l.rect.x2 && l.rect.z1 <= z && z <= l.rect.z2)
            .map(|(&id, _)| id)
    }
    fn occupant_lot(&self, occ: u64) -> Option<u64> {
        self.occ(occ, |o| o.lot).flatten()
    }
    fn as_lot(&self, token: u64) -> Option<u64> {
        self.lots.borrow().contains_key(&token).then_some(token)
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.lots.borrow().get(&lot).map(|l| l.rect).unwrap_or_default()
    }
    fn lot_state(&self, lot: u64) -> u32 {
        self.lots.borrow().get(&lot).map_or(0, |l| l.state)
    }
    fn lot_needs_power(&self, lot: u64) -> bool {
        self.lots.borrow().get(&lot).is_some_and(|l| l.needs_power)
    }
    fn lot_has_power(&self, lot: u64) -> bool {
        self.lots.borrow().get(&lot).is_some_and(|l| l.has_power)
    }
    fn set_lot_has_power(&self, lot: u64, on: bool) {
        self.events
            .push("lot_has_power", vec![("lot", ji(lot as i64)), ("on", Json::Bool(on))]);
        if let Some(l) = self.lots.borrow_mut().get_mut(&lot) {
            l.has_power = on;
        }
    }
    fn set_lot_condition(&self, lot: u64, condition: u32) {
        self.events.push(
            "lot_condition",
            vec![("lot", ji(lot as i64)), ("condition", ji(condition))],
        );
    }
    fn demolish_lots(&self, lots: &[u64], flags: u32) {
        let l = lots.iter().map(|&t| ji(t as i64)).collect();
        self.events
            .push("demolish", vec![("lots", Json::Array(l)), ("flags", ji(flags))]);
    }
    fn add_radiation(&self, position: [f32; 3], strength: f32, radius: f32) {
        self.events.push(
            "radiation",
            vec![
                ("position", Json::Array(position.iter().map(|&v| jf(v)).collect())),
                ("strength", jf(strength)),
                ("radius", jf(radius)),
            ],
        );
    }
    fn current_view_input_control(&self) -> Option<u32> {
        self.view_control.get()
    }
    fn remove_current_view_input_control(&self) {
        self.events.push("remove_view_control", vec![]);
    }

    fn find_deals(&self, deal_type: u32) -> Vec<Deal> {
        self.deals
            .borrow()
            .iter()
            .filter(|(_, d)| d.t == deal_type)
            .map(|(t, d)| Deal {
                token: *t,
                city: d.city,
                amount: d.amount,
            })
            .collect()
    }
    fn deal_amount_used(&self, deal: u64) -> i32 {
        self.deals
            .borrow()
            .iter()
            .find(|(t, _)| *t == deal)
            .map_or(0, |(_, d)| d.used)
    }
    fn set_deal_amount_used(&self, deal: u64, used: i32) {
        self.events
            .push("deal_used", vec![("deal", ji(deal as i64)), ("used", ji(used))]);
        self.deal_mut(deal, |d| d.used = used);
    }
    fn set_deal_flags(&self, deal: u64, flags: u32, on: bool) {
        self.events.push(
            "deal_flags",
            vec![("deal", ji(deal as i64)), ("flags", ji(flags)), ("on", Json::Bool(on))],
        );
        self.deal_mut(deal, |d| {
            if on {
                d.flags |= flags;
            } else {
                d.flags &= !flags;
            }
        });
    }
    fn deal_type(&self, token: u64) -> Option<u32> {
        self.deals.borrow().iter().find(|(t, _)| *t == token).map(|(_, d)| d.t)
    }
    fn mark_deals_dirty(&self) {
        self.events.push("mark_deals_dirty", vec![]);
    }
    fn connections_of_type(&self, connection_type: u32) -> Vec<u64> {
        if connection_type != 4 {
            return Vec::new();
        }
        self.conns.borrow().iter().map(|(t, _)| *t).collect()
    }
    fn connections_to_city(&self, connection_type: u32, city: u64) -> Vec<u64> {
        if connection_type != 4 {
            return Vec::new();
        }
        self.conns
            .borrow()
            .iter()
            .filter(|(_, c)| c.city == city)
            .map(|(t, _)| *t)
            .collect()
    }
    fn find_unvisited_connection(&self, connection_type: u32, city: u64) -> Option<u64> {
        if connection_type != 4 {
            return None;
        }
        self.conns
            .borrow()
            .iter()
            .find(|(_, c)| c.city == city && !c.visited)
            .map(|(t, _)| *t)
    }
    fn connection_on_other_side(&self, conn: u64) -> Option<u64> {
        self.conn(conn, |c| c.other).flatten()
    }
    fn connection_cell(&self, conn: u64) -> (i32, i32) {
        self.conn(conn, |c| c.cell).unwrap_or_default()
    }
    fn set_connection_visited(&self, conn: u64, on: bool) {
        self.conn(conn, |c| c.visited = on);
    }
    fn connection_is_active(&self, conn: u64) -> bool {
        self.conn(conn, |c| c.active).unwrap_or(false)
    }
    fn set_connection_active(&self, conn: u64, on: bool) {
        self.events
            .push("conn_active", vec![("conn", ji(conn as i64)), ("on", Json::Bool(on))]);
        self.conn(conn, |c| c.active = on);
    }

    fn department(&self, id: u32) -> Option<u64> {
        self.depts.borrow().get(&id).map(|d| d.token)
    }
    fn create_department(&self, id: u32, group: u32) -> Option<u64> {
        self.events
            .push("create_department", vec![("id", hx(id)), ("group", hx(group))]);
        if !self.department_ok.get() {
            return None;
        }
        let token = self.token();
        self.depts.borrow_mut().insert(
            id,
            Dept {
                token,
                funding: 1.0,
                ..Default::default()
            },
        );
        Some(token)
    }
    fn set_department_name(&self, dept: u64, group: u32, instance: u32) {
        self.events.push(
            "department_name",
            vec![
                ("department", self.dept_json(dept)),
                ("group", hx(group)),
                ("instance", hx(instance)),
            ],
        );
    }
    fn set_department_max_funding(&self, dept: u64, fraction: f32) {
        self.events.push(
            "max_funding",
            vec![("department", self.dept_json(dept)), ("value", jf(fraction))],
        );
    }
    fn department_funding(&self, dept: u64) -> f32 {
        self.dept_by_token(dept)
            .and_then(|id| self.depts.borrow().get(&id).map(|d| d.funding))
            .unwrap_or(0.0)
    }
    fn budget_funding(&self, dept: u64) -> i64 {
        self.dept_by_token(dept)
            .and_then(|id| self.depts.borrow().get(&id).map(|d| d.budget_funding))
            .unwrap_or(0)
    }
    fn create_line_item(&self, dept: u64, line: u32, locally_funded: bool) -> Option<u64> {
        self.events.push(
            "create_line_item",
            vec![
                ("department", self.dept_json(dept)),
                ("line", ji(line)),
                ("local", Json::Bool(locally_funded)),
            ],
        );
        self.make_line_item(dept, line)
    }
    fn create_line_item_for_building_type(&self, dept: u64, line: u32, flag: bool) -> Option<u64> {
        self.events.push(
            "create_line_item_for_type",
            vec![
                ("department", self.dept_json(dept)),
                ("line", ji(line)),
                ("flag", Json::Bool(flag)),
            ],
        );
        self.make_line_item(dept, line)
    }
    fn line_item(&self, dept: u64, line: u32) -> Option<u64> {
        let id = self.dept_by_token(dept)?;
        self.depts.borrow().get(&id)?.lines.get(&line).copied()
    }
    fn set_line_item_name(&self, item: u64, group: u32, instance: u32) {
        self.events.push(
            "line_item_name",
            vec![
                ("item", self.line_of(item)),
                ("group", hx(group)),
                ("instance", hx(instance)),
            ],
        );
    }
    fn set_line_item_display_flag(&self, item: u64, flag: u32, on: bool) {
        self.events.push(
            "display_flag",
            vec![("item", self.line_of(item)), ("flag", ji(flag)), ("on", Json::Bool(on))],
        );
    }
    fn line_item_secondary_info(&self, item: u64) -> i32 {
        self.line_item(item, |l| l.secondary).unwrap_or(0)
    }
    fn set_line_item_secondary_info(&self, item: u64, value: i32) {
        self.events.push(
            "secondary_info",
            vec![("item", self.line_of(item)), ("value", ji(value))],
        );
        self.line_item(item, |l| l.secondary = value);
    }
    fn set_line_item_full_expenses(&self, item: u64, value: i64) {
        self.events.push(
            "full_expenses",
            vec![("item", self.line_of(item)), ("value", ji(value))],
        );
        self.line_item(item, |l| l.expenses = value);
    }
    fn budget_item_for_purpose(&self, occ: u64, purpose: u32) -> Option<BudgetItem> {
        if purpose != fc4_core::power::budget::PURPOSE {
            return None;
        }
        self.occ(occ, |o| o.budget).flatten()
    }
    fn change_budget_item_line(&self, occ: u64, index: u32, line: u32) {
        self.events.push(
            "budget_line",
            vec![("occupant", ji(occ as i64)), ("index", ji(index)), ("line", ji(line))],
        );
        if let Some(Some(b)) = self.occs.borrow_mut().get_mut(&occ).map(|o| o.budget.as_mut()) {
            b.line = line;
        }
    }
    fn change_budget_item_cost(&self, occ: u64, index: u32, cost: i64) {
        self.events.push(
            "budget_cost",
            vec![("occupant", ji(occ as i64)), ("index", ji(index)), ("cost", ji(cost))],
        );
        if let Some(Some(b)) = self.occs.borrow_mut().get_mut(&occ).map(|o| o.budget.as_mut()) {
            b.cost = cost;
        }
    }
    fn add_locally_funded_object(&self, dept: u64, occ: u64, purpose: u32) {
        self.events.push(
            "add_funded",
            vec![
                ("department", self.dept_json(dept)),
                ("occupant", ji(occ as i64)),
                ("purpose", hx(purpose)),
            ],
        );
    }
    fn remove_locally_funded_object(&self, dept: u64, occ: u64, purpose: u32) {
        self.events.push(
            "remove_funded",
            vec![
                ("department", self.dept_json(dept)),
                ("occupant", ji(occ as i64)),
                ("purpose", hx(purpose)),
            ],
        );
    }
    fn is_locally_funded_object(&self, _dept: u64, occ: u64, _purpose: u32) -> bool {
        self.occ(occ, |o| o.budget.is_some()).unwrap_or(false)
    }
    fn set_local_funding_percent(&self, dept: u64, occ: u64, fraction: f32, purpose: u32) -> bool {
        self.events.push(
            "local_funding",
            vec![
                ("department", self.dept_json(dept)),
                ("occupant", ji(occ as i64)),
                ("value", jf(fraction)),
                ("purpose", hx(purpose)),
            ],
        );
        if let Some(Some(b)) = self.occs.borrow_mut().get_mut(&occ).map(|o| o.budget.as_mut()) {
            b.local_funding = fraction;
        }
        true
    }
    fn set_local_full_funding(&self, dept: u64, occ: u64, amount: i64, purpose: u32) {
        self.events.push(
            "local_full_funding",
            vec![
                ("department", self.dept_json(dept)),
                ("occupant", ji(occ as i64)),
                ("amount", ji(amount)),
                ("purpose", hx(purpose)),
            ],
        );
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "power", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("power_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "tuning" => tuning(&w, op),
        "create" => (|| {
            let s = PowerSim::new(w.rc() as Rc<dyn PowerHost>, w.token());
            if let Some(seed) = opt(op, "crt_seed") {
                s.set_crt(CrtRand::new(u32_of(seed, "crt_seed")?));
            }
            w.crt_mark.set(s.crt());
            *w.sim.borrow_mut() = Some(s);
            Ok(())
        })(),
        "init" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => message(&w, op),
        "idle" => idle(&w, op),
        "cycle" => cycle(&w, op),
        "call" => call(&w, op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "expect_bitmap" => expect_bitmap(&w, op),
        "expect_grid" => expect_grid(&w, op),
        "expect_occupant" => expect_occupant(&w, op),
        "expect_world" => expect_world(&w, op),
        "expect_rand" => expect_rand(&w, op),
        "expect_refs" => expect_refs(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect_save" => expect_save(&w, op),
        "patch" => patch(&w, op),
        "expect_payload" => expect_payload(&w, op),
        "read" => read_record(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- environment ----------------------------------------------------------------------------------

fn i32_of(j: &Json, what: &str) -> R<i32> {
    Ok(i64_of(j, what)? as i32)
}

fn pair(j: &Json, what: &str) -> R<(i32, i32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be [x, z]"));
    }
    Ok((i32_of(&a[0], what)?, i32_of(&a[1], what)?))
}

fn rect_of(j: &Json, what: &str) -> R<CellRect> {
    let a = arr(j, what)?;
    if a.len() != 4 {
        return Err(format!("'{what}' must be [x1, z1, x2, z2]"));
    }
    Ok(CellRect::new(
        i32_of(&a[0], what)?,
        i32_of(&a[1], what)?,
        i32_of(&a[2], what)?,
        i32_of(&a[3], what)?,
    ))
}

fn vec3(j: &Json, what: &str) -> R<[f32; 3]> {
    let a = arr(j, what)?;
    if a.len() != 3 {
        return Err(format!("'{what}' must be [x, y, z]"));
    }
    Ok([f32_of(&a[0], what)?, f32_of(&a[1], what)?, f32_of(&a[2], what)?])
}

fn occupant_type_of(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("building") => 0x2781_28A0,
        Some("pole") => 0x2890_D4DE,
        Some("line") => 0x0990_B2EA,
        Some("network") => 0xC772_BF98,
        _ => u32_of(j, "type")?,
    })
}

fn deal_type_of(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("buy") => 0x4773_FFE0,
        Some("sell") => 0x4773_FFE1,
        _ => u32_of(j, "type")?,
    })
}

fn budget_of(j: &Json) -> R<BudgetItem> {
    Ok(BudgetItem {
        department: opt(j, "department").map_or(Ok(0x8910_BC8A), |v| u32_of(v, "department"))?,
        line: opt(j, "line").map_or(Ok(0), |v| u32_of(v, "line"))?,
        cost: opt(j, "cost").map_or(Ok(0), |v| i64_of(v, "cost"))?,
        index: opt(j, "index").map_or(Ok(0), |v| u32_of(v, "index"))?,
        local_funding: opt(j, "funding").map_or(Ok(1.0), |v| f32_of(v, "funding"))?,
    })
}

/// An occupant from JSON (see README.md, `power_env`).
fn occupant(w: &World, e: &Json) -> R<(u64, Occ)> {
    let id = u32_of(field(e, "id")?, "id")? as u64;
    let t = occupant_type_of(field(e, "type")?)?;
    let (wx, wz) = w.widths.get();
    let cell = match opt(e, "cell") {
        Some(c) => pair(c, "cell")?,
        None => (0, 0),
    };
    let cells = match opt(e, "cells") {
        Some(c) => rect_of(c, "cells")?,
        None => CellRect::new(cell.0, cell.1, cell.0, cell.1),
    };
    let bbox = match opt(e, "bbox") {
        Some(b) => {
            let a = arr(b, "bbox")?;
            if a.len() != 2 {
                return Err("'bbox' must be [[x, y, z], [x, y, z]]".into());
            }
            (vec3(&a[0], "bbox")?, vec3(&a[1], "bbox")?)
        }
        None => (
            [cells.x1 as f32 * wx, 0.0, cells.z1 as f32 * wz],
            [(cells.x2 + 1) as f32 * wx, 0.0, (cells.z2 + 1) as f32 * wz],
        ),
    };
    let pos = match opt(e, "pos") {
        Some(p) => vec3(p, "pos")?,
        None => [(bbox.0[0] + bbox.1[0]) * 0.5, 0.0, (bbox.0[2] + bbox.1[2]) * 0.5],
    };
    let props = match opt(e, "props") {
        Some(p) => scalar::holder_of(p)?,
        None => PropertyHolder::new(),
    };
    let flag = |k: &str, d: bool| opt(e, k).map_or(Ok(d), |v| bool_of(v, k));
    let line_poles = match opt(e, "poles") {
        Some(p) => {
            let (a, b) = pair(p, "poles")?;
            Some((a as u64, b as u64))
        }
        None => None,
    };
    Ok((
        id,
        Occ {
            t,
            props,
            pos,
            bbox,
            cells,
            lot: opt(e, "lot").map(|v| u32_of(v, "lot").map(u64::from)).transpose()?,
            building_type: match e.get("building_type") {
                Some(Json::Null) => None,
                Some(v) => Some(u32_of(v, "building_type")?),
                None => (t == 0x2781_28A0).then_some(0x1234),
            },
            qi: flag("qi", true)?,
            cell,
            ignored: flag("ignored", false)?,
            preview: flag("preview", false)?,
            flags: opt(e, "flags").map_or(Ok(0), |v| u32_of(v, "flags"))?,
            line_poles,
            in_manager: flag("in_manager", true)?,
            distressed: flag("distressed", false)?,
            budget: opt(e, "budget").map(budget_of).transpose()?,
        },
    ))
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        w.cells.set(pair(v, "cells")?);
    }
    if let Some(v) = opt(op, "cell_width") {
        let a = arr(v, "cell_width")?;
        w.widths
            .set((f32_of(&a[0], "cell_width")?, f32_of(&a[1], "cell_width")?));
    }
    if let Some(v) = opt(op, "factor") {
        w.factor.set(f32_of(v, "factor")?);
    }
    if let Some(v) = opt(op, "sim_time") {
        w.sim_time.set(u32_of(v, "sim_time")?);
    }
    if let Some(v) = opt(op, "department_ok") {
        w.department_ok.set(bool_of(v, "department_ok")?);
    }
    if let Some(v) = op.get("view_control") {
        w.view_control.set(match v {
            Json::Null => None,
            v => Some(u32_of(v, "view_control")?),
        });
    }
    if let Some(v) = opt(op, "zones") {
        for z in arr(v, "zones")? {
            let (x0, z0) = (i32_of(field(z, "x")?, "x")?, i32_of(field(z, "z")?, "z")?);
            let wd = opt(z, "w").map_or(Ok(1), |v| i32_of(v, "w"))?;
            let d = opt(z, "d").map_or(Ok(1), |v| i32_of(v, "d"))?;
            let zone = u32_of(field(z, "zone")?, "zone")?;
            for x in x0..x0 + wd {
                for zz in z0..z0 + d {
                    if zone == 0 {
                        w.zones.borrow_mut().remove(&(x, zz));
                    } else {
                        w.zones.borrow_mut().insert((x, zz), zone);
                    }
                }
            }
        }
    }
    if let Some(v) = opt(op, "occupants") {
        for e in arr(v, "occupants")? {
            let (id, o) = occupant(w, e)?;
            w.occs.borrow_mut().insert(id, o);
        }
    }
    if let Some(v) = opt(op, "pole_set") {
        // `{"first": id, "count": n, "cost": c, "origin": [x, z]}`: n poles
        // with IDs first.., cost property 0x09132DE8 = c (i64), at cells
        // origin + (i mod 50, i div 50).
        let first = u32_of(field(v, "first")?, "first")? as u64;
        let n = u32_of(field(v, "count")?, "count")? as u64;
        let cost = opt(v, "cost").map_or(Ok(1), |c| i64_of(c, "cost"))?;
        let (ox, oz) = opt(v, "origin").map_or(Ok((0, 0)), |o| pair(o, "origin"))?;
        for i in 0..n {
            let mut props = PropertyHolder::new();
            props.add(0x0913_2DE8, Variant::Scalar(Elem::I64(cost)));
            let o = Occ {
                t: 0x2890_D4DE,
                props,
                qi: true,
                cell: (ox + (i % 50) as i32, oz + (i / 50) as i32),
                in_manager: true,
                ..Default::default()
            };
            w.occs.borrow_mut().insert(first + i, o);
        }
    }
    if let Some(v) = opt(op, "occupant_props") {
        // {"id": {"0xPROP": value or null}}
        for (k, props) in v.as_object().ok_or("occupant_props must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad occupant id")? as u64;
            let mut occs = w.occs.borrow_mut();
            let o = occs.get_mut(&id).ok_or("unknown occupant")?;
            for (pk, pv) in props.as_object().ok_or("properties must be an object")? {
                let pid = Json::Str(pk.clone()).as_i64().ok_or("bad property id")? as u32;
                match pv {
                    Json::Null => {
                        o.props.remove(pid);
                    }
                    pv => o.props.add(pid, scalar::variant_of(pv)?),
                }
            }
        }
    }
    if let Some(v) = opt(op, "occupant_state") {
        // {"id": {in_manager?, distressed?, budget?}}
        for (k, st) in v.as_object().ok_or("occupant_state must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad occupant id")? as u64;
            let mut occs = w.occs.borrow_mut();
            let o = occs.get_mut(&id).ok_or("unknown occupant")?;
            if let Some(b) = opt(st, "in_manager") {
                o.in_manager = bool_of(b, "in_manager")?;
            }
            if let Some(b) = opt(st, "distressed") {
                o.distressed = bool_of(b, "distressed")?;
            }
            if let Some(b) = st.get("budget") {
                o.budget = match b {
                    Json::Null => None,
                    b => Some(budget_of(b)?),
                };
            }
        }
    }
    if let Some(v) = opt(op, "lots") {
        for l in arr(v, "lots")? {
            let id = u32_of(field(l, "id")?, "id")? as u64;
            let flag = |k: &str| opt(l, k).map_or(Ok(false), |v| bool_of(v, k));
            w.lots.borrow_mut().insert(
                id,
                Lot {
                    rect: rect_of(field(l, "rect")?, "rect")?,
                    state: opt(l, "state").map_or(Ok(0), |v| u32_of(v, "state"))?,
                    needs_power: flag("needs_power")?,
                    has_power: flag("has_power")?,
                },
            );
        }
    }
    if let Some(v) = opt(op, "departments") {
        for d in arr(v, "departments")? {
            let id = u32_of(field(d, "id")?, "id")?;
            let existing = w.depts.borrow().get(&id).cloned();
            let mut dept = existing.unwrap_or_else(|| Dept {
                token: w.token(),
                funding: 1.0,
                ..Default::default()
            });
            if let Some(f) = opt(d, "funding") {
                dept.funding = f32_of(f, "funding")?;
            }
            if let Some(f) = opt(d, "budget_funding") {
                dept.budget_funding = i64_of(f, "budget_funding")?;
            }
            let token = dept.token;
            w.depts.borrow_mut().insert(id, dept);
            if let Some(lines) = opt(d, "lines") {
                for line in arr(lines, "lines")? {
                    w.make_line_item(token, u32_of(line, "line")?);
                }
            }
        }
    }
    if let Some(v) = op.get("deals") {
        let mut deals = Vec::new();
        if !matches!(v, Json::Null) {
            for d in arr(v, "deals")? {
                deals.push((
                    u32_of(field(d, "id")?, "id")? as u64,
                    DealS {
                        t: deal_type_of(field(d, "type")?)?,
                        city: u32_of(field(d, "city")?, "city")? as u64,
                        amount: i32_of(field(d, "amount")?, "amount")?,
                        used: opt(d, "used").map_or(Ok(0), |v| i32_of(v, "used"))?,
                        flags: 0,
                    },
                ));
            }
        }
        *w.deals.borrow_mut() = deals;
    }
    if let Some(v) = op.get("connections") {
        let mut conns = Vec::new();
        if !matches!(v, Json::Null) {
            for c in arr(v, "connections")? {
                conns.push((
                    u32_of(field(c, "id")?, "id")? as u64,
                    Conn {
                        city: u32_of(field(c, "city")?, "city")? as u64,
                        cell: pair(field(c, "cell")?, "cell")?,
                        visited: opt(c, "visited").map_or(Ok(false), |v| bool_of(v, "visited"))?,
                        active: opt(c, "active").map_or(Ok(false), |v| bool_of(v, "active"))?,
                        other: opt(c, "other").map(|v| u32_of(v, "other").map(u64::from)).transpose()?,
                    },
                ));
            }
        }
        *w.conns.borrow_mut() = conns;
    }
    Ok(())
}

fn tuning(w: &World, op: &Json) -> R<()> {
    if opt(op, "none").is_some() {
        *w.tuning.borrow_mut() = None;
        return Ok(());
    }
    let shipped = opt(op, "shipped").map_or(Ok(true), |v| bool_of(v, "shipped"))?;
    let mut h = if shipped {
        shipped_tuning()
    } else {
        PropertyHolder::new()
    };
    if let Some(p) = opt(op, "props") {
        let extra = scalar::holder_of(p)?;
        for id in extra.ids() {
            if let Some(v) = extra.get(id) {
                h.add(id, v.clone());
            }
        }
    }
    if let Some(r) = opt(op, "remove") {
        for id in arr(r, "remove")? {
            h.remove(u32_of(id, "remove")?);
        }
    }
    *w.tuning.borrow_mut() = Some(Rc::new(h));
    Ok(())
}

// ---- calls ----------------------------------------------------------------------------------------

fn msg_type(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("SimNewMonth") => msg::SIM_NEW_MONTH,
        Some("SimNewDay") => msg::SIM_NEW_DAY,
        Some("SimIdle") => msg::SIM_IDLE,
        Some("OccupantInserted") => msg::OCCUPANT_INSERTED,
        Some("OccupantRemoved") => msg::OCCUPANT_REMOVED,
        Some("ZoningChanged") => msg::ZONING_CHANGED,
        Some("TuningReload") => msg::TUNING_RELOAD,
        Some("TerrainChanged") => msg::TERRAIN_CHANGED,
        Some("OrdinanceUpdated") => msg::ORDINANCE_UPDATED,
        Some("StateChanged") => msg::STATE_CHANGED,
        Some("DealStatus") => msg::DEAL_STATUS,
        Some("ConnectionsReady") => msg::CONNECTIONS_READY,
        Some("PostCityInit") => msg::POST_CITY_INIT,
        _ => u32_of(j, "type")?,
    })
}

fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let d = |k: &str| opt(op, k).map_or(Ok(0), |v| u32_of(v, k));
    let m = PowerMessage {
        msg_type: msg_type(field(op, "type")?)?,
        data1: d("data1")?,
        data2: d("data2")?,
        data3: d("data3")?,
        data4: d("data4")?,
        void1: opt(op, "occupant")
            .or_else(|| opt(op, "void1"))
            .map_or(Ok(0), |v| u32_of(v, "occupant"))? as u64,
        rect: opt(op, "rect").map_or(Ok(CellRect::default()), |v| rect_of(v, "rect"))?,
    };
    let count = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    let mut r = true;
    // `occupants`: the message once per listed occupant (`[id, …]` or
    // `{"first": id, "count": n}`), in order.
    let targets: Vec<u64> = match opt(op, "occupants") {
        Some(Json::Array(a)) => a
            .iter()
            .map(|v| u32_of(v, "occupants").map(u64::from))
            .collect::<R<_>>()?,
        Some(o) => {
            let first = u32_of(field(o, "first")?, "first")? as u64;
            let n = u32_of(field(o, "count")?, "count")? as u64;
            (first..first + n).collect()
        }
        None => vec![m.void1],
    };
    for t in targets {
        let m = PowerMessage { void1: t, ..m };
        for _ in 0..count {
            r = s.do_message(&m);
        }
    }
    check_expect(op, Json::Bool(r))
}

fn idle(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let count = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    for _ in 0..count {
        s.do_idle();
    }
    if let Some(v) = opt(op, "expect_state") {
        let want = u32_of(v, "expect_state")?;
        let got = s.state().state;
        if got != want {
            return Err(format!("state {got}, expected {want}"));
        }
    }
    Ok(())
}

/// `power_cycle`: `DoIdle` until the state is back at 4 after leaving it.
fn cycle(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let max = opt(op, "max").map_or(Ok(16), |v| i64_of(v, "max"))?;
    let repeat = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    let request = opt(op, "recompute").map_or(Ok(false), |v| bool_of(v, "recompute"))?;
    let mut calls = 0;
    for _ in 0..repeat {
        if request {
            s.update_state(|st| st.recompute = true);
        }
        calls = 0;
        let mut left = false;
        while calls < max {
            s.do_idle();
            calls += 1;
            let st = s.state().state;
            if st != 4 {
                left = true;
            } else if left {
                break;
            }
        }
    }
    if let Some(v) = opt(op, "expect_calls") {
        if calls != i64_of(v, "expect_calls")? {
            return Err(format!("{calls} DoIdle calls"));
        }
    }
    Ok(())
}

fn a_i32(op: &Json, i: usize) -> R<i32> {
    i32_of(arg(op, i)?, "argument")
}

fn a_occ(op: &Json, i: usize) -> R<u64> {
    Ok(u32_of(arg(op, i)?, "occupant")? as u64)
}

fn opt_u32(v: Option<u32>) -> Json {
    v.map_or(Json::Bool(false), ji)
}

fn call(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let method = str_of(field(op, "method")?, "method")?;
    let got = match method {
        "query_interface" => match query_interface(u32_of(arg(op, 0)?, "iid")?) {
            Some(PowerInterface::Primary) => Json::Str("primary".into()),
            Some(PowerInterface::MessageTarget) => Json::Str("message_target".into()),
            Some(PowerInterface::Serializable) => Json::Str("serializable".into()),
            None => Json::Bool(false),
        },
        "class_id" => hx(s.class_id()),
        "slot_pops" => {
            let slot = u32_of(arg(op, 0)?, "slot")?;
            let e = SLOT_TABLE.iter().find(|e| e.0 == slot).ok_or("no such slot")?;
            ji(e.2)
        }
        "position_to_cell" => {
            let (x, z) = s.position_to_cell(f32_of(arg(op, 0)?, "x")?, f32_of(arg(op, 1)?, "z")?);
            Json::Array(vec![ji(x), ji(z)])
        }
        "cell_to_position" => {
            let (x, z) = s.cell_to_position(a_i32(op, 0)?, a_i32(op, 1)?);
            Json::Array(vec![jf(x), jf(z)])
        }
        "cell_to_standard_city_cell" => {
            let (x, z) = s.cell_to_standard_city_cell(a_i32(op, 0)?, a_i32(op, 1)?);
            Json::Array(vec![ji(x), ji(z)])
        }
        "annual_consumption" => ji(s.annual_consumption()),
        "annual_consumption_by_type" => ji(s.annual_consumption_by_type(a_i32(op, 0)?)),
        "monthly_requirement" => ji(s.monthly_requirement()),
        "monthly_production" => ji(s.monthly_production()),
        "monthly_import" => ji(s.monthly_import()),
        "monthly_export" => ji(s.monthly_export()),
        "monthly_demand" => ji(s.monthly_demand()),
        "monthly_consumption_at" => ji(s.monthly_consumption_at(a_i32(op, 0)?, a_i32(op, 1)?)),
        "monthly_consumption" => ji(s.monthly_consumption()),
        "supply_value" | "is_powered" => Json::Bool(s.is_powered(a_i32(op, 0)?, a_i32(op, 1)?)),
        "is_wired" => Json::Bool(s.is_wired(a_i32(op, 0)?, a_i32(op, 1)?)),
        "unpowered_consumer_count" => ji(s.unpowered_consumer_count()),
        "powered_consumer_count" => ji(s.powered_consumer_count()),
        "power_plant_count_of_type" => ji(s.power_plant_count_of_type(a_i32(op, 0)?)),
        "power_plant_count" => ji(s.power_plant_count()),
        "power_plants" => Json::Array(s.power_plants().into_iter().map(|p| ji(p as i64)).collect()),
        "power_pole_count" => ji(s.power_pole_count()),
        "overworked_plant_count" => ji(s.overworked_plant_count()),
        "aged_plant_count" => ji(s.aged_plant_count()),
        "distressed_plant_count" => ji(s.distressed_plant_count()),
        "find_power_plants" => {
            let f = s.find_power_plants();
            Json::Object(vec![
                (
                    "lowest".into(),
                    f.lowest_funded
                        .map_or(Json::Null, |(o, v)| Json::Array(vec![ji(o as i64), jf(v)])),
                ),
                (
                    "worn".into(),
                    f.most_worn
                        .map_or(Json::Null, |(o, v)| Json::Array(vec![ji(o as i64), ji(v)])),
                ),
            ])
        }
        "is_occupant_powered" => Json::Bool(s.is_occupant_powered(a_occ(op, 0)?)),
        "occupant_power_consumed" => opt_u32(s.occupant_power_consumed(a_occ(op, 0)?)),
        "occupant_power_produced" => s
            .occupant_power_produced(a_occ(op, 0)?)
            .map_or(Json::Bool(false), |(p, u)| Json::Array(vec![ji(p), ji(u)])),
        "occupant_max_power_produced" => opt_u32(s.occupant_max_power_produced(a_occ(op, 0)?)),
        "is_on_strike" => Json::Bool(s.is_on_strike()),
        "begin_strike" => Json::Bool(s.begin_strike()),
        "end_strike" => Json::Bool(s.end_strike()),
        "chance_of_strike" => jf(s.chance_of_strike()),
        "strike_start_date" => ji(s.strike_start_date()),
        "strike_length" => ji(s.strike_length()),
        "current_plant_efficiency" => jf(s.current_plant_efficiency(a_occ(op, 0)?)),
        "current_plant_condition" => jf(s.current_plant_condition(a_occ(op, 0)?)),
        "current_power_produced" => ji(s.current_power_produced(a_occ(op, 0)?)),
        "strike_efficiency" => jf(s.strike_efficiency()),
        "funding_percent" => s.funding_percent().map_or(Json::Null, jf),
        "monthly_generation_cost" => s.monthly_generation_cost(a_occ(op, 0)?).map_or(Json::Bool(false), ji),
        "plant_funding_percentage" => s.plant_funding_percentage(a_occ(op, 0)?).map_or(Json::Bool(false), jf),
        "set_plant_funding_percentage" => {
            Json::Bool(s.set_plant_funding_percentage(a_occ(op, 0)?, f32_of(arg(op, 1)?, "fraction")?))
        }
        "update_power_grid" => {
            s.update_power_grid(a_i32(op, 0)?, a_i32(op, 1)?);
            Json::Null
        }
        "flood" => {
            let (lo, hi) = s.flood(a_i32(op, 0)?, a_i32(op, 1)?);
            Json::Array(vec![ji(lo), ji(hi)])
        }
        "age_plant" => {
            s.age_plant(a_occ(op, 0)?);
            Json::Null
        }
        "apply_pole_funding" => {
            s.apply_pole_funding(f32_of(arg(op, 0)?, "fraction")?);
            Json::Null
        }
        "monthly" => {
            s.monthly();
            Json::Null
        }
        "do_idle" => {
            s.do_idle();
            Json::Null
        }
        "occupant_inserted" => {
            s.occupant_inserted(a_occ(op, 0)?);
            Json::Null
        }
        "occupant_removed" => {
            s.occupant_removed(a_occ(op, 0)?);
            Json::Null
        }
        "cell_changed" => {
            s.cell_changed(rect_of(arg(op, 0)?, "rect")?);
            Json::Null
        }
        "expand" => rect_json(s.expand(rect_of(arg(op, 0)?, "rect")?)),
        "read_tuning" => {
            s.read_tuning();
            Json::Null
        }
        "curve" => {
            // [name, x]
            let name = str_of(arg(op, 0)?, "curve")?;
            let x = f32_of(arg(op, 1)?, "x")?;
            let st = s.state();
            let t = &st.tuning;
            let c = match name {
                "funding_efficiency" => &t.funding_efficiency,
                "funding_decay" => &t.funding_decay,
                "usage_decay" => &t.usage_decay,
                "pole_decay" => &t.pole_decay,
                "strike_response" => &t.strike_response,
                _ => return Err(format!("unknown curve '{name}'")),
            };
            jf(c.eval(x))
        }
        _ => return Err(format!("unknown method '{method}'")),
    };
    check_expect(op, got)
}

// ---- state --------------------------------------------------------------------------------------

fn bitmap<'a>(s: &'a PowerState, name: &str) -> R<&'a Bitmap> {
    Ok(match name {
        "mark" => &s.mark,
        "wire" => &s.wire,
        "powered" | "current" => &s.powered_current,
        "next" => &s.powered_next,
        _ => return Err(format!("unknown bitmap '{name}'")),
    })
}

fn bitmap_mut<'a>(s: &'a mut PowerState, name: &str) -> R<&'a mut Bitmap> {
    Ok(match name {
        "mark" => &mut s.mark,
        "wire" => &mut s.wire,
        "powered" | "current" => &mut s.powered_current,
        "next" => &mut s.powered_next,
        _ => return Err(format!("unknown bitmap '{name}'")),
    })
}

fn i32s(j: &Json, what: &str) -> R<Vec<i32>> {
    arr(j, what)?.iter().map(|v| i32_of(v, what)).collect()
}

fn set(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let mut err = Ok(());
    s.update_state(|st| {
        err = (|| -> R<()> {
            for name in ["mark", "wire", "powered", "next"] {
                if let Some(v) = opt(op, name) {
                    let b = bitmap_mut(st, name)?;
                    for c in arr(v, name)? {
                        let a = i32s(c, name)?;
                        match a.len() {
                            2 => b.set(a[0], a[1]),
                            4 => b.rect_put(CellRect::new(a[0], a[1], a[2], a[3]), true),
                            _ => return Err(format!("'{name}' entries are [x, z] or [x1, z1, x2, z2]")),
                        }
                    }
                }
            }
            if let Some(v) = opt(op, "consumption") {
                for c in arr(v, "consumption")? {
                    let a = i32s(c, "consumption")?;
                    if let Some(p) = st
                        .consumption_map
                        .get_mut(a[0] as usize)
                        .and_then(|r| r.get_mut(a[1] as usize))
                    {
                        *p = a[2] as i16;
                    }
                }
            }
            let Some(state) = opt(op, "state") else { return Ok(()) };
            for (k, v) in state.as_object().ok_or("'state' must be an object")? {
                let u = || u32_of(v, k);
                match k.as_str() {
                    "enabled" => st.enabled = bool_of(v, k)?,
                    "initialized" => st.initialized = bool_of(v, k)?,
                    "running" => st.running = bool_of(v, k)?,
                    "recompute" => st.recompute = bool_of(v, k)?,
                    "age_plants" => st.age_plants = bool_of(v, k)?,
                    "wire_dirty" => st.wire_dirty = bool_of(v, k)?,
                    "state" => st.state = u()?,
                    "ring_index" => st.ring_index = i32_of(v, k)?,
                    "production" => st.production = u()?,
                    "consumption" => st.consumption = u()?,
                    "import" => st.import = u()?,
                    "export" => st.export = u()?,
                    "demand" => st.demand = u()?,
                    "consumers" => st.consumers = u()?,
                    "powered" => st.powered = u()?,
                    "unpowered" => st.unpowered = u()?,
                    "requirement" => st.requirement = u()?,
                    "cost_base" => st.cost_base = i64_of(v, k)?,
                    "deficit" => st.deficit = f32_of(v, k)?,
                    "broken_fraction" => st.broken_fraction = f32_of(v, k)?,
                    "broken_count" => st.broken_count = u()?,
                    "pole_count" => st.pole_count = u()?,
                    "factor" => st.factor = f32_of(v, k)?,
                    "strike_start" => st.strike_start = i32_of(v, k)?,
                    "strike_flag" => st.strike_flag = u()? as u8,
                    "unused_flag" => st.unused_flag = u()? as u8,
                    "radius" => st.tuning.radius = u()?,
                    "annual" => {
                        for (d, x) in st.annual.iter_mut().zip(i32s(v, k)?) {
                            *d = x;
                        }
                    }
                    "ring_row" => {
                        let row = st.ring_row();
                        for (d, x) in st.ring[row].iter_mut().zip(i32s(v, k)?) {
                            *d = x;
                        }
                    }
                    "per_type" => {
                        for (d, x) in st.per_type.iter_mut().zip(i32s(v, k)?) {
                            *d = x;
                        }
                    }
                    _ => return Err(format!("unknown state field '{k}'")),
                }
            }
            Ok(())
        })();
    });
    err
}

fn state_json(w: &World, s: &PowerSim) -> Vec<(String, Json)> {
    let st = s.state();
    let i32_arr = |v: &[i32]| Json::Array(v.iter().map(|&x| ji(x)).collect());
    let tok = |t: u64| ji(t as i64);
    let has_grid = s.powered_grid().is_some();
    let _ = w;
    vec![
        ("initialized".into(), Json::Bool(st.initialized)),
        ("running".into(), Json::Bool(st.running)),
        ("enabled".into(), Json::Bool(st.enabled)),
        ("department".into(), st.department.map_or(Json::Null, tok)),
        ("ring_index".into(), ji(st.ring_index)),
        ("production".into(), ji(st.production)),
        ("consumption".into(), ji(st.consumption)),
        ("import".into(), ji(st.import)),
        ("export".into(), ji(st.export)),
        ("demand".into(), ji(st.demand)),
        ("last_production".into(), ji(st.last_production)),
        ("last_consumption".into(), ji(st.last_consumption)),
        ("last_import".into(), ji(st.last_import)),
        ("last_export".into(), ji(st.last_export)),
        ("last_demand".into(), ji(st.last_demand)),
        ("deficit".into(), jf(st.deficit)),
        ("consumers".into(), ji(st.consumers)),
        ("powered".into(), ji(st.powered)),
        ("unpowered".into(), ji(st.unpowered)),
        ("requirement".into(), ji(st.requirement)),
        ("cost_base".into(), ji(st.cost_base)),
        ("annual".into(), i32_arr(&st.annual)),
        ("ring_row".into(), i32_arr(&st.ring[st.ring_row()])),
        (
            "plants".into(),
            Json::Array(
                st.plants
                    .iter()
                    .map(|p| Json::Array(vec![ji(p.visited), tok(p.occupant)]))
                    .collect(),
            ),
        ),
        ("plant_count".into(), ji(st.plant_count)),
        ("per_type".into(), i32_arr(&st.per_type)),
        ("poles".into(), Json::Array(st.poles.iter().map(|&p| tok(p)).collect())),
        ("pole_count".into(), ji(st.pole_count)),
        ("broken_count".into(), ji(st.broken_count)),
        ("unused_flag".into(), ji(st.unused_flag)),
        ("strike_flag".into(), ji(st.strike_flag)),
        ("strike_start".into(), ji(st.strike_start)),
        ("recompute".into(), Json::Bool(st.recompute)),
        ("age_plants".into(), Json::Bool(st.age_plants)),
        ("state".into(), ji(st.state)),
        ("wire_dirty".into(), Json::Bool(st.wire_dirty)),
        ("dirty_rect".into(), rect_json(st.dirty_rect)),
        ("changed".into(), Json::Bool(st.changed)),
        ("overworked".into(), ji(st.overworked)),
        ("aged".into(), ji(st.aged)),
        ("distressed".into(), ji(st.distressed)),
        ("broken_fraction".into(), jf(st.broken_fraction)),
        ("factor".into(), jf(st.factor)),
        ("radius".into(), ji(st.tuning.radius)),
        ("max_funding".into(), jf(st.tuning.max_funding)),
        ("strike_efficiency".into(), jf(st.tuning.strike_efficiency)),
        ("max_production_ratio".into(), jf(st.tuning.max_production_ratio)),
        ("min_acceptable_funding".into(), jf(st.tuning.min_acceptable_funding)),
        ("strike_duration".into(), ji(st.tuning.strike_duration)),
        ("min_funding_end_strike".into(), jf(st.tuning.min_funding_end_strike)),
        (
            "map_size".into(),
            Json::Array(vec![
                ji(st.consumption_map.len() as i64),
                ji(st.consumption_map.first().map_or(0, Vec::len) as i64),
            ]),
        ),
        ("bitmap_words".into(), ji(st.mark.words().len() as i64)),
        ("has_grid".into(), Json::Bool(has_grid)),
    ]
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    check_state(&state_json(w, &s), field(op, "state")?)
}

/// `power_expect_bitmap`: `bitmap`, `cells` (set), `unset`, `rects` (all
/// set), `count` (set bits in the city), `exact` (set bits are exactly
/// `cells` ∪ `rects`).
fn expect_bitmap(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let st = s.state();
    let name = str_of(field(op, "bitmap")?, "bitmap")?;
    let b = bitmap(&st, name)?;
    let (cx, cz) = w.cells.get();
    let mut want = std::collections::BTreeSet::new();
    if let Some(v) = opt(op, "cells") {
        for c in arr(v, "cells")? {
            want.insert(pair(c, "cells")?);
        }
    }
    if let Some(v) = opt(op, "rects") {
        for r in arr(v, "rects")? {
            let r = rect_of(r, "rects")?;
            for x in r.x1..=r.x2 {
                for z in r.z1..=r.z2 {
                    want.insert((x, z));
                }
            }
        }
    }
    for &(x, z) in &want {
        if !b.test(x, z) {
            return Err(format!("{name} ({x}, {z}) is not set"));
        }
    }
    if let Some(v) = opt(op, "unset") {
        for c in arr(v, "unset")? {
            let (x, z) = pair(c, "unset")?;
            if b.test(x, z) {
                return Err(format!("{name} ({x}, {z}) is set"));
            }
        }
    }
    let mut set_cells = Vec::new();
    for z in 0..cz {
        for x in 0..cx {
            if b.test(x, z) {
                set_cells.push((x, z));
            }
        }
    }
    if let Some(v) = opt(op, "count") {
        if set_cells.len() as i64 != i64_of(v, "count")? {
            return Err(format!("{name}: {} bits set", set_cells.len()));
        }
    }
    if opt(op, "exact").map_or(Ok(false), |v| bool_of(v, "exact"))? {
        let extra: Vec<_> = set_cells.iter().filter(|c| !want.contains(c)).take(8).collect();
        if !extra.is_empty() {
            return Err(format!("{name}: unexpected bits {extra:?}"));
        }
    }
    Ok(())
}

/// `power_expect_grid`: `exists`, `instance`, `tract_size`, `counts`,
/// `cells` (`[[x, z, v]]`), `rects` (`[[x1, z1, x2, z2, v]]`), `others`
/// (every unlisted cell).
fn expect_grid(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let g = s.powered_grid();
    if let Some(v) = opt(op, "exists") {
        if g.is_some() != bool_of(v, "exists")? {
            return Err(format!("grid exists = {}", g.is_some()));
        }
    }
    let Some(g) = g else { return Ok(()) };
    let geo = g.geometry();
    if let Some(v) = opt(op, "instance") {
        let p = Rc::as_ptr(&g) as *const u8;
        let inst = w
            .grids
            .borrow()
            .iter()
            .find(|s| Rc::as_ptr(s) as *const u8 == p)
            .map(|s| s.instance_id());
        if inst != Some(u32_of(v, "instance")?) {
            return Err(format!("grid instance {inst:?}"));
        }
    }
    if let Some(v) = opt(op, "tract_size") {
        if i64::from(geo.tract_size) != i64_of(v, "tract_size")? {
            return Err(format!("tract size {}", geo.tract_size));
        }
    }
    if let Some(v) = opt(op, "counts") {
        if (geo.count_x, geo.count_z) != pair(v, "counts")? {
            return Err(format!("counts {} x {}", geo.count_x, geo.count_z));
        }
    }
    let mut listed = std::collections::BTreeSet::new();
    if let Some(v) = opt(op, "cells") {
        for c in arr(v, "cells")? {
            let a = i32s(c, "cells")?;
            listed.insert((a[0], a[1]));
            let got = g.get(a[0], a[1]);
            if i32::from(got) != a[2] {
                return Err(format!("grid ({}, {}) = {got}, expected {}", a[0], a[1], a[2]));
            }
        }
    }
    if let Some(v) = opt(op, "rects") {
        for c in arr(v, "rects")? {
            let a = i32s(c, "rects")?;
            if a.len() != 5 {
                return Err("'rects' entries are [x1, z1, x2, z2, value]".into());
            }
            for x in a[0]..=a[2] {
                for z in a[1]..=a[3] {
                    listed.insert((x, z));
                    let got = g.get(x, z);
                    if i32::from(got) != a[4] {
                        return Err(format!("grid ({x}, {z}) = {got}, expected {}", a[4]));
                    }
                }
            }
        }
    }
    if let Some(v) = opt(op, "others") {
        let want = i32_of(v, "others")?;
        for x in 0..geo.count_x {
            for z in 0..geo.count_z {
                if !listed.contains(&(x, z)) && i32::from(g.get(x, z)) != want {
                    return Err(format!("grid ({x}, {z}) = {}, expected {want}", g.get(x, z)));
                }
            }
        }
    }
    Ok(())
}

/// `power_expect_occupant`: `occupant`, `props` (`{"0xID": value or null}`),
/// `distressed?`.
fn expect_occupant(w: &World, op: &Json) -> R<()> {
    let id = u32_of(field(op, "occupant")?, "occupant")? as u64;
    let occs = w.occs.borrow();
    let o = occs.get(&id).ok_or("unknown occupant")?;
    if let Some(p) = opt(op, "props") {
        for (k, v) in p.as_object().ok_or("'props' must be an object")? {
            let pid = Json::Str(k.clone()).as_i64().ok_or("bad property id")? as u32;
            let got = o.props.get(pid);
            match (got, v) {
                (None, Json::Null) => {}
                (None, _) => return Err(format!("property {k} is absent")),
                (Some(g), want) => {
                    let gj = match g.items().first() {
                        Some(Elem::F32(f)) => jf(*f),
                        Some(e) => ji(e.as_i64()),
                        None => Json::Null,
                    };
                    if !scalar::matches(&gj, want, false) {
                        return Err(format!(
                            "property {k} = {}, expected {}",
                            scalar::show(&gj),
                            scalar::show(want)
                        ));
                    }
                }
            }
        }
    }
    if let Some(v) = opt(op, "distressed") {
        if o.distressed != bool_of(v, "distressed")? {
            return Err(format!("distressed = {}", o.distressed));
        }
    }
    Ok(())
}

/// `power_expect_world`: partial checks of host objects: `lots`
/// (`{"id": {has_power}}`), `deals` (`{"id": {used, flags}}`),
/// `connections` (`{"id": {visited, active}}`), `line_items`
/// (`[{department, line, expenses?, secondary?}]`), `distressed` (count of
/// distressed poles), `departments` (IDs, ascending).
fn expect_world(w: &World, op: &Json) -> R<()> {
    let key = |k: &str| -> R<u64> { Ok(Json::Str(k.into()).as_i64().ok_or("bad id")? as u64) };
    if let Some(v) = opt(op, "lots") {
        for (k, want) in v.as_object().ok_or("lots: object")? {
            let lots = w.lots.borrow();
            let l = lots.get(&key(k)?).ok_or("unknown lot")?;
            check_state(&[("has_power".into(), Json::Bool(l.has_power))], want)?;
        }
    }
    if let Some(v) = opt(op, "deals") {
        for (k, want) in v.as_object().ok_or("deals: object")? {
            let id = key(k)?;
            let deals = w.deals.borrow();
            let d = deals.iter().find(|(t, _)| *t == id).ok_or("unknown deal")?.1;
            check_state(&[("used".into(), ji(d.used)), ("flags".into(), ji(d.flags))], want)?;
        }
    }
    if let Some(v) = opt(op, "connections") {
        for (k, want) in v.as_object().ok_or("connections: object")? {
            let c = w.conn(key(k)?, |c| *c).ok_or("unknown connection")?;
            check_state(
                &[
                    ("visited".into(), Json::Bool(c.visited)),
                    ("active".into(), Json::Bool(c.active)),
                ],
                want,
            )?;
        }
    }
    if let Some(v) = opt(op, "line_items") {
        for e in arr(v, "line_items")? {
            let dept = u32_of(field(e, "department")?, "department")?;
            let line = u32_of(field(e, "line")?, "line")?;
            let item = w.depts.borrow().get(&dept).and_then(|d| d.lines.get(&line).copied());
            let exists = item.is_some();
            if let Some(x) = opt(e, "exists") {
                if exists != bool_of(x, "exists")? {
                    return Err(format!("line item {line} exists = {exists}"));
                }
            }
            if let Some(item) = item {
                let li = w.line_items.borrow().get(&item).copied().unwrap_or_default();
                let mut got = vec![
                    ("expenses".into(), ji(li.expenses)),
                    ("secondary".into(), ji(li.secondary)),
                ];
                got.push(("department".into(), ji(dept)));
                got.push(("line".into(), ji(line)));
                let mut want = e.clone();
                if let Json::Object(kv) = &mut want {
                    kv.retain(|(k, _)| k != "exists");
                }
                check_state(&got, &want)?;
            }
        }
    }
    if let Some(v) = opt(op, "distressed") {
        let n = w.occs.borrow().values().filter(|o| o.distressed).count() as i64;
        if n != i64_of(v, "distressed")? {
            return Err(format!("{n} distressed poles"));
        }
    }
    if let Some(v) = opt(op, "departments") {
        let got: Vec<Json> = w.depts.borrow().keys().map(|&k| hx(k)).collect();
        if !scalar::matches(&Json::Array(got.clone()), v, false) {
            return Err(format!("departments {}", scalar::show(&Json::Array(got))));
        }
    }
    Ok(())
}

/// `power_expect_rand`: `calls` (the simulator's `rand()` calls since the
/// last check).
fn expect_rand(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let want = i64_of(field(op, "calls")?, "calls")?;
    let mut probe = w.crt_mark.get();
    let now = s.crt();
    let mut n = 0i64;
    while probe != now && n <= 1_000_000 {
        probe.rand();
        n += 1;
    }
    w.crt_mark.set(now);
    if n != want {
        return Err(format!("{n} rand() calls, expected {want}"));
    }
    Ok(())
}

/// `power_expect_refs`: `refs` (`{"token": count}`; counts start at 0).
fn expect_refs(w: &World, op: &Json) -> R<()> {
    let refs = w.refs.borrow();
    for (k, v) in field(op, "refs")?.as_object().ok_or("'refs' must be an object")? {
        let t = Json::Str(k.clone()).as_i64().ok_or("bad token")? as u64;
        let got = refs.get(&t).copied().unwrap_or(0);
        if got != i64_of(v, "refs")? {
            return Err(format!("{k}: {got} references"));
        }
    }
    Ok(())
}

// ---- persistence ------------------------------------------------------------------------------------

fn slot(op: &Json) -> R<String> {
    Ok(str_of(field(op, "slot")?, "slot")?.to_owned())
}

fn save_now(w: &World) -> R<SavedCity> {
    let s = w.sim()?;
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut root = ByteWriter::with_registry(reg.clone());
    root.put_ref(&RefValue::Object(s as Rc<dyn SaveObject>));
    let mut pkg = Package::new();
    let rep = save_store(&reg, &mut pkg);
    if rep.aborted || rep.entries_discarded != 0 {
        return Err(format!("save failed: {rep:?}"));
    }
    let hosts = reg
        .borrow()
        .entries
        .iter()
        .filter(|e| e.2.is_none())
        .map(|e| (e.1, e.3))
        .collect();
    Ok(SavedCity {
        pkg,
        root: root.into_bytes(),
        hosts,
    })
}

fn save(w: &World, op: &Json) -> R<()> {
    let s = save_now(w)?;
    w.saves.borrow_mut().insert(slot(op)?, s);
    Ok(())
}

/// `power_load`: `slot`, `expect?` (clean store load), `crt_seed?`.
fn load(w: &World, op: &Json) -> R<()> {
    let saved = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    let me = w.rc();
    let mut factory = |class: u32, object_id: u32| -> Option<RefValue> {
        match class {
            CLSID_POWER_SIMULATOR => Some(RefValue::Object(
                PowerSim::new(me.clone() as Rc<dyn PowerHost>, me.token()) as Rc<dyn SaveObject>,
            )),
            OCCUPANT_CLASS | DEPARTMENT_CLASS => {
                // The same game object comes back with a reference for the reader.
                let token = *saved.hosts.get(&object_id)?;
                me.add_ref(token);
                Some(RefValue::Host { token, class_id: class })
            }
            c => {
                let o = create_for_clsid(c)?;
                if let Ok(g) = o.clone().as_any_rc().downcast::<SimGrid<u8>>() {
                    me.grids.borrow_mut().push(g);
                }
                Some(RefValue::Object(o))
            }
        }
    };
    let mut reg = LoadRegistry::new(BTreeMap::new(), Box::new(&mut factory));
    let report = load_store(&saved.pkg, &mut reg);
    let ok = report.index_ok && report.failed_classes.is_empty();
    let mut r = ByteReader::with_resolver(&saved.root, &mut reg);
    let root = r.get_ref();
    let want = opt(op, "expect").map_or(Ok(true), |v| bool_of(v, "expect"))?;
    if ok != want {
        return Err(format!("store load clean = {ok}, expected {want}"));
    }
    let sim = root
        .as_ref()
        .and_then(|v| v.object::<PowerSim>())
        .ok_or("the root did not resolve to a power simulator")?;
    if let Some(seed) = opt(op, "crt_seed") {
        sim.set_crt(CrtRand::new(u32_of(seed, "crt_seed")?));
    }
    w.crt_mark.set(sim.crt());
    *w.sim.borrow_mut() = Some(sim);
    Ok(())
}

fn expect_save(w: &World, op: &Json) -> R<()> {
    let want = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    let got = save_now(w)?;
    scalar::same_save(
        &scalar::Saved {
            pkg: got.pkg,
            root: got.root,
        },
        &scalar::Saved {
            pkg: want.pkg,
            root: want.root,
        },
    )
}

fn patch(w: &World, op: &Json) -> R<()> {
    let mut saves = w.saves.borrow_mut();
    let s = saves.get_mut(&slot(op)?).ok_or("no such slot")?;
    let class = opt(op, "class").map_or(Ok(CLSID_POWER_SIMULATOR), |c| u32_of(c, "class"))?;
    let mut tmp = scalar::Saved {
        pkg: std::mem::take(&mut s.pkg),
        root: Vec::new(),
    };
    let r = scalar::patch(&mut tmp, class, op);
    s.pkg = tmp.pkg;
    r
}

/// `power_read`: calls the current simulator's `Read` directly on a record
/// payload (`slot`: the saved entry's payload; or `hex`), without a
/// reference resolver; `expect` is its result.
fn read_record(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let payload = match opt(op, "hex") {
        Some(h) => hex_decode(str_of(h, "hex")?)?,
        None => {
            let saves = w.saves.borrow();
            let saved = saves.get(&slot(op)?).ok_or("no such slot")?;
            let tmp = scalar::Saved {
                pkg: saved.pkg.clone(),
                root: Vec::new(),
            };
            scalar::entry_payload(&tmp, CLSID_POWER_SIMULATOR, 0)?
        }
    };
    let mut r = ByteReader::new(&payload);
    check_expect(op, Json::Bool(SaveObject::read(&*s, &mut r)))
}

/// `power_expect_payload`: `slot`, `hex?`, `at?` (`[{offset, hex}]`),
/// `length?`.
fn expect_payload(w: &World, op: &Json) -> R<()> {
    let saves = w.saves.borrow();
    let s = saves.get(&slot(op)?).ok_or("no such slot")?;
    let tmp = scalar::Saved {
        pkg: s.pkg.clone(),
        root: Vec::new(),
    };
    let got = scalar::entry_payload(&tmp, CLSID_POWER_SIMULATOR, 0)?;
    if let Some(h) = opt(op, "hex") {
        let want = hex_decode(str_of(h, "hex")?)?;
        if got != want {
            return Err(format!("payload differs:\n    got      {}", hex_encode(&got)));
        }
    }
    if let Some(v) = opt(op, "at") {
        for e in arr(v, "at")? {
            let off = i64_of(field(e, "offset")?, "offset")? as usize;
            let want = hex_decode(str_of(field(e, "hex")?, "hex")?)?;
            let slice = got.get(off..off + want.len()).ok_or("slice outside the payload")?;
            if slice != want.as_slice() {
                return Err(format!(
                    "payload at {off}: got {}, expected {}",
                    hex_encode(slice),
                    hex_encode(&want)
                ));
            }
        }
    }
    if let Some(v) = opt(op, "length") {
        if got.len() as i64 != i64_of(v, "length")? {
            return Err(format!("payload length {}", got.len()));
        }
    }
    Ok(())
}
