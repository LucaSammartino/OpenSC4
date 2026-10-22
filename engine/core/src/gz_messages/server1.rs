// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors. See LICENSE at the repository root.

//! `cGZMessageServer` (§4A): the first-generation server. Same structure and
//! algorithms as message server 2 (§4) except: messages are 16-byte value
//! records, send has no wildcard and ignores type 0, and the target snapshot
//! holds at most 385 targets, each AddRef'd for its delivery.

use std::collections::VecDeque;

use super::registry::{self, HasRegistry, Registry, RegistryHost};
use super::{Token, MESSAGE_SERVER1_IID};

/// A first-generation message: 16 bytes, the first dword is the type.
pub type Record = [u32; 4];

/// The snapshot capacity (§4A: 386 slots, one used as the terminator).
pub const SERVER1_SNAPSHOT_LIMIT: usize = 385;

/// Host callbacks of the first-generation server.
pub trait Server1Host {
    fn add_ref_target(&mut self, target: Token);
    fn add_ref_target_in_server(&mut self, _server: &mut MessageServer1, target: Token) {
        self.add_ref_target(target);
    }
    fn release_target(&mut self, server: &mut MessageServer1, target: Token);
    /// `target->DoMessage(&record)` (+0x0C); return value ignored.
    fn do_message(&mut self, server: &mut MessageServer1, target: Token, msg: &Record);
}

impl<H: Server1Host + ?Sized> RegistryHost<MessageServer1> for H {
    fn add_ref_target(&mut self, server: &mut MessageServer1, target: Token) {
        self.add_ref_target_in_server(server, target);
    }
    fn release_target(&mut self, server: &mut MessageServer1, target: Token) {
        Server1Host::release_target(self, server, target);
    }
}

/// First-generation server state (§4A).
#[derive(Debug, Clone)]
pub struct MessageServer1 {
    registry: Registry,
    /// The embedded `cGZMessageQueue` (+0x38): record copies.
    post_queue: VecDeque<Record>,
    taken: u64,
    /// Drain-all flag (+0x64), default true.
    drain_all: bool,
    countdown: i32,
    /// Raw target and copied record; caller preserves both lifetimes (GZ5).
    directed: VecDeque<(Token, Record)>,
}

impl Default for MessageServer1 {
    fn default() -> Self {
        Self::new()
    }
}

impl HasRegistry for MessageServer1 {
    const RAW_PENDING: bool = true;
    fn registry_mut(&mut self) -> &mut Registry {
        &mut self.registry
    }
}

impl MessageServer1 {
    pub fn new() -> Self {
        Self {
            registry: Registry::default(),
            post_queue: VecDeque::new(),
            taken: 0,
            drain_all: true,
            countdown: 0,
            directed: VecDeque::new(),
        }
    }

    /// QueryInterface of the +0x1C interface: iid `0xC1085722`.
    pub fn answers_iid(iid: u32) -> bool {
        iid == MESSAGE_SERVER1_IID
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn taken(&self) -> u64 {
        self.taken
    }

    pub fn drain_all(&self) -> bool {
        self.drain_all
    }

    pub fn countdown(&self) -> i32 {
        self.countdown
    }

    pub fn posted(&self) -> impl Iterator<Item = &Record> + '_ {
        self.post_queue.iter()
    }

    pub fn directed(&self) -> impl Iterator<Item = &(Token, Record)> + '_ {
        self.directed.iter()
    }

    /// Slot 3 MessageSend. Type 0 delivers to nobody; otherwise the targets
    /// of exactly that type, in registration order, each AddRef'd in the
    /// snapshot and released right after its call. Returns true.
    pub fn message_send<H: Server1Host + ?Sized>(&mut self, host: &mut H, msg: &Record) -> bool {
        registry::apply_pending(self, host);
        self.registry.depth = self.registry.depth.wrapping_add(1);
        let ty = msg[0];
        let mut targets = if ty == 0 {
            Vec::new()
        } else {
            self.registry.snapshot(ty, false)
        };
        // The original has no bound check past 385 (undefined behaviour);
        // §4A allows treating 385 as the limit: the rest are not delivered.
        targets.truncate(SERVER1_SNAPSHOT_LIMIT);
        for &t in &targets {
            host.add_ref_target_in_server(self, t);
        }
        self.registry.depth = self.registry.depth.wrapping_sub(1);
        let copy = *msg;
        for t in targets {
            host.do_message(self, t, &copy);
            Server1Host::release_target(host, self, t);
        }
        true
    }

    /// Slot 4 MessagePost(msg, toFront): stores a copy. Always true.
    pub fn message_post(&mut self, msg: &Record, to_front: bool) -> bool {
        if to_front {
            self.post_queue.push_front(*msg);
        } else {
            self.post_queue.push_back(*msg);
        }
        true
    }

    pub fn add_notification<H: Server1Host + ?Sized>(&mut self, host: &mut H, target: Token, ty: u32) -> bool {
        registry::add_notification(self, host, target, ty)
    }

    pub fn remove_notification<H: Server1Host + ?Sized>(&mut self, host: &mut H, target: Token, ty: u32) -> bool {
        registry::remove_notification(self, host, target, ty)
    }

    /// Slot 7 post directed (msg, target).
    pub fn post_directed<H: Server1Host + ?Sized>(&mut self, _host: &mut H, msg: &Record, target: Token) {
        self.directed.push_back((target, *msg));
    }

    /// Slot 8: native prefix erase / suffix skip, with no reference calls.
    /// Precondition: erasure must not leave the pre-advanced scan past end
    /// (in particular, matching the last/sole entry is native undefined behavior).
    /// Native slot 8 is void.
    pub fn cancel_directed<H: Server1Host + ?Sized>(&mut self, _host: &mut H, target: Token) {
        let mut scan = 0;
        while scan < self.directed.len() {
            let i = scan;
            scan += 1;
            if self.directed[i].0 == target {
                let prefix = i < self.directed.len() / 2;
                self.directed.remove(i);
                if prefix {
                    scan -= 1;
                }
            }
        }
    }

    /// Slot 9 OnTick (as §4.4): the front record is taken into a local copy
    /// and sent.
    pub fn on_tick<H: Server1Host + ?Sized>(&mut self, host: &mut H) -> bool {
        let limit = self.taken.wrapping_add(self.post_queue.len() as u64);
        while let Some(rec) = self.post_queue.pop_front() {
            self.taken = self.taken.wrapping_add(1);
            self.message_send(host, &rec);
            if !self.drain_all && self.taken >= limit {
                break;
            }
        }
        if !self.directed.is_empty() {
            self.deliver_directed(host);
        }
        true
    }

    /// Directed delivery (as §4.5).
    pub fn deliver_directed<H: Server1Host + ?Sized>(&mut self, host: &mut H) {
        if self.countdown <= 0 {
            self.countdown = self.directed.len() as i32;
        }
        while self.countdown > 0 {
            self.countdown = self.countdown.wrapping_sub(1);
            if let Some((t, rec)) = self.directed.pop_front() {
                host.do_message(self, t, &rec);
            }
        }
        self.countdown = self.countdown.wrapping_sub(1);
    }

    /// Slot 10 posted-message count.
    pub fn posted_count(&self) -> u32 {
        self.post_queue.len() as u32
    }

    /// Slot 11 set drain-all flag.
    pub fn set_drain_all(&mut self, on: bool) {
        self.drain_all = on;
    }

    /// Destruction (as §4.7).
    pub fn destroy<H: Server1Host + ?Sized>(&mut self, host: &mut H) {
        registry::clear(self, host);
        self.directed.clear();
        self.post_queue.clear();
    }
}
