// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests for the S3D reader: `docs/specs/graphics-formats.md` §8.2 (tests 21–37, numbered as there) on the test
//! model T and its variants, built in code, plus hostile input.

use super::*;

/// Bytes from hexadecimal text; spaces are ignored.
fn hex(s: &str) -> Vec<u8> {
    let digits: Vec<u8> = s.bytes().filter(u8::is_ascii_hexdigit).collect();
    assert_eq!(digits.len() % 2, 0, "odd hex string {s:?}");
    digits
        .chunks(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect()
}

fn f32s(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// T's three vertices (x, y, z, u, v), 20 bytes each.
fn t_vertices() -> Vec<u8> {
    f32s(&[0., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1.])
}

/// T's material without its texture, then the texture with or without the filter bytes.
const MATERIAL: &str = "AB 00 00 00  04 03 01 00  FF 7F  00 00 00 00  00  01";
const TEXTURE_15: &str = "78 56 34 12  03 03 01 01  21 00 02 00  01 00";
const TEXTURE_OLD: &str = "78 56 34 12  03 03  21 00 02 00  01 00";
const ANIM: &str = "01 00 1E 00 03 00  00 00 00 00  00 00 00 00  01 00  02 00 6D 00  00 00 00 00 00 00 00 00";

/// The test model T (§8.2) as chunk bodies, so variants can change one part.
#[derive(Clone)]
struct T {
    major: u16,
    minor: u16,
    /// Every length field (including `3DMD`'s) set to this value; `None` = the real sizes.
    len: Option<u32>,
    /// VERT body after the block count: block header and vertices.
    vert: Vec<u8>,
    indx: Vec<u8>,
    prim: Vec<u8>,
    mats: Vec<u8>,
    anim: Vec<u8>,
    prop: Option<Vec<u8>>,
    regp: Option<Vec<u8>>,
    /// A raw chunk inserted before ANIM.
    before_anim: Option<Vec<u8>>,
    indx_first: bool,
}

impl T {
    fn new() -> T {
        let mut vert = hex("00 00  03 00  01 40 00 80");
        vert.extend(t_vertices());
        T {
            major: 1,
            minor: 5,
            len: None,
            vert,
            indx: hex("01 00 00 00  00 00  02 00  03 00  00 00 01 00 02 00"),
            prim: hex("01 00 00 00  01 00  00 00 00 00  00 00 00 00  03 00 00 00"),
            mats: hex(&format!("01 00 00 00 {MATERIAL} {TEXTURE_15}")),
            anim: hex(ANIM),
            prop: Some(hex("00 00 00 00")),
            regp: Some(hex("00 00 00 00")),
            before_anim: None,
            indx_first: false,
        }
    }

    /// T at an older version: the legacy VERT header (standard format 2, stride 20) for 1.0–1.3, no filter bytes,
    /// and PROP/REGP only where the version has them.
    fn old(major: u16, minor: u16) -> T {
        let mut t = T::new();
        t.major = major;
        t.minor = minor;
        let v = Version { major, minor };
        if v.legacy_vert() {
            t.vert = hex("00 00  03 00  02 00  14 00");
            t.vert.extend(t_vertices());
        }
        t.mats = hex(&format!("01 00 00 00 {MATERIAL} {TEXTURE_OLD}"));
        if !v.has_prop() {
            t.prop = None;
        }
        if !v.has_regp() {
            t.regp = None;
        }
        t
    }

    fn chunk(&self, out: &mut Vec<u8>, tag: &[u8; 4], body: &[u8]) {
        out.extend_from_slice(tag);
        out.extend_from_slice(&self.len.unwrap_or(8 + body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
    }

    fn build(&self) -> Vec<u8> {
        let mut c = Vec::new();
        let mut head = self.major.to_le_bytes().to_vec();
        head.extend_from_slice(&self.minor.to_le_bytes());
        self.chunk(&mut c, b"HEAD", &head);
        let mut vert = 1u32.to_le_bytes().to_vec();
        vert.extend_from_slice(&self.vert);
        if self.indx_first {
            self.chunk(&mut c, b"INDX", &self.indx);
            self.chunk(&mut c, b"VERT", &vert);
        } else {
            self.chunk(&mut c, b"VERT", &vert);
            self.chunk(&mut c, b"INDX", &self.indx);
        }
        self.chunk(&mut c, b"PRIM", &self.prim);
        self.chunk(&mut c, b"MATS", &self.mats);
        if let Some(x) = &self.before_anim {
            c.extend_from_slice(x);
        }
        self.chunk(&mut c, b"ANIM", &self.anim);
        if let Some(p) = &self.prop {
            self.chunk(&mut c, b"PROP", p);
        }
        if let Some(r) = &self.regp {
            self.chunk(&mut c, b"REGP", r);
        }
        let mut out = b"3DMD".to_vec();
        out.extend_from_slice(&self.len.unwrap_or(8 + c.len() as u32).to_le_bytes());
        out.extend(c);
        out
    }

    fn parse(&self) -> Result<Model> {
        Model::parse_record(TYPE_S3D, &self.build())
    }

    fn model(&self) -> Model {
        self.parse().unwrap()
    }
}

/// The geometry of T: the vertex block's positions and uvs, the index list and the primitive.
fn assert_t_geometry(m: &Model) {
    let v = &m.vertex_blocks[0];
    assert_eq!(v.count, 3);
    assert_eq!(v.bbox.min, [0.0, 0.0, 0.0]);
    assert_eq!(v.bbox.max, [1.0, 1.0, 0.0]);
    assert_eq!(v.draw_stride(), 20);
    assert_eq!(m.index_blocks[0].indices, [0, 1, 2]);
    assert_eq!(
        m.primitive_blocks[0],
        [Primitive {
            kind: 0,
            first: 0,
            count: 3
        }]
    );
}

#[test]
fn t21_t_loads() {
    // The builder reproduces the spec's byte listing exactly.
    let listing = hex(concat!(
        "33444D44 FC000000",
        "48454144 0C000000 0100 0500",
        "56455254 50000000 01000000 0000 0300 01400080",
        "00000000 00000000 00000000 00000000 00000000",
        "0000803F 00000000 00000000 0000803F 00000000",
        "00000000 0000803F 00000000 00000000 0000803F",
        "494E4458 18000000 01000000 0000 0200 0300 0000 0100 0200",
        "5052494D 1A000000 01000000 0100 00000000 00000000 03000000",
        "4D415453 2A000000 01000000 AB000000 04030100 FF7F 00000000 00 01",
        "78563412 03030101 2100 0200 0100",
        "414E494D 24000000 0100 1E00 0300 00000000 00000000 0100",
        "0200 6D00 0000 0000 0000 0000",
        "50524F50 0C000000 00000000",
        "52454750 0C000000 00000000",
    ));
    assert_eq!(T::new().build(), listing);
    assert_eq!(listing.len(), 252);

    let m = Model::parse_record(TYPE_S3D, &listing).unwrap();
    assert_eq!(m.version, Version { major: 1, minor: 5 });
    assert_eq!(m.anim.frame_count, 1);
    assert_eq!((m.anim.frame_rate, m.anim.mode, m.anim.flags), (30, 3, 0));
    assert_eq!(m.anim.meshes.len(), 1);
    assert_eq!(m.anim.meshes[0].name, "m");
    let v = &m.vertex_blocks[0];
    assert_eq!(
        (v.format, v.packed_format(), v.stride),
        (0x8000_4001, VertexFormat(0x8000_4001), 20)
    );
    assert_t_geometry(&m);

    let meshes = m.frame(0).unwrap();
    assert_eq!(meshes.len(), 1);
    let mesh = &meshes[0];
    assert!(mesh.is_drawn());
    assert!(mesh.ranges_fit());
    let calls = mesh.draw_calls();
    assert_eq!(
        calls,
        [DrawCall::Elements {
            primitive: 0,
            first: 0,
            count: 3,
            indices: Some(&[0, 1, 2][..])
        }]
    );
    assert_eq!(PrimitiveType::from_u32(0), Some(PrimitiveType::TriangleList));

    let mat = mesh.material;
    assert_eq!(mat.flags, 0x1AB);
    assert!(mat.has(material_flags::ALWAYS));
    assert_eq!(CompareFunc::from_u8(mat.alpha_func), Some(CompareFunc::Greater));
    assert_eq!(mat.alpha_ref_level(), 7);
    assert_eq!(mat.alpha_ref(), 7.0 / 15.0);
    assert_eq!(CompareFunc::from_u8(mat.depth_func), Some(CompareFunc::LessEqual));
    assert_eq!(
        (BlendFactor::from_u8(mat.src_blend), BlendFactor::from_u8(mat.dst_blend)),
        (Some(BlendFactor::One), Some(BlendFactor::Zero))
    );
    assert_eq!(mat.active_textures().len(), 1);
    let tex = &mat.active_textures()[0];
    assert_eq!(tex.id, 0x1234_5678);
    let b = crate::keys::resolve_texture(0xBADB_57F1, tex.id, |_| true).unwrap();
    assert_eq!(b.key, crate::tgi::Tgi::new(0x7AB5_0E44, 0x1ABE_787D, 0x1234_5678));
    assert_eq!(
        (TextureParam::from_u8(tex.wrap_s), TextureParam::from_u8(tex.wrap_t)),
        (Some(TextureParam::Repeat), Some(TextureParam::Repeat))
    );
    assert_eq!(
        (
            TextureParam::from_u8(tex.mag_filter),
            TextureParam::from_u8(tex.min_filter)
        ),
        (Some(TextureParam::Linear), Some(TextureParam::Linear))
    );
    assert_eq!((tex.anim_rate, tex.anim_mode, tex.name.as_str()), (0x21, 2, ""));
    assert!(m.frame(1).is_none());
    assert!(m.props.is_empty() && m.regions.is_empty());
}

#[test]
fn t22_lengths_ignored() {
    let base = T::new().model();
    for len in [0, 0xFFFF_FFFF] {
        let mut t = T::new();
        t.len = Some(len);
        assert_eq!(t.model(), base);
    }
}

#[test]
fn t23_version_1_3() {
    let t = T::old(1, 3);
    assert!(t.prop.is_some() && t.regp.is_some());
    let m = t.model();
    assert_t_geometry(&m);
    assert_eq!((m.vertex_blocks[0].format, m.vertex_blocks[0].stride), (2, 20));
    let tex = &m.materials[0].textures[0];
    assert_eq!((tex.mag_filter, tex.min_filter), (0, 0));
    assert_eq!(TextureParam::from_u8(tex.mag_filter), Some(TextureParam::Nearest));
    // Stride 24: 72 vertex bytes are read, the bounding box steps by 24, the draw uses 20.
    let mut t = T::old(1, 3);
    t.vert = hex("00 00  03 00  02 00  18 00");
    for v in t_vertices().chunks(20) {
        t.vert.extend_from_slice(v);
        t.vert.extend_from_slice(&[0xEE; 4]);
    }
    let m = t.model();
    let v = &m.vertex_blocks[0];
    assert_eq!((v.stride, v.draw_stride(), v.data.len()), (24, 20, 72));
    assert_eq!((v.bbox.min, v.bbox.max), ([0.0; 3], [1.0, 1.0, 0.0]));
    // The 72 bytes are consumed: the rest of the model still parses.
    assert_eq!(m.index_blocks[0].indices, [0, 1, 2]);
}

#[test]
fn t24_version_1_0() {
    let t = T::old(1, 0);
    assert!(t.prop.is_none() && t.regp.is_none());
    assert_t_geometry(&t.model());
    let mut t = T::old(1, 0);
    t.prop = Some(hex("00 00 00 00"));
    t.regp = Some(hex("00 00 00 00"));
    assert_t_geometry(&t.model());
}

#[test]
fn t25_version_1_1() {
    let t = T::old(1, 1);
    assert!(t.prop.is_some() && t.regp.is_none());
    assert_t_geometry(&t.model());
    let mut t = T::old(1, 1);
    t.prop = None;
    assert!(t.parse().is_err());
}

#[test]
fn t26_version_0_x() {
    let t = T::old(0, 9);
    assert!(t.prop.is_none() && t.regp.is_none());
    assert_eq!(t.vert[4..8], hex("01 40 00 80")[..]); // the new VERT layout
    let m = t.model();
    assert_t_geometry(&m);
    assert_eq!(m.materials[0].textures[0].min_filter, 0);
}

#[test]
fn t27_rejected_versions() {
    let mut t = T::new();
    t.minor = 6;
    assert!(matches!(t.parse(), Err(Error::Unsupported { .. })));
    let mut t = T::new();
    t.major = 2;
    t.minor = 0;
    assert!(matches!(t.parse(), Err(Error::Unsupported { .. })));
    // 1.2 and 1.4 load.
    assert_t_geometry(&T::old(1, 2).model());
    assert_t_geometry(&T::old(1, 4).model());
}

#[test]
fn t28_chunk_order() {
    let mut t = T::new();
    t.indx_first = true;
    assert!(matches!(t.parse(), Err(Error::BadMagic { .. })));
    let mut t = T::new();
    t.before_anim = Some(hex("58 58 58 58 08 00 00 00"));
    assert!(matches!(t.parse(), Err(Error::BadMagic { .. })));
    // A missing tag, or a model cut anywhere, fails.
    let mut t = T::new();
    t.regp = None;
    assert!(t.parse().is_err());
    let full = T::new().build();
    for n in 0..full.len() {
        assert!(Model::parse(&full[..n]).is_err(), "prefix {n}");
    }
    // Trailing bytes after REGP are ignored.
    let mut longer = full.clone();
    longer.extend_from_slice(b"junk");
    assert_eq!(Model::parse(&longer).unwrap(), Model::parse(&full).unwrap());
}

#[test]
fn t29_index_size() {
    let mut t = T::new();
    t.indx = hex("01 00 00 00  00 00  04 00  03 00  00 00 01 00 02 00");
    match t.parse() {
        Err(Error::Corrupt { detail, .. }) => assert!(detail.contains("INDX"), "{detail}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn t30_empty_index_block() {
    let mut t = T::new();
    t.indx = hex("01 00 00 00  00 00  02 00  00 00");
    let m = t.model();
    assert!(m.index_blocks[0].indices.is_empty());
    let meshes = m.frame(0).unwrap();
    assert_eq!(
        meshes[0].draw_calls(),
        [DrawCall::Arrays {
            primitive: 0,
            first: 0,
            count: 3
        }]
    );
    assert!(meshes[0].ranges_fit());
}

#[test]
fn t31_dropped_mesh() {
    let mut t = T::new();
    t.anim = hex("01 00 1E 00 03 00  00 00 00 00  00 00 00 00  01 00  02 00 6D 00  FF FF 00 00 00 00 00 00");
    let m = t.model();
    assert_eq!((m.anim.meshes.len(), m.anim.dropped), (0, 1));
    assert!(m.frame(0).unwrap().is_empty());
    let mut t = T::new();
    t.anim = hex("01 00 1E 00 03 00  00 00 00 00  00 00 00 00  01 00  02 00 6D 00  00 00 00 00 01 00 00 00");
    let m = t.model();
    assert_eq!((m.anim.meshes.len(), m.anim.dropped), (0, 1));
    // One bad frame of two drops the mesh; a good mesh next to it stays.
    let mut t = T::new();
    t.anim = hex(concat!(
        "02 00 1E 00 03 00  00 00 00 00  00 00 00 00  02 00",
        "02 00 61 00  00 00 00 00 00 00 00 00  00 00 00 00 00 00 01 00",
        "02 00 62 00  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00"
    ));
    let m = t.model();
    assert_eq!((m.anim.meshes.len(), m.anim.dropped), (1, 1));
    assert_eq!(m.anim.meshes[0].name, "b");
    assert_eq!(m.frame(1).unwrap().len(), 1);
    assert!(m.frame(2).is_none());
}

#[test]
fn t32_texture_count() {
    let mut t = T::new();
    t.mats = hex(&format!("01 00 00 00 {}", MATERIAL.replace("00  01", "00  00")));
    let mat = &t.model().materials[0];
    assert_eq!((mat.textures.len(), mat.texture_count), (0, 0));
    assert!(mat.active_textures().is_empty());
    let mut t = T::new();
    t.mats = hex(&format!(
        "01 00 00 00 {MATERIAL} {}",
        TEXTURE_15.replace("78 56 34 12", "00 00 00 00")
    ));
    let mat = &t.model().materials[0];
    assert_eq!((mat.textures.len(), mat.texture_count), (1, 0));
    // Two textures, the last ID 0: T becomes 0; the first ID 0: T stays 2.
    let two = |a: &str, b: &str| {
        let mut t = T::new();
        t.mats = hex(&format!(
            "01 00 00 00 {} {} {}",
            MATERIAL.replace("00  01", "00  02"),
            TEXTURE_15.replace("78 56 34 12", a),
            TEXTURE_15.replace("78 56 34 12", b)
        ));
        t.model().materials[0].texture_count
    };
    assert_eq!(two("01 00 00 00", "00 00 00 00"), 0);
    assert_eq!(two("00 00 00 00", "02 00 00 00"), 2);
    let mut t = T::new();
    t.mats = hex(&format!(
        "01 00 00 00 {} {TEXTURE_15} {TEXTURE_15} {TEXTURE_15}",
        MATERIAL.replace("00  01", "00  03")
    ));
    assert!(matches!(t.parse(), Err(Error::Corrupt { .. })));
}

fn material_with(flags: &str, threshold: &str) -> Material {
    let mut t = T::new();
    let m = MATERIAL.replace("AB 00 00 00", flags).replace("FF 7F", threshold);
    t.mats = hex(&format!("01 00 00 00 {m} {TEXTURE_15}"));
    t.model().materials.remove(0)
}

#[test]
fn t33_flags() {
    let m = material_with("22 00 01 00", "FF 7F");
    assert_eq!(m.stored_flags, 0x0001_0022);
    assert_eq!(m.flags, 0x01A2);
    assert!(m.has(material_flags::BIND_TEXTURES | material_flags::DEPTH_TEST));
    assert!(!m.has(material_flags::ALPHA_TEST));
}

/// A foreign system-service base is irrelevant to material construction.
struct MaterialBase;
impl fc4_core::textures_materials::SystemService for MaterialBase {
    fn query_interface(&mut self, _iid: u32) -> Option<u32> {
        None
    }
    fn add_ref(&mut self) -> u32 {
        1
    }
    fn release(&mut self) -> u32 {
        0
    }
    fn destroy_base(&mut self) {
        // This independent reader/owner test uses a stateless foreign base.
    }
}

#[test]
fn parsed_mats_feeds_portable_owner_with_versioned_filters() {
    use fc4_core::textures_materials::{MaterialManager, TextureSlot};
    let manager = MaterialManager::new(MaterialBase);
    for (model, filter) in [(T::new().model(), 1), (T::old(1, 4).model(), 0)] {
        let material = manager.create_material(&model.materials[0]).unwrap();
        assert_eq!((material.references, material.header_tail), (1, None));
        assert_eq!((material.flags, material.alpha_threshold), (0x01AB, 0x7F));
        assert_eq!(
            (
                material.alpha_func,
                material.depth_func,
                material.src_blend,
                material.dst_blend
            ),
            (4, 3, 1, 0)
        );
        assert_eq!(
            material.textures[0],
            TextureSlot {
                id: Some(0x1234_5678),
                mag_filter: filter,
                min_filter: filter,
                ..TextureSlot::default()
            }
        );
        assert_eq!(
            material.textures[1],
            TextureSlot {
                environment: 0xFF,
                ..TextureSlot::default()
            }
        );
    }
}

#[test]
fn parsed_last_id_zero_quirk_stays_in_reader_and_marks_both_owner_slots_unused() {
    use fc4_core::textures_materials::MaterialManager;
    let mut t = T::new();
    t.mats = hex(&format!(
        "01 00 00 00 {MATERIAL} {}",
        TEXTURE_15.replace("78 56 34 12", "00 00 00 00")
    ));
    let model = t.model();
    assert_eq!(model.materials[0].textures.len(), 1);
    let material = MaterialManager::new(MaterialBase)
        .create_material(&model.materials[0])
        .unwrap();
    assert!(material
        .textures
        .iter()
        .all(|slot| slot.environment == 0xFF && slot.id.is_none()));
}

#[test]
fn t34_alpha_reference() {
    for (threshold, level) in [("00 80", 8), ("00 00", 0), ("00 FF", 15), ("7F 08", 0), ("FF 7F", 7)] {
        let m = material_with("AB 00 00 00", threshold);
        assert_eq!(m.alpha_ref_level(), level, "threshold {threshold}");
        assert_eq!(m.alpha_ref(), f32::from(level) / 15.0);
    }
    // Every high byte rounds as round(b / 255 × 15).
    for b in 0..=255u16 {
        let m = material_with("AB 00 00 00", &format!("00 {b:02X}"));
        assert_eq!(f64::from(m.alpha_ref_level()), (f64::from(b) / 255.0 * 15.0).round());
    }
}

#[test]
fn t35_vertex_formats() {
    let base = T::new().model();
    let mut t = T::new();
    t.vert[4..8].copy_from_slice(&2u32.to_le_bytes());
    let m = t.model();
    assert_eq!((m.vertex_blocks[0].format, m.vertex_blocks[0].stride), (2, 20));
    assert_eq!(m.vertex_blocks[0].data, base.vertex_blocks[0].data);
    assert_t_geometry(&m);

    // 0x80004101: a colour dword after the position, stride 24, uv at 16.
    let mut t = T::new();
    t.vert = hex("00 00  03 00  01 41 00 80");
    for v in t_vertices().chunks(20) {
        t.vert.extend_from_slice(&v[..12]);
        t.vert.extend_from_slice(&[0x10, 0x20, 0x30, 0xFF]);
        t.vert.extend_from_slice(&v[12..]);
    }
    let m = t.model();
    let v = &m.vertex_blocks[0];
    assert_eq!((v.stride, v.draw_stride(), v.data.len()), (24, 24, 72));
    let f = v.packed_format();
    assert_eq!(f.element_offset(kind::TEX2), Some(16));
    assert_eq!(f.element_offset(kind::COLOUR), Some(12));
    assert_eq!(f.element_offset(kind::POSITION), Some(0));
    assert_eq!(f.element_offset(kind::NORMAL), None);
    assert_eq!((v.bbox.min, v.bbox.max), ([0.0; 3], [1.0, 1.0, 0.0]));

    // Standard format 4 is unknown: FreeCity4 rejects it.
    let mut t = T::new();
    t.vert[4..8].copy_from_slice(&4u32.to_le_bytes());
    assert!(matches!(t.parse(), Err(Error::Corrupt { .. })));
}

#[test]
fn t35b_format_tables() {
    let strides = [
        (1, 16),
        (2, 20),
        (3, 28),
        (10, 24),
        (11, 32),
        (32, 12),
        (33, 24),
        (34, 28),
        (35, 32),
        (36, 40),
        (37, 36),
        (38, 44),
    ];
    for (n, stride) in strides {
        assert_eq!(VertexFormat::from_stored(n).stride(), stride, "standard {n}");
        assert_eq!(VertexFormat::from_stored(n), VertexFormat(pack_standard(n)));
    }
    for n in [0, 4, 9, 12, 31, 39, 0x7FFF_FFFF] {
        assert_eq!(VertexFormat::from_stored(n).stride(), 0, "standard {n}");
    }
    // Kinds 1-4, 8 and 9 count only when bits 0-3 are not all zero.
    assert_eq!(VertexFormat(0x8000_0040).stride(), 0); // a normal without a position
    assert_eq!(VertexFormat(0x8000_0044).stride(), 12 + 4); // ... with one weight (bits 2-4)
    assert_eq!(VertexFormat(0x8040_0001).stride(), 12 + 16); // a 4-component texture coordinate
    assert_eq!(VertexFormat(0x8000_0100).stride(), 4); // a colour alone counts
    assert_eq!(VertexFormat(0x8000_0003).stride(), 36); // three positions
                                                        // Packed formats in a 1.5 block; a stride below 12 is rejected [choice].
    let mut t = T::new();
    t.vert = hex("00 00  01 00  00 01 00 80  01 02 03 04");
    assert!(matches!(t.parse(), Err(Error::Corrupt { .. })));
    // A legacy stride below the standard one is rejected [choice].
    let mut t = T::old(1, 3);
    t.vert = hex("00 00  03 00  02 00  10 00");
    t.vert.extend_from_slice(&[0; 48]);
    assert!(matches!(t.parse(), Err(Error::Corrupt { .. })));
    // N = 0: accepted, empty box, not drawn.
    let mut t = T::new();
    t.vert = hex("00 00  00 00  01 40 00 80");
    let m = t.model();
    assert_eq!(m.vertex_blocks[0].bbox, BoundingBox::EMPTY);
    assert!(m.vertex_blocks[0].bbox.is_empty());
    let meshes = m.frame(0).unwrap();
    assert!(!meshes[0].is_drawn());
    assert!(meshes[0].draw_calls().is_empty());
}

#[test]
fn t36_strings() {
    let mut t = T::new();
    t.anim = hex("01 00 1E 00 03 00  00 00 00 00  00 00 00 00  01 00  01 00 6D  00 00 00 00 00 00 00 00");
    assert_eq!(t.model().anim.meshes[0].name, "m");
    // Bytes after the NUL are not part of the name; an empty name is fine.
    let mut t = T::new();
    t.anim = hex("01 00 1E 00 03 00  00 00 00 00  00 00 00 00  01 00  03 00 6D 00 6E  00 00 00 00 00 00 00 00");
    assert_eq!(t.model().anim.meshes[0].name, "m");
    let mut t = T::new();
    t.anim = hex("01 00 1E 00 03 00  00 00 00 00  00 00 00 00  01 00  00 00  00 00 00 00 00 00 00 00");
    assert_eq!(t.model().anim.meshes[0].name, "");
}

#[test]
fn t37_regp() {
    let mut body = 2u32.to_le_bytes().to_vec();
    for (name, base) in [(&b"FX_A\0"[..], 1.0f32), (&b"fx_a\0"[..], 8.0)] {
        body.push(name.len() as u8);
        body.extend_from_slice(name);
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend(f32s(&[
            base,
            base + 1.,
            base + 2.,
            base + 3.,
            base + 4.,
            base + 5.,
            base + 6.,
        ]));
    }
    let mut t = T::new();
    t.regp = Some(body);
    let m = t.model();
    assert_eq!(m.regions.len(), 1);
    assert_eq!(
        m.regions["fx_a"],
        [[1., 2., 3., 4., 5., 6., 7.], [8., 9., 10., 11., 12., 13., 14.]]
    );
}

/// FS12 (§3.11): a valid UTF-8 name is lowered letter by letter (Unicode simple case mapping), so `FX_Ä` and
/// `fx_ä` merge; a name that is not UTF-8 is Latin-1 and only its ASCII letters are lowered.
#[test]
fn regp_non_ascii_names() {
    let mut body = 4u32.to_le_bytes().to_vec();
    for name in [
        &b"FX_\xC3\x84\0"[..],         // "FX_Ä" in UTF-8
        &b"fx_\xC3\xA4\0"[..],         // "fx_ä"
        &b"FX_\xC4\xB0\xCE\xA3\0"[..], // "FX_İΣ": simple mappings i and σ
        &b"FX_\xC4\0"[..],             // not UTF-8: Latin-1 "FX_Ä"
    ] {
        body.push(name.len() as u8);
        body.extend_from_slice(name);
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend(f32s(&[0.0; 7]));
    }
    let mut t = T::new();
    t.regp = Some(body);
    let m = t.model();
    let names: Vec<&str> = m.regions.keys().map(String::as_str).collect();
    assert_eq!(names, ["fx_iσ", "fx_Ä", "fx_ä"]);
    assert_eq!(m.regions["fx_ä"].len(), 2);
    assert_eq!(simple_lowercase('\u{130}'), 'i');
    assert_eq!(simple_lowercase('Σ'), 'σ');
    assert_eq!(simple_lowercase('ß'), 'ß');
}

#[test]
fn prop_entries() {
    let mut body = 2u32.to_le_bytes().to_vec();
    for (key, value) in [("skeleton", "adult"), ("suit", "RangerFFit_01")] {
        body.extend_from_slice(&(-1i16).to_le_bytes());
        body.extend_from_slice(&0i16.to_le_bytes());
        for s in [key, value] {
            body.push(s.len() as u8 + 1);
            body.extend_from_slice(s.as_bytes());
            body.push(0);
        }
    }
    let mut t = T::new();
    t.prop = Some(body);
    let m = t.model();
    assert_eq!(
        m.props,
        [
            Prop {
                mesh: -1,
                frame: 0,
                key: "skeleton".into(),
                value: "adult".into()
            },
            Prop {
                mesh: -1,
                frame: 0,
                key: "suit".into(),
                value: "RangerFFit_01".into()
            }
        ]
    );
}

/// FS11 (§3.10): PROP is sorted by frame, signed and ascending; equal frames keep file order *\[choice\]*.
#[test]
fn prop_sorted_by_frame() {
    let entries: [(i16, i16, &str); 5] = [(0, 3, "a"), (1, -1, "b"), (2, 0, "c"), (3, -1, "d"), (4, -32768, "e")];
    let mut body = (entries.len() as u32).to_le_bytes().to_vec();
    for (mesh, frame, key) in entries {
        body.extend_from_slice(&mesh.to_le_bytes());
        body.extend_from_slice(&frame.to_le_bytes());
        for s in [key, "v"] {
            body.push(s.len() as u8 + 1);
            body.extend_from_slice(s.as_bytes());
            body.push(0);
        }
    }
    let mut t = T::new();
    t.prop = Some(body);
    let m = t.model();
    let order: Vec<(i16, i16, &str)> = m.props.iter().map(|p| (p.mesh, p.frame, p.key.as_str())).collect();
    assert_eq!(
        order,
        [(4, -32768, "e"), (1, -1, "b"), (3, -1, "d"), (2, 0, "c"), (0, 3, "a")]
    );
}

#[test]
fn draw_ranges() {
    // A primitive whose range leaves the index block: no slice, and ranges_fit is false.
    let mut t = T::new();
    t.prim = hex("01 00 00 00  01 00  00 00 00 00  02 00 00 00  03 00 00 00");
    let m = t.model();
    let mesh = m.frame(0).unwrap()[0];
    assert!(matches!(mesh.draw_calls()[0], DrawCall::Elements { indices: None, .. }));
    assert!(!mesh.ranges_fit());
    // An index past the vertex block.
    let mut t = T::new();
    t.indx = hex("01 00 00 00  00 00  02 00  03 00  00 00 01 00 03 00");
    assert!(!t.model().frame(0).unwrap()[0].ranges_fit());
    // Arrays past the vertex block.
    let mut t = T::new();
    t.indx = hex("01 00 00 00  00 00  02 00  00 00");
    t.prim = hex("01 00 00 00  01 00  00 00 00 00  01 00 00 00  03 00 00 00");
    assert!(!t.model().frame(0).unwrap()[0].ranges_fit());
    assert_eq!(PrimitiveType::from_u32(8), None);
}

#[test]
fn wrong_type() {
    assert!(matches!(
        Model::parse_record(0x7AB5_0E44, &T::new().build()),
        Err(Error::Unsupported { .. })
    ));
}

/// Truncations and single-byte corruptions: parse and frame building return, never panic.
#[test]
fn hostile_input() {
    let mut animated = T::new();
    animated.anim = hex(concat!(
        "02 00 1E 00 03 00  00 00 00 00  00 00 00 00  02 00",
        "02 00 61 00  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00",
        "02 00 62 00  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00"
    ));
    for t in [T::new(), T::old(1, 3), T::old(0, 1), animated] {
        let d = t.build();
        for i in 0..d.len() {
            for v in [0x00, 0x01, 0x7F, 0x80, 0xFF] {
                let mut m = d.clone();
                m[i] = v;
                if let Ok(model) = Model::parse(&m) {
                    for f in 0..usize::from(model.anim.frame_count) {
                        for mesh in model.frame(f).unwrap() {
                            let _ = mesh.ranges_fit();
                        }
                    }
                }
            }
        }
    }
    // Huge counts with no data fail before allocating.
    let mut t = T::new();
    t.vert = hex("00 00  FF FF  01 40 00 80");
    assert!(matches!(t.parse(), Err(Error::Truncated { .. })));
    let mut t = T::new();
    t.indx = hex("FF FF FF FF");
    assert!(matches!(t.parse(), Err(Error::Truncated { .. })));
}
