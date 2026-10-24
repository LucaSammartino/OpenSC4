// SPDX-License-Identifier: GPL-3.0-or-later
//! Semantic transform inputs for component positions (effects §§8.3, 9.2).
//! Legacy placement decoding and component arithmetic (effects §3, EF19).

/// Precision of each arithmetic operation; stores always narrow to binary32.
#[derive(Clone, Copy, Debug)]
pub enum Precision {
    Single,
    Double,
}
impl Precision {
    fn round(self, value: f64) -> f64 {
        match self {
            Self::Single => {
                // Round the significand without imposing binary32's exponent
                // range before an explicit store (EF19).
                if !value.is_finite() || value == 0. {
                    return value;
                }
                let bits = value.to_bits();
                let discarded = bits & ((1u64 << 29) - 1);
                let high = bits & !((1u64 << 29) - 1);
                let increment = discarded > (1 << 28) || (discarded == (1 << 28) && high & (1 << 29) != 0);
                f64::from_bits(high + if increment { 1 << 29 } else { 0 })
            }
            Self::Double => value,
        }
    }
}

/// Decoded native transform. Flags are independent of the numeric values.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Placement {
    pub revision: u8,
    pub flags: u8,
    pub transform: ParentTransform,
}
impl Placement {
    /// Load into an existing destination, preserving unrelated flag bits.
    pub fn load(&mut self, bytes: &[u8; 56]) {
        let words: [u32; 14] = std::array::from_fn(|i| u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap()));
        self.transform.rotation = std::array::from_fn(|i| std::array::from_fn(|j| f32::from_bits(words[i * 3 + j])));
        self.transform.translation = std::array::from_fn(|i| f32::from_bits(words[9 + i]));
        self.transform.scale = f32::from_bits(words[12]);
        self.flags = (self.flags | 5) & !2;
        if self.transform.scale != 1. {
            self.flags |= 2;
        }
        self.revision = words[13] as u8;
    }

    pub fn decode(bytes: &[u8; 56]) -> Self {
        let mut value = Self::default();
        value.load(bytes);
        value
    }

    pub fn encode(&self) -> [u8; 56] {
        let mut bytes = [0; 56];
        let words = self
            .transform
            .rotation
            .into_iter()
            .flatten()
            .chain(self.transform.translation)
            .chain([self.transform.scale])
            .map(f32::to_bits)
            .chain([u32::from(self.revision)]);
        for (out, word) in bytes.chunks_exact_mut(4).zip(words) {
            out.copy_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    /// Brush intentionally evaluates only x and z. Game spills its z dot before scaling.
    pub fn world_position(&self, parent: &Self, game: bool, precision: Precision) -> [f32; 3] {
        if self.flags == 0 {
            return parent.transform.translation;
        }
        if parent.flags == 0 {
            return self.transform.translation;
        }
        let p = parent.transform;
        let t = self.transform.translation.map(f64::from);
        let r = |v| precision.round(v);
        let mut out = [0.; 3];
        for (i, row) in p.rotation.iter().enumerate() {
            if !game && i == 1 {
                continue;
            }
            let row = row.map(f64::from);
            let mut dot = r(r(r(row[2] * t[2]) + r(row[0] * t[0])) + r(row[1] * t[1]));
            if game && i == 2 {
                dot = f64::from(dot as f32);
            }
            let scaled = r(dot * f64::from(p.scale));
            let offset = if game && i == 2 {
                scaled
            } else {
                f64::from(scaled as f32)
            };
            out[i] = r(f64::from(p.translation[i]) + offset) as f32;
        }
        out
    }
}

/// Parent rotation/scale and translation, independent of native memory layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParentTransform {
    /// Rows of the rotation matrix.
    pub rotation: [[f32; 3]; 3],
    pub scale: f32,
    pub translation: [f32; 3],
}

impl Default for ParentTransform {
    fn default() -> Self {
        Self {
            rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            scale: 1.,
            translation: [0.; 3],
        }
    }
}

impl ParentTransform {
    /// Parent translation plus parent rotation/scale applied to the local offset.
    /// Convenience input: default parent means flag-zero identity; the local
    /// translation is explicitly set (nonzero flags), even when numerically zero.
    pub fn world_position(&self, local_offset: [f32; 3]) -> [f32; 3] {
        let local = Placement {
            flags: 1,
            transform: Self {
                translation: local_offset,
                ..Self::default()
            },
            ..Placement::default()
        };
        let parent = Placement {
            flags: u8::from(*self != Self::default()),
            transform: *self,
            ..Placement::default()
        };
        local.world_position(&parent, true, Precision::Single)
    }
}

#[cfg(test)]
mod tests;
