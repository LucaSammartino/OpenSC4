// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable lot-model decisions, from `docs/specs/lot-models.md` §§1–4, 8.
//!
//! These owners retain their textures and geometry; providers supply only texture
//! resources and lighting/height samples, never an original owner's result.
//! Native buffer layouts, inherited basic-model behaviour and pool/GZCOM lifetime
//! are deliberately not emulated. See `README.md` for the public inventory and
//! the exact limits (LM2–LM8). All getters borrow immutable state.

mod cell;
mod foundation;
mod wall;

pub use cell::*;
pub use foundation::*;
pub use wall::*;
// Reuse the existing portable coordinate primitive, without a camera dependency
// in the model algorithms themselves.
pub use crate::camera::{Vec2, Vec3};

pub const CELL_METRES: f32 = 16.0;
pub const CELL_TEXTURE_IID: u32 = 0xA9E2_831D;
pub const RETAINING_WALL_IID: u32 = 0xA9E2_831E;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractError {
    /// Outside this portable API's domain; original behaviour is not published.
    InvalidInput,
    /// LM5: key + zoom overflow is not defined by the public spec.
    TextureKeyOverflow,
    /// LM4: the published wall description does not determine a triangle buffer.
    WallTriangleLayout,
}

/// The five published texture levels. No native out-of-range behaviour is implied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zoom(u8);

impl Zoom {
    pub fn new(value: u8) -> Result<Self, ContractError> {
        if value < 5 { Ok(Self(value)) } else { Err(ContractError::InvalidInput) }
    }

    pub fn index(self) -> usize { self.0 as usize }
    pub fn value(self) -> u32 { u32::from(self.0) }
}

/// Outward edge normals from view-utilities §4.2 (not wall range traversal).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge { North, East, South, West }

impl Edge {
    pub const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];
    pub fn index(self) -> usize { self as usize }
    pub fn normal(self) -> Vec3 {
        match self {
            Self::North => [0.0, 0.0, -1.0],
            Self::East => [1.0, 0.0, 0.0],
            Self::South => [0.0, 0.0, 1.0],
            Self::West => [-1.0, 0.0, 0.0],
        }
    }
    pub fn visible_foundation_edges(self) -> [Self; 2] {
        [Self::ALL[(self.index() + 1) % 4], Self::ALL[(self.index() + 2) % 4]]
    }
    pub fn wall_faces(self, rotation: Self) -> bool {
        self == rotation || self.index() == (rotation.index() + 3) % 4
    }
}

/// Factory queries retain only the namespace actually published for each owner.
/// Foundation type/group and wall type are intentionally absent (LM5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureKey {
    FoundationSide(u32),
    FoundationTop(u32),
    Wall { group: u32, instance: u32 },
    Cell { type_id: u32, group: u32, instance: u32 },
}

pub trait Texture {
    fn id(&self) -> u32;
    fn has_alpha(&self) -> bool;
}

/// Each successful load returns one owned lease. An adapter must acquire that
/// lease itself; moving/dropping it must not release an unacquired game pointer.
/// Rust Drop owns portable lifetime; no native AddRef/Release count is invented.
pub trait TextureFactory {
    type Binding: Texture;
    fn load(&mut self, key: TextureKey) -> Option<Self::Binding>;
}

pub trait Lighting {
    /// Foundation normals; §2 does not ask for position-dependent light here.
    fn for_normal(&mut self, normal: Vec3) -> Vec3;
    /// Cell lighting §4, slot 19, with the explicit up normal.
    fn at_corner(&mut self, position: Vec3, normal: Vec3) -> Vec3;
}

/// One primary attempt followed by fallback on absence/failure. LM5 records
/// the ambiguity between a missing configured key and an unresolved resource;
/// this is the literal "loaded again" reading, not a native failure contract.
pub(super) fn load_with_fallback<F: TextureFactory>(
    factory: &mut F, primary: Option<TextureKey>, fallback: TextureKey,
) -> Option<F::Binding> {
    primary.and_then(|key| factory.load(key)).or_else(|| factory.load(fallback))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepthTest { LessOrEqual, Less }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blend { Disabled, SourceAlphaOneMinusSourceAlpha }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaTest { Disabled, GreaterThanZero }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawState {
    pub colour_write: bool,
    pub depth_write: bool,
    pub depth_test: DepthTest,
    pub texture_count: u8,
    pub alpha_test: AlphaTest,
    pub blend: Blend,
    pub cull: bool,
}

impl DrawState {
    pub const OPAQUE: Self = Self {
        colour_write: true, depth_write: true, depth_test: DepthTest::LessOrEqual,
        texture_count: 1, alpha_test: AlphaTest::Disabled, blend: Blend::Disabled, cull: false,
    };
    pub const ALPHA: Self = Self {
        alpha_test: AlphaTest::GreaterThanZero,
        blend: Blend::SourceAlphaOneMinusSourceAlpha, ..Self::OPAQUE
    };
    pub const TINTED: Self = Self {
        depth_write: false, depth_test: DepthTest::Less,
        blend: Blend::SourceAlphaOneMinusSourceAlpha, ..Self::OPAQUE
    };
}

/// This is a semantic box, not an ordered native vertex buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds { pub min: Vec3, pub max: Vec3 }

#[cfg(test)]
mod tests;
