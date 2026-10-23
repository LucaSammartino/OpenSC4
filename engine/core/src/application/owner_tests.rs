// SPDX-License-Identifier: GPL-3.0-or-later
use super::city::{self, CityHost, Effect as E, Object as O, Query as Q, RegionalCity, Reply, Scope};
use super::compatibility::*;
use super::*;
use std::collections::VecDeque;

#[derive(Default)]
struct Foreign {
    app_effects: Vec<super::Effect>,
    resource_log: Vec<String>,
    reject_driver: bool,
    effects: Vec<E>,
    queries: Vec<Q>,
    replies: VecDeque<(Q, Reply)>,
    next: u64,
    geometry: Vec<RegionalCity>,
    record: Option<(u32, Vec<u8>)>,
    written: Vec<u8>,
    files: Vec<Vec<u8>>,
    enabled: Option<Vec<u8>>,
    loaded: Vec<Vec<u8>>,
    remaining: u32,
    ticks: u64,
    step: u64,
    deliveries: u32,
    available: bool,
    missing_region: bool,
    naming_occupied: Vec<(Vec<u8>, Option<u32>)>,
    missing_library: bool,
    missing_gzcom: bool,
    resolved_library: Option<Vec<u8>>,
    library_events: Vec<&'static str>,
    library_ini_paths: Vec<Vec<u8>>,
}
impl Foreign {
    fn reply(&mut self, query: Q, reply: Reply) {
        self.replies.push_back((query, reply));
    }
    fn flag(&mut self, query: Q, flag: bool) {
        self.reply(
            query,
            Reply {
                flag,
                ..Reply::default()
            },
        );
    }
    fn number(&mut self, query: Q, number: u32) {
        self.reply(
            query,
            Reply {
                number: number as i32,
                ..Reply::default()
            },
        );
    }
    fn text(&mut self, query: Q, text: &[u8]) {
        self.reply(
            query,
            Reply {
                text: text.to_vec(),
                ..Reply::default()
            },
        );
    }
    fn token(&mut self, query: Q, token: u64) {
        self.reply(
            query,
            Reply {
                token: Some(token),
                ..Reply::default()
            },
        );
    }
}
impl CityHost for Foreign {
    fn query(&mut self, query: Q) -> Reply {
        self.queries.push(query.clone());
        if self.replies.front().is_some_and(|(q, _)| *q == query) {
            self.replies.pop_front().unwrap().1
        } else {
            Reply {
                flag: matches!(query, Q::HasSourceView | Q::HasDestinationView),
                ..Reply::default()
            }
        }
    }
    fn effect(&mut self, effect: E) {
        self.effects.push(effect);
    }
    fn create(&mut self, _: O) -> u64 {
        self.next += 1;
        self.next
    }
    fn regional_geometry(&mut self, token: u64) -> RegionalCity {
        *self.geometry.iter().find(|c| c.token == token).unwrap()
    }
    fn all_cities(&mut self, _: u64) -> Vec<RegionalCity> {
        self.geometry.clone()
    }
    fn city_at(&mut self, _: u64, x: i32, y: i32) -> u64 {
        self.geometry.iter().find(|c| c.x == x && c.y == y).unwrap().token
    }
    fn adjacent_cities(&mut self, _: u64) -> Vec<u64> {
        self.geometry.iter().map(|c| c.token).collect()
    }
}
impl super::save_path::SavePathHost for Foreign {
    fn save_directory(&mut self, _: u64) -> Option<Vec<u8>> {
        (!self.missing_region).then(Vec::new)
    }
    fn city_name(&mut self, _: u64) -> Vec<u8> {
        b"New".to_vec()
    }
    fn city_id(&mut self, _: u64) -> u32 {
        0
    }
    fn filename_system_roundtrip(&mut self, bytes: &[u8]) -> Vec<u8> {
        bytes.to_vec()
    }
    fn filename_encoding_valid(&mut self, bytes: &[u8]) -> bool {
        std::str::from_utf8(bytes).is_ok()
    }
    fn file_exists(&mut self, path: &[u8]) -> bool {
        self.naming_occupied.iter().any(|(p, _)| p == path)
    }
    fn existing_city_id(&mut self, path: &[u8]) -> Option<u32> {
        self.naming_occupied
            .iter()
            .find(|(p, _)| p == path)
            .and_then(|(_, id)| *id)
    }
}
impl CompatibilityHost for Foreign {
    fn library_path(&mut self, library: &[u8]) -> Vec<u8> {
        self.library_events.push("resolve");
        self.resolved_library.clone().unwrap_or_else(|| library.to_vec())
    }
    fn library_exists(&mut self, _: &[u8]) -> bool {
        self.library_events.push("exists");
        !self.missing_library
    }
    fn gzcom_available(&mut self) -> bool {
        self.library_events.push("gzcom");
        !self.missing_gzcom
    }
    fn read_preferences_record(&mut self, _: &[u8], key: [u32; 3]) -> Option<(u32, Vec<u8>)> {
        assert_eq!(key, PREFERENCES_KEY);
        self.record.take()
    }
    fn delete_preferences_file(&mut self, _: &[u8]) {
        self.written.clear();
    }
    fn write_preferences_record(&mut self, _: &[u8], key: [u32; 3], version: u32, bytes: &[u8]) -> bool {
        assert_eq!(key, PREFERENCES_KEY);
        assert_eq!(version, 0x15);
        self.written = bytes.to_vec();
        true
    }
    fn dll_files(&mut self, _: &[u8]) -> Vec<Vec<u8>> {
        self.files.clone()
    }
    fn library_admin_enabled(&mut self, path: &[u8], _: bool) -> Option<Vec<u8>> {
        self.library_events.push("ini");
        self.library_ini_paths.push(path.to_vec());
        self.enabled.clone()
    }
    fn add_library(&mut self, path: &[u8], _: bool) -> bool {
        self.library_events.push("add");
        self.loaded.push(path.to_vec());
        true
    }
}
impl PumpHost for Foreign {
    fn server_available(&mut self, _: u8) -> bool {
        self.available
    }
    fn now_ms(&mut self) -> u64 {
        let t = self.ticks;
        self.ticks += self.step;
        t
    }
    fn message_waiting(&mut self, _: u8) -> bool {
        self.remaining > 0
    }
    fn deliver_one(&mut self, _: u8) {
        self.remaining -= 1;
        self.deliveries += 1;
    }
}
impl StartupHost for Foreign {
    fn other_instance_running(&mut self) -> bool {
        self.remaining > 0
    }
    fn restart_time(&mut self) -> u64 {
        self.ticks
    }
    fn restart_sleep(&mut self, units: u32) {
        assert_eq!(units, 200_000);
        self.ticks += 1;
        self.remaining -= 1;
    }
    fn driver_description_hardware(&mut self) -> bool {
        false
    }
}
impl super::preference_application::PreferenceHost for Foreign {
    fn language_manager_present(&mut self) -> bool {
        false
    }
    fn language_by_name(&mut self, _: &[u8], _: bool) -> Option<u32> {
        None
    }
    fn installed_language(&mut self, _: bool) -> Option<u32> {
        None
    }
    fn system_language(&mut self) -> u32 {
        0
    }
    fn can_switch_language(&mut self, _: u32) -> bool {
        false
    }
    fn select_language(&mut self, _: u32) {
        unreachable!()
    }
    fn graphics_mode(&mut self) -> Option<[u32; 3]> {
        None
    }
    fn request_mode(&mut self, _: [u32; 3]) {
        unreachable!()
    }
    fn request_fullscreen(&mut self, _: bool) {
        unreachable!()
    }
    fn query_graphics_interface(&mut self) -> Option<u64> {
        None
    }
    fn select_driver(&mut self, _: u64, _: u32) {
        unreachable!()
    }
    fn release_graphics_interface(&mut self, _: u64) {
        unreachable!()
    }
    fn simulator(&mut self, _: u64) -> Option<u64> {
        None
    }
    fn set_simulator_speed(&mut self, _: u64, _: u32) {
        unreachable!()
    }
    fn framework_command_line(&mut self) -> crate::cmdline::CmdLine {
        crate::cmdline::CmdLine::new(b"")
    }
    fn default_cheat(&mut self, _: u64, _: &[u8]) {
        unreachable!()
    }
}
impl super::resources::ResourceOwnerHost for Foreign {
    fn language_name(&mut self, _: i32) -> Option<Vec<u8>> {
        None
    }
    fn registry_entries(&mut self, _: &[u8], _: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        Vec::new()
    }
    fn directory_exists(&mut self, _: &[u8]) -> bool {
        false
    }
    fn create_directory(&mut self, _: &[u8]) {}
    fn enumerate(&mut self, _: &[u8], _: &[u8]) -> Vec<super::resources::DirectoryEntry> {
        Vec::new()
    }
    fn is_database_file(&mut self, _: &[u8], _: u32) -> i32 {
        -1
    }
    fn create_segment(&mut self, _: u32, _: u32) -> Option<u64> {
        None
    }
    fn initialize_segment(&mut self, _: u64) -> bool {
        unreachable!()
    }
    fn set_segment_path(&mut self, _: u64, _: &[u8]) -> bool {
        unreachable!()
    }
    fn open_segment(&mut self, _: u64, _: u32, _: u32) -> bool {
        unreachable!()
    }
    fn register_segment(&mut self, _: u64, _: Option<u32>) -> bool {
        unreachable!()
    }
    fn shutdown_segment(&mut self, _: u64) {
        unreachable!()
    }
    fn release_segment(&mut self, _: u64) {
        unreachable!()
    }
    fn register_factory(&mut self, _: u32, _: u32, _: u32) {}
    fn resource_count(&mut self) -> u32 {
        3
    }
}
impl super::Host for Foreign {
    fn effect(&mut self, effect: super::Effect) {
        self.app_effects.push(effect);
    }
    fn create(&mut self, _: super::Object) -> Result<u64, HostFailure> {
        self.next += 1;
        Ok(self.next)
    }
    fn register_service(&mut self, _: Service, _: u64) -> Result<(), HostFailure> {
        Ok(())
    }
    fn release_created(&mut self, token: u64) {
        self.effects.push(E::Release(token));
    }
    fn install_directory(&mut self) -> Option<Vec<u8>> {
        Some(b"install".to_vec())
    }
    fn graphics_driver(&mut self) -> Option<u64> {
        Some(9)
    }
    fn check_graphics_driver(&mut self, _: u64) -> bool {
        !self.reject_driver
    }
    fn directory_exists(&mut self, _: &[u8]) -> bool {
        true
    }
    fn watcher_running(&mut self, _: u64) -> bool {
        false
    }
    fn watched_directory(&mut self, _: u64) -> Vec<u8> {
        b"plugins".to_vec()
    }
    fn install_exception_filter(&mut self) -> u64 {
        0
    }
}
impl ResourceHost for Foreign {
    fn ini_value(&mut self, _: &[u8], section: &[u8], key: &[u8]) -> Option<Vec<u8>> {
        if section == b"Directories" && key == b"Data" {
            Some(b"override".to_vec())
        } else if key == b"packedtextures" {
            Some(b"on".to_vec())
        } else {
            None
        }
    }
    fn ini_entries(&mut self, _: &[u8], section: &[u8]) -> Vec<Vec<u8>> {
        if section == b"DBSegments" {
            vec![b"id;%DataDirectory%/%Language%".to_vec()]
        } else {
            Vec::new()
        }
    }
    fn directory_exists(&mut self, _: &[u8]) -> bool {
        false
    }
    fn create_directory(&mut self, path: &[u8]) {
        self.resource_log
            .push(format!("mkdir:{}", String::from_utf8_lossy(path)));
    }
    fn database_files(&mut self, path: &[u8], recursive: bool) -> Vec<Vec<u8>> {
        self.resource_log
            .push(format!("list:{}:{recursive}", String::from_utf8_lossy(path)));
        vec![b"a.dat".to_vec(), b"b.sc4".to_vec()]
    }
    fn open_resource_segment(&mut self, _: &[u8], path: &[u8]) {
        self.resource_log
            .push(format!("open:{}", String::from_utf8_lossy(path)));
    }
    fn register_resource_factory(&mut self, kind: u32) {
        self.resource_log.push(format!("factory:{kind}"));
    }
    fn packed_setting(&mut self, textures: bool, enabled: bool) {
        self.resource_log.push(format!("packed:{textures}:{enabled}"));
    }
}
impl EntryHost for Foreign {
    fn create_framework(&mut self, sc4: bool) -> u64 {
        self.resource_log.push(format!("framework:{sc4}"));
        1
    }
    fn create_application(&mut self, name: &[u8]) -> u64 {
        assert_eq!(name, b"SimCity 4");
        self.resource_log.push("app".into());
        2
    }
    fn set_framework(&mut self, _: u64) {
        self.resource_log.push("set framework".into());
    }
    fn set_application_framework_face(&mut self, app: u64) {
        assert_eq!(app, 2);
        self.resource_log.push("set app face".into());
    }
    fn framework_main(&mut self, _: u64, _: &[Vec<u8>]) {
        self.resource_log.push("main".into());
    }
    fn release_application(&mut self, _: u64) {
        self.resource_log.push("release app".into());
    }
    fn framework_slot40(&mut self, _: u64, value: u32) {
        assert_eq!(value, 1);
        self.resource_log.push("slot40".into());
    }
    fn resource_error(&mut self) -> Option<Vec<u8>> {
        Some(b"error".to_vec())
    }
    fn display_resource_error(&mut self, _: &[u8]) {
        self.resource_log.push("error".into());
    }
}
fn app() -> Application {
    super::tests::app()
}
fn pos(token: u64, x: i32, y: i32) -> RegionalCity {
    RegionalCity {
        token,
        x,
        y,
        width: 4,
        height: 4,
    }
}

#[test]
fn new_and_close_city_order_and_inclusive_region_refresh() {
    let mut app = app();
    let mut h = Foreign::default();
    app.message_server2_available = true;
    app.city_state.regional_city = Some(20);
    assert!(app.new_city(&mut h));
    assert!(!app.new_city(&mut h));
    let messages: Vec<_> = h
        .effects
        .iter()
        .filter_map(|e| {
            if let E::CityMessage { kind, d1, post, .. } = e {
                Some((*kind, *d1, *post))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        messages,
        [
            (0x26D3_1EC0, 1, false),
            (0x2B96_B3EA, 0, true),
            (0x26D3_1EC1, 1, true),
            (0xAB96_B05F, 0, true),
            (0xEA8A_E29A, 0, true)
        ]
    );
    assert_eq!(app.mode, 0); // queued PostCityInitComplete has not been delivered
    h.effects.clear();
    app.city_state.region_owner = Some(29);
    h.token(Q::Region(29), 30);
    h.text(Q::RegionalPath(20), b"city.sc4");
    h.geometry = vec![pos(20, 4, 4), pos(21, 8, 4), pos(22, 8, 8), pos(23, 9, 4)];
    assert!(app.close_city(&mut h));
    assert!(h.effects.contains(&E::ReloadRegional(30, 20)));
    assert!(h.effects.contains(&E::ReloadRegional(30, 21)));
    assert!(h.effects.contains(&E::ReloadRegional(30, 22)));
    assert!(!h.effects.contains(&E::ReloadRegional(30, 23)));
    assert_eq!(h.effects.iter().filter(|e| **e == E::ResourceIdle).count(), 2);
    assert_eq!(app.city_state.city, None);
    assert!(!app.close_city(&mut h));
}

#[test]
fn quit_requests_queue_directed_delivery_and_balance_cancelled_pause() {
    let mut app = app();
    let mut h = Foreign::default();
    app.message_server2_available = true;
    h.number(Q::Dialog(0x0A55_161D, 0xAA92_1F4F), 0);
    assert_eq!(app.request_quit(true, false, &mut h), Ok(false));
    assert_eq!(h.effects, [E::Scope(Scope::Pause, true), E::Scope(Scope::Pause, false)]);
    h.effects.clear();
    assert_eq!(app.request_quit(false, true, &mut h), Ok(true));
    assert!(h.effects.contains(&E::DirectedQuit));
    assert!(!app.quitting());
    assert_eq!(h.effects.iter().filter(|e| matches!(e, E::Pump(..))).count(), 2);
    h.effects.clear();
    h.number(Q::Dialog(0x4A55_1B4C, 0xAA92_1F4F), 0x8A92_1F70);
    assert!(!app.request_quit_from_region(true, &mut h));
    assert!(!h.effects.contains(&E::DirectedQuit));
    assert!(app.request_quit_from_region(false, &mut h));
}

#[test]
fn request_close_no_and_blocked_save_do_not_attempt_file_io() {
    let mut app = app();
    let mut h = Foreign::default();
    app.city_state.city = Some(4);
    h.flag(Q::Confirm(0x2A56_CDA8, 0x2A56_CD9B), true);
    h.flag(Q::AltDown, true);
    h.flag(Q::BlocksSaving(4), true);
    assert_eq!(app.request_close_city(true, &mut h), Ok(false));
    assert_eq!(app.city_state.city, Some(4));
    assert!(h.effects.contains(&E::Notify(0xAA66_6AFD, 0xCA66_6B15, Vec::new())));
    assert!(!h.queries.contains(&Q::TempPath));
    assert_eq!(app.request_close_city(true, &mut h), Ok(true));
}

#[test]
fn read_load_reject_versions_and_release_half_read_objects() {
    for version in [61, 66] {
        let mut app = app();
        let mut h = Foreign::default();
        h.flag(Q::SegmentInit(1), true);
        h.flag(Q::SegmentOpen(1, false), true);
        h.number(Q::AppVersion(1), version);
        assert!(!app.load_city(b"x", None, &mut h));
        assert_eq!(app.city_state.segment, None);
        assert!(h.effects.contains(&E::Release(1)));
        assert!(h.replies.is_empty());
    }
    let mut app = app();
    let mut h = Foreign::default();
    h.flag(Q::SegmentInit(1), true);
    h.flag(Q::SegmentOpen(1, false), true);
    h.number(Q::AppVersion(1), 65);
    h.flag(Q::SegmentLoadStore(1), true);
    h.number(Q::RootVersion(1), 1);
    h.number(Q::AppRootVersion(1), 2);
    h.token(Q::ReadCity(1), 10);
    assert!(!app.load_city(b"x", None, &mut h));
    assert_eq!(app.city_state.city, None);
    assert_eq!(app.city_state.segment, None);
    assert_eq!(h.effects.iter().filter(|e| **e == E::Release(1)).count(), 2); // retained + local
    assert!(h.effects.contains(&E::Release(10)));
}

#[test]
fn successful_load_orders_store_root_messages_init_and_pumps() {
    let mut app = app();
    let mut h = Foreign::default();
    app.message_server2_available = true;
    h.flag(Q::SegmentInit(1), true);
    h.flag(Q::SegmentOpen(1, false), true);
    h.number(Q::AppVersion(1), 62);
    h.flag(Q::SegmentLoadStore(1), true);
    h.number(Q::RootVersion(1), 1);
    h.number(Q::AppRootVersion(1), 2);
    h.token(Q::ReadCity(1), 10);
    h.token(Q::ReadRegionalCity(1), 11);
    assert!(app.load_city(b"x", None, &mut h));
    assert_eq!(app.city_state.city, Some(10));
    assert_eq!(app.city_state.segment, Some(1));
    assert_eq!(app.city_state.regional_city, Some(11));
    let kinds: Vec<_> = h
        .effects
        .iter()
        .filter_map(|e| match e {
            E::PackageMessage { kind, .. } | E::CityMessage { kind, .. } => Some(*kind),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        [
            0x26C6_3340,
            0x26C6_3341,
            0x26C6_3342,
            0x26D3_1EC0,
            0x2B96_B3EA,
            0x26D3_1EC1,
            0xAB96_B05F,
            0xEA8A_E29A
        ]
    );
    assert_eq!(h.effects.iter().filter(|e| matches!(e, E::Pump(..))).count(), 4);
}

fn save_fixture() -> (Application, Foreign) {
    let mut app = app();
    app.message_server2_available = true;
    app.city_state.city = Some(10);
    let mut h = Foreign::default();
    h.text(Q::CityPath(10), b"old");
    h.text(Q::TempPath, b"temp");
    h.flag(Q::SegmentInit(1), true);
    h.flag(Q::SegmentOpen(1, true), true);
    h.flag(Q::SegmentClose(1), true);
    (app, h)
}

#[test]
fn save_replacement_failure_rolls_back_backup_and_live_path() {
    let (mut app, mut h) = save_fixture();
    h.flag(Q::Exists(b"City - New.sc4".to_vec()), true);
    h.flag(Q::Exists(b"City - New.sc4_".to_vec()), false);
    h.flag(Q::Rename(b"City - New.sc4".to_vec(), b"City - New.sc4_".to_vec()), true);
    h.flag(Q::Rename(b"temp".to_vec(), b"City - New.sc4".to_vec()), false);
    h.flag(Q::Remove(b"temp".to_vec()), true);
    h.flag(Q::Rename(b"City - New.sc4_".to_vec(), b"City - New.sc4".to_vec()), true);
    assert_eq!(app.save_city(b"old", false, &mut h), Ok(false));
    assert!(h.effects.contains(&E::SetCityPath(10, b"old".to_vec())));
    assert!(!h.queries.contains(&Q::Remove(b"old".to_vec())));
    assert!(h.replies.is_empty());
    assert_eq!(h.effects.last(), Some(&E::Scope(Scope::BusyCursor, false)));
}

#[test]
fn save_success_removes_stale_file_and_orders_messages_before_object_store() {
    let (mut app, mut h) = save_fixture();
    h.flag(Q::Exists(b"City - New.sc4".to_vec()), false);
    h.flag(Q::Rename(b"temp".to_vec(), b"City - New.sc4".to_vec()), true);
    h.flag(Q::Exists(b"old".to_vec()), true);
    assert_eq!(app.save_city(b"old", true, &mut h), Ok(true));
    assert!(h.queries.contains(&Q::Remove(b"old".to_vec())));
    let kinds: Vec<_> = h
        .effects
        .iter()
        .filter_map(|e| {
            if let E::PackageMessage { kind, .. } = e {
                Some(*kind)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(kinds, [0x26C6_3343, 0x26C6_3346, 0x26C6_3344, 0x26C6_3345]);
    let post = h
        .effects
        .iter()
        .position(|e| matches!(e, E::PackageMessage { kind: 0x26C6_3345, .. }))
        .unwrap();
    let store = h.effects.iter().position(|e| *e == E::SaveObjectStore(1)).unwrap();
    assert!(post < store);
}

#[test]
fn save_invalid_region_balances_scopes_without_changing_city_or_files() {
    let mut app = app();
    let mut h = Foreign::default();
    app.city_state.city = Some(10);
    h.missing_region = true;
    assert_eq!(
        app.save_city(b"old", false, &mut h),
        Err(Boundary::InvalidNativeInput("save requires a selected region"))
    );
    assert!(h.queries.is_empty());
    assert_eq!(h.effects.len(), 8);
}

#[test]
fn height_field_picker_centres_truncates_and_preserves_cancelled_output() {
    let mut app = app();
    let mut h = Foreign::default();
    h.text(Q::TerrainDirectory, b"start");
    h.flag(Q::HasRootWindow, true);
    h.number(Q::RootWidth, 101);
    h.number(Q::DialogWidth, 104);
    h.number(Q::RootHeight, 100);
    h.number(Q::DialogHeight, 80);
    h.number(Q::TerrainModal, 1);
    h.text(Q::TerrainCurrentDirectory, b"chosen");
    h.reply(
        Q::TerrainSelection,
        Reply {
            flag: true,
            text: vec![b'x'; 600],
            ..Reply::default()
        },
    );
    assert!(app.terrain_message(8, 0x27DF_05BE, [3, 0x2872_59F6, 0x30], &mut h));
    assert_eq!(app.city_state.new_city_specification.len(), 511);
    assert!(h.effects.contains(&E::TerrainCenter(-1, 10)));
    assert_eq!(h.effects.last(), Some(&E::TerrainIniRelease));
    let mut out = b"untouched".to_vec();
    assert!(!app.load_height_field(&mut out, &mut h));
    assert_eq!(out, b"untouched");
}

#[test]
fn preferences_preserve_unmodeled_bytes_and_clear_live_language_and_ime() {
    let mut app = app();
    let mut h = Foreign {
        record: Some((0x14, vec![3; PREFERENCES_SIZE])),
        ..Foreign::default()
    };
    assert!(!app.read_preferences(&mut h));
    let mut image = vec![0; PREFERENCES_SIZE];
    image[4] = 0xA5;
    image[0x1000] = 0xAB;
    image[0x4AC..0x4B2].copy_from_slice(b"French");
    image[0x4EF] = 0;
    h.record = Some((0x15, image));
    assert!(app.read_preferences(&mut h));
    assert_eq!(app.preferences.language, b"French");
    assert!(!app.preferences.ime);
    assert!(app.save_preferences(&mut h));
    assert!(app.preferences.language.is_empty());
    assert!(app.preferences.ime);
    assert_eq!(h.written[0x1000], 0xAB);
    assert_eq!(h.written[4], 0xA5);
    assert_eq!(h.written[0x4A8], 1);
    assert_eq!(h.written[0x4EF], 1);
    assert!(h.written[0x4AC..0x4EC].iter().all(|b| *b == 0));
}

#[test]
fn plugins_filter_disabled_libraries_and_registry_substitutes_each_occurrence() {
    let app = app();
    let mut h = Foreign {
        files: vec![b"a.dll".to_vec(), b"b.dll".to_vec()],
        ..Foreign::default()
    };
    for disabled in [b"false".as_slice(), b"0"] {
        h.enabled = Some(disabled.to_vec());
        app.add_code_plugins(b"P", &mut h);
    }
    assert!(h.loaded.is_empty());
    h.enabled = None;
    app.add_code_plugins(b"P", &mut h);
    assert_eq!(h.loaded.len(), 2);
    let entry = registry_entry(b"id;%DataDirectory%/%Language%/%Language%", b"D", None).unwrap();
    assert_eq!(entry.class, b"id");
    assert_eq!(entry.path, b"D/English/English");
}

#[test]
fn pumps_obey_absence_max_and_strict_minimum_time_bounds() {
    let mut h = Foreign {
        remaining: 1000,
        available: true,
        step: 1,
        ..Foreign::default()
    };
    assert_eq!(pump_messages(1, [100, 500, 0], &mut h), 101);
    assert_eq!(pump_messages(2, [100, 2, 5000], &mut h), 2);
    assert_eq!(pump_messages(2, [0, 0, 0], &mut h), 0);
    h.available = false;
    assert_eq!(pump_messages(1, [0, 5, 0], &mut h), 0);
    h.available = true;
    h.remaining = 2;
    assert_eq!(pump_messages(1, [100, 500, 5000], &mut h), 2);
}

#[test]
fn restart_waits_only_when_needed_and_times_out_at_thirty_units() {
    let mut h = Foreign {
        remaining: 5,
        ..Foreign::default()
    };
    assert!(!wait_for_instance(false, false, &mut h));
    assert_eq!(h.ticks, 0);
    assert!(wait_for_instance(true, false, &mut h));
    assert!(wait_for_instance(false, true, &mut h));
    assert_eq!(h.ticks, 5);
    h.remaining = 40;
    assert!(!wait_for_instance(false, true, &mut h));
    assert_eq!(h.ticks, 35);
    assert_eq!(
        FACES
            .iter()
            .find(|(face, _, _)| *face == Face::App)
            .map(|(_, offset, slots)| (*offset, *slots)),
        Some((12, 71))
    );
    assert!(city::touches(pos(1, 4, 4), pos(2, 8, 8)));
}

#[test]
fn owned_runner_executes_preference_failure_autosave_plugins_and_close() {
    let mut app = app();
    let mut h = Foreign {
        reject_driver: true,
        ..Foreign::default()
    };
    app.begin_init().unwrap();
    assert_eq!(app.run_owned(&mut h), Flow::Complete(false));
    assert_eq!(h.written.len(), PREFERENCES_SIZE);
    assert!(matches!(h.app_effects.last(), Some(super::Effect::Warning { .. })));
    app.preferences.language = b"French".to_vec();
    app.timers.gate = 1001;
    app.timers.autosave = 1;
    app.begin_idle().unwrap();
    assert_eq!(app.run_owned(&mut h), Flow::Complete(false));
    assert!(app.preferences.language.is_empty());
    app.watchers = [Some(11), None];
    h.files = vec![b"new.dll".to_vec()];
    app.message_v1(0x8716_3754, 11, b"", &mut h).unwrap();
    assert_eq!(app.run_owned(&mut h), Flow::Complete(true));
    assert_eq!(h.loaded, [b"new.dll".to_vec()]);
    app.city_state.city = Some(5);
    assert_eq!(app.begin_quit(), Ok(true));
    assert_eq!(app.run_owned(&mut h), Flow::Complete(true));
    assert_eq!(app.city_state.city, None);
    assert_eq!(h.app_effects.last(), Some(&super::Effect::FrameworkQuit(0)));
}

#[test]
fn entry_keeps_order_and_rejects_missing_ini_directory() {
    let mut app = app();
    let mut h = Foreign::default();
    app_start(true, &[], &mut h);
    assert_eq!(
        h.resource_log,
        [
            "framework:true",
            "app",
            "set framework",
            "set app face",
            "main",
            "release app",
            "slot40",
            "error"
        ]
    );
    app.directories.data_7c = b"data".to_vec();
    app.find_and_possibly_fix_ini_files(b"app.ini", &mut h);
    assert_eq!(app.directories.data_7c, b"data");
}

#[test]
fn obliteration_keeps_template_only_during_new_city_and_cleans_temp_on_read_failure() {
    let mut app = app();
    app.city_state.city = Some(10);
    let mut h = Foreign::default();
    h.text(Q::TempPath, b"temp");
    h.flag(Q::SegmentInit(1), true);
    h.flag(Q::SegmentOpen(1, true), true);
    h.flag(Q::ObliterationWriteResult(10), true);
    h.number(Q::Zoom, 4);
    h.number(Q::Rotation, 2);
    h.text(Q::CityPath(10), b"old");
    assert_eq!(app.do_city_obliteration(&mut h), Ok(true));
    assert_eq!(app.city_state.template_segment, None);
    assert_eq!(app.city_state.city, None);
    assert!(h.effects.contains(&E::Zoom(4)));
    assert!(h.effects.contains(&E::Rotation(2)));
    assert!(h.queries.contains(&Q::Remove(b"temp".to_vec())));
    assert_eq!(h.effects.last(), Some(&E::Scope(Scope::BusyCursor, false)));
}

#[test]
fn requests_new_load_and_region_follow_their_distinct_no_city_paths() {
    let mut app = app();
    let mut h = Foreign::default();
    assert_eq!(app.request_new_city(Some(20), &mut h), Ok(true));
    assert_eq!(app.city_state.regional_city, Some(20));
    assert_eq!(app.request_go_to_region_view(false, &mut h), Ok(true));
    assert!(h.effects.contains(&E::RegionScreen));
    h.text(Q::SavedCitySuffix, b"Suffix");
    assert_eq!(app.request_load_city(&mut h), Ok(false));
    assert!(h
        .effects
        .iter()
        .any(|e| matches!(e, E::SegmentPath(_, p) if p == b"SavedCitySuffix")));
    assert!(h.effects.contains(&E::Notify(0xCA56_CE8C, 0x4A56_CEA6, Vec::new())));
    assert_eq!(app.save_city_current(false, &mut h), Ok(false));
}

#[test]
fn published_getters_preserve_identity_and_report_unpublished_slots() {
    let mut app = app();
    let mut h = Foreign::default();
    app.handles.flag_668 = 9;
    app.handles.lua_interpreter = Some(44);
    assert_eq!(app.app_interface_getter(50, None, &mut h), Ok(AppValue::Byte(9)));
    assert_eq!(
        app.app_interface_getter(47, None, &mut h),
        Ok(AppValue::Token(Some(44)))
    );
    assert_eq!(
        app.app_interface_getter(45, None, &mut h),
        Ok(AppValue::Embedded(Embedded::GlyphTextureManager))
    );
    assert_eq!(
        app.app_interface_getter(48, None, &mut h),
        Ok(AppValue::Embedded(Embedded::GlyphTextureManager))
    );
    assert_eq!(
        app.app_interface_getter(56, None, &mut h),
        Ok(AppValue::String(Vec::new()))
    );
    assert_eq!(app.app_interface_getter(19, None, &mut h), Err(19));
    assert_eq!(
        app.app_interface_getter(59, None, &mut h),
        Ok(AppValue::String(b"U:\\Plugins\\".to_vec()))
    );
    app.city_state.region_owner = Some(3);
    h.token(Q::Region(3), 4);
    assert_eq!(app.app_interface_getter(35, None, &mut h), Ok(AppValue::Token(Some(4))));
    assert!(!h.effects.iter().any(|e| matches!(e, E::Release(_))));
}

#[test]
fn settled_directory_and_path_slots_use_distinct_sources_and_literal_concatenation() {
    let mut app = app();
    let mut h = Foreign::default();
    app.app_name = b"SimCity 4".to_vec();
    app.directories.data_7c = b"D".to_vec();
    app.directories.cd_data_90 = b"CD".to_vec();
    app.directories.plugin_a4 = b"P".to_vec();
    app.directories.cd_plugin_b8 = b"CP".to_vec();
    app.directories.sku_cc = b"SKU".to_vec();
    app.directories.user_data_e0 = b"UD".to_vec();
    app.directories.slot_e4_108 = b"108".to_vec();
    app.directories.user_base_11c = b"BASE".to_vec();
    assert_eq!(
        app.app_interface_getter_language(53, -1, None, &mut h),
        Ok(AppValue::String(b"P".to_vec()))
    );
    assert_eq!(
        app.app_interface_getter_language(54, -1, None, &mut h),
        Ok(AppValue::String(b"CP".to_vec()))
    );
    for (slot, value) in [
        (51, b"D".as_slice()),
        (52, b"CD"),
        (53, b"P\\English\\"),
        (54, b"CP\\English\\"),
        (55, b"SKU"),
        (56, b"UD"),
        (57, b"108"),
        (58, b"BASE"),
    ] {
        assert_eq!(
            app.app_interface_getter(slot, None, &mut h),
            Ok(AppValue::String(value.to_vec()))
        );
    }
    h.flag(Q::Exists(b"DSimCity 4.ini".to_vec()), true);
    assert_eq!(
        app.app_interface_getter(29, None, &mut h),
        Ok(AppValue::String(b"DSimCity 4.ini".to_vec()))
    );
    assert_eq!(
        app.app_interface_getter(29, None, &mut h),
        Ok(AppValue::String(b"BASESimCity 4.ini".to_vec()))
    );
    assert_eq!(
        app.app_interface_getter(31, None, &mut h),
        Ok(AppValue::String(b"BASESimCity 4.cfg".to_vec()))
    );
    app.mode = 3;
    app.handles.stored_4c = 7;
    app.field_314 = Some(8);
    app.city_state.region_owner = Some(9);
    app.city_state.city = Some(10);
    for (slot, value) in [
        (25, AppValue::Word(3)),
        (26, AppValue::Word(7)),
        (32, AppValue::Token(Some(8))),
        (33, AppValue::Token(None)),
        (34, AppValue::Token(Some(9))),
        (37, AppValue::Token(Some(10))),
    ] {
        assert_eq!(app.app_interface_getter(slot, None, &mut h), Ok(value));
    }
    assert!(h.effects.is_empty());
}

#[test]
fn admin_enabled_uses_only_exact_zero_or_case_insensitive_false_and_bypass() {
    let app = app();
    let mut h = Foreign::default();
    for value in [b"FALSE".as_slice(), b"False", b"false", b"0"] {
        h.enabled = Some(value.to_vec());
        assert!(!app.add_dynamic_library(b"x.dll", false, &mut h));
        assert!(app.add_dynamic_library_with_bypass(b"x.dll", true, true, &mut h));
    }
    for value in [b"00".as_slice(), b"no", b" false", b"false "] {
        h.enabled = Some(value.to_vec());
        assert!(app.add_dynamic_library(b"x.dll", false, &mut h));
    }
}

#[test]
fn plugin_resolution_precedes_filter_and_absence_or_bypass_skips_ini() {
    let app = app();
    let mut h = Foreign {
        enabled: Some(b"FALSE".to_vec()),
        resolved_library: Some(b"resolved/path.dll".to_vec()),
        ..Foreign::default()
    };
    assert!(!app.add_dynamic_library(b"name", true, &mut h));
    assert_eq!(h.library_events, ["resolve", "gzcom", "exists", "ini"]);
    assert_eq!(h.library_ini_paths, [b"resolved/path.dll".to_vec()]);
    h.library_events.clear();
    h.missing_library = true;
    assert!(!app.add_dynamic_library_with_bypass(b"name", true, true, &mut h));
    assert_eq!(h.library_events, ["resolve", "gzcom", "exists"]);
    h.library_events.clear();
    h.missing_library = false;
    h.missing_gzcom = true;
    assert!(!app.add_dynamic_library(b"path", false, &mut h));
    assert_eq!(h.library_events, ["gzcom"]);
}

#[test]
fn terrain_ini_path_uses_slot29_when_global_app_exists() {
    let mut app = app();
    app.app_name = b"SimCity 4".to_vec();
    app.directories.user_base_11c = b"User/".to_vec();
    let mut h = Foreign::default();
    h.flag(Q::HasGlobalApp, true);
    let mut out = Vec::new();
    assert!(!app.load_height_field(&mut out, &mut h));
    assert!(h.effects.contains(&E::TerrainIniOpen(b"User/SimCity 4.ini".to_vec())));
}

#[test]
fn save_names_filter_spaces_cap_encoding_and_resolve_collisions_without_accumulation() {
    use super::save_path::{create_save_path, sanitize_filename};
    let mut h = Foreign::default();
    assert_eq!(
        create_save_path(b"C:\\Region\\", b" A<>:*?  B ", 0, &mut h),
        b"C:\\Region\\City - A B .sc4"
    );
    let long = create_save_path(b"", &vec![b'a'; 300], 0, &mut h);
    assert_eq!(long.len(), 256);
    assert!(!long.ends_with(b".sc4"));
    let split_multibyte = [vec![b'a'; 255], vec![0xC3, 0xA9]].concat();
    assert_eq!(sanitize_filename(&split_multibyte, true, &mut h), vec![b'a'; 255]);
    assert_eq!(sanitize_filename(b"CON\x01<>:\\/\"|*?", true, &mut h), b"CON\x01");
    h.naming_occupied = vec![
        (b"C:\\Region\\City - Oak.sc4".to_vec(), None),
        (b"C:\\Region\\City - Oak (2).sc4".to_vec(), None),
    ];
    assert_eq!(
        create_save_path(b"C:\\Region\\", b"Oak", 0, &mut h),
        b"C:\\Region\\City - Oak (3).sc4"
    );
    assert!(h.effects.is_empty());
}

#[test]
fn save_names_reuse_only_readable_matching_or_zero_existing_id() {
    use super::save_path::create_save_path;
    for (existing, expected) in [
        (Some(7), b"City - Oak.sc4".as_slice()),
        (Some(0), b"City - Oak.sc4"),
        (Some(8), b"City - Oak (2).sc4"),
        (None, b"City - Oak (2).sc4"),
    ] {
        let mut h = Foreign {
            naming_occupied: vec![(b"City - Oak.sc4".to_vec(), existing)],
            ..Foreign::default()
        };
        assert_eq!(create_save_path(b"", b"Oak", 7, &mut h), expected);
    }
}

#[test]
fn obliteration_failed_writer_is_destructive_and_read_init_is_ignored() {
    let mut app = app();
    app.city_state.city = Some(10);
    let mut h = Foreign::default();
    h.text(Q::TempPath, b"temp");
    h.flag(Q::SegmentInit(1), false);
    h.flag(Q::HasSourceView, false);
    h.text(Q::CityPath(10), b"old");
    h.flag(Q::Remove(b"old".to_vec()), true);
    h.flag(Q::SegmentInit(2), false);
    h.flag(Q::SegmentOpen(2, false), false);
    h.flag(Q::HasDestinationView, true);
    assert_eq!(app.do_city_obliteration(&mut h), Ok(true));
    assert_eq!(app.city_state.city, None);
    assert!(h.replies.is_empty());
    assert!(h.effects.contains(&E::Release(1)));
    assert!(!h.effects.contains(&E::SegmentShutdown(1)));
    assert!(h.effects.contains(&E::SegmentPath(2, b"temp".to_vec())));
    assert!(h.effects.contains(&E::SegmentShutdown(2)));
    assert!(!h.effects.contains(&E::SegmentClose(2)));
    assert!(h.effects.contains(&E::Zoom(0)) && h.effects.contains(&E::Rotation(0)));
    assert!(h.queries.contains(&Q::Remove(b"temp".to_vec())));
}

#[test]
fn obliteration_view_checks_are_independent_and_invalid_inputs_are_explicit() {
    let mut app = app();
    let mut h = Foreign::default();
    assert_eq!(
        app.do_city_obliteration(&mut h),
        Err(Boundary::InvalidNativeInput(
            "obliteration temp-name failure has indeterminate return byte"
        ))
    );
    h.text(Q::TempPath, b"temp");
    assert_eq!(
        app.do_city_obliteration(&mut h),
        Err(Boundary::InvalidNativeInput("obliteration requires an open city"))
    );
    app.city_state.city = Some(10);
    h.text(Q::TempPath, b"temp");
    h.flag(Q::HasSourceView, true);
    h.number(Q::Zoom, 7);
    h.number(Q::Rotation, 2);
    h.flag(Q::HasDestinationView, false);
    assert_eq!(app.do_city_obliteration(&mut h), Ok(true));
    assert!(!h.effects.iter().any(|e| matches!(e, E::Zoom(_) | E::Rotation(_))));
    assert!(h.replies.is_empty());
}

#[test]
fn driver_description_correction_and_subscriptions_are_exact() {
    let mut app = app();
    let mut h = Foreign::default();
    app.apply_driver_description(false, &mut h);
    assert_eq!((app.preferences.driver_f13, app.preferences.driver_f0c), (0, 1));
    let warnings = h.app_effects.len();
    app.apply_driver_description(false, &mut h);
    assert_eq!(h.app_effects.len(), warnings);
    app.apply_driver_description(true, &mut h);
    assert_eq!(app.preferences.driver_f13, 1);
    let mut line = crate::cmdline::CmdLine::new(b"-x -w");
    assert!(erase_one_shot_x(&mut line));
    assert!(!erase_one_shot_x(&mut line));
    assert!(line.is_switch_present(b"w"));
    app.subscribe_app_messages(&mut h);
    app.unsubscribe_app_messages(&mut h);
    assert_eq!(
        &h.app_effects[h.app_effects.len() - 3..],
        [
            super::Effect::RegisterV2 {
                kinds: V2_SUBSCRIPTIONS,
                target: 1
            },
            super::Effect::UnregisterV2 {
                kinds: V2_SUBSCRIPTIONS,
                target: 1
            },
            super::Effect::UnregisterV1 {
                kind: QUIT_REQUEST,
                target: 1
            }
        ]
    );
}

#[test]
fn segment_registration_failure_and_success_cleanup_are_balanced() {
    let mut app = app();
    let mut h = Foreign::default();
    assert!(!app.register_city_segment(b"x", &mut h));
    assert_eq!(h.effects.last(), Some(&E::Release(1)));
    assert_eq!(app.city_state.segment, None);
    h.flag(Q::SegmentInit(2), true);
    h.flag(Q::SegmentOpen(2, false), true);
    assert!(app.register_city_segment(b"x", &mut h));
    app.unregister_city_segment(&mut h);
    assert_eq!(app.city_state.segment, None);
    assert_eq!(
        &h.effects[h.effects.len() - 4..],
        [
            E::UnregisterSegment(2),
            E::SegmentShutdown(2),
            E::SegmentClose(2),
            E::Release(2)
        ]
    );
}

#[test]
fn successful_obliteration_restores_view_and_cleans_template_and_both_segments() {
    let mut app = app();
    let mut h = Foreign::default();
    app.city_state.city = Some(10);
    h.text(Q::TempPath, b"temp");
    h.flag(Q::SegmentInit(1), true);
    h.flag(Q::SegmentOpen(1, true), true);
    h.flag(Q::ObliterationWriteResult(10), true);
    h.number(Q::Zoom, 4);
    h.number(Q::Rotation, 2);
    h.text(Q::CityPath(10), b"old");
    h.flag(Q::SegmentInit(2), true);
    h.flag(Q::SegmentOpen(2, false), true);
    // New city is token 3; a failed save temp lookup still restores its path,
    // registers its segment, and must not change the obliteration result.
    assert_eq!(app.do_city_obliteration(&mut h), Ok(true));
    assert_eq!(app.city_state.city, Some(3));
    assert_eq!(app.city_state.template_segment, None);
    assert!(h.effects.contains(&E::SetCityPath(3, b"old".to_vec())));
    assert!(h.effects.contains(&E::Release(1)));
    assert!(h.effects.contains(&E::Release(2)));
    assert!(h.effects.contains(&E::Zoom(4)));
    assert!(h.replies.is_empty());
}

#[test]
fn blocked_and_cancelled_region_and_quit_dialogs_never_save() {
    let mut app = app();
    let mut h = Foreign::default();
    app.city_state.city = Some(10);
    for quit in [false, true] {
        h.flag(Q::BlocksSaving(10), true);
        h.number(Q::Dialog(0x4A89_B3F2, 0x2A96_ED21), 0);
        let result = if quit {
            app.request_quit(true, true, &mut h)
        } else {
            app.request_go_to_region_view(true, &mut h)
        };
        assert_eq!(result, Ok(false));
        assert_eq!(app.city_state.city, Some(10));
    }
    assert!(!h.queries.contains(&Q::TempPath));
    assert!(!h.effects.contains(&E::DirectedQuit));
}

#[test]
fn request_save_notification_uses_reread_filename_and_alt_flag() {
    let (mut app, mut h) = save_fixture();
    // request_save's initial path read precedes save_city's remembered path.
    h.replies.push_front((
        Q::CityPath(10),
        Reply {
            text: b"old".to_vec(),
            ..Reply::default()
        },
    ));
    h.flag(Q::Exists(b"City - New.sc4".to_vec()), false);
    h.flag(Q::Rename(b"temp".to_vec(), b"City - New.sc4".to_vec()), true);
    h.flag(Q::Exists(b"old".to_vec()), false);
    h.flag(Q::Remove(b"temp".to_vec()), true);
    h.flag(Q::SegmentInit(2), true);
    h.flag(Q::SegmentOpen(2, false), true);
    h.text(Q::CityPath(10), b"C:\\Regions\\new.sc4");
    assert_eq!(app.request_save_city(true, true, &mut h), Ok(true));
    assert!(h
        .effects
        .contains(&E::Notify(0x2A56_CDA8, 0x2A56_CE36, b" (new.sc4)".to_vec())));
    assert!(h
        .effects
        .iter()
        .any(|e| matches!(e, E::PackageMessage { flag: true, .. })));
    assert!(h.replies.is_empty());
}
