// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4MySimDispatch` (class `0xCBC14674`, read as IID `0xEB8B6F9A`), the
//! per-MySim trip controller. mysims.md §1.1 leaves the class itself to the
//! automata spec, but the simulation side names its contract:
//!
//! - `FindNewJob` runs a traffic pathfinder from the home lot towards job
//!   destinations and takes the lot (building) or the neighbour connection
//!   at the end (§4.7, commuter travel-type mask `0x1CF` *[inferred]*).
//! - `ChangeJob(b)` sets the commute destination to building `b`; the
//!   result is the dispatch's (§4.7).
//! - `CheckCommute` pathfinds home → job (or the neighbour cell) and
//!   reports the total time; the MySim stores `trunc(max(0, t))`, and a
//!   failure runs `FailCommute` (§4.7).
//! - `CheckFlags(0x100)` is the tracking flag of §4.1 step 3.
//! - `Init` only takes and drops the clock reference (clock24.md).
//! - the `0x2BA06B99` notification sub-IDs the MySim reacts to (§3.3):
//!   `0x4BB2D020`/`0x4BB2CFC0` trip states, `0xEBA06BF4` job found,
//!   `0xEBA06C12` arrival, `0xEBA06BD4`/`0x4BB71A7E` state transitions.
//!
//! What the spec leaves to the automata layer — the trip automaton, the
//! walking character, balloons — is not implemented; the module drives the
//! pathfinder through [`DispatchHost`] and the notifications through it.
//! The default answers of the MySim host keep the shipped stub behaviour
//! (mysims.md §10 *[choice]*: "arrived" immediately, no trip fields).

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use crate::pathfinder::{filter, flag, result, CellRect, Pathfinder, PathfinderHost, SharedWorkCheck};
use crate::traffic_network::NetworkModel;

use super::{JobDestination, DISPATCH_CLSID, DISPATCH_IID};

/// The commuter travel-type mask of the MySim job/commute pathfinder
/// (§4.7 *[inferred: `0x1CF`]* — the same mask the traffic simulator sets
/// for commuter trips: Walk, Car, Bus, Passenger train, Subway, Elevated
/// train, Monorail).
const COMMUTER_TRAVEL_MASK: u16 = 0x1CF;
/// The pathfinder setup of pathfinder.md §8: start rect, travel-type flags,
/// density 0, flags 1 | 2, filter 3, a route deque, end rect, `FindPath`,
/// `Reset`.
const PATHFINDER_FLAGS: u32 = flag::NO_BUDGET | flag::NEAREST_ON_FAILURE;
const DENSITY: u32 = 0;
const MAX_WORK_TIME: u32 = 0x1000;

/// What the dispatch reads from the game beyond the pathfinder host: the
/// MySim's home and job, the city's buildings, and a way to post the
/// notification messages. Callbacks must not call back into the dispatch.
pub trait DispatchHost: PathfinderHost {
    /// The home lot's bounding rectangle.
    fn home_rect(&self, sim: i32) -> Option<CellRect>;
    /// The job building's lot rectangle, when the MySim has one.
    fn job_rect(&self, sim: i32) -> Option<CellRect>;
    /// The lot rectangle of a building the commute is aimed at
    /// (`ChangeJob`); `None` when there is no such building.
    fn building_rect(&self, building: u64) -> Option<CellRect>;
    /// The neighbour city cell the MySim would commute to.
    fn neighbor_cell(&self, sim: i32) -> Option<(i32, i32)>;
    /// The job candidate rectangles: every lot with free commuter
    /// capacity. An empty list means the search fails (§4.7).
    fn job_candidates(&self, sim: i32) -> Vec<CellRect>;
    /// The building on a destination cell the pathfinder reached.
    fn building_at_cell(&self, x: i32, z: i32) -> Option<u64>;
    /// Whether a cell is a neighbour connection to another city.
    fn is_neighbor_cell(&self, x: i32, z: i32) -> bool;
    /// Posts a `0x2BA06B99` notification at the MySim.
    fn notify(&self, sim: i32, sub_id: u32, data1: u32, data2: u32);
}

/// One MySim's dispatch state: the fields the simulation side reads (§4.6)
/// plus the commute destination.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DispatchState {
    /// 2 travelling, 3 at a location (§4.6).
    pub trip_state: i32,
    pub flags: u32,
    pub cell: (i32, i32),
    /// The commute destination building (`ChangeJob`, `FindNewJob`).
    pub destination: Option<u64>,
    /// The destination is still "by commute" (arrival, §3.3).
    pub destination_by_commute: bool,
    /// The last pathfinder run's total time (§4.7 `CheckCommute`).
    pub trip_length: f32,
}

/// The per-MySim trip controller.
pub struct MySimDispatch {
    host: Rc<dyn DispatchHost>,
    net: Rc<NetworkModel>,
    shared: SharedWorkCheck,
    sims: RefCell<BTreeMap<i32, DispatchState>>,
}

impl MySimDispatch {
    pub fn new(host: Rc<dyn DispatchHost>, net: Rc<NetworkModel>, shared: SharedWorkCheck) -> Rc<MySimDispatch> {
        Rc::new(MySimDispatch {
            host,
            net,
            shared,
            sims: RefCell::new(BTreeMap::new()),
        })
    }

    /// The class and interface IDs (§1.1).
    pub fn class_id(&self) -> u32 {
        DISPATCH_CLSID
    }

    pub fn interface_id(&self) -> u32 {
        DISPATCH_IID
    }

    /// `Init` (clock24.md: only takes and drops the clock reference).
    pub fn init(&self, sim: i32) -> bool {
        self.sims.borrow_mut().entry(sim).or_default();
        true
    }

    /// `Shutdown` (the MySim's Shutdown and LeaveCity call it, §4.7).
    pub fn shutdown(&self, sim: i32) {
        self.sims.borrow_mut().remove(&sim);
    }

    /// The state the simulation side reads (§4.6 trip variables).
    pub fn get_state(&self, sim: i32) -> DispatchState {
        self.sims.borrow().get(&sim).cloned().unwrap_or_default()
    }

    /// Tests the dispatch's flags (§4.1 step 3 tracking flag): all bits of
    /// `mask` are set.
    pub fn check_flags(&self, sim: i32, mask: u32) -> bool {
        self.get_state(sim).flags & mask == mask
    }

    /// `FindNewJob` (§4.7): the pathfinder from the home lot towards the
    /// job candidates; true when a destination was set. The destination is
    /// reported by [`Self::job_destination`] when the `0xEBA06BF4`
    /// notification fires (the MySim's job_found).
    pub fn find_new_job(&self, sim: i32) -> bool {
        let Some(home) = self.host.home_rect(sim) else {
            return false;
        };
        let candidates = self.host.job_candidates(sim);
        let Some(end) = candidates.first() else {
            return false;
        };
        let Some(found) = self.run_pathfinder(home, *end) else {
            return false;
        };
        {
            let mut sims = self.sims.borrow_mut();
            let state = sims.entry(sim).or_default();
            state.destination = found.building;
            state.destination_by_commute = true;
            state.cell = found.cell;
            state.trip_state = 2;
            state.trip_length = found.trip_length;
        }
        self.host.notify(sim, super::dispatch_note::JOB_FOUND, 0, 0);
        true
    }

    /// `ChangeJob(b)` (§4.7): set the commute destination to building `b`;
    /// the result of the dispatch.
    pub fn change_job(&self, sim: i32, building: u64) -> bool {
        let Some(home) = self.host.home_rect(sim) else {
            return false;
        };
        let Some(end) = self.host.building_rect(building) else {
            return false;
        };
        let Some(found) = self.run_pathfinder(home, end) else {
            return false;
        };
        let mut sims = self.sims.borrow_mut();
        let state = sims.entry(sim).or_default();
        state.destination = Some(building);
        state.destination_by_commute = true;
        state.cell = found.cell;
        state.trip_state = 2;
        state.trip_length = found.trip_length;
        true
    }

    /// `CheckCommute` (§4.7): the total time of home → job (or the
    /// neighbour cell), `None` on failure (the MySim then runs
    /// `FailCommute`).
    pub fn check_commute(&self, sim: i32) -> Option<f32> {
        let home = self.host.home_rect(sim)?;
        let end = match self.get_state(sim).destination {
            Some(building) => self.host.building_rect(building)?,
            None => {
                let (x, z) = self.host.neighbor_cell(sim)?;
                CellRect::new(x, z, x, z)
            }
        };
        let found = self.run_pathfinder(home, end)?;
        self.sims.borrow_mut().entry(sim).or_default().trip_length = found.trip_length;
        Some(found.trip_length)
    }

    /// The destination the `0xEBA06BF4` notification reports (§4.7): a city
    /// lot's building, or the neighbour city at the reached cell.
    pub fn job_destination(&self, sim: i32) -> Option<JobDestination> {
        let state = self.get_state(sim);
        let (x, z) = state.cell;
        if let Some(building) = state.destination.or_else(|| self.host.building_at_cell(x, z)) {
            return Some(JobDestination::Lot { building, cell: (x, z) });
        }
        if self.host.is_neighbor_cell(x, z) {
            return Some(JobDestination::Neighbor { cell: (x, z) });
        }
        None
    }

    /// Clears the commute destination (`FailCommute`'s "reset the commute
    /// destination", §4.7).
    pub fn reset_commute(&self, sim: i32) {
        if let Some(state) = self.sims.borrow_mut().get_mut(&sim) {
            state.destination = None;
            state.destination_by_commute = false;
        }
    }

    /// The dispatch reports a trip state change (§3.3: `0x4BB2CFC0` passes
    /// (data3, data2) as (new, old), `0x4BB2D020` (−1, −1, data3, data2)).
    pub fn report_trip_state(&self, sim: i32, new_state: i32, old_state: i32) {
        {
            let mut sims = self.sims.borrow_mut();
            let state = sims.entry(sim).or_default();
            state.trip_state = new_state;
        }
        self.host.notify(
            sim,
            super::dispatch_note::TRIP_STATE_B,
            old_state as u32,
            new_state as u32,
        );
    }

    /// The dispatch reports a flags change (§3.3, the other half of the
    /// pair).
    pub fn report_flags(&self, sim: i32, new_flags: u32, old_flags: u32) {
        self.sims.borrow_mut().entry(sim).or_default().flags = new_flags;
        self.host
            .notify(sim, super::dispatch_note::TRIP_STATE_A, old_flags, new_flags);
    }

    /// The dispatch reports an arrival (§3.3 `0xEBA06C12`): the trip state
    /// is 3, at home or at work.
    pub fn report_arrival(&self, sim: i32, at_home: bool) {
        {
            let mut sims = self.sims.borrow_mut();
            let state = sims.entry(sim).or_default();
            state.trip_state = 3;
            state.destination_by_commute = false;
        }
        self.host
            .notify(sim, super::dispatch_note::ARRIVAL, u32::from(at_home), 0);
    }

    /// The dispatch reports a raw MySim state transition (`0xEBA06BD4` /
    /// `0x4BB71A7E`, §3.3).
    pub fn report_sim_state(&self, sim: i32, state: u32) {
        if (1..=6).contains(&state) {
            self.sims.borrow_mut().entry(sim).or_default().trip_state = state as i32;
        }
        self.host.notify(sim, super::dispatch_note::STATE_1, state, 0);
    }

    /// One pathfinder run (pathfinder.md §8's MySim dispatch setup):
    /// commuter travel-type mask, density 0, flags 1 | 2, filter 3, a route
    /// deque, `FindPath`, `Reset`.
    fn run_pathfinder(&self, home: CellRect, end: CellRect) -> Option<Found> {
        let mut pf = Pathfinder::create(&*self.host, self.shared.clone());
        pf.set_start_rect(home);
        pf.set_end_rect(end);
        pf.set_trip_destination_filter(filter::END_RECT);
        pf.set_flag(PATHFINDER_FLAGS);
        pf.set_allowed_travel_types(COMMUTER_TRAVEL_MASK);
        pf.set_starting_travel_types(COMMUTER_TRAVEL_MASK);
        pf.set_traffic_density(DENSITY);
        pf.set_max_work_time(MAX_WORK_TIME);
        pf.set_route_info(Some(VecDeque::new()));
        let found = pf.find_path_on(&self.net, &*self.host);
        let outcome = if found == result::FOUND {
            let (x, z) = pf.get_end_cell();
            Some(Found {
                cell: (x, z),
                trip_length: pf.get_trip_length(),
                building: self.host.building_at_cell(x, z),
            })
        } else {
            pf.reset();
            return None;
        };
        pf.reset();
        outcome
    }
}

/// One successful pathfinder run.
struct Found {
    cell: (i32, i32),
    trip_length: f32,
    building: Option<u64>,
}
