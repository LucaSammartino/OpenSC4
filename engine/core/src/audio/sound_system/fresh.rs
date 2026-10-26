// SPDX-License-Identifier: GPL-3.0-or-later
//! Adaptive score owners (§6). No cell-selection, tempo or fade law is invented.
pub use super::fresh_playback::{FreshHost, FreshLifecycle, PathScoreInput, VolumeFade};
use super::{Properties, Scheduler, SoundRef, Unsupported};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct CellData {
    pub id: u64,
    pub volume_word: i32,
    pub fade_out: u32,
    pub name: String,
    pub group: i32,
    pub class: i32,
    pub links: Vec<usize>,
    pub looping: bool,
    pub loop_length: i32,
    pub fade_in_selector: i32,
    pub random_volume: bool,
    pub pitch: [i32; 2],
    pub pan: [i32; 2],
    pub random_pan: bool,
}
#[derive(Default)]
pub struct ScoreData {
    pub cells: Vec<CellData>,
    pub volume_word: i32,
    pub starting_coordinates: [i32; 2],
    /// Raw score configuration, without invented editor-facing names.
    pub settings: Properties,
    pub rows: usize,
    pub columns: usize,
    pub half: [i32; 2],
    pub block_mode: bool,
    pub tempo: i32,
    pub beats_per_bar: i32,
    /// Maximum/minimum base, column/row gradient (§freshness 4.4).
    pub budget_bases: [i32; 2],
    pub gradients: [i32; 2],
}
pub trait ScoreInput {
    type Record;
    fn persist_record(&mut self, resource: u32) -> Option<Self::Record>;
    fn load_score_as_file(&mut self, record: Self::Record) -> Result<ScoreData, Unsupported>;
    fn persist_record_key(&mut self, _key: super::hit::ResourceKey) -> Option<Self::Record> {
        None
    }
}

pub struct FreshPlayer {
    pub score: Option<ScoreData>,
    pub initialized: bool,
    pub playing: bool,
    pub volume: i32,
    pub zoom: Option<i32>,
    pub play_percent: Option<i32>,
    pub coordinates: [i32; 2],
    pub settings: Properties,
    pub timer_running: bool,
    pub cells: Vec<FreshCellPlayer>,
    pub fade: Option<VolumeFade>,
    pub dimensions: [usize; 2],
    pub half: [i32; 2],
    pub block_mode: bool,
    pub last_subtracted: Option<usize>,
    pub remembered_coordinates: [i32; 2],
    pub selectable_count: i32,
    pub distinct_groups: i32,
    pub tempo: i32,
    pub beats_per_bar: i32,
    pub beat_in_bar: i32,
    pub next_beat: i32,
    pub origin: i32,
    pub paused: bool,
    pub manual: bool,
    pub start_switch: bool,
    pub budget_bases: [i32; 2],
    pub divisions: [bool; 9],
    pub division_positions: [[i32; 3]; 9],
    pub division_counts: [usize; 2],
    pub head_relative: bool,
    pub distances: [u32; 2],
    pub resource_key: Option<super::hit::ResourceKey>,
    pub data_directory: String,
    pub controller: Option<super::directors::ForeignRef>,
    owner_identity: Arc<()>,
}
impl Default for FreshPlayer {
    fn default() -> Self {
        Self {
            score: None,
            initialized: false,
            playing: false,
            volume: 0x400,
            zoom: None,
            play_percent: None,
            coordinates: [0, 0],
            settings: Properties::new(),
            timer_running: false,
            cells: Vec::new(),
            fade: None,
            dimensions: [0, 0],
            half: [0, 0],
            block_mode: false,
            last_subtracted: None,
            remembered_coordinates: [-1, -1],
            selectable_count: 0,
            distinct_groups: 0,
            tempo: 0,
            beats_per_bar: 0,
            beat_in_bar: 0,
            next_beat: 0,
            origin: 0,
            paused: false,
            manual: false,
            start_switch: true,
            budget_bases: [0, 4],
            divisions: [false, true, false, true, false, true, false, true, false],
            division_positions: std::array::from_fn(|n| [(n / 3) as i32 * 100 - 100, 0, 100 - (n % 3) as i32 * 100]),
            division_counts: [3, 3],
            head_relative: false,
            distances: [100, 1000],
            resource_key: None,
            data_directory: String::new(),
            controller: None,
            owner_identity: Arc::new(()),
        }
    }
}
impl FreshPlayer {
    pub const CLSID: u32 = 0x3C1C_F59E;
    pub const CELL_STORAGE_CAPACITY: usize = 4096;
    pub const ROW_STRIDE: usize = 64;
    pub fn init_resource(&mut self, resource: u32, input: &mut impl ScoreInput) -> Result<bool, Unsupported> {
        let Some(record) = input.persist_record(resource) else {
            return Ok(false);
        };
        let score = input.load_score_as_file(record)?;
        self.init_loaded_score(score)?;
        Ok(true)
    }
    pub fn init_resource_key(
        &mut self,
        key: super::hit::ResourceKey,
        input: &mut impl ScoreInput,
    ) -> Result<bool, Unsupported> {
        self.resource_key = Some(key);
        let Some(record) = input.persist_record_key(key) else {
            return Ok(false);
        };
        let score = input.load_score_as_file(record)?;
        self.init_loaded_score(score)?;
        Ok(true)
    }
    pub fn init_loaded_score(&mut self, score: ScoreData) -> Result<(), Unsupported> {
        self.score = Some(score);
        self.init()
    }
    pub fn init(&mut self) -> Result<(), Unsupported> {
        if self.initialized {
            return Ok(());
        }
        let score = self.score.as_ref().ok_or(Unsupported("missing-score"))?;
        // SPEC-GAP: invalid dimensions/sparse grids have unsafe original behavior; reject safely.
        if score.rows == 0
            || score.columns == 0
            || score.rows > 64
            || score.columns > 64
            || score.cells.len() != score.rows * score.columns
        {
            return Err(Unsupported("invalid-score-dimensions"));
        }
        self.cells = score
            .cells
            .iter()
            .map(|cell| {
                let mut player = FreshCellPlayer::new(Arc::clone(&self.owner_identity));
                player.set_cell(cell.clone());
                player
            })
            .collect();
        self.settings = score.settings.clone();
        self.volume = score.volume_word;
        self.half = score.half;
        self.dimensions = [score.rows, score.columns];
        self.coordinates = [1, 1];
        self.remembered_coordinates = [-1, -1];
        self.block_mode = score.block_mode;
        self.budget_bases = score.budget_bases;
        self.recount();
        self.initialized = true;
        Ok(())
    }
    pub fn play(&mut self, scheduler: Option<&mut dyn Scheduler>) -> bool {
        if self.initialized && !self.playing {
            if let Some(score) = &self.score {
                self.settings = score.settings.clone();
                self.tempo = score.tempo;
                self.beats_per_bar = score.beats_per_bar;
                self.origin = score.beats_per_bar - i32::from(score.beats_per_bar > 2);
                self.beat_in_bar = self.origin;
                self.next_beat = self.origin;
                let start = score.starting_coordinates;
                self.set_y_position(start[0]);
                self.set_x_position(start[1]);
            }
            self.fade = None;
            self.manual = false;
            self.timer_running = true;
            self.playing = true;
            if let Some(scheduler) = scheduler {
                scheduler.register(250);
            }
        }
        true
    }
    pub fn set_zoom(&mut self, zoom: i32) -> bool {
        if self.initialized {
            self.zoom = Some(zoom);
        }
        true
    }
    pub fn set_play_percent(&mut self, percent: i32) -> bool {
        if self.initialized {
            self.play_percent = Some(percent);
        }
        true
    }
    pub fn update_volume(&mut self, volume: i32, scheduler: Option<&mut dyn Scheduler>) -> bool {
        self.volume = volume;
        self.play(scheduler)
    }
    /// The probability-named routine counts busy players (§6.1).
    pub fn prob_of_cell_starting(&self) -> usize {
        self.cells.iter().filter(|cell| cell.is_busy()).count()
    }
}

/// Shared critical section for freshness state; callback callers use try_lock
/// and may skip work if an update already holds it (§6.1). Never RefCell.
pub type SharedFreshPlayer = Arc<Mutex<FreshPlayer>>;
pub fn try_callback(player: &SharedFreshPlayer, callback: impl FnOnce(&mut FreshPlayer)) -> bool {
    match player.try_lock() {
        Ok(mut guard) => {
            callback(&mut guard);
            true
        }
        Err(_) => false,
    }
}

pub trait CellHost {
    /// Adapter queries the optional update interface and balances that QI.
    fn unregister_source_update_if_available(&mut self, sound: &SoundRef);
    fn fade_to_zero(&mut self, sound: &SoundRef, duration: u32);
    fn sound_system_available(&self) -> bool {
        false
    }
    fn sound_by_id(&mut self, _id: u32) -> Option<SoundRef> {
        None
    }
    fn sound_by_name(&mut self, _name: &str) -> Option<SoundRef> {
        None
    }
    fn private_draw(&mut self) -> u32 {
        0
    }
    fn crc24_filename(&mut self, _name: &str) -> u32 {
        0
    }
    /// SPEC-GAP: P's literal doubles are not listed; the adapter supplies the semitone table.
    fn pitch_ratio(&mut self, index: usize) -> Option<f64> {
        if index == 0 {
            Some(1.0)
        } else {
            None
        }
    }
    fn register_source_update_if_available(&mut self, _sound: &SoundRef, _period: u32) {}
    fn external_start(&mut self, _cell: &CellData, _id: u32, _head: bool, _looping: bool, _fade: f32) -> bool {
        false
    }
    fn external_position(&mut self, _cell: &CellData, _position: [f32; 3]) {}
    fn external_distances(&mut self, _cell: &CellData, _distances: [f32; 2]) {}
    fn external_volume(&mut self, _cell: &CellData, _volume: f32) {}
    fn external_pitch(&mut self, _cell: &CellData, _pitch: f32) {}
    fn external_stop(&mut self, _cell: &CellData, _fade: f32) {}
    fn external_playing(&mut self, _cell: &CellData) -> bool {
        false
    }
    fn lifecycle(&mut self, _event: FreshLifecycle) {}
}
pub struct FreshCellPlayer {
    pub owner: Arc<()>,
    pub cell: Option<CellData>,
    pub source: Option<SoundRef>,
    pub state: u32,
    pub stop_flag: bool,
    pub local_volume: i32,
    pub intermediate_volume: i32,
    pub scaled_volume: i32,
    pub external_controller: bool,
    pub dead: bool,
    pub counter: i32,
    pub random_factor: i32,
    pub head_relative: bool,
    pub distances: [u32; 2],
    pub position: [i32; 3],
    pub position_set: bool,
    pub division: Option<usize>,
    pub controller: Option<super::directors::ForeignRef>,
}
impl FreshCellPlayer {
    pub fn new(owner: Arc<()>) -> Self {
        Self {
            owner,
            cell: None,
            source: None,
            state: 0,
            stop_flag: false,
            local_volume: 0x400,
            intermediate_volume: 0,
            scaled_volume: 0,
            external_controller: false,
            dead: false,
            counter: 0,
            random_factor: 1024,
            head_relative: false,
            distances: [0, 0],
            position: [0, 0, 0],
            position_set: false,
            division: None,
            controller: None,
        }
    }
    pub fn set_cell(&mut self, cell: CellData) {
        self.cell = Some(cell);
    }
    pub fn is_playing(&self) -> bool {
        self.state == 2
    }
    pub fn is_busy(&self) -> bool {
        self.state != 0
    }
    pub fn is_dead(&self) -> bool {
        self.dead
    }
    pub fn set_dead(&mut self, dead: bool) {
        self.dead = dead;
    }
    pub fn kill(&mut self) -> bool {
        self.source = None;
        self.state = 0;
        self.stop_flag = false;
        true
    }
    pub fn stop(&mut self, host: &mut (impl CellHost + ?Sized)) -> Result<(), Unsupported> {
        if self.external_controller || self.controller.is_some() {
            if self.state == 2 {
                let cell = self.cell.as_ref().ok_or(Unsupported("missing-cell"))?;
                host.external_stop(cell, cell.fade_out as i32 as f32);
                if cell.fade_out != 0 {
                    self.state = 3;
                }
            } else if self.state != 3 {
                self.state = 0;
            }
            return Ok(());
        }
        if let Some(sound) = &self.source {
            host.unregister_source_update_if_available(sound);
        }
        match self.state {
            2 => {
                let Some(cell) = &self.cell else {
                    return Err(Unsupported("missing-cell"));
                };
                if cell.fade_out == 0 {
                    self.kill();
                } else {
                    self.state = 3;
                    if let Some(sound) = &self.source {
                        host.fade_to_zero(sound, cell.fade_out);
                    }
                }
            }
            3 => (),
            _ => self.state = 0,
        }
        Ok(())
    }
    /// Ordinary callers without a controller can use this convenience entry.
    pub fn update_source(&mut self, special_countdown: bool) -> Result<(), Unsupported> {
        if self.external_controller || self.controller.is_some() {
            return Err(Unsupported("cell-host-required"));
        }
        if special_countdown {
            self.update_countdown();
            return Ok(());
        }
        if self.source.as_ref().is_some_and(|sound| !sound.is_playing()) {
            self.state = 0;
        }
        Ok(())
    }
    pub fn update_scaled_volume(&mut self, c: i32, a: i32, p: i32, s: i32) -> i32 {
        self.intermediate_volume = p.wrapping_mul(c.wrapping_mul(a)) / 1_048_576;
        self.scaled_volume = s.wrapping_mul(self.local_volume).wrapping_mul(self.intermediate_volume) / 1_048_576;
        self.scaled_volume
    }
    pub fn set_volume(&mut self, local: i32, c: i32, a: i32, p: i32, s: i32) -> bool {
        self.local_volume = local;
        let scaled = self.update_scaled_volume(c, a, p, s);
        self.source.as_ref().map_or(true, |sound| sound.set_volume(scaled))
    }
}
