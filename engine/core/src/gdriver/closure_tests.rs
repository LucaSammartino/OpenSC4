// SPDX-License-Identifier: GPL-3.0-or-later
//! renderer-driver §15 DX7-19–49, with only recording foreign interfaces.
use super::*;

fn memory(h: &mut Recorder, address: u32, len: usize) -> Vec<u8> {
    let mut bytes = vec![0; len];
    h.read(address, &mut bytes).unwrap();
    bytes
}

#[test]
fn dx7_19_20_compact_writes_preserve_absent_uvs() {
    for (format, stride, color, uv0) in [(32, 12, false, false), (2, 20, false, true), (1, 16, true, false)] {
        let (mut d, mut h) = fixture();
        h.replies.push_back((Call::LockVertexBuffer, 0, vec![0xa000]));
        h.write(0xa000, &[0xa5; 32]).unwrap();
        let src: Vec<_> = (0..stride as u8).collect();
        h.write(0x4000, &src).unwrap();
        d.interleaved_arrays(format, stride, 0x4000);
        d.draw_arrays(0, 0, 1, &mut h).unwrap();
        let bytes = memory(&mut h, 0xa000, 32);
        assert_eq!(&bytes[..12], &src[..12]);
        assert_eq!(&bytes[12..16], if color { &src[12..16] } else { &[255; 4] });
        assert_eq!(&bytes[16..24], if uv0 { &src[12..20] } else { &[0xa5; 8] });
        assert_eq!(&bytes[24..], &[0xa5; 8]);
    }
}

#[test]
fn dx7_21_flush_returns_last_submission_and_resets_scratch_when_stopped() {
    for results in [[FAIL, 0], [0, FAIL]] {
        let (mut d, mut h) = fixture();
        d.draw_arrays(0, 0, 3, &mut h).unwrap();
        d.draw_arrays(3, 0, 1, &mut h).unwrap();
        for hr in results {
            h.replies.push_back((Call::DrawBuffer, hr, vec![]));
        }
        assert_eq!(d.primitives.flush(&mut h), results[1]);
        assert_eq!(positions(&h, Call::DrawBuffer).len(), 2);
        assert_eq!(d.primitives.flush(&mut h), 0);
    }
    let (mut d, mut h) = fixture();
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    d.primitives.started = false;
    d.primitives.scratch_used = 17;
    assert_eq!(d.primitives.flush(&mut h), FAIL);
    assert_eq!(d.primitives.scratch_used, 0);
    assert_eq!(d.primitives.entries.len(), 1);
}

#[test]
fn dx7_22_23_incomplete_quad_writes_and_declared_counts() {
    let input = [10, 11, 12, 13, 14, 15, 90, 91];
    for (count, declared) in [(5, 6), (6, 9)] {
        let (_, generated, n) = primitive::expanded_writes(6, &[], count, false).unwrap();
        assert_eq!(n, declared);
        assert_eq!(generated.unwrap(), [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
        let (_, generated, n) = primitive::expanded_writes(6, &input, count, true).unwrap();
        assert_eq!(n, declared);
        let expected = [10, 11, 12, 10, 12, 13, 14, 15, 90, 14, 90, 91];
        assert_eq!(generated.unwrap(), expected[..if count == 5 { 6 } else { 12 }]);
    }
    assert_eq!(
        primitive::expanded_writes(6, &input[..6], 6, true),
        Err(Error::InvalidInput)
    );
    let (_, generated, _) = primitive::expanded_writes(7, &[], 5, false).unwrap();
    assert_eq!(generated.unwrap(), [0, 1, 3, 0, 3, 2, 2, 3, 5, 2, 5, 4]);
}

#[test]
fn dx7_24_projection_extended_discriminator() {
    let (mut d, mut h) = fixture();
    h.viewport[2] = 30;
    d.matrix_mode = 1;
    let mut m = [0.; 16];
    m[0] = f32::from_bits(0x3e2f59ff);
    m[3] = f32::from_bits(0x40a4645f);
    d.load_matrix(m, &mut h);
    let call = h.calls.iter().find(|c| c.call == Call::Transform).unwrap();
    assert_eq!(u32::from_le_bytes(call.data[..4].try_into().unwrap()), 0x30888889);
}

#[test]
fn dx7_25_26_constructor_and_default_reset_are_distinct() {
    let d = Driver::new(1, 2);
    assert_eq!((d.clear_color, d.clear_depth, d.clear_stencil), (0, 1., 0));
    assert_eq!(d.material.map(f32::to_bits), [0; 17]);
    let s = Stage::default();
    assert!(!s.dirty && !s.enabled && s.alpha);
    assert_eq!((s.name, s.coordinates, s.environment, s.applied), (0, 0, 0, None));
    assert_eq!(s.parameters, [0, 0, 3, 3]);
    assert_eq!(s.function, [0; 2]);
    assert_eq!(s.sources, [[3; 3]; 2]);
    assert_eq!(s.operands, [[2; 3], [4; 3]]);
    assert_eq!(s.scale, [0; 2]);
    let (mut d, mut h) = fixture();
    d.stages[0].name = 27;
    d.stages[0].environment = 4;
    d.stages[0].applied = Some(4);
    d.stages[0].function = [2, 3];
    d.default_state(&mut h);
    let s = &d.stages[0];
    assert!(d.stages_dirty && s.dirty);
    assert_eq!((s.name, s.environment, s.applied, s.function), (27, 4, Some(4), [2, 3]));
    assert!(!positions(&h, Call::RenderState).is_empty());
}

#[test]
fn dx7_27_shutdown_does_not_assign_error_from_mode_list_end() {
    let mut d = Driver::new(1, 2);
    let mut h = Recorder::default();
    d.initialized = true;
    d.modes.push(Mode::default());
    d.error = 73;
    d.shutdown(&mut h).unwrap();
    assert!(d.modes.is_empty());
    assert_eq!(d.error, 73);
}

fn request() -> Request {
    Request {
        selector: u32::MAX,
        fullscreen: false,
        flip: 0,
        window: 1,
        width: 800,
        height: 600,
        back_buffers: 2,
    }
}

#[test]
fn dx7_28_device_ranking_sentinel_and_selectors() {
    use context::*;
    let a = DeviceDescriptor {
        guid: [1; 4],
        caps: 0x10000,
        render_bits: 16,
    };
    let b = DeviceDescriptor {
        guid: [2; 4],
        caps: 0x90000,
        render_bits: 16,
    };
    let c = DeviceDescriptor { guid: [3; 4], ..b };
    assert_eq!(select_device(u32::MAX, &[a, b, c]).unwrap(), b);
    let empty_sentinel = DeviceDescriptor { render_bits: 0, ..b };
    assert_eq!(select_device(u32::MAX, &[empty_sentinel, a]).unwrap(), a);
    let p = DeviceDescriptor { guid: PREFERRED, ..a };
    let f = DeviceDescriptor { guid: FALLBACK, ..b };
    assert_eq!(select_device((-4i32) as u32, &[f, p, f]).unwrap(), p);
    assert_eq!(select_device((-3i32) as u32, &[]).unwrap().guid, DIRECT);
    assert_eq!(select_device(0, &[]), Err(Error::InvalidInput));
}

#[test]
fn dx7_29_30_first_depth_format_and_terminal_failures() {
    let mut h = Recorder::default();
    let (mut c, hr) = Context::create(request(), &mut h).unwrap();
    assert_eq!(hr, 0);
    assert_eq!(positions(&h, Call::EnumerateDepthFormats).len(), 1);
    let depth = h
        .calls
        .iter()
        .find(|c| c.call == Call::CreateSurface && c.args[0] == 0x1007)
        .unwrap();
    assert_eq!(&depth.args[4..], &[32, 0, 0, 16, 0, 0, 0, 0]);
    c.destroy(&mut h);
    for (call, result) in [
        (Call::EnumerateDepthFormats, 0),
        (Call::AttachDepth, FAIL),
        (Call::Viewport, LOST),
    ] {
        let mut h = Recorder::default();
        h.replies.push_back((call, result, vec![]));
        let (c, hr) = Context::create(request(), &mut h).unwrap();
        assert_eq!(
            hr,
            if call == Call::EnumerateDepthFormats {
                FAIL
            } else {
                result
            }
        );
        assert_eq!(c.depth, 0);
        assert_eq!(positions(&h, Call::EnumerateDevices).len(), 2);
        if call == Call::AttachDepth {
            let depth = h.calls.iter().find(|c| c.call == Call::AttachDepth).unwrap().args[0];
            assert_eq!(h.refs.iter().filter(|r| **r == (depth, Op::Release)).count(), 1);
        }
    }
}

#[test]
fn dx7_31_final_reference_manager_ownership_without_submission() {
    let (mut d, mut h) = fixture();
    d.refs = 1;
    d.texture_identity = 0x300;
    d.primitives.refs = 2;
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    assert_eq!(d.release(&mut h), Ok(0));
    assert_eq!(d.primitives.refs, 1);
    assert_eq!(d.textures.refs, 2);
    assert!(h.refs.contains(&(0x200, Op::Release)));
    assert!(h.refs.contains(&(0x300, Op::AddRef)));
    assert!(!h.refs.contains(&(0x300, Op::Release)));
    assert!(positions(&h, Call::DrawBuffer).is_empty());
}

#[test]
fn dx7_32_38_auxiliary_lighting_and_material_vectors() {
    assert_eq!(D3DLight::default().range.to_bits(), 0x5f7fffff);
    let (mut d, mut h) = fixture();
    for v in [2, 2, 0x102, 0] {
        d.lighting_enable(v, &mut h);
    }
    let values: Vec<_> = h
        .calls
        .iter()
        .filter(|c| c.call == Call::RenderState && c.args[0] == 137)
        .map(|c| c.args[1])
        .collect();
    assert_eq!(values, [2, 0]);
    for _ in 0..2 {
        d.light_enable(17, 0x102, &mut h);
    }
    assert_eq!(positions(&h, Call::LightEnable).len(), 2);
    assert!(d.lights.is_empty());
    d.global_ambient([0., 0.5, 1.], &mut h);
    assert_eq!(h.states[&139], 0xff0080ff);
    d.global_ambient([2., 0., 0.], &mut h);
    assert_eq!(h.states[&139], 0xfffe0000);
    d.light_colors(2, [Some([1., 2., 3., 4.]), None, None], &mut h);
    d.light_property(2, 99, None, &mut h).unwrap();
    assert_eq!(d.lights.len(), 3);
    assert_eq!(d.lights[0], D3DLight::default());
    assert_eq!(d.lights[2].colors[0], [1., 2., 3., 4.]);
    assert_eq!(positions(&h, Call::Light).len(), 2);
    assert_eq!(h.calls.iter().find(|c| c.call == Call::Light).unwrap().data.len(), 104);
    d.material_property(4, Some(&[-7.]), &mut h).unwrap();
    d.material_colors([None; 4], -1., &mut h);
    assert_eq!(d.material[16], -7.);
    d.material_colors([None; 4], -0., &mut h);
    assert_eq!(d.material[16].to_bits(), 0x80000000);
    for (result, error) in [(0, 1), (LOST, 0)] {
        d.draw_arrays(0, 0, 3, &mut h).unwrap();
        h.replies.push_back((Call::DrawBuffer, FAIL, vec![]));
        h.replies.push_back((Call::Material, result, vec![]));
        d.material_colors([Some([0.25; 4]), None, None, None], f32::NAN, &mut h);
        assert_eq!(d.error, error);
        assert_eq!(&d.material[4..8], &[0.25; 4]);
    }
}

fn descriptor(bits: u32, alpha: u32, key: u32) -> texture::Format {
    texture::Format {
        words: [0, 0, 0, 0, 0, 0, bits, key],
        alpha_bits: alpha,
        size: 32,
    }
}

#[test]
fn dx7_39_40_ordinary_format_ranks_and_stable_ties() {
    let mut h = Recorder::default();
    let mut formats = vec![descriptor(15, 0, 1), descriptor(32, 8, 2), descriptor(24, 8, 3)];
    assert_eq!(texture::select_format(0, &formats, &mut h).unwrap().words[7], 3);
    formats.push(descriptor(16, 1, 4));
    assert_eq!(texture::select_format(0, &formats, &mut h).unwrap().words[7], 4);
    formats.push(descriptor(16, 0, 5));
    assert_eq!(texture::select_format(0, &formats, &mut h).unwrap().words[7], 5);
    let mut formats = Vec::new();
    for (i, (bits, alpha)) in [(16, 0), (32, 1), (24, 8), (32, 4), (16, 4)].into_iter().enumerate() {
        formats.push(descriptor(bits, alpha, i as u32));
        assert_eq!(texture::select_format(2, &formats, &mut h).unwrap().words[7], i as u32);
    }
    let mut duplicate = *formats.last().unwrap();
    duplicate.words[1] = 0xf000;
    formats.push(duplicate);
    assert_eq!(texture::select_format(2, &formats, &mut h).unwrap().words[1], 0);
}

#[test]
fn dx7_41_42_empty_formats_dxt5_fallback_and_all_availability_cells() {
    let mut h = Recorder::default();
    for request in 0..8 {
        assert_eq!(texture::select_format(request, &[], &mut h).unwrap().words, [0; 8]);
    }
    let (mut d, mut h) = fixture();
    let chosen = texture::select_format(7, &[descriptor(32, 8, 1)], &mut h).unwrap();
    d.textures.formats = Some([chosen; 8]);
    d.textures.converters = Some(std::array::from_fn(|s| [texture::converter_available(s, 1); 8]));
    let name = d.create_texture(7, 4, 4, 1, 0, &mut h).unwrap();
    assert_ne!(d.textures.surface(name), 0);
    let mut u = upload(name);
    u.format = 10;
    assert!(!d.textures.load(u, &mut h).unwrap());
    assert!(positions(&h, Call::LockSurface).is_empty());
    let keys = [0, 1, 3, 4, 7, 9, 11, 13];
    for source in 0..15 {
        let row = keys.map(|key| texture::converter_available(source, key));
        let expected = match source {
            0..=9 => "11111000",
            10 => "01011100",
            12 => "01011010",
            _ => "00000011",
        };
        assert_eq!(
            row.map(|v| if v { '1' } else { '0' }).iter().collect::<String>(),
            expected
        );
    }
}

#[test]
fn dx7_43_pixel_expansions_and_independent_padding() {
    let mut h = Recorder::default();
    for (source, word, pixel) in [(5, 0x1234u16, 0x11443322u32), (9, 0x0821, 0xff080408)] {
        for key in [1, 2] {
            h.write(0x1000, &word.to_le_bytes()).unwrap();
            h.write(0x1004, &word.to_le_bytes()).unwrap();
            h.write(0x2000, &[0xa5; 16]).unwrap();
            texture::convert(source, key, 0x1000, 0x2000, 4, 8, 1, 2, &mut h).unwrap();
            for address in [0x2000, 0x2008] {
                assert_eq!(memory(&mut h, address, 4), pixel.to_le_bytes());
            }
            for address in [0x2004, 0x200c] {
                assert_eq!(memory(&mut h, address, 4), [0xa5; 4]);
            }
        }
    }
}

#[test]
fn dx7_44_linear_copy_ignores_dimensions_and_nonzero_offset_copy_result() {
    let (mut d, mut h) = fixture();
    d.textures.formats = Some([descriptor(8, 0, 11); 8]);
    let name = d.create_texture(6, 4, 4, 1, 0, &mut h).unwrap();
    let mut u = upload(name);
    u.width = 1;
    u.height = 1;
    u.row_length = 99;
    u.format = 9;
    let src: Vec<u8> = (0..32).collect();
    h.write(u.pixels, &src).unwrap();
    for x in [0, 1] {
        u.x = x;
        h.write(0x8000, &[0xa5; 32]).unwrap();
        h.replies.push_back((Call::LockSurface, 0, vec![0x8000, 256, 8, 1, 32]));
        assert!(d.textures.load(u, &mut h).unwrap());
        assert_eq!(
            memory(&mut h, 0x8000, 32),
            if x == 0 { src.clone() } else { vec![0xa5; 32] }
        );
    }
    assert_eq!(positions(&h, Call::UnlockSurface).len(), 2);
}

#[test]
fn dx7_45_46_region_handles_retry_and_no_error_assignment() {
    let (mut d, mut h) = fixture();
    let refs = d.refs;
    h.replies.push_back((Call::CreateSurface, 0x8876017c, vec![]));
    assert_eq!(d.new_region(0, &mut h), 1);
    assert_eq!(positions(&h, Call::EvictManagedTextures).len(), 1);
    let creates: Vec<_> = h.calls.iter().filter(|c| c.call == Call::CreateSurface).collect();
    assert_eq!(creates[0].args, creates[1].args);
    assert_eq!(d.new_region(1, &mut h), 2);
    assert!(d.is_region(1).unwrap());
    assert!(d.delete_region(1, &mut h));
    assert!(!d.delete_region(1, &mut h));
    assert!(!d.is_region(1).unwrap());
    assert_eq!(d.new_region(2, &mut h), 1);
    assert_eq!(d.refs, refs);
    h.failures.push_back((Call::CreateSurface, FAIL));
    d.error = 77;
    assert_eq!(d.new_region(0, &mut h), 0);
    assert_eq!(d.error, 77);
}

#[test]
fn dx7_47_48_region_direction_retries_and_restore_guards() {
    let (mut d, mut h) = fixture();
    let handle = d.new_region(0, &mut h);
    let surface = d.regions[0].surface;
    for save in [true, false] {
        h.calls.clear();
        h.failures.push_back((Call::Blt, FAIL));
        assert!(d
            .transfer_region(save, [handle, 10, 20, 30, 40, 50, 60], &mut h)
            .unwrap());
        let blts: Vec<_> = h.calls.iter().filter(|c| c.call == Call::Blt).collect();
        assert_eq!(blts.len(), 2);
        assert_eq!(blts[0].target, if save { surface } else { 14 });
        assert_eq!(
            &blts[0].args[1..9],
            if save {
                &[10, 20, 40, 60, 50, 60, 80, 100]
            } else {
                &[50, 60, 80, 100, 10, 20, 40, 60]
            }
        );
        assert_eq!(blts[0].args[9], 0x01000200);
        assert_eq!(blts[1].args[9], 0x01000000);
    }
    h.calls.clear();
    d.auxiliary_surfaces[1] = 0;
    assert!(!d.transfer_region(false, [handle, 0, 0, 1, 1, 0, 0], &mut h).unwrap());
    assert!(positions(&h, Call::Blt).is_empty());
    d.auxiliary_surfaces[1] = 13;
    for _ in 0..2 {
        h.failures.push_back((Call::Blt, LOST));
    }
    assert!(!d.transfer_region(false, [handle, 0, 0, 1, 1, 0, 0], &mut h).unwrap());
    assert_eq!(positions(&h, Call::SurfaceLost).len(), 1);
    assert!(positions(&h, Call::RestoreSurface).is_empty());
}

#[test]
fn dx7_49_partial_save_clears_but_returns_false_after_recovery() {
    for (kind, full) in [(0, false), (1, false), (2, false), (0, true)] {
        let (mut d, mut h) = fixture();
        let handle = d.new_region(kind, &mut h);
        h.calls.clear();
        for _ in 0..2 {
            h.failures.push_back((Call::Blt, LOST));
        }
        let (w, height) = if full { (800, 600) } else { (30, 40) };
        assert_eq!(
            d.transfer_region(true, [handle, 0, 0, w, height, 0, 0], &mut h)
                .unwrap(),
            full
        );
        assert_eq!(positions(&h, Call::RestoreContext).len(), 1);
        assert_eq!(positions(&h, Call::PrimitiveRecovery).len(), 1);
        let clears: Vec<_> = h
            .calls
            .iter()
            .filter(|c| c.call == Call::Blt && c.data.len() == 100)
            .collect();
        assert_eq!(clears.len(), usize::from(!full && kind < 2));
        if let Some(clear) = clears.first() {
            assert_eq!(clear.args[2], if kind == 0 { 0x01000400 } else { 0x03000000 });
            assert_eq!(clear.data, [0; 100]);
        }
    }
}

#[test]
fn texture_initialization_uploads_white_pixel() {
    let mut h = Recorder::default();
    let mut manager = TextureManager::default();
    manager.init(11, 10, &mut h).unwrap();
    assert_eq!(memory(&mut h, 0x8000, 4), [255; 4]);
    assert!(manager.valid(0));
    assert_eq!((manager.records[0].width, manager.records[0].height), (1, 1));
    assert_eq!(positions(&h, Call::LockSurface).len(), 1);
    assert!(manager.converters.unwrap()[0][4]);
    assert_eq!(manager.formats.unwrap()[4].size, 32);
    assert!(h.refs.contains(&(11, Op::AddRef)) && h.refs.contains(&(10, Op::AddRef)));
    let calls = h.calls.len();
    manager.init(11, 10, &mut h).unwrap();
    assert_eq!(h.calls.len(), calls);
}

#[test]
fn manager_destructors_do_not_submit() {
    let (mut d, mut h) = fixture();
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    let buffer = d.primitives.buffer.token;
    d.primitives.destroy(&mut h);
    assert!(d.primitives.entries.is_empty());
    assert_eq!(d.primitives.scratch_capacity, 0);
    assert!(h.refs.contains(&(buffer, Op::Release)));
    assert!(positions(&h, Call::DrawBuffer).is_empty());
    d.create_texture(4, 8, 8, 1, 0, &mut h).unwrap();
    let surfaces: Vec<_> = d
        .textures
        .records
        .iter()
        .map(|r| r.surface)
        .filter(|s| *s != 0)
        .collect();
    d.textures.destroy(&mut h);
    assert!(d.textures.records.is_empty() && d.textures.freed.is_empty());
    assert!(d.textures.formats.is_none() && d.textures.converters.is_none());
    assert_eq!((d.textures.dd, d.textures.device), (0, 0));
    for token in surfaces {
        assert!(h.refs.contains(&(token, Op::Release)));
    }
    assert!(positions(&h, Call::LockSurface).is_empty());
}

#[test]
fn context_rebuild_releases_previous_surfaces() {
    let mut h = Recorder::default();
    let (mut c, hr) = Context::create(request(), &mut h).unwrap();
    assert_eq!(hr, 0);
    let old = [c.device, c.depth, c.target, c.primary, c.clipper];
    h.calls.clear();
    h.refs.clear();
    assert_eq!(c.restore(&mut h), Ok(0));
    for token in old {
        assert_eq!(h.refs.iter().filter(|r| **r == (token, Op::Release)).count(), 1);
    }
    assert_ne!(old[0], c.device);
    assert_eq!(h.calls[0].call, Call::RestoreAll);
    assert_eq!(positions(&h, Call::LoadLibrary).len(), 0);
    assert!(c.active);
    // Context setup ignores clipper setter HRESULTs, but normalizes positive viewport results.
    h.failures.push_back((Call::ClipperWindow, FAIL));
    h.failures.push_back((Call::AttachClipper, FAIL));
    h.replies.push_back((Call::Viewport, 1, vec![]));
    assert_eq!(c.rebuild(request(), &mut h), Ok(0));
}

#[test]
fn adapter_version_search_and_shared_information() {
    let _guard = INFO_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut d = Driver::new(1, 2);
    let mut h = Recorder::default();
    h.write(0x7000, b"Test").unwrap();
    h.replies
        .push_back((Call::AdapterInfo, 0, vec![0x7000, 4, 0, 0, 0, 0, 0x7100, 1, 2, 3, 4]));
    for _ in 0..5 {
        h.replies.push_back((Call::FindVersionFile, 0, vec![0]));
    }
    h.replies.push_back((Call::FindVersionFile, 0, vec![0x9000]));
    h.replies
        .push_back((Call::FileVersion, 0, vec![0x00010002, 0x00030004]));
    d.init(&mut h).unwrap();
    assert!(d.driver_info().windows(8).any(|w| w == b"1.2.3.4,"));
    let searches: Vec<_> = h
        .calls
        .iter()
        .filter(|c| c.call == Call::FindVersionFile)
        .map(|c| c.data.clone())
        .collect();
    assert_eq!(
        searches,
        [
            b"\\\0".to_vec(),
            b"\\\0.dll".to_vec(),
            b"\\\0.vxd".to_vec(),
            b"\\\0.sys".to_vec(),
            b"\\Drivers\\\0".to_vec(),
            b"\\Drivers\\\0.dll".to_vec()
        ]
    );
    let second = Driver::new(3, 4);
    let info = d.driver_info().clone();
    assert_eq!(*second.driver_info(), info);
    // A matched file with unreadable version resource terminates the search.
    let mut d = Driver::new(5, 6);
    let mut h = Recorder::default();
    h.replies
        .push_back((Call::AdapterInfo, 0, vec![0x7000, 0, 0, 0, 0, 0, 0x7100, 0, 0, 0, 0]));
    h.replies.push_back((Call::FindVersionFile, 0, vec![0x9000]));
    h.replies.push_back((Call::FileVersion, FAIL, vec![]));
    d.init(&mut h).unwrap();
    assert_eq!(positions(&h, Call::FindVersionFile).len(), 1);
    assert!(d.driver_info().windows(8).any(|w| w == b"0.0.0.0,"));
    assert_ne!(*second.driver_info(), info);
}

#[test]
fn auxiliary_noops_unchecked_properties_and_nonzero_results() {
    let (mut d, mut h) = fixture();
    d.error = 91;
    assert!(d.auxiliary_constant());
    d.lighting_noop(0, 0);
    assert!(h.calls.is_empty());
    assert_eq!(d.error, 91);
    d.light_property(1, 2, Some([f32::from_bits(0x7fc01234); 4]), &mut h)
        .unwrap();
    assert_eq!(d.lights[1].colors[2].map(f32::to_bits), [0x7fc01234; 4]);
    d.material_property(1, Some(&[2., 3., 4., 5.]), &mut h).unwrap();
    assert_eq!(&d.material[..4], &[2., 3., 4., 5.]);
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    h.replies.push_back((Call::DrawBuffer, 1, vec![]));
    d.light_enable(999, 0x102, &mut h);
    assert_eq!(d.error, 1);
    assert_eq!(d.lights.len(), 2);
    h.replies.push_back((Call::Light, LOST, vec![]));
    d.light_property(1, 99, None, &mut h).unwrap();
    assert_eq!(d.error, 0);
}
