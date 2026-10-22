// SPDX-License-Identifier: GPL-3.0-or-later
//! Typed foreign primitives, not ScriptServer result oracles. See lua-host.md §6.3.
use super::ScriptServer;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Handle(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecordToken(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interface {
    Lua,
    Generator,
    Thread,
    MessageService,
    Message,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StackOp {
    PushString(Vec<u8>),
    PushNumber(f64),
    PushBoolean(bool),
    PushLightUserdata(Handle),
    NewUserdata(usize),
    NewTable,
    PushValue(i32),
    Insert(i32),
    RawSet(i32),
    RawGet(i32),
    SetField { index: i32, name: &'static str },
    PushArgIndexCallback { upvalues: u32 },
    SetMetatable(i32),
    Pop(i32),
    SetTop(i32),
    PushNil,
}

/// Each return type is documented here and checked by the owner. Handles returned by
/// factories/CreateThread/FindMessageService/AllocateMessage are owned references;
/// MainThread, GeneratorTarget and BaseQuery follow their foreign interface contracts.
#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    BaseQuery(u32), // Interface(Option<Handle>); base service owns QI/AddRef
    BaseDestroy,
    FreeAllocation,
    CreateLua { class: u32, iid: u32 }, // Handle(Option)
    Open { lua: Handle, libraries: u32 }, // Bool
    Close(Handle),
    CreateGenerator { class: u32, iid: u32 }, // Handle(Option)
    InitGenerator { generator: Handle, lua: Handle, global: &'static str }, // Bool; command-system accessor is foreign
    GeneratorTarget(Handle), // Handle(Option), borrowed
    ShutdownGeneratorTarget(Handle),
    FrameworkAvailable, // Bool
    RegisterService,
    UnregisterService,
    AddRef { interface: Interface, handle: Handle },
    Release { interface: Interface, handle: Handle },
    MainThread(Handle), // Handle(Some), borrowed
    LoadBuffer { thread: Handle, source: Vec<u8>, length: usize, chunk: Vec<u8> }, // Int
    FileExists(Vec<u8>), // Bool
    PathName { path: Vec<u8>, flags: u32 }, // Bytes
    ReadFile { path: Vec<u8>, limit: u32 }, // File(Option<Vec<u8>>)
    CreateThread { lua: Handle, count: i32, iid: u32 }, // CreatedThread
    SetUserContext { thread: Handle, record: RecordToken },
    ThreadId(Handle), // Int
    RawContext(Handle), // Handle(Some)
    InstallLineHook { state: Handle, mask: u32, count: i32 },
    Status(Handle), // Int
    ResumeThread { lua: Handle, id: i32, count: i32, arguments: Option<Handle> }, // Bool (ignored)
    DestroyThread { lua: Handle, id: i32 },
    ParseCommandLine(Vec<u8>), // Tokens; parser, quoting and C-string interpretation are foreign
    Lowercase(Vec<u8>), // Bytes; cRZString's case-folding primitive
    DebugLocation { thread: Handle, depth: i32 }, // Location
    GetTop(Handle), // Int
    Stack { thread: Handle, op: StackOp },
    WrapperFromState(Handle), // Handle(Option), borrowed
    UserContext(Handle), // Record(Option)
    RawYieldField(Handle), // U16
    DebugLine { state: Handle, options: &'static str }, // Int; getinfo result is ignored by the adapter
    Yield { state: Handle, results: i32 },
    Type { thread: Handle, index: i32 }, // Int
    FindMessageService { service: u32, iid: u32 }, // Handle(Option), owned
    AllocateMessage { bytes: usize }, // Handle(Option), allocation failure violates precondition
    MessageType { message: Handle, value: u32 },
    MessageId { message: Handle, value: i32 },
    MessagePayload { message: Handle, value: i32 },
    SendMessage { service: Handle, message: Handle, flags: u32 }, // Bool (ignored)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Unit,
    Bool(bool),
    Int(i32),
    U16(u16),
    Handle(Option<Handle>),
    Interface(Option<Handle>),
    Bytes(Vec<u8>),
    File(Option<Vec<u8>>),
    Tokens(Vec<Vec<u8>>),
    CreatedThread { id: i32, thread: Option<Handle> },
    Location { name: Vec<u8>, line: i32 },
    Record(Option<RecordToken>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    Foreign(&'static str),
    UnexpectedReply,
    MissingLua,
    MissingThread,
    DuplicateLiveId(i32),
    InvalidatingCallback,
    StorageRequired,
    Destroyed,
    CounterExhausted,
}

/// Foreign calls may reenter through `owner`. The owner checks lifetime and borrowed
/// record identity on return. Invalidating callbacks terminate the operation with a
/// typed failure; they have no deferred-control or rollback guarantee (§6.3.4).
pub trait Foreign {
    fn invoke(&mut self, owner: &mut ScriptServer, call: Primitive) -> Result<Value, Failure>;
}
