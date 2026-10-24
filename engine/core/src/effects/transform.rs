// SPDX-License-Identifier: GPL-3.0-or-later
//! Semantic transform inputs for component positions (effects §§8.3, 9.2).
//! This does not decode the opaque 56-byte legacy placement record (EF19).

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
    /// Literal semantic implementation: binary32 products and left-to-right sums;
    /// native intermediate-store/order qualification remains open in EF19.
    pub fn world_position(&self, local_offset: [f32; 3]) -> [f32; 3] {
        if *self == Self::default() {
            return local_offset;
        }
        if local_offset == [0.; 3] {
            return self.translation;
        }
        let mut position = self.translation;
        for (axis, row) in self.rotation.iter().enumerate() {
            let rotated = (row[0] * local_offset[0] + row[1] * local_offset[1]) + row[2] * local_offset[2];
            position[axis] += self.scale * rotated;
        }
        position
    }
}
