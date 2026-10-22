// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable `cGZLegacyControl` owner, from legacy-control §§1–12.
//!
//! The core owns scans, ordering, maps, ref effects and mutations. `Foreign` only
//! performs individual interface/field/resource operations. No GUI is created by
//! this module. Handles are opaque identities, never dereferenced pointers.
//!
//! LC4: a successful QI to an unpublished interpreter is released and reported
//! unsupported. During a build the preceding effects are: old control forgotten
//! WITHOUT Release, factory/fallback calls, requested-ID parameters, own=true,
//! then QI/Release. Common, procedure handoff, Init and attachment have not run.
//! LC5: the default parent is *only* the opaque WinMgr slot-3 result.
//! LC8: fragments have explicit coverage; common-only output is partial whenever
//! the resolved interface answers an unknown per-type writer's QI.
//!
//! Numeric scans support bounded CRT decimal/hex inputs; overflow and invalid
//! UTF-8 are outside the safe published domain. See LC9/LC10. Deterministic
//! choices for indeterminate fields are labeled at their use sites.

use std::collections::BTreeMap;
use crate::ui_windows::win::WinToken;
use crate::variant::{Elem, Variant};

mod vocabulary;
mod parse;
mod interpreters;
mod serialize;
pub use interpreters::{interpret, interpret_common};
pub use serialize::{Fragment, FragmentCoverage};

pub type Handle = WinToken;
pub type Rect = [i32; 4];
pub const WIN_IID: u32 = 0x22ba0121;
pub const LEGACY_IID: u32 = 0x9a4527cb;
pub const GEN_IID: u32 = 0x5386d516;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceKey { pub kind: u32, pub group: u32, pub instance: u32 }

/// Owned parameter payloads. Refcounted variant/list/map allocation is represented
/// by the typed value transfer; the adapter must create and hand over those values,
/// balancing its temporary references (§8.9), never computing their contents.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    None, U32(u32), I32(i32), Handle(Handle), Text(String), Rect(Rect),
    Colour([u8;3]), Pair(u32,u32), Words(Vec<u32>), Key(ResourceKey),
    Comments([String;8]), Points(Vec<[i32;2]>),
    StringNumbers(BTreeMap<String,u32>), NumberStrings(BTreeMap<u32,String>),
    StringKeys(BTreeMap<String,(u32,u32)>),
    Notifier(Notifier), Cell(CellDescriptor), CellData(CellData),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notifier { Cells, Columns, Rows }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellDescriptor { pub flags:u32, pub group:u32, pub instance:u32, pub text:String }
/// Only published fields; the foreign grid primitive constructs the foreign
/// record's own defaults before copying these fields. No owner outcome delegated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellData {
    pub font: Option<Handle>, pub word04:u32, pub alignment:u8,
    pub bytes15_17:[u8;3], pub word18:u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Read { Id, Area, Fill, Caption, Flag(u32), Param(u32), Parent, LegacyControl }
#[derive(Debug, Clone, PartialEq)]
pub enum Write {
    Id(u32), Area(Rect), Move(i32,i32), Size(i32,i32), Fill([u8;3]), Caption(String),
    Flag(u32,bool), Param(u32,Value), Init, Shutdown, ChildAdd(Handle), ChildRemove(Handle), Front,
}

/// Primitive-only foreign interface. QI/create return acquired references;
/// `read` returns borrowed/copied fields, including GetParentWin/GetControl.
/// Raw widget slots are byte offsets from the published interface vtable.
/// `widget` returns raw getter outputs, never an interpreted owner result.
pub trait Foreign {
    fn create(&mut self, class:u32, iid:u32) -> Option<Handle>;
    fn query(&mut self, object:Handle, iid:u32) -> Option<Handle>;
    fn add_ref(&mut self, object:Handle);
    fn release(&mut self, object:Handle) -> u32;
    fn vocabulary_add(&mut self, vocabulary:Handle, word:&str, id:u32);
    fn read(&mut self, window:Handle, field:Read) -> Value;
    fn write(&mut self, window:Handle, operation:Write);
    fn widget(&mut self, interface:Handle, slot:u32, arguments:Vec<Value>) -> Value;
    fn font_id(&mut self, name:&str) -> u32;
    fn font(&mut self, id:u32) -> Handle;
    fn colour(&mut self, rgb:[u8;3]) -> u32;
    fn colour_four(&mut self, rgb:[u8;3], fourth:u32) -> u32;
    fn grid_colour(&mut self, window:Handle, rgb:[u8;3]) -> u32;
    fn load_image(&mut self, group:u32, instance:u32) -> Option<Handle>;
    fn load_string(&mut self, group:u32, instance:u32) -> Option<String>;
    fn image_rect(&mut self, buffer:Handle) -> Rect;
    /// Foreign notifier helpers (§9.5), receiving core-computed records only.
    fn notify_cell(&mut self, window:Handle, col:i32, row:i32, descriptor:CellDescriptor);
    fn notify_axis(&mut self, window:Handle, kind:Notifier, values:[i32;3]);
    fn winmgr_slot3(&mut self) -> Handle;
    fn new_string(&mut self) -> Handle;
    fn append_string(&mut self, string:Handle, text:&str);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unsupported { Interpreter(u32), Malformed(&'static str), ForeignResult(&'static str) }
pub type Result<T> = std::result::Result<T,Unsupported>;

#[derive(Debug, Clone)]
pub struct Property { pub key:u32, pub value:Option<Variant> }
#[derive(Debug, Clone, Copy)]
pub struct Properties<'a>(pub &'a [Property]);
impl Properties<'_> {
    pub fn has(self,key:u32)->bool {self.0.iter().any(|p|p.key==key)}
    pub fn uint32(self,key:u32,out:&mut u32)->bool {
        for p in self.0.iter().filter(|p|p.key==key) {
            if let Some(Variant::Scalar(Elem::U32(v)))=&p.value { *out=*v; return true; }
        }
        false
    }
    pub fn sint32(self,key:u32,out:&mut i32)->bool {
        let mut v=0; if self.uint32(key,&mut v) {*out=v as i32;true} else {false}
    }
    pub fn string(self,key:u32,out:&mut String)->bool {
        out.clear();
        for p in self.0.iter().filter(|p|p.key==key) {
            if let Some(v)=&p.value {
                if v.mem_type()==0x800e {
                    // UTF-8 only in this portable surface; interpreters validate
                    // before any effect. Never replace invalid bytes with garbage.
                    if let Some(bytes)=v.bytes() { if let Ok(s)=String::from_utf8(bytes) { *out=s; return true; } }
                }
            }
        }
        false
    }
    pub fn number(self,key:u32)->Option<u32> {let mut n=0;self.uint32(key,&mut n).then_some(n)}
    pub fn text(self,key:u32)->Option<String> {let mut s=String::new();self.string(key,&mut s).then_some(s)}
    pub fn texts(self,key:u32)->impl Iterator<Item=(usize,String)> + '_ {
        self.0.iter().enumerate().filter(move |(_,p)|p.key==key).filter_map(|(k,p)|{
            let v=p.value.as_ref()?; if v.mem_type()!=0x800e {return None;}
            String::from_utf8(v.bytes()?).ok().map(|s|(k,s))
        })
    }
    fn validate(self)->Result<()> {
        for p in self.0 {if let Some(v)=&p.value {if v.mem_type()==0x800e && v.bytes().is_none_or(|b|String::from_utf8(b).is_err()) {return Err(Unsupported::Malformed("non-UTF8 string"));}}}
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind { Common,FlatRect,Text,TextEdit,ListBox,Combo,Btn,BMP,Slider,Scrollbar,LineInput,OptGrp,Spinner,Outline,TextTicker,FileBrowser,TreeView,Folders,Grid,Custom,Gen }
impl Kind {
    pub fn iid(self)->u32 { vocabulary::TABLES.iter().find(|t|t.kind==self).map_or(0,|t|t.iid) }
}

/// Explicit process context, shared by callers (not a second hidden singleton).
/// No RefCell or pointer-sized identity assumptions are used by getters.
#[derive(Debug, Default)]
pub struct Globals {
    vocabulary:Option<Handle>,
    pub classes:BTreeMap<u32,u32>,
    pub interpreters:BTreeMap<u32,Kind>,
}
impl Globals {
    pub fn vocabulary(&self)->Option<Handle>{self.vocabulary}
    pub fn words(&self)->Vec<(String,u32)> {
        if self.vocabulary.is_none(){return vec![];}
        vocabulary::TABLES.iter().flat_map(|t|t.words.iter().map(|(w,id)|(w.to_string(),*id))).collect()
    }
    /// Each function registers words before unique map inserts. Exposed so repeated
    /// registration preserves first-insert map values (§4).
    pub fn register(&mut self,kind:Kind,f:&mut impl Foreign) {
        if let Some(v)=self.vocabulary {
            let t=vocabulary::TABLES.iter().find(|t|t.kind==kind).expect("all Kind variants have tables");
            for &(word,id) in t.words { f.vocabulary_add(v,word,id); }
            if kind!=Kind::Common {self.interpreters.entry(t.iid).or_insert(kind);self.classes.entry(t.class).or_insert(t.iid);}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interface { Primary, Area, Legacy }
#[derive(Debug)]
pub struct LegacyControl {
    refs:u32, control:Option<Handle>, parent:Option<Handle>, procedure:Option<Handle>, own:bool, dead:bool,
}
impl LegacyControl {
    pub fn new(g:&mut Globals,f:&mut impl Foreign)->Self {
        if let Some(v)=g.vocabulary {f.add_ref(v);} else {
            g.vocabulary=f.create(0xba2e7954,0x1a2bdebb);
            if g.vocabulary.is_some() {for t in vocabulary::TABLES {g.register(t.kind,f);}}
        }
        Self{refs:0,control:None,parent:None,procedure:None,own:false,dead:false}
    }
    pub fn ref_count(&self)->u32{self.refs}
    pub fn control(&self)->Option<Handle>{self.control}
    pub fn pending_parent(&self)->Option<Handle>{self.parent}
    pub fn procedure(&self)->Option<Handle>{self.procedure}
    pub fn owns_control(&self)->bool{self.own}
    pub fn destroyed(&self)->bool{self.dead}
    pub fn get_control_vocabulary(&self,g:&Globals)->Option<Handle>{g.vocabulary}
    pub fn add_ref(&mut self)->u32{self.refs=self.refs.wrapping_add(1);self.refs}
    pub fn query_interface(&mut self,iid:u32,out:&mut Option<Interface>)->bool {
        let i=match iid {1|0x1a356083=>Interface::Primary,0x5a356159=>Interface::Area,LEGACY_IID=>Interface::Legacy,_=>return false};
        *out=Some(i);self.add_ref();true
    }
    pub fn implement_interface(&self,_arguments:[u32;4])->bool{true}
    /// Area-interface slots 3,4,7 are shared ignored-argument true stubs.
    pub fn area_stub(&self,_arguments:&[u32])->bool{true}
    pub fn set_own_control(&mut self,own:bool){self.own=own;}
    pub fn set_own_control_thunk(&mut self,own:bool){self.set_own_control(own);}
    pub fn force_control(&mut self,h:Option<Handle>,f:&mut impl Foreign){replace(&mut self.control,h,f);}
    pub fn use_win_proc(&mut self,h:Option<Handle>,f:&mut impl Foreign){replace(&mut self.procedure,h,f);}
    pub fn get_id(&self,f:&mut impl Foreign)->Result<u32> {
        match self.control {None=>Ok(0),Some(h)=>word(f.read(h,Read::Id))}
    }
    pub fn set_id(&self,id:u32,f:&mut impl Foreign){if let Some(h)=self.control {f.write(h,Write::Id(id));}}
    pub fn set_area(&self,area:Rect,f:&mut impl Foreign)->bool {
        if let Some(h)=self.control {f.write(h,Write::Area(area));true}else{false}
    }
    pub fn get_area(&self,out:&mut Rect,f:&mut impl Foreign)->Result<bool> {
        if let Some(h)=self.control {match f.read(h,Read::Area) {Value::Rect(r)=>{*out=r;Ok(true)},_=>Err(Unsupported::ForeignResult("window area"))}} else {Ok(false)}
    }
    pub fn set_parent(&mut self,obj:Option<Handle>,f:&mut impl Foreign)->bool {
        if let Some(p)=self.parent.take(){f.release(p);}
        if let Some(o)=obj {if let Some(i)=f.query(o,LEGACY_IID) {
            if let Value::Handle(p)=f.read(i,Read::LegacyControl) {f.add_ref(p);self.parent=Some(p);}
            f.release(i);
        }}
        true
    }
    pub fn do_exceptions(&mut self,win:Handle,flag:bool,f:&mut impl Foreign) {
        if flag {if let Some(i)=f.query(win,GEN_IID) {
            if let Some(p)=self.procedure.take(){f.widget(i,0x1c,vec![Value::Handle(p)]);f.release(p);}
            f.release(i);
        }}
    }
    pub fn release(&mut self,g:&mut Globals,f:&mut impl Foreign)->u32 {
        if self.dead {return 0;} // Safe portable tombstone, not native use-after-free.
        if self.refs>1 {self.refs-=1;self.refs} else {self.destroy(g,f);0}
    }
    pub fn destroy(&mut self,g:&mut Globals,f:&mut impl Foreign) {
        if self.dead{return;}
        if let Some(c)=self.control.take(){
            if self.own {
                if let Value::Handle(p)=f.read(c,Read::Parent){f.write(p,Write::ChildRemove(c));}
                f.write(c,Write::Shutdown);
            }
            f.release(c);
        }
        if let Some(p)=self.parent.take(){f.release(p);}
        if let Some(v)=g.vocabulary {if f.release(v)==0 {g.vocabulary=None;}}
        if let Some(p)=self.procedure.take(){f.release(p);}
        self.dead=true;
    }
    /// Successful builds still return Ok(false). Unsupported is an additional
    /// portable-domain channel, never presented as a native boolean result.
    pub fn set_parameters(&mut self,p:Properties<'_>,g:&Globals,f:&mut impl Foreign)->Result<bool> {
        let Some(clsid)=p.number(0xffffffff).filter(|v|*v!=0) else{return Ok(false);};
        p.validate()?;
        let iid=p.number(0xfffffffe).unwrap_or_else(||g.classes.get(&clsid).copied().unwrap_or(0));
        self.control=None; // Q1: deliberately do not Release the previous control.
        self.control=f.create(clsid,WIN_IID);
        if self.control.is_none(){
            if let Some((&fallback,_))=g.classes.iter().find(|(_,i)|**i==iid){self.control=f.create(fallback,WIN_IID);}
            if self.control.is_none(){self.control=f.create(0x478d1e6f,WIN_IID);}
        }
        let Some(c)=self.control else{return Ok(false);};
        param(c,0xe85eebde,Value::U32(clsid),f);
        if iid!=0 {param(c,0x08d6e462,Value::U32(iid),f);}
        self.own=true;
        let kind=g.interpreters.get(&iid).copied();
        if let Some(kind)=kind {interpret(kind,c,p,true,f)?;}
        interpret_common(c,p,true,f)?;
        self.do_exceptions(c,true,f);
        f.write(c,Write::Init);
        if let Some(parent)=self.parent.take(){f.write(parent,Write::ChildAdd(c));f.release(parent);}else{
            let parent=f.winmgr_slot3(); // LC5: opaque foreign identity, never root/main.
            f.write(parent,Write::ChildAdd(c));
        }
        f.write(c,Write::Front);
        if let Some(kind)=kind {interpret(kind,c,p,false,f)?;}
        interpret_common(c,p,false,f)?;
        Ok(false)
    }
}

fn replace(slot:&mut Option<Handle>,new:Option<Handle>,f:&mut impl Foreign) {
    if *slot!=new {if let Some(n)=new{f.add_ref(n);}if let Some(old)=*slot{f.release(old);}*slot=new;}
}
fn param(h:Handle,k:u32,v:Value,f:&mut impl Foreign){f.write(h,Write::Param(k,v));}
fn word(v:Value)->Result<u32>{match v{Value::U32(v)=>Ok(v),_=>Err(Unsupported::ForeignResult("uint32"))}}
fn handle(v:Value)->Result<Handle>{match v{Value::Handle(v)=>Ok(v),_=>Err(Unsupported::ForeignResult("handle"))}}
fn words(v:Value,n:usize)->Result<Vec<u32>>{match v{Value::Words(v) if v.len()==n=>Ok(v),_=>Err(Unsupported::ForeignResult("word tuple"))}}

#[cfg(test)]
mod tests;
