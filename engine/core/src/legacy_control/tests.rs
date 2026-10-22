// SPDX-License-Identifier: GPL-3.0-or-later
// Expected inputs/effects fixed from legacy-control §§1–10 before production code.
use super::*;
use crate::variant::{Elem, Variant};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, PartialEq)]
enum Event {
    Create(u32, u32), Query(Handle, u32), Add(Handle), Release(Handle),
    Word(Handle, String, u32), Read(Handle, Read), Write(Handle, Write),
    Widget(Handle, u32, Vec<Value>), FontId(String), Font(u32),
    Colour([u8; 3]), ColourFour([u8;3],u32), GridColour(Handle, [u8; 3]),
    NotifyCell(Handle,i32,i32,CellDescriptor), NotifyAxis(Handle,Notifier,[i32;3]),
    Image(u32, u32), StringResource(u32, u32), ImageRect(Handle),
    ManagerSlot3, NewString, Append(Handle, String),
}

#[derive(Default)]
struct Fake {
    events: Vec<Event>, creates: VecDeque<Option<Handle>>,
    queries: BTreeMap<(Handle, u32), Handle>, reads: BTreeMap<(Handle, Read), Value>,
    raw: BTreeMap<(Handle, u32), Value>, release_counts: BTreeMap<Handle, u32>,
    images: BTreeMap<(u32, u32), Handle>, strings: BTreeMap<(u32, u32), String>,
}
impl Foreign for Fake {
    fn create(&mut self, c: u32, i: u32) -> Option<Handle> {
        self.events.push(Event::Create(c,i)); self.creates.pop_front().flatten()
    }
    fn query(&mut self, h: Handle, i: u32) -> Option<Handle> {
        self.events.push(Event::Query(h,i)); self.queries.get(&(h,i)).copied()
    }
    fn add_ref(&mut self, h: Handle) { self.events.push(Event::Add(h)); }
    fn release(&mut self, h: Handle) -> u32 {
        self.events.push(Event::Release(h)); self.release_counts.get(&h).copied().unwrap_or(1)
    }
    fn vocabulary_add(&mut self, h: Handle, s: &str, id: u32) {
        self.events.push(Event::Word(h,s.into(),id));
    }
    fn read(&mut self, h: Handle, r: Read) -> Value {
        self.events.push(Event::Read(h,r.clone())); self.reads.get(&(h,r)).cloned().unwrap_or(Value::None)
    }
    fn write(&mut self, h: Handle, w: Write) { self.events.push(Event::Write(h,w)); }
    fn widget(&mut self, h: Handle, slot: u32, args: Vec<Value>) -> Value {
        self.events.push(Event::Widget(h,slot,args)); self.raw.get(&(h,slot)).cloned().unwrap_or(Value::None)
    }
    fn font_id(&mut self, n: &str) -> u32 { self.events.push(Event::FontId(n.into())); 7 }
    fn font(&mut self, id: u32) -> Handle { self.events.push(Event::Font(id)); 700 }
    fn colour(&mut self, c: [u8;3]) -> u32 { self.events.push(Event::Colour(c)); u32::from_le_bytes([c[0],c[1],c[2],0]) }
    fn colour_four(&mut self,c:[u8;3],fourth:u32)->u32 {self.events.push(Event::ColourFour(c,fourth));0}
    fn grid_colour(&mut self, h: Handle, c: [u8;3]) -> u32 { self.events.push(Event::GridColour(h,c)); u32::from_le_bytes([c[0],c[1],c[2],0]) }
    fn load_image(&mut self, g:u32, i:u32) -> Option<Handle> { self.events.push(Event::Image(g,i)); self.images.get(&(g,i)).copied() }
    fn load_string(&mut self, g:u32, i:u32) -> Option<String> { self.events.push(Event::StringResource(g,i)); self.strings.get(&(g,i)).cloned() }
    fn image_rect(&mut self, h:Handle) -> Rect { self.events.push(Event::ImageRect(h)); [0,0,16,20] }
    fn notify_cell(&mut self,h:Handle,col:i32,row:i32,d:CellDescriptor) {self.events.push(Event::NotifyCell(h,col,row,d));}
    fn notify_axis(&mut self,h:Handle,kind:Notifier,v:[i32;3]) {self.events.push(Event::NotifyAxis(h,kind,v));}
    fn winmgr_slot3(&mut self) -> Handle { self.events.push(Event::ManagerSlot3); 900 }
    fn new_string(&mut self) -> Handle { self.events.push(Event::NewString); 800 }
    fn append_string(&mut self, h:Handle, text:&str) { self.events.push(Event::Append(h,text.into())); }
}
fn u(k:u32,v:u32)->Property { Property{key:k,value:Some(Variant::Scalar(Elem::U32(v)))} }
fn s(k:u32,v:&str)->Property { Property{key:k,value:Some(Variant::string(v.as_bytes()))} }
fn win(h:Handle,w:Write)->Event { Event::Write(h,w) }
fn call(h:Handle,slot:u32,args:Vec<Value>)->Event { Event::Widget(h,slot,args) }
fn n(v:u32)->Value {Value::U32(v)}
fn i(v:i32)->Value {Value::I32(v)}
fn text(v:&str)->Value {Value::Text(v.into())}
fn initialized()->(LegacyControl,Globals,Fake) {
    let mut f=Fake::default(); f.creates.push_back(Some(99));
    let mut g=Globals::default(); let c=LegacyControl::new(&mut g,&mut f);
    f.events.clear(); (c,g,f)
}

#[test]
fn property_scan_skips_wrong_types_and_preserves_numeric_output() {
    let p=vec![Property{key:5,value:None},s(5,"wrong"),u(5,0xffff_fffe),u(5,9)];
    let p=Properties(&p); let mut number=42; let mut string="old".into();
    assert!(p.has(5)); assert!(p.uint32(5,&mut number)); assert_eq!(number,0xffff_fffe);
    let mut signed=0; assert!(p.sint32(5,&mut signed)); assert_eq!(signed,-2);
    assert!(!p.uint32(9,&mut number)); assert_eq!(number,0xffff_fffe);
    assert!(p.string(5,&mut string)); assert_eq!(string,"wrong");
    assert!(!p.string(9,&mut string)); assert_eq!(string,"");
}

#[test]
fn exact_vocabulary_order_ids_aliases_and_maps() {
    let (_,g,f)=initialized();
    let expected=include_str!("vocabulary.expected");
    let mut entries=vec![]; let mut expected_classes=BTreeMap::new();
    let mut expected_iids=vec![];
    for line in expected.lines() {
        if line.starts_with('[') {
            let fields:Vec<_>=line.trim_matches(['[',']']).split_whitespace().collect();
            let cls=u32::from_str_radix(fields[1],16).unwrap();
            let iid=u32::from_str_radix(fields[2],16).unwrap();
            if cls!=0 { expected_classes.insert(cls,iid); expected_iids.push(iid); }
        } else {
            let (name,id)=line.split_once(' ').unwrap();
            entries.push((name.to_string(),u32::from_str_radix(id,16).unwrap()));
        }
    }
    assert_eq!(g.classes,expected_classes); assert_eq!(g.interpreters.len(),20);
    for iid in expected_iids { assert!(g.interpreters.contains_key(&iid)); }
    assert_eq!(g.words(),entries); assert_eq!(entries.len(),393);
    // A fresh constructor must reuse, not register again.
    let mut f=f; let mut g=g; let _second=LegacyControl::new(&mut g,&mut f);
    assert_eq!(f.events,vec![Event::Add(99)]);
}

#[test]
fn constructor_failure_retry_maps_survive_and_zero_release_destroys() {
    let mut f=Fake::default(); let mut g=Globals::default();
    let mut c=LegacyControl::new(&mut g,&mut f);
    assert_eq!(c.ref_count(),0); assert_eq!(c.control(),None); assert!(!c.owns_control());
    assert!(g.classes.is_empty()); assert_eq!(c.release(&mut g,&mut f),0); assert!(c.destroyed());
    f.creates.push_back(Some(99)); let mut c=LegacyControl::new(&mut g,&mut f);
    f.release_counts.insert(99,0); f.events.clear(); c.release(&mut g,&mut f);
    assert_eq!(f.events,vec![Event::Release(99)]); assert_eq!(g.vocabulary(),None); assert_eq!(g.classes.len(),20);
    f.creates.push_back(Some(98)); let _=LegacyControl::new(&mut g,&mut f); assert_eq!(g.classes.len(),20);
}

#[test]
fn lifecycle_identity_self_assignment_parent_and_one_shot_procedure() {
    let (mut c,mut g,mut f)=initialized(); let mut out=Some(Interface::Legacy);
    assert!(!c.query_interface(123,&mut out)); assert_eq!(out,Some(Interface::Legacy));
    assert!(c.query_interface(0x5a356159,&mut out)); assert_eq!(out,Some(Interface::Area));
    assert_eq!(c.ref_count(),1); assert!(c.implement_interface([1,2,3,4]));
    c.force_control(Some(10),&mut f); c.force_control(Some(10),&mut f);
    c.use_win_proc(Some(20),&mut f); c.use_win_proc(Some(20),&mut f);
    assert!(!c.owns_control());
    assert_eq!(f.events,vec![Event::Add(10),Event::Add(20)]); f.events.clear();
    f.queries.insert((10,0x5386d516),11);
    c.do_exceptions(10,false,&mut f); c.do_exceptions(10,true,&mut f); c.do_exceptions(10,true,&mut f);
    assert_eq!(f.events,vec![Event::Query(10,0x5386d516),call(11,0x1c,vec![Value::Handle(20)]),Event::Release(20),Event::Release(11),Event::Query(10,0x5386d516),Event::Release(11)]);
    f.events.clear(); f.queries.insert((50,0x9a4527cb),51); f.reads.insert((51,Read::LegacyControl),Value::Handle(52));
    assert!(c.set_parent(Some(50),&mut f)); assert!(c.set_parent(Some(60),&mut f));
    assert_eq!(f.events,vec![Event::Query(50,0x9a4527cb),Event::Read(51,Read::LegacyControl),Event::Add(52),Event::Release(51),Event::Release(52),Event::Query(60,0x9a4527cb)]);
    f.events.clear(); c.set_own_control(true); f.reads.insert((10,Read::Parent),Value::Handle(30));
    assert_eq!(c.release(&mut g,&mut f),0);
    assert_eq!(f.events,vec![Event::Read(10,Read::Parent),win(30,Write::ChildRemove(10)),win(10,Write::Shutdown),Event::Release(10),Event::Release(99)]);
}

#[test]
fn fallback_attempts_one_lowest_class_then_custom_and_retains_requested_ids() {
    let (mut c,mut g,mut f)=initialized();
    g.classes.insert(1,0x678d1e84); g.classes.insert(2,0x678d1e84);
    f.creates.extend([None,None,Some(10)]);
    let result=c.set_parameters(Properties(&[u(0xffffffff,0x1234),u(0xfffffffe,0x678d1e84)]),&g,&mut f);
    assert_eq!(result,Ok(false));
    assert_eq!(&f.events[..5],&[Event::Create(0x1234,0x22ba0121),Event::Create(1,0x22ba0121),Event::Create(0x478d1e6f,0x22ba0121),win(10,Write::Param(0xe85eebde,n(0x1234))),win(10,Write::Param(0x08d6e462,n(0x678d1e84)))]);
    assert_eq!(c.control(),Some(10));
}

#[test]
fn missing_zero_clsid_and_second_build_leak() {
    let (mut c,g,mut f)=initialized(); c.force_control(Some(5),&mut f); f.events.clear();
    for props in [vec![],vec![u(0xffffffff,0)],vec![s(0xffffffff,"wrong")]] {
        assert_eq!(c.set_parameters(Properties(&props),&g,&mut f),Ok(false));
        assert_eq!(c.control(),Some(5)); assert!(f.events.is_empty());
    }
    f.creates.extend([Some(10),Some(20)]);
    for _ in 0..2 { assert_eq!(c.set_parameters(Properties(&[u(0xffffffff,0x478d1e6f)]),&g,&mut f),Ok(false)); }
    assert_eq!(c.control(),Some(20)); assert!(!f.events.contains(&Event::Release(5))); assert!(!f.events.contains(&Event::Release(10)));
}

#[test]
fn exact_two_pass_slider_common_init_parent_front_order() {
    let (mut c,g,mut f)=initialized();
    f.queries.insert((50,0x9a4527cb),51); f.reads.insert((51,Read::LegacyControl),Value::Handle(52));
    c.set_parent(Some(50),&mut f); f.events.clear(); f.creates.push_back(Some(10));
    f.queries.insert((10,0x21325207),12);
    let props=[u(0xffffffff,0x21325208),s(0xc00,"(100,0)"),u(0xc01,0xc03),u(0x40a,30),u(0xf01b,0)];
    assert_eq!(c.set_parameters(Properties(&props),&g,&mut f),Ok(false));
    assert_eq!(f.events,vec![
        Event::Create(0x21325208,0x22ba0121),win(10,Write::Param(0xe85eebde,n(0x21325208))),win(10,Write::Param(0x08d6e462,n(0x21325207))),
        Event::Query(10,0x21325207),call(12,0x20,vec![i(100)]),call(12,0x18,vec![i(0)]),call(12,0x28,vec![n(2)]),Event::Release(12),
        win(10,Write::Flag(1,true)),win(10,Write::Flag(0x10000,true)),win(10,Write::Flag(0x800,false)),win(10,Write::Flag(0x20000,true)),win(10,Write::Flag(0x40000,true)),win(10,Write::Flag(4,false)),win(10,Write::Flag(0x8000,true)),
        win(10,Write::Param(0x3aca6906,Value::Pair(0,0))),win(10,Write::Param(0xbaca6879,Value::Comments(std::array::from_fn(|_|String::new())))),
        Event::Query(10,0x5386d516),win(10,Write::Init),win(52,Write::ChildAdd(10)),Event::Release(52),win(10,Write::Front),
        Event::Query(10,0x21325207),call(12,0x14,vec![i(30)]),Event::Release(12),win(10,Write::Flag(2,false))]);
    assert_eq!(c.pending_parent(),None);
}

#[test]
fn unknown_interpreter_stops_at_precise_prefix() {
    let (mut c,g,mut f)=initialized(); f.creates.push_back(Some(10)); f.queries.insert((10,0x212cdc1f),11);
    assert_eq!(c.set_parameters(Properties(&[u(0xffffffff,0x592)]),&g,&mut f),Err(Unsupported::Interpreter(0x212cdc1f)));
    assert_eq!(f.events,vec![Event::Create(0x592,0x22ba0121),win(10,Write::Param(0xe85eebde,n(0x592))),win(10,Write::Param(0x08d6e462,n(0x212cdc1f))),Event::Query(10,0x212cdc1f),Event::Release(11)]);
    assert!(c.owns_control()); assert_eq!(c.control(),Some(10));
}

#[test]
fn common_geometry_caption_resource_and_unconditional_parameters() {
    let mut f=Fake::default(); f.strings.insert((1,2),"Loaded".into());
    let p=[s(0x101,"(1,2,3,4)"),s(0x102,"(8,9)"),s(0x103,"(9,9)"),s(0x105,"literal"),s(0x106,"{1,2}"),Property{key:0x107,value:None}];
    interpret_common(10,Properties(&p),true,&mut f).unwrap();
    assert_eq!(&f.events[7..],&[
        win(10,Write::Area([1,2,3,4])),win(10,Write::Caption("literal".into())),Event::StringResource(1,2),win(10,Write::Caption("Loaded".into())),
        win(10,Write::Param(0x0b31771a,Value::Key(ResourceKey{kind:0x2026960b,group:1,instance:2}))),win(10,Write::Param(0x3aca6906,Value::Pair(1,2))),win(10,Write::Flag(0x20000,true)),
        win(10,Write::Param(0xbaca6879,Value::Comments(std::array::from_fn(|_|String::new()))))]);
}

#[test]
fn flat_rect_outline_folders_and_slider_boundary_effects() {
    let mut f=Fake::default();
    for iid in [0xc2afa76f,0xe303e5ae,0xa13351b8,0x21325207] { f.queries.insert((10,iid),11); }
    interpret(Kind::FlatRect,10,Properties(&[u(0xf005,0x204),u(0xf005,0x205),u(0xf005,0xf00b),s(0x201,"(266,-1,30)")]),true,&mut f).unwrap();
    assert_eq!(f.events,vec![Event::Query(10,0xc2afa76f),call(11,0x30,vec![n(0x2a)]),call(11,0x28,vec![n(1),n(10),n(255),n(30)]),Event::Release(11)]);
    f.events.clear(); interpret(Kind::Outline,10,Properties(&[]),true,&mut f).unwrap();
    assert_eq!(f.events,vec![Event::Query(10,0xe303e5ae),Event::Release(11)]);
    f.events.clear(); let p:Vec<_>=(0..34).map(|v|s(0x1102,&format!("({v},2)"))).collect();
    interpret(Kind::Outline,10,Properties(&p),true,&mut f).unwrap();
    assert_eq!(f.events[1],call(11,0x34,vec![n(32),Value::Points((0..32).map(|v|[v,2]).collect()),n(1),n(1)]));
    f.events.clear(); interpret(Kind::Folders,10,Properties(&[s(0x502,"A"),s(0x502,"B")]),false,&mut f).unwrap();
    assert_eq!(f.events,vec![Event::Query(10,0xa13351b8),call(11,0x1c,vec![]),call(11,0x10,vec![text("A"),n(0)]),call(11,0x10,vec![text("B"),n(0)]),Event::Release(11)]);
    f.events.clear(); interpret(Kind::Slider,10,Properties(&[s(0xc00,"(-5,-5)"),u(0xc01,999)]),true,&mut f).unwrap();
    assert_eq!(f.events,vec![Event::Query(10,0x21325207),call(11,0x20,vec![i(-5)]),call(11,0x18,vec![i(-5)]),call(11,0x28,vec![n(1)]),Event::Release(11)]);
}

#[test]
fn tree_position_retention_and_depth_first_search() {
    let mut f=Fake::default(); f.queries.insert((10,0x3ae8bad2),11); f.raw.insert((11,0x60),n(40));
    let props=[s(0x1102,"b:Beta:a:first::::::::::no:yes"),s(0x1102,"a:Alpha:Root:invalid::::::::::no:no"),s(0x1102,"c:Gamma:Root:invalid::::::::::no:no")];
    interpret(Kind::TreeView,10,Properties(&props),false,&mut f).unwrap();
    let adds:Vec<_>=f.events.iter().filter(|e|matches!(e,Event::Widget(_,0x60,_))).cloned().collect();
    assert_eq!(adds,vec![call(11,0x60,vec![n(0),i(-2),text("Alpha"),n(0)]),call(11,0x60,vec![n(40),i(-2),text("Beta"),n(0)]),call(11,0x60,vec![n(0),i(-2),text("Gamma"),n(0)])]);
    f.events.clear(); interpret(Kind::TreeView,10,Properties(&[s(0x1102,"z:Z:Root:bad::::::::::no:no")]),false,&mut f).unwrap();
    assert!(f.events.contains(&call(11,0x60,vec![n(0),i(-3),text("Z"),n(0)])));
}

#[test]
fn grid_shared_width_header_and_column_only_sizing() {
    let mut f=Fake::default(); f.queries.insert((10,0xdaa6b9be),11); f.raw.insert((11,0x14),Value::Handle(12));
    let p=[u(0x1607,1),s(0x1626,"2,4,60"),s(0x1627,"2,4,18")];
    interpret(Kind::Grid,10,Properties(&p),true,&mut f).unwrap();
    assert!(f.events.contains(&call(11,0x128,vec![n(1),n(0)])));
    assert!(f.events.contains(&call(11,0x104,vec![i(2),i(1),i(60)])));
    assert!(f.events.contains(&call(11,0x10c,vec![i(2),i(3),i(18)])));
    f.events.clear(); interpret(Kind::Grid,10,Properties(&[]),true,&mut f).unwrap();
    assert!(f.events.contains(&call(11,0x128,vec![n(0),n(0)])));
    f.events.clear(); let p=[s(0x1625,"0,0,2000,1,2")];
    assert_eq!(interpret(Kind::Grid,10,Properties(&p),true,&mut f),Err(Unsupported::Malformed("Grid style scratch uninitialized")));
    assert_eq!(f.events.last(),Some(&Event::Release(11)));
}
