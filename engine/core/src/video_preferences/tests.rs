// SPDX-License-Identifier: GPL-3.0-or-later
use super::fixtures::*;
use super::*;
use std::collections::VecDeque;

const OWNER: Object = Object(1);
const HOST: Object = Object(2);
const ROOT: Object = Object(3);
const APP: Object = Object(4);
const NOTIFY: Object = Object(5);
const ALLOCATION: Object = Object(6);
const MESSAGE: Object = Object(7);
const DATA: Object = Object(8);
const STORAGE: Object = Object(9);
const SERVICE: Object = Object(10);
const MODAL: Object = Object(11);
const MANAGER: Object = Object(12);
const PAUSE: Object = Object(13);
const FRAMEWORK: Object = Object(14);
const PARENT: Object = Object(15);

fn slot(object: Object, slot: u16, args: Vec<Arg>) -> Call {
    Call::Slot { object, slot, args }
}

fn lookup(id: u32) -> Call {
    Call::Lookup {
        root: ROOT,
        id,
        iid: 0x8810,
    }
}
fn control(id: u32) -> Object {
    Object(u64::from(id))
}
fn widget(id: u32) -> Object {
    Object(0x100000000 + u64::from(id))
}

fn state() -> RawState {
    let mut raw = [0xa5; 0x29];
    for (offset, value) in INITIAL_BYTES {
        raw[offset] = value;
    }
    for (offset, value) in INITIAL_DWORDS {
        raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    RawState {
        owner: OWNER,
        host: Some(HOST),
        preferences: Some(Preferences::from_raw(STORAGE, raw)),
        baselines: Baselines {
            b10: 1,
            b12: 1,
            b14: 32,
            b16: 1,
            b18: 1,
            b1c: 1,
        },
        marker: 0,
        first_redraw: 255,
        guard: 0,
        initialized: 1,
    }
}

fn owner(state: RawState) -> Owner {
    Owner::from_raw(
        state,
        Domain {
            valid_objects: true,
            unaliased_storage: true,
            no_foreign_reentry: true,
        },
    )
    .unwrap()
}

/// Raw independent callback inputs. No branch, preference, or marker oracle.
struct Inputs {
    checked: Vec<(u32, u8)>,
    utilities: [u8; 2],
    app: Option<Object>,
    source_al: u8,
    parent: Option<Object>,
    parent_test: u8,
    focus_test: u8,
    service_al: u8,
    modal_result: u32,
    ignored: u32,
    lookup_override: Option<Reply>,
    allocation: Option<Object>,
}

impl Default for Inputs {
    fn default() -> Self {
        Self {
            checked: vec![],
            utilities: [0, 0],
            app: Some(APP),
            source_al: 1,
            parent: None,
            parent_test: 1,
            focus_test: 1,
            service_al: 0,
            modal_result: 0xdeadbeef,
            ignored: 0xabcdef12,
            lookup_override: None,
            allocation: Some(ALLOCATION),
        }
    }
}

impl Host for Inputs {
    fn call(&mut self, call: &Call) -> Result<Reply, Unsupported> {
        Ok(match call {
            Call::Root(_) => Reply::Object(Some(ROOT)),
            Call::Lookup { id, iid: 0x8810, .. } => {
                if let Some(reply) = &self.lookup_override {
                    reply.clone()
                } else if self.checked.iter().any(|(i, _)| i == id) {
                    Reply::Lookup {
                        al: 255,
                        output: Some(control(*id)),
                    }
                } else {
                    Reply::Lookup { al: 0, output: None }
                }
            }
            Call::Utility { id: 0x6a57da54, .. } => Reply::Al(self.utilities[0]),
            Call::Utility { id: 0x0a8b7e00, .. } => Reply::Al(self.utilities[1]),
            Call::Global(Global::App) => Reply::Object(self.app),
            Call::Global(Global::Notifications) => Reply::Object(Some(NOTIFY)),
            Call::Global(Global::ModalService) => Reply::Object(Some(MODAL)),
            Call::Allocate(44) => Reply::Object(self.allocation),
            Call::ConstructMessage(ALLOCATION) => Reply::Object(Some(MESSAGE)),
            Call::Framework => Reply::Object(Some(FRAMEWORK)),
            Call::Service {
                framework: FRAMEWORK,
                id: 0xa417445e,
                iid: 0x5a4,
            } => Reply::Lookup {
                al: self.service_al,
                output: Some(SERVICE),
            },
            Call::WindowManager => Reply::Object(Some(MANAGER)),
            Call::Pause([0, 0, 1, 1]) => Reply::Object(Some(PAUSE)),
            Call::Slot {
                object,
                slot: 0x20,
                args,
            } if args.is_empty() => {
                let (_, al) = self
                    .checked
                    .iter()
                    .find(|(id, _)| control(*id) == *object)
                    .expect("unexpected checked call");
                Reply::Al(*al)
            }
            Call::Slot {
                object: ROOT,
                slot: 0x11c,
                ..
            } => Reply::Al(self.source_al),
            Call::Slot {
                object: ROOT,
                slot: 0x18,
                ..
            } => Reply::Object(self.parent),
            Call::Slot {
                object: ROOT,
                slot: 0x10c,
                ..
            } => Reply::Al(self.parent_test),
            Call::Slot {
                object: OWNER, slot: 8, ..
            } => Reply::U32(3),
            Call::Slot {
                object: ROOT,
                slot: 0x2c,
                ..
            } => Reply::Object(Some(DATA)),
            Call::Slot {
                object: DATA,
                slot: 0x8c,
                args,
            } => match args.as_slice() {
                [Arg::U32(id)] => Reply::Object(Some(widget(*id))),
                _ => panic!("wrong raw widget arguments"),
            },
            Call::Slot {
                object: SERVICE,
                slot: 0x90,
                ..
            } => Reply::U32(0x87654321),
            Call::Slot {
                object: SERVICE,
                slot: 0x60,
                ..
            } => Reply::Al(self.focus_test),
            Call::Slot {
                object: MANAGER,
                slot: 0xa4,
                ..
            } => Reply::U32(self.modal_result),
            Call::Slot { .. } | Call::Unpause(_) | Call::FreeOwner(_) => Reply::U32(self.ignored),
            other => panic!("unprovided primitive input {other:?}"),
        })
    }
}

/// Exact independent primitive scripts for boundary sequences.
struct Script(VecDeque<(Call, Reply)>);
impl Host for Script {
    fn call(&mut self, call: &Call) -> Result<Reply, Unsupported> {
        let (expected, reply) = self.0.pop_front().expect("unexpected call");
        assert_eq!(*call, expected);
        Ok(reply)
    }
}

fn calls(o: &Owner) -> Vec<Call> {
    o.effects()
        .iter()
        .filter_map(|e| match e {
            Effect::Call(c) => Some(c.clone()),
            _ => None,
        })
        .collect()
}

fn writes(o: &Owner) -> Vec<Effect> {
    o.effects()
        .iter()
        .filter(|e| !matches!(e, Effect::Call(_)))
        .cloned()
        .collect()
}

fn message(o: &mut Owner, h: &mut impl Host, index: usize) -> Result<bool, Unsupported> {
    let (kind, subtype, id) = MESSAGES[index];
    o.message(h, ROOT, Some([kind, subtype, id]))
}

fn word_values(o: &Owner) -> [u16; 6] {
    let b = o.state().baselines;
    [b.b10, b.b12, b.b14, b.b16, b.b18, b.b1c]
}

#[test]
fn vp_t1_t3_four_row_projection_only() {
    for (responses, expected_bytes) in PREFIX {
        let mut s = state();
        let p = s.preferences.as_mut().unwrap();
        p.raw[0x0d] = 7;
        p.raw[0x0e] = 9;
        let mut o = owner(s);
        let mut h = Inputs::default();
        for (index, al) in responses.into_iter().enumerate() {
            if let Some(al) = al {
                h.checked.push((0x6a57da49 + index as u32, al));
            }
        }
        o.readback(&mut h).unwrap();
        let prefix_end = o
            .effects()
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e, Effect::Call(Call::Root(_))))
            .nth(4)
            .unwrap()
            .0;
        let actual = &o.effects()[..prefix_end];
        let mut expected = Vec::new();
        let mut prior = None;
        for (i, response) in responses.into_iter().enumerate() {
            let id = 0x6a57da49 + i as u32;
            expected.push(Effect::Call(Call::Root(HOST)));
            if let Some(previous) = prior.take() {
                expected.push(Effect::Call(slot(previous, 8, vec![])));
            }
            expected.push(Effect::Call(lookup(id)));
            if let Some(al) = response {
                let object = control(id);
                prior = Some(object);
                expected.push(Effect::Call(slot(object, 0x20, vec![])));
                if al != 0 {
                    let (field, value) = if i < 3 {
                        (Byte::F0d, i as u8 + 1)
                    } else {
                        (Byte::F0e, 1)
                    };
                    expected.push(Effect::Byte(field, value));
                }
            }
        }
        assert_eq!(actual, expected);
        let p = o.state().preferences.as_ref().unwrap();
        assert_eq!([p.byte(Byte::F0d), p.byte(Byte::F0e)], expected_bytes);
        // No assertion of later callbacks, modal result, or readback AL/EAX.
    }
}

#[test]
fn vp_t4_asymmetric_full_mapping_stores_and_matched_reference_chronology() {
    let initial = state();
    let initial_raw = *initial.preferences.as_ref().unwrap().raw();
    let mut o = owner(initial);
    let mut checked = T4_CHECKED.to_vec();
    checked.extend(T4_DEPTH);
    let mut h = Inputs {
        checked: checked.clone(),
        utilities: [255, 0],
        ..Inputs::default()
    };
    o.readback(&mut h).unwrap();
    let p = o.state().preferences.as_ref().unwrap();
    let mut expected_raw = initial_raw;
    for ((offset, _), value) in INITIAL_BYTES.into_iter().zip(T4_BYTES) {
        expected_raw[offset] = value;
    }
    for ((offset, _), value) in INITIAL_DWORDS.into_iter().zip(T4_DWORDS) {
        expected_raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    assert_eq!(*p.raw(), expected_raw);
    assert_eq!(word_values(&o), INITIAL_WORDS);
    assert_eq!(o.state().marker, 1);
    assert_eq!(
        writes(&o),
        vec![
            Effect::Byte(Byte::F0d, 2),
            Effect::Byte(Byte::F0e, 3),
            Effect::Byte(Byte::F0f, 1),
            Effect::Byte(Byte::F16, 2),
            Effect::Byte(Byte::F28, 1),
            Effect::Byte(Byte::F10, 2),
            Effect::Byte(Byte::F10, 3),
            Effect::Byte(Byte::F13, 1),
            Effect::Byte(Byte::F11, 1),
            Effect::Byte(Byte::F12, 0),
            Effect::Byte(Byte::F15, 1),
            Effect::Marker(1),
            Effect::Byte(Byte::F14, 0),
            Effect::Marker(1),
            Effect::Byte(Byte::F18, 0),
            Effect::Dword(Dword::Width, 1024),
            Effect::Dword(Dword::Height, 768),
            Effect::Dword(Dword::Depth, 32),
        ]
    );
    // Expected primitive chronology comes from the independently transcribed IDs.
    let mut expected = Vec::new();
    let mut prior = None;
    for (id, _) in checked {
        for (before, utility) in [(0x6a57da55, 0x6a57da54), (0x6a57da5a, 0x0a8b7e00)] {
            if id == before {
                expected.push(Call::Root(HOST));
                expected.push(Call::Utility {
                    root: ROOT,
                    id: utility,
                });
            }
        }
        expected.push(Call::Root(HOST));
        if let Some(previous) = prior.take() {
            expected.push(slot(previous, 8, vec![]));
        }
        expected.push(lookup(id));
        expected.push(slot(control(id), 0x20, vec![]));
        prior = Some(control(id));
    }
    expected.push(slot(control(0x2ab6c377), 8, vec![]));
    assert_eq!(calls(&o), expected);
}

#[test]
fn vp_t5_t6_failed_lookups_preserve_all_raw_domains_and_sticky_marker() {
    for marker in [0, 1] {
        let mut s = state();
        s.marker = marker;
        let mut expected = s.clone();
        expected.preferences.as_mut().unwrap().raw[0x11] = 0;
        expected.preferences.as_mut().unwrap().raw[0x18] = 0;
        let mut o = owner(s);
        o.readback(&mut Inputs::default()).unwrap();
        assert_eq!(*o.state(), expected);
        let ids: Vec<_> = calls(&o)
            .iter()
            .filter_map(|c| match c {
                Call::Lookup { id, .. } => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(ids, ALL_LOOKUPS);
        assert!(!calls(&o).iter().any(|c| matches!(c, Call::Slot { slot: 8, .. })));
        assert_eq!(writes(&o), [Effect::Byte(Byte::F11, 0), Effect::Byte(Byte::F18, 0)]);
    }
}

#[test]
fn vp_t7_earlier_resolution_differences_survive_final_agreement() {
    let mut s = state();
    s.baselines.b18 = 3;
    let mut o = owner(s);
    let mut h = Inputs {
        checked: vec![(0x6a57da5a, 1), (0x6a57da5b, 1), (0x6a57da5c, 1), (0x6a57da5d, 1)],
        ..Inputs::default()
    };
    o.readback(&mut h).unwrap();
    let resolution: Vec<_> = writes(&o)
        .into_iter()
        .filter(|e| matches!(e, Effect::Dword(_, _)))
        .collect();
    assert_eq!(
        resolution,
        [
            Effect::Dword(Dword::Width, 800),
            Effect::Dword(Dword::Height, 600),
            Effect::Dword(Dword::Width, 1024),
            Effect::Dword(Dword::Height, 768),
            Effect::Dword(Dword::Width, 1280),
            Effect::Dword(Dword::Height, 1024),
            Effect::Dword(Dword::Width, 1600),
            Effect::Dword(Dword::Height, 1200),
        ]
    );
    assert_eq!(o.state().marker, 1);
}

#[test]
fn vp_t8_t10_message_admission_outchannels_and_unadmitted_gap() {
    for (index, al) in [(0, false), (1, true), (2, true)] {
        let mut o = owner(state());
        assert_eq!(message(&mut o, &mut Inputs::default(), index), Ok(al));
        assert_eq!(calls(&o), [Call::Root(HOST), slot(ROOT, 0x11c, vec![])]);
        assert!(writes(&o).is_empty());
    }
}

#[test]
fn vp_t11_end_dialog_guard_order_and_repeat() {
    let mut s = state();
    s.guard = 1;
    let p = s.preferences.clone();
    let mut o = owner(s);
    assert_eq!(message(&mut o, &mut Inputs::default(), 3), Ok(true));
    assert_eq!(
        o.effects(),
        [
            Effect::Call(Call::Root(HOST)),
            Effect::Call(slot(ROOT, 0x11c, vec![])),
            Effect::Call(slot(OWNER, 4, vec![])),
            Effect::Call(Call::Root(HOST)),
            Effect::Call(slot(ROOT, 0x18, vec![])),
            Effect::Host(None),
            Effect::Call(slot(HOST, 8, vec![])),
            Effect::Guard(0),
            Effect::Call(slot(OWNER, 8, vec![])),
        ]
    );
    assert_eq!(o.state().preferences, p);
    assert_eq!(word_values(&o), INITIAL_WORDS);
    o.take_effects();
    assert_eq!(o.end_dialog(&mut Inputs::default()), Ok(EndResult::GuardZeroAl));
    assert!(o.effects().is_empty());
}

fn save_suffix() -> Vec<Call> {
    vec![
        Call::Global(Global::App),
        slot(APP, 4, vec![]),
        slot(APP, 0x44, vec![]),
        slot(
            APP,
            0x4c,
            vec![Arg::Payload(Payload {
                storage: STORAGE,
                offset: 0xf0c,
            })],
        ),
        slot(APP, 8, vec![]),
        Call::Allocate(44),
        Call::ConstructMessage(ALLOCATION),
        slot(MESSAGE, 4, vec![]),
        slot(MESSAGE, 0x14, vec![Arg::U32(0x231bbf91)]),
        Call::Global(Global::Notifications),
        slot(NOTIFY, 0x10, vec![Arg::Object(Some(MESSAGE)), Arg::U32(0)]),
        slot(MESSAGE, 8, vec![]),
    ]
}

#[test]
fn vp_t12_t14_save_before_apply_even_without_changes_and_despite_marker() {
    for (index, marker) in [(4, 0), (6, 1)] {
        for ignored in [0, 0xffffffff] {
            let mut s = state();
            s.marker = marker;
            let mut o = owner(s);
            assert_eq!(
                message(
                    &mut o,
                    &mut Inputs {
                        ignored,
                        ..Inputs::default()
                    },
                    index
                ),
                Ok(true)
            );
            let actual = calls(&o);
            let start = actual.iter().position(|c| *c == Call::Global(Global::App)).unwrap();
            assert_eq!(&actual[start..], save_suffix());
            assert_eq!(o.state().marker, marker);
            assert_eq!(writes(&o), [Effect::Byte(Byte::F11, 0), Effect::Byte(Byte::F18, 0)]);
        }
    }
}

#[test]
fn vp_t13_sticky_marker_avoids_app_calls_and_keeps_dialog_open() {
    let mut s = state();
    s.marker = 1;
    s.guard = 1;
    let mut o = owner(s);
    assert_eq!(message(&mut o, &mut Inputs::default(), 5), Ok(true));
    let actual = calls(&o);
    assert_eq!(
        &actual[actual.len() - 8..],
        [
            Call::Root(HOST),
            slot(ROOT, 0x2c, vec![]),
            slot(DATA, 0x8c, vec![Arg::U32(0x2a57cb83)]),
            slot(widget(0x2a57cb83), 0x110, vec![Arg::U32(1), Arg::U32(1)]),
            Call::Root(HOST),
            slot(ROOT, 0x2c, vec![]),
            slot(DATA, 0x8c, vec![Arg::U32(0x2a57cb84)]),
            slot(widget(0x2a57cb84), 0x110, vec![Arg::U32(1), Arg::U32(0)]),
        ]
    );
    assert!(!actual
        .iter()
        .any(|c| matches!(c, Call::Global(Global::App) | Call::Allocate(_))));
    assert_eq!((o.state().marker, o.state().guard), (1, 1));
}

#[test]
fn vp_t15_t16_auto_null_domains_have_no_invented_defaults() {
    for (index, app, p) in [(7, None, true), (8, Some(APP), false)] {
        let mut s = state();
        if !p {
            s.preferences = None;
        }
        let initial = s.clone();
        let mut o = owner(s);
        assert_eq!(
            message(
                &mut o,
                &mut Inputs {
                    app,
                    ..Inputs::default()
                },
                index
            ),
            Ok(true)
        );
        let mut expected = vec![Call::Root(HOST), slot(ROOT, 0x11c, vec![]), Call::Global(Global::App)];
        if app.is_some() {
            expected.extend([slot(APP, 4, vec![]), slot(APP, 8, vec![])]);
        }
        assert_eq!(calls(&o), expected);
        assert_eq!(*o.state(), initial);
    }
}

#[test]
fn vp_t17_selection_group_repeat_has_exact_calls_and_no_preference_writes() {
    let initial = state();
    let mut o = owner(initial.clone());
    let mut h = Inputs {
        checked: vec![(0xab81ded5, 1), (0xab81ded6, 0), (0xab81ded7, 1)],
        ..Inputs::default()
    };
    for _ in 0..2 {
        o.take_effects();
        assert_eq!(message(&mut o, &mut h, 9), Ok(true));
        let mut expected = vec![Call::Root(HOST), slot(ROOT, 0x11c, vec![])];
        let mut prior = None;
        for (id, request) in T17_REQUESTS {
            expected.push(Call::Root(HOST));
            if let Some(p) = prior {
                expected.push(slot(p, 8, vec![]));
            }
            expected.extend([lookup(id), slot(control(id), request, vec![])]);
            prior = Some(control(id));
        }
        expected.push(slot(control(0xab81ded7), 8, vec![]));
        assert_eq!(calls(&o), expected);
        assert_eq!(*o.state(), initial);
    }
}

#[test]
fn vp_t18_null_preferences_redraw_is_complete_and_has_no_defined_return_register() {
    let mut s = state();
    s.preferences = None;
    let mut o = owner(s);
    o.redraw(&mut Inputs::default()).unwrap();
    assert_eq!(o.effects(), [Effect::FirstRedraw(0)]);
}

#[test]
fn vp_t19_first_redraw_slice_repeats_and_invalid_enum_looks_up_nothing() {
    for raw in [2, 255] {
        let mut s = state();
        s.preferences.as_mut().unwrap().raw[0x0d] = raw;
        let initial = s.clone();
        let mut o = owner(s);
        let mut h = Inputs {
            checked: vec![(0x6a57da4a, 0), (0x6a57da4b, 1)],
            ..Inputs::default()
        };
        for _ in 0..2 {
            o.take_effects();
            let exit = o.redraw_first_group(&mut h).unwrap();
            if raw == 255 {
                assert_eq!(exit, FirstGroupExit::At0079020d { retained: None });
                assert!(o.effects().is_empty());
            } else {
                assert_eq!(
                    exit,
                    FirstGroupExit::At0079020d {
                        retained: Some(control(0x6a57da4b))
                    }
                );
                let mut expected = vec![Call::Root(HOST), lookup(0x6a57da49)];
                for (i, (id, request)) in T19_REQUESTS.into_iter().enumerate() {
                    expected.push(Call::Root(HOST));
                    if i == 1 {
                        expected.push(slot(control(0x6a57da4a), 8, vec![]));
                    }
                    expected.extend([lookup(id), slot(control(id), request, vec![])]);
                }
                assert_eq!(calls(&o), expected);
                // Vector ends at 0079020d: NO invented final Release here.
            }
            assert_eq!(*o.state(), initial);
        }
    }
}

#[test]
fn vp_t20_unknown_qi_preserves_output_without_addref() {
    let mut o = owner(state());
    let (id, sentinel, expected) = T20;
    let mut out = Object(sentinel);
    assert_eq!(
        o.query_interface(&mut Inputs::default(), id, Some(&mut out)),
        Ok(expected)
    );
    assert_eq!(out, Object(0x12345678));
    assert!(o.effects().is_empty());
}

#[test]
fn vp_t21_shared_release_count_and_idempotent_live_shutdown() {
    let mut o = owner(state());
    let (mut count, expected) = T21;
    assert_eq!(
        o.release_from_raw_count(&mut Inputs::default(), &mut count),
        Ok(expected)
    );
    assert_eq!(count, 2);
    assert_eq!(o.effects(), [Effect::Count(2)]);
    o.take_effects();
    assert_eq!(o.shutdown(), Ok(true));
    assert_eq!(o.shutdown(), Ok(true));
    assert_eq!(o.effects(), [Effect::Preferences(None), Effect::Preferences(None)]);
}

#[test]
fn vp_t22_null_host_modal_returns_true_without_foreign_calls() {
    let mut s = state();
    s.host = None;
    let mut o = owner(s);
    assert_eq!(o.do_modal(&mut Inputs::default()), Ok(true));
    assert!(o.effects().is_empty());
}

#[test]
fn vp_t23_t24_show_guard_precedes_selector_and_preserves_host() {
    for (selector, initialized, guard, expected) in T23_T24 {
        let mut s = state();
        s.initialized = initialized;
        s.guard = guard;
        let initial = s.clone();
        let mut o = owner(s);
        assert_eq!(o.show(selector), Ok(expected));
        assert_eq!(*o.state(), initial);
        assert!(o.effects().is_empty());
    }
}

#[test]
fn unsupported_domains_and_unknown_work_never_become_successful_noops() {
    for (valid_objects, unaliased_storage, no_foreign_reentry, error) in [
        (false, true, true, Unsupported::InvalidObject),
        (true, false, true, Unsupported::AliasedStorage),
        (true, true, false, Unsupported::ForeignReentry),
    ] {
        assert!(
            matches!(Owner::from_raw(state(), Domain { valid_objects, unaliased_storage, no_foreign_reentry }), Err(e) if e == error)
        );
    }
    let mut o = owner(state());
    assert_eq!(o.redraw(&mut Inputs::default()), Err(Unsupported::RemainingRedrawVp1));
    assert_eq!(
        o.action(&mut Inputs::default(), 0x6a57da49),
        Err(Unsupported::UnqualifiedEvent(0x6a57da49))
    );
    assert!(o.effects().is_empty());
    assert_eq!(
        o.action(&mut Inputs::default(), 0xea5e99d9),
        Err(Unsupported::AutoRedrawVp1Vp4)
    );
    assert_eq!(calls(&o), [Call::Global(Global::App)]);
    for (reply, expected) in [
        (Reply::Lookup { al: 1, output: None }, Unsupported::NullRequiredOutput),
        (
            Reply::Lookup {
                al: 0,
                output: Some(Object(99)),
            },
            Unsupported::FailedLookupWithOutput,
        ),
        (
            Reply::Lookup {
                al: 1,
                output: Some(Object(0)),
            },
            Unsupported::InvalidObject,
        ),
    ] {
        let mut o = owner(state());
        let initial = o.state().clone();
        assert_eq!(
            o.readback(&mut Inputs {
                lookup_override: Some(reply),
                ..Inputs::default()
            }),
            Err(expected)
        );
        assert_eq!(*o.state(), initial);
        assert_eq!(calls(&o), [Call::Root(HOST), lookup(0x6a57da49)]);
    }
}

#[test]
fn independent_u16_baselines_nonzero_al_store_before_marker_and_first_checked() {
    let mut s = state();
    s.baselines.b12 = 0x101;
    s.baselines.b16 = 0x100;
    s.baselines.b14 = 0x120;
    let mut o = owner(s);
    let mut h = Inputs {
        checked: vec![
            (0x2ab6c373, 255),
            (0x2ab6c374, 1),
            (0x2ab6c375, 2),
            (0xab81ded5, 0),
            (0xab81ded6, 2),
            (0xab81ded7, 255),
            (0x6a57da57, 255),
            (0x6a57da58, 2),
            (0x2ab6c376, 255),
            (0x2ab6c377, 2),
        ],
        utilities: [2, 255],
        ..Inputs::default()
    };
    o.readback(&mut h).unwrap();
    assert_eq!(
        writes(&o),
        [
            Effect::Byte(Byte::F16, 1),
            Effect::Marker(1),
            Effect::Byte(Byte::F16, 2),
            Effect::Marker(1),
            Effect::Byte(Byte::F16, 3),
            Effect::Marker(1),
            Effect::Byte(Byte::F28, 2),
            Effect::Byte(Byte::F13, 1),
            Effect::Marker(1),
            Effect::Byte(Byte::F0c, 1),
            Effect::Byte(Byte::F13, 0),
            Effect::Marker(1),
            Effect::Byte(Byte::F0c, 1),
            Effect::Byte(Byte::F11, 1),
            Effect::Byte(Byte::F18, 1),
            Effect::Dword(Dword::Depth, 16),
            Effect::Marker(1),
            Effect::Dword(Dword::Depth, 32),
            Effect::Marker(1),
        ]
    );
    assert!(!calls(&o).contains(&lookup(0xab81ded7)));
    assert_eq!(o.state().baselines.b14, 0x120);
}

#[test]
fn admission_source_checks_and_null_message_are_ordered() {
    let mut o = owner(state());
    assert_eq!(o.message(&mut Inputs::default(), Object(99), None), Ok(false));
    assert_eq!(calls(&o), [Call::Root(HOST)]);
    o.take_effects();
    assert_eq!(
        o.message(
            &mut Inputs {
                source_al: 0,
                ..Inputs::default()
            },
            ROOT,
            None
        ),
        Ok(false)
    );
    assert_eq!(calls(&o), [Call::Root(HOST), slot(ROOT, 0x11c, vec![])]);
    o.take_effects();
    assert_eq!(
        o.message(&mut Inputs::default(), ROOT, None),
        Err(Unsupported::NullRequiredOutput)
    );
    for id in [0x6a57da47, 0x6a57da52, 0x6a57da53, 0x6a57da59, 0x6a57da5e, 0xffffffff] {
        assert!(!Owner::admitted(id));
    }
    for id in [
        0x0a8b7e00, 0x2ab6c373, 0x2ab6c377, 0x6a57da48, 0x6a57da51, 0x6a57da54, 0x6a57da58, 0x6a57da5a, 0x6a57da5d,
        0xab81ded7, 0xea57da6f,
    ] {
        assert!(Owner::admitted(id));
    }
}

#[test]
fn end_dialog_parent_paths_have_distinct_al_and_eax_contracts() {
    for al in [0, 255] {
        let mut s = state();
        s.guard = 255;
        let mut o = owner(s);
        assert_eq!(
            o.end_dialog(&mut Inputs {
                parent: Some(PARENT),
                parent_test: al,
                ..Inputs::default()
            }),
            Ok(EndResult::TailReleaseEax(3))
        );
        let mut expected = vec![
            slot(OWNER, 4, vec![]),
            Call::Root(HOST),
            slot(ROOT, 0x18, vec![]),
            slot(ROOT, 0x10c, vec![Arg::U32(0x1000)]),
        ];
        if al != 0 {
            expected.push(slot(
                PARENT,
                0x24,
                vec![Arg::U32(15), Arg::Object(Some(ROOT)), Arg::U32(1)],
            ));
        }
        expected.extend([
            slot(PARENT, 0x5c, vec![Arg::Object(Some(ROOT))]),
            slot(HOST, 8, vec![]),
            slot(OWNER, 8, vec![]),
        ]);
        assert_eq!(calls(&o), expected);
    }
}

#[test]
fn destruction_releases_owned_host_but_not_borrowed_preferences_and_forbids_use_after_free() {
    for mut count in [0, 1] {
        let initial = count;
        let mut o = owner(state());
        assert_eq!(o.release_from_raw_count(&mut Inputs::default(), &mut count), Ok(0));
        assert_eq!(count, initial); // destruction does not decrement first
        assert_eq!(
            o.effects(),
            [
                Effect::Preferences(None),
                Effect::ResetPublishedVptr,
                Effect::Call(slot(HOST, 8, vec![])),
                Effect::Call(Call::FreeOwner(OWNER)),
                Effect::Freed
            ]
        );
        assert_eq!(o.shutdown(), Err(Unsupported::FreedOwner));
        assert_eq!(
            o.release_from_raw_count(&mut Inputs::default(), &mut count),
            Err(Unsupported::FreedOwner)
        );
    }
    for id in [1, 0x22e85d8e] {
        let mut o = owner(state());
        let mut out = Object(99);
        assert_eq!(o.query_interface(&mut Inputs::default(), id, Some(&mut out)), Ok(true));
        assert_eq!(out, OWNER);
        assert_eq!(calls(&o), [slot(OWNER, 4, vec![])]);
        assert_eq!(
            o.query_interface(&mut Inputs::default(), id, None),
            Err(Unsupported::NullRequiredOutput)
        );
    }
}

#[test]
fn depth_tail_has_qualified_incoming_reference_capture_and_cleanup() {
    for (depth, requests) in [(16, Some([0x18, 0x1c])), (32, Some([0x1c, 0x18])), (0x12345678, None)] {
        let mut s = state();
        s.preferences.as_mut().unwrap().raw[0x24..0x28].copy_from_slice(&u32::to_le_bytes(depth));
        let mut o = owner(s);
        let mut h = Inputs {
            checked: vec![(0x2ab6c376, 0), (0x2ab6c377, 0)],
            ..Inputs::default()
        };
        o.redraw_depth_tail(&mut h, Some(Object(99))).unwrap();
        let expected = if let Some([a, b]) = requests {
            vec![
                Call::Root(HOST),
                slot(Object(99), 8, vec![]),
                lookup(0x2ab6c376),
                slot(control(0x2ab6c376), a, vec![]),
                Call::Root(HOST),
                slot(control(0x2ab6c376), 8, vec![]),
                lookup(0x2ab6c377),
                slot(control(0x2ab6c377), b, vec![]),
                slot(control(0x2ab6c377), 8, vec![]),
            ]
        } else {
            vec![slot(Object(99), 8, vec![])]
        };
        assert_eq!(calls(&o), expected);
        assert_eq!(
            writes(&o),
            [Effect::BaselineDepth(depth as u16), Effect::FirstRedraw(0)]
        );
    }
}

#[test]
fn modal_sequence_retains_root_ignores_raw_result_and_conditionally_restores_focus() {
    for focus_test in [0, 255] {
        for modal_result in [0, 0xffffffff] {
            let mut o = owner(state());
            assert_eq!(
                o.do_modal(&mut Inputs {
                    focus_test,
                    modal_result,
                    service_al: 0,
                    ..Inputs::default()
                }),
                Ok(true)
            );
            let mut expected = vec![
                Call::Pause([0, 0, 1, 1]),
                Call::Framework,
                Call::Service {
                    framework: FRAMEWORK,
                    id: 0xa417445e,
                    iid: 0x5a4,
                },
                slot(SERVICE, 0x90, vec![]),
                Call::Root(HOST),
                slot(ROOT, 4, vec![]),
                Call::Global(Global::ModalService),
                slot(MODAL, 0x28, vec![Arg::Object(Some(ROOT)), Arg::U32(1)]),
                slot(SERVICE, 0x94, vec![Arg::Object(Some(ROOT))]),
                Call::WindowManager,
                slot(MANAGER, 0xa4, vec![Arg::Object(Some(ROOT))]),
                slot(MODAL, 0x18, vec![Arg::Object(Some(ROOT))]),
                slot(SERVICE, 0x60, vec![Arg::U32(0x87654321)]),
            ];
            if focus_test != 0 {
                expected.push(slot(SERVICE, 0x94, vec![Arg::U32(0x87654321)]));
            }
            expected.extend([slot(ROOT, 8, vec![]), slot(SERVICE, 8, vec![]), Call::Unpause(PAUSE)]);
            assert_eq!(calls(&o), expected);
        }
    }
}

#[test]
fn missing_services_allocation_and_app_are_explicit_unsupported() {
    let mut o = owner(state());
    let mut script = Script(VecDeque::from([
        (Call::Pause([0, 0, 1, 1]), Reply::Object(Some(PAUSE))),
        (Call::Framework, Reply::Object(None)),
    ]));
    assert_eq!(o.do_modal(&mut script), Err(Unsupported::MissingService));
    assert!(script.0.is_empty());
    let mut o = owner(state());
    assert_eq!(
        o.action(
            &mut Inputs {
                app: None,
                ..Inputs::default()
            },
            0xea57da59
        ),
        Err(Unsupported::NullRequiredOutput)
    );
    assert!(!calls(&o).contains(&Call::Allocate(44)));
    let mut o = owner(state());
    assert_eq!(
        o.action(
            &mut Inputs {
                allocation: None,
                ..Inputs::default()
            },
            0xea57da6f
        ),
        Err(Unsupported::AllocationFailure)
    );
    assert!(!calls(&o).iter().any(|c| matches!(c, Call::ConstructMessage(_))));
}

#[test]
fn show_first_call_resets_guard_and_script_failure_preserves_explicit_outputs() {
    let mut s = state();
    s.initialized = 0;
    s.guard = 1;
    let mut o = owner(s);
    assert_eq!(o.show(1), Ok(false));
    assert_eq!(o.effects(), [Effect::Initialized(1), Effect::Guard(0)]);
    o.take_effects();
    let mut script = Script(VecDeque::from([
        (slot(HOST, 8, vec![]), Reply::Ignored),
        (
            Call::CreateScript {
                key: [0, 0x96a006b0, 0x8a7e052f],
                args: [0, 0x2a57cb82, 0x5386d516],
            },
            Reply::Lookup { al: 0, output: None },
        ),
    ]));
    assert_eq!(o.show_prefix(&mut script, 0, None), Ok(ShowPrefix::ReturnedAl(false)));
    assert!(script.0.is_empty());
    assert_eq!(o.state().host, None);
}

#[test]
fn show_qualified_prefix_borrows_explicit_p_and_stops_before_redraw() {
    let mut s = state();
    s.preferences = None;
    let global_p = state().preferences;
    let mut o = owner(s);
    let ignored = Reply::U32(0xdeadbeef);
    let new_host = Object(20);
    let mut script = Script(VecDeque::from([
        (slot(HOST, 8, vec![]), ignored.clone()),
        (
            Call::CreateScript {
                key: [0, 0x96a006b0, 0x8a7e052f],
                args: [0, 0x2a57cb82, 0x5386d516],
            },
            Reply::Lookup {
                al: 255,
                output: Some(new_host),
            },
        ),
        (Call::Root(new_host), Reply::Object(Some(ROOT))),
        (slot(ROOT, 0x2c, vec![]), Reply::Object(None)),
        (Call::Root(new_host), Reply::Object(Some(ROOT))),
        (slot(ROOT, 0xcc, vec![Arg::U32(640)]), ignored.clone()),
        (Call::Root(new_host), Reply::Object(Some(ROOT))),
        (slot(ROOT, 0xd0, vec![Arg::U32(480)]), ignored.clone()),
        (Call::Root(new_host), Reply::Object(Some(ROOT))),
        (slot(new_host, 0x1c, vec![Arg::Object(Some(OWNER))]), ignored),
    ]));
    assert_eq!(
        o.show_prefix(&mut script, 0, global_p.clone()),
        Ok(ShowPrefix::AtRedraw)
    );
    assert!(script.0.is_empty());
    assert_eq!(o.state().preferences, global_p);
    assert_eq!(o.state().host, Some(new_host));
    assert_eq!(o.state().guard, 1);
    assert_eq!(
        writes(&o),
        [
            Effect::Host(None),
            Effect::Host(Some(new_host)),
            Effect::Preferences(Some(STORAGE)),
            Effect::Guard(1)
        ]
    );
}

#[test]
fn qualified_show_bindings_and_tail_preserve_raw_order_and_false_al() {
    let mut o = owner(state());
    let mut script = VecDeque::new();
    for id in [0xea57da6f, 0xea57da59, 0x6a57da48] {
        script.extend([
            (Call::Root(HOST), Reply::Object(Some(ROOT))),
            (slot(ROOT, 0x8c, vec![Arg::U32(id)]), Reply::Object(Some(widget(id)))),
            (Call::Root(HOST), Reply::Object(Some(ROOT))),
            (slot(widget(id), 0x158, vec![Arg::Object(Some(ROOT))]), Reply::Ignored),
        ]);
    }
    let mut h = Script(script);
    o.show_first_bindings(&mut h).unwrap();
    assert!(h.0.is_empty());
    o.take_effects();
    assert_eq!(o.show_tail(&mut Inputs::default()), Ok(false));
    assert_eq!(
        calls(&o),
        [
            Call::Root(HOST),
            slot(ROOT, 0x2c, vec![]),
            slot(DATA, 0x8c, vec![Arg::U32(0x2a57cb83)]),
            slot(widget(0x2a57cb83), 0x110, vec![Arg::U32(1), Arg::U32(0)]),
            Call::Root(HOST),
            slot(ROOT, 0x2c, vec![]),
            slot(DATA, 0x8c, vec![Arg::U32(0x2a57cb84)]),
            slot(widget(0x2a57cb84), 0x110, vec![Arg::U32(1), Arg::U32(1)]),
        ]
    );
    assert!(writes(&o).is_empty());
}
