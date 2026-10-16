// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The 83 leaf classes of `cSC4AudioCOMDirector` (`docs/specs/audio.md` §11).
//! Section references (`§n`) point at that spec.
//!
//! Their shapes, sizes, IIDs and capability slots are measured; what they *do*
//! is not (**AU5**). This module implements the measured surface only:
//! the three allocation shapes, the two always-true capability predicates, the
//! `QueryInterface` / `Release` protocol and the per-family refcount offsets.
//! Slot 5, the one real method of each class, is left unimplemented.
//!
//! See `# Choices` item 2 in [`crate::audio`].

/// The three allocation shapes of §11.1 (**measured**): 16 of the 83
/// factories allocate 8 bytes, 31 allocate 16 and 36 allocate 20.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeafShape {
    /// 8 bytes: `{vtable, refcount(+4)}` — 16 factories.
    Size8,
    /// 16 bytes: `{vtable, id(+4), flag u8(+8), refcount(+0x0C)}` — 31.
    Size16,
    /// 20 bytes: `{vtable, …, refcount(+0x10)}` — 36.
    Size20,
}

impl LeafShape {
    /// The allocation size of the shape (§11.1).
    pub fn size(self) -> usize {
        match self {
            LeafShape::Size8 => 8,
            LeafShape::Size16 => 16,
            LeafShape::Size20 => 20,
        }
    }

    /// The offset of the refcount, which differs per family (§11.1).
    pub fn refcount_offset(self) -> usize {
        match self {
            LeafShape::Size8 => 0x04,
            LeafShape::Size16 => 0x0C,
            LeafShape::Size20 => 0x10,
        }
    }

    /// The IID the family answers in addition to the common one (§11.1).
    pub fn second_iid(self) -> u32 {
        match self {
            LeafShape::Size8 => 0xEA40_074F,
            LeafShape::Size16 => 0xCA40_0753,
            LeafShape::Size20 => 0xCA40_0757,
        }
    }
}

/// The common IID shared by all three families (§11.1): `0x4A40074C`.
pub const LEAF_COMMON_IID: u32 = 0x4A40_074C;

/// The 83 registrations of §11.2, as `CLSID ← factory`. The factory addresses
/// group into the three shapes of §11.1.
pub const LEAF_REGISTRATIONS: [(u32, u32); 83] = [
    (0xCA42_6982, 0x004E_C6F0),
    (0x8A40_1E0F, 0x004E_C3D0),
    (0x2A42_6986, 0x004E_C3F0),
    (0x8A42_698A, 0x004E_C410),
    (0x6A42_698E, 0x004E_C430),
    (0x0A42_6991, 0x004E_C450),
    (0x6A42_6995, 0x004E_C470),
    (0x0A42_6998, 0x004E_C490),
    (0xAA54_E710, 0x004E_C4B0),
    (0xAA54_E715, 0x004E_C4D0),
    (0x0A54_E719, 0x004E_C4F0),
    (0x0B81_7C41, 0x004E_C510),
    (0xAB81_7C0B, 0x004E_C530),
    (0xCB81_7C1B, 0x004E_C550),
    (0x2B81_7C27, 0x004E_C570),
    (0x8B81_7C32, 0x004E_C590),
    (0x0A40_1BA0, 0x004E_C600),
    (0x4A40_1BA7, 0x004E_B260),
    (0xCA40_1BAE, 0x004E_B290),
    (0x6A40_1BB3, 0x004E_B2C0),
    (0x6A40_1BBA, 0x004E_B2F0),
    (0x0A42_B520, 0x004E_B320),
    (0x6A43_C75D, 0x004E_B350),
    (0xCA5A_6A5F, 0x004E_B380),
    (0x0A5A_6A64, 0x004E_B3B0),
    (0x2A5A_6A68, 0x004E_B3E0),
    (0x6A5A_6A6B, 0x004E_B410),
    (0x4A5A_6A6F, 0x004E_B440),
    (0x0A5A_6A72, 0x004E_B470),
    (0x0A5A_6A76, 0x004E_B4A0),
    (0xAA5A_6A7A, 0x004E_B4D0),
    (0xAA5A_6A7D, 0x004E_B500),
    (0xCA5A_6A81, 0x004E_B530),
    (0x2A5A_6A84, 0x004E_B560),
    (0x4A5A_6A88, 0x004E_B590),
    (0xEA5A_6A8B, 0x004E_B5C0),
    (0x6A5A_6A8F, 0x004E_B5F0),
    (0x4A5D_0EB0, 0x004E_B620),
    (0xAA5D_0EB5, 0x004E_B650),
    (0xEA5F_4954, 0x004E_B680),
    (0x8A5F_603E, 0x004E_B6B0),
    (0x8A60_F033, 0x004E_B6E0),
    (0x2A87_2395, 0x004E_B710),
    (0xCA9D_68A1, 0x004E_B740),
    (0xCA9D_68A5, 0x004E_B770),
    (0xAA9D_68A9, 0x004E_B7A0),
    (0x2AA1_7957, 0x004E_B7D0),
    (0x8A42_7F7C, 0x004E_C650),
    (0xEA42_98FC, 0x004E_B800),
    (0x2A42_9900, 0x004E_B830),
    (0x4A42_9903, 0x004E_B860),
    (0x2A42_9907, 0x004E_B890),
    (0xCA42_990A, 0x004E_B8C0),
    (0x8A53_E23E, 0x004E_B8F0),
    (0xCA53_E243, 0x004E_B920),
    (0x0A53_E247, 0x004E_B950),
    (0x4A53_E24B, 0x004E_B980),
    (0x2A53_E24F, 0x004E_B9B0),
    (0xCA53_E252, 0x004E_B9E0),
    (0x0A53_E290, 0x004E_BA10),
    (0x2A53_F0C3, 0x004E_BA40),
    (0x8A53_F0E4, 0x004E_BAA0),
    (0x4A53_F0E8, 0x004E_BAD0),
    (0x8A53_F584, 0x004E_BB00),
    (0xEA5A_6FCE, 0x004E_BB30),
    (0x8A5A_6FD1, 0x004E_BB60),
    (0xEA5A_6FD4, 0x004E_BB90),
    (0x6A5D_0682, 0x004E_BBC0),
    (0x0A5D_068D, 0x004E_BBF0),
    (0x2A5D_0691, 0x004E_BC20),
    (0xEA5F_495C, 0x004E_BC50),
    (0x4A60_D870, 0x004E_BC80),
    (0x8A60_D873, 0x004E_BCB0),
    (0x6A60_F166, 0x004E_BCE0),
    (0xEA80_835E, 0x004E_BD10),
    (0x0A80_8367, 0x004E_BD40),
    (0xCA80_836C, 0x004E_BD70),
    (0x2A80_8371, 0x004E_BDA0),
    (0x4A80_8375, 0x004E_BDD0),
    (0x4A80_84F4, 0x004E_BE00),
    (0x2A9D_6690, 0x004E_BE30),
    (0xAA9D_6694, 0x004E_BE60),
    (0x2A9D_6697, 0x004E_BE90),
];

/// The factory address ranges that identify each shape (§11.1, measured).
///
/// The 8-byte family is factories `0x004EC6F0` and `0x004EC3D0`–`0x004EC590`;
/// the 16-byte family `0x004EB260`–`0x004EB7D0` and `0x004EC600`; the 20-byte
/// family `0x004EC650` and `0x004EB800`–`0x004EBE90`.
pub fn shape_of_factory(factory: u32) -> LeafShape {
    match factory {
        0x004E_C6F0 => LeafShape::Size8,
        0x004E_C3D0..=0x004E_C590 => LeafShape::Size8,
        0x004E_B260..=0x004E_B7D0 => LeafShape::Size16,
        0x004E_C600 => LeafShape::Size16,
        0x004E_C650 => LeafShape::Size20,
        0x004E_B800..=0x004E_BE90 => LeafShape::Size20,
        // §11.1 names no other factory; treat an unknown one as the 16-byte
        // shape, which is the largest of the three "leaf" shapes that carries
        // the `id`/`flag` pair. [choice] AU5.
        _ => LeafShape::Size16,
    }
}

/// The counts §11.1 measures: 16 x 8 bytes, 31 x 16 bytes, 36 x 20 bytes.
pub const SHAPE_COUNTS: (usize, usize, usize) = (16, 31, 36);

/// A leaf object (§11.1): the factory allocates a fixed-size object from the
/// ordinary heap, writes a vtable and leaves the remaining fields zeroed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeafObject {
    /// The shape this object was allocated with.
    pub shape: LeafShape,
    /// `+0x04` of the 16-byte family: the id field.
    pub id: u32,
    /// `+0x08` of the 16-byte family: a `u8` flag.
    pub flag: u8,
    /// The refcount, at the family's offset.
    pub refcount: u32,
}

impl LeafObject {
    /// A factory's output: the vtable is written, the rest is zeroed (§11.1).
    /// The refcount starts at 0 and is brought to 1 by the COM factory
    /// protocol's `AddRef` (§11.1).
    pub fn from_factory(shape: LeafShape) -> Self {
        LeafObject {
            shape,
            id: 0,
            flag: 0,
            refcount: 0,
        }
    }

    /// The object size (§11.1).
    pub fn size(&self) -> usize {
        self.shape.size()
    }

    /// `AddRef` (§11.1).
    pub fn add_ref(&mut self) -> u32 {
        self.refcount = self.refcount.wrapping_add(1);
        self.refcount
    }

    /// `Release` (`0x004EC5B0` and its two siblings, §11.1): decrements only
    /// while the count is **greater than 1**; at `<= 1` it frees the object
    /// and returns 0. So the refcount is the number of live references and the
    /// object is destroyed on the last one *[compat]*.
    ///
    /// Returns `true` when the object was freed.
    pub fn release(&mut self) -> bool {
        if self.refcount > 1 {
            self.refcount -= 1;
            false
        } else {
            self.refcount = 0;
            true
        }
    }

    /// `QueryInterface` (§11.1): the two IIDs of the family both answer the
    /// object's **own** pointer — **no sub-object offset** — and call `AddRef`.
    /// Every other IID returns `false` without touching the out-parameter
    /// (conformance test 15).
    pub fn query_interface(&mut self, iid: u32) -> bool {
        if iid == LEAF_COMMON_IID || iid == self.shape.second_iid() {
            self.add_ref();
            true
        } else {
            false
        }
    }

    /// Slots 3 and 4 of every one of the 83 leaf vtables are the shared stub
    /// `0x00648F00` = `mov al, 1; ret`: predicates that **always return true**
    /// (§11.1, conformance test 14).
    pub fn capability_slot(&self, slot: u32) -> bool {
        matches!(slot, 3 | 4)
    }
}

/// Slot 5 is the only per-class behaviour (§11.1). Its role is **AU5**; this
/// module does not implement it.
pub const LEAF_UNIQUE_SLOT: u32 = 5;

#[cfg(test)]
mod tests {
    use super::*;

    /// Conformance test 14: 16 x 8 bytes, 31 x 16 bytes, 36 x 20 bytes.
    #[test]
    fn shape_counts_are_16_31_36() {
        assert_eq!(LEAF_REGISTRATIONS.len(), 83);
        let mut n8 = 0;
        let mut n16 = 0;
        let mut n20 = 0;
        for &(_, factory) in LEAF_REGISTRATIONS.iter() {
            match shape_of_factory(factory) {
                LeafShape::Size8 => n8 += 1,
                LeafShape::Size16 => n16 += 1,
                LeafShape::Size20 => n20 += 1,
            }
        }
        assert_eq!((n8, n16, n20), SHAPE_COUNTS, "8/16/20-byte factory counts");
    }

    /// Conformance test 14: the factory writes the vtable and zeroes the rest.
    #[test]
    fn factory_zeroes_the_fields() {
        for shape in [LeafShape::Size8, LeafShape::Size16, LeafShape::Size20] {
            let o = LeafObject::from_factory(shape);
            assert_eq!(o.size(), shape.size());
            assert_eq!(o.id, 0);
            assert_eq!(o.flag, 0);
            assert_eq!(o.refcount, 0);
        }
    }

    /// Conformance test 14: slots 3 and 4 return 1 (the shared stub
    /// `0x00648F00`).
    #[test]
    fn capability_slots_always_return_true() {
        let o = LeafObject::from_factory(LeafShape::Size16);
        assert!(o.capability_slot(3));
        assert!(o.capability_slot(4));
        assert!(!o.capability_slot(2));
        assert!(!o.capability_slot(5), "slot 5 is the per-class method");
    }

    /// Conformance test 15: QI answers both IIDs with the object's own pointer
    /// and AddRefs; anything else is refused without touching the out-param.
    #[test]
    fn query_interface_answers_two_iids() {
        let mut o = LeafObject::from_factory(LeafShape::Size16);
        o.add_ref(); // the COM factory protocol brings the count to 1.
        assert!(o.query_interface(LEAF_COMMON_IID));
        assert_eq!(o.refcount, 2);
        assert!(o.query_interface(0xCA40_0753));
        assert_eq!(o.refcount, 3);
        assert!(!o.query_interface(0x1234_5678));
        assert_eq!(o.refcount, 3, "a miss does not AddRef");
    }

    /// Conformance test 15: each family has its own second IID and refcount
    /// offset.
    #[test]
    fn family_iids_and_refcount_offsets() {
        assert_eq!(LeafShape::Size8.second_iid(), 0xEA40_074F);
        assert_eq!(LeafShape::Size16.second_iid(), 0xCA40_0753);
        assert_eq!(LeafShape::Size20.second_iid(), 0xCA40_0757);
        assert_eq!(LeafShape::Size8.refcount_offset(), 0x04);
        assert_eq!(LeafShape::Size16.refcount_offset(), 0x0C);
        assert_eq!(LeafShape::Size20.refcount_offset(), 0x10);

        for (shape, iid) in [(LeafShape::Size8, 0xEA40_074Fu32), (LeafShape::Size20, 0xCA40_0757)] {
            let mut o = LeafObject::from_factory(shape);
            o.add_ref();
            assert!(o.query_interface(iid), "{shape:?}");
        }
    }

    /// Conformance test 15: `Release` decrements while > 1 and frees at 1.
    #[test]
    fn release_frees_at_the_last_reference() {
        let mut o = LeafObject::from_factory(LeafShape::Size8);
        o.add_ref();
        o.add_ref();
        o.add_ref(); // 3 live references
        assert!(!o.release());
        assert_eq!(o.refcount, 2);
        assert!(!o.release());
        assert_eq!(o.refcount, 1);
        assert!(o.release(), "at 1 the object is freed");
        assert_eq!(o.refcount, 0);
    }

    /// §11.2: the 83 CLSIDs are distinct.
    #[test]
    fn registrations_are_distinct() {
        let mut seen = std::collections::BTreeSet::new();
        for &(clsid, _) in LEAF_REGISTRATIONS.iter() {
            assert!(seen.insert(clsid), "0x{clsid:08X} appears twice");
        }
    }
}
