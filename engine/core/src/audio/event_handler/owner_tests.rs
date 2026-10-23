// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
enum Call {
    Service(HandlerService),
    Register(bool),
    Subscribe(MessageTarget, u32, bool),
    ReleaseCapability,
    StopRadio,
    Radio31,
    Radio7(u64),
    Radio30(bool),
    Message(StandardMessage, bool),
    Sound(SndSysEvent),
    Position(u32, u32, [f32; 3]),
    Random,
    Query(u32),
    ReleasePayload,
    Slot9,
    Slot10,
    ReleaseMessage,
    Slot14(Vec<u32>),
    GetPosition(u32, [f32; 3]),
}

type Trace = Rc<RefCell<Vec<Call>>>;

struct Host {
    trace: Trace,
    missing: Option<HandlerService>,
    capability: bool,
    random: u32,
}

impl Host {
    fn new() -> Self {
        Self {
            trace: Trace::default(),
            missing: None,
            capability: true,
            random: 0,
        }
    }

    fn take(&mut self) -> Vec<Call> {
        std::mem::take(&mut *self.trace.borrow_mut())
    }

    fn message(&self, ty: u32) -> Message {
        Message {
            trace: self.trace.clone(),
            ty,
            qi: true,
            sound: 9,
            instance: 47,
            position: Some([1.0, 2.0, 3.0]),
        }
    }
}

impl EventHandlerHost for Host {
    fn get_service(&mut self, service: HandlerService) -> bool {
        self.trace.borrow_mut().push(Call::Service(service));
        self.missing != Some(service)
    }
    fn capability_present(&self) -> bool {
        self.capability
    }
    fn register_sound_handler(&mut self, add: bool) {
        self.trace.borrow_mut().push(Call::Register(add));
    }
    fn subscribe(&mut self, target: MessageTarget, message: u32, add: bool) {
        self.trace.borrow_mut().push(Call::Subscribe(target, message, add));
    }
    fn release_capability(&mut self) {
        self.trace.borrow_mut().push(Call::ReleaseCapability);
        self.capability = false;
    }
    fn stop_radio(&mut self) {
        self.trace.borrow_mut().push(Call::StopRadio);
    }
    fn radio_slot31(&mut self) {
        self.trace.borrow_mut().push(Call::Radio31);
    }
    fn radio_slot7(&mut self, value: u64) {
        self.trace.borrow_mut().push(Call::Radio7(value));
    }
    fn radio_slot30(&mut self, value: bool) {
        self.trace.borrow_mut().push(Call::Radio30(value));
    }
    fn post_message(&mut self, message: StandardMessage, immediate: bool) {
        self.trace.borrow_mut().push(Call::Message(message, immediate));
    }
    fn post_sound(&mut self, event: SndSysEvent) {
        self.trace.borrow_mut().push(Call::Sound(event));
    }
    fn post_position(&mut self, event: u32, instance: u32, vector: [f32; 3]) {
        self.trace.borrow_mut().push(Call::Position(event, instance, vector));
    }
    fn vector_seed(&self) -> [f32; 3] {
        [4.0, 5.0, 6.0]
    }
    fn random(&mut self) -> u32 {
        self.trace.borrow_mut().push(Call::Random);
        self.random
    }
}

struct Message {
    trace: Trace,
    ty: u32,
    qi: bool,
    sound: u32,
    instance: u32,
    position: Option<[f32; 3]>,
}

impl EventHandlerMessage for Message {
    fn message_type(&self) -> u32 {
        self.ty
    }
    fn query_payload(&mut self, iid: u32) -> bool {
        self.trace.borrow_mut().push(Call::Query(iid));
        self.qi
    }
    fn release_payload(&mut self) {
        self.trace.borrow_mut().push(Call::ReleasePayload);
    }
    fn slot9(&mut self) -> u32 {
        self.trace.borrow_mut().push(Call::Slot9);
        self.sound
    }
    fn slot10(&mut self) -> u64 {
        self.trace.borrow_mut().push(Call::Slot10);
        0x1234_5678_9ABC_DEF0
    }
    fn release_message(&mut self) {
        self.trace.borrow_mut().push(Call::ReleaseMessage);
    }
    fn slot14(&mut self, arguments: &[u32]) -> u32 {
        self.trace.borrow_mut().push(Call::Slot14(arguments.to_vec()));
        self.instance
    }
    fn position(&mut self, iid: u32, seed: [f32; 3]) -> Option<[f32; 3]> {
        self.trace.borrow_mut().push(Call::GetPosition(iid, seed));
        self.position
    }
}

fn handler(host: &mut Host) -> AudioEventHandler {
    let mut handler = AudioEventHandler::construct(host);
    assert_eq!(handler.init_with(host), Ok(1));
    host.take();
    handler
}

#[test]
fn owner_identity_and_interfaces() {
    assert_eq!(crate::audio::CLSID_EVENT_HANDLER, 0xBDB6_171E);
    assert_eq!(super::super::EVENT_HANDLER_OBJECT_SIZE, 0x50);
    assert_eq!(HANDLER_SERVICE_IID, 0xBB52_3BB2);
    assert_eq!(PAYLOAD_IID, 0x6529_7976);
    assert_eq!(POSITION_IID, 0xA9B4_0F05);
    assert_eq!(super::super::timer_offset(), 0x30);
    assert_eq!(super::super::window_offset(), 0x48);
}

#[test]
fn construction_registration_and_shutdown_preserve_city_set() {
    let mut host = Host::new();
    let mut handler = AudioEventHandler::construct(&mut host);
    assert_eq!(
        host.take(),
        [Call::Service(RADIO_SERVICE), Call::Service(SOUND_SERVICE)]
    );
    assert!(!handler.initialized);
    assert!(!handler.sound_registered);
    assert!(handler.subscribed.is_empty());
    assert!(handler.modal_subscribed.is_empty());
    assert_eq!(handler.limiter, super::super::RateLimiter::new());
    assert_eq!(handler.init_with(&mut host), Ok(1));
    let mut expected: Vec<_> = INIT_SERVICES.into_iter().map(Call::Service).collect();
    expected.push(Call::Register(true));
    expected.extend(MODAL_MESSAGE_TYPES.map(|ty| Call::Subscribe(MessageTarget::Modal, ty, true)));
    expected.extend(init_message_types().map(|ty| Call::Subscribe(MessageTarget::Main, ty, true)));
    assert_eq!(host.take(), expected);
    assert_eq!(handler.modal_subscribed.len(), 2);
    assert_eq!(handler.subscribed.len(), 13);
    let mut city_init = host.message(MSG_POST_CITY_INIT);
    handler.do_message(Some(&mut city_init), &mut host);
    assert_eq!(
        host.take(),
        city_message_types().map(|ty| Call::Subscribe(MessageTarget::Main, ty, true))
    );
    handler.shutdown_with(&mut host);
    let mut expected = vec![Call::Register(false), Call::ReleaseCapability, Call::StopRadio];
    expected.extend(init_message_types().map(|ty| Call::Subscribe(MessageTarget::Main, ty, false)));
    expected.extend(MODAL_MESSAGE_TYPES.map(|ty| Call::Subscribe(MessageTarget::Modal, ty, false)));
    assert_eq!(host.take(), expected);
    assert!(!handler.initialized);
    assert!(!handler.sound_registered);
    assert!(!handler.capability_present);
    assert!(handler.modal_subscribed.is_empty());
    assert_eq!(handler.subscribed, city_message_types().into_iter().collect());
    let mut shutdown = host.message(MSG_PRE_CITY_SHUTDOWN);
    handler.do_message(Some(&mut shutdown), &mut host);
    assert_eq!(
        host.take(),
        city_message_types().map(|ty| Call::Subscribe(MessageTarget::Main, ty, false))
    );
    assert!(handler.subscribed.is_empty());
}

#[test]
fn service_failure_is_explicit_without_fabricated_registration() {
    for missing in INIT_SERVICES {
        let mut host = Host::new();
        host.missing = Some(missing);
        let mut handler = AudioEventHandler::construct(&mut host);
        host.take();
        assert_eq!(handler.init_with(&mut host), Err(HandlerGap::ServiceFailure));
        assert_eq!(host.take(), INIT_SERVICES.map(Call::Service));
        assert!(!handler.initialized);
        assert!(!handler.sound_registered);
        assert!(handler.subscribed.is_empty());
    }
}

#[test]
fn defaults_agree_and_no_capability_is_released_without_acquisition() {
    let a = AudioEventHandler::new();
    let b = AudioEventHandler::default();
    assert_eq!(a.snd_sys_present, b.snd_sys_present);
    let mut host = Host::new();
    host.capability = false;
    host.missing = Some(RADIO_SERVICE);
    let mut handler = AudioEventHandler::construct(&mut host);
    host.take();
    handler.shutdown_with(&mut host);
    assert!(!host
        .take()
        .iter()
        .any(|call| matches!(call, Call::ReleaseCapability | Call::StopRadio | Call::Register(_))));
}

#[test]
fn complete_dispatch_table_has_only_four_automata_types() {
    let cases = [
        (0x89F1_0866, MessageRoute::Construction),
        (0xA9F1_0E45, MessageRoute::Construction),
        (0x2A61_1B51, MessageRoute::RadioSlot31),
        (MSG_POST_CITY_INIT, MessageRoute::CityLifecycle),
        (MSG_PRE_CITY_SHUTDOWN, MessageRoute::CityLifecycle),
        (0x26D3_1EC4, MessageRoute::BuildMessage),
        (0x26D3_1EC5, MessageRoute::SetStationOne),
        (0x2A3A_D653, MessageRoute::Radio),
        (MSG_AUTOMATA_PLAY_SFX, MessageRoute::Automata),
        (MSG_AUTOMATA_A, MessageRoute::Automata),
        (MSG_AUTOMATA_STOP_SFX, MessageRoute::Automata),
        (MSG_AUTOMATA_B, MessageRoute::Automata),
        (0xAA61_1B6E, MessageRoute::RadioSlot7),
        (0xAA65_0861, MessageRoute::RadioSlot30),
        (0xC9F1_011A, MessageRoute::Lab),
        (0xE946_71EB, MessageRoute::Lab),
        (0xFA0C_6390, MessageRoute::ModalMouse),
        (0, MessageRoute::Ignore),
    ];
    for (ty, expected) in cases {
        assert_eq!(message_route(ty), expected);
        assert_eq!(
            AudioEventHandler::route_to_automata(ty).is_some(),
            expected == MessageRoute::Automata
        );
    }
    for ty in init_message_types().into_iter().chain(city_message_types()) {
        if !super::super::automata_message_types().contains(&ty) {
            assert_ne!(message_route(ty), MessageRoute::Automata);
        }
    }
}

#[test]
fn message_return_values_radio_slots_and_deferred_station_message() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    assert_eq!(
        handler.do_message(None, &mut host),
        DispatchOutcome { returned: 0, gap: None }
    );
    let mut unknown = host.message(0xFFFF_FFFF);
    assert_eq!(
        handler.do_message(Some(&mut unknown), &mut host),
        DispatchOutcome { returned: 1, gap: None }
    );
    assert!(host.take().is_empty());
    for ty in [0x2A61_1B51, 0xAA61_1B6E, 0xAA65_0861, 0x26D3_1EC5] {
        let mut message = host.message(ty);
        assert_eq!(
            handler.do_message(Some(&mut message), &mut host),
            DispatchOutcome { returned: 1, gap: None }
        );
    }
    let mut posted = StandardMessage::new();
    posted.set_type(0x2A3A_D653);
    posted.set_data1(1);
    assert_eq!(
        host.take(),
        [
            Call::Radio31,
            Call::Slot10,
            Call::Radio7(0x1234_5678_9ABC_DEF0),
            Call::ReleaseMessage,
            Call::Slot9,
            Call::Radio30(true),
            Call::Message(posted, false)
        ]
    );
    let mut zero = host.message(0xAA65_0861);
    zero.sound = 0;
    handler.do_message(Some(&mut zero), &mut host);
    assert_eq!(host.take(), [Call::Slot9, Call::Radio30(false)]);
    handler.radio_present = false;
    assert_eq!(
        handler.do_message(Some(&mut zero), &mut host).gap,
        Some(HandlerGap::ServiceFailure)
    );
    assert!(host.take().is_empty());
}

#[test]
fn play_stop_position_and_payload_release_follow_spec_order() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    let mut message = host.message(MSG_AUTOMATA_PLAY_SFX);
    assert_eq!(handler.handle_automata(&mut message, &mut host), Ok(1));
    assert_eq!(
        host.take(),
        [
            Call::Query(PAYLOAD_IID),
            Call::Slot9,
            Call::GetPosition(POSITION_IID, [4.0, 5.0, 6.0]),
            Call::Slot14(vec![1, 0]),
            Call::Sound(SndSysEvent {
                event: EVT_START_SOUND,
                a: 9,
                b: 47,
                c: 0,
                d: 0
            }),
            Call::Position(EVT_SET_POSITION, 47, [1.0, 2.0, 3.0]),
            Call::ReleasePayload
        ]
    );
    message.ty = MSG_AUTOMATA_STOP_SFX;
    assert_eq!(handler.handle_automata(&mut message, &mut host), Ok(1));
    assert_eq!(
        host.take(),
        [
            Call::Query(PAYLOAD_IID),
            Call::Slot14(vec![0, 0, 0]),
            Call::Sound(SndSysEvent {
                event: EVT_STOP_SOUND,
                a: 47,
                b: 0,
                c: 0,
                d: 0
            }),
            Call::ReleasePayload
        ]
    );
    message.ty = MSG_AUTOMATA_PLAY_SFX;
    message.position = None;
    handler.handle_automata(&mut message, &mut host).unwrap();
    assert!(host
        .take()
        .contains(&Call::Position(EVT_SET_POSITION, 47, [4.0, 5.0, 6.0])));
}

#[test]
fn entry_failures_balance_only_successful_queries_and_main_still_returns_one() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    let mut message = host.message(MSG_AUTOMATA_PLAY_SFX);
    handler.capability_present = false;
    assert_eq!(handler.handle_automata(&mut message, &mut host), Ok(0));
    assert!(host.take().is_empty());
    handler.capability_present = true;
    message.qi = false;
    assert_eq!(handler.handle_automata(&mut message, &mut host), Ok(0));
    assert_eq!(host.take(), [Call::Query(PAYLOAD_IID)]);
    message.qi = true;
    handler.snd_sys_present = false;
    assert_eq!(handler.handle_automata(&mut message, &mut host), Ok(0));
    assert_eq!(host.take(), [Call::Query(PAYLOAD_IID), Call::ReleasePayload]);
    assert_eq!(handler.do_message(Some(&mut message), &mut host).returned, 1);
    assert_eq!(host.take(), [Call::Query(PAYLOAD_IID), Call::ReleasePayload]);
    assert_eq!(handler.limiter, super::super::RateLimiter::new());
}

#[test]
fn subscribed_delivery_only_hears_automata_in_city_but_direct_posts_work() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    let mut play = host.message(MSG_AUTOMATA_PLAY_SFX);
    assert_eq!(handler.deliver_main(&mut play, &mut host), None);
    assert!(host.take().is_empty());
    let mut city = host.message(MSG_POST_CITY_INIT);
    assert!(handler.deliver_main(&mut city, &mut host).is_some());
    assert!(handler.deliver_main(&mut play, &mut host).is_some());
    city.ty = MSG_PRE_CITY_SHUTDOWN;
    handler.deliver_main(&mut city, &mut host);
    host.take();
    assert_eq!(handler.deliver_main(&mut play, &mut host), None);
    assert!(host.take().is_empty());
    assert_eq!(handler.do_message(Some(&mut play), &mut host).returned, 1);
    assert!(host.take().iter().any(|call| matches!(call, Call::Sound(_))));
}

#[test]
fn limiter_draws_only_on_restart_is_shared_and_accepts_exact_threshold() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    let mut message = host.message(MSG_AUTOMATA_PLAY_SFX);
    message.sound = RATE_LIMIT_PAYLOAD;
    host.random = crate::audio::RAND_MAX;
    handler.handle_automata(&mut message, &mut host).unwrap();
    assert_eq!(handler.limiter.window_ms, 4000);
    assert_eq!(host.take().iter().filter(|call| **call == Call::Random).count(), 1);
    handler.limiter.elapsed_ms = 3999;
    message.instance = 99;
    handler.handle_automata(&mut message, &mut host).unwrap();
    assert_eq!(
        host.take(),
        [Call::Query(PAYLOAD_IID), Call::Slot9, Call::ReleasePayload]
    );
    handler.limiter.elapsed_ms = 4000;
    host.random = 0;
    handler.handle_automata(&mut message, &mut host).unwrap();
    assert_eq!(handler.limiter.window_ms, 1000);
    assert_eq!(handler.limiter.elapsed_ms, 0);
    assert!(host.take().contains(&Call::Random));
    message.sound = 3;
    handler.handle_automata(&mut message, &mut host).unwrap();
    let calls = host.take();
    assert!(!calls.contains(&Call::Random));
    assert!(calls.iter().any(|call| matches!(call, Call::Sound(_))));
}

#[test]
fn missing_owner_algorithms_are_visible_and_do_not_emit_guessed_sounds() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    for ty in [
        0x89F1_0866,
        0xA9F1_0E45,
        0x26D3_1EC4,
        0x2A3A_D653,
        0xC9F1_011A,
        0xE946_71EB,
        0xFA0C_6390,
    ] {
        let mut message = host.message(ty);
        assert_eq!(
            handler.do_message(Some(&mut message), &mut host),
            DispatchOutcome {
                returned: 1,
                gap: Some(HandlerGap::MessageAlgorithm(message_route(ty))),
            }
        );
        assert!(host.take().is_empty());
    }
    for (ty, gap) in [
        (MSG_AUTOMATA_A, HandlerGap::CapabilityAutomata),
        (MSG_AUTOMATA_B, HandlerGap::DirectAutomata),
    ] {
        let mut message = host.message(ty);
        assert_eq!(handler.handle_automata(&mut message, &mut host), Err(gap));
        assert_eq!(host.take(), [Call::Query(PAYLOAD_IID), Call::ReleasePayload]);
    }
    assert_eq!(handler.do_modal_message(), Err(HandlerGap::ModalNotice));
    assert_eq!(handler.do_event(), Err(HandlerGap::EventCallback));
}

/// §16.21 and §16.25: consume actual automaton posts and forward only the
/// handler's resulting sound events to a sound manager across the foreign bus.
#[test]
fn full_chain_crosses_handler_and_sound_system_without_a_save_record() {
    use crate::audio::automaton::AutomatonSfxHost;
    use crate::audio::sound_manager::AudioSoundManager;

    let mut host = Host::new();
    let mut handler = handler(&mut host);
    let mut city = host.message(MSG_POST_CITY_INIT);
    handler.do_message(Some(&mut city), &mut host);
    host.take();
    let mut automaton = AutomatonSfxHost::new();
    let mut manager = AudioSoundManager::new();
    assert!(automaton.handle_enter_state(123, 0x1000));
    assert_eq!(automaton.posted.len(), 1);
    assert!(manager.instances.is_empty()); // MessagePost is asynchronous.
    let posted = automaton.posted[0];
    let instance = automaton.state.instance_id;
    assert_ne!(instance, 0);
    assert_eq!(posted.data2, instance);
    let mut message = host.message(posted.msg_type);
    message.sound = posted.data1;
    message.instance = posted.data2;
    assert!(!manager.do_message(posted.msg_type));
    assert!(handler.deliver_main(&mut message, &mut host).unwrap().gap.is_none());
    let calls = host.take();
    let events: Vec<_> = calls
        .iter()
        .filter_map(|call| match call {
            Call::Sound(event) => Some(*event),
            _ => None,
        })
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event, EVT_START_SOUND);
    assert!(calls.contains(&Call::Position(EVT_SET_POSITION, instance, [1.0, 2.0, 3.0])));
    for event in events {
        manager.do_event(event.event, event.a, event.b, 0);
    }
    assert_eq!(manager.instances.len(), 1);
    assert!(manager.instances.contains_key(&instance));

    assert!(automaton.handle_exit_state(123, 0x1000));
    assert_eq!(automaton.posted.len(), 2);
    let posted = automaton.posted[1];
    assert!(!manager.do_message(posted.msg_type));
    message.ty = posted.msg_type;
    message.sound = posted.data1;
    message.instance = posted.data2;
    assert!(handler.deliver_main(&mut message, &mut host).unwrap().gap.is_none());
    let events: Vec<_> = host
        .take()
        .into_iter()
        .filter_map(|call| match call {
            Call::Sound(event) => Some(event),
            _ => None,
        })
        .collect();
    assert_eq!(
        events,
        [SndSysEvent {
            event: EVT_STOP_SOUND,
            a: instance,
            b: 0,
            c: 0,
            d: 0
        }]
    );
    for event in events {
        manager.do_event(event.event, event.a, event.b, 0);
    }
    assert!(manager.instances.is_empty());
    assert!(!crate::audio::save::is_saved("AudioEventHandler"));
    assert!(crate::audio::save::SAVE_RECORDS.is_empty());
}
