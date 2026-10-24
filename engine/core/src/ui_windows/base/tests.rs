// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Default)]
struct TestHost {
    events: Vec<NativeOp>,
    buffers: BTreeMap<Object, BufferInfo>,
    time: u32,
    fail_init: Option<Window>,
    fail_create: bool,
    fail_lock: bool,
    handled: bool,
    filter_handles: bool,
    acceleration: bool,
    transparent: bool,
    no_service: bool,
    calls: Vec<(Window, u32, Vec<u32>)>,
    mutate_filter: bool,
    shutdown_in_filter: bool,
}
impl Host for TestHost {
    fn window_manager(&mut self) -> Object {
        100
    }
    fn manager_keyboard(&mut self, manager: Object) -> Option<Object> {
        assert_eq!(manager, 100);
        Some(200)
    }
    fn event(&mut self, event: NativeOp) {
        match event {
            NativeOp::BufferColor(b, c) => self.buffers.entry(b).or_default().color = Some(c),
            NativeOp::TransparentColor(b, c) => self.buffers.entry(b).or_default().transparent = c,
            _ => {}
        }
        self.events.push(event);
    }
    fn clock(&mut self) -> u32 {
        self.time
    }
    fn screen_buffer(&mut self) -> Option<Object> {
        Some(900)
    }
    fn draw_service_present(&mut self) -> bool {
        !self.no_service
    }
    fn buffer_info(&mut self, b: Object) -> BufferInfo {
        self.buffers.get(&b).copied().unwrap_or_default()
    }
    fn create_private_buffer(&mut self, w: Window, width: i32, height: i32) -> Option<Object> {
        if self.fail_create {
            return None;
        }
        let b = 1000 + w;
        self.buffers.insert(
            b,
            BufferInfo {
                area: Rect(0, 0, width, height),
                usable: true,
                content: true,
                color: Some(u32::MAX),
                ..BufferInfo::default()
            },
        );
        Some(b)
    }
    fn lock_buffer(&mut self, _: Object, _: u32) -> bool {
        !self.fail_lock
    }
    fn pixel_transparent(&mut self, _: Object, _: i32, _: i32) -> bool {
        self.transparent
    }
    fn transparent_color(&mut self, id: u32) -> Option<u32> {
        assert_eq!(id, 0x08963f6c);
        Some(42)
    }
    fn native_to_rgb(&mut self, _: Option<Object>, _: u32) -> Option<[u8; 3]> {
        Some([1, 2, 3])
    }
    fn rgb_to_native(&mut self, _: Option<Object>, rgb: [u8; 3]) -> Option<u32> {
        Some(u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2]))
    }
    fn init(&mut self, wins: &mut Windows, w: Window) -> bool {
        if self.fail_init == Some(w) {
            false
        } else {
            wins.init(self, w)
        }
    }
    fn filter(&mut self, wins: &mut Windows, f: Object, w: Window, _: Message) -> bool {
        if self.mutate_filter {
            self.mutate_filter = false;
            wins.remove_filter(self, w, f);
            if self.shutdown_in_filter {
                wins.shutdown(self, w);
            }
        }
        self.filter_handles
    }
    fn handler(&mut self, _: &mut Windows, w: Window, slot: u32, args: &[u32]) -> bool {
        self.calls.push((w, slot, args.to_vec()));
        self.handled
    }
    fn accelerate(&mut self, _: Object, _: Message) -> bool {
        self.acceleration
    }
    fn cursor(&mut self, _: Window) -> Option<Object> {
        Some(555)
    }
    fn query_interface(&mut self, w: Window, iid: u32) -> Option<Object> {
        (iid == 99).then_some(w)
    }
}
fn setup() -> (Windows, TestHost, Window) {
    let mut s = Windows::new(7);
    let mut h = TestHost::default();
    let p = s.create(&mut h, 100, Some(200));
    s.set_area(&mut h, p, Rect(0, 0, 100, 100));
    s.main_window = Some(p);
    (s, h, p)
}
fn child(s: &mut Windows, h: &mut TestHost, p: Window, r: Rect) -> Window {
    let c = s.create(h, 100, Some(200));
    s.set_area(h, c, r);
    assert!(s.child_add(h, p, c));
    c
}
fn alpha(h: &TestHost, b: Object) -> u32 {
    h.buffers[&b].color.unwrap() >> 24
}

#[test]
fn wt01_constructor_and_lifecycle() {
    let (mut s, mut h, p) = setup();
    let w = s.get(p);
    assert_eq!(
        (w.flags, w.fade_periods, w.shade, w.instance_id),
        (0x8903, (500, 500), u32::MAX, 7)
    );
    assert_eq!((w.fill, w.unnamed_0c, w.unknown_a0_a4), (0, 0, [0; 2]));
    s.init(&mut h, p);
    s.init(&mut h, p);
    s.set_accelerator(&mut h, p, Some(70));
    s.add_filter(&mut h, p, 71);
    s.set_param(&mut h, p, 9, Some(Param::Object(72)));
    s.shutdown(&mut h, p);
    let n = h.events.len();
    s.shutdown(&mut h, p);
    assert_eq!(n, h.events.len());
    assert!(s.get(p).params().is_empty());
    for o in [70, 71, 72] {
        assert!(h.events.contains(&NativeOp::Release(o)));
    }
    s.destroy(&mut h, p);
    assert!(!s.valid(p));
    assert!(h.events.contains(&NativeOp::Unregister(p)));
}
#[test]
fn wt02_03_front_order_duplicate() {
    let (mut s, mut h, p) = setup();
    let a = child(&mut s, &mut h, p, Rect(0, 0, 20, 20));
    let b = child(&mut s, &mut h, p, Rect(0, 0, 20, 20));
    assert_eq!(s.child_from_point(&mut h, p, 10, 10), Some(b));
    assert!(!s.child_add(&mut h, p, b));
    assert_eq!(s.get(p).children(), &[b, a]);
    assert!(s.child_to_back(p, b));
    assert_eq!(s.child_from_point(&mut h, p, 10, 10), Some(a));
}
#[test]
fn wt04_failed_init() {
    let (mut s, mut h, p) = setup();
    let c = s.create(&mut h, 100, None);
    h.fail_init = Some(c);
    assert!(!s.child_add(&mut h, p, c));
    assert_eq!(s.get(c).parent, None);
    assert!(s.get(p).children().is_empty());
}
#[test]
fn wt05_06_exact_mask_focus_capture() {
    let (mut s, mut h, p) = setup();
    let c = child(&mut s, &mut h, p, Rect(0, 0, 10, 10));
    s.set_focus(&mut h, Some(c));
    s.capture = Some(c);
    assert_eq!(s.set_flag(&mut h, c, 3, false), s.get(c).flags);
    assert_eq!(s.focus, Some(c));
    s.set_flag(&mut h, c, 1, false);
    assert_eq!(s.focus, Some(p));
    s.set_focus(&mut h, Some(c));
    s.set_flag(&mut h, c, 2, false);
    assert_eq!(s.capture, None);
    assert_eq!(s.focus, Some(p));
    assert!(!s.set_focus_to(&mut h, Some(99999)));
}
#[test]
fn wt07_08_bounds_and_descendant_id() {
    let (mut s, mut h, p) = setup();
    let c = child(&mut s, &mut h, p, Rect(90, 90, 120, 120));
    assert_eq!(s.child_from_point(&mut h, p, 110, 110), Some(c));
    s.get_mut(p).id = 7;
    s.get_mut(c).id = 7;
    assert_eq!(s.child_from_id_recursive(p, 7), Some(c));
    assert_eq!(s.child_from_id(p, 7), Some(c));
    assert!(!s.point_parent(c, 120, 110));
    assert!(s.in_child_chain(p, c));
    assert!(s.in_parent_chain(c, p));
}
#[test]
fn wt09_fade_and_unsigned_clock() {
    let (mut s, mut h, p) = setup();
    s.set_flag(&mut h, p, 0x10000, true);
    s.set_flag(&mut h, p, 0x20, true);
    let b = s.get(p).private_buffer.unwrap();
    for (time, value) in [(100, 51), (250, 127), (502, 255)] {
        h.time = time;
        s.fade_advance(&mut h, p);
        assert_eq!(alpha(&h, b), value);
    }
    assert_eq!(s.get(p).integer(FADE_PERIOD), 0);
    h.time = u32::MAX - 49;
    s.set_fade(&mut h, p, 0, 500);
    h.time = 50;
    s.fade_advance(&mut h, p);
    assert_eq!(alpha(&h, b), 51);
    s.hide(&mut h, p);
    s.show(&mut h, p);
    let time = s.get(p).integer(FADE_TIME);
    h.time += 10;
    s.show(&mut h, p);
    assert_eq!(s.get(p).integer(FADE_TIME), time);
}
#[test]
fn wt10_dispatch_filters_and_arguments() {
    let (mut s, mut h, p) = setup();
    for kind in [2, 9, 12, 15, 99] {
        assert!(!s.do_message(&mut h, p, Message::new(kind, 1, 2, 3, 4)));
    }
    assert!(h.calls.is_empty());
    s.add_filter(&mut h, p, 50);
    h.filter_handles = true;
    assert!(s.do_message(&mut h, p, Message::new(7, 1, 2, 3, 4)));
    assert!(h.calls.is_empty());
    h.filter_handles = false;
    h.mutate_filter = true;
    s.do_message(&mut h, p, Message::new(7, 0x1234ffff, 0xabcd8000, 0xbeef0000, 0));
    assert_eq!(h.calls.last().unwrap().2, vec![u32::MAX, (-32768i32) as u32, 0xbeef]);
    assert!(s.get(p).filters().is_empty());
    assert!(h.events.contains(&NativeOp::Release(50)));
    s.do_message(&mut h, p, Message::new(14, 0x12345678, 0x87654321, 0x8002fffe, 0));
    assert_eq!(
        h.calls.last().unwrap().2,
        vec![0x12345678, 0x87654321, 0x8002, (-2i32) as u32]
    );
    for (kind, slot) in [
        (3, 143),
        (4, 129),
        (5, 130),
        (6, 131),
        (8, 135),
        (10, 136),
        (11, 137),
        (13, 138),
        (16, 133),
        (17, 132),
        (18, 140),
        (19, 141),
        (20, 142),
    ] {
        s.do_message(&mut h, p, Message::new(kind, 0x123, 4, 0, 0));
        assert_eq!(h.calls.last().unwrap().1, slot);
    }
    s.set_accelerator(&mut h, p, Some(80));
    h.acceleration = true;
    assert!(s.send(&mut h, p, Message::new(5, 78, 0, 0, 0)));
    assert!(!s.send(&mut h, 12345, Message::default()));
    let n = h.calls.len();
    s.post(&mut h, p, Message::new(3, 4, 5, 0, 0));
    assert_eq!(h.calls.len(), n);
    s.dispatch_posted(&mut h);
    assert_eq!(h.calls.len(), n + 1);
}
#[test]
fn wt11_painter_order() {
    let (mut s, mut h, p) = setup();
    let b = child(&mut s, &mut h, p, Rect(0, 0, 10, 10));
    let a = child(&mut s, &mut h, p, Rect(0, 0, 10, 10));
    h.events.clear();
    assert!(s.plot(&mut h, p));
    let order: Vec<_> = h
        .events
        .iter()
        .filter_map(|e| if let NativeOp::Paint(w) = e { Some(*w) } else { None })
        .collect();
    assert_eq!(order, [p, b, a]);
}
#[test]
fn wt12_overlay_queue_and_present() {
    let (mut s, mut h, p) = setup();
    let o = child(&mut s, &mut h, p, Rect(0, 0, 20, 20));
    let c = child(&mut s, &mut h, p, Rect(10, 10, 30, 30));
    for w in [o, c] {
        s.set_flag(&mut h, w, 0x10000, true);
    }
    s.set_flag(&mut h, o, 0x8000000, true);
    h.events.clear();
    s.plot(&mut h, p);
    let queues: Vec<_> = h
        .events
        .iter()
        .filter_map(|e| if let NativeOp::QueuePlot(w) = e { Some(*w) } else { None })
        .collect();
    assert_eq!(queues, [o, c]);
    assert!(!s.get(o).flag(0x4000000));
    assert!(!s.get(c).flag(0x4000000));
    let first_paint = h.events.iter().position(|e| *e == NativeOp::Paint(o)).unwrap();
    let last_queue = h
        .events
        .iter()
        .rposition(|e| matches!(e, NativeOp::QueuePlot(_)))
        .unwrap();
    assert!(last_queue < first_paint);
}
#[test]
fn wt13_clean_private_buffer() {
    let (mut s, mut h, p) = setup();
    let c = child(&mut s, &mut h, p, Rect(0, 0, 10, 10));
    s.set_flag(&mut h, c, 0x10000, true);
    s.plot(&mut h, c);
    h.events.clear();
    s.plot(&mut h, c);
    assert!(!h.events.contains(&NativeOp::Paint(c)));
    s.invalidate_self(c);
    s.plot(&mut h, c);
    assert!(h.events.contains(&NativeOp::Paint(c)));
}
#[test]
fn wt14_fade_temporary() {
    let (mut s, mut h, p) = setup();
    let c = child(&mut s, &mut h, p, Rect(10, 10, 30, 30));
    s.set_flag(&mut h, c, 0x10000, true);
    s.set_flag(&mut h, c, 0x20, true);
    s.plot(&mut h, c);
    let b = s.get(c).private_buffer.unwrap();
    s.set_flag(&mut h, c, 1, false);
    let tmp = *s.get(p).children().iter().find(|&&w| w != c).unwrap();
    assert_eq!(s.get(tmp).area, s.get(c).area);
    assert_eq!(s.get(tmp).private_buffer, Some(b));
    assert_eq!(alpha(&h, b), 255);
    for (time, expected) in [(250, 127), (500, 0), (501, 0)] {
        h.time = time;
        s.plot(&mut h, tmp);
        assert_eq!(alpha(&h, b), expected);
    }
    assert!(s.valid(tmp));
    s.plot(&mut h, tmp);
    assert!(!s.valid(tmp));
}
#[test]
fn wt15_cursor_disabled_click_and_move() {
    let (mut s, mut h, p) = setup();
    let b = child(&mut s, &mut h, p, Rect(5, 5, 20, 20));
    let a = child(&mut s, &mut h, p, Rect(5, 5, 20, 20));
    s.set_flag(&mut h, a, 2, false);
    let mut click = Message::new(7, 10, 10, 0, 0);
    s.process_cursor(&mut h, p, &mut click);
    assert_eq!(s.focus, Some(b));
    assert_eq!(s.mouse_window, Some(b));
    assert_eq!(&click.0[1..3], &[5, 5]);
    s.child_to_front(p, a);
    h.events.clear();
    let mut msg = Message::new(13, 10, 10, 0, 0);
    s.process_cursor(&mut h, p, &mut msg);
    assert_eq!(s.mouse_window, Some(a));
    assert!(!h.events.iter().any(|e| matches!(e, NativeOp::CursorActivate(_))));
    s.set_flag(&mut h, a, 2, true);
    s.update_cursor(&mut h, a);
    s.update_cursor(&mut h, a);
    assert_eq!(h.events.iter().filter(|e| **e == NativeOp::CursorLookup(a)).count(), 1);
}
#[test]
fn wt16_key_equivalents() {
    let (mut s, mut h, p) = setup();
    let c = child(&mut s, &mut h, p, Rect::default());
    s.get_mut(c).key_equivalent = 0x00020041;
    assert!(s.check_key_equivalent(&mut h, c, 0x41, 0x10002));
    assert_eq!(s.child_key_equivalent(&mut h, p, 0x41, 2), Some(c));
}
#[test]
fn wt17_draw_rectangles_and_converters() {
    let (mut s, mut h, r) = setup();
    s.set_area(&mut h, r, Rect(0, 0, 800, 600));
    let p = child(&mut s, &mut h, r, Rect(100, 50, 500, 450));
    s.set_flag(&mut h, p, 0x10000, true);
    let c = child(&mut s, &mut h, p, Rect(10, 20, 110, 70));
    let d = child(&mut s, &mut h, c, Rect(5, 5, 25, 15));
    for (w, area, origin) in [
        (p, Rect(0, 0, 400, 400), Rect(0, 0, 400, 400)),
        (c, Rect(0, 0, 100, 50), Rect(10, 20, 110, 70)),
        (d, Rect(0, 0, 20, 10), Rect(15, 25, 35, 35)),
    ] {
        assert_eq!(s.get(w).draw_area, area);
        assert_eq!(s.get(w).draw_origin, origin);
    }
    assert_eq!(s.get(p).blit, Rect(100, 50, 500, 450));
    let mut point = (0, 0);
    assert!(s.window_to_window(&mut h, d, Some(p), &mut point));
    assert_eq!(point, (15, 25));
    assert!(!s.window_to_window(&mut h, d, None, &mut point));
    assert_eq!(point, (15, 25));
}
#[test]
fn wt18_overlay_invalidation() {
    let (mut s, mut h, r) = setup();
    let o = child(&mut s, &mut h, r, Rect::default());
    let d = child(&mut s, &mut h, o, Rect::default());
    s.set_flag(&mut h, o, 0x8000000, true);
    for w in [r, o, d] {
        s.get_mut(w).dirty = false;
    }
    s.invalidate_self_and_parents(d);
    assert!(!s.get(r).dirty);
    assert!(s.get(o).dirty && s.get(d).dirty);
    s.get_mut(o).dirty = false;
    s.invalidate_self_and_parents(o);
    assert!(!s.get(r).dirty);
}
#[test]
fn wt19_sort_and_reorder() {
    let (mut s, mut h, p) = setup();
    let mut children = Vec::new();
    for id in [2, 1, 3] {
        let c = child(&mut s, &mut h, p, Rect::default());
        s.get_mut(c).id = id;
        s.set_flag(&mut h, c, 0x800, false);
        children.push(c);
    }
    assert!(s.sort_children_by_id(p, false));
    let ids: Vec<_> = s.get(p).children().iter().map(|&c| s.get(c).id).collect();
    assert_eq!(ids, [1, 2, 3]);
    let a = children[1];
    let b = children[0];
    assert!(s.child_step(p, a, false));
    s.get_mut(a).flags |= 0x800;
    s.child_move_relative(p, b, a, true);
    assert!(s.sort_children_by_id(p, false));
    assert_eq!(&s.get(p).children()[..2], &[b, a]);
    assert!(!s.child_to_front(p, b));
    assert!(s.child_step(p, b, true));
}
#[test]
fn wt20_placement() {
    let (mut s, mut h, p) = setup();
    let mut r = Rect(-10, 0, 30, 20);
    assert!(s.fit_rect(p, &mut r, 4));
    assert_eq!(r, Rect(4, 0, 30, 20));
    s.set_area(&mut h, p, Rect(0, 0, 13, 10));
    s.center(&mut h, p, Some(Rect(0, 0, 10, 10)));
    assert_eq!(s.get(p).area, Rect(-1, 0, 12, 10));
}
#[test]
fn wt21_params_ownership_order_and_enum() {
    let (mut s, mut h, p) = setup();
    s.set_param(&mut h, p, 5, Some(Param::Object(100)));
    let mut out = None;
    assert!(s.get_param(&mut h, p, 5, &mut out));
    assert_eq!(out, Some(Param::Object(100)));
    h.events.clear();
    s.set_param(&mut h, p, 5, Some(Param::Object(100)));
    assert_eq!(&h.events[..2], &[NativeOp::Release(100), NativeOp::AddRef(100)]);
    s.set_param(&mut h, p, 2, Some(Param::Object(101)));
    let mut keys = Vec::new();
    assert!(s.enum_params(p, |_, _, k, _| {
        keys.push(k);
        true
    }));
    assert_eq!(keys, [2, 5]);
    s.set_param(&mut h, p, 5, None);
    assert!(!s.get_param(&mut h, p, 5, &mut out));
    assert_eq!(out, Some(Param::Object(100)));
}
#[test]
fn snapshot_enumeration_and_busy_destruction() {
    let (mut s, mut h, p) = setup();
    let a = child(&mut s, &mut h, p, Rect::default());
    let b = child(&mut s, &mut h, p, Rect::default());
    let refs = s.get(a).references;
    let snapshot = s.read_children(p);
    assert_eq!(s.get(a).references, refs);
    s.child_to_back(p, b);
    assert_eq!(s.get(a).references, refs + 1);
    assert_eq!(&*snapshot, &[b, a]);
    s.unlock_children(snapshot);
    assert_eq!(s.get(a).references, refs);
    let mut visited = Vec::new();
    assert!(!s.enum_children(
        &mut h,
        p,
        99,
        Some(|s: &mut Windows, _, _, obj| {
            visited.push(obj);
            s.child_to_front(p, b);
            false
        })
    ));
    assert_eq!(visited, [a, b]);
    assert!(h.events.contains(&NativeOp::Release(a)));
    s.begin_busy();
    assert!(s.child_delete_all(&mut h, p));
    assert!(s.get(p).children().is_empty());
    assert!(s.valid(a));
    s.end_busy(&mut h);
    assert!(!s.valid(a));
    assert!(!s.valid(b));
}
#[test]
fn transparency_native_colors_and_draw_context() {
    let (mut s, mut h, p) = setup();
    s.set_flag(&mut h, p, 0x10000, true);
    s.set_flag(&mut h, p, 0x20000, true);
    s.set_flag(&mut h, p, 0x80000, true);
    h.transparent = true;
    assert!(s.point_parent(p, 5, 5));
    assert!(!s.point_window(&mut h, p, 5, 5));
    assert!(s.fill_transparent(&mut h, p));
    assert_eq!(s.packed_to_native(&mut h, p, 0xaabbccdd), 0xbbccdd);
    assert_eq!(s.native_to_packed(&mut h, p, 99), 0xff010203);
    let b = s.get(p).private_buffer.unwrap();
    h.buffers.get_mut(&b).unwrap().context = Some(99);
    h.buffers.insert(
        900,
        BufferInfo {
            context: Some(77),
            present_context: Some(98),
            ..BufferInfo::default()
        },
    );
    s.set_buffer(&mut h, p);
    h.events.clear();
    s.plot(&mut h, p);
    assert!(h.events.contains(&NativeOp::Context(99, 22, vec![0])));
    assert!(h.events.contains(&NativeOp::Context(98, 21, vec![u32::MAX])));
    assert!(!h.events.iter().any(|e| matches!(e, NativeOp::Context(77, _, _))));
    h.fail_lock = true;
    assert!(!s.fill_transparent(&mut h, p));
}
#[test]
fn private_buffer_failure_and_absent_draw_service() {
    let (mut s, mut h, p) = setup();
    h.fail_create = true;
    assert!(!s.private_buffer(&mut h, p, true));
    assert!(s.get(p).flag(0x10000));
    assert!(s.get(p).dirty);
    let mut s = Windows::new(1);
    h.no_service = true;
    let r = s.create(&mut h, 0, None);
    s.set_area(&mut h, r, Rect(10, 20, 110, 120));
    assert_eq!(s.get(r).draw_area, Rect(10, 20, 110, 120));
    assert_eq!(s.get(r).draw_origin, Rect(20, 40, 120, 140));
}

#[test]
fn shutdown_during_filter_releases_each_entry_once() {
    let (mut s, mut h, p) = setup();
    s.init(&mut h, p);
    s.add_filter(&mut h, p, 50);
    h.mutate_filter = true;
    h.shutdown_in_filter = true;
    s.do_message(&mut h, p, Message::new(3, 0, 0, 0, 0));
    assert_eq!(h.events.iter().filter(|e| **e == NativeOp::Release(50)).count(), 1);
    assert!(s.get(p).filters().is_empty());
    assert!(s.get(p).removed_filters.is_empty());
}

#[test]
fn ineligible_fade_releases_surface_without_temporary() {
    let (mut s, mut h, p) = setup();
    let c = child(&mut s, &mut h, p, Rect(0, 0, 20, 20));
    s.set_flag(&mut h, c, 0x10000, true);
    s.set_flag(&mut h, c, 0x20, true);
    s.plot(&mut h, c);
    let b = s.get(c).private_buffer.unwrap();
    // Combined masks deliberately bypass private-buffer side effects.
    s.set_flag(&mut h, p, 3, false);
    h.events.clear();
    s.private_buffer(&mut h, c, false);
    assert!(h.events.contains(&NativeOp::BufferReleaseSurface(b)));
    assert_eq!(s.get(p).children(), &[c]);
    assert!(!s.get(c).flag(0x400000));
}

#[test]
fn constructors_reparenting_and_tree_forwarders() {
    let (mut s, mut h, r) = setup();
    let p = s.create_default(&mut h);
    assert_eq!((s.get(p).manager, s.get(p).keyboard), (100, Some(200)));
    s.child_add(&mut h, r, p);
    let q = child(&mut s, &mut h, r, Rect::default());
    let a = child(&mut s, &mut h, p, Rect::default());
    assert_eq!(s.get(a).notification, Some(p));
    s.child_add(&mut h, q, a);
    assert_eq!((s.get(a).parent, s.get(a).notification), (Some(q), Some(q)));
    assert!(!s.get(p).children().contains(&a));
    s.get_mut(a).notification = Some(r);
    s.child_add(&mut h, p, a);
    assert_eq!(s.get(a).notification, Some(r));
    s.set_flag(&mut h, a, 0x800, false);
    assert!(!s.child_to_front(p, a));
    assert_eq!(s.get(r).children()[0], p); // ancestors raised even on rejection
    assert!(s.send_to_back(p));
    s.get_mut(p).id = 42;
    assert!(s.child_to_front_id(r, 42));
    assert!(!s.pull_to_front(r));
    assert!(!s.in_child_chain(r, r));
    assert!(!s.in_parent_chain(r, r));
    assert!(s.child_remove(&mut h, p, a));
    assert_eq!(s.get(a).parent, None);
    assert_eq!(s.get(a).notification, Some(r));
    assert!(s.get(a).flag(0x4000)); // detach is not shutdown
    s.child_add(&mut h, p, a);
    assert!(s.child_delete_absolute(&mut h, p, a));
    assert!(!s.valid(a));
    assert!(!s.child_delete_absolute(&mut h, p, a));
}

#[test]
fn movement_inherited_buffers_and_child_buffer_rules() {
    let (mut s, mut h, r) = setup();
    let p = child(&mut s, &mut h, r, Rect(10, 20, 90, 80));
    s.set_flag(&mut h, p, 0x10000, true);
    let c = child(&mut s, &mut h, p, Rect(1, 2, 21, 22));
    let d = child(&mut s, &mut h, c, Rect(3, 4, 13, 14));
    assert_eq!(s.get(d).draw_buffer, s.get(p).private_buffer);
    s.set_flag(&mut h, d, 0x8000000, true);
    assert_eq!(s.get(d).draw_buffer, Some(900)); // only first parent is checked
    s.move_to(&mut h, c, 5, 6);
    s.offset(&mut h, c, 2, 3);
    assert_eq!(s.get(c).area, Rect(7, 9, 27, 29));
    assert_eq!(s.get(d).absolute, Rect(20, 33, 30, 43));
    let before = s.get(c).area;
    s.center(&mut h, c, None);
    assert_eq!(s.get(c).area, before);
    let mut rect = Rect(90, 90, 120, 120);
    assert!(s.fit_rect(r, &mut rect, 4));
    assert_eq!(rect, Rect(66, 66, 96, 96));
    s.set_flag(&mut h, c, 0x10000, true);
    let buffer = s.get(c).private_buffer;
    s.set_flag(&mut h, c, 3, false);
    s.child_private_buffers(&mut h, p, true);
    assert_eq!(s.get(c).private_buffer, buffer); // hidden buffered child's own buffer retained
    s.child_private_buffers(&mut h, p, false);
    assert_eq!(s.get(c).private_buffer, None);
}

#[test]
fn empty_enumeration_paint_filter_and_queue_boundaries() {
    let (mut s, mut h, p) = setup();
    type Callback = fn(&mut Windows, Window, u32, Object) -> bool;
    assert!(!s.enum_children(&mut h, p, IID_WIN, None::<Callback>));
    assert!(!s.enum_children(&mut h, p, IID_WIN, Some(|_: &mut Windows, _, _, _| true)));
    child(&mut s, &mut h, p, Rect::default());
    assert!(s.enum_children(
        &mut h,
        p,
        123,
        Some(|_: &mut Windows, _, _, _| panic!("failed QI must skip callback"))
    ));
    s.add_filter(&mut h, p, 50);
    h.filter_handles = true;
    h.events.clear();
    assert!(s.plot_composite(&mut h, p));
    assert!(!h.events.contains(&NativeOp::Paint(p)));
    s.queue_enabled = false;
    assert!(!s.queue_plot(&mut h, Some(p)));
    assert!(!s.flush_plots(&mut h));
    assert!(!s.queue_plot(&mut h, None));
    s.remove_filter(&mut h, p, 50);
    s.hide(&mut h, p);
    assert!(!s.plot(&mut h, p));
    s.set_param(&mut h, p, 1, Some(Param::Integer(1)));
    s.set_param(&mut h, p, 2, Some(Param::Integer(2)));
    let mut keys = Vec::new();
    assert!(!s.enum_params(p, |_, _, key, _| {
        keys.push(key);
        false
    }));
    assert_eq!(keys, [1]);
}
