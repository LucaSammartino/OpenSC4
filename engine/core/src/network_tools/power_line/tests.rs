// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent public-contract vectors. No native output or owner-result stubs.
#![allow(clippy::field_reassign_with_default)]

use super::*;
use crate::network_tools::{ToolRun, ToolState};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Add(Reference),
    Release(Reference),
    Zone((i32, i32)),
    Query((i32, i32)),
    Qi(Reference, u32),
    Create(Reference),
    Insert(Reference),
    Wire(Reference),
    Highlight(Reference, u32),
    Remove(Reference),
    Reset(Reference),
    Refresh(Reference),
    Selection((i32, i32), u32),
    Notify(Reference),
    Ray(Point, Point),
    Extra((i32, i32)),
    Detach(Reference, Reference),
    Clear(Reference),
    Draw([(i32, i32); 2]),
    Base,
    Attempt,
    Regional,
}

struct TestWorld {
    poles: BTreeMap<Reference, PoleData>,
    occupants: BTreeMap<(i32, i32), Vec<Reference>>,
    classes: BTreeMap<Reference, u32>,
    qis: BTreeMap<(Reference, u32), Reference>,
    positions: BTreeMap<Reference, Point>,
    wires: BTreeMap<Reference, [Option<Reference>; 2]>,
    wire_cells: BTreeMap<Reference, (i32, i32)>,
    cells: BTreeMap<(Reference, Reference), Vec<(i32, i32)>>,
    zones: VecDeque<i32>,
    blocked: BTreeMap<(i32, i32), bool>,
    events: Vec<Event>,
    corners: [f32; 4],
    special: bool,
    height: f32,
    width: f32,
    hit: Option<Point>,
    extra: bool,
    manager: bool,
    next: u64,
    attempt: bool,
    params: CurveParams,
    rectangle_query: Vec<Reference>,
}

impl Default for TestWorld {
    fn default() -> Self {
        Self {
            poles: BTreeMap::new(),
            occupants: BTreeMap::new(),
            classes: BTreeMap::new(),
            qis: BTreeMap::new(),
            positions: BTreeMap::new(),
            wires: BTreeMap::new(),
            wire_cells: BTreeMap::new(),
            cells: BTreeMap::new(),
            zones: VecDeque::new(),
            blocked: BTreeMap::new(),
            events: Vec::new(),
            corners: [10.0; 4],
            special: false,
            height: 10.0,
            width: 16.0,
            hit: None,
            extra: false,
            manager: true,
            next: 100,
            attempt: false,
            params: CurveParams::default(),
            rectangle_query: Vec::new(),
        }
    }
}

impl TestWorld {
    fn pole(&mut self, id: u64, cell: (i32, i32)) -> Reference {
        let reference = Reference(id);
        let position = [
            (cell.0 as f32 + 0.5) * self.width,
            20.0,
            (cell.1 as f32 + 0.5) * self.width,
        ];
        self.poles.insert(
            reference,
            PoleData {
                cell,
                position,
                divisors: [self.width; 2],
                attachments: [[[0.0; 3]; 4]; 4],
                connections: Vec::new(),
                dirty: false,
            },
        );
        self.positions.insert(reference, position);
        reference
    }
    fn register(&mut self, occupant: Reference, cell: (i32, i32), iid: u32, qi: Reference) {
        self.occupants.entry(cell).or_default().push(occupant);
        self.qis.insert((occupant, iid), qi);
        self.positions.entry(occupant).or_insert([
            (cell.0 as f32 + 0.5) * self.width,
            20.0,
            (cell.1 as f32 + 0.5) * self.width,
        ]);
    }
}

impl Terrain for TestWorld {
    fn cell_width(&mut self) -> f32 {
        self.width
    }
    fn special_height(&mut self, _: (i32, i32)) -> bool {
        self.special
    }
    fn height(&mut self, _: f32, _: f32) -> f32 {
        self.height
    }
    fn corners(&mut self, _: (i32, i32)) -> [f32; 4] {
        self.corners
    }
    fn intersect(&mut self, origin: Point, direction: Point, a: u32, b: u32) -> Option<Point> {
        assert_eq!((a, b), (0, 0));
        self.events.push(Event::Ray(origin, direction));
        self.hit.take()
    }
}

impl World for TestWorld {
    fn add_ref(&mut self, r: Reference) {
        self.events.push(Event::Add(r));
    }
    fn release(&mut self, r: Reference) {
        self.events.push(Event::Release(r));
    }
    fn query_cell(&mut self, cell: (i32, i32)) -> Vec<Reference> {
        self.events.push(Event::Query(cell));
        self.occupants.get(&cell).cloned().unwrap_or_default()
    }
    fn class(&mut self, r: Reference) -> u32 {
        self.classes.get(&r).copied().unwrap_or(0)
    }
    fn query_interface(&mut self, r: Reference, iid: u32) -> Option<Reference> {
        self.events.push(Event::Qi(r, iid));
        self.qis.get(&(r, iid)).copied()
    }
    fn occupant_position(&mut self, r: Reference) -> Point {
        self.positions[&r]
    }
    fn world_to_cell(&mut self, p: Point) -> (i32, i32) {
        ((p[0] / self.width) as i32, (p[2] / self.width) as i32)
    }
    fn zone(&mut self, cell: (i32, i32)) -> i32 {
        self.events.push(Event::Zone(cell));
        self.zones.pop_front().unwrap_or(0)
    }
    fn blocked(&mut self, cell: (i32, i32), arg: u32) -> bool {
        assert_eq!(arg, 0);
        self.blocked.get(&cell).copied().unwrap_or(false)
    }
    fn wire_endpoints(&mut self, wire: Reference) -> [Option<Reference>; 2] {
        self.wires[&wire]
    }
    fn wire_cell(&mut self, wire: Reference) -> (i32, i32) {
        self.wire_cells[&wire]
    }
    fn pole_data(&self, pole: Reference) -> &PoleData {
        &self.poles[&pole]
    }
    fn pole_data_mut(&mut self, pole: Reference) -> &mut PoleData {
        self.poles.get_mut(&pole).unwrap()
    }
    fn create_pole(&mut self, cell: (i32, i32), _: u8, position: Point) -> Reference {
        self.next += 1;
        let pole = self.pole(self.next, cell);
        self.poles.get_mut(&pole).unwrap().position = position;
        self.register(pole, cell, POLE_IID, pole);
        self.events.push(Event::Create(pole));
        pole
    }
    fn allocate_wire(&mut self) -> Reference {
        self.next += 1;
        let wire = Reference(self.next);
        self.events.push(Event::Wire(wire));
        wire
    }
    fn init_wire(&mut self, _: Reference) {}
    fn set_wire_mode(&mut self, _: Reference, real: bool) { assert!(real); }
    fn bind_wire(&mut self, wire: Reference, cell: (i32,i32), endpoints: [Reference;2], _:bool) {
        self.wires.insert(wire,endpoints.map(Some)); self.wire_cells.insert(wire,cell);
    }
    fn set_wire_position(&mut self, wire: Reference, position: Point) { self.positions.insert(wire,position); }
    fn insert_pole(&mut self, p: Reference, arg: u32) {
        assert_eq!(arg, 0);
        self.events.push(Event::Insert(p));
    }
    fn has_occupant_manager(&self) -> bool {
        self.manager
    }
    fn remove_pole(&mut self, p: Reference, a: bool, b: bool) {
        assert!(a && !b);
        self.events.push(Event::Remove(p));
    }
    fn highlight_pole(&mut self, p: Reference, mode: u32, arg: u32) {
        assert_eq!(arg, 0);
        self.events.push(Event::Highlight(p, mode));
    }
    fn reset_pole_preview(&mut self, p: Reference, arg: u32) {
        assert_eq!(arg, 0);
        self.events.push(Event::Reset(p));
    }
    fn refresh_pole(&mut self, p: Reference) {
        self.events.push(Event::Refresh(p));
    }
    fn remove_connection(&mut self, p: Reference, other: Reference, arg: u32) {
        assert_eq!(arg, 0);
        self.events.push(Event::Detach(p, other));
        self.poles
            .get_mut(&p)
            .unwrap()
            .connections
            .retain(|c| c.other != Some(other));
    }
    fn connection_cells(&mut self, p: Reference, other: Reference) -> Vec<(i32, i32)> {
        self.cells.get(&(p, other)).cloned().unwrap_or_default()
    }
    fn connection_cells_at(&mut self, p:Reference,index:usize) -> Vec<(i32,i32)> {
        let other=self.poles[&p].connections[index].other;
        other.map(|o|self.cells.get(&(p,o)).cloned().unwrap_or_default()).unwrap_or_default()
    }
    fn clear_connections(&mut self, p: Reference, arg: bool) {
        assert!(arg);
        self.events.push(Event::Clear(p));
    }
    fn draw_line(&mut self, run: &mut ToolRun, anchors: [(i32, i32); 2], snap: u32) -> Vec<(usize, usize)> {
        assert_eq!(snap, 0);
        self.events.push(Event::Draw(anchors));
        // A real external geometry primitive in fixtures, never a scripted power-line result.
        assert_eq!(anchors[0].1, anchors[1].1);
        let (a, b) = (anchors[0].0, anchors[1].0);
        let step = if a <= b { 1 } else { -1 };
        run.cells = (0..=(a - b).abs()).map(|i| (a + i * step, anchors[0].1)).collect();
        run.dragged_count = run.cells.len() as u32;
        (0..run.cells.len()).map(|i| (i, i)).collect()
    }
    fn select(&mut self, cell: (i32, i32), color: u32, args: [u32; 3]) {
        assert_eq!(args, [0; 3]);
        self.events.push(Event::Selection(cell, color));
    }
    fn extra_geometry(&mut self, cell: (i32, i32)) -> bool {
        self.events.push(Event::Extra(cell));
        self.extra
    }
    fn notify_pole(&mut self, p: Reference, message: u32, arg: u32) {
        assert_eq!((message, arg), (0x99EF_1145, 0));
        self.events.push(Event::Notify(p));
    }
    fn curve_params(&self) -> CurveParams {
        self.params
    }
    fn base_cleanup(&mut self) {
        self.events.push(Event::Base);
    }
}

impl PowerLineHost for TestWorld {
    fn attempt_neighbor_connections(&mut self, _: &mut ToolRun, _: &mut PowerLineTool) -> bool {
        self.events.push(Event::Attempt);
        self.attempt
    }
    fn place_neighbor_connections(&mut self, _: &mut ToolRun, _: &mut PowerLineTool) {
        self.events.push(Event::Regional);
    }
}
impl ZoneWorld for TestWorld {
    fn query_rectangle(&mut self, _: Rectangle, flags: u32, mask: i32) -> Vec<Reference> {
        assert_eq!((flags, mask), (0, -1));
        self.rectangle_query.clone()
    }
}

fn drag(n: usize, spacing: u32) -> (ToolRun, PowerLineTool) {
    let mut run = ToolRun::new(5, (0, 0), (n as i32 - 1, 0));
    run.cells = (0..n as i32).map(|x| (x, 0)).collect();
    run.dragged_count = n as u32;
    run.valid_count = n as u32;
    let mut tool = PowerLineTool::default();
    tool.anchors = vec![run.start, run.end];
    tool.steps = (0..n).map(|i| (i, i)).collect();
    tool.params.v_3a4 = spacing;
    (run, tool)
}

#[test]
fn pl_g1_g2_spacing_is_in_steps_at_pivots() {
    // Axis pivots use the prior-run first-cell rule; diagonals use anchor-distance equality.
    for (spacing, expected) in [(3, vec![0, 3, 5]), (20, vec![0, 5])] {
        let (mut run, mut tool) = drag(6, spacing);
        run.straight_count = 6;
        let mut w = TestWorld::default();
        let mut failure = 99;
        determine_pole_positions(&mut run, &mut tool, &mut w, &mut failure);
        assert_eq!(tool.poles.iter().map(|p| p.x).collect::<Vec<_>>(), expected);
        assert_eq!(tool.wire_cells, if spacing == 3 { 3 } else { 4 });
        assert_eq!((run.valid_count, failure), (6, 0));
        assert!(w.events.iter().all(|e| !matches!(e, Event::Create(_))));
    }
    assert_eq!(pivot(&[(0, 0), (1, 0), (2, 0)], (0, 2), 0, 0, Some((2, 0))), Some(2));
    assert_eq!(pivot(&[(0, 1), (1, 1), (2, 1)], (0, 2), 0, 0, Some((4, 3))), Some(2));
}

#[test]
fn pl_g3_g4_segment_ray_is_inclusive_and_clearance_does_not_raise_pole_base() {
    for (hit, expected) in [([40.0, 17.5, 8.0], vec![0, 1, 3]), ([41.0, 17.5, 8.0], vec![0, 3])] {
        let (mut run, mut tool) = drag(4, 20);
        run.straight_count = 4;
        let mut w = TestWorld {
            hit: Some(hit),
            ..TestWorld::default()
        };
        let mut failure = 0;
        determine_pole_positions(&mut run, &mut tool, &mut w, &mut failure);
        assert_eq!(tool.poles.iter().map(|p| p.x).collect::<Vec<_>>(), expected);
        assert!(w.events.contains(&Event::Ray([8.0, 17.5, 8.0], [1.0, 0.0, 0.0])));
        assert_eq!(compute_pole_position(0, 0, &mut w), [8.0, 10.5, 8.0]);
    }
}

#[test]
fn pl_g5_g6_sentinel_nan_and_special_height() {
    let mut w = TestWorld {
        corners: [-f32::MAX; 4],
        ..TestWorld::default()
    };
    assert_eq!(compute_pole_position(3, 10, &mut w), [56.0, -f32::MAX, 168.0]);
    w.corners = [f32::NAN, 4.0, 3.0, 2.0];
    assert_eq!(compute_pole_position(0, 0, &mut w)[1], 4.5);
    w.special = true;
    w.height = 270.5;
    w.width = 8.0;
    assert_eq!(compute_pole_position(3, 10, &mut w), [28.0, 271.0, 84.0]);
}

#[test]
fn pl_cn3_independent_default_curve_and_literal_sample_count() {
    let p = CurveParams::default();
    for (length, expected) in [(32.0, 5), (45.0, 5), (48.0, 6), (80.0, 10)] {
        assert_eq!(control_points([0.0, 20.0, 0.0], [length, 20.0, 0.0], p).count, expected);
    }
    let a = [0.0, 20.0, 0.0];
    let b = [32.0, 20.0, 0.0];
    let c = control_points(a, b, p);
    for (actual, expected) in c.first.into_iter().zip([12.8, 18.464, 0.0]) {
        assert!((actual - expected).abs() < 0.00001);
    }
    for (actual, expected) in c.second.into_iter().zip([19.2, 18.464, 0.0]) {
        assert!((actual - expected).abs() < 0.00001);
    }
    let main = sample_cubic(a, c.first, c.second, b, 4);
    for (point, expected) in main.iter().zip([
        [8.6, 19.136, 0.0],
        [16.0, 18.848, 0.0],
        [23.4, 19.136, 0.0],
        [32.0, 20.0, 0.0],
    ]) {
        for (actual, expected) in point.iter().zip(expected) {
            assert!((actual - expected).abs() < 0.00002);
        }
    }
    let extra = sample_cubic(a, c.first, c.second, b, 3);
    assert_eq!(extra.len(), 3);
    assert_ne!(extra[0], a);
    // No invented zero-length/count clamp: arithmetic result remains observable.
    let c = control_points(a, a, CurveParams { minimum_count: 0, ..p });
    assert_eq!(c.count, 0);
    assert!(c.first[0].is_nan());
}

#[test]
fn pl_cn1_cn2_cn4_connection_gates_and_owned_storage() {
    let mut w = TestWorld {
        extra: true,
        ..TestWorld::default()
    };
    let a = w.pole(1, (0, 0));
    let b = w.pole(2, (2, 0));
    let wires = vec![
        WireCell {
            wire: Some(Reference(8)),
            ..WireCell::default()
        };
        2
    ];
    connect(&mut w, a, None, Some(&wires)).unwrap();
    assert!(w.events.is_empty());
    connect(&mut w, a, Some(b), Some(&wires)).unwrap();
    let record = &w.poles[&a].connections[0];
    assert_eq!((record.geometry.len(), record.extra.len()), (4, 4));
    assert!(record.geometry.iter().all(|v| v.len() == 5));
    assert!(record.extra.iter().all(|v| v.len() == 3));
    assert_eq!(record.geometry[0][0], [8.0, 20.0, 8.0]);
    assert_ne!(record.extra[0][0], record.geometry[0][0]);
    assert_eq!(record.wires, vec![Reference(8); 2]);
    assert_eq!(wires[0].wire, Some(Reference(8)));
    assert_eq!(w.events.iter().filter(|e| **e == Event::Add(Reference(8))).count(), 2);
    let snapshot = w.poles[&a].connections[0].clone();
    w.poles.get_mut(&a).unwrap().connections[0].geometry[0][0][0] = 123.0;
    assert_eq!(snapshot.geometry[0][0][0], 8.0);
    w.events.clear();
    connect(&mut w, a, Some(b), None).unwrap();
    assert!(w.events.is_empty());
    connect(&mut w, b, Some(a), Some(&wires)).unwrap();
    assert!(w.poles[&b].connections[0].geometry.is_empty());
    assert!(w.poles[&b].connections[0].wires.is_empty());
    assert!(w.poles[&b].dirty);
    assert!(!w.events.contains(&Event::Notify(b)));
}

#[test]
fn pl_c1_c2_zone_asymmetry_and_last_qualifying_occupant() {
    let mut w = TestWorld {
        zones: [2, 2, 14].into(),
        ..TestWorld::default()
    };
    assert_eq!(classify(&mut w, (3, 9)), 0);
    assert_eq!(
        &w.events[..3],
        &[Event::Zone((3, 9)), Event::Zone((3, 9)), Event::Zone((9, 9))]
    );
    for (occupants, expected) in [
        (vec![Reference(1), Reference(2)], 3),
        (vec![Reference(2), Reference(1)], 2),
    ] {
        let mut w = TestWorld::default();
        w.occupants.insert((0, 0), occupants);
        w.classes.insert(Reference(1), 0x2781_28A0);
        w.classes.insert(Reference(2), 0xC772_BF98);
        assert_eq!(classify(&mut w, (0, 0)), expected);
        assert_eq!(w.events.iter().filter(|e| matches!(e, Event::Release(_))).count(), 2);
    }
}

#[test]
fn pl_c3_c4_last_pole_and_wire_qi_ownership() {
    for pole in [true, false] {
        let mut w = TestWorld::default();
        for id in 1..=3 {
            let p = w.pole(id, (4, 5));
            w.register(Reference(id + 10), (4, 5), if pole { POLE_IID } else { WIRE_IID }, p);
        }
        w.positions.insert(Reference(13), [200.0, 20.0, 200.0]);
        assert_eq!(probe(&mut w, (4, 5), pole), Some(Reference(if pole { 2 } else { 3 })));
        assert!(w.events.contains(&Event::Release(Reference(1))));
        for id in 11..=13 {
            assert!(w.events.contains(&Event::Release(Reference(id))));
        }
        if pole {
            assert!(w.events.contains(&Event::Release(Reference(3))));
        }
    }
}

#[test]
fn pl_i1_to_i6_ordered_descriptor_forced_and_or_predicates() {
    for (diagonal, forced, step, duplicate, accepted) in [
        (false, false, 1, false, true),
        (true, true, 1, false, false),
        (false, false, 1, true, false),
        (false, true, 1, true, true),
        (false, true, 3, false, false),
    ] {
        let mut w = TestWorld::default();
        let a = w.pole(1, (4, 5));
        let b = w.pole(2, if diagonal { (8, 9) } else { (8, 5) });
        w.wires.insert(Reference(3), [Some(a), Some(b)]);
        let mut tool = PowerLineTool::default();
        if duplicate {
            tool.append_pole((4, 90), 0, false, -1, SavedConnection::default());
        }
        let mut pending = forced;
        let d = intercept(&tool, &mut w, Reference(3), step, 3, forced, &mut pending);
        assert_eq!(d.endpoints[0].is_some(), accepted);
        assert_eq!(w.events.last(), Some(&Event::Release(Reference(3))));
        assert_eq!(w.events.contains(&Event::Release(a)), !accepted);
        if accepted {
            assert!(pending);
        }
    }
    let mut tool = PowerLineTool::default();
    let d = SavedConnection {
        endpoints: [Some(Reference(1)), Some(Reference(2))],
        enabled: [1, 1],
    };
    tool.append_pole((99, 99), 0, false, -1, d);
    assert!(!tool.descriptor_present(SavedConnection {
        endpoints: [d.endpoints[1], d.endpoints[0]],
        ..d
    }));
}

#[test]
fn pl_s1_s2_enable_and_reference_copy_do_not_deduplicate() {
    let mut w = TestWorld::default();
    let a = w.pole(1, (0, 0));
    let b = w.pole(2, (0, 0));
    w.poles.get_mut(&a).unwrap().connections.push(Connection {
        other: Some(b),
        ..Connection::default()
    });
    enable_connection(&mut w, a, b, true);
    assert_eq!(w.poles[&a].connections[0].enabled, 0);
    w.poles.get_mut(&b).unwrap().cell = (0, 1);
    enable_connection(&mut w, a, b, true);
    assert_eq!(w.poles[&a].connections[0].enabled, 1);
    assert!(w.events.is_empty());
    let mut destination = vec![Reference(7)];
    let source = vec![
        WireCell {
            wire: Some(Reference(8)),
            ..WireCell::default()
        };
        2
    ];
    copy_wire_references(&mut w, &mut destination, Some(&source));
    assert_eq!(destination, vec![Reference(7), Reference(8), Reference(8)]);
    assert_eq!(w.events, vec![Event::Add(Reference(8)); 2]);
}

#[test]
fn pl_h1_valid_skip_and_invalid_red_are_separate_passes() {
    let (mut run, mut tool) = drag(3, 20);
    run.valid_count = 2;
    for i in 0..3 {
        tool.append_pole((i, 0), i as usize, false, -1, SavedConnection::default());
    }
    let mut w = TestWorld::default();
    terrain_highlight(&run, &tool, &mut w, 1);
    assert_eq!(w.events, vec![Event::Selection((2, 0), 0x80FF_0000)]);
}

#[test]
fn pl_p1_u1_u2_preview_ownership_and_manager_null_asymmetry() {
    let (mut run, mut tool) = drag(2, 20);
    let mut w = TestWorld::default();
    let mut failure = 0;
    tool.append_pole((0, 0), 0, false, -1, SavedConnection::default());
    tool.append_pole((1, 0), 1, true, -1, SavedConnection::default());
    place_poles(&mut run, &mut tool, &mut w, &mut failure, true).unwrap();
    let create_positions: Vec<_> = w
        .events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| matches!(e, Event::Create(_)).then_some(i))
        .collect();
    let first_notify = w.events.iter().position(|e| matches!(e, Event::Notify(_))).unwrap();
    assert!(create_positions.into_iter().all(|i| i < first_notify));
    assert!(!w.events.iter().any(|e| matches!(e, Event::Wire(_))));
    let created = tool.newly_created.clone();
    w.events.clear();
    undo_preview(&mut tool, &mut w);
    assert_eq!(w.events.last(), Some(&Event::Base));
    for p in created {
        assert!(w
            .events
            .windows(3)
            .any(|v| v == [Event::Highlight(p, 0), Event::Remove(p), Event::Release(p)]));
    }
    tool.newly_created.push(Reference(99));
    w.events.clear();
    w.manager = false;
    undo_preview(&mut tool, &mut w);
    assert_eq!(w.events, vec![Event::Highlight(Reference(99), 0), Event::Base]);
}

#[test]
fn pl_ep1_ep2_saved_endpoint_flags_and_duplicate_repair_entries() {
    for same in [false, true] {
        let mut w = TestWorld::default();
        let a = w.pole(1, (0, 0));
        let b = if same { a } else { w.pole(2, (4, 0)) };
        let (mut run, mut tool) = drag(3, 20);
        tool.real = true;
        let saved = SavedConnection {
            endpoints: [Some(a), Some(b)],
            enabled: if same { [1, 1] } else { [1, 0] },
        };
        tool.append_pole((2, 0), 2, false, -1, saved);
        place_poles(&mut run, &mut tool, &mut w, &mut 0, false).unwrap();
        assert_eq!(&w.events[2..4], &[Event::Detach(a, b), Event::Detach(b, a)]);
        assert_eq!(
            w.events.iter().filter(|e| **e == Event::Add(a)).count(),
            if same { 2 } else { 1 }
        );
        assert!(tool.poles[0].saved.endpoints.iter().all(Option::is_none));
        assert!(!w.events.iter().any(|e| matches!(e, Event::Draw(_))));
    }
}

#[test]
fn pl_n1_n2_n3_wrapper_attempt_and_real_success_gate() {
    for (attempt, real, regional) in [(false, true, false), (true, false, false), (true, true, true)] {
        let (mut run, mut tool) = drag(4, 20);
        run.straight_count = 4;
        let mut w = TestWorld {
            attempt,
            ..TestWorld::default()
        };
        let mut state = ToolState::new(5);
        assert!(do_network_specific_placement(
            &mut state, &mut tool, &mut w, &mut run, 1, real
        ).unwrap());
        assert!(w.events.contains(&Event::Attempt));
        assert_eq!(w.events.contains(&Event::Regional), regional);
        assert!(tool.poles.is_empty());
    }
}

#[test]
fn pl_co1_co2_structural_copy_and_erase_keep_independent_inner_buffers() {
    let p = PoleInfo {
        links: vec![(2, 3), (4, 5)],
        saved: SavedConnection {
            endpoints: [Some(Reference(7)), Some(Reference(8))],
            enabled: [1, 0],
        },
        ..PoleInfo::default()
    };
    let mut copy = copy_pole_record(&p);
    copy.links[0] = (90, 90);
    assert_eq!(p.links[0], (2, 3));
    assert_eq!(copy.saved, p.saved);
    let mut poles = repeat_pole_record(&p, 3);
    let capacity = poles.capacity();
    erase_pole_range(&mut poles, 1..2);
    assert_eq!((poles.len(), poles.capacity()), (2, capacity));
    poles[0].links.clear();
    assert_eq!(poles[1].links.len(), 2);
    insert_pole_copies(&mut poles, 1, &p, 2, true);
    assert_eq!(poles.len(), 3);
    assert_eq!(poles.capacity(), 4);
    destroy_pole_vector(poles);
}

#[test]
fn pl_rl1_rl2_rl3_unsigned_rectangle_and_null_other() {
    let rectangle = Rectangle {
        minimum: (0, 0),
        maximum: (2, 2),
    };
    let (mut run, mut tool) = drag(4, 20);
    let mut w = TestWorld::default();
    let cells = vec![(2, 2), (3, 2), (4, 2)];
    let scan = Relocation {
        start: 0,
        stop: 3,
        increment: 1,
        unused_ninth: Some(Reference(999)),
        other: None,
    };
    assert_eq!(
        find_new_pole_position(&mut run, &mut tool, &mut w, rectangle, &cells, scan, &mut 0).unwrap(),
        Some((3, 2))
    );
    assert!(!w.events.contains(&Event::Zone((2, 2))));
    assert!(!w.events.iter().any(|e| matches!(e, Event::Draw(_))));
    let p = Reference(101);
    assert!(w.events.windows(2).any(|v| v == [Event::Insert(p), Event::Release(p)]));
    w.events.clear();
    assert_eq!(
        find_new_pole_position(
            &mut run,
            &mut tool,
            &mut w,
            rectangle,
            &cells,
            Relocation {
                start: 2,
                stop: 2,
                ..scan
            },
            &mut 0
        )
        .unwrap(),
        None
    );
    assert!(w.events.is_empty());
}

#[test]
fn pl_z1_zu1_zu2_disabled_and_signed_pole_query_order() {
    let rectangle = Rectangle {
        minimum: (0, 0),
        maximum: (2, 2),
    };
    let (mut run, mut tool) = drag(3, 20);
    let mut w = TestWorld::default();
    update_on_zone_change(&mut run, &mut tool, &mut w, false, rectangle, &mut 0).unwrap();
    assert!(!tool.real && w.events.is_empty());
    let outside = w.pole(1, (-1, 1));
    w.qis.insert((outside, POLE_IID), outside);
    w.qis.insert((outside, WIRE_IID), outside);
    let inside = w.pole(2, (1, 1));
    w.qis.insert((inside, POLE_IID), inside);
    w.rectangle_query = vec![outside, inside];
    update_on_zone_change(&mut run, &mut tool, &mut w, true, rectangle, &mut 0).unwrap();
    assert!(!w.events.contains(&Event::Qi(outside, WIRE_IID)));
    assert!(w
        .events
        .ends_with(&[Event::Clear(inside), Event::Remove(inside), Event::Release(inside)]));
    assert!(tool.real);
}

#[test]
fn pl_zw1_zw2_zw3_zu3_endpoint_overlap_and_withheld_backward_split() {
    for null in [false, true] {
        let (mut run, mut tool) = drag(4, 20);
        let mut w = TestWorld::default();
        let a = w.pole(1, (0, 0));
        let b = w.pole(2, (3, 0));
        let c = w.pole(3, (6, 0));
        let shared = if null { None } else { Some(b) };
        for (id, endpoints) in [(8, [Some(a), shared]), (9, [shared, Some(c)])] {
            let wire = Reference(id);
            w.wires.insert(wire, endpoints);
            w.wire_cells.insert(wire, (1, 0));
            w.qis.insert((wire, WIRE_IID), wire);
            w.rectangle_query.push(wire);
        }
        w.cells.insert((a, b), vec![(0, 0), (1, 0), (2, 0), (1, 0), (3, 0)]);
        let result = update_on_zone_change(
            &mut run,
            &mut tool,
            &mut w,
            true,
            Rectangle {
                minimum: (1, 0),
                maximum: (2, 0),
            },
            &mut 0,
        )
        .unwrap();
        assert_eq!(result.withheld_wire_splits, if null { 0 } else { 1 });
        assert!(!w.events.iter().any(|e| matches!(e, Event::Detach(..) | Event::Draw(_))));
        assert_eq!(
            w.events.iter().filter(|e| **e == Event::Release(Reference(9))).count(),
            2
        ); // QI and occupant
    }
}

#[test]
fn pl_b2_b3_building_termination_and_zone_reference_asymmetry() {
    for zone in [false,true] {
        let (mut run,mut tool)=drag(2,20); run.straight_count=2;
        let mut w=TestWorld::default();
        let a=w.pole(1,(10,10)); let b=w.pole(2,(15,10));
        w.wires.insert(Reference(8),[Some(a),Some(b)]);
        w.register(Reference(8),(0,0),WIRE_IID,Reference(8));
        if zone {
            // First cell zone=0; second cell three uncached zone reads reject.
            w.zones=[0,1,1,1].into();
        } else {
            w.occupants.insert((1,0),vec![Reference(9)]); w.classes.insert(Reference(9),0x2781_28A0);
        }
        let mut failure=0; determine_pole_positions(&mut run,&mut tool,&mut w,&mut failure);
        assert!(tool.poles.is_empty());
        assert_eq!(w.events.contains(&Event::Release(a)),!zone);
        assert_eq!(w.events.contains(&Event::Release(b)),!zone);
        assert_eq!(failure,if zone{0}else{FAIL_BUILDING});
    }
    for building in [true,false] {
        let (mut run,mut tool)=drag(4,20); run.straight_count=4;
        let mut w=TestWorld::default(); w.occupants.insert((1,0),vec![Reference(9)]);
        w.classes.insert(Reference(9),if building{0x2781_28A0}else{0xC772_BF98});
        let mut failure=0; determine_pole_positions(&mut run,&mut tool,&mut w,&mut failure);
        if building { assert_eq!(failure,FAIL_BUILDING); assert_eq!(run.valid_count,1); }
        else { assert_eq!(tool.wire_cells,2); assert_eq!(run.valid_count,4); }
    }
}

#[test]
fn pl_p2_real_repair_calls_draw_once_and_recursive_place_disallows_more_repairs() {
    for allow in [false,true] {
        let (mut run,mut tool)=drag(3,20); tool.real=true;
        let mut w=TestWorld::default(); let old=w.pole(1,(0,0));
        tool.append_pole((2,0),2,false,-1,SavedConnection {endpoints:[Some(old),None],enabled:[1,0]});
        place_poles(&mut run,&mut tool,&mut w,&mut 0,allow).unwrap();
        assert_eq!(w.events.iter().filter(|e|matches!(e,Event::Draw(_))).count(),usize::from(allow));
    }
}

#[test]
fn ordered_adjacency_groups_link_every_pair_without_dedup_and_do_not_invent_a_producer() {
    let (mut run,mut tool)=drag(2,20); let mut w=TestWorld::default();
    tool.append_pole((0,0),0,false,-1,SavedConnection::default());
    tool.append_pole((1,0),1,false,-1,SavedConnection::default());
    tool.groups=vec![vec![0,1,1,99]];
    place_poles(&mut run,&mut tool,&mut w,&mut 0,false).unwrap();
    assert_eq!(tool.poles[0].links,vec![(1,-1);4]);
    assert_eq!(tool.poles[1].links,vec![(0,-1);4]);
    assert_eq!(tool.poles[0].mask,tool.poles[1].mask);
    assert!(tool.groups.is_empty());
}

#[test]
fn invalid_sample_count_is_reported_without_clamping_or_synthesizing_geometry() {
    let mut w=TestWorld {params:CurveParams {minimum_count:0,sample_distance:-8.0,..CurveParams::default()},..TestWorld::default()};
    let a=w.pole(1,(0,0)); let b=w.pole(2,(2,0));
    assert_eq!(connect(&mut w,a,Some(b),None),Err(GeometryBoundary::UnsupportedSampleCount(0)));
    assert!(!w.poles[&a].dirty && w.poles[&a].connections.is_empty());
}
