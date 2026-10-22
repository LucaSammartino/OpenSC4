// SPDX-License-Identifier: GPL-3.0-or-later
//! Value types for lighting.md. Coordinates are metres, y is up.

pub type Color = [f32; 3];
pub type Vector = [f32; 3];
pub const WHITE: Color = [1.0; 3];
pub const UP: Vector = [0.0, 1.0, 0.0];

pub(crate) fn add(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] + b[i])
}
pub(crate) fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(crate) fn scale(a: Vector, s: f32) -> Vector {
    a.map(|v| v * s)
}
pub(crate) fn product(a: Color, b: Color) -> Color {
    std::array::from_fn(|i| a[i] * b[i])
}
pub(crate) fn lerp(a: Vector, b: Vector, t: f32) -> Vector {
    add(scale(a, 1.0 - t), scale(b, t))
}
pub(crate) fn dot(a: Vector, b: Vector) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(crate) fn normalize(a: Vector) -> Vector {
    let length = dot(a, a).sqrt();
    if length == 0.0 {
        a
    } else {
        scale(a, 1.0 / length)
    }
}
pub(crate) fn rotate(a: Vector, rotation: u8) -> Vector {
    match rotation % 4 {
        0 => a,
        1 => [-a[2], a[1], a[0]],
        2 => [-a[0], a[1], -a[2]],
        _ => [a[2], a[1], -a[0]],
    }
}

/// §4.7: adding 49152 rounds to 1/256 with IEEE ties-to-even. No clamp.
/// Conversion to u8 retains the low byte, including for out-of-range channels.
pub fn color_byte(channel: f32) -> u8 {
    let bits = (channel + 49152.0).to_bits();
    let n = bits.wrapping_sub(49152.0_f32.to_bits()) as i32;
    n.wrapping_sub(n >> 8) as u8
}
pub fn color_bytes(color: Color) -> [u8; 3] {
    color.map(color_byte)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightingError {
    InvalidDimensions,
    InitializationInterrupted,
    BufferTooShort,
    SampleOutsideImage,
    MissingModelMap,
    InvalidTime,
    InvalidMonth,
    NoTerrain,
    LocationOutsideGrid,
    InvalidStride,
    DegenerateProjection,
    /// Missing public contract; no substitute original method is called.
    ContractGap(&'static str),
    Released,
    ReferenceOverflow,
}
pub type Result<T> = std::result::Result<T, LightingError>;

/// Inclusive vertex coordinates, also used for stored-grid changed rectangles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub min_x: i32,
    pub min_z: i32,
    pub max_x: i32,
    pub max_z: i32,
}
impl Rect {
    pub const EMPTY: Self = Self {
        min_x: -1,
        min_z: -1,
        max_x: -1,
        max_z: -1,
    };
    pub fn is_empty(self) -> bool {
        self.min_x < 0 || self.min_z < 0 || self.max_x < self.min_x || self.max_z < self.min_z
    }
    pub fn include(&mut self, x: i32, z: i32) {
        if self.is_empty() {
            *self = Self {
                min_x: x,
                min_z: z,
                max_x: x,
                max_z: z,
            };
        } else {
            self.min_x = self.min_x.min(x);
            self.min_z = self.min_z.min(z);
            self.max_x = self.max_x.max(x);
            self.max_z = self.max_z.max(z);
        }
    }
    pub fn union(&mut self, other: Self) {
        if !other.is_empty() {
            self.include(other.min_x, other.min_z);
            self.include(other.max_x, other.max_z);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: Vector,
    pub max: Vector,
}
impl Bounds {
    pub fn center(self) -> Vector {
        scale(add(self.min, self.max), 0.5)
    }
}

/// Primitive interface answers, rather than a precomputed lighting result (§4.3).
#[derive(Clone, Copy, Debug, Default)]
pub struct Occupant {
    pub network_flag_40000: bool,
    pub flora: bool,
    pub group_5004: bool,
    pub vehicle: bool,
    pub pedestrian: bool,
    pub aircraft: bool,
    pub watercraft: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Interface {
    Lighting,
    MessageTarget,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Subscribe(u32),
    Unsubscribe(u32),
    Post(u32),
    Fractions {
        day: f32,
        night: f32,
    },
    Redraw,
    /// §6, terrain slot 67. See LT9 for vertex-to-cell boundary semantics.
    TerrainLightingChanged(Rect),
    RedisplayTerrain(Rect),
}
pub trait EventSink {
    fn emit(&mut self, event: Event);
}
impl EventSink for Vec<Event> {
    fn emit(&mut self, event: Event) {
        self.push(event);
    }
}

/// Only external terrain data/queries. No same-owner lighting operations.
pub trait Terrain: Send + Sync {
    fn cell_counts(&self) -> (usize, usize);
    fn initialized(&self) -> bool;
    fn altitude(&self, x: usize, z: usize) -> f32;
    fn normal(&self, x: f32, z: f32) -> Vector;
}
