// SPDX-License-Identifier: GPL-3.0-or-later
//! Layer update/freshness/loop/easing algorithms, ambience §8.5–8.7.
use super::{
    distributed_positions, FilterHost, GroupLayer, LayerFilters, LayerGlobals, LayerTuning, Position, Rectangle,
};
use crate::audio::instance_id::InstanceIdCounter;
use crate::x87::ftol32;

pub trait LayerEventHost {
    fn create_event(&mut self, event: u32, iid: u32) -> Option<u64>;
    fn set_data(&mut self, event: u64, word: usize, value: u32);
    fn set_void1(&mut self, event: u64, player: u64);
    fn post_event(&mut self, event: u64);
    fn release_event(&mut self, event: u64);
    fn division_position(&mut self, player: u64, col: u32, row: u32, position: [i32; 3]);
    fn division_enabled(&mut self, player: u64, col: u32, row: u32, enabled: bool);
}

fn send(host: &mut impl LayerEventHost, id: u32, player: Option<u64>, words: &[u32]) -> bool {
    let Some(event) = host.create_event(id, 0x5B52_3BA4) else {
        return false;
    };
    if let Some(player) = player {
        host.set_void1(event, player);
    }
    for (index, value) in words.iter().enumerate() {
        host.set_data(event, index + 1, *value);
    }
    host.post_event(event);
    host.release_event(event);
    true
}

#[derive(Clone, Debug)]
pub struct ScoreEntry {
    pub playing: bool,
    pub ready: bool,
    pub last_percent: i32,
    pub last_volume: u32,
    pub player: u64,
}

#[derive(Clone, Debug)]
pub struct LoopEntry {
    pub sound: u32,
    pub instance: u32,
    pub last_position: Position,
    pub last_volume: i32,
}

#[derive(Clone, Debug)]
pub struct Ease {
    pub target_position: Position,
    pub current_position: Position,
    pub target_scalar: i32,
    pub current_scalar: i32,
    pub stepped: bool,
    pub vector_rate: f32,
    pub scalar_rate: f32,
}

impl Ease {
    pub fn set_position(&mut self, target: Position, snap: bool) {
        self.target_position = target;
        if snap || !self.stepped {
            self.current_position = target;
        }
    }
    pub fn set_scalar(&mut self, target: i32, snap: bool) {
        self.target_scalar = target;
        if snap || !self.stepped {
            self.current_scalar = target;
        }
    }
    pub fn step(&mut self, milliseconds: f32) {
        let dx = f64::from(self.target_position[0]) - f64::from(self.current_position[0]);
        let dy = self.target_position[1] - self.current_position[1];
        let dz = f64::from(self.target_position[2]) - f64::from(self.current_position[2]);
        let length = ((dz * dz + dx * dx) + f64::from(dy) * f64::from(dy)).sqrt();
        let length32 = length as f32;
        if length32 != 0.0 {
            let inverse = 1.0 / length;
            let nx = (dx * inverse) as f32;
            let ny = (f64::from(dy) * inverse) as f32;
            let nz = dz * inverse;
            let step = milliseconds * self.vector_rate;
            let movement = if length32 < step { length32 } else { step };
            self.current_position[0] =
                (f64::from(self.current_position[0]) + f64::from(nx) * f64::from(movement)) as f32;
            self.current_position[1] += movement * ny;
            self.current_position[2] += (nz * f64::from(movement)) as f32;
        }
        let difference = self.target_scalar.wrapping_sub(self.current_scalar);
        let absolute = f64::from(difference).abs() as f32;
        if absolute != 0.0 {
            let step = milliseconds * self.scalar_rate;
            let movement = if absolute < step { absolute } else { step };
            self.current_scalar = self.current_scalar.wrapping_add(ftol32(
                f64::from(difference) / f64::from(absolute) * f64::from(movement),
            ));
        }
        self.stepped = true;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerUpdateError {
    ModeSixNativeFault,
}

/// State after the unresolved constructor/loading stage. There is deliberately
/// no native `new` that invents those defaults (AA14). All state is replayable.
#[derive(Debug)]
pub struct AmbienceLayer {
    pub tuning: LayerTuning,
    pub filters: LayerFilters,
    pub occupant_groups: GroupLayer,
    pub cell_groups: GroupLayer,
    pub intersection: GroupLayer,
    pub rectangle: Rectangle,
    /// occupants accepted, cells accepted, occupants rejected, cells rejected.
    pub counters: [u32; 4],
    pub can_handle_this_update: bool,
    pub paused: bool,
    pub group_count: u32,
    pub score: f32,
    pub samples: Vec<Position>,
    pub listener_position: Position,
    pub listener_vector: Position,
    pub average: Position,
    pub score_volume: u32,
    pub play_percent: i32,
    pub loop_volume: i32,
    pub scores: Vec<ScoreEntry>,
    pub loops: Vec<LoopEntry>,
    pub ease: Ease,
}

impl AmbienceLayer {
    pub fn create_group_layers(&mut self) {
        self.occupant_groups = GroupLayer::new(self.tuning.occupant_radius);
        self.cell_groups = GroupLayer::new(self.tuning.cell_radius);
        self.intersection = GroupLayer::new(if self.tuning.group_mode == 2 {
            self.tuning.cell_radius
        } else {
            self.tuning.occupant_radius
        });
    }
    pub fn begin_update(&mut self, rectangle: Rectangle) {
        self.rectangle = rectangle;
        self.occupant_groups.begin();
        if self.tuning.group_mode != 1 {
            self.cell_groups.begin();
        }
        self.counters = [0; 4];
        self.can_handle_this_update = false;
        self.filters.begin();
    }
    pub fn end_update(&mut self) {
        self.occupant_groups.end();
        if self.tuning.group_mode != 1 {
            self.cell_groups.end();
        }
    }
    pub fn handle_cell(
        &mut self,
        x: i32,
        z: i32,
        enabled: bool,
        globals: &mut LayerGlobals,
        host: &mut impl FilterHost,
    ) {
        let groups = if self.tuning.group_mode == 1 {
            &mut self.occupant_groups
        } else {
            &mut self.cell_groups
        };
        self.filters
            .handle_cell(x, z, enabled, groups, &mut self.counters, globals, host);
    }
    pub fn handle_occupant(&mut self, occupant: Option<u64>, enabled: bool, host: &mut impl FilterHost) {
        self.filters
            .handle_occupant(occupant, enabled, &mut self.occupant_groups, &mut self.counters, host);
    }

    #[allow(clippy::too_many_arguments)] // Separate foreign filtering/event domains and shared ID state.
    pub fn update(
        &mut self,
        elapsed: u32,
        p: Position,
        q: Position,
        input: LayerUpdateInput,
        filters: &mut impl FilterHost,
        events: &mut impl LayerEventHost,
        ids: &mut InstanceIdCounter,
    ) -> Result<(), LayerUpdateError> {
        self.listener_position = p;
        self.listener_vector = q;
        self.can_handle_this_update = self.filters.can_handle(input.sound_enabled, filters);
        if self.can_handle_this_update {
            let groups = match self.tuning.group_mode {
                1 => Some(&self.occupant_groups),
                2 | 3 => {
                    self.intersection
                        .intersect(&self.occupant_groups, &self.cell_groups, self.tuning.group_mode == 2);
                    Some(&self.intersection)
                }
                _ => None,
            };
            let mut positions = Vec::new();
            let mut peaks = 0u32;
            if let Some(groups) = groups {
                for group in &groups.groups {
                    positions.push(group.center);
                    peaks = peaks.wrapping_add(group.peak);
                }
                self.group_count = positions.len() as u32;
            }
            self.score = match self.tuning.score_mode {
                1 => peaks as f32,
                2 => input.population.unwrap_or(0) as f32,
                3 => f32::MAX,
                4 => input.zoom as f32,
                5 => {
                    if self.group_count == 0 {
                        0.0
                    } else {
                        (f64::from(peaks) / f64::from(self.group_count)) as f32
                    }
                }
                // Explicit native fault, not an invented tracked-occupant count.
                6 => return Err(LayerUpdateError::ModeSixNativeFault),
                _ => self.score,
            };
            if self.group_count != 0 {
                self.samples = distributed_positions(&positions, self.rectangle);
                self.group_count = self.samples.len() as u32;
                if self.group_count != 0 {
                    self.average = weighted_average(&self.samples, p, q, self.tuning.sharpen);
                }
            }
        }
        self.update_scores(input.zoom_height_five, events);
        self.update_loops(elapsed, events, ids);
        Ok(())
    }

    pub fn update_freshness_parameters(&mut self) {
        if !self.can_handle_this_update {
            self.score_volume = 0;
            self.play_percent = 0;
            return;
        }
        self.score_volume = (ftol32(f64::from(self.tuning.curves[1].eval(self.score))) as u32)
            .wrapping_mul(self.tuning.master_volume)
            >> 10;
        let percent = ftol32(f64::from(self.tuning.curves[0].eval(self.score)));
        self.play_percent = match self.tuning.preference_mode {
            0 => percent.wrapping_mul(50) / 100,
            1 => percent.wrapping_mul(75) / 100,
            _ => percent,
        };
    }

    pub fn update_scores(&mut self, zoom_height_five: f32, host: &mut impl LayerEventHost) {
        self.update_freshness_parameters();
        let on = self.score_volume != 0 && self.play_percent != 0 && !self.paused;
        for score in &mut self.scores {
            if on && !score.playing && score.ready {
                score.last_percent = 0;
                score.last_volume = 0;
                let distance =
                    (f64::from(self.tuning.distance_percent) * f64::from(0.01f32) * f64::from(zoom_height_five)) as f32;
                send(
                    host,
                    0xEA2D_827C,
                    Some(score.player),
                    &[
                        ftol32(f64::from(distance)) as u32,
                        ftol32(f64::from(distance) * 16.0) as u32,
                    ],
                );
                if send(
                    host,
                    0xEA2C_0390,
                    Some(score.player),
                    &[u32::from(self.tuning.group_mode == 4)],
                ) {
                    score.playing = true;
                }
            }
            if !on && score.playing && send(host, 0x2A2C_0393, Some(score.player), &[]) {
                score.playing = false;
                continue;
            }
            if score.playing {
                if self.tuning.group_mode != 4 {
                    for i in 0..9 {
                        let col = i as u32 % 3;
                        let row = i as u32 / 3;
                        if i < self.group_count as usize {
                            host.division_position(
                                score.player,
                                col,
                                row,
                                self.samples[i].map(|x| ftol32(f64::from(x))),
                            );
                            host.division_enabled(score.player, col, row, true);
                        } else {
                            host.division_enabled(score.player, col, row, false);
                        }
                    }
                }
                if self.score_volume != score.last_volume {
                    score.last_volume = self.score_volume;
                    send(host, 0x4A3D_A208, Some(score.player), &[self.score_volume]);
                }
                if self.play_percent != score.last_percent {
                    send(host, 0x0A41_4C5C, Some(score.player), &[self.play_percent as u32]);
                    score.last_percent = self.play_percent;
                }
            }
        }
    }

    pub fn update_loops(&mut self, elapsed: u32, host: &mut impl LayerEventHost, ids: &mut InstanceIdCounter) {
        self.loop_volume = if self.can_handle_this_update && self.score != 0.0 {
            ftol32(f64::from(self.tuning.curves[2].eval(self.score)))
        } else {
            0
        };
        for entry in &mut self.loops {
            if self.loop_volume == 0 {
                if entry.instance != 0 {
                    send(host, 0x6A45_1908, None, &[entry.instance, 0, 0, 0]);
                    entry.instance = 0;
                }
            } else if entry.instance == 0 {
                self.ease.set_position(self.average, true);
                self.ease.set_scalar(self.loop_volume, true);
                entry.last_volume = 0;
                entry.last_position = [0.0; 3];
                entry.instance = ids.generate();
                send(
                    host,
                    0xEA45_1903,
                    None,
                    &[entry.sound, entry.instance, 1, self.loop_volume as u32],
                );
            }
            self.ease.set_position(self.average, false);
            self.average = self.ease.current_position;
            self.ease.set_scalar(self.loop_volume, false);
            self.loop_volume = self.ease.current_scalar;
            if self.loop_volume != entry.last_volume {
                entry.last_volume = self.loop_volume;
                if entry.instance != 0 {
                    send(
                        host,
                        0x4A45_1915,
                        None,
                        &[entry.instance, self.loop_volume as u32, 0, 0],
                    );
                }
            }
            if entry.instance != 0
                && entry
                    .last_position
                    .iter()
                    .zip(self.average)
                    .any(|(last, current)| (f64::from(*last) - f64::from(current)).abs() >= f64::from(0.0001f32))
            {
                let position = self.average.map(|x| ftol32(f64::from(x)) as u32);
                send(
                    host,
                    0xCA45_190F,
                    None,
                    &[entry.instance, position[0], position[1], position[2]],
                );
                entry.last_position = self.average;
            }
        }
        self.ease.step(elapsed as f32);
    }

    pub fn pause(
        &mut self,
        flag: bool,
        zoom_height_five: f32,
        host: &mut impl LayerEventHost,
        ids: &mut InstanceIdCounter,
    ) {
        self.paused = flag;
        self.update_scores(zoom_height_five, host);
        self.update_loops(0x7FFF_FFFF, host, ids);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LayerUpdateInput {
    pub sound_enabled: bool,
    pub population: Option<u32>,
    pub zoom: u32,
    pub zoom_height_five: f32,
}

/// Literal §8.5 operation order; no singularity sanitization. Positions on the
/// view line may produce infinities/NaNs, as the spec explicitly permits.
pub fn weighted_average(samples: &[Position], p: Position, q: Position, sharpen: f32) -> Position {
    let d: Position = std::array::from_fn(|i| p[i] - q[i]);
    let norm = ((f64::from(d[2]).powi(2) + f64::from(d[1]).powi(2)) + f64::from(d[0]).powi(2)).sqrt();
    let inverse = (1.0 / norm) as f32;
    let n = d.map(|x| x * inverse);
    let mut weight_sum = 0.0f64;
    let mut sum = [0.0f64; 3];
    for point in samples {
        let v: Position = std::array::from_fn(|i| point[i] - q[i]);
        let length = ((f64::from(v[2]).powi(2) + f64::from(v[1]).powi(2)) + f64::from(v[0]).powi(2)).sqrt();
        let k = 1.0 / length;
        let a = (((f64::from(v[2]) * k) * f64::from(n[2])
            + (f64::from(v[1]) * k) * f64::from(n[1])
            + (f64::from(v[0]) * k) * f64::from(n[0]))
        .abs()) as f32;
        let weight = f64::from(a) * (1.0 / (f64::from(sharpen) - f64::from(a)));
        weight_sum += weight;
        for i in 0..3 {
            sum[i] += weight * f64::from(point[i]);
        }
    }
    let inverse_weight = (1.0 / weight_sum) as f32;
    sum.map(|x| (x * f64::from(inverse_weight)) as f32)
}
