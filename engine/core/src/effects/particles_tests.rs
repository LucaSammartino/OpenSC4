// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Tests of the particle emitters (effects.md §7.1, §7.1.1, EF13).

use super::*;
use crate::rng::FixedSeeds;

fn rng() -> RzRandom {
    RzRandom::new(
        0x1234,
        &FixedSeeds {
            wall_clock: 0,
            process_clock: 7,
        },
    )
}

fn inject(count: f32, life: f32) -> EmitterDesc {
    EmitterDesc::new(emitter_flag::LOOP_BUDGET, count, life)
}

/// Steps until inactive; returns the update count (None: still active).
fn run(p: &mut ParticleSimulator, r: &mut RzRandom, dt: f32, max: usize) -> Option<usize> {
    for i in 1..=max {
        p.update(dt, r);
        if !p.is_active() {
            return Some(i);
        }
    }
    None
}

#[test]
fn emitter_choice_by_flags() {
    assert_eq!(EmitterKind::from_flags(0x04), EmitterKind::Plain);
    assert_eq!(EmitterKind::from_flags(0x06), EmitterKind::Plain);
    assert_eq!(EmitterKind::from_flags(0x02), EmitterKind::LoopBudget);
    assert_eq!(EmitterKind::from_flags(0x00), EmitterKind::Rate);
    assert_eq!(EmitterKind::from_flags(0x09), EmitterKind::Rate);
}

#[test]
fn start_seeds_the_countdown_from_the_start_delay_then_the_interval() {
    // EF13 (a): start-delay range if D+0x20 >= 0, else the interval range if
    // D+0x28 >= 0, else 0; drawn from the global generator.
    let mut d = inject(1.0, 1.0);
    d.start_delay = (0.5, 1.5);
    d.interval = (3.0, 4.0);
    let (mut r, mut want) = (rng(), rng());
    let mut p = ParticleSimulator::new(d.clone());
    p.start(0, &mut r);
    assert_eq!(p.countdown(), want.range_f64(0.5, 1.5) as f32);
    d.start_delay = (-1.0, 2.0);
    let mut p = ParticleSimulator::new(d.clone());
    p.start(0, &mut r);
    assert_eq!(p.countdown(), want.range_f64(3.0, 4.0) as f32);
    d.interval = (-0.5, 4.0);
    let mut p = ParticleSimulator::new(d);
    p.start(0, &mut r);
    assert_eq!(p.countdown(), 0.0);
    assert_eq!(r.lcg_state(), want.lcg_state());
    assert_eq!(r.mul_state(), want.mul_state());
}

#[test]
fn first_burst_comes_after_the_countdown_with_the_overshoot() {
    // w = 0.375: update 1 leaves 0.125; update 2 overshoots by 0.125, the
    // emit time is that overshoot, so the burst is 0.125 s old.
    let mut d = inject(2.0, 1.0);
    d.start_delay = (0.375, 0.375);
    let mut r = rng();
    let mut p = ParticleSimulator::new(d);
    p.start(0, &mut r);
    p.update(0.25, &mut r);
    assert_eq!((p.live_count(), p.countdown()), (0, 0.125));
    p.update(0.25, &mut r);
    assert_eq!(p.live(), &[0.875, 0.875]);
    assert_eq!(p.loops_emitted(), 1);
}

#[test]
fn both_ranges_negative_burst_on_the_first_update() {
    let mut r = rng();
    let mut p = ParticleSimulator::new(inject(1.0, 1.0));
    p.start(0, &mut r);
    p.update(0.25, &mut r);
    // Aged before emission: the new particle carries only its burst age.
    assert_eq!(p.live(), &[0.75]);
}

#[test]
fn burst_is_ceil_of_the_curve_scaled_above_20() {
    let mut r = rng();
    let count = |d: EmitterDesc, r: &mut RzRandom| {
        let mut p = ParticleSimulator::new(d);
        p.start(0, r);
        p.update(0.25, r);
        p.particles_created()
    };
    assert_eq!(count(inject(2.2, 1.0), &mut r), 3);
    assert_eq!(count(inject(20.0, 1.0), &mut r), 20);
    let mut d = inject(25.0, 1.0);
    d.scales = [0.5, 1.0, 1.0];
    assert_eq!(count(d.clone(), &mut r), 13);
    d.flags_b = 1;
    assert_eq!(count(d, &mut r), 25);
    let mut d = inject(10.0, 1.0);
    d.scales = [0.5, 0.5, 0.5];
    assert_eq!(count(d, &mut r), 10);
}

#[test]
fn loop_budget_stops_delayed_and_plays_out() {
    // T16: one loop of four (life 3 s at 0.5 s): Stop(0) at once, inactive
    // when the last particle dies (burst age 0.5, so 5 more updates).
    let mut r = rng();
    let mut p = ParticleSimulator::new(inject(4.0, 3.0));
    p.start(0, &mut r);
    p.update(0.5, &mut r);
    assert_eq!((p.loops_emitted(), p.live_count()), (1, 4));
    assert!(p.delayed_stop_requested() && p.is_active());
    assert_eq!(run(&mut p, &mut r, 0.5, 100), Some(5));
    assert_eq!(p.particles_created(), 4);
}

#[test]
fn loop_budget_interval_reseeds_the_countdown() {
    // Interval 2 s: after the first burst P := 0.5 − 2 < 1, so
    // w := (1 − (−1.5))·1 = 2.5 and P := 1.
    let mut d = inject(1.0, 0.75);
    d.loop_count = 0;
    // Start delay [0, 0] (so not the interval) seeds w = 0.
    d.start_delay = (0.0, 0.0);
    d.interval = (2.0, 2.0);
    let mut r = rng();
    let mut p = ParticleSimulator::new(d);
    p.start(0, &mut r);
    p.update(0.5, &mut r);
    assert_eq!((p.loops_emitted(), p.countdown(), p.phase()), (1, 2.5, 1.0));
    for _ in 0..5 {
        p.update(0.5, &mut r);
    }
    // w reached exactly 0: e = 0, no new loop.
    assert_eq!((p.loops_emitted(), p.countdown()), (1, 0.0));
    p.update(0.5, &mut r);
    assert_eq!(p.loops_emitted(), 2);
}

#[test]
fn no_emission_after_a_delayed_stop() {
    // EF13 (d).
    let mut d = inject(1.0, 10.0);
    d.loop_count = 0;
    let mut r = rng();
    let mut p = ParticleSimulator::new(d);
    p.start(0, &mut r);
    p.update(1.0, &mut r);
    p.stop(0);
    let created = p.particles_created();
    for _ in 0..5 {
        p.update(1.0, &mut r);
    }
    assert_eq!(p.particles_created(), created);
    // Burst age 1 s: 9 s left at the stop, 4 after the five updates.
    assert!(p.is_active());
    assert_eq!(run(&mut p, &mut r, 1.0, 100), Some(4));
}

#[test]
fn start_one_does_not_restart_a_finite_emitter_and_takes_the_preroll() {
    let mut r = rng();
    let mut p = ParticleSimulator::new(inject(1.0, 1.0));
    p.start(1, &mut r);
    assert!(!p.is_active());
    // Hold flag: restartable; the pre-roll is added once to the first step
    // and the emit time is capped at pre-roll − 0.01.
    let mut d = EmitterDesc::new(emitter_flag::LOOP_BUDGET | emitter_flag::HOLD, 1.0, 100.0);
    d.loop_count = 0;
    d.preroll = 2.0;
    let mut p = ParticleSimulator::new(d.clone());
    p.start(1, &mut r);
    assert!(p.is_active());
    p.update(0.5, &mut r);
    assert_eq!(p.age(), 2.5);
    assert_eq!(p.phase(), f64::from(0.0f32 + (2.0f64 - 0.01) as f32));
    p.update(0.5, &mut r);
    assert_eq!(p.age(), 3.0);
    // Start(0) never takes the pre-roll.
    let mut p = ParticleSimulator::new(d);
    p.start(0, &mut r);
    p.update(0.5, &mut r);
    assert_eq!(p.age(), 0.5);
}

#[test]
fn rate_emitter_carries_fractions_and_counts_loops() {
    // Rate 8/s at 1/16 s: x = 0.5 per update; a particle every second
    // update; loop count 1 stops when P passes 1 (update 17).
    let mut d = EmitterDesc::new(0, 8.0, 100.0);
    d.loop_count = 1;
    let mut r = rng();
    let mut p = ParticleSimulator::new(d);
    p.start(0, &mut r);
    p.update(0.0625, &mut r);
    assert_eq!((p.particles_created(), p.carry()), (0, 0.5));
    p.update(0.0625, &mut r);
    assert_eq!((p.particles_created(), p.carry()), (1, 0.0));
    for _ in 0..14 {
        p.update(0.0625, &mut r);
    }
    assert_eq!((p.particles_created(), p.loops_emitted()), (8, 0));
    assert!(!p.delayed_stop_requested());
    p.update(0.0625, &mut r);
    assert!(p.delayed_stop_requested());
    assert_eq!((p.particles_created(), p.loops_emitted()), (8, 1));
}

#[test]
fn rate_emitter_ages_oldest_first_in_substeps() {
    // Rate 30/s, one 1/15 s step: x = 2, ages e and e − s/2.
    let mut d = EmitterDesc::new(0, 30.0, 1.0);
    d.loop_count = 0;
    let mut r = rng();
    let mut p = ParticleSimulator::new(d.clone());
    p.start(0, &mut r);
    p.update(RATE_SUBSTEP, &mut r);
    let s = RATE_SUBSTEP;
    assert_eq!(p.live(), &[1.0 - s, 1.0 - (s - s / 2.0)]);
    // A 0.25 s step is cut into 1/15 s sub-steps: 7.5 particles over it.
    let mut p = ParticleSimulator::new(d);
    p.start(0, &mut r);
    p.update(0.25, &mut r);
    assert_eq!(p.particles_created(), 7);
}

#[test]
fn rate_emitter_holds_at_the_last_loop_with_flag_8() {
    let mut d = EmitterDesc::new(emitter_flag::HOLD, 8.0, 0.5);
    d.loop_count = 1;
    let mut r = rng();
    let mut p = ParticleSimulator::new(d);
    p.start(0, &mut r);
    assert_eq!(run(&mut p, &mut r, 0.1, 200), None);
    assert_eq!(p.loops_emitted(), 0);
    assert!(p.phase() < 1.0);
    assert!(p.particles_created() > 150);
}

#[test]
fn plain_emitter_never_self_stops() {
    let mut r = rng();
    let mut d = EmitterDesc::new(emitter_flag::PLAIN, 4.0, 1.5);
    d.loop_count = 1;
    let mut p = ParticleSimulator::new(d);
    p.start(0, &mut r);
    assert_eq!(run(&mut p, &mut r, 0.5, 1000), None);
    p.stop(0);
    assert!(p.is_active() && p.live_count() > 0);
    assert!(run(&mut p, &mut r, 0.5, 100).is_some());
}

#[test]
fn hard_stop_is_immediate_and_stop_needs_started() {
    let mut r = rng();
    let mut d = inject(1.0, 10.0);
    d.loop_count = 0;
    let mut p = ParticleSimulator::new(d);
    p.stop(0);
    assert!(!p.is_active() && !p.delayed_stop_requested());
    p.start(0, &mut r);
    p.update(0.5, &mut r);
    assert!(p.live_count() > 0);
    p.stop(1);
    assert!(!p.is_active());
    assert_eq!(p.live_count(), 0);
}

#[test]
fn volcano_particle_ends_within_its_life() {
    // §7.1: loop count 1, life 3-20 s: ends on its own, well before the
    // crater decal's 50 s.
    for life in [3.0f32, 20.0] {
        let mut r = rng();
        let mut p = ParticleSimulator::new(inject(4.0, life));
        p.start(0, &mut r);
        let n = run(&mut p, &mut r, 1.0 / 30.0, 10_000).expect("ends") as f32 / 30.0;
        assert!(n <= life + 0.1 && n < 50.0, "{life}: {n}");
    }
}
