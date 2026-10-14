// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Search nodes, the node store and the open set (§4.7).

/// Nodes per page of the node store (§4.7). FreeCity4 keeps the nodes in one
/// vector; allocation order is index order, as the pages are filled in order.
pub const PAGE_NODES: usize = 4096;
/// Buckets of the node hash table (§4.7).
pub const BUCKETS: usize = 3079;
/// The node hash multiplier (§4.7): bucket = `(key × 0x9E3779B1 mod 2^32)
/// mod 3079`.
pub const HASH_MULTIPLIER: u32 = 0x9E37_79B1;
/// Bit 31 of an exit node's key (§4.7).
pub const EXIT_KEY_BIT: u32 = 0x8000_0000;
/// The open set's initial capacity (§4.7: a vector of 100 entries, entry 0
/// unused).
const OPEN_CAPACITY: usize = 100;
/// "No node" in the chains and heap positions.
const NIL: u32 = u32::MAX;

/// A node key (§4.7): `(prefix << 24) | (t << 16) | (x & 0xFF) << 8 |
/// (z & 0xFF)`, where `prefix` is `pSide << 2 | side` for an ordinary node and
/// `0x10 | side` for a start node. Exit nodes add [`EXIT_KEY_BIT`].
pub fn node_key(prefix: u32, t: u32, x: i32, z: i32) -> u32 {
    (prefix << 24) | (t << 16) | ((x as u32 & 0xFF) << 8) | (z as u32 & 0xFF)
}

/// The cell a new node takes from its key (§4.7, `0x006D5DF0`): `x = (key >>
/// 8) & 0xFF` and `z = key & 0xFF` zero-extended, and for an exit node (bit
/// 31) plus `dir(side)` (`side = (key >> 24) & 3`) added as a 16-bit value.
/// For every node the game makes this is the true cell: in-city cells fit a
/// byte, and an exit node's in-city cell is adjacent to its off-map cell, so
/// `0 − 1 = −1` and `255 + 1 = 256` come out right (PF15). Only the
/// unreachable off-map park-and-ride target (§4.8.4) decodes differently
/// from its passed cell *[quirk]*.
pub fn key_cell(key: u32) -> (i32, i32) {
    let x = ((key >> 8) & 0xFF) as u16;
    let z = (key & 0xFF) as u16;
    if key & EXIT_KEY_BIT == 0 {
        return (i32::from(x), i32::from(z));
    }
    let side = ((key >> 24) & 3) as usize;
    let dx = crate::traffic_network::DX[side] as u16;
    let dz = crate::traffic_network::DZ[side] as u16;
    (
        i32::from(x.wrapping_add(dx) as i16),
        i32::from(z.wrapping_add(dz) as i16),
    )
}

/// A search node: a (cell, travel type, entry side, predecessor's entry side)
/// state (§4.7).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    /// The identity key (§4.7).
    pub key: u32,
    /// The node's cell; for an exit node the off-map cell.
    pub x: i32,
    pub z: i32,
    /// The travel type 0…8.
    pub t: u8,
    /// The entry side (for an exit node: the direction of the move).
    pub side: u8,
    /// The predecessor (an index into the store).
    pub parent: Option<u32>,
    /// The cost so far.
    pub g: f32,
    /// The score `f` the open set orders by.
    pub f: f32,
    pub open: bool,
    pub closed: bool,
    /// The heap position of the node's latest open-set entry.
    pub(crate) heap_pos: u32,
    /// The next node of the hash chain.
    pub(crate) next: u32,
}

impl Node {
    /// A new node: neither open nor closed (§4.7).
    pub fn new(key: u32, cell: (i32, i32), t: u8, side: u8) -> Node {
        Node {
            key,
            x: cell.0,
            z: cell.1,
            t,
            side,
            parent: None,
            g: 0.0,
            f: 0.0,
            open: false,
            closed: false,
            heap_pos: NIL,
            next: NIL,
        }
    }
}

/// The node store (§4.7): the nodes in allocation order and a chained hash
/// table of 3079 buckets, new nodes at the head of their chain.
#[derive(Clone, Debug)]
pub struct NodeStore {
    pub(crate) nodes: Vec<Node>,
    heads: Vec<u32>,
}

impl Default for NodeStore {
    fn default() -> Self {
        NodeStore::new()
    }
}

impl NodeStore {
    /// An empty store: one page, 3079 empty buckets (§1).
    pub fn new() -> NodeStore {
        NodeStore {
            nodes: Vec::with_capacity(PAGE_NODES),
            heads: vec![NIL; BUCKETS],
        }
    }

    /// `ReleasePathNodes` (§4.18): the page cursor back to the first page
    /// (storage kept) and every bucket emptied.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.heads.fill(NIL);
    }

    /// The bucket of a key.
    pub fn bucket(key: u32) -> usize {
        (key.wrapping_mul(HASH_MULTIPLIER) % BUCKETS as u32) as usize
    }

    /// The number of stored nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The nodes in allocation order.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// The node with an index.
    pub fn get(&self, id: u32) -> &Node {
        &self.nodes[id as usize]
    }

    /// The node with a key, if stored.
    pub fn find(&self, key: u32) -> Option<u32> {
        let mut id = self.heads[Self::bucket(key)];
        while id != NIL {
            let n = &self.nodes[id as usize];
            if n.key == key {
                return Some(id);
            }
            id = n.next;
        }
        None
    }

    /// The node with a key; a new one (made by `make`) is inserted at the head
    /// of its chain. Returns the index and whether it is new.
    pub fn find_or_create(&mut self, key: u32, make: impl FnOnce() -> Node) -> (u32, bool) {
        if let Some(id) = self.find(key) {
            return (id, false);
        }
        let b = Self::bucket(key);
        let id = self.nodes.len() as u32;
        let mut n = make();
        n.key = key;
        n.next = self.heads[b];
        n.heap_pos = NIL;
        self.nodes.push(n);
        self.heads[b] = id;
        (id, true)
    }

    /// The stored nodes visited bucket by bucket (0…3078), each chain from
    /// its head (§4.12).
    pub fn bucket_order(&self) -> Vec<u32> {
        let mut out = Vec::with_capacity(self.nodes.len());
        for &head in &self.heads {
            let mut id = head;
            while id != NIL {
                out.push(id);
                id = self.nodes[id as usize].next;
            }
        }
        out
    }
}

/// The open set: a 1-indexed binary min-heap on `f` (§4.7) *[compat]*. The
/// entries are node indices; entry 0 is unused.
#[derive(Clone, Debug)]
pub struct OpenSet {
    heap: Vec<u32>,
}

impl Default for OpenSet {
    fn default() -> Self {
        OpenSet::new()
    }
}

impl OpenSet {
    /// An empty open set (capacity 99, §1).
    pub fn new() -> OpenSet {
        let mut heap = Vec::with_capacity(OPEN_CAPACITY);
        heap.push(NIL);
        OpenSet { heap }
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.heap.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Empties the open set (`ReleasePathNodes`).
    pub fn clear(&mut self) {
        self.heap.truncate(1);
    }

    /// The entries in heap order (position 1 first).
    pub fn entries(&self) -> &[u32] {
        &self.heap[1..]
    }

    fn place(&mut self, nodes: &mut [Node], pos: usize, id: u32) {
        self.heap[pos] = id;
        nodes[id as usize].heap_pos = pos as u32;
    }

    /// Sift up from `pos` with the push rule: while the parent's `f` is
    /// strictly greater, move the parent down (equal or NaN stops).
    fn sift_up(&mut self, nodes: &mut [Node], mut pos: usize) {
        let id = self.heap[pos];
        let f = nodes[id as usize].f;
        while pos > 1 {
            let parent = pos / 2;
            let pid = self.heap[parent];
            if nodes[pid as usize].f > f {
                self.place(nodes, pos, pid);
                pos = parent;
            } else {
                break;
            }
        }
        self.place(nodes, pos, id);
    }

    /// Push: append at `count + 1` and sift up.
    pub fn push(&mut self, nodes: &mut [Node], id: u32) {
        self.heap.push(id);
        let pos = self.heap.len() - 1;
        self.sift_up(nodes, pos);
    }

    /// Pop: take position 1; move the last entry there and sift down. At
    /// position `i`, choose the left child if its `f` is strictly less than
    /// the moving entry's, then the right child if its `f` is strictly less
    /// than the chosen one's; stop if neither; continue while the new
    /// position is below `count`.
    pub fn pop(&mut self, nodes: &mut [Node]) -> Option<u32> {
        if self.is_empty() {
            return None;
        }
        let top = self.heap[1];
        let last = self.heap.pop().unwrap_or(NIL);
        let count = self.len();
        if count == 0 {
            return Some(top);
        }
        let fm = nodes[last as usize].f;
        let mut i = 1;
        loop {
            let (l, r) = (2 * i, 2 * i + 1);
            let mut chosen = None;
            let mut cf = fm;
            if l <= count && nodes[self.heap[l] as usize].f < cf {
                chosen = Some(l);
                cf = nodes[self.heap[l] as usize].f;
            }
            if r <= count && nodes[self.heap[r] as usize].f < cf {
                chosen = Some(r);
            }
            let Some(c) = chosen else {
                break;
            };
            let cid = self.heap[c];
            self.place(nodes, i, cid);
            i = c;
            if i >= count {
                break;
            }
        }
        self.place(nodes, i, last);
        Some(top)
    }

    /// Update (§4.7): a node already open got a lower `f`; sift up from its
    /// recorded position (only up). A node whose recorded position no longer
    /// holds it (a popped node) is left alone *[choice]* (PF9, §4.7
    /// "Position": the original sifts from that slot unchecked and overwrites
    /// whatever it holds, but in the game an open node's recorded slot always
    /// holds it). A node in the heap twice records the slot of the copy
    /// written last, and only that copy moves, as in the original.
    pub fn update(&mut self, nodes: &mut [Node], id: u32) {
        let pos = nodes[id as usize].heap_pos as usize;
        if pos >= 1 && pos <= self.len() && self.heap[pos] == id {
            self.sift_up(nodes, pos);
        }
    }
}

/// Pushes nodes with the given `f` values in order into an empty open set
/// and pops them all; returns the indices in pop order (§11 test 1).
pub fn heap_order(fs: &[f32]) -> Vec<usize> {
    let mut nodes: Vec<Node> = fs
        .iter()
        .enumerate()
        .map(|(i, &f)| {
            let mut n = Node::new(i as u32, (0, 0), 0, 0);
            n.f = f;
            n
        })
        .collect();
    let mut open = OpenSet::new();
    for i in 0..nodes.len() {
        open.push(&mut nodes, i as u32);
    }
    let mut out = Vec::new();
    while let Some(id) = open.pop(&mut nodes) {
        out.push(id as usize);
    }
    out
}
