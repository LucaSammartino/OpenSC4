// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The automata layer through the C ABI (docs/specs/automata.md): fake game
//! objects behind the host tables, byte-buffer streams.

use std::cell::RefCell;
use std::ffi::{c_char, c_void, CStr, CString};

use crate::automata::*;
use crate::stream::fc4_ref;
use crate::test_streams::{istream, ostream, In, Out};

const BUILDING_CLASS: u32 = 0xA9BD_882D;
const BUILDING_TYPE: u32 = 0x2781_28A0;
const GENERATOR: u32 = 0x4A16_28C5;
const ATTRACTOR: u32 = 0x6A16_28BA;
const PEDESTRIAN: u32 = 0x896E_75AF;
const VEHICLE: u32 = 0x896E_714A;
const PATROL_CAR: u32 = 0x4A22_2B98;
const TRAILER: u32 = 0x4A5B_69ED;
const VEHICLE_MANAGER: u32 = 0xA98F_9D3B;
const PEDESTRIAN_MANAGER: u32 = 0xC98F_9D17;
const AIRCRAFT_MANAGER: u32 = 0xC98F_9D4A;
const WATERCRAFT_MANAGER: u32 = 0xA98F_9D28;

#[test]
fn derived_owner_records_replay_full_payload_and_release_every_acquisition() {
    use fc4_core::automata::{AircraftPlacementRecord, AutomatonBase, PedestrianRecord, VehicleRecord};
    use fc4_core::stream::{OutStream, RefValue};

    fn encode(write: impl FnOnce(&mut dyn OutStream) -> bool) -> Vec<u8> {
        unsafe { write_with(|s| write(&mut crate::stream::COut(&*s))) }
    }

    let leader = RefValue::Host {
        token: 17,
        class_id: PEDESTRIAN,
    };
    let mut base = AutomatonBase {
        leader: leader.clone(),
        tail: 0x1234_5678,
        ..AutomatonBase::default()
    };
    base.vectors[0] = [12.5, -7.0, 42.0];
    let mut aircraft = AircraftPlacementRecord::default();
    aircraft.aircraft.base = base.clone();
    aircraft.home_position = [-8.0, 91.5, 3.0];
    let placement = encode(|out| aircraft.write(out));
    let mut vehicle = VehicleRecord::new();
    vehicle.traffic.base = base.clone();
    // Independent wire framing: Bus/TrainCar add one u16; Train adds two
    // and retains a distinct car list, trailing network and raw dwords.
    let car = encode(|out| out.put_u16(2) && vehicle.write(out));
    let train = encode(|out| {
        out.put_u16(2);
        out.put_bytes(&car);
        out.put_u32(1);
        out.put_ref(&RefValue::Host {
            token: 18,
            class_id: 0x4A41_6254,
        });
        out.put_u32(0x7FC0_0001);
        for x in [1.0, 2.0, 3.0] {
            out.put_f32(x);
        }
        out.put_ref(&RefValue::Host {
            token: 19,
            class_id: 0x49C1_A034,
        });
        out.put_u32(0x8000_0000)
    });
    let water = encode(|out| out.put_u16(2) && base.write(out));
    let aircraft_base = encode(|out| out.put_u16(1) && base.write(out));
    let mut pedestrian = PedestrianRecord::new();
    pedestrian.traffic.base = base;
    let one_shot = encode(|out| pedestrian.write(out));
    for (class, bytes, tokens) in [
        (0xCBEB_4F62, &placement, vec![17]),
        (0x8BA9_8EDA, &placement, vec![17]),
        (0xC96E_75DC, &aircraft_base, vec![17]),
        (0x096E_75F5, &water, vec![17]),
        (0xAA09_DC28, &one_shot, vec![17]),
        (0xAA9F_4A3D, &car, vec![17]),
        (0x4A41_6254, &car, vec![17]),
        (0x8A41_625A, &train, vec![17, 18, 19]),
    ] {
        let log = RefCell::new(Log::default());
        let host = ref_host(&log);
        let mut input = In::new(bytes);
        let mut output = Out::default();
        unsafe {
            assert_eq!(
                fc4_automata_derived_record_roundtrip(class, &istream(&mut input), &ostream(&mut output), &host),
                2,
                "class {class:08X}"
            );
        }
        assert_eq!(input.pos, bytes.len());
        assert_eq!(&output.bytes, bytes);
        assert_eq!(log.borrow().releases, tokens);

        log.borrow_mut().releases.clear();
        let mut input = In::new(&bytes[..bytes.len() - 1]);
        unsafe {
            assert_eq!(
                fc4_automata_derived_record_roundtrip(
                    class,
                    &istream(&mut input),
                    &ostream(&mut Out::default()),
                    &host,
                ),
                1
            );
        }
        assert_eq!(log.borrow().releases, tokens);
    }
}

#[test]
fn derived_owner_records_reject_unknown_classes_and_failed_streams() {
    unsafe extern "C" fn refuse(_: *mut c_void, _: u16) -> bool {
        false
    }
    unsafe {
        let log = RefCell::new(Log::default());
        let host = ref_host(&log);
        let mut bytes = Out::default();
        fc4_helicopter_shadow_fixture(&ostream(&mut bytes));
        let mut input = In::new(&bytes.bytes);
        for class in [0, 0xFFFF_FFFF] {
            assert_eq!(
                fc4_automata_derived_record_roundtrip(
                    class,
                    &istream(&mut input),
                    &ostream(&mut Out::default()),
                    &host,
                ),
                0
            );
            assert_eq!(input.pos, 0);
        }
        assert_eq!(
            fc4_automata_derived_record_roundtrip(0x8BA9_8EDA, std::ptr::null(), std::ptr::null(), &host),
            1
        );
        let mut output = Out::default();
        let mut stream = ostream(&mut output);
        stream.put_u16 = Some(refuse);
        assert_eq!(
            fc4_automata_derived_record_roundtrip(0x8BA9_8EDA, &istream(&mut input), &stream, &host),
            1
        );
        assert!(output.bytes.is_empty());
        let mut bad_version = bytes.bytes.clone();
        bad_version[0] = 2;
        assert_eq!(
            fc4_automata_derived_record_roundtrip(
                0x8BA9_8EDA,
                &istream(&mut In::new(&bad_version)),
                &ostream(&mut Out::default()),
                &host,
            ),
            1
        );
    }
}

#[test]
fn at29_at33_derived_records_replay_and_balance_crew_and_stream_references() {
    use fc4_core::automata::*;
    use fc4_core::stream::{OutStream, RefValue};
    let vehicle = VehicleWrapperRecord::default();
    let mut ambulance = AmbulanceRecord::default();
    ambulance.emergency.v = [1.0, 2.0, 3.0];
    ambulance.emergency.b = 2;
    ambulance.emergency.w = 0x1234;
    ambulance.emergency.u = 3;
    let fire = FireTruckRecord {
        emergency: ambulance.emergency.clone(),
        crew: RefValue::Host {
            token: 37,
            class_id: 0x4A22_2E4D,
        },
    };
    let tank = TankRecord {
        saved_indices: [3, -1],
        ..TankRecord::default()
    };
    let train = MassTransitTrainRecord {
        cars: vec![RefValue::Host {
            token: 38,
            class_id: 0x4A41_6254,
        }],
        raw_2c0: 0x1122_3344,
        vector_2d4: [1.0, 2.0, 3.0],
        raw_2e4: 0x5566_7788,
        ..MassTransitTrainRecord::default()
    };
    let encode =
        |write: &dyn Fn(&mut dyn OutStream) -> bool| unsafe { write_with(|s| write(&mut crate::stream::COut(&*s))) };
    let car_bytes = encode(&|out| vehicle.write(out));
    for (class, bytes, added, released) in [
        (CLSID_CARJACKED, car_bytes.clone(), vec![], vec![]),
        (CLSID_LIMO, car_bytes, vec![], vec![]),
        (CLSID_AMBULANCE, encode(&|out| ambulance.write(out)), vec![], vec![]),
        (CLSID_FIRE_TRUCK, encode(&|out| fire.write(out)), vec![37], vec![37, 37]),
        (CLSID_TANK, encode(&|out| tank.write(out)), vec![], vec![]),
        (
            CLSID_MASS_TRANSIT_TRAIN,
            encode(&|out| train.write(out)),
            vec![],
            vec![38],
        ),
    ] {
        let log = RefCell::new(Log::default());
        let host = ref_host(&log);
        let mut input = In::new(&bytes);
        let mut output = Out::default();
        unsafe {
            assert_eq!(
                fc4_automata_derived_record_roundtrip(class, &istream(&mut input), &ostream(&mut output), &host),
                2,
                "{class:08X}"
            );
        }
        assert_eq!(input.pos, bytes.len());
        assert_eq!(output.bytes, bytes);
        assert_eq!(log.borrow().add_refs, added);
        assert_eq!(log.borrow().releases, released);
        // This fixture's error callback always returns zero. Derived tail helpers
        // retain inherited success; the shadow must also compare exhaustion/bytes.
        let mut input = In::new(&bytes[..bytes.len() - 1]);
        let mut output = Out::default();
        unsafe {
            assert_eq!(
                fc4_automata_derived_record_roundtrip(class, &istream(&mut input), &ostream(&mut output), &host),
                if matches!(class, CLSID_CARJACKED | CLSID_LIMO) {
                    1
                } else {
                    2
                }
            );
        }
        assert_ne!(output.bytes, bytes[..bytes.len() - 1]);
    }
}

#[test]
fn helicopter_shadow_fixture_is_independent_and_propagates_stream_failure() {
    unsafe extern "C" fn refuse(_: *mut c_void, _: u16) -> bool {
        false
    }
    unsafe {
        assert_eq!(fc4_helicopter_shadow_fixture(std::ptr::null()), 0);
        let mut bytes = Out::default();
        let mut stream = ostream(&mut bytes);
        assert_eq!(fc4_helicopter_shadow_fixture(&stream), 0x8BA9_8EDA);
        // Concrete helicopter, aircraft, automaton and occupant versions.
        assert_eq!(&bytes.bytes[..8], &[1, 0, 1, 0, 8, 0, 4, 0]);
        stream.put_u16 = Some(refuse);
        assert_eq!(fc4_helicopter_shadow_fixture(&stream), 0);
        stream.struct_size = 0;
        assert_eq!(fc4_helicopter_shadow_fixture(&stream), 0);
    }
}

#[test]
fn construction_shadow_keeps_crane_state_and_releases_partial_reads() {
    use fc4_core::lot_records::{construction::ConstructionRecord, OccupantRecord};
    use fc4_core::stream::RefValue;

    unsafe {
        let mut record = ConstructionRecord::new();
        record.has_crane = true;
        record.lot = RefValue::Host {
            token: 17,
            class_id: 0xC9BD_5D4A,
        };
        record.building = RefValue::Host {
            token: 18,
            class_id: BUILDING_CLASS,
        };
        record.scalars.state = 3;
        record.scalars.next_day = 1234;
        record.scalars.crane_target = 0.75;
        let bytes = write_with(|s| record.write(&mut crate::stream::COut(&*s)));
        let log = RefCell::new(Log::default());
        let host = ref_host(&log);
        let mut input = In::new(&bytes);
        let mut output = Out::default();
        assert!(fc4_automata_construction_roundtrip(
            &istream(&mut input),
            &ostream(&mut output),
            &host
        ));
        assert_eq!(input.pos, bytes.len());
        assert_eq!(output.bytes, bytes);
        assert_eq!(log.borrow().releases, [17, 18]);

        log.borrow_mut().releases.clear();
        let mut truncated = In::new(&bytes[..bytes.len() - 1]);
        // The shared fake stream normally reports no error even on failed
        // reads. CaptureIn (the actual shadow input) latches a stream error.
        unsafe extern "C" fn truncated_error(user: *mut c_void) -> i32 {
            let input = &*(user as *const In<'_>);
            i32::from(input.pos + 4 > input.bytes.len())
        }
        let mut input_stream = istream(&mut truncated);
        input_stream.error = Some(truncated_error);
        assert!(!fc4_automata_construction_roundtrip(
            &input_stream,
            &ostream(&mut Out::default()),
            &host
        ));
        assert_eq!(log.borrow().releases, [17, 18]);
    }
}

/// LR13 reference-helper rejection is independent of the stream error flag.
#[test]
fn lr13_t69_t70_rejected_helpers_continue_and_never_keep_rejected_tokens() {
    use fc4_core::lot_records::construction::CrewEntry;
    use fc4_core::lot_records::{construction::ConstructionRecord, OccupantRecord};
    use fc4_core::stream::RefValue;

    // A native helper can fail without setting the stream error. It has
    // already consumed and released its temporary Serializable (LR13).
    unsafe extern "C" fn rejected_reference(u: *mut c_void, iid: u32, out: *mut fc4_ref) -> bool {
        let input = &mut *(u as *mut In<'_>);
        let native = istream(input).get_ref.unwrap();
        if !native(u, iid, out) {
            return false;
        }
        if (*out).token == 99 {
            *out = fc4_ref::default();
            return false;
        }
        true
    }

    unsafe {
        for list_rejections in [false, true] {
            let mut record = ConstructionRecord::new();
            let bare = RefValue::Host {
                token: 99,
                class_id: 0xA9BC_9AB6,
            };
            record.current_effect = bare.clone();
            if list_rejections {
                record.effects = vec![bare.clone(), RefValue::Null];
                record.crew = vec![CrewEntry {
                    first: true,
                    second: false,
                    automaton: bare,
                }];
            }
            // A tail sentinel proves consumption continues past rejected refs.
            record.scalars.crane_type = 0xA5;
            let source = write_with(|s| record.write(&mut crate::stream::COut(&*s)));
            let mut input = In::new(&source);
            let mut stream = istream(&mut input);
            stream.get_ref = Some(rejected_reference);
            let log = RefCell::new(Log::default());
            let mut output = Out::default();
            assert!(fc4_automata_construction_roundtrip(
                &stream,
                &ostream(&mut output),
                &ref_host(&log)
            ));
            assert_eq!(input.pos, source.len());
            assert_ne!(output.bytes, source, "rejection must never be reported exact");
            assert!(log.borrow().releases.is_empty(), "rejected/null fields acquire nothing");
            record.current_effect = RefValue::Null;
            if list_rejections {
                record.effects = vec![RefValue::Null, RefValue::Null];
                record.crew[0].automaton = RefValue::Null;
            }
            let expected = write_with(|s| record.write(&mut crate::stream::COut(&*s)));
            assert_eq!(
                output.bytes, expected,
                "preserve list counts, nulls, bools, order and all tail scalars"
            );
        }
    }
}

/// What a fake host saw.
#[derive(Default)]
struct Log {
    lines: Vec<String>,
    add_refs: Vec<u64>,
    releases: Vec<u64>,
    flags: Vec<(u64, u32)>,
}

fn log_of(u: *mut c_void) -> &'static RefCell<Log> {
    unsafe { &*(u as *const RefCell<Log>) }
}

unsafe extern "C" fn h_log(u: *mut c_void, text: *const c_char) {
    let s = CStr::from_ptr(text).to_string_lossy().into_owned();
    log_of(u).borrow_mut().lines.push(s);
}
unsafe extern "C" fn h_add_ref(u: *mut c_void, t: u64) {
    log_of(u).borrow_mut().add_refs.push(t);
}
unsafe extern "C" fn h_release(u: *mut c_void, t: u64) {
    log_of(u).borrow_mut().releases.push(t);
}
unsafe extern "C" fn h_flags(u: *mut c_void, t: u64) -> u32 {
    log_of(u)
        .borrow()
        .flags
        .iter()
        .find(|(k, _)| *k == t)
        .map_or(0, |&(_, f)| f)
}

fn cm_host(log: &RefCell<Log>) -> fc4_automata_cm_host {
    fc4_automata_cm_host {
        struct_size: std::mem::size_of::<fc4_automata_cm_host>() as u32,
        user: log as *const RefCell<Log> as *mut c_void,
        log: Some(h_log),
        add_ref: Some(h_add_ref),
        release: Some(h_release),
        controller_flags: Some(h_flags),
    }
}

fn ref_host(log: &RefCell<Log>) -> fc4_automata_ref_host {
    fc4_automata_ref_host {
        struct_size: std::mem::size_of::<fc4_automata_ref_host>() as u32,
        user: log as *const RefCell<Log> as *mut c_void,
        add_ref: Some(h_add_ref),
        release: Some(h_release),
    }
}

fn host_ref(token: u64, class_id: u32) -> fc4_ref {
    fc4_ref {
        token,
        class_id,
        ..fc4_ref::default()
    }
}

fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

unsafe fn write_with(f: impl FnOnce(*const crate::stream::fc4_ostream) -> bool) -> Vec<u8> {
    let mut o = Out::default();
    let s = ostream(&mut o);
    assert!(f(&s));
    o.bytes
}

unsafe fn scripts_with(templates: &[(&str, u32)], groups: &[(u32, &str, &[&str])]) -> *mut fc4_automata_scripts {
    let s = fc4_automata_scripts_create();
    for (name, kind) in templates {
        assert!(fc4_automata_scripts_define_template(s, cs(name).as_ptr(), *kind, 7));
    }
    for (id, name, names) in groups {
        let owned: Vec<CString> = names.iter().map(|n| cs(n)).collect();
        let ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        assert!(fc4_automata_scripts_define_group(
            s,
            *id,
            cs(name).as_ptr(),
            ptrs.as_ptr(),
            ptrs.len()
        ));
    }
    s
}

// ---- component 40 ---------------------------------------------------------------------------------------

#[test]
fn script_system_record_is_version_1() {
    unsafe {
        let s = scripts_with(&[("dog_make", FC4_AUTOMATA_GENERATOR)], &[(7, "dogs", &["dog_make"])]);
        assert_eq!(fc4_automata_scripts_template_count(s), 1);
        assert_eq!(fc4_automata_scripts_group_count(s), 1);
        assert!(!fc4_automata_scripts_define_template(s, cs("x").as_ptr(), 9, 0));
        assert_eq!(write_with(|o| fc4_automata_scripts_write(s, o)), vec![1, 0]);
        let mut i = In::new(&[1, 0]);
        assert!(fc4_automata_scripts_read(s, &istream(&mut i)));
        let mut i = In::new(&[2, 0]);
        assert!(!fc4_automata_scripts_read(s, &istream(&mut i)));
        fc4_automata_scripts_destroy(s);
    }
}

// ---- objects --------------------------------------------------------------------------------------------

#[test]
fn objects_of_every_class_round_trip() {
    unsafe {
        for class in [GENERATOR, ATTRACTOR, PEDESTRIAN, VEHICLE, PATROL_CAR, TRAILER] {
            let o = fc4_automata_object_create(class);
            assert!(!o.is_null(), "{class:#x}");
            assert_eq!(fc4_automata_object_class(o), class);
            let controller = matches!(class, GENERATOR | ATTRACTOR);
            assert_eq!(fc4_automata_object_is_controller(o), controller);
            assert_eq!(fc4_automata_object_query_interface(o, 0x6A18_57F8), controller);
            assert_eq!(fc4_automata_object_query_interface(o, 0xA9B4_0F05), !controller);
            assert_eq!(fc4_automata_object_query_interface(o, 0x26BB_767E), !controller);
            let bytes = write_with(|s| fc4_automata_object_write(o, s));
            let p = fc4_automata_object_create(class);
            let mut i = In::new(&bytes);
            assert!(fc4_automata_object_read(p, &istream(&mut i), std::ptr::null()));
            assert_eq!(i.pos, bytes.len(), "{class:#x} parses to its end");
            assert_eq!(write_with(|s| fc4_automata_object_write(p, s)), bytes, "{class:#x}");
            // A cut record fails and changes nothing.
            let mut i = In::new(&bytes[..bytes.len() - 1]);
            assert!(!fc4_automata_object_read(p, &istream(&mut i), std::ptr::null()));
            assert_eq!(write_with(|s| fc4_automata_object_write(p, s)), bytes);
            fc4_automata_object_destroy(o);
            fc4_automata_object_destroy(p);
        }
        assert!(fc4_automata_object_create(0x1234).is_null());
        let e = fc4_automata_object_create_emergency();
        assert_eq!(fc4_automata_object_class(e), VEHICLE);
        let mut st = fc4_automata_automaton_state::default();
        assert!(fc4_automata_automaton_get_state(e, &mut st));
        assert_eq!(st.emergency, 1);
        fc4_automata_object_destroy(e);
    }
}

#[test]
fn automaton_slots_act_on_the_record() {
    unsafe {
        let a = fc4_automata_object_create(PEDESTRIAN);
        let c = fc4_automata_object_clone(a);
        assert!(fc4_automata_object_same(a, c));
        fc4_automata_object_set_token(a, 0x77);
        assert_eq!(fc4_automata_object_token(c), 0x77);
        // Not `final`, so this one can be popped away.
        assert!(fc4_automata_automaton_set_state(a, 8, 0, -1.0, 0, 0, 0, 0));
        assert!(fc4_automata_automaton_push_state(a, 0x10, 0, -1.0, 0, 0, 0, 0));
        fc4_automata_automaton_set_visible(a, true);
        fc4_automata_automaton_set_position(a, [1.0f32, 2.0, 3.0].as_ptr());
        let mut st = fc4_automata_automaton_state::default();
        assert!(fc4_automata_automaton_get_state(c, &mut st));
        assert_eq!((st.states, st.current_state, st.visible), (2, 0x10, 1));
        assert_eq!(st.position, [1.0, 2.0, 3.0]);
        assert!(fc4_automata_automaton_pop_state(a));
        assert!(fc4_automata_automaton_pop_state(a));
        assert!(!fc4_automata_automaton_pop_state(a));
        assert!(fc4_automata_automaton_get_state(c, &mut st));
        assert_eq!(st.current_state, FC4_AUTOMATA_NO_STATE);
        fc4_automata_automaton_remove(a);
        fc4_automata_automaton_return_to_source(a);
        assert!(fc4_automata_automaton_get_state(c, &mut st));
        assert_eq!((st.removed, st.returning), (1, 1));
        // Controller calls on an automaton do nothing.
        assert_eq!(fc4_automata_controller_flags(a), 0);
        fc4_automata_object_destroy(a);
        fc4_automata_object_destroy(c);
    }
}

/// §6.2 and §12 quirks 10-11 (§14 tests 26, 27, 29): `final` blocks `SetState`
/// (the riot end's `SetState(1, …)` is a no-op, §8.1), `PopState` on a `final`
/// state that is not already type 1 pushes DISAPPEAR instead of revealing the
/// state below, and the stack index is recomputed after every stack mutation.
#[test]
fn state_stack_final_and_index_rules() {
    unsafe {
        let a = fc4_automata_object_create(PEDESTRIAN);
        let mut st = fc4_automata_automaton_state::default();

        // §8.1 start step 5: SetState(8, 0, -1.0, **1**, 0, 0, 0) — final.
        assert!(fc4_automata_automaton_set_state(a, 8, 0, -1.0, 1, 0, 0, 0));
        assert!(fc4_automata_automaton_get_state(a, &mut st));
        assert_eq!((st.states, st.current_state), (1, 8));

        // §8.1 end step 1: SetState(1, 0, -1.0, 0, 0, 0, 0) returns 0 and
        // changes nothing — the leader keeps state 8.
        assert!(!fc4_automata_automaton_set_state(a, 1, 0, -1.0, 0, 0, 0, 0));
        assert!(fc4_automata_automaton_get_state(a, &mut st));
        assert_eq!((st.states, st.current_state), (1, 8));

        // PushState(0, 0, …) delegates to SetState and is refused the same way.
        assert!(!fc4_automata_automaton_push_state(a, 0, 0, -1.0, 0, 0, 0, 0));
        assert!(fc4_automata_automaton_get_state(a, &mut st));
        assert_eq!((st.states, st.current_state), (1, 8));

        // A push still works over a `final` top; only SetState is refused.
        // Collide pushed over it, not `final`: popping it does reveal the
        // RiotLeader.
        assert!(fc4_automata_automaton_push_state(a, 0x14, 0, 5000.0, 0, 0, 0, 0));
        assert!(fc4_automata_automaton_get_state(a, &mut st));
        assert_eq!((st.states, st.current_state), (2, 0x14));
        assert!(fc4_automata_automaton_pop_state(a));
        assert!(fc4_automata_automaton_get_state(a, &mut st));
        assert_eq!((st.states, st.current_state), (1, 8));

        // §14 test 27: a `final` Collide does not pop away. PopState pushes
        // DISAPPEAR (type 1) instead, so the stack grows and the Collide stays.
        assert!(fc4_automata_automaton_push_state(a, 0x14, 0, 5000.0, 1, 0, 0, 0));
        assert!(fc4_automata_automaton_pop_state(a));
        assert!(fc4_automata_automaton_get_state(a, &mut st));
        assert_eq!((st.states, st.current_state), (3, 1));
        // DISAPPEAR itself pops normally (it is type 1), back to the Collide.
        assert!(fc4_automata_automaton_pop_state(a));
        assert!(fc4_automata_automaton_get_state(a, &mut st));
        assert_eq!((st.states, st.current_state), (2, 0x14));

        // §14 test 29 (the stack index is 0-based from the bottom and is
        // recomputed after every mutation) needs the record's own fields; it
        // is checked in `fc4-core`'s automata tests.
        fc4_automata_object_destroy(a);
    }
}

// ---- component 41 ---------------------------------------------------------------------------------------

#[test]
fn controller_manager_slots_and_references() {
    let log = RefCell::new(Log::default());
    unsafe {
        let scripts = scripts_with(
            &[
                ("dog_make", FC4_AUTOMATA_GENERATOR),
                ("police_attract", FC4_AUTOMATA_ATTRACTOR),
            ],
            &[(9, "fire_occupant", &["police_attract"])],
        );
        let cm = fc4_automata_cm_create(&cm_host(&log));
        let occ = host_ref(0x100, BUILDING_CLASS);
        // Slot 5: a known template makes a controller; the occupant is listed once.
        let c = fc4_automata_cm_add_controller_from_template_name(cm, scripts, &occ, cs("dog_make").as_ptr(), 0);
        assert!(!c.is_null());
        assert_eq!(fc4_automata_object_class(c), GENERATOR);
        let mut name = [0 as c_char; 32];
        assert_eq!(fc4_automata_controller_template(c, name.as_mut_ptr(), name.len()), 8);
        assert_eq!(CStr::from_ptr(name.as_ptr()).to_str().unwrap(), "dog_make");
        let mut cst = fc4_automata_controller_state::default();
        assert!(fc4_automata_controller_get_state(c, &mut cst));
        assert_eq!((cst.template_bound, cst.occupant_token), (1, 0x100));
        // Unknown template, or no script system: null.
        let none = fc4_automata_cm_add_controller_from_template_name(cm, scripts, &occ, cs("nope").as_ptr(), 0);
        assert!(none.is_null());
        let none =
            fc4_automata_cm_add_controller_from_template_name(cm, std::ptr::null(), &occ, cs("dog_make").as_ptr(), 0);
        assert!(none.is_null());
        // Slots 9 / 10.
        assert!(fc4_automata_cm_add_controllers_from_group_name(
            cm,
            scripts,
            cs("fire_occupant").as_ptr(),
            &occ,
            0
        ));
        assert!(fc4_automata_cm_add_controllers_from_group_id(cm, scripts, 9, &occ, 0));
        assert!(!fc4_automata_cm_add_controllers_from_group_id(cm, scripts, 10, &occ, 0));
        assert!(!fc4_automata_cm_add_controllers_from_group_id(
            cm,
            std::ptr::null(),
            9,
            &occ,
            0
        ));
        assert_eq!(fc4_automata_cm_controller_count(cm), 3);
        assert_eq!(log.borrow().add_refs, vec![0x100]);
        // Slots 11 / 12 remove the group's controllers.
        assert!(fc4_automata_cm_remove_controllers_from_group_id(cm, scripts, 9, &occ));
        assert_eq!(fc4_automata_cm_controller_count(cm), 1);
        assert!(!fc4_automata_cm_remove_controllers_from_group_name(
            cm,
            scripts,
            cs("x").as_ptr(),
            &occ
        ));
        // Slot 14 removes the rest; the occupant goes with its last controller.
        assert_eq!(fc4_automata_cm_remove_controllers(cm, &occ), 1);
        assert!(fc4_automata_controller_get_state(c, &mut cst));
        assert_eq!(cst.shut_down, 1);
        assert_eq!(log.borrow().releases, vec![0x100]);
        assert_eq!(fc4_automata_cm_controller_count(cm), 0);
        fc4_automata_object_destroy(c);
        fc4_automata_cm_destroy(cm);
        fc4_automata_scripts_destroy(scripts);
    }
}

#[test]
fn controller_manager_times_clock_and_state() {
    let log = RefCell::new(Log::default());
    unsafe {
        let cm = fc4_automata_cm_create(&cm_host(&log));
        let a = fc4_automata_time {
            hour: 13.5,
            day: 3,
            reserved: 0,
        };
        let b = fc4_automata_time {
            hour: 14.0,
            day: 4,
            reserved: 0,
        };
        assert!(fc4_automata_cm_set_times(cm, &a, &b));
        let mut t = fc4_automata_time::default();
        assert!(fc4_automata_cm_controller_time(cm, &mut t));
        assert_eq!(t, b, "slot 24 returns time B");
        fc4_automata_cm_set_clock(cm, &host_ref(0x40, 0xA7E7_F929));
        let mut st = fc4_automata_cm_state::default();
        assert!(fc4_automata_cm_get_state(cm, &mut st));
        assert_eq!((st.loaded, st.clock_token, st.clock_class), (0, 0x40, 0xA7E7_F929));
        assert_eq!((st.time_a, st.time_b), (a, b));
        assert_eq!(log.borrow().add_refs, vec![0x40]);
        fc4_automata_cm_destroy(cm);
        assert_eq!(log.borrow().releases, vec![0x40]);
    }
}

#[test]
fn controller_manager_round_trip_flag_8_and_unknown_template() {
    let log = RefCell::new(Log::default());
    unsafe {
        let scripts = scripts_with(&[("dog_make", FC4_AUTOMATA_GENERATOR)], &[]);
        let cm = fc4_automata_cm_create(&cm_host(&log));
        let occ = host_ref(0x100, BUILDING_CLASS);
        let c1 = fc4_automata_cm_add_controller_from_template_name(cm, scripts, &occ, cs("dog_make").as_ptr(), 0);
        let c2 = fc4_automata_cm_add_controller_from_template_name(cm, scripts, &occ, cs("dog_make").as_ptr(), 0);
        fc4_automata_object_set_token(c1, 0x501);
        fc4_automata_object_set_token(c2, 0x502);
        let bytes = write_with(|o| fc4_automata_cm_write(cm, o));
        // u16, clock (test refs: u64 + u32), 2 × time, n1 = 1, occupant, m = 2.
        assert_eq!(&bytes[26..30], &1u32.to_le_bytes());
        assert_eq!(&bytes[42..46], &2u32.to_le_bytes());
        // Flag 8 (§3.2): left out.
        fc4_automata_controller_set_flags(c2, 8 | 7);
        let flagged = write_with(|o| fc4_automata_cm_write(cm, o));
        assert_eq!(&flagged[42..46], &1u32.to_le_bytes());
        fc4_automata_controller_set_flags(c1, 8);
        let none = write_with(|o| fc4_automata_cm_write(cm, o));
        assert_eq!(
            &none[26..30],
            &0u32.to_le_bytes(),
            "no occupant entry without saved controllers"
        );

        // Read the first record into a new manager: the references resolve to
        // fresh core controllers (as the game's loader would make them).
        let r1 = fc4_automata_object_create(GENERATOR);
        let r2 = fc4_automata_object_create(GENERATOR);
        // The host's wrappers give the objects it made their identities.
        fc4_automata_object_set_token(r1, 0x501);
        fc4_automata_object_set_token(r2, 0x502);
        let resolve = |r: &mut fc4_ref| {
            r.core_handle = match r.token {
                0x501 => r1 as *mut c_void,
                0x502 => r2 as *mut c_void,
                _ => std::ptr::null_mut(),
            }
        };
        // The controllers' own records (written separately by the game).
        for (src, dst) in [(c1, r1), (c2, r2)] {
            fc4_automata_controller_set_flags(src, 7);
            let rec = write_with(|o| fc4_automata_object_write(src, o));
            let mut i = In::new(&rec);
            assert!(fc4_automata_object_read(dst, &istream(&mut i), std::ptr::null()));
        }
        let cm2 = fc4_automata_cm_create(&cm_host(&log));
        let mut i = In::new(&bytes);
        i.resolve = Some(&resolve);
        assert!(fc4_automata_cm_read(cm2, &istream(&mut i)));
        assert_eq!(i.pos, bytes.len());
        assert!(fc4_automata_cm_is_loaded(cm2));
        assert_eq!(
            write_with(|o| fc4_automata_cm_write(cm2, o)),
            bytes,
            "byte-identical re-save"
        );
        let mut out = fc4_ref::default();
        let h = fc4_automata_cm_controller_at(cm2, 1, &mut out);
        assert!(fc4_automata_object_same(h, r2));
        assert_eq!((out.token, out.class_id), (0x502, GENERATOR));
        fc4_automata_object_destroy(h);
        assert!(fc4_automata_cm_controller_at(cm2, 2, std::ptr::null_mut()).is_null());

        // PostCityInit with scripts that lack the template drops both (§3.4).
        let empty = scripts_with(&[], &[]);
        assert_eq!(fc4_automata_cm_post_city_init(cm2, std::ptr::null()), -1);
        assert_eq!(fc4_automata_cm_post_city_init(cm2, empty), 2);
        assert_eq!(fc4_automata_cm_controller_count(cm2), 0);
        let msg = "Couldn't find automata controller template 'dog_make' in scripts. Removing controller.";
        assert_eq!(log.borrow().lines, vec![msg.to_owned(), msg.to_owned()]);
        // A known template binds them instead.
        let cm3 = fc4_automata_cm_create(&cm_host(&log));
        let mut i = In::new(&bytes);
        i.resolve = Some(&resolve);
        assert!(fc4_automata_cm_read(cm3, &istream(&mut i)));
        assert_eq!(fc4_automata_cm_post_city_init(cm3, scripts), 0);
        let mut cst = fc4_automata_controller_state::default();
        assert!(fc4_automata_controller_get_state(r1, &mut cst));
        assert_eq!((cst.template_bound, cst.initialized, cst.in_grid), (1, 1, 1));
        // A new manager (not loaded) skips the pass.
        let cm4 = fc4_automata_cm_create(std::ptr::null());
        assert_eq!(fc4_automata_cm_post_city_init(cm4, empty), 0);
        // A cut record fails.
        let mut i = In::new(&bytes[..bytes.len() - 2]);
        i.resolve = Some(&resolve);
        assert!(!fc4_automata_cm_read(cm4, &istream(&mut i)));
        assert!(!fc4_automata_cm_is_loaded(cm4));
        for x in [c1, c2, r1, r2] {
            fc4_automata_object_destroy(x);
        }
        for m in [cm, cm2, cm3, cm4] {
            fc4_automata_cm_destroy(m);
        }
        fc4_automata_scripts_destroy(scripts);
        fc4_automata_scripts_destroy(empty);
    }
}

// ---- components 43-46 -----------------------------------------------------------------------------------

#[test]
fn managers_of_each_class() {
    unsafe {
        for (class, version) in [
            (VEHICLE_MANAGER, 3),
            (PEDESTRIAN_MANAGER, 3),
            (AIRCRAFT_MANAGER, 5),
            (WATERCRAFT_MANAGER, 6),
        ] {
            let m = fc4_automata_manager_create(class, std::ptr::null());
            let mut st = fc4_automata_manager_state::default();
            assert!(fc4_automata_manager_get_state(m, &mut st));
            assert_eq!((st.class_id, st.version, st.base_version), (class, version, 6));
            let bytes = write_with(|o| fc4_automata_manager_write(m, o));
            let n = fc4_automata_manager_create(class, std::ptr::null());
            let mut i = In::new(&bytes);
            assert!(fc4_automata_manager_read(n, &istream(&mut i)));
            assert_eq!(i.pos, bytes.len());
            assert_eq!(write_with(|o| fc4_automata_manager_write(n, o)), bytes);
            assert!(fc4_automata_manager_get_state(n, &mut st));
            assert_eq!((st.loaded, st.old_version), (1, 0));
            fc4_automata_manager_destroy(m);
            fc4_automata_manager_destroy(n);
        }
        assert!(fc4_automata_manager_create(0x1234, std::ptr::null()).is_null());
    }
}

#[test]
fn version_2_traffic_record_round_trips() {
    // §14 test 4: the empty vehicle manager, 22 bytes.
    let bytes: Vec<u8> = vec![2, 0, 4, 0, 0x50, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0];
    unsafe {
        let m = fc4_automata_manager_create(VEHICLE_MANAGER, std::ptr::null());
        let mut i = In::new(&bytes);
        assert!(fc4_automata_manager_read(m, &istream(&mut i)));
        let mut st = fc4_automata_manager_state::default();
        assert!(fc4_automata_manager_get_state(m, &mut st));
        assert_eq!((st.version, st.base_version, st.max_count, st.active), (2, 4, 336, 0));
        assert_eq!(st.old_version, 1);
        assert_eq!(write_with(|o| fc4_automata_manager_write(m, o)), bytes);
        // Init applies the saved max count: hard limit trunc(336 × 1.25).
        assert!(fc4_automata_manager_init(m));
        assert!(fc4_automata_manager_get_state(m, &mut st));
        assert_eq!((st.initialized, st.hard_limit), (1, 420));
        fc4_automata_manager_destroy(m);
    }
}

#[test]
fn manager_slots_and_references() {
    let log = RefCell::new(Log::default());
    unsafe {
        let m = fc4_automata_manager_create(PEDESTRIAN_MANAGER, &ref_host(&log));
        assert_eq!(fc4_automata_manager_set_max_count(m, 4), 0);
        let mut st = fc4_automata_manager_state::default();
        assert!(fc4_automata_manager_get_state(m, &mut st));
        assert_eq!((st.max_count, st.hard_limit), (4, 5));
        let args = fc4_automata_create_args {
            group: 0x4107,
            position: [24.0, 0.0, 24.0],
            flags: 2,
            priority: 0xFF,
            ..Default::default()
        };
        let mut made = Vec::new();
        for _ in 0..4 {
            let a = fc4_automata_manager_create_automaton(m, &args);
            assert!(!a.is_null());
            made.push(a);
        }
        let mut ast = fc4_automata_automaton_state::default();
        assert!(fc4_automata_automaton_get_state(made[0], &mut ast));
        assert_eq!((ast.group, ast.position), (0x4107, [24.0, 0.0, 24.0]));
        assert_eq!(fc4_automata_object_class(made[0]), PEDESTRIAN);
        // A host automaton is listed with a reference; the fifth fills the hard limit.
        assert!(fc4_automata_manager_add_automaton(m, &host_ref(0x900, PEDESTRIAN)));
        assert!(!fc4_automata_manager_add_automaton(m, &host_ref(0x900, PEDESTRIAN)));
        assert!(!fc4_automata_manager_add_automaton(m, std::ptr::null()));
        assert_eq!(log.borrow().add_refs, vec![0x900]);
        assert!(
            fc4_automata_manager_create_automaton(m, &args).is_null(),
            "at the hard limit"
        );
        assert_eq!(fc4_automata_manager_automata_count(m), 5);
        let mut r = fc4_ref::default();
        assert!(fc4_automata_manager_automaton_at(m, 4, &mut r).is_null());
        assert_eq!(r.token, 0x900);
        let h = fc4_automata_manager_automaton_at(m, 0, &mut r);
        assert!(fc4_automata_object_same(h, made[0]));
        fc4_automata_object_destroy(h);
        // SetMaxCount(2) removes the newest three; the host one is released.
        assert_eq!(fc4_automata_manager_set_max_count(m, 2), 3);
        assert_eq!(log.borrow().releases, vec![0x900]);
        assert_eq!(fc4_automata_manager_set_max_count(m, 9999), 0);
        assert!(fc4_automata_manager_get_state(m, &mut st));
        assert_eq!((st.max_count, st.hard_limit), (5000, 6250));
        assert!(!fc4_automata_manager_remove_automaton(m, &host_ref(0, 0)));
        // Slot 13, 9-11, 5-6.
        assert!(fc4_automata_manager_is_driving_mode(m, 0x2B70_9676));
        assert!(!fc4_automata_manager_is_driving_mode(m, 0x2B70_9677));
        fc4_automata_manager_activate(m, true);
        assert!(fc4_automata_manager_is_active(m));
        let v = fc4_automata_manager_create(VEHICLE_MANAGER, std::ptr::null());
        let all = [m, v];
        fc4_automata_pause_all(all.as_ptr(), all.len());
        assert!(fc4_automata_manager_is_paused(m) && fc4_automata_manager_is_paused(v));
        fc4_automata_resume_all(all.as_ptr(), all.len());
        assert!(!fc4_automata_manager_is_paused(m) && !fc4_automata_manager_is_paused(v));
        // Slots 31 / 32.
        fc4_automata_manager_track_group(m, 0x1305, 2);
        fc4_automata_manager_track_group(m, 0x1305, -1);
        assert_eq!(fc4_automata_manager_instances_of_group(m, 0x1305), 1);
        assert_eq!(fc4_automata_manager_instances_of_group(m, 0x1306), 0);
        // Destroy releases what the record holds.
        assert!(fc4_automata_manager_add_automaton(m, &host_ref(0x901, PEDESTRIAN)));
        fc4_automata_manager_destroy(m);
        assert_eq!(log.borrow().releases, vec![0x900, 0x901]);
        fc4_automata_manager_destroy(v);
        for a in made {
            fc4_automata_object_destroy(a);
        }
    }
}

#[test]
fn manager_read_releases_the_replaced_and_the_failed() {
    let log = RefCell::new(Log::default());
    unsafe {
        let m = fc4_automata_manager_create(VEHICLE_MANAGER, &ref_host(&log));
        assert!(fc4_automata_manager_add_automaton(m, &host_ref(0x10, VEHICLE)));
        assert!(fc4_automata_manager_add_automaton(m, &host_ref(0x11, VEHICLE)));
        let bytes = write_with(|o| fc4_automata_manager_write(m, o));
        // A good read replaces the record: the old references go.
        let mut i = In::new(&bytes);
        assert!(fc4_automata_manager_read(m, &istream(&mut i)));
        assert_eq!(log.borrow().releases, vec![0x10, 0x11]);
        // A cut read keeps the record and gives back what it read.
        let mut i = In::new(&bytes[..bytes.len() - 1]);
        assert!(!fc4_automata_manager_read(m, &istream(&mut i)));
        assert_eq!(log.borrow().releases, vec![0x10, 0x11, 0x10, 0x11]);
        assert_eq!(fc4_automata_manager_automata_count(m), 2);
        // RemoveAutomaton releases.
        assert!(fc4_automata_manager_remove_automaton(m, &host_ref(0x11, VEHICLE)));
        assert_eq!(log.borrow().releases.last(), Some(&0x11));
        fc4_automata_manager_destroy(m);
        assert_eq!(log.borrow().releases, vec![0x10, 0x11, 0x10, 0x11, 0x11, 0x10]);
    }
}

#[test]
fn object_refs_lists_the_host_references() {
    let log = RefCell::new(Log::default());
    unsafe {
        let scripts = scripts_with(&[("dog_pull", FC4_AUTOMATA_ATTRACTOR)], &[]);
        let cm = fc4_automata_cm_create(&cm_host(&log));
        let c = fc4_automata_cm_add_controller_from_template_name(
            cm,
            scripts,
            &host_ref(0x300, BUILDING_CLASS),
            cs("dog_pull").as_ptr(),
            0,
        );
        let mut refs = [0u64; 4];
        assert_eq!(fc4_automata_object_refs(c, refs.as_mut_ptr(), refs.len()), 1);
        assert_eq!(refs[0], 0x300);
        // A failed read gives back the references it read.
        let rec = write_with(|o| fc4_automata_object_write(c, o));
        let d = fc4_automata_object_create(ATTRACTOR);
        let mut i = In::new(&rec[..rec.len() - 1]);
        assert!(!fc4_automata_object_read(d, &istream(&mut i), &ref_host(&log)));
        assert_eq!(log.borrow().releases, vec![0x300]);
        fc4_automata_object_destroy(c);
        fc4_automata_object_destroy(d);
        fc4_automata_cm_destroy(cm);
        fc4_automata_scripts_destroy(scripts);
    }
}

// AT24: independent counted interfaces and a hand-encoded v5 record. The
// segment owns one reference independently of each Read/QI result; disposal
// and optional external owners are controlled by the test, not the manager.
#[derive(Default)]
struct AircraftOwners {
    counts: std::collections::BTreeMap<u64, u32>,
    events: Vec<(u32, u64)>,
}

impl AircraftOwners {
    fn take(&mut self, token: u64) {
        *self.counts.entry(token).or_default() += 1;
    }
    fn release(&mut self, token: u64) {
        let count = self.counts.get_mut(&token).expect("known interface");
        assert!(*count > 0, "over-release {token:x}");
        *count -= 1;
        self.events.push((9, token));
    }
}

unsafe extern "C" fn aircraft_take(user: *mut c_void, token: u64) {
    (&*(user as *const RefCell<AircraftOwners>)).borrow_mut().take(token);
}
unsafe extern "C" fn aircraft_release(user: *mut c_void, token: u64) {
    (&*(user as *const RefCell<AircraftOwners>)).borrow_mut().release(token);
}
unsafe extern "C" fn aircraft_event(user: *mut c_void, event: u32, token: u64) {
    let mut owners = (&*(user as *const RefCell<AircraftOwners>)).borrow_mut();
    assert!(owners.counts[&token] > 0, "action on dead interface");
    owners.events.push((event, token));
}

fn aircraft_bytes(pairs: &[(u64, u64)], listed: &[u64]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&5u16.to_le_bytes()); // aircraft version
    bytes.extend_from_slice(&6u16.to_le_bytes()); // base version
    bytes.extend_from_slice(&20i32.to_le_bytes());
    bytes.extend_from_slice(&[1, 0]); // active, time scaling
    bytes.extend_from_slice(&(listed.len() as u32).to_le_bytes());
    for token in listed {
        bytes.extend_from_slice(&token.to_le_bytes());
        bytes.extend_from_slice(&0xCBEB_4F62u32.to_le_bytes());
    }
    bytes.extend_from_slice(&0u32.to_le_bytes()); // base group counts
    bytes.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
    for (airport, plane) in pairs {
        bytes.extend_from_slice(&airport.to_le_bytes());
        bytes.extend_from_slice(&BUILDING_CLASS.to_le_bytes());
        bytes.extend_from_slice(&plane.to_le_bytes());
        bytes.extend_from_slice(&0xCBEB_4F62u32.to_le_bytes());
    }
    bytes.extend_from_slice(&1u32.to_le_bytes()); // one runway spot
    bytes.extend_from_slice(&[0; 13]); // vec3 + u8
    bytes
}

unsafe fn loaded_aircraft(
    owners: &RefCell<AircraftOwners>,
    pairs: &[(u64, u64)],
    listed: &[u64],
) -> *mut fc4_automata_manager {
    let user = owners as *const _ as *mut c_void;
    let host = fc4_automata_ref_host {
        struct_size: std::mem::size_of::<fc4_automata_ref_host>() as u32,
        user,
        add_ref: Some(aircraft_take),
        release: Some(aircraft_release),
    };
    // Independent segment/table ownership: one per unique object.
    for token in pairs.iter().flat_map(|(a, p)| [*a, *p]).chain(listed.iter().copied()) {
        owners.borrow_mut().counts.entry(token).or_insert(1);
    }
    let acquire = |r: &mut fc4_ref| owners.borrow_mut().take(r.token);
    let bytes = aircraft_bytes(pairs, listed);
    let mut input = In::new(&bytes);
    input.resolve = Some(&acquire);
    let manager = fc4_automata_manager_create(AIRCRAFT_MANAGER, &host);
    assert!(fc4_automata_manager_read(manager, &istream(&mut input)));
    assert!(fc4_automata_manager_init(manager));
    manager
}

fn dispose_aircraft_table(owners: &RefCell<AircraftOwners>) {
    let tokens: Vec<_> = owners.borrow().counts.keys().copied().collect();
    for token in tokens {
        owners.borrow_mut().release(token);
    }
    owners.borrow_mut().events.clear();
}

unsafe fn aircraft_op(
    manager: *mut fc4_automata_manager,
    owners: &RefCell<AircraftOwners>,
    operation: u32,
    airport: u64,
) {
    assert!(fc4_automata_aircraft_lifecycle(
        manager,
        operation,
        &host_ref(airport, BUILDING_CLASS),
        Some(aircraft_event),
        owners as *const _ as *mut c_void
    ));
}

#[test]
fn at24_segment_disposal_preserves_two_independent_plane_owners() {
    let owners = RefCell::new(AircraftOwners::default());
    unsafe {
        let m = loaded_aircraft(&owners, &[(0x900, 0x500)], &[0x500]);
        assert_eq!(owners.borrow().counts[&0x500], 3);
        dispose_aircraft_table(&owners);
        assert_eq!(owners.borrow().counts[&0x500], 2);
        assert_eq!(owners.borrow().counts[&0x900], 1);
        aircraft_op(m, &owners, 1, 0);
        fc4_automata_manager_destroy(m);
        assert!(owners.borrow().counts.values().all(|&n| n == 0));
    }
}

#[test]
fn at24_pre_shutdown_clears_list_only_then_map_airport_plane() {
    let owners = RefCell::new(AircraftOwners::default());
    unsafe {
        let m = loaded_aircraft(&owners, &[(0x900, 0x500)], &[0x500]);
        dispose_aircraft_table(&owners);
        aircraft_op(m, &owners, 0, 0);
        assert_eq!(owners.borrow().events, [(1, 0x500), (5, 0x500), (9, 0x500)]);
        assert_eq!(owners.borrow().counts[&0x500], 1);
        assert_eq!(owners.borrow().counts[&0x900], 1);
        owners.borrow_mut().events.clear();
        aircraft_op(m, &owners, 1, 0);
        assert_eq!(owners.borrow().events, [(3, 0x900), (9, 0x900), (4, 0x500), (9, 0x500)]);
        fc4_automata_manager_destroy(m);
    }
}

#[test]
fn at24_shutdown_without_pre_is_map_first_then_plane_shutdown_and_list_release() {
    let owners = RefCell::new(AircraftOwners::default());
    unsafe {
        let m = loaded_aircraft(&owners, &[(0x900, 0x500)], &[0x500]);
        dispose_aircraft_table(&owners);
        aircraft_op(m, &owners, 1, 0);
        assert_eq!(
            owners.borrow().events,
            [
                (3, 0x900),
                (9, 0x900),
                (4, 0x500),
                (9, 0x500),
                (1, 0x500),
                (5, 0x500),
                (9, 0x500)
            ]
        );
        let bytes = write_with(|o| fc4_automata_manager_write(m, o));
        // Empty base/map and runway list after initialized Shutdown.
        assert_eq!(
            bytes,
            [5, 0, 6, 0, 20, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        fc4_automata_manager_destroy(m);
    }
}

#[test]
fn at24_external_owner_survives_both_shutdown_paths() {
    for pre in [false, true] {
        let owners = RefCell::new(AircraftOwners::default());
        unsafe {
            let m = loaded_aircraft(&owners, &[(0x900, 0x500)], &[0x500]);
            owners.borrow_mut().take(0x500);
            dispose_aircraft_table(&owners);
            if pre {
                aircraft_op(m, &owners, 0, 0);
            }
            aircraft_op(m, &owners, 1, 0);
            fc4_automata_manager_destroy(m);
            assert_eq!(owners.borrow().counts[&0x500], 1);
            owners.borrow_mut().release(0x500);
            assert_eq!(owners.borrow().counts[&0x500], 0);
        }
    }
}

#[test]
fn at24_reverse_keys_release_pairs_in_unsigned_pointer_order() {
    let owners = RefCell::new(AircraftOwners::default());
    unsafe {
        let m = loaded_aircraft(&owners, &[(0xF000_0900, 0x500), (0x100, 0x600)], &[]);
        // Keep table observers independently alive throughout traversal.
        aircraft_op(m, &owners, 1, 0);
        assert_eq!(
            owners.borrow().events,
            [
                (3, 0x100),
                (9, 0x100),
                (4, 0x600),
                (9, 0x600),
                (3, 0xF000_0900),
                (9, 0xF000_0900),
                (4, 0x500),
                (9, 0x500)
            ]
        );
        fc4_automata_manager_destroy(m);
        assert!(owners.borrow().counts.values().all(|&n| n == 1));
        dispose_aircraft_table(&owners);
    }
}

#[test]
fn at24_removal_deactivates_erases_releases_plane_airport_and_miss_is_empty() {
    let owners = RefCell::new(AircraftOwners::default());
    unsafe {
        let m = loaded_aircraft(&owners, &[(0x900, 0x500)], &[]);
        dispose_aircraft_table(&owners);
        aircraft_op(m, &owners, 2, 0x900);
        assert_eq!(
            owners.borrow().events,
            [(2, 0x500), (4, 0x500), (9, 0x500), (3, 0x900), (9, 0x900)]
        );
        owners.borrow_mut().events.clear();
        aircraft_op(m, &owners, 2, 0x900);
        aircraft_op(m, &owners, 1, 0);
        fc4_automata_manager_destroy(m);
        assert!(owners.borrow().events.is_empty());
    }
}

#[test]
fn at24_repeated_shutdown_and_storage_destruction_do_not_repeat_payload_pass() {
    for explicit in [false, true] {
        let owners = RefCell::new(AircraftOwners::default());
        unsafe {
            let m = loaded_aircraft(&owners, &[(0x900, 0x500)], &[0x500]);
            dispose_aircraft_table(&owners);
            if explicit {
                aircraft_op(m, &owners, 1, 0);
            }
            // Same entry used by plugin final Release, even after explicit Shutdown.
            aircraft_op(m, &owners, 1, 0);
            let events = owners.borrow().events.clone();
            aircraft_op(m, &owners, 1, 0);
            fc4_automata_manager_destroy(m);
            assert_eq!(owners.borrow().events, events);
            assert!(owners.borrow().counts.values().all(|&n| n == 0));
        }
    }
    let owners = RefCell::new(AircraftOwners::default());
    unsafe {
        let m = loaded_aircraft(&owners, &[(0x900, 0x500)], &[]);
        dispose_aircraft_table(&owners);
        fc4_automata_manager_destroy(m); // deleting storage is not public Release
        assert!(owners.borrow().events.is_empty());
        assert!(owners.borrow().counts.values().all(|&n| n == 1));
        // Explicitly balance the deliberately bypassed public lifecycle.
        owners.borrow_mut().release(0x900);
        owners.borrow_mut().release(0x500);
    }
}

// ---- hooks (§8) -----------------------------------------------------------------------------------------

#[derive(Default)]
struct City {
    calls: Vec<String>,
    road: bool,
    cap: bool,
    create_null: bool,
    next: u64,
}

fn city(u: *mut c_void) -> &'static RefCell<City> {
    unsafe { &*(u as *const RefCell<City>) }
}
fn call(u: *mut c_void, s: String) {
    city(u).borrow_mut().calls.push(s);
}

unsafe extern "C" fn c_road(u: *mut c_void, x: i32, z: i32, out: *mut f32) -> bool {
    call(u, format!("FindRoad {x} {z}"));
    if !city(u).borrow().road {
        return false;
    }
    *out = (16 * x + 8) as f32;
    *out.add(1) = 0.0;
    *out.add(2) = (16 * z + 8) as f32;
    true
}
unsafe extern "C" fn c_cap(u: *mut c_void, n: u32) -> bool {
    call(u, format!("LeaderCap {n}"));
    city(u).borrow().cap
}
unsafe extern "C" fn c_create(u: *mut c_void, a: *const fc4_automata_create_args) -> u64 {
    let a = &*a;
    call(
        u,
        format!(
            "CreateAutomaton {:#x} {:?} {:#x} {:?} callback={:#x} priority={:#x} orientation={:?}",
            a.group,
            a.position,
            a.flags,
            a.model_key,
            a.callback,
            a.priority,
            if a.has_orientation != 0 {
                Some(a.orientation)
            } else {
                None
            }
        ),
    );
    let mut c = city(u).borrow_mut();
    if c.create_null {
        return 0;
    }
    c.next += 1;
    0x500 + c.next - 1
}
unsafe extern "C" fn c_add_ref(u: *mut c_void, t: u64) {
    call(u, format!("AddRef {t:#x}"));
}
#[allow(clippy::too_many_arguments)]
unsafe extern "C" fn c_set_state(u: *mut c_void, t: u64, ty: u32, a: u32, b: f32, c: u32, d: u32, e: u32, f: u32) {
    call(u, format!("SetState {t:#x} {ty} {a} {b} {c} {d} {e} {f}"));
}
unsafe extern "C" fn c_visible(u: *mut c_void, t: u64, v: bool) {
    call(u, format!("SetVisible {t:#x} {v}"));
}
unsafe extern "C" fn c_as_occupant(_: *mut c_void, t: u64) -> u64 {
    t | 0x10000
}
unsafe extern "C" fn c_register(u: *mut c_void, t: u64, p: *const f32) {
    call(u, format!("RegisterLocation {t:#x} {}", *p));
}
unsafe extern "C" fn c_add_controller(u: *mut c_void, occ: u64, name: *const c_char, extra: u32) -> bool {
    let n = CStr::from_ptr(name).to_string_lossy();
    call(u, format!("AddControllerFromTemplateName {occ:#x} {n} {extra}"));
    true
}
unsafe extern "C" fn c_remove_controllers(u: *mut c_void, occ: u64) {
    call(u, format!("RemoveControllers {occ:#x}"));
}
unsafe extern "C" fn c_unregister(u: *mut c_void, occ: u64) {
    call(u, format!("UnregisterLocation {occ:#x}"));
}
unsafe extern "C" fn c_release(u: *mut c_void, t: u64) {
    call(u, format!("Release {t:#x}"));
}
unsafe extern "C" fn c_occupant_type(_: *mut c_void, occ: u64) -> u32 {
    if occ == 0x700 {
        BUILDING_TYPE
    } else {
        0x1234
    }
}
unsafe extern "C" fn c_add_group(u: *mut c_void, name: *const c_char, occ: u64, extra: u32) -> bool {
    let n = CStr::from_ptr(name).to_string_lossy();
    call(u, format!("AddControllersFromOccupantGroup {n} {occ:#x} {extra}"));
    true
}

fn riot_host(c: &RefCell<City>) -> fc4_automata_riot_host {
    fc4_automata_riot_host {
        struct_size: std::mem::size_of::<fc4_automata_riot_host>() as u32,
        user: c as *const RefCell<City> as *mut c_void,
        nearest_road_tile: Some(c_road),
        leader_cap_reached: Some(c_cap),
        create_automaton: Some(c_create),
        add_ref: Some(c_add_ref),
        set_state: Some(c_set_state),
        set_visible: Some(c_visible),
        as_occupant: Some(c_as_occupant),
        register_location: Some(c_register),
        add_controller_from_template_name: Some(c_add_controller),
        remove_controllers: Some(c_remove_controllers),
        unregister_location: Some(c_unregister),
        release: Some(c_release),
    }
}

fn take_calls(c: &RefCell<City>) -> Vec<String> {
    std::mem::take(&mut c.borrow_mut().calls)
}

#[test]
fn riot_leader_calls_in_order() {
    let c = RefCell::new(City {
        road: true,
        ..City::default()
    });
    unsafe {
        let r = fc4_automata_riot_create(&riot_host(&c));
        assert!(!r.is_null());
        assert_eq!(fc4_automata_riot_add(r, 3, 4), 0);
        assert_eq!(
            take_calls(&c),
            vec![
                "FindRoad 3 4",
                "LeaderCap 0",
                "CreateAutomaton 0x4107 [56.0, 0.0, 72.0] 0x2 [0, 0, 0] callback=0x0 priority=0xff orientation=None",
                "AddRef 0x500",
                "SetState 0x500 8 0 -1 1 0 0 0",
                "SetVisible 0x500 true",
                "RegisterLocation 0x500 56",
            ]
        );
        let mut l = fc4_automata_riot_leader::default();
        assert!(fc4_automata_riot_get_leader(r, 0, &mut l));
        assert_eq!((l.automaton, l.position), (0x500, [56.0, 0.0, 72.0]));
        assert!(fc4_automata_riot_set_stage(r, 0, 2));
        assert!(!fc4_automata_riot_set_stage(r, 0, 2));
        assert_eq!(
            take_calls(&c),
            vec!["AddControllerFromTemplateName 0x10500 riot_leader_2 0"]
        );
        assert!(fc4_automata_riot_end(r, 0));
        assert_eq!(
            take_calls(&c),
            vec![
                "SetState 0x500 1 0 -1 0 0 0 0",
                "RemoveControllers 0x10500",
                "UnregisterLocation 0x10500",
                "Release 0x500",
            ]
        );
        assert!(!fc4_automata_riot_end(r, 0));
        assert_eq!(fc4_automata_riot_count(r), 0);
        // No road, a reached cap and a null automaton stop the start.
        c.borrow_mut().road = false;
        assert_eq!(fc4_automata_riot_add(r, 1, 1), -1);
        c.borrow_mut().road = true;
        c.borrow_mut().cap = true;
        assert_eq!(fc4_automata_riot_add(r, 1, 1), -1);
        c.borrow_mut().cap = false;
        c.borrow_mut().create_null = true;
        assert_eq!(fc4_automata_riot_add(r, 1, 1), -1);
        assert_eq!(take_calls(&c).len(), 6);
        fc4_automata_riot_destroy(r);
        assert!(fc4_automata_riot_create(std::ptr::null()).is_null());
    }
}

#[test]
fn fire_occupant_calls() {
    let c = RefCell::new(City::default());
    let host = fc4_automata_fire_host {
        struct_size: std::mem::size_of::<fc4_automata_fire_host>() as u32,
        user: &c as *const RefCell<City> as *mut c_void,
        occupant_type: Some(c_occupant_type),
        add_controllers_from_occupant_group_name: Some(c_add_group),
        remove_controllers: Some(c_remove_controllers),
    };
    unsafe {
        let f = fc4_automata_fire_create(&host);
        assert!(fc4_automata_fire_set_target(f, 0x700));
        assert_eq!(fc4_automata_fire_target(f), 0x700);
        assert_eq!(
            take_calls(&c),
            vec!["AddControllersFromOccupantGroup fire_occupant 0x700 0"]
        );
        assert!(fc4_automata_fire_shutdown(f));
        assert_eq!(take_calls(&c), vec!["RemoveControllers 0x700"]);
        assert!(!fc4_automata_fire_shutdown(f));
        assert!(!fc4_automata_fire_set_target(f, 0x701), "not a building");
        assert!(take_calls(&c).is_empty());
        fc4_automata_fire_destroy(f);
        let mut short = host;
        short.struct_size = 8;
        assert!(fc4_automata_fire_create(&short).is_null());
    }
}
