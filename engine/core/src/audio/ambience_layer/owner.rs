// SPDX-License-Identifier: GPL-3.0-or-later
//! Layer lifecycle and uncovered slots, audio-ambience §8.1.1 (AA14).
use super::*;

pub trait Host: FilterHost + LayerEventHost + LayerTuningSource {
    fn resource_manager(&mut self) -> Option<u64>; // borrowed
    fn sound_system(&mut self) -> Option<u64>; // acquired
    fn add_ref(&mut self, object: u64);
    fn message_server(&mut self) -> u64; // required, borrowed
    fn subscription(&mut self, server: u64, subscribe: bool, message: u32);
    fn tuning_available(&mut self, layer: u32) -> bool;
    fn preference_mode(&mut self) -> u32;
    fn create_player(&mut self, class: u32, iid: u32) -> Option<u64>;
    fn group_insert(&mut self, occupant: u64, position: Position, radius: f32) -> bool;
    fn group_update(&mut self, occupant: u64, position: Position, radius: f32) -> bool;
    fn group_remove(&mut self, occupant: u64) -> bool;
}

#[derive(Default, Debug)]
pub struct Process {
    pub globals: LayerGlobals,
    pub resources: Option<u64>,
    pub sounds: Option<u64>,
}
impl Process {
    pub fn static_init(&mut self, host: &mut impl Host) -> bool {
        let Some(resources) = host.resource_manager() else {
            return false;
        };
        host.add_ref(resources);
        if let Some(old) = self.resources.replace(resources) {
            host.release(old);
        }
        let Some(sounds) = host.sound_system() else {
            return false;
        };
        host.add_ref(sounds);
        if let Some(old) = self.sounds.replace(sounds) {
            host.release(old);
        }
        host.release(sounds); // service lookup temporary
        true
    }
    pub fn static_shutdown(&mut self, host: &mut impl Host) -> bool {
        for (_, filter) in std::mem::take(&mut self.globals.filters) {
            host.release(filter);
        }
        if let Some(object) = self.resources.take() {
            host.release(object);
        }
        if let Some(object) = self.sounds.take() {
            host.release(object);
        }
        true
    }
}

#[derive(Debug)]
pub struct LayerOwner {
    pub layer: AmbienceLayer,
    pub id: u32,
    pub initialized: bool,
    pub enabled: bool,
    pub refcount: u32,
    pub occupant_count: u32,
    pub score_tgis: Vec<[u32; 3]>,
}

impl LayerOwner {
    pub fn add_ref(&mut self) -> u32 {
        self.refcount = self.refcount.wrapping_add(1);
        self.refcount
    }
    pub fn construct(id: u32) -> Self {
        // Only defined constructor words are exposed. Tuning defaults are staged
        // for safe Rust storage, and are not a claim about native padding.
        let mut tuning = LayerTuning::defaults(id, 0, 0);
        tuning.name.clear();
        tuning.master_volume = 0;
        let ease = Ease {
            target_position: [0.0; 3],
            current_position: [0.0; 3],
            target_scalar: 0,
            current_scalar: 0,
            stepped: false,
            vector_rate: 0.0,
            scalar_rate: 0.0,
        };
        Self {
            id,
            initialized: false,
            enabled: true,
            refcount: 0,
            occupant_count: 0,
            score_tgis: Vec::new(),
            layer: AmbienceLayer {
                tuning,
                filters: LayerFilters::default(),
                occupant_groups: GroupLayer::new(50.0),
                cell_groups: GroupLayer::new(50.0),
                intersection: GroupLayer::new(50.0),
                rectangle: [0.0; 4],
                counters: [0; 4],
                can_handle_this_update: false,
                paused: false,
                group_count: 0,
                score: 0.0,
                samples: Vec::new(),
                listener_position: [0.0; 3],
                listener_vector: [0.0; 3],
                average: [0.0; 3],
                score_volume: 0,
                play_percent: 0,
                loop_volume: 0,
                scores: Vec::new(),
                loops: Vec::new(),
                ease,
            },
        }
    }
    pub fn query(&mut self, iid: u32) -> bool {
        if !matches!(iid, 1 | 0x4A28_35FE) {
            return false;
        }
        self.refcount = self.refcount.wrapping_add(1);
        true
    }
    fn tune(&mut self, host: &mut impl Host) {
        let mode = host.preference_mode();
        let available = host.tuning_available(self.id);
        self.layer.tuning = LayerTuning::read(
            self.id,
            self.layer.tuning.loop_id,
            mode,
            if available {
                Some(host as &mut dyn LayerTuningSource)
            } else {
                None
            },
        )
        .0;
        self.layer.ease.vector_rate = self.layer.tuning.vector_rate;
        self.layer.ease.scalar_rate = self.layer.tuning.scalar_rate;
    }
    pub fn init(&mut self, process: &mut Process, host: &mut impl Host) -> bool {
        if self.initialized {
            return true;
        }
        self.initialized = true;
        process.globals.layer_count = process.globals.layer_count.wrapping_add(1);
        if process.globals.layer_count == 1 && !process.static_init(host) {
            return false;
        }
        self.tune(host);
        let server = host.message_server();
        host.subscription(server, true, 0xC53D_10AA);
        self.layer
            .filters
            .create(&self.layer.tuning.filter_ids, &mut process.globals, host);
        self.layer.create_group_layers();
        self.load_scores(host);
        self.add_loop(self.layer.tuning.loop_id);
        true
    }
    pub fn shutdown(&mut self, process: &mut Process, host: &mut impl Host) -> bool {
        if !self.initialized {
            return true;
        }
        let server = host.message_server();
        host.subscription(server, false, 0xC53D_10AA);
        self.layer.intersection.groups.clear();
        self.layer.cell_groups.groups.clear();
        self.layer.occupant_groups.groups.clear();
        self.layer.filters = LayerFilters::default();
        self.remove_all_scores(host);
        self.remove_all_loops(host);
        process.globals.layer_count = process.globals.layer_count.wrapping_sub(1);
        if process.globals.layer_count == 0 {
            process.static_shutdown(host);
        }
        self.initialized = false;
        true
    }
    pub fn do_message(&mut self, message: Option<u32>, process: &mut Process, host: &mut impl Host) -> bool {
        if message != Some(0xC53D_10AA) {
            return false;
        }
        self.tune(host);
        self.layer.filters = LayerFilters::default();
        self.layer
            .filters
            .create(&self.layer.tuning.filter_ids, &mut process.globals, host);
        self.layer.create_group_layers();
        self.enable(!self.enabled);
        self.enable(!self.enabled);
        true
    }
    pub fn enable(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    pub fn can_handle_occupant(&self, _: Option<u64>) -> bool {
        false
    }
    pub fn insert_occupant(&mut self, occupant: Option<u64>, host: &mut impl Host) -> bool {
        let Some(o) = occupant else { return false };
        let p = host.occupant_position(o);
        if !host.group_insert(o, p, 0.0) {
            return false;
        }
        host.add_ref(o);
        self.occupant_count = self.occupant_count.wrapping_add(1);
        true
    }
    pub fn move_occupant(&mut self, occupant: Option<u64>, host: &mut impl Host) -> bool {
        let Some(o) = occupant else { return false };
        let p = host.occupant_position(o);
        host.group_update(o, p, self.layer.tuning.occupant_radius);
        true
    }
    pub fn remove_occupant(&mut self, occupant: Option<u64>, host: &mut impl Host) -> bool {
        let Some(o) = occupant else { return false };
        if !host.group_remove(o) {
            return false;
        }
        host.release(o);
        self.occupant_count = self.occupant_count.wrapping_sub(1);
        true
    }
    pub fn add_loop(&mut self, sound: u32) -> bool {
        if sound == 0 || self.layer.loops.iter().any(|l| l.sound == sound) {
            return false;
        }
        self.layer.loops.push(LoopEntry {
            sound,
            instance: 0,
            last_position: [0.0; 3],
            last_volume: 0,
        });
        true
    }
    pub fn remove_loop(&mut self, sound: u32) -> bool {
        let Some(i) = self.layer.loops.iter().position(|l| l.sound == sound) else {
            return false;
        };
        self.layer.loops.remove(i);
        true
    }
    pub fn remove_all_loops(&mut self, host: &mut impl Host) -> bool {
        for l in self.layer.loops.drain(..) {
            if l.instance != 0 {
                let e = host
                    .create_event(0x6A45_1908, 0x5B52_3BA4)
                    .expect("unchecked stop event output");
                for (i, w) in [l.instance, 0, 0, 0].into_iter().enumerate() {
                    host.set_data(e, i + 1, w);
                }
                host.post_event(e);
                host.release_event(e);
            }
        }
        true
    }
    pub fn load_scores(&mut self, host: &mut impl Host) {
        let words = self.layer.tuning.score_ids.clone();
        for pair in words.chunks_exact(2) {
            if !self.add_score([0x2026_960B, pair[0], pair[1]], host) {
                host.log("Failed freshness score addition");
            }
        }
    }
    pub fn add_score(&mut self, tgi: [u32; 3], host: &mut impl Host) -> bool {
        if self.score_tgis.contains(&tgi) {
            return false;
        }
        let Some(player) = host.create_player(0x3C1C_F59E, 0xDC1C_F595) else {
            return false;
        };
        let Some(e) = host.create_event(0x2A2C_0384, 0x5B52_3BA4) else {
            host.release(player);
            return false;
        };
        for i in 1..=4 {
            host.set_data(e, i, 0);
        }
        host.set_void1(e, player);
        for (i, w) in tgi.into_iter().enumerate() {
            host.set_data(e, i + 1, w);
        }
        host.post_event(e);
        host.add_ref(player);
        self.layer.scores.push(ScoreEntry {
            playing: false,
            ready: true,
            last_percent: 0,
            last_volume: 0,
            player,
        });
        self.score_tgis.push(tgi);
        host.release(player);
        host.release_event(e);
        true
    }
    fn remove_score_at(&mut self, index: usize, singular: bool, host: &mut impl Host) {
        let score = self.layer.scores.remove(index);
        self.score_tgis.remove(index);
        let e = host.create_event(0x4A2C_0389, 0x5B52_3BA4);
        if let Some(e) = e {
            host.set_void1(e, score.player);
            host.post_event(e);
        }
        if singular {
            host.release(score.player);
        } // specified extra Release quirk
        host.release(score.player);
        if let Some(e) = e {
            host.release_event(e);
        }
    }
    pub fn remove_score(&mut self, instance: u32, host: &mut impl Host) -> bool {
        let Some(i) = self.score_tgis.iter().position(|t| t[2] == instance) else {
            return false;
        };
        self.remove_score_at(i, true, host);
        true
    }
    pub fn remove_all_scores(&mut self, host: &mut impl Host) -> bool {
        while !self.layer.scores.is_empty() {
            self.remove_score_at(0, false, host);
        }
        true
    }
    pub fn set_score_master_volume(&mut self, volume: u32, zoom: f32, host: &mut impl Host) {
        self.layer.tuning.master_volume = volume;
        // UpdateAmbienceScores' elapsed argument is 0x7fffffff; its extracted
        // score stage has no elapsed-dependent operation.
        self.layer.update_scores(zoom, host);
    }
    pub fn release(&mut self, host: &mut impl Host) -> u32 {
        if (self.refcount as i32) > 1 {
            self.refcount -= 1;
            return self.refcount;
        }
        // Destruction is deliberately separate from Shutdown and process count.
        self.layer.filters = LayerFilters::default();
        for score in self.layer.scores.drain(..) {
            host.release(score.player);
        }
        self.score_tgis.clear();
        self.layer.loops.clear();
        self.layer.intersection.groups.clear();
        self.layer.cell_groups.groups.clear();
        self.layer.occupant_groups.groups.clear();
        self.refcount = 0;
        0
    }
}
