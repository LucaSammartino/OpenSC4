// SPDX-License-Identifier: GPL-3.0-or-later
//! Selected finite, unrotated normal domain of public §3.2.
//! f64 intermediates model the explicitly named stores, not ambient x87 precision.
use super::{CellRect, Error, Geometry, Placement, Unsupported};

pub(super) fn calculate(p: Placement, aspect: f32, g: Geometry) -> Result<([f32; 16], CellRect, bool), Error> {
    let [cx, cz, w, turns, stretch, ru, rv, offset, _] = p.0;
    let a = (f64::from(aspect) * f64::from(stretch)) as f32;
    if p.0.iter().any(|x| !x.is_finite()) || !a.is_finite() || a <= 0.0 || w <= 0.0 || turns != 0.0 {
        return Err(Error::Unsupported(Unsupported::PlacementNumericDomain));
    }
    let hx = (f64::from(w) * 0.5) as f32;
    let hz = ((f64::from(a) * f64::from(w)) * 0.5) as f32;
    let rw = 1.0 / f64::from(w);
    let r = (1.0 / f64::from(a)) * rw;
    let du = ((f64::from(hx) - f64::from(cx)) * rw) as f32;
    let dv = ((f64::from(hz) - f64::from(cz)) * r) as f32;
    let q = (0.0 * rw) as f32;
    let v = (f64::from((f64::from(rv) * f64::from(dv)) as f32) + f64::from(offset)) as f32;
    let t = f64::from((f64::from(ru) * f64::from(du)) as f32) + f64::from(offset);
    let matrix = [
        (f64::from(ru)*rw) as f32, (f64::from(rv)*f64::from((0.0*r) as f32)) as f32, 0.0, 0.0,
        (f64::from(ru)*f64::from(q)) as f32, (f64::from(rv)*(0.0*r)) as f32, 0.0, 0.0,
        (f64::from(ru)*f64::from(q)) as f32, (f64::from(rv)*f64::from(r as f32)) as f32, 1.0, 0.0,
        t as f32, v, 0.0, 1.0,
    ];
    let bounds = [
        ((f64::from(cx)-f64::from(hx))*f64::from(g.cell_scale)) as f32,
        ((f64::from(cz)-f64::from(hz))*f64::from(g.cell_scale)) as f32,
        ((f64::from(cx)+f64::from(hx))*f64::from(g.cell_scale)) as f32,
        ((f64::from(cz)+f64::from(hz))*f64::from(g.cell_scale)) as f32,
    ];
    if bounds.iter().chain(matrix.iter()).any(|x| !x.is_finite()) || bounds.iter().any(|x| f64::from(*x).abs() >= f64::from(i32::MAX)) {
        return Err(Error::Unsupported(Unsupported::PlacementNumericDomain));
    }
    let b = [bounds[0].floor() as i32, bounds[1].floor() as i32, bounds[2].ceil() as i32, bounds[3].ceil() as i32];
    let cells = if b[2] < 0 || b[3] < 0 || i64::from(b[0]) > i64::from(g.cells[0]) || i64::from(b[1]) > i64::from(g.cells[1]) {
        CellRect { min: [0, 0], max: [0, 0] }
    } else {
        CellRect { min: [b[0].max(0) as u32, b[1].max(0) as u32], max: [(b[2].max(0) as u32).min(g.cells[0]), (b[3].max(0) as u32).min(g.cells[1])] }
    };
    Ok((matrix, cells, ru.to_bits() != 0x3f80_0000 || rv.to_bits() != 0x3f80_0000))
}
