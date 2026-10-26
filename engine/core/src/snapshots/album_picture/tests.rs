// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Default)]
struct Services {
    ops: Vec<ViewOp>,
    notifications: Vec<(Window, u32, [u32; 3])>,
    elapsed: Option<u32>,
    locked: bool,
    initializations: usize,
    clicks: Vec<[i32; 2]>,
    restarts: usize,
    resets: usize,
    messages: Vec<[u32; 4]>,
}

impl Host for Services {}
impl PictureHost for Services {
    fn init_view(&mut self, _: Window) -> Result<()> {
        self.initializations += 1;
        Ok(())
    }
    fn view_op(&mut self, _: Window, op: ViewOp) {
        self.ops.push(op);
    }
    fn view_left_click(&mut self, _: Window, point: [i32; 2]) {
        self.clicks.push(point);
    }
    fn view_message(&mut self, _: Window, message: [u32; 4]) -> bool {
        self.messages.push(message);
        true
    }
    fn scroll_lock(&mut self) -> bool {
        self.locked
    }
    fn elapsed_ms(&mut self, _: Window) -> Option<u32> {
        self.elapsed
    }
    fn restart_timer(&mut self, _: Window) {
        self.restarts += 1;
        self.elapsed = Some(0);
    }
    fn reset_timer(&mut self, _: Window) {
        self.resets += 1;
        self.elapsed = None;
    }
    fn send_parent(&mut self, parent: Window, message: u32, words: [u32; 3]) {
        self.notifications.push((parent, message, words));
    }
}

#[test]
fn constructor_init_does_not_resize_or_apply_fit() {
    let mut windows = Windows::default();
    let mut host = Services::default();
    let window = windows.create_default(&mut host);
    let before = windows.get(window).area;
    let mut picture = AlbumPicture::new(window, true);
    picture.init(&mut host).unwrap();
    assert!(picture.is_large());
    assert_eq!(host.initializations, 1);
    assert_eq!(host.ops, [ViewOp::BitmapFlag { flag: 2, on: true }]);
    assert!(picture.set_size(&mut windows, &mut host, true));
    assert_eq!(windows.get(window).area, before);
    assert!(host.notifications.is_empty());
    assert_eq!(host.ops.len(), 1);
}

#[test]
fn size_switches_use_base_tree_geometry_and_notify_parent() {
    let mut windows = Windows::default();
    let mut host = Services::default();
    let parent = windows.create_default(&mut host);
    let window = windows.create_default(&mut host);
    windows.get_mut(window).id = 17;
    assert!(windows.child_add(&mut host, parent, window));
    let sibling = windows.create_default(&mut host);
    assert!(windows.child_add(&mut host, parent, sibling));
    let mut picture = AlbumPicture::new(window, true);
    host.locked = true;
    picture.set_size(&mut windows, &mut host, false);
    assert_eq!(windows.get(window).area, Rect(0, 0, 32, 32));
    assert_eq!(windows.get(parent).children().first(), Some(&window));
    assert_eq!(
        host.ops,
        [
            ViewOp::Fit(false),
            ViewOp::Centred(1),
            ViewOp::ZoomKeys(false),
            ViewOp::Scrolling(false)
        ]
    );
    assert_eq!(host.notifications, [(parent, 0x9127_484a, [17, 0, 0])]);
    host.locked = false;
    picture.set_should_fit(&mut host, true);
    assert_eq!(host.ops.last(), Some(&ViewOp::Fit(true)));
    picture.set_size_areas(Rect(2, 3, 202, 303), Rect(4, 5, 44, 55));
    picture.set_size(&mut windows, &mut host, true);
    assert_eq!(windows.get(window).area, Rect(2, 3, 202, 303));
    assert_eq!(host.notifications.last(), Some(&(parent, 0x9127_484a, [17, 1, 0])));
}

#[test]
fn double_click_under_500_toggles_and_reset_prevents_a_third_toggle() {
    let mut windows = Windows::default();
    let mut host = Services::default();
    let window = windows.create_default(&mut host);
    let mut picture = AlbumPicture::new(window, false);
    picture.left_click(&mut windows, &mut host, [1, 2]);
    host.elapsed = Some(499);
    picture.left_click(&mut windows, &mut host, [3, 4]);
    assert!(!picture.is_large());
    assert_eq!(host.resets, 1);
    assert_eq!(host.clicks, [[1, 2]]);
    picture.left_click(&mut windows, &mut host, [5, 6]);
    assert!(!picture.is_large());
    host.elapsed = Some(500);
    picture.left_click(&mut windows, &mut host, [7, 8]);
    assert!(!picture.is_large());
    assert_eq!(host.restarts, 3);
    assert_eq!(host.clicks, [[1, 2], [5, 6], [7, 8]]);
    assert!(picture.message(&mut host, [42, 1, 2, 3]));
    assert_eq!(host.messages, [[42, 1, 2, 3]]);
}
