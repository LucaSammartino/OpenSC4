// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The process-wide statics of the lot configuration manager (§B2).

use std::cell::RefCell;
use std::rc::Rc;

use crate::stl::{StlHashMap, StlHashSet};

/// The catalogue every lot configuration manager of a process shares.
#[derive(Clone, Debug, Default)]
pub struct LotConfigCatalogue {
    /// `0x00B216BC`.
    pub(crate) loaded: bool,
    /// `0x00B216C0`: the detail level the catalogue was built for.
    pub(crate) detail: u32,
    /// `0x00B216C8`: configuration ID → configuration (one reference each).
    pub(crate) configs: StlHashMap<u64>,
    /// `0x00B216DC`: every building family ID listed by an added
    /// configuration.
    pub(crate) families: StlHashSet,
    /// The unload callback is registered.
    pub(crate) callback_registered: bool,
}

/// The shared handle.
pub type SharedLotConfigCatalogue = Rc<RefCell<LotConfigCatalogue>>;

impl LotConfigCatalogue {
    pub fn new() -> LotConfigCatalogue {
        LotConfigCatalogue::default()
    }

    pub fn shared() -> SharedLotConfigCatalogue {
        Rc::new(RefCell::new(LotConfigCatalogue::new()))
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn detail(&self) -> u32 {
        self.detail
    }

    pub fn callback_registered(&self) -> bool {
        self.callback_registered
    }

    /// Slot 20: the map's entries `(id, configuration)` in bucket order.
    pub fn map(&self) -> Vec<(u32, u64)> {
        self.configs.iter().map(|(k, v)| (k, *v)).collect()
    }

    /// Slot 21: the family set in bucket order.
    pub fn family_set(&self) -> Vec<u32> {
        self.families.iter_order()
    }

    pub fn len(&self) -> usize {
        self.configs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }

    pub fn get(&self, id: u32) -> Option<u64> {
        self.configs.get(id).copied()
    }
}
