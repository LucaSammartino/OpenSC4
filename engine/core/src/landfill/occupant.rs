// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The landfill tile occupant `cSC4LandfillOccupant` (§2.3): its state, the
//! interface slots of §7.3 and the record of §6.4.
//!
//! It is a GZCOM class (`0x8A22BEBE`, factory `0x004776B0`) and an occupant, so
//! the **occupant manager** saves it as its own class record, not the landfill
//! developer (§1 "Owned objects"). Its base part is the `cSC4Occupant` record of
//! `docs/specs/lot-records.md` §2.3 (14 bytes + the exemplar holder).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::property::ExemplarHolder;
use crate::stream::{InStream, OutStream, SaveObject};

/// Class ID (`GetGZCLSID`, §7.3).
pub const CLSID_LANDFILL_OCCUPANT: u32 = 0x8A22_BEBE;
/// `cISC4LandfillOccupant` (§4.5.2 step 2, §7.3).
pub const IID_LANDFILL_OCCUPANT: u32 = 0x2A15_F262;
/// `cISC4Occupant`, at its second vtable (§7.3).
pub const IID_OCCUPANT: u32 = 0x26BB_767E;
/// `cIGZUnknown` (§7.3).
pub const IID_UNKNOWN: u32 = 0x0000_0001;
/// `cIGZSerializable` (§7.3, vtable `0x00A8CD84`).
pub const IID_SERIALIZABLE: u32 = 0xE4FD_A3D4;

/// The occupant type the constructor sets (§2.3).
pub const OCCUPANT_TYPE: u32 = 0x6A11_BD2B;
/// The occupant flag the constructor sets (§2.3, §7.3).
pub const CONSTRUCTOR_FLAG: u32 = 4;
/// The tile exemplar's default instance (§2.3, §4.1).
pub const DEFAULT_TILE_ID: u32 = 0x02F1_0000;
/// The tile exemplar's group and type (§2.3, §4.1).
pub const TILE_EXEMPLAR: (u32, u32) = (super::tile::TYPE, super::tile::GROUP);

/// Record versions (§6.4).
pub const OCCUPANT_RECORD_VERSION: u16 = 3;
/// The `cSC4Occupant` base record's version (lot-records §2.3).
pub const BASE_RECORD_VERSION: u16 = 4;

/// `cISC4LandfillOccupant` (§7.3, vtable `0x00A8CE28`, 10 slots). Slot 6's
/// return is an **SDK error**: the SDK declares `bool SetTileID(uint32)`, but
/// the game returns the **previous** tile ID in EAX (non-zero, so a bool-reading
/// caller sees true).
pub const OCCUPANT_SLOTS: [super::SlotRow; 10] = {
    use super::SlotReturn::*;
    [
        (0, "QueryInterface", 8, Al),
        (1, "AddRef", 0, Eax),
        (2, "Release", 0, Eax),
        (3, "Init", 0, Al),
        (4, "Shutdown", 0, Al),
        (5, "AsOccupant", 0, Eax),
        (6, "SetTileID", 4, Eax),
        (7, "GetOrientation", 0, Eax),
        (8, "SetOrientation", 4, Al),
        (9, "IsEmptyLandfillTile", 0, Al),
    ]
};

/// The base occupant state of lot-records §2.3, which §6.4 writes inline.
#[derive(Clone, Debug, Default)]
pub struct BaseOccupant {
    /// The removal counters word: low byte = holders of removal flag 2, high
    /// byte = holders of removal flag 1.
    pub removal: u16,
    /// `GetHighlight()` (`cISC4Occupant` slot 16). Always 0 in saves.
    pub highlight: u8,
    /// The status byte: `(highlight << 4) | 2` after a read (lot-records
    /// §2.3), with bit 0 = initialised and bit 1 = loaded.
    pub status: u8,
    /// The flag byte, written with bits 1 and 5 cleared (`& 0xDD`).
    pub flags: u8,
    /// The occupant type; [`OCCUPANT_TYPE`] for this class.
    pub occupant_type: u32,
    /// The occupant manager's spatial-index rectangle x1, z1, x2, z2
    /// (first byte `0xFF` = not indexed).
    pub cell_rect: [u8; 4],
    /// The exemplar property holder (foundations §10.3); its parent is the
    /// tile exemplar, attached by §4.8.1 step 3.
    pub holder: ExemplarHolder,
}

/// `ExemplarHolder` has no `PartialEq` (it holds an `Rc` to a shared exemplar),
/// so the base state compares the holder by its own properties and its parent
/// **key**: two occupants with the same tile attached are equal, whatever the
/// identity of the holder object.
impl PartialEq for BaseOccupant {
    fn eq(&self, other: &Self) -> bool {
        self.removal == other.removal
            && self.highlight == other.highlight
            && self.status == other.status
            && self.flags == other.flags
            && self.occupant_type == other.occupant_type
            && self.cell_rect == other.cell_rect
            && self.holder.own == other.holder.own
            && self.holder.parent_key() == other.holder.parent_key()
    }
}

/// The occupant's own state (§2.3).
#[derive(Clone, Debug, PartialEq)]
pub struct OccupantData {
    pub base: BaseOccupant,
    /// The landfill tile exemplar instance (§2.3, §4.1).
    pub tile_id: u32,
    /// The model rotation (§2.3, §7.3 slot 7).
    pub orientation: i32,
    /// The cell centre with the terrain altitude (§2.3).
    pub position: [f32; 3],
    /// `IsEmptyLandfillTile()` (§2.3, §7.3 slot 9). Nothing in the shipped code
    /// sets it true *[inferred]* (spec open question 7).
    pub is_empty: bool,
}

impl Default for OccupantData {
    /// The constructor's state (§2.3): occupant type [`OCCUPANT_TYPE`], flag 4
    /// set, the default tile, orientation 0, position (0, 0, 0), not empty.
    fn default() -> OccupantData {
        OccupantData {
            base: BaseOccupant {
                occupant_type: OCCUPANT_TYPE,
                flags: CONSTRUCTOR_FLAG as u8,
                ..BaseOccupant::default()
            },
            tile_id: DEFAULT_TILE_ID,
            orientation: 0,
            position: [0.0; 3],
            is_empty: false,
        }
    }
}

/// One landfill tile occupant (§2.3). A GZCOM object with one reference count;
/// the occupant manager saves it (§6.4). Interior mutability, like every core
/// `SaveObject`: a save may read it while the developer holds it.
pub struct LandfillOccupant {
    token: Cell<u64>,
    d: RefCell<OccupantData>,
}

impl LandfillOccupant {
    /// The constructor `cSC4LandfillOccupant(tileID, orientation)` (§2.3):
    /// occupant type [`OCCUPANT_TYPE`], flag 4 set, the given tile and
    /// orientation, position (0, 0, 0), not empty.
    pub fn new(tile_id: u32, orientation: i32) -> Rc<LandfillOccupant> {
        Rc::new(LandfillOccupant {
            token: Cell::new(0),
            d: RefCell::new(OccupantData {
                tile_id,
                orientation,
                ..OccupantData::default()
            }),
        })
    }

    /// A default-constructed occupant, as §6.2's zone read makes one before
    /// reading it (§6.4's `Read` fills the fields).
    pub fn empty() -> Rc<LandfillOccupant> {
        Rc::new(LandfillOccupant {
            token: Cell::new(0),
            d: RefCell::new(OccupantData::default()),
        })
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn snapshot(&self) -> OccupantData {
        self.d.borrow().clone()
    }

    /// `GetTileID` (§7.3 slot 6's stored value).
    pub fn tile_id(&self) -> u32 {
        self.d.borrow().tile_id
    }

    /// Slot 7 `GetOrientation()`.
    pub fn orientation(&self) -> i32 {
        self.d.borrow().orientation
    }

    /// Slot 8 `SetOrientation(o)`: stores it, returns true.
    pub fn set_orientation(&self, orientation: i32) -> bool {
        self.d.borrow_mut().orientation = orientation;
        true
    }

    /// Slot 6 `SetTileID(id)`: returns the **previous** tile ID *[compat]*. If
    /// `id ≠ tileID`, stores it and changes the **attached exemplar's** key
    /// instance to `id` (§7.3 *[quirk]*: it mutates a shared exemplar, so the
    /// host performs that part; the core records the new key on its own
    /// holder). No caller was found in the shipped code.
    pub fn set_tile_id(&self, id: u32) -> u32 {
        let mut d = self.d.borrow_mut();
        let previous = d.tile_id;
        if id != previous {
            d.tile_id = id;
            if let Some((key, _)) = d.base.holder.parent.as_mut() {
                key.instance = id;
            }
        }
        previous
    }

    /// Slot 9 `IsEmptyLandfillTile()`.
    pub fn is_empty_landfill_tile(&self) -> bool {
        self.d.borrow().is_empty
    }

    /// Stores the `isEmpty` flag (§4.8.1 step 3 sets it false explicitly after
    /// the tile is created). Nothing in the shipped code sets it true
    /// *[inferred]* (spec open question 7).
    pub fn set_empty(&self, empty: bool) {
        self.d.borrow_mut().is_empty = empty;
    }

    /// Slot 3 `Init()` (`cSC4Occupant::Init`): sets status bit 0. A loaded
    /// occupant keeps its bit 1 and is not announced as newly created
    /// (lot-records §2.4).
    pub fn init(&self) -> bool {
        let mut d = self.d.borrow_mut();
        d.base.status |= 1;
        true
    }

    /// Slot 4 `Shutdown()` (`cSC4Occupant::Shutdown`): clears status bit 0.
    pub fn shutdown(&self) -> bool {
        self.d.borrow_mut().base.status &= !1;
        true
    }

    /// Slot 29 of `cISC4Occupant` `SetFlag(f)` (§4.8.1 step 3 sets 4).
    pub fn set_flag(&self, flag: u32) {
        self.d.borrow_mut().base.flags |= flag as u8;
    }

    /// Slot 12 of `cISC4Occupant` `SetRemovalFlags(flags)` (§4.8.1 step 3
    /// passes 2): it counts holders of removal flags 1 and 2 in the base word.
    pub fn set_removal_flags(&self, flags: u32) {
        let mut d = self.d.borrow_mut();
        let mut word = d.base.removal;
        if flags & 1 != 0 {
            word = (word & 0x00FF) | ((word >> 8).saturating_add(1) << 8);
        }
        if flags & 2 != 0 {
            word = (word & 0xFF00) | u16::from((word as u8).saturating_add(1));
        }
        d.base.removal = word;
    }

    /// `SetPosition(p)` (slot 9 of `cISC4Occupant`): stores the position and
    /// recomputes the bounding box (§2.3). The box is not saved and no formula
    /// of this spec reads it, so it is derived on demand
    /// ([`LandfillOccupant::bounding_box`]).
    pub fn set_position(&self, position: [f32; 3]) {
        self.d.borrow_mut().position = position;
    }

    /// `GetPosition()` (slot 8 of `cISC4Occupant`).
    pub fn position(&self) -> [f32; 3] {
        self.d.borrow().position
    }

    /// The bounding box `(x−8, y, z−8) … (x+8, y+16, z+8)` metres, recomputed
    /// on `Read` and `SetPosition` (§2.3). Derived from the position, so it
    /// needs no stored state.
    pub fn bounding_box(&self) -> ([f32; 3], [f32; 3]) {
        let [x, y, z] = self.d.borrow().position;
        ([x - 8.0, y, z - 8.0], [x + 8.0, y + 16.0, z + 8.0])
    }

    /// Attaches the tile exemplar to the property holder (§4.8.1 step 3).
    pub fn attach_exemplar(&self, holder: ExemplarHolder) {
        self.d.borrow_mut().base.holder = holder;
    }

    /// `QueryInterface(iid)` (§7.3): the occupant interface, `cISC4Occupant`
    /// (at the second vtable) and `cIGZUnknown` give this object, plus
    /// `cIGZSerializable`. Anything else → false.
    pub fn answers_iid(&self, iid: u32) -> bool {
        matches!(
            iid,
            IID_UNKNOWN | IID_LANDFILL_OCCUPANT | IID_OCCUPANT | IID_SERIALIZABLE
        )
    }
}

impl SaveObject for LandfillOccupant {
    fn class_id(&self) -> u32 {
        CLSID_LANDFILL_OCCUPANT
    }

    /// §6.4. Returns whether the stream has no error afterwards.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        if out.error() != 0 {
            return false;
        }
        let d = self.d.borrow();
        out.put_u16(OCCUPANT_RECORD_VERSION);
        if !write_base(&d.base, out) {
            return false;
        }
        out.put_u32(d.tile_id);
        out.put_i32(d.orientation);
        out.put_f32(d.position[0]);
        out.put_f32(d.position[1]);
        out.put_f32(d.position[2]);
        out.put_bool(d.is_empty);
        out.error() == 0
    }

    /// §6.4: requires version 3 (and 4 for the base record), sets
    /// `isEmpty := (byte = 1)`, recomputes the bounding box, returns
    /// `!GetError()`.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.error() != 0 {
            return false;
        }
        let Some(version) = inp.get_u16() else { return false };
        if version != OCCUPANT_RECORD_VERSION {
            return false;
        }
        let Some(mut d) = (|| -> Option<OccupantData> {
            let base = read_base(inp)?;
            let mut d = OccupantData {
                base,
                tile_id: inp.get_u32()?,
                orientation: inp.get_i32()?,
                position: [inp.get_f32()?, inp.get_f32()?, inp.get_f32()?],
                // isEmpty := (byte = 1): only the byte 1 reads as true.
                is_empty: inp.get_u8()? == 1,
            };
            // A read occupant is "not yet initialised, loaded from a save"
            // (lot-records §2.3).
            d.base.status = (d.base.status & 1) | 2;
            Some(d)
        })() else {
            return false;
        };
        d.base.occupant_type = OCCUPANT_TYPE;
        *self.d.borrow_mut() = d;
        inp.error() == 0
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn std::any::Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}

/// The `cSC4Occupant` base block of lot-records §2.3: 14 bytes then the
/// exemplar holder.
fn write_base(b: &BaseOccupant, out: &mut dyn OutStream) -> bool {
    out.put_u16(BASE_RECORD_VERSION);
    out.put_u16(b.removal);
    out.put_u8(b.highlight);
    // Bits 1 and 5 are cleared on write (`& 0xDD`).
    out.put_u8(b.flags & 0xDD);
    out.put_u32(b.occupant_type);
    for &c in &b.cell_rect {
        out.put_u8(c);
    }
    b.holder.write(out)
}

fn read_base(inp: &mut dyn InStream) -> Option<BaseOccupant> {
    let version = inp.get_u16()?;
    if version != BASE_RECORD_VERSION {
        return None;
    }
    let mut b = BaseOccupant {
        removal: inp.get_u16()?,
        highlight: inp.get_u8()?,
        status: 0,
        flags: inp.get_u8()?,
        occupant_type: inp.get_u32()?,
        cell_rect: [inp.get_u8()?, inp.get_u8()?, inp.get_u8()?, inp.get_u8()?],
        holder: ExemplarHolder::default(),
    };
    // The highlight byte becomes the status byte as (b << 4) | 2.
    b.status = (b.highlight << 4) | 2;
    // The holder's parent exemplar key is resolved by the class-store loader,
    // not here: read it with an empty source, so an unresolvable parent is
    // simply absent (as for a freshly created occupant).
    // landfill §6.4 (LG33): the base ignores the holder's Boolean. A
    // rejected holder version leaves a good stream at its current cursor;
    // the landfill tail starts there, without resynchronisation.
    let _ = b.holder.read(inp, &NoExemplars);
    if inp.error() != 0 {
        return None;
    }
    Some(b)
}

/// An exemplar source with nothing in it: the base holder's parent key is
/// resolved by the class-store loader, not here.
struct NoExemplars;

impl crate::property::ExemplarSource for NoExemplars {
    fn exemplar(&self, _key: crate::stream::ResKey) -> Option<Rc<crate::property::PropertyHolder>> {
        None
    }
}

#[cfg(test)]
mod reader_tests {
    use super::*;
    use crate::stream::ByteReader;

    // Literal B from landfill-occupant-read-vectors §1, independent of Write.
    const RECORD: [u8; 57] = [
        3, 0, 4, 0, 2, 1, 0, 0xF4, 0x2B, 0xBD, 0x11, 0x6A, 0xFF, 0, 0, 0, 2, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0x10, 0, 0xF1, 2, 0xF9, 0xFF, 0xFF, 0xFF, 0, 0, 0xC4, 0xC1, 0, 0, 0x44, 0x41, 0, 0, 0x20, 0x42,
        0,
    ];

    #[test]
    fn rejected_holder_version_does_not_reject_landfill_tail() {
        // LG33 V0/V1/V3 and P1/P3: no resynchronisation at either boundary.
        for (offset, version, consumed, tile, z_bits) in [
            (16, 0, 39, 2, 0x0010_0000),
            (16, 1, 39, 2, 0x0010_0000),
            (16, 3, 39, 2, 0x0010_0000),
            (18, 1, 41, 0, 0x02F1_0010),
            (18, 3, 41, 0, 0x02F1_0010),
        ] {
            let mut bytes = RECORD;
            bytes[offset] = version;
            let mut input = ByteReader::new(&bytes);
            let occupant = LandfillOccupant::empty();
            assert!(occupant.read(&mut input));
            assert_eq!(input.position(), consumed);
            assert_eq!(input.error(), 0);
            assert_eq!(occupant.tile_id(), tile);
            assert_eq!(occupant.orientation(), 0);
            assert_eq!(occupant.position().map(f32::to_bits), [0, 0, z_bits]);
            assert!(!occupant.is_empty_landfill_tile());
            assert_eq!(occupant.bounding_box(), ([-8.0, 0.0, -8.0], [8.0, 16.0, 8.0]));
        }
    }

    #[test]
    fn holder_stream_error_still_rejects_landfill() {
        // Accepted holder versions followed by an incomplete count: sticky
        // stream error must not be ignored with the holder's Boolean.
        let mut input = ByteReader::new(&RECORD[..23]);
        assert!(!LandfillOccupant::empty().read(&mut input));
        assert_ne!(input.error(), 0);
    }
}
