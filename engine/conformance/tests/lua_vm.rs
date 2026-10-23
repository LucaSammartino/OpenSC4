// SPDX-License-Identifier: GPL-3.0-or-later
//! Host conformance with an in-memory C API fixture, not a Lua interpreter.
//! Compiled chunks below have scripted foreign outcomes. Host callbacks execute
//! the actual core algorithms against stack/table primitives.
use fc4_core::lua_script_server::vm::{generator::*, *};
use fc4_core::lua_script_server::{Handle, GLOBALS, REGISTRY};
use std::collections::{BTreeMap, VecDeque};
include!("support/lua_vm_lifecycle.rs");

#[derive(Clone, Debug, PartialEq)]
enum V {
    Nil,
    Bool(bool),
    Number(f64),
    String(Vec<u8>),
    Table(usize),
    Function(Callback),
    Closure(Callback, Vec<V>),
    Chunk(i32, Vec<V>),
    Pointer(Handle),
}
impl V {
    fn kind(&self) -> i32 {
        match self {
            Self::Nil => 0,
            Self::Bool(_) => 1,
            Self::Number(_) => 3,
            Self::String(_) => 4,
            Self::Table(_) => 5,
            Self::Function(_) | Self::Closure(_, _) | Self::Chunk(_, _) => 6,
            Self::Pointer(_) => 7,
        }
    }
    fn key(&self) -> String {
        match self {
            Self::String(s) => format!("s:{}", String::from_utf8_lossy(s)),
            Self::Number(n) => format!("n:{n}"),
            Self::Pointer(p) => format!("p:{}", p.0),
            _ => panic!("invalid key {self:?}"),
        }
    }
}
struct Api {
    stack: Vec<V>,
    tables: Vec<BTreeMap<String, V>>,
    metas: BTreeMap<usize, usize>,
    pointers: BTreeMap<Handle, u32>,
    trace: Vec<(Op, u32)>,
    precision: u32,
    changes: Vec<u32>,
    reports: Vec<Report>,
    logs: Vec<Vec<u8>>,
    stderr: Vec<Option<Vec<u8>>>,
    load: VecDeque<(i32, Vec<V>)>,
    runtime_status: i32,
    gc_threshold: i32,
    open: bool,
    reserve: bool,
    fail: Option<Op>,
    generator_trace: Vec<String>,
    register: bool,
    unregister: bool,
    active: Handle,
    stacks: BTreeMap<Handle, Vec<V>>,
    backpointers: BTreeMap<Handle, Handle>,
    frames: BTreeMap<Handle, bool>,
    next_context: u64,
    factory: bool,
    destroy_hook: bool,
    create_hook_writes: bool,
    deny_destroy: Option<Handle>,
    destroy_hooks: Vec<Handle>,
    references: Vec<(bool, Handle)>,
    resume_values: Vec<V>,
    resume_frame: bool,
    params: BTreeMap<Handle, Vec<(u32, Parameter)>>,
    output_values: Vec<(u32, Option<(u32, Parameter)>)>,
    command_ids: Vec<u32>,
    command_status: i32,
    allocate_parameters: bool,
}
type Report = (Handle, Handle, i32, Option<Vec<u8>>);
impl Default for Api {
    fn default() -> Self {
        let mut a = Self {
            stack: vec![],
            tables: vec![BTreeMap::new(), BTreeMap::new()],
            metas: BTreeMap::new(),
            pointers: BTreeMap::new(),
            trace: vec![],
            precision: 24,
            changes: vec![],
            reports: vec![],
            logs: vec![],
            stderr: vec![],
            load: VecDeque::new(),
            runtime_status: 0,
            gc_threshold: 100,
            open: true,
            reserve: true,
            fail: None,
            generator_trace: vec![],
            register: true,
            unregister: true,
            active: Handle(20),
            stacks: BTreeMap::new(),
            backpointers: BTreeMap::from([(Handle(20), Handle(10))]),
            frames: BTreeMap::new(),
            next_context: 100,
            factory: true,
            destroy_hook: true,
            create_hook_writes: true,
            deny_destroy: None,
            destroy_hooks: vec![],
            references: vec![],
            resume_values: vec![],
            resume_frame: false,
            params: BTreeMap::new(),
            output_values: vec![],
            command_ids: vec![],
            command_status: 0,
            allocate_parameters: true,
        };
        a.global(b"_ALERT", V::Function(Callback::Alert));
        a
    }
}
impl Api {
    fn index(&self, i: i32) -> usize {
        if i > 0 {
            (i - 1) as usize
        } else {
            (self.stack.len() as i32 + i) as usize
        }
    }
    fn value(&self, i: i32) -> V {
        if i == GLOBALS {
            V::Table(0)
        } else if i == REGISTRY {
            V::Table(1)
        } else {
            self.stack.get(self.index(i)).cloned().unwrap_or(V::Nil)
        }
    }
    fn table(&self, i: i32) -> usize {
        if let V::Table(t) = self.value(i) {
            t
        } else {
            panic!("not table {i}")
        }
    }
    fn global(&mut self, name: &[u8], value: V) {
        self.tables[0].insert(V::String(name.to_vec()).key(), value);
    }
    fn new_table(&mut self) -> V {
        let id = self.tables.len();
        self.tables.push(BTreeMap::new());
        V::Table(id)
    }
    fn session(&mut self) -> Session<'_, Self> {
        thread().session(self, Reporter::Callback(Handle(99))).unwrap()
    }
    fn operations(&self) -> Vec<Op> {
        self.trace.iter().map(|(op, _)| op.clone()).collect()
    }
    fn call(&mut self, nargs: i32, nresults: i32, protected: bool) -> Result<i32> {
        let function = self.stack.len() - nargs as usize - 1;
        let arguments = self.stack.split_off(function + 1);
        let callable = self.stack.pop().unwrap();
        let (status, mut values) = match callable {
            V::Chunk(status, values) => (status, values),
            V::Function(callback) => {
                let caller = std::mem::replace(&mut self.stack, arguments);
                let result = thread()
                    .session(self, Reporter::Callback(Handle(99)))?
                    .dispatch_callback(callback);
                let result = match result {
                    Ok(count) => (0, self.stack[self.stack.len() - count as usize..].to_vec()),
                    Err(Error::Foreign(_)) if protected => (1, vec![V::String(b"callback error".to_vec())]),
                    Err(error) => {
                        self.stack = caller;
                        return Err(error);
                    }
                };
                self.stack = caller;
                result
            }
            _ if protected => (1, vec![V::String(b"not callable".to_vec())]),
            _ => return Err(Error::Foreign("panic/exit 1".into())),
        };
        if status == 0 && nresults >= 0 {
            values.resize(nresults as usize, V::Nil);
        }
        self.stack.extend(values);
        Ok(status)
    }
}
fn thread() -> Thread {
    Thread::from_foreign_view(Handle(10), ThreadKind::Main, Handle(20), 7, Some(Handle(10)))
}
impl LuaApi for Api {
    fn precision(&mut self) -> u32 {
        self.precision
    }
    fn set_precision(&mut self, bits: u32) {
        self.precision = bits;
        self.changes.push(bits);
    }
    fn compatibility_source(&mut self) -> Vec<u8> {
        b"foreign etc/compat.lua fixture".to_vec()
    }
    fn report_callback(&mut self, callback: Handle, thread: Handle, status: i32, message: Option<&[u8]>) {
        self.reports
            .push((callback, thread, status, message.map(<[u8]>::to_vec)));
    }
    fn log(&mut self, message: &[u8]) {
        self.logs.push(message.to_vec());
    }
    fn stderr(&mut self, message: Option<&[u8]>) {
        self.stderr.push(message.map(<[u8]>::to_vec));
    }
    fn invoke(&mut self, state: Option<Handle>, op: Op) -> Result<Output> {
        if let Some(state) = state {
            if self.active != state {
                let stack = std::mem::take(&mut self.stack);
                self.stacks.insert(self.active, stack);
                self.stack = self.stacks.remove(&state).unwrap_or_default();
                self.active = state;
            }
        }
        self.trace.push((op.clone(), self.precision));
        if self.fail.as_ref() == Some(&op) {
            return Err(Error::Foreign("injected".into()));
        }
        Ok(match op {
            Op::Open => Output::Handle(self.open.then_some(Handle(20))),
            Op::SetExtraOwner(view) => {
                self.backpointers.insert(self.active, view);
                Output::None
            }
            Op::ClearExtraOwner => {
                self.backpointers.remove(&self.active);
                Output::None
            }
            Op::GetExtraOwner => Output::Handle(self.backpointers.get(&self.active).copied()),
            Op::NewThread => {
                let context = Handle(self.next_context);
                self.next_context += 1;
                self.stack.push(V::Pointer(context));
                Output::Handle(Some(context))
            }
            Op::Move(destination, count) => {
                let values = self.stack.split_off(self.stack.len() - count as usize);
                self.stacks.entry(destination).or_default().extend(values);
                Output::None
            }
            Op::HasActivationFrame => Output::Bool(self.frames.get(&self.active).copied().unwrap_or(false)),
            Op::Resume(_) => {
                self.stack = self.resume_values.clone();
                self.frames.insert(self.active, self.resume_frame);
                Output::Int(self.runtime_status)
            }
            Op::Yield(n) => Output::Int(n),
            Op::OpenLibrary(Library::Coroutine) => {
                let table = self.new_table();
                self.global(b"coroutine", table);
                Output::None
            }
            Op::Close | Op::OpenLibrary(_) => Output::None,
            Op::GetGCCount => Output::Int(25),
            Op::GetGCThreshold => Output::Int(self.gc_threshold),
            Op::SetGCThreshold(value) => {
                self.gc_threshold = value;
                Output::None
            }
            Op::GetTop => Output::Int(self.stack.len() as i32),
            Op::SetTop(i) => {
                let len = if i >= 0 {
                    i as usize
                } else {
                    (self.stack.len() as i32 + i + 1) as usize
                };
                self.stack.resize(len, V::Nil);
                Output::None
            }
            Op::PushNil => {
                self.stack.push(V::Nil);
                Output::None
            }
            Op::PushBoolean(value) => {
                self.stack.push(V::Bool(value));
                Output::None
            }
            Op::ToBoolean(index) => Output::Bool(!matches!(self.value(index), V::Nil | V::Bool(false))),
            Op::PushNumber(n) => {
                self.stack.push(V::Number(n));
                Output::None
            }
            Op::PushLightUserdata(pointer) => {
                self.stack.push(V::Pointer(pointer));
                Output::None
            }
            Op::PushString(s) | Op::PushLString(s) => {
                self.stack.push(V::String(s));
                Output::None
            }
            Op::PushCClosure(c, count) => {
                if count == 0 {
                    self.stack.push(V::Function(c));
                } else {
                    let values = self.stack.split_off(self.stack.len() - count as usize);
                    self.stack.push(V::Closure(c, values));
                }
                Output::None
            }
            Op::PushValue(i) => {
                self.stack.push(self.value(i));
                Output::None
            }
            Op::Insert(i) => {
                let index = self.index(i);
                let v = self.stack.pop().unwrap();
                self.stack.insert(index, v);
                Output::None
            }
            Op::Remove(i) => {
                self.stack.remove(self.index(i));
                Output::None
            }
            Op::Type(i) => Output::Int(self.value(i).kind()),
            Op::IsNumber(i) => Output::Bool(matches!(self.value(i), V::Number(_))),
            Op::ToNumber(i) => Output::Number(if let V::Number(n) = self.value(i) { n } else { 0.0 }),
            Op::ToString(i) => Output::Bytes(match self.value(i) {
                V::String(s) => Some(s),
                V::Number(n) => Some(n.to_string().into_bytes()),
                _ => None,
            }),
            Op::NewTable => {
                let t = self.new_table();
                self.stack.push(t);
                Output::None
            }
            Op::RawGet(i) | Op::GetTable(i) => {
                let table = self.table(i);
                let key = self.stack.pop().unwrap();
                let value = if key == V::Nil {
                    V::Nil
                } else {
                    self.tables[table].get(&key.key()).cloned().unwrap_or(V::Nil)
                };
                self.stack.push(value);
                Output::None
            }
            Op::RawSet(i) | Op::SetTable(i) => {
                let table = self.table(i);
                let value = self.stack.pop().unwrap();
                let key = self.stack.pop().unwrap();
                if key == V::Nil {
                    return Err(Error::Foreign("nil table key".into()));
                }
                self.tables[table].insert(key.key(), value);
                Output::None
            }
            Op::GetMetatable(i) => {
                let meta = match self.value(i) {
                    V::Table(t) => self.metas.get(&t).copied(),
                    V::Pointer(p) => self.metas.get(&(p.0 as usize + 1_000_000)).copied(),
                    _ => None,
                };
                if let Some(t) = meta {
                    self.stack.push(V::Table(t));
                }
                Output::Int(i32::from(meta.is_some()))
            }
            Op::SetMetatable(i) => {
                let target = self.value(i);
                let meta = self.stack.pop().unwrap();
                if let V::Table(meta) = meta {
                    let key = match target {
                        V::Table(t) => t,
                        V::Pointer(p) => p.0 as usize + 1_000_000,
                        _ => return Err(Error::Foreign("cannot set metatable".into())),
                    };
                    self.metas.insert(key, meta);
                } else {
                    return Err(Error::Foreign("cannot set metatable".into()));
                }
                Output::Int(1)
            }
            Op::Next(i) => {
                let table = self.table(i);
                let previous = self.stack.pop().unwrap();
                let next = self.tables[table]
                    .iter()
                    .find(|(k, _)| previous == V::Nil || **k > previous.key())
                    .map(|(k, v)| (k.clone(), v.clone()));
                if let Some((k, v)) = next {
                    let key = if let Some(n) = k.strip_prefix("n:") {
                        V::Number(n.parse().unwrap())
                    } else {
                        V::String(k.as_bytes()[2..].to_vec())
                    };
                    self.stack.extend([key, v]);
                    Output::Int(1)
                } else {
                    Output::Int(0)
                }
            }
            Op::PCall(a, r, _) => Output::Int(self.call(a, r, true)?),
            Op::Call(a, r) => {
                self.call(a, r, false)?;
                Output::None
            }
            Op::LoadBuffer { .. } | Op::LoadFile(_) => {
                let (status, values) = self.load.pop_front().unwrap_or((0, vec![]));
                if status == 0 {
                    self.stack.push(V::Chunk(self.runtime_status, values));
                } else {
                    self.stack.extend(values);
                }
                Output::Int(status)
            }
            Op::ToUserdata(i) => Output::Handle(if let V::Pointer(p) = self.value(i) {
                Some(p)
            } else {
                None
            }),
            Op::NewUserdata(size) => {
                assert_eq!(size, 4);
                let p = Handle(1000 + self.pointers.len() as u64);
                self.pointers.insert(p, 0);
                self.stack.push(V::Pointer(p));
                Output::Handle(Some(p))
            }
            Op::WritePointer32(p, value) => {
                *self.pointers.get_mut(&p).unwrap() = value;
                Output::None
            }
            Op::ReadPointer32(p) => Output::Handle(Some(Handle(u64::from(self.pointers[&p])))),
            Op::Unref(table, reference) => {
                let table = self.table(table);
                self.tables[table].remove(&V::Number(f64::from(reference)).key());
                Output::None
            }
            Op::RawGetI(table, reference) => {
                let table = self.table(table);
                let value = self.tables[table]
                    .get(&V::Number(f64::from(reference)).key())
                    .cloned()
                    .unwrap_or(V::Nil);
                self.stack.push(value);
                Output::None
            }
            Op::Ref(table) => {
                let table = self.table(table);
                let value = self.stack.pop().unwrap();
                if value == V::Nil {
                    Output::Int(-1)
                } else {
                    let reference = self.tables[table].len() as i32 + 1;
                    self.tables[table].insert(V::Number(f64::from(reference)).key(), value);
                    Output::Int(reference)
                }
            }
            Op::Raise => return Err(Error::Foreign("panic/exit 1".into())),
            other => panic!("fixture has no semantics for {other:?}"),
        })
    }
}
impl GeneratorApi for Api {
    fn create_parameters(&mut self) -> Option<Handle> {
        self.generator_trace.push("create ParameterSet".into());
        if !self.allocate_parameters {
            return None;
        }
        let handle = Handle(500 + self.params.len() as u64);
        self.params.insert(handle, vec![]);
        Some(handle)
    }
    fn clear_parameters(&mut self, set: Handle, flags: u32) {
        assert_eq!(flags, 0);
        self.params.get_mut(&set).unwrap().clear();
    }
    fn set_parameter(&mut self, set: Handle, key: u32, value: Parameter) {
        self.params.get_mut(&set).unwrap().push((key, value));
    }
    fn command(&mut self, _: Handle, id: u32, _: Handle, _: Handle) -> i32 {
        self.command_ids.push(id);
        self.command_status
    }
    fn first_parameter(&mut self, _: Handle) -> u32 {
        self.output_values.first().map(|v| v.0).unwrap_or(u32::MAX)
    }
    fn next_parameter(&mut self, _: Handle, key: u32) -> u32 {
        let index = self.output_values.iter().position(|v| v.0 == key).unwrap();
        self.output_values.get(index + 1).map(|v| v.0).unwrap_or(u32::MAX)
    }
    fn parameter(&mut self, _: Handle, key: u32) -> Option<Handle> {
        let index = self.output_values.iter().position(|v| v.0 == key).unwrap();
        self.output_values[index].1.as_ref().map(|_| Handle(index as u64))
    }
    fn variant_type(&mut self, value: Handle) -> u32 {
        self.output_values[value.0 as usize].1.as_ref().unwrap().0
    }
    fn variant_bool(&mut self, value: Handle) -> bool {
        if let Parameter::Bool(v) = self.output_values[value.0 as usize].1.as_ref().unwrap().1 {
            v
        } else {
            panic!("not bool")
        }
    }
    fn variant_number(&mut self, value: Handle) -> f64 {
        if let Parameter::Number(v) = self.output_values[value.0 as usize].1.as_ref().unwrap().1 {
            v
        } else {
            panic!("not number")
        }
    }
    fn variant_string(&mut self, value: Handle) -> Vec<u8> {
        if let Parameter::String(v) = &self.output_values[value.0 as usize].1.as_ref().unwrap().1 {
            v.clone()
        } else {
            panic!("not string")
        }
    }
    fn register_generator(&mut self, _: Handle, _: Handle) -> bool {
        self.generator_trace.push("register".into());
        self.register
    }
    fn unregister_generator(&mut self, _: Handle, _: Handle) -> bool {
        self.generator_trace.push("unregister".into());
        self.unregister
    }
    fn main_thread(&mut self, _: Handle) -> Result<Thread> {
        self.generator_trace.push("main borrowed".into());
        Ok(thread())
    }
    fn add_ref_endpoint(&mut self, e: Endpoint, _: Handle) {
        self.generator_trace.push(format!("acquire {e:?}"));
    }
    fn release_endpoint(&mut self, e: Endpoint, _: Handle) {
        self.generator_trace.push(format!("release {e:?}"));
    }
}

impl ThreadApi for Api {
    fn registry_identity(&mut self, _: Handle) -> Handle {
        Handle(21)
    }
    fn factory_thread(&mut self, class: u32, iid: u32) -> Result<Option<Thread>> {
        assert_eq!(class, THREAD_CLASS);
        assert_eq!(iid, THREAD_IID);
        Ok(self.factory.then(|| {
            let thread = Thread::new(Handle(self.next_context + 1000), ThreadKind::Standalone);
            thread.add_ref();
            thread
        }))
    }
    fn retain(&mut self, object: Handle) {
        self.references.push((true, object));
    }
    fn release(&mut self, object: Handle) {
        self.references.push((false, object));
    }
    fn on_create(&mut self, thread: &mut Thread, owner: &Thread, context: Handle, id: i32) -> Result<bool> {
        if self.create_hook_writes {
            thread.on_create(self, owner, context, id)?;
        }
        Ok(self.reserve)
    }
    fn on_destroy(&mut self, thread: &mut Thread) -> Result<bool> {
        self.destroy_hooks.push(thread.context().unwrap());
        if !self.destroy_hook || self.deny_destroy == thread.context() {
            return Ok(false);
        }
        thread.on_destroy(self)
    }
}

#[test]
fn t10_process_tags_and_absolute_index_quirk() {
    let mut a = Api::default();
    assert_eq!(a.session().new_tag().unwrap(), 1);
    let mut b = Api::default();
    assert_eq!(b.session().new_tag().unwrap(), 2);
    a.stack.push(V::Number(7.0));
    assert_eq!(a.session().new_tag().unwrap(), 7);
    a.stack.clear();
    assert_eq!(a.session().new_tag().unwrap(), 4);
    let table = a.new_table();
    a.stack.push(table);
    assert_eq!(a.session().tag(-1).unwrap(), -1);
    a.session().set_tag(23).unwrap();
    assert_eq!(a.session().tag(-1).unwrap(), 23);
    assert_eq!(a.stack.len(), 1);
    a.stack.clear();
    assert_eq!(a.session().clone_tag(23).unwrap(), 5);
}

#[test]
fn tag_methods_previous_value_unmapped_error_and_string_only_copy() {
    let mut a = Api::default();
    a.stack.push(V::Function(Callback::Alert));
    a.session().set_tag_method(30, b"index").unwrap();
    assert!(a.stack.is_empty());
    a.session().get_tag_method(30, b"gettable").unwrap();
    assert_eq!(a.stack.pop(), Some(V::Function(Callback::Alert)));
    assert_eq!(a.session().copy_tag_methods(31, 30).unwrap(), 31);
    a.session().get_tag_method(31, b"index").unwrap();
    assert_eq!(a.stack.pop(), Some(V::Function(Callback::Alert)));
    let table = a.new_table();
    a.stack.push(table);
    a.session().set_tag(31).unwrap();
    assert_eq!(a.session().tag(-1).unwrap(), 31); // numeric marker was not copied
    a.stack.clear();
    a.stack.push(V::Function(Callback::Alert));
    a.session().set_tag_method(30, b"eq").unwrap();
    assert_eq!(a.reports.last().unwrap().2, 1);
    assert!(a.stack.is_empty());
    a.session().get_tag_method(30, b"eq").unwrap();
    assert_eq!(a.stack, [V::Nil]);
}

#[test]
fn t11_getn_non_table_success_missing_and_call_failure() {
    let mut a = Api::default();
    a.stack.push(V::Number(9.0));
    assert_eq!(a.session().get_n(-1).unwrap(), 0);
    a.stack.clear();
    let t = a.new_table();
    a.stack.push(t);
    assert_eq!(a.session().get_n(-1).unwrap(), -1);
    a.global(b"getn", V::Chunk(0, vec![V::Number(3.9)]));
    assert_eq!(a.session().get_n(-1).unwrap(), 3);
    a.global(b"getn", V::Chunk(1, vec![V::String(b"failed".to_vec())]));
    assert_eq!(a.session().get_n(1).unwrap(), 1);
    assert_eq!(a.stack.len(), 1);
}

#[test]
fn t13_traceback_handler_report_and_pop_preserve_prefix() {
    let mut a = Api::default();
    a.stack.extend([
        V::Number(42.0),
        V::Chunk(1, vec![V::String(b"with traceback".to_vec())]),
        V::Number(9.0),
    ]);
    assert_eq!(a.session().call_protected(1, 0, 0, true).unwrap(), 1);
    assert_eq!(a.stack, [V::Number(42.0)]);
    assert_eq!(
        a.reports,
        [(Handle(99), Handle(10), 1, Some(b"with traceback".to_vec()))]
    );
    assert!(a.operations().contains(&Op::PCall(1, 0, 2)));
    assert!(a.operations().contains(&Op::Remove(2)));
    assert_eq!(a.precision, 24);
    assert!(a.trace.iter().all(|(_, p)| *p == 53));
}

#[test]
fn protected_explicit_handler_disabled_reporter_and_retained_error() {
    let mut a = Api::default();
    a.stack.push(V::Chunk(3, vec![V::Number(13.0)]));
    assert_eq!(
        thread()
            .session(&mut a, Reporter::Disabled)
            .unwrap()
            .call_protected(0, 0, 5, false)
            .unwrap(),
        3
    );
    assert_eq!(a.stack, [V::Number(13.0)]);
    assert!(a.reports.is_empty());
    assert!(!a.operations().contains(&Op::RawGet(GLOBALS)));
    a.stack.clear();
    a.stack.push(V::Chunk(0, vec![]));
    assert_eq!(a.session().call_protected(0, 0, 0, true).unwrap(), 0);
}

#[test]
fn do_functions_no_traceback_alert_status_one_and_stderr_fallback() {
    let mut a = Api::default();
    a.load.push_back((3, vec![V::String(b"syntax".to_vec())]));
    assert_eq!(a.session().do_string(b"bad\0ignored").unwrap(), 3);
    assert_eq!(a.reports[0].2, 1);
    assert!(a.stack.is_empty());
    assert!(a.operations().contains(&Op::LoadBuffer {
        bytes: b"bad".to_vec(),
        name: b"bad".to_vec()
    }));
    assert!(!a.operations().contains(&Op::RawGet(GLOBALS)));
    a.global(b"_ALERT", V::Nil);
    a.load.push_back((2, vec![V::String(b"missing".to_vec())]));
    assert_eq!(a.session().do_file(b"missing.lua").unwrap(), 2);
    assert_eq!(a.stderr, [Some(b"missing".to_vec())]);
    assert!(a.stack.is_empty());
}

#[test]
fn t18_foreign_double_result_and_precision_restore_on_success_and_failure() {
    let mut a = Api::default();
    // The foreign interpreter supplies the arithmetic result. This checks host
    // precision at load/pcall/tonumber boundaries, not Lua arithmetic itself.
    a.load.push_back((0, vec![V::Number(1.0 / 3.0)]));
    assert_eq!(a.session().do_string(b"return 1/3").unwrap(), 0);
    assert_eq!(
        a.session().execute(Op::ToNumber(-1)).unwrap(),
        Output::Number(1.0 / 3.0)
    );
    assert!(a.operations().contains(&Op::PCall(0, MULTRET, 0)));
    a.fail = Some(Op::GetTop);
    assert!(a.session().get_top().is_err());
    assert_eq!(a.precision, 24);
    assert!(a.trace.iter().all(|(_, p)| *p == 53));
    a.fail = None;
    a.session().unref(23).unwrap();
    assert_eq!(a.trace.last(), Some(&(Op::Unref(REGISTRY, 23), 24)));
    a.precision = 64;
    a.changes.clear();
    a.session().get_top().unwrap();
    assert!(a.changes.is_empty());
}

#[test]
fn base_open_flags_repeat_failures_close_and_default_reporter() {
    let mut a = Api::default();
    let mut host = LuaHost::new(Handle(10));
    assert_eq!(host.error_reporter(), Reporter::Default);
    assert!(host.open(&mut a, 0x7e).unwrap());
    let libraries: Vec<_> = a
        .operations()
        .into_iter()
        .filter_map(|o| if let Op::OpenLibrary(l) = o { Some(l) } else { None })
        .collect();
    assert_eq!(
        libraries,
        [
            Library::Base,
            Library::Io,
            Library::String,
            Library::Math,
            Library::Debug,
            Library::Table,
            Library::Coroutine
        ]
    );
    assert!(a.operations().contains(&Op::SetExtraOwner(Handle(10))));
    let before = a.trace.len();
    assert!(!host.open(&mut a, 0).unwrap());
    assert_eq!(a.trace.len(), before);
    host.close(&mut a).unwrap();
    assert_eq!(host.main_thread.context(), None);
    a.open = false;
    assert!(!host.open(&mut a, 0).unwrap());
    a.open = true;
    a.reserve = false;
    assert!(!host.open(&mut a, 0).unwrap());
    assert_eq!(host.main_thread.context(), Some(Handle(20))); // false hook mutations survive
    host.close(&mut a).unwrap();
    a.reserve = true;
    // A custom false hook inserted no main record, so Close has no OnDestroy
    // that would invalidate its mutated context. Start a fresh concrete owner.
    assert_eq!(host.main_thread.context(), Some(Handle(20)));
    host.destroy(&mut a).unwrap();
    let mut host = LuaHost::new(Handle(10));
    a.trace.clear();
    assert!(host.open(&mut a, 1).unwrap());
    assert_eq!(
        a.operations()
            .iter()
            .filter(|o| matches!(o, Op::OpenLibrary(_)))
            .count(),
        2
    );
    host.set_error_reporter(None, false);
    assert_eq!(host.error_reporter(), Reporter::Disabled);
    host.set_error_reporter(Some(Handle(80)), true);
    assert_eq!(host.error_reporter(), Reporter::Callback(Handle(80)));
    host.set_error_reporter(None, true);
    assert_eq!(host.error_reporter(), Reporter::Default);
}

#[test]
fn registry_conversion_and_ignored_arguments_and_invalid_paths() {
    let mut a = Api::default();
    for (number, signed) in [(4294967297.9, 1), (-2.9, -2), (f64::NAN, 0)] {
        a.stack = vec![V::Number(number)];
        assert_eq!(a.session().to_sint32(-1).unwrap(), signed);
        assert_eq!(a.session().to_uint32(-1).unwrap(), signed as u32);
    }
    a.stack = vec![V::Nil];
    assert_eq!(a.session().reference(999).unwrap(), -1);
    assert!(a.operations().contains(&Op::Ref(REGISTRY)));
    assert_eq!(Session::<Api>::get_upvalue_index(2), -10003);
    a.stack = vec![V::String(b"actual".to_vec())];
    assert!(a.session().error(b"ignored").is_err());
    assert_eq!(a.operations().last(), Some(&Op::Raise));
    a.stack = vec![V::Nil];
    assert_eq!(a.session().to_user_tag(1), Err(Error::InvalidUserdata));
    assert_eq!(a.precision, 24);
}

#[test]
fn identities_and_explicit_open_lifecycle_boundaries() {
    let host = LuaHost::new(Handle(1));
    let child = Thread::new(Handle(2), ThreadKind::Standalone);
    assert!(child.supports(1));
    assert!(child.supports(THREAD_IID));
    assert!(!child.supports(LUA_IID));
    assert!(!child.supports(0xea10c4a2));
    assert!(host.main_thread.supports(LUA_IID));
    assert_eq!(child.get_id(), -1);
    assert_eq!(child.references(), 0);
    assert_eq!(child.owner(), None);
    assert_eq!(child.user_context(), None);
    assert!(CommandGenerator::supports(GENERATOR_IID));
    assert!(CommandGenerator::supports(COMMAND_GENERATOR_IID));
}

#[test]
fn generator_validation_retention_and_shutdown_order() {
    let mut a = Api::default();
    let mut g = CommandGenerator::new(Handle(40));
    assert!(!g.init(&mut a, None, None, b"").unwrap());
    assert!(!g.init(&mut a, None, Some(Handle(1)), b"x").unwrap());
    assert!(a.generator_trace.is_empty());
    a.register = false;
    assert!(!g.init(&mut a, Some(Handle(2)), Some(Handle(1)), b"x").unwrap());
    a.register = true;
    a.generator_trace.clear();
    a.global(b"x", V::Number(1.0));
    assert!(!g.init(&mut a, Some(Handle(2)), Some(Handle(1)), b"x").unwrap());
    assert_eq!(a.generator_trace, ["register", "main borrowed", "unregister"]);
    a.global(b"x", V::Nil);
    a.generator_trace.clear();
    assert_eq!(g.init(&mut a, Some(Handle(2)), Some(Handle(1)), b"x"), Ok(true));
    assert_eq!(g.retained_thread(), Some(Handle(10)));
    assert_eq!(g.retained_server(), Some(Handle(2)));
    assert_eq!(g.name(), b"x");
    assert_eq!(g.as_command_generator(), Handle(40));
    assert_eq!(
        a.generator_trace,
        [
            "register",
            "main borrowed",
            "acquire MainThread",
            "acquire CommandServer",
            "create ParameterSet",
            "create ParameterSet"
        ]
    );
    a.generator_trace.clear();
    a.unregister = false;
    assert!(!g.shutdown(&mut a));
    assert_eq!(
        a.generator_trace,
        [
            "unregister",
            "release CommandServer",
            "release MainThread",
            "release ParameterSet",
            "release ParameterSet"
        ]
    );
    assert_eq!(g.retained_server(), None);
    assert_eq!(g.retained_thread(), None);
    assert!(g.shutdown(&mut a));
}

#[test]
fn boxed_pointer_tags_and_registry_reference_lifetime() {
    let mut a = Api::default();
    a.session().push_user_tag(0x12345678, 45).unwrap();
    assert_eq!(a.session().to_user_tag(-1).unwrap(), Some(Handle(0x12345678)));
    assert_eq!(a.session().tag(-1).unwrap(), 45);
    assert!(a.session().is_userdata(-1).unwrap());
    let reference = a.session().reference(700).unwrap();
    assert!(a.stack.is_empty());
    a.session().get_ref(reference).unwrap();
    assert_eq!(a.session().to_user_tag(-1).unwrap(), Some(Handle(0x12345678)));
    a.session().pop(1).unwrap();
    a.session().unref(reference).unwrap();
    a.session().get_ref(reference).unwrap();
    assert_eq!(a.stack, [V::Nil]);
}

#[test]
fn compatibility_failure_is_ignored_and_alert_only_reports_strings() {
    let mut a = Api::default();
    a.load.push_back((3, vec![V::String(b"compat failed".to_vec())]));
    let mut host = LuaHost::new(Handle(10));
    assert!(host.open(&mut a, 64).unwrap());
    assert!(!a.operations().contains(&Op::PushCClosure(Callback::NewTag, 0)));
    a.stack.push(V::Number(7.0));
    let count = a.reports.len();
    assert_eq!(a.session().dispatch_callback(Callback::Alert).unwrap(), 0);
    assert_eq!(a.reports.len(), count);
    for (status, kind) in [
        (1, "Run-time"),
        (2, "File-related"),
        (3, "Syntax"),
        (4, "Allocation"),
        (5, "Unknown"),
    ] {
        a.stack = vec![V::Chunk(status, vec![V::String(b"message".to_vec())])];
        thread()
            .session(&mut a, Reporter::Default)
            .unwrap()
            .call_protected(0, 0, 0, true)
            .unwrap();
        let text = String::from_utf8(a.logs.last().unwrap().clone()).unwrap();
        assert!(text.contains("GZLua5 script error"));
        assert!(text.contains(kind));
        assert!(text.contains("thread 7"));
        assert!(text.contains("message"));
    }
}

#[test]
fn runtime_dobuffer_error_gc_and_load_without_execution() {
    let mut a = Api {
        runtime_status: 1,
        ..Api::default()
    };
    a.load.push_back((0, vec![V::String(b"runtime".to_vec())]));
    assert_eq!(a.session().do_buffer(b"a\0b", b"chunk\0tail").unwrap(), 1);
    assert!(a.stack.is_empty());
    assert_eq!(a.reports.last().unwrap().2, 1);
    assert!(a.operations().contains(&Op::LoadBuffer {
        bytes: b"a\0b".to_vec(),
        name: b"chunk".to_vec()
    }));
    a.trace.clear();
    assert_eq!(a.session().load_string(b"source\0tail").unwrap(), 0);
    assert!(!a.operations().iter().any(|op| matches!(op, Op::PCall(..))));
    let mut host = LuaHost::new(Handle(10));
    host.main_thread = Thread::from_foreign_view(Handle(10), ThreadKind::Main, Handle(20), 7, Some(Handle(10)));
    assert_eq!(host.get_gc_count(&mut a).unwrap(), 25);
    assert_eq!(host.get_gc_threshold(&mut a).unwrap(), 100);
    host.set_gc_threshold(&mut a, 200).unwrap();
    assert_eq!(host.get_gc_threshold(&mut a).unwrap(), 200);
    assert_eq!(a.precision, 24);
}
