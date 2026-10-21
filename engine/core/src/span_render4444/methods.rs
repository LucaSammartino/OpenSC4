// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors. See LICENSE at the repository root.

//! Vtable slots 1-4 of the 4444 span renderer (§3-§6).

use super::{blend4444, Span, SpanError, Target};

/// RenderSolid (slot 1, §3): fill each span with the low 16 bits of
/// `colour`, alpha nibble included, verbatim.
pub fn render_solid(spans: &[Span], target: &mut Target, colour: u32) -> Result<(), SpanError> {
    let offs = target.check_spans(spans)?;
    let px = colour as u16;
    for (span, off) in spans.iter().zip(offs) {
        for i in 0..span.width() as usize {
            target.put(off + 2 * i, px);
        }
    }
    Ok(())
}

/// RenderAlpha weight (§4): `((alpha >> 27) + 1) >> 1`, 0..16.
pub fn render_alpha_weight(alpha: u32) -> u32 {
    ((alpha >> 27) + 1) >> 1
}

/// RenderAlpha (slot 3, §4): blend the R, G, B nibbles of the low 16 bits
/// of `colour` over each destination pixel with the §4 weight; output alpha
/// 0xF.
pub fn render_alpha(spans: &[Span], target: &mut Target, colour: u32, alpha: u32) -> Result<(), SpanError> {
    let offs = target.check_spans(spans)?;
    let a = render_alpha_weight(alpha);
    let s = colour as u16;
    for (span, off) in spans.iter().zip(offs) {
        for i in 0..span.width() as usize {
            let o = off + 2 * i;
            let d = target.get(o);
            target.put(o, blend4444(s, d, a));
        }
    }
    Ok(())
}

/// Gradient record (§5): 12 signed dwords. Index 0, 1, 2, 3 = B, G, R, A in
/// 8.16 fixed point (8-bit value in bits 16-23).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Gradient {
    pub start: [i32; 4],
    pub dx: [i32; 4],
    pub dy: [i32; 4],
}

impl Gradient {
    /// `start + dx*x + dy*y` in 32-bit wrapping maths (§5).
    fn at(&self, ch: usize, x: u32, y: u32) -> u32 {
        (self.start[ch] as u32)
            .wrapping_add((self.dx[ch] as u32).wrapping_mul(x))
            .wrapping_add((self.dy[ch] as u32).wrapping_mul(y))
    }

    /// B, G, R accumulators and steps for a span starting at (x, y) (§5):
    /// B start `>> 4` logical, step `>> 4` arithmetic; G unchanged; R start
    /// and step `<< 4` (the caller ORs 0xF0000000 for RenderGradient).
    fn rgb_setup(&self, x: u32, y: u32) -> ([u32; 3], [u32; 3]) {
        let acc = [self.at(0, x, y) >> 4, self.at(1, x, y), self.at(2, x, y) << 4];
        let step = [(self.dx[0] >> 4) as u32, self.dx[1] as u32, (self.dx[2] as u32) << 4];
        (acc, step)
    }
}

/// RenderGradient (slot 2, §5): opaque linear ramp, channels truncated to
/// their top nibble. The alpha nibble is the top nibble of the R
/// accumulator, so an R ramp leaving 0..255.99 mid-span carries into alpha
/// (U2); this is reproduced, not corrected.
pub fn render_gradient(spans: &[Span], target: &mut Target, grad: &Gradient) -> Result<(), SpanError> {
    let offs = target.check_spans(spans)?;
    for (span, off) in spans.iter().zip(offs) {
        let (mut acc, step) = grad.rgb_setup(span.x(), span.y());
        acc[2] |= 0xF000_0000;
        for i in 0..span.width() as usize {
            let v = (acc[2] & 0xFF00_0000)
                .wrapping_add(acc[1] & 0x00F0_0000)
                .wrapping_add(acc[0] & 0x000F_0000);
            target.put(off + 2 * i, (v >> 16) as u16);
            for (a, s) in acc.iter_mut().zip(step) {
                *a = a.wrapping_add(s);
            }
        }
    }
    Ok(())
}

/// RenderAlphaGradient weight from an alpha accumulator (§6): `acc >> 27`.
/// For an 8.16 alpha `A` (accumulator `A * 0x88`) this is floor(A*17/256).
pub fn alpha_gradient_weight(acc: u32) -> u32 {
    acc >> 27
}

/// RenderAlphaGradient (slot 4, §6): colour and alpha ramp blended over the
/// destination with the §4 blend; output alpha 0xF.
///
/// An alpha ramp outside 0..255 gives weights 17..31 (or wraps); the blend is
/// then evaluated as documented on `blend4444` ([choice], question SP1).
pub fn render_alpha_gradient(spans: &[Span], target: &mut Target, grad: &Gradient) -> Result<(), SpanError> {
    let offs = target.check_spans(spans)?;
    for (span, off) in spans.iter().zip(offs) {
        let (mut acc, step) = grad.rgb_setup(span.x(), span.y());
        let mut a_acc = grad.at(3, span.x(), span.y()).wrapping_mul(0x88);
        let a_step = (grad.dx[3] as u32).wrapping_mul(0x88);
        for i in 0..span.width() as usize {
            let r = (acc[2] >> 24) & 0xF;
            let g = (acc[1] >> 20) & 0xF;
            let b = (acc[0] >> 16) & 0xF;
            let s = ((r << 8) | (g << 4) | b) as u16;
            let o = off + 2 * i;
            let d = target.get(o);
            target.put(o, blend4444(s, d, alpha_gradient_weight(a_acc)));
            for (a, s) in acc.iter_mut().zip(step) {
                *a = a.wrapping_add(s);
            }
            a_acc = a_acc.wrapping_add(a_step);
        }
    }
    Ok(())
}
