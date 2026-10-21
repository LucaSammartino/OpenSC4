// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors. See LICENSE at the repository root.

//! `cGZMessageQueue2` (§2): a FIFO of counted message references.

use std::collections::VecDeque;

use super::{Token, BLOCKING_WAIT_TIMEOUT};

/// Host callbacks a queue needs (§1 "Reference counting", §2).
pub trait MessageHost {
    /// The message's `GetType` (message vtable +0x10).
    fn message_type(&mut self, msg: Token) -> u32;
    /// The message equality test (message vtable +0x18).
    /// [choice] The spec does not say which side's vtable is called; the
    /// core calls it as `queued.equals(argument)`, i.e. `(queued, argument)`.
    fn messages_equal(&mut self, queued: Token, argument: Token) -> bool;
    /// One counted reference taken on `token`.
    fn add_ref(&mut self, token: Token);
    /// One counted reference dropped on `token`.
    fn release(&mut self, token: Token);
    /// Wait on the queue's thread signal (§2 slots 20/21). The unit of
    /// `timeout` and whether the wait resets the signal are unknown (GZ2);
    /// the host decides. The queue is passed so a test host (or another
    /// producer) can fill it. Default: return immediately.
    fn wait_signal(&mut self, queue: &mut MessageQueue2, timeout: u32) {
        let _ = (queue, timeout);
    }
}

/// The queue state (§2). Front = oldest for `push_back`.
#[derive(Debug, Default, Clone)]
pub struct MessageQueue2 {
    items: VecDeque<Token>,
    /// The thread signal: set on the empty to non-empty transition, never
    /// reset by the queue (§2).
    signal: bool,
    /// The queue's own reference count, 0 after construction (§2).
    ref_count: u32,
    destroyed: bool,
}

impl MessageQueue2 {
    pub fn new() -> Self {
        Self::default()
    }

    /// The thread signal state (§2).
    pub fn signal(&self) -> bool {
        self.signal
    }

    /// Reset the thread signal. The queue never does this itself (§2); the
    /// host's signal object may (GZ2).
    pub fn reset_signal(&mut self) {
        self.signal = false;
    }

    /// The queue's reference count.
    pub fn ref_count(&self) -> u32 {
        self.ref_count
    }

    /// True once `release` destroyed the queue (§2 Release).
    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    /// The queued messages, front first (inspection only).
    pub fn messages(&self) -> impl Iterator<Item = Token> + '_ {
        self.items.iter().copied()
    }

    /// AddRef: increment and return the count (§2).
    pub fn add_ref(&mut self) -> u32 {
        self.ref_count = self.ref_count.wrapping_add(1);
        self.ref_count
    }

    /// Release (§2): above 1, decrement and return it; otherwise (0 or 1)
    /// destroy the queue (slot 23, releasing every queued message) and
    /// return 0.
    pub fn release<H: MessageHost + ?Sized>(&mut self, host: &mut H) -> u32 {
        if self.ref_count > 1 {
            self.ref_count -= 1;
            return self.ref_count;
        }
        self.destroy(host);
        0
    }

    /// Slot 23, the destructor: releases every queued message.
    pub fn destroy<H: MessageHost + ?Sized>(&mut self, host: &mut H) {
        self.ref_count = 0;
        self.destroyed = true;
        while let Some(m) = self.items.pop_front() {
            host.release(m);
        }
    }

    /// Slot 3 PushBack: append (AddRef); set the signal if the queue was
    /// empty before the append.
    pub fn push_back<H: MessageHost + ?Sized>(&mut self, host: &mut H, msg: Token) {
        let was_empty = self.items.is_empty();
        host.add_ref(msg);
        self.items.push_back(msg);
        if was_empty {
            self.signal = true;
        }
    }

    /// Slot 4 PushFront: prepend; same signal rule.
    pub fn push_front<H: MessageHost + ?Sized>(&mut self, host: &mut H, msg: Token) {
        let was_empty = self.items.is_empty();
        host.add_ref(msg);
        self.items.push_front(msg);
        if was_empty {
            self.signal = true;
        }
    }

    fn find<H, F>(&self, host: &mut H, from_back: bool, mut pred: F) -> Option<usize>
    where
        H: MessageHost + ?Sized,
        F: FnMut(&mut H, Token) -> bool,
    {
        let n = self.items.len();
        for k in 0..n {
            let i = if from_back { n - 1 - k } else { k };
            if pred(host, self.items[i]) {
                return Some(i);
            }
        }
        None
    }

    fn remove_at<H: MessageHost + ?Sized>(&mut self, host: &mut H, i: usize) {
        if let Some(m) = self.items.remove(i) {
            host.release(m);
        }
    }

    fn remove_one<H, F>(&mut self, host: &mut H, from_back: bool, pred: F) -> bool
    where
        H: MessageHost + ?Sized,
        F: FnMut(&mut H, Token) -> bool,
    {
        match self.find(host, from_back, pred) {
            Some(i) => {
                self.remove_at(host, i);
                true
            }
            None => false,
        }
    }

    fn remove_every<H, F>(&mut self, host: &mut H, mut pred: F) -> u32
    where
        H: MessageHost + ?Sized,
        F: FnMut(&mut H, Token) -> bool,
    {
        let mut removed = 0u32;
        let mut i = 0;
        while i < self.items.len() {
            if pred(host, self.items[i]) {
                self.remove_at(host, i);
                removed += 1;
            } else {
                i += 1;
            }
        }
        removed
    }

    /// Slot 5: remove the first (oldest) message of type `ty`.
    pub fn remove_first_by_type<H: MessageHost + ?Sized>(&mut self, host: &mut H, ty: u32) -> bool {
        self.remove_one(host, false, |h, m| h.message_type(m) == ty)
    }

    /// Slot 6: remove the first message equal to `msg`.
    pub fn remove_first_equal<H: MessageHost + ?Sized>(&mut self, host: &mut H, msg: Token) -> bool {
        self.remove_one(host, false, |h, m| h.messages_equal(m, msg))
    }

    /// Slot 7: remove the last (newest) message of type `ty`.
    pub fn remove_last_by_type<H: MessageHost + ?Sized>(&mut self, host: &mut H, ty: u32) -> bool {
        self.remove_one(host, true, |h, m| h.message_type(m) == ty)
    }

    /// Slot 8: remove the last message equal to `msg`.
    pub fn remove_last_equal<H: MessageHost + ?Sized>(&mut self, host: &mut H, msg: Token) -> bool {
        self.remove_one(host, true, |h, m| h.messages_equal(m, msg))
    }

    /// Slot 9: remove every message of type `ty`; the number removed.
    pub fn remove_all_by_type<H: MessageHost + ?Sized>(&mut self, host: &mut H, ty: u32) -> u32 {
        self.remove_every(host, |h, m| h.message_type(m) == ty)
    }

    /// Slot 10: remove every message equal to `msg`; the number removed.
    pub fn remove_all_equal<H: MessageHost + ?Sized>(&mut self, host: &mut H, msg: Token) -> u32 {
        self.remove_every(host, |h, m| h.messages_equal(m, msg))
    }

    /// Slot 11 RemoveAll: empty the queue; how many were in it.
    pub fn remove_all<H: MessageHost + ?Sized>(&mut self, host: &mut H) -> u32 {
        let n = self.items.len() as u32;
        while let Some(m) = self.items.pop_front() {
            host.release(m);
        }
        n
    }

    /// Slot 12: type of the front message, 0 when empty.
    pub fn front_type<H: MessageHost + ?Sized>(&self, host: &mut H) -> u32 {
        self.items.front().map_or(0, |&m| host.message_type(m))
    }

    /// Slot 13: type of the back message, 0 when empty.
    pub fn back_type<H: MessageHost + ?Sized>(&self, host: &mut H) -> u32 {
        self.items.back().map_or(0, |&m| host.message_type(m))
    }

    /// Slot 14: true if any message has type `ty`.
    pub fn contains_type<H: MessageHost + ?Sized>(&self, host: &mut H, ty: u32) -> bool {
        self.items.iter().any(|&m| host.message_type(m) == ty)
    }

    fn peek_at<H: MessageHost + ?Sized>(&self, host: &mut H, i: Option<usize>) -> Option<Token> {
        let m = self.items.get(i?).copied()?;
        host.add_ref(m);
        Some(m)
    }

    /// Slot 15: peek front; the returned message is AddRef'd for the caller
    /// and stays queued. `None` = out pointer null, return false.
    pub fn peek_front<H: MessageHost + ?Sized>(&self, host: &mut H) -> Option<Token> {
        self.peek_at(host, Some(0))
    }

    /// Slot 16: peek back.
    pub fn peek_back<H: MessageHost + ?Sized>(&self, host: &mut H) -> Option<Token> {
        let i = self.items.len().checked_sub(1);
        self.peek_at(host, i)
    }

    /// Slot 17: first message equal to `msg`, front to back, AddRef'd.
    pub fn peek_equal<H: MessageHost + ?Sized>(&self, host: &mut H, msg: Token) -> Option<Token> {
        let i = self.find(host, false, |h, m| h.messages_equal(m, msg));
        self.peek_at(host, i)
    }

    /// Slot 18: get front: remove it; the caller now owns the queue's
    /// reference (AddRef for the caller + Release by the queue, reported
    /// as the net zero: no callback).
    pub fn get_front(&mut self) -> Option<Token> {
        self.items.pop_front()
    }

    /// Slot 19: get back.
    pub fn get_back(&mut self) -> Option<Token> {
        self.items.pop_back()
    }

    /// Slot 24: get the first message equal to `msg` and remove it.
    pub fn get_equal<H: MessageHost + ?Sized>(&mut self, host: &mut H, msg: Token) -> Option<Token> {
        let i = self.find(host, false, |h, m| h.messages_equal(m, msg))?;
        self.items.remove(i)
    }

    /// Slot 20: blocking get front: retry slot 18, waiting on the signal
    /// with timeout 1,000,000 between tries. Always succeeds.
    pub fn blocking_get_front<H: MessageHost + ?Sized>(&mut self, host: &mut H) -> Token {
        loop {
            if let Some(m) = self.get_front() {
                return m;
            }
            host.wait_signal(self, BLOCKING_WAIT_TIMEOUT);
        }
    }

    /// Slot 21: blocking get back.
    pub fn blocking_get_back<H: MessageHost + ?Sized>(&mut self, host: &mut H) -> Token {
        loop {
            if let Some(m) = self.get_back() {
                return m;
            }
            host.wait_signal(self, BLOCKING_WAIT_TIMEOUT);
        }
    }

    /// Slot 22: message count.
    pub fn count(&self) -> u32 {
        self.items.len() as u32
    }
}
