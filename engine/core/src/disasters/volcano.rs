// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The volcano (§10, short form; Q25). The "volcano_control" effect and the
//! lava are the host's (the lava is a terrain class); the core runs the
//! states, the clock and the rates:
//!
//! - start: the location set, `Init` (state 1), state 3, which starts the
//!   effect once and, with lava enabled, creates the lava and goes to 5,
//!   which starts the flow, all inside the start call;
//! - each tick while the lava flows: after `T` seconds (strictly)
//!   `EndVolcanoLava` (which ends the disaster) and state 3, or 8 when the
//!   end was requested; otherwise the rates;
//! - `0xCA493C1D` for the instance (the effect finished) is the end request,
//!   state 7: stay in the flow, or go to 8, which removes and shuts the
//!   instance down.
//!
//! The start registers a location twice, once when the location is set and
//! once in `Init`; the spec lists both steps, and we keep both [question,
//! DS42].

use super::system::{Core, InstanceState};
use super::{instance_class, kind, msg, TICK_PRIORITY};
use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;

/// Tuning properties of the "Volcano" exemplar (§10).
pub mod volcano_prop {
    pub const PRODUCTION: u32 = 0xEA4B_C6E4;
    pub const SOLIDIFICATION: u32 = 0x4A4B_C714;
    pub const SCALE: u32 = 0xAA4B_C72A;
    pub const LAVA_SECONDS: u32 = 0x0A4B_C742;
    pub const VENT_RADIUS: u32 = 0x0A4B_C752;
    pub const LAVA_FLOWS: u32 = 0x6A4B_C8ED;
}

/// The shipped "Volcano" values (§10).
pub const SHIPPED_PRODUCTION: [(f32, f32); 5] = [(0.0, 0.0), (0.075, 0.0), (0.25, 0.25), (0.8, 0.25), (1.0, 0.0)];
pub const SHIPPED_SOLIDIFICATION: [(f32, f32); 4] = [(0.0, 0.0), (0.4, 0.0), (0.9, 0.01), (1.0, 0.15)];
pub const SHIPPED_SCALE: f32 = 6.0;
pub const SHIPPED_LAVA_SECONDS: f32 = 50.0;
/// The process statics' values before any read (§10): also what a missing
/// property keeps.
pub const DEFAULT_SCALE: f32 = 1.0;
pub const DEFAULT_LAVA_SECONDS: f32 = 20.0;
pub const DEFAULT_VENT_RADIUS: f32 = 100.0;
/// The rate divisor (§10).
pub const RATE_DIVISOR: f32 = 100_000.0;

#[derive(Clone, Debug, PartialEq)]
pub struct VolcanoTuning {
    pub production: ResponseCurve,
    pub solidification: ResponseCurve,
    pub scale: f32,
    pub lava_seconds: f32,
    pub vent_radius: f32,
    /// `0x6A4BC8ED`: lava is enabled.
    pub lava_flows: bool,
}

impl Default for VolcanoTuning {
    fn default() -> VolcanoTuning {
        VolcanoTuning::from_holder(None)
    }
}

impl VolcanoTuning {
    /// The read at `Init` (§10): without the exemplar, T = 20 s, scale 1,
    /// radius 100, empty curves and no lava; with it, each property present
    /// overrides, and the lava flag is false unless present.
    pub fn from_holder(h: Option<&PropertyHolder>) -> VolcanoTuning {
        let curve = |id| h.and_then(|h| crate::props::curve(h, id)).unwrap_or_default();
        let f = |id, d: f32| h.and_then(|h| crate::props::f32_of(h, id)).unwrap_or(d);
        VolcanoTuning {
            production: curve(volcano_prop::PRODUCTION),
            solidification: curve(volcano_prop::SOLIDIFICATION),
            scale: f(volcano_prop::SCALE, DEFAULT_SCALE),
            lava_seconds: f(volcano_prop::LAVA_SECONDS, DEFAULT_LAVA_SECONDS),
            vent_radius: f(volcano_prop::VENT_RADIUS, DEFAULT_VENT_RADIUS),
            lava_flows: h
                .and_then(|h| crate::props::bool_of(h, volcano_prop::LAVA_FLOWS))
                .unwrap_or(false),
        }
    }

    /// (production, solidification) at `t_us` µs into the flow (§10 "Lava"
    /// step 1): both 0 when the production curve has no points *[quirk]*.
    pub fn rates(&self, t_us: u64) -> (f32, f32) {
        if self.production.is_empty() {
            return (0.0, 0.0);
        }
        let period = self.lava_seconds * 1e6;
        let r = if period > 0.0 { t_us as f32 / period } else { 1.0 };
        (
            self.production.eval(r) * self.scale / RATE_DIVISOR,
            self.solidification.eval(r) / RATE_DIVISOR,
        )
    }
}

/// The shipped "Volcano" exemplar (§10), for tests and tools: the lava is
/// enabled (shipped true); T and scale as shipped; the vent radius is not
/// given, so it is left out.
pub fn shipped_volcano() -> PropertyHolder {
    use crate::variant::{mem, Elem, Variant};
    let mut h = PropertyHolder::new();
    let curve = |v: &[(f32, f32)]| Variant::Array {
        elem: mem::FLOAT32,
        items: v.iter().flat_map(|&(x, y)| [Elem::F32(x), Elem::F32(y)]).collect(),
    };
    h.add(volcano_prop::PRODUCTION, curve(&SHIPPED_PRODUCTION));
    h.add(volcano_prop::SOLIDIFICATION, curve(&SHIPPED_SOLIDIFICATION));
    h.add(volcano_prop::SCALE, Variant::Scalar(Elem::F32(SHIPPED_SCALE)));
    h.add(volcano_prop::LAVA_SECONDS, Variant::Scalar(Elem::F32(SHIPPED_LAVA_SECONDS)));
    h.add(volcano_prop::LAVA_FLOWS, Variant::Scalar(Elem::Bool(true)));
    h
}

#[derive(Clone, Debug, Default)]
pub(crate) struct VolcanoState {
    pub state: u32,
    pub position: (f32, f32),
    pub cell: (i32, i32),
    /// The flow's clock (µs).
    pub lava_us: u64,
    pub end_requested: bool,
    /// The "volcano_control" effect was started.
    pub effect_started: bool,
    /// The lava object exists / flows / has run once.
    pub lava_created: bool,
    pub lava_flowing: bool,
    pub lava_has_run: bool,
    /// `DisasterHasBegun` ran and `DisasterHasEnded` has not yet.
    pub begun: bool,
    /// Flows started.
    pub lava_phases: u32,
}

/// A volcano's observable state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VolcanoSnapshot {
    pub id: u32,
    /// `GetPhase` (§3.2).
    pub phase: u32,
    pub state: u32,
    pub position: (f32, f32),
    pub cell: (i32, i32),
    pub lava_us: u64,
    pub end_requested: bool,
    pub lava_flowing: bool,
    pub lava_has_run: bool,
    pub lava_phases: u32,
}

/// The volcano's `GetPhase` (§3.2): state 0 → 0, 1 → 2, 2 and 3 → 1,
/// 4 → 3, 5 to 8 → 2.
pub fn volcano_phase(state: u32) -> u32 {
    match state {
        0 => 0,
        2 | 3 => 1,
        4 => 3,
        _ => 2,
    }
}

impl Core {
    fn vs(&mut self, id: u32) -> Option<&mut VolcanoState> {
        match &mut self.instances.get_mut(&id)?.state {
            InstanceState::Volcano(v) => Some(v),
            _ => None,
        }
    }

    pub(crate) fn vs_ref(&self, id: u32) -> Option<&VolcanoState> {
        match &self.instances.get(&id)?.state {
            InstanceState::Volcano(v) => Some(v),
            _ => None,
        }
    }

    fn vt(&self) -> VolcanoTuning {
        self.volcano_tuning.clone().unwrap_or_default()
    }

    /// `AttemptDisasterStart(x, z)` (§10), world units: only when the
    /// manager's list is empty.
    pub(crate) fn volcano_attempt_start(&mut self, x: i32, z: i32) -> Option<u32> {
        if self
            .managers
            .get(&kind::VOLCANO)
            .is_some_and(|m| !m.instances.is_empty())
        {
            return None;
        }
        let id = self.new_instance(
            kind::VOLCANO,
            instance_class::VOLCANO,
            InstanceState::Volcano(Box::default()),
        );
        self.add_instance_to_list(kind::VOLCANO, id);
        // SetLocation: store and register at the position's cell.
        let (px, pz) = (x as f32, z as f32);
        let cell = self.world_to_cell(px, pz);
        if let Some(v) = self.vs(id) {
            v.position = (px, pz);
            v.cell = cell;
        }
        self.register_location(id, cell.0, cell.1, 0, 0, false);
        self.volcano_init(id);
        self.volcano_set_state(id, 3);
        Some(id)
    }

    /// `Init` (§10).
    fn volcano_init(&mut self, id: u32) {
        let (sx, sz) = self.city_world_size();
        let Some(v) = self.vs_ref(id) else { return };
        let x = v.position.0.clamp(0.0, (sx - 1.0).max(0.0));
        let z = v.position.1.clamp(0.0, (sz - 1.0).max(0.0));
        let cell = self.world_to_cell(x, z);
        if let Some(v) = self.vs(id) {
            v.position = (x, z);
            v.cell = cell;
        }
        self.subscribe(id, msg::VOLCANO_MESSAGE);
        let h = self.host.tuning(super::tuning_instance::VOLCANO);
        self.volcano_tuning = Some(VolcanoTuning::from_holder(h.as_deref()));
        self.register_tick(id, TICK_PRIORITY);
        self.volcano_set_state(id, 1);
        self.register_location(id, cell.0, cell.1, 0, 0, false);
        if let Some(v) = self.vs(id) {
            v.begun = true;
        }
        self.disaster_has_begun(id);
    }

    /// `SetState` (§10): setting the current state again does nothing.
    fn volcano_set_state(&mut self, id: u32, state: u32) {
        let tuning = self.vt();
        let host = self.host.clone();
        let Some(v) = self.vs(id) else { return };
        if v.state == state {
            return;
        }
        v.state = state;
        let (x, z) = v.position;
        match state {
            3 => {
                if !v.effect_started {
                    v.effect_started = true;
                    host.volcano_start_deformation(id, x, z);
                }
                let Some(v) = self.vs(id) else { return };
                if tuning.lava_flows && !v.lava_flowing && !v.lava_has_run {
                    v.lava_created = true;
                    self.volcano_set_state(id, 5);
                }
            }
            5 => {
                if tuning.lava_flows && !v.lava_flowing {
                    v.lava_us = 0;
                    v.lava_flowing = true;
                    v.lava_created = true;
                    v.lava_phases += 1;
                    host.volcano_start_lava(id, x, z, tuning.vent_radius);
                }
            }
            7 => {
                v.end_requested = true;
                if v.lava_flowing {
                    self.volcano_set_state(id, 5);
                } else {
                    self.volcano_set_state(id, 8);
                }
            }
            8 => {
                if tuning.lava_flows {
                    self.end_volcano_lava(id);
                }
                if !self.vs_ref(id).is_some_and(|v| v.lava_flowing) {
                    self.instance_finish(id);
                }
            }
            _ => {}
        }
    }

    /// `EndVolcanoLava` (§10): the lava shut down and marked as run; a
    /// begun disaster ends and its locations go.
    fn end_volcano_lava(&mut self, id: u32) {
        let host = self.host.clone();
        let Some(v) = self.vs(id) else { return };
        if v.lava_created {
            host.volcano_end_lava(id);
        }
        v.lava_created = false;
        v.lava_flowing = false;
        v.lava_has_run = true;
        let begun = std::mem::take(&mut v.begun);
        if begun {
            self.disaster_has_ended();
            self.unregister_locations(id);
        }
    }

    /// The tick (§10), only while lava is enabled and flowing.
    pub(crate) fn volcano_tick(&mut self, id: u32, us: u32) {
        let tuning = self.vt();
        let host = self.host.clone();
        let Some(v) = self.vs(id) else { return };
        if !(tuning.lava_flows && v.lava_flowing) {
            return;
        }
        v.lava_us = v.lava_us.wrapping_add(u64::from(us));
        let t = v.lava_us as f64 * 1e-6;
        let end = v.end_requested;
        let lava_us = v.lava_us;
        if t > f64::from(tuning.lava_seconds) {
            self.end_volcano_lava(id);
            self.volcano_set_state(id, if end { 8 } else { 3 });
        } else if v.state == 5 {
            let (p, s) = tuning.rates(lava_us);
            host.volcano_lava_rates(id, p, s);
        }
    }

    /// `0xCA493C1D` (§10): the "volcano_control" effect of this instance
    /// finished (data1 = the instance): the end request, state 7.
    pub(crate) fn volcano_message(&mut self, id: u32, data1: i64) -> bool {
        if data1 != i64::from(id) {
            return false;
        }
        self.volcano_set_state(id, 7);
        true
    }

    /// The end request (state 7) for a host that has no effect message.
    pub(crate) fn volcano_request_end(&mut self, id: u32) -> bool {
        if self.vs_ref(id).is_none() {
            return false;
        }
        self.volcano_set_state(id, 7);
        true
    }

    /// `Shutdown` (§3.1): a begun disaster ends through `EndVolcanoLava`;
    /// the caller removes the instance from its manager.
    pub(crate) fn volcano_shutdown(&mut self, id: u32) {
        if self.vs_ref(id).is_some_and(|v| v.begun) {
            self.end_volcano_lava(id);
        }
    }

    pub(crate) fn volcano_snapshot(&self, id: u32) -> Option<VolcanoSnapshot> {
        let v = self.vs_ref(id)?;
        Some(VolcanoSnapshot {
            id,
            phase: volcano_phase(v.state),
            state: v.state,
            position: v.position,
            cell: v.cell,
            lava_us: v.lava_us,
            end_requested: v.end_requested,
            lava_flowing: v.lava_flowing,
            lava_has_run: v.lava_has_run,
            lava_phases: v.lava_phases,
        })
    }
}
