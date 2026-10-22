// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable connection geometry, power-line-tool §§5.2–5.5.

use crate::x87::{ftol32, r24};

pub type Point = [f32; 3];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveParams {
    pub maximum_displacement: f32,
    pub displacement_percent: f32,
    pub maximum_sag: f32,
    pub sag_percent: f32,
    pub sample_distance: f32,
    pub minimum_count: u32,
}

impl Default for CurveParams {
    fn default() -> Self {
        Self {
            maximum_displacement: 16.0,
            displacement_percent: 10.0,
            maximum_sag: 10.0,
            sag_percent: f32::from_bits(0x3e19_999a),
            sample_distance: 8.0,
            minimum_count: 5,
        }
    }
}

impl CurveParams {
    /// Every call resets defaults before the six property reads in table order.
    pub fn load(mut float: impl FnMut(u32) -> Option<f32>, mut uint: impl FnMut(u32) -> Option<u32>) -> Self {
        let mut p = Self::default();
        for (id, dest) in [
            (0xE9A1_8D3F, &mut p.maximum_displacement),
            (0x49A1_E8C4, &mut p.displacement_percent),
            (0x89A1_8FC4, &mut p.maximum_sag),
            (0xE9A1_E8D4, &mut p.sag_percent),
            (0x49A1_95C0, &mut p.sample_distance),
        ] {
            if let Some(v) = float(id) {
                *dest = v;
            }
        }
        if let Some(v) = uint(0x89A1_E8FB) {
            p.minimum_count = v;
        }
        p
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub first: Point,
    pub second: Point,
    /// Literal `_ftol` low dword. No inferred count clamp, including invalid resources.
    pub count: i32,
}

/// x87 operation rounding uses the established core precision contract (§18 foundations).
pub fn control_points(a: Point, b: Point, p: CurveParams) -> Controls {
    let d = std::array::from_fn::<_, 3, _>(|i| r24(f64::from(b[i]) - f64::from(a[i])));
    let z2 = r24(d[2] * f64::from(d[2] as f32));
    let squared = r24(r24(z2 + r24(d[1] * d[1])) + r24(d[0] * d[0]));
    let root = r24(squared.sqrt());
    let length = f64::from(root as f32);
    let inverse = r24(1.0 / root) as f32;
    let unit = d.map(|v| r24(v * f64::from(inverse)) as f32);
    let percentage = f64::from(f32::from_bits(0x3c23_d70a));
    let displacement = r24(r24(f64::from(p.displacement_percent) * percentage) * length) as f32;
    let sag = r24(r24(r24(f64::from(p.sag_percent) * percentage) * length) * length) as f32;
    // Strict comparison preserves unordered computed values, unlike f32::min.
    let g = if p.maximum_displacement < displacement {
        p.maximum_displacement
    } else {
        displacement
    };
    let s = if p.maximum_sag < sag { p.maximum_sag } else { sag };
    let half = r24(length * 0.5);
    let first_distance = r24(half - f64::from(g));
    let second_distance = r24(half + f64::from(g));
    let mut first = std::array::from_fn(|i| r24(f64::from(a[i]) + r24(f64::from(unit[i]) * first_distance)) as f32);
    let mut second = std::array::from_fn(|i| r24(f64::from(a[i]) + r24(f64::from(unit[i]) * second_distance)) as f32);
    first[1] = r24(f64::from(first[1]) - f64::from(s)) as f32;
    second[1] = r24(f64::from(second[1]) - f64::from(s)) as f32;
    let quotient = r24(length / f64::from(p.sample_distance));
    let minimum = f64::from(p.minimum_count as f32);
    let count = ftol32(if quotient < minimum {
        minimum
    } else {
        f64::from(quotient as f32)
    });
    Controls { first, second, count }
}

/// Writes t=1/n through repeated x87 addition; never overwrites the final point with B.
pub fn sample_cubic(a: Point, c1: Point, c2: Point, b: Point, count: usize) -> Vec<Point> {
    let step = r24(1.0 / count as f64);
    let mut t = step;
    let mut output = Vec::with_capacity(count);
    for _ in 0..count {
        let t2 = r24(t * t);
        let t3 = r24(t2 * t);
        let three_t = r24(3.0 * t);
        let three_t2 = r24(3.0 * t2);
        let w0 = r24(r24(r24(1.0 - three_t) + three_t2) - t3);
        let w1 = r24(r24(three_t - r24(6.0 * t2)) + r24(3.0 * t3));
        let w2 = r24(three_t2 - r24(3.0 * t3));
        let weights = [w0, w1, w2, t3];
        let points = [a, c1, c2, b];
        let mut out = [0.0; 3];
        for (axis, order) in [[0, 1, 2, 3], [3, 1, 2, 0], [2, 0, 3, 1]].iter().enumerate() {
            let mut sum = r24(f64::from(points[order[0]][axis]) * weights[order[0]]);
            for &i in &order[1..] {
                sum = r24(sum + r24(f64::from(points[i][axis]) * weights[i]));
            }
            out[axis] = sum as f32;
        }
        output.push(out);
        t = r24(t + step);
    }
    output
}

/// Attachment orientation, not a compass direction. Comparisons are unsigned.
pub fn direction(a: (i32, i32), b: (i32, i32)) -> u32 {
    if a.1 == b.1 {
        0
    } else if a.0 == b.0 {
        2
    } else if ((a.0 as u32) < (b.0 as u32)) == ((a.1 as u32) < (b.1 as u32)) {
        1
    } else {
        3
    }
}

pub fn lesser(a: (i32, i32), b: (i32, i32)) -> bool {
    (a.1 as u32, a.0 as u32) < (b.1 as u32, b.0 as u32)
}
