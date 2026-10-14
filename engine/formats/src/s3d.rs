// SPDX-License-Identifier: GPL-3.0-or-later
//! S3D (`3DMD`) models, type [`TYPE_S3D`], as the original loads them (`docs/specs/graphics-formats.md` §3).
//!
//! # Grammar (§3.2–§3.11)
//!
//! A fixed sequence of chunks, each a 4-byte tag (compared exactly) and a `u32` length that is **read and
//! ignored** (the shipped lengths are unreliable):
//!
//! ```text
//! "3DMD" len
//! "HEAD" len  u16 major, u16 minor          accepted: 0.x, 1.0-1.5
//! "VERT" len  u32 blocks × { u16 flags, u16 N,
//!                             1.0-1.3:          u16 format, u16 stride S
//!                             0.x, 1.4, 1.5:    u32 format (S = its standard stride)
//!                             S × N bytes }
//! "INDX" len  u32 blocks × { u16 flags, u16 size (must be 2), u16 K, K × u16 }
//! "PRIM" len  u32 blocks × { u16 P, P × (u32 type, u32 first, u32 count) }
//! "MATS" len  u32 count × material (flags, 4 enum bytes, u16 threshold, u32 class, u8 reserved, u8 T,
//!                                    T × texture: u32 ID, u8 wrap S, u8 wrap T, [1.5: u8 mag, u8 min],
//!                                                 u16 anim rate, u16 anim mode, u8 L + name)
//! "ANIM" len  u16 F, u16 rate, u16 mode, u32 flags, f32 displacement, u16 meshes ×
//!             { u8 L, u8 flags, L bytes name, F × 4 × s16 (VERT, INDX, PRIM, MATS index) }
//! "PROP" len  1.1 and later: u32 count × { s16 mesh, s16 frame, u8 L + key, u8 L + value }
//! "REGP" len  1.2 and later: u32 count × { u8 L + name (lower-cased), u16 C, C × 7 × f32 }
//! ```
//!
//! Anything after the last expected chunk is ignored. Material flags keep their low 16 bits and gain `0x180`; the
//! alpha reference keeps 4 bits of the threshold's high byte; if the last texture's ID is 0 the material has no
//! texture. A mesh with any invalid frame index (negative, or not below its chunk's count) is dropped. Names are C
//! strings (up to the first NUL). The bounding box of a vertex block is computed from the first three floats of
//! each vertex; the draw uses the format's standard stride.
//!
//! # FreeCity4 choices
//!
//! *\[choice\]* rejections where the original misreads or overflows (Q11–Q13): an unknown vertex format (standard
//! stride 0), an index size other than 2, more than 2 textures in a material. Also rejected *\[choice\]*: a vertex
//! format whose stride is below 12 (the bounding box reads 12 bytes per vertex), and a legacy block whose stored
//! stride is below the format's standard stride (the draw would read past the block). A vertex block with N = 0
//! is accepted with the empty box. A name without a NUL is its whole stored bytes (test 36). Names are decoded as
//! UTF-8 when valid, else Latin-1. PROP entries keep the file order (the spec says the array is sorted but not by
//! what). REGP names are lower-cased in ASCII.

use std::collections::BTreeMap;

use crate::error::{corrupt, Error, Result};
use crate::reader::Reader;
use crate::tgi::TYPE_S3D;

/// A model's format version (`HEAD`, §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
}

impl Version {
    /// Major 0 with any minor, or major 1 with minor 0–5.
    pub fn is_accepted(self) -> bool {
        self.major == 0 || (self.major == 1 && self.minor <= 5)
    }

    /// The legacy VERT block header (`u16` format, `u16` stride): 1.0–1.3 only (0.x takes the new one *\[quirk\]*).
    pub fn legacy_vert(self) -> bool {
        self.major == 1 && self.minor <= 3
    }

    /// Material textures carry the mag/min filter bytes: 1.5 only.
    pub fn has_texture_filters(self) -> bool {
        self.major == 1 && self.minor >= 5
    }

    /// The `PROP` chunk: 1.1 and later.
    pub fn has_prop(self) -> bool {
        self.major == 1 && self.minor >= 1
    }

    /// The `REGP` chunk: 1.2 and later.
    pub fn has_regp(self) -> bool {
        self.major == 1 && self.minor >= 2
    }
}

/// Vertex element kinds of the packed format (§3.5), in layout order.
pub mod kind {
    /// Position, 3 × f32 (bits 0–1).
    pub const POSITION: usize = 0;
    /// 1 × f32, inferred weight (bits 2–4).
    pub const WEIGHT: usize = 1;
    /// 4 bytes (bit 5).
    pub const KIND2: usize = 2;
    /// Normal, 3 × f32 (bit 6).
    pub const NORMAL: usize = 3;
    /// 4 bytes (bit 7).
    pub const KIND4: usize = 4;
    /// 32-bit colour (bits 8–9; byte order GF6).
    pub const COLOUR: usize = 5;
    /// 1-component texture coordinates (bits 10–13).
    pub const TEX1: usize = 6;
    /// 2-component texture coordinates, u and v as f32 (bits 14–17).
    pub const TEX2: usize = 7;
    /// 3-component texture coordinates (bits 18–21).
    pub const TEX3: usize = 8;
    /// 4-component texture coordinates (bits 22–25).
    pub const TEX4: usize = 9;
}

/// (bit position, bit width, bytes per element) of each kind (§3.5).
const KINDS: [(u32, u32, u32); 10] = [
    (0, 2, 12),
    (2, 3, 4),
    (5, 1, 4),
    (6, 1, 12),
    (7, 1, 4),
    (8, 2, 4),
    (10, 4, 4),
    (14, 4, 8),
    (18, 4, 12),
    (22, 4, 16),
];

/// Standard vertex format numbers and their packed forms (§3.5, `PackStandardVertexFormat`).
pub const STANDARD_FORMATS: [(u32, u32); 12] = [
    (1, 0x8000_0101),
    (2, 0x8000_4001),
    (3, 0x8000_8001),
    (10, 0x8000_4101),
    (11, 0x8000_8101),
    (32, 0x8000_0001),
    (33, 0x8000_0041),
    (34, 0x8000_0141),
    (35, 0x8000_4041),
    (36, 0x8000_8041),
    (37, 0x8000_4141),
    (38, 0x8000_8141),
];

/// The packed form of a standard format number; 0 for a number not in the table.
pub fn pack_standard(n: u32) -> u32 {
    STANDARD_FORMATS.iter().find(|(s, _)| *s == n).map_or(0, |(_, p)| *p)
}

/// A vertex format in its packed form (bit 31 set, or 0 for an unknown standard number).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VertexFormat(pub u32);

impl VertexFormat {
    /// The packed form of a stored format: itself when bit 31 is set, else the standard number translated.
    pub fn from_stored(format: u32) -> Self {
        if format & 0x8000_0000 != 0 {
            VertexFormat(format)
        } else {
            VertexFormat(pack_standard(format))
        }
    }

    /// The element count of a kind (0 for a kind above 9).
    pub fn count(self, kind: usize) -> u32 {
        KINDS
            .get(kind)
            .map_or(0, |&(shift, width, _)| (self.0 >> shift) & ((1 << width) - 1))
    }

    /// Whether a kind counts in the stride: kinds 1–4, 8 and 9 only when bits 0–3 of the format are not all zero
    /// *\[quirk\]* (§3.5).
    fn counted(self, kind: usize) -> bool {
        !matches!(kind, 1..=4 | 8 | 9) || self.0 & 0xF != 0
    }

    /// `StandardVFStride`: the sum of count × size over the counted kinds.
    pub fn stride(self) -> u32 {
        (0..KINDS.len())
            .filter(|&k| self.counted(k))
            .map(|k| self.count(k) * KINDS[k].2)
            .sum()
    }

    /// Byte offset of a kind's first element in a vertex, in the plain kind-order layout *\[choice\]* (§3.5:
    /// the original's offset table leaves out the kind just below, which changes nothing for the standard
    /// formats). `None` when the format has no such element.
    pub fn element_offset(self, kind: usize) -> Option<u32> {
        if kind >= KINDS.len() || self.count(kind) == 0 || !self.counted(kind) {
            return None;
        }
        Some(
            (0..kind)
                .filter(|&k| self.counted(k))
                .map(|k| self.count(k) * KINDS[k].2)
                .sum(),
        )
    }
}

/// An axis-aligned box computed from vertex positions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundingBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl BoundingBox {
    /// The box of a new vertex block before any vertex: min (1, 1, 1), max (0, 0, 0) (§3.4).
    pub const EMPTY: BoundingBox = BoundingBox {
        min: [1.0; 3],
        max: [0.0; 3],
    };

    /// True for a box with min above max on some axis (the empty box).
    pub fn is_empty(&self) -> bool {
        (0..3).any(|i| self.min[i] > self.max[i])
    }
}

/// One vertex block (§3.4).
#[derive(Clone, Debug, PartialEq)]
pub struct VertexBlock {
    /// Read, ignored.
    pub flags: u16,
    /// The format as stored (a standard number or a packed format).
    pub format: u32,
    /// The stride S the vertices were read with: as stored in a legacy block, else the standard stride.
    pub stride: u32,
    /// N.
    pub count: u16,
    /// S × N bytes of interleaved vertices.
    pub data: Vec<u8>,
    /// Computed from the first three floats of each vertex, at step S.
    pub bbox: BoundingBox,
}

impl VertexBlock {
    /// The packed format.
    pub fn packed_format(&self) -> VertexFormat {
        VertexFormat::from_stored(self.format)
    }

    /// The stride the draw uses: always the format's standard stride (§3.4; a legacy S that differs is ignored
    /// there *\[quirk\]*).
    pub fn draw_stride(&self) -> u32 {
        self.packed_format().stride()
    }
}

/// One index block (§3.6). The index size is always 2 here (other sizes are rejected).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexBlock {
    /// Read, ignored.
    pub flags: u16,
    /// Empty: the block has no index buffer and meshes using it draw vertices directly.
    pub indices: Vec<u16>,
}

/// The renderer's primitive enum (§3.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    TriangleList = 0,
    TriangleStrip = 1,
    TriangleFan = 2,
    Points = 3,
    Lines = 4,
    LineStrip = 5,
    Quads = 6,
    QuadStrip = 7,
}

impl PrimitiveType {
    /// `None` above 7 (the DX7 driver ignores those).
    pub fn from_u32(v: u32) -> Option<Self> {
        use PrimitiveType as P;
        Some(match v {
            0 => P::TriangleList,
            1 => P::TriangleStrip,
            2 => P::TriangleFan,
            3 => P::Points,
            4 => P::Lines,
            5 => P::LineStrip,
            6 => P::Quads,
            7 => P::QuadStrip,
            _ => return None,
        })
    }
}

/// One primitive of a primitive block (§3.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Primitive {
    /// The primitive type number ([`PrimitiveType::from_u32`]).
    pub kind: u32,
    pub first: u32,
    pub count: u32,
}

/// Material flag bits (§3.8).
pub mod material_flags {
    pub const ALPHA_TEST: u16 = 0x0001;
    pub const DEPTH_TEST: u16 = 0x0002;
    pub const BACK_FACE_CULLING: u16 = 0x0008;
    pub const BLENDING: u16 = 0x0010;
    /// Bind the material's textures (texturing itself follows the texture count).
    pub const BIND_TEXTURES: u16 = 0x0020;
    pub const DEPTH_WRITE: u16 = 0x0080;
    pub const COLOUR_WRITE: u16 = 0x0100;
    /// Always added to the stored flags.
    pub const ALWAYS: u16 = DEPTH_WRITE | COLOUR_WRITE;
}

/// The compare enum (§3.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompareFunc {
    Never = 0,
    Less = 1,
    Equal = 2,
    LessEqual = 3,
    Greater = 4,
    NotEqual = 5,
    GreaterEqual = 6,
    Always = 7,
}

impl CompareFunc {
    pub fn from_u8(v: u8) -> Option<Self> {
        use CompareFunc as C;
        Some(match v {
            0 => C::Never,
            1 => C::Less,
            2 => C::Equal,
            3 => C::LessEqual,
            4 => C::Greater,
            5 => C::NotEqual,
            6 => C::GreaterEqual,
            7 => C::Always,
            _ => return None,
        })
    }
}

/// The blend enum (§3.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendFactor {
    Zero = 0,
    One = 1,
    SrcColour = 2,
    OneMinusSrcColour = 3,
    SrcAlpha = 4,
    OneMinusSrcAlpha = 5,
    DstAlpha = 6,
    OneMinusDstAlpha = 7,
    DstColour = 8,
    OneMinusDstColour = 9,
    SrcAlphaSaturate = 10,
}

impl BlendFactor {
    pub fn from_u8(v: u8) -> Option<Self> {
        use BlendFactor as B;
        Some(match v {
            0 => B::Zero,
            1 => B::One,
            2 => B::SrcColour,
            3 => B::OneMinusSrcColour,
            4 => B::SrcAlpha,
            5 => B::OneMinusSrcAlpha,
            6 => B::DstAlpha,
            7 => B::OneMinusDstAlpha,
            8 => B::DstColour,
            9 => B::OneMinusDstColour,
            10 => B::SrcAlphaSaturate,
            _ => return None,
        })
    }
}

/// The texture-parameter enum used by the wrap and filter bytes (§3.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureParam {
    Nearest = 0,
    Linear = 1,
    Clamp = 2,
    Repeat = 3,
    NearestMipmapNearest = 4,
    LinearMipmapNearest = 5,
    NearestMipmapLinear = 6,
    LinearMipmapLinear = 7,
}

impl TextureParam {
    pub fn from_u8(v: u8) -> Option<Self> {
        use TextureParam as T;
        Some(match v {
            0 => T::Nearest,
            1 => T::Linear,
            2 => T::Clamp,
            3 => T::Repeat,
            4 => T::NearestMipmapNearest,
            5 => T::LinearMipmapNearest,
            6 => T::NearestMipmapLinear,
            7 => T::LinearMipmapLinear,
            _ => return None,
        })
    }
}

/// One texture slot of a material (§3.8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialTexture {
    /// Instance of the FSH (§4.1, [`crate::keys::resolve_texture`]); 0 = none.
    pub id: u32,
    pub wrap_s: u8,
    pub wrap_t: u8,
    /// Version 1.5 only; 0 (nearest) before.
    pub mag_filter: u8,
    /// Version 1.5 only; 0 (nearest) before.
    pub min_filter: u8,
    /// Read, ignored by the game.
    pub anim_rate: u16,
    /// Read, ignored by the game.
    pub anim_mode: u16,
    /// Read, ignored by the game.
    pub name: String,
}

/// One material (§3.8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Material {
    /// The flags as stored.
    pub stored_flags: u32,
    /// The low 16 bits of the stored flags, OR `0x180` ([`material_flags`]).
    pub flags: u16,
    /// Compare enum ([`CompareFunc::from_u8`]).
    pub alpha_func: u8,
    /// Compare enum.
    pub depth_func: u8,
    /// Blend enum ([`BlendFactor::from_u8`]).
    pub src_blend: u8,
    /// Blend enum.
    pub dst_blend: u8,
    /// As stored; only the high byte is used ([`Material::alpha_ref_level`]).
    pub alpha_threshold: u16,
    /// Read, ignored.
    pub class: u32,
    /// The texture slots as stored (at most 2).
    pub textures: Vec<MaterialTexture>,
    /// The texture count the game uses: the stored count, or 0 when the last texture's ID is 0 *\[quirk\]*.
    pub texture_count: usize,
}

impl Material {
    /// The 4-bit alpha reference: round(high byte / 255 × 15).
    pub fn alpha_ref_level(&self) -> u8 {
        // high byte / 17, rounded; no value is exactly halfway.
        ((u32::from(self.alpha_threshold >> 8) + 8) / 17) as u8
    }

    /// The alpha reference, [`Material::alpha_ref_level`] / 15.
    pub fn alpha_ref(&self) -> f32 {
        f32::from(self.alpha_ref_level()) / 15.0
    }

    /// The textures the game uses (`texture_count` of them).
    pub fn active_textures(&self) -> &[MaterialTexture] {
        &self.textures[..self.texture_count.min(self.textures.len())]
    }

    /// True if every bit of `flag` is set in [`Material::flags`].
    pub fn has(&self, flag: u16) -> bool {
        self.flags & flag == flag
    }
}

/// One animated mesh (§3.9): its four block indices per frame (all valid: an invalid one drops the mesh).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimMesh {
    pub name: String,
    /// Read, stored.
    pub flags: u8,
    /// Per frame: VERT block, INDX block, PRIM block, MATS material.
    pub frames: Vec<[u16; 4]>,
}

/// The `ANIM` chunk (§3.9).
#[derive(Clone, Debug, PartialEq)]
pub struct Anim {
    /// F: frames 0 … F − 1 can be drawn.
    pub frame_count: u16,
    pub frame_rate: u16,
    /// Stored as a byte (the low byte of the `u16`).
    pub mode: u8,
    /// Stored as a byte (the low byte of the `u32`).
    pub flags: u8,
    pub displacement: f32,
    /// The meshes kept by the validation.
    pub meshes: Vec<AnimMesh>,
    /// Meshes dropped by the validation (§3.9).
    pub dropped: usize,
}

/// One `PROP` entry (§3.10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prop {
    pub mesh: i16,
    pub frame: i16,
    pub key: String,
    pub value: String,
}

/// One mesh of a frame (§3.12).
#[derive(Clone, Copy, Debug)]
pub struct Mesh<'a> {
    pub name: &'a str,
    pub flags: u8,
    pub vertices: &'a VertexBlock,
    pub indices: &'a IndexBlock,
    pub primitives: &'a [Primitive],
    pub material: &'a Material,
    /// The four block indices of this frame (VERT, INDX, PRIM, MATS).
    pub blocks: [u16; 4],
}

/// One draw of a primitive (§3.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawCall<'a> {
    /// `DrawElements(type, count, u16, indices + first)`: `indices` is that range of the index block, `None` when
    /// it does not fit (the original reads past the block; a renderer must not).
    Elements {
        primitive: u32,
        first: u32,
        count: u32,
        indices: Option<&'a [u16]>,
    },
    /// `DrawArrays(type, first, count)`: the mesh's index block is empty.
    Arrays { primitive: u32, first: u32, count: u32 },
}

impl<'a> Mesh<'a> {
    /// `RenderMesh` skips a mesh whose vertex block has no vertices.
    pub fn is_drawn(&self) -> bool {
        self.vertices.count > 0
    }

    /// The draws of this mesh, one per primitive (none when [`Mesh::is_drawn`] is false).
    pub fn draw_calls(&self) -> Vec<DrawCall<'a>> {
        if !self.is_drawn() {
            return Vec::new();
        }
        let indices: &'a [u16] = &self.indices.indices;
        self.primitives
            .iter()
            .map(|p| {
                if indices.is_empty() {
                    DrawCall::Arrays {
                        primitive: p.kind,
                        first: p.first,
                        count: p.count,
                    }
                } else {
                    let range = (p.first as usize).checked_add(p.count as usize);
                    DrawCall::Elements {
                        primitive: p.kind,
                        first: p.first,
                        count: p.count,
                        indices: range.and_then(|end| indices.get(p.first as usize..end)),
                    }
                }
            })
            .collect()
    }

    /// True if every draw stays inside its buffers: each element range inside the index block and each index
    /// below the vertex count, or each vertex range inside the vertex block. (The original does not check.)
    pub fn ranges_fit(&self) -> bool {
        let n = u64::from(self.vertices.count);
        self.draw_calls().iter().all(|d| match *d {
            DrawCall::Elements { indices, .. } => indices.is_some_and(|ix| ix.iter().all(|&i| u64::from(i) < n)),
            DrawCall::Arrays { first, count, .. } => u64::from(first) + u64::from(count) <= n,
        })
    }
}

/// A loaded S3D model.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub version: Version,
    pub vertex_blocks: Vec<VertexBlock>,
    pub index_blocks: Vec<IndexBlock>,
    pub primitive_blocks: Vec<Vec<Primitive>>,
    pub materials: Vec<Material>,
    pub anim: Anim,
    /// `PROP` entries in file order (empty before 1.1).
    pub props: Vec<Prop>,
    /// `REGP` entries by lower-cased name; a repeated name appends (empty before 1.2).
    pub regions: BTreeMap<String, Vec<[f32; 7]>>,
}

impl Model {
    /// The factory read (§3.1): only [`TYPE_S3D`].
    pub fn parse_record(type_id: u32, data: &[u8]) -> Result<Model> {
        if type_id != TYPE_S3D {
            return Err(Error::Unsupported {
                what: "S3D",
                detail: format!("type {type_id:#010X} is not the S3D type"),
            });
        }
        Model::parse(data)
    }

    /// Parses a model with the chunk grammar of its version (no type check; see [`Model::parse_record`]).
    pub fn parse(data: &[u8]) -> Result<Model> {
        let mut r = Reader::new(data, "S3D");
        chunk(&mut r, b"3DMD")?;
        chunk(&mut r, b"HEAD")?;
        let version = Version {
            major: r.u16()?,
            minor: r.u16()?,
        };
        if !version.is_accepted() {
            return Err(Error::Unsupported {
                what: "S3D",
                detail: format!("version {}.{}", version.major, version.minor),
            });
        }

        chunk(&mut r, b"VERT")?;
        let n = r.u32()?;
        r.ensure(u64::from(n), 8)?;
        let mut vertex_blocks = Vec::with_capacity(n as usize);
        for i in 0..n {
            vertex_blocks.push(vertex_block(&mut r, version).map_err(|e| in_block(e, "VERT", i))?);
        }

        chunk(&mut r, b"INDX")?;
        let n = r.u32()?;
        r.ensure(u64::from(n), 6)?;
        let mut index_blocks = Vec::with_capacity(n as usize);
        for i in 0..n {
            let flags = r.u16()?;
            let size = r.u16()?;
            let count = r.u16()?;
            // [choice] Q12: the original reads no index bytes and misreads the rest of the model.
            if size != 2 {
                return Err(corrupt(
                    "S3D",
                    format!("INDX block {i}: index size {size} (only 2 is read)"),
                ));
            }
            let bytes = r.bytes(2 * usize::from(count))?;
            let indices = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            index_blocks.push(IndexBlock { flags, indices });
        }

        chunk(&mut r, b"PRIM")?;
        let n = r.u32()?;
        r.ensure(u64::from(n), 2)?;
        let mut primitive_blocks = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let p = r.u16()?;
            r.ensure(u64::from(p), 12)?;
            let mut prims = Vec::with_capacity(usize::from(p));
            for _ in 0..p {
                prims.push(Primitive {
                    kind: r.u32()?,
                    first: r.u32()?,
                    count: r.u32()?,
                });
            }
            primitive_blocks.push(prims);
        }

        chunk(&mut r, b"MATS")?;
        let n = r.u32()?;
        r.ensure(u64::from(n), 16)?;
        let mut materials = Vec::with_capacity(n as usize);
        for i in 0..n {
            materials.push(material(&mut r, version).map_err(|e| in_block(e, "MATS material", i))?);
        }

        chunk(&mut r, b"ANIM")?;
        let counts = [
            vertex_blocks.len(),
            index_blocks.len(),
            primitive_blocks.len(),
            materials.len(),
        ];
        let anim = anim(&mut r, counts)?;

        let mut props = Vec::new();
        if version.has_prop() {
            chunk(&mut r, b"PROP")?;
            let n = r.u32()?;
            r.ensure(u64::from(n), 6)?;
            for _ in 0..n {
                let mesh = r.i16()?;
                let frame = r.i16()?;
                let key = c_string(short_string(&mut r)?, false);
                let value = c_string(short_string(&mut r)?, false);
                props.push(Prop {
                    mesh,
                    frame,
                    key,
                    value,
                });
            }
        }

        let mut regions: BTreeMap<String, Vec<[f32; 7]>> = BTreeMap::new();
        if version.has_regp() {
            chunk(&mut r, b"REGP")?;
            let n = r.u32()?;
            r.ensure(u64::from(n), 3)?;
            for _ in 0..n {
                let name = c_string(short_string(&mut r)?, true);
                let c = r.u16()?;
                r.ensure(u64::from(c), 28)?;
                let list = regions.entry(name).or_default();
                for _ in 0..c {
                    let mut t = [0f32; 7];
                    for v in &mut t {
                        *v = r.f32()?;
                    }
                    list.push(t);
                }
            }
        }
        // Anything after the last expected chunk is ignored (§3.2).
        Ok(Model {
            version,
            vertex_blocks,
            index_blocks,
            primitive_blocks,
            materials,
            anim,
            props,
            regions,
        })
    }

    /// The meshes of frame `f` (§3.12): one per animated mesh, with its vertex, index and primitive blocks and its
    /// material. `None` for `f` ≥ F (`GetModelFrame` fails).
    pub fn frame(&self, f: usize) -> Option<Vec<Mesh<'_>>> {
        if f >= usize::from(self.anim.frame_count) {
            return None;
        }
        Some(
            self.anim
                .meshes
                .iter()
                .filter_map(|m| {
                    let b = *m.frames.get(f)?;
                    Some(Mesh {
                        name: &m.name,
                        flags: m.flags,
                        vertices: self.vertex_blocks.get(usize::from(b[0]))?,
                        indices: self.index_blocks.get(usize::from(b[1]))?,
                        primitives: self.primitive_blocks.get(usize::from(b[2]))?,
                        material: self.materials.get(usize::from(b[3]))?,
                        blocks: b,
                    })
                })
                .collect(),
        )
    }
}

/// Reads a chunk tag (compared exactly) and its length (ignored).
fn chunk(r: &mut Reader<'_>, tag: &[u8; 4]) -> Result<()> {
    let found = r.bytes(4)?;
    if found != tag {
        return Err(Error::BadMagic {
            what: match tag {
                b"3DMD" => "S3D 3DMD tag",
                b"HEAD" => "S3D HEAD tag",
                b"VERT" => "S3D VERT tag",
                b"INDX" => "S3D INDX tag",
                b"PRIM" => "S3D PRIM tag",
                b"MATS" => "S3D MATS tag",
                b"ANIM" => "S3D ANIM tag",
                b"PROP" => "S3D PROP tag",
                _ => "S3D REGP tag",
            },
            found: found.to_vec(),
        });
    }
    r.u32()?; // the length: read and ignored (§3.2)
    Ok(())
}

/// Adds the block to a corruption message.
fn in_block(e: Error, what: &str, i: u32) -> Error {
    match e {
        Error::Corrupt { what: w, detail } => corrupt(w, format!("{what} {i}: {detail}")),
        other => other,
    }
}

fn vertex_block(r: &mut Reader<'_>, version: Version) -> Result<VertexBlock> {
    let flags = r.u16()?;
    let count = r.u16()?;
    let (format, stored_stride) = if version.legacy_vert() {
        let f = u32::from(r.u16()?);
        (f, Some(u32::from(r.u16()?)))
    } else {
        (r.u32()?, None)
    };
    let standard = VertexFormat::from_stored(format).stride();
    // [choice] Q11: an unknown format has stride 0 and the original's bounding box reads an empty buffer.
    if standard == 0 {
        return Err(corrupt("S3D", format!("unknown vertex format {format:#X}")));
    }
    // [choice] the bounding box reads 3 floats of every vertex.
    if standard < 12 {
        return Err(corrupt(
            "S3D",
            format!("vertex format {format:#X} has stride {standard} (< 12)"),
        ));
    }
    let stride = stored_stride.unwrap_or(standard);
    // [choice] a legacy stride below the standard one would make the draw read past the block.
    if stride < standard {
        return Err(corrupt(
            "S3D",
            format!("legacy stride {stride} is below the standard stride {standard} of format {format:#X}"),
        ));
    }
    r.ensure(u64::from(count), u64::from(stride))?;
    let data = r.bytes(stride as usize * usize::from(count))?.to_vec();
    let bbox = bounding_box(&data, stride as usize, usize::from(count));
    Ok(VertexBlock {
        flags,
        format,
        stride,
        count,
        data,
        bbox,
    })
}

/// The box of the first three floats of each vertex: seeded from vertex 0, then each coordinate below the minimum
/// or above the maximum replaces it. N = 0 gives [`BoundingBox::EMPTY`] *\[choice\]*.
fn bounding_box(data: &[u8], stride: usize, count: usize) -> BoundingBox {
    let pos = |i: usize| -> [f32; 3] {
        let o = i * stride;
        let f = |k: usize| f32::from_le_bytes([data[o + k], data[o + k + 1], data[o + k + 2], data[o + k + 3]]);
        [f(0), f(4), f(8)]
    };
    if count == 0 {
        return BoundingBox::EMPTY;
    }
    let p0 = pos(0);
    let mut b = BoundingBox { min: p0, max: p0 };
    for i in 1..count {
        for (a, v) in pos(i).into_iter().enumerate() {
            if v < b.min[a] {
                b.min[a] = v;
            }
            if v > b.max[a] {
                b.max[a] = v;
            }
        }
    }
    b
}

fn material(r: &mut Reader<'_>, version: Version) -> Result<Material> {
    let stored_flags = r.u32()?;
    let alpha_func = r.u8()?;
    let depth_func = r.u8()?;
    let src_blend = r.u8()?;
    let dst_blend = r.u8()?;
    let alpha_threshold = r.u16()?;
    let class = r.u32()?;
    r.u8()?; // reserved: read, then overwritten by the count
    let t = r.u8()?;
    // [choice] Q13: the original has two texture slots and overflows them.
    if t > 2 {
        return Err(corrupt("S3D", format!("{t} textures (at most 2)")));
    }
    let mut textures = Vec::with_capacity(usize::from(t));
    for _ in 0..t {
        let id = r.u32()?;
        let wrap_s = r.u8()?;
        let wrap_t = r.u8()?;
        let (mag_filter, min_filter) = if version.has_texture_filters() {
            (r.u8()?, r.u8()?)
        } else {
            (0, 0)
        };
        let anim_rate = r.u16()?;
        let anim_mode = r.u16()?;
        let name = c_string(short_string(r)?, false);
        textures.push(MaterialTexture {
            id,
            wrap_s,
            wrap_t,
            mag_filter,
            min_filter,
            anim_rate,
            anim_mode,
            name,
        });
    }
    let texture_count = match textures.last() {
        Some(last) if last.id == 0 => 0,
        _ => textures.len(),
    };
    Ok(Material {
        stored_flags,
        flags: (stored_flags & 0xFFFF) as u16 | material_flags::ALWAYS,
        alpha_func,
        depth_func,
        src_blend,
        dst_blend,
        alpha_threshold,
        class,
        textures,
        texture_count,
    })
}

/// `ANIM` with the §3.9 validation; `counts` are the VERT, INDX, PRIM and MATS counts.
fn anim(r: &mut Reader<'_>, counts: [usize; 4]) -> Result<Anim> {
    let frame_count = r.u16()?;
    let frame_rate = r.u16()?;
    let mode = r.u16()? as u8;
    let flags = r.u32()? as u8;
    let displacement = r.f32()?;
    let n = r.u16()?;
    r.ensure(u64::from(n), 2 + 8 * u64::from(frame_count))?;
    let mut meshes = Vec::with_capacity(usize::from(n));
    let mut dropped = 0;
    for _ in 0..n {
        let len = r.u8()?;
        let mesh_flags = r.u8()?;
        let name = c_string(r.bytes(usize::from(len))?, false);
        let mut frames = Vec::with_capacity(usize::from(frame_count));
        let mut valid = true;
        for _ in 0..frame_count {
            let mut b = [0u16; 4];
            for (k, slot) in b.iter_mut().enumerate() {
                let v = r.i16()?;
                if v < 0 || v as usize >= counts[k] {
                    valid = false;
                }
                *slot = v as u16;
            }
            frames.push(b);
        }
        if valid {
            meshes.push(AnimMesh {
                name,
                flags: mesh_flags,
                frames,
            });
        } else {
            dropped += 1;
        }
    }
    Ok(Anim {
        frame_count,
        frame_rate,
        mode,
        flags,
        displacement,
        meshes,
        dropped,
    })
}

/// A `u8` length and that many bytes.
fn short_string<'a>(r: &mut Reader<'a>) -> Result<&'a [u8]> {
    let len = r.u8()?;
    r.bytes(usize::from(len))
}

/// A C string: the bytes up to the first NUL, or all of them when there is none *\[choice\]*; UTF-8 when valid,
/// else Latin-1. `lower` lower-cases ASCII letters (REGP names).
fn c_string(b: &[u8], lower: bool) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    let mut bytes = b[..end].to_vec();
    if lower {
        bytes.make_ascii_lowercase();
    }
    match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => e.into_bytes().iter().map(|&c| c as char).collect(),
    }
}

#[cfg(test)]
mod tests;
