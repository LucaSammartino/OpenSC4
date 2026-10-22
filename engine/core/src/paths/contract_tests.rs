//! Discriminating PA4–PA8 vectors, using only primitive providers and literal expected bytes.
use super::helpers::*;
use super::tests::{Cwd, Finder};
use super::*;
use crate::rz_files::{OsError, OsHandle};
use std::collections::VecDeque;

#[test]
fn crt_boundary_receives_signed_bytes_and_fullpath_stays_ascii() {
    fn classified(byte: i32) -> Result<bool, UnsupportedLocaleByte> {
        assert_eq!(byte, -23); // 0xE9 sign-extended, not Unicode U+00E9 or unsigned 233.
        Ok(true)
    }
    let bytes = b"\xE9:\\x";
    assert!(is_full_path_with_classifier(bytes, classified).unwrap());
    assert_eq!(is_full_path(bytes), Err(UnsupportedLocaleByte(-23)));
    let mut drive = vec![0xE9, b'!', b'X'];
    assert_eq!(fix_drive_string(&mut drive), Err(UnsupportedLocaleByte(-23)));
    assert_eq!(drive, [0xE9, b'!', b'X']);
    fix_drive_string_with_classifier(&mut drive, classified).unwrap();
    assert_eq!(drive, [0xE9, b':']);
    let mut path = RzPath::with_classifier(bytes, &mut Cwd::default(), classified).unwrap();
    assert_eq!(path.get_drive().unwrap(), &[0xE9, b':']);
    let mut out = b"unchanged".to_vec();
    assert_eq!(resolve_path(&mut out, bytes, None, &mut Cwd::default()), Err(UnsupportedLocaleByte(-23)));
    assert_eq!(out, b"unchanged");
    assert_eq!(
        fullpath_current(Some(b"\xE9:rel"), 520, &mut Cwd::sc4()).unwrap(),
        b"C:\\Games\\SC4\\\xE9:rel"
    );
}

#[test]
fn counted_setters_compare_past_nul_before_deciding_whether_to_split() {
    let mut path = RzPath::new(b"C:\\original.ext", &mut Cwd::default()).unwrap();
    let mut source = b"a\0x".to_vec();
    path.set_file_name(&source, false);
    source.fill(b'!');
    assert_eq!(path.get_file_name(), b"a\0x");
    path.set_file_name(b"a\0x", true); // equal counted range: no split performed.
    assert_eq!(path.get_file_name(), b"a\0x");
    path.set_file_name(b"a\0y", true); // differs beyond NUL: invokes C-string split.
    assert_eq!(path.get_file_name(), b"a");
    path.set_file_name(b"A", false);
    assert_eq!(path.get_file_name(), b"A");
    path.set_extension(b".e\0x", false);
    path.set_extension(b".e\0x", true);
    assert_eq!(path.get_extension(), b".e\0x");
    path.set_extension(b".e\0y", true);
    assert_eq!(path.get_extension(), b".e");
    path.set_extension(b"e\0z", false);
    assert_eq!(path.get_extension(), b".e\0z");
    let mut out = Vec::new();
    path.get_file_name_and_extension(&mut out).unwrap();
    assert_eq!(out, b"A.e\0z");
    path.set_file_name(b"", false);
    assert_eq!(path.file_name(), b"A"); // lazy split of the last counted join sees its C prefix.
    let two_byte = [0xC3, 0xA9].repeat(128);
    assert_eq!(splitpath(&two_byte).name.len(), 255); // byte count, can split an encoding unit.
    assert!(fullpath_with_base(Some(&[0xC3, 0xA9].repeat(260)), 1000, b"").is_some());
    assert!(fullpath_with_base(Some(&[0xC3, 0xA9].repeat(261)), 1000, b"").is_none());
}

#[test]
fn split_directory_literal_boundaries_signed_capacity_and_partial_writes() {
    for (input, expected) in [
        (b"\\a\\".as_slice(), vec![b"a\\".to_vec()]),
        (b"C:\\a\\", vec![b"a\\".to_vec(), b"C:\\".to_vec()]),
        (b"x", vec![]),
        (b"\\", vec![]),
    ] {
        assert_eq!(split_directory(input, 8), Some(expected));
    }
    let mut parts = [[0xCC; 260]; 2];
    for capacity in [0, -1, i32::MIN] {
        let mut count = capacity;
        assert!(split_directory_into(b"abc", &mut parts, &mut count));
        assert_eq!(count, 0);
        assert_eq!(parts, [[0xCC; 260]; 2]);
    }
    let mut count = 0;
    assert!(!split_directory_into(b"", &mut parts, &mut count));
    assert!(split_directory_into(b"x", &mut [], &mut count));
    for (n, expected) in [(254, true), (255, false)] {
        let mut input = vec![b'x'; n];
        input.push(b'\\');
        let got = split_directory(&input, 2);
        assert_eq!(got.is_some(), expected);
        if expected {
            assert_eq!(got.unwrap(), vec![input]);
        }
    }
    for (n, expected) in [(253, true), (254, false)] {
        let mut input = b"a\\".to_vec();
        input.extend(vec![b'x'; n]);
        input.push(b'\\');
        assert_eq!(split_directory(&input, 2).is_some(), expected);
    }
    // All short components, but the final aggregate prefix has a computed span of 255.
    let long_prefix = b"a\\".repeat(128);
    assert!(split_directory(&long_prefix, 1).is_none());
    let mut input = long_prefix;
    input.extend_from_slice(b"ok\\");
    let mut count = 2;
    assert!(!split_directory_into(&input, &mut parts, &mut count));
    assert_eq!(count, 0);
    assert_eq!(&parts[0][..4], b"ok\\\0");
    assert!(parts[0][4..].iter().all(|&b| b == 0xCC));
    assert_eq!(parts[1], [0xCC; 260]);
    let (mut last, mut parent) = (b"last".to_vec(), b"parent".to_vec());
    assert!(!split_directory_path(&input, &mut last, &mut parent));
    assert_eq!((last.as_slice(), parent.as_slice()), (b"last".as_slice(), b"parent".as_slice()));
}

#[test]
fn entry_lifecycle_copies_the_name_but_never_keeps_the_directory_alive() {
    let empty = RzDirectoryEntry::default();
    assert!(empty.owner.is_null());
    assert!(empty.name.is_empty());
    assert_eq!(empty.kind, 0);
    let mut host = Finder::default();
    host.put(b"p\\*", &[(b"name\0tail", false)]);
    let mut owner = Box::new(RzDirectory::new(&mut host, b"p", true, false, false));
    let payload = owner.get_nth_entry(0).unwrap();
    let address = payload as *const _;
    // A caller explicitly makes a standalone copy; get_nth_entry itself remains a list borrow.
    let original = RzDirectoryEntry { owner: &*owner, name: payload.name.clone(), kind: payload.entry_type };
    let copied = original.clone();
    assert_ne!(original.name.as_ptr(), copied.name.as_ptr());
    assert_eq!(copied.owner, &*owner as *const _);
    drop(original); // owner is still present and usable.
    owner.change_directory(&mut host, b"missing", false);
    assert_eq!(owner.get_nth_entry(0).unwrap() as *const _, address);
    assert!(!owner.read_directory_entries(&mut host)); // old list erased even on failure.
    assert!(owner.get_nth_entry(0).is_none());
    assert_eq!(copied.name, b"name\0tail");
    drop(owner); // copied opaque pointer now dangles; do not dereference it.
    assert_eq!(copied.name, b"name\0tail");
    assert_eq!(copied.kind, 8);
    drop(copied);
}

#[test]
fn string_sort_uses_unsigned_counted_bytes_and_preserves_equal_allocations() {
    let a1 = b"a".to_vec();
    let a2 = b"a".to_vec();
    let (p1, p2) = (a1.as_ptr(), a2.as_ptr());
    let mut list: VecDeque<_> = [vec![0xFF], b"a\0y".to_vec(), a1, b"b".to_vec(),
        b"a\0x".to_vec(), b"A".to_vec(), vec![0x80], a2].into();
    RzDirectory::alphabetize_entry_list(&mut list);
    let expected: VecDeque<_> = [b"A".to_vec(), b"a".to_vec(), b"a".to_vec(), b"a\0x".to_vec(),
        b"a\0y".to_vec(), b"b".to_vec(), vec![0x80], vec![0xFF]].into();
    assert_eq!(list, expected);
    assert_eq!((list[1].as_ptr(), list[2].as_ptr()), (p1, p2));
}

#[test]
fn repeated_failed_next_retries_the_same_handle_and_finish_alone_closes() {
    struct Host { attempts: usize, closes: usize }
    impl DirectoryProvider for Host {
        fn find_first(&mut self, pattern: &[u8]) -> Result<(OsHandle, FindData), OsError> {
            assert_eq!(pattern, b"pattern\0counted");
            Ok((42, FindData { name: b"old\0name".to_vec(), directory: true }))
        }
        fn find_next(&mut self, handle: OsHandle) -> Option<FindData> {
            assert_eq!(handle, 42);
            assert_eq!(self.closes, 0);
            self.attempts += 1;
            (self.attempts == 3).then(|| FindData { name: b"new".to_vec(), directory: false })
        }
        fn find_finish(&mut self, handle: OsHandle) {
            assert_eq!(handle, 42);
            self.closes += 1;
        }
    }
    let mut host = Host { attempts: 0, closes: 0 };
    let mut search = FindSearch::first(&mut host, b"pattern\0counted").unwrap();
    for _ in 0..2 {
        assert!(!search.advance());
        assert_eq!(search.current().name, b"old\0name");
        assert!(search.current().directory);
    }
    assert!(search.advance());
    assert_eq!(search.current().name, b"new");
    assert!(!search.current().directory);
    assert!(!search.advance());
    assert_eq!(search.current().name, b"new");
    assert!(!search.current().directory);
    search.finish();
    assert_eq!((host.attempts, host.closes), (4, 1));
}
