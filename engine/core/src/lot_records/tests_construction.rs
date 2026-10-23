//! Construction occupant tests: lot-records §9 record rules and
//! lot-developer §16 tests 20, 21, 23, 24, 25.

use super::construction::*;
use super::tests::{read_obj, rec, token, write_obj};
use super::*;
use crate::network_occupants::Xform;
use crate::rng::{CrtRand, FixedSeeds, RzRandom};
use crate::stream::{ByteReader, ByteWriter, RefValue};
use crate::variant::{mem, Elem, Variant};
use crate::x87::r24;

fn host_ref(id: u64, class_id: u32) -> RefValue {
    RefValue::Host { token: id, class_id }
}

fn sample() -> ConstructionRecord {
    let mut r = ConstructionRecord::new();
    r.base.flags = 5;
    r.base.cell_rect = [64, 65, 66, 67];
    r.lot = host_ref(7, CLSID_LOT);
    r.building = host_ref(8, CLSID_BUILDING_OCCUPANT);
    r.transform = Xform::default();
    r.world_bounds = ([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]);
    r.local_bounds = ([-1.5, 0.0, -2.5], [1.5, 3.0, 2.5]);
    r.has_crane = false;
    r.crane_position = [10.0, 20.0, 30.0];
    r.ground_props = vec![host_ref(11, 0x2977_AA47), host_ref(12, 0x2977_AA47)];
    r.roof_props = vec![host_ref(13, 0x2977_AA47)];
    r.effects = vec![host_ref(14, 0x2977_AA49)];
    r.current_effect = host_ref(14, 0x2977_AA49);
    r.scalars = ConstructionScalars {
        state: -1,
        t0: 1,
        unused_148: 2,
        next_day: 3,
        segments: 2_451_545,
        counter: 2_451_560,
        span: 0.25,
        start_y: 6.0,
        a: 4,
        b: 0x42C8_3333,
        c: 0x42C8_0000,
        e: 6,
        ms_acc: 7,
        crane_state: 3,
        swing_min: 12.5,
        swing_max: 1000.0,
        unused_180: true,
        crane_acc: 9,
        crane_duration: 10,
        crane_target: 0.5,
        crane_previous: 1.0,
        crane_type: 0xAB,
    };
    r.crew = vec![
        CrewEntry {
            first: true,
            second: false,
            automaton: host_ref(21, 0x896E_75AF),
        },
        CrewEntry {
            first: false,
            second: true,
            automaton: RefValue::Null,
        },
    ];
    r
}

fn sample_bytes() -> Vec<u8> {
    write_obj(&RefValue::Object(RecordObject::new(sample())))
}

#[test]
fn record_layout_and_round_trip() {
    let b = sample_bytes();
    assert_eq!(&b[..2], &[5, 0]);
    // §2.3 base: 14 bytes + an empty exemplar holder (u16 2, u16 2, u32 0,
    // 3 × u32 parent key) = 34; then the refs, xform mask 0, two boxes …
    let (o, ok, rem) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &b);
    assert!(ok);
    assert_eq!(rem, 0);
    let r: ConstructionRecord = rec(&o);
    assert_eq!(token(&r.lot), Some((7, CLSID_LOT)));
    assert_eq!(token(&r.building), Some((8, CLSID_BUILDING_OCCUPANT)));
    assert_eq!(r.ground_props.len(), 2);
    assert_eq!(r.crew.len(), 2);
    assert!(r.crew[0].first && !r.crew[0].second && r.crew[1].automaton.is_null());
    assert_eq!(r.scalars, sample().scalars);
    // #8 is read into the load-only flag (+0xD8) and marks the refresh
    // pending; the live crane flag stays false, so a Write before the
    // refresh saves #8 = false [quirk, lr §9.2] and nothing else changes.
    assert!(!r.loaded_has_crane && r.refresh_pending && !r.has_crane);
    assert_eq!(write_obj(&o), b);
    let mut live = sample();
    live.has_crane = true;
    let bc = write_obj(&RefValue::Object(RecordObject::new(live)));
    let (o, ok, _) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &bc);
    assert!(ok);
    let r: ConstructionRecord = rec(&o);
    assert!(r.loaded_has_crane && r.refresh_pending && !r.has_crane);
    let w = write_obj(&o);
    let diff: Vec<usize> = (0..bc.len()).filter(|&i| w[i] != bc[i]).collect();
    assert_eq!((w.len(), diff.len()), (bc.len(), 1));
    assert_eq!(
        (bc[diff[0]], w[diff[0]], &w[..diff[0]], &w[diff[0] + 1..]),
        (1, 0, &b[..diff[0]], &b[diff[0] + 1..])
    );
    // The tail: u8 0xAB, crew count 2, {1, 0, ref 21}, {0, 1, null}.
    let tail = &b[b.len() - sample_tail_len()..];
    assert_eq!(tail[0], 0xAB);
    assert_eq!(&tail[1..5], &[2, 0, 0, 0]);
    assert_eq!(&tail[5..7], &[1, 0]);
    assert_eq!(&tail[15..17], &[0, 1]);
    assert_eq!(&tail[17..21], &[0, 0, 0, 0]);
}

#[test]
fn bool_fields_read_nonzero_and_write_one() {
    let mut b = sample_bytes();
    let pos = b.len() - sample_tail_len() + 5; // crew entry 0, first bool
    b[pos] = 7;
    let (o, ok, _) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &b);
    assert!(ok);
    let w = write_obj(&o);
    assert_eq!(w[pos], 1);
}

fn sample_tail_len() -> usize {
    21
}

#[test]
fn version_rules() {
    let b = sample_bytes();
    // < 4: false before the base, 2 bytes consumed.
    let mut v3 = b.clone();
    v3[0] = 3;
    let (_, ok, rem) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &v3);
    assert!(!ok);
    assert_eq!(rem, b.len() - 2);
    // > 5: read as 5, written back as 5.
    let mut v9 = b.clone();
    v9[0] = 9;
    let (o, ok, rem) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &v9);
    assert!(ok);
    assert_eq!(rem, 0);
    assert_eq!(write_obj(&o), b);
    // 4: the v5 layout with the 56-byte legacy transform (NO15); see
    // `v4_legacy_transform_record`.
    // Short payload: false.
    let (_, ok, _) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &b[..b.len() - 1]);
    assert!(!ok);
}

/// The NO15 worked example: mask 1, translation (8, 2.5, 8), 13 bytes.
const EXAMPLE_XFORM: [u8; 13] = [1, 0, 0, 0, 0x41, 0, 0, 0x20, 0x40, 0, 0, 0, 0x41];

fn le(vals: &[f32]) -> Vec<u8> {
    vals.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// A v4 stream built from the v5 sample (transform = the worked example)
/// with its 13-byte transform replaced by a 56-byte legacy one.
fn v4_bytes(scale: f32, tail: u32) -> (Vec<u8>, Vec<u8>, usize) {
    let mut r = sample();
    r.transform.mask = 1;
    r.transform.translation = [8.0, 2.5, 8.0];
    let v5 = write_obj(&RefValue::Object(RecordObject::new(r)));
    let at = v5.windows(13).position(|w| w == EXAMPLE_XFORM).unwrap();
    assert_eq!(v5.windows(13).filter(|w| *w == EXAMPLE_XFORM).count(), 1);
    let mut legacy = le(&[0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 8.0, 2.5, 8.0, scale]);
    legacy.extend_from_slice(&tail.to_le_bytes());
    assert_eq!(legacy.len(), 56);
    let mut v4 = v5[..at].to_vec();
    v4[0] = 4;
    v4.extend_from_slice(&legacy);
    v4.extend_from_slice(&v5[at + 13..]);
    (v4, v5, at)
}

#[test]
fn v4_legacy_transform_record() {
    // Scale 1.0: mask 4|1, the scale bit cleared; low byte of the u32 is the
    // revision counter; the rest of the record reads as in v5.
    let (v4, v5, at) = v4_bytes(1.0, 0x1234_5678);
    assert_eq!(v4.len(), v5.len() - 13 + 56);
    let (o, ok, rem) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &v4);
    assert!(ok);
    assert_eq!(rem, 0);
    let r: ConstructionRecord = rec(&o);
    assert_eq!(r.transform.mask, 5);
    assert_eq!(r.transform.matrix, [0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
    assert_eq!(r.transform.translation, [8.0, 2.5, 8.0]);
    assert_eq!(r.transform.scale, 1.0);
    assert_eq!(r.transform.counter, 0x78);
    assert_eq!(token(&r.building), Some((8, CLSID_BUILDING_OCCUPANT)));
    assert_eq!(r.ground_props.len(), 2);
    assert_eq!(r.scalars.crane_type, 0xAB);
    // Written back as v5: mask 5, 9 + 3 floats; the tail identical.
    let w = write_obj(&o);
    assert_eq!(w[0], 5);
    let mut head = v4[..at].to_vec();
    head[0] = 5;
    assert_eq!(&w[..at], &head[..]);
    assert_eq!(w[at], 5);
    assert_eq!(&w[at + 1..at + 49], &v4[at..at + 48]);
    assert_eq!(&w[at + 49..], &v5[at + 13..]);
    // Scale 2.0: the scale bit set (mask 7); NaN counts as different.
    let (v4, _, _) = v4_bytes(2.0, 0xFFFF_FF00);
    let (o, ok, _) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &v4);
    assert!(ok);
    let r: ConstructionRecord = rec(&o);
    assert_eq!((r.transform.mask, r.transform.scale, r.transform.counter), (7, 2.0, 0));
    let (v4, _, _) = v4_bytes(f32::NAN, 3);
    let (o, ok, _) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &v4);
    assert!(ok);
    let r: ConstructionRecord = rec(&o);
    assert_eq!(r.transform.mask, 7);
    // v4 cut inside the legacy transform, or one byte short at the end: false.
    let (v4, _, at) = v4_bytes(1.0, 0);
    for cut in [at + 1, at + 52, at + 55, v4.len() - 1] {
        let (_, ok, _) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &v4[..cut]);
        assert!(!ok, "cut at {cut}");
    }
    // The v5 form relabelled 4 is not the legacy form: 43 bytes more are
    // consumed as the transform, so the stream misreads (no exact match).
    let (_, v5, _) = v4_bytes(1.0, 0);
    let mut relabel = v5.clone();
    relabel[0] = 4;
    let (o, ok, rem) = read_obj(construction::CLSID_CONSTRUCTION_OCCUPANT, &relabel);
    assert!(!ok || rem != 0 || write_obj(&o) != v5);
}

#[test]
fn xform_read_rules_no15() {
    // Worked example writes 13 bytes; mask 7, identity, scale 2.0 writes 53
    // bytes ending 00 00 00 40.
    let x = Xform {
        mask: 1,
        translation: [8.0, 2.5, 8.0],
        ..Xform::default()
    };
    let mut w = ByteWriter::new();
    assert!(x.write(&mut w));
    assert_eq!(w.into_bytes(), EXAMPLE_XFORM);
    let y = Xform {
        mask: 7,
        scale: 2.0,
        ..Xform::default()
    };
    let mut w = ByteWriter::new();
    assert!(y.write(&mut w));
    let b = w.into_bytes();
    assert_eq!(b.len(), 53);
    assert_eq!(&b[49..], &[0, 0, 0, 0x40]);
    // Mask 7 with scale 1.0 reads back as 5; live mask 0: no reset, counter
    // +1 per part only.
    let mut b7 = vec![7u8];
    b7.extend(le(&[2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0, 1.0, 2.0, 3.0, 1.0]));
    let mut t = Xform::default();
    let mut c = 0u8;
    t.counter = c;
    t.read_into(&mut ByteReader::new(&b7)).unwrap();
    c = t.counter;
    assert_eq!((t.mask, t.scale, c), (5, 1.0, 3));
    // Live mask non-zero: reset to identity first (+1); an omitted part is
    // the identity value, not the prior one.
    let mut live = Xform {
        counter: 0,
        mask: 7,
        matrix: [9.0; 9],
        translation: [5.0; 3],
        scale: 4.0,
    };
    let mut c = 250u8;
    live.counter = c;
    live.read_into(&mut ByteReader::new(&EXAMPLE_XFORM)).unwrap();
    c = live.counter;
    assert_eq!(live.mask, 1);
    assert_eq!(live.matrix, [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    assert_eq!(live.translation, [8.0, 2.5, 8.0]);
    assert_eq!(live.scale, 1.0);
    assert_eq!(c, 252);
    // Live mask 0 with non-identity parts: omitted parts untouched.
    let mut odd = Xform {
        mask: 0,
        scale: 4.0,
        ..Xform::default()
    };
    let mut c = 0u8;
    odd.counter = c;
    odd.read_into(&mut ByteReader::new(&EXAMPLE_XFORM)).unwrap();
    c = odd.counter;
    assert_eq!((odd.mask, odd.scale, c), (1, 4.0, 1));
    // Legacy: mask never reset (bits stay), scale bit by the 1.0 test.
    let mut lg = le(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    lg.extend_from_slice(&0x0000_01FEu32.to_le_bytes());
    let mut t = Xform {
        mask: 2,
        scale: 3.0,
        ..Xform::default()
    };
    let mut c = 9u8;
    t.counter = c;
    t.legacy_load_into(&mut ByteReader::new(&lg)).unwrap();
    c = t.counter;
    assert_eq!((t.mask, t.scale, c), (5, 1.0, 0xFE));
    let mut t = Xform {
        mask: 0x10,
        ..Xform::default()
    };
    t.counter = c;
    t.legacy_load_into(&mut ByteReader::new(&lg)).unwrap();
    c = t.counter;
    assert_eq!(t.mask, 0x15);
    let mut short = Xform::default();
    let mut cut = ByteReader::new(&lg[..55]);
    short.counter = c;
    assert!(short.legacy_load_into(&mut cut).is_none());
}

#[test]
fn tuning_shipped_and_defaults() {
    let shipped = |id: u32| -> Option<f32> {
        Some(match id {
            0x8989_EC22 => 3.0,
            0x8989_EC24 => 2.0,
            0x8989_EC26 => 15.0,
            0x8989_EC27 | 0x8989_EC28 => 25.0,
            0x8989_EC29 => 1.0,
            0x8989_EC2A => 0.5,
            0x8989_EC2D => 3.0,
            _ => return None,
        })
    };
    let t = ConstructionTuning::from_properties(shipped);
    assert_eq!(t.growth_rate, 1.0);
    assert_eq!(t.removal_rate, 2.0);
    assert_eq!(t.scaffold_gap, 2.0);
    assert_eq!(t.plank_texture_scale, Some(1.0 / 15.0));
    assert_eq!(t.girder_texture_scale, Some(0.04));
    assert_eq!(t.crane_height_above_scaffold, 3.0);
    assert_eq!(t.post_width, 1.0);
    let d = ConstructionTuning::from_properties(|_| None);
    assert_eq!(d, ConstructionTuning::default());
    assert_eq!((d.growth_rate, d.removal_rate, d.plank_texture_scale), (1.0, 2.0, None));
}

fn shipped_tuning() -> ConstructionTuning {
    ConstructionTuning {
        scaffold_gap: 2.0,
        ..ConstructionTuning::default()
    }
}

/// (max.y, corner altitude, D, f bits or 0, N, A, B, C, E)
type Row = (f32, Option<f32>, Option<u8>, u32, u32, i64, i64, i64, u32);

const TEST20: [Row; 9] = [
    (110.0, None, None, 0x4053_3335, 4, 0, 1, 1, 6),
    (110.0, Some(99.0), None, 0, 4, 1, 1, 1, 6),
    (110.0, Some(99.5), None, 0, 4, 1, 1, 1, 6),
    (110.0, None, Some(0), 0, 4, 0, 1, 1, 1),
    (110.0, None, Some(255), 0, 4, 0, 1, 1, 62),
    (160.0, None, None, 0x419F_BBBC, 20, 0, 1, 1, 1),
    (100.25, None, None, 0, 1, 0, 1, 0, 29),
    (103.125, None, None, 0, 2, 0, 1, 1, 13),
    (103.0, None, None, 0, 1, 0, 1, 1, 28),
];

fn geom_for(row: &Row) -> ConstructionGeometry {
    let bbox = ([0.0, 100.0, 0.0], [10.0, row.0, 10.0]);
    let mut first = true;
    rebuild_model(&bbox, &shipped_tuning(), |_, _| {
        let a = if first { row.1.unwrap_or(100.0) } else { 100.0 };
        first = false;
        a
    })
}

#[test]
fn test20_construction_timing() {
    for row in &TEST20 {
        let g = geom_for(row);
        assert_eq!(g.start_y.to_bits(), 0x42C8_3333, "{row:?}");
        if row.3 != 0 {
            assert_eq!(g.f.to_bits(), row.3, "{row:?}");
        }
        let t = timing(&g, &shipped_tuning(), row.2);
        assert_eq!(
            (g.segments, t.a, t.b, t.c, t.e),
            (row.4, row.5, row.6, row.7, row.8),
            "{row:?}"
        );
    }
    let g = geom_for(&TEST20[2]);
    assert_eq!(g.ground, 99.5);
    assert_eq!(g.scaffold, ([-2.0, 99.5, -2.0], [12.0, 110.0, 12.0]));
    // A NaN altitude keeps the current ground.
    let g = rebuild_model(&([0.0, 100.0, 0.0], [1.0, 110.0, 1.0]), &shipped_tuning(), |_, _| {
        f32::NAN
    });
    assert_eq!(g.ground, 100.0);
}

// ---- a recording host -------------------------------------------------------------------------------

#[derive(Default)]
struct Host {
    day: u32,
    bbox: Bounds,
    corner: Option<f32>,
    days: Option<u8>,
    completion: f32,
    crane: bool,
    log: Vec<String>,
}

impl Host {
    fn new(max_y: f32, corner: Option<f32>, days: Option<u8>) -> Self {
        Host {
            bbox: ([0.0, 100.0, 0.0], [10.0, max_y, 10.0]),
            corner,
            days,
            ..Host::default()
        }
    }
    fn say(&mut self, s: String) {
        self.log.push(s);
    }
}

impl ConstructionHost for Host {
    fn subscribe(&mut self, msg: u32) {
        self.say(format!("sub {msg:08X}"));
    }
    fn unsubscribe(&mut self, msg: u32) {
        self.say(format!("unsub {msg:08X}"));
    }
    fn add_tickable(&mut self, p: u32) {
        self.say(format!("tick+ {p:X}"));
    }
    fn remove_tickable(&mut self) {
        self.say("tick-".into());
    }
    fn post_construction_message(&mut self, msg: u32, d: u8) {
        let day = self.day;
        self.say(format!("post {msg:08X} {d} day {day}"));
    }
    fn sim_date(&mut self) -> u32 {
        self.day
    }
    fn lot_building(&mut self, lot: &RefValue) -> Option<RefValue> {
        (!lot.is_null()).then(|| host_ref(2, CLSID_BUILDING_OCCUPANT))
    }
    fn building_bounds(&mut self, _: &RefValue) -> Bounds {
        self.bbox
    }
    fn altitude(&mut self, x: f32, z: f32) -> f32 {
        if x == 0.0 && z == 0.0 {
            self.corner.unwrap_or(100.0)
        } else {
            100.0
        }
    }
    fn construction_days(&mut self, _: &RefValue) -> Option<u8> {
        self.days
    }
    fn building_completion(&mut self, _: &RefValue) -> f32 {
        self.completion
    }
    fn set_building_completion(&mut self, _: &RefValue, p: f32) {
        self.completion = p;
        self.say(format!("completion {p}"));
    }
    fn post_completion_changed(&mut self, _: &RefValue) {
        self.say("post 2977AE2F".into());
    }
    fn set_lot_state(&mut self, _: &RefValue, s: u32, a: u32, b: u32) {
        let day = self.day;
        self.say(format!("lot state {s} {a} {b} day {day}"));
    }
    fn lot_cell_rect(&mut self, _: &RefValue) -> [i32; 4] {
        [0, 0, 2, 1]
    }
    fn cell_usable(&mut self, _: i32, _: i32) -> bool {
        false
    }
    fn set_lot_texture(&mut self, x: i32, z: i32, i: u32) {
        self.say(format!("texture {x} {z} {i:08X}"));
    }
    fn cell_corner(&mut self, x: i32, z: i32) -> [f32; 3] {
        [x as f32 * 16.0, 0.0, z as f32 * 16.0]
    }
    fn crane_policy(&mut self, _: &RefValue) -> u8 {
        u8::from(self.crane)
    }
    fn create_prop(&mut self, type_id: u32) -> Option<RefValue> {
        Some(host_ref(u64::from(type_id), IID_PROP))
    }
    fn place_prop(&mut self, _: &RefValue, _: [f32; 3], _: u8) {
        self.say("prop placed".into());
    }
    fn render_detail(&mut self) -> i32 {
        0
    }
    fn create_automaton(&mut self, _: u32, _: [f32; 3], _: bool) -> Option<RefValue> {
        None
    }
    fn worker_direction(&mut self, _: &RefValue, _: [f32; 2]) {}
    fn occupant_base_shutdown(&mut self) {
        self.say("base shutdown".into());
    }
    fn release_texture_bindings(&mut self) {
        self.say("textures-".into());
    }
    fn free_model_buffers(&mut self) {
        self.say("buffers-".into());
    }
    fn release_crane_model(&mut self) {
        self.say("crane-".into());
    }
    fn remove_prop(&mut self, _: &RefValue) {
        self.say("prop-".into());
    }
    fn remove_automaton(&mut self, _: &RefValue) {
        self.say("worker-".into());
    }
    fn prepare_render_state(&mut self, _: [[TextureBinding; 5]; 6], _: [f32; 14]) {}
    fn visibility_result(&mut self) -> bool {
        true
    }
    fn model_message_keys(&mut self) -> (u32, u32) {
        (1, 2)
    }
    fn create_effect(&mut self, _: &RefValue, r: EffectRequest) -> Option<RefValue> {
        if r.name.ends_with("dust") {
            self.say("dust".into());
            Some(host_ref(39, IID_EFFECT_PROP))
        } else {
            self.say("sparks 1".into());
            Some(host_ref(40, IID_EFFECT_PROP))
        }
    }
    fn effect_slot8(&mut self, _: &RefValue, _: bool) {}
    fn building_position(&mut self, _: &RefValue) -> [f32; 3] {
        [0., 100., 0.]
    }
    fn lot_developer_building_position(&mut self, _: &RefValue) -> Option<[f32; 3]> {
        Some([0., 100., 0.])
    }
    fn associate_prop(&mut self, _: &RefValue, _: &RefValue, _: bool) {}
    fn prop_position(&mut self, _: &RefValue) -> [f32; 3] {
        [0.; 3]
    }
    fn move_roof_prop(&mut self, _: &RefValue, _: [f32; 3], _: u32, _: u32, _: u32) {}
    fn automaton_position(&mut self, _: &RefValue) -> [f32; 3] {
        [0.; 3]
    }
    fn move_automaton(&mut self, _: &RefValue, _: [f32; 3]) {}
    fn translate_crane(&mut self, _: [f32; 3]) {}
    fn spark_slot8(&mut self, i: usize, v: u32) {
        self.say(format!("spark {i} slot8 {v}"));
    }
    fn place_spark(&mut self, i: usize, _: [f32; 3]) {
        self.say(format!("spark {i} placed"));
    }
    fn start_scaffold_removal(&mut self) {
        self.say(format!("roof props removed day {}", self.day));
        self.say("scaffold removal".into());
    }
    fn add_to_renderer(&mut self) {
        self.say("render+".into());
    }
    fn remove_from_renderer(&mut self) {
        self.say("render-".into());
    }
    fn remove_from_occupant_manager(&mut self) {
        self.say("occupant manager remove".into());
    }
    fn occupant_base_init(&mut self) {
        self.say("base init".into());
    }
    fn post_occupant_group_added(&mut self, g: u32) {
        self.say(format!("group+ {g:X}"));
    }
    fn pose_crane(&mut self, a: f32) {
        self.say(format!("pose {a}"));
    }
    fn create_crane(&mut self, r: CraneRequest) -> bool {
        let [x, _, z] = r.position;
        let t = CRANE_MODELS.iter().position(|&id| id == r.resource[2]).unwrap();
        self.say(format!("create crane {x} {z} {t}"));
        true
    }
}

fn statics() -> ConstructionStatics {
    let seeds = FixedSeeds {
        wall_clock: 1,
        process_clock: 1,
    };
    ConstructionStatics {
        tuning: shipped_tuning(),
        rng: RzRandom::new(12345, &seeds),
        crt: CrtRand::default(),
        cranes: [CraneSize::default(); 3],
        ground: vec![],
        roof: vec![],
        girder_plank: None,
        crane_post_half: None,
    }
}

/// Runs SetLot on day 0 then SimNewDay from `first` on; returns the host.
fn run(mut h: Host, first: u32) -> (Host, ConstructionOccupant) {
    let mut st = statics();
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, host_ref(1, CLSID_LOT));
    h.log.clear();
    for day in first..400 {
        h.day = day;
        // data2 of SimNewDay is not used; pass junk.
        c.do_message(&mut h, &mut st, MSG_SIM_NEW_DAY, 0xDEAD_BEEF);
        if c.rec.scalars.state == STATE_DONE {
            break;
        }
    }
    (h, c)
}

fn days_of(h: &Host, prefix: &str) -> Vec<u32> {
    h.log
        .iter()
        .filter(|l| l.starts_with(prefix))
        .map(|l| l.rsplit(' ').next().unwrap().parse().unwrap())
        .collect()
}

#[test]
fn test21_construction_schedule() {
    let (h, c) = run(Host::new(110.0, None, None), 1);
    assert_eq!(days_of(&h, "post 49E7B1E2"), vec![1, 8, 15, 22]);
    assert_eq!(days_of(&h, "roof props removed"), vec![29]);
    assert_eq!(days_of(&h, "lot state 3 0 0"), vec![30]);
    assert_eq!(c.rec.scalars.state, STATE_DONE);
    let (h, _) = run(Host::new(110.0, None, None), 0);
    assert_eq!(days_of(&h, "lot state 3 0 0"), vec![29]);
    let cases: [(f32, Option<f32>, Option<u8>, u32); 8] = [
        (110.0, Some(99.0), None, 30),
        (110.0, Some(99.5), None, 30),
        (110.0, None, Some(0), 10),
        (110.0, None, Some(255), 254),
        (160.0, None, None, 42),
        (100.25, None, None, 32),
        (103.125, None, None, 30),
        (103.0, None, None, 31),
    ];
    for (my, corner, d, done) in cases {
        let (h, _) = run(Host::new(my, corner, d), 1);
        assert_eq!(days_of(&h, "lot state 3 0 0"), vec![done], "{my} {corner:?} {d:?}");
    }
    let (h, _) = run(Host::new(110.0, Some(99.0), None), 0);
    assert_eq!(days_of(&h, "lot state 3 0 0"), vec![30]);
    let (h, _) = run(Host::new(110.0, None, Some(0)), 1);
    assert_eq!(days_of(&h, "post 49E7B1E2"), vec![1, 3, 5, 7]);
    assert_eq!(days_of(&h, "roof props removed"), vec![9]);
}

#[test]
fn set_lot_order_and_completion_order() {
    let mut h = Host::new(110.0, None, None);
    h.day = 5;
    h.completion = 0.5;
    let mut st = statics();
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, host_ref(1, CLSID_LOT));
    let l = h.log.join("|");
    assert!(l.starts_with(
        "base init|sub 66956814|sub A6B79602|sub 06CDB65B|sub 26D31EC1|tick+ 80|group+ 5007|dust|sparks 1|completion 0|post 2977AE2F|post 49E7B1E2 1 day 5|post C9E7B1E8 1 day 5|render+"
    ), "{l}");
    let s = c.rec.scalars;
    assert_eq!((s.t0, s.next_day, s.state, s.counter), (5, 5, 0, 0));
    assert_eq!(c.rec.effects.len(), 1);
    assert_eq!(c.scaffold_height.to_bits(), (100.001f32).to_bits());
    assert!(!c.rec.has_crane && s.crane_state == 0);
    // Completion order on the last day.
    let mut last = Vec::new();
    for day in 6..60 {
        h.day = day;
        h.log.clear();
        c.do_daily_sim(&mut h, &mut st);
        if c.rec.scalars.state == STATE_DONE {
            last = h.log.clone();
            break;
        }
    }
    assert_eq!(
        last.join("|"),
        "completion 1|post 2977AE2F|render-|occupant manager remove|lot state 3 0 0 day 35|post C9E7B1E8 0 day 35|base shutdown|unsub 66956814|unsub A6B79602|unsub 06CDB65B|unsub 26D31EC1|tick-|textures-|buffers-|crane-|prop-|prop-"
    );
    assert!(c.rec.lot.is_null() && c.rec.building.is_null());
    // Done: further days change nothing.
    h.log.clear();
    h.day = 100;
    c.do_daily_sim(&mut h, &mut st);
    assert!(h.log.is_empty());
}

#[test]
fn no_building_means_no_schedule() {
    let mut h = Host::new(110.0, None, None);
    let mut st = statics();
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, RefValue::Null);
    assert_eq!(h.log.len(), 7); // Init only
    h.day = 10;
    c.do_daily_sim(&mut h, &mut st);
    assert_eq!(h.log.len(), 7);
}

#[test]
fn set_building_completion_posts_only_on_change() {
    let mut h = Host::new(110.0, None, None);
    let mut c = ConstructionOccupant::default();
    c.rec.building = host_ref(2, CLSID_BUILDING_OCCUPANT);
    c.set_building_completion(&mut h, 0.0);
    assert!(h.log.is_empty());
    c.set_building_completion(&mut h, 0.25);
    assert_eq!(h.log, vec!["completion 0.25".to_string(), "post 2977AE2F".to_string()]);
}

#[test]
fn test23_lot_texture() {
    let mut st = statics();
    let mut n = 0;
    let t = lot_texture(&mut st.rng, [1, 1, 2, 2], |_, _| {
        n += 1;
        true
    });
    assert_eq!(
        t,
        vec![
            (1, 1, 0xA990_C100),
            (1, 2, 0xA990_C400),
            (2, 1, 0xA990_C200),
            (2, 2, 0xA990_C400)
        ]
    );
    // A water / outside cell draws nothing.
    let mut st = statics();
    let t = lot_texture(&mut st.rng, [1, 1, 2, 2], |x, z| (x, z) != (1, 1));
    assert_eq!(t[0], (1, 2, 0xA990_C100));
}

#[test]
fn test24_spark_scatter() {
    let mut st = statics();
    let p = scatter_sparks(&mut st, &([0.0, 100.0, 0.0], [20.0, 110.0, 30.0]), 1);
    assert_eq!(p.len(), 1);
    assert_eq!(p[0][0].to_bits(), 0x4138_7420);
    assert_eq!(p[0][2], 0.0);
    assert_eq!(p[0][1].to_bits(), 0x42CC_C8DD);
    // Three rand() consumed: the next is the fourth of a fresh CRT.
    let mut fresh = CrtRand::default();
    for _ in 0..3 {
        fresh.rand();
    }
    assert_eq!(st.crt, fresh);
    assert_eq!(reset_iterator(&mut CrtRand::default(), 4), vec![0, 1, 3, 2]);
}

#[test]
fn spark_counts() {
    assert_eq!(spark_count(&([0.0; 3], [19.9, 5.0, 3.0])), 1);
    assert_eq!(spark_count(&([0.0; 3], [20.0, 5.0, 3.0])), 2);
    assert_eq!(spark_count(&([0.0; 3], [2.0, 5.0, 30.0])), 3);
    // y is excluded (ld §6.7.7, OP16).
    assert_eq!(spark_count(&([0.0; 3], [2.0, 30.0, 3.0])), 1);
}

#[test]
fn test25_crane_pause() {
    let mut st = statics();
    let mut s = ConstructionScalars {
        swing_min: 0.0,
        swing_max: 1.0,
        ..ConstructionScalars::default()
    };
    animate_crane(&mut s, 16, &mut st.rng, 6000.0);
    assert_eq!((s.crane_state, s.crane_duration, s.crane_acc), (1, 1048, 0));
    animate_crane(&mut s, 1000, &mut st.rng, 6000.0);
    assert_eq!(s.crane_state, 1);
    animate_crane(&mut s, 48, &mut st.rng, 6000.0);
    assert_eq!((s.crane_state, s.crane_duration), (2, 1048));
    // SetLot's initial values (ld §6.7.6): target min, previous max, acc 1,
    // duration 0, state 2: the first tick swings at once, from RDRU(mid, max).
    let mut s = ConstructionScalars {
        swing_min: 0.0,
        swing_max: 1.0,
        crane_target: 0.0,
        crane_previous: 1.0,
        crane_acc: 1,
        crane_duration: 0,
        crane_state: 2,
        ..ConstructionScalars::default()
    };
    animate_crane(&mut s, 0, &mut st.rng, 6000.0);
    assert_eq!(s.crane_state, 0);
    assert!(s.crane_target >= 0.5 && s.crane_target <= 1.0);
    assert_eq!(s.crane_previous, 0.0);
    assert_eq!(s.crane_duration, crane_duration(6000.0, f64::from(s.crane_target)));
}

#[test]
fn crane_duration_uses_tunable_t_and_k() {
    assert_eq!(f32::from_bits(CRANE_K_BITS), 0.159_154_94);
    assert_eq!(crane_duration(6000.0, std::f64::consts::PI), 3000);
    assert_eq!(crane_duration(6000.0, 0.5), 477);
    assert_eq!(crane_duration(12000.0, 0.5), 955);
    assert_eq!(crane_duration(6000.0, 0.0), 0);
}

#[test]
fn animation_tick_message_divides_by_1000() {
    let mut h = Host::new(110.0, None, None);
    h.crane = true;
    let mut st = statics();
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, host_ref(1, CLSID_LOT));
    c.rec.has_crane = true;
    c.rec.scalars.crane_state = 2;
    assert!(c.rec.has_crane);
    assert!(c.do_message(&mut h, &mut st, MSG_ANIMATION_TICK, 16_700));
    assert_eq!(c.rec.scalars.ms_acc, 16);
    assert_eq!(c.rec.scalars.crane_state, 0);
    assert!(!SUBSCRIPTIONS.contains(&MSG_ANIMATION_TICK));
}

#[test]
fn set_lot_crane_initial_values() {
    let mut h = Host::new(120.0, None, None);
    h.crane = true;
    let mut st = statics();
    st.cranes[2] = CraneSize {
        width: 5.0,
        reach: 2.0,
        back: 0.25,
    };
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, host_ref(1, CLSID_LOT));
    let s = c.rec.scalars;
    assert_eq!(
        (s.crane_type, s.swing_min, s.swing_max, s.crane_target, s.crane_previous),
        (2, s.swing_min, s.swing_max, s.swing_min, s.swing_max)
    );
    assert!(s.swing_min < s.swing_max);
    assert_eq!((s.crane_acc, s.crane_duration, s.crane_state), (1, 0, 2));
    assert!(c.rec.has_crane);
    let l = h.log.join("|");
    assert!(l.contains("|sparks 1|pose "), "{l}");
    // RebuildModel's saved values: N, span N·segH, startY, scaffold box #7.
    let g = c.geometry.unwrap();
    assert_eq!((s.segments, s.start_y), (g.segments, g.start_y));
    assert_eq!(
        s.span,
        r24(f64::from(g.segments) * f64::from(st.tuning.segment_height)) as f32
    );
    assert_eq!(c.rec.local_bounds, g.scaffold);
    assert_eq!(c.position(), [5.0, 110.0, 5.0]);
}

#[test]
fn init_adds_occupant_group_once() {
    let mut h = Host::new(110.0, None, None);
    let mut c = ConstructionOccupant::default();
    c.rec.base.holder.own.add(
        PROP_OCCUPANT_GROUPS,
        Variant::array(mem::UINT32, vec![Elem::U32(0x1234)]),
    );
    assert!(c.init(&mut h));
    assert_eq!(
        h.log.join("|"),
        "base init|sub 66956814|sub A6B79602|sub 06CDB65B|sub 26D31EC1|tick+ 80|group+ 5007"
    );
    let groups = c.rec.base.holder.get(PROP_OCCUPANT_GROUPS).unwrap().items();
    assert_eq!(groups, vec![Elem::U32(0x1234), Elem::U32(0x5007)]);
    // Already initialised: nothing; already present: no second entry or post.
    h.log.clear();
    assert!(c.init(&mut h));
    assert!(!c.add_occupant_group(&mut h, OCCUPANT_GROUP_CONSTRUCTION));
    assert!(h.log.is_empty());
}

#[test]
fn loaded_construction_resumes_schedule() {
    let (href, _) = run(Host::new(110.0, None, Some(0)), 1);
    let want = days_of(&href, "post 49E7B1E2");
    let done = days_of(&href, "lot state 3 0 0");
    let mut h = Host::new(110.0, None, Some(0));
    let mut st = statics();
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, host_ref(1, CLSID_LOT));
    h.log.clear();
    for day in 1..5 {
        h.day = day;
        c.do_message(&mut h, &mut st, MSG_SIM_NEW_DAY, 0);
    }
    assert_eq!(c.rec.scalars.state, 2);
    let bytes = write_obj(&RefValue::Object(RecordObject::new(c.rec.clone())));
    let (o, ok, rem) = read_obj(CLSID_CONSTRUCTION_OCCUPANT, &bytes);
    assert!(ok && rem == 0);
    let r: ConstructionRecord = rec(&o);
    assert_eq!(r.scalars, c.rec.scalars);
    assert_eq!(r.effects.len(), 1);
    let mut c2 = ConstructionOccupant::from_record(r);
    let mut h2 = Host::new(110.0, None, Some(0));
    h2.log = h.log.clone();
    // Read before Init: the loaded base already carries 0x5007.
    c2.init(&mut h2);
    assert!(!h2.log.iter().any(|l| l.starts_with("group+")));
    for day in 5..400 {
        h2.day = day;
        c2.do_message(&mut h2, &mut st, MSG_SIM_NEW_DAY, 0);
        if c2.rec.scalars.state == STATE_DONE {
            break;
        }
    }
    assert_eq!(days_of(&h2, "post 49E7B1E2"), want);
    assert_eq!(days_of(&h2, "lot state 3 0 0"), done);
}

#[test]
fn refresh_after_load_recreates_crane_once() {
    let mut h = Host::new(110.0, None, None);
    h.crane = true;
    let mut st = statics();
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, host_ref(1, CLSID_LOT));
    c.rec.has_crane = true;
    c.rec.crane_position = [1.0, 2.0, 3.0];
    c.rec.scalars.crane_type = 2;
    c.rec.scalars.crane_state = 2;
    c.rec.scalars.crane_acc = 1;
    c.rec.scalars.state = 2;
    let bytes = write_obj(&RefValue::Object(RecordObject::new(c.rec.clone())));
    let (o, ok, _) = read_obj(CLSID_CONSTRUCTION_OCCUPANT, &bytes);
    assert!(ok);
    let mut c2 = ConstructionOccupant::from_record(rec(&o));
    c2.init(&mut h);
    assert!(!c2.rec.has_crane && c2.rec.loaded_has_crane && c2.rec.refresh_pending);
    // The tick before the refresh does not animate: no crane model yet.
    c2.do_message(&mut h, &mut st, MSG_ANIMATION_TICK, 5_000);
    assert_eq!((c2.rec.scalars.ms_acc, c2.rec.scalars.crane_state), (5, 2));
    h.log.clear();
    assert!(c2.do_message(&mut h, &mut st, MSG_VIEW_REFRESH, 0));
    let seen: Vec<&String> = h
        .log
        .iter()
        .filter(|l| l.starts_with("create crane") || l.starts_with("render"))
        .collect();
    assert_eq!(seen, vec!["create crane 1 3 2", "render+"]);
    let s = c2.rec.scalars;
    // DoAnimationTick(0) with state 2 shown as 1: the crane swung (acc 1 ≥ 0).
    assert_eq!((s.state, s.ms_acc, s.crane_state), (2, 5, 0));
    assert!(c2.rec.has_crane && !c2.rec.refresh_pending);
    h.log.clear();
    c2.do_message(&mut h, &mut st, MSG_VIEW_REFRESH, 0);
    assert!(h.log.is_empty());
}

#[test]
fn live_refs_released_by_set_lot_read_and_shutdown() {
    let a = RecordObject::new(sample());
    let b = RecordObject::new(sample());
    let mut h = Host::new(110.0, None, None);
    let mut st = statics();
    let mut c = ConstructionOccupant::default();
    c.set_lot(&mut h, &mut st, RefValue::Object(a.clone()));
    assert_eq!(std::rc::Rc::strong_count(&a), 2);
    c.set_lot(&mut h, &mut st, RefValue::Object(b.clone()));
    assert_eq!((std::rc::Rc::strong_count(&a), std::rc::Rc::strong_count(&b)), (1, 2));
    c.shutdown(&mut h);
    assert_eq!(std::rc::Rc::strong_count(&b), 1);
    assert!(c.rec.effects.is_empty() && c.rec.building.is_null());
}
