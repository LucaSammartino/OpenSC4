// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled slices of budget-window-bodies.md. BW17 bounds every incomplete path.

use super::*;

/// A precisely identified unresolved owner operation; never an input from the
/// native owner's computed output. See budget-window-bodies.md §7 (BW17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    Setup,
    OrdinanceRefresh,
    OrdinanceScale,
    OrdinanceInfo,
    NeighborOptions,
    NeighborRefresh,
    NeighborListReferences,
    IndividualServiceValues,
    IndividualControls,
    GroupRefreshRemainder,
    ItemPropertySetter,
}

/// Low-level presentation instructions. All row choice, ordering, totals and
/// viewport membership belong to the core. No host callback computes a sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BodyPaint {
    Empty { title: Option<u32>, body: u32 },
    Lists { a: u32, b: u32 },
    Ordinance { index: u32, check: u32, info: u32, edited: u8 },
    NeighborHeader { kind: u32, income: bool, row: u32 },
    NeighborDeal { deal: DealRef, income: bool, row: u32 },
    /// Reuse name at control, SC4MakeMoneyString at control+1 when the owner
    /// scale is known, and hide control+2 with SetFlag(1,false).
    GroupHeader { control: u32, dept: DeptRef, amount: Option<i64> },
}

/// External public interfaces needed by the harvested owner bodies. Returned
/// ordinance/occupant vectors preserve provider order. FindDeals references
/// have the unresolved native list balance in BW17; see `NeighborListReferences`.
pub trait SubBudgetSources {
    fn available_ordinance_count(&mut self) -> u32;
    fn available_ordinances(&mut self, capacity: u32) -> Vec<OrdId>;
    fn ordinance_income(&mut self, id: OrdId) -> i64;
    fn is_income_ordinance(&mut self, id: OrdId) -> bool;
    fn is_ordinance_on(&mut self, id: OrdId) -> u8;
    fn propose_deals(&mut self, include: bool) -> Vec<DealRef>;
    fn find_deals(&mut self, kind: u32) -> Vec<DealRef>;
    fn deal_type(&mut self, deal: DealRef) -> u32;
    fn deal_monthly_amount(&mut self, deal: DealRef) -> i64;
    fn deal_city_id(&mut self, deal: DealRef) -> u32;
    /// Raw slots 10, 16 and 18 respectively, not renamed folded getters.
    fn deal_a(&mut self, deal: DealRef) -> u32;
    fn deal_i(&mut self, deal: DealRef) -> u32;
    fn deal_m(&mut self, deal: DealRef) -> u32;
    /// Raw slots 13 and 15 (initial snapshot).
    fn deal_snapshot_a(&mut self, deal: DealRef, value: u32);
    fn deal_snapshot_i(&mut self, deal: DealRef, value: u32);
    fn deal_test_mask(&mut self, deal: DealRef, mask: u32) -> bool;
    /// Raw slot 37, arguments (1,1).
    fn deal_notify_cancel(&mut self, deal: DealRef);
    /// Raw slot 17(+0x44) call with the fresh M value (§3.2).
    fn deal_slot17(&mut self, deal: DealRef, value: u32);
    /// QI combo IID 0x412CE496, read slot 30, Release interface.
    fn combo_selection(&mut self, win: WinRef, control: u32) -> Option<u32>;
    /// Combo slot 24(0), slot 27(localized text); if notify, slot 28(0,1).
    fn set_combo_text(&mut self, win: WinRef, control: u32, text: u32, notify: bool);
    /// Department lookup by ID, then slot 32(kind, output list). The list
    /// owns a temporary reference for each returned occupant.
    fn department_occupants(&mut self, department: u32, kind: u32) -> Vec<ItemRef>;
    /// Raw budget slot 29 records; only word zero is used by this owner slice.
    fn budget_group_ids(&mut self) -> Vec<u32>;
    fn paint_body(&mut self, win: WinRef, paint: BodyPaint);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DealRow {
    pub control: u32,
    pub deal: DealRef,
    // BW17: the native +8/+12 metadata and combo option construction are open.
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComboError {
    /// Native IDIV would fault. Safe literal divergence, BW19.
    InvalidDivision,
}

/// The signed-64 accumulation boundary of §5.1. Service amount derivation is
/// explicitly BW17; callers must not source that amount from the native owner.
pub fn individual_amount(amount: i32, context: u32) -> i64 {
    i64::from(if context == 1 { amount.wrapping_mul(12) } else { amount })
}

impl SubBudgetWindow {
    pub(super) fn unsupported(&mut self, operation: Unsupported) {
        if !self.unsupported.contains(&operation) {
            self.unsupported.push(operation);
        }
    }

    pub fn unsupported_operations(&self) -> &[Unsupported] {
        &self.unsupported
    }

    pub(super) fn empty_body<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef, title: Option<u32>, body: u32) {
        // Text selection is settled. Setup/factory ownership is not (BW17).
        self.unsupported(Unsupported::Setup);
        host.paint_body(win, BodyPaint::Empty { title, body });
    }

    pub(super) fn init_ordinances<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef) {
        let count = host.available_ordinance_count();
        let ids = host.available_ordinances(count);
        self.ordinances.clear();
        self.row_map.clear();
        self.ordinance_count = ids.len() as u32;
        for id in ids {
            let _income = host.ordinance_income(id);
            let income = host.is_income_ordinance(id);
            let list = if income { &mut self.list_a } else { &mut self.list_b };
            list.len = list.len.wrapping_add(1);
            let original = host.is_ordinance_on(id);
            let edited = host.is_ordinance_on(id);
            self.ordinances.push(OrdinanceRow { ordinance: id, original, edited });
        }
        if self.list_a.len == 0 && self.list_b.len == 0 {
            self.empty_body(host, win, Some(0x8A579F31), 0x0A5CB1A1);
            return;
        }
        self.unsupported(Unsupported::Setup);
        self.unsupported(Unsupported::OrdinanceScale);
        host.paint_body(win, BodyPaint::Lists { a: self.list_a.len.min(9), b: self.list_b.len.min(9) });
        for income in [true, false] {
            let mut visible = 0;
            for (index, row) in self.ordinances.iter().enumerate() {
                // These are distinct fresh provider observations on each pass.
                let _amount = host.ordinance_income(row.ordinance);
                if host.is_income_ordinance(row.ordinance) != income {
                    continue;
                }
                if visible < 9 {
                    let ordinal = self.row_map.len() as u32;
                    self.row_map.push(index as u32);
                    host.paint_body(win, BodyPaint::Ordinance {
                        index: index as u32, check: 300 + ordinal, info: 500 + ordinal, edited: row.edited,
                    });
                    visible += 1;
                }
            }
        }
        // Do not invent the owner scale or B formatting. Monthly raw amounts
        // remain observable above; total/display production is BW17.
        self.create_scroll_arrows(host, win, false);
    }

    /// Known multiplication boundary (§2.2). Explicit scales exercise low64
    /// math; the initializer does not invent the unspecified owner scale.
    pub fn ordinance_totals<H: BudgetWindowHost + ?Sized>(&self, host: &mut H, scale: i64) -> (i64, i64) {
        let (mut a, mut b) = (0i64, 0i64);
        for row in &self.ordinances {
            let amount = host.ordinance_income(row.ordinance);
            let income = host.is_income_ordinance(row.ordinance);
            if row.edited != 0 {
                let product = amount.wrapping_mul(scale);
                if income { a = a.wrapping_add(product); } else { b = b.wrapping_sub(product); }
            }
        }
        (a, b)
    }

    pub(super) fn init_neighbors<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef) {
        self.clear_deal_rows();
        self.deals.clear(); // Native held-vector erase does not Release (§4).
        self.deal_types.clear();
        self.active_deals.clear();
        self.neighbor_totals = [0; 6];
        self.list_a.len = 0;
        self.list_b.len = 0;
        for deal in host.propose_deals(true) {
            let _kind = host.deal_type(deal);
        }
        // BW17: no inferred temporary-list Release balance.
        self.unsupported(Unsupported::NeighborListReferences);
        for (section, kind) in [0x4773FFE1, 0x4773FFE0, 0x4773FFE3, 0x4773FFE2, 0x4773FFE5, 0x4773FFE4].into_iter().enumerate() {
            let deals = host.find_deals(kind);
            let list = if section % 2 == 0 { &mut self.list_a } else { &mut self.list_b };
            if !deals.is_empty() {
                list.len = list.len.wrapping_add(1).wrapping_add(deals.len() as u32);
            }
            for deal in deals {
                host.add_ref_deal(deal);
                host.add_ref_deal(deal);
                self.deals.push(deal);
                self.deal_types.push(kind);
                host.release_deal(deal);
                if host.deal_flags(deal) & 2 != 0 {
                    self.neighbor_totals[section] = self.neighbor_totals[section].wrapping_add(host.deal_monthly_amount(deal));
                    let a = host.deal_a(deal);
                    host.deal_snapshot_a(deal, a);
                    let i = host.deal_i(deal);
                    host.deal_snapshot_i(deal, i);
                    self.active_deals.push((host.deal_city_id(deal), kind));
                }
            }
        }
        if self.list_a.len == 0 && self.list_b.len == 0 {
            self.empty_body(host, win, Some(0x2A579AD2), 0xCA5CB52D);
            return;
        }
        self.unsupported(Unsupported::Setup);
        self.unsupported(Unsupported::NeighborOptions);
        host.paint_body(win, BodyPaint::Lists { a: self.list_a.len.min(9), b: self.list_b.len.min(9) });
        for (income, kinds) in [(true, [0x4773FFE1, 0x4773FFE3, 0x4773FFE5]), (false, [0x4773FFE0, 0x4773FFE2, 0x4773FFE4])] {
            let mut row = 0;
            for kind in kinds {
                let mut first = true;
                for (&deal, &deal_kind) in self.deals.iter().zip(&self.deal_types) {
                    if deal_kind != kind { continue; }
                    if first {
                        first = false;
                        if row < 9 { host.paint_body(win, BodyPaint::NeighborHeader { kind, income, row }); }
                        row += 1;
                    }
                    if row < 9 { host.paint_body(win, BodyPaint::NeighborDeal { deal, income, row }); }
                    row += 1;
                }
            }
        }
        self.create_scroll_arrows(host, win, true);
    }

    /// Called once a combo child has actually been created. BW17 leaves its
    /// options/metadata unsettled, so initialization never fabricates these IDs.
    pub fn retain_deal_row<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, row: DealRow) {
        self.references = Some(host.references());
        host.add_ref_deal(row.deal);
        host.add_ref_deal(row.deal);
        self.deal_rows.push(row);
        host.release_deal(row.deal);
    }

    pub(super) fn clear_deal_rows(&mut self) {
        if let Some(refs) = &self.references {
            for row in self.deal_rows.drain(..) { refs.release_deal(row.deal); }
        }
    }

    pub fn edit_deal_rows<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef, control: u32) -> Result<(), ComboError> {
        let mut result = Ok(());
        for row in self.deal_rows.clone() {
            if row.control != control { continue; }
            let Some(s) = host.combo_selection(win, control) else { continue };
            if s == 0 {
                if host.deal_flags(row.deal) & 2 != 0 {
                    host.deal_cancel(row.deal);
                    host.set_combo_text(win, control, 0xCA5793C0, true);
                    host.deal_notify_cancel(row.deal);
                }
                continue;
            }
            if host.deal_test_mask(row.deal, 0x100) { continue; }
            if host.deal_flags(row.deal) & 2 == 0 { let _ = host.deal_activate(row.deal); }
            let m = host.deal_m(row.deal);
            let special = m != 0 && host.deal_i(row.deal).wrapping_mul(s) > host.deal_a(row.deal);
            let amount = if special {
                let a0 = host.deal_a(row.deal) as i32;
                let m0 = host.deal_m(row.deal) as i32;
                let a1 = host.deal_a(row.deal) as i32;
                let i1 = host.deal_i(row.deal) as i32;
                let Some(k) = a0.checked_div(m0).zip(a1.checked_div(i1)).map(|(x, y)| x.wrapping_sub(y).wrapping_add(s as i32)) else {
                    result = Err(ComboError::InvalidDivision);
                    continue;
                };
                host.deal_m(row.deal).wrapping_mul(k as u32)
            } else { host.deal_i(row.deal).wrapping_mul(s) };
            host.deal_set_amount(row.deal, amount as i32);
            host.set_combo_text(win, control, 0x2A5794A0, false);
            if special {
                let fresh = host.deal_m(row.deal);
                host.deal_slot17(row.deal, fresh);
            }
        }
        self.request_neighbor_refresh();
        result
    }

    pub(super) fn request_neighbor_refresh(&mut self) {
        self.neighbor_refresh_requests = self.neighbor_refresh_requests.wrapping_add(1);
        self.unsupported(Unsupported::NeighborRefresh);
    }

    pub fn enumerate_individual<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef, rebuild: bool) {
        self.references = Some(host.references());
        let occupants = host.department_occupants(self.department, self.service_kind);
        self.list_b.len = occupants.len() as u32;
        self.individual_visible.clear();
        self.individual_total = Some(0);
        if rebuild { self.clear_restore(); }
        let quota = self.list_b.len.min(9);
        for (ordinal, occupant) in occupants.into_iter().enumerate() {
            let queried = host.query_occupant(occupant);
            let resolved = !queried || self.service_kind == 0;
            if !resolved { self.individual_total = None; }
            // Retain every original list pointer, never the QI holder. Funding
            // bytes remain unresolved for successful known-service queries.
            if rebuild {
                let refs = self.references.as_ref().expect("init keeps reference service");
                refs.add_ref_item(occupant);
                refs.add_ref_item(occupant);
                self.restore_occupants.push(occupant);
                if resolved { self.restore.push(IndividualRestoreInfo { occupant, level0: 0.0, level1: 0.0 }); }
                refs.release_item(occupant);
            }
            if ordinal as u32 >= self.list_b.pos && self.individual_visible.len() < quota as usize {
                self.individual_visible.push(occupant);
            }
            if queried { host.release_occupant(occupant); }
            self.references.as_ref().expect("init keeps reference service").release_item(occupant);
        }
        self.unsupported(Unsupported::IndividualServiceValues);
        self.unsupported(Unsupported::IndividualControls);
        if self.list_b.len == 0 { self.empty_body(host, win, None, 0x4A5CB4B6); }
    }

    pub(super) fn clear_restore(&mut self) {
        if let Some(refs) = &self.references {
            for item in self.restore_occupants.drain(..) { refs.release_item(item); }
        }
        self.restore.clear();
    }

    /// Append the owner's already-derived snapshot, with the §5.2 copy
    /// reference sequence. Service derivation is BW17, not a host row provider.
    pub fn retain_individual_restore<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, row: IndividualRestoreInfo) {
        self.references = Some(host.references());
        let refs = self.references.as_ref().expect("reference service retained");
        refs.add_ref_item(row.occupant);
        refs.add_ref_item(row.occupant);
        self.restore.push(row);
        self.restore_occupants.push(row.occupant);
        refs.release_item(row.occupant);
    }

    /// Settled A traversal of UpdateValues, §6. Returns q and p for diagnostics.
    /// Missing group records are an invalid provider input (BW19), not a
    /// successful empty-group result. B/item rendering stays Unsupported BW17.
    pub fn refresh_group<H: BudgetWindowHost + ?Sized>(&mut self, host: &mut H, win: WinRef) -> Option<(u32, u32)> {
        if self.refresh_busy { return None; }
        self.refresh_busy = true;
        let groups = host.budget_group_ids();
        let Some(&group) = groups.iter().find(|&&id| id == self.selector).or_else(|| groups.first()) else {
            self.refresh_busy = false;
            return None;
        };
        let depts = host.department_budgets_in_group(group);
        self.clear_sliders();
        let (mut q, mut p, mut control) = (0u32, 0u32, Some(0x0ABCDE03u32));
        // Owner scale initialization is BW17; do not publish a fabricated total.
        self.group_total_a = self.amount_scale.map(|_| 0);
        if self.list_a.len != 0 {
            for &dept in &depts {
                let mut first = true;
                for item in host.department_line_items(dept) {
                    if item.kind != LINE_ITEM_INCOME { continue; }
                    if std::mem::replace(&mut first, false) && header_shown(host, self.selector, dept, true) {
                        if q >= self.list_a.pos && p < 9 {
                            let raw = host.department_total_income(dept);
                            if let Some(id) = control {
                                let amount = self.amount_scale.map(|scale| raw.wrapping_mul(scale));
                                host.paint_body(win, BodyPaint::GroupHeader { control: id, dept, amount });
                                control = Some(id.wrapping_add(3));
                            }
                            p = p.wrapping_add(1);
                        }
                        // Accumulation makes a separate accessor observation.
                        let raw = host.department_total_income(dept);
                        if let (Some(total), Some(scale)) = (&mut self.group_total_a, self.amount_scale) {
                            *total = total.wrapping_add(raw.wrapping_mul(scale));
                        }
                        q = q.wrapping_add(1);
                    }
                    if item.id > 100 && item.income == 0 { continue; }
                    if item.display_flags & 1 != 0 {
                        // BW17: after a drawable item, the next control ID
                        // depends on its unsettled rendering body. Stop reuse
                        // rather than guessing which later header ID to write.
                        if q >= self.list_a.pos && p < 9 { control = None; }
                        p = p.wrapping_add(1);
                    }
                    q = q.wrapping_add(1);
                }
            }
        }
        for dept in depts { host.references().release_dept(dept); }
        self.refresh_busy = false;
        self.unsupported(Unsupported::GroupRefreshRemainder);
        Some((q, p))
    }
}
