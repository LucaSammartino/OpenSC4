// SPDX-License-Identifier: GPL-3.0-or-later
//! Deterministic foreign inputs. No owner operation is mocked/delegated.
use fc4_core::lua_script_server::*;
use std::collections::{BTreeMap, VecDeque};

pub type Callback = Box<dyn FnOnce(&mut Mock, &mut ScriptServer)>;
pub struct Hook {
    pub when: fn(&Primitive) -> bool,
    pub callback: Callback,
}

pub struct Mock {
    pub calls: Vec<Primitive>,
    pub lua_results: VecDeque<Option<Handle>>,
    pub open: bool,
    pub generator: Option<Handle>,
    pub generator_init: bool,
    pub framework: bool,
    pub service: bool,
    pub message_storage: bool,
    pub compile: VecDeque<i32>,
    pub created: VecDeque<(i32, Option<Handle>)>,
    pub thread_ids: BTreeMap<Handle, i32>,
    pub statuses: BTreeMap<Handle, VecDeque<i32>>,
    pub tokens: Vec<Vec<u8>>,
    pub file: Option<Vec<u8>>,
    pub file_exists: bool,
    pub path_component: Vec<u8>,
    pub debug: (Vec<u8>, i32),
    pub lines: VecDeque<i32>,
    pub raw_field: u16,
    pub wrapper: Option<Handle>,
    pub contexts: BTreeMap<Handle, RecordToken>,
    pub top: i32,
    pub registry_type: i32,
    pub resume_result: bool,
    pub send_result: bool,
    pub hooks: VecDeque<Hook>,
    pub sent: Vec<(u32, i32, Option<i32>)>,
    pub observations: Vec<(u32, i32, Option<i32>, Option<u32>)>,
    message: (u32, i32, Option<i32>),
}

impl Default for Mock {
    fn default() -> Self {
        Self {
            calls: vec![], lua_results: VecDeque::new(), open: true,
            generator: Some(Handle(2)), generator_init: true, framework: true,
            service: true, message_storage: true, compile: VecDeque::new(), created: VecDeque::new(),
            thread_ids: BTreeMap::new(), statuses: BTreeMap::new(), tokens: vec![],
            file: None, file_exists: false, path_component: b"basename".to_vec(),
            debug: (b"job".to_vec(), 10), lines: VecDeque::new(), raw_field: 0,
            wrapper: Some(Handle(107)), contexts: BTreeMap::new(), top: 3,
            registry_type: 5, resume_result: true, send_result: true, hooks: VecDeque::new(),
            sent: vec![], observations: vec![], message: (0, 0, None),
        }
    }
}

impl Mock {
    pub fn initialized() -> (ScriptServer, Self) {
        let mut owner = ScriptServer::new();
        let mut host = Self::default();
        assert_eq!(owner.init(&mut host), Ok(true));
        host.clear();
        (owner, host)
    }
    pub fn clear(&mut self) {
        self.calls.clear(); self.sent.clear(); self.observations.clear();
    }
    pub fn create(&mut self, owner: &mut ScriptServer, id: i32) {
        self.created.push_back((id, Some(Handle((id as i64 + 1000) as u64))));
        assert_eq!(owner.run_script_string(self, b"source", b"job", b""), Ok(id));
    }
    pub fn with_script(id: i32) -> (ScriptServer, Self) {
        let (mut owner, mut host) = Self::initialized();
        host.create(&mut owner, id);
        host.wrapper = owner.record(id).unwrap().thread();
        host.clear();
        (owner, host)
    }
    pub fn status(&mut self, owner: &ScriptServer, id: i32, values: &[i32]) {
        self.statuses.insert(owner.record(id).unwrap().thread().unwrap(), values.iter().copied().collect());
    }
    pub fn count(&self, predicate: fn(&Primitive) -> bool) -> usize {
        self.calls.iter().filter(|c| predicate(c)).count()
    }
    pub fn position(&self, predicate: fn(&Primitive) -> bool) -> usize {
        self.calls.iter().position(predicate).expect("required foreign invocation")
    }
    pub fn stack_ops(&self, thread: Handle) -> Vec<StackOp> {
        self.calls.iter().filter_map(|c| match c {
            Primitive::Stack { thread: t, op } if *t == thread => Some(op.clone()), _ => None
        }).collect()
    }
    pub fn hook(&mut self, when: fn(&Primitive) -> bool, callback: impl FnOnce(&mut Mock, &mut ScriptServer) + 'static) {
        self.hooks.push_back(Hook { when, callback: Box::new(callback) });
    }
}

impl Foreign for Mock {
    fn invoke(&mut self, owner: &mut ScriptServer, call: Primitive) -> Result<Value, Failure> {
        self.calls.push(call.clone());
        let reply = match &call {
            Primitive::BaseQuery(_) => Value::Interface(Some(Handle(88))),
            Primitive::CreateLua { .. } => Value::Handle(self.lua_results.pop_front().unwrap_or(Some(Handle(1)))),
            Primitive::Open { .. } => Value::Bool(self.open),
            Primitive::CreateGenerator { .. } => Value::Handle(self.generator),
            Primitive::InitGenerator { .. } => Value::Bool(self.generator_init),
            Primitive::GeneratorTarget(_) => Value::Handle(Some(Handle(20))),
            Primitive::FrameworkAvailable => Value::Bool(self.framework),
            Primitive::MainThread(_) => Value::Handle(Some(Handle(10))),
            Primitive::LoadBuffer { .. } => Value::Int(self.compile.pop_front().unwrap_or(0)),
            Primitive::FileExists(_) => Value::Bool(self.file_exists),
            Primitive::PathName { .. } => Value::Bytes(self.path_component.clone()),
            Primitive::ReadFile { .. } => Value::File(self.file.clone()),
            Primitive::CreateThread { .. } => {
                let (id, thread) = self.created.pop_front().expect("test must supply VM ID/thread");
                if let Some(t) = thread { self.thread_ids.insert(t, id); }
                Value::CreatedThread { id, thread }
            },
            Primitive::SetUserContext { thread, record } => {
                self.contexts.insert(*thread, *record); Value::Unit
            },
            Primitive::ThreadId(t) => Value::Int(*self.thread_ids.get(t).expect("known foreign thread")),
            Primitive::RawContext(t) => Value::Handle(Some(*t)),
            Primitive::Status(t) => Value::Int(self.statuses.get_mut(t).and_then(|q| q.pop_front()).unwrap_or(1)),
            Primitive::ResumeThread { .. } => Value::Bool(self.resume_result),
            Primitive::ParseCommandLine(_) => Value::Tokens(self.tokens.clone()),
            Primitive::Lowercase(bytes) => Value::Bytes(bytes.iter().map(u8::to_ascii_lowercase).collect()),
            Primitive::DebugLocation { .. } => Value::Location { name: self.debug.0.clone(), line: self.debug.1 },
            Primitive::GetTop(_) => Value::Int(self.top),
            Primitive::WrapperFromState(_) => Value::Handle(self.wrapper),
            Primitive::UserContext(t) => Value::Record(self.contexts.get(t).copied()),
            Primitive::RawYieldField(_) => Value::U16(self.raw_field),
            Primitive::DebugLine { .. } => Value::Int(self.lines.pop_front().expect("supplied debug line")),
            Primitive::Type { .. } => Value::Int(self.registry_type),
            Primitive::FindMessageService { .. } => Value::Handle(self.service.then_some(Handle(90))),
            Primitive::AllocateMessage { .. } => {
                self.message = (0, 0, None); Value::Handle(self.message_storage.then_some(Handle(91)))
            },
            Primitive::MessageType { value, .. } => { self.message.0 = *value; Value::Unit },
            Primitive::MessageId { value, .. } => { self.message.1 = *value; Value::Unit },
            Primitive::MessagePayload { value, .. } => { self.message.2 = Some(*value); Value::Unit },
            Primitive::SendMessage { .. } => {
                self.sent.push(self.message);
                let r = owner.record(self.message.1);
                self.observations.push((self.message.0, self.message.1, r.map(|r| r.pause_count), r.map(|r| r.resume_transitions)));
                Value::Bool(self.send_result)
            },
            _ => Value::Unit,
        };
        if self.hooks.front().is_some_and(|h| (h.when)(&call)) {
            let hook = self.hooks.pop_front().unwrap();
            (hook.callback)(self, owner);
        }
        Ok(reply)
    }
}
