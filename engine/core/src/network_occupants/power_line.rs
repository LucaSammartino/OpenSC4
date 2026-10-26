//! `cSC4PowerLineOccupant` (`0xC9C05C5D`), network-occupants.md §5: the
//! `cISC4PowerLineOccupant` slots, `SetPosition`, the degenerate bounding
//! box, `Shutdown`, and the v2 save record.
//!
//! §5.1–5.2 (NO13 answered) name the same live and saved fields. Preview is
//! transient; reading a record must not derive its saved cell from position.

use super::identity::{IID_POWER_POLE, OCCUPANT_TYPE_POWER_LINE};
use super::pylon::References;
use crate::automata::OccupantBase;
use crate::stream::{InStream, OutStream, RefValue};

/// `Write`'s version (§5.2, quirk 6).
pub const POWER_LINE_RECORD_VERSION: u16 = 2;

/// The adapter forwards the final call through the occupant view, not primary.
pub trait PowerLineHost: References {
    fn occupant_shutdown(&mut self, base: &mut OccupantBase) -> bool;
}

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
    /// Hydrate the live object before Init, without consulting city services.
    pub fn from_record(record: PowerLineRecord) -> Self {
        Self {
            base: record.base,
            field_30: 0,
            position: record.position_bits.map(f32::from_bits),
            cell: (record.cell[0], record.cell[1]),
            poles: record.refs,
            pole_flag: u8::from(record.flag),
            preview: 0,
        }
    }

    /// Snapshot the live state using the v2 record's two counted pole views.
    pub fn record(&self) -> PowerLineRecord {
        PowerLineRecord {
            version: POWER_LINE_RECORD_VERSION,
            base: self.base.clone(),
            position_bits: self.position.map(f32::to_bits),
            cell: [self.cell.0, self.cell.1],
            flag: self.pole_flag != 0,
            refs: self.poles.clone(),
        }
    }

    /// A freshly constructed line: occupant type `0x0990B2EA` (§1.1).
    pub fn new() -> PowerLine {
        PowerLine {
            base: OccupantBase {
                // occupant-manager 13.1: newly constructed occupants are not indexed.
                cell_rect: [0xFF, 0, 0, 0],
                ..OccupantBase::new(OCCUPANT_TYPE_POWER_LINE)
            },
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

    /// Live slot 4: equal pointers still acquire before releasing, independently
    /// for each field. RefValue clones alone do not acquire host references.
    pub fn set_connected_poles_acquired(&mut self, poles: [RefValue; 2], flag: u8, host: &mut impl References) {
        for (slot, value) in self.poles.iter_mut().zip(poles) {
            if !value.is_null() {
                host.add_ref(&value);
            }
            if !slot.is_null() {
                host.release(slot);
            }
            *slot = value;
        }
        self.pole_flag = flag;
    }

    pub fn get_connected_poles_acquired(&self, host: &mut impl References) -> (RefValue, RefValue, u8) {
        for pole in &self.poles {
            if !pole.is_null() {
                host.add_ref(pole);
            }
        }
        self.get_connected_poles()
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
    /// truncation toward zero is specified by §5.1 (NO13).
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
        self.pole_flag = 0;
    }

    /// §5.1 / test 74: live setter effects precede the foreign base Shutdown.
    pub fn shutdown_acquired(&mut self, host: &mut impl PowerLineHost) -> bool {
        self.set_connected_poles_acquired([RefValue::Null, RefValue::Null], 0, host);
        host.occupant_shutdown(&mut self.base)
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    #[derive(Default)]
    struct Host(Vec<&'static str>);
    impl References for Host {
        fn add_ref(&mut self, _: &RefValue) {
            self.0.push("add");
        }
        fn release(&mut self, _: &RefValue) {
            self.0.push("release");
        }
    }
    impl PowerLineHost for Host {
        fn occupant_shutdown(&mut self, base: &mut OccupantBase) -> bool {
            self.0.push("base-occupant");
            if base.status & 1 != 0 {
                base.status &= !1;
                self.0.push("99EF1141/0");
            }
            true
        }
    }
    #[test]
    fn line_layout_and_shutdown_forwarding_conformance_74() {
        let mut line = PowerLine::new();
        let pole = RefValue::Host { token: 7, class_id: 0 };
        line.set_connected_poles(pole.clone(), pole.clone(), 1);
        line.set_position([24., 40., 56.]);
        line.set_preview(1);
        line.base.status |= 1;
        let mut host = Host::default();
        line.set_connected_poles_acquired([pole.clone(), pole], 1, &mut host);
        assert_eq!(host.0, ["add", "release", "add", "release"]);
        host.0.clear();
        let position = line.position;
        let cell = line.cell;
        assert!(line.shutdown_acquired(&mut host));
        assert_eq!(host.0, ["release", "release", "base-occupant", "99EF1141/0"]);
        assert!(line.poles.iter().all(RefValue::is_null));
        assert_eq!(
            (line.pole_flag, line.preview, line.position, line.cell),
            (0, 1, position, cell)
        );
        host.0.clear();
        assert!(line.shutdown_acquired(&mut host));
        assert_eq!(host.0, ["base-occupant"]);
        let loaded = PowerLine::from_record(line.record());
        assert_eq!((loaded.position, loaded.cell, loaded.pole_flag), (position, cell, 0));
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
