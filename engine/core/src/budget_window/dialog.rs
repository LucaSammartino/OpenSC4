// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4SubBudgetWindow` (budget-window.md §3-§6).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::ids::*;
use super::{
    BudgetWindowHost, BudgetWindowReferences, DealRef, DeptRef, FundingPair, ItemRef, LineItem, OrdId, WinRef,
};

mod bodies;
pub use bodies::{individual_amount, BodyPaint, ComboError, DealRow, SubBudgetSources, Unsupported};

/// One group-mode slider row (`SliderInfo`, vector `+0x2C`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderInfo {
    pub slider: u32,
    pub dept: DeptRef,
}

/// One ordinance: its ID (`+0xA4`) and its byte pair (`+0xA8`: `[2k]`
/// original, `[2k+1]` edited).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrdinanceRow {
    pub ordinance: OrdId,
    pub original: u8,
    pub edited: u8,
}

/// What `initOrdinanceBudget` leaves in the dialog: the ordinances (count
/// `+0xA0`) and the row map `+0x74` (button `300 + r` / `500 + r` → `k`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OrdinanceInit {
    pub rows: Vec<OrdinanceRow>,
    pub row_map: Vec<u32>,
    /// How many ordinances the helper counted into list A / list B by the
    /// ordinance's own test (`0x0077C734`/`0x0077C73C`, BW12). The helper
    /// only increments `+0x124` / `+0x128`; it never resets them.
    pub added_a: u32,
    pub added_b: u32,
}

/// What `initNeighborBudget` leaves in the dialog: the held deals (vector
/// `+0x68`) and the list counts it wrote (it zeroes `+0x124` / `+0x128`
/// itself, then counts rows and section headers; BW12).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NeighborInit {
    pub deals: Vec<DealRef>,
    pub count_a: u32,
    pub count_b: u32,
}

/// `IndividualRestoreInfo` (12 bytes: item, level0 f32, level1 f32, in
/// percent; vector `+0x44`; §5.1, BW8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IndividualRestoreInfo {
    pub occupant: ItemRef,
    pub level0: f32,
    pub level1: f32,
}

/// A scrolled row list (§5.3): position and row count, both u32.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollList {
    pub pos: u32,
    pub len: u32,
}

/// The two list positions handed to the mode refresh.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollView {
    pub a: u32,
    pub b: u32,
}

/// The dialog mode, in the order `on_init` tests it (§4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubBudgetMode {
    Ordinances,
    Neighbors,
    /// `+0x24 ≠ 0`: the department ID.
    Individual(u32),
    Group(u32),
}

/// The messages `DoWinMsg` routes (§4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubBudgetMsg {
    Init,
    Shutdown,
    Button(u32),
    /// Slider moved: the control ID (the position is read from the slider).
    Slider(u32),
    Combo(u32),
    KeyDown {
        key: u32,
        modifiers: u32,
    },
    Other(u32),
}

/// §4.4: the group-mode fraction, `f32(position) × 0.01f` rounded once. On
/// the x87 the product of the integer and the f32 constant is exact before the
/// single store; the f64 product here is exact too for `|position| < 2^29`.
pub fn slider_fraction(position: i32) -> f32 {
    (f64::from(position) * f64::from(FUNDING_SCALE)) as f32
}

/// §4.4: the property is owned and released; its object result is borrowed.
pub fn get_window_indiv_item<H: BudgetWindowHost + ?Sized>(host: &mut H, win: WinRef, id: u32) -> Option<ItemRef> {
    let child = host.find_child(win, id)?;
    let property = host.get_property(child, 0x4BFEE088)?;
    let item = if host.property_type(property) == 0xF { host.property_object(property) } else { None };
    host.release_property(property);
    item
}

/// §5.5: the service kind `+0x134` `initIndividualBudget` sets from the
/// department ID `+0x24`.
pub fn service_kind(department: u32) -> u32 {
    match department {
        DEPT_HEALTH | DEPT_HEALTH_B => KIND_HOSPITAL,
        DEPT_EDUCATION | DEPT_SCHOOLS => KIND_SCHOOL,
        DEPT_POLICE | DEPT_JAIL => KIND_POLICE,
        DEPT_FIRE => KIND_FIRE,
        DEPT_POWER => KIND_POWER,
        _ => 0,
    }
}

/// §4.1: whether a department's header row is added to list A (income) or
/// B (expense). Only the three special groups test the department.
pub fn header_shown<H: BudgetWindowHost + ?Sized>(host: &mut H, group: u32, dept: DeptRef, income: bool) -> bool {
    if !matches!(group, GROUP_PARKS_LANDMARKS | GROUP_CIVIC2 | GROUP_CIVIC) {
        return true;
    }
    if income {
        host.department_total_income(dept) != 0
    } else {
        host.department_ideal_monthly_funding(dept) != 0
    }
}

/// §4.1: whether a line item adds a row to its list.
pub fn line_item_shown(item: &LineItem) -> bool {
    let amount = if item.kind == LINE_ITEM_INCOME {
        item.income
    } else {
        item.full_expenses
    };
    item.display_flags & LINE_ITEM_SHOW != 0 && (item.id <= LINE_ITEM_ALWAYS_ID_MAX || amount != 0)
}

/// `end_dialog(win, code)` (`0x00779850`, §6): null → false; a modal window
/// (flag `0x1000`) hands `code` to its parent's end-modal call, any other is
/// removed from its parent (`code` unused). Returns true.
pub fn end_dialog<H: BudgetWindowHost + ?Sized>(host: &mut H, win: Option<WinRef>, code: u32) -> bool {
    let Some(win) = win else { return false };
    if host.win_flag(win, WIN_FLAG_MODAL) {
        host.end_modal_via_parent(win, code);
    } else {
        host.remove_from_parent(win);
    }
    true
}

/// §2 step 4 up to the generator flags: returns whether the window answered
/// the generator interface (a reference the caller releases after the run).
pub(crate) fn show_sequence<H: BudgetWindowHost + ?Sized>(host: &mut H, win: WinRef) -> bool {
    host.show_win(win);
    host.set_win_flag(win, 0x800, true);
    host.pull_to_front(win);
    host.set_win_flag(win, 0x800, false);
    host.set_win_flag(win, 0x100, true);
    let gen = host.query_generator(win);
    if gen {
        host.set_generator_flag(win, 0x1000, true);
        host.set_generator_flag(win, 0x2000, true);
        host.set_generator_flag(win, 0x4000, true);
    }
    gen
}

/// `cSC4SubBudgetWindow`.
#[derive(Debug)]
pub struct SubBudgetWindow {
    references: Option<Arc<dyn BudgetWindowReferences>>,
    /// `+0x08`: initialised.
    initialised: bool,
    /// `+0x0C`: the dialog window.
    window: Option<WinRef>,
    /// `+0x10`: the content window `on_init` creates (BW10a).
    content: Option<WinRef>,
    /// `+0x14`: the child (ordinance info popup or empty-group window).
    inner_child: Option<WinRef>,
    /// `+0x1C`: the nested individual dialog.
    nested: Option<Box<SubBudgetWindow>>,
    /// `+0x20`.
    selector: u32,
    /// `+0x24`: department ID (individual mode when non-zero).
    department: u32,
    /// `+0x28`: the panel's `+0x198` value.
    context: u32,
    /// `+0x134`.
    service_kind: u32,
    sliders: Vec<SliderInfo>,
    saved_fractions: Vec<(f32, DeptRef)>,
    restore: Vec<IndividualRestoreInfo>,
    restore_occupants: Vec<ItemRef>,
    ordinances: Vec<OrdinanceRow>,
    /// `+0xA0`: not reset by on_shutdown (§4).
    ordinance_count: u32,
    row_map: Vec<u32>,
    deals: Vec<DealRef>,
    deal_types: Vec<u32>,
    deal_rows: Vec<DealRow>,
    active_deals: Vec<(u32, u32)>,
    neighbor_totals: [i64; 6],
    neighbor_refresh_requests: u64,
    individual_visible: Vec<ItemRef>,
    individual_total: Option<i64>,
    group_total_a: Option<i64>,
    amount_scale: Option<i64>,
    refresh_busy: bool,
    unsupported: Vec<Unsupported>,
    list_a: ScrollList,
    list_b: ScrollList,
    /// The process-wide slider re-entry byte (`0x00B4DFC8`); share one
    /// guard between every dialog to get the process-wide behaviour.
    slider_guard: Arc<AtomicBool>,
}

impl Default for SubBudgetWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl SubBudgetWindow {
    /// Construct (`0x007862B0`) with a guard of its own.
    pub fn new() -> Self {
        Self::with_slider_guard(Arc::new(AtomicBool::new(false)))
    }

    pub fn with_slider_guard(slider_guard: Arc<AtomicBool>) -> Self {
        Self {
            references: None,
            initialised: false,
            window: None,
            content: None,
            inner_child: None,
            nested: None,
            selector: 0,
            department: 0,
            context: 0,
            service_kind: 0,
            sliders: Vec::new(),
            saved_fractions: Vec::new(),
            restore: Vec::new(),
            restore_occupants: Vec::new(),
            ordinances: Vec::new(),
            ordinance_count: 0,
            row_map: Vec::new(),
            deals: Vec::new(),
            deal_types: Vec::new(),
            deal_rows: Vec::new(),
            active_deals: Vec::new(),
            neighbor_totals: [0; 6],
            neighbor_refresh_requests: 0,
            individual_visible: Vec::new(),
            individual_total: None,
            group_total_a: None,
            amount_scale: None, // BW17: owner-scale initialization is unsettled.
            refresh_busy: false,
            unsupported: vec![Unsupported::ItemPropertySetter],
            list_a: ScrollList::default(),
            list_b: ScrollList::default(),
            slider_guard,
        }
    }

    pub fn is_initialised(&self) -> bool {
        self.initialised
    }
    pub fn window(&self) -> Option<WinRef> {
        self.window
    }
    pub fn selector(&self) -> u32 {
        self.selector
    }
    pub fn department(&self) -> u32 {
        self.department
    }
    pub fn context(&self) -> u32 {
        self.context
    }
    pub fn service_kind(&self) -> u32 {
        self.service_kind
    }
    pub fn content(&self) -> Option<WinRef> {
        self.content
    }
    pub fn inner_child(&self) -> Option<WinRef> {
        self.inner_child
    }
    pub fn nested(&self) -> Option<&SubBudgetWindow> {
        self.nested.as_deref()
    }
    pub fn sliders(&self) -> &[SliderInfo] {
        &self.sliders
    }
    pub fn saved_fractions(&self) -> &[(f32, DeptRef)] {
        &self.saved_fractions
    }
    pub fn restore_records(&self) -> &[IndividualRestoreInfo] {
        &self.restore
    }
    /// Every retained snapshot pointer, including BW17 records whose funding
    /// values are unresolved. Only resolved values appear in restore_records.
    pub fn restore_occupants(&self) -> &[ItemRef] { &self.restore_occupants }
    pub fn individual_viewport(&self) -> &[ItemRef] { &self.individual_visible }
    pub fn active_deal_pairs(&self) -> &[(u32, u32)] { &self.active_deals }
    pub fn neighbor_section_totals(&self) -> &[i64; 6] { &self.neighbor_totals }
    pub fn neighbor_refresh_requests(&self) -> u64 { self.neighbor_refresh_requests }
    pub fn ordinance_row_map(&self) -> &[u32] { &self.row_map }
    pub fn ordinances(&self) -> &[OrdinanceRow] {
        &self.ordinances
    }
    pub fn ordinance_count(&self) -> u32 {
        self.ordinance_count
    }
    pub fn deals(&self) -> &[DealRef] {
        &self.deals
    }
    pub fn list_a(&self) -> ScrollList {
        self.list_a
    }
    pub fn list_b(&self) -> ScrollList {
        self.list_b
    }

    /// The mode in `on_init`'s test order (§4): selector 1, selector 2,
    /// `+0x24` non-zero, otherwise a department group.
    pub fn mode(&self) -> SubBudgetMode {
        match (self.selector, self.department) {
            (SELECTOR_ORDINANCES, _) => SubBudgetMode::Ordinances,
            (SELECTOR_NEIGHBORS, _) => SubBudgetMode::Neighbors,
            (_, 0) => SubBudgetMode::Group(self.selector),
            (_, d) => SubBudgetMode::Individual(d),
        }
    }

    /// Test-only precondition for the scroll/arrow boundary cases.
    #[cfg(test)]
    pub(super) fn set_list_len(&mut self, list_b: bool, len: u32) {
        if list_b {
            self.list_b.len = len;
        } else {
            self.list_a.len = len;
        }
    }

    /// `CreateSubBudgetWindow(out, selector, arg3, department)`
    /// (`0x00779E50`, §3). Returns the window (the `*out` reference, AddRef'd)
    /// or `None` on failure.
    pub fn create<H: BudgetWindowHost + ?Sized>(
        &mut self,
        host: &mut H,
        selector: u32,
        context: u32,
        department: u32,
    ) -> Option<WinRef> {
        self.references = Some(host.references());
        let win = host.create_window_generator()?;
        host.add_ref_win(win);
        self.window = Some(win);
        for (flag, on) in CREATE_GENERATOR_FLAGS {
            host.set_generator_flag(win, flag, on);
        }
        host.set_generator_win_proc(win);
        for (flag, on) in CREATE_WIN_FLAGS {
            host.set_win_flag(win, flag, on);
        }
        host.set_win_id(win, DIALOG_WIN_ID);
        self.selector = selector;
        self.context = context;
        self.department = department;
        Some(win)
    }

    /// `DoWinMsg` (`0x0078BCA0`, §4) for window `win`: returns the message
    /// result (BW9).
    pub fn do_win_msg<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef, msg: SubBudgetMsg) -> bool {
        match msg {
            SubBudgetMsg::Init => self.on_init(host, Some(win)),
            SubBudgetMsg::Shutdown => self.on_shutdown(host),
            SubBudgetMsg::Button(id) => self.on_button_activated(host, win, id),
            SubBudgetMsg::Slider(control) => {
                self.on_slider(host, win, control);
                false
            }
            SubBudgetMsg::Combo(id) => {
                if self.mode() == SubBudgetMode::Neighbors {
                    let _ = self.edit_deal_rows(host, win, id);
                }
                false
            }
            SubBudgetMsg::KeyDown { key, modifiers } => {
                // §4.0 (BW5): bit 0x4 is Alt, so Alt+Enter / Alt+F4 close;
                // other bits are ignored. Whether those keystrokes reach the
                // dialog at runtime is BW11, outside the core.
                let close =
                    key == KEY_ESC || ((key == KEY_ENTER || key == KEY_0X73) && modifiers & KEY_CLOSE_MODIFIER != 0);
                if !close {
                    return false;
                }
                self.close_child(host, win);
                end_dialog(host, Some(win), END_CODE);
                true
            }
            SubBudgetMsg::Other(_) => false,
        }
    }

    /// `on_init(win)` (`0x007876B0`, §4).
    pub fn on_init<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: Option<WinRef>) -> bool {
        let Some(win) = win else { return false };
        if self.initialised {
            return true;
        }
        self.references = Some(host.references());
        host.hidden_pause();
        // The string loads between the pause and the content window are
        // layout (§8); nothing the core keeps.
        self.create_content(host, win);
        match self.mode() {
            SubBudgetMode::Ordinances => {
                host.load_background(win, BG_TYPE, BG_GROUP, BG_ORDINANCES);
                self.init_ordinances(host, win);
            }
            SubBudgetMode::Neighbors => {
                host.load_background(win, BG_TYPE, BG_GROUP, BG_NEIGHBORS);
                host.set_neighbor_ui_disable(true);
                self.init_neighbors(host, win);
            }
            SubBudgetMode::Individual(dept) => {
                self.service_kind = service_kind(dept);
                self.enumerate_individual(host, win, true);
            }
            SubBudgetMode::Group(group) => self.init_group(host, win, group),
        }
        self.initialised = true;
        true
    }

    /// §4 (BW10a): the content window `+0x10`, in every mode, before the
    /// selector branch.
    fn create_content<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef) {
        // BW13 deliberate safe divergence: skip this block on failure, retain
        // the hidden pause and continue the mode branch. Child insertions
        // into absent content are no-ops; sliders still belong to `win`.
        let Some(c) = host.create_content_window() else { return };
        // cRZAutoRefCount<cIGZWin>::operator=: the reference +0x10 holds.
        host.add_ref_win(c);
        self.content = Some(c);
        for (flag, on) in CONTENT_GENERATOR_FLAGS {
            host.set_generator_flag(c, flag, on);
        }
        for (flag, on) in CONTENT_WIN_FLAGS {
            host.set_win_flag(c, flag, on);
        }
        host.set_win_id(c, CONTENT_WIN_ID);
        host.child_add(win, c);
        host.release_generator(c);
    }

    /// §5.3 (BW10c, BW12): the four scroll arrows and their initial state.
    /// `inclusive`: hide a down arrow when its count is `<= 9` (group and
    /// neighbour modes); otherwise when `< 9` (the ordinance helper). Up
    /// arrows are always hidden; a down arrow not hidden is left as created.
    pub(super) fn create_scroll_arrows<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef, inclusive: bool) {
        for (id, image) in [
            (BTN_LIST_A_UP, ARROW_UP_IMAGE),
            (BTN_LIST_A_DOWN, ARROW_DOWN_IMAGE),
            (BTN_LIST_B_UP, ARROW_UP_IMAGE),
            (BTN_LIST_B_DOWN, ARROW_DOWN_IMAGE),
        ] {
            let button = host.add_image_button2(win, id, image);
            host.set_notification_target(button, win);
        }
        // Unsigned u32 compares with 9 (a count of 0x80000000 is large).
        let hide = |len: u32| {
            if inclusive {
                len <= LIST_ROWS_VISIBLE
            } else {
                len < LIST_ROWS_VISIBLE
            }
        };
        // Hide order 0x451, 0x452, 0x453, 0x454 follows the compare
        // addresses (0x452 before 0x454); the spec lists no other order.
        host.set_child_visible(win, BTN_LIST_A_UP, false);
        if hide(self.list_a.len) {
            host.set_child_visible(win, BTN_LIST_A_DOWN, false);
        }
        host.set_child_visible(win, BTN_LIST_B_UP, false);
        if hide(self.list_b.len) {
            host.set_child_visible(win, BTN_LIST_B_DOWN, false);
        }
    }

    /// §4, §4.1: group mode.
    fn init_group<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef, group: u32) {
        let depts = host.department_budgets_in_group(group);
        // BW20: literal shared provider contract with refresh (§6): returned
        // department interfaces own temporary references, released after use.
        let (mut a, mut b) = (0u32, 0u32);
        // BW14: clear old pairs, then temporary +1, stored copy +1,
        // temporary -1, before any visibility test. Rust vector growth moves
        // ownership rather than copy-constructing then destroying old pairs;
        // it preserves the specified one-reference-per-element invariant.
        self.clear_saved_fractions();
        for &dept in &depts {
            let f = host.funding_percentage(dept);
            self.add_ref_dept(dept);
            self.add_ref_dept(dept);
            self.saved_fractions.push((f, dept));
            self.release_dept(dept);
            let (mut first_a, mut first_b) = (true, true);
            for item in host.department_line_items(dept) {
                let income = item.kind == LINE_ITEM_INCOME;
                let first = if income { &mut first_a } else { &mut first_b };
                if std::mem::replace(first, false) && header_shown(host, group, dept, income) {
                    if income {
                        a += 1;
                    } else {
                        b += 1;
                    }
                }
                if line_item_shown(&item) {
                    if income {
                        a += 1;
                    } else {
                        b += 1;
                    }
                }
            }
        }
        self.list_a.len = a;
        self.list_b.len = b;
        host.load_background(win, BG_TYPE, BG_GROUP, BG_GROUP_MODE);
        if a == 0 && b == 0 {
            // BW10b: no slider rows, no arrows; +0x38 keeps its pairs.
            self.empty_body(host, win, None, 0x4A5CB4B6);
            for dept in depts { self.release_dept(dept); }
            return;
        }
        self.unsupported(Unsupported::Setup);
        host.paint_body(win, BodyPaint::Lists { a: a.min(LIST_ROWS_VISIBLE), b: b.min(LIST_ROWS_VISIBLE) });
        self.clear_sliders();
        // Separate A/B build loops, in department and line-item order, with
        // one shared nine-row cap per list. Only B rows own SliderInfo refs.
        for income in [true, false] {
            let mut row = 0;
            for &dept in &depts {
                let mut first = true;
                for item in host.department_line_items(dept) {
                    if (item.kind == LINE_ITEM_INCOME) != income {
                        continue;
                    }
                    if std::mem::replace(&mut first, false) && header_shown(host, group, dept, income) {
                        self.build_group_row(host, win, group, dept, None, income, &mut row);
                    }
                    if line_item_shown(&item) {
                        self.build_group_row(host, win, group, dept, Some(item), income, &mut row);
                    }
                }
            }
        }
        // BW10c: unclamped counts, inclusive bound.
        self.create_scroll_arrows(host, win, true);
        for dept in depts { self.release_dept(dept); }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_group_row<H: BudgetWindowHost + ?Sized>(
        &mut self,
        host: &mut H,
        win: WinRef,
        group: u32,
        dept: DeptRef,
        item: Option<LineItem>,
        income: bool,
        row: &mut u32,
    ) {
        if *row >= LIST_ROWS_VISIBLE {
            return;
        }
        if let Some(content) = self.content {
            host.paint_group_row(content, group, dept, item, income, *row);
        }
        if !income {
            let slider = host.create_department_slider(win, dept, item, *row);
            // BW15: temporary and copied element each AddRef; then drop the
            // temporary. The slider ID itself has no reference ownership.
            self.add_ref_dept(dept);
            self.add_ref_dept(dept);
            self.sliders.push(SliderInfo { slider, dept });
            self.release_dept(dept);
        }
        *row += 1;
    }

    fn add_ref_dept(&self, dept: DeptRef) {
        if dept != 0 {
            self.references
                .as_ref()
                .expect("reference provider before init")
                .add_ref_dept(dept);
        }
    }

    fn release_dept(&self, dept: DeptRef) {
        if dept != 0 {
            self.references
                .as_ref()
                .expect("reference provider before init")
                .release_dept(dept);
        }
    }

    fn clear_saved_fractions(&mut self) {
        for &(_, dept) in &self.saved_fractions {
            self.release_dept(dept);
        }
        self.saved_fractions.clear();
    }

    fn clear_sliders(&mut self) {
        for slider in &self.sliders {
            self.release_dept(slider.dept);
        }
        self.sliders.clear();
    }

    /// `on_shutdown` (`0x0077E440`, §4).
    pub fn on_shutdown<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H) -> bool {
        if !self.initialised {
            return true;
        }
        self.initialised = false;
        self.clear_sliders();
        self.clear_saved_fractions();
        self.clear_restore();
        self.clear_deal_rows();
        // Held deals are emptied without a per-deal Release.
        self.deals.clear();
        self.row_map.clear();
        // +0xA4 and +0xA8 freed; the count +0xA0 is kept.
        self.ordinances.clear();
        // The AddRef/Release of the object itself around this is a
        // lifetime guard of the native object; nothing to model.
        // Release +0x0C, then +0x10 (each nulled first).
        if let Some(w) = self.window.take() {
            host.release_win(w);
        }
        if let Some(c) = self.content.take() {
            host.release_win(c);
        }
        true
    }

    /// §4.4: slider moved.
    fn on_slider<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef, control: u32) {
        if !host.find_slider(win, control) {
            return;
        }
        if self.slider_guard.swap(true, Ordering::AcqRel) {
            return;
        }
        let position = host.slider_value(win, control);
        if self.department == 0 {
            let p = slider_fraction(position);
            for s in self.sliders.clone() {
                if s.slider == control {
                    host.set_funding_percentage(s.dept, p, 0);
                }
            }
            self.refresh_group(host, win);
        } else if let Some(item) = get_window_indiv_item(host, win, control) {
            let which = control >= OCCUPANT_LEVEL1_SLIDER_MIN;
            self.set_occupant_funding_level(host, item, which, position as f32);
            self.enumerate_individual(host, win, false);
        }
        self.slider_guard.store(false, Ordering::Release);
    }

    /// `SetOccupantFundingLevel(item, which, level)` (`0x0077BC90`, §5.5).
    pub fn set_occupant_funding_level<H: BudgetWindowHost + ?Sized>(
        &self,
        host: &mut H,
        item: ItemRef,
        which: bool,
        level: f32,
    ) -> bool {
        if !host.query_occupant(item) {
            return false;
        }
        let p = level * FUNDING_SCALE;
        let ok = match (self.service_kind, which) {
            (KIND_HOSPITAL | KIND_SCHOOL, _) => {
                let school = self.service_kind == KIND_SCHOOL;
                let rec = if school {
                    host.school_query_data(item)
                } else {
                    host.hospital_query_data(item)
                };
                match rec {
                    Some(mut r) => {
                        if which {
                            r.vehicle = p;
                        } else {
                            r.staff = p;
                        }
                        host.set_occupant_funding_percentages(item, FundingPair { ..r }, school);
                        true
                    }
                    None => false,
                }
            }
            (KIND_POLICE, false) => {
                host.police_set_station_funding(item, p);
                true
            }
            (KIND_FIRE, false) => {
                host.fire_set_station_funding(item, p);
                true
            }
            (KIND_POWER, false) => {
                host.power_set_plant_funding(item, p);
                true
            }
            _ => false,
        };
        host.release_occupant(item);
        ok
    }

    /// "Close the child" with the null test first (§5).
    fn close_child<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef) {
        if let Some(child) = self.inner_child.take() {
            host.remove_child(win, Some(child));
            host.release_win(child);
        }
    }

    /// Buttons `0x68` / `0x384`: `RemoveChild(+0x14)` without a null check,
    /// then the test (§5.6).
    fn close_child_unchecked<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef) {
        host.remove_child(win, self.inner_child);
        if let Some(child) = self.inner_child.take() {
            host.release_win(child);
        }
    }

    /// `RestoreStartSettings` (`0x0077C5E0`, §5.1).
    pub fn restore_start_settings<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H) {
        if self.department == 0 {
            for &(f, dept) in &self.saved_fractions {
                host.set_funding_percentage(dept, f, 0);
            }
        } else {
            for r in self.restore.clone() {
                self.set_occupant_funding_level(host, r.occupant, false, r.level0);
                self.set_occupant_funding_level(host, r.occupant, true, r.level1);
            }
        }
    }

    /// `ApplyDealEdits` (`0x0077AB40`, §5.4).
    pub fn apply_deal_edits<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H) {
        for &d in &self.deals {
            host.add_ref_deal(d);
            if host.deal_flags(d) & DEAL_ACTIVE != 0 {
                let last = host.deal_last_amount(d);
                if last != 0 {
                    host.deal_set_amount(d, last);
                    let inc = host.deal_last_inc_amount(d);
                    host.deal_set_inc_amount(d, inc);
                } else {
                    host.deal_cancel(d);
                    host.deal_set_last_amount(d, 0);
                    host.deal_set_last_inc_amount(d, 0);
                }
            }
            if host.deal_flags(d) & DEAL_CANCELLED != 0 && host.deal_last_amount(d) != 0 {
                let _ = host.deal_activate(d);
                let last = host.deal_last_amount(d);
                host.deal_set_amount(d, last);
                let inc = host.deal_last_inc_amount(d);
                host.deal_set_inc_amount(d, inc);
            }
            host.release_deal(d);
        }
    }

    /// §5.3: scroll buttons `0x451`..`0x454`, then the per-mode refresh.
    fn scroll<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef, id: u32) {
        let list_b = id >= BTN_LIST_B_UP;
        let down = id == BTN_LIST_A_DOWN || id == BTN_LIST_B_DOWN;
        let (up_id, down_id) = if list_b {
            (BTN_LIST_B_UP, BTN_LIST_B_DOWN)
        } else {
            (BTN_LIST_A_UP, BTN_LIST_A_DOWN)
        };
        let l = if list_b { &mut self.list_b } else { &mut self.list_a };
        let fits_after = |l: &ScrollList| l.pos.wrapping_add(SCROLL_PAGE) >= l.len;
        if down {
            if !fits_after(l) {
                l.pos = l.pos.wrapping_add(SCROLL_STEP);
            }
            let hide_down = fits_after(l);
            if hide_down {
                host.set_child_visible(win, down_id, false);
            }
            host.set_child_visible(win, up_id, true);
        } else {
            if l.pos > 0 {
                l.pos = l.pos.wrapping_sub(SCROLL_STEP);
            }
            let (at_top, more) = (l.pos == 0, !fits_after(l));
            if at_top {
                host.set_child_visible(win, up_id, false);
            }
            if more {
                host.set_child_visible(win, down_id, true);
            }
        }
        match self.selector {
            SELECTOR_ORDINANCES => self.unsupported(Unsupported::OrdinanceRefresh),
            SELECTOR_NEIGHBORS => self.request_neighbor_refresh(),
            _ if list_b && self.department != 0 => self.enumerate_individual(host, win, false),
            _ => { self.refresh_group(host, win); }
        }
    }

    /// `on_button_activated(win, id)` (`0x0078B120`, §5). No common tail.
    pub fn on_button_activated<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef, id: u32) -> bool {
        match id {
            BTN_OK | BTN_OK_ALT | BTN_END | BTN_END_ALT => {
                end_dialog(host, Some(win), END_CODE);
                true
            }
            BTN_CANCEL => {
                self.restore_start_settings(host);
                end_dialog(host, Some(win), END_CODE);
                true
            }
            BTN_CLOSE_CHILD | BTN_CLOSE_CHILD_ALT => {
                self.close_child_unchecked(host, win);
                false
            }
            BTN_ORDINANCE_APPLY => {
                for i in 0..self.ordinance_count as usize {
                    // After on_shutdown the count outlives the freed arrays;
                    // missing rows are skipped here.
                    if let Some(r) = self.ordinances.get(i) {
                        if r.edited != r.original {
                            host.set_ordinance_on(r.ordinance, u32::from(r.edited));
                        }
                    }
                }
                self.close_child(host, win);
                end_dialog(host, Some(win), END_CODE);
                false
            }
            BTN_CE => {
                self.close_child(host, win);
                end_dialog(host, Some(win), END_CODE);
                false
            }
            BTN_CF => {
                self.apply_deal_edits(host);
                self.close_child(host, win);
                end_dialog(host, Some(win), END_CODE);
                false
            }
            BTN_LIST_A_UP..=BTN_LIST_B_DOWN => {
                self.scroll(host, win, id);
                false
            }
            BTN_NESTED_ROW_FIRST..=BTN_NESTED_ROW_LAST => {
                if let Some(item) = get_window_indiv_item(host, win, id) {
                    let dept = host.item_id(item);
                    self.do_individual_budget_window(host, dept);
                }
                false
            }
            BTN_CAMERA_JUMP_FIRST..=BTN_CAMERA_JUMP_LAST if self.department != 0 => {
                if let Some(item) = get_window_indiv_item(host, win, id) {
                    if host.query_occupant(item) {
                        let v = host.occupant_position(item, [CAMERA_JUMP_FILL; 3]);
                        host.camera_jump(v);
                        host.release_occupant(item);
                    }
                }
                false
            }
            BTN_ORDINANCE_CHECK_FIRST..=BTN_ORDINANCE_CHECK_LAST => {
                let k = self.row_map.get((id - BTN_ORDINANCE_CHECK_FIRST) as usize).copied();
                if let Some(state) = host.check_box_state(win, id) {
                    // A row outside the map is outside the native arrays;
                    // no edit is made for it here.
                    if let Some(r) = k.and_then(|k| self.ordinances.get_mut(k as usize)) {
                        r.edited = state;
                    }
                }
                self.unsupported(Unsupported::OrdinanceRefresh);
                false
            }
            BTN_ORDINANCE_INFO_FIRST..=BTN_ORDINANCE_INFO_LAST => {
                self.close_child(host, win);
                let k = self.row_map.get((id - BTN_ORDINANCE_INFO_FIRST) as usize).copied();
                let ord = k.and_then(|k| self.ordinances.get(k as usize)).map(|r| r.ordinance);
                let _ = (k, ord); // Popup/factory ownership remains BW17.
                self.unsupported(Unsupported::OrdinanceInfo);
                false
            }
            _ => false,
        }
    }

    /// `DoIndividualBudgetWindow(department)` (`0x00786530`, §2): the nested
    /// dialog at `+0x1C`; no location save, no focus, no balance refresh.
    pub fn do_individual_budget_window<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, department: u32) {
        if self.nested.is_some() {
            return;
        }
        let context = self.context;
        let n = self
            .nested
            .insert(Box::new(SubBudgetWindow::with_slider_guard(self.slider_guard.clone())));
        if let Some(win) = n.create(host, 0, context, department) {
            let gen = show_sequence(host, win);
            host.do_modal_win(win, n);
            host.release_win(win);
            if gen {
                host.release_generator(win);
            }
        }
        n.destroy(host);
        if let Some(mut n) = self.nested.take() {
            n.release_final(host);
        }
    }

    /// `DestroySubBudgetWindow` (`0x0077A1A0`, §6). The pause ends before the
    /// window is ended; `+0x0C` is neither released nor cleared.
    pub fn destroy<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H) -> bool {
        if self.selector == SELECTOR_NEIGHBORS {
            for &deal in &self.deals {
                host.add_ref_deal(deal);
                if host.deal_flags(deal) & DEAL_PROPOSED_BIT != 0 {
                    host.deal_cancel(deal);
                }
                host.release_deal(deal);
            }
            host.set_neighbor_ui_disable(false);
            host.mark_deals_dirty();
        }
        // Unguarded (BW6b): also when CreateSubBudgetWindow failed and no
        // on_init paused, so the counter can go below zero.
        host.hidden_resume();
        match self.window {
            Some(w) => end_dialog(host, Some(w), END_CODE),
            None => true,
        }
    }

    /// The last Release: the destructor's base cleanup (`0x0077C4C0`)
    /// releases `+0x1C`, `+0x18`, `+0x14`, `+0x10`, `+0x0C` when non-null.
    /// Of these the core holds all but `+0x18`, which nothing in the spec
    /// sets.
    pub fn release_final<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H) {
        self.clear_saved_fractions();
        self.clear_sliders();
        self.clear_restore();
        self.clear_deal_rows();
        if let Some(mut n) = self.nested.take() {
            n.release_final(host);
        }
        if let Some(c) = self.inner_child.take() {
            host.release_win(c);
        }
        if let Some(c) = self.content.take() {
            host.release_win(c);
        }
        if let Some(w) = self.window.take() {
            host.release_win(w);
        }
    }
}

impl Drop for SubBudgetWindow {
    fn drop(&mut self) {
        // Destructor, not DestroySubBudgetWindow: never resume the simulator
        // here. Drain exactly once, including when the host borrow has ended.
        self.clear_saved_fractions();
        self.clear_sliders();
        self.clear_restore();
        self.clear_deal_rows();
        drop(self.nested.take());
        if let Some(refs) = &self.references {
            for win in [self.inner_child.take(), self.content.take(), self.window.take()]
                .into_iter()
                .flatten()
            {
                refs.release_win(win);
            }
        }
    }
}
