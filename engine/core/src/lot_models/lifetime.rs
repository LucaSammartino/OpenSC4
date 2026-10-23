// SPDX-License-Identifier: GPL-3.0-or-later
use std::sync::Arc;

/// An adapter's stable owner identity. Arc protects the Rust callback object;
/// the explicit calls below account for the separate native owner reference.
pub trait ModelOwner: Send + Sync {
    fn add_ref(&self);
    fn release(&self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelInterface {
    Basic,
    Cell,
    Wall,
}

/// Only the basic fields and operations published in §1.1. No native layout,
/// transform, slot 11 or pool-allocation semantics are inferred (open LM2).
pub struct BasicModel {
    references: u16,
    owner: Option<Arc<dyn ModelOwner>>,
    custom_data: u32,
    properties: u32,
    flags: [u8; 2],
    scale: f32,
}

impl Default for BasicModel {
    fn default() -> Self {
        Self {
            references: 0,
            owner: None,
            custom_data: 0,
            properties: 2,
            flags: [0; 2],
            scale: 1.0,
        }
    }
}

impl BasicModel {
    pub fn reference_count(&self) -> u32 {
        u32::from(self.references)
    }

    pub fn add_ref(&mut self) -> u32 {
        self.references = self.references.wrapping_add(1);
        self.reference_count()
    }

    /// Borrowed, with no native AddRef. Adapters must reuse the same Arc for
    /// the same native pointer so pointer equality remains observable.
    pub fn owner(&self) -> Option<&Arc<dyn ModelOwner>> {
        self.owner.as_ref()
    }

    pub fn set_owner(&mut self, owner: Option<Arc<dyn ModelOwner>>) {
        let equal = match (&self.owner, &owner) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        if equal {
            return;
        }
        self.clear_owner();
        if let Some(new) = &owner {
            new.add_ref();
        }
        self.owner = owner;
    }

    fn clear_owner(&mut self) {
        if let Some(old) = self.owner.take() {
            old.release();
        }
    }

    pub fn custom_data(&self) -> u32 {
        self.custom_data
    }

    pub fn set_custom_data(&mut self, value: u32) {
        self.custom_data = value;
    }

    pub fn properties(&self) -> u32 {
        self.properties
    }

    pub fn initial_flags(&self) -> [u8; 2] {
        self.flags
    }

    pub fn initial_scale(&self) -> f32 {
        self.scale
    }
}

impl Drop for BasicModel {
    fn drop(&mut self) {
        self.clear_owner();
    }
}

/// Semantic QI/refcount operations. A terminal Release destroys retained
/// resources immediately; the Rust allocation remains for safe inspection.
/// Calling other native operations after terminal Release is outside contract.
pub trait LotModelLifetime {
    fn basic(&self) -> &BasicModel;
    fn basic_mut(&mut self) -> &mut BasicModel;
    fn owner_interface(&self) -> ModelInterface;
    fn destroy_resources(&mut self);

    fn add_ref(&mut self) -> u32 {
        self.basic_mut().add_ref()
    }

    fn query_interface(&mut self, iid: u32, output: &mut Option<ModelInterface>) -> bool {
        let interface = match iid {
            1 | 0x26A6_8449 => ModelInterface::Basic,
            super::CELL_TEXTURE_IID if self.owner_interface() == ModelInterface::Cell => ModelInterface::Cell,
            super::RETAINING_WALL_IID if self.owner_interface() == ModelInterface::Wall => ModelInterface::Wall,
            _ => return false, // Failure preserves the caller's output.
        };
        *output = Some(interface);
        self.add_ref();
        true
    }

    fn release(&mut self) -> u32 {
        let basic = self.basic_mut();
        if basic.references > 1 {
            basic.references -= 1;
            return basic.reference_count();
        }
        self.destroy_resources();
        self.basic_mut().clear_owner();
        0
    }
}
