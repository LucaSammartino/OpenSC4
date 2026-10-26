// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The conformance tests of §14 (1–11; 12, the corpus, is
//! `formats/tests/automata_records.rs`) and the version gates, the flag-8
//! rule, the load behaviour and the hooks.

use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue, ResKey, SaveObject};

#[test]
fn construction_setup_updates_only_flags_and_orientation() {
    let a = automaton::Automaton::for_class(CLSID_VEHICLE).unwrap();
    a.set_position([10.0, 20.0, 30.0]);
    a.set_automata_flag(0x900, true);
    a.set_automata_flag(0x100, false);
    a.set_automata_flag(0x40, true);
    a.set_orientation([3.0, 0.0, 4.0]);
    let rec = a.record();
    assert_eq!(rec.base().automata_flags, 0x840);
    assert_eq!(rec.base().vectors[0], [10.0, 20.0, 30.0]);
    assert_eq!(rec.base().vectors[1], [0.0; 3]);
    assert_eq!(rec.base().vectors[2], [0.6, 0.0, 0.8]);
    assert!(rec.base().states.is_empty());
    a.set_orientation([0.0; 3]);
    assert_eq!(a.record().base().vectors[2], [0.0; 3]);
}

fn hex(s: &str) -> Vec<u8> {
    let c: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    (0..c.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&c[i..i + 2], 16).unwrap())
        .collect()
}

fn hex_str(text: &str) -> String {
    text.bytes().map(|b| format!("{b:02X}")).collect()
}

/// References resolve to host tokens `class << 32 | id`.
struct Tokens;

impl RefResolver for Tokens {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: (u64::from(class_id) << 32) | u64::from(object_id),
            class_id,
        })
    }
}

fn host(class: u32, id: u32) -> RefValue {
    RefValue::Host {
        token: (u64::from(class) << 32) | u64::from(id),
        class_id: class,
    }
}

/// Writes host references as their object ID and class; core objects as the
/// host token they carry.
struct RawOut(ByteWriter);

impl OutStream for RawOut {
    fn put_u8(&mut self, v: u8) -> bool {
        self.0.put_u8(v)
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.0.put_u16(v)
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.0.put_u32(v)
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.0.put_u64(v)
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.0.put_f32(v)
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.0.put_f64(v)
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.0.put_bytes(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.0.put_str(v)
    }
    fn put_ref(&mut self, r: &RefValue) -> bool {
        match r {
            RefValue::Null => self.0.put_u32(0),
            RefValue::Host { token, class_id } => self.0.put_u32(*token as u32) && self.0.put_u32(*class_id),
            RefValue::Object(o) => self.0.put_u32(o.host_token() as u32) && self.0.put_u32(o.class_id()),
        }
    }
    fn error(&self) -> i32 {
        self.0.error()
    }
}

fn out() -> RawOut {
    RawOut(ByteWriter::new())
}

fn read_manager(kind: ManagerKind, bytes: &[u8]) -> (AutomataManager, bool, usize) {
    let mut m = AutomataManager::new(kind);
    let mut t = Tokens;
    let mut r = ByteReader::with_resolver(bytes, &mut t);
    let ok = m.read(&mut r);
    (m, ok, r.remaining())
}

fn write_manager(m: &AutomataManager) -> Vec<u8> {
    let mut o = out();
    assert!(m.write(&mut o));
    o.0.into_bytes()
}

fn read_cm(bytes: &[u8]) -> (ControllerManager, bool, usize) {
    let mut m = ControllerManager::new(Box::new(NoControllerHost));
    let mut t = Tokens;
    let mut r = ByteReader::with_resolver(bytes, &mut t);
    let ok = m.read(&mut r);
    (m, ok, r.remaining())
}

fn write_cm(m: &ControllerManager) -> Vec<u8> {
    let mut o = out();
    assert!(m.write(&mut o));
    o.0.into_bytes()
}

fn read_obj(obj: &dyn SaveObject, bytes: &[u8]) -> (bool, usize) {
    let mut t = Tokens;
    let mut r = ByteReader::with_resolver(bytes, &mut t);
    let ok = obj.read(&mut r);
    (ok, r.remaining())
}

fn write_obj(obj: &dyn SaveObject) -> Vec<u8> {
    let mut o = out();
    assert!(obj.write(&mut o));
    o.0.into_bytes()
}

// ---- §14 test 1: the script system ------------------------------------------------------------------------------

#[test]
fn t01_script_system() {
    let mut s = ScriptSystem::new();
    assert!(s.read(&mut ByteReader::new(&[1, 0])));
    let mut w = ByteWriter::new();
    assert!(s.write(&mut w));
    assert_eq!(w.bytes(), &[1, 0]);
    assert!(!ScriptSystem::new().read(&mut ByteReader::new(&[2, 0])));
    assert!(!ScriptSystem::new().read(&mut ByteReader::new(&[1])));
}

// ---- §14 tests 2 and 3: the controller manager ------------------------------------------------------------------

const EMPTY_CM: &str = "0100 1893010A29F9E7A7 000000000000 000000000000 00000000 00000000";

#[test]
fn t02_controller_manager_empty() {
    let bytes = hex(EMPTY_CM);
    assert_eq!(bytes.len(), 30);
    let (m, ok, left) = read_cm(&bytes);
    assert!(ok && left == 0);
    assert!(m.is_loaded());
    let r = m.record();
    assert_eq!(r.version, 1);
    assert!(refs_equal(&r.clock, &host(0xA7E7_F929, 0x0A01_9318)));
    assert_eq!(r.time_a, ControllerTime { hour: 0.0, day: 0 });
    assert_eq!(r.time_b, ControllerTime { hour: 0.0, day: 0 });
    assert!(r.entries.is_empty() && r.sets.is_empty());
    assert_eq!(write_cm(&m), bytes);
    // A new manager has not been loaded.
    assert!(!ControllerManager::new(Box::new(NoControllerHost)).is_loaded());
}

#[test]
fn t03_controller_manager_times() {
    let bytes = hex("0100 1893010A29F9E7A7 77775F410300 77775F410300 00000000 00000000");
    let (m, ok, left) = read_cm(&bytes);
    assert!(ok && left == 0);
    let t = m.controller_time();
    assert_eq!(t.day, 3);
    assert!((t.hour - 13.9667).abs() < 1e-4, "{}", t.hour);
    assert_eq!(m.record().time_a, t);
    assert_eq!(write_cm(&m), bytes);
}

// ---- §14 test 4: the vehicle and pedestrian managers -----------------------------------------------------------

#[test]
fn t04_traffic_managers_empty() {
    let v = hex("0200 0400 50010000 00 00 00000000 01000000 01000000");
    assert_eq!(v.len(), 22);
    let (m, ok, left) = read_manager(ManagerKind::Vehicle, &v);
    assert!(ok && left == 0);
    let r = m.record();
    assert_eq!(r.base.version, 4);
    assert_eq!(r.base.max_count, 336);
    assert!(!r.base.active && !r.base.time_scaling);
    assert!(r.base.automata.is_empty() && r.base.group_counts.is_empty());
    let ManagerPart::Traffic(t) = &r.part else { panic!() };
    assert_eq!((t.version, t.tag, t.cursor), (2, 1, 1));
    assert!(m.is_loaded() && m.is_old_version());
    assert_eq!(write_manager(&m), v);

    let p = hex("0200 0400 80010000 00 00 00000000 02000000 01000000");
    let (m, ok, left) = read_manager(ManagerKind::Pedestrian, &p);
    assert!(ok && left == 0);
    assert_eq!(m.max_count(), 384);
    let ManagerPart::Traffic(t) = &m.record().part else {
        panic!()
    };
    assert_eq!(t.tag, 2);
    assert_eq!(write_manager(&m), p);
}

#[test]
fn traffic_manager_version_gates_and_incidents() {
    // Read accepts 2 or 3.
    for v in [1u8, 4] {
        let b = hex(&format!("{v:02X}00 0400 50010000 00 00 00000000 01000000 01000000"));
        assert!(!read_manager(ManagerKind::Vehicle, &b).1);
    }
    // A base version above 6 fails.
    let b = hex("0300 0700 50010000 00 00 00000000 01000000 01000000 0100 00000000");
    assert!(!read_manager(ManagerKind::Vehicle, &b).1);
    // Version 3 with base 6, one automaton, a group count and one incident.
    let inc = "2F000000 16000000 02000000 0100000002000000030000000400000005000000060000000700000008000000 \
               05000000 6E5CC0C9 01 78180000";
    let b = hex(&format!(
        "0300 0600 50010000 01 01 01000000 07000000 AF756E89 01000000 05410000 03000000 \
         01000000 07000000 0100 01000000 {inc}"
    ));
    let (m, ok, left) = read_manager(ManagerKind::Vehicle, &b);
    assert!(ok && left == 0);
    assert!(!m.is_old_version());
    assert_eq!(m.automata_count(), 1);
    assert_eq!(m.instances_of_group(0x4105), 3);
    let ManagerPart::Traffic(t) = &m.record().part else {
        panic!()
    };
    assert_eq!(t.incidents.len(), 1);
    let i = &t.incidents[0];
    assert_eq!((i.x, i.z, i.kind, i.flag, i.last), (47, 22, 2, true, 6264));
    assert_eq!(class_id_of(&i.network), Some(0xC9C0_5C6E));
    assert_eq!(write_manager(&m), b);
    // A cut record fails and changes nothing.
    let mut m2 = AutomataManager::new(ManagerKind::Vehicle);
    let mut t = Tokens;
    let mut r = ByteReader::with_resolver(&b[..b.len() - 2], &mut t);
    assert!(!m2.read(&mut r));
    assert!(!m2.is_loaded());
}

fn class_id_of(r: &RefValue) -> Option<u32> {
    match r {
        RefValue::Host { class_id, .. } => Some(*class_id),
        RefValue::Object(o) => Some(o.class_id()),
        RefValue::Null => None,
    }
}

#[test]
fn old_traffic_record_rises_to_version_3_when_it_gains_incidents() {
    let v = hex("0200 0400 50010000 00 00 00000000 01000000 01000000");
    let (mut m, _, _) = read_manager(ManagerKind::Vehicle, &v);
    if let ManagerPart::Traffic(t) = &mut m.record_mut().part {
        t.incidents.push(Incident {
            x: 1,
            z: 2,
            kind: 3,
            values: [0; 8],
            network: RefValue::Null,
            flag: false,
            last: 0,
        });
    }
    let b = write_manager(&m);
    assert_eq!(&b[..2], &[3, 0]);
    let (m2, ok, left) = read_manager(ManagerKind::Vehicle, &b);
    assert!(ok && left == 0);
    assert_eq!(write_manager(&m2), b);
    // A group count raises an old base to 6.
    let (mut m, _, _) = read_manager(ManagerKind::Vehicle, &v);
    m.track_group_instances(0x10, 2);
    let b = write_manager(&m);
    assert_eq!(&b[2..4], &[6, 0]);
    // A new manager writes the current versions.
    let b = write_manager(&AutomataManager::new(ManagerKind::Pedestrian));
    assert_eq!(&b[..4], &[3, 0, 6, 0]);
    assert_eq!(b.len(), 2 + 16 + 8 + 6);
}

// ---- §14 test 5: the attractor ----------------------------------------------------------------------------------

#[test]
fn t05_attractor() {
    let bytes = hex(&format!(
        "0500 2C2189172D88BDA9 0E000000 {} {} {} 00000000 07000000 00000000 0100 02000000 \
         00000000 00007042 00004842 00007042",
        hex_str("police_attract"),
        "00".repeat(12),
        "00".repeat(8)
    ));
    assert_eq!(bytes.len(), 82);
    let c = Controller::for_class(CLSID_ATTRACTOR).unwrap();
    let (ok, left) = read_obj(&*c, &bytes);
    assert!(ok && left == 0);
    let r = c.record();
    assert_eq!(r.base.version, 5);
    assert!(refs_equal(&r.base.occupant, &host(0xA9BD_882D, 0x1789_212C)));
    assert_eq!(c.template_name(), "police_attract");
    assert_eq!(r.base.flags, 7);
    assert!(r.base.automata.is_empty());
    assert_eq!(
        r.kind,
        ControllerKind::Attractor {
            tag: 1,
            curve: vec![(0.0, 60.0), (50.0, 60.0)]
        }
    );
    assert!(!c.template_bound());
    assert_eq!(write_obj(&*c), bytes);
}

// ---- §14 test 6: the generator ----------------------------------------------------------------------------------

const DOG_MAKE: &str = "0400 0500 546EBD0947AA7729 08000000 646F675F6D616B65 \
    0000000000000000000000000000000000000000 00000000 87000000 00000000 \
    000020410000 ABAA2A3E0000 22410000 00000000";

#[test]
fn t06_generator() {
    let bytes = hex(DOG_MAKE);
    assert_eq!(bytes.len(), 76);
    let c = Controller::for_class(CLSID_GENERATOR).unwrap();
    let (ok, left) = read_obj(&*c, &bytes);
    assert!(ok && left == 0);
    let r = c.record();
    assert!(refs_equal(&r.base.occupant, &host(0x2977_AA47, 0x09BD_6E54)));
    assert_eq!(c.template_name(), "dog_make");
    assert_eq!(r.base.flags, 0x87);
    let ControllerKind::Generator {
        period,
        second_period,
        day_stamp,
        counter,
    } = r.kind
    else {
        panic!()
    };
    assert_eq!(period, ControllerTime { hour: 10.0, day: 0 });
    assert!((second_period.hour - 0.16667).abs() < 1e-5 && second_period.day == 0);
    assert_eq!((day_stamp, counter), (16674, 0));
    assert_eq!(write_obj(&*c), bytes);
    // Read requires generator version 4.
    let mut bad = bytes.clone();
    bad[0] = 3;
    let c2 = Controller::for_class(CLSID_GENERATOR).unwrap();
    assert!(!read_obj(&*c2, &bad).0);
    assert_eq!(c2.record(), ControllerRecord::for_class(CLSID_GENERATOR).unwrap());
    // The base version is ignored by Read and kept.
    let mut v9 = bytes.clone();
    v9[2] = 9;
    let c3 = Controller::for_class(CLSID_GENERATOR).unwrap();
    assert!(read_obj(&*c3, &v9).0);
    assert_eq!(write_obj(&*c3), v9);
}

// ---- §14 test 7: the watercraft manager -------------------------------------------------------------------------

const WATERCRAFT_V6: &str = "0600 0600 14000000 01 00 00000000 0C000000 \
    05130000 00000000 06130000 00000000 07130000 00000000 08130000 00000000 09130000 00000000 \
    39190000 00000000 41190000 00000000 10310100 00000000 20310100 01000000 30310100 02000000 \
    0A150000 00000000 0F150000 00000000 00000000 00000000 00000000";

#[test]
fn t07_watercraft_v6() {
    let bytes = hex(WATERCRAFT_V6);
    assert_eq!(bytes.len(), 126);
    let (m, ok, left) = read_manager(ManagerKind::Watercraft, &bytes);
    assert!(ok && left == 0);
    let r = m.record();
    assert_eq!((r.version, r.base.version, r.base.max_count), (6, 6, 20));
    assert!(r.base.active && !r.base.time_scaling && r.base.automata.is_empty());
    assert_eq!(r.base.group_counts.len(), 12);
    assert_eq!(r.base.group_counts[0], (0x1305, 0));
    assert_eq!(r.base.group_counts[7], (0x13110, 0));
    assert_eq!(r.base.group_counts[8], (0x13120, 1));
    assert_eq!(r.base.group_counts[9], (0x13130, 2));
    assert_eq!(r.base.group_counts[11], (0x150F, 0));
    let ManagerPart::Watercraft {
        lots,
        occupants,
        ferries,
    } = &r.part
    else {
        panic!()
    };
    assert!(lots.is_empty() && occupants.is_empty() && ferries.is_empty());
    assert_eq!(write_manager(&m), bytes);
}

#[test]
fn aircraft_and_watercraft_version_gates() {
    // Aircraft above 5 fails; below 5 the record is a bare base of that version.
    assert!(
        !read_manager(
            ManagerKind::Aircraft,
            &hex("0600 0600 00000000 00 00 00000000 00000000")
        )
        .1
    );
    let v4 = hex("0400 64000000 01 00 00000000");
    assert_eq!(v4.len(), 12);
    let (m, ok, left) = read_manager(ManagerKind::Aircraft, &v4);
    assert!(ok && left == 0);
    assert_eq!(
        (m.record().version, m.record().base.version, m.max_count()),
        (4, 4, 100)
    );
    assert_eq!(write_manager(&m), v4);
    // Watercraft version 5 is a bare version-5 base (with its group map).
    let v5 = hex("0500 14000000 01 00 00000000 01000000 05130000 02000000");
    let (m, ok, left) = read_manager(ManagerKind::Watercraft, &v5);
    assert!(ok && left == 0);
    assert_eq!(m.instances_of_group(0x1305), 2);
    assert_eq!(write_manager(&m), v5);
    assert!(!read_manager(ManagerKind::Watercraft, &hex("0700")).1);
}

#[test]
fn aircraft_record_with_pairs_and_spots() {
    let b = hex("0500 0500 32000000 01 01 00000000 00000000 \
         01000000 2D000000 2D88BDA9 31000000 AF756E89 \
         02000000 0000803F 00000040 00004040 07 00000000 00000000 00000000 00");
    let (mut m, ok, left) = read_manager(ManagerKind::Aircraft, &b);
    assert!(ok && left == 0);
    let ManagerPart::Aircraft { pairs, spots } = &m.record().part else {
        panic!()
    };
    assert_eq!(pairs.len(), 1);
    assert_eq!(spots[0], ([1.0, 2.0, 3.0], 7));
    assert_eq!(write_manager(&m), b);
    // The occupant leaving the city takes its pair with it (§10).
    m.occupant_removed(&host(0xA9BD_882D, 0x2D));
    let ManagerPart::Aircraft { pairs, .. } = &m.record().part else {
        panic!()
    };
    assert!(pairs.is_empty());
}

#[test]
fn watercraft_ferry_entries_round_trip() {
    let route = "03000000 4A5DBDC9 04000000 4A5DBDC9 01000000 0100 0200 0300 0400 01 FEFF 0500 \
                 02000000 AABB 00000000";
    let b = hex(&format!(
        "0600 0600 14000000 01 00 00000000 00000000 \
         01000000 03000000 4A5DBDC9 09000000 2D88BDA9 \
         01000000 09000000 2D88BDA9 \
         01000000 03000000 4A5DBDC9 09000000 2D88BDA9 01000000 {route}"
    ));
    let (mut m, ok, left) = read_manager(ManagerKind::Watercraft, &b);
    assert!(ok && left == 0);
    let ManagerPart::Watercraft { ferries, .. } = &m.record().part else {
        panic!()
    };
    let e = &ferries[0].route[0];
    assert_eq!((e.kind, e.words, e.flag, e.a, e.b), (1, [1, 2, 3, 4], true, -2, 5));
    assert_eq!((e.bytes1.clone(), e.bytes2.len()), (vec![0xAA, 0xBB], 0));
    assert_eq!(write_manager(&m), b);
    m.occupant_removed(&host(0xA9BD_882D, 9));
    let ManagerPart::Watercraft {
        lots,
        occupants,
        ferries,
    } = &m.record().part
    else {
        panic!()
    };
    assert!(lots.is_empty() && occupants.is_empty() && ferries.is_empty());
}

// ---- §14 test 10: flag 8 -----------------------------------------------------------------------------------------

#[test]
fn t10_flag_8() {
    let mut cm = ControllerManager::new(Box::new(NoControllerHost));
    let mut scripts = ScriptSystem::new();
    scripts.define_template(ControllerTemplate::new("keep", TemplateKind::Generator));
    let mut transient = ControllerTemplate::new("transient", TemplateKind::Attractor);
    transient.flags = 7 | FLAG_TRANSIENT;
    scripts.define_template(transient);
    let occ = host(0xA9BD_882D, 0x40);
    let kept = cm.add_controller_from_template_name(&scripts, &occ, "keep", 0).unwrap();
    let gone = cm
        .add_controller_from_template_name(&scripts, &occ, "transient", 0)
        .unwrap();
    assert!(gone.is_transient() && !kept.is_transient());
    kept.set_host_token(0x77);
    let b = write_cm(&cm);
    // version, clock (null), two times, n1 = 1, the occupant, m = 1, one controller, n2 = 0.
    let want =
        hex("0100 00000000 000000000000 000000000000 01000000 40000000 2D88BDA9 01000000 77000000 C528164A 00000000");
    assert_eq!(b, want);
    // An occupant whose controllers are all transient is left out.
    let occ2 = host(0xA9BD_882D, 0x41);
    cm.add_controller_from_template_name(&scripts, &occ2, "transient", 0)
        .unwrap();
    assert_eq!(write_cm(&cm), want);

    // An automaton whose controller has flag 8 writes a null controller ref;
    // a state's controller ref too.
    let a = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
    a.set_controller(RefValue::Object(gone.clone()));
    a.push_state(state_type::T14, SetStateArgs::default());
    a.with_record(|r| r.base_mut().states[0].base.controller = RefValue::Object(gone.clone()));
    let with_flag = write_obj(&*a);
    a.set_controller(RefValue::Null);
    a.with_record(|r| r.base_mut().states[0].base.controller = RefValue::Null);
    assert_eq!(with_flag, write_obj(&*a));
    a.set_controller(RefValue::Object(kept.clone()));
    assert_ne!(with_flag, write_obj(&*a));
}

// ---- §14 test 11 and §3.4: unknown templates ---------------------------------------------------------------------

#[derive(Default)]
struct LogHost {
    log: Rc<RefCell<Vec<String>>>,
}

impl ControllerManagerHost for LogHost {
    fn log(&self, text: &str) {
        self.log.borrow_mut().push(format!("log {text}"));
    }
    fn release(&self, token: u64) {
        self.log.borrow_mut().push(format!("release {token:#x}"));
    }
    fn add_ref(&self, token: u64) {
        self.log.borrow_mut().push(format!("add_ref {token:#x}"));
    }
}

/// A resolver that builds core controllers for the controller classes.
#[derive(Default)]
struct Objects {
    made: Vec<(u32, Rc<Controller>)>,
}

impl RefResolver for Objects {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        if let Some(c) = Controller::for_class(class_id) {
            c.set_host_token(u64::from(object_id));
            self.made.push((object_id, c.clone()));
            return Some(RefValue::Object(c));
        }
        Tokens.resolve(class_id, object_id)
    }
}

#[test]
fn t11_unknown_template_dropped_at_post_city_init() {
    // Two occupants: 0x40 with controllers 1 (dog_make) and 2 (gone_name), 0x41 with 3 (gone_name).
    let b = hex("0100 00000000 000000000000 000000000000 02000000 \
         40000000 2D88BDA9 02000000 01000000 C528164A 02000000 BA28166A \
         41000000 2D88BDA9 01000000 03000000 C528164A 00000000");
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut cm = ControllerManager::new(Box::new(LogHost { log: log.clone() }));
    let mut objs = Objects::default();
    let mut r = ByteReader::with_resolver(&b, &mut objs);
    assert!(cm.read(&mut r));
    assert_eq!(r.remaining(), 0);
    // The controllers' own records (loaded from their class records).
    let dog = Controller::for_class(CLSID_GENERATOR).unwrap();
    assert!(dog.read(&mut ByteReader::with_resolver(&hex(DOG_MAKE), &mut Tokens)));
    for (id, c) in &objs.made {
        let name = if *id == 1 { "dog_make" } else { "gone_name" };
        c.with_record(|r| r.base.template_name = name.as_bytes().to_vec());
    }
    let mut scripts = ScriptSystem::new();
    scripts.define_template(ControllerTemplate::new("dog_make", TemplateKind::Generator));
    let dropped = cm.post_city_init(&scripts);
    assert_eq!(dropped, vec!["gone_name".to_string(), "gone_name".to_string()]);
    let all = cm.all_controllers();
    assert_eq!(all.len(), 1);
    let kept = &objs.made[0].1;
    assert!(kept.template_bound() && kept.is_initialized() && kept.in_grid());
    assert!(objs.made[1].1.is_shut_down() && objs.made[2].1.is_shut_down());
    // The occupant left without controllers is dropped and released.
    assert!(cm.controllers_of(&host(0xA9BD_882D, 0x41)).is_empty());
    let l = log.borrow();
    assert_eq!(l.iter().filter(|x| x.starts_with("log ")).count(), 2, "{l:?}");
    assert!(l.contains(&format!("log {}", missing_template_message("gone_name"))));
    assert!(l.contains(&"release 0xa9bd882d00000041".to_string()), "{l:?}");
}

#[test]
fn post_city_init_does_nothing_for_a_new_city() {
    let mut cm = ControllerManager::new(Box::new(NoControllerHost));
    let occ = host(0xA9BD_882D, 0x40);
    let mut scripts = ScriptSystem::new();
    scripts.define_template(ControllerTemplate::new("a", TemplateKind::Generator));
    let c = cm.add_controller_from_template_name(&scripts, &occ, "a", 0).unwrap();
    // The template disappears; a city that was not loaded skips the pass (§12 item 7).
    let empty = ScriptSystem::new();
    assert!(cm.post_city_init(&empty).is_empty());
    assert_eq!(cm.all_controllers().len(), 1);
    assert!(c.in_grid());
}

#[test]
fn duplicate_occupant_entries_are_read_then_dropped() {
    let b = hex("0100 00000000 000000000000 000000000000 02000000 \
         40000000 2D88BDA9 01000000 01000000 C528164A \
         40000000 2D88BDA9 01000000 02000000 C528164A 00000000");
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut cm = ControllerManager::new(Box::new(LogHost { log: log.clone() }));
    let mut t = Tokens;
    let mut r = ByteReader::with_resolver(&b, &mut t);
    assert!(cm.read(&mut r) && r.remaining() == 0);
    assert_eq!(cm.record().entries.len(), 1);
    assert_eq!(cm.all_controllers().len(), 1);
    let l = log.borrow();
    assert!(l.contains(&"release 0xa9bd882d00000040".to_string()));
    assert!(l.contains(&"release 0x4a1628c500000002".to_string()));
}

#[test]
fn a_failed_controller_manager_read_gives_back_its_references() {
    let b = hex("0100 1893010A29F9E7A7 000000000000 000000000000 01000000 40000000 2D88BDA9 02000000");
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut cm = ControllerManager::new(Box::new(LogHost { log: log.clone() }));
    let mut t = Tokens;
    assert!(!cm.read(&mut ByteReader::with_resolver(&b, &mut t)));
    assert!(!cm.is_loaded());
    assert_eq!(log.borrow().len(), 2);
}

#[test]
fn automaton_sets_round_trip_and_leave_with_their_occupant() {
    // The Big City Tutorial entry: a building, k = 0, (1256, 0, 1041), (−1, 0, 0), true.
    let b = hex("0100 00000000 000000000000 000000000000 00000000 01000000 \
         40000000 2D88BDA9 00000000 00009D44 00000000 00208244 000080BF 00000000 00000000 01");
    let (mut cm, ok, left) = read_cm(&b);
    assert!(ok && left == 0);
    let s = &cm.record().sets[0];
    assert_eq!((s.v1, s.v2, s.flag), ([1256.0, 0.0, 1041.0], [-1.0, 0.0, 0.0], true));
    assert_eq!(write_cm(&cm), b);
    cm.occupant_removed(&host(0xA9BD_882D, 0x40));
    assert!(cm.record().sets.is_empty());
}

// ---- §7.1: the controller manager's calls ------------------------------------------------------------------------

#[test]
fn controller_manager_group_calls() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut cm = ControllerManager::new(Box::new(LogHost { log: log.clone() }));
    let mut s = ScriptSystem::new();
    s.define_template(ControllerTemplate::new("fire_crowd", TemplateKind::Generator));
    s.define_template(ControllerTemplate::new("fire_pull", TemplateKind::Attractor));
    s.define_template(ControllerTemplate::new("other", TemplateKind::Generator));
    s.define_occupant_group(OccupantGroup {
        id: 0x99,
        name: "fire_occupant".into(),
        templates: vec!["fire_crowd".into(), "fire_pull".into()],
    });
    let occ = host(0xA9BD_882D, 0x40);
    // No script system or no group: false.
    assert!(!cm.add_controllers_from_occupant_group_name(None, "fire_occupant", &occ, 0));
    assert!(!cm.add_controllers_from_occupant_group_name(Some(&s), "nope", &occ, 0));
    assert!(!cm.add_controllers_from_occupant_group_id(Some(&s), 0x98, &occ, 0));
    assert!(cm.add_controllers_from_occupant_group_name(Some(&s), "fire_occupant", &occ, 0));
    assert!(cm.add_controller_from_template_name(&s, &occ, "missing", 0).is_none());
    cm.add_controller_from_template_name(&s, &occ, "other", 0).unwrap();
    assert_eq!(cm.controllers_of(&occ).len(), 3);
    assert_eq!(log.borrow().as_slice(), ["add_ref 0xa9bd882d00000040"]);
    let classes: Vec<u32> = cm.controllers_of(&occ).iter().filter_map(class_id_of).collect();
    assert_eq!(classes, vec![CLSID_GENERATOR, CLSID_ATTRACTOR, CLSID_GENERATOR]);
    // Remove by group ID: only the group's templates go.
    assert!(cm.remove_controllers_from_occupant_group_id(Some(&s), 0x99, &occ));
    assert_eq!(cm.controllers_of(&occ).len(), 1);
    assert!(cm.add_controllers_from_occupant_group_id(Some(&s), 0x99, &occ, 0));
    assert!(cm.remove_controllers_from_occupant_group_name(Some(&s), "fire_occupant", &occ));
    assert!(!cm.remove_controllers_from_occupant_group_name(None, "fire_occupant", &occ));
    // RemoveControllers: everything, and the occupant entry with it.
    assert_eq!(cm.remove_controllers(&occ), 1);
    assert!(cm.record().entries.is_empty());
    assert_eq!(log.borrow().last().unwrap(), "release 0xa9bd882d00000040");
}

// ---- §6: automata ----------------------------------------------------------------------------------------------

fn sample_pedestrian() -> Rc<Automaton> {
    let a = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
    a.with_record(|r| {
        let b = r.base_mut();
        b.occupant.cell_rect = [64, 65, 64, 65];
        b.occupant.holder.parent = Some((
            ResKey {
                type_id: 0x6534_284A,
                group: 0x1,
                instance: 0x2,
            },
            Rc::new(crate::property::PropertyHolder::new()),
        ));
        b.vectors = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
        b.automata_flags = 0x10;
        b.model_keys = vec![ResKey {
            type_id: 0x5AD0_E817,
            group: 3,
            instance: 4,
        }];
        b.model = b.model_keys[0];
        b.callbacks = vec![host(CLSID_CONTROLLER_MANAGER, 5)];
        b.f3[7] = 0.5;
        b.tail = 9;
        for t in [
            0, 2, 7, 8, 0x0B, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x13, 0x14, 0x15, 0x34, 0x99,
        ] {
            b.states.push(State::new(t));
        }
        b.states[3].extra = StateExtra::RiotLeader { a: -1, b: 2, c: 3 };
        b.states[8].extra = StateExtra::PlayerDrive(1.5);
        b.states[10].extra = StateExtra::AttachAnim {
            model: host(0x1234, 6),
            anim_selector: 7,
            flag: true,
        };
        b.states[13].extra = StateExtra::Chase(host(CLSID_VEHICLE, 8));
        b.states[1].base.version = 2;
    });
    a
}

#[test]
fn automaton_records_round_trip_with_every_state_type() {
    let a = sample_pedestrian();
    let bytes = write_obj(&*a);
    let b = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
    let (ok, left) = read_obj(&*b, &bytes);
    assert!(ok && left == 0);
    assert_eq!(write_obj(&*b), bytes);
    let rec = b.record();
    assert_eq!(rec.base().states.len(), 15);
    assert_eq!(rec.base().states[3].extra, StateExtra::RiotLeader { a: -1, b: 2, c: 3 });
    assert_eq!(rec.base().states[8].extra, StateExtra::PlayerDrive(1.5));
    assert_eq!(rec.base().states[1].base.version, 2);
    assert_eq!(rec.base().states[5].extra, StateExtra::None); // 0x0D: default record
    assert_eq!(rec.base().states[14].extra, StateExtra::None); // outside the factory
    assert_eq!(
        rec.base().occupant.holder.parent_key(),
        Some(ResKey {
            type_id: 0x6534_284A,
            group: 1,
            instance: 2
        })
    );
    assert_eq!(rec.base().occupant.status, 2);
    assert_eq!(rec.base().occupant.occupant_type, AUTOMATON_OCCUPANT_TYPE);
}

#[test]
fn automaton_version_gates() {
    let a = sample_pedestrian();
    let bytes = write_obj(&*a);
    // Layout: u16 pedestrian version, u16 traffic version, u16 base version, u16 occupant version.
    assert_eq!(&bytes[..8], &[5, 0, 2, 0, 8, 0, 4, 0]);
    let fails = |at: usize, v: u16| {
        let mut b = bytes.clone();
        b[at..at + 2].copy_from_slice(&v.to_le_bytes());
        let x = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
        !read_obj(&*x, &b).0
    };
    assert!(fails(2, 1) && fails(2, 3)); // the traffic record requires 2
    assert!(fails(4, 9)); // the base accepts ≤ 8
    assert!(fails(6, 3) && fails(6, 5)); // the occupant base requires 4
                                         // A version-7 base has no final u32 and is written back as 7.
    let v7 = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
    v7.with_record(|r| r.base_mut().version = 7);
    let b7 = write_obj(&*v7);
    let n8 = write_obj(&*Automaton::for_class(CLSID_PEDESTRIAN).unwrap());
    assert_eq!(b7.len() + 4, n8.len());
    let x = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
    assert_eq!(read_obj(&*x, &b7), (true, 0));
    assert_eq!(write_obj(&*x), b7);
    // ...unless it gained the last u32.
    v7.with_record(|r| r.base_mut().tail = 1);
    assert_eq!(write_obj(&*v7).len(), n8.len());
    // A state base above 3 fails the whole automaton.
    let s = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
    s.push_state(0x14, SetStateArgs::default());
    let mut sb = write_obj(&*s);
    let n = n8.len();
    // Find the state's version: the state list follows the fixed part; patch it by search.
    let pos = (0..sb.len() - 6)
        .find(|&i| sb[i..i + 4] == [0x14, 0, 0, 0] && sb[i + 4..i + 6] == [3, 0])
        .unwrap();
    sb[pos + 4] = 4;
    let x = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
    assert!(!read_obj(&*x, &sb).0);
    assert!(n > 0);
}

#[test]
fn vehicle_family_round_trips() {
    let v = Automaton::for_class(CLSID_VEHICLE).unwrap();
    v.with_record(|r| {
        if let AutomatonRecord::Vehicle(x) = r {
            x.path = vec![(1, -2, 3)];
            x.f = 2.5;
        }
    });
    let b = write_obj(&*v);
    let tail = &b[b.len() - 4 - 24 - 4..];
    assert_eq!(&tail[..4], &[0, 0, 0, 0]);
    let x = Automaton::for_class(CLSID_VEHICLE).unwrap();
    assert_eq!(read_obj(&*x, &b), (true, 0));
    assert_eq!(write_obj(&*x), b);
    let AutomatonRecord::Vehicle(r) = x.record() else {
        panic!()
    };
    assert_eq!(r.constant, VEHICLE_CONSTANT);
    assert_eq!(r.path, vec![(1, -2, 3)]);

    for class in [CLSID_PATROL_CAR, CLSID_TRAILER] {
        let a = Automaton::for_class(class).unwrap();
        a.with_record(|r| match r {
            AutomatonRecord::PatrolCar(p) => p.target = host(CLSID_PEDESTRIAN, 3),
            AutomatonRecord::Trailer(t) => {
                t.trailers = vec![host(CLSID_VEHICLE, 4)];
                t.values = vec![5, 6];
                t.network = host(0xC9C0_5C6E, 7);
            }
            _ => unreachable!(),
        });
        let b = write_obj(&*a);
        let x = Automaton::for_class(class).unwrap();
        assert_eq!(read_obj(&*x, &b), (true, 0));
        assert_eq!(write_obj(&*x), b);
        // Equal but for the status byte a read sets (lot-records §2.3).
        let mut want = a.record();
        want.base_mut().occupant.status = 2;
        assert_eq!(x.record(), want);
    }

    // An emergency vehicle answers the vehicle's class, so it reloads as a
    // plain vehicle (§12 quirk 3), which cannot read its record: with the
    // layout of §6.4 the vehicle reader meets the emergency's own version
    // first, misaligns and fails at the occupant base's version (AM8).
    let e = Automaton::new_emergency();
    assert_eq!(e.class_id(), CLSID_VEHICLE);
    let b = write_obj(&*e);
    let x = Automaton::for_class(CLSID_VEHICLE).unwrap();
    let (ok, left) = read_obj(&*x, &b);
    assert!(!ok || left != 0);
    let y = Automaton::new_emergency();
    assert_eq!(read_obj(&*y, &b), (true, 0));
    assert_eq!(write_obj(&*y), b);
}

#[test]
fn managers_do_not_save_emergency_vehicles() {
    let mut m = AutomataManager::new(ManagerKind::Vehicle);
    let e = Automaton::new_emergency();
    let v = Automaton::for_class(CLSID_VEHICLE).unwrap();
    v.set_host_token(0x21);
    assert!(m.add_automaton(RefValue::Object(e)));
    assert!(m.add_automaton(RefValue::Object(v.clone())));
    assert!(!m.add_automaton(RefValue::Object(v)));
    let b = write_manager(&m);
    // base: version, max, active, flag, n = 1, one ref, group map.
    assert_eq!(&b[2..22], &hex("0600 00000000 00 00 01000000 21000000 4A716E89")[..]);
}

#[test]
fn automaton_slots() {
    let a = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
    assert!(a.is_visible());
    a.set_visible(false);
    assert!(!a.is_visible());
    a.set_visible(true);
    assert!(a.is_visible());
    // §8.1 start step 5 pushes the RiotLeader `final`, so the end step 1's
    // SetState(1, …) is a no-op (§12 quirk 11) and the leader keeps state 8.
    assert!(a.set_state(8, RIOT_LEADER_STATE_ARGS));
    assert_eq!(a.current_state(), Some(8));
    assert_eq!(a.last_set_state(), Some((8, RIOT_LEADER_STATE_ARGS)));
    assert!(!a.set_state(state_type::RIOT_END, RIOT_END_STATE_ARGS));
    assert_eq!(a.current_state(), Some(8));
    assert!(a.push_state(0x14, SetStateArgs::default()));
    assert_eq!(a.current_state(), Some(0x14));
    // Popping Collide (not `final`) reveals the RiotLeader; popping that
    // `final` state pushes DISAPPEAR instead of revealing below (quirk 11);
    // popping DISAPPEAR does reveal it.
    assert!(a.pop_state());
    assert_eq!(a.current_state(), Some(8));
    assert!(a.pop_state());
    assert_eq!(a.current_state(), Some(state_type::RIOT_END));
    assert!(a.pop_state());
    assert_eq!(a.current_state(), Some(8));
    a.remove();
    a.return_to_source_occupant();
    assert!(a.is_removed() && a.is_returning());
    assert!(a.answers_iid(IID_AUTOMATON) && a.answers_iid(IID_OCCUPANT));
    assert!(!a.answers_iid(IID_AUTOMATA_CONTROLLER));
    // A reference to a core object of the wrong interface reads as null.
    let c = Controller::for_class(CLSID_GENERATOR).unwrap();
    assert!(ref_answers(&RefValue::Object(c.clone()), IID_AUTOMATA_CONTROLLER));
    assert!(!ref_answers(&RefValue::Object(c), IID_AUTOMATON));
}

// ---- §7.3 and §9: the manager slots ------------------------------------------------------------------------------

#[test]
fn set_max_count_clamps_and_sets_the_hard_limit() {
    let mut m = AutomataManager::new(ManagerKind::Pedestrian);
    m.set_max_count(9000);
    assert_eq!((m.max_count(), m.hard_limit()), (5000, 6250));
    m.set_max_count(-3);
    assert_eq!((m.max_count(), m.hard_limit()), (0, 0));
    m.set_max_count(3);
    assert_eq!(m.hard_limit(), 3);
    m.set_max_count(199);
    assert_eq!(m.hard_limit(), 248);
    for i in 0..5 {
        let a = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
        a.set_host_token(i);
        m.add_automaton(RefValue::Object(a));
    }
    let removed = m.set_max_count(2);
    assert_eq!((removed.len(), m.automata_count()), (3, 2));
    // Init applies the saved max count.
    let (mut m, _, _) = read_manager(
        ManagerKind::Vehicle,
        &hex("0200 0400 50010000 00 00 00000000 01000000 01000000"),
    );
    assert!(m.init());
    assert_eq!(m.hard_limit(), 420);
}

#[test]
fn create_automaton_stub_and_driving_mode() {
    let mut m = AutomataManager::new(ManagerKind::Pedestrian);
    m.set_max_count(1);
    let args = CreateArgs {
        group: RIOT_AUTOMATA_GROUP,
        position: [10.0, 0.0, 20.0],
        flags: RIOT_CREATE_FLAGS,
        model_key: ResKey::default(),
        callback: 0,
        priority: RIOT_CREATE_PRIORITY,
        orientation: None,
    };
    let a = m.create_automaton(args).unwrap();
    assert_eq!(
        (a.group(), a.position(), a.class_id()),
        (0x4107, [10.0, 0.0, 20.0], CLSID_PEDESTRIAN)
    );
    assert!(m.create_automaton(args).is_none()); // hard limit trunc(1 × 1.25) = 1
    assert!(AutomataManager::new(ManagerKind::Aircraft)
        .create_automaton(args)
        .is_none());
    assert!(m.is_driving_mode(U_DRIVE_IT_VIEW) && !m.is_driving_mode(0));
    let mut v = AutomataManager::new(ManagerKind::Vehicle);
    let mut air = AutomataManager::new(ManagerKind::Aircraft);
    let mut boat = AutomataManager::new(ManagerKind::Watercraft);
    pause_all(&mut [&mut m, &mut v, &mut air, &mut boat]);
    assert!(m.is_paused() && v.is_paused() && air.is_paused() && boat.is_paused());
    resume_all(&mut [&mut m, &mut v, &mut air, &mut boat]);
    assert!(!m.is_paused() && !boat.is_paused());
    m.activate(true);
    assert!(m.is_active());
}

// ---- §14 tests 8 and 9: the hooks -------------------------------------------------------------------------------

#[derive(Default)]
struct FakeCity {
    calls: Vec<String>,
    road: bool,
    cap: bool,
    create_null: bool,
}

impl RiotHost for FakeCity {
    fn nearest_road_tile(&mut self, x: i32, z: i32) -> Option<Vec3> {
        self.calls.push(format!("road {x} {z}"));
        self.road.then_some([x as f32 * 16.0, 0.0, z as f32 * 16.0])
    }
    fn leader_cap_reached(&mut self, leaders: usize) -> bool {
        self.calls.push(format!("cap {leaders}"));
        self.cap
    }
    fn create_automaton(&mut self, a: CreateArgs) -> u64 {
        self.calls.push(format!(
            "CreateAutomaton {:#x} {:?} {} {:?} {} {:#x} {}",
            a.group,
            a.position,
            a.flags,
            [a.model_key.type_id, a.model_key.group, a.model_key.instance],
            a.callback,
            a.priority,
            u8::from(a.orientation.is_some())
        ));
        if self.create_null {
            0
        } else {
            0x500
        }
    }
    fn add_ref(&mut self, a: u64) {
        self.calls.push(format!("AddRef {a:#x}"));
    }
    fn set_state(&mut self, a: u64, t: u32, s: SetStateArgs) {
        self.calls.push(format!(
            "SetState {a:#x} {t} {} {} {} {} {} {}",
            s.controller, s.timeout_ms, s.final_flag, s.anim_id, s.sfx_id, s.template
        ));
    }
    fn set_visible(&mut self, a: u64, v: bool) {
        self.calls.push(format!("SetVisible {a:#x} {v}"));
    }
    fn as_occupant(&mut self, a: u64) -> u64 {
        a + 1
    }
    fn register_location(&mut self, a: u64, _p: Vec3) {
        self.calls.push(format!("Register {a:#x}"));
    }
    fn add_controller_from_template_name(&mut self, occ: u64, name: &str, extra: u32) -> bool {
        self.calls
            .push(format!("AddControllerFromTemplateName {occ:#x} {name} {extra}"));
        true
    }
    fn remove_controllers(&mut self, occ: u64) {
        self.calls.push(format!("RemoveControllers {occ:#x}"));
    }
    fn unregister_location(&mut self, occ: u64) {
        self.calls.push(format!("Unregister {occ:#x}"));
    }
    fn release(&mut self, a: u64) {
        self.calls.push(format!("Release {a:#x}"));
    }
}

#[test]
fn t08_riot_order() {
    let mut city = FakeCity {
        road: true,
        ..FakeCity::default()
    };
    let mut riot = RiotLeaders::new();
    assert_eq!(riot.add(&mut city, 3, 4), Some(0));
    assert_eq!(
        city.calls,
        [
            "road 3 4",
            "cap 0",
            "CreateAutomaton 0x4107 [48.0, 0.0, 64.0] 2 [0, 0, 0] 0 0xff 0",
            "AddRef 0x500",
            "SetState 0x500 8 0 -1 1 0 0 0",
            "SetVisible 0x500 true",
            "Register 0x500",
        ]
    );
    city.calls.clear();
    assert!(riot.set_stage(&mut city, 0, 2));
    assert_eq!(city.calls, ["AddControllerFromTemplateName 0x501 riot_leader_2 0"]);
    // Not a new maximum, or stage 6 and above: nothing.
    city.calls.clear();
    assert!(!riot.set_stage(&mut city, 0, 1));
    assert!(!riot.set_stage(&mut city, 0, 2));
    assert!(!riot.set_stage(&mut city, 0, 6));
    assert!(city.calls.is_empty());
    assert!(riot.end(&mut city, 0));
    assert_eq!(
        city.calls,
        [
            "SetState 0x500 1 0 -1 0 0 0 0",
            "RemoveControllers 0x501",
            "Unregister 0x501",
            "Release 0x500",
        ]
    );
    assert!(riot.leaders().is_empty());
    assert!(!riot.end(&mut city, 0));
    // The failure paths stop where the spec says.
    let mut no_road = FakeCity::default();
    assert_eq!(riot.add(&mut no_road, 1, 1), None);
    assert_eq!(no_road.calls, ["road 1 1"]);
    let mut capped = FakeCity {
        road: true,
        cap: true,
        ..FakeCity::default()
    };
    assert_eq!(riot.add(&mut capped, 1, 1), None);
    assert_eq!(capped.calls.len(), 2);
    let mut null = FakeCity {
        road: true,
        create_null: true,
        ..FakeCity::default()
    };
    assert_eq!(riot.add(&mut null, 1, 1), None);
    assert_eq!(null.calls.len(), 3);
    assert_eq!(riot_leader_template(5), "riot_leader_5");
}

#[derive(Default)]
struct FakeFire {
    calls: Vec<String>,
}

impl FireHost for FakeFire {
    fn occupant_type(&mut self, occ: u64) -> u32 {
        if occ == 0x10 {
            OCCUPANT_TYPE_BUILDING
        } else {
            0x7475_8926
        }
    }
    fn add_controllers_from_occupant_group_name(&mut self, name: &str, occ: u64, extra: u32) -> bool {
        self.calls.push(format!("slot10 {name} {occ:#x} {extra}"));
        true
    }
    fn remove_controllers(&mut self, occ: u64) {
        self.calls.push(format!("slot14 {occ:#x}"));
    }
}

#[test]
fn t09_fire() {
    let mut f = FakeFire::default();
    let mut hook = FireHook::new();
    assert!(hook.set_target(&mut f, 0x10));
    assert_eq!(f.calls, ["slot10 fire_occupant 0x10 0"]);
    assert!(hook.shutdown(&mut f));
    assert_eq!(f.calls[1], "slot14 0x10");
    assert_eq!(hook.target(), 0);
    let mut f = FakeFire::default();
    assert!(!hook.set_target(&mut f, 0x20));
    assert!(f.calls.is_empty());
    assert!(hook.shutdown(&mut f));
    assert_eq!(f.calls, ["slot14 0x20"]);
    assert!(!hook.shutdown(&mut f));
}

#[test]
fn message_and_class_tables() {
    assert_eq!(msg::CONTROLLER_MANAGER.len(), 8);
    assert_eq!(msg::MANAGER_BASE.len(), 16);
    assert_eq!(RECORD_CLASSES.len(), 12);
    assert_eq!(
        ManagerKind::from_class(CLSID_AIRCRAFT_MANAGER),
        Some(ManagerKind::Aircraft)
    );
    for k in [
        ManagerKind::Vehicle,
        ManagerKind::Pedestrian,
        ManagerKind::Aircraft,
        ManagerKind::Watercraft,
    ] {
        assert_eq!(ManagerKind::from_class(k.class_id()), Some(k));
    }
    assert!(Automaton::for_class(0x1234).is_none() && Controller::for_class(0x1234).is_none());
}

// ---- §6.2 quirks 10-14 (re-integrate1's corrections; §14 tests 24, 25, 29) -------------------------------

fn f32s(v: &[f32]) -> Vec<u8> {
    let mut b = Vec::with_capacity(v.len() * 4);
    for x in v {
        b.extend_from_slice(&x.to_le_bytes());
    }
    b
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

fn round_trip(a: &Automaton) -> Rc<Automaton> {
    let bytes = write_obj(a);
    let b = Automaton::for_class(a.class_id()).unwrap();
    let (ok, left) = read_obj(&*b, &bytes);
    assert!(ok && left == 0);
    b
}

/// §14 test 24 (§6.2, quirk 12): a Plummet record writes its five `f32` in
/// the stream order `+0x28, +0x34, +0x30, +0x2C, +0x38`, **not** offset order,
/// and writes nothing for the object's sixth slot `+0x3C` (an unsaved `ref`).
#[test]
fn plummet_stream_order_is_not_offset_order() {
    let a = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
    assert!(a.set_state(state_type::T0E, SetStateArgs::default()));
    let make = |descent, roll_rate, roll_centre, tumble_ms, post_splash_ms| StateExtra::T0E {
        descent,
        roll_rate,
        roll_centre,
        tumble_ms,
        post_splash_ms,
    };
    a.with_record(|r| {
        r.base_mut().states[0].extra = make(1.0, 2.0, 3.0, 4.0, 5.0);
    });
    let bytes = write_obj(&*a);
    // Stream order: +0x28, +0x34, +0x30, +0x2C, +0x38.
    assert!(contains(&bytes, &f32s(&[1.0, 4.0, 3.0, 2.0, 5.0])));
    // Offset order is not what the record holds.
    assert!(!contains(&bytes, &f32s(&[1.0, 2.0, 3.0, 4.0, 5.0])));
    // Reading it back leaves each `f32` at its own offset.
    let b = round_trip(&a);
    assert_eq!(b.record().base().states[0].extra, make(1.0, 2.0, 3.0, 4.0, 5.0));
}

/// §14 test 25 (§6.2, quirk 14): an AttachAnim record writes `ref`,
/// `u16`, `bool` in that stream order while the object holds the `bool` at
/// `+0x2C` before the `u16` at `+0x30`.
#[test]
fn attach_anim_stream_order_hides_the_memory_order() {
    let a = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
    assert!(a.set_state(state_type::ATTACH_ANIM, SetStateArgs::default()));
    let make = |flag, anim_selector| StateExtra::AttachAnim {
        model: host(0x1234, 6),
        flag,
        anim_selector,
    };
    a.with_record(|r| {
        r.base_mut().states[0].extra = make(true, 7);
    });
    let bytes = write_obj(&*a);
    // Stream: ref (object 6 of class 0x1234), u16 7, bool 1.
    let mut stream = Vec::new();
    stream.extend_from_slice(&6u32.to_le_bytes());
    stream.extend_from_slice(&0x1234u32.to_le_bytes());
    stream.extend_from_slice(&7u16.to_le_bytes());
    stream.push(1u8);
    assert!(contains(&bytes, &stream));
    // The memory order (the bool before the u16) is not what the record holds.
    let mut memory = Vec::new();
    memory.extend_from_slice(&6u32.to_le_bytes());
    memory.extend_from_slice(&0x1234u32.to_le_bytes());
    memory.push(1u8);
    memory.extend_from_slice(&7u16.to_le_bytes());
    assert!(!contains(&bytes, &memory));
    let b = round_trip(&a);
    assert_eq!(b.record().base().states[0].extra, make(true, 7));
}

/// §14 test 29 (§6.2, quirk 10): the state's `+0x0C` is a stack index --
/// 0-based from the bottom, recomputed after every stack **mutation**.
/// §14 test 4 (byte-exact re-save) wins over recompute-on-**Read**, which the
/// corpus contradicts: see `AutomatonBase::read` and glue19 question A1.
#[test]
fn stack_index_is_recomputed_on_stack_mutations() {
    let idx = |a: &Automaton| -> Vec<u32> { a.record().base().states.iter().map(|s| s.base.stack_index).collect() };
    let a = Automaton::for_class(CLSID_PEDESTRIAN).unwrap();
    assert!(a.push_state(0x0B, SetStateArgs::default()));
    assert!(a.push_state(0x0F, SetStateArgs::default()));
    assert!(a.push_state(state_type::T14, SetStateArgs::default()));
    assert_eq!(idx(&a), vec![0, 1, 2]);
    assert!(a.pop_state());
    assert_eq!(idx(&a), vec![0, 1]);

    // A stale index is kept across a round trip (byte-exact re-save) and is
    // corrected the next time the stack changes.
    a.with_record(|r| {
        for s in r.base_mut().states.iter_mut() {
            s.base.stack_index = 9;
        }
    });
    assert_eq!(idx(&a), vec![9, 9]);
    let b = round_trip(&a);
    assert_eq!(idx(&b), vec![9, 9]);
    assert!(b.push_state(0x0B, SetStateArgs::default()));
    assert_eq!(idx(&b), vec![0, 1, 2]);
}
