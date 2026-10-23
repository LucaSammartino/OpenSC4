// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::*;
use crate::terrain_ripples::mesh::{self, Animation, NativeRandom, Parameters, StripWalk, Vertex};
use crate::terrain_ripples::render::{self, Rect, RenderBox, Renderer};
use std::sync::Mutex;

struct Sea {
    counts: [u32; 2],
    flags: Mutex<Vec<u16>>,
    heights: Vec<f32>,
    counter: AtomicU64,
    refreshes: AtomicUsize,
}
impl Sea {
    fn new(counts: [u32; 2]) -> Self {
        let len = ((counts[0] + 1) * (counts[1] + 1)) as usize;
        Self {
            counts,
            flags: Mutex::new(vec![0; len]),
            heights: vec![9.0; len],
            counter: AtomicU64::new(0),
            refreshes: AtomicUsize::new(0),
        }
    }
    fn index(&self, x: u32, z: u32) -> usize {
        (z * (self.counts[0] + 1) + x) as usize
    }
    fn height(&mut self, x: u32, z: u32, y: f32) {
        let i = self.index(x, z);
        self.heights[i] = y;
    }
    fn flag(&self, x: u32, z: u32, value: u16) {
        self.flags.lock().unwrap()[self.index(x, z)] = value;
    }
    fn long() -> Self {
        let mut s = Self::new([200, 20]);
        for x in 1..200 {
            s.height(x, 9, 8.0);
        }
        for x in 2..=190 {
            s.flag(x, 10, 0x10);
        }
        s
    }
}
impl TerrainProvider for Sea {
    fn cell_counts(&self) -> [u32; 2] {
        self.counts
    }
    fn vertex_flags(&self, c: Cell) -> u32 {
        u32::from(self.flags.lock().unwrap()[self.index(c.x, c.z)])
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
    fn vertex_height(&self, c: Cell) -> Result<f32, Error> {
        Ok(self.heights[self.index(c.x, c.z)])
    }
    fn refresh_shore_flags(&self) -> Result<(), Error> {
        self.refreshes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn set_vertex_flags(&self, c: Cell, flags: u16) -> Result<(), Error> {
        self.flag(c.x, c.z, flags);
        Ok(())
    }
    fn city_extent(&self) -> Result<Point, Error> {
        Ok(point(self.counts[0] as f32 * 16.0, self.counts[1] as f32 * 16.0))
    }
    fn current_zoom(&self) -> Result<Zoom, Error> {
        Ok(zoom(0))
    }
}

fn parameters() -> Parameters {
    Parameters {
        step: 0.25,
        spacing_squared: 256.0,
        turn_radians: 1.0471976,
        advance: 0.15,
        outer: 45.0,
        inner: 1.0,
        wobble: 0.2,
        height: 0.3,
        alpha: [100, 100],
        fade: 6,
    }
}
fn vertices(outer: (f32, f32), inner: (f32, f32)) -> [Vertex; 2] {
    [
        Vertex {
            position: [outer.0, 9.0, outer.1],
            color: 0x64FF_FFFF,
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [inner.0, 9.3, inner.1],
            color: 0x64FF_FFFF,
            uv: [0.0, 0.0],
        },
    ]
}
fn straight(x: f32) -> [Vertex; 2] {
    vertices((x, 100.0), (x, 101.0))
}
fn owner() -> (WaterRipple, Arc<Sea>, Arc<Factory>, Arc<Mutex<retained::RippleProcess>>) {
    let ground = Arc::new(Sea::long());
    let factory = Arc::new(Factory {
        texture: Some(Arc::new(Texture(8))),
        calls: AtomicUsize::new(0),
    });
    let shared = process();
    let mut w = WaterRipple::default();
    w.init(
        ground.clone(),
        factory.clone(),
        &Source::default(),
        shared.clone(),
        arithmetic(),
    )
    .unwrap();
    {
        let mut p = shared.lock().unwrap();
        p.tuning.floats = [
            vec![256.0; 5],
            vec![0.15, 0.3, 0.3, 0.6, 0.75],
            vec![45.0, 40.0, 35.0, 30.0, 26.0],
            vec![1.0; 5],
            vec![0.2; 5],
            vec![1.0471976; 5],
        ];
        p.tuning.bytes = [vec![100, 150, 200, 255, 255], vec![100, 150, 200, 255, 255]];
        p.random = Some(NativeRandom { state: 1 });
    }
    (w, ground, factory, shared)
}

#[test]
fn directional_trace_backward_seed_barriers_and_global_visit_clear() {
    let mut s = Sea::new([2, 2]);
    s.heights.fill(10.0);
    s.height(0, 1, 8.0);
    s.flag(1, 1, 0x90);
    s.flag(0, 0, 0x10);
    s.flag(0, 2, 0x10);
    assert_eq!(shore::trace(&s, cell(1, 1)).unwrap(), vec![6, 4, 0]);
    assert_eq!(s.vertex_flags(cell(1, 1)), 0x91);
    // Preexisting visits are cleared too; other bits survive.
    s.flag(2, 2, 0x81);
    assert!(shore::detect(&s, 2).unwrap().is_empty());
    assert!(s.flags.lock().unwrap().iter().all(|f| f & 1 == 0));
    assert_eq!(s.vertex_flags(cell(2, 2)), 0x80);
}

#[test]
fn trace_singleton_equal_heights_strict_minimum_and_append_detection() {
    let mut s = Sea::new([2, 2]);
    s.heights.fill(8.0);
    s.flag(1, 1, 0x10);
    assert_eq!(shore::trace(&s, cell(1, 1)).unwrap(), vec![4]);
    assert!(shore::detect(&s, 2).unwrap().is_empty());
    s.heights.fill(9.0);
    s.flag(1, 1, 0x10);
    s.flag(2, 1, 0x10);
    assert_eq!(shore::trace(&s, cell(1, 1)).unwrap(), vec![4]);
    let (mut w, ground, _, shared) = owner();
    shared.lock().unwrap().tuning.minimum = 189;
    w.detect_shores().unwrap();
    assert!(w.shores().is_empty());
    shared.lock().unwrap().tuning.minimum = 188;
    w.detect_shores().unwrap();
    w.detect_shores().unwrap();
    assert_eq!(w.shores().len(), 2);
    assert_eq!(w.shores()[0].points.len(), 189);
    assert_eq!(w.shores()[0].points[0], point(32.0, 160.0));
    assert_eq!(ground.refreshes.load(Ordering::SeqCst), 3);
}

#[test]
fn partial_outer_and_inner_come_from_different_candidates() {
    let mut walk = StripWalk::default();
    let p = parameters();
    let extent = point(10000.0, 10000.0);
    walk.candidate(straight(100.0), extent, &p, &mut || true);
    walk.candidate(vertices((120.0, 100.0), (110.0, 101.0)), extent, &p, &mut || true);
    assert_eq!(walk.strips[0].len(), 3);
    walk.candidate(vertices((150.0, 100.0), (125.0, 101.0)), extent, &p, &mut || true);
    assert_eq!(walk.strips[0].len(), 4);
    assert_eq!(walk.strips[0][2].position[0], 120.0);
    assert_eq!(walk.strips[0][3].position[0], 125.0);
    walk.candidate(straight(145.0), extent, &p, &mut || true);
    assert_eq!(walk.strips[0][4].uv[0], p.advance);
    assert_eq!(walk.strips[0][0].uv[0], 0.0);
    assert_eq!(walk.strips[0][2].uv[0], 0.0);
}

#[test]
fn owner_spacing_turn_limit_edges_and_short_strip_reuse() {
    let extent = point(100000.0, 1000.0);
    let p = parameters();
    let mut w = StripWalk::default();
    for x in [100.0, 110.0, 116.0, 132.0] {
        w.candidate(straight(x), extent, &p, &mut || true);
    }
    assert_eq!(w.strips[0].len(), 6); // 10 rejected, equality 16 accepted.
    let dx = 70.0_f32.to_radians().cos() * 20.0;
    let dz = 70.0_f32.to_radians().sin() * 20.0;
    w.candidate(
        vertices((132.0 + dx, 100.0 + dz), (132.0 + dx, 101.0 + dz)),
        extent,
        &p,
        &mut || true,
    );
    assert_eq!(w.strips[0].len(), 6); // turn consumes, no retry.
    w.candidate(straight(200.0), extent, &p, &mut || true);
    assert_eq!(w.strips.len(), 1);
    assert_eq!(w.strips[0].len(), 2);
    assert!(w.finish().is_empty());
    let mut w = StripWalk::default();
    for i in 0..1003 {
        w.candidate(straight(100.0 + i as f32 * 16.0), extent, &p, &mut || true);
    }
    assert_eq!(w.strips[0].len(), 2004);
    assert_eq!(w.strips[1].len(), 2);
    assert_eq!(w.finish().len(), 1);
    let mut w = StripWalk::default();
    w.candidate(vertices((0.2, 100.0), (0.2, 101.0)), extent, &p, &mut || true);
    assert_eq!(w.strips[0].len(), 2);
    w.candidate(straight(extent.x - 0.2), extent, &p, &mut || true);
    assert!(w.strips[0].is_empty());
}

#[test]
fn odd_outer_turn_failure_is_not_rolled_back_and_fade_uses_configured_count() {
    let mut w = StripWalk::default();
    let extent = point(10000.0, 10000.0);
    let p = parameters();
    for x in [100.0, 116.0, 132.0, 148.0] {
        w.candidate(straight(x), extent, &p, &mut || true);
    }
    w.candidate(vertices((164.0, 100.0), (148.0, 121.0)), extent, &p, &mut || true);
    assert_eq!(w.strips[0].len(), 9);
    let mut v = w.finish().remove(0);
    mesh::fade(&mut v, [100, 100], 6).unwrap();
    assert_eq!(v.len(), 8);
    assert_eq!(
        v.chunks_exact(2).map(|v| v[0].color >> 24).collect::<Vec<_>>(),
        [10, 24, 24, 10]
    );
    assert_eq!(
        v.iter().map(|v| v.color & 0xFF_FFFF).collect::<Vec<_>>(),
        vec![0xFF_FFFF; 8]
    );
}

#[test]
fn spline_domain_no_padding_phase_shared_and_only_inner_height_raised() {
    let points: Vec<_> = (0..5).map(|x| point(100.0 + x as f32 * 16.0, 100.0)).collect();
    let mut phase = 0.0;
    let mut p = parameters();
    p.spacing_squared = 1.0;
    let v = mesh::traverse(&points, &mut phase, 9.0, point(1000.0, 1000.0), &p, &mut || true).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].len(), 16);
    close(phase, 0.8);
    close(v[0][0].position[0], 116.0);
    close(v[0][0].position[2], 55.0);
    close(v[0][0].position[1], 9.0);
    close(v[0][1].position[1], 9.3);
    mesh::traverse(&points, &mut phase, 9.0, point(1000.0, 1000.0), &p, &mut || true).unwrap();
    close(phase, 1.6);
    assert!(
        mesh::traverse(&points[..3], &mut phase, 9.0, point(1000.0, 1000.0), &p, &mut || true)
            .unwrap()
            .is_empty()
    );
    p.outer = 26.0;
    let mut phase = 0.0;
    let v = mesh::traverse(&points, &mut phase, 9.0, point(1000.0, 1000.0), &p, &mut || true).unwrap();
    close(v[0][0].position[2], 74.0);
    close(v[0][1].position[2], 101.0);
}

#[test]
fn native_rng_high_product_zero_bound_and_one_advance_overshoot_wrap() {
    let mut random = NativeRandom { state: 0xFFFF_FFFF };
    assert_eq!(random.bounded(0), 0);
    assert_eq!(random.state, u32::MAX);
    let q = u64::from(u32::MAX) * 0x41C6_4E6D + 0x3039;
    assert_eq!(random.raw(), (q >> 16) as u32);
    assert_ne!((q >> 16) as u32, (q as u32) >> 16);
    let mut a = Animation {
        frame: 2,
        period: 150000,
        clock: 0,
    };
    let before = random.state;
    a.tick(900000, 8, [122499, 227499], &mut random);
    assert_eq!((a.frame, a.clock), (3, 0));
    assert_eq!(random.state, before.wrapping_mul(0x41C6_4E6D).wrapping_add(0x3039));
    assert!((122499..227499).contains(&a.period));
    a.frame = 7;
    a.clock = 0;
    a.tick(a.period, 8, [122499, 227499], &mut random);
    assert_eq!(a.frame, 0);
    a.clock = u32::MAX;
    a.period = 100;
    let before = random.state;
    a.tick(2, 8, [122499, 227499], &mut random);
    assert_eq!(a.clock, 1);
    assert_eq!(random.state, before);
}

#[derive(Default)]
struct Draw {
    packed: usize,
    states: Vec<u32>,
    models: usize,
    boxes: Vec<RenderBox>,
    frames: Vec<u32>,
    vertices: Vec<Vec<Vertex>>,
    reject: Option<usize>,
}
impl Renderer for Draw {
    fn visible(&mut self, b: RenderBox) -> bool {
        self.boxes.push(b);
        self.reject != Some(self.boxes.len())
    }
    fn pack_state(&mut self, p: [u32; 14]) -> u32 {
        assert_eq!(p, [1, 0, 1, 0, 1, 1, 1, 3, 4, 5, u32::MAX, 0xBF80_0000, 1, u32::MAX]);
        self.packed += 1;
        123
    }
    fn set_state(&mut self, s: u32) {
        self.states.push(s);
    }
    fn default_model_transform(&mut self) {
        self.models += 1;
    }
    fn bind_frame(&mut self, _: &dyn TextureBinding, frame: u32, unit: u32) {
        assert_eq!(unit, 0);
        self.frames.push(frame);
    }
    fn submit(&mut self, primitive: u32, format: u32, v: &[Vertex]) {
        assert_eq!((primitive, format), (7, 10));
        self.vertices.push(v.to_vec());
    }
}

#[test]
fn first_update_draw_gates_frustum_tokens_and_process_state_cache() {
    let (mut w, ground, factory, _) = owner();
    let mut draw = Draw::default();
    w.do_message(RippleMessage::AnimationTick(9000000)).unwrap();
    assert!(!w.rebuild_state().pending());
    assert_eq!(ground.refreshes.load(Ordering::SeqCst), 0);
    assert_eq!(
        w.update_with_renderer(zoom(4), false, &mut draw),
        Ok(RippleUpdate::Disabled)
    );
    assert!(!w.shores()[0].strips.is_empty());
    assert_eq!(draw.packed, 0);
    let old_phase = w.phase();
    let vertices = w.shores()[0].strips[0].vertices.clone();
    w.update_with_renderer(zoom(4), false, &mut draw).unwrap();
    assert_eq!(w.phase(), old_phase);
    assert_eq!(w.shores()[0].strips[0].vertices, vertices);
    assert!(
        matches!(w.update_with_renderer(zoom(4), true, &mut draw), Ok(RippleUpdate::Drawn { strips }) if strips > 0)
    );
    assert_eq!(ground.refreshes.load(Ordering::SeqCst), 1);
    assert_eq!(factory.calls.load(Ordering::SeqCst), 1);
    assert_eq!(draw.packed, 1);
    assert_eq!(draw.states, [123]);
    assert_eq!(draw.models, 1);
    assert!(draw.boxes.iter().all(|b| b.min[1] == 9.0 && b.max[1] == 9.0));
    draw.boxes.clear();
    draw.reject = Some(1);
    let submitted = draw.vertices.len();
    assert_eq!(
        w.update_with_renderer(zoom(4), true, &mut draw),
        Ok(RippleUpdate::Drawn { strips: 0 })
    );
    assert_eq!(draw.boxes.len(), 1);
    assert_eq!(draw.vertices.len(), submitted);
    assert_eq!(draw.packed, 1);
    draw.boxes.clear();
    draw.reject = Some(2);
    w.update_with_renderer(zoom(4), true, &mut draw).unwrap();
    assert_eq!(draw.packed, 1);
}

#[test]
fn grouped_bounds_do_not_bridge_gaps_and_contact_is_inclusive() {
    let mut v = vec![straight(16.1)[0]; 20];
    v.push(straight(1600.0)[0]);
    let r = render::contact_rectangles(&v);
    assert_eq!(r.len(), 2);
    assert_eq!((r[0].min_x, r[0].max_x), (1, 2));
    assert_eq!(r[1].min_x, 100);
    let middle = Rect {
        min_x: 50,
        min_z: 6,
        max_x: 51,
        max_z: 7,
    };
    assert!(r[0].union(r[1]).overlaps(middle));
    assert!(r.iter().all(|r| !r.overlaps(middle)));
    assert!(r[0].overlaps(Rect {
        min_x: 2,
        min_z: 7,
        max_x: 3,
        max_z: 8
    }));
}

#[test]
fn edit_removes_all_contacts_rearms_clock_and_delayed_tick_animates_new_strips() {
    let (mut w, ground, _, _) = owner();
    w.update(zoom(0), true).unwrap();
    assert!(!w.shores()[0].strips.is_empty());
    w.terrain_edit_packed(200, 20).unwrap(); // (0,0)..(200,20)
    assert!(w.shores()[0].strips.is_empty());
    assert!(w.rebuild_state().pending());
    w.do_message(RippleMessage::AnimationTick(2000000)).unwrap();
    assert!(w.shores()[0].strips.is_empty());
    w.do_message(RippleMessage::AnimationTick(1)).unwrap();
    assert!(!w.rebuild_state().pending());
    assert!(w.shores()[0].strips.iter().all(|s| s.animation.clock == 1));
    // Delayed tick clears/detects, then pending Create clears/detects again.
    assert_eq!(ground.refreshes.load(Ordering::SeqCst), 3);
    w.terrain_edit_packed(200, 20).unwrap();
    w.do_message(RippleMessage::AnimationTick(100)).unwrap();
    w.do_message(RippleMessage::RebuildNow).unwrap();
    assert_eq!(ground.refreshes.load(Ordering::SeqCst), 5);
    assert!(!w.rebuild_state().pending());
}

#[test]
fn edit_wrapping_thresholds_dirty_persistence_and_zoom_pending_redetection() {
    let (mut w, ground, _, shared) = owner();
    w.update(zoom(0), true).unwrap();
    shared.lock().unwrap().tuning.edit_area = 0;
    w.terrain_edit_packed((1 << 16) | 2, (18 << 16) | 19).unwrap();
    let dirty = w.rebuild_state().signed_dirty_rectangle();
    assert_eq!(
        dirty,
        Rect {
            min_x: 1,
            min_z: 18,
            max_x: 2,
            max_z: 19
        }
    );
    w.rebuild_now().unwrap();
    assert_eq!(w.rebuild_state().signed_dirty_rectangle(), dirty);
    ground.counter.store(251, Ordering::SeqCst);
    w.do_message(RippleMessage::AnimationTick(100)).unwrap();
    assert_eq!(w.rebuild_state().elapsed(), 0);
    let refreshes = ground.refreshes.load(Ordering::SeqCst);
    w.update(zoom(4), false).unwrap();
    assert!(w.rebuild_state().pending());
    w.update(zoom(4), true).unwrap();
    assert!(!w.rebuild_state().pending());
    assert_eq!(ground.refreshes.load(Ordering::SeqCst), refreshes + 1);
    let mut s = RebuildState::default();
    let t = RippleTuning::default();
    assert!(!s.tick(10, 100, 0xFFFF_FFF0, &t));
    assert!(!s.pending());
    assert!(!s.tick(10, 235, 0xFFFF_FFF0, &t));
    assert!(s.pending());
    s.tick(u64::from(u32::MAX), 0, 0, &t);
    assert!(s.tick(0, 0, 0, &t));
    assert!(!s.tick(1, 0, 0, &t));
    assert_eq!(s.elapsed(), 0);
}

#[test]
fn resource_failure_retry_preserves_old_strip_and_missing_rng_is_explicit() {
    let mut walk = StripWalk::default();
    let p = parameters();
    let extent = point(10000.0, 10000.0);
    for i in 0..4 {
        walk.candidate(straight(100.0 + i as f32 * 16.0), extent, &p, &mut || false);
    }
    walk.candidate(straight(extent.x), extent, &p, &mut || false);
    assert_eq!(walk.strips[0].len(), 8);
    assert_eq!(walk.strips.len(), 2);
    walk.candidate(straight(200.0), extent, &p, &mut || true);
    assert_eq!(walk.strips.len(), 2);
    assert_eq!(walk.strips[1].len(), 2);
    let (mut w, ground, _, shared) = owner();
    w.shutdown();
    w.init(
        ground,
        Arc::new(Factory::default()),
        &Source::default(),
        shared,
        arithmetic(),
    )
    .unwrap();
    assert_eq!(w.update(zoom(0), true), Err(Error::UndefinedTexturePreparation));
    assert!(!w.shores()[0].strips.is_empty());
    let (mut w, _, _, shared) = owner();
    shared.lock().unwrap().random = None;
    assert_eq!(w.update(zoom(0), true), Err(Error::UnsetRandomState));
}

#[test]
fn shared_random_and_state_survive_owners_reset_and_empty_animation() {
    let (mut first, ground, factory, shared) = owner();
    let mut draw = Draw::default();
    first.update_with_renderer(zoom(0), true, &mut draw).unwrap();
    let after_first = shared.lock().unwrap().random.unwrap();
    assert_ne!(after_first.state, 1);
    let mut second = WaterRipple::default();
    second
        .init(
            ground.clone(),
            factory,
            &Source::default(),
            shared.clone(),
            arithmetic(),
        )
        .unwrap();
    let mut expected = after_first;
    second.update_with_renderer(zoom(0), true, &mut draw).unwrap();
    let bounds = shared.lock().unwrap().tuning.period_bounds;
    for shore in second.shores() {
        for strip in &shore.strips {
            let animation = Animation::prepare(8, bounds, &mut expected);
            assert_eq!(strip.animation, animation);
        }
    }
    assert_eq!(shared.lock().unwrap().random, Some(expected));
    assert_eq!(draw.packed, 1); // Packed state belongs to the process, not the owner.
    shared.lock().unwrap().reset();
    assert_eq!(shared.lock().unwrap().random, Some(expected));
    assert_eq!(shared.lock().unwrap().render_state, Some(123));

    // Remove every strip; an empty tick has no random operation to service.
    first.terrain_edit_packed(200, 20).unwrap();
    assert!(first.shores().iter().all(|s| s.strips.is_empty()));
    shared.lock().unwrap().random = None;
    first.do_message(RippleMessage::AnimationTick(0)).unwrap();
}

#[test]
fn owner_reference_pick_shutdown_reinit_keeps_collection_and_first_update() {
    let (mut w, ground, factory, shared) = owner();
    assert_eq!(w.references().count(), 0);
    assert_eq!(w.remove_ref(), 0);
    let mut out = Some(lifecycle::Interface::Message);
    assert!(!w.query_interface(99, &mut out));
    assert_eq!(out, Some(lifecycle::Interface::Message));
    assert!(w.query_interface(1, &mut out));
    assert_eq!(w.add_ref(), 2);
    assert_eq!(w.release(), lifecycle::Release::Count(1));
    let mut words = [1, 2, 3, 4];
    assert!(!w.pick(&mut words));
    assert_eq!(words, [1, 2, 3, 4]);
    w.update(zoom(0), true).unwrap();
    let count = w.shores()[0].strips.len();
    assert!(w.shutdown());
    assert_eq!(w.shores()[0].strips.len(), count);
    assert_eq!(
        &w.lifecycle_events[w.lifecycle_events.len() - 3..],
        &[
            lifecycle::Event::Unsubscribe,
            lifecycle::Event::ReleaseTerrain,
            lifecycle::Event::ReleaseBinding
        ]
    );
    let refreshes = ground.refreshes.load(Ordering::SeqCst);
    w.init(ground.clone(), factory, &Source::default(), shared, arithmetic())
        .unwrap();
    let mut draw = Draw::default();
    assert_eq!(
        w.update_with_renderer(zoom(0), true, &mut draw),
        Ok(RippleUpdate::Drawn { strips: 0 })
    );
    assert_eq!(ground.refreshes.load(Ordering::SeqCst), refreshes);
    assert_eq!(w.release(), lifecycle::Release::Delete);
    assert!(w.is_initialized());
}
