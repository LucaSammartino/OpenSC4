// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::*;
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc,
};

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}
fn zoom(n: usize) -> Zoom {
    Zoom::new(n).unwrap()
}
fn point(x: f32, z: f32) -> Point {
    Point::new(x, z)
}
fn cell(x: u32, z: u32) -> Cell {
    Cell { x, z }
}
fn rect(x: u32, z: u32, xx: u32, zz: u32) -> CellRect {
    CellRect::new(cell(x, z), cell(xx, zz)).unwrap()
}
fn bounds() -> Bounds {
    Bounds {
        min: point(-100_000.0, -100_000.0),
        max: point(100_000.0, 100_000.0),
    }
}
fn pair(x: f32, z: f32) -> CandidatePair {
    CandidatePair {
        outer: point(x, z),
        inner: point(x, z + 1.0),
        height: 9.3,
    }
}
fn strip(pairs: usize) -> Option<StripGeometry> {
    let mut filter = PairFilter::new(&RippleTuning::default(), zoom(0)).unwrap();
    for i in 0..pairs {
        assert_eq!(
            filter.push(pair(100.0 + i as f32 * 16.0, 100.0), bounds()).unwrap(),
            PairDecision::Kept
        );
    }
    filter.finish()
}

#[derive(Default)]
struct Source {
    reads: AtomicUsize,
    missing: bool,
}
impl Source {
    fn read(&self, key: [u32; 3]) {
        assert_eq!(key, [0x6534_284A, 0x88CD_66E9, 2]);
        self.reads.fetch_add(1, Ordering::SeqCst);
    }
}
impl retained::RawTuningSource for Source {
    fn acquire(&self, key: [u32; 3]) -> bool {
        assert_eq!(key, RIPPLE_EXEMPLAR);
        !self.missing
    }
    fn release(&self) {}
    fn property(&self, property: u32) -> Option<retained::Property<'_>> {
        use crate::variant::Elem;
        self.read(RIPPLE_EXEMPLAR);
        let scalar = match property & 255 {
            0 => Some(Elem::Bool(true)),
            1 => Some(Elem::U32(50)),
            3 => Some(Elem::F32(0.3)),
            7 => Some(Elem::U32(175000)),
            12 => Some(Elem::U32(12)),
            13 => Some(Elem::U32(6)),
            _ => None,
        };
        const SIXTEEN: &[u8] = &[
            0, 0, 128, 65, 0, 0, 128, 65, 0, 0, 128, 65, 0, 0, 128, 65, 0, 0, 128, 65,
        ];
        Some(retained::Property {
            tag: scalar.map_or(0x800A, |v| v.mem_type()),
            scalar,
            backing: SIXTEEN,
            count: 5,
        })
    }
}

fn arithmetic() -> retained::ReadArithmetic {
    // Literal parameter for the unpublished stored factor; WR11, no native claim.
    retained::ReadArithmetic {
        mode: retained::NumericMode::Published24,
        degree_factor: 0.5,
    }
}
fn process() -> Arc<std::sync::Mutex<retained::RippleProcess>> {
    Arc::new(std::sync::Mutex::new(retained::RippleProcess::default()))
}

struct Ground {
    counts: [u32; 2],
    flags: Vec<u32>,
    counter: AtomicU64,
}
impl Ground {
    fn new(counts: [u32; 2]) -> Self {
        Self {
            counts,
            flags: vec![0; ((counts[0] + 1) * (counts[1] + 1)) as usize],
            counter: AtomicU64::new(0),
        }
    }
}
impl TerrainProvider for Ground {
    fn cell_counts(&self) -> [u32; 2] {
        self.counts
    }
    fn vertex_flags(&self, c: Cell) -> u32 {
        self.flags[(c.z * (self.counts[0] + 1) + c.x) as usize]
    }
    fn vertex_position(&self, c: Cell) -> Point {
        point(c.x as f32 * 16.0, c.z as f32 * 16.0)
    }
    fn water_level(&self) -> f32 {
        9.0
    }
    fn edit_counter(&self) -> u64 {
        self.counter.load(Ordering::SeqCst)
    }
}
struct Texture(u32);
impl TextureBinding for Texture {
    fn frame_count(&self) -> u32 {
        self.0
    }
}
#[derive(Default)]
struct Factory {
    texture: Option<Arc<Texture>>,
    calls: AtomicUsize,
}
impl TextureFactory for Factory {
    fn bind_slot6(&self, key: [u32; 3]) -> Option<Arc<dyn TextureBinding>> {
        assert_eq!(key, [0x7AB5_0E44, 0x891B_0E1A, 0xE9A4_4A88]);
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.texture.as_ref().map(|t| t.clone() as Arc<dyn TextureBinding>)
    }
}
struct Random {
    frame: u32,
    period: u64,
    bounds: Option<(u64, u64)>,
}
impl Default for Random {
    fn default() -> Self {
        Self {
            frame: 0,
            period: 175000,
            bounds: None,
        }
    }
}
impl RandomSource for Random {
    fn below(&mut self, _: u32) -> u32 {
        self.frame
    }
    fn inclusive(&mut self, min: u64, max: u64) -> u64 {
        self.bounds = Some((min, max));
        self.period
    }
}
fn selection(counts: [u32; 2]) -> SelectedTerrainArea {
    let mut s = SelectedTerrainArea::default();
    s.init(
        Arc::new(Ground::new(counts)),
        Arc::new(Factory::default()),
        SelectionRenderProperties::default(),
    )
    .unwrap();
    s
}

// The eight prescribed §6 vectors use explicit expected values, not native
// owner output, self-generated golden files, or snapshots of the implementation.
#[test]
fn vector1_spline() {
    let p = spline(
        [point(0.0, 0.0), point(16.0, 0.0), point(32.0, 0.0), point(48.0, 0.0)],
        0.0,
    )
    .unwrap();
    assert_eq!(
        p,
        SplinePoint {
            point: point(16.0, 0.0),
            tangent: point(1.0, 0.0)
        }
    );
}
#[test]
fn vector2_width_zoom4() {
    let p = candidate_pair(
        SplinePoint {
            point: point(80.0, 90.0),
            tangent: point(1.0, 0.0),
        },
        0.0,
        9.0,
        &RippleTuning::default(),
        zoom(4),
    )
    .unwrap();
    assert_eq!(p.outer, point(80.0, 64.0));
    assert_eq!(p.inner, point(80.0, 91.0));
    close(p.height, 9.3);
}
#[test]
fn vector3_spacing() {
    let mut f = PairFilter::new(&RippleTuning::default(), zoom(0)).unwrap();
    assert_eq!(f.push(pair(100.0, 100.0), bounds()), Ok(PairDecision::Kept));
    assert_eq!(f.push(pair(110.0, 100.0), bounds()), Ok(PairDecision::TooClose));
}
#[test]
fn vector4_turn() {
    let mut f = PairFilter::new(&RippleTuning::default(), zoom(2)).unwrap();
    f.push(pair(100.0, 100.0), bounds()).unwrap();
    f.push(pair(120.0, 100.0), bounds()).unwrap();
    let angle = 70.0f32.to_radians();
    assert_eq!(
        f.push(pair(120.0 + 20.0 * angle.cos(), 100.0 + 20.0 * angle.sin()), bounds()),
        Ok(PairDecision::EndAtTurn)
    );
}
#[test]
fn vector5_short_shore() {
    let tuning = RippleTuning::default();
    assert!(!tuning.shore_is_long_enough(40));
    assert!(!tuning.shore_is_long_enough(50));
    assert!(tuning.shore_is_long_enough(51));
}
#[test]
fn vector6_frames_at_exact_boundary() {
    let mut a = FrameAnimation::new(2, 4, 150000).unwrap();
    let mut r = Random::default();
    assert_eq!(a.tick(149999, &RippleTuning::default(), &mut r), Ok(false));
    assert_eq!(a.frame(), 2);
    assert_eq!(a.tick(1, &RippleTuning::default(), &mut r), Ok(true));
    assert_eq!(a.frame(), 3);
    assert_eq!(a.elapsed(), 0);
    assert_eq!(a.period(), 175000);
    assert_eq!(r.bounds, Some((122500, 227500)));
}
#[test]
fn vector7_plain_replaces_textured() {
    let mut s = selection([4, 3]);
    s.mark_palette(cell(2, 1), 12, 0x80, 0x42, 0, 0xff, false).unwrap();
    s.mark_cell(cell(2, 1), 0xff010203, false).unwrap();
    assert_eq!(s.plain_cells().get(&7), Some(&0xff010203));
    assert!(s.textured_cells().is_empty());
}
#[test]
fn vector8_palette3() {
    let mut s = selection([3, 2]);
    s.mark_palette(cell(3, 2), 15, 1, 2, 3, 99, false).unwrap();
    let t = s.textured_cells()[&11];
    assert_eq!(
        t.material,
        Material {
            rgb: [0.4, 0.3, 0.0],
            alpha: 1.0,
            mode: 2
        }
    );
    assert_eq!((t.b1, t.b2, t.b3), (1, 2, Some(99)));
}

#[test]
fn asymmetric_spline_has_independently_calculated_position_and_derivative() {
    // At t=.5 weights=(1,23,23,1)/48; derivative=(-1,-5,5,1)/8.
    let s = spline(
        [point(0.0, 3.0), point(12.0, 0.0), point(18.0, 18.0), point(45.0, -6.0)],
        0.5,
    )
    .unwrap();
    close(s.point.x, 15.3125);
    close(s.point.z, 8.5625);
    close(s.tangent.x, 75.0 / 12186.0f32.sqrt());
    close(s.tangent.z, 81.0 / 12186.0f32.sqrt());
}
#[test]
fn asymmetric_normal_and_wobble_not_axis_swapped() {
    let p = candidate_pair(
        SplinePoint {
            point: point(100.0, 200.0),
            tangent: point(3.0, 4.0),
        },
        std::f32::consts::FRAC_PI_2,
        7.0,
        &RippleTuning::default(),
        zoom(4),
    )
    .unwrap();
    close(p.outer.x, 124.96);
    close(p.outer.z, 181.28);
    close(p.inner.x, 99.2);
    close(p.inner.z, 200.6);
}
#[test]
fn spline_endpoint_continuity() {
    let p = [
        point(1.0, 7.0),
        point(20.0, -3.0),
        point(15.0, 29.0),
        point(40.0, 5.0),
        point(60.0, 17.0),
    ];
    let a = spline([p[0], p[1], p[2], p[3]], 1.0).unwrap();
    let b = spline([p[1], p[2], p[3], p[4]], 0.0).unwrap();
    close(a.point.x, b.point.x);
    close(a.point.z, b.point.z);
    close(a.tangent.x, b.tangent.x);
    close(a.tangent.z, b.tangent.z);
}
#[test]
fn degenerate_or_nonfinite_geometry_rejected() {
    assert_eq!(spline([point(1.0, 1.0); 4], 0.5), Err(Error::InvalidGeometry));
    assert_eq!(spline([point(1.0, 1.0); 4], f32::NAN), Err(Error::InvalidGeometry));
    let mut f = PairFilter::new(&RippleTuning::default(), zoom(0)).unwrap();
    assert_eq!(f.push(pair(f32::NAN, 0.0), bounds()), Err(Error::InvalidGeometry));
}
#[test]
fn strip_minimum_even_count_coordinates_and_fade_limit() {
    assert!(strip(3).is_none());
    let s = strip(4).unwrap();
    assert_eq!(s.vertex_count(), 8);
    let v = s.vertices();
    assert_eq!(v[0].point, point(100.0, 100.0));
    assert_eq!(v[1].point, point(100.0, 101.0));
    close(v[6].texture_advance, 0.45);
    assert_eq!(s.fade_pair_count(&RippleTuning::default()), 2);
    assert_eq!(strip(20).unwrap().fade_pair_count(&RippleTuning::default()), 6);
    assert_eq!(
        s.prepare_for_display(),
        Err(Error::Gap(ContractGap::StripPreparationWr5))
    );
}
#[test]
fn spacing_equality_and_skip_do_not_change_previous_point() {
    let mut f = PairFilter::new(&RippleTuning::default(), zoom(0)).unwrap();
    for x in [100.0, 116.0] {
        assert_eq!(f.push(pair(x, 100.0), bounds()), Ok(PairDecision::Kept));
    }
    assert_eq!(f.push(pair(126.0, 100.0), bounds()), Ok(PairDecision::TooClose));
    assert_eq!(f.push(pair(132.0, 100.0), bounds()), Ok(PairDecision::Kept));
}
#[test]
fn either_inner_or_outer_too_close_skips_entire_pair() {
    let mut f = PairFilter::new(&RippleTuning::default(), zoom(0)).unwrap();
    f.push(pair(100.0, 100.0), bounds()).unwrap();
    let candidate = CandidatePair {
        outer: point(120.0, 100.0),
        inner: point(110.0, 101.0),
        height: 9.3,
    };
    assert_eq!(f.push(candidate, bounds()), Ok(PairDecision::TooClose));
}
#[test]
fn edge_and_pair_limit_end_strip() {
    let mut f = PairFilter::new(&RippleTuning::default(), zoom(0)).unwrap();
    let b = Bounds {
        min: point(0.0, 0.0),
        max: point(100.0, 100.0),
    };
    assert_eq!(f.push(pair(0.2, 20.0), b), Ok(PairDecision::EndAtEdge));
    for i in 0..1000 {
        assert_eq!(f.push(pair(i as f32 * 16.0, 20.0), bounds()), Ok(PairDecision::Kept));
    }
    assert_eq!(f.push(pair(16000.0, 20.0), bounds()), Ok(PairDecision::EndAtLimit));
    assert_eq!(f.finish().unwrap().vertex_count(), 2000);
}
#[test]
fn frame_wrap_and_overshoot_is_explicit_and_atomic() {
    let mut a = FrameAnimation::new(3, 4, 150000).unwrap();
    let mut r = Random::default();
    let before = a.clone();
    assert_eq!(
        a.tick(150001, &RippleTuning::default(), &mut r),
        Err(Error::Gap(ContractGap::AnimationOvershootWr6))
    );
    assert_eq!(a, before);
    a.tick(150000, &RippleTuning::default(), &mut r).unwrap();
    assert_eq!(a.frame(), 0);
}
#[test]
fn random_failure_does_not_advance_frame_or_clock() {
    let mut a = FrameAnimation::new(1, 4, 150000).unwrap();
    let before = a.clone();
    let mut random = Random {
        period: 227501,
        ..Random::default()
    };
    assert_eq!(
        a.tick(150000, &RippleTuning::default(), &mut random),
        Err(Error::InvalidRandomValue)
    );
    assert_eq!(a, before);
    assert_eq!(FrameAnimation::new(0, 0, 175000), Err(Error::EmptyTexture));
}
#[test]
fn failed_texture_keeps_no_strip_and_empty_texture_releases_binding() {
    let mut r = Random::default();
    let f = Factory::default();
    assert!(strip(4)
        .unwrap()
        .bind_texture(&f, &RippleTuning::default(), &mut r)
        .unwrap()
        .is_none());
    assert_eq!(f.calls.load(Ordering::SeqCst), 1);
    let texture = Arc::new(Texture(0));
    let f = Factory {
        texture: Some(texture.clone()),
        ..Factory::default()
    };
    assert!(matches!(
        strip(4).unwrap().bind_texture(&f, &RippleTuning::default(), &mut r),
        Err(Error::EmptyTexture)
    ));
    assert_eq!(Arc::strong_count(&texture), 2);
}
#[test]
fn bound_strip_owns_exactly_one_texture_reference_until_drop() {
    let texture = Arc::new(Texture(4));
    let factory = Factory {
        texture: Some(texture.clone()),
        ..Factory::default()
    };
    let mut random = Random {
        frame: 3,
        ..Random::default()
    };
    let strip = strip(4)
        .unwrap()
        .bind_texture(&factory, &RippleTuning::default(), &mut random)
        .unwrap()
        .unwrap();
    assert_eq!(Arc::strong_count(&texture), 3);
    assert_eq!(strip.animation().frame(), 3);
    drop(strip);
    assert_eq!(Arc::strong_count(&texture), 2);
}

#[test]
fn tuning_property_routes_and_raw_thresholds() {
    let source = Source::default();
    let mut p = retained::RippleProcess::default();
    p.eligible_init(&source, arithmetic()).unwrap();
    assert_eq!(p.tuning.period, 175000);
    assert_eq!(p.tuning.minimum, 50);
    assert_eq!(p.tuning.period_bounds, [122499, 227499]);
    close(p.tuning.step[4], 0.25);
    assert_eq!(source.reads.load(Ordering::SeqCst), 14);
    assert_eq!(Zoom::new(5), Err(Error::InvalidTuning));
}

#[path = "tests_wr8.rs"]
mod wr8;
#[test]
fn tuning_rejects_nonfinite_and_zero_spacing() {
    let mut t = RippleTuning::default();
    t.spacing[4] = 0.0;
    assert_eq!(t.validate(), Err(Error::InvalidTuning));
    t.spacing[4] = 16.0;
    t.height = f32::NAN;
    assert_eq!(t.validate(), Err(Error::InvalidTuning));
}
#[test]
fn literal_flags_all_low_byte_combinations() {
    for flags in 0..256 {
        assert_eq!(
            WaterRipple::eligible_flags(flags),
            (flags & 16 == 16) && (flags & 1 == 0)
        );
    }
    assert!(WaterRipple::eligible_flags(0xffff_fffe));
    assert!(!WaterRipple::eligible_flags(0xffff_ffff));
}
#[test]
fn actual_seed_blocks_detection_without_host_trace_delegation() {
    let mut g = Ground::new([2, 1]);
    g.flags = vec![0, 0x10, 0x11, 0x90, 0x01, 0];
    let mut w = WaterRipple::default();
    w.init(
        Arc::new(g),
        Arc::new(Factory::default()),
        &Source::default(),
        process(),
        arithmetic(),
    )
    .unwrap();
    assert_eq!(w.shore_seeds().unwrap(), vec![cell(1, 0), cell(0, 1)]);
    assert_eq!(w.update(zoom(4), false), Err(Error::Gap(ContractGap::ShoreTraceWr4)));
}
#[test]
fn counter_threshold_then_raw_delay_exact_boundary() {
    let tuning = RippleTuning::default();
    let mut state = RebuildState::default();
    assert!(!state.tick(9000000, 350, 100, &tuning));
    assert!(!state.pending());
    assert!(!state.tick(9000000, 351, 100, &tuning));
    assert!(state.pending());
    assert_eq!(state.elapsed(), 0);
    assert!(!state.tick(1999999, 351, 100, &tuning));
    assert!(state.tick(1, 351, 100, &tuning));
}
#[test]
fn terrain_area_and_dirty_rectangle_boundaries() {
    let tuning = RippleTuning::default();
    let mut state = RebuildState::default();
    assert!(!state.terrain_edit(rect(0, 0, 3, 4), &tuning));
    assert_eq!(state.accumulated_area(), 12);
    assert!(!state.pending());
    assert!(state.terrain_edit(rect(3, 4, 4, 5), &tuning));
    assert!(state.pending());
    assert_eq!(state.accumulated_area(), 0);
    let mut dirty = RebuildState::default();
    dirty.terrain_edit(rect(0, 0, 1, 1), &tuning);
    dirty.terrain_edit(rect(19, 19, 20, 20), &tuning);
    assert!(!dirty.pending());
    dirty.terrain_edit(rect(20, 19, 21, 20), &tuning);
    assert!(dirty.pending());
    assert_eq!(dirty.dirty_rectangle(), Some(rect(0, 0, 21, 20)));
}
#[test]
fn repeated_arm_preserves_clock_and_counter_decrease_does_not_wrap() {
    let mut s = RebuildState::default();
    let t = RippleTuning::default();
    assert!(!s.tick(1, 0, u64::MAX, &t));
    assert!(!s.pending());
    s.arm();
    s.tick(123, 0, 0, &t);
    s.arm();
    assert_eq!(s.elapsed(), 123);
}
#[test]
fn rebuild_message_refreshes_counter_and_cleanup_releases_resources() {
    let ground = Arc::new(Ground::new([2, 2]));
    let factory = Arc::new(Factory::default());
    let source = Source::default();
    let mut w = WaterRipple::default();
    let shared = process();
    w.init(ground.clone(), factory.clone(), &source, shared.clone(), arithmetic())
        .unwrap();
    assert_eq!(w.update(zoom(0), true), Ok(RippleUpdate::Empty));
    ground.counter.store(251, Ordering::SeqCst);
    w.do_message(RippleMessage::AnimationTick(1)).unwrap();
    assert!(w.rebuild_state().pending());
    w.do_message(RippleMessage::AnimationTick(2000000)).unwrap();
    assert!(!w.rebuild_state().pending());
    assert_eq!(w.remembered_counter(), 251);
    ground.counter.store(300, Ordering::SeqCst);
    w.do_message(RippleMessage::RebuildNow).unwrap();
    assert_eq!(w.remembered_counter(), 300);
    w.shutdown();
    assert_eq!(Arc::strong_count(&ground), 1);
    assert_eq!(Arc::strong_count(&factory), 1);
    w.init(ground, factory, &source, shared, arithmetic()).unwrap();
    assert_eq!(source.reads.load(Ordering::SeqCst), 14);
}
#[test]
fn missing_exemplar_init_continues_and_safe_before_init() {
    let mut w = WaterRipple::default();
    assert_eq!(w.update(zoom(0), true), Err(Error::NotInitialized));
    let source = Source {
        missing: true,
        ..Source::default()
    };
    let g = Arc::new(Ground::new([1, 1]));
    let f = Arc::new(Factory::default());
    // DIFFERS: §2.4 missing whole exemplar does not fail/roll back Init.
    assert_eq!(w.init(g.clone(), f.clone(), &source, process(), arithmetic()), Ok(true));
    assert!(w.is_initialized());
    assert_eq!(Arc::strong_count(&g), 2);
    assert_eq!(Arc::strong_count(&f), 2);
    w.shutdown();
    w.shutdown();
}

#[test]
fn inclusive_asymmetric_rectangle_covers_last_vertex_and_uses_vertex_stride() {
    let mut s = selection([4, 2]);
    s.mark_rectangle(rect(2, 1, 4, 2), 77, false, SelectionRenderProperties::default())
        .unwrap();
    assert_eq!(
        s.plain_cells().keys().copied().collect::<Vec<_>>(),
        vec![7, 8, 9, 12, 13, 14]
    );
}
#[test]
fn all_mark_forms_clear_both_tables_and_textured_replaces_plain() {
    let mut s = selection([3, 2]);
    s.mark_cell(cell(0, 0), 1, false).unwrap();
    s.mark_palette(cell(1, 0), 2, 3, 4, 0, 5, false).unwrap();
    s.mark_list(&[cell(3, 2), cell(3, 2)], 99, true).unwrap();
    assert_eq!(s.plain_cells().len(), 1);
    assert!(s.textured_cells().is_empty());
    s.mark_palette(cell(3, 2), 2, 3, 4, 0, 5, false).unwrap();
    assert!(s.plain_cells().is_empty());
    s.mark_packed(
        cell(0, 0),
        99,
        1,
        2,
        0x123456ff,
        2,
        true,
        SelectionRenderProperties::default(),
    )
    .unwrap();
    assert_eq!(s.textured_cells().len(), 1);
    s.mark_rectangle(rect(0, 1, 1, 1), 2, true, SelectionRenderProperties::default())
        .unwrap();
    assert!(s.textured_cells().is_empty());
    assert_eq!(s.plain_cells().len(), 2);
    s.mark_palette(cell(0, 0), 2, 3, 4, 0, 5, true).unwrap();
    assert!(s.plain_cells().is_empty());
    s.mark_cell(cell(1, 1), 4, true).unwrap();
    assert!(s.textured_cells().is_empty());
    s.mark_list(&[], 0, true).unwrap();
    assert!(s.plain_cells().is_empty());
}
#[test]
fn packed_rgb_ignores_low_byte_and_only_mode3_falls_back() {
    let mut s = selection([1, 1]);
    let p = SelectionRenderProperties {
        no_add_signed: true,
        ..SelectionRenderProperties::default()
    };
    s.mark_packed(cell(0, 0), 8, 0xa1, 0xb2, 0xff804000, 3, false, p)
        .unwrap();
    let t = s.textured_cells()[&0];
    assert_eq!((t.key, t.b1, t.b2, t.b3), (8, 0xa1, 0xb2, None));
    assert_eq!(t.material.mode, 1);
    assert_eq!(t.material.alpha, 0.5);
    assert_eq!(t.material.rgb, [1.0, 128.0 / 255.0, 64.0 / 255.0]);
    s.mark_packed(cell(0, 0), 8, 0xa1, 0xb2, 0xff8040ff, 2, false, p)
        .unwrap();
    assert_eq!(
        s.textured_cells()[&0].material,
        Material {
            rgb: t.material.rgb,
            alpha: 1.0,
            mode: 2
        }
    );
    s.mark_packed(
        cell(1, 1),
        8,
        0,
        0,
        0x123456ff,
        3,
        false,
        SelectionRenderProperties::default(),
    )
    .unwrap();
    assert_eq!(s.textured_cells()[&3].material.mode, 3);
    assert_eq!(s.textured_cells()[&3].material.alpha, 1.0);
}
#[test]
fn palette_all_five_known_entries_and_overflow_gap_preserves_tables() {
    let mut s = selection([4, 1]);
    let expected = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.4, 0.3, 0.0],
        [0.0, 0.0, 0.0],
    ];
    for (i, rgb) in expected.into_iter().enumerate() {
        s.mark_palette(cell(i as u32, 0), 5, 0, 0, i as u32, 0, false).unwrap();
        assert_eq!(s.textured_cells()[&(i as u32)].material.rgb, rgb);
    }
    let before = s.textured_cells().clone();
    assert_eq!(
        s.mark_palette(cell(0, 1), 5, 0, 0, 5, 0, true),
        Err(Error::Gap(ContractGap::PaletteOutsideTableWr9))
    );
    assert_eq!(s.textured_cells(), &before);
}
#[test]
fn invalid_selection_inputs_do_not_clear_or_partially_write() {
    let mut s = selection([2, 1]);
    s.mark_cell(cell(1, 1), 7, false).unwrap();
    let before = s.plain_cells().clone();
    assert_eq!(
        s.mark_list(&[cell(0, 0), cell(3, 0)], 99, true),
        Err(Error::InvalidCell)
    );
    assert_eq!(
        s.mark_rectangle(
            CellRect {
                min: cell(2, 1),
                max: cell(0, 0)
            },
            1,
            true,
            SelectionRenderProperties::default()
        ),
        Err(Error::InvalidRectangle)
    );
    assert_eq!(s.mark_cell(cell(0, 2), 0, true), Err(Error::InvalidCell));
    assert_eq!(s.plain_cells(), &before);
}
#[test]
fn rendering_stays_blocked_until_masks_states_and_order_are_specified() {
    let mut s = selection([1, 1]);
    assert_eq!(s.update(SelectionRenderProperties::default()), Ok(()));
    let props = SelectionRenderProperties {
        prefer_opaque: true,
        ..SelectionRenderProperties::default()
    };
    s.mark_rectangle(rect(0, 0, 0, 0), 1, false, props).unwrap();
    assert_eq!(s.state_preference(), Some(true));
    assert!(s.render_states_dirty());
    assert_eq!(s.update(props), Err(Error::Gap(ContractGap::SelectionGeometryWr7)));
}
#[test]
fn selection_shutdown_and_drop_release_only_owned_references() {
    let g = Arc::new(Ground::new([1, 1]));
    let f = Arc::new(Factory::default());
    let mut s = SelectedTerrainArea::default();
    s.init(g.clone(), f.clone(), SelectionRenderProperties::default())
        .unwrap();
    assert_eq!(Arc::strong_count(&g), 2);
    s.mark_cell(cell(0, 0), 1, false).unwrap();
    s.shutdown();
    s.shutdown();
    assert!(s.plain_cells().is_empty());
    assert_eq!(Arc::strong_count(&g), 1);
    assert_eq!(Arc::strong_count(&f), 1);
    assert_eq!(s.mark_cell(cell(0, 0), 1, false), Err(Error::NotInitialized));
    s.init(g.clone(), f.clone(), SelectionRenderProperties::default())
        .unwrap();
    drop(s);
    assert_eq!(Arc::strong_count(&g), 1);
    assert_eq!(Arc::strong_count(&f), 1);
}
#[test]
fn owner_getters_can_be_shared_between_threads() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<WaterRipple>();
    send_sync::<SelectedTerrainArea>();
    send_sync::<AnimatedStrip>();
    let mut s = selection([2, 1]);
    s.mark_cell(cell(2, 1), 42, false).unwrap();
    let s = Arc::new(s);
    let other = s.clone();
    let thread = std::thread::spawn(move || assert_eq!(other.plain_cells().get(&5), Some(&42)));
    assert_eq!(s.plain_cells().len(), 1);
    thread.join().unwrap();
}
