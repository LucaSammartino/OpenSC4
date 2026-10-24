// SPDX-License-Identifier: GPL-3.0-or-later
//! LR12 codec discriminators complement native inventory tests T58-T65.
use fc4_core::lot_records::shadow::{reencode, IdentityWriter};
use fc4_core::lot_records::{LotRecord, CLSID_BUSINESS_UNIT, CLSID_LOT};
use fc4_core::stream::{ByteWriter, RefValue};
use fc4_core::traffic_simulator::TripData;
use fc4_core::variant::{Elem, Variant};

fn bytes(record: &LotRecord) -> Vec<u8> {
    let mut w = IdentityWriter(ByteWriter::new());
    assert!(record.write(&mut w));
    w.0.into_bytes()
}
fn identity(token: u64, class_id: u32) -> RefValue {
    RefValue::Host { token, class_id }
}
fn rich() -> LotRecord {
    let mut lot = LotRecord {
        configuration_id: 0x12345678,
        altitude: f32::from_bits(0x80000000),
        slope: f32::from_bits(0x7fc01234),
        business_unit: identity(17, CLSID_BUSINESS_UNIT),
        travel_designate: identity(42, CLSID_LOT),
        trips: vec![TripData {
            count: 12,
            trip_length: 5.25,
            travel_types_used: 0x1234,
            ..TripData::default()
        }],
        strategy: 7,
        ..LotRecord::default()
    };
    lot.properties.add(19, Variant::Scalar(Elem::U32(12)));
    lot
}

#[test]
fn lr12_t59_configuration_nested_fields_and_float_bits() {
    let original = bytes(&rich());
    assert_eq!(reencode(&original), Some(original));
}

#[test]
fn lr12_t60_aliases_and_cycles_are_identities_only() {
    let mut a = rich();
    let mut b = rich();
    a.travel_designate = identity(2, CLSID_LOT);
    b.travel_designate = identity(1, CLSID_LOT);
    for lot in [a, b] {
        let original = bytes(&lot);
        assert_eq!(reencode(&original), Some(original));
    }
}

#[test]
fn lr12_t62_partial_capture_is_not_accepted() {
    let original = bytes(&rich());
    for length in 0..original.len() {
        assert!(reencode(&original[..length]).is_none(), "length {length}");
    }
}

#[test]
fn lr12_t63_scalar_trip_and_property_type_differences_survive() {
    let lot = rich();
    let original = bytes(&lot);
    for variant in 0..3 {
        let mut changed = lot.clone();
        match variant {
            0 => changed.strategy ^= 1,
            1 => changed.trips[0].travel_types_used ^= 1,
            _ => changed.properties.add(19, Variant::Scalar(Elem::I32(12))),
        }
        let encoded = bytes(&changed);
        assert_ne!(encoded, original);
        assert_eq!(reencode(&encoded), Some(encoded));
    }
}

#[test]
fn lr12_t65_reference_identity_and_exact_consumption() {
    let mut lot = rich();
    let original = bytes(&lot);
    lot.business_unit = identity(18, CLSID_BUSINESS_UNIT);
    assert_ne!(reencode(&bytes(&lot)).unwrap(), original);
    let mut trailing = original;
    trailing.push(0);
    assert!(reencode(&trailing).is_none());
}
