//! Independent examples and boundary cases from paths.md §§1–4, 8; no host filesystem involved.
use std::collections::{BTreeMap, VecDeque};

#[test]
fn fully_qualified_resolution_copies_the_entire_counted_range() {
    let mut source = b"C:\\name\0/.tail".to_vec();
    let mut out = Vec::new();
    assert!(resolve_path(&mut out, &source, None, &mut Cwd::default()).unwrap());
    source.fill(b'!');
    assert_eq!(out, b"C:\\name\0/.tail");
    assert_eq!(splitpath(&out).name, b"name");
}

use super::helpers::*;
use super::*;
use crate::rz_files::{OsError, OsHandle};

#[derive(Default)]
pub(super) struct Cwd {
    drive: Option<u8>,
    directories: BTreeMap<u8, Vec<u8>>,
    calls: Vec<u8>,
}

impl Cwd {
    pub(super) fn sc4() -> Self {
        Self {
            drive: Some(b'C'),
            directories: [(b'C', br"C:\Games\SC4".to_vec()), (b'D', br"D:\Data".to_vec())].into(),
            calls: Vec::new(),
        }
    }
}

impl CurrentDirectory for Cwd {
    fn current_drive(&mut self) -> Option<u8> {
        self.drive
    }
    fn drive_directory(&mut self, drive: u8) -> Option<Vec<u8>> {
        self.calls.push(drive);
        self.directories.get(&drive).cloned()
    }
}

#[test]
fn full_path_classification_is_literal_and_byte_based() {
    for path in [br"C:\a".as_slice(), br"\\s\x", br"z:\", br"C:\a\\b"] {
        assert!(is_full_path(path).unwrap(), "{path:?}");
    }
    for path in [
        br"C:/a".as_slice(),
        br"\x",
        b"C:",
        br"1:\x",
        br"C:\a/.b",
        br"C:\a\.hidden\f",
        b"",
        b"C",
        b"\\",
    ] {
        assert!(!is_full_path(path).unwrap(), "{path:?}");
    }
    assert!(is_full_path(b"C:\\x\0/.bad").unwrap());
    assert_eq!(is_full_path(b"\xE9:\\x"), Err(UnsupportedLocaleByte(-23)));
    assert!(is_path_separator(b'/'));
    assert!(!is_path_separator(0xA5));
    assert!(!is_letter(0xE9));
}

#[test]
fn explicit_base_normalisation_vectors() {
    let cases: &[(&[u8], &[u8], &[u8])] = &[
        (br"adv_const.lua", br"C:\Games\SC4", br"C:\Games\SC4\adv_const.lua"),
        (br"..\x.dat", br"C:\Games\SC4", br"C:\Games\x.dat"),
        (br"C:\a\..\b.txt", b"", br"C:\b.txt"),
        (br"C:\a\.\b", b"", br"C:\a\b"),
        (br"C:\a\", b"", br"C:\a\"),
        (br"C:/", b"", br"C:\\"), // §1.2/§6.9; conflicting §8 vector remains PA3.
        (br"a\..\..\x", br"C:\Games\SC4", br"C:\Games\x"),
        (br"C:\a/./b", b"", br"C:\a/b"),
        (br"C:\a\\b", b"", br"C:\a\\b"),
        (br"\\srv\..\x", b"", br"\\x"),
        (br"C:\a\.hidden\f", b"", br"C:\a\.hidden\f"),
        (br"C:\...\x", b"", br"C:\...\x"),
        (b"", b"", b""),
        (b"C:", b"", br"C:\"),
        (b"x", b"a/", b"a/x"),
        (br"C:\other", br"C:\base", br"C:\base\C:\other"), // explicit base always joins.
    ];
    for &(rel, base, expected) in cases {
        assert_eq!(
            fullpath_with_base(Some(rel), 520, base).as_deref(),
            Some(expected),
            "{rel:?}, {base:?}"
        );
    }
    for path in [br"C:\..".as_slice(), br"a\..", br"\..", br"C:\a\..\..\x"] {
        assert!(fullpath_with_base(Some(path), 520, b"").is_none(), "{path:?}");
    }
}

#[test]
fn current_directory_drive_root_unc_and_fallback_vectors() {
    let mut cwd = Cwd::sc4();
    for (input, expected) in [
        (br"d:rel.txt".as_slice(), br"D:\Data\rel.txt".as_slice()),
        (br"\x\y.z", br"C:\x\y.z"),
        (br"C:/a/b.txt", br"C:\a/b.txt"),
        (br"\\srv\..\x", br"\\x"),
        (br"//srv/share", br"\\srv/share"),
        (br"C:/", br"C:\\"),
        (br"d:\x", br"D:\x"),
        (b"C:", br"C:\Games\SC4\"),
    ] {
        assert_eq!(fullpath_current(Some(input), 520, &mut cwd).as_deref(), Some(expected));
    }
    assert_eq!(cwd.calls, b"DC");
    cwd.drive = None;
    assert_eq!(
        fullpath_current(Some(b"x"), 520, &mut cwd).as_deref(),
        Some(br"C:\Games\SC4\x".as_slice())
    );
    cwd.directories.clear();
    assert!(fullpath_current(Some(b"x"), 520, &mut cwd).is_none());
    assert_eq!(
        fullpath_current(Some(br"\x"), 520, &mut cwd).as_deref(),
        Some(br"C:\x".as_slice())
    );
}

#[test]
fn sizes_include_nul_and_fail_without_partial_output() {
    assert!(fullpath_with_base(None, 520, b"").is_none());
    assert!(fullpath_with_base(Some(b""), 0, b"").is_none());
    assert_eq!(fullpath_with_base(Some(b""), 1, b""), Some(Vec::new()));
    assert!(fullpath_with_base(Some(b"abc"), 3, b"").is_none());
    assert_eq!(fullpath_with_base(Some(b"abc"), 4, b""), Some(b"abc".to_vec()));
    assert!(fullpath_with_base(Some(&vec![b'x'; 521]), 1000, b"").is_none());
    assert!(fullpath_with_base(Some(b""), 1000, &vec![b'x'; 521]).is_none());
    assert!(fullpath_with_base(Some(b""), 1000, &vec![b'x'; 520]).is_none()); // appended slash overflows join.
    assert!(fullpath_with_base(Some(&vec![b'x'; 520]), 520, b"").is_none());
    assert_eq!(fullpath_with_base(Some(&vec![b'x'; 519]), 520, b"").unwrap().len(), 519);
    let mut output = [0xCC; 4];
    assert!(!copy_c_result(&mut output, None));
    assert!(!copy_c_result(&mut output, Some(b"abcd")));
    assert_eq!(output, [0xCC; 4]);
    assert!(copy_c_result(&mut output, Some(b"ab")));
    assert_eq!(output, [b'a', b'b', 0, 0xCC]);
}

#[test]
fn resolve_preserves_failure_and_bypasses_limits_for_already_full_text() {
    let mut cwd = Cwd::default();
    let mut out = b"old".to_vec();
    assert!(!resolve_path(&mut out, b"relative", None, &mut cwd).unwrap());
    assert_eq!(out, b"old");
    let mut long = br"c:\".to_vec();
    long.extend(vec![b'x'; 700]);
    assert!(resolve_path(&mut out, &long, None, &mut cwd).unwrap());
    assert_eq!(out, long);
    assert_eq!(cwd.calls, b"C"); // only the failed relative resolution asked for a drive directory.
    assert!(resolve_path(&mut out, b"relative", Some(b""), &mut cwd).unwrap());
    assert_eq!(out, b"relative");
    assert!(resolve_path(&mut out, br"\\srv\share\f.txt", Some(b"ignored"), &mut cwd).unwrap());
    assert_eq!(out, br"\\srv\share\f.txt");
}

#[test]
fn splitpath_drive_last_dot_separator_and_truncation() {
    let cases: &[(&[u8], [&[u8]; 4])] = &[
        (br"C:\x\advisors\Taxi.lua", [b"C:", br"\x\advisors\", b"Taxi", b".lua"]),
        (b".lua", [b"", b"", b"", b".lua"]),
        (b"a.b.c", [b"", b"", b"a.b", b".c"]),
        (b"C:foo.txt", [b"C:", b"", b"foo", b".txt"]),
        (br"a\b.c\d", [b"", br"a\b.c\", b"d", b""]),
        (b"1:x.", [b"1:", b"", b"x", b"."]),
        (b"", [b"", b"", b"", b""]),
        (b"a/b.C", [b"", b"a/", b"b", b".C"]),
    ];
    for &(input, expected) in cases {
        let p = splitpath(input);
        assert_eq!([p.drive.as_slice(), &p.directory, &p.name, &p.extension], expected);
    }
    let p = splitpath(&vec![b'a'; 300]);
    assert_eq!(p.name, vec![b'a'; 255]);
    let mut path = vec![b'd'; 300];
    path.push(b'/');
    path.extend(vec![b'n'; 300]);
    path.push(b'.');
    path.extend(vec![b'e'; 300]);
    let p = splitpath(&path);
    assert_eq!(p.directory.len(), 255);
    assert_eq!(p.name.len(), 255);
    assert_eq!(p.extension.len(), 255);
    assert_eq!(splitpath(b"a\xC3\xA9.txt").name, b"a\xC3\xA9");
    assert_eq!(splitpath(b"a\0b.txt").name, b"a");
}

#[test]
fn reverse_split_and_parent_failure_are_explicit() {
    for input in [br"C:\a\b\".as_slice(), br"C:\a\b"] {
        assert_eq!(
            split_directory(input, 2).unwrap(),
            [br"b\".to_vec(), br"C:\a\".to_vec()]
        );
    }
    assert_eq!(split_directory(br"abc\", 2).unwrap(), [br"abc\".to_vec()]);
    assert_eq!(split_directory(b"abc", 2).unwrap(), [br"ab\".to_vec()]);
    assert!(split_directory(b"x", 2).unwrap().is_empty());
    assert!(split_directory(b"\\", 2).unwrap().is_empty());
    assert!(split_directory(b"", 2).is_none());
    assert!(split_directory(b"abc", 0).unwrap().is_empty());
    assert!(split_directory(&vec![b'x'; 256], 2).is_none());
    let (mut last, mut parent) = (b"old-last".to_vec(), b"old-parent".to_vec());
    assert!(!split_directory_path(b"", &mut last, &mut parent));
    assert_eq!(last, b"old-last");
    assert_eq!(parent, b"old-parent");
    assert!(split_directory_path(b"abc", &mut last, &mut parent));
    assert_eq!(last, br"ab\");
    assert!(parent.is_empty());
}

#[test]
fn c_output_allocations_optional_parts_and_count_contract() {
    let mut out = [0xCC; 8];
    assert!(!fullpath_into(&mut out, Some(b"abcdefg"), 7, b""));
    assert_eq!(out, [0xCC; 8]);
    assert!(fullpath_into(&mut out, Some(b"abcdefg"), 8, b""));
    assert_eq!(&out, b"abcdefg\0");
    let mut a = fullpath_alloc(Some(b"abc"), 4, b"").unwrap();
    let b = fullpath_alloc(Some(b"abc"), 4, b"").unwrap();
    a[0] = b'X';
    assert_eq!(b, b"abc\0");
    let mut drive = [0xCC; 3];
    let mut name = [0xCC; 256];
    splitpath_into(b"x:dir/a.dat", Some(&mut drive), None, Some(&mut name), None);
    assert_eq!(&drive, b"x:\0");
    assert_eq!(&name[..3], &[b'a', 0, 0xCC]);
    let mut parts = [[0xCC; 260]; 3];
    let mut count = 2;
    assert!(split_directory_into(br"C:\a\b", &mut parts, &mut count));
    assert_eq!(count, 2);
    assert_eq!(&parts[0][..3], b"b\\\0");
    assert_eq!(&parts[1][..6], b"C:\\a\\\0");
    assert_eq!(parts[2], [0xCC; 260]);
    let old = parts;
    assert!(!split_directory_into(b"", &mut parts, &mut count));
    assert_eq!(count, 0);
    assert_eq!(parts, old);
}

#[test]
fn published_part_fixes_preserve_case_and_separator_spelling() {
    for (input, expected) in [
        (b"".as_slice(), b"".as_slice()),
        (b"x", b"x:"),
        (b"1:abc", b"C:"),
        (b"\xE9", b"C:"),
        (b"zXYZ", b"z:"),
    ] {
        let mut text = input.to_vec();
        fix_drive_string(&mut text).unwrap();
        assert_eq!(text, expected);
    }
    for (input, expected) in [
        (b"".as_slice(), b"".as_slice()),
        (b"foo", br"\foo\"),
        (b"/foo/", b"/foo/"),
        (b"foo/", br"\foo/"),
    ] {
        let mut text = input.to_vec();
        fix_directory_string(&mut text);
        assert_eq!(text, expected);
    }
    for (input, expected) in [
        (b"".as_slice(), b"".as_slice()),
        (b"dat", b".dat"),
        (b".UP", b".UP"),
        (b"..", b".."),
    ] {
        let mut text = input.to_vec();
        fix_extension_string(&mut text);
        assert_eq!(text, expected);
    }
}

#[test]
fn path_owner_join_setters_and_lazy_buffer_reload() {
    let mut cwd = Cwd::sc4();
    let mut path = RzPath::new(b"adv_const.lua", &mut cwd).unwrap();
    let mut out = Vec::new();
    path.get_path(&mut out).unwrap();
    assert_eq!(out, br"C:\Games\SC4\adv_const.lua");
    path.get_drive_and_directory(&mut out).unwrap();
    assert_eq!(out, br"C:\Games\SC4\");
    path.get_file_name_and_extension(&mut out).unwrap();
    assert_eq!(out, b"adv_const.lua");
    assert!(path.set_extension(b"dat", false));
    path.get_path(&mut out).unwrap();
    assert_eq!(out, br"C:\Games\SC4\adv_const.dat");
    assert!(path.set_file_name(br"x\y.txt", true));
    assert_eq!(path.get_file_name(), b"y");
    assert!(path.set_file_name(b"", false));
    assert_eq!(path.get_file_name(), b"");
    assert_eq!(path.file_name(), b"adv_const"); // last joined result, not the pending y.
    path.set_extension(br"a/b.tar.gz", true);
    assert_eq!(path.get_extension(), b".gz");
    path.set_extension(b"", false);
    assert_eq!(path.get_extension(), b"");
    path.set_file_name(b"UPPER", false);
    path.set_file_name(b"upper", false);
    assert_eq!(path.get_file_name(), b"upper");
    path.set_buffer(0xF0).unwrap();
    path.set_file_name(b"", false);
    assert_eq!(path.file_name(), b"");
}

#[test]
fn path_resolution_failure_resplits_last_join_and_owns_all_bytes() {
    let mut cwd = Cwd::sc4();
    let mut input = b"first.txt".to_vec();
    let mut path = RzPath::new(&input, &mut cwd).unwrap();
    input.fill(b'X');
    assert_eq!(path.get_file_name(), b"first");
    let mut out = Vec::new();
    path.get_file_name_and_extension(&mut out).unwrap();
    cwd.directories.clear();
    assert!(path.set_path(b"unresolved", &mut cwd).unwrap()); // true despite resolution failure.
    assert_eq!(path.get_drive().unwrap(), b"");
    assert_eq!(path.get_directory(), b"");
    assert_eq!(path.get_file_name(), b"first");
    path.set_file_name(b"second", false);
    assert_eq!(out, b"first.txt"); // output is an independent owned string.
    let empty = RzPath::new(b"unresolved", &mut cwd).unwrap();
    assert_eq!(empty.get_file_name(), b"");
}

#[derive(Default)]
pub(super) struct Finder {
    matches: BTreeMap<Vec<u8>, Vec<FindData>>,
    active: BTreeMap<OsHandle, VecDeque<FindData>>,
    patterns: Vec<Vec<u8>>,
    next_handle: OsHandle,
    closed: Vec<OsHandle>,
}

impl Finder {
    pub(super) fn put(&mut self, pattern: &[u8], names: &[(&[u8], bool)]) {
        self.matches.insert(
            pattern.to_vec(),
            names
                .iter()
                .map(|(name, directory)| FindData {
                    name: name.to_vec(),
                    directory: *directory,
                })
                .collect(),
        );
    }
}

impl DirectoryProvider for Finder {
    fn find_first(&mut self, pattern: &[u8]) -> Result<(OsHandle, FindData), OsError> {
        self.patterns.push(pattern.to_vec());
        let mut names: VecDeque<_> = self.matches.get(pattern).cloned().unwrap_or_default().into();
        let first = names.pop_front().ok_or(2u32)?;
        self.next_handle += 1;
        self.active.insert(self.next_handle, names);
        Ok((self.next_handle, first))
    }
    fn find_next(&mut self, handle: OsHandle) -> Option<FindData> {
        self.active.get_mut(&handle)?.pop_front()
    }
    fn find_finish(&mut self, handle: OsHandle) {
        assert!(self.active.remove(&handle).is_some(), "double-close or unowned handle");
        self.closed.push(handle);
    }
}

// Primitive-only bridge for exercising the one authoritative directory owner.
impl crate::rz_files::directory::RzDirectoryHost for Finder {
    type FindHandle = OsHandle;

    fn find_first(&mut self, pattern: &[u8]) -> Option<(OsHandle, crate::rz_files::directory::DirectoryMatch)> {
        DirectoryProvider::find_first(self, pattern)
            .ok()
            .map(|(h, data)| (h, directory_match(data)))
    }

    fn find_next(&mut self, handle: &mut OsHandle) -> Option<crate::rz_files::directory::DirectoryMatch> {
        DirectoryProvider::find_next(self, *handle).map(directory_match)
    }

    fn find_finish(&mut self, handle: OsHandle) {
        DirectoryProvider::find_finish(self, handle);
    }

    fn create_directory(&mut self, _: &[u8]) -> bool {
        panic!("unexpected create primitive")
    }

    fn remove_directory(&mut self, _: &[u8]) -> bool {
        panic!("unexpected remove primitive")
    }
}

fn directory_match(data: FindData) -> crate::rz_files::directory::DirectoryMatch {
    crate::rz_files::directory::DirectoryMatch {
        name: data.name,
        attributes: if data.directory { 0x10 } else { 0 },
    }
}

fn entries(directory: &RzDirectory) -> Vec<(Vec<u8>, u32)> {
    (0..directory.count())
        .map(|i| {
            let e = directory.get_nth_entry(i).unwrap();
            (e.name.to_vec(), e.entry_type)
        })
        .collect()
}

#[test]
fn entry_state_preserves_system_order_and_owner_identity() {
    let mut provider = Finder::default();
    provider.put(
        br"C:\P\*",
        &[
            (b".", true),
            (b"..", true),
            (b"b.dat", false),
            (b"a.dat", false),
            (b"sub", true),
        ],
    );
    let directory = RzDirectory::new(&mut provider, br"C:\P", true, false, false);
    assert_eq!(directory.path(), br"C:\P\");
    assert_eq!(
        entries(&directory),
        [(b"b.dat".to_vec(), 8), (b"a.dat".to_vec(), 8), (b"sub".to_vec(), 4)]
    );
    assert!(directory.get_nth_entry(3).is_none());
    let first = directory.get_nth_entry(0).unwrap() as *const _;
    let mut names = [b"b.dat".to_vec(), b"a.dat".to_vec()].into();
    RzDirectory::alphabetize_entry_list(&mut names);
    assert_eq!(names.front().unwrap(), b"a.dat");
    assert_eq!(directory.get_nth_entry(0).unwrap().name, b"b.dat");
    assert_eq!(directory.get_nth_entry(0).unwrap() as *const _, first);
    assert_eq!(provider.closed.len(), 1);
    assert!(provider.active.is_empty());
    let all = RzDirectory::new(&mut provider, br"C:\P\", true, true, true);
    assert_eq!(all.get_nth_entry(0).unwrap().name, b".");
    assert_eq!(all.get_nth_entry(1).unwrap().name, b"..");
}

#[test]
fn dot_synthesis_is_filter_specific_and_never_turns_no_match_into_success() {
    let mut provider = Finder::default();
    provider.put(br"C:\*", &[(b"x", false)]);
    provider.put(br"C:\*.dat", &[(b"x.dat", false)]);
    let mut directory = RzDirectory::new(&mut provider, br"C:\", true, true, true);
    assert_eq!(
        entries(&directory),
        [(b".".to_vec(), 4), (b"..".to_vec(), 4), (b"x".to_vec(), 8)]
    );
    directory.set_new_directory_entry_filter(&mut provider, b"*.dat", true);
    assert_eq!(entries(&directory), [(b"x.dat".to_vec(), 8)]);
    directory.set_new_directory_entry_filter(&mut provider, b"missing", true);
    assert!(entries(&directory).is_empty());
    assert!(!directory.read_directory_entries(&mut provider));
    directory.set_new_directory_entry_filter(&mut provider, b"", false);
    assert_eq!(directory.filter(), b"*");
    assert!(directory.read_directory_entries(&mut provider));
    assert!(directory.change_directory(&mut provider, b"", false));
    assert_eq!(directory.path(), br"\");
    assert_eq!(directory.count(), 3); // read=false preserves list even when path changes.
}

#[test]
fn name_lists_append_and_leave_entry_list_unchanged() {
    let mut provider = Finder::default();
    provider.put(br"p/*", &[(b"a", false), (b"sub", true)]);
    let directory = RzDirectory::new(&mut provider, b"p/", true, false, false);
    let before = entries(&directory);
    let mut list: VecDeque<_> = [b"existing".to_vec()].into();
    assert!(directory.read_directory_file_entries_into_string_list(&mut provider, &mut list));
    assert_eq!(list, VecDeque::from([b"existing".to_vec(), b"a".to_vec()]));
    assert!(directory.read_directory_directory_entries_into_string_list(&mut provider, &mut list, 3));
    assert_eq!(
        list,
        VecDeque::from([
            b".".to_vec(),
            b"..".to_vec(),
            b"existing".to_vec(),
            b"a".to_vec(),
            b"sub".to_vec()
        ])
    );
    assert_eq!(entries(&directory), before);
    let missing = RzDirectory::new(&mut provider, b"missing", false, true, true);
    let old = list.clone();
    assert!(!missing.read_directory_specific_entries_into_string_list(&mut provider, &mut list, 15));
    assert_eq!(list, old);
}

#[test]
fn search_handle_closes_once_at_exhaustion_early_exit_and_failed_start() {
    let mut provider = Finder::default();
    provider.put(b"*", &[(b"one", false), (b"two", true)]);
    {
        let mut search = FindSearch::first(&mut provider, b"*").unwrap();
        assert_eq!(search.pattern(), b"*");
        assert_eq!(search.current().name, b"one");
        assert!(search.advance());
        assert!(search.current().directory);
        assert!(!search.advance());
    }
    assert_eq!(provider.closed, [1]);
    drop(FindSearch::first(&mut provider, b"*"));
    assert_eq!(provider.closed, [1, 2]);
    assert!(FindSearch::first(&mut provider, b"missing").is_none());
    assert_eq!(provider.closed, [1, 2]);
}
