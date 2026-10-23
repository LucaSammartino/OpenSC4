// SPDX-License-Identifier: GPL-3.0-or-later
//! Adaptive score owners (§6). No cell-selection, tempo or fade law is invented.
use super::{Properties, Scheduler, SoundRef, Unsupported};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct CellData {
    pub id: u64,
    pub volume_word: i32,
    pub fade_out: u32,
}
pub struct ScoreData {
    pub cells: Vec<CellData>,
    pub volume_word: i32,
    pub starting_coordinates: [i32; 2],
    /// Raw score configuration, without invented editor-facing names.
    pub settings: Properties,
}
pub trait ScoreInput {
    type Record;
    fn persist_record(&mut self, resource: u32) -> Option<Self::Record>;
    fn load_score_as_file(&mut self, record: Self::Record) -> Result<ScoreData, Unsupported>;
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
    /// Takes decoded cells, never arbitrary 64x64 coordinates. Dimension
    /// acceptance and overloaded argument contracts remain SS6.
    pub fn init_loaded_score(&mut self, score: ScoreData) -> Result<(), Unsupported> {
        if score.cells.len() > Self::CELL_STORAGE_CAPACITY {
            return Err(Unsupported("SS6"));
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
        self.score = Some(score);
        self.initialized = true;
        Ok(())
    }
    pub fn play(&mut self, scheduler: Option<&mut dyn Scheduler>) -> bool {
        if self.initialized && !self.playing {
            if let Some(score) = &self.score {
                self.settings = score.settings.clone();
                self.coordinates = score.starting_coordinates;
            }
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
    pub fn periodic_update(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported("SS6"))
    }
    pub fn fade_volume(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported("SS6"))
    }
    pub fn stop_or_shutdown(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported("SS6"))
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
    pub fn kill(&mut self) -> bool {
        self.source = None;
        self.state = 0;
        self.stop_flag = false;
        true
    }
    pub fn stop(&mut self, host: &mut dyn CellHost) -> Result<(), Unsupported> {
        if self.external_controller {
            return Err(Unsupported("SS7"));
        }
        if let Some(sound) = &self.source {
            host.unregister_source_update_if_available(sound);
        }
        match self.state {
            2 => {
                let Some(cell) = &self.cell else {
                    return Err(Unsupported("SS7"));
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
    /// Known ordinary-source poll; special loop/countdown and controllers SS7.
    pub fn update_source(&mut self, special_countdown: bool) -> Result<(), Unsupported> {
        if self.external_controller || special_countdown {
            return Err(Unsupported("SS7"));
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
    pub fn start_or_spatial_policy(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported("SS7"))
    }
}
