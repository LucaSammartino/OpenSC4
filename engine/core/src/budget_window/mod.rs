// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The budget window family (`docs/specs/budget-window.md`): the sub-budget
//! dialog `cSC4SubBudgetWindow` and the parts of the budget panel
//! `cSC4BudgetUIWinProc` that open it and coalesce its refreshes. Section
//! references (`§n`) point at that spec.
//!
//! External window, simulator, budget, ordinance, neighbour, deal and occupant
//! calls use [`BudgetWindowHost`]. Population and the settled refresh slices
//! use raw public interfaces in [`SubBudgetSources`]. Unsettled BW17 paths
//! report [`Unsupported`]; no native same-owner result is accepted as input.

pub mod dialog;
pub mod ids;
pub mod panel;

#[cfg(test)]
mod tests;

pub use dialog::{
    end_dialog, header_shown, line_item_shown, service_kind, slider_fraction, IndividualRestoreInfo, NeighborInit,
    OrdinanceInit, OrdinanceRow, ScrollList, ScrollView, SliderInfo, SubBudgetMode, SubBudgetMsg, SubBudgetWindow,
};
pub use dialog::{individual_amount, BodyPaint, ComboError, DealRow, SubBudgetSources, Unsupported};
pub use panel::{panel_command, BudgetPanel, CommandOutcome, PanelCommand};

/// An opaque handle to a host-side `cIGZWin` (or a window generator's window).
pub type WinRef = u64;
/// An opaque handle to a `cISC4DepartmentBudget`.
pub type DeptRef = u64;
/// An ordinance ID (the array at `+0xA4`).
pub type OrdId = u32;
/// An opaque handle to a neighbour deal (`cISC4NeighborDeal`).
pub type DealRef = u64;
/// An opaque handle to a row item (`get_window_indiv_item`'s object, a
/// `cIGZUnknown` that may answer the occupant IID `0x26BB767E`).
pub type ItemRef = u64;

/// Reference operations retained independently of a mutable host borrow.
/// Implementations must address the same objects as the host's handles and
/// remain usable until the last dialog is dropped (including after host drop).
pub trait BudgetWindowReferences: std::fmt::Debug + Send + Sync {
    fn add_ref_dept(&self, dept: DeptRef);
    fn release_dept(&self, dept: DeptRef);
    fn release_win(&self, win: WinRef);
    fn add_ref_item(&self, item: ItemRef);
    fn release_item(&self, item: ItemRef);
    fn release_deal(&self, deal: DealRef);
}

/// One department line item as `on_init` reads it (§4.1): `GetType()`,
/// `GetID()`, `GetDisplayFlags()`, `GetIncome()`, `GetFullExpenses()`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineItem {
    pub kind: u32,
    pub id: u32,
    pub display_flags: u32,
    pub income: i64,
    pub full_expenses: i64,
}

/// The two f32 funding values of a hospital / school query record that
/// `SetOccupantFundingLevel` rewrites (§5.5: `rec+0x2C` staff, `rec+0x34`
/// vehicle).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FundingPair {
    pub staff: f32,
    pub vehicle: f32,
}

/// Foreign services and window primitives. Owner algorithms stay in the core.
pub trait BudgetWindowHost: SubBudgetSources {
    fn references(&self) -> std::sync::Arc<dyn BudgetWindowReferences>;
    // ---- §3 window generator and window (cIGZWin) ----
    /// Create the window generator (class `0x4386D516`, interface
    /// `0x5386D516`) and return its `cIGZWin`, or `None` on failure. The
    /// returned reference is the one the dialog holds at `+0x0C`.
    fn create_window_generator(&mut self) -> Option<WinRef>;
    /// `AddRef` on a window (the `*out` reference of `CreateSubBudgetWindow`).
    fn add_ref_win(&mut self, win: WinRef);
    /// Release a window reference.
    fn release_win(&mut self, win: WinRef);
    /// Set a generator flag (generator interface `0x5386D516`).
    fn set_generator_flag(&mut self, win: WinRef, flag: u32, on: bool);
    /// Set the generator's window procedure (generator `+0x1C`) to the dialog.
    fn set_generator_win_proc(&mut self, win: WinRef);
    /// QueryInterface `0x5386D516` on the window (§2 step 4); true = it
    /// answered and a reference was taken (released by `release_generator`).
    fn query_generator(&mut self, win: WinRef) -> bool;
    /// Release the generator interface reference `query_generator` (or
    /// `create_content_window`) took.
    fn release_generator(&mut self, win: WinRef);
    /// §4 (BW10a): GZCOM create class `0x4386D516`, interface `0x5386D516`,
    /// and return the object's `cIGZWin` (generator slot 3). The returned
    /// handle carries the temporary generator reference, released by
    /// `release_generator`; `None` = the create failed.
    fn create_content_window(&mut self) -> Option<WinRef>;
    /// `win.ChildAdd(child)` (dialog slot 14 `+0x38`).
    fn child_add(&mut self, win: WinRef, child: WinRef);
    /// `add_image_button2(win, id, image)` (§5.3, BW10c); returns the button.
    /// Position and size are layout (§8, out of scope).
    fn add_image_button2(&mut self, win: WinRef, id: u32, image: u32) -> WinRef;
    /// The button's notification target := `target` (child slot 86 `+0x158`).
    fn set_notification_target(&mut self, child: WinRef, target: WinRef);
    /// `cIGZWin::SetFlag`.
    fn set_win_flag(&mut self, win: WinRef, flag: u32, on: bool);
    /// `cIGZWin` flag test (window slot 67; `end_dialog` tests `0x1000`).
    fn win_flag(&mut self, win: WinRef, flag: u32) -> bool;
    /// `cIGZWin::SetID` (slot 64).
    fn set_win_id(&mut self, win: WinRef, id: u32);
    /// `ShowWindow` on the dialog window (§2 step 4).
    fn show_win(&mut self, win: WinRef);
    /// `PullToFront` (§2 step 4).
    fn pull_to_front(&mut self, win: WinRef);
    /// `end_dialog` modal branch: `parent` slot 9 with `(0xF, win, code)`.
    fn end_modal_via_parent(&mut self, win: WinRef, code: u32);
    /// `end_dialog` modeless branch: `win` slot 11 with 1, its result and
    /// `win` to `parent` slot 14.
    fn remove_from_parent(&mut self, win: WinRef);
    /// `win.RemoveChild(child)` (window slot 15). `None` is the null
    /// argument the `0x68` / `0x384` buttons pass when `+0x14` is empty.
    fn remove_child(&mut self, win: WinRef, child: Option<WinRef>);
    /// `win.GetChild(id)` then `SetFlag(1, on)` (§5.3, no null check).
    fn set_child_visible(&mut self, win: WinRef, id: u32, on: bool);

    // ---- §2 the panel ----
    /// `WinLocationSaver.RestoreWindowLocation(win, 1)`.
    fn restore_window_location(&mut self, win: WinRef);
    /// `WinLocationSaver.SaveWindowLocation(win)`.
    fn save_window_location(&mut self, win: WinRef);
    /// `DoModalWin` (window manager `+0xA4`): run the modal loop, delivering
    /// the dialog's messages to `dialog.do_win_msg`, until it is ended.
    fn do_modal_win(&mut self, win: WinRef, dialog: &mut SubBudgetWindow);
    /// Give focus back to the 3D view window (manager `+0x94`).
    fn focus_view3d(&mut self);
    /// The panel's `UpdateBalanceSheetInfo` (`0x00767990`).
    fn update_balance_sheet(&mut self);
    /// `nsSC4TutorialSystem::FilterUICommandMessage`: true = swallow.
    fn filter_ui_command(&mut self, command: u32) -> bool;
    /// `ShowWindow` (`0x00768670`): show or hide a panel tab window.
    fn set_panel_tab_visible(&mut self, tab: u32, visible: bool);
    /// `ShowWindow`: `SetFlag` on the shown panel tab.
    fn set_panel_tab_flag(&mut self, tab: u32, flag: u32, on: bool);

    // ---- §4, §6 simulator ----
    /// `cISC4Simulator::HiddenPause` (slot 10).
    fn hidden_pause(&mut self);
    /// `cISC4Simulator::HiddenResume` (slot 13).
    fn hidden_resume(&mut self);
    /// Load the mode's background image (type, group, instance).
    fn load_background(&mut self, win: WinRef, type_id: u32, group: u32, instance: u32);

    // ---- ordinances (§4, §5.6) ----
    /// `OrdinanceSimulator.SetOrdinanceOn(id, edited)` (slot 13).
    fn set_ordinance_on(&mut self, ordinance: OrdId, on: u32);
    /// Child `id`'s check-box interface `0x8810`: its state's low byte, or
    /// `None` when the child does not answer.
    fn check_box_state(&mut self, win: WinRef, id: u32) -> Option<u8>;

    // ---- neighbour deals (§4, §5.4, §6) ----
    /// `NeighborsSimulator.SetUIDisableFlag`.
    fn set_neighbor_ui_disable(&mut self, on: bool);
    fn add_ref_deal(&mut self, deal: DealRef);
    fn release_deal(&mut self, deal: DealRef);
    /// Deal slot 36.
    fn deal_flags(&mut self, deal: DealRef) -> u32;
    /// Deal slot 34 (`Cancel`).
    fn deal_cancel(&mut self, deal: DealRef);
    /// Deal slot 35 (`Activate`).
    fn deal_activate(&mut self, deal: DealRef) -> bool;
    /// Deal slot 11.
    fn deal_set_amount(&mut self, deal: DealRef, v: i32);
    /// Deal slot 12.
    fn deal_last_amount(&mut self, deal: DealRef) -> i32;
    /// Deal slot 13.
    fn deal_set_last_amount(&mut self, deal: DealRef, v: i32);
    /// Deal slot 14.
    fn deal_set_inc_amount(&mut self, deal: DealRef, v: i32);
    /// Deal slot 15.
    fn deal_set_last_inc_amount(&mut self, deal: DealRef, v: i32);
    /// Deal slot 17.
    fn deal_last_inc_amount(&mut self, deal: DealRef) -> i32;
    /// `NeighborsSimulator.MarkDealsDirty`.
    fn mark_deals_dirty(&mut self);

    // ---- group mode (§4, §4.1) ----
    /// `budget.GetDepartmentBudgetsInGroup(group, list)` (budget slot 31).
    fn department_budgets_in_group(&mut self, group: u32) -> Vec<DeptRef>;
    /// Department slot 21: the line items, in order.
    fn department_line_items(&mut self, dept: DeptRef) -> Vec<LineItem>;
    /// Department slot 16 `GetTotalIncome()`.
    fn department_total_income(&mut self, dept: DeptRef) -> i64;
    /// Department slot 14 `GetIdealMonthlyFunding()`.
    fn department_ideal_monthly_funding(&mut self, dept: DeptRef) -> i64;
    /// The f32 read through department slot 10 (`+0x28`, returns a
    /// pointer): the funding fraction `on_init` saves in `+0x38` (BW10b).
    fn funding_percentage(&mut self, dept: DeptRef) -> f32;
    /// Department slot 11: `SetFundingPercentage(&fraction, arg)`.
    fn set_funding_percentage(&mut self, dept: DeptRef, fraction: f32, arg: u32);
    /// Paint a row selected by the core. `item == None` means a header;
    /// `row` is the zero-based visible slot in its list. Presentation only:
    /// the host does not choose visibility, ordering, or department ownership.
    /// Called only when the content window exists (BW13).
    fn paint_group_row(
        &mut self,
        content: WinRef,
        group: u32,
        dept: DeptRef,
        item: Option<LineItem>,
        income: bool,
        row: u32,
    );
    /// Create/configure a slider through the external window factory and add
    /// it to `win`, releasing the creation reference. Returns its unique window
    /// ID; the core stores no slider-window reference (BW15).
    fn create_department_slider(&mut self, win: WinRef, dept: DeptRef, item: Option<LineItem>, row: u32) -> u32;

    // ---- individual mode (§4.4, §5.5, §5.7) ----
    /// §4.4: the slider child by control ID (window slot 37, IID
    /// `0x21325207`); false = the lookup failed.
    fn find_slider(&mut self, win: WinRef, control: u32) -> bool;
    /// The slider's position (slider slot 4).
    fn slider_value(&mut self, win: WinRef, control: u32) -> i32;
    /// Window slot 35, child lookup. Property handles are opaque and owned.
    fn find_child(&mut self, win: WinRef, id: u32) -> Option<WinRef>;
    fn get_property(&mut self, child: WinRef, key: u32) -> Option<u64>;
    fn property_type(&mut self, property: u64) -> u32;
    fn property_object(&mut self, property: u64) -> Option<ItemRef>;
    fn release_property(&mut self, property: u64);
    /// The item's `GetID()` (slot 3): a department ID.
    fn item_id(&mut self, item: ItemRef) -> u32;
    /// QueryInterface the occupant IID `0x26BB767E` into a holder.
    fn query_occupant(&mut self, item: ItemRef) -> bool;
    /// Release the holder `query_occupant` filled.
    fn release_occupant(&mut self, item: ItemRef);
    /// `ResidentialSimulator.GetHospitalQueryData` (slot 27).
    fn hospital_query_data(&mut self, occ: ItemRef) -> Option<FundingPair>;
    /// `ResidentialSimulator.GetSchoolQueryData` (slot 26).
    fn school_query_data(&mut self, occ: ItemRef) -> Option<FundingPair>;
    /// `ResidentialSimulator.SetOccupantFundingPercentages` (slot 19).
    fn set_occupant_funding_percentages(&mut self, occ: ItemRef, rec: FundingPair, is_school: bool);
    /// `PoliceSimulator.SetStationFunding` (slot 8).
    fn police_set_station_funding(&mut self, occ: ItemRef, p: f32);
    /// `FireProtectionSimulator.SetStationFunding` (slot 9).
    fn fire_set_station_funding(&mut self, occ: ItemRef, p: f32);
    /// `PowerSimulator.SetPowerPlantFundingPercentage` (slot 51).
    fn power_set_plant_funding(&mut self, occ: ItemRef, p: f32);
    /// `occupant.GetPosition(&v)` (slot 8), `v` pre-filled.
    fn occupant_position(&mut self, occ: ItemRef, v: [f32; 3]) -> [f32; 3];
    /// The renderer camera control's jump (`0x007CD810`).
    fn camera_jump(&mut self, v: [f32; 3]);
}
