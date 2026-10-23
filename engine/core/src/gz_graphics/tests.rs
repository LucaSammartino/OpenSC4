// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::{Arc, Mutex, atomic::{AtomicU32, Ordering}};

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Init, Shutdown, State(StateChange), Bind(Point,Point,bool), Flush, Suspend, Resume,
    Draw(u32,Vec<Point>), Generic(GeometryRoute,u32,Vec<Point>), Lock(u32), Unlock(u32),
    Graphics(GraphicsCall), Canvas(CanvasCall), Device(&'static str),
}
type Log = Arc<Mutex<Vec<Event>>>;
fn log() -> Log { Arc::new(Mutex::new(Vec::new())) }
fn push(log: &Log, event: Event) { log.lock().unwrap().push(event); }
fn take(log: &Log) -> Vec<Event> { std::mem::take(&mut *log.lock().unwrap()) }

struct TestBuffer { log: Log, record: BufferRecord, lock_ok: bool, pixels: Mutex<Vec<u8>> }
impl Buffer for TestBuffer {
    fn lock(&self, flags: u32) -> bool { push(&self.log, Event::Lock(flags)); self.lock_ok }
    fn unlock(&self, flags: u32) { push(&self.log, Event::Unlock(flags)); }
    fn record(&self) -> BufferRecord { self.record }
    fn native_rgb(&self, colour: u32) -> u32 { colour & 0x00ff_ffff }
    fn pixels(&self, record: BufferRecord, op: &mut dyn FnMut(&mut [u8], usize) -> Result<()>) -> Result<()> { assert_eq!(record,self.record); op(&mut self.pixels.lock().unwrap(),0) }
}
fn buffer(log: &Log, colour_type: u32, lock_ok: bool) -> Arc<TestBuffer> {
    Arc::new(TestBuffer { log: log.clone(), record: BufferRecord { pixels: 42, stride: 32, width: 8, height: 2, colour_type, bytes_per_pixel: 4 }, lock_ok, pixels: Mutex::new(vec![0;64]) })
}
struct TestTarget { area: Rect, buffer: Option<Arc<dyn Buffer>>, simgl: bool }
impl Target for TestTarget {
    fn area(&self) -> Rect { self.area }
    fn native_rgb(&self, colour: u32) -> u32 { colour & 0x00ff_ffff }
    fn buffer(&self) -> Option<Arc<dyn Buffer>> { self.buffer.clone() }
    fn supports_simgl(&self) -> bool { self.simgl }
}
fn target(area: Rect, buffer: Option<Arc<dyn Buffer>>) -> Arc<dyn Target> { Arc::new(TestTarget { area, buffer, simgl: false }) }
struct Recorder { log: Log, area: Rect, code: Arc<AtomicU32> }
impl RenderEngine for Recorder {
    fn init(&mut self, _: &Arc<dyn Target>) -> bool { push(&self.log,Event::Init); true }
    fn shutdown(&mut self) { push(&self.log,Event::Shutdown); }
    fn canvas(&self) -> Rect { self.area }
    fn state(&mut self, c: StateChange) -> Result<()> { push(&self.log,Event::State(c)); Ok(()) }
    fn bind(&mut self, b: BlitBinding) -> Result<Option<Rect>> { let active = b.source.is_some(); push(&self.log,Event::Bind(b.placement,b.scale,active)); Ok(active.then_some(Rect(2,3,6,7))) }
    fn flush(&mut self) -> Result<()> { push(&self.log,Event::Flush); Ok(()) }
    fn suspend(&mut self) -> Result<()> { push(&self.log,Event::Suspend); Ok(()) }
    fn resume(&mut self) -> Result<()> { push(&self.log,Event::Resume); Ok(()) }
    fn lock(&self) -> Result<BufferRecord> { Ok(BufferRecord::default()) }
    fn unlock(&mut self) -> Result<()> { Ok(()) }
    fn draw(&mut self, k:u32, p:&[Point]) -> Result<u32> { push(&self.log,Event::Draw(k,p.to_vec())); Ok(self.code.load(Ordering::Relaxed)) }
    fn generic(&mut self, route:GeometryRoute, k:u32, p:&[Point]) -> Result<()> { push(&self.log,Event::Generic(route,k,p.to_vec())); Ok(()) }
}
struct Factory { log: Log, code: Arc<AtomicU32> }
impl EngineFactory for Factory {
    fn create(&mut self, t:&Arc<dyn Target>) -> Result<Box<dyn RenderEngine>> { Ok(Box::new(Recorder { log:self.log.clone(), area:t.area(), code:self.code.clone() })) }
}
fn setup() -> (DrawContext,Arc<dyn Target>,Factory) {
    let log = log(); let code = Arc::new(AtomicU32::new(2));
    let mut f = Factory { log, code };
    let t = target(Rect(0,0,100,80),None);
    let mut c = DrawContext::new([0xaabbccdd;4]); c.set_target(Some(&t),&mut f).unwrap();
    (c,t,f)
}
#[test]
fn gzg_v01_query_and_reference_lifetimes() {
    let a = Arc::new(1); let sentinel = Arc::new(2); let mut out = Some(sentinel.clone());
    assert!(!query(&a,9,CONTEXT_IIDS,&mut out)); assert!(Arc::ptr_eq(out.as_ref().unwrap(),&sentinel));
    assert!(query(&a,1,CONTEXT_IIDS,&mut out)); assert_eq!(Arc::strong_count(&a),2);
    out = None; assert!(out.is_none()); assert_eq!(Arc::strong_count(&a),1);
    for (ids,iid) in [(ENGINE_IIDS,0x2ae78a07),(GRAPHICS_IIDS,0x0073283c),(CANVAS_IIDS,0xe46397db),(ALPHA_IIDS,0xe3260321)] { assert!(query(&a,iid,ids,&mut out)); }
}
#[test]
fn gzg_v02_empty_intersection() { assert_eq!(Rect(0,0,10,10).intersect(Rect(10,2,20,8)),Rect::default()); }
#[test]
fn gzg_v03_v04_attachment_and_same_pointer() {
    let (mut c,t,mut f) = setup();
    assert_eq!(c.origin(),[0.,0.]); assert_eq!(c.clip_rect(),Rect(0,0,100,80));
    assert!(!c.blit_active() && !c.tiling() && !c.is_suspended());
    assert_eq!(take(&f.log),vec![Event::Init,Event::State(StateChange::Clip(Rect(0,0,100,80))),Event::State(StateChange::Colours([0xaabbccdd;4])),Event::State(StateChange::Alpha(0)),Event::State(StateChange::Modulation(0)),Event::State(StateChange::Antialias(0)),Event::State(StateChange::Thickness(1.)),Event::Bind([0.;2],[1.;2],false),Event::State(StateChange::Gradient(None,0,0))]);
    let count = Arc::strong_count(&t); c.set_target(Some(&t),&mut f).unwrap(); assert_eq!(count,Arc::strong_count(&t)); assert!(take(&f.log).is_empty());
    c.set_target(None,&mut f).unwrap(); assert_eq!(take(&f.log),vec![Event::Shutdown]); assert!(!c.is_target_attached()); assert_eq!(Arc::strong_count(&t),1);
}
#[test]
fn gzg_v05_v06_v07_v08_transforms_and_clip() {
    let (mut c,_,f) = setup(); take(&f.log);
    c.clip_absolute(Rect(0,0,20,20)).unwrap(); c.push_transform(); c.offset(5,7); c.clip(Rect(2,3,9,10)).unwrap();
    assert_eq!(c.clip_rect(),Rect(7,10,14,17)); assert_eq!(c.transform_depth(),1);
    take(&f.log); c.pop_transform().unwrap(); c.pop_transform().unwrap(); assert_eq!(c.origin(),[0.,0.]); assert_eq!(take(&f.log),vec![Event::State(StateChange::Clip(Rect(0,0,20,20)))]);
    c.offset(5,7); c.clip_absolute(Rect(2,3,9,10)).unwrap(); assert_eq!(c.clip_rect(),Rect(2,3,9,10));
    take(&f.log); c.clip_reset(); assert_eq!(c.clip_rect(),Rect(0,0,100,80)); assert!(take(&f.log).is_empty());
}
#[test]
fn gzg_v09_v10_state_stack_and_narrow_reset() {
    let (mut c,_,_) = setup(); c.push_state(); c.set_colour(0,123).unwrap(); c.offset(5,7);
    c.set_alpha(2).unwrap(); c.set_antialias(3).unwrap(); c.set_modulation(4).unwrap(); c.set_thickness(6.).unwrap();
    c.pop_state().unwrap(); c.pop_state().unwrap(); assert_eq!(c.attributes,Attributes::default()); assert_eq!(c.origin(),[5.,7.]); assert_eq!(c.colours[0],123);
    c.push_transform(); c.set_alpha(2).unwrap(); c.set_antialias(3).unwrap(); c.set_modulation(4).unwrap(); c.set_thickness(6.).unwrap(); c.reset_defaults().unwrap();
    assert_eq!(c.attributes,Attributes::default()); assert_eq!(c.origin(),[5.,7.]); assert_eq!(c.colours[0],123); assert_eq!(c.transform_depth(),1);
}
#[test]
fn gzg_v11_v12_setter_suppression_and_thickness() {
    let (mut c,_,f) = setup(); take(&f.log); c.set_antialias(1).unwrap(); c.set_antialias(1).unwrap();
    assert_eq!(take(&f.log),vec![Event::State(StateChange::Antialias(1))]);
    c.set_thickness(0.).unwrap(); assert_eq!((c.attributes.thickness,c.attributes.raster_thickness),(0.,1));
    c.set_thickness(1.6).unwrap(); assert_eq!((c.attributes.thickness,c.attributes.raster_thickness),(1.6,2));
    c.set_thickness(-2.).unwrap(); take(&f.log); c.set_thickness(-2.).unwrap(); assert_eq!(take(&f.log),vec![Event::State(StateChange::Thickness(0.))]);
    assert_eq!(c.set_thickness(f32::NAN),Err(Error::InvalidInput));
}
#[test]
fn gzg_v13_v14_v15_integer_geometry_and_clear() {
    let (mut c,_,f) = setup(); c.offset(10,20); take(&f.log);
    c.point(2,3).unwrap(); c.fill(Rect(2,3,6,8)).unwrap(); c.clip_absolute(Rect(1,2,7,9)).unwrap(); c.clear().unwrap();
    assert_eq!(take(&f.log),vec![Event::Generic(GeometryRoute::Outline,0,vec![[12.5,23.5]]),Event::Draw(3,vec![[12.,23.],[16.,28.]]),Event::Generic(GeometryRoute::Rasterize,3,vec![[12.,23.],[16.,28.]]),Event::State(StateChange::Clip(Rect(1,2,7,9))),Event::Draw(3,vec![[1.,2.],[7.,9.]]),Event::Generic(GeometryRoute::Rasterize,3,vec![[1.,2.],[7.,9.]])]);
}
#[test]
fn gzg_v16_suspension_is_nested_but_every_suspend_calls_engine() {
    let (mut c,_,f) = setup(); take(&f.log); c.suspend().unwrap(); c.suspend().unwrap(); c.resume().unwrap(); assert!(c.is_suspended()); c.resume().unwrap(); c.resume().unwrap(); assert!(!c.is_suspended());
    assert_eq!(take(&f.log),vec![Event::Flush,Event::Suspend,Event::Flush,Event::Suspend,Event::Resume]);
}
#[test]
fn gzg_v17_blit_effective_clip_is_separate() {
    let (mut c,t,f) = setup(); take(&f.log); c.offset(10,20); assert!(c.set_blit(t,[1.,2.],[2.,3.]).unwrap()); c.set_tiling([1,2,3,4]).unwrap(); c.clear_tiling().unwrap();
    assert_eq!(take(&f.log),vec![Event::Bind([11.,22.],[2.,3.],true),Event::State(StateChange::Clip(Rect(2,3,6,7))),Event::State(StateChange::Tiling(Some([1,2,3,4]))),Event::State(StateChange::Clip(Rect(0,0,100,80))),Event::State(StateChange::Tiling(None)),Event::State(StateChange::Clip(Rect(2,3,6,7)))]);
    assert_eq!(c.clip_rect(),Rect(0,0,100,80));
    c.clip_absolute(Rect(0,0,10,10)).unwrap(); assert_eq!(take(&f.log),vec![Event::State(StateChange::Clip(Rect(0,0,10,10)))]);
}
#[test]
fn gzg_v18_v19_buffer_query_and_lock_failures() {
    let log = log(); let mut e = BufferEngine::default(); assert!(!e.init(&target(Rect::default(),None))); assert!(take(&log).is_empty());
    let b = buffer(&log,5,false); let t = target(Rect::default(),Some(b.clone())); let count = Arc::strong_count(&b);
    assert!(!e.init(&t)); assert_eq!(Arc::strong_count(&b),count); assert_eq!(take(&log),vec![Event::Lock(0x8040)]); assert_eq!(e.lock(),Err(Error::Detached));
}
#[test]
fn gzg_v20_v21_v22_buffer_lifetime_format_and_allocation() {
    let log = log(); let b = buffer(&log,5,true); let t = target(Rect::default(),Some(b.clone())); let mut e = BufferEngine::default();
    assert!(e.init(&t)); assert_eq!(e.renderer,Some(SpanFormat::Rgb565)); assert_eq!(e.lock().unwrap(),b.record()); assert_eq!(Arc::strong_count(&b),3);
    e.shutdown(); assert_eq!(Arc::strong_count(&b),2); assert_eq!(take(&log),vec![Event::Lock(0x8040),Event::Unlock(0x8040)]);
    let unsupported = buffer(&log,3,true); assert!(!e.init(&target(Rect::default(),Some(unsupported.clone())))); assert_eq!(Arc::strong_count(&unsupported),1); assert_eq!(take(&log),vec![Event::Lock(0x8040)]);
    e.renderer_allocation_available = false; assert!(e.init(&t)); assert!(e.renderer.is_none()); assert_eq!(e.span_route(),Err(Error::Foreign)); e.shutdown();
}
#[test]
fn gzg_v23_v24_v25_buffer_aa_dispatch_plan_and_open_coverage() {
    let mut e = BufferEngine::default(); let p = [[1.,2.];7];
    assert_eq!(e.draw(6,&p),Ok(2)); assert!(e.plan_draw(6,&p).polygons.is_empty());
    e.state(StateChange::Antialias(1)).unwrap(); let plan = e.plan_draw(6,&p); assert_eq!(plan.result,0); assert_eq!(plan.polygons,vec![vec![[1.,2.];3];2]);
    assert_eq!(e.draw(6,&p),Err(Error::Unsupported("GZG2"))); assert_eq!(e.draw(3,&p),Ok(1));
    assert_eq!(e.plan_draw(7,&p).polygons,vec![vec![[1.,2.];4]]); assert_eq!(e.plan_draw(8,&p).polygons,vec![p.to_vec()]);
}
struct Device { log:Log, init_ok:bool }
impl HardwareDevice for Device {
    fn init(&mut self,_:&Arc<dyn Buffer>) -> bool { push(&self.log,Event::Device("init")); self.init_ok }
    fn apply_state(&mut self,_:&EngineState) -> Result<()> { push(&self.log,Event::Device("state")); Ok(()) }
    fn update_transform(&mut self,_:&EngineState) -> Result<()> { push(&self.log,Event::Device("transform")); Ok(()) }
    fn submit(&mut self,_:u32,_:&[Point],_:&EngineState) -> Result<()> { push(&self.log,Event::Device("submit")); Ok(()) }
    fn suspend(&mut self) -> Result<()> { Ok(()) }
    fn resume(&mut self) -> Result<()> { Ok(()) }
}
#[test]
fn gzg_v26_v27_hardware_failure_and_alpha_predicates() {
    let log = log(); let b = buffer(&log,5,true); let t = target(Rect::default(),Some(b.clone())); let mut e = HardwareEngine::new(Box::new(Device { log:log.clone(),init_ok:false }));
    assert!(!e.init(&t)); assert_eq!(Arc::strong_count(&b),2); assert_eq!(e.canvas(),Rect::default());
    for (mode,a,b) in [(0,false,false),(1,true,false),(2,true,true)] { e.state.dirty = false; e.state(StateChange::Alpha(mode)).unwrap(); assert_eq!((e.state.alpha,e.state.alpha_above_one,e.state.dirty),(a,b,true)); }
}
struct GHost { log:Log, service:Option<Arc<dyn DrawService>>, framework:bool, fail:Option<GraphicsCall> }
impl GraphicsHost for GHost {
    fn framework_available(&self) -> bool { self.framework }
    fn call(&mut self,c:GraphicsCall) -> bool { push(&self.log,Event::Graphics(c)); self.fail != Some(c) }
    fn draw_service(&self) -> Option<Arc<dyn DrawService>> { self.service.clone() }
}
struct Service(AtomicU32);
impl DrawService for Service { fn dispatch_slot9(&self) { self.0.fetch_add(1,Ordering::Relaxed); } }
fn graphics(log:&Log) -> GraphicSystem { GraphicSystem::new(Box::new(GHost { log:log.clone(), service:None, framework:true, fail:None })) }
#[test]
fn gzg_v28_graphics_dispatch_and_service_release() {
    let log = log(); let service = Arc::new(Service(AtomicU32::new(0))); let g = GraphicSystem::new(Box::new(GHost { log,service:Some(service.clone()),framework:true,fail:None }));
    assert!(g.do_message(99)); assert!(g.do_message(0x8acaa95f)); assert_eq!(service.0.load(Ordering::Relaxed),1); assert_eq!(Arc::strong_count(&service),2);
}
#[test]
fn gzg_v29_translucency_is_the_byte_not_device_capability() { let mut g = graphics(&log()); g.set_force_no_translucency(1,42); assert!(!g.translucency_enabled()); assert_eq!(g.translucency_word,42); g.set_force_no_translucency(0,77); assert!(g.translucency_enabled()); }
struct TestDriver { id:u32, modes:Vec<ScreenMode>, enumerations:AtomicU32 }
impl Driver for TestDriver { fn id(&self)->u32 { self.id } fn modes(&self)->Vec<ScreenMode> { self.modes.clone() } fn enumerate_setup(&self) { self.enumerations.fetch_add(1,Ordering::Relaxed); } }
fn screen(w:u32,h:u32,b:u32)->ScreenSetting { ScreenSetting { width:w,height:h,bits:b } }
#[test]
fn gzg_v30_v31_v32_screen_downgrade_reduces_before_testing() {
    let mut g = graphics(&log()); let d:Arc<dyn Driver> = Arc::new(TestDriver { id:1,modes:vec![ScreenMode { setting:screen(1024,768,16),discriminator:0 },ScreenMode { setting:screen(800,600,16),discriminator:0 }],enumerations:AtomicU32::new(0) }); g.drivers.push(d.clone()); g.set_driver(Some(&d));
    let mut s = screen(1024,768,32); assert!(g.downgrade(0,&mut s)); assert_eq!(s,screen(1024,768,16));
    assert!(g.downgrade(0,&mut s)); assert_eq!(s,screen(800,600,16)); s = screen(320,240,16); assert!(!g.downgrade(0,&mut s));
    g.discriminator = 1; s = screen(1024,768,32); assert!(!g.downgrade(0,&mut s)); assert_eq!(s,screen(320,240,16));
}
struct Delegate { calls:Vec<(AlphaOperation,Vec<u32>)> }
impl AlphaDelegate for Delegate { type Arguments = Vec<u32>; type Output = u32; fn invoke(&mut self,op:AlphaOperation,args:&mut Vec<u32>)->u32 { self.calls.push((op,args.clone())); 123 } }
struct FileIo(bool);
impl AlphaFileIo for FileIo { fn open(&mut self,_:&str)->bool { self.0 } fn close(&mut self) {} }
#[test]
fn gzg_v33_v34_v35_alpha_stateless_forwarding_and_open_failure() {
    let mut a = AlphaBlt { delegate:Delegate { calls:Vec::new() } }; assert_eq!(a.alpha_level_count(),32);
    let ops = [AlphaOperation::GenericBufferClip,AlphaOperation::InitAlphaSystem,AlphaOperation::AlphaBlt,AlphaOperation::AlphaBltNoClip,AlphaOperation::AlphaBltConstAlpha,AlphaOperation::AlphaBltConstAlphaNoClip,AlphaOperation::AlphaBltConstSource,AlphaOperation::AlphaBltConstSourceNoClip,AlphaOperation::ShadeBltConstAlpha,AlphaOperation::ShadeBltConstAlphaNoClip,AlphaOperation::ShadowBlitConstAlpha,AlphaOperation::ShadowBlitConstAlphaNoClip,AlphaOperation::BrightBlitConstAlpha,AlphaOperation::BrightBlitConstAlphaNoClip,AlphaOperation::ShadeBlt,AlphaOperation::ShadeBltNoClip,AlphaOperation::ShadeBltStretch,AlphaOperation::ShadeBltStretchNoClip,AlphaOperation::ShadeBltWithAdditionalAlpha,AlphaOperation::ShadeBltWithAdditionalAlphaNoClip,AlphaOperation::ShadeBltStretchWithAdditionalAlpha,AlphaOperation::ShadeBltStretchWithAdditionalAlphaNoClip,AlphaOperation::AlphaBltSelfAlpha,AlphaOperation::AlphaBltSelfAlphaNoClip,AlphaOperation::AlphaBltHalfAlphaVariableSource1ConstSource2,AlphaOperation::AlphaBltHalfAlphaVariableSource1ConstSource2NoClip];
    for op in ops { let mut args = vec![7,9,11]; assert_eq!(a.call(op,&mut args),123); assert_eq!(a.delegate.calls.last(),Some(&(op,vec![7,9,11]))); assert_eq!(args,vec![7,9,11]); }
    for mask in [false,true] { assert!(a.surface_from_file("absent",mask,&mut FileIo(false)).unwrap().is_none()); assert!(matches!(a.surface_from_file("present",mask,&mut FileIo(true)),Err(Error::Unsupported("GZG5")))); }
}
struct Window { log:Log, services:bool, realize:bool, post:bool, cp:u32 }
impl WindowHost for Window {
    fn call(&mut self,_:Option<usize>,c:CanvasCall)->bool { let post = matches!(c,CanvasCall::Post(_)); push(&self.log,Event::Canvas(c)); !post || self.post }
    fn locale_ansi_code_page(&self)->u32 { self.cp }
    fn services_available(&self)->bool { self.services }
    fn realize(&mut self,_:ScreenSetting,_:u32)->Result<usize> { if self.realize { Ok(123) } else { Err(Error::Foreign) } }
    fn register_wheel_message(&mut self)->Option<u32> { None }
}
fn canvas(log:&Log)->CanvasW32 { CanvasW32::new(Box::new(Window { log:log.clone(),services:true,realize:true,post:true,cp:0 })) }
#[test]
fn gzg_v36_v37_canvas_bounds_and_title_quirk() {
    let log = log(); let mut c = canvas(&log); c.handle = Some(123); c.set_bounds(Rect(10,20,110,70)); assert_eq!((c.width(),c.height()),(100,50)); assert_eq!(c.bounds,Rect(10,20,110,70));
    c.set_title(b"hello"); c.set_title(b""); assert!(c.title.is_empty()); assert_eq!(take(&log),vec![Event::Canvas(CanvasCall::Position { x:10,y:20,width:100,height:50,flags:0x14 }),Event::Canvas(CanvasCall::Title(b"hello\0".to_vec()))]);
}
#[test]
fn gzg_v38_v39_nested_channels_and_selector() {
    let log = log(); let mut c = canvas(&log); for (enabled,count) in [(false,1),(false,2),(true,1),(true,0),(true,0)] { c.enable_input(2,enabled).unwrap(); assert_eq!(c.input_counters()[1],count); }
    assert_eq!(take(&log),vec![Event::Canvas(CanvasCall::Input2Disabled(true)),Event::Canvas(CanvasCall::Input2Disabled(false))]); assert!(c.input_enabled(1) && c.input_enabled(2)); assert!(!c.input_enabled(3));
    c.enable_input(3,false).unwrap(); assert_eq!(c.input_counters(),[1,1]);
}
#[test]
fn gzg_v40_v41_v42_packages_and_messages() {
    let log = log(); let mut c = canvas(&log); assert!(!c.post_package(None)); assert!(!c.post_package(Some((1,&[1,2,3])))); assert!(!c.post_package(Some((0x8006,&[1,2])))); assert!(take(&log).is_empty());
    assert!(c.post_package(Some((0x8006,&[11,22,33,44])))); assert_eq!(take(&log),vec![Event::Canvas(CanvasCall::Post([11,22,33]))]);
    assert!(c.do_message(99,Some((0x1a0c615f,13,[7,8])))); assert_eq!(c.coordinates,[0,0]); assert!(c.do_message(0xfa0c6390,Some((0x1a0c615f,13,[7,8])))); assert_eq!(c.coordinates,[7,8]);
    c.do_message(0xfa0c6390,None); c.do_message(0xfa0c6390,Some((0x1a0c615f,12,[9,10]))); assert_eq!(c.coordinates,[7,8]);
}
#[test]
fn gzg_v43_v44_filled_result_switches() {
    let (mut c,_,f) = setup(); let p = [[1.,2.];8]; f.code.store(1,Ordering::Relaxed); take(&f.log);
    c.draw_no_transform(6,&p).unwrap(); c.draw_no_transform(7,&p).unwrap(); assert_eq!(take(&f.log),vec![Event::Draw(6,p.to_vec()),Event::Draw(7,p.to_vec()),Event::Generic(GeometryRoute::Tessellate,7,p.to_vec())]);
    f.code.store(2,Ordering::Relaxed); for k in [6,7,8] { c.draw_no_transform(k,&p).unwrap(); }
    assert_eq!(take(&f.log),[6,7,8].into_iter().flat_map(|k| [Event::Draw(k,p.to_vec()),Event::Generic(GeometryRoute::Rasterize,k,p.to_vec())]).collect::<Vec<_>>());
    f.code.store(99,Ordering::Relaxed); c.draw_no_transform(8,&p).unwrap(); assert_eq!(take(&f.log),vec![Event::Draw(8,p.to_vec())]); c.draw_no_transform(99,&p).unwrap(); assert!(take(&f.log).is_empty());
}
