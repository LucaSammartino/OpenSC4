use super::*;
use crate::network_occupants::tests_pylon::{reference, token, Host};

impl BitmapHost for Host {
    fn load_bitmap(&mut self, key: [u32; 3], format: u32, immediate: bool) -> RefValue {
        assert_eq!(format, 9);
        assert!(immediate);
        self.log.push(format!("bitmap/{:x}/{:x}", key[1], key[2]));
        if !self.bitmaps { RefValue::Null } else if key[1] == 0xAB7E_5421 { reference(101) }
        else if self.portrait { reference(102) } else { RefValue::Null }
    }
    fn portrait_service(&mut self, class: u32, iid: u32) -> RefValue {
        assert_eq!((class, iid), (0xA2FF_B5D3, 0x02FF_BB60));
        reference(103)
    }
    fn portrait(&mut self, _: &RefValue, _: &[u8], argument: u32) -> RefValue {
        assert_eq!(argument, 8);
        if self.portrait { reference(102) } else { RefValue::Null }
    }
    fn bitmap_info(&mut self, value: &RefValue) -> BitmapInfo {
        if token(value) == 102 { BitmapInfo { width: 1, height: 1, stride: 12, ..Default::default() } }
        else { BitmapInfo { width: 208, height: 20, stride: 208 * 4, ..Default::default() } }
    }
    fn lock_bitmap(&mut self, value: &RefValue, flags: u32) -> bool { self.log.push(format!("lock/{}/{flags:x}", token(value))); self.bitmap_lock }
    fn unlock_bitmap(&mut self, value: &RefValue, flags: u32) { self.log.push(format!("unlock/{}/{flags:x}", token(value))); }
    fn pixel(&mut self, value: &RefValue, offset: u32) -> u32 {
        if token(value) == 102 { assert_eq!(offset, 0); self.source_pixel } else { self.pixels.get(&offset).copied().unwrap_or(0) }
    }
    fn put_pixel(&mut self, _: &RefValue, offset: u32, pixel: u32) { self.pixels.insert(offset, pixel); }
    fn setup_texture(&mut self, bitmap: &RefValue, id: &mut u32) -> bool {
        self.log.push(format!("upload/{}", token(bitmap)));
        *id = token(bitmap) as u32 + 1000;
        true
    }
    fn font(&mut self, resource: u32) -> RefValue { assert_eq!(resource, 0x6896_3C4C); reference(104) }
    fn font_setting(&mut self, _: &RefValue) -> u32 { 17 }
    fn set_font_setting(&mut self, _: &RefValue, value: u32) { self.log.push(format!("font-setting/{value}")); }
    fn measure_text(&mut self, _: &RefValue, slot: u32, text: &[u8]) -> i32 {
        self.log.push(format!("measure/{slot:x}/{}", String::from_utf8_lossy(text)));
        self.measurements[usize::from(slot == 0x94)]
    }
    fn acquire_bitmap(&mut self) -> RefValue { if self.bitmap_acquisition { reference(105) } else { RefValue::Null } }
    fn create_bitmap(&mut self, _: &RefValue, side: u32, descriptor: [u32; 2]) -> bool {
        assert_eq!(descriptor, [9, 32]); self.log.push(format!("create/{side}")); true
    }
    fn clear_bitmap(&mut self, _: &RefValue, value: u32, argument: u32) { assert_eq!((value, argument), (0, 0)); self.log.push("clear".into()); }
    fn draw_text(&mut self, _: &RefValue, _: &RefValue, text: &[u8], colours: [u32; 2]) {
        assert_eq!(colours, [0xFF00_0000, 0x0800_0000]); self.log.push(format!("text/{}", String::from_utf8_lossy(text)));
    }
}
impl PylonModelHost for Host {
    fn viewport_to_world(&mut self, value: f32) -> f64 { f64::from(value) }
    fn pitch(&mut self) -> f64 { 0.0 }
    fn facing(&mut self) -> [f64; 2] { [0.0, -1.0] }
    fn renderer_refresh_remove(&mut self) { self.log.push("renderer-refresh-remove".into()); }
    fn renderer_refresh_insert(&mut self, value: u32) { assert_eq!(value, 3); self.log.push("renderer-refresh-insert".into()); }
    fn manager_changed(&mut self, value: u32) { assert_eq!(value, 0); self.log.push("manager-changed".into()); }
}
impl PylonDrawHost for Host {
    fn install_transform(&mut self, _: Xform) { self.log.push("transform".into()); }
    fn clear_texture_transform(&mut self, stage: u32) { assert_eq!(stage, 0); }
    fn transparency(&mut self, value: Option<f32>) { self.log.push(format!("transparency/{:?}", value.map(f32::to_bits))); }
    fn render_state(&mut self, value: u64) { self.log.push(format!("state/{value}")); }
    fn draw_quad(&mut self, texture: u32, vertices: [BillboardVertex; 4], primitive: u32, format: u32) {
        assert_eq!((primitive, format), (6, 2)); self.quads.push((texture, vertices));
    }
}

#[test]
fn mysim_pixel_predicate_and_resource_lifetimes_conformance_56() {
    let mut p = Pylon::new([0, 0, 4]);
    let mut h = Host { bitmaps: true, portrait: true, ..Default::default() };
    let offset = |x: u32| (12 * 208 + x) * 4;
    h.pixels.insert(offset(129), 0x4000_0000);
    assert_eq!(p.rebuild_current_texture(&mut h), Ok(true));
    assert_eq!((p.texture, p.label_texture), (1101, 0));
    assert_eq!(h.pixels[&offset(77)], 0x0011_2233);
    assert_eq!(h.pixels[&offset(181)], 0x0011_2233);
    assert_eq!(h.pixels[&offset(129)], 0x4000_0000);
    for id in [101, 102, 103] { assert!(h.log.contains(&format!("release{id}"))); }
    h.pixels.clear(); h.source_pixel = 0x12FF_00FF;
    p.rebuild_current_texture(&mut h).unwrap();
    assert!(h.pixels.is_empty());
    h.portrait = false;
    for (state, background) in [(0, "2bb075b4"), (1, "2bb06f3f"), (6, "2bb075b4")] {
        h.log.clear(); p.record.states[1] = state;
        p.rebuild_current_texture(&mut h).unwrap();
        assert!(h.log.contains(&format!("bitmap/ab7e5421/{background}")));
        assert!(h.log.contains(&"upload/101".into()));
    }
    h.bitmaps = false; h.log.clear();
    assert_eq!(p.rebuild_current_texture(&mut h), Ok(false));
    assert!(!h.log.iter().any(|s| s.starts_with("upload")));
}

#[test]
fn main_billboard_and_registration_conformance_57() {
    let mut p = Pylon::new([0, 0, 0]);
    p.record.transform.translation = [10.0, 20.0, 30.0];
    let mut h = Host { prepare: false, ..Default::default() };
    p.rebuild_model(&mut h).unwrap();
    assert_eq!(p.main_vertices.map(|v| v.position), [[-22.0,150.0,0.0],[-22.0,0.0,0.0],[22.0,0.0,0.0],[22.0,150.0,0.0]]);
    assert_eq!(p.main_vertices.map(|v| v.uv), [[0.171875,0.0],[0.171875,0.5859375],[0.34375,0.5859375],[0.34375,0.0]]);
    assert_eq!(p.bounds(), [[-22.0,0.0,0.0],[22.0,150.0,0.0]]);
    assert_eq!(p.world_bounds(), [[-12.0,20.0,30.0],[32.0,170.0,30.0]]);
    assert_eq!((p.record.transform.mask, p.record.transform.counter, p.record.dirty), (4,1,false));
    p.rebuild_model(&mut h).unwrap();
    assert_eq!(p.record.transform.counter, 3);
    p.record.blink = true; p.record.states[0] = -7; p.placement_flags = 3;
    h.log.clear(); p.rebuild_model(&mut h).unwrap();
    assert_eq!(p.main_vertices[0].uv[0], 3.0*0.171875);
    assert!(h.log.ends_with(&["renderer-refresh-remove".into(), "renderer-refresh-insert".into(), "manager-changed".into()]));
}

#[test]
fn label_replacement_geometry_and_failed_bitmap_conformance_58_59() {
    let mut p = Pylon::new([0;3]); p.record.call_number = 7; p.label_texture = 42;
    let mut h = Host { bitmap_acquisition: false, ..Default::default() };
    p.rebuild_label(44.0, 150.0, &mut h);
    assert_eq!(p.label_texture, 0);
    assert!(h.log.contains(&"texture42".into()));
    assert!(h.log.contains(&"measure/c0/7".into()));
    assert!(h.log.ends_with(&["font-setting/17".into(), "release104".into()]));
    assert_eq!(p.label_vertices.map(|v| v.position), [[-10.0,127.0,0.01],[-10.0,113.0,0.01],[10.0,113.0,0.01],[10.0,127.0,0.01]]);
    assert_eq!(p.label_vertices[2].uv, [0.625, 0.3125]);
    let before = p.label_vertices;
    p.rebuild_label(999.0, 150.0, &mut h); assert_eq!(p.label_vertices, before);
    h.bitmap_acquisition = true; h.bitmap_lock = false; h.log.clear();
    p.rebuild_label(1.0, 150.0, &mut h);
    assert!(h.log.contains(&"create/32".into()));
    assert!(h.log.contains(&"text/7".into()));
    assert!(!h.log.contains(&"clear".into()));
    assert_eq!(p.label_texture, 1105);
    h.measurements[0] = 33; h.log.clear(); p.rebuild_label(1.0, 150.0, &mut h);
    assert!(h.log.contains(&"create/64".into()));
    p.record.call_number = 0;
    let vertices = p.label_vertices; p.rebuild_model(&mut h).unwrap();
    assert_eq!((p.label_texture, p.label_vertices), (1105, vertices));
}

#[test]
fn pylon_draw_pass_dirty_and_message_conformance_52_53() {
    let mut p = Pylon::new([0;3]); p.record.call_number = 7;
    let mut h = Host::default();
    assert_eq!(p.draw(0,true,false,&mut h), Ok(1)); assert!(p.record.dirty); assert!(h.quads.is_empty());
    assert_eq!(p.draw(1,false,false,&mut h), Ok(1)); assert!(p.record.dirty); assert_eq!(h.quads.len(),2);
    for (state,call,kind) in [(1,7,0),(4,7,0),(0,0,0),(0,7,4)] {
        p.record.states[1]=state; p.record.states[2]=kind; p.record.call_number=call; h.quads.clear();
        p.draw(1,false,false,&mut h).unwrap(); assert_eq!(h.quads.len(),1);
    }
    h.quads.clear(); p.draw(2,false,false,&mut h).unwrap(); assert!(h.quads.is_empty()); assert!(p.record.dirty);
    p.record.states=[0;3]; p.record.call_number=0; h.log.clear();
    assert_eq!(p.draw(0,false,false,&mut h),Ok(2)); assert!(!p.record.dirty); assert_eq!(h.quads.len(),1);
    assert!(h.log.contains(&format!("transparency/{:?}",Some(0x3ECC_CCCDu32))));
    let mut globals=PylonGlobals::default(); p.record.states[1]=1;
    for (payload,total) in [(999,0),(1000,1),(1999,2),(2000,4)] {
        assert_eq!(p.do_message(0x86AD_10EE,payload,true,&mut globals,&mut h),Ok(true));
        assert_eq!(globals.blink_accumulator,total);
    }
    p.placement_flags=0; h.log.clear();
    p.do_message(0x06CD_B65B,0,false,&mut globals,&mut h).unwrap(); assert!(h.log.is_empty());
    p.do_message(0x06CD_B65B,0,true,&mut globals,&mut h).unwrap();
    p.do_message(0x06CD_B65B,0,true,&mut globals,&mut h).unwrap();
    p.do_message(0x06CD_B65B,0,false,&mut globals,&mut h).unwrap();
    assert_eq!(h.log,["renderer+","renderer-"]);
    p.record.dirty=false; h.log.clear(); p.do_message(0xA6B7_9602,0,true,&mut globals,&mut h).unwrap();
    assert!(p.record.dirty); assert!(h.log.is_empty());
    p.do_message(0x26D3_1EC1,0,true,&mut globals,&mut h).unwrap(); assert!(!p.record.dirty);
    assert_eq!(p.do_message(123,0,true,&mut globals,&mut h),Ok(true));
}
