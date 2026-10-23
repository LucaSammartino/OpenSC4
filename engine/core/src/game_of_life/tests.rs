// SPDX-License-Identifier: GPL-3.0-or-later
// Literal proposed vectors from game-of-life.md Â§6, exercised against production
// methods. No native execution claim and no second next-grid implementation.
use super::*;
use std::collections::VecDeque;

const L: u64 = 11;
const D: u64 = 12;
const K: u64 = 13;
const CALLBACKS: Callbacks = Callbacks {
    live: Some(L),
    dead: Some(D),
    context: Some(K),
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Call {
    identity: u64,
    x: u32,
    y: u32,
    context: Option<u64>,
    c_id: Allocation,
    p_id: Allocation,
    c: Vec<u8>,
    p: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Effect {
    Owner(Event),
    Construct(u32),
    Draw(u32),
    Callback(u64, u32, u32),
    ViewClear,
}

#[derive(Default)]
struct Recorder {
    effects: Vec<Effect>,
    calls: Vec<Call>,
    draws: VecDeque<u32>,
    allocations: VecDeque<bool>,
}

impl Recorder {
    fn reset(&mut self) {
        self.effects.clear();
        self.calls.clear();
    }

    fn owner_events(&self) -> Vec<Event> {
        self.effects
            .iter()
            .filter_map(|e| match e {
                Effect::Owner(event) => Some(*event),
                _ => None,
            })
            .collect()
    }
}

impl Host for Recorder {
    fn allocate(&mut self, _bytes: usize) -> bool {
        self.allocations.pop_front().unwrap_or(true)
    }

    fn rng_construct(&mut self, seed: u32) -> Result<(), Unsupported> {
        self.effects.push(Effect::Construct(seed));
        Ok(())
    }

    fn rng_draw(&mut self, bound: u32) -> Result<u32, Unsupported> {
        self.effects.push(Effect::Draw(bound));
        self.draws.pop_front().ok_or(Unsupported::RandomPrimitive)
    }

    fn trace(&mut self, event: Event) {
        self.effects.push(Effect::Owner(event));
    }

    fn callback(&mut self, identity: u64, x: u32, y: u32, context: Option<u64>, view: View<'_>) -> u32 {
        let c = view.current.unwrap();
        let p = view.alternate.unwrap();
        self.effects.push(Effect::Callback(identity, x, y));
        self.calls.push(Call {
            identity,
            x,
            y,
            context,
            c_id: c.allocation,
            p_id: p.allocation,
            c: c.bytes.to_vec(),
            p: p.bytes.to_vec(),
        });
        u32::MAX // deliberately ignored by production generation
    }

    fn view_clear(&mut self) {
        self.effects.push(Effect::ViewClear);
    }
}

fn bytes(sim: &GameOfLife) -> (Vec<u8>, Vec<u8>) {
    let view = sim.view();
    (
        view.current.unwrap().bytes.to_vec(),
        view.alternate.unwrap().bytes.to_vec(),
    )
}

fn roles(sim: &GameOfLife) -> (Allocation, Allocation) {
    let view = sim.view();
    (view.current.unwrap().allocation, view.alternate.unwrap().allocation)
}

fn seed(width: u32, height: u32, edits: &[(u32, u32)], host: &mut Recorder) -> GameOfLife {
    let mut sim = GameOfLife::new();
    sim.init(width, height, 0, host).unwrap();
    sim.wire(Controls::default(), CALLBACKS);
    for &(x, y) in edits {
        assert!(sim.edit(x, y, host).unwrap());
    }
    host.reset();
    sim
}

fn v7(host: &mut Recorder) -> GameOfLife {
    seed(4, 4, &[(0, 0), (1, 0), (2, 0)], host)
}

#[test]
fn v1_defaults_independent_and_allocation_clear_order() {
    for (width, height, expected_w, expected_h, length) in [(0, 1, 16, 1, 16), (1, 0, 1, 16, 16), (0, 0, 16, 16, 256)] {
        let mut sim = GameOfLife::new();
        assert_eq!(sim.view().controls, Controls::default());
        assert_eq!(sim.view().callbacks, Callbacks::default());
        assert!(sim.view().current.is_none());
        assert!(sim.view().alternate.is_none());
        assert_eq!((sim.view().width, sim.view().height), (0, 0));
        let mut host = Recorder::default();
        sim.init(width, height, 0x100, &mut host).unwrap(); // low byte only
        let events = host.owner_events();
        assert_eq!(
            &events[..7],
            &[
                Event::Free(Role::Current, None),
                Event::Null(Role::Current),
                Event::Free(Role::Alternate, None),
                Event::Null(Role::Alternate),
                Event::Dimensions(expected_w, expected_h),
                Event::Allocate(Role::Alternate, Some(Allocation(1)), length),
                Event::Allocate(Role::Current, Some(Allocation(2)), length),
            ]
        );
        let expected_stores: Vec<_> = [Allocation(1), Allocation(2)]
            .into_iter()
            .flat_map(|id| (0..length).map(move |i| Event::Store(id, i, 0)))
            .collect();
        assert_eq!(&events[7..], expected_stores);
        assert_eq!(bytes(&sim), (vec![0; length], vec![0; length]));
        assert_eq!((sim.view().width, sim.view().height), (expected_w, expected_h));
        assert!(host.calls.is_empty());
    }
}

#[test]
fn v2_fresh_raw_rng_rectangular_draw_and_store_order() {
    let expected_first = vec![1, 0, 1, 0, 0, 1];
    let expected_second = vec![0, 1, 0, 1, 1, 0];
    let mut host = Recorder {
        draws: [1, 0, 1, 0, 0, 1].into(),
        ..Recorder::default()
    };
    let mut sim = GameOfLife::new();
    sim.init(3, 2, 0x101, &mut host).unwrap();
    assert_eq!(bytes(&sim), (expected_first.clone(), expected_first.clone()));
    let mut expected_effects = vec![Effect::Construct(u32::MAX)];
    for (index, value) in expected_first.into_iter().enumerate() {
        expected_effects.extend([
            Effect::Draw(2),
            Effect::Owner(Event::Store(Allocation(1), index, value)),
            Effect::Owner(Event::Store(Allocation(2), index, value)),
        ]);
    }
    assert_eq!(&host.effects[7..], expected_effects);
    host.reset();
    host.draws = [0, 1, 0, 1, 1, 0].into();
    sim.randomize(&mut host).unwrap();
    assert_eq!(bytes(&sim), (expected_second.clone(), expected_second));
    assert_eq!(host.effects[0], Effect::Construct(u32::MAX));
    assert_eq!(host.effects.iter().filter(|e| **e == Effect::Draw(2)).count(), 6);
    assert!(host.calls.is_empty());
}

#[test]
fn v3_reinit_frees_current_role_after_swap_preserves_wiring() {
    let mut host = Recorder::default();
    let mut sim = v7(&mut host);
    sim.advance(&mut host).unwrap();
    sim.edit(3, 2, &mut host).unwrap();
    let controls = Controls {
        suppress: 7,
        edit_mode: u32::MAX,
    };
    sim.wire(controls, CALLBACKS);
    host.reset();
    sim.init(3, 2, 0, &mut host).unwrap();
    assert_eq!(
        &host.owner_events()[..7],
        &[
            Event::Free(Role::Current, Some(Allocation(1))),
            Event::Null(Role::Current),
            Event::Free(Role::Alternate, Some(Allocation(2))),
            Event::Null(Role::Alternate),
            Event::Dimensions(3, 2),
            Event::Allocate(Role::Alternate, Some(Allocation(3)), 6),
            Event::Allocate(Role::Current, Some(Allocation(4)), 6),
        ]
    );
    assert_eq!(bytes(&sim), (vec![0; 6], vec![0; 6]));
    assert_eq!(sim.view().controls, controls);
    assert_eq!(sim.view().callbacks, CALLBACKS);
    assert!(host.calls.is_empty());
}

#[test]
fn v4_partial_allocation_prefix_is_not_native_success() {
    let mut host = Recorder {
        allocations: [false, true].into(),
        ..Recorder::default()
    };
    let mut sim = GameOfLife::new();
    assert_eq!(sim.init(2, 1, 0, &mut host), Err(Unsupported::MissingBuffer));
    assert!(sim.view().alternate.is_none());
    assert_eq!(sim.view().current.unwrap().bytes, &[0, 0]);
    assert_eq!(
        &host.owner_events()[5..],
        &[
            Event::Allocate(Role::Alternate, None, 2),
            Event::Allocate(Role::Current, Some(Allocation(1)), 2),
            Event::Store(Allocation(1), 0, 0),
            Event::Store(Allocation(1), 1, 0),
        ]
    );
    host.reset();
    assert_eq!(sim.advance(&mut host), Err(Unsupported::MissingBuffer));
    assert_eq!(sim.randomize(&mut host), Err(Unsupported::MissingBuffer));
    assert!(host.effects.is_empty());
}

#[test]
fn v5_add_false_can_mutate_current_and_current_readout() {
    let mut host = Recorder::default();
    let mut sim = GameOfLife::from_stage(1, 1, vec![9], vec![0]).unwrap();
    sim.wire(Controls::default(), CALLBACKS);
    assert!(sim.edit(0, 0, &mut host).unwrap());
    assert_eq!(bytes(&sim), (vec![1], vec![2]));
    assert!(!sim.edit(0, 0, &mut host).unwrap());
    assert_eq!(bytes(&sim), (vec![1], vec![2]));
    let mut sim = GameOfLife::from_stage(1, 1, vec![9], vec![7]).unwrap();
    assert!(!sim.edit(0, 0, &mut host).unwrap());
    assert_eq!(bytes(&sim), (vec![1], vec![7]));
    assert!(sim.current_nonzero(0, 0).unwrap());
    assert!(host.calls.is_empty());
}

#[test]
fn v6_erase_false_mutation_and_unknown_mode_no_write() {
    let mut host = Recorder::default();
    for (c, p, mode, al, expected_c, expected_p) in [
        (9, 0, 1, false, 0, 0),
        (0, 7, 1, true, 0, 0),
        (9, 7, 2, false, 9, 7),
        (9, 7, u32::MAX, false, 9, 7),
    ] {
        let mut sim = GameOfLife::from_stage(1, 1, vec![c], vec![p]).unwrap();
        sim.wire(
            Controls {
                suppress: 0,
                edit_mode: mode,
            },
            CALLBACKS,
        );
        host.reset();
        assert_eq!(sim.edit(0, 0, &mut host).unwrap(), al);
        assert_eq!(bytes(&sim), (vec![expected_c], vec![expected_p]));
        if mode > 1 {
            assert!(host.effects.is_empty());
        }
        assert!(host.calls.is_empty());
    }
    let sim = GameOfLife::from_stage(1, 1, vec![0], vec![7]).unwrap();
    assert!(!sim.current_nonzero(0, 0).unwrap());
}

#[test]
fn v7_two_advances_exact_both_byte_buffers_and_roles() {
    let first_c = vec![0, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0];
    let first_p = vec![1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let second_c = vec![1, 3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let second_p = vec![0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0];
    let mut host = Recorder::default();
    let mut sim = v7(&mut host);
    assert_eq!(roles(&sim), (Allocation(2), Allocation(1)));
    sim.advance(&mut host).unwrap();
    assert_eq!(bytes(&sim), (first_c, first_p));
    assert_eq!(roles(&sim), (Allocation(1), Allocation(2)));
    sim.advance(&mut host).unwrap();
    assert_eq!(bytes(&sim), (second_c, second_p));
    assert_eq!(roles(&sim), (Allocation(2), Allocation(1)));
}

#[test]
fn v8_one_cell_counts_eight_samples_not_distinct_cells() {
    let mut host = Recorder::default();
    let mut sim = seed(1, 1, &[(0, 0)], &mut host);
    sim.advance(&mut host).unwrap();
    assert_eq!(bytes(&sim), (vec![0], vec![1]));
    sim.advance(&mut host).unwrap();
    assert_eq!(bytes(&sim), (vec![0], vec![0]));
    sim.advance(&mut host).unwrap();
    assert_eq!(bytes(&sim), (vec![0], vec![0]));
}

#[test]
fn v9_two_by_one_duplicate_directions_age_both_buffers() {
    let mut host = Recorder::default();
    let mut sim = seed(2, 1, &[(0, 0)], &mut host);
    sim.advance(&mut host).unwrap();
    assert_eq!(bytes(&sim), (vec![2, 0], vec![2, 0]));
    assert_eq!(host.calls.iter().map(|c| c.identity).collect::<Vec<_>>(), vec![L, D]);
}

#[test]
fn v10_conditional_per_visit_transition_stages() {
    // Center (1,1) is dead before its visit except supplied s. The top row
    // contributes exactly 2 or 3 nonzero neighbors, untouched by earlier births.
    // Earlier survivors can age, but cannot erase occupancy in these fixtures.
    let mut host = Recorder::default();
    for n in [2, 3] {
        for (s, expected) in [(1, 2), (190, 191), (191, 191), (192, 193), (254, 255), (255, 0)] {
            let c = if n == 2 {
                vec![1, 1, 0, 0, s, 0, 0, 0, 0]
            } else {
                vec![1, 1, 1, 0, s, 0, 0, 0, 0]
            };
            let mut sim = GameOfLife::from_stage(3, 3, c, vec![0; 9]).unwrap();
            sim.wire(Controls::default(), CALLBACKS);
            host.reset();
            sim.advance(&mut host).unwrap();
            let call = &host.calls[4];
            assert_eq!(
                (call.x, call.y, call.identity, call.c[4], call.p[4]),
                (1, 1, L, expected, expected)
            );
        }
    }
    for (c, identity, destination, retained) in [
        (vec![1, 1, 0, 0, 0, 0, 0, 0, 0], D, 0, 0),
        (vec![1, 1, 1, 0, 0, 0, 0, 0, 0], L, 1, 0),
        (vec![0, 0, 0, 0, 7, 0, 0, 0, 0], D, 0, 7),
        (vec![1, 0, 0, 0, 7, 0, 0, 0, 0], D, 0, 7),
        (vec![1, 1, 1, 1, 7, 0, 0, 0, 0], D, 0, 7),
        (vec![1, 1, 1, 1, 7, 1, 1, 1, 1], D, 0, 7),
    ] {
        let mut sim = GameOfLife::from_stage(3, 3, c, vec![0; 9]).unwrap();
        sim.wire(Controls::default(), CALLBACKS);
        host.reset();
        sim.advance(&mut host).unwrap();
        let call = &host.calls[4];
        assert_eq!((call.identity, call.c[4], call.p[4]), (identity, destination, retained));
    }
}

#[test]
fn v11_corner_and_interior_source_read_order() {
    let corner = vec![(4, 0), (12, 0), (5, 0), (13, 0), (7, 0), (15, 0), (1, 1), (3, 0)];
    let interior = vec![(8, 0), (10, 0), (0, 1), (2, 1), (4, 0), (6, 0), (1, 2), (9, 0)];
    let mut host = Recorder::default();
    let mut sim = v7(&mut host);
    sim.advance(&mut host).unwrap();
    let reads: Vec<_> = host
        .owner_events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Read(id, i, v) => {
                assert_eq!(id, Allocation(2));
                Some((i, v))
            }
            _ => None,
        })
        .collect();
    assert_eq!(&reads[..8], corner);
    assert_eq!(&reads[5 * 9..5 * 9 + 8], interior);
}

#[test]
fn v12_sixteen_callbacks_after_stores_observe_partial_publication() {
    let selections = [D, L, D, D, D, L, D, D, D, D, D, D, D, L, D, D];
    let coordinates = [
        (0, 0),
        (1, 0),
        (2, 0),
        (3, 0),
        (0, 1),
        (1, 1),
        (2, 1),
        (3, 1),
        (0, 2),
        (1, 2),
        (2, 2),
        (3, 2),
        (0, 3),
        (1, 3),
        (2, 3),
        (3, 3),
    ];
    let first_c = vec![0, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let first_p = vec![1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let second_c = vec![0, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let second_p = vec![1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut host = Recorder::default();
    let mut sim = v7(&mut host);
    sim.advance(&mut host).unwrap();
    assert_eq!(host.calls.len(), 16);
    for ((call, identity), (x, y)) in host.calls.iter().zip(selections).zip(coordinates) {
        assert_eq!((call.identity, call.x, call.y, call.context), (identity, x, y, Some(K)));
        assert_eq!((call.c_id, call.p_id), (Allocation(1), Allocation(2)));
    }
    assert_eq!((&host.calls[0].c, &host.calls[0].p), (&first_c, &first_p));
    assert_eq!((&host.calls[1].c, &host.calls[1].p), (&second_c, &second_p));
    for (index, effect) in host.effects.iter().enumerate() {
        if let Effect::Callback(_, x, y) = effect {
            let cell = (*y * 4 + *x) as usize;
            assert!(matches!(host.effects[index - 1], Effect::Owner(Event::Store(_, i, _)) if i == cell));
        }
    }
    let mut sim = seed(2, 1, &[], &mut host);
    sim.advance(&mut host).unwrap();
    assert_eq!(
        host.calls
            .iter()
            .map(|c| (c.identity, c.x, c.y, c.context))
            .collect::<Vec<_>>(),
        vec![(D, 0, 0, Some(K)), (D, 1, 0, Some(K))]
    );
    sim.wire(Controls::default(), Callbacks::default());
    host.reset();
    sim.advance(&mut host).unwrap();
    assert!(host.calls.is_empty());
}

#[test]
fn v13_suppress_direct_vs_parent_tick_clear_and_unused_time() {
    let mut host = Recorder::default();
    let mut sim = v7(&mut host);
    sim.wire(
        Controls {
            suppress: 255,
            edit_mode: 0,
        },
        CALLBACKS,
    );
    let before = bytes(&sim);
    let before_roles = roles(&sim);
    sim.advance(&mut host).unwrap();
    assert!(host.effects.is_empty());
    assert!(sim.tick_boundary(u32::MAX, &mut host).unwrap());
    assert_eq!(host.effects, vec![Effect::ViewClear]);
    assert_eq!(bytes(&sim), before);
    assert_eq!(roles(&sim), before_roles);
    sim.wire(Controls::default(), CALLBACKS);
    host.reset();
    assert!(sim.tick_boundary(u32::MAX, &mut host).unwrap());
    assert_eq!(&host.effects[..2], &[Effect::ViewClear, Effect::Owner(Event::Swap)]);
    assert_eq!(host.calls.len(), 16);
    assert_eq!(roles(&sim), (Allocation(1), Allocation(2)));
}

#[test]
fn v14_clear_p_then_c_preserves_roles_and_controls() {
    let mut host = Recorder::default();
    let mut sim = v7(&mut host);
    sim.advance(&mut host).unwrap();
    sim.edit(3, 2, &mut host).unwrap();
    let controls = Controls {
        suppress: 4,
        edit_mode: 1,
    };
    sim.wire(controls, CALLBACKS);
    let mut expected: Vec<Effect> = [Allocation(2), Allocation(1)]
        .into_iter()
        .flat_map(|id| (0..16).map(move |i| Effect::Owner(Event::Store(id, i, 0))))
        .collect();
    expected.push(Effect::ViewClear);
    for _ in 0..2 {
        host.reset();
        assert!(sim.clear_key_boundary(&mut host));
        assert_eq!(host.effects, expected);
        assert_eq!(bytes(&sim), (vec![0; 16], vec![0; 16]));
        assert_eq!(roles(&sim), (Allocation(1), Allocation(2)));
        assert_eq!(sim.view().controls, controls);
        assert_eq!(sim.view().callbacks, CALLBACKS);
    }
}

#[test]
fn v15_shutdown_vs_destruction_protocol_not_implicit_drop() {
    let mut host = Recorder::default();
    let mut sim = seed(2, 1, &[(0, 0)], &mut host);
    let controls = Controls {
        suppress: 1,
        edit_mode: 9,
    };
    sim.wire(controls, CALLBACKS);
    sim.shutdown_boundary(false, &mut host);
    assert!(host.effects.is_empty());
    sim.shutdown_boundary(true, &mut host);
    assert_eq!(
        host.owner_events(),
        vec![
            Event::ClearLive,
            Event::ClearDead,
            Event::ClearContext,
            Event::Free(Role::Current, Some(Allocation(2))),
            Event::Null(Role::Current),
            Event::Free(Role::Alternate, Some(Allocation(1))),
            Event::Null(Role::Alternate),
        ]
    );
    assert_eq!(sim.view().callbacks, Callbacks::default());
    assert_eq!(sim.view().controls, controls);
    assert_eq!((sim.view().width, sim.view().height), (2, 1));
    host.reset();
    sim.destruction_boundary(&mut host); // null ledger only; not foreign safety
    assert_eq!(
        host.owner_events(),
        vec![
            Event::Free(Role::Current, None),
            Event::Null(Role::Current),
            Event::Free(Role::Alternate, None),
            Event::Null(Role::Alternate)
        ]
    );
    let mut sim = seed(2, 1, &[], &mut host);
    sim.destruction_boundary(&mut host);
    assert_eq!(sim.view().callbacks, CALLBACKS); // destruction does not null callbacks
    host.reset();
    drop(sim);
    assert!(host.effects.is_empty()); // Rust memory ownership is separate
}

#[test]
fn v16_reachable_block_ages_saturate_at_191_through_200_advances() {
    let positions = [6, 7, 11, 12];
    let mut expected = vec![0; 20];
    let mut host = Recorder::default();
    let mut sim = seed(5, 4, &[(1, 1), (2, 1), (1, 2), (2, 2)], &mut host);
    for (until, age) in [(1, 2), (2, 3), (189, 190), (190, 191), (191, 191), (200, 191)] {
        // Fixed milestones, not a second transition oracle.
        let previous = match until {
            1 => 0,
            2 => 1,
            189 => 2,
            190 => 189,
            191 => 190,
            200 => 191,
            _ => unreachable!(),
        };
        for _ in previous..until {
            sim.advance(&mut host).unwrap();
        }
        for index in positions {
            expected[index] = age;
        }
        assert_eq!(bytes(&sim), (expected.clone(), expected.clone()));
        let tail = &host.calls[host.calls.len() - 20..];
        assert_eq!(tail.iter().filter(|c| c.identity == L).count(), 4);
        for index in positions {
            assert_eq!(tail[index].identity, L);
        }
        host.reset();
    }
}

#[test]
fn v17_sparse_and_crowded_owner_reachable_states() {
    let mut host = Recorder::default();
    for edits in [&[(2, 1)][..], &[(2, 1), (3, 1)][..]] {
        let mut expected_p = vec![0; 20];
        expected_p[7] = 1;
        if edits.len() == 2 {
            expected_p[8] = 1;
        }
        let mut sim = seed(5, 4, edits, &mut host);
        sim.advance(&mut host).unwrap();
        assert_eq!(bytes(&sim), (vec![0; 20], expected_p));
        assert_eq!(host.calls.len(), 20);
        assert!(host.calls.iter().all(|c| c.identity == D));
    }
    let mut sim = seed(
        3,
        3,
        &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1), (0, 2), (1, 2), (2, 2)],
        &mut host,
    );
    sim.advance(&mut host).unwrap();
    assert_eq!(bytes(&sim), (vec![0; 9], vec![1; 9]));
    assert_eq!(host.calls.len(), 9);
    assert!(host.calls.iter().all(|c| c.identity == D));
}

#[test]
fn zero_height_conditional_boundary_swaps_and_constructs_rng_without_draws() {
    let mut sim = GameOfLife::from_stage(3, 0, vec![], vec![]).unwrap();
    let mut host = Recorder::default();
    sim.advance(&mut host).unwrap();
    assert_eq!(roles(&sim), (Allocation(2), Allocation(1)));
    assert_eq!(host.effects, vec![Effect::Owner(Event::Swap)]);
    host.reset();
    sim.randomize(&mut host).unwrap();
    assert_eq!(host.effects, vec![Effect::Construct(u32::MAX)]);
}

#[test]
fn portable_refusals_and_raw_draw_low_byte_are_explicit() {
    let mut host = Recorder::default();
    let mut sim = GameOfLife::new();
    assert_eq!(sim.init(u32::MAX, 2, 0, &mut host), Err(Unsupported::Arithmetic));
    assert!(sim.view().current.is_none());
    assert_eq!(sim.init(1 << 16, 1 << 16, 0, &mut host), Err(Unsupported::Arithmetic));
    sim.init(1, 1, 0, &mut host).unwrap();
    assert_eq!(sim.edit(1, 0, &mut host), Err(Unsupported::Coordinates));
    assert_eq!(sim.current_nonzero(0, 1), Err(Unsupported::Coordinates));
    host.draws = [0x1234_ff80].into();
    sim.randomize(&mut host).unwrap();
    assert_eq!(bytes(&sim), (vec![128], vec![128]));
    assert_eq!(sim.randomize(&mut host), Err(Unsupported::RandomPrimitive));
    assert!(matches!(
        GameOfLife::from_stage(2, 1, vec![1], vec![0; 2]),
        Err(Unsupported::StageLength)
    ));
}

#[test]
fn conditional_255_source_write_changes_later_neighbor_read() {
    // Conditional high-byte source, not ordinary owner reachability. First cell
    // sees two live neighbors, wraps to zero, then the next cell reads that zero.
    let mut sim = GameOfLife::from_stage(
        4,
        4,
        vec![255, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![0; 16],
    )
    .unwrap();
    sim.wire(Controls::default(), CALLBACKS);
    let mut host = Recorder::default();
    sim.advance(&mut host).unwrap();
    assert_eq!(
        (host.calls[0].identity, host.calls[0].c[0], host.calls[0].p[0]),
        (L, 0, 0)
    );
    assert_eq!((host.calls[1].identity, host.calls[1].c[1]), (D, 0));
    let reads: Vec<_> = host
        .owner_events()
        .into_iter()
        .filter_map(|e| match e {
            Event::Read(_, i, v) => Some((i, v)),
            _ => None,
        })
        .collect();
    assert_eq!(reads[9 + 7], (0, 0)); // W sample of second cell sees mutated source
}
