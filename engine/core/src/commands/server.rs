// SPDX-License-Identifier: GPL-3.0-or-later
//! cGZCommandServer: gz-framework-core §4, reusing the command algorithms.
use super::*;
use crate::gz_framework::{Result, Unsupported};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};
use std::sync::RwLock;

pub const IID: u32 = 0xCB90253A;
pub const SERVICE_ID: u32 = 0xEB903A32;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Info { pub name: Vec<u8>, pub parameters: Vec<u8> }
pub trait Dispatcher {
    fn execute(&self, id: u32, input: Option<&SetRef>, output: Option<&SetRef>) -> i32;
}
impl Dispatcher for Message2CommandDispatcher {
    fn execute(&self, id: u32, input: Option<&SetRef>, output: Option<&SetRef>) -> i32 {
        self.execute_shared(id,input,output)
    }
}
pub trait Generator {
    fn registered(&self, id: u32, name: &[u8], info: &[u8]);
    fn unregistered(&self, id: u32, name: &[u8], info: &[u8]);
    fn from_text(&self, text: &[u8], length: u32, id: &mut u32, input: &mut Option<SetRef>) -> bool;
    fn to_text(&self, id: u32, input: Option<&SetRef>, out: &mut Vec<u8>) -> bool;
    fn output_text(&self, output: Option<&SetRef>, out: &mut Vec<u8>) -> bool;
    fn shutdown(&self);
}
struct StringGenerator(RwLock<StringCommandGenerator>);
impl Generator for StringGenerator {
    fn registered(&self, id: u32, name: &[u8], info: &[u8]) { self.0.write().unwrap().command_registered(id,name,info); }
    fn unregistered(&self, id: u32, name: &[u8], info: &[u8]) { self.0.write().unwrap().command_unregistered(id,name,info); }
    fn from_text(&self, text: &[u8], length: u32, id: &mut u32, input: &mut Option<SetRef>) -> bool {
        self.0.write().unwrap().convert_string_to_command(text,length,id,input)
    }
    fn to_text(&self, id: u32, input: Option<&SetRef>, out: &mut Vec<u8>) -> bool {
        self.0.read().unwrap().convert_command_to_string(id,input,out)
    }
    fn output_text(&self, output: Option<&SetRef>, out: &mut Vec<u8>) -> bool {
        self.0.read().unwrap().convert_command_output_to_string(output,out)
    }
    fn shutdown(&self) { let mut g = self.0.write().unwrap(); g.shutdown(); g.release(); }
}
#[derive(Default)]
struct State {
    commands: BTreeMap<u32,Info>,
    generators: Vec<Rc<dyn Generator>>,
    active: Option<Rc<dyn Dispatcher>>,
    default_dispatcher: Option<Rc<Message2CommandDispatcher>>,
    default_generator: Option<Rc<dyn Generator>>,
    generate_description: bool,
}
pub struct Server { state: RwLock<State>, conv: Rc<dyn ValueConvert> }
struct ServerLink(Weak<Server>);
impl CommandServer for ServerLink {
    fn get_command_id(&self, name: &[u8]) -> u32 { self.0.upgrade().map_or(0,|s| s.get_command_id(name)) }
    fn get_command_name(&self, id: u32) -> Option<Vec<u8>> { self.0.upgrade()?.get_command_name(id) }
    fn execute_command_id(&self, id: u32, input: Option<&SetRef>, output: Option<&SetRef>) -> i32 {
        // Native null-dispatcher behavior is open. The public checked API exposes it.
        self.0.upgrade().expect("live command service").execute(id,input,output).expect("GZF3: null dispatcher")
    }
}
impl Server {
    pub fn new(conv: Rc<dyn ValueConvert>) -> Rc<Self> { Rc::new(Self { state: RwLock::new(State::default()), conv }) }
    pub fn command_count(&self) -> usize { self.state.read().unwrap().commands.len() }
    pub fn generator_count(&self) -> usize { self.state.read().unwrap().generators.len() }
    pub fn dispatcher(&self) -> Option<Rc<dyn Dispatcher>> { self.state.read().unwrap().active.clone() }
    pub fn set_dispatcher(&self, dispatcher: Option<Rc<dyn Dispatcher>>) -> bool { self.state.write().unwrap().active = dispatcher; true }
    pub fn create_parameter_set(&self) -> SetRef { CommandParameterSet::new_owned() }
    pub fn init(self: &Rc<Self>, messages: Option<Rc<dyn CommandMessageServer>>) -> bool {
        let link: Rc<dyn CommandServer> = Rc::new(ServerLink(Rc::downgrade(self)));
        if self.dispatcher().is_none() {
            let mut d = Message2CommandDispatcher::new(self.conv.clone()); d.add_ref();
            d.init(messages,Some(link.clone()));
            let d = Rc::new(d); let mut s = self.state.write().unwrap();
            s.active = Some(d.clone()); s.default_dispatcher = Some(d);
        }
        let mut g = StringCommandGenerator::new(self.conv.clone()); g.add_ref(); g.init(Some(link));
        let g: Rc<dyn Generator> = Rc::new(StringGenerator(RwLock::new(g)));
        let mut s = self.state.write().unwrap(); s.generators.push(g.clone()); s.default_generator = Some(g); true
    }
    pub fn register(&self, id: u32, name: Option<&[u8]>, parameters: Option<&[u8]>) -> bool {
        let Some(name) = name else { return false };
        let (info, generators) = {
            let mut s = self.state.write().unwrap();
            if id == 0 || s.commands.contains_key(&id) { return false }
            let info = Info { name: name.to_vec(), parameters: description(name,parameters,s.generate_description) };
            s.commands.insert(id,info.clone()); (info,s.generators.clone())
        };
        // Callbacks run without a registry lock. Ordering among generators is GZF3.
        for g in generators { g.registered(id,&info.name,&info.parameters); } true
    }
    pub fn parameter_info(&self, id: u32, text: Option<&[u8]>) -> bool {
        let mut s = self.state.write().unwrap(); let generate = s.generate_description;
        let Some(info) = s.commands.get_mut(&id) else { return false };
        if text.is_some_and(|t| !t.is_empty()) || generate { info.parameters = description(&info.name,text,generate); } true
    }
    pub fn set_generate_description(&self, enabled: bool) { self.state.write().unwrap().generate_description = enabled; }
    pub fn unregister(&self, id: u32) -> bool {
        let (info,generators) = { let mut s = self.state.write().unwrap();
            let Some(info) = s.commands.remove(&id) else { return false }; (info,s.generators.clone()) };
        for g in generators { g.unregistered(id,&info.name,&info.parameters); } true
    }
    pub fn info(&self, id: u32, name: &mut Vec<u8>, parameters: Option<&mut Vec<u8>>) -> bool {
        let s = self.state.read().unwrap(); let Some(info) = s.commands.get(&id) else { return false };
        name.clone_from(&info.name); if let Some(out) = parameters { out.clone_from(&info.parameters); } true
    }
    pub fn get_command_id_from_string(&self, text: Option<&[u8]>, length: u32) -> u32 {
        let Some(text) = text else { return 0 }; let (ok,words) = parse_delimited_text(line_of(text,length),b' ',1);
        if !ok { return 0 } words.first().map_or(0,|w| self.get_command_id(w))
    }
    pub fn register_generator(&self, g: Rc<dyn Generator>) -> bool {
        let mut s = self.state.write().unwrap(); if s.generators.iter().any(|v| Rc::ptr_eq(v,&g)) { return false }
        s.generators.push(g); true
    }
    pub fn unregister_generator(&self, g: &Rc<dyn Generator>) -> bool {
        let mut s = self.state.write().unwrap(); let Some(i) = s.generators.iter().position(|v| Rc::ptr_eq(v,g)) else { return false };
        s.generators.remove(i); true
    }
    pub fn execute(&self, id: u32, input: Option<&SetRef>, output: Option<&SetRef>) -> Result<i32> {
        let d = self.dispatcher().ok_or(Unsupported("GZF3"))?; Ok(d.execute(id,input,output))
    }
    pub fn convert_string(&self, text: &[u8], length: u32, id: &mut u32, input: &mut Option<SetRef>) -> bool {
        let gs = self.state.read().unwrap().generators.clone(); gs.iter().any(|g| g.from_text(text,length,id,input))
    }
    pub fn convert_command(&self, id: u32, input: Option<&SetRef>, out: &mut Vec<u8>) -> bool {
        let gs = self.state.read().unwrap().generators.clone(); gs.iter().any(|g| g.to_text(id,input,out))
    }
    pub fn convert_output(&self, output: Option<&SetRef>, out: &mut Vec<u8>) -> bool {
        let gs = self.state.read().unwrap().generators.clone(); gs.iter().any(|g| g.output_text(output,out))
    }
    pub fn execute_text(&self, text: &[u8], length: u32, output: Option<&SetRef>) -> Result<i32> {
        let mut id = 0; let mut input = None;
        if !self.convert_string(text,length,&mut id,&mut input) { return Ok(2) }
        let result = self.execute(id,input.as_ref(),output);
        if let Some(s) = input { s.borrow_mut().release(); } result
    }
    /// Native varargs formatting is a host responsibility; the resulting bytes
    /// execute through execute_text, never through a second dispatch algorithm.
    pub fn bulk_or_cancellable_enumeration(&self) -> Result<()> { Err(Unsupported("GZF3")) }
    pub fn shutdown(&self) -> Result<bool> {
        let mut s = self.state.write().unwrap(); s.commands.clear(); s.active = None;
        if let Some(d) = s.default_dispatcher.as_mut() {
            let Some(d) = Rc::get_mut(d) else { return Err(Unsupported("GZF3")) }; d.shutdown(); d.release();
        }
        s.default_dispatcher = None;
        let g = s.default_generator.take(); s.generators.clear(); drop(s);
        if let Some(g) = g { g.shutdown(); } Ok(true)
    }
}
impl CommandServer for Server {
    fn get_command_id(&self, name: &[u8]) -> u32 {
        self.state.read().unwrap().commands.iter().find(|(_,i)| make_lower(&i.name) == make_lower(name)).map_or(0,|(&id,_)| id)
    }
    fn get_command_name(&self, id: u32) -> Option<Vec<u8>> { self.state.read().unwrap().commands.get(&id).map(|i| i.name.clone()) }
    fn execute_command_id(&self, id: u32, input: Option<&SetRef>, output: Option<&SetRef>) -> i32 {
        self.execute(id,input,output).expect("GZF3: null dispatcher has no specified native result")
    }
}
fn description(name: &[u8], text: Option<&[u8]>, generate: bool) -> Vec<u8> {
    if let Some(t) = text.filter(|t| !t.is_empty()) { return t.to_vec() }
    if !generate { return Vec::new() }
    let mut result = b"Format: ".to_vec(); result.extend_from_slice(name); result.extend_from_slice(b"\nOutput: <none>"); result
}
