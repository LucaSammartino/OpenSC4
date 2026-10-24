// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent renderer-driver §15 vectors DX7-50–62.
use super::*;

fn bytes(h: &mut Recorder, address: u32, size: usize) -> Vec<u8> {
    let mut result = vec![0; size];
    h.read(address, &mut result).unwrap();
    result
}
fn ordinary_vector(source: u32, input: &[u8], expected: &[(u32, u32, usize)]) {
    for &(key, pixel, size) in expected {
        let mut h = Recorder::default();
        h.write(0x1000, input).unwrap();
        h.write(0x1010, input).unwrap();
        h.write(0x2000, &[0xa5; 32]).unwrap();
        texture::convert(source, key, 0x1000, 0x2000, 16, 16, 1, 2, &mut h).unwrap();
        for offset in [0, 16] {
            assert_eq!(
                bytes(&mut h, 0x2000 + offset, size),
                pixel.to_le_bytes()[..size],
                "source {source}, key {key}"
            );
            assert_eq!(
                bytes(&mut h, 0x2000 + offset + size as u32, 16 - size),
                vec![0xa5; 16 - size]
            );
        }
    }
}

#[test]
fn dx7_50_argb4_all_ordinary_destinations() {
    let expected = [
        (0, 0x001188ff, 3),
        (1, 0x881188ff, 4),
        (2, 0x881188ff, 4),
        (3, 0x145f, 2),
        (4, 0x8a3f, 2),
        (5, 0x8a3f, 2),
        (7, 0x818f, 2),
    ];
    ordinary_vector(4, &0x818fu16.to_le_bytes(), &expected);
    ordinary_vector(5, &0x8f81u16.to_le_bytes(), &expected);
    ordinary_vector(4, &0x1234u16.to_le_bytes(), &[(4, 0x10c8, 2)]);
}

#[test]
fn dx7_51_argb1555_replication() {
    let expected = [
        (0, 0x002184ff, 3),
        (1, 0xff2184ff, 4),
        (2, 0xff2184ff, 4),
        (3, 0x243f, 2),
        (4, 0x921f, 2),
        (5, 0x921f, 2),
        (7, 0xf28f, 2),
    ];
    ordinary_vector(6, &0x921fu16.to_le_bytes(), &expected);
    ordinary_vector(7, &0xfe04u16.to_le_bytes(), &expected);
    ordinary_vector(6, &0x8000u16.to_le_bytes(), &[(7, 0xf000, 2)]);
}

#[test]
fn dx7_52_byte_quantization_alpha_and_order() {
    let expected = [
        (0, 0x00123456, 3),
        (1, 0x7f123456, 4),
        (2, 0x7f123456, 4),
        (3, 0x11aa, 2),
        (4, 0x08ca, 2),
        (5, 0x08ca, 2),
        (7, 0x7135, 2),
    ];
    ordinary_vector(0, &0x7f123456u32.to_le_bytes(), &expected);
    ordinary_vector(1, &0x7f563412u32.to_le_bytes(), &expected);
    let absent = [
        (0, 0x00123456, 3),
        (1, 0xff123456, 4),
        (2, 0xff123456, 4),
        (3, 0x11aa, 2),
        (4, 0x88ca, 2),
        (5, 0x88ca, 2),
        (7, 0xf135, 2),
    ];
    ordinary_vector(2, &[0x56, 0x34, 0x12], &absent);
    ordinary_vector(3, &[0x12, 0x34, 0x56], &absent);
    ordinary_vector(8, &0x243fu16.to_le_bytes(), &[(1, 0xff2186ff, 4)]);
    ordinary_vector(9, &0xfc24u16.to_le_bytes(), &[(1, 0xff2186ff, 4)]);
}

fn block(c0: u16, c1: u16, selectors: u32, alpha: Option<u64>) -> Vec<u8> {
    let mut result = Vec::new();
    if let Some(alpha) = alpha {
        result.extend(alpha.to_le_bytes());
    }
    result.extend(c0.to_le_bytes());
    result.extend(c1.to_le_bytes());
    result.extend(selectors.to_le_bytes());
    result
}
fn decoded_row(source: u32, key: u32, block: &[u8]) -> Vec<u32> {
    let mut h = Recorder::default();
    h.write(0x1000, block).unwrap();
    let size = if matches!(key, 1 | 2) { 4 } else { 2 };
    h.write(0x2000, &[0xa5; 96]).unwrap();
    texture::convert(source, key, 0x1000, 0x2000, 99, 24, 4, 4, &mut h).unwrap();
    for row in 0..4 {
        assert_eq!(
            bytes(&mut h, 0x2000 + row * 24 + 4 * size, (24 - 4 * size) as usize),
            vec![0xa5; (24 - 4 * size) as usize]
        );
    }
    (0..4)
        .map(|i| {
            let data = bytes(&mut h, 0x2000 + i * size, size as usize);
            if size == 4 {
                u32::from_le_bytes(data.try_into().unwrap())
            } else {
                u16::from_le_bytes(data.try_into().unwrap()) as u32
            }
        })
        .collect()
}

#[test]
fn dx7_53_dxt1_weighted_four_color_palette() {
    let data = block(0xffff, 0, 0xe4e4e4e4, None);
    for (key, expected) in [
        (1, [0xffffffff, 0xff000000, 0xffaaaaaa, 0xff555555]),
        (2, [0xffffffff, 0xff000000, 0xffaaaaaa, 0xff555555]),
        (4, [0xffff, 0x8000, 0xd294, 0xad6b]),
        (5, [0xffff, 0x8000, 0xd294, 0xad6b]),
        (7, [0xffff, 0xf000, 0xfaaa, 0xf555]),
    ] {
        assert_eq!(decoded_row(10, key, &data), expected);
    }
}

#[test]
fn dx7_54_dxt1_and_dxt3_endpoint_order_branch() {
    for (key, mid, opaque) in [
        (1, 0xff808080, 0xff000000),
        (2, 0xff808080, 0xff000000),
        (4, 0xc210, 0x8000),
        (5, 0xc210, 0x8000),
        (7, 0xf888, 0xf000),
    ] {
        assert_eq!(
            decoded_row(10, key, &block(0, 0xffff, 0xeeeeeeee, None)),
            [mid, 0, mid, 0]
        );
        assert_eq!(
            decoded_row(12, key, &block(0, 0xffff, 0xeeeeeeee, Some(u64::MAX))),
            [mid, opaque, mid, opaque]
        );
    }
}

#[test]
fn dx7_55_dxt3_independent_alpha_nibbles() {
    let data = block(0, 0, 0, Some(0xf870f870f870f870));
    for (key, expected) in [
        (1, [0, 0x77000000, 0x88000000, 0xff000000]),
        (4, [0, 0, 0x8000, 0x8000]),
        (7, [0, 0x7000, 0x8000, 0xf000]),
    ] {
        assert_eq!(decoded_row(12, key, &data), expected);
    }
}

#[test]
fn dx7_56_dxt_whole_blocks_ignore_source_stride() {
    for (source, data) in [
        (10, block(0xffff, 0, 0, None)),
        (12, block(0xffff, 0, 0, Some(u64::MAX))),
    ] {
        for stride in [0, 8, 999] {
            let mut h = Recorder::default();
            h.write(0x1000, &data).unwrap();
            h.write(0x2000, &[0xa5; 120]).unwrap();
            texture::convert(source, 1, 0x1000, 0x2000, stride, 24, 6, 5, &mut h).unwrap();
            for row in 0..5 {
                for col in 0..6 {
                    assert_eq!(
                        bytes(&mut h, 0x2000 + row * 24 + col * 4, 4),
                        if row < 4 && col < 4 {
                            vec![255; 4]
                        } else {
                            vec![0xa5; 4]
                        }
                    );
                }
            }
        }
        for (w, hgt) in [(3, 4), (4, 3)] {
            let mut h = Recorder::default();
            h.write(0x2000, &[0xa5; 120]).unwrap();
            texture::convert(source, 1, 0x1000, 0x2000, 999, 24, w, hgt, &mut h).unwrap();
            assert_eq!(bytes(&mut h, 0x2000, 120), [0xa5; 120]);
        }
    }
}

#[test]
fn dx7_57_failed_probe_persists_through_mode_and_loss() {
    let (mut d, mut h) = fixture();
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    h.failures
        .extend([(Call::DrawBuffer, FAIL), (Call::LockColorSurface, FAIL)]);
    assert_eq!(d.copy_color_buffer(0, 0, 1, 1, 90, &mut h), Ok(0));
    assert_eq!(d.error, 1); // draw error retained; probe failure does not assign it
    assert_eq!(positions(&h, Call::DrawBuffer).len(), 1);
    assert_eq!(d.color_probe, Some(false));
    d.error = 27;
    assert_eq!(d.copy_color_buffer(0, 0, 1, 1, 90, &mut h), Ok(0));
    assert_eq!(d.error, 27);
    d.set_video_mode(u32::MAX, 0, false, 0, &mut h).unwrap();
    assert!(d.setup(0, &mut h).unwrap());
    h.failures.push_back((Call::CooperativeLevel, LOST));
    d.flush(&mut h).unwrap();
    assert!(d.lost);
    let before = h.calls.len();
    assert_eq!(d.copy_color_buffer(0, 0, 1, 1, 90, &mut h), Ok(0));
    assert_eq!(h.calls.len(), before);
    d.flush(&mut h).unwrap();
    assert!(!d.lost);
    assert_eq!(d.copy_color_buffer(0, 0, 1, 1, 90, &mut h), Ok(0));
    assert_eq!(positions(&h, Call::LockColorSurface).len(), 1);
}

#[test]
fn dx7_58_capture_failure_returns_borrowed_destination() {
    let (mut d, mut h) = fixture();
    h.replies.push_back((Call::LockColorSurface, 0x8876021c, vec![]));
    h.replies.push_back((Call::LockColorSurface, 0, vec![]));
    h.replies.push_back((Call::LockColorSurface, FAIL, vec![]));
    h.refs.clear();
    assert_eq!(d.copy_color_buffer(0, 0, 1, 1, 90, &mut h), Ok(90));
    assert_eq!(positions(&h, Call::UnlockSurface).len(), 1); // probe only
    assert!(positions(&h, Call::ColorBufferInfo).is_empty());
    assert!(h.refs.is_empty());
    h.calls.clear();
    h.replies.push_back((Call::ColorBufferInfo, 0, vec![0, 0, 0]));
    h.failures.push_back((Call::InitColorBuffer, FAIL));
    assert_eq!(d.copy_color_buffer(0, 0, 1, 1, 90, &mut h), Ok(90));
    assert_eq!(positions(&h, Call::UnlockSurface).len(), 1);
    assert_eq!(
        h.calls.iter().find(|r| r.call == Call::InitColorBuffer).unwrap().args,
        [1, 1, 9]
    );
    h.calls.clear();
    h.replies.extend([
        (Call::LockColorBuffer, FAIL, vec![]),
        (Call::LockColorBuffer, FAIL, vec![]),
    ]);
    assert_eq!(d.copy_color_buffer(0, 0, 1, 1, 90, &mut h), Ok(90));
    assert_eq!(positions(&h, Call::LockColorBuffer).len(), 2);
    assert!(positions(&h, Call::UnlockColorBuffer).is_empty());
    assert_eq!(positions(&h, Call::UnlockSurface).len(), 1);
    h.calls.clear();
    h.replies.push_back((Call::CreateColorBuffer, FAIL, vec![0]));
    assert_eq!(d.copy_color_buffer(0, 0, 1, 1, 0, &mut h), Ok(0));
    assert_eq!(positions(&h, Call::UnlockSurface).len(), 1);
    assert!(h.refs.is_empty());
    // A newly created destination retains its factory reference even on init failure.
    h.replies.extend([
        (Call::CreateColorBuffer, 0, vec![91]),
        (Call::ColorBufferInfo, 0, vec![0, 0, 0]),
    ]);
    h.failures.push_back((Call::InitColorBuffer, FAIL));
    assert_eq!(d.copy_color_buffer(0, 0, 2, 3, 0, &mut h), Ok(91));
    assert!(h.refs.is_empty());
}

#[test]
fn dx7_59_capture_clips_to_destination_origin_without_flip() {
    let (mut d, mut h) = fixture();
    d.color_probe = Some(true);
    h.write(0x9000, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
    h.write(0x9020, &[9, 10, 11, 12, 13, 14, 15, 16]).unwrap();
    h.write(0xa000, &[0xa5; 64]).unwrap();
    assert_eq!(d.copy_color_buffer(-1, 0, 3, 2, 90, &mut h), Ok(90));
    assert_eq!(bytes(&mut h, 0xa000, 8), [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(bytes(&mut h, 0xa010, 8), [9, 10, 11, 12, 13, 14, 15, 16]);
    // Recorder records Fill rather than emulating it; conversion leaves row tails alone.
    assert_eq!(bytes(&mut h, 0xa008, 8), [0xa5; 8]);
    let fill = h.calls.iter().find(|r| r.call == Call::FillColorBuffer).unwrap();
    assert_eq!(fill.args, [0xff000000, 0]);
    assert!(positions(&h, Call::InitColorBuffer).is_empty());
    let flags: Vec<_> = h
        .calls
        .iter()
        .filter(|r| r.call == Call::LockColorBuffer)
        .map(|r| r.args[0])
        .collect();
    assert_eq!(flags, [0x8001, 0x8080]);
}

#[test]
fn dx7_60_capture_pixel_formats_differ_from_texture_expansion() {
    for (bits, masks, input, expected) in [
        (16, [0xf800, 0x07e0, 0x001f, 0], vec![0, 0x20], vec![0, 0, 0x20, 255]),
        (16, [0x7c00, 0x03e0, 0x001f, 0x8000], vec![0, 0x10], vec![0, 0, 0x20, 0]),
        (16, [0x7c00, 0x03e0, 0x001f, 0], vec![0, 0x10], vec![0, 0, 0x20, 255]),
        (16, [0xf800, 0x07c0, 0x003e, 1], vec![1, 0x20], vec![0, 0, 0x20, 255]),
        (24, [0; 4], vec![0x12, 0x34, 0x56], vec![255, 0x12, 0x34, 0x56]),
        (32, [0; 4], vec![0x12, 0x34, 0x56, 0x78], vec![0x12, 0x34, 0x56, 0x78]),
    ] {
        let (mut d, mut h) = fixture();
        d.color_probe = Some(true);
        let mut desc = vec![0x9000, 32, 8, 8, 0x40, bits];
        desc.extend(masks);
        h.replies.push_back((Call::LockColorSurface, 0, desc));
        h.write(0x9000, &input).unwrap();
        assert_eq!(d.copy_color_buffer(0, 0, 1, 1, 90, &mut h), Ok(90));
        assert_eq!(bytes(&mut h, 0xa000, 4), expected);
    }
    for (flags, bits) in [(0, 32), (0x40, 8), (0x40, 16)] {
        let (mut d, mut h) = fixture();
        d.color_probe = Some(true);
        h.replies.push_back((
            Call::LockColorSurface,
            0,
            vec![0x9000, 32, 8, 8, flags, bits, 0, 0, 0, 0],
        ));
        h.write(0xa000, &[0xa5; 4]).unwrap();
        assert_eq!(d.copy_color_buffer(0, 0, 1, 1, 90, &mut h), Ok(90));
        assert_eq!(bytes(&mut h, 0xa000, 4), [0xa5; 4]);
        let fills: Vec<_> = h
            .calls
            .iter()
            .filter(|r| r.call == Call::FillColorBuffer)
            .map(|r| r.args[0])
            .collect();
        assert_eq!(
            fills,
            if bits == 16 {
                vec![0xff000000]
            } else {
                vec![0xff000000, 0xff00ffff]
            }
        );
        assert_eq!(positions(&h, Call::UnlockColorBuffer).len(), 2);
    }
}

#[test]
fn dx7_61_owner_surfaces_setup_loss_recovery_order() {
    let mut d = Driver::new(1, 2);
    let mut h = Recorder::default();
    d.window = 15;
    d.current.words[1] = 800;
    d.current.words[2] = 600;
    assert!(d.setup(0, &mut h).unwrap());
    let old = [d.device, d.context.depth, d.context.primary, d.context.target];
    assert_eq!(d.primary_surface, old[2]);
    assert_eq!(d.auxiliary_surfaces, [old[3], old[1], 0]);
    assert_eq!(d.new_region(2, &mut h), 0);
    for token in [old[1], old[2]] {
        assert!(h.refs.contains(&(token, Op::AddRef)));
    }
    assert!(positions(&h, Call::GetRenderTarget).len() == 1);
    h.refs.clear();
    h.failures.push_back((Call::CooperativeLevel, LOST));
    d.flush(&mut h).unwrap();
    let releases: Vec<_> = h
        .refs
        .iter()
        .filter(|(_, op)| *op == Op::Release)
        .map(|(token, _)| *token)
        .collect();
    let indices: Vec<_> = old
        .iter()
        .map(|token| releases.iter().position(|t| t == token).unwrap())
        .collect();
    assert!(indices.windows(2).all(|p| p[0] < p[1]));
    assert_eq!(d.primary_surface, 0);
    assert_eq!(d.auxiliary_surfaces, [0; 3]);
    h.calls.clear();
    h.refs.clear();
    d.flush(&mut h).unwrap();
    assert!(!d.lost);
    assert_eq!(d.primary_surface, d.context.primary);
    assert_eq!(d.auxiliary_surfaces, [d.context.target, d.context.depth, 0]);
    assert_ne!(d.primary_surface, old[2]);
    let depth = h.refs.iter().position(|r| *r == (d.context.depth, Op::AddRef)).unwrap();
    let primary = h
        .refs
        .iter()
        .position(|r| *r == (d.context.primary, Op::AddRef))
        .unwrap();
    assert!(depth < primary);
    assert_eq!(positions(&h, Call::GetRenderTarget).len(), 1);
    // Borrowed same-pointer reassignment does nothing; target is released/requeried.
    h.refs.clear();
    d.bind_owner_surfaces(&mut h);
    assert!(!h.refs.contains(&(d.primary_surface, Op::AddRef)));
    assert!(!h.refs.contains(&(d.context.depth, Op::AddRef)));
    assert!(h.refs.contains(&(d.context.target, Op::Release)));
}

#[test]
fn dx7_62_aliased_fallback_failed_recovery_and_cleanup() {
    let mut d = Driver::new(1, 2);
    let mut h = Recorder::default();
    d.window = 15;
    d.current.words[1] = 800;
    d.current.words[2] = 600;
    d.color_probe = Some(false);
    assert!(d.setup(1, &mut h).unwrap()); // mode back-buffer count defaults to zero
    assert_eq!(d.context.primary, d.context.target);
    assert_eq!(d.primary_surface, d.auxiliary_surfaces[0]);
    let surface = d.primary_surface;
    h.refs.clear();
    h.failures.push_back((Call::CooperativeLevel, LOST));
    d.flush(&mut h).unwrap();
    assert_eq!(h.refs.iter().filter(|r| **r == (surface, Op::Release)).count(), 2);
    h.failures.push_back((Call::CreateSurface, FAIL));
    d.flush(&mut h).unwrap();
    assert!(d.lost);
    assert_eq!(d.primary_surface, 0);
    assert_eq!(d.auxiliary_surfaces, [0; 3]);
    assert_eq!(d.color_probe, Some(false));
    // Full cleanup releases every externally populated slot independently.
    d.primary_surface = surface;
    d.auxiliary_surfaces = [surface, 41, 42];
    h.refs.clear();
    d.destroy_viewport(&mut h);
    assert_eq!(h.refs.iter().filter(|r| **r == (surface, Op::Release)).count(), 2);
    for token in [41, 42] {
        assert_eq!(h.refs.iter().filter(|r| **r == (token, Op::Release)).count(), 1);
    }
    assert_eq!(d.color_probe, Some(false));
}
