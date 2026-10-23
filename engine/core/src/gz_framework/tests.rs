// SPDX-License-Identifier: GPL-3.0-or-later
//! Event-recorder conformance vectors GZC01–GZC38 (§10).
use super::*;
use crate::commands::{self,CommandServer,ValueConvert};
use crate::variant::{Elem,Variant};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{Arc,Mutex};

#[derive(Default)]
struct Spy {
    events: Vec<String>,
    counts: BTreeMap<Token,i32>,
    init_ok: bool,
    services: bool,
    exists: bool,
    vocab: BTreeMap<(Token,Vec<u8>),Variant>,
    parameters: Vec<(u32,Variant)>,
    nested: Option<bool>,
    resource: bool,
    stream: bool,
    setting: Option<String>,
}
impl References for Spy {
    fn add_ref(&mut self, t: Token) { *self.counts.entry(t).or_default() += 1; self.events.push(format!("+{t}")); }
    fn release(&mut self, t: Token) { *self.counts.entry(t).or_default() -= 1; self.events.push(format!("-{t}")); }
    fn query(&mut self, t: Token, iid: u32) -> Option<Token> {
        self.events.push(format!("qi:{t}:{iid:x}"));
        if iid == script::STREAM_IID && !self.stream { return None }
        self.add_ref(t); Some(t)
    }
}
impl framework::Host for Spy {
    fn prepare_com(&mut self) { self.events.push("prepare".into()); }
    fn register_com_service(&mut self) { self.events.push("com-service".into()); }
    fn setup_application(&mut self) { self.events.push("setup".into()); }
    fn init_application(&mut self) -> bool { self.events.push("app-init".into()); self.init_ok }
    fn init_framework(&mut self) -> bool { self.events.push("framework-init".into()); self.init_ok }
    fn run(&mut self) -> u32 { self.events.push("run".into()); 7 }
    fn shutdown(&mut self) { self.events.push("shutdown".into()); }
    fn post_quit(&mut self, result: u32) { self.events.push(format!("quit:{result}")); }
    fn abort_cleanup(&mut self) { self.events.push("abort".into()); }
    fn terminate(&mut self, result: u32) { self.events.push(format!("exit:{result}")); }
    fn enable(&mut self, value: bool) { self.events.push(format!("enable:{value}")); }
    fn exception_dialog(&mut self, text: &str) { self.events.push(text.into()); }
}
impl com::Host for Spy {
    fn acquire_framework(&mut self) -> Token { self.add_ref(10); 10 }
    fn acquire_director(&mut self) -> Token { self.add_ref(11); 11 }
    fn application_path(&mut self) -> String { "app/".into() }
    fn init_director(&mut self, d: Token, c: Token, path: &str) -> bool { self.events.push(format!("director:{d}:{c}:{path}")); self.init_ok }
    fn director_class(&mut self, _: Token, _: u32, _: u32) -> Option<Token> { self.events.push("director-class".into()); None }
    fn library_class(&mut self, l: Token, _: u32, _: u32) -> Option<Token> { self.events.push("library-class".into()); self.add_ref(l); Some(l) }
    fn load_library(&mut self, name: &str) -> Option<Token> { self.events.push(format!("load:{name}")); (name != "missing").then_some(12) }
    fn library_director(&mut self, _: Token) -> Option<Token> { Some(13) }
    fn notify_com(&mut self, _: Token, _: Token) { self.events.push("notify".into()); }
    fn can_unload(&mut self, _: Token) -> bool { false }
    fn ids(&mut self, out: &mut [u32]) -> bool { self.events.push("ids".into()); for (i,v) in out.iter_mut().enumerate() { *v = i as u32 + 41; } true }
}
impl support::Host for Spy {
    fn command_service(&mut self) -> Option<Token> { if self.services { self.add_ref(20); Some(20) } else { None } }
    fn message_service(&mut self) -> Option<Token> { self.add_ref(21); Some(21) }
    fn subscribe(&mut self, _: Token, _: Token, ids: &[u32]) { self.events.push(format!("subscribe:{}",ids.len())); }
    fn unsubscribe(&mut self, _: Token, _: Token, ids: &[u32]) { self.events.push(format!("unsubscribe:{}",ids.len())); }
    fn register(&mut self, _: Token, id: u32, name: &str, description: Option<&str>) { assert!(description.is_none()); self.events.push(format!("register:{id:x}:{name}")); }
    fn unregister(&mut self, _: Token, id: u32) { self.events.push(format!("unregister:{id:x}")); }
}
impl filesystem::Host for Spy {
    fn application_directory(&mut self) -> String { "app".into() }
    fn directory_setting(&mut self, section: &str, kind: filesystem::Directory, key: Option<&str>) -> Option<String> {
        assert_eq!(section,"Directories"); if matches!(kind,filesystem::Directory::Plugin) { assert_eq!(key,Some("PlugIn")); }
        self.events.push("setting".into()); self.setting.clone()
    }
    fn resolve(&mut self, base: &str, path: &str) -> String { format!("{base}/{path}") }
    fn directory_exists(&mut self, _: &str) -> bool { self.exists }
}
impl script::Host for Spy {
    fn translate(&mut self, v: Token, text: &[u8]) -> Option<Variant> { self.vocab.get(&(v,text.to_vec())).cloned() }
    fn numeric(&mut self, _: &[u8]) -> Option<Variant> { None }
    fn unassigned_parameter(&mut self) -> Result<Variant> { Err(Unsupported("GZF9")) }
    fn start(&mut self, _: Token, _: &mut script::Script) { self.events.push("script-start".into()); }
    fn text(&mut self, _: Token, _: &mut script::Script, text: &[u8]) { self.events.push(format!("text:{}",String::from_utf8_lossy(text))); }
    fn command_start(&mut self, _: Token, s: &mut script::Script, command: u32) {
        self.events.push(format!("command:{command}")); self.nested = Some(s.execute(self,b"nested").unwrap());
    }
    fn parameter(&mut self, _: Token, _: &mut script::Script, id: u32, value: &Variant) { self.parameters.push((id,value.clone())); }
    fn command_end(&mut self, _: Token, _: &mut script::Script, id: u32) { self.events.push(format!("end:{id}")); }
    fn parameters(&mut self, _: Token, _: &mut script::Script, id: u32, values: &[(u32,Variant)]) { self.events.push(format!("collection:{id}:{}",values.len())); }
    fn read_file(&mut self, _: &str) -> Result<Option<Vec<u8>>> { Ok(Some(b"plain".to_vec())) }
    fn acquire_resource(&mut self, _: u64) -> Option<Token> { self.resource.then_some(30) }
    fn read_stream(&mut self, _: Token) -> Result<Vec<u8>> { Ok(b"resource".to_vec()) }
    fn return_resource(&mut self, t: Token) { self.events.push(format!("return:{t}")); }
}

#[test]
fn gzc01_04_framework_identity_and_counts() {
    let mut f = framework::Framework::default(); let mut out = Some(framework::Face::Secondary);
    assert!(!f.query_interface(99,&mut out)); assert_eq!(out,Some(framework::Face::Secondary));
    for (iid,face) in [(1,framework::Face::Primary),(0x65,framework::Face::Primary),(0x23EA70A1,framework::Face::Secondary)] {
        let before = f.ref_count(); assert!(f.query_interface(iid,&mut out)); assert_eq!(out,Some(face)); assert_eq!(f.ref_count(),before+1);
    }
    let mut f = framework::Framework::default(); assert_eq!(f.scheduling_count(),4); assert_eq!(f.debug_level(),0);
    assert_eq!([f.add_ref(),f.add_ref(),f.release(),f.release()],[1,2,1,0]); assert_eq!(f.ref_count(),1);
    for (input,want) in [(0,1),(-1,1),(7,7)] { f.set_scheduling_count(input); assert_eq!(f.scheduling_count(),want); }
}
#[test]
fn gzc05_07_services_and_protection() {
    let mut f = framework::Framework::default(); let mut h = Spy::default(); let mut out = Some(99);
    assert!(!f.add_service(&mut h,1,None)); assert!(f.add_service(&mut h,1,Some(1))); assert!(!f.add_service(&mut h,1,Some(2)));
    let before = h.events.len(); assert!(!f.lookup(&mut h,2,77,&mut out)); assert_eq!(out,Some(99)); assert_eq!(h.events.len(),before);
    assert!(f.lookup(&mut h,1,77,&mut out)); assert_eq!(out,Some(1)); assert!(h.events.contains(&"qi:1:4d".into())); h.release(1);
    assert!(f.add_tick(&mut h,1)); assert!(!f.add_tick(&mut h,1)); assert_eq!(f.add_ordered(&mut h,8,1,1),Ok(true));
    assert_eq!(f.add_ordered(&mut h,9,1,2),Err(Unsupported("GZF1")));
    f.registry_protected = true; assert!(!f.remove_service(&mut h,1)); assert!(!f.add_service(&mut h,2,Some(2))); f.registry_protected = false;
    f.ticks_protected = true; assert!(!f.remove_tick(&mut h,1)); f.ticks_protected = false;
    assert!(f.remove_service(&mut h,1)); assert!(!f.remove_tick(&mut h,1)); assert_eq!(h.counts[&1],0);
}
#[test]
fn gzc08_10_quit_and_main_failure() {
    let mut f = framework::Framework::default(); let mut h = Spy::default(); f.alternate_loop = true; f.quit(&mut h,9);
    assert!(f.pending_quit); assert_eq!(f.process_result,9); assert!(h.events.is_empty());
    f.alternate_loop = false; f.quit(&mut h,9); assert_eq!(h.events,["quit:9"]); h.events.clear();
    assert_eq!(f.main(&mut h),u32::MAX); assert_eq!(h.events,["prepare","com-service","setup","app-init","shutdown"]);
    h.init_ok = true; h.events.clear(); assert_eq!(f.main(&mut h),7); assert!(h.events.contains(&"run".into()));
    h.events.clear(); f.report_exception(&mut h,None); assert_eq!(h.events,["enable:false","??? Unknown exception ???","enable:true"]);
    h.events.clear(); f.abort(&mut h,8); assert_eq!(h.events,["abort","exit:8"]);
}
#[test]
fn gzc11_13_com_initialization_and_ids() {
    let mut c = com::Com::default(); let mut h = Spy::default();
    assert!(!c.real_init(&mut h,9)); assert!(!c.initialized); h.init_ok = true;
    assert!(c.real_init(&mut h,9)); let count = h.events.len(); assert!(c.real_init(&mut h,9)); assert_eq!(h.events.len(),count);
    assert_eq!(h.counts[&10],1); assert_eq!(h.counts[&11],1);
    let mut sentinel = [77]; assert!(c.create_guids(&mut h,&mut sentinel[..0])); assert_eq!(sentinel,[77]); assert!(!h.events.contains(&"ids".into()));
    assert_eq!(c.create_guid(&mut h),Some(41));
}
#[test]
fn com_libraries_and_distinct_lookup_paths() {
    let mut c = com::Com::default(); let mut h = Spy { init_ok: true, ..Default::default() }; assert!(c.real_init(&mut h,9));
    assert!(!c.add_library(&mut h,9,"missing")); assert!(c.add_library(&mut h,9,"lib")); let n = h.events.len();
    assert!(c.add_library(&mut h,9,"lib")); assert_eq!(h.events.len(),n);
    assert_eq!(c.can_unload_library(&mut h,"absent"),Ok(true)); assert_eq!(c.can_unload_library(&mut h,"lib"),Ok(false));
    c.import_class_index(88,12); h.events.clear(); assert_eq!(c.get_class_object(&mut h,88,1),Some(12)); assert_eq!(h.events[0],"library-class");
    h.events.clear(); assert_eq!(c.get_class_object(&mut h,89,7),Some(10)); assert_eq!(h.events[0],"director-class"); assert_eq!(h.events[1],"qi:10:7");
}
struct Convert;
impl ValueConvert for Convert {
    fn get_val_string(&self, v: &Variant, out: &mut Vec<u8>) -> bool { if let Some(s) = v.bytes() { *out = s; true } else { false } }
    fn get_val_sint32(&self, v: &Variant) -> Option<i32> { if let Variant::Scalar(Elem::I32(v)) = v { Some(*v) } else { None } }
}
#[test]
fn gzc14_17_command_registry_and_defaults() {
    let s = commands::server::Server::new(Rc::new(Convert)); assert_eq!(s.command_count(),0); assert_eq!(s.generator_count(),0); assert!(s.dispatcher().is_none());
    assert!(!s.register(0,Some(b"zero"),None)); assert!(!s.register(1,None,None)); assert!(s.register(8,Some(b"X"),None)); assert!(!s.register(8,Some(b"Y"),None));
    assert_eq!(s.get_command_name(8),Some(b"X".to_vec())); assert!(!s.parameter_info(9,None));
    s.set_generate_description(true); assert!(s.parameter_info(8,None)); let mut name = Vec::new(); let mut info = Vec::new(); assert!(s.info(8,&mut name,Some(&mut info))); assert_eq!(info,b"Format: X\nOutput: <none>");
    assert!(s.register(3,Some(b"x"),None)); assert_eq!(s.get_command_id(b"X"),3); assert_eq!(s.get_command_id_from_string(Some(b"X arg"),u32::MAX),0);
    assert_eq!(s.execute(3,None,None),Err(Unsupported("GZF3")));
    assert!(s.init(None)); assert!(s.init(None)); assert_eq!(s.generator_count(),2); assert!(s.dispatcher().is_some());
    let p = s.create_parameter_set(); assert_eq!(p.borrow().ref_count(),1);
    assert_eq!(s.execute_text(b"x a",u32::MAX,None),Ok(2)); let mut text = Vec::new(); assert!(s.convert_command(3,None,&mut text)); assert_eq!(text,b"x");
    assert_eq!(s.shutdown(),Ok(true)); assert_eq!(s.command_count(),0); assert_eq!(s.generator_count(),0); assert!(s.dispatcher().is_none());
}
#[test]
fn gzc18_20_tool_support_lifetime() {
    let mut s = support::Support::new(support::Kind::Tool); let mut h = Spy { services: true, ..Default::default() };
    assert!(s.init(&mut h,1)); assert!(s.init(&mut h,1));
    for &(id,name) in support::TOOLS { assert!(h.events.contains(&format!("register:{id:x}:{name}"))); }
    assert_eq!(h.events.iter().filter(|e| e.starts_with("register:")).count(),7);
    assert!(s.shutdown(&mut h,1)); let n = h.events.len(); assert!(s.shutdown(&mut h,1)); assert_eq!(h.events.len(),n); assert_eq!(h.counts[&20],0); assert_eq!(h.counts[&21],0);
    let mut s = support::Support::new(support::Kind::Tool); let mut h = Spy::default(); assert!(s.init(&mut h,1)); h.services = true; assert!(s.init(&mut h,1)); assert!(!h.events.iter().any(|e| e.starts_with("register:")));
}
#[test]
fn gzc21_misc_inventory_and_notification_quirk() {
    let mut s = support::Support::new(support::Kind::Misc); let mut h = Spy { services: true, ..Default::default() }; assert!(s.init(&mut h,1));
    assert_eq!(support::MISC.len(),35); for &(id,name) in support::MISC { assert!(h.events.contains(&format!("register:{id:x}:{name}"))); }
    assert_eq!(s.notifications().iter().filter(|&&id| id == 0x0BCE8AA5).count(),2); assert!(!s.notifications().contains(&0xABCE8AA7)); assert!(s.pending_waits.is_empty()); assert_eq!(s.random_seed_argument,0);
    assert_eq!(s.do_message(0,None,None),Err(Unsupported("GZF4"))); assert!(s.shutdown(&mut h,1));
}
struct Alloc;
impl buffer::Allocator for Alloc { fn allocate(&mut self, bytes: usize) -> Option<Vec<u8>> { Some(vec![0xCD;bytes]) } }
#[test]
fn gzc22_25_buffer_storage_locks_and_colour() {
    let mut b = buffer::Buffer::default(); assert!(!b.lock(0)); assert_eq!(b.init(&mut Alloc,0,3,2,8),Ok(false)); assert_eq!(b.init(&mut Alloc,10,3,2,8),Ok(true));
    assert_eq!(b.stride(),12); assert_eq!(b.pixels().unwrap(),[0xCD;36]); assert_eq!(b.init(&mut Alloc,1,1,2,8),Ok(false)); assert_eq!(b.map_rgb(255,127,64),Ok(0));
    assert!(b.lock(0x8040)); assert!(b.lock(2)); assert_eq!(b.nesting,2); assert_eq!(b.lock_flags,0x8042); assert_eq!(b.modifications,1); assert!(b.unlock(0)); assert_eq!(b.nesting,1); assert_eq!(b.lock_flags,0);
    assert_eq!(b.rect(),[0,0,10,3]); assert_eq!(b.colour_type(),(2,8)); assert_eq!(b.bytes_per_pixel(),1);
}
struct Back {
    events: Mutex<Vec<String>>,
    accepts: bool,
    recurse: bool,
}
impl logging::Backend for Back {
    fn accepts(&self, _: &str, _: i32) -> bool { self.events.lock().unwrap().push("filter".into()); self.accepts }
    fn output(&self, router: &logging::Log, record: &logging::Record) {
        self.events.lock().unwrap().push(record.text.clone());
        if self.recurse { router.output("c",1,"recursive"); self.events.lock().unwrap().push("after".into()); }
    }
    fn set_level(&self, category: Option<&str>, level: i32) { self.events.lock().unwrap().push(format!("level:{category:?}:{level}")); }
}
fn backend(accepts: bool, recurse: bool) -> Arc<Back> { Arc::new(Back { events: Mutex::new(Vec::new()), accepts, recurse }) }
#[test]
fn gzc26_log_level_sentinels() {
    use logging::*;
    assert_eq!((string_from_filter_level(0),string_from_filter_level(MAX_LEVEL)),("all","none")); assert_eq!((string_from_log_level(0),string_from_log_level(MAX_LEVEL)),("none","all"));
    assert_eq!(filter_level_from_string(Some("ALL")),0); assert_eq!(log_level_from_string(Some("aLl")),MAX_LEVEL); assert_eq!(log_level_from_string(None),-1); assert_eq!(filter_level_from_string(Some("")),-1); assert_eq!(string_from_log_level(-1),"invalid filter level");
}
#[test]
fn gzc27_log_filter_decline() {
    let l = logging::Log::default(); let b = backend(false,false); l.add_backend("test",b.clone()).unwrap(); l.output("c",1,"rejected"); assert_eq!(*b.events.lock().unwrap(),["filter"]);
}
#[test]
fn gzc28_log_recursion_deferred_and_bounded() {
    let l = logging::Log::default(); let b = backend(true,true); l.add_backend("test",b.clone()).unwrap(); l.output("c",1,"outer");
    assert_eq!(*b.events.lock().unwrap(),["filter","outer","after","filter","recursive","after","filter","recursive","after"]);
}
#[test]
fn gzc29_foreign_thread_fifo_owns_text() {
    let l = Arc::new(logging::Log::default()); let b = backend(true,false); l.add_backend("test",b.clone()).unwrap(); let producer = l.clone();
    std::thread::spawn(move || { for text in [String::from("A"),String::from("B")] { producer.output("c",1,&text); } }).join().unwrap();
    assert!(b.events.lock().unwrap().is_empty()); assert!(l.on_tick()); assert_eq!(*b.events.lock().unwrap(),["filter","A","filter","B"]);
}
#[test]
fn gzc30_31_filesystem_defaults_setter_failure_and_cache() {
    let mut f = filesystem::FileSystem::default(); let mut h = Spy::default(); assert_eq!(f.plugin_name(),"Plugins");
    assert!(!f.set_data_directory(&mut h,"missing")); assert_eq!(f.cached_data(),("missing",false)); assert_eq!(f.data_directory(&mut h),"app"); assert_eq!(f.plugin_directory(&mut h),"app/Plugins");
    let count = h.events.len(); h.setting = Some("configured".into()); assert_eq!(f.plugin_directory(&mut h),"app/Plugins"); assert_eq!(h.events.len(),count);
    let mut f = filesystem::FileSystem::default(); assert_eq!(f.plugin_directory(&mut h),"app/configured"); h.exists = true; assert!(f.set_plugin_directory(&mut h,"custom")); assert_eq!(f.plugin_directory(&mut h),"custom");
}
fn script_fixture() -> (script::Script,Spy) {
    let mut s = script::Script::new_fixture(); let mut h = Spy::default(); s.set_target(&mut h,Some(40)); s.push_vocabulary(&mut h,Some(1));
    h.vocab.insert((1,b"C".to_vec()),Variant::Scalar(Elem::U32(7))); h.vocab.insert((1,b"P".to_vec()),Variant::Scalar(Elem::U32(8))); (s,h)
}
#[test]
fn gzc32_33_vocabulary_ownership_shadowing() {
    let (mut s,mut h) = script_fixture(); assert!(!s.push_vocabulary(&mut h,None)); assert!(!s.pop_vocabulary(&mut h));
    h.vocab.insert((2,b"C".to_vec()),Variant::Scalar(Elem::U32(9))); assert!(s.push_vocabulary(&mut h,Some(2))); assert_eq!(s.lookup(&mut h,b"C"),Some(Variant::Scalar(Elem::U32(9))));
    assert!(s.pop_vocabulary(&mut h)); assert_eq!(h.counts[&2],0); assert_eq!(s.lookup(&mut h,b"C"),Some(Variant::Scalar(Elem::U32(7))));
    s.dispose(&mut h); assert!(h.counts.values().all(|&v| v == 0));
}
#[test]
fn gzc34_non_uint_translation_is_not_a_command() {
    let (mut s,mut h) = script_fixture(); h.vocab.insert((1,b"Bad".to_vec()),Variant::Scalar(Elem::I32(7)));
    assert_eq!(s.execute(&mut h,b"<Bad>"),Ok(true)); assert!(!h.events.iter().any(|e| e.starts_with("command:")));
}
#[test]
fn gzc35_36_quoted_value_and_reentry_callbacks() {
    let (mut s,mut h) = script_fixture(); assert_eq!(s.execute(&mut h,b"<C P=\"a b\">"),Ok(true)); assert_eq!(h.parameters,[(8,Variant::string(b"a b"))]); assert_eq!(h.nested,Some(false));
    assert!(!s.active()); let calls: Vec<_> = h.events.iter().filter(|e| e.starts_with("command:") || e.starts_with("end:") || e.starts_with("collection:")).map(String::as_str).collect(); assert_eq!(calls,["command:7","end:7","collection:7:1"]);
    assert_eq!(s.execute(&mut h,b"<C P=\"&amp;\\n\">"),Ok(true)); assert_eq!(h.parameters[1].1,Variant::string(b"&amp;\\n"));
}
#[test]
fn gzc37_unknown_tag_then_text_and_error_reset() {
    let (mut s,mut h) = script_fixture(); assert_eq!(s.execute(&mut h,b"<unknown X=\"x\">later"),Ok(true)); assert!(h.events.contains(&"text:later".into()));
    assert_eq!(s.execute(&mut h,b"<C = >"),Ok(false)); assert!(s.error); assert_eq!(s.execute(&mut h,b"later"),Ok(true)); assert!(!s.error);
    assert_eq!(s.execute(&mut h,b"<C"),Ok(true)); // EOF does not demand a closing tag.
}
#[test]
fn gzc38_resource_missing_stream_and_release() {
    let (mut s,mut h) = script_fixture(); assert_eq!(s.execute_resource(&mut h,1),Ok(false)); h.resource = true;
    assert_eq!(s.execute_resource(&mut h,1),Ok(false)); assert_eq!(h.events.last().unwrap(),"return:30"); h.stream = true;
    assert_eq!(s.execute_resource(&mut h,1),Ok(true)); assert_eq!(h.counts[&30],0); assert_eq!(h.events.last().unwrap(),"return:30"); assert!(h.events.contains(&"text:resource".into()));
    assert_eq!(s.execute_file(&mut h,"script"),Ok(true)); assert!(h.events.contains(&"text:plain".into()));
}
