// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable, bounded owner of the published drive-tool algorithms.
//!
//! This is not a native Init/Shutdown or a complete EnterDrivingMode. Partial
//! entry executes the published projection, marking every omitted step in the
//! output and result. In particular, a selected dashboard ID is NOT a loaded
//! dashboard. No host hook implements a same-owner algorithm.
//!
//! Operations are ordered primitive plans, not COM calls. A consumer must not
//! deliver callbacks reentrantly or claim native failure/ownership semantics
//! from this interface (DR3). Queries are raw foreign inputs; failures remain
//! explicit and already emitted prefixes are not rolled back. No RefCell or
//! hidden getter mutation is used.

pub const ID: u32 = 0x2B70_9676;
pub const CURSOR: u32 = 0xC16C_4E1B;
pub const CAMERA_SPEED_BITS: u32 = 0x3D2C_0831;
pub const DASHBOARD_PROPERTY: u32 = 0x2BE8_FBF3;
pub const SOUND_PROPERTY: u32 = 0xCBBB_A52F;
pub const CALLBACK_MESSAGE: u32 = 0x0BB3_24BD;
const CURSOR_TEXT: u32 = 0xAB73_510C;
const MANAGERS: [u32; 4] = [43, 44, 45, 46];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Automaton(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetworkOccupant(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DashboardWindow(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallbackTarget(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pixel {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell(pub i32, pub i32);
/// World coordinates retained as raw IEEE-754 words, without unit conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct World(pub [u32; 3]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    NativeInit,
    NativeShutdown,
    DashboardShowUi, // DR1
    UpdateGauges,    // DR1; no camera tracking cadence is inferred either
    HandleKeyEffects,
    AudioTable,
    SettingsSelection,
    NumericBoundary,
    NonBooleanDriveByte,
    MissingDriveState,
    MissingSoundProperty,
    DashboardWindowMissing,
    HoverOwnershipAndText,
    CallbackListPolicy,
    CallbackDelivery,
    EndCallbackValue,
    PanelPolicy,
    AlternateModel,
    UnknownMessage,
    UnknownButton,
    KeyboardCommand8,
    UnknownCommand,
    ReleaseRepeatPolicy,
    ForeignPrimitive,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    Applied,
    Ignored,
    NoDrivableForm,
    Partial(Vec<Unsupported>),
}
pub type Result = std::result::Result<Progress, Unsupported>;
pub type Raw<T> = std::result::Result<T, Unsupported>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawValue {
    Known(u32),
    Unpublished,
}

/// Only primitive effects. `Gap` is diagnostic, never permission to synthesize
/// or delegate an omitted owner algorithm to a host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Gap(Unsupported),
    DisasterLayer(bool),
    ClearCursorText(u32),
    SimulatorPause,
    SimulatorResume,
    ManagerPause(u32),
    ManagerResume(u32),
    JunctionHover(Option<Cell>),
    JunctionSelected(Option<Cell>),
    PositionedSound {
        id: u32,
        position: World,
    },
    MinimizeCityUi,
    ShowCityWindow(u32),
    DashboardIdSelected(u32),
    OccupantSound {
        id: u32,
        start: bool,
    },
    EngineEvent {
        event: u32,
        start: bool,
    },
    AcquireDriven(Automaton),
    ReleaseDriven(Automaton),
    State52 {
        automaton: Automaton,
        words: [u32; 7],
    },
    State13 {
        automaton: Automaton,
        value: u32,
    },
    Slot53(Automaton),
    VehicleManagerTrack(Option<Automaton>),
    DispatchTrack(Option<Automaton>),
    Callback39(Automaton),
    Callback40(Automaton),
    ZoomLimits(i32, i32),
    Zoom(i32),
    WindowEnabled {
        window: DashboardWindow,
        id: u32,
        enabled: bool,
    },
    WindowLight {
        window: DashboardWindow,
        id: u32,
        on: bool,
    },
    DriveCommand {
        automaton: Automaton,
        command: u32,
        value: u32,
    },
    DriveByte229 {
        automaton: Automaton,
        value: u8,
    },
    DriveMouse {
        automaton: Automaton,
        at: Pixel,
    },
    TimerReset,
    CallbackPlan {
        target: CallbackTarget,
        message: u32,
        tool: u64,
        values: [RawValue; 2],
    },
    RemoveTopControl(bool),
    PostCommand(u32),
}

/// Foreign primitive queries only. A renderer pick returns an individual
/// occupant queried for the specified IID, not a tool-level selection outcome.
/// `emit` records/transports individual operations infallibly; native adapters
/// with failure or reentrant delivery need contracts not yet published (DR3).
pub trait Primitives {
    fn emit(&mut self, operation: Operation);
    fn simulator_paused(&mut self) -> Raw<bool>;
    fn situation_slot32(&mut self) -> Raw<bool>;
    fn flags31(&mut self, a: Automaton) -> Raw<u32>;
    fn group(&mut self, a: Automaton) -> Raw<u32>;
    fn state(&mut self, a: Automaton) -> Raw<u32>;
    fn slot18(&mut self, a: Automaton) -> Raw<bool>;
    fn drivable_form50(&mut self, a: Automaton) -> Raw<Option<Automaton>>;
    fn occupant_type(&mut self, a: Automaton) -> Raw<u32>;
    fn property(&mut self, a: Automaton, id: u32) -> Raw<Option<u32>>;
    fn drive_state_present(&mut self, a: Automaton) -> Raw<bool>;
    fn drive_byte229(&mut self, a: Automaton) -> Raw<u8>;
    fn renderer_pick_automaton(&mut self, at: Pixel, iid: u32) -> Raw<Option<Automaton>>;
    fn renderer_pick_network(&mut self, at: Pixel, iid: u32) -> Raw<Option<NetworkOccupant>>;
    fn network_slot42(&mut self, n: NetworkOccupant) -> Raw<bool>;
    fn network_position(&mut self, n: NetworkOccupant) -> Raw<World>;
    fn terrain_pick(&mut self, at: Pixel) -> Raw<Option<World>>;
    fn position_to_cell(&mut self, at: World) -> Raw<Cell>;
    fn script_keymap14(&mut self, key: u32) -> Raw<u32>;
    fn can_zoom_in(&mut self) -> Raw<bool>;
    fn can_zoom_out(&mut self) -> Raw<bool>;
}

/// Explicit raw fields for the bounded entry projection. Resolving these from
/// slot67/+0x64 versus AEMS is NOT delegated: it remains SettingsSelection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CameraFields {
    pub zoom: Option<i32>,
    pub level: Option<i32>,
    pub speed_bits: Option<u32>,
    pub preference_f10: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Choosing,
    Driving,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerState {
    Constructed,
    Reset,
    /// Code B stopped audio, but no alternate-model contract permits restart.
    RestartBlocked,
}

#[derive(Debug)]
pub struct PlayerDrive {
    tool_identity: u64,
    current: Option<Automaton>,
    // Borrowed identity only: DR3 does not publish hover AddRef/Release rules.
    hover: Option<Automaton>,
    selected: Option<Cell>,
    hover_cell: Option<Cell>,
    toggles: [bool; 2],
    camera_speed_bits: u32,
    timer: TimerState,
    callbacks: Vec<CallbackTarget>,
    dashboard_id: Option<u32>,
    dashboard: Option<DashboardWindow>,
}

impl PlayerDrive {
    pub fn new(tool_identity: u64) -> Self {
        Self {
            tool_identity,
            current: None,
            hover: None,
            selected: None,
            hover_cell: None,
            toggles: [false; 2],
            camera_speed_bits: CAMERA_SPEED_BITS,
            timer: TimerState::Constructed,
            callbacks: Vec::new(),
            dashboard_id: None,
            dashboard: None,
        }
    }
    pub fn mode(&self) -> Mode {
        if self.current.is_some() {
            Mode::Driving
        } else {
            Mode::Choosing
        }
    }
    pub fn current(&self) -> Option<Automaton> {
        self.current
    }
    pub fn hover(&self) -> Option<Automaton> {
        self.hover
    }
    pub fn selected(&self) -> Option<Cell> {
        self.selected
    }
    pub fn hover_cell(&self) -> Option<Cell> {
        self.hover_cell
    }
    pub fn toggles(&self) -> [bool; 2] {
        self.toggles
    }
    pub fn camera_speed_bits(&self) -> u32 {
        self.camera_speed_bits
    }
    pub fn cell_scales(&self) -> (u32, u32) {
        (0x3F80_0000, 0x3F80_0000)
    }
    pub fn timer(&self) -> TimerState {
        self.timer
    }
    pub fn dashboard_id(&self) -> Option<u32> {
        self.dashboard_id
    }

    /// Attach only an already-created window. Does not implement ShowUI or
    /// assert that this window is the selected dashboard's resource instance.
    pub fn attach_dashboard_window(&mut self, window: Option<DashboardWindow>) {
        self.dashboard = window;
    }
    pub fn init(&mut self) -> Result {
        Err(Unsupported::NativeInit)
    }
    pub fn shutdown(&mut self) -> Result {
        Err(Unsupported::NativeShutdown)
    }
    pub fn show_ui(&mut self, _id: u32) -> Result {
        Err(Unsupported::DashboardShowUi)
    }
    pub fn update(&mut self) -> Result {
        Err(Unsupported::UpdateGauges)
    }
    pub fn handle_key_effects(&mut self, _key: u32) -> Result {
        Err(Unsupported::HandleKeyEffects)
    }

    pub fn is_drive_auto_valid(&self, host: &mut impl Primitives) -> Raw<bool> {
        match self.current {
            Some(a) => Ok(host.flags31(a)? & (0x0800_0000 | 1) == 0),
            None => Ok(false),
        }
    }

    pub fn pause_everything(&mut self, host: &mut impl Primitives, pause: bool) -> Result {
        let paused = host.simulator_paused()?;
        if pause && !paused {
            host.emit(Operation::SimulatorPause);
        }
        if !pause && paused {
            host.emit(Operation::SimulatorResume);
        }
        for manager in MANAGERS {
            host.emit(if pause {
                Operation::ManagerPause(manager)
            } else {
                Operation::ManagerResume(manager)
            });
        }
        Ok(Progress::Applied)
    }

    fn clear_hover(&mut self, host: &mut impl Primitives) {
        self.hover = None;
        host.emit(Operation::ClearCursorText(CURSOR_TEXT));
    }
    pub fn enter_choosing_mode(&mut self, host: &mut impl Primitives) -> Result {
        self.stop_tracking(host)?;
        self.clear_hover(host);
        self.pause_everything(host, true)?;
        Ok(Progress::Partial(vec![Unsupported::HoverOwnershipAndText]))
    }

    pub fn dashboard_selection(kind: u32, property: Option<u32>) -> u32 {
        // DR4: literal presence override, including zero; no resource fallback.
        property.unwrap_or(if kind == 0x480E_6151 {
            0x6BE8_846C
        } else {
            0x0BE8_824F
        })
    }

    /// Executes only the published portable projection. Gaps are emitted at
    /// their sequence positions and returned even when later known steps run.
    /// This intentionally cannot return complete entry success (DR1/DR4).
    pub fn enter_driving_mode(
        &mut self,
        host: &mut impl Primitives,
        picked: Automaton,
        camera: CameraFields,
    ) -> Result {
        host.emit(Operation::DisasterLayer(false));
        let Some(a) = host.drivable_form50(picked)? else {
            return Ok(Progress::NoDrivableForm);
        };
        // DR4: no invented wrapping/clamping for the two subtractions.
        let zoom = camera
            .zoom
            .unwrap_or(1)
            .checked_sub(1)
            .filter(|v| *v >= 0)
            .ok_or(Unsupported::NumericBoundary)?;
        let level = camera
            .level
            .unwrap_or(5)
            .checked_sub(1)
            .filter(|v| *v >= 0)
            .ok_or(Unsupported::NumericBoundary)?;
        let max_zoom = if camera.preference_f10 > 0 { 5 } else { 4 };
        // DR4: inconsistent raw window ranges are unsupported, not clamped.
        if zoom > max_zoom || level < zoom || level > max_zoom {
            return Err(Unsupported::NumericBoundary);
        }
        self.stop_tracking(host)?;
        self.clear_hover(host);
        host.emit(Operation::MinimizeCityUi);
        host.emit(Operation::ShowCityWindow(0x43));
        let kind = host.occupant_type(a)?;
        let dashboard = Self::dashboard_selection(kind, host.property(a, DASHBOARD_PROPERTY)?);
        self.dashboard_id = Some(dashboard);
        host.emit(Operation::DashboardIdSelected(dashboard));
        host.emit(Operation::Gap(Unsupported::DashboardShowUi));
        let sound = host
            .property(a, SOUND_PROPERTY)?
            .ok_or(Unsupported::MissingSoundProperty)?;
        host.emit(Operation::OccupantSound {
            id: sound,
            start: true,
        });
        host.emit(Operation::EngineEvent {
            event: 0,
            start: true,
        });
        host.emit(Operation::Gap(Unsupported::AudioTable));
        host.emit(Operation::AcquireDriven(a));
        self.current = Some(a);
        host.emit(Operation::State52 {
            automaton: a,
            words: [0x10, 0, 0xBF80_0000, 0, 0, 0, 0],
        });
        host.emit(Operation::VehicleManagerTrack(Some(a)));
        host.emit(Operation::Gap(Unsupported::SettingsSelection));
        self.camera_speed_bits = camera.speed_bits.unwrap_or(CAMERA_SPEED_BITS);
        host.emit(Operation::ZoomLimits(zoom, max_zoom));
        host.emit(Operation::Zoom(level));
        let mut gaps = vec![
            Unsupported::DashboardShowUi,
            Unsupported::AudioTable,
            Unsupported::SettingsSelection,
            Unsupported::HoverOwnershipAndText,
        ];
        if self.dashboard.is_some() {
            self.enable_zoom_buttons(host)?;
        } else {
            host.emit(Operation::Gap(Unsupported::DashboardWindowMissing));
            gaps.push(Unsupported::DashboardWindowMissing);
        }
        self.toggles = [true, false];
        host.emit(Operation::DispatchTrack(Some(a)));
        host.emit(Operation::Callback39(a));
        self.timer = TimerState::Reset;
        host.emit(Operation::TimerReset);
        self.pause_everything(host, false)?;
        Ok(Progress::Partial(gaps))
    }

    pub fn stop_tracking(&mut self, host: &mut impl Primitives) -> Result {
        let Some(a) = self.current else {
            return Ok(Progress::Ignored);
        };
        host.emit(Operation::Callback40(a));
        host.emit(Operation::DispatchTrack(None));
        host.emit(Operation::VehicleManagerTrack(None));
        host.emit(Operation::ZoomLimits(0, 5));
        host.emit(Operation::EngineEvent {
            event: 0,
            start: false,
        });
        let sound = host
            .property(a, SOUND_PROPERTY)?
            .ok_or(Unsupported::MissingSoundProperty)?;
        host.emit(Operation::OccupantSound {
            id: sound,
            start: false,
        });
        if host.state(a)? == 0x10 {
            if host.flags31(a)? & 0x0800_0000 == 0 && host.group(a)? != 0x4003 {
                host.emit(Operation::State13 {
                    automaton: a,
                    value: 0,
                });
            } else {
                host.emit(Operation::Slot53(a));
            }
        }
        host.emit(Operation::ReleaseDriven(a));
        self.current = None;
        Ok(Progress::Applied)
    }

    pub fn add_callback_target(&mut self, target: CallbackTarget) -> Result {
        // DR3: no invented duplicate/dedup or native target ownership policy.
        if self.callbacks.contains(&target) {
            return Err(Unsupported::CallbackListPolicy);
        }
        self.callbacks.push(target);
        Ok(Progress::Partial(vec![Unsupported::CallbackDelivery]))
    }
    fn callback_plans(&self, host: &mut impl Primitives, values: [RawValue; 2]) {
        for &target in &self.callbacks {
            host.emit(Operation::CallbackPlan {
                target,
                message: CALLBACK_MESSAGE,
                tool: self.tool_identity,
                values,
            });
        }
    }
    pub fn send_callback_message(
        &self,
        host: &mut impl Primitives,
        first: u32,
        second: u32,
    ) -> Result {
        self.callback_plans(host, [RawValue::Known(first), RawValue::Known(second)]);
        Ok(Progress::Partial(vec![Unsupported::CallbackDelivery]))
    }
    pub fn end_input(&mut self, host: &mut impl Primitives) -> Result {
        // DR3: this is a plan with an unresolved value, never a native delivery.
        self.callback_plans(host, [RawValue::Known(1), RawValue::Unpublished]);
        if let Some(a) = self.current {
            if host.slot18(a)? {
                host.emit(Operation::State13 {
                    automaton: a,
                    value: 0,
                });
            }
        }
        self.stop_tracking(host)?;
        self.clear_hover(host);
        host.emit(Operation::DisasterLayer(true));
        host.emit(Operation::RemoveTopControl(false));
        Ok(Progress::Partial(vec![
            Unsupported::EndCallbackValue,
            Unsupported::CallbackDelivery,
            Unsupported::HoverOwnershipAndText,
        ]))
    }
    pub fn automaton_removed(&mut self, host: &mut impl Primitives, a: Automaton) -> Result {
        if self.current != Some(a) {
            return Ok(Progress::Ignored);
        }
        self.stop_tracking(host)?;
        self.end_input(host)
    }

    pub fn pick_automaton(&self, host: &mut impl Primitives, at: Pixel) -> Raw<Option<Automaton>> {
        host.renderer_pick_automaton(at, 0xA9B4_0F05)
    }
    pub fn pick_network_occupant(
        &self,
        host: &mut impl Primitives,
        at: Pixel,
    ) -> Raw<Option<NetworkOccupant>> {
        host.renderer_pick_network(at, 0xA821_EF94)
    }
    pub fn on_mouse_move(&mut self, host: &mut impl Primitives, at: Pixel) -> Result {
        self.hover_cell = match host.terrain_pick(at)? {
            Some(world) => Some(host.position_to_cell(world)?),
            None => None,
        };
        host.emit(Operation::JunctionHover(self.hover_cell));
        if self.is_drive_auto_valid(host)? {
            let a = self.current.expect("validity requires a current automaton");
            if !host.drive_state_present(a)? {
                return Err(Unsupported::MissingDriveState);
            }
            host.emit(Operation::DriveMouse { automaton: a, at });
            Ok(Progress::Applied)
        } else {
            self.hover = self.pick_automaton(host, at)?;
            if self.hover.is_none() {
                host.emit(Operation::ClearCursorText(CURSOR_TEXT));
            }
            Ok(Progress::Partial(vec![Unsupported::HoverOwnershipAndText]))
        }
    }
    pub fn on_mouse_down_l(
        &mut self,
        host: &mut impl Primitives,
        at: Pixel,
        camera: CameraFields,
    ) -> Result {
        if let Some(n) = self.pick_network_occupant(host, at)? {
            if host.network_slot42(n)? {
                let position = host.network_position(n)?;
                host.emit(Operation::PositionedSound {
                    id: 0x8BE8_D9AC,
                    position,
                });
                let cell = host.position_to_cell(position)?;
                self.selected = Some(cell);
                host.emit(Operation::JunctionSelected(Some(cell)));
                return Ok(Progress::Applied);
            }
        }
        if self.current.is_none() {
            return match self.pick_automaton(host, at)? {
                Some(a) => self.enter_driving_mode(host, a, camera),
                None => Ok(Progress::Ignored),
            };
        }
        if host.situation_slot32()? {
            return Ok(Progress::Ignored);
        }
        if self.is_drive_auto_valid(host)? {
            let a = self.current.expect("validity requires a current automaton");
            if host.group(a)? == 0x4006 {
                return self.command_effects(host, a, 7, true);
            }
        }
        Ok(Progress::Ignored)
    }

    fn mouse_command(&mut self, host: &mut impl Primitives, command: u32, down: bool) -> Result {
        self.apply_numeric_command(host, command, down)
    }
    pub fn on_mouse_up_l(&mut self, host: &mut impl Primitives) -> Result {
        self.selected = None;
        host.emit(Operation::JunctionSelected(None));
        self.mouse_command(host, 7, false)
    }
    pub fn on_mouse_down_r(&mut self, host: &mut impl Primitives) -> Result {
        self.mouse_command(host, 8, true)
    }
    pub fn on_mouse_up_r(&mut self, host: &mut impl Primitives) -> Result {
        self.mouse_command(host, 8, false)
    }

    /// Owned numeric table, deliberately separate from unpublished key effects.
    /// 8 is exposed for the published mouse path, not asserted as a key row.
    /// The input gate belongs here too so direct numeric use cannot bypass it.
    pub fn apply_numeric_command(
        &mut self,
        host: &mut impl Primitives,
        command: u32,
        down: bool,
    ) -> Result {
        if !self.is_drive_auto_valid(host)? {
            return Ok(Progress::Ignored);
        }
        if host.situation_slot32()? {
            return Ok(Progress::Ignored);
        }
        if command == 13 {
            return Ok(Progress::Ignored);
        }
        if command > 13 {
            return Err(Unsupported::UnknownCommand);
        }
        let a = self.current.expect("validity requires a current automaton");
        self.command_effects(host, a, command, down)
    }
    fn command_effects(
        &mut self,
        host: &mut impl Primitives,
        a: Automaton,
        command: u32,
        down: bool,
    ) -> Result {
        let mut value = u32::from(down);
        let mut byte = None;
        if command == 9 && down {
            if !host.drive_state_present(a)? {
                return Err(Unsupported::MissingDriveState);
            }
            byte = Some(match host.drive_byte229(a)? {
                0 => 1,
                1 => 0,
                _ => return Err(Unsupported::NonBooleanDriveByte),
            });
        }
        if command == 4 || command == 10 {
            if !down {
                return Ok(Progress::Ignored);
            }
            let index = usize::from(command == 10);
            self.toggles[index] = !self.toggles[index];
            value = u32::from(self.toggles[index]);
        }
        if command == 2 || command == 3 {
            let window = self.dashboard.ok_or(Unsupported::DashboardWindowMissing)?;
            host.emit(Operation::WindowLight {
                window,
                id: if command == 2 {
                    0x8BCB_93FC
                } else {
                    0x6BCB_93F5
                },
                on: down,
            });
        }
        host.emit(Operation::DriveCommand {
            automaton: a,
            command,
            value,
        });
        if let Some(value) = byte {
            host.emit(Operation::DriveByte229 {
                automaton: a,
                value,
            });
        }
        let sound = match command {
            0 => Some(1),
            1 => Some(2),
            2 | 3 => Some(3),
            5 => Some(6),
            7 => Some(4),
            _ => None,
        };
        if let Some(event) = sound {
            host.emit(Operation::EngineEvent { event, start: down });
        }
        Ok(Progress::Applied)
    }

    pub fn on_key_down(&mut self, host: &mut impl Primitives, key: u32, flags: u32) -> Result {
        if flags & 0x4000_0000 != 0 {
            return Ok(Progress::Ignored);
        }
        if key == 0x1B {
            return self.end_input(host);
        }
        self.key_lookup(host, key)
    }
    pub fn on_key_up(&mut self, host: &mut impl Primitives, key: u32, flags: u32) -> Result {
        // DR5 only publishes autorepeat suppression for key-down tests.
        if flags & 0x4000_0000 != 0 {
            return Err(Unsupported::ReleaseRepeatPolicy);
        }
        self.key_lookup(host, key)
    }
    fn key_lookup(&mut self, host: &mut impl Primitives, key: u32) -> Result {
        if host.situation_slot32()? || !self.is_drive_auto_valid(host)? {
            return Ok(Progress::Ignored);
        }
        let command = host.script_keymap14(key)?;
        if command > 13 {
            return Err(Unsupported::UnknownCommand);
        }
        if command == 8 {
            return Err(Unsupported::KeyboardCommand8);
        }
        // Even command13 may be consumed by HandleKeyEffects (DR5).
        // Never ask a host to return that same-owner policy outcome.
        self.handle_key_effects(key)
    }

    pub fn do_win_proc_message(&self, host: &mut impl Primitives, button: u32) -> Result {
        let command = match button {
            0x4C1A_5CC9 => 0xCC1F_0457,
            0x2C16_C9CB => 0xAC1F_0449,
            0x4992_046A => 0x6A93_5CB7,
            0xE992_0494 => 0x6A93_5CBA,
            _ => return Err(Unsupported::UnknownButton),
        };
        host.emit(Operation::PostCommand(command));
        Ok(Progress::Applied)
    }
    fn enable_zoom_buttons(&self, host: &mut impl Primitives) -> Result {
        let window = self.dashboard.ok_or(Unsupported::DashboardWindowMissing)?;
        let enabled = host.can_zoom_in()?;
        host.emit(Operation::WindowEnabled {
            window,
            id: 0x4992_046A,
            enabled,
        });
        let enabled = host.can_zoom_out()?;
        host.emit(Operation::WindowEnabled {
            window,
            id: 0xE992_0494,
            enabled,
        });
        Ok(Progress::Applied)
    }
    pub fn do_message(
        &mut self,
        host: &mut impl Primitives,
        id: u32,
        sender: Option<Automaton>,
        code: u32,
    ) -> Result {
        match id {
            0x86AD_10EE => self.update(),
            0xA6B7_9602 => self.enable_zoom_buttons(host),
            0xAC1F_0449 | 0xAC1F_045D | 0xCC1F_0457 => Err(Unsupported::PanelPolicy),
            0x09D9_2FC3 => {
                // DR6: do not apply another automaton's notification to ours.
                let Some(a) = sender.filter(|a| self.current == Some(*a)) else {
                    return Ok(Progress::Ignored);
                };
                match code {
                    3 => self.automaton_removed(host, a),
                    0xB => {
                        host.emit(Operation::EngineEvent {
                            event: 0,
                            start: false,
                        });
                        self.timer = TimerState::RestartBlocked;
                        Err(Unsupported::AlternateModel)
                    }
                    0xD => {
                        if host.drive_state_present(a)? {
                            return Ok(Progress::Ignored);
                        }
                        host.emit(Operation::EngineEvent {
                            event: 0,
                            start: false,
                        });
                        Ok(Progress::Applied)
                    }
                    _ => Err(Unsupported::UnknownMessage),
                }
            }
            _ => Err(Unsupported::UnknownMessage),
        }
    }
}

#[cfg(test)]
mod tests;
