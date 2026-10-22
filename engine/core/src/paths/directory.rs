//! Entry lifecycle and primitive find wrappers (paths.md §§3.10, 4).
//! The directory owner, enumeration and string-list sort live only in rz_files::directory.

use crate::rz_files::{OsError, OsHandle};

pub use crate::rz_files::directory::RzDirectory;

/// The three published entry bodies: default, independent name copy, and name-only destruction.
/// `owner` is an opaque borrowed address. It may dangle after directory destruction; it must never
/// be dereferenced without an independently valid directory borrow, nor released on entry drop.
/// This standalone value does not turn GetNthEntry's borrowed list payload into an owned entry.
/// Rust/Box handles allocation reclamation; no native deleting-destructor flags ABI is asserted.
#[derive(Clone, Debug)]
pub struct RzDirectoryEntry {
    pub owner: *const RzDirectory,
    pub name: Vec<u8>,
    pub kind: u32,
}

impl Default for RzDirectoryEntry {
    fn default() -> Self {
        Self {
            owner: std::ptr::null(),
            name: Vec::new(),
            kind: 0,
        }
    }
}

// Vec's destructor destroys only the name; the raw borrowed pointer has no drop operation.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindData {
    pub name: Vec<u8>,
    pub directory: bool,
}

/// Primitive ANSI find operations. The adapter alone converts framework/system encoding and
/// supplies OS wildcard matching and order. No owner algorithm or native output is delegated.
pub trait DirectoryProvider {
    fn find_first(&mut self, pattern: &[u8]) -> Result<(OsHandle, FindData), OsError>;
    fn find_next(&mut self, handle: OsHandle) -> Option<FindData>;
    fn find_finish(&mut self, handle: OsHandle);
}

/// Owns only its created search handle. Finish (or Drop's finish fallback) closes it exactly once.
pub struct FindSearch<'a, P: DirectoryProvider> {
    provider: &'a mut P,
    handle: OsHandle,
    pattern: Vec<u8>,
    current: FindData,
}

impl<'a, P: DirectoryProvider> FindSearch<'a, P> {
    pub fn first(provider: &'a mut P, pattern: &[u8]) -> Option<Self> {
        let (handle, current) = provider.find_first(pattern).ok()?;
        Some(Self {
            provider,
            handle,
            pattern: pattern.to_vec(),
            current,
        })
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
            // PA7: retain the cached name, flag and handle; another call retries the provider.
            false
        }
    }

    /// Consuming finish delegates closure to the same Drop path, including after failed next.
    pub fn finish(self) {}
}

impl<P: DirectoryProvider> Drop for FindSearch<'_, P> {
    fn drop(&mut self) {
        self.provider.find_finish(self.handle);
    }
}

/// Nullable free-function next boundary (§4.1); no search means false and no provider call.
pub fn find_next<P: DirectoryProvider>(search: Option<&mut FindSearch<'_, P>>) -> bool {
    search.is_some_and(FindSearch::advance)
}

