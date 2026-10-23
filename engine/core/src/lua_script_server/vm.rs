// SPDX-License-Identifier: GPL-3.0-or-later
//! cGZLua5 owners. Behaviour source: lua-host.md §§1–6, 14, 16.
//! `LuaApi` contains interpreter/storage primitives, never host result oracles.
mod compat;
pub mod generator;
mod lifecycle;
mod thread;
use super::{Handle, GLOBALS, REGISTRY};
pub use lifecycle::{Record, ThreadApi};
pub use thread::Session;

pub const LUA_IID: u32 = 0x6b2811a6;
pub const THREAD_IID: u32 = 0xeb2810f1;
pub const THREAD_CLASS: u32 = 0x2b2810f7;
pub const GENERATOR_CLASS: u32 = 0x2bd13fb4;
pub const GENERATOR_IID: u32 = 0x2bd144df;
pub const COMMAND_GENERATOR_IID: u32 = 0x0b90251f;
pub const MULTRET: i32 = -1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Foreign(String),
    InvalidContext,
    InvalidUserdata,
    Unsupported(&'static str),
    BadPrimitiveResult,
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reporter {
    Default,
    Callback(Handle),
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Callback {
    Foreign(Handle),
    Alert,
    Tag,
    SetTag,
    GetTagMethod,
    SetTagMethod,
    NewTag,
    CopyTagMethods,
    Globals,
    CoroutineCreate,
    CoroutineResume,
    Command,
}

/// Operations are Lua 5.0 C API calls, except the explicitly named storage and
/// precision primitives. An error-raising Lua call must unwind to its active
/// protected call, or follow the foreign panic/exit path (§4.2); never return OK.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Open,
    Close,
    SetExtraOwner(Handle),
    ClearExtraOwner,
    GetExtraOwner,
    NewThread,
    Resume(i32),
    Yield(i32),
    HasActivationFrame,
    OpenLibrary(Library),
    GetTop,
    SetTop(i32),
    PushValue(i32),
    Remove(i32),
    Insert(i32),
    Replace(i32),
    Move(Handle, i32),
    CheckStack(i32),
    Type(i32),
    TypeName(i32),
    IsNumber(i32),
    IsString(i32),
    IsCFunction(i32),
    Equal(i32, i32),
    RawEqual(i32, i32),
    LessThan(i32, i32),
    ToNumber(i32),
    ToBoolean(i32),
    ToString(i32),
    Strlen(i32),
    ToCFunction(i32),
    ToUserdata(i32),
    ToThread(i32),
    ReadPointer32(Handle),
    WritePointer32(Handle, u32),
    PushNil,
    PushNumber(f64),
    PushString(Vec<u8>),
    PushLString(Vec<u8>),
    PushCClosure(Callback, i32),
    PushBoolean(bool),
    PushLightUserdata(Handle),
    NewUserdata(usize),
    NewTable,
    GetTable(i32),
    RawGet(i32),
    RawGetI(i32, i32),
    GetEnvironment(i32),
    GetMetatable(i32),
    SetTable(i32),
    RawSet(i32),
    RawSetI(i32, i32),
    SetEnvironment(i32),
    SetMetatable(i32),
    Call(i32, i32),
    PCall(i32, i32, i32),
    LoadFile(Vec<u8>),
    LoadBuffer {
        bytes: Vec<u8>,
        name: Vec<u8>,
    },
    Load {
        reader: Handle,
        data: Handle,
        name: Vec<u8>,
    },
    Raise,
    Next(i32),
    Concat(i32),
    Ref(i32),
    Unref(i32, i32),
    GetGCCount,
    GetGCThreshold,
    SetGCThreshold(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Library {
    Base,
    Io,
    String,
    Math,
    Debug,
    Table,
    Coroutine,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    None,
    Int(i32),
    Number(f64),
    Bool(bool),
    Bytes(Option<Vec<u8>>),
    Handle(Option<Handle>),
}
impl Output {
    pub fn int(self) -> Result<i32> {
        if let Self::Int(v) = self {
            Ok(v)
        } else {
            Err(Error::BadPrimitiveResult)
        }
    }
    pub fn boolean(self) -> Result<bool> {
        if let Self::Bool(v) = self {
            Ok(v)
        } else {
            Err(Error::BadPrimitiveResult)
        }
    }
    pub fn number(self) -> Result<f64> {
        if let Self::Number(v) = self {
            Ok(v)
        } else {
            Err(Error::BadPrimitiveResult)
        }
    }
    pub fn bytes(self) -> Result<Option<Vec<u8>>> {
        if let Self::Bytes(v) = self {
            Ok(v)
        } else {
            Err(Error::BadPrimitiveResult)
        }
    }
    pub fn handle(self) -> Result<Option<Handle>> {
        if let Self::Handle(v) = self {
            Ok(v)
        } else {
            Err(Error::BadPrimitiveResult)
        }
    }
}

pub trait LuaApi {
    fn precision(&mut self) -> u32;
    fn set_precision(&mut self, bits: u32);
    fn invoke(&mut self, state: Option<Handle>, operation: Op) -> Result<Output>;
    /// The standard Lua 5.0 etc/compat.lua payload; script execution remains foreign.
    fn compatibility_source(&mut self) -> Vec<u8>;
    fn report_callback(&mut self, callback: Handle, thread: Handle, status: i32, message: Option<&[u8]>);
    fn log(&mut self, message: &[u8]);
    fn stderr(&mut self, message: Option<&[u8]>);
}

pub(super) fn guarded<A: LuaApi + ?Sized, T>(api: &mut A, f: impl FnOnce(&mut A) -> Result<T>) -> Result<T> {
    let change = api.precision() == 24;
    if change {
        api.set_precision(53);
    }
    let result = f(api);
    if change {
        api.set_precision(24);
    }
    result
}

/// Pointer identity is supplied by the eventual adapter; core has no native layout.
#[derive(Clone, Debug)]
pub struct Thread {
    pub identity: Handle,
    pub kind: ThreadKind,
    state: std::sync::Arc<std::sync::Mutex<ThreadState>>,
}
#[derive(Default)]
struct ThreadState {
    context: Option<Handle>,
    user_context: Option<Handle>,
    id: Option<i32>,
    owner: Option<Thread>,
    references: u32,
}
impl std::fmt::Debug for ThreadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadState")
            .field("context", &self.context)
            .field("id", &self.id)
            .field("owner", &self.owner.as_ref().map(|t| t.identity))
            .field("references", &self.references)
            .finish()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadKind {
    Main,
    Standalone,
}
impl Thread {
    pub fn new(identity: Handle, kind: ThreadKind) -> Self {
        Self {
            identity,
            kind,
            state: std::sync::Arc::default(),
        }
    }
    pub fn supports(&self, iid: u32) -> bool {
        iid == 1 || iid == THREAD_IID || (self.kind == ThreadKind::Main && iid == LUA_IID)
    }
    /// Borrowed adapter view of an already-existing foreign thread. Cloning a
    /// view shares state but does not perform the native AddRef operation.
    pub fn from_foreign_view(
        identity: Handle,
        kind: ThreadKind,
        context: Handle,
        id: i32,
        owner: Option<Handle>,
    ) -> Self {
        let thread = Self::new(identity, kind);
        {
            let mut state = thread.state.lock().unwrap();
            state.context = Some(context);
            state.id = Some(id);
            state.owner = owner.map(|identity| Self::new(identity, ThreadKind::Main));
        }
        thread
    }
    pub fn session<'a, A: LuaApi>(&self, api: &'a mut A, reporter: Reporter) -> Result<Session<'a, A>> {
        Ok(Session {
            api,
            state: self.context().ok_or(Error::InvalidContext)?,
            identity: self.identity,
            thread_id: Some(self.get_id()),
            reporter: if self.owner().is_some() {
                reporter
            } else {
                Reporter::Disabled
            },
        })
    }
}

#[derive(Debug)]
pub struct LuaHost {
    pub main_thread: Thread,
    reporter: Reporter,
    records: std::collections::BTreeMap<i32, Record>,
    next_id: i32,
    registry_owner: Option<Thread>,
}
impl LuaHost {
    pub fn new(identity: Handle) -> Self {
        Self {
            main_thread: Thread::new(identity, ThreadKind::Main),
            reporter: Reporter::Default,
            records: std::collections::BTreeMap::new(),
            next_id: 0,
            registry_owner: None,
        }
    }
    pub fn set_error_reporter(&mut self, callback: Option<Handle>, use_default: bool) {
        self.reporter = match callback {
            Some(p) => Reporter::Callback(p),
            None if use_default => Reporter::Default,
            None => Reporter::Disabled,
        };
    }
    pub fn error_reporter(&self) -> Reporter {
        self.reporter
    }
    pub fn main<'a, A: LuaApi>(&self, api: &'a mut A) -> Result<Session<'a, A>> {
        self.main_thread.session(api, self.reporter)
    }
    pub fn open<A: ThreadApi>(&mut self, api: &mut A, flags: u32) -> Result<bool> {
        if self.main_thread.context().is_some() {
            return Ok(false);
        }
        guarded(api, |api| {
            let Some(state) = api.invoke(None, Op::Open)?.handle()? else {
                return Ok(false);
            };
            let owner = self.main_thread.identity;
            api.retain(owner);
            self.main_thread.add_ref();
            if let Some(old) = self.registry_owner.replace(self.main_thread.clone()) {
                api.release(old.identity);
                old.release(api);
            }
            let owner_view = self.main_thread.clone();
            if !api.on_create(&mut self.main_thread, &owner_view, state, self.next_id)? {
                return Ok(false);
            }
            api.invoke(Some(state), Op::PushLightUserdata(state))?;
            let registry = api.registry_identity(owner);
            api.invoke(Some(state), Op::PushLightUserdata(registry))?;
            api.invoke(Some(state), Op::RawSet(REGISTRY))?;
            api.invoke(Some(state), Op::SetExtraOwner(self.main_thread.identity))?;
            self.main_thread.add_ref();
            api.retain(self.main_thread.identity);
            self.records.insert(self.next_id, Record::main(state));
            self.next_id = self.next_id.wrapping_add(1);
            let mut s = self.main(api)?;
            s.op(Op::OpenLibrary(Library::Base))?;
            for (bit, library) in [
                (2, Library::Io),
                (4, Library::String),
                (8, Library::Math),
                (16, Library::Debug),
                (32, Library::Table),
            ] {
                if flags & bit != 0 {
                    s.op(Op::OpenLibrary(library))?;
                }
            }
            if flags & 64 != 0 {
                let _ = s.open_compatibility()?;
            }
            s.op(Op::OpenLibrary(Library::Coroutine))?;
            // The standard library supplies the other Lua 5.0 coroutine helpers;
            // host create/resume are routed to core, not the original host.
            s.get_global_inner(b"coroutine")?;
            for (name, callback) in [
                (b"create".as_slice(), Callback::CoroutineCreate),
                (b"resume", Callback::CoroutineResume),
            ] {
                s.op(Op::PushString(name.to_vec()))?;
                s.op(Op::PushCClosure(callback, 0))?;
                s.op(Op::RawSet(-3))?;
            }
            s.op(Op::SetTop(-2))?;
            s.op(Op::SetTop(0))?;
            Ok(true)
        })
    }
    pub fn close<A: ThreadApi>(&mut self, api: &mut A) -> Result<bool> {
        if let Some(state) = self.main_thread.context() {
            guarded(api, |api| {
                self.shutdown_registry(api)?;
                api.invoke(Some(state), Op::Close)?;
                Ok(())
            })?;
        }
        Ok(true)
    }
    pub fn get_gc_count<A: LuaApi>(&self, api: &mut A) -> Result<i32> {
        self.main(api)?.execute(Op::GetGCCount)?.int()
    }
    pub fn get_gc_threshold<A: LuaApi>(&self, api: &mut A) -> Result<i32> {
        self.main(api)?.execute(Op::GetGCThreshold)?.int()
    }
    pub fn set_gc_threshold<A: LuaApi>(&self, api: &mut A, value: i32) -> Result<()> {
        self.main(api)?.execute(Op::SetGCThreshold(value))?;
        Ok(())
    }
}
