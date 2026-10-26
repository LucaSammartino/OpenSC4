// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct Fake {
    refs: BTreeMap<Handle, i32>,
    calls: Vec<String>,
    view: Window,
    wealth: u32,
    fail_badge: bool,
    occupied: bool,
    job: bool,
    not_current: bool,
    fail_change: bool,
    missing_lot: bool,
    slot_gap: bool,
}
impl WindowHost for Fake {}
impl InputServices for Fake {
    fn add_ref(&mut self, object: Handle) {
        *self.refs.entry(object).or_default() += 1;
    }
    fn release(&mut self, object: Handle) {
        let count = self.refs.get_mut(&object).expect("release requires ownership");
        *count -= 1;
        assert!(*count >= 0);
        self.calls.push(format!("release:{object}"));
    }
    fn base_init(&mut self) -> bool { self.calls.push("base-init".into()); true }
    fn base_shutdown(&mut self) { self.calls.push("base-shutdown".into()); }
    fn base_destroy(&mut self) { self.calls.push("base-destroy".into()); }
    fn set_cursor(&mut self, cursor: u32) { self.calls.push(format!("cursor:{cursor:x}")); }
    fn end_input(&mut self) { self.calls.push("end".into()); }
    fn is_current_control(&mut self) -> bool { !self.not_current }
    fn view_window(&mut self) -> Option<Window> { Some(self.view) }
    fn main_window(&mut self) -> Option<Window> { Some(self.view) }
    fn focus_view(&mut self) { self.calls.push("focus".into()); }
}

impl HoverServices for Fake {
    fn pick_occupant(&mut self, _: i32, _: i32) -> Option<(Handle, u32)> { Some((900, 0x2781_28a0)) }
    fn building_interface(&mut self, occupant: Handle) -> Option<Handle> { Some(occupant) }
    fn building_slot9(&mut self, _: Handle) -> Result<f32> { if self.slot_gap { Err(Unsupported::BuildingSlotsMst1) } else { Ok(1.0) } }
    fn building_slot11(&mut self, _: Handle) -> Result<u32> { Ok(if self.job { 2 } else { 1 }) }
    fn lot_slot41(&mut self, _: Handle) -> Result<u32> { Ok(2) }
    fn lot_slot33(&mut self, _: Handle) -> Result<u32> { Ok(3) }
    fn lot_slot35(&mut self, _: Handle) -> Result<u32> { Ok(3) }
    fn lot_slot63(&mut self, _: Handle, _: u32) -> Result<i32> { Ok(1) }
    fn my_sim_lives_here(&mut self, _: Handle, _: Handle) -> Option<Handle> { self.occupied.then_some(901) }
    fn test_residence(&mut self, _: SimView, _: Handle, a: bool, b: bool) -> bool { self.calls.push(format!("test:{a}:{b}")); true }
    fn change_residence(&mut self, _: SimView, _: Handle) -> bool { self.calls.push("change-house".into()); !self.fail_change }
    fn change_job(&mut self, _: SimView, _: Handle) -> bool { self.calls.push("change-job".into()); !self.fail_change }
    fn set_residence(&mut self, _: SimView, _: Handle) { self.calls.push("set-house".into()); }
    fn lot_by_occupant(&mut self, _: Handle) -> Option<Handle> { (!self.missing_lot).then_some(902) }
    fn lot_by_position(&mut self, _: [f32; 3]) -> Option<Handle> { (!self.missing_lot).then_some(902) }
    fn building_position(&mut self, _: Handle) -> [f32; 3] { [1.0, 0.0, 2.0] }
    fn building_name(&mut self, _: Handle) -> String { "Building".into() }
    fn occupant_groups(&mut self, _: Handle) -> Option<Vec<u32>> { Some(vec![0x1500, 0x1002]) }
    fn create_callout(&mut self, windows: &mut Windows) -> Option<Window> { let w = windows.create_default(self); self.add_ref(w); Some(w) }
    fn callout_font(&mut self, _: Window, slot: u32, font: u32) { self.calls.push(format!("font:{slot}:{font:x}")); }
    fn callout_icon(&mut self, _: Window, icon: u32) { assert_eq!(icon, 0); }
    fn callout_skin(&mut self, _: Window, key: Key, padding: (i32, i32)) { assert_eq!(padding, (12, 10)); self.calls.push(format!("skin:{:x}", key[2])); }
    fn callout_offset(&mut self, _: Window, offset: Rect) { assert_eq!(offset, Rect(-20, -1000, 20, 28)); }
    fn callout_meter(&mut self, _: Window, key: Key, count: u32, fill: f32) { assert_eq!(key, [0, 0x46a0_06b0, 0x1441_621f]); assert_eq!(count, 3); self.calls.push(format!("meter:{fill}")); }
    fn callout_title(&mut self, _: Window, title: &str) { self.calls.push(format!("title:{title}")); }
    fn callout_body(&mut self, _: Window, body: &str) { self.calls.push(format!("body:{body}")); }
    fn callout_point(&mut self, _: Window, point: (i32, i32)) { self.calls.push(format!("point:{point:?}")); }
    fn position_to_cell(&mut self, _: [f32; 3]) -> (i32, i32) { (1, 2) }
    fn cell_eq(&mut self, _: (i32, i32), _: u32) -> f32 { 49.9 }
    fn cell_hq(&mut self, _: (i32, i32), _: u32) -> f32 { 100.0 }
    fn string(&mut self, table: u32, instance: u32) -> String { assert_eq!(table, 0xea23_1e96); if instance == 0xea5b_8c93 { "EQ".into() } else { assert_eq!(instance, 0xea5b_8c92); "HQ".into() } }
    fn job_title(&mut self, _: SimView, _: Handle) -> String { "Job".into() }
}
impl BuildingServices for Fake {
    fn sim_interface(&mut self, object: Handle) -> Option<Handle> { Some(object + 100) }
    fn agent(&mut self) -> Option<Handle> { Some(200) }
    fn create_badge(&mut self, windows: &mut Windows, parent: Window, key: Key, child: u32, iid: u32) -> Option<Window> {
        assert_eq!(key, [0, 0x96a0_06b0, 0x6a94_55c9]);
        assert_eq!(child, 0x27df_05be);
        assert_eq!(iid, 0x5386_d516);
        if self.fail_badge { return None; }
        let mut allowed = None;
        for (id, children) in [(0x27df_05be, &[0xea94_57ba, 0xcbf3_3d81, 0xcbf3_3d8c][..]), (0x27df_05bf, &[0xea94_57bb][..])] {
            let badge = windows.create_default(self);
            windows.get_mut(badge).id = id;
            windows.set_area(self, badge, Rect(0, 0, 41, 30));
            windows.child_add(self, parent, badge);
            for id in children {
                let child = windows.create_default(self);
                windows.get_mut(child).id = *id;
                windows.child_add(self, badge, child);
            }
            if id == 0x27df_05be { allowed = Some(badge); self.add_ref(badge); }
        }
        allowed
    }
    fn bitmap_id(&mut self, _: SimView) -> u32 { 12 }
    fn bitmap_file(&mut self, _: SimView, index: i32) -> String { assert_eq!(index, -1); "head.bmp".into() }
    fn application_directory(&mut self) -> String { "portraits/".into() }
    fn resource_image(&mut self, key: Key) -> Option<Handle> { self.calls.push(format!("image:{key:x?}")); self.add_ref(300); Some(300) }
    fn file_image(&mut self, path: &str, mode: u32) -> Option<Handle> { self.calls.push(format!("file:{path}:{mode}")); self.add_ref(300); Some(300) }
    fn bitmap_image(&mut self, _: Window, iid: u32, _: Handle) { assert_eq!(iid, 0xc12c_ea13); self.calls.push("bitmap".into()); }
    fn wealth(&mut self, _: SimView) -> u32 { self.wealth }
    fn highlight(&mut self, _: Handle) -> u32 { 7 }
    fn set_highlight(&mut self, building: Handle, level: u32, persistent: bool) { assert!(!persistent); self.calls.push(format!("highlight:{building}:{level}")); }
}

fn setup() -> (Windows, Fake) {
    let mut windows = Windows::default();
    let mut host = Fake { wealth: 1, ..Fake::default() };
    host.view = windows.create_default(&mut host);
    (windows, host)
}

#[test]
fn building_lifecycle_badges_wealth_and_interface_ownership() {
    for kind in [BuildingKind::ChangeHouse, BuildingKind::ChangeJob, BuildingKind::MoveIn] {
        let (mut windows, mut host) = setup();
        let mut tool = BuildingTool::new(kind);
        assert_eq!(tool.init(Some(500), &mut windows, &mut host), Ok(true));
        assert_eq!(tool.sim(), Some(if kind == BuildingKind::MoveIn { SimView::Interface(600) } else { SimView::Object(500) }));
        assert_eq!(&host.calls[..3], ["cursor:43401c71", "base-init", "focus"]);
        let allowed = tool.badges().0.unwrap();
        let wealth = windows.child_from_id_recursive(allowed, 0xcbf3_3d81).unwrap();
        assert_eq!(windows.get(wealth).flags & 1 != 0, kind != BuildingKind::ChangeHouse);
        assert_eq!(host.calls.iter().filter(|s| s.as_str() == "bitmap").count(), 2);
        assert!(!tool.on_key_down(0x41, &mut host));
        assert!(tool.on_key_down(0x1b, &mut host));
        assert!(tool.on_mouse_exit(&mut windows, &mut host));
        tool.shutdown(&mut windows, &mut host);
        tool.destroy(&mut host);
        assert!(host.refs.values().all(|count| *count == 0));
    }
}

#[test]
fn badge_failure_is_success_and_default_owner_kinds_are_distinct() {
    let (mut windows, mut host) = setup();
    host.fail_badge = true;
    let mut tool = change_house::ChangeHouse::default();
    assert_eq!(tool.init(Some(500), &mut windows, &mut host), Ok(true));
    assert_eq!(tool.badges(), (None, None));
    tool.shutdown(&mut windows, &mut host);
    assert!(host.refs.values().all(|count| *count == 0));
    assert_eq!(change_job::ChangeJob::default().kind, BuildingKind::ChangeJob);
    assert_eq!(move_in::MoveIn::default().kind, BuildingKind::MoveIn);
}

#[test]
fn mt_c1_c2_house_callout_meter_and_rejected_occupied_house() {
    let (mut windows, mut host) = setup();
    let mut tool = change_house::ChangeHouse::default();
    tool.init(Some(500), &mut windows, &mut host).unwrap();
    assert_eq!(tool.on_mouse_move(100, 80, &mut windows, &mut host), Ok(true));
    let (allowed, denied) = tool.badges();
    assert_eq!(windows.get(allowed.unwrap()).area, Rect(80, 50, 121, 80));
    assert_eq!(windows.get(denied.unwrap()).flags & 1, 0);
    assert!(host.calls.contains(&"meter:0.66".into()));
    assert!(host.calls.contains(&"body:EQ\nHQ".into()));
    assert!(host.calls.contains(&"test:true:true".into()));
    assert_eq!(tool.highlighted(), Some(900));
    let callout = tool.callout.unwrap();
    tool.on_mouse_exit(&mut windows, &mut host);
    assert_eq!(tool.highlighted(), Some(900));
    assert_ne!(windows.get(callout).flags & 1, 0);
    host.occupied = true;
    tool.on_mouse_move(100, 80, &mut windows, &mut host).unwrap();
    assert_eq!(tool.highlighted(), None);
    assert_eq!(windows.get(callout).flags & 1, 0);
    assert_ne!(windows.get(denied.unwrap()).flags & 1, 0);
    assert!(host.calls.contains(&"highlight:900:7".into()));
    tool.shutdown(&mut windows, &mut host);
    assert!(host.refs.values().all(|count| *count == 0));
}

#[test]
fn mt_c3_c4_job_rejects_house_and_uses_first_group_skin() {
    let (mut windows, mut host) = setup();
    let mut tool = change_job::ChangeJob::default();
    tool.init(Some(500), &mut windows, &mut host).unwrap();
    tool.on_mouse_move(100, 80, &mut windows, &mut host).unwrap();
    assert_eq!(tool.highlighted(), None);
    host.job = true;
    tool.on_mouse_move(100, 80, &mut windows, &mut host).unwrap();
    assert_eq!(tool.highlighted(), Some(900));
    assert!(host.calls.contains(&"skin:14416206".into()));
    assert!(!host.calls.contains(&"skin:14416208".into()));
    assert!(host.calls.contains(&"body:Job".into()));
    tool.shutdown(&mut windows, &mut host);
}

#[test]
fn mt_c5_thresholds_nan_and_unsupported_stale_fill() {
    assert_eq!(eq_text(49.9), 0xea5b_8c93);
    assert_eq!(eq_text(50.0), 0xea5b_8c94);
    assert_eq!(hq_text(100.0), 0xea5b_8c92);
    assert_eq!(eq_text(f32::NAN), 0xea5b_8c95);
    assert_eq!(hq_text(f32::NAN), 0xea5b_8c92);
    assert_eq!(wealth_fill(0), Err(Unsupported::StaleMeterFill));
    assert_eq!(job_skin(None), 0x1441_6200);
}

#[test]
fn clicks_retest_only_change_house_and_end_job_even_on_failure() {
    for kind in [BuildingKind::ChangeHouse, BuildingKind::ChangeJob, BuildingKind::MoveIn] {
        let (mut windows, mut host) = setup();
        host.job = kind == BuildingKind::ChangeJob;
        let mut tool = BuildingTool::new(kind);
        tool.init(Some(500), &mut windows, &mut host).unwrap();
        tool.on_mouse_move(100, 80, &mut windows, &mut host).unwrap();
        host.calls.clear();
        host.not_current = true;
        assert_eq!(tool.on_mouse_down_l(&mut host), Ok(false));
        assert!(host.calls.is_empty());
        host.not_current = false;
        host.fail_change = true;
        assert_eq!(tool.on_mouse_down_l(&mut host), Ok(true));
        match kind {
            BuildingKind::ChangeHouse => assert_eq!(host.calls, ["test:true:false", "change-house"]),
            BuildingKind::ChangeJob => assert_eq!(host.calls, ["change-job", "end"]),
            BuildingKind::MoveIn => assert_eq!(host.calls, ["set-house", "end"]),
        }
        tool.shutdown(&mut windows, &mut host);
    }
}

#[test]
fn raw_slot_gaps_and_null_lot_stop_without_inventing_eligibility() {
    let (mut windows, mut host) = setup();
    let mut tool = change_house::ChangeHouse::default();
    tool.init(Some(500), &mut windows, &mut host).unwrap();
    host.slot_gap = true;
    assert_eq!(tool.on_mouse_move(0, 0, &mut windows, &mut host), Err(Unsupported::BuildingSlotsMst1));
    host.slot_gap = false;
    host.missing_lot = true;
    assert_eq!(tool.on_mouse_move(0, 0, &mut windows, &mut host), Err(Unsupported::MissingLot));
    assert_eq!(tool.highlighted(), None);
    tool.shutdown(&mut windows, &mut host);
}
