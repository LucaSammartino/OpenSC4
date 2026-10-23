// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::Arc;

fn playback() -> Playback {
    Playback {
        frame: 0,
        position: 0.0,
        rate: 10.0,
        direction: 1,
        play_mode: 3,
        frame_skipping: true,
    }
}

fn transform() -> Transform {
    Transform {
        kind: 0,
        rotation: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        translation: [0.0; 3],
        scale: 1.0,
        change: 0,
    }
}

fn snapshot() -> InstanceState {
    InstanceState {
        playback: playback(),
        zoom: 2,
        rotation: 0,
        flags: 0,
        alpha: 1.0,
        highlight: 0,
        custom_data: 0,
        properties: 0,
        draw_attributes: [0; 20],
        transform: transform(),
        bounds: Bounds {
            min: [0.0; 3],
            max: [0.0; 3],
        },
    }
}

pub(super) fn sprite() -> AnimatedSprite<()> {
    AnimatedSprite::from_snapshot(snapshot(), None)
}

fn key(index: u32) -> ResourceKey {
    ResourceKey {
        resource_type: 0x09ad_cd75,
        group: 7,
        instance: index,
    }
}

fn frame() -> FrameEntry {
    FrameEntry {
        texture: TextureHandle(17),
        page_side: 256,
        pixel_origin: [32, 64],
        pixel_size: [32, 16],
        pixel_offset: [-3.0, 5.0],
        flip_u: false,
        flip_v: false,
    }
}

fn table(frames: u32, directions: u32) -> SpriteTable {
    SpriteTable {
        frames,
        directions,
        base_angle: 0.0,
        entries: (0..frames * directions)
            .map(|i| {
                let mut f = frame();
                f.texture = TextureHandle(u64::from(i));
                f
            })
            .collect(),
    }
}

fn animation() -> AnimationRecord {
    AnimationRecord {
        zooms: std::array::from_fn(|i| ZoomRecord {
            table_key: key(i as u32),
            scale: 1.0,
        }),
    }
}

struct Resources {
    tables: Vec<Option<Arc<SpriteTable>>>,
    calls: Vec<(ResourceKey, u32)>,
}
impl SpriteResources for Resources {
    fn sprite_table(&mut self, key: ResourceKey, interface: u32) -> Option<Arc<SpriteTable>> {
        self.calls.push((key, interface));
        self.tables.get(key.instance as usize).cloned().flatten()
    }
}

// §10's five prescribed vectors, each independently asserted.
#[test]
fn vector_zoom_one_suppressed_two_visible() {
    let mut s = sprite();
    s.set_view(1, 0);
    assert!(s.drawing_suppressed());
    s.set_view(2, 0);
    assert!(!s.drawing_suppressed());
    // Draw command count awaits AS7; this vector claims visibility only.
}

#[test]
fn vector_direction_eight_zero_yaw() {
    assert_eq!(direction_index(8, 0.0, 0.0), Ok(4));
}

#[test]
fn vector_last_frame_loop_and_ping_pong() {
    let mut p = playback();
    p.frame = 3;
    assert_eq!(p.advance_unsequenced(4), Ok(true));
    assert_eq!(p.frame, 0);
    p.frame = 3;
    p.play_mode = 2;
    assert_eq!(p.advance_unsequenced(4), Ok(true));
    assert_eq!((p.frame, p.direction), (2, -1));
}

#[test]
fn vector_clock_raw_units_skip_and_no_skip() {
    let mut p = playback();
    assert_eq!(p.tick_unsequenced(0.25, 10).unwrap().advances, 2);
    assert_eq!((p.frame, p.position), (2, 0.5));
    let mut p = playback();
    p.frame_skipping = false;
    assert_eq!(p.tick_unsequenced(0.25, 10).unwrap().advances, 1);
    assert_eq!((p.frame, p.position), (1, 1.0));
}

#[test]
fn vector_size_thirty_two_pixels_half_metre() {
    let c = billboard_corners([0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [32.0, 32.0], [0.0; 2], 0.5);
    assert_eq!(
        c,
        [[0.0, 0.0, 0.0], [16.0, 0.0, 0.0], [16.0, 16.0, 0.0], [0.0, 16.0, 0.0]]
    );
}

#[test]
fn clock_independent_one_and_two_boundaries() {
    for (elapsed, frame, position) in [
        (0.999, 0, 0.999),
        (1.0, 1, 0.0),
        (1.5, 1, 0.5),
        (1.999, 1, 0.999),
        (2.0, 2, 0.0),
        (2.5, 2, 0.5),
    ] {
        let mut p = playback();
        p.rate = 1.0;
        p.tick_unsequenced(elapsed, 10).unwrap();
        assert_eq!(p.frame, frame);
        assert!((p.position - position).abs() < 0.000001);
    }
}

#[test]
fn no_skip_long_tick_keeps_one_for_next_zero_elapsed_tick() {
    let mut p = playback();
    p.frame_skipping = false;
    p.tick_unsequenced(0.25, 10).unwrap();
    p.tick_unsequenced(0.0, 10).unwrap();
    assert_eq!((p.frame, p.position), (2, 0.0));
}

#[test]
fn no_skip_exact_two_differs_from_below_two() {
    let mut p = playback();
    p.frame_skipping = false;
    p.rate = 1.0;
    p.tick_unsequenced(1.5, 10).unwrap();
    assert_eq!((p.frame, p.position), (1, 0.5));
    let mut p = playback();
    p.frame_skipping = false;
    p.rate = 1.0;
    p.tick_unsequenced(2.0, 10).unwrap();
    assert_eq!((p.frame, p.position), (1, 1.0));
}

#[test]
fn frame_modulo_and_empty_failure_without_mutation() {
    let mut p = playback();
    p.set_frame(14, 5).unwrap();
    assert_eq!(p.frame, 4);
    let before = p;
    assert_eq!(p.set_frame(3, 0), Err(PlaybackError::EmptySequence));
    assert_eq!(p, before);
}

#[test]
fn clock_never_converts_elapsed_and_rate_zero_does_nothing() {
    let mut p = playback();
    p.rate = 0.001;
    p.tick_unsequenced(1000.0, 10).unwrap();
    assert_eq!(p.frame, 1);
    p.rate = 0.0;
    p.position = 2.5;
    let before = p;
    assert!(!p.tick_unsequenced(1.0, 10).unwrap().frame_changed);
    assert_eq!(p, before);
}

#[test]
fn reverse_interior_and_unpublished_endpoint() {
    let mut p = playback();
    p.frame = 3;
    p.direction = -1;
    p.tick_unsequenced(0.15, 10).unwrap();
    assert_eq!(p.frame, 2);
    assert_eq!(p.position, -0.5);
    p.frame = 0;
    let before = p;
    assert_eq!(
        p.advance_unsequenced(10),
        Err(PlaybackError::ReverseBoundaryUnspecified)
    );
    assert_eq!(p, before);
}

#[test]
fn stopped_and_single_frame_do_not_invent_changes() {
    let mut p = playback();
    p.direction = 0;
    assert_eq!(p.advance_unsequenced(10), Ok(false));
    let mut p = playback();
    assert_eq!(p.advance_unsequenced(1), Ok(false));
    p.frame = 3;
    p.play_mode = 1;
    assert_eq!(p.advance_unsequenced(4), Ok(false));
    assert_eq!((p.direction, p.position), (0, 0.0));
}

#[test]
fn sentinel_nonfinite_and_nonprogress_clock_fail_transactionally() {
    for rate in [-100.0, -200.0] {
        let mut p = playback();
        p.rate = rate;
        let before = p;
        assert_eq!(p.tick_unsequenced(1.0, 10), Err(PlaybackError::RateSentinel));
        assert_eq!(p, before);
    }
    let mut p = playback();
    let before = p;
    assert_eq!(p.tick_unsequenced(f32::INFINITY, 10), Err(PlaybackError::NonFinite));
    assert_eq!(p, before);
    assert_eq!(p.tick_unsequenced(-0.25, 10), Err(PlaybackError::ClockCannotProgress));
    assert_eq!(p, before);
}

#[test]
fn view_and_frame_dirty_and_independent_flags() {
    let mut s = sprite();
    s.frame_dirty = false;
    s.set_view(2, 0);
    assert!(!s.frame_dirty());
    s.set_view(2, 9);
    assert!(s.frame_dirty());
    s.frame_dirty = false;
    s.set_frame(5, 4).unwrap();
    assert!(s.frame_dirty());
    s.state.flags = 0x80;
    s.set_fade(true, 0.3);
    s.set_self_lit(true);
    s.set_highlight(9);
    assert_eq!(s.state.flags, 0x87);
    assert!(!s.drawing_suppressed()); // DIFFERS §5.2: only incoming highlight-pass rejects.
    s.set_highlight(0);
    assert_eq!(s.state.flags, 0x86);
    assert!(!s.drawing_suppressed());
    s.set_self_lit(false);
    assert_eq!(s.state.flags, 0x82);
}

#[test]
fn fade_bounds_nan_and_material_constants() {
    let mut s = sprite();
    assert_eq!(s.material().draw, DrawState::SPRITE);
    // DIFFERS §§4.4/5.2: -1 enables, 1 disables, alpha reference is alpha/2.
    for (alpha, reference) in [(0.0, 0.0), (-1.0, -0.5), (0.375, 0.1875)] {
        s.set_fade(true, alpha);
        let m = s.material();
        assert_eq!(m.transparency, Some(alpha));
        assert_eq!(m.draw.alpha_reference, reference);
    }
    for alpha in [-0.1, 1.1, f32::NAN] {
        s.set_fade(false, alpha);
        // DIFFERS: §4.4 now explicitly requires disable/restore even for NaN.
        assert_eq!(s.state.alpha, 1.0);
        assert_eq!(s.material().transparency, None);
    }
    s.set_fade(false, 0.0);
    assert_eq!(s.material().transparency, None);
    assert_eq!(s.material().draw.alpha_reference, 0.5);
}

#[test]
fn asymmetric_direction_rows_base_and_modulo() {
    let t = table(3, 8);
    assert_eq!(t.entry(0.0, 4).unwrap().texture, TextureHandle(13));
    assert_eq!(direction_index(8, std::f32::consts::FRAC_PI_2, 0.0), Ok(6));
    assert_eq!(direction_index(8, 0.0, std::f32::consts::FRAC_PI_2), Ok(2));
    assert_eq!(direction_index(8, -std::f32::consts::TAU, 0.0), Ok(4));
    assert_eq!(direction_index(0, 0.0, 0.0), Err(FrameError::Empty));
    assert_eq!(direction_index(8, f32::NAN, 0.0), Err(FrameError::NonFinite));
}

#[test]
fn missing_entries_and_zero_frames_are_errors() {
    let mut t = table(3, 8);
    t.entries.clear();
    assert_eq!(t.entry(0.0, 0), Err(FrameError::MissingEntry));
    t.frames = 0;
    assert_eq!(t.entry(0.0, 0), Err(FrameError::Empty));
}

#[test]
fn asymmetric_uv_flips_and_scale_epsilon() {
    let mut f = frame();
    assert_eq!(f.uv_edges(), Ok([[0.125, 0.25], [0.25, 0.3125]]));
    f.flip_u = true;
    assert_eq!(f.uv_edges(), Ok([[0.25, 0.125], [0.25, 0.3125]]));
    f.flip_v = true;
    assert_eq!(f.uv_edges(), Ok([[0.25, 0.125], [0.3125, 0.25]]));
    assert_eq!(f.scaled_metrics(0.5), ([16.0, 8.0], [-1.5, 2.5]));
    assert_eq!(f.scaled_metrics(1.00005), ([32.0, 16.0], [-3.0, 5.0]));
    assert_ne!(f.scaled_metrics(1.0002).0, [32.0, 16.0]);
    f.page_side = 255;
    assert_eq!(f.uv_edges(), Err(FrameError::InvalidPage));
}

#[test]
fn resource_routes_keep_native_name_swap() {
    let mut k = key(0);
    assert_eq!(creation_route(k), CreationRoute::Unsupported);
    k.resource_type = SPRITE_ANIMATION_TYPE;
    assert_eq!(creation_route(k), CreationRoute::SpriteInstance);
    k.resource_type = FOREIGN_MODEL_TYPE;
    assert_eq!(creation_route(k), CreationRoute::ForeignResource);
}

#[test]
fn loading_stops_first_missing_zoom_and_acquires_counted_records() {
    let t = Arc::new(table(3, 8));
    let mut r = Resources {
        tables: vec![Some(t.clone()), None, Some(t.clone())],
        calls: vec![],
    };
    let p = acquire_zoom_prefix(&animation(), &mut r);
    assert_eq!(p.end, PrefixEnd::Missing(1));
    assert_eq!(
        r.calls,
        vec![(key(0), SPRITE_TABLE_INTERFACE), (key(1), SPRITE_TABLE_INTERFACE)]
    );
    assert_eq!(p.zooms.len(), 1);
    assert_eq!(p.zooms[0].angle_step, std::f32::consts::FRAC_PI_4);
    assert_eq!(Arc::strong_count(&t), 4);
    drop(p);
    assert_eq!(Arc::strong_count(&t), 3);
}

#[test]
fn loading_empty_and_invalid_prefix_releases_temporary() {
    let mut r = Resources {
        tables: vec![],
        calls: vec![],
    };
    assert_eq!(acquire_zoom_prefix(&animation(), &mut r).end, PrefixEnd::Missing(0));
    let t = Arc::new(table(0, 8));
    r.tables.push(Some(t.clone()));
    assert_eq!(acquire_zoom_prefix(&animation(), &mut r).end, PrefixEnd::Invalid(0));
    assert_eq!(Arc::strong_count(&t), 2);
}

#[test]
fn adopting_new_prefix_releases_old_zoom_references() {
    let t = Arc::new(table(3, 8));
    let mut r = Resources {
        tables: vec![Some(t.clone()); 5],
        calls: vec![],
    };
    let mut s = sprite();
    s.adopt_zoom_prefix(acquire_zoom_prefix(&animation(), &mut r));
    assert_eq!(s.loaded_zooms().len(), 5);
    assert_eq!(Arc::strong_count(&t), 11);
    s.adopt_zoom_prefix(ZoomPrefix {
        zooms: vec![],
        end: PrefixEnd::Missing(0),
    });
    assert_eq!(Arc::strong_count(&t), 6);
}

#[test]
fn noncommuting_transform_and_inverse_do_not_translate_vectors() {
    let t = Transform {
        kind: 7,
        rotation: [0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        translation: [7.0, 11.0, 13.0],
        scale: 2.0,
        change: 9,
    };
    assert_eq!(t.point([1.0, 2.0, 3.0]), [3.0, 13.0, 19.0]);
    assert_eq!(t.vector([1.0, 2.0, 3.0]), [-4.0, 2.0, 6.0]);
    assert_eq!(t.inverse_point([3.0, 13.0, 19.0]), Some([1.0, 2.0, 3.0]));
    assert_eq!(t.inverse_vector([-4.0, 2.0, 6.0]), Some([1.0, 2.0, 3.0]));
    let mut t = t;
    t.scale = 0.0;
    assert_eq!(t.inverse_point([0.0; 3]), None);
}

#[test]
fn every_published_transform_kind() {
    let mut t = Transform {
        kind: 0,
        rotation: [0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        translation: [7.0, 11.0, 13.0],
        scale: 2.0,
        change: 0,
    };
    for (kind, expected) in [
        (0, [1.0, 2.0, 3.0]),
        (1, [8.0, 13.0, 16.0]),
        (2, [2.0, 4.0, 6.0]),
        (3, [9.0, 15.0, 19.0]),
        (4, [-2.0, 1.0, 3.0]),
        (5, [5.0, 12.0, 16.0]),
        (6, [3.0, 13.0, 19.0]),
        (255, [3.0, 13.0, 19.0]),
    ] {
        t.kind = kind;
        assert_eq!(t.point([1.0, 2.0, 3.0]), expected);
        assert_eq!(t.inverse_point(expected), Some([1.0, 2.0, 3.0]));
    }
}

#[test]
fn asymmetric_billboard_axes_offsets_and_widen_only_box() {
    let c = billboard_corners(
        [7.0, 11.0, 13.0],
        [0.0, 0.0, -1.0],
        [0.0, 1.0, 0.0],
        [12.0, 8.0],
        [-3.0, 5.0],
        0.5,
    );
    assert_eq!(
        c,
        [[7.0, 13.5, 14.5], [7.0, 13.5, 8.5], [7.0, 17.5, 8.5], [7.0, 17.5, 14.5]]
    );
    let mut b = Bounds {
        min: [-10.0, -20.0, -30.0],
        max: [1.0, 2.0, 3.0],
    };
    b.widen(&c);
    assert_eq!(
        b,
        Bounds {
            min: [-10.0, -20.0, -30.0],
            max: [7.0, 17.5, 14.5]
        }
    );
    assert_eq!(b.center(), [-1.5, -1.25, -7.75]);
}

struct Picker {
    hits: Vec<bool>,
    calls: Vec<[Vec3; 3]>,
}
impl TrianglePicker for Picker {
    fn triangle_pick(&mut self, a: Vec3, b: Vec3, c: Vec3) -> bool {
        self.calls.push([a, b, c]);
        self.hits[self.calls.len() - 1]
    }
}

#[test]
fn picking_short_circuits_and_uses_fan_triangles() {
    let c = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0], [10.0, 11.0, 12.0]];
    for (hits, count, expected) in [
        (vec![true, false], 1, true),
        (vec![false, true], 2, true),
        (vec![false, false], 2, false),
    ] {
        let mut p = Picker { hits, calls: vec![] };
        assert_eq!(pick_quad(&c, &mut p), expected);
        assert_eq!(p.calls.len(), count);
        assert_eq!(p.calls[0], [c[0], c[1], c[2]]);
        if count == 2 {
            // DIFFERS §6: preserve the primitive's actual argument order.
            assert_eq!(p.calls[1], [c[2], c[3], c[0]]);
        }
    }
}

#[test]
fn basic_query_addrefs_and_last_release_drops_owner_and_zooms() {
    let owner = Arc::new(());
    let table = Arc::new(table(2, 1));
    let mut s = AnimatedSprite::from_snapshot(snapshot(), Some(owner.clone()));
    s.adopt_zoom_prefix(ZoomPrefix {
        zooms: vec![LoadedZoom {
            table: table.clone(),
            scale: 1.0,
            angle_step: std::f32::consts::TAU,
        }],
        end: PrefixEnd::Missing(1),
    });
    let mut a = SpriteArena::default();
    let h = a.insert(s, 1).unwrap();
    assert_eq!(a.query_basic_interface(h, 1), Ok(h));
    assert_eq!(a.query_basic_interface(h, BASIC_INSTANCE_IID), Ok(h));
    assert_eq!(a.references(h), Some(3));
    assert_eq!(a.release(h), Ok(2));
    assert_eq!(a.release(h), Ok(1));
    assert_eq!((Arc::strong_count(&owner), Arc::strong_count(&table)), (2, 2));
    assert_eq!(a.release(h), Ok(0));
    assert!(a.get(h).is_none());
    assert_eq!((Arc::strong_count(&owner), Arc::strong_count(&table)), (1, 1));
    assert_eq!(a.release(h), Err(ReferenceError::Missing));
    let next = a.insert(sprite(), 1).unwrap();
    assert_ne!(next, h);
}

#[test]
fn references_overflow_and_unknown_iid_do_not_mutate_count() {
    let mut a = SpriteArena::default();
    let h = a.insert(sprite(), u16::MAX).unwrap();
    assert_eq!(a.add_ref(h), Err(ReferenceError::Overflow));
    assert_eq!(a.references(h), Some(u16::MAX));
    assert_eq!(a.query_basic_interface(h, 0), Err(ReferenceError::UnsupportedInterface));
    assert_eq!(a.references(h), Some(u16::MAX));
    assert_eq!(a.insert(sprite(), 0), Err(ReferenceError::ZeroInitialCount));
}

#[test]
fn owner_replacement_counts_and_getters_are_shared_thread_safe() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<AnimatedSprite<()>>();
    let owner = Arc::new(());
    let mut s = AnimatedSprite::from_snapshot(snapshot(), Some(owner.clone()));
    assert_eq!(Arc::strong_count(&owner), 2);
    s.set_owner(Some(owner.clone()));
    assert_eq!(Arc::strong_count(&owner), 2);
    let _ = s.owner();
    assert_eq!(Arc::strong_count(&owner), 2);
    s.set_owner(None);
    assert_eq!(Arc::strong_count(&owner), 1);
    s.set_custom_data(u32::MAX);
    s.set_properties(0xa5);
    s.set_draw_attributes([0x73; 20]);
    assert_eq!(s.custom_data(), u32::MAX);
    assert_eq!(s.properties(), 0xa5);
    assert_eq!(s.draw_attributes(), &[0x73; 20]);
}
