// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent literals from network-routines §§3–8. The scripted host supplies
//! only individual foreign returns; it never computes the expected region.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

use super::placement_demolition::*;
use crate::grid::CellRect;
use crate::zones::CellRegion;

#[derive(Debug)]
enum Call {
    Manager,
    Lot(i32, i32, Option<u64>),
    Rect(u64, CellRect),
    Zone(i32, i32, i8),
    First(i32, i32, Option<u64>, u8, Option<u64>),
    Release(u64),
    Qi(u64, u32, u8, Option<u64>),
    Preview,
}

struct ExpectedPreview {
    rect: CellRect,
    size: (i32, i32),
    words: Vec<Vec<u32>>,
    cost_address: usize,
    set_address: Option<usize>,
    initial_set: Vec<u64>,
    written_cost: i64,
    inserted: Option<u64>,
    al: u8,
    probes: Vec<(u64, bool)>,
}

struct Host {
    calls: RefCell<VecDeque<Call>>,
    preview: RefCell<Option<ExpectedPreview>>,
    filter: RefCell<Option<Weak<ExclusionFilter>>>,
    ref_ledger: RefCell<Vec<usize>>,
}

impl Host {
    fn new(calls: Vec<Call>, preview: Option<ExpectedPreview>) -> Self {
        Self {
            calls: RefCell::new(calls.into()),
            preview: RefCell::new(preview),
            filter: RefCell::new(None),
            ref_ledger: RefCell::new(Vec::new()),
        }
    }

    fn next(&self) -> Call {
        self.calls.borrow_mut().pop_front().expect("unexpected call")
    }

    fn done(&self, previewed: bool) {
        assert!(self.calls.borrow().is_empty(), "unconsumed call ledger");
        assert!(self.preview.borrow().is_none());
        if previewed {
            let filter = self.filter.borrow();
            let weak = filter.as_ref().unwrap();
            assert!(weak.upgrade().is_none(), "owner reference not released");
            assert_eq!(weak.strong_count(), 0);
            // owner, foreign hold, foreign release, owner cleanup
            let mut ledger = self.ref_ledger.borrow().clone();
            ledger.push(weak.strong_count());
            assert_eq!(ledger, [1, 2, 1, 0]);
        } else {
            assert!(self.filter.borrow().is_none());
            assert!(self.ref_ledger.borrow().is_empty());
        }
    }
}

impl RawReferenceHost for Host {
    fn query_interface(&self, occupant: u64, iid: u32, output: &mut Option<u64>) -> u8 {
        assert_eq!(*output, None, "QI output must start null");
        match self.next() {
            Call::Qi(o, i, al, result) => {
                assert_eq!((occupant, iid), (o, i));
                *output = result;
                al
            }
            other => panic!("QI out of order: {other:?}"),
        }
    }

    fn release(&self, reference: u64) {
        match self.next() {
            Call::Release(r) => assert_eq!(reference, r),
            other => panic!("release out of order: {other:?}"),
        }
    }
}

impl PlacementDemolitionHost for Host {
    type OccupantSet = Vec<u64>;

    fn get_lot_manager(&self) {
        assert!(matches!(self.next(), Call::Manager));
    }

    fn get_lot(&self, x: i32, z: i32, check_containment: bool) -> Option<u64> {
        assert!(!check_containment);
        match self.next() {
            Call::Lot(ex, ez, lot) => {
                assert_eq!((x, z), (ex, ez));
                lot
            }
            other => panic!("lot out of order: {other:?}"),
        }
    }

    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        match self.next() {
            Call::Rect(id, rect) => {
                assert_eq!(lot, id);
                rect
            }
            other => panic!("rect out of order: {other:?}"),
        }
    }

    fn zone_type(&self, x: i32, z: i32) -> i8 {
        match self.next() {
            Call::Zone(ex, ez, zone) => {
                assert_eq!((x, z), (ex, ez));
                zone
            }
            other => panic!("zone out of order: {other:?}"),
        }
    }

    fn first_occupant(&self, output: &mut Option<u64>, x: i32, z: i32, placement_filter: Option<u64>) -> u8 {
        assert_eq!(*output, None, "lookup output must start null");
        match self.next() {
            Call::First(ex, ez, filter, al, result) => {
                assert_eq!((x, z, placement_filter), (ex, ez, filter));
                *output = result;
                al
            }
            other => panic!("first occupant out of order: {other:?}"),
        }
    }

    fn demolish_region(&self, args: PreviewArgs<'_, Vec<u64>>) -> u8 {
        assert!(matches!(self.next(), Call::Preview));
        let expected = self.preview.borrow_mut().take().expect("unexpected preview");
        assert!(!args.do_it);
        assert_eq!(args.region.rect, expected.rect);
        assert_eq!((args.region.width(), args.region.height()), expected.size);
        assert_eq!(project_words(args.region), expected.words);
        assert_eq!(args.privilege, 1);
        assert_eq!(args.flags, 0x2001);
        assert!(!args.dezone);
        assert_eq!(args.cost as *mut i64 as usize, expected.cost_address);
        assert_eq!(*args.cost, 0, "old cost must be cleared before preview");
        assert_eq!(
            args.demolished.as_ref().map(|s| &**s as *const Vec<u64> as usize),
            expected.set_address
        );
        if let Some(set) = args.demolished {
            assert_eq!(*set, expected.initial_set);
            if let Some(occupant) = expected.inserted {
                set.push(occupant);
            }
        }
        assert_eq!(args.clicked, None);
        assert_eq!((args.screen_x, args.screen_y), (0, 0));

        assert!(args.caller_filter.is_type_included(0));
        assert!(args.caller_filter.is_type_included(u32::MAX));
        assert!(args.caller_filter.is_property_holder_included(&()));
        for (occupant, verdict) in expected.probes {
            assert_eq!(args.caller_filter.is_occupant_included(self, occupant), verdict);
        }
        *self.filter.borrow_mut() = Some(Rc::downgrade(args.caller_filter));
        self.ref_ledger.borrow_mut().push(Rc::strong_count(args.caller_filter));
        let held = Rc::clone(args.caller_filter);
        self.ref_ledger.borrow_mut().push(Rc::strong_count(&held));
        drop(held);
        self.ref_ledger.borrow_mut().push(Rc::strong_count(args.caller_filter));
        *args.cost = expected.written_cost;
        expected.al
    }
}

/// Independent logical projection, not a native allocator/layout assertion.
/// Explicitly construct column-local 32-bit LSB-first words, zeroing padding.
fn project_words(region: &CellRegion) -> Vec<Vec<u32>> {
    let words_per_column = (region.height() as usize).div_ceil(32);
    let mut words = vec![vec![0_u32; words_per_column]; region.width() as usize];
    for (dx, column) in words.iter_mut().enumerate() {
        for dz in 0..region.height() as usize {
            if region.includes(region.rect.x1 + dx as i32, region.rect.z1 + dz as i32) {
                column[dz / 32] |= 1_u32 << (dz % 32);
            }
        }
    }
    words
}

fn seed() -> RegionOutput {
    RegionOutput {
        current: Some(Box::new(CellRegion::from_cells(
            CellRect::new(50, 50, 50, 50),
            &[(50, 50)],
        ))),
        displaced: Vec::new(),
    }
}

fn preview(
    rect: CellRect,
    size: (i32, i32),
    words: Vec<Vec<u32>>,
    cost: &mut i64,
    set: Option<&mut Vec<u64>>,
) -> ExpectedPreview {
    ExpectedPreview {
        rect,
        size,
        words,
        cost_address: cost as *mut i64 as usize,
        set_address: set.as_ref().map(|s| &**s as *const Vec<u64> as usize),
        initial_set: set.as_ref().map_or_else(Vec::new, |s| s.to_vec()),
        written_cost: 37,
        inserted: None,
        al: 1,
        probes: Vec::new(),
    }
}

#[test]
fn literal_1_empty_getter_and_unchanged_outputs() {
    let host = Host::new(vec![Call::Manager], None);
    let mut output = seed();
    let old = &**output.current.as_ref().unwrap() as *const CellRegion;
    let (mut cost, mut set) = (91, vec![77]);
    assert_eq!(
        compute_demolition_region_for_placement_cells(
            &host,
            CitySide::Small,
            &[],
            &mut output,
            &mut cost,
            Some(&mut set),
            Some(55),
        ),
        Ok(1)
    );
    assert_eq!(&**output.current.as_ref().unwrap() as *const CellRegion, old);
    assert!(output.displaced.is_empty());
    assert_eq!(output.current.as_ref().unwrap().cells(), [(50, 50)]);
    assert_eq!((cost, set), (91, vec![77]));
    host.done(false);
}

#[test]
fn literal_2_sparse_clear_border_and_old_region_retained() {
    let mut output = seed();
    let old = &**output.current.as_ref().unwrap() as *const CellRegion;
    let mut cost = 91;
    let mut expected = preview(
        CellRect::new(2, 3, 8, 6),
        (7, 4),
        vec![vec![0], vec![0], vec![0], vec![0], vec![0], vec![0], vec![0]],
        &mut cost,
        None,
    );
    expected.al = 0;
    let host = Host::new(
        vec![
            Call::Manager,
            Call::Lot(2, 3, None),
            Call::Lot(7, 5, None),
            Call::Preview,
        ],
        Some(expected),
    );
    assert_eq!(
        compute_demolition_region_for_placement_cells(
            &host,
            CitySide::Small,
            &[(2, 3), (7, 5)],
            &mut output,
            &mut cost,
            None,
            None,
        ),
        Ok(0)
    );
    assert_eq!(cost, 37);
    assert!(output.current.as_ref().unwrap().is_empty());
    assert_eq!(output.displaced.len(), 1);
    assert_eq!(&*output.displaced[0] as *const CellRegion, old);
    assert_eq!(output.displaced[0].cells(), [(50, 50)]);
    host.done(true);
}

#[test]
fn literal_3_lot_extension_no_transitive_queries_zero_zone_skips() {
    use Call::*;
    let mut cost = 91;
    let expected = preview(
        CellRect::new(4, 5, 7, 8),
        (4, 4),
        vec![vec![1], vec![0], vec![0], vec![0]],
        &mut cost,
        None,
    );
    let host = Host::new(
        vec![
            Manager,
            Lot(5, 6, Some(10)),
            Rect(10, CellRect::new(4, 5, 6, 7)),
            Zone(4, 5, 1),
            First(4, 5, Some(55), 0, None),
            Zone(4, 6, 0),
            Zone(4, 7, 0),
            Zone(5, 5, 0),
            Zone(5, 6, 0),
            Zone(5, 7, 0),
            Zone(6, 5, 0),
            Zone(6, 6, 0),
            Zone(6, 7, 2),
            First(6, 7, Some(55), 0x80, Some(200)),
            Release(200),
            Preview,
        ],
        Some(expected),
    );
    let mut output = seed();
    assert_eq!(
        compute_demolition_region_for_placement_cells(
            &host,
            CitySide::Small,
            &[(5, 6)],
            &mut output,
            &mut cost,
            None,
            Some(55),
        ),
        Ok(1)
    );
    assert_eq!(output.current.as_ref().unwrap().cells(), [(4, 5)]);
    host.done(true);
}

#[test]
fn literal_4_duplicate_rectangles_repeat_queries_releases_keep_prior_bit() {
    use Call::*;
    let mut cost = 91;
    let expected = preview(
        CellRect::new(4, 5, 7, 8),
        (4, 4),
        vec![vec![1], vec![0], vec![0], vec![0]],
        &mut cost,
        None,
    );
    let host = Host::new(
        vec![
            Manager,
            Lot(5, 6, Some(10)),
            Rect(10, CellRect::new(4, 5, 6, 7)),
            Lot(5, 6, Some(10)),
            Rect(10, CellRect::new(4, 5, 6, 7)),
            Zone(4, 5, 1),
            First(4, 5, None, 0, None),
            Zone(4, 6, 0),
            Zone(4, 7, 0),
            Zone(5, 5, 0),
            Zone(5, 6, 0),
            Zone(5, 7, 0),
            Zone(6, 5, 0),
            Zone(6, 6, 0),
            Zone(6, 7, 2),
            First(6, 7, None, 1, Some(201)),
            Release(201),
            Zone(4, 5, 1),
            First(4, 5, None, 1, Some(202)),
            Release(202),
            Zone(4, 6, 0),
            Zone(4, 7, 0),
            Zone(5, 5, 0),
            Zone(5, 6, 0),
            Zone(5, 7, 0),
            Zone(6, 5, 0),
            Zone(6, 6, 0),
            Zone(6, 7, 2),
            First(6, 7, None, 1, Some(203)),
            Release(203),
            Preview,
        ],
        Some(expected),
    );
    let mut output = seed();
    assert_eq!(
        compute_demolition_region_for_placement_cells(
            &host,
            CitySide::Small,
            &[(5, 6), (5, 6)],
            &mut output,
            &mut cost,
            None,
            None,
        ),
        Ok(1)
    );
    assert_eq!(output.current.as_ref().unwrap().cells(), [(4, 5)]);
    host.done(true);
}

#[test]
fn literal_5_blocked_mask_after_all_scans_but_high_zone_hit_passes() {
    use Call::*;
    for hit in [false, true] {
        let mut cost = 91;
        let mut set = vec![77];
        let expected = if hit {
            Some(preview(
                CellRect::new(1, 1, 5, 5),
                (5, 5),
                vec![vec![0], vec![0], vec![0], vec![8], vec![0]],
                &mut cost,
                Some(&mut set),
            ))
        } else {
            None
        };
        let mut calls = vec![
            Manager,
            Lot(1, 1, Some(10)),
            Rect(10, CellRect::new(1, 1, 1, 1)),
            Lot(4, 4, Some(11)),
            Rect(11, CellRect::new(4, 4, 4, 4)),
            Zone(1, 1, 10),
            First(1, 1, None, u8::from(hit), hit.then_some(200)),
        ];
        if hit {
            calls.push(Release(200));
        }
        calls.extend([Zone(4, 4, 2), First(4, 4, None, 0, None)]);
        if hit {
            calls.push(Preview);
        }
        let host = Host::new(calls, expected);
        let mut output = seed();
        assert_eq!(
            compute_demolition_region_for_placement_cells(
                &host,
                CitySide::Small,
                &[(1, 1), (4, 4)],
                &mut output,
                &mut cost,
                Some(&mut set),
                None,
            ),
            Ok(u8::from(hit))
        );
        assert_eq!(set, [77]);
        if !hit {
            assert_eq!(cost, 91);
            assert_eq!(
                project_words(output.current.as_ref().unwrap()),
                [vec![1], vec![0], vec![0], vec![8], vec![0]]
            );
        }
        host.done(hit);
    }
}

#[test]
fn literal_6_relative_word_boundary_columns_padding_and_clear_edge() {
    use Call::*;
    let mut cost = 91;
    let expected = preview(
        CellRect::new(2, 31, 3, 34),
        (2, 4),
        vec![vec![5], vec![0]],
        &mut cost,
        None,
    );
    let host = Host::new(
        vec![
            Manager,
            Lot(2, 31, Some(10)),
            Rect(10, CellRect::new(2, 31, 2, 31)),
            Lot(2, 33, Some(11)),
            Rect(11, CellRect::new(2, 33, 2, 33)),
            Zone(2, 31, 1),
            First(2, 31, None, 0, None),
            Zone(2, 33, 1),
            First(2, 33, None, 0, None),
            Preview,
        ],
        Some(expected),
    );
    assert_eq!(
        compute_demolition_region_for_placement_cells(
            &host,
            CitySide::Small,
            &[(2, 31), (2, 33)],
            &mut seed(),
            &mut cost,
            None,
            None,
        ),
        Ok(1)
    );
    host.done(true);

    let expected = preview(
        CellRect::new(2, 0, 4, 34),
        (3, 35),
        vec![vec![0x8000_0000, 1], vec![1, 2], vec![0, 0]],
        &mut cost,
        None,
    );
    let mut calls = vec![Manager, Lot(2, 0, Some(10)), Rect(10, CellRect::new(2, 0, 3, 33))];
    // Fixed fixture visit ledger; no production bounds/rect loop generates it.
    let visits = [
        (2, 0),
        (2, 1),
        (2, 2),
        (2, 3),
        (2, 4),
        (2, 5),
        (2, 6),
        (2, 7),
        (2, 8),
        (2, 9),
        (2, 10),
        (2, 11),
        (2, 12),
        (2, 13),
        (2, 14),
        (2, 15),
        (2, 16),
        (2, 17),
        (2, 18),
        (2, 19),
        (2, 20),
        (2, 21),
        (2, 22),
        (2, 23),
        (2, 24),
        (2, 25),
        (2, 26),
        (2, 27),
        (2, 28),
        (2, 29),
        (2, 30),
        (2, 31),
        (2, 32),
        (2, 33),
        (3, 0),
        (3, 1),
        (3, 2),
        (3, 3),
        (3, 4),
        (3, 5),
        (3, 6),
        (3, 7),
        (3, 8),
        (3, 9),
        (3, 10),
        (3, 11),
        (3, 12),
        (3, 13),
        (3, 14),
        (3, 15),
        (3, 16),
        (3, 17),
        (3, 18),
        (3, 19),
        (3, 20),
        (3, 21),
        (3, 22),
        (3, 23),
        (3, 24),
        (3, 25),
        (3, 26),
        (3, 27),
        (3, 28),
        (3, 29),
        (3, 30),
        (3, 31),
        (3, 32),
        (3, 33),
    ];
    for (x, z) in visits {
        let selected = matches!((x, z), (2, 31) | (2, 32) | (3, 0) | (3, 33));
        calls.push(Zone(x, z, if selected { 1 } else { 0 }));
        if selected {
            calls.push(First(x, z, None, 0, None));
        }
    }
    calls.push(Preview);
    let host = Host::new(calls, Some(expected));
    let mut output = seed();
    assert_eq!(
        compute_demolition_region_for_placement_cells(
            &host,
            CitySide::Small,
            &[(2, 0)],
            &mut output,
            &mut cost,
            None,
            None,
        ),
        Ok(1)
    );
    assert_eq!(
        output.current.as_ref().unwrap().cells(),
        [(2, 31), (2, 32), (3, 0), (3, 33)]
    );
    host.done(true);
}

#[test]
fn literal_7_borrowed_placement_vs_owned_qi_complement_and_identity() {
    use Call::*;
    for hit in [true, false] {
        let mut cost = 91;
        let mut expected = preview(
            CellRect::new(3, 3, 4, 4),
            (2, 2),
            if hit {
                vec![vec![0], vec![0]]
            } else {
                vec![vec![1], vec![0]]
            },
            &mut cost,
            None,
        );
        expected.probes = vec![(9, false), (10, true), (11, false), (12, true)];
        let mut calls = vec![
            Manager,
            Lot(3, 3, Some(10)),
            Rect(10, CellRect::new(3, 3, 3, 3)),
            Zone(3, 3, 1),
            First(3, 3, Some(55), u8::from(hit), hit.then_some(9)),
        ];
        if hit {
            calls.push(Release(9));
        }
        calls.extend([
            Preview,
            Qi(9, 0xA976_4C8C, 0x80, Some(90)),
            Release(90),
            Qi(10, 0xA976_4C8C, 0, None),
            Qi(11, 0xA976_4C8C, 1, None),
            Qi(12, 0xA976_4C8C, 0, Some(91)),
            Release(91),
        ]);
        let host = Host::new(calls, Some(expected));
        assert_eq!(
            compute_demolition_region_for_placement_cells(
                &host,
                CitySide::Small,
                &[(3, 3)],
                &mut seed(),
                &mut cost,
                None,
                Some(55),
            ),
            Ok(1)
        );
        host.done(true);
    }
    let filter = Rc::new(ExclusionFilter);
    for iid in [1, 0x46A8_0026, 0xE4FD_A3D4] {
        let view = ExclusionFilter::query_interface(&filter, iid).unwrap();
        assert_eq!(Rc::strong_count(&filter), 2);
        match view {
            FilterInterface::Primary(ref p) => {
                assert_ne!(iid, 0xE4FD_A3D4);
                assert!(Rc::ptr_eq(p, &filter));
            }
            FilterInterface::Serializable(ref p) => {
                assert_eq!(iid, 0xE4FD_A3D4);
                assert!(Rc::ptr_eq(p, &filter));
            }
        }
        drop(view);
        assert_eq!(Rc::strong_count(&filter), 1);
    }
    assert!(ExclusionFilter::query_interface(&filter, 0xA976_4C8C).is_none());
    assert_eq!(Rc::strong_count(&filter), 1);
}

#[test]
fn literal_8_final_false_negative_cost_set_writes_and_raw_al_survive_cleanup() {
    use Call::*;
    for al in [0, 0xA5] {
        let (mut cost, mut set) = (91, vec![77]);
        let mut expected = preview(
            CellRect::new(1, 1, 2, 2),
            (2, 2),
            vec![vec![1], vec![0]],
            &mut cost,
            Some(&mut set),
        );
        expected.written_cost = -4_294_967_301;
        expected.inserted = Some(88);
        expected.al = al;
        let host = Host::new(
            vec![
                Manager,
                Lot(1, 1, Some(10)),
                Rect(10, CellRect::new(1, 1, 1, 1)),
                Zone(1, 1, 1),
                First(1, 1, None, 0, None),
                Preview,
            ],
            Some(expected),
        );
        let mut output = seed();
        assert_eq!(
            compute_demolition_region_for_placement_cells(
                &host,
                CitySide::Small,
                &[(1, 1)],
                &mut output,
                &mut cost,
                Some(&mut set),
                None,
            ),
            Ok(al)
        );
        assert_eq!(cost, -4_294_967_301);
        assert_eq!(set, [77, 88]);
        assert_eq!(output.current.as_ref().unwrap().cells(), [(1, 1)]);
        host.done(true);
    }
}

#[test]
fn literal_mask_retains_high_zone_miss_across_later_duplicate_hit() {
    use Call::*;
    let host = Host::new(
        vec![
            Manager,
            Lot(1, 1, Some(10)),
            Rect(10, CellRect::new(1, 1, 1, 1)),
            Lot(1, 1, Some(10)),
            Rect(10, CellRect::new(1, 1, 1, 1)),
            Zone(1, 1, 10),
            First(1, 1, None, 0, None),
            Zone(1, 1, 10),
            First(1, 1, None, 1, Some(200)),
            Release(200),
        ],
        None,
    );
    let (mut output, mut cost, mut set) = (seed(), 91, vec![77]);
    assert_eq!(
        compute_demolition_region_for_placement_cells(
            &host,
            CitySide::Small,
            &[(1, 1), (1, 1)],
            &mut output,
            &mut cost,
            Some(&mut set),
            None,
        ),
        Ok(0)
    );
    assert_eq!(project_words(output.current.as_ref().unwrap()), [vec![1], vec![0]]);
    assert_eq!((cost, set), (91, vec![77]));
    host.done(false);
}

#[test]
fn portable_admission_is_unsupported_not_native_false() {
    for cell in [(-1, 0), (0, -1), (64, 0), (0, 64), (i32::MAX, 1)] {
        let host = Host::new(vec![Call::Manager], None);
        let (mut output, mut cost) = (seed(), 91);
        assert_eq!(
            compute_demolition_region_for_placement_cells(
                &host,
                CitySide::Small,
                &[cell],
                &mut output,
                &mut cost,
                None,
                None,
            ),
            Err(Unsupported::InputOutsideCity)
        );
        assert_eq!(cost, 91);
        assert_eq!(output.current.as_ref().unwrap().cells(), [(50, 50)]);
        assert!(output.displaced.is_empty());
        host.done(false);
    }
    for rect in [CellRect::new(2, 0, 1, 0), CellRect::new(0, 0, 0, 64)] {
        let host = Host::new(
            vec![Call::Manager, Call::Lot(1, 1, Some(10)), Call::Rect(10, rect)],
            None,
        );
        let (mut output, mut cost) = (seed(), 91);
        assert_eq!(
            compute_demolition_region_for_placement_cells(
                &host,
                CitySide::Small,
                &[(1, 1)],
                &mut output,
                &mut cost,
                None,
                None,
            ),
            Err(Unsupported::MalformedOrOutsideLot)
        );
        assert_eq!(cost, 91);
        assert_eq!(output.current.as_ref().unwrap().cells(), [(50, 50)]);
        assert!(output.displaced.is_empty());
        host.done(false);
    }
}

#[test]
fn portable_largest_ordinary_domain_fits_shared_storage_without_cap() {
    let mut cost = 91;
    let expected = preview(
        CellRect::new(0, 0, 256, 256),
        (257, 257),
        vec![vec![0; 9]; 257],
        &mut cost,
        None,
    );
    let host = Host::new(
        vec![
            Call::Manager,
            Call::Lot(0, 0, None),
            Call::Lot(255, 255, None),
            Call::Preview,
        ],
        Some(expected),
    );
    let mut output = seed();
    assert_eq!(
        compute_demolition_region_for_placement_cells(
            &host,
            CitySide::Large,
            &[(0, 0), (255, 255)],
            &mut output,
            &mut cost,
            None,
            None,
        ),
        Ok(1)
    );
    assert_eq!(output.current.as_ref().unwrap().count(), 0);
    host.done(true);
}
