// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::audio::snd_event::SndEventStd;
use directors::*;
use framework::*;
use fresh::*;
use hit::*;
use playlist::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use track::*;

#[derive(Default)]
struct Child {
    load_ok: bool,
    playing: AtomicBool,
    calls: Mutex<Vec<String>>,
}
impl Child {
    fn record(&self, call: impl Into<String>) {
        self.calls.lock().unwrap().push(call.into());
    }
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}
impl Sound for Child {
    fn load(&self) -> bool {
        self.record("load");
        self.load_ok
    }
    fn unload(&self) -> bool {
        self.record("unload");
        false
    }
    fn is_playing(&self) -> bool {
        self.playing.load(Ordering::SeqCst)
    }
    fn pause(&self) {
        self.record("pause");
    }
    fn unpause(&self) {
        self.record("unpause");
    }
    fn play(&self, p: u32, looping: bool, flag: bool) -> bool {
        self.record(format!("play:{p}:{looping}:{flag}"));
        true
    }
    fn set_volume(&self, volume: i32) -> bool {
        self.record(format!("volume:{volume}"));
        false
    }
}
#[derive(Default)]
struct Schedule(Vec<u32>);
impl Scheduler for Schedule {
    fn register(&mut self, period: u32) {
        self.0.push(period);
    }
    fn unregister(&mut self) {
        self.0.push(0);
    }
}

#[test]
fn ss_v01_nested_loads_ignore_child_failure() {
    let child = Arc::new(Child::default());
    let mut track = HitTrackPlayer {
        current: Some(child.clone()),
        ..Default::default()
    };
    assert!(track.load());
    assert!(track.load());
    assert!(track.unload());
    assert!(track.unload());
    assert!(!track.is_loaded());
    assert_eq!(child.calls(), ["load", "unload"]);
}
#[test]
fn ss_v02_unload_underflows() {
    let mut track = HitTrackPlayer::default();
    assert!(track.unload());
    assert!(track.is_loaded());
    assert!(track.load());
    assert!(!track.is_loaded());
}
#[test]
fn ss_v03_signed_register_check() {
    let mut errors = Vec::new();
    for (input, expected) in [(74, 74), (75, 1), (-1, -1)] {
        assert_eq!(
            HitTrackPlayer::checked_reg_id(input, &mut |s| errors.push(s.to_owned())),
            expected
        );
    }
    assert_eq!(errors, ["Illegal Register Id"]);
}
#[test]
fn ss_v04_dependents_outlive_execution() {
    let child = Arc::new(Child::default());
    child.playing.store(true, Ordering::SeqCst);
    let track = HitTrackPlayer {
        dependents: vec![child],
        ..Default::default()
    };
    assert!(!track.executing);
    assert!(track.is_playing());
}
#[test]
fn ss_v05_repeat_play_does_not_restart() {
    let mut track = HitTrackPlayer {
        executing: true,
        ..Default::default()
    };
    track.properties.insert(0xFF30_3B1B, 0);
    assert_eq!(track.play_one(9), Ok(true));
    assert!(track.property_notifications.is_empty());
    track.properties.insert(0xFF30_3B1B, 1);
    assert_eq!(
        track.prepare_play([9, 0, 0, 0]),
        PlayPreparation::Execute {
            kill_first: true,
            routing_mode: None,
            payload: [9, 0, 0, 0]
        }
    );
    assert_eq!(track.play_one(9), Err(Unsupported("SS1")));
}

#[derive(Default)]
struct Backend {
    child: Option<SoundRef>,
    source_qi: bool,
    spatial_qi: bool,
    calls: Vec<String>,
}
impl ChannelBackend for Backend {
    fn create(&mut self, class: u32, iid: u32) -> Option<SoundRef> {
        self.calls.push(format!("create:{class}:{iid}"));
        self.child.clone()
    }
    fn initialize(&mut self, _: &SoundRef) {
        self.calls.push("initialize".into());
    }
    fn query_source(&mut self, _: &SoundRef, iid: u32) -> bool {
        self.calls.push(format!("qi:{iid}"));
        self.source_qi
    }
    fn configure_3d(&mut self, _: &SoundRef, iid: u32, _: &Properties) -> bool {
        self.calls.push(format!("3d:{iid}"));
        self.spatial_qi
    }
    fn refresh_source_state(&mut self, _: &SoundRef) {
        self.calls.push("refresh".into());
    }
}
fn channel_with(child: Option<SoundRef>) -> HitChannel {
    let mut channel = HitChannel::new(Arc::new(SndMgr::default()));
    channel.source = child;
    channel
}
#[test]
fn ss_v06_channel_note_on_loop() {
    let child = Arc::new(Child {
        load_ok: true,
        ..Default::default()
    });
    let mut channel = channel_with(Some(child.clone()));
    channel.properties.insert(0xFFF7_B8DF, -1);
    let mut backend = Backend::default();
    assert_eq!(channel.note_on_after_patch(77, Ok(()), &mut backend), Ok(true));
    assert_eq!(child.calls(), ["load", "play:77:true:true"]);
    assert_eq!(backend.calls, ["refresh"]);
}
#[test]
fn ss_v07_channel_failed_load_never_plays() {
    let child = Arc::new(Child::default());
    let mut channel = channel_with(Some(child.clone()));
    assert_eq!(
        channel.note_on_after_patch(77, Ok(()), &mut Backend::default()),
        Ok(false)
    );
    assert_eq!(child.calls(), ["load"]);
}
#[test]
fn ss_v08_free_channel_releases_once() {
    let child: SoundRef = Arc::new(Child::default());
    let mut hit = HitMan::default();
    let output = hit.alloc_channel(|| Some(child.clone())).unwrap();
    assert_eq!(Arc::strong_count(&child), 3);
    hit.free_channel(&output);
    assert_eq!(Arc::strong_count(&child), 2);
    hit.free_channel(&output);
    assert_eq!(Arc::strong_count(&child), 2);
    assert_eq!(hit.channel_count(), 0);
    assert!(hit.alloc_channel(|| None).is_none());
}
#[test]
fn ss_v09_uninitialized_freshness_success_without_work() {
    let mut player = FreshPlayer::default();
    let mut scheduler = Schedule::default();
    assert!(player.play(Some(&mut scheduler)));
    assert!(player.set_zoom(9));
    assert!(player.set_play_percent(25));
    assert!(!player.playing);
    assert_eq!(player.zoom, None);
    assert_eq!(player.play_percent, None);
    assert!(scheduler.0.is_empty());
}
#[test]
fn ss_v10_cell_state_predicates() {
    let mut cell = FreshCellPlayer::new(Arc::new(()));
    for (state, playing, busy) in [(0, false, false), (2, true, true), (3, false, true)] {
        cell.state = state;
        assert_eq!(cell.is_playing(), playing);
        assert_eq!(cell.is_busy(), busy);
    }
}
#[test]
fn ss_v11_kill_releases_sound() {
    let child: SoundRef = Arc::new(Child::default());
    let mut cell = FreshCellPlayer::new(Arc::new(()));
    cell.source = Some(child.clone());
    cell.state = 3;
    cell.stop_flag = true;
    assert_eq!(Arc::strong_count(&child), 2);
    assert!(cell.kill());
    assert_eq!(Arc::strong_count(&child), 1);
    assert_eq!(cell.state, 0);
    assert!(!cell.stop_flag);
}
fn scaled_vector(c: i32, a: i32, p: i32, s: i32, l: i32, expected: i32) {
    let mut cell = FreshCellPlayer::new(Arc::new(()));
    cell.local_volume = l;
    assert_eq!(cell.update_scaled_volume(c, a, p, s), expected);
    assert_eq!(cell.intermediate_volume, expected);
}
#[test]
fn ss_v12_scaled_unity() {
    scaled_vector(1024, 1024, 1024, 1024, 1024, 1024);
}
#[test]
fn ss_v13_scaled_fraction() {
    scaled_vector(513, 1024, 1024, 1024, 1024, 513);
}
#[test]
fn ss_v14_scaled_signed_truncation() {
    scaled_vector(-1, 1, 1024, 1024, 1024, 0);
}

struct Handler {
    name: &'static str,
    answer: bool,
    trace: Arc<Mutex<Vec<&'static str>>>,
}
impl EventHandler for Handler {
    fn do_event(&self, _: &mut SndSys, _: &SndEventStd) -> bool {
        self.trace.lock().unwrap().push(self.name);
        self.answer
    }
}
#[test]
fn ss_v15_event_chain_consumes() {
    let trace = Arc::new(Mutex::new(Vec::new()));
    let mut system = SndSys::default();
    for (name, answer) in [("C", true), ("B", true), ("A", false)] {
        system.register_handler(Arc::new(Handler {
            name,
            answer,
            trace: trace.clone(),
        }));
    }
    assert!(system.do_event(&SndEventStd::new()));
    assert_eq!(*trace.lock().unwrap(), ["A", "B"]);
}
#[test]
fn ss_v16_unconsumed_event_is_false() {
    let mut system = SndSys::default();
    assert!(!system.do_event(&SndEventStd::new()));
    system.register_handler(Arc::new(Handler {
        name: "A",
        answer: false,
        trace: Arc::new(Mutex::new(Vec::new())),
    }));
    assert!(!system.do_event(&SndEventStd::new()));
    assert!(!system.do_payload(1, [2, 3, 4, 5], || None));
}
struct Target(Arc<AtomicUsize>, bool);
impl UpdateTarget for Target {
    fn update(&self, _: &mut SndSys) -> bool {
        self.0.fetch_add(1, Ordering::SeqCst);
        self.1
    }
}
#[test]
fn ss_v17_updates_can_be_disabled() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut system = SndSys::default();
    system.register_classified_target(Arc::new(Target(calls.clone(), true)), TargetClass::Ordinary);
    system.disable_updates();
    system.enumerate_targets(TargetClass::Ordinary);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    system.enabled_flag = Some(false); // Independent control must not suppress EnableUpdates.
    system.enable_updates();
    system.enumerate_targets(TargetClass::Ordinary);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
#[derive(Default)]
struct ManagerHost {
    available: bool,
    acquisitions: usize,
    registrations: Vec<(u32, u32)>,
    retains: usize,
}
impl ManagerServices for ManagerHost {
    fn acquire_sound_system(&mut self) -> bool {
        self.acquisitions += 1;
        self.available
    }
    fn register_manager_update(&mut self, offset: u32, period: u32) {
        self.registrations.push((offset, period));
    }
    fn retain_persist_manager(&mut self) {
        self.retains += 1;
    }
}
#[test]
fn ss_v18_manager_preserves_partial_init() {
    let mut manager = SndMgr::default();
    let mut host = ManagerHost::default();
    assert!(!manager.init(&mut host));
    host.available = true;
    assert!(manager.init(&mut host));
    assert_eq!(host.acquisitions, 1);
    assert!(host.registrations.is_empty());
    assert!(!manager.sound_service_acquired);
    assert_eq!(host.retains, 0);
}
#[test]
fn ss_v19_missing_system_sound_is_zero() {
    let mut manager = SndMgr::default();
    assert_eq!(manager.system_sound_id(1), 0);
    manager.set_system_sound_id(1, 22);
    assert_eq!(manager.system_sound_id(1), 22);
    assert_eq!(manager.tagged_sound_id(0x12345678), 0xFF345678);
}
#[test]
fn ss_v20_freshness_director_singleton() {
    let constructions = AtomicUsize::new(0);
    let a = get_freshness_director(|classes| {
        assert_eq!(classes, FRESH_CLASSES);
        constructions.fetch_add(1, Ordering::SeqCst);
    });
    for _ in 0..3 {
        let b = get_freshness_director_alt(|_| {
            constructions.fetch_add(1, Ordering::SeqCst);
        });
        assert!(std::ptr::eq(a, b));
    }
    assert_eq!(constructions.load(Ordering::SeqCst), 1);
    assert_eq!(a.classes, FRESH_CLASSES);
}
struct Text(Vec<String>);
impl Text {
    fn from(text: &str) -> Self {
        Self(text.lines().map(str::to_owned).collect())
    }
}
impl PlaylistInput for Text {
    fn converted_lines(&mut self) -> Result<Vec<String>, Unsupported> {
        Ok(self.0.clone())
    }
    // Fixture converter emulates C decimal prefix conversion for these inputs.
    fn decimal_long(&mut self, text: &str) -> i32 {
        text.split(',').next().unwrap().trim().parse().unwrap_or(0)
    }
}
#[test]
fn ss_v21_extended_eof_between_records_succeeds() {
    let mut list = PlayList::default();
    assert_eq!(
        list.parse_converted("C:/Music/", &mut Text::from("#EXTM3U\n#EXTINF:12,Title\n song.mp3 \n")),
        Ok(true)
    );
    assert_eq!(
        list.entries,
        [Entry {
            title: "Title".into(),
            path: "C:/Music/song.mp3".into(),
            duration: 12
        }]
    );
}
#[test]
fn ss_v22_extended_bad_record_keeps_previous_entries() {
    for tail in ["#EXTINF:3,Missing path", "#comment"] {
        let mut list = PlayList::default();
        let text = format!("#EXTM3U\n#EXTINF:1,First\na.mp3\n{tail}");
        assert_eq!(list.parse_converted("C:/Music", &mut Text::from(&text)), Ok(false));
        assert_eq!(list.entries.len(), 1);
    }
}
#[test]
fn ss_v23_plain_keeps_first_line() {
    let mut list = PlayList::default();
    assert_eq!(
        list.parse_converted("C:/Music/", &mut Text::from(" first.mp3 ")),
        Ok(true)
    );
    assert_eq!(list.entries[0].path, "C:/Music/first.mp3");
    assert_eq!(PlayList::select_format("[playlist]"), (Format::Pls, 1));
}
#[derive(Default)]
struct ResourceIo(Vec<ResourceFormat>);
impl TrackResourceIo for ResourceIo {
    fn read(&mut self, format: ResourceFormat, properties: &mut Properties) -> Result<bool, Unsupported> {
        assert!(properties.contains_key(&0xFF6F_9546));
        self.0.push(format);
        Ok(true)
    }
    fn write(&mut self, format: ResourceFormat, _: &Properties) -> Result<bool, Unsupported> {
        self.0.push(format);
        Ok(true)
    }
}
#[test]
fn ss_v24_resource_write_and_read_are_asymmetric() {
    let key = ResourceKey {
        kind: 99,
        group: 7,
        instance: 0xFFFFFFFF,
    };
    let mut resource = HitPropertySetTrackResource::new(key);
    let mut io = ResourceIo::default();
    assert_eq!(resource.write(&mut io), Ok(false));
    assert!(io.0.is_empty());
    assert_eq!(resource.read(&mut io), Ok(true));
    assert_eq!(io.0, [ResourceFormat::Properties]);
    assert_eq!(resource.properties[&0xFF6F_9546], -1);
    resource.set_type(0x0B8D_821A);
    assert_eq!(resource.write(&mut io), Ok(true));
    assert_eq!(resource.read(&mut io), Ok(true));
    resource.set_type(0x0B9E_B87E);
    assert_eq!(resource.write(&mut io), Ok(true));
    assert_eq!(
        io.0,
        [
            ResourceFormat::Properties,
            ResourceFormat::Legacy,
            ResourceFormat::Legacy,
            ResourceFormat::Properties
        ]
    );
}

#[test]
fn track_views_acquire_references_and_preserve_secondary_offsets() {
    let track = Arc::new(Mutex::new(HitTrackPlayer::default()));
    for (iid, offset) in [
        (1, 0),
        (0xBB1BE367, 0),
        (0xFD584467, 4),
        (0x306077D2, 8),
        (0x206077D2, 12),
        (0xAA11A64F, 16),
        (0x5B34F0AF, 20),
        (0xDD583E2D, 24),
        (0xE9D00A9A, 28),
    ] {
        let view = HitTrackPlayer::query_interface(&track, iid).unwrap();
        assert_eq!(view.offset, offset);
        assert_eq!(Arc::strong_count(&track), 2);
        drop(view);
        assert_eq!(Arc::strong_count(&track), 1);
    }
    assert!(HitTrackPlayer::query_interface(&track, 9).is_none());
    assert_eq!(Arc::strong_count(&track), 1);
    let manager = Arc::new(Mutex::new(SndMgr::default()));
    assert_eq!(
        SndMgr::query_manager_interface(&manager, 0x3AF85BB7).unwrap().offset,
        0x1C
    );
}
#[test]
fn track_pause_properties_and_payload_notifications() {
    let source = Arc::new(Child::default());
    let dependent = Arc::new(Child::default());
    let mut track = HitTrackPlayer {
        current: Some(source.clone()),
        dependents: vec![dependent.clone()],
        executing: true,
        ..Default::default()
    };
    let mut scheduler = Schedule::default();
    track.pause(false, &mut scheduler);
    assert!(track.paused);
    assert!(source.calls().is_empty());
    assert!(track.unpause(&mut scheduler));
    track.pause(true, &mut scheduler);
    track.error = 7;
    assert!(!track.unpause(&mut scheduler));
    assert_eq!(source.calls(), ["pause", "unpause"]);
    assert_eq!(dependent.calls(), ["pause", "unpause", "pause", "unpause"]);
    assert_eq!(scheduler.0, [0, 20, 0, 20]);
    for (property, value) in [(0x13, 3), (0x15, 4), (0x16, 5), (0x49, 6)] {
        track.properties.insert(property, value);
    }
    assert_eq!(
        (
            track.priority(),
            track.volume(),
            track.pan(),
            track.original_frequency()
        ),
        (Some(3), Some(4), Some(5), Some(6))
    );
    track.properties.insert(0x15, 88);
    assert_eq!(track.volume(), Some(88));
    track.executing = false;
    assert_eq!(track.play_one(0xFFFFFFFF), Err(Unsupported("SS1")));
    assert_eq!(
        track.property_notifications,
        PAYLOAD_PROPERTIES.into_iter().zip([-1, 0, 0, 0]).collect::<Vec<_>>()
    );
}
#[test]
fn channel_creation_requires_source_qi_and_selects_3d() {
    let child: SoundRef = Arc::new(Child::default());
    let mut manager = SndMgr::default();
    manager.set_default_2d(21);
    manager.set_default_3d(31);
    let mut channel = HitChannel::new(Arc::new(manager));
    let mut backend = Backend {
        child: Some(child),
        ..Default::default()
    };
    assert!(!channel.create_sound(&mut backend));
    assert!(channel.source.is_none());
    backend.source_qi = true;
    assert!(channel.create_sound(&mut backend));
    assert_eq!(backend.calls[0], format!("create:21:{}", 0x206077D2));
    channel.source = None;
    channel.properties.insert(0xFFCC156F, 1);
    backend.spatial_qi = true;
    assert!(channel.create_sound(&mut backend));
    assert!(backend.calls.contains(&format!("create:31:{}", 0x206077D2)));
    assert!(backend.calls.contains(&format!("3d:{}", 0x306077D2)));
    let mut empty = channel_with(None);
    assert_eq!(empty.load_after_patch(Ok(()), &mut backend), Ok(true));
    assert_eq!(empty.note_on_after_patch(0, Ok(()), &mut backend), Ok(true));
    assert_eq!(empty.note_on(0, &mut backend), Err(Unsupported("SS3")));
}

#[derive(Default)]
struct Ini {
    entries: BTreeMap<(String, String), String>,
    requests: usize,
    filename: String,
}
impl IniFile for Ini {
    fn set_filename(&mut self, path: &str) {
        self.filename = path.to_owned();
    }
    fn read_entry(&mut self, section: &str, key: &str) -> Option<String> {
        self.requests += 1;
        self.entries.get(&(section.into(), key.into())).cloned()
    }
    fn enum_entries(&mut self, section: &str) -> Vec<(String, String)> {
        self.requests += 1;
        self.entries
            .iter()
            .filter(|((s, _), _)| s == section)
            .map(|((_, k), v)| (k.clone(), v.clone()))
            .collect()
    }
    fn unsigned_number(&mut self, text: &str) -> Result<u32, Unsupported> {
        text.parse().map_err(|_| Unsupported("SS4"))
    }
}
#[derive(Default)]
struct IniHost {
    manager: bool,
    sound: bool,
    players: Vec<(bool, u32)>,
    extensions: Vec<(String, u32)>,
}
impl IniServices for IniHost {
    fn has_manager(&self) -> bool {
        self.manager
    }
    fn has_sound_system(&self) -> bool {
        self.sound
    }
    fn set_default_player(&mut self, spatial: bool, class: u32) {
        self.players.push((spatial, class));
    }
    fn map_extension(&mut self, extension: &str, class: u32) {
        self.extensions.push((extension.into(), class));
    }
}
#[test]
fn ini_delegation_defaults_and_missing_services() {
    let mut ini = HitIniFile { ini: Ini::default() };
    for (section, key, value) in [
        ("Config", "DefaultSndDataSourcePlayer2DClassId", "12"),
        ("Config", "DefaultSndDataSourcePlayer3DClassId", "13"),
        ("SndFileDefaultDataSourceClassIds", "xa", "14"),
    ] {
        ini.ini.entries.insert((section.into(), key.into()), value.into());
    }
    ini.set_filename("Hit.ini");
    assert_eq!(ini.ini.filename, "Hit.ini");
    assert_eq!(ini.enum_default_entries(&mut IniHost::default()), Ok(()));
    assert_eq!(ini.ini.requests, 0);
    let mut host = IniHost {
        manager: true,
        sound: true,
        ..Default::default()
    };
    assert_eq!(ini.enum_default_entries(&mut host), Ok(()));
    assert_eq!(host.players, [(false, 12), (true, 13)]);
    assert_eq!(host.extensions, [("xa".into(), 14)]);
    assert_eq!(ini.read_entry("Config", "missing"), None);
    assert_eq!(
        ini.enum_entries("SndFileDefaultDataSourceClassIds"),
        [("xa".into(), "14".into())]
    );
}

fn score() -> ScoreData {
    ScoreData {
        cells: vec![CellData {
            id: 17,
            volume_word: 513,
            fade_out: 0,
        }],
        volume_word: 1024,
        starting_coordinates: [2, 3],
        settings: [(9, 8)].into_iter().collect(),
    }
}
#[test]
fn fresh_initialization_play_and_lock_contention() {
    let mut player = FreshPlayer::default();
    player.init_loaded_score(score()).unwrap();
    assert_eq!(player.cells.len(), 1);
    assert_eq!(player.cells[0].cell.as_ref().unwrap().id, 17);
    let mut scheduler = Schedule::default();
    assert!(player.play(Some(&mut scheduler)));
    assert!(player.play(Some(&mut scheduler)));
    assert_eq!(scheduler.0, [250]);
    assert_eq!(player.coordinates, [2, 3]);
    assert!(player.timer_running);
    assert!(player.set_zoom(9));
    assert!(player.set_play_percent(25));
    assert_eq!(player.zoom, Some(9));
    player.cells[0].state = 3;
    assert_eq!(player.prob_of_cell_starting(), 1);
    assert!(player.update_volume(512, None));
    assert_eq!(player.volume, 512);
    let shared = Arc::new(Mutex::new(player));
    let guard = shared.lock().unwrap();
    assert!(!try_callback(&shared, |_| panic!("must skip")));
    drop(guard);
    assert!(try_callback(&shared, |player| player.volume = 1));
    assert_eq!(shared.lock().unwrap().volume, 1);
}
#[derive(Default)]
struct CellSpy {
    unregisters: usize,
    fades: Vec<u32>,
}
impl CellHost for CellSpy {
    fn unregister_source_update_if_available(&mut self, _: &SoundRef) {
        self.unregisters += 1;
    }
    fn fade_to_zero(&mut self, _: &SoundRef, duration: u32) {
        self.fades.push(duration);
    }
}
#[test]
fn cell_stop_fade_poll_and_scaled_forwarding() {
    let child = Arc::new(Child::default());
    let mut host = CellSpy::default();
    let mut cell = FreshCellPlayer::new(Arc::new(()));
    cell.source = Some(child.clone());
    cell.state = 2;
    cell.set_cell(CellData {
        fade_out: 11,
        ..Default::default()
    });
    assert_eq!(cell.state, 2);
    cell.stop(&mut host).unwrap();
    assert_eq!(cell.state, 3);
    assert_eq!(host.fades, [11]);
    cell.stop(&mut host).unwrap();
    assert_eq!(host.fades, [11]);
    assert_eq!(host.unregisters, 2);
    cell.update_source(false).unwrap();
    assert_eq!(cell.state, 0);
    assert!(!cell.set_volume(1024, 513, 1024, 1024, 1024));
    assert_eq!(child.calls(), ["volume:513"]);
    cell.state = 2;
    cell.set_cell(CellData::default());
    cell.stop(&mut host).unwrap();
    assert!(cell.source.is_none());
    assert!(cell.set_volume(17, 1, 1, 1, 1));
    cell.state = 2;
    cell.external_controller = true;
    assert_eq!(cell.stop(&mut host), Err(Unsupported("SS7")));
    assert_eq!(cell.state, 2);
    // Overflow is low-32-bit signed multiplication, with no saturation/clamp.
    cell.local_volume = 1024;
    assert_eq!(cell.update_scaled_volume(i32::MAX, 2, 1_048_576, 1024), -2);
}

struct MutatingHandler {
    nested: AtomicBool,
    add: HandlerRef,
    remove: HandlerRef,
}
impl EventHandler for MutatingHandler {
    fn do_event(&self, system: &mut SndSys, event: &SndEventStd) -> bool {
        if !self.nested.swap(true, Ordering::SeqCst) {
            // Removal requested before add: drain order still adds then removes.
            system.unregister_handler(self.add.clone());
            system.register_handler(self.add.clone());
            system.unregister_handler(self.remove.clone());
            system.do_event(event);
        }
        false
    }
}
#[test]
fn deferred_handlers_survive_nested_dispatch_and_drain_adds_first() {
    let trace = Arc::new(Mutex::new(Vec::new()));
    let mut system = SndSys::default();
    let old: HandlerRef = Arc::new(Handler {
        name: "old",
        answer: false,
        trace: trace.clone(),
    });
    let new: HandlerRef = Arc::new(Handler {
        name: "new",
        answer: false,
        trace: trace.clone(),
    });
    system.register_handler(Arc::new(MutatingHandler {
        nested: AtomicBool::new(false),
        add: new.clone(),
        remove: old.clone(),
    }));
    system.register_handler(old.clone());
    system.do_event(&SndEventStd::new());
    assert_eq!(*trace.lock().unwrap(), ["old", "old"]);
    assert_eq!(Arc::strong_count(&new), 2); // local + mutator, no active/queued copy
    assert_eq!(Arc::strong_count(&old), 2);
    trace.lock().unwrap().clear();
    system.do_event(&SndEventStd::new());
    assert!(trace.lock().unwrap().is_empty());
}
struct MutatingTarget {
    once: AtomicBool,
    add: TargetRef,
}
impl UpdateTarget for MutatingTarget {
    fn update(&self, system: &mut SndSys) -> bool {
        if !self.once.swap(true, Ordering::SeqCst) {
            system.register_classified_target(self.add.clone(), TargetClass::Ordinary);
        }
        true
    }
}
#[test]
fn target_queues_and_callback_short_circuit() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut system = SndSys::default();
    let added: TargetRef = Arc::new(Target(calls.clone(), true));
    system.register_classified_target(
        Arc::new(MutatingTarget {
            once: AtomicBool::new(false),
            add: added.clone(),
        }),
        TargetClass::Ordinary,
    );
    system.enumerate_targets(TargetClass::Ordinary);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    system.enumerate_targets(TargetClass::Ordinary);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    system.register_classified_target(added.clone(), TargetClass::Slow);
    system.unregister_target(added);
    system.enumerate_targets(TargetClass::Slow);
    system.enumerate_targets(TargetClass::Ordinary);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    system.register_classified_target(Arc::new(Target(calls.clone(), false)), TargetClass::Ordinary);
    system.register_classified_target(Arc::new(Target(calls.clone(), true)), TargetClass::Ordinary);
    system.enumerate_targets(TargetClass::Ordinary);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
struct RenderSpy(Mutex<Vec<i32>>);
impl Renderer for RenderSpy {
    fn master_volume(&self, volume: i32) {
        self.0.lock().unwrap().push(volume);
    }
}
struct Posts(Vec<Arc<SndEventStd>>);
impl MessageService for Posts {
    fn post(&mut self, event: Arc<SndEventStd>) {
        self.0.push(event);
    }
}
#[test]
fn renderer_volume_and_posting_are_foreign_io() {
    let mut system = SndSys::new(Some("Audio".into()));
    assert_eq!(system.master_volume(), 1024);
    let renderer = Arc::new(RenderSpy(Mutex::new(Vec::new())));
    let reference: RendererRef = renderer.clone();
    system.register_renderer(reference.clone()).unwrap();
    system.set_master_volume(-12);
    assert_eq!(*renderer.0.lock().unwrap(), [-12]);
    assert_eq!(system.master_volume(), -12);
    assert!(system.is_registered_renderer(&reference));
    assert_eq!(system.num_renderers(), 1);
    system.unregister_renderer(&reference);
    assert_eq!(system.num_renderers(), 0);
    let event = Arc::new(SndEventStd::with_payload(1, 2, 3, 4, 5));
    let mut posts = Posts(Vec::new());
    assert!(system.post_event(event.clone(), &mut posts));
    assert!(Arc::ptr_eq(&posts.0[0], &event));
}

#[derive(Default)]
struct Factory {
    null: bool,
    classes: Vec<u32>,
    calls: Vec<(u32, u32)>,
    services: usize,
    handlers: usize,
    panics: usize,
    hooks: Vec<u32>,
}
impl DirectorHost for Factory {
    fn register_class(&mut self, class: u32) {
        self.classes.push(class);
    }
    fn register_hook(&mut self, director: u32) {
        self.hooks.push(director);
    }
    fn create(&mut self, class: u32, iid: u32) -> Option<ForeignRef> {
        self.calls.push((class, iid));
        if self.null {
            None
        } else {
            Some(Arc::new(class))
        }
    }
    fn add_service(&mut self, _: ForeignRef) {
        self.services += 1;
    }
    fn register_handler(&mut self, _: &ForeignRef, _: ForeignRef) {
        self.handlers += 1;
    }
    fn disable_updates(&mut self, _: &ForeignRef) {
        self.panics += 1;
    }
    fn sound_system_service(&mut self) -> Option<ForeignRef> {
        if self.null {
            None
        } else {
            Some(Arc::new(0u32))
        }
    }
    fn acquire_property_cache(&mut self, _: &ForeignRef) -> Option<Arc<dyn PropertyCache>> {
        None
    }
}
#[test]
fn director_factories_null_paths_and_disable_updates() {
    let mut host = Factory::default();
    let mut director = SoundDirector::construct(&mut host);
    assert_eq!(host.classes, SOUND_CLASSES);
    assert!(director.on_start(&mut host));
    assert_eq!(
        host.calls,
        [
            (0xE41CE9FD, SERVICE_IID),
            (0x7AF85B98, SERVICE_IID),
            (0x7DC495CC, 0xBB523BB2)
        ]
    );
    assert_eq!((host.services, host.handlers), (2, 1));
    assert!(director.pre_app_shutdown(&mut host));
    assert_eq!(host.panics, 1);
    assert_eq!(
        director.post_app_init(&mut host),
        Err(NativeFault::MissingPropertyCache)
    );
    let mut null = Factory {
        null: true,
        ..Default::default()
    };
    let mut director = SoundDirector::construct(&mut null);
    assert!(director.on_start(&mut null));
    assert_eq!(director.post_app_init(&mut null), Ok(false));
    assert!(director.pre_app_shutdown(&mut null));
    assert_eq!(null.panics, 0);
    let mut mac = Factory::default();
    assert!(MacSoundDirector::construct(&mut mac).on_start(&mut mac));
    assert_eq!(mac.classes, MAC_CLASSES);
    assert_eq!(mac.hooks, [MacSoundDirector::ID]);
    assert_eq!(mac.calls, [(0xE9D28415, SERVICE_IID), (0x89D3D983, SERVICE_IID)]);
    assert_eq!(mac.services, 2);
}
#[test]
fn playlist_absolute_paths_and_incomplete_pls() {
    assert!(PlayList::is_drive_spec("Z:track"));
    assert_eq!(PlayList::make_absolute_path("C:/base", "D:/song"), Ok("D:/song".into()));
    assert_eq!(
        PlayList::make_absolute_path("C:\\base", "song"),
        Ok("C:\\basesong".into())
    );
    assert_eq!(PlayList::make_absolute_path("C:/base", "../song"), Ok("".into()));
    assert_eq!(
        PlayList::default().parse_converted("", &mut Text::from("[playlist]\nFile1=a")),
        Ok(false)
    );
}

struct Acquisitions {
    calls: Vec<&'static str>,
    manager: Arc<SndMgr>,
}
impl TrackServices for Acquisitions {
    fn sound_system(&mut self) -> Option<ForeignRef> {
        self.calls.push("sound");
        Some(Arc::new(1u32))
    }
    fn hit_manager(&mut self) -> Option<ForeignRef> {
        self.calls.push("hit");
        Some(Arc::new(2u32))
    }
    fn sound_manager(&mut self) -> Option<Arc<SndMgr>> {
        self.calls.push("manager");
        Some(self.manager.clone())
    }
}
#[test]
fn track_construction_acquires_services_without_loading() {
    let manager = Arc::new(SndMgr::default());
    let mut host = Acquisitions {
        calls: Vec::new(),
        manager: manager.clone(),
    };
    let track = HitTrackPlayer::construct(&mut host);
    assert_eq!(host.calls, ["sound", "hit", "manager"]);
    assert_eq!(track.services.len(), 2);
    assert!(Arc::ptr_eq(track.embedded_channel.manager.as_ref().unwrap(), &manager));
    assert!(!track.is_loaded());
    assert!(!track.is_playing());
    assert!(!track.paused);
    assert_eq!(track.error, 0);
    assert_eq!(track.embedded_channel.play_value, 0x400);
    assert!(track.embedded_channel.source.is_none());
}
#[derive(Default)]
struct HitHost(Vec<&'static str>);
impl HitServices for HitHost {
    fn install_default_semantics(&mut self) {
        self.0.push("semantics");
    }
    fn acquire_sound_and_persist(&mut self) {
        self.0.push("services");
    }
}
#[test]
fn hit_defaults_and_initialization_services() {
    let mut hit = HitMan::default();
    assert_eq!(
        (hit.service_id, hit.priority, hit.random_seed),
        (0x1B2CE671, 0x7A116, u32::MAX)
    );
    assert!(!hit.initialized);
    assert!(!hit.paused);
    assert_eq!(hit.channel_count(), 0);
    let mut host = HitHost::default();
    hit.init(&mut host);
    assert_eq!(host.0, ["semantics", "services"]);
    assert!(hit.initialized);
}
struct ScoreIo {
    has_record: bool,
    load_ok: bool,
    loads: usize,
}
impl ScoreInput for ScoreIo {
    type Record = u32;
    fn persist_record(&mut self, resource: u32) -> Option<u32> {
        self.has_record.then_some(resource)
    }
    fn load_score_as_file(&mut self, _: u32) -> Result<ScoreData, Unsupported> {
        self.loads += 1;
        if self.load_ok {
            Ok(score())
        } else {
            Err(Unsupported("SS6"))
        }
    }
}
#[test]
fn score_resource_failure_does_not_initialize() {
    let mut player = FreshPlayer::default();
    let mut io = ScoreIo {
        has_record: false,
        load_ok: false,
        loads: 0,
    };
    assert_eq!(player.init_resource(7, &mut io), Ok(false));
    assert_eq!(io.loads, 0);
    io.has_record = true;
    assert_eq!(player.init_resource(7, &mut io), Err(Unsupported("SS6")));
    assert!(!player.initialized);
    io.load_ok = true;
    assert_eq!(player.init_resource(7, &mut io), Ok(true));
    assert!(player.initialized);
    assert_eq!(io.loads, 2);
}
#[test]
fn manager_successful_init_registers_once() {
    let mut manager = SndMgr::default();
    let mut host = ManagerHost {
        available: true,
        ..Default::default()
    };
    assert!(manager.init(&mut host));
    assert!(manager.init(&mut host));
    assert_eq!(host.registrations, [(0x20, 250)]);
    assert_eq!((host.acquisitions, host.retains), (1, 1));
    assert!(manager.sound_service_acquired);
}
struct RemovingTarget {
    once: AtomicBool,
    remove: TargetRef,
}
impl UpdateTarget for RemovingTarget {
    fn update(&self, system: &mut SndSys) -> bool {
        if !self.once.swap(true, Ordering::SeqCst) {
            system.unregister_target(self.remove.clone());
            system.enumerate_targets(TargetClass::Slow);
        }
        true
    }
}
#[test]
fn target_removal_is_deferred_through_nested_enumeration() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut system = SndSys::default();
    let target: TargetRef = Arc::new(Target(calls.clone(), true));
    system.register_classified_target(
        Arc::new(RemovingTarget {
            once: AtomicBool::new(false),
            remove: target.clone(),
        }),
        TargetClass::Ordinary,
    );
    system.register_classified_target(target.clone(), TargetClass::Ordinary);
    system.register_classified_target(target.clone(), TargetClass::Slow);
    system.enumerate_targets(TargetClass::Ordinary);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // Retained only by this test and the removing target, not by either list/queue.
    assert_eq!(Arc::strong_count(&target), 2);
    system.enumerate_targets(TargetClass::Slow);
    system.enumerate_targets(TargetClass::Ordinary);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
struct PayloadHandler;
impl EventHandler for PayloadHandler {
    fn do_event(&self, _: &mut SndSys, event: &SndEventStd) -> bool {
        assert_eq!(event.payload, 99);
        assert_eq!(event.flag, 1);
        assert_eq!(
            [event.base.data1, event.base.data2, event.base.data3, event.base.data4],
            [1, 2, 3, 4]
        );
        true
    }
}
#[test]
fn payload_overload_reuses_existing_event_fields() {
    let mut system = SndSys::default();
    system.register_handler(Arc::new(PayloadHandler));
    assert!(system.do_payload(99, [1, 2, 3, 4], || Some(SndEventStd::new())));
    let mut posts = Posts(Vec::new());
    assert!(system.post_payload(99, [1, 2, 3, 4], || Some(SndEventStd::new()), &mut posts));
    assert_eq!(posts.0[0].payload, 99);
    assert!(!system.post_payload(99, [1, 2, 3, 4], || None, &mut posts));
    assert_eq!(posts.0.len(), 1);
}
