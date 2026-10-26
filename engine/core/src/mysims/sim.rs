// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4MySim` (§2.2, §3.3, §4, §5, §6.2, §7.2): an advisor extension.

use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::advisors::{AdviceSubject, Advisor, AdvisorExt, AdvisorMessage, Target};
use crate::rng::{derive_seed, FixedSeeds, RzRandom};
use crate::stream::{InStream, OutStream, RefValue, ResKey};
use crate::x87::{ftol, r24};

use super::{
    command, disaster, dispatch_note, distance, event, iid, msg, Field, JobDestination, Lot, MySimHost, MySimPost,
    DESIRABILITY_DEVELOPERS, DIED_TRIGGER, GROUP_LANDMARK, LOCAL_BOX, NO_DISTANCE, RECORD_VERSION, TUNING,
};
use super::{Grid, MY_STREET_ID};

/// One MySim's fields (§2.2).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MySimState {
    /// 0 none, 1 moving in / going home, 2 at home, 3 at work, 4–6 trips.
    pub state: u8,
    pub slot: u8,
    pub residence: Option<u64>,
    pub home_cell: (u32, u32),
    pub job: Option<u64>,
    /// A neighbour job (the neighbour city, not saved).
    pub neighbor_job: bool,
    pub neighbor_cell: (i32, i32),
    pub commute_cell: (i32, i32),
    pub jobless_days: u32,
    pub vehicle_key: ResKey,
    pub wealth: u8,
    pub generation: u16,
    pub sign: u8,
    pub is_male: bool,
    pub bio: Vec<u8>,
    pub age: u8,
    pub birth_month: u8,
    pub bitmap_id: u32,
    pub bitmap_path: Vec<u8>,
    pub last_commute_time: u32,
    pub retired: bool,
    pub last_local_disaster: u32,
    pub home_position: [f32; 3],
    pub eq: f32,
    pub hq: f32,
    pub eq_index: u8,
    pub eq_delta: f32,
    pub eq_ring: [f32; 6],
    pub hq_index: u8,
    pub hq_delta: f32,
    pub hq_ring: [f32; 6],
    pub car_key: ResKey,
    pub pedestrian_key: ResKey,
    pub unnamed_pair: (i32, i32),
    pub dispatch: Option<u64>,
    /// Pylon timer: units, elapsed, running.
    pub pylon: (u8, u64, u8),
    pub dispatch_tool: Option<u64>,
    pub unnamed_flag: bool,
    pub last_player_trip_day: i32,
    pub commute_check: bool,
    pub last_trip_update_day: i32,
    /// Occupant group → metres (§4.9).
    pub local: BTreeMap<u32, u32>,
    pub lua_table: bool,
    pub subscribed: bool,
    /// `MYSIM_TRIP_LOCAL_AREA_SIZE` (§4.11).
    pub local_area_size: i32,
    pub tuning: BTreeMap<String, f64>,
}

/// One MySim.
struct StateLock(RwLock<MySimState>);

impl StateLock {
    fn new(state: MySimState) -> Self {
        Self(RwLock::new(state))
    }
    fn borrow(&self) -> RwLockReadGuard<'_, MySimState> {
        self.0.read().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn borrow_mut(&self) -> RwLockWriteGuard<'_, MySimState> {
        self.0.write().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    fn get_mut(&mut self) -> &mut MySimState {
        self.0.get_mut().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Public getters run on the game's second thread during OnTick (§5.1).
pub struct MySim {
    host: Rc<dyn MySimHost>,
    class_id: u32,
    adv: RefCell<Weak<Advisor>>,
    rng: RefCell<RzRandom>,
    s: StateLock,
}

fn num(b: bool) -> f64 {
    if b {
        1.0
    } else {
        0.0
    }
}

/// `int(f × 100)` of a fraction.
fn pct_int(f: f32) -> f64 {
    ftol(r24(f64::from(f) * 100.0)) as f64
}

impl MySim {
    /// A new MySim of `class_id`, the `index`-th created in the session.
    pub fn new(host: Rc<dyn MySimHost>, class_id: u32, session_seed: u32, index: u32) -> Rc<MySim> {
        let seed = derive_seed(session_seed, super::clsid::MYSIM ^ index);
        Rc::new(MySim {
            host,
            class_id,
            adv: RefCell::new(Weak::new()),
            rng: RefCell::new(RzRandom::new(
                seed,
                &FixedSeeds {
                    wall_clock: seed,
                    process_clock: seed,
                },
            )),
            s: StateLock::new(MySimState {
                local_area_size: 10,
                ..MySimState::default()
            }),
        })
    }

    /// Runs `f` on the MySim of `adv`.
    pub fn with<R>(adv: &Rc<Advisor>, f: impl FnOnce(&MySim) -> R) -> Option<R> {
        let ext = adv.ext()?;
        let m = ext.as_any().downcast_ref::<MySim>()?;
        Some(f(m))
    }

    pub fn state(&self) -> MySimState {
        self.s.borrow().clone()
    }
    /// Pokes the state (tests).
    pub fn with_state<R>(&self, f: impl FnOnce(&mut MySimState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }
    /// The MySim's generator (§8).
    pub fn with_random<R>(&self, f: impl FnOnce(&mut RzRandom) -> R) -> R {
        f(&mut self.rng.borrow_mut())
    }

    pub fn advisor(&self) -> Option<Rc<Advisor>> {
        self.adv.borrow().upgrade()
    }
    fn id(&self) -> i32 {
        self.advisor().map_or(0, |a| a.id())
    }
    fn push(&self, ev: u32, subject: Option<Rc<AdviceSubject>>) {
        if let Some(a) = self.advisor() {
            a.push_event(ev, subject);
        }
    }
    fn new_subject(&self) -> Option<Rc<AdviceSubject>> {
        self.advisor().map(|a| AdviceSubject::new(&a.shared))
    }
    fn field(&self, name: &str, v: Field) {
        self.host.set_field(self.id(), name, v);
    }
    fn f(&self, name: &str, v: f64) {
        self.field(name, Field::Num(v));
    }
    fn post(&self, msg_type: u32, data1: u32, data2: u32, void1: Option<u64>) {
        self.host.post(&MySimPost {
            msg_type,
            data1,
            data2,
            void1,
            sim: Some(self.id()),
        });
    }
    fn set_ref(&self, which: fn(&mut MySimState) -> &mut Option<u64>, v: Option<u64>) {
        if let Some(t) = v {
            self.host.add_ref(t);
        }
        let old = std::mem::replace(which(&mut self.s.borrow_mut()), v);
        if let Some(t) = old {
            self.host.release(t);
        }
    }
    fn set_residence_ref(&self, v: Option<u64>) {
        self.set_ref(|s| &mut s.residence, v);
    }
    fn set_job_ref(&self, v: Option<u64>) {
        self.set_ref(|s| &mut s.job, v);
    }

    /// `IsActive` (slot 20).
    pub fn is_active(&self) -> bool {
        self.s.borrow().state != 0
    }
    /// Slot 10.
    pub fn slot(&self) -> u8 {
        self.s.borrow().slot
    }
    /// Slot 11.
    pub fn residence(&self) -> Option<u64> {
        self.s.borrow().residence
    }
    /// Slot 12.
    pub fn job(&self) -> Option<u64> {
        self.s.borrow().job
    }

    /// The live dispatch is also the version-17 saved reference (§6.2).
    pub fn set_dispatch(&self, token: Option<u64>) {
        self.set_ref(|s| &mut s.dispatch, token);
    }
    fn has_job(&self) -> bool {
        let s = self.s.borrow();
        s.job.is_some() || s.neighbor_job
    }

    // ---- move-in (§4.1) ----

    /// Slot 4 `MoveIn`.
    #[allow(clippy::too_many_arguments)]
    pub fn move_in(
        &self,
        slot: u8,
        name: &[u8],
        is_male: bool,
        sign: u8,
        bio: &[u8],
        bitmap_id: u32,
        bitmap_path: &[u8],
    ) {
        let month = self.advisor().map_or(0, |a| a.month_counter());
        if let Some(a) = self.advisor() {
            a.set_personal_name(name);
        }
        {
            let mut s = self.s.borrow_mut();
            s.slot = slot;
            s.is_male = is_male;
            s.sign = sign;
            s.bio = bio.to_vec();
            s.bitmap_id = bitmap_id;
            s.bitmap_path = bitmap_path.to_vec();
            s.generation = 1;
            s.birth_month = (month % 12) as u8;
            s.last_player_trip_day = 0;
        }
        self.host.dispatch_command(self.id(), "move_in_control");
    }

    /// Slot 13 `SetResidence(b)` → true.
    ///
    /// §4.1 step 3 ("otherwise: the tracking flag := dispatch
    /// `CheckFlags(0x100)`") is not stored: the flag feeds only the location
    /// tracker, which is UI (§0) and rides the host's `dispatch_command`.
    /// A dispatch replacement that needs it reads its own flags.
    pub fn set_residence(&self, b: Option<u64>) -> bool {
        let id = self.id();
        self.set_residence_ref(b);
        let lot = b.and_then(|b| self.host.lot(b));
        let pos = b.and_then(|b| self.host.occupant_position(b));
        {
            let mut s = self.s.borrow_mut();
            if let Some(l) = &lot {
                s.home_cell = (l.cell.0 as u32, l.cell.1 as u32);
            }
            if let Some(p) = pos {
                s.home_position = p;
            }
        }
        if self.s.borrow().state == 0 {
            self.reset_advisor_state();
            let (hx, hz) = self.home_xz();
            let age = self.host.average_age(hx, hz).unwrap_or(0).max(18);
            let wealth = lot.as_ref().map_or(0, |l| l.wealth);
            let eq = self.host.eq_by_wealth(hx, hz, wealth).unwrap_or(0.0);
            let hq = self.host.hq_by_wealth(hx, hz, wealth).unwrap_or(0.0);
            {
                let mut s = self.s.borrow_mut();
                s.age = age.clamp(0, 255) as u8;
                s.state = 1;
                s.wealth = wealth;
                s.eq = eq;
                s.hq = hq;
                s.eq_ring = [eq; 6];
                s.hq_ring = [hq; 6];
                s.eq_delta = 0.0;
                s.hq_delta = 0.0;
                s.eq_index = 0;
                s.hq_index = 0;
            }
            self.post(super::msg::MYSIMS_CHANGED, 0, 0, None);
            self.find_new_job();
            self.host.dispatch_command(id, "start_commute");
        } else {
            // The "tracking" flag := dispatch CheckFlags(0x100) (§4.1 step
            // 3). The default host answers false (the pre-dispatch-core
            // stub); a real dispatch answers its own flag. The flag feeds
            // only the location tracker (UI, §0), so it is not stored.
            let _ = self.host.dispatch_check_flags(id, 0x100);
        }
        self.update_variables();
        true
    }

    fn home_xz(&self) -> (i32, i32) {
        let c = self.s.borrow().home_cell;
        (c.0 as i32, c.1 as i32)
    }

    /// `ResetAdvisorState`: the advisor's Shutdown and Init with the same
    /// table index, keeping the personal name [choice].
    fn reset_advisor_state(&self) {
        if let Some(a) = self.advisor() {
            let name = a.personal_name();
            let idx = a.state().table_index;
            a.base_shutdown();
            a.base_init(idx);
            a.set_personal_name(&name);
        }
    }

    /// Slot 40 `AnnounceMoveIn`.
    pub fn announce_move_in(&self) {
        let (home, male, wealth) = {
            let s = self.s.borrow();
            (s.residence, s.is_male, s.wealth)
        };
        self.post(msg::MYSIM_MOVED_IN, u32::from(male), u32::from(wealth), home);
        self.update_variables();
        self.push(event::MOVE_IN, None);
    }

    // ---- the daily and monthly steps (§4.2, §4.3) ----

    fn skipped(&self) -> bool {
        self.s.borrow().state == 0 || self.host.ui_blocked()
    }

    fn day(&self, adv: &Rc<Advisor>) {
        if self.skipped() {
            return;
        }
        // 1.
        let search = {
            let s = self.s.borrow();
            s.job.is_none() && !s.neighbor_job && !s.retired && s.state == 2
        };
        if search && !self.find_new_job() {
            let mut s = self.s.borrow_mut();
            s.jobless_days = s.jobless_days.wrapping_add(1);
        }
        // 2.
        if std::mem::take(&mut self.s.borrow_mut().commute_check) {
            self.check_commute();
        }
        // 3–4.
        self.host.activate_lua(self.id());
        adv.base_day_pass();
        // 5.
        let day = adv.current_day();
        if day > self.s.borrow().last_trip_update_day {
            self.s.borrow_mut().last_trip_update_day = day;
            self.update_trip_variables(-1, -1, -1, -1, false);
        }
    }

    fn month(&self, adv: &Rc<Advisor>) {
        if self.skipped() {
            return;
        }
        // 1. Ageing (signed remainder).
        let mc = adv.month_counter();
        {
            let mut s = self.s.borrow_mut();
            if mc % 12 == i32::from(s.birth_month) {
                s.age = s.age.wrapping_add(1);
            }
        }
        // 2.
        if self.has_job() {
            self.check_commute();
        }
        // 3.
        self.update_variables();
        // 4.
        {
            let mut s = self.s.borrow_mut();
            let i = usize::from(s.eq_index) % 6;
            s.eq_delta = r24(f64::from(s.eq) - f64::from(s.eq_ring[i])) as f32;
            s.eq_ring[i] = s.eq;
            s.eq_index = ((i + 1) % 6) as u8;
            let j = usize::from(s.hq_index) % 6;
            s.hq_delta = r24(f64::from(s.hq) - f64::from(s.hq_ring[j])) as f32;
            s.hq_ring[j] = s.hq;
            s.hq_index = ((j + 1) % 6) as u8;
        }
        // 5.
        self.host.activate_lua(self.id());
    }

    // ---- the Lua table (§4.5) ----

    /// `UpdateQuickVariables`.
    pub fn update_quick_variables(&self) {
        let s = self.state();
        self.f("state", f64::from(s.state));
        self.f("age", f64::from(s.age));
        self.f("wealth", f64::from(s.wealth));
        self.f("generation", f64::from(s.generation));
        self.f("retired", num(s.retired));
        self.f("zodiac", f64::from(s.sign));
        self.f("is_male", num(s.is_male));
        self.f("last_local_disaster", f64::from(s.last_local_disaster));
        self.field("home_location", Field::Building(s.residence));
        self.field("job_location", Field::Building(s.job));
        let name = self.advisor().map(|a| a.personal_name()).unwrap_or_default();
        self.field("name", Field::Str(name));
    }

    /// `UpdateVariables` (§4.5).
    pub fn update_variables(&self) {
        self.update_quick_variables();
        if !self.s.borrow().lua_table {
            return;
        }
        let h = self.host.clone();
        let st = self.state();
        if let Some(home) = st.residence {
            let (hx, hz) = self.home_xz();
            let w = st.wealth;
            // EQ / HQ: only a positive value is kept.
            if let Some(v) = h.eq_by_wealth(hx, hz, w) {
                if v > 0.0 {
                    self.s.borrow_mut().eq = v;
                }
            }
            if let Some(v) = h.hq_by_wealth(hx, hz, w) {
                if v > 0.0 {
                    self.s.borrow_mut().hq = v;
                }
            }
            let (eq, hq) = {
                let s = self.s.borrow();
                (s.eq, s.hq)
            };
            self.f("eq", ftol(f64::from(eq)) as f64);
            self.f("hq", ftol(f64::from(hq)) as f64);
            if let Some(le) = h.life_expectancy(hx, hz) {
                self.f("le", f64::from(le));
            }
            let lot = h.lot(home);
            self.f("home_condition", f64::from(lot.as_ref().map_or(0, |l| l.condition)));
            if let Some(b) = h.building(home) {
                self.field("home_building", Field::Str(b.name));
            }
            if let Some(l) = &lot {
                self.f("home_has_road", num(l.road_access));
            }
            if let Some(p) = h.powered(home) {
                self.f("home_has_power", num(p));
            }
            if let Some(p) = h.watered(home) {
                self.f("home_has_water", num(p));
            }
            if let Some(v) = h.grid(Grid::Crime, hx, hz) {
                self.f("local_crime", f64::from(v as u8));
            }
            if let Some(l) = &lot {
                let (x0, z0, x1, z1) = l.rect;
                for (name, police) in [("home_police_coverage", true), ("home_fire_coverage", false)] {
                    if let Some(v) = h.coverage(police, x0, z0, x1 - x0 + 1, z1 - z0 + 1) {
                        self.f(name, f64::from(v as u8));
                    }
                }
            }
            for (i, name) in [
                "local_air_pollution",
                "local_water_pollution",
                "local_garbage",
                "local_radiation",
            ]
            .iter()
            .enumerate()
            {
                if let Some(v) = h.grid(Grid::Pollution(i as u8), hx, hz) {
                    self.f(name, f64::from(v as i16));
                }
            }
            self.f("last_player_trip_day", f64::from(st.last_player_trip_day));
            if (1..=3).contains(&w) {
                let dev = DESIRABILITY_DEVELOPERS[usize::from(w - 1)];
                if let Some(v) = h.grid(Grid::Desirability(dev), hx, hz) {
                    self.f("desirability", f64::from(v as i16));
                }
            }
            if let Some(l) = &lot {
                self.f("home_occupant_wealth", f64::from(l.wealth));
            }
            for (name, police) in [("local_police_funding", true), ("local_fire_funding", false)] {
                let f = self.nearest_station_funding(police, st.home_position);
                self.f(name, pct_int(f));
            }
            if let Some(l) = &lot {
                if let Some(v) = self.lot_congestion(l) {
                    self.f("local_traffic_congestion", f64::from(v));
                }
            }
            self.update_local_buildings();
        } else {
            self.f("last_player_trip_day", f64::from(st.last_player_trip_day));
        }
        self.f("delta_eq", ftol(f64::from(st.eq_delta)) as f64);
        self.f("delta_hq", ftol(f64::from(st.hq_delta)) as f64);
        let st = self.state();
        let title = st.job.map(|j| h.job_title(j)).unwrap_or_default();
        self.field("job_title", Field::Str(title));
        let jb = st.job.and_then(|j| h.building(j)).map(|b| b.name).unwrap_or_default();
        self.field("job_building", Field::Str(jb));
        let city = if st.neighbor_job {
            h.neighbor_city_name(st.neighbor_cell).unwrap_or_default()
        } else {
            Vec::new()
        };
        self.field("job_city", Field::Str(city));
        self.f("last_commute_time", f64::from(st.last_commute_time));
        self.f("employed", num(st.job.is_some() || st.neighbor_job));
        self.f("unemployed_days", f64::from(st.jobless_days));
        if let Some(j) = st.job {
            self.f("job_is_neighbor", 0.0);
            self.f("job_condition", f64::from(h.lot(j).map_or(0, |l| l.condition)));
            if let Some(p) = h.powered(j) {
                self.f("job_has_power", num(p));
            }
            if let Some(p) = h.watered(j) {
                self.f("job_has_water", num(p));
            }
        } else if st.neighbor_job {
            // [inferred] condition 0, power 1, water 1.
            self.f("job_is_neighbor", 1.0);
            self.f("job_condition", 0.0);
            self.f("job_has_power", 1.0);
            self.f("job_has_water", 1.0);
        }
    }

    /// The nearest station's funding (strict `<`, list order); 0 without a
    /// station.
    fn nearest_station_funding(&self, police: bool, home: [f32; 3]) -> f32 {
        let mut best: Option<(u32, f32)> = None;
        for (st, funding) in self.host.stations(police).unwrap_or_default() {
            let Some(p) = self.host.occupant_position(st) else {
                continue;
            };
            let d = distance(home, p[0], p[2]);
            if best.map_or(true, |(bd, _)| d < bd) {
                best = Some((d, funding));
            }
        }
        best.map_or(0.0, |b| b.1)
    }

    /// The congestion maximum over the lot's rectangle grown by 1, clamped
    /// to the city [inferred].
    fn lot_congestion(&self, l: &Lot) -> Option<i32> {
        let (cx, cz) = self.host.cell_counts();
        let (mx, mz) = (cx as i32 - 1, cz as i32 - 1);
        let (x0, z0, x1, z1) = l.rect;
        let mut best: Option<i32> = None;
        for x in (x0 - 1).max(0)..=(x1 + 1).min(mx) {
            for z in (z0 - 1).max(0)..=(z1 + 1).min(mz) {
                if let Some(v) = self.host.grid(Grid::Congestion, x, z) {
                    best = Some(best.map_or(v, |b| b.max(v)));
                }
            }
        }
        best
    }

    // ---- local buildings (§4.9) ----

    /// `cMySimBuildingFilter(residentialOnly)`.
    pub fn filter(&self, b: u64, residential_only: bool) -> bool {
        let Some(info) = self.host.building(b) else {
            return false;
        };
        if let Some(l) = self.host.lot(b) {
            if l.condition > 3 {
                return false;
            }
        }
        info.completion >= 1.0 && (!residential_only || info.purpose == 1)
    }

    /// `UpdateLocalBuildings`.
    pub fn update_local_buildings(&self) {
        let (hx, hz) = self.home_xz();
        let home = self.s.borrow().home_position;
        let mut map: BTreeMap<u32, u32> = BTreeMap::new();
        let list = self
            .host
            .buildings_in_box(hx - LOCAL_BOX, hz - LOCAL_BOX, hx + LOCAL_BOX, hz + LOCAL_BOX)
            .unwrap_or_default();
        for b in list {
            if !self.filter(b, false) {
                continue;
            }
            let Some(info) = self.host.building(b) else { continue };
            let d = distance(home, info.position[0], info.position[2]);
            for g in info.groups {
                let e = map.entry(g).or_insert(0);
                if *e == 0 || d < *e {
                    *e = d;
                }
            }
        }
        self.s.borrow_mut().local = map;
    }

    /// Slot 50 `DistanceToClosest(group)`.
    pub fn distance_to_closest(&self, group: u32) -> u32 {
        let s = self.s.borrow();
        if s.state == 0 {
            return NO_DISTANCE;
        }
        s.local.get(&group).copied().unwrap_or(NO_DISTANCE)
    }

    /// Slot 51 `DistanceToClosestSpecial(type)`.
    pub fn distance_to_closest_special(&self, building_type: u32) -> u32 {
        let home = self.s.borrow().home_position;
        self.host
            .special_buildings()
            .into_iter()
            .filter_map(|b| self.host.building(b))
            .filter(|i| i.building_type == building_type)
            .map(|i| distance(home, i.position[0], i.position[2]))
            .min()
            .unwrap_or(NO_DISTANCE)
    }

    /// Slots 52/53: the grade of the nearest building of `group`.
    pub fn local_grade(&self, group: u32, hospital: bool) -> f32 {
        let home = self.s.borrow().home_position;
        let mut best: Option<(u32, u64)> = None;
        for b in self.host.education_buildings(hospital) {
            let Some(info) = self.host.building(b) else { continue };
            if !info.groups.contains(&group) {
                continue;
            }
            let d = distance(home, info.position[0], info.position[2]);
            if best.map_or(true, |(bd, _)| d < bd) {
                best = Some((d, b));
            }
        }
        best.and_then(|(_, b)| self.host.grade(b, hospital)).unwrap_or(0.0)
    }

    // ---- trips (§4.6) ----

    /// `UpdateTripVariables(newState, oldState, newFlags, oldFlags, force)`.
    pub fn update_trip_variables(&self, new_state: i32, old_state: i32, new_flags: i32, old_flags: i32, force: bool) {
        if !self.s.borrow().lua_table || self.s.borrow().state == 0 {
            return;
        }
        let h = self.host.clone();
        let disp = h.dispatch(self.id()).unwrap_or_default();
        let mut changed = new_state != old_state;
        if changed || force {
            self.f("trip_in_transit", num(new_state == 2));
            self.f("trip_at_location", num(new_state == 3));
            if new_state == 3 {
                let was = self.s.borrow().state;
                if disp.at_home {
                    self.s.borrow_mut().state = 2;
                    if was == 1 {
                        self.announce_move_in();
                    }
                } else if disp.at_work {
                    self.s.borrow_mut().state = 3;
                }
            }
            self.update_quick_variables();
            let dest = if new_state == 2 {
                disp.destination
            } else if !disp.at_home && !disp.at_work {
                disp.location
            } else {
                None
            };
            self.field("destination_subject", Field::Building(dest));
        }
        let (nf, of) = if new_flags == -1 && old_flags == -1 {
            (disp.flags, disp.flags)
        } else {
            (new_flags as u32, old_flags as u32)
        };
        let flags = if force { disp.flags } else { nf };
        if (nf ^ of) & !0x100 != 0 || force {
            self.f("trip_at_transit_switch", num(flags & 0x10 != 0));
            self.f("trip_idle", num(flags & 1 != 0));
            if of & 0x8000 == 0 && nf & 0x8000 != 0 {
                self.push(event::APPROACHING_LOCATION, None);
            }
            let tt = disp.travel_type;
            let mut last = 255u32;
            for (name, n) in [
                ("trip_on_bus", 2u32),
                ("trip_on_foot", 0),
                ("trip_on_subway", 6),
                ("trip_in_car", 1),
                ("trip_on_train", 3),
                ("trip_on_elevated_train", 7),
                ("trip_on_monorail", 8),
            ] {
                self.f(name, num(tt == n));
                if tt == n {
                    last = n;
                }
            }
            self.f("last_trip_type", f64::from(last));
            changed = false;
        }
        let half = self.s.borrow().local_area_size / 2;
        let cell = if force { (0, 0) } else { disp.cell };
        let (x0, z0, x1, z1) = (cell.0 - half, cell.1 - half, cell.0 + half, cell.1 + half);
        if !h.trip_simulators() {
            return;
        }
        let idle = flags & 1 != 0;
        let tt = disp.travel_type;
        if disp.state == 2 || force {
            let (px, pz) = (disp.position[0], disp.position[2]);
            if !idle || force {
                // Literal reading of §4.6 "All grid reads convert the current
                // position with PositionToTract": the congestion map too
                // (question MS15 b).
                if let Some(v) = h.grid_at(Grid::Congestion, px, pz) {
                    self.f("trip_traffic_congestion", f64::from(v as u8));
                }
                let ttype = if tt == 0xFE || tt == 0xFF { 1 } else { tt };
                if let Some(r) = h.travel_time_ratio(cell.0, cell.1, ttype) {
                    self.f("trip_time_ratio", ftol(f64::from(r)) as f64);
                }
                self.f("trip_wait_length", f64::from(disp.wait_minutes));
            }
            for (i, name) in ["trip_pollution_air", "trip_pollution_water", "trip_pollution_garbage"]
                .iter()
                .enumerate()
            {
                if let Some(v) = h.grid_at(Grid::Pollution(i as u8), px, pz) {
                    self.f(name, f64::from(v as i16));
                }
            }
            if let Some(v) = h.grid_at(Grid::Crime, px, pz) {
                self.f("trip_crime", f64::from(v as u8));
            }
            if let Some(v) = h.grid_at(Grid::Aura, px, pz) {
                self.f("trip_mayor_rating", f64::from(v as i8));
            }
            if let Some(v) = h.grid_at(Grid::AverageEq, px, pz) {
                self.f("trip_eq", f64::from(v));
            }
            if let Some(v) = h.grid_at(Grid::AverageHq, px, pz) {
                self.f("trip_hq", f64::from(v));
            }
        }
        if force || disp.state == 3 || tt == 0 || tt == 1 || idle {
            self.trip_area(x0, z0, x1, z1);
        }
        if changed {
            self.post(msg::MYSIM_UPDATE, 0, 0, None);
        }
    }

    /// The lot statistics and the disaster flags of the trip square.
    pub fn trip_area(&self, x0: i32, z0: i32, x1: i32, z1: i32) {
        let h = self.host.clone();
        let mut n = 0u32;
        let mut c = [0u32; 6]; // distress1..3, low, med, high
        let mut landmark: Option<u64> = None;
        for l in h.lots_in_rect(x0, z0, x1, z1) {
            let Some(b) = l.building else { continue };
            n += 1;
            match l.condition {
                0 | 1 => {}
                2 => c[0] += 1,
                3 => c[1] += 1,
                _ => c[2] += 1,
            }
            match l.wealth {
                0 => {}
                1 => c[3] += 1,
                2 => c[4] += 1,
                _ => c[5] += 1,
            }
            if landmark.is_none() && h.building(b).is_some_and(|i| i.groups.contains(&GROUP_LANDMARK)) {
                landmark = Some(b);
            }
        }
        let sum = r24(f64::from(n) + f64::from(0.0001f32));
        let r = r24(1.0 / sum);
        for (i, name) in [
            "trip_distress1_p",
            "trip_distress2_p",
            "trip_distress3_p",
            "trip_wealth_low_p",
            "trip_wealth_med_p",
            "trip_wealth_high_p",
        ]
        .iter()
        .enumerate()
        {
            let v = r24(r24(f64::from(c[i]) * r) * 100.0);
            self.f(name, ftol(v) as f64);
        }
        self.f("trip_landmark_nearby", num(landmark.is_some()));
        let name = landmark.and_then(|b| h.building(b)).map(|i| i.name).unwrap_or_default();
        self.field("trip_landmark_name", Field::Str(name));
        let funding = landmark.map_or(0.0, |b| h.landmark_funding(b).unwrap_or(0.0));
        self.f("trip_landmark_funding_p", pct_int(funding));
        let (mut any, mut fire, mut riot) = (false, false, false);
        for (t, (x, z)) in h.disaster_locations() {
            if (x0..=x1).contains(&x) && (z0..=z1).contains(&z) {
                any = true;
                fire |= t == disaster::FIRE;
                riot |= t == disaster::RIOT;
                if fire && riot {
                    break;
                }
            }
        }
        self.f("trip_any_disaster_nearby", num(any));
        self.f("trip_fire_nearby", num(fire));
        self.f("trip_riot_nearby", num(riot));
    }

    // ---- jobs, commutes, residences (§4.7) ----

    /// `FindNewJob`.
    pub fn find_new_job(&self) -> bool {
        self.host.find_new_job(self.id())
    }

    /// The dispatch reported the job destination (`0xEBA06BF4`).
    pub fn job_found(&self, dest: JobDestination) {
        match dest {
            JobDestination::Lot { building, cell } => {
                self.set_job_ref(Some(building));
                let mut s = self.s.borrow_mut();
                s.neighbor_job = false;
                s.commute_cell = cell;
            }
            JobDestination::Neighbor { cell } => {
                self.set_job_ref(None);
                let mut s = self.s.borrow_mut();
                s.neighbor_job = true;
                s.neighbor_cell = cell;
                s.commute_cell = cell;
            }
        }
        self.s.borrow_mut().jobless_days = 0;
        self.update_variables();
        self.push(event::NEW_JOB, None);
    }

    /// Slot 8 `ChangeJob(b)`.
    pub fn change_job(&self, b: u64) -> bool {
        self.host.change_job(self.id(), b)
    }

    /// `CheckCommute` (§4.7, `0x0042D9E0`): home lot rect → job lot rect
    /// (or the neighbour cell) on a pathfinder of its own; success stores
    /// `lastCommuteTime := uint(trunc(max(0, totalTime)))`, failure runs
    /// `FailCommute`. The pathfinder's configuration is not published
    /// (§4.7 refers to §7.2, which lists no slot use; question MS15), so
    /// the query is the host's typed traffic callback `commute_time`.
    pub fn check_commute(&self) {
        match self.host.commute_time(self.id()) {
            Some(t) => {
                let v = ftol(f64::from(t.max(0.0))) as u32;
                self.s.borrow_mut().last_commute_time = v;
            }
            None => self.fail_commute(),
        }
    }

    /// `FailCommute` (§4.7, `0x0042C8E0`): `QuitJob()`, push
    /// `MYSIM_COMMUTE_FAILED`, then `SetCurrentCommuteDestination(default
    /// location, byCommute 1, notify = true)`: the same call as `FindNewJob`
    /// (§4.7.1 callers table), so a new job search, not a plain reset. Its
    /// result is not used.
    pub fn fail_commute(&self) {
        self.quit_job();
        self.push(event::COMMUTE_FAILED, None);
        let _ = self.host.find_new_job(self.id());
    }

    /// `QuitJob` (state ≠ 0).
    pub fn quit_job(&self) {
        if self.s.borrow().state == 0 {
            return;
        }
        self.set_job_ref(None);
        {
            let mut s = self.s.borrow_mut();
            s.neighbor_job = false;
            s.neighbor_cell = (0, 0);
            s.commute_cell = (0, 0);
            if s.state != 5 {
                s.state = 2;
            }
        }
        self.update_variables();
    }

    /// `DoNeighborConnectionRemoved` (the arguments are ignored).
    pub fn neighbor_connection_removed(&self) {
        if self.s.borrow().neighbor_job {
            self.s.borrow_mut().commute_check = true;
        }
    }

    /// Slot 7 `TestForResidence(b, filter, matchWealth)`; `lives_here` is
    /// the agent's `MySimLivesHere(b)` answer.
    pub fn test_for_residence(&self, b: u64, filter: bool, match_wealth: bool, lives_here: bool) -> bool {
        if self.residence() == Some(b) {
            return false;
        }
        if match_wealth {
            if let Some(l) = self.host.lot(b) {
                if l.wealth != self.s.borrow().wealth {
                    return false;
                }
            }
        }
        if lives_here {
            return false;
        }
        !filter || self.filter(b, true)
    }

    /// Whether another MySim lives in `b` (the agent's `MySimLivesHere`,
    /// which skips the My Street sim).
    fn other_lives_here(&self, b: u64) -> bool {
        let Some(adv) = self.advisor() else { return false };
        let Some(sys) = adv.shared.sys() else { return false };
        sys.advisors().into_iter().any(|(_, a)| {
            !Rc::ptr_eq(&a, &adv)
                && a.id() != MY_STREET_ID
                && MySim::with(&a, |m| m.residence() == Some(b)).unwrap_or(false)
        })
    }

    /// Slot 6 `ChangeResidence(b)`.
    pub fn change_residence(&self, b: u64) -> bool {
        let old = self.residence();
        if self.test_for_residence(b, false, false, self.other_lives_here(b)) {
            self.move_out();
            self.set_residence(Some(b));
            let would = self
                .advisor()
                .map(|a| a.command_would_trigger(command::MOVE_OUT))
                .unwrap_or(false);
            if would {
                self.move_out();
                if old.is_some() {
                    self.set_residence(old);
                }
            }
        }
        let new = self.residence();
        if new != old && new.is_some() {
            let w = new.and_then(|b| self.host.lot(b)).map_or(0, |l| l.wealth);
            let mut s = self.s.borrow_mut();
            s.wealth = w;
            s.state = 1;
            true
        } else {
            false
        }
    }

    /// Slot 5 `FindNewResidence` → always false.
    pub fn find_new_residence(&self) -> bool {
        let old = self.residence();
        if let Some(all) = self.host.all_buildings() {
            let list: Vec<u64> = all.into_iter().filter(|&b| self.filter(b, true)).collect();
            let n = list.len() as u32;
            if n > 0 {
                let r = self.rng.borrow_mut().uniform_u32(n);
                let start = r.saturating_sub(1) as usize;
                for k in 0..list.len() {
                    let b = list[(start + k) % list.len()];
                    if self.test_for_residence(b, false, false, self.other_lives_here(b)) && self.change_residence(b) {
                        break;
                    }
                }
            }
        }
        let now = self.residence();
        if now.is_none() || now == old {
            if now.is_some() {
                self.move_out();
            }
            self.push(event::CANT_FIND_HOME, None);
        }
        false
    }

    /// `MoveOut` → true.
    pub fn move_out(&self) -> bool {
        self.set_residence_ref(None);
        true
    }

    /// `LeaveCity` (state ≠ 0).
    pub fn leave_city(&self) {
        if self.s.borrow().state == 0 {
            return;
        }
        self.move_out();
        self.s.borrow_mut().state = 0;
        self.host.dispatch_command(self.id(), "shutdown");
        self.post(msg::MYSIMS_CHANGED, 0, 0, None);
    }

    /// Slot 9 `DeleteFromGame` → true.
    pub fn delete_from_game(&self) -> bool {
        self.host.dispatch_command(self.id(), "shutdown");
        self.move_out();
        self.s.borrow_mut().state = 0;
        self.reset_advisor_state();
        self.post(msg::MYSIMS_CHANGED, 0, 0, None);
        true
    }

    // ---- script commands (§4.8) ----

    /// Slot 41 `DoScriptCommand`.
    pub fn do_script_command(&self, cmd: u32) {
        if self.s.borrow().state == 0 {
            return;
        }
        match cmd {
            command::MOVE_OUT => {
                self.find_new_residence();
            }
            command::RETIRE => {
                self.quit_job();
                self.s.borrow_mut().retired = true;
            }
            command::DIE => {
                let pos = self.s.borrow().home_position;
                self.host.automata_trigger(DIED_TRIGGER, pos);
                self.host.dispatch_command(self.id(), "ghost");
                self.leave_city();
            }
            command::LEAVE_CITY => self.leave_city(),
            command::SPAWN => {
                let w = self.s.borrow().wealth;
                let age = self.host.new_age_by_wealth(w);
                {
                    let mut s = self.s.borrow_mut();
                    s.retired = false;
                    s.generation = s.generation.wrapping_add(1);
                    s.bio.clear();
                    if let Some(a) = age {
                        s.age = a;
                    }
                }
                self.quit_job();
                self.host.dispatch_command(self.id(), "ghost");
                self.post(msg::MYSIMS_CHANGED, 0, 0, None);
            }
            command::INCREASE_WEALTH => {
                if self.s.borrow().wealth < 3 {
                    self.s.borrow_mut().wealth += 1;
                    self.quit_job();
                    self.find_new_residence();
                }
            }
            command::QUIT_JOB => self.quit_job(),
            _ => {}
        }
    }

    // ---- city events (§5.2) ----

    fn distance_event(&self, ev: u32, x: f32, z: f32) {
        if self.s.borrow().state == 0 {
            return;
        }
        let d = distance(self.s.borrow().home_position, x, z);
        if let Some(s) = self.new_subject() {
            s.set_data1(d);
            self.push(ev, Some(s));
        }
    }

    /// Slot 42 `DoBuildingAdded`.
    pub fn building_added(&self, b: u64) {
        if self.s.borrow().state == 0 || self.residence().is_none() {
            return;
        }
        let Some(p) = self.host.occupant_position(b) else {
            return;
        };
        let d = distance(self.s.borrow().home_position, p[0], p[2]);
        if let Some(s) = self.new_subject() {
            s.set_object(Some(b));
            s.set_data1(d);
            self.push(event::BUILDING_ADD, Some(s));
        }
    }

    /// Slot 43 `DoBuildingChanged`.
    pub fn building_changed(&self, b: u64) {
        if self.s.borrow().state == 0 {
            return;
        }
        if self.residence() == Some(b) || self.job() == Some(b) {
            self.update_variables();
        }
    }

    /// Slot 44 `DoBuildingRemoved(b, cause)`.
    pub fn building_removed(&self, b: u64, cause: u32) {
        if self.s.borrow().state == 0 {
            return;
        }
        if self.residence() == Some(b) {
            if cause == 0 {
                self.push(event::HOME_DESTROYED, None);
                self.find_new_residence();
            } else {
                self.s.borrow_mut().last_local_disaster = cause;
                self.update_variables();
                self.push(event::HOME_DESTROYED_BY_DISASTER, None);
                self.move_out();
            }
        } else if self.job() == Some(b) {
            if cause == 0 {
                self.push(event::JOB_DESTROYED, None);
            } else {
                self.s.borrow_mut().last_local_disaster = cause;
                self.update_variables();
                self.push(event::JOB_DESTROYED_BY_DISASTER, None);
            }
            self.quit_job();
        }
    }

    /// Slot 45.
    pub fn fire_added(&self, x: f32, z: f32) {
        self.distance_event(event::FIRE_STARTED, x, z);
    }
    /// Slot 46.
    pub fn pipe_burst(&self, x: f32, z: f32) {
        self.distance_event(event::PIPE_BURST, x, z);
    }
    /// Slot 47.
    pub fn tornado(&self, x: f32, z: f32) {
        self.distance_event(event::TORNADO, x, z);
    }

    /// Slot 48 `DoDisaster(type, cx, cz)`: riots and volcanoes only.
    pub fn disaster(&self, disaster_type: u32, cx: i32, cz: i32) {
        if self.s.borrow().state == 0 {
            return;
        }
        let ev = match disaster_type {
            disaster::RIOT => event::RIOT,
            disaster::VOLCANO => event::VOLCANO,
            _ => return,
        };
        let (x, z) = self.host.cell_center(cx, cz);
        let d = distance(self.s.borrow().home_position, x, z);
        if let Some(s) = self.new_subject() {
            // Cell indices, not metres [quirk]; unclamped by SetLocation's
            // own clamp only.
            s.set_location(cx as f32, cz as f32);
            s.set_data1(d);
            self.push(ev, Some(s));
        }
    }

    // ---- the record (§6.2) ----

    fn put_ref(&self, out: &mut dyn OutStream, t: Option<u64>) {
        let r = t.map_or(RefValue::Null, |t| self.host.object_ref(t));
        out.put_ref(&r);
    }

    fn write_fields(&self, out: &mut dyn OutStream) {
        let s = self.state();
        out.put_u16(RECORD_VERSION);
        out.put_u8(s.state);
        if s.state == 0 {
            return;
        }
        out.put_u8(s.slot);
        self.put_ref(out, s.residence);
        out.put_u32(s.home_cell.0);
        out.put_u32(s.home_cell.1);
        self.put_ref(out, s.job);
        for v in [s.neighbor_cell.0, s.neighbor_cell.1, s.commute_cell.0, s.commute_cell.1] {
            out.put_i32(v);
        }
        out.put_u32(s.jobless_days);
        out.put_res_key(s.vehicle_key);
        out.put_u8(s.wealth);
        out.put_u16(s.generation);
        out.put_u8(s.sign);
        out.put_bool(s.is_male);
        out.put_str(&s.bio);
        out.put_u8(s.age);
        out.put_u8(s.birth_month);
        out.put_u32(s.bitmap_id);
        out.put_str(&s.bitmap_path);
        out.put_u32(s.last_commute_time);
        out.put_bool(s.retired);
        out.put_u32(s.last_local_disaster);
        for v in s.home_position {
            out.put_f32(v);
        }
        for (idx, delta, ring) in [(s.eq_index, s.eq_delta, s.eq_ring), (s.hq_index, s.hq_delta, s.hq_ring)] {
            out.put_u8(idx);
            out.put_f32(delta);
            for v in ring {
                out.put_f32(v);
            }
        }
        out.put_res_key(s.car_key);
        out.put_res_key(s.pedestrian_key);
        out.put_i32(s.unnamed_pair.0);
        out.put_i32(s.unnamed_pair.1);
        self.put_ref(out, s.dispatch);
        out.put_u8(s.pylon.0);
        out.put_u64(s.pylon.1);
        out.put_u8(s.pylon.2);
        out.put_bool(s.dispatch_tool.is_some());
        if s.dispatch_tool.is_some() {
            self.put_ref(out, s.dispatch_tool);
        }
        out.put_bool(s.unnamed_flag);
        out.put_i32(s.last_player_trip_day);
    }

    fn get_ref(&self, inp: &mut dyn InStream, iid_: u32) -> Option<Option<u64>> {
        let r = inp.get_ref_as(iid_)?;
        if r.is_null() {
            return Some(None);
        }
        // A resolved reference is held (AddRef).
        let t = self.host.resolve_ref(&r, iid_);
        if let Some(t) = t {
            self.host.add_ref(t);
        }
        Some(t)
    }

    fn read_fields(&self, inp: &mut dyn InStream) -> Option<()> {
        let v = inp.get_u16()?;
        if !(11..=17).contains(&v) {
            return None;
        }
        let state = inp.get_u8()?;
        let mut s = self.state();
        s.state = state;
        if state == 0 {
            *self.s.borrow_mut() = s;
            return Some(());
        }
        s.slot = inp.get_u8()?;
        let residence = self.get_ref(inp, iid::BUILDING_OCCUPANT)?;
        s.home_cell = (inp.get_u32()?, inp.get_u32()?);
        let job = self.get_ref(inp, iid::BUILDING_OCCUPANT)?;
        s.neighbor_cell = (inp.get_i32()?, inp.get_i32()?);
        s.commute_cell = (inp.get_i32()?, inp.get_i32()?);
        s.jobless_days = inp.get_u32()?;
        s.vehicle_key = inp.get_res_key()?;
        s.wealth = inp.get_u8()?;
        s.generation = inp.get_u16()?;
        s.sign = inp.get_u8()?;
        s.is_male = inp.get_bool()?;
        s.bio = inp.get_str()?;
        s.age = inp.get_u8()?;
        s.birth_month = inp.get_u8()?;
        s.bitmap_id = inp.get_u32()?;
        s.bitmap_path = inp.get_str()?;
        s.last_commute_time = inp.get_u32()?;
        s.retired = inp.get_bool()?;
        s.last_local_disaster = inp.get_u32()?;
        for p in s.home_position.iter_mut() {
            *p = inp.get_f32()?;
        }
        s.eq_index = inp.get_u8()?;
        s.eq_delta = inp.get_f32()?;
        for p in s.eq_ring.iter_mut() {
            *p = inp.get_f32()?;
        }
        s.hq_index = inp.get_u8()?;
        s.hq_delta = inp.get_f32()?;
        for p in s.hq_ring.iter_mut() {
            *p = inp.get_f32()?;
        }
        if (11..=14).contains(&v) {
            for _ in 0..5 {
                inp.get_i32()?;
            }
        } else if v >= 15 {
            s.car_key = inp.get_res_key()?;
            s.pedestrian_key = inp.get_res_key()?;
        }
        if v >= 17 {
            s.unnamed_pair = (inp.get_i32()?, inp.get_i32()?);
        }
        let old_dispatch = s.dispatch.take();
        if let Some(d) = old_dispatch {
            if v != 12 {
                self.host.release(d);
            } else {
                s.dispatch = Some(d);
            }
        }
        if v >= 12 {
            if v == 12 {
                if !self.host.read_dispatch_inline(self.id(), inp) {
                    return None;
                }
            } else {
                s.dispatch = self.get_ref(inp, iid::DISPATCH)?;
            }
            s.pylon = (inp.get_u8()?, inp.get_u64()?, inp.get_u8()?);
            let has_tool = inp.get_bool()?;
            s.dispatch_tool = if has_tool {
                self.get_ref(inp, iid::DISPATCH_TOOL)?
            } else {
                None
            };
        }
        if v >= 14 {
            s.unnamed_flag = inp.get_bool()?;
        }
        if v >= 16 {
            s.last_player_trip_day = inp.get_i32()?;
        }
        // Swap the references in.
        let (old_res, old_job) = {
            let cur = self.s.borrow();
            (cur.residence, cur.job)
        };
        s.residence = residence;
        s.job = job;
        for t in [old_res, old_job].into_iter().flatten() {
            self.host.release(t);
        }
        *self.s.borrow_mut() = s;
        Some(())
    }
}

impl Drop for MySim {
    fn drop(&mut self) {
        let s = self.s.get_mut();
        for t in [
            s.residence.take(),
            s.job.take(),
            s.dispatch.take(),
            s.dispatch_tool.take(),
        ]
        .into_iter()
        .flatten()
        {
            self.host.release(t);
        }
    }
}

impl AdvisorExt for MySim {
    fn class_id(&self) -> u32 {
        self.class_id
    }

    fn attach(&self, adv: &Rc<Advisor>) {
        *self.adv.borrow_mut() = Rc::downgrade(adv);
    }

    fn is_mysim(&self) -> bool {
        true
    }

    /// Slot 3: the base Init, then the dispatch, the Lua table and the
    /// tuning (§7.3, §4.11).
    fn init(&self, adv: &Rc<Advisor>, table_index: i32) -> bool {
        if !adv.base_init(table_index) {
            return false;
        }
        let id = adv.id();
        if !self.s.borrow().subscribed {
            let h = adv.host();
            for m in msg::MYSIM_SUBSCRIPTIONS {
                h.subscribe(&Target::Advisor(adv.clone()), m);
            }
            self.s.borrow_mut().subscribed = true;
        }
        // [inferred, Q3] The neighbour job city from the saved cell.
        {
            let (job, cell, state) = {
                let s = self.s.borrow();
                (s.job, s.neighbor_cell, s.state)
            };
            if state != 0 && job.is_none() && cell != (0, 0) {
                let found = self.host.neighbor_city_name(cell).is_some();
                self.s.borrow_mut().neighbor_job = found;
            }
        }
        if self.s.borrow().dispatch.is_none() {
            self.host.dispatch_command(id, "create");
        }
        let mut tuning = BTreeMap::new();
        for (k, d) in TUNING {
            tuning.insert(k.to_string(), self.host.tuning(k).unwrap_or(d));
        }
        let area = tuning["MYSIM_TRIP_LOCAL_AREA_SIZE"];
        {
            let mut s = self.s.borrow_mut();
            s.local_area_size = ftol(area) as i32;
            s.tuning = tuning;
        }
        if !self.s.borrow().lua_table {
            let made = self.host.create_table(id);
            self.s.borrow_mut().lua_table = made;
            if made {
                self.update_trip_variables(-1, -1, -1, -1, true);
            }
        }
        true
    }

    fn shutdown(&self, adv: &Rc<Advisor>) -> bool {
        if std::mem::take(&mut self.s.borrow_mut().subscribed) {
            let h = adv.host();
            for m in msg::MYSIM_SUBSCRIPTIONS {
                h.unsubscribe(&Target::Advisor(adv.clone()), m);
            }
        }
        adv.base_shutdown()
    }

    fn simulate_day(&self, adv: &Rc<Advisor>) {
        self.day(adv);
    }

    fn monthly(&self, adv: &Rc<Advisor>) {
        self.month(adv);
    }

    /// Slot 19: false when state = 0; otherwise the Lua context, then the
    /// base.
    fn command_would_trigger(&self, adv: &Rc<Advisor>, cmd: u32) -> bool {
        if self.s.borrow().state == 0 {
            return false;
        }
        self.host.activate_lua(adv.id());
        adv.base_command_would_trigger(cmd)
    }

    /// §3.3. `unknown` of the MySim update is the MySim's advisor ID
    /// [choice].
    fn do_message(&self, adv: &Rc<Advisor>, m: &AdvisorMessage) -> bool {
        match m.msg_type {
            msg::MYSIM_UPDATE => {
                if m.unknown.is_none() || m.unknown == Some(adv.id() as u64) {
                    self.update_variables();
                }
                true
            }
            msg::DISPATCH_NOTIFICATION => {
                let (d2, d3) = (m.data2 as i32, m.data3 as i32);
                match m.data1 {
                    dispatch_note::TRIP_STATE_B => self.update_trip_variables(d3, d2, -1, -1, false),
                    dispatch_note::TRIP_STATE_A => self.update_trip_variables(-1, -1, d3, d2, false),
                    dispatch_note::JOB_FOUND => {
                        if let Some(d) = self.host.job_destination(adv.id()) {
                            self.job_found(d);
                        }
                    }
                    dispatch_note::ARRIVAL => {
                        let d = self.host.dispatch(adv.id()).unwrap_or_default();
                        if d.by_commute {
                            self.fail_commute();
                        } else if d.at_home && self.s.borrow().state == 1 {
                            self.s.borrow_mut().state = 2;
                            self.announce_move_in();
                        }
                    }
                    dispatch_note::STATE_1 | dispatch_note::STATE_2 => {
                        // [inferred] data2 is the new state.
                        if (1..=6).contains(&m.data2) {
                            self.s.borrow_mut().state = m.data2 as u8;
                        }
                        self.post(msg::MYSIM_UPDATE, 0, 0, None);
                    }
                    _ => {}
                }
                true
            }
            msg::POST_CITY_INIT
            | msg::DEBUG_PRINT
            | msg::DEBUG_NEW_JOB
            | msg::DEBUG_NEW_HOUSE
            | msg::DEBUG_DISPATCH => true,
            _ => adv.base_do_message(m),
        }
    }

    fn write_prefix(&self, out: &mut dyn OutStream) -> bool {
        self.write_fields(out);
        out.error() == 0
    }

    fn read_prefix(&self, inp: &mut dyn InStream) -> bool {
        self.read_fields(inp).is_some()
    }

    fn distance_to_closest(&self, group: u32) -> Option<u32> {
        Some(MySim::distance_to_closest(self, group))
    }

    fn distance_to_closest_special(&self, building_type: u32) -> Option<u32> {
        Some(MySim::distance_to_closest_special(self, building_type))
    }

    fn local_school_grade(&self, group: u32) -> Option<f32> {
        Some(self.local_grade(group, false))
    }

    fn local_hospital_grade(&self, group: u32) -> Option<f32> {
        Some(self.local_grade(group, true))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
