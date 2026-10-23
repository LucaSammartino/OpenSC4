// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned §3.7 fallback and secondary matrix. Foreign lighting retains its opaque inputs.
use super::{round, CellRect, Error, Geometry, Unsupported};

pub(super) fn fallback(direction: [f32; 3], b: [f32; 6], g: Geometry) -> Result<CellRect, Error> {
    if b.iter().any(|v| !v.is_finite()) {
        return Err(Error::Unsupported(Unsupported::ModelNumericDomain));
    }
    let e = f64::from(f32::from_bits(0x3d8f_5c29));
    let scale = f64::from(g.cell_scale);
    let mut bounds = [
        ((f64::from(b[0]) + e) * scale).floor(),
        ((f64::from(b[2]) + e) * scale).floor(),
        ((f64::from(b[3]) - e) * scale).ceil(),
        ((f64::from(b[5]) - e) * scale).ceil(),
    ];
    let h = ((f64::from(b[4]) - f64::from(b[1])) * 0.0625).ceil();
    if direction[0] > 0.0 {
        bounds[2] += h;
    } else {
        bounds[0] -= h;
    }
    if direction[2] > 0.0 {
        bounds[3] += h;
    } else {
        bounds[1] -= h;
    }
    if bounds
        .iter()
        .any(|v| !v.is_finite() || *v <= f64::from(i32::MIN) || *v >= f64::from(i32::MAX))
    {
        return Err(Error::Unsupported(Unsupported::ModelNumericDomain));
    }
    Ok(round::clip(bounds.map(|v| v as i32), g.cells))
}

pub(super) fn extrema(pairs: &[[f32; 2]]) -> Result<[f32; 4], Error> {
    let first = pairs
        .first()
        .ok_or(Error::Unsupported(Unsupported::ModelNumericDomain))?;
    let mut b = [first[0], first[1], first[0], first[1]];
    for &[u, v] in &pairs[1..] {
        if u < b[0] {
            b[0] = u;
        }
        if v < b[1] {
            b[1] = v;
        }
        if u > b[2] {
            b[2] = u;
        }
        if v > b[3] {
            b[3] = v;
        }
    }
    Ok(b)
}

pub(super) fn secondary(p: [f32; 16], pairs: &[[f32; 2]]) -> Result<[f32; 16], Error> {
    let [umin, vmin, umax, vmax] = extrema(pairs)?;
    let du = (f64::from(umax) - f64::from(umin)) as f32;
    let dv2 = ((f64::from(vmax) - f64::from(vmin)) * 2.0) as f32;
    if p.iter().any(|v| !v.is_finite())
        || !du.is_finite()
        || !dv2.is_finite()
        || du <= 0.0
        || dv2 <= 0.0
    {
        return Err(Error::Unsupported(Unsupported::ModelNumericDomain));
    }
    let ru = 1.0 / (f64::from(du) * 2.0);
    let rv = 1.0 / f64::from(dv2);
    let su = f64::from(ru as f32);
    let sv = f64::from(rv as f32);
    let tu = f64::from((0.25 - ru * f64::from(umin)) as f32);
    let tv = f64::from((0.25 - rv * f64::from(vmin)) as f32);
    let mut out = [0.0; 16];
    for row in (0..4).rev() {
        let [a, b, c, d] =
            [p[row * 4], p[row * 4 + 1], p[row * 4 + 2], p[row * 4 + 3]].map(f64::from);
        let z = f64::from((c * 0.0) as f32);
        let y = f64::from((b * 0.0) as f32);
        let x = f64::from((a * 0.0) as f32);
        out[row * 4] = (((a * su + d * tu) + y) + z) as f32;
        out[row * 4 + 1] = (((d * tv + b * sv) + x) + z) as f32;
        out[row * 4 + 2] = (((d * 0.0 + c) + x) + y) as f32;
        out[row * 4 + 3] = (((d + x) + y) + z) as f32;
    }
    Ok(out)
}
