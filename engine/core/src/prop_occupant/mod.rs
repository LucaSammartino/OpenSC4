// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The in-world prop occupant (`cSC4PropOccupant`, class `0x2977AA47`),
//! `docs/specs/prop-occupant.md`. Section references (`§n`) point at that
//! spec; `nt §13` is `docs/specs/network-tools.md` §13, `lr §10.1` is
//! `docs/specs/lot-records.md` §10.1 and `pd §4.3` is
//! `docs/specs/prop-developer.md` §4.3.
//!
//! This is the object the game places, moves, draws, saves and removes — as
//! opposed to the prop *catalogue* and *manager* (`crate::prop_manager`) and
//! the prop *developer* (`crate::prop_developer`), which decide **which** prop
//! to place. Nothing here duplicates the catalogue: the prop type is a
//! **u16 index into a process-wide ID table** (§5.1), not an exemplar key.
//!
//! # Structure
//!
//! - `mod.rs`: identity (§1), the object layout (§2), the six sub-objects
//!   (§2.3–§2.4), the 24-slot `cISC4PropOccupant` table **with its measured
//!   return channels** (§4.1) and the slot bodies, the spatial record (§5) and
//!   the drawing hooks (§6.1).
//! - `record.rs`: the save record (§7, `lr` §10.1) and the class-object store
//!   framing (§7, foundations §3.3–§3.4).
//! - `tag.rs`: the lot tag (§5.3) and the PV2 tag-matching rule
//!   (`pd` §4.3 step 2.3.2).
//! - `effect.rs`: `cSC4PropOccupantVisualEffect` (§6.2, `nt` §13.2).
//! - `tests.rs`: a fake game and the §11 conformance tests.
//!
//! # The object
//!
//! **Every field offset in the spec is from the object base**, which *is* the
//! `cISC4PropOccupant` interface pointer (§2.1). `this + 4` is a different
//! interface, `cIGZMessageTarget2`. The layout is `0x68` bytes for the plain
//! class: six interface pointers across the first `0x28` bytes, the reference
//! count at `+0x14`, and the real data in `+0x3C`…`+0x64` (§2.2).
//!
//! # Return channels (§4.1, §8 item 2)
//!
//! `docs/abi/interfaces.tsv` labels all 24 rows `eax-or-void`. The measured
//! channels differ, and two of them matter to a caller:
//!
//! - `GetState` (slot 12) is **`eax`** — `movzx eax, byte [ecx+0x63]` — a clean
//!   zero-extended value.
//! - `GetRandomChance` (slot 18) is **`al`** — `mov al, byte [ecx+0x62]` — the
//!   upper 24 bits of EAX are **undefined**. `[choice]` FreeCity4 zero-extends
//!   (quirk 3), so the core returns a `u8` and every consumer masks to `0xFF`.
//! - `GetTimeOfDayRange` (slot 14) is **`void`**, not `st0`: its two `fstp`s
//!   pop, so ST(0) is empty at return (§8 item 3). `SetTimeOfDayRange` is the
//!   same.
//!
//! [`PROP_OCCUPANT_VTABLE`] is the table, and [`RetChannel`] is the vocabulary.
//! A C host that reads a slot's result must match that row; where the core
//! exposes a `bool` from a slot whose channel is not `al`, the call site is
//! marked `// [abi-ret]` and cites this section (see `engine/tests/check_ret_channels.py`).
//!
//! # Choices (`[choice]`) and literal readings
//!
//! - **Slot 5** (`0x004E13E0`) is `xor eax, eax; ret`: a genuine 0 in EAX, not
//!   a no-op. **Slot 6** (`0x00648F10`) is `ret 4` alone: a true no-op. The two
//!   adjacent slots have opposite channels (quirk 9).
//! - **Slot 8** (`SetPropType`) is the one slot the spec leaves `eax-or-void`:
//!   EAX holds the interned index at return "but nothing constructs it as a
//!   return value — implement as `void` unless a caller proves otherwise"
//!   (§4.1). `[choice]` FreeCity4 implements it as `void`. Question 4 below.
//! - **`GetRandomChance` / `GetPropUsage`** return `al` (quirks 3, 11);
//!   `[choice]` the core zero-extends.
//! - **`SetOrientation`** clamps to 0…3 (§5.2), so a caller cannot store 4 or a
//!   negative value; the clamped value is what `GetOrientation` reports and
//!   what field 5 of the record holds.
//! - **Threading** (foundations §19.3, HANDOFF §5.1): every field a getter
//!   reads is a `Cell`, so a getter never takes an interior borrow and is safe
//!   against a concurrent writer in `OnTick`. Structural state only the
//!   mutating slots touch (the `+0x54` date-range sub-record) is a `RefCell`;
//!   the core is main-thread only for those, as the other cores are.
//! - **`Read` before `Init`** (HANDOFF §5.1b): the record reader sizes nothing
//!   from the city and owns no host reference.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

mod effect;
mod record;
mod tag;

#[cfg(test)]
mod tests;

pub use effect::{
    EffectHost, VisualEffectProp, IID_VISUAL_EFFECT, OFF_EFFECT_CTOR_FLAG, OFF_EFFECT_ID, OFF_EFFECT_NAME,
    OFF_EFFECT_OBJECT, OFF_EFFECT_OCCUPANT, OFF_EFFECT_PROP, OFF_EFFECT_SCRATCH, OFF_EFFECT_STARTED,
};
pub use record::{
    read_effect_record, read_prop_record, write_effect_record, write_prop_record, EffectRecord, EffectRecordHead,
    PropRecord, ReadOutcome, RecordVersion, EFFECT_RECORD_TRAILER, EFFECT_RECORD_VERSION, LEGACY_TIMER_BITS,
    OCCUPANT_BASE_VERSION, PROP_RECORD_VERSION,
};
pub use tag::{classify, corpus, BlockSlots, TagLookup, TagMatch, TagTally, NO_LOT_TAG};

/// `cSC4PropOccupant` (§1).
pub const CLSID_PROP_OCCUPANT: u32 = 0x2977_AA47;
/// `cSC4PropOccupantTerrainDecal` (§1.2).
pub const CLSID_PROP_TERRAIN_DECAL: u32 = 0x2977_AA48;
/// `cSC4PropOccupantVisualEffect` (§1.2, §6.2).
pub const CLSID_PROP_VISUAL_EFFECT: u32 = 0x2977_AA49;
/// `cSC4PropOccupantBurntBuilding` (§1.2).
pub const CLSID_PROP_BURNT_BUILDING: u32 = 0xC9EF_B016;
/// `cSC4PropOccupantAutomataController` (§1.2; occurs in no save).
pub const CLSID_PROP_AUTOMATA_CONTROLLER: u32 = 0x2A40_17A0;

/// `cISC4PropOccupant`, IID `0xE9793A65`, the primary interface (§1).
pub const IID_PROP_OCCUPANT: u32 = 0xE979_3A65;
/// `cIGZMessageTarget2`, answered at `this + 4` (§1.1).
pub const IID_MESSAGE_TARGET2: u32 = 0x4522_94AA;
/// `cIGZUnknown`, which the occupant base answers (§1.1).
pub const IID_UNKNOWN: u32 = 0x0000_0001;

/// The occupant type `QueryInterface` also accepts as an IID: the class ID
/// itself (§1.1).
pub const OCCUPANT_TYPE_PROP: u32 = 0xA823_821E;

/// Object sizes by occupant class (§1.2, confirmed by the pool strides of
/// §4.2). `CreateProp` allocates one of these when GZCOM has no registered
/// class for the profile's occupant class ID.
pub const OBJECT_SIZE: usize = 0x68;
/// Size of `cSC4PropOccupantTerrainDecal` (§1.2).
pub const OBJECT_SIZE_TERRAIN_DECAL: usize = 0xA0;
/// Size of `cSC4PropOccupantVisualEffect` (§1.2).
pub const OBJECT_SIZE_VISUAL_EFFECT: usize = 0x94;
/// Size of `cSC4PropOccupantBurntBuilding` (§1.2).
pub const OBJECT_SIZE_BURNT_BUILDING: usize = 0xD8;

/// `CreateOccupant` (lot-records.md §2.4): posted by the occupant base's `Init`
/// only when the occupant was **not** loaded from a save (test 20).
pub const MSG_CREATE_OCCUPANT: u32 = 0x99EF_1140;
/// The message `SetState` posts when the occupant is in the city (nt §13.1).
pub const MSG_PROP_STATE_CHANGED: u32 = 0xE9C8_6DDB;
/// The exemplar property that gates the model-state switch in `SetState`
/// (nt §13.1, quirk 6).
pub const PROP_MODEL_STATE: u32 = 0x4A70_D491;

// ---- object layout (§2.2) ---------------------------------------------------

/// `cISC4PropOccupant` vtable pointer; **the object base** (§2.1).
pub const OFF_PROP_OCCUPANT: usize = 0x00;
/// `cIGZMessageTarget2` sub-object (§2.3).
pub const OFF_MESSAGE_TARGET: usize = 0x04;
/// `cISC4Occupant` sub-object (§2.3); `AsOccupant` returns `this + 8`.
pub const OFF_OCCUPANT: usize = 0x08;
/// `cIGZSerializable` sub-object (§2.3).
pub const OFF_SERIALIZABLE: usize = 0x0C;
/// `cSCExemplarPropertyHolder` sub-object (§2.3).
pub const OFF_PROPERTY_HOLDER: usize = 0x10;
/// The **reference count** (§2.2, quirk 10): `AddRef`/`Release` are
/// this-adjusting thunks that do `add ecx, 8` first, so a naive refcount of
/// `this` lands on `+0x04` and corrupts the message-target vptr.
pub const OFF_REFCOUNT: usize = 0x14;
/// The sixth, unidentified sub-object (§2.4).
pub const OFF_AUX: usize = 0x24;
/// The occupant type `0xA823821E`, written by the constructor (§2.2).
pub const OFF_OCCUPANT_TYPE: usize = 0x2C;
/// The bounding box: 6 × f32, min x,y,z then max x,y,z (§2.2).
pub const OFF_BBOX: usize = 0x3C;
/// The owned date-range sub-object pointer (§2.2, slots 15/16).
pub const OFF_DATE_RANGE: usize = 0x54;
/// The lot tag, u32, `0xFFFFFFFF` = none (§2.2, §5.3).
pub const OFF_LOT_TAG: usize = 0x58;
/// The prop type **index**, u16, into the process-wide ID table (§5.1).
pub const OFF_PROP_TYPE_INDEX: usize = 0x5C;
/// Time-of-day start, in tenths of an hour (§2.2).
pub const OFF_TOD_START: usize = 0x5E;
/// Time-of-day end, in tenths of an hour (§2.2).
pub const OFF_TOD_END: usize = 0x5F;
/// The timer bits; the low nibble is a 4-flag set (§2.2, nt §13.1).
pub const OFF_TIMER_BITS: usize = 0x60;
/// The orientation, 0…3 (§2.2, §5.2).
pub const OFF_ORIENTATION: usize = 0x61;
/// The random chance, percent (§2.2).
pub const OFF_RANDOM_CHANCE: usize = 0x62;
/// The state (§2.2).
pub const OFF_STATE: usize = 0x63;
/// The prop usage: 1 city, 2 lot, 3 building, 4 street (§2.2).
pub const OFF_PROP_USAGE: usize = 0x64;

// ---- slot table (§4.1) ------------------------------------------------------

/// What a compiled slot leaves behind. The vocabulary is §4.1's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetChannel {
    /// A genuine value in EAX: a pointer, a u32, or a byte the code widens
    /// with `movzx eax`.
    Eax,
    /// Only AL is defined; the upper 24 bits of EAX are **undefined**. Do not
    /// widen to a bool (§4.1).
    Al,
    /// No defined return value, and the x87 stack is empty at return (§8
    /// item 3). `void` does not mean "x87 empty by accident".
    Void,
    /// A tail call: the value is whatever the callee returns.
    ViaCall,
    /// Measured as "EAX holds something at return but nothing constructs it as
    /// a return value" — the one ambiguous slot (§4.1, slot 8).
    EaxOrVoid,
}

/// One compiled slot of `cISC4PropOccupant` (§4.1). `slot` is the compiled
/// index; the SDK's own index is `slot - 3` (nt §13.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotSpec {
    /// Compiled slot index, 0…23.
    pub slot: u8,
    /// Byte offset of the vtable entry from the vtable base.
    pub offset: u32,
    /// The SDK's method index (`slot - 3`).
    pub sdk_index: u8,
    /// The Windows address of the implementing body.
    pub address: u32,
    /// The method's behaviour name. Slots the ABI rows name from an
    /// identical-code fold keep the name the behaviour gives them (§8 item 4).
    pub method: &'static str,
    /// Dwords the callee pops after `this`.
    pub callee_pop: u8,
    /// Stack dwords beyond `this`.
    pub arg_dwords: u8,
    /// The measured return channel (§4.1).
    pub ret: RetChannel,
    /// Where the behaviour is specified.
    pub note: &'static str,
}

impl SlotSpec {
    /// The SDK's method index (compiled slot minus 3).
    pub const fn sdk_index(&self) -> u8 {
        self.sdk_index
    }
}

/// The whole `cISC4PropOccupant` interface: **exactly 24 slots**, `+0x00`…
/// `+0x5F` (§4.1, §4.2). What follows in the image is a pool/factory method
/// table at `0x00AA4960` and then the burnt-building subclass's vtables —
/// not part of this interface (test 4).
pub const PROP_OCCUPANT_VTABLE: [SlotSpec; 24] = [
    SlotSpec {
        slot: 0,
        offset: 0x00,
        sdk_index: 0,
        address: 0x005E_C960,
        method: "QueryInterface",
        callee_pop: 8,
        arg_dwords: 2,
        ret: RetChannel::Al,
        note: "1.1",
    },
    SlotSpec {
        slot: 1,
        offset: 0x04,
        sdk_index: 0,
        address: 0x0064_91C0,
        method: "AddRef",
        callee_pop: 0,
        arg_dwords: 0,
        ret: RetChannel::ViaCall,
        note: "2.2/quirk 10",
    },
    SlotSpec {
        slot: 2,
        offset: 0x08,
        sdk_index: 0,
        address: 0x0064_91D0,
        method: "Release",
        callee_pop: 0,
        arg_dwords: 0,
        ret: RetChannel::ViaCall,
        note: "2.2/quirk 10",
    },
    SlotSpec {
        slot: 3,
        offset: 0x0C,
        sdk_index: 0,
        address: 0x006C_2040,
        method: "AsOccupant",
        callee_pop: 0,
        arg_dwords: 0,
        ret: RetChannel::Eax,
        note: "2.3",
    },
    SlotSpec {
        slot: 4,
        offset: 0x10,
        sdk_index: 1,
        address: 0x0045_8C60,
        method: "SetBoundingBox",
        callee_pop: 8,
        arg_dwords: 2,
        ret: RetChannel::Void,
        note: "2.2",
    },
    SlotSpec {
        slot: 5,
        offset: 0x14,
        sdk_index: 2,
        address: 0x004E_13E0,
        method: "(stub returning 0)",
        callee_pop: 0,
        arg_dwords: 0,
        ret: RetChannel::Eax,
        note: "4.1/quirk 9",
    },
    SlotSpec {
        slot: 6,
        offset: 0x18,
        sdk_index: 3,
        address: 0x0064_8F10,
        method: "(no-op stub)",
        callee_pop: 4,
        arg_dwords: 1,
        ret: RetChannel::Void,
        note: "4.1/quirk 9",
    },
    SlotSpec {
        slot: 7,
        offset: 0x1C,
        sdk_index: 4,
        address: 0x005E_D280,
        method: "GetPropType",
        callee_pop: 0,
        arg_dwords: 0,
        ret: RetChannel::Eax,
        note: "5.1",
    },
    SlotSpec {
        slot: 8,
        offset: 0x20,
        sdk_index: 5,
        address: 0x005E_F4C0,
        method: "SetPropType",
        callee_pop: 4,
        arg_dwords: 1,
        ret: RetChannel::EaxOrVoid,
        note: "4.1 [choice] void",
    },
    SlotSpec {
        slot: 9,
        offset: 0x24,
        sdk_index: 6,
        address: 0x005E_C9D0,
        method: "GetOrientation",
        callee_pop: 0,
        arg_dwords: 0,
        ret: RetChannel::Eax,
        note: "5.2",
    },
    SlotSpec {
        slot: 10,
        offset: 0x28,
        sdk_index: 7,
        address: 0x005E_C9E0,
        method: "SetOrientation",
        callee_pop: 4,
        arg_dwords: 1,
        ret: RetChannel::Al,
        note: "5.2",
    },
    SlotSpec {
        slot: 11,
        offset: 0x2C,
        sdk_index: 8,
        address: 0x005E_D290,
        method: "SetState",
        callee_pop: 4,
        arg_dwords: 1,
        ret: RetChannel::Void,
        note: "nt 13.1",
    },
    SlotSpec {
        slot: 12,
        offset: 0x30,
        sdk_index: 9,
        address: 0x005E_D560,
        method: "GetState",
        callee_pop: 0,
        arg_dwords: 0,
        ret: RetChannel::Eax,
        note: "4.1 movzx",
    },
    SlotSpec {
        slot: 13,
        offset: 0x34,
        sdk_index: 10,
        address: 0x005E_D400,
        method: "SetTimeOfDayRange",
        callee_pop: 8,
        arg_dwords: 2,
        ret: RetChannel::Void,
        note: "nt 13.1; x87, ST(0) empty",
    },
    SlotSpec {
        slot: 14,
        offset: 0x38,
        sdk_index: 11,
        address: 0x005E_CA10,
        method: "GetTimeOfDayRange",
        callee_pop: 8,
        arg_dwords: 2,
        ret: RetChannel::Void,
        note: "8 item 3; ST(0) empty",
    },
    SlotSpec {
        slot: 15,
        offset: 0x3C,
        sdk_index: 12,
        address: 0x005E_EFF0,
        method: "SetSimulatorDateRange",
        callee_pop: 16,
        arg_dwords: 4,
        ret: RetChannel::Void,
        note: "pd 4.6",
    },
    SlotSpec {
        slot: 16,
        offset: 0x40,
        sdk_index: 13,
        address: 0x005E_E230,
        method: "TickSimulatorDate",
        callee_pop: 4,
        arg_dwords: 1,
        ret: RetChannel::Void,
        note: "pd 3.3",
    },
    SlotSpec {
        slot: 17,
        offset: 0x44,
        sdk_index: 14,
        address: 0x007C_2740,
        method: "SetRandomChance",
        callee_pop: 4,
        arg_dwords: 1,
        ret: RetChannel::Void,
        note: "2.2",
    },
    SlotSpec {
        slot: 18,
        offset: 0x48,
        sdk_index: 15,
        address: 0x005E_D570,
        method: "GetRandomChance",
        callee_pop: 0,
        arg_dwords: 0,
        ret: RetChannel::Al,
        note: "4.1; upper EAX undefined",
    },
    SlotSpec {
        slot: 19,
        offset: 0x4C,
        sdk_index: 16,
        address: 0x005E_D580,
        method: "GetPropTimerBitfield",
        callee_pop: 0,
        arg_dwords: 0,
        ret: RetChannel::Eax,
        note: "4.1 movzx",
    },
    SlotSpec {
        slot: 20,
        offset: 0x50,
        sdk_index: 17,
        address: 0x005E_D590,
        method: "GetPropTimerState",
        callee_pop: 4,
        arg_dwords: 1,
        ret: RetChannel::Eax,
        note: "4.1 real 0/1",
    },
    SlotSpec {
        slot: 21,
        offset: 0x54,
        sdk_index: 18,
        address: 0x005E_CA50,
        method: "SetPropTimerState",
        callee_pop: 8,
        arg_dwords: 2,
        ret: RetChannel::Void,
        note: "nt 13.1/quirk 7",
    },
    SlotSpec {
        slot: 22,
        offset: 0x58,
        sdk_index: 19,
        address: 0x0047_7120,
        method: "GetPropUsage",
        callee_pop: 0,
        arg_dwords: 0,
        ret: RetChannel::Al,
        note: "4.1; icf with flora/landfill",
    },
    SlotSpec {
        slot: 23,
        offset: 0x5C,
        sdk_index: 20,
        address: 0x005E_CAE0,
        method: "SetPropUsage",
        callee_pop: 4,
        arg_dwords: 1,
        ret: RetChannel::Void,
        note: "2.2",
    },
];

/// The number of slots in `cISC4PropOccupant` (§4.2).
pub const VTABLE_LEN: usize = PROP_OCCUPANT_VTABLE.len();

/// The vtable address in the original image (§1). Recorded for a host that
/// compares against `docs/abi/interfaces.tsv`.
pub const VTABLE_ADDR: u32 = 0x00AA_4900;
/// The pool/factory method table that follows the vtable in the image. It is
/// **not** part of this interface (§4.2, test 4).
pub const VTABLE_NEXT_ADDR: u32 = 0x00AA_4960;

/// Returns the measured channel of a compiled slot, or `None` past the end.
pub fn slot_channel(slot: usize) -> Option<RetChannel> {
    PROP_OCCUPANT_VTABLE.get(slot).map(|s| s.ret)
}

/// Returns the slot spec of a compiled slot, or `None` past the end.
pub fn slot_spec(slot: usize) -> Option<&'static SlotSpec> {
    PROP_OCCUPANT_VTABLE.get(slot)
}

// ---- the process-wide prop type ID table (§5.1) -----------------------------

/// The process-wide table of prop type IDs that the `+0x5C` u16 indexes
/// (`0x00B231A4`, §5.1).
///
/// This is **not** the prop catalogue's type map (`0x00B21760 + 0x10`): that
/// counts how many types the catalogue holds, this maps an index to an ID
/// (§5.1 point 3). `GetPropTypeIndex` **appends** an unknown ID, so the table
/// grows across a session (§5.1 point 2, quirk 2).
#[derive(Default)]
pub struct PropTypeTable {
    ids: Vec<u32>,
}

impl PropTypeTable {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many IDs the table holds. A table that has interned an unknown type
    /// is one longer than before (test 6).
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether the table holds no IDs yet.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// `GetPropTypeIndex` (`0x005EF2E0`): the index of `id`, interning it at
    /// the end when it is not known yet (§5.1, quirk 2).
    pub fn intern(&mut self, id: u32) -> u16 {
        if let Some(i) = self.ids.iter().position(|&x| x == id) {
            return i as u16;
        }
        let i = self.ids.len();
        self.ids.push(id);
        // A u16 index: past 65536 entries the original would wrap the index
        // while the table kept growing. [choice] FreeCity4 keeps the index
        // exact and records the overflow as a saturating value so a save still
        // round-trips the u32 ID (test 18).
        u16::try_from(i).unwrap_or(u16::MAX)
    }

    /// `GetPropType` (slot 7, `0x005ED280`): `table[+0x5C]`. An index past the
    /// end reads as 0 [choice] (the original would read past the array).
    pub fn id_of(&self, index: u16) -> u32 {
        self.ids.get(index as usize).copied().unwrap_or(0)
    }

    /// The IDs in table order.
    pub fn ids(&self) -> &[u32] {
        &self.ids
    }
}

/// A handle to the process-wide prop type table. Every occupant of a process
/// shares one (§5.1 point 1), so two props of the same type hold the same
/// index (test 5).
pub type SharedPropTypeTable = Rc<RefCell<PropTypeTable>>;

/// A fresh process-wide prop type table.
pub fn shared_type_table() -> SharedPropTypeTable {
    Rc::new(RefCell::new(PropTypeTable::new()))
}

// ---- host seam --------------------------------------------------------------

/// What the occupant needs from the rest of the game (§3, §6.1, `nt` §13.1).
///
/// The core never reaches into the prop manager, the prop developer or the
/// catalogue: those own *which* prop is placed. This trait is the seam a glue
/// lane wires to the occupant manager, the 24-hour clock, the message server
/// and the model maker.
pub trait PropOccupantHost {
    /// Whether the occupant is in the city (inserted in the occupant manager).
    /// `SetState` posts only when this is true (`nt` §13.1 step 3).
    fn is_in_city(&self) -> bool;

    /// Post an occupant message carrying this prop and `data1`.
    fn post_occupant_message(&self, msg_type: u32, data1: u32);

    /// Whether the prop's exemplar has property [`PROP_MODEL_STATE`]
    /// (`nt` §13.1 step 4). `None` = no exemplar is attached, which is not the
    /// same as "the property is absent" (quirk 6).
    fn has_model_state_property(&self) -> Option<bool>;

    /// Switch the drawn model instances to `state` (`nt` §13.1 step 4).
    fn set_model_state(&self, state: u8);

    /// Register or clear the prop's 24-hour clock entry. `start == end` clears
    /// it (`nt` §13.1 slot 13; test 11).
    fn set_clock_entry(&self, start_tenths: u8, end_tenths: u8);

    /// One draw from the prop occupant's own generator `0x00B231B0`, uniform
    /// in `0..100` (quirk 7, foundations §6). The generator is per-prop, so
    /// two props never share a stream.
    fn draw_chance(&self) -> u8;

    /// Remove this prop from the occupant manager (occupant slot 4
    /// `Shutdown`, §3). Answers whether it was still there, so a second
    /// removal is a no-op (test 25).
    fn remove_from_city(&self) -> bool;
}

/// The prop usage codes the four `Add*Prop` paths write (§3). The same values
/// as [`crate::prop_manager::usage`], at the field's width (`+0x64` is a byte).
pub mod usage {
    /// `AddCityProp` (§3).
    pub const CITY: u8 = 1;
    /// `AddLotProp` (§3).
    pub const LOT: u8 = 2;
    /// `AddBuildingProp` (§3).
    pub const BUILDING: u8 = 3;
    /// `AddStreetProp` (§3).
    pub const STREET: u8 = 4;
}

/// Which `Add*Prop` path placed the prop (§3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddKind {
    /// `AddCityProp`.
    City,
    /// `AddLotProp`.
    Lot,
    /// `AddBuildingProp`.
    Building,
    /// `AddStreetProp`.
    Street,
}

/// The prop usage each `Add*Prop` path writes (§3, test 15).
pub fn add_usage(kind: AddKind) -> u8 {
    match kind {
        AddKind::City => usage::CITY,
        AddKind::Lot => usage::LOT,
        AddKind::Building => usage::BUILDING,
        AddKind::Street => usage::STREET,
    }
}

// ---- the class --------------------------------------------------------------

/// The 0x28-byte date-range sub-record the `+0x54` pointer owns (§2.2,
/// §4.1 slots 15/16). Its meaning is still open (spec PO4); the shape is
/// `lr` §10.1 field 9: interval, duration, start day, end day, each u32.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DateRange {
    /// Interval in days (365 in every corpus save).
    pub interval: u32,
    /// Duration in days (4…28 in the corpus).
    pub duration: u32,
    /// Start day.
    pub start_day: u32,
    /// End day.
    pub end_day: u32,
}

/// One in-world prop (§0). Fields a getter reads are `Cell`s so a getter never
/// borrows (see the module doc's threading note).
pub struct PropOccupant {
    /// The reference count at `+0x14` (quirk 10).
    refcount: Cell<u32>,
    /// The bounding box at `+0x3C`, min x,y,z then max x,y,z.
    bbox: Cell<[f32; 6]>,
    /// The date-range sub-record at `+0x54`, owned (slots 15/16).
    date_range: RefCell<Option<DateRange>>,
    /// The lot tag at `+0x58`; `None` = `0xFFFFFFFF` (§5.3).
    lot_tag: Cell<u32>,
    /// The prop type index at `+0x5C` (§5.1).
    type_index: Cell<u16>,
    /// Time-of-day start at `+0x5E`, tenths of an hour.
    tod_start: Cell<u8>,
    /// Time-of-day end at `+0x5F`, tenths of an hour.
    tod_end: Cell<u8>,
    /// The timer bits at `+0x60`.
    timer_bits: Cell<u8>,
    /// The orientation at `+0x61`, already clamped to 0…3 (§5.2).
    orientation: Cell<u8>,
    /// The random chance at `+0x62`, percent; the constructor default is 100
    /// (quirk 1).
    random_chance: Cell<u8>,
    /// The state at `+0x63`.
    state: Cell<u8>,
    /// The prop usage at `+0x64`.
    prop_usage: Cell<u8>,
    /// The process-wide prop type table this prop indexes (§5.1).
    types: SharedPropTypeTable,
    /// Whether the occupant was loaded from a save (lot-records.md §2.4): a
    /// loaded prop does not announce itself as new (test 20).
    loaded: Cell<bool>,
}

impl Default for PropOccupant {
    fn default() -> Self {
        Self::new(shared_type_table())
    }
}

impl PropOccupant {
    /// A freshly constructed prop (§2.2). The two constructor values an
    /// implementer is most likely to get wrong are both here: the default
    /// random chance is **100** and the default lot tag is **`0xFFFFFFFF`**
    /// (quirk 1).
    pub fn new(types: SharedPropTypeTable) -> Self {
        Self {
            refcount: Cell::new(0),
            bbox: Cell::new([0.0; 6]),
            date_range: RefCell::new(None),
            lot_tag: Cell::new(u32::MAX),
            type_index: Cell::new(0),
            tod_start: Cell::new(0),
            tod_end: Cell::new(0),
            timer_bits: Cell::new(0),
            orientation: Cell::new(0),
            random_chance: Cell::new(100),
            state: Cell::new(0),
            prop_usage: Cell::new(0),
            types,
            loaded: Cell::new(false),
        }
    }

    /// The occupant type the constructor writes at `+0x2C` (§2.2).
    pub fn occupant_type(&self) -> u32 {
        OCCUPANT_TYPE_PROP
    }

    /// The class ID (§1).
    pub fn class_id(&self) -> u32 {
        CLSID_PROP_OCCUPANT
    }

    /// The object size in bytes (§1.2, test 3).
    pub fn object_size(&self) -> usize {
        OBJECT_SIZE
    }

    // -- QueryInterface (§1.1) ---------------------------------------------

    /// `QueryInterface` (`0x005EC960`), slot 0, channel **`al`** (§4.1).
    ///
    /// Returns `Some(offset)` where `offset` is the byte offset of the
    /// sub-object the IID answers, taken from the **object base**:
    ///
    /// | IID | offset |
    /// |---|---|
    /// | `0xE9793A65` (`cISC4PropOccupant`) | `+0x00` |
    /// | `0x2977AA47` (the class ID, accepted as an IID) | `+0x00` |
    /// | `0x452294AA` (`cIGZMessageTarget2`) | `+0x04` |
    /// | anything else | passed to the `cSC4Occupant` base at `+0x08` |
    ///
    /// Every successful branch calls `AddRef` before returning 1 (§1.1); the
    /// caller does that, this returns where the interface lives. `None` means
    /// the base refused it, i.e. the call returns false (test 1).
    pub fn query_interface(iid: u32) -> Option<usize> {
        match iid {
            IID_PROP_OCCUPANT | CLSID_PROP_OCCUPANT => Some(OFF_PROP_OCCUPANT),
            IID_MESSAGE_TARGET2 => Some(OFF_MESSAGE_TARGET),
            // Anything else reaches the occupant base rather than returning
            // false directly (quirk 4).
            _ => Some(OFF_OCCUPANT),
        }
    }

    /// `QueryInterface` as the base answers it for an IID the class does not
    /// know: the occupant base at `+0x08` handles `cIGZUnknown`, and anything
    /// else fails (test 1's unknown IID).
    pub fn query_interface_base(iid: u32) -> Option<usize> {
        if iid == IID_UNKNOWN {
            Some(OFF_OCCUPANT)
        } else {
            None
        }
    }

    // -- reference counting (§2.2, quirk 10) --------------------------------

    /// `AddRef` (slot 1). The thunk does `add ecx, 8`, so the count it touches
    /// is this one at `+0x14`, **not** `+0x04` (quirk 10, test 4). Returns the
    /// new count, which is what EAX holds after the tail call.
    pub fn add_ref(&self) -> u32 {
        let n = self.refcount.get().wrapping_add(1);
        self.refcount.set(n);
        n
    }

    /// `Release` (slot 2). Returns the count left; 0 means the last reference
    /// is gone (refaudit: the owner frees here, the core does not).
    pub fn release(&self) -> u32 {
        let n = self.refcount.get().wrapping_sub(1);
        self.refcount.set(n);
        n
    }

    /// The reference count at `+0x14`.
    pub fn refcount(&self) -> u32 {
        self.refcount.get()
    }

    // -- sub-objects (§2.3) -------------------------------------------------

    /// `AsOccupant` (slot 3), channel **`eax`**: `this + 8`, and null stays
    /// null (§4.1, test 4).
    pub fn as_occupant_offset(this: Option<usize>) -> Option<usize> {
        this.map(|_| OFF_OCCUPANT)
    }

    /// The byte offset of each sub-object (§2.3).
    pub fn sub_object_offsets() -> [(u32, usize); 6] {
        [
            (0x00AA_4900, OFF_PROP_OCCUPANT),
            (0x00AA_48F0, OFF_MESSAGE_TARGET),
            (0x00AA_4868, OFF_OCCUPANT),
            (0x00AA_484C, OFF_SERIALIZABLE),
            (0x00AA_47E8, OFF_PROPERTY_HOLDER),
            (0x00AA_47D0, OFF_AUX),
        ]
    }

    // -- slot 4: SetBoundingBox (§2.2) --------------------------------------

    /// `SetBoundingBox` (slot 4), channel **`void`**: two `const float[3]*`,
    /// written to the AABB at `+0x3C` (§2.2, §4.1).
    pub fn set_bounding_box(&self, min: [f32; 3], max: [f32; 3]) {
        self.bbox.set([min[0], min[1], min[2], max[0], max[1], max[2]]);
    }

    /// The AABB as `(min, max)`.
    pub fn bounding_box(&self) -> ([f32; 3], [f32; 3]) {
        let b = self.bbox.get();
        ([b[0], b[1], b[2]], [b[3], b[4], b[5]])
    }

    /// The bounding box from an exemplar's `OccupantSize` (`pd` / `plt`
    /// §A6.1 step 5, test 9): from the origin to the size; a prop with **no**
    /// size gives `(0, 0, 0)`–`(1, 1, 1)`. One or two values leave the missing
    /// ones 0 [choice] (the prop manager's bracket, PL5).
    pub fn set_bounding_box_from_size(&self, size: Option<&[f32]>) {
        match size {
            None | Some([]) => self.set_bounding_box([0.0; 3], [1.0, 1.0, 1.0]),
            Some(s) => {
                let mut max = [0.0f32; 3];
                for (i, v) in s.iter().take(3).enumerate() {
                    max[i] = *v;
                }
                self.set_bounding_box([0.0; 3], max);
            }
        }
    }

    // -- slots 5 and 6 (§4.1, quirk 9) --------------------------------------

    /// Slot 5 (`0x004E13E0`), channel **`eax`**: `xor eax, eax; ret` — a
    /// genuine 0, not a no-op (quirk 9). The SDK name is unknown (spec PO6).
    pub fn slot5_stub(&self) -> u32 {
        0
    }

    /// Slot 6 (`0x00648F10`), channel **`void`**: `ret 4` alone; the argument
    /// is ignored (quirk 9). Adjacent to slot 5 and with the opposite channel.
    pub fn slot6_noop(&self, _arg: u32) {}

    // -- slots 7 and 8: the prop type (§5.1) --------------------------------

    /// `GetPropType` (slot 7, `0x005ED280`), channel **`eax`**:
    /// `movzx eax, word [ecx+0x5C]` then `table[+0x5C]` (§5.1).
    pub fn get_prop_type(&self) -> u32 {
        self.types.borrow().id_of(self.type_index.get())
    }

    /// `SetPropType` (slot 8, `0x005EF4C0`). Interns `id` and stores the index
    /// at `+0x5C`. `[choice]` FreeCity4 implements it as `void` (§4.1); the
    /// index is reachable through [`Self::prop_type_index`].
    pub fn set_prop_type(&self, id: u32) {
        let index = self.types.borrow_mut().intern(id);
        self.type_index.set(index);
    }

    /// The u16 index at `+0x5C` (§5.1). Two props of the same type hold the
    /// same value (test 5).
    pub fn prop_type_index(&self) -> u16 {
        self.type_index.get()
    }

    /// Restore an index verbatim, as the record reader does (`lr` §10.1 field 3
    /// saves the u32 ID and interns it on load).
    pub fn set_prop_type_index(&self, index: u16) {
        self.type_index.set(index);
    }

    /// The process-wide type table this prop indexes (§5.1).
    pub fn type_table(&self) -> &SharedPropTypeTable {
        &self.types
    }

    // -- slots 9 and 10: orientation (§5.2) ---------------------------------

    /// `GetOrientation` (slot 9, `0x005EC9D0`), channel **`eax`**: `movsx` of
    /// the byte at `+0x61` (§4.1). The value is always 0…3.
    pub fn get_orientation(&self) -> i32 {
        self.orientation.get() as i32
    }

    /// `SetOrientation` (slot 10, `0x005EC9E0`), channel **`al`**: clamps the
    /// argument to 0…3 and always returns 1 (§4.1, §5.2). A caller cannot
    /// store 4 or a negative value (test 10).
    pub fn set_orientation(&self, orientation: i32) -> bool {
        self.orientation.set(orientation.clamp(0, 3) as u8);
        true
    }

    // -- slots 11 and 12: state (§4.1, nt §13.1) -----------------------------

    /// `GetState` (slot 12, `0x005ED560`), channel **`eax`**:
    /// `movzx eax, byte [ecx+0x63]` — a clean zero-extended value, **not**
    /// merely AL (§4.1, quirk 11, test 7).
    pub fn get_state(&self) -> u32 {
        u32::from(self.state.get())
    }

    /// `SetState(s)` (slot 11, `0x005ED290`), channel **`void`** (`nt` §13.1):
    ///
    /// 1. `s` equals `+0x63` → nothing (test 23).
    /// 2. otherwise store `s`.
    /// 3. occupant in the city → post [`MSG_PROP_STATE_CHANGED`] carrying the
    ///    prop and `s`.
    /// 4. exemplar has [`PROP_MODEL_STATE`] → switch the model state to `s`
    ///    *[inferred: the model-state call]* (quirk 6).
    pub fn set_state(&self, state: u8, host: &dyn PropOccupantHost) {
        if state == self.state.get() {
            return;
        }
        self.state.set(state);
        if host.is_in_city() {
            host.post_occupant_message(MSG_PROP_STATE_CHANGED, u32::from(state));
        }
        if host.has_model_state_property() == Some(true) {
            host.set_model_state(state);
        }
    }

    /// Store the state byte without the `SetState` side effects (the record
    /// reader: `lr` §2.3 "stored as read").
    pub fn restore_state(&self, state: u8) {
        self.state.set(state);
    }

    // -- slots 13 and 14: time of day (§4.1, §8 item 3) ----------------------

    /// `SetTimeOfDayRange(a, b)` (slot 13, `0x005ED400`), channel **`void`**.
    /// The two arguments are **by-value floats**; each is multiplied by 10 and
    /// stored as a byte at `+0x5E` / `+0x5F`. x87 is used internally and
    /// **ST(0) is empty at return** (§8 item 3, quirk 12). Registers with the
    /// 24-hour clock when `a != b` and both are `>= 0` (`nt` §13.1), so
    /// `(0, 0)` registers nothing (test 11).
    pub fn set_time_of_day_range(&self, start: f32, end: f32, host: &dyn PropOccupantHost) {
        let s = tenths_of(start);
        let e = tenths_of(end);
        self.tod_start.set(s);
        self.tod_end.set(e);
        if start != end && start >= 0.0 && end >= 0.0 {
            host.set_clock_entry(s, e);
        } else {
            host.set_clock_entry(0, 0);
        }
    }

    /// `GetTimeOfDayRange(&a, &b)` (slot 14, `0x005ECA10`), channel **`void`**:
    /// two `float*` out-params, each byte × 0.1. **ST(0) is empty at return** —
    /// the method does not return in `st0` (§8 item 3, quirk 12, test 11).
    /// Returns the two out-params; the method's own result is nothing.
    pub fn get_time_of_day_range(&self) -> (f32, f32) {
        (
            f32::from(self.tod_start.get()) * 0.1,
            f32::from(self.tod_end.get()) * 0.1,
        )
    }

    /// The stored tenths-of-an-hour pair at `+0x5E` / `+0x5F` (§2.2).
    pub fn time_of_day_bytes(&self) -> (u8, u8) {
        (self.tod_start.get(), self.tod_end.get())
    }

    /// Restore the stored pair verbatim (the record reader).
    pub fn set_time_of_day_bytes(&self, start: u8, end: u8) {
        self.tod_start.set(start);
        self.tod_end.set(end);
    }

    // -- slots 15 and 16: the date-range sub-record (§2.2) -------------------

    /// `SetSimulatorDateRange` (slot 15, `0x005EEFF0`), channel **`void`**:
    /// four u32s that drive the 0x28-byte sub-record at `+0x54`
    /// (`pd` §4.6). Its meaning is still open (spec PO4); the core keeps the
    /// four values exactly.
    pub fn set_simulator_date_range(&self, range: DateRange) {
        *self.date_range.borrow_mut() = Some(range);
    }

    /// The date-range sub-record at `+0x54`, if one has been set (§2.2).
    pub fn simulator_date_range(&self) -> Option<DateRange> {
        *self.date_range.borrow()
    }

    /// `TickSimulatorDate(date)` (slot 16, `0x005EE230`), channel **`void`**
    /// (`pd` §3.3). The core records the tick count; the schedule itself is
    /// the prop developer's (§4.1 slots 15/16).
    pub fn tick_simulator_date(&self, _date: u32) {}

    /// Whether the prop has a date-range sub-record (record field 8, `lr` §10.1).
    pub fn has_date_range(&self) -> bool {
        self.date_range.borrow().is_some()
    }

    // -- slots 17 and 18: the random chance (§4.1) ---------------------------

    /// `SetRandomChance(c)` (slot 17, `0x007C2740`), channel **`void`**: stores
    /// the byte at `+0x62`.
    pub fn set_random_chance(&self, chance: u8) {
        self.random_chance.set(chance);
    }

    /// `GetRandomChance` (slot 18, `0x005ED570`), channel **`al`**:
    /// `mov al, byte [ecx+0x62]` — the upper 24 bits of EAX are **undefined**
    /// (§4.1, quirks 3 and 11). `[choice]` FreeCity4 zero-extends, so this
    /// returns a `u8`; a C host must read AL only (test 8).
    ///
    /// [abi-ret] Slot 18 is `al` (docs/abi/interfaces.tsv says `eax-or-void`);
    /// the value is a percent 1…100, never a bool. Reading EAX as a 32-bit
    /// value must mask to `0xFF`.
    pub fn get_random_chance(&self) -> u8 {
        self.random_chance.get()
    }

    // -- slots 19, 20, 21: the timer bits (§2.2, nt §13.1) ------------------

    /// `GetPropTimerBitfield` (slot 19, `0x005ED580`), channel **`eax`**:
    /// `movzx eax, byte [ecx+0x60]` (§4.1, test 12).
    pub fn get_prop_timer_bitfield(&self) -> u32 {
        u32::from(self.timer_bits.get())
    }

    /// `GetPropTimerState(mask)` (slot 20, `0x005ED590`), channel **`eax`**: a
    /// real 0/1 from `(+0x60 & mask) != 0`, produced with `sbb`/`neg` (§4.1).
    ///
    /// [abi-ret] Slot 20 is `eax` with a real 0/1 (docs/abi/interfaces.tsv
    /// says `eax-or-void`): a bool read is exact here, unlike an `al` slot.
    pub fn get_prop_timer_state(&self, mask: u8) -> bool {
        self.timer_bits.get() & mask != 0
    }

    /// `SetPropTimerState(mask, on)` (slot 21, `0x005ECA50`), channel
    /// **`void`** (`nt` §13.1, quirk 7):
    ///
    /// 1. set or clear `mask` in `+0x60`;
    /// 2. notice whether the **low nibble** went from "not all set" to "all
    ///    set" (`0xF`), or the reverse;
    /// 3. becoming all set: draw `r := RandomUint32Uniform(100)` from the
    ///    prop's own generator; `r < chance` → `SetState(0)`, else nothing;
    /// 4. becoming not all set: `SetState(1)`.
    ///
    /// The draw is the only random draw here and happens only on the
    /// transition (test 13). Bits outside the low nibble never draw.
    pub fn set_prop_timer_state(&self, mask: u8, on: bool, host: &dyn PropOccupantHost) {
        let before = self.timer_bits.get();
        let after = if on { before | mask } else { before & !mask };
        self.timer_bits.set(after);
        let nibble_before = before & 0x0F;
        let nibble_after = after & 0x0F;
        if nibble_before != 0x0F && nibble_after == 0x0F {
            if u32::from(host.draw_chance()) < u32::from(self.random_chance.get()) {
                self.set_state(0, host);
            }
        } else if nibble_before == 0x0F && nibble_after != 0x0F {
            self.set_state(1, host);
        }
    }

    /// Restore the timer bits verbatim (the record reader).
    pub fn set_prop_timer_bits(&self, bits: u8) {
        self.timer_bits.set(bits);
    }

    // -- slots 22 and 23: the prop usage (§2.2) ------------------------------

    /// `GetPropUsage` (slot 22, `0x00477120`), channel **`al`**: the byte at
    /// `+0x64` (§4.1). Identical-code-folded with the flora and landfill
    /// occupants (quirk 13). `[choice]` FreeCity4 zero-extends.
    ///
    /// [abi-ret] Slot 22 is `al` (docs/abi/interfaces.tsv says `eax-or-void`);
    /// the value is a usage code 1…4, never a bool.
    pub fn get_prop_usage(&self) -> u8 {
        self.prop_usage.get()
    }

    /// `SetPropUsage` (slot 23, `0x005ECAE0`), channel **`void`**.
    pub fn set_prop_usage(&self, usage: u8) {
        self.prop_usage.set(usage);
    }

    // -- the lot tag (§5.3) --------------------------------------------------

    /// The lot tag at `+0x58`, `u32::MAX` = none (§5.3). It is the
    /// configuration object's ID (`cSC4LotConfigurationObject` `+0x24`) of the
    /// object that caused the prop to be placed.
    pub fn lot_tag(&self) -> u32 {
        self.lot_tag.get()
    }

    /// Whether the prop has a lot tag (occupant slot 27 `GetLotTag` is false
    /// when there is none, §5.3, test 14).
    pub fn has_lot_tag(&self) -> bool {
        self.lot_tag.get() != u32::MAX
    }

    /// `SetLotTag` (occupant slot 28). `u32::MAX` clears the tag.
    pub fn set_lot_tag(&self, tag: u32) {
        self.lot_tag.set(tag);
    }

    // -- load bookkeeping (lot-records.md §2.4) ------------------------------

    /// Whether this prop was loaded from a save. The occupant base's `Init`
    /// posts [`MSG_CREATE_OCCUPANT`] only when it was **not** (test 20).
    pub fn is_loaded(&self) -> bool {
        self.loaded.get()
    }

    /// Mark the prop as loaded, as the record reader does.
    pub fn set_loaded(&self, loaded: bool) {
        self.loaded.set(loaded);
    }

    /// The occupant base's `Init` (`0x005E8C90`, lot-records.md §2.4): post
    /// `CreateOccupant` only for a prop that was not loaded; a loaded prop
    /// instead clears its highlight. Returns whether anything was posted.
    pub fn init_after_load(&self, host: &dyn PropOccupantHost) -> bool {
        if self.loaded.get() {
            return false;
        }
        host.post_occupant_message(MSG_CREATE_OCCUPANT, 0);
        true
    }

    /// `Shutdown` (occupant slot 4) after `RemovePropA`: the prop is removed
    /// from the occupant manager (§3). Answers whether it was still there, so
    /// a **second** removal is a no-op (test 25).
    ///
    /// [abi-ret] Not a game-method return; the bool is the core's own "was
    /// removed" answer so a caller can tell the first removal from the rest.
    pub fn shutdown(&self, host: &dyn PropOccupantHost) -> bool {
        host.remove_from_city()
    }
}

/// `hour * 10` as the byte the slot stores (`nt` §13.1: "each hour × 10,
/// converted to a byte"). A value outside 0…25.5 wraps as the byte does
/// [choice] (the original multiplies and truncates).
fn tenths_of(hour: f32) -> u8 {
    let scaled = hour * 10.0;
    if !(scaled.is_finite() && scaled >= 0.0) {
        return 0;
    }
    (scaled as i64).clamp(0, 255) as u8
}
