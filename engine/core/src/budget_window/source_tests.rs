// SPDX-License-Identifier: GPL-3.0-or-later
//! Recording public services used by the owner conformance tests.
use super::*;

impl SubBudgetSources for Rec {
    fn available_ordinance_count(&mut self) -> u32 { self.ord.rows.len() as u32 }
    fn available_ordinances(&mut self, capacity: u32) -> Vec<OrdId> {
        assert_eq!(capacity as usize, self.ord.rows.len());
        self.ord.rows.iter().map(|r| r.ordinance).collect()
    }
    fn ordinance_income(&mut self, id: OrdId) -> i64 { self.ord_income.get(&id).copied().unwrap_or(0) }
    fn is_income_ordinance(&mut self, id: OrdId) -> bool { self.ord_categories.get(&id).copied().unwrap_or(true) }
    fn is_ordinance_on(&mut self, id: OrdId) -> u8 {
        if let Some(values) = self.ord_observations.get_mut(&id) {
            return values.remove(0);
        }
        self.ord.rows.iter().find(|r| r.ordinance == id).unwrap().original
    }
    fn propose_deals(&mut self, include: bool) -> Vec<DealRef> {
        assert!(include);
        self.l("propose".into());
        self.deal_list.clone()
    }
    fn find_deals(&mut self, kind: u32) -> Vec<DealRef> {
        self.l(format!("find {kind:#x}"));
        self.deal_list.iter().copied().filter(|d| self.deal_kinds.get(d).copied().unwrap_or(0x4773FFE1) == kind).collect()
    }
    fn deal_type(&mut self, deal: DealRef) -> u32 { self.deal_kinds.get(&deal).copied().unwrap_or(0x4773FFE1) }
    fn deal_monthly_amount(&mut self, deal: DealRef) -> i64 { self.deals[&deal].monthly }
    fn deal_city_id(&mut self, deal: DealRef) -> u32 { deal as u32 + 100 }
    fn deal_a(&mut self, deal: DealRef) -> u32 { self.l(format!("a {deal}")); self.deals[&deal].last as u32 }
    fn deal_i(&mut self, deal: DealRef) -> u32 { self.l(format!("i {deal}")); self.deals[&deal].last_inc as u32 }
    fn deal_m(&mut self, deal: DealRef) -> u32 { self.l(format!("m {deal}")); self.deals[&deal].m }
    fn deal_snapshot_a(&mut self, deal: DealRef, value: u32) { self.deals.get_mut(&deal).unwrap().last = value as i32; }
    fn deal_snapshot_i(&mut self, deal: DealRef, value: u32) { self.deals.get_mut(&deal).unwrap().last_inc = value as i32; }
    fn deal_test_mask(&mut self, _: DealRef, mask: u32) -> bool { assert_eq!(mask, 0x100); self.blocked }
    fn deal_notify_cancel(&mut self, deal: DealRef) { self.l(format!("notify {deal}")); }
    fn deal_slot17(&mut self, deal: DealRef, value: u32) { self.l(format!("slot17 {deal} {value}")); }
    fn combo_selection(&mut self, _: WinRef, control: u32) -> Option<u32> { self.combo.get(&control).copied() }
    fn set_combo_text(&mut self, _: WinRef, control: u32, text: u32, notify: bool) { self.l(format!("combo_text {control} {text:#x} {notify}")); }
    fn department_occupants(&mut self, department: u32, _: u32) -> Vec<ItemRef> {
        self.l(format!("occupants {department:#x}"));
        for row in &self.restore { self.refs.add_ref_item(row.occupant); }
        self.restore.iter().map(|row| row.occupant).collect()
    }
    fn budget_group_ids(&mut self) -> Vec<u32> { self.l("values".into()); vec![ids::GROUP_UTILITIES] }
    fn paint_body(&mut self, _: WinRef, paint: BodyPaint) {
        match paint {
            BodyPaint::Empty { .. } => self.l("empty".into()),
            BodyPaint::Lists { a, b } => self.l(format!("big {a} {b}")),
            _ => {},
        }
        self.paints.push(paint);
    }
}

#[test]
fn available_ordinances_classify_map_and_observe_twice() {
    let mut h = Rec::default();
    h.ord.rows = [10, 20, 30].into_iter().map(|ordinance| OrdinanceRow { ordinance, original: 1, edited: 1 }).collect();
    h.ord_categories.insert(10, false);
    h.ord_observations.insert(20, vec![1, 0]);
    h.ord_income.extend([(10, -7), (20, 0), (30, 11)]);
    let d = open(&mut h, 1, 0);
    assert_eq!((d.list_a().len, d.list_b().len), (2, 1));
    assert_eq!(d.ordinance_row_map(), [1, 2, 0]);
    assert_eq!((d.ordinances()[1].original, d.ordinances()[1].edited), (1, 0));
    assert_eq!(d.ordinance_totals(&mut h, 12), (132, 84));
    let controls: Vec<_> = h.paints.iter().filter_map(|p| match p {
        BodyPaint::Ordinance { index, check, info, .. } => Some((*index, *check, *info)),
        _ => None,
    }).collect();
    assert_eq!(controls, [(1, 300, 500), (2, 301, 501), (0, 302, 502)]);
}

#[test]
fn offscreen_ordinances_contribute_with_low64_product() {
    let mut h = Rec::default();
    h.ord.rows = (0..10).map(|ordinance| OrdinanceRow { ordinance, original: 1, edited: 1 }).collect();
    h.ord_income.insert(9, i64::MAX);
    let d = open(&mut h, 1, 0);
    assert_eq!(d.list_a().len, 10);
    assert_eq!(d.ordinance_row_map(), (0..9).collect::<Vec<_>>());
    assert_eq!(d.ordinance_totals(&mut h, 2), (-2, 0));
}

#[test]
fn neighbor_six_scans_headers_totals_and_hidden_rows() {
    let mut h = Rec::default();
    h.deal_list = vec![1, 2, 3, 4];
    h.deal_kinds.extend([(1, 0x4773FFE1), (2, 0x4773FFE1), (3, 0x4773FFE3), (4, 0x4773FFE2)]);
    for deal in 1..=4 { h.deals.insert(deal, Deal::default()); }
    h.deals.get_mut(&3).unwrap().flags = 2;
    h.deals.get_mut(&3).unwrap().monthly = 17;
    let d = open(&mut h, 2, 0);
    assert_eq!((d.list_a().len, d.list_b().len), (5, 2));
    assert_eq!(d.deals(), [1, 2, 3, 4]);
    assert_eq!(d.active_deal_pairs(), [(103, 0x4773FFE3)]);
    assert_eq!(d.neighbor_section_totals(), &[0, 0, 17, 0, 0, 0]);
    let scans: Vec<_> = h.log.iter().filter(|s| s.starts_with("find ")).map(String::as_str).collect();
    assert_eq!(scans, ["find 0x4773ffe1", "find 0x4773ffe0", "find 0x4773ffe3", "find 0x4773ffe2", "find 0x4773ffe5", "find 0x4773ffe4"]);
    let refs = h.refs.clone();
    drop(d);
    // Literal native held-vector behavior: shutdown erases without per-deal
    // Release. BW17 leaves temporary-list balance open; no invented releases.
    assert!(refs.deals.lock().unwrap().values().all(|&n| n == 1));

    let mut h = Rec::default();
    h.deal_list = (1..=10).collect();
    for deal in 1..=10 { h.deals.insert(deal, Deal::default()); }
    let d = open(&mut h, 2, 0);
    assert_eq!(d.list_a().len, 11);
    assert_eq!(h.paints.iter().filter(|p| matches!(p, BodyPaint::NeighborDeal { .. })).count(), 8);
}

#[test]
fn combo_rows_edit_all_matches_and_own_separate_references() {
    let mut h = Rec::default();
    h.deals.insert(1, Deal { flags: 2, last: 100, last_inc: 30, m: 20, ..Default::default() });
    let mut d = open(&mut h, 2, 0);
    for _ in 0..2 { d.retain_deal_row(&mut h, DealRow { control: 42, deal: 1 }); }
    h.combo.insert(42, 4);
    h.log.clear();
    assert_eq!(d.edit_deal_rows(&mut h, WIN, 42), Ok(()));
    assert_eq!(d.neighbor_refresh_requests(), 1);
    assert_eq!(h.deals[&1].amount, 120);
    assert_eq!(h.count("slot17 1 20"), 2);
    let reads: Vec<_> = h.log.iter().filter(|s| s.starts_with("a ") || s.starts_with("i ") || s.starts_with("m ")).map(String::as_str).collect();
    assert_eq!(&reads[..10], ["m 1", "i 1", "a 1", "a 1", "m 1", "a 1", "i 1", "m 1", "m 1", "m 1"]);
    h.combo.insert(42, 3);
    h.log.clear();
    d.edit_deal_rows(&mut h, WIN, 42).unwrap();
    assert_eq!(h.deals[&1].amount, 90);
    assert_eq!(h.count("slot17"), 0);
    h.blocked = true;
    h.combo.insert(42, 99);
    d.edit_deal_rows(&mut h, WIN, 42).unwrap();
    assert_eq!(h.deals[&1].amount, 90);
    h.blocked = false;
    h.combo.insert(42, 0);
    d.edit_deal_rows(&mut h, WIN, 42).unwrap();
    assert_eq!(h.count("notify 1"), 1); // Second match sees inactive flags.
    d.on_shutdown(&mut h);
    assert_eq!(h.refs.deals.lock().unwrap()[&1], 0);
    drop(d);
    assert_eq!(h.refs.deals.lock().unwrap()[&1], 0);
}

#[test]
fn combo_missing_interface_and_invalid_division_are_safe() {
    let mut h = Rec::default();
    h.deals.insert(1, Deal { flags: 2, last: 1, last_inc: -1, m: 2, ..Default::default() });
    let mut d = open(&mut h, 2, 0);
    d.retain_deal_row(&mut h, DealRow { control: 1, deal: 1 });
    assert_eq!(d.edit_deal_rows(&mut h, WIN, 1), Ok(())); // QI failure
    // M changes are represented by a stable INT_MIN / -1 fault in A/I.
    h.deals.get_mut(&1).unwrap().last = i32::MIN;
    h.combo.insert(1, 1);
    assert_eq!(d.edit_deal_rows(&mut h, WIN, 1), Err(ComboError::InvalidDivision));
    assert_eq!(h.deals[&1].amount, 0);
}

#[test]
fn individual_reenumerates_viewport_preserves_and_rebuilds_snapshot() {
    let mut h = Rec::default();
    h.restore = (1..=12).map(|occupant| IndividualRestoreInfo { occupant, level0: 0.0, level1: 0.0 }).collect();
    let mut d = open(&mut h, 0, 0x1234); // Unknown service: zero working values.
    assert_eq!(d.restore_records().len(), 12);
    assert_eq!(d.restore_occupants(), (1..=12).collect::<Vec<_>>());
    for _ in 0..5 {
        d.set_list_len(true, 30);
        d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_LIST_B_DOWN));
    }
    assert_eq!(d.list_b().pos, 10);
    assert_eq!(d.individual_viewport(), [11, 12]);
    h.restore.pop();
    d.enumerate_individual(&mut h, WIN, false);
    assert_eq!(d.list_b().len, 11);
    assert_eq!(d.individual_viewport(), [11]);
    assert_eq!(d.restore_records().len(), 12);
    d.enumerate_individual(&mut h, WIN, true);
    assert_eq!(d.restore_records().len(), 11);
    assert_eq!(h.refs.items.lock().unwrap()[&12], 0);
    let refs = h.refs.clone();
    drop(h);
    drop(d);
    assert!(refs.items.lock().unwrap().values().all(|&n| n == 0));
    assert_eq!(individual_amount(i32::MAX, 1), -12);
    assert_eq!(individual_amount(-7, 0), -7);
}

#[test]
fn known_service_retains_pointers_without_inventing_funding_values() {
    let mut h = Rec { restore: vec![IndividualRestoreInfo { occupant: 9, level0: 73.0, level1: 81.0 }], ..Default::default() };
    let mut d = open(&mut h, 0, DEPT_HEALTH);
    assert_eq!(d.restore_occupants(), [9]);
    assert!(d.restore_records().is_empty());
    assert!(d.unsupported_operations().contains(&Unsupported::IndividualServiceValues));
    d.on_shutdown(&mut h);
    assert_eq!(h.refs.items.lock().unwrap()[&9], 0);
}

#[test]
fn group_refresh_keeps_distinct_ordinal_and_quota() {
    let mut h = group_host();
    h.items.insert(10, vec![item(1, 1, 1, 25, 0), item(1, 2, 1, 25, 0)]);
    h.group = vec![10];
    h.funding.insert(10, 0.5);
    h.total_income.insert(10, 50);
    let mut d = open(&mut h, GROUP_UTILITIES, 0);
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_LIST_A_DOWN));
    // Fitting list doesn't move: seed longer count to reach offset 2.
    d.set_list_len(false, 12);
    d.do_win_msg(&mut h, WIN, SubBudgetMsg::Button(BTN_LIST_A_DOWN));
    assert_eq!(d.refresh_group(&mut h, WIN), Some((3, 2)));
    assert_eq!(h.refs.depts.lock().unwrap()[&10], 1);
    assert!(d.unsupported_operations().contains(&Unsupported::GroupRefreshRemainder));
}

#[test]
fn item_property_is_type_checked_released_and_returns_borrowed_object() {
    let mut h = Rec::default();
    h.by_id.insert(5, 77);
    assert_eq!(dialog::get_window_indiv_item(&mut h, WIN, 5), Some(77));
    assert_eq!(h.property_object_reads, 1);
    assert_eq!(h.released_properties, [5]);
    assert!(h.refs.items.lock().unwrap().is_empty());
    h.property_kind = Some(3);
    assert_eq!(dialog::get_window_indiv_item(&mut h, WIN, 5), None);
    assert_eq!(h.property_object_reads, 1);
    assert_eq!(h.released_properties, [5, 5]);
    assert_eq!(dialog::get_window_indiv_item(&mut h, WIN, 6), None);
    assert_eq!(h.released_properties, [5, 5]);
}
