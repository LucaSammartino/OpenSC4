// SPDX-License-Identifier: GPL-3.0-or-later
//! Confirmed score selection/lifetime (§6.1) and cell playback (§6.2).
use super::fresh::{CellHost, FreshCellPlayer, FreshPlayer};
use std::collections::BTreeSet;

#[derive(Debug, PartialEq, Eq)]
pub enum FreshLifecycle {
    FadeStart(i32),
    FadeEnd,
    TimerStop,
    ScoreDelete,
    PlayerDelete(u64),
    ControllerRelease,
    TimerRestart,
}
#[derive(Debug, Clone, Copy)]
pub struct VolumeFade {
    pub step: i32,
    pub target: i32,
    pub stop: bool,
    pub period: i32,
}
pub trait FreshHost: CellHost {
    fn c_rand(&mut self) -> i32;
    fn reseed_private(&mut self, seed: u32);
    fn random_below(&mut self, upper: u32) -> u32;
}
pub trait PathScoreInput {
    /// Returns the allocated score even when Load(path) returned false (§6.1.1).
    fn load_path(&mut self, path: &str) -> (super::fresh::ScoreData, bool);
}

impl FreshCellPlayer {
    pub fn filename_has_guid(name: &str) -> bool {
        name.as_bytes()
            .windows(8)
            .any(|word| word.iter().all(u8::is_ascii_hexdigit))
    }
    fn name_id(name: &str) -> u32 {
        if !Self::filename_has_guid(name) {
            return 0;
        }
        // SPEC-GAP: SplitToken without a dot yields an empty prefix; safely use ID zero.
        let prefix = name.split_once('.').map(|(prefix, _)| prefix).unwrap_or("");
        let suffix = prefix
            .as_bytes()
            .get(prefix.len().saturating_sub(8)..)
            .unwrap_or_default();
        super::hit::unsigned_number(&format!("0x{}", String::from_utf8_lossy(suffix)))
    }
    fn pitch(&self, host: &mut (impl CellHost + ?Sized)) -> Option<(i32, f64)> {
        let cell = self.cell.as_ref()?;
        let [low, high] = cell.pitch;
        let step = if low != 0 && high != 0 && high >= low {
            let span = high.wrapping_sub(low).wrapping_add(1);
            if span == 0 {
                return None;
            }
            (host.private_draw() as i32).checked_rem(span)?.wrapping_add(low)
        } else {
            0
        };
        Some((step, host.pitch_ratio(step.unsigned_abs() as usize)?))
    }
    fn reset_factor(&mut self, host: &mut (impl CellHost + ?Sized), owner_volume: i32, score_volume: i32) {
        let cell = self.cell.as_ref().expect("bound cell");
        self.random_factor = 1024;
        if cell.random_volume {
            self.random_factor -= ((host.private_draw() as i32).unsigned_abs() % 256) as i32;
        }
        let volume = cell.volume_word;
        self.update_scaled_volume(volume, self.random_factor, owner_volume, score_volume);
    }
    pub fn start(&mut self, host: &mut (impl CellHost + ?Sized), owner_volume: i32, score_volume: i32) -> bool {
        if !host.sound_system_available() || self.dead || self.is_busy() || self.cell.is_none() {
            return false;
        }
        self.stop_flag = false;
        let cell = self.cell.as_ref().expect("bound cell");
        let name = cell.name.clone();
        let id = Self::name_id(&name);
        if Self::filename_has_guid(&name) {
            self.cell.as_mut().expect("bound cell").id = u64::from(id);
        }
        if self.external_controller || self.controller.is_some() {
            self.state = 2;
            self.reset_factor(host, owner_volume, score_volume);
            let Some((_, ratio)) = self.pitch(host) else {
                return false;
            };
            let cell = self.cell.as_ref().expect("bound cell");
            if cell.looping {
                self.counter = cell.loop_length;
            }
            let sound_id = if id != 0 {
                id
            } else {
                host.crc24_filename(name.rsplit(['/', '\\']).next().unwrap_or(&name)) | 0xFF00_0000
            };
            if !host.external_start(
                cell,
                sound_id,
                self.head_relative,
                cell.looping,
                cell.fade_in_selector as f32,
            ) {
                return false;
            }
            host.external_position(cell, self.position.map(|word| word as f32));
            host.external_distances(cell, self.distances.map(|word| word as f32));
            host.external_volume(cell, self.scaled_volume as f32 * (1.0 / 1024.0));
            host.external_pitch(cell, ratio as f32);
            return true;
        }
        let sound = if Self::filename_has_guid(&name) {
            host.sound_by_id(id)
        } else {
            host.sound_by_name(name.rsplit(['/', '\\']).next().unwrap_or(&name))
        };
        let Some(sound) = sound else {
            return false;
        };
        sound.set_integer(0x1C, 1, false);
        if !sound.spatial(
            self.head_relative,
            self.distances[0] as f32,
            self.distances[1] as f32,
            self.position_set.then(|| self.position.map(|word| word as f32)),
        ) {
            return false; // safe rejection of the original unchecked 3D QI
        }
        let cell = self.cell.as_ref().expect("bound cell");
        if cell.fade_in_selector == 0 {
            sound.play(self.scaled_volume as u32, cell.looping, true);
        } else {
            sound.play(0, cell.looping, true);
            sound.fade(cell.fade_out, self.scaled_volume);
        }
        self.source = Some(sound);
        self.state = 2;
        host.register_source_update_if_available(self.source.as_ref().expect("owned sound"), 250);
        self.reset_factor(host, owner_volume, score_volume);
        let sound = self.source.as_ref().expect("owned sound");
        sound.set_volume(self.scaled_volume);
        let cell = self.cell.as_ref().expect("bound cell");
        if cell.looping {
            self.counter = cell.loop_length;
        }
        let Some((step, ratio)) = self.pitch(host) else {
            return false;
        };
        let frequency = f64::from(sound.frequency());
        sound.set_frequency((if step < 0 { frequency / ratio } else { frequency * ratio }).trunc() as i64 as i32);
        true
    }
    pub fn update_countdown(&mut self) {
        self.counter = self.counter.wrapping_sub(1);
        if self.stop_flag {
            if self.counter != 0 {
                self.kill();
            }
        } else if self.counter == 0 {
            self.counter = self.cell.as_ref().map_or(0, |cell| cell.loop_length);
        }
    }
    pub fn update(&mut self, host: &mut (impl CellHost + ?Sized)) {
        let Some(cell) = &self.cell else {
            return;
        };
        if self.state == 2 && cell.looping {
            self.update_countdown();
            return;
        }
        if (self.state == 2 || self.state == 3)
            && (self.external_controller || self.controller.is_some())
            && !host.external_playing(cell)
        {
            host.external_stop(cell, cell.fade_out as i32 as f32);
            self.state = 0;
            return;
        }
        if let Some(sound) = &self.source {
            if self.state == 2 && !sound.is_playing() {
                let _ = self.stop(host);
            } else if self.state == 3 && !sound.is_fading() {
                self.kill();
            }
        }
    }
    pub fn auto_pan(
        &mut self,
        host: &mut (impl CellHost + ?Sized),
        force_random: bool,
        forced_quadrant: Option<u32>,
    ) -> Option<[f32; 3]> {
        let cell = self.cell.as_mut()?;
        if force_random {
            cell.random_pan = true;
        }
        let mut x = f64::from(cell.pan[0].wrapping_sub(512)) / 512.0;
        let mut z = f64::from(cell.pan[1].wrapping_sub(512)) / 512.0;
        if cell.random_pan {
            if x == 0.0 && z == 0.0 {
                let quadrant = forced_quadrant.unwrap_or_else(|| host.private_draw()) & 3;
                (x, z) = [(0.0, 1.0), (0.0, -1.0), (-1.0, 0.0), (1.0, 0.0)][quadrant as usize];
            } else if x == 0.0 || z == 0.0 {
                // SPEC-GAP: the prose gives low-bit selection but not its polarity; use 0 => -1.
                let sign = if host.private_draw() & 1 == 0 { -1.0 } else { 1.0 };
                if x == 0.0 {
                    x = sign;
                } else {
                    z = sign;
                }
            }
        }
        let position = [(x * 100.0) as f32, 0.0, (z * 100.0) as f32];
        self.position = position.map(|word| word as i32);
        self.position_set = true;
        if let Some(sound) = &self.source {
            sound.spatial(
                self.head_relative,
                self.distances[0] as f32,
                self.distances[1] as f32,
                Some(position),
            );
        }
        Some(position)
    }
}

impl FreshPlayer {
    pub fn init_path(&mut self, path: &str, input: &mut dyn PathScoreInput) -> Result<(), super::Unsupported> {
        let split = path.rfind(['/', '\\']).map_or(0, |index| index + 1);
        self.init_directory(&path[..split], &path[split..], input)
    }
    pub fn init_directory(
        &mut self,
        directory: &str,
        filename: &str,
        input: &mut dyn PathScoreInput,
    ) -> Result<(), super::Unsupported> {
        let (score, _loaded) = input.load_path(&format!("{directory}{filename}"));
        self.data_directory = directory.to_owned();
        self.init_loaded_score(score)
    }
    pub fn in_window(&self, index: usize) -> bool {
        let columns = self.dimensions[1];
        columns != 0
            && ((index / columns) as i32 - self.coordinates[0]).abs() <= self.half[0]
            && ((index % columns) as i32 - self.coordinates[1]).abs() <= self.half[1]
    }
    pub fn recount(&mut self) {
        let mut groups = BTreeSet::new();
        let mut zero_groups = 0;
        self.selectable_count = 0;
        for (index, player) in self.cells.iter().enumerate() {
            if !self.in_window(index) {
                continue;
            }
            if let Some(cell) = &player.cell {
                self.selectable_count += 1;
                if cell.group == 0 {
                    zero_groups += 1;
                } else {
                    groups.insert(cell.group);
                }
            }
        }
        self.distinct_groups = groups.len() as i32 + zero_groups;
    }
    fn set_position(&mut self, axis: usize, value: i32) {
        if !self.initialized {
            return;
        }
        let dimension = self.dimensions[axis] as i32;
        if dimension <= 0 {
            return;
        }
        let half = self.half[axis];
        let mut position = dimension.wrapping_mul(value) / 1000;
        let span = half.wrapping_mul(2).wrapping_add(1);
        if self.block_mode && span != 0 {
            position = (position / span).wrapping_mul(span).wrapping_add(half);
        }
        self.coordinates[axis] = position.min(dimension - half - 1).max(half).clamp(0, dimension - 1);
        self.recount();
    }
    pub fn set_x_position(&mut self, value: i32) {
        self.set_position(1, value);
    }
    pub fn set_y_position(&mut self, value: i32) {
        self.set_position(0, value);
    }
    pub fn start_score(&mut self, index: usize, group: i32, c_draw: impl FnOnce() -> i32) -> i32 {
        let Some(player) = self.cells.get(index) else {
            return 0;
        };
        let Some(cell) = player.cell.as_ref() else {
            return 0;
        };
        if player.dead || player.is_busy() || !self.in_window(index) || self.last_subtracted == Some(index) {
            return 0;
        }
        if cell.group != 0
            && self.cells.iter().any(|other| {
                other.is_busy() && !other.stop_flag && other.cell.as_ref().is_some_and(|c| c.group == cell.group)
            })
        {
            return 0;
        }
        if cell.links.iter().any(|link| {
            self.cells
                .get(*link)
                .is_some_and(|other| other.is_busy() && !other.stop_flag)
        }) {
            return 0;
        }
        let cell = self.cells[index].cell.as_mut().expect("tested cell");
        cell.class = cell.class.clamp(0, 20);
        let class = cell.class;
        if class == 0 {
            return 0;
        }
        let matched = group != 0 && group == cell.group;
        if class == 20 {
            return if matched { 101 } else { 100 };
        }
        if matched {
            return 2;
        }
        if self.selectable_count == 0 {
            self.selectable_count = 10;
        }
        const T: [i32; 21] = [
            0, 600, 1500, 3000, 7500, 12000, 15000, 18000, 21000, 25500, 32767, 36000, 45000, 58000, 67500, 90000,
            120000, 210000, 300000, 40000000, 40000000,
        ];
        i32::from(c_draw() < T[class as usize] / self.selectable_count)
    }
    fn scan(&self, start: [usize; 2], window: bool) -> Vec<usize> {
        let [rows, columns] = self.dimensions;
        if rows == 0 || columns == 0 {
            return Vec::new();
        }
        let mut indices = Vec::new();
        for row in 0..rows {
            for column in 0..columns {
                let index = ((start[0] + row) % rows) * columns + (start[1] + column) % columns;
                if !window || self.in_window(index) {
                    indices.push(index);
                }
            }
        }
        indices
    }
    pub fn best_start(&mut self, group: i32, draw: u32, mut c_draw: impl FnMut() -> i32) -> Option<usize> {
        let mut origin = [0usize; 2];
        for (axis, byte) in [(0, (draw >> 8) & 255), (1, draw & 255)] {
            let width = self.half[axis] * 2 + 1;
            if width <= 0 || self.dimensions[axis] == 0 {
                return None;
            }
            origin[axis] = (self.coordinates[axis] + (byte as i32 % width) - self.half[axis])
                .clamp(0, self.dimensions[axis] as i32 - 1) as usize;
        }
        let mut best = None;
        let mut score = 0;
        for index in self.scan(origin, true) {
            let candidate = self.start_score(index, group, &mut c_draw);
            if candidate > score {
                best = Some(index);
                score = candidate;
            }
        }
        best
    }
    pub fn cell_to_subtract(&self, draw: u32, mut next: impl FnMut() -> u32) -> Option<usize> {
        let [rows, columns] = self.dimensions;
        if rows == 0 || columns == 0 {
            return None;
        }
        let order = self.scan(
            [((draw >> 8) & 255) as usize % rows, (draw & 255) as usize % columns],
            false,
        );
        let mut inside = Vec::new();
        for index in order {
            let player = &self.cells[index];
            if !player.is_playing() || player.stop_flag {
                continue;
            }
            if !self.in_window(index) {
                return Some(index);
            }
            if player.cell.as_ref().is_some_and(|c| c.class != 20) {
                inside.push(index);
            }
        }
        if inside.is_empty() {
            None
        } else {
            Some(inside[next() as usize % inside.len()])
        }
    }
    pub fn beat_due(&self, beat: i32) -> i32 {
        if self.tempo == 0 {
            -1
        } else {
            beat.wrapping_sub(self.origin)
                .wrapping_mul(60000)
                .wrapping_div(self.tempo)
        }
    }
    pub fn set_tempo(&mut self, tempo: i32, host: &mut (impl CellHost + ?Sized)) {
        if self.playing && tempo != self.tempo {
            self.tempo = tempo;
            self.next_beat = self.origin.wrapping_add(self.beat_in_bar);
            host.lifecycle(FreshLifecycle::TimerRestart);
        }
    }
    pub fn fade_volume(
        &mut self,
        duration: i32,
        target: i32,
        stop: bool,
        scheduler: Option<&mut dyn super::Scheduler>,
        host: &mut (impl CellHost + ?Sized),
    ) -> bool {
        if !self.initialized || !self.playing {
            return true;
        }
        let step = target.wrapping_sub(self.volume) / 20;
        if step == 0 {
            self.volume = target;
            if stop {
                self.stop(scheduler, host);
            }
            return true;
        }
        let period = self.fade.map_or(duration / 20, |fade| fade.period);
        if self.fade.is_none() {
            host.lifecycle(FreshLifecycle::FadeStart(period));
        }
        self.fade = Some(VolumeFade {
            step,
            target,
            stop,
            period,
        });
        true
    }
    /// Return true asks the caller to perform StopNoLock (including zero-step fades).
    pub fn fade_callback(&mut self) -> bool {
        let Some(fade) = self.fade else {
            return false;
        };
        self.volume = self.volume.wrapping_add(fade.step).clamp(0, 1024);
        if (fade.step < 0 && self.volume <= fade.target) || (fade.step > 0 && self.volume >= fade.target) {
            self.fade = None;
            return fade.step < 0 && fade.stop;
        }
        false
    }
    pub fn on_fade(&mut self, scheduler: Option<&mut dyn super::Scheduler>, host: &mut (impl CellHost + ?Sized)) {
        let active = self.fade.is_some();
        let stop = self.fade_callback();
        if active && self.fade.is_none() {
            host.lifecycle(FreshLifecycle::FadeEnd);
        }
        if stop {
            self.stop(scheduler, host);
        }
    }
    pub fn stop(&mut self, scheduler: Option<&mut dyn super::Scheduler>, host: &mut (impl CellHost + ?Sized)) {
        if !self.initialized || !self.playing {
            return;
        }
        if let Some(scheduler) = scheduler {
            scheduler.unregister();
        }
        self.fade = None;
        host.lifecycle(FreshLifecycle::FadeEnd);
        self.timer_running = false;
        host.lifecycle(FreshLifecycle::TimerStop);
        for cell in &mut self.cells {
            if cell.is_busy() {
                let _ = cell.stop(host);
            }
        }
        self.playing = false;
    }
    pub fn shutdown(&mut self, scheduler: Option<&mut dyn super::Scheduler>, host: &mut (impl CellHost + ?Sized)) {
        self.fade = None;
        host.lifecycle(FreshLifecycle::FadeEnd);
        self.stop(scheduler, host);
        if self.initialized {
            self.score = None;
            host.lifecycle(FreshLifecycle::ScoreDelete);
        }
        for cell in self.cells.drain(..) {
            host.lifecycle(FreshLifecycle::PlayerDelete(cell.cell.as_ref().map_or(0, |c| c.id)));
        }
        self.playing = false;
        self.initialized = false;
        self.controller = None;
        host.lifecycle(FreshLifecycle::ControllerRelease);
    }
    pub fn reseed(&mut self, seed: u32, mut scheduler: Option<&mut dyn super::Scheduler>, host: &mut dyn FreshHost) {
        host.reseed_private(seed);
        self.beat_in_bar = 0;
        self.next_beat = 1;
        self.origin = 0;
        self.budget_bases = [0, 4];
        self.remembered_coordinates = [0, 0];
        self.recount();
        if self.playing {
            if let Some(schedule) = &mut scheduler {
                self.stop(Some(&mut **schedule), host);
            } else {
                self.stop(None, host);
            }
            self.play(scheduler);
        }
    }
    pub fn max_cells(&self) -> i32 {
        let Some(score) = &self.score else {
            return 0;
        };
        let base = self.play_percent.unwrap_or(100).wrapping_mul(self.budget_bases[0]) / 100;
        (base
            + (self.coordinates[0] + 1).wrapping_mul(score.gradients[1]) / (self.dimensions[0] as i32 - 1).max(1)
            + (self.coordinates[1] + 1).wrapping_mul(score.gradients[0]) / (self.dimensions[1] as i32 - 1).max(1))
        .min(self.distinct_groups)
    }
    pub fn min_cells(&self) -> i32 {
        self.budget_bases[1].wrapping_mul(self.play_percent.unwrap_or(100)) / 100
    }
    pub fn choose_division(&mut self, index: usize, host: &mut dyn FreshHost) -> bool {
        let enabled: Vec<_> = (0..9)
            .filter(|n| n / 3 < self.division_counts[0] && n % 3 < self.division_counts[1] && self.divisions[*n])
            .collect();
        if enabled.is_empty() {
            return false;
        }
        let player = &mut self.cells[index];
        let mut choice = player.division;
        let mut draw = host.random_below(enabled.len() as u32 * 1000);
        for division in enabled {
            if Some(division) == player.division {
                continue;
            }
            if draw < 1000 {
                choice = Some(division);
                break;
            }
            draw -= 1000;
        }
        let Some(choice) = choice else {
            return false;
        };
        player.division = Some(choice);
        player.position = self.division_positions[choice];
        player.position_set = true;
        player.head_relative = self.head_relative;
        player.distances = self.distances;
        true
    }
    pub fn periodic_update(&mut self, elapsed: u32, host: &mut dyn FreshHost) {
        if !self.initialized || !self.playing || self.paused {
            return;
        }
        while self.beat_due(self.next_beat) as u32 <= elapsed {
            if elapsed > 86_400_000 {
                self.origin = self.beats_per_bar;
                self.beat_in_bar = self.beats_per_bar;
                self.next_beat = self.beats_per_bar;
                host.lifecycle(FreshLifecycle::TimerRestart);
                break;
            }
            let decision = self.beat_in_bar == self.beats_per_bar - 1 || self.beat_in_bar == self.beats_per_bar - 6;
            let mut group = 0;
            if decision && !self.manual {
                let playing = self.cells.iter().filter(|cell| cell.is_playing()).count() as i32;
                if self.block_mode && playing >= self.min_cells() {
                    let draw = host.private_draw();
                    if let Some(index) = self.cell_to_subtract(draw, || host.private_draw()) {
                        group = self.cells[index].cell.as_ref().map_or(0, |cell| cell.group);
                        self.last_subtracted = Some(index);
                        self.remembered_coordinates =
                            [(index / self.dimensions[1]) as i32, (index % self.dimensions[1]) as i32];
                        self.cells[index].stop_flag = true;
                        let _ = self.cells[index].stop(host);
                    }
                }
                let active = self
                    .cells
                    .iter()
                    .filter(|cell| cell.is_busy() && !cell.stop_flag)
                    .count() as i32;
                if self.start_switch && active < self.max_cells() {
                    let draw = host.private_draw();
                    let mut best = self.best_start(group, draw, || host.c_rand());
                    if best.is_none() {
                        let draw = host.private_draw();
                        best = self.best_start(19, draw, || host.c_rand());
                    }
                    if best.is_none() && active < self.min_cells() {
                        let draw = host.private_draw();
                        best = self.best_start(0, draw, || host.c_rand());
                    }
                    if let Some(index) = best {
                        if self.choose_division(index, host) {
                            self.cells[index].controller.clone_from(&self.controller);
                            let score_volume = self.score.as_ref().map_or(0, |score| score.volume_word);
                            self.cells[index].start(host, self.volume, score_volume);
                        }
                    }
                }
            }
            for cell in &mut self.cells {
                cell.update(host);
            }
            self.beat_in_bar = self.beat_in_bar.wrapping_add(1);
            if self.beat_in_bar > self.beats_per_bar {
                self.beat_in_bar = 1;
            }
            self.next_beat = self.next_beat.wrapping_add(1);
        }
    }
}
