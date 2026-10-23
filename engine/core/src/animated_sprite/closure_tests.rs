// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::Arc;

fn key(n: u32) -> ResourceKey {
    ResourceKey {
        resource_type: 9,
        group: 7,
        instance: n,
    }
}
fn raw() -> RawSpriteHolder {
    RawSpriteHolder {
        divisors: [1; 3],
        angle_limits: [f32::from_bits(0x40490fdb), f32::from_bits(0x40490fdb), 0., 0.],
        entries: vec![[0, 6, 0, 0, 16, 16, 0, 0]],
        scale: 1.,
    }
}
fn header() -> ModelHeader {
    ModelHeader {
        texture_key: key(99),
        zoom_type: 9,
        zoom_group: 7,
        instances: [0, 1, 2, 3, 4],
        scales: [2., 3., 4., 5., 6.],
        frame_count: 4,
    }
}
struct Service(Option<Arc<TextureBinding>>);
impl SpriteTextureService for Service {
    fn binding(&self, key: ResourceKey) -> Option<Arc<TextureBinding>> {
        assert_eq!(key, super::closure_tests::key(99));
        self.0.clone()
    }
}
struct Resources {
    fail: usize,
    binding: Option<Arc<TextureBinding>>,
    service: bool,
    calls: Vec<u32>,
    tables: Vec<Arc<RawSpriteHolder>>,
}
impl OwnerResourceServices for Resources {
    fn header(&mut self, supplied: ResourceKey, iid: u32, options: [u32; 2]) -> Option<Arc<ModelHeader>> {
        assert_eq!((supplied, iid, options), (key(10), MODEL_HEADER_IID, [0, 0]));
        Some(Arc::new(header()))
    }
    fn zoom(&mut self, supplied: ResourceKey, iid: u32, options: [u32; 2]) -> Option<Arc<RawSpriteHolder>> {
        assert_eq!(
            (supplied.resource_type, supplied.group, iid, options),
            (9, 7, SPRITE_TABLE_INTERFACE, [0, 0])
        );
        self.calls.push(supplied.instance);
        if supplied.instance as usize == self.fail {
            None
        } else {
            Some(self.tables[supplied.instance as usize].clone())
        }
    }
    fn texture_service(&mut self, service: u32, iid: u32) -> Option<Arc<dyn SpriteTextureService>> {
        assert_eq!((service, iid), (TEXTURE_SERVICE_ID, TEXTURE_SERVICE_IID));
        self.service
            .then(|| Arc::new(Service(self.binding.clone())) as Arc<dyn SpriteTextureService>)
    }
}
fn resources() -> Resources {
    Resources {
        fail: 5,
        binding: Some(Arc::new(TextureBinding {
            table: vec![TextureHandle(123)],
        })),
        service: true,
        calls: vec![],
        tables: (0..5).map(|_| Arc::new(raw())).collect(),
    }
}

#[test]
fn load_transaction_repeated_prefix_binding_and_counted_lifetimes() {
    let mut sprite = AnimatedSprite::<()>::default();
    let mut r = resources();
    assert_eq!(sprite.load(key(10), &mut r), Ok(true));
    let old: [Arc<RawSpriteHolder>; 5] = std::array::from_fn(|i| sprite.loaded.zooms[i].resource.clone().unwrap());
    let binding = sprite.loaded.binding.clone().unwrap();
    let before = sprite.state;
    sprite.frame_dirty = false;
    let mut r = resources();
    r.fail = 2;
    assert_eq!(sprite.load_from_resource(Some(&header()), &mut r), Ok(false));
    assert_eq!(r.calls, [0, 1, 2]);
    assert!(sprite.loaded.zooms[0].resource.is_none() && sprite.loaded.zooms[1].resource.is_none());
    for (i, old) in old.iter().enumerate().skip(2) {
        assert!(Arc::ptr_eq(sprite.loaded.zooms[i].resource.as_ref().unwrap(), old));
    }
    assert!(Arc::ptr_eq(sprite.loaded.binding.as_ref().unwrap(), &binding));
    assert_eq!(sprite.frame_count(), 4);
    assert_eq!(sprite.loaded.zooms[2].scale, Some(4.));
    assert!(sprite.loaded.zooms[0].metrics.is_some());
    assert_eq!(sprite.state, before);
    assert!(!sprite.frame_dirty);
    assert_eq!(Arc::strong_count(&r.tables[0]), 1);
    r.fail = 5;
    r.service = false;
    assert_eq!(sprite.load_from_resource(Some(&header()), &mut r), Ok(false));
    assert!(Arc::ptr_eq(sprite.loaded.binding.as_ref().unwrap(), &binding));
    assert!(sprite.loaded.zooms.iter().all(|z| z.resource.is_none()));
    r.service = true;
    r.binding = None;
    assert_eq!(sprite.load_from_resource(Some(&header()), &mut r), Ok(false));
    assert!(sprite.loaded.binding.is_none());
    r.fail = 0;
    assert_eq!(sprite.load_from_resource(None, &mut r), Ok(false));
    assert!(create_sprite_from_key(key(10), &mut r).is_err());
    let sprite_key = ResourceKey {
        resource_type: SPRITE_ANIMATION_TYPE,
        ..key(10)
    };
    // Factory forwarding of the full key is independently checked by Load.
    assert_eq!(creation_route(sprite_key), CreationRoute::SpriteInstance);
}

#[test]
fn load_all_failure_prefixes_preserve_unvisited_state_and_release_temporaries() {
    for fail in 0..=5 {
        let mut sprite = AnimatedSprite::<()>::default();
        let mut old = resources();
        assert_eq!(sprite.load(key(10), &mut old), Ok(true));
        let binding = sprite.loaded.binding.clone().unwrap();
        sprite.select_frame(3).unwrap();
        sprite.state.playback.position = 0.5;
        sprite.frame_dirty = false;
        let before = sprite.state;
        let mut r = resources();
        r.fail = fail;
        r.service = false;
        let mut h = header();
        h.frame_count = 300;
        h.scales = [11., 12., 13., 14., 15.];
        assert_eq!(sprite.load_from_resource(Some(&h), &mut r), Ok(false));
        assert_eq!(r.calls, (0..(fail + 1).min(5) as u32).collect::<Vec<_>>());
        assert_eq!(sprite.frame_count(), 300);
        assert_eq!(sprite.state, before);
        assert!(!sprite.frame_dirty);
        assert!(sprite.controller().is_none());
        assert!(Arc::ptr_eq(sprite.loaded.binding.as_ref().unwrap(), &binding));
        for i in 0..5 {
            let slot = &sprite.loaded.zooms[i];
            assert_eq!(
                slot.scale,
                Some(if i <= fail { h.scales[i] } else { header().scales[i] })
            );
            assert!(slot.metrics.is_some());
            if i < fail {
                assert!(slot.resource.is_none());
                assert_eq!(Arc::strong_count(&old.tables[i]), 1);
            } else {
                assert!(Arc::ptr_eq(slot.resource.as_ref().unwrap(), &old.tables[i]));
            }
            assert_eq!(Arc::strong_count(&r.tables[i]), 1);
        }
    }
}

#[test]
fn load_header_failure_null_and_sprite_factory_lifetimes() {
    struct Headers {
        inner: Resources,
        header: Arc<ModelHeader>,
        fail: bool,
        requests: usize,
    }
    impl OwnerResourceServices for Headers {
        fn header(&mut self, key: ResourceKey, iid: u32, options: [u32; 2]) -> Option<Arc<ModelHeader>> {
            assert_eq!(
                key,
                ResourceKey {
                    resource_type: SPRITE_ANIMATION_TYPE,
                    ..super::closure_tests::key(10)
                }
            );
            assert_eq!((iid, options), (MODEL_HEADER_IID, [0, 0]));
            self.requests += 1;
            if self.fail {
                None
            } else {
                Some(self.header.clone())
            }
        }
        fn zoom(&mut self, key: ResourceKey, iid: u32, options: [u32; 2]) -> Option<Arc<RawSpriteHolder>> {
            self.inner.zoom(key, iid, options)
        }
        fn texture_service(&mut self, service: u32, iid: u32) -> Option<Arc<dyn SpriteTextureService>> {
            self.inner.texture_service(service, iid)
        }
    }
    let sprite_key = ResourceKey {
        resource_type: SPRITE_ANIMATION_TYPE,
        ..key(10)
    };
    let mut services = Headers {
        inner: resources(),
        header: Arc::new(header()),
        fail: true,
        requests: 0,
    };
    let mut sprite = AnimatedSprite::<()>::default();
    let before = sprite.state;
    assert_eq!(sprite.load(sprite_key, &mut services), Ok(false));
    assert_eq!(sprite.load_from_resource(None, &mut services), Ok(false));
    assert_eq!(services.requests, 1);
    assert!(services.inner.calls.is_empty());
    assert_eq!(sprite.state, before);
    services.fail = false;
    let created = create_sprite_from_key(sprite_key, &mut services).unwrap().unwrap();
    assert_eq!(Arc::strong_count(&services.header), 1);
    assert_eq!(created.frame_count(), 4);
    assert_eq!(Arc::strong_count(&services.inner.tables[0]), 2);
    drop(created);
    assert_eq!(Arc::strong_count(&services.inner.tables[0]), 1);
    services.inner.calls.clear();
    services.inner.fail = 0;
    assert!(create_sprite_from_key(sprite_key, &mut services).unwrap().is_none());
    assert_eq!(Arc::strong_count(&services.header), 1);
    assert_eq!(services.inner.calls, [0]);
}

#[test]
fn foreign_creation_preserves_caller_output_and_releases_acquisition_once() {
    use std::cell::Cell;
    use std::rc::Rc;

    // The model has no Clone/AddRef/QI operation available to the wrapper.
    struct Model(u32);
    struct Resource {
        calls: Rc<Cell<u32>>,
        releases: Rc<Cell<u32>>,
        result: bool,
        write: bool,
        output_address: usize,
    }
    impl ForeignAnimatedResource<Model> for Resource {
        fn create_model(&self, argument: u32, output: &mut Model) -> bool {
            assert_eq!(argument, 0);
            assert_eq!(output as *mut Model as usize, self.output_address);
            self.calls.set(self.calls.get() + 1);
            if self.write {
                *output = Model(42);
            }
            self.result
        }
    }
    impl Drop for Resource {
        fn drop(&mut self) {
            self.releases.set(self.releases.get() + 1);
        }
    }
    struct Services {
        resource: Option<Resource>,
        key: ResourceKey,
        requests: u32,
    }
    impl ForeignAnimatedResources for Services {
        type ModelOutput = Model;
        type Resource = Resource;
        fn animated_resource(&mut self, key: ResourceKey, iid: u32, options: [u32; 2]) -> Option<Resource> {
            assert_eq!((key, iid, options), (self.key, 0x9ad0e7e3, [0, 0]));
            self.requests += 1;
            self.resource.take()
        }
    }
    for (acquire, result, write) in [
        (false, false, false),
        (true, true, true),
        (true, false, false),
        (true, false, true),
    ] {
        let mut output = Model(17);
        let calls = Rc::new(Cell::new(0));
        let releases = Rc::new(Cell::new(0));
        let key = ResourceKey {
            resource_type: FOREIGN_MODEL_TYPE,
            group: 123,
            instance: 456,
        };
        assert_eq!(creation_route(key), CreationRoute::ForeignResource);
        let mut services = Services {
            resource: acquire.then(|| Resource {
                calls: calls.clone(),
                releases: releases.clone(),
                result,
                write,
                output_address: &mut output as *mut Model as usize,
            }),
            key,
            requests: 0,
        };
        assert_eq!(create_foreign_from_key(key, &mut services, &mut output), result);
        assert_eq!(output.0, if acquire && write { 42 } else { 17 });
        assert_eq!(
            (services.requests, calls.get(), releases.get()),
            (1, u32::from(acquire), u32::from(acquire))
        );
    }
}

#[test]
fn literal_playback_signed_counts_byte_endpoints_and_as5_clock_cases() {
    let base = Playback {
        frame: 0,
        position: 0.,
        rate: 1.,
        direction: 1,
        play_mode: 3,
        frame_skipping: true,
    };
    for count in [0, 1, 0x80000000, u32::MAX] {
        let mut p = base;
        assert_eq!(p.advance_unsequenced(count), Ok(false));
        assert_eq!(p, base);
        let result = p.tick_unsequenced(2.5, count).unwrap();
        assert_eq!((result.status(), result.advances, result.frame_changed), (2, 2, false));
        assert_eq!((p.frame, p.position), (0, 0.5));
    }
    for count in [300, 65537, i32::MAX as u32] {
        let mut p = Playback { frame: 255, ..base };
        assert_eq!(p.advance_unsequenced(count), Ok(true));
        assert_eq!((p.frame, p.direction), (0, 1));
        // Even count 65537's endpoint does not alias the stored frame zero.
        p.advance_unsequenced(count).unwrap();
        assert_eq!(p.frame, 1);
        p.frame = 0;
        p.direction = -1;
        p.advance_unsequenced(count).unwrap();
        assert_eq!(p.frame, u32::from((count - 1) as u8));
    }
    let mut reverse = Playback {
        direction: -1,
        play_mode: 2,
        ..base
    };
    assert_eq!(reverse.tick_unsequenced(1.25, 4).unwrap().status(), 2);
    assert_eq!((reverse.frame, reverse.direction, reverse.position), (1, 1, -2.25));
    let mut stop = Playback {
        frame: 3,
        play_mode: 0,
        ..base
    };
    assert_eq!(stop.tick_unsequenced(2.5, 4).unwrap().status(), 2);
    assert_eq!((stop.frame, stop.direction, stop.position), (3, 0, 0.));
    for mut p in [
        Playback {
            direction: 0,
            position: 2.,
            ..base
        },
        Playback { rate: -1., ..base },
    ] {
        let before = p;
        assert_eq!(p.tick_unsequenced(2., 4), Err(PlaybackError::ClockCannotProgress));
        assert_eq!(p, before);
    }
    for rate in [-100., -200.] {
        let mut s = AnimatedSprite::<()>::default();
        assert_eq!(s.set_rate(rate), 2);
        assert_eq!(s.state.playback.rate, rate);
        assert_eq!(s.tick(0.), Ok(0));
        assert_eq!((s.state.playback.rate, s.state.playback.position), (-200., 0.));
    }
}

#[test]
fn controller_tick_keeps_owner_direction_and_rolls_back_safe_diagnostics() {
    let mut s = AnimatedSprite::<()>::default();
    s.install_controller_snapshot(Some(SequenceController {
        selected: 0,
        count: 1,
        length: 3,
        starts: vec![10, 13],
        current: 12,
        mode: 2,
        direction: 1,
    }));
    s.set_animation(0).unwrap();
    s.set_rate(1.);
    s.state.playback.frame_skipping = true;
    assert_eq!(s.tick(2.5), Ok(2));
    assert_eq!(
        (s.frame(), s.state.playback.position, s.state.playback.direction),
        (10, 0.5, 1)
    );
    assert_eq!(s.controller().unwrap().direction, -1);
    s.state.playback.direction = 0;
    s.state.playback.position = 2.;
    let before = s.state;
    let controller = s.controller().cloned();
    s.frame_dirty = false;
    assert_eq!(s.tick(0.), Err(PlaybackError::ClockCannotProgress));
    assert_eq!(s.state, before);
    assert_eq!(s.controller(), controller.as_ref());
    assert!(!s.frame_dirty);
    // A stopped controller leaves the owner's clock and direction alone.
    s.state.playback.direction = 1;
    s.state.playback.position = 0.;
    let c = s.sequence.as_mut().unwrap();
    c.current = 12;
    c.direction = 1;
    c.mode = 0;
    assert_eq!(s.tick(1.25), Ok(2));
    assert_eq!(
        (s.frame(), s.state.playback.position, s.state.playback.direction),
        (12, 0.25, 1)
    );
    assert_eq!(s.controller().unwrap().direction, 0);
    // A full u16 controller frame can equal the owner's byte after narrowing,
    // but still requires dirtying, including with a one-frame controller.
    let c = s.sequence.as_mut().unwrap();
    c.current = 268;
    c.length = 1;
    s.frame_dirty = false;
    assert_eq!(s.tick(1.), Ok(2));
    assert_eq!(s.frame(), 12);
    assert!(s.frame_dirty);
}

#[test]
fn sprite_reserved_slot_and_raw_play_mode_leave_unrelated_state() {
    let mut s = AnimatedSprite::<()>::default();
    s.install_controller_snapshot(Some(SequenceController {
        selected: 0,
        count: 1,
        length: 1,
        starts: vec![0, 1],
        current: 0,
        mode: 3,
        direction: -1,
    }));
    let before = s.state;
    let controller = s.controller().cloned();
    assert_eq!(s.reserved_status(), 0);
    assert_eq!(s.state, before);
    s.set_play_mode_raw(0x123402);
    let mut expected = before;
    expected.playback.play_mode = 2;
    assert_eq!(s.state, expected);
    assert_eq!(s.controller(), controller.as_ref());
}

#[test]
fn shared_draw_record_aliases_material_fields_and_preserves_opaque_bytes() {
    let mut sprite = AnimatedSprite::<()>::default();
    for offset in [4, 8, 12, 16] {
        assert_eq!(&sprite.draw_attributes()[offset..offset + 4], &1_f32.to_le_bytes());
    }
    let mut record = [0x91; 20];
    record[0] = 0x80 | FLAG_FADE | FLAG_HIGHLIGHT;
    record[1] = 9;
    record[16..20].copy_from_slice(&0.25_f32.to_le_bytes());
    sprite.frame_dirty = false;
    sprite.set_draw_attributes(record);
    assert_eq!(
        (sprite.state.flags, sprite.state.highlight, sprite.state.alpha),
        (0x83, 9, 0.25)
    );
    assert_eq!(sprite.material().transparency, Some(0.25));
    assert_eq!(sprite.set_highlight(9), 0);
    assert_eq!(sprite.set_self_lit(true), 2);
    assert_eq!(sprite.set_highlight(0x107), 2);
    assert_eq!(sprite.set_fade(false, 0.5), 2);
    record[0] = 0x85;
    record[1] = 7;
    record[16..20].copy_from_slice(&1_f32.to_le_bytes());
    assert_eq!(sprite.draw_attributes(), &record);
    assert!(!sprite.frame_dirty);
    // Shared copies preserve arbitrary float words, including NaN payloads.
    record[16..20].copy_from_slice(&0x7fc12345_u32.to_le_bytes());
    sprite.set_draw_attributes(record);
    assert_eq!(sprite.state.alpha.to_bits(), 0x7fc12345);
    assert_eq!(sprite.draw_attributes(), &record);
}

#[test]
fn defaults_shared_surface_copies_and_equal_position_counter_wrap() {
    let mut sprite = AnimatedSprite::<()>::default();
    assert_eq!(
        (sprite.state.properties, sprite.state.zoom, sprite.state.rotation),
        (1, 255, 255)
    );
    assert_eq!(sprite.state.playback.rate.to_bits(), 0x3a83126f);
    assert_eq!(sprite.cache().unwrap().vertices[0].colour, u32::MAX);
    assert_eq!(sprite.set_position([10., 20., 30.]), 1);
    assert_eq!(
        sprite.world_bounds().unwrap(),
        Bounds {
            min: [10., 20., 30.],
            max: [11., 21., 31.]
        }
    );
    sprite.state.transform.change = 255;
    let mut imported = sprite.transform();
    imported.change = 44;
    imported.scale = 9.;
    sprite.set_transform(imported);
    assert_eq!(sprite.transform().change, 0);
    let mut output = imported;
    output.change = 255;
    sprite.get_transform(&mut output);
    assert_eq!(output.change, 0);
    assert_eq!(sprite.transform().change, 0);
    assert_eq!(sprite.look_at([10., 20., 29.], [0., 1., 0.]), Ok(2));
    assert_eq!(sprite.transform().rotation, [1., 0., 0., 0., 1., 0., 0., 0., 1.]);
    assert_eq!(sprite.transform().change, 2);
    assert_eq!(sprite.resource_model(), None);
    assert_eq!(sprite.sort_value(), Ok(0));
    sprite.loaded.binding = Some(Arc::new(TextureBinding {
        table: vec![TextureHandle(4321)],
    }));
    assert_eq!(sprite.sort_value(), Ok(4321));
}

#[test]
fn sequence_absolute_frames_boundary_equality_and_independent_clock_direction() {
    let mut s = AnimatedSprite::<()>::default();
    assert_eq!(s.set_animation(0), Ok(0));
    s.install_controller_snapshot(Some(SequenceController {
        selected: 0,
        count: 1,
        length: 3,
        starts: vec![10, 13],
        current: 12,
        mode: 3,
        direction: 1,
    }));
    assert_eq!(s.set_animation(0), Ok(2));
    assert_eq!(s.frame(), 12);
    for (n, expected) in [(3, 13), (4, 11), (-1, 9)] {
        assert_eq!(s.select_frame(n), Ok(2));
        assert_eq!(s.frame(), expected);
    }
    s.select_frame(2).unwrap();
    s.set_play_mode(1);
    assert_eq!(s.advance_frame(), Ok(2));
    assert_eq!(s.frame(), 10);
    assert_eq!(s.controller().unwrap().mode, 3);
    s.set_rate(1.);
    assert_eq!(s.tick(1.25), Ok(2));
    assert_eq!((s.frame(), s.state.playback.position), (11, 0.25));
    s.sequence.as_mut().unwrap().length = 0;
    assert_eq!(s.select_frame(1), Err(PlaybackError::EmptySequence));
    s.sequence = None;
    s.frame_dirty = false;
    s.select_frame(267).unwrap();
    assert_eq!(s.frame(), 11);
    assert!(s.frame_dirty);
    s.set_rate(-100.);
    assert_eq!(s.tick(0.), Ok(0));
    assert_eq!(s.state.playback.rate, -200.);
}

#[test]
fn literal_reverse_single_frame_and_stopped_dirty_vectors() {
    for (frame, direction, mode, expected_frame, expected_direction, expected_clock) in
        [(3, 1, 2, 2, -1, 2.25), (0, -1, 3, 3, -1, -0.25)]
    {
        let mut p = Playback {
            frame,
            position: 0.,
            rate: 1.,
            direction,
            play_mode: mode,
            frame_skipping: false,
        };
        assert_eq!(p.tick_unsequenced(1.25, 4).unwrap().status(), 2);
        assert_eq!(
            (p.frame, p.direction, p.position),
            (expected_frame, expected_direction, expected_clock)
        );
    }
    let mut p = Playback {
        frame: 0,
        position: 0.,
        rate: 1.,
        direction: 1,
        play_mode: 3,
        frame_skipping: true,
    };
    assert_eq!(p.tick_unsequenced(2.5, 1).unwrap().status(), 2);
    assert_eq!((p.frame, p.position), (0, 0.5));
    p.direction = 0;
    assert_eq!(p.advance_unsequenced(4), Ok(true));
}

#[test]
fn conditional_direction_conversion_and_all_zero_angle_masks() {
    for (input, expected) in [(0.5, 0), (1.5, 2), (-0.5, 0), (-1.5, 0xfffffffe)] {
        assert_eq!(RawSpriteHolder::convert_bin_nearest_even(input), expected);
    }
    assert_eq!(raw().select_angles(0., 0., 0), Ok(0));
    for flags in [0xa3, 0xa7] {
        for mask in 0_u8..8 {
            let mut s = AnimatedSprite::<()>::default();
            s.state.transform.kind = flags;
            s.state.transform.change = 255;
            let before = s.state;
            let angles = std::array::from_fn(|i| if mask & (1 << i) == 0 { -100. } else { 0. });
            assert_eq!(s.set_orientation(angles), Ok(0));
            let increment = if mask == 0 {
                0
            } else if mask == 7 {
                1
            } else {
                mask.count_ones() as u8 + u8::from(flags & 4 != 0)
            };
            assert_eq!(s.transform().change, 255_u8.wrapping_add(increment));
            assert_eq!(s.transform().kind, if mask == 0 { flags } else { flags | 4 });
            assert_eq!(s.state.bounds, before.bounds);
            assert_eq!(s.state.playback, before.playback);
        }
    }
    let mut s = AnimatedSprite::<()>::default();
    let before = s.state;
    assert_eq!(s.set_orientation([1., 0., 0.]), Err(Unsupported::Arithmetic));
    assert_eq!(s.state, before);
}

#[test]
fn conditional_stage_permutations_old_sources_and_signed_zero() {
    use super::numeric::X;
    let original = [2., 3., 5., 7., 11., 13., 17., 19., 23.];
    let expected = [
        [2., 5., -3., 7., 13., -11., 17., 23., -19.],
        [-5., 3., 2., -13., 11., 7., -23., 19., 17.],
        [3., -2., 5., 11., -7., 13., 19., -17., 23.],
    ];
    for (axis, expected) in expected.into_iter().enumerate() {
        let mut m = original;
        super::placement::stage(&mut m, axis, X::f(0.), X::f(1.));
        assert_eq!(m, expected);
    }
    let mut m = [-0.; 9];
    m[3] = 1.;
    m[6] = 2.;
    super::placement::stage(&mut m, 0, X::f(1.), X::f(0.));
    for i in 0..3 {
        assert_eq!(m[i * 3 + 1].to_bits(), 0x80000000);
        assert_eq!(m[i * 3 + 2].to_bits(), 0);
    }
}
