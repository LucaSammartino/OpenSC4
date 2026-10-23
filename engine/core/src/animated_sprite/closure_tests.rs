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
