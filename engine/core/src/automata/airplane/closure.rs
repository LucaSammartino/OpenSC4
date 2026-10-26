// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded stages specified by aircraft.md §§3.2.1, 3.3.2–3 and 4.3.3–4.4.

use super::{length, Airplane, Vec3};
use crate::automata::{aircraft_altitude, clip_aircraft_to_world, AircraftBounds, AircraftWorld};
use crate::automata::{DriveParameterSource, FlightPhase, SoundRequest};

pub trait RunwayTerrain: AircraftWorld {
    fn accepts_point(&mut self, x: f32, z: f32) -> bool;
    fn auxiliary_height(&mut self, mode: u32, x: f32, z: f32) -> f32;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlightCalibration {
    pub orientation: Vec3,
    pub velocity: Vec3,
    pub controls: [f32; 6],
    pub seconds: f32,
    pub maximum_iterations: u32,
}
impl Airplane {
    /// §3.2.1 synthetic seeds; convergence/attitude remains the §9.1 boundary.
    pub fn calibration_inputs(&self) -> [FlightCalibration; 3] {
        let angle = f64::from(self.drive_parameters.0[4]);
        let orientation = [0.0, angle.sin() as f32, angle.cos() as f32];
        let velocity = orientation.map(|v| (f64::from(v) * f64::from(self.drive_parameters.0[2])) as f32);
        [
            [1.0, 0.0, 0.0, 1.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0, self.player_gains[7], 1.0, 0.0],
            [self.player_gains[8], 1.0, 1.0, self.player_gains[7], 1.0, 0.0],
        ]
        .map(|controls| FlightCalibration {
            orientation,
            velocity,
            controls,
            seconds: 0.1,
            maximum_iterations: 100,
        })
    }
}

pub fn evaluate_clearance<W: RunwayTerrain>(world: &mut W, origin: Vec3, direction: Vec3) -> f32 {
    let mut point = origin;
    let mut score = 0.0_f32;
    for sample in 1..=20 {
        point = std::array::from_fn(|i| (f64::from(point[i]) + 16.0 * f64::from(direction[i])) as f32);
        let accepted = world.accepts_point(point[0], point[2]);
        let mut height = aircraft_altitude(world, point[0], point[2]);
        if accepted {
            height = world.auxiliary_height(1, point[0], point[2]);
        }
        let slope = (f64::from(height) - f64::from(origin[1])) / f64::from(16 * sample);
        if slope > f64::from(score) {
            score = slope as f32;
        }
    }
    score
}

pub trait RunwayRandom {
    /// Service draws must be in [0,count) and [0,total], respectively.
    fn uniform_index(&mut self, count: usize) -> usize;
    fn uniform_weight(&mut self, total: f64) -> f64;
}

pub fn weighted_endpoint(weights: &[f64], random: &mut dyn RunwayRandom) -> usize {
    assert!(!weights.is_empty());
    let sum: f64 = weights.iter().copied().filter(|w| *w > 0.0).sum();
    if sum == 0.0 {
        return random.uniform_index(weights.len());
    }
    let mut remaining = random.uniform_weight(sum);
    for (index, weight) in weights.iter().copied().enumerate() {
        if weight > 0.0 {
            if weight > remaining {
                return index;
            }
            remaining -= weight;
        }
    }
    weights.len() - 1
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RunwaySelection {
    pub start: Vec3,
    pub direction: Vec3,
    pub other_endpoint: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RunwayResult {
    RemoveOwner(u32),
    Selected(RunwaySelection),
}

pub fn pick_runway<W: RunwayTerrain>(
    world: &mut W,
    endpoints: &[Vec3],
    final_flag: bool,
    allowances: [f32; 2],
    random: &mut dyn RunwayRandom,
) -> RunwayResult {
    if endpoints.len() < 2 || endpoints.len() % 2 != 0 {
        return RunwayResult::RemoveOwner(0);
    }
    let allowance = allowances[usize::from(final_flag)];
    let mut weights = Vec::with_capacity(endpoints.len());
    for pair in endpoints.chunks_exact(2) {
        let delta = [pair[1][0] - pair[0][0], 0.0, pair[1][2] - pair[0][2]];
        let size = length(delta);
        let direction = delta.map(|v| (f64::from(v) / size) as f32);
        for (origin, direction) in [(pair[0], direction), (pair[1], direction.map(|v| -v))] {
            let excess = (f64::from(evaluate_clearance(world, origin, direction)) - f64::from(allowance)) as f32;
            weights.push(f64::from(if excess > 0.0 { excess } else { 0.0 }));
        }
    }
    let mut start = weighted_endpoint(&weights, random);
    let mut end = start ^ 1;
    if !final_flag {
        std::mem::swap(&mut start, &mut end);
    }
    let delta = std::array::from_fn(|i| endpoints[end][i] - endpoints[start][i]);
    let size = length(delta);
    RunwayResult::Selected(RunwaySelection {
        start: endpoints[start],
        direction: delta.map(|v| (f64::from(v) / size) as f32),
        other_endpoint: endpoints[end],
    })
}

/// Output order: runway, first rear leg, lateral leg, forward leg (§3.3.3).
pub fn approach_box(
    runway: Vec3,
    direction: Vec3,
    slope: f32,
    base_altitudes: [f32; 2],
    bounds: AircraftBounds,
) -> [Vec3; 4] {
    let lateral = [-direction[2], 0.0, direction[0]];
    let size = length(lateral);
    let mut lateral = lateral.map(|v| (f64::from(v) / size) as f32);
    let center_dot = f64::from(lateral[0])
        * ((f64::from(bounds.min_x) + f64::from(bounds.max_x)) * 0.5 - f64::from(runway[0]))
        + f64::from(lateral[2]) * ((f64::from(bounds.min_z) + f64::from(bounds.max_z)) * 0.5 - f64::from(runway[2]));
    if center_dot <= 0.0 {
        lateral = lateral.map(|v| -v);
    }
    let dimension = (bounds.max_x - bounds.min_x).max(bounds.max_z - bounds.min_z);
    let nominal = (f64::from(base_altitudes[0].max(base_altitudes[1])) - f64::from(runway[1])) / f64::from(slope);
    let leg = if nominal < 500.0 {
        500.0
    } else {
        nominal.min(f64::from(dimension))
    };
    let mut result = [runway; 4];
    let mut point = runway;
    let mut distance = 0.0;
    for (index, axis) in [direction.map(|v| -v), lateral, direction].into_iter().enumerate() {
        let previous = result[index];
        let next = std::array::from_fn(|i| (f64::from(point[i]) + leg * f64::from(axis[i])) as f32);
        point = clip_aircraft_to_world(next, bounds);
        let mut next = point;
        distance += length(std::array::from_fn(|i| next[i] - previous[i]));
        next[1] = (f64::from(runway[1]) + distance * f64::from(slope)) as f32;
        result[index + 1] = next;
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FireQuery {
    pub minimum: Vec3,
    pub maximum: Vec3,
    pub filter_present: bool,
    pub argument: i32,
}

/// Consumes the adapter's retained service reference. A successful query owns a list,
/// including an empty one. Only the first occupant's position operation is requested.
pub trait FireSearch {
    fn query(&mut self, query: FireQuery) -> Option<usize>;
    fn first_position(&mut self) -> Vec3;
    fn release_list(&mut self);
    fn release_service(&mut self);
}

impl Airplane {
    pub fn search_fire(&mut self, service: &mut dyn FireSearch) -> bool {
        let count = service.query(self.fire_query());
        let found = count.is_some_and(|count| count > 0);
        if found {
            self.base_mut().vectors[2] = service.first_position();
        }
        if count.is_some() {
            service.release_list();
        }
        service.release_service();
        found
    }

    pub fn fire_query(&self) -> FireQuery {
        let [x, _, z] = self.fire_search_anchor;
        FireQuery {
            minimum: [x - 100.0, -f32::MAX, z - 100.0],
            maximum: [x + 100.0, f32::MAX, z + 100.0],
            filter_present: self.fire_filter_present,
            argument: -1,
        }
    }
}

/// Source offsets in the destination gain order +0x300..+0x32C (§4.3.3).
pub const PLAYER_GAIN_RECORD_OFFSETS: [u32; 12] =
    [0x60, 0x78, 0x64, 0x68, 0x6C, 0x7C, 0x80, 0x84, 0x88, 0x90, 0x70, 0x8C];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirplanePlayerRecord {
    pub speed: f32,
    pub gains: [f32; 12],
}
impl AirplanePlayerRecord {
    pub fn import(mut field: impl FnMut(u32) -> f32) -> Self {
        Self {
            speed: field(8),
            gains: PLAYER_GAIN_RECORD_OFFSETS.map(field),
        }
    }
}
pub trait AirplanePlayerSource {
    fn direct_record(&self) -> Option<&AirplanePlayerRecord>;
    fn script_record(&self) -> Option<&AirplanePlayerRecord>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct AirplaneStateCallback {
    pub base_result: bool,
    pub limits: Option<[f32; 2]>,
    pub sounds: Vec<SoundRequest>,
}
impl Airplane {
    pub fn refresh_player_parameters(&mut self, provider: Option<&dyn AirplanePlayerSource>) {
        let Some(record) = provider.and_then(|p| p.direct_record().or_else(|| p.script_record())) else {
            return;
        };
        let speed = (f64::from(record.speed) * 1000.0) as f32;
        if speed != 0.0 {
            self.drive_parameters.0[3] = speed;
        }
        for (destination, value) in self.player_gains.iter_mut().zip(record.gains) {
            let value = (f64::from(value) * f64::from(f32::from_bits(0x3C23D70A))) as f32;
            if value != 0.0 {
                *destination = value;
            }
        }
    }

    pub fn enter_state(
        &mut self,
        state: Option<u32>,
        player: Option<&dyn AirplanePlayerSource>,
        base: impl FnOnce() -> bool,
    ) -> AirplaneStateCallback {
        let base_result = base();
        let is_player = state == Some(0x10);
        let mut sounds = vec![];
        if is_player {
            self.refresh_player_parameters(player);
            if self.phase != Some(FlightPhase::Phase0) {
                sounds.push(SoundRequest::Start(8));
            }
        } else {
            self.control_fields = [0.0; 4];
            if state == Some(0x39) {
                self.fire_search_anchor = self.base().vectors[2];
            }
        }
        self.water_drop_active = false;
        AirplaneStateCallback {
            base_result,
            limits: is_player.then_some([0.0, 1.0]),
            sounds,
        }
    }

    pub fn exit_state(
        &mut self,
        state: Option<u32>,
        ai: Option<&dyn DriveParameterSource>,
        base: impl FnOnce() -> bool,
    ) -> bool {
        let base_result = base();
        if state == Some(0x10) {
            self.drive_parameters = super::AirplaneDriveParameters::load(ai);
            self.control_fields = [0.0; 4];
        }
        self.water_drop_active = false;
        base_result
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AirplaneGoalInputs {
    pub land_succeeded: bool,
    pub destination_succeeded: bool,
    pub destination_sentinel: bool,
    pub fire_found: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirplaneGoalAction {
    Dispatch(u32),
    Push(u32),
    Pop,
    Stop,
    Land,
    ClipDestination,
    GotoDestination,
    VisitLoiter,
    SearchFire,
}
impl Airplane {
    /// Ordered §4.4 dispatch plan. Controller/query results are supplied, not invented.
    /// An adapter executes Push/Pop with the shared stack and its entry/exit callbacks.
    pub fn goal_actions(&self, input: AirplaneGoalInputs) -> Vec<AirplaneGoalAction> {
        use AirplaneGoalAction::*;
        let goal = self.base().states.last().map_or(0, |s| s.state_type);
        let mut actions = vec![];
        match goal {
            0x32 => {
                if !matches!(self.dispatch_state, 0 | 4) {
                    actions.push(Dispatch(6));
                }
                actions.push(Land);
                if input.land_succeeded {
                    actions.extend([Dispatch(0), Pop]);
                }
                return actions;
            }
            0x33 => {
                actions.push(ClipDestination);
                if input.destination_sentinel {
                    actions.push(Pop);
                } else {
                    actions.push(GotoDestination);
                    if input.destination_succeeded {
                        actions.push(Pop);
                    }
                }
                return actions;
            }
            0x35 => return vec![Dispatch(5), VisitLoiter],
            0x39 => {
                actions.push(SearchFire);
                if self.ammunition.is_some_and(|n| n > 0) {
                    if !input.fire_found {
                        actions.push(Pop);
                    }
                    actions.push(Dispatch(3));
                    if !input.fire_found {
                        actions.push(Push(0x32));
                    }
                    actions.extend([Push(0x35), Push(0x33)]);
                } else {
                    actions.extend([Dispatch(4), Push(0x32)]);
                }
                return actions;
            }
            0 => {}
            _ => actions.push(Pop),
        }
        if matches!(self.phase, Some(FlightPhase::Phase1)) {
            actions.push(Stop);
        } else if self.phase.is_some_and(|p| p != FlightPhase::Phase0) {
            actions.push(Push(0x32));
        }
        actions
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirplaneCleanup {
    ReleaseFilter,
    StopEffect(u32, u32),
    ClearEffect(u32),
    ReleaseEffect(u32),
}

impl Airplane {
    /// Execute retained-reference cleanup before invoking base Shutdown (§3.2.1).
    pub fn shutdown(&mut self, mut effect: impl FnMut(AirplaneCleanup), base: impl FnOnce() -> bool) -> bool {
        if self.fire_filter_present {
            effect(AirplaneCleanup::ReleaseFilter);
            self.fire_filter_present = false;
        }
        for index in [1, 0, 3, 2] {
            if self.retained_effects_present[index] {
                let offset = 0x208 + index as u32 * 4;
                effect(AirplaneCleanup::StopEffect(offset, 1));
                self.retained_effects_present[index] = false;
                effect(AirplaneCleanup::ClearEffect(offset));
                effect(AirplaneCleanup::ReleaseEffect(offset));
            }
        }
        base()
    }
}
