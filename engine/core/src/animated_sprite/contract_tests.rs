// SPDX-License-Identifier: GPL-3.0-or-later
// Independent literal transcription of public Â§Â§4.4,5,6; native UNEXECUTED.
use super::*;

const Q_CAMERA: [[f32; 3]; 3] = [[0.5, -0.5, 0.5], [0.0, 0.5, 1.0], [-0.5, -0.5, 0.5]];
const Q1: [Vec3; 4] = [[6.0, 20.0, 8.0], [6.0, 0.0, 8.0], [9.0, 0.0, 5.0], [9.0, 20.0, 5.0]];
const Q1_WORLD: [Vec3; 4] = [
    [16.0, 40.0, 38.0],
    [16.0, 20.0, 38.0],
    [19.0, 20.0, 35.0],
    [19.0, 40.0, 35.0],
];
const Q2: [Vec3; 4] = [
    [1.25, 4.0, 1.75],
    [1.25, 0.0, 1.75],
    [2.25, 0.0, 0.75],
    [2.25, 4.0, 0.75],
];
const Q3: [Vec3; 4] = [
    [12.0, 40.0, 16.0],
    [12.0, 0.0, 16.0],
    [18.0, 0.0, 10.0],
    [18.0, 40.0, 10.0],
];
const Q5: [Vec3; 4] = [[-3.0, -1.0, 1.0], [-3.0, -3.0, 1.0], [1.0, 1.0, 1.0], [1.0, 3.0, 1.0]];

#[test]
fn setters_literal_completing_paths_and_unrelated_state() {
    // Inputs and outcomes are literals, independent of production thresholds.
    let cases = [
        (true, (-100.0_f32).to_bits(), true, 0.375_f32.to_bits()),
        (true, (-200.0_f32).to_bits(), false, 1.0_f32.to_bits()),
        (true, 0x3f7d70a3, true, 0x3f7d70a3),
        (true, 0x3f7d70a4, true, 0x3f7d70a4),
        (true, 0x3f7d70a5, false, 1.0_f32.to_bits()),
        (true, (-1.0_f32).to_bits(), true, (-1.0_f32).to_bits()),
        (true, 1.0_f32.to_bits(), false, 1.0_f32.to_bits()),
        (true, 0x7fc12345, true, 0x7fc12345),
        (false, 0x7fc12345, false, 1.0_f32.to_bits()),
        (false, (-100.0_f32).to_bits(), false, 1.0_f32.to_bits()),
        (false, (-200.0_f32).to_bits(), false, 1.0_f32.to_bits()),
    ];
    for (on, input, enabled, output) in cases {
        let mut s = cached_sprite();
        s.state.flags = 0xfa;
        s.state.alpha = 0.375;
        s.frame_dirty = false;
        s.cache.as_mut().unwrap().view_dirty = true;
        let cache_before = s.cache;
        let before = s.state;
        assert_eq!(s.set_fade(on, f32::from_bits(input)), 2);
        assert_eq!(s.state.alpha.to_bits(), output);
        assert_eq!(s.state.flags, 0xf8 | if enabled { 2 } else { 0 });
        s.state.alpha = before.alpha;
        s.state.flags = before.flags;
        assert_eq!(s.state, before);
        assert!(!s.frame_dirty);
        assert_eq!(s.cache, cache_before);
    }
    let mut s = tests::sprite();
    s.state.flags = 0x80;
    for (input, ret, byte, flag) in [
        (0, 0, 0, false),
        (255, 2, 255, true),
        (255, 0, 255, true),
        (256, 2, 0, true),
        (256, 2, 0, true),
        (0, 2, 0, false),
    ] {
        assert_eq!(s.set_highlight(input), ret);
        assert_eq!(s.state.highlight, byte);
        assert_eq!(s.state.flags, 0x80 | u8::from(flag));
    }
    for (input, ret, flag) in [
        (0, 0, false),
        (1, 2, true),
        (1, 0, true),
        (2, 2, true),
        (2, 2, true),
        (0x80, 2, true),
        (0, 2, false),
    ] {
        assert_eq!(s.set_self_lit_raw(input), ret);
        assert_eq!(s.state.flags, 0x80 | if flag { 4 } else { 0 });
    }
}

#[test]
fn raw_decoder_literal_packing_flips_scale_and_rejection() {
    // Public Â§4.1 vector; UV/page expectations are literal, not derived here.
    let (m, uv) = decode_entry_exact([2, 6, 48, 32, 16, 240, 253, 4], 5, 2.0).unwrap();
    assert_eq!(
        m,
        StoredMetric {
            page: 3,
            width: 32.0,
            height: 32.0,
            offset: [-6.0, 8.0]
        }
    );
    assert_eq!(uv, [[0.0, 0.25], [0.0, 0.0], [0.25, 0.0], [0.25, 0.25]]);
    assert_eq!(
        decode_entry_exact([0, 6, 0, 0, 0, 10, 0, 0], 0, 1.0),
        Err(Unsupported::Packing)
    );
    let mut c = camera(2.0);
    c.matrix[1][1] = 3.0; // PC/RC-sensitive stores remain explicitly open.
    assert_eq!(update_view_exact(metric(), c), Err(Unsupported::Arithmetic));
    c = camera(2.0);
    c.mpp = MetresPerPixel::ViewVolume {
        left: -8.0,
        right: 32.0,
        context_width: 0,
    };
    assert_eq!(update_view_exact(metric(), c), Err(Unsupported::Arithmetic));
    c = camera(f32::from_bits(1));
    assert_eq!(update_view_exact(metric(), c), Err(Unsupported::Arithmetic));
}

#[test]
fn stored_metric_suboperation_shrinks_without_scale_rotation_or_dirty_changes() {
    let mut s = cached_sprite();
    s.state.transform.rotation = [0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    s.state.transform.scale = 7.0;
    s.cache.as_mut().unwrap().view_dirty = true;
    s.update_stored_view_exact(camera(2.0)).unwrap();
    assert_eq!(s.cache().unwrap().local, Q1);
    assert!(s.cache().unwrap().view_dirty);
    s.cache.as_mut().unwrap().metric = StoredMetric {
        page: 3,
        width: 2.0,
        height: 2.0,
        offset: [0.5, 0.5],
    };
    s.update_stored_view_exact(camera(2.0)).unwrap();
    assert_eq!(s.cache().unwrap().local, Q2);
    assert_eq!(
        s.state.bounds,
        Bounds {
            min: [-2.25, 0.0, -1.75],
            max: [2.25, 4.0, 1.75]
        }
    );
    assert!(!s.frame_dirty);
    assert_eq!(s.cache().unwrap().counter, 7);
    assert_eq!(s.cache().unwrap().vertices.map(|v| v.position), Q1_WORLD);
}

fn camera(m: f32) -> ViewInputs {
    ViewInputs {
        matrix: Q_CAMERA,
        mpp: MetresPerPixel::Utilities(m),
        magnification: 0.5,
    }
}
fn metric() -> StoredMetric {
    StoredMetric {
        page: 3,
        width: 6.0,
        height: 10.0,
        offset: [2.0, 3.0],
    }
}

#[test]
fn independently_transcribed_q1_q2_q3_q4_q5() {
    let q = update_view_exact(metric(), camera(2.0)).unwrap();
    assert_eq!(q.corners, Q1);
    assert_eq!(
        q.bounds,
        Bounds {
            min: [-9.0, 0.0, -8.0],
            max: [9.0, 20.0, 8.0]
        }
    );
    let small = StoredMetric {
        page: 3,
        width: 2.0,
        height: 2.0,
        offset: [0.5, 0.5],
    };
    let q = update_view_exact(small, camera(2.0)).unwrap();
    assert_eq!(q.corners, Q2);
    assert_eq!(
        q.bounds,
        Bounds {
            min: [-2.25, 0.0, -1.75],
            max: [2.25, 4.0, 1.75]
        }
    );
    assert_eq!(update_view_exact(metric(), camera(4.0)).unwrap().corners, Q3);
    let mut fallback = camera(0.0);
    fallback.mpp = MetresPerPixel::ViewVolume {
        left: -8.0,
        right: 32.0,
        context_width: 20,
    };
    assert_eq!(update_view_exact(metric(), fallback).unwrap().corners, Q1);
    let q = update_view_exact(
        StoredMetric {
            page: 0,
            width: 4.0,
            height: 2.0,
            offset: [3.0, 1.0],
        },
        ViewInputs {
            matrix: [[1.0, 0.0, 0.0], [1.0, 1.0, 1.0], [0.0, 0.0, 1.0]],
            mpp: MetresPerPixel::Utilities(1.0),
            magnification: 1.0,
        },
    )
    .unwrap();
    assert_eq!(q.corners, Q5);
    assert_eq!(
        q.bounds,
        Bounds {
            min: [-3.0, -1.0, -1.0],
            max: [1.0, 3.0, 1.0]
        }
    );
}

#[derive(Default)]
struct Context {
    calls: Vec<&'static str>,
    vertices: Option<[DrawVertex; 4]>,
}
impl SpriteDrawContext for Context {
    fn set_model_transform(&mut self) -> bool {
        self.calls.push("transform");
        false
    }
    fn set_model_shade(&mut self, _: &InstanceState, rgb: u32) -> bool {
        assert_eq!(rgb, 0x123456);
        self.calls.push("shade");
        false
    }
    fn set_self_lit_shade(&mut self, rgb: u32) -> bool {
        assert_eq!(rgb, 0x123456);
        self.calls.push("selflit");
        false
    }
    fn set_render_state(&mut self, state: u32) -> bool {
        assert_eq!(state, 0x87654321);
        self.calls.push("state");
        false
    }
    fn set_transparency(&mut self, alpha: f32) -> bool {
        assert_eq!(alpha, 0.375);
        self.calls.push("transparency");
        false
    }
    fn set_alpha_func(&mut self, function: u32, reference: f32) -> bool {
        assert_eq!((function, reference), (4, 0.1875));
        self.calls.push("alpha");
        false
    }
    fn set_texture(&mut self, texture: TextureHandle, stage: u32) -> bool {
        assert_eq!((texture, stage), (TextureHandle(103), 0));
        self.calls.push("texture");
        false
    }
    fn draw_prims(&mut self, a: u32, b: u32, count: u32, stride: u32, vertices: &[DrawVertex; 4]) -> bool {
        assert_eq!((a, b, count, stride), (2, 10, 4, 24));
        self.vertices = Some(*vertices);
        self.calls.push("draw");
        false
    }
    fn reset_shade(&mut self) -> bool {
        self.calls.push("resetshade");
        false
    }
    fn reset_transparency(&mut self) -> bool {
        self.calls.push("resettransparency");
        false
    }
}
fn holder() -> RawSpriteHolder {
    RawSpriteHolder {
        divisors: [1, 1, 1],
        angle_limits: [f32::from_bits(0x40490fdb), f32::from_bits(0x40490fdb), 0.0, 0.0],
        entries: vec![[3, 6, 0, 0, 6, 10, 2, 3]],
        scale: 1.0,
    }
}
const TEXTURES: [TextureHandle; 4] = [
    TextureHandle(100),
    TextureHandle(101),
    TextureHandle(102),
    TextureHandle(103),
];
fn inputs() -> DrawInputs {
    DrawInputs {
        highlight_pass: false,
        zoom: 2,
        rotation: 0,
        view: camera(2.0),
        rgb: 0x123456,
        packed_state: 0x87654321,
    }
}
fn cached_sprite() -> AnimatedSprite<()> {
    let mut s = tests::sprite();
    s.frame_dirty = false;
    s.state.transform.translation = [10.0, 20.0, 30.0];
    s.state.transform.change = 7;
    s.install_cache(SpriteCache {
        metric: metric(),
        local: Q1,
        vertices: std::array::from_fn(|i| DrawVertex {
            position: Q1_WORLD[i],
            colour: 0xabcddcba,
            uv: [0.0, 0.0],
        }),
        view_dirty: false,
        counter: 7,
    });
    s
}

#[test]
fn clean_cache_protocol_and_ignored_false_returns() {
    let mut s = cached_sprite();
    s.state.flags = 7; // Stored highlight must not suppress the normal pass.
    s.state.alpha = 0.375;
    let h = holder();
    let resources = DrawResources {
        holders: [Some(&h); 5],
        textures: &TEXTURES,
    };
    let mut ctx = Context::default();
    assert_eq!(s.draw(inputs(), resources, &mut ctx), Ok(1));
    assert_eq!(
        ctx.calls,
        [
            "transform",
            "selflit",
            "state",
            "transparency",
            "alpha",
            "texture",
            "draw",
            "resetshade",
            "resettransparency"
        ]
    );
    assert_eq!(ctx.vertices.unwrap().map(|v| v.position), Q1_WORLD);
    assert_eq!(std::mem::size_of::<DrawVertex>(), 24);
    assert_eq!(std::mem::offset_of!(DrawVertex, uv), 16);
}

#[test]
fn cache_staleness_dirty_frame_only_and_counter_wrap() {
    let h = holder();
    let resources = DrawResources {
        holders: [Some(&h); 5],
        textures: &TEXTURES,
    };
    let mut s = cached_sprite();
    let mut input = inputs();
    input.view = camera(4.0);
    input.view.matrix = [[f32::NAN; 3]; 3]; // Unused on a clean cache.
    let mut ctx = Context::default();
    s.draw(input, resources, &mut ctx).unwrap();
    assert_eq!(s.cache().unwrap().vertices.map(|v| v.position), Q1_WORLD);
    s.state.transform.translation = [100.0; 3]; // Equal byte after wrap retains stale translation.
    s.draw(input, resources, &mut ctx).unwrap();
    assert_eq!(s.cache().unwrap().vertices.map(|v| v.position), Q1_WORLD);
    s.state.transform.translation = [10.0, 20.0, 30.0];
    s.frame_dirty = true;
    s.state.playback.frame = 1;
    s.draw(input, resources, &mut ctx).unwrap();
    assert!(!s.frame_dirty);
    assert_eq!(
        s.cache().unwrap().vertices.map(|v| v.uv),
        [[0.09375, 0.0], [0.09375, 0.15625], [0.1875, 0.15625], [0.1875, 0.0]]
    );
    assert_eq!(s.cache().unwrap().local, Q1);
    assert_eq!(s.cache().unwrap().vertices.map(|v| v.position), Q1_WORLD);
    s.cache.as_mut().unwrap().view_dirty = true;
    input.view = camera(4.0);
    s.draw(input, resources, &mut ctx).unwrap();
    assert_eq!(s.cache().unwrap().local, Q3);
    assert_eq!(
        s.cache().unwrap().vertices.map(|v| v.position),
        [
            [22.0, 60.0, 46.0],
            [22.0, 20.0, 46.0],
            [28.0, 20.0, 40.0],
            [28.0, 60.0, 40.0]
        ]
    );
    assert_eq!(
        s.state.bounds,
        Bounds {
            min: [-18.0, 0.0, -16.0],
            max: [18.0, 40.0, 16.0]
        }
    );
    assert!(!s.cache().unwrap().view_dirty);
    s.state.transform.change = 8;
    s.state.transform.translation = [0.0; 3];
    s.draw(input, resources, &mut ctx).unwrap();
    assert_eq!(s.cache().unwrap().vertices.map(|v| v.position), Q3);
    assert_eq!(s.cache().unwrap().counter, 8);
}

#[test]
fn full_context_comparison_early_exit_and_unsupported_refresh() {
    let h = holder();
    let resources = DrawResources {
        holders: [Some(&h); 5],
        textures: &TEXTURES,
    };
    let mut s = cached_sprite();
    let mut ctx = Context::default();
    let mut input = inputs();
    input.zoom = 258;
    s.draw(input, resources, &mut ctx).unwrap();
    assert_eq!(s.state.zoom, 2);
    input.view = camera(4.0);
    s.draw(input, resources, &mut ctx).unwrap(); // 258 != zero-extended 2, again.
    assert_eq!(s.cache().unwrap().local, Q3);
    input.zoom = 2;
    input.rotation = 256;
    input.view = camera(2.0);
    s.draw(input, resources, &mut ctx).unwrap();
    input.view = camera(4.0);
    s.draw(input, resources, &mut ctx).unwrap();
    assert_eq!(s.state.rotation, 0);
    assert_eq!(s.cache().unwrap().local, Q3);
    ctx.calls.clear();
    input.zoom = 5;
    assert_eq!(s.draw(input, resources, &mut ctx), Err(Unsupported::Holder));
    assert!(ctx.calls.is_empty());
    input.highlight_pass = true;
    assert_eq!(s.draw(input, resources, &mut ctx), Ok(1));
    input.highlight_pass = false;
    input.zoom = -1;
    assert_eq!(s.draw(input, resources, &mut ctx), Ok(1));
    assert!(ctx.calls.is_empty());
    input.zoom = 2;
    input.rotation = 1;
    s.state.transform.rotation[2] = 0.5;
    assert_eq!(s.draw(input, resources, &mut ctx), Err(Unsupported::Selection));
    assert!(ctx.calls.is_empty());
}

#[test]
fn pick_uses_stale_vertices_and_actual_second_result() {
    struct Picker {
        calls: Vec<[Vec3; 3]>,
        results: [bool; 2],
    }
    impl TrianglePicker for Picker {
        fn triangle_pick(&mut self, a: Vec3, b: Vec3, c: Vec3) -> bool {
            self.calls.push([a, b, c]);
            self.results[self.calls.len() - 1]
        }
    }
    let mut s = cached_sprite();
    s.state.transform.translation = [900.0; 3];
    s.state.zoom = 0;
    s.state.flags = 1;
    s.frame_dirty = true;
    for results in [[true, false], [false, true], [false, false]] {
        let mut p = Picker { calls: vec![], results };
        assert_eq!(s.pick_cached(&mut p), Ok(results[0] || results[1]));
        assert_eq!(p.calls[0], [[16.0, 40.0, 38.0], [16.0, 20.0, 38.0], [19.0, 20.0, 35.0]]);
        assert_eq!(p.calls.len(), if results[0] { 1 } else { 2 });
        if !results[0] {
            assert_eq!(p.calls[1], [[19.0, 20.0, 35.0], [19.0, 40.0, 35.0], [16.0, 40.0, 38.0]]);
        }
    }
}
