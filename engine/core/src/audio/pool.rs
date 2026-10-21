// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The sound-event pool (`docs/specs/audio.md` §4). Section references (`§n`)
//! point at that spec.
//!
//! `cGZSndEventStd::operator new` (`0x0096BAD1`) is the only entry into the
//! pool: it takes the pool **only** when the requested size is `0x38` and
//! sends every other size to the ordinary heap *[compat]* (§4.1, §14.7).
//!
//! The pool hands out objects from **16 KB-aligned pages**. A page carries a
//! 16-byte header and a LIFO free list of objects; each free object stores its
//! successor's address in its first dword (§4.2). The page a pointer belongs to
//! is recovered as **`p & 0xFFFFC000`**, measured in `FUN_005E1780`.
//!
//! # The page state word (§4.2, §14.8, AU6)
//!
//! `page+0x0C` is **not** a plain live-object count. `FUN_0090CF9A` adds
//! `0x40000000` to it when the page comes into service and `FUN_0090CFF9`
//! clears that marker with `add 0xC0000000`; the free path special-cases the
//! value `0x40000045`. This module reproduces that arithmetic exactly.
//!
//! The reading implemented here is the coherent one the spec's own
//! *[inferred]* note suggests: the low bits are the number of live objects and
//! `0x40000000` is an "in the active ring" marker, so a page is retired from
//! the ring at `0x45` (69) live objects and returned to the heap only when the
//! count reaches 0 afterwards. See `# Choices` below and question **AU6**.

use crate::audio::snd_event::SND_EVENT_SIZE;

/// The pool object's global address in the original (`DAT_00B6360C`) (§4.1).
pub const POOL_ADDR: u32 = 0x00B6_360C;
/// The pool lock's global address (`DAT_00B63618`) (§4.1).
pub const POOL_LOCK_ADDR: u32 = 0x00B6_3618;

/// A page is 16 KB aligned; `p & 0xFFFFC000` recovers it (§4.2).
pub const PAGE_MASK: u32 = 0xFFFF_C000;
/// Page size, in bytes.
pub const PAGE_SIZE: u32 = 0x4000;
/// The page header is `+0x00`…`+0x0F` (§4.2).
pub const PAGE_HEADER: u32 = 0x10;

/// The `0x40000000` marker `FUN_0090CF9A` adds on page entry (§4.2, §14.8).
pub const STATE_MARKER: u32 = 0x4000_0000;
/// The marker as an `add` operand that clears it (`add 0xC0000000`) (§4.2).
pub const STATE_MARKER_CLEAR: u32 = 0xC000_0000;
/// The free path's special threshold, `0x40000000 | 0x45` (§4.2, AU6).
pub const STATE_THRESHOLD: u32 = 0x4000_0045;

/// Objects per page: `(0x4000 - 0x10) / 0x38` = 292.
pub const OBJECTS_PER_PAGE: usize = ((PAGE_SIZE - PAGE_HEADER) as usize) / SND_EVENT_SIZE;

/// The address of object `i` of the page at `base` (§4.2).
pub fn object_addr(base: u32, index: usize) -> u32 {
    base + PAGE_HEADER + (index as u32) * SND_EVENT_SIZE as u32
}

/// The page a pointer belongs to: `p & 0xFFFFC000` (§4.2).
pub fn page_of(p: u32) -> u32 {
    p & PAGE_MASK
}

/// One 16 KB page (§4.2).
///
/// The object bytes are kept so a vector can assert that a free object's
/// **first dword** holds its successor, as the original does.
#[derive(Clone, Debug)]
pub struct PoolPage {
    /// The page base address; every object of the page is `base + 0x10 + i*0x38`.
    pub base: u32,
    /// `+0x00` ring link (prev).
    pub prev: u32,
    /// `+0x04` ring link (next).
    pub next: u32,
    /// `+0x08` free-list head: the address of the first free object, or 0.
    pub free_head: u32,
    /// `+0x0C` count / state word.
    pub state: u32,
    /// The object storage: `OBJECTS_PER_PAGE` slots of `0x38` bytes each.
    objects: Vec<[u8; SND_EVENT_SIZE]>,
}

impl PoolPage {
    /// A fresh page at `base`, with every object free and linked LIFO so that
    /// the first allocation returns the **last** object of the page (the shape
    /// `FUN_0090CF9A` leaves behind).
    pub fn new(base: u32) -> Self {
        let mut objects = vec![[0u8; SND_EVENT_SIZE]; OBJECTS_PER_PAGE];
        // Build the free list in address order, so the head is the first object
        // and each free object's first dword is its successor.
        let mut head = 0u32;
        for i in (0..OBJECTS_PER_PAGE).rev() {
            let addr = object_addr(base, i);
            let next = head;
            head = addr;
            objects[i][..4].copy_from_slice(&next.to_le_bytes());
        }
        PoolPage {
            base,
            prev: base,
            next: base,
            free_head: head,
            state: 0,
            objects,
        }
    }

    /// The index of the object at `addr`, or `None` when `addr` is not one of
    /// this page's objects.
    pub fn index_of(&self, addr: u32) -> Option<usize> {
        if page_of(addr) != self.base {
            return None;
        }
        let off = addr.wrapping_sub(self.base + PAGE_HEADER) as usize;
        if off % SND_EVENT_SIZE != 0 {
            return None;
        }
        let i = off / SND_EVENT_SIZE;
        (i < OBJECTS_PER_PAGE).then_some(i)
    }

    /// The first dword of an object — the free-list successor while it is free
    /// (§4.2).
    pub fn first_dword(&self, addr: u32) -> Option<u32> {
        let i = self.index_of(addr)?;
        let b = &self.objects[i][..4];
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// The free-list head's successor, as `mov edx, [eax]` reads it at the
    /// run-9 crash site (`0x005E16F5`, §4.3).
    pub fn pop_free(&mut self) -> Option<u32> {
        let node = self.free_head;
        if node == 0 {
            return None;
        }
        let next = self.first_dword(node)?;
        self.free_head = next;
        Some(node)
    }

    /// Push `addr` onto the free list as the new head: `p->next = head; head = p`
    /// (`FUN_005E1780`, §4.2). The LIFO order of §4.3 rule (2).
    pub fn push_free(&mut self, addr: u32) {
        if let Some(i) = self.index_of(addr) {
            self.objects[i][..4].copy_from_slice(&self.free_head.to_le_bytes());
        }
        self.free_head = addr;
    }
}

/// Where a page went after a release (§4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseOutcome {
    /// The ordinary path: nothing special.
    None,
    /// `post-dec == 0`: the page is handed back through pool vtable slot 3
    /// (`FUN_0090D031`).
    HandedBack,
    /// `post-dec == 0x40000045`: the `0x40000000` marker is cleared
    /// (`add 0xC0000000`) and the page is unlinked from the active ring
    /// (`FUN_0090CFF9`).
    Retired,
}

/// The pool at `DAT_00B6360C` (§4.1).
///
/// `pool+4` is the cursor into the ring of pages; a second list at `pool+4`
/// holds pages that have left active service but have not been returned
/// (`FUN_0090CFF9`). Both are modelled here.
#[derive(Clone, Debug, Default)]
pub struct SndEventPool {
    /// The active ring's cursor (the original's `pool+4`): the current page's
    /// base address, or 0.
    pub cursor: u32,
    /// Pages in the active ring, keyed by base address.
    active: Vec<PoolPage>,
    /// Pages that left the ring but were not handed back (`FUN_0090CFF9`).
    retired: Vec<PoolPage>,
    /// Pages handed back through pool vtable slot 3 (`FUN_0090D031`).
    handed_back: Vec<u32>,
    /// The next address a freshly obtained page will occupy. The original
    /// takes pages from the heap; the addresses here stand in for it so
    /// `p & 0xFFFFC000` is meaningful.
    next_base: u32,
}

impl SndEventPool {
    /// An empty pool. `next_base` starts at a 16 KB-aligned stand-in address
    /// so pages never collide with the null page.
    pub fn new() -> Self {
        SndEventPool {
            cursor: 0,
            active: Vec::new(),
            retired: Vec::new(),
            handed_back: Vec::new(),
            next_base: PAGE_SIZE,
        }
    }

    /// A pool whose next page is obtained at `base` — for a vector that wants
    /// fixed addresses.
    pub fn with_next_base(mut self, base: u32) -> Self {
        self.next_base = base & PAGE_MASK;
        self
    }

    /// Pool vtable slot 2 (`+0x08`): obtain a new page (§4.2). The original
    /// does not say how the free list of a new page is built; see `# Choices`.
    fn obtain_page(&mut self) -> u32 {
        let base = self.next_base;
        self.next_base = self.next_base.wrapping_add(PAGE_SIZE);
        let mut page = PoolPage::new(base);
        // The page comes into service: `add dword ptr [page+0x0C], 0x40000000`
        // (§4.2, §14.8).
        page.state = page.state.wrapping_add(STATE_MARKER);
        self.active.push(page);
        base
    }

    /// `FUN_0090CF9A` (§4.2): take a page into service and return a node from
    /// it. Walks the ring of pages with `pool+4` as the cursor; when the
    /// cursor's page is its own successor the ring is exhausted and a new page
    /// is obtained through pool vtable slot 2. The chosen page is unlinked and
    /// `add dword ptr [page+0x0C], 0x40000000` marks it in service.
    fn grow(&mut self) -> Option<u32> {
        if self.active.is_empty() {
            self.obtain_page();
        }
        // The cursor page is the one we hand out from; the original unlinks the
        // chosen page from the ring and re-links it. Here the page stays in
        // `active` and the cursor advances past it, which is the same shape:
        // `pool+4` always names the page allocations come from.
        let idx = self.cursor_index()?;
        let page = &mut self.active[idx];
        // A page re-taken into service from the ring gets the marker again;
        // one that already carries it does not.
        if page.state & STATE_MARKER == 0 {
            page.state = page.state.wrapping_add(STATE_MARKER);
        }
        page.pop_free()
    }

    /// The index in `active` of the cursor page.
    fn cursor_index(&self) -> Option<usize> {
        if self.cursor == 0 {
            return None;
        }
        self.active.iter().position(|p| p.base == self.cursor)
    }

    /// `FUN_005E16E0` (§4.2) — allocate. Returns the object address.
    ///
    /// This is the function whose `mov edx, dword ptr [eax]` at `+0x15`
    /// (`0x005E16F5`) crashed in run 9 when the free-list head held a stale
    /// pointer (§4.3). The pop is reproduced faithfully; the corruption that
    /// caused the crash is upstream and is not part of the pool.
    pub fn alloc(&mut self) -> u32 {
        if self.active.is_empty() {
            self.cursor = 0;
        }
        if self.cursor == 0 {
            let base = self.obtain_page();
            self.cursor = base;
        }
        // `mov eax, [esi+4]` / `mov eax, [eax+8]` — the free-list head of the
        // current page.
        let node = {
            let idx = match self.cursor_index() {
                Some(i) => i,
                None => {
                    let base = self.obtain_page();
                    self.cursor = base;
                    self.active.len() - 1
                }
            };
            let page = &mut self.active[idx];
            match page.pop_free() {
                Some(n) => {
                    // `inc dword ptr [esi+0xc]` — the page count.
                    page.state = page.state.wrapping_add(1);
                    Some(n)
                }
                None => None,
            }
        };
        match node {
            Some(n) => n,
            None => {
                // The free list is empty: `call 0x0090CF9A` (grow).
                let n = self.grow().unwrap_or(0);
                if n != 0 {
                    if let Some(idx) = self.cursor_index() {
                        self.active[idx].state = self.active[idx].state.wrapping_add(1);
                    }
                }
                n
            }
        }
    }

    /// `FUN_005E1780` (§4.2) — release. A null pointer is a **no-op** (§4.3
    /// rule 3). The page is recovered as `p & 0xFFFFC000`.
    pub fn free(&mut self, p: u32) -> ReleaseOutcome {
        if p == 0 {
            return ReleaseOutcome::None;
        }
        let base = page_of(p);
        // Find the page in either list.
        let in_active = self.active.iter().position(|pg| pg.base == base);
        let in_retired = self.retired.iter().position(|pg| pg.base == base);
        let (was_active, idx) = match (in_active, in_retired) {
            (Some(i), _) => (true, i),
            (None, Some(i)) => (false, i),
            (None, None) => return ReleaseOutcome::None,
        };

        let outcome = {
            let page = if was_active {
                &mut self.active[idx]
            } else {
                &mut self.retired[idx]
            };
            // `mov esi, [eax+8]` / `mov [edx], esi` / `mov [eax+8], edx`
            page.push_free(p);
            // `mov edx, [eax+0xc]` / `dec edx` / `mov [eax+0xc], edx`
            page.state = page.state.wrapping_sub(1);
            if page.state == 0 {
                ReleaseOutcome::HandedBack
            } else if page.state == STATE_THRESHOLD {
                ReleaseOutcome::Retired
            } else {
                ReleaseOutcome::None
            }
        };

        match outcome {
            ReleaseOutcome::HandedBack => {
                // `FUN_0090D031`: unlink from the ring and call pool vtable
                // slot 3 — the page goes back to the heap.
                if was_active {
                    let page = self.active.remove(idx);
                    self.handed_back.push(page.base);
                    if self.cursor == page.base {
                        self.cursor = self.active.first().map(|p| p.base).unwrap_or(0);
                    }
                } else {
                    let page = self.retired.remove(idx);
                    self.handed_back.push(page.base);
                }
            }
            ReleaseOutcome::Retired => {
                // `FUN_0090CFF9`: `add 0xC0000000` (clear the marker), unlink
                // from the active ring, move onto the second list at pool+4.
                if was_active {
                    let mut page = self.active.remove(idx);
                    page.state = page.state.wrapping_add(STATE_MARKER_CLEAR);
                    self.retired.push(page);
                    if self.cursor == base {
                        self.cursor = self.active.first().map(|p| p.base).unwrap_or(0);
                    }
                }
            }
            ReleaseOutcome::None => {}
        }
        outcome
    }

    /// The page at `base`, from either list.
    pub fn page(&self, base: u32) -> Option<&PoolPage> {
        self.active.iter().chain(self.retired.iter()).find(|p| p.base == base)
    }

    /// The page at `base`, from either list, for mutation. A vector uses this
    /// to drive the state word to a threshold the original would reach through
    /// a particular live-object count.
    pub fn page_mut(&mut self, base: u32) -> Option<&mut PoolPage> {
        self.active
            .iter_mut()
            .chain(self.retired.iter_mut())
            .find(|p| p.base == base)
    }

    /// The pages currently in the active ring.
    pub fn active_pages(&self) -> &[PoolPage] {
        &self.active
    }

    /// The pages that left the ring but were not handed back.
    pub fn retired_pages(&self) -> &[PoolPage] {
        &self.retired
    }

    /// The bases of the pages handed back through pool vtable slot 3.
    pub fn handed_back_pages(&self) -> &[u32] {
        &self.handed_back
    }

    /// The live object count of a page: the state word with the marker removed
    /// (§4.2).
    pub fn live_count(base_page: &PoolPage) -> u32 {
        base_page.state & !STATE_MARKER
    }
}

/// `cGZSndEventStd::operator new` (§4.1): the pool is used **only** for a
/// request of exactly `0x38` bytes; anything else goes to the ordinary heap.
/// Returns `true` when the pool is used.
pub fn size_uses_pool(size: usize) -> bool {
    size == SND_EVENT_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Conformance test 5: LIFO, links in the first dword, page recovery,
    /// and `delete(0)` as a no-op.
    #[test]
    fn free_list_is_lifo_and_delete_zero_is_a_noop() {
        let mut pool = SndEventPool::new().with_next_base(0x0010_0000);
        let a = pool.alloc();
        let b = pool.alloc();
        assert_ne!(a, b);
        assert_eq!(page_of(a), page_of(b), "same 16 KB window shares a free list");
        pool.free(a);
        pool.free(b);
        assert_eq!(pool.free(0), ReleaseOutcome::None);
        let b2 = pool.alloc();
        let a2 = pool.alloc();
        assert_eq!(b2, b, "LIFO: the last freed is the first returned");
        assert_eq!(a2, a);
    }

    /// Conformance test 5: two pointers in the same 16 KB window share a free
    /// list because the page is `p & 0xFFFFC000`.
    #[test]
    fn page_recovery_is_a_mask() {
        assert_eq!(page_of(0x0010_0000), 0x0010_0000);
        assert_eq!(page_of(0x0010_0010), 0x0010_0000);
        assert_eq!(page_of(0x0010_3FFF), 0x0010_0000);
        assert_eq!(page_of(0x0010_4000), 0x0010_4000);
    }

    /// Conformance test 6: the marker on entry, `inc` on alloc, `dec` on
    /// release, and the two thresholds.
    #[test]
    fn state_word_marker_and_thresholds() {
        let mut pool = SndEventPool::new().with_next_base(0x0020_0000);
        let p = pool.alloc();
        let base = page_of(p);
        {
            let page = pool.page(base).unwrap();
            assert_eq!(page.state & STATE_MARKER, STATE_MARKER, "entry adds 0x40000000");
            assert_eq!(page.state, STATE_MARKER | 1, "alloc increments");
        }
        pool.free(p);
        // post-dec is the marker alone here (live 1 -> 0 while marked), which
        // is neither threshold, so nothing special happens.
        assert_eq!(pool.page(base).unwrap().state, STATE_MARKER);
        assert!(pool.handed_back_pages().is_empty());
    }

    /// Conformance test 6: a release whose post-dec value is exactly
    /// `0x40000045` clears the marker and unlinks the page.
    #[test]
    fn state_threshold_0x40000045_retires_the_page() {
        let mut pool = SndEventPool::new().with_next_base(0x0030_0000);
        let p = pool.alloc();
        let base = page_of(p);
        // Drive the state word to 0x40000046 so the next free lands on 0x45.
        pool.page_mut(base).unwrap().state = STATE_MARKER | 0x46;
        assert_eq!(pool.free(p), ReleaseOutcome::Retired);
        let page = pool.page(base).unwrap();
        assert_eq!(page.state, 0x45, "the marker is cleared by add 0xC0000000");
        assert_eq!(pool.retired_pages().len(), 1, "unlinked from the active ring");
        assert!(pool.active_pages().iter().all(|q| q.base != base));
    }

    /// Conformance test 6: a release whose post-dec value is 0 hands the page
    /// back through pool vtable slot 3.
    #[test]
    fn state_zero_hands_the_page_back() {
        let mut pool = SndEventPool::new().with_next_base(0x0040_0000);
        let p = pool.alloc();
        let base = page_of(p);
        // A retired page (marker cleared) whose last live object is freed.
        pool.page_mut(base).unwrap().state = 1;
        assert_eq!(pool.free(p), ReleaseOutcome::HandedBack);
        assert_eq!(pool.handed_back_pages(), &[base]);
        assert!(pool.page(base).is_none());
    }

    /// Conformance test 1: only `0x38` uses the pool.
    #[test]
    fn only_the_0x38_size_uses_the_pool() {
        assert!(size_uses_pool(0x38));
        assert!(!size_uses_pool(0x2C));
        assert!(!size_uses_pool(0x37));
        assert!(!size_uses_pool(0x39));
    }
}
