//! Independent chapter 6 vectors. The fake host supplies only scripted primitive OS results.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use super::*;

#[derive(Debug, PartialEq, Eq)]
enum Call {
    First(Vec<u8>),
    Next(u32),
    Finish(u32),
    Create(Vec<u8>),
    Remove(Vec<u8>, bool),
}

#[derive(Default)]
struct Host {
    finds: BTreeMap<Vec<u8>, Vec<DirectoryMatch>>,
    active: BTreeMap<u32, VecDeque<DirectoryMatch>>,
    serial: u32,
    calls: Vec<Call>,
    created: BTreeSet<Vec<u8>>,
    fail_create: BTreeSet<Vec<u8>>,
    nested_hook: bool,
    threaded_hook: bool,
    nested_output: VecDeque<Vec<u8>>,
}

impl Host {
    fn set(&mut self, pattern: &[u8], matches: &[(&[u8], u32)]) {
        self.finds.insert(
            pattern.to_vec(),
            matches
                .iter()
                .map(|(name, attributes)| DirectoryMatch {
                    name: name.to_vec(),
                    attributes: *attributes,
                })
                .collect(),
        );
    }

    fn patterns(&self) -> Vec<Vec<u8>> {
        self.calls
            .iter()
            .filter_map(|call| match call {
                Call::First(pattern) => Some(pattern.clone()),
                _ => None,
            })
            .collect()
    }

    fn assert_closed(&self) {
        assert!(self.active.is_empty());
        assert_eq!(
            self.calls.iter().filter(|call| matches!(call, Call::Finish(_))).count(),
            self.serial as usize
        );
    }
}

impl RzDirectoryHost for Host {
    type FindHandle = u32;

    fn find_first(&mut self, pattern: &[u8]) -> Option<(u32, DirectoryMatch)> {
        self.calls.push(Call::First(pattern.to_vec()));
        if self.nested_hook && pattern == b"outer\\*" {
            self.nested_hook = false;
            let mut other = RzDirectory::new(self, b"", false, false, false);
            let mut out = VecDeque::new();
            assert!(other.read_directory_file_entries_into_string_list_recursive(self, b"nested\\", &mut out, false,));
            self.nested_output = out;
        }
        if self.threaded_hook && pattern == b"outer\\*" {
            self.threaded_hook = false;
            self.nested_output = std::thread::spawn(|| {
                let mut host = Host::default();
                host.set(b"thread\\*", &[(b"thread-file", 0)]);
                let mut owner = RzDirectory::new(&mut host, b"", false, false, false);
                let mut out = VecDeque::new();
                assert!(owner.read_directory_file_entries_into_string_list_recursive(
                    &mut host,
                    b"thread\\",
                    &mut out,
                    false,
                ));
                host.assert_closed();
                out
            })
            .join()
            .unwrap();
        }
        let mut entries: VecDeque<_> = self.finds.get(pattern)?.clone().into();
        let first = entries.pop_front()?;
        self.serial += 1;
        self.active.insert(self.serial, entries);
        Some((self.serial, first))
    }

    fn find_next(&mut self, handle: &mut u32) -> Option<DirectoryMatch> {
        self.calls.push(Call::Next(*handle));
        self.active.get_mut(handle).unwrap().pop_front()
    }

    fn find_finish(&mut self, handle: u32) {
        self.calls.push(Call::Finish(handle));
        assert!(self.active.remove(&handle).is_some(), "finish exactly once");
    }

    fn create_directory(&mut self, path: &[u8]) -> bool {
        self.calls.push(Call::Create(path.to_vec()));
        if self.fail_create.contains(path) {
            return false;
        }
        self.created.insert(path.to_vec());
        true
    }

    fn remove_directory(&mut self, path: &[u8]) -> bool {
        let has_child = self
            .created
            .iter()
            .any(|child| child.len() > path.len() && child.starts_with(path));
        let removed = !has_child && self.created.remove(path);
        self.calls.push(Call::Remove(path.to_vec(), removed));
        removed
    }
}

fn names(values: &[&[u8]]) -> VecDeque<Vec<u8>> {
    values.iter().map(|value| value.to_vec()).collect()
}

fn entry_values(owner: &RzDirectory) -> Vec<(Vec<u8>, u32)> {
    owner
        .entries()
        .iter()
        .map(|entry| (entry.name.clone(), entry.entry_type))
        .collect()
}

fn expected_entries(values: &[(&[u8], u32)]) -> Vec<(Vec<u8>, u32)> {
    values.iter().map(|(name, kind)| (name.to_vec(), *kind)).collect()
}

#[test]
fn construction_and_separator_byte_vectors() {
    let mut host = Host::default();
    for (input, expected) in [
        (b"".as_slice(), b"\\".as_slice()),
        (b"C:", b"C:\\"),
        (b"C:\\a/../ X ", b"C:\\a/../ X \\"),
        (b"C:/a/", b"C:/a/"),
        (b"\\\\server\\share\\", b"\\\\server\\share\\"),
        (b"\xff\x80", b"\xff\x80\\"),
    ] {
        let owner = RzDirectory::new(&mut host, input, false, true, false);
        assert_eq!(owner.path(), expected);
        assert_eq!(owner.filter(), b"*");
        assert_eq!(owner.count(), 0);
        assert!(owner.get_nth_entry(0).is_none());
    }
    assert!(host.calls.is_empty());
    assert!(is_separator(b'/') && is_separator(b'\\'));
    assert!(!is_separator(0xff) && !is_separator(b':'));
}

#[test]
fn owner_keeps_os_order_and_only_attribute_10_matters() {
    let mut host = Host::default();
    host.set(b"X\\*", &[(b"z", 0x27), (b"Zoo", 0x12), (b"apple", 0), (b"\xff", 0x90)]);
    let owner = RzDirectory::new(&mut host, b"X", true, false, false);
    assert_eq!(
        entry_values(&owner),
        expected_entries(&[(b"z", 8), (b"Zoo", 4), (b"apple", 8), (b"\xff", 4)])
    );
    assert_eq!(owner.count(), 4);
    assert_eq!(owner.get_nth_entry(2).unwrap().name, b"apple");
    assert!(owner.get_nth_entry(4).is_none());
    assert!(owner.get_nth_entry(u32::MAX).is_none());
    assert_eq!(host.patterns(), vec![b"X\\*".to_vec()]);
    host.assert_closed();
}

#[test]
fn setters_without_read_preserve_snapshot_and_read_failure_clears_it() {
    let mut host = Host::default();
    host.set(b"old\\*", &[(b"old-entry", 0)]);
    let mut owner = RzDirectory::new(&mut host, b"old", true, false, false);
    let old = entry_values(&owner);
    assert!(owner.change_directory(&mut host, b"new/", false));
    owner.set_new_directory_entry_filter(&mut host, b"*.abc", false);
    assert_eq!(owner.path(), b"new/");
    assert_eq!(owner.filter(), b"*.abc");
    assert_eq!(entry_values(&owner), old);
    assert_eq!(owner.count(), 1);
    assert_eq!(host.patterns().len(), 1);
    assert!(!owner.read_directory_entries(&mut host));
    assert_eq!(owner.count(), 0);
    assert!(owner.entries().is_empty());
    assert!(owner.change_directory(&mut host, b"missing", true));
    owner.set_new_directory_entry_filter(&mut host, b"", true);
    assert_eq!(owner.filter(), b"*");
    assert_eq!(
        host.patterns(),
        vec![
            b"old\\*".to_vec(),
            b"new/*.abc".to_vec(),
            b"missing\\*.abc".to_vec(),
            b"missing\\*".to_vec(),
        ]
    );
    host.assert_closed();
}

#[test]
fn first_find_failure_does_not_synthesize_dots() {
    let mut host = Host::default();
    let owner = RzDirectory::new(&mut host, b"missing", true, true, true);
    assert_eq!(owner.count(), 0);
    let mut out = names(&[b"seed"]);
    assert!(!owner.read_directory_specific_entries_into_string_list(&mut host, &mut out, 15));
    assert_eq!(out, names(&[b"seed"]));
    assert_eq!(host.serial, 0);
    host.assert_closed();
}

#[test]
fn first_dot_is_special_but_repeated_directory_dots_are_ordinary() {
    let mut host = Host::default();
    host.set(
        b"X\\*",
        &[(b".", 0x10), (b"..", 0x10), (b"f", 0), (b".", 0x10), (b"..", 0x10)],
    );
    type DotVector<'a> = (bool, bool, &'a [(&'a [u8], u32)]);
    let vectors: &[DotVector<'_>] = &[
        (false, false, &[(b"f", 8), (b".", 4), (b"..", 4)]),
        (true, false, &[(b"..", 4), (b"f", 8), (b".", 4), (b"..", 4)]),
        (false, true, &[(b".", 4), (b"f", 8), (b".", 4), (b"..", 4)]),
        (true, true, &[(b".", 4), (b"..", 4), (b"f", 8), (b".", 4), (b"..", 4)]),
    ];
    for (parent, this, expected) in vectors {
        let owner = RzDirectory::new(&mut host, b"X", true, *parent, *this);
        assert_eq!(entry_values(&owner), expected_entries(expected));
        assert_eq!(owner.count() as usize, expected.len());
    }
    host.assert_closed();
}

#[test]
fn synthesis_is_front_parent_then_self_and_checks_existing_bare_names() {
    let mut host = Host::default();
    host.set(b"X\\*", &[(b"z", 0), (b"A", 0x10)]);
    let mut owner = RzDirectory::new(&mut host, b"X", true, true, true);
    assert_eq!(
        entry_values(&owner),
        expected_entries(&[(b".", 4), (b"..", 4), (b"z", 8), (b"A", 4)])
    );
    host.set(b"X\\*", &[(b".", 0), (b"..", 0)]);
    assert!(owner.read_directory_entries(&mut host));
    assert_eq!(entry_values(&owner), expected_entries(&[(b".", 8), (b"..", 8)]));
    assert_eq!(owner.count(), 2);
    // Non-star filters never synthesize. Host result is already matched; core does not rematch it.
    for filter in [b"*.*".as_slice(), b"*.ini", b"*\xff"] {
        let pattern = [b"X\\".as_slice(), filter].concat();
        host.set(&pattern, &[(b"unrelated-name", 0)]);
        owner.set_new_directory_entry_filter(&mut host, filter, true);
        assert_eq!(entry_values(&owner), expected_entries(&[(b"unrelated-name", 8)]));
    }
    host.assert_closed();
}

#[test]
fn successful_search_with_every_entry_skipped_still_returns_true() {
    let mut host = Host::default();
    host.set(b"X\\*", &[(b".", 0x10), (b"..", 0x10)]);
    let mut owner = RzDirectory::new(&mut host, b"X", false, false, false);
    assert!(owner.read_directory_entries(&mut host));
    assert_eq!(owner.count(), 0);
    host.assert_closed();
}

#[test]
fn only_exact_directory_dot_bytes_are_special_and_seen_dots_keep_os_position() {
    let mut host = Host::default();
    host.set(b"X\\*", &[(b". ", 0x10), (b".\xff", 0x10), (b".", 0x10)]);
    let owner = RzDirectory::new(&mut host, b"X", true, true, true);
    assert_eq!(
        entry_values(&owner),
        expected_entries(&[(b"..", 4), (b". ", 4), (b".\xff", 4), (b".", 4),])
    );
    assert_eq!(owner.count(), 4);
    host.assert_closed();
}

#[test]
fn all_flag_masks_are_independent_append_vectors() {
    let mut host = Host::default();
    host.set(b"X\\*", &[(b"f", 0), (b"..", 0x10), (b"sub", 0x10), (b".", 0x10)]);
    let owner = RzDirectory::new(&mut host, b"X", true, false, false);
    let snapshot = entry_values(&owner);
    // RZ9: 1/2 without 4 follow the literal skip-before-seen interpretation of §6.5.
    let expected: &[&[&[u8]]] = &[
        &[b"seed"],
        &[b".", b"seed"],
        &[b"..", b"seed"],
        &[b".", b"..", b"seed"],
        &[b"seed", b"sub"],
        &[b"seed", b"sub", b"."],
        &[b"seed", b"..", b"sub"],
        &[b"seed", b"..", b"sub", b"."],
        &[b"seed", b"f"],
        &[b".", b"seed", b"f"],
        &[b"..", b"seed", b"f"],
        &[b".", b"..", b"seed", b"f"],
        &[b"seed", b"f", b"sub"],
        &[b"seed", b"f", b"sub", b"."],
        &[b"seed", b"f", b"..", b"sub"],
        &[b"seed", b"f", b"..", b"sub", b"."],
    ];
    for (flags, expected) in expected.iter().enumerate() {
        let mut out = names(&[b"seed"]);
        assert!(owner.read_directory_specific_entries_into_string_list(
            &mut host,
            &mut out,
            flags as u32 | 0x8000_0000,
        ));
        assert_eq!(out, names(expected), "flags={flags}");
        assert_eq!(entry_values(&owner), snapshot);
        assert_eq!(owner.count(), 2);
    }
    host.assert_closed();
}

#[test]
fn caller_synthesis_has_no_duplicate_check_and_wrappers_mask_flags() {
    let mut host = Host::default();
    host.set(b"X\\*", &[(b"f", 0), (b"sub", 0x10)]);
    let owner = RzDirectory::new(&mut host, b"X", false, false, false);
    let mut out = names(&[b".", b"..", b"seed"]);
    assert!(owner.read_directory_specific_entries_into_string_list(&mut host, &mut out, 15));
    assert_eq!(out, names(&[b".", b"..", b".", b"..", b"seed", b"f", b"sub"]));
    let mut out = names(&[b"seed"]);
    assert!(owner.read_directory_directory_entries_into_string_list(&mut host, &mut out, u32::MAX));
    assert_eq!(out, names(&[b".", b"..", b"seed", b"sub"]));
    let mut out = names(&[b"seed"]);
    assert!(owner.read_directory_file_entries_into_string_list(&mut host, &mut out));
    assert_eq!(out, names(&[b"seed", b"f"]));
    host.assert_closed();
}

#[test]
fn caller_dot_duplicates_and_file_dot_names_are_not_owner_synthesis_rules() {
    let mut host = Host::default();
    host.set(b"X\\*", &[(b".", 0x10), (b"..", 0x10), (b".", 0x10), (b"..", 0x10)]);
    let mut owner = RzDirectory::new(&mut host, b"X", false, false, false);
    let mut out = VecDeque::new();
    assert!(owner.read_directory_directory_entries_into_string_list(&mut host, &mut out, 0));
    assert_eq!(out, names(&[b".", b".."]));
    host.set(b"X\\*", &[(b".", 0), (b"..", 0)]);
    out.clear();
    assert!(owner.read_directory_specific_entries_into_string_list(&mut host, &mut out, 15));
    assert_eq!(out, names(&[b".", b"..", b".", b".."]));
    host.set(b"X\\*.*", &[(b"f", 0)]);
    owner.set_new_directory_entry_filter(&mut host, b"*.*", false);
    out.clear();
    assert!(owner.read_directory_specific_entries_into_string_list(&mut host, &mut out, 15));
    assert_eq!(out, names(&[b"f"]));
    host.assert_closed();
}

#[test]
fn alphabetize_is_unsigned_bytewise_prefix_sensitive_and_preserves_duplicate_order() {
    let mut out = names(&[b"apple", b"\xff", b"a", b"Zoo", b"", b"a\x80", b"a", b"A", b"\x80"]);
    // Save allocation identity of equal strings to observe stable ordering independently of values.
    let first_a = out[2].as_ptr();
    let second_a = out[6].as_ptr();
    RzDirectory::alphabetize_entry_list(&mut out);
    assert_eq!(
        out,
        names(&[b"", b"A", b"Zoo", b"a", b"a", b"apple", b"a\x80", b"\x80", b"\xff"])
    );
    assert_eq!(out[3].as_ptr(), first_a);
    assert_eq!(out[4].as_ptr(), second_a);
}

static RECURSIVE_TEST_LOCK: Mutex<()> = Mutex::new(());

fn tree_host() -> Host {
    let mut host = Host::default();
    host.set(b"P\\*.dat", &[(b"z.dat", 0), (b"a.dat", 0)]);
    host.set(
        b"P\\*",
        &[(b"B", 0x10), (b".", 0x10), (b"ignored", 0), (b"A", 0x10), (b"..", 0x10)],
    );
    host.set(b"P\\B\\*.dat", &[(b"b.dat", 0)]);
    host.set(b"P\\B\\*", &[(b"D", 0x10)]);
    host.set(b"P\\B\\D\\*.dat", &[(b"d.dat", 0)]);
    host.set(b"P\\B\\D\\*", &[(b"not-a-dir", 0)]);
    host.set(b"P\\A\\*.dat", &[(b"a.dat", 0)]);
    host.set(b"P\\A\\*", &[(b"not-a-dir", 0)]);
    host
}

#[test]
fn recursive_preorder_global_depth_filter_restore_and_final_snapshot() {
    let _lock = RECURSIVE_TEST_LOCK.lock().unwrap();
    for include_top in [true, false] {
        let mut host = tree_host();
        let mut owner = RzDirectory::new(&mut host, b"old", false, true, true);
        owner.set_new_directory_entry_filter(&mut host, b"*.dat", false);
        let mut out = names(&[b"seed"]);
        assert!(owner.read_directory_file_entries_into_string_list_recursive(&mut host, b"P\\", &mut out, include_top,));
        let expected = if include_top {
            names(&[
                b"seed",
                b"P\\z.dat",
                b"P\\a.dat",
                b"P\\B\\b.dat",
                b"P\\B\\D\\d.dat",
                b"P\\A\\a.dat",
            ])
        } else {
            names(&[b"seed", b"P\\B\\b.dat", b"P\\B\\D\\d.dat", b"P\\A\\a.dat"])
        };
        assert_eq!(out, expected);
        assert_eq!(owner.path(), b"P\\A\\");
        assert_eq!(owner.filter(), b"*.dat");
        assert_eq!(entry_values(&owner), expected_entries(&[(b"a.dat", 8)]));
        let mut expected_patterns = vec![b"P\\*.dat".to_vec()];
        if include_top {
            expected_patterns.push(b"P\\*.dat".to_vec());
        }
        expected_patterns.extend([
            b"P\\*".to_vec(),
            b"P\\B\\*.dat".to_vec(),
            b"P\\B\\*.dat".to_vec(),
            b"P\\B\\*".to_vec(),
            b"P\\B\\D\\*.dat".to_vec(),
            b"P\\B\\D\\*.dat".to_vec(),
            b"P\\B\\D\\*".to_vec(),
            b"P\\A\\*.dat".to_vec(),
            b"P\\A\\*.dat".to_vec(),
            b"P\\A\\*".to_vec(),
        ]);
        assert_eq!(host.patterns(), expected_patterns);
        host.assert_closed();
    }
    assert_eq!(RECURSIVE_DEPTH.load(Ordering::SeqCst), 0);
}

#[test]
fn recursive_child_path_does_not_repair_missing_separator_and_keeps_byte_names() {
    let _lock = RECURSIVE_TEST_LOCK.lock().unwrap();
    let mut host = Host::default();
    host.set(b"C:\\P\\*", &[(b"top", 0), (b"\xff", 0x10)]);
    host.set(b"C:\\P\xff\\*", &[(b"\x80", 0)]);
    host.set(b"C:\\P\\\xff\\*", &[(b"wrong-repaired-path", 0)]);
    let mut owner = RzDirectory::new(&mut host, b"", false, false, false);
    let mut out = VecDeque::new();
    assert!(owner.read_directory_file_entries_into_string_list_recursive(&mut host, b"C:\\P", &mut out, true));
    assert_eq!(out, names(&[b"C:\\P\\top", b"C:\\P\xff\\\x80"]));
    assert_eq!(owner.path(), b"C:\\P\xff\\");
    host.assert_closed();
}

#[test]
fn recursive_first_failures_still_search_star_children_and_always_succeed() {
    let _lock = RECURSIVE_TEST_LOCK.lock().unwrap();
    let mut host = Host::default();
    host.set(b"P\\*", &[(b"child", 0x10)]);
    host.set(b"P\\child\\*.x", &[(b"x", 0)]);
    let mut owner = RzDirectory::new(&mut host, b"", false, false, false);
    owner.set_new_directory_entry_filter(&mut host, b"*.x", false);
    let mut out = names(&[b"seed"]);
    assert!(owner.read_directory_file_entries_into_string_list_recursive(&mut host, b"P\\", &mut out, true));
    assert_eq!(out, names(&[b"seed", b"P\\child\\x"]));
    assert!(owner.read_directory_file_entries_into_string_list_recursive(&mut host, b"absent", &mut out, true));
    assert_eq!(out, names(&[b"seed", b"P\\child\\x"]));
    assert_eq!(owner.path(), b"absent\\");
    assert_eq!(owner.count(), 0);
    assert_eq!(owner.filter(), b"*.x");
    assert_eq!(RECURSIVE_DEPTH.load(Ordering::SeqCst), 0);
    host.assert_closed();
}

#[test]
fn recursive_depth_is_shared_between_distinct_owners_and_restored_after_return() {
    let _lock = RECURSIVE_TEST_LOCK.lock().unwrap();
    let mut host = Host {
        nested_hook: true,
        ..Host::default()
    };
    host.set(b"outer\\*", &[(b"outer-file", 0)]);
    host.set(b"nested\\*", &[(b"nested-file", 0)]);
    let mut owner = RzDirectory::new(&mut host, b"", false, false, false);
    let mut out = VecDeque::new();
    assert!(owner.read_directory_file_entries_into_string_list_recursive(&mut host, b"outer\\", &mut out, false));
    assert!(out.is_empty());
    assert_eq!(host.nested_output, names(&[b"nested\\nested-file"]));
    assert!(owner.read_directory_file_entries_into_string_list_recursive(&mut host, b"nested\\", &mut out, false));
    assert!(out.is_empty());
    assert_eq!(RECURSIVE_DEPTH.load(Ordering::SeqCst), 0);
    host.assert_closed();
}

#[test]
fn recursive_depth_is_process_wide_rather_than_thread_local() {
    let _lock = RECURSIVE_TEST_LOCK.lock().unwrap();
    let mut host = Host {
        threaded_hook: true,
        ..Host::default()
    };
    host.set(b"outer\\*", &[(b"outer-file", 0)]);
    let mut owner = RzDirectory::new(&mut host, b"", false, false, false);
    let mut out = VecDeque::new();
    assert!(owner.read_directory_file_entries_into_string_list_recursive(&mut host, b"outer\\", &mut out, false,));
    assert!(out.is_empty());
    assert_eq!(host.nested_output, names(&[b"thread\\thread-file"]));
    assert_eq!(RECURSIVE_DEPTH.load(Ordering::SeqCst), 0);
    host.assert_closed();
}

#[test]
fn recursive_depth_has_no_invented_limit_or_max_path_truncation() {
    let _lock = RECURSIVE_TEST_LOCK.lock().unwrap();
    let mut host = Host::default();
    let mut path = b"root\\".to_vec();
    for _ in 0..80 {
        host.set(&[path.as_slice(), b"*"].concat(), &[(b"next", 0x10)]);
        path.extend_from_slice(b"next\\");
    }
    host.set(&[path.as_slice(), b"*"].concat(), &[(b"last", 0)]);
    let mut owner = RzDirectory::new(&mut host, b"", false, false, false);
    let mut out = names(&[b"seed"]);
    assert!(owner.read_directory_file_entries_into_string_list_recursive(&mut host, b"root\\", &mut out, false,));
    assert_eq!(
        out,
        VecDeque::from([b"seed".to_vec(), [path.as_slice(), b"last"].concat()])
    );
    assert_eq!(owner.path(), path);
    assert_eq!(owner.count(), 1);
    assert_eq!(host.patterns().len(), 242);
    assert_eq!(RECURSIVE_DEPTH.load(Ordering::SeqCst), 0);
    host.assert_closed();
}

#[test]
fn existence_uses_first_then_immediate_finish_without_next_or_attribute_check() {
    let mut host = Host::default();
    host.set(b"literal-pattern", &[(b"file", 0)]);
    assert!(RzDirectory::does_any_entry_exist_that_matches_pattern(
        &mut host,
        b"literal-pattern"
    ));
    assert_eq!(
        host.calls,
        vec![Call::First(b"literal-pattern".to_vec()), Call::Finish(1)]
    );
    for (path, pattern) in [
        (b"".as_slice(), b"\\*".as_slice()),
        (b"X/", b"X/*"),
        (b"\xff", b"\xff\\*"),
    ] {
        host.set(pattern, &[(b".", 0x10)]);
        assert!(RzDirectory::does_directory_exist(&mut host, path));
        assert_eq!(host.patterns().last().unwrap(), pattern);
    }
    assert!(!RzDirectory::does_directory_exist(&mut host, b"file-path"));
    assert!(!RzDirectory::does_any_entry_exist_that_matches_pattern(
        &mut host, b"absent"
    ));
    host.assert_closed();
}

#[test]
fn create_scans_every_separator_and_skips_existing_levels() {
    let mut host = Host::default();
    assert!(!RzDirectory::create_directory(&mut host, b""));
    assert!(host.calls.is_empty());
    host.set(b"C:\\*", &[(b".", 0x10)]);
    assert!(RzDirectory::create_directory(&mut host, b"C:\\A/B"));
    assert_eq!(
        host.calls,
        vec![
            Call::First(b"C:\\*".to_vec()),
            Call::Finish(1),
            Call::First(b"C:\\A/*".to_vec()),
            Call::Create(b"C:\\A/".to_vec()),
            Call::First(b"C:\\A/B\\*".to_vec()),
            Call::Create(b"C:\\A/B\\".to_vec()),
        ]
    );
    host.assert_closed();
}

#[test]
fn create_unc_skips_server_only_and_repeated_separators_remain_significant() {
    let mut host = Host::default();
    assert!(RzDirectory::create_directory(&mut host, b"\\\\srv\\share\\sub"));
    assert_eq!(
        host.patterns(),
        vec![b"\\\\srv\\share\\*".to_vec(), b"\\\\srv\\share\\sub\\*".to_vec()]
    );
    assert!(RzDirectory::create_directory(&mut host, b"A//B/"));
    assert_eq!(
        &host.patterns()[2..],
        &[b"A/*".to_vec(), b"A//*".to_vec(), b"A//B/*".to_vec()]
    );
    let mut host = Host::default();
    // Three initial backslashes do not qualify for the special UNC skip.
    assert!(RzDirectory::create_directory(&mut host, b"\\\\\\"));
    assert_eq!(
        host.patterns(),
        vec![b"\\*".to_vec(), b"\\\\*".to_vec(), b"\\\\\\*".to_vec()]
    );
}

#[test]
fn create_rollback_is_creation_order_even_when_parent_removal_fails() {
    let mut host = Host::default();
    host.fail_create.insert(b"A\\B\\C\\".to_vec());
    assert!(!RzDirectory::create_directory(&mut host, b"A\\B\\C"));
    assert_eq!(
        host.calls,
        vec![
            Call::First(b"A\\*".to_vec()),
            Call::Create(b"A\\".to_vec()),
            Call::First(b"A\\B\\*".to_vec()),
            Call::Create(b"A\\B\\".to_vec()),
            Call::First(b"A\\B\\C\\*".to_vec()),
            Call::Create(b"A\\B\\C\\".to_vec()),
            Call::Remove(b"A\\".to_vec(), false),
            Call::Remove(b"A\\B\\".to_vec(), true),
        ]
    );
    assert_eq!(host.created, BTreeSet::from([b"A\\".to_vec()]));
    // Failure at the first missing level has nothing to roll back.
    let mut host = Host::default();
    host.fail_create.insert(b"A\\".to_vec());
    assert!(!RzDirectory::create_directory(&mut host, b"A\\B"));
    assert_eq!(host.calls.len(), 2);
    host.assert_closed();
}

#[test]
fn create_all_existing_and_server_only_do_not_create_anything() {
    let mut host = Host::default();
    host.set(b"A\\*", &[(b".", 0x10)]);
    host.set(b"A\\B\\*", &[(b".", 0x10)]);
    assert!(RzDirectory::create_directory(&mut host, b"A\\B"));
    assert!(RzDirectory::create_directory(&mut host, b"\\\\server"));
    assert!(host.created.is_empty());
    assert_eq!(host.patterns().len(), 2);
    host.assert_closed();
    // RZ11's literal fallback: slash terminates the server name, but no third backslash exists.
    assert!(RzDirectory::create_directory(&mut host, b"\\\\server/"));
    assert_eq!(host.patterns().len(), 2);
}

#[test]
fn split_published_examples_mixed_separators_roots_and_empty_input() {
    let mut host = Host::default();
    for (path, last, parent) in [
        (b"C:\\A\\B\\".as_slice(), b"B\\".as_slice(), b"C:\\A\\".as_slice()),
        (b"C:\\A\\B", b"B\\", b"C:\\A\\"),
        (b"C:\\", b"C:\\", b""),
        (b"\\", b"\\", b""),
        (b"foo", b"foo\\", b""),
        (b"C:/A\\\xff/", b"\xff/", b"C:/A\\"),
    ] {
        let owner = RzDirectory::new(&mut host, path, false, false, false);
        let mut out_last = b"last-seed".to_vec();
        let mut out_parent = b"parent-seed".to_vec();
        assert!(owner.split_directory_path(&mut out_last, &mut out_parent));
        assert_eq!(out_last, last);
        assert_eq!(out_parent, parent);
    }
    let mut last = b"last-seed".to_vec();
    let mut parent = b"parent-seed".to_vec();
    assert!(!split_directory_path(b"", &mut last, &mut parent));
    assert_eq!(last, b"last-seed");
    assert_eq!(parent, b"parent-seed");
}

#[test]
fn split_254_byte_result_boundaries_and_transactional_failure() {
    let mut host = Host::default();
    for (last_len, success) in [(253, true), (254, false), (255, false)] {
        let path = [b"P\\".as_slice(), &vec![b'x'; last_len]].concat();
        let owner = RzDirectory::new(&mut host, &path, false, false, false);
        let mut last = b"seed-last".to_vec();
        let mut parent = b"seed-parent".to_vec();
        assert_eq!(owner.split_directory_path(&mut last, &mut parent), success);
        if success {
            assert_eq!(last.len(), 254);
            assert_eq!(parent, b"P\\");
        } else {
            assert_eq!(last, b"seed-last");
            assert_eq!(parent, b"seed-parent");
        }
    }
    for (prefix_len, success) in [(253, true), (254, false)] {
        let path = [vec![b'p'; prefix_len], b"\\x".to_vec()].concat();
        let owner = RzDirectory::new(&mut host, &path, false, false, false);
        let mut last = b"seed-last".to_vec();
        let mut parent = b"seed-parent".to_vec();
        assert_eq!(owner.split_directory_path(&mut last, &mut parent), success);
        if success {
            assert_eq!(last, b"x\\");
            assert_eq!(parent.len(), 254);
        } else {
            assert_eq!(last, b"seed-last");
            assert_eq!(parent, b"seed-parent");
        }
    }
}
