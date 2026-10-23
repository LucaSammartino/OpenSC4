// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec fixtures use primitive foreign inputs, not captured native outputs.
use super::*;
use crate::effects::particles::{EmitterDesc, ParticleSimulator};
use crate::rigid_body_2d::simulator::*;
use crate::rng::{FixedSeeds, RzRandom};
use std::sync::Arc;

#[derive(Default)]
struct Foreign {
    calls: Vec<String>,
    terrain: bool,
    created: u32,
}
impl geometry::Resources for Foreign {
    fn retain(&mut self, r: u64) {
        self.calls.push(format!("retain:{r}"));
    }
    fn release(&mut self, r: u64) {
        self.calls.push(format!("release:{r}"));
    }
    fn group_count(&mut self, _: u64) -> usize {
        2
    }
    fn group(&mut self, _: u64, i: usize) -> u64 {
        i as u64
    }
    fn material(&mut self, source: u64, group: u64) -> (bool, Option<u64>) {
        (group == 1, Some(source + 100))
    }
    fn lookup(&mut self, key: [u32; 3], interface: u32) -> Option<u64> {
        assert_eq!(&key[..2], &[0x5ad0_e817, 0xbadb_57f1]);
        assert_eq!(interface, 0x9ad0_e7e3);
        self.calls.push("lookup".into());
        None
    }
    fn model_interface(&mut self, _: u64, interface: u32) -> Option<u64> {
        assert_eq!(interface, 0x26a6_8448);
        None
    }
    fn ordinary_shutdown(&mut self) {
        self.calls.push("shutdown".into());
    }
}
impl decal::Renderer for Foreign {
    fn terrain_available(&mut self) -> bool {
        self.calls.push("terrain".into());
        self.terrain
    }
    fn acquire_service(&mut self) -> bool {
        self.calls.push("service".into());
        true
    }
    fn release_service(&mut self) {
        self.calls.push("release-service".into());
    }
    fn create(&mut self) -> u32 {
        self.calls.push("create".into());
        self.created
    }
    fn remove(&mut self, handle: u32) {
        self.calls.push(format!("remove:{handle}"));
    }
    fn opacity(&mut self, _: u32, value: f32) {
        self.calls.push(format!("opacity:{value}"));
    }
    fn colour(&mut self, _: u32, _: [f32; 3]) {
        self.calls.push("colour".into());
    }
    fn layer(&mut self, _: u32, _: u8) {
        self.calls.push("layer".into());
    }
    fn retain_texture(&mut self, r: u64) {
        self.calls.push(format!("retain:{r}"));
    }
    fn release_texture(&mut self, r: u64) {
        self.calls.push(format!("release:{r}"));
    }
}
impl dynamic::Foreign for Foreign {
    fn retain_drawable(&mut self, r: u64) {
        self.calls.push(format!("retain-drawable:{r}"));
    }
    fn release_drawable(&mut self, r: u64) {
        self.calls.push(format!("release-drawable:{r}"));
    }
    fn static_occupants(&mut self, key: dynamic::SectorKey) -> Vec<u64> {
        self.calls.push(format!("collect:{key:?}"));
        Vec::new()
    }
    fn reset_shading(&mut self) {
        self.calls.push("reset-shading".into());
    }
}
fn body() -> PhysicalDescription {
    PhysicalDescription {
        position: [1., 2.],
        velocity: [3., 4.],
        height: 2.,
        height_velocity: 3.,
        angle: 0.,
        angular_velocity: 0.,
        acceleration: [2., 0.],
        friction: false,
    }
}
fn decal() -> decal::Decal {
    decal::Decal::new(decal::Description {
        flags: 0x48,
        resource: 0,
        layer: 0,
        lifetime_mode: 2,
        duration: 1.,
        rotation: vec![0.],
        size: vec![1.],
        opacity: vec![1.],
        colour: vec![[1.; 3]],
        aspect: vec![1.],
    })
}

#[test]
fn po01_through_po06_geometry_channels_and_reference_order() {
    let mut g = geometry::GeometryParticles::default();
    let mut f = Foreign::default();
    assert_eq!(g.set_float(8, &[2., 3., 5.]), Some(Ok(true)));
    assert_eq!(g.center, Some([2., 3., 5.]));
    assert_eq!(g.set_float(0x101, &[7.5]), Some(Ok(true)));
    assert_eq!(g.parameter_101, 7.5);
    assert_eq!(g.set_float(9, &[0.; 6]), Some(Ok(true)));
    assert!(f.calls.is_empty());
    g.set_source(Some(1), &mut f);
    f.calls.clear();
    g.set_resource(0, &mut f);
    assert!(f.calls.is_empty());
    assert_eq!(g.source, Some(1));
    g.set_source(Some(2), &mut f);
    assert_eq!(
        f.calls,
        ["retain:2", "release:1", "release:101", "retain:102"]
    );
    assert_eq!(g.textures, [102]);
    f.calls.clear();
    g.set_resource(9, &mut f);
    assert_eq!(g.source, Some(2));
    assert_eq!(f.calls, ["lookup"]);
    f.calls.clear();
    g.initialized = true;
    g.shutdown(&mut f);
    assert_eq!(f.calls, ["release:2", "shutdown"]);
    assert_eq!(g.source, None);
    f.calls.clear();
    g.shutdown(&mut f);
    assert!(f.calls.is_empty());
}
#[test]
fn geometry_unsettled_domains_are_explicit_and_not_ordinary_motion() {
    let mut g = geometry::GeometryParticles::default();
    let mut f = Foreign::default();
    g.set_source(Some(2), &mut f);
    assert_eq!(g.set_float(9, &[0.; 6]), Some(Err(Unsupported::PT8)));
    assert_eq!(g.set_float(2, &[1.]), None);
    assert_eq!(g.physics_is_ordinary(0.), Ok(true));
    assert_eq!(g.physics_is_ordinary(1.), Err(Unsupported::PT8));
    let mut countdown = 0.;
    assert_eq!(g.emit(false, &mut countdown, 1., 0), Err(Unsupported::PT8));
    g.built = true;
    assert_eq!(g.emit(false, &mut countdown, 1., 0), Ok(true));
}
#[test]
fn po07_through_po18_decal_lifetime_parameters_and_strict_end() {
    let mut d = decal();
    let mut f = Foreign {
        terrain: true,
        created: u32::MAX,
        ..Foreign::default()
    };
    assert!(!d.started);
    assert_eq!(d.handle, u32::MAX);
    assert_eq!(d.texture, None);
    assert!(!d.service_retained);
    d.started = true;
    d.start(0, &mut f).unwrap();
    assert!(f.calls.is_empty());
    d.started = false;
    d.start(1, &mut f).unwrap();
    assert!(f.calls.is_empty());
    f.terrain = false;
    d.start(0, &mut f).unwrap();
    assert!(!d.started);
    assert_eq!(d.handle, u32::MAX);
    f.terrain = true;
    f.calls.clear();
    d.start(0, &mut f).unwrap();
    assert_eq!(f.calls, ["terrain", "service", "create"]);
    assert!(!d.started);
    d.started = true;
    d.handle = 42;
    f.calls.clear();
    d.stop(2, &mut f);
    assert_eq!(f.calls, ["remove:42", "release-service"]);
    assert_eq!(d.handle, u32::MAX);
    f.calls.clear();
    d.stop(0, &mut f);
    assert!(f.calls.is_empty());
    assert!(d.set_float(0x102, &[9., 2.]));
    assert_eq!(d.external_size, 2.);
    assert!(d.set_float(12, &[1., 2., 4., 6.]));
    assert_eq!(d.center, [40., 64.]);
    assert_eq!(d.size_scale, 48.);
    assert_eq!(d.angle, 0.);
    assert_eq!(d.mapping, [3., 4. / 3.]);
    assert!(d.explicit);
    let before = d.clone();
    assert!(!d.set_float(99, &[]));
    assert_eq!(d, before);
    d.started = true;
    d.handle = 42;
    d.elapsed = Some(0.);
    d.variance = Some([1.; 3]);
    f.calls.clear();
    d.update(1., &mut f).unwrap();
    assert!(d.started);
    assert_eq!(d.elapsed, Some(1.));
    d.update(0.001, &mut f).unwrap();
    assert!(!d.started);
    assert_eq!(d.handle, u32::MAX);
    let elapsed = d.elapsed;
    f.calls.clear();
    d.update(1., &mut f).unwrap();
    assert_eq!(d.elapsed, elapsed);
    assert!(f.calls.is_empty());
}
#[test]
fn decal_copy_preserves_handle_and_pointer_replacement_is_balanced() {
    let mut d = decal();
    let mut f = Foreign::default();
    assert!(d.set_pointer(6, Some(1), &mut f));
    f.calls.clear();
    assert!(d.set_pointer(6, Some(2), &mut f));
    assert_eq!(f.calls, ["retain:2", "release:1"]);
    f.calls.clear();
    d.set_pointer(6, Some(2), &mut f);
    assert!(f.calls.is_empty());
    d.started = true;
    d.handle = 42;
    assert_eq!(
        d.set_description(&d.description.clone(), &mut f),
        Err(Unsupported::PT9)
    );
    assert!(d.started);
    assert_eq!(d.handle, 42);
    assert_eq!(f.calls, ["release:2"]);
}
#[test]
fn po19_through_po27_dynamic_messages_sectors_and_lifetimes() {
    let mut d = dynamic::DynamicParticles::new(16.);
    let mut f = Foreign::default();
    assert_eq!(d.sector_world_size, 64.);
    assert!(d.entries.is_empty());
    assert_eq!(
        d.do_message(7, || panic!("must not read payload"), [100.; 2]),
        Ok(false)
    );
    assert_eq!(
        d.do_message(dynamic::MESSAGE, || 10000, [100.; 2]),
        Ok(true)
    );
    assert_eq!(d.seconds, 0.01);
    assert!(d.steps.iter().all(|&s| s > 0. && s <= dynamic::MAX_STEP));
    d.tick(100000, [100.; 2]).unwrap();
    let steps = d.steps.clone();
    assert_eq!(d.seconds, 0.1);
    d.tick(200000, [100.; 2]).unwrap();
    assert_eq!(d.steps, steps);
    assert_eq!(d.seconds, 0.1);
    d.tick(0, [100.; 2]).unwrap();
    assert!(d.steps.is_empty());
    let a = (0, 0);
    let shared = (0, 1);
    let b = (1, 1);
    d.activate_sector(a, &mut f).unwrap();
    d.activate_sector(shared, &mut f).unwrap();
    f.calls.clear();
    d.move_neighborhood(&[a, shared], &[shared, b], &mut f)
        .unwrap();
    assert_eq!(d.sectors[&shared].references, 1);
    assert!(!d.sectors.contains_key(&a));
    assert_eq!(f.calls, ["collect:(1, 1)"]);
    f.calls.clear();
    d.activate_sector(shared, &mut f).unwrap();
    assert!(f.calls.is_empty());
    d.deactivate_sector(shared);
    assert!(d.sectors.contains_key(&shared));
    d.deactivate_sector(shared);
    assert!(!d.sectors.contains_key(&shared));
    f.calls.clear();
    assert_eq!(d.draw(&mut f), Ok(true));
    assert_eq!(f.calls, ["reset-shading"]);
    let id = d.add_particle(&body(), None, 1., 7, &mut f);
    f.calls.clear();
    assert_eq!(d.draw(&mut f), Ok(true));
    assert_eq!(f.calls, ["reset-shading"]);
    let entries = d.entries.clone();
    d.delete_particle(id + 10, &mut f).unwrap();
    assert_eq!(d.entries, entries);
    let body = d.entries[&id].body.unwrap();
    d.bodies.remove_object(body);
    d.tick(0, [100.; 2]).unwrap();
    assert!(d.entries.contains_key(&id));
    assert_eq!(d.entries[&id].body, None);
    d.delete_particle(id, &mut f).unwrap();
    assert!(d.entries.is_empty());
}
#[test]
fn po22_dynamic_live_zero_time_reaches_unsettled_transform_without_integrating() {
    let mut d = dynamic::DynamicParticles::new(16.);
    let mut f = Foreign::default();
    let id = d.add_particle(&body(), Some(8), 1., 9, &mut f);
    assert_eq!(d.tick(0, [100.; 2]), Err(Unsupported::PT10));
    assert!(d.steps.is_empty());
    assert!(d.entries[&id].body.is_some());
    d.delete_particle(id, &mut f).unwrap();
    assert_eq!(f.calls, ["retain-drawable:8", "release-drawable:8"]);
}
#[test]
fn po28_through_po31_noncontact_body_stores_dead_sweep_and_list_order() {
    assert!(primary_collision_sector(0x00040005, 0x00020005, 0x00040003));
    assert!(!primary_collision_sector(
        0x00020005, 0x00020005, 0x00040003
    ));
    let mut s = Simulator::default();
    let a = s.add_object(&body(), false);
    s.advance_time(0.5).unwrap();
    assert_eq!(s.objects[&a].description.position, [2.5, 4.]);
    assert_eq!(s.objects[&a].description.velocity, [4., 4.]);
    let b = s.add_object(&body(), true);
    assert_eq!(s.advance_time(0.5), Err(Unsupported::PT11));
    // Both lists have integrated before the explicit collision-domain boundary.
    assert_eq!(s.objects[&a].description.position, [4.5, 6.]);
    assert_eq!(s.objects[&b].description.position, [2.5, 4.]);
    s.joints.push([a, b]);
    s.sectors.insert(1, vec![a, b]);
    s.remove_object(a);
    assert!(s.joints.is_empty());
    assert_eq!(s.sectors[&1], [b]);
    assert!(!s.objects[&a].live);
    let before = s.objects[&a].clone();
    s.integrate_list(false, 0.5).unwrap();
    assert_eq!(s.objects[&a], before);
    s.remove_dead_objects();
    assert!(!s.objects.contains_key(&a));
}

#[derive(Default)]
struct Stream {
    values: std::collections::VecDeque<u32>,
    strings: std::collections::VecDeque<Vec<u8>>,
    writes: Vec<String>,
}
impl descriptions::Read for Stream {
    type Error = ();
    fn u8(&mut self) -> Result<u8, ()> {
        Ok(self.u32()? as u8)
    }
    fn u16(&mut self) -> Result<u16, ()> {
        Ok(self.u32()? as u16)
    }
    fn u32(&mut self) -> Result<u32, ()> {
        self.values.pop_front().ok_or(())
    }
    fn f32(&mut self) -> Result<f32, ()> {
        Ok(f32::from_bits(self.u32()?))
    }
    fn string(&mut self) -> Result<Vec<u8>, ()> {
        self.strings.pop_front().ok_or(())
    }
}
impl descriptions::Write for Stream {
    type Error = ();
    fn u16(&mut self, v: u16) -> Result<(), ()> {
        self.writes.push(format!("word:{v}"));
        Ok(())
    }
    fn u32(&mut self, v: u32) -> Result<(), ()> {
        self.writes.push(format!("dword:{v}"));
        Ok(())
    }
    fn f32(&mut self, v: f32) -> Result<(), ()> {
        self.writes.push(format!("float:{v}"));
        Ok(())
    }
    fn string(&mut self, v: &[u8]) -> Result<(), ()> {
        self.writes.push(format!("string:{v:?}"));
        Ok(())
    }
}
#[test]
fn po32_through_po35_sequence_primitive_order_and_factory_failure() {
    let mut d = descriptions::SequenceDescription::default();
    assert!(d.elements.is_empty());
    assert_eq!(d.flags, 0);
    for version in [1, 2] {
        let mut s = Stream {
            values: [version, 0, u32::MAX].into(),
            ..Stream::default()
        };
        d.read(&mut s).unwrap();
        assert_eq!(d.flags, 7);
        assert!(s.values.is_empty());
    }
    struct Allocation {
        calls: Vec<usize>,
    }
    impl descriptions::SequenceAllocator for Allocation {
        type Allocation = ();
        type Component = ();
        fn allocate(&mut self, bytes: usize) -> Option<()> {
            self.calls.push(bytes);
            None
        }
        fn construct(&mut self, _: (), _: Arc<descriptions::SequenceDescription>) {
            panic!("null allocation skips construction");
        }
    }
    let mut allocator = Allocation { calls: Vec::new() };
    assert_eq!(Arc::new(d).create(&mut allocator), None);
    assert_eq!(allocator.calls, [168]);
    let d = descriptions::SequenceDescription {
        elements: vec![descriptions::SequenceElement {
            first: 2.,
            second: 1.,
            name: vec![],
        }],
        flags: u32::MAX,
    };
    let mut s = Stream::default();
    d.write(&mut s).unwrap();
    assert_eq!(
        s.writes,
        [
            "word:1",
            "dword:1",
            "float:2",
            "float:1",
            "string:[]",
            "dword:4294967295"
        ]
    );
}
fn particle_description() -> descriptions::ParticleDescriptionBase {
    descriptions::ParticleDescriptionBase::new(Some(descriptions::ParticleGlobals {
        scalar: 0.,
        colour_curve: [1.; 3],
    }))
}
#[test]
fn po36_particle_value_copy_is_deep_and_unknown_globals_remain_unknown() {
    let unknown = descriptions::ParticleDescriptionBase::new(None);
    assert_eq!(unknown.value.get_scalar(0xe0), None);
    let mut a = particle_description();
    a.value.floats.insert(0x88, vec![2., 4.]);
    a.value.strings.insert(0x160, b"before".to_vec());
    let mut b = particle_description();
    b.assign(&a);
    a.value.floats.get_mut(&0x88).unwrap()[0] = 9.;
    a.value.strings.get_mut(&0x160).unwrap().clear();
    assert_eq!(b.value.floats[&0x88], [2., 4.]);
    assert_eq!(b.value.strings[&0x160], b"before");
}
#[test]
fn full_records_share_existing_emitter_and_preserve_init_rng_order() {
    let mut d = particle_description();
    d.value.words.insert(0, 4);
    d.value.floats.insert(0x88, vec![1.]);
    for at in [0x48, 0x4c] {
        d.value.scalar(at, 2.);
    }
    let motion = ordinary::Motion::new(&d).unwrap();
    let mut simulator = ParticleSimulator::new(EmitterDesc::new(0, 0., 0.));
    simulator.set_motion(motion).unwrap();
    let seeds = FixedSeeds {
        wall_clock: 0,
        process_clock: 7,
    };
    let mut rng = RzRandom::new(123, &seeds);
    let mut expected = rng.clone();
    simulator.start(0, &mut rng);
    simulator.update(0.5, &mut rng);
    for _ in 0..14 {
        expected.range_f64(0., 1.);
    }
    assert_eq!(rng.lcg_state(), expected.lcg_state());
    let record = simulator.records().next().unwrap();
    assert_eq!(record.velocity, [0., 2., 0.]);
    assert_eq!(record.factors, [1.; 7]);
    let old = record.position;
    simulator.update(0.5, &mut rng);
    let record = simulator.records().next().unwrap();
    assert_eq!(record.position[1], old[1] + 1.);
    assert_eq!(simulator.draw_slot(true, false), 17);
    simulator.remove_particle(0);
    assert_eq!(simulator.live_count(), 1);
    simulator.set_suppressed(true);
    simulator.update(0., &mut rng);
    assert_eq!(simulator.live_count(), 0);
}
#[test]
fn ordinary_identity_event_boundaries_and_open_physics() {
    use ordinary::ComponentKind::*;
    assert_eq!(ordinary::component_kind(0, f32::NAN), Ordinary);
    assert_eq!(ordinary::component_kind(1, -0.), ModelBased);
    assert_eq!(ordinary::component_kind(1, f32::NAN), GeometrySourced);
    let events = vec![
        (b"birth".to_vec(), 0.),
        (b"later".to_vec(), 1.),
        (b"last".to_vec(), 0.),
    ];
    assert_eq!(
        ordinary::birth_events(&events).collect::<Vec<_>>(),
        vec![b"birth".as_slice()]
    );
    assert_eq!(
        ordinary::crossing_events(&events, 0., 1.).collect::<Vec<_>>(),
        vec![b"birth".as_slice(), b"last".as_slice()]
    );
    assert_eq!(
        ordinary::crossing_events(&events, 1., 0.1).collect::<Vec<_>>(),
        vec![b"later".as_slice()]
    );
    let mut d = particle_description();
    d.value.words.insert(0, 0x100);
    let m = ordinary::Motion::new(&d).unwrap();
    assert_eq!(m.validate(), Err(Unsupported::PT1));
}

#[test]
fn particle_binary_readers_preserve_field_widths_and_consume_both_layouts() {
    for version in [1, 2, 99] {
        let mut s = Stream::default();
        s.values.push_back(0x87654321);
        if version != 1 {
            s.values.push_back(u32::MAX);
        }
        s.values.push_back(u32::MAX);
        let floats = |s: &mut Stream, count: usize, value: f32| {
            for _ in 0..count {
                s.values.push_back(value.to_bits());
            }
        };
        floats(&mut s, 3, 2.);
        s.values.push_back(3);
        floats(&mut s, 27, 4.);
        s.values.push_back(2);
        floats(&mut s, 2, 5.);
        s.values.push_back(1);
        floats(&mut s, 3, 6.);
        for _ in 0..4 {
            s.values.push_back(1);
            floats(&mut s, 1, 7.);
        }
        s.values.push_back(0xabcdef01);
        s.values.extend([2, 3]);
        floats(&mut s, 12, 8.);
        s.values.push_back(1);
        floats(&mut s, 6, 9.);
        floats(&mut s, 6, 10.);
        s.values.push_back(0);
        floats(&mut s, 10, 11.);
        s.strings.push_back(b"name".to_vec());
        s.values.extend([0x1234, 0xabcd]);
        floats(&mut s, 13, 12.);
        s.values.push_back(0);
        floats(&mut s, 1, 13.);
        s.values.push_back(0x11223344);
        s.values.push_back(1);
        floats(&mut s, 6, 14.);
        floats(&mut s, 1, 15.);
        if version != 1 {
            s.values.push_back(1);
            s.strings.push_back(b"event".to_vec());
            floats(&mut s, 1, 16.);
        }
        floats(&mut s, 2, 17.);
        s.values.extend([2, 0xdeadbeef, 0xfedcba98]);
        floats(&mut s, 2, 18.);
        s.values.push_back(1);
        floats(&mut s, 1, 19.);
        floats(&mut s, 1, 20.);
        let mut d = particle_description();
        d.read(version, &mut s).unwrap();
        assert!(s.values.is_empty());
        assert!(s.strings.is_empty());
        assert_eq!(d.value.words[&0], 0x87654321);
        assert_eq!(d.value.words[&4], if version == 1 { 0 } else { 255 });
        assert_eq!(d.value.words[&8], 0x7ff);
        assert_eq!(d.value.words[&0x16c], 0xabcd1234);
        assert_eq!(d.value.bytes[&0xd5], 3);
        assert_eq!(d.value.vectors[&0x108], [[9.; 3]; 2]);
        assert_eq!(d.value.vectors[&0x1b8], [[14.; 3]; 2]);
        assert_eq!(d.value.get_scalar(0x200), Some(20.));
        assert_eq!(d.value.lists[&0x1e0], [0xdeadbeef, 0xfedcba98]);
        assert_eq!(
            d.value.timed_effects.len(),
            if version == 1 { 0 } else { 1 }
        );
    }
}
#[test]
fn start_walk_turn_sea_priority_and_last_countdown_draw() {
    let mut d = particle_description();
    d.value.words.insert(0, 0x3800000);
    d.value.scalar(0x30, 1.);
    d.value.scalar(0x3c, 1.);
    d.value.scalar(0x20, 2.);
    d.value.scalar(0x24, 4.);
    let mut p = ParticleSimulator::new(EmitterDesc::new(0, 0., 0.));
    p.set_motion(ordinary::Motion::new(&d).unwrap()).unwrap();
    let seeds = FixedSeeds {
        wall_clock: 0,
        process_clock: 7,
    };
    let mut r = RzRandom::new(333, &seeds);
    let mut expected = r.clone();
    p.start_checked(0, &mut r, Some(1 | 2 | 8 | 4)).unwrap();
    for _ in 0..5 {
        expected.range_f64(0., 1.);
    }
    let countdown = expected.range_f64(2., 4.) as f32;
    assert_eq!(p.countdown(), countdown);
    assert_eq!(r.lcg_state(), expected.lcg_state());
    assert_eq!(p.update_checked(0., &mut r), Err(Unsupported::PT1));
}
#[test]
fn stepped_kill_uses_old_velocity_and_strict_city_bounds() {
    let mut d = particle_description();
    d.value.words.insert(0, 0x800);
    let mut m = ordinary::Motion::new(&d).unwrap();
    m.city_extent = Some([10., 10.]);
    let mut record = ordinary::Record {
        alive: true,
        position: [0.5, 0., 5.],
        velocity: [-1., 0., 0.],
        field_18: 0.,
        factors: [1.; 7],
    };
    m.update(&mut record, 0.5).unwrap();
    assert!(!record.alive);
    assert!(record.position[0] <= 0.);
    assert_eq!(record.position[2], 5.);
}

#[test]
fn effect_pool_refills_128_reuses_lifo_and_allocation_failure_is_empty() {
    use std::sync::Mutex;
    let pool = Arc::new(Mutex::new(pool::Pool::default()));
    assert!(pool::Pool::allocate(&pool, |bytes| {
        assert_eq!(bytes, 8712);
        false
    })
    .is_none());
    assert_eq!(pool.lock().unwrap().blocks(), 0);
    let mut slots = Vec::new();
    for _ in 0..129 {
        slots.push(
            pool::Pool::allocate(&pool, |bytes| {
                assert_eq!(bytes, 8712);
                true
            })
            .unwrap(),
        );
    }
    assert_eq!(pool.lock().unwrap().blocks(), 2);
    assert_eq!(pool.lock().unwrap().free_count(), 127);
    let id = slots.last().unwrap().id();
    drop(slots.pop());
    let reused = pool::Pool::allocate(&pool, |_| panic!("must reuse")).unwrap();
    assert_eq!(reused.id(), id);
    drop(reused);
    drop(slots);
    assert_eq!(pool.lock().unwrap().free_count(), 256);
}
#[test]
fn drawing_dispatch_self_lit_and_empty_no_draw_are_before_finish() {
    use crate::effects::particles::DrawCommand::*;
    let mut p = ParticleSimulator::new(EmitterDesc::new(5, 1., 2.));
    let seeds = FixedSeeds {
        wall_clock: 0,
        process_clock: 7,
    };
    let mut r = RzRandom::new(9, &seeds);
    let mut commands = Vec::new();
    p.start(0, &mut r);
    p.update_with_draw(0.1, &mut r, true, true, &mut |command| {
        commands.push(command)
    })
    .unwrap();
    assert_eq!(
        commands,
        [
            Slot(14),
            SelfLit(true),
            ModelTransform,
            RenderState,
            Slot(18),
            SelfLit(false)
        ]
    );
    p.stop(0);
    commands.clear();
    p.update_with_draw(3., &mut r, true, true, &mut |command| {
        commands.push(command)
    })
    .unwrap();
    assert_eq!(commands, [Slot(19)]);
    assert!(!p.is_active());
}
#[test]
fn geometry_description_retains_and_draw_groups_keep_material_order() {
    let mut g = geometry::GeometryParticles::default();
    let mut f = Foreign::default();
    assert_eq!(g.set_description(50, 0, 0, &mut f), Ok(false));
    assert!(f.calls.is_empty());
    assert_eq!(g.set_description(50, 1, 0, &mut f), Err(Unsupported::PT8));
    assert_eq!(f.calls, ["retain:50"]);
    struct Draw(Vec<String>);
    impl geometry::PrimitiveRenderer for Draw {
        fn bind_texture(&mut self, texture: u64) {
            self.0.push(format!("texture:{texture}"));
        }
        fn draw_pool(&mut self, index: usize) {
            self.0.push(format!("pool:{index}"));
        }
    }
    g.textures = vec![101, 102, 103];
    let mut draw = Draw(Vec::new());
    g.draw_groups(&[2, 0, 4], &mut draw);
    assert_eq!(draw.0, ["texture:101", "pool:0", "texture:103", "pool:2"]);
}

#[test]
fn ordinary_description_preparation_and_parameter_projection() {
    let mut d = particle_description();
    d.value.words.insert(0, 0x1000 | 0x100);
    d.value.scalar(0x18c, 1.);
    d.value.strings.insert(0x160, b"fire".to_vec());
    let mut m = ordinary::Motion::new(&d).unwrap();
    m.configure_source(5., [32., 64.]).unwrap();
    assert_eq!(m.emission_box, [[0., 4., 0.], [32., 6., 64.]]);
    m.configure_preferred([0.; 3]);
    assert_eq!(m.preferred, Some([1., 0., 0.]));
    let mut names = Vec::new();
    m.resolve_collision_effects(|name| {
        names.push(name.to_vec());
        names.len() as u8
    });
    assert_eq!(names, [b"fire_terrain".to_vec(), b"fire_water".to_vec()]);
    assert_eq!(m.collision_effects, Some([1, 2]));
    let mut p = ParticleSimulator::new(EmitterDesc::new(0, 0., 0.));
    assert_eq!(p.init_native_services(), Err(Unsupported::PT13));
    p.component_update(1.);
    assert_eq!(p.accumulated_time(), 0.);
    assert_eq!(p.set_float_parameter(99, &[]), Err(Unsupported::PT13));
    assert_eq!(p.set_float_parameter(2, &[2.]), Ok(()));
}
#[test]
fn geometry_uninitialized_teardown_releases_retained_source() {
    let mut g = geometry::GeometryParticles::default();
    let mut f = Foreign::default();
    g.set_source(Some(5), &mut f);
    f.calls.clear();
    g.teardown(&mut f);
    assert_eq!(f.calls, ["release:5", "release:105"]);
    assert!(g.source.is_none());
}
#[test]
fn decal_sampling_and_pending_handle_cleanup() {
    assert_eq!(decal::initial_phase(3), 1.);
    assert_eq!(decal::initial_phase(0), 0.);
    assert_eq!(decal::sample(&[2., 4.], 0.25), 2.5);
    assert_eq!(decal::sample_colour(&[[0.; 3], [4.; 3]], 0.25), [1.; 3]);
    let mut d = decal();
    let mut f = Foreign {
        terrain: true,
        created: 42,
        ..Foreign::default()
    };
    assert_eq!(d.start(0, &mut f), Err(Unsupported::PT9));
    assert!(!d.started);
    assert_eq!(d.handle, 42);
    f.calls.clear();
    assert_eq!(d.start(0, &mut f), Err(Unsupported::PT9));
    assert_eq!(f.calls, ["terrain"]);
    f.calls.clear();
    d.teardown(&mut f);
    assert_eq!(f.calls, ["remove:42", "release-service"]);
}
