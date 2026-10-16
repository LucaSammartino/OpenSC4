// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The process-wide prop catalogue (§A2, §A3) and its queries (§A5).

use std::cell::RefCell;
use std::rc::Rc;

use crate::property::PropertyHolder;
use crate::props;
use crate::stl::StlHashMap;
use crate::stream::ResKey;

use super::{class, prop, EXEMPLAR_TYPE_PROP};

/// The 0x1C-byte prop profile (§A3.2), kept as its bytes so that the
/// unbounded OccupantSize copy (quirk 8) and the one-byte wealth write are
/// exact.
///
/// | Offset | Content |
/// |---|---|
/// | +0x00 | occupant class ID |
/// | +0x04, +0x08, +0x0C | size x, y, z (floats) |
/// | +0x10 | requester satisfaction (u32) |
/// | +0x14 | wealth (u8; the other three bytes are not written) |
/// | +0x18 | crime effect (u32) |
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PropProfile {
    pub raw: [u8; 0x1C],
}

impl PropProfile {
    /// The dword at `index` (offset `4 × index`).
    pub fn word(&self, index: usize) -> u32 {
        let o = index * 4;
        u32::from_le_bytes([self.raw[o], self.raw[o + 1], self.raw[o + 2], self.raw[o + 3]])
    }

    fn set_word(&mut self, index: usize, v: u32) {
        let o = index * 4;
        self.raw[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// +0x00.
    pub fn class_id(&self) -> u32 {
        self.word(0)
    }

    /// +0x04, +0x08, +0x0C.
    pub fn size(&self) -> [f32; 3] {
        [
            f32::from_bits(self.word(1)),
            f32::from_bits(self.word(2)),
            f32::from_bits(self.word(3)),
        ]
    }

    /// +0x10.
    pub fn satisfaction(&self) -> u32 {
        self.word(4)
    }

    /// +0x14 (the byte).
    pub fn wealth(&self) -> u8 {
        self.raw[0x14]
    }

    /// +0x18.
    pub fn crime_effect(&self) -> u32 {
        self.word(6)
    }

    /// The profile `0x004A3DB0` builds from an exemplar (§A3.2): the
    /// defaults (class `0x2977AA47`, everything else 0), then each property
    /// that is present, in the table's order *[inferred]* (PL4). The size is
    /// copied value by value for as many values as it has (quirk 8): value
    /// `i` goes to the dword at `+0x04 + 4i`; values past the profile's end
    /// are dropped [choice].
    pub fn from_exemplar(h: &PropertyHolder) -> PropProfile {
        let mut p = PropProfile::default();
        p.set_word(0, class::DEFAULT_PROP);
        if let Some(c) = props::u32_of(h, prop::OCCUPANT_CLASS) {
            p.set_word(0, c);
        }
        if let Some(size) = props::f32s(h, prop::OCCUPANT_SIZE) {
            for (i, v) in size.iter().enumerate().take(6) {
                p.set_word(1 + i, v.to_bits());
            }
        }
        if let Some(v) = props::u32_of(h, prop::REQUESTER_SATISFACTION) {
            p.set_word(4, v);
        }
        if let Some(v) = props::u32_of(h, prop::WEALTH) {
            p.raw[0x14] = v as u8;
        }
        if let Some(v) = props::u32_of(h, prop::CRIME_EFFECT) {
            p.set_word(6, v);
        }
        // The name (0x20) is read and thrown away.
        p
    }
}

/// The process-wide catalogue of §A2: shared by every prop manager of a
/// process.
#[derive(Clone, Debug, Default)]
pub struct PropCatalogue {
    /// `0x00B21758`.
    built: bool,
    /// `0x00B21760`: prop type ID → resource key.
    types: StlHashMap<ResKey>,
    /// `0x00B21774`: family ID → member types in load order.
    families: StlHashMap<Vec<u32>>,
    /// `0x00B2179C`: prop type ID → profile.
    profiles: StlHashMap<PropProfile>,
    /// The name map: never filled (§A5 slot 12).
    names: StlHashMap<String>,
    /// `0x00B21770`: data2 of the catalogue message (PL2: 0).
    data2: u32,
    /// The shutdown callback was registered.
    callback_registered: bool,
}

/// The handle every prop manager of a process shares.
pub type SharedPropCatalogue = Rc<RefCell<PropCatalogue>>;

impl PropCatalogue {
    pub fn new() -> PropCatalogue {
        PropCatalogue::default()
    }

    /// A new shared, empty catalogue.
    pub fn shared() -> SharedPropCatalogue {
        Rc::new(RefCell::new(PropCatalogue::new()))
    }

    pub fn is_built(&self) -> bool {
        self.built
    }

    pub(crate) fn set_built(&mut self) {
        self.built = true;
    }

    pub fn callback_registered(&self) -> bool {
        self.callback_registered
    }

    pub(crate) fn set_callback_registered(&mut self) {
        self.callback_registered = true;
    }

    /// The value sent as data2 with the catalogue message.
    pub fn data2(&self) -> u32 {
        self.data2
    }

    /// Sets the value at `0x00B21770` (PL2; tests and tools).
    pub fn set_data2(&mut self, v: u32) {
        self.data2 = v;
    }

    /// Whether `StaticInit` catalogues an exemplar: its `ExemplarType` is
    /// 0x1E (§A3.1 step 2).
    pub fn is_prop_exemplar(h: &PropertyHolder) -> bool {
        props::u32_of(h, props::PROP_EXEMPLAR_TYPE) == Some(EXEMPLAR_TYPE_PROP)
    }

    /// `AddPropExemplarData` (`0x004A4210`) in "add" mode (§A3.1 step 2,
    /// §A3.2, §A3.3): the type ID is the key's instance.
    pub fn add_exemplar(&mut self, key: ResKey, h: &PropertyHolder) {
        let id = key.instance;
        *self.types.entry(id, || key) = key;
        let profile = PropProfile::from_exemplar(h);
        *self.profiles.entry(id, PropProfile::default) = profile;
        for f in props::u32s(h, prop::FAMILIES).unwrap_or_default() {
            let members = self.families.entry(f, Vec::new);
            if !members.contains(&id) {
                members.push(id);
            }
        }
    }

    /// The shutdown callback (`0x004A2A30`): tears the catalogue down.
    pub fn app_shutdown(&mut self) {
        *self = PropCatalogue::default();
    }

    // ---- queries (§A5) ----

    /// Slot 5: every type map key in bucket order.
    pub fn all_prop_types(&self) -> Vec<u32> {
        self.types.keys()
    }

    /// Slot 6: every family map key in bucket order.
    pub fn all_family_types(&self) -> Vec<u32> {
        self.families.keys()
    }

    /// Slot 7.
    pub fn key_of(&self, id: u32) -> Option<ResKey> {
        self.types.get(id).copied()
    }

    /// Slot 8.
    pub fn is_prop_type_valid(&self, id: u32) -> bool {
        self.types.contains(id)
    }

    /// Slot 9 (quirk 1): a family assigns its vector and returns its length;
    /// a type assigns `[id]` and returns 0; anything else returns 0 and
    /// leaves `vec` untouched.
    pub fn prop_types_by_family(&self, vec: &mut Vec<u32>, id: u32) -> u32 {
        if let Some(members) = self.families.get(id) {
            *vec = members.clone();
            members.len() as u32
        } else if self.types.contains(id) {
            *vec = vec![id];
            0
        } else {
            0
        }
    }

    /// The members of a family, in load order (no fallback).
    pub fn family(&self, id: u32) -> Option<&[u32]> {
        self.families.get(id).map(Vec::as_slice)
    }

    /// Slot 10.
    pub fn is_prop_in_family(&self, ty: u32, family: u32) -> bool {
        ty == family || self.families.get(family).is_some_and(|m| m.contains(&ty))
    }

    /// Slot 11.
    pub fn is_family_type_valid(&self, id: u32) -> bool {
        self.families.contains(id) || self.types.contains(id)
    }

    /// Slot 12 (quirk 3): inserts an empty name and returns it.
    pub fn prop_name(&mut self, id: u32) -> String {
        self.names.entry(id, String::new).clone()
    }

    /// Slot 13 (quirk 2): the profile, inserting an all-zero one for an
    /// unknown ID.
    pub fn prop_profile(&mut self, id: u32) -> PropProfile {
        *self.profiles.entry(id, PropProfile::default)
    }

    /// Slot 14: the profile or `None`.
    pub fn prop_profile_ptr(&self, id: u32) -> Option<PropProfile> {
        self.profiles.get(id).copied()
    }

    /// The number of entries in the type / family / profile maps.
    pub fn counts(&self) -> (usize, usize, usize) {
        (self.types.len(), self.families.len(), self.profiles.len())
    }
}
