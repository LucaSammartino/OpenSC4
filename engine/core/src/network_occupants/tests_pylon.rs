use super::pylon::*;
use crate::{automata::OccupantBase, stream::{ByteReader, ByteWriter, InStream, OutStream, RefValue, SaveRegistry}};
use std::{cell::RefCell, rc::Rc};

pub(super) fn reference(token: u64) -> RefValue { RefValue::Host { token, class_id: 0x1234 } }
pub(super) fn token(value: &RefValue) -> u64 { match value { RefValue::Host { token,.. } => *token, RefValue::Null => 0, _ => panic!("unexpected core ref") } }
pub(super) fn writer() -> ByteWriter { ByteWriter::with_registry(Rc::new(RefCell::new(SaveRegistry::new()))) }

/// Independent stream adapter records requested IIDs and deliberately resolves
/// references without imposing an owner-specific interface policy.
pub(super) struct Reader<'a> { pub inner: ByteReader<'a>, pub iids: Vec<u32> }
impl<'a> Reader<'a> { pub fn new(bytes: &'a [u8]) -> Self { Self { inner: ByteReader::new(bytes), iids: Vec::new() } } }
impl InStream for Reader<'_> {
    fn get_u8(&mut self)->Option<u8> { self.inner.get_u8() }
    fn get_u16(&mut self)->Option<u16> { self.inner.get_u16() }
    fn get_u32(&mut self)->Option<u32> { self.inner.get_u32() }
    fn get_u64(&mut self)->Option<u64> { self.inner.get_u64() }
    fn get_f32(&mut self)->Option<f32> { self.inner.get_f32() }
    fn get_f64(&mut self)->Option<f64> { self.inner.get_f64() }
    fn get_bytes(&mut self,n:usize)->Option<Vec<u8>> { self.inner.get_bytes(n) }
    fn get_str(&mut self)->Option<Vec<u8>> { self.inner.get_str() }
    fn get_ref(&mut self)->Option<RefValue> {
        let id = self.get_u32()?;
        if id == 0 { return Some(RefValue::Null); }
        let class_id = self.get_u32()?;
        Some(RefValue::Host { token: u64::from(id)*100, class_id })
    }
    fn get_ref_as(&mut self,iid:u32)->Option<RefValue> { self.iids.push(iid); self.get_ref() }
    fn skip(&mut self,n:usize)->bool { self.inner.skip(n) }
    fn error(&self)->i32 { self.inner.error() }
    fn set_error(&mut self) { self.inner.set_error(); }
}

pub(super) struct Host {
    pub log: Vec<String>, pub renderer: bool, pub manager: bool,
    pub insert_manager: bool, pub prepare: bool, pub insert_renderer: bool,
    pub packs: Vec<RenderStateArgs>,
}
impl Default for Host {
    fn default()->Self { Self { log: Vec::new(), renderer: true, manager: true, insert_manager: true, prepare: true, insert_renderer: true, packs: Vec::new() } }
}
impl References for Host {
    fn add_ref(&mut self,v:&RefValue) { self.log.push(format!("add{}",token(v))); }
    fn release(&mut self,v:&RefValue) { self.log.push(format!("release{}",token(v))); }
}
impl PylonHost for Host {
    fn occupant_init(&mut self,_:&mut OccupantBase) { self.log.push("init".into()); }
    fn occupant_shutdown(&mut self,_:&mut OccupantBase) { self.log.push("shutdown".into()); }
    fn message(&mut self,id:u32,subscribe:bool) { self.log.push(format!("message{id:x}/{subscribe}")); }
    fn animation(&mut self,class:u32,iid:u32,tick:u32,subscribe:bool) {
        assert_eq!((class,iid,tick),(ANIMATION_TICK_MANAGER_CLASS,ANIMATION_TICK_MANAGER_IID,0x80));
        self.log.push(format!("tick/{subscribe}"));
    }
    fn renderer_exists(&self)->bool { self.renderer }
    fn renderer_probe(&mut self) { self.log.push("probe".into()); }
    fn pack(&mut self,args:RenderStateArgs)->u64 { self.packs.push(args); self.packs.len() as u64 }
    fn city_manager(&mut self)->bool { self.log.push("city".into()); self.manager }
    fn manager_insert(&mut self)->bool { self.log.push("manager+".into()); self.insert_manager }
    fn manager_remove(&mut self) { self.log.push("manager-".into()); }
    fn prepare_instance(&mut self,arg:u32)->bool { assert_eq!(arg,4); self.log.push("prepare".into()); self.prepare }
    fn renderer_insert(&mut self)->bool { self.log.push("renderer+".into()); self.insert_renderer }
    fn renderer_remove(&mut self) { self.log.push("renderer-".into()); }
    fn base_visibility(&mut self,visible:bool,arg:u32,_:&mut u8) { self.log.push(format!("visibility/{visible}/{arg}")); }
    fn release_texture(&mut self,id:u32) { self.log.push(format!("texture{id}")); }
    fn texture_manager_exists(&self)->bool { true }
}

#[test]
fn pylon_record_bytes_one_transform_signed_enums_and_iids() {
    let mut r = PylonRecord::new([-1,-128,127]);
    r.call_number = 0x1234; r.source = reference(1); r.owner = reference(2);
    r.bitmap = -7; r.text = b"hello".to_vec(); r.drag = 0x80;
    r.transform.mask = 1; r.transform.translation = [4.0,5.0,6.0];
    let mut w = writer(); assert!(r.write(&mut w)); let bytes = w.into_bytes();
    let mut peek = ByteReader::new(&bytes); assert_eq!(peek.get_u16(),Some(5));
    OccupantBase::read(&mut peek).unwrap(); let body = peek.position();
    assert_eq!(&bytes[body..body+7], &[1,255,128,127,0,0x34,0x12]);
    let mut back = PylonRecord::new([99;3]); let mut h = Host::default(); let mut input = Reader::new(&bytes);
    assert!(back.read_into(&mut input,&mut h)); assert_eq!(input.inner.remaining(),0);
    assert_eq!(input.iids, [0x26BB_767E,1]);
    assert_eq!(back.states,[-1,-128,127]); assert_eq!(back.call_number,0x1234);
    assert_eq!(back.transform.translation,[4.0,5.0,6.0]); assert_eq!(back.drag,1);
    assert_eq!((back.bitmap,back.text),(-7,b"hello".to_vec()));
    // Force failure in the reached body. Both old refs are still cleared;
    // a base/version failure clears neither.
    for end in [body+7,bytes.len()-1] {
        let mut back = PylonRecord::new([0;3]); back.source=reference(8); back.owner=reference(9);
        let mut h = Host::default();
        assert!(!back.read_into(&mut Reader::new(&bytes[..end]),&mut h));
        assert_eq!(h.log,["release8","release9"]);
    }
    for bytes in [&[2,0][..], &bytes[..3]] {
        let mut back = PylonRecord::new([0;3]); back.source=reference(8);
        let mut h = Host::default(); assert!(!back.read_into(&mut Reader::new(bytes),&mut h)); assert!(h.log.is_empty());
    }
}

#[test]
fn pylon_v3_v4_future_and_tail_preservation() {
    for version in [3u16,4,6] {
        let mut w = ByteWriter::new(); w.put_u16(version); OccupantBase::new(0xA9B6_D7DE).write(&mut w);
        w.put_bytes(&[0,255,128,127,0]); w.put_u16(11); w.put_u32(0);
        if version == 3 {
            for v in super::xform::IDENTITY_MATRIX { w.put_f32(v); }
            for v in [4.0,5.0,6.0,1.0] { w.put_f32(v); }
            w.put_u32(0);
        } else { w.put_u8(1); for v in [4.0,5.0,6.0] { w.put_f32(v); } }
        w.put_u8(0x80);
        if version >= 5 { w.put_u32(7); w.put_str(b"new"); w.put_u32(0); }
        let bytes=w.into_bytes(); let mut r=PylonRecord::new([0;3]);
        r.bitmap=9; r.text=b"kept".to_vec(); r.owner=reference(8); let mut h=Host::default();
        assert!(r.read_into(&mut Reader::new(&bytes),&mut h));
        assert_eq!(r.transform.translation,[4.0,5.0,6.0]); assert_eq!(r.drag,1);
        if version<5 { assert_eq!(r.bitmap,9); assert_eq!(r.text,b"kept"); assert_eq!(token(&r.owner),8); assert!(h.log.is_empty()); }
        else { assert_eq!(r.bitmap,7); assert_eq!(h.log,["release8"]); }
    }
}

#[test]
fn pylon_setters_references_and_shared_blink_conformance_40_41() {
    let mut p=Pylon::new([2,1,2]); let mut q=Pylon::new([2,4,2]); let mut g=PylonGlobals::default();
    p.record.dirty=false; p.record.blink=true; p.set_ui_state(2);
    assert!(!p.record.dirty && p.record.blink); p.set_ui_state(300); assert!(p.record.dirty && !p.record.blink);
    for index in 1..3 {
        p.record.dirty=false; p.record.blink=true;
        let equal=p.record.states[index]; p.set_state_for_test(index,equal);
        assert!(!p.record.dirty && p.record.blink); p.set_state_for_test(index,-400);
        assert_eq!(p.record.states[index],-400); assert!(p.record.dirty && !p.record.blink);
    }
    p.record.dirty=false; p.set_bitmap_info(0,b""); assert!(p.record.dirty);
    p.set_call_number(0xFFFF); assert_eq!(p.get_call_number(),0xFFFF); p.set_drag_state(7); assert_eq!(p.get_drag_state(),7);
    let mut h=Host::default(); p.record.dirty=false; p.record.blink=true;
    p.set_source_occupant(reference(1),&mut h); p.set_source_occupant(reference(2),&mut h);
    p.set_source_occupant(reference(2),&mut h); p.set_source_occupant(RefValue::Null,&mut h);
    p.set_owner(reference(1),&mut h); p.set_owner(reference(2),&mut h); p.set_owner(reference(2),&mut h); p.set_owner(RefValue::Null,&mut h);
    assert_eq!(h.log,["add1","add2","release1","release2","add1","add2","release1","release2"]);
    assert!(!p.record.dirty && p.record.blink);
    p.record.blink=false; p.set_dispatch_state(1); p.record.dirty=false;
    p.animation_tick(300,&mut g); q.animation_tick(250,&mut g);
    assert_eq!(g.blink_accumulator,0); assert!(!p.record.blink && q.record.blink && q.record.dirty);
    p.animation_tick(1200,&mut g); assert!(p.record.blink); assert_eq!(g.blink_accumulator,0);
    p.set_dispatch_state(2); p.animation_tick(500,&mut g); assert_eq!(g.blink_accumulator,0);
    p.set_dispatch_state(1); g.blink_accumulator=0xFFFF_FFF0; p.animation_tick(32,&mut g);
    assert_eq!(g.blink_accumulator,16); assert!(!p.record.blink);
}
impl Pylon {
    fn set_state_for_test(&mut self,index:usize,value:i32) { match index { 1=>self.set_dispatch_state(value),2=>self.set_dispatch_type(value),_=>unreachable!() } }
}

#[test]
fn pylon_lifecycle_packing_and_all_placement_edges() {
    let mut p=Pylon::new([0;3]); let mut h=Host::default(); let mut g=PylonGlobals::default();
    assert_eq!(p.auxiliary,[[1.0,1.0,1.0,0.0,0.0,0.0];2]);
    assert!(p.init(&mut g,&mut h)); assert_eq!(h.packs,PYLON_RENDER_STATES); let count=h.log.len();
    assert!(p.init(&mut g,&mut h)); assert_eq!(h.log.len(),count);
    p.placement_flags=0x80; h.log.clear(); assert!(p.place_in_renderer(true,&mut h));
    assert_eq!(h.log,["city","city","manager+","prepare","renderer+"]); assert_eq!(p.placement_flags,0x83);
    h.log.clear(); assert!(p.place_in_manager(false,&mut h));
    assert_eq!(h.log,["city","renderer-","manager-"]); assert_eq!(p.placement_flags,0x80);
    h.insert_renderer=false; assert!(!p.place_in_renderer(true,&mut h)); assert_eq!(p.placement_flags,0x81);
    p.placement_flags=0x83; h.renderer=false; assert!(!p.place_in_renderer(false,&mut h)); assert_eq!(p.placement_flags,0x83);
    assert!(p.place_in_manager(false,&mut h)); assert_eq!(p.placement_flags,0x80);
    h.manager=false; assert!(p.place_in_manager(false,&mut h)); assert!(p.place_in_renderer(false,&mut h));
    assert!(!p.place_in_manager(true,&mut h)); h.renderer=true; assert!(!p.place_in_renderer(true,&mut h));
    h.manager=true; h.insert_manager=false; assert!(!p.place_in_manager(true,&mut h));
    h.insert_manager=true; h.prepare=false; assert!(!p.place_in_renderer(true,&mut h)); assert_eq!(p.placement_flags,0x81);
    h.prepare=true; h.insert_renderer=true; assert!(p.place_in_renderer(true,&mut h));
    assert!(p.place_in_renderer(false,&mut h)); assert_eq!(p.placement_flags,0x81);
    p.record.source=reference(1); p.record.owner=reference(2); p.mysim_texture=reference(3); p.texture_binding=reference(4); p.texture=9;
    h.log.clear(); assert!(p.shutdown(&mut h));
    assert_eq!(&h.log[h.log.len()-4..],["release3","release4","release1","release2"]);
    assert!(h.log.iter().any(|s|s=="tick/false")); let count=h.log.len(); assert!(p.shutdown(&mut h)); assert_eq!(h.log.len(),count);
    p.initialized=true; p.texture=9; h.log.clear(); p.shutdown(&mut h); assert!(h.log.iter().any(|s|s=="texture9"));
}

#[test]
fn pylon_position_visibility_and_explicit_model_gap_conformance_42() {
    let mut p=Pylon::new([0;3]); let mut h=Host::default();
    assert_eq!(p.set_position(None,&mut h),Ok(())); assert!(h.log.is_empty());
    p.record.transform.counter=255; p.record.transform.mask=0x80;
    assert_eq!(p.set_position(Some([4.0,5.0,6.0]),&mut h),Err(PylonUnsupported::ModelTextureGeometryDrawBoundsTransformMessagesNO36));
    assert_eq!((p.record.transform.counter,p.record.transform.mask),(0,0x81));
    assert_eq!(h.log,["city","city","city","manager+","city","city","prepare","renderer+"]);
    let mut out=[0.0;3]; p.get_position(Some(&mut out)); p.get_position(None); assert_eq!(out,[4.0,5.0,6.0]);
    h.log.clear(); p.set_visibility(false,7,&mut h); assert_eq!(h.log,["visibility/false/7","city","renderer-"]);
    p.placement_flags=0; h.log.clear(); p.set_visibility(true,8,&mut h); assert_eq!(h.log,["visibility/true/8"]);
}
