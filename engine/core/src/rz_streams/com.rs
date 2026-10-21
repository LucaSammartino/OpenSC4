// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The 16-byte object both stream bases share (§1.1, §1.6, §5): reference
//! count, byte-order flag, counted user data; `QueryInterface`, `Release`,
//! `SetUserData`, `GetUserData`, `Init`, `Shutdown`.

use std::sync::Arc;

/// `cIGZUnknown`.
pub const IID_GZ_UNKNOWN: u32 = 0x0000_0001;
/// `cIGZOStream` (§1.6).
pub const IID_GZ_OSTREAM: u32 = 0xC51B_11CB;
/// `cIGZIStream` (§5).
pub const IID_GZ_ISTREAM: u32 = 0x051B_11BD;

/// A counted interface pointer held as user data (the SDK's `cIGZVariant*`).
pub trait CountedRef: Send + Sync {
    fn add_ref(&self) -> u32;
    fn release(&self) -> u32;
}

/// What `Release()` did (§1.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseOutcome {
    /// The count was at most 1: the object must be destroyed; EAX 0.
    Deleted,
    /// The count was decremented to this value.
    Remaining(u32),
}

impl ReleaseOutcome {
    /// EAX as the game returns it.
    pub fn value(self) -> u32 {
        match self {
            ReleaseOutcome::Deleted => 0,
            ReleaseOutcome::Remaining(n) => n,
        }
    }
}

/// `+0x04` count, `+0x08` littleEndian, `+0x0C` user data.
pub struct StreamCore {
    ref_count: u32,
    little_endian: bool,
    user_data: Option<Arc<dyn CountedRef>>,
    /// The second IID `QueryInterface` answers besides 1.
    iid: u32,
}

impl StreamCore {
    /// Constructor (§1.2 / §5): count 0, the flag, no user data.
    pub fn new(little_endian: bool, iid: u32) -> Self {
        StreamCore {
            ref_count: 0,
            little_endian,
            user_data: None,
            iid,
        }
    }

    pub fn little_endian(&self) -> bool {
        self.little_endian
    }

    pub fn ref_count(&self) -> u32 {
        self.ref_count
    }

    /// Slot 1: count + 1, the new count.
    pub fn add_ref(&mut self) -> u32 {
        self.ref_count = self.ref_count.wrapping_add(1);
        self.ref_count
    }

    /// Slot 2 (§1.6) *[quirk]*: at a count of at most 1 (also 0) the object is
    /// deleted without decrementing; otherwise the count drops by one. The
    /// caller destroys the owner on [`ReleaseOutcome::Deleted`] (dropping it
    /// runs the destructor, which releases the user data).
    pub fn release(&mut self) -> ReleaseOutcome {
        if self.ref_count <= 1 {
            ReleaseOutcome::Deleted
        } else {
            self.ref_count -= 1;
            ReleaseOutcome::Remaining(self.ref_count)
        }
    }

    /// Slot 0: IID 1 or the stream's own IID: AddRef and true (the caller
    /// stores `this` in `*out`); any other IID false, `*out` untouched.
    pub fn query_interface(&mut self, iid: u32) -> bool {
        if iid == IID_GZ_UNKNOWN || iid == self.iid {
            self.add_ref();
            true
        } else {
            false
        }
    }

    /// Slot 19: counted assignment. The same pointer changes nothing; else
    /// AddRef the new one, store it, release the old one.
    pub fn set_user_data(&mut self, p: Option<Arc<dyn CountedRef>>) {
        let same = match (&p, &self.user_data) {
            (Some(a), Some(b)) => Arc::as_ptr(a) as *const () == Arc::as_ptr(b) as *const (),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        if let Some(new) = &p {
            new.add_ref();
        }
        let old = std::mem::replace(&mut self.user_data, p);
        if let Some(old) = old {
            old.release();
        }
    }

    /// Slot 20: the stored pointer, without AddRef.
    pub fn user_data(&self) -> Option<&Arc<dyn CountedRef>> {
        self.user_data.as_ref()
    }

    /// Slot 22 `Init(littleEndian)`: sets the flag; AL 1.
    pub fn init(&mut self, little_endian: bool) -> bool {
        self.little_endian = little_endian;
        true
    }

    /// Slot 23 `Shutdown()`: nothing; AL 1.
    pub fn shutdown(&mut self) -> bool {
        true
    }
}

impl Drop for StreamCore {
    /// The destructor releases the user data if set (§1.2).
    fn drop(&mut self) {
        if let Some(old) = self.user_data.take() {
            old.release();
        }
    }
}
