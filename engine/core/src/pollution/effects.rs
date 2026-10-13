// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The visual effects and worst tracts (`UpdatePollutionVisualEffect`,
//! §4.10).

use super::simulator::{fmul_i, PollutionState, WorstTract, EFFECT_KINDS, G_RAD_SRC};
use super::{effect, PollutionSim};

/// Strictly greater replaces the stored worst block.
fn worse(w: &mut WorstTract, x: u32, z: u32, v: i32) {
    if v > w.value {
        *w = WorstTract { x, z, value: v };
    }
}

impl PollutionSim {
    /// `UpdatePollutionVisualEffect` (§4.10): monthly, on PostCityInit, when
    /// `effectsVisible` changes and after `AddRadiationToCity`.
    pub fn update_visual_effects(&self) {
        let t = self.tuning();
        let h = self.host.clone();
        let mut fx = self.fx.borrow().clone();
        let (cw, cwz) = h.cell_widths();
        self.edit(|s: &mut PollutionState| {
            self.with_crt(|crt| s.effect_iterator.reset(true, crt));
            s.worst_air.value = 0;
            s.worst_water.value = 0;
            s.worst_garbage.value = 0;
            while let Some((bx, bz)) = s.effect_iterator.next_cell() {
                let (ox, oz) = (bx.wrapping_mul(8), bz.wrapping_mul(8));
                let mut sums = [0i32; 4]; // water, air, radiation, garbage
                for i in 0..8 {
                    for j in 0..8 {
                        let (x, z) = (ox.wrapping_add(i), oz.wrapping_add(j));
                        sums[0] = sums[0].wrapping_add(s.water_value(x, z));
                        sums[1] = sums[1].wrapping_add(s.air_value(x, z));
                        sums[2] = sums[2].wrapping_add(i32::from(s.cell(G_RAD_SRC, x, z)));
                        sums[3] = sums[3].wrapping_add(s.garbage_value(x, z) & 0x7FFF);
                    }
                }
                let avg = sums.map(|v| ((v as u32) >> 6) as i32);
                let (cx, cz) = (ox.wrapping_add(4) as u32, oz.wrapping_add(4) as u32);
                worse(&mut s.worst_air, cx, cz, avg[1]);
                worse(&mut s.worst_water, cx, cz, avg[0]);
                worse(&mut s.worst_garbage, cx, cz, avg[3]);
                let idx = if bx >= 0 && bz >= 0 && bx < s.blocks_x && bz < s.blocks_z {
                    Some((bz as usize) * (s.blocks_x as usize) + bx as usize)
                } else {
                    None
                };
                let Some(idx) = idx else { continue };
                for k in [effect::WATER, effect::AIR, effect::RADIATION, effect::GARBAGE] {
                    let strength = t.effect_curves[k].eval(fmul_i(f64::from(avg[k]), 1.0f32 / 32767.0));
                    let shown = strength > effect::THRESHOLD && (k == effect::WATER || s.effects_visible);
                    let Some(slot) = s.effects[k].get_mut(idx) else {
                        continue;
                    };
                    let param = if k == effect::WATER { 3 } else { 2 };
                    match (*slot, shown) {
                        (None, true) => {
                            let Some(e) = h.create_visual_effect(effect::ID, EFFECT_KINDS[k]) else {
                                continue;
                            };
                            let (x0, z0) = (ox as f32 * cw, oz as f32 * cwz);
                            let pos = if k == effect::WATER {
                                let ux = fx.range_f64(f64::from(-2.0 * cw), f64::from(2.0 * cw));
                                let uz = fx.range_f64(f64::from(-2.0 * cwz), f64::from(2.0 * cwz));
                                [(f64::from(x0) + ux) as f32, 0.0, (f64::from(z0) + uz) as f32]
                            } else {
                                let (px, pz) = (x0 + 4.0 * cw, z0 + 4.0 * cwz);
                                let mut y = h.altitude(px, pz);
                                if k == effect::RADIATION {
                                    y += 3.0;
                                }
                                [px, y, pz]
                            };
                            h.set_effect_position(e, pos);
                            h.set_effect_param(e, param, strength);
                            h.start_effect(e, 0);
                            *slot = Some(e);
                        }
                        (Some(e), true) => h.set_effect_param(e, param, strength),
                        (Some(e), false) => {
                            if h.is_effect_started(e) {
                                h.stop_effect(e, 0);
                            }
                            h.release(e);
                            *slot = None;
                        }
                        (None, false) => {}
                    }
                }
            }
        });
        *self.fx.borrow_mut() = fx;
    }
}
