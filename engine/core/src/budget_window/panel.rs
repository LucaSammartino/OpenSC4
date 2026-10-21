// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The budget panel parts that own the sub-budget dialog (budget-window.md
//! §2, §2.1, §7). The panel's own taxes, loan and balance-sheet logic is not
//! modelled here: commands that reach it are returned to the caller.

use super::dialog::{show_sequence, SubBudgetWindow};
use super::ids::*;
use super::BudgetWindowHost;

/// A decoded budget-panel command (§2.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelCommand {
    /// Open a sub-budget dialog with this selector.
    SubBudget(u32),
    DisplayTaxesWindow,
    DisplayLoanWindow,
    /// Set the tab byte `+0x194`, then `ShowWindow` (slot 6, BW4); returns 0.
    SetTab(bool),
    RemoveLoanWindow,
    /// Scroll the loan list (`0x2A4C33A6` / `0xAA4C339C`; which one scrolls
    /// which way is not given).
    ScrollLoanList(u32),
    TakeLoan,
}

/// §2.1: decode a button-message value of the budget panel.
pub fn panel_command(id: u32) -> Option<PanelCommand> {
    use PanelCommand::*;
    if (0xAA3A_C400..=0xAA3A_C40F).contains(&id) || (0xAA3A_C500..=0xAA3A_C50F).contains(&id) {
        let sel = match id & 0xFF {
            0x00 => return Some(DisplayTaxesWindow),
            0x01 | 0x09 => SELECTOR_ORDINANCES,
            0x02 | 0x0A => SELECTOR_NEIGHBORS,
            0x03 => GROUP_CIVIC,
            0x04 | 0x05 => GROUP_TRANSPORTATION,
            0x06 => GROUP_PROTECTION,
            0x07 => GROUP_EDUCATION_HEALTH,
            0x08 => GROUP_UTILITIES,
            0x0B => GROUP_PARKS_LANDMARKS,
            0x0C => GROUP_CIVIC2,
            0x0D => return Some(DisplayLoanWindow),
            0x0E => return Some(SetTab(true)),
            _ => return Some(SetTab(false)), // 0x0F
        };
        return Some(SubBudget(sel));
    }
    match id {
        0x4A88_79CB => Some(SetTab(true)),
        0x4A88_79DB => Some(SetTab(false)),
        0x0A4C_3354 => Some(RemoveLoanWindow),
        0x2A4C_33A6 | 0xAA4C_339C => Some(ScrollLoanList(id)),
        0xAA4C_33E6 => Some(TakeLoan),
        _ => None,
    }
}

/// What [`BudgetPanel::on_command`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandOutcome {
    /// The tutorial filter swallowed the command (the handler returns 1).
    Swallowed,
    /// A sub-budget command was run through `DoSubBudgetWindow`.
    SubBudgetRun,
    /// A tab command ran `ShowWindow` (the handler returns 0).
    TabShown,
    /// A panel command this core does not own; the caller performs it.
    Panel(PanelCommand),
    /// Not a budget-panel command.
    Unknown,
}

/// The panel state the sub-budget path uses.
#[derive(Debug, Default)]
pub struct BudgetPanel {
    /// `+0x18`: the held sub-budget object.
    pub(crate) held: Option<SubBudgetWindow>,
    /// `+0x198`: passed as `CreateSubBudgetWindow`'s third argument.
    pub context: u32,
    /// `+0x192`: the money-message dirty byte.
    dirty: bool,
    /// `+0x194`: the tab flag `ShowWindow` reads.
    pub tab_flag: bool,
    /// `+0x19D`: set by `ShowWindow`.
    pub shown: bool,
}

impl BudgetPanel {
    pub fn new(context: u32) -> Self {
        Self {
            held: None,
            context,
            dirty: false,
            tab_flag: false,
            shown: false,
        }
    }

    pub fn is_holding_sub_budget(&self) -> bool {
        self.held.is_some()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Route a panel button command (§2.1): the tutorial filter first, then
    /// the sub-budget selectors.
    pub fn on_command<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, id: u32) -> CommandOutcome {
        let Some(cmd) = panel_command(id) else {
            return CommandOutcome::Unknown;
        };
        // "Every command first passes FilterUICommandMessage".
        if host.filter_ui_command(id) {
            return CommandOutcome::Swallowed;
        }
        match cmd {
            PanelCommand::SubBudget(sel) => {
                self.do_sub_budget_window(host, sel);
                CommandOutcome::SubBudgetRun
            }
            PanelCommand::SetTab(on) => {
                self.tab_flag = on;
                self.show_window(host);
                CommandOutcome::TabShown
            }
            other => CommandOutcome::Panel(other),
        }
    }

    /// `ShowWindow` (`0x00768670`, §2.1, BW4): `+0x194` = 1 hides tab
    /// `0xAA3AC000` and shows `0xAA3AC001`, 0 the reverse; the shown tab gets
    /// `SetFlag(0x800, 0)`; then the balance sheet; `+0x19D := 1` (§7).
    pub fn show_window<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H) {
        let (shown, hidden) = if self.tab_flag {
            (PANEL_TAB_1, PANEL_TAB_0)
        } else {
            (PANEL_TAB_0, PANEL_TAB_1)
        };
        host.set_panel_tab_visible(hidden, false);
        host.set_panel_tab_visible(shown, true);
        host.set_panel_tab_flag(shown, PANEL_TAB_SHOWN_FLAG, false);
        host.update_balance_sheet();
        self.shown = true;
    }

    /// `DoSubBudgetWindow` (`0x00768460`, §2).
    pub fn do_sub_budget_window<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, selector: u32) {
        if self.held.is_some() {
            return;
        }
        let dlg = self.held.insert(SubBudgetWindow::new());
        if let Some(win) = dlg.create(host, selector, self.context, 0) {
            let gen = show_sequence(host, win);
            host.restore_window_location(win);
            host.do_modal_win(win, dlg);
            host.focus_view3d();
            host.save_window_location(win);
            // BW6a: the panel releases `*out` and the generator reference.
            host.release_win(win);
            if gen {
                host.release_generator(win);
            }
        }
        dlg.destroy(host);
        // `+0x18 := null` and Release: the panel's is the only reference, so
        // the destructor runs.
        if let Some(mut d) = self.held.take() {
            d.release_final(host);
        }
        host.update_balance_sheet();
    }

    /// `DoMessage` (`0x00768620`, §7): money messages mark dirty; the tick
    /// refreshes at most once. The return value is not specified.
    pub fn do_message<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, msg: u32) {
        match msg {
            PANEL_MSG_MONEY_A | PANEL_MSG_MONEY_B => self.dirty = true,
            PANEL_MSG_TICK if self.dirty => {
                self.dirty = false;
                host.update_balance_sheet();
            }
            _ => {}
        }
    }
}
