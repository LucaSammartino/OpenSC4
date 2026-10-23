// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4AudioAmbienceManager` (`docs/specs/audio-ambience.md`): the city-side
//! service that turns occupants into positional sounds, runs the occupant
//! alarm clock, keeps the listener's attention rectangle and drives the
//! ambience layers once per tick. Section references (`§n`) point at that
//! spec.
//!
//! The manager is pure state plus a typed [`AmbienceHost`]: every native
//! service it touches (framework, `cIGZSndSys`, message server, occupant and
//! lot managers, the city, the simulation clock, the audio director, the
//! audio preferences, the freshness player) is a host method. Nothing here
//! fakes a sound: a sound is a [`SndRequest`] handed to the host.
//!
//! Inputs are typed providers only: the host reports what the world holds
//! (occupant properties and flags, lot occupants and condition, clock,
//! listener, tuning); the manager's own sound outputs are never fed back to
//! it as inputs.
//!
//! # Method inventory (§2-§7)
//!
//! `Init`, `Shutdown`, `_HandleInitMessage`, city shutdown + `0x004E9660`,
//! `_ReadTuningValues`, `_UpdateAttentionRect`, `OnTick`, `_DoStageUpdate`
//! (prime-step `RandomIterator`, AA7), `_AddOccupantSndInstance`,
//! `_RemoveOccupantSndInstance`, `HandleInsertOccupant`/`HandleInitOccupant`,
//! `HandleMoveOccupant`, `HandleRemoveOccupant`,
//! `_UpdateLotOccupantsForConditionChange`, `_HandleAutomatonMessage`,
//! `_PlayOccupantSound`, `_HandleRailwayCrossingMessage`, `_UpdateAlarms`,
//! `_AddAlarmOccupant`, `_RemoveAlarmOccupant`, `DoEvent`, the focus handler
//! and `DoMessage`. The ambience **layer** (`cSC4AudioAmbienceLayer`, §8) is
//! a separate class: the manager only calls its slots through
//! [`AmbienceLayer`]. The init walk includes buildings, props and vehicles
//! (AA4). Only alarm creation without a clock remains unsupported (AA10).

use std::collections::BTreeSet;

use crate::grid::CellIterator;
use crate::rng::CrtRand;

/// Object size (§1).
pub const AMBIENCE_MANAGER_SIZE: usize = 0x120;
/// `kGZGUID_AudioAmbienceManagerSystemServiceId` (§1).
pub const SERVICE_ID: u32 = 0xAA12_F911;
/// Ambience layer object size (§2.2).
pub const AMBIENCE_LAYER_SIZE: usize = 0x25C;
/// Ambience layer IID (§8).
pub const IID_AMBIENCE_LAYER: u32 = 0x4A28_35FE;

/// Audio scape manager service / IID (§2.1 step 2).
pub const SCAPE_MANAGER_SERVICE: (u32, u32) = (0x49E5_0BA1, 0x29E3_9DB4);
/// `cIGZSndSys` service / IID (§2.1 step 3).
pub const SND_SYS_SERVICE: (u32, u32) = (0x23CE_CBA0, 0x1060_77D2);
/// Message server service / IID (§2.1 step 4).
pub const MESSAGE_SERVER_SERVICE: (u32, u32) = (0x04FA_845B, 0x6522_94C7);
/// Tracked-occupant set CLSID / IID (§2.1 step 5).
pub const TRACKED_SET_CLASS: (u32, u32) = (0x4A40_1BA7, 0x4A40_074C);

pub const MSG_POST_CITY_INIT: u32 = 0x26D3_1EC1;
pub const MSG_PRE_CITY_SHUTDOWN: u32 = 0x26D3_1EC2;
pub const MSG_LISTENER_POS_CHANGED: u32 = 0x8A4A_69C7;
pub const MSG_INSERT_OCCUPANT: u32 = 0x99EF_1142;
pub const MSG_MOVE_OCCUPANT: u32 = 0x99EF_1146;
pub const MSG_REMOVE_OCCUPANT: u32 = 0x99EF_1143;
pub const MSG_CONDITION_CHANGED: u32 = 0xE99B_3D4D;
pub const MSG_STATE_CHANGED: u32 = 0xE8BD_A2E7;
pub const MSG_AUTOMATA_EVENT: u32 = 0x09D9_2FC3;
pub const MSG_EXEMPLAR_CHANGED: u32 = 0xC53D_10AA;
pub const MSG_AUTOMATON_APPROACHING_INTERSECTION: u32 = 0x8A87_8385;
pub const MSG_APP_FOCUS: u32 = 0x4348_B111;

/// The 12 message types `Init` subscribes, in order (§2.1 step 4).
pub const SUBSCRIBED_MESSAGES: [u32; 12] = [
    MSG_POST_CITY_INIT,
    MSG_PRE_CITY_SHUTDOWN,
    MSG_LISTENER_POS_CHANGED,
    MSG_INSERT_OCCUPANT,
    MSG_MOVE_OCCUPANT,
    MSG_REMOVE_OCCUPANT,
    MSG_CONDITION_CHANGED,
    MSG_STATE_CHANGED,
    MSG_AUTOMATA_EVENT,
    MSG_EXEMPLAR_CHANGED,
    MSG_AUTOMATON_APPROACHING_INTERSECTION,
    MSG_APP_FOCUS,
];

/// Message payload IID every message must answer (§7).
pub const IID_MESSAGE_PAYLOAD: u32 = 0x6529_7976;
/// Occupant IID the occupant messages are QI'd with (§7).
pub const IID_OCCUPANT: u32 = 0x26BB_767E;
/// Lot IID of the condition-changed object (§3.4).
pub const IID_LOT: u32 = 0xE818_FCED;
/// Automaton IID (§3.1, §3.5).
pub const IID_AUTOMATON: u32 = 0xA9B4_0F05;
/// Freshness-player IID (§6).
pub const IID_FRESH_PLAYER: u32 = 0xDC1C_F595;

/// Sound-system events posted to `cIGZSndSys` slot `+0x2C` (§3).
pub const EVT_START_SOUND: u32 = 0xEA45_1903;
pub const EVT_STOP_SOUND: u32 = 0x6A45_1908;
pub const EVT_SET_POSITION: u32 = 0xCA45_190F;
/// Tag of the sound system's extended post (slot `+0x34`, §3.6).
pub const EXTENDED_POST_TAG: u32 = 0x5B52_3BA4;

/// Preview-occupant flag (§3).
pub const PROP_PREVIEW: u32 = 0x0292_44DB;
pub const PROP_AMBIENT_SOUND: u32 = 0x6A36_AFAC;
pub const PROP_SECOND_SOUND: u32 = 0xC9B9_3A56;
pub const PROP_ALARM_TIMES: u32 = 0x6A36_AFAB;
pub const PROP_ONE_SHOT_ALARM_SOUND: u32 = 0x0A36_AFA2;
pub const PROP_ON_OFF_ALARM_SOUND: u32 = 0x2A53_BE13;
pub const PROP_CONDITION_SOUND_A: u32 = 0xCA19_D7CA;
pub const PROP_CONDITION_SOUND_B: u32 = 0x0A4C_13CB;
pub const PROP_CONDITION_SOUND_C: u32 = 0x4A4C_132E;
pub const PROP_CROSSING_SOUND: u32 = 0xAA87_817A;

/// Occupant types of the city-init walk (§3.1; the last two are **AA4**).
pub const INIT_WALK_TYPES: [u32; 3] = [0x2781_28A0, 0xA823_821E, 0xC7CC_75F1];

/// Audio settings exemplar (§4.1).
pub const TUNING_EXEMPLAR: (u32, u32, u32) = (0x6534_284A, 0xCA25_875D, 0x6A35_9C5C);
pub const PROP_TUNING_MARGINS: u32 = 0x0A36_F410;
pub const PROP_TUNING_LAYER_IDS: u32 = 0x2A3F_F8CD;
/// Margin when `0x0A36F410` is missing (`0x428C0000`, §4.1).
pub const DEFAULT_MARGIN: f32 = 70.0;
/// Number of margins, indexed by zoom (§1 `+0x54`).
pub const MARGIN_COUNT: usize = 5;

/// `DoEvent` event ids (§6).
pub const EVT_MASTER_VOLUME: u32 = 0x2A5B_69C0;
pub const EVT_FRESH_STOP: u32 = 0x4A2C_0389;
pub const EVT_FRESH_PLAY_PERCENT: u32 = 0x0A41_4C5C;
pub const EVT_FRESH_LOAD_SCORE: u32 = 0x2A2C_0384;
pub const EVT_FRESH_FADE_OUT: u32 = 0x2A2C_0393;
pub const EVT_FRESH_SET_VOLUME: u32 = 0x4A3D_A208;
pub const EVT_FRESH_PLAY: u32 = 0xEA2C_0390;
pub const EVT_FRESH_MIN_MAX_DISTANCE: u32 = 0xEA2D_827C;

/// Automaton kinds whose sound codes 12/13 play (§3.5).
pub const AUTOMATON_SOUND_KINDS: [u32; 3] = [0x10, 0x14, 0x1E];

/// Layer-init failure log line (§2.2), `%08x` = the layer id.
pub fn layer_init_failure_message(layer_id: u32) -> String {
    format!(
        "Error : cSC4AudioAmbienceManager::_CreateAmbienceLayers : Failed to init layer ({:08x})\n",
        layer_id
    )
}

/// Freshness-score load failure log line (§6).
pub fn score_load_failure_message(t: u32, g: u32, i: u32) -> String {
    format!(
        "Error: cSC4AudioAmbienceLayer::DoEvent : Failed to load freshness score ({:08x}:{:08x}:{:08x})\n",
        t, g, i
    )
}

/// Occupant identity: the occupant's address in the host. Instance ids are
/// `occupant + sound id` (wrapping, §3).
pub type OccupantRef = u32;

/// Instance id of an occupant sound (§3).
pub fn occupant_instance_id(occupant: OccupantRef, sound_id: u32) -> u32 {
    occupant.wrapping_add(sound_id)
}

/// A sound-system event the manager posts (§3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SndRequest {
    /// `0xEA451903`.
    Start {
        sound_id: u32,
        instance_id: u32,
        looping: bool,
        position: [i32; 3],
    },
    /// `0x6A451908`.
    Stop { instance_id: u32 },
    /// `0xCA45190F`.
    SetPosition { instance_id: u32, position: [i32; 3] },
}

/// Which post a request goes through (§3, §3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostKind {
    /// Slot `+0x2C`.
    Normal,
    /// Slot `+0x34`, tag [`EXTENDED_POST_TAG`].
    Extended,
}

/// What the manager needs to know about one occupant, read by the host from
/// its property holder (slot `+0x18`) and position (slot `+0x20`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OccupantInfo {
    pub occupant: OccupantRef,
    /// `IsFlagSet(5)`: occupant flags 1 (in the occupant manager) and 4
    /// (visible) both set (§3 `_AddOccupantSndInstance`, §3.6).
    pub flags_1_and_4: bool,
    /// `IsFlagSet(0x10)` (§3.6 alarm creation).
    pub flag_0x10: bool,
    /// `0x029244DB` true.
    pub preview: bool,
    /// Answers [`IID_AUTOMATON`].
    pub is_automaton: bool,
    /// Position (f32 in the game; truncated to integers by the manager).
    pub position: [f32; 3],
    pub ambient_sound: Option<u32>,
    pub second_sound: Option<u32>,
    pub alarm_times: Option<Vec<f32>>,
    pub one_shot_alarm_sound: Option<u32>,
    pub on_off_alarm_sound: Option<u32>,
    pub condition_sound_a: Option<u32>,
    pub condition_sound_b: Option<u32>,
    pub condition_sound_c: Option<u32>,
    pub crossing_sound: Option<u32>,
}

impl OccupantInfo {
    fn int_position(&self) -> [i32; 3] {
        truncate(self.position)
    }
}

/// f32 → integer truncation of a position (§3).
pub fn truncate(p: [f32; 3]) -> [i32; 3] {
    [p[0] as i32, p[1] as i32, p[2] as i32]
}

/// Tuning read from the audio settings exemplar (§4.1).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AmbienceTuning {
    /// `0x0A36F410`; `None` = missing.
    pub margins: Option<Vec<f32>>,
    /// `0x2A3FF8CD`; `None` = missing.
    pub layer_ids: Option<Vec<u32>>,
}

/// A cell rectangle: x0, z0, x1, z1 (§1 `+0x6C`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CellRect {
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
}

/// A world rectangle (§1 `+0x8C..+0xA8`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WorldRect {
    pub x0: f32,
    pub z0: f32,
    pub x1: f32,
    pub z1: f32,
}

/// One call made on the freshness player (§6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FreshCall {
    /// Slot 10.
    Stop,
    /// Slot 7.
    Shutdown,
    /// Slot 16, raw event value.
    SetPlayPercent(u32),
    /// Slot 12: `FadeVolume(3000, 0, 1)`.
    FadeVolume { ms: u32, target: u32, flag: u32 },
    /// Slot 11, raw event value.
    SetVolume(u32),
    /// Slot 28.
    SetHeadRelative(bool),
    /// Slot 8.
    Play,
    /// Slot 29, the payload's two raw values.
    SetMinMaxDistance(u32, u32),
}

/// A sound-system event delivered to the manager's `DoEvent` (§6).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SndEventIn {
    pub id: u32,
    /// The event's value (raw 32-bit word).
    pub value: u32,
    /// Second payload value (`0xEA2D827C`).
    pub value2: u32,
    /// The data object (slot `+0x18`, then `+0x80`) answers [`IID_FRESH_PLAYER`].
    pub fresh_player: bool,
    /// Event type/group/instance (`0x2A2C0384`); `None` = the event itself
    /// does not answer [`IID_MESSAGE_PAYLOAD`] (AA9b).
    pub score_tgi: Option<(u32, u32, u32)>,
}

/// An automaton event message `0x09D92FC3` (§3.5).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AutomatonEvent {
    /// Message slot `+0x38`.
    pub code: u32,
    /// Message slot `+0x4C`.
    pub kind: u32,
    /// The automaton's slot `+0xE4(0x10)`.
    pub slot_e4_0x10: u32,
    /// The automaton's position (slot `+0x1C`), used by code 9.
    pub automaton_position: [f32; 3],
    /// The automaton's occupant (slot `+0x14`), used by code 9; `None` =
    /// null.
    pub occupant: Option<OccupantInfo>,
    /// The automaton's occupant base `A + 4`, used by codes 12/13
    /// (`_PlayOccupantSound`); `None` = null.
    pub base_occupant: Option<OccupantInfo>,
}

/// The automaton of `0x8A878385` (§3.5), when the message object answers
/// [`IID_AUTOMATON`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CrossingAutomaton {
    /// The automaton interface address `A`: the instance id (AA8e).
    pub automaton: u32,
    /// `A` slot `+0x1C`.
    pub position: [f32; 3],
    /// `0xAA87817A` read from `A`'s occupant (slot `+0x14`); `None` = absent.
    pub crossing_sound: Option<u32>,
}

/// One occupant of a lot as `_UpdateLotOccupantsForConditionChange` sees it
/// (§3.4).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LotOccupant {
    /// Answers `cISC4BuildingOccupant` (`0x87DDA3A9`) or `cISC4PropOccupant`
    /// (`0xE9793A65`).
    pub building_or_prop: bool,
    /// The property holder (slot `+0x18`) is non-null.
    pub has_holder: bool,
    pub info: OccupantInfo,
}

/// The messages `DoMessage` dispatches (§7). Each carries the inputs the host
/// resolved from the message object.
#[derive(Debug, Clone, PartialEq)]
pub enum AmbienceMessage {
    PostCityInit,
    PreCityShutdown,
    ListenerPosChanged,
    ExemplarChanged,
    AppFocus {
        flag: bool,
    },
    /// `occupant` is `None` when the message object does not answer
    /// [`IID_OCCUPANT`].
    Occupant {
        msg_type: u32,
        occupant: Option<OccupantInfo>,
    },
    /// `lot` is `None` when the object does not answer [`IID_LOT`].
    ConditionChanged {
        lot: Option<u32>,
        values: (u32, u32),
    },
    AutomataEvent(AutomatonEvent),
    /// `automaton` is `None` when the object does not answer [`IID_AUTOMATON`].
    ApproachingIntersection {
        automaton: Option<CrossingAutomaton>,
    },
    Other(u32),
}

/// Foreign ambience layer (§8). Its algorithms belong to the layer owner.
pub trait AmbienceLayer {
    /// Slot 4.
    fn init(&mut self) -> bool;
    /// Slot 5 (`0x004E9660`, before the layer's `Release`).
    fn shutdown(&mut self);
    /// Slot 17 (raw volume word).
    fn set_score_master_volume(&mut self, volume: u32);
    /// Slot 20.
    fn handle_cell(&mut self, x: i32, z: i32);
    /// Slot 21, through the cell walk callback.
    fn handle_occupant(&mut self, occupant: OccupantRef);
    /// Slot 25 with the attention rectangle.
    fn begin_update(&mut self, rect: WorldRect);
    /// Slot 26.
    fn update(&mut self, elapsed_ms: u32, listener_pos: [f32; 3], listener_vec: [f32; 3]);
    /// Slot 27.
    fn end_update(&mut self);
    /// Slot 28.
    fn pause(&mut self, flag: bool);
}

/// The native services the manager uses.
pub trait AmbienceHost {
    type Layer: AmbienceLayer;

    // ---- Init / Shutdown (§2.1, §2.4)
    /// Resolve the framework and call its slot 3; false = absent.
    fn framework_init(&mut self) -> bool;
    fn acquire_scape_manager(&mut self) -> bool;
    fn acquire_snd_sys(&mut self) -> bool;
    /// Sound-system slot `+0x3C` / `+0x40`.
    fn register_event_target(&mut self);
    fn unregister_event_target(&mut self);
    /// False = no message server.
    fn acquire_message_server(&mut self) -> bool;
    fn subscribe(&mut self, msg_type: u32);
    fn unsubscribe(&mut self, msg_type: u32);
    fn create_tracked_set(&mut self) -> bool;
    /// Start/restart the millisecond tick timer (§2.1, §5). `on_tick` takes
    /// the elapsed value sampled immediately before this call.
    fn restart_tick_timer(&mut self);
    fn release_services(&mut self);
    /// Framework tick list (slots `+0x24` / `+0x28`).
    fn add_to_tick_list(&mut self);
    fn remove_from_tick_list(&mut self);
    /// Create the city occupant-walk filter (§2.2 step 5). The host owns it.
    fn create_occupant_filter(&mut self);
    /// Consume the process-wide first-attention-update byte (§4.2 step 5).
    /// This state belongs to the host so it survives manager recreation.
    fn take_first_attention_update(&mut self) -> bool;

    // ---- sound system
    fn post(&mut self, kind: PostKind, request: SndRequest);
    /// `cSC4AudioSoundManager::GenerateNewInstanceId` (audio.md §8.4).
    fn generate_instance_id(&mut self) -> u32;

    // ---- tuning, preferences, layers
    fn read_tuning(&mut self) -> AmbienceTuning;
    /// Audio preferences: (byte `+4` set, volume word); `None` = absent.
    fn audio_preferences(&mut self) -> Option<(bool, u32)>;
    /// Allocate a layer for an exemplar id (not yet `Init`ed).
    fn create_layer(&mut self, layer_id: u32) -> Self::Layer;
    fn log(&mut self, line: &str);

    // ---- city / view / listener
    /// City cell counts (slots `+0x174`, `+0x178`).
    fn city_cell_counts(&mut self) -> (i32, i32);
    /// Occupant-manager slot `+0x48`.
    fn cell_to_world(&mut self, x: i32, z: i32) -> (f32, f32);
    /// Occupant-manager slot `+0x4C`.
    fn world_to_cell(&mut self, x: f32, z: f32) -> (i32, i32);
    /// View-utilities `+0x0C`; `None` = no view-utilities global (AA6d).
    fn zoom_index(&mut self) -> Option<u32>;
    /// Audio director listener: (position words 9-11, vector words 3-5).
    fn listener(&mut self) -> ([f32; 3], [f32; 3]);

    // ---- occupants and lots
    /// Occupant-manager slot `+0xB0` with a type filter.
    fn occupants_of_types(&mut self, types: &[u32]) -> Vec<OccupantInfo>;
    /// Occupant-manager slot `+0xBC` with the city filter, one cell.
    /// Return only occupants admitted by that foreign filter; release native
    /// list/interface references inside the host before returning owned IDs.
    fn occupants_in_cell(&mut self, x: i32, z: i32) -> Vec<OccupantRef>;
    /// Lot manager slot `+0x38`.
    fn lot_of_occupant(&mut self, occupant: OccupantRef) -> Option<u32>;
    /// Lot slot `+0x8C` (`GetCondition`, byte).
    fn lot_condition(&mut self, lot: u32) -> u8;
    /// Lot slot `+0xC4` (`GetLotOccupants`), in list order (§3.4).
    fn lot_occupants(&mut self, lot: u32) -> Vec<LotOccupant>;
    /// Automaton slot `+0x9C` / `+0xA0`.
    fn link_automaton(&mut self, occupant: OccupantRef);
    fn unlink_automaton(&mut self, occupant: OccupantRef);
    /// The occupant as it is now (flags, position, properties re-read), for
    /// the alarm clock (§3.6). Properties are `None` when the holder is
    /// absent or the property does not read.
    fn occupant_info(&mut self, occupant: OccupantRef) -> OccupantInfo;

    // ---- clock / freshness
    /// Simulation clock (hour, minute); `None` = absent.
    fn clock(&mut self) -> Option<(u32, u32)>;
    fn fresh_player_call(&mut self, call: FreshCall);
    /// Load a freshness score; false = failure.
    fn fresh_player_load_score(&mut self, t: u32, g: u32, i: u32) -> bool;
}

/// Alarm kind byte `+0x25` (§3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlarmKind {
    OneShot = 0,
    OnOff = 1,
}

/// One alarm entry (§3.6).
#[derive(Debug, Clone, PartialEq)]
pub struct AlarmEntry {
    pub occupant: OccupantRef,
    pub sound_id: u32,
    /// `+0x10`: the time of day at creation (AA5), then the last tick's `t`.
    pub last: f32,
    pub times: Vec<f32>,
    /// `+0x24`.
    pub on: bool,
    pub kind: AlarmKind,
}

/// `OnTick` stage `+0xB0` (§5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Begin0,
    Begin1,
    Walk,
    End,
    /// Stage 4 (focus lost, AA9a): no `OnTick` case.
    Unfocused,
}

/// The manager state (§1).
pub struct AmbienceManager<L> {
    pub ref_count: u32,
    pub running: bool,
    pub layers: Vec<L>,
    pub zoom: u32,
    pub listener_pos: [f32; 3],
    pub listener_vec: [f32; 3],
    pub alarms: Vec<AlarmEntry>,
    /// `+0x50`.
    pub ticking: bool,
    /// `+0x54` (a `std::vector<float>`, empty after the constructor, AA6).
    pub margins: Vec<f32>,
    /// `+0x60` (a `std::vector<u32>`, empty after the constructor, AA6).
    pub layer_ids: Vec<u32>,
    /// `+0x20` `cIGZSndSys` held (Init step 3 until `Shutdown`).
    pub snd_sys: bool,
    pub current: CellRect,
    pub previous: CellRect,
    pub attention_world: WorldRect,
    pub city_world: WorldRect,
    /// `+0xAC`.
    pub city_ready: bool,
    pub stage: Stage,
    /// `+0xB4` the block `RandomIterator` (prime-step mode, AA7); its
    /// `current` carries over between walks and cities.
    pub iterator: CellIterator,
    /// `+0x118` (`None` until created) and `+0x11C`.
    pub tracked: Option<BTreeSet<OccupantRef>>,
    pub tracked_count: i32,
    pub cell_counts: (i32, i32),
    /// Last unsupported input. No substitute alarm entry is manufactured.
    pub unsupported: Option<Unsupported>,
}

/// Behaviour explicitly unsettled in the spec (§9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unsupported {
    /// AA10: native alarm creation faults without a simulation clock;
    /// replacement policy is still open. All other insert work continues.
    AlarmCreationWithoutClock,
}

impl<L: AmbienceLayer> Default for AmbienceManager<L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: AmbienceLayer> AmbienceManager<L> {
    /// Constructor (§1 initial values).
    pub fn new() -> Self {
        Self {
            ref_count: 0,
            running: false,
            layers: Vec::new(),
            zoom: 0,
            listener_pos: [0.0; 3],
            listener_vec: [0.0; 3],
            alarms: Vec::new(),
            ticking: true,
            margins: Vec::new(),
            layer_ids: Vec::new(),
            snd_sys: false,
            current: CellRect::default(),
            previous: CellRect::default(),
            attention_world: WorldRect::default(),
            city_world: WorldRect::default(),
            city_ready: false,
            stage: Stage::Begin0,
            iterator: CellIterator::constructed(),
            tracked: None,
            tracked_count: 0,
            cell_counts: (0, 0),
            unsupported: None,
        }
    }

    /// `Init` (§2.1).
    pub fn init<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H) -> bool {
        if !host.framework_init() || !host.acquire_scape_manager() || !host.acquire_snd_sys() {
            return false;
        }
        self.snd_sys = true;
        host.register_event_target();
        // The spec does not say what an absent message server does: the
        // literal reading of "any failure returns 0" *[choice]*.
        if !host.acquire_message_server() {
            return false;
        }
        for t in SUBSCRIBED_MESSAGES {
            host.subscribe(t);
        }
        if !host.create_tracked_set() {
            return false;
        }
        self.tracked = Some(BTreeSet::new());
        host.restart_tick_timer();
        self.ticking = true;
        self.running = true;
        true
    }

    /// `Shutdown` (§2.4). The unsubscribe order differs in the game; the set
    /// is the same and the order is not given, so §2.1's order is used.
    pub fn shutdown<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H) -> bool {
        host.unregister_event_target();
        for t in SUBSCRIBED_MESSAGES {
            host.unsubscribe(t);
        }
        self.release_layers();
        host.release_services();
        self.snd_sys = false;
        self.running = false;
        true
    }

    /// `0x004E9660` (AA9c): each layer's `Shutdown` then `Release`, in list
    /// order; only the layer list is emptied. The alarm list and the tracked
    /// set survive *[compat]*. Returns 1.
    fn release_layers(&mut self) -> bool {
        for mut l in self.layers.drain(..) {
            l.shutdown();
        }
        true
    }

    /// `_ReadTuningValues` (§4.1, AA6). Absent IDs retain their vector;
    /// absent/short margins use the spec's safe replacement policy.
    pub fn read_tuning<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H) {
        let t = host.read_tuning();
        match t.margins {
            Some(v) => {
                self.margins = v;
                // Fewer than 5 values: the missing entries are 70.0 (the game
                // reads past the vector's end) *[choice, §4.1]*.
                while self.margins.len() < MARGIN_COUNT {
                    self.margins.push(DEFAULT_MARGIN);
                }
            }
            None => self.margins = vec![DEFAULT_MARGIN; MARGIN_COUNT],
        }
        if let Some(ids) = t.layer_ids {
            self.layer_ids = ids;
        }
    }

    /// City init (§2.2).
    pub fn handle_city_init<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H) -> bool {
        self.read_tuning(host);
        let ids = self.layer_ids.clone();
        for id in ids {
            let mut layer = host.create_layer(id);
            if !layer.init() {
                host.log(&layer_init_failure_message(id));
                continue;
            }
            let volume = match host.audio_preferences() {
                Some((true, v)) => v,
                // Absent preferences: volume 0, the §2.2 replacement choice.
                _ => 0,
            };
            layer.set_score_master_volume(volume);
            self.layers.push(layer);
        }
        let (nx, nz) = host.city_cell_counts();
        self.cell_counts = (nx, nz);
        let (ax, az) = host.cell_to_world(0, 0);
        let (bx, bz) = host.cell_to_world(nx - 1, nz - 1);
        self.city_world = WorldRect {
            x0: ax,
            z0: az,
            x1: bx,
            z1: bz,
        };
        self.city_ready = true;
        self.update_attention_rect(host);
        host.create_occupant_filter();
        self.reset_iterator();
        self.stage = Stage::Begin0;
        host.add_to_tick_list();
        for occ in host.occupants_of_types(&INIT_WALK_TYPES) {
            self.handle_occupant_common(host, &occ, false);
        }
        self.ticking = true;
        true
    }

    /// City shutdown (§2.3).
    pub fn handle_city_shutdown<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H) -> bool {
        self.ticking = false;
        host.remove_from_tick_list();
        // AA9c: the alarms are kept (stale entries reach the next city)
        // *[compat; the spec's "should drop" is a choice not taken]*.
        self.release_layers()
    }

    /// `_UpdateAttentionRect` (§4.2).
    pub fn update_attention_rect<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H) {
        let (pos, vec) = host.listener();
        self.listener_pos = pos;
        self.listener_vec = vec;
        // AA6d: the game checks neither the view global nor the margin
        // vector; no global, an empty vector or a zoom past its end keeps the
        // previous rectangle *[choice, §4.2]*.
        let Some(zoom) = host.zoom_index() else {
            return;
        };
        self.zoom = zoom;
        // §4.2's safe replacement policy admits only city-view zooms 0–4,
        // even when a tuning property contains more than five margins.
        if zoom >= MARGIN_COUNT as u32 {
            return;
        }
        let Some(m) = self.margins.get(zoom as usize).copied() else {
            return;
        };
        let (x, z) = (pos[0], pos[2]);
        self.attention_world = WorldRect {
            x0: x - m,
            z0: z - m,
            x1: x + m,
            z1: z + m,
        };
        let (cx0, cz0) = host.world_to_cell(x - m, z - m);
        let (cx1, cz1) = host.world_to_cell(x + m, z + m);
        let (nx, nz) = self.cell_counts;
        let fix = |v: i32, n: i32| v.clamp(0, (n - 1).max(0)) & !1;
        let r = CellRect {
            x0: fix(cx0, nx),
            z0: fix(cz0, nz),
            x1: fix(cx1, nx),
            z1: fix(cz1, nz),
        };
        if host.take_first_attention_update() {
            self.previous = r;
        } else {
            self.previous = self.current;
        }
        self.current = r;
    }

    /// `Reset(0)` (`0x0049D1C0`, AA7): no shuffle, so no `rand()` and
    /// `current` is not reseeded. The `CrtRand` is never drawn from.
    fn reset_iterator(&mut self) {
        let mut unused = CrtRand::default();
        self.iterator.reset(false, &mut unused);
    }

    /// Stage 0/1 (§5, AA7): `SetClipRect` with the current rectangle halved
    /// by an arithmetic shift, inclusive bounds (x1/z1 raised to x0/z0 when
    /// smaller), then `Reset(0)`.
    fn seed_iterator(&mut self) {
        let r = self.current;
        let (x0, z0) = (r.x0 >> 1, r.z0 >> 1);
        let (x1, z1) = ((r.x1 >> 1).max(x0), (r.z1 >> 1).max(z0));
        self.iterator.set_rect(x0, z0, x1, z1);
        self.reset_iterator();
    }

    /// `OnTick` (§5). `elapsed_ms` is the tick timer's elapsed time;
    /// `budget` answers "may another block be taken" before each block
    /// (§5 allows any budget) *[choice]*.
    pub fn on_tick<H: AmbienceHost<Layer = L>>(
        &mut self,
        host: &mut H,
        elapsed_ms: u32,
        mut budget: impl FnMut() -> bool,
    ) {
        if !self.ticking {
            return;
        }
        host.restart_tick_timer();
        self.update_alarms(host);
        // Stage 4 (`Unfocused`) matches no case below *[compat, AA9a]*.
        if matches!(self.stage, Stage::Begin0 | Stage::Begin1) {
            self.seed_iterator();
            // Literal world-space attention rectangle (AA12); the separate
            // cell rectangle seeds the iterator.
            let r = self.attention_world;
            for l in &mut self.layers {
                l.begin_update(r);
            }
            self.stage = Stage::Walk;
        }
        if self.stage == Stage::Walk && self.do_stage_update(host, elapsed_ms, &mut budget) {
            self.stage = Stage::End;
        }
        if self.stage == Stage::End {
            for l in &mut self.layers {
                l.end_update();
            }
            self.stage = Stage::Begin1;
        }
    }

    /// `_DoStageUpdate` (§5): returns true when the walk finished.
    fn do_stage_update<H: AmbienceHost<Layer = L>>(
        &mut self,
        host: &mut H,
        elapsed_ms: u32,
        budget: &mut dyn FnMut() -> bool,
    ) -> bool {
        let mut finished = false;
        // §5 guarantees progress even with an exhausted budget at entry.
        let mut first = true;
        while first || budget() {
            first = false;
            let Some((bx, bz)) = self.iterator.next_cell() else {
                finished = true;
                break;
            };
            let cells = [
                (2 * bx, 2 * bz),
                (2 * bx + 1, 2 * bz),
                (2 * bx, 2 * bz + 1),
                (2 * bx + 1, 2 * bz + 1),
            ];
            for &(x, z) in &cells {
                for occ in host.occupants_in_cell(x, z) {
                    for l in &mut self.layers {
                        l.handle_occupant(occ);
                    }
                }
            }
            for &(x, z) in &cells {
                for l in &mut self.layers {
                    l.handle_cell(x, z);
                }
            }
        }
        let (p, v) = (self.listener_pos, self.listener_vec);
        for l in &mut self.layers {
            l.update(elapsed_ms, p, v);
        }
        finished
    }

    // ---- occupants (§3)

    /// `_AddOccupantSndInstance(o, id, loop)` (`0x004E8AD0`, §3) in full.
    /// Returns false when nothing was posted for a missing flag or a preview
    /// occupant.
    fn add_occupant_snd_instance<H: AmbienceHost<Layer = L>>(
        &mut self,
        host: &mut H,
        occ: &OccupantInfo,
        sound_id: u32,
        looping: bool,
    ) -> bool {
        if occ.occupant == 0 || !occ.flags_1_and_4 || occ.preview {
            return false;
        }
        if occ.is_automaton {
            host.link_automaton(occ.occupant);
        }
        if self.snd_sys {
            let instance_id = occupant_instance_id(occ.occupant, sound_id);
            let position = occ.int_position();
            host.post(
                PostKind::Normal,
                SndRequest::Start {
                    sound_id,
                    instance_id,
                    looping,
                    position,
                },
            );
            host.post(PostKind::Normal, SndRequest::SetPosition { instance_id, position });
        }
        true
    }

    /// `_RemoveOccupantSndInstance(o, id)` (`0x004E8BF0`, §3).
    fn remove_occupant_snd_instance<H: AmbienceHost<Layer = L>>(
        &mut self,
        host: &mut H,
        occ: &OccupantInfo,
        sound_id: u32,
    ) -> bool {
        if occ.occupant == 0 {
            return false;
        }
        if occ.is_automaton {
            host.unlink_automaton(occ.occupant);
        }
        if self.snd_sys {
            host.post(
                PostKind::Normal,
                SndRequest::Stop {
                    instance_id: occupant_instance_id(occ.occupant, sound_id),
                },
            );
        }
        true
    }

    /// Insert (`insert = true`) and the init walk (§3.1).
    fn handle_occupant_common<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H, occ: &OccupantInfo, insert: bool) {
        if occ.preview {
            return;
        }
        if let Some(set) = self.tracked.as_mut() {
            if set.insert(occ.occupant) {
                self.tracked_count += 1;
            }
        }
        if insert {
            if let Some(s) = occ.ambient_sound {
                self.add_occupant_snd_instance(host, occ, s, true);
            }
        }
        if let Some(times) = &occ.alarm_times {
            if let Some(s) = occ.one_shot_alarm_sound {
                self.add_alarm(host, occ, s, times, AlarmKind::OneShot);
            }
            if let Some(s) = occ.on_off_alarm_sound {
                self.add_alarm(host, occ, s, times, AlarmKind::OnOff);
            }
        }
        if insert {
            if let Some(s) = occ.second_sound {
                // AA8a: start once (loop 0), instance `o + value`.
                self.add_occupant_snd_instance(host, occ, s, false);
            }
        }
        if occ.condition_sound_a.is_some() || occ.condition_sound_b.is_some() || occ.condition_sound_c.is_some() {
            let lot = host.lot_of_occupant(occ.occupant);
            self.update_lot_condition(host, lot, -1, -1);
        }
    }

    /// `_UpdateLotOccupantsForConditionChange(lot, old, new)` (`0x004E9DA0`,
    /// §3.4). Names: spec `A` = `0x0A4C13CB` (`condition_sound_b`), `C` =
    /// `0xCA19D7CA` (`condition_sound_a`), `R` = `0x4A4C132E`
    /// (`condition_sound_c`).
    fn update_lot_condition<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H, lot: Option<u32>, old: i32, new: i32) {
        let Some(lot) = lot else {
            return;
        };
        let new = if new < 0 {
            i32::from(host.lot_condition(lot))
        } else {
            new
        };
        for lo in host.lot_occupants(lot) {
            // No preview check here *[compat]*; `_AddOccupantSndInstance`
            // makes its own.
            if !lo.building_or_prop || !lo.has_holder {
                continue;
            }
            let o = &lo.info;
            let (a, c, r) = (o.condition_sound_b, o.condition_sound_a, o.condition_sound_c);
            match new {
                0 | 5 | 6 => {
                    if let Some(a) = a {
                        self.remove_occupant_snd_instance(host, o, a);
                    }
                    if let Some(c) = c {
                        self.add_occupant_snd_instance(host, o, c, true);
                    }
                }
                2 | 3 => {
                    if let Some(c) = c {
                        self.remove_occupant_snd_instance(host, o, c);
                    }
                    if let Some(a) = a {
                        self.add_occupant_snd_instance(host, o, a, true);
                    }
                }
                1 | 4 => {
                    if let Some(a) = a {
                        self.remove_occupant_snd_instance(host, o, a);
                    }
                    if let Some(c) = c {
                        self.remove_occupant_snd_instance(host, o, c);
                    }
                }
                _ => {}
            }
            if new == 0 && old == 1 {
                if let Some(r) = r {
                    self.add_occupant_snd_instance(host, o, r, true);
                }
            }
        }
    }

    /// `_AddAlarmOccupant(o, id, &times, kind)` (`0x004EA690`, §3.6, AA5).
    fn add_alarm<H: AmbienceHost<Layer = L>>(
        &mut self,
        host: &mut H,
        occ: &OccupantInfo,
        sound_id: u32,
        times: &[f32],
        kind: AlarmKind,
    ) -> bool {
        if occ.occupant == 0 || !occ.flags_1_and_4 || occ.flag_0x10 {
            return false;
        }
        let Some((hour, minute)) = host.clock() else {
            self.unsupported = Some(Unsupported::AlarmCreationWithoutClock);
            return false;
        };
        let last = time_of_day(hour, minute);
        self.alarms.push(AlarmEntry {
            occupant: occ.occupant,
            sound_id,
            last,
            times: times.to_vec(),
            on: false,
            kind,
        });
        true
    }

    /// `_RemoveAlarmOccupant`: the entry of this occupant and kind.
    fn remove_alarm(&mut self, occupant: OccupantRef, kind: AlarmKind) {
        // §3.3 says remove the alarm entry without describing duplicate
        // matching. Literal singular: first occupant/kind entry (AA13).
        if let Some(i) = self
            .alarms
            .iter()
            .position(|a| a.occupant == occupant && a.kind == kind)
        {
            self.alarms.remove(i);
        }
    }

    /// Move (§3.2, AA8b): both set-positions carry the **bare occupant
    /// address** as instance id *[quirk]*.
    fn handle_move<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H, occ: &OccupantInfo) {
        if occ.preview {
            return;
        }
        let position = occ.int_position();
        if occ.ambient_sound.is_some() && self.snd_sys {
            host.post(
                PostKind::Normal,
                SndRequest::SetPosition {
                    instance_id: occ.occupant,
                    position,
                },
            );
        }
        // AA11: the two posts are independent, each requiring the sound system.
        if occ.condition_sound_a.is_some() && self.snd_sys {
            host.post(
                PostKind::Normal,
                SndRequest::SetPosition {
                    instance_id: occ.occupant,
                    position,
                },
            );
        }
    }

    /// Remove (§3.3).
    fn handle_remove<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H, occ: &OccupantInfo) {
        if occ.preview {
            return;
        }
        if let Some(set) = self.tracked.as_mut() {
            if set.remove(&occ.occupant) {
                self.tracked_count -= 1;
            }
        }
        for s in [occ.condition_sound_b, occ.condition_sound_a, occ.ambient_sound]
            .into_iter()
            .flatten()
        {
            self.remove_occupant_snd_instance(host, occ, s);
        }
        if occ.one_shot_alarm_sound.is_some() {
            self.remove_alarm(occ.occupant, AlarmKind::OneShot);
        }
        if occ.on_off_alarm_sound.is_some() {
            self.remove_alarm(occ.occupant, AlarmKind::OnOff);
        }
    }

    /// Automaton event (§3.5). No preview or flag test (AA8f).
    fn handle_automaton<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H, ev: &AutomatonEvent) {
        match ev.code {
            9 => {
                if !self.snd_sys {
                    return;
                }
                let Some(o) = &ev.occupant else {
                    return;
                };
                let Some(id) = o.ambient_sound else {
                    return;
                };
                host.post(
                    PostKind::Normal,
                    SndRequest::SetPosition {
                        instance_id: occupant_instance_id(o.occupant, id),
                        position: truncate(ev.automaton_position),
                    },
                );
            }
            12 | 13 if AUTOMATON_SOUND_KINDS.contains(&ev.kind) => {
                let start = ev.code == 13 && (ev.kind != 0x10 || ev.slot_e4_0x10 == 0);
                self.play_occupant_sound(host, ev.base_occupant.as_ref(), start);
            }
            _ => {}
        }
    }

    /// `_PlayOccupantSound(o, start)` (`0x004E8720`, §3.5, AA8c): start flag
    /// 1, no automaton link.
    fn play_occupant_sound<H: AmbienceHost<Layer = L>>(
        &mut self,
        host: &mut H,
        occ: Option<&OccupantInfo>,
        start: bool,
    ) {
        let Some(o) = occ else {
            return;
        };
        if !self.snd_sys {
            return;
        }
        let Some(id) = o.ambient_sound else {
            return;
        };
        let instance_id = occupant_instance_id(o.occupant, id);
        if start {
            let position = o.int_position();
            host.post(
                PostKind::Normal,
                SndRequest::Start {
                    sound_id: id,
                    instance_id,
                    looping: true,
                    position,
                },
            );
            host.post(PostKind::Normal, SndRequest::SetPosition { instance_id, position });
        } else {
            host.post(PostKind::Normal, SndRequest::Stop { instance_id });
        }
    }

    /// Railway crossing (§3.5, AA8e): instance id = the automaton interface
    /// address, flag 1. Returns 1.
    fn handle_crossing<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H, a: Option<&CrossingAutomaton>) -> bool {
        if !self.snd_sys {
            return true;
        }
        let Some(a) = a else {
            return true;
        };
        let Some(id) = a.crossing_sound else {
            return true;
        };
        let instance_id = a.automaton;
        let position = truncate(a.position);
        host.post(
            PostKind::Normal,
            SndRequest::Start {
                sound_id: id,
                instance_id,
                looping: true,
                position,
            },
        );
        host.post(PostKind::Normal, SndRequest::SetPosition { instance_id, position });
        true
    }

    /// `_UpdateAlarms` (§3.6).
    pub fn update_alarms<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H) {
        let Some((hour, minute)) = host.clock() else {
            return;
        };
        let t = time_of_day(hour, minute);
        let mut alarms = std::mem::take(&mut self.alarms);
        for a in &mut alarms {
            let n = a.times.len();
            for i in 0..n {
                let ai = a.times[i];
                let next = a.times[(i + 1) % n];
                match a.kind {
                    AlarmKind::OneShot => {
                        if a.last < ai && ai <= t {
                            let instance_id = host.generate_instance_id();
                            let position = truncate(host.occupant_info(a.occupant).position);
                            host.post(
                                PostKind::Extended,
                                SndRequest::Start {
                                    sound_id: a.sound_id,
                                    instance_id,
                                    looping: false,
                                    position,
                                },
                            );
                            host.post(PostKind::Extended, SndRequest::SetPosition { instance_id, position });
                        }
                    }
                    AlarmKind::OnOff => {
                        let inside =
                            (ai <= t && t < next) || (next < ai && t >= ai) || (t - 24.0 >= ai && t - 24.0 < next);
                        if !inside {
                            continue;
                        }
                        // AA8d: `0x2A53BE13` is re-read; no holder or no
                        // value → nothing, `on` unchanged *[compat]*.
                        let o = host.occupant_info(a.occupant);
                        let Some(v) = o.on_off_alarm_sound else {
                            continue;
                        };
                        if i % 2 == 0 {
                            if !a.on {
                                // `on := 1` whatever the call returned.
                                self.add_occupant_snd_instance(host, &o, v, true);
                                a.on = true;
                            }
                        } else if a.on {
                            self.remove_occupant_snd_instance(host, &o, v);
                            a.on = false;
                        }
                    }
                }
            }
            a.last = t;
        }
        self.alarms = alarms;
    }

    // ---- events (§6) and messages (§7)

    /// `DoEvent` (§6): true when handled.
    pub fn do_event<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H, ev: &SndEventIn) -> bool {
        if ev.id == EVT_MASTER_VOLUME {
            for l in &mut self.layers {
                l.set_score_master_volume(ev.value);
            }
            // AA9b: 0 after setting the volumes *[compat]*.
            return false;
        }
        if !ev.fresh_player {
            return false;
        }
        match ev.id {
            EVT_FRESH_STOP => {
                host.fresh_player_call(FreshCall::Stop);
                host.fresh_player_call(FreshCall::Shutdown);
            }
            EVT_FRESH_PLAY_PERCENT => host.fresh_player_call(FreshCall::SetPlayPercent(ev.value)),
            EVT_FRESH_LOAD_SCORE => {
                // AA9b: the event itself is QI'd for 0x65297976; when it
                // does not answer, nothing is loaded or logged and the
                // result is still 1 *[compat]*.
                if let Some((t, g, i)) = ev.score_tgi {
                    if !host.fresh_player_load_score(t, g, i) {
                        host.log(&score_load_failure_message(t, g, i));
                    }
                }
            }
            EVT_FRESH_FADE_OUT => host.fresh_player_call(FreshCall::FadeVolume {
                ms: 3000,
                target: 0,
                flag: 1,
            }),
            EVT_FRESH_SET_VOLUME => host.fresh_player_call(FreshCall::SetVolume(ev.value)),
            EVT_FRESH_PLAY => {
                host.fresh_player_call(FreshCall::SetHeadRelative(ev.value != 0));
                host.fresh_player_call(FreshCall::Play);
            }
            EVT_FRESH_MIN_MAX_DISTANCE => host.fresh_player_call(FreshCall::SetMinMaxDistance(ev.value, ev.value2)),
            _ => return false,
        }
        true
    }

    /// Nullable sound-system event boundary (§6, AA9b).
    pub fn do_optional_event<H: AmbienceHost<Layer = L>>(&mut self, host: &mut H, event: Option<&SndEventIn>) -> bool {
        event.is_some_and(|event| self.do_event(host, event))
    }

    /// `DoMessage` (§7). `is_payload`: the message answers
    /// [`IID_MESSAGE_PAYLOAD`].
    pub fn do_message<H: AmbienceHost<Layer = L>>(
        &mut self,
        host: &mut H,
        is_payload: bool,
        msg: &AmbienceMessage,
    ) -> bool {
        if !is_payload {
            return false;
        }
        match msg {
            AmbienceMessage::PostCityInit => self.handle_city_init(host),
            AmbienceMessage::PreCityShutdown => self.handle_city_shutdown(host),
            AmbienceMessage::AppFocus { flag } => {
                // AA9a: stage 1 on focus gained, 4 on focus lost; then
                // `Pause(f == 0)` on every layer; returns 1.
                self.stage = if *flag { Stage::Begin1 } else { Stage::Unfocused };
                for l in &mut self.layers {
                    l.pause(!*flag);
                }
                true
            }
            AmbienceMessage::ListenerPosChanged => {
                self.update_attention_rect(host);
                true
            }
            AmbienceMessage::ExemplarChanged => {
                self.read_tuning(host);
                true
            }
            AmbienceMessage::AutomataEvent(ev) => {
                self.handle_automaton(host, ev);
                true
            }
            AmbienceMessage::ApproachingIntersection { automaton } => self.handle_crossing(host, automaton.as_ref()),
            AmbienceMessage::Occupant { msg_type, occupant } => {
                if let Some(o) = occupant {
                    match *msg_type {
                        MSG_INSERT_OCCUPANT => self.handle_occupant_common(host, o, true),
                        MSG_MOVE_OCCUPANT => self.handle_move(host, o),
                        MSG_REMOVE_OCCUPANT => self.handle_remove(host, o),
                        // StateChanged: no case *[quirk]*.
                        _ => {}
                    }
                }
                true
            }
            AmbienceMessage::ConditionChanged { lot, values } => {
                // old := data1, new := data2 (§3.4); `lot` None = void1 does
                // not answer the lot IID.
                if lot.is_some() {
                    self.update_lot_condition(host, *lot, values.0 as i32, values.1 as i32);
                }
                true
            }
            AmbienceMessage::Other(_) => true,
        }
    }
}

/// Time of day (§3.6): `float(hour) + float(minute / 60)` with an unsigned
/// integer division, so the fraction is 0 for minutes 0-59 *[quirk]*.
pub fn time_of_day(hour: u32, minute: u32) -> f32 {
    hour as f32 + (minute / 60) as f32
}

#[cfg(test)]
mod tests;
