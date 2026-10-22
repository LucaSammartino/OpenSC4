// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Register,
    Open(ResourceKey),
    Release(u32),
}

struct File(u32, Vec<u8>);

impl AsRef<[u8]> for File {
    fn as_ref(&self) -> &[u8] {
        &self.1
    }
}

#[derive(Default)]
struct Resources {
    files: BTreeMap<u32, Vec<u8>>,
    events: Vec<Event>,
    callback: Option<FeatureProcess>,
    reenter: bool,
}

impl PackResources for Resources {
    type File = File;

    fn register_static_shutdown(&mut self, process: &FeatureProcess) {
        self.events.push(Event::Register);
        self.callback = Some(process.clone());
        if self.reenter {
            // The guard must be set before this call and no state lock held.
            assert!(process.scan(self));
            assert!(!process.is_feature_enabled(1));
        }
    }

    fn open_resource(&mut self, key: ResourceKey) -> Option<File> {
        self.events.push(Event::Open(key));
        let pack = key.instance_id - FIRST_PACK_INSTANCE + 1;
        self.files.get(&pack).map(|bytes| File(pack, bytes.clone()))
    }

    fn release_file(&mut self, file: File) {
        // Installed bit is published before release; a foreign getter can read it.
        assert!(self.callback.as_ref().unwrap().is_expansion_pack_installed(file.0));
        self.events.push(Event::Release(file.0));
    }
}

fn resources(files: &[(u32, &[u8])]) -> Resources {
    Resources {
        files: files.iter().map(|(n, b)| (*n, b.to_vec())).collect(),
        ..Resources::default()
    }
}

#[test]
fn empty_store_and_each_single_pack_cover_all_shift_boundaries() {
    let mut empty = FeatureManager::new(FeatureProcess::new());
    assert_eq!(empty.init(&mut Resources::default()), Some(true));
    assert_eq!(empty.get_current_expansion_pack(), Some(0));
    for n in 0..=33 {
        assert_eq!(empty.is_expansion_pack_installed(n), Some(false));
    }
    for pack in 1..=32 {
        let mut manager = FeatureManager::new(FeatureProcess::new());
        manager.init(&mut resources(&[(pack, b"")]));
        assert_eq!(manager.get_current_expansion_pack(), Some(pack));
        for n in 1..=32 {
            assert_eq!(manager.is_expansion_pack_installed(n), Some(n == pack));
            assert_eq!(manager.is_expansion_pack_installed(n + 32), Some(n == pack));
        }
        assert_eq!(manager.is_expansion_pack_installed(0), Some(pack == 32));
    }
}

#[test]
fn scan_exact_keys_order_references_guard_and_non_ini_presence() {
    let process = FeatureProcess::new();
    let mut store = resources(&[(1, b"[Features]\n1=on"), (3, b"not an ini"), (32, b"")]);
    store.reenter = true;
    let mut manager = FeatureManager::new(process.clone());
    assert_eq!(manager.initialized(), Some(false));
    assert_eq!(manager.get_current_expansion_pack(), Some(0));
    assert_eq!(manager.init(&mut store), Some(true));
    let mut expected = vec![Event::Register];
    for k in 0..32 {
        expected.push(Event::Open(ResourceKey {
            type_id: 0,
            group_id: 0x8A59_71C5,
            instance_id: 0x2B56_3701 + k,
        }));
        if matches!(k, 0 | 2 | 31) {
            expected.push(Event::Release(k + 1));
        }
    }
    assert_eq!(store.events, expected);
    assert_eq!(manager.get_current_expansion_pack(), Some(32));
    for n in [0, 32, 64, 0x8000_0000] {
        assert_eq!(manager.is_expansion_pack_installed(n), Some(true), "{n}");
    }
    for n in [1, 3, 33, 35, 0x8000_0001, u32::MAX - 28] {
        assert_eq!(manager.is_expansion_pack_installed(n), Some(true), "{n}");
    }
    assert_eq!(manager.is_expansion_pack_installed(u32::MAX), Some(false));
    assert_eq!(manager.is_feature_enabled(1), Some(true));
    let calls = store.events.len();
    assert_eq!(manager.init(&mut store), Some(true));
    assert_eq!(store.events.len(), calls);
    let mut second = FeatureManager::new(process);
    assert_eq!(second.init(&mut store), Some(true));
    assert_eq!(store.events.len(), calls);
}

#[test]
fn ini_bytes_first_header_delimiters_trimming_and_exact_off() {
    let mut store = resources(&[(
        1,
        b"[ Features ]\r1=on\r\n\t[FEATURES]ignored\r\n \
        0x10\t=\tOFF\t\r20=yes\n1A=enabled\rxyz=on\nabc=on\n\
        0x40=0\r0x50=off ; note\n;0x60=on\n0x70= \n =on\n\
        0x80=off=extra\n0x90=off\n0x90=ON\n0x91=ON\n0x91=off\n\
        0xFFFFFFFF=on\n4294967296=off\n0xFFFFFFFF=on\n\
        [Features]\n0x99=on\n",
    )]);
    let mut manager = FeatureManager::new(FeatureProcess::new());
    manager.init(&mut store);
    for id in [0, 20, 26, 0xABC, 0x40, 0x50, 0x80, 0x90, u32::MAX] {
        assert_eq!(manager.is_feature_enabled(id), Some(true), "{id:#x}");
    }
    for id in [1, 0x10, 0x60, 0x70, 0x91, 0x99] {
        assert_eq!(manager.is_feature_enabled(id), Some(false), "{id:#x}");
    }
}

#[test]
fn malformed_section_headers_end_first_section_and_raw_bytes_are_not_lossy() {
    for end in [b"[]".as_slice(), b"[broken", b"[Other]", b"[Features]"] {
        let mut bytes = b"[Features]\n1=on\n".to_vec();
        bytes.extend_from_slice(end);
        bytes.extend_from_slice(b"\n2=on\n");
        let mut store = resources(&[(1, &bytes)]);
        let mut manager = FeatureManager::new(FeatureProcess::new());
        manager.init(&mut store);
        assert_eq!(manager.is_feature_enabled(1), Some(true));
        assert_eq!(manager.is_feature_enabled(2), Some(false));
    }
    let mut store = resources(&[(1, b"[Features]\n9=o\xFFf\n8=off\n")]);
    let mut manager = FeatureManager::new(FeatureProcess::new());
    manager.init(&mut store);
    assert_eq!(manager.is_feature_enabled(9), Some(true));
    assert_eq!(manager.is_feature_enabled(8), Some(false));
}

#[test]
fn shutdown_destruction_and_static_reset_have_separate_scopes() {
    let process = FeatureProcess::new();
    let mut store = resources(&[(1, b"[Features]\n1=on"), (2, b"[Features]\n1=off\n2=on")]);
    let mut first = FeatureManager::new(process.clone());
    first.init(&mut store);
    assert_eq!(first.is_feature_enabled(1), Some(false));
    assert_eq!(first.is_feature_enabled(2), Some(true));
    first.shutdown();
    store.files.clear();
    first.init(&mut store);
    assert_eq!(store.events.iter().filter(|e| matches!(e, Event::Register)).count(), 1);
    assert_eq!(first.release(), Some(0));
    assert!(process.is_feature_enabled(2));
    let mut second = FeatureManager::new(process.clone());
    second.init(&mut store);
    assert_eq!(second.get_current_expansion_pack(), Some(2));
    assert!(store.callback.as_ref().unwrap().static_shutdown());
    assert!(process.static_shutdown());
    assert_eq!(second.get_current_expansion_pack(), Some(0));
    assert_eq!(second.is_feature_enabled(2), Some(false));
    store.files.insert(32, b"[Features]\n3=on".to_vec());
    second.init(&mut store); // Already initialized: does not trigger static rescan.
    assert_eq!(second.get_current_expansion_pack(), Some(0));
    second.shutdown();
    second.init(&mut store);
    assert_eq!(second.get_current_expansion_pack(), Some(32));
    assert_eq!(second.is_feature_enabled(3), Some(true));
    assert_eq!(store.events.iter().filter(|e| matches!(e, Event::Register)).count(), 2);
}

#[test]
fn query_interface_preserves_output_identity_and_reference_count() {
    let process = FeatureProcess::new();
    let mut manager = FeatureManager::new(process.clone());
    let other = FeatureManager::new(process);
    let borrowed = manager.interface().unwrap();
    assert_eq!(manager.reference_count(), Some(0));
    let mut output = other.interface();
    let sentinel = output.clone();
    assert!(!manager.query_interface(0x1234_5678, &mut output));
    assert_eq!(output, sentinel);
    assert_eq!(manager.reference_count(), Some(0));
    for iid in [IID_UNKNOWN, IID_FEATURE_MANAGER] {
        assert!(manager.query_interface(iid, &mut output));
        assert_eq!(output.as_ref(), Some(&borrowed));
    }
    assert_eq!(manager.reference_count(), Some(2));
    assert_eq!(manager.release(), Some(1));
    assert_eq!(manager.release(), Some(0));
    let deletion = manager.deletion().unwrap();
    assert_eq!(deletion.count_at_destruction, 1); // Not decremented first.
    assert!(!deletion.initialized_at_destruction);
    assert!(deletion.allocation_freed);
    assert_eq!(deletion.returned_this, borrowed);
    assert_eq!(manager.add_ref(), None); // Safe Rust terminal-handle extension only.
    assert_eq!(manager.release(), None);
}

#[test]
fn signed_release_boundary_zero_and_negative_destroy_without_decrement() {
    for count in [i32::MIN, -1, 0, 1, 2, i32::MAX] {
        let mut manager = FeatureManager::new(FeatureProcess::new());
        manager.object_mut().unwrap().references = count;
        manager.init(&mut Resources::default());
        let result = manager.release();
        if count > 1 {
            assert_eq!(result, Some((count - 1) as u32));
            assert_eq!(manager.initialized(), Some(true));
            assert!(manager.deletion().is_none());
        } else {
            assert_eq!(result, Some(0));
            let deleted = manager.deletion().unwrap();
            assert_eq!(deleted.count_at_destruction, count);
            assert!(!deleted.initialized_at_destruction);
        }
    }
}

#[test]
fn increment_uses_32_bit_register_bits_fm4() {
    // Published independent FM4 bit vectors, not native overflow injection.
    for (initial, incremented, released) in [
        (0x7fff_fffeu32, 0x7fff_ffffu32, 0x7fff_fffeu32),
        (0x7fff_ffff, 0x8000_0000, 0),
        (0xffff_ffff, 0, 0),
    ] {
        let process = FeatureProcess::new();
        let mut manager = FeatureManager::new(process.clone());
        manager.init(&mut resources(&[(3, b"[Features]\n16=on\n17=off")]));
        manager.object_mut().unwrap().references = initial as i32;
        assert_eq!(manager.add_ref(), Some(incremented));
        assert_eq!(manager.reference_count(), Some(incremented as i32));
        assert_eq!(manager.release(), Some(released));
        if released == 0 {
            let deletion = manager.deletion().unwrap();
            assert_eq!(deletion.count_at_destruction, incremented as i32);
            assert!(!deletion.initialized_at_destruction);
            assert!(deletion.allocation_freed);
        } else {
            assert!(manager.deletion().is_none());
            assert_eq!(manager.initialized(), Some(true));
        }
        assert!(process.is_feature_enabled(16));
        assert!(!process.is_feature_enabled(17));
        assert!(process.is_expansion_pack_installed(3));
        assert_eq!(process.mentioned_features(), vec![16, 17]);
    }
}

#[test]
fn deleting_destructor_only_bit_zero_frees_and_never_calls_shutdown() {
    for flags in [0, 1, 2, 3, 0x8000_0000, u32::MAX] {
        let process = FeatureProcess::new();
        let mut manager = FeatureManager::new(process.clone());
        let mut store = resources(&[(1, b"[Features]\n1=on")]);
        manager.init(&mut store);
        let identity = manager.interface().unwrap();
        manager.add_ref();
        let deletion = manager.deleting_destructor(flags).unwrap();
        assert_eq!(deletion.allocation_freed, flags & 1 != 0);
        assert_eq!(manager.object.is_none(), flags & 1 != 0);
        assert_eq!(deletion.returned_this, identity);
        assert!(deletion.initialized_at_destruction);
        assert_eq!(deletion.count_at_destruction, 1);
        assert!(process.is_feature_enabled(1));
        assert!(process.is_expansion_pack_installed(1));
        drop(manager);
        assert!(process.is_feature_enabled(1));
    }
}

#[test]
fn current_pack_dispatches_descending_through_overridden_slot_six() {
    use std::sync::Mutex;
    struct Override {
        calls: Mutex<Vec<u32>>,
        installed: u32,
    }
    impl ExpansionPackQuery for Override {
        fn is_expansion_pack_installed(&self, n: u32) -> bool {
            self.calls.lock().unwrap().push(n);
            n == self.installed
        }
    }
    for installed in [0, 1, 17, 32] {
        let object = Override {
            calls: Mutex::default(),
            installed,
        };
        assert_eq!(object.get_current_expansion_pack(), installed);
        let expected: Vec<_> = (installed.max(1)..=32).rev().collect();
        assert_eq!(*object.calls.lock().unwrap(), expected);
    }
}

#[test]
fn getters_can_run_on_a_second_thread_during_scan() {
    let process = FeatureProcess::new();
    let observer = FeatureManager::new(process.clone());
    let mut writer = FeatureManager::new(process);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            for _ in 0..1_000 {
                assert!(observer.get_current_expansion_pack().unwrap() <= 32);
                observer.is_feature_enabled(1);
            }
        });
        writer.init(&mut resources(&[(1, b"[Features]\n1=on")]));
    });
    assert_eq!(observer.is_feature_enabled(1), Some(true));
}
