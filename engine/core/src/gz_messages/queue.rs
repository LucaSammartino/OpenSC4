// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors. See LICENSE at the repository root.

//! `cGZMessageQueue2` (§2): a FIFO of counted message references.

use std::collections::VecDeque;

use super::{Token, BLOCKING_WAIT_TIMEOUT};

/// Host callbacks a queue needs (§1 "Reference counting", §2).
pub trait MessageHost {
    /// The message's `GetType` (message vtable +0x10).
    fn message_type(&mut self, msg: Token) -> u32;
    /// The queued receiver's equality test (message vtable +0x18, GZ7).
    fn messages_equal(&mut self, queued: Token, argument: Token) -> bool;
    /// Raw native return; adapters override this to preserve low-byte truth.
    fn equality_result(&mut self, queued: Token, argument: Token) -> u32 {
        u32::from(self.messages_equal(queued, argument))
    }
    /// One counted reference taken on `token`.
    fn add_ref(&mut self, token: Token);
    /// One counted reference dropped on `token`.
    fn release(&mut self, token: Token);
    /// WaitForSingleObject equivalent: timeout is unsigned MILLISECONDS,
    /// u32::MAX means infinite; return the raw OS status. Hold the platform
    /// signal critical section across this call. No explicit ResetEvent.
    /// The default invalid handle fails; a blocking get still retries.
    fn wait_signal(&mut self, queue: &mut MessageQueue2, timeout_ms: u32) -> u32 {
        let _ = (queue, timeout_ms);
        u32::MAX
    }
    /// Mode-0 PulseEvent, NOT a persistent notification. The host creates an
    /// initially nonsignalled auto-reset event; no eligible waiter loses it.
    fn pulse_signal(&mut self) {}
    /// Close event, destroy platform signal lock/string and free its state.
    /// Called before releasing queue messages. No join or cancellation wake.
    fn close_signal(&mut self) {}
}

/// GZ2: all finite microsecond timeouts truncate to milliseconds.
pub const fn signal_wait_millis(timeout_us: u32) -> u32 {
    if timeout_us == u32::MAX {
        u32::MAX
    } else {
        timeout_us / 1_000
    }
}

/// GZ2: WAIT_FAILED -> 0, WAIT_TIMEOUT -> 2, every other status -> 1.
pub const fn signal_wait_result(os_status: u32) -> u32 {
    match os_status {
        u32::MAX => 0,
        0x102 => 2,
        _ => 1,
    }
}

/// The queue state (§2). Front = oldest for `push_back`.
#[derive(Debug, Default, Clone)]
pub struct MessageQueue2 {
    items: VecDeque<Token>,
    /// Diagnostic pulse count, never a wake token or a signalled state.
    pulses: u64,
    /// The queue's own reference count, 0 after construction (§2).
    ref_count: u32,
    destroyed: bool,
}

impl MessageQueue2 {
    pub fn new() -> Self {
        Self::default()
    }

    /// Empty-to-nonempty pulses issued, including pulses lost without waiters.
    pub fn pulses(&self) -> u64 {
        self.pulses
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
        host.close_signal();
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
            self.pulses = self.pulses.wrapping_add(1);
            host.pulse_signal();
        }
    }

    /// Slot 4 PushFront: prepend; same signal rule.
    pub fn push_front<H: MessageHost + ?Sized>(&mut self, host: &mut H, msg: Token) {
        let was_empty = self.items.is_empty();
        host.add_ref(msg);
        self.items.push_front(msg);
        if was_empty {
            self.pulses = self.pulses.wrapping_add(1);
            host.pulse_signal();
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
        self.remove_one(host, false, |h, m| h.equality_result(m, msg) as u8 != 0)
    }

    /// Slot 7: remove the last (newest) message of type `ty`.
    pub fn remove_last_by_type<H: MessageHost + ?Sized>(&mut self, host: &mut H, ty: u32) -> bool {
        self.remove_one(host, true, |h, m| h.message_type(m) == ty)
    }

    /// Slot 8: remove the last message equal to `msg`.
    pub fn remove_last_equal<H: MessageHost + ?Sized>(&mut self, host: &mut H, msg: Token) -> bool {
        self.remove_one(host, true, |h, m| h.equality_result(m, msg) as u8 != 0)
    }

    /// Slot 9: remove every message of type `ty`; the number removed.
    pub fn remove_all_by_type<H: MessageHost + ?Sized>(&mut self, host: &mut H, ty: u32) -> u32 {
        self.remove_every(host, |h, m| h.message_type(m) == ty)
    }

    /// Slot 10: remove every message equal to `msg`; the number removed.
    pub fn remove_all_equal<H: MessageHost + ?Sized>(&mut self, host: &mut H, msg: Token) -> u32 {
        self.remove_every(host, |h, m| h.equality_result(m, msg) as u8 != 0)
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
        let i = self.find(host, false, |h, m| h.equality_result(m, msg) as u8 != 0);
        self.peek_at(host, i)
    }

    /// Slot 18: acquire the caller hold, then remove/release the queue hold.
    pub fn get_front<H: MessageHost + ?Sized>(&mut self, host: &mut H) -> Option<Token> {
        self.get_at(host, 0)
    }

    /// Slot 19: get back.
    pub fn get_back<H: MessageHost + ?Sized>(&mut self, host: &mut H) -> Option<Token> {
        self.get_at(host, self.items.len().checked_sub(1)?)
    }

    /// Slot 24: get the first message equal to `msg` and remove it.
    pub fn get_equal<H: MessageHost + ?Sized>(&mut self, host: &mut H, msg: Token) -> Option<Token> {
        let i = self.find(host, false, |h, m| h.equality_result(m, msg) as u8 != 0)?;
        self.get_at(host, i)
    }

    fn get_at<H: MessageHost + ?Sized>(&mut self, host: &mut H, i: usize) -> Option<Token> {
        let m = *self.items.get(i)?;
        host.add_ref(m);
        self.items.remove(i);
        host.release(m);
        Some(m)
    }

    /// Slot 20: blocking get front: retry slot 18, waiting on the signal
    /// with timeout 1,000,000 between tries. Always succeeds.
    pub fn blocking_get_front<H: MessageHost + ?Sized>(&mut self, host: &mut H) -> Token {
        loop {
            if let Some(m) = self.get_front(host) {
                return m;
            }
            let _ = signal_wait_result(host.wait_signal(self, signal_wait_millis(BLOCKING_WAIT_TIMEOUT)));
        }
    }

    /// Slot 21: blocking get back.
    pub fn blocking_get_back<H: MessageHost + ?Sized>(&mut self, host: &mut H) -> Token {
        loop {
            if let Some(m) = self.get_back(host) {
                return m;
            }
            let _ = signal_wait_result(host.wait_signal(self, signal_wait_millis(BLOCKING_WAIT_TIMEOUT)));
        }
    }

    /// Slot 22: message count.
    pub fn count(&self) -> u32 {
        self.items.len() as u32
    }
}
