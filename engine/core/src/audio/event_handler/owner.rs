// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Foreign-service boundary for the measured owner behavior of audio.md §7.
//! Unknown owner algorithms are reported as gaps, never delegated to a host
//! pretending to implement them. See the module README's owner closure table.

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
    /// slot32(), then QI POSITION_IID. The adapter balances any acquired
    /// occupant/vector references. None retains the supplied global seed (AU15).
    fn position(&mut self, iid: u32, seed: [f32; 3]) -> Option<[f32; 3]>;
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
        0xFA0C_6390 => MessageRoute::ModalMouse,
        _ => MessageRoute::Ignore,
    }
}

/// These are spec gaps, not claims of owner closure (audio questions AU13–AU16).
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

impl AudioEventHandler {
    /// Constructor service requests (§7.1). AU14: +0x18 acquisition and any
    /// missing-service fallback are unspecified, so presence comes from host.
    pub fn construct(host: &mut impl EventHandlerHost) -> Self {
        let mut handler = Self::new();
        handler.radio_present = host.get_service(RADIO_SERVICE);
        handler.snd_sys_present = host.get_service(SOUND_SERVICE);
        handler.capability_present = host.capability_present();
        handler
    }

    /// Successful Init path (§7.2). AU14: a failed service request is surfaced
    /// before registration; rollback/return codes are not specified.
    pub fn init_with(&mut self, host: &mut impl EventHandlerHost) -> Result<u32, HandlerGap> {
        let mut available = true;
        for service in INIT_SERVICES {
            let present = host.get_service(service);
            if service == SOUND_SERVICE {
                self.snd_sys_present = present;
            }
            available &= present;
        }
        self.capability_present = host.capability_present();
        if !available {
            return Err(HandlerGap::ServiceFailure);
        }
        host.register_sound_handler(true);
        self.sound_registered = true;
        for message in MODAL_MESSAGE_TYPES {
            host.subscribe(MessageTarget::Modal, message, true);
        }
        for message in init_message_types() {
            host.subscribe(MessageTarget::Main, message, true);
        }
        Ok(self.init())
    }

    /// Shutdown preserves city subscriptions; only PreCityShutdown removes them.
    pub fn shutdown_with(&mut self, host: &mut impl EventHandlerHost) {
        if self.sound_registered {
            host.register_sound_handler(false);
            self.sound_registered = false;
        }
        if self.capability_present {
            host.release_capability();
            self.capability_present = false;
        }
        if self.radio_present {
            host.stop_radio();
        }
        for message in init_message_types() {
            host.subscribe(MessageTarget::Main, message, false);
            self.subscribed.remove(&message);
        }
        for message in MODAL_MESSAGE_TYPES {
            host.subscribe(MessageTarget::Modal, message, false);
            self.modal_subscribed.remove(&message);
        }
        self.initialized = false;
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
            MessageRoute::SetStationOne => {
                let mut posted = StandardMessage::new();
                posted.set_type(0x2A3A_D653);
                posted.set_data1(1);
                host.post_message(posted, false);
                None
            }
            MessageRoute::RadioSlot31 | MessageRoute::RadioSlot7 | MessageRoute::RadioSlot30 => {
                if !self.radio_present {
                    Some(HandlerGap::ServiceFailure) // AU14: null-radio behavior absent.
                } else {
                    match route {
                        MessageRoute::RadioSlot31 => host.radio_slot31(),
                        MessageRoute::RadioSlot7 => {
                            let value = message.slot10();
                            host.radio_slot7(value);
                            message.release_message();
                        }
                        MessageRoute::RadioSlot30 => {
                            let value = message.slot9() != 0;
                            host.radio_slot30(value);
                        }
                        _ => unreachable!(),
                    }
                    None
                }
            }
            MessageRoute::Automata => self.handle_automata(message, host).err(),
            MessageRoute::Ignore => None,
            _ => Some(HandlerGap::MessageAlgorithm(route)), // AU13: no algorithm given.
        };
        DispatchOutcome { returned: 1, gap }
    }

    /// The separate modal target's algorithm is absent from §7 (AU13).
    pub fn do_modal_message(&mut self) -> Result<(), HandlerGap> {
        Err(HandlerGap::ModalNotice)
    }

    /// §7 lists this callback's address but gives no event table (AU16).
    pub fn do_event(&mut self) -> Result<(), HandlerGap> {
        Err(HandlerGap::EventCallback)
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
                let vector = message.position(POSITION_IID, seed).unwrap_or(seed);
                let instance = message.slot14(&[1, 0]);
                host.post_sound(SndSysEvent {
                    event: EVT_START_SOUND,
                    a: sound,
                    b: instance,
                    c: 0,
                    d: 0,
                });
                host.post_position(EVT_SET_POSITION, instance, vector);
                Ok(1)
            }
            MSG_AUTOMATA_STOP_SFX => {
                let instance = message.slot14(&[0, 0, 0]);
                host.post_sound(SndSysEvent {
                    event: EVT_STOP_SOUND,
                    a: instance,
                    b: 0,
                    c: 0,
                    d: 0,
                });
                Ok(1)
            }
            // AU15: branch conditions, arguments and derived id aren't specified.
            MSG_AUTOMATA_A => Err(HandlerGap::CapabilityAutomata),
            MSG_AUTOMATA_B => Err(HandlerGap::DirectAutomata),
            _ => Ok(1),
        }
    }
}
