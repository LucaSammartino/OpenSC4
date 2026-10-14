// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The landfill developer's state and behaviour (§1–§4, §6.1).
//!
//! A dezone with `changeZone` calls the zone manager's `PlaceZone`, which
//! **posts** `0x07699455` (§4.4.8 step 4 *[compat]*): the type-0
//! `DoZoneChange` arrives through [`LandfillDeveloper::do_message`] on a later
//! tick, so no loop here is re-entered.
//!
//! # Threading
//!
//! This object uses `Rc`/`RefCell`/`Cell` and is main-thread only, like every
//! other city component the game ticks from the simulator (foundations §19
//! rule 1). It is deliberately **not** `Sync`: no clock-reachable state lives
//! here, so the diagnostic and autotest threads of foundations §19 never see
//! it. Rule 4 is observed throughout — no `RefCell` borrow is held across a
//! call into the host, so every zone loop first clones the list of `Rc`
//! handles ([`LandfillDeveloper::zone_list`]).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::grid::CellRect;
use crate::rng::CrtRand;
use crate::stream::{InStream, OutStream, SaveObject};
use crate::x87::{ftol32, mul_i64_f32};

use super::algo::{altitude_variance, zoning_cost};
use super::tuning::Tuning;
use super::zone::LandfillZone;
use super::{
    budget_ids, history, msg, CellRegion, LandfillHost, LandfillMessage, CLSID_LANDFILL_DEVELOPER, DEFAULT_EFFICIENCY,
    IID_LANDFILL_DEVELOPER, IID_MESSAGE_TARGET2, IID_SERIALIZABLE, IID_UNKNOWN, ZONE_TYPE_LANDFILL,
};

/// The developer record's version (§6.1).
pub const DEVELOPER_RECORD_VERSION: u16 = 3;
/// [choice] The most zones one save read creates. The original has no limit;
/// the bound only stops a hostile record from allocating without end.
pub const MAX_SAVED_ZONES: usize = 1 << 16;

/// `QueryInterface` (§1): the four interfaces the class answers.
pub fn answers_iid(iid: u32) -> bool {
    matches!(
        iid,
        IID_UNKNOWN | IID_LANDFILL_DEVELOPER | IID_MESSAGE_TARGET2 | IID_SERIALIZABLE
    )
}

/// The float helpers of §4.7.1 step 3 and §4.9.2: one x87 division of two
/// exact `int64` values under 24-bit precision (foundations §18.3), i.e. the
/// quotient rounded once to `f32`.
///
/// The `int64 → f64` conversion is exact below 2^53 and rounds once above it;
/// the original's `fild qword` keeps the full 64-bit integer, so a department
/// expense of 2^53 or more could differ *[choice: the values in play — city
/// funds and a maintenance sum over at most 65 536 cells — are far below 2^53]*.
fn div_i64_f32(a: i64, b: i64) -> f32 {
    ((a as f64) / (b as f64)) as f32
}

/// The non-saved developer state (§2.1). Everything but `zones` is recomputed
/// on PostCityInit after a load.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DevState {
    /// Σ zone maintenance, the last `UpdateBudgetRequirements` (§4.9.3).
    pub maintenance: i64,
    /// Σ heap capacity of the **connected** zones (§4.6.5).
    pub capacity_total: u32,
    /// Σ garbage contained in the **connected** zones (§4.6.5).
    pub used_total: u32,
    /// Set by `Read` (§6.1), consulted on PostCityInit (§3.2).
    pub loaded: bool,
    /// The sanitation department `0x6A42A8F5`, from PostCityInit (§3.2).
    pub department: Option<u64>,
}

/// `cSC4LandfillDeveloper` (§1): the month-level agent that owns the landfill
/// zones, prices zoning, and moves garbage from pollution's supply into the
/// heaps.
pub struct LandfillDeveloper {
    host: Rc<dyn LandfillHost>,
    /// The GZCOM wrapper's identity, 0 when the core owns the object (§6.1's
    /// `SaveObject::host_token`).
    token: Cell<u64>,
    /// `Init` runs once per object (§1.1).
    initialised: Cell<bool>,
    /// The zones, in creation (or load) order (§2.1).
    zones: RefCell<Vec<Rc<LandfillZone>>>,
    state: RefCell<DevState>,
    tuning: RefCell<Tuning>,
    /// The process-wide C `rand()` (§4.8, foundations §12). Only the interior
    /// tile of a redrawn cell draws from it.
    crt: RefCell<CrtRand>,
}

impl LandfillDeveloper {
    /// A developer with no zones and the code-default tuning (§2.1, §4.1).
    /// `rand()` starts at its process-start state 1 (foundations §12: the game
    /// never seeds it); the landfill classes have no `cRZRandom`, so the
    /// session-derived seed of foundations §6.3 does not apply here.
    pub fn new(host: Rc<dyn LandfillHost>, token: u64) -> Rc<LandfillDeveloper> {
        Rc::new(LandfillDeveloper {
            host,
            token: Cell::new(token),
            initialised: Cell::new(false),
            zones: RefCell::new(Vec::new()),
            state: RefCell::new(DevState::default()),
            tuning: RefCell::new(Tuning::default()),
            crt: RefCell::new(CrtRand::default()),
        })
    }

    pub fn host(&self) -> &Rc<dyn LandfillHost> {
        &self.host
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    /// The tuning in effect (§4.1). A copy, so no borrow is held across a host
    /// call.
    pub fn tuning(&self) -> Tuning {
        self.tuning.borrow().clone()
    }

    pub fn state(&self) -> DevState {
        *self.state.borrow()
    }

    pub fn is_initialised(&self) -> bool {
        self.initialised.get()
    }

    /// The C `rand()` state the tile redraws draw from (§4.8).
    pub fn crt(&self) -> CrtRand {
        *self.crt.borrow()
    }

    /// Replaces the C `rand()` state (the host's `srand`, foundations §12.6).
    pub fn set_crt(&self, crt: CrtRand) {
        *self.crt.borrow_mut() = crt;
    }

    /// The zone list as owned handles (§2.1). Every loop over the zones starts
    /// here so that no borrow outlives a host call (foundations §19 rule 4).
    pub fn zone_list(&self) -> Vec<Rc<LandfillZone>> {
        self.zones.borrow().clone()
    }

    pub fn zone_count(&self) -> usize {
        self.zones.borrow().len()
    }

    fn edit<R>(&self, f: impl FnOnce(&mut DevState) -> R) -> R {
        let mut st = *self.state.borrow();
        let r = f(&mut st);
        *self.state.borrow_mut() = st;
        r
    }

    // ---- lifecycle (§1.1, §1.2) ----

    /// `Init()` (slot 3), once per object; a second call returns true at once.
    ///
    /// 1. On the first `Init` read the tuning exemplar (§4.1).
    /// 2. `AddAgent(messageTarget, 6, "cSC4LandfillDeveloper", 2)`.
    /// 3. Acquire the city, terrain, pollution, budget and message server.
    /// 4. Subscribe the five messages of §3.2.
    ///
    /// The sanitation department is **not** acquired here; PostCityInit fetches
    /// it (§3.2).
    pub fn init(&self) -> bool {
        if self.initialised.get() {
            return true;
        }
        self.initialised.set(true);
        // 1.
        let exemplar = self.host.tuning();
        self.tuning.borrow_mut().read(exemplar.as_deref());
        // 2.
        self.host
            .add_agent(super::AGENT_LEVEL, super::AGENT_NAME, super::AGENT_FLAGS);
        // 3.
        self.host.acquire_services();
        // 4.
        for m in msg::SUBSCRIPTIONS {
            self.host.subscribe(m);
        }
        true
    }

    /// `Shutdown()` (slot 4): unsubscribe the five messages; for every zone
    /// `Shutdown` and `Release`; clear the list; release the department, then
    /// the services; `RemoveAgent`.
    ///
    /// *[choice]* The original releases the message server right after the
    /// unsubscribes and the city last; the core makes one
    /// [`LandfillHost::release_services`] call after the zones are shut down,
    /// so a zone's effects are stopped while the city still holds them.
    pub fn shutdown(&self) -> bool {
        for m in msg::SUBSCRIPTIONS {
            self.host.unsubscribe(m);
        }
        let zones = std::mem::take(&mut *self.zones.borrow_mut());
        for z in zones {
            z.shutdown();
            // The core's release is dropping the `Rc` (§1.2).
        }
        let department = self.edit(|st| st.department.take());
        if let Some(d) = department {
            self.host.release(d);
        }
        self.host.release_services();
        self.host.remove_agent();
        true
    }

    // ---- messages (§3.2) ----

    /// `DoMessage` (§3.1, §3.2). Returns **true** for every message.
    pub fn do_message(&self, m: &LandfillMessage) -> bool {
        match m.msg_type {
            msg::ZONING_CHANGE => {
                // `region = GetVoid2`, `zoneType = GetData3`. Runs for every
                // zone type. A message without a region cannot be acted on
                // [choice: guard].
                if let Some(region) = &m.region {
                    self.do_zone_change(region, m.data3 as i32);
                }
            }
            msg::TERRAIN_MODIFIED => self.handle_terrain_modified(m),
            msg::FUNDS_CHANGED => self.handle_funds_changed(),
            msg::POST_CITY_INIT => self.handle_post_city_init(),
            msg::SIM_NEW_MONTH => self.do_monthly_simulation(),
            msg::PRE_VIEW_LEVEL_CHANGE => self.handle_view_level_change(),
            _ => {}
        }
        true
    }

    /// Terrain modified (§3.2): only when the zone list is non-empty, and only
    /// for the terraforming tool reasons 1–3 (the landfill's own levelling uses
    /// reason 5, which is ignored).
    fn handle_terrain_modified(&self, m: &LandfillMessage) {
        if self.zones.borrow().is_empty() {
            return;
        }
        let (cx, cz) = self.host.cell_counts();
        let rect = CellRect::new(
            ((m.data1 >> 16) as i32 - 1).max(0),
            ((m.data2 >> 16) as i32 - 1).max(0),
            ((m.data1 & 0xFFFF) as i32).min(cx - 1),
            ((m.data2 & 0xFFFF) as i32).min(cz - 1),
        );
        if super::TERRAIN_TOOL_REASONS.contains(&m.data4) {
            self.do_terrain_changed(rect);
        }
    }

    /// FundsChanged (§3.2): only when `department` is set. The truck automaton
    /// follows the funding; no simulation state changes.
    fn handle_funds_changed(&self) {
        let Some(department) = self.state.borrow().department else {
            return;
        };
        let funded = self.host.department_funding_percentage(department) > 0.0;
        for z in self.zone_list() {
            if let Some(truck) = z.effects().truck {
                self.host.set_truck_enabled(truck, funded);
            }
        }
    }

    /// PostCityInit (§3.2): fetch the sanitation department, then — when this
    /// object was loaded — `Init` every zone and recompute the budget and the
    /// capacity totals (§2.1).
    fn handle_post_city_init(&self) {
        // A previous department is **not** released [quirk: leak on a second
        // PostCityInit]. The core drops its token, which is all it has.
        if let Some(d) = self.host.get_department_budget(budget_ids::SANITATION_DEPARTMENT) {
            self.host.add_ref(d);
            self.edit(|st| st.department = Some(d));
        }
        if !self.state.borrow().loaded {
            return;
        }
        for z in self.zone_list() {
            z.init();
        }
        self.update_budget_requirements();
        self.update_capacity_values();
    }

    /// PreViewLevelChange (§3.2): rendering only. The 3D view stores the
    /// **new** level before sending this message.
    fn handle_view_level_change(&self) {
        if self.zones.borrow().is_empty() {
            return;
        }
        let level = self.host.current_view_level();
        for z in self.zone_list() {
            let effects = z.effects();
            let Some(seagull) = effects.seagull else { continue };
            if level == 1 && !effects.seagull_started {
                self.host.start_seagull(seagull, 0);
                z.set_seagull_started(true);
            } else if level != 1 && effects.seagull_started {
                self.host.stop_seagull(seagull, 1);
                z.set_seagull_started(false);
            }
        }
    }

    // ---- zoning (§4.4.1–§4.4.6) ----

    /// `DoZoneChange(region, zoneType)` (§4.4.1).
    ///
    /// 1. Collect, in list order, every zone whose rect intersects the region's
    ///    rect inflated by one cell (inclusive; diagonal neighbours count).
    /// 2. With at least one collected zone: zone type 14 shuts the effects of
    ///    every collected zone down and extends the **first** one, handling the
    ///    others as overlaps; any other type handles them all as overlaps with
    ///    the texture removed. Then `UpdateBudgetRequirements`.
    /// 3. With none, and type 14: create, `Init`, level (or just record the
    ///    mean), populate, price, append. *[quirk]* `UpdateBudgetRequirements`
    ///    is **not** called.
    /// 4. Always `UpdateCapacityValues`.
    pub fn do_zone_change(&self, region: &CellRegion, zone_type: i32) {
        let rect = region.rect();
        let inflated = CellRegion::inflate(rect);
        let collected: Vec<Rc<LandfillZone>> = self
            .zone_list()
            .into_iter()
            .filter(|z| CellRegion::intersects(z.rect(), inflated))
            .collect();

        if !collected.is_empty() {
            for (i, z) in collected.iter().enumerate() {
                if zone_type == ZONE_TYPE_LANDFILL {
                    z.shutdown_zone_effects();
                    if i == 0 {
                        z.add_region_to_zone(region);
                        z.populate_zone();
                        let tile_capacity = self.host.landfill_tile_capacity();
                        z.recalculate_capacity(tile_capacity);
                        z.init_zone_effects();
                    } else {
                        self.handle_zone_overlap(z, region, false, false);
                    }
                } else {
                    self.handle_zone_overlap(z, region, false, true);
                }
            }
            // 2.
            self.update_budget_requirements();
        } else if zone_type == ZONE_TYPE_LANDFILL {
            // 3.
            let zone = LandfillZone::new(self.host.clone(), region.clone());
            zone.init();
            let v = altitude_variance(rect, self.host.sea_level(), |x, z| self.host.cell_altitude(x, z));
            let tolerance = self.tuning.borrow().level_tolerance;
            if v.variance <= tolerance {
                zone.set_level_altitude(v.mean);
            } else {
                zone.level_rect(rect, v.mean, false);
            }
            zone.populate_zone();
            let tile_capacity = self.host.landfill_tile_capacity();
            zone.recalculate_capacity(tile_capacity);
            self.zones.borrow_mut().push(zone);
        }
        // 4.
        self.update_capacity_values();
    }

    /// `HandleZoneOverlap(Z, region, changeZone, removeTexture)` (§4.4.2).
    fn handle_zone_overlap(
        &self,
        zone: &Rc<LandfillZone>,
        region: &CellRegion,
        change_zone: bool,
        remove_texture: bool,
    ) {
        // 1.
        let intersection = CellRegion::intersection(zone.rect(), region.rect());
        if let Some(i) = intersection {
            self.dezone_area(zone, i, region, change_zone, remove_texture);
        }
        // 2.
        if zone.is_completely_dezoned() {
            self.remove_zone(zone, true);
        } else {
            let tile_capacity = self.host.landfill_tile_capacity();
            zone.recalculate_capacity(tile_capacity);
        }
        // 3. Display only; there is no intersection to redisplay when step 1
        // found none [choice].
        if let Some(i) = intersection {
            self.host.redisplay_terrain(i);
        }
    }

    /// `DezoneArea(Z, I, region, changeZone, removeTexture)` (§4.4.3): dezone
    /// every cell of `I` whose bit is set in the **region** mask, then mark the
    /// zone completely dezoned when it keeps no valid heap.
    fn dezone_area(
        &self,
        zone: &Rc<LandfillZone>,
        i: CellRect,
        region: &CellRegion,
        change_zone: bool,
        remove_texture: bool,
    ) {
        for x in i.x1..=i.x2 {
            for z in i.z1..=i.z2 {
                if region.is_set(x, z) {
                    zone.dezone_cell(x, z, change_zone, remove_texture);
                }
            }
        }
        if zone.num_valid_heaps() < 1 {
            zone.set_completely_dezoned(true);
        }
    }

    /// `RemoveZone(Z, dezoneCells)` (§4.4.6): out of the list; with
    /// `dezoneCells` every mask cell gets `DezoneCell(x, z, true, true)` (its
    /// tile removed and a type-0 `PlaceZone` posted); then `Shutdown` and
    /// `Release`. No capacity or budget update here — the callers do it, or
    /// not (§4.5.1).
    pub fn remove_zone(&self, zone: &Rc<LandfillZone>, dezone_cells: bool) {
        {
            let mut zones = self.zones.borrow_mut();
            if let Some(i) = zones.iter().position(|z| Rc::ptr_eq(z, zone)) {
                zones.remove(i);
            }
        }
        if dezone_cells {
            let region = zone.region();
            for (x, z) in region.mask_cells().collect::<Vec<_>>() {
                zone.dezone_cell(x, z, true, true);
            }
        }
        zone.shutdown();
    }

    // ---- terrain (§4.5.1, §4.5.2) ----

    /// `DoTerrainChanged(rect)` (§4.5.1): every zone handles the change
    /// (`HandleTerrainChange`, §4.5.2), then every zone for which
    /// `IsCompletelyDezoned()` is true, in list order, is removed with
    /// `RemoveZone(Z, true)`.
    ///
    /// *[quirk]* Neither `RecalculateCapacity` nor `UpdateCapacityValues` nor
    /// `UpdateBudgetRequirements` runs.
    pub fn do_terrain_changed(&self, rect: CellRect) {
        let move_with_terrain = self.tuning.borrow().move_with_terrain;
        for z in self.zone_list() {
            z.handle_terrain_change(rect, move_with_terrain);
        }
        for z in self.zone_list() {
            if z.is_completely_dezoned() {
                self.remove_zone(&z, true);
            }
        }
    }

    // ---- capacity (§4.6.5) ----

    /// `UpdateCapacityValues()`: `capacityTotal := usedTotal := 0`, then the
    /// totals of every **connected** zone, each a u32 add that saturates at
    /// `0xFFFFFFFF`. Disconnected zones contribute nothing.
    pub fn update_capacity_values(&self) {
        let mut used = 0u32;
        let mut capacity = 0u32;
        for z in self.zone_list() {
            if !z.connected() {
                continue;
            }
            let (_, contained, total) = z.get_capacity();
            used = used.saturating_add(contained);
            capacity = capacity.saturating_add(total);
        }
        self.edit(|st| {
            st.used_total = used;
            st.capacity_total = capacity;
        });
    }

    /// `GetLandfillUsage(usage&, capacity&)` (slot 10): `capacity :=
    /// capacityTotal`, `usage := min(usedTotal, capacityTotal)`.
    pub fn get_landfill_usage(&self) -> (u32, u32) {
        let st = *self.state.borrow();
        (st.used_total.min(st.capacity_total), st.capacity_total)
    }

    /// `GetLandfillTileCount()` (slot 11): the number of zone-14 cells in the
    /// city, not limited to connected zones or mask cells.
    pub fn get_landfill_tile_count(&self) -> i32 {
        self.host.zone_count(ZONE_TYPE_LANDFILL)
    }

    // ---- monthly simulation (§4.7.1) ----

    /// `DoMonthlySimulation()` (§4.7.1).
    pub fn do_monthly_simulation(&self) {
        let t = self.tuning();
        if self.zones.borrow().is_empty() {
            // The stale totals are recorded, the budget is refreshed and the
            // valve is closed.
            self.record_history();
            self.update_budget_requirements();
            self.host.set_garbage_capacity(0);
            return;
        }

        // 1.
        let current = self.get_current_budget();
        let maint = self.state.borrow().maintenance;
        // 2.
        let mut taken = self.host.garbage_supply().min(t.max_per_month);
        // 3. An underfunded department takes proportionally less: one f32
        //    division of the exact int64 values, one f32 multiplication by the
        //    int32 `t`, then truncation toward zero (`_ftol`).
        if maint > 0 && current < maint {
            let fraction = div_i64_f32(current, maint);
            taken = ftol32(f64::from(mul_i64_f32(i64::from(taken as i32), fraction))) as u32;
        }
        // 4. `t` is the remaining supply, shared by every zone in list order;
        //    the zone's budget and the remaining count are `int32` in the
        //    original, so the `u32` supply is reinterpreted here. It is at most
        //    `kMaxGarbageToTakePerMonth` (shipped 327 680 000, inside `int32`)
        //    *[choice: a tuning value above 2^31 wraps, as the original's would]*.
        let mut remaining = taken as i32;
        let mut available = 0i32;
        for z in self.zone_list() {
            z.update_zone(
                &mut remaining,
                t.max_per_month_per_tile,
                t.decay_rate,
                t.max_decay_per_zone,
                &mut self.crt.borrow_mut(),
            );
            if z.connected() {
                available = available.wrapping_add(z.available_capacity());
            }
        }
        // 5.
        self.host.set_garbage_capacity(available);
        self.update_budget_requirements();
        self.update_capacity_values();
        // 6.
        self.record_history();
    }

    /// §4.7.1 step 6 (and the empty-list case): `0x2A8C4696 := capacityTotal`,
    /// `0x2A8C4697 := usedTotal + max(0, GetTotalGarbageUncollected()) +
    /// GetGarbageSupply()` (u32 wrap), with the pollution values read **after**
    /// the zones were updated.
    fn record_history(&self) {
        let st = *self.state.borrow();
        self.host.send_history(history::CAPACITY, st.capacity_total);
        let uncollected = self.host.total_garbage_uncollected().max(0) as u32;
        let total = st
            .used_total
            .wrapping_add(uncollected)
            .wrapping_add(self.host.garbage_supply());
        self.host.send_history(history::TOTAL_GARBAGE, total);
    }

    // ---- costs and budget (§4.3, §4.9) ----

    /// `GetCurrentBudget()` (slot 8) = line 30's current expenses (§4.9.2). The
    /// department is not null-checked in the original *[quirk: crash before
    /// PostCityInit]*; the core reads 0 when there is none [choice: guard].
    pub fn get_current_budget(&self) -> i64 {
        let Some(department) = self.state.borrow().department else {
            return 0;
        };
        self.host
            .line_item_current_expenses(department, budget_ids::LANDFILL_LINE)
            .unwrap_or(0)
    }

    /// `GetMaintenanceCosts()` (slot 7) = `maintenance` (§4.9.2).
    pub fn get_maintenance_costs(&self) -> i64 {
        self.state.borrow().maintenance
    }

    /// `GetEfficiency()` (slot 5, §4.9.2): `float32(current / maintenance)`
    /// when the maintenance is positive — not clamped, so up to the
    /// department's maximum funding — else [`DEFAULT_EFFICIENCY`].
    pub fn get_efficiency(&self) -> f32 {
        let maint = self.state.borrow().maintenance;
        if maint > 0 {
            div_i64_f32(self.get_current_budget(), maint)
        } else {
            DEFAULT_EFFICIENCY
        }
    }

    /// `UpdateBudgetRequirements()` (§4.9.3): `sum := Σ
    /// zone.GetMaintenanceCost()` (int64, saturating at `INT64_MAX`), written to
    /// line 30's full expenses, then stored as `maintenance`. The original does
    /// not null-check the department; the core skips the write when there is
    /// none and still stores the sum [choice: guard].
    pub fn update_budget_requirements(&self) {
        let default_tile_cost = self.tuning.borrow().default_tile_cost;
        let mut sum = 0i64;
        for z in self.zone_list() {
            sum = sum.saturating_add(z.maintenance_cost(default_tile_cost));
        }
        if let Some(department) = self.state.borrow().department {
            self.host
                .set_line_item_full_expenses(department, budget_ids::LANDFILL_LINE, sum);
        }
        self.edit(|st| st.maintenance = sum);
    }

    /// `GetZoningCost(region)` (slot 9, §4.3):
    ///
    /// 1. `target := levelAltitude` of the **first** zone (list order) whose
    ///    rect intersects the region's rect, inclusively. No tolerance test.
    /// 2. Otherwise the region's own variance decides: within the tolerance the
    ///    zoning is free, else the target is the mean.
    /// 3. The price is `kCostPerCellHeightChange × Σ |target − altitude|` over
    ///    the **mask** cells, magic-rounded, with a high dword that is always 0.
    pub fn get_zoning_cost(&self, region: &CellRegion) -> i64 {
        let t = self.tuning();
        let rect = region.rect();
        let target = match self.zone_list().iter().find(|z| CellRegion::intersects(z.rect(), rect)) {
            Some(z) => z.level_altitude(),
            None => {
                let v = altitude_variance(rect, self.host.sea_level(), |x, z| self.host.cell_altitude(x, z));
                if v.variance <= t.level_tolerance {
                    return 0;
                }
                v.mean
            }
        };
        zoning_cost(region, Some(target), t.cost_per_cell_height_change, |x, z| {
            self.host.cell_altitude(x, z)
        })
    }

    /// `GetLandfillZoneFromCell(x, z)` (slot 12, §7.1): the first zone (list
    /// order) whose **rect** contains the cell, half-open, so the last row and
    /// column of a zone are never found *[quirk 1]* and the mask is ignored. No
    /// reference is taken: the core returns an `Rc` clone and the plugin decides
    /// what to hand out.
    pub fn get_landfill_zone_from_cell(&self, x: i32, z: i32) -> Option<Rc<LandfillZone>> {
        self.zone_list().into_iter().find(|zone| zone.contains_cell(x, z))
    }

    // ---- persistence (§6.1) ----

    /// Writes the developer record (§6.1). Returns true always.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        let zones = self.zone_list();
        out.put_u16(DEVELOPER_RECORD_VERSION);
        out.put_i32(zones.len() as i32);
        for z in &zones {
            z.write(out);
        }
        true
    }

    /// Reads the developer record (§6.1).
    ///
    /// Sets `loaded` **before** checking the version; a version other than 3
    /// fails. The zone list is cleared **without** releasing the old zones
    /// *[quirk: only matters on a second Read]* — the core drops its `Rc`s,
    /// which is the same thing once nothing else holds one. A zone whose own
    /// read fails is skipped. Returns **true** regardless of stream errors
    /// [compat].
    pub fn read(&self, inp: &mut dyn InStream) -> bool {
        self.edit(|st| st.loaded = true);
        let Some(version) = inp.get_u16() else {
            return false;
        };
        if version != DEVELOPER_RECORD_VERSION {
            return false;
        }
        // On a stream error the original leaves the count uninitialised
        // [quirk]; the core uses 0 [choice].
        let count = inp.get_i32().unwrap_or(0).max(0) as usize;
        self.zones.borrow_mut().clear();
        for _ in 0..count.min(MAX_SAVED_ZONES) {
            let zone = LandfillZone::empty(self.host.clone());
            if zone.read(inp) {
                self.zones.borrow_mut().push(zone);
            }
            // On failure the zone is leaked and skipped: the core drops it.
        }
        true
    }
}

impl SaveObject for LandfillDeveloper {
    fn class_id(&self) -> u32 {
        CLSID_LANDFILL_DEVELOPER
    }

    /// §6.1. Returns true always.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        LandfillDeveloper::write(self, out)
    }

    /// §6.1: sets `loaded`, requires version 3, returns true regardless of
    /// stream errors [compat].
    fn read(&self, inp: &mut dyn InStream) -> bool {
        LandfillDeveloper::read(self, inp)
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn std::any::Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
