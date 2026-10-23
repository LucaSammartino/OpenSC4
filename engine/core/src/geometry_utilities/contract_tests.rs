// SPDX-License-Identifier: GPL-3.0-or-later
//! Expected values are the independently recorded public §10.1 ledger.
use super::{clipping::*, pc24, raw_texture::*, *};

struct Host {
    events: Vec<String>, mode: u32, w: i32, h: i32,
    font: Option<u32>, gs: bool, factory: bool, surface: Option<u32>, create: bool,
    fail_access: Vec<u32>, d: SurfaceDescriptor, data: Vec<u8>,
    upload_result: bool, uploaded: Option<(UploadDescriptor, Vec<u8>)>,
}
impl Default for Host {
    fn default() -> Self {
        Self { events: vec![], mode: 7, w: 30, h: 12, font: Some(1), gs: true,
            factory: true, surface: Some(2), create: true, fail_access: vec![],
            d: SurfaceDescriptor {width: 4, height: 4, byte_pitch: 16, bytes_per_pixel: 4, pixel_code: 9, base: 0},
            data: vec![0xab; 64], upload_result: false, uploaded: None }
    }
}
impl Host { fn event(&mut self, s: impl Into<String>) { self.events.push(s.into()); } }
impl RawTextureHost for Host {
    fn default_font(&mut self, id: u32) -> Option<u32> { assert_eq!(id, 0x68963c4c); self.event("lookup"); self.font }
    fn font_add_ref(&mut self, _: u32) { self.event("font+"); }
    fn font_release(&mut self, _: u32) { self.event("font-"); }
    fn font_mode(&mut self, _: u32) -> u32 { self.event("save"); self.mode }
    fn font_set_mode(&mut self, _: u32, m: u32) { self.event(format!("mode{m}")); self.mode = m; }
    fn font_width(&mut self, _: u32, text: &[u8]) -> i32 { assert_eq!(self.mode, 3); self.event(format!("width{}", text.len())); self.w }
    fn font_height(&mut self, _: u32, text: &[u8]) -> i32 { self.event(format!("height{}", text.len())); self.h }
    fn font_draw(&mut self, _: u32, text: &[u8], _: u32, xy: [i32; 2], colour: u32) {
        assert_eq!(self.mode, 3); assert_eq!(xy, [2, 1]); assert_eq!(colour, u32::MAX);
        self.event(format!("draw{}", text.len())); self.data.fill(0xab);
    }
    fn graphics_system(&mut self) -> bool { self.event("gs"); self.gs }
    fn surface_factory(&mut self) -> (bool, Option<u32>) { self.event("factory"); (self.factory, self.surface) }
    fn surface_create(&mut self, _: u32, width: i32, height: i32, code: u32) -> bool {
        self.event("create"); self.d.width = width; self.d.height = height;
        self.d.pixel_code = code; self.d.byte_pitch = width * 4; self.data = vec![0xff; (width * height * 4) as usize]; self.create
    }
    fn surface_release(&mut self, _: u32) { self.event("surface-"); }
    fn descriptor(&mut self, _: u32) -> SurfaceDescriptor { self.d }
    fn access(&mut self, _: u32, flags: u32) -> bool { self.event(format!("access{flags:x}")); !self.fail_access.contains(&flags) }
    fn end_access(&mut self, _: u32, flags: u32) { self.event(format!("end{flags:x}")); }
    fn bytes(&mut self, _: u32) -> &mut [u8] { &mut self.data }
    fn acquire_service(&mut self) -> bool { self.event("service+"); true }
    fn cleanup_service(&mut self) { self.event("service-"); }
    fn upload(&mut self, texture: u32, d: UploadDescriptor, bytes: &[u8]) -> bool {
        assert_eq!(texture, 9); self.event(format!("upload-mode{}", self.mode));
        self.uploaded = Some((d, bytes.to_vec())); self.upload_result
    }
    fn allocate_temporary(&mut self, len: usize) -> Option<Vec<u8>> { self.event("alloc"); Some(vec![0; len]) }
    fn free_temporary(&mut self, _: Vec<u8>) { self.event("free"); }
}

#[test]
fn t1_font_only_early_false_and_common_failure_outputs() {
    let mut host = Host { font: None, ..Host::default() };
    let mut size = [71; 2]; let mut rect = [72.; 4];
    assert_eq!(create_text_raw(&mut host, b"x\0", 9, None, Some(&mut size), Some(&mut rect), false), Ok(false));
    assert_eq!(host.events, ["lookup"]); assert_eq!(size, [71; 2]); assert_eq!(rect, [72.; 4]);
    for scenario in 0..3 {
        let mut host = Host::default();
        if scenario == 0 { host.gs = false; }
        if scenario == 1 { host.factory = false; }
        if scenario == 2 { host.create = false; }
        assert_eq!(create_text_raw(&mut host, b"\xff\xc3\0ignored", 9, Some(1), Some(&mut size), Some(&mut rect), false), Ok(false));
        assert_eq!(size, [32, 16]); assert_eq!(rect, [32., 13., 2., 1.]);
        assert_eq!(&host.events[..7], ["font+", "save", "mode3", "width2", "height2", "gs", if scenario == 0 {"mode7"} else {"factory"}]);
        let end = if scenario == 0 { vec!["mode7", "font-"] } else {vec!["mode7", "surface-", "font-"]};
        assert!(host.events.ends_with(&end.into_iter().map(String::from).collect::<Vec<_>>()));
        assert!(host.uploaded.is_none());
    }
}

#[test]
fn t2_access_gates_only_own_operations_upload_keeps_mode_three() {
    for flags in [vec![], vec![0x8001], vec![0x8004], vec![0x8001, 0x8004]] {
        let mut host = Host { fail_access: flags.clone(), ..Host::default() };
        let mut size = [0; 2]; let mut rect = [0.; 4];
        assert_eq!(create_text_raw(&mut host, b"ab\0", 9, Some(1), Some(&mut size), Some(&mut rect), false), Ok(false));
        assert_eq!(size, [32, 16]); assert_eq!(rect, [32., 13., 2., 1.]);
        for access in [0x8001, 0x8004] {
            assert_eq!(host.events.contains(&format!("end{access:x}")), !flags.contains(&access));
        }
        assert!(host.events.contains(&"draw2".into()));
        assert!(host.events.ends_with(&["upload-mode3", "end8040", "service-", "mode7", "surface-", "font-"].map(String::from)));
        let (_, bytes) = host.uploaded.unwrap();
        if !flags.contains(&0x8004) { assert_eq!(&bytes[..128], &[0; 128]); }
        else { assert_eq!(&bytes[..4], &[0xab; 4]); }
        assert_eq!(host.mode, 7);
    }
}

#[test]
fn t_domain_wrapping_dimensions_and_nonnull_factory_precondition() {
    assert_eq!(text_layout(-1, -2).unwrap().size, [1, 1]);
    assert_eq!(text_layout(i32::MAX, i32::MIN).unwrap().size, [1, 1]);
    assert_eq!(text_layout((1 << 30) - 1, 1), Err(Unsupported::NonterminatingDimension));
    assert_eq!(text_layout((1 << 30) - 2, 1).unwrap().size, [1 << 30, 4]);
    let mut host = Host { surface: None, ..Host::default() };
    assert_eq!(create_text_raw(&mut host, b"a\0", 9, Some(1), None, None, false), Err(Unsupported::RequiredHandle));
    assert!(!host.events.contains(&"create".into()));
    assert_eq!(create_text_raw(&mut host, b"unterminated", 9, Some(1), None, None, false), Err(Unsupported::UnterminatedText));
}

#[test]
fn u1_descriptor_stride_offset_and_false_cleanup() {
    let mut host = Host::default();
    host.d.width = 8; host.d.height = 4; host.d.byte_pitch = 32; host.data = vec![0; 128];
    assert_eq!(upload_plain_raw(&mut host, Some(2), Some(9), Some([1, 1, 3, 3])), Ok(false));
    let (d, _) = host.uploaded.as_ref().unwrap();
    assert_eq!(*d, UploadDescriptor { width: 2, height: 2, row_stride: 8, format: 3, element_type: 1, auxiliary: [1, 1], source_offset: 36 });
    assert_eq!(host.events, ["service+", "access8040", "upload-mode7", "end8040", "service-"]);
    host.d.byte_pitch = -33; host.d.base = 100;
    assert_eq!(upload_plain_raw(&mut host, Some(2), Some(9), Some([1, 1, 3, 3])), Ok(false));
    let (d, _) = host.uploaded.as_ref().unwrap();
    assert_eq!((d.row_stride, d.source_offset), (-8, 71));
    host.d.pixel_code = u32::MAX; host.uploaded = None;
    assert_eq!(upload_plain_raw(&mut host, Some(2), Some(9), None), Err(Unsupported::Descriptor));
    assert!(host.uploaded.is_none());
    let mut format = 71; let mut ty = 72;
    for code in [1, 10, u32::MAX] {
        assert!(!texture_format(code, 0, &mut format, &mut ty)); assert_eq!((format, ty), (71, 72));
    }
    for (code, expected) in [(0,14),(2,14),(3,14),(5,14),(4,10),(6,10),(7,13),(8,1),(9,1)] {
        assert!(texture_format(code, u32::MAX, &mut format, &mut ty)); assert_eq!((format, ty), (3, expected));
    }
}

#[test]
fn u2_dxt3_bytes_descriptor_and_free_before_end() {
    let mut host = Host::default();
    assert_eq!(upload_compressed_raw(&mut host, 2, 9), Ok(false));
    let (d, bytes) = host.uploaded.as_ref().unwrap();
    assert_eq!(*bytes, [0xaa,0xaa,0xaa,0xaa,0xaa,0xaa,0xaa,0xaa,0,0,255,255,0x55,0x55,0x55,0x55]);
    assert_eq!((d.width,d.height,d.row_stride,d.format,d.element_type,d.auxiliary), (4,4,4,9,0,[1,1]));
    assert_eq!(host.events, ["access8040", "alloc", "upload-mode7", "free", "end8040"]);
    for scenario in 0..3 {
        let mut host = Host::default();
        if scenario == 0 {host.d.pixel_code = 8;}
        if scenario == 1 {host.d.width = 2;}
        if scenario == 2 {host.fail_access.push(0x8040);}
        assert_eq!(upload_compressed_raw(&mut host, 2, 9), Ok(false));
        assert!(!host.events.contains(&"end8040".into())); assert!(host.uploaded.is_none());
    }
}

#[test]
fn c1_c2_c3_signed_count_and_untouched_backing_tail() {
    let poly: Vec<_> = (0..41).map(|i| [i as f32, 0.]).collect();
    for m in [-1, 0, 1] {
        for max in [-1, 0, 1, 100] {
            let mut out = [[99.; 2]; 44]; let mut count = 99;
            clip_polygon_signed(&poly, 41, &[[0.; 2]], m, &mut out, max, &mut count).unwrap();
            let expected = if m <= 0 {41} else {40.min(max)};
            assert_eq!(count, expected); assert_eq!(&out[..41], poly); assert_eq!(out[41], [99.; 2]);
        }
    }
    let mut out = [[99.; 2]; 40]; let mut count = 99;
    assert_eq!(clip_polygon_signed(&poly, 41, &[], 0, &mut out, 0, &mut count), Err(ClipUnsupported::Backing));
}

#[test]
fn c4_c5_failed_crossing_gates_and_nonpositive_counts() {
    let mut out = [[71.; 2]; 8]; let mut count = 99;
    let poly = [[0., -1./1024.], [0., 1./1024.]];
    clip_edge_signed([0.; 2], [1., 0.], &poly, 2, &mut out, 0, &mut count).unwrap();
    assert_eq!(count, 0); assert_eq!(out, [[71.; 2]; 8]);
    assert_eq!(clip_edge_signed([0.; 2], [1., 0.], &poly, 2, &mut out, 1, &mut count), Err(ClipUnsupported::UninitializedCrossing));
    for n in [-1, 0] {
        clip_edge_signed([f32::NAN; 2], [f32::NAN; 2], &[], n, &mut out, 1, &mut count).unwrap(); assert_eq!(count, 0);
        clip_polygon_signed(&[], n, &[], 0, &mut out, -1, &mut count).unwrap(); assert_eq!(count, n);
    }
}

#[test]
fn n1_n2_n4_pc24_bits_and_explicit_boundary() {
    assert_eq!(pc24::squared_length([0.; 3], [4096., 1., 1.]).unwrap().to_bits(), 0x4b800001);
    assert_eq!(pc24::plane([1.; 3], [1., 16777216., -16777216.]).unwrap()[3].to_bits(), 0xbf800000);
    let b = [4096., 17./16., 17./16.];
    assert_eq!(pc24::squared_length([0.; 3], b).unwrap().to_bits(), 0x4b800001);
    assert_eq!(pc24::length([0.; 3], b).unwrap().to_bits(), 0x45800000);
    assert!(pc24::squared_length([0.; 3], [f32::MAX; 3]).is_err());
    assert!(pc24::squared_length([0.; 3], [f32::MIN_POSITIVE; 3]).is_err());
    assert!(pc24::plane([f32::NAN; 3], [1.; 3]).is_err());
}

#[test]
fn b1_b2_b3_operand_selection_and_output_gates() {
    let mut min = [71.; 3]; let mut max = [72.; 3];
    assert!(bounding_boxes_intersect([0.; 3], [1.; 3], [-0.,0.,0.], [1.; 3], Some(&mut min), Some(&mut max)));
    assert_eq!(min[0].to_bits(), 0x80000000);
    assert!(bounding_boxes_intersect([-1.; 3], [0.,1.,1.], [-1.; 3], [-0.,1.,1.], Some(&mut min), Some(&mut max)));
    assert_eq!(max[0].to_bits(), 0x80000000);
    let nan = f32::from_bits(0x7fc00001);
    for (a,b,expected_nan) in [(nan,0.,false),(0.,nan,true)] {
        assert!(bounding_boxes_intersect([a,0.,0.], [1.; 3], [b,0.,0.], [1.; 3], Some(&mut min), Some(&mut max)));
        assert_eq!(min[0].is_nan(), expected_nan); if !expected_nan {assert_eq!(min[0], 0.);}
    }
    assert!(!point_in_rectangle_2d([nan,0.], [0.;2], [1.;2]));
    min = [71.;3]; max = [72.;3];
    assert!(!bounding_boxes_intersect([0.;3], [1.;3], [1.,0.,0.], [2.;3], Some(&mut min), Some(&mut max)));
    assert_eq!(min,[71.;3]); assert_eq!(max,[72.;3]);
    assert!(bounding_boxes_intersect([0.;3], [1.;3], [0.;3], [1.;3], None, Some(&mut max))); assert_eq!(max,[72.;3]);
}

#[test]
fn m1_m2_m3_m4_m5_magic_colour_and_wrapped_closure() {
    assert_eq!(pc24::colour_byte(65537.).unwrap(),128);
    assert_eq!(pc24::colour_byte(1.).unwrap(),255);
    assert_eq!(pc24::lit_colour([1.,0.,0.],[-1.,0.,0.],[1.,0.,0.],1.).unwrap(),[0,0,155,255]);
    let mut b = MeshBuilder::new(); b.begin_primitive(1); b.set_colour(1.,0.,0.);
    for _ in 0..65535 { b.add_vertex([0.;3]); }
    b.end_primitive(); b.mesh(); assert!(!b.changed());
    let count = b.primitives()[0].count;
    b.add_vertex([71.;3]); assert_eq!(b.vertex_count(),0); assert!(!b.changed()); assert_eq!(b.primitives()[0].count,count);
    b.add_vertex([72.;3]); assert_eq!(b.vertices()[0].position,[72.;3]); assert_eq!(b.vertex_count(),1);
    b.reset(); assert!(!b.changed()); assert_eq!(b.vertex_count(),0);
    b.begin_primitive(1); b.begin_primitive(2); assert_eq!(b.primitives().len(),2); assert!(b.changed());
    b.reset(); assert!(b.changed());
    for _ in 0..65535 { b.add_vertex([0.;3]); }
    add_box_mesh(&mut b,&box_corners([0.;3],[1.;3]));
    let p = b.primitives().last().unwrap(); assert_eq!(p.first,65535); assert_eq!(p.count,0xffff0018); assert_eq!(b.vertex_count(),23);
}
