// SPDX-License-Identifier: GPL-3.0-or-later
//! Transform record and geometry helper, model-maker §§12.5/12.10.
pub type Vector = [f32; 3];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub counter: u8,
    pub flags: u8,
    pub rotation: [[f32; 3]; 3],
    pub translation: Vector,
}

impl Default for Transform {
    fn default() -> Self {
        // SPEC-GAP: a fresh record has a zero counter/flags and an identity transform.
        Self {
            counter: 0,
            flags: 0,
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: [0.0; 3],
        }
    }
}

impl Transform {
    fn changed_rotation(&mut self) {
        self.flags |= 4;
        self.counter = self.counter.wrapping_add(1);
    }

    /// §12.12: rotate columns by the binary32-rounded -π/2 constant.
    pub fn rotate_shadow(&mut self) {
        let angle = f64::from(-std::f32::consts::FRAC_PI_2);
        let (sin, cos) = angle.sin_cos();
        for row in &mut self.rotation {
            let [x, _, z] = *row;
            row[0] = (f64::from(x) * cos - f64::from(z) * sin) as f32;
            row[2] = (f64::from(x) * sin + f64::from(z) * cos) as f32;
        }
        self.changed_rotation();
    }

    pub fn point(&self, p: Vector) -> Vector {
        std::array::from_fn(|i| {
            self.rotation[i][0] * p[0] + self.rotation[i][1] * p[1] + self.rotation[i][2] * p[2] + self.translation[i]
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TransformInput {
    pub flag: bool,
    pub orientation: i32,
    pub position: Vector,
    pub offset: Option<Vector>,
    pub tilt_to_terrain: bool,
}

pub trait TransformHost {
    fn camera_yaw(&mut self) -> f32;
    /// Foreign query at p with mask 0x1F4F and flag 1. Returns the raw normal.
    fn object_normal(&mut self, p: Vector, mask: u32, flag: bool) -> Option<Vector>;
    fn terrain_blocks_normal(&mut self, x: f32, z: f32) -> bool;
    fn terrain_normal(&mut self, x: f32, z: f32) -> Vector;
}

pub fn object_y_rotation(rotation: i32) -> f64 {
    -f64::from(rotation) * std::f64::consts::FRAC_PI_2
}

/// Only the two components of the selected plane are written. The first
/// component rounds the product/sum before adding the centre; the second does not.
pub fn rotate_about_axis(centre: Vector, p: Vector, angle: f64, axis: u32, out: &mut Vector) {
    let (i, j) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let du = f64::from(p[i]) - f64::from(centre[i]);
    let dv = f64::from(p[j]) - f64::from(centre[j]);
    let (sin, cos) = angle.sin_cos();
    out[i] = (cos * du - sin * dv) as f32 + centre[i];
    out[j] = (sin * du + cos * dv + f64::from(centre[j])) as f32;
}

fn cross(a: Vector, b: Vector) -> Vector {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalise(v: Vector) -> Vector {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    // SPEC-GAP: a zero-length normal is left unchanged rather than dividing by zero.
    if length == 0.0 {
        v
    } else {
        v.map(|c| c / length)
    }
}

pub fn find_model_transform(input: TransformInput, record: &mut Transform, host: &mut impl TransformHost) {
    let theta = if input.flag {
        object_y_rotation(input.orientation)
    } else {
        -f64::from(host.camera_yaw())
    };
    let mut position = input.position;
    if let Some(offset) = input.offset {
        let mut rotated = offset;
        if !input.flag {
            rotate_about_axis([0.0; 3], offset, -object_y_rotation(input.orientation), 1, &mut rotated);
        }
        for i in 0..3 {
            position[i] += rotated[i];
        }
    }
    let (sin, cos) = theta.sin_cos();
    record.rotation = [
        [cos as f32, -0.0, sin as f32],
        [0.0, 1.0, -0.0],
        [-sin as f32, 0.0, cos as f32],
    ];
    record.changed_rotation();
    if input.tilt_to_terrain {
        let p = input.position;
        let normal = if let Some(n) = host.object_normal(p, 0x1F4F, true) {
            Some(normalise(n))
        } else if !host.terrain_blocks_normal(p[0], p[2]) {
            Some(host.terrain_normal(p[0], p[2]))
        } else {
            None
        };
        if let Some(mut n) = normal.filter(|n| n[1] != 1.0) {
            if n[1] < 0.866 && n[0] > 0.5 {
                let length = (n[1] * n[1] + n[2] * n[2] + 0.25).sqrt();
                n = [0.5 / length, n[1] / length, n[2] / length];
            }
            if n[1] < 0.866 && n[2] > 0.5 {
                let length = (n[1] * n[1] + n[0] * n[0] + 0.25).sqrt();
                n = [n[0] / length, n[1] / length, 0.5 / length];
            }
            let mut forward = [0.0, 0.0, 1.0];
            rotate_about_axis([0.0; 3], forward, -theta, 1, &mut forward);
            let right = normalise(cross(n, forward));
            let ahead = cross(right, n);
            record.rotation = std::array::from_fn(|i| [right[i], n[i], ahead[i]]);
            record.changed_rotation();
        }
    }
    record.translation = position;
    record.flags |= 1;
    record.counter = record.counter.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        object: Option<Vector>,
        terrain: Option<Vector>,
        terrain_reads: usize,
    }
    impl TransformHost for Host {
        fn camera_yaw(&mut self) -> f32 {
            0.25
        }
        fn object_normal(&mut self, _: Vector, mask: u32, flag: bool) -> Option<Vector> {
            assert_eq!(mask, 0x1F4F);
            assert!(flag);
            self.object
        }
        fn terrain_blocks_normal(&mut self, _: f32, _: f32) -> bool {
            self.terrain.is_none()
        }
        fn terrain_normal(&mut self, _: f32, _: f32) -> Vector {
            self.terrain_reads += 1;
            self.terrain.unwrap()
        }
    }
    fn input(flag: bool) -> TransformInput {
        TransformInput {
            flag,
            orientation: 1,
            position: [10.0, 20.0, 30.0],
            offset: Some([2.0, 3.0, 4.0]),
            tilt_to_terrain: false,
        }
    }
    #[test]
    fn rotate_axis_preserves_third_component_and_asymmetric_rounding() {
        let mut out = [8.0; 3];
        rotate_about_axis([16_777_216.0, 16_777_216.0, 0.0], [-1.0, -1.0, 0.0], 0.0, 2, &mut out);
        assert_eq!(out, [0.0, -1.0, 8.0]);
        rotate_about_axis([0.0; 3], [1.0, 2.0, 3.0], 0.0, 0, &mut out);
        assert_eq!(out, [0.0, 2.0, 3.0]);
    }
    #[test]
    fn transform_offset_quirk_negative_zeros_and_counter_wrap() {
        let mut host = Host::default();
        let mut record = Transform {
            counter: 255,
            flags: 0x80,
            ..Transform::default()
        };
        find_model_transform(input(false), &mut record, &mut host);
        assert_eq!(record.translation, [6.0, 23.0, 32.0]);
        assert_eq!((record.counter, record.flags), (1, 0x85));
        assert_eq!(record.rotation[0][1].to_bits(), (-0.0f32).to_bits());
        assert_eq!(record.rotation[1][2].to_bits(), (-0.0f32).to_bits());
        find_model_transform(input(true), &mut record, &mut host);
        assert_eq!(record.translation, [12.0, 23.0, 34.0]);
        assert_eq!(record.rotation[0][2], -1.0);
        assert_eq!(record.rotation[2][0], 1.0);
    }
    #[test]
    fn tilt_normalises_objects_limits_only_positive_axes_and_prefers_objects() {
        let mut host = Host {
            object: Some([4.0, 3.0, 0.0]),
            terrain: Some([0.0, 1.0, 0.0]),
            ..Host::default()
        };
        let mut record = Transform::default();
        let mut params = input(true);
        params.tilt_to_terrain = true;
        find_model_transform(params, &mut record, &mut host);
        assert_eq!(record.counter, 3);
        assert!((record.rotation[0][1] - 0.6401844).abs() < 1e-6);
        assert!((record.rotation[1][1] - 0.7682213).abs() < 1e-6);
        assert_eq!(host.terrain_reads, 0);
        host.object = Some([-4.0, 3.0, 0.0]);
        find_model_transform(params, &mut record, &mut host);
        assert_eq!(record.rotation[0][1], -0.8);
        assert_eq!(record.rotation[1][1], 0.6);
        host.object = None;
        find_model_transform(params, &mut record, &mut host);
        assert_eq!(host.terrain_reads, 1);
        assert_eq!(record.counter, 8); // upward normal needs no second rotation
    }
    #[test]
    fn shadow_rotation_uses_binary32_angle_and_changes_columns_once() {
        let mut record = Transform::default();
        record.rotate_shadow();
        assert_eq!((record.flags, record.counter), (4, 1));
        assert_eq!(record.rotation[0][0].to_bits(), 0xB33B_BD2E);
        assert_eq!(record.rotation[0][2], -1.0);
        assert_eq!(record.rotation[2][0], 1.0);
    }
}
