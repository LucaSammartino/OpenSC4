// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Logical reference events, not COM pointers or a Rust Drop policy (§2.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interface {
    Reference,
    Message,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    Count(u32),
    Delete,
}
#[derive(Default, Debug)]
pub struct References {
    count: u32,
}
impl References {
    pub fn count(&self) -> u32 {
        self.count
    }
    pub fn add_ref(&mut self) -> u32 {
        self.count = self.count.wrapping_add(1);
        self.count
    }
    pub fn remove_ref(&mut self) -> u32 {
        self.count = self.count.saturating_sub(1);
        self.count
    }
    /// Delete is an instruction to the logical owner, not implicit Shutdown.
    pub fn release(&mut self) -> Release {
        if self.count > 1 {
            self.count -= 1;
            Release::Count(self.count)
        } else {
            Release::Delete
        }
    }
    pub fn query(&mut self, ripple: bool, id: u32, output: &mut Option<Interface>) -> bool {
        let value = match id {
            1 => Interface::Reference,
            0x4522_94AA if ripple => Interface::Message,
            _ => return false,
        };
        *output = Some(value);
        self.add_ref();
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Initialized,
    AcquireTerrain,
    Subscribe,
    Unsubscribe,
    ReleaseTerrain,
    ReleaseBinding,
}
