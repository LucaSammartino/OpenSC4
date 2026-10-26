// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The particle component's start/stop protocol and its emitters
//! (`docs/specs/effects.md` §7.1, §7.1.1; EF1, EF13; conformance test 16).
//!
//! The default projection models what the finish test (§7) observes: `started`, the
//! delayed-stop request, the live particles and the emitter state (phase,
//! loop counter, carry, countdown). `set_motion` adds the complete creation
//! record and supported physics from particles.md without a second emitter.
//! Checked entry points expose PT1/PT8 helper limits. Abstract draw commands
//! preserve dispatch and finish ordering, without claiming pixel conformance.
//! Native owner outputs are not inputs.
//!
//! The protocol (§7.1):
//! - base `Stop(mode1, _)` with `started` set: `delayed := mode1`; mode1 == 0
//!   is immediate (`started := false`, every particle destroyed); otherwise
//!   only the request is recorded and the particles play out;
//! - the component's `Stop(mode)` calls the base with `(mode == 0, 0)`, so
//!   **`Stop(0)` is the delayed stop** and `Stop(1)`/`Stop(2)` are immediate;
//! - `CheckForStop`, at the end of every draw-time update: live count 0 and a
//!   delayed stop requested → base `Stop(0, 0)` (immediate), `started` clears.
//!
//! The emitters (§7.1.1): the loop-budget emitter (flag A `0x02`) makes one
//! burst of `ceil(curve[0])` per loop; the rate emitter (neither `0x02` nor
//! `0x04`) integrates the count curve in sub-steps of at most 1/15 s; both
//! count loops of `D+0x14` seconds in `n` against `D+0x18` and call `Stop(0)`
//! when it is reached. Random draws come from the global `cRZRandom`
//! (`0x00B22DF8`), passed in by the caller.

use super::particle_owners::{
    ordinary::{birth_events, crossing_events, Motion, Record},
    Unsupported,
};
use crate::rng::RzRandom;

/// Particle description flag A bits (§7.1, §7.1.1).
pub mod emitter_flag {
    /// `0x02` (`inject`): the loop-budget emitter.
    pub const LOOP_BUDGET: u32 = 0x02;
    /// `0x04` (`maintain`): the plain rate-integral emitter (no self-stop);
    /// wins over `0x02`.
    pub const PLAIN: u32 = 0x04;
    /// `0x08`: a finite emitter may be restarted by `Start(arg != 0)`, and
    /// the rate emitter holds at its last loop instead of stopping.
    pub const HOLD: u32 = 0x08;
    /// Bits 19 / 20 / 21: the rate scale `+0xC0` once, squared, cubed.
    pub const SCALE_1: u32 = 1 << 19;
    pub const SCALE_2: u32 = 1 << 20;
    pub const SCALE_3: u32 = 1 << 21;
}

/// The rate emitter's largest sub-step (`0x00AA0CF4`).
pub const RATE_SUBSTEP: f32 = 1.0 / 15.0;

/// Which emitter the dispatch picks (§7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitterKind {
    /// `0x005B3CF0`: no self-stop.
    Plain,
    /// `0x005B8B50`: one burst per loop, counts loops against `D+0x18`.
    LoopBudget,
    /// `0x005B87B0`: the rate emitter, counts loops against `D+0x18`.
    Rate,
}

impl EmitterKind {
    /// By flag A: `0x04` → plain, else `0x02` → loop budget, else rate.
    pub fn from_flags(flags: u32) -> EmitterKind {
        if flags & emitter_flag::PLAIN != 0 {
            EmitterKind::Plain
        } else if flags & emitter_flag::LOOP_BUDGET != 0 {
            EmitterKind::LoopBudget
        } else {
            EmitterKind::Rate
        }
    }
}

/// The particle description fields the emitters read (§7.1.1), plus the
/// detail scales. Life is sampled by particle init, not supplied by an owner.
#[derive(Clone, Debug, PartialEq)]
pub struct EmitterDesc {
    /// Flags A (`D+0x00`).
    pub flags_a: u32,
    /// `D+0x08`; bit 0 set: the burst is not scaled.
    pub flags_b: u32,
    /// Lower particle-life bound (`D+0x0C`), seconds.
    pub life_min: f32,
    /// Upper particle-life bound and max burst age (`D+0x10`), seconds.
    pub max_age: f32,
    /// Period (`D+0x14`), seconds per loop.
    pub period: f32,
    /// Loop count (`D+0x18`), 0 = unlimited.
    pub loop_count: u32,
    /// Pre-roll (`D+0x1C`), seconds.
    pub preroll: f32,
    /// Start-delay range (`D+0x20..+0x24`).
    pub start_delay: (f32, f32),
    /// Interval range (`D+0x28..+0x2C`).
    pub interval: (f32, f32),
    /// Count curve (`D+0x88..+0x8C`).
    pub curve: Vec<f32>,
    /// The detail/density scales `*(+0x1E8)`, `+0x1E0`, `+0x1C4`.
    pub scales: [f32; 3],
    /// The rate scale `+0xC0`.
    pub rate_scale: f32,
}

impl EmitterDesc {
    /// A description with no scaling, no delays (both ranges negative), no
    /// pre-roll, period 1 s, one loop, a one-entry curve, and a degenerate
    /// life range `[life, life]` (init still consumes a random draw).
    pub fn new(flags_a: u32, count: f32, life: f32) -> EmitterDesc {
        EmitterDesc {
            flags_a,
            flags_b: 0,
            life_min: life,
            max_age: life,
            period: 1.0,
            loop_count: 1,
            preroll: 0.0,
            start_delay: (-1.0, -1.0),
            interval: (-1.0, -1.0),
            curve: vec![count],
            scales: [1.0; 3],
            rate_scale: 1.0,
        }
    }

    fn kind(&self) -> EmitterKind {
        EmitterKind::from_flags(self.flags_a)
    }

    /// `m`: the product of the three detail scales.
    fn m(&self) -> f32 {
        self.scales[0] * self.scales[1] * self.scales[2]
    }

    /// `m'`: `m` times `+0xC0` once, squared or cubed for bits 19/20/21.
    /// EF14: with several bits set the lowest wins; with none the
    /// rate scale is not applied.
    fn m_rate(&self) -> f32 {
        let k = self.rate_scale;
        let f = if self.flags_a & emitter_flag::SCALE_1 != 0 {
            k
        } else if self.flags_a & emitter_flag::SCALE_2 != 0 {
            k * k
        } else if self.flags_a & emitter_flag::SCALE_3 != 0 {
            k * k * k
        } else {
            1.0
        };
        self.m() * f
    }

    /// The rate at loop fraction `frac`: one entry → `curve[0]`, else linear
    /// interpolation at `(len − 1)·frac`. EF14's documented safe choices:
    /// an empty curve is 0; clamp out-of-range fractions (native UB).
    fn rate_at(&self, frac: f64) -> f32 {
        match self.curve.len() {
            0 => 0.0,
            1 => self.curve[0],
            len => {
                let pos = (len - 1) as f64 * frac.clamp(0.0, 1.0);
                let i = pos.floor() as usize;
                let t = pos - i as f64;
                if t > 0.0 {
                    (f64::from(self.curve[i]) + t * (f64::from(self.curve[i + 1]) - f64::from(self.curve[i]))) as f32
                } else {
                    self.curve[i]
                }
            }
        }
    }
}

/// The stored float age and life used by main slot 20 (EF14).
#[derive(Clone, Debug, PartialEq)]
struct Particle {
    age: f32,
    life: f32,
    record: Option<Record>,
    _lease: Option<super::particle_owners::pool::Lease>,
}

impl Particle {
    fn update(&mut self, t: f32, motion: Option<&Motion>) -> Result<bool, Unsupported> {
        if self.record.as_ref().is_some_and(|p| !p.alive) {
            return Ok(false);
        }
        self.age += t;
        let alive = self.age < self.life;
        if let Some(record) = &mut self.record {
            record.alive = alive;
            if alive {
                motion.expect("motion record owner").update(record, t)?;
            }
        }
        Ok(alive && self.record.as_ref().map_or(true, |p| p.alive))
    }
}

/// One particle component (`cSCParticleSimulatorBase` with the emitter of
/// `cSC4ParticlesEffect` / `cSC4ModelBasedParticlesEffect`).
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleSimulator {
    desc: EmitterDesc,
    /// `+0x05`.
    started: bool,
    /// `+0x0C`: a delayed stop was requested.
    delayed_stop: bool,
    /// The live particles (`+0x14` is their count).
    live: Vec<Particle>,
    /// `P` (`+0x120`), in loops.
    phase: f64,
    /// `n` (`+0x128`).
    loops: u32,
    /// `ρ` (`+0x130`), loops per second.
    rho: f64,
    /// `c` (`+0x138`).
    carry: f32,
    /// `w` (`+0x190`), seconds.
    countdown: f32,
    /// `dt` (`+0x108`).
    dt: f32,
    /// Total age (`+0x110`).
    age: f64,
    /// Pre-roll pending (`+0x118`).
    preroll_pending: bool,
    /// Emission suppressed (`+0x21A`).
    suppressed: bool,
    /// `+0x188`: the last `Start` argument.
    start_arg: u32,
    /// `+0x18C`: the last component `Stop` mode.
    stop_mode: u32,
    /// Particles created over the component's life (diagnostics).
    created: u64,
    /// Optional full particles.md records; legacy finish-only callers retain
    /// their existing projection without consuming extra random draws.
    motion: Option<Motion>,
    events: Vec<(Vec<u8>, [f32; 3])>,
}

impl ParticleSimulator {
    pub fn init_native_services(
        &mut self,
        state: &mut super::particle_owners::ordinary::NativeServices,
        shared: u64,
        foreign: &mut impl super::particle_owners::ordinary::TerrainServices,
    ) {
        state.init(shared, foreign);
        if let Some(motion) = &mut self.motion {
            motion.city_extent = Some(state.extent);
        }
    }
    /// Component main slot 4 is distinct from the draw-associated update.
    pub fn component_update(&mut self, _dt: f32) {}
    pub fn set_float_parameter(&mut self, id: u32, values: &[f32]) -> Result<(), Unsupported> {
        if let Some(motion) = &mut self.motion {
            if let Some(result) = motion.set_float(id, values) {
                return match result? {
                    true => Ok(()),
                    false => Err(Unsupported::PT13),
                };
            }
        }
        match id {
            2 => self.desc.scales[1] = values[0],
            0x102 => self.desc.scales[2] = values[0],
            _ => return Err(Unsupported::PT13),
        }
        Ok(())
    }
    /// A component with its description set (`SetDescription`).
    pub fn new(desc: EmitterDesc) -> ParticleSimulator {
        let mut p = ParticleSimulator {
            desc: EmitterDesc::new(0, 0.0, 0.0),
            started: false,
            delayed_stop: false,
            live: Vec::new(),
            phase: 0.0,
            loops: 0,
            rho: 1.0,
            carry: 0.0,
            countdown: 0.0,
            dt: 0.0,
            age: 0.0,
            preroll_pending: false,
            suppressed: false,
            start_arg: 0,
            stop_mode: 0,
            created: 0,
            motion: None,
            events: Vec::new(),
        };
        p.set_description(desc);
        p
    }

    /// `SetDescription` `0x005B7C90`: `ρ := 1 / D+0x14`, the period replaced
    /// by the local float `1e-6` unless greater; clears `+0x21A` and resets
    /// scale `+0x1E0`. EF14: the replacement is used for `ρ` only; burst
    /// age and gaps still use the description's raw period.
    pub fn set_description(&mut self, mut desc: EmitterDesc) {
        let period = if desc.period > 1e-6 { desc.period } else { 1e-6 };
        self.rho = 1.0 / f64::from(period);
        self.suppressed = false;
        desc.scales[1] = 1.0;
        self.desc = desc;
    }

    /// `+0x21A`: while set the emitter dispatch is skipped.
    pub fn set_suppressed(&mut self, on: bool) {
        self.suppressed = on;
    }

    /// `Start(arg)` `0x005B3000`. A finite emitter (`D+0x18 > 0`, flag A
    /// `0x08` clear) is not restarted by `Start(arg != 0)`. Otherwise
    /// `started` is set (no stop pending), `P`, `n`, `c` and the age reset,
    /// the countdown seeded from the start-delay range if `D+0x20 ≥ 0`, else
    /// the interval range if `D+0x28 ≥ 0`, else 0 (`rng` is the global
    /// `cRZRandom`), and the pre-roll is pending only for `arg != 0`.
    pub fn start(&mut self, arg: u32, rng: &mut RzRandom) {
        self.start_checked(arg, rng, None)
            .expect("use start_checked for unresolved motion helpers");
    }
    pub fn start_checked(&mut self, arg: u32, rng: &mut RzRandom, sea_access: Option<u32>) -> Result<(), Unsupported> {
        let d = &self.desc;
        if arg != 0 && d.loop_count > 0 && d.flags_a & emitter_flag::HOLD == 0 {
            return Ok(());
        }
        if self.started && !self.delayed_stop {
            return Ok(());
        }
        self.started = true;
        self.delayed_stop = false;
        self.phase = 0.0;
        self.loops = 0;
        self.carry = 0.0;
        self.age = 0.0;
        if let Some(motion) = &mut self.motion {
            motion.prepare_start(rng, sea_access)?;
        }
        let (sd, iv) = (d.start_delay, d.interval);
        self.countdown = if sd.0 >= 0.0 {
            rng.range_f64(f64::from(sd.0), f64::from(sd.1)) as f32
        } else if iv.0 >= 0.0 {
            rng.range_f64(f64::from(iv.0), f64::from(iv.1)) as f32
        } else {
            0.0
        };
        self.start_arg = arg;
        self.stop_mode = 0;
        self.preroll_pending = arg != 0;
        Ok(())
    }

    /// The component's `Stop(mode)` `0x005B21E0`: stores the mode and calls
    /// the base with `(mode == 0, 0)` — mode 0 is the delayed stop.
    pub fn stop(&mut self, mode: u32) {
        self.stop_mode = mode;
        self.base_stop(mode == 0);
    }

    /// The base `Stop(mode1, 0)`.
    fn base_stop(&mut self, mode1: bool) {
        if !self.started {
            return;
        }
        self.delayed_stop = mode1;
        if !mode1 {
            self.started = false;
            self.live.clear();
        }
    }

    /// `IsActive` of the component (§7 table): the `started` byte.
    pub fn is_active(&self) -> bool {
        self.started
    }

    pub fn delayed_stop_requested(&self) -> bool {
        self.delayed_stop
    }

    /// `+0x14`.
    pub fn live_count(&self) -> usize {
        self.live.len()
    }

    /// Diagnostic remaining lives, in creation order, computed from the
    /// stored age and life (updates accumulate age, not remaining life).
    pub fn live(&self) -> Vec<f32> {
        self.live.iter().map(|p| p.life - p.age).collect()
    }

    /// `n` (`+0x128`).
    pub fn loops_emitted(&self) -> u32 {
        self.loops
    }

    /// `P` (`+0x120`).
    pub fn phase(&self) -> f64 {
        self.phase
    }

    /// `w` (`+0x190`).
    pub fn countdown(&self) -> f32 {
        self.countdown
    }

    /// `c` (`+0x138`).
    pub fn carry(&self) -> f32 {
        self.carry
    }

    /// Total age (`+0x110`).
    pub fn age(&self) -> f32 {
        self.age as f32
    }
    pub fn accumulated_time(&self) -> f64 {
        self.age
    }

    pub fn particles_created(&self) -> u64 {
        self.created
    }

    /// Enable full records in the SAME emitter, rather than a second emission
    /// implementation. Unsettled helpers are rejected explicitly before Start.
    pub fn set_motion(&mut self, motion: Motion) -> Result<(), Unsupported> {
        assert!(self.live.is_empty(), "configure before particle creation");
        let d = &motion.description.value;
        let f = |at| d.get_scalar(at).expect("initialized description");
        self.set_description(EmitterDesc {
            flags_a: d.words[&0],
            flags_b: d.words[&8],
            life_min: f(0x0c),
            max_age: f(0x10),
            period: f(0x14),
            loop_count: d.words[&0x18],
            preroll: f(0x1c),
            start_delay: (f(0x20), f(0x24)),
            interval: (f(0x28), f(0x2c)),
            curve: d.floats[&0x88].clone(),
            scales: [1.; 3],
            rate_scale: motion.scale,
        });
        self.motion = Some(motion);
        Ok(())
    }
    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.live.iter().filter_map(|particle| particle.record.as_ref())
    }
    pub fn remove_particle(&mut self, index: usize) {
        let particle = &mut self.live[index];
        if let Some(record) = &mut particle.record {
            record.alive = false;
        } else {
            particle.life = particle.age;
        }
    }
    /// Suffix requests can be consumed directly or fired through §8.3's directory.
    pub fn take_events(&mut self) -> Vec<(Vec<u8>, [f32; 3])> {
        std::mem::take(&mut self.events)
    }
    pub fn fire_events(&mut self, directory: &mut impl super::particle_owners::ordinary::EventDirectory) {
        if let Some(motion) = &self.motion {
            let name = motion
                .description
                .value
                .strings
                .get(&0x160)
                .map_or(&[][..], Vec::as_slice);
            for (suffix, position) in std::mem::take(&mut self.events) {
                super::particle_owners::ordinary::fire_event(name, &suffix, position, directory);
            }
        }
    }
    pub fn draw_slot(&self, context_present: bool, model_present: bool) -> u8 {
        if !context_present || self.live.is_empty() {
            19
        } else if model_present {
            18
        } else {
            17
        }
    }

    /// The draw-time update `0x005B8540` (only while started): the step
    /// (plus the pending pre-roll once) is stored and added to the age;
    /// every live particle is aged first, then the emitter dispatch runs
    /// unless suppressed, then `CheckForStop`.
    pub fn update(&mut self, dt: f32, rng: &mut RzRandom) {
        self.update_checked(dt, rng)
            .expect("use update_checked for unresolved motion helpers");
    }
    pub fn update_checked(&mut self, dt: f32, rng: &mut RzRandom) -> Result<(), Unsupported> {
        self.update_with_draw(dt, rng, false, false, &mut |_| {})
    }
    /// Abstract slot commands preserve draw-before-finish ordering (§7). Exact
    /// vertex/state packing remains PT8; the foreign context owns its lifetime.
    pub fn update_with_draw(
        &mut self,
        dt: f32,
        rng: &mut RzRandom,
        context: bool,
        model: bool,
        draw: &mut impl FnMut(DrawCommand),
    ) -> Result<(), Unsupported> {
        if !self.started {
            return Ok(());
        }
        let mut t = dt;
        if self.preroll_pending {
            t += self.desc.preroll;
            self.preroll_pending = false;
        }
        self.dt = t;
        self.age += f64::from(t);
        if let Some(motion) = &mut self.motion {
            if motion.flags() & 0x2000 != 0 {
                return Err(Unsupported::PT1);
            }
            if !motion.description.value.floats.get(&0x1a4).map_or(true, Vec::is_empty)
                && motion.description.value.get_scalar(0x1b0).unwrap_or(0.) > 0.
            {
                return Err(Unsupported::PT1);
            }
            motion.frame_force();
            if motion.flags() & 0x8000_0000 != 0 {
                for particle in &self.live {
                    if let Some(record) = &particle.record {
                        if record.alive {
                            for name in crossing_events(&motion.description.value.timed_effects, particle.age, t) {
                                self.events.push((name.to_vec(), record.position));
                            }
                        }
                    }
                }
            }
        }
        let mut index = 0;
        while index < self.live.len() {
            if self.live[index].update(t, self.motion.as_ref())? {
                index += 1;
            } else {
                self.live.remove(index);
            }
        }
        if !self.suppressed {
            self.dispatch(rng)?;
        }
        let slot = self.draw_slot(context, model);
        if slot == 19 {
            draw(DrawCommand::Slot(19));
        } else {
            draw(DrawCommand::Slot(14));
            if self.desc.flags_a & 1 != 0 {
                draw(DrawCommand::SelfLit(true));
            }
            draw(DrawCommand::ModelTransform);
            draw(DrawCommand::RenderState);
            draw(DrawCommand::Slot(slot));
            if self.desc.flags_a & 1 != 0 {
                draw(DrawCommand::SelfLit(false));
            }
        }
        if !self.check_for_stop() && self.motion.as_ref().is_some_and(|m| m.flags() & 0x800000 != 0) {
            return Err(Unsupported::PT1);
        }
        Ok(())
    }

    /// Init always sets alive and age 0 and draws life first, even for a
    /// degenerate range or a particle rejected by the later slot-20 test.
    /// Plain emitters skip slot 20 entirely (`initial_age == None`).
    fn create(&mut self, initial_age: Option<f32>, rng: &mut RzRandom) -> Result<(), Unsupported> {
        self.created += 1;
        let lease = if self.motion.is_some() {
            let pool = super::particle_owners::pool::global_effect_pool();
            let Some(lease) = super::particle_owners::pool::Pool::allocate(&pool, |_| true) else {
                return Ok(());
            };
            Some(lease)
        } else {
            None
        };
        let mut p = Particle {
            age: 0.0,
            life: rng.range_f64(f64::from(self.desc.life_min), f64::from(self.desc.max_age)) as f32,
            record: self.motion.as_ref().map(|motion| motion.init(rng)).transpose()?,
            _lease: lease,
        };
        let alive = match initial_age {
            Some(a) => p.update(a, self.motion.as_ref())?,
            None => true,
        };
        if alive {
            if initial_age.is_some() {
                if let (Some(motion), Some(record)) = (&self.motion, &p.record) {
                    if motion.flags() & 0x8000_0000 != 0 {
                        for name in birth_events(&motion.description.value.timed_effects) {
                            self.events.push((name.to_vec(), record.position));
                        }
                    }
                }
            }
            self.live.push(p);
        }
        Ok(())
    }

    /// The dispatch `0x005B96E0`.
    fn dispatch(&mut self, rng: &mut RzRandom) -> Result<(), Unsupported> {
        if self.delayed_stop {
            return Ok(());
        }
        let mut e = if self.countdown > 0.0 {
            self.countdown -= self.dt;
            if self.countdown > 0.0 {
                return Ok(());
            }
            -self.countdown
        } else {
            self.dt
        };
        let pre = f64::from(self.desc.preroll);
        if pre > 0.0 && f64::from(e) > pre - 0.01 {
            e = (pre - 0.01) as f32;
        }
        match self.desc.kind() {
            EmitterKind::Plain => self.emit_plain(e, rng),
            EmitterKind::LoopBudget => self.emit_loop_budget(e, rng),
            EmitterKind::Rate => self.emit_rate(e, rng),
        }
    }

    /// The loop-budget emitter `0x005B8B50`: one burst per loop.
    fn emit_loop_budget(&mut self, e: f32, rng: &mut RzRandom) -> Result<(), Unsupported> {
        self.phase += f64::from(e) * self.rho;
        if self.phase <= f64::from(self.loops) {
            return Ok(());
        }
        let d = &self.desc;
        let mut x = d.curve.first().copied().unwrap_or(0.0);
        if x > 20.0 && d.flags_b & 1 == 0 {
            x *= d.m();
        }
        // [choice] A negative count creates nothing.
        let burst = x.ceil().max(0.0) as u32;
        let (period, max_age, loop_count, iv) = (d.period, d.max_age, d.loop_count, d.interval);
        while self.phase > f64::from(self.loops) {
            let a = ((self.phase - f64::from(self.loops)) * f64::from(period)) as f32;
            if a < max_age && burst != 0 {
                for _ in 0..burst {
                    self.create(Some(a), rng)?;
                }
            }
            self.loops += 1;
            if loop_count != 0 && self.loops >= loop_count {
                self.stop(0);
                return Ok(());
            }
            if iv.0 >= 0.0 {
                let gap = rng.range_f64(f64::from(iv.0), f64::from(iv.1));
                self.phase -= gap * self.rho;
                if self.phase < f64::from(self.loops) {
                    self.countdown = ((f64::from(self.loops) - self.phase) * f64::from(period)) as f32;
                    self.phase = f64::from(self.loops);
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// The rate emitter `0x005B87B0`: sub-steps of at most 1/15 s.
    fn emit_rate(&mut self, mut e: f32, rng: &mut RzRandom) -> Result<(), Unsupported> {
        let (period, loop_count, iv, hold) = (
            self.desc.period,
            self.desc.loop_count,
            self.desc.interval,
            self.desc.flags_a & emitter_flag::HOLD != 0,
        );
        let m = self.desc.m_rate();
        while e > 0.0 {
            let mut s = e.min(RATE_SUBSTEP);
            self.phase += f64::from(s) * self.rho;
            let mut gap = false;
            if self.phase > f64::from(self.loops) + 1.0 {
                self.loops += 1;
                if loop_count == 0 || self.loops != loop_count {
                    if iv.0 >= 0.0 {
                        s -= ((self.phase - f64::from(self.loops)) * f64::from(period)) as f32;
                        self.phase = f64::from(self.loops);
                        gap = true;
                    }
                } else if !hold {
                    self.stop(0);
                    return Ok(());
                } else {
                    self.loops = loop_count - 1;
                    self.phase = f64::from(loop_count) - 1e-6;
                }
            }
            let r = self.desc.rate_at(self.phase - f64::from(self.loops));
            let x = r * s * m + self.carry;
            let n = x.floor();
            self.carry = x - n;
            let n = n.max(0.0) as u32;
            for k in 0..n {
                self.create(Some(e - k as f32 * s / n as f32), rng)?;
            }
            e -= s;
            if gap {
                e -= rng.range_f64(f64::from(iv.0), f64::from(iv.1)) as f32;
                if e < 0.0 {
                    self.countdown = -e;
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// EF14(a): top up to ceil(curve[0]), with no scaling, wrapping,
    /// counter/carry/countdown updates, slot 20 or birth hook.
    fn emit_plain(&mut self, e: f32, rng: &mut RzRandom) -> Result<(), Unsupported> {
        self.phase += f64::from(e) * self.rho;
        if self.phase <= f64::from(self.loops) {
            return Ok(());
        }
        // [choice, EF14] Empty curve -> 0; T <= live -> no creations,
        // avoiding the native unsigned-underflow loop (~2^32 attempts).
        let target = self.desc.curve.first().copied().unwrap_or(0.0).ceil().max(0.0) as u32;
        let count = if self.motion.is_some() {
            target.wrapping_sub(self.live.len() as u32)
        } else {
            target.saturating_sub(self.live.len() as u32)
        };
        for _ in 0..count {
            self.create(None, rng)?;
        }
        Ok(())
    }

    /// `CheckForStop` `0x005B9D40`: a requested delayed stop becomes
    /// immediate once no particle is alive.
    fn check_for_stop(&mut self) -> bool {
        if self.started && self.delayed_stop && self.live.is_empty() {
            self.base_stop(false);
            return true;
        }
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawCommand {
    Slot(u8),
    SelfLit(bool),
    ModelTransform,
    RenderState,
}

#[cfg(test)]
#[path = "particles_tests.rs"]
mod tests;
