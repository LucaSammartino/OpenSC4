// SPDX-License-Identifier: GPL-3.0-or-later
//! Resource keys (type, group, instance) and well-known type IDs.

use std::fmt;

/// A DBPF resource key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Tgi {
    pub type_id: u32,
    pub group: u32,
    pub instance: u32,
}

impl Tgi {
    pub const fn new(type_id: u32, group: u32, instance: u32) -> Self {
        Tgi {
            type_id,
            group,
            instance,
        }
    }

    /// All three words zero: "no key" (used for "no parent cohort").
    pub const fn is_zero(&self) -> bool {
        self.type_id == 0 && self.group == 0 && self.instance == 0
    }
}

impl fmt::Display for Tgi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08X}-{:08X}-{:08X}", self.type_id, self.group, self.instance)
    }
}

/// Exemplar resources.
pub const TYPE_EXEMPLAR: u32 = 0x6534_284A;
/// Cohort resources (exemplars that other exemplars name as their parent).
pub const TYPE_COHORT: u32 = 0x0534_2861;
/// Localized text.
pub const TYPE_LTEXT: u32 = 0x2026_960B;
/// The DIR record ("directory of compressed files"); its group and instance are fixed.
pub const TYPE_DIR: u32 = 0xE86B_1EEF;
/// Key of the DIR record.
pub const DIR_TGI: Tgi = Tgi::new(0xE86B_1EEF, 0xE86B_1EEF, 0x286B_1F03);
