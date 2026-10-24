// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::VecDeque;

struct Routes {
    answers: VecDeque<(u32, usize)>,
    candidates: Vec<[i32; 2]>,
    requests: Vec<PathRequest>,
    types: Vec<u32>,
    lookup: Option<RefValue>,
    lookup_cells: Vec<[i32; 2]>,
    exits: [u8; 4],
    eligible: usize,
    accepted: Vec<u8>,
    tested: Vec<(u8, u8, u8)>,
    modes: Vec<u32>,
    list: Vec<RefValue>,
    generated: usize,
    reset: usize,
    flag_two: bool,
}
fn network() -> RefValue {
    RefValue::Host {
        token: 10,
        class_id: 0xA821EF94,
    }
}
impl Default for Routes {
    fn default() -> Self {
        Self {
            answers: VecDeque::new(),
            candidates: Vec::new(),
            requests: Vec::new(),
            types: Vec::new(),
            lookup: Some(network()),
            lookup_cells: Vec::new(),
            exits: [3, 2, 0, 1],
            eligible: 0,
            accepted: vec![2],
            tested: Vec::new(),
            modes: Vec::new(),
            list: vec![network()],
            generated: 0,
            reset: 0,
            flag_two: false,
        }
    }
}
impl RoutingWorld for Routes {
    fn city_size(&mut self) -> Option<(i32, i32)> {
        Some((8, 6))
    }
    fn connections_available(&mut self) -> bool {
        true
    }
    fn finder_available(&mut self) -> bool {
        true
    }
    fn position_cell(&mut self, _: Vec3) -> [i32; 2] {
        [1, 2]
    }
    fn connections(&mut self, t: u32) -> Vec<[i32; 2]> {
        self.types.push(t);
        if t == 3 {
            self.candidates.clone()
        } else {
            vec![]
        }
    }
    fn find_path(&mut self, r: PathRequest) -> (u32, usize) {
        self.requests.push(r);
        self.answers.pop_front().unwrap()
    }
    fn reset_finder(&mut self) {
        self.reset += 1;
    }
    fn network_cell(&mut self, cell: [i32; 2]) -> Option<RefValue> {
        self.lookup_cells.push(cell);
        self.lookup.clone()
    }
    fn next_tile(&mut self, _: &LookAheadNode, mode: u32, _: u8) -> Option<RefValue> {
        self.modes.push(mode);
        Some(network())
    }
    fn ordered_exits(&mut self, _: &RefValue, _: u8, _: u32) -> ([u8; 4], usize) {
        (self.exits, self.eligible)
    }
    fn accepts(&mut self, _: &RefValue, s: u8, e: u8, x: u8) -> bool {
        self.tested.push((s, e, x));
        self.accepted.contains(&x)
    }
    fn network_list(&mut self, _: [u8; 2]) -> Vec<RefValue> {
        self.list.clone()
    }
    fn network_flag_two(&mut self, _: &RefValue) -> bool {
        self.flag_two
    }
    fn generate_path(&mut self, _: &RefValue, _: u8, _: u8, exit: u8) -> u32 {
        assert_eq!(exit, 255);
        self.generated += 1;
        0x03000003
    }
}
fn previous() -> LookAheadNode {
    LookAheadNode {
        network: network(),
        path: 0xA3EE0102,
        terminal: false,
    }
}
fn output() -> LookAheadNode {
    LookAheadNode {
        network: RefValue::Null,
        path: 0,
        terminal: false,
    }
}

#[test]
fn neighbor_rank_status_ties_zero_route_clamp_and_lookup() {
    let a = TrafficAutomaton::default();
    for (answers, winner) in [(vec![(1, 9), (1, 4), (1, 4)], 1), (vec![(2, 1), (1, 8), (1, 0)], 2)] {
        let mut w = Routes {
            candidates: vec![[9, -1], [3, 4], [5, 5]],
            answers: answers.into(),
            ..Routes::default()
        };
        let result = a.best_neighbor_connection(&mut w);
        assert!(crate::automata::refs_equal(&result, &network()));
        assert_eq!(w.types, [3, 12, 8, 2, 10]);
        assert_eq!(w.reset, 3);
        assert_eq!(w.requests.len(), 3);
        assert_eq!(
            w.requests[0],
            PathRequest {
                start: [1, 2, 1, 2],
                end: [7, 0, 7, 0],
                travel_mask: 2,
                starting_mask: 2,
                density: 0,
                flags: 1,
                destination_filter: 3
            }
        );
        assert_eq!(w.lookup_cells, vec![[[7, 0], [3, 4], [5, 5]][winner]]);
    }
    let mut w = Routes {
        candidates: vec![[9, -1]],
        answers: vec![(1, 0)].into(),
        lookup: None,
        ..Routes::default()
    };
    assert!(a.best_neighbor_connection(&mut w).is_null());
    assert_eq!(w.requests.len(), 1);
}

#[test]
fn ordinary_expansion_packed_word_order_and_fallback_gates() {
    let mut a = TrafficAutomaton::default();
    let mut w = Routes {
        accepted: vec![3],
        ..Routes::default()
    };
    let mut out = output();
    assert!(a.owner().expand_look_ahead(&previous(), &mut out, &mut w));
    assert_eq!(out.path, 0x03000003);
    assert!(!out.terminal);
    let mut w = Routes {
        accepted: vec![2, 0],
        ..Routes::default()
    };
    let mut out = output();
    assert!(a.owner().expand_look_ahead(&previous(), &mut out, &mut w));
    assert_eq!(out.path, 0x03000002);
    assert_eq!(w.tested, vec![(3, 0, 3), (3, 0, 2)]);
    for (eligible, travel, flag, fallback, expected) in [
        (1, 0, 0, 0, true),
        (0, 0, 0, 0, false),
        (1, 7, 0, 0, false),
        (1, 8, 0, 0, false),
        (1, 0, 0x200000, 0, false),
        (1, 0, 0, 1, false),
    ] {
        a.runtime.travel_type = travel;
        a.runtime.fallback_byte = fallback;
        a.record.base.automata_flags = flag;
        let mut w = Routes {
            eligible,
            accepted: vec![2],
            ..Routes::default()
        };
        let mut out = output();
        assert!(a.owner().expand_look_ahead(&previous(), &mut out, &mut w));
        assert_eq!(w.generated, usize::from(expected));
        assert_eq!(out.terminal, expected);
    }
    a.runtime.travel_type = 0;
    a.runtime.fallback_byte = 0;
    a.record.base.automata_flags = 0;
    a.record.network = network();
    let mut w = Routes {
        eligible: 1,
        flag_two: true,
        ..Routes::default()
    };
    let mut out = output();
    assert!(a.owner().expand_look_ahead(&previous(), &mut out, &mut w));
    assert_eq!(w.generated, 0);
}

#[test]
fn explicit_expansion_consumption_diagonal_duplicate_and_terminal_order() {
    let mut a = TrafficAutomaton::default();
    a.record.base.automata_flags = SPECIAL_ROUTE;
    for (second, exit) in [([3, 4, 0], 1), ([4, 5, 0], 2)] {
        a.record.route = vec![[4, 5, 9], second];
        let mut w = Routes {
            accepted: vec![exit],
            ..Routes::default()
        };
        let mut out = output();
        assert!(a.owner().expand_look_ahead(&previous(), &mut out, &mut w));
        assert_eq!(w.tested, vec![(3, 0, exit)]);
        assert_eq!(a.record.route, vec![second]);
    }
    a.record.route = vec![[4, 5, 9], [3, 4, 0]];
    let mut w = Routes {
        list: vec![],
        ..Routes::default()
    };
    let mut out = output();
    assert!(!a.owner().expand_look_ahead(&previous(), &mut out, &mut w));
    assert_eq!(a.record.route.len(), 1);
    a.record.route = vec![[4, 5, 9]];
    let mut w = Routes {
        accepted: vec![2],
        ..Routes::default()
    };
    let mut out = output();
    let mut prev = previous();
    prev.path = 0x03000003;
    assert!(a.owner().expand_look_ahead(&prev, &mut out, &mut w));
    assert_eq!(w.modes, vec![1]);
    assert_eq!(w.tested, vec![(3, 1, 3), (3, 1, 0), (3, 1, 1), (3, 1, 2)]);
    assert!(out.terminal);
    assert_eq!(w.generated, 0);
}

#[test]
fn look_ahead_seed_failure_terminal_and_slowdown_vectors() {
    let mut a = TrafficAutomaton::default();
    a.record.network = network();
    a.runtime.path_word = previous().path;
    let mut w = Routes {
        accepted: vec![],
        ..Routes::default()
    };
    assert!(!a.owner().fill_look_ahead(2, &mut w));
    assert_eq!(a.runtime.look_ahead.len(), 1);
    assert!(a.runtime.look_ahead[0].terminal);
    a.runtime.look_ahead.clear();
    w.tested.clear();
    assert!(a.owner().fill_look_ahead(0, &mut w));
    assert_eq!(a.runtime.look_ahead.len(), 1);
    assert!(w.tested.is_empty());
    for (modifier, duration) in [(2.0, 0.75), (0.0, 0.1), (-1.0, 0.1)] {
        a.owner().slow_down(modifier);
        assert_eq!(a.record.base.f3[1], 0.0);
        assert_eq!(a.record.base.f3[2], duration);
    }
}
