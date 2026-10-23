// SPDX-License-Identifier: GPL-3.0-or-later
//! Recording foreign-device conformance vectors from renderer-driver §15.
use super::*;
use crate::gdrivers_director::{Allocation,Host,Op,Result as HostResult};
use std::collections::{BTreeMap,VecDeque};
#[derive(Debug,Clone)]
struct Record { target:u32, call:Call, args:Vec<u32>, data:Vec<u8> }
struct Recorder {
    calls:Vec<Record>, refs:Vec<(u32,Op)>, failures:VecDeque<(Call,u32)>, memory:BTreeMap<u32,u8>,
    next:u32, viewport:[u32;6], states:BTreeMap<u32,u32>, rect:[u32;4], modes:Vec<[u32;3]>,
}
impl Default for Recorder {
    fn default()->Self {Self {calls:Vec::new(),refs:Vec::new(),failures:VecDeque::new(),memory:BTreeMap::new(),next:0x100000,
        viewport:[0,0,800,600,0,1f32.to_bits()],states:BTreeMap::new(),rect:[0,0,800,600],modes:vec![[640,480,16],[800,600,32]]}}
}
impl Host for Recorder {
    fn alloc(&mut self,_bytes:u32,_kind:Allocation)->u32 {self.next+=0x10000;self.next}
    fn free(&mut self,_address:u32,_kind:Allocation) {}
    fn read(&mut self,address:u32,bytes:&mut [u8])->HostResult<()> {for (i,b) in bytes.iter_mut().enumerate() {*b=*self.memory.get(&(address+i as u32)).unwrap_or(&0);}Ok(())}
    fn write(&mut self,address:u32,bytes:&[u8])->HostResult<()> {for (i,b) in bytes.iter().enumerate() {self.memory.insert(address+i as u32,*b);}Ok(())}
    fn call(&mut self,target:u32,op:Op,_args:&[u32],_data:&[u8],_out:&mut [u32])->u32 {self.refs.push((target,op));0}
    fn enumerate_child(&mut self,_target:u32,_context:u32,_callback:&mut dyn FnMut(u32,u32,u32)->bool) {}
}
impl Device for Recorder {
    fn dx(&mut self,target:u32,call:Call,args:&[u32],data:&[u8],out:&mut [u32])->u32 {
        self.calls.push(Record{target,call,args:args.to_vec(),data:data.to_vec()});
        if self.failures.front().is_some_and(|(op,_)|*op==call) {return self.failures.pop_front().unwrap().1;}
        match call {
            Call::GetViewport=>out.copy_from_slice(&self.viewport),Call::Viewport=>self.viewport.copy_from_slice(args),
            Call::RenderState=>{self.states.insert(args[0],args[1]);},Call::GetRenderState=>out[0]=*self.states.get(&args[0]).unwrap_or(&0),
            Call::GetCaps=>{for (o,v) in out.iter_mut().zip([0x80000,1,2,0x80,1]) {*o=v;}},
            Call::PixelFormat=>out.copy_from_slice(&[0xff000000,0xff0000,0xff00,0xff]),
            Call::ClientRect=>out.copy_from_slice(&self.rect),Call::ClientToScreen=>{out[0]=args[0]+50;out[1]=args[1]+70;},
            Call::EnumerateModes=>if args.is_empty() {out[0]=self.modes.len() as u32;} else {out.copy_from_slice(&self.modes[args[0] as usize]);},
            Call::LockSurface=>out.copy_from_slice(&[0x8000,256,32,0,0]),
            _=>if !out.is_empty() {self.next+=0x10000;out[0]=self.next;},
        }
        0
    }
    fn unresolved(&mut self,question:&'static str,args:&[u32],out:&mut [u32])->Result<()> {
        // Explicit test inputs at open proof boundaries, never claimed as native defaults.
        match question {
            "RD9-stage-defaults"=>{out[3..7].copy_from_slice(&[0,0,3,3]);},
            "RD8"|"RD8-restore"=>{for o in out {self.next+=0x10000;*o=self.next;}},
            "RD6"=>{if args.len()==1 {for o in out {*o=1;}}},
            _=>return Err(Error::Unsupported(question)),
        }
        Ok(())
    }
}
fn fixture()->(Driver,Recorder) {
    let mut h=Recorder::default();let mut d=Driver::new(0x100,0x200);
    d.device=10;d.context.dd=11;d.context.d3d=12;d.context.device=10;d.context.target=13;d.context.primary=14;d.context.window=15;d.context.active=true;
    d.ready=true;d.window=15;d.current.words[0]=0;d.current.set_byte(0x21,1);d.current.set_byte(0x29,1);d.full_viewport=h.viewport;
    d.stages=vec![Stage{parameters:[0,0,3,3],..Stage::default()};2];
    d.textures.dd=11;d.textures.device=10;d.textures.formats=Some([texture::Format{words:[0,0,0,0,0,0,32,0]};8]);d.textures.converters=Some([[true;8];15]);
    d.textures.new_name();d.primitives.init(11,12,10,true,&mut h);d.interleaved_arrays(11,32,0x4000);h.calls.clear();(d,h)
}
fn upload(name:u32)->Upload {Upload{name,level:0,x:0,y:0,width:2,height:2,format:3,kind:0,row_length:0,pixels:0x5000}}
fn positions(h:&Recorder,call:Call)->Vec<usize> {h.calls.iter().enumerate().filter_map(|(i,c)|(c.call==call).then_some(i)).collect()}

#[test]
fn dx7_01_02_04_noops_and_identity() {
    let mut d=Driver::new(0x100,0x200);let mut b=true;let mut f=3.25;d.error=6;
    d.get_boolean(0,&mut b);d.get_float(0,&mut f);assert!(b);assert_eq!(f,3.25);assert_eq!(d.tail(11,22),0);assert_eq!(d.error,6);
    let mut out=0xfeed;assert!(!d.query_interface(42,Some(&mut out)).unwrap());assert_eq!(out,0xfeed);assert_eq!(d.refs,0);
    assert!(d.query_interface(0xa4554849,Some(&mut out)).unwrap());assert_eq!(out,0x10c);assert_eq!(d.refs,1);
    assert!(d.query_interface(0x09cd86f9,Some(&mut out)).unwrap());assert_eq!(d.refs,1);assert_eq!(d.primitives.refs,1);
}
#[test]
fn dx7_03_09_11_loss_restore_metadata_and_defaults() {
    let (mut d,mut h)=fixture();assert!(d.is_device_ready());
    let n=d.create_texture(4,8,8,2,0,&mut h).unwrap();d.state(16,&[7],&mut h).unwrap();assert_eq!(h.states[&23],8);
    h.failures.push_back((Call::CooperativeLevel,FAIL));d.flush(&mut h).unwrap();assert!(!d.is_device_ready());assert!(d.is_texture(n));assert_eq!(d.textures.surface(n),0);
    let mut resident=[false];assert!(d.resident(&[n],&mut resident));assert!(resident[0]);
    d.flush(&mut h).unwrap();assert!(d.is_device_ready());assert_ne!(d.textures.surface(n),0);assert_eq!(h.states[&23],4);
    let messages:Vec<_>=h.calls.iter().filter(|c|c.call==Call::PostMessage).map(|c|c.args[0]).collect();assert_eq!(messages,[0xaacb92a3,0x8acaa95f,0x6acb92a9]);
}
#[test]
fn dx7_05_deferred_binding_and_presentation_order() {
    let (mut d,mut h)=fixture();d.scene=true;d.draw_arrays(0,0,3,&mut h).unwrap();
    let n=d.create_texture(4,2,2,1,0,&mut h).unwrap();d.bind_texture(0,n).unwrap();d.capability(5,true,&mut h).unwrap();
    assert!(positions(&h,Call::DrawBuffer).is_empty());d.draw_arrays(0,0,3,&mut h).unwrap();d.flush(&mut h).unwrap();
    let draws=positions(&h,Call::DrawBuffer);let texture=positions(&h,Call::Texture)[0];let end=positions(&h,Call::EndScene)[0];let present=positions(&h,Call::Blt)[0];
    assert!(draws[0]<texture&&texture<draws[1]&&draws[1]<end&&end<present);
}
#[test]
fn dx7_06_combiner_cache_and_min_filter_fallthrough() {
    let (mut d,mut h)=fixture();let n=d.create_texture(4,2,2,1,0,&mut h).unwrap();d.bind_texture(0,n).unwrap();d.capability(5,true,&mut h).unwrap();
    let a=[0,0,1,2,0,0,1,2,0,0];d.set_combiner(&a,0,&mut h).unwrap();d.tex_parameter(1,5).unwrap();d.draw_arrays(0,0,0,&mut h).unwrap();
    let filters:Vec<_>=h.calls.iter().filter(|c|c.call==Call::StageState&&c.args[0]==0&&matches!(c.args[1],17|18)).map(|c|(c.args[1],c.args[2])).collect();assert_eq!(filters,[(17,2),(18,2),(17,1),(18,3)]);
    h.calls.clear();let mut b=a;b[0]=2;d.set_combiner(&b,0,&mut h).unwrap();d.draw_arrays(0,0,0,&mut h).unwrap();
    assert!(!h.calls.iter().any(|c|c.call==Call::StageState&&c.args[1]==1));
    d.combine_field(57,0,2).unwrap();d.draw_arrays(0,0,0,&mut h).unwrap();assert!(h.calls.iter().any(|c|c.call==Call::StageState&&c.args==[0,1,7]));
}
#[test]
fn dx7_07_08_follow_parameters_and_lifo() {
    let (mut d,mut h)=fixture();let names=d.gen_textures(2);let (a,b)=(names[0],names[1]);
    d.bind_texture(0,a).unwrap();d.tex_parameter(0,1).unwrap();assert_eq!(d.textures.get(a).unwrap().parameters[0],1);
    d.set_texture(a,0).unwrap();assert!(d.follow_texture);d.set_texture(b,0).unwrap();d.tex_parameter(0,7).unwrap();assert_eq!(d.textures.get(b).unwrap().parameters[0],0);
    d.textures.priority(a,0.95);d.textures.create(a,3,2,7,9,5,&mut h).unwrap();d.delete_textures(&[a,b,0],&mut h);assert!(d.is_texture(0));
    assert_eq!(d.gen_textures(2),[b,a]);let r=d.textures.get(a).unwrap();assert_eq!((r.width,r.height,r.levels,r.format),(0,0,5,2));assert_eq!(r.priority,0.95);assert_eq!(r.parameters[0],1);
}
#[test]
fn dx7_10_texture_upload_flush_boundary() {
    let (mut d,mut h)=fixture();let n=d.create_texture(4,2,2,1,0,&mut h).unwrap();d.bind_texture(0,n).unwrap();d.draw_arrays(0,0,3,&mut h).unwrap();
    h.calls.clear();d.tex_image(upload(n),4,&mut h).unwrap();assert!(positions(&h,Call::DrawBuffer).is_empty());assert_eq!(positions(&h,Call::LockSurface).len(),1);
    h.calls.clear();d.load_texture_level(upload(n),&mut h).unwrap();assert!(positions(&h,Call::DrawBuffer)[0]<positions(&h,Call::LockSurface)[0]);
}
#[test]
fn dx7_12_flip_fallback() {
    let mut h=Recorder::default();h.failures.extend([(Call::CreateSurface,FAIL),(Call::CreateSurface,FAIL)]);
    let (mut c,hr)=Context::create(Request{selector:u32::MAX,fullscreen:true,flip:1,window:20,width:800,height:600,back_buffers:2},&mut h).unwrap();assert_eq!(hr,0);
    let counts:Vec<_>=h.calls.iter().filter(|c|c.call==Call::CreateSurface).map(|c|c.args[2]).collect();assert_eq!(counts,[2,1,0]);assert!(!c.flipping);c.present(0,&mut h);assert!(positions(&h,Call::Flip).is_empty());c.destroy(&mut h);
}
#[test]
fn dx7_13_14_present_retry_and_empty_rect() {
    let (d,mut h)=fixture();h.failures.push_back((Call::Blt,FAIL));assert_eq!(d.context.present(0,&mut h),0);
    let blits:Vec<_>=h.calls.iter().filter(|c|c.call==Call::Blt).collect();assert_eq!(blits.len(),2);assert_eq!(&blits[0].args[1..5],&[50,70,850,670]);assert_eq!(blits[0].args[5],0);assert_eq!(blits[0].args[6],0x01000200);assert_eq!(blits[1].args[6],0x01000000);assert_eq!(blits[0].target,14);
    h.calls.clear();h.rect[2]=0;assert_eq!(d.context.present(0,&mut h),LOST);assert!(positions(&h,Call::Blt).is_empty());
}
#[test]
fn dx7_15_modes_and_descriptor_enrichment() {
    let mut d=Driver::new(0x100,0x200);let mut h=Recorder::default();assert_eq!(d.count_modes(&mut h),4);
    assert_eq!(d.modes.iter().map(|m|m.byte(0x20)).collect::<Vec<_>>(),[1,0,1,0]);let mut before=[0;13];d.mode_info(0,&mut before);assert_eq!(&before[4..8],&[0;4]);
    d.textures.formats=Some([texture::Format::default();8]);d.textures.converters=Some([[true;8];15]);
    d.set_video_mode(0,0,false,0x9000,&mut h).unwrap();let mut after=[0;13];d.mode_info(0,&mut after);assert_ne!(&after[4..8],&[0;4]);assert_eq!(d.current.byte(0x21),1);assert_eq!(d.stages.len(),2);assert_eq!(after.len()*4,52);
}
#[test]
fn dx7_16_buffer_reservation_and_reference_order() {
    let mut h=Recorder::default();let mut b=primitive::VertexBuffer{format:11,stride:32,flags:0x10000,..Default::default()};assert_eq!(b.recreate(4,11,12,&mut h),0);
    assert!(b.reserve(3,false,11,12,&mut h).is_some());assert!(b.reserve(1,false,11,12,&mut h).is_none());b.unlock(11,12,&mut h);
    assert!(b.reserve(2,false,11,12,&mut h).is_none());assert!(b.reserve(2,true,11,12,&mut h).is_some());let locks:Vec<_>=h.calls.iter().filter(|c|c.call==Call::LockVertexBuffer).map(|c|c.args[0]).collect();assert_eq!(locks,[0x1021,0x2021]);
    let mut a=primitive::VertexBuffer{token:123,..Default::default()};h.refs.clear();a.assign(&b,&mut h);assert_eq!(h.refs,[(b.token,Op::AddRef),(123,Op::Release)]);
}
#[test]
fn dx7_17_sticky_hresult_channel() {
    let (mut d,mut h)=fixture();for hr in [LOST,FAIL] {h.failures.push_back((Call::RenderState,hr));d.state(67,&[1],&mut h).unwrap();assert_eq!(d.get_error(),u32::from(hr!=LOST));assert_eq!(d.get_error(),0);}
    d.error=6;d.capability(0,false,&mut h).unwrap();assert_eq!(d.error,0);
}
#[test]
fn dx7_18_failed_setup_writes_fourth_word_after_cleanup() {
    let mut h=Recorder::default();let mut d=Driver::new(0x100,0x200);d.count_modes(&mut h);h.failures.push_back((Call::LoadLibrary,FAIL));
    d.set_video_mode(0,0,false,0x9000,&mut h).unwrap();assert_eq!(d.error,6);assert_eq!(d.window,0);let mut bytes=[0;4];h.read(0x9000,&mut bytes).unwrap();assert_eq!(u32::from_le_bytes(bytes),u32::MAX);assert_eq!(positions(&h,Call::DestroyWindow).len(),1);
}
#[test]
fn matrices_viewport_formats_and_draw_patterns() {
    assert_eq!(vertex::make(&[0,5,7]).unwrap(),0x80004101);assert_eq!(vertex::stride(11),32);assert_eq!(vertex::elements(0x80008101,7).unwrap(),2);assert_eq!(vertex::offset(11,7,1).unwrap(),24);
    assert_eq!(primitive::expanded(1,&[0,1,2,3]).unwrap().1.unwrap(),[0,1,2,2,1,3]);
    assert_eq!(primitive::expanded(6,&[0,1,2,3]).unwrap().1.unwrap(),[0,1,2,0,2,3]);
    let (mut d,mut h)=fixture();d.matrix_mode=1;d.load_identity(&mut h);
    let m=&h.calls.iter().find(|c|c.call==Call::Transform).unwrap().data;
    let values:Vec<_>=m.chunks(4).map(|b|f32::from_le_bytes(b.try_into().unwrap())).collect();assert_eq!(values[12],-1./800.);assert_eq!(values[13],1./600.);assert_eq!(values[14],0.5);assert_eq!(values[10],-0.5);
    d.set_viewport(Some([100,50,200,100]),&mut h);assert_eq!(&h.viewport[..4],&[100,450,200,100]);assert_eq!(d.get_viewport(&mut h),[100,50,300,150]);d.lost=true;assert_eq!(d.get_viewport(&mut h),[0;4]);
}
#[test]
fn defaults_clear_quantization_and_stencil_failures() {
    let (mut d,mut h)=fixture();d.default_state(&mut h);let states:Vec<_>=h.calls.iter().filter(|c|c.call==Call::RenderState).map(|c|(c.args[0],c.args[1])).collect();assert_eq!(states.len(),26);assert_eq!(&states[12..19],&[(53,1),(54,6),(55,1),(56,8),(57,1),(58,1),(59,1)]);
    assert_eq!(color([1.,0.5,0.,1.]),0xffff8000);d.current.set_byte(0x21,0);d.clear(0x7000,&mut h);assert_eq!(h.calls.last().unwrap().args[0],3);
    let count=h.calls.len();d.state(18,&[0,1,1],&mut h).unwrap();assert_eq!(d.error,3);assert_eq!(h.calls.len(),count);
}
