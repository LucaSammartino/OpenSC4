//! cRZDirectory owner, from docs/specs/framework-rz-files.md §6.
//!
//! Paths and names remain framework-encoded **bytes**, including invalid UTF-8. The host converts
//! them at the OS boundary (§1); it must use Windows wildcard/path semantics even on another OS.
//! Enumeration, traversal, dot handling and ordering belong here, not in the host. This module does
//! not implement the separate cRZDirectoryEntry or path owners: entries below are passive values.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};

pub const DIRECTORY: u32 = 4;
pub const FILE: u32 = 8;
pub const SELF: u32 = 1;
pub const PARENT: u32 = 2;

/// One primitive OS find result, in OS enumeration order. Only attribute bit 0x10 is interpreted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryMatch {
    pub name: Vec<u8>,
    pub attributes: u32,
}

/// Passive owned entry data (§6.1), with type 4 (directory) or 8 (file).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: Vec<u8>,
    pub entry_type: u32,
}

/// Primitive OS services only. The host must neither sort nor synthesize/filter owner entries.
pub trait RzDirectoryHost {
    type FindHandle;

    /// Perform the OS wildcard find on exactly `pattern`; None means first-find failure.
    /// Names are converted to framework encoding without requiring valid Rust UTF-8.
    fn find_first(&mut self, pattern: &[u8]) -> Option<(Self::FindHandle, DirectoryMatch)>;
    /// None terminates enumeration, including a later OS find failure (§6.4).
    fn find_next(&mut self, handle: &mut Self::FindHandle) -> Option<DirectoryMatch>;
    fn find_finish(&mut self, handle: Self::FindHandle);
    /// One OS create, without security attributes; not a recursive create.
    fn create_directory(&mut self, path: &[u8]) -> bool;
    /// One OS remove; failure is ignored by owner rollback (§6.7).
    fn remove_directory(&mut self, path: &[u8]) -> bool;
}

// §6.6 has a process-wide, non-thread-safe counter, not a per-owner or per-thread depth. Atomics
// avoid Rust data races without serializing traversal or promising concurrent traversal isolation.
static RECURSIVE_DEPTH: AtomicU32 = AtomicU32::new(0);

struct RecursiveDepth;

impl RecursiveDepth {
    fn enter() -> Self {
        RECURSIVE_DEPTH.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for RecursiveDepth {
    fn drop(&mut self) {
        RECURSIVE_DEPTH.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct Dots {
    parent: bool,
    this: bool,
}

impl Dots {
    fn keep(&mut self, name: &[u8], keep_parent: bool, keep_self: bool) -> bool {
        match name {
            b".." if !self.parent => {
                self.parent = true;
                keep_parent
            }
            b"." if !self.this => {
                self.this = true;
                keep_self
            }
            _ => true,
        }
    }
}

/// Owned ordered entries, count, normalized path, filter and dot policy (§6.1–§6.3).
pub struct RzDirectory {
    entries: VecDeque<DirectoryEntry>,
    count: u32,
    path: Vec<u8>,
    filter: Vec<u8>,
    keep_parent: bool,
    keep_self: bool,
}

impl RzDirectory {
    pub fn new(
        host: &mut impl RzDirectoryHost,
        path: &[u8],
        read_now: bool,
        keep_parent: bool,
        keep_self: bool,
    ) -> Self {
        let mut result = Self {
            entries: VecDeque::new(),
            count: 0,
            path: path.to_vec(),
            filter: b"*".to_vec(),
            keep_parent,
            keep_self,
        };
        ensure_trailing_separator(&mut result.path);
        if read_now {
            result.read_directory_entries(host);
        }
        result
    }

    pub fn path(&self) -> &[u8] {
        &self.path
    }

    pub fn filter(&self) -> &[u8] {
        &self.filter
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn entries(&self) -> &VecDeque<DirectoryEntry> {
        &self.entries
    }

    // §6.4 explicitly walks from the front. Keep that traversal on the portable deque too.
    #[allow(clippy::iter_nth)]
    pub fn get_nth_entry(&self, n: u32) -> Option<&DirectoryEntry> {
        if n >= self.count {
            None
        } else {
            self.entries.iter().nth(n as usize)
        }
    }

    /// Always succeeds, including when the requested read fails (§6.3).
    pub fn change_directory(&mut self, host: &mut impl RzDirectoryHost, path: &[u8], read_now: bool) -> bool {
        self.path = path.to_vec();
        ensure_trailing_separator(&mut self.path);
        if read_now {
            self.read_directory_entries(host);
        }
        true
    }

    pub fn set_new_directory_entry_filter(&mut self, host: &mut impl RzDirectoryHost, filter: &[u8], read_now: bool) {
        self.filter = if filter.is_empty() { b"*" } else { filter }.to_vec();
        if read_now {
            self.read_directory_entries(host);
        }
    }

    fn pattern(&self) -> Vec<u8> {
        [self.path.as_slice(), self.filter.as_slice()].concat()
    }

    pub fn read_directory_entries(&mut self, host: &mut impl RzDirectoryHost) -> bool {
        self.entries.clear();
        self.count = 0;
        let mut dots = Dots::default();
        if !enumerate(host, &self.pattern(), |found| {
            let directory = found.attributes & 0x10 != 0;
            if !directory || dots.keep(&found.name, self.keep_parent, self.keep_self) {
                self.entries.push_back(DirectoryEntry {
                    name: found.name,
                    entry_type: if directory { DIRECTORY } else { FILE },
                });
                self.count += 1;
            }
        }) {
            return false;
        }
        if self.filter == b"*" {
            for (name, keep, seen) in [
                (b"..".as_slice(), self.keep_parent, dots.parent),
                (b".".as_slice(), self.keep_self, dots.this),
            ] {
                if keep && !seen && !self.entries.iter().any(|entry| entry.name == name) {
                    self.entries.push_front(DirectoryEntry {
                        name: name.to_vec(),
                        entry_type: DIRECTORY,
                    });
                    self.count += 1;
                }
            }
        }
        true
    }

    /// Appends OS matches, then may prepend synthesized dots to the *whole* caller list (§6.5).
    pub fn read_directory_specific_entries_into_string_list(
        &self,
        host: &mut impl RzDirectoryHost,
        list: &mut VecDeque<Vec<u8>>,
        flags: u32,
    ) -> bool {
        let mut dots = Dots::default();
        if !enumerate(host, &self.pattern(), |found| {
            if found.attributes & 0x10 == 0 {
                if flags & FILE != 0 {
                    list.push_back(found.name);
                }
            } else if flags & DIRECTORY != 0 && dots.keep(&found.name, flags & PARENT != 0, flags & SELF != 0) {
                // Literal skip-without-4 ordering in §6.5: skipped directories do not mark dots
                // seen. RZ9 asks whether the native seen bookkeeping precedes that skip.
                list.push_back(found.name);
            }
        }) {
            return false;
        }
        if self.filter == b"*" {
            if flags & PARENT != 0 && !dots.parent {
                list.push_front(b"..".to_vec());
            }
            if flags & SELF != 0 && !dots.this {
                list.push_front(b".".to_vec());
            }
        }
        true
    }

    pub fn read_directory_file_entries_into_string_list(
        &self,
        host: &mut impl RzDirectoryHost,
        list: &mut VecDeque<Vec<u8>>,
    ) -> bool {
        self.read_directory_specific_entries_into_string_list(host, list, FILE)
    }

    pub fn read_directory_directory_entries_into_string_list(
        &self,
        host: &mut impl RzDirectoryHost,
        list: &mut VecDeque<Vec<u8>>,
        dot_flags: u32,
    ) -> bool {
        self.read_directory_specific_entries_into_string_list(host, list, (dot_flags & 3) | DIRECTORY)
    }

    /// Bytewise, case-sensitive stable ordering of a caller's list, never the owner's entries.
    pub fn alphabetize_entry_list(list: &mut VecDeque<Vec<u8>>) {
        *list = merge_sort(std::mem::take(list));
    }

    /// §6.6: preorder in OS order, no cycle repair or per-owner recursion depth.
    pub fn read_directory_file_entries_into_string_list_recursive(
        &mut self,
        host: &mut impl RzDirectoryHost,
        path: &[u8],
        out_list: &mut VecDeque<Vec<u8>>,
        include_top_level_files: bool,
    ) -> bool {
        let _depth = RecursiveDepth::enter();
        self.change_directory(host, path, true);
        if include_top_level_files || RECURSIVE_DEPTH.load(Ordering::SeqCst) > 1 {
            let mut files = VecDeque::new();
            self.read_directory_file_entries_into_string_list(host, &mut files);
            let mut prefix = path.to_vec();
            ensure_trailing_separator(&mut prefix);
            out_list.extend(files.into_iter().map(|name| [prefix.as_slice(), &name].concat()));
        }

        let filter = std::mem::replace(&mut self.filter, b"*".to_vec());
        let mut directories = VecDeque::new();
        self.read_directory_directory_entries_into_string_list(host, &mut directories, 0);
        self.filter = filter;
        for name in directories {
            // Intentionally no separator between the original argument and child name (§6.6).
            let child = [path, name.as_slice(), b"\\"].concat();
            self.read_directory_file_entries_into_string_list_recursive(
                host,
                &child,
                out_list,
                include_top_level_files,
            );
        }
        true
    }

    pub fn does_any_entry_exist_that_matches_pattern(host: &mut impl RzDirectoryHost, pattern: &[u8]) -> bool {
        if let Some((handle, _)) = host.find_first(pattern) {
            host.find_finish(handle);
            true
        } else {
            false
        }
    }

    pub fn does_directory_exist(host: &mut impl RzDirectoryHost, path: &[u8]) -> bool {
        let mut pattern = path.to_vec();
        ensure_trailing_separator(&mut pattern);
        pattern.push(b'*');
        Self::does_any_entry_exist_that_matches_pattern(host, &pattern)
    }

    pub fn create_directory(host: &mut impl RzDirectoryHost, path: &[u8]) -> bool {
        if path.is_empty() {
            return false;
        }
        let mut path = path.to_vec();
        ensure_trailing_separator(&mut path);
        let start = if path.len() > 2 && path.starts_with(b"\\\\") && path[2] != b'\\' {
            // Skip the server prefix, through the third backslash, not the share (§6.7).
            // With no third backslash, literally skip the entire server prefix (RZ11).
            path[2..]
                .iter()
                .position(|&byte| byte == b'\\')
                .map_or(path.len(), |offset| offset + 3)
        } else {
            0
        };
        let mut created: Vec<&[u8]> = Vec::new();
        for index in start..path.len() {
            if is_separator(path[index]) {
                let prefix = &path[..=index];
                if !Self::does_directory_exist(host, prefix) {
                    if !host.create_directory(prefix) {
                        // Outermost first, even though removal may fail while children exist.
                        for prefix in created {
                            host.remove_directory(prefix);
                        }
                        return false;
                    }
                    created.push(prefix);
                }
            }
        }
        true
    }

    /// Transactional outputs: both unchanged on empty input or a part exceeding 254 bytes (§6.7).
    pub fn split_directory_path(&self, last: &mut Vec<u8>, parent: &mut Vec<u8>) -> bool {
        split_directory_path(&self.path, last, parent)
    }
}

impl Drop for RzDirectory {
    fn drop(&mut self) {
        // §6.3 destroys filter, then path, then entries. Empty fields subsequently drop harmlessly.
        drop(std::mem::take(&mut self.filter));
        drop(std::mem::take(&mut self.path));
        drop(std::mem::take(&mut self.entries));
    }
}

fn merge_sort(mut left: VecDeque<Vec<u8>>) -> VecDeque<Vec<u8>> {
    if left.len() < 2 {
        return left;
    }
    let right = left.split_off(left.len() / 2);
    let mut left = merge_sort(left);
    let mut right = merge_sort(right);
    let mut sorted = VecDeque::with_capacity(left.len() + right.len());
    while let (Some(a), Some(b)) = (left.front(), right.front()) {
        // Choosing the left on equal bytes preserves the original order of equal strings.
        if a <= b {
            sorted.push_back(left.pop_front().unwrap());
        } else {
            sorted.push_back(right.pop_front().unwrap());
        }
    }
    sorted.append(&mut left);
    sorted.append(&mut right);
    sorted
}

fn enumerate(host: &mut impl RzDirectoryHost, pattern: &[u8], mut visit: impl FnMut(DirectoryMatch)) -> bool {
    let Some((mut handle, first)) = host.find_first(pattern) else {
        return false;
    };
    visit(first);
    while let Some(found) = host.find_next(&mut handle) {
        visit(found);
    }
    host.find_finish(handle);
    true
}

pub fn is_separator(byte: u8) -> bool {
    byte == b'\\' || byte == b'/'
}

/// §6.2: empty becomes backslash, existing mixed separators and all other bytes are preserved.
pub fn ensure_trailing_separator(path: &mut Vec<u8>) {
    if !path.last().copied().is_some_and(is_separator) {
        path.push(b'\\');
    }
}

fn split_directory_path(path: &[u8], last: &mut Vec<u8>, parent: &mut Vec<u8>) -> bool {
    if path.is_empty() {
        return false;
    }
    let end = path.len() - usize::from(is_separator(path[path.len() - 1]));
    let split = path[..end]
        .iter()
        .rposition(|&byte| is_separator(byte))
        .map_or(0, |index| index + 1);
    let mut new_last = path[split..].to_vec();
    let mut new_parent = path[..split].to_vec();
    ensure_trailing_separator(&mut new_last);
    // The specified drive-root case explicitly returns an empty parent, not a backslash.
    if !new_parent.is_empty() {
        ensure_trailing_separator(&mut new_parent);
    }
    // Literal resulting-part limit; RZ10 asks whether native checks before appending instead.
    if new_last.len() > 254 || new_parent.len() > 254 {
        return false;
    }
    *last = new_last;
    *parent = new_parent;
    true
}

#[cfg(test)]
#[path = "directory_tests.rs"]
mod tests;
