// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned runtime state. Fleet links refer only to other Rust owners, never
//! native watercraft tick/movement/collision results.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::automata::{State, Vec3};

use super::math;
use super::{DriveParameters, WatercraftRecord};

pub type BoatId = u64;

pub mod state {
    pub const IDLE: u32 = 7;
    pub const PLAYER_DRIVE: u32 = 16;
    pub const RUN_AGROUND: u32 = 70;
    pub const SINK: u32 = 71;
    pub const IMMEDIATE_REMOVE: u32 = 72;
    pub const SLOW_REMOVE: u32 = 73;
    pub const DOCK: u32 = 74;
    pub const QUEUE: u32 = 75;
    pub const PARKED: u32 = 76;
    pub const TOWED: u32 = 77;
    pub const UNDOCK: u32 = 78;
}

pub const TOW_MESSAGE: u32 = 0xAC40_0A31;
pub const RELOAD_MESSAGE: u32 = 0x26D3_1EC1;
pub const EFFECTS: [&str; 6] = [
    "SMLmushroomcloud",
    "boat_grounding",
    "boat_splash",
    "tugropethrow",
    "tugropeconnection",
    "waterski",
];
pub const WAKES: [&str; 6] = ["SMLWake", "SMLWash", "MEDWake", "MEDWash", "LargeWake", "LargeWash"];

/// Resource identity; owning an Arc takes precisely one Rust reference.
#[derive(Debug, PartialEq, Eq)]
pub struct Resource(pub u64);

#[derive(Clone, Debug)]
pub struct Group {
    pub id: u32,
    pub parameters: Option<[f32; 40]>,
    pub aems_value: Option<f32>,
    pub parent: Option<Arc<Group>>,
}

impl Group {
    pub fn drive_parameters(&self) -> DriveParameters {
        let mut group = Some(self);
        while let Some(current) = group {
            if let Some(values) = current.parameters.as_ref() {
                return DriveParameters::load(Some(values));
            }
            group = current.parent.as_deref();
        }
        DriveParameters::default()
    }

    pub fn sound_value(&self) -> Option<f32> {
        let mut group = Some(self);
        while let Some(current) = group {
            if let Some(value) = current.aems_value {
                return Some(value);
            }
            group = current.parent.as_deref();
        }
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub x: i32,
    pub z: i32,
}

#[derive(Clone, Debug)]
pub struct Station {
    pub lot: Arc<Resource>,
    pub occupant: Arc<Resource>,
    pub position: Vec3,
    pub direction: Vec3,
    /// Decoded network route data, not original watercraft route-pick output.
    /// Compressed-byte decoding is not specified (WC7).
    pub route: Arc<[Cell]>,
}

#[derive(Clone, Debug)]
pub struct NeighborRoute {
    pub occupant: Arc<Resource>,
    pub position: Vec3,
    pub cells: Arc<[Cell]>,
    pub next: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TravelPoint {
    pub cell: Cell,
    pub tried: u8,
    pub direction: u32,
}

/// Only foreign terrain operations; all movement decisions remain in Rust.
pub trait Terrain {
    fn contains(&self, cell: Cell) -> bool;
    fn cell_at(&self, position: Vec3) -> Cell;
    fn terrain_height(&self, position: Vec3) -> f32;
    fn sea_level(&self) -> f32;
    fn water_height(&self, position: Vec3) -> f32;
    fn navigable(&self, cell: Cell, radius: u32, lot: Option<&Resource>) -> bool;
}

/// The foreign watercraft manager owns berth arbitration and retention policy.
pub trait DockManager {
    fn take_berth(&mut self, lot: &Resource, boat: BoatId) -> bool;
    fn take_waiting_spot(&mut self, lot: &Resource, boat: BoatId) -> bool;
    fn done_docking(&mut self, lot: &Resource, boat: BoatId);
    fn may_stay(&mut self, boat: BoatId) -> bool;
}

pub trait EffectResources {
    fn create(&mut self, name: &'static str) -> Option<Arc<Resource>>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Register(u32),
    Unregister(u32),
    ActiveSound(bool),
    /// The adapter must stop the effect before dropping this owned reference.
    StopEffect(Arc<Resource>),
    StartWake(&'static str),
    StopWake(&'static str),
    Notify(u32, u32),
    Remove(bool),
    Aems(u32),
    PlayEffect(&'static str),
    ResetSpeedTween,
    BaseCommandTranslation,
    BaseShutdown,
    Publish(super::Publication),
    EffectUpdate,
    ModelUpdate,
    FindState(u32),
}

/// Unsupported owner calculations are explicit, never success-shaped stubs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractGap {
    UnknownBoat,
    DegenerateDirection,
    AiMotionWc4,
    CrashSequenceWc5,
    MissingStationWc7,
    MissingRouteWc7,
}

pub type Result<T> = std::result::Result<T, ContractGap>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerState {
    pub speed_tween: super::SpeedTween,
    pub field_30: f32,
    pub turn_rate: f32,
    pub aems_value: f32,
    pub field_144: u32,
    pub shots: u32,
    pub field_1ac: f32,
    pub skier_running: bool,
    pub gear: super::Gear,
}

/// Runtime-only data is deliberately separate from the saved base record.
#[derive(Debug)]
pub struct Watercraft {
    pub models: super::ModelStorage,
    pub crash_direction: [u8; 2],
    pub occupant_bounds: [Vec3; 2],
    pub model_geometry: Option<super::ModelGeometry>,
    pub footprint_scratch: super::Footprint,
    pub transform: super::Transform,
    pub inherited_position: Vec3,
    pub inherited_heading: Vec3,
    pub inherited_destination: Vec3,
    pub inherited_measured_speed: f32,
    pub record: WatercraftRecord,
    pub group: Arc<Group>,
    pub drive: DriveParameters,
    pub position: Vec3,
    pub previous_position: Vec3,
    pub heading: Vec3,
    pub previous_heading: Vec3,
    pub speed: f32,
    pub turn: f32,
    pub radius: f32,
    pub lifetime_limit: f32,
    pub lifetime_elapsed: f32,
    pub idle_elapsed: f32,
    pub crash_elapsed: f32,
    pub crash_angle: f32,
    pub shore_contact: bool,
    pub still_elapsed: f32,
    pub backwards_elapsed: f32,
    pub count_idle_always: bool,
    pub sunk_by_collision: bool,
    pub docking: bool,
    pub queued: bool,
    pub docking_in_progress: bool,
    pub being_towed: bool,
    pub tow_armed: bool,
    pub ai_turn_latch: bool,
    pub player: PlayerState,
    pub travel: Vec<TravelPoint>,
    pub stations: [Option<Arc<Station>>; 2],
    pub dock_requests: [bool; 2],
    pub current_station: Option<Arc<Station>>,
    pub route_start: Vec3,
    pub used_cells: usize,
    pub route_started: bool,
    pub ferry: bool,
    pub held_berth: bool,
    pub neighbor: Option<NeighborRoute>,
    pub towed: Option<BoatId>,
    pub tug: Option<BoatId>,
    pub effects: [Option<Arc<Resource>>; 6],
    pub events: Vec<Event>,
    pub(super) route_next: usize,
    pub(super) initialized: bool,
    pub(super) initialization_attempted: bool,
    pub(super) shut_down: bool,
}

impl Watercraft {
    pub fn class_id(&self) -> u32 {
        super::CLASS_ID
    }

    /// Constructs portable storage. This is not a claim about the unreported
    /// native constructor's 79-method inventory (WC3).
    pub fn new(mut record: WatercraftRecord, group: Arc<Group>, radius: f32) -> Self {
        record.base.occupant.flags |= 0x10;
        record.base.occupant.occupant_type = 0x897F_7EC2;
        Self {
            models: super::ModelStorage::default(),
            crash_direction: [0; 2],
            occupant_bounds: [[0.0; 3]; 2],
            model_geometry: None,
            footprint_scratch: super::Footprint::default(),
            transform: super::Transform::default(),
            inherited_position: [0.0; 3],
            inherited_heading: [0.0; 3],
            inherited_destination: [0.0; 3],
            inherited_measured_speed: 0.0,
            record,
            drive: group.drive_parameters(),
            group,
            position: [0.0; 3],
            previous_position: [0.0; 3],
            heading: [0.0; 3],
            previous_heading: [0.0; 3],
            speed: 0.0,
            turn: 0.0,
            radius,
            lifetime_limit: 0.0,
            lifetime_elapsed: 0.0,
            idle_elapsed: 0.0,
            crash_elapsed: 0.0,
            crash_angle: 0.0,
            shore_contact: false,
            still_elapsed: 0.0,
            backwards_elapsed: 0.0,
            count_idle_always: false,
            sunk_by_collision: false,
            docking: false,
            queued: false,
            docking_in_progress: false,
            being_towed: false,
            tow_armed: false,
            ai_turn_latch: false,
            player: PlayerState::default(),
            travel: vec![
                TravelPoint {
                    cell: Cell { x: 0, z: 0 },
                    tried: 0,
                    direction: 0
                };
                11
            ],
            stations: [None, None],
            dock_requests: [false; 2],
            current_station: None,
            route_start: [0.0; 3],
            used_cells: 0,
            route_started: false,
            ferry: false,
            held_berth: false,
            neighbor: None,
            towed: None,
            tug: None,
            effects: std::array::from_fn(|_| None),
            events: Vec::new(),
            route_next: 0,
            initialized: false,
            initialization_attempted: false,
            shut_down: false,
        }
    }

    pub fn top_state(&self) -> Option<u32> {
        self.record.base.states.last().map(|s| s.state_type)
    }

    pub fn has_state(&self, state: u32) -> bool {
        self.record.base.states.iter().any(|s| s.state_type == state)
    }

    pub fn load_drive_parameters(&mut self) {
        self.drive = self.group.drive_parameters();
    }

    /// The specified owner portion of Init, after the inherited Init.
    /// Required inherited Init precedes this; its result is ignored (§3).
    pub fn initialize_runtime(
        &mut self,
        position: Vec3,
        heading: Vec3,
        effects: &mut dyn EffectResources,
    ) -> Result<bool> {
        if self.initialized {
            return Ok(true);
        }
        let heading = math::normalize(heading);
        self.initialization_attempted = true;
        self.load_drive_parameters();
        self.effects = EFFECTS.map(|name| effects.create(name));
        // All six factory results are ignored, including absent effects (§3).
        self.record.base.automata_flags |= 0x8000 | 0x2000 | 0x20000 | 4;
        self.events.push(Event::Register(TOW_MESSAGE));
        self.towed = None;
        self.tug = None;
        self.tow_armed = false;
        self.stations = [None, None];
        self.dock_requests = [false; 2];
        self.current_station = None;
        self.travel.clear();
        self.lifetime_limit = 0.0;
        self.lifetime_elapsed = 0.0;
        self.idle_elapsed = 0.0;
        self.crash_elapsed = 0.0;
        self.still_elapsed = 0.0;
        self.backwards_elapsed = 0.0;
        self.speed = 0.0;
        self.turn = 0.0;
        self.count_idle_always = false;
        self.position = position;
        self.previous_position = position;
        self.heading = heading;
        self.previous_heading = heading;
        self.inherited_position = position;
        self.inherited_heading = heading;
        self.events.push(Event::ActiveSound(true));
        self.initialized = true;
        Ok(true)
    }

    pub fn on_land(&self, terrain: &dyn Terrain) -> bool {
        terrain.contains(terrain.cell_at(self.position))
            && terrain.terrain_height(self.position) > terrain.water_height(self.position)
    }

    pub fn other_station(&self) -> Option<Arc<Station>> {
        let a = self.stations[0].as_ref()?;
        let index = usize::from(
            self.route_start
                .iter()
                .zip(a.position)
                .all(|(x, y)| (*x - y).abs() <= 0.0001),
        );
        self.stations[index].clone()
    }

    pub fn initialize_current_route(&mut self, station: Arc<Station>) {
        self.route_start = station.position;
        self.current_station = Some(station);
        self.held_berth = true;
        if self.used_cells == 0 {
            self.used_cells = 2;
        }
        self.route_started = false;
        self.route_next = 0;
        self.travel.clear();
    }

    pub fn set_station_information(&mut self, stations: [Arc<Station>; 2], start: bool) {
        self.stations = stations.map(Some);
        if start {
            self.initialize_current_route(self.stations[0].as_ref().unwrap().clone());
            self.docking_in_progress = true;
        }
    }

    pub fn set_neighbor_route(&mut self, mut route: NeighborRoute, cell_position: impl Fn(Cell) -> Vec3) {
        route.next = 0;
        while route.next < route.cells.len() {
            let point = cell_position(route.cells[route.next]);
            route.next += 1;
            // The first point beyond the skip radius is consumed too (§5.4.1).
            if math::length(math::sub(point, route.position)) > 3.0 * self.radius {
                break;
            }
        }
        self.neighbor = Some(route);
    }

    pub fn debug_extra_text(&self, counts: [i32; 2], cell: Cell) -> String {
        format!(
            "Tow: {}, (FT,MT): ({},{}), (Q,D): ({},{}), Turn: {:3.2}, Speed: {:3.2}, Cell({:3},{:3}), Pos({:3.2},{:3.2},{:3.2}), Ornt<{:3.2},{:3.2},{:3.2}>",
            i32::from(self.towed.is_some()), counts[0], counts[1],
            i32::from(self.queued), i32::from(self.docking), self.turn, self.speed,
            cell.x, cell.z, self.position[0], self.position[1], self.position[2],
            self.heading[0], self.heading[1], self.heading[2],
        )
    }
}

/// Fleet storage owns boats. Tow links are non-owning IDs, so two-sided links
/// cannot create an Arc cycle; shutdown/exit explicitly clears both ends.
#[derive(Default)]
pub struct Fleet {
    pub(super) boats: BTreeMap<BoatId, Watercraft>,
    pub(super) order: Vec<BoatId>,
}

impl Fleet {
    pub(super) fn boat(&self, id: BoatId) -> Result<&Watercraft> {
        self.boats.get(&id).ok_or(ContractGap::UnknownBoat)
    }

    pub(super) fn boat_mut(&mut self, id: BoatId) -> Result<&mut Watercraft> {
        self.boats.get_mut(&id).ok_or(ContractGap::UnknownBoat)
    }

    pub(super) fn push_raw(&mut self, id: BoatId, mut state: State) -> Result<()> {
        let boat = self.boat_mut(id)?;
        state.base.stack_index = boat.record.base.states.len() as u32;
        boat.record.base.states.push(state);
        Ok(())
    }
}
