// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Tests from budget-window.md §9 plus the routing edge cases of §2-§7.

use std::collections::HashMap;

use super::ids::*;
use super::*;

const WIN: WinRef = 77;
const NESTED_WIN: WinRef = 78;

#[derive(Clone, Copy, Debug, Default)]
struct Deal {
    flags: u32,
    amount: i32,
    last: i32,
    inc: i32,
    last_inc: i32,
}

/// A recording host. `script` is what `do_modal_win` delivers to the first
/// dialog, `nested_script` to the nested one.
#[derive(Default)]
struct Rec {
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
    restore: Vec<IndividualRestoreInfo>,
    by_id: HashMap<u32, ItemRef>,
    item_dept: HashMap<ItemRef, u32>,
    not_occupant: bool,
    query: Option<FundingPair>,
    slider_missing: bool,
    slider_pos: i32,
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
    fn init_ordinance_budget(&mut self, _: WinRef) -> OrdinanceInit {
        self.ord.clone()
    }
    fn update_ordinance_budget(&mut self, _: WinRef, _: &[OrdinanceRow], v: ScrollView) {
        self.l(format!("upd_ord {} {}", v.a, v.b));
    }
    fn set_ordinance_on(&mut self, o: OrdId, on: u32) {
        self.l(format!("ord {o} {on}"));
    }
    fn check_box_state(&mut self, _: WinRef, id: u32) -> Option<u8> {
        self.checks.get(&id).copied()
    }
    fn open_ordinance_info(&mut self, _: WinRef, k: Option<u32>, o: Option<OrdId>) -> Option<WinRef> {
        self.l(format!("info {k:?} {o:?}"));
        Some(500)
    }
    fn set_neighbor_ui_disable(&mut self, on: bool) {
        self.ui_disabled = on;
        self.l(format!("uidisable {on}"));
    }
    fn init_neighbor_budget(&mut self, _: WinRef) -> Vec<DealRef> {
        self.deal_list.clone()
    }
    fn update_neighbor_budget(&mut self, _: WinRef, _: &[DealRef], v: ScrollView) {
        self.l(format!("upd_nb {} {}", v.a, v.b));
    }
    fn rewrite_selected_deal_rows(&mut self, _: WinRef, c: u32) {
        self.l(format!("combo {c}"));
    }
    fn add_ref_deal(&mut self, d: DealRef) {
        self.l(format!("addref_deal {d}"));
    }
    fn release_deal(&mut self, d: DealRef) {
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
    fn set_empty_window(&mut self, _: WinRef) -> Option<WinRef> {
        self.l("empty".into());
        Some(600)
    }
    fn setup_window_big(&mut self, _: WinRef, a: u32, b: u32) {
        self.l(format!("big {a} {b}"));
    }
    fn funding_percentage(&mut self, d: DeptRef) -> f32 {
        self.funding[&d]
    }
    fn set_funding_percentage(&mut self, d: DeptRef, f: f32, a: u32) {
        self.funding.insert(d, f);
        self.l(format!("fund {d} {:#x} {a}", f.to_bits()));
    }
    fn add_department_row(&mut self, _: WinRef, _: u32, d: DeptRef) -> u32 {
        1000 + d as u32
    }
    fn update_values(&mut self, _: WinRef) {
        self.l("values".into());
    }
    fn init_individual_budget(&mut self, _: WinRef, d: u32) -> Vec<IndividualRestoreInfo> {
        self.l(format!("init_indiv {d:#x}"));
        self.restore.clone()
    }
    fn find_slider(&mut self, _: WinRef, _: u32) -> bool {
        !self.slider_missing
    }
    fn slider_value(&mut self, _: WinRef, _: u32) -> i32 {
        self.slider_pos
    }
    fn indiv_item(&mut self, _: WinRef, id: u32) -> Option<ItemRef> {
        self.by_id.get(&id).copied()
    }
    fn item_id(&mut self, i: ItemRef) -> u32 {
        self.item_dept[&i]
    }
    fn update_individual_budget(&mut self, _: WinRef, a: u32) {
        self.l(format!("upd_indiv {a}"));
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
    assert_eq!(d.sliders().len(), 3);
    h.slider_pos = 30;
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Slider(1002)));
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
    assert_eq!(d.inner_child(), Some(600));
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
    };
    h.checks.extend([(300, 1u8), (301, 1), (302, 0)]);
    let mut d = open(&mut h, 1, 0);
    assert!(h.has("bg 0x140155f2"));
    for id in [300, 301, 302] {
        assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(id)));
    }
    assert_eq!(h.count("upd_ord"), 3);
    // Row 300 → k 2 (42 on), 301 → k 0 (40 on), 302 → k 1 (41 off).
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_ORDINANCE_APPLY)));
    let ords: Vec<_> = h.log.iter().filter(|s| s.starts_with("ord ")).cloned().collect();
    assert_eq!(ords, ["ord 40 1", "ord 41 0", "ord 42 1"]);
    assert_eq!(h.count("upd_ord"), 3);

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
    assert_eq!(h3.count("upd_ord"), 1);
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
    assert!(h.has("info Some(0) Some(9)"));
    assert_eq!(d.inner_child(), Some(500));
    // A second info button closes the first popup.
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(500));
    assert!(h.has("release 500"));
    h.log.clear();
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_CLOSE_CHILD_ALT)));
    assert_eq!(h.log, ["remove_child Some(500)", "release 500"]);
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
    assert!(h.pos("combo 5") < h.pos("upd_nb 0 0"));
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
        },
        ..Default::default()
    };
    let mut d = open(&mut h, 1, 0);
    h.log.clear();
    assert!(d.do_win_msg(&mut h, WIN, SubBudgetMsg::Shutdown));
    assert_eq!(h.log, ["release 77"]);
    assert_eq!(d.window(), None);
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
    assert_eq!(h.log, ["arrow 0x453 false", "upd_ord 0 0"]);
    let mut h = Rec::default();
    let mut d = open(&mut h, 0, DEPT_FIRE);
    h.log.clear();
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_LIST_B_UP));
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_LIST_A_UP));
    assert_eq!(
        h.log,
        ["arrow 0x453 false", "upd_indiv 0", "arrow 0x451 false", "values"]
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
    assert!(h.has("init_indiv 0x4a538cc6"));
    h.slider_pos = 75;
    h.log.clear();
    assert!(!d.do_win_msg(&mut h, WIN, SubBudgetMsg::Slider(0x0ABC_E500)));
    assert_eq!(
        h.log,
        [
            "qi 7",
            "occfund 7 0x3f000000 0x3f400000 true",
            "rel_occ 7",
            "upd_indiv 0"
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
    assert!(h.has("init_indiv 0x28f55a9f"));
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
