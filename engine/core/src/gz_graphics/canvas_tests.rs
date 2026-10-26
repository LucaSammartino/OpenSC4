// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::VecDeque;

struct NativeHost {
    calls: Vec<NativeCall>,
    interfaces: bool,
    modes: Vec<CanvasMode>,
    mode_calls: Vec<(i32, bool)>,
    status: i32,
    flags: u32,
    code_page: u32,
    extra_bytes: usize,
    default_calls: Vec<NativeMessage>,
}
fn mode() -> CanvasMode {
    CanvasMode {
        width: 100,
        height: 50,
        discriminator: 1,
        format: 5,
    }
}
fn native_host() -> NativeHost {
    NativeHost {
        calls: vec![],
        interfaces: true,
        modes: vec![mode()],
        mode_calls: vec![],
        status: 0,
        flags: 0,
        code_page: 1252,
        extra_bytes: 0,
        default_calls: vec![],
    }
}
impl CanvasNativeHost for NativeHost {
    fn call(&mut self, call: NativeCall) {
        self.calls.push(call);
    }
    fn default_message(&mut self, message: NativeMessage) -> isize {
        self.default_calls.push(message);
        99
    }
    fn interfaces_available(&self) -> bool {
        self.interfaces
    }
    fn modes(&self) -> Vec<CanvasMode> {
        self.modes.clone()
    }
    fn change_mode(&mut self, index: i32, create: bool) -> Option<usize> {
        self.mode_calls.push((index, create));
        create.then_some(123)
    }
    fn mode_status(&self) -> i32 {
        self.status
    }
    fn windowed(&self) -> bool {
        true
    }
    fn work_area(&self) -> Option<Rect> {
        Some(Rect(10, 20, 1010, 820))
    }
    fn window_rect(&self) -> Option<Rect> {
        Some(Rect(5, 5, 205, 105))
    }
    fn visible(&self) -> bool {
        true
    }
    fn mapped_client(&self) -> Rect {
        Rect(10, 20, 110, 70)
    }
    fn has_paint_dc(&self) -> bool {
        true
    }
    fn owns_capture(&self, _: usize) -> bool {
        false
    }
    fn key_flags(&self) -> u32 {
        self.flags
    }
    fn screen_to_client(&self, p: [i32; 2]) -> [i32; 2] {
        [p[0] - 10, p[1] - 20]
    }
    fn wheel_lines(&self) -> Option<u32> {
        Some(3)
    }
    fn locale_code_page(&self, _: u16) -> u32 {
        0
    }
    fn framework_code_page(&self) -> u32 {
        self.code_page
    }
    fn additional_bytes(&self, _: u32, _: u8) -> usize {
        self.extra_bytes
    }
    fn convert_text(&self, _: u32, _: u32, bytes: &[u8]) -> Vec<u8> {
        assert_eq!(bytes, [0x81, 0x40]);
        vec![0xe9, 0x41]
    }
}
fn message(id: u32, wparam: u32, lparam: u32) -> NativeMessage {
    NativeMessage {
        handle: 123,
        id,
        wparam,
        lparam,
    }
}

#[test]
fn gzg4_v62_v63_v64_realization_failure_release_and_repaint() {
    let trace = log();
    let mut c = canvas(&trace);
    c.request_repaint();
    c.paint_suppressed = true;
    c.request_repaint();
    assert_eq!(take(&trace), [Event::Canvas(CanvasCall::Event(4))]);
    let mut host = native_host();
    host.modes.clear();
    assert!(!c.initialize_native(Rect(0, 0, 100, 50), mode(), &mut host));
    assert!(c.realized && c.handle.is_none());
    assert!(take(&trace).is_empty());
    c.handle = Some(123);
    c.auxiliary = Some(456);
    host.interfaces = false;
    assert!(c.realize_native(true, mode(), &mut host));
    assert!(c.handle.is_none() && c.auxiliary.is_none());
    assert_eq!(host.calls, [NativeCall::FreeAuxiliary(456)]);
    assert_eq!(take(&trace), [Event::Canvas(CanvasCall::Event(9))]);
    assert!(host.mode_calls.is_empty());
}

#[test]
fn gzg4_native_mode_matching_center_suspend_resume_and_failure() {
    let trace = log();
    let mut c = canvas(&trace);
    let mut host = native_host();
    host.modes.insert(0, CanvasMode { format: 6, ..mode() });
    host.modes[1].format |= 0x80;
    assert!(c.initialize_native(Rect(0, 0, 100, 50), mode(), &mut host));
    assert_eq!(host.mode_calls, [(1, true)]);
    assert_eq!(
        host.calls,
        [NativeCall::ClassIcon(1000), NativeCall::MoveWindow(410, 370, true)]
    );
    assert_eq!(take(&trace), [Event::Canvas(CanvasCall::Event(8))]);
    assert!(c.initialize_native(Rect::default(), mode(), &mut host));
    assert_eq!(c.bounds, Rect(0, 0, 100, 50));
    assert!(c.suspend_native(mode(), &mut host));
    assert!(c.suspended && !c.realized && c.handle.is_none());
    assert_eq!(
        take(&trace),
        [
            Event::Canvas(CanvasCall::Show(1)),
            Event::Canvas(CanvasCall::Event(6)),
            Event::Canvas(CanvasCall::Event(9))
        ]
    );
    assert!(c.resume_native(Rect(0, 0, 100, 50), mode(), &mut host));
    assert!(!c.suspended && c.realized);
    assert_eq!(
        take(&trace),
        [
            Event::Canvas(CanvasCall::Event(8)),
            Event::Canvas(CanvasCall::Event(7)),
            Event::Canvas(CanvasCall::Show(1)),
            Event::Canvas(CanvasCall::Focus)
        ]
    );
    c.suspend_native(mode(), &mut host);
    take(&trace);
    host.status = 1;
    assert!(!c.resume_native(Rect::default(), mode(), &mut host));
    assert!(!c.realized);
    assert_eq!(take(&trace)[0], Event::Canvas(CanvasCall::Event(1)));
}

struct Listener {
    calls: Arc<Mutex<Vec<u32>>>,
    id: u32,
    result: Option<isize>,
}
impl CanvasListener for Listener {
    fn message(&self, _: NativeMessage) -> Option<isize> {
        self.calls.lock().unwrap().push(self.id);
        self.result
    }
}
#[test]
fn gzg4_v65_v66_v77_listener_order_ownership_and_hwnd_getters() {
    let mut c = canvas(&log());
    c.handle = Some(123);
    let calls = Arc::new(Mutex::new(vec![]));
    let a: Arc<dyn CanvasListener> = Arc::new(Listener {
        calls: calls.clone(),
        id: 1,
        result: None,
    });
    let b: Arc<dyn CanvasListener> = Arc::new(Listener {
        calls: calls.clone(),
        id: 2,
        result: Some(123),
    });
    assert!(c.native_listener(Some(&a), true));
    assert!(c.native_listener(Some(&b), true));
    assert!(!c.native_listener(Some(&a), true));
    assert_eq!((Arc::strong_count(&a), Arc::strong_count(&b)), (2, 2));
    let mut host = native_host();
    assert_eq!(c.native_message(message(16, 0, 0), &mut host), 123);
    assert_eq!(*calls.lock().unwrap(), [2]);
    assert!(host.default_calls.is_empty());
    assert!(!c.native_listener(None, false));
    assert!(c.native_listener(Some(&a), false));
    assert!(!c.native_listener(Some(&a), false));
    assert_eq!(Arc::strong_count(&a), 1);
    assert!(std::ptr::eq(c.hwnd_word(), &c.handle));
    assert_eq!(c.hwnd(), Some(123));
}

#[test]
fn gzg4_v67_through_v74_mouse_key_wheel_and_signed_text() {
    let mut c = canvas(&log());
    c.bounds = Rect(0, 0, 100, 50);
    c.handle = Some(123);
    c.code_page = 1252;
    let mut host = native_host();
    let xy = (60 << 16) | u32::from((-3_i16) as u16);
    c.native_message(message(512, 0, xy), &mut host);
    assert_eq!(c.coordinates, [(-3_i32) as u32, 60]);
    assert_eq!(
        host.calls.pop(),
        Some(NativeCall::Mouse {
            subtype: 13,
            position: [0, 49],
            amount: 0,
            flags: 0
        })
    );
    c.do_message(0xfa0c6390, Some((0x1a0c615f, 13, [0, 49])));
    assert_eq!(c.coordinates, [0, 49]);
    c.enable_input(2, false).unwrap();
    c.native_message(message(512, 0, xy), &mut host);
    assert_eq!(c.coordinates, [(-3_i32) as u32, 60]);
    assert!(host.calls.is_empty());
    c.enable_input(2, true).unwrap();
    c.bounds = Rect::default();
    c.native_message(message(512, 0, xy), &mut host);
    assert_eq!(
        host.calls.pop(),
        Some(NativeCall::Mouse {
            subtype: 13,
            position: [-1, -1],
            amount: 0,
            flags: 0
        })
    );
    host.flags = 1;
    c.native_message(message(260, 0x41, 0x40000003), &mut host);
    assert_eq!(
        host.calls.pop(),
        Some(NativeCall::Key {
            subtype: 5,
            value: 0x41,
            repeat: 3,
            flags: 0x40000001
        })
    );
    c.native_message(message(256, 0xe5, 1), &mut host);
    assert!(host.calls.is_empty());
    c.bounds = Rect(0, 0, 100, 50);
    for (delta, expected) in [(120_i16, 300), (1200, 300), (-1200, -3000)] {
        c.native_message(message(522, u32::from(delta as u16) << 16, (25 << 16) | 20), &mut host);
        assert_eq!(
            host.calls.pop(),
            Some(NativeCall::Mouse {
                subtype: 14,
                position: [10, 5],
                amount: expected,
                flags: 0
            })
        );
    }
    c.native_message(message(258, 0xe9, 2), &mut host);
    assert_eq!(
        host.calls.pop(),
        Some(NativeCall::Key {
            subtype: 4,
            value: -23,
            repeat: 2,
            flags: 1
        })
    );
    host.code_page = 65001;
    host.extra_bytes = 1;
    c.native_message(message(258, 0x81, 1), &mut host);
    assert!(host.calls.is_empty());
    c.native_message(message(258, 0x40, 1), &mut host);
    assert_eq!(
        host.calls,
        [
            NativeCall::Key {
                subtype: 4,
                value: -23,
                repeat: 1,
                flags: 1
            },
            NativeCall::Key {
                subtype: 4,
                value: 65,
                repeat: 1,
                flags: 1
            }
        ]
    );
}

struct Queue {
    pending: VecDeque<NativeMessage>,
    reposted: Vec<NativeMessage>,
}
impl CanvasQueue for Queue {
    fn peek_remove(&mut self, first: u32, last: u32) -> Option<NativeMessage> {
        assert_eq!((first, last), (0x100, 0x108));
        self.pending.pop_front()
    }
    fn repost(&mut self, m: NativeMessage) -> bool {
        self.reposted.push(m);
        false
    }
}
#[test]
fn gzg4_v75_queue_boundary_and_discard_count() {
    let messages: Vec<_> = (0..1025).map(|key| message(256, key, 1)).collect();
    let mut queue = Queue {
        pending: messages.clone().into(),
        reposted: vec![],
    };
    assert_eq!(filter_keyboard_queue(&mut queue), 0);
    assert!(queue.pending.is_empty());
    assert_eq!(queue.reposted, messages[..1024]);
    queue.pending = [message(256, 65, 1), message(258, 65, 1), message(257, 65, 1)].into();
    queue.reposted.clear();
    assert_eq!(filter_keyboard_queue(&mut queue), 3);
    assert!(queue.reposted.is_empty());
}

#[test]
fn gzg4_v76_v78_default_route_and_unconditional_delete() {
    let trace = log();
    let mut c = canvas(&trace);
    c.handle = Some(123);
    let mut host = native_host();
    assert_eq!(c.native_message(message(2, 0, 0), &mut host), 0);
    assert_eq!(c.native_message(message(16, 0, 0), &mut host), 0);
    assert_eq!(c.native_message(message(18, 0, 0), &mut host), 99);
    assert_eq!(host.default_calls, [message(18, 0, 0)]);
    assert_eq!(take(&trace), [Event::Canvas(CanvasCall::Event(10))]);
    c.realized = true;
    assert_eq!(c.delete_native(&mut host), 0);
    assert_eq!(host.calls, [NativeCall::Delete(1)]);
    assert_eq!(take(&trace)[0], Event::Canvas(CanvasCall::Event(1)));
}
