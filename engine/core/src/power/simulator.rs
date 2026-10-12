// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cISC4PowerSimulator`: state (§3), lifecycle (§4), messages and occupant
//! changes (§5) and the interface methods of §10.1. The recompute cycle
//! (§6) and plants and poles (§7) are in `cycle.rs`, the record (§9) in
//! `record.rs`.

use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

use crate::curve::ResponseCurve;
use crate::grid::{Bitmap, CellRect, TractGrid, MAX_BITMAP_SIDE};
use crate::rng::CrtRand;
use crate::variant::{Elem, Variant};

use super::tuning::curve_of;
use super::{
    budget, deal, ftol32, history, msg, network_flag, occupant_type, prop, state, PowerHost, PowerMessage, PowerTuning,
    AGENT_FLAGS, AGENT_LEVELS, AGENT_NAME, GRID_INSTANCE, GRID_TRACT_SIZE, NFZ_DEMOLISH_FLAGS, NUCLEAR_TOOL_CONTROL,
    ORDINANCE_FACTOR_ID, PLANT_TYPES, PLANT_TYPE_NUCLEAR, POWERED, UNWIRED, WIRED,
};

pub(crate) type G = Rc<dyn TractGrid<u8>>;

/// One node of the plant list (§3, §5.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlantNode {
    /// 0 = not reached this cycle, 1 = supplied a network, 2 = done (§6).
    pub visited: u32,
    pub occupant: u64,
}

/// The plain fields of §3 (the powered grid and the `rand()` generator
/// are held separately).
#[derive(Clone, Debug, PartialEq)]
pub struct PowerState {
    pub initialized: bool,
    pub running: bool,
    pub enabled: bool,
    pub department: Option<u64>,
    pub ring_index: i32,
    pub production: u32,
    pub consumption: u32,
    pub import: u32,
    pub export: u32,
    pub demand: u32,
    pub last_production: u32,
    pub last_consumption: u32,
    pub last_import: u32,
    pub last_export: u32,
    pub last_demand: u32,
    pub deficit: f32,
    pub consumers: u32,
    pub powered: u32,
    pub unpowered: u32,
    pub requirement: u32,
    pub cost_base: i64,
    /// `[x][z]`.
    pub consumption_map: Vec<Vec<i16>>,
    pub ring: [[i32; PLANT_TYPES]; 12],
    pub annual: [i32; PLANT_TYPES],
    pub plants: Vec<PlantNode>,
    pub plant_count: i32,
    pub per_type: [i32; PLANT_TYPES],
    pub poles: Vec<u64>,
    pub pole_count: u32,
    pub broken_count: u32,
    pub mark: Bitmap,
    pub wire: Bitmap,
    pub powered_current: Bitmap,
    pub powered_next: Bitmap,
    pub unused_flag: u8,
    pub strike_flag: u8,
    pub strike_start: i32,
    pub recompute: bool,
    pub age_plants: bool,
    pub state: u32,
    pub wire_dirty: bool,
    pub dirty_rect: CellRect,
    pub changed: bool,
    pub overworked: u32,
    pub aged: u32,
    pub distressed: u32,
    pub broken_fraction: f32,
    pub factor: f32,
    pub tuning: PowerTuning,
}

impl Default for PowerState {
    /// The constructor values (§3).
    fn default() -> Self {
        PowerState {
            initialized: false,
            running: false,
            enabled: false,
            department: None,
            ring_index: 0,
            production: 0,
            consumption: 0,
            import: 0,
            export: 0,
            demand: 0,
            last_production: 0,
            last_consumption: 0,
            last_import: 0,
            last_export: 0,
            last_demand: 0,
            deficit: 0.0,
            consumers: 0,
            powered: 0,
            unpowered: 0,
            requirement: 0,
            cost_base: 0,
            consumption_map: Vec::new(),
            ring: [[0; PLANT_TYPES]; 12],
            annual: [0; PLANT_TYPES],
            plants: Vec::new(),
            plant_count: 0,
            per_type: [0; PLANT_TYPES],
            poles: Vec::new(),
            pole_count: 0,
            broken_count: 0,
            mark: Bitmap::default(),
            wire: Bitmap::default(),
            powered_current: Bitmap::default(),
            powered_next: Bitmap::default(),
            unused_flag: 0,
            strike_flag: 0,
            strike_start: 0,
            recompute: false,
            age_plants: false,
            state: 0,
            wire_dirty: false,
            dirty_rect: CellRect::default(),
            changed: false,
            overworked: 0,
            aged: 0,
            distressed: 0,
            broken_fraction: 0.0,
            factor: 1.0,
            tuning: PowerTuning::default(),
        }
    }
}

impl PowerState {
    /// The ring row of `ringIndex` (used modulo 12, [choice]).
    pub fn ring_row(&self) -> usize {
        self.ring_index.rem_euclid(12) as usize
    }

    /// `consumption[x][z]`, 0 outside the map.
    pub fn consumption_at(&self, x: i32, z: i32) -> i16 {
        if x < 0 || z < 0 {
            return 0;
        }
        self.consumption_map
            .get(x as usize)
            .and_then(|r| r.get(z as usize))
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn consumption_mut(&mut self, x: i32, z: i32) -> Option<&mut i16> {
        if x < 0 || z < 0 {
            return None;
        }
        self.consumption_map.get_mut(x as usize)?.get_mut(z as usize)
    }
}

/// What `FindPowerPlants` (slot 30) reports.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FoundPlants {
    /// The lowest-funded plant and its local funding.
    pub lowest_funded: Option<(u64, f32)>,
    /// The most worn plant and `trunc(wear / max(t1, t2) × 100)`.
    pub most_worn: Option<(u64, i32)>,
}

#[derive(Default)]
pub(crate) struct Inner {
    pub(crate) s: PowerState,
    pub(crate) grid: Option<G>,
    pub(crate) crt: CrtRand,
}

/// The working context of one call: the host and the state taken out of
/// the simulator for the call.
pub(crate) struct Cx<'a> {
    pub(crate) h: &'a dyn PowerHost,
    pub(crate) i: &'a mut Inner,
}

/// The power simulator.
pub struct PowerSim {
    pub(crate) host: Rc<dyn PowerHost>,
    token: Cell<u64>,
    pub(crate) inner: RefCell<Inner>,
}

// ---- property helpers -----------------------------------------------------------------------------

fn first(v: &Variant) -> Option<Elem> {
    match v {
        Variant::Scalar(e) => Some(*e),
        Variant::Array { elem, items } if *elem != crate::variant::mem::RZ_CHAR => items.first().copied(),
        _ => None,
    }
}

fn elem_f32(e: Elem) -> f32 {
    match e {
        Elem::F32(x) => x,
        other => other.as_f64() as f32,
    }
}

impl Cx<'_> {
    pub(crate) fn s(&mut self) -> &mut PowerState {
        &mut self.i.s
    }

    pub(crate) fn prop(&self, occ: u64, id: u32) -> Option<Elem> {
        if occ == 0 {
            return None;
        }
        first(&self.h.occupant_property(occ, id)?)
    }

    pub(crate) fn prop_u32(&self, occ: u64, id: u32) -> Option<u32> {
        self.prop(occ, id).map(|e| e.as_i64() as u32)
    }

    pub(crate) fn prop_i32(&self, occ: u64, id: u32) -> Option<i32> {
        self.prop(occ, id).map(|e| e.as_i64() as i32)
    }

    pub(crate) fn prop_i64(&self, occ: u64, id: u32) -> Option<i64> {
        self.prop(occ, id).map(|e| e.as_i64())
    }

    pub(crate) fn prop_f32(&self, occ: u64, id: u32) -> Option<f32> {
        self.prop(occ, id).map(elem_f32)
    }

    /// A per-building curve, fresh per call ("delete" pattern, §8).
    pub(crate) fn prop_curve(&self, occ: u64, id: u32) -> Option<ResponseCurve> {
        if occ == 0 {
            return None;
        }
        curve_of(self.h.occupant_property(occ, id).as_ref())
    }

    // ---- geometry -----------------------------------------------------------------------------------

    /// The city's cell counts, clamped to `0 ..= MAX_BITMAP_SIDE`.
    pub(crate) fn cells(&self) -> (i32, i32) {
        let (x, z) = self.h.cell_counts();
        (x.clamp(0, MAX_BITMAP_SIDE), z.clamp(0, MAX_BITMAP_SIDE))
    }

    /// Expand (§2.2) by `r`.
    pub(crate) fn expand(&self, rect: CellRect, r: u32) -> CellRect {
        let (cx, cz) = self.cells();
        let lo = |v: i32| -> i32 {
            if (v as u32) > r {
                (v as u32).wrapping_sub(r) as i32
            } else {
                0
            }
        };
        let hi = |v: i32, n: i32| -> i32 { (i64::from(v) + i64::from(r)).min(i64::from(n) - 1) as i32 };
        CellRect {
            x1: lo(rect.x1),
            z1: lo(rect.z1),
            x2: hi(rect.x2, cx),
            z2: hi(rect.z2, cz),
        }
    }

    /// The neighbourhood of a cell (§2.2).
    pub(crate) fn neighbourhood(&self, x: i32, z: i32) -> CellRect {
        let r = self.i.s.tuning.radius;
        self.expand(CellRect::new(x, z, x, z), r)
    }

    pub(crate) fn wire_rect(&mut self, r: CellRect) {
        self.i.s.wire.rect_put(r, true);
    }

    /// Slot 5 `PositionToCell`: `trunc(x × (1 / CellWidthX))`.
    pub(crate) fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let (wx, wz) = self.h.cell_widths();
        (ftol32(x * (1.0f32 / wx)), ftol32(z * (1.0f32 / wz)))
    }

    /// A building's centre cell (§5.1).
    pub(crate) fn centre_cell(&self, occ: u64) -> (i32, i32) {
        let (min, max) = self.h.occupant_bounding_box(occ);
        let (wx, wz) = self.h.cell_widths();
        let axis = |a: f32, b: f32, w: f32| ftol32(((a + b) * 0.5f32) / w);
        (axis(min[0], max[0], wx), axis(min[2], max[2], wz))
    }

    /// The bounding cells of a building expanded by R (§5.1).
    pub(crate) fn building_rect(&self, occ: u64) -> CellRect {
        let r = self.i.s.tuning.radius;
        self.expand(self.h.occupant_bounding_cells(occ), r)
    }

    pub(crate) fn is_powered(&self, x: i32, z: i32) -> bool {
        self.i.s.powered_current.test(x, z)
    }

    /// The powered grid value of a cell (§2).
    pub(crate) fn grid_value(&self, x: i32, z: i32) -> u8 {
        if !self.i.s.wire.test(x, z) {
            UNWIRED
        } else if self.is_powered(x, z) {
            POWERED
        } else {
            WIRED
        }
    }

    pub(crate) fn write_grid_cell(&self, x: i32, z: i32) {
        if let Some(g) = &self.i.grid {
            g.set(x, z, self.grid_value(x, z));
        }
    }

    pub(crate) fn plant_type(&self, occ: u64) -> i32 {
        self.prop_i32(occ, prop::PLANT_TYPE).unwrap_or(0)
    }

    pub(crate) fn type_index(t: i32) -> Option<usize> {
        usize::try_from(t).ok().filter(|&t| t < PLANT_TYPES)
    }

    pub(crate) fn post(&self, msg_type: u32, data: [u32; 4], unknown: Option<u64>) {
        self.h.post(&super::Posted {
            msg_type,
            data,
            unknown,
        });
    }

    pub(crate) fn read_tuning(&mut self) {
        let t = self.h.tuning();
        self.i.s.tuning.read(t.as_deref());
        self.i.s.factor = self.h.cumulative_factor(ORDINANCE_FACTOR_ID, 0);
    }

    // ---- lifecycle (§4) -----------------------------------------------------------------------------

    pub(crate) fn init(&mut self) -> bool {
        // The first Init after a successful Read.
        let after_load = self.i.s.running && !self.i.s.initialized;
        // Step 1: the department is looked up only while none is held (a
        // load has set it from the saved ref).
        self.h.acquire_services();
        if self.i.s.department.is_none() {
            if let Some(d) = self.h.department(budget::DEPARTMENT) {
                self.h.add_ref(d);
                self.i.s.department = Some(d);
            }
        }
        // Step 2.
        if !self.i.s.initialized {
            self.i.s.initialized = true;
            self.read_tuning();
            for level in AGENT_LEVELS {
                self.h.add_agent(level, AGENT_NAME, AGENT_FLAGS);
            }
            for id in history::PER_TYPE {
                self.h.set_history_queue_length(id, history::PER_TYPE_QUEUE_LENGTH);
            }
            self.h.subscribe(&msg::SUBSCRIPTIONS);
            let (cx, cz) = self.cells();
            if self.i.s.consumption_map.len() != cx as usize {
                self.i.s.consumption_map = vec![vec![0; cz as usize]; cx as usize];
            }
        }
        // Step 3: a new city.
        if !self.i.s.running {
            let (cx, cz) = self.cells();
            let s = &mut self.i.s;
            s.running = true;
            s.unused_flag = 0;
            s.recompute = false;
            s.age_plants = false;
            s.wire_dirty = false;
            for b in [&mut s.mark, &mut s.wire, &mut s.powered_current, &mut s.powered_next] {
                *b = Bitmap::new(cx, cz);
            }
            if self.i.grid.is_none() {
                self.i.grid = self.h.create_grid(GRID_INSTANCE, GRID_TRACT_SIZE);
                if let Some(g) = &self.i.grid {
                    g.fill(0);
                }
            }
            let s = &mut self.i.s;
            s.ring = [[0; PLANT_TYPES]; 12];
            s.annual = [0; PLANT_TYPES];
            s.state = state::IDLE;
            if self.i.s.department.is_none() {
                self.create_department();
            }
        } else if after_load && self.i.s.department.is_none() {
            // [choice] A loaded city that still has no department gets one
            // as in step 3 (the original keeps null and crashes in state 1).
            self.create_department();
        }
        true
    }

    /// §4.1 step 3: create, name and set up the power department.
    fn create_department(&mut self) {
        let Some(d) = self.h.create_department(budget::DEPARTMENT, budget::DEPARTMENT_GROUP) else {
            return;
        };
        self.h.add_ref(d);
        self.i.s.department = Some(d);
        self.h
            .set_department_name(d, budget::NAME_GROUP, budget::DEPARTMENT_NAME);
        self.h.set_department_max_funding(d, self.i.s.tuning.max_funding);
        if let Some(li) = self.h.create_line_item(d, budget::LINE_POLES, false) {
            self.h
                .set_line_item_name(li, budget::NAME_GROUP, budget::LINE_POLES_NAME);
        }
    }

    pub(crate) fn shutdown(&mut self) -> bool {
        if self.i.s.running {
            self.i.s.running = false;
            self.h.release_services();
            if let Some(d) = self.i.s.department.take() {
                self.h.release(d);
            }
            self.h.unsubscribe(&msg::SUBSCRIPTIONS);
            self.h.remove_agent();
        }
        if self.i.s.initialized {
            self.i.s.initialized = false;
            for p in std::mem::take(&mut self.i.s.plants) {
                self.h.release(p.occupant);
            }
            for pole in std::mem::take(&mut self.i.s.poles) {
                self.h.break_all_connections(pole, false);
                self.h.release(pole);
            }
            let s = &mut self.i.s;
            s.pole_count = 0;
            s.broken_count = 0;
            for b in [&mut s.mark, &mut s.wire, &mut s.powered_current, &mut s.powered_next] {
                *b = Bitmap::default();
            }
            // Dropping the handle is the grid's Shutdown + Release.
            self.i.grid = None;
            self.i.s.consumption_map.clear();
        }
        true
    }

    // ---- messages (§5) ------------------------------------------------------------------------------

    pub(crate) fn do_message(&mut self, m: &PowerMessage) -> bool {
        match m.msg_type {
            msg::SIM_NEW_MONTH => self.monthly(),
            msg::SIM_NEW_DAY | msg::SIM_IDLE => self.do_idle(),
            msg::OCCUPANT_INSERTED => {
                if self.i.s.initialized && m.void1 != 0 {
                    self.occupant_inserted(m.void1);
                }
            }
            msg::OCCUPANT_REMOVED => {
                if m.void1 != 0 {
                    self.occupant_removed(m.void1);
                }
            }
            msg::ZONING_CHANGED => {
                self.cell_changed(m.rect, true);
                self.h.power_line_tool_zone_change(m.rect, m.data3);
            }
            msg::TUNING_RELOAD => self.read_tuning(),
            msg::TERRAIN_CHANGED => {
                if !msg::TERRAIN_IGNORED.contains(&m.data4) {
                    let (cx, cz) = self.cells();
                    let r = CellRect {
                        x1: ((m.data1 >> 16) as i32 - 1).clamp(0, (cx - 1).max(0)),
                        x2: ((m.data1 & 0xFFFF) as i32).clamp(0, (cx - 1).max(0)),
                        z1: ((m.data2 >> 16) as i32 - 1).clamp(0, (cz - 1).max(0)),
                        z2: ((m.data2 & 0xFFFF) as i32).clamp(0, (cz - 1).max(0)),
                    };
                    self.terrain_heights_changed(r);
                }
            }
            msg::ORDINANCE_UPDATED => {
                if m.data2 == msg::NUCLEAR_FREE_ZONE && m.data3 != 0 {
                    self.nuclear_free_zone();
                }
            }
            msg::STATE_CHANGED => self.lot_state_changed(m.void1),
            msg::DEAL_STATUS => {
                if m.data1 == 0 && m.void1 != 0 {
                    if let Some(t) = self.h.deal_type(m.void1) {
                        if t == deal::BUY_POWER || t == deal::SELL_POWER {
                            self.i.s.state = state::IDLE;
                            self.i.s.recompute = true;
                        }
                    }
                }
            }
            msg::CONNECTIONS_READY => self.i.s.enabled = true,
            msg::POST_CITY_INIT => self.post_city_init(),
            _ => {}
        }
        true
    }

    /// `kMessageStateChanged` (§5).
    fn lot_state_changed(&mut self, token: u64) {
        if token == 0 {
            return;
        }
        let Some(lot) = self.h.as_lot(token) else { return };
        if self.h.lot_state(lot) != 1 {
            return;
        }
        let r = self.h.lot_bounding_rect(lot);
        let cx = ((i64::from(r.x1) + i64::from(r.x2)) / 2) as i32;
        let cz = ((i64::from(r.z1) + i64::from(r.z2)) / 2) as i32;
        if self.is_powered(cx, cz) {
            if let Some(l) = self.h.lot_at(cx, cz) {
                self.h.set_lot_has_power(l, true);
            }
        }
    }

    /// `DoOccupantInserted` (§5.1).
    pub(crate) fn occupant_inserted(&mut self, occ: u64) {
        match self.h.occupant_type(occ) {
            occupant_type::POWER_POLE => {
                let Some(pole) = self.h.power_pole(occ) else { return };
                if pole.ignored {
                    return;
                }
                let r = self.neighbourhood(pole.cell.0, pole.cell.1);
                self.wire_rect(r);
                self.h.add_ref(occ);
                let cost = self.prop_i64(occ, prop::POLE_COST).unwrap_or(0);
                let s = self.s();
                s.poles.push(occ);
                s.pole_count = s.pole_count.wrapping_add(1);
                s.cost_base = s.cost_base.wrapping_add(cost);
                s.recompute = true;
            }
            occupant_type::POWER_LINE => {
                let Some(line) = self.h.power_line(occ) else { return };
                if line.preview {
                    return;
                }
                let r = self.neighbourhood(line.cell.0, line.cell.1);
                self.wire_rect(r);
                self.s().recompute = true;
            }
            occupant_type::NETWORK => {
                let Some((x, z)) = self.h.network_occupant_cell(occ) else {
                    return;
                };
                if self.h.has_network_flag(occ, network_flag::SKIP) {
                    return;
                }
                if self.h.has_network_flag(occ, network_flag::CONDUCTS) {
                    self.wire_rect(CellRect::new(x, z, x, z));
                    self.s().recompute = true;
                }
            }
            occupant_type::BUILDING => self.building_inserted(occ),
            _ => {}
        }
    }

    fn building_inserted(&mut self, occ: u64) {
        let (cx, cz) = self.centre_cell(occ);
        let r = self.building_rect(occ);
        self.wire_rect(r);
        let consumed = self.prop_u32(occ, prop::REQUIREMENT).unwrap_or(0);
        let produced = self.prop_u32(occ, prop::CAPACITY).unwrap_or(0);
        if consumed > produced {
            let s = self.s();
            if let Some(c) = s.consumption_mut(cx, cz) {
                *c = c.wrapping_add(consumed as i16);
            }
            s.consumers = s.consumers.wrapping_add(1);
            s.requirement = s.requirement.wrapping_add(consumed);
            if self.is_powered(cx, cz) {
                if let Some(lot) = self.h.occupant_lot(occ) {
                    self.h.set_lot_has_power(lot, true);
                }
            }
        } else if produced > consumed {
            self.h.add_ref(occ);
            let t = self.plant_type(occ);
            let s = self.s();
            s.plants.push(super::PlantNode {
                visited: 0,
                occupant: occ,
            });
            s.plant_count = s.plant_count.wrapping_add(1);
            if let Some(i) = Cx::type_index(t) {
                s.per_type[i] = s.per_type[i].wrapping_add(1);
            }
            s.recompute = true;
            self.plant_budget_hookup(occ);
        }
    }

    /// The budget hookup of a new plant (§5.1): every department call goes
    /// to the department the item names.
    fn plant_budget_hookup(&mut self, occ: u64) {
        let Some(item) = self.h.budget_item_for_purpose(occ, budget::PURPOSE) else {
            return;
        };
        let Some(building_type) = self.h.building_type(occ) else {
            return;
        };
        let dept = self.h.department(item.department);
        let mut line = item.line;
        if line == 0 {
            line = building_type;
            self.h.change_budget_item_line(occ, item.index, line);
        }
        if line == 0 {
            return;
        }
        // [choice] The original does not test the department for null.
        let Some(dept) = dept else { return };
        let li = match self.h.line_item(dept, line) {
            Some(li) => Some(li),
            None => {
                let li = self.h.create_line_item_for_building_type(dept, line, true);
                if let Some(li) = li {
                    self.h.set_line_item_display_flag(li, budget::DISPLAY_FLAG, true);
                }
                li
            }
        };
        self.h.add_locally_funded_object(dept, occ, budget::PURPOSE);
        if let Some(li) = li {
            let v = self.h.line_item_secondary_info(li);
            self.h.set_line_item_secondary_info(li, v.wrapping_add(1));
        }
        self.h
            .set_occupant_property(occ, prop::BASE_MAINTENANCE, Variant::Scalar(Elem::I64(item.cost)));
    }

    /// `DoOccupantRemoved` (§5.2).
    pub(crate) fn occupant_removed(&mut self, occ: u64) {
        match self.h.occupant_type(occ) {
            occupant_type::POWER_POLE => {
                let Some(pole) = self.h.power_pole(occ) else { return };
                if pole.ignored {
                    return;
                }
                let r = self.neighbourhood(pole.cell.0, pole.cell.1);
                self.mark_dirty(r, false);
                if let Some(i) = self.i.s.poles.iter().position(|&p| p == occ) {
                    self.h.release(occ);
                    let s = self.s();
                    s.poles.remove(i);
                    s.pole_count = s.pole_count.saturating_sub(1);
                }
                // Whether or not the pole was listed; brokenCount is not
                // lowered for a distressed pole (§5.2).
                let cost = self.prop_i64(occ, prop::POLE_COST).unwrap_or(0);
                let s = self.s();
                s.cost_base = s.cost_base.wrapping_sub(cost);
                s.recompute = true;
            }
            occupant_type::POWER_LINE => {
                let Some(line) = self.h.power_line(occ) else { return };
                if line.preview {
                    return;
                }
                let r = self.neighbourhood(line.cell.0, line.cell.1);
                self.mark_dirty(r, false);
                self.s().recompute = true;
            }
            occupant_type::NETWORK => {
                let Some((x, z)) = self.h.network_occupant_cell(occ) else {
                    return;
                };
                if self.h.has_network_flag(occ, network_flag::SKIP) {
                    return;
                }
                if self.h.has_network_flag(occ, network_flag::CONDUCTS) {
                    self.mark_dirty(CellRect::new(x, z, x, z), false);
                    self.s().recompute = true;
                }
            }
            occupant_type::BUILDING => self.building_removed(occ),
            _ => {}
        }
    }

    fn building_removed(&mut self, occ: u64) {
        let consumed = self.prop_u32(occ, prop::REQUIREMENT).unwrap_or(0);
        let produced = self.prop_u32(occ, prop::CAPACITY).unwrap_or(0);
        if consumed > produced && consumed > 0 {
            let (cx, cz) = self.centre_cell(occ);
            let s = self.s();
            if let Some(c) = s.consumption_mut(cx, cz) {
                *c = c.wrapping_sub(consumed as i16);
            }
            // Signed clamps (§3): a result negative as int32 becomes 0.
            let clamp = |v: u32| (v as i32).max(0) as u32;
            s.consumers = clamp(s.consumers.wrapping_sub(1));
            s.requirement = clamp(s.requirement.wrapping_sub(consumed));
        }
        let r = self.building_rect(occ);
        self.mark_dirty(r, false);
        if let Some(item) = self.h.budget_item_for_purpose(occ, budget::PURPOSE) {
            if let Some(dept) = self.h.department(item.department) {
                if let Some(li) = self.h.line_item(dept, item.line) {
                    self.h.remove_locally_funded_object(dept, occ, budget::PURPOSE);
                    let v = self.h.line_item_secondary_info(li);
                    self.h.set_line_item_secondary_info(li, v.wrapping_sub(1));
                }
            }
        }
        if let Some(i) = self.i.s.plants.iter().position(|p| p.occupant == occ) {
            let t = self.plant_type(occ);
            self.h.release(occ);
            let s = self.s();
            s.plants.remove(i);
            if let Some(i) = Cx::type_index(t) {
                s.per_type[i] = s.per_type[i].wrapping_sub(1);
            }
            s.plant_count = s.plant_count.wrapping_sub(1);
            s.recompute = true;
        }
    }

    /// Merges an (already expanded) rectangle into the dirty rectangle and
    /// requests a rebuild (§5.3).
    pub(crate) fn mark_dirty(&mut self, r: CellRect, zone_test: bool) {
        if zone_test && self.h.zone_type(r.x1, r.z1) != 0 {
            self.wire_rect(r);
        }
        let s = self.s();
        s.dirty_rect = if s.wire_dirty {
            CellRect {
                x1: s.dirty_rect.x1.min(r.x1),
                z1: s.dirty_rect.z1.min(r.z1),
                x2: s.dirty_rect.x2.max(r.x2),
                z2: s.dirty_rect.z2.max(r.z2),
            }
        } else {
            r
        };
        s.wire_dirty = true;
        s.recompute = true;
    }

    /// `CellChanged(rect)` (§5.3): expand by R, then [`Cx::mark_dirty`]
    /// with the zone test.
    pub(crate) fn cell_changed(&mut self, rect: CellRect, zone_test: bool) {
        let r = self.expand(rect, self.i.s.tuning.radius);
        self.mark_dirty(r, zone_test);
    }

    /// `DoTerrainHeightsChanged` (§5.4): one occupant query, then three
    /// passes over its list.
    fn terrain_heights_changed(&mut self, r: CellRect) {
        let occupants = self.h.occupants_in_cells(r);
        // 1. Poles (QI) whose own cell lies in the rect; no slot-13 test.
        for &occ in &occupants {
            if let Some(p) = self.h.power_pole(occ) {
                let (x, z) = p.cell;
                if r.x1 <= x && x <= r.x2 && r.z1 <= z && z <= r.z2 {
                    self.h.break_all_connections(occ, true);
                    self.h.remove_occupant(occ);
                }
            }
        }
        // 2. Lines (QI) with two connected poles, distinct by pole pair.
        let mut lines: Vec<(u64, (u64, u64))> = Vec::new();
        for &occ in &occupants {
            if self.h.power_line(occ).is_none() {
                continue;
            }
            if let Some(pair) = self.h.line_connected_poles(occ) {
                if !lines.iter().any(|&(_, p)| p == pair) {
                    lines.push((occ, pair));
                }
            }
        }
        // 3. `first.RemoveConnection(second, 0)` with the poles asked again.
        for (line, _) in lines {
            if let Some((first, second)) = self.h.line_connected_poles(line) {
                self.h.remove_pole_connection(first, second);
            }
        }
    }

    /// Nuclear Free Zone (§5.5).
    fn nuclear_free_zone(&mut self) {
        let mut lots = Vec::new();
        let plants: Vec<u64> = self.i.s.plants.iter().map(|p| p.occupant).collect();
        for occ in plants {
            if self.plant_type(occ) == PLANT_TYPE_NUCLEAR {
                if let Some(lot) = self.h.occupant_lot(occ) {
                    lots.push(lot);
                }
            }
        }
        self.h.demolish_lots(&lots, NFZ_DEMOLISH_FLAGS);
        if self.h.current_view_input_control() == Some(NUCLEAR_TOOL_CONTROL) {
            self.h.remove_current_view_input_control();
        }
    }

    /// PostCityInit (§5.6).
    fn post_city_init(&mut self) {
        let poles = std::mem::take(&mut self.i.s.poles);
        let mut kept = Vec::with_capacity(poles.len());
        for pole in poles {
            if self.h.occupant_in_manager(pole) {
                kept.push(pole);
            } else {
                self.h.release(pole);
                self.i.s.pole_count = self.i.s.pole_count.saturating_sub(1);
            }
        }
        let mut cost = 0i64;
        for &pole in &kept {
            let v = self.prop_i64(pole, prop::POLE_COST).unwrap_or(0);
            cost = cost.wrapping_add(i64::from(v as u32));
        }
        let s = self.s();
        s.poles = kept;
        if cost != s.cost_base {
            s.cost_base = cost;
            s.recompute = true;
        }
    }
}

// ---- the simulator object -------------------------------------------------------------------------

impl PowerSim {
    /// A new simulator with constructor values (§3); its `rand()` starts at
    /// state 1 (foundations §12.6).
    pub fn new(host: Rc<dyn PowerHost>, token: u64) -> Rc<PowerSim> {
        Rc::new(PowerSim {
            host,
            token: Cell::new(token),
            inner: RefCell::new(Inner::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    /// Runs `f` on the state taken out of the simulator (see the module
    /// documentation on re-entrancy).
    pub(crate) fn run<R>(&self, f: impl FnOnce(&mut Cx<'_>) -> R) -> R {
        let mut inner = std::mem::take(&mut *self.inner.borrow_mut());
        let r = {
            let mut cx = Cx {
                h: &*self.host,
                i: &mut inner,
            };
            f(&mut cx)
        };
        *self.inner.borrow_mut() = inner;
        r
    }

    /// The plain state.
    pub fn state(&self) -> Ref<'_, PowerState> {
        Ref::map(self.inner.borrow(), |i| &i.s)
    }

    /// Replaces parts of the state (tests, hosts restoring state).
    pub fn update_state(&self, f: impl FnOnce(&mut PowerState)) {
        f(&mut self.inner.borrow_mut().s);
    }

    /// The subsystem's `rand()` generator.
    pub fn crt(&self) -> CrtRand {
        self.inner.borrow().crt
    }

    pub fn set_crt(&self, crt: CrtRand) {
        self.inner.borrow_mut().crt = crt;
    }

    // ---- lifecycle and messages ---------------------------------------------------------------------

    /// `Init()` (§4.1): always true.
    pub fn init(&self) -> bool {
        self.run(|cx| cx.init())
    }

    /// `Shutdown()` (§4.2): always true.
    pub fn shutdown(&self) -> bool {
        self.run(|cx| cx.shutdown())
    }

    /// `DoMessage` (§4.3, §5): always true.
    pub fn do_message(&self, m: &PowerMessage) -> bool {
        self.run(|cx| cx.do_message(m))
    }

    /// Re-reads the tuning and the ordinance factor (§8).
    pub fn read_tuning(&self) {
        self.run(|cx| cx.read_tuning());
    }

    /// `DoOccupantInserted` (§5.1), without the `initialized` test.
    pub fn occupant_inserted(&self, occ: u64) {
        self.run(|cx| cx.occupant_inserted(occ));
    }

    /// `DoOccupantRemoved` (§5.2).
    pub fn occupant_removed(&self, occ: u64) {
        self.run(|cx| cx.occupant_removed(occ));
    }

    /// `CellChanged(rect)` (§5.3).
    pub fn cell_changed(&self, rect: CellRect) {
        self.run(|cx| cx.cell_changed(rect, true));
    }

    /// The expanded rectangle (§2.2) with the current R.
    pub fn expand(&self, rect: CellRect) -> CellRect {
        self.run(|cx| {
            let r = cx.i.s.tuning.radius;
            cx.expand(rect, r)
        })
    }

    // ---- interface methods (§10.1) ------------------------------------------------------------------

    /// Slot 5 `PositionToCell`.
    pub fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        self.run(|cx| cx.position_to_cell(x, z))
    }

    /// Slot 6 `CellToPosition`: the cell corner.
    pub fn cell_to_position(&self, cx: i32, cz: i32) -> (f32, f32) {
        let (wx, wz) = self.host.cell_widths();
        (super::mul_i(f64::from(cx), wx), super::mul_i(f64::from(cz), wz))
    }

    /// Slots 7 / 8: identity copies.
    pub fn cell_to_standard_city_cell(&self, x: i32, z: i32) -> (i32, i32) {
        (x, z)
    }

    /// Slot 9: `Σ annual`.
    pub fn annual_consumption(&self) -> u32 {
        self.state().annual.iter().fold(0u32, |a, &v| a.wrapping_add(v as u32))
    }

    /// Slot 10 (0 for a type outside 0..9, [choice]).
    pub fn annual_consumption_by_type(&self, t: i32) -> i32 {
        Cx::type_index(t).map_or(0, |i| self.state().annual[i])
    }

    /// Slot 11.
    pub fn monthly_requirement(&self) -> u32 {
        self.state().requirement
    }

    /// Slot 12.
    pub fn monthly_production(&self) -> u32 {
        self.state().last_production
    }

    /// Slot 13.
    pub fn monthly_import(&self) -> u32 {
        self.state().last_import
    }

    /// Slot 14.
    pub fn monthly_export(&self) -> u32 {
        self.state().last_export
    }

    /// Slot 15.
    pub fn monthly_demand(&self) -> u32 {
        self.state().last_demand
    }

    /// Slot 16 `GetMonthlyConsumption(x, z)`: sign-extended.
    pub fn monthly_consumption_at(&self, x: i32, z: i32) -> i32 {
        i32::from(self.state().consumption_at(x, z))
    }

    /// Slot 17 `GetMonthlyConsumption()`.
    pub fn monthly_consumption(&self) -> u32 {
        self.state().last_consumption
    }

    /// Slots 19 / 37 `GetSupplyValue` / `IsPowered`.
    pub fn is_powered(&self, x: i32, z: i32) -> bool {
        self.state().powered_current.test(x, z)
    }

    /// Slot 38 `IsWired`.
    pub fn is_wired(&self, x: i32, z: i32) -> bool {
        self.state().wire.test(x, z)
    }

    /// Slots 20 / 35 / 36: the powered grid (no reference added).
    pub fn powered_grid(&self) -> Option<G> {
        self.inner.borrow().grid.clone()
    }

    /// Slot 21.
    pub fn unpowered_consumer_count(&self) -> u32 {
        self.state().unpowered
    }

    /// Slot 22.
    pub fn powered_consumer_count(&self) -> u32 {
        self.state().powered
    }

    /// Slot 23 `GetPowerPlantCount(type)` (0 outside 0..9, [choice]).
    pub fn power_plant_count_of_type(&self, t: i32) -> i32 {
        Cx::type_index(t).map_or(0, |i| self.state().per_type[i])
    }

    /// Slot 24 `GetPowerPlantCount()`.
    pub fn power_plant_count(&self) -> i32 {
        self.state().plant_count
    }

    /// Slot 25 `GetPowerPlants`: every plant, each with a reference added.
    pub fn power_plants(&self) -> Vec<u64> {
        let plants: Vec<u64> = self.state().plants.iter().map(|p| p.occupant).collect();
        for &p in &plants {
            self.host.add_ref(p);
        }
        plants
    }

    /// Slot 26.
    pub fn power_pole_count(&self) -> u32 {
        self.state().pole_count
    }

    /// Slots 27–29.
    pub fn overworked_plant_count(&self) -> u32 {
        self.state().overworked
    }

    pub fn aged_plant_count(&self) -> u32 {
        self.state().aged
    }

    pub fn distressed_plant_count(&self) -> u32 {
        self.state().distressed
    }

    /// Slot 30 `FindPowerPlants` (outputs `AddRef`ed).
    pub fn find_power_plants(&self) -> FoundPlants {
        let found = self.run(|cx| cx.find_power_plants());
        if let Some((p, _)) = found.lowest_funded {
            self.host.add_ref(p);
        }
        if let Some((p, _)) = found.most_worn {
            self.host.add_ref(p);
        }
        found
    }

    /// Slot 31 `IsOccupantPowered` (§7.2).
    pub fn is_occupant_powered(&self, occ: u64) -> bool {
        let Some(lot) = self.host.occupant_lot(occ) else {
            return false;
        };
        self.host.lot_has_power(lot) || !self.host.lot_needs_power(lot)
    }

    /// Slot 32 `GetOccupantPowerConsumed` (§7.2).
    pub fn occupant_power_consumed(&self, occ: u64) -> Option<u32> {
        self.run(|cx| cx.prop_u32(occ, prop::REQUIREMENT))
    }

    /// Slot 33 `GetOccupantPowerProduced` (§7.2): (produced, used).
    pub fn occupant_power_produced(&self, occ: u64) -> Option<(u32, u32)> {
        self.run(|cx| {
            cx.prop(occ, prop::CAPACITY)?;
            let used = cx.prop_i32(occ, prop::USED).unwrap_or(0) as u32;
            Some((cx.current_power_produced(occ), used))
        })
    }

    /// Slot 34 `GetOccupantMaxPowerProduced` (§7.2).
    pub fn occupant_max_power_produced(&self, occ: u64) -> Option<u32> {
        self.run(|cx| cx.prop_u32(occ, prop::CAPACITY))
    }

    /// Slots 39 / 41 / 42: strikes are disabled (false).
    pub fn is_on_strike(&self) -> bool {
        false
    }

    pub fn begin_strike(&self) -> bool {
        false
    }

    pub fn end_strike(&self) -> bool {
        false
    }

    /// Slot 40 `GetChanceOfStrike`: writes 0.0.
    pub fn chance_of_strike(&self) -> f32 {
        0.0
    }

    /// Slot 43.
    pub fn strike_start_date(&self) -> i32 {
        self.state().strike_start
    }

    /// Slot 44: `start ≠ 0 ? GetSimTime() − start : −1`.
    pub fn strike_length(&self) -> i32 {
        let start = self.state().strike_start;
        if start != 0 {
            self.host.sim_time().wrapping_sub(start as u32) as i32
        } else {
            -1
        }
    }

    /// Slot 45 `GetCurrentPlantEfficiencyPercent` (§7.1).
    pub fn current_plant_efficiency(&self, occ: u64) -> f32 {
        self.run(|cx| cx.efficiency(occ))
    }

    /// Slot 46 `GetCurrentPlantConditionPercent` (§7.1).
    pub fn current_plant_condition(&self, occ: u64) -> f32 {
        self.run(|cx| cx.condition(occ))
    }

    /// `GetCurrentPowerProduced` (§7.1).
    pub fn current_power_produced(&self, occ: u64) -> u32 {
        self.run(|cx| cx.current_power_produced(occ))
    }

    /// Slot 47: the strike efficiency tuning value.
    pub fn strike_efficiency(&self) -> f32 {
        self.state().tuning.strike_efficiency
    }

    /// Slot 48: the department's funding percentage (`None` without one).
    pub fn funding_percent(&self) -> Option<f32> {
        let d = self.state().department?;
        Some(self.host.department_funding(d))
    }

    /// Slot 49 `GetMonthlyPowerGenerationCost` (§7.2).
    pub fn monthly_generation_cost(&self, occ: u64) -> Option<i64> {
        self.host.budget_item_for_purpose(occ, budget::PURPOSE).map(|i| i.cost)
    }

    /// Slot 50 `GetPowerPlantFundingPercentage` (§7.2).
    pub fn plant_funding_percentage(&self, occ: u64) -> Option<f32> {
        self.host
            .budget_item_for_purpose(occ, budget::PURPOSE)
            .map(|i| i.local_funding)
    }

    /// Slot 51 `SetPowerPlantFundingPercentage` (§7.2).
    pub fn set_plant_funding_percentage(&self, occ: u64, fraction: f32) -> bool {
        let Some(item) = self.host.budget_item_for_purpose(occ, budget::PURPOSE) else {
            return false;
        };
        let Some(dept) = self.host.department(item.department) else {
            return false;
        };
        self.host
            .set_local_funding_percent(dept, occ, fraction, budget::PURPOSE)
    }

    // ---- the cycle (§6, §7), exposed for hosts and tests --------------------------------------------

    /// One `DoIdle` step (§6).
    pub fn do_idle(&self) {
        self.run(|cx| cx.do_idle());
    }

    /// `DoMonthlySimulation` (§7.5).
    pub fn monthly(&self) {
        self.run(|cx| cx.monthly());
    }

    /// `UpdatePowerGrid(x, z)` (§6.3).
    pub fn update_power_grid(&self, x: i32, z: i32) {
        self.run(|cx| cx.update_power_grid(x, z));
    }

    /// §6.3 step 1 alone: the flood fill of the mark bitmap from (x, z);
    /// returns `(rowMin, rowMax)` after the clamp.
    pub fn flood(&self, x: i32, z: i32) -> (i32, i32) {
        self.run(|cx| cx.flood(x, z))
    }

    /// `AgePowerPlant(occ)` (§7.4).
    pub fn age_plant(&self, occ: u64) {
        self.run(|cx| cx.age_plant(occ));
    }

    /// `ApplyPoleFunding(fraction)` (§7.6).
    pub fn apply_pole_funding(&self, fraction: f32) {
        self.run(|cx| cx.apply_pole_funding(fraction));
    }
}
