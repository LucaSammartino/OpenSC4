// SPDX-License-Identifier: GPL-3.0-or-later
//! LR15 scoped codecs: no Init, membership recursion, or normalization.
use fc4_core::lot_records::shadow::{reencode_owner, IdentityWriter};
use fc4_core::lot_records::{
    BaseTextureRecord, BusinessUnitRecord, OccupantRecord, RetainingWallRecord, TextureRecord,
    CLSID_BASE_TEXTURE_OCCUPANT, CLSID_BUSINESS_UNIT, CLSID_LOT, CLSID_RETAINING_WALL_OCCUPANT,
};
use fc4_core::stream::{ByteWriter, RefValue};

#[test]
fn t80_ordered_members_include_off_master_and_aliases() {
    let record = BusinessUnitRecord {
        lots: [17, 29, 83, 17]
            .map(|token| RefValue::Host {
                token,
                class_id: CLSID_LOT,
            })
            .into(),
    };
    let mut w = IdentityWriter(ByteWriter::new());
    assert!(record.write(&mut w));
    let bytes = w.0.into_bytes();
    assert_eq!(reencode_owner(CLSID_BUSINESS_UNIT, &bytes), Some(bytes));
}

#[test]
fn t88_preserve_all_texture_bytes_and_saved_frames() {
    let mut record = BaseTextureRecord::new();
    record.textures = vec![
        TextureRecord {
            texture_instance: 0x12345678,
            cell_x: 19,
            cell_z: 37,
            orientation: 3,
            config_byte: 1,
            unnamed: [1, 2, 0xfe, 0, 0xff],
            frame: 0xff,
        },
        TextureRecord::default(),
    ];
    let mut w = IdentityWriter(ByteWriter::new());
    assert!(record.write(&mut w));
    let bytes = w.0.into_bytes();
    assert_eq!(reencode_owner(CLSID_BASE_TEXTURE_OCCUPANT, &bytes), Some(bytes));
}

#[test]
fn t88_wall_mask_growth_remains_a_mismatch() {
    let mut wall = RetainingWallRecord::new();
    wall.edge_hashes = [1, 0, 0, 0];
    wall.position[0] = f32::from_bits(0x80000000);
    wall.position[1] = f32::from_bits(0x7fc01234);
    let mut w = IdentityWriter(ByteWriter::new());
    assert!(wall.write(&mut w));
    let original = w.0.into_bytes();
    let encoded = reencode_owner(CLSID_RETAINING_WALL_OCCUPANT, &original).unwrap();
    assert_ne!(encoded, original);
    wall.edge_mask = 1;
    let mut expected = IdentityWriter(ByteWriter::new());
    assert!(wall.write(&mut expected));
    assert_eq!(encoded, expected.0.into_bytes());
}

#[test]
fn t85_t87_reject_partial_trailing_and_unsupported_records() {
    let mut unit = IdentityWriter(ByteWriter::new());
    assert!(BusinessUnitRecord::default().write(&mut unit));
    let mut texture = IdentityWriter(ByteWriter::new());
    assert!(BaseTextureRecord::new().write(&mut texture));
    let mut wall = IdentityWriter(ByteWriter::new());
    assert!(RetainingWallRecord::new().write(&mut wall));
    for (class_id, bytes) in [
        (CLSID_BUSINESS_UNIT, unit.0.into_bytes()),
        (CLSID_BASE_TEXTURE_OCCUPANT, texture.0.into_bytes()),
        (CLSID_RETAINING_WALL_OCCUPANT, wall.0.into_bytes()),
    ] {
        assert_eq!(reencode_owner(class_id, &bytes), Some(bytes.clone()));
        for n in 0..bytes.len() {
            assert!(reencode_owner(class_id, &bytes[..n]).is_none());
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert!(reencode_owner(class_id, &trailing).is_none());
    }
    assert!(reencode_owner(CLSID_LOT, &[]).is_none());
}
