// SPDX-License-Identifier: GPL-3.0-or-later
//! audio.md §16.26–42 owner-boundary vectors.
use super::*;

#[test]
fn enumeration_callback_preserves_the_property_and_balances_qi() {
    struct Object {
        query_succeeds: bool,
        calls: Vec<(u32, u32)>,
    }
    impl HandlerSoundObject for Object {
        fn query(&mut self, iid: u32) -> bool {
            self.calls.push((0, iid));
            self.query_succeeds
        }
        fn read(&mut self, key: u32, default: u32) -> u32 {
            self.calls.push((key, default));
            91
        }
        fn write(&mut self, key: u32, value: u32) {
            self.calls.push((key, value));
        }
        fn release(&mut self) {
            self.calls.push((2, 0));
        }
    }
    let mut object = Object {
        query_succeeds: true,
        calls: Vec::new(),
    };
    assert!(enumerate_callback(&mut object));
    assert_eq!(object.calls, [(0, 0xFD58_4467), (32, 0), (32, 91), (2, 0)]);
    object.calls.clear();
    object.query_succeeds = false;
    assert!(enumerate_callback(&mut object));
    assert_eq!(object.calls, [(0, 0xFD58_4467)]);
}

fn controls(host: &mut Host) -> Vec<HandlerCall> {
    host.take()
        .into_iter()
        .filter_map(|call| match call {
            Call::Control(value) => Some(value),
            _ => None,
        })
        .collect()
}

#[test]
fn construction_matrix_and_acquired_cleanup() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    for (ty, positional, mask, expected_slot) in [
        (0x89F1_0866, false, 0, 14),
        (0x89F1_0866, false, 4, 14),
        (0x89F1_0866, true, 2, 13),
        (0xA9F1_0E45, true, 2, 15),
    ] {
        let mut message = host.message(ty);
        message.sound = 91;
        message.instance = u32::from(positional);
        message.third = mask;
        assert!(handler.handle_construction(&mut message, &mut host));
        let calls = host.take();
        let sound = if mask & 2 != 0 { 0xEA62_C800 } else { 0xAA62_C82A };
        assert!(calls.contains(&Call::Control(HandlerCall::Scape(
            expected_slot,
            sound,
            91,
            positional.then_some([1.0, 2.0, 3.0]),
            0.0
        ))));
        assert_eq!(calls.last(), Some(&Call::ReleasePayload));
        assert_eq!(calls.contains(&Call::ReleasePosition), positional);
        if positional {
            message.position = None;
            assert!(handler.handle_construction(&mut message, &mut host));
            assert!(controls(&mut host).is_empty());
        }
    }
    let mut message = host.message(0xA9F1_0E45);
    message.pointer = 0;
    assert!(!handler.handle_construction(&mut message, &mut host));
    assert_eq!(host.take().last(), Some(&Call::ReleasePayload));
}

#[test]
fn station_defaults_explicit_fade_and_same_station() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    for (selector, station) in [(0, "kmay"), (1, "krgn")] {
        let mut message = host.message(0x2A3A_D653);
        message.sound = selector;
        message.present[1] = false;
        assert!(handler.handle_radio(&mut message, &mut host));
        assert_eq!(
            controls(&mut host),
            [
                HandlerCall::RadioCurrent,
                HandlerCall::RadioResolveCurrent("current".into()),
                HandlerCall::RadioResolveRequested(Some(station)),
                HandlerCall::RadioSelect(station, [0, 4000])
            ]
        );
        message.present[1] = true;
        message.instance = 123;
        assert!(handler.handle_radio(&mut message, &mut host));
        assert!(controls(&mut host).contains(&HandlerCall::RadioSelect(station, [0, 123])));
        host.reply_word = 100; // Both resolutions now return the same station.
        assert!(handler.handle_radio(&mut message, &mut host));
        let calls = host.take();
        assert!(calls.contains(&Call::Radio30(false)));
        assert!(!calls
            .iter()
            .any(|c| matches!(c, Call::Control(HandlerCall::RadioSelect(..)))));
        host.reply_word = 0;
    }
    let mut message = host.message(0x2A3A_D653);
    message.sound = 2;
    assert!(handler.handle_radio(&mut message, &mut host));
    assert!(controls(&mut host).is_empty());
    message.qi = false;
    assert!(!handler.handle_radio(&mut message, &mut host));
    assert_eq!(host.take(), [Call::Query(PAYLOAD_IID)]);
}

#[test]
fn transferred_reference_and_radio_control_rules() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    let mut message = host.message(0xAA61_1B6E);
    handler.radio_present = false;
    handler.do_message(Some(&mut message), &mut host);
    assert_eq!(
        host.take(),
        [
            Call::Slot10,
            Call::Control(HandlerCall::ReleaseTransferred(0x1234_5678_9ABC_DEF0))
        ]
    );
    handler.radio_present = true;
    message.ty = 0x0A61_1B34;
    message.sound = 0;
    message.instance = 44;
    message.present[2] = false;
    host.reply_word = 91;
    handler.do_message(Some(&mut message), &mut host);
    assert_eq!(
        controls(&mut host),
        [
            HandlerCall::RadioResolveRequested(Some("kmay")),
            HandlerCall::StationVolume(91, 44),
            HandlerCall::StationMode(91, 2)
        ]
    );
    message.third = (-7_i32) as u32;
    message.present[2] = true;
    handler.do_message(Some(&mut message), &mut host);
    assert!(controls(&mut host).contains(&HandlerCall::StationMode(91, 1)));
    message.ty = 0xAA65_0860;
    message.sound = 1;
    host.reply_word = 0;
    handler.do_message(Some(&mut message), &mut host);
    let calls = host.take();
    assert!(calls.contains(&Call::Radio16(44)));
    assert!(calls.contains(&Call::Radio23(true)));
    assert!(calls.contains(&Call::Control(HandlerCall::RadioStart(0))));
    host.reply_word = 1;
    handler.do_message(Some(&mut message), &mut host);
    assert!(!host.take().contains(&Call::Radio23(true)));
    message.ty = 0x4348_B111;
    host.reply_word = 0;
    handler.do_message(Some(&mut message), &mut host);
    assert_eq!(host.take(), [Call::Control(HandlerCall::PreferenceGate)]);
    host.reply_word = 1;
    handler.do_message(Some(&mut message), &mut host);
    assert_eq!(
        host.take(),
        [
            Call::Control(HandlerCall::PreferenceGate),
            Call::Query(PAYLOAD_IID),
            Call::Radio23(true),
            Call::ReleasePayload
        ]
    );
    message.ty = 0x4A7F_B7E9;
    handler.do_message(Some(&mut message), &mut host);
    assert_eq!(
        controls(&mut host),
        [HandlerCall::SoundRequest(0x0BAA_B7A4, [1, 0, 0, 0])]
    );
}

#[test]
fn dispatch_categories_and_capability_notifications() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    for (ty, sound) in [(0x8946_71F6, 0x09C6_6951), (0xE946_71EB, 0x29D9_9E9E)] {
        for category in [0, 1, 5, 6, 2, u32::MAX] {
            let mut message = host.message(ty);
            message.sound = category;
            assert!(handler.handle_dispatch(&mut message, &mut host));
            let calls = host.take();
            if matches!(category, 0 | 1 | 5 | 6) {
                assert!(calls.contains(&Call::Control(HandlerCall::CreateEvent(
                    EVT_START_SOUND,
                    0x5B52_3BA4,
                    [sound, 77, 0, 0]
                ))));
                assert!(calls.contains(&Call::Control(HandlerCall::SubmitEvent(0))));
                assert!(calls.contains(&Call::Control(HandlerCall::ReleaseEvent(0))));
            } else {
                assert_eq!(calls, [Call::Query(PAYLOAD_IID), Call::ReleasePayload]);
            }
            assert_eq!(calls.last(), Some(&Call::ReleasePayload));
        }
    }
    for ty in [0xC9F1_011A, 0x49F1_0111] {
        let mut message = host.message(ty);
        host.reply_word = 100;
        assert!(handler.handle_dispatch(&mut message, &mut host));
        assert_eq!(
            controls(&mut host),
            [
                HandlerCall::RetainObject(77),
                HandlerCall::QueryObject(77, 0xC9B6_D7D6),
                HandlerCall::ReleaseObject(100),
                HandlerCall::ReleaseObject(77)
            ]
        );
        host.reply_word = 0;
        assert!(!handler.handle_dispatch(&mut message, &mut host));
        assert_eq!(
            controls(&mut host),
            [
                HandlerCall::RetainObject(77),
                HandlerCall::QueryObject(77, 0xC9B6_D7D6),
                HandlerCall::ReleaseObject(77)
            ]
        );
    }
}

#[test]
fn alternate_automata_matrix_and_indeterminate_position() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    let mut message = host.message(MSG_AUTOMATA_A);
    message.sound = 0;
    assert_eq!(handler.handle_automata(&mut message, &mut host), Ok(1));
    assert_eq!(
        controls(&mut host),
        [
            HandlerCall::Scape(14, 0x49DE_8A94, 77, None, 0.0),
            HandlerCall::Scape(14, 0xA9DE_8B59, 77, None, 0.0)
        ]
    );
    for (selector, sound) in [(1, 0x49DE_8A94), (u32::MAX, 0xA9DE_8B59)] {
        message.sound = selector;
        assert_eq!(handler.handle_automata(&mut message, &mut host), Ok(1));
        assert_eq!(
            controls(&mut host),
            [HandlerCall::Scape(13, sound, 77, Some([1.0, 2.0, 3.0]), 50.0)]
        );
    }
    for index in [0, 1] {
        message.present[index] = false;
        assert_eq!(handler.handle_automata(&mut message, &mut host), Ok(0));
        assert_eq!(host.take().last(), Some(&Call::ReleasePayload));
        message.present[index] = true;
    }
    message.ty = MSG_AUTOMATA_B;
    host.reply_word = 99;
    for (second, first, sound) in [
        (1, 0, 0x4A5E_7FFC),
        (1, 1, 0x0A5E_802C),
        (2, 0, 0x2A5E_7F2C),
        (2, 1, 0x0A5E_7F51),
        (3, 0, 0x2A5E_8067),
        (3, 1, 0x6A5E_8088),
        (0, 1, 0x69F0_1280),
        (u32::MAX, 0, 0x69F0_1280),
    ] {
        message.sound = first;
        message.instance = second;
        assert_eq!(handler.handle_automata(&mut message, &mut host), Ok(1));
        assert_eq!(
            controls(&mut host),
            [
                HandlerCall::SoundManagerAccess,
                HandlerCall::GenerateInstance,
                HandlerCall::Scape(13, sound, 99, Some([1.0, 2.0, 3.0]), 50.0)
            ]
        );
    }
    message.present = [false; 3];
    message.position = None;
    assert_eq!(handler.handle_automata(&mut message, &mut host), Ok(1));
    assert_eq!(
        controls(&mut host),
        [
            HandlerCall::SoundManagerAccess,
            HandlerCall::GenerateInstance,
            HandlerCall::Scape(13, 0x69F0_1280, 99, None, 50.0)
        ]
    );
}

#[test]
fn modal_stack_transitions_and_inclusive_mouse_bounds() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    host.reply_word = 100;
    handler.do_modal_message(0x62BA_7B42, 11, &mut host);
    handler.do_modal_message(0x62BA_7B42, 22, &mut host);
    handler.do_modal_message(0x62BA_7B43, 999, &mut host);
    assert_eq!(handler.modal_windows, [11]);
    handler.do_modal_message(0x62BA_7B43, 999, &mut host);
    let calls = controls(&mut host);
    assert_eq!(
        calls
            .iter()
            .filter(|call| matches!(call, HandlerCall::WindowSubscription(..)))
            .count(),
        2
    );
    assert!(calls.contains(&HandlerCall::WindowSubscription(100, 0xFA0C_6390, true)));
    assert!(calls.contains(&HandlerCall::WindowSubscription(100, 0xFA0C_6390, false)));
    assert!(calls.contains(&HandlerCall::ReleaseObject(22)));
    assert!(!calls.contains(&HandlerCall::ReleaseObject(999)));
    handler.do_modal_message(0x62BA_7B42, 11, &mut host);
    host.take();
    let mut message = host.message(0xFA0C_6390);
    for subtype in [7, 8, 9] {
        message.subtype = subtype;
        for point in [
            [10, 20],
            [30, 20],
            [10, 40],
            [30, 40],
            [20, 20],
            [20, 40],
            [10, 30],
            [30, 30],
        ] {
            message.point = point;
            assert_eq!(handler.do_message(Some(&mut message), &mut host).returned, 1);
            assert!(!controls(&mut host)
                .iter()
                .any(|call| matches!(call, HandlerCall::AcquireGlobalSound)));
        }
        for point in [[9, 20], [31, 40], [10, 19], [30, 41]] {
            message.point = point;
            assert_eq!(handler.do_message(Some(&mut message), &mut host).returned, 1);
            assert!(controls(&mut host).contains(&HandlerCall::GlobalSoundRequest(100, [1, 0xAA62_C9D5, 0, 0, 0])));
        }
    }
    host.reply_word = 0;
    assert_eq!(handler.do_message(Some(&mut message), &mut host).returned, 0);
    assert_eq!(host.take().last(), Some(&Call::Control(HandlerCall::ReleaseObject(11))));
    message.qi = false;
    assert_eq!(handler.do_message(Some(&mut message), &mut host).returned, 1);
}

#[test]
fn packed_play_truncates_and_releases_after_both_posts() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    let mut message = host.message(MSG_AUTOMATA_PLAY_SFX);
    message.sound = 55;
    message.instance = 77;
    message.position = Some([1.9, -2.9, 3.1]);
    assert_eq!(handler.handle_automata(&mut message, &mut host), Ok(1));
    let calls = host.take();
    assert!(calls.contains(&Call::Sound(SndSysEvent {
        event: EVT_START_SOUND,
        a: 55,
        b: 77,
        c: 1,
        d: 0
    })));
    assert!(calls.contains(&Call::Sound(SndSysEvent {
        event: EVT_SET_POSITION,
        a: 77,
        b: 1,
        c: (-2_i32) as u32,
        d: 3
    })));
    assert_eq!(&calls[calls.len() - 2..], [Call::ReleasePosition, Call::ReleasePayload]);
    assert!(calls
        .iter()
        .all(|call| !matches!(call, Call::Slot14(args) if !args.is_empty())));
}

#[test]
fn torch_shared_history_volume_gate_wrap_and_sticky_mode() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    let mut other = handler.clone();
    host.reply_word = 1;
    let mut state = TorchState {
        movement_volume: true,
        ..TorchState::default()
    };
    let mut event = host.message(0x4A10_6126);
    event.sound = 3;
    event.instance = 4;
    event.third = 100;
    assert!(!handler.handle_torch(&mut event, &mut host, &mut state));
    assert_eq!(state.speed, 2.5);
    let calls = host.take();
    assert!(calls.contains(&Call::Sound(SndSysEvent {
        event: 0x4A45_1915,
        a: 0x602,
        b: 617,
        c: 0,
        d: 0
    })));
    other.handle_torch(&mut event, &mut host, &mut state);
    assert_eq!(state.speed, 1.25);
    assert!(host.take().contains(&Call::Sound(SndSysEvent {
        event: 0x4A45_1915,
        a: 0x603,
        b: 608,
        c: 0,
        d: 0
    })));
    state.timestamp = 0xFFFF_FFF0;
    state.speed = 0.0;
    event.third = 0x10;
    event.sound = i32::MAX as u32;
    other.handle_torch(&mut event, &mut host, &mut state);
    assert_eq!(state.speed, 30.0);
    host.take();
    event.ty = 0xAA96_FE8F;
    other.handle_torch(&mut event, &mut host, &mut state);
    assert!(state.mode);
    let calls = host.take();
    assert!(calls.contains(&Call::Sound(SndSysEvent {
        event: EVT_START_SOUND,
        a: 0x6A10_ABCF,
        b: 0x600,
        c: 0,
        d: 0
    })));
    other.handle_torch(&mut event, &mut host, &mut state);
    assert_eq!(host.take(), [Call::Control(HandlerCall::GlobalSoundPresent)]);
    event.ty = 0x8A10_611A;
    other.handle_torch(&mut event, &mut host, &mut state);
    let calls = host.take();
    assert_eq!(calls.iter().filter(|call| matches!(call, Call::Sound(_))).count(), 3);
    assert!(state.mode && state.movement_volume);
    let saved = state;
    host.reply_word = 0;
    other.handle_torch(&mut event, &mut host, &mut state);
    assert_eq!(state, saved);
}

#[test]
fn callback_return_qi_independence_and_destructor_not_shutdown() {
    let mut host = Host::new();
    let mut handler = handler(&mut host);
    let mut event = host.message(0xFFFF_FFFF);
    let mut state = TorchState::default();
    event.qi = false;
    assert!(!handler.do_event_with_torch(&mut event, &mut host, &mut state));
    assert_eq!(
        host.take(),
        [
            Call::Service(HandlerService(0x1AF8_5BFB, 0x3AF8_5BB7)),
            Call::Query(PAYLOAD_IID),
            Call::ReleaseService(HandlerService(0x1AF8_5BFB, 0x3AF8_5BB7))
        ]
    );
    event.ty = 0xAA36_D9B6;
    handler.do_event_with_torch(&mut event, &mut host, &mut state);
    assert!(host.take().contains(&Call::Enumerate));
    host.reply_word = 45;
    event.ty = 0x0BAA_B7A4;
    event.sound = 45;
    handler.do_event_with_torch(&mut event, &mut host, &mut state);
    assert_eq!(
        controls(&mut host),
        [
            HandlerCall::ObtainSoundInterface,
            HandlerCall::SoundInterfaceValue(45, 127, 45),
            HandlerCall::ReleaseObject(45)
        ]
    );
    handler.do_modal_message(0x62BA_7B42, 11, &mut host);
    host.take();
    assert!(!handler.destroy_with(0, &mut host));
    assert_eq!(
        host.take(),
        [
            Call::ReleaseCapability,
            Call::ReleaseService(SOUND_SERVICE),
            Call::ReleaseRadio
        ]
    );
    assert!(handler.modal_windows.is_empty());
    assert!(handler.destroy_with(1, &mut host));
    assert!(host.take().is_empty());
    assert!(handler.do_event_with_torch(&mut event, &mut host, &mut state));
    assert!(host.take().is_empty());
}
