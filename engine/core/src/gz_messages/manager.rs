// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors. See LICENSE at the repository root.

//! `cGZMessageQueueManager2` (§3): a sorted map queue id -> queue reference.

use std::collections::BTreeMap;

use super::{Token, QUEUE_MANAGER2_IID};

/// Host callbacks of the queue manager. Queues are host tokens; the host
/// owns the queue objects (typically [`super::MessageQueue2`] values).
pub trait QueueHost {
    /// One counted reference taken on the queue.
    fn add_ref_queue(&mut self, queue: Token);
    /// One counted reference dropped on the queue (the host destroys a
    /// queue nobody holds, §2 Release).
    fn release_queue(&mut self, queue: Token);
    /// Allocate a new empty queue (§3 slot 5), reference count 0.
    fn new_queue(&mut self) -> Token;
}

/// The queue registry (§3).
#[derive(Debug, Default, Clone)]
pub struct QueueManager2 {
    map: BTreeMap<u32, Token>,
}

impl QueueManager2 {
    pub fn new() -> Self {
        Self::default()
    }

    /// Slot 0 QueryInterface: true for iid `0x652294BC` (the glue returns
    /// `this` AddRef'd); other iids go to the system-service base.
    pub fn answers_iid(iid: u32) -> bool {
        iid == QUEUE_MANAGER2_IID
    }

    /// The registered (id, queue) pairs in id order (inspection only).
    pub fn entries(&self) -> impl Iterator<Item = (u32, Token)> + '_ {
        self.map.iter().map(|(&k, &v)| (k, v))
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Slot 3 GetQueue(id, out): the queue AddRef'd for the caller, or None.
    pub fn get_queue<H: QueueHost + ?Sized>(&self, host: &mut H, id: u32) -> Option<Token> {
        let q = *self.map.get(&id)?;
        host.add_ref_queue(q);
        Some(q)
    }

    /// Slot 4 AddQueue(id, queue): insert when absent (the map takes a
    /// reference), true; false and no change when the id exists.
    pub fn add_queue<H: QueueHost + ?Sized>(&mut self, host: &mut H, id: u32, queue: Token) -> bool {
        if self.map.contains_key(&id) {
            return false;
        }
        host.add_ref_queue(queue);
        self.map.insert(id, queue);
        true
    }

    /// Slot 5 CreateQueue(id): when absent, allocate a new empty queue and
    /// insert it (the map's reference is its only one), true; false when
    /// present (nothing is allocated).
    pub fn create_queue<H: QueueHost + ?Sized>(&mut self, host: &mut H, id: u32) -> bool {
        if self.map.contains_key(&id) {
            return false;
        }
        let q = host.new_queue();
        host.add_ref_queue(q);
        self.map.insert(id, q);
        true
    }

    /// Slot 6 RemoveQueue(queue): linear scan (in id order) for the entry
    /// holding that queue, erase it, true; false if none. Only the first
    /// such entry is erased.
    pub fn remove_queue<H: QueueHost + ?Sized>(&mut self, host: &mut H, queue: Token) -> bool {
        let Some(id) = self.map.iter().find(|(_, &q)| q == queue).map(|(&k, _)| k) else {
            return false;
        };
        self.map.remove(&id);
        host.release_queue(queue);
        true
    }

    /// Slot 7 RemoveQueue(id): erase the entry, true; false if absent.
    pub fn remove_queue_id<H: QueueHost + ?Sized>(&mut self, host: &mut H, id: u32) -> bool {
        match self.map.remove(&id) {
            Some(q) => {
                host.release_queue(q);
                true
            }
            None => false,
        }
    }

    /// Slot 8 RemoveAllQueues: clear the map (releasing each queue in id
    /// order, [choice, GZ8] the spec gives no order); always true.
    pub fn remove_all_queues<H: QueueHost + ?Sized>(&mut self, host: &mut H) -> bool {
        let map = std::mem::take(&mut self.map);
        for (_, q) in map {
            host.release_queue(q);
        }
        true
    }
}
