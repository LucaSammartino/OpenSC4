// SPDX-License-Identifier: GPL-3.0-or-later
//! Numerical conformance from lighting.md §11; additional lifecycle and safe
//! boundary tests cover resource ownership, partial failure and preserved quirks.
use super::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Empty;
impl PropertyHolder for Empty {}
struct NoResources;
impl Resources for NoResources {
    fn image(&mut self, _: u32, _: u32) -> Option<Box<dyn TexturePixels>> {
        None
    }
}
#[derive(Default)]
struct Props {
    floats: BTreeMap<u32, f32>,
    colors: BTreeMap<u32, Color>,
    booleans: BTreeMap<u32, bool>,
}
impl PropertyHolder for Props {
    fn float(&self, id: u32) -> Option<f32> {
        self.floats.get(&id).copied()
    }
    fn color(&self, id: u32) -> Option<Color> {
        self.colors.get(&id).copied()
    }
    fn boolean(&self, id: u32) -> Option<bool> {
        self.booleans.get(&id).copied()
    }
}
struct FlatTerrain {
    cells: usize,
    peak: Option<(usize, usize, f32)>,
}
impl Terrain for FlatTerrain {
    fn cell_counts(&self) -> (usize, usize) {
        (self.cells, self.cells)
    }
    fn initialized(&self) -> bool {
        true
    }
    fn altitude(&self, x: usize, z: usize) -> f32 {
        self.peak
            .filter(|&(i, j, _)| x == i && z == j)
            .map_or(0.0, |(_, _, h)| h)
    }
    fn normal(&self, _: f32, _: f32) -> Vector {
        UP
    }
}
pub(super) fn manager() -> LightingManager {
    let mut m = LightingManager::new(LightingStatics::new(Parameters::shipped()), RegisterPrecision::Bits64);
    assert!(m
        .init(0, Some(Arc::new(Empty)), &mut NoResources, &mut Vec::new())
        .unwrap());
    m.do_zoom_and_rotation_change(0, 0).unwrap();
    m
}
pub(super) fn city(m: &mut LightingManager, cells: usize) {
    let terrain = Arc::new(FlatTerrain { cells, peak: None });
    m.do_message(
        LightingMessage::CityStarted {
            terrain: terrain.clone(),
            city_terrain: Some(terrain),
            zoom: 0,
            rotation: 0,
        },
        &mut Vec::new(),
    )
    .unwrap();
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.00005, "{a} != {b}");
}
fn close_color(a: Color, b: Color) {
    for i in 0..3 {
        close(a[i], b[i]);
    }
}
fn stream(normals: &[Vector], stride: usize) -> Vec<u8> {
    let mut bytes = vec![0x55; normals.len() * stride];
    for (i, n) in normals.iter().enumerate() {
        for j in 0..3 {
            bytes[i * stride + 4 * j..i * stride + 4 * j + 4].copy_from_slice(&n[j].to_le_bytes());
        }
    }
    bytes
}
const IDENTITY: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

#[test]
fn conformance_1_day_keys_and_wrap_month() {
    let mut m = manager();
    for (time, expected) in [(0.0, [0.5, 0.5, 0.78]), (6.0, [0.875, 0.875, 0.945]), (12.0, WHITE)] {
        m.set_time_of_day(time, 7, false, &mut Vec::new()).unwrap();
        close_color(m.day_light(), expected);
    }
    assert_eq!(m.month(), 6);
    m.set_time_of_day(13.0, 0, false, &mut Vec::new()).unwrap(); // month ignored without wrap
    let time = m.get_time_of_day();
    assert_eq!(
        m.set_time_of_day(0.0, 0, false, &mut Vec::new()),
        Err(LightingError::InvalidMonth)
    );
    assert_eq!(m.get_time_of_day(), time);
}

#[test]
fn conformance_2_threshold_and_untinted_night_announcements() {
    let mut m = manager();
    city(&mut m, 2);
    m.day_keys = vec![[0.81; 3]];
    let mut events = Vec::new();
    assert!(m.set_time_of_day(12.0, 1, false, &mut events).unwrap());
    events.clear();
    m.day_keys = vec![[0.79; 3]];
    assert!(m.set_time_of_day(12.0, 1, false, &mut events).unwrap());
    assert_eq!(events.iter().filter(|e| **e == Event::Post(0xc9da96e9)).count(), 1);
    assert!(m.is_night());
    events.clear();
    assert!(!m.set_time_of_day(12.0, 1, false, &mut events).unwrap());
    assert!(events.is_empty());
    m.day_keys = vec![[0.81; 3]];
    m.set_time_of_day(12.0, 1, false, &mut events).unwrap();
    assert_eq!(events[0], Event::Post(0xc9da96e8));
    events.clear();
    m.day_keys = vec![[0.810001; 3]];
    assert!(!m.set_time_of_day(12.0, 1, false, &mut events).unwrap());
    assert!(events.is_empty());
    m.day_keys = vec![[0.5; 3]];
    m.set_lighting_tint(WHITE, 0.5, false, &mut events).unwrap();
    assert!(m.is_night()); // 0.5 untinted, despite 0.75 tinted
    assert_eq!(events.last(), Some(&Event::Redraw));
}

#[test]
fn conformance_3_day_night_fractions() {
    let mut m = manager();
    city(&mut m, 2);
    m.minimum_red = 0.5;
    for (red, expected) in [(0.9, (0.5, 0.0)), (0.65, (0.0, 0.5))] {
        m.day_keys = vec![[red; 3]];
        m.set_time_of_day(12.0, 1, false, &mut Vec::new()).unwrap();
        close(m.fractions().0, expected.0);
        close(m.fractions().1, expected.1);
    }
    m.minimum_red = 0.8;
    m.day_keys = vec![[0.8; 3]];
    m.set_time_of_day(12.0, 1, false, &mut Vec::new()).unwrap();
    close(m.fractions().1, 1.0);
}

#[test]
fn tint_reset_recomputes_light_and_redraws_only_above_announced_threshold() {
    let mut m = manager();
    city(&mut m, 2);
    let mut events = Vec::new();
    m.set_time_of_day(12.0, 1, false, &mut events).unwrap();
    let fractions = m.fractions();
    events.clear();
    m.set_lighting_tint([0.0; 3], 0.5, false, &mut events).unwrap();
    assert_eq!(m.get_global_light(), [0.5; 3]);
    assert_eq!(m.day_light(), WHITE);
    assert_eq!(m.fractions(), fractions);
    assert!(!m.is_night());
    assert_eq!(events, [Event::Fractions { day: 1.0, night: 0.0 }, Event::Redraw]);
    events.clear();
    m.reset_lighting_tint(false, &mut events).unwrap();
    assert_eq!(m.get_global_light(), WHITE);
    assert_eq!(m.fade_color, Parameters::shipped().fade_color);
    assert_eq!(events, [Event::Fractions { day: 1.0, night: 0.0 }, Event::Redraw]);
    events.clear();
    m.reset_lighting_tint(false, &mut events).unwrap();
    assert!(events.is_empty());
    // A tint below the change threshold is stored even without notification.
    m.set_lighting_tint([0.0; 3], 0.000001, false, &mut events).unwrap();
    assert!(m.get_global_light()[0] < 1.0);
    assert!(events.is_empty());
    m.reset_lighting_tint(false, &mut events).unwrap();
    assert_eq!(m.get_global_light(), WHITE);
    assert!(events.is_empty());
}

#[test]
fn conformance_4_lambert_and_rotations() {
    let mut m = manager();
    m.statics.update(|p| p.use_maps = false);
    let expected = [
        0.05 + 1.0 / 3.0_f32.sqrt(),
        0.05 + 1.0 / 3.0_f32.sqrt() + 1.0 / 30.0,
        0.05 + 0.8 / 3.0_f32.sqrt() + 1.0 / 6.0,
    ];
    for r in 0..4 {
        m.do_zoom_and_rotation_change(0, r).unwrap();
        close_color(m.get_color_up().unwrap(), expected);
    }
}

#[test]
fn conformance_5_model_weights_and_network_priority() {
    let mut m = manager();
    city(&mut m, 2);
    m.maps[0] = Some(ImageArgb8::new(1, 1, 1, vec![0xff808080]).unwrap());
    m.maps[1] = m.maps[0].clone();
    let bounds = Bounds {
        min: [8.0, 0.0, 8.0],
        max: [12.0, 2.0, 12.0],
    };
    let half = 128.0 / 255.0;
    for (occupant, w) in [
        (None, 0.4),
        (
            Some(Occupant {
                flora: true,
                ..Occupant::default()
            }),
            0.9,
        ),
        (
            Some(Occupant {
                vehicle: true,
                ..Occupant::default()
            }),
            0.7,
        ),
    ] {
        close_color(m.get_model_light(bounds, occupant).unwrap(), [w * half + 1.0 - w; 3]);
    }
    m.statics.update(|p| p.debug_model = true);
    close_color(
        m.get_model_light(
            bounds,
            Some(Occupant {
                network_flag_40000: true,
                flora: true,
                ..Occupant::default()
            }),
        )
        .unwrap(),
        [0.4 * half + 0.6; 3],
    );
    close_color(
        m.get_model_light(bounds, None).unwrap(),
        [10.0 / 256.0, 10.0 / 256.0, 1.0],
    );
}

#[test]
fn conformance_6_vertex_down_is_up_and_strides_are_respected() {
    let m = manager();
    let normals = stream(&[UP, [0.0, -1.0, 0.0]], 16);
    let mut colors = [0x55; 16];
    m.get_vertex_colors(2, &normals, 16, &mut colors, 8, IDENTITY).unwrap();
    assert_eq!(&colors[0..4], &colors[8..12]);
    assert_eq!(colors[3], 255);
    assert_eq!(&colors[4..8], &[0x55; 4]);
    let before = colors;
    assert_eq!(
        m.get_vertex_colors(2, &normals, 8, &mut colors, 8, IDENTITY),
        Err(LightingError::InvalidStride)
    );
    assert_eq!(colors, before);
}

#[test]
fn conformance_7_shadow_strength() {
    let mut m = manager();
    city(&mut m, 1);
    for (depth, night, expected) in [(25.0, 0.0, 0.5), (60.0, 0.0, 0.0), (-3.0, 1.0, 0.4)] {
        m.heights[0] = 0.0;
        m.shadows[0] = depth;
        m.night_fraction = night;
        close(m.get_shadow_strength_from_location([0.0, 0.0]).unwrap(), expected);
    }
}

#[test]
fn conformance_8_peak_sweep_and_skipped_rect() {
    let mut m = manager();
    m.shadows_on = true;
    m.sun = [std::f32::consts::FRAC_1_SQRT_2, -std::f32::consts::FRAC_1_SQRT_2, 0.0];
    let terrain = Arc::new(FlatTerrain {
        cells: 10,
        peak: Some((0, 5, 100.0)),
    });
    m.do_message(
        LightingMessage::CityStarted {
            terrain: terrain.clone(),
            city_terrain: Some(terrain),
            zoom: 0,
            rotation: 0,
        },
        &mut Vec::new(),
    )
    .unwrap();
    for row in 1..=8 {
        close(
            m.shadow_depth([row as f32 * 16.0, 0.0, 80.0]).unwrap(),
            100.0 - 11.2 * row as f32,
        );
    }
    close(m.shadow_depth([144.0, 0.0, 80.0]).unwrap(), 0.0);
    let changed = m.changed_shadow_rect();
    assert_eq!(
        changed,
        Rect {
            min_x: 5,
            min_z: 0,
            max_x: 5,
            max_z: 8
        }
    );
    m.sun[0] = -1.0;
    m.find_hill_shadows().unwrap();
    assert_eq!(m.changed_shadow_rect(), changed);
    m.sun = [1.0, -1.0, 3.0];
    m.find_hill_shadows().unwrap();
    assert_eq!(m.changed_shadow_rect(), changed);
    m.sun = [1.0, -1.0, 0.0];
    m.find_hill_shadows().unwrap();
    m.find_hill_shadows().unwrap();
    assert_eq!(
        m.changed_shadow_rect(),
        Rect {
            min_x: -1,
            min_z: -1,
            max_x: 0,
            max_z: 0
        }
    );
}

#[test]
fn conformance_9_bytes_and_ties() {
    for (input, expected) in [
        (0.0, 0),
        (1.0, 255),
        (0.5, 128),
        (0.999, 255),
        (0.0019, 0),
        (0.5 / 256.0, 0),
        (1.5 / 256.0, 2),
        (2.5 / 256.0, 2),
    ] {
        assert_eq!(color_byte(input), expected);
    }
}

#[test]
fn image_sampling_stride_row_spill_minimum_and_test_pattern() {
    let pattern = ImageArgb8::make_test_pattern();
    assert_eq!(pattern.pixels()[32], 0xff000000);
    assert_eq!(pattern.pixels()[33], 0xffff0000);
    assert_eq!(pattern.pixels()[32 * 64], 0xff000000);
    assert_eq!(pattern.pixels()[33 * 64], 0xff00ff00);
    assert_eq!(pattern.pixels()[63 * 64 + 63], 0xffffff00);
    let image = ImageArgb8::new(2, 2, 2, vec![0xff000000, 0xffff0000, 0xff00ff00, 0xff0000ff]).unwrap();
    close_color(image.sample(0.75, 0.0).unwrap(), [0.5, 0.5, 0.0]);
    assert_eq!(image.sample(0.75, 0.75), Err(LightingError::SampleOutsideImage));
    let padded = ImageArgb8::new(1, 2, 2, vec![0xff880000, 0, 0xffff0000, 0]).unwrap();
    assert_eq!(padded.find_min_red_component_value(), 0.0); // LT7: padding inside vector end
    close(
        ImageArgb8::new(0, 0, 0, vec![]).unwrap().find_min_red_component_value(),
        1.0,
    );
    assert_eq!(
        ImageArgb8::new(3, 2, 2, vec![0; 6]),
        Err(LightingError::InvalidDimensions)
    );
    assert_eq!(ImageArgb8::new(2, 2, 2, vec![0; 3]), Err(LightingError::BufferTooShort));
}

#[test]
fn surface_map_size_drives_model_map_coordinates_and_rotation() {
    let mut m = manager();
    m.maps[0] = Some(ImageArgb8::new(2, 2, 2, vec![0; 4]).unwrap());
    m.maps[1] = Some(ImageArgb8::new(4, 4, 4, (0..16).map(|i| 0xff000000 | (i * 0x110000)).collect()).unwrap());
    close_color(m.get_model_color(UP).unwrap(), [85.0 / 255.0, 0.0, 0.0]); // (1,1) using 2x2 coordinates
    m.maps[0] = Some(ImageArgb8::make_test_pattern());
    m.maps[1] = m.maps[0].clone();
    m.do_zoom_and_rotation_change(0, 1).unwrap();
    close_color(m.get_color([1.0, 0.0, 0.0]).unwrap(), [0.0, 0.0, 0.0]); // inverse view maps +x to -z
    close_color(m.get_color([0.0, 0.0, 1.0]).unwrap(), [1.0, 0.0, 0.0]);
    m.maps[1] = None;
    assert_eq!(m.get_model_color(UP), Err(LightingError::MissingModelMap));
}

#[test]
fn persistent_properties_override_every_time_and_rebuild_keys() {
    let statics = LightingStatics::new(Parameters::shipped());
    let mut first = Props::default();
    first.floats.insert(0x29189beb, 0.25);
    first.colors.insert(0x29189bf0, [0.8, 0.7, 0.6]);
    first.booleans.insert(0x29189bed, true);
    let mut m = LightingManager::new(statics.clone(), RegisterPrecision::Bits64);
    m.init(2, Some(Arc::new(first)), &mut NoResources, &mut Vec::new())
        .unwrap();
    assert_eq!(m.day_light(), [0.8, 0.7, 0.6]);
    assert_eq!(m.scaled_light, [0.2, 0.175, 0.15]);
    assert_eq!(m.maps[0].as_ref().unwrap().width(), 64);
    assert!(m.maps[2].is_none());
    let mut second = LightingManager::new(statics.clone(), RegisterPrecision::Bits64);
    second
        .init(0, Some(Arc::new(Empty)), &mut NoResources, &mut Vec::new())
        .unwrap();
    close(statics.snapshot().light_scale, 0.25);
    assert_eq!(second.day_light(), [0.8, 0.7, 0.6]);
    m.do_message(
        LightingMessage::RenderPropertiesChanged {
            threshold: 0.123,
            shadow_setting: 2,
            zoom: 0,
            rotation: 0,
        },
        &mut Vec::new(),
    )
    .unwrap();
    close(statics.snapshot().change_threshold, 0.123);
    m.do_message(LightingMessage::ExemplarChanged, &mut Vec::new()).unwrap();
    close(statics.snapshot().change_threshold, 0.123); // missing property keeps the override
}

#[test]
fn lifecycle_subscription_order_identity_and_owned_references() {
    let mut m = LightingManager::new(LightingStatics::new(Parameters::shipped()), RegisterPrecision::Bits64);
    assert_eq!(m.get_time_of_day(), 12.0);
    assert!(!m.is_night());
    assert_eq!(m.get_global_light(), WHITE);
    assert_eq!(m.query_interface(123).unwrap(), None);
    assert_eq!(m.query_interface(1).unwrap(), Some(Interface::Lighting));
    assert_eq!(
        m.query_interface(MESSAGE_TARGET_IID).unwrap(),
        Some(Interface::MessageTarget)
    );
    assert_eq!(m.release(), 1);
    let exemplar: Arc<dyn PropertyHolder> = Arc::new(Empty);
    let weak = Arc::downgrade(&exemplar);
    let mut events = Vec::new();
    m.init(0, Some(exemplar.clone()), &mut NoResources, &mut events)
        .unwrap();
    assert_eq!(events, SUBSCRIPTIONS.map(Event::Subscribe));
    drop(exemplar);
    assert!(weak.upgrade().is_some());
    assert!(!m.init(2, None, &mut NoResources, &mut events).unwrap());
    assert_eq!(events.len(), 7);
    let terrain: Arc<dyn Terrain> = Arc::new(FlatTerrain { cells: 2, peak: None });
    let weak_terrain = Arc::downgrade(&terrain);
    m.do_message(
        LightingMessage::CityStarted {
            terrain: terrain.clone(),
            city_terrain: Some(terrain.clone()),
            zoom: 0,
            rotation: 0,
        },
        &mut events,
    )
    .unwrap();
    drop(terrain);
    assert_eq!(weak_terrain.strong_count(), 2); // two acquired references, never an unowned release
    events.clear();
    assert!(m.shutdown(&mut events));
    assert_eq!(events, SUBSCRIPTIONS.map(Event::Unsubscribe));
    assert!(!m.view_applied());
    assert!(weak.upgrade().is_none());
    assert_eq!(weak_terrain.strong_count(), 1); // city terrain belongs to destructor, not Shutdown
    assert!(!m.shutdown(&mut events));
    assert_eq!(m.release(), 0);
    assert!(weak_terrain.upgrade().is_none());
    assert_eq!(m.add_ref(), Err(LightingError::Released));
    assert_eq!(
        m.init(0, None, &mut NoResources, &mut events),
        Err(LightingError::Released)
    );
}

struct CountedPixels {
    dropped: Arc<AtomicUsize>,
    bad: bool,
}
impl TexturePixels for CountedPixels {
    fn dimensions(&self) -> (usize, usize) {
        (1, 1)
    }
    fn pixel(&self, _: usize, _: usize) -> Option<u32> {
        if self.bad {
            None
        } else {
            Some(0xff804020)
        }
    }
}
impl Drop for CountedPixels {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}
struct TestResources {
    dropped: Arc<AtomicUsize>,
    calls: Vec<(u32, u32)>,
    fail_at: Option<usize>,
}
impl Resources for TestResources {
    fn image(&mut self, group: u32, instance: u32) -> Option<Box<dyn TexturePixels>> {
        self.calls.push((group, instance));
        Some(Box::new(CountedPixels {
            dropped: self.dropped.clone(),
            bad: self.fail_at == Some(self.calls.len()),
        }))
    }
}
#[test]
fn resource_copy_lifetime_and_partial_failure_are_owned() {
    let dropped = Arc::new(AtomicUsize::new(0));
    let mut resources = TestResources {
        dropped: dropped.clone(),
        calls: vec![],
        fail_at: Some(2),
    };
    let mut m = LightingManager::new(LightingStatics::new(Parameters::shipped()), RegisterPrecision::Bits64);
    let mut events = Vec::new();
    assert_eq!(
        m.init(0, None, &mut resources, &mut events),
        Err(LightingError::BufferTooShort)
    );
    assert!(!m.initialized());
    assert!(events.is_empty());
    assert_eq!(dropped.load(Ordering::SeqCst), 2);
    assert!(m.maps[0].is_some()); // copied first image does not borrow the destroyed decoder
                                  // LT7: invalid foreign pixels interrupted Init, without a retry contract.
    drop(m);
    let mut m = LightingManager::new(LightingStatics::new(Parameters::shipped()), RegisterPrecision::Bits64);
    resources.fail_at = None;
    assert!(m.init(0, None, &mut resources, &mut events).unwrap());
    assert_eq!(dropped.load(Ordering::SeqCst), 6);
    assert_eq!(
        &resources.calls[2..],
        &[0x0917660c, 0x0917660d, 0x0917660e, 0x0917660f].map(|id| (0xa9179251, id))
    );
    close(m.minimum_red, 128.0 / 255.0);
    m.shutdown(&mut events);
    assert!(m.maps.iter().all(Option::is_none));
}

#[test]
fn fade_gradient_stays_at_view_change_and_vertices_skip_it_without_shadows() {
    let mut m = manager();
    city(&mut m, 2);
    let p = [0.0, 0.0, 0.0];
    close(m.fade_term(p), 0.15);
    m.set_time_of_day(0.0, 1, false, &mut Vec::new()).unwrap();
    close(m.fade_strength, 0.0);
    close(m.fade_term(p), 0.15); // deliberately not recomputed on time update
    let n = stream(&[UP], 12);
    let mut colors = [0; 6];
    m.get_terrain_vertex_colors(0, 0, 1, &n, 12, &mut colors, 6).unwrap();
    assert_eq!(&colors[..3], &m.get_color_up_bytes().unwrap());
    assert_ne!(&colors[..3], &m.get_terrain_color(p, UP).unwrap());
    m.do_zoom_and_rotation_change(0, 0).unwrap();
    close(m.fade_term(p), 0.0);
}

#[test]
fn suppressed_fade_still_tints_and_does_not_post_dawn_dusk() {
    let mut m = manager();
    city(&mut m, 1);
    let mut events = Vec::new();
    m.set_time_of_day(0.0, 1, true, &mut events).unwrap();
    assert!(!events.iter().any(|e| matches!(e, Event::Post(_))));
    assert!(m.is_night());
    assert_eq!(m.fade_color, [0.0; 3]);
    m.set_lighting_tint([1.0, 0.0, 0.0], 0.5, true, &mut events).unwrap();
    assert_eq!(m.fade_color, [0.5, 0.0, 0.0]);
    close(m.fade_strength, 0.0);
    m.reset_lighting_tint(true, &mut events).unwrap();
    close_color(m.get_global_light(), [0.5, 0.5, 0.78]);
}

#[test]
fn native_fit_cutoff_and_empty_projection_preserve_output() {
    let points = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
    let uvs = points.map(|[x, y]| [2.0 * x + 3.0 * y + 5.0, -x + 4.0 * y - 2.0]);
    assert_eq!(
        fit_shadow_plane(&points, &uvs, RegisterPrecision::Bits64).unwrap(),
        [[2.0, 3.0, 5.0], [-1.0, 4.0, -2.0]]
    );
    assert_eq!(
        fit_shadow_plane(&[[0.0; 2]; 3], &[[0.0; 2]; 3], RegisterPrecision::Bits64),
        Err(LightingError::DegenerateProjection)
    );
    let small = [[0.0, 0.0], [0.1, 0.0], [0.0, 0.1]];
    assert_eq!(
        fit_shadow_plane(&small, &small, RegisterPrecision::Bits64),
        Err(LightingError::DegenerateProjection)
    );
    let m = manager();
    let mut output = [9.0; 16];
    assert_eq!(
        m.calc_shadow_projection(
            &[],
            &[],
            ProjectionTransform {
                linear: IDENTITY,
                translation: [0.0; 3],
                scale: 1.0,
                flags: 0
            },
            &mut output
        ),
        Ok(false)
    );
    assert_eq!(output, [9.0; 16]);
}

#[test]
fn manager_and_providers_are_send_sync() {
    fn assert_traits<T: Send + Sync>() {}
    assert_traits::<LightingManager>();
    assert_traits::<LightingStatics>();
}

#[test]
fn deleting_destructor_always_drops_resources_but_frees_storage_only_for_bit_zero() {
    for flags in [0, 1, 2, 3] {
        let mut m = Box::new(manager());
        let exemplar: Arc<dyn PropertyHolder> = Arc::new(Empty);
        let weak = Arc::downgrade(&exemplar);
        m.exemplar = Some(exemplar);
        let storage = m.deleting_destructor(flags);
        assert!(weak.upgrade().is_none());
        assert_eq!(storage.is_none(), flags & 1 != 0);
        if let Some(mut destroyed) = storage {
            assert_eq!(destroyed.add_ref(), Err(LightingError::Released));
            assert!(destroyed.exemplar.is_none());
            assert!(destroyed.day_keys.is_empty());
        }
    }
}

#[test]
fn sweep_drift_both_directions_uses_ordered_in_place_interpolation() {
    let mut m = manager();
    city(&mut m, 2);
    m.heights[1] = 100.0;
    for (sz, expected) in [(-0.5, [38.8, 38.8, 0.0]), (0.5, [0.0, 38.8, 38.8])] {
        m.sun = [1.0, -1.0, sz];
        m.shadows.fill(0.0);
        m.find_hill_shadows().unwrap();
        close_color(m.shadows[3..6].try_into().unwrap(), expected);
    }
}

struct SlopedTerrain {
    samples: std::sync::Mutex<Vec<[f32; 2]>>,
}
impl Terrain for SlopedTerrain {
    fn cell_counts(&self) -> (usize, usize) {
        (8, 8)
    }
    fn initialized(&self) -> bool {
        true
    }
    fn altitude(&self, x: usize, z: usize) -> f32 {
        (100 * x + z) as f32
    }
    fn normal(&self, x: f32, z: f32) -> Vector {
        self.samples.lock().unwrap().push([x, z]);
        normalize([x, 32.0, z])
    }
}
#[test]
fn all_grid_rotations_copy_world_heights_and_rotate_changed_rectangles() {
    let mut m = manager();
    let terrain = Arc::new(SlopedTerrain {
        samples: std::sync::Mutex::new(Vec::new()),
    });
    m.shadows_on = true;
    m.sun = [1.0, -10000.0, 0.0];
    for rotation in 0..4 {
        m.do_message(
            LightingMessage::CityStarted {
                terrain: terrain.clone(),
                city_terrain: Some(terrain.clone()),
                zoom: 0,
                rotation,
            },
            &mut Vec::new(),
        )
        .unwrap();
        for i in 0..=8 {
            for j in 0..=8 {
                close(
                    m.height_above_land([i as f32 * 16.0, 0.0, j as f32 * 16.0]).unwrap(),
                    -((100 * i + j) as f32),
                );
            }
        }
        // A stored column 1,row 2 is inverted according to §3 strides.
        m.changed = Rect {
            min_x: 1,
            min_z: 2,
            max_x: 1,
            max_z: 2,
        };
        let expected = match rotation {
            0 => (2, 1),
            1 => (7, 2),
            2 => (6, 7),
            _ => (1, 6),
        };
        assert_eq!(
            m.changed_world_rect(),
            Rect {
                min_x: expected.0,
                min_z: expected.1,
                max_x: expected.0,
                max_z: expected.1
            }
        );
    }
}

#[test]
fn model_normal_uses_center_or_four_corners_without_renormalizing_average() {
    let mut m = manager();
    let terrain = Arc::new(SlopedTerrain {
        samples: std::sync::Mutex::new(Vec::new()),
    });
    m.do_message(
        LightingMessage::CityStarted {
            terrain: terrain.clone(),
            city_terrain: Some(terrain.clone()),
            zoom: 0,
            rotation: 0,
        },
        &mut Vec::new(),
    )
    .unwrap();
    let small = Bounds {
        min: [16.0, 0.0, 16.0],
        max: [32.0, 2.0, 32.0],
    };
    m.get_model_light(small, None).unwrap();
    assert_eq!(*terrain.samples.lock().unwrap(), [[24.0, 24.0]]);
    terrain.samples.lock().unwrap().clear();
    let big = Bounds {
        min: [0.0, 0.0, 16.0],
        max: [64.0, 2.0, 32.0],
    };
    let light = m.get_model_light(big, None).unwrap();
    assert_eq!(
        *terrain.samples.lock().unwrap(),
        [[0.0, 16.0], [0.0, 32.0], [64.0, 16.0], [64.0, 32.0]]
    );
    let average = scale(
        [
            [0.0, 32.0, 16.0],
            [0.0, 32.0, 32.0],
            [64.0, 32.0, 16.0],
            [64.0, 32.0, 32.0],
        ]
        .map(normalize)
        .into_iter()
        .fold([0.0; 3], add),
        0.25,
    );
    close_color(light, lerp(WHITE, m.get_model_color(average).unwrap(), 0.4));
    terrain.samples.lock().unwrap().clear();
    m.shadows_on = true;
    m.get_model_light(big, None).unwrap();
    assert!(terrain.samples.lock().unwrap().is_empty()); // shadow path starts from UP, not terrain
}

struct MutableTerrain {
    heights: std::sync::RwLock<Vec<f32>>,
    ready: std::sync::atomic::AtomicBool,
}
impl Terrain for MutableTerrain {
    fn cell_counts(&self) -> (usize, usize) {
        (2, 2)
    }
    fn initialized(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }
    fn altitude(&self, x: usize, z: usize) -> f32 {
        self.heights.read().unwrap()[x * 3 + z]
    }
    fn normal(&self, _: f32, _: f32) -> Vector {
        UP
    }
}
fn rect(x: i32, z: i32) -> Rect {
    Rect {
        min_x: x,
        min_z: z,
        max_x: x,
        max_z: z,
    }
}
#[test]
fn long_operations_accumulate_dirty_vertices_then_sweep_and_redisplay() {
    let mut m = manager();
    m.shadows_on = true;
    m.sun = [1.0, -1.0, 0.0];
    let terrain = Arc::new(MutableTerrain {
        heights: std::sync::RwLock::new(vec![0.0; 9]),
        ready: std::sync::atomic::AtomicBool::new(true),
    });
    let mut events = Vec::new();
    m.do_message(
        LightingMessage::CityStarted {
            terrain: terrain.clone(),
            city_terrain: Some(terrain.clone()),
            zoom: 0,
            rotation: 0,
        },
        &mut events,
    )
    .unwrap();
    m.do_message(LightingMessage::LongOperationBegin, &mut events).unwrap();
    terrain.heights.write().unwrap()[1] = 100.0;
    terrain.heights.write().unwrap()[3] = 50.0;
    m.do_message(LightingMessage::TerrainChanged(rect(0, 1)), &mut events)
        .unwrap();
    m.do_message(LightingMessage::TerrainChanged(rect(1, 0)), &mut events)
        .unwrap();
    assert!(events.is_empty());
    close(m.height_above_land([0.0, 0.0, 16.0]).unwrap(), 0.0);
    assert_eq!(
        m.dirty,
        Rect {
            min_x: 0,
            min_z: 0,
            max_x: 1,
            max_z: 1
        }
    );
    m.do_message(LightingMessage::LongOperationEnd, &mut events).unwrap();
    close(m.height_above_land([0.0, 0.0, 16.0]).unwrap(), -100.0);
    close(m.shadow_depth([16.0, 0.0, 16.0]).unwrap(), 88.8);
    let changed = Rect {
        min_x: 0,
        min_z: 0,
        max_x: 2,
        max_z: 1,
    };
    assert_eq!(
        events,
        [Event::TerrainLightingChanged(changed), Event::RedisplayTerrain(changed)]
    );
    events.clear();
    m.do_message(LightingMessage::TerrainChanged(rect(0, 1)), &mut events)
        .unwrap();
    assert!(events.is_empty()); // unchanged sweep does not invalidate
    terrain.ready.store(false, Ordering::SeqCst);
    terrain.heights.write().unwrap()[1] = 200.0;
    m.do_message(LightingMessage::TerrainChanged(rect(0, 1)), &mut events)
        .unwrap();
    close(m.height_above_land([0.0, 0.0, 16.0]).unwrap(), -100.0);
    assert!(events.is_empty());
}

#[test]
fn render_change_updates_threshold_and_only_changed_shadow_setting_emits() {
    let mut m = manager();
    city(&mut m, 2);
    let mut events = Vec::new();
    m.do_message(
        LightingMessage::RenderPropertiesChanged {
            threshold: 0.2,
            shadow_setting: 1,
            zoom: 2,
            rotation: 1,
        },
        &mut events,
    )
    .unwrap();
    close(m.statics.snapshot().change_threshold, 0.2);
    assert!(events.is_empty());
    assert_eq!(m.rotation, 0);
    m.do_message(
        LightingMessage::RenderPropertiesChanged {
            threshold: 0.3,
            shadow_setting: 2,
            zoom: 2,
            rotation: 1,
        },
        &mut events,
    )
    .unwrap();
    assert!(m.shadows_enabled());
    assert_eq!(m.rotation, 1);
    assert_eq!(events, [Event::Fractions { day: 1.0, night: 0.0 }, Event::Redraw]);
    events.clear();
    m.do_message(LightingMessage::ViewChanged { zoom: 0, rotation: 3 }, &mut events)
        .unwrap();
    assert_eq!(m.rotation, 3);
    assert!(events.is_empty());
}

#[test]
fn point_terrain_and_vertex_color_paths_keep_distinct_fade_shadow_rules() {
    let mut m = manager();
    city(&mut m, 2);
    let p = [0.0, 1000.0, 0.0];
    let n = [1.0, 0.0, 0.0];
    m.shadows_on = true;
    m.shadows[0] = 25.0;
    let bent = m.bend_shadowed_normal(n, 25.0);
    let base = m.get_color(n).unwrap();
    let shadow = m.get_color(bent).unwrap();
    assert_eq!(m.get_color_at(n, p).unwrap(), base); // p is above shadow
    assert_eq!(m.get_color_at_bytes([0.0, 0.0, 0.0], n).unwrap(), color_bytes(shadow));
    let k2 = 0.15 * 0.15;
    assert_eq!(m.get_terrain_color(p, n).unwrap(), color_bytes(scale(shadow, 1.0 - k2))); // lowered to land first
    assert_eq!(
        m.get_terrain_color_floor(p, n).unwrap(),
        color_bytes(add(scale(m.fade_color, k2), product(base, m.scaled_light)))
    ); // never bent
    let up = m.get_color(m.bend_shadowed_normal(UP, 25.0)).unwrap();
    assert_eq!(
        m.get_terrain_lighting([0.0, 0.0, 0.0]).unwrap(),
        color_bytes(lerp(WHITE, up, 0.8))
    );
    let mut colors = [0x55; 8];
    m.get_terrain_vertex_colors(0, 0, 1, &stream(&[n], 12), 12, &mut colors, 8)
        .unwrap();
    assert_eq!(&colors[..3], &color_bytes(scale(shadow, 1.0 - k2)));
    assert_eq!(
        &colors[3..6],
        &color_bytes(add(scale(m.fade_color, k2), product(shadow, m.scaled_light)))
    );
    assert_eq!(&colors[6..], &[0x55; 2]);
}

#[test]
fn time_maps_use_month_center_and_no_terrain_never_announces_or_changes_night() {
    let mut m = manager();
    let pixels = (0..12).flat_map(|month| [0xff000000 | (month * 20) << 16; 2]).collect();
    m.maps[2] = Some(ImageArgb8::new(2, 12, 2, pixels).unwrap());
    let mut events = Vec::new();
    m.set_time_of_day(0.0, 4, false, &mut events).unwrap();
    close(m.day_light()[0], 60.0 / 255.0);
    assert!(events.is_empty());
    assert!(!m.is_night());
    m.set_time_of_day(12.0, 8, false, &mut events).unwrap();
    assert_eq!(m.month(), 3); // no wrap, ignore external month
    m.set_time_of_day(0.0, 8, false, &mut events).unwrap();
    close(m.day_light()[0], 140.0 / 255.0);
    assert_eq!(m.month(), 7);
}

#[test]
fn decal_holder_overrides_only_present_properties() {
    struct Decal;
    impl PropertyHolder for Decal {
        fn float(&self, id: u32) -> Option<f32> {
            if id == 0x085489ca {
                Some(24.0)
            } else {
                None
            }
        }
        fn unsigned(&self, id: u32) -> Option<u32> {
            if id == 0x085489c8 {
                Some(77)
            } else {
                None
            }
        }
        fn pair(&self, id: u32) -> Option<[f32; 2]> {
            if id == 0x085489c9 {
                Some([3.0, 4.0])
            } else {
                None
            }
        }
        fn color(&self, id: u32) -> Option<Color> {
            if id == 0x085489cb {
                Some([0.2, 0.3, 0.4])
            } else {
                None
            }
        }
    }
    let m = manager();
    let values = m.get_decal_shadow_params(&Decal);
    assert_eq!(values.texture, 77);
    assert_eq!((values.a, values.b, values.c), (0.4, 0.55, 24.0));
    assert_eq!(values.pair, [3.0, 4.0]);
    assert_eq!(values.color, [0.2, 0.3, 0.4]);
    assert_eq!(m.get_shadow_params(), (0.4, 0.55, [0.08, 0.06, 0.23]));
}

#[test]
fn known_upper_clamp_and_invalid_buffers_have_checked_boundaries() {
    let mut m = manager();
    city(&mut m, 2);
    assert_eq!(m.terrain_normal([32.0, 0.0, 0.0]), Ok(UP));
    assert_eq!(m.terrain_normal([-5.0, 0.0, -4.0]).unwrap(), UP);
    assert_eq!(
        m.shadow_depth([-1.0, 0.0, 0.0]),
        Err(LightingError::LocationOutsideGrid)
    );
    assert_eq!(
        m.set_time_of_day(f32::NAN, 1, false, &mut Vec::new()),
        Err(LightingError::InvalidTime)
    );
    let mut colors = [0x55; 8];
    assert_eq!(
        m.get_vertex_colors(2, &[0; 12], 12, &mut colors, 4, IDENTITY),
        Err(LightingError::BufferTooShort)
    );
    assert_eq!(colors, [0x55; 8]);
    m.get_vertex_colors(0, &[], 0, &mut [], 0, IDENTITY).unwrap();
    assert_eq!(
        m.get_terrain_vertex_colors(3, 0, 1, &[0; 12], 12, &mut colors, 6),
        Err(LightingError::LocationOutsideGrid)
    );
}
