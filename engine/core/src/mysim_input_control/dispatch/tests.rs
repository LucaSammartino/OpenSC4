// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
struct Pylon {
    owner: Option<Handle>,
    slot7: u32,
    dragging: bool,
    position: [f32; 3],
    state: u32,
}
struct Fake {
    refs: BTreeMap<Handle, i32>,
    pylons: BTreeMap<Handle, Pylon>,
    sim_pylons: BTreeMap<Handle, Handle>,
    hits: Vec<Handle>,
    calls: Vec<String>,
    posts: Vec<(u32, bool, Handle)>,
    running: bool,
    elapsed: u32,
    timers: i32,
    view: bool,
    location: bool,
    active: bool,
    slot_gap: bool,
}
impl Default for Fake {
    fn default() -> Self {
        Self {
            refs: BTreeMap::new(),
            pylons: [(
                10,
                Pylon {
                    owner: Some(1),
                    slot7: 1,
                    dragging: false,
                    position: [10.0, 2.0, 20.0],
                    state: 2,
                },
            )]
            .into(),
            sim_pylons: [(1, 10)].into(),
            hits: vec![10],
            calls: Vec::new(),
            posts: Vec::new(),
            running: false,
            elapsed: 0,
            timers: 0,
            view: true,
            location: true,
            active: true,
            slot_gap: false,
        }
    }
}
impl WindowHost for Fake {}
impl InputServices for Fake {
    fn add_ref(&mut self, object: Handle) {
        *self.refs.entry(object).or_default() += 1;
    }
    fn release(&mut self, object: Handle) {
        let count = self.refs.get_mut(&object).expect("retained before release");
        *count -= 1;
        assert!(*count >= 0);
    }
    fn base_init(&mut self) -> bool {
        self.calls.push("init".into());
        true
    }
    fn base_shutdown(&mut self) {
        self.calls.push("shutdown".into());
    }
    fn base_destroy(&mut self) {
        self.calls.push("destroy".into());
    }
    fn set_cursor(&mut self, cursor: u32) {
        self.calls.push(format!("cursor:{cursor:x}"));
    }
    fn end_input(&mut self) {
        self.calls.push("end".into());
    }
    fn is_current_control(&mut self) -> bool {
        true
    }
    fn view_window(&mut self) -> Option<Window> {
        self.view.then_some(99)
    }
    fn main_window(&mut self) -> Option<Window> {
        Some(99)
    }
    fn focus_view(&mut self) {}
}
impl DispatchServices for Fake {
    fn set_control_id(&mut self, id: u32) {
        assert_eq!(id, 0xebab_04a9);
    }
    fn subscribe(&mut self, id: u32, subscribe: bool) {
        self.calls.push(format!("subscribe:{id:x}:{subscribe}"));
    }
    fn clear_cursor_text(&mut self, key: u32) {
        assert_eq!(key, CURSOR_TEXT);
        self.calls.push("clear-text".into());
    }
    fn remove_all_controls(&mut self, argument: bool) {
        assert!(!argument);
        self.calls.push("remove-all".into());
    }
    fn key_is_passed(&mut self, key: u32) -> bool {
        self.calls.push(format!("key:{key}"));
        true
    }
    fn create_timer(&mut self, units: u32) -> Handle {
        assert_eq!(units, 4);
        self.timers += 1;
        1000
    }
    fn timer_units(&mut self, _: Handle, units: u32) {
        assert_eq!(units, 4);
    }
    fn timer_running(&mut self, _: Handle) -> bool {
        self.running
    }
    fn timer_elapsed(&mut self, _: Handle) -> u32 {
        self.elapsed
    }
    fn timer_restart(&mut self, _: Handle) {
        self.running = true;
        self.elapsed = 0;
    }
    fn timer_reset(&mut self, _: Handle) {
        self.running = false;
    }
    fn destroy_timer(&mut self, _: Handle) {
        self.timers -= 1;
    }
    fn create_filter(&mut self) -> Handle {
        self.add_ref(2000);
        2000
    }
    fn sim_active(&mut self, _: Handle) -> bool {
        self.active
    }
    fn sim_pylon(&mut self, sim: Handle) -> Option<Handle> {
        self.sim_pylons.get(&sim).copied()
    }
    fn create_sim_pylon(&mut self, sim: Handle) -> Option<Handle> {
        self.pylons.insert(
            12,
            Pylon {
                owner: Some(sim),
                slot7: 1,
                dragging: false,
                position: [0.0; 3],
                state: 0,
            },
        );
        self.sim_pylons.insert(sim, 12);
        Some(12)
    }
    fn dispatch_tool_end(&mut self, sim: Handle) {
        self.calls.push(format!("end-sim:{sim}"));
    }
    fn lock_pylon(&mut self, sim: Handle, locked: bool) {
        self.calls.push(format!("lock:{sim}:{locked}"));
    }
    fn test_location(&mut self, _: Handle, _: f32, _: f32) -> bool {
        self.calls.push("location".into());
        self.location
    }
    fn pylon_interface(&mut self, occupant: Handle) -> Option<Handle> {
        self.pylons.contains_key(&occupant).then_some(occupant)
    }
    fn occupant_type(&mut self, occupant: Handle) -> u32 {
        if self.pylons.contains_key(&occupant) {
            0xa9b6_d7de
        } else {
            0
        }
    }
    fn pylon_slot7(&mut self, pylon: Handle) -> Result<u32> {
        if self.slot_gap {
            Err(Unsupported::PylonSlotsMst3)
        } else {
            Ok(self.pylons[&pylon].slot7)
        }
    }
    fn pylon_slot20_sim(&mut self, pylon: Handle) -> Result<Option<Handle>> {
        if self.slot_gap {
            Err(Unsupported::PylonSlotsMst3)
        } else {
            Ok(self.pylons[&pylon].owner)
        }
    }
    fn drag_state(&mut self, pylon: Handle) -> bool {
        self.pylons[&pylon].dragging
    }
    fn set_drag_state(&mut self, pylon: Handle, dragging: bool) {
        self.pylons.get_mut(&pylon).unwrap().dragging = dragging;
    }
    fn dispatch_state(&mut self, pylon: Handle) -> u32 {
        self.pylons[&pylon].state
    }
    fn set_dispatch_state(&mut self, pylon: Handle, state: u32) {
        self.pylons.get_mut(&pylon).unwrap().state = state;
    }
    fn position(&mut self, pylon: Handle) -> [f32; 3] {
        self.pylons[&pylon].position
    }
    fn set_position(&mut self, pylon: Handle, position: [f32; 3]) {
        self.pylons.get_mut(&pylon).unwrap().position = position;
    }
    fn set_ui_state(&mut self, pylon: Handle, state: u32) {
        self.calls.push(format!("ui:{pylon}:{state}"));
    }
    fn renderer_pick(
        &mut self,
        _: i32,
        _: i32,
        mut filter: impl FnMut(&mut Self, Handle) -> Result<bool>,
    ) -> Result<Option<Handle>> {
        for hit in self.hits.clone() {
            if filter(self, hit)? {
                return Ok(Some(hit));
            }
        }
        Ok(None)
    }
    fn pick_terrain(&mut self, x: i32, z: i32) -> Option<[f32; 3]> {
        self.calls.push(format!("terrain:{x}:{z}"));
        Some([x as f32, 999.0, z as f32])
    }
    fn altitude(&mut self, _: f32, _: f32) -> f32 {
        7.0
    }
    fn world_to_screen(&mut self, position: [f32; 3]) -> [f32; 2] {
        [position[0] + 0.75, position[2] - 0.75]
    }
    fn post_selection(&mut self, id: u32, selected: bool, sim: Handle) {
        self.posts.push((id, selected, sim));
    }
    fn play_result_sound(&mut self, result: u32) {
        assert_eq!(result, 0);
        self.calls.push("sound:0".into());
    }
}

fn setup() -> (Dispatch, Fake) {
    let mut host = Fake::default();
    let mut tool = Dispatch::new(50, Some(1), &mut host);
    tool.init(&mut host).unwrap();
    (tool, host)
}
fn finish(mut tool: Dispatch, mut host: Fake) {
    tool.set_hover_pylon(None, &mut host);
    tool.shutdown(&mut host);
    tool.destroy(&mut host);
    assert!(host.refs.values().all(|count| *count == 0));
    assert_eq!(host.timers, 0);
}

#[test]
fn mt_c6_clicks_three_hundred_apart_lock_and_post_double_click() {
    let (mut tool, mut host) = setup();
    assert_eq!(tool.on_mouse_down_l(10, 20, &mut host), Ok(true));
    assert!(host.pylons[&10].dragging);
    assert_eq!(host.posts, [(PYLON_MESSAGE, false, 1)]);
    host.elapsed = 300;
    assert_eq!(tool.on_mouse_down_l(10, 20, &mut host), Ok(true));
    assert!(!host.pylons[&10].dragging);
    assert!(!host.running);
    assert_eq!(host.posts[1], (PYLON_MESSAGE, true, 1));
    assert!(host.calls.contains(&"lock:1:true".into()));
    finish(tool, host);
}

#[test]
fn mt_c7_tool_takeover_restores_drag_and_exempt_tool_does_not_suspend() {
    let (mut tool, mut host) = setup();
    tool.on_mouse_down_l(8, 18, &mut host).unwrap();
    assert_eq!(tool.drag_offset(), [2, 1]);
    host.set_position(10, [55.0, 7.0, 66.0]);
    host.set_dispatch_state(10, 3);
    let this = Some(Control {
        identity: 50,
        id: 0xebab_04a9,
    });
    let exempt = Some(Control {
        identity: 51,
        id: 0x48e2_a1f8,
    });
    assert!(tool.do_message(0x09c8_b8ea, this, exempt, None, &mut host));
    assert!(tool.active());
    assert!(tool.do_message(0x09c8_b8ea, this, None, None, &mut host));
    assert!(!tool.active());
    assert_eq!(host.pylons[&10].position, [10.0, 2.0, 20.0]);
    assert_eq!(host.pylons[&10].state, 2);
    assert!(tool.do_message(0x09c8_b8ea, None, this, None, &mut host));
    assert!(tool.active());
    assert!(!tool.on_key_down(27, &mut host));
    host.active = false;
    assert!(!tool.do_message(0xaa25_827c, None, None, None, &mut host));
    assert!(host.calls.contains(&"remove-all".into()));
    finish(tool, host);
}

#[test]
fn dragging_waits_five_hundred_then_uses_altitude_and_location_test() {
    let (mut tool, mut host) = setup();
    tool.on_mouse_down_l(8, 18, &mut host).unwrap();
    host.elapsed = 499;
    tool.on_mouse_move(30, 40, &mut host).unwrap();
    assert_eq!(host.pylons[&10].position, [10.0, 2.0, 20.0]);
    host.elapsed = 500;
    tool.on_mouse_move(30, 40, &mut host).unwrap();
    assert_eq!(host.pylons[&10].position, [32.0, 7.0, 41.0]);
    assert_eq!(host.pylons[&10].state, 3);
    assert!(tool.last_location_test());
    host.location = false;
    tool.on_mouse_move(50, 60, &mut host).unwrap();
    assert_eq!(host.pylons[&10].position, [32.0, 7.0, 41.0]);
    assert_eq!(host.pylons[&10].state, 1);
    assert!(!tool.last_location_test());
    // The click release deliberately does not test or overwrite terrain y.
    tool.on_mouse_down_l(70, 80, &mut host).unwrap();
    assert_eq!(host.pylons[&10].position, [72.0, 999.0, 81.0]);
    assert!(!host.pylons[&10].dragging);
    finish(tool, host);
}

#[test]
fn filter_raw_slot_boundary_and_hover_double_holds() {
    let (mut tool, mut host) = setup();
    assert_eq!(PylonFilter::includes(10, &mut host), Ok(true));
    host.slot_gap = true;
    assert_eq!(tool.pick_pylon(0, 0, &mut host), Err(Unsupported::PylonSlotsMst3));
    host.slot_gap = false;
    tool.on_mouse_move(10, 20, &mut host).unwrap();
    assert_eq!(tool.hovered(), Some(10));
    assert_eq!(host.refs[&10], 2);
    tool.set_hover_pylon(Some(10), &mut host);
    assert_eq!(host.refs[&10], 2);
    host.hits.clear();
    tool.on_mouse_move(10, 20, &mut host).unwrap();
    assert_eq!(tool.hovered(), None);
    assert_eq!(host.refs[&10], 0);
    assert!(host.calls.contains(&"cursor:2bac4970".into()));
    finish(tool, host);
}

#[test]
fn shutdown_restores_pylon_and_reports_the_specified_hover_leak() {
    let (mut tool, mut host) = setup();
    let subscriptions = host.calls.iter().filter(|s| s.starts_with("subscribe:")).count();
    tool.init(&mut host).unwrap();
    assert_eq!(
        host.calls.iter().filter(|s| s.starts_with("subscribe:")).count(),
        subscriptions
    );
    tool.set_hover_pylon(Some(10), &mut host);
    tool.on_mouse_down_l(10, 20, &mut host).unwrap();
    host.set_position(10, [50.0, 7.0, 60.0]);
    tool.shutdown(&mut host);
    assert_eq!(host.pylons[&10].position, [10.0, 2.0, 20.0]);
    assert!(!host.pylons[&10].dragging);
    assert_eq!(tool.leaked_hover_references(), 1);
    tool.destroy(&mut host);
    assert_eq!(host.refs[&10], 1); // §5.2 explicitly leaks one of two holds.
    assert_eq!(host.refs[&1], 0);
    assert_eq!(host.refs[&2000], 0);
    assert_eq!(host.timers, 0);
}

#[test]
fn first_placement_returns_true_even_when_location_fails_and_no_view_is_false() {
    for location in [true, false] {
        let (mut tool, mut host) = setup();
        host.sim_pylons.clear();
        host.hits.clear();
        host.location = location;
        assert_eq!(tool.on_mouse_down_l(30, 40, &mut host), Ok(true));
        assert_eq!(
            host.pylons[&12].position,
            if location { [30.0, 7.0, 40.0] } else { [0.0; 3] }
        );
        host.view = false;
        assert_eq!(tool.on_mouse_down_l(0, 0, &mut host), Ok(false));
        assert_eq!(host.calls.last().unwrap(), "sound:0");
        finish(tool, host);
    }
}

#[test]
fn picked_other_sim_changes_owner_and_deleted_hover_releases_once() {
    let (mut tool, mut host) = setup();
    host.pylons.insert(
        11,
        Pylon {
            owner: Some(2),
            slot7: 1,
            dragging: false,
            position: [5.0, 1.0, 6.0],
            state: 2,
        },
    );
    host.sim_pylons.insert(2, 11);
    host.hits = vec![11];
    tool.on_mouse_down_l(5, 6, &mut host).unwrap();
    assert_eq!(tool.sim(), Some(2));
    assert_eq!(host.refs[&1], 0);
    assert_eq!(host.refs[&2], 1);
    assert!(host.calls.contains(&"end-sim:1".into()));
    tool.set_hover_pylon(Some(11), &mut host);
    assert!(tool.do_message(0x99ef_1143, None, None, Some(11), &mut host));
    assert_eq!(tool.hovered(), None);
    assert_eq!(tool.leaked_hover_references(), 1);
    tool.shutdown(&mut host);
    tool.destroy(&mut host);
    assert_eq!(host.refs[&11], 1);
}
