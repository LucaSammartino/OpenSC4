// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned checked adapter (SR10), not a native object layout. See span-render-8 §§1–7.
use fc4_core::span_render8::{self as sr, BltParams, Gradient, SourceImage, Span, Target};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Image {
    pub bytes: *mut u8,
    pub len: usize,
    pub origin: usize,
    pub stride: i32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Job {
    pub slot: u32,
    pub count: u32,
    pub spans: *const u32,
    pub dst: Image,
    pub colour: u32,
    pub argb: u32,
    pub gradient: *const u32,
    pub src: Image,
    pub params: *const u32,
    pub kind: u32,
}

fn region(p: *const u8, n: usize) -> Option<(usize, usize)> {
    let a = p as usize;
    if a == 0 || n == 0 || n > isize::MAX as usize {
        return None;
    }
    Some((a, a.checked_add(n)?))
}

/// Return 0 rendered, 1 invalid adapter arguments, 2 checked core refusal,
/// 3 unsupported row 13 (SR8-3). All refusals leave the destination unchanged.
/// Separate source/destination regions only; aliasing is an adapter refusal.
///
/// # Safety
/// `job` is readable and aligned; spans hold `2*count` aligned dwords.
/// Images denote live allocations of the stated lengths, destination writable.
/// Gradient holds 12 dwords for slots 2/4; params holds 8 for slot 5.
/// The caller must exclude concurrent access to the destination allocation.
#[no_mangle]
pub unsafe extern "C" fn fc4_span8_render(job: *const Job) -> i32 {
    if job.is_null() {
        return 1;
    }
    let j = *job;
    if !(1..=5).contains(&j.slot) || j.spans.is_null() || j.count == 0 || j.count > 65536 {
        return 1;
    }
    let Some(d) = region(j.dst.bytes, j.dst.len) else {
        return 1;
    };
    let spans: Vec<_> = std::slice::from_raw_parts(j.spans, j.count as usize * 2)
        .chunks_exact(2)
        .map(|p| Span { start: p[0], end: p[1] })
        .collect();
    // Copy metadata before constructing the exclusive destination slice.
    let mut g = Gradient::default();
    if j.slot == 2 || j.slot == 4 {
        if j.gradient.is_null() {
            return 1;
        }
        let raw = std::slice::from_raw_parts(j.gradient, 12);
        g.start.copy_from_slice(&raw[..4]);
        g.dx.copy_from_slice(&raw[4..8]);
        g.dy.copy_from_slice(&raw[8..12]);
    }
    let mut b = BltParams::default();
    if j.slot == 5 {
        if j.params.is_null() {
            return 1;
        }
        let p = std::slice::from_raw_parts(j.params, 8);
        b = BltParams {
            u0: p[0],
            v0: p[1],
            du: p[2],
            dv: p[3],
            format: p[4],
            colour: p[5],
            key: p[6],
            key_flag: p[7] as u8,
        };
        if b.row() == 13 {
            return 3; // SR8-3: intentionally unsupported even though the core models the literal reading.
        }
        let Some(s) = region(j.src.bytes, j.src.len) else {
            return 1;
        };
        if d.0 < s.1 && s.0 < d.1 {
            return 1;
        }
    }
    let mut t = Target {
        data: std::slice::from_raw_parts_mut(j.dst.bytes, j.dst.len),
        origin: j.dst.origin,
        stride: j.dst.stride,
    };
    let result = match j.slot {
        1 => sr::render_solid(&spans, &mut t, j.colour),
        2 => sr::render_gradient(&spans, &mut t, &g),
        3 => sr::render_alpha(&spans, &mut t, j.colour, j.argb),
        4 => sr::render_alpha_gradient(&spans, &mut t, &g),
        _ => sr::render_blt(
            &spans,
            &mut t,
            std::slice::from_raw_parts(j.src.bytes, j.src.len),
            &SourceImage {
                origin: j.src.origin,
                stride: j.src.stride,
                width: j.src.width,
                height: j.src.height,
            },
            &b,
            j.kind,
        ),
    };
    if result.is_ok() {
        0
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_adapter_atomic_refusal_and_guarded_alpha_plane() {
        let mut dst = [0xA5; 24];
        let mut src = [0x40; 16];
        let spans = [0, 3, 0x10001, 0x10004];
        let p = [0, 0, 0x10000, 0x10000, 6, 0xC0123456, 0, 0];
        let image = |bytes: *mut u8, len| Image {
            bytes,
            len,
            origin: 0,
            stride: 8,
            width: 5,
            height: 2,
        };
        let mut j = Job {
            slot: 5,
            count: 2,
            spans: spans.as_ptr(),
            dst: image(dst.as_mut_ptr(), 24),
            colour: 0,
            argb: 0,
            gradient: std::ptr::null(),
            src: image(src.as_mut_ptr(), 16),
            params: p.as_ptr(),
            kind: 2,
        };
        j.dst.origin = 4;
        assert_eq!(unsafe { fc4_span8_render(&j) }, 0);
        let expected = [
            0xA5, 0xA5, 0xA5, 0xA5, 0x30, 0x30, 0x30, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0x30, 0x30, 0x30, 0xA5, 0xA5,
            0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5,
        ];
        assert_eq!(dst, expected);
        j.dst.len = 14; // Later span out of bounds: no first-span partial write.
        assert_eq!(unsafe { fc4_span8_render(&j) }, 2);
        assert_eq!(dst, expected);
        j.dst.len = 24;
        let mut row13 = p;
        row13[7] = 1;
        j.params = row13.as_ptr();
        assert_eq!(unsafe { fc4_span8_render(&j) }, 3);
        j.params = p.as_ptr();
        j.src.bytes = dst.as_mut_ptr().wrapping_add(5);
        assert_eq!(unsafe { fc4_span8_render(&j) }, 1);
        assert_eq!(dst, expected);
    }
}
