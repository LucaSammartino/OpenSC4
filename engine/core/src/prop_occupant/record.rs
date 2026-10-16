// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The prop occupant's save record (§7), specified field by field in
//! `docs/specs/lot-records.md` §10.1. The occupant base block of field 2 is
//! [`crate::automata::OccupantBase`] (`lr` §2.3) — the same 14 bytes plus the
//! exemplar holder every occupant record carries — so it is not duplicated
//! here.
//!
//! # Layout (`lr` §10.1)
//!
//! | # | Field | Type | Notes |
//! |---|---|---|---|
//! | 1 | version | u16 | [`PROP_RECORD_VERSION`] written |
//! | 2 | occupant base | | `lr` §2.3, type [`OCCUPANT_TYPE_PROP`] |
//! | 3 | prop type ID | u32 | the **ID**, not the `+0x5C` index (§5.1) |
//! | 4 | bounds | box | 6 × f32 |
//! | 5 | orientation | s8 | 0…3 |
//! | 6 | state | u8 | 0…2 |
//! | 7 | time of day start, end | 2 × u8 | tenths of an hour |
//! | 8 | has date range | bool | |
//! | 9 | date range | 4 × u32 | only when #8 |
//! | 10 | chance | u8 | 1…100 |
//! | 11 | prop usage | u8 | 1, 2, 4 |
//! | 12 | lot tag | u32 | `0xFFFFFFFF` = none |
//! | 13 | timer bits | u8 | only when version > 5, else `0x0F` |
//!
//! # Quirks reproduced
//!
//! - **`Read` does not check the version** (quirk 5): any version is read with
//!   this layout. Field 13 is present only when the version is **greater** than
//!   5; otherwise the timer bits become `0x0F` (test 17).
//! - **A failing base read reports success** (`lr` §10.1): when the base block
//!   cannot be read, the method returns `GetError() = 0` without reading more,
//!   so it can return true. The caller sees [`ReadOutcome::BaseFailed`] and
//!   nothing else was consumed (test 16's negative half).
//! - **Field 3 saves the u32 ID, not the index** (§5.1): an unknown type is
//!   interned on read, so the table grows, and a save written after that still
//!   carries the same ID (tests 6, 18).
//!
//! # Choices (`[choice]`)
//!
//! - **`Write` is version 6 always** (§7): the core never writes a legacy
//!   record, so the `<= 5` path is read-only.
//! - **A state or chance outside the corpus range** is written as is; the
//!   ranges 0…2 and 1…100 are observations, not validation.

use crate::automata::OccupantBase as BaseRecord;
use crate::prop_occupant::{DateRange, PropOccupant, OCCUPANT_TYPE_PROP};
use crate::stream::{InStream, OutStream};

/// The occupant base block's version (`lr` §2.3).
pub use crate::automata::OCCUPANT_BASE_VERSION;

/// The version `Write` emits (§7 field 1).
pub const PROP_RECORD_VERSION: u16 = 6;
/// The version threshold for field 13: the timer bits are in the record only
/// when the version is **greater** than this (§7 field 13, quirk 5).
pub const TIMER_BITS_VERSION: u16 = 5;
/// The value the timer bits take when field 13 is absent (§7 field 13, test 17).
pub const LEGACY_TIMER_BITS: u8 = 0x0F;
/// The effect record's version (`lr` §10.2 field 1); `Read` accepts only this.
pub const EFFECT_RECORD_VERSION: u16 = 3;
/// `lr` §10.2 field 5: the unnamed u32, 0 in every corpus save.
pub const EFFECT_RECORD_TRAILER: u32 = 0;

/// Which version a read saw (§7 field 1, quirk 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordVersion {
    /// Version greater than 5: field 13 (timer bits) is present.
    WithTimerBits,
    /// Version 5 or below: field 13 is absent and the timer bits become
    /// [`LEGACY_TIMER_BITS`].
    WithoutTimerBits,
}

impl RecordVersion {
    /// The class of `version` (§7 field 13).
    pub fn of(version: u16) -> RecordVersion {
        if version > TIMER_BITS_VERSION {
            RecordVersion::WithTimerBits
        } else {
            RecordVersion::WithoutTimerBits
        }
    }

    /// Whether field 13 is present.
    pub fn has_timer_bits(self) -> bool {
        self == RecordVersion::WithTimerBits
    }
}

/// The 13 fields of `lr` §10.1 as data, independent of any live object.
#[derive(Clone, Debug, PartialEq)]
pub struct PropRecord {
    /// Field 2: the occupant base block.
    pub base: BaseRecord,
    /// Field 3: the prop type **ID** (§5.1).
    pub prop_type: u32,
    /// Field 4: the bounding box, `(min, max)`.
    pub bounds: ([f32; 3], [f32; 3]),
    /// Field 5: the orientation, 0…3.
    pub orientation: i8,
    /// Field 6: the state, 0…2.
    pub state: u8,
    /// Field 7: time-of-day start and end, tenths of an hour.
    pub time_of_day: (u8, u8),
    /// Field 8 / 9: the date range, present only when set.
    pub date_range: Option<DateRange>,
    /// Field 10: the chance, 1…100.
    pub chance: u8,
    /// Field 11: the prop usage, 1, 2 or 4.
    pub prop_usage: u8,
    /// Field 12: the lot tag; [`crate::prop_occupant::NO_LOT_TAG`] = none.
    pub lot_tag: u32,
    /// Field 13: the timer bits (absent below version 6).
    pub timer_bits: u8,
    /// The version field 1 was written with. `Write` always emits
    /// [`PROP_RECORD_VERSION`]; a read keeps what it saw (test 17).
    pub version: u16,
}

impl Default for PropRecord {
    fn default() -> Self {
        Self {
            base: BaseRecord::new(OCCUPANT_TYPE_PROP),
            prop_type: 0,
            bounds: ([0.0; 3], [0.0; 3]),
            orientation: 0,
            state: 0,
            time_of_day: (0, 0),
            date_range: None,
            chance: 100,
            prop_usage: 0,
            lot_tag: u32::MAX,
            timer_bits: 0,
            version: PROP_RECORD_VERSION,
        }
    }
}

impl PropRecord {
    /// A record from a live prop (§7). The base block is supplied because it
    /// lives on the occupant sub-object, not on [`PropOccupant`].
    pub fn of(prop: &PropOccupant, base: BaseRecord) -> PropRecord {
        let (min, max) = prop.bounding_box();
        PropRecord {
            base,
            prop_type: prop.get_prop_type(),
            bounds: (min, max),
            orientation: prop.get_orientation() as i8,
            state: prop.get_state() as u8,
            time_of_day: prop.time_of_day_bytes(),
            date_range: prop.simulator_date_range(),
            chance: prop.get_random_chance(),
            prop_usage: prop.get_prop_usage(),
            lot_tag: prop.lot_tag(),
            timer_bits: prop.get_prop_timer_bitfield() as u8,
            version: PROP_RECORD_VERSION,
        }
    }

    /// Which version this record is classed as (§7 field 13).
    pub fn version_class(&self) -> RecordVersion {
        RecordVersion::of(self.version)
    }

    /// The timer bits as a reader would resolve them (§7 field 13, quirk 5):
    /// field 13 when the version is greater than 5, else `0x0F`.
    pub fn effective_timer_bits(&self) -> u8 {
        if self.version_class().has_timer_bits() {
            self.timer_bits
        } else {
            LEGACY_TIMER_BITS
        }
    }

    /// Copy the record's fields into a live prop (the read path). Fields are
    /// stored as read (`lr` §2.3): no message is posted and no model is
    /// switched, because this is the record reader rather than the `SetState`
    /// slot.
    pub fn apply(&self, prop: &mut PropOccupant) {
        prop.set_prop_type(self.prop_type);
        prop.set_bounding_box(self.bounds.0, self.bounds.1);
        prop.set_orientation(i32::from(self.orientation));
        prop.restore_state(self.state);
        prop.set_time_of_day_bytes(self.time_of_day.0, self.time_of_day.1);
        prop.set_random_chance(self.chance);
        prop.set_prop_usage(self.prop_usage);
        prop.set_lot_tag(self.lot_tag);
        prop.set_prop_timer_bits(self.effective_timer_bits());
        if let Some(r) = self.date_range {
            prop.set_simulator_date_range(r);
        }
    }
}

/// What a record read produced. `lr` §10.1's "a failing base read returns
/// `GetError() = 0` without reading more (so it can return true)" is
/// [`ReadOutcome::BaseFailed`]: the method reported success but the payload
/// was never touched.
#[derive(Clone, Debug, PartialEq)]
pub enum ReadOutcome {
    /// The whole record was read.
    Ok(PropRecord),
    /// The base block failed; nothing after it was read, and the method
    /// returns **true** (quirk 5's companion in `lr` §10.1).
    BaseFailed,
    /// The stream ended inside the payload. The method returns false.
    Truncated,
}

impl ReadOutcome {
    /// Whether `Read` reports success (`lr` §10.1: both `Ok` and `BaseFailed`
    /// do).
    pub fn read_ok(&self) -> bool {
        !matches!(self, ReadOutcome::Truncated)
    }

    /// The record, when there is one.
    pub fn record(&self) -> Option<&PropRecord> {
        match self {
            ReadOutcome::Ok(r) => Some(r),
            _ => None,
        }
    }
}

/// `Write` (`0x005ECE90`), fields 1–13 of `lr` §10.1 in order. The version is
/// always [`PROP_RECORD_VERSION`] (§7).
pub fn write_prop_record(record: &PropRecord, out: &mut dyn OutStream) -> bool {
    out.put_u16(PROP_RECORD_VERSION);
    if !record.base.write(out) {
        return false;
    }
    out.put_u32(record.prop_type);
    out.mark("bounds");
    for v in record.bounds.0.iter().chain(record.bounds.1.iter()) {
        out.put_f32(*v);
    }
    out.put_i8(record.orientation);
    out.put_u8(record.state);
    out.put_u8(record.time_of_day.0);
    out.put_u8(record.time_of_day.1);
    match record.date_range {
        Some(r) => {
            out.put_bool(true);
            out.put_u32(r.interval);
            out.put_u32(r.duration);
            out.put_u32(r.start_day);
            out.put_u32(r.end_day);
        }
        None => {
            out.put_bool(false);
        }
    }
    out.put_u8(record.chance);
    out.put_u8(record.prop_usage);
    out.put_u32(record.lot_tag);
    out.put_u8(record.timer_bits);
    out.error() == 0
}

/// `Read` (`0x005EF360`), the mirror of [`write_prop_record`]. **The version
/// is not checked** (quirk 5): any version is read with this layout, and field
/// 13 is consumed only when it is greater than [`TIMER_BITS_VERSION`].
pub fn read_prop_record(inp: &mut dyn InStream) -> ReadOutcome {
    let Some(version) = inp.get_u16() else {
        return ReadOutcome::Truncated;
    };
    let Some(base) = BaseRecord::read(inp) else {
        // `lr` §10.1: returns GetError() = 0 without reading more.
        return ReadOutcome::BaseFailed;
    };
    let Some(prop_type) = inp.get_u32() else {
        return ReadOutcome::Truncated;
    };
    let mut lo = [0.0f32; 3];
    let mut hi = [0.0f32; 3];
    for slot in lo.iter_mut().chain(hi.iter_mut()) {
        let Some(v) = inp.get_f32() else {
            return ReadOutcome::Truncated;
        };
        *slot = v;
    }
    let (Some(orientation), Some(state), Some(tod0), Some(tod1)) =
        (inp.get_i8(), inp.get_u8(), inp.get_u8(), inp.get_u8())
    else {
        return ReadOutcome::Truncated;
    };
    let Some(has_range) = inp.get_bool() else {
        return ReadOutcome::Truncated;
    };
    let mut date_range = None;
    if has_range {
        let (Some(interval), Some(duration), Some(start_day), Some(end_day)) =
            (inp.get_u32(), inp.get_u32(), inp.get_u32(), inp.get_u32())
        else {
            return ReadOutcome::Truncated;
        };
        date_range = Some(DateRange {
            interval,
            duration,
            start_day,
            end_day,
        });
    }
    let (Some(chance), Some(prop_usage), Some(lot_tag)) = (inp.get_u8(), inp.get_u8(), inp.get_u32()) else {
        return ReadOutcome::Truncated;
    };
    // Field 13 only when version > 5 (§7 field 13, quirk 5).
    let version_class = RecordVersion::of(version);
    let timer_bits = if version_class.has_timer_bits() {
        match inp.get_u8() {
            Some(b) => b,
            None => return ReadOutcome::Truncated,
        }
    } else {
        LEGACY_TIMER_BITS
    };
    ReadOutcome::Ok(PropRecord {
        base,
        prop_type,
        bounds: (lo, hi),
        orientation,
        state,
        time_of_day: (tod0, tod1),
        date_range,
        chance,
        prop_usage,
        lot_tag,
        timer_bits,
        version,
    })
}

/// The unnamed pair of `lr` §10.2 field 3. The corpus holds `(1, 0)`, `(1, 1)`
/// and `(0, 1)` only; the core keeps whatever is read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EffectRecordHead {
    /// The first unnamed byte.
    pub byte0: u8,
    /// The second unnamed byte.
    pub byte1: u8,
}

/// `lr` §10.2: the visual-effect record. Field 2 is a **complete** §10.1 prop
/// record with its own version 6.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectRecord {
    /// Field 1: always [`EFFECT_RECORD_VERSION`] on write; anything else is
    /// refused on read.
    pub version: u16,
    /// Field 2: the embedded prop record.
    pub prop: PropRecord,
    /// Field 3: the unnamed pair.
    pub head: EffectRecordHead,
    /// Field 4: the effect name. **Dropped to empty** while the effect is live
    /// (the record quirk, test 22).
    pub name: Vec<u8>,
    /// Field 5: the unnamed u32, [`EFFECT_RECORD_TRAILER`] in every corpus
    /// save; written as 0 while the effect is live.
    pub trailer: u32,
}

/// `Write` of the effect record (`lr` §10.2). `live` is what
/// [`crate::prop_occupant::VisualEffectProp::should_drop_name_on_write`]
/// answers: while it is true the name and the trailing u32 are written as
/// empty and 0 (the record quirk).
pub fn write_effect_record(record: &EffectRecord, live: bool, out: &mut dyn OutStream) -> bool {
    out.put_u16(EFFECT_RECORD_VERSION);
    if !write_prop_record(&record.prop, out) {
        return false;
    }
    out.put_u8(record.head.byte0);
    out.put_u8(record.head.byte1);
    if live {
        out.put_str(&[]);
        out.put_u32(EFFECT_RECORD_TRAILER);
    } else {
        out.put_str(&record.name);
        out.put_u32(record.trailer);
    }
    out.error() == 0
}

/// `Read` of the effect record (`lr` §10.2): any version other than
/// [`EFFECT_RECORD_VERSION`] fails.
pub fn read_effect_record(inp: &mut dyn InStream) -> Option<EffectRecord> {
    let version = inp.get_u16()?;
    if version != EFFECT_RECORD_VERSION {
        return None;
    }
    let prop = match read_prop_record(inp) {
        ReadOutcome::Ok(r) => r,
        // The embedded prop record is read in full; a failed base read leaves
        // the rest of the effect record unconsumed, so the outer read fails.
        _ => return None,
    };
    let head = EffectRecordHead {
        byte0: inp.get_u8()?,
        byte1: inp.get_u8()?,
    };
    let name = inp.get_str()?;
    let trailer = inp.get_u32()?;
    Some(EffectRecord {
        version,
        prop,
        head,
        name,
        trailer,
    })
}
