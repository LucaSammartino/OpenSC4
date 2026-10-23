// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent expected values: animation_tick_manager_vectors.md, not captured outputs.
use fc4_core::animation_tick_manager::*;

const A: Target = Target(4);
const B: Target = Target(8);
const C: Target = Target(12);
type Callback = fn(&mut Recording, Target, &mut Manager);

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Effect(Effect),
    Read(Timer),
    Allocate(u32),
    Delivery(Target, u32, u32, u32, u32),
}

struct Recording {
    events: Vec<Event>,
    main: u32,
    auto: u32,
    framework: bool,
    allocation: bool,
    callback: Option<Callback>,
}

impl Default for Recording {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            main: 20000,
            auto: 0,
            framework: true,
            allocation: true,
            callback: None,
        }
    }
}

impl Host for Recording {
    fn effect(&mut self, effect: Effect) {
        if effect == Effect::Reset(Timer::Auto) {
            self.auto = 0;
        }
        self.events.push(Event::Effect(effect));
    }
    fn elapsed(&mut self, timer: Timer) -> u32 {
        self.events.push(Event::Read(timer));
        match timer {
            Timer::Main => self.main,
            Timer::Auto => self.auto,
        }
    }
    fn framework_present(&mut self) -> bool {
        self.framework
    }
    fn allocate_message(&mut self, bytes: u32) -> bool {
        self.events.push(Event::Allocate(bytes));
        self.allocation
    }
    fn do_message(&mut self, target: Target, manager: &mut Manager) -> bool {
        assert!(manager.in_tick());
        let message = manager.message().unwrap();
        self.events.push(Event::Delivery(
            target,
            message.kind,
            message.data1,
            message.data2,
            message.data3,
        ));
        if let Some(callback) = self.callback.take() {
            callback(self, target, manager);
        }
        false // Must be ignored, including before another bucket.
    }
}

impl Recording {
    fn effects(&self) -> Vec<Effect> {
        self.events
            .iter()
            .filter_map(|e| match e {
                Event::Effect(e) => Some(*e),
                _ => None,
            })
            .collect()
    }
    fn targets(&self) -> Vec<Target> {
        self.events
            .iter()
            .filter_map(|e| match e {
                Event::Delivery(t, ..) => Some(*t),
                _ => None,
            })
            .collect()
    }
    fn deltas(&self) -> Vec<u32> {
        self.events
            .iter()
            .filter_map(|e| match e {
                Event::Delivery(_, _, _, d, _) => Some(*d),
                _ => None,
            })
            .collect()
    }
}

fn ready() -> (Manager, Recording) {
    let mut host = Recording::default();
    let mut manager = Manager::new(0x1000, &mut host);
    manager.add_ref(); // External owner, independent of the message cycle.
    assert_eq!(manager.init(&mut host), Ok(true));
    host.events.clear();
    (manager, host)
}

#[test]
fn v1_eight_phase_band_cycle_and_empty_timestamps() {
    let (mut m, mut h) = ready();
    for (target, priority) in [(A, 192), (B, 128), (C, 64), (Target(16), 0)] {
        m.add(&mut h, target, priority);
    }
    let expected = [
        vec![A, C],
        vec![A, B],
        vec![A, Target(16)],
        vec![A, B],
        vec![A, C],
        vec![A, B],
        vec![A],
        vec![A, B],
    ];
    let mut counts = [0; 4];
    for (index, targets) in expected.into_iter().enumerate() {
        h.events.clear();
        h.main = 20000 + index as u32 * 10000;
        m.tick(&mut h);
        assert_eq!(h.targets(), targets);
        assert_eq!(m.frame(), index as u32 + 1);
        for t in h.targets() {
            counts[(t.0 / 4 - 1) as usize] += 1;
        }
    }
    assert_eq!(counts, [8, 4, 2, 1]);
    let (mut empty, mut h) = ready();
    assert_eq!(empty.tick(&mut h), 2000);
    assert_eq!(empty.bucket_timestamps(), [20000, 0, 20000, 0]);
    h.main = 30000;
    empty.tick(&mut h);
    assert_eq!(empty.bucket_timestamps(), [30000, 30000, 20000, 0]);
}

#[test]
fn v2_v3_edges_duplicates_and_exact_pointer_identity() {
    let (mut m, mut h) = ready();
    for (p, b) in [
        (0, 3),
        (63, 3),
        (64, 2),
        (127, 2),
        (128, 1),
        (191, 1),
        (192, 0),
        (u32::MAX, 0),
    ] {
        assert_eq!(priority_bucket(p), b);
    }
    assert!(m.add(&mut h, A, 192));
    assert!(m.add(&mut h, A, ALL));
    m.add(&mut h, A, 191);
    assert_eq!(h.effects(), vec![Effect::TargetAddRef(A), Effect::TargetAddRef(A)]);
    assert!(m.find(A, ALL));
    assert!(!m.find(Target(5), ALL)); // Even another interface-looking pointer is distinct.
    assert!(!m.find(A, 64));
    m.tick(&mut h);
    h.events.clear();
    m.tick(&mut h);
    assert_eq!(h.targets(), vec![A, A]);
    assert!(h.effects().is_empty()); // No per-delivery AddRef/Release.
}

#[test]
fn v4_v5_remove_one_fallback_and_priority_reference_trace() {
    let (mut m, mut h) = ready();
    for p in [192, 128, 0] {
        m.add(&mut h, A, p);
    }
    h.events.clear();
    assert!(m.remove(&mut h, A, ALL));
    assert!(!m.find(A, 0));
    m.remove(&mut h, A, 64);
    assert!(!m.find(A, 128));
    assert!(m.find(A, 192));
    assert_eq!(h.effects(), vec![Effect::TargetRelease(A), Effect::TargetRelease(A)]);
    h.events.clear();
    m.set_new_priority(&mut h, A, 200);
    assert_eq!(
        h.effects(),
        vec![
            Effect::TargetAddRef(A),
            Effect::TargetRelease(A),
            Effect::TargetAddRef(A),
            Effect::TargetRelease(A)
        ]
    );
    h.events.clear();
    m.set_new_priority(&mut h, B, 64);
    assert_eq!(
        h.effects(),
        vec![
            Effect::TargetAddRef(B),
            Effect::TargetAddRef(B),
            Effect::TargetRelease(B)
        ]
    );
    assert!(m.find(B, 64));
    m.remove(&mut h, A, ALL);
    h.events.clear();
    assert!(m.remove(&mut h, A, ALL));
    assert!(h.effects().is_empty());
}

#[test]
fn v6_self_removal_recursive_tick_does_not_read_or_drain() {
    let (mut m, mut h) = ready();
    m.add(&mut h, A, 192);
    m.add(&mut h, A, 64);
    h.events.clear();
    h.callback = Some(|h, target, m| {
        assert_eq!(target, A);
        m.remove(h, A, 192);
        assert!(m.find(A, 64));
        let frame = m.frame();
        let events = h.events.len();
        assert_eq!(m.tick(h), 0);
        assert_eq!(h.events.len(), events);
        assert_eq!(m.frame(), frame);
        assert!(m.find(A, 64));
    });
    m.tick(&mut h);
    assert_eq!(h.targets(), vec![A]);
    assert!(m.find(A, 192));
    assert!(!m.find(A, 64));
    assert_eq!(
        h.effects(),
        vec![
            Effect::TargetAddRef(A),
            Effect::TargetRelease(A),
            Effect::TargetRelease(A)
        ]
    );
    assert!(!m.in_tick());
}

#[test]
fn v7_fifo_both_orders_and_new_target_not_delivered_now() {
    for remove_first in [true, false] {
        let (mut m, mut h) = ready();
        m.add(&mut h, A, 192);
        h.callback = Some(if remove_first {
            |h, _, m| {
                m.remove(h, B, 0);
                m.add(h, B, 64);
                assert!(!m.find(B, ALL));
            }
        } else {
            |h, _, m| {
                m.add(h, B, 64);
                m.remove(h, B, 192);
                assert!(!m.find(B, ALL));
            }
        });
        h.events.clear();
        m.tick(&mut h);
        assert_eq!(h.targets(), vec![A]);
        assert_eq!(m.find(B, ALL), remove_first);
        let expected = if remove_first {
            vec![
                Effect::TargetAddRef(B),
                Effect::TargetAddRef(B),
                Effect::TargetRelease(B),
                Effect::TargetAddRef(B),
                Effect::TargetRelease(B),
            ]
        } else {
            vec![
                Effect::TargetAddRef(B),
                Effect::TargetAddRef(B),
                Effect::TargetAddRef(B),
                Effect::TargetRelease(B),
                Effect::TargetRelease(B),
                Effect::TargetRelease(B),
            ]
        };
        assert_eq!(h.effects(), expected);
    }
}

#[test]
fn v8_duplicate_queued_adds_suppress_and_retain_every_request() {
    let (mut m, mut h) = ready();
    m.add(&mut h, A, 192);
    m.add(&mut h, A, 128);
    m.tick(&mut h); // phase2 next
    h.events.clear();
    h.callback = Some(|h, _, m| {
        m.add(h, A, 192);
        m.add(h, A, 192);
        assert!(m.find(A, 128));
    });
    m.tick(&mut h);
    assert_eq!(h.targets(), vec![A]);
    assert_eq!(
        h.effects(),
        vec![
            Effect::TargetAddRef(A),
            Effect::TargetAddRef(A),
            Effect::TargetRelease(A),
            Effect::TargetRelease(A)
        ]
    );
    // Suppression clears only after the drain, so a subsequent tick delivers again.
    h.events.clear();
    m.tick(&mut h);
    assert_eq!(h.targets(), vec![A]);
}

#[test]
fn v9_v10_raw_timestamps_clamp_wrap_and_backward() {
    for (last, now, delta) in [
        (10000, 11999, 2000),
        (10000, 12000, 2000),
        (10000, 12001, 2001),
        (10000, 509999, 499999),
        (10000, 510000, 500000),
        (10000, 510001, 500000),
        (0xFFFF_FFF0, 0x20, 2000),
        (10000, 9999, 500000),
    ] {
        let (mut m, mut h) = ready();
        m.add(&mut h, A, 192);
        h.main = last;
        m.tick(&mut h);
        h.events.clear();
        h.main = now;
        assert_eq!(m.tick(&mut h), delta);
        assert_eq!(h.deltas(), vec![delta]);
        assert_eq!(m.timestamp(), now);
        assert_eq!(m.bucket_timestamps()[0], now);
    }
}

#[test]
fn v11_bounds_equality_and_unsigned_inputs() {
    let (mut m, mut h) = ready();
    m.add(&mut h, A, 192);
    for (min, max, expected) in [(0, 0, 1), (7, 3, 7), (9, 9, 9), (0x8000_0000, 1, 0x8000_0000)] {
        m.set_bounds(min, max);
        assert_eq!(m.bounds(), (expected, expected));
        h.events.clear();
        assert_eq!(m.tick(&mut h), expected);
        assert_eq!(h.deltas(), vec![expected]);
    }
}

#[test]
fn v12_step_delta_edges_save_unclamped_timestamp() {
    for (step, expected) in [
        (0, 2000),
        (1999, 2000),
        (2000, 2000),
        (500000, 500000),
        (500001, 500000),
        (u32::MAX, 500000),
    ] {
        let (mut m, mut h) = ready();
        m.add(&mut h, A, 192);
        h.main = 10000;
        m.tick(&mut h);
        m.step(&mut h);
        m.step(&mut h);
        m.set_step_delta(step);
        h.events.clear();
        h.main = 123;
        assert_eq!(m.tick(&mut h), expected);
        assert_eq!(h.deltas(), vec![expected]);
        assert_eq!(m.timestamp(), 10000u32.wrapping_add(step));
        assert!(!m.pending_step());
    }
}

fn prime_timestamps(m: &mut Manager, h: &mut Recording, last0: u32, last2: u32) {
    for frame in 1..=8 {
        h.main = if frame <= 5 { last2 } else { last0 };
        m.tick(h);
    }
    h.events.clear();
}

#[test]
fn v13_shared_step_timestamp_and_empty_bucket_copy() {
    for bucket2_nonempty in [true, false] {
        let (mut m, mut h) = ready();
        m.add(&mut h, A, 192);
        if bucket2_nonempty {
            m.add(&mut h, B, 64);
        }
        prime_timestamps(&mut m, &mut h, 10000, 20000);
        m.step(&mut h);
        m.step(&mut h);
        h.main = 999;
        assert_eq!(m.tick(&mut h), 100000);
        let last2 = if bucket2_nonempty { 120000 } else { 110000 };
        assert_eq!(m.timestamp(), last2);
        assert_eq!(m.bucket_timestamps()[0], 110000);
        assert_eq!(m.bucket_timestamps()[2], last2);
        assert_eq!(
            h.deltas(),
            if bucket2_nonempty {
                vec![100000, 100000]
            } else {
                vec![100000]
            }
        );
    }
}

#[test]
fn v14_v15_pause_first_step_pending_coalescing_and_underflow() {
    let (mut m, mut h) = ready();
    m.add(&mut h, A, 192);
    m.step(&mut h);
    assert!(!m.pending_step());
    assert!(m.is_paused());
    assert_eq!(m.tick(&mut h), 0);
    m.step(&mut h);
    m.step(&mut h);
    assert_eq!(m.pause_depth_bits(), 1);
    assert!(m.tick(&mut h) > 0);
    assert!(!m.pending_step());
    assert_eq!(m.tick(&mut h), 0);
    assert_eq!(m.frame(), 3);
    assert_eq!(h.events.iter().filter(|e| **e == Event::Read(Timer::Main)).count(), 3);
    let (mut m, mut h) = ready();
    m.pause(&mut h, false);
    assert_eq!(m.pause_depth_bits(), ALL);
    assert!(!m.is_paused());
    assert_eq!(m.tick(&mut h), 0);
    m.step(&mut h);
    assert_eq!(m.pause_depth_bits(), 0);
    assert!(!m.pending_step());
}

#[test]
fn v16_auto_transition_restart_threshold_reset_and_zero_interval() {
    let (mut m, mut h) = ready();
    m.add(&mut h, A, 192);
    m.enable_auto_step(&mut h, true, 100);
    m.pause(&mut h, true);
    assert_eq!(
        h.effects()
            .iter()
            .filter(|&&e| e == Effect::Restart(Timer::Auto))
            .count(),
        2
    );
    h.events.clear();
    h.auto = 99;
    assert_eq!(m.tick(&mut h), 0);
    h.auto = 100;
    assert!(m.tick(&mut h) > 0);
    assert!(h.effects().contains(&Effect::Reset(Timer::Auto)));
    assert_eq!(h.auto, 0);
    assert_eq!(m.tick(&mut h), 0);
    h.events.clear();
    m.enable_auto_step(&mut h, true, 0);
    assert!(h.effects().is_empty());
    assert!(m.tick(&mut h) > 0);
    assert!(m.tick(&mut h) > 0);
    m.enable_auto_step(&mut h, false, 999);
    assert_eq!(h.effects().last(), Some(&Effect::Reset(Timer::Auto)));
    assert_eq!(m.tick(&mut h), 0);
    // No elapsed auto read while unpaused, even when enabled.
    m.pause(&mut h, false);
    m.enable_auto_step(&mut h, true, 0);
    h.events.clear();
    m.tick(&mut h);
    assert!(!h.events.contains(&Event::Read(Timer::Auto)));
}

#[test]
fn v17_first_above_minimum_return_and_callback_minimum_reload() {
    for (last0, last2, expected, deltas) in [
        (8000, 0, 10000, vec![2000, 10000]),
        (1000, 7000, 9000, vec![9000, 3000]),
    ] {
        let (mut m, mut h) = ready();
        m.add(&mut h, A, 192);
        m.add(&mut h, B, 64);
        prime_timestamps(&mut m, &mut h, last0, last2);
        h.main = 10000;
        assert_eq!(m.tick(&mut h), expected);
        assert_eq!(h.deltas(), deltas);
    }
    let (mut m, mut h) = ready();
    m.add(&mut h, A, 192);
    m.add(&mut h, B, 64);
    h.main = 3000;
    h.callback = Some(|_, _, m| m.set_bounds(9000, 10000));
    assert_eq!(m.tick(&mut h), 9000);
    assert_eq!(h.deltas(), vec![3000, 9000]);
}

#[test]
fn v18_qi_reference_results_sentinel_and_destructor_does_not_shutdown() {
    let mut h = Recording::default();
    let mut m = Manager::new(0x1234, &mut h);
    let sentinel = InterfaceIdentity {
        base: 99,
        interface: Interface::Service,
    };
    let mut out = sentinel;
    assert_eq!(m.reference_count(), 0);
    assert!(m.query_interface(INTERFACE_ID, &mut out));
    assert_eq!(out, m.primary_identity());
    assert!(m.query_interface(0x4522_94AA, &mut out));
    assert_eq!(out.interface, Interface::MessageTarget);
    assert_eq!(m.reference_count(), 2);
    out = sentinel;
    assert!(!m.query_interface(0xDEAD_BEEF, &mut out));
    assert_eq!(out, sentinel);
    assert_eq!(m.release(&mut h), Ok(1));
    assert_eq!(m.release(&mut h), Ok(0));
    let mut m = Manager::new(8, &mut h);
    m.add(&mut h, A, 192);
    h.events.clear();
    assert_eq!(m.release(&mut h), Ok(0));
    assert_eq!(h.effects(), vec![Effect::DestroyStorage]);
    assert_eq!(m.release(&mut h), Err(Unsupported::Destroyed));
}

#[test]
fn v19_orderly_shutdown_cycle_and_retained_state_no_seeded_pending_path() {
    let (mut m, mut h) = ready();
    assert_eq!(m.reference_count(), 2);
    m.add(&mut h, A, 192);
    m.add(&mut h, A, 128);
    m.set_bounds(7, 9);
    m.set_step_delta(3);
    m.enable_auto_step(&mut h, true, 10);
    m.tick(&mut h);
    let stamps = m.bucket_timestamps();
    let now = m.timestamp();
    h.events.clear();
    assert_eq!(m.shutdown(&mut h), Ok(true));
    assert_eq!(
        h.effects(),
        vec![
            Effect::UnknownRelease(m.primary_identity()),
            Effect::MessageRelease,
            Effect::FrameworkSlot(10),
            Effect::FrameworkSlot(4),
            Effect::TargetRelease(A),
            Effect::TargetRelease(A),
            Effect::Reset(Timer::Main)
        ]
    );
    assert_eq!(m.reference_count(), 1);
    assert!(!m.is_initialized());
    assert_eq!(m.bounds(), (7, 9));
    assert_eq!(m.frame(), 1);
    assert_eq!(m.timestamp(), now);
    assert_eq!(m.bucket_timestamps(), stamps);
    h.events.clear();
    assert_eq!(m.shutdown(&mut h), Ok(true));
    assert!(h.events.is_empty());
    m.init(&mut h).unwrap();
    assert_eq!(m.frame(), 1);
    assert_eq!(m.bounds(), (7, 9));
    h.events.clear();
    m.init(&mut h).unwrap();
    assert!(h.events.is_empty());
    // A pre-Init registration is not released by no-op Shutdown or destruction.
    let mut m = Manager::new(7, &mut h);
    m.add(&mut h, B, 128);
    h.events.clear();
    assert_eq!(m.shutdown(&mut h), Ok(true));
    assert!(m.find(B, 128));
    assert!(h.events.is_empty());
}

#[test]
fn v20_literal_float_bits_and_large_integer_load_schedule() {
    for (delta, bits) in [(2000, 0x4000_0000), (100000, 0x42C8_0001), (500000, 0x43FA_0001)] {
        assert_eq!(millisecond_bits(delta), bits);
        let (mut m, mut h) = ready();
        m.add(&mut h, A, 192);
        h.main = delta;
        m.tick(&mut h);
        assert!(h
            .events
            .contains(&Event::Delivery(A, MESSAGE_TYPE, 0x1000, delta, bits)));
    }
    // These check mathematical PC24 load/correction branches, not native execution.
    // 2^24+1 is loaded exactly before multiply, not rounded to 2^24 first.
    // Derived with integer significands and nearest-even, before production runs.
    for (delta, bits) in [
        (16_777_217, 0x4683_1270),
        (2_147_483_647, 0x4A03_126F),
        (2_147_483_648, 0x4A03_126F),
        (3_242_996_796, 0x4A45_EFD4),
        (ALL, 0x4A83_126F),
    ] {
        assert_eq!(millisecond_bits(delta), bits);
    }
}

#[test]
fn mutable_message_is_shared_next_bucket_rewrites_only_data2_data3() {
    let (mut m, mut h) = ready();
    m.add(&mut h, A, 192);
    m.add(&mut h, B, 192);
    m.add(&mut h, C, 64);
    h.callback = Some(|h, _, m| {
        let msg = m.message_mut().unwrap();
        msg.kind = 7;
        msg.data1 = 8;
        msg.data2 = 9;
        msg.data3 = 10;
        m.pause(h, true); // Does not retroactively skip this admitted sweep.
    });
    m.tick(&mut h);
    assert_eq!(h.targets(), vec![A, B, C]);
    assert!(h.events.contains(&Event::Delivery(B, 7, 8, 9, 10)));
    assert!(h
        .events
        .contains(&Event::Delivery(C, 7, 8, 20000, millisecond_bits(20000))));
    assert!(m.is_paused());
}

#[test]
fn synchronous_priority_time_step_message_and_unsupported_boundaries() {
    let (mut m, mut h) = ready();
    m.add(&mut h, A, 192);
    m.add(&mut h, B, 64);
    h.callback = Some(|h, _, m| {
        assert_eq!(m.timestamp(), 20000);
        m.set_new_priority(h, A, 128);
        assert!(m.find(A, 192));
        assert!(!m.find(A, 128));
        m.pause(h, true);
        m.set_step_delta(42);
        m.step(h);
        assert_eq!(m.shutdown(h), Err(Unsupported::DestructiveReentry));
        let identity = InterfaceIdentity {
            base: 0x2000,
            interface: Interface::Primary,
        };
        m.set_retained_unknown(h, Some(identity));
    });
    m.tick(&mut h);
    assert_eq!(h.deltas(), vec![20000, 2000]);
    assert_eq!(m.timestamp(), 42);
    assert!(!m.pending_step());
    assert!(m.find(A, 128));
    assert!(!m.find(A, 192));
    assert_eq!(m.read(), Err(Unsupported::PersistenceSchema));
    assert_eq!(m.write(), Err(Unsupported::PersistenceSchema));
    let frame = m.frame();
    assert!(m.receive_message(m.message().unwrap()));
    assert_eq!(m.frame(), frame);
}

#[test]
fn initial_hash_slot_order_and_collision_prepend_are_portable_only() {
    let (mut m, mut h) = ready();
    let collision = Target(4 + 4 * 193);
    m.add(&mut h, B, 192);
    m.add(&mut h, A, 192);
    m.add(&mut h, collision, 192);
    m.tick(&mut h);
    assert_eq!(h.targets(), vec![collision, A, B]);
    // Withheld: unknown native resize threshold/prime/transfer order (TK4).
}

#[test]
fn phase_zero_rate_is_separate_from_timestamp_and_updates_when_paused() {
    let (mut m, mut h) = ready();
    m.pause(&mut h, true);
    h.main = 10000;
    for _ in 0..7 {
        assert_eq!(m.tick(&mut h), 0);
        assert_eq!(m.smoothed_rate(), 0.0);
    }
    h.main = 19997; // Unsigned difference9997 +3 =10000; rate100/8=12.5.
    assert_eq!(m.tick(&mut h), 0);
    assert_eq!(m.smoothed_rate().to_bits(), 12.5f32.to_bits());
    assert_eq!(m.timestamp(), 19997);
    for _ in 0..7 {
        m.tick(&mut h);
    }
    h.main = 29994;
    m.tick(&mut h);
    assert_eq!(m.smoothed_rate().to_bits(), 23.4375f32.to_bits());
    assert_eq!(m.bucket_timestamps(), [0; 4]); // All sweeps skipped, clocks still sampled.

    let (mut m, mut h) = ready();
    h.main = 10000;
    for _ in 0..7 {
        m.tick(&mut h);
    }
    h.main = 9997; // Wrapping -3 +3 =0: deliberately no invented denominator guard.
    m.tick(&mut h);
    assert!(m.smoothed_rate().is_infinite());
}

#[test]
fn construction_init_raw_sequence_qi_auxiliaries_and_lifetime_boundary() {
    let mut h = Recording::default();
    let mut m = Manager::new(0x1000, &mut h);
    assert_eq!(
        h.effects(),
        vec![
            Effect::ConstructTimer(Timer::Main, 3),
            Effect::ConstructTimer(Timer::Auto, 3)
        ]
    );
    assert_eq!(m.bounds(), (2000, 500000));
    assert_eq!(m.bucket_timestamps(), [0; 4]);
    assert_eq!(m.reference_count(), 0);
    h.events.clear();
    m.add_ref();
    assert_eq!(m.init(&mut h), Ok(true));
    assert_eq!(
        h.events,
        vec![
            Event::Effect(Effect::Restart(Timer::Main)),
            Event::Effect(Effect::FrameworkSlot(3)),
            Event::Effect(Effect::FrameworkSlot(9)),
            Event::Allocate(44),
            Event::Effect(Effect::ConstructMessage),
            Event::Effect(Effect::MessageAddRef),
            Event::Effect(Effect::UnknownAddRef(m.primary_identity()))
        ]
    );
    assert_eq!(m.message().unwrap().retained_unknown(), Some(m.primary_identity()));
    for (iid, interface) in [
        (1, Interface::Primary),
        (0xE4FD_A3D4, Interface::Serializable),
        (0x287F_B697, Interface::Service),
    ] {
        let mut out = m.primary_identity();
        let old = m.reference_count();
        assert!(m.query_interface(iid, &mut out));
        assert_eq!(
            out,
            InterfaceIdentity {
                base: 0x1000,
                interface
            }
        );
        assert_eq!(m.reference_count(), old + 1);
        m.release(&mut h).unwrap();
    }
    // With only the message's reference, Shutdown would destroy its receiver.
    m.release(&mut h).unwrap();
    h.events.clear();
    assert_eq!(m.shutdown(&mut h), Err(Unsupported::ExternalOwnerRequired));
    assert!(h.events.is_empty());
    m.add_ref();
    m.shutdown(&mut h).unwrap();
    assert_eq!(m.reference_count(), 1);
}
