// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent public lighting.md §11 vectors; expected rows are literal
//! spec results, never computed by the projection implementation under test.
use super::tests::{city, manager};
use super::*;
use std::sync::Mutex;

struct EmptyResources;
impl Resources for EmptyResources {
    fn image(&mut self, _: u32, _: u32) -> Option<Box<dyn TexturePixels>> {
        None
    }
}
#[test]
fn constructor_zero_map_rotation_and_shutdown_retained_matrix() {
    let statics = LightingStatics::new(Parameters::initial());
    statics.update(|p| p.test_pattern = true);
    let mut m = LightingManager::new(statics, RegisterPrecision::Bits64);
    let mut events = Vec::new();
    assert_eq!(m.init(0, None, &mut EmptyResources, &mut events), Ok(true));
    for normal in [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0]] {
        assert_eq!(m.get_color(normal), Ok([0.0; 3]));
        assert_eq!(m.get_model_color(normal), Ok([0.0; 3]));
    }
    m.do_zoom_and_rotation_change(0, 0).unwrap();
    assert_eq!(m.get_color([1.0, 0.0, 0.0]), Ok([1.0, 0.0, 0.0]));
    assert!(m.shutdown(&mut events));
    assert!(!m.view_applied());
    assert_eq!(m.init(0, None, &mut EmptyResources, &mut events), Ok(true));
    assert_eq!(m.get_model_color([1.0, 0.0, 0.0]), Ok([1.0, 0.0, 0.0]));
    m.shutdown(&mut events);
}

#[test]
fn native_initial_state_shared_defaults_and_query_counts() {
    let statics = LightingStatics::new(Parameters::initial());
    let mut m = LightingManager::new(statics.clone(), RegisterPrecision::Bits53);
    assert_eq!(m.refs, 0);
    assert_eq!(m.sun, [1.0, -1.0, -1.0]);
    assert_eq!(m.scaled_light, [0.0; 3]);
    assert_eq!(m.announced_global, [0.0; 3]);
    assert_eq!(m.announced_day, [0.0; 3]);
    assert_eq!(m.axes, [[0.0; 3]; 6]);
    assert_eq!(m.fade_color, [0.0; 3]);
    assert_eq!(m.fractions(), (1.0, 0.0));
    assert_eq!(m.query_interface(123), Ok(None));
    assert_eq!(m.refs, 0);
    assert_eq!(m.query_interface(1), Ok(Some(Interface::Lighting)));
    assert_eq!(
        m.query_interface(MESSAGE_TARGET_IID),
        Ok(Some(Interface::MessageTarget))
    );
    assert_eq!(m.refs, 2);
    assert_eq!(m.release(), 1);
    assert_eq!(m.release(), 0);
    let p = statics.snapshot();
    assert_eq!(
        [p.terrain_weight, p.flora_weight, p.model_weight, p.mobile_weight],
        [0.6; 4]
    );
    assert_eq!((p.night, p.fade_color, p.fade_strength), (WHITE, [0.0, 0.5, 1.0], 0.5));
    assert_eq!((p.decal.texture, p.decal.c), (0x0917660e, 8.0));
    assert_eq!((p.decal.a, p.decal.b, p.decal.color), (0.5, 0.0, [0.0; 3]));
    assert_eq!((p.azimuth, p.elevation, p.normal_bend), (90.0, 45.0, 1.0));
}
#[test]
fn lt6_length_equality_nan_and_sub_binary32_distance() {
    let mut m = manager();
    for precision in [RegisterPrecision::Bits53, RegisterPrecision::Bits64] {
        m.register_precision = precision;
        m.statics.update(|p| p.change_threshold = 0.0001);
        m.announced_global = [0.0; 3];
        for (current, expected) in [
            ([0.00008, 0.00008, 0.0], true),
            ([0.0001, 0.0, 0.0], false),
            ([f32::NAN, 0.0, 0.0], false),
        ] {
            m.day_keys = vec![current];
            assert_eq!(m.set_time_of_day(12.0, 1, false, &mut Vec::new()), Ok(expected));
        }
        m.statics.update(|p| p.change_threshold = 0.0);
        m.day_keys = vec![[f32::from_bits(1), 0.0, 0.0]];
        assert_eq!(m.set_time_of_day(12.0, 1, false, &mut Vec::new()), Ok(true));
    }
}
struct TerrainSpy {
    counts: (usize, usize),
    trace: Arc<Mutex<Vec<String>>>,
}
impl Terrain for TerrainSpy {
    fn cell_counts(&self) -> (usize, usize) {
        self.counts
    }
    fn initialized(&self) -> bool {
        true
    }
    fn altitude(&self, x: usize, z: usize) -> f32 {
        self.trace.lock().unwrap().push(format!("height {x},{z}"));
        0.0
    }
    fn normal(&self, x: f32, z: f32) -> Vector {
        self.trace.lock().unwrap().push(format!("normal {x},{z}"));
        [0.25, 0.5, 0.75]
    }
}
#[test]
fn vector12_upper_epsilon_is_stored_only_at_forwarding_boundary() {
    let trace = Arc::new(Mutex::new(Vec::new()));
    let terrain = Arc::new(TerrainSpy {
        counts: (64, 64),
        trace: trace.clone(),
    });
    let mut m = manager();
    m.do_message(
        LightingMessage::CityStarted {
            terrain,
            city_terrain: None,
            zoom: 0,
            rotation: 0,
        },
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(m.terrain_normal([1024.0, 0.0, -1.0]), Ok([0.25, 0.5, 0.75]));
    assert_eq!(*trace.lock().unwrap(), ["normal 1023.999,0"]);
}
#[test]
fn vector15_rectangular_sweep_counts_do_not_change_with_rotation() {
    let mut m = manager();
    m.cells = (2, 4);
    m.heights = vec![0.0; 15];
    m.shadows = vec![0.0; 15];
    m.sun = [1.0, -1.0, 0.0];
    for rotation in 0..4 {
        m.rotation = rotation;
        m.find_hill_shadows().unwrap();
        assert_eq!(
            m.changed,
            Rect {
                min_x: -1,
                min_z: -1,
                max_x: 0,
                max_z: 0
            }
        );
        m.heights[4] = 100.0;
        m.find_hill_shadows().unwrap();
        assert_eq!(
            m.changed,
            Rect {
                min_x: 4,
                min_z: 0,
                max_x: 4,
                max_z: 2
            }
        );
        let previous = m.changed;
        for sun in [[0.0, -1.0, 0.0], [0.0, -1.0, 1.0], [-1.0, -1.0, 0.0]] {
            m.sun = sun;
            m.find_hill_shadows().unwrap();
            assert_eq!(m.changed, previous);
        }
        m.sun = [1.0, -1.0, 0.0];
        m.heights.fill(0.0);
        m.shadows.fill(0.0);
    }
}
struct TraceEvents(Arc<Mutex<Vec<String>>>);
impl EventSink for TraceEvents {
    fn emit(&mut self, event: Event) {
        self.0.lock().unwrap().push(format!("{event:?}"));
    }
}
#[test]
fn vector16_copy_vertices_then_transformed_callbacks_and_no_change_silence() {
    let trace = Arc::new(Mutex::new(Vec::new()));
    let terrain = Arc::new(TerrainSpy {
        counts: (8, 4),
        trace: trace.clone(),
    });
    let mut m = manager();
    m.do_message(
        LightingMessage::CityStarted {
            terrain: terrain.clone(),
            city_terrain: Some(terrain),
            zoom: 0,
            rotation: 1,
        },
        &mut Vec::new(),
    )
    .unwrap();
    m.shadows_on = true;
    m.sun = [-1.0, -1.0, 0.0];
    m.changed = Rect {
        min_x: 1,
        min_z: 2,
        max_x: 3,
        max_z: 4,
    };
    let incoming = Rect {
        min_x: 2,
        min_z: 1,
        max_x: 3,
        max_z: 2,
    };
    let mut events = TraceEvents(trace.clone());
    m.do_message(LightingMessage::LongOperationBegin, &mut events).unwrap();
    m.do_message(LightingMessage::TerrainChanged(incoming), &mut events)
        .unwrap();
    assert!(trace.lock().unwrap().is_empty());
    m.do_message(LightingMessage::LongOperationEnd, &mut events).unwrap();
    assert!(!m.long_operation);
    assert_eq!(
        *trace.lock().unwrap(),
        [
            "height 2,1",
            "height 2,2",
            "height 3,1",
            "height 3,2",
            "TerrainLightingChanged(Rect { min_x: 5, min_z: 2, max_x: 7, max_z: 4 })",
            "RedisplayTerrain(Rect { min_x: 5, min_z: 2, max_x: 7, max_z: 4 })"
        ]
    );
    trace.lock().unwrap().clear();
    m.sun = [1.0, -1.0, 0.0];
    m.do_message(LightingMessage::TerrainChanged(incoming), &mut events)
        .unwrap();
    assert_eq!(
        *trace.lock().unwrap(),
        ["height 2,1", "height 2,2", "height 3,1", "height 3,2"]
    );
}
#[test]
fn independent_noncommuting_shears_preserve_composition_order() {
    let mut m = manager();
    m.shadow_direction = [0.0, -1.0, 0.0];
    // X-shear(2) and Z-shear(3), independently multiplied in both orders.
    let cases = [
        (
            [[7.0, 0.0, 2.0], [0.0, 1.0, 0.0], [3.0, 0.0, 1.0]],
            [
                [7.0, 0.0, 2.0, -8.0],
                [3.0, 0.0, 1.0, -32.0],
                [3.0, 0.0, 1.0, -32.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        ),
        (
            [[1.0, 0.0, 2.0], [0.0, 1.0, 0.0], [3.0, 0.0, 7.0]],
            [
                [1.0, 0.0, 2.0, -8.0],
                [3.0, 0.0, 7.0, -32.0],
                [3.0, 0.0, 7.0, -32.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        ),
    ];
    for (linear, expected) in cases {
        let mut out = [0.0; 16];
        assert_eq!(
            m.calc_shadow_projection(
                &POINTS,
                &UVS,
                ProjectionTransform {
                    linear,
                    ..transform(128)
                },
                &mut out
            ),
            Ok(true)
        );
        assert_eq!(rows(out), expected);
    }
}
#[test]
fn nonfinite_determinant_proceeds_zero_scale_and_literal_output_row() {
    let mut m = manager();
    m.shadow_direction = [0.0, -1.0, 0.0];
    let mut out = [777.0; 16];
    let mut points = POINTS;
    points[1][0] = f32::NAN;
    assert_eq!(
        m.calc_shadow_projection(&points, &UVS, transform(0), &mut out),
        Ok(true)
    );
    assert!(out[0].is_nan());
    for index in [3, 7, 11] {
        assert_eq!(out[index].to_bits(), 0);
    }
    assert_eq!(out[15], 1.0);
    assert_eq!(
        m.calc_shadow_projection(
            &POINTS,
            &UVS,
            ProjectionTransform {
                linear: I,
                scale: 0.0,
                ..transform(0)
            },
            &mut out
        ),
        Ok(true)
    );
    assert_eq!(rows(out)[0], [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(
        m.calc_shadow_projection(
            &POINTS,
            &UVS,
            ProjectionTransform {
                linear: I,
                scale: 0.0,
                ..transform(2)
            },
            &mut out
        ),
        Ok(true)
    );
    assert!(out.iter().any(|v| !v.is_finite()));
}
#[test]
fn terrain_probe_signs_integer_ties_and_zero_component_nan_stop() {
    let mut m = manager();
    city(&mut m, 8);
    for (direction, expected) in [
        ([1.0, -1.0, 1.0], [1, 1, 4, 4]),
        ([-1.0, -1.0, -1.0], [0, 0, 2, 2]),
        ([-1.0, -1.0, 1.0], [0, 1, 2, 4]),
        ([0.0, -1.0, 0.0], [1, 1, 2, 2]),
    ] {
        m.shadow_direction = direction;
        let mut out = Rect::EMPTY;
        m.calc_shadow_terrain_rect(
            Bounds {
                min: [16.0, 0.0, 16.0],
                max: [32.0, 32.0, 32.0],
            },
            &mut out,
        )
        .unwrap();
        assert_eq!([out.min_x, out.min_z, out.max_x, out.max_z], expected);
    }
    m.shadow_direction = [1.0, -1.0, -1.0];
    m.heights[2 * 9 + 2] = 32.0;
    let mut out = Rect::EMPTY;
    m.calc_shadow_terrain_rect(
        Bounds {
            min: [16.0, 0.0, 16.0],
            max: [32.0, 32.0, 32.0],
        },
        &mut out,
    )
    .unwrap();
    assert_eq!([out.min_x, out.min_z, out.max_x, out.max_z], [1, 0, 4, 2]);
}

const R: [[f32; 3]; 3] = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
const I: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
const POINTS: [Vector; 3] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
const UVS: [[f32; 2]; 3] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
fn transform(flags: u8) -> ProjectionTransform {
    ProjectionTransform {
        linear: R,
        translation: [8.0, 16.0, 32.0],
        scale: 2.0,
        flags,
    }
}
fn rows(flat: [f32; 16]) -> [[f32; 4]; 4] {
    std::array::from_fn(|i| std::array::from_fn(|j| flat[4 * j + i]))
}

#[test]
fn vector19_direction_transpose_negation_no_normalization_or_scale() {
    for precision in [RegisterPrecision::Bits53, RegisterPrecision::Bits64] {
        for flag in [0, 2, 4, 6] {
            let expected = if flag & 4 == 0 {
                [[1.0, 0.0, 0.0], [0.0, 1.0, -2.0], [0.0, 2.0, 1.0]]
            } else {
                [[-1.0, 0.0, 0.0], [0.0, 1.0, 2.0], [0.0, 2.0, -1.0]]
            };
            assert_eq!(
                projection::direction_basis([1.0, -2.0, 0.0], transform(flag), precision),
                expected
            );
        }
    }
}
#[test]
fn vector20_whole_byte_flags_inverse_translation_and_scale() {
    let cases = [
        (0, [[1.0, 0.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 1.0, 0.0]]),
        (
            1,
            [[0.0, 0.0, 1.0, -8.0], [-1.0, 0.0, 0.0, -32.0], [-1.0, 0.0, 0.0, -32.0]],
        ),
        (
            128,
            [[0.0, 0.0, 1.0, -8.0], [-1.0, 0.0, 0.0, -32.0], [-1.0, 0.0, 0.0, -32.0]],
        ),
        (
            2,
            [[0.0, 0.0, 0.5, -4.0], [-0.5, 0.0, 0.0, -16.0], [-0.5, 0.0, 0.0, -16.0]],
        ),
        (
            4,
            [[0.0, 0.0, -1.0, 32.0], [1.0, 0.0, 0.0, -8.0], [1.0, 0.0, 0.0, -8.0]],
        ),
        (
            6,
            [[0.0, 0.0, -0.5, 16.0], [0.5, 0.0, 0.0, -4.0], [0.5, 0.0, 0.0, -4.0]],
        ),
    ];
    let mut m = manager();
    m.shadow_direction = [0.0, -1.0, 0.0];
    for precision in [RegisterPrecision::Bits53, RegisterPrecision::Bits64] {
        m.register_precision = precision;
        for (flag, expected) in cases {
            let mut output = [999.0; 16];
            assert_eq!(
                m.calc_shadow_projection(&POINTS, &UVS, transform(flag), &mut output),
                Ok(true)
            );
            let actual = rows(output);
            assert_eq!(&actual[..3], &expected, "flag {flag}");
            assert_eq!(actual[3], [0.0, 0.0, 0.0, 1.0]);
        }
    }
}
#[test]
fn vector21_native_product_spill_rejects_noncollinear_triangle() {
    let mut m = manager();
    m.shadow_direction = [0.0, 0.0, -1.0];
    for precision in [RegisterPrecision::Bits53, RegisterPrecision::Bits64] {
        m.register_precision = precision;
        let mut output = [123.0; 16];
        assert_eq!(
            m.calc_shadow_projection(
                &[[4096.0, 0.0, 0.0], [4097.0, 0.0, 0.0], [4096.0, 1.0, 0.0]],
                &UVS,
                ProjectionTransform {
                    linear: I,
                    ..transform(0)
                },
                &mut output
            ),
            Ok(false)
        );
        assert_eq!(output, [123.0; 16]);
    }
}
#[test]
fn vector22_u_v_translation_spills_differ_with_identical_uv_columns() {
    let mut m = manager();
    m.shadow_direction = [0.0, -1.0, 0.0];
    for precision in [RegisterPrecision::Bits53, RegisterPrecision::Bits64] {
        m.register_precision = precision;
        let mut output = [0.0; 16];
        assert_eq!(
            m.calc_shadow_projection(
                &POINTS,
                &[[1.0, 1.0], [2.0, 2.0], [2.0, 2.0]],
                ProjectionTransform {
                    linear: I,
                    translation: [16777216.0, 0.0, 1.0],
                    ..transform(1)
                },
                &mut output
            ),
            Ok(true)
        );
        assert_eq!(
            rows(output),
            [
                [1.0, 0.0, 1.0, -16777215.0],
                [1.0, 0.0, 1.0, -16777216.0],
                [0.0, 0.0, 1.0, -1.0],
                [0.0, 0.0, 0.0, 1.0]
            ]
        );
    }
}
#[test]
fn vector17_terrain_probes_keep_stopped_vertex() {
    let mut m = manager();
    city(&mut m, 8);
    m.shadow_direction = [1.0, -1.0, -1.0];
    let mut out = Rect::EMPTY;
    assert_eq!(
        m.calc_shadow_terrain_rect(
            Bounds {
                min: [16.0, 0.0, 16.0],
                max: [32.0, 32.0, 32.0]
            },
            &mut out
        ),
        Ok(true)
    );
    assert_eq!(
        out,
        Rect {
            min_x: 1,
            min_z: 0,
            max_x: 4,
            max_z: 2
        }
    );
}
