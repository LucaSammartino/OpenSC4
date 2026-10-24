// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Foreign-service boundary for the measured owner behavior of audio.md §7.
//! Core owns dispatch, lifecycle, modal state, selection and torch arithmetic.
//! Foreign adapters supply interface calls and acquired-reference IO only.

use super::{city_message_types, init_message_types, AudioEventHandler, SndSysEvent};
use crate::audio::message::StandardMessage;
use crate::audio::{
    EVT_SET_POSITION, EVT_START_SOUND, EVT_STOP_SOUND, MSG_AUTOMATA_A, MSG_AUTOMATA_B, MSG_AUTOMATA_PLAY_SFX,
    MSG_AUTOMATA_STOP_SFX, MSG_POST_CITY_INIT, MSG_PRE_CITY_SHUTDOWN, RATE_LIMIT_PAYLOAD,
};

#[cfg(test)]
#[path = "owner_tests.rs"]
mod tests;

pub const MODAL_MESSAGE_TYPES: [u32; 2] = [0x62BA_7B42, 0x62BA_7B43];
pub const PAYLOAD_IID: u32 = 0x6529_7976;
pub const POSITION_IID: u32 = 0xA9B4_0F05;
/// The director registers the handler under this service IID (§1.3).
pub const HANDLER_SERVICE_IID: u32 = 0xBB52_3BB2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageTarget {
    Modal,
    Main,
}

/// Service IDs and IIDs are literal pairs from §7.1–7.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandlerService(pub u32, pub u32);

pub const RADIO_SERVICE: HandlerService = HandlerService(0x1D07_B39F, 0xE051_0260);
pub const SOUND_SERVICE: HandlerService = HandlerService(0x23CE_CBA0, 0x1060_77D2);
pub const INIT_SERVICES: [HandlerService; 4] = [
    SOUND_SERVICE,
    HandlerService(0x49E5_0BA1, 0x29E3_9DB4),
    HandlerService(0x2073_215D, 0xE020_3660),
    HandlerService(0x04FA_845B, 0x6522_94C7),
];

/// Operations on foreign services. Acquired COM references stay owned by the
/// adapter; `release_capability` releases only its owned +0x18 reference.
/// Positions cross this boundary as vectors: the spec gives no wire packing.
pub trait EventHandlerHost {
    fn get_service(&mut self, service: HandlerService) -> bool;
    fn capability_present(&self) -> bool;
    fn register_sound_handler(&mut self, add: bool);
    fn subscribe(&mut self, target: MessageTarget, message: u32, add: bool);
    fn release_capability(&mut self);
    fn stop_radio(&mut self);
    fn radio_slot31(&mut self);
    fn radio_slot7(&mut self, value: u64);
    fn radio_slot30(&mut self, value: bool);
    fn post_message(&mut self, message: StandardMessage, high_priority: bool);
    fn post_sound(&mut self, event: SndSysEvent);
    fn post_position(&mut self, event: u32, instance: u32, vector: [f32; 3]);
    fn vector_seed(&self) -> [f32; 3];
    /// A CRT rand() value in [0, 0x7FFF]. Called only on a limiter restart.
    fn random(&mut self) -> u32;
    /// Release a temporary service lookup, not a borrowed service pointer.
    fn release_service(&mut self, service: HandlerService);
    fn retain_capability(&mut self);
    fn release_radio(&mut self);
    /// Filesystem slot 7 plus the runtime path fragment are foreign string IO.
    fn radio_path(&mut self) -> String;
    fn radio_slot29(&mut self, path: &str);
    fn radio_slot23(&mut self, enabled: bool);
    fn radio_slot16(&mut self, volume: u32);
    /// Load the application preference object; conversion/list IO stays foreign.
    fn load_preferences(&mut self) -> AudioPreferences;
    fn set_default_sliders(&mut self, music: u32, sfx: u32);
    fn reset_music_lists(&mut self);
    fn update_exclusion_lists(&mut self);
    /// Individual foreign interface calls. Branching and state stay in core.
    fn control(&mut self, call: HandlerCall) -> HandlerValue;
    /// Sound-system slot 32(callback, 0, 0).
    fn enumerate_sounds(&mut self, callback: fn(&mut dyn HandlerSoundObject) -> bool);
}

pub trait HandlerSoundObject {
    fn query(&mut self, iid: u32) -> bool;
    fn read(&mut self, key: u32, default: u32) -> u32;
    fn write(&mut self, key: u32, value: u32);
    fn release(&mut self);
}

pub fn enumerate_callback(object: &mut dyn HandlerSoundObject) -> bool {
    if object.query(0xFD58_4467) {
        let value = object.read(0x20, 0);
        object.write(0x20, value);
        object.release();
    }
    true
}

#[derive(Clone, Debug, PartialEq)]
pub enum HandlerCall {
    ReleaseTransferred(u64),
    RadioCurrent,
    RadioResolveCurrent(String),
    RadioResolveRequested(Option<&'static str>),
    RadioSelect(&'static str, [u32; 2]),
    StationVolume(u64, u32),
    StationMode(u64, u32),
    RadioEnabled,
    RadioStart(u32),
    PreferenceGate,
    SoundRequest(u32, [u32; 4]),
    Scape(u32, u32, u64, Option<[f32; 3]>, f32),
    CreateEvent(u32, u32, [u32; 4]),
    SubmitEvent(u64),
    ReleaseEvent(u64),
    RetainObject(u64),
    QueryObject(u64, u32),
    ReleaseObject(u64),
    SoundManagerAccess,
    GenerateInstance,
    AcquireModalMessageService,
    ReleaseModalMessageService,
    GlobalWindow,
    WindowMessageServer(u64),
    WindowSubscription(u64, u32, bool),
    WindowRectangle(u64),
    AcquireGlobalSound,
    GlobalSoundRequest(u64, [u32; 5]),
    GlobalSoundPresent,
    ObtainSoundInterface,
    SoundInterfaceValue(u64, u32, u32),
}

/// A single slot's return, not an owner-algorithm result.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HandlerValue {
    pub word: u64,
    pub text: String,
    pub rectangle: [i32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioPreferences {
    pub first_or_default: bool,
    pub music_enabled: bool,
    pub music_volume: u32,
}

/// Foreign message/payload slots used by §7.2–7.3. A successful QI acquires
/// exactly one reference; the owner balances it via `release_payload`.
pub trait EventHandlerMessage {
    fn message_type(&self) -> u32;
    fn query_payload(&mut self, iid: u32) -> bool;
    fn release_payload(&mut self);
    fn slot9(&mut self) -> u32;
    fn slot10(&mut self) -> u64;
    fn release_message(&mut self);
    fn slot14(&mut self, arguments: &[u32]) -> u32;
    /// Borrowed payload slot32 then QI. POSITION_IID uses slot7; 0x26BB767E
    /// uses slot8; 0xA821EF94 uses slot48's object then slot8. A successful
    /// QI stays acquired until release_position; failed returned refs are
    /// balanced by the adapter. No guessed position on failure.
    fn position(&mut self, iid: u32, seed: [f32; 3]) -> Option<[f32; 3]>;
    fn original_position(&mut self, iid: u32) -> Option<[f32; 3]>;
    fn release_position(&mut self);
    fn payload_type(&mut self) -> u32;
    fn has_data(&mut self, index: u32) -> bool;
    /// Standard payload fields (1-based); getters take no arguments.
    fn data(&mut self, index: u32) -> u32;
    fn void1(&mut self, original_message: bool) -> u64;
    fn mouse_subtype(&mut self) -> u32;
    fn mouse_point(&mut self) -> [i32; 2];
}

/// Every route explicitly assigned by §7.2; subscribed types absent from that
/// table fall through to `Ignore`, just like an unknown directly posted type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageRoute {
    Construction,
    RadioSlot31,
    CityLifecycle,
    BuildMessage,
    SetStationOne,
    Radio,
    Automata,
    RadioSlot7,
    RadioSlot30,
    Lab,
    ModalMouse,
    RadioSettings,
    Dispatch,
    Ignore,
}

pub fn message_route(message: u32) -> MessageRoute {
    match message {
        0x89F1_0866 | 0xA9F1_0E45 => MessageRoute::Construction,
        0x2A61_1B51 => MessageRoute::RadioSlot31,
        MSG_POST_CITY_INIT | MSG_PRE_CITY_SHUTDOWN => MessageRoute::CityLifecycle,
        0x26D3_1EC4 => MessageRoute::BuildMessage,
        0x26D3_1EC5 => MessageRoute::SetStationOne,
        0x2A3A_D653 => MessageRoute::Radio,
        MSG_AUTOMATA_PLAY_SFX | MSG_AUTOMATA_STOP_SFX | MSG_AUTOMATA_A | MSG_AUTOMATA_B => MessageRoute::Automata,
        0xAA61_1B6E => MessageRoute::RadioSlot7,
        0xAA65_0861 => MessageRoute::RadioSlot30,
        0xC9F1_011A | 0xE946_71EB => MessageRoute::Lab,
        0x49F1_0111 | 0x8946_71F6 => MessageRoute::Dispatch,
        // AU45: completed §7.2 explicitly includes this accepted inner no-op,
        // although the older §16.12 fixture lists only four operation types.
        0x49E6_4801 => MessageRoute::Automata,
        0x0A61_1B34 | 0xAA65_0860 | 0x4348_B111 | 0x4A7F_B7E9 => MessageRoute::RadioSettings,
        0xFA0C_6390 => MessageRoute::ModalMouse,
        _ => MessageRoute::Ignore,
    }
}

/// Legacy diagnostic vocabulary. Extended §7 algorithms no longer return the
/// resolved AU13–AU16 algorithm-gap variants; service failure remains checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandlerGap {
    MessageAlgorithm(MessageRoute), // AU13
    ModalNotice,                    // AU13
    ServiceFailure,                 // AU14
    CapabilityAutomata,             // AU15
    DirectAutomata,                 // AU15
    EventCallback,                  // AU16
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispatchOutcome {
    pub returned: u32,
    pub gap: Option<HandlerGap>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TorchState {
    pub mode: bool,
    pub movement_volume: bool,
    pub speed: f32,
    pub x: i32,
    pub y: i32,
    pub timestamp: u32,
}

static TORCH_STATE: std::sync::Mutex<TorchState> = std::sync::Mutex::new(TorchState {
    mode: false,
    movement_volume: false,
    speed: 0.0,
    x: 0,
    y: 0,
    timestamp: 0,
});

fn torch_start(host: &mut impl EventHandlerHost, sound: u32, key: u32, volume: u32) {
    host.post_sound(SndSysEvent {
        event: EVT_START_SOUND,
        a: sound,
        b: key,
        c: 0,
        d: 0,
    });
    torch_volume(host, key, volume);
}

fn torch_volume(host: &mut impl EventHandlerHost, key: u32, volume: u32) {
    host.post_sound(SndSysEvent {
        event: 0x4A45_1915,
        a: key,
        b: volume,
        c: 0,
        d: 0,
    });
}

fn torch_stop(host: &mut impl EventHandlerHost, key: u32) {
    host.post_sound(SndSysEvent {
        event: EVT_STOP_SOUND,
        a: key,
        b: 0,
        c: 0,
        d: 0,
    });
}

impl AudioEventHandler {
    /// Independent constructor lookups; +0x18 is initially null (§7.1.1).
    pub fn construct(host: &mut impl EventHandlerHost) -> Self {
        let mut handler = Self::new();
        handler.radio_present = host.get_service(RADIO_SERVICE);
        handler.snd_sys_present = host.get_service(SOUND_SERVICE);
        handler.capability_present = false;
        handler
    }

    /// §7.1.1: registration survives a failed AudioScape lookup; optional
    /// filesystem failure skips only path setup. Required servers are asserted.
    pub fn init_with(&mut self, host: &mut impl EventHandlerHost) -> Result<u32, HandlerGap> {
        if self.initialized {
            return Ok(1);
        }
        if !host.get_service(SOUND_SERVICE) {
            return Err(HandlerGap::ServiceFailure);
        }
        host.register_sound_handler(true);
        self.sound_registered = true;
        if !host.get_service(INIT_SERVICES[1]) {
            host.release_service(SOUND_SERVICE);
            return Err(HandlerGap::ServiceFailure);
        }
        host.retain_capability();
        if self.capability_present {
            host.release_capability();
        }
        self.capability_present = true;
        if host.get_service(INIT_SERVICES[2]) {
            if self.radio_present {
                let path = host.radio_path();
                host.radio_slot29(&path);
                host.radio_slot23(false);
                host.radio_slot16(0);
            }
            host.release_service(INIT_SERVICES[2]);
        }
        for message in MODAL_MESSAGE_TYPES {
            host.subscribe(MessageTarget::Modal, message, true);
        }
        assert!(host.get_service(INIT_SERVICES[3]), "required message server absent");
        for message in init_message_types() {
            host.subscribe(MessageTarget::Main, message, true);
        }
        let preferences = host.load_preferences();
        if preferences.first_or_default {
            host.set_default_sliders(0x2D0, 0x400);
            host.reset_music_lists();
            host.update_exclusion_lists();
        }
        if self.radio_present {
            host.radio_slot30(true);
            host.radio_slot23(preferences.music_enabled);
            host.radio_slot16(preferences.music_volume);
        }
        host.release_service(INIT_SERVICES[3]);
        host.release_service(INIT_SERVICES[1]);
        host.release_service(SOUND_SERVICE);
        Ok(self.init())
    }

    /// Shutdown preserves city subscriptions; only PreCityShutdown removes them.
    pub fn shutdown_with(&mut self, host: &mut impl EventHandlerHost) -> bool {
        if !self.initialized {
            return true;
        }
        if !host.get_service(SOUND_SERVICE) {
            return false;
        }
        host.register_sound_handler(false);
        self.sound_registered = false;
        if self.capability_present {
            host.release_capability();
            self.capability_present = false;
        }
        if self.radio_present {
            host.radio_slot23(false);
            host.release_radio();
            self.radio_present = false;
        }
        assert!(host.get_service(INIT_SERVICES[3]), "required message server absent");
        for message in init_message_types() {
            host.subscribe(MessageTarget::Main, message, false);
            self.subscribed.remove(&message);
        }
        for message in MODAL_MESSAGE_TYPES {
            host.subscribe(MessageTarget::Modal, message, false);
            self.modal_subscribed.remove(&message);
        }
        self.initialized = false;
        host.release_service(INIT_SERVICES[3]);
        host.release_service(SOUND_SERVICE);
        true
    }

    /// Message server delivery honors main-target subscriptions. Direct posts
    /// should use `do_message`, which does not impose a city-loaded guard.
    pub fn deliver_main(
        &mut self,
        message: &mut dyn EventHandlerMessage,
        host: &mut impl EventHandlerHost,
    ) -> Option<DispatchOutcome> {
        self.subscribed
            .contains(&message.message_type())
            .then(|| self.do_message(Some(message), host))
    }

    /// Null alone returns 0; every non-null message returns 1, even when a
    /// sub-handler rejects its payload (§7.2).
    pub fn do_message(
        &mut self,
        message: Option<&mut dyn EventHandlerMessage>,
        host: &mut impl EventHandlerHost,
    ) -> DispatchOutcome {
        let Some(message) = message else {
            return DispatchOutcome { returned: 0, gap: None };
        };
        let route = message_route(message.message_type());
        let gap = match route {
            MessageRoute::CityLifecycle => {
                let add = message.message_type() == MSG_POST_CITY_INIT;
                for ty in city_message_types() {
                    host.subscribe(MessageTarget::Main, ty, add);
                }
                self.handle_city_init_shutdown(message.message_type());
                None
            }
            MessageRoute::SetStationOne | MessageRoute::BuildMessage => {
                let mut posted = StandardMessage::new();
                posted.set_type(0x2A3A_D653);
                posted.set_data1(u32::from(route == MessageRoute::SetStationOne));
                host.post_message(posted, false);
                None
            }
            MessageRoute::RadioSlot31 | MessageRoute::RadioSlot7 | MessageRoute::RadioSlot30 => {
                match route {
                    MessageRoute::RadioSlot31 if self.radio_present => host.radio_slot31(),
                    MessageRoute::RadioSlot7 => {
                        let value = message.slot10();
                        if value != 0 {
                            if self.radio_present {
                                host.radio_slot7(value);
                            }
                            host.control(HandlerCall::ReleaseTransferred(value));
                        }
                    }
                    MessageRoute::RadioSlot30 => {
                        let value = message.slot9() != 0;
                        if self.radio_present {
                            host.radio_slot30(value);
                        }
                    }
                    _ => {}
                }
                None
            }
            MessageRoute::Automata => self.handle_automata(message, host).err(),
            MessageRoute::Radio => {
                self.handle_radio(message, host);
                None
            }
            MessageRoute::RadioSettings => {
                self.handle_settings(message, host);
                None
            }
            MessageRoute::Construction => {
                self.handle_construction(message, host);
                None
            }
            MessageRoute::Lab | MessageRoute::Dispatch => {
                self.handle_dispatch(message, host);
                None
            }
            MessageRoute::ModalMouse => {
                return DispatchOutcome {
                    returned: u32::from(self.handle_modal_mouse(message, host)),
                    gap: None,
                };
            }
            MessageRoute::Ignore => None,
        };
        DispatchOutcome { returned: 1, gap }
    }

    /// §7.2.1 station selection. The payload type is checked after QI.
    pub fn handle_radio(&mut self, message: &mut dyn EventHandlerMessage, host: &mut impl EventHandlerHost) -> bool {
        if !message.query_payload(PAYLOAD_IID) {
            return false;
        }
        let accepted = message.payload_type() == 0x2A3A_D653;
        if accepted {
            let station = match message.data(1) {
                0 => Some("kmay"),
                1 => Some("krgn"),
                _ => None,
            };
            if let Some(station) = station {
                let duration = if message.has_data(2) { message.data(2) } else { 4000 };
                if self.radio_present {
                    let current = host.control(HandlerCall::RadioCurrent).text;
                    host.radio_slot30(false);
                    let current = host.control(HandlerCall::RadioResolveCurrent(current)).word;
                    let requested = host.control(HandlerCall::RadioResolveRequested(Some(station))).word;
                    if current == 0 || current != requested {
                        host.control(HandlerCall::RadioSelect(station, [0, duration]));
                    }
                }
            }
        }
        message.release_payload();
        accepted
    }

    fn handle_settings(&mut self, message: &mut dyn EventHandlerMessage, host: &mut impl EventHandlerHost) {
        match message.message_type() {
            0x0A61_1B34 => {
                let station = match message.data(1) {
                    0 => Some("kmay"),
                    1 => Some("krgn"),
                    _ => None,
                };
                let volume = message.data(2);
                let mode = if message.has_data(3) { message.data(3) } else { 0 };
                if self.radio_present {
                    let object = host.control(HandlerCall::RadioResolveRequested(station)).word;
                    if object != 0 {
                        host.control(HandlerCall::StationVolume(object, volume));
                        host.control(HandlerCall::StationMode(object, if mode == 0 { 2 } else { 1 }));
                    }
                }
            }
            0xAA65_0860 if self.radio_present => {
                if message.has_data(2) {
                    host.radio_slot16(message.data(2));
                }
                if message.has_data(1) {
                    let enabled = message.data(1) == 1;
                    if (host.control(HandlerCall::RadioEnabled).word != 0) != enabled {
                        host.radio_slot23(enabled);
                        if enabled {
                            host.control(HandlerCall::RadioStart(0));
                        }
                    }
                }
            }
            0x4348_B111 => {
                if host.control(HandlerCall::PreferenceGate).word != 0 && message.query_payload(PAYLOAD_IID) {
                    if self.radio_present {
                        host.radio_slot23(message.data(1) != 0);
                    }
                    message.release_payload();
                }
            }
            0x4A7F_B7E9 => {
                host.control(HandlerCall::SoundRequest(0x0BAA_B7A4, [message.data(1), 0, 0, 0]));
            }
            _ => {}
        }
    }

    /// §7.2.2; LAB and dispatch table entries share this one algorithm.
    pub fn handle_dispatch(&mut self, message: &mut dyn EventHandlerMessage, host: &mut impl EventHandlerHost) -> bool {
        if !message.query_payload(PAYLOAD_IID) {
            return false;
        }
        let accepted = if !self.snd_sys_present || !message.has_data(1) {
            false
        } else {
            let category = message.data(1);
            let key = message.void1(false);
            match message.message_type() {
                0x8946_71F6 | 0xE946_71EB => {
                    if matches!(category, 0 | 1 | 5 | 6) && key != 0 {
                        let sound = if message.message_type() == 0x8946_71F6 {
                            0x09C6_6951
                        } else {
                            0x29D9_9E9E
                        };
                        let event = host
                            .control(HandlerCall::CreateEvent(
                                EVT_START_SOUND,
                                0x5B52_3BA4,
                                [sound, key as u32, 0, 0],
                            ))
                            .word;
                        host.control(HandlerCall::SubmitEvent(event));
                        host.control(HandlerCall::ReleaseEvent(event));
                    }
                    true
                }
                0xC9F1_011A | 0x49F1_0111 if key != 0 => {
                    host.control(HandlerCall::RetainObject(key));
                    let interface = host.control(HandlerCall::QueryObject(key, 0xC9B6_D7D6)).word;
                    if interface != 0 {
                        host.control(HandlerCall::ReleaseObject(interface));
                    }
                    host.control(HandlerCall::ReleaseObject(key));
                    interface != 0
                }
                _ => false,
            }
        };
        message.release_payload();
        accepted
    }

    /// §7.3.2 construction selection and the slot-48/slot-8 position IO.
    pub fn handle_construction(
        &mut self,
        message: &mut dyn EventHandlerMessage,
        host: &mut impl EventHandlerHost,
    ) -> bool {
        if !self.capability_present || !message.query_payload(PAYLOAD_IID) {
            return false;
        }
        let key = message.data(1) as u64;
        let sound = if message.data(3) & 2 != 0 {
            0xEA62_C800
        } else {
            0xAA62_C82A
        };
        let alternate = message.message_type() == 0xA9F1_0E45;
        let accepted = if alternate && message.void1(false) == 0 {
            false
        } else {
            if !alternate && (!message.has_data(2) || message.data(2) == 0) {
                host.control(HandlerCall::Scape(14, sound, key, None, 0.0));
            } else if let Some(position) = message.position(0xA821_EF94, [0.0; 3]) {
                host.control(HandlerCall::Scape(
                    if alternate { 15 } else { 13 },
                    sound,
                    key,
                    Some(position),
                    0.0,
                ));
                message.release_position();
            }
            true
        };
        message.release_payload();
        accepted
    }

    fn alternate_automata(&mut self, message: &mut dyn EventHandlerMessage, host: &mut impl EventHandlerHost) -> u32 {
        let key = message.void1(false);
        if key == 0 || !message.has_data(1) || !message.has_data(2) {
            return 0;
        }
        let selector = message.data(1) as i32;
        if selector == 0 {
            host.control(HandlerCall::Scape(14, 0x49DE_8A94, message.void1(false), None, 0.0));
            host.control(HandlerCall::Scape(14, 0xA9DE_8B59, message.void1(false), None, 0.0));
        } else {
            let Some(position) = message.position(0x26BB_767E, [0.0; 3]) else {
                return 0;
            };
            let sound = if selector > 0 { 0x49DE_8A94 } else { 0xA9DE_8B59 };
            host.control(HandlerCall::Scape(13, sound, key, Some(position), 50.0));
            message.release_position();
        }
        1
    }

    /// §7.2.3: nested windows use a LIFO stack; ends ignore their window word.
    pub fn do_modal_message(&mut self, notice: u32, window: u64, host: &mut impl EventHandlerHost) -> bool {
        if !MODAL_MESSAGE_TYPES.contains(&notice) {
            return true;
        }
        host.control(HandlerCall::AcquireModalMessageService);
        let was_empty = self.modal_windows.is_empty();
        if notice == MODAL_MESSAGE_TYPES[0] {
            host.control(HandlerCall::RetainObject(window));
            self.modal_windows.insert(0, window);
        } else {
            assert!(!was_empty, "modal end requires a matching begin");
            let window = self.modal_windows.remove(0);
            host.control(HandlerCall::ReleaseObject(window));
        }
        let is_empty = self.modal_windows.is_empty();
        if was_empty != is_empty {
            let global_window = host.control(HandlerCall::GlobalWindow).word;
            if global_window != 0 {
                host.control(HandlerCall::RetainObject(global_window));
                let interface = host.control(HandlerCall::QueryObject(global_window, 0x3AC7_A210)).word;
                if interface != 0 {
                    let server = host.control(HandlerCall::WindowMessageServer(interface)).word;
                    if server != 0 {
                        host.control(HandlerCall::WindowSubscription(server, 0xFA0C_6390, !is_empty));
                    }
                    host.control(HandlerCall::ReleaseObject(interface));
                }
                host.control(HandlerCall::ReleaseObject(global_window));
            }
        }
        host.control(HandlerCall::ReleaseModalMessageService);
        true
    }

    /// §7.2.4: inclusive signed bounds; this route alone forwards false.
    pub fn handle_modal_mouse(
        &mut self,
        message: &mut dyn EventHandlerMessage,
        host: &mut impl EventHandlerHost,
    ) -> bool {
        let window = *self.modal_windows.first().expect("modal mouse requires a window");
        host.control(HandlerCall::RetainObject(window));
        let mut accepted = true;
        if message.query_payload(0x1A0C_615F) {
            if matches!(message.mouse_subtype(), 7..=9) {
                let [left, top, right, bottom] = host.control(HandlerCall::WindowRectangle(window)).rectangle;
                let [x, y] = message.mouse_point();
                if x < left || x > right || y < top || y > bottom {
                    let sound = host.control(HandlerCall::AcquireGlobalSound).word;
                    if sound == 0 {
                        accepted = false;
                    } else {
                        host.control(HandlerCall::GlobalSoundRequest(sound, [1, 0xAA62_C9D5, 0, 0, 0]));
                        host.control(HandlerCall::ReleaseObject(sound));
                    }
                }
            }
            message.release_payload();
        }
        host.control(HandlerCall::ReleaseObject(window));
        accepted
    }

    /// §7.6 intentionally leaks retained modal-window references. No Shutdown
    /// side effects occur here. Rust owns the list/timer storage itself.
    pub fn destroy_with(&mut self, flags: u32, host: &mut impl EventHandlerHost) -> bool {
        self.modal_windows.clear();
        if self.capability_present {
            host.release_capability();
            self.capability_present = false;
        }
        if self.snd_sys_present {
            host.release_service(SOUND_SERVICE);
            self.snd_sys_present = false;
        }
        if self.radio_present {
            host.release_radio();
            self.radio_present = false;
        }
        flags & 1 != 0
    }

    /// §7.5. `message_type` is original event slot 4; `data` reads the
    /// original slot-6 payload, independently of the unused temporary QI.
    pub fn do_event(&mut self, event: &mut dyn EventHandlerMessage, host: &mut impl EventHandlerHost) -> bool {
        let mut state = TORCH_STATE.lock().unwrap_or_else(|poison| poison.into_inner());
        self.do_event_with_torch(event, host, &mut state)
    }

    /// Explicit process-state argument permits deterministic replay; callers
    /// must share this state across handlers, never keep one state per handler.
    pub fn do_event_with_torch(
        &mut self,
        event: &mut dyn EventHandlerMessage,
        host: &mut impl EventHandlerHost,
        state: &mut TorchState,
    ) -> bool {
        if !self.snd_sys_present {
            return true;
        }
        let service = HandlerService(0x1AF8_5BFB, 0x3AF8_5BB7);
        let acquired = host.get_service(service);
        let queried = event.query_payload(PAYLOAD_IID);
        match event.message_type() {
            0x2A10_6113 | 0x8A10_611A | 0x0A10_6122 | 0x4A10_6126 | 0x8A10_611D | 0xAA96_FE8F => {
                self.handle_torch(event, host, state);
            }
            0xAA36_D9B6 => host.enumerate_sounds(enumerate_callback),
            0x0BAA_B7A4 => {
                let value = event.data(1);
                let interface = host.control(HandlerCall::ObtainSoundInterface).word;
                assert_ne!(interface, 0, "required sound interface absent");
                host.control(HandlerCall::SoundInterfaceValue(interface, 0x7F, value));
                host.control(HandlerCall::ReleaseObject(interface));
            }
            _ => {}
        }
        if queried {
            event.release_payload();
        }
        if acquired {
            host.release_service(service);
        }
        false
    }

    pub fn handle_torch(
        &mut self,
        event: &mut dyn EventHandlerMessage,
        host: &mut impl EventHandlerHost,
        state: &mut TorchState,
    ) -> bool {
        if !self.capability_present || host.control(HandlerCall::GlobalSoundPresent).word == 0 {
            return false;
        }
        let x = event.data(1) as i32;
        let y = event.data(2) as i32;
        let time = event.data(3);
        let elapsed = time.wrapping_sub(state.timestamp);
        // §7.5: x rounds to binary32 before squaring; previous y rounds
        // before subtraction. f64 preserves these x87 intermediates here.
        let dx = (x as f64 - state.x as f64) as f32 as f64;
        let dy = y as f64 - state.y as f32 as f64;
        let speed = if elapsed == 0 {
            0.0
        } else {
            (((dx * dx + dy * dy).sqrt() / (elapsed as f64 * 0.01_f32 as f64)) as f32).min(60.0)
        };
        let smoothed = (state.speed as f64 + speed as f64) * 0.5;
        let volume = (smoothed * 424.0 * f32::from_bits(0x3C88_8889) as f64).trunc() as u32;
        match event.message_type() {
            0x2A10_6113 => torch_start(host, 0x6A10_AC51, 0x601, 1024),
            0x8A10_611A => {
                for key in [0x601, 0x602, 0x603] {
                    torch_stop(host, key);
                }
            }
            0x0A10_6122 => state.movement_volume = false,
            0x8A10_611D => state.movement_volume = true,
            0x4A10_6126 if state.movement_volume => {
                torch_volume(host, 0x602, volume + 600);
                torch_volume(host, 0x603, volume + 600);
            }
            0xAA96_FE8F if x != 0 && !state.mode => {
                state.mode = true;
                torch_start(host, 0x6A10_ABCF, 0x600, 1024);
                torch_start(host, 0x0A10_AD41, 0x602, volume + 600);
                torch_start(host, 0x2AA7_3C50, 0x603, 1024 - volume);
                torch_stop(host, 0x601);
            }
            0xAA96_FE8F if x == 0 && state.mode => {
                state.mode = false;
                torch_stop(host, 0x602);
                torch_stop(host, 0x603);
                torch_start(host, 0x6A10_AC51, 0x601, 1024);
            }
            _ => {}
        }
        state.speed = smoothed as f32;
        state.x = x;
        state.y = y;
        state.timestamp = time;
        false
    }

    /// Full foreign-payload entry to §7.3, unlike the legacy word-only fixture
    /// helpers. Sound events go to the sound system, never to a simulator.
    pub fn handle_automata(
        &mut self,
        message: &mut dyn EventHandlerMessage,
        host: &mut impl EventHandlerHost,
    ) -> Result<u32, HandlerGap> {
        // AU14: literal non-null/QI preconditions reject without side effects.
        if !self.capability_present || !message.query_payload(PAYLOAD_IID) {
            return Ok(0);
        }
        let result = if !self.snd_sys_present {
            Ok(0)
        } else {
            self.handle_acquired_payload(message, host)
        };
        message.release_payload();
        result
    }

    fn handle_acquired_payload(
        &mut self,
        message: &mut dyn EventHandlerMessage,
        host: &mut impl EventHandlerHost,
    ) -> Result<u32, HandlerGap> {
        match message.message_type() {
            MSG_AUTOMATA_PLAY_SFX => {
                let sound = message.slot9();
                if sound == RATE_LIMIT_PAYLOAD {
                    if self.limiter.running && self.limiter.elapsed_ms < self.limiter.window_ms {
                        return Ok(1);
                    }
                    self.limiter.gate(host.random());
                }
                let seed = host.vector_seed();
                assert_ne!(message.void1(false), 0, "play requires void1");
                let Some(vector) = message.position(POSITION_IID, seed) else {
                    return Ok(0);
                };
                let instance = message.slot14(&[]);
                host.post_sound(SndSysEvent {
                    event: EVT_START_SOUND,
                    a: sound,
                    b: instance,
                    c: 1,
                    d: 0,
                });
                let instance = message.slot14(&[]);
                host.post_sound(SndSysEvent {
                    event: EVT_SET_POSITION,
                    a: instance,
                    b: (vector[0] as i32) as u32,
                    c: (vector[1] as i32) as u32,
                    d: (vector[2] as i32) as u32,
                });
                message.release_position();
                Ok(1)
            }
            MSG_AUTOMATA_STOP_SFX => {
                let instance = message.slot14(&[]);
                host.post_sound(SndSysEvent {
                    event: EVT_STOP_SOUND,
                    a: instance,
                    b: 0,
                    c: 0,
                    d: 0,
                });
                Ok(1)
            }
            MSG_AUTOMATA_A => Ok(self.alternate_automata(message, host)),
            MSG_AUTOMATA_B => {
                assert_ne!(message.void1(true), 0, "required original void1 missing");
                // §7.3.1 explicitly leaves position indeterminate on failed QI.
                let position = message.original_position(0x26BB_767E);
                if position.is_some() {
                    message.release_position();
                }
                let first = if message.has_data(1) { message.data(1) } else { u32::MAX };
                let second = if message.has_data(2) { message.data(2) } else { u32::MAX };
                let sound = match (second, first != 0) {
                    (1, false) => 0x4A5E_7FFC,
                    (1, true) => 0x0A5E_802C,
                    (2, false) => 0x2A5E_7F2C,
                    (2, true) => 0x0A5E_7F51,
                    (3, false) => 0x2A5E_8067,
                    (3, true) => 0x6A5E_8088,
                    _ => 0x69F0_1280,
                };
                host.control(HandlerCall::SoundManagerAccess);
                let id = host.control(HandlerCall::GenerateInstance).word;
                host.control(HandlerCall::Scape(13, sound, id, position, 50.0));
                Ok(1)
            }
            _ => Ok(1),
        }
    }
}
