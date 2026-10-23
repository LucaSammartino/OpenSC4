// SPDX-License-Identifier: GPL-3.0-or-later
//! Literal axis/ANGLE reading and row-matrix composition, graphics-formats §3.11.

pub type Vector3 = [f32; 3];
pub type Rotation = [[f32; 3]; 3];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub rotation: Rotation,
    pub translation: Vector3,
    pub scale: f32,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        translation: [0.0; 3],
        scale: 1.0,
    };

    pub fn from_record(record: [f32; 7]) -> Self {
        let [px, py, pz, ax, ay, az, angle] = record;
        // GF13's deliberate choice: a zero axis means no rotation even when
        // the angle is nonzero. This is NOT the data's quaternion interpretation.
        let rotation = if ax == 0.0 && ay == 0.0 && az == 0.0 {
            Self::IDENTITY.rotation
        } else {
            // Binary32 loads, wider evaluation in the specified (z²+y²)+x²
            // order, followed by separate binary32 normalized-component stores.
            // Wider intermediates avoid binary32 overflow/underflow in x87's
            // valid domain; these portable transcendentals are tolerance tested.
            let [ax, ay, az] = [ax, ay, az].map(f64::from);
            let inverse_length = 1.0 / ((az * az + ay * ay) + ax * ax).sqrt();
            let [nx, ny, nz] = [ax, ay, az].map(|a| f64::from((a * inverse_length) as f32));
            let half_angle = f64::from(angle) * -0.5;
            let sine = half_angle.sin();
            let x = nx * sine;
            let y = ny * sine;
            let z = nz * sine;
            let w = half_angle.cos();
            [
                [
                    1.0 - 2.0 * y * y - 2.0 * z * z,
                    2.0 * x * y + 2.0 * z * w,
                    2.0 * x * z - 2.0 * y * w,
                ],
                [
                    2.0 * x * y - 2.0 * z * w,
                    1.0 - 2.0 * x * x - 2.0 * z * z,
                    2.0 * y * z + 2.0 * x * w,
                ],
                [
                    2.0 * x * z + 2.0 * y * w,
                    2.0 * y * z - 2.0 * x * w,
                    1.0 - 2.0 * x * x - 2.0 * y * y,
                ],
            ]
            .map(|row| row.map(|value| value as f32))
        };
        Self {
            rotation,
            translation: [px, py, pz],
            scale: 1.0,
        }
    }

    /// `self` is the outer placement/instance transform, `incoming` the inner
    /// local transform. Translation never depends on incoming rotation (§3.11).
    pub fn compose(self, incoming: Self) -> Self {
        let mut rotation = [[0.0; 3]; 3];
        for (i, row) in rotation.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = self.rotation[i][0] * incoming.rotation[0][j]
                    + self.rotation[i][1] * incoming.rotation[1][j]
                    + self.rotation[i][2] * incoming.rotation[2][j];
            }
        }
        Self {
            rotation,
            translation: self.transform_point(incoming.translation),
            scale: self.scale * incoming.scale,
        }
    }

    pub fn transform_point(self, point: Vector3) -> Vector3 {
        let mut result = [0.0; 3];
        for (i, cell) in result.iter_mut().enumerate() {
            *cell = (self.rotation[i][0] * point[0] + self.rotation[i][1] * point[1] + self.rotation[i][2] * point[2])
                * self.scale
                + self.translation[i];
        }
        result
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}
