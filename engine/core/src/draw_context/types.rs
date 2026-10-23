// SPDX-License-Identifier: GPL-3.0-or-later
//! Data crossing the external driver/camera/lighting/resource boundary.

use std::sync::Arc;

use super::Error;

/// Column-major driver matrix. Camera providers return row-major projections.
pub type Matrix = [f32; 16];
pub type Rgb = [f32; 3];
pub type Rgba = [f32; 4];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub rotation: [[f32; 3]; 3],
    pub translation: [f32; 3],
    pub scale: f32,
    /// camera.md 1.1: translated=1, scaled=2, rotated=4.
    pub flags: u8,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        translation: [0.0; 3],
        scale: 1.0,
        flags: 0,
    };
    pub const ZERO: Self = Self {
        rotation: [[0.0; 3]; 3],
        translation: [0.0; 3],
        scale: 0.0,
        flags: 0,
    };

    fn active_scale(self) -> f32 {
        if self.flags & 2 == 0 {
            1.0
        } else {
            self.scale
        }
    }

    /// Qualified DC8 composition, with whole-byte branches and distinct stores.
    pub fn followed_by(self, model: Self) -> Result<Self, Error> {
        self.followed_by_with_control(model, super::Arithmetic::default())
    }

    pub fn followed_by_with_control(self, model: Self, arithmetic: super::Arithmetic) -> Result<Self, Error> {
        if model.flags == 0 {
            return Ok(self);
        }
        if self.flags == 0 {
            return Ok(model);
        }
        let a = self.rotation;
        let b = model.rotation;
        let mut rotation = [[0.0; 3]; 3];
        let mut translation = self.translation;
        let mt = model.translation;
        let s = f64::from(self.scale);
        // Literal grouping table from section 8.3, independently exercised by
        // fixtures; never replace it with a generic matrix multiplication.
        let orders = [
            [[0, 2, 1], [2, 0, 1], [1, 0, 2]],
            [[0, 2, 1], [1, 0, 2], [1, 2, 0]],
            [[2, 1, 0], [0, 2, 1], [2, 0, 1]],
        ];
        for row in 0..3 {
            let p = [
                arithmetic.mul(f64::from(a[row][0]), f64::from(mt[0]))?,
                arithmetic.mul(f64::from(a[row][1]), f64::from(mt[1]))?,
                arithmetic.mul(f64::from(a[row][2]), f64::from(mt[2]))?,
            ];
            let dot = arithmetic.add(arithmetic.add(p[1], p[2])?, p[0])?;
            let scaled = if row < 2 {
                super::numeric::Float::from(f64::from(arithmetic.store(arithmetic.mul(dot, s)?)?))
            } else {
                arithmetic.mul(f64::from(arithmetic.store(dot)?), s)?
            };
            translation[row] = arithmetic.store(arithmetic.add(scaled, f64::from(self.translation[row]))?)?;
            for col in 0..3 {
                let p = [
                    arithmetic.mul(f64::from(a[row][0]), f64::from(b[0][col]))?,
                    arithmetic.mul(f64::from(a[row][1]), f64::from(b[1][col]))?,
                    arithmetic.mul(f64::from(a[row][2]), f64::from(b[2][col]))?,
                ];
                let [x, y, z] = orders[row][col];
                rotation[row][col] = arithmetic.store(arithmetic.add(arithmetic.add(p[x], p[y])?, p[z])?)?;
            }
        }
        Ok(Self {
            rotation,
            translation,
            scale: arithmetic.store(arithmetic.mul(s, f64::from(model.scale))?)?,
            flags: self.flags | model.flags,
        })
    }

    pub fn matrix(self) -> Matrix {
        let r = self.rotation;
        let t = self.translation;
        let s = self.active_scale();
        let mut m = [0.0; 16];
        for row in 0..3 {
            for col in 0..3 {
                m[col * 4 + row] = r[row][col] * s;
            }
            m[12 + row] = t[row];
        }
        m[15] = 1.0;
        m
    }

    pub(super) fn qualified_matrix(self, arithmetic: super::Arithmetic) -> Result<Matrix, Error> {
        let mut matrix = [0.0; 16];
        for row in 0..3 {
            for col in 0..3 {
                matrix[col * 4 + row] = if self.flags & 2 != 0 {
                    arithmetic.store(arithmetic.mul(f64::from(self.rotation[row][col]), f64::from(self.scale))?)?
                } else {
                    self.rotation[row][col]
                };
            }
            matrix[12 + row] = self.translation[row];
        }
        matrix[15] = 1.0;
        Ok(matrix)
    }
}

/// Borrowed opaque texture identity. Arc only manages the portable token's
/// storage; cloning/dropping it does not model native AddRef/Release.
#[derive(Debug, PartialEq, Eq)]
pub struct Texture {
    pub token: u64,
}
pub type TextureRef = Arc<Texture>;

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    ColorMask(bool),
    DepthMask(bool),
    Capability {
        capability: u8,
        enabled: bool,
    },
    DepthFunc(u8),
    AlphaFunc {
        function: u8,
        reference: f32,
    },
    BlendFunc {
        source: u8,
        destination: u8,
    },
    SelectStage(usize),
    Texture(Option<TextureRef>, usize),
    TexCoord(u32),
    TexParameter {
        parameter: u8,
        value: i32,
    },
    TexMatrix(Option<Matrix>),
    TexEnv(u8),
    Combiner(Vec<u8>, usize),
    TexColor(Rgba),
    MatrixMode(u8),
    LoadMatrix(Matrix),
    AlphaMultiplier(f32),
    ColorMultiplier(Rgb),
    FogInteger(u32, i32),
    /// Snapshot of slot 30 selector 1; pointer-aware adapters override fog_color.
    FogColor(Rgba),
    /// Snapshot of slot 30 selector 3 or 4; adapters override fog_scalar.
    FogScalar(u32, f32),
    InterleavedArrays {
        format: u32,
        stride: u32,
        data: usize,
    },
    /// Same external InterleavedArrays call with core-generated owned storage.
    InterleavedOwned {
        format: u32,
        stride: u32,
        vertices: Vec<LitVertex>,
    },
    DrawArrays {
        primitive: u32,
        first: u32,
        count: u32,
    },
    DrawElements {
        primitive: u32,
        count: u32,
        index_type: u8,
        indices: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DriverInfo {
    pub stage_count: usize,
    /// DC1: slot-64 record bytes +0x29 and +0x2A, without an interpretation.
    pub bytes: [u8; 2],
}

pub trait Driver {
    fn info(&mut self) -> Result<DriverInfo, Error>;
    fn submit(&mut self, command: Command) -> Result<(), Error>;

    /// A normal foreign return, including a failure-valued register, is distinct
    /// from an adapter/provider boundary error. The owner does not test this
    /// register. Err propagates and supplies no exception-cleanup guarantee.
    fn submit_returned(&mut self, command: Command) -> Result<NormalReturn, Error> {
        self.submit(command).map(|()| NormalReturn { register: 0 })
    }

    /// Slot 30 selector 1 receives the original caller's RGBA, including alpha.
    /// The default command adapter records a value snapshot instead.
    fn fog_color(&mut self, color: &Rgba) -> Result<NormalReturn, Error> {
        self.submit_returned(Command::FogColor(*color))
    }

    /// Slot 30 receives a reference to the respective binary32 argument, not
    /// an RGBA vector. Selectors 3 and 4 mean start and end respectively.
    fn fog_scalar(&mut self, selector: u32, value: &f32) -> Result<NormalReturn, Error> {
        self.submit_returned(Command::FogScalar(selector, *value))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NormalReturn {
    pub register: u32,
}

pub trait Camera: Send + Sync {
    fn view_transform(&self) -> Result<Transform, Error>;
    fn projection(&self) -> Result<[[f32; 4]; 4], Error>;
    fn set_depth_bias(&self, bias: i32) -> Result<(), Error>;
}

pub trait Lighting: Send + Sync {
    fn model_light(&self, instance: u64) -> Result<Rgb, Error>;

    /// None is the explicitly checked false AL result, not a provider error.
    fn checked_model_light(&self, instance: u64) -> Result<Option<Rgb>, Error> {
        self.model_light(instance).map(Some)
    }

    fn default_light(&self) -> Result<Rgb, Error> {
        Err(Error::MissingLighting)
    }
}

pub trait View: Send + Sync {
    fn pixels_per_metre(&self) -> Result<f32, Error>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Properties {
    pub force_no_alpha_test: bool,
    pub force_no_blend: bool,
    pub use_second_stage: bool,
    pub force_point: bool,
    pub force_bilinear: bool,
    pub force_repeat: bool,
    pub show_boxes: bool,
    pub bit30: bool,
    pub model_threshold: f32,
    pub depth_offset: i32,
}

// Only the documented depth-offset default is supplied. Other render properties
// belong to the external render-properties owner; require an explicit snapshot.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawFlags {
    /// DC2: neutral labels; the owner implements the measured bit effects.
    pub bits: u8,
    pub highlight_color: u8,
    pub color: Rgb,
    pub a: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextureSlot {
    pub texture: Option<TextureRef>,
    pub coordinate_set: u32,
    pub wrap: [u8; 2],
    pub filter: [u8; 2],
    pub mode: u8,
    pub flags: u8,
    pub scale: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub flags: u16,
    pub depth_func: u8,
    pub alpha_func: u8,
    pub alpha_threshold: u8,
    pub blend: [u8; 2],
    pub textures: Vec<TextureSlot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    pub primitive: u32,
    pub first: u32,
    pub count: u32,
}

/// Borrowed data addresses are forwarded synchronously, never dereferenced by
/// the core. Their storage must live through the call to the external driver.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub format: u32,
    pub vertices: Option<usize>,
    pub indices: Option<usize>,
    pub groups: Vec<Group>,
    pub material: Material,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LitVertex {
    pub position: [f32; 3],
    pub bgra: [u8; 4],
}

impl Bounds {
    pub fn is_empty(self) -> bool {
        (0..3).any(|axis| self.max[axis] <= self.min[axis])
    }
    pub fn largest_extent(self) -> f32 {
        (0..3).map(|axis| self.max[axis] - self.min[axis]).fold(0.0, f32::max)
    }
}

/// External instance/occupant queries, not draw decisions or generated commands.
pub trait Instance {
    fn token(&self) -> u64;
    fn bounds(&self) -> Result<Bounds, Error>;
    fn effects_collection_bounds(&self) -> Result<Option<Bounds>, Error>;
    fn transform(&self) -> Result<Transform, Error>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    /// Borrowed identity; constructor sentinel 0xFFFFFFFF is distinct from null.
    pub texture: Option<TextureRef>,
    pub texture_known: bool,
    pub coordinate_set: Option<u32>,
    pub wrap: [Option<i32>; 2],
    pub filter: [Option<i32>; 2],
    pub combiner: [u8; 12],
    pub saved_matrix: Matrix,
    pub transform: Option<Matrix>,
    pub transform_known: bool,
}

impl Stage {
    pub(super) fn default_record() -> Self {
        // DC5 section 1.4. The three unspecified padding bytes have no portable
        // representation. Texture is a borrowed opaque sentinel, not null.
        Self {
            texture: Some(Arc::new(Texture { token: 0xffff_ffff })),
            texture_known: true,
            coordinate_set: Some(0),
            wrap: [Some(-1); 2],
            filter: [Some(-1); 2],
            combiner: [1, 0, 3, 1, 1, 0, 3, 1, 0, 0, 1, 0],
            saved_matrix: [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.],
            transform: None,
            transform_known: true,
        }
    }
}

/// Complete public DC4 tables; Option is retained for portable API compatibility.
pub const CONSTRUCTOR_COMBINER_BYTES: [Option<u8>; 12] = [
    Some(1),
    Some(0),
    Some(3),
    Some(1),
    Some(1),
    Some(0),
    Some(3),
    Some(1),
    Some(0),
    Some(0),
    Some(0),
    Some(0),
];
pub const DEFAULT_COMBINER_BYTES: [Option<u8>; 12] = [
    Some(2),
    Some(0),
    Some(1),
    Some(1),
    Some(0),
    Some(1),
    Some(3),
    Some(1),
    Some(0),
    Some(0),
    Some(0),
    Some(0),
];
pub const HIGHLIGHT_COMBINER_BYTES: [Option<u8>; 12] = [
    Some(2),
    Some(2),
    Some(0),
    Some(1),
    Some(1),
    Some(2),
    Some(0),
    Some(1),
    Some(0),
    Some(0),
    Some(0),
    Some(0),
];
