// SPDX-License-Identifier: GPL-3.0-or-later
//! Published §8 literals are portable-executed here, NEVER native goldens.
use super::*;
use storage::{Allocator, Event, Foreign, Node, Storage};

fn body(position: Pair, extents: Pair) -> Body {
    Body {
        position,
        extents,
        axis: [1.0, 0.0],
        velocity: [0.0, 0.0],
        angular_velocity: 0.0,
    }
}
fn assert_pair(actual: Pair, expected: Pair) {
    // Zero signs are checked separately where the contract fixes them.
    assert_eq!(actual, expected);
    for i in 0..2 {
        if expected[i] != 0.0 {
            assert_eq!(actual[i].to_bits(), expected[i].to_bits());
        }
    }
}

#[test]
fn rb_p1_p2_unsigned_mixed_halves() {
    for (a, b) in [(0x0002_0009, 0x0005_0003), (0x0005_0003, 0x0002_0009)] {
        assert!(primary_sector(0x0002_0003, a, b));
        assert!(!primary_sector(a, a, b));
        assert!(!primary_sector(b, a, b));
    }
    assert!(primary_sector(0xffff_0000, 0xffff_0000, 0xffff_0000));
    assert!(primary_sector(0x0001_0002, 0x8000_0002, 0x0001_ffff));
    assert!(!primary_sector(0x8000_0002, 0x8000_0002, 0x0001_ffff));
}

#[test]
fn rb_s1_s2_direction_zero_and_negative_zero_tie() {
    let b = body([10.0, 20.0], [2.0, 3.0]);
    assert_pair(support_point([0.0, 0.0], [1.0, 0.0], &b).unwrap(), [8.0, 23.0]);
    assert_pair(support_point([0.0, 0.0], [0.0, 0.0], &b).unwrap(), [12.0, 23.0]);
    assert_pair(support_point([0.0, 0.0], [-0.0, -0.0], &b).unwrap(), [12.0, 23.0]);
    assert_pair(support_point(b.position, [1.0, 0.0], &b).unwrap(), [12.0, 23.0]);
}

#[test]
fn support_y_store_is_not_regrouped() {
    let mut b = body([0.0, 16_777_216.0], [1.0, 2.0]);
    b.axis = [0.0, 1.0];
    // Y first sum 2^24+1 rounds to 2^24. The perpendicular adds zero.
    assert_pair(support_point(b.position, [0.0, 0.0], &b).unwrap(), [-2.0, 16_777_216.0]);
    // X does not store its first addition: (2^24 + 1) - 2 survives as 2^24-1.
    // This uses a raw nonorthonormal record, so only the numeric helper is
    // tested directly; it grants no geometric admission to that record.
    assert_eq!(
        numeric::store(numeric::add(numeric::add(16_777_216.0, 1.0).unwrap(), -2.0).unwrap()).unwrap(),
        16_777_215.0
    );
}

#[test]
fn rb_v1_swapped_and_v2_linear_only() {
    let mut a = body([1.0, 2.0], [2.0, 3.0]);
    let mut b = body([-2.0, 1.0], [2.0, 3.0]);
    a.velocity = [3.0, 4.0];
    a.angular_velocity = 2.0;
    b.velocity = [-1.0, 5.0];
    b.angular_velocity = -1.0;
    let before = (a, b);
    assert_pair(relative_velocity(&a, &b, [4.0, 6.0]).unwrap(), [9.0, -11.0]);
    assert_pair(relative_velocity(&b, &a, [4.0, 6.0]).unwrap(), [-9.0, 11.0]);
    assert_eq!((a, b), before);
    a.angular_velocity = 0.0;
    b.angular_velocity = 0.0;
    assert_pair(relative_velocity(&a, &b, [4.0, 6.0]).unwrap(), [-4.0, 1.0]);
}

#[derive(Default)]
struct Writes(Vec<(OutputField, f32)>);
impl OutputObserver for Writes {
    fn stored(&mut self, f: OutputField, v: f32) {
        self.0.push((f, v));
    }
}

#[test]
fn rb_c1_through_c5_independent_contacts_and_sentinels() {
    let a = body([0.0, 0.0], [2.0, 1.0]);
    // Each expected contact comes from the public table, not an implementation.
    for (velocity, expected, point, normal) in [
        ([0.0, 0.0], true, [1.0, 1.0], [1.0, 0.0]),
        ([-1.0, 0.0], true, [1.0, 1.0], [1.0, 0.0]),
        ([1.0, 0.0], true, [2.0, 1.0], [0.0, 1.0]),
        ([1.0, -1.0], false, [91.0, 92.0], [81.0, 82.0]),
        ([0.0, 1.0], true, [1.0, 1.0], [1.0, 0.0]),
    ] {
        let mut b = body([3.0, 0.0], [2.0, 1.0]);
        b.velocity = velocity;
        let mut p = [91.0, 92.0];
        let mut n = [81.0, 82.0];
        let mut writes = Writes::default();
        assert_eq!(
            test_for_collision_observed(&a, &b, &mut n, &mut p, &mut writes),
            Ok(expected)
        );
        assert_pair(p, point);
        assert_pair(n, normal);
        let expected_order = if velocity == [1.0, -1.0] {
            vec![]
        } else if velocity == [1.0, 0.0] {
            vec![
                OutputField::PointX,
                OutputField::PointY,
                OutputField::NormalX,
                OutputField::NormalY,
            ]
        } else {
            vec![
                OutputField::PointY,
                OutputField::PointX,
                OutputField::NormalX,
                OutputField::NormalY,
            ]
        };
        assert_eq!(writes.0.iter().map(|&(f, _)| f).collect::<Vec<_>>(), expected_order);
    }
}

#[test]
fn rb_c6_tangent_later_strict_improvement_and_c7_early_separation() {
    let a = body([0.0, 0.0], [2.0, 1.0]);
    let b = body([4.0, 0.0], [2.0, 1.0]);
    let mut p = [91.0, 92.0];
    let mut n = [81.0, 82.0];
    let mut writes = Writes::default();
    assert_eq!(
        test_for_collision_observed(&a, &b, &mut n, &mut p, &mut writes),
        Ok(true)
    );
    assert_pair(p, [2.0, 1.0]);
    assert_pair(n, [0.0, 1.0]);
    assert_eq!(
        writes.0.iter().map(|&(f, _)| f).collect::<Vec<_>>(),
        vec![
            OutputField::PointY,
            OutputField::PointX,
            OutputField::NormalX,
            OutputField::NormalY,
            OutputField::PointX,
            OutputField::PointY,
            OutputField::NormalX,
            OutputField::NormalY
        ]
    );
    let b = body([5.0, 0.0], [2.0, 1.0]);
    p = [f32::from_bits(0x7fc0_1234), -0.0];
    n = [81.0, 82.0];
    writes.0.clear();
    let old_p = p.map(f32::to_bits);
    let old_n = n.map(f32::to_bits);
    assert_eq!(
        test_for_collision_observed(&a, &b, &mut n, &mut p, &mut writes),
        Ok(false)
    );
    assert_eq!(p.map(f32::to_bits), old_p);
    assert_eq!(n.map(f32::to_bits), old_n);
    assert!(writes.0.is_empty());
}

#[test]
fn accepted_candidate_can_return_false_with_writes_above_both_constants() {
    // §6.1's published branch, instantiated with exact powers of two. This is
    // a portable derived fixture, NOT a native golden or new published vector.
    let h = f32::from_bits((126 + 127) << 23); // 2^126
    let a = body([0.0, 0.0], [h, h]);
    let b = body([h * 1.5, 0.0], [h, h]);
    let mut p = [91.0, 92.0];
    let mut n = [81.0, 82.0];
    let mut writes = Writes::default();
    assert!(f64::from(h * 0.5) > 1e35 && h * 0.5 > f32::from_bits(0x7cf0_bdc2));
    assert_eq!(
        test_for_collision_observed(&a, &b, &mut n, &mut p, &mut writes),
        Ok(false)
    );
    assert_pair(p, [h * 0.5, h]);
    assert_pair(n, [1.0, 0.0]);
    assert_eq!(writes.0.len(), 4);
    assert_eq!(writes.0[0], (OutputField::PointY, h));
}

#[test]
fn explicit_numeric_errors_and_no_false_contract() {
    assert_eq!(
        numeric::add(1.0, 2_f64.powi(-60)),
        Err(Unsupported::InexactUnstoredArithmetic)
    );
    assert_eq!(
        numeric::store(f64::from(f32::MAX) * 2.0),
        Err(Unsupported::Binary32StoreRange)
    );
    assert_eq!(
        numeric::mul(f64::from(f32::MIN_POSITIVE), 0.5),
        Err(Unsupported::NonFiniteOrSubnormal)
    );
    assert_eq!(numeric::store(16_777_217.0).unwrap(), 16_777_216.0);
    assert_eq!(numeric::store(16_777_219.0).unwrap(), 16_777_220.0);
    assert_eq!(numeric::store(-0.0).unwrap().to_bits(), (-0_f32).to_bits());
    let a = body([0.0, 0.0], [-1.0, 1.0]);
    let b = body([3.0, 0.0], [2.0, 1.0]);
    assert_eq!(
        test_for_collision(&a, &b, &mut [81.0, 82.0], &mut [91.0, 92.0]),
        Err(Unsupported::NegativeExtent)
    );
    assert_eq!(advance_time(0), Err(Unsupported::InnerAdvance));
    assert_eq!(resolve_collision(), Err(Unsupported::CollisionResponse));
    assert_eq!(do_collisions(), Err(Unsupported::CollisionCycle));
    assert_eq!(map_sector(), Err(Unsupported::SectorMapping));
    assert_eq!(add_object(), Err(Unsupported::RegistrationLifetime));
}

#[derive(Default)]
struct Calls(Vec<Event>);
impl Foreign for Calls {
    fn free_allocation(&mut self, allocation: u32, allocator: Allocator) {
        self.0.push(Event::FreeAllocation { allocation, allocator });
    }
    fn free_pair_node(&mut self, node: u32) {
        self.0.push(Event::FreePair(node));
    }
    fn remove_from_sector_map(&mut self, body: u32) {
        self.0.push(Event::RemoveSector(body));
    }
}
fn list(storage: &mut Storage, root: u32, addresses: &[(u32, u8, u8, [u32; 2])]) {
    let ids: Vec<_> = std::iter::once(root).chain(addresses.iter().map(|v| v.0)).collect();
    for (i, &id) in ids.iter().enumerate() {
        let (live, class, pair) = if i == 0 {
            (1, 0, [0, 0])
        } else {
            (addresses[i - 1].1, addresses[i - 1].2, addresses[i - 1].3)
        };
        storage.nodes.insert(
            id,
            Node {
                next: ids[(i + 1) % ids.len()],
                previous: ids[(i + ids.len() - 1) % ids.len()],
                live,
                allocator_class: class,
                pair,
            },
        );
    }
}

#[test]
fn rb_l1_empty_sweep_order_and_l2_stable_adjacent_dead_nodes() {
    let mut s = Storage::default();
    for root in [100, 200, 300] {
        list(&mut s, root, &[]);
    }
    let mut foreign = Calls::default();
    let mut events = vec![];
    s.remove_dead_objects([100, 200, 300], &mut foreign, &mut events)
        .unwrap();
    assert_eq!(events, vec![Event::Sweep(300), Event::Sweep(200), Event::Sweep(100)]);
    assert!(foreign.0.is_empty());
    list(
        &mut s,
        300,
        &[(320, 7, 0, [0, 0]), (340, 0, 8, [0, 0]), (360, 0, 0, [0, 0])],
    );
    events.clear();
    s.remove_dead_objects([100, 200, 300], &mut foreign, &mut events)
        .unwrap();
    assert_eq!(
        events,
        vec![
            Event::Sweep(300),
            Event::SetNext { node: 320, next: 360 },
            Event::SetPrevious {
                node: 360,
                previous: 320
            },
            Event::FreeAllocation {
                allocation: 336,
                allocator: Allocator::Class(8)
            },
            Event::SetNext { node: 320, next: 300 },
            Event::SetPrevious {
                node: 300,
                previous: 320
            },
            Event::FreeAllocation {
                allocation: 356,
                allocator: Allocator::Class(0)
            },
            Event::Sweep(200),
            Event::Sweep(100)
        ]
    );
    assert_eq!(s.nodes[&300].next, 320);
    assert_eq!(s.nodes[&320].next, 300);
    assert_eq!(
        foreign.0,
        vec![
            Event::FreeAllocation {
                allocation: 336,
                allocator: Allocator::Class(8)
            },
            Event::FreeAllocation {
                allocation: 356,
                allocator: Allocator::Class(0)
            }
        ]
    );
}

#[test]
fn allocator_boundary_single_dead_and_invalid_storage() {
    let mut s = Storage::default();
    list(&mut s, 100, &[(120, 0, 9, [0, 0])]);
    let mut calls = Calls::default();
    let mut events = vec![];
    s.sweep_object_list(100, &mut calls, &mut events).unwrap();
    assert_eq!(s.nodes[&100].next, 100);
    assert_eq!(s.nodes[&100].previous, 100);
    assert_eq!(
        calls.0,
        vec![Event::FreeAllocation {
            allocation: 116,
            allocator: Allocator::Global
        }]
    );
    s.nodes.get_mut(&100).unwrap().next = 999;
    assert_eq!(
        s.sweep_object_list(100, &mut calls, &mut events),
        Err(Unsupported::InvalidStableList)
    );
}

#[test]
fn rb_l3_remove_all_raw_matches_then_sector_then_defer_object_free() {
    let mut s = Storage::default();
    list(
        &mut s,
        100,
        &[
            (120, 1, 0, [128, 500]),
            (140, 1, 0, [600, 128]),
            (160, 1, 0, [600, 500]),
            (180, 1, 0, [128, 128]),
        ],
    );
    // Object storage is distinct from the pair list (body=208=node200+8).
    // Use body128 in the pair literals; object node120 would overlap the pair
    // list, so install the object node at its own raw identity instead.
    list(&mut s, 400, &[(200, 1, 0, [0, 0])]);
    for node in s.nodes.values_mut() {
        for p in &mut node.pair {
            if *p == 128 {
                *p = 208;
            }
        }
    }
    let mut calls = Calls::default();
    let mut events = vec![];
    s.remove_object(100, 208, 7, &mut calls, &mut events).unwrap();
    assert_eq!(
        calls.0,
        vec![
            Event::FreePair(120),
            Event::FreePair(140),
            Event::FreePair(180),
            Event::RemoveSector(208)
        ]
    );
    assert_eq!(
        events[events.len() - 2..],
        [Event::RemoveSector(208), Event::SetLive { body: 208, value: 0 }]
    );
    assert_eq!(s.nodes[&100].next, 160);
    assert_eq!(s.nodes[&160].next, 100);
    assert_eq!(s.nodes[&200].live, 0);
    assert!(s.nodes.contains_key(&200));
    calls.0.clear();
    events.clear();
    s.remove_object(100, 208, 0, &mut calls, &mut events).unwrap();
    assert!(calls.0.is_empty());
    assert_eq!(events, vec![Event::SetLive { body: 208, value: 0 }]);
    s.sweep_object_list(400, &mut calls, &mut events).unwrap();
    assert_eq!(
        calls.0,
        vec![Event::FreeAllocation {
            allocation: 196,
            allocator: Allocator::Class(0)
        }]
    );
}

#[test]
fn rb_a1_phase_plan_preserves_timestep_bits_and_has_no_fake_inner_success() {
    for bits in [
        0,
        (-0_f32).to_bits(),
        (-1_f32).to_bits(),
        f32::NAN.to_bits(),
        0x3f00_0000,
    ] {
        assert_eq!(
            advance_phases(bits),
            [
                AdvancePhase::Inner {
                    root_offset: 0,
                    dt_bits: bits
                },
                AdvancePhase::Inner {
                    root_offset: 4,
                    dt_bits: bits
                },
                AdvancePhase::Collisions
            ]
        );
        assert_eq!(advance_time(bits), Err(Unsupported::InnerAdvance));
    }
}
