// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! §2.7 ordinary allocation-success domain. Identities are logical, never addresses.
//! Explicit destruction emits native storage events; Rust Drop only provides safety.
use std::ops::Range;

pub type Identity = u64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Vertices,
    Rectangles,
    Points,
    Node,
    Sentinel,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Allocate(Identity, Kind, usize),
    Free(Identity),
}
#[derive(Default, Debug)]
pub struct Ledger {
    next: Identity,
    pub events: Vec<Event>,
}
impl Ledger {
    fn allocate(&mut self, kind: Kind, bytes: usize) -> Identity {
        self.next += 1;
        self.events.push(Event::Allocate(self.next, kind, bytes));
        self.next
    }
    fn free(&mut self, id: Option<Identity>) {
        if let Some(id) = id {
            self.events.push(Event::Free(id));
        }
    }
}

pub enum Assignment<'a, T> {
    This,
    Other(&'a T),
}

#[derive(Debug)]
pub struct Buffer<const N: usize> {
    pub id: Option<Identity>,
    pub capacity: usize,
    pub records: Vec<[u32; N]>,
    kind: Kind,
}
impl<const N: usize> Buffer<N> {
    pub fn new(kind: Kind, records: &[[u32; N]], ledger: &mut Ledger) -> Self {
        Self {
            id: (!records.is_empty()).then(|| ledger.allocate(kind, records.len() * N * 4)),
            capacity: records.len(),
            records: records.to_vec(),
            kind,
        }
    }
    pub fn copy(&self, ledger: &mut Ledger) -> Self {
        Self::new(self.kind, &self.records, ledger)
    }
    pub fn assign(&mut self, source: Assignment<'_, Self>, ledger: &mut Ledger) {
        let Assignment::Other(source) = source else {
            return;
        };
        if source.records.len() > self.capacity {
            let fresh = ledger.allocate(self.kind, source.records.len() * N * 4);
            let records = source.records.clone();
            ledger.free(self.id);
            self.id = Some(fresh);
            self.capacity = records.len();
            self.records = records;
        } else {
            self.records.clone_from(&source.records);
        }
    }
    fn destroy(&mut self, ledger: &mut Ledger) {
        ledger.free(self.id.take());
    }
}

#[derive(Debug)]
pub struct Strip {
    pub vertices: Buffer<6>,
    /// Literal +0C..+27 and +34..+4B words, no inferred pointer semantics.
    pub words: [u32; 13],
    pub rectangles: Buffer<4>,
}
impl Strip {
    pub fn copy(&self, ledger: &mut Ledger) -> Self {
        Self {
            vertices: self.vertices.copy(ledger),
            words: self.words,
            rectangles: self.rectangles.copy(ledger),
        }
    }
    fn assign(&mut self, source: &Self, ledger: &mut Ledger) {
        self.vertices.assign(Assignment::Other(&source.vertices), ledger);
        self.rectangles.assign(Assignment::Other(&source.rectangles), ledger);
        self.words = source.words;
    }
    fn destroy(&mut self, ledger: &mut Ledger) {
        self.rectangles.destroy(ledger);
        self.vertices.destroy(ledger);
    }
}

#[derive(Debug)]
pub struct Node {
    pub id: Identity,
    pub strip: Strip,
}
#[derive(Debug)]
pub struct StripList {
    pub sentinel: Identity,
    pub nodes: Vec<Node>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    InvalidRange,
    EmptyPop,
    InvalidDestination,
}
impl StripList {
    #[cfg(test)]
    pub(super) fn push_owned(&mut self, strip: Strip, ledger: &mut Ledger) {
        let id = ledger.allocate(Kind::Node, 0x54);
        self.nodes.push(Node { id, strip });
    }
    pub fn new(ledger: &mut Ledger) -> Self {
        Self {
            sentinel: ledger.allocate(Kind::Sentinel, 0x54),
            nodes: Vec::new(),
        }
    }
    /// Distinct, independently owned source; no universal self-insertion contract.
    pub fn insert_range(
        &mut self,
        position: usize,
        source: &Self,
        range: Range<usize>,
        ledger: &mut Ledger,
    ) -> Result<(), Unsupported> {
        if position > self.nodes.len() {
            return Err(Unsupported::InvalidRange);
        }
        let source = source.nodes.get(range).ok_or(Unsupported::InvalidRange)?;
        for (offset, node) in source.iter().enumerate() {
            // Allocate node before its payload, link only after copy construction.
            let id = ledger.allocate(Kind::Node, 0x54);
            let strip = node.strip.copy(ledger);
            self.nodes.insert(position + offset, Node { id, strip });
        }
        Ok(())
    }
    /// Writes the logical end iterator even for an empty range.
    pub fn erase(
        &mut self,
        range: Range<usize>,
        output: &mut Identity,
        ledger: &mut Ledger,
    ) -> Result<(), Unsupported> {
        if range.start > range.end || range.end > self.nodes.len() {
            return Err(Unsupported::InvalidRange);
        }
        let end = self.nodes.get(range.end).map_or(self.sentinel, |n| n.id);
        for mut node in self.nodes.drain(range) {
            node.strip.destroy(ledger);
            ledger.free(Some(node.id));
        }
        *output = end;
        Ok(())
    }
    pub fn clear(&mut self, ledger: &mut Ledger) {
        let mut end = self.sentinel;
        self.erase(0..self.nodes.len(), &mut end, ledger)
            .expect("valid list range");
    }
    pub fn remove_last(&mut self, ledger: &mut Ledger) -> Result<(), Unsupported> {
        let index = self.nodes.len().checked_sub(1).ok_or(Unsupported::EmptyPop)?;
        let mut output = self.sentinel;
        self.erase(index..index + 1, &mut output, ledger)
    }
    pub fn assign(&mut self, source: Assignment<'_, Self>, ledger: &mut Ledger) {
        let Assignment::Other(source) = source else {
            return;
        };
        let common = self.nodes.len().min(source.nodes.len());
        for (dest, src) in self.nodes.iter_mut().zip(&source.nodes) {
            dest.strip.assign(&src.strip, ledger);
        }
        if self.nodes.len() > common {
            let mut end = self.sentinel;
            self.erase(common..self.nodes.len(), &mut end, ledger)
                .expect("valid list range");
        } else {
            self.insert_range(common, source, common..source.nodes.len(), ledger)
                .expect("distinct list range");
        }
    }
    fn destroy(&mut self, ledger: &mut Ledger) {
        self.clear(ledger);
        ledger.free(Some(self.sentinel));
    }
}

#[derive(Debug)]
pub struct Shore {
    pub points: Buffer<2>,
    pub strips: StripList,
    pub words: [u32; 10],
}
impl Shore {
    pub fn copy(&self, ledger: &mut Ledger) -> Self {
        let points = self.points.copy(ledger);
        let mut strips = StripList::new(ledger);
        strips
            .insert_range(0, &self.strips, 0..self.strips.nodes.len(), ledger)
            .expect("distinct list range");
        Self {
            points,
            strips,
            words: self.words,
        }
    }
    pub fn assign(&mut self, source: Assignment<'_, Self>, ledger: &mut Ledger) {
        let Assignment::Other(source) = source else {
            return;
        };
        self.points.assign(Assignment::Other(&source.points), ledger);
        self.strips.assign(Assignment::Other(&source.strips), ledger);
        self.words = source.words;
    }
    /// Consume the live descriptor: repeated destruction is outside the contract.
    pub fn destroy(mut self, ledger: &mut Ledger) {
        self.strips.destroy(ledger);
        self.points.destroy(ledger);
    }
}

/// §2.7 range assignment, not vector replacement: advance live destination.
pub fn assign_shores(
    source: &[Shore],
    destination: &mut [Shore],
    start: usize,
    ledger: &mut Ledger,
) -> Result<usize, Unsupported> {
    let end = start.checked_add(source.len()).ok_or(Unsupported::InvalidDestination)?;
    let destination = destination.get_mut(start..end).ok_or(Unsupported::InvalidDestination)?;
    for (dest, src) in destination.iter_mut().zip(source) {
        dest.assign(Assignment::Other(src), ledger);
    }
    Ok(end)
}
