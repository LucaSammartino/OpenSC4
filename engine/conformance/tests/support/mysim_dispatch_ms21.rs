// SPDX-License-Identifier: GPL-3.0-or-later
//! Normative mysims §12.30 vectors: MS21-01..10 and MS22-01.
use super::*;
use std::collections::VecDeque;

fn route() -> VecDeque<RouteData> {
    [0, 0, 1, 1, 0xFE]
        .into_iter()
        .enumerate()
        .map(|(i, travel_type)| RouteData {
            x: 200 + i as u8,
            z: 201,
            travel_type,
        })
        .collect()
}

#[derive(Default)]
struct Inputs {
    results: RefCell<VecDeque<u32>>,
    probes: RefCell<Vec<(u32, u32, u32, u32)>>,
    dispatched: RefCell<Vec<VecDeque<RouteData>>>,
    result: Cell<u32>,
    remaining: RefCell<VecDeque<RouteData>>,
    submitted: RefCell<Vec<(VecDeque<RouteData>, bool, bool)>>,
}
impl Inputs {
    fn new(values: &[u32]) -> Self {
        Self {
            results: RefCell::new(values.iter().copied().collect()),
            ..Self::default()
        }
    }
    fn check_probes(&self, xs: &[u32]) {
        assert_eq!(
            *self.probes.borrow(),
            xs.iter().map(|&x| (x, 201, 0x1FCF, 0)).collect::<Vec<_>>()
        );
        assert!(self.results.borrow().is_empty());
    }
}
impl TripServices for Inputs {
    fn route_network_probe(&self, x: u32, z: u32, mask: u32, last: u32) -> Option<u32> {
        self.probes.borrow_mut().push((x, z, mask, last));
        Some(self.results.borrow_mut().pop_front().expect("unexpected network probe"))
    }
    fn basic_dispatch(&self, _: i32, route: &VecDeque<RouteData>, _: Option<u64>) -> Option<u32> {
        self.dispatched.borrow_mut().push(route.clone());
        Some(self.result.get())
    }
    fn remaining_automaton_route(&self, _: u64) -> Option<VecDeque<RouteData>> {
        Some(self.remaining.borrow().clone())
    }
    fn automaton_position(&self, _: u64) -> Option<(f32, f32)> {
        Some((64.0, 80.0))
    }
    fn submit_remaining_route(&self, _: u64, route: &VecDeque<RouteData>, accept: bool, last: bool) -> Option<()> {
        self.submitted.borrow_mut().push((route.clone(), accept, last));
        Some(())
    }
}
fn fixture(start: u32, end: u32) -> DispatchFixture {
    let f = DispatchFixture::new();
    let mut st = f.d.get_state(1);
    st.search.route = route();
    st.path_indices = (end, start);
    st.search.state = 2;
    st.search.flags = 2;
    st.travel_type = 0;
    st.automata_occupant = Some(88);
    restore(&f, &st);
    f
}
fn indices(f: &DispatchFixture) -> (u32, u32) {
    let (end, start) = f.d.get_state(1).path_indices;
    (start, end)
}

#[test]
fn ms21_01_selected_marker_and_nonadvancing_extract() {
    let f = fixture(0, 0);
    let s = Inputs::new(&[7]);
    let before = f.d.get_state(1);
    assert_eq!(
        f.d.install_trip_route(1, route().range(0..4).copied().collect(), (204, 201), &s),
        Some(true)
    );
    assert_eq!(indices(&f), (0, 2));
    assert_eq!(f.d.selected_route_marker(1), Some(route()[0]));
    assert_eq!(f.d.dispatch_selected_leg(1, None, &s), Some(0));
    assert_eq!(
        *s.dispatched.borrow(),
        vec![route().range(0..2).copied().collect::<VecDeque<_>>()]
    );
    assert_eq!(indices(&f), (0, 2));
    assert_eq!(f.d.get_state(1).current, before.current);
    assert_eq!(f.d.get_state(1).search.route, before.search.route);
    s.check_probes(&[200]);
}
#[test]
fn ms21_02_front_trim_single_marker_rejects_without_dispatch() {
    let f = fixture(0, 0);
    let s = Inputs::new(&[0, 9]);
    assert_eq!(f.d.select_next_leg(1, &s), Some(true));
    assert_eq!(indices(&f), (1, 2));
    assert_eq!(f.d.selected_route_marker(1), Some(route()[1]));
    assert_eq!(f.d.dispatch_selected_leg(1, None, &s), Some(0xFF));
    assert!(s.dispatched.borrow().is_empty());
    assert_eq!(indices(&f), (1, 2));
    s.check_probes(&[200, 201]);
}
#[test]
fn ms21_03_jump_recovery_uses_marker_before_trimmed_start() {
    let f = fixture(1, 2);
    let s = Inputs::new(&[0, 1]);
    assert_eq!(f.d.update_selected_route(1, &s), TripOutcome::Accepted);
    let st = f.d.get_state(1);
    assert_eq!(indices(&f), (3, 4));
    assert_eq!(st.current.cell, (202, 201));
    assert_eq!((st.travel_type, st.search.flags), (1, 0x220));
    assert!(f.take().iter().all(|n| !matches!(n.1, 0xEBA06BD4 | 0xEBA06C12)));
    s.check_probes(&[202, 203]);
}
#[test]
fn ms21_04_fe_rejection_retains_new_range_and_current() {
    let f = fixture(3, 4);
    let before = f.d.get_state(1).current;
    let s = Inputs::new(&[1]);
    assert_eq!(f.d.update_selected_route(1, &s), TripOutcome::Accepted);
    assert_eq!(indices(&f), (4, 5));
    assert_eq!(f.d.get_state(1).current, before);
    assert_eq!(f.d.get_state(1).search.state, -1);
    assert_eq!(f.take()[0].1, 0xEBA06C12);
    s.check_probes(&[204]);
}
#[test]
fn ms21_05_exhaustion_repeat_is_inert() {
    let f = fixture(3, 4);
    let s = Inputs::new(&[0]);
    assert_eq!(f.d.jump_to_next_switch(1, &s), Some(false));
    assert_eq!(indices(&f), (5, 5));
    assert_eq!(f.d.jump_to_next_switch(1, &s), Some(false));
    assert_eq!(indices(&f), (5, 5));
    s.check_probes(&[204]);
}
#[test]
fn ms21_06_absent_run_probes_order_unsigned_and_selection_only() {
    let f = fixture(0, 0);
    let before = f.d.get_state(1);
    let s = Inputs::new(&[0, 0, 0, 0x8000_0000]);
    assert_eq!(f.d.select_next_leg(1, &s), Some(true));
    let mut after = f.d.get_state(1);
    assert_eq!(indices(&f), (3, 4));
    after.path_indices = before.path_indices;
    assert_eq!(after, before);
    assert!(f.take().is_empty());
    s.check_probes(&[200, 201, 202, 203]);
}
#[test]
fn ms21_07_all_runs_absent() {
    let f = fixture(0, 0);
    let s = Inputs::new(&[0; 5]);
    assert_eq!(f.d.select_next_leg(1, &s), Some(false));
    assert_eq!(indices(&f), (5, 5));
    assert!(f.take().is_empty());
    s.check_probes(&[200, 201, 202, 203, 204]);
}
#[test]
fn ms21_08_empty_range_does_not_repair() {
    let f = fixture(2, 2);
    let s = Inputs::default();
    assert_eq!(f.d.jump_to_next_switch(1, &s), Some(false));
    assert_eq!(f.d.dispatch_selected_leg(1, None, &s), Some(0xFF));
    assert_eq!(indices(&f), (2, 2));
    assert!(s.dispatched.borrow().is_empty());
    s.check_probes(&[]);
}
#[test]
fn ms21_09_failed_dispatch_preserves_then_recovery_advances_once() {
    let f = fixture(0, 2);
    let s = Inputs::new(&[1]);
    s.result.set(0xFF);
    assert_eq!(f.d.dispatch_selected_leg(1, None, &s), Some(0xFF));
    assert_eq!(indices(&f), (0, 2));
    assert_eq!(f.d.update_selected_route(1, &s), TripOutcome::Accepted);
    assert_eq!(indices(&f), (2, 4));
    assert_eq!(f.d.get_state(1).current.cell, (201, 201));
    assert_eq!(s.dispatched.borrow().len(), 2);
    assert_eq!(
        s.dispatched.borrow()[0],
        route().range(0..2).copied().collect::<VecDeque<_>>()
    );
    s.check_probes(&[202]);
}
#[test]
fn ms21_10_callback_five_selects_after_notice_or_submits_remaining() {
    let f = fixture(0, 2);
    let s = Inputs::new(&[1]);
    *s.remaining.borrow_mut() = route().range(0..1).copied().collect();
    let weak = Rc::downgrade(&f.d);
    *f.w.on_notify.borrow_mut() = Some(Rc::new(move |_, note, _, flags| {
        if note == 0x4BB2D020 && flags == 0x422 {
            let st = weak.upgrade().unwrap().get_state(1);
            assert_eq!(st.current.cell, (4, 5));
            assert_eq!(st.path_indices, (2, 0));
        }
    }));
    assert_eq!(f.d.automaton_callback(1, 5, None, &s), TripOutcome::Accepted);
    assert_eq!(indices(&f), (2, 4));
    // R[2] selects a different-mode wait; R[0] would dispatch the held
    // automaton instead. The source travel remains 0 during this wait.
    assert_eq!(f.d.get_state(1).automata_occupant, None);
    assert_eq!(f.d.get_state(1).travel_type, 0);
    s.check_probes(&[202]);
    let f = fixture(0, 2);
    let s = Inputs::default();
    *s.remaining.borrow_mut() = route();
    let before = f.d.get_state(1);
    assert_eq!(f.d.automaton_callback(1, 5, None, &s), TripOutcome::Accepted);
    assert_eq!(f.d.get_state(1), before);
    assert_eq!(
        *s.submitted.borrow(),
        vec![(route().range(1..).copied().collect(), true, false)]
    );
    s.check_probes(&[]);
}
#[test]
fn ms22_01_auxiliary_defaults_and_unpromised_flag() {
    for auxiliary_flag in [false, true] {
        let st = DispatchState {
            auxiliary_flag,
            ..DispatchState::default()
        };
        let mut bytes = ByteWriter::new();
        assert!(st.write(&mut bytes, &Refs));
        let restored = DispatchState::read(&mut ByteReader::new(bytes.bytes()), &Refs).unwrap();
        assert_eq!(restored.auxiliary_flag, auxiliary_flag);
        assert_eq!(restored.auxiliary_occupant, None);
        assert_eq!(restored.auxiliary_value, 0);
    }
}
