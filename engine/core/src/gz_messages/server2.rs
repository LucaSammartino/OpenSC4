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
    fn add_ref_target(&mut self, target: Token) {
        self.add_ref(target);
    }
    fn release_target(&mut self, server: &mut MessageServer2, target: Token) {
        Server2Host::release_target(self, server, target);
    }
}

/// Message server 2 state (§4.1).
#[derive(Debug, Clone)]
pub struct MessageServer2 {
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

    /// The directed deque, front first: (target, message).
    pub fn directed(&self) -> impl Iterator<Item = (Token, Token)> + '_ {
        self.directed.iter().copied()
    }

    /// Slot 3 MessageSend (§4.3). Always true.
    pub fn message_send<H: Server2Host + ?Sized>(&mut self, host: &mut H, msg: Token) -> bool {
        registry::apply_pending(self, host);
        self.registry.depth += 1;
        let ty = host.message_type(msg);
        let targets = self.registry.snapshot(ty, true);
        self.registry.depth -= 1;
        // [choice] The spec lists no AddRef of the snapshot for server 2
        // (unlike §4A), so the copied targets are not referenced.
        for t in targets {
            host.do_message(self, t, msg);
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
    /// referenced. [choice] Returns true (the spec gives no return value).
    pub fn post_directed<H: MessageHost + ?Sized>(&mut self, host: &mut H, msg: Token, target: Token) -> bool {
        host.add_ref(target);
        host.add_ref(msg);
        self.directed.push_back((target, msg));
        true
    }

    /// Slot 8 cancel directed messages for `target` (§4.5): erase every
    /// entry of that target, dropping both references (message first,
    /// [choice]). Returns the number erased ([choice], the spec gives none).
    pub fn cancel_directed<H: MessageHost + ?Sized>(&mut self, host: &mut H, target: Token) -> u32 {
        let mut erased = 0;
        let mut kept = VecDeque::with_capacity(self.directed.len());
        for (t, m) in std::mem::take(&mut self.directed) {
            if t == target {
                host.release(m);
                host.release(t);
                erased += 1;
            } else {
                kept.push_back((t, m));
            }
        }
        // A release above cannot re-enter (MessageHost has no server
        // access), so nothing was appended meanwhile.
        self.directed = kept;
        erased
    }

    /// Slot 9 OnTick (§4.4). Always true.
    pub fn on_tick<H: Server2Host + ?Sized>(&mut self, host: &mut H) -> bool {
        let limit = self.taken.wrapping_add(u64::from(self.post_queue.count()));
        while let Some(m) = self.post_queue.get_front() {
            self.taken = self.taken.wrapping_add(1);
            self.message_send(host, m);
            host.release(m);
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
        if self.countdown <= 0 {
            self.countdown = self.directed.len() as i32;
        }
        while self.countdown > 0 {
            self.countdown -= 1;
            if let Some((t, m)) = self.directed.pop_front() {
                host.do_message(self, t, m);
                // [choice] The deque's references are dropped after the call.
                host.release(m);
                host.release(t);
            }
        }
        self.countdown -= 1;
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
            host.release(m);
            host.release(t);
        }
        self.post_queue.remove_all(host);
    }
}
