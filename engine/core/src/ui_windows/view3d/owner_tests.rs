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
    Send(u32, u32, u32),
    HoldFilter(u32),
    ReleaseFilter(u32),
    CreateFilter([bool; 3]),
    Size(f32),
    Property(String, String),
    Recorder(bool),
    Contour(bool, u32),
    CostDisplay(CostDisplay),
    Clock(u32, u32, u32),
    Rating(i32, i32),
    Alert(Option<u32>),
    Speed(u32, bool, bool),
    Mission(String, bool),
    Panel(u32, bool),
    Tab(u32, bool),
    Close(bool),
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
    recorder_exists: bool,
    filter_allocation: bool,
    tabs: std::collections::BTreeMap<u32, bool>,
    established: bool,
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
            recorder_exists: false,
            filter_allocation: true,
            tabs: std::collections::BTreeMap::new(),
            established: true,
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
                ControlCall::CanStack => true,
                _ => false,
            },
            cursor: (call == ControlCall::Cursor).then_some(id + 100),
            tool_id: (call == ControlCall::ToolId).then_some(0xC7AF_928E),
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

impl StateHost for Host {
    fn send(&mut self, m: u32, a: u32, b: u32) {
        self.ops.push(Op::Send(m, a, b));
    }
    fn hold_filter(&mut self, t: u32) {
        self.ops.push(Op::HoldFilter(t));
    }
    fn release_filter(&mut self, t: u32) {
        self.ops.push(Op::ReleaseFilter(t));
    }
    fn add_contour_layer(&mut self, t: u32) -> Option<i32> {
        self.ops.push(Op::Contour(true, t));
        Some(42)
    }
    fn remove_contour_layer(&mut self, id: i32) {
        self.ops.push(Op::Contour(false, id as u32));
    }
}

impl CheatHost for Host {
    fn create_cheat_control(&mut self, _: CheatTool) -> Option<u32> {
        self.scroll_control
    }
    fn create_realistic_filter(&mut self, flags: [bool; 3]) -> Option<FilterInfo> {
        self.ops.push(Op::CreateFilter(flags));
        self.filter_allocation.then_some(FilterInfo {
            token: 9,
            realistic_flags: Some(flags),
        })
    }
    fn camera_size(&mut self, size: f32) {
        self.ops.push(Op::Size(size));
    }
    fn render_property(&mut self, name: &str, value: &str) -> bool {
        self.ops.push(Op::Property(name.into(), value.into()));
        self.changed
    }
    fn post_render_property_change(&mut self, m: u32) {
        self.ops.push(Op::Message(m));
    }
    fn recorder_child_exists(&self) -> bool {
        self.recorder_exists
    }
    fn create_recorder_child(&mut self, id: u32, path: &str) {
        assert_eq!(id, 0xA802_B4EB);
        assert_eq!(path, "RecordedAnimations/");
        self.recorder_exists = true;
        self.ops.push(Op::Recorder(true));
    }
    fn remove_recorder_child(&mut self, id: u32) {
        assert_eq!(id, 0xA802_B4EB);
        self.recorder_exists = false;
        self.ops.push(Op::Recorder(false));
    }
}

#[test]
fn vw_t5_sizeof_clamps_arguments_and_needs_one() {
    let mut v = view();
    let mut h = Host::default();
    for arg in ["100", "0.5", "abc", "5", "12junk", "2e1tail", "3e+"] {
        assert!(v.process_cheat(&mut h, 0xCA7B_C5B5, &["SizeOf", arg, "ignored"]));
    }
    assert!(!v.process_cheat(&mut h, 0xCA7B_C5B5, &["SizeOf"]));
    assert_eq!(
        h.ops,
        [
            Op::Size(64.0),
            Op::Size(0.9),
            Op::Size(0.9),
            Op::Size(5.0),
            Op::Size(12.0),
            Op::Size(20.0),
            Op::Size(3.0)
        ]
    );
}

#[test]
fn vw_t6_filter_cheat_false_results_and_terrain_query() {
    let mut v = view();
    let mut h = Host::default();
    v.set_filter(
        &mut h,
        Some(FilterInfo {
            token: 1,
            realistic_flags: Some([true, true, true]),
        }),
    );
    assert!(!v.process_cheat(&mut h, 0xAA7A_2746, &["TastyZots"]));
    assert_eq!(
        v.owner_state().view.filter.unwrap().realistic_flags,
        Some([true, false, true])
    );
    assert!(!v.process_cheat(&mut h, 0xAA7A_2746, &["TastyZots"]));
    assert_eq!(
        v.owner_state().view.filter.unwrap().realistic_flags,
        Some([true, true, true])
    );
    assert!(!v.process_cheat(&mut h, 0x6A7A_276E, &["Zoneria"]));
    assert_eq!(
        v.owner_state().view.filter.unwrap().realistic_flags,
        Some([false, true, false])
    );
    assert!(v.process_cheat(&mut h, 0x8AB7_30AE, &["TerrainQuery", "OFF"]));
    assert!(!v.owner_state().view.terrain_query);
    assert!(v.process_cheat(&mut h, 0x8AB7_30AE, &["TerrainQuery"]));
    assert!(v.owner_state().view.terrain_query);
}

#[test]
fn filter_messages_and_reference_order_are_synchronous() {
    let mut v = view();
    let mut h = Host::default();
    v.set_filter(
        &mut h,
        Some(FilterInfo {
            token: 1,
            realistic_flags: None,
        }),
    );
    h.ops.clear();
    v.set_filter(
        &mut h,
        Some(FilterInfo {
            token: 2,
            realistic_flags: None,
        }),
    );
    assert_eq!(
        h.ops,
        [
            Op::Send(0x06CD_B65A, 1, 2),
            Op::HoldFilter(2),
            Op::ReleaseFilter(1),
            Op::Send(0x06CD_B65B, 0, 0)
        ]
    );
    h.ops.clear();
    assert_eq!(v.get_filter(&mut h).unwrap().token, 2);
    assert_eq!(h.ops, [Op::HoldFilter(2)]);
    h.filter_allocation = false;
    assert!(!v.process_cheat(&mut h, 0xAA7A_2746, &["TastyZots"]));
    assert_eq!(v.owner_state().view.filter.unwrap().token, 2);
}

#[test]
fn render_cheat_recorder_and_debug_flags_keep_their_quirks() {
    let mut v = view();
    let mut h = Host::default();
    assert!(!v.process_cheat(&mut h, 0x88F3_D4BB, &["RP"]));
    assert!(h.ops.is_empty());
    assert!(v.process_cheat(&mut h, 0x88F3_D4BC, &["RP", "property"]));
    assert_eq!(
        h.ops,
        [Op::Property("property".into(), "".into()), Op::Message(0xC9F7_75BB)]
    );
    h.ops.clear();
    h.changed = false;
    assert!(v.process_cheat(&mut h, 0x88F3_D4BB, &["RenderProp", "unknown", "1"]));
    assert_eq!(h.ops, [Op::Property("unknown".into(), "1".into())]);
    h.ops.clear();
    v.recorder(&mut h, true);
    v.recorder(&mut h, true);
    v.recorder(&mut h, false);
    v.recorder(&mut h, false);
    assert_eq!(h.ops, [Op::Recorder(true), Op::Recorder(false)]);
    assert!(v.process_cheat(&mut h, 0x4A97_2F37, &["WatchMeMove"]));
    assert!(v.process_cheat(&mut h, 0x4A82_253E, &["You don't deserve it"]));
    assert!(v.owner_state().view.animation_debug);
    assert!(v.owner_state().view.all_rewards);
    assert!(!v.process_cheat(&mut h, 0, &["unknown"]));
}

#[test]
fn cheat_control_allocation_failure_still_replaces_stack() {
    let (mut v, mut h) = controls();
    assert!(v.process_cheat(&mut h, 0x8973_A69C, &["GOL"]));
    assert_eq!(v.input_control(), None);
    assert_eq!(v.persistent_controls(), 2);
    h.scroll_control = Some(7);
    assert!(v.process_cheat(&mut h, 0x4AC0_96CA, &["BuildingPlop"]));
    assert_eq!(v.input_control(), Some(7));
    assert_eq!(h.ops.last(), Some(&Op::Control(7, ControlCall::Release)));
}

#[test]
fn vw_t16_trigger_transitions_and_four_reference_save_roundtrip() {
    let mut v = view();
    let mut h = Host::default();
    v.assert_global_trigger(&mut h, 123);
    v.assert_global_trigger(&mut h, 123);
    v.retract_global_trigger(&mut h, 123);
    v.retract_global_trigger(&mut h, 123);
    v.retract_global_trigger(&mut h, 123);
    assert_eq!(h.ops, [Op::Send(0xAA37_5FE8, 123, 1), Op::Send(0xAA37_5FE8, 123, 0)]);
    assert!(!v.global_trigger_state(123));
    v.assert_global_trigger(&mut h, 123);
    v.assert_global_trigger(&mut h, 123);
    let refs = [0xC9C6_28EC, 0xE9C6_22D8, 0xA9C7_41B3, 0x8695_664E].map(SaveRef::of);
    v.view_state_mut().references = refs;
    let record = v.save_record();
    assert_eq!(record.triggers, [(123, 2)]);
    let mut loaded = view();
    loaded.load_record(&record).unwrap();
    assert_eq!(loaded.owner_state().view.references, refs);
    assert!(!loaded.owner_state().view.not_loaded);
    assert_eq!(loaded.save_record(), record);
    for version in 0..5 {
        let mut older = record.clone();
        older.header.version = version;
        assert!(loaded.load_record(&older).is_ok());
    }
    let mut newer = record;
    newer.header.version = 5;
    assert!(loaded.load_record(&newer).is_err());
}

#[test]
fn level_only_notifies_changes_and_contour_pair_removes_returned_layer() {
    let mut v = view();
    let mut h = Host::default();
    v.set_view_level(&mut h, 1);
    assert!(h.ops.is_empty());
    v.set_view_level(&mut h, 3);
    assert_eq!(h.ops, [Op::Send(0xC6CD_BF1A, 0, 0), Op::Send(0xC6CD_BF1B, 0, 0)]);
    h.ops.clear();
    v.toggle_contours(&mut h);
    v.toggle_contours(&mut h);
    assert_eq!(h.ops, [Op::Contour(true, 0x07F9_4205), Op::Contour(false, 42)]);
    assert_eq!(v.owner_state().view.contour_layer, None);
}

impl StatusHost for Host {
    fn funds(&self) -> i32 {
        100
    }
    fn draw_cost(&mut self, display: CostDisplay) {
        self.ops.push(Op::CostDisplay(display));
    }
    fn cursor_callout(&mut self, _: Option<&CursorText>, _: (i32, i32)) {}
    fn project(&mut self, point: [f32; 3]) -> (i32, i32) {
        (point[0] as i32, point[2] as i32)
    }
    fn clock_time(&self) -> (u32, u32) {
        (3, 15)
    }
    fn clock_widgets(&mut self, h: u32, m: u32, f: u32) {
        self.ops.push(Op::Clock(h, m, f));
    }
    fn clock_hover(&mut self) {}
    fn rating_widgets(&mut self, col: i32, delta: i32) {
        self.ops.push(Op::Rating(col, delta));
    }
    fn alert_picture(&mut self, picture: Option<u32>) {
        self.ops.push(Op::Alert(picture));
    }
    fn speed_button(&mut self, id: u32, toggle: bool, lower: bool) {
        self.ops.push(Op::Speed(id, toggle, lower));
    }
    fn create_mission_panel(&mut self, id: u32, margin: i32) -> Option<u32> {
        assert_eq!(margin, 8);
        Some(id)
    }
    fn delete_mission_panel(&mut self, _: u32) {}
    fn mission_text(&mut self, _: u32, time: &str, red: bool, _: &str) {
        self.ops.push(Op::Mission(time.into(), red));
    }
}

#[test]
fn vw_t17_clock_frame_and_out_of_range_hour_fallback() {
    let v = view();
    let mut h = Host::default();
    v.update_24hour_clock(&mut h, 13, 45);
    v.update_24hour_clock(&mut h, 24, 59);
    assert_eq!(h.ops, [Op::Clock(13, 45, 27), Op::Clock(3, 15, 6)]);
}

#[test]
fn vw_t18_cost_red_when_above_funds_fades_frozen_for_1500ms() {
    let mut v = view();
    let mut h = Host::default();
    v.update_cursor_position(&mut h, 100, 200);
    v.set_cost_indicator(&mut h, -200, 101);
    let cost = v.owner_state().display.cost;
    assert_eq!(cost.colour, [255, 40, 40]);
    assert_eq!(cost.alpha, 255);
    assert_eq!(cost.position, (85, 185));
    let old_len = h.ops.len();
    v.set_cost_indicator(&mut h, -200, 101);
    assert_eq!(h.ops.len(), old_len);
    v.clear_cost_indicator(&mut h, true);
    v.update_cursor_position(&mut h, 300, 400);
    v.update_cost_indicator(&mut h, 0.75);
    let halfway = v.owner_state().display.cost;
    assert_eq!(halfway.mode, CostMode::Fading);
    assert_eq!(halfway.position, (85, 185));
    assert_eq!(halfway.colour, [255; 3]);
    assert!(halfway.alpha < cost.alpha);
    v.update_cost_indicator(&mut h, 0.75);
    assert_eq!(v.owner_state().display.cost.mode, CostMode::Hidden);
    assert_eq!(v.owner_state().display.cost.alpha, 0);
    v.set_cost_amount(&mut h, 100);
    assert_eq!(v.owner_state().display.cost.alpha, 150);
}

#[test]
fn rating_clamps_and_alert_priority_disaster_situation_pause() {
    let mut v = view();
    let mut h = Host::default();
    v.update_mayor_rating(&mut h, 127, 100);
    v.update_mayor_rating(&mut h, -128, 100);
    v.update_mayor_rating(&mut h, 0, 0);
    assert_eq!(h.ops, [Op::Rating(99, 21), Op::Rating(0, -21)]);
    h.ops.clear();
    v.update_alert_border(&mut h, true, true, true);
    v.update_alert_border(&mut h, false, true, true);
    v.update_alert_border(&mut h, false, false, true);
    v.update_alert_border(&mut h, false, false, false);
    assert_eq!(
        h.ops,
        [
            Op::Alert(Some(0x1431_5E60)),
            Op::Alert(Some(0x1431_5E62)),
            Op::Alert(Some(0x1431_5E61)),
            Op::Alert(None)
        ]
    );
}

#[test]
fn mission_warning_thresholds_and_speed_paused_picture_half() {
    let mut v = view();
    let mut h = Host::default();
    v.show_mission_status_panel(&mut h, 42);
    v.show_mission_status_panel(&mut h, 43);
    assert_eq!(v.owner_state().display.mission_panel, Some(42));
    for seconds in [60, 59, 21, 20, 19, 0] {
        v.update_mission_status_panel(&mut h, seconds, "target");
    }
    assert_eq!(
        h.ops,
        [
            Op::Mission("01:00".into(), false),
            Op::Mission("00:59".into(), true),
            Op::Mission("00:21".into(), true),
            Op::Mission("00:20".into(), false),
            Op::Mission("00:19".into(), true),
            Op::Mission("00:00".into(), false)
        ]
    );
    v.hide_mission_status_panel(&mut h);
    assert_eq!(v.owner_state().display.mission_panel, None);
    h.ops.clear();
    v.update_speed_buttons(&mut h, 1, true);
    assert_eq!(
        h.ops,
        [
            Op::Speed(0x6A4F_BB31, false, true),
            Op::Speed(0x4A4F_BB60, true, true),
            Op::Speed(0x8998_BBDF, false, true)
        ]
    );
}

impl CommandHost for Host {
    fn city_established(&self) -> bool {
        self.established
    }
    fn edge_scrolling_preference(&self) -> bool {
        true
    }
    fn simulator_paused(&self) -> bool {
        false
    }
    fn pause_simulator(&mut self) {}
    fn resume_simulator(&mut self) {}
    fn set_sim_speed(&mut self, _: u32) {}
    fn refresh_speed_buttons(&mut self) {}
    fn close_secondary(&mut self) {
        self.ops.push(Op::Close(false));
    }
    fn close_tertiary(&mut self) {
        self.ops.push(Op::Close(true));
    }
    fn tab_toggled(&self, id: u32) -> bool {
        self.tabs.get(&id).copied().unwrap_or(false)
    }
    fn toggle_tab(&mut self, id: u32, on: bool) {
        self.tabs.insert(id, on);
        self.ops.push(Op::Tab(id, on));
    }
    fn panel_buttons(&mut self, id: u32, on: bool) {
        self.ops.push(Op::Panel(id, on));
    }
    fn mayor_subpanel(&mut self, _: u32, _: bool) {}
    fn mysim_visible(&mut self, _: bool) {}
    fn advisor_visible(&mut self, _: bool) {}
    fn news_ticker_visible(&mut self, _: bool) {}
    fn control_panel_visible(&mut self, _: bool) {}
    fn restore_button_visible(&mut self, _: bool) {}
    fn auxiliary_ui_visible(&mut self, _: bool) {}
    fn rotation_ui_visible(&mut self, _: bool) {}
    fn create_query_tool(&mut self, _: QueryTool) -> Option<u32> {
        Some(7)
    }
}

#[test]
fn vw_t14_god_panel_override_and_toggle_close() {
    for (mods, established, expected) in [(0, true, 4), (7, true, 0), (0, false, 0)] {
        let mut v = view();
        let mut h = Host {
            established,
            ..Host::default()
        };
        v.owner.modifiers = mods;
        assert!(!v.process_motion_panel_command(&mut h, 0x6A93_5C34));
        assert_eq!(v.owner_state().panels.current, Some(expected));
        assert!(h.ops.contains(&Op::Panel(expected, true)));
        assert!(!v.process_motion_panel_command(&mut h, 0x6A93_5C34));
        assert_eq!(v.owner_state().panels.current, None);
        assert!(h.ops.contains(&Op::Panel(expected, false)));
    }
}

#[test]
fn vw_t15_query_tool_twice_is_one_tool_and_cancel_closes_both_menus() {
    let mut v = view();
    let mut h = Host::default();
    assert!(!v.process_motion_panel_command(&mut h, 0x6A93_5CF3));
    assert!(!v.process_motion_panel_command(&mut h, 0x6A93_5CF3));
    assert_eq!(v.owner.stack, [7]);
    assert_eq!(
        h.ops
            .iter()
            .filter(|op| **op == Op::Control(7, ControlCall::Init))
            .count(),
        1
    );
    h.ops.clear();
    assert!(!v.process_motion_panel_command(&mut h, 0x6A93_5CF1));
    assert!(v.owner.stack.is_empty());
    assert_eq!(&h.ops[h.ops.len() - 2..], &[Op::Close(true), Op::Close(false)]);
    h.established = false;
    v.process_motion_panel_command(&mut h, 0x6A93_5CF3);
    assert!(v.owner.stack.is_empty());
}

#[test]
fn rotate_cw_is_left_and_stop_command_stops_any_axis() {
    let mut v = view();
    let mut h = Host::default();
    v.process_motion_panel_command(&mut h, 0x6A93_5CC9);
    v.process_motion_panel_command(&mut h, 0x6A93_5CCC);
    assert!(h.ops.contains(&Op::View(ViewChange::RotateLeft)));
    assert!(h.ops.contains(&Op::View(ViewChange::RotateRight)));
    v.process_motion_panel_command(&mut h, 0x6A93_5CD8);
    v.process_motion_panel_command(&mut h, 0x6A93_5CDD);
    assert_eq!(v.owner_state().motion.keyboard, [-1, 0]);
    v.process_motion_panel_command(&mut h, 0x2A94_8272);
    assert_eq!(v.owner_state().motion.keyboard, [0, 0]);
    assert!(v.owner_state().motion.stopping);
}
