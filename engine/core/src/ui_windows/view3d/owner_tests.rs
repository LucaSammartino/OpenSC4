// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::ui_windows::win::WinStore;

#[derive(Debug, PartialEq)]
enum Op {
    Control(u32, ControlCall),
    Change(Option<u32>, Option<u32>),
    Cursor(u32),
    Capture(bool),
    Position(i32, i32),
    Jump([f32; 3]),
    View(ViewChange),
    Scroll(f32, f32),
    Message(u32),
    Begin,
    End,
    Pause,
    Resume,
    Restart,
    ResetPaint,
    Cost(f32),
    Time(f32),
    Draw,
    Tail,
}

struct Host {
    ops: Vec<Op>,
    takers: Vec<u32>,
    init_fail: Option<u32>,
    capture: bool,
    tutorial: bool,
    rewrite: Option<(i32, i32)>,
    zoom: i32,
    changed: bool,
    moved: bool,
    simulator: bool,
    scroll_control: Option<u32>,
    occupant: Option<[f32; 3]>,
    terrain: Option<[f32; 3]>,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            ops: Vec::new(),
            takers: Vec::new(),
            init_fail: None,
            capture: false,
            tutorial: true,
            rewrite: None,
            zoom: 3,
            changed: true,
            moved: false,
            simulator: true,
            scroll_control: None,
            occupant: None,
            terrain: Some([10.0, 20.0, 30.0]),
        }
    }
}

impl Host {
    fn events(&self) -> Vec<u32> {
        self.ops
            .iter()
            .filter_map(|op| match op {
                Op::Control(id, ControlCall::Event(_)) => Some(*id),
                _ => None,
            })
            .collect()
    }
}

impl MotionHost for Host {
    fn virtual_zoom(&self) -> i32 {
        self.zoom
    }
    fn camera_zoom(&self) -> f32 {
        self.zoom as f32
    }
    fn view_change(&mut self, c: ViewChange) -> bool {
        self.ops.push(Op::View(c));
        self.changed
    }
    fn scroll_camera(&mut self, a: f32, d: f32) -> bool {
        self.ops.push(Op::Scroll(a, d));
        self.moved
    }
    fn post_scroll_message(&mut self, m: u32) {
        self.ops.push(Op::Message(m));
    }
    fn begin_scroll_render_settings(&mut self) {
        self.ops.push(Op::Begin);
    }
    fn end_scroll_render_settings(&mut self) {
        self.ops.push(Op::End);
    }
    fn simulator_present(&self) -> bool {
        self.simulator
    }
    fn hidden_pause(&mut self) {
        self.ops.push(Op::Pause);
    }
    fn hidden_resume(&mut self) {
        self.ops.push(Op::Resume);
    }
    fn restart_hidden_timer(&mut self) {
        self.ops.push(Op::Restart);
    }
    fn reset_paint_timer(&mut self) {
        self.ops.push(Op::ResetPaint);
    }
    fn animation_time(&mut self) -> f32 {
        7.0
    }
    fn set_draw_context_time(&mut self, t: f32) {
        self.ops.push(Op::Time(t));
    }
    fn draw(&mut self) {
        self.ops.push(Op::Draw);
    }
    fn update_cost_indicator(&mut self, dt: f32) {
        self.ops.push(Op::Cost(dt));
    }
    fn paint_tail(&mut self) {
        self.ops.push(Op::Tail);
    }
}

impl InputHost for Host {
    fn control(&mut self, id: u32, call: ControlCall) -> ControlReply {
        self.ops.push(Op::Control(id, call));
        ControlReply {
            taken: match call {
                ControlCall::Init => self.init_fail != Some(id),
                ControlCall::Event(_) => self.takers.contains(&id),
                _ => false,
            },
            cursor: (call == ControlCall::Cursor).then_some(id + 100),
        }
    }
    fn control_changed(&mut self, old: Option<u32>, new: Option<u32>) {
        self.ops.push(Op::Change(old, new));
    }
    fn set_cursor(&mut self, c: u32) {
        self.ops.push(Op::Cursor(c));
    }
    fn has_capture(&self) -> bool {
        self.capture
    }
    fn set_capture(&mut self, c: bool) {
        self.capture = c;
        self.ops.push(Op::Capture(c));
    }
    fn tutorial_mouse(&mut self, x: &mut i32, y: &mut i32, _: i32) -> bool {
        if let Some((a, b)) = self.rewrite {
            *x = a;
            *y = b;
        }
        self.tutorial
    }
    fn cursor_position(&mut self, x: i32, y: i32) {
        self.ops.push(Op::Position(x, y));
    }
    fn create_mouse_scroll(&mut self) -> Option<u32> {
        self.scroll_control
    }
    fn occupant_centre(&mut self, _: i32, _: i32) -> Option<[f32; 3]> {
        self.occupant
    }
    fn terrain_point(&mut self, _: i32, _: i32) -> Option<[f32; 3]> {
        self.terrain
    }
    fn camera_jump(&mut self, p: [f32; 3]) {
        self.ops.push(Op::Jump(p));
    }
}

fn view() -> View3DWin {
    let mut store = WinStore::new();
    let token = store.create(1);
    View3DWin::new(0x2000, token)
}

fn context() -> MouseContext {
    MouseContext {
        width: 1024,
        height: 768,
        edge_preference: true,
        tutorial_edge_allowed: true,
        fullscreen: true,
    }
}

fn controls() -> (View3DWin, Host) {
    let mut v = view();
    let mut h = Host::default();
    v.add_persistent_control(&mut h, 1);
    v.add_persistent_control(&mut h, 2);
    v.set_current_control(&mut h, Some(3), 0);
    v.set_current_control(&mut h, Some(4), 0);
    h.ops.clear();
    (v, h)
}

#[test]
fn vw_t1_qi_exact_interfaces_and_shared_count() {
    let mut v = view();
    for (iid, offset) in [
        (1, 0),
        (0xFA47_B3F9, 0),
        (0x4522_94AA, 4),
        (0xC6AE_7085, 8),
        (0x22BA_0121, 12),
    ] {
        assert_eq!(v.query_interface(iid), Some(0x2000 + offset));
    }
    assert_eq!(v.refs(), 6);
    assert_eq!(v.query_interface(0x5386_D516), None);
    assert_eq!(v.refs(), 6);
}

#[test]
fn vw_t7_and_t10_keys_broadcast_persistent_after_stack_taker() {
    let (mut v, mut h) = controls();
    h.takers = vec![4];
    assert!(v.process_key(&mut h, 42, 3, true));
    assert_eq!(h.events(), [4, 2, 1]);
    assert_eq!(v.owner_state().modifiers, 3);
    h.ops.clear();
    h.takers = vec![1];
    assert!(v.process_key(&mut h, 42, 2, false));
    assert_eq!(h.events(), [4, 3, 2, 1]);
    h.takers.clear();
    assert!(!v.process_key(&mut h, 42, 0, true));
}

#[test]
fn vw_t10_buttons_visit_persistent_before_stack_and_stop() {
    let (mut v, mut h) = controls();
    v.on_mouse(&mut h, InputEvent::DownL(40, 40, 0), context());
    assert_eq!(h.events(), [2, 1, 4, 3]);
    assert!(!h.capture);
    h.ops.clear();
    h.takers = vec![1];
    v.on_mouse(&mut h, InputEvent::DownL(40, 40, 0), context());
    assert_eq!(h.events(), [2, 1]);
    for pair in h.ops.chunks(3) {
        assert!(matches!(
            pair,
            [
                Op::Control(_, ControlCall::AddRef),
                Op::Control(_, ControlCall::Event(_)),
                Op::Control(_, ControlCall::Release)
            ]
        ));
    }
}

#[test]
fn vw_t8_right_click_drag_threshold_and_modifiers() {
    for (point, mods, jumps) in [((102, 103), 0, true), ((104, 100), 0, false), ((100, 100), 1, false)] {
        let mut v = view();
        let mut h = Host::default();
        v.on_mouse(&mut h, InputEvent::DownR(100, 100, 0), context());
        assert!(h.capture);
        v.on_mouse(&mut h, InputEvent::Move(point.0, point.1, mods), context());
        v.on_mouse(&mut h, InputEvent::UpR(point.0, point.1, mods), context());
        assert!(!h.capture);
        assert_eq!(h.ops.iter().any(|op| matches!(op, Op::Jump(_))), jumps);
    }
}

#[test]
fn vw_t9_zero_wheel_zooms_out_and_stack_can_suppress_everything() {
    let (mut v, mut h) = controls();
    v.on_mouse(&mut h, InputEvent::Wheel(10, 10, 2, 0), context());
    assert_eq!(h.events(), [4, 3, 2, 1]);
    assert!(h.ops.contains(&Op::View(ViewChange::ZoomOut)));
    h.ops.clear();
    h.takers = vec![4];
    v.on_mouse(&mut h, InputEvent::Wheel(10, 10, 0, 1), context());
    assert_eq!(h.events(), [4]);
    assert!(!h.ops.iter().any(|op| matches!(op, Op::View(_))));
}

#[test]
fn character_persistent_pass_has_stack_size_bound() {
    let (mut v, mut h) = controls();
    v.on_character(&mut h, 65);
    assert_eq!(h.events(), [4, 3, 2, 1]);
    v.remove_current_control(&mut h, true);
    h.ops.clear();
    v.on_character(&mut h, 65);
    assert_eq!(h.events(), [3]);
}

#[test]
fn tutorial_blocks_move_before_drag_and_rewrites_button_coordinates() {
    let (mut v, mut h) = controls();
    h.tutorial = false;
    v.on_mouse(&mut h, InputEvent::Move(100, 100, 4), context());
    assert!(h.ops.is_empty());
    assert!(!v.owner_state().dragged);
    assert_eq!(v.owner_state().modifiers, 4);
    h.tutorial = true;
    h.rewrite = Some((12, 13));
    v.on_mouse(&mut h, InputEvent::DownL(1, 2, 0), context());
    assert!(h
        .ops
        .contains(&Op::Control(2, ControlCall::Event(InputEvent::DownL(12, 13, 0)))));
}

#[test]
fn exit_calls_only_top_without_reference_and_broadcasts_persistent() {
    let (mut v, mut h) = controls();
    v.on_mouse(&mut h, InputEvent::Exit, context());
    assert_eq!(h.events(), [4, 2, 1]);
    assert!(h.ops.contains(&Op::Position(-10000, -10000)));
    assert!(!h.ops.contains(&Op::Control(4, ControlCall::AddRef)));
}

#[test]
fn control_modes_init_failure_and_persistent_lifetime() {
    let (mut v, mut h) = controls();
    h.init_fail = Some(5);
    v.set_current_control(&mut h, Some(5), 1);
    assert_eq!(v.input_control(), Some(3));
    assert_eq!(v.persistent_controls(), 2);
    assert!(!h.ops.contains(&Op::Control(5, ControlCall::AddRef)));
    assert!(h.ops.contains(&Op::Control(4, ControlCall::Shutdown)));
    assert!(h.ops.contains(&Op::Control(3, ControlCall::Deactivate)));
    v.set_current_control(&mut h, None, 2);
    assert_eq!(v.input_control(), None);
    assert_eq!(v.persistent_controls(), 2);
    assert!(v.remove_persistent_control(&mut h, 1));
    assert!(!v.remove_persistent_control(&mut h, 1));
    v.remove_all_persistent_controls(&mut h);
    assert_eq!(v.persistent_controls(), 0);
}

#[test]
fn remove_control_sets_default_cursor_and_releases_capture() {
    let mut v = view();
    let mut h = Host::default();
    v.set_current_control(&mut h, Some(1), 0);
    h.capture = true;
    h.ops.clear();
    assert!(v.remove_current_control(&mut h, false));
    assert_eq!(
        h.ops,
        [
            Op::Capture(false),
            Op::Control(1, ControlCall::Deactivate),
            Op::Control(1, ControlCall::Shutdown),
            Op::Control(1, ControlCall::Release),
            Op::Cursor(0xC16C_4E1B)
        ]
    );
    assert!(!v.remove_current_control(&mut h, false));
}

#[test]
fn vw_t11_t12_keyboard_cannot_add_diagonal_and_any_stop_stops_both() {
    let mut v = view();
    let mut h = Host::default();
    v.keyboard_scroll(&mut h, [-1, 0], true);
    v.keyboard_scroll(&mut h, [0, -1], true);
    let m = &v.owner_state().motion;
    assert_eq!(m.keyboard, [-1, 0]);
    assert_eq!(m.angle, std::f32::consts::PI);
    assert!((m.speed - 0.3456).abs() < 0.000_001);
    v.keyboard_scroll(&mut h, [0, 0], true);
    assert!(v.owner_state().motion.stopping);
}

#[test]
fn edge_recalculation_forgets_edge_and_keyboard_replaces_axis() {
    let mut v = view();
    let mut h = Host::default();
    v.motion_mut().edge = [1, 1];
    v.motion_mut().keyboard = [-1, 0];
    v.recalc_scroll_velocity(&mut h, true);
    assert_eq!(v.owner_state().motion.edge, [0, 0]);
    assert!((v.owner_state().motion.angle - 3.0 * std::f32::consts::PI / 4.0).abs() < 0.000_001);
}

#[test]
fn vw_t13_zoom_limits_compare_against_unclamped_maximum() {
    let mut v = view();
    let mut h = Host {
        zoom: 5,
        ..Host::default()
    };
    v.set_zoom_limits(&mut h, 0, 9, Some(false));
    assert_eq!(v.owner_state().motion.zoom_max, 4);
    assert!(h.ops.is_empty());
    v.set_zoom(&mut h, 99);
    assert!(h.ops.contains(&Op::View(ViewChange::Zoom(4))));
}

#[test]
fn paint_gates_preserve_tail_and_write_time_before_draw() {
    let mut v = view();
    let mut h = Host::default();
    assert!(!v.paint(&mut h, 10, 0));
    assert_eq!(h.ops, [Op::Tail]);
    h.ops.clear();
    v.motion_mut().paint_enabled = true;
    v.motion_mut().paint_period_us = 100.0;
    assert!(!v.paint(&mut h, 99, 0));
    assert!(h.ops.is_empty());
    assert!(v.paint(&mut h, 1_000_000, 0));
    assert_eq!(
        h.ops,
        [Op::ResetPaint, Op::Cost(0.5), Op::Time(7.0), Op::Draw, Op::Tail]
    );
    h.ops.clear();
    v.motion_mut().city_initializing = true;
    v.paint(&mut h, 250_000, 0);
    assert!(h.ops.contains(&Op::Time(0.25)));
}

#[test]
fn scroll_decay_and_hidden_pause_restart_do_not_resume_in_same_paint() {
    let mut v = view();
    let mut h = Host {
        moved: true,
        ..Host::default()
    };
    v.motion_mut().paint_enabled = true;
    v.motion_mut().reduced_render_settings = true;
    v.set_scrolling(&mut h, true, 1.0, 2.0);
    v.scroll_stop();
    v.paint(&mut h, 100_000, 5000);
    assert!(h.ops.contains(&Op::Scroll(1.0, 5.0)));
    assert!(!h.ops.contains(&Op::Resume));
    v.paint(&mut h, 100_000, 5000);
    assert!(!v.owner_state().motion.scrolling);
    assert!(h.ops.contains(&Op::End));
    assert_eq!(h.ops.iter().filter(|op| **op == Op::Pause).count(), 1);
    h.moved = false;
    v.paint(&mut h, 100_000, 2000);
    assert!(!h.ops.contains(&Op::Resume));
    v.paint(&mut h, 100_000, 2001);
    assert!(h.ops.contains(&Op::Resume));
}

#[test]
fn failed_view_change_does_not_pause_and_no_simulator_does_not_own_pause() {
    let mut v = view();
    let mut h = Host {
        changed: false,
        ..Host::default()
    };
    assert!(!v.change_view(&mut h, ViewChange::RotateLeft));
    assert_eq!(h.ops, [Op::View(ViewChange::RotateLeft)]);
    h.simulator = false;
    v.temporary_hidden_pause(&mut h);
    assert!(!v.owner_state().motion.hidden_paused);
    assert!(!h.ops.contains(&Op::Pause));
}
