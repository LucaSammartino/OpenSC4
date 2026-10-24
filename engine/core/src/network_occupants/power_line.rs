//! `cSC4PowerLineOccupant` (`0xC9C05C5D`), network-occupants.md §5: the
//! `cISC4PowerLineOccupant` slots, `SetPosition`, the degenerate bounding
//! box, `Shutdown`, and the v2 save record.
//!
//! §5.1's layout table and its slot table disagree on the offsets of the
//! cell and the poles (layout: cell `+0x40/+0x44`, poles `+0x48/+0x4C/+0x50`;
//! slots: `GetCell` = `+0x44/+0x48`, `GetConnectedPoles` = `+0x4C/+0x50` and
//! byte `+0x54`), and §5.2's record writes references from `+0x4C`, `+0x44`
//! and `+0x48`. Question NO13 asks for the reconciliation. This module
//! therefore keeps the live object (named by the *slots*) and the record (a
//! literal list of the written fields) apart: [`PowerLineRecord`] does not
//! claim to know which live field each written word comes from.

use super::identity::{IID_POWER_POLE, OCCUPANT_TYPE_POWER_LINE};
use crate::automata::OccupantBase;
use crate::stream::{InStream, OutStream, RefValue};

/// `Write`'s version (§5.2, quirk 6).
pub const POWER_LINE_RECORD_VERSION: u16 = 2;

/// A power line occupant's live state (§5.1).
#[derive(Clone, Debug)]
pub struct PowerLine {
    /// The `cSC4Occupant` base block.
    pub base: OccupantBase,
    /// `+0x30`, the record's field 1.
    pub field_30: u32,
    /// World position x, y, z (`GetPosition`).
    pub position: [f32; 3],
    /// The cell (x, z) `SetPosition` stored (`GetCell`, slot 5).
    pub cell: (i32, i32),
    /// The two poles `GetConnectedPoles` returns (out1, out2).
    pub poles: [RefValue; 2],
    /// The byte `GetConnectedPoles` returns as out3 ("the third pole's
    /// flag").
    pub pole_flag: u8,
    /// The preview byte (slots 6/7).
    pub preview: u8,
}

impl Default for PowerLine {
    fn default() -> Self {
        PowerLine::new()
    }
}

impl PowerLine {
    /// A freshly constructed line: occupant type `0x0990B2EA` (§1.1).
    pub fn new() -> PowerLine {
        PowerLine {
            base: OccupantBase::new(OCCUPANT_TYPE_POWER_LINE),
            field_30: 0,
            position: [0.0; 3],
            cell: (0, 0),
            poles: [RefValue::Null, RefValue::Null],
            pole_flag: 0,
            preview: 0,
        }
    }

    /// Slot 3 `GetConnectedPoles`: the two pole references (cloned: the
    /// game `AddRef`s them for the caller) and the flag byte.
    pub fn get_connected_poles(&self) -> (RefValue, RefValue, u8) {
        (self.poles[0].clone(), self.poles[1].clone(), self.pole_flag)
    }

    /// Slot 4 `SetConnectedPoles`: releases the old references (dropped)
    /// and holds the new ones.
    pub fn set_connected_poles(&mut self, a: RefValue, b: RefValue, flag: u8) {
        self.poles = [a, b];
        self.pole_flag = flag;
    }

    /// Slot 5 `GetCell`.
    pub fn get_cell(&self) -> (i32, i32) {
        self.cell
    }

    /// Slot 6 `IsPreview`.
    pub fn is_preview(&self) -> bool {
        self.preview != 0
    }

    /// Slot 7 `SetPreview`: stores the byte.
    pub fn set_preview(&mut self, preview: u8) {
        self.preview = preview;
    }

    /// `SetPosition` (`0x0064A460`): stores the position and the cell
    /// `world × 0.0625` per axis through the game's float→int helper. The
    /// helper's rounding is not stated; truncation toward zero (the C
    /// conversion) is used (question NO13).
    pub fn set_position(&mut self, pos: [f32; 3]) {
        self.position = pos;
        self.cell = ((pos[0] * 0.0625) as i32, (pos[2] * 0.0625) as i32);
    }

    /// `GetBoundingBox` (`0x0064A4A0`): min = max = the position.
    pub fn get_bounding_box(&self) -> ([f32; 3], [f32; 3]) {
        (self.position, self.position)
    }

    /// `Shutdown` (`0x00649270`): releases the two pole references, then
    /// the base `Shutdown` (the caller's).
    pub fn shutdown(&mut self) {
        self.poles = [RefValue::Null, RefValue::Null];
    }
}

/// `ComputePolePosition` x/z (§5.2): `(cell + 0.5) × 16`.
pub fn pole_position_axis(cell: i32) -> f32 {
    (cell as f32 + 0.5) * 16.0
}

/// The v2 record, in written order (§5.2).
#[derive(Clone, Debug)]
pub struct PowerLineRecord {
    pub version: u16,
    pub base: OccupantBase,
    /// Position `+0x38` (three float bit patterns).
    pub position_bits: [u32; 3],
    pub cell: [i32; 2],
    pub flag: bool,
    /// The two pole references, in order (NO13).
    pub refs: [RefValue; 2],
}

/// `Write` (`0x0064A310`).
pub fn write_power_line_record(rec: &PowerLineRecord, out: &mut dyn OutStream) -> bool {
    out.put_u16(POWER_LINE_RECORD_VERSION);
    if !rec.base.write(out) {
        return false;
    }
    for &w in &rec.position_bits {
        out.put_u32(w);
    }
    out.put_i32(rec.cell[0]);
    out.put_i32(rec.cell[1]);
    out.put_bool(rec.flag);
    for r in &rec.refs {
        out.put_ref(r);
    }
    out.error() == 0
}

/// `Read` (`0x0064A3A0`): symmetric, references read with IID
/// `0xA8C86142`. Only version 2 is accepted (NO13).
pub fn read_power_line_record(inp: &mut dyn InStream) -> Option<PowerLineRecord> {
    let version = inp.get_u16()?;
    if version != POWER_LINE_RECORD_VERSION {
        return None;
    }
    let base = OccupantBase::read(inp)?;
    let position_bits = [inp.get_u32()?, inp.get_u32()?, inp.get_u32()?];
    let cell = [inp.get_i32()?, inp.get_i32()?];
    let flag = inp.get_bool()?;
    let refs = [inp.get_ref_as(IID_POWER_POLE)?, inp.get_ref_as(IID_POWER_POLE)?];
    Some(PowerLineRecord {
        version,
        base,
        position_bits,
        cell,
        flag,
        refs,
    })
}
