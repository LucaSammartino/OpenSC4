// SPDX-License-Identifier: GPL-3.0-or-later
//! Public OV9 outline partition, finite conversion domain only.
use super::{CellRect, Error, Unsupported};

pub(super) fn clip(b: [i32; 4], city: [u32; 2]) -> CellRect {
    if b[2] < 0
        || b[3] < 0
        || i64::from(b[0]) > i64::from(city[0])
        || i64::from(b[1]) > i64::from(city[1])
    {
        return CellRect {
            min: [0, 0],
            max: [0, 0],
        };
    }
    CellRect {
        min: [b[0].max(0) as u32, b[1].max(0) as u32],
        max: [
            (b[2].max(0) as u32).min(city[0]),
            (b[3].max(0) as u32).min(city[1]),
        ],
    }
}

pub(super) fn partition(f: [f32; 4], city: [u32; 2]) -> Result<[CellRect; 8], Error> {
    // Conservative portable conversion boundary, not a native guard or saturation rule.
    if f.iter().any(|v| !v.is_finite() || v.abs() > 1_000_000.0) || f[2] < f[0] || f[3] < f[1] {
        return Err(Error::Unsupported(Unsupported::PlacementNumericDomain));
    }
    let [fx0, fz0, fx1, fz1] = f.map(f64::from);
    let w = fx1 - fx0;
    let h = (fz1 - fz0) as f32;
    let cx = ((fx0 + fx1) * 0.5) as f32;
    let cz = ((fz0 + fz1) * 0.5) as f32;
    let r2 = ((w * 0.5) * (w * 0.5)) as f32;
    let inner = ((1.0 - f64::from(0.1_f32)) * w) * 0.5;
    let i2 = (inner * inner) as f32;
    let n1 = ((f64::from(0.2_f32) * f64::from(h)) * 0.5).ceil() as i32;
    let n2 = ((f64::from(0.4_f32) * f64::from(h)) * 0.5).ceil() as i32;
    let x0 = fx0.floor() as i32;
    let x1 = fx1.ceil() as i32;
    let z0 = fz0.floor() as i32;
    let z5 = fz1.ceil() as i32;
    let rows = [z0 + n1, z0 + n2, z5 - n2, z5 - n1];
    let mut left = [0; 4];
    let mut right = [0; 4];
    let mut il = [0; 4];
    let mut ir = [0; 4];
    for (i, z) in rows.into_iter().enumerate() {
        let delta = f64::from(cz) - f64::from(z);
        let square = (delta * delta) as f32;
        let outer = (f64::from(r2) - f64::from(square.min(r2))).sqrt() as f32;
        let inner = (f64::from(i2) - f64::from(square.min(i2))).sqrt();
        let inner = if i == 3 {
            inner
        } else {
            f64::from(inner as f32)
        };
        left[i] = (f64::from(cx) - f64::from(outer)).floor() as i32;
        right[i] = (f64::from(cx) + f64::from(outer)).ceil() as i32;
        il[i] = (f64::from(cx) - inner).ceil() as i32;
        ir[i] = (f64::from(cx) + inner).floor() as i32;
    }
    let [z1, z2, z3, z4] = rows;
    let b = ir[1].min(ir[2]);
    let a = il[1].max(il[2]).min(b);
    Ok([
        [left[0], z0, right[0], z1],
        [left[3], z4, right[3], z5],
        [x0, z2, a, z3],
        [b, z2, x1, z3],
        [left[1], z1, il[0], z2],
        [left[2], z3, il[3], z4],
        [ir[0], z1, right[1], z2],
        [ir[3], z3, right[2], z4],
    ]
    .map(|r| clip(r, city)))
}
