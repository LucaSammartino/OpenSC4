// SPDX-License-Identifier: GPL-3.0-or-later
//! Signed §5.1/5.2 APIs preserve logical count independently of output backing.
//! Arithmetic is qualified to PC24 nearest, masked, normal/zero intermediates;
//! the legacy Vec adapters cannot expose tail storage or negative counts.
use super::{pc24, Vec2, EPSILON};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipUnsupported {
    Backing,
    UninitializedCrossing,
    NumericDomain,
}

fn side(p: Vec2, e0: Vec2, e1: Vec2) -> Result<bool, ClipUnsupported> {
    // Quiet NaN is outside, per the public unordered side branch.
    if p.into_iter().chain(e0).chain(e1).any(f32::is_nan) {
        return Ok(false);
    }
    let result = (|| {
        let x = pc24::mul(pc24::sub(p[0], e0[0])?, pc24::sub(e1[1], e0[1])?)?;
        let y = pc24::mul(pc24::sub(p[1], e0[1])?, pc24::sub(e1[0], e0[0])?)?;
        Ok(pc24::sub(x, y)? >= 0.0)
    })();
    result.map_err(|_: pc24::NumericUnsupported| ClipUnsupported::NumericDomain)
}

fn crossing(p: Vec2, current: Vec2, e0: Vec2, e1: Vec2) -> Result<Option<Vec2>, ClipUnsupported> {
    let result = (|| {
        let d1 = [pc24::sub(current[0], p[0])?, pc24::sub(current[1], p[1])?];
        let d2 = [pc24::sub(e1[0], e0[0])?, pc24::sub(e1[1], e0[1])?];
        let neg_y = -d2[1];
        let k = pc24::add(pc24::mul(d2[0], d1[1])?, pc24::mul(neg_y, d1[0])?)?;
        if k.abs() < EPSILON {
            return Ok(None);
        }
        let numerator = pc24::add(
            pc24::mul(d2[0], pc24::sub(e0[1], p[1])?)?,
            pc24::mul(neg_y, pc24::sub(e0[0], p[0])?)?,
        )?;
        let t = pc24::div(numerator, k)?;
        Ok(Some([
            pc24::add(p[0], pc24::mul(t, d1[0])?)?,
            pc24::add(p[1], pc24::mul(t, d1[1])?)?,
        ]))
    })();
    result.map_err(|_: pc24::NumericUnsupported| ClipUnsupported::NumericDomain)
}

/// Each actual write checks accessible backing; max is a signed emission gate,
/// not permission to resize or to read uninitialized crossing coordinates.
#[allow(clippy::too_many_arguments)]
pub fn clip_edge_signed(
    e0: Vec2,
    e1: Vec2,
    poly: &[Vec2],
    n: i32,
    out: &mut [Vec2],
    max: i32,
    count: &mut i32,
) -> Result<(), ClipUnsupported> {
    *count = 0;
    if n <= 0 {
        return Ok(());
    }
    let poly = poly.get(..n as usize).ok_or(ClipUnsupported::Backing)?;
    let mut previous = poly[poly.len() - 1];
    let mut previous_inside = side(previous, e0, e1)?;
    let mut scratch = None;
    for &current in poly {
        let current_inside = side(current, e0, e1)?;
        if previous_inside != current_inside {
            // The call precedes the capacity test. A failed call preserves the
            // shared scratch from an earlier successful crossing, if one exists.
            if let Some(candidate) = crossing(previous, current, e0, e1)? {
                scratch = Some(candidate);
            }
            if *count < max {
                let point = scratch.ok_or(ClipUnsupported::UninitializedCrossing)?;
                *out.get_mut(*count as usize).ok_or(ClipUnsupported::Backing)? = point;
                *count += 1;
            }
        }
        if current_inside && *count < max {
            *out.get_mut(*count as usize).ok_or(ClipUnsupported::Backing)? = current;
            *count += 1;
        }
        previous = current;
        previous_inside = current_inside;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn clip_polygon_signed(
    poly: &[Vec2],
    n: i32,
    clip: &[Vec2],
    m: i32,
    out: &mut [Vec2],
    max: i32,
    count: &mut i32,
) -> Result<(), ClipUnsupported> {
    *count = n;
    if n > 0 {
        let source = poly.get(..n as usize).ok_or(ClipUnsupported::Backing)?;
        out.get_mut(..n as usize)
            .ok_or(ClipUnsupported::Backing)?
            .copy_from_slice(source);
    }
    if m <= 0 {
        return Ok(());
    }
    let clip = clip.get(..m as usize).ok_or(ClipUnsupported::Backing)?;
    let mut previous = clip[clip.len() - 1];
    let mut scratch = [[0.; 2]; 40];
    for &current in clip {
        let mut emitted = 0;
        clip_edge_signed(previous, current, out, *count, &mut scratch, 40, &mut emitted)?;
        *count = emitted.min(max);
        if *count > 0 {
            let len = *count as usize;
            out.get_mut(..len)
                .ok_or(ClipUnsupported::Backing)?
                .copy_from_slice(&scratch[..len]);
        }
        previous = current;
    }
    Ok(())
}
