// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent effects §3 file-layout checks, without a resource-writer oracle.
//! The placement payload deliberately has no asserted semantic decoding (EF19).

use super::*;

#[test]
fn resource_root_version_rejects_unsupported_formats_before_section_search() {
    // Effects sections 3 and 18.8.1: only root versions 3 and 4 are readable.
    let mut bytes = crate::effects::shipped::volcano_directory().to_bytes();
    let directory = EffectDirectory::parse(&bytes).unwrap();
    for version in [3u16, 4] {
        bytes[..2].copy_from_slice(&version.to_le_bytes());
        assert_eq!(EffectDirectory::parse(&bytes).unwrap().root_version, version);
        assert_eq!(
            EffectDirectory::parse_at(&bytes, directory.offsets.brushes, directory.offsets.effects)
                .unwrap()
                .root_version,
            version
        );
    }
    for version in [0u16, 1, 2, 5, 0x0300, 0x0400, u16::MAX] {
        bytes[..2].copy_from_slice(&version.to_le_bytes());
        for result in [
            EffectDirectory::parse(&bytes),
            EffectDirectory::parse_at(&bytes, directory.offsets.brushes, directory.offsets.effects),
            EffectDirectory::parse(&bytes[..2]),
        ] {
            let error = result.unwrap_err();
            assert_eq!(error.offset, 0);
            assert!(error.what.contains("root version"));
        }
    }
}

fn component_fixture(name: &[u8], kind: u8) -> Vec<u8> {
    // §3 file offsets after the variable-length name, not in-memory offsets.
    let mut fixed = [0u8; 91];
    fixed[0] = kind;
    fixed[1..5].copy_from_slice(&3u32.to_le_bytes());
    for (byte, value) in fixed[5..61].iter_mut().zip(0u8..56) {
        *byte = value;
    }
    fixed[61..63].copy_from_slice(&[2, 7]);
    fixed[63..65].copy_from_slice(&0x1234u16.to_le_bytes());
    fixed[65..67].copy_from_slice(&0x5678u16.to_le_bytes());
    for (slot, bits) in fixed[67..83]
        .chunks_exact_mut(4)
        .zip([0x8000_0000u32, 0x3F80_0000, 0xC000_0000, 0x4040_0000])
    {
        slot.copy_from_slice(&bits.to_le_bytes());
    }
    fixed[83..85].copy_from_slice(&0x9ABCu16.to_le_bytes());
    fixed[85..87].copy_from_slice(&0xDEF0u16.to_le_bytes());
    fixed[87..91].copy_from_slice(&0x1234_5678u32.to_le_bytes());
    let mut bytes = (name.len() as u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(name);
    bytes.extend_from_slice(&fixed);
    bytes
}

#[test]
fn section3_variable_component_stride_preserves_all_placement_bytes() {
    // Header, two components of different name lengths, no bindings, empty
    // effect text, and the three section-version-2 trailing words (§3).
    let mut bytes = vec![0xFF, 1, 0, 0, 5, 0, 0, 0, 2, 0, 0, 0];
    bytes.extend_from_slice(&component_fixture(b"", 3));
    bytes.extend_from_slice(&component_fixture(b"longer-child", 0x10));
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&[1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]);

    let mut cursor = Cursor::new(&bytes, 0);
    let effect = read_effect(&mut cursor, 2).unwrap();
    assert_eq!(cursor.p, bytes.len());
    assert_eq!((effect.flags, effect.priority, effect.extra), (0x1FF, 5, [1, 2, 3]));
    assert_eq!(effect.components.len(), 2);
    assert_eq!(effect.components[0].name, "");
    assert_eq!(effect.components[1].name, "longer-child");
    assert_eq!(effect.components[0].kind, 3);
    assert_eq!(effect.components[1].kind, 0x10);
    for component in &effect.components {
        assert_eq!(component.placement.as_slice(), (0u8..56).collect::<Vec<_>>());
        assert_eq!(component.flags, 3);
        assert_eq!((component.lod_min, component.lod_max), (2, 7));
        assert_eq!((component.copies, component.spacing), (0x1234, 0x5678));
        assert_eq!(
            component.render.map(f32::to_bits),
            [0x8000_0000, 0x3F80_0000, 0xC000_0000, 0x4040_0000]
        );
        assert_eq!((component.select_group, component.select_weight), (0x9ABC, 0xDEF0));
        assert_eq!(component.index, 0x1234_5678);
    }
    let mut written = Vec::new();
    write_effect(&mut written, &effect, 2);
    assert_eq!(written, bytes);

    // Every required field must be present, including the entire opaque block.
    for end in 0..bytes.len() {
        assert!(
            read_effect(&mut Cursor::new(&bytes[..end], 0), 2).is_err(),
            "prefix length {end}"
        );
    }
}
