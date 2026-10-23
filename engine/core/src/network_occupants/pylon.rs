//! Pylon record and published lifecycle (network-occupants §§6.1-6.3).
//! Texture/model building, geometry, drawing and unspecified setters are not
//! replaced by host owner-result callbacks. Their missing contracts are NO36.

use crate::automata::OccupantBase;
use crate::stream::{InStream, OutStream, RefValue};

use super::identity::OCCUPANT_TYPE_PYLON;
use super::xform::Xform;

pub const PYLON_RECORD_VERSION: u16 = 5;
pub const PYLON_MESSAGES: [u32; 3] = [0xA6B7_9602, 0x06CD_B65B, 0x26D3_1EC1];
pub const PYLON_TICK: u32 = 0x80;
pub const ANIMATION_TICK_MANAGER_CLASS: u32 = 0x86F9_FB4D;
pub const ANIMATION_TICK_MANAGER_IID: u32 = 0x86F9_FB4C;

/// Fields in stream order, not a claim about the contradictory native aliases
/// (NO37). Supplied state avoids inventing defaults for fields absent in v3/4.
#[derive(Clone, Debug)]
pub struct PylonRecord {
    pub base: OccupantBase,
    pub transform_5c: Xform,
    pub bytes_60_64_68: [u8; 3],
    pub byte_198: u8,
    pub field_140: u32,
    pub source_6c: RefValue,
    pub transform_70: Xform,
    pub byte_199: u8,
    pub field_19c: u32,
    pub field_1a0: i64,
    pub reference_1b4: RefValue,
}

impl PylonRecord {
    /// §6.3 Write always emits v5, regardless of the version last read.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(PYLON_RECORD_VERSION);
        if !self.base.write(out) || !self.transform_5c.write(out) {
            return false;
        }
        for byte in self.bytes_60_64_68 {
            out.put_u8(byte);
        }
        out.put_u8(self.byte_198);
        out.put_u32(self.field_140);
        out.put_ref(&self.source_6c);
        if !self.transform_70.write(out) {
            return false;
        }
        out.put_u8(self.byte_199);
        out.put_u32(self.field_19c);
        out.put_i64(self.field_1a0);
        out.put_ref(&self.reference_1b4);
        out.error() == 0
    }

    /// §6.3 Read: versions >=3; retain existing state for the v5-only tail
    /// when loading v3/4. No IID is published for either reference, so the
    /// serializable reference reader is used directly (NO37).
    pub fn read_into(&mut self, inp: &mut dyn InStream) -> Option<()> {
        let version = inp.get_u16()?;
        if version < 3 {
            return None;
        }
        self.base = OccupantBase::read(inp)?;
        if version < 4 {
            self.transform_5c.legacy_load_into(inp)?;
        } else {
            self.transform_5c.read_into(inp)?;
        }
        for byte in &mut self.bytes_60_64_68 {
            *byte = inp.get_u8()?;
        }
        self.byte_198 = inp.get_u8()?;
        self.field_140 = inp.get_u32()?;
        // Release before resolving the replacement, including a failed read.
        self.source_6c = RefValue::Null;
        self.source_6c = inp.get_ref()?;
        // Literal reading: the v<4 transform rule applies to both transforms
        // named by the symmetric record. Clarification requested in NO37.
        if version < 4 {
            self.transform_70.legacy_load_into(inp)?;
        } else {
            self.transform_70.read_into(inp)?;
        }
        self.byte_199 = inp.get_u8()?;
        if version > 4 {
            self.field_19c = inp.get_u32()?;
            self.field_1a0 = inp.get_i64()?;
            self.reference_1b4 = inp.get_ref()?;
        }
        Some(())
    }
}

/// Only foreign base/message/tick/city primitives. No pylon-owned draw,
/// model-build, texture-build, or placement computation is delegated.
pub trait PylonLifecycleHost {
    fn is_initialized(&mut self, base: &OccupantBase) -> bool;
    fn occupant_init(&mut self, base: &mut OccupantBase);
    fn subscribe_message(&mut self, message: u32);
    fn subscribe_animation(&mut self, class: u32, iid: u32, tick: u32);
    fn occupant_shutdown(&mut self, base: &mut OccupantBase);
    fn city_handle(&mut self) -> u32;
    fn remove_occupant(&mut self, handle: u32);
    fn unsubscribe_message(&mut self, message: u32);
}

/// The published global write by Init, kept outside individual pylons.
#[derive(Clone, Debug, Default)]
pub struct PylonGlobals {
    pub field_b23b78: bool,
}

#[derive(Clone, Debug)]
pub struct Pylon {
    pub record: PylonRecord,
    pub call_number: u32,
    pub drag_state: u32,
    /// Byte +0x37; not inferred from the serialized occupant flags (NO37).
    pub placement_flags: u8,
    /// Shutdown explicitly releases +0x78, whereas the record source is +0x6C.
    /// Preserve the distinction pending NO37 instead of releasing both.
    pub reference_78: RefValue,
}

impl Pylon {
    /// Construct with explicitly supplied unpublished defaults. The named
    /// owner's occupant type is always its own type, never the bare type.
    pub fn from_state(
        mut record: PylonRecord,
        call_number: u32,
        drag_state: u32,
        placement_flags: u8,
        reference_78: RefValue,
    ) -> Self {
        record.base.occupant_type = OCCUPANT_TYPE_PYLON;
        Self {
            record,
            call_number,
            drag_state,
            placement_flags,
            reference_78,
        }
    }

    pub fn set_call_number(&mut self, number: u32) {
        self.call_number = number;
    }

    pub fn get_call_number(&self) -> u32 {
        self.call_number
    }

    pub fn set_drag_state(&mut self, state: u32) {
        self.drag_state = state;
    }

    pub fn get_drag_state(&self) -> u32 {
        self.drag_state
    }

    pub fn source_occupant(&self) -> RefValue {
        self.record.source_6c.clone()
    }

    /// Published subscription portion of Init. Render-state packing is absent:
    /// the descriptions and slot-0x28 result shape are not specified (NO36).
    pub fn init_published_lifecycle(&mut self, globals: &mut PylonGlobals, host: &mut impl PylonLifecycleHost) {
        // Literal guard interpretation of the IsInitialized call; NO37 asks
        // for the actual repeated-Init branch/return contract.
        if host.is_initialized(&self.record.base) {
            return;
        }
        host.occupant_init(&mut self.record.base);
        globals.field_b23b78 = true;
        for message in PYLON_MESSAGES {
            host.subscribe_message(message);
        }
        host.subscribe_animation(ANIMATION_TICK_MANAGER_CLASS, ANIMATION_TICK_MANAGER_IID, PYLON_TICK);
    }

    /// §6.2's explicit Shutdown order. No animation unsubscription is added:
    /// the spec only lists the three message unsubscriptions here.
    pub fn shutdown_published_lifecycle(&mut self, host: &mut impl PylonLifecycleHost) {
        host.occupant_shutdown(&mut self.record.base);
        if self.placement_flags & 1 != 0 {
            let handle = host.city_handle();
            host.remove_occupant(handle);
            // NO37: placement's remaining renderer/state-bit behavior is not
            // published; no guessed bit mask or renderer calls are made.
        }
        for message in PYLON_MESSAGES {
            host.unsubscribe_message(message);
        }
        self.reference_78 = RefValue::Null;
    }
}
