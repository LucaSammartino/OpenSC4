//! Directory entry state, its owning list, and native find lifetime (paths.md §§3.1–3.4, 4).
//!
//! This is the entry/list portion of cRZDirectory, not a claim for its recursive/create/remove
//! methods. PA5 requests cRZDirectoryEntry's missing standalone method contracts. A borrowed entry
//! exposes precisely the published owner/name/kind state, without inventing pointer ownership.

use std::cmp::Ordering;

use crate::rz_files::{OsError, OsHandle};

use super::helpers::is_path_separator;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindData {
    pub name: Vec<u8>,
    pub directory: bool,
}

/// Primitive ANSI Win32 find operations (§4). No helper or owner result is delegated here.
/// The adapter converts framework patterns to system encoding and names back; it must preserve
/// native wildcard/short-name/case-insensitive matching and native enumeration order, not emulate
/// them with the host platform's path library. No such adapter is supplied by this portable module.
pub trait DirectoryProvider {
    fn find_first(&mut self, pattern: &[u8]) -> Result<(OsHandle, FindData), OsError>;
    fn find_next(&mut self, handle: OsHandle) -> Option<FindData>;
    fn find_finish(&mut self, handle: OsHandle);
}

/// Owns exactly the search handle it created. Drop implements FindFinish, including early exits.
pub struct FindSearch<'a, P: DirectoryProvider> {
    provider: &'a mut P,
    handle: OsHandle,
    pattern: Vec<u8>,
    current: FindData,
}

impl<'a, P: DirectoryProvider> FindSearch<'a, P> {
    pub fn first(provider: &'a mut P, pattern: &[u8]) -> Option<Self> {
        let (handle, current) = provider.find_first(pattern).ok()?;
        Some(Self { provider, handle, pattern: pattern.to_vec(), current })
    }

    pub fn current(&self) -> &FindData {
        &self.current
    }

    pub fn pattern(&self) -> &[u8] {
        &self.pattern
    }

    pub fn advance(&mut self) -> bool {
        if let Some(current) = self.provider.find_next(self.handle) {
            self.current = current;
            true
        } else {
            // PA7: §4 does not specify the cached fields after a failed next; leave them unchanged.
            false
        }
    }
}

impl<P: DirectoryProvider> Drop for FindSearch<'_, P> {
    fn drop(&mut self) {
        self.provider.find_finish(self.handle);
    }
}

#[derive(Debug)]
struct EntryState {
    name: Vec<u8>,
    kind: u32,
}

/// Published cRZDirectoryEntry state (§3.1), valid only while its directory/list remains borrowed.
/// Names belong to the directory; callers cannot retain a dangling owner or mutate/free the name.
/// No AddRef/Release, deep-copy or independent entry factory is asserted by the spec (PA5).
pub struct RzDirectoryEntry<'a> {
    pub owner: &'a RzDirectory,
    pub name: &'a [u8],
    pub kind: u32,
}

#[derive(Debug)]
pub struct RzDirectory {
    path: Vec<u8>,
    filter: Vec<u8>,
    entries: Vec<EntryState>,
    include_parent: bool,
    include_self: bool,
}

fn end_with_separator(path: &mut Vec<u8>) {
    if !path.last().is_some_and(|&c| is_path_separator(c)) {
        path.push(b'\\');
    }
}

impl RzDirectory {
    pub fn new(
        path: &[u8],
        read: bool,
        include_parent: bool,
        include_self: bool,
        provider: &mut impl DirectoryProvider,
    ) -> Self {
        let mut result = Self {
            path: path.to_vec(), filter: b"*".to_vec(), entries: Vec::new(),
            include_parent, include_self,
        };
        end_with_separator(&mut result.path);
        if read {
            result.read_directory_entries(provider);
        }
        result
    }

    pub fn path(&self) -> &[u8] { &self.path }
    pub fn filter(&self) -> &[u8] { &self.filter }
    pub fn entry_count(&self) -> usize { self.entries.len() }

    pub fn change_directory(&mut self, path: &[u8], read: bool, provider: &mut impl DirectoryProvider) -> bool {
        self.path = path.to_vec();
        end_with_separator(&mut self.path);
        if read {
            self.read_directory_entries(provider);
        }
        true
    }

    pub fn set_new_directory_entry_filter(&mut self, filter: &[u8], read: bool, provider: &mut impl DirectoryProvider) {
        self.filter = if filter.is_empty() { b"*".to_vec() } else { filter.to_vec() };
        if read {
            self.read_directory_entries(provider);
        }
    }

    pub fn get_nth_entry(&self, n: usize) -> Option<RzDirectoryEntry<'_>> {
        self.entries.get(n).map(|e| RzDirectoryEntry { owner: self, name: &e.name, kind: e.kind })
    }

    pub fn alphabetize_entry_list(&mut self, mut cmp: impl FnMut(&[u8], &[u8]) -> Ordering) {
        // PA8: equal-key ordering is unpublished; use Rust's stable sort without a built-in comparer.
        self.entries.sort_by(|a, b| cmp(&a.name, &b.name));
    }

    fn pattern(&self) -> Vec<u8> {
        let mut pattern = self.path.clone();
        pattern.extend_from_slice(&self.filter);
        pattern
    }

    pub fn read_directory_entries(&mut self, provider: &mut impl DirectoryProvider) -> bool {
        self.entries.clear();
        let Some(mut search) = FindSearch::first(provider, &self.pattern()) else { return false };
        let mut seen_self = false;
        let mut seen_parent = false;
        loop {
            let data = search.current();
            let keep = if data.directory && data.name == b"." {
                seen_self = true;
                self.include_self
            } else if data.directory && data.name == b".." {
                seen_parent = true;
                self.include_parent
            } else { true };
            if keep {
                self.entries.push(EntryState { name: data.name.clone(), kind: if data.directory { 4 } else { 8 } });
            }
            if !search.advance() { break; }
        }
        if self.filter == b"*" {
            if self.include_parent && !seen_parent && !self.entries.iter().any(|e| e.name == b"..") {
                self.entries.insert(0, EntryState { name: b"..".to_vec(), kind: 4 });
            }
            if self.include_self && !seen_self && !self.entries.iter().any(|e| e.name == b".") {
                self.entries.insert(0, EntryState { name: b".".to_vec(), kind: 4 });
            }
        }
        true
    }

    pub fn read_directory_specific_entries_into_string_list(
        &self, list: &mut Vec<Vec<u8>>, flags: u32, provider: &mut impl DirectoryProvider,
    ) -> bool {
        let Some(mut search) = FindSearch::first(provider, &self.pattern()) else { return false };
        let mut seen_self = false;
        let mut seen_parent = false;
        loop {
            let data = search.current();
            let keep = if data.directory {
                let dot = if data.name == b"." { seen_self = true; flags & 1 != 0 }
                    else if data.name == b".." { seen_parent = true; flags & 2 != 0 }
                    else { true };
                flags & 4 != 0 && dot
            } else { flags & 8 != 0 };
            if keep { list.push(data.name.clone()); }
            if !search.advance() { break; }
        }
        if self.filter == b"*" {
            if flags & 2 != 0 && !seen_parent { list.insert(0, b"..".to_vec()); }
            if flags & 1 != 0 && !seen_self { list.insert(0, b".".to_vec()); }
        }
        true
    }

    pub fn read_directory_file_entries_into_string_list(&self, list: &mut Vec<Vec<u8>>, provider: &mut impl DirectoryProvider) -> bool {
        self.read_directory_specific_entries_into_string_list(list, 8, provider)
    }

    pub fn read_directory_directory_entries_into_string_list(&self, list: &mut Vec<Vec<u8>>, dots: u32, provider: &mut impl DirectoryProvider) -> bool {
        self.read_directory_specific_entries_into_string_list(list, (dots & 3) | 4, provider)
    }
}
