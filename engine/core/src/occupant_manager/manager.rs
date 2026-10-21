// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The occupant manager object (§2, §4–§13).

use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

use crate::stream::{InStream, OutStream, SaveObject};
use crate::x87::ftol32;

use super::cells::{
    cell_box_of, clamp_cell, overlaps, position_to_occupant_manager_cell, position_to_standard_city_cell,
    position_to_world_cell, shrink, standard_city_cell_to_position, world_cell_to_occupant_manager_cell,
    world_cell_to_position, Vec3, SHRINK,
};
use super::{
    msg, Filter, OccupantHost, CELL_SIZE, CITY_ORIGIN_CELL, CLSID_OCCUPANT_MANAGER, FLAG_INSERTED, IID_MESSAGE_TARGET2,
    IID_OCCUPANT, IID_OCCUPANT_MANAGER, IID_SERIALIZABLE, IID_UNKNOWN, RECORD_VERSION, WORLD_CELLS,
};

/// The view `QueryInterface` returns (§1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interface {
    /// `1` and `0xC6BB8334`: this.
    OccupantManager,
    /// `0xE4FDA3D4`: this + 4.
    Serializable,
    /// `0x452294AA`: this + 8.
    MessageTarget2,
}

/// `QueryInterface` (§1); any other IID fails.
pub fn query_interface(iid: u32) -> Option<Interface> {
    match iid {
        IID_UNKNOWN | IID_OCCUPANT_MANAGER => Some(Interface::OccupantManager),
        IID_SERIALIZABLE => Some(Interface::Serializable),
        IID_MESSAGE_TARGET2 => Some(Interface::MessageTarget2),
        _ => None,
    }
}

/// A list node's value (§4.2): the occupant and bit 0, "this occupant's box
/// is a single cell".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node {
    pub occupant: u64,
    pub single: bool,
}

/// A cell: `None` = null pointer; `Some` = holder and list, front first.
type CellList = Option<VecDeque<Node>>;

/// A parsed record (§13): sizes, counts and the grid.
type Parsed = (f32, f32, [i32; 4], Vec<Vec<CellList>>);

/// The state behind the lock.
#[derive(Default)]
struct Inner {
    ref_count: u32,
    destroyed: bool,
    /// `+0xD`: set by `Read`, never read (§15 quirk 14).
    was_read: bool,
    /// Rows indexed by x, cells by z (§4.1).
    grid: Vec<Vec<CellList>>,
}

/// Diagnostics: the object's fields (§4.1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OccupantSnapshot {
    pub initialized: bool,
    pub was_read: bool,
    pub ref_count: u32,
    pub destroyed: bool,
    pub size: (f32, f32),
    /// `+0x24`, `+0x28`, `+0x2C`, `+0x30`.
    pub counts: [i32; 4],
    /// The shared filter (`None` = null) and its type.
    pub filter_type: Option<u32>,
    /// Rows of the grid (0 before the first `Init` / `Read`).
    pub rows: usize,
    /// Cells with a holder (a list, possibly empty).
    pub holders: usize,
    /// List nodes in all cells.
    pub nodes: usize,
}

/// Leaves the host lock when dropped.
struct Held<'a>(&'a dyn OccupantHost);

impl Drop for Held<'_> {
    fn drop(&mut self) {
        self.0.unlock();
    }
}

/// The occupant manager.
pub struct OccupantManager {
    host: Rc<dyn OccupantHost>,
    /// `+0xC`.
    initialized: AtomicBool,
    /// `+0x1C`, `+0x20` (`float` bits).
    size: [AtomicU32; 2],
    /// `+0x24`…`+0x30`.
    counts: [AtomicI32; 4],
    /// `+0x34` is non-null.
    filter_present: AtomicBool,
    /// The shared filter's type (`+0xC` of the filter).
    filter_type: AtomicU32,
    s: RefCell<Inner>,
}

fn in_list(l: &CellList, occ: u64) -> bool {
    l.as_ref().is_some_and(|l| l.iter().any(|n| n.occupant == occ))
}

impl OccupantManager {
    /// A constructed manager: not initialised, reference count 0, no grid
    /// rows, null filter, sizes and counts 0.
    pub fn new(host: Rc<dyn OccupantHost>) -> OccupantManager {
        OccupantManager {
            host,
            initialized: AtomicBool::new(false),
            size: [AtomicU32::new(0), AtomicU32::new(0)],
            counts: [
                AtomicI32::new(0),
                AtomicI32::new(0),
                AtomicI32::new(0),
                AtomicI32::new(0),
            ],
            filter_present: AtomicBool::new(false),
            filter_type: AtomicU32::new(0),
            s: RefCell::new(Inner::default()),
        }
    }

    fn lock(&self) -> Held<'_> {
        self.host.lock();
        Held(&*self.host)
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    fn count(&self, i: usize) -> i32 {
        self.counts[i].load(Ordering::SeqCst)
    }

    fn size_of(&self, i: usize) -> f32 {
        f32::from_bits(self.size[i].load(Ordering::SeqCst))
    }

    /// The diagnostics snapshot. Takes the lock.
    pub fn snapshot(&self) -> OccupantSnapshot {
        let _g = self.lock();
        let s = self.s.borrow();
        let holders = s.grid.iter().flatten().filter(|c| c.is_some()).count();
        let nodes = s.grid.iter().flatten().flatten().map(VecDeque::len).sum();
        OccupantSnapshot {
            initialized: self.is_initialized(),
            was_read: s.was_read,
            ref_count: s.ref_count,
            destroyed: s.destroyed,
            size: (self.size_of(0), self.size_of(1)),
            counts: [self.count(0), self.count(1), self.count(2), self.count(3)],
            filter_type: self.shared_filter_type(),
            rows: s.grid.len(),
            holders,
            nodes,
        }
    }

    /// The shared filter's type, `None` while the filter is null (§4.4).
    pub fn shared_filter_type(&self) -> Option<u32> {
        self.filter_present
            .load(Ordering::SeqCst)
            .then(|| self.filter_type.load(Ordering::SeqCst))
    }

    /// A cell's list, front first: `None` for a null cell or a cell outside
    /// the grid. Takes the lock.
    pub fn cell(&self, x: i32, z: i32) -> Option<Vec<Node>> {
        let _g = self.lock();
        self.cell_nodes(x, z)
    }

    /// The number of grid rows and of cells in row 0.
    pub fn grid_size(&self) -> (usize, usize) {
        let _g = self.lock();
        let s = self.s.borrow();
        (s.grid.len(), s.grid.first().map_or(0, Vec::len))
    }

    // ---- reference count (§2.4) ----

    /// Slot 1 `AddRef`.
    pub fn add_ref(&self) -> u32 {
        let _g = self.lock();
        let mut s = self.s.borrow_mut();
        s.ref_count = s.ref_count.wrapping_add(1);
        s.ref_count
    }

    /// Slot 2 `Release` (§2.4): above 1 it decrements; at the last reference
    /// (1, or 0 for a never-held object) it calls `Shutdown`, destroys the
    /// object (here: marks it) and returns 0.
    pub fn release(&self) -> u32 {
        let _g = self.lock();
        let c = self.s.borrow().ref_count;
        if c > 1 {
            self.s.borrow_mut().ref_count = c - 1;
            return c - 1;
        }
        self.shutdown();
        let mut s = self.s.borrow_mut();
        s.ref_count = 0;
        s.destroyed = true;
        0
    }

    // ---- lifecycle (§2) ----

    /// Slot 5 `SetCitySize(fx, fz)` (§2.1): refused (false) once
    /// initialised. Counts are `_ftol` of fx · 1/64 and fx · 1/16.
    pub fn set_city_size(&self, fx: f32, fz: f32) -> bool {
        let _g = self.lock();
        if self.is_initialized() {
            return false;
        }
        self.store_sizes(
            fx,
            fz,
            [
                ftol32(f64::from(fx * 0.015_625)),
                ftol32(f64::from(fz * 0.015_625)),
                ftol32(f64::from(fx * 0.0625)),
                ftol32(f64::from(fz * 0.0625)),
            ],
        );
        true
    }

    fn store_sizes(&self, fx: f32, fz: f32, counts: [i32; 4]) {
        self.size[0].store(fx.to_bits(), Ordering::SeqCst);
        self.size[1].store(fz.to_bits(), Ordering::SeqCst);
        for (a, v) in self.counts.iter().zip(counts) {
            a.store(v, Ordering::SeqCst);
        }
    }

    /// Slot 3 `Init` (§2.2): once. Creates the shared filter (type 0); builds
    /// an empty 192 × 192 grid for a new city, or initialises every loaded
    /// occupant in the city's cells through slot 44's defaults (§15 quirk
    /// 13; a loaded occupant's base `Init` posts no Create, §2.2). Always
    /// true.
    pub fn init(&self) -> bool {
        let _g = self.lock();
        if self.is_initialized() {
            return true;
        }
        self.initialized.store(true, Ordering::SeqCst);
        if !self.filter_present.load(Ordering::SeqCst) {
            self.filter_type.store(0, Ordering::SeqCst);
            self.filter_present.store(true, Ordering::SeqCst);
        }
        let empty = self.s.borrow().grid.is_empty();
        if empty {
            let n = WORLD_CELLS as usize;
            self.s.borrow_mut().grid = (0..n).map(|_| vec![None; n]).collect();
        } else {
            let h = &*self.host;
            self.iterate(
                &mut |o| {
                    h.init(o);
                    true
                },
                None,
                None,
                Filter::All,
            );
        }
        true
    }

    /// Slot 4 `Shutdown` (§2.3): when initialised, removes every occupant
    /// with `shutdown` (Remove, then Destroy, per occupant), nulls the shared
    /// filter and clears the flag. The rows are kept. Always true.
    pub fn shutdown(&self) -> bool {
        let _g = self.lock();
        if self.is_initialized() {
            self.remove_occupants(Filter::All, true, 0);
            self.filter_present.store(false, Ordering::SeqCst);
            self.filter_type.store(0, Ordering::SeqCst);
            self.initialized.store(false, Ordering::SeqCst);
        }
        true
    }

    // ---- slots 6–19 (§3.2): no lock ----

    /// Slot 6: 64.0, 64.0.
    pub fn cell_sizes(&self) -> (f32, f32) {
        (CELL_SIZE, CELL_SIZE)
    }

    /// Slot 7: 192, 192.
    pub fn world_cell_count(&self) -> (i32, i32) {
        (WORLD_CELLS, WORLD_CELLS)
    }

    /// Slot 8: the manager cell counts.
    pub fn cell_count(&self) -> (i32, i32) {
        (self.count(0), self.count(1))
    }

    /// The standard cell counts (`+0x2C`, `+0x30`).
    pub fn standard_cell_count(&self) -> (i32, i32) {
        (self.count(2), self.count(3))
    }

    /// The city size in metres (`+0x1C`, `+0x20`).
    pub fn city_size(&self) -> (f32, f32) {
        (self.size_of(0), self.size_of(1))
    }

    /// Slot 9: (64, 64, count X + 63, count Z + 63), the city's world cells.
    pub fn cell_bounds(&self) -> (i32, i32, i32, i32) {
        (
            CITY_ORIGIN_CELL,
            CITY_ORIGIN_CELL,
            self.count(0).wrapping_add(CITY_ORIGIN_CELL - 1),
            self.count(1).wrapping_add(CITY_ORIGIN_CELL - 1),
        )
    }

    // ---- the grid (§4.2) ----

    fn index(grid: &[Vec<CellList>], x: i32, z: i32) -> Option<(usize, usize)> {
        let (xu, zu) = (usize::try_from(x).ok()?, usize::try_from(z).ok()?);
        (xu < grid.len() && zu < grid[xu].len()).then_some((xu, zu))
    }

    fn cell_nodes(&self, x: i32, z: i32) -> Option<Vec<Node>> {
        let s = self.s.borrow();
        let (xu, zu) = Self::index(&s.grid, x, z)?;
        s.grid[xu][zu].as_ref().map(|l| l.iter().copied().collect())
    }

    fn cell_has(&self, x: i32, z: i32, occ: u64) -> bool {
        let s = self.s.borrow();
        Self::index(&s.grid, x, z).is_some_and(|(xu, zu)| in_list(&s.grid[xu][zu], occ))
    }

    /// Unlinks the first node of `occ` in the cell; frees an emptied cell
    /// (§5.2 step 5). Cells without one, null cells and cells outside the
    /// grid are skipped.
    fn unlink(&self, x: i32, z: i32, occ: u64) {
        let mut s = self.s.borrow_mut();
        let Some((xu, zu)) = Self::index(&s.grid, x, z) else {
            return;
        };
        let cell = &mut s.grid[xu][zu];
        if let Some(l) = cell {
            if let Some(i) = l.iter().position(|n| n.occupant == occ) {
                l.remove(i);
                if l.is_empty() {
                    *cell = None;
                }
            }
        }
    }

    /// Creates the cell's holder and list when null; false when the cell is
    /// outside the grid.
    fn ensure(&self, x: i32, z: i32) -> bool {
        let mut s = self.s.borrow_mut();
        let Some((xu, zu)) = Self::index(&s.grid, x, z) else {
            return false;
        };
        s.grid[xu][zu].get_or_insert_with(VecDeque::new);
        true
    }

    fn push_front(&self, x: i32, z: i32, node: Node) {
        let mut s = self.s.borrow_mut();
        if let Some((xu, zu)) = Self::index(&s.grid, x, z) {
            s.grid[xu][zu].get_or_insert_with(VecDeque::new).push_front(node);
        }
    }

    /// Rewrites the single-cell bit of the first node of `occ` (§5.4 step 8).
    fn rewrite(&self, x: i32, z: i32, occ: u64, single: bool) {
        let mut s = self.s.borrow_mut();
        if let Some((xu, zu)) = Self::index(&s.grid, x, z) {
            if let Some(n) = s.grid[xu][zu]
                .as_mut()
                .and_then(|l| l.iter_mut().find(|n| n.occupant == occ))
            {
                n.single = single;
            }
        }
    }

    /// The grid's rows and the length of row `x`.
    fn rows(&self) -> usize {
        self.s.borrow().grid.len()
    }

    fn row_len(&self, x: usize) -> usize {
        self.s.borrow().grid.get(x).map_or(0, Vec::len)
    }

    fn includes(&self, filter: Filter, occ: u64) -> bool {
        match filter {
            Filter::All => true,
            Filter::Host(f) => self.host.filter_includes(f, occ),
            Filter::Type(t) => self.host.occupant_type(occ) == t,
        }
    }

    // ---- insert, remove, move (§5) ----

    /// Slot 20 `InsertOccupant(occ, data4)` (§5.1).
    pub fn insert(&self, occ: u64, data4: u32) -> bool {
        if occ == 0 {
            return false;
        }
        let h = &*self.host;
        if h.is_flag_set(occ, FLAG_INSERTED) {
            return false;
        }
        let g = self.lock();
        h.add_ref(occ);
        // Before any other check: a failed insert can still post Create
        // (§15 quirk 5).
        h.init(occ);
        if !h.has_property_holder(occ) {
            drop(g);
            h.release(occ);
            return false;
        }
        let b = cell_box_of(&h.bounding_box(occ));
        h.set_manager_bbox(occ, b);
        let node = Node {
            occupant: occ,
            single: b.is_single(),
        };
        for (i, (x, z)) in b.cells().enumerate() {
            if !self.ensure(x, z) {
                // [choice] outside the grid: skipped (OM6).
                continue;
            }
            if i == 0 && self.cell_has(x, z, occ) {
                // The box written above stays (§5.1 step 9).
                drop(g);
                h.release(occ);
                return false;
            }
            self.push_front(x, z, node);
        }
        drop(g);
        h.set_flag(occ, FLAG_INSERTED);
        h.post_message(occ, msg::INSERT, data4);
        true
    }

    /// Slot 21 `RemoveOccupant(occ, shutdown, data4)` (§5.2). `Shutdown` runs
    /// when asked, whether or not the remove succeeded (§15 quirk 6).
    pub fn remove(&self, occ: u64, shutdown: bool, data4: u32) -> bool {
        if occ == 0 {
            return false;
        }
        let h = &*self.host;
        h.add_ref(occ);
        let mut result = false;
        if h.is_flag_set(occ, FLAG_INSERTED) {
            let g = self.lock();
            let b = if h.has_property_holder(occ) {
                h.manager_bbox(occ)
            } else {
                None
            };
            if let Some(b) = b {
                for (x, z) in b.cells() {
                    self.unlink(x, z, occ);
                }
                result = true;
                drop(g);
                h.post_message(occ, msg::REMOVE, data4);
                h.clear_flag(occ, FLAG_INSERTED);
                h.release(occ);
            } else {
                drop(g);
            }
        }
        if shutdown {
            h.shutdown(occ);
        }
        h.release(occ);
        result
    }

    /// Slot 22 `RemoveOccupants(filter, shutdown, data4)` (§5.3): walks the
    /// grid x-major under the lock, removing each included occupant (with a
    /// property holder and a stored box) from every cell of its box at its
    /// first cell and clearing flag 1; then, unlocked and per occupant in
    /// removal order: Remove, `Shutdown` (if asked), `Release`. True.
    pub fn remove_occupants(&self, filter: Filter, shutdown: bool, data4: u32) -> bool {
        let h = &*self.host;
        let g = self.lock();
        let mut removed = Vec::new();
        let mut x = 0;
        while x < self.rows() {
            let mut z = 0;
            while z < self.row_len(x) {
                if let Some(nodes) = self.cell_nodes(x as i32, z as i32) {
                    let collected: Vec<u64> = nodes
                        .iter()
                        .map(|n| n.occupant)
                        .filter(|&o| self.includes(filter, o))
                        .collect();
                    for o in collected {
                        if !h.has_property_holder(o) {
                            continue;
                        }
                        let Some(b) = h.manager_bbox(o) else {
                            continue;
                        };
                        for (cx, cz) in b.cells() {
                            self.unlink(cx, cz, o);
                        }
                        removed.push(o);
                        h.clear_flag(o, FLAG_INSERTED);
                    }
                }
                z += 1;
            }
            x += 1;
        }
        drop(g);
        for o in removed {
            h.post_message(o, msg::REMOVE, data4);
            if shutdown {
                h.shutdown(o);
            }
            h.release(o);
        }
        true
    }

    /// Slot 23 `RemoveOccupants(type, shutdown, data4)` (§5.3): the shared
    /// filter with `type` (type 0 literally). [choice] With a null shared
    /// filter (before `Init`, after `Shutdown`; the original crashes) nothing
    /// is removed and the result is true.
    pub fn remove_occupants_of_type(&self, occ_type: u32, shutdown: bool, data4: u32) -> bool {
        if !self.filter_present.load(Ordering::SeqCst) {
            return true;
        }
        self.filter_type.store(occ_type, Ordering::SeqCst);
        self.remove_occupants(Filter::Type(occ_type), shutdown, data4)
    }

    /// Slot 24 `MoveOccupant(occ, post)` (§5.4): recomputes the box; unlinks
    /// the cells it left, adds it at the front of the cells it entered
    /// (clamped to the grid, no duplicate check) and keeps its place in the
    /// others. Move is posted with `post`, even when the box did not change.
    pub fn move_occupant(&self, occ: u64, post: bool) -> bool {
        if occ == 0 {
            return false;
        }
        let h = &*self.host;
        if !h.is_flag_set(occ, FLAG_INSERTED) {
            return false;
        }
        let g = self.lock();
        let old = if h.has_property_holder(occ) {
            h.manager_bbox(occ)
        } else {
            None
        };
        let Some(old) = old else {
            return false;
        };
        let new = cell_box_of(&h.bounding_box(occ));
        h.set_manager_bbox(occ, new);
        if new != old {
            for (x, z) in old.cells().filter(|&(x, z)| !new.contains(x, z)) {
                self.unlink(x, z, occ);
            }
            let node = Node {
                occupant: occ,
                single: new.is_single(),
            };
            for (x, z) in new.cells().filter(|&(x, z)| !old.contains(x, z)) {
                // The lookup clamps to 0…191 and creates the holder (§5.4
                // step 7).
                self.push_front(clamp_cell(x), clamp_cell(z), node);
            }
            if old.is_single() != new.is_single() {
                for (x, z) in old.cells().filter(|&(x, z)| new.contains(x, z)) {
                    self.rewrite(x, z, occ, new.is_single());
                }
            }
        }
        drop(g);
        if post {
            h.post_message(occ, msg::MOVE, 0);
        }
        true
    }

    // ---- cell helpers (§6) ----

    /// Slot 25 `IsCellEmpty(x, z)` (§6.1): world cells; true outside the
    /// grid, for a null cell or an empty list. The original takes no lock
    /// (OM8); [choice] the core takes it (the grid's `RefCell` is touched
    /// only under the lock; OM8: same thread, no callbacks, not observable).
    pub fn is_cell_empty(&self, x: i32, z: i32) -> bool {
        let _g = self.lock();
        self.cell_nodes(x, z).map_or(true, |l| l.is_empty())
    }

    /// Slot 27 `FindOccupant(occ, searchAll, &x, &z)` (§6.3). The fast path
    /// (the guess cell holds it) returns true **without writing** `x` and
    /// `z` (§15 quirk 7); the full scan writes the first cell found.
    pub fn find_occupant(&self, occ: u64, search_all: bool, x: &mut i32, z: &mut i32) -> bool {
        let h = &*self.host;
        let _g = self.lock();
        let guess = if !h.has_property_holder(occ) {
            // [choice] the original reads uninitialised stack data.
            None
        } else if let Some(b) = h.manager_bbox(occ) {
            Some((i32::from(b.x0), i32::from(b.z0)))
        } else {
            let bb = h.bounding_box(occ);
            let (min_x, _) = shrink(bb.min[0], bb.max[0]);
            Some(position_to_world_cell(min_x, bb.min[2]))
        };
        if let Some((gx, gz)) = guess {
            if self.cell_has(clamp_cell(gx), clamp_cell(gz), occ) {
                return true;
            }
        }
        if search_all {
            for cx in 0..self.rows() {
                for cz in 0..self.row_len(cx) {
                    if self.cell_has(cx as i32, cz as i32, occ) {
                        *x = cx as i32;
                        *z = cz as i32;
                        return true;
                    }
                }
            }
        }
        false
    }

    // ---- first-occupant queries (§7) ----

    /// The first occupant of the cells (x-major, list order) passing the
    /// filter and `test`; AddRef'd.
    fn first_in(
        &self,
        xs: std::ops::RangeInclusive<i32>,
        zs: std::ops::RangeInclusive<i32>,
        filter: Filter,
        test: &dyn Fn(u64) -> bool,
    ) -> Option<u64> {
        // Cells outside the grid hold nothing: walk only the grid's part.
        let (rows, cols) = {
            let s = self.s.borrow();
            (
                s.grid.len() as i32,
                s.grid.iter().map(Vec::len).max().unwrap_or(0) as i32,
            )
        };
        let xs = (*xs.start()).max(0)..=(*xs.end()).min(rows - 1);
        let zs = (*zs.start()).max(0)..=(*zs.end()).min(cols - 1);
        for x in xs {
            for z in zs.clone() {
                let Some(nodes) = self.cell_nodes(x, z) else {
                    continue;
                };
                for n in nodes {
                    if self.includes(filter, n.occupant) && test(n.occupant) {
                        self.host.add_ref(n.occupant);
                        return Some(n.occupant);
                    }
                }
            }
        }
        None
    }

    /// Writes a typed thunk's type into the shared filter (§10).
    fn set_shared_type(&self, occ_type: u32) {
        self.filter_type.store(occ_type, Ordering::SeqCst);
    }

    /// The filter of a list / iteration typed thunk (§8.5, §9.4): type 0 is
    /// a null filter; any other type goes into the shared filter.
    fn typed_or_all(&self, occ_type: u32) -> Filter {
        if occ_type == 0 {
            Filter::All
        } else {
            self.set_shared_type(occ_type);
            Filter::Type(occ_type)
        }
    }

    /// Slot 28 `GetFirstOccupantByPosition(fx, fz, filter)`: the one world
    /// cell of slot 15, **not clamped** (§15 quirk 4; outside the grid:
    /// none); the filter, then `min.x ≤ fx < max.x` and `min.z ≤ fz <
    /// max.z` on the occupant's real box. The caller owns one reference.
    pub fn first_by_position(&self, fx: f32, fz: f32, filter: Filter) -> Option<u64> {
        let _g = self.lock();
        if !self.is_initialized() {
            return None;
        }
        let (x, z) = position_to_world_cell(fx, fz);
        let h = &*self.host;
        self.first_in(x..=x, z..=z, filter, &|o| {
            let b = h.bounding_box(o);
            b.min[0] <= fx && fx < b.max[0] && b.min[2] <= fz && fz < b.max[2]
        })
    }

    /// Slot 29: the typed thunk of slot 28 (type 0 literally, §10).
    pub fn first_by_position_typed(&self, fx: f32, fz: f32, occ_type: u32) -> Option<u64> {
        if !self.is_initialized() {
            return None;
        }
        self.set_shared_type(occ_type);
        self.first_by_position(fx, fz, Filter::Type(occ_type))
    }

    /// Slot 30 `GetFirstOccupant(x, z, filter)`: world cell (x, z), filter
    /// only; outside 0…191, none.
    pub fn first_in_cell(&self, x: i32, z: i32, filter: Filter) -> Option<u64> {
        let _g = self.lock();
        if !self.is_initialized() || !(0..WORLD_CELLS).contains(&x) || !(0..WORLD_CELLS).contains(&z) {
            return None;
        }
        self.first_in(x..=x, z..=z, filter, &|_| true)
    }

    /// Slot 31: the typed thunk of slot 30.
    pub fn first_in_cell_typed(&self, x: i32, z: i32, occ_type: u32) -> Option<u64> {
        if !self.is_initialized() {
            return None;
        }
        self.set_shared_type(occ_type);
        self.first_in_cell(x, z, Filter::Type(occ_type))
    }

    /// The x/z query box of standard cells `min`…`max` (§7, §8.4):
    /// `min·16 + 0.1 … (max + 1)·16 − 0.1`.
    fn standard_box(min: (i32, i32), max: (i32, i32)) -> (Vec3, Vec3) {
        let (ax, az) = standard_city_cell_to_position(min.0, min.1);
        let (bx, bz) = standard_city_cell_to_position(max.0.wrapping_add(1), max.1.wrapping_add(1));
        (
            [ax + SHRINK, -f32::MAX, az + SHRINK],
            [bx - SHRINK, f32::MAX, bz - SHRINK],
        )
    }

    /// The first occupant overlapping (strictly, on x and z) the standard
    /// cell box.
    fn first_in_standard_box(&self, qmin: Vec3, qmax: Vec3, filter: Filter) -> Option<u64> {
        let (x0, z0) = position_to_world_cell(qmin[0], qmin[2]);
        let (x1, z1) = position_to_world_cell(qmax[0], qmax[2]);
        let h = &*self.host;
        self.first_in(x0..=x1, z0..=z1, filter, &|o| {
            overlaps(&h.bounding_box(o), &qmin, &qmax, &[0, 2])
        })
    }

    /// Slot 32 `GetFirstOccupantByStandardCityCell(x, z, filter)`: the box
    /// `x·16 + 0.1 … (x + 1)·16 − 0.1` (same for z), strict overlap on x
    /// and z.
    pub fn first_by_standard_cell(&self, x: i32, z: i32, filter: Filter) -> Option<u64> {
        let _g = self.lock();
        if !self.is_initialized() {
            return None;
        }
        let (qmin, qmax) = Self::standard_box((x, z), (x, z));
        self.first_in_standard_box(qmin, qmax, filter)
    }

    /// Slot 33: the typed thunk of slot 32.
    pub fn first_by_standard_cell_typed(&self, x: i32, z: i32, occ_type: u32) -> Option<u64> {
        if !self.is_initialized() {
            return None;
        }
        self.set_shared_type(occ_type);
        self.first_by_standard_cell(x, z, Filter::Type(occ_type))
    }

    /// Slot 34 `GetFirstOccupantByStandardCityCells(min, max, filter)` (§7,
    /// OM5): a non-null corner is clamped to `0..=count − 1` on each axis;
    /// null min = (0, 0), null max = **(count, count)**, unclamped (one cell
    /// past the city edge, as slot 40: §15 quirk 17). The box's world cells
    /// (slot 15 of the corners, not clamped) are walked x outer, z inner.
    pub fn first_by_standard_cells(
        &self,
        min: Option<(i32, i32)>,
        max: Option<(i32, i32)>,
        filter: Filter,
    ) -> Option<u64> {
        let _g = self.lock();
        if !self.is_initialized() {
            return None;
        }
        let (qmin, qmax) = self.standard_cells_box(min, max);
        self.first_in_standard_box(qmin, qmax, filter)
    }

    /// Slot 35: the typed thunk of slot 34.
    pub fn first_by_standard_cells_typed(
        &self,
        min: Option<(i32, i32)>,
        max: Option<(i32, i32)>,
        occ_type: u32,
    ) -> Option<u64> {
        if !self.is_initialized() {
            return None;
        }
        self.set_shared_type(occ_type);
        self.first_by_standard_cells(min, max, Filter::Type(occ_type))
    }

    // ---- list queries (§8) ----

    /// The shared walk of slots 36 and 38 (§8.2 steps 4–7): cells x-major
    /// (already clamped), list order; the duplicate check (multi-cell only,
    /// skipped for single-cell nodes) before the filter, then `test`.
    /// Appends AddRef'd matches; stops at `max_count` appended.
    #[allow(clippy::too_many_arguments)]
    fn collect(
        &self,
        list: &mut Vec<u64>,
        (x0, z0): (i32, i32),
        (x1, z1): (i32, i32),
        filter: Filter,
        max_count: u32,
        test: &dyn Fn(u64) -> bool,
    ) {
        let multi = (x0, z0) != (x1, z1);
        let mut seen = HashSet::new();
        let mut count = 0u32;
        for x in x0..=x1 {
            for z in z0..=z1 {
                let Some(nodes) = self.cell_nodes(x, z) else {
                    continue;
                };
                for n in nodes {
                    if multi && !n.single && !seen.insert(n.occupant) {
                        continue;
                    }
                    if !self.includes(filter, n.occupant) || !test(n.occupant) {
                        continue;
                    }
                    self.host.add_ref(n.occupant);
                    list.push(n.occupant);
                    count += 1;
                    if count >= max_count {
                        return;
                    }
                }
            }
        }
    }

    /// Slot 36 `GetOccupantsByBBox(list, min, max, filter, maxCount)`
    /// (§8.2). Null min = (0, −FLT_MAX, 0), null max = (size X, +FLT_MAX,
    /// size Z); cells of slot 15, clamped; strict overlap on x, y and z.
    /// Does not clear `list`. False (list untouched) when not initialised
    /// or `max_count` is 0; true otherwise, matches or not. `max_count` is
    /// unsigned: −1 is "all".
    pub fn occupants_by_bbox(
        &self,
        list: &mut Vec<u64>,
        min: Option<Vec3>,
        max: Option<Vec3>,
        filter: Filter,
        max_count: i32,
    ) -> bool {
        let _g = self.lock();
        if !self.is_initialized() || max_count == 0 {
            return false;
        }
        let qmin = min.unwrap_or([0.0, -f32::MAX, 0.0]);
        let qmax = max.unwrap_or([self.size_of(0), f32::MAX, self.size_of(1)]);
        let (a, b) = Self::bbox_cells(&qmin, &qmax);
        let h = &*self.host;
        self.collect(list, a, b, filter, max_count as u32, &|o| {
            overlaps(&h.bounding_box(o), &qmin, &qmax, &[0, 1, 2])
        });
        true
    }

    /// The clamped world cells of a box's corners (§8.2 step 3).
    fn bbox_cells(qmin: &Vec3, qmax: &Vec3) -> ((i32, i32), (i32, i32)) {
        let (x0, z0) = position_to_world_cell(qmin[0], qmin[2]);
        let (x1, z1) = position_to_world_cell(qmax[0], qmax[2]);
        ((clamp_cell(x0), clamp_cell(z0)), (clamp_cell(x1), clamp_cell(z1)))
    }

    /// Slot 37: the typed thunk of slot 36 (type 0 = null filter).
    pub fn occupants_by_bbox_typed(
        &self,
        list: &mut Vec<u64>,
        min: Option<Vec3>,
        max: Option<Vec3>,
        occ_type: u32,
        max_count: i32,
    ) -> bool {
        if !self.is_initialized() {
            return false;
        }
        let f = self.typed_or_all(occ_type);
        self.occupants_by_bbox(list, min, max, f, max_count)
    }

    /// The world-cell range of slots 38 and 44: null min = (64, 64), null
    /// max = the city's last cells; clamped to 0…191, inclusive.
    fn city_cells(&self, min: Option<(i32, i32)>, max: Option<(i32, i32)>) -> ((i32, i32), (i32, i32)) {
        let (_, _, bx, bz) = self.cell_bounds();
        let (x0, z0) = min.unwrap_or((CITY_ORIGIN_CELL, CITY_ORIGIN_CELL));
        let (x1, z1) = max.unwrap_or((bx, bz));
        ((clamp_cell(x0), clamp_cell(z0)), (clamp_cell(x1), clamp_cell(z1)))
    }

    /// Slot 38 `GetOccupantsByOccupantManagerCells(list, min, max, filter,
    /// maxCount)` (§8.3): **world** cells, inclusive; filter and duplicate
    /// check only.
    pub fn occupants_by_cells(
        &self,
        list: &mut Vec<u64>,
        min: Option<(i32, i32)>,
        max: Option<(i32, i32)>,
        filter: Filter,
        max_count: i32,
    ) -> bool {
        let _g = self.lock();
        if !self.is_initialized() || max_count == 0 {
            return false;
        }
        let (a, b) = self.city_cells(min, max);
        self.collect(list, a, b, filter, max_count as u32, &|_| true);
        true
    }

    /// Slot 39: the typed thunk of slot 38.
    pub fn occupants_by_cells_typed(
        &self,
        list: &mut Vec<u64>,
        min: Option<(i32, i32)>,
        max: Option<(i32, i32)>,
        occ_type: u32,
        max_count: i32,
    ) -> bool {
        if !self.is_initialized() {
            return false;
        }
        let f = self.typed_or_all(occ_type);
        self.occupants_by_cells(list, min, max, f, max_count)
    }

    /// The box of slots 34, 40 and 47 (§7, §8.4): min null → (0, 0), else
    /// clamped to 0…count − 1; max null → **(count, count)** (§15 quirks 8
    /// and 17), else clamped to 0…count − 1; y open.
    fn standard_cells_box(&self, min: Option<(i32, i32)>, max: Option<(i32, i32)>) -> (Vec3, Vec3) {
        let (cx, cz) = self.standard_cell_count();
        let clamp = |p: (i32, i32)| {
            (
                p.0.clamp(0, cx.wrapping_sub(1).max(0)),
                p.1.clamp(0, cz.wrapping_sub(1).max(0)),
            )
        };
        let min = min.map_or((0, 0), clamp);
        let max = max.map_or((cx, cz), clamp);
        Self::standard_box(min, max)
    }

    /// Slot 40 `GetOccupantsByStandardCityCells(list, min, max, filter,
    /// maxCount)` (§8.4): no lock of its own; the box of standard cells,
    /// then slot 36. An occupant is found when its box reaches more than
    /// 0.1 m into the rectangle.
    pub fn occupants_by_standard_cells(
        &self,
        list: &mut Vec<u64>,
        min: Option<(i32, i32)>,
        max: Option<(i32, i32)>,
        filter: Filter,
        max_count: i32,
    ) -> bool {
        if !self.is_initialized() {
            return false;
        }
        let (qmin, qmax) = self.standard_cells_box(min, max);
        self.occupants_by_bbox(list, Some(qmin), Some(qmax), filter, max_count)
    }

    /// Slot 41: the typed thunk of slot 40.
    pub fn occupants_by_standard_cells_typed(
        &self,
        list: &mut Vec<u64>,
        min: Option<(i32, i32)>,
        max: Option<(i32, i32)>,
        occ_type: u32,
        max_count: i32,
    ) -> bool {
        if !self.is_initialized() {
            return false;
        }
        let f = self.typed_or_all(occ_type);
        self.occupants_by_standard_cells(list, min, max, f, max_count)
    }

    // ---- iteration (§9) ----

    /// Slot 42 `IterateOccupantsByBBox(fn, data, min, max, filter)` (§9.1).
    /// Not initialised: true, nothing done (§15 quirk 10). Per cell
    /// (x-major): under the lock, collect the matches (duplicate check for
    /// multi-cell walks, filter, strict overlap on x, y, z), each AddRef'd;
    /// unlocked, call `f` on each while it keeps returning true and release
    /// each. True.
    pub fn iterate_by_bbox(&self, f: &mut dyn FnMut(u64) -> bool, min: Vec3, max: Vec3, filter: Filter) -> bool {
        if !self.is_initialized() {
            return true;
        }
        let h = &*self.host;
        let ((x0, z0), (x1, z1)) = Self::bbox_cells(&min, &max);
        let multi = (x0, z0) != (x1, z1);
        let mut seen = HashSet::new();
        for x in x0..=x1 {
            for z in z0..=z1 {
                let g = self.lock();
                let mut batch = Vec::new();
                if let Some(nodes) = self.cell_nodes(x, z) {
                    for n in nodes {
                        if multi && !n.single && !seen.insert(n.occupant) {
                            continue;
                        }
                        if self.includes(filter, n.occupant)
                            && overlaps(&h.bounding_box(n.occupant), &min, &max, &[0, 1, 2])
                        {
                            h.add_ref(n.occupant);
                            batch.push(n.occupant);
                        }
                    }
                }
                drop(g);
                if !self.run_batch(f, batch) {
                    return true;
                }
            }
        }
        true
    }

    /// The callbacks of one batch (§9.1 step 5.4, §9.2 steps 3.4 and 4.4):
    /// `f` for each occupant while it keeps returning true; every occupant
    /// is released (the batch's reference) whether it was called or not.
    /// False when `f` stopped the walk.
    fn run_batch(&self, f: &mut dyn FnMut(u64) -> bool, batch: Vec<u64>) -> bool {
        let mut go = true;
        for o in batch {
            if go {
                go = f(o);
            }
            self.host.release(o);
        }
        go
    }

    /// Slot 43: the typed thunk of slot 42 (not initialised: false; type 0
    /// = null filter).
    pub fn iterate_by_bbox_typed(&self, f: &mut dyn FnMut(u64) -> bool, min: Vec3, max: Vec3, occ_type: u32) -> bool {
        if !self.is_initialized() {
            return false;
        }
        let filter = self.typed_or_all(occ_type);
        self.iterate_by_bbox(f, min, max, filter)
    }

    /// Slot 44 `IterateOccupants(fn, data, min, max, filter)` (§9.2, OM7):
    /// `min` and `max` are three-int vectors (x = element 0, z = element 2;
    /// element 1 is ignored, §15 quirk 9), world cells, inclusive, clamped;
    /// null = the city's cells. No geometry test.
    ///
    /// The **outer** lock is held for the whole walk, callbacks included.
    /// Each cell is collected under an **inner** hold into a batch (each
    /// match AddRef'd), the inner hold is dropped, then the callbacks run
    /// (slot 42's structure, inside the outer lock). A callback may
    /// therefore insert, remove or move occupants, including the one it was
    /// given (the batch holds a reference); changes to cells not yet visited
    /// are seen by the walk.
    /// - Single-cell walk (min cell == max cell): every node passing the
    ///   filter, in list order, with **no** duplicate check.
    /// - Multi-cell walk, x outer, z inner: per node the duplicate check
    ///   against a local set, skipped for single-cell (bit-0) nodes, then the
    ///   filter.
    ///
    /// True, initialised or not; an uninitialised manager walks no cells.
    pub fn iterate(
        &self,
        f: &mut dyn FnMut(u64) -> bool,
        min: Option<[i32; 3]>,
        max: Option<[i32; 3]>,
        filter: Filter,
    ) -> bool {
        let _outer = self.lock();
        if !self.is_initialized() {
            return true;
        }
        let ((x0, z0), (x1, z1)) = self.city_cells(min.map(|v| (v[0], v[2])), max.map(|v| (v[0], v[2])));
        if (x0, z0) == (x1, z1) {
            let batch = self.batch_of(x0, z0, filter, None);
            self.run_batch(f, batch);
            return true;
        }
        let mut seen = HashSet::new();
        for x in x0..=x1 {
            for z in z0..=z1 {
                let batch = self.batch_of(x, z, filter, Some(&mut seen));
                if !self.run_batch(f, batch) {
                    return true;
                }
            }
        }
        true
    }

    /// One cell's batch of slot 44 (§9.2 steps 3.1–3.3 and 4.1–4.3), under
    /// the inner hold: in list order, the duplicate check against `seen`
    /// when given (skipped for bit-0 nodes), then the filter; each match
    /// AddRef'd.
    fn batch_of(&self, x: i32, z: i32, filter: Filter, mut seen: Option<&mut HashSet<u64>>) -> Vec<u64> {
        let _inner = self.lock();
        let mut batch = Vec::new();
        for n in self.cell_nodes(x, z).unwrap_or_default() {
            if let Some(seen) = seen.as_deref_mut() {
                if !n.single && !seen.insert(n.occupant) {
                    continue;
                }
            }
            if self.includes(filter, n.occupant) {
                self.host.add_ref(n.occupant);
                batch.push(n.occupant);
            }
        }
        batch
    }

    /// Slot 45: the typed thunk of slot 44 (not initialised: false; type 0
    /// = null filter).
    pub fn iterate_typed(
        &self,
        f: &mut dyn FnMut(u64) -> bool,
        min: Option<[i32; 3]>,
        max: Option<[i32; 3]>,
        occ_type: u32,
    ) -> bool {
        if !self.is_initialized() {
            return false;
        }
        let filter = self.typed_or_all(occ_type);
        self.iterate(f, min, max, filter)
    }

    /// Slot 47 `IterateOccupantsByStandardCityCell(fn, data, min, max,
    /// filter)` (§9.3), the **filter** overload (§15 quirk 12): not
    /// initialised → false; the box of slot 40 (null-max quirk included),
    /// then slot 42.
    pub fn iterate_by_standard_cells(
        &self,
        f: &mut dyn FnMut(u64) -> bool,
        min: Option<(i32, i32)>,
        max: Option<(i32, i32)>,
        filter: Filter,
    ) -> bool {
        if !self.is_initialized() {
            return false;
        }
        let (qmin, qmax) = self.standard_cells_box(min, max);
        self.iterate_by_bbox(f, qmin, qmax, filter)
    }

    /// Slot 46: the **typed** overload of slot 47 (it comes first in the
    /// vtable, §9.4).
    pub fn iterate_by_standard_cells_typed(
        &self,
        f: &mut dyn FnMut(u64) -> bool,
        min: Option<(i32, i32)>,
        max: Option<(i32, i32)>,
        occ_type: u32,
    ) -> bool {
        if !self.is_initialized() {
            return false;
        }
        let filter = self.typed_or_all(occ_type);
        self.iterate_by_standard_cells(f, min, max, filter)
    }

    /// Slot 48 `ReleaseOccupantList(list)` (§9.5): releases every element
    /// once and empties the list. No lock.
    pub fn release_occupant_list(&self, list: &mut Vec<u64>) {
        for o in list.drain(..) {
            self.host.release(o);
        }
    }

    // ---- conversions with this object's state ----

    /// Slot 10 on this object (§3.2).
    pub fn world_to_manager_cell(&self, x: i32, z: i32) -> (i32, i32) {
        world_cell_to_occupant_manager_cell(x, z)
    }

    /// Slot 14 on this object.
    pub fn world_cell_position(&self, x: i32, z: i32) -> (f32, f32) {
        world_cell_to_position(x, z)
    }

    /// Slot 17 on this object.
    pub fn position_to_manager_cell(&self, fx: f32, fz: f32) -> (i32, i32) {
        position_to_occupant_manager_cell(fx, fz)
    }

    /// Slot 19 on this object.
    pub fn position_to_standard_cell(&self, fx: f32, fz: f32) -> (i32, i32) {
        position_to_standard_city_cell(fx, fz)
    }

    // ---- the record (§13) ----

    fn parse(&self, inp: &mut dyn InStream, kept: &mut Vec<u64>) -> Option<Parsed> {
        if inp.get_u16()? != RECORD_VERSION {
            return None;
        }
        let fx = inp.get_f32()?;
        let fz = inp.get_f32()?;
        let mut counts = [0i32; 4];
        for c in &mut counts {
            *c = inp.get_i32()?;
        }
        let rows = inp.get_u32()?;
        let mut grid = Vec::new();
        let mut seen = HashSet::new();
        for _ in 0..rows {
            let cells = inp.get_u32()?;
            let mut row = Vec::new();
            for _ in 0..cells {
                let n = inp.get_u32()?;
                if n == 0 {
                    row.push(None);
                    continue;
                }
                let mut list = VecDeque::new();
                for _ in 0..n {
                    let r = inp.get_ref_as(IID_OCCUPANT)?;
                    let p = if r.is_null() {
                        None
                    } else {
                        self.host.resolve_ref(&r, IID_OCCUPANT)
                    };
                    let Some(p) = p else {
                        continue;
                    };
                    let single = self.host.manager_bbox(p).is_some_and(|b| b.is_single());
                    list.push_back(Node { occupant: p, single });
                    if !single && !seen.insert(p) {
                        // One manager reference per occupant (§13 step 4.4).
                        self.host.release(p);
                    } else {
                        kept.push(p);
                    }
                }
                row.push(Some(list));
            }
            grid.push(row);
        }
        if inp.error() != 0 {
            return None;
        }
        Some((fx, fz, counts, grid))
    }
}

impl SaveObject for OccupantManager {
    fn class_id(&self) -> u32 {
        CLSID_OCCUPANT_MANAGER
    }

    /// §13: version 1, the sizes and counts, then every row, cell and
    /// occupant reference in list order (an occupant spanning k cells is
    /// written k times). True unless the stream reports an error. The
    /// original takes no lock (OM8); [choice] the core does (see slot 25).
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let _g = self.lock();
        out.put_u16(RECORD_VERSION);
        out.put_f32(self.size_of(0));
        out.put_f32(self.size_of(1));
        for i in 0..4 {
            out.put_i32(self.count(i));
        }
        let grid: Vec<Vec<Vec<u64>>> = self
            .s
            .borrow()
            .grid
            .iter()
            .map(|row| {
                row.iter()
                    .map(|c| {
                        c.as_ref()
                            .map_or(Vec::new(), |l| l.iter().map(|n| n.occupant).collect())
                    })
                    .collect()
            })
            .collect();
        out.put_u32(grid.len() as u32);
        for row in &grid {
            out.put_u32(row.len() as u32);
            for cell in row {
                out.put_u32(cell.len() as u32);
                for &o in cell {
                    out.put_ref(&self.host.object_ref(o));
                }
            }
        }
        out.error() == 0
    }

    /// §13: sets "was read"; false unless the version is 1. Sizes and counts
    /// are read raw; each cell with a count gets a holder, its occupants
    /// appended at the back (the saved order); one manager reference per
    /// occupant. [choice] Atomic (see the module notes). The original takes
    /// no lock (OM8); [choice] the core does (see slot 25).
    fn read(&self, inp: &mut dyn InStream) -> bool {
        let _g = self.lock();
        self.s.borrow_mut().was_read = true;
        let mut kept = Vec::new();
        match self.parse(inp, &mut kept) {
            Some((fx, fz, counts, grid)) => {
                self.store_sizes(fx, fz, counts);
                self.s.borrow_mut().grid = grid;
                true
            }
            None => {
                for o in kept {
                    self.host.release(o);
                }
                false
            }
        }
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}
