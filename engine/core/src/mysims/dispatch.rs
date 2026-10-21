// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4MySimDispatch` (class `0xCBC14674`, read as IID `0xEB8B6F9A`), the
//! per-MySim trip controller. mysims.md §1.1 leaves the class itself to the
//! automata spec, but the simulation side names its contract:
//!
//! - `FindNewJob` → `SetCurrentCommuteDestination(default loc, notify)` →
//!   `validateCommuteLocation` (§4.7.1): one filter-0 search on the
//!   dispatch's **long-lived** pathfinder (flags `0x11`, destination flags by
//!   wealth OR-accumulated *[quirk]*, density 1, home lot rect, travel
//!   `0x1CF` allowed and starting, one `FindPath`), then the end cell is
//!   resolved to a city lot's building or a neighbour city.
//! - `ChangeJob(b)` sets the commute destination to building `b`; the
//!   result is the dispatch's (§4.7.1).
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

use crate::pathfinder::{filter, flag, result, CellRect, Pathfinder, PathfinderHost, RouteData, SharedWorkCheck};
use crate::traffic_network::NetworkModel;

use super::{JobDestination, DISPATCH_CLSID, DISPATCH_IID};

/// The commuter travel-type mask of the job search (§4.7.1 step 8: Walk,
/// Car, Bus, Passenger train, Subway, Elevated train, Monorail; allowed
/// **and** starting).
pub const COMMUTER_TRAVEL_MASK: u16 = 0x1CF;
/// §4.7.1 step 3: `SetFlag(1)` then `SetFlag(0x10)`.
pub const JOB_SEARCH_FLAGS: [u32; 2] = [flag::NO_BUDGET, flag::NO_FERRIES];
/// §4.7.1 step 6.
pub const JOB_SEARCH_DENSITY: u32 = 1;
/// Dispatch flag `0x80`: a valid default destination (home) is set
/// (`SetDefaultDestination`, §4.7.1 step 2).
pub const FLAG_HAS_HOME: u32 = 0x80;
/// The dispatch flags `resetPath` clears (§4.7.1 step 0).
pub const RESET_PATH_FLAGS: u32 = 0x800 | 0x1000 | 0x4000;
/// The dispatch state (`+0x40`) at which an invalid location is stored
/// unvalidated (§4.7.1 step 2).
pub const STATE_STORE_UNVALIDATED: i32 = 2;

/// The `CheckCommute` pathfinder (pathfinder.md §8's MySim dispatch row:
/// flags 1 | 2, filter 3, density 0). Not part of §4.7.1; unchanged.
const COMMUTE_FLAGS: u32 = flag::NO_BUDGET | flag::NEAREST_ON_FAILURE;
const COMMUTE_DENSITY: u32 = 0;
const MAX_WORK_TIME: u32 = 0x1000;

/// `SetDestinationFlag` value by the MySim's `GetWealth()` (§4.7.1 step 5):
/// 1 → `0x1`, 2 → `0x2`, 3 → `0x4`, anything else → `0x1`.
pub fn destination_flag_for_wealth(wealth: u32) -> u8 {
    match wealth {
        2 => 0x2,
        3 => 0x4,
        _ => 0x1,
    }
}

/// A dispatch location (home `+0x84`, commute destination `+0xD4`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DispatchLocation {
    /// The occupant (`SetByOccupant`).
    pub occupant: Option<u64>,
    /// The occupant flag: the location is an occupant, not a cell.
    pub by_occupant: bool,
    /// Byte `+0x1D`, "by commute".
    pub by_commute: bool,
    /// The cell (`SetCellPosition`); −1, −1 by default.
    pub cell: (i32, i32),
    /// The neighbour-city fields (`+0x14`, `+0x18`): the regional city's
    /// position. The default bytes are not specified (question MS11):
    /// `None` = never written by this code.
    pub neighbor_city: Option<(i32, i32)>,
}

impl Default for DispatchLocation {
    /// The default location (§4.7.1): occupant flag set, no occupant,
    /// cells −1 → invalid.
    fn default() -> Self {
        DispatchLocation {
            occupant: None,
            by_occupant: true,
            by_commute: false,
            cell: (-1, -1),
            neighbor_city: None,
        }
    }
}

impl DispatchLocation {
    /// A location by occupant (`SetByOccupant`).
    pub fn by_occupant(occupant: u64) -> Self {
        DispatchLocation {
            occupant: Some(occupant),
            ..DispatchLocation::default()
        }
    }

    /// A location by cell (occupant flag 0, `SetCellPosition`).
    pub fn by_cell(x: i32, z: i32) -> Self {
        DispatchLocation {
            by_occupant: false,
            cell: (x, z),
            ..DispatchLocation::default()
        }
    }

    /// Validity. §4.7.1 only says the default location (occupant flag set,
    /// cells −1) is invalid; the literal reading used here *[question MS11]*:
    /// an occupant location is valid when it holds an occupant, a cell
    /// location when both cells are ≥ 0.
    pub fn is_valid(&self) -> bool {
        if self.by_occupant {
            self.occupant.is_some()
        } else {
            self.cell.0 >= 0 && self.cell.1 >= 0
        }
    }
}

/// The lot at an end cell (§4.7.1 after `FindPath`): `GetLot(x, z, false)`
/// and the lot's `GetBuilding()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LotAt {
    NoLot,
    NoBuilding,
    Building(u64),
}

/// The pathfinder slots the job search calls (§4.7.1), so a recording
/// pathfinder can stand in for the core one (conformance).
pub trait JobPathfinder {
    /// `Reset` (slot 22).
    fn reset(&mut self);
    /// `ClearAllTravelTypeFlags` (slot 15).
    fn clear_all_travel_type_flags(&mut self);
    /// `SetRouteInfo(route deque)` (slot 8).
    fn set_route_info(&mut self, route: VecDeque<RouteData>);
    /// `SetFlag` (slot 5), OR.
    fn set_flag(&mut self, flags: u32);
    /// `SetTripDestinationFilter` (slot 9).
    fn set_trip_destination_filter(&mut self, filter: u32);
    /// `SetDestinationFlag` (slot 10), OR.
    fn set_destination_flag(&mut self, bits: u8);
    /// `SetTrafficDensity` (slot 18).
    fn set_traffic_density(&mut self, trips: u32);
    /// `SetStartRect` (slot 19).
    fn set_start_rect(&mut self, rect: CellRect);
    /// `SetTravelTypeFlag(mask, alsoStarting)` (slot 13).
    fn set_travel_type_flag(&mut self, mask: u16, also_starting: bool);
    /// `FindPath` (slot 23): 1 = found.
    fn find_path(&mut self) -> u32;
    /// `GetEndCell` (slot 27).
    fn get_end_cell(&self) -> (i32, i32);
}

/// What the job search reads besides the pathfinder (§4.7.1). Typed host
/// input; no field is derived here.
pub trait JobSearchWorld {
    /// The MySim's `GetWealth()` (cISC4MySim slot 19).
    fn wealth(&self) -> u32;
    /// The bounding rect of an occupant's lot; `None` = no lot.
    fn occupant_lot_rect(&self, occupant: u64) -> Option<CellRect>;
    /// The bounding rect of the lot at a cell; `None` = no lot.
    fn lot_rect_at(&self, x: i32, z: i32) -> Option<CellRect>;
    /// `cISC4City::CellIsInBounds` (`+0x18C`).
    fn cell_is_in_bounds(&self, x: i32, z: i32) -> bool;
    /// The city's cell counts (`+0x174`, `+0x178`).
    fn cell_counts(&self) -> (i32, i32);
    /// `GetLot(x, z, false)` and the lot's `GetBuilding()`.
    fn lot_at(&self, x: i32, z: i32) -> LotAt;
    /// `FindCityOnOtherSideOfConnection(x, z)`: the regional city's
    /// position (its slot 5); `None` = no city.
    fn city_across_connection(&self, x: i32, z: i32) -> Option<(i32, i32)>;
}

/// The core pathfinder as a [`JobPathfinder`].
pub struct CoreJobPathfinder<'a> {
    pub pf: &'a mut Pathfinder,
    pub net: &'a NetworkModel,
    pub host: &'a dyn PathfinderHost,
}

impl JobPathfinder for CoreJobPathfinder<'_> {
    fn reset(&mut self) {
        self.pf.reset();
    }
    fn clear_all_travel_type_flags(&mut self) {
        self.pf.clear_all_travel_type_flags();
    }
    fn set_route_info(&mut self, route: VecDeque<RouteData>) {
        self.pf.set_route_info(Some(route));
    }
    fn set_flag(&mut self, flags: u32) {
        self.pf.set_flag(flags);
    }
    fn set_trip_destination_filter(&mut self, f: u32) {
        self.pf.set_trip_destination_filter(f);
    }
    fn set_destination_flag(&mut self, bits: u8) {
        self.pf.set_destination_flag(bits);
    }
    fn set_traffic_density(&mut self, trips: u32) {
        self.pf.set_traffic_density(trips);
    }
    fn set_start_rect(&mut self, rect: CellRect) {
        self.pf.set_start_rect(rect);
    }
    fn set_travel_type_flag(&mut self, mask: u16, also_starting: bool) {
        self.pf.set_travel_type_flag(mask, also_starting);
    }
    fn find_path(&mut self) -> u32 {
        self.pf.find_path_on(self.net, self.host)
    }
    fn get_end_cell(&self) -> (i32, i32) {
        self.pf.get_end_cell()
    }
}

/// The dispatch fields the job search reads and writes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JobSearchState {
    /// Dispatch flags (`0x80` home set, `0x800|0x1000|0x4000` path).
    pub flags: u32,
    /// Dispatch state `+0x40` (§4.7.1 step 2). Whether it is the trip state
    /// of §4.6 is not specified (question MS11), so it is its own field.
    pub state: i32,
    /// The home location (`+0x84`, `SetDefaultDestination`).
    pub home: DispatchLocation,
    /// The commute destination (`+0xD4`).
    pub commute_destination: DispatchLocation,
    /// The route deque handed to `SetRouteInfo`.
    pub route: VecDeque<RouteData>,
    /// The last end cell the search resolved (for the job-found report).
    pub end_cell: (i32, i32),
}

/// `resetPath` (§4.7.1 step 0).
pub fn reset_path(st: &mut JobSearchState, pf: &mut dyn JobPathfinder) {
    st.route.clear();
    st.flags &= !RESET_PATH_FLAGS;
    pf.reset();
    pf.clear_all_travel_type_flags();
    pf.set_route_info(st.route.clone());
}

/// The cell rect of the home location (§4.7.1 step 7): the lot of its
/// occupant (or the lot at its cell) → the lot's bounding rect; no lot →
/// the single cell.
fn home_start_rect(home: &DispatchLocation, world: &dyn JobSearchWorld) -> CellRect {
    let lot = match (home.by_occupant, home.occupant) {
        (true, Some(occ)) => world.occupant_lot_rect(occ),
        _ => world.lot_rect_at(home.cell.0, home.cell.1),
    };
    lot.unwrap_or(CellRect::new(home.cell.0, home.cell.1, home.cell.0, home.cell.1))
}

/// `validateCommuteLocation(loc)` (§4.7.1, `0x00432F50`).
pub fn validate_commute_location(
    st: &mut JobSearchState,
    pf: &mut dyn JobPathfinder,
    world: &dyn JobSearchWorld,
    loc: &mut DispatchLocation,
) -> bool {
    reset_path(st, pf);
    *loc = DispatchLocation::default();
    loc.by_commute = true;
    if st.flags & FLAG_HAS_HOME == 0 {
        return false;
    }
    for f in JOB_SEARCH_FLAGS {
        pf.set_flag(f);
    }
    pf.set_trip_destination_filter(filter::STANDARD);
    pf.set_destination_flag(destination_flag_for_wealth(world.wealth()));
    pf.set_traffic_density(JOB_SEARCH_DENSITY);
    pf.set_start_rect(home_start_rect(&st.home, world));
    pf.set_travel_type_flag(COMMUTER_TRAVEL_MASK, true);
    if pf.find_path() != result::FOUND {
        return false;
    }
    let (x, z) = pf.get_end_cell();
    st.end_cell = (x, z);
    if world.cell_is_in_bounds(x, z) {
        let LotAt::Building(b) = world.lot_at(x, z) else {
            return false;
        };
        loc.occupant = Some(b);
        loc.by_occupant = true;
        true
    } else {
        let Some(city) = world.city_across_connection(x, z) else {
            return false;
        };
        let (cx, cz) = world.cell_counts();
        loc.occupant = None;
        loc.by_occupant = false;
        loc.cell = (x.clamp(0, (cx - 1).max(0)), z.clamp(0, (cz - 1).max(0)));
        loc.neighbor_city = Some(city);
        true
    }
}

/// `SetCurrentCommuteDestination(loc, notify)` (§4.7.1, `0x004350D0`):
/// `(result, notify the MySim with 0xEBA06BF4)`.
pub fn set_current_commute_destination(
    st: &mut JobSearchState,
    pf: &mut dyn JobPathfinder,
    world: &dyn JobSearchWorld,
    mut loc: DispatchLocation,
    notify: bool,
) -> (bool, bool) {
    if !loc.is_valid() {
        if !notify {
            return (false, false);
        }
        if st.state != STATE_STORE_UNVALIDATED && !validate_commute_location(st, pf, world, &mut loc) {
            return (false, false);
        }
    }
    st.commute_destination = loc;
    (true, notify)
}

/// What the dispatch reads from the game beyond the pathfinder host: the
/// MySim's home, wealth and job lots, the city, and a way to post the
/// notification messages. Callbacks must not call back into the dispatch.
pub trait DispatchHost: PathfinderHost {
    /// The home lot's bounding rectangle (`CheckCommute`).
    fn home_rect(&self, sim: i32) -> Option<CellRect>;
    /// The lot rectangle of a building the commute is aimed at
    /// (`CheckCommute`); `None` when there is no such building.
    fn building_rect(&self, building: u64) -> Option<CellRect>;
    /// The MySim's `GetWealth()` (§4.7.1 step 5).
    fn wealth(&self, sim: i32) -> u32;
    /// The bounding rect of an occupant's lot.
    fn occupant_lot_rect(&self, occupant: u64) -> Option<CellRect>;
    /// The bounding rect of the lot at a cell.
    fn lot_rect_at(&self, x: i32, z: i32) -> Option<CellRect>;
    /// `cISC4City::CellIsInBounds`.
    fn cell_is_in_bounds(&self, x: i32, z: i32) -> bool;
    /// `GetLot(x, z, false)` → `GetBuilding()`.
    fn lot_at_cell(&self, x: i32, z: i32) -> LotAt;
    /// `FindCityOnOtherSideOfConnection(x, z)` → the city's position.
    fn city_across_connection(&self, x: i32, z: i32) -> Option<(i32, i32)>;
    /// Posts a `0x2BA06B99` notification at the MySim.
    fn notify(&self, sim: i32, sub_id: u32, data1: u32, data2: u32);
}

/// One MySim's [`DispatchHost`] as a [`JobSearchWorld`].
struct SimWorld<'a> {
    host: &'a dyn DispatchHost,
    sim: i32,
}

impl JobSearchWorld for SimWorld<'_> {
    fn wealth(&self) -> u32 {
        self.host.wealth(self.sim)
    }
    fn occupant_lot_rect(&self, occupant: u64) -> Option<CellRect> {
        self.host.occupant_lot_rect(occupant)
    }
    fn lot_rect_at(&self, x: i32, z: i32) -> Option<CellRect> {
        self.host.lot_rect_at(x, z)
    }
    fn cell_is_in_bounds(&self, x: i32, z: i32) -> bool {
        self.host.cell_is_in_bounds(x, z)
    }
    fn cell_counts(&self) -> (i32, i32) {
        PathfinderHost::cell_counts(self.host)
    }
    fn lot_at(&self, x: i32, z: i32) -> LotAt {
        self.host.lot_at_cell(x, z)
    }
    fn city_across_connection(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        self.host.city_across_connection(x, z)
    }
}

/// One MySim's dispatch state: the fields the simulation side reads (§4.6)
/// plus the job-search fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DispatchState {
    /// 2 travelling, 3 at a location (§4.6).
    pub trip_state: i32,
    /// The job search's dispatch fields (§4.7.1).
    pub search: JobSearchState,
    /// The last pathfinder run's total time (§4.7 `CheckCommute`).
    pub trip_length: f32,
}

/// The per-MySim trip controller.
pub struct MySimDispatch {
    host: Rc<dyn DispatchHost>,
    net: Rc<NetworkModel>,
    shared: SharedWorkCheck,
    sims: RefCell<BTreeMap<i32, DispatchState>>,
    /// The long-lived job-search pathfinder of each dispatch (created in
    /// `Init`, §4.7.1).
    pathfinders: RefCell<BTreeMap<i32, Pathfinder>>,
}

impl MySimDispatch {
    pub fn new(host: Rc<dyn DispatchHost>, net: Rc<NetworkModel>, shared: SharedWorkCheck) -> Rc<MySimDispatch> {
        Rc::new(MySimDispatch {
            host,
            net,
            shared,
            sims: RefCell::new(BTreeMap::new()),
            pathfinders: RefCell::new(BTreeMap::new()),
        })
    }

    /// The class and interface IDs (§1.1).
    pub fn class_id(&self) -> u32 {
        DISPATCH_CLSID
    }

    pub fn interface_id(&self) -> u32 {
        DISPATCH_IID
    }

    /// `Init` (clock24.md: only takes and drops the clock reference). The
    /// job-search pathfinder is created here once and kept for the
    /// dispatch's lifetime (§4.7.1).
    pub fn init(&self, sim: i32) -> bool {
        self.sims.borrow_mut().entry(sim).or_default();
        self.pathfinders
            .borrow_mut()
            .entry(sim)
            .or_insert_with(|| Pathfinder::create(&*self.host, self.shared.clone()));
        true
    }

    /// `Shutdown` (the MySim's Shutdown and LeaveCity call it, §4.7).
    pub fn shutdown(&self, sim: i32) {
        self.sims.borrow_mut().remove(&sim);
        self.pathfinders.borrow_mut().remove(&sim);
    }

    /// The job-search pathfinder's configuration as it stands (for tests
    /// and shadows): `(flags, filter, destination flags, density, start
    /// rect, allowed, starting)`.
    pub fn job_pathfinder_config(&self, sim: i32) -> Option<(u32, u32, u8, u32, CellRect, u16, u16)> {
        self.pathfinders.borrow().get(&sim).map(|pf| {
            (
                pf.flags(),
                pf.trip_destination_filter(),
                pf.destination_flags(),
                pf.traffic_density(),
                pf.start_rect(),
                pf.allowed_travel_types(),
                pf.starting_travel_types(),
            )
        })
    }

    /// `SetDefaultDestination(home)`: the home location; sets dispatch
    /// flag `0x80` (§4.7.1 step 2).
    pub fn set_default_destination(&self, sim: i32, home: DispatchLocation) {
        let mut sims = self.sims.borrow_mut();
        let st = &mut sims.entry(sim).or_default().search;
        st.home = home;
        st.flags |= FLAG_HAS_HOME;
    }

    /// Writes the dispatch state `+0x40` (§4.7.1 step 2).
    pub fn set_dispatch_state(&self, sim: i32, state: i32) {
        self.sims.borrow_mut().entry(sim).or_default().search.state = state;
    }

    /// `SetCurrentCommuteDestination(loc, notify)` (§4.7.1). On success with
    /// `notify`, the MySim is notified with `0xEBA06BF4`. A dispatch never
    /// `Init`ed has no pathfinder: an invalid location then fails
    /// *[choice]* (the original always has one after `Init`).
    pub fn set_current_commute_destination(&self, sim: i32, loc: DispatchLocation, notify: bool) -> bool {
        let (ok, fire) = {
            let mut pfs = self.pathfinders.borrow_mut();
            let mut sims = self.sims.borrow_mut();
            let st = &mut sims.entry(sim).or_default().search;
            let world = SimWorld { host: &*self.host, sim };
            match pfs.get_mut(&sim) {
                Some(pf) => {
                    let mut core = CoreJobPathfinder {
                        pf,
                        net: &self.net,
                        host: &*self.host,
                    };
                    set_current_commute_destination(st, &mut core, &world, loc, notify)
                }
                None if loc.is_valid() => {
                    st.commute_destination = loc;
                    (true, notify)
                }
                None => (false, false),
            }
        };
        if fire {
            self.host.notify(sim, super::dispatch_note::JOB_FOUND, 0, 0);
        }
        ok
    }

    /// The state the simulation side reads (§4.6 trip variables).
    pub fn get_state(&self, sim: i32) -> DispatchState {
        self.sims.borrow().get(&sim).cloned().unwrap_or_default()
    }

    /// Tests the dispatch's flags (§4.1 step 3 tracking flag): all bits of
    /// `mask` are set.
    pub fn check_flags(&self, sim: i32, mask: u32) -> bool {
        self.get_state(sim).search.flags & mask == mask
    }

    /// `FindNewJob` (§4.7.1, `0x00425A30`): a default location marked "by
    /// commute" through `SetCurrentCommuteDestination(loc, notify = true)`.
    /// The destination is reported by [`Self::job_destination`] when the
    /// `0xEBA06BF4` notification fires (the MySim's job_found).
    pub fn find_new_job(&self, sim: i32) -> bool {
        let loc = DispatchLocation {
            by_commute: true,
            ..DispatchLocation::default()
        };
        self.set_current_commute_destination(sim, loc, true)
    }

    /// `ChangeJob(b)` (§4.7.1): set the commute destination to building
    /// `b`; the result of the dispatch. The `notify` argument of this call
    /// is not specified (question MS11): false is used, so a valid building
    /// is stored without a search and without `0xEBA06BF4`.
    pub fn change_job(&self, sim: i32, building: u64) -> bool {
        self.set_current_commute_destination(sim, DispatchLocation::by_occupant(building), false)
    }

    /// `CheckCommute` (§4.7): the total time of home → job (or the
    /// neighbour cell), `None` on failure (the MySim then runs
    /// `FailCommute`).
    pub fn check_commute(&self, sim: i32) -> Option<f32> {
        let home = self.host.home_rect(sim)?;
        let dest = self.get_state(sim).search.commute_destination;
        let end = match (dest.by_occupant, dest.occupant) {
            (true, Some(building)) => self.host.building_rect(building)?,
            (true, None) => return None,
            (false, _) => CellRect::new(dest.cell.0, dest.cell.1, dest.cell.0, dest.cell.1),
        };
        let found = self.run_pathfinder(home, end)?;
        self.sims.borrow_mut().entry(sim).or_default().trip_length = found;
        Some(found)
    }

    /// The destination the `0xEBA06BF4` notification reports (§4.7): a city
    /// lot's building, or the neighbour city at the reached cell.
    pub fn job_destination(&self, sim: i32) -> Option<JobDestination> {
        let st = self.get_state(sim).search;
        let dest = st.commute_destination;
        match (dest.by_occupant, dest.occupant) {
            // The lot's cell is the search's end cell (what `SetByOccupant`
            // stores as the cell is not specified, question MS11).
            (true, Some(building)) => Some(JobDestination::Lot {
                building,
                cell: st.end_cell,
            }),
            (false, _) if dest.is_valid() => Some(JobDestination::Neighbor { cell: dest.cell }),
            _ => None,
        }
    }

    /// Clears the commute destination (`FailCommute`'s "reset the commute
    /// destination", §4.7).
    pub fn reset_commute(&self, sim: i32) {
        if let Some(state) = self.sims.borrow_mut().get_mut(&sim) {
            state.search.commute_destination = DispatchLocation::default();
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
        self.sims.borrow_mut().entry(sim).or_default().search.flags = new_flags;
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
            state.search.commute_destination.by_commute = false;
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
    fn run_pathfinder(&self, home: CellRect, end: CellRect) -> Option<f32> {
        let mut pf = Pathfinder::create(&*self.host, self.shared.clone());
        pf.set_start_rect(home);
        pf.set_end_rect(end);
        pf.set_trip_destination_filter(filter::END_RECT);
        pf.set_flag(COMMUTE_FLAGS);
        pf.set_allowed_travel_types(COMMUTER_TRAVEL_MASK);
        pf.set_starting_travel_types(COMMUTER_TRAVEL_MASK);
        pf.set_traffic_density(COMMUTE_DENSITY);
        pf.set_max_work_time(MAX_WORK_TIME);
        pf.set_route_info(Some(VecDeque::new()));
        let found = pf.find_path_on(&self.net, &*self.host);
        let outcome = (found == result::FOUND).then(|| pf.get_trip_length());
        pf.reset();
        outcome
    }
}

/// A [`JobPathfinder`] that records its configuration and the slot calls,
/// with a scripted `FindPath` answer (tests, conformance). The OR
/// semantics of slots 5, 10 and 13 are the pathfinder's (pathfinder.md
/// §4.2); `Reset` does not clear them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RecordingJobPathfinder {
    pub flags: u32,
    pub filter: u32,
    pub dest_flags: u8,
    pub density: u32,
    pub start_rect: Option<CellRect>,
    pub allowed: u16,
    pub starting: u16,
    /// Slot names in call order.
    pub calls: Vec<&'static str>,
    /// What `FindPath` answers.
    pub find_result: u32,
    /// What `GetEndCell` answers.
    pub end_cell: (i32, i32),
}

impl JobPathfinder for RecordingJobPathfinder {
    fn reset(&mut self) {
        self.calls.push("Reset");
    }
    fn clear_all_travel_type_flags(&mut self) {
        self.calls.push("ClearAllTravelTypeFlags");
        self.allowed = 0;
        self.starting = 0;
    }
    fn set_route_info(&mut self, _route: VecDeque<RouteData>) {
        self.calls.push("SetRouteInfo");
    }
    fn set_flag(&mut self, flags: u32) {
        self.calls.push("SetFlag");
        self.flags |= flags;
    }
    fn set_trip_destination_filter(&mut self, f: u32) {
        self.calls.push("SetTripDestinationFilter");
        self.filter = f;
    }
    fn set_destination_flag(&mut self, bits: u8) {
        self.calls.push("SetDestinationFlag");
        self.dest_flags |= bits;
    }
    fn set_traffic_density(&mut self, trips: u32) {
        self.calls.push("SetTrafficDensity");
        self.density = trips;
    }
    fn set_start_rect(&mut self, rect: CellRect) {
        self.calls.push("SetStartRect");
        self.start_rect = Some(rect);
    }
    fn set_travel_type_flag(&mut self, mask: u16, also_starting: bool) {
        self.calls.push("SetTravelTypeFlag");
        self.allowed |= mask;
        if also_starting {
            self.starting |= mask;
        }
    }
    fn find_path(&mut self) -> u32 {
        self.calls.push("FindPath");
        self.find_result
    }
    fn get_end_cell(&self) -> (i32, i32) {
        self.end_cell
    }
}
