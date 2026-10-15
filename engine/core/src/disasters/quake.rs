// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The earthquake (§7, short form). The fault line is ours (§7 allows a
//! different one with the same brush tuning): a straight line from the
//! epicentre in a direction drawn from the earthquake's generator
//! (`RandomDoubleRangeUniform(0, 2π)`), as long as the furthest point of the
//! two distance curves [choice, Q21]. The curviness and the slope variation
//! are read and kept, not used.

use super::system::{Core, InstanceState};
use super::{instance_class, kind, CellRect, TICK_PRIORITY};
use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;

/// Tuning properties of the "Earthquake" exemplar (§7).
pub mod quake_prop {
    pub const STRENGTH: u32 = 0x4A08_C500;
    pub const WIDTH: u32 = 0x4A08_C501;
    pub const STRENGTH_BY_DISTANCE: u32 = 0x4A08_C502;
    pub const WIDTH_BY_DISTANCE: u32 = 0x4A08_C503;
    pub const EFFECT_SPACING: u32 = 0x4A08_C504;
    pub const CURVINESS: u32 = 0x4A08_C505;
    pub const MAX_SLOPE_VARIATION: u32 = 0x4A08_C506;
    pub const SPEED: u32 = 0x4A08_C507;
    pub const SHAKER_PERIOD: u32 = 0x4A08_C508;
    pub const REPULSOR_SPACING: u32 = 0x4A08_C509;
}

/// The values of §7 (also the defaults when a property is missing
/// [choice, Q20]).
pub const SHIPPED_STRENGTH: [(f32, f32); 2] = [(0.0, 0.1), (1.0, 1.0)];
pub const SHIPPED_WIDTH: [(f32, f32); 2] = [(0.0, 0.5), (1.0, 1.0)];
pub const SHIPPED_STRENGTH_BY_DISTANCE: [(f32, f32); 5] =
    [(0.0, 1.0), (200.0, 1.0), (800.0, 0.8), (1200.0, 0.3), (2048.0, 0.0)];
pub const SHIPPED_WIDTH_BY_DISTANCE: [(f32, f32); 5] =
    [(0.0, 1.0), (500.0, 1.0), (900.0, 0.82), (1800.0, 0.2), (3000.0, 0.0)];
pub const SHIPPED_EFFECT_SPACING: f32 = 35.0;
pub const SHIPPED_CURVINESS: f32 = 300.0;
pub const SHIPPED_MAX_SLOPE_DEGREES: f32 = 4.0;
pub const SHIPPED_SPEED: f32 = 400.0;
pub const SHIPPED_SHAKER_PERIOD_US: u32 = 10_000;
pub const SHIPPED_REPULSOR_SPACING: f32 = 100.0;

/// The brush step (§7: 8 m), the brushes (terrain slot 57), the end
/// thresholds and the settle time.
pub const STEP_M: f32 = 8.0;
pub const BRUSH_A: u32 = 0x11;
pub const BRUSH_B: u32 = 0x12;
pub const MIN_STRENGTH: f32 = 0.001;
pub const MIN_WIDTH: f32 = 0.002;
pub const SETTLE_US: u64 = 3_000_000;
/// The rumble's intensity growth per µs (§7 state 1).
pub const RUMBLE_PER_US: f64 = 1e-7;

#[derive(Clone, Debug, PartialEq)]
pub struct QuakeTuning {
    pub strength: ResponseCurve,
    pub width: ResponseCurve,
    pub strength_by_distance: ResponseCurve,
    pub width_by_distance: ResponseCurve,
    pub effect_spacing: f32,
    pub curviness: f32,
    /// Radians.
    pub max_slope_variation: f32,
    /// Metres per second.
    pub speed: f32,
    pub shaker_period_us: u32,
    pub repulsor_spacing: f32,
}

impl Default for QuakeTuning {
    fn default() -> QuakeTuning {
        QuakeTuning::from_holder(None)
    }
}

impl QuakeTuning {
    pub fn from_holder(h: Option<&PropertyHolder>) -> QuakeTuning {
        let curve = |id, d: &[(f32, f32)]| {
            h.and_then(|h| crate::props::curve(h, id))
                .unwrap_or_else(|| ResponseCurve::from_points(d))
        };
        let f = |id, d: f32| h.and_then(|h| crate::props::f32_of(h, id)).unwrap_or(d);
        QuakeTuning {
            strength: curve(quake_prop::STRENGTH, &SHIPPED_STRENGTH),
            width: curve(quake_prop::WIDTH, &SHIPPED_WIDTH),
            strength_by_distance: curve(quake_prop::STRENGTH_BY_DISTANCE, &SHIPPED_STRENGTH_BY_DISTANCE),
            width_by_distance: curve(quake_prop::WIDTH_BY_DISTANCE, &SHIPPED_WIDTH_BY_DISTANCE),
            effect_spacing: f(quake_prop::EFFECT_SPACING, SHIPPED_EFFECT_SPACING),
            curviness: f(quake_prop::CURVINESS, SHIPPED_CURVINESS),
            max_slope_variation: f(quake_prop::MAX_SLOPE_VARIATION, SHIPPED_MAX_SLOPE_DEGREES).to_radians(),
            speed: f(quake_prop::SPEED, SHIPPED_SPEED),
            shaker_period_us: h
                .and_then(|h| crate::props::u32_of(h, quake_prop::SHAKER_PERIOD))
                .unwrap_or(SHIPPED_SHAKER_PERIOD_US),
            repulsor_spacing: f(quake_prop::REPULSOR_SPACING, SHIPPED_REPULSOR_SPACING),
        }
    }

    /// The line length: the furthest point of the two distance curves.
    pub fn line_length(&self) -> f32 {
        self.strength_by_distance
            .points()
            .iter()
            .chain(self.width_by_distance.points().iter())
            .map(|p| p.0)
            .fold(0.0f32, f32::max)
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct QuakeState {
    pub state: u32,
    pub epicentre: (f32, f32),
    pub cell: (i32, i32),
    pub heading: (f32, f32),
    pub length: f32,
    pub clock_us: u64,
    pub shaker_us: u64,
    pub intensity: f32,
    pub base_strength: f32,
    pub base_width: f32,
    pub travelled: f32,
    pub budget_m: f64,
    pub brushes: u32,
}

/// An earthquake's observable state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuakeSnapshot {
    pub id: u32,
    pub phase: u32,
    pub state: u32,
    pub epicentre: (f32, f32),
    pub cell: (i32, i32),
    pub heading: (f32, f32),
    pub intensity: f32,
    pub travelled: f32,
    pub clock_us: u64,
    pub brushes: u32,
}

impl Core {
    fn qs(&mut self, id: u32) -> Option<&mut QuakeState> {
        match &mut self.instances.get_mut(&id)?.state {
            InstanceState::Quake(q) => Some(q),
            _ => None,
        }
    }

    fn qs_ref(&self, id: u32) -> Option<&QuakeState> {
        match &self.instances.get(&id)?.state {
            InstanceState::Quake(q) => Some(q),
            _ => None,
        }
    }

    fn qt(&mut self) -> QuakeTuning {
        if self.quake_tuning.is_none() {
            let h = self.host.tuning(super::tuning_instance::EARTHQUAKE);
            self.quake_tuning = Some(QuakeTuning::from_holder(h.as_deref()));
        }
        self.quake_tuning.clone().unwrap_or_default()
    }

    /// `AttemptDisasterStart(x, z)` (§7), world units: only when no
    /// earthquake instance exists.
    pub(crate) fn quake_attempt_start(&mut self, x: i32, z: i32) -> Option<u32> {
        if self.managers.get(&kind::EARTHQUAKE).is_some_and(|m| !m.instances.is_empty()) {
            return None;
        }
        let id = self.new_instance(
            kind::EARTHQUAKE,
            instance_class::EARTHQUAKE,
            InstanceState::Quake(Box::default()),
        );
        self.add_instance_to_list(kind::EARTHQUAKE, id);
        self.quake_init(id, x as f32, z as f32);
        self.quake_enter(id, 2);
        Some(id)
    }

    fn quake_init(&mut self, id: u32, x: f32, z: f32) {
        let tuning = self.qt();
        let host = self.host.clone();
        let (sx, sz) = self.city_world_size();
        let x = x.clamp(0.0, (sx - 1.0).max(0.0));
        let z = z.clamp(0.0, (sz - 1.0).max(0.0));
        let theta = self.rng_quake.range_f64(0.0, std::f64::consts::TAU);
        let cell = self.world_to_cell(x, z);
        if let Some(q) = self.qs(id) {
            q.epicentre = (x, z);
            q.cell = cell;
            q.heading = (theta.cos() as f32, theta.sin() as f32);
            q.length = tuning.line_length();
        }
        if let Some(i) = self.instances.get_mut(&id) {
            i.phase = 1;
        }
        self.register_tick(id, TICK_PRIORITY);
        host.play_effect_at("QuakeStart", x, z);
        self.quake_enter(id, 1);
        self.register_location(id, cell.0, cell.1, 0, 0, false);
        self.disaster_has_begun(id);
    }

    fn quake_point(q: &QuakeState, d: f32) -> (f32, f32) {
        (q.epicentre.0 + q.heading.0 * d, q.epicentre.1 + q.heading.1 * d)
    }

    fn quake_enter(&mut self, id: u32, state: u32) {
        let tuning = self.qt();
        let host = self.host.clone();
        let Some(q) = self.qs(id) else { return };
        q.state = state;
        q.clock_us = 0;
        match state {
            1 => {
                let q = q.clone();
                let spacing = tuning.repulsor_spacing;
                if spacing > 0.0 {
                    let mut d = 0.0f32;
                    while d <= q.length {
                        let (px, pz) = Self::quake_point(&q, d);
                        let (cx, cz) = self.world_to_cell(px, pz);
                        if self.inside_city(cx, cz) {
                            host.place_repulsor(px, pz);
                        }
                        d += spacing;
                    }
                }
            }
            2 => {
                q.base_strength = tuning.strength.eval(q.intensity);
                q.base_width = tuning.width.eval(q.intensity);
                q.travelled = 0.0;
                q.budget_m = 0.0;
            }
            3 => host.end_long_terrain_operation(),
            4 => {
                host.stop_terrain_texture_updates();
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

    pub(crate) fn quake_tick(&mut self, id: u32, us: u32) {
        let tuning = self.qt();
        let host = self.host.clone();
        let Some(q) = self.qs(id) else { return };
        match q.state {
            1 => {
                q.clock_us += u64::from(us);
                q.intensity = (f64::from(q.intensity) + f64::from(us) * RUMBLE_PER_US).min(1.0) as f32;
                q.shaker_us += u64::from(us);
                let period = u64::from(tuning.shaker_period_us.max(1));
                let (ex, ez) = q.epicentre;
                let mut shakes = 0;
                while q.shaker_us >= period {
                    q.shaker_us -= period;
                    shakes += 1;
                }
                // The rumble hands over to the deformation at full
                // intensity [choice, Q22].
                let done = q.intensity >= 1.0;
                for _ in 0..shakes {
                    host.play_effect_at("QuakerShaker", ex, ez);
                }
                if done {
                    self.quake_enter(id, 2);
                }
            }
            2 => {
                q.clock_us += u64::from(us);
                q.budget_m += f64::from(tuning.speed) / 1e6 * f64::from(us);
                let mut brushes = Vec::new();
                let mut end = false;
                while q.budget_m >= f64::from(STEP_M) {
                    q.budget_m -= f64::from(STEP_M);
                    q.travelled += STEP_M;
                    let s = q.base_strength * tuning.strength_by_distance.eval(q.travelled);
                    let w = q.base_width * tuning.width_by_distance.eval(q.travelled);
                    if q.travelled > q.length || s <= MIN_STRENGTH || w < MIN_WIDTH {
                        end = true;
                        break;
                    }
                    let (px, pz) = Self::quake_point(q, q.travelled);
                    let (nx, nz) = (-q.heading.1, q.heading.0);
                    brushes.push((BRUSH_A, px + nx * w, pz + nz * w, s, w));
                    brushes.push((BRUSH_B, px - nx * w, pz - nz * w, s, w));
                }
                q.brushes += brushes.len() as u32;
                let mut touched: Option<CellRect> = None;
                for (b, x, z, s, w) in brushes {
                    host.apply_terrain_brush(b, x, z, s, w);
                    let (cx, cz) = self.world_to_cell(x, z);
                    touched = Some(match touched {
                        None => CellRect::cell(cx, cz),
                        Some(r) => CellRect {
                            x0: r.x0.min(cx),
                            z0: r.z0.min(cz),
                            x1: r.x1.max(cx),
                            z1: r.z1.max(cz),
                        },
                    });
                }
                if let Some(r) = touched {
                    let r = self.clamp_rect(r);
                    host.redraw_terrain(r);
                }
                if end {
                    self.quake_enter(id, 3);
                }
            }
            3 => {
                q.clock_us += u64::from(us);
                if q.clock_us >= SETTLE_US {
                    self.quake_enter(id, 4);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn quake_snapshot(&self, id: u32) -> Option<QuakeSnapshot> {
        let i = self.instances.get(&id)?;
        let q = self.qs_ref(id)?;
        Some(QuakeSnapshot {
            id,
            phase: i.phase,
            state: q.state,
            epicentre: q.epicentre,
            cell: q.cell,
            heading: q.heading,
            intensity: q.intensity,
            travelled: q.travelled,
            clock_us: q.clock_us,
            brushes: q.brushes,
        })
    }
}
