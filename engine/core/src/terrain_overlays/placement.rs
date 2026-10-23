// SPDX-License-Identifier: GPL-3.0-or-later
//! Finite placement domain of public §§3.2/3.2.1.
//! f64 intermediates model the explicitly named stores, not ambient x87 precision.
use super::{CellRect, Error, Geometry, Placement, Unsupported};

pub(super) fn calculate(
    p: Placement,
    aspect: f32,
    g: Geometry,
) -> Result<([f32; 16], CellRect, bool, [f32; 4]), Error> {
    let [cx, cz, w, turns, stretch, ru, rv, offset, _] = p.0;
    let a = (f64::from(aspect) * f64::from(stretch)) as f32;
    // OV10: exceptional cell conversion is not specified; never fabricate bounds.
    if p.0[..8].iter().any(|x| !x.is_finite()) || !a.is_finite() || a == 0.0 || w == 0.0 {
        return Err(Error::Unsupported(Unsupported::PlacementNumericDomain));
    }
    let mut hx = (f64::from(w) * 0.5) as f32;
    let mut hz = ((f64::from(a) * f64::from(w)) * 0.5) as f32;
    let rw = 1.0 / f64::from(w);
    let r = (1.0 / f64::from(a)) * rw;
    let du = ((f64::from(hx) - f64::from(cx)) * rw) as f32;
    let dv = ((f64::from(hz) - f64::from(cz)) * r) as f32;
    let q = (0.0 * rw) as f32;
    let v = (f64::from((f64::from(rv) * f64::from(dv)) as f32) + f64::from(offset)) as f32;
    let t = f64::from((f64::from(ru) * f64::from(du)) as f32) + f64::from(offset);
    let mut matrix = [
        (f64::from(ru) * rw) as f32,
        (f64::from(rv) * f64::from((0.0 * r) as f32)) as f32,
        0.0,
        0.0,
        (f64::from(ru) * f64::from(q)) as f32,
        (f64::from(rv) * (0.0 * r)) as f32,
        0.0,
        0.0,
        (f64::from(ru) * f64::from(q)) as f32,
        (f64::from(rv) * f64::from(r as f32)) as f32,
        1.0,
        0.0,
        t as f32,
        v,
        0.0,
        1.0,
    ];
    let repeat = ru.to_bits() != 0x3f80_0000 || rv.to_bits() != 0x3f80_0000;
    if turns != 0.0 {
        // Ordered explicit stores from §3.2.1; portable f64 arithmetic models E.
        // Native x87 transcendental/ambient control-word equivalence is not claimed.
        let angle = f64::from(turns) * f64::from(f32::from_bits(0x40c9_0fdb));
        let c = f64::from(angle.cos() as f32);
        let s = f64::from(angle.sin() as f32);
        let n = f64::from((-angle.sin()) as f32);
        let dx = f64::from((f64::from(cx) * (1.0 - c) + f64::from(cz) * s) as f32);
        let dz = f64::from(cz) * (1.0 - c) - f64::from(cx) * s;
        let [aa, pp, _, _, bb, qq, _, _, cc, ss, _, _, _, vv, _, _] = matrix.map(f64::from);
        let b0 = f64::from((bb * 0.0) as f32);
        let q0 = f64::from((qq * 0.0) as f32);
        let t0 = f64::from((t * 0.0) as f32);
        let v0 = f64::from((vv * 0.0) as f32);
        let c0 = f64::from((c * 0.0) as f32);
        let n0 = n * 0.0;
        matrix = [
            (((aa * c + cc * s) + t0) + b0) as f32,
            (((pp * c + ss * s) + v0) + q0) as f32,
            (c0 + s) as f32,
            (s * 0.0 + c0) as f32,
            (((aa + cc) * 0.0 + t0) + bb) as f32,
            (((pp + ss) * 0.0 + v0) + qq) as f32,
            0.0,
            0.0,
            (((cc * c + aa * n) + t0) + b0) as f32,
            (((ss * c + pp * n) + v0) + q0) as f32,
            (c + n0) as f32,
            (c0 + n0) as f32,
            (((cc * dz + aa * dx) + b0) + t) as f32,
            (((ss * dz + pp * dx) + q0) + vv) as f32,
            (dx * 0.0 + dz) as f32,
            ((dz * 0.0 + dx * 0.0) + 1.0) as f32,
        ];
        if !repeat {
            hx = ((c.abs() + f64::from(a) * s.abs()) * f64::from(hx)) as f32;
            hz = ((c.abs() + f64::from((1.0 / f64::from(a)) as f32) * s.abs()) * f64::from(hz)) as f32;
        }
    }
    let bounds = [
        ((f64::from(cx) - f64::from(hx)) * f64::from(g.cell_scale)) as f32,
        ((f64::from(cz) - f64::from(hz)) * f64::from(g.cell_scale)) as f32,
        ((f64::from(cx) + f64::from(hx)) * f64::from(g.cell_scale)) as f32,
        ((f64::from(cz) + f64::from(hz)) * f64::from(g.cell_scale)) as f32,
    ];
    if bounds.iter().chain(matrix.iter()).any(|x| !x.is_finite())
        || bounds.iter().any(|x| f64::from(*x).abs() >= f64::from(i32::MAX))
    {
        return Err(Error::Unsupported(Unsupported::PlacementNumericDomain));
    }
    let b = [
        bounds[0].floor() as i32,
        bounds[1].floor() as i32,
        bounds[2].ceil() as i32,
        bounds[3].ceil() as i32,
    ];
    let cells = super::round::clip(b, g.cells);
    Ok((matrix, cells, repeat, bounds))
}
