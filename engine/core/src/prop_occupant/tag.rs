// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The lot tag (§5.3) and the PV2 unmatched-tag rule (`pd` §4.3 step 2.3.2).
//!
//! The lot tag at `+0x58` is the **configuration object's ID**
//! (`cSC4LotConfigurationObject` `+0x24`) of the object that caused the prop
//! to be placed (§5.3). `CreateLotProp` (lot-developer.md §6.5) and the network
//! lot manager's `CreateProp` (network-manager.md §8.6) both call `SetLotTag`
//! right after creating the prop.
//!
//! # PV2: what happens when the tag names nothing
//!
//! The prop developer's block update (`pd` §4.3 step 2.3.2) walks each cell's
//! props in list order and, for each **tagged** prop:
//!
//! 1. no lot tag (occupant slot 27 false) → skip the prop;
//! 2. `i :=` the first configuration object whose object ID equals the tag;
//!    **none → `k := k - 1`** and skip the prop *[quirk]*.
//!
//! So an unmatched tag does not merely fail to match: it leaves the block's
//! slot count **one lower**, which lowers the target `N` of step 4 and with it
//! the number of props the block wants. That is the PV2 quirk, reproduced
//! literally here by [`BlockSlots::see_tag`].
//!
//! # The corpus count
//!
//! **40,830 of 147,162 tagged lot props over the 421 saves under `Regions\`
//! are unmatched (27.7 %)**, split as
//!
//! | Category | Count |
//! |---|---|
//! | [`TagMatch::ConfigLacksTag`] — the lot has a configuration, but none of its objects names the tag | 40,794 |
//! | [`TagMatch::NoConfiguration`] — the lot has no configuration at all | 36 |
//! | [`TagMatch::UnknownConfiguration`] — the configuration's objects could not be read | 0 |
//!
//! Effect props are clean: none of the 948 effect props is unmatched. The rule
//! therefore fires on more than a quarter of the tagged lot props in the
//! corpus. This module classifies a tag into exactly those four outcomes so a
//! caller can reproduce the count and the quirk; see [`classify`].

use std::cell::Cell;

/// The empty tag: `+0x58`'s constructor default (§2.2, §5.3).
pub const NO_LOT_TAG: u32 = u32::MAX;

/// The PV2 corpus figures (`pd` §10 quirk 1, §5.3): over the 421 distinct
/// saves under `Regions\`, 40,830 of 147,162 tagged lot props are unmatched
/// (27.7 %). Effect props are clean. Recorded as constants so a test can check
/// that the categories sum to the total (test 24).
pub mod corpus {
    /// Tagged lot props in the corpus.
    pub const TAGGED: u64 = 147_162;
    /// Unmatched tagged lot props in total.
    pub const UNMATCHED: u64 = 40_830;
    /// On a lot whose configuration lacks the tag.
    pub const CONFIG_LACKS_TAG: u64 = 40_794;
    /// On a lot with no configuration.
    pub const NO_CONFIGURATION: u64 = 36;
    /// On a lot whose configuration could not be read.
    pub const UNKNOWN_CONFIGURATION: u64 = 0;
    /// The unmatched share, as the corpus reports it.
    pub const UNMATCHED_PERCENT: f64 = 27.7;
}

/// What a tagged prop's tag resolves to against a lot's configuration
/// (PV2, `pd` §4.3 step 2.3.2 step 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagMatch {
    /// The configuration's object array has an object whose ID equals the tag.
    /// `object_index` is the **first** such object in record order, which is
    /// the one step 2.3.2 uses.
    Matched {
        /// Index of the first object whose ID equals the tag.
        object_index: usize,
    },
    /// The lot has a configuration, but none of its objects names the tag
    /// (40,794 corpus props).
    ConfigLacksTag,
    /// The lot has no configuration at all (36 corpus props).
    NoConfiguration,
    /// The configuration's object array could not be read (0 corpus props).
    UnknownConfiguration,
}

impl TagMatch {
    /// Whether the tag named a configuration object.
    pub fn is_matched(self) -> bool {
        matches!(self, TagMatch::Matched { .. })
    }

    /// Whether this is one of the three unmatched outcomes PV2 counts.
    pub fn is_unmatched(self) -> bool {
        !self.is_matched()
    }
}

/// Classify one prop's tag against a lot's configuration.
///
/// `objects` is the configuration's object array in record order, or `None`
/// when the lot has no configuration or its array could not be read. Pass
/// `unknown_configuration: true` for the second case so the two corpus
/// categories stay distinct (they differ in count: 36 and 0).
///
/// A prop with **no** tag (`tag == `[`NO_LOT_TAG`]`) answers `None`: step
/// 2.3.2 step 1 skips such a prop before any lookup, so it is neither matched
/// nor unmatched (test 24).
pub fn classify(tag: u32, objects: Option<&[u32]>, unknown_configuration: bool) -> Option<TagMatch> {
    if tag == NO_LOT_TAG {
        return None;
    }
    Some(match objects {
        None if unknown_configuration => TagMatch::UnknownConfiguration,
        None => TagMatch::NoConfiguration,
        Some(ids) => match ids.iter().position(|&id| id == tag) {
            Some(object_index) => TagMatch::Matched { object_index },
            None => TagMatch::ConfigLacksTag,
        },
    })
}

/// The block's running slot accounting of `pd` §4.3 step 2.3.2, reduced to the
/// part the tag rule touches.
///
/// `k` counts the props the cell walk has kept. Step 2.3.2 step 2 decrements it
/// for an unmatched tag — the PV2 quirk — and step 2.3.8 increments it when a
/// prop is kept. The free-slot list is the other half of `slots` and is not
/// modelled here.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlockSlots {
    /// `k`: props kept so far (and the unmatched-tag decrements).
    kept: i64,
    /// How many unmatched tags have fired, for a shadow or a test to read.
    unmatched: u64,
}

impl BlockSlots {
    /// A fresh counter (step 2 initialises `k := 0`).
    pub fn new() -> Self {
        Self::default()
    }

    /// One prop seen by the cell walk. Answers the outcome the walk acts on.
    pub fn see_tag(&mut self, outcome: Option<TagMatch>) -> Option<TagMatch> {
        match outcome {
            // Step 2.3.2 step 2: none → `k := k - 1`, then next.
            Some(m) if m.is_unmatched() => {
                self.kept -= 1;
                self.unmatched += 1;
                Some(m)
            }
            other => other,
        }
    }

    /// Step 2.3.8: the prop is kept.
    pub fn keep(&mut self) {
        self.kept += 1;
    }

    /// `k`, which the unmatched tags have already lowered (test 24).
    pub fn kept(&self) -> i64 {
        self.kept
    }

    /// How many unmatched tags have fired.
    pub fn unmatched(&self) -> u64 {
        self.unmatched
    }

    /// Step 2.3.5: `slots := slots + opt + k`. The caller adds `opt` (the
    /// objects still open) to this to get the block's slot count.
    pub fn slots_with(&self, open_objects: u32) -> i64 {
        i64::from(open_objects) + self.kept
    }
}

/// A counter of the PV2 corpus categories, so a reader of the corpus can
/// reproduce the 40,830 / 40,794 / 36 / 0 split (§5.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TagTally {
    /// Tagged lot props seen.
    pub tagged: u64,
    /// [`TagMatch::Matched`].
    pub matched: u64,
    /// [`TagMatch::ConfigLacksTag`].
    pub config_lacks_tag: u64,
    /// [`TagMatch::NoConfiguration`].
    pub no_configuration: u64,
    /// [`TagMatch::UnknownConfiguration`].
    pub unknown_configuration: u64,
}

impl TagTally {
    /// A fresh tally.
    pub fn new() -> Self {
        Self::default()
    }

    /// Count one tagged prop.
    pub fn add(&mut self, outcome: TagMatch) {
        self.tagged += 1;
        match outcome {
            TagMatch::Matched { .. } => self.matched += 1,
            TagMatch::ConfigLacksTag => self.config_lacks_tag += 1,
            TagMatch::NoConfiguration => self.no_configuration += 1,
            TagMatch::UnknownConfiguration => self.unknown_configuration += 1,
        }
    }

    /// Every unmatched category summed: the corpus's 40,830.
    pub fn unmatched(&self) -> u64 {
        self.config_lacks_tag + self.no_configuration + self.unknown_configuration
    }

    /// The unmatched share as a percentage of `tagged`, or 0 when nothing was
    /// counted. The corpus reads 27.7 %.
    pub fn unmatched_percent(&self) -> f64 {
        if self.tagged == 0 {
            return 0.0;
        }
        (self.unmatched() as f64) * 100.0 / (self.tagged as f64)
    }
}

/// A small helper that owns one prop's tag and answers [`classify`] against a
/// caller-supplied configuration. Kept separate from [`crate::PropOccupant`] so
/// the PV2 rule can be exercised without a city (test 24).
pub struct TagLookup {
    tag: Cell<u32>,
}

impl Default for TagLookup {
    fn default() -> Self {
        Self::new()
    }
}

impl TagLookup {
    /// A prop with no tag.
    pub fn new() -> Self {
        Self {
            tag: Cell::new(NO_LOT_TAG),
        }
    }

    /// `SetLotTag`.
    pub fn set(&self, tag: u32) {
        self.tag.set(tag);
    }

    /// The tag at `+0x58`.
    pub fn get(&self) -> u32 {
        self.tag.get()
    }

    /// Occupant slot 27 `GetLotTag`: false when the prop has no tag (§5.3).
    pub fn has(&self) -> bool {
        self.tag.get() != NO_LOT_TAG
    }

    /// Classify against a configuration's object IDs (see [`classify`]).
    pub fn classify(&self, objects: Option<&[u32]>, unknown_configuration: bool) -> Option<TagMatch> {
        classify(self.tag.get(), objects, unknown_configuration)
    }
}
