// SPDX-License-Identifier: GPL-3.0-or-later
//! Command bindings from lua-host §§6.3.2.1–6.3.2.3.
use super::*;
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, PartialEq)]
pub enum Parameter {
    Bool(bool),
    Number(f64),
    String(Vec<u8>),
}
#[derive(Clone, Debug, Default)]
pub struct CommandRecord {
    pub id: u32,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endpoint {
    Generator,
    CommandServer,
    MainThread,
    ParameterSet,
}
pub trait GeneratorApi: ThreadApi {
    fn register_generator(&mut self, server: Handle, generator: Handle) -> bool;
    fn unregister_generator(&mut self, server: Handle, generator: Handle) -> bool;
    /// Borrowed view, just like cIGZLua5::AsIGZLua5Thread.
    fn main_thread(&mut self, lua: Handle) -> Result<Thread>;
    fn add_ref_endpoint(&mut self, endpoint: Endpoint, object: Handle);
    fn release_endpoint(&mut self, endpoint: Endpoint, object: Handle);
    /// Constructed/retained reference, or null; null does not fail Init.
    fn create_parameters(&mut self) -> Option<Handle>;
    fn clear_parameters(&mut self, set: Handle, flags: u32);
    fn set_parameter(&mut self, set: Handle, key: u32, value: Parameter);
    fn command(&mut self, server: Handle, id: u32, input: Handle, output: Handle) -> i32;
    fn first_parameter(&mut self, set: Handle) -> u32;
    fn next_parameter(&mut self, set: Handle, key: u32) -> u32;
    fn parameter(&mut self, set: Handle, key: u32) -> Option<Handle>;
    fn variant_type(&mut self, value: Handle) -> u32;
    fn variant_bool(&mut self, value: Handle) -> bool;
    fn variant_number(&mut self, value: Handle) -> f64;
    fn variant_string(&mut self, value: Handle) -> Vec<u8>;
}

#[derive(Debug)]
pub struct CommandGenerator {
    pub identity: Handle,
    server: Option<Handle>,
    thread: Option<Thread>,
    name: Vec<u8>,
    input: Option<Handle>,
    output: Option<Handle>,
    records: Vec<CommandRecord>,
    free: VecDeque<usize>,
    ids: BTreeMap<u32, usize>,
    references: u32,
}
impl CommandGenerator {
    pub fn new(identity: Handle) -> Self {
        Self {
            identity,
            server: None,
            thread: None,
            name: Vec::new(),
            input: None,
            output: None,
            records: vec![CommandRecord::default(); 500],
            free: VecDeque::new(),
            ids: BTreeMap::new(),
            references: 0,
        }
    }
    pub fn supports(iid: u32) -> bool {
        matches!(iid, 1 | GENERATOR_IID | COMMAND_GENERATOR_IID)
    }
    /// Borrowed sibling identity: this conversion neither acquires nor releases.
    pub fn as_command_generator(&self) -> Handle {
        self.identity
    }
    pub fn query_interface<A: GeneratorApi>(&mut self, api: &mut A, iid: u32) -> Option<Handle> {
        if !Self::supports(iid) {
            return None;
        }
        self.add_ref();
        api.add_ref_endpoint(Endpoint::Generator, self.identity);
        Some(self.identity)
    }
    pub fn name(&self) -> &[u8] {
        &self.name
    }
    pub fn retained_server(&self) -> Option<Handle> {
        self.server
    }
    pub fn retained_thread(&self) -> Option<Handle> {
        self.thread.as_ref().map(|t| t.identity)
    }
    pub fn init<A: GeneratorApi>(
        &mut self,
        api: &mut A,
        server: Option<Handle>,
        lua: Option<Handle>,
        name: &[u8],
    ) -> Result<bool> {
        if name.is_empty() {
            return Ok(false);
        }
        let Some(lua) = lua else {
            return Ok(false);
        };
        let Some(server) = server else {
            return Ok(false);
        };
        if !api.register_generator(server, self.identity) {
            return Ok(false);
        }
        let thread = api.main_thread(lua)?;
        let valid = {
            let mut s = thread.session(api, Reporter::Default)?;
            s.get_global(name)?;
            let kind = s.execute(Op::Type(-1))?.int()?;
            match kind {
                5 => {
                    s.pop(1)?;
                    true
                }
                0 => {
                    s.pop(1)?;
                    s.execute(Op::NewTable)?;
                    s.set_global(name)?;
                    true
                }
                _ => {
                    s.pop(1)?;
                    false
                }
            }
        };
        if !valid {
            api.unregister_generator(server, self.identity);
            return Ok(false);
        }
        api.add_ref_endpoint(Endpoint::MainThread, thread.identity);
        thread.add_ref();
        if let Some(old) = self.thread.replace(thread) {
            api.release_endpoint(Endpoint::MainThread, old.identity);
            old.release(api);
        }
        api.add_ref_endpoint(Endpoint::CommandServer, server);
        if let Some(old) = self.server.replace(server) {
            api.release_endpoint(Endpoint::CommandServer, old);
        }
        self.name = name.to_vec();
        let input = api.create_parameters();
        if let Some(old) = std::mem::replace(&mut self.input, input) {
            api.release_endpoint(Endpoint::ParameterSet, old);
        }
        let output = api.create_parameters();
        if let Some(old) = std::mem::replace(&mut self.output, output) {
            api.release_endpoint(Endpoint::ParameterSet, old);
        }
        self.clear_records();
        Ok(true)
    }
    pub fn shutdown<A: GeneratorApi>(&mut self, api: &mut A) -> bool {
        let status = self
            .server
            .map(|p| api.unregister_generator(p, self.identity))
            .unwrap_or(true);
        if let Some(server) = self.server.take() {
            api.release_endpoint(Endpoint::CommandServer, server);
        }
        if let Some(thread) = self.thread.take() {
            api.release_endpoint(Endpoint::MainThread, thread.identity);
            thread.release(api);
        }
        for parameter in [self.input.take(), self.output.take()].into_iter().flatten() {
            api.release_endpoint(Endpoint::ParameterSet, parameter);
        }
        self.clear_records();
        status
    }
    pub fn generic_init(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn clear_records(&mut self) {
        self.records.clear();
        self.free.clear();
        self.ids.clear();
    }
    pub fn records(&self) -> &[CommandRecord] {
        &self.records
    }
    pub fn command_index(&self, id: u32) -> Option<usize> {
        self.ids.get(&id).copied()
    }
    pub fn add_ref(&mut self) -> u32 {
        self.references = self.references.wrapping_add(1);
        self.references
    }
    pub fn release<A: GeneratorApi>(&mut self, api: &mut A) -> u32 {
        if self.references > 1 {
            self.references -= 1;
            return self.references;
        }
        self.references = 0;
        self.destroy(api);
        0
    }
    /// Explicit destructor: unlike Shutdown this never unregisters the endpoint.
    pub fn destroy<A: GeneratorApi>(&mut self, api: &mut A) {
        if let Some(server) = self.server.take() {
            api.release_endpoint(Endpoint::CommandServer, server);
        }
        for set in [self.output.take(), self.input.take()].into_iter().flatten() {
            api.release_endpoint(Endpoint::ParameterSet, set);
        }
        if let Some(thread) = self.thread.take() {
            api.release_endpoint(Endpoint::MainThread, thread.identity);
            thread.release(api);
        }
        self.name.clear();
        self.clear_records();
    }
    pub fn command_registered<A: GeneratorApi>(
        &mut self,
        api: &mut A,
        id: u32,
        name: &[u8],
        description: &[u8],
    ) -> Result<bool> {
        if let Some(index) = self.ids.get(&id) {
            return Ok(self
                .records
                .get(*index)
                .filter(|r| r.id == id)
                .is_some_and(|r| r.name == name));
        }
        if self.free.is_empty() {
            let start = self.records.len();
            self.records.resize(start + 20, CommandRecord::default());
            self.free.extend(start..start + 20);
        }
        let index = self.free.pop_front().unwrap();
        self.records[index] = CommandRecord {
            id,
            name: name.to_vec(),
            description: description.to_vec(),
        };
        self.ids.insert(id, index);
        let thread = self.thread.as_ref().ok_or(Error::InvalidContext)?;
        let mut s = thread.session(api, Reporter::Disabled)?;
        let top = s.get_top()?;
        s.get_global(&self.name)?;
        let valid = s.is_type(-1, 5)?;
        if valid {
            s.push_string(name)?;
            s.execute(Op::PushNumber(index as f64))?;
            s.execute(Op::PushLightUserdata(self.identity))?;
            s.execute(Op::PushLightUserdata(thread.identity))?;
            s.execute(Op::PushCClosure(Callback::Command, 3))?;
            s.execute(Op::SetTable(-3))?;
        }
        s.set_top(top)?;
        Ok(valid)
    }
    pub fn command_unregistered<A: GeneratorApi>(&mut self, api: &mut A, id: u32) -> Result<bool> {
        let Some(index) = self.ids.get(&id).copied() else {
            return Ok(false);
        };
        let Some(record) = self.records.get(index).filter(|r| r.id == id) else {
            return Ok(false);
        };
        self.free.push_front(index);
        self.ids.remove(&id);
        let mut s = self
            .thread
            .as_ref()
            .ok_or(Error::InvalidContext)?
            .session(api, Reporter::Disabled)?;
        s.get_global(&self.name)?;
        if s.is_type(-1, 5)? {
            s.push_string(&record.name)?;
            s.execute(Op::PushNil)?;
            s.execute(Op::SetTable(-3))?;
        }
        s.pop(1)?;
        Ok(true)
    }
    /// Captures are unowned, checked at invocation; no map/old-ID validation.
    pub fn callback<A: GeneratorApi>(
        &self,
        api: &mut A,
        invoking: &Thread,
        captured_generator: Option<Handle>,
        captured_main: Handle,
        index: f64,
    ) -> Result<i32> {
        guarded(api, |api| {
            self.callback_inner(api, invoking, captured_generator, captured_main, index)
        })
    }
    fn callback_inner<A: GeneratorApi>(
        &self,
        api: &mut A,
        invoking: &Thread,
        captured_generator: Option<Handle>,
        captured_main: Handle,
        index: f64,
    ) -> Result<i32> {
        let index = crate::x87::ftol32(index);
        let state = invoking.context().ok_or(Error::InvalidContext)?;
        let identity = api
            .invoke(Some(state), Op::GetExtraOwner)?
            .handle()?
            .ok_or(Error::InvalidContext)?;
        let mut s = Session {
            api,
            state,
            identity,
            thread_id: None,
            reporter: Reporter::Disabled,
        };
        let valid = captured_generator == Some(self.identity)
            && self.server.is_some()
            && self.retained_thread() == Some(captured_main)
            && index >= 0
            && (index as usize) < self.records.len();
        if !valid {
            s.execute(Op::PushNumber(2.0))?;
            return Ok(1);
        }
        let (input, output) = (
            self.input.ok_or(Error::InvalidContext)?,
            self.output.ok_or(Error::InvalidContext)?,
        );
        let nargs = s.get_top()?;
        s.api.clear_parameters(input, 0);
        s.api.clear_parameters(output, 0);
        for i in 1..=nargs {
            let value = match s.execute(Op::Type(i))?.int()? {
                1 => Parameter::Bool(s.execute(Op::ToBoolean(i))?.boolean()?),
                3 => Parameter::Number(s.execute(Op::ToNumber(i))?.number()?),
                4 => Parameter::String(s.execute(Op::ToString(i))?.bytes()?.unwrap_or_default()),
                _ => Parameter::Number(0.0),
            };
            s.api.set_parameter(input, (i - 1) as u32, value);
        }
        let status = s
            .api
            .command(self.server.unwrap(), self.records[index as usize].id, input, output);
        s.execute(Op::PushNumber(f64::from(status)))?;
        let mut count = 1;
        let mut key = s.api.first_parameter(output);
        while key != u32::MAX {
            if key != 0x2b999d2b {
                let op = match s.api.parameter(output, key) {
                    None => Op::PushNil,
                    Some(value) => match s.api.variant_type(value) {
                        1 => Op::PushBoolean(s.api.variant_bool(value)),
                        2..=14 => Op::PushNumber(s.api.variant_number(value)),
                        _ => Op::PushString(s.api.variant_string(value)),
                    },
                };
                s.execute(op)?;
                count += 1;
            }
            key = s.api.next_parameter(output, key);
        }
        Ok(count)
    }
}
