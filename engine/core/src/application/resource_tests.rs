// SPDX-License-Identifier: GPL-3.0-or-later
use super::{resources::*, *};
use std::collections::{BTreeMap, VecDeque};

type RegistryEntries = Vec<(Vec<u8>, Vec<u8>)>;

#[derive(Default)]
struct Files {
    sections: BTreeMap<Vec<u8>, RegistryEntries>,
    directories: BTreeMap<Vec<u8>, Vec<DirectoryEntry>>,
    paths: Vec<Vec<u8>>,
    log: Vec<String>,
    factories: Vec<(u32, u32, u32)>,
    count: u32,
    next: u64,
    reject_open: VecDeque<bool>,
    reject_init: VecDeque<bool>,
}
impl ResourceOwnerHost for Files {
    fn language_name(&mut self, _: i32) -> Option<Vec<u8>> {
        None
    }
    fn registry_entries(&mut self, _: &[u8], section: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.sections.get(section).cloned().unwrap_or_default()
    }
    fn directory_exists(&mut self, _: &[u8]) -> bool {
        false
    }
    fn create_directory(&mut self, path: &[u8]) {
        self.log.push(format!("mkdir:{}", String::from_utf8_lossy(path)));
    }
    fn enumerate(&mut self, path: &[u8], pattern: &[u8]) -> Vec<DirectoryEntry> {
        self.log.push(format!(
            "enum:{}:{}",
            String::from_utf8_lossy(path),
            String::from_utf8_lossy(pattern)
        ));
        self.directories.get(path).cloned().unwrap_or_default()
    }
    fn is_database_file(&mut self, path: &[u8], mode: u32) -> i32 {
        assert_eq!(mode, 1);
        self.log.push(format!("probe:{}", String::from_utf8_lossy(path)));
        if path.ends_with(b"bad") {
            -1
        } else {
            0
        }
    }
    fn create_segment(&mut self, class: u32, interface: u32) -> Option<Token> {
        assert_eq!(interface, SEGMENT_INTERFACE);
        self.next += 1;
        self.log.push(format!("create:{}:{class:X}", self.next));
        Some(self.next)
    }
    fn initialize_segment(&mut self, segment: Token) -> bool {
        self.log.push(format!("init:{segment}"));
        !self.reject_init.pop_front().unwrap_or(false)
    }
    fn set_segment_path(&mut self, segment: Token, path: &[u8]) -> bool {
        self.log
            .push(format!("path:{segment}:{}", String::from_utf8_lossy(path)));
        self.paths.push(path.to_vec());
        true
    }
    fn open_segment(&mut self, segment: Token, mode: u32, flags: u32) -> bool {
        assert_eq!((mode, flags), (1, 0));
        self.log.push(format!("open:{segment}"));
        !self.reject_open.pop_front().unwrap_or(false)
    }
    fn register_segment(&mut self, segment: Token, id: Option<u32>) -> bool {
        self.log.push(format!("register:{segment}:{id:?}"));
        true
    }
    fn shutdown_segment(&mut self, segment: Token) {
        self.log.push(format!("shutdown:{segment}"));
    }
    fn release_segment(&mut self, segment: Token) {
        self.log.push(format!("release:{segment}"));
    }
    fn register_factory(&mut self, resource_type: u32, class: u32, flags: u32) {
        self.factories.push((resource_type, class, flags));
    }
    fn resource_count(&mut self) -> u32 {
        self.count
    }
}

fn pair(name: &[u8], value: &[u8]) -> (Vec<u8>, Vec<u8>) {
    (name.to_vec(), value.to_vec())
}
fn file(name: &[u8]) -> DirectoryEntry {
    DirectoryEntry {
        name: name.to_vec(),
        directory: false,
    }
}
fn directory(name: &[u8]) -> DirectoryEntry {
    DirectoryEntry {
        name: name.to_vec(),
        directory: true,
    }
}

#[test]
fn registry_hex_malformed_substitutions_groups_and_raw_directory_values() {
    for (text, expected) in [
        (b"12junk".as_slice(), 0x12),
        (b"123456789", 0x12345678),
        (b"-1", 0),
        (b" \t0XAb", 0xAB),
        (b"zero", 0),
    ] {
        assert_eq!(hex_id(text), expected);
    }
    let mut app = super::tests::app();
    app.directories.data_7c = b"D".to_vec();
    let mut h = Files::default();
    h.sections.insert(
        b"DBSegments".to_vec(),
        vec![
            pair(b"12junk", b" 0xAb;pre;%DataDirectory%/%Language%"),
            pair(b"-1", b"whole-path"),
            pair(b"BEGIN_GROUP", b"PACKED_EXEMPLARS"),
            pair(b"BEGIN_GROUP", b"PACKED_EXEMPLARS"),
            pair(b"1", b"1;hidden"),
            pair(b"END_GROUP", b"ignored"),
            pair(b"2", b"2;outer-condition-lost"),
            pair(b"END_GROUP", b"ignored"),
            pair(b"BEGIN_GROUP", b"unknown"),
            pair(b"3", b"3;unknown-does-not-gate"),
        ],
    );
    h.sections
        .insert(b"DBSegmentDirectoryFiles".to_vec(), vec![pair(b"raw-key", b"x;y")]);
    let mut registry = Registry::default();
    assert_eq!(registry.load(&app, None, [Some(b"OFF"), Some(b"ON")], &mut h), Ok(()));
    assert_eq!(
        registry.sources.iter().map(|s| s.id).collect::<Vec<_>>(),
        [0x12, 0, 2, 3]
    );
    assert_eq!(
        registry.sources[0],
        Source {
            id: 0x12,
            class: 0xAB,
            path: b"pre;D/English".to_vec()
        }
    );
    assert_eq!(registry.sources[1].class, 0);
    assert_eq!(registry.sources[1].path, b"whole-path");
    assert!(registry.packed_textures);
    assert_eq!(registry.directory_files, [pair(b"raw-key", b"x;y")]);
    h.sections.insert(b"DBSegments".to_vec(), vec![pair(b"END_GROUP", b"")]);
    assert_eq!(registry.load(&app, None, [None, None], &mut h), Err(UnmatchedEndGroup));
}

#[test]
fn resources_keep_all_eight_passes_filters_native_order_and_failure_cleanup() {
    let mut app = super::tests::app();
    app.directories.plugin_a4 = b"P".to_vec();
    app.directories.sku_cc = b"S".to_vec();
    app.directories.slot_e4_108 = b"O".to_vec();
    app.directories.user_base_11c = b"U".to_vec();
    let registry = Registry {
        sources: vec![Source {
            id: 0x12,
            class: 0xAB,
            path: b"registry".to_vec(),
        }],
        ..Registry::default()
    };
    let mut h = Files {
        count: 2,
        reject_open: [true].into(),
        ..Files::default()
    };
    h.directories.insert(b"P".to_vec(), vec![file(b"a.dat")]);
    h.directories.insert(b"P\\English\\".to_vec(), vec![file(b"b.dat")]);
    h.directories.insert(
        b"S".to_vec(),
        vec![
            directory(b"z"),
            file(b"b.dll"),
            file(b"a"),
            file(b"skip.DAT"),
            directory(b"y"),
            file(b"bad"),
        ],
    );
    h.directories.insert(b"S\\z".to_vec(), vec![file(b"z1")]);
    h.directories.insert(b"S\\y".to_vec(), vec![file(b"y1")]);
    h.directories.insert(b"U\\Plugins\\".to_vec(), vec![file(b"user")]);
    assert!(!app.setup_resources(&registry, None, &mut h));
    assert_eq!(
        h.paths,
        [
            b"registry".as_slice(),
            b"P\\a.dat",
            b"P\\English\\b.dat",
            b"O",
            b"S\\b.dll",
            b"S\\a",
            b"S\\z\\z1",
            b"S\\y\\y1",
            b"S",
            b"U\\Plugins\\user",
            b"U\\Plugins\\"
        ]
    );
    assert!(!h
        .log
        .iter()
        .any(|s| s == "probe:S\\skip.DAT" || s.starts_with("probe:P")));
    assert!(h.log.windows(3).any(|s| s == ["open:1", "shutdown:1", "release:1"]));
    assert!(!h.log.iter().any(|s| s == "shutdown:2"));
    assert_eq!(h.factories, FACTORIES.map(|(ty, class)| (ty, class, 0)));
    h.count = 3;
    h.paths.clear();
    assert!(app.setup_resources(&registry, None, &mut h));
    assert_eq!(h.paths[0], b"registry");
}

#[test]
fn resource_initialization_failure_releases_without_shutdown_and_missing_ini_is_success() {
    let mut app = super::tests::app();
    let mut h = Files {
        reject_init: [true].into(),
        count: 3,
        ..Files::default()
    };
    let mut registry = Registry::default();
    assert_eq!(registry.load(&app, None, [None, None], &mut h), Ok(()));
    registry.sources.push(Source {
        id: 0,
        class: 0,
        path: b"zero".to_vec(),
    });
    assert!(app.setup_resources(&registry, None, &mut h));
    assert_eq!(&h.log[..3], ["create:1:0", "init:1", "release:1"]);
    assert!(!h.log.iter().any(|s| s == "shutdown:1"));
}
