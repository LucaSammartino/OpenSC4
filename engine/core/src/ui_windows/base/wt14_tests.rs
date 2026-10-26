// SPDX-License-Identifier: GPL-3.0-or-later
//! widget-toolkit.md §14.5.3 WT14-A..N: service results and outputs are independent.
use super::*;

#[derive(Debug, PartialEq, Eq)]
enum Call {
    Event(NativeOp),
    AcquireDraw,
    Graphics,
    AcquireGraphics,
    DrawCreate(Object, i32, i32, u32),
    RawCreate(Object),
    Init(Object, i32, i32, u32, u32),
    Query(Object),
    Context(Object),
    Screen,
    Area(Window),
    Transparency,
}
#[derive(Default)]
struct GraphicsHost {
    calls: Vec<Call>,
    draw: Option<Object>,
    graphics: Option<Object>,
    draw_output: (bool, Option<Object>),
    raw_output: (bool, Option<Object>),
    init_ok: bool,
    buffers: BTreeMap<Object, BufferInfo>,
    query_output: (bool, Option<Object>),
    context: Option<Object>,
    screen: Option<Object>,
}
impl Host for GraphicsHost {
    fn event(&mut self, event: NativeOp) {
        self.calls.push(Call::Event(event));
    }
    fn acquire_draw_system(&mut self) -> Option<Object> {
        self.calls.push(Call::AcquireDraw);
        self.draw
    }
    fn graphics_system(&mut self) -> Option<Object> {
        self.calls.push(Call::Graphics);
        self.graphics
    }
    fn acquire_graphics_system(&mut self) -> Option<Object> {
        self.calls.push(Call::AcquireGraphics);
        self.graphics
    }
    fn draw_system_create_buffer(
        &mut self,
        service: Object,
        out: &mut Option<Object>,
        width: i32,
        height: i32,
        mode: u32,
    ) -> bool {
        assert_eq!(*out, None);
        self.calls.push(Call::DrawCreate(service, width, height, mode));
        *out = self.draw_output.1;
        if let Some(b) = *out {
            self.buffers.entry(b).or_default().area = Rect(0, 0, width, height);
        }
        self.draw_output.0
    }
    fn graphics_system_create_buffer(&mut self, graphics: Object, out: &mut Option<Object>) -> bool {
        assert_eq!(*out, None);
        self.calls.push(Call::RawCreate(graphics));
        *out = self.raw_output.1;
        self.raw_output.0
    }
    fn init_buffer(&mut self, b: Object, width: i32, height: i32, kind: u32, bpp: u32) -> bool {
        self.calls.push(Call::Init(b, width, height, kind, bpp));
        self.init_ok
    }
    fn buffer_info(&mut self, b: Object) -> BufferInfo {
        self.buffers.get(&b).copied().unwrap_or_default()
    }
    fn query_draw_interface(&mut self, b: Object, out: &mut Option<Object>) -> bool {
        assert_eq!(*out, None);
        self.calls.push(Call::Query(b));
        *out = self.query_output.1;
        self.query_output.0
    }
    fn drawing_context(&mut self, interface: Object) -> Option<Object> {
        self.calls.push(Call::Context(interface));
        self.context
    }
    fn screen_buffer(&mut self) -> Option<Object> {
        self.calls.push(Call::Screen);
        self.screen
    }
    fn set_area(&mut self, wins: &mut Windows, w: Window, area: Rect) {
        self.calls.push(Call::Area(w));
        wins.set_area(self, w, area);
    }
    fn transparent_color(&mut self, _: u32) -> Option<u32> {
        self.calls.push(Call::Transparency);
        None
    }
}

fn window() -> (Windows, GraphicsHost, Window) {
    let mut wins = Windows::default();
    let mut host = GraphicsHost::default();
    let w = wins.create(&mut host, 1, None);
    let s = wins.get_mut(w);
    s.area = Rect(0, 0, 30, 40);
    s.private_buffer = Some(1);
    s.draw_buffer = Some(91);
    s.draw_context = Some(92);
    s.dirty = false;
    host.calls.clear();
    (wins, host, w)
}
fn event(op: NativeOp) -> Call {
    Call::Event(op)
}

#[test]
fn wt14_a_reject_only_when_both_dimensions_nonpositive() {
    let (mut wins, mut host, w) = window();
    host.draw = Some(20);
    assert!(!wins.create_private_buffer(&mut host, w, 0, -1));
    assert_eq!(wins.get(w).private_buffer, None);
    assert_eq!(host.calls, [event(NativeOp::Release(1))]);
    // Slot 150 leaves all borrowed caches and higher-level flags/areas unchanged.
    assert_eq!(
        (wins.get(w).draw_buffer, wins.get(w).draw_context),
        (Some(91), Some(92))
    );
    assert_eq!(wins.get(w).flags, 0x8903);
    assert!(!wins.get(w).dirty);
}

#[test]
fn wt14_b_c_d_draw_service_destructive_outputs_and_reference_order() {
    for (ok, output) in [(true, Some(2)), (false, None), (false, Some(2))] {
        for dimensions in [(0, 20), (30, -1)] {
            for old in [Some(1), None] {
                let (mut wins, mut host, w) = window();
                wins.get_mut(w).private_buffer = old;
                host.draw = Some(20);
                host.draw_output = (ok, output);
                assert_eq!(wins.create_private_buffer(&mut host, w, dimensions.0, dimensions.1), ok);
                assert_eq!(wins.get(w).private_buffer, output);
                let mut expected = vec![Call::AcquireDraw];
                if old.is_some() {
                    expected.extend([event(NativeOp::BufferReleaseSurface(1)), event(NativeOp::Release(1))]);
                }
                expected.extend([
                    Call::DrawCreate(20, dimensions.0, dimensions.1, 1),
                    event(NativeOp::Release(20)),
                ]);
                assert_eq!(host.calls, expected);
            }
        }
    }
}

#[test]
fn wt14_e_f_fallback_reuses_old_buffer_without_double_shutdown() {
    for init_ok in [false, true] {
        let (mut wins, mut host, w) = window();
        host.init_ok = init_ok;
        assert_eq!(wins.create_private_buffer(&mut host, w, 30, 40), init_ok);
        assert_eq!(wins.get(w).private_buffer, init_ok.then_some(1));
        let mut expected = vec![
            Call::AcquireDraw,
            event(NativeOp::BufferReleaseSurface(1)),
            Call::Init(1, 30, 40, 1, 0),
        ];
        if !init_ok {
            expected.push(event(NativeOp::Release(1)));
        }
        assert_eq!(host.calls, expected);
    }
}

#[test]
fn wt14_g_h_fallback_examines_output_ignoring_factory_byte() {
    for graphics in [None, Some(30)] {
        for output in [None, Some(2)] {
            for factory_ok in [false, true] {
                for init_ok in [false, true] {
                    let (mut wins, mut host, w) = window();
                    wins.get_mut(w).private_buffer = None;
                    host.graphics = graphics;
                    host.raw_output = (factory_ok, output);
                    host.init_ok = init_ok;
                    let candidate = graphics.and(output);
                    let success = candidate.is_some() && init_ok;
                    assert_eq!(wins.create_private_buffer(&mut host, w, 30, 40), success);
                    assert_eq!(wins.get(w).private_buffer, candidate.filter(|_| init_ok));
                    let mut expected = vec![Call::AcquireDraw, Call::Graphics];
                    if graphics.is_some() {
                        expected.push(Call::RawCreate(30));
                    }
                    if let Some(b) = candidate {
                        expected.push(Call::Init(b, 30, 40, 1, 0));
                        if !init_ok {
                            expected.push(event(NativeOp::Release(b)));
                        }
                    }
                    assert_eq!(host.calls, expected);
                }
            }
        }
    }
}

#[test]
fn wt14_i_same_size_skips_allocator_transparency_and_area() {
    let (mut wins, mut host, w) = window();
    host.buffers.insert(
        1,
        BufferInfo {
            area: Rect(7, 9, 37, 49),
            ..BufferInfo::default()
        },
    );
    wins.get_mut(w).flags |= 0x20000;
    assert!(wins.private_buffer(&mut host, w, true));
    assert!(wins.get(w).flag(0x10000));
    assert!(wins.get(w).dirty);
    assert!(wins.get(w).draw_origin.is_some());
    assert_eq!(host.calls, [Call::Query(1)]);
}

#[test]
fn wt14_j_failed_allocation_preserves_bit_and_runs_recursive_common_tail() {
    let (mut wins, mut host, w) = window();
    let child = wins.create(&mut host, 1, None);
    wins.get_mut(w).children.push(child);
    let c = wins.get_mut(child);
    c.parent = Some(w);
    c.private_buffer = Some(3);
    c.flags |= 0x10000;
    c.area = Rect(0, 0, 10, 10);
    c.dirty = false;
    host.buffers.insert(
        3,
        BufferInfo {
            area: c.area,
            ..BufferInfo::default()
        },
    );
    wins.get_mut(w).flags |= 0x20000;
    host.draw = Some(20);
    host.calls.clear();
    assert!(!wins.private_buffer(&mut host, w, true));
    assert!(wins.get(w).flag(0x10000));
    assert_eq!(wins.get(w).private_buffer, None);
    assert!(wins.get(w).dirty && wins.get(child).dirty);
    assert_eq!(wins.get(child).private_buffer, Some(3));
    assert!(wins.get(w).draw_origin.is_some() && wins.get(child).draw_origin.is_some());
    assert_eq!(host.calls.iter().filter(|c| **c == Call::Query(3)).count(), 2);
    assert!(!host.calls.contains(&Call::Area(w)));
    assert!(!host.calls.contains(&Call::Transparency));
}

#[test]
fn wt14_k_l_readiness_gate_and_painted_flag_without_fade() {
    for initialized in [false, true] {
        let (mut wins, mut host, w) = window();
        host.buffers.insert(
            1,
            BufferInfo {
                initialized,
                ..BufferInfo::default()
            },
        );
        wins.get_mut(w).flags |= 0x410000;
        assert!(wins.private_buffer(&mut host, w, false));
        assert_eq!(wins.get(w).private_buffer, None);
        assert_eq!(wins.get(w).flags & 0x410000, 0x410000);
        let mut expected = Vec::new();
        if initialized {
            expected.push(event(NativeOp::BufferReleaseSurface(1)));
        }
        expected.extend([event(NativeOp::Release(1)), Call::AcquireGraphics]);
        assert_eq!(host.calls, expected);
    }
}

#[test]
fn wt14_m_missing_graphics_clears_borrowed_caches_but_keeps_buffered_byte() {
    let (mut wins, mut host, w) = window();
    wins.get_mut(w).private_buffer = None;
    wins.set_buffer(&mut host, w);
    assert_eq!((wins.get(w).draw_buffer, wins.get(w).draw_context), (None, None));
    assert!(wins.get(w).buffered);
    assert!(wins.get(w).draw_origin.is_some());
    assert_eq!(host.calls, [Call::AcquireGraphics]);
}

#[test]
fn wt14_n_query_failure_output_is_released_without_context_call() {
    for query_ok in [false, true] {
        let (mut wins, mut host, w) = window();
        host.query_output = (query_ok, Some(40));
        host.context = Some(50);
        wins.set_buffer(&mut host, w);
        assert_eq!(wins.get(w).draw_context, query_ok.then_some(50));
        assert_eq!(wins.get(w).draw_buffer, Some(1));
        assert!(wins.get(w).draw_origin.is_some());
        let mut expected = vec![Call::Query(1)];
        if query_ok {
            expected.push(Call::Context(40));
        }
        expected.push(event(NativeOp::Release(40)));
        assert_eq!(host.calls, expected);
    }
}

#[test]
fn wt14_graphics_screen_null_output_still_clears_buffered_byte() {
    let (mut wins, mut host, w) = window();
    wins.get_mut(w).private_buffer = None;
    host.graphics = Some(30);
    wins.set_buffer(&mut host, w);
    assert!(!wins.get(w).buffered);
    assert_eq!(wins.get(w).draw_buffer, None);
    assert_eq!(
        host.calls,
        [Call::AcquireGraphics, Call::Screen, event(NativeOp::Release(30))]
    );
}

#[test]
fn wt13_final_fade_release_has_only_one_base_buffer_teardown() {
    let mut wins = Windows::default();
    let mut host = GraphicsHost::default();
    let w = wins.create_fade(&mut host, 1, None);
    wins.init(&mut host, w);
    wins.get_mut(w).private_buffer = Some(1);
    host.buffers.insert(
        1,
        BufferInfo {
            initialized: true,
            ..BufferInfo::default()
        },
    );
    wins.add_ref(w);
    host.calls.clear();
    assert_eq!(wins.release(&mut host, w), 0);
    assert!(!wins.windows.contains_key(&w));
    for op in [
        NativeOp::BufferReleaseSurface(1),
        NativeOp::Release(1),
        NativeOp::Unregister(w),
    ] {
        assert_eq!(host.calls.iter().filter(|c| **c == event(op.clone())).count(), 1);
    }
    assert!(!host.calls.iter().any(|c| matches!(c, Call::Event(NativeOp::AddRef(_)))));
}
