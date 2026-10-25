// SPDX-License-Identifier: GPL-3.0-or-later
//! Flora occupant persistence: flora §3.4 and lot-records §11.

use crate::automata::{KeepParentKeys, OccupantBase, OCCUPANT_BASE_VERSION};
use crate::stream::{InStream, OutStream};

use super::{occupant_type, FloraOccupant, Vec3};

#[derive(Clone, Debug, PartialEq)]
pub struct FloraInstance {
    pub base: OccupantBase,
    pub flora: FloraOccupant,
    pub position: Vec3,
    pub lot_tag: u32,
}

impl Default for FloraInstance {
    fn default() -> Self {
        let mut base = OccupantBase::new(occupant_type::FLORA);
        base.flags |= 8;
        base.cell_rect = [255, 0, 0, 0];
        Self {
            base,
            flora: FloraOccupant::default(),
            position: [0.0; 3],
            lot_tag: 0,
        }
    }
}

impl FloraInstance {
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(3);
        self.base.write(out);
        out.put_u32(self.flora.flora_type);
        for v in self.position {
            out.put_f32(v);
        }
        out.put_u32(self.flora.birth_date);
        out.put_u32(self.flora.last_seeding_date);
        out.put_u8(self.flora.state as u8);
        out.put_u8(self.flora.orientation);
        out.put_u32(self.lot_tag);
        out.error() == 0
    }

    pub fn read(&mut self, inp: &mut dyn InStream) -> bool {
        self.read_fields(inp).is_some() && inp.error() == 0
    }

    fn read_fields(&mut self, inp: &mut dyn InStream) -> Option<()> {
        if inp.get_u16()? != 3 {
            return None;
        }
        // The containing flora reader tests base Read; base ignores holder's
        // Boolean and returns only the stream error (lot-records §2.3, §11).
        if inp.get_u16()? != OCCUPANT_BASE_VERSION {
            return (inp.error() == 0).then_some(());
        }
        self.base.removal = inp.get_u16()?;
        self.base.highlight = inp.get_u8()?;
        self.base.status = (self.base.highlight << 4) | 2;
        self.base.flags = inp.get_u8()?;
        self.base.occupant_type = inp.get_u32()?;
        for c in &mut self.base.cell_rect {
            *c = inp.get_u8()?;
        }
        self.base.holder.read(inp, &KeepParentKeys);
        if inp.error() != 0 {
            return None;
        }
        self.flora.flora_type = inp.get_u32()?;
        for v in &mut self.position {
            *v = inp.get_f32()?;
        }
        self.flora.birth_date = inp.get_u32()?;
        self.flora.last_seeding_date = inp.get_u32()?;
        self.flora.state = inp.get_u8()? as i8;
        self.flora.orientation = inp.get_u8()?;
        self.lot_tag = inp.get_u32()?;
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::{ByteReader, ByteWriter};

    #[test]
    fn record_roundtrip_flags_and_signed_state() {
        let mut value = FloraInstance::default();
        value.base.flags = 255;
        value.flora.flora_type = 0x29cf_f3cb;
        value.flora.birth_date = 2451545;
        value.flora.last_seeding_date = 2466676;
        value.flora.state = -1;
        value.flora.orientation = 3;
        value.position = [-12.5, 63.0, 4096.0];
        value.lot_tag = 123;
        let mut out = ByteWriter::new();
        assert!(value.write(&mut out));
        let mut copy = FloraInstance::default();
        assert!(copy.read(&mut ByteReader::new(out.bytes())));
        value.base.flags &= 0xdd;
        value.base.status = 2;
        assert_eq!(copy, value);
        assert_eq!(out.bytes().len(), 66);
    }

    #[test]
    fn rejects_version_and_truncated_tail() {
        let mut value = FloraInstance::default();
        assert!(!value.read(&mut ByteReader::new(&[2, 0])));
        let mut out = ByteWriter::new();
        assert!(value.write(&mut out));
        for length in 0..out.bytes().len() {
            assert!(!FloraInstance::default().read(&mut ByteReader::new(&out.bytes()[..length])));
        }
    }
}
