// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Tests from budget-window.md §9 plus the routing edge cases of §2-§7.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::ids::*;
use super::*;

#[path = "source_tests.rs"]
mod sources;

const WIN: WinRef = 77;
const NESTED_WIN: WinRef = 78;
const CONTENT: WinRef = 900;

#[derive(Debug, Default)]
struct RefLedger {
    depts: Mutex<HashMap<DeptRef, i32>>,
    items: Mutex<HashMap<ItemRef, i32>>,
    deals: Mutex<HashMap<DealRef, i32>>,
    events: Mutex<Vec<(char, u64)>>,
}

impl BudgetWindowReferences for RefLedger {
    fn add_ref_dept(&self, dept: DeptRef) {
        *self.depts.lock().unwrap().entry(dept).or_default() += 1;
        self.events.lock().unwrap().push(('+', dept));
    }
    fn release_dept(&self, dept: DeptRef) {
        let mut counts = self.depts.lock().unwrap();
        let count = counts.entry(dept).or_default();
        *count -= 1;
        assert!(*count >= 0, "released unowned department {dept}");
        self.events.lock().unwrap().push(('-', dept));
    }
    fn release_win(&self, win: WinRef) {
        self.events.lock().unwrap().push(('w', win));
    }
    fn add_ref_item(&self, item: ItemRef) {
        *self.items.lock().unwrap().entry(item).or_default() += 1;
    }
    fn release_item(&self, item: ItemRef) {
        let mut counts = self.items.lock().unwrap();
        let count = counts.entry(item).or_default();
        *count -= 1;
        assert!(*count >= 0, "released unowned item {item}");
    }
    fn release_deal(&self, deal: DealRef) {
        let mut counts = self.deals.lock().unwrap();
        let count = counts.entry(deal).or_default();
        *count -= 1;
        assert!(*count >= 0, "released unowned deal {deal}");
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Deal {
    flags: u32,
    amount: i32,
    last: i32,
    inc: i32,
    last_inc: i32,
    m: u32,
    monthly: i64,
}

/// A recording host. `script` is what `do_modal_win` delivers to the first
/// dialog, `nested_script` to the nested one.
#[derive(Default)]
struct Rec {
    refs: Arc<RefLedger>,
    painted: Vec<(bool, DeptRef, Option<u32>, u32)>,
    slider_children: Vec<(WinRef, u32, DeptRef)>,
    log: Vec<String>,
    create_fails: bool,
    created: u64,
    modal_flag: bool,
    gen_ok: bool,
    script: Vec<SubBudgetMsg>,
    nested_script: Vec<SubBudgetMsg>,
    funding: HashMap<DeptRef, f32>,
    group: Vec<DeptRef>,
    items: HashMap<DeptRef, Vec<LineItem>>,
    total_income: HashMap<DeptRef, i64>,
    ideal: HashMap<DeptRef, i64>,
    ord: OrdinanceInit,
    checks: HashMap<u32, u8>,
    deal_list: Vec<DealRef>,
    deals: HashMap<DealRef, Deal>,
    deal_kinds: HashMap<DealRef, u32>,
    combo: HashMap<u32, u32>,
    blocked: bool,
    paints: Vec<BodyPaint>,
    ord_income: HashMap<OrdId, i64>,
    ord_categories: HashMap<OrdId, bool>,
    ord_observations: HashMap<OrdId, Vec<u8>>,
    property_kind: Option<u32>,
    released_properties: Vec<u64>,
    property_object_reads: usize,
    restore: Vec<IndividualRestoreInfo>,
    by_id: HashMap<u32, ItemRef>,
    item_dept: HashMap<ItemRef, u32>,
    not_occupant: bool,
    query: Option<FundingPair>,
    slider_missing: bool,
    slider_pos: i32,
    content_fails: bool,
    paused: i32,
    pause_msgs: Vec<bool>,
    paused_in_modal: Vec<i32>,
    ui_disabled: bool,
    filter: bool,
    results: Vec<bool>,
}

impl Rec {
    fn l(&mut self, s: String) {
        self.log.push(s);
    }
    fn count(&self, prefix: &str) -> usize {
        self.log.iter().filter(|s| s.starts_with(prefix)).count()
    }
    fn pos(&self, s: &str) -> usize {
        self.log
            .iter()
            .position(|x| x == s)
            .unwrap_or_else(|| panic!("{s} not in {:?}", self.log))
    }
    fn has(&self, s: &str) -> bool {
        self.log.iter().any(|x| x == s)
    }
}

impl BudgetWindowHost for Rec {
    fn references(&self) -> Arc<dyn BudgetWindowReferences> {
        self.refs.clone()
    }
    fn create_window_generator(&mut self) -> Option<WinRef> {
        self.l("create".into());
        if self.create_fails {
            return None;
        }
        self.created += 1;
        Some(WIN + self.created - 1)
    }
    fn add_ref_win(&mut self, w: WinRef) {
        self.l(format!("addref {w}"));
    }
    fn release_win(&mut self, w: WinRef) {
        self.l(format!("release {w}"));
        self.refs.release_win(w);
    }
    fn set_generator_flag(&mut self, _: WinRef, f: u32, on: bool) {
        self.l(format!("genflag {f:#x} {on}"));
    }
    fn set_generator_win_proc(&mut self, _: WinRef) {
        self.l("winproc".into());
    }
    fn query_generator(&mut self, _: WinRef) -> bool {
        self.gen_ok
    }
    fn release_generator(&mut self, w: WinRef) {
        self.l(format!("release_gen {w}"));
    }
    fn set_win_flag(&mut self, _: WinRef, f: u32, on: bool) {
        self.l(format!("winflag {f:#x} {on}"));
    }
    fn win_flag(&mut self, _: WinRef, _: u32) -> bool {
        self.modal_flag
    }
    fn set_win_id(&mut self, _: WinRef, id: u32) {
        self.l(format!("id {id:#x}"));
    }
    fn show_win(&mut self, _: WinRef) {
        self.l("show".into());
    }
    fn pull_to_front(&mut self, _: WinRef) {
        self.l("front".into());
    }
    fn end_modal_via_parent(&mut self, w: WinRef, code: u32) {
        self.l(format!("end_modal {w} {code:#x}"));
    }
    fn remove_from_parent(&mut self, w: WinRef) {
        self.l(format!("remove_from_parent {w}"));
    }
    fn remove_child(&mut self, _: WinRef, c: Option<WinRef>) {
        self.l(format!("remove_child {c:?}"));
    }
    fn create_content_window(&mut self) -> Option<WinRef> {
        self.l("content_create".into());
        if self.content_fails {
            return None;
        }
        Some(CONTENT)
    }
    fn child_add(&mut self, w: WinRef, c: WinRef) {
        self.l(format!("child_add {w} {c}"));
    }
    fn add_image_button2(&mut self, _: WinRef, id: u32, image: u32) -> WinRef {
        self.l(format!("button {id:#x} {image:#x}"));
        1000 + id as WinRef
    }
    fn set_notification_target(&mut self, c: WinRef, t: WinRef) {
        self.l(format!("target {c} {t}"));
    }
    fn set_child_visible(&mut self, _: WinRef, id: u32, on: bool) {
        self.l(format!("arrow {id:#x} {on}"));
    }
    fn restore_window_location(&mut self, _: WinRef) {
        self.l("restore_loc".into());
    }
    fn save_window_location(&mut self, _: WinRef) {
        self.l("save_loc".into());
    }
    fn do_modal_win(&mut self, w: WinRef, dialog: &mut SubBudgetWindow) {
        self.l(format!("modal {w}"));
        let script = std::mem::take(if w == WIN {
            &mut self.script
        } else {
            &mut self.nested_script
        });
        for m in script {
            let r = dialog.do_win_msg(self, w, m);
            self.results.push(r);
            self.paused_in_modal.push(self.paused);
        }
    }
    fn focus_view3d(&mut self) {
        self.l("focus".into());
    }
    fn update_balance_sheet(&mut self) {
        self.l("balance".into());
    }
    fn filter_ui_command(&mut self, _: u32) -> bool {
        self.filter
    }
    fn set_panel_tab_visible(&mut self, t: u32, v: bool) {
        self.l(format!("tab {t:#x} {v}"));
    }
    fn set_panel_tab_flag(&mut self, t: u32, f: u32, on: bool) {
        self.l(format!("tabflag {t:#x} {f:#x} {on}"));
    }
    fn hidden_pause(&mut self) {
        self.paused += 1;
        if self.paused == 1 {
            self.pause_msgs.push(true);
        }
        self.l("pause".into());
    }
    fn hidden_resume(&mut self) {
        self.paused -= 1;
        if self.paused == 0 {
            self.pause_msgs.push(false);
        }
        self.l("resume".into());
    }
    fn load_background(&mut self, _: WinRef, _: u32, _: u32, i: u32) {
        self.l(format!("bg {i:#x}"));
    }
    fn set_ordinance_on(&mut self, o: OrdId, on: u32) {
        self.l(format!("ord {o} {on}"));
    }
    fn check_box_state(&mut self, _: WinRef, id: u32) -> Option<u8> {
        self.checks.get(&id).copied()
    }
    fn set_neighbor_ui_disable(&mut self, on: bool) {
        self.ui_disabled = on;
        self.l(format!("uidisable {on}"));
    }
    fn add_ref_deal(&mut self, d: DealRef) {
        *self.refs.deals.lock().unwrap().entry(d).or_default() += 1;
        self.l(format!("addref_deal {d}"));
    }
    fn release_deal(&mut self, d: DealRef) {
        self.refs.release_deal(d);
        self.l(format!("release_deal {d}"));
    }
    fn deal_flags(&mut self, d: DealRef) -> u32 {
        self.deals[&d].flags
    }
    fn deal_cancel(&mut self, d: DealRef) {
        self.l(format!("cancel {d}"));
        let x = self.deals.get_mut(&d).unwrap();
        x.flags = (x.flags | DEAL_CANCELLED) & !DEAL_ACTIVE;
        x.amount = 0;
    }
    fn deal_activate(&mut self, d: DealRef) -> bool {
        let x = self.deals.get_mut(&d).unwrap();
        if x.flags & DEAL_PROPOSED_BIT == 0 {
            return false;
        }
        x.flags = DEAL_ACTIVE;
        true
    }
    fn deal_set_amount(&mut self, d: DealRef, v: i32) {
        self.deals.get_mut(&d).unwrap().amount = v;
    }
    fn deal_last_amount(&mut self, d: DealRef) -> i32 {
        self.deals[&d].last
    }
    fn deal_set_last_amount(&mut self, d: DealRef, v: i32) {
        self.deals.get_mut(&d).unwrap().last = v;
    }
    fn deal_set_inc_amount(&mut self, d: DealRef, v: i32) {
        self.deals.get_mut(&d).unwrap().inc = v;
    }
    fn deal_set_last_inc_amount(&mut self, d: DealRef, v: i32) {
        self.deals.get_mut(&d).unwrap().last_inc = v;
    }
    fn deal_last_inc_amount(&mut self, d: DealRef) -> i32 {
        self.deals[&d].last_inc
    }
    fn mark_deals_dirty(&mut self) {
        self.l("dirty".into());
    }
    fn department_budgets_in_group(&mut self, _: u32) -> Vec<DeptRef> {
        for &dept in &self.group { if dept != 0 { self.refs.add_ref_dept(dept); } }
        self.group.clone()
    }
    fn department_line_items(&mut self, d: DeptRef) -> Vec<LineItem> {
        self.items.get(&d).cloned().unwrap_or_default()
    }
    fn department_total_income(&mut self, d: DeptRef) -> i64 {
        self.total_income.get(&d).copied().unwrap_or(0)
    }
    fn department_ideal_monthly_funding(&mut self, d: DeptRef) -> i64 {
        self.ideal.get(&d).copied().unwrap_or(0)
    }
    fn funding_percentage(&mut self, d: DeptRef) -> f32 {
        self.funding[&d]
    }
    fn set_funding_percentage(&mut self, d: DeptRef, f: f32, a: u32) {
        self.funding.insert(d, f);
        self.l(format!("fund {d} {:#x} {a}", f.to_bits()));
    }
    fn paint_group_row(&mut self, content: WinRef, _: u32, d: DeptRef, item: Option<LineItem>, income: bool, row: u32) {
        assert_eq!(content, CONTENT);
        self.painted.push((income, d, item.map(|i| i.id), row));
    }
    fn create_department_slider(&mut self, win: WinRef, d: DeptRef, _: Option<LineItem>, _: u32) -> u32 {
        let id = 1001 + self.slider_children.len() as u32;
        self.slider_children.push((win, id, d));
        id
    }
    fn find_slider(&mut self, _: WinRef, _: u32) -> bool {
        !self.slider_missing
    }
    fn slider_value(&mut self, _: WinRef, _: u32) -> i32 {
        self.slider_pos
    }
    fn find_child(&mut self, _: WinRef, id: u32) -> Option<WinRef> { self.by_id.contains_key(&id).then_some(u64::from(id)) }
    fn get_property(&mut self, child: WinRef, key: u32) -> Option<u64> { assert_eq!(key, 0x4BFEE088); Some(child) }
    fn property_type(&mut self, _: u64) -> u32 { self.property_kind.unwrap_or(0xF) }
    fn property_object(&mut self, property: u64) -> Option<ItemRef> {
        self.property_object_reads += 1;
        self.by_id.get(&(property as u32)).copied()
    }
    fn release_property(&mut self, property: u64) { self.released_properties.push(property); }
    fn item_id(&mut self, i: ItemRef) -> u32 {
        self.item_dept[&i]
    }
    fn query_occupant(&mut self, i: ItemRef) -> bool {
        self.l(format!("qi {i}"));
        !self.not_occupant
    }
    fn release_occupant(&mut self, i: ItemRef) {
        self.l(format!("rel_occ {i}"));
    }
    fn hospital_query_data(&mut self, _: ItemRef) -> Option<FundingPair> {
        self.query
    }
    fn school_query_data(&mut self, _: ItemRef) -> Option<FundingPair> {
        self.query
    }
    fn set_occupant_funding_percentages(&mut self, o: ItemRef, r: FundingPair, s: bool) {
        self.l(format!(
            "occfund {o} {:#x} {:#x} {s}",
            r.staff.to_bits(),
            r.vehicle.to_bits()
        ));
    }
    fn police_set_station_funding(&mut self, o: ItemRef, p: f32) {
        self.l(format!("police {o} {:#x}", p.to_bits()));
    }
    fn fire_set_station_funding(&mut self, o: ItemRef, p: f32) {
        self.l(format!("fire {o} {:#x}", p.to_bits()));
    }
    fn power_set_plant_funding(&mut self, o: ItemRef, p: f32) {
        self.l(format!("power {o} {:#x}", p.to_bits()));
    }
    fn occupant_position(&mut self, _: ItemRef, v: [f32; 3]) -> [f32; 3] {
        [v[0] + 1.0, v[1] + 2.0, v[2] + 3.0]
    }
    fn camera_jump(&mut self, v: [f32; 3]) {
        self.l(format!("jump {v:?}"));
    }
}

fn group_host() -> Rec {
    let mut h = Rec {
        group: vec![1, 2, 3],
        ..Default::default()
    };
    h.funding.extend([(1, 0.5f32), (2, 0.75), (3, 1.0)]);
    for d in 1..=3 {
        h.items.insert(d, vec![item(2, 7, 1, 0, 300)]);
    }
    h
}

fn item(kind: u32, id: u32, flags: u32, income: i64, full_expenses: i64) -> LineItem {
    LineItem {
        kind,
        id,
        display_flags: flags,
        income,
        full_expenses,
    }
}

/// A dialog created and initialised outside the panel.
fn open(h: &mut Rec, selector: u32, dept: u32) -> SubBudgetWindow {
    let mut d = SubBudgetWindow::new();
    assert_eq!(d.create(h, selector, 5, dept), Some(WIN));
    assert!(d.do_win_msg(h, WIN, SubBudgetMsg::Init));
    d
}

#[test]
fn panel_command_selector_table() {
    use PanelCommand::*;
    for (base, sel) in [
        (0x01, SELECTOR_ORDINANCES),
        (0x09, SELECTOR_ORDINANCES),
        (0x02, SELECTOR_NEIGHBORS),
        (0x0A, SELECTOR_NEIGHBORS),
        (0x03, GROUP_CIVIC),
        (0x04, GROUP_TRANSPORTATION),
        (0x05, GROUP_TRANSPORTATION),
        (0x06, GROUP_PROTECTION),
        (0x07, GROUP_EDUCATION_HEALTH),
        (0x08, GROUP_UTILITIES),
        (0x0B, GROUP_PARKS_LANDMARKS),
        (0x0C, GROUP_CIVIC2),
    ] {
        assert_eq!(panel_command(0xAA3A_C400 | base), Some(SubBudget(sel)));
        assert_eq!(panel_command(0xAA3A_C500 | base), Some(SubBudget(sel)));
    }
    assert_eq!(panel_command(0xAA3A_C400), Some(DisplayTaxesWindow));
    assert_eq!(panel_command(0xAA3A_C50D), Some(DisplayLoanWindow));
    assert_eq!(panel_command(0xAA3A_C40E), Some(SetTab(true)));
    assert_eq!(panel_command(0x4A88_79DB), Some(SetTab(false)));
    assert_eq!(panel_command(0xAA4C_33E6), Some(TakeLoan));
    assert_eq!(panel_command(0x1234), None);
}

#[test]
fn create_sequence_and_failure() {
    let mut h = Rec::default();
    let mut d = SubBudgetWindow::new();
    assert_eq!(d.create(&mut h, 2, 9, 0x1234), Some(WIN));
    assert_eq!((d.selector(), d.context(), d.department()), (2, 9, 0x1234));
    let want = [
        "create",
        "addref 77",
        "genflag 0x1 true",
        "genflag 0x2 false",
        "genflag 0x10000 false",
        "genflag 0x20 false",
        "genflag 0x40 false",
        "genflag 0x1000 true",
        "genflag 0x2000 false",
        "genflag 0x4000 false",
        "winproc",
        "winflag 0x100 true",
        "winflag 0x10000 false",
        "winflag 0x4 true",
        "winflag 0x800 false",
        "id 0x423278f",
    ];
    assert_eq!(h.log, want);
    let mut h = Rec {
        create_fails: true,
        ..Default::default()
    };
    let mut d = SubBudgetWindow::new();
    assert_eq!(d.create(&mut h, 1, 0, 0), None);
    assert_eq!(d.window(), None);
}

/// §9 test 1.
#[test]
fn opening_while_held_does_nothing() {
    let mut h = Rec::default();
    let mut p = BudgetPanel::new(3);
    p.held = Some(SubBudgetWindow::new());
    p.do_sub_budget_window(&mut h, 1);
    assert!(h.log.is_empty());
}

/// §9 tests 2 and 7, §2 references (BW6a).
#[test]
fn modal_run_pauses_releases_and_refreshes_once() {
    let mut h = group_host();
    h.gen_ok = true;
    h.modal_flag = true;
    h.script = vec![SubBudgetMsg::Init, SubBudgetMsg::Button(BTN_OK)];
    let mut p = BudgetPanel::new(3);
    p.do_sub_budget_window(&mut h, GROUP_UTILITIES);
    assert_eq!(h.results, [true, true]);
    assert_eq!(h.paused_in_modal, [1, 1]);
    assert_eq!(h.paused, 0);
    assert_eq!(h.pause_msgs, [true, false]);
    assert!(!p.is_holding_sub_budget());
    assert_eq!(h.count("balance"), 1);
    let order = [
        "show",
        "winflag 0x800 true",
        "front",
        "winflag 0x800 false",
        "winflag 0x100 true",
        "genflag 0x1000 true",
        "genflag 0x2000 true",
        "genflag 0x4000 true",
        "restore_loc",
        "modal 77",
        "focus",
        "save_loc",
        "release 77",
        "release_gen 77",
        "resume",
        "balance",
    ];
    let mut last = 0;
    for s in order {
        let i = h.log[last..].iter().position(|x| x == s).map(|i| i + last);
        last = i.unwrap_or_else(|| panic!("{s} missing after {last}: {:?}", h.log));
    }
    // OK ends the modal loop with the code; Destroy ends it again after the
    // resume (§6); the destructor releases +0x0C (no shutdown delivered).
    assert_eq!(h.count("end_modal 77 0x42b7c353"), 2);
    assert!(h.pos("resume") < h.log.iter().rposition(|x| x.starts_with("end_modal")).unwrap());
    assert_eq!(h.count("release 77"), 2);
}

/// §9 test 7 second half (BW6b): the unguarded resume.
#[test]
fn create_failure_underflows_pause_counter() {
    let mut h = Rec {
        create_fails: true,
        ..Default::default()
    };
    let mut p = BudgetPanel::new(0);
    p.do_sub_budget_window(&mut h, 1);
    assert_eq!(h.paused, -1);
    assert!(h.pause_msgs.is_empty());
    assert_eq!(h.count("release"), 0);
    assert_eq!(h.count("end_"), 0);
    assert_eq!(h.count("balance"), 1);
    // The next dialog's pause brings it to 0: no message, not paused.
    h.create_fails = false;
    h.script = vec![SubBudgetMsg::Init];
    p.do_sub_budget_window(&mut h, 1);
    assert_eq!(h.paused_in_modal, [0]);
    assert!(h.pause_msgs.is_empty());
    assert_eq!(h.paused, -1);
}

#[test]
fn tutorial_filter_swallows() {
    let mut h = Rec {
        filter: true,
        ..Default::default()
    };
    let mut p = BudgetPanel::new(0);
    assert_eq!(p.on_command(&mut h, 0xAA3A_C401), CommandOutcome::Swallowed);
    assert!(h.log.is_empty());
    h.filter = false;
    assert_eq!(p.on_command(&mut h, 0x99), CommandOutcome::Unknown);
    assert_eq!(
        p.on_command(&mut h, 0xAA3A_C40D),
        CommandOutcome::Panel(PanelCommand::DisplayLoanWindow)
    );
}

/// §9 test 11 (BW4).
#[test]
fn tab_commands_run_show_window() {
    let mut h = Rec::default();
    let mut p = BudgetPanel::new(0);
    assert_eq!(p.on_command(&mut h, 0xAA3A_C40E), CommandOutcome::TabShown);
    assert!(p.tab_flag && p.shown);
    assert_eq!(
        h.log,
        [
            "tab 0xaa3ac000 false",
            "tab 0xaa3ac001 true",
            "tabflag 0xaa3ac001 0x800 false",
            "balance"
        ]
    );
    h.log.clear();
    assert_eq!(p.on_command(&mut h, 0x4A88_79DB), CommandOutcome::TabShown);
    assert!(!p.tab_flag);
    assert_eq!(h.log[1], "tab 0xaa3ac000 true");
}

/// §9 tests 3 and 13 (BW8).
#[test]
fn group_slider_then_cancel_restores_start_fractions() {
    let mut h = group_host();
    let mut d = open(&mut h, GROUP_UTILITIES, 0);
    assert_eq!(d.sliders().len(), 6);
    h.slider_pos = 30;
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Slider(1003)));
    assert!(h.has("fund 2 0x3e999999 0"));
    assert_eq!(h.count("values"), 1);
    assert!(d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_CANCEL)));
    assert_eq!(h.funding[&1], 0.5);
    assert_eq!(h.funding[&2], 0.75);
    assert_eq!(h.funding[&3], 1.0);
    // Cancel ends the dialog with no refresh.
    assert_eq!(h.count("values"), 1);
    // A failed slider lookup does nothing.
    h.slider_missing = true;
    h.log.clear();
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Slider(1002)));
    assert!(h.log.is_empty());
}

#[test]
fn slider_reentry_guard_is_shared() {
    let guard = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let mut h = group_host();
    let mut d = SubBudgetWindow::with_slider_guard(guard.clone());
    d.create(&mut h, GROUP_UTILITIES, 0, 0);
    d.on_init(&mut h, Some(WIN));
    h.log.clear();
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Slider(1001));
    assert!(h.log.is_empty());
    guard.store(false, std::sync::atomic::Ordering::SeqCst);
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Slider(1001));
    assert_eq!(h.count("fund 1"), 1);
    assert!(!guard.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn slider_fraction_is_the_f32_product() {
    assert_eq!(slider_fraction(30).to_bits(), 0x3E99_9999);
    let differ = (0..=200)
        .filter(|&p| slider_fraction(p) != (f64::from(p) * 0.01) as f32)
        .count();
    assert_eq!(differ, 59);
    for p in [5, 9, 10, 15, 18, 20] {
        assert_ne!(slider_fraction(p), (f64::from(p) * 0.01) as f32, "{p}");
    }
    assert_eq!(slider_fraction(100), 1.0);
}

/// §4.1 worked example (BW1) and the empty window.
#[test]
fn group_counts_and_special_headers() {
    let make = |group: u32| {
        let mut h = Rec {
            group: vec![10, 11],
            ..Default::default()
        };
        h.funding.extend([(10, 0.5f32), (11, 0.5)]);
        h.items.insert(10, vec![item(1, 5, 1, 0, 0), item(1, 200, 1, 0, 0)]);
        h.items.insert(11, vec![item(2, 7, 1, 0, 300)]);
        h.ideal.insert(11, 300);
        let d = open(&mut h, group, 0);
        (d, h)
    };
    let (d, h) = make(GROUP_PARKS_LANDMARKS);
    assert_eq!((d.list_a().len, d.list_b().len), (1, 2));
    assert!(h.has("big 1 2") && h.has("bg 0x140155d2"));
    let (d, _) = make(GROUP_UTILITIES);
    assert_eq!((d.list_a().len, d.list_b().len), (2, 2));
    // Hidden flag bit: no row; header still counted outside special groups.
    let mut h = Rec {
        group: vec![1],
        ..Default::default()
    };
    h.items.insert(1, vec![item(2, 500, 0, 0, 9)]);
    h.funding.insert(1, 0.5);
    let d = open(&mut h, GROUP_UTILITIES, 0);
    assert_eq!(d.list_b().len, 1);
    // Both lists empty: the empty window becomes the child, no rows.
    let mut h = Rec {
        group: vec![1],
        ..Default::default()
    };
    h.items.insert(1, vec![item(2, 500, 0, 0, 9)]);
    h.funding.insert(1, 0.5);
    let d = open(&mut h, GROUP_CIVIC, 0);
    assert_eq!((d.list_a().len, d.list_b().len), (0, 0));
    assert_eq!(d.inner_child(), None);
    assert!(d.unsupported_operations().contains(&Unsupported::Setup));
    assert!(d.sliders().is_empty() && h.count("big") == 0);
}

/// §9 test 4 with the check boxes and the row map (BW9).
#[test]
fn ordinance_apply_calls_only_changed_rows() {
    let mut h = Rec { ..Default::default() };
    h.ord = OrdinanceInit {
        rows: vec![
            OrdinanceRow {
                ordinance: 40,
                original: 0,
                edited: 0,
            },
            OrdinanceRow {
                ordinance: 41,
                original: 1,
                edited: 1,
            },
            OrdinanceRow {
                ordinance: 42,
                original: 0,
                edited: 0,
            },
        ],
        row_map: vec![2, 0, 1],
        ..Default::default()
    };
    h.checks.extend([(300, 1u8), (301, 0), (302, 1)]);
    let mut d = open(&mut h, 1, 0);
    assert!(h.has("bg 0x140155f2"));
    for id in [300, 301, 302] {
        assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(id)));
    }
    assert!(d.unsupported_operations().contains(&Unsupported::OrdinanceRefresh));
    // All-income provider order: 40 on, 41 off, 42 on.
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_ORDINANCE_APPLY)));
    let ords: Vec<_> = h.log.iter().filter(|s| s.starts_with("ord ")).cloned().collect();
    assert_eq!(ords, ["ord 40 1", "ord 41 0", "ord 42 1"]);
    assert_eq!(h.count("upd_ord"), 0);

    let mut h2 = Rec {
        ord: h.ord.clone(),
        checks: h.checks.clone(),
        ..Default::default()
    };
    let mut d = open(&mut h2, 1, 0);
    d.do_win_msg(&mut h2, WIN, SubBudgetMsg::Button(300));
    assert!(d.do_win_msg(&mut h2, WIN, SubBudgetMsg::Button(BTN_CANCEL)));
    assert_eq!(h2.count("ord "), 0);
    // A child that does not answer the check-box interface: no edit, still a refresh.
    let mut h3 = Rec {
        ord: h.ord.clone(),
        ..Default::default()
    };
    let mut d = open(&mut h3, 1, 0);
    d.do_win_msg(&mut h3, WIN, SubBudgetMsg::Button(300));
    assert_eq!(d.ordinances()[2].edited, 0);
    assert!(d.unsupported_operations().contains(&Unsupported::OrdinanceRefresh));
}

/// §5.6: the info popup and the close-child buttons.
#[test]
fn info_popup_and_close_child() {
    let mut h = Rec::default();
    h.ord.rows = vec![OrdinanceRow {
        ordinance: 9,
        original: 0,
        edited: 0,
    }];
    h.ord.row_map = vec![0];
    let mut d = open(&mut h, 1, 0);
    // 0x68 with no child: RemoveChild(null) runs, nothing is released.
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_CLOSE_CHILD)));
    assert_eq!(h.log.last().unwrap(), "remove_child None");
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(500)));
    assert!(d.unsupported_operations().contains(&Unsupported::OrdinanceInfo));
    assert_eq!(d.inner_child(), None);
    // A second info button closes the first popup.
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(500));
    assert!(!h.has("release 500"));
    h.log.clear();
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_CLOSE_CHILD_ALT)));
    assert_eq!(h.log, ["remove_child None"]);
    assert_eq!(d.inner_child(), None);
}

/// §9 test 5, §5.4 worked cases (BW2), §6 deal close.
#[test]
fn neighbour_mode_apply_deal_edits() {
    let mut h = Rec {
        deal_list: vec![1, 2, 3, 4],
        ..Default::default()
    };
    h.deals.insert(
        1,
        Deal {
            flags: DEAL_ACTIVE,
            last: 50,
            last_inc: 4,
            amount: 10,
            ..Default::default()
        },
    );
    h.deals.insert(
        2,
        Deal {
            flags: DEAL_ACTIVE,
            last: 0,
            last_inc: 3,
            amount: 10,
            ..Default::default()
        },
    );
    h.deals.insert(
        3,
        Deal {
            flags: DEAL_PROPOSED_BIT | DEAL_CANCELLED,
            last: 30,
            last_inc: 2,
            ..Default::default()
        },
    );
    h.deals.insert(
        4,
        Deal {
            flags: DEAL_PROPOSED_BIT,
            last: 70,
            ..Default::default()
        },
    );
    let mut p = BudgetPanel::new(0);
    h.modal_flag = true;
    h.script = vec![SubBudgetMsg::Init, SubBudgetMsg::Combo(5), SubBudgetMsg::Button(BTN_CF)];
    p.do_sub_budget_window(&mut h, SELECTOR_NEIGHBORS);
    assert_eq!(h.results, [true, false, false]);
    // No native owner callback supplies combo rows or refresh output (BW17).
    assert_eq!(h.count("upd_nb"), 0);
    let d1 = h.deals[&1];
    assert_eq!((d1.flags, d1.amount, d1.inc), (DEAL_ACTIVE, 50, 4));
    let d2 = h.deals[&2];
    assert_eq!((d2.flags, d2.amount, d2.last, d2.last_inc), (DEAL_CANCELLED, 0, 0, 0));
    let d3 = h.deals[&3];
    assert_eq!((d3.flags, d3.amount, d3.inc), (DEAL_ACTIVE, 30, 2));
    // Proposed only: untouched by the apply, cancelled by Destroy (bit 0).
    let d4 = h.deals[&4];
    assert_eq!((d4.flags & DEAL_CANCELLED, d4.amount), (DEAL_CANCELLED, 0));
    assert_eq!(h.count("cancel 4"), 1);
    assert!(!h.ui_disabled);
    assert!(h.pos("uidisable false") < h.pos("dirty"));
    assert!(h.pos("dirty") < h.pos("resume"));
}

#[test]
fn combo_outside_neighbour_mode_is_ignored() {
    let mut h = group_host();
    let mut d = open(&mut h, GROUP_UTILITIES, 0);
    h.log.clear();
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Combo(1)));
    assert!(h.log.is_empty());
}

/// §9 test 6 (BW5 open: the raw bit only).
#[test]
fn keys_close_only_as_specified() {
    let mut h = group_host();
    let mut d = open(&mut h, GROUP_UTILITIES, 0);
    h.log.clear();
    for (key, m) in [(KEY_ENTER, 0), (KEY_0X73, 0x3), (0x20, 0x4), (KEY_ENTER, 0xFB)] {
        assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::KeyDown { key, modifiers: m }));
    }
    assert!(h.log.is_empty());
    for (key, m) in [(KEY_ESC, 0), (KEY_ENTER, 0x4), (KEY_0X73, 0x5)] {
        h.log.clear();
        assert!(d.do_win_msg(&mut h, WIN, SubBudgetMsg::KeyDown { key, modifiers: m }));
        assert_eq!(h.log, ["remove_from_parent 77"]);
    }
}

/// §9 test 9 (BW9).
#[test]
fn do_win_msg_returns() {
    let mut h = group_host();
    let mut d = open(&mut h, GROUP_UTILITIES, 0);
    assert_eq!(h.paused, 1);
    assert!(d.do_win_msg(&mut h, WIN, SubBudgetMsg::Init));
    assert_eq!(h.paused, 1);
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Slider(1001)));
    h.slider_missing = true;
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Slider(1001)));
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Other(0x1234)));
    // Unmatched buttons: 0 with no refresh.
    h.log.clear();
    for id in [0x66, 0x385 + 1, 0x0ABC_E700, BTN_CAMERA_JUMP_FIRST] {
        assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(id)));
    }
    assert!(h.log.is_empty());
    assert!(d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_END_ALT)));
    // Null window: on_init returns 0 and does nothing.
    let mut d2 = SubBudgetWindow::new();
    assert!(!d2.on_init(&mut h, None));
    assert!(!d2.is_initialised());
}

/// §9 test 10 (BW9).
#[test]
fn on_shutdown_releases_window_once_initialised() {
    let mut h = Rec::default();
    let mut d = SubBudgetWindow::new();
    d.create(&mut h, GROUP_UTILITIES, 0, 0);
    assert!(d.do_win_msg(&mut h, WIN, SubBudgetMsg::Shutdown));
    assert_eq!(d.window(), Some(WIN));
    let mut h = Rec {
        ord: OrdinanceInit {
            rows: vec![OrdinanceRow {
                ordinance: 1,
                original: 0,
                edited: 1,
            }],
            row_map: vec![0],
            ..Default::default()
        },
        ..Default::default()
    };
    let mut d = open(&mut h, 1, 0);
    h.log.clear();
    assert!(d.do_win_msg(&mut h, WIN, SubBudgetMsg::Shutdown));
    // +0x0C, then +0x10 (BW9, BW10a).
    assert_eq!(h.log, ["release 77", "release 900"]);
    assert_eq!(d.window(), None);
    assert_eq!(d.content(), None);
    assert!(!d.is_initialised());
    assert_eq!(d.ordinance_count(), 1);
    assert!(d.ordinances().is_empty());
    h.log.clear();
    assert!(d.destroy(&mut h));
    assert_eq!(h.log, ["resume"]);
    d.release_final(&mut h);
    assert_eq!(h.log, ["resume"]);
}

#[test]
fn end_dialog_branches() {
    let mut h = Rec::default();
    assert!(!end_dialog(&mut h, None, END_CODE));
    assert!(end_dialog(&mut h, Some(5), 7));
    h.modal_flag = true;
    assert!(end_dialog(&mut h, Some(5), 7));
    assert_eq!(h.log, ["remove_from_parent 5", "end_modal 5 0x7"]);
}

/// §9 test 12, §5.3 (BW7).
#[test]
fn scroll_steps_arrows_and_refresh() {
    let mut h = group_host();
    let mut d = open(&mut h, GROUP_UTILITIES, 0);
    d.set_list_len(false, 12);
    let mut seen = vec![];
    for _ in 0..4 {
        h.log.clear();
        assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_LIST_A_DOWN)));
        seen.push(d.list_a().pos);
    }
    assert_eq!(seen, [2, 4, 4, 4]);
    assert_eq!(h.log, ["arrow 0x452 false", "arrow 0x451 true", "values"]);
    h.log.clear();
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_LIST_A_UP));
    assert_eq!(d.list_a().pos, 2);
    assert_eq!(h.log, ["arrow 0x452 true", "values"]);
    h.log.clear();
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_LIST_A_UP));
    assert_eq!(h.log, ["arrow 0x451 false", "arrow 0x452 true", "values"]);
    // A list that fits: down does not move but shows up.
    d.set_list_len(true, 5);
    h.log.clear();
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_LIST_B_DOWN));
    assert_eq!(d.list_b().pos, 0);
    assert_eq!(h.log, ["arrow 0x454 false", "arrow 0x453 true", "values"]);
    // Mode refreshes.
    let mut h = Rec::default();
    let mut d = open(&mut h, 1, 0);
    h.log.clear();
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_LIST_B_UP));
    assert_eq!(h.log, ["arrow 0x453 false"]);
    assert!(d.unsupported_operations().contains(&Unsupported::OrdinanceRefresh));
    let mut h = Rec::default();
    let mut d = open(&mut h, 0, DEPT_FIRE);
    h.log.clear();
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_LIST_B_UP));
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_LIST_A_UP));
    assert_eq!(
        h.log,
        ["arrow 0x453 false", "occupants 0x28f55a9f", "empty", "arrow 0x451 false", "values"]
    );
}

/// §5.5 table and example (BW3, BW8), §5.1 individual restore.
#[test]
fn occupant_funding_levels() {
    let mut h = Rec {
        query: Some(FundingPair {
            staff: 0.5,
            vehicle: 0.25,
        }),
        ..Default::default()
    };
    h.by_id.extend([(0x0ABC_E4FF, 7), (0x0ABC_E500, 7)]);
    let mut d = open(&mut h, 0, DEPT_SCHOOLS);
    assert_eq!(d.mode(), SubBudgetMode::Individual(DEPT_SCHOOLS));
    assert_eq!(d.service_kind(), KIND_SCHOOL);
    assert!(h.has("occupants 0x4a538cc6"));
    h.slider_pos = 75;
    h.log.clear();
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Slider(0x0ABC_E500)));
    assert_eq!(
        h.log,
        [
            "qi 7",
            "occfund 7 0x3f000000 0x3f400000 true",
            "rel_occ 7",
            "occupants 0x4a538cc6",
            "empty"
        ]
    );
    h.log.clear();
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Slider(0x0ABC_E4FF));
    assert_eq!(h.log[1], "occfund 7 0x3f400000 0x3e800000 true");
    // No item for the control: nothing but the guard.
    h.log.clear();
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Slider(0x0ABC_E501));
    assert!(h.log.is_empty());
    for (dept, which, want) in [
        (DEPT_HEALTH, false, Some("occfund 7 0x3f400000 0x3e800000 false")),
        (DEPT_POLICE, false, Some("police 7 0x3f400000")),
        (DEPT_JAIL, true, None),
        (DEPT_FIRE, false, Some("fire 7 0x3f400000")),
        (DEPT_FIRE, true, None),
        (DEPT_POWER, false, Some("power 7 0x3f400000")),
        (0x1234, false, None),
    ] {
        let mut h2 = Rec {
            query: h.query,
            ..Default::default()
        };
        let d = open(&mut h2, 0, dept);
        h2.log.clear();
        assert_eq!(d.set_occupant_funding_level(&mut h2, 7, which, 75.0), want.is_some());
        let mut log = vec!["qi 7".to_string()];
        log.extend(want.map(String::from));
        log.push("rel_occ 7".into());
        assert_eq!(h2.log, log, "{dept:#x}");
    }
    // Query data fails: false, holder released. Not an occupant: false, nothing else.
    h.query = None;
    h.log.clear();
    assert!(!d.set_occupant_funding_level(&mut h, 7, false, 1.0));
    assert_eq!(h.log, ["qi 7", "rel_occ 7"]);
    h.not_occupant = true;
    h.log.clear();
    assert!(!d.set_occupant_funding_level(&mut h, 7, false, 1.0));
    assert_eq!(h.log, ["qi 7"]);
    // Cancel restores both levels, unscaled percent in, scaled at the call.
    let mut h = Rec {
        query: Some(FundingPair::default()),
        restore: vec![IndividualRestoreInfo {
            occupant: 9,
            level0: 50.0,
            level1: 100.0,
        }],
        ..Default::default()
    };
    let mut d = open(&mut h, 0, DEPT_HEALTH);
    // RestoreStartSettings is settled; production service-value derivation
    // remains explicitly Unsupported BW17. Seed its specified precondition.
    let snapshot = h.restore[0];
    d.retain_individual_restore(&mut h, snapshot);
    h.log.clear();
    assert!(d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_CANCEL)));
    let f: Vec<_> = h.log.iter().filter(|s| s.starts_with("occfund")).cloned().collect();
    assert_eq!(f, ["occfund 9 0x3f000000 0x0 false", "occfund 9 0x0 0x3f800000 false"]);
}

/// §5.7: camera jump only in individual mode.
#[test]
fn camera_jump_in_individual_mode() {
    let mut h = Rec::default();
    h.by_id.insert(0x0ABC_DE05, 4);
    let mut d = open(&mut h, 0, DEPT_POLICE);
    h.log.clear();
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(0x0ABC_DE05)));
    assert_eq!(h.log, ["qi 4", "jump [1.0, 2.0, 3.0]", "rel_occ 4"]);
    h.not_occupant = true;
    h.log.clear();
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(0x0ABC_DE05));
    assert_eq!(h.log, ["qi 4"]);
    let mut h2 = group_host();
    h2.by_id.insert(0x0ABC_DE05, 4);
    let mut g = open(&mut h2, GROUP_UTILITIES, 0);
    h2.log.clear();
    g.do_win_msg(&mut h2, WIN, SubBudgetMsg::Button(0x0ABC_DE05));
    assert!(h2.log.is_empty());
}

/// §9 test 8, §2 nested dialog.
#[test]
fn nested_individual_dialog() {
    let mut h = group_host();
    h.by_id.insert(0x0ABC_E601, 3);
    h.item_dept.insert(3, DEPT_FIRE);
    h.modal_flag = true;
    h.script = vec![
        SubBudgetMsg::Init,
        SubBudgetMsg::Button(0x0ABC_E601),
        SubBudgetMsg::Button(BTN_OK),
    ];
    h.nested_script = vec![SubBudgetMsg::Init, SubBudgetMsg::Button(BTN_OK)];
    let mut p = BudgetPanel::new(6);
    p.do_sub_budget_window(&mut h, GROUP_UTILITIES);
    // Results: outer init, nested init, nested OK, outer row button, outer OK.
    assert_eq!(h.results, [true, true, true, false, true]);
    assert_eq!(h.paused_in_modal, [1, 2, 2, 1, 1]);
    assert_eq!(h.pause_msgs, [true, false]);
    assert!(h.has("occupants 0x28f55a9f"));
    assert_eq!(h.count("balance"), 1);
    assert_eq!(h.count("restore_loc"), 1);
    assert_eq!(h.count("focus"), 1);
    assert_eq!(h.count(&format!("release {NESTED_WIN}")), 2);
    // A null item: nothing opens.
    let mut h = group_host();
    let mut d = open(&mut h, GROUP_UTILITIES, 0);
    h.log.clear();
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(0x0ABC_E602)));
    assert!(h.log.is_empty() && d.nested().is_none());
}

#[test]
fn panel_refreshes_at_most_once_per_tick() {
    let mut h = Rec::default();
    let mut p = BudgetPanel::new(0);
    p.do_message(&mut h, PANEL_MSG_TICK);
    assert_eq!(h.count("balance"), 0);
    p.do_message(&mut h, PANEL_MSG_MONEY_A);
    p.do_message(&mut h, PANEL_MSG_MONEY_B);
    assert!(p.is_dirty());
    p.do_message(&mut h, PANEL_MSG_TICK);
    p.do_message(&mut h, PANEL_MSG_TICK);
    assert_eq!(h.count("balance"), 1);
}

fn arrows(h: &Rec) -> Vec<String> {
    h.log.iter().filter(|s| s.starts_with("arrow ")).cloned().collect()
}

/// §9 test 14: Alt (bit 0x4) with Enter / F4 closes; other masks do not.
#[test]
fn alt_enter_and_alt_f4_close() {
    for key in [KEY_ENTER, KEY_0X73] {
        for (mask, closes) in [
            (0x4, true),
            (0x5, true),
            (0x4000_0004, true),
            (0x1, false),
            (0x2, false),
            (0, false),
        ] {
            let mut h = group_host();
            let mut d = open(&mut h, GROUP_UTILITIES, 0);
            let before = h.log.len();
            let r = d.do_win_msg(&mut h, WIN, SubBudgetMsg::KeyDown { key, modifiers: mask });
            assert_eq!(r, closes, "key {key:#x} mask {mask:#x}");
            assert_eq!(h.log.len() > before, closes, "key {key:#x} mask {mask:#x}");
        }
    }
}

/// §9 test 15 (BW10a): the content window `+0x10` in every mode.
#[test]
fn content_window_created_in_every_mode() {
    for (selector, dept) in [
        (SELECTOR_ORDINANCES, 0),
        (SELECTOR_NEIGHBORS, 0),
        (GROUP_UTILITIES, 0),
        (0, 0x2B),
    ] {
        let mut h = group_host();
        let mut d = open(&mut h, selector, dept);
        assert_eq!(d.content(), Some(CONTENT));
        let start = h.pos("content_create");
        let mut want = vec!["content_create".to_string(), format!("addref {CONTENT}")];
        want.extend(
            CONTENT_GENERATOR_FLAGS
                .iter()
                .map(|(f, on)| format!("genflag {f:#x} {on}")),
        );
        want.extend(CONTENT_WIN_FLAGS.iter().map(|(f, on)| format!("winflag {f:#x} {on}")));
        want.extend([
            "id 0x423278e".to_string(),
            format!("child_add {WIN} {CONTENT}"),
            format!("release_gen {CONTENT}"),
        ]);
        assert_eq!(h.log[start..start + want.len()], want[..], "selector {selector}");
        // Before the selector branch.
        if let Some(bg) = h.log.iter().position(|s| s.starts_with("bg ")) {
            assert!(start < bg);
        }
        assert!(d.on_shutdown(&mut h));
        assert_eq!(d.content(), None);
        assert!(h.pos(&format!("release {WIN}")) < h.pos(&format!("release {CONTENT}")));
    }
    // Create fails: +0x10 stays null, no ChildAdd, the mode is still built
    // (literal reading, BW13).
    let mut h = group_host();
    h.content_fails = true;
    let d = open(&mut h, GROUP_UTILITIES, 0);
    assert_eq!(d.content(), None);
    assert!(h.count("child_add") == 0 && h.count("id 0x423278e") == 0);
    assert!(d.is_initialised() && h.has("bg 0x140155d2"));
}

/// The destructor releases `+0x14`, `+0x10`, `+0x0C` in that order.
#[test]
fn release_final_releases_content() {
    let mut h = group_host();
    let mut d = open(&mut h, GROUP_UTILITIES, 0);
    let start = h.log.len();
    d.release_final(&mut h);
    assert_eq!(h.log[start..], [format!("release {CONTENT}"), format!("release {WIN}")]);
}

/// §9 test 16 (BW10b): an all-hidden group keeps one saved pair per
/// department, no slider rows and no arrows.
#[test]
fn empty_group_keeps_saved_pairs() {
    let mut h = Rec {
        group: vec![1, 2],
        ..Default::default()
    };
    h.funding.extend([(1, 0.25f32), (2, 0.5)]);
    for d in 1..=2 {
        h.items.insert(d, vec![item(2, 101, 1, 0, 0)]);
    }
    let mut d = open(&mut h, GROUP_CIVIC, 0);
    assert_eq!((d.list_a().len, d.list_b().len), (0, 0));
    assert!(d.sliders().is_empty());
    assert_eq!(d.saved_fractions(), &[(0.25f32, 1), (0.5, 2)]);
    assert!(h.count("button ") == 0 && arrows(&h).is_empty());
    // A later restore writes each saved fraction back unchanged.
    d.restore_start_settings(&mut h);
    let fund: Vec<_> = h.log.iter().filter(|s| s.starts_with("fund ")).cloned().collect();
    assert_eq!(
        fund,
        [
            format!("fund 1 {:#x} 0", 0.25f32.to_bits()),
            format!("fund 2 {:#x} 0", 0.5f32.to_bits())
        ]
    );
    // A group with no departments has no pairs.
    let mut h = Rec::default();
    let d = open(&mut h, GROUP_CIVIC, 0);
    assert!(d.saved_fractions().is_empty());
}

/// §9 test 17 (BW10c): group mode, A = 9, B = 10.
#[test]
fn group_initial_arrows() {
    let mut h = Rec {
        group: vec![1],
        ..Default::default()
    };
    h.funding.insert(1, 0.5);
    let mut items: Vec<_> = (0..8).map(|i| item(1, 200 + i, 1, 1, 0)).collect();
    items.extend((0..9).map(|i| item(2, 300 + i, 1, 0, 1)));
    h.items.insert(1, items);
    let d = open(&mut h, GROUP_UTILITIES, 0);
    assert_eq!((d.list_a().len, d.list_b().len), (9, 10));
    let mut want = Vec::new();
    for (id, img) in [
        (0x451u32, 0x140155cbu32),
        (0x452, 0x140155cc),
        (0x453, 0x140155cb),
        (0x454, 0x140155cc),
    ] {
        want.push(format!("button {id:#x} {img:#x}"));
        want.push(format!("target {} {WIN}", 1000 + id));
    }
    let start = h.pos("button 0x451 0x140155cb");
    assert_eq!(h.log[start..start + 8], want[..]);
    assert_eq!(
        arrows(&h),
        ["arrow 0x451 false", "arrow 0x452 false", "arrow 0x453 false"]
    );
}

/// §9 test 18 (BW12) and the boundary table: ordinance bound `< 9`,
/// group and neighbour `<= 9`, unsigned counts.
#[test]
fn selector_initial_arrows() {
    let ord = |a: u32, b: u32| {
        let mut h = Rec::default();
        let mut d = open(&mut h, SELECTOR_ORDINANCES, 0);
        d.set_list_len(false, a);
        d.set_list_len(true, b);
        h.log.clear();
        d.create_scroll_arrows(&mut h, WIN, false);
        arrows(&h)
    };
    let nb = |a: u32, b: u32| {
        let mut h = Rec::default();
        let mut d = open(&mut h, SELECTOR_NEIGHBORS, 0);
        d.set_list_len(false, a);
        d.set_list_len(true, b);
        h.log.clear();
        d.create_scroll_arrows(&mut h, WIN, true);
        assert_eq!(h.count("button "), 4);
        arrows(&h)
    };
    assert_eq!(
        ord(9, 8),
        ["arrow 0x451 false", "arrow 0x453 false", "arrow 0x454 false"]
    );
    assert_eq!(
        nb(9, 8),
        [
            "arrow 0x451 false",
            "arrow 0x452 false",
            "arrow 0x453 false",
            "arrow 0x454 false"
        ]
    );
    let all = [
        "arrow 0x451 false",
        "arrow 0x452 false",
        "arrow 0x453 false",
        "arrow 0x454 false",
    ];
    let ups = ["arrow 0x451 false", "arrow 0x453 false"];
    for (n, ord_hidden, nb_hidden) in [
        (0u32, true, true),
        (8, true, true),
        (9, false, true),
        (10, false, false),
        (0x8000_0000, false, false),
    ] {
        assert_eq!(ord(n, n), if ord_hidden { &all[..] } else { &ups[..] }, "ordinance {n}");
        assert_eq!(nb(n, n), if nb_hidden { &all[..] } else { &ups[..] }, "neighbour {n}");
    }
}

fn ownership_host() -> Rec {
    let mut host = group_host();
    host.items.insert(1, vec![item(1, 101, 5, 25, 0)]);
    host.items
        .insert(2, vec![item(2, 101, 5, 0, 25), item(2, 102, 5, 0, 25)]);
    host.items
        .insert(3, (0..12).map(|i| item(2, 101 + i, 5, 0, 25)).collect());
    host
}

fn department_counts(refs: &RefLedger) -> Vec<(DeptRef, i32)> {
    let mut counts: Vec<_> = refs.depts.lock().unwrap().iter().map(|(&d, &n)| (d, n)).collect();
    counts.sort_unstable();
    counts
}

fn department_releases(refs: &RefLedger) -> Vec<DeptRef> {
    refs.events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|&(op, d)| (op == '-').then_some(d))
        .collect()
}

/// §9 tests 20-21: no departments, all hidden, vector growth, null refs.
#[test]
fn saved_pairs_own_references_even_without_visible_rows() {
    let mut h = Rec::default();
    let d = open(&mut h, GROUP_CIVIC, 0);
    assert!(d.sliders().is_empty() && d.saved_fractions().is_empty());
    assert!(department_counts(&h.refs).is_empty());
    drop(d);

    for shutdown in [false, true] {
        let mut h = Rec {
            group: (1..=64).collect(),
            ..Default::default()
        };
        for dept in 1..=64 {
            h.funding.insert(dept, 0.5);
            h.items.insert(dept, vec![item(2, 101, 4, 0, 25)]);
        }
        let mut d = open(&mut h, GROUP_CIVIC, 0);
        assert_eq!(d.saved_fractions().len(), 64);
        assert!(d.sliders().is_empty());
        assert_eq!(department_counts(&h.refs), (1..=64).map(|d| (d, 1)).collect::<Vec<_>>());
        // Exact temporary / stored copy / temporary release for each pair;
        // moving Rust Vec elements requires no extra reference operations.
        let events = h.refs.events.lock().unwrap().clone();
        assert_eq!(
            events,
            (1..=64).map(|d| ('+', d)).chain((1..=64)
                .flat_map(|d| [('+', d), ('+', d), ('-', d)])
            ).chain((1..=64).map(|d| ('-', d)))
                .collect::<Vec<_>>()
        );
        if shutdown {
            d.on_shutdown(&mut h);
        }
        let refs = h.refs.clone();
        drop(h); // the reference provider must outlive the host
        drop(d);
        assert!(department_counts(&refs).iter().all(|&(_, n)| n == 0));
    }

    let mut h = Rec {
        group: vec![0],
        ..Default::default()
    };
    h.funding.insert(0, 0.5);
    let d = open(&mut h, GROUP_CIVIC, 0);
    assert_eq!(d.saved_fractions(), &[(0.5, 0)]);
    drop(d);
    assert!(department_counts(&h.refs).is_empty());
}

/// §9 tests 22-24: exact per-row ownership, shared cap, repeated init,
/// shutdown order and destructor order. A slider ID owns no window ref.
#[test]
fn department_ownership_reinit_shutdown_and_drop() {
    for shutdown in [false, true] {
        let mut h = ownership_host();
        let mut d = open(&mut h, GROUP_UTILITIES, 0);
        let rows = [2, 2, 2, 3, 3, 3, 3, 3, 3];
        assert_eq!(d.sliders().iter().map(|s| s.dept).collect::<Vec<_>>(), rows);
        assert_eq!(department_counts(&h.refs), [(1, 1), (2, 4), (3, 7)]);
        assert_eq!((d.list_a().len, d.list_b().len), (2, 16));
        let mut ids = std::collections::HashSet::new();
        for (&(win, id, dept), slider) in h.slider_children.iter().zip(d.sliders()) {
            assert_eq!(win, WIN);
            assert_eq!((id, dept), (slider.slider, slider.dept));
            assert!(ids.insert(id));
        }
        assert_eq!(ids.len(), 9);
        // Pair references precede slider references. Each append has the
        // same temporary+copy+release sequence, irrespective of capacity.
        let expected = [1, 2, 3].into_iter().map(|d| ('+', d)).chain([1, 2, 3]
            .into_iter()
            .chain(rows)
            .flat_map(|dept| [('+', dept), ('+', dept), ('-', dept)]))
            .chain([1, 2, 3].into_iter().map(|d| ('-', d)))
            .collect::<Vec<_>>();
        assert_eq!(*h.refs.events.lock().unwrap(), expected);
        let before = h.refs.events.lock().unwrap().clone();
        let saved = d.saved_fractions().to_vec();
        let sliders = d.sliders().to_vec();
        assert!(d.on_init(&mut h, Some(WIN)));
        assert_eq!(*h.refs.events.lock().unwrap(), before);
        assert_eq!(d.saved_fractions(), saved);
        assert_eq!(d.sliders(), sliders);
        assert_eq!(h.paused, 1);

        h.refs.events.lock().unwrap().clear();
        if shutdown {
            assert!(d.on_shutdown(&mut h));
            assert_eq!(
                department_releases(&h.refs),
                rows.into_iter().chain([1, 2, 3]).collect::<Vec<_>>()
            );
            assert_eq!(department_counts(&h.refs), [(1, 0), (2, 0), (3, 0)]);
            let after = h.refs.events.lock().unwrap().clone();
            assert!(d.on_shutdown(&mut h));
            assert_eq!(*h.refs.events.lock().unwrap(), after);
            assert!(d.on_init(&mut h, Some(WIN)));
            assert_eq!(department_counts(&h.refs), [(1, 1), (2, 4), (3, 7)]);
            assert_eq!(d.sliders().iter().map(|s| s.dept).collect::<Vec<_>>(), rows);
            d.on_shutdown(&mut h);
            h.refs.events.lock().unwrap().clear();
        }
        let refs = h.refs.clone();
        drop(h);
        drop(d);
        assert_eq!(department_counts(&refs), [(1, 0), (2, 0), (3, 0)]);
        if shutdown {
            assert!(refs.events.lock().unwrap().is_empty());
        } else {
            assert_eq!(
                department_releases(&refs),
                [1, 2, 3].into_iter().chain(rows).collect::<Vec<_>>()
            );
            let events = refs.events.lock().unwrap();
            assert_eq!(&events[events.len() - 2..], &[('w', CONTENT), ('w', WIN)]);
        }
    }
}

#[test]
fn explicit_final_release_then_drop_is_idempotent() {
    let mut h = ownership_host();
    let mut d = open(&mut h, GROUP_UTILITIES, 0);
    h.refs.events.lock().unwrap().clear();
    d.release_final(&mut h);
    assert_eq!(department_releases(&h.refs), [1, 2, 3, 2, 2, 2, 3, 3, 3, 3, 3, 3]);
    let events = h.refs.events.lock().unwrap().clone();
    d.release_final(&mut h);
    d.on_shutdown(&mut h);
    drop(d);
    assert_eq!(*h.refs.events.lock().unwrap(), events);
    assert_eq!(h.paused, 1); // destructor and shutdown never HiddenResume
    assert_eq!(department_counts(&h.refs), [(1, 0), (2, 0), (3, 0)]);
}

/// §9 tests 19 and 25 (BW13): deliberate safe continuation on null content.
#[test]
fn failed_content_keeps_pause_and_department_ownership() {
    let mut h = ownership_host();
    h.content_fails = true;
    let mut d = open(&mut h, GROUP_UTILITIES, 0);
    assert!(d.is_initialised());
    assert_eq!(d.content(), None);
    assert!(h.painted.is_empty()); // no child insertions into missing content
    assert_eq!(d.saved_fractions().len(), 3);
    assert_eq!(
        d.sliders().iter().map(|s| s.dept).collect::<Vec<_>>(),
        [2, 2, 2, 3, 3, 3, 3, 3, 3]
    );
    assert!(h.slider_children.iter().all(|&(parent, _, _)| parent == WIN));
    assert_eq!(department_counts(&h.refs), [(1, 1), (2, 4), (3, 7)]);
    assert_eq!(h.paused, 1);
    assert!(d.destroy(&mut h));
    assert_eq!(h.paused, 0);
    d.on_shutdown(&mut h);
    drop(d);
    assert_eq!(department_counts(&h.refs), [(1, 0), (2, 0), (3, 0)]);
}

/// §9 tests 26-27: the same predicate drives counts and built rows, for
/// both lists, flags 5/4/1, and IDs just below/at/above the boundary.
#[test]
fn bw16_id_boundary_counts_and_built_rows() {
    for income in [true, false] {
        for flags in [5, 4, 1] {
            let mut h = Rec {
                group: vec![1],
                ..Default::default()
            };
            h.funding.insert(1, 0.5);
            let items = [0, 25]
                .into_iter()
                .flat_map(|amount| {
                    [99, 100, 101].map(|id| {
                        item(
                            if income { 1 } else { 2 },
                            id,
                            flags,
                            if income { amount } else { 0 },
                            if income { 0 } else { amount },
                        )
                    })
                })
                .collect();
            h.items.insert(1, items);
            let d = open(&mut h, GROUP_UTILITIES, 0);
            let list = if income { d.list_a() } else { d.list_b() };
            assert_eq!(list.len, if flags == 4 { 1 } else { 6 });
            let ids = h
                .painted
                .iter()
                .filter_map(|&(inc, dept, id, _)| {
                    assert_eq!((inc, dept), (income, 1));
                    id
                })
                .collect::<Vec<_>>();
            assert_eq!(
                ids,
                if flags == 4 {
                    vec![]
                } else {
                    vec![99, 100, 99, 100, 101]
                }
            );
            assert_eq!(d.sliders().len(), if income { 0 } else { list.len as usize });
        }
    }
    // The test is on a full signed 64-bit amount; either nonzero half counts.
    for kind in [1, 2, u32::MAX] {
        for amount in [1_i64 << 32, -(1_i64 << 32), i64::MIN, -1, i64::MAX] {
            assert!(line_item_shown(&item(kind, 101, 1, amount, amount)));
        }
    }
    assert!(!line_item_shown(&item(2, 101, 1, 25, 0)));
    assert!(!line_item_shown(&item(1, 101, 1, 0, 25)));
}
