// SPDX-License-Identifier: GPL-3.0-or-later
//! Recording foreign-device conformance vectors from renderer-driver §15.
use super::*;
use crate::gdrivers_director::{Allocation, Host, Op, Result as HostResult};
use std::collections::{BTreeMap, VecDeque};
static INFO_TEST_LOCK: Mutex<()> = Mutex::new(());
#[derive(Debug, Clone)]
struct Record {
    target: u32,
    call: Call,
    args: Vec<u32>,
    data: Vec<u8>,
}
struct Recorder {
    calls: Vec<Record>,
    refs: Vec<(u32, Op)>,
    failures: VecDeque<(Call, u32)>,
    memory: BTreeMap<u32, u8>,
    next: u32,
    viewport: [u32; 6],
    states: BTreeMap<u32, u32>,
    rect: [u32; 4],
    modes: Vec<[u32; 3]>,
    replies: VecDeque<(Call, u32, Vec<u32>)>,
}
impl Default for Recorder {
    fn default() -> Self {
        Self {
            calls: Vec::new(),
            refs: Vec::new(),
            failures: VecDeque::new(),
            memory: BTreeMap::new(),
            next: 0x100000,
            viewport: [0, 0, 800, 600, 0, 1f32.to_bits()],
            states: BTreeMap::new(),
            rect: [0, 0, 800, 600],
            modes: vec![[640, 480, 16], [800, 600, 32]],
            replies: VecDeque::new(),
        }
    }
}
impl Host for Recorder {
    fn alloc(&mut self, _bytes: u32, _kind: Allocation) -> u32 {
        self.next += 0x10000;
        self.next
    }
    fn free(&mut self, _address: u32, _kind: Allocation) {}
    fn read(&mut self, address: u32, bytes: &mut [u8]) -> HostResult<()> {
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = *self.memory.get(&(address + i as u32)).unwrap_or(&0);
        }
        Ok(())
    }
    fn write(&mut self, address: u32, bytes: &[u8]) -> HostResult<()> {
        for (i, b) in bytes.iter().enumerate() {
            self.memory.insert(address + i as u32, *b);
        }
        Ok(())
    }
    fn call(&mut self, target: u32, op: Op, _args: &[u32], _data: &[u8], _out: &mut [u32]) -> u32 {
        self.refs.push((target, op));
        0
    }
    fn enumerate_child(&mut self, _target: u32, _context: u32, _callback: &mut dyn FnMut(u32, u32, u32) -> bool) {}
}
impl Device for Recorder {
    fn dx(&mut self, target: u32, call: Call, args: &[u32], data: &[u8], out: &mut [u32]) -> u32 {
        self.calls.push(Record {
            target,
            call,
            args: args.to_vec(),
            data: data.to_vec(),
        });
        if self.replies.front().is_some_and(|(op, _, _)| *op == call) {
            let (_, hr, words) = self.replies.pop_front().unwrap();
            for (o, value) in out.iter_mut().zip(words) { *o = value; }
            return hr;
        }
        if self.failures.front().is_some_and(|(op, _)| *op == call) {
            return self.failures.pop_front().unwrap().1;
        }
        match call {
            Call::GetViewport => out.copy_from_slice(&self.viewport),
            Call::Viewport => self.viewport.copy_from_slice(args),
            Call::RenderState => {
                self.states.insert(args[0], args[1]);
            }
            Call::GetRenderState => out[0] = *self.states.get(&args[0]).unwrap_or(&0),
            Call::GetCaps => {
                for (o, v) in out.iter_mut().zip([0x80000, 1, 2, 0x80, 1, 16]) {
                    *o = v;
                }
            }
            Call::PixelFormat => {
                let values = if out.len() == 4 { vec![0xff000000, 0xff0000, 0xff00, 0xff] }
                    else { vec![32, 0x41, 0, 32, 0xff0000, 0xff00, 0xff, 0xff000000] };
                out.copy_from_slice(&values);
            }
            Call::EnumerateDevices => {
                if args.is_empty() { out[0] = 1; }
                else { out.copy_from_slice(&[1, 2, 3, 4, 0x90000, 16]); }
            }
            Call::EnumerateDepthFormats => out.copy_from_slice(&[32, 0, 0, 16, 0, 0, 0, 0]),
            Call::CurrentDisplayMode => out[0] = 32,
            Call::TextureFormats => {
                if args.is_empty() { out[0] = 1; }
                else { out.copy_from_slice(&[0xff000000, 0xff0000, 0xff00, 0xff, 0, 0, 32, 1, 8, 32]); }
            }
            Call::SurfaceDescriptor => out.copy_from_slice(&[800, 600, 0x4000]),
            Call::RestoreContext => { for o in out { self.next += 0x10000; *o = self.next; } }
            Call::FindVersionFile => out[0] = 0,
            Call::ClientRect => out.copy_from_slice(&self.rect),
            Call::ClientToScreen => {
                out[0] = args[0] + 50;
                out[1] = args[1] + 70;
            }
            Call::EnumerateModes => {
                if args.is_empty() {
                    out[0] = self.modes.len() as u32;
                } else {
                    out.copy_from_slice(&self.modes[args[0] as usize]);
                }
            }
            Call::LockSurface => out.copy_from_slice(&[0x8000, 256, 32, 0, 0]),
            Call::AdapterInfo => out.copy_from_slice(&[0x7000, 4, 1, 2, 3, 4, 0x7100, 1, 2, 3, 4]),
            _ => {
                if !out.is_empty() {
                    self.next += 0x10000;
                    out[0] = self.next;
                }
            }
        }
        0
    }
    fn unresolved(&mut self, question: &'static str, args: &[u32], out: &mut [u32]) -> Result<()> {
        // Explicit test inputs at open proof boundaries, never claimed as native defaults.
        match question {
            "RD6" => {
                if args.len() == 1 {
                    for o in out {
                        *o = 1;
                    }
                }
            }
            _ => return Err(Error::Unsupported(question)),
        }
        Ok(())
    }
}
fn fixture() -> (Driver, Recorder) {
    let mut h = Recorder::default();
    let mut d = Driver::new(0x100, 0x200);
    d.device = 10;
    d.dd = 11;
    d.d3d = 12;
    d.auxiliary_surfaces = [14, 13, 16];
    d.current.words[1] = 800;
    d.current.words[2] = 600;
    d.context.dd = 11;
    d.context.d3d = 12;
    d.context.device = 10;
    d.context.target = 13;
    d.context.primary = 14;
    d.context.window = 15;
    d.context.active = true;
    d.ready = true;
    d.window = 15;
    d.current.words[0] = 0;
    d.current.set_byte(0x21, 1);
    d.current.set_byte(0x29, 1);
    d.full_viewport = h.viewport;
    d.stages = vec![
        Stage {
            parameters: [0, 0, 3, 3],
            ..Stage::default()
        };
        2
    ];
    d.textures.dd = 11;
    d.textures.device = 10;
    d.textures.formats = Some(
        [texture::Format {
            words: [0, 0, 0, 0, 0, 0, 32, 0],
            alpha_bits: 8,
            size: 32,
        }; 8],
    );
    d.textures.converters = Some([[true; 8]; 15]);
    d.textures.new_name();
    d.primitives.init(11, 12, 10, &mut h);
    d.interleaved_arrays(11, 32, 0x4000);
    h.calls.clear();
    (d, h)
}
fn upload(name: u32) -> Upload {
    Upload {
        name,
        level: 0,
        x: 0,
        y: 0,
        width: 2,
        height: 2,
        format: 3,
        kind: 0,
        row_length: 0,
        pixels: 0x5000,
    }
}
fn positions(h: &Recorder, call: Call) -> Vec<usize> {
    h.calls
        .iter()
        .enumerate()
        .filter_map(|(i, c)| (c.call == call).then_some(i))
        .collect()
}

#[test]
fn dx7_01_02_04_noops_and_identity() {
    let mut h = Recorder::default();
    let mut d = Driver::new(0x100, 0x200);
    let mut b = true;
    let mut f = 3.25;
    d.error = 6;
    d.get_boolean(0, &mut b);
    d.get_float(0, &mut f);
    assert!(b);
    assert_eq!(f, 3.25);
    assert_eq!(d.tail(11, 22), 0);
    assert_eq!(d.error, 6);
    let mut out = 0xfeed;
    assert!(!d.query_interface(42, Some(&mut out)).unwrap());
    assert_eq!(out, 0xfeed);
    assert_eq!(d.refs, 0);
    assert!(d.query_interface(0xa4554849, Some(&mut out)).unwrap());
    assert_eq!(out, 0x10c);
    assert_eq!(d.refs, 1);
    assert!(d.query_interface(0x09cd86f9, Some(&mut out)).unwrap());
    assert_eq!(d.refs, 1);
    assert_eq!(d.primitives.refs, 2);
    for (iid, token) in [
        (1, 0x10c),
        (0x669565fe, 0x100),
        (0xe69bfe2a, 0x104),
        (0x87e2b87d, 0x108),
    ] {
        assert!(d.query_interface(iid, Some(&mut out)).unwrap());
        assert_eq!(out, token);
    }
    assert_eq!(d.refs, 5);
    assert_eq!(d.release(&mut h), Ok(4));
    assert_eq!(d.add_ref(), 5);
    assert_eq!(d.class_id(), crate::gdrivers_director::FOREIGN_DX7_CLASS);
    assert_eq!(
        d.query_interface(1, None),
        Err(Error::Foreign(crate::gdrivers_director::Fault::NullWrite))
    );
    assert_eq!(d.refs, 5);
}
#[test]
fn dx7_03_09_11_loss_restore_metadata_and_defaults() {
    let (mut d, mut h) = fixture();
    assert!(d.is_device_ready());
    let n = d.create_texture(4, 8, 8, 2, 0, &mut h).unwrap();
    d.state(16, &[7], &mut h).unwrap();
    assert_eq!(h.states[&23], 8);
    h.failures.push_back((Call::CooperativeLevel, FAIL));
    d.flush(&mut h).unwrap();
    assert!(!d.is_device_ready());
    assert!(d.is_texture(n));
    assert_eq!(d.textures.surface(n), 0);
    let mut resident = [false];
    assert!(d.resident(&[n], &mut resident));
    assert!(resident[0]);
    d.flush(&mut h).unwrap();
    assert!(d.is_device_ready());
    assert_ne!(d.textures.surface(n), 0);
    assert_eq!(h.states[&23], 4);
    let messages: Vec<_> = h
        .calls
        .iter()
        .filter(|c| c.call == Call::PostMessage)
        .map(|c| c.args[0])
        .collect();
    assert_eq!(messages, [0xaacb92a3, 0x8acaa95f, 0x6acb92a9]);
}
#[test]
fn dx7_05_deferred_binding_and_presentation_order() {
    let (mut d, mut h) = fixture();
    d.scene = true;
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    let n = d.create_texture(4, 2, 2, 1, 0, &mut h).unwrap();
    d.bind_texture(0, n).unwrap();
    d.capability(5, true, &mut h).unwrap();
    assert!(positions(&h, Call::DrawBuffer).is_empty());
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    d.flush(&mut h).unwrap();
    let draws = positions(&h, Call::DrawBuffer);
    let texture = positions(&h, Call::Texture)[0];
    let end = positions(&h, Call::EndScene)[0];
    let present = positions(&h, Call::Blt)[0];
    assert!(draws[0] < texture && texture < draws[1] && draws[1] < end && end < present);
}
#[test]
fn dx7_06_combiner_cache_and_min_filter_fallthrough() {
    let (mut d, mut h) = fixture();
    let n = d.create_texture(4, 2, 2, 1, 0, &mut h).unwrap();
    d.bind_texture(0, n).unwrap();
    d.capability(5, true, &mut h).unwrap();
    let a = [0, 0, 1, 2, 0, 0, 1, 2, 0, 0];
    d.set_combiner(&a, 0, &mut h).unwrap();
    d.tex_parameter(1, 5).unwrap();
    d.draw_arrays(0, 0, 0, &mut h).unwrap();
    let filters: Vec<_> = h
        .calls
        .iter()
        .filter(|c| c.call == Call::StageState && c.args[0] == 0 && matches!(c.args[1], 17 | 18))
        .map(|c| (c.args[1], c.args[2]))
        .collect();
    assert_eq!(filters, [(17, 2), (18, 2), (17, 1), (18, 3)]);
    h.calls.clear();
    let mut b = a;
    b[0] = 2;
    d.set_combiner(&b, 0, &mut h).unwrap();
    d.draw_arrays(0, 0, 0, &mut h).unwrap();
    assert!(!h.calls.iter().any(|c| c.call == Call::StageState && c.args[1] == 1));
    d.combine_field(57, 0, 2).unwrap();
    d.draw_arrays(0, 0, 0, &mut h).unwrap();
    assert!(h
        .calls
        .iter()
        .any(|c| c.call == Call::StageState && c.args == [0, 1, 7]));
}
#[test]
fn dx7_07_08_follow_parameters_and_lifo() {
    let (mut d, mut h) = fixture();
    let names = d.gen_textures(2);
    let (a, b) = (names[0], names[1]);
    d.bind_texture(0, a).unwrap();
    d.tex_parameter(0, 1).unwrap();
    assert_eq!(d.textures.get(a).unwrap().parameters[0], 1);
    d.set_texture(a, 0).unwrap();
    assert!(d.follow_texture);
    d.set_texture(b, 0).unwrap();
    d.tex_parameter(0, 7).unwrap();
    assert_eq!(d.textures.get(b).unwrap().parameters[0], 0);
    d.textures.priority(a, 0.95);
    d.textures.create(a, 3, 2, 7, 9, 5, &mut h).unwrap();
    d.delete_textures(&[a, b, 0], &mut h);
    assert!(d.is_texture(0));
    assert_eq!(d.gen_textures(2), [b, a]);
    let r = d.textures.get(a).unwrap();
    assert_eq!((r.width, r.height, r.levels, r.format), (0, 0, 5, 2));
    assert_eq!(r.priority, 0.95);
    assert_eq!(r.parameters[0], 1);
}
#[test]
fn dx7_10_texture_upload_flush_boundary() {
    let (mut d, mut h) = fixture();
    let n = d.create_texture(4, 2, 2, 1, 0, &mut h).unwrap();
    d.bind_texture(0, n).unwrap();
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    h.calls.clear();
    d.tex_image(upload(n), 4, &mut h).unwrap();
    assert!(positions(&h, Call::DrawBuffer).is_empty());
    assert_eq!(positions(&h, Call::LockSurface).len(), 1);
    h.calls.clear();
    d.load_texture_level(upload(n), &mut h).unwrap();
    assert!(positions(&h, Call::DrawBuffer)[0] < positions(&h, Call::LockSurface)[0]);
}
#[test]
fn dx7_12_flip_fallback() {
    let mut h = Recorder::default();
    h.failures
        .extend([(Call::CreateSurface, FAIL), (Call::CreateSurface, FAIL)]);
    let (mut c, hr) = Context::create(
        Request {
            selector: u32::MAX,
            fullscreen: true,
            flip: 1,
            window: 20,
            width: 800,
            height: 600,
            back_buffers: 2,
        },
        &mut h,
    )
    .unwrap();
    assert_eq!(hr, 0);
    let counts: Vec<_> = h
        .calls
        .iter()
        .filter(|c| c.call == Call::CreateSurface && matches!(c.args[0], 1 | 0x21))
        .map(|c| c.args[2])
        .collect();
    assert_eq!(counts, [2, 1, 0]);
    assert!(!c.flipping);
    c.present(0, &mut h);
    assert!(positions(&h, Call::Flip).is_empty());
    c.destroy(&mut h);
}
#[test]
fn dx7_13_14_present_retry_and_empty_rect() {
    let (d, mut h) = fixture();
    h.failures.push_back((Call::Blt, FAIL));
    assert_eq!(d.context.present(0, &mut h), 0);
    let blits: Vec<_> = h.calls.iter().filter(|c| c.call == Call::Blt).collect();
    assert_eq!(blits.len(), 2);
    assert_eq!(&blits[0].args[1..5], &[50, 70, 850, 670]);
    assert_eq!(blits[0].args[5], 0);
    assert_eq!(blits[0].args[6], 0x01000200);
    assert_eq!(blits[1].args[6], 0x01000000);
    assert_eq!(blits[0].target, 14);
    h.calls.clear();
    h.rect[2] = 0;
    assert_eq!(d.context.present(0, &mut h), LOST);
    assert!(positions(&h, Call::Blt).is_empty());
}
#[test]
fn dx7_15_modes_and_descriptor_enrichment() {
    let mut d = Driver::new(0x100, 0x200);
    let mut h = Recorder::default();
    assert_eq!(d.count_modes(&mut h), 4);
    assert_eq!(d.modes.iter().map(|m| m.byte(0x20)).collect::<Vec<_>>(), [1, 0, 1, 0]);
    let mut before = [0; 13];
    d.mode_info(0, &mut before);
    assert_eq!(&before[4..8], &[0; 4]);
    d.textures.formats = Some([texture::Format { words: [0, 0, 0, 0, 0, 0, 32, 1], alpha_bits: 8, size: 32 }; 8]);
    d.textures.converters = Some([[true; 8]; 15]);
    d.set_video_mode(0, 0, false, 0x9000, &mut h).unwrap();
    let mut after = [0; 13];
    d.mode_info(0, &mut after);
    assert_ne!(&after[4..8], &[0; 4]);
    assert_eq!(d.current.byte(0x21), 1);
    assert_eq!(d.stages.len(), 2);
    assert_eq!(after.len() * 4, 52);
}
#[test]
fn dx7_16_buffer_reservation_and_reference_order() {
    let mut h = Recorder::default();
    let mut b = primitive::VertexBuffer {
        format: 11,
        stride: 32,
        flags: 0x10000,
        ..Default::default()
    };
    assert_eq!(b.recreate(4, 11, 12, &mut h), 0);
    assert!(b.reserve(3, false, 11, 12, &mut h).is_some());
    assert!(b.reserve(1, false, 11, 12, &mut h).is_none());
    b.unlock(11, 12, &mut h);
    assert!(b.reserve(2, false, 11, 12, &mut h).is_none());
    assert!(b.reserve(2, true, 11, 12, &mut h).is_some());
    let locks: Vec<_> = h
        .calls
        .iter()
        .filter(|c| c.call == Call::LockVertexBuffer)
        .map(|c| c.args[0])
        .collect();
    assert_eq!(locks, [0x1021, 0x2021]);
    let mut a = primitive::VertexBuffer {
        token: 123,
        ..Default::default()
    };
    h.refs.clear();
    a.assign(&b, &mut h);
    assert_eq!(h.refs, [(b.token, Op::AddRef), (123, Op::Release)]);
}
#[test]
fn dx7_17_sticky_hresult_channel() {
    let (mut d, mut h) = fixture();
    for hr in [LOST, FAIL] {
        h.failures.push_back((Call::RenderState, hr));
        d.state(67, &[1], &mut h).unwrap();
        assert_eq!(d.get_error(), u32::from(hr != LOST));
        assert_eq!(d.get_error(), 0);
    }
    d.error = 6;
    d.capability(0, false, &mut h).unwrap();
    assert_eq!(d.error, 0);
}
#[test]
fn enable_preserves_error_across_failed_queued_draw_and_device_state() {
    for cap in [0, 1, 2, 3, 4, 6] {
        for draw_hr in [FAIL, LOST] {
            for state_hr in [0, FAIL, LOST] {
                let (mut d, mut h) = fixture();
                d.fog_capable = true;
                d.draw_arrays(0, 0, 3, &mut h).unwrap();
                assert_eq!(d.primitives.entries.len(), 1);
                d.error = 6;
                h.failures.push_back((Call::DrawBuffer, draw_hr));
                h.failures.push_back((Call::RenderState, state_hr));
                d.capability(cap, true, &mut h).unwrap();
                assert_eq!(d.error, 6, "cap={cap}, draw={draw_hr:x}, state={state_hr:x}");
                assert!(d.primitives.entries.is_empty());
                assert!(h.failures.is_empty());
                assert!(positions(&h, Call::DrawBuffer)[0] < positions(&h, Call::RenderState)[0]);
            }
        }
    }
}

#[test]
fn disable_replaces_error_with_state_result_after_failed_queued_draw() {
    for cap in 0..5 {
        for draw_hr in [FAIL, LOST] {
            for state_hr in [0, FAIL, LOST] {
                let (mut d, mut h) = fixture();
                d.draw_arrays(0, 0, 3, &mut h).unwrap();
                d.error = 6;
                h.failures.push_back((Call::DrawBuffer, draw_hr));
                h.failures.push_back((Call::RenderState, state_hr));
                d.capability(cap, false, &mut h).unwrap();
                assert_eq!(d.get_error(), u32::from(state_hr == FAIL));
                assert!(h.failures.is_empty());
                assert!(d.primitives.entries.is_empty());
            }
        }
    }
}

#[test]
fn capability_deferred_and_ignored_paths_preserve_queue_and_error() {
    let (mut d, mut h) = fixture();
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    h.calls.clear();
    d.error = 6;
    for enabled in [true, false] {
        for cap in [5, 6, 7] {
            d.capability(cap, enabled, &mut h).unwrap();
            assert_eq!(d.error, 6);
            assert_eq!(d.primitives.entries.len(), 1);
            assert!(h.calls.is_empty());
        }
        assert_eq!(d.stages[0].enabled, enabled);
        assert!(d.stages[0].dirty);
        assert!(d.stages_dirty);
    }
    d.lost = true;
    for cap in 0..8 {
        assert!(!d.is_enabled(cap, &mut h).unwrap());
    }
    assert!(h.calls.is_empty());
}

#[test]
fn dx7_18_failed_setup_writes_fourth_word_after_cleanup() {
    let mut h = Recorder::default();
    let mut d = Driver::new(0x100, 0x200);
    d.count_modes(&mut h);
    h.failures.push_back((Call::LoadLibrary, FAIL));
    d.set_video_mode(0, 0, false, 0x9000, &mut h).unwrap();
    assert_eq!(d.error, 6);
    assert_eq!(d.window, 0);
    let mut bytes = [0; 4];
    h.read(0x9000, &mut bytes).unwrap();
    assert_eq!(u32::from_le_bytes(bytes), u32::MAX);
    assert_eq!(positions(&h, Call::DestroyWindow).len(), 1);
}
#[test]
fn matrices_viewport_formats_and_draw_patterns() {
    assert_eq!(vertex::make(&[0, 5, 7]).unwrap(), 0x80004101);
    assert_eq!(vertex::stride(11), 32);
    assert_eq!(vertex::elements(0x80008101, 7).unwrap(), 2);
    assert_eq!(vertex::offset(11, 7, 1).unwrap(), 24);
    assert_eq!(
        primitive::expanded(1, &[0, 1, 2, 3]).unwrap().1.unwrap(),
        [0, 1, 2, 2, 1, 3]
    );
    assert_eq!(
        primitive::expanded(6, &[0, 1, 2, 3]).unwrap().1.unwrap(),
        [0, 1, 2, 0, 2, 3]
    );
    let (mut d, mut h) = fixture();
    d.matrix_mode = 1;
    d.load_identity(&mut h);
    let m = &h.calls.iter().find(|c| c.call == Call::Transform).unwrap().data;
    let values: Vec<_> = m.chunks(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect();
    assert_eq!(values[12], -1. / 800.);
    assert_eq!(values[13], 1. / 600.);
    assert_eq!(values[14], 0.5);
    assert_eq!(values[10], -0.5);
    d.set_viewport(Some([100, 50, 200, 100]), &mut h);
    assert_eq!(&h.viewport[..4], &[100, 450, 200, 100]);
    assert_eq!(d.get_viewport(&mut h), [100, 50, 300, 150]);
    d.lost = true;
    assert_eq!(d.get_viewport(&mut h), [0; 4]);
}
#[test]
fn matrix_modes_unchecked_failures_viewport_reset_and_unsupported_blits() {
    let (mut d, mut h) = fixture();
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    d.error = 6;
    h.calls.clear();
    d.matrix_mode(0);
    assert!(h.calls.is_empty());
    assert_eq!(d.primitives.entries.len(), 1);
    h.failures.push_back((Call::DrawBuffer, FAIL));
    h.failures.push_back((Call::Transform, FAIL));
    d.load_identity(&mut h);
    assert_eq!(d.error, 6);
    assert!(h.failures.is_empty());
    assert!(positions(&h, Call::DrawBuffer)[0] < positions(&h, Call::Transform)[0]);
    let transforms: Vec<_> = h.calls.iter().filter(|c| c.call == Call::Transform).collect();
    assert_eq!(transforms.len(), 2);
    assert_eq!(transforms[0].args, [1]);
    assert_eq!(transforms[0].data, floats(&IDENTITY));
    let mut view = IDENTITY;
    view[10] = -1.;
    assert_eq!(transforms[1].args, [2]);
    assert_eq!(transforms[1].data, floats(&view));
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    h.calls.clear();
    d.matrix_mode(99);
    assert!(h.calls.is_empty());
    d.load_identity(&mut h);
    assert_eq!(positions(&h, Call::DrawBuffer).len(), 1);
    assert!(positions(&h, Call::Transform).is_empty());
    assert_eq!(d.error, 6);
    d.projection = IDENTITY;
    d.set_viewport(Some([900, 700, 10, 20]), &mut h);
    assert_eq!(&h.viewport[..4], &[800, 0, 0, 0]);
    assert_eq!(d.matrix_mode, 99);
    assert_eq!(d.projection, IDENTITY);
    d.set_viewport(None, &mut h);
    assert_eq!(h.viewport, d.full_viewport);
    assert_eq!(d.matrix_mode, 99);
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    h.calls.clear();
    d.blit_unsupported();
    assert_eq!(d.error, 3);
    assert_eq!(d.primitives.entries.len(), 1);
    assert!(h.calls.is_empty());
}

#[test]
fn defaults_clear_quantization_and_stencil_failures() {
    let (mut d, mut h) = fixture();
    d.default_state(&mut h);
    let states: Vec<_> = h
        .calls
        .iter()
        .filter(|c| c.call == Call::RenderState)
        .map(|c| (c.args[0], c.args[1]))
        .collect();
    assert_eq!(states.len(), 26);
    assert_eq!(
        &states[12..19],
        &[(53, 1), (54, 6), (55, 1), (56, 8), (57, 1), (58, 1), (59, 1)]
    );
    assert_eq!(color([1., 0.5, 0., 1.]), 0xffff8000);
    d.current.set_byte(0x21, 0);
    d.clear(0x7000, &mut h);
    assert_eq!(h.calls.last().unwrap().args[0], 3);
    let count = h.calls.len();
    d.state(18, &[0, 1, 1], &mut h).unwrap();
    assert_eq!(d.error, 3);
    assert_eq!(h.calls.len(), count);
}

#[test]
fn immediate_states_capabilities_fog_and_multipliers() {
    let (mut d, mut h) = fixture();
    d.fog_capable = true;
    for (cap, state) in [(0, 15), (1, 7), (2, 52), (3, 22), (4, 27), (6, 28)] {
        d.capability(cap, true, &mut h).unwrap();
        assert_eq!(h.states[&state], if cap == 3 { 2 } else { 1 });
        assert_eq!(d.is_enabled(cap, &mut h).unwrap(), cap != 6);
        d.capability(cap, false, &mut h).unwrap();
        assert!(!d.is_enabled(cap, &mut h).unwrap());
    }
    assert!(d.is_enabled(7, &mut h).unwrap());
    d.lost = true;
    assert!(!d.is_enabled(7, &mut h).unwrap());
    d.lost = false;
    for (slot, args, states) in [
        (16, vec![6], vec![(23, 7)]),
        (17, vec![0x123], vec![(14, 0x23)]),
        (18, vec![3, 99, 0xff], vec![(58, 0xff), (56, 4), (57, 99)]),
        (19, vec![0xabc], vec![(59, 0xabc)]),
        (20, vec![0, 1, 5], vec![(53, 2), (54, 1), (55, 6)]),
        (21, vec![4, 5], vec![(19, 5), (20, 6)]),
        (22, vec![7, 0.5f32.to_bits()], vec![(25, 8), (24, 128)]),
        (23, vec![1], vec![(9, 2)]),
    ] {
        d.state(slot, &args, &mut h).unwrap();
        for (state, value) in states {
            assert_eq!(h.states[&state], value);
        }
    }
    d.error = 6;
    h.failures.push_back((Call::RenderState, FAIL));
    d.state(16, &[0], &mut h).unwrap();
    assert_eq!(d.error, 6);
    d.fog_int(0, 2, &mut h).unwrap();
    assert_eq!(h.states[&140], 3);
    d.fog_float(1, &[1., 0., 0., 1.], &mut h).unwrap();
    assert_eq!(h.states[&34], 0xffff0000);
    for (p, state) in [(2, 38), (3, 36), (4, 37)] {
        d.fog_float(p, &[0.25], &mut h).unwrap();
        assert_eq!(h.states[&state], 0.25f32.to_bits());
    }
    d.fog_int(1, 0, &mut h).unwrap();
    assert_eq!(d.get_error(), 1);
    d.fog_capable = false;
    d.fog_int(0, 0, &mut h).unwrap();
    assert_eq!(d.get_error(), 3);
    d.color_multiplier([0.5, 1., 0.], &mut h);
    assert_eq!(h.states[&139], 0xff80ff00);
    assert_eq!(h.states[&137], 1);
    assert_eq!(&d.material[4..7], &[1.; 3]);
    d.alpha_multiplier(0.25, &mut h);
    assert_eq!(h.states[&145], 0);
    assert_eq!(d.material[3], 0.25);
    d.enable_vertex_colors(false, true, &mut h);
    assert_eq!(h.states[&147], 0);
    d.alpha_multiplier(f32::NAN, &mut h);
    assert_eq!(h.states[&145], 1);
    assert_eq!(
        h.calls
            .iter()
            .filter(|c| c.call == Call::RenderState && c.args == [137, 1])
            .count(),
        1
    );
    d.default_state(&mut h);
    d.color_multiplier([1.; 3], &mut h);
    assert_eq!(
        h.calls
            .iter()
            .filter(|c| c.call == Call::RenderState && c.args == [137, 1])
            .count(),
        2
    );
}

#[test]
fn primitive_copy_merge_immediate_overtake_and_failure_paths() {
    let (mut d, mut h) = fixture();
    for i in 0..96 {
        h.memory.insert(0x4000 + i, (i % 256) as u8);
    }
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    assert_eq!(d.primitives.entries.len(), 1);
    assert_eq!(d.primitives.entries[0].count, 6);
    let mapped = h
        .calls
        .iter()
        .find(|c| c.call == Call::LockVertexBuffer)
        .unwrap()
        .target;
    assert_eq!(mapped, d.primitives.buffer.token);
    d.interleaved_arrays(38, 44, 0x4000);
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    assert_eq!(positions(&h, Call::Draw).len(), 1);
    assert!(positions(&h, Call::DrawBuffer).is_empty());
    d.state(15, &[], &mut h).unwrap();
    assert!(positions(&h, Call::Draw)[0] < positions(&h, Call::DrawBuffer)[0]);
    d.interleaved_arrays(33, 24, 0x4000);
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    assert_eq!(d.get_error(), 0);
    assert_eq!(d.primitives.entries.len(), 1);
    d.primitives.flush(&mut h);
    d.interleaved_arrays(11, 32, 0x4000);
    h.write(0x9000, &[2, 3, 4]).unwrap();
    d.draw_elements(0, 3, 1, 0x9000, &mut h).unwrap();
    d.draw_elements(0, 3, 1, 0x9000, &mut h).unwrap();
    assert_eq!(d.primitives.entries.len(), 1);
    assert_eq!(
        d.primitives.entries[0].indices.as_deref(),
        Some(&[0, 1, 2, 3, 4, 5][..])
    );
    d.primitives.started = false;
    assert_eq!(d.primitives.flush(&mut h), FAIL);
    assert_eq!(d.primitives.entries.len(), 1);
    d.primitives.stop(&mut h);
    assert!(d.primitives.entries.is_empty());
    assert_eq!(
        primitive::expanded(2, &[4, 5, 6, 7]).unwrap().1.unwrap(),
        [4, 5, 6, 4, 6, 7]
    );
    assert_eq!(
        primitive::expanded(7, &[0, 1, 2, 3, 4, 5]).unwrap().1.unwrap(),
        [0, 1, 3, 0, 3, 2, 2, 3, 5, 2, 5, 4]
    );
}

#[test]
fn vertex_buffer_lost_lock_retry_and_failed_unlock_clear_mapping() {
    let (mut d, mut h) = fixture();
    let b = &mut d.primitives.buffer;
    h.failures.push_back((Call::LockVertexBuffer, LOST));
    let before = b.token;
    assert!(b.reserve(2, false, 11, 12, &mut h).is_some());
    assert_ne!(b.token, before);
    assert!(b.locked);
    h.failures.push_back((Call::UnlockVertexBuffer, FAIL));
    assert_eq!(b.unlock(11, 12, &mut h), FAIL);
    assert!(!b.locked);
    assert_eq!(b.mapped, 0);
    assert_eq!(b.cursor, 0);
    h.failures.push_back((Call::CreateVertexBuffer, FAIL));
    assert_eq!(b.recreate(16, 11, 12, &mut h), FAIL);
    assert_eq!(b.token, 0);
    assert_eq!(b.capacity, 0);
    assert!(b.reserve(1, true, 11, 12, &mut h).is_none());
    assert_eq!(b.recreate(0, 11, 12, &mut h), 0);
}

#[test]
fn texture_descriptor_upload_retry_addressing_and_no_device_creation() {
    let (mut d, mut h) = fixture();
    let n = d.create_texture(2, 8, 4, 3, 3, &mut h).unwrap();
    let create = h.calls.iter().find(|c| c.call == Call::CreateSurface).unwrap();
    assert_eq!(&create.args[..7], &[0x7c, 0x21007, 8, 4, 3, 0x401008, 0x20004]);
    h.failures.push_back((Call::LockSurface, LOST));
    assert!(d
        .textures
        .load(
            Upload {
                x: 1,
                y: 2,
                ..upload(n)
            },
            &mut h
        )
        .unwrap());
    assert_eq!(positions(&h, Call::LockSurface).len(), 2);
    assert_eq!(positions(&h, Call::RestoreSurface).len(), 1);
    assert_eq!(positions(&h, Call::UnlockSurface).len(), 1);
    assert!(!d.textures.load(Upload { level: 3, ..upload(n) }, &mut h).unwrap());
    assert!(!d.textures.load(Upload { kind: 99, ..upload(n) }, &mut h).unwrap());
    assert!(d.textures.load(Upload { level: 2, ..upload(n) }, &mut h).unwrap());
    assert_eq!(positions(&h, Call::AttachedSurface).len(), 2);
    d.textures.lose(&mut h);
    let m = d.create_texture(1, 16, 16, 0, 2, &mut h).unwrap();
    assert_eq!(d.textures.surface(m), 0);
    assert!(d.textures.valid(m));
    d.textures.init_device(10, &mut h).unwrap();
    assert_ne!(d.textures.surface(m), 0);
    assert!(!d.textures.alpha(m));
    let count = h.calls.len();
    let allocated = d.create_texture(4, 0, 2, 1, 0, &mut h).unwrap();
    assert_eq!(d.error, 3);
    assert!(d.is_texture(allocated));
    assert_eq!(h.calls.len(), count);
    d.ready = false;
    d.delete_textures(&[m], &mut h);
    assert!(d.is_texture(m));
    assert_eq!(texture::source_format(8, 1), Some((10, 4)));
    assert_eq!(texture::source_format(9, 0), Some((12, 8)));
    assert_eq!(texture::source_format(10, 0), Some((14, 8)));
}

#[test]
fn stage_coordinates_matrices_environments_and_capability_gate() {
    let (mut d, mut h) = fixture();
    let n = d.create_texture(1, 2, 2, 1, 0, &mut h).unwrap();
    d.bind_texture(0, n).unwrap();
    d.capability(5, true, &mut h).unwrap();
    d.tex_stage_coord(0x32, &mut h);
    assert!(h
        .calls
        .iter()
        .any(|c| c.call == Call::StageState && c.args == [0, 11, 0x30002]));
    let before = h.calls.len();
    d.tex_stage_coord(0x40, &mut h);
    assert_eq!(d.get_error(), 2);
    assert_eq!(h.calls.len(), before);
    d.tex_stage_matrix(None, 4, 3, &mut h);
    assert_eq!(h.calls.last().unwrap().args, [0, 24, 0]);
    let mut m = IDENTITY;
    m[8] = 2.;
    d.tex_stage_matrix(Some(m), 3, 2, &mut h);
    let transform = h.calls.last().unwrap();
    assert_eq!(transform.args, [16]);
    assert_eq!(f32::from_le_bytes(transform.data[32..36].try_into().unwrap()), -2.);
    for (mode, expected) in [(0, (2, 2)), (1, (4, 2)), (2, (2, 2)), (3, (1, 1))] {
        d.tex_env(mode).unwrap();
        h.calls.clear();
        d.draw_arrays(0, 0, 0, &mut h).unwrap();
        assert!(h
            .calls
            .iter()
            .any(|c| c.call == Call::StageState && c.args == [0, 1, expected.0]));
        assert!(h
            .calls
            .iter()
            .any(|c| c.call == Call::StageState && c.args == [0, 4, expected.1]));
    }
    d.current.set_byte(0x29, 0);
    d.combine_field(54, 0, 1).unwrap();
    assert_eq!(d.get_error(), 2);
    d.combine_field(56, 0, 2).unwrap();
    assert_eq!(d.stages[0].sources[0][0], 2);
    d.tex_stage(8);
    assert_eq!(d.get_error(), 2);
    assert_eq!(d.current_stage, 0);
}

#[test]
fn lifecycle_initialization_info_noop_and_known_light_fields() {
    let _guard = INFO_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut d = Driver::new(0x100, 0x200);
    let mut h = Recorder::default();
    h.write(0x7000, b"Test").unwrap();
    d.init(&mut h).unwrap();
    assert!(d.initialized);
    assert_eq!(d.init_word, Some(7));
    assert!(d
        .driver_info()
        .starts_with(b"Maxis 3D GDriver DirectX 2.0\nTest\n1.2.3.4, GUID:"));
    assert!(d
        .driver_info()
        .ends_with(b"Vendor: 0001, Device: 0002, Board: 0003, Chipset: 0004"));
    assert!(h.refs.iter().any(|(_, op)| *op == Op::Release));
    let count = h.calls.len();
    d.error = 6;
    d.init(&mut h).unwrap();
    assert_eq!(d.error, 0);
    assert_eq!(h.calls.len(), count);
    let light = D3DLight::default();
    assert_eq!(light.kind, 1);
    assert_eq!(light.colors, [[0.; 4]; 3]);
    assert_eq!(light.position, [0.; 3]);
    assert_eq!(light.direction, [0., 0., 10.]);
    assert_eq!(light.falloff, 1.);
    assert_eq!(light.attenuation, [0.; 3]);
    assert_eq!(light.theta.to_bits(), 0x3f47ae14);
    assert_eq!(light.phi.to_bits(), 0x3fc8f5c3);
    assert_eq!(light.range.to_bits(), 0x5f7fffff);
    assert_eq!(d.auxiliary_operation(), Err(Error::Unsupported("RD7")));
    let mut untouched = 0xfeed;
    d.pixel_store(0, 123);
    d.get_integer(1, &mut untouched);
    assert_eq!(untouched, 0xfeed);
    d.get_integer(7, &mut untouched);
    assert_eq!(untouched, 123);
    let mut mode = [0xbeef; 13];
    d.mode_info(0, &mut mode);
    assert_eq!(mode, [0xbeef; 13]);
    assert_eq!(d.get_error(), 2);
}

#[test]
fn initialization_failures_and_shutdown_preserve_specified_cleanup() {
    let _guard = INFO_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for call in [Call::RegisterClass, Call::CreateDirectDraw, Call::AdapterInfo] {
        let mut d = Driver::new(0x100, 0x200);
        let mut h = Recorder::default();
        assert_eq!(d.init_word, None);
        d.count_modes(&mut h);
        assert_eq!(d.modes.len(), 4);
        h.calls.clear();
        h.refs.clear();
        h.failures.push_back((call, FAIL));
        let result = d.init(&mut h);
        assert!(d.initialized);
        assert_eq!(d.init_word, Some(7));
        assert_eq!(d.current.words[0], u32::MAX);
        assert!(d.modes.is_empty());
        if call == Call::AdapterInfo {
            assert_eq!(result, Ok(()));
            assert_eq!(h.refs.len(), 1);
            assert_eq!(h.refs[0].1, Op::Release);
        } else {
            assert_eq!(result, Ok(()));
            assert_eq!(d.error, 6);
            assert!(h.refs.is_empty());
            assert_eq!(
                positions(&h, Call::MessageBox).len(),
                usize::from(call == Call::CreateDirectDraw)
            );
        }
        let count = h.calls.len();
        d.init_word = Some(99);
        d.init(&mut h).unwrap();
        assert_eq!(h.calls.len(), count);
        assert_eq!(d.error, 0);
        assert_eq!(d.init_word, Some(99));
    }
    let (mut d, mut h) = fixture();
    d.initialized = true;
    d.scene = true;
    d.draw_arrays(0, 0, 3, &mut h).unwrap();
    let device = d.device;
    let buffer = d.primitives.buffer.token;
    assert_eq!(d.shutdown(&mut h), Ok(()));
    assert!(!d.initialized);
    assert!(!d.ready);
    assert!(!d.scene);
    assert!(d.modes.is_empty());
    assert!(d.stages.is_empty());
    assert!(d.primitives.entries.is_empty());
    assert_eq!(d.window, 0);
    assert_eq!(d.current.words[0], u32::MAX);
    // The fixture gives the context and texture manager separate device references.
    for (token, count) in [(device, 3), (buffer, 1)] {
        assert_eq!(h.refs.iter().filter(|r| **r == (token, Op::Release)).count(), count);
    }
    assert!(positions(&h, Call::EndScene)[0] < positions(&h, Call::DestroyWindow)[0]);
    assert!(positions(&h, Call::DestroyWindow)[0] < positions(&h, Call::UnregisterClass)[0]);
    let count = h.calls.len();
    d.shutdown(&mut h).unwrap();
    assert_eq!(h.calls.len(), count);
}

#[path = "closure_tests.rs"]
mod closure_tests;

#[test]
fn context_failure_does_not_release_borrowed_entry_or_stale_output() {
    let mut h = Recorder::default();
    h.failures.push_back((Call::CreateDirectDraw, FAIL));
    let (c, hr) = Context::create(
        Request {
            selector: u32::MAX,
            fullscreen: false,
            flip: 0,
            window: 1,
            width: 800,
            height: 600,
            back_buffers: 2,
        },
        &mut h,
    )
    .unwrap();
    assert_eq!(hr, FAIL);
    assert_eq!(c.identity, 0);
    assert!(h.refs.is_empty());
    assert_eq!(positions(&h, Call::UnloadLibrary).len(), 1);
}

#[test]
fn compressed_descriptor_preference_and_ordinary_fallback() {
    let mut h = Recorder::default();
    let formats: Vec<_> = [0, 2, 9, 11, 13]
        .into_iter()
        .map(|key| {
            let mut words = [0; 8];
            words[7] = key;
            words[6] = if key == 0 { 16 } else { 24 };
            texture::Format { words, alpha_bits: if key == 0 { 1 } else { 4 }, size: 32 }
        })
        .collect();
    for (request, key) in [(5, 9), (6, 11), (7, 13)] {
        assert_eq!(texture::select_format(request, &formats, &mut h).unwrap().words[7], key);
    }
    assert_eq!(texture::select_format(5, &formats[..2], &mut h).unwrap().words[7], 0);
    assert_eq!(texture::select_format(6, &formats[..2], &mut h).unwrap().words[7], 2);
    assert_eq!(texture::select_format(7, &formats[..2], &mut h).unwrap().words[7], 2);
}

#[test]
fn null_setup_failure_keeps_native_hazard_and_cleans_window() {
    let mut h = Recorder::default();
    let mut d = Driver::new(0x100, 0x200);
    d.count_modes(&mut h);
    h.failures.push_back((Call::LoadLibrary, FAIL));
    assert_eq!(
        d.set_video_mode(0, 0, false, 0, &mut h),
        Err(Error::Foreign(crate::gdrivers_director::Fault::NullWrite))
    );
    assert_eq!(d.error, 6);
    assert_eq!(d.window, 0);
}

#[test]
fn loss_placeholder_is_owned_and_released_on_restore() {
    let (mut d, mut h) = fixture();
    h.failures.push_back((Call::CooperativeLevel, FAIL));
    d.flush(&mut h).unwrap();
    let placeholder = d.device;
    assert_ne!(placeholder, 0);
    h.refs.clear();
    d.flush(&mut h).unwrap();
    assert_eq!(
        h.refs
            .iter()
            .filter(|(token, op)| *token == placeholder && *op == Op::Release)
            .count(),
        1
    );
    let mut fresh = Driver::new(0x100, 0x200);
    h.calls.clear();
    fresh.flush(&mut h).unwrap();
    assert!(h.calls.is_empty());
}

#[test]
fn buffer_null_mapping_and_discard_survive_failed_lock() {
    let (mut d, mut h) = fixture();
    let b = &mut d.primitives.buffer;
    h.failures.push_back((Call::LockVertexBuffer, 0));
    assert!(b.reserve(1, false, 11, 12, &mut h).is_none());
    assert!(!b.locked);
    assert_eq!(b.mapped, 0);
    assert_eq!(positions(&h, Call::UnlockVertexBuffer).len(), 1);
    b.cursor = b.capacity;
    h.failures.push_back((Call::LockVertexBuffer, FAIL));
    assert!(b.reserve(1, true, 11, 12, &mut h).is_none());
    assert!(b.reserve(1, false, 11, 12, &mut h).is_some());
    assert_eq!(
        h.calls
            .iter()
            .rev()
            .find(|c| c.call == Call::LockVertexBuffer)
            .unwrap()
            .args,
        [0x2021]
    );
    b.unlock(11, 12, &mut h);
    h.failures.push_back((Call::RestoreAll, FAIL));
    assert_eq!(b.recreate(16, 11, 12, &mut h), FAIL);
    assert_eq!(b.token, 0);
    assert_eq!(b.capacity, 16);
}
