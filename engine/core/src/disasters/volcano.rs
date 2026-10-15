// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The volcano (§10, short form). The deformation and the lava are the
//! host's (the lava is a terrain class); the core runs the states and the
//! rates. The spec does not say when state 3 hands over to 5, who asks the
//! volcano to end, or what state 7 does: state 3 goes to 5 at the next tick,
//! [`super::Disasters::volcano_request_end`] asks it to end, and state 7 is
//! not used [choice, Q25].

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

/// The values of §10, also the defaults [choice, Q20].
pub const SHIPPED_PRODUCTION: [(f32, f32); 5] = [(0.0, 0.0), (0.075, 0.0), (0.25, 0.25), (0.8, 0.25), (1.0, 0.0)];
pub const SHIPPED_SOLIDIFICATION: [(f32, f32); 4] = [(0.0, 0.0), (0.4, 0.0), (0.9, 0.01), (1.0, 0.15)];
pub const SHIPPED_SCALE: f32 = 6.0;
pub const SHIPPED_LAVA_SECONDS: f32 = 50.0;
pub const SHIPPED_VENT_RADIUS: f32 = 20.0;
/// The rate divisor (§10).
pub const RATE_DIVISOR: f32 = 100_000.0;

#[derive(Clone, Debug, PartialEq)]
pub struct VolcanoTuning {
    pub production: ResponseCurve,
    pub solidification: ResponseCurve,
    pub scale: f32,
    pub lava_seconds: f32,
    pub vent_radius: f32,
    pub lava_flows: bool,
}

impl Default for VolcanoTuning {
    fn default() -> VolcanoTuning {
        VolcanoTuning::from_holder(None)
    }
}

impl VolcanoTuning {
    pub fn from_holder(h: Option<&PropertyHolder>) -> VolcanoTuning {
        let curve = |id, d: &[(f32, f32)]| {
            h.and_then(|h| crate::props::curve(h, id))
                .unwrap_or_else(|| ResponseCurve::from_points(d))
        };
        let f = |id, d: f32| h.and_then(|h| crate::props::f32_of(h, id)).unwrap_or(d);
        VolcanoTuning {
            production: curve(volcano_prop::PRODUCTION, &SHIPPED_PRODUCTION),
            solidification: curve(volcano_prop::SOLIDIFICATION, &SHIPPED_SOLIDIFICATION),
            scale: f(volcano_prop::SCALE, SHIPPED_SCALE),
            lava_seconds: f(volcano_prop::LAVA_SECONDS, SHIPPED_LAVA_SECONDS),
            vent_radius: f(volcano_prop::VENT_RADIUS, SHIPPED_VENT_RADIUS),
            lava_flows: h
                .and_then(|h| crate::props::bool_of(h, volcano_prop::LAVA_FLOWS))
                .unwrap_or(true),
        }
    }

    /// (production, solidification) at `t` µs into a lava phase (§10).
    pub fn rates(&self, t_us: u64) -> (f32, f32) {
        let period = self.lava_seconds * 1e6;
        let r = if period > 0.0 { t_us as f32 / period } else { 1.0 };
        (
            self.production.eval(r) * self.scale / RATE_DIVISOR,
            self.solidification.eval(r) / RATE_DIVISOR,
        )
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct VolcanoState {
    pub state: u32,
    pub position: (f32, f32),
    pub cell: (i32, i32),
    pub lava_us: u64,
    pub end_requested: bool,
    pub deforming: bool,
    pub lava_phases: u32,
}

/// A volcano's observable state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VolcanoSnapshot {
    pub id: u32,
    pub phase: u32,
    pub state: u32,
    pub position: (f32, f32),
    pub cell: (i32, i32),
    pub lava_us: u64,
    pub end_requested: bool,
    pub lava_phases: u32,
}

impl Core {
    fn vs(&mut self, id: u32) -> Option<&mut VolcanoState> {
        match &mut self.instances.get_mut(&id)?.state {
            InstanceState::Volcano(v) => Some(v),
            _ => None,
        }
    }

    fn vs_ref(&self, id: u32) -> Option<&VolcanoState> {
        match &self.instances.get(&id)?.state {
            InstanceState::Volcano(v) => Some(v),
            _ => None,
        }
    }

    fn vt(&mut self) -> VolcanoTuning {
        if self.volcano_tuning.is_none() {
            let h = self.host.tuning(super::tuning_instance::VOLCANO);
            self.volcano_tuning = Some(VolcanoTuning::from_holder(h.as_deref()));
        }
        self.volcano_tuning.clone().unwrap_or_default()
    }

    /// `AttemptDisasterStart(x, z)` (§10), world units: only when no volcano
    /// exists.
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
            InstanceState::Volcano(Box::new(VolcanoState {
                position: (x as f32, z as f32),
                ..VolcanoState::default()
            })),
        );
        self.add_instance_to_list(kind::VOLCANO, id);
        self.volcano_init(id);
        self.volcano_enter(id, 3);
        Some(id)
    }

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
        self.vt();
        self.register_tick(id, TICK_PRIORITY);
        self.volcano_enter(id, 1);
        if let Some(i) = self.instances.get_mut(&id) {
            i.phase = 1;
        }
        self.register_location(id, cell.0, cell.1, 0, 0, false);
        self.disaster_has_begun(id);
    }

    fn volcano_enter(&mut self, id: u32, state: u32) {
        let tuning = self.vt();
        let host = self.host.clone();
        let Some(v) = self.vs(id) else { return };
        v.state = state;
        let (x, z) = v.position;
        match state {
            3 if !v.deforming => {
                v.deforming = true;
                host.volcano_start_deformation(id, x, z);
            }
            5 => {
                v.lava_us = 0;
                v.lava_phases += 1;
                if tuning.lava_flows {
                    host.volcano_start_lava(id, x, z, tuning.vent_radius);
                }
            }
            8 => {
                if v.deforming {
                    v.deforming = false;
                    host.volcano_end_deformation(id);
                }
                self.unregister_locations(id);
                if let Some(i) = self.instances.get_mut(&id) {
                    i.phase = 3;
                }
                self.disaster_has_ended();
                self.instance_finish(id);
            }
            _ => {}
        }
    }

    pub(crate) fn volcano_tick(&mut self, id: u32, us: u32) {
        let tuning = self.vt();
        let host = self.host.clone();
        let Some(v) = self.vs(id) else { return };
        match v.state {
            3 => {
                if v.end_requested {
                    self.volcano_enter(id, 8);
                } else {
                    self.volcano_enter(id, 5);
                }
            }
            5 => {
                v.lava_us += u64::from(us);
                let t = v.lava_us;
                let end = v.end_requested;
                let (p, s) = tuning.rates(t);
                if tuning.lava_flows {
                    host.volcano_lava_rates(id, p, s);
                }
                if t as f64 >= f64::from(tuning.lava_seconds) * 1e6 {
                    if tuning.lava_flows {
                        host.volcano_end_lava(id);
                    }
                    self.volcano_enter(id, if end { 8 } else { 3 });
                }
            }
            _ => {}
        }
    }

    pub(crate) fn volcano_request_end(&mut self, id: u32) -> bool {
        match self.vs(id) {
            Some(v) => {
                v.end_requested = true;
                true
            }
            None => false,
        }
    }

    pub(crate) fn volcano_snapshot(&self, id: u32) -> Option<VolcanoSnapshot> {
        let i = self.instances.get(&id)?;
        let v = self.vs_ref(id)?;
        Some(VolcanoSnapshot {
            id,
            phase: i.phase,
            state: v.state,
            position: v.position,
            cell: v.cell,
            lava_us: v.lava_us,
            end_requested: v.end_requested,
            lava_phases: v.lava_phases,
        })
    }
}
