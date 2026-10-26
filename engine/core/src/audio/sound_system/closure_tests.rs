// SPDX-License-Identifier: GPL-3.0-or-later
//! Worked cases from sound-system.md, SS1–SS7.
use super::fresh::*;
use super::{hit::*, track::*, *};
use std::sync::atomic::{AtomicI32, Ordering};

#[derive(Default)]
struct Source {
    frequency: AtomicI32,
    calls: Mutex<Vec<String>>,
}
impl Source {
    fn record(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}
impl Sound for Source {
    fn load(&self) -> bool {
        self.record("load".into());
        true
    }
    fn unload(&self) -> bool {
        true
    }
    fn is_loaded(&self) -> bool {
        true
    }
    fn is_playing(&self) -> bool {
        true
    }
    fn pause(&self) {
        self.record("pause".into());
    }
    fn unpause(&self) {
        self.record("unpause".into());
    }
    fn play(&self, p: u32, looping: bool, flag: bool) -> bool {
        self.record(format!("play:{p}:{looping}:{flag}"));
        false
    }
    fn set_volume(&self, v: i32) -> bool {
        self.record(format!("vol:{v}"));
        true
    }
    fn frequency(&self) -> i32 {
        self.frequency.load(Ordering::Relaxed)
    }
    fn set_frequency(&self, v: i32) -> bool {
        self.record(format!("freq:{v}"));
        true
    }
    fn set_pan(&self, v: i32) -> bool {
        self.record(format!("pan:{v}"));
        true
    }
    fn spatial(&self, head: bool, min: f32, max: f32, pos: Option<[f32; 3]>) -> bool {
        self.record(format!("spatial:{head}:{min}:{max}:{pos:?}"));
        true
    }
    fn set_velocity(&self, v: [f32; 3]) -> bool {
        self.record(format!("velocity:{v:?}"));
        true
    }
    fn set_min_distance(&self, v: f32) -> bool {
        self.record(format!("min:{v}"));
        true
    }
    fn set_max_distance(&self, v: f32) -> bool {
        self.record(format!("max:{v}"));
        true
    }
    fn set_position(&self, v: [f32; 3]) -> bool {
        self.record(format!("position:{v:?}"));
        true
    }
}

#[derive(Default)]
struct TrackSpy {
    effects: Vec<TrackEffect>,
    errors: Vec<String>,
    error_result: i32,
}
impl TrackIo for TrackSpy {
    fn effect(&mut self, effect: TrackEffect) -> bool {
        self.effects.push(effect);
        true
    }
    fn report(&mut self, message: &str) -> i32 {
        self.errors.push(message.into());
        self.error_result
    }
    fn entry(&mut self, id: u32) -> Option<usize> {
        Some(id as usize)
    }
}

#[test]
fn ss1_mode3_and_play_order_suppress_notifications() {
    let mut track = HitTrackPlayer {
        executing: true,
        ..Default::default()
    };
    track
        .properties
        .extend([(0xFF4D_CD0D, 3), (0xFF30_3B1B, 1), (0xFF59_FDD8, 23)]);
    let mut io = TrackSpy::default();
    assert!(track.play_with([9, 512, (-20i32) as u32, 77], &mut io));
    assert_eq!(
        io.effects,
        [
            TrackEffect::Kill,
            TrackEffect::Start,
            TrackEffect::StartDependent(23),
            TrackEffect::Unpause
        ]
    );
    assert_eq!(
        [
            track.properties[&0xFFB4_335C],
            track.properties[&0xFF90_AAC8],
            track.properties[&0xFF3E_C880]
        ],
        [9, 512, -20]
    );
    assert_eq!(track.properties[&PAYLOAD_PROPERTIES[3]], 77);
    assert_eq!(track.flags, [false, false, true]);
    assert!(track.property_notifications.is_empty());
}

#[test]
fn ss1_byte_width_wait_and_strict_ramp_worked_cases() {
    let mut track = HitTrackPlayer::default();
    let mut io = TrackSpy::default();
    track
        .execute_next(&[4, 13, 255, 5, 14, 0x78, 0x56, 0x34, 0x12], &mut io)
        .unwrap();
    assert_eq!((track.pc, track.properties[&13]), (3, 255));
    track
        .execute_next(&[4, 13, 255, 5, 14, 0x78, 0x56, 0x34, 0x12], &mut io)
        .unwrap();
    assert_eq!((track.pc, track.properties[&14]), (9, 0x1234_5678));
    track.pc = 0;
    track.executing = true;
    track.wait = 30;
    track.properties.extend([
        (13, 30),
        (0xFF0A_D592, 1),
        (0xFF47_F685, 14),
        (14, 0),
        (0xFF2D_16F2, 10),
        (0xFFD1_C82F, 100),
    ]);
    track.timer_callback(20, &[9, 13], &mut io).unwrap();
    assert_eq!(track.wait, 10);
    track.timer_callback(20, &[9, 13], &mut io).unwrap();
    assert_eq!(track.wait, 30); // excess ten milliseconds is discarded
    track.wait = 1000;
    track.properties.insert(14, 0);
    track.timer_callback(100, &[], &mut io).unwrap();
    assert_eq!((track.properties[&14], track.properties[&0xFF0A_D592]), (10, 1));
    track.timer_callback(100, &[], &mut io).unwrap();
    assert_eq!((track.properties[&14], track.properties[&0xFF0A_D592]), (10, 0));
    track.pc = 0;
    assert_eq!(track.execute_next(&[0x58], &mut io), Err(Unsupported("SS1")));
    track.pc = 0;
    track.execute_next(&[0], &mut io).unwrap();
    assert_eq!(io.errors, ["Illegal Instruction"]);
}

#[test]
fn ss2_whole_file_duplicate_registration_semantic_duck_and_random() {
    fn block(offsets: &[u32]) -> Arc<TrackDataBlock> {
        let mut bytes = b"prefix!ENTP".to_vec(); // deliberately unaligned; TRAX is not required
        for offset in offsets {
            bytes.extend(7u32.to_le_bytes());
            bytes.extend(offset.to_le_bytes());
        }
        bytes.extend(b"EENT");
        bytes.resize(100, 0);
        Arc::new(TrackDataBlock::parse("same.hit".into(), bytes).unwrap())
    }
    let a = block(&[32]);
    let b = block(&[48, 64]);
    let mut manager = HitMan::default();
    manager.register_block(a.clone());
    manager.register_block(b.clone());
    let (found, offset) = manager.entry(7).unwrap();
    assert!(std::ptr::eq(found, b.as_ref()));
    assert_eq!(offset, 64);
    assert!(std::ptr::eq(
        found.entry_bytes(offset).unwrap().as_ptr(),
        found.data[64..].as_ptr()
    ));
    manager.register_block(a.clone());
    assert_eq!(manager.entry(7).unwrap().1, 32);
    assert_eq!(manager.blocks.len(), 2);
    assert!(!manager.register_semantic(0, Arc::new(12u32)));
    assert_eq!(manager.semantics[&0].downcast_ref::<u32>(), Some(&12));
    manager.initialized = true;
    manager.set_duck_priority(1, 5);
    manager.set_duck_priority(2, -2);
    assert_eq!(manager.duck_max(), 5);
    manager.set_duck_priority(1, 0);
    assert_eq!(
        (manager.duck_max(), manager.registers[&27], manager.view_updates),
        (0, 0, 3)
    );
    assert!(manager.pause());
    assert!(!manager.paused);
    assert!(manager.unpause());
    assert_eq!(manager.random(20, 10), 13);
    assert_eq!(manager.random_seed, 0xBE39_E1CC);
    let seed = manager.random_seed;
    assert_eq!(manager.random(7, 7), 7);
    assert_eq!(manager.random_seed, seed);
}

#[derive(Default)]
struct Backend {
    binds: usize,
    forwards: Vec<(u32, u32, f32)>,
    resolved: BTreeMap<u32, Arc<Properties>>,
}
impl ChannelBackend for Backend {
    fn create(&mut self, _: u32, _: u32) -> Option<SoundRef> {
        None
    }
    fn initialize(&mut self, _: &SoundRef) {}
    fn query_source(&mut self, _: &SoundRef, _: u32) -> bool {
        false
    }
    fn configure_3d(&mut self, _: &SoundRef, _: u32, _: &Properties) -> bool {
        false
    }
    fn refresh_source_state(&mut self, _: &SoundRef) {}
    fn bind_source(&mut self, _: &SoundRef, _: Option<&directors::ForeignRef>, mode: u32) -> bool {
        assert_eq!(mode, 2);
        self.binds += 1;
        true
    }
    fn optional_float(&mut self, _: &SoundRef, iid: u32, slot: u32, value: f32) -> bool {
        self.forwards.push((iid, slot, value));
        false
    }
    fn resolve_properties(&mut self, key: ResourceKey, iid: u32) -> Option<Arc<Properties>> {
        assert_eq!((key.kind, key.group, iid), (0xEBFE_E345, 0xFC45_CAF7, 0xEBB1_0CDD));
        self.resolved.get(&key.instance).cloned()
    }
}

#[test]
fn ss3_notifications_preserve_return_asymmetry_and_retained_sets() {
    let child = Arc::new(Source::default());
    let mut channel = HitChannel {
        source: Some(child),
        ..Default::default()
    };
    channel.properties.insert(0xFF09_9DB2, 22051);
    channel
        .float_properties
        .extend([(0xFF30_71B9, 0.5), (0xFF01_50F2, 1.25), (0xFF86_FE79, -0.5)]);
    let mut backend = Backend::default();
    assert!(channel.notify(0xFF09_9DB2, &mut backend)); // inverted pitch return is discarded
    assert!(channel.notify(0xFF01_50F2, &mut backend));
    assert!(!channel.notify(0xFF86_FE79, &mut backend));
    assert_eq!(backend.forwards, [(0x6B04_46EB, 6, 1.25), (0x4B79_C1A9, 7, -0.5)]);
    let parent = Arc::new([(0xFF53_DBF3, 512)].into_iter().collect());
    backend.resolved.insert(7, Arc::clone(&parent));
    let notifying = Arc::new([(0xFF4C_97BE, 7), (0xFF90_AAC8, -3)].into_iter().collect());
    assert!(channel.notify_inherited(Arc::clone(&notifying), &mut backend));
    assert_eq!(Arc::strong_count(&notifying), 2);
    assert_eq!(Arc::strong_count(&parent), 3);
    assert_eq!(channel.play_value as i32, -1);
    channel.notify_inherited(Arc::new(Properties::new()), &mut backend);
    assert_eq!(Arc::strong_count(&notifying), 1);
    assert_eq!(Arc::strong_count(&parent), 2);
    let retained: directors::ForeignRef = Arc::new(7u32);
    channel.retained_source = Some(Arc::clone(&retained));
    assert!(channel.notify(0xFFAE_C905, &mut backend));
    assert_eq!(Arc::strong_count(&retained), 1);
}

#[test]
fn ss1_instruction_budget_reports_after_102_and_preserves_stack_offsets() {
    let mut track = HitTrackPlayer {
        executing: true,
        ..Default::default()
    };
    let mut io = TrackSpy {
        error_result: 1,
        ..Default::default()
    };
    track.timer_callback(0, &[15; 103], &mut io).unwrap();
    assert_eq!(track.pc, 102);
    assert_eq!(
        io.errors,
        ["Too many instructions executed without wait (infinite loop?)"]
    );
    track.pc = 0;
    let stream = [7, 6, 0, 0, 0, 12, 8];
    track.execute_next(&stream, &mut io).unwrap();
    assert_eq!((track.pc, track.return_stack.clone()), (6, vec![5]));
    track.execute_next(&stream, &mut io).unwrap();
    assert_eq!(track.pc, 5);
    assert!(track.return_stack.is_empty());
}

#[test]
fn ss6_path_init_ignores_load_boolean_and_preserves_directory() {
    struct Paths(Vec<String>);
    impl PathScoreInput for Paths {
        fn load_path(&mut self, path: &str) -> (ScoreData, bool) {
            self.0.push(path.to_owned());
            (
                ScoreData {
                    rows: 1,
                    columns: 1,
                    cells: vec![CellData::default()],
                    ..Default::default()
                },
                false,
            )
        }
    }
    let mut player = FreshPlayer::default();
    let mut paths = Paths(Vec::new());
    player.init_path("Music/score.txt", &mut paths).unwrap();
    assert!(player.initialized);
    assert_eq!(player.data_directory, "Music/");
    assert_eq!(paths.0, ["Music/score.txt"]);
}

#[test]
fn ss3_pitch_pan_volume_patch_and_interface_worked_cases() {
    let child = Arc::new(Source::default());
    child.frequency.store(22051, Ordering::Relaxed);
    let mut channel = HitChannel {
        source: Some(child.clone()),
        ..Default::default()
    };
    channel.float_properties.insert(0xFF30_71B9, 0.5);
    assert!(channel.update_pitch());
    assert_eq!(channel.properties[&0xFF09_9DB2], 11025);
    assert!(child.calls().is_empty());
    channel.properties.insert(0xFF09_9DB2, 22051);
    assert!(!channel.update_pitch());
    assert_eq!(child.calls(), ["freq:11025"]);
    channel
        .properties
        .extend([(0xFF90_AAC8, -3), (0xFF53_DBF3, 512), (0xFF3E_C880, -20)]);
    channel.update_vol_pan();
    assert_eq!(channel.play_value as i32, -1);
    channel.properties.extend([(0xFFCC_156F, 1), (0xFF90_AAC8, 1024)]);
    channel.update_vol_pan();
    assert_eq!(channel.play_value, 512);
    assert!(child.calls().iter().any(|c| c == "max:100000"));
    channel.properties.insert(0xFFA6_A614, -1);
    channel.update_vol_pan();
    assert!(child.calls().iter().any(|c| c == "max:-1"));
    let mut backend = Backend::default();
    assert!(channel.update_patch(&mut backend));
    assert_eq!(backend.binds, 1);
    let mut empty = HitChannel::default();
    assert!(!empty.update_patch(&mut backend));
    assert_eq!(backend.binds, 1);
    let owner = Arc::new(Mutex::new(channel));
    for (iid, offset) in [
        (0x2060_77D2, 0),
        (0x3060_77D2, 4),
        (0x5B34_F0AF, 8),
        (0xFD58_4467, 12),
        (0xAA11_A64F, 16),
        (0x09D0_02D2, 20),
    ] {
        let view = HitChannel::query_interface(&owner, iid).unwrap();
        assert_eq!(view.offset, offset);
        assert_eq!(Arc::strong_count(&owner), 2);
    }
    assert!(HitChannel::query_interface(&owner, 1).is_none());
}

#[derive(Default)]
struct Ini {
    sections: BTreeMap<String, Vec<(String, String)>>,
}
impl IniFile for Ini {
    fn set_filename(&mut self, _: &str) {}
    fn read_entry(&mut self, section: &str, key: &str) -> Option<String> {
        self.enum_entries(section)
            .into_iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }
    fn enum_entries(&mut self, section: &str) -> Vec<(String, String)> {
        self.sections.get(section).cloned().unwrap_or_default()
    }
    fn unsigned_number(&mut self, text: &str) -> Result<u32, Unsupported> {
        Ok(unsigned_number(text))
    }
}
#[derive(Default)]
struct InitHost {
    actions: Vec<IniAction>,
}
impl IniServices for InitHost {
    fn has_manager(&self) -> bool {
        false
    }
    fn has_sound_system(&self) -> bool {
        false
    }
    fn set_default_player(&mut self, _: bool, _: u32) {}
    fn map_extension(&mut self, _: &str, _: u32) {}
    fn data_directory(&self) -> &str {
        "data/"
    }
    fn apply(&mut self, action: &IniAction) -> bool {
        self.actions.push(action.clone());
        false
    }
}
#[test]
fn ss4_numeric_order_duplicates_and_failure_continuation() {
    for (text, expected) in [
        ("12tail", 12),
        ("12AB", 0x12AB),
        ("-1", u32::MAX),
        ("xyz", 0),
        (" AB", 0),
        ("10A?", 0x10A),
        ("4294967296", u32::MAX),
    ] {
        assert_eq!(unsigned_number(text), expected, "{text}");
    }
    let mut ini = Ini::default();
    ini.sections.insert(
        "TrackDataBlocks".into(),
        [("16", "a.hit"), ("2", "b.hit"), ("0x10", "c.hit")]
            .map(|(k, v)| (k.into(), v.into()))
            .to_vec(),
    );
    ini.sections.insert(
        "DBSegmentDirectoryFiles".into(),
        vec![("7".into(), "0x2026960B:wav,0x00000001:bin".into())],
    );
    let mut file = HitIniFile::new(ini);
    let mut host = InitHost::default();
    assert_eq!(file.enum_default_entries(&mut host), Ok(()));
    assert_eq!(
        host.actions,
        [
            IniAction::DirectoryBind {
                group: 7,
                kind: 0x2026_960B,
                extension: "wav".into()
            },
            IniAction::TypeGroup {
                kind: 0x2026_960B,
                group: 7
            },
            IniAction::DirectoryBind {
                group: 7,
                kind: 1,
                extension: "bin".into()
            },
            IniAction::TypeGroup { kind: 1, group: 7 },
            IniAction::LoadTrack("data/b.hit".into()),
            IniAction::LoadTrack("data/c.hit".into())
        ]
    );
    assert!(file.init_resources(&mut host));
}

#[test]
fn ss5_legacy_framing_defaults_self_reference_writer_and_private_copy() {
    let key = ResourceKey {
        kind: 0x0B8D_821A,
        group: 7,
        instance: 42,
    };
    let mut resource = HitPropertySetTrackResource::new(key);
    let mut text = String::from("TKDT,0,,");
    for field in 1..=21 {
        if field != 6 {
            text.push_str(if field == 7 { "1," } else { "," });
        }
    }
    text.push_str("ETKD");
    let mut record = 0u32.to_le_bytes().to_vec();
    record.extend(text.as_bytes());
    resource.properties.insert(LEGACY_FIELDS[5], 99);
    assert!(resource.read_legacy_record(&record));
    assert_eq!(resource.properties[&LEGACY_FIELDS[8]], 0x7DCD_ED3D);
    assert_eq!(resource.properties[&LEGACY_FIELDS[5]], 99);
    let mut self_record = 0u32.to_le_bytes().to_vec();
    self_record.extend(b"TKDT,2,,42,");
    assert!(!resource.read_legacy_record(&self_record));
    assert_eq!(resource.properties[&0xFF6F_9546], 42);
    resource.properties.clear();
    resource.properties.insert(LEGACY_FIELDS[11], -1);
    let text = resource.legacy_text();
    let tokens: Vec<_> = text.split(',').collect();
    assert_eq!(tokens[14], "-1");
    assert_eq!(tokens.len(), 31);
    assert!(resource.write_legacy(|bytes| {
        assert_eq!(bytes, text.as_bytes());
        false
    }));
    resource.reference_count = 5;
    resource.cursor_word = 123;
    resource.properties.insert(LEGACY_FIELDS[11], 512);
    let mut copy = resource.private_copy().unwrap();
    copy.properties.insert(LEGACY_FIELDS[11], 256);
    assert_eq!(resource.properties[&LEGACY_FIELDS[11]], 512);
    assert_eq!(copy.key, key);
    assert_eq!(
        (copy.reference_count, copy.cursor_word, copy.legacy_tables),
        (6, 123, true)
    );
}

#[derive(Default)]
struct FreshSpy {
    source: Option<SoundRef>,
    draw: u32,
    c_draws: usize,
    start_ok: bool,
    events: Vec<String>,
    seeds: Vec<u32>,
}
impl CellHost for FreshSpy {
    fn unregister_source_update_if_available(&mut self, _: &SoundRef) {
        self.events.push("source-unregister".into());
    }
    fn fade_to_zero(&mut self, _: &SoundRef, duration: u32) {
        self.events.push(format!("fade:{duration}"));
    }
    fn sound_system_available(&self) -> bool {
        true
    }
    fn sound_by_id(&mut self, id: u32) -> Option<SoundRef> {
        self.events.push(format!("id:{id:08x}"));
        self.source.clone()
    }
    fn sound_by_name(&mut self, name: &str) -> Option<SoundRef> {
        self.events.push(format!("name:{name}"));
        self.source.clone()
    }
    fn private_draw(&mut self) -> u32 {
        self.draw
    }
    fn crc24_filename(&mut self, _: &str) -> u32 {
        0x123456
    }
    fn pitch_ratio(&mut self, index: usize) -> Option<f64> {
        Some(if index == 0 { 1.0 } else { 2.0 })
    }
    fn register_source_update_if_available(&mut self, _: &SoundRef, period: u32) {
        self.events.push(format!("source-register:{period}"));
    }
    fn external_start(&mut self, cell: &CellData, id: u32, head: bool, looping: bool, fade: f32) -> bool {
        self.events
            .push(format!("external:{}:{id:08x}:{head}:{looping}:{fade}", cell.name));
        self.start_ok
    }
    fn external_position(&mut self, _: &CellData, _: [f32; 3]) {
        self.events.push("position".into());
    }
    fn external_distances(&mut self, _: &CellData, _: [f32; 2]) {
        self.events.push("distances".into());
    }
    fn external_volume(&mut self, _: &CellData, _: f32) {
        self.events.push("volume".into());
    }
    fn external_pitch(&mut self, _: &CellData, pitch: f32) {
        self.events.push(format!("pitch:{pitch}"));
    }
    fn external_stop(&mut self, _: &CellData, fade: f32) {
        self.events.push(format!("external-stop:{fade}"));
    }
    fn external_playing(&mut self, _: &CellData) -> bool {
        false
    }
    fn lifecycle(&mut self, event: FreshLifecycle) {
        self.events.push(format!("{event:?}"));
    }
}
impl FreshHost for FreshSpy {
    fn c_rand(&mut self) -> i32 {
        self.c_draws += 1;
        0
    }
    fn reseed_private(&mut self, seed: u32) {
        self.seeds.push(seed);
    }
    fn random_below(&mut self, upper: u32) -> u32 {
        self.draw % upper
    }
}
fn fresh_grid(rows: usize, columns: usize) -> FreshPlayer {
    let mut player = FreshPlayer::default();
    let score = ScoreData {
        cells: (0..rows * columns)
            .map(|id| CellData {
                id: id as u64,
                volume_word: 1024,
                class: 20,
                ..Default::default()
            })
            .collect(),
        volume_word: 1024,
        rows,
        columns,
        half: [1, 1],
        tempo: 120,
        beats_per_bar: 8,
        ..Default::default()
    };
    player.init_loaded_score(score).unwrap();
    player
}

#[test]
fn ss6_block_coordinate_selection_scores_and_separate_random_sources() {
    let mut player = fresh_grid(9, 9);
    player.block_mode = true;
    player.set_x_position(500);
    player.set_y_position(500);
    assert_eq!(player.coordinates, [4, 4]);
    assert_eq!((player.selectable_count, player.distinct_groups), (9, 9));
    let first = 3 * 9 + 3;
    assert_eq!(
        player.best_start(0, 0, || panic!("class20 must not draw C rand")),
        Some(first)
    );
    player.cells[first].cell.as_mut().unwrap().class = 1;
    player.selectable_count = 10;
    assert_eq!(player.start_score(first, 0, || 59), 1);
    assert_eq!(player.start_score(first, 0, || 60), 0);
    player.cells[first].cell.as_mut().unwrap().class = 0;
    assert_eq!(player.start_score(first, 0, || panic!()), 0);
    let mut host = FreshSpy::default();
    player.reseed(77, None, &mut host);
    assert_eq!(host.seeds, [77]);
    assert_eq!(host.c_draws, 0);
    assert_eq!((player.beat_in_bar, player.next_beat, player.origin), (0, 1, 0));
    player.cells[0].state = 2;
    assert_eq!(player.cell_to_subtract(0, || panic!("outside chosen first")), Some(0));
}

#[test]
fn ss6_fade_overshoot_retarget_zero_step_and_clock_worked_cases() {
    let mut player = fresh_grid(1, 1);
    let mut host = FreshSpy::default();
    player.play(None);
    player.volume = 100;
    assert!(player.fade_volume(400, 145, true, None, &mut host));
    assert_eq!((player.fade.unwrap().step, player.fade.unwrap().period), (2, 20));
    for _ in 0..22 {
        player.on_fade(None, &mut host);
    }
    assert_eq!(player.volume, 144);
    assert!(player.fade.is_some());
    player.on_fade(None, &mut host);
    assert_eq!(player.volume, 146);
    assert!(player.fade.is_none());
    assert!(player.playing);
    player.volume = 100;
    player.fade_volume(400, 145, false, None, &mut host);
    player.fade_volume(1000, 40, false, None, &mut host);
    assert_eq!(player.fade.unwrap().period, 20);
    player.fade_volume(1, 110, false, None, &mut host);
    assert_eq!(player.volume, 110);
    assert!(player.fade.is_some());
    player.tempo = 0;
    assert_eq!(player.beat_due(5), -1);
    let next = player.next_beat;
    player.periodic_update(1000, &mut host);
    assert_eq!(player.next_beat, next);
    player.tempo = 120;
    player.origin = 0;
    player.next_beat = 1;
    player.beat_in_bar = 1;
    player.manual = true;
    player.periodic_update(500, &mut host);
    assert_eq!((player.next_beat, player.beat_in_bar), (2, 2));
    player.periodic_update(86_400_001, &mut host);
    assert_eq!((player.origin, player.next_beat, player.beat_in_bar), (8, 8, 8));
    assert!(host.events.contains(&"TimerRestart".into()));
    player.volume = 100;
    player.fade_volume(1, 110, true, None, &mut host);
    assert!(!player.playing);
}

struct ScheduleEvents(Arc<Mutex<Vec<String>>>);
impl Scheduler for ScheduleEvents {
    fn register(&mut self, period: u32) {
        self.0.lock().unwrap().push(format!("register:{period}"));
    }
    fn unregister(&mut self) {
        self.0.lock().unwrap().push("unregister".into());
    }
}
#[test]
fn ss6_stop_shutdown_and_repeated_init_lifetime_order() {
    let mut player = fresh_grid(1, 1);
    let source = Arc::new(Source::default());
    player.play(None);
    player.cells[0].source = Some(source);
    player.cells[0].state = 2;
    let mut host = FreshSpy::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut schedule = ScheduleEvents(events.clone());
    player.stop(Some(&mut schedule), &mut host);
    assert_eq!(*events.lock().unwrap(), ["unregister"]);
    assert_eq!(&host.events[..3], ["FadeEnd", "TimerStop", "source-unregister"]);
    player.shutdown(None, &mut host);
    let score_delete = host.events.iter().position(|e| e == "ScoreDelete").unwrap();
    let player_delete = host.events.iter().position(|e| e == "PlayerDelete(0)").unwrap();
    assert!(score_delete < player_delete);
    assert!(player.score.is_none());
    assert!(player.cells.is_empty());
    assert!(!player.initialized);
    let mut player = fresh_grid(1, 1);
    player.init_loaded_score(ScoreData::default()).unwrap(); // initialized grids are not rebuilt
    assert_eq!(player.cells.len(), 1);
}

#[test]
fn ss7_guid_external_failure_countdown_pan_and_controller_worked_cases() {
    assert!(FreshCellPlayer::filename_has_guid("amb_1234ABCD.wav"));
    assert!(!FreshCellPlayer::filename_has_guid("1234-ABCD.wav"));
    let mut host = FreshSpy::default();
    let mut player = FreshCellPlayer::new(Arc::new(()));
    player.external_controller = true;
    player.set_cell(CellData {
        name: "amb_1234ABCD.wav".into(),
        volume_word: 1024,
        loop_length: 1,
        looping: true,
        ..Default::default()
    });
    assert!(!player.start(&mut host, 1024, 1024));
    assert!(player.is_busy());
    assert_eq!(player.cell.as_ref().unwrap().id, 0x1234_ABCD);
    assert!(!player.start(&mut host, 1024, 1024));
    assert_eq!(host.events.len(), 1);
    player.stop_flag = true;
    player.update(&mut host);
    assert_eq!((player.state, player.counter), (2, 0));
    player.update(&mut host);
    assert_eq!(player.state, 0);
    player.cell.as_mut().unwrap().pan = [768, 512];
    assert_eq!(player.auto_pan(&mut host, false, None), Some([50.0, 0.0, 0.0]));
    player.cell.as_mut().unwrap().pan = [512, 512];
    player.cell.as_mut().unwrap().random_pan = true;
    host.draw = 2;
    assert_eq!(player.auto_pan(&mut host, false, None), Some([-100.0, 0.0, 0.0]));
    player.state = 2;
    player.cell.as_mut().unwrap().looping = false;
    host.events.clear();
    player.update(&mut host);
    assert_eq!(player.state, 0);
    assert_eq!(host.events, ["external-stop:0"]);
}

#[test]
fn ss7_internal_start_ignored_play_failure_and_external_setter_order() {
    let source = Arc::new(Source::default());
    source.frequency.store(22051, Ordering::Relaxed);
    let mut host = FreshSpy {
        source: Some(source.clone()),
        start_ok: true,
        ..Default::default()
    };
    let mut player = FreshCellPlayer::new(Arc::new(()));
    player.set_cell(CellData {
        name: "amb_1234ABCD.wav".into(),
        volume_word: 1024,
        ..Default::default()
    });
    assert!(player.start(&mut host, 1024, 1024));
    assert!(player.is_playing());
    assert_eq!(host.events, ["id:1234abcd", "source-register:250"]);
    assert!(source.calls().iter().any(|e| e.starts_with("play:")));
    player.kill();
    player.external_controller = true;
    player.cell.as_mut().unwrap().pitch = [-1, -1];
    host.events.clear();
    assert!(player.start(&mut host, 1024, 1024));
    assert_eq!(&host.events[1..], ["position", "distances", "volume", "pitch:2"]);
    player.cell.as_mut().unwrap().fade_out = 5;
    player.stop(&mut host).unwrap();
    assert_eq!(player.state, 3);
    player.update(&mut host);
    assert_eq!(player.state, 0);
}
