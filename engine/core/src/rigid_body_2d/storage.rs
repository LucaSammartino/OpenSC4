// SPDX-License-Identifier: GPL-3.0-or-later
//! §7.1 logical raw-address list storage, NOT an allocator or COM owner.
//!
//! Records are supplied complete by the caller, including sentinel roots. Root
//! and next/previous links must be valid, mutually linked, stable and disjoint
//! between lists. Foreign frees and sector removal must return normally without
//! reentrancy. Raw identities never become host pointers. Nodes are forgotten
//! only after the corresponding foreign deallocator call returns. No allocation,
//! reference acquisition, destructor or registration policy is supplied here.
use super::{Result, Unsupported};
use std::collections::{BTreeMap, BTreeSet};

pub type Address = u32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub next: Address,
    pub previous: Address,
    /// node+7E; for object nodes this is body+76, body=node+08.
    pub live: u8,
    /// Byte at node-1, NOT a body or reference count.
    pub allocator_class: u8,
    /// Raw node+08,+0C words in the pair-like list, not owned COM references.
    pub pair: [Address; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Allocator {
    Class(u8),
    Global,
}

/// Only specifically published foreign scalar/lifetime boundaries. The host
/// supplies no sweep, unlink, collision, registration, or integration answer.
pub trait Foreign {
    fn free_allocation(&mut self, allocation: Address, allocator: Allocator);
    fn free_pair_node(&mut self, node: Address);
    /// Unproved RB2 internals; this bounded operation composes only the call.
    fn remove_from_sector_map(&mut self, body: Address);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Sweep(Address),
    SetNext { node: Address, next: Address },
    SetPrevious { node: Address, previous: Address },
    FreeAllocation { allocation: Address, allocator: Allocator },
    FreePair(Address),
    RemoveSector(Address),
    SetLive { body: Address, value: u8 },
}

/// A caller-populated view of raw storage identities, not an allocation API.
#[derive(Clone, Debug, Default)]
pub struct Storage {
    pub nodes: BTreeMap<Address, Node>,
}

impl Storage {
    fn node(&self, address: Address) -> Result<&Node> {
        self.nodes.get(&address).ok_or(Unsupported::InvalidStableList)
    }
    /// Validation defines the admitted logical-storage domain. Invalid graphs
    /// are Unsupported, not successful empty lists. No guessed pointer reads.
    fn validate(&self, root: Address) -> Result<()> {
        let mut seen = BTreeSet::new();
        let mut current = root;
        loop {
            if !seen.insert(current) {
                return Err(Unsupported::InvalidStableList);
            }
            let node = self.node(current)?;
            if self.node(node.next)?.previous != current || self.node(node.previous)?.next != current {
                return Err(Unsupported::InvalidStableList);
            }
            if current != root && current < 4 {
                return Err(Unsupported::InvalidStableList);
            }
            current = node.next;
            if current == root {
                return Ok(());
            }
        }
    }
    fn unlink(&mut self, address: Address, events: &mut Vec<Event>) -> Result<Address> {
        let node = self.node(address)?;
        let (next, previous) = (node.next, node.previous);
        self.nodes
            .get_mut(&previous)
            .ok_or(Unsupported::InvalidStableList)?
            .next = next;
        events.push(Event::SetNext { node: previous, next });
        self.nodes
            .get_mut(&next)
            .ok_or(Unsupported::InvalidStableList)?
            .previous = previous;
        events.push(Event::SetPrevious { node: next, previous });
        Ok(next)
    }
    /// Stable forward sweep, saving next before unlink/free. Live bytes of ANY
    /// nonzero value survive; no body method or Release is invoked.
    pub fn sweep_object_list(
        &mut self,
        root: Address,
        foreign: &mut impl Foreign,
        events: &mut Vec<Event>,
    ) -> Result<()> {
        self.validate(root)?;
        events.push(Event::Sweep(root));
        let mut current = self.node(root)?.next;
        while current != root {
            let node = self.node(current)?;
            if node.live != 0 {
                current = node.next;
                continue;
            }
            let class = node.allocator_class;
            let next = self.unlink(current, events)?;
            let allocator = if class < 9 {
                Allocator::Class(class)
            } else {
                Allocator::Global
            };
            let allocation = current - 4;
            events.push(Event::FreeAllocation { allocation, allocator });
            foreign.free_allocation(allocation, allocator);
            self.nodes.remove(&current);
            current = next;
        }
        Ok(())
    }
    /// Roots indexed by receiver offsets +00,+04,+08; execution is +08,+04,+00.
    pub fn remove_dead_objects(
        &mut self,
        roots: [Address; 3],
        foreign: &mut impl Foreign,
        events: &mut Vec<Event>,
    ) -> Result<()> {
        for root in roots.into_iter().rev() {
            self.sweep_object_list(root, foreign, events)?;
        }
        Ok(())
    }
    /// §7.1 bounded RemoveObject only. Pair nodes free by their own address;
    /// sector call precedes deferred flag clearing. No object free or Release.
    pub fn remove_object(
        &mut self,
        pair_root: Address,
        body: Address,
        sector_word: u32,
        foreign: &mut impl Foreign,
        events: &mut Vec<Event>,
    ) -> Result<()> {
        self.validate(pair_root)?;
        let object_node = body.checked_sub(8).ok_or(Unsupported::InvalidStableList)?;
        self.node(object_node)?;
        let mut current = self.node(pair_root)?.next;
        while current != pair_root {
            let node = self.node(current)?;
            if node.pair.contains(&body) {
                let next = self.unlink(current, events)?;
                events.push(Event::FreePair(current));
                foreign.free_pair_node(current);
                self.nodes.remove(&current);
                current = next;
            } else {
                current = node.next;
            }
        }
        if sector_word != 0 {
            events.push(Event::RemoveSector(body));
            foreign.remove_from_sector_map(body);
        }
        self.nodes
            .get_mut(&object_node)
            .ok_or(Unsupported::InvalidStableList)?
            .live = 0;
        events.push(Event::SetLive { body, value: 0 });
        Ok(())
    }
}
