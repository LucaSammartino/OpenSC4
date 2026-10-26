use super::pylon::*;
use crate::{
    automata::OccupantBase,
    stream::{ByteReader, ByteWriter, InStream, OutStream, RefValue, SaveRegistry},
};
use std::{cell::RefCell, rc::Rc};

pub(super) fn reference(token: u64) -> RefValue {
    RefValue::Host {
        token,
        class_id: 0x1234,
    }
}
pub(super) fn token(value: &RefValue) -> u64 {
    match value {
        RefValue::Host { token, .. } => *token,
        RefValue::Null => 0,
        _ => panic!("unexpected core ref"),
    }
}
pub(super) fn writer() -> ByteWriter {
    ByteWriter::with_registry(Rc::new(RefCell::new(SaveRegistry::new())))
}

/// Independent stream adapter records requested IIDs and deliberately resolves
/// references without imposing an owner-specific interface policy.
pub(super) struct Reader<'a> {
    pub inner: ByteReader<'a>,
    pub iids: Vec<u32>,
}
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            inner: ByteReader::new(bytes),
            iids: Vec::new(),
        }
    }
}
impl InStream for Reader<'_> {
    fn get_u8(&mut self) -> Option<u8> {
        self.inner.get_u8()
    }
    fn get_u16(&mut self) -> Option<u16> {
        self.inner.get_u16()
    }
    fn get_u32(&mut self) -> Option<u32> {
        self.inner.get_u32()
    }
    fn get_u64(&mut self) -> Option<u64> {
        self.inner.get_u64()
    }
    fn get_f32(&mut self) -> Option<f32> {
        self.inner.get_f32()
    }
    fn get_f64(&mut self) -> Option<f64> {
        self.inner.get_f64()
    }
    fn get_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        self.inner.get_bytes(n)
    }
    fn get_str(&mut self) -> Option<Vec<u8>> {
        self.inner.get_str()
    }
    fn get_ref(&mut self) -> Option<RefValue> {
        let id = self.get_u32()?;
        if id == 0 {
            return Some(RefValue::Null);
        }
        let class_id = self.get_u32()?;
        Some(RefValue::Host {
            token: u64::from(id) * 100,
            class_id,
        })
    }
    fn get_ref_as(&mut self, iid: u32) -> Option<RefValue> {
        self.iids.push(iid);
        self.get_ref()
    }
    fn skip(&mut self, n: usize) -> bool {
        self.inner.skip(n)
    }
    fn error(&self) -> i32 {
        self.inner.error()
    }
    fn set_error(&mut self) {
        self.inner.set_error();
    }
}

pub(super) struct Host {
    pub log: Vec<String>,
    pub renderer: bool,
    pub manager: bool,
    pub insert_manager: bool,
    pub prepare: bool,
    pub insert_renderer: bool,
    pub packs: Vec<RenderStateArgs>,
    pub bitmaps: bool,
    pub portrait: bool,
    pub source_pixel: u32,
    pub pixels: std::collections::HashMap<u32, u32>,
    pub measurements: [i32; 2],
    pub bitmap_acquisition: bool,
    pub bitmap_lock: bool,
    pub quads: Vec<(u32, [BillboardVertex; 4])>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            log: Vec::new(),
            renderer: true,
            manager: true,
            insert_manager: true,
            prepare: true,
            insert_renderer: true,
            packs: Vec::new(),
            bitmaps: false,
            portrait: false,
            source_pixel: 0x0011_2233,
            pixels: Default::default(),
            measurements: [20, 10],
            bitmap_acquisition: true,
            bitmap_lock: true,
            quads: Vec::new(),
        }
    }
}
impl References for Host {
    fn add_ref(&mut self, v: &RefValue) {
        self.log.push(format!("add{}", token(v)));
    }
    fn release(&mut self, v: &RefValue) {
        self.log.push(format!("release{}", token(v)));
    }
}
impl PylonHost for Host {
    fn occupant_init(&mut self, _: &mut OccupantBase) {
        self.log.push("init".into());
    }
    fn occupant_shutdown(&mut self, _: &mut OccupantBase) {
        self.log.push("shutdown".into());
    }
    fn message(&mut self, id: u32, subscribe: bool) {
        self.log.push(format!("message{id:x}/{subscribe}"));
    }
    fn animation(&mut self, class: u32, iid: u32, tick: u32, subscribe: bool) {
        assert_eq!(
            (class, iid, tick),
            (ANIMATION_TICK_MANAGER_CLASS, ANIMATION_TICK_MANAGER_IID, 0x80)
        );
        self.log.push(format!("tick/{subscribe}"));
    }
    fn renderer_exists(&self) -> bool {
        self.renderer
    }
    fn renderer_probe(&mut self) {
        self.log.push("probe".into());
    }
    fn pack(&mut self, args: RenderStateArgs) -> u64 {
        self.packs.push(args);
        self.packs.len() as u64
    }
    fn city_manager(&mut self) -> bool {
        self.log.push("city".into());
        self.manager
    }
    fn manager_insert(&mut self) -> bool {
        self.log.push("manager+".into());
        self.insert_manager
    }
    fn manager_remove(&mut self) {
        self.log.push("manager-".into());
    }
    fn prepare_instance(&mut self, arg: u32) -> bool {
        assert_eq!(arg, 4);
        self.log.push("prepare".into());
        self.prepare
    }
    fn renderer_insert(&mut self) -> bool {
        self.log.push("renderer+".into());
        self.insert_renderer
    }
    fn renderer_remove(&mut self) {
        self.log.push("renderer-".into());
    }
    fn base_visibility(&mut self, visible: bool, arg: u32, _: &mut u8) {
        self.log.push(format!("visibility/{visible}/{arg}"));
    }
    fn release_texture(&mut self, id: u32) {
        self.log.push(format!("texture{id}"));
    }
    fn texture_manager_exists(&self) -> bool {
        true
    }
}

#[test]
fn pylon_record_bytes_one_transform_signed_enums_and_iids() {
    let mut r = PylonRecord::new([-1, -128, 127]);
    r.call_number = 0x1234;
    r.source = reference(1);
    r.owner = reference(2);
    r.bitmap = -7;
    r.text = b"hello".to_vec();
    r.drag = 0x80;
    r.transform.mask = 1;
    r.transform.translation = [4.0, 5.0, 6.0];
    let mut w = writer();
    assert!(r.write(&mut w));
    let bytes = w.into_bytes();
    let mut peek = ByteReader::new(&bytes);
    assert_eq!(peek.get_u16(), Some(5));
    OccupantBase::read(&mut peek).unwrap();
    let body = peek.position();
    assert_eq!(&bytes[body..body + 7], &[1, 255, 128, 127, 0, 0x34, 0x12]);
    let mut back = PylonRecord::new([99; 3]);
    let mut h = Host::default();
    let mut input = Reader::new(&bytes);
    assert!(back.read_into(&mut input, &mut h));
    assert_eq!(input.inner.remaining(), 0);
    assert_eq!(input.iids, [0x26BB_767E, 1]);
    assert_eq!(back.states, [-1, -128, 127]);
    assert_eq!(back.call_number, 0x1234);
    assert_eq!(back.transform.translation, [4.0, 5.0, 6.0]);
    assert_eq!(back.drag, 1);
    assert_eq!((back.bitmap, back.text), (-7, b"hello".to_vec()));
    // Force failure in the reached body. Both old refs are still cleared;
    // a base/version failure clears neither.
    for end in [body + 7, bytes.len() - 1] {
        let mut back = PylonRecord::new([0; 3]);
        back.source = reference(8);
        back.owner = reference(9);
        let mut h = Host::default();
        assert!(!back.read_into(&mut Reader::new(&bytes[..end]), &mut h));
        assert_eq!(h.log, ["release8", "release9"]);
    }
    for bytes in [&[2, 0][..], &bytes[..3]] {
        let mut back = PylonRecord::new([0; 3]);
        back.source = reference(8);
        let mut h = Host::default();
        assert!(!back.read_into(&mut Reader::new(bytes), &mut h));
        assert!(h.log.is_empty());
    }
}

#[test]
fn pylon_v3_v4_future_and_tail_preservation() {
    for version in [3u16, 4, 6] {
        let mut w = ByteWriter::new();
        w.put_u16(version);
        OccupantBase::new(0xA9B6_D7DE).write(&mut w);
        w.put_bytes(&[0, 255, 128, 127, 0]);
        w.put_u16(11);
        w.put_u32(0);
        if version == 3 {
            for v in super::xform::IDENTITY_MATRIX {
                w.put_f32(v);
            }
            for v in [4.0, 5.0, 6.0, 1.0] {
                w.put_f32(v);
            }
            w.put_u32(0);
        } else {
            w.put_u8(1);
            for v in [4.0, 5.0, 6.0] {
                w.put_f32(v);
            }
        }
        w.put_u8(0x80);
        if version >= 5 {
            w.put_u32(7);
            w.put_str(b"new");
            w.put_u32(0);
        }
        let bytes = w.into_bytes();
        let mut r = PylonRecord::new([0; 3]);
        r.bitmap = 9;
        r.text = b"kept".to_vec();
        r.owner = reference(8);
        let mut h = Host::default();
        assert!(r.read_into(&mut Reader::new(&bytes), &mut h));
        assert_eq!(r.transform.translation, [4.0, 5.0, 6.0]);
        assert_eq!(r.drag, 1);
        if version < 5 {
            assert_eq!(r.bitmap, 9);
            assert_eq!(r.text, b"kept");
            assert_eq!(token(&r.owner), 8);
            assert!(h.log.is_empty());
        } else {
            assert_eq!(r.bitmap, 7);
            assert_eq!(h.log, ["release8"]);
        }
    }
}

#[test]
fn pylon_setters_references_and_shared_blink_conformance_40_41() {
    let mut p = Pylon::new([2, 1, 2]);
    let mut q = Pylon::new([2, 4, 2]);
    let mut g = PylonGlobals::default();
    p.record.dirty = false;
    p.record.blink = true;
    p.set_ui_state(2);
    assert!(!p.record.dirty && p.record.blink);
    p.set_ui_state(300);
    assert!(p.record.dirty && !p.record.blink);
    for index in 1..3 {
        p.record.dirty = false;
        p.record.blink = true;
        let equal = p.record.states[index];
        p.set_state_for_test(index, equal);
        assert!(!p.record.dirty && p.record.blink);
        p.set_state_for_test(index, -400);
        assert_eq!(p.record.states[index], -400);
        assert!(p.record.dirty && !p.record.blink);
    }
    p.record.dirty = false;
    p.set_bitmap_info(0, b"");
    assert!(p.record.dirty);
    p.set_call_number(0xFFFF);
    assert_eq!(p.get_call_number(), 0xFFFF);
    p.set_drag_state(7);
    assert_eq!(p.get_drag_state(), 7);
    let mut h = Host::default();
    p.record.dirty = false;
    p.record.blink = true;
    p.set_source_occupant(reference(1), &mut h);
    p.set_source_occupant(reference(2), &mut h);
    p.set_source_occupant(reference(2), &mut h);
    p.set_source_occupant(RefValue::Null, &mut h);
    p.set_owner(reference(1), &mut h);
    p.set_owner(reference(2), &mut h);
    p.set_owner(reference(2), &mut h);
    p.set_owner(RefValue::Null, &mut h);
    assert_eq!(
        h.log,
        ["add1", "add2", "release1", "release2", "add1", "add2", "release1", "release2"]
    );
    assert!(!p.record.dirty && p.record.blink);
    p.record.blink = false;
    p.set_dispatch_state(1);
    p.record.dirty = false;
    p.animation_tick(300, &mut g);
    q.animation_tick(250, &mut g);
    assert_eq!(g.blink_accumulator, 0);
    assert!(!p.record.blink && q.record.blink && q.record.dirty);
    p.animation_tick(1200, &mut g);
    assert!(p.record.blink);
    assert_eq!(g.blink_accumulator, 0);
    p.set_dispatch_state(2);
    p.animation_tick(500, &mut g);
    assert_eq!(g.blink_accumulator, 0);
    p.set_dispatch_state(1);
    g.blink_accumulator = 0xFFFF_FFF0;
    p.animation_tick(32, &mut g);
    assert_eq!(g.blink_accumulator, 16);
    assert!(!p.record.blink);
}
impl Pylon {
    fn set_state_for_test(&mut self, index: usize, value: i32) {
        match index {
            1 => self.set_dispatch_state(value),
            2 => self.set_dispatch_type(value),
            _ => unreachable!(),
        }
    }
}

#[test]
fn pylon_lifecycle_packing_and_all_placement_edges() {
    let mut p = Pylon::new([0; 3]);
    let mut h = Host::default();
    let mut g = PylonGlobals::default();
    assert_eq!(p.auxiliary, [[1.0, 1.0, 1.0, 0.0, 0.0, 0.0]; 2]);
    assert!(p.init(&mut g, &mut h));
    assert_eq!(h.packs, PYLON_RENDER_STATES);
    let count = h.log.len();
    assert!(p.init(&mut g, &mut h));
    assert_eq!(h.log.len(), count);
    p.placement_flags = 0x80;
    h.log.clear();
    assert!(p.place_in_renderer(true, &mut h));
    assert_eq!(h.log, ["city", "city", "manager+", "prepare", "renderer+"]);
    assert_eq!(p.placement_flags, 0x83);
    h.log.clear();
    assert!(p.place_in_manager(false, &mut h));
    assert_eq!(h.log, ["city", "renderer-", "manager-"]);
    assert_eq!(p.placement_flags, 0x80);
    h.insert_renderer = false;
    assert!(!p.place_in_renderer(true, &mut h));
    assert_eq!(p.placement_flags, 0x81);
    p.placement_flags = 0x83;
    h.renderer = false;
    assert!(!p.place_in_renderer(false, &mut h));
    assert_eq!(p.placement_flags, 0x83);
    assert!(p.place_in_manager(false, &mut h));
    assert_eq!(p.placement_flags, 0x80);
    h.manager = false;
    assert!(p.place_in_manager(false, &mut h));
    assert!(p.place_in_renderer(false, &mut h));
    assert!(!p.place_in_manager(true, &mut h));
    h.renderer = true;
    assert!(!p.place_in_renderer(true, &mut h));
    h.manager = true;
    h.insert_manager = false;
    assert!(!p.place_in_manager(true, &mut h));
    h.insert_manager = true;
    h.prepare = false;
    assert!(!p.place_in_renderer(true, &mut h));
    assert_eq!(p.placement_flags, 0x81);
    h.prepare = true;
    h.insert_renderer = true;
    assert!(p.place_in_renderer(true, &mut h));
    assert!(p.place_in_renderer(false, &mut h));
    assert_eq!(p.placement_flags, 0x81);
    p.record.source = reference(1);
    p.record.owner = reference(2);
    p.mysim_texture = reference(3);
    p.texture_binding = reference(4);
    p.texture = 9;
    h.log.clear();
    assert!(p.shutdown(&mut h));
    assert_eq!(
        &h.log[h.log.len() - 4..],
        ["release3", "release4", "release1", "release2"]
    );
    assert!(h.log.iter().any(|s| s == "tick/false"));
    let count = h.log.len();
    assert!(p.shutdown(&mut h));
    assert_eq!(h.log.len(), count);
    p.initialized = true;
    p.texture = 9;
    h.log.clear();
    p.shutdown(&mut h);
    assert!(h.log.iter().any(|s| s == "texture9"));
}

#[test]
fn pylon_position_visibility_and_model_refresh_conformance_42() {
    let mut p = Pylon::new([0; 3]);
    let mut h = Host::default();
    assert_eq!(p.set_position(None, &mut h), Ok(()));
    assert!(h.log.is_empty());
    p.record.transform.counter = 255;
    p.record.transform.mask = 0x80;
    assert_eq!(p.set_position(Some([4.0, 5.0, 6.0]), &mut h), Ok(()));
    assert_eq!((p.record.transform.counter, p.record.transform.mask), (1, 0x85));
    assert_eq!(
        h.log,
        [
            "city",
            "city",
            "texture7ab50e44/1abe787d/ea3ee100",
            "city",
            "manager+",
            "city",
            "city",
            "prepare",
            "renderer+",
            "city",
            "city"
        ]
    );
    let mut out = [0.0; 3];
    p.get_position(Some(&mut out));
    p.get_position(None);
    assert_eq!(out, [4.0, 5.0, 6.0]);
    h.log.clear();
    p.set_visibility(false, 7, &mut h);
    assert_eq!(h.log, ["visibility/false/7", "city", "renderer-"]);
    p.placement_flags = 0;
    h.log.clear();
    p.set_visibility(true, 8, &mut h);
    assert_eq!(h.log, ["visibility/true/8"]);
}

#[test]
fn pylon_constructor_conformance_45() {
    for retained in [0xA5A5_A5A5u32 as i32, 0x1357_9BDF] {
        let mut p = Pylon::fresh(retained);
        assert_eq!(p.record.states, [1, 0, retained]);
        p.record.dirty = false;
        p.record.blink = true;
        p.set_dispatch_type(4);
        assert_eq!(p.dispatch_type(), 4);
        assert!(p.record.dirty);
        assert!(!p.record.blink);
    }
}

#[test]
fn pylon_cached_bounds_and_transforms_conformance_46() {
    use super::xform::Xform;
    let mut p = Pylon::fresh(2);
    p.auxiliary = [[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], [7.0, 8.0, 9.0, 10.0, 11.0, 12.0]];
    let boxes = p.auxiliary;
    assert_eq!(p.bounds(), [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
    assert_eq!(p.world_bounds(), [[7.0, 8.0, 9.0], [10.0, 11.0, 12.0]]);
    let (mut min, mut max) = ([0.0; 3], [0.0; 3]);
    p.get_bounding_box(&mut min, &mut max);
    assert_eq!([min, max], p.world_bounds());
    assert_eq!(p.sort_value(), 0);
    p.texture = 0x1234_5678;
    assert_eq!(p.sort_value(), 0x1234_5678);
    let input = Xform {
        counter: 99,
        mask: 0xA7,
        matrix: [3.0; 9],
        translation: [4.0, 5.0, 6.0],
        scale: 7.0,
    };
    p.record.transform.counter = 255;
    p.set_transform(&input);
    assert_eq!(p.record.transform, Xform { counter: 0, ..input });
    let mut output = Xform {
        counter: 255,
        ..Xform::default()
    };
    p.get_transform(&mut output);
    assert_eq!(output, Xform { counter: 0, ..input });
    output.counter = 17;
    p.get_transform(&mut output);
    assert_eq!(output.counter, 18);
    assert_eq!(p.record.transform.counter, 0);
    assert!(p.record.dirty);
    assert_eq!(p.auxiliary, boxes);
    p.record.dirty = false;
    p.set_transform(&input);
    assert!(!p.record.dirty);
    assert_eq!(p.auxiliary, boxes);
}

struct PickHost {
    boxes: Vec<[[f32; 3]; 2]>,
    hit: Option<f32>,
    pitch: f64,
    units: f64,
}
impl PylonPickHost for PickHost {
    fn viewport_units_to_world_units(&mut self, units: u32) -> f64 {
        assert_eq!(units, 50);
        self.units
    }
    fn view_pitch(&mut self) -> f64 {
        self.pitch
    }
    fn ray_box_distance(&mut self, bounds: [[f32; 3]; 2]) -> Option<f32> {
        self.boxes.push(bounds);
        self.hit
    }
}

#[test]
fn pylon_pick_conformance_47() {
    let mut p = Pylon::fresh(2);
    p.auxiliary[1] = [-1.0, -999.0, -3.0, 4.0, 100.0, 6.0];
    let mut h = PickHost {
        boxes: Vec::new(),
        hit: Some(8.0),
        pitch: 0.0,
        units: 20.0,
    };
    let mut distance = 10.0;
    p.set_ui_state(0);
    assert!(!p.pick(&mut distance, &mut h));
    assert!(h.boxes.is_empty());
    p.set_ui_state(1);
    assert!(p.pick(&mut distance, &mut h));
    assert_eq!(distance, 8.0);
    assert_eq!(h.boxes[0], [[-1.0, 80.0, -3.0], [4.0, 100.0, 6.0]]);
    assert!(!p.pick(&mut distance, &mut h));
    assert_eq!(distance, 8.0);
    for hit in [None, Some(9.0), Some(f32::NAN)] {
        h.hit = hit;
        assert!(!p.pick(&mut distance, &mut h));
        assert_eq!(distance, 8.0);
    }
    h.pitch = 0.73;
    p.pick(&mut distance, &mut h);
    let expected = (100.0 - 20.0 / f64::from(0.73f64.cos() as f32)) as f32;
    assert_eq!(h.boxes.last().unwrap()[0][1].to_bits(), expected.to_bits());
    assert_eq!(p.auxiliary[1][1], -999.0);
    assert!(p.record.dirty);
}

impl PylonTextureHost for Host {
    fn request_texture_binding(&mut self, key: [u32; 3]) -> RefValue {
        self.log.push(format!("texture{:x}/{:x}/{:x}", key[0], key[1], key[2]));
        // An existing mock primitive controls successful resource acquisition.
        if self.prepare {
            reference(77)
        } else {
            RefValue::Null
        }
    }
    fn binding_texture_id(&mut self, value: &RefValue) -> u32 {
        assert_eq!(token(value), 77);
        0x9876_5432
    }
}

#[test]
fn pylon_texture_replacement_conformance_48() {
    let mut p = Pylon::fresh(6);
    let mut h = Host::default();
    let offsets = [0, 0x10, 0x20, 0x30, 0x40, 0x50, 0x20];
    for (kind, base) in [
        (0, 0xEA3E_E100u32),
        (6, 0xEA3E_E100),
        (1, 0x1431_5E00),
        (5, 0x1431_5E00),
    ] {
        p.set_dispatch_type(kind);
        for (state, offset) in offsets.into_iter().enumerate() {
            p.set_dispatch_state(state as i32);
            h.log.clear();
            assert_eq!(p.rebuild_current_texture(&mut h), Ok(true));
            assert!(h.log.contains(&format!("texture7ab50e44/1abe787d/{:x}", base + offset)));
            assert_eq!(p.texture, 0x9876_5432);
        }
    }
    p.set_dispatch_type(2);
    p.set_dispatch_state(-1); // This type never indexes a table.
    h.log.clear();
    assert_eq!(p.rebuild_current_texture(&mut h), Ok(true));
    assert_eq!(h.log, ["release77", "texture7ab50e44/1abe787d/0"]);
    p.texture_binding = RefValue::Null;
    p.texture = 7;
    h.prepare = false;
    h.log.clear();
    assert_eq!(p.rebuild_current_texture(&mut h), Ok(true));
    assert_eq!(p.texture, 7);
    assert_eq!(h.log, ["texture7", "texture7ab50e44/1abe787d/0"]);
    p.texture = 0;
    assert_eq!(p.rebuild_current_texture(&mut h), Ok(false));
    p.texture_binding = reference(33);
    p.texture = 7;
    h.log.clear();
    assert_eq!(p.rebuild_current_texture(&mut h), Ok(true));
    assert_eq!(h.log, ["release33", "texture7ab50e44/1abe787d/0"]);
    p.set_dispatch_state(0);
    p.set_dispatch_type(4);
    assert_eq!(p.rebuild_current_texture(&mut h), Ok(false));
    p.set_dispatch_type(0);
    for state in [-1, 7] {
        p.set_dispatch_state(state);
        assert_eq!(
            p.rebuild_current_texture(&mut h),
            Err(PylonUnsupported::TextureStateOutsideTable)
        );
    }
}
