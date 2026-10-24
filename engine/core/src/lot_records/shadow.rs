// SPDX-License-Identifier: GPL-3.0-or-later
//! LR12 isolated record observation: no Init and no reference resolution into live objects.
use super::Lot;
use crate::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue, SaveObject};

struct Identities;
impl RefResolver for Identities {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: u64::from(object_id),
            class_id,
        })
    }
}

// Unlike a save registry, this writer preserves the observation's assigned IDs.
pub struct IdentityWriter(pub ByteWriter);
impl OutStream for IdentityWriter {
    fn put_u8(&mut self, v: u8) -> bool {
        self.0.put_u8(v)
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.0.put_u16(v)
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.0.put_u32(v)
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.0.put_u64(v)
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.0.put_f32(v)
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.0.put_f64(v)
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.0.put_bytes(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.0.put_str(v)
    }
    fn put_ref(&mut self, v: &RefValue) -> bool {
        match v {
            RefValue::Null => self.0.put_u32(0),
            RefValue::Host { token, class_id } if *token <= u64::from(u32::MAX) => {
                self.0.put_u32(*token as u32) && self.0.put_u32(*class_id)
            }
            _ => {
                self.0.set_error();
                false
            }
        }
    }
    fn error(&self) -> i32 {
        self.0.error()
    }
    fn set_error(&mut self) {
        self.0.set_error();
    }
}

/// Full record re-encoding, preserving configuration metadata instead of calling
/// the live object's Write (which intentionally uses the Init configuration).
pub fn reencode(bytes: &[u8]) -> Option<Vec<u8>> {
    reencode_with_trip_count(bytes).map(|(encoded, _)| encoded)
}

/// Count inline trips only after accepting the complete master-lot payload.
/// This is the §14.1 inventory, not trips owned by off-list lots or simulators.
pub fn reencode_with_trip_count(bytes: &[u8]) -> Option<(Vec<u8>, u32)> {
    let lot = Lot::default();
    let mut ids = Identities;
    let mut input = ByteReader::with_resolver(bytes, &mut ids);
    if !lot.read(&mut input) || input.remaining() != 0 {
        return None;
    }
    let mut output = IdentityWriter(ByteWriter::new());
    if !lot.rec.borrow().write(&mut output) {
        return None;
    }
    let trips = u32::try_from(lot.rec.borrow().trips.len()).ok()?;
    Some((output.0.into_bytes(), trips))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lot_records::LotRecord;
    use crate::traffic_simulator::TripData;

    #[test]
    fn inline_trip_inventory_requires_complete_record() {
        let record = LotRecord {
            trips: vec![
                TripData {
                    count: 148,
                    trip_length: f32::from_bits(0x80000000),
                    travel_types_used: 0xcccccccc,
                    ..TripData::default()
                },
                TripData::default(),
            ],
            strategy: 73,
            ..LotRecord::default()
        };
        let mut writer = IdentityWriter(ByteWriter::new());
        assert!(record.write(&mut writer));
        let bytes = writer.0.into_bytes();
        let (encoded, trips) = reencode_with_trip_count(&bytes).unwrap();
        assert_eq!(trips, 2);
        assert_eq!(encoded, bytes);
        // A failed final field or an extra byte must not yield a trip inventory.
        assert!(reencode_with_trip_count(&bytes[..bytes.len() - 1]).is_none());
        let mut trailing = bytes;
        trailing.push(0);
        assert!(reencode_with_trip_count(&trailing).is_none());
    }

    #[test]
    fn empty_lot_supplies_no_nonempty_trip_evidence() {
        let mut writer = IdentityWriter(ByteWriter::new());
        assert!(LotRecord::default().write(&mut writer));
        let (encoded, trips) = reencode_with_trip_count(writer.0.bytes()).unwrap();
        assert_eq!(trips, 0);
        assert_eq!(encoded, writer.0.bytes());
    }
}
