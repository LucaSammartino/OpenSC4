// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn state() -> EngineState {
    EngineState {
        clip: Rect(0, 0, 100, 80),
        ..EngineState::default()
    }
}
fn recorder(code: u32) -> Recorder {
    Recorder {
        log: log(),
        area: Rect(0, 0, 100, 80),
        code: Arc::new(AtomicU32::new(code)),
    }
}
fn spans(events: Vec<Event>) -> Vec<PixelSpan> {
    events
        .into_iter()
        .filter_map(|e| if let Event::Spans(s) = e { Some(s) } else { None })
        .flatten()
        .collect()
}

#[test]
fn gzg_v47_ellipse_convenience_origin_once() {
    let (mut c, _, f) = setup();
    c.offset(10, 20);
    take(&f.log);
    c.ellipse(1, 2, 5, 8).unwrap();
    c.filled_ellipse(1, 2, 5, 8).unwrap();
    let expected = Event::Generic(GeometryRoute::Outline, 5, vec![[11., 22.], [15., 28.]]);
    assert_eq!(take(&f.log), vec![expected.clone(), expected]);
}

#[test]
fn gzg_v48_curve_groups_and_sixteen_stroke_quads() {
    let mut e = recorder(0);
    let p = [
        [0., 0.],
        [0., 2.],
        [2., 2.],
        [2., 0.],
        [99., 99.],
        [99., 99.],
        [99., 99.],
    ];
    super::geometry::execute(&mut e, GeometryRoute::Tessellate, 2, &p, &state()).unwrap();
    let events = take(&e.log);
    assert!(matches!(&events[..], [Event::Draw(7,p)] if p.len() == 64));
}

#[test]
fn gzg_v49_v50_v51_filled_ellipse_raster_and_repeated_first_pair() {
    let mut e = recorder(0);
    for p in [vec![[0., 0.], [4., 4.]], vec![[4., 4.], [0., 0.]]] {
        super::geometry::execute(&mut e, GeometryRoute::Rasterize, 5, &p, &state()).unwrap();
        assert_eq!(
            spans(take(&e.log)),
            vec![
                PixelSpan {
                    y: 0,
                    left: 1,
                    right: 3
                },
                PixelSpan {
                    y: 1,
                    left: 0,
                    right: 4
                },
                PixelSpan {
                    y: 2,
                    left: 0,
                    right: 4
                },
                PixelSpan {
                    y: 3,
                    left: 1,
                    right: 3
                }
            ]
        );
    }
    super::geometry::execute(
        &mut e,
        GeometryRoute::Tessellate,
        5,
        &[[0., 0.], [2., 2.], [20., 20.], [40., 40.]],
        &state(),
    )
    .unwrap();
    let events = take(&e.log);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], events[1]);
    assert!(matches!(&events[0],Event::Draw(8,p) if p.len() == 4));
}

#[test]
fn gzg_v52_v53_v54_convenience_blits_and_empty_noop() {
    let (mut c, t, f) = setup();
    take(&f.log);
    c.convenience_blit(t.clone(), Rect(2, 3, 6, 5), Rect(10, 20, 18, 26))
        .unwrap();
    let events = take(&f.log);
    assert_eq!(events[0], Event::Bind([8., 17.], [2., 3.], true));
    assert!(events.contains(&Event::State(StateChange::Tiling(None))));
    assert!(events.contains(&Event::Draw(3, vec![[10., 20.], [18., 26.]])));
    assert_eq!(events[events.len() - 2], Event::Bind([0., 0.], [1., 1.], false));
    c.blit_tiled(t.clone(), Rect(2, 3, 6, 5), Rect(10, 20, 18, 26), [-2, 4])
        .unwrap();
    let events = take(&f.log);
    assert_eq!(events[0], Event::Bind([8., 24.], [1., 1.], true));
    assert!(events.contains(&Event::State(StateChange::Tiling(Some([2, 3, 6, 5])))));
    assert!(!c.blit_active() && !c.tiling());
    c.convenience_blit(t.clone(), Rect(0, 0, 0, 5), Rect(1, 1, 9, 9))
        .unwrap();
    c.blit_tiled(t, Rect(0, 0, 5, 5), Rect(1, 1, 1, 9), [0, 0]).unwrap();
    assert!(take(&f.log).is_empty());
}

#[test]
fn gzg_v55_gradient_conversion_cache_and_kernel_writes() {
    for ty in [2, 4, 5, 6, 7, 8, 9] {
        let b = buffer(&log(), ty, true);
        let t = target(Rect::default(), Some(b.clone()));
        let mut e = BufferEngine::default();
        assert!(e.init(&t));
        e.state(StateChange::Gradient(Some(vec![0; 12]), 12.5, -3.25)).unwrap();
        assert_eq!(
            e.gradient,
            GradientCoefficients {
                constant: [0x8000; 4],
                x: [0; 4],
                y: [0; 4]
            }
        );
        e.draw_preclipped_spans(&[(1, 3)]).unwrap();
        e.state(StateChange::Alpha(1)).unwrap();
        e.draw_preclipped_spans(&[(1, 3)]).unwrap();
        e.state(StateChange::Gradient(None, 0., 0.)).unwrap();
        assert_eq!(e.gradient.constant, [0x8000; 4]);
        assert_eq!(e.span_route(), Ok(SpanRoute::Alpha));
        e.shutdown();
    }
    let mut words = vec![0; 12];
    words[0] = 0.5_f32.to_bits();
    words[4] = 0.25_f32.to_bits();
    words[8] = (-0.125_f32).to_bits();
    let g = GradientCoefficients::convert(&words, [1.5, 2.25]).unwrap();
    assert_eq!(g.x[0], 4_177_920);
    assert_eq!(g.y[0], -2_088_960);
    assert_eq!(g.constant[0], 6_821_888);
}

#[test]
fn gzg_v56_v57_ordinary_clip_and_wrapper_local_copies() {
    let (mut s, mut d) = (Rect(0, 0, 8, 8), Rect(-2, -1, 6, 7));
    assert!(generic_buffer_clip(
        Rect(5, 5, 10, 10),
        &mut s,
        &mut d,
        Some(Rect(1, 2, 5, 6))
    ));
    assert_eq!((s, d), (Rect(3, 3, 7, 7), Rect(1, 2, 5, 6)));
    let (mut s, mut d) = (Rect(0, 0, 4, 4), Rect(12, 3, 16, 7));
    assert!(!generic_buffer_clip(Rect(0, 0, 10, 10), &mut s, &mut d, None));
    assert_eq!((s, d), (Rect(0, 0, -2, 4), Rect(12, 3, 10, 7)));
    let original = Rect(0, 0, 8, 8);
    assert_eq!(
        clipped_alpha_rectangles(Rect(0, 0, 10, 10), original, [-2, -1], Some(Rect(1, 2, 5, 6))),
        Some((Rect(3, 3, 7, 7), Rect(1, 2, 5, 6)))
    );
    assert_eq!(original, Rect(0, 0, 8, 8));
}

#[test]
fn gzg_v79_v80_horizontal_directional_lines() {
    let mut e = recorder(0);
    for (p, xs) in [
        ([[2.5, 3.5], [4.5, 3.5]], vec![2, 3]),
        ([[4.5, 3.5], [2.5, 3.5]], vec![3, 4]),
        ([[2.5, 3.5], [2.5, 3.5]], vec![]),
    ] {
        super::geometry::execute(&mut e, GeometryRoute::Rasterize, 1, &p, &state()).unwrap();
        let got = spans(take(&e.log));
        assert_eq!(
            got,
            xs.into_iter()
                .map(|x| PixelSpan {
                    y: 3,
                    left: x,
                    right: x + 1
                })
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn gzg1_integer_line_phase_worked_cases() {
    let mut e = recorder(0);
    for (p, clip, expected) in [
        (
            [[2.5, 3.5], [6.5, 5.5]],
            Rect(0, 0, 10, 10),
            vec![(2, 3), (3, 4), (4, 4), (5, 5)],
        ),
        (
            [[2.5, 3.5], [6.5, 5.5]],
            Rect(3, 0, 10, 10),
            vec![(3, 4), (4, 4), (5, 5)],
        ),
        (
            [[2.5, 3.5], [6.5, 1.5]],
            Rect(0, 0, 10, 10),
            vec![(2, 3), (3, 3), (4, 2), (5, 2)],
        ),
        (
            [[2.5, 2.5], [5.5, 5.5]],
            Rect(0, 0, 10, 10),
            vec![(2, 2), (3, 3), (4, 4)],
        ),
    ] {
        let s = EngineState { clip, ..state() };
        super::geometry::execute(&mut e, GeometryRoute::Rasterize, 1, &p, &s).unwrap();
        assert_eq!(
            spans(take(&e.log)).iter().map(|s| (s.left, s.y)).collect::<Vec<_>>(),
            expected
        );
    }
    for (p, clip) in [
        ([[0.5, 0.5], [4.5, 2.5]], Rect(2, 0, 3, 8)),
        ([[0.5 + 1. / 256., 0.5], [4.5, 2.5]], Rect(0, 0, 8, 8)),
    ] {
        assert_eq!(
            super::geometry::execute(
                &mut e,
                GeometryRoute::Rasterize,
                1,
                &p,
                &EngineState { clip, ..state() }
            ),
            Err(Error::Unsupported("GZG1"))
        );
    }
}

#[test]
fn gzg7_stroke_corners_and_constant_cubic_worked_cases() {
    use super::geometry::{execute, stroke_quad};
    assert_eq!(
        stroke_quad([1., 2.], [2., 2.], 2., 2, true),
        [[1., 1.], [1., 3.], [2., 3.], [2., 1.]]
    );
    assert_eq!(
        stroke_quad([1., 2.], [2., 2.], 2., 2, false),
        [[0.5, 1.], [0.5, 3.], [2.5, 3.], [2.5, 1.]]
    );
    let mut e = recorder(0);
    for (aa, quad) in [(1, [[0.; 2]; 4]), (0, [[3.5, 5.], [3.5, 3.], [2.5, 3.], [2.5, 5.]])] {
        let s = EngineState {
            aa,
            thickness: 2.,
            ..state()
        };
        execute(&mut e, GeometryRoute::Tessellate, 2, &[[3., 4.]; 4], &s).unwrap();
        assert_eq!(take(&e.log), vec![Event::Draw(7, quad.repeat(16))]);
    }
    // A straight cubic has exact Horner endpoints at i/16, not rounded Bernstein products.
    execute(
        &mut e,
        GeometryRoute::Tessellate,
        2,
        &[[1., 2.], [1. + 1. / 3., 2.], [1. + 2. / 3., 2.], [2., 2.]],
        &EngineState {
            aa: 1,
            thickness: 2.,
            ..state()
        },
    )
    .unwrap();
    let events = take(&e.log);
    let [Event::Draw(7, p)] = events.as_slice() else {
        panic!()
    };
    assert_eq!(&p[..2], &[[1., 1.], [1., 3.]]);
    assert_eq!(&p[62..], &[[2., 3.], [2., 1.]]);
}

#[test]
fn gzg1_non_aa_outline_ellipse_topology_worked_case() {
    let mut e = recorder(0);
    let p = [[-1., -1.], [1., 1.], [-1., -1.], [1., 1.], [99., 99.]];
    super::geometry::execute(
        &mut e,
        GeometryRoute::Tessellate,
        4,
        &p,
        &EngineState {
            aa: 0,
            thickness: 9.,
            ..state()
        },
    )
    .unwrap();
    let events = take(&e.log);
    let [Event::Draw(7, points)] = events.as_slice() else {
        panic!("one kind-7 submission")
    };
    assert_eq!(points.len(), 96);
    assert_eq!(&points[..2], &[[2., 0.], [1., 0.]]);
    assert_eq!(&points[..48], &points[48..]);
    // Sample m=1 adds the offset-transition quad after the first ordinary quad.
    assert_eq!(&points[4..6], &[points[3], points[2]]);
    assert_eq!(points[5], points[6]);
}

#[test]
fn gzg_v81_triangle_fan_no_second_fallback() {
    let p = [[0., 0.], [1., 2.], [2., 3.], [3., 2.], [4., 0.]];
    for code in [0, 1, 2, 99] {
        let mut e = recorder(code);
        super::geometry::execute(&mut e, GeometryRoute::Tessellate, 8, &p, &state()).unwrap();
        assert_eq!(
            take(&e.log),
            vec![Event::Draw(
                6,
                vec![p[0], p[1], p[2], p[0], p[2], p[3], p[0], p[3], p[4]]
            )]
        );
    }
}

#[test]
fn gzg_v82_v83_v84_ordered_polygon_chains() {
    let p = [[0., 0.], [0., 4.], [4., 4.], [4., 0.]];
    let expected: Vec<_> = (0..4).map(|y| PixelSpan { y, left: 0, right: 4 }).collect();
    assert_eq!(polygon_spans(&p, state().clip), expected);
    let mut reversed = p;
    reversed.reverse();
    assert!(polygon_spans(&reversed, state().clip).is_empty());
    let mut repeated = p.to_vec();
    repeated.push(p[0]);
    assert_eq!(polygon_spans(&repeated, state().clip), expected);
    assert!(polygon_spans(&p[..2], state().clip).is_empty());
    assert_eq!(
        polygon_spans(&p, Rect(1, 1, 3, 3)),
        vec![
            PixelSpan {
                y: 1,
                left: 1,
                right: 3
            },
            PixelSpan {
                y: 2,
                left: 1,
                right: 3
            }
        ]
    );
}

#[test]
fn gzg_v85_outline_ellipse_cubics_use_each_pair() {
    let mut e = recorder(0);
    super::geometry::execute(
        &mut e,
        GeometryRoute::Rasterize,
        4,
        &[[0., 0.], [4., 4.], [10., 10.], [12., 12.]],
        &state(),
    )
    .unwrap();
    let events = take(&e.log);
    let p = match &events[..] {
        [Event::Draw(2, p)] => p,
        _ => panic!("expected one cubic submission"),
    };
    assert_eq!(p.len(), 32);
    assert_eq!(p[0], [4., 2.]);
    assert_eq!(p[16], [12., 11.]);
    assert_eq!(p[3], p[4]);
    assert_eq!(p[15], p[0]);
}

#[test]
fn gzg_v86_cubic_forward_difference_overshoot() {
    let p = [[0.5, 0.5], [1.5, 0.5], [2.5, 0.5], [3.5, 0.5]];
    assert_eq!(
        cubic_segments(&p),
        vec![([128, 128], [896, 128]), ([896, 128], [1664, 128])]
    );
    let mut e = recorder(0);
    super::geometry::execute(&mut e, GeometryRoute::Rasterize, 2, &p, &state()).unwrap();
    assert_eq!(spans(take(&e.log)).len(), 6);
}

#[test]
fn gzg_v87_v88_v89_v90_stretch_clip() {
    for (src, dest, extra, ok, want_s, want_d) in [
        (
            Rect(2, 4, 6, 8),
            Rect(-2, -2, 6, 6),
            None,
            true,
            Rect(3, 5, 6, 8),
            Rect(0, 0, 6, 6),
        ),
        (
            Rect(0, 0, 2, 2),
            Rect(9, 0, 13, 4),
            None,
            false,
            Rect(0, 0, 2, 2),
            Rect(9, 0, 10, 4),
        ),
        (
            Rect(0, 0, 1, 1),
            Rect(0, 0, 2, 2),
            Some(Rect(0, 0, 1, 2)),
            false,
            Rect(0, 0, 1, 1),
            Rect(0, 0, 1, 2),
        ),
        (
            Rect(0, 0, 1, 1),
            Rect(0, 0, 2, 2),
            None,
            true,
            Rect(0, 0, 1, 1),
            Rect(0, 0, 2, 2),
        ),
        (
            Rect(5, 7, 6, 8),
            Rect(9, 9, 13, 13),
            None,
            true,
            Rect(4, 6, 5, 7),
            Rect(9, 9, 10, 10),
        ),
    ] {
        let (mut s, mut d) = (src, dest);
        assert_eq!(generic_buffer_clip_stretch(10, 10, &mut s, &mut d, extra), ok);
        assert_eq!((s, d), (want_s, want_d));
    }
}

struct LoaderIo {
    buffer: Arc<TestBuffer>,
    file: Vec<u8>,
    opened: bool,
    calls: Vec<&'static str>,
    frees: Vec<usize>,
}
impl AlphaFileIo for LoaderIo {
    fn create_buffer(&mut self) -> Result<Arc<dyn Buffer>> {
        self.calls.push("create");
        Ok(self.buffer.clone())
    }
    fn open(&mut self, _: &str) -> bool {
        self.calls.push("open");
        self.opened
    }
    fn close(&mut self) {
        self.calls.push("close");
    }
    fn read_at(&mut self, offset: usize, out: &mut [u8]) -> usize {
        self.calls.push(if offset == 14 { "header" } else { "pixels" });
        assert!(offset == 14 || offset == 14 + 0x428);
        let n = out.len().min(self.file.len().saturating_sub(offset));
        out[..n].copy_from_slice(&self.file[offset..offset + n]);
        n
    }
    fn initialize(&mut self, _: &Arc<dyn Buffer>, w: u32, h: u32, kind: u32, depth: u32) -> bool {
        self.calls.push("init");
        assert_eq!((w, h, kind, depth), (6, 2, 2, 8));
        true
    }
    fn free_source(&mut self, offset: usize) {
        self.frees.push(offset);
    }
}
fn loader(lock_ok: bool) -> LoaderIo {
    let mut file = vec![0; 14 + 0x428 + 16];
    // No signature validation, palette lookup or BMP offset-driven decoding.
    file[18..22].copy_from_slice(&6_u32.to_le_bytes());
    file[22..26].copy_from_slice(&2_u32.to_le_bytes());
    file[14 + 0x428..].copy_from_slice(&[0, 7, 8, 247, 248, 255, 99, 99, 255, 248, 247, 8, 7, 0, 99, 99]);
    LoaderIo {
        buffer: buffer(&log(), 2, lock_ok),
        file,
        opened: true,
        calls: Vec::new(),
        frees: Vec::new(),
    }
}

#[test]
fn gzg_v58_v59_indexed_loader_levels_raw_mask_and_row_stride() {
    let mut alpha = AlphaBlt {
        delegate: Delegate { calls: Vec::new() },
    };
    for mask in [false, true] {
        let mut io = loader(true);
        let result = alpha.surface_from_file("indexed", mask, &mut io).unwrap().unwrap();
        assert!(Arc::ptr_eq(&result, &(io.buffer.clone() as Arc<dyn Buffer>)));
        let pixels = io.buffer.pixels.lock().unwrap();
        let (top, bottom) = if mask {
            (vec![255, 248, 247, 8, 7, 0], vec![0, 7, 8, 247, 248, 255])
        } else {
            (vec![31, 31, 30, 1, 0, 0], vec![0, 0, 1, 30, 31, 31])
        };
        assert_eq!(&pixels[..6], top.as_slice());
        assert_eq!(&pixels[32..38], bottom.as_slice());
        assert!(pixels[6..32].iter().all(|&v| v == 0));
        assert_eq!(io.frees, vec![0]);
        assert_eq!(io.calls, vec!["create", "open", "header", "pixels", "init", "close"]);
        assert_eq!(take(&io.buffer.log), vec![Event::Lock(0x8040), Event::Unlock(0x8040)]);
    }
    let mut io = loader(true);
    alpha.alpha_surface_from_file("indexed", 3, &mut io).unwrap();
    assert_eq!(io.buffer.pixels.lock().unwrap()[0], 3);
}

#[test]
fn gzg_v35_v60_loader_open_and_lock_failure_ownership() {
    let mut alpha = AlphaBlt {
        delegate: Delegate { calls: Vec::new() },
    };
    let mut io = loader(true);
    io.opened = false;
    assert!(alpha.mask_surface_from_file("absent", &mut io).unwrap().is_none());
    assert_eq!(io.calls, vec!["create", "open"]);
    assert_eq!(Arc::strong_count(&io.buffer), 2);
    let mut io = loader(false);
    assert!(matches!(
        alpha.mask_surface_from_file("indexed", &mut io),
        Err(Error::Foreign)
    ));
    assert_eq!(io.frees, vec![16]);
    assert_eq!(take(&io.buffer.log), vec![Event::Lock(0x8040)]);
    assert_eq!(Arc::strong_count(&io.buffer), 2);
    assert_eq!(io.calls.last(), Some(&"close"));
}

#[test]
fn settled_polygon_spans_reach_real_buffer_kernels() {
    let b = buffer(&log(), 2, true);
    let mut e = BufferEngine::default();
    assert!(e.init(&target(Rect::default(), Some(b.clone()))));
    e.state(StateChange::Clip(Rect(0, 0, 8, 2))).unwrap();
    e.set_colours(0, &[0x00303030]).unwrap();
    e.generic(GeometryRoute::Rasterize, 8, &[[0., 0.], [0., 2.], [4., 2.], [4., 0.]])
        .unwrap();
    let pixels = b.pixels.lock().unwrap();
    assert_eq!(&pixels[..5], &[0x30, 0x30, 0x30, 0x30, 0]);
    assert_eq!(&pixels[32..37], &[0x30, 0x30, 0x30, 0x30, 0]);
}

#[test]
fn gzg_v61_driver_getters_are_borrowed_exact_and_null_on_missing() {
    let mut g = graphics(&log());
    g.set_default_driver_class_id(0x12345678);
    g.set_driver_settings_ini_file_path("driver.ini");
    assert_eq!(g.default_driver_class, 0x12345678);
    assert_eq!(g.driver_ini, "driver.ini");
    assert!(g.driver_by_index(0).is_none());
    assert!(g.driver_by_id(7).is_none());
    let d: Arc<dyn Driver> = Arc::new(TestDriver {
        id: 7,
        modes: Vec::new(),
        enumerations: AtomicU32::new(0),
    });
    g.drivers.push(d.clone());
    g.set_driver(Some(&d));
    let count = Arc::strong_count(&d);
    assert!(Arc::ptr_eq(g.driver_by_index(0).unwrap(), &d));
    assert!(Arc::ptr_eq(g.driver_by_id(7).unwrap(), &d));
    assert!(g.driver_by_index(g.driver_count()).is_none());
    assert!(g.driver_by_index(usize::MAX).is_none());
    assert!(g.driver_by_id(8).is_none());
    assert_eq!(Arc::strong_count(&d), count);
}

#[test]
fn gradient_binary32_products_obey_integer_rounding_mode() {
    let mut words = vec![0; 12];
    words[4] = (0.5_f32 / 16_711_680.).to_bits();
    words[8] = (-0.5_f32 / 16_711_680.).to_bits();
    for (mode, x, y) in [
        (IntegerRounding::NearestEven, 0, 0),
        (IntegerRounding::Down, 0, -1),
        (IntegerRounding::Up, 1, 0),
        (IntegerRounding::TowardZero, 0, 0),
    ] {
        let g = GradientCoefficients::convert_with_rounding(&words, [0., 0.], mode).unwrap();
        assert_eq!((g.x[0], g.y[0]), (x, y));
    }
}
