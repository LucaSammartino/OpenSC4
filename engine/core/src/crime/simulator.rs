// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4CrimeSimulator`: state (§3), lifecycle (§4), messages and pipeline
//! (§5), algorithms (§6), interface slots (§11) and the save record (§12).
//!
//! No `RefCell` borrow is held across a host callback.

use std::any::Any;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::rc::Rc;

use crate::grid::{add_radius_effect, CellIterator, TractGrid, IID_SIM_GRID, MAX_TRACTS};
use crate::stream::{InStream, OutStream, RefValue, ResKey, SaveObject};

use super::algo::{
    census_bucket, crimes_to_commit, criminal_base_term, decay_value, ftol32, jobless_factor, mul_f32, rhe, PROP_SCALE,
};
use super::types::{choose_candidate, CrimeType, LotBits};
use super::{
    msg, prop, CrimeHost, CrimeShared, CrimeTuning, LotRef, NearbyOccupant, NearestKind, AGENT_FLAGS, AGENT_LEVELS,
    AGENT_NAME, ARSON_CAUSE, CLSID_CRIME_SIMULATOR, CRIME_TYPE_GROUP, DEMAND_IDS, IID_LOT, INSTANCE_CRIME_GRID,
    INSTANCE_EFFECT_GRID, PROP_SATISFACTION_ID, SIMULATOR_TYPE, TUNING_KEY,
};

/// The record version written and accepted (§12).
pub const CRIME_RECORD_VERSION: u16 = 6;
/// The x87 default NaN (`float32` bits), the average of an unzoned city
/// (§6.9).
pub const DEFAULT_NAN: u32 = 0xFFC0_0000;
/// The idle pipeline state (§5).
pub const STATE_IDLE: u32 = 6;

/// One incident of the last run (§6.8, §12).
#[derive(Clone, Debug, PartialEq)]
pub struct Incident {
    /// `None` only after a load of a null reference.
    pub lot: Option<LotRef>,
    pub type_id: u32,
    pub name: Vec<u8>,
    pub icon: ResKey,
    /// Not saved (false after a load, §11.5).
    pub visuals_created: bool,
}

/// One element of `GetCrimeIncidentArray` (`tCrimeEvent`, 28 bytes in the
/// game: `u32 cellX, cellZ, typeID; u8 visuals; 3 padding; ResKey`, §11.4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrimeEvent {
    pub cell_x: u32,
    pub cell_z: u32,
    pub type_id: u32,
    pub visuals_created: bool,
    pub icon: ResKey,
}

/// The simulator state (§3).
#[derive(Clone)]
pub struct CrimeData {
    pub initialized: bool,
    /// Set by `Read` (§12).
    pub loaded_from_save: bool,
    /// The message server is held and the four subscriptions made (§4.1).
    pub has_message_server: bool,
    pub crime_grid: Option<Rc<dyn TractGrid<u8>>>,
    pub effect_grid: Option<Rc<dyn TractGrid<u8>>>,
    /// Saved, never advanced (§3).
    pub iterator: CellIterator,
    pub average_crime: f32,
    pub ordinance_factor: f32,
    pub monthly_crime_count: u32,
    pub hotspot_x: i32,
    pub hotspot_z: i32,
    pub hotspot_value: i32,
    pub incidents: Vec<Incident>,
    /// Pipeline state 0–6 (§5), not saved.
    pub state: u32,
    pub criminal_base: i32,
    pub crimes_to_commit: i32,
    /// `pop[wealth][bucket]` (§6.1).
    pub census: [[u32; 4]; 3],
    /// Jobless factor per wealth (§6.2).
    pub jobless: [f32; 3],
    pub tuning: CrimeTuning,
    /// Not saved, not in the original: bumped whenever the incident list
    /// changes (cleared, appended to, or extended by a load), so a host that
    /// mirrors the list (cross-thread getters, foundations §19) knows when to
    /// rebuild its copy. The visuals flags do not bump it.
    pub incident_epoch: u32,
}

impl Default for CrimeData {
    /// The constructor's state (§3).
    fn default() -> Self {
        CrimeData {
            initialized: false,
            loaded_from_save: false,
            has_message_server: false,
            crime_grid: None,
            effect_grid: None,
            iterator: CellIterator::constructed(),
            average_crime: 0.0,
            ordinance_factor: 1.0,
            monthly_crime_count: 0,
            hotspot_x: 0,
            hotspot_z: 0,
            hotspot_value: 0,
            incidents: Vec::new(),
            state: STATE_IDLE,
            criminal_base: 0,
            crimes_to_commit: 0,
            census: [[0; 4]; 3],
            jobless: [0.0; 3],
            tuning: CrimeTuning::default(),
            incident_epoch: 0,
        }
    }
}

/// A message delivered to `DoMessage` (the fields crime reads).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrimeMessage {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    /// `GetVoid1()`: the occupant of the occupant messages.
    pub void1: u64,
}

/// The crime simulator.
pub struct CrimeSim {
    host: Rc<dyn CrimeHost>,
    shared: Rc<CrimeShared>,
    token: Cell<u64>,
    d: RefCell<CrimeData>,
}

/// The fields of a record (§12) read before they are committed.
struct Record {
    crime_grid: Option<Rc<dyn TractGrid<u8>>>,
    effect_grid: Option<Rc<dyn TractGrid<u8>>>,
    iterator: CellIterator,
    average_crime: f32,
    ordinance_factor: f32,
    monthly_crime_count: u32,
    hotspot: [i32; 3],
    incidents: Vec<Incident>,
}

fn clamp_count(c: i32) -> i32 {
    c.clamp(0, MAX_TRACTS)
}

impl CrimeSim {
    /// A new simulator in the constructor's state (§3), sharing the
    /// process-wide type list and generator.
    pub fn new(host: Rc<dyn CrimeHost>, token: u64, shared: Rc<CrimeShared>) -> Rc<CrimeSim> {
        Rc::new(CrimeSim {
            host,
            shared,
            token: Cell::new(token),
            d: RefCell::new(CrimeData::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn shared(&self) -> &Rc<CrimeShared> {
        &self.shared
    }

    /// The state (read-only view).
    pub fn data(&self) -> Ref<'_, CrimeData> {
        self.d.borrow()
    }

    /// Direct state access, for hosts that restore state and for tests.
    pub fn data_mut(&self) -> RefMut<'_, CrimeData> {
        self.d.borrow_mut()
    }

    // ---- lifecycle (§4) ---------------------------------------------------------------------------

    /// `Init()` (§4.1). Always true.
    pub fn init(&self) -> bool {
        {
            let mut d = self.d.borrow_mut();
            if d.initialized {
                return true;
            }
            d.initialized = true;
        }
        if self.shared.types.borrow().is_none() {
            self.load_types();
        }
        self.read_tuning();
        self.host.add_agent(AGENT_NAME, &AGENT_LEVELS, AGENT_FLAGS);
        let subscribe = !std::mem::replace(&mut self.d.borrow_mut().has_message_server, true);
        if subscribe {
            self.host.subscribe(&msg::SUBSCRIBE);
        }
        self.host.acquire_services();
        let size = self.d.borrow().tuning.tract_size as i32;
        for instance in [INSTANCE_CRIME_GRID, INSTANCE_EFFECT_GRID] {
            let missing = {
                let d = self.d.borrow();
                if instance == INSTANCE_CRIME_GRID {
                    d.crime_grid.is_none()
                } else {
                    d.effect_grid.is_none()
                }
            };
            if !missing {
                continue;
            }
            let grid = self.host.create_grid(instance, size);
            if let Some(g) = &grid {
                g.fill(0);
            }
            let mut d = self.d.borrow_mut();
            if instance == INSTANCE_CRIME_GRID {
                d.crime_grid = grid;
            } else {
                d.effect_grid = grid;
            }
        }
        let new_city = !self.d.borrow().loaded_from_save;
        if new_city {
            {
                let mut d = self.d.borrow_mut();
                let (nx, nz) = d.crime_grid.as_ref().map_or((0, 0), |g| (g.count_x(), g.count_z()));
                d.iterator.set_rect(0, 0, nx.wrapping_sub(1), nz.wrapping_sub(1));
            }
            self.host.add_prop_requester();
        }
        let mut d = self.d.borrow_mut();
        d.hotspot_x = 0;
        d.hotspot_z = 0;
        d.hotspot_value = 0;
        d.ordinance_factor = 1.0;
        true
    }

    /// `Shutdown()` (§4.2). Always true.
    pub fn shutdown(&self) -> bool {
        let (grids, had_server) = {
            let mut d = self.d.borrow_mut();
            if !d.initialized {
                return true;
            }
            d.initialized = false;
            let grids = [d.crime_grid.take(), d.effect_grid.take()];
            (grids, std::mem::replace(&mut d.has_message_server, false))
        };
        for g in grids.into_iter().flatten() {
            self.host.shutdown_grid(&g);
        }
        self.host.release_services();
        self.clear_incidents();
        if had_server {
            self.host.unsubscribe(&msg::UNSUBSCRIBE);
        }
        self.host.remove_agent();
        true
    }

    pub fn is_initialized(&self) -> bool {
        self.d.borrow().initialized
    }

    /// Loads (or reloads) the process-wide crime type list (§7).
    pub fn load_types(&self) {
        let exemplars = self.host.crime_type_exemplars();
        let host = &self.host;
        let ltext = |k: ResKey| host.ltext(k);
        let types: Vec<CrimeType> = exemplars
            .iter()
            .map(|(instance, h)| CrimeType::from_exemplar(*instance, h, &ltext))
            .collect();
        *self.shared.types.borrow_mut() = Some(Rc::new(types));
    }

    /// Reads the tuning (§8).
    pub fn read_tuning(&self) {
        let exemplar = self.host.tuning();
        self.d.borrow_mut().tuning.read(exemplar.as_deref());
    }

    fn clear_incidents(&self) {
        let old = {
            let mut d = self.d.borrow_mut();
            d.incident_epoch = d.incident_epoch.wrapping_add(1);
            std::mem::take(&mut d.incidents)
        };
        for i in old {
            if let Some(lot) = i.lot {
                self.host.release(lot.token);
            }
        }
    }

    // ---- messages (§5) ----------------------------------------------------------------------------

    /// `DoMessage` (§5). Always true.
    pub fn handle_message(&self, m: CrimeMessage) -> bool {
        match m.msg_type {
            msg::SIM_NEW_DAY | msg::SIM_IDLE => self.do_idle(),
            msg::SIM_NEW_MONTH => {
                let mut d = self.d.borrow_mut();
                if d.state == STATE_IDLE {
                    d.state = 0;
                }
            }
            msg::OCCUPANT_INSERTED | msg::OCCUPANT_REMOVED => {
                if !self.host.occupant_has_property(m.void1, prop::PREVIEW_OCCUPANT) {
                    let sign = if m.msg_type == msg::OCCUPANT_INSERTED { 1 } else { -1 };
                    self.occupant_changed(m.void1, sign);
                }
            }
            msg::ORDINANCE_UPDATED => {
                // [choice] §4.1 step 6, §16 Q1 (CQ6): with no ordinance
                // simulator the factor is left unchanged (1.0 since Init).
                if let Some(f) = self.host.ordinance_crime_factor() {
                    self.d.borrow_mut().ordinance_factor = f;
                }
            }
            msg::EXEMPLAR_CHANGED => {
                if m.data1 == TUNING_KEY.group && m.data2 == TUNING_KEY.instance {
                    self.read_tuning();
                }
                if m.data1 == CRIME_TYPE_GROUP {
                    self.load_types();
                }
            }
            _ => {}
        }
        true
    }

    /// `DoIdle()`: one pipeline step (§5).
    pub fn do_idle(&self) {
        let (state, initialized) = {
            let d = self.d.borrow();
            (d.state, d.initialized)
        };
        let next = match state {
            0 => {
                if initialized {
                    1
                } else {
                    0
                }
            }
            1 => {
                self.do_census();
                self.calculate_economic_factors();
                2
            }
            2 => {
                self.compute_criminal_base();
                3
            }
            3 => {
                self.compute_crimes_to_commit();
                4
            }
            4 => {
                self.decay_crime_grid();
                self.clear_incidents();
                let n = self.d.borrow().crimes_to_commit;
                let count = self.add_crimes(n);
                self.d.borrow_mut().monthly_crime_count = count as u32;
                self.host.process_monthly_crime_stats();
                self.compute_average_crime();
                5
            }
            5 => {
                self.host.post_crime_map_updated();
                STATE_IDLE
            }
            other => other,
        };
        self.d.borrow_mut().state = next;
    }

    // ---- algorithms (§6) --------------------------------------------------------------------------

    /// `DoCensus` (§6.1): `GetPopulationGrids` (residential slot 25) first,
    /// then `GetEQGrids` (slot 23); only their three out pointers are used,
    /// never the return value (§13, CQ15).
    pub fn do_census(&self) {
        let pops = self.host.population_grids();
        let eqs = self.host.eq_grids();
        let mut census = [[0u32; 4]; 3];
        for w in 0..3 {
            let (Some(p), Some(e)) = (&pops[w], &eqs[w]) else {
                continue;
            };
            for x in 0..clamp_count(p.count_x()) {
                for z in 0..clamp_count(p.count_z()) {
                    if let Some(b) = census_bucket(e.get(x, z)) {
                        census[w][b] = census[w][b].wrapping_add(u32::from(p.get(x, z)));
                    }
                }
            }
        }
        self.d.borrow_mut().census = census;
    }

    /// `CalculateEconomicFactors` (§6.2).
    pub fn calculate_economic_factors(&self) {
        let mut jobless = [0.0f32; 3];
        for (w, ids) in DEMAND_IDS.iter().enumerate() {
            let (mut a, mut b, mut c) = (0.0f32, 0.0f32, 0.0f32);
            for &id in ids.iter() {
                if let Some(v) = self.host.demand(id) {
                    a += v.value;
                    b += v.max;
                    c += v.min;
                }
            }
            jobless[w] = jobless_factor(a, b, c);
        }
        self.d.borrow_mut().jobless = jobless;
    }

    /// `ComputeCriminalBase` (§6.3).
    pub fn compute_criminal_base(&self) {
        let mut d = self.d.borrow_mut();
        let mut base = 0i32;
        for w in 0..3 {
            let t = &d.tuning;
            let eq_y = [0.0f32, 25.0, 50.0, 75.0].map(|x| t.eq_curves[w].eval(x));
            let jobless_y = t.jobless_curves[w].eval(d.jobless[w]);
            base = base.wrapping_add(criminal_base_term(eq_y, d.census[w], jobless_y));
        }
        d.criminal_base = base;
    }

    /// Pipeline state 3 (§6.4): two police calls, in this order.
    pub fn compute_crimes_to_commit(&self) {
        let jailed = self.host.jail_population();
        let released = self.host.reset_unjailed_criminals();
        let mut d = self.d.borrow_mut();
        d.crimes_to_commit = crimes_to_commit(
            d.criminal_base,
            jailed,
            released,
            d.tuning.reoffend,
            d.ordinance_factor,
            &d.tuning.crimes_curve,
        );
    }

    /// `DecayCrimeGrid` (§6.5).
    pub fn decay_crime_grid(&self) {
        let (grid, decay, hx, hz) = {
            let d = self.d.borrow();
            (d.crime_grid.clone(), d.tuning.decay, d.hotspot_x, d.hotspot_z)
        };
        let value = match grid {
            Some(g) => {
                decay_grid(&*g, decay);
                i32::from(g.get(hx, hz))
            }
            None => 0,
        };
        self.d.borrow_mut().hotspot_value = value;
    }

    /// `AddCrimes(n)` (§6.6): the number of crimes that happened.
    pub fn add_crimes(&self, n: i32) -> i32 {
        if n <= 0 {
            return 0;
        }
        let (crime, effect, curve) = {
            let d = self.d.borrow();
            (
                d.crime_grid.clone(),
                d.effect_grid.clone(),
                d.tuning.attenuation_curve.clone(),
            )
        };
        let police = self.host.police_grid();
        let mut count = 0;
        // All `n` attempts (§6.4: up to `rhe(+∞)` = 876,609,536 with
        // modded tuning, as in the original).
        for _ in 0..n {
            let Some(lot) = self.host.random_lot() else { continue };
            let [x1, z1, x2, z2] = self.host.lot_bounding_rect(lot.token);
            let w = x2.wrapping_sub(x1).wrapping_add(1);
            let h = z2.wrapping_sub(z1).wrapping_add(1);
            let pol = police.as_ref().map_or(0, |g| g.average_in_cell_rect(x1, z1, w, h));
            let eff = effect.as_ref().map_or(0, |g| g.average_in_cell_rect(x1, z1, w, h));
            let p = curve.eval((i32::from(pol) - i32::from(eff)) as f32);
            let u = self.shared.rng.borrow_mut().uniform_f64();
            if f64::from(p) > u {
                continue; // stopped by the police
            }
            let s = self.add_crime_event(lot);
            if s == 0 {
                continue;
            }
            if let Some(g) = &crime {
                self.stamp(&**g, [x1, z1, x2, z2], s);
            }
            count += 1;
        }
        count
    }

    /// §6.6 step 7 ([`stamp_rect`]) with the simulator's hotspot.
    fn stamp(&self, g: &dyn TractGrid<u8>, rect: [i32; 4], s: u8) {
        let mut hot = {
            let d = self.d.borrow();
            (d.hotspot_x, d.hotspot_z, d.hotspot_value)
        };
        stamp_rect(g, rect, s, &mut hot);
        let mut d = self.d.borrow_mut();
        d.hotspot_x = hot.0;
        d.hotspot_z = hot.1;
        d.hotspot_value = hot.2;
    }

    /// The lot's bit sets (`GetLotBits`, §6.7).
    pub fn lot_bits(&self, lot: u64) -> LotBits {
        let zone = self
            .host
            .lot_location(lot)
            .map_or(0, |(x, z)| self.host.zone_type(x, z));
        let building = self.host.lot_building(lot);
        let condition = self.host.lot_condition(lot);
        LotBits::new(zone, building.map(|b| (b.purpose, b.wealth)), condition)
    }

    /// Type choice and `AddCrimeEvent` (§6.7, §6.8): the severity of the
    /// crime, 0 if there is no candidate (no draw then).
    pub fn add_crime_event(&self, lot: LotRef) -> u8 {
        let Some(types) = self.shared.types() else { return 0 };
        let bits = self.lot_bits(lot.token);
        let candidates: Vec<&CrimeType> = types.iter().filter(|t| t.matches(&bits)).collect();
        if candidates.is_empty() {
            return 0;
        }
        let weights: Vec<f32> = candidates.iter().map(|t| t.weight).collect();
        let total = weights.iter().fold(0.0f32, |acc, &w| acc + w);
        let r = self.shared.rng.borrow_mut().range_f64(0.0, f64::from(total));
        let i = choose_candidate(&weights, r).unwrap_or(0);
        let t = candidates[i].clone();
        self.record_incident(lot, &t)
    }

    /// `AddCrimeEvent(lot)` for a chosen type (§6.8): the incident, arson,
    /// the posted message; returns the severity.
    pub fn record_incident(&self, lot: LotRef, t: &CrimeType) -> u8 {
        self.host.add_ref(lot.token);
        {
            let mut d = self.d.borrow_mut();
            d.incident_epoch = d.incident_epoch.wrapping_add(1);
            d.incidents.push(Incident {
                lot: Some(lot),
                type_id: t.id,
                name: t.name.clone(),
                icon: t.icon,
                visuals_created: false,
            });
        }
        if t.arson {
            if let Some(b) = self.host.lot_building(lot.token) {
                if self.host.disaster_layer_present() {
                    self.host.ignite(b.token, ARSON_CAUSE);
                }
            }
        }
        self.host.post_crime_committed(t.id, lot, &t.name);
        t.severity
    }

    /// `ComputeAverageCrime` (§6.9).
    pub fn compute_average_crime(&self) {
        let grid = self.d.borrow().crime_grid.clone();
        let average = match grid {
            Some(g) => zoned_average(&*g, |x, z| self.host.zone_type(x, z)),
            // No grid: no tract, 0/0 (§6.9).
            None => f32::from_bits(DEFAULT_NAN),
        };
        self.d.borrow_mut().average_crime = average;
    }

    /// `DoOccupantChanged(occ, sign)` (§6.10).
    pub fn occupant_changed(&self, occupant: u64, sign: i32) {
        let Some(bytes) = self.host.occupant_property_bytes(occupant, prop::CRIME_EFFECT) else {
            return;
        };
        if bytes.len() < 2 {
            return; // [choice] the original reads past the data
        }
        let Some(grid) = self.d.borrow().effect_grid.clone() else {
            return;
        };
        let magnitude = f32::from(bytes[0]);
        let value = if sign >= 0 { magnitude } else { -magnitude };
        let geo = grid.geometry();
        let radius = f32::from(bytes[1]) / geo.tract_size as f32;
        let pos = self.host.occupant_position(occupant);
        let (tx, tz) = geo.position_to_tract(pos[0], pos[2]);
        add_radius_effect(&*grid, geo.count_x, geo.count_z, tx, tz, radius, value, 0.0);
    }

    // ---- cISC4CrimeSimulator (§11) ------------------------------------------------------------------

    /// `GetCrimeGrid()` (borrowed).
    pub fn crime_grid(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.d.borrow().crime_grid.clone()
    }

    /// The crime-effect grid (not exposed by the interface).
    pub fn effect_grid(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.d.borrow().effect_grid.clone()
    }

    /// `GetAverageCrime()`: `trunc(average)` (`_ftol`, so NaN gives 0).
    pub fn average_crime(&self) -> u32 {
        ftol32(f64::from(self.d.borrow().average_crime)) as u32
    }

    /// `GetMonthlyCrimeCount()`.
    pub fn monthly_crime_count(&self) -> u32 {
        self.d.borrow().monthly_crime_count
    }

    /// `GetCrimeHotspot` (§11.2): (value, S·x, S·z) with S the crime grid's
    /// tract size, or `None` (nothing written) when the value is 0 or less.
    /// [choice] Without a crime grid (the original would crash) S is the
    /// tuning tract size, the size a grid would be created with.
    pub fn crime_hotspot(&self) -> Option<(u8, i32, i32)> {
        let d = self.d.borrow();
        if d.hotspot_value <= 0 {
            return None;
        }
        let s = d
            .crime_grid
            .as_ref()
            .map_or(d.tuning.tract_size as i32, |g| g.geometry().tract_size);
        Some((
            d.hotspot_value as u8,
            s.wrapping_mul(d.hotspot_x),
            s.wrapping_mul(d.hotspot_z),
        ))
    }

    /// `GetNearestCrimeSim` / `GetNearestCrimeProp` (§11.3): the borrowed
    /// occupant, or `None`.
    pub fn nearest(&self, kind: NearestKind, pos: [f32; 3], radius: f32) -> Option<u64> {
        let (min, max) = nearest_box(pos, radius);
        let found = self.host.occupants_in_box(min, max, kind);
        nearest_of(kind, pos, radius, &found).map(|i| found[i].token)
    }

    /// `GetCrimeIncidentArray(out)` (§11.4): resizes `out` to the incident
    /// count, fills the elements of incidents that have a lot (others keep
    /// their content) and returns whether there is any incident.
    pub fn crime_incident_array(&self, out: &mut Vec<CrimeEvent>) -> bool {
        let incidents: Vec<(Option<LotRef>, u32, bool, ResKey)> = self
            .d
            .borrow()
            .incidents
            .iter()
            .map(|i| (i.lot, i.type_id, i.visuals_created, i.icon))
            .collect();
        out.resize(incidents.len(), CrimeEvent::default());
        for (i, (lot, type_id, visuals_created, icon)) in incidents.iter().enumerate() {
            if let Some(lot) = lot {
                let (cx, cz) = self.host.lot_center_cell(lot.token);
                out[i] = CrimeEvent {
                    cell_x: cx as u32,
                    cell_z: cz as u32,
                    type_id: *type_id,
                    visuals_created: *visuals_created,
                    icon: *icon,
                };
            }
        }
        !incidents.is_empty()
    }

    /// `SetVisualsCreated(i, b)` (§11.5).
    pub fn set_visuals_created(&self, i: u32, b: bool) -> bool {
        match self.d.borrow_mut().incidents.get_mut(i as usize) {
            Some(inc) => {
                inc.visuals_created = b;
                true
            }
            None => false,
        }
    }

    /// `GetVisualsCreated(i)` (§11.5).
    pub fn visuals_created(&self, i: u32) -> bool {
        self.d
            .borrow()
            .incidents
            .get(i as usize)
            .is_some_and(|inc| inc.visuals_created)
    }

    /// Class slot 15 (§1).
    pub fn simulator_type(&self) -> u32 {
        SIMULATOR_TYPE
    }

    // ---- cISC4PropRequester (§11.6) -----------------------------------------------------------------

    /// `GetRequestedShare(rect)`: rect `(x1, z1, x2, z2)`, averaged with
    /// width `x2 − x1` and height `z2 − z1` (no +1).
    pub fn requested_share(&self, x1: i32, z1: i32, x2: i32, z2: i32) -> i32 {
        let d = self.d.borrow();
        let a = d.crime_grid.as_ref().map_or(0, |g| {
            g.average_in_cell_rect(x1, z1, x2.wrapping_sub(x1), z2.wrapping_sub(z1))
        });
        let input = mul_f32(f64::from(a), f64::from(PROP_SCALE));
        rhe(d.tuning.prop_request_curve.eval(input))
    }

    /// `GetRequesterSatisfactionId()`.
    pub fn requester_satisfaction_id(&self) -> u32 {
        PROP_SATISFACTION_ID
    }

    /// `UsesCustomPropSelector()`.
    pub fn uses_custom_prop_selector(&self) -> bool {
        true
    }

    /// `SelectProp(props, cellX, cellZ)`: a prop matching an incident at the
    /// cell's lot, else a generic one, else 0.
    pub fn select_prop(&self, props: &[u32], cell_x: i32, cell_z: i32) -> u32 {
        let lot = self.host.lot_at(cell_x, cell_z).map(|l| l.token);
        let incidents: Vec<(Option<u64>, u32)> = self
            .d
            .borrow()
            .incidents
            .iter()
            .map(|i| (i.lot.map(|l| l.token), i.type_id))
            .collect();
        let mut generic = Vec::new();
        let mut matching = Vec::new();
        for &p in props {
            let id = self.host.prop_crime_incident_id(p);
            if id == 0 {
                generic.push(p);
            } else if incidents.iter().any(|&(l, t)| l == lot && t == id) {
                matching.push(p);
            }
        }
        let list = if !matching.is_empty() {
            matching
        } else if !generic.is_empty() {
            generic
        } else {
            return 0;
        };
        let n = u32::try_from(list.len()).unwrap_or(u32::MAX);
        let i = self.shared.rng.borrow_mut().uniform_u32(n) as usize;
        list[i.min(list.len() - 1)]
    }

    // ---- persistence (§12) --------------------------------------------------------------------------

    fn read_grid_ref(&self, inp: &mut dyn InStream) -> Option<Option<Rc<dyn TractGrid<u8>>>> {
        let r = inp.get_ref_as(IID_SIM_GRID)?;
        Some(if r.is_null() { None } else { self.host.resolve_grid(&r) })
    }

    fn read_record(&self, inp: &mut dyn InStream, incidents: &mut Vec<Incident>) -> Option<Record> {
        let crime_grid = self.read_grid_ref(inp)?;
        let effect_grid = self.read_grid_ref(inp)?;
        let iterator = CellIterator::read(inp)?;
        let _unused0 = inp.get_u32()?;
        let _unused1 = inp.get_u32()?;
        let average_crime = inp.get_f32()?;
        let ordinance_factor = inp.get_f32()?;
        let monthly_crime_count = inp.get_u32()?;
        let hotspot = [inp.get_i32()?, inp.get_i32()?, inp.get_i32()?];
        let count = inp.get_i32()?;
        for _ in 0..count.max(0) {
            let lot = match inp.get_ref_as(IID_LOT)? {
                RefValue::Host { token, class_id } => Some(LotRef { token, class_id }),
                _ => None,
            };
            // Owned from here on (released by the caller on failure).
            incidents.push(Incident {
                lot,
                type_id: 0,
                name: Vec::new(),
                icon: ResKey::default(),
                visuals_created: false,
            });
            let last = incidents.last_mut()?;
            last.type_id = inp.get_u32()?;
            last.name = inp.get_str()?;
            last.icon = inp.get_res_key()?;
        }
        if inp.error() != 0 {
            return None;
        }
        Some(Record {
            crime_grid,
            effect_grid,
            iterator,
            average_crime,
            ordinance_factor,
            monthly_crime_count,
            hotspot,
            incidents: std::mem::take(incidents),
        })
    }
}

// ---- the grid steps as free functions -------------------------------------------------------------
//
// The simulator's own steps call these; they are public so that a host can run
// the same arithmetic on grids it read from elsewhere (the plugin's crime
// shadow replays the original simulator's month with them).

/// §6.5 on one grid: every **non-zero** tract becomes `decay_value(v, decay)`
/// (x outer, z inner). The caller then reads the hotspot tract's new value.
pub fn decay_grid(g: &dyn TractGrid<u8>, decay: f32) {
    for x in 0..clamp_count(g.count_x()) {
        for z in 0..clamp_count(g.count_z()) {
            let v = g.get(x, z);
            if v != 0 {
                g.set(x, z, decay_value(v, decay));
            }
        }
    }
}

/// §6.6 step 7: adds `s` to every tract of the inclusive cell rectangle
/// `[x1, z1, x2, z2]` shifted right by the grid's tract shift (x outer, z
/// inner), saturating at 255, and moves the hotspot `(x, z, value)` to the
/// first strictly larger cell. [choice] Tracts outside the grid are skipped.
pub fn stamp_rect(g: &dyn TractGrid<u8>, rect: [i32; 4], s: u8, hot: &mut (i32, i32, i32)) {
    let geo = g.geometry();
    let sh = geo.safe_shift();
    let (nx, nz) = (clamp_count(geo.count_x), clamp_count(geo.count_z));
    let (tx1, tz1) = ((rect[0] >> sh).max(0), (rect[1] >> sh).max(0));
    let (tx2, tz2) = ((rect[2] >> sh).min(nx - 1), (rect[3] >> sh).min(nz - 1));
    for x in tx1..=tx2 {
        for z in tz1..=tz2 {
            let cell = (u32::from(g.get(x, z)) + u32::from(s)).min(255) as u8;
            g.set(x, z, cell);
            if i32::from(cell) > hot.2 {
                *hot = (x, z, i32::from(cell));
            }
        }
    }
}

/// §6.9 on one grid: the mean value of the tracts whose corner cell
/// `(x·S, z·S)` has a zone type ≠ 0, `S` the grid's own tract size; the x87
/// default NaN (`0xFFC00000`) when no tract is zoned.
pub fn zoned_average(g: &dyn TractGrid<u8>, zone_type: impl Fn(i32, i32) -> u32) -> f32 {
    let s = g.geometry().tract_size;
    let mut sum = 0u64;
    let mut count = 0u64;
    for x in 0..clamp_count(g.count_x()) {
        for z in 0..clamp_count(g.count_z()) {
            if zone_type(x.wrapping_mul(s), z.wrapping_mul(s)) != 0 {
                sum += u64::from(g.get(x, z));
                count += 1;
            }
        }
    }
    if count == 0 {
        // 0/0: the x87 default NaN, stored and saved as such (§6.9).
        f32::from_bits(DEFAULT_NAN)
    } else {
        (sum as f64 / count as f64) as f32
    }
}

/// The query box of §11.3: `(pos.x − r, −FLT_MAX, pos.z − r)` to
/// `(pos.x + r, +FLT_MAX, pos.z + r)`, each corner a `float32`.
pub fn nearest_box(pos: [f32; 3], radius: f32) -> ([f32; 3], [f32; 3]) {
    (
        [pos[0] - radius, -f32::MAX, pos[2] - radius],
        [pos[0] + radius, f32::MAX, pos[2] + radius],
    )
}

/// The selection of §11.3 over the occupants the box query returned, in its
/// order: the index of the candidate, or `None`. It starts with
/// `best := radius`; an occupant the kind's filter accepts wins when its
/// distance is strictly below `best` and it qualifies (the QI step).
pub fn nearest_of(kind: NearestKind, pos: [f32; 3], radius: f32, occupants: &[NearbyOccupant]) -> Option<usize> {
    let mut best = radius;
    let mut candidate = None;
    for (i, o) in occupants.iter().enumerate() {
        if !kind.filter_accepts(o) {
            continue;
        }
        // One f32 operation per step, in the order of §11.3 [compat]:
        // sqrt((dy² + dz²) + dx²).
        let [dx, dy, dz] = [0, 1, 2].map(|k| o.position[k] - pos[k]);
        let d = ((dy * dy + dz * dz) + dx * dx).sqrt();
        if d < best && kind.qualifies(o) {
            candidate = Some(i);
            best = d;
        }
    }
    candidate
}

impl SaveObject for CrimeSim {
    fn class_id(&self) -> u32 {
        CLSID_CRIME_SIMULATOR
    }

    /// The v6 record (§12); returns `!GetError()`.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        if out.error() != 0 {
            return false;
        }
        let d = self.d.borrow();
        let grid_ref = |g: &Option<Rc<dyn TractGrid<u8>>>| g.as_ref().map_or(RefValue::Null, |g| g.save_ref());
        out.put_u16(CRIME_RECORD_VERSION);
        out.put_ref(&grid_ref(&d.crime_grid));
        out.put_ref(&grid_ref(&d.effect_grid));
        d.iterator.write(out);
        out.put_u32(0);
        out.put_u32(0);
        out.put_f32(d.average_crime);
        out.put_f32(d.ordinance_factor);
        out.put_u32(d.monthly_crime_count);
        out.put_i32(d.hotspot_x);
        out.put_i32(d.hotspot_z);
        out.put_i32(d.hotspot_value);
        out.put_i32(i32::try_from(d.incidents.len()).unwrap_or(i32::MAX));
        for i in &d.incidents {
            let lot = i.lot.map_or(RefValue::Null, |l| RefValue::Host {
                token: l.token,
                class_id: l.class_id,
            });
            out.put_ref(&lot);
            out.put_u32(i.type_id);
            out.put_str(&i.name);
            out.put_res_key(i.icon);
        }
        out.error() == 0
    }

    /// Rejects versions other than 6 after reading only the version;
    /// otherwise reads the record, **appends** its incidents to the current
    /// list, clears every incident's visuals flag, sets `loadedFromSave` and
    /// returns true (§12). [choice] On a stream error it returns false and
    /// changes nothing: `loadedFromSave` stays clear and the lots read are
    /// released, so the next `Init` sets the city up as new. (CQ9: the
    /// original reads on, sets `loadedFromSave` and is left unusable; §12
    /// keeps this choice.)
    fn read(&self, inp: &mut dyn InStream) -> bool {
        let Some(version) = inp.get_u16() else { return false };
        if version != CRIME_RECORD_VERSION {
            return false;
        }
        let mut partial = Vec::new();
        let record = self.read_record(inp, &mut partial);
        for i in partial {
            if let Some(lot) = i.lot {
                self.host.release(lot.token);
            }
        }
        let Some(r) = record else { return false };
        let mut d = self.d.borrow_mut();
        d.crime_grid = r.crime_grid;
        d.effect_grid = r.effect_grid;
        d.iterator = r.iterator;
        d.average_crime = r.average_crime;
        d.ordinance_factor = r.ordinance_factor;
        d.monthly_crime_count = r.monthly_crime_count;
        d.hotspot_x = r.hotspot[0];
        d.hotspot_z = r.hotspot[1];
        d.hotspot_value = r.hotspot[2];
        d.incidents.extend(r.incidents);
        for i in d.incidents.iter_mut() {
            i.visuals_created = false;
        }
        d.incident_epoch = d.incident_epoch.wrapping_add(1);
        d.loaded_from_save = true;
        true
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crime::{
        shipped_tuning, shipped_type_exemplars, BuildingInfo, CrimeRandom, DemandValues, NearbyOccupant,
        CRIME_PROP_TYPE, CRIME_SIM_TYPE,
    };
    use crate::grid::{CityDims, GridProvider, SimGrid};
    use crate::property::PropertyHolder;
    use crate::stream::{ByteReader, ByteWriter, RefResolver, SaveRegistry};
    use std::collections::{BTreeMap, VecDeque};

    const LOT_CLASS: u32 = 0x1234_5678;

    #[derive(Clone, Default)]
    struct Lot {
        rect: [i32; 4],
        condition: u8,
        building: Option<BuildingInfo>,
    }

    /// (preview, crime-effect bytes, position).
    type OccupantFacts = (bool, Vec<u8>, [f32; 3]);

    #[derive(Default)]
    struct Fake {
        log: Rc<RefCell<Vec<String>>>,
        grids: RefCell<Vec<Rc<SimGrid<u8>>>>,
        tuning: RefCell<Option<Rc<PropertyHolder>>>,
        pops: RefCell<Vec<Rc<SimGrid<u16>>>>,
        eqs: RefCell<Vec<Rc<SimGrid<f32>>>>,
        demands: RefCell<BTreeMap<u32, DemandValues>>,
        no_ordinance: Cell<bool>,
        police: RefCell<Option<Rc<SimGrid<u8>>>>,
        released: Cell<i32>,
        lots: RefCell<BTreeMap<u64, Lot>>,
        random: RefCell<VecDeque<u64>>,
        random_calls: Cell<u64>,
        zones: RefCell<BTreeMap<(i32, i32), u32>>,
        occupants: RefCell<BTreeMap<u64, OccupantFacts>>,
        nearby: RefCell<Vec<NearbyOccupant>>,
        prop_ids: RefCell<BTreeMap<u32, u32>>,
    }

    fn city() -> CityDims {
        CityDims::square(32)
    }

    impl Fake {
        fn new() -> Rc<Fake> {
            let f = Fake::default();
            *f.tuning.borrow_mut() = Some(Rc::new(shipped_tuning()));
            Rc::new(f)
        }
        fn push(&self, s: String) {
            self.log.borrow_mut().push(s);
        }
        fn take(&self) -> Vec<String> {
            std::mem::take(&mut *self.log.borrow_mut())
        }
        fn count(&self, prefix: &str) -> usize {
            self.log.borrow().iter().filter(|s| s.starts_with(prefix)).count()
        }
        fn lot(&self, id: u64, lot: Lot) {
            self.lots.borrow_mut().insert(id, lot);
        }
    }

    impl GridProvider<u8> for Fake {
        fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
            self.push(format!("create {instance:08X} {tract_size}"));
            let g = SimGrid::<u8>::create(&city(), instance, tract_size);
            g.fill(7); // Init must clear it
            self.grids.borrow_mut().push(g.clone());
            Some(g)
        }
    }

    impl CrimeHost for Fake {
        fn add_agent(&self, name: &str, levels: &[u32], flags: u32) {
            self.push(format!("agent {name} {levels:?} {flags}"));
        }
        fn remove_agent(&self) {
            self.push("remove_agent".into());
        }
        fn subscribe(&self, t: &[u32]) {
            self.push(format!("subscribe {t:X?}"));
        }
        fn unsubscribe(&self, t: &[u32]) {
            self.push(format!("unsubscribe {t:X?}"));
        }
        fn acquire_services(&self) {
            self.push("acquire".into());
        }
        fn release_services(&self) {
            self.push("release_services".into());
        }
        fn add_prop_requester(&self) {
            self.push("prop_requester".into());
        }
        fn shutdown_grid(&self, _g: &Rc<dyn TractGrid<u8>>) {
            self.push("grid_shutdown".into());
        }
        fn tuning(&self) -> Option<Rc<PropertyHolder>> {
            self.tuning.borrow().clone()
        }
        fn crime_type_exemplars(&self) -> Vec<(u32, Rc<PropertyHolder>)> {
            self.push("types".into());
            shipped_type_exemplars()
                .into_iter()
                .map(|(i, h)| (i, Rc::new(h)))
                .collect()
        }
        fn ltext(&self, _k: ResKey) -> crate::crime::LtextLoad {
            crate::crime::LtextLoad::Missing
        }
        fn population_grids(&self) -> [Option<Rc<dyn TractGrid<u16>>>; 3] {
            self.push("population_grids".into());
            let p = self.pops.borrow();
            [0, 1, 2].map(|i| p.get(i).map(|g| g.clone() as Rc<dyn TractGrid<u16>>))
        }
        fn eq_grids(&self) -> [Option<Rc<dyn TractGrid<f32>>>; 3] {
            self.push("eq_grids".into());
            let p = self.eqs.borrow();
            [0, 1, 2].map(|i| p.get(i).map(|g| g.clone() as Rc<dyn TractGrid<f32>>))
        }
        fn demand(&self, id: u32) -> Option<DemandValues> {
            self.demands.borrow().get(&id).copied()
        }
        fn police_grid(&self) -> Option<Rc<dyn TractGrid<u8>>> {
            self.police.borrow().clone().map(|g| g as Rc<dyn TractGrid<u8>>)
        }
        fn jail_population(&self) -> u32 {
            self.push("jail".into());
            0
        }
        fn reset_unjailed_criminals(&self) -> i32 {
            self.push("reset_unjailed".into());
            self.released.get()
        }
        fn process_monthly_crime_stats(&self) {
            self.push("stats".into());
        }
        fn ordinance_crime_factor(&self) -> Option<f32> {
            (!self.no_ordinance.get()).then_some(0.5)
        }
        fn random_lot(&self) -> Option<LotRef> {
            self.random_calls.set(self.random_calls.get() + 1);
            let t = self.random.borrow_mut().pop_front()?;
            self.lots.borrow().contains_key(&t).then_some(LotRef {
                token: t,
                class_id: LOT_CLASS,
            })
        }
        fn lot_at(&self, x: i32, z: i32) -> Option<LotRef> {
            self.lots
                .borrow()
                .iter()
                .find(|(_, l)| (l.rect[0]..=l.rect[2]).contains(&x) && (l.rect[1]..=l.rect[3]).contains(&z))
                .map(|(t, _)| LotRef {
                    token: *t,
                    class_id: LOT_CLASS,
                })
        }
        fn lot_bounding_rect(&self, lot: u64) -> [i32; 4] {
            self.lots.borrow().get(&lot).map_or([0; 4], |l| l.rect)
        }
        fn lot_location(&self, lot: u64) -> Option<(i32, i32)> {
            self.lots.borrow().get(&lot).map(|l| (l.rect[0], l.rect[1]))
        }
        fn lot_condition(&self, lot: u64) -> u8 {
            self.lots.borrow().get(&lot).map_or(0, |l| l.condition)
        }
        fn lot_building(&self, lot: u64) -> Option<BuildingInfo> {
            self.lots.borrow().get(&lot).and_then(|l| l.building)
        }
        fn lot_center_cell(&self, lot: u64) -> (i32, i32) {
            self.lots.borrow().get(&lot).map_or((0, 0), |l| (l.rect[2], l.rect[3]))
        }
        fn zone_type(&self, x: i32, z: i32) -> u32 {
            self.zones.borrow().get(&(x, z)).copied().unwrap_or(0)
        }
        fn add_ref(&self, t: u64) {
            self.push(format!("add_ref {t}"));
        }
        fn release(&self, t: u64) {
            self.push(format!("release {t}"));
        }
        fn occupant_has_property(&self, o: u64, id: u32) -> bool {
            id == prop::PREVIEW_OCCUPANT && self.occupants.borrow().get(&o).is_some_and(|x| x.0)
        }
        fn occupant_property_bytes(&self, o: u64, id: u32) -> Option<Vec<u8>> {
            if id != prop::CRIME_EFFECT {
                return None;
            }
            self.occupants.borrow().get(&o).map(|x| x.1.clone())
        }
        fn occupant_position(&self, o: u64) -> [f32; 3] {
            self.occupants.borrow().get(&o).map_or([0.0; 3], |x| x.2)
        }
        fn occupants_in_box(&self, _min: [f32; 3], _max: [f32; 3], _k: NearestKind) -> Vec<NearbyOccupant> {
            self.nearby.borrow().clone()
        }
        fn prop_crime_incident_id(&self, p: u32) -> u32 {
            self.prop_ids.borrow().get(&p).copied().unwrap_or(0)
        }
        fn disaster_layer_present(&self) -> bool {
            true
        }
        fn ignite(&self, b: u64, cause: u32) {
            self.push(format!("ignite {b} {cause}"));
        }
        fn post_crime_committed(&self, type_id: u32, lot: LotRef, _name: &[u8]) {
            self.push(format!("committed {type_id:X} {}", lot.token));
        }
        fn post_crime_map_updated(&self) {
            self.push("map_updated".into());
        }
    }

    /// Scripted draws: each method pops the next `u` (`range` maps it to
    /// `a + u(b − a)`, `uint` to `floor(u n)`) and logs the call.
    struct Script(VecDeque<f64>, Rc<RefCell<Vec<String>>>);

    impl Script {
        fn next(&mut self, kind: &str) -> f64 {
            self.1.borrow_mut().push(format!("rng {kind}"));
            self.0.pop_front().unwrap_or(0.5)
        }
    }

    impl CrimeRandom for Script {
        fn uniform_f64(&mut self) -> f64 {
            self.next("uniform")
        }
        fn range_f64(&mut self, a: f64, b: f64) -> f64 {
            a + self.next("range") * (b - a)
        }
        fn uniform_u32(&mut self, n: u32) -> u32 {
            (self.next("uint") * f64::from(n)) as u32
        }
    }

    fn setup(draws: &[f64]) -> (Rc<Fake>, Rc<CrimeSim>) {
        let f = Fake::new();
        let shared = CrimeShared::new(Box::new(Script(draws.iter().copied().collect(), f.log.clone())));
        let sim = CrimeSim::new(f.clone(), 1, shared);
        (f, sim)
    }

    fn msg(t: u32) -> CrimeMessage {
        CrimeMessage {
            msg_type: t,
            ..Default::default()
        }
    }

    fn lot_ref(token: u64) -> LotRef {
        LotRef {
            token,
            class_id: LOT_CLASS,
        }
    }

    fn building(token: u64, purpose: i32, wealth: i32) -> Option<BuildingInfo> {
        Some(BuildingInfo { token, purpose, wealth })
    }

    /// Test 18.
    #[test]
    fn lifecycle() {
        let (f, sim) = setup(&[]);
        assert!(sim.init());
        assert!(sim.init());
        assert_eq!(
            f.take(),
            vec![
                "types".to_string(),
                "agent cSC4CrimeSimulator [4, 6, 10] 2".into(),
                "subscribe [99EF1142, 99EF1143, C2C09554, C53D10AA]".into(),
                "acquire".into(),
                "create 89D5B895 2".into(),
                "create 2A5BA1B3 2".into(),
                "prop_requester".into(),
            ]
        );
        let g = sim.crime_grid().unwrap();
        assert_eq!((g.get(0, 0), g.count_x()), (0, 16), "cleared; 32 cells / 2");
        let it = sim.data().iterator.clone();
        assert_eq!(
            (it.x0, it.z0, it.x1, it.z1, it.step, it.dirty, it.mode, it.state_raw),
            (0, 0, 15, 15, 1, 1, 1, 1)
        );
        assert!(sim.shutdown());
        assert!(sim.shutdown());
        assert_eq!(
            f.take(),
            vec![
                "grid_shutdown".to_string(),
                "grid_shutdown".into(),
                "release_services".into(),
                "unsubscribe [C2C09554, C53D10AA, 99EF1142, 99EF1143]".into(),
                "remove_agent".into(),
            ]
        );
        assert!(sim.crime_grid().is_none());
        // The type list is process-wide: a second simulator does not reload it.
        let sim2 = CrimeSim::new(f.clone(), 2, sim.shared().clone());
        sim2.init();
        assert_eq!(f.count("types"), 0);
        sim2.handle_message(CrimeMessage {
            msg_type: msg::EXEMPLAR_CHANGED,
            data1: CRIME_TYPE_GROUP,
            ..Default::default()
        });
        assert_eq!(f.count("types"), 1);
    }

    /// Test 13.
    #[test]
    fn pipeline() {
        let (f, sim) = setup(&[]);
        sim.init();
        f.take();
        for _ in 0..10 {
            assert!(sim.handle_message(msg(msg::SIM_IDLE)));
        }
        assert_eq!(sim.data().state, STATE_IDLE);
        assert!(f.take().is_empty());
        sim.handle_message(msg(msg::SIM_NEW_MONTH));
        let mut states = Vec::new();
        for i in 0..7 {
            if i == 2 {
                sim.handle_message(msg(msg::SIM_NEW_MONTH)); // ignored mid-run
            }
            sim.handle_message(msg(if i % 2 == 0 { msg::SIM_NEW_DAY } else { msg::SIM_IDLE }));
            states.push(sim.data().state);
        }
        assert_eq!(states, vec![1, 2, 3, 4, 5, 6, 6]);
        // §13 (CQ15): the census asks GetPopulationGrids (slot 25) first, then
        // GetEQGrids (slot 23); their return values are not used.
        assert_eq!(
            f.take(),
            vec![
                "population_grids",
                "eq_grids",
                "jail",
                "reset_unjailed",
                "stats",
                "map_updated"
            ]
        );
    }

    /// Tests 8, 14 and 15.
    #[test]
    fn commit() {
        let (f, sim) = setup(&[0.9, 0.5, 0.0, 0.9, 0.0]);
        sim.init();
        f.lot(
            1,
            Lot {
                rect: [10, 10, 13, 11],
                condition: 1,
                building: building(11, 1, 1),
            },
        );
        f.lot(
            3,
            Lot {
                rect: [0, 0, 0, 0],
                condition: 4,
                building: building(33, 1, 1),
            },
        );
        f.zones.borrow_mut().insert((0, 0), 1);
        let police = SimGrid::<u8>::create(&city(), 0, 2);
        police.set(5, 5, 150);
        police.set(6, 5, 150);
        *f.police.borrow_mut() = Some(police);
        let g = sim.crime_grid().unwrap();
        g.set(6, 5, 200);
        *f.random.borrow_mut() = [1, 1, 99, 3].into_iter().collect();
        f.take();
        // Lot 1: p = 0.125, u 0.9 commits; r = 0.5 · 20 → Flashing (40).
        // Lot 1: u 0.0 < 0.125, stopped. Lot 99: none. Lot 3: p 0, u 0.9;
        // r = 0 → Fighting (60). Fifth attempt: no lot.
        assert_eq!(sim.add_crimes(5), 2);
        let log = f.take();
        let draws: Vec<&String> = log.iter().filter(|s| s.starts_with("rng")).collect();
        assert_eq!(
            draws,
            vec!["rng uniform", "rng range", "rng uniform", "rng uniform", "rng range"]
        );
        assert_eq!((g.get(5, 5), g.get(6, 5), g.get(7, 5), g.get(0, 0)), (40, 240, 0, 60));
        assert_eq!(sim.crime_hotspot(), Some((240, 12, 10)));
        assert!(log.contains(&"committed 10 1".to_string()));
        assert!(log.contains(&"committed 3 3".to_string()));
        let mut out = vec![CrimeEvent::default(); 5];
        assert!(sim.crime_incident_array(&mut out));
        assert_eq!(out.len(), 2);
        assert_eq!((out[0].cell_x, out[0].cell_z, out[0].type_id), (13, 11, 0x10));
        assert!(sim.set_visuals_created(1, true));
        assert!(sim.visuals_created(1) && !sim.visuals_created(0) && !sim.visuals_created(2));
        assert!(!sim.set_visuals_created(2, true));
        assert_eq!(sim.add_crimes(0), 0);
        assert_eq!(sim.add_crimes(-3), 0);
    }

    /// §6.6 runs all n attempts (no cap): 1,500,000 attempts ask for
    /// 1,500,000 random lots (none found, so no draws).
    #[test]
    fn add_crimes_runs_every_attempt() {
        let (f, sim) = setup(&[]);
        sim.init();
        f.random_calls.set(0);
        assert_eq!(sim.add_crimes(1_500_000), 0);
        assert_eq!(f.random_calls.get(), 1_500_000);
    }

    #[test]
    fn arson() {
        let (f, sim) = setup(&[]);
        sim.init();
        f.lot(
            3,
            Lot {
                rect: [0, 0, 0, 0],
                condition: 4,
                building: building(33, 9, 1),
            },
        );
        let types = sim.shared().types().unwrap();
        let arson = types.iter().find(|t| t.arson).unwrap().clone();
        f.take();
        assert_eq!(sim.record_incident(lot_ref(3), &arson), 100);
        assert_eq!(f.take(), vec!["add_ref 3", "ignite 33 2", "committed 5 3"]);
        sim.shutdown();
        assert!(f.take().contains(&"release 3".to_string()));
    }

    /// Test 11.
    #[test]
    fn crime_effect() {
        let (f, sim) = setup(&[]);
        sim.init();
        // Tract (8, 8) of S = 2 (32 world units per tract).
        for (o, preview, bytes) in [(5, false, vec![100, 8]), (6, true, vec![100, 8]), (7, false, vec![100])] {
            f.occupants
                .borrow_mut()
                .insert(o, (preview, bytes, [257.0, 0.0, 257.0]));
        }
        let ins = |o, t| CrimeMessage {
            msg_type: t,
            void1: o,
            ..Default::default()
        };
        for o in [5, 6, 7] {
            sim.handle_message(ins(o, msg::OCCUPANT_INSERTED));
        }
        let g = sim.effect_grid().unwrap();
        let v = |x, z| g.get(x, z);
        assert_eq!(
            [v(8, 8), v(9, 8), v(10, 8), v(11, 8), v(9, 9), v(12, 8)],
            [100, 75, 50, 25, 65, 0]
        );
        g.set(8, 8, 30);
        sim.handle_message(ins(5, msg::OCCUPANT_REMOVED));
        assert_eq!([v(8, 8), v(9, 8), v(9, 9)], [0, 0, 0]);
    }

    /// Tests 1 (the hotspot follows the decay) and 12.
    #[test]
    fn decay_and_average() {
        let (f, sim) = setup(&[]);
        sim.init();
        let g = sim.crime_grid().unwrap();
        g.set(0, 0, 10);
        g.set(1, 0, 20);
        for x in [0, 2, 4] {
            f.zones.borrow_mut().insert((x, 0), 1);
        }
        sim.compute_average_crime();
        assert_eq!(sim.average_crime(), 10);
        g.set(0, 0, 1);
        g.set(1, 0, 2);
        f.zones.borrow_mut().remove(&(4, 0));
        sim.compute_average_crime();
        assert_eq!((sim.data().average_crime, sim.average_crime()), (1.5, 1));
        f.zones.borrow_mut().clear();
        sim.compute_average_crime();
        assert_eq!(sim.average_crime(), 0, "unzoned: NaN, read back through _ftol as 0");
        assert_eq!(sim.data().average_crime.to_bits(), DEFAULT_NAN);
        // A tuning reload to S = 4 keeps the grid's S = 2 for §6.9 and §11.2.
        sim.data_mut().tuning.tract_size = 4;
        f.zones.borrow_mut().insert((2, 0), 1);
        sim.compute_average_crime();
        assert_eq!(sim.data().average_crime, 2.0, "tract (1, 0) sampled at cell (2, 0)");
        {
            let mut d = sim.data_mut();
            d.hotspot_x = 3;
            d.hotspot_z = 1;
            d.hotspot_value = 9;
        }
        assert_eq!(sim.crime_hotspot(), Some((9, 6, 2)));
        f.zones.borrow_mut().clear();
        g.set(3, 3, 200);
        {
            let mut d = sim.data_mut();
            d.hotspot_x = 3;
            d.hotspot_z = 3;
            d.hotspot_value = 200;
        }
        sim.decay_crime_grid();
        assert_eq!((g.get(3, 3), sim.data().hotspot_value), (160, 160));
        sim.data_mut().hotspot_value = 0;
        assert_eq!(sim.crime_hotspot(), None);
    }

    /// The free grid steps on a plain grid (tests 1, 8, 12) and the incident
    /// epoch.
    #[test]
    fn free_grid_steps_and_epoch() {
        let g = SimGrid::<u8>::create(&city(), 0, 2);
        g.set(0, 0, 200);
        g.set(1, 0, 255);
        g.set(2, 0, 1);
        decay_grid(&*g, 0.8);
        assert_eq!((g.get(0, 0), g.get(1, 0), g.get(2, 0), g.get(3, 0)), (160, 204, 0, 0));
        // Test 8: cells (10, 10)-(13, 11) → tracts x 5..6, z 5.
        g.set(6, 5, 200);
        let mut hot = (0, 0, 0);
        stamp_rect(&*g, [10, 10, 13, 11], 60, &mut hot);
        assert_eq!((g.get(5, 5), g.get(6, 5), hot), (60, 255, (6, 5, 255)));
        // Strictly larger only: an equal cell does not move the hotspot.
        stamp_rect(&*g, [0, 0, 1, 1], 95, &mut hot);
        assert_eq!((g.get(0, 0), hot), (255, (6, 5, 255)));
        // Off-grid parts are skipped.
        stamp_rect(&*g, [-4, -4, 0, 0], 1, &mut hot);
        assert_eq!(g.get(0, 0), 255);
        let zoned = |x: i32, z: i32| u32::from(z == 0 && x <= 2);
        assert_eq!(zoned_average(&*g, zoned), (255.0 + 204.0) / 2.0);
        assert_eq!(zoned_average(&*g, |_, _| 0).to_bits(), DEFAULT_NAN);
        let o = |x: f32, ok: bool| NearbyOccupant {
            occupant_type: CRIME_SIM_TYPE,
            in_crime_group: true,
            position: [x, 0.0, 0.0],
            answers_iid: ok,
            ..Default::default()
        };
        let found = [o(5.0, true), o(3.0, false), o(4.0, true), o(4.0, true)];
        assert_eq!(
            nearest_of(NearestKind::Sim, [0.0; 3], 10.0, &found),
            Some(2),
            "the first at 4"
        );
        assert_eq!(nearest_of(NearestKind::Prop, [0.0; 3], 10.0, &found), None);
        assert_eq!(
            nearest_box([1.0, 2.0, 3.0], 0.5),
            ([0.5, -f32::MAX, 2.5], [1.5, f32::MAX, 3.5])
        );

        let (f, sim) = setup(&[0.0]);
        sim.init();
        let e0 = sim.data().incident_epoch;
        f.lot(1, Lot::default());
        let t = sim.shared().types().unwrap()[0].clone();
        sim.record_incident(lot_ref(1), &t);
        let e1 = sim.data().incident_epoch;
        assert_ne!(e0, e1);
        sim.set_visuals_created(0, true);
        assert_eq!(sim.data().incident_epoch, e1, "visuals flags do not bump it");
        sim.shutdown();
        assert_ne!(sim.data().incident_epoch, e1, "cleared");
    }

    /// Census, factors, base and crimes through the host (tests 2–6).
    #[test]
    fn monthly_numbers() {
        let (f, sim) = setup(&[]);
        sim.init();
        let pop = SimGrid::<u16>::create(&city(), 0, 2);
        let eq = SimGrid::<f32>::create(&city(), 0, 2);
        // EQ 49.99 → bucket 0; 50 → 1 (24-bit product, foundations §18); 250 → none.
        for (x, p, e) in [(0, 1000, 49.99f32), (1, 234, 50.0), (2, 99, 250.0)] {
            pop.set(x, 0, p);
            eq.set(x, 0, e);
        }
        f.pops.borrow_mut().push(pop);
        f.eqs.borrow_mut().push(eq);
        f.demands.borrow_mut().insert(
            0x4100,
            DemandValues {
                value: -1000.0,
                max: 1000.0,
                min: -1000.0,
            },
        );
        sim.do_census();
        sim.calculate_economic_factors();
        assert_eq!(sim.data().census[0], [1000, 234, 0, 0]);
        assert_eq!(sim.data().jobless, [-1.0, 0.0, 0.0]);
        sim.compute_criminal_base();
        // 0.4 · 1000 + 0.375 · 234 + 0.1 · 1234 = 611.15
        assert_eq!(sim.data().criminal_base, 611);
        f.released.set(10);
        sim.data_mut().criminal_base = 7500;
        sim.compute_crimes_to_commit();
        assert_eq!(sim.data().crimes_to_commit, 13);
        sim.handle_message(msg(msg::ORDINANCE_UPDATED));
        sim.compute_crimes_to_commit();
        assert_eq!(sim.data().crimes_to_commit, 6, "13 × 0.5 = 6.5 → 6");
    }

    /// §14.1 (corrected 2026-10-14): the patrol car and the helicopter call
    /// `GetNearestCrimeProp` with their position and radius 20, set the prop
    /// found to state 1 and query again, at most five times. A prop in state
    /// 1 no longer qualifies (§11.3), so each query returns the next nearest
    /// one; nothing in the selection keeps state between queries.
    #[test]
    fn nearest_prop_patrol_loop() {
        let prop = |token, x: f32| NearbyOccupant {
            token,
            occupant_type: CRIME_PROP_TYPE,
            has_incident_property: true,
            position: [x, 0.0, 0.0],
            answers_iid: true,
            ..Default::default()
        };
        // Seven props in the box, out of distance order; the one at 20 is
        // not strictly inside the radius.
        let mut props = [7.0, 3.0, 20.0, 1.0, 5.0, 2.0, 4.0].map(|x| prop(x as u64, x)).to_vec();
        let mut set_to_state_1 = Vec::new();
        for _ in 0..5 {
            let Some(i) = nearest_of(NearestKind::Prop, [0.0; 3], 20.0, &props) else {
                break;
            };
            props[i].prop_state = 1; // the vehicle's SetState(1), prop slot 11
            set_to_state_1.push(props[i].token);
        }
        assert_eq!(set_to_state_1, [1, 2, 3, 4, 5]);
        // The sixth query would find the prop at 7; the one at 20 never wins.
        assert_eq!(nearest_of(NearestKind::Prop, [0.0; 3], 20.0, &props), Some(0));
        props[0].prop_state = 1;
        assert_eq!(nearest_of(NearestKind::Prop, [0.0; 3], 20.0, &props), None);
    }

    /// Test 16 and §11.3.
    #[test]
    fn props_and_nearest() {
        let (f, sim) = setup(&[0.0, 0.0]);
        sim.init();
        let g = sim.crime_grid().unwrap();
        g.fill(51);
        assert_eq!(sim.requested_share(0, 0, 8, 8), 4);
        g.fill(255);
        assert_eq!(sim.requested_share(0, 0, 8, 8), 10);
        g.fill(0);
        assert_eq!(sim.requested_share(0, 0, 8, 8), 0);
        f.lot(
            1,
            Lot {
                rect: [10, 10, 11, 11],
                ..Default::default()
            },
        );
        f.prop_ids.borrow_mut().insert(0xB, 0x11);
        let graffiti = sim
            .shared()
            .types()
            .unwrap()
            .iter()
            .find(|t| t.id == 0x11)
            .unwrap()
            .clone();
        sim.record_incident(lot_ref(1), &graffiti);
        assert_eq!(sim.select_prop(&[0xA, 0xB], 10, 10), 0xB);
        assert_eq!(sim.select_prop(&[0xA, 0xB], 20, 20), 0xA);
        assert_eq!(sim.select_prop(&[], 10, 10), 0);
        assert_eq!(
            (
                sim.requester_satisfaction_id(),
                sim.uses_custom_prop_selector(),
                sim.simulator_type()
            ),
            (0x49A1_E996, true, 0x2756_7222)
        );
        let occ = |token, d: f32, ok| NearbyOccupant {
            token,
            occupant_type: CRIME_SIM_TYPE,
            in_crime_group: true,
            position: [d, 0.0, 0.0],
            answers_iid: ok,
            ..Default::default()
        };
        *f.nearby.borrow_mut() = vec![
            occ(1, 5.0, true),
            occ(2, 3.0, false),
            occ(3, 4.0, true),
            occ(4, 1.0, true),
        ];
        f.nearby.borrow_mut()[3].in_crime_group = false;
        assert_eq!(sim.nearest(NearestKind::Sim, [0.0; 3], 10.0), Some(3));
        assert_eq!(sim.nearest(NearestKind::Sim, [0.0; 3], 4.0), None);
        assert_eq!(sim.nearest(NearestKind::Prop, [0.0; 3], 10.0), None);
        // Test 22: the f32 order sqrt((dy² + dz²) + dx²) gives 0x459075B3.
        *f.nearby.borrow_mut() = vec![NearbyOccupant {
            position: [2109.0, 1812.0, 3693.0],
            ..occ(9, 0.0, true)
        }];
        assert_eq!(
            sim.nearest(NearestKind::Sim, [0.0; 3], f32::from_bits(0x4590_75B4)),
            Some(9)
        );
        assert_eq!(
            sim.nearest(NearestKind::Sim, [0.0; 3], f32::from_bits(0x4590_75B3)),
            None
        );
        *f.nearby.borrow_mut() = vec![NearbyOccupant {
            position: [3.0, 4.0, 0.0],
            ..occ(8, 0.0, true)
        }];
        assert_eq!(sim.nearest(NearestKind::Sim, [0.0; 3], 5.0), None, "strict");
    }

    struct Resolver(BTreeMap<(u32, u32), RefValue>);

    impl RefResolver for Resolver {
        fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
            self.0.get(&(class_id, object_id)).cloned()
        }
    }

    fn write(s: &CrimeSim) -> Vec<u8> {
        let mut w = ByteWriter::with_registry(Rc::new(RefCell::new(SaveRegistry::new())));
        assert!(s.write(&mut w));
        w.into_bytes()
    }

    /// Test 17.
    #[test]
    fn save_round_trip() {
        let (f, sim) = setup(&[]);
        sim.init();
        let t = sim.shared().types().unwrap()[1].clone();
        sim.record_incident(lot_ref(9), &t);
        sim.data_mut().incidents.push(Incident {
            lot: None,
            type_id: 3,
            name: b"x".to_vec(),
            icon: ResKey::default(),
            visuals_created: true,
        });
        {
            let mut d = sim.data_mut();
            d.hotspot_value = 9;
            d.ordinance_factor = 0.5;
            d.average_crime = 2.5;
            d.monthly_crime_count = 4;
        }
        let bytes = write(&sim);
        assert_eq!(&bytes[..2], &[6, 0]);
        let grids = f.grids.borrow().clone();
        let mut map = BTreeMap::new();
        map.insert((0x49B9_E602, 1), RefValue::Object(grids[0].clone()));
        map.insert((0x49B9_E602, 2), RefValue::Object(grids[1].clone()));
        map.insert(
            (LOT_CLASS, 3),
            RefValue::Host {
                token: 9,
                class_id: LOT_CLASS,
            },
        );
        let (f2, loaded) = setup(&[]);
        let mut res = Resolver(map.clone());
        assert!(loaded.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
        assert_eq!(write(&loaded), bytes);
        assert!(loaded.data().incidents.iter().all(|i| !i.visuals_created));
        assert!(loaded.data().loaded_from_save);
        loaded.init();
        assert_eq!(f2.count("prop_requester") + f2.count("create"), 0);
        {
            let d = loaded.data();
            assert_eq!((d.hotspot_value, d.ordinance_factor, d.state), (0, 1.0, STATE_IDLE));
        }
        // CQ6: OrdinanceUpdated without an ordinance simulator leaves the 1.0.
        f2.no_ordinance.set(true);
        loaded.handle_message(msg(msg::ORDINANCE_UPDATED));
        assert_eq!(loaded.data().ordinance_factor, 1.0);
        f2.no_ordinance.set(false);
        // A second Read appends its incidents and clears every visuals flag.
        loaded.data_mut().incidents[0].visuals_created = true;
        let mut res = Resolver(map.clone());
        assert!(loaded.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
        assert_eq!(loaded.data().incidents.len(), 4);
        assert!(loaded.data().incidents.iter().all(|i| !i.visuals_created));
        // Version 5 / 7: false after the version only.
        for v in [5u8, 7] {
            let mut b = bytes.clone();
            b[0] = v;
            let (_, s) = setup(&[]);
            let mut r = ByteReader::new(&b);
            assert!(!s.read(&mut r));
            assert_eq!(r.position(), 2);
            assert!(!s.data().loaded_from_save);
        }
        // Truncations never panic, keep nothing and release the lots read.
        for n in 0..bytes.len() {
            let (f3, s) = setup(&[]);
            let mut res = Resolver(map.clone());
            assert!(!s.read(&mut ByteReader::with_resolver(&bytes[..n], &mut res)));
            assert!(s.data().incidents.is_empty() && s.data().crime_grid.is_none());
            assert!(!s.data().loaded_from_save, "[choice] a failed Read sets nothing");
            assert!(f3.count("release") <= 1);
        }
        // A hostile incident count (offset 89).
        let mut b = bytes.clone();
        b[89..93].copy_from_slice(&i32::MAX.to_le_bytes());
        let (_, s) = setup(&[]);
        let mut res = Resolver(map);
        assert!(!s.read(&mut ByteReader::with_resolver(&b, &mut res)));
        // Without a registry the references cannot be written.
        let mut w = ByteWriter::new();
        assert!(!sim.write(&mut w));
    }
}
