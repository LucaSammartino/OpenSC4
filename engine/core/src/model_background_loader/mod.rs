// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4ModelBackgroundLoader, docs/specs/model-maker.md §15.
//!
//! The shared state is locked for worker slices and main-thread communications.
//! Thread, clock, simulator, resource, occupant and shared spiral services are
//! foreign ports. Native time values are opaque: this module never assumes µs.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Mutex, MutexGuard};

use crate::model_maker::{Handle, Rect};

pub type Cell = [i32; 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// Open question MM1, §15.2 / §15.4: interpolation and time units.
    CurveInterpolationMm1,
    ClockUnitsMm1,
    InvalidView,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Curve {
    Constant(f32),
    Samples(Vec<(f32, f32)>),
}

impl Curve {
    pub fn value(&self, _progress: f32) -> Result<f32, Unsupported> {
        match self {
            Self::Constant(value) => Ok(*value),
            // SPEC-GAP: MM1 is open; do not invent interpolation between points.
            Self::Samples(_) => Err(Unsupported::CurveInterpolationMm1),
        }
    }
}

/// No conversion to Duration is supported while MM1 remains open.
pub fn native_delay_duration(_delay: i32) -> Result<std::time::Duration, Unsupported> {
    Err(Unsupported::ClockUnitsMm1)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    pub query_occupants: bool,
    pub worker_limit: Curve,
    pub sleep: Curve,
    pub main_limit: Curve,
    pub minimum_cells: Curve,
    pub occupied_cost: Curve,
    pub empty_cost: Curve,
    pub budget: Curve,
    pub minimum_radius: [u32; 5],
    pub radius_factor: [f32; 5],
    pub maximum_waiting_cells: usize,
    pub resume_progress: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            // SPEC-GAP: the fallback query flag is omitted; use the shipped true value.
            query_occupants: true,
            worker_limit: Curve::Constant(16384.0),
            sleep: Curve::Constant(65536.0),
            main_limit: Curve::Constant(400000.0),
            minimum_cells: Curve::Constant(100.0),
            occupied_cost: Curve::Constant(1.0),
            empty_cost: Curve::Constant(0.1),
            budget: Curve::Constant(8.0),
            minimum_radius: [8, 8, 6, 6, 4],
            radius_factor: [0.0; 5],
            maximum_waiting_cells: 1000,
            resume_progress: 0.7,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Binding {
    Maker,
    ZoomRotationFilter,
    ResourceManager,
    OccupantManager,
    Simulator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    StartThread { priority: u32 },
    WakeThread,
    WaitThread { timeout: u32 },
    SuspendThread,
    HiddenPause,
    HiddenResume,
    BackgroundFinished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timer {
    Worker,
    Main,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceKey {
    pub type_id: u32,
    pub group: u32,
    pub instance: u32,
}

impl Default for ResourceKey {
    fn default() -> Self {
        Self {
            type_id: 0x5AD0_E817,
            group: 0xBADB_57F1,
            instance: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ResourceOccupant {
    pub zoom_mask: u32,
    pub rotation_mask: u32,
    pub fixed_property_exists: bool,
    pub orientation: i32,
    pub flip: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ModelQuery {
    pub zoom: u32,
    pub rotation: u32,
    pub fixed: bool,
    pub orientation: i32,
    pub flip: bool,
}

/// Acquisition methods return one owned reference; getters otherwise borrow.
/// Calls made under the loader lock must not reenter this loader.
pub trait Host {
    fn force_speed_three(&mut self) -> bool;
    fn loader_speed_option(&mut self) -> u32;
    fn tuning(&mut self, speed: u32) -> Option<Tuning>;
    fn acquire(&mut self, binding: Binding) -> Handle;
    fn release(&mut self, handle: Handle);
    fn city_cell_counts(&mut self) -> [i32; 2];
    fn thread_running(&mut self) -> bool;
    fn event(&mut self, event: Event);
    fn memory_mib(&mut self) -> Option<u32>;
    fn screen_width(&mut self) -> Option<u32>;
    fn view_exists(&mut self) -> bool;
    fn set_filter_view(&mut self, zoom: u32, rotation: u32);
    /// Shared simgrid spiral service, clockwise; rect has nondecreasing edges.
    fn reset_spiral(&mut self, rect: Rect, centre: Cell);
    fn next_spiral_cell(&mut self) -> Option<Cell>;
    fn spiral_ring(&mut self) -> i32;
    /// Apply the zoom/rotation filter; return acquired occupants in list order.
    fn occupants(&mut self, cell: Cell, zoom: u32, rotation: u32) -> Vec<Handle>;
    fn resource_occupant(&mut self, occupant: Handle) -> ResourceOccupant;
    /// Foreign cSC4ModelMakerUtility (graphics-formats GF7), not maker logic.
    fn model_ids(&mut self, occupant: Handle, query: ModelQuery, ids: &mut Vec<u32>, key: &mut ResourceKey);
    /// Resource manager slot 3, IID 0x456B8F1D; acquired on success.
    fn load_resource(&mut self, key: ResourceKey) -> Option<Handle>;
    /// Calls the other owner's §10.4 operation through its interface.
    fn load_occupant_models(&mut self, occupant: Handle, zoom: u32, rotation: u32);
    fn restart_timer(&mut self, timer: Timer);
    /// `limit` is in the host timer's native, currently unspecified units.
    fn timer_passed(&mut self, timer: Timer, limit: f32) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    New,
    Running,
    Done,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub state: TaskState,
    pub progress: f32,
    pub rect: Rect,
    pub centre: Cell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThreadState {
    Stopped,
    Running,
    Suspended,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Commands {
    pub values: Vec<u32>,
    pub capacity: usize,
}

impl Commands {
    pub fn push(&mut self, value: u32) {
        if self.values.len() == self.capacity {
            self.grow_insert(self.values.len(), value, 1, true);
        } else {
            self.values.push(value);
        }
    }

    /// §15.6 grow path, including the unused-suffix `at_end` contract.
    pub fn grow_insert(&mut self, pos: usize, value: u32, n: usize, at_end: bool) {
        let size = self.values.len();
        self.capacity = size + size.max(n);
        let mut values = Vec::with_capacity(self.capacity);
        values.extend_from_slice(&self.values[..pos]);
        values.extend(std::iter::repeat(value).take(n));
        if !at_end {
            values.extend_from_slice(&self.values[pos..]);
        }
        self.values = values;
    }
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub initialized: bool,
    pub stop: bool,
    pub paused: bool,
    pub simulator_paused: bool,
    pub speed: u32,
    pub city_max: Cell,
    pub view: [u32; 2],
    pub progress: f32,
    pub progress_width: i32,
    pub tasks: VecDeque<Task>,
    pub waiting: VecDeque<Cell>,
    pub commands: Commands,
    pub resource_count: usize,
}

struct State {
    initialized: bool,
    stop: bool,
    paused: bool,
    simulator_paused: bool,
    speed: u32,
    city_max: Cell,
    view: [u32; 2],
    progress: f32,
    progress_width: i32,
    tasks: VecDeque<Task>,
    waiting: VecDeque<Cell>,
    commands: Commands,
    command_pending: bool,
    resources: BTreeSet<Handle>,
    bindings: BTreeMap<Binding, Handle>,
    tuning: Tuning,
    thread: ThreadState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            initialized: false,
            stop: false,
            paused: false,
            simulator_paused: false,
            speed: 0,
            city_max: [0; 2],
            view: [0; 2],
            progress: 0.0,
            progress_width: 0,
            tasks: VecDeque::new(),
            waiting: VecDeque::new(),
            commands: Commands::default(),
            command_pending: false,
            resources: BTreeSet::new(),
            bindings: BTreeMap::new(),
            tuning: Tuning::default(),
            thread: ThreadState::Stopped,
        }
    }
}

impl State {
    fn post(&mut self, command: u32) {
        self.commands.push(command);
        self.command_pending = true;
    }

    fn clear_resources(&mut self, host: &mut impl Host) {
        for resource in std::mem::take(&mut self.resources) {
            host.release(resource);
        }
    }

    fn speed(host: &mut impl Host) -> u32 {
        if host.force_speed_three() {
            3
        } else {
            host.loader_speed_option()
        }
    }

    fn reset_spiral(task: &Task, host: &mut impl Host) {
        let mut rect = task.rect;
        rect.x1 = rect.x1.max(rect.x0);
        rect.z1 = rect.z1.max(rect.z0);
        host.reset_spiral(rect, task.centre);
    }

    fn load_resources(&mut self, o: Handle, host: &mut impl Host) {
        let occupant = host.resource_occupant(o);
        let [zoom, rotation] = self.view;
        if occupant.zoom_mask & (1 << zoom) != 0 && occupant.rotation_mask & (1 << rotation) != 0 {
            return;
        }
        let mut ids = vec![0; 4];
        let mut key = ResourceKey::default();
        host.model_ids(
            o,
            ModelQuery {
                zoom,
                rotation,
                fixed: occupant.fixed_property_exists,
                orientation: occupant.orientation,
                flip: occupant.flip,
            },
            &mut ids,
            &mut key,
        );
        for instance in ids.into_iter().filter(|id| *id != 0) {
            key.instance = instance;
            if let Some(resource) = host.load_resource(key) {
                if !self.resources.insert(resource) {
                    host.release(resource);
                }
            }
        }
    }

    fn continue_task(&mut self, task: &mut Task, budget: &mut f32, host: &mut impl Host) -> Result<bool, Unsupported> {
        let mut appended = false;
        while *budget > 0.0 && self.waiting.len() < self.tuning.maximum_waiting_cells {
            let Some(cell) = host.next_spiral_cell() else {
                task.state = TaskState::Done;
                break;
            };
            if self.tuning.query_occupants {
                let occupants = host.occupants(cell, self.view[0], self.view[1]);
                let cost = if occupants.is_empty() {
                    &self.tuning.empty_cost
                } else {
                    &self.tuning.occupied_cost
                }
                .value(task.progress);
                // Resolve unsupported tuning before taking any resource references.
                let cost = match cost {
                    Ok(v) => v,
                    Err(e) => {
                        for o in occupants {
                            host.release(o);
                        }
                        return Err(e);
                    }
                };
                for o in occupants {
                    self.load_resources(o, host);
                    host.release(o);
                }
                *budget -= cost;
            }
            self.waiting.push_back(cell);
            appended = true;
        }
        *budget = -1.0;
        if appended {
            self.post(0);
        }
        if task.state == TaskState::Done {
            self.post(1);
            self.post(4);
        }
        Ok(self.waiting.len() < self.tuning.maximum_waiting_cells)
    }

    fn process_cells(&mut self, host: &mut impl Host) -> Result<bool, Unsupported> {
        host.restart_timer(Timer::Main);
        let p = self.tasks.front().map_or(0.0, |task| task.progress);
        let limit = self.tuning.main_limit.value(p)?;
        let minimum = self.tuning.minimum_cells.value(p)?;
        let mut n = 0;
        while let Some(cell) = self.waiting.pop_front() {
            for o in host.occupants(cell, self.view[0], self.view[1]) {
                host.load_occupant_models(o, self.view[0], self.view[1]);
                host.release(o);
            }
            n += 1;
            if n as f32 > minimum && host.timer_passed(Timer::Main, limit) {
                break;
            }
        }
        Ok(self.waiting.is_empty())
    }
}

#[derive(Default)]
pub struct ModelBackgroundLoader {
    state: Mutex<State>,
}

impl ModelBackgroundLoader {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn snapshot(&self) -> Snapshot {
        let s = self.lock();
        Snapshot {
            initialized: s.initialized,
            stop: s.stop,
            paused: s.paused,
            simulator_paused: s.simulator_paused,
            speed: s.speed,
            city_max: s.city_max,
            view: s.view,
            progress: s.progress,
            progress_width: s.progress_width,
            tasks: s.tasks.clone(),
            waiting: s.waiting.clone(),
            commands: s.commands.clone(),
            resource_count: s.resources.len(),
        }
    }

    pub fn init(&self, host: &mut impl Host) -> bool {
        let mut s = self.lock();
        if s.initialized {
            return true;
        }
        s.speed = State::speed(host);
        s.tuning = host.tuning(s.speed).unwrap_or_default();
        for binding in [
            Binding::Maker,
            Binding::ZoomRotationFilter,
            Binding::ResourceManager,
            Binding::OccupantManager,
            Binding::Simulator,
        ] {
            s.bindings.insert(binding, host.acquire(binding));
        }
        s.city_max = host.city_cell_counts().map(|count| count - 1);
        if !host.thread_running() {
            s.stop = false;
            host.event(Event::StartThread { priority: 3 });
        }
        s.thread = ThreadState::Running;
        s.initialized = true;
        true
    }

    pub fn shutdown(&self, host: &mut impl Host) -> bool {
        let mut s = self.lock();
        if !s.initialized {
            return true;
        }
        s.initialized = false;
        if s.thread != ThreadState::Stopped {
            s.stop = true;
            if s.thread == ThreadState::Suspended {
                host.event(Event::WakeThread);
                s.thread = ThreadState::Running;
            }
            // A worker finishing its exit needs the lock to post command 4.
            drop(s);
            host.event(Event::WaitThread { timeout: 2_000_000 });
            s = self.lock();
            s.thread = ThreadState::Stopped;
        }
        if s.simulator_paused {
            host.event(Event::HiddenResume);
            s.simulator_paused = false;
        }
        s.clear_resources(host);
        for binding in [
            Binding::ResourceManager,
            Binding::OccupantManager,
            Binding::Simulator,
            Binding::ZoomRotationFilter,
            Binding::Maker,
        ] {
            if let Some(handle) = s.bindings.remove(&binding) {
                host.release(handle);
            }
        }
        true
    }

    /// Explicit native destructor boundary, since host calls need a live port.
    pub fn destroy(&self, host: &mut impl Host) {
        self.shutdown(host);
        let mut s = self.lock();
        s.tasks.clear();
        s.waiting.clear();
        s.commands = Commands::default();
        s.tuning = Tuning::default();
    }

    pub fn expand_load_rect(&self, rect: &mut Rect, zoom: usize, host: &mut impl Host) -> Result<(), Unsupported> {
        if zoom >= 5 {
            return Err(Unsupported::InvalidView);
        }
        let s = self.lock();
        let radius = ((host.memory_mib().unwrap_or(0) as f32 * s.tuning.radius_factor[zoom]).trunc() as u32)
            .max(s.tuning.minimum_radius[zoom]);
        rect.grow(radius as i32);
        Ok(())
    }

    pub fn start(
        &self,
        zoom: u32,
        rotation: u32,
        rect: Rect,
        centre: Cell,
        host: &mut impl Host,
    ) -> Result<(), Unsupported> {
        // SPEC-GAP: the caller contract only specifies five zooms and four rotations.
        if zoom >= 5 || rotation >= 4 {
            return Err(Unsupported::InvalidView);
        }
        let mut s = self.lock();
        let speed = State::speed(host);
        if speed != s.speed {
            s.speed = speed;
            s.tuning = host.tuning(speed).unwrap_or_default();
        }
        s.tasks.clear();
        s.progress = 0.0;
        s.clear_resources(host);
        s.waiting.clear();
        s.view = [zoom, rotation];
        s.progress_width = (host.screen_width().unwrap_or(1024) as f32
            * [0.166015625, 0.08886719, 0.044921875, 0.01953125, 0.009765625][zoom as usize])
            .trunc() as i32;
        host.set_filter_view(zoom, rotation);
        if host.view_exists() {
            s.tasks.push_back(Task {
                state: TaskState::New,
                progress: 0.0,
                rect,
                centre,
            });
            s.post(3);
            if s.thread == ThreadState::Suspended {
                host.event(Event::WakeThread);
                s.thread = ThreadState::Running;
            }
        }
        Ok(())
    }

    pub fn refocus(&self, rect: Rect, centre: Cell, host: &mut impl Host) {
        let mut s = self.lock();
        if let Some(task) = s.tasks.front_mut() {
            task.progress = 0.0;
            task.rect = rect;
            task.centre = centre;
            State::reset_spiral(task, host);
            s.waiting.clear();
        }
    }

    pub fn cancel(&self, host: &mut impl Host) {
        let mut s = self.lock();
        s.tasks.clear();
        s.waiting.clear();
        s.clear_resources(host);
    }

    pub fn pause(&self) {
        self.lock().paused = true;
    }
    pub fn resume(&self) {
        self.lock().paused = false;
    }
    pub fn post_command(&self, command: u32) {
        self.lock().post(command);
    }

    /// One slice; returns the native delay for the foreign thread service, or
    /// None when stopped. The host owns the outer thread loop and actual sleep.
    pub fn worker_slice(&self, host: &mut impl Host) -> Result<Option<i32>, Unsupported> {
        let mut s = self.lock();
        if s.stop {
            s.post(4);
            return Ok(None);
        }
        host.restart_timer(Timer::Worker);
        let mut budget = s.tuning.budget.value(0.0)?;
        let mut keep_going = true;
        while !s.tasks.is_empty() && !s.paused && keep_going {
            let p = s.tasks.front().unwrap().progress;
            if budget <= 0.0 {
                if host.timer_passed(Timer::Worker, s.tuning.worker_limit.value(p)?) {
                    break;
                }
                budget = s.tuning.budget.value(p)?;
            }
            let mut task = s.tasks.pop_front().unwrap();
            if task.state == TaskState::New {
                State::reset_spiral(&task, host);
                task.state = TaskState::Running;
            }
            let continued = s.continue_task(&mut task, &mut budget, host);
            let ring = host.spiral_ring() as f64;
            task.progress =
                ((4.0 * ring * ring - s.waiting.len() as f64).sqrt().trunc() / (s.progress_width + 1) as f64) as f32;
            s.progress = task.progress;
            if task.state != TaskState::Done {
                s.tasks.push_front(task);
            }
            keep_going = continued?;
        }
        if s.tasks.is_empty() {
            s.post(2);
        }
        if s.simulator_paused && s.progress > s.tuning.resume_progress {
            s.post(4);
        }
        let delay = if let Some(task) = s.tasks.front() {
            s.tuning.sleep.value(task.progress)?.min(5_000_000.0).trunc() as i32
        } else {
            1_000_000
        };
        Ok(Some(delay))
    }

    /// False means command 0 left cells waiting: keep the entire command list.
    pub fn process_communications(&self, host: &mut impl Host) -> Result<bool, Unsupported> {
        let mut s = self.lock();
        if !s.command_pending {
            return Ok(true);
        }
        for index in 0..s.commands.values.len() {
            match s.commands.values[index] {
                0 if !s.waiting.is_empty() => {
                    if !s.process_cells(host)? {
                        return Ok(false);
                    }
                }
                1 => host.event(Event::BackgroundFinished),
                2 if s.tasks.is_empty() && s.thread != ThreadState::Suspended => {
                    host.event(Event::SuspendThread);
                    s.thread = ThreadState::Suspended;
                }
                3 if !s.simulator_paused => {
                    host.event(Event::HiddenPause);
                    s.simulator_paused = true;
                }
                4 if s.simulator_paused => {
                    host.event(Event::HiddenResume);
                    s.simulator_paused = false;
                }
                _ => {}
            }
        }
        s.commands.values.clear();
        s.command_pending = false;
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
