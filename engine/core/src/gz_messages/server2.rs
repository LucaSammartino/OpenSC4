// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors. See LICENSE at the repository root.

//! `cGZMessageServer2` (§4): synchronous send, posted messages drained by the
//! tick, and directed messages.

use std::collections::VecDeque;

use super::queue::{MessageHost, MessageQueue2};
use super::registry::{self, HasRegistry, Registry, RegistryHost};
use super::{Token, MESSAGE_SERVER2_IID, STANDARD_MESSAGE_CLSID};

/// Host callbacks of message server 2.
pub trait Server2Host: MessageHost {
    /// AddRef/GetType can re-enter while registry depth is positive (GZ6b).
    fn add_ref_target(&mut self, _server: &mut MessageServer2, target: Token) {
        self.add_ref(target);
    }
    fn message_type_in_server(&mut self, _server: &mut MessageServer2, msg: Token) -> u32 {
        self.message_type(msg)
    }
    fn add_ref_message(&mut self, _server: &mut MessageServer2, msg: Token) {
        self.add_ref(msg);
    }
    fn release_message(&mut self, _server: &mut MessageServer2, msg: Token) {
        self.release(msg);
    }
    /// `target->DoMessage(msg)` (target vtable +0x0C). The return value is
    /// ignored by every caller (§4.3, §4.5). The server is passed so the
    /// target may re-enter it.
    fn do_message(&mut self, server: &mut MessageServer2, target: Token, msg: Token);
    /// Drop one reference on a target. Default: [`MessageHost::release`].
    /// Override to let a target destructor re-enter the server.
    fn release_target(&mut self, server: &mut MessageServer2, target: Token) {
        let _ = server;
        self.release(target);
    }
    /// Allocate a `cRZMessage2Standard` and ask it for `iid` (§4.6). The
    /// returned token carries the caller's only reference. Default: none.
    fn create_standard_message(&mut self, iid: u32) -> Option<Token> {
        let _ = iid;
        None
    }
}

impl<H: Server2Host + ?Sized> RegistryHost<MessageServer2> for H {
    fn add_ref_target(&mut self, server: &mut MessageServer2, target: Token) {
        Server2Host::add_ref_target(self, server, target);
    }
    fn release_target(&mut self, server: &mut MessageServer2, target: Token) {
        Server2Host::release_target(self, server, target);
    }
}

/// Message server 2 state (§4.1).
#[derive(Debug, Clone)]
pub struct MessageServer2 {
    ref_count: u32,
    registry: Registry,
    /// The post queue (+0x38).
    post_queue: MessageQueue2,
    /// "Messages taken" (+0x58).
    taken: u64,
    /// Drain-all flag (+0x60), default true.
    drain_all: bool,
    /// Directed countdown (+0x80).
    countdown: i32,
    /// Directed deque (+0x84): (target, message), both referenced.
    directed: VecDeque<(Token, Token)>,
}

impl Default for MessageServer2 {
    fn default() -> Self {
        Self::new()
    }
}

impl HasRegistry for MessageServer2 {
    fn registry_mut(&mut self) -> &mut Registry {
        &mut self.registry
    }
}

impl MessageServer2 {
    /// Constructor (§4.1).
    pub fn new() -> Self {
        Self {
            ref_count: 0,
            registry: Registry::default(),
            post_queue: MessageQueue2::new(),
            taken: 0,
            drain_all: true,
            countdown: 0,
            directed: VecDeque::new(),
        }
    }

    /// Slot 0: true for iid `0x652294C7`; other iids go to the
    /// system-service QueryInterface.
    pub fn answers_iid(iid: u32) -> bool {
        iid == MESSAGE_SERVER2_IID
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn post_queue(&self) -> &MessageQueue2 {
        &self.post_queue
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

    /// Slot 12 and the +8 unknown-interface getter read the same dword.
    pub fn ref_count(&self) -> u32 {
        self.ref_count
    }

    /// Both message/service and +8-interface AddRef forward here (GZ1).
    pub fn add_ref(&mut self) -> u32 {
        self.ref_count = self.ref_count.wrapping_add(1);
        self.ref_count
    }

    /// +8-interface slot 4: decrement only if positive, never destroy.
    pub fn remove_ref(&mut self) -> u32 {
        self.ref_count = self.ref_count.saturating_sub(1);
        self.ref_count
    }

    /// The directed deque, front first: (target, message).
    pub fn directed(&self) -> impl Iterator<Item = (Token, Token)> + '_ {
        self.directed.iter().copied()
    }

    /// Slot 3 MessageSend (§4.3). Always true.
    pub fn message_send<H: Server2Host + ?Sized>(&mut self, host: &mut H, msg: Token) -> bool {
        registry::apply_pending(self, host);
        self.registry.depth = self.registry.depth.wrapping_add(1);
        let ty = host.message_type_in_server(self, msg);
        let mut targets = self.registry.snapshot(ty, false);
        for &t in &targets {
            Server2Host::add_ref_target(host, self, t);
        }
        // Section 4.3 copies the typed list before looking up wildcard 0.
        // A typed target's AddRef can enter a nested send which applies work
        // to the separate wildcard list without invalidating this traversal.
        if ty != 0 {
            for t in self.registry.targets(0).to_vec() {
                Server2Host::add_ref_target(host, self, t);
                targets.push(t);
            }
        }
        self.registry.depth = self.registry.depth.wrapping_sub(1);
        for &t in &targets {
            host.do_message(self, t, msg);
        }
        // GZ6a: keep every per-occurrence hold until ALL callbacks finish.
        for t in targets {
            Server2Host::release_target(host, self, t);
        }
        true
    }

    /// Slot 4 MessagePost(msg, toFront) (§4.4). Always true.
    pub fn message_post<H: MessageHost + ?Sized>(&mut self, host: &mut H, msg: Token, to_front: bool) -> bool {
        if to_front {
            self.post_queue.push_front(host, msg);
        } else {
            self.post_queue.push_back(host, msg);
        }
        true
    }

    /// Slot 5 AddNotification(target, type) (§4.2). Always true.
    pub fn add_notification<H: Server2Host + ?Sized>(&mut self, host: &mut H, target: Token, ty: u32) -> bool {
        registry::add_notification(self, host, target, ty)
    }

    /// Slot 6 RemoveNotification(target, type) (§4.2). Always true.
    pub fn remove_notification<H: Server2Host + ?Sized>(&mut self, host: &mut H, target: Token, ty: u32) -> bool {
        registry::remove_notification(self, host, target, ty)
    }

    /// Slot 7 post directed (msg, target) (§4.5): append (target, msg), both
    /// referenced. Native slot 7 is void. Construct the temporary, copy it
    /// into the deque, then destroy it before leaving the directed lock.
    pub fn post_directed<H: Server2Host + ?Sized>(&mut self, host: &mut H, msg: Token, target: Token) {
        Server2Host::add_ref_target(host, self, target);
        host.add_ref_message(self, msg);
        Server2Host::add_ref_target(host, self, target);
        host.add_ref_message(self, msg);
        self.directed.push_back((target, msg));
        host.release_message(self, msg);
        Server2Host::release_target(host, self, target);
    }

    /// Slot 8: preserve the native pre-advanced iterator and suffix skip.
    /// Precondition: no erase may advance the scan beyond the new end
    /// (notably erasing the last/sole element is unsupported native UB).
    /// Native slot 8 is void.
    pub fn cancel_directed<H: Server2Host + ?Sized>(&mut self, host: &mut H, target: Token) {
        let mut scan = 0;
        while scan < self.directed.len() {
            let i = scan;
            scan += 1;
            if self.directed[i].0 != target {
                continue;
            }
            if i < self.directed.len() / 2 {
                for j in (1..=i).rev() {
                    let pair = self.directed[j - 1];
                    self.assign_directed(host, j, pair);
                }
                let (t, m) = self.directed.pop_front().unwrap();
                host.release_message(self, m);
                Server2Host::release_target(host, self, t);
                scan -= 1; // prefix pop moves the advanced iterator's index
            } else {
                for j in i..self.directed.len() - 1 {
                    let pair = self.directed[j + 1];
                    self.assign_directed(host, j, pair);
                }
                let (t, m) = self.directed.pop_back().unwrap();
                host.release_message(self, m);
                Server2Host::release_target(host, self, t);
                // Do not turn native iterator invalidation into remove-all.
            }
        }
    }

    fn assign_directed<H: Server2Host + ?Sized>(&mut self, host: &mut H, i: usize, (t, m): (Token, Token)) {
        let (old_t, old_m) = self.directed[i];
        if t != old_t {
            Server2Host::add_ref_target(host, self, t);
            self.directed[i].0 = t;
            Server2Host::release_target(host, self, old_t);
        }
        if m != old_m {
            host.add_ref_message(self, m);
            self.directed[i].1 = m;
            host.release_message(self, old_m);
        }
    }

    /// Slot 9 OnTick (§4.4). Always true.
    pub fn on_tick<H: Server2Host + ?Sized>(&mut self, host: &mut H) -> bool {
        let limit = self.taken.wrapping_add(u64::from(self.post_queue.count()));
        while let Some(m) = self.post_queue.get_front(host) {
            self.taken = self.taken.wrapping_add(1);
            self.message_send(host, m);
            host.release_message(self, m);
            if !self.drain_all && self.taken >= limit {
                break;
            }
        }
        if !self.directed.is_empty() {
            self.deliver_directed(host);
        }
        true
    }

    /// Directed delivery (§4.5, end of OnTick).
    pub fn deliver_directed<H: Server2Host + ?Sized>(&mut self, host: &mut H) {
        let (mut local_t, mut local_m) = (None, None);
        if self.countdown <= 0 {
            self.countdown = self.directed.len() as i32;
        }
        while self.countdown > 0 {
            self.countdown = self.countdown.wrapping_sub(1);
            if let Some((t, m)) = self.directed.front().copied() {
                if local_t != Some(t) {
                    Server2Host::add_ref_target(host, self, t);
                    let old = local_t.replace(t);
                    if let Some(old) = old {
                        Server2Host::release_target(host, self, old);
                    }
                }
                if local_m != Some(m) {
                    host.add_ref_message(self, m);
                    let old = local_m.replace(m);
                    if let Some(old) = old {
                        host.release_message(self, old);
                    }
                }
                self.directed.pop_front();
                host.release_message(self, m);
                Server2Host::release_target(host, self, t);
                host.do_message(self, t, m);
            }
        }
        self.countdown = self.countdown.wrapping_sub(1);
        // Directed lock has been left; each invocation owns its local pair.
        if let Some(m) = local_m {
            host.release_message(self, m);
        }
        if let Some(t) = local_t {
            Server2Host::release_target(host, self, t);
        }
    }

    /// Slot 10 posted-message count.
    pub fn posted_count(&self) -> u32 {
        self.post_queue.count()
    }

    /// Slot 11 set drain-all flag.
    pub fn set_drain_all(&mut self, on: bool) {
        self.drain_all = on;
    }

    /// Slot 13 CreateMessage(clsid, iid, out) (§4.6): `None` = false with
    /// `out` untouched.
    pub fn create_message<H: Server2Host + ?Sized>(&self, host: &mut H, clsid: u32, iid: u32) -> Option<Token> {
        if clsid != STANDARD_MESSAGE_CLSID {
            return None;
        }
        host.create_standard_message(iid)
    }

    /// Destruction (§4.7): the type lists, the pending list, the deque and
    /// the queue, each with its references.
    pub fn destroy<H: Server2Host + ?Sized>(&mut self, host: &mut H) {
        registry::clear(self, host);
        for (t, m) in std::mem::take(&mut self.directed) {
            host.release_message(self, m);
            Server2Host::release_target(host, self, t);
        }
        self.post_queue.destroy(host);
    }
}

#[cfg(test)]
mod counter_boundary {
    use super::MessageServer2;
    #[test]
    fn shared_unsigned_counter_wraps_without_deletion() {
        let mut server = MessageServer2::new();
        server.ref_count = u32::MAX;
        assert_eq!(server.add_ref(), 0);
        assert_eq!(server.remove_ref(), 0);
        assert_eq!(server.ref_count(), 0);
    }
}
