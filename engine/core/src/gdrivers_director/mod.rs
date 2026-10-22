// SPDX-License-Identifier: GPL-3.0-or-later
//! Pure-core implementation of the bounded director inventory. Contracts come only from
//! renderer-gdrivers-director.md and renderer-gdrivers-director-inventory.md.
//! Addresses are 32-bit foreign identity tokens, never dereferenced Rust pointers.

pub mod aggregate;
pub mod capture;
pub mod products;
pub mod software;

pub const DIRECTOR_ID: u32 = 0x64b2_a903;
pub const OPENGL_CLASS: u32 = 0xc455_4841;
pub const SOFTWARE_CLASS: u32 = 0x7aca_35c6;
pub const FOREIGN_DX7_CLASS: u32 = 0xbadb_6906;

/// Safe representation of a path which the native code would dereference unchecked.
/// Errors preserve preceding writes/calls; they are not native Boolean failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    NullRead,
    NullWrite,
    NullFactory,
    Division,
    MissingSlot,
    UnspecifiedGrowth,
}
pub type Result<T> = std::result::Result<T, Fault>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Allocation {
    Heap,
    Virtual { flags: u32, protection: u32 },
}

/// Only primitive operations on foreign services, COM interfaces, GL/WGL and memory.
/// No entry supplies a director, capture, buffer, collection, or construction algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Framework,
    StringLength,
    StringData,
    StringAssign,
    ChildInitialize,
    ChildCount,
    ChildAt,
    ChildLookup,
    ChildEnumerate,
    ChildCanUnload,
    QueryInterface,
    InterfaceFactory,
    AddRef,
    Release,
    ForeignDx7Factory,
    ExitRegister,
    BranchQueryInterface,
    BranchAddRef,
    BranchRelease,
    DriverSlot80,
    BranchReset,
    BranchStop,
    ReferenceBaseCleanup,
    ExtensionBool,
    ExtensionSelector,
    ExtensionOne,
    ExtensionFive,
    ExtensionSeven,
    GlEnable,
    GlDisable,
    GlAmbient,
    GlLight,
    GlMaterial,
    GlViewport,
    GraphicsService,
    BitmapCreate,
    ComService,
    ComBitmapCreate,
    BitmapInitialized,
    BitmapFormat,
    BitmapWidth,
    BitmapHeight,
    BitmapInitialize,
    BitmapLock,
    BitmapUnlock,
    BitmapPixel,
    BitmapStorageRequest,
    BitmapStorageAddress,
    GlReadBuffer,
    GlPackAlignment,
    GlReadPixels,
    CurrentDc,
    CurrentContext,
    MakeCurrent,
    DeleteContext,
    ReleaseDc,
    WindowAssociate,
    ModeRecord,
    ModePrepare,
    ModeCreate,
    Synchronize,
    CopyEndpoint,
    CompletionEndpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Register(u32, u32, u8),
    AppendChild(u32),
    BaseDispatch,
    RegistryDestroyed,
    ChildrenDestroyed,
    PathDestroyed,
    ProductDispatch(u32),
    PoolSetDestroyed,
}

pub trait Host {
    fn alloc(&mut self, bytes: u32, kind: Allocation) -> u32;
    fn free(&mut self, address: u32, kind: Allocation);
    fn read(&mut self, address: u32, bytes: &mut [u8]) -> Result<()>;
    fn write(&mut self, address: u32, bytes: &[u8]) -> Result<()>;
    /// `data` is an immediate byte argument; output cells are shared with callers.
    fn call(&mut self, target: u32, op: Op, args: &[u32], data: &[u8], out: &mut [u32]) -> u32;
    fn observe(&mut self, _event: Event) {}
    /// Pass the very same callback into a foreign child's enumeration operation.
    fn enumerate_child(&mut self, target: u32, context: u32, callback: &mut dyn FnMut(u32, u32, u32) -> bool);
}

pub(crate) fn call(host: &mut impl Host, target: u32, op: Op, args: &[u32]) -> u32 {
    host.call(target, op, args, &[], &mut [])
}

pub(crate) fn zero(host: &mut impl Host, address: u32, bytes: u32) -> Result<()> {
    if address == 0 {
        return Err(Fault::NullWrite);
    }
    let chunk = [0; 4096];
    let mut offset = 0u32;
    while offset < bytes {
        let n = (bytes - offset).min(chunk.len() as u32);
        host.write(address.wrapping_add(offset), &chunk[..n as usize])?;
        offset += n;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Registration {
    pub class: u32,
    pub factory: u32,
    pub kind: u8,
}

/// Native bucket ordering, without prescribing its node layout. General growth beyond
/// the specified table prefix is deliberately outside this bounded implementation.
#[derive(Debug)]
pub struct Registry {
    buckets: Vec<Vec<Registration>>,
    len: u32,
}
impl Registry {
    pub fn new() -> Self {
        Self {
            buckets: vec![Vec::new(); 193],
            len: 0,
        }
    }
    pub fn insert(&mut self, entry: Registration) -> Result<bool> {
        if self.find(entry.class).is_some() {
            return Ok(false);
        }
        if self.len == self.buckets.len() as u32 {
            // General rehash chain order is not specified; see question GD1. Reject rather
            // than silently inventing observable callback ordering on that path.
            return Err(Fault::UnspecifiedGrowth);
        }
        let bucket = entry.class as usize % self.buckets.len();
        self.buckets[bucket].insert(0, entry);
        self.len += 1;
        Ok(true)
    }
    pub fn find(&self, class: u32) -> Option<Registration> {
        self.buckets[class as usize % self.buckets.len()]
            .iter()
            .find(|v| v.class == class)
            .copied()
    }
    pub fn entries(&self) -> impl Iterator<Item = Registration> + '_ {
        self.buckets.iter().flat_map(|b| b.iter().copied())
    }
    pub fn clear(&mut self) {
        self.buckets.clear();
        self.buckets.shrink_to_fit();
        self.len = 0;
    }
}
impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Director {
    pub primary: u32,
    pub secondary: u32,
    pub path_identity: u32,
    refs: u32,
    service: u32,
    framework: u32,
    path: Vec<u8>,
    children: Vec<u32>,
    registry: Registry,
}
impl Director {
    /// Parent §§2,6: construction never calls a factory or service.
    pub fn new(primary: u32, secondary: u32, path_identity: u32, host: &mut impl Host) -> Self {
        let mut this = Self {
            primary,
            secondary,
            path_identity,
            refs: 0,
            service: 0,
            framework: 0,
            path: Vec::new(),
            children: Vec::new(),
            registry: Registry::new(),
        };
        for (class, factory) in [
            (OPENGL_CLASS, 0x0087_e1d0),
            (SOFTWARE_CLASS, 0x008d_cd30),
            (FOREIGN_DX7_CLASS, 0x0088_6700),
        ] {
            // Three unique keys in 193 buckets cannot take a resize/error path.
            let _ = this.register(class, factory, 1, host);
        }
        this
    }
    pub fn register(&mut self, class: u32, factory: u32, kind: u8, host: &mut impl Host) -> Result<bool> {
        let inserted = self.registry.insert(Registration { class, factory, kind })?;
        host.observe(Event::Register(class, factory, kind));
        Ok(inserted)
    }
    pub const fn id(&self) -> u32 {
        DIRECTOR_ID
    }
    pub const fn refs(&self) -> u32 {
        self.refs
    }
    pub const fn service(&self) -> u32 {
        self.service
    }
    pub const fn framework(&self) -> u32 {
        self.framework
    }
    pub fn path(&self) -> &[u8] {
        &self.path
    }
    pub fn children(&self) -> &[u32] {
        &self.children
    }
    /// Fixture for the shared unsigned counter, not a new native setter.
    pub fn set_reference_fixture(&mut self, value: u32) {
        self.refs = value;
    }
    pub fn add_ref(&mut self) -> u32 {
        self.refs = self.refs.wrapping_add(1);
        self.refs
    }
    pub fn remove_ref(&mut self) -> u32 {
        self.refs = self.refs.saturating_sub(1);
        self.refs
    }
    pub fn release(&mut self) -> u32 {
        self.remove_ref()
    }
    pub fn query_interface(&mut self, iid: u32, output: Option<&mut u32>) -> Result<bool> {
        let identity = match iid {
            1 | 0xa21e_e941 => self.primary,
            0x03fa_40bf => self.secondary,
            _ => return Ok(false),
        };
        *output.ok_or(Fault::NullWrite)? = identity;
        self.add_ref();
        Ok(true)
    }
    pub const fn primary_noop_one(&self, _argument: u32) -> bool {
        true
    }
    pub const fn primary_noop(&self) -> bool {
        true
    }
    pub const fn secondary_callback(&self, slot: u8) -> Option<bool> {
        if slot >= 3 && slot <= 10 {
            Some(true)
        } else {
            None
        }
    }
    pub const fn heap_allocated_size(&self) -> u32 {
        0
    }
    pub fn initialize(&mut self, service: u32, source: u32, host: &mut impl Host) -> Result<bool> {
        if service == 0 {
            return Ok(false);
        }
        self.service = service;
        self.framework = call(host, service, Op::Framework, &[]);
        if source == 0 {
            return Err(Fault::NullRead);
        }
        let len = call(host, source, Op::StringLength, &[]);
        let data = call(host, source, Op::StringData, &[]);
        let mut bytes = vec![0; len as usize];
        host.read(data, &mut bytes)?;
        self.path = bytes;
        for child in &self.children {
            call(host, *child, Op::ChildInitialize, &[service, source]);
        }
        Ok(true)
    }
    pub fn library_path(&self, destination: u32, host: &mut impl Host) -> Result<bool> {
        if destination == 0 {
            return Err(Fault::NullRead);
        }
        host.call(
            destination,
            Op::StringAssign,
            &[self.path.len() as u32],
            &self.path,
            &mut [],
        );
        Ok(true)
    }
    /// Parent §3.1: flatten descendants before ancestor, but initialize ancestor first.
    pub fn add_director(&mut self, child: u32, host: &mut impl Host) {
        host.call(
            child,
            Op::ChildInitialize,
            &[self.service, self.path_identity],
            &self.path,
            &mut [],
        );
        let count = call(host, child, Op::ChildCount, &[]);
        for i in 0..count {
            let descendant = call(host, child, Op::ChildAt, &[i]);
            self.add_director(descendant, host);
        }
        self.children.push(child);
        host.observe(Event::AppendChild(child));
    }
    pub fn can_unload(&self, host: &mut impl Host) -> bool {
        if self.refs != 0 {
            return true;
        }
        self.children
            .iter()
            .all(|c| call(host, *c, Op::ChildCanUnload, &[]) as u8 != 0)
    }
    pub fn enumerate(&self, context: u32, host: &mut impl Host, callback: &mut impl FnMut(u32, u32, u32) -> bool) {
        for child in &self.children {
            host.enumerate_child(*child, context, callback);
        }
        for entry in self.registry.entries() {
            callback(entry.class, 0, context);
        }
    }
    /// Parent §4: factory construction belongs to the two products below; the third
    /// factory and product remain foreign. Output writes from failed children survive.
    pub fn get_class(
        &self,
        class: u32,
        iid: u32,
        output: &mut u32,
        host: &mut impl Host,
        products: &mut products::Products,
    ) -> Result<bool> {
        for child in &self.children {
            if host.call(
                *child,
                Op::ChildLookup,
                &[class, iid],
                &[],
                std::slice::from_mut(output),
            ) as u8
                != 0
            {
                return Ok(true);
            }
        }
        let Some(entry) = self.registry.find(class) else {
            return Ok(false);
        };
        if entry.kind == 2 {
            return Ok(host.call(
                entry.factory,
                Op::InterfaceFactory,
                &[iid],
                &[],
                std::slice::from_mut(output),
            ) as u8
                != 0);
        }
        if entry.kind != 1 {
            return Ok(false);
        }
        let object = products.factory(entry.factory, host)?;
        if object == 0 {
            return Err(Fault::NullFactory);
        }
        let success = host.call(object, Op::QueryInterface, &[iid], &[], std::slice::from_mut(output)) as u8 != 0;
        if !success {
            call(host, object, Op::AddRef, &[]);
            call(host, object, Op::Release, &[]);
        }
        Ok(success)
    }
    pub fn destroy_base(&mut self, host: &mut impl Host) {
        host.observe(Event::BaseDispatch);
        self.registry.clear();
        host.observe(Event::RegistryDestroyed);
        self.children.clear();
        self.children.shrink_to_fit();
        host.observe(Event::ChildrenDestroyed);
        self.path.clear();
        self.path.shrink_to_fit();
        host.observe(Event::PathDestroyed);
    }
    pub fn delete(&mut self, flags: u32, host: &mut impl Host) -> u32 {
        self.destroy_base(host);
        if flags & 1 != 0 {
            host.free(self.primary, Allocation::Heap);
        }
        self.primary
    }
}

#[derive(Default)]
pub struct Singleton {
    pub guard: u8,
    pub director: Option<Director>,
}
impl Singleton {
    pub fn access(&mut self, primary: u32, secondary: u32, path: u32, host: &mut impl Host) -> u32 {
        if self.guard & 1 == 0 {
            self.guard |= 1;
            self.director = Some(Director::new(primary, secondary, path, host));
            call(host, primary, Op::ExitRegister, &[]);
        }
        self.director.as_ref().map_or(primary, |d| d.primary)
    }
    pub fn exit(&mut self, host: &mut impl Host) {
        if let Some(director) = &mut self.director {
            director.destroy_base(host);
        }
    }
}
