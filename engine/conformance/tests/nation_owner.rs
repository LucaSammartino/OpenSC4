// SPDX-License-Identifier: GPL-3.0-or-later
//! nation.md §5.1 owner-contract vectors; synthetic interface provider only.
use fc4_core::nation::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

type Trace = Arc<Mutex<Vec<String>>>;
fn record(trace: &Trace, event: impl Into<String>) {
    trace.lock().unwrap().push(event.into());
}

struct Region {
    directory: String,
    name: Vec<u8>,
    tutorial: bool,
    succeeds: bool,
    trace: Trace,
    failed_init: BTreeSet<String>,
    tutorials: BTreeSet<String>,
}
impl Drop for Region {
    fn drop(&mut self) {
        record(&self.trace, format!("drop:{}", self.directory));
    }
}
impl NationRegion for Region {
    type City = City;
    fn directory_name(&self) -> &str {
        &self.directory
    }
    fn display_name(&self) -> &[u8] {
        &self.name
    }
    fn set_directory_name(&mut self, name: &str) {
        self.directory = name.into();
        self.succeeds = !self.failed_init.contains(name);
        self.tutorial = self.tutorials.contains(name);
        record(&self.trace, format!("directory:{name}"));
    }
    fn set_display_name(&mut self, name: &[u8]) {
        self.name = name.to_vec();
        record(&self.trace, format!("display:{}", String::from_utf8_lossy(name)));
    }
    fn read_region_ini(&mut self) -> bool {
        record(&self.trace, format!("ini:{}", self.directory));
        false
    }
    fn init(&mut self) -> bool {
        record(&self.trace, format!("init:{}", self.directory));
        self.succeeds
    }
    fn shutdown(&mut self) -> bool {
        record(&self.trace, format!("shutdown:{}", self.directory));
        false
    }
    fn has_tutorial_city(&self) -> bool {
        self.tutorial
    }
    fn delete(&mut self) {
        record(&self.trace, format!("delete:{}", self.directory));
    }
    fn move_city(&mut self, _: &mut City, x: u32, z: u32) -> bool {
        record(&self.trace, format!("move:{}:{x}:{z}", self.directory));
        false
    }
}
struct City(Trace);
impl Drop for City {
    fn drop(&mut self) {
        record(&self.0, "city:drop");
    }
}
impl TutorialCity for City {
    fn set_save_path(&mut self, path: &str) {
        record(&self.0, format!("city:path:{path}"));
    }
    fn position(&self) -> (u32, u32) {
        (4, 6)
    }
    fn shutdown(&mut self) -> bool {
        record(&self.0, "city:shutdown");
        false
    }
}

struct Host {
    trace: Trace,
    app: bool,
    city: bool,
    root_exists: bool,
    entries: Vec<(String, u32)>,
    after_copy: Vec<(String, u32)>,
    failed_init: BTreeSet<String>,
    tutorials: BTreeSet<String>,
    preference: [u8; 64],
    default_tutorial: String,
    reset: bool,
    screen: Option<u32>,
    registered: Vec<String>,
    valid: bool,
    factory_fails: bool,
    export_fails: BTreeSet<u32>,
    read_fails: BTreeSet<String>,
    tuning: Option<TuningProperties>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            trace: Default::default(),
            app: true,
            city: false,
            root_exists: true,
            entries: vec![("A".into(), 4), ("B".into(), 4), ("C".into(), 4)],
            after_copy: vec![],
            failed_init: Default::default(),
            tutorials: Default::default(),
            preference: [0; 64],
            default_tutorial: "tutorial".into(),
            reset: false,
            screen: None,
            registered: vec![],
            valid: true,
            factory_fails: false,
            export_fails: Default::default(),
            read_fails: Default::default(),
            tuning: None,
        }
    }
}
impl Host {
    fn region(&self, name: &str) -> Region {
        Region {
            directory: name.into(),
            name: name.as_bytes().to_vec(),
            tutorial: self.tutorials.contains(name),
            succeeds: !self.failed_init.contains(name),
            trace: self.trace.clone(),
            failed_init: self.failed_init.clone(),
            tutorials: self.tutorials.clone(),
        }
    }
    fn clear(&self) {
        self.trace.lock().unwrap().clear();
    }
    fn events(&self) -> Vec<String> {
        self.trace.lock().unwrap().clone()
    }
}
impl NationHost for Host {
    type Region = Region;
    type City = City;
    fn has_app(&self) -> bool {
        self.app
    }
    fn has_live_city(&self) -> bool {
        self.city
    }
    fn user_regions_root(&mut self) -> String {
        record(&self.trace, "root");
        "U\\".into()
    }
    fn data_directory(&self) -> String {
        "D\\".into()
    }
    fn language_directory(&self) -> Option<String> {
        Some("French".into())
    }
    fn directory_exists(&mut self, path: &str) -> bool {
        record(&self.trace, format!("exists:{path}"));
        self.root_exists
    }
    fn create_directory(&mut self, path: &str) {
        record(&self.trace, format!("mkdir:{path}"));
    }
    fn entries(&mut self, root: &str) -> Vec<DirectoryEntry> {
        record(&self.trace, format!("entries:{root}"));
        self.entries
            .iter()
            .map(|(name, kind)| DirectoryEntry {
                name: name.clone(),
                kind: *kind,
            })
            .collect()
    }
    fn recursive_files(&mut self, source: &str) -> Vec<String> {
        record(&self.trace, format!("files:{source}"));
        if !self.after_copy.is_empty() {
            self.entries = self.after_copy.clone();
        }
        vec!["nested\\one.sc4".into(), "region.ini".into()]
    }
    fn copy_overwrite(&mut self, source: &str, destination: &str) {
        record(&self.trace, format!("copy:{source}:{destination}"));
    }
    fn make_writable(&mut self, path: &str) {
        record(&self.trace, format!("writable:{path}"));
    }
    fn remove_directory(&mut self, path: &str, recursive: bool) {
        record(&self.trace, format!("remove:{path}:{recursive}"));
    }
    fn has_pattern(&mut self, _: &str, pattern: &str) -> bool {
        record(&self.trace, format!("pattern:{pattern}"));
        self.valid || pattern == "*.sc4"
    }
    fn new_region(&mut self) -> Option<Region> {
        Some(self.region(""))
    }
    fn create_region(&mut self, name: &str) -> Option<Region> {
        record(&self.trace, format!("factory:{name}"));
        if self.factory_fails {
            None
        } else {
            Some(self.region(name))
        }
    }
    fn read_tuning(&mut self) -> Option<TuningProperties> {
        record(&self.trace, "tuning");
        self.tuning.clone()
    }
    fn preference_name(&self) -> [u8; 64] {
        self.preference
    }
    fn default_tutorial_directory(&self) -> String {
        self.default_tutorial.clone()
    }
    fn should_reset_tutorials(&self) -> bool {
        self.reset
    }
    fn subscription(&mut self, id: u32, subscribe: bool) {
        record(&self.trace, format!("subscription:{id}:{subscribe}"));
    }
    fn tutorial_regions(&self) -> Vec<String> {
        self.registered.clone()
    }
    fn tutorial_entries(&self, _: &str) -> Vec<u32> {
        vec![1, 2, 3]
    }
    fn screen_class_id(&self) -> Option<u32> {
        self.screen
    }
    fn destroy_screen(&mut self) {
        record(&self.trace, "screen:destroy");
    }
    fn recreate_region_screen(&mut self) {
        record(&self.trace, "screen:create");
    }
    fn export_tutorial(&mut self, resource: (u32, u32, u32), path: &str) -> i32 {
        assert_eq!((resource.0, resource.1), TUTORIAL_RESOURCE);
        record(&self.trace, format!("export:{path}"));
        if self.export_fails.contains(&resource.2) {
            -1
        } else {
            0
        }
    }
    fn read_tutorial_city(&mut self, path: &str) -> Option<City> {
        record(&self.trace, format!("read:{path}"));
        if self.read_fails.contains(path) {
            None
        } else {
            Some(City(self.trace.clone()))
        }
    }
    fn set_tutorial_active(&mut self, id: u32, active: bool) {
        record(&self.trace, format!("active:{id}:{active}"));
    }
}

fn fixture(host: &mut Host) -> Nation<Region> {
    let mut nation = Nation::new();
    assert_eq!(nation.init(&ProcessDirectories::default(), host), Ok(true));
    host.clear();
    nation
}
fn date() -> EconomyDate {
    EconomyDate {
        sim_year: 2000,
        sim_month: 1,
        start_year: 2000,
        start_month: 1,
    }
}

#[test]
fn construction_selection_and_failures() {
    let nation = Nation::<Region>::new();
    assert!(!nation.is_initialized());
    assert_eq!(nation.get_active_region_index(), Some(0));
    assert_eq!(nation.get_region_count(), 0);
    assert!(matches!(
        nation.get_active_region(),
        Err(NationError::InvalidRegionIndex(0))
    ));
    let mut host = Host {
        failed_init: BTreeSet::from(["B".into()]),
        ..Default::default()
    };
    let mut nation = fixture(&mut host);
    nation.set_active_region(0).unwrap();
    assert!(host.events().is_empty());
    assert!(matches!(nation.get_region(3), Err(NationError::InvalidRegionIndex(3))));
    assert_eq!(nation.set_active_region(3), Err(NationError::InvalidRegionIndex(3)));
    assert!(host.events().is_empty());
    nation.set_active_region(1).unwrap();
    assert_eq!(host.events(), ["shutdown:A", "init:B"]);
    assert_eq!(nation.get_active_region_index(), Some(1));
}

#[test]
fn discovery_preference_and_probe_selection() {
    let mut host = Host {
        entries: vec![
            ("B".into(), 4),
            ("X".into(), 1),
            ("dOwNlOaDs".into(), 4),
            ("A".into(), 4),
        ],
        ..Default::default()
    };
    let nation = fixture(&mut host);
    assert_eq!(nation.get_region_count(), 2);
    assert_eq!(nation.get_region(0).unwrap().directory_name(), "B");
    assert_eq!(nation.get_region(1).unwrap().directory_name(), "A");
    let mut preference = [0; 64];
    preference[..3].copy_from_slice(b"A\0x");
    let mut index = 99;
    assert!(nation.get_index_of_current_region_setting_in_preferences(&preference, &mut index));
    assert_eq!(index, 1);
    for name in [b"a".as_slice(), b"AX"] {
        preference = [0; 64];
        preference[..name.len()].copy_from_slice(name);
        assert!(!nation.get_index_of_current_region_setting_in_preferences(&preference, &mut index));
        assert_eq!(index, 0);
    }
    let mut host = Host {
        failed_init: BTreeSet::from(["A".into()]),
        screen: Some(REGION_SCREEN),
        ..Default::default()
    };
    let mut nation = Nation::new();
    nation.init(&ProcessDirectories::default(), &mut host).unwrap();
    assert_eq!(nation.get_active_region_index(), Some(0));
    assert!(host.events().contains(&"screen:destroy".into()));
    assert!(!host.events().contains(&"shutdown:A".into()));
    let mut host = Host {
        tutorials: BTreeSet::from(["B".into()]),
        default_tutorial: "b".into(),
        ..Default::default()
    };
    assert_eq!(fixture(&mut host).get_active_region_index(), Some(1));
}

#[test]
fn lifecycle_rereads_tuning_releases_catalog_and_guards_only_directories() {
    let process = ProcessDirectories::default();
    let mut host = Host::default();
    let mut nation = Nation::new();
    nation.init(&process, &mut host).unwrap();
    host.clear();
    nation.init(&process, &mut host).unwrap();
    assert!(host.events().is_empty());
    nation.shutdown(&mut host);
    let events = host.events();
    assert_eq!(events.iter().filter(|s| s.starts_with("subscription:")).count(), 3);
    assert_eq!(&events[3..], ["shutdown:A", "drop:A", "drop:B", "drop:C"]);
    assert_eq!(nation.get_region_count(), 0);
    assert_eq!(nation.get_active_region_index(), Some(0));
    assert_eq!(
        nation.get_economy_modifier(0, date(), None),
        Err(NationError::NotInitialized)
    );
    host.clear();
    nation.shutdown(&mut host);
    assert!(host.events().is_empty());
    host.tuning = Some(TuningProperties {
        difficulty: Some(vec![0.75]),
        ..Default::default()
    });
    nation.init(&process, &mut host).unwrap();
    assert!(host.events().contains(&"tuning".into()));
    assert!(!host.events().contains(&"root".into()));
    assert_eq!(nation.get_economy_modifier(0, date(), Some(0)), Ok(0.75));
    let process = ProcessDirectories::default();
    let mut host = Host {
        app: false,
        ..Default::default()
    };
    assert_eq!(process.static_init(&mut host), &Directories::default());
    host.app = true;
    assert_eq!(process.static_init(&mut host), &Directories::default());
    assert!(!host.events().contains(&"root".into()));
}

#[test]
fn deletion_order_neighbors_last_region_and_null_creation() {
    let mut host = Host::default();
    let mut nation = fixture(&mut host);
    nation.delete_region(&mut host, 1).unwrap();
    assert_eq!(
        host.events(),
        [
            "shutdown:A",
            "init:B",
            "delete:B",
            "shutdown:B",
            "init:C",
            "remove:U\\B:false",
            "drop:B"
        ]
    );
    assert_eq!(nation.get_region_count(), 2);
    assert_eq!(nation.get_active_region_index(), Some(1));
    host.clear();
    nation.delete_region(&mut host, 1).unwrap();
    assert_eq!(
        host.events(),
        ["delete:C", "shutdown:C", "init:A", "remove:U\\C:false", "drop:C"]
    );
    host.clear();
    assert_eq!(nation.delete_region(&mut host, 0), Err(NationError::LastRegion));
    assert!(host.events().is_empty());
    host.factory_fails = true;
    assert_eq!(
        nation.add_new_region(&mut host, "new"),
        Err(NationError::RegionCreationFailed)
    );
    assert_eq!(nation.get_region_count(), 1);
    assert_eq!(nation.get_active_region_index(), Some(0));
    host.factory_fails = false;
    assert_eq!(nation.add_new_region(&mut host, "new"), Ok(1));
    assert_eq!(nation.get_active_region_index(), Some(0));
}

#[test]
fn template_copy_order_paths_overwrite_and_starter_fallback() {
    let mut host = Host {
        entries: vec![],
        after_copy: vec![("copied".into(), 4)],
        ..Default::default()
    };
    let mut nation = Nation::new();
    nation.init(&ProcessDirectories::default(), &mut host).unwrap();
    assert_eq!(nation.get_region(0).unwrap().directory_name(), "copied");
    let files: Vec<_> = host.events().into_iter().filter(|s| s.starts_with("files:")).collect();
    assert_eq!(files, ["files:D\\Regions\\", "files:D\\French\\Regions\\"]);
    assert!(host
        .events()
        .contains(&"copy:D\\Regions\\nested\\one.sc4:U\\nested\\one.sc4".into()));
    assert!(host.events().contains(&"writable:U\\nested\\one.sc4".into()));
    let mut host = Host {
        entries: vec![],
        root_exists: false,
        ..Default::default()
    };
    let mut nation = Nation::new();
    nation.init(&ProcessDirectories::default(), &mut host).unwrap();
    assert!(host.events().contains(&"factory:".into()));
    assert!(!host.events().iter().any(|s| s.starts_with("files:")));
    assert_eq!(nation.get_region_count(), 1);
}

#[test]
fn tutorial_retention_exports_failures_and_screen_lifecycle() {
    let mut host = Host::default();
    let mut nation = fixture(&mut host);
    host.registered = vec!["a".into()];
    host.screen = Some(REGION_SCREEN);
    host.export_fails.insert(1);
    host.read_fails.insert("U\\A\\Tutorial-City-00000002.sc4".into());
    assert_eq!(
        nation.do_message(&mut host, RESET_TUTORIAL_CITIES),
        Ok(MessageOutcome::TutorialReset)
    );
    let events = host.events();
    assert_eq!(events.first().unwrap(), "screen:destroy");
    assert_eq!(events.last().unwrap(), "screen:create");
    assert!(!events.iter().any(|s| s.starts_with("files:")));
    assert!(events.contains(&"active:1:false".into()));
    assert!(events.contains(&"active:2:false".into()));
    let moved = events.iter().position(|s| s == "move:A:4:6").unwrap();
    assert_eq!(
        &events[moved..moved + 4],
        ["move:A:4:6", "city:shutdown", "city:drop", "active:3:true"]
    );
    host.clear();
    host.city = true;
    nation.reset_tutorial_cities(&mut host).unwrap();
    assert!(host.events().is_empty());
    host.city = false;
    host.app = false;
    nation.reset_tutorial_cities(&mut host).unwrap();
    assert!(host.events().is_empty());
}

#[test]
fn tutorial_recreation_default_first_and_early_failure_keep_partial_effects() {
    let mut host = Host::default();
    let mut nation = fixture(&mut host);
    host.registered = vec!["a".into(), "b".into()];
    host.default_tutorial = "b".into();
    host.valid = false;
    host.screen = Some(REGION_SCREEN);
    nation.reset_tutorial_cities(&mut host).unwrap();
    let events = host.events();
    let factories: Vec<_> = events.iter().filter(|s| s.starts_with("factory:")).collect();
    assert_eq!(factories, [&"factory:b", &"factory:a"]);
    assert!(events.contains(&"files:D\\Regions\\b".into()));
    assert_eq!(nation.get_active_region().unwrap().directory_name(), "a");
    host.clear();
    host.registered = vec!["unlisted".into()];
    host.factory_fails = true;
    assert_eq!(
        nation.reset_tutorial_cities(&mut host),
        Err(NationError::RegionCreationFailed)
    );
    assert!(host.events().contains(&"remove:U\\unlisted:true".into()));
    assert!(!host.events().contains(&"screen:create".into()));
    assert_eq!(nation.get_active_region().unwrap().directory_name(), "a");
}

#[test]
fn nq3_numeric_and_error_vectors() {
    let p = TuningProperties {
        demand_ids: Some(vec![7]),
        curves: BTreeMap::from([(FIRST_CURVE_PROPERTY, vec![0.0, 2.0])]),
        difficulty: Some(vec![0.25]),
    };
    let tuning = EconomyTuning::from_properties(Some(&p));
    assert_eq!(tuning.get_economy_modifier(7, date(), None), Ok(2.0));
    assert_eq!(
        tuning.get_economy_modifier(8, date(), None),
        Err(NationError::MissingCurve(8))
    );
    let p = TuningProperties {
        demand_ids: Some(vec![0]),
        ..p
    };
    let tuning = EconomyTuning::from_properties(Some(&p));
    assert_eq!(tuning.get_economy_modifier(9, date(), None), Ok(2.0));
    assert_eq!(tuning.get_economy_modifier(9, date(), Some(0)), Ok(2.25));
    assert_eq!(
        tuning.get_economy_modifier(9, date(), Some(1)),
        Err(NationError::InvalidDifficulty(1))
    );
}

#[test]
fn preference_uses_display_bytes_and_sixty_four_byte_prefix_without_probing() {
    let long_name = "R".repeat(70);
    let mut host = Host {
        entries: vec![("Alpha".into(), 4), (long_name.clone(), 4)],
        preference: [b'R'; 64],
        ..Default::default()
    };
    let mut nation = Nation::new();
    nation.init(&ProcessDirectories::default(), &mut host).unwrap();
    assert_eq!(nation.get_active_region_index(), Some(1));
    let events = host.events();
    let lifecycle: Vec<_> = events
        .iter()
        .filter(|s| s.starts_with("init:") || s.starts_with("shutdown:"))
        .collect();
    assert_eq!(lifecycle, [&format!("init:{long_name}")]);
    let setters: Vec<_> = events
        .iter()
        .filter(|s| s.starts_with("directory:") || s.starts_with("display:") || s.starts_with("ini:"))
        .cloned()
        .collect();
    assert_eq!(
        setters,
        [
            "directory:Alpha".to_string(),
            "display:Alpha".into(),
            "ini:Alpha".into(),
            format!("directory:{long_name}"),
            format!("display:{long_name}"),
            format!("ini:{long_name}")
        ]
    );
    host.clear();
    for message in [POST_CITY_INIT, PRE_CITY_SHUTDOWN, 0] {
        assert_eq!(nation.do_message(&mut host, message), Ok(MessageOutcome::Ignored));
    }
    assert!(host.events().is_empty());
}

#[test]
fn deleting_last_catalog_entry_selects_previous_before_compaction() {
    let mut host = Host::default();
    let mut nation = fixture(&mut host);
    nation.delete_region(&mut host, 2).unwrap();
    assert_eq!(
        host.events(),
        [
            "shutdown:A",
            "init:C",
            "delete:C",
            "shutdown:C",
            "init:B",
            "remove:U\\C:false",
            "drop:C"
        ]
    );
    assert_eq!(nation.get_active_region_index(), Some(1));
    assert_eq!(nation.get_active_region().unwrap().directory_name(), "B");
}

#[test]
fn recursive_copy_creates_parents_and_continues_after_ignored_results() {
    let mut host = Host {
        root_exists: false,
        ..Default::default()
    };
    Nation::<Region>::copy_region_files(&mut host, "U\\target", "D\\source");
    assert_eq!(
        host.events(),
        [
            "files:D\\source",
            "exists:U\\target\\nested",
            "mkdir:U\\target\\nested",
            "copy:D\\source\\nested\\one.sc4:U\\target\\nested\\one.sc4",
            "writable:U\\target\\nested\\one.sc4",
            "exists:U\\target",
            "mkdir:U\\target",
            "copy:D\\source\\region.ini:U\\target\\region.ini",
            "writable:U\\target\\region.ini",
        ]
    );
}
