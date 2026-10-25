// SPDX-License-Identifier: GPL-3.0-or-later
//! TE8 owned-action vector, including observable allocation/copy commit order.
use super::Action;
use std::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    DependencyStopped,
    InvalidVector { begin: u32, end: u32, capacity: u32 },
}

/// Addresses are controlled tokens, never dereferenced. A null allocation is
/// observed by range construction's skip-at-exactly-zero rule, not as an OOM
/// recovery policy. The environment can stop construction at invalid addresses.
pub trait Storage {
    fn allocate(&mut self, count: u32, bytes: u32) -> u32;
    fn construct(&mut self, address: u32, source: &Action) -> std::result::Result<Action, Failure>;
    fn destroy(&mut self, action: &Action);
    fn free(&mut self, address: u32);
}

struct Owned;
impl Storage for Owned {
    fn allocate(&mut self, _: u32, _: u32) -> u32 {
        0x1000
    }
    fn construct(&mut self, _: u32, source: &Action) -> std::result::Result<Action, Failure> {
        Ok(source.clone())
    }
    fn destroy(&mut self, _: &Action) {}
    fn free(&mut self, _: u32) {}
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct History {
    items: Vec<Action>,
    block: u32,
    invalid: Option<Failure>,
}

impl Deref for History {
    type Target = Vec<Action>;
    fn deref(&self) -> &Self::Target {
        assert!(self.invalid.is_none(), "TE8 invalid native vector is not empty history");
        &self.items
    }
}
impl DerefMut for History {
    fn deref_mut(&mut self) -> &mut Self::Target {
        assert!(self.invalid.is_none(), "TE8 invalid native vector is not empty history");
        &mut self.items
    }
}

impl History {
    pub fn push(&mut self, action: Action) {
        self.append_with(&action, &mut Owned)
            .expect("owned action storage returns normally");
    }

    pub fn invalid_state(&self) -> Option<Failure> {
        self.invalid
    }

    pub fn append_with(&mut self, action: &Action, storage: &mut impl Storage) -> std::result::Result<(), Failure> {
        if let Some(failure) = self.invalid {
            return Err(failure);
        }
        let n = self.items.len();
        if n < self.items.capacity() {
            let copy = storage.construct(self.block.wrapping_add((n as u32).wrapping_mul(60)), action)?;
            self.items.push(copy); // End advances only after construction returns.
            return Ok(());
        }
        self.grow_insert_with(n, std::slice::from_ref(action), storage)
    }

    /// TE8 range-insertion growth for a valid insertion point and a range that
    /// cannot fit in the existing storage. Spare-capacity append stays above.
    pub fn grow_insert_with(
        &mut self,
        index: usize,
        actions: &[Action],
        storage: &mut impl Storage,
    ) -> std::result::Result<(), Failure> {
        if let Some(failure) = self.invalid {
            return Err(failure);
        }
        let n = self.items.len();
        assert!(
            index <= n && n + actions.len() > self.items.capacity(),
            "TE8 valid range-growth call"
        );
        let capacity = n + n.max(actions.len());
        let block = storage.allocate(capacity as u32, (capacity as u32).wrapping_mul(60));
        // No local cleanup on a nonreturning construction: retain the new
        // allocation and constructed prefix, just as the native growth body.
        let mut copies = ManuallyDrop::new(Vec::with_capacity(capacity));
        let mut address = block;
        for source in self.items[..index].iter().chain(actions).chain(&self.items[index..]) {
            if address != 0 {
                copies.push(storage.construct(address, source)?);
            }
            address = address.wrapping_add(60);
        }
        for old in self.items.drain(..) {
            storage.destroy(&old);
            drop(old);
        }
        if self.block != 0 {
            storage.free(self.block);
        }
        self.block = block;
        self.items = ManuallyDrop::into_inner(copies);
        if block == 0 {
            let failure = Failure::InvalidVector {
                begin: 0,
                end: address,
                capacity: (capacity as u32).wrapping_mul(60),
            };
            self.invalid = Some(failure);
            return Err(failure);
        }
        Ok(())
    }
}
