// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable app-directors.md owners. Tokens are opaque identities, not addresses.
//! Child receivers, COM, factories, framework and resource manager remain foreign.
//! Native layout and factory-owned service algorithms are outside this module.

mod registrations;
pub use registrations::{GAME_CLASSES, RESMAN_CLASSES, RESOURCE_CLASSES, TOOLS_CLASSES};

use std::collections::BTreeMap;

pub type Token = u32;
pub const DIRECTOR_IID: u32 = 0xA21E_E941;
pub const HOOKS_IID: u32 = 0x03FA_40BF;
pub const SERVICE_IID: u32 = 0x287F_B697;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Tools,
    ResourceLib,
    Game,
    ResMan,
}

impl Owner {
    pub const fn id(self) -> u32 {
        match self {
            Self::Tools => 0x1CEC_20BE,
            Self::ResourceLib => 0x84FE_3294,
            Self::Game => 0x63CC_0AF8,
            Self::ResMan => 0xC3CA_EC3B,
        }
    }

    pub fn registrations(self) -> &'static [(u32, u32)] {
        match self {
            Self::Tools => TOOLS_CLASSES,
            Self::ResourceLib => RESOURCE_CLASSES,
            Self::Game => GAME_CLASSES,
            Self::ResMan => RESMAN_CLASSES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Primary,
    Hooks,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    pub object: Token,
    pub face: Face,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Registration {
    Factory(Token),
    Callback(Token),
    /// A malformed/unknown kind is an ordinary false dispatch, per §2.3.
    Unknown(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    NullFactoryObject,
    RequiredCom,
    RequiredFramework,
    InvalidHooksSlot,
    StaticStorageFree,
    /// ADIR2: only Tools' exception cleanup is specified.
    ExceptionalSingletonConstruction,
}

pub type Result<T> = std::result::Result<T, Boundary>;

/// Only foreign receiver calls. Returned COM/QI/lookup outputs carry acquired
/// references. Child, heap, COM, singleton and framework identities are borrowed.
/// `out` is passed through unchanged until the foreign callee writes it.
pub trait Host {
    fn heap(&mut self, com: Token) -> Option<Token>;
    fn child_initialize(&mut self, child: Token, com: Option<Token>, path: &[u8]) -> bool;
    fn child_children(&mut self, child: Token) -> Vec<Token>;
    fn child_class(&mut self, child: Token, class: u32, iid: u32, out: &mut Option<Token>) -> bool;
    fn child_enum(&mut self, child: Token, callback: &mut dyn FnMut(u32, u32, Token) -> bool, context: Token);
    fn child_unload(&mut self, child: Token) -> bool;
    fn factory(&mut self, factory: Token) -> Option<Token>;
    fn query(&mut self, object: Token, iid: u32, out: &mut Option<Token>) -> bool;
    fn add_ref(&mut self, object: Token);
    fn release(&mut self, object: Token);
    fn callback(&mut self, callback: Token, iid: u32, out: &mut Option<Token>) -> bool;
    fn create(&mut self, com: Token, class: u32, iid: u32, out: &mut Option<Token>) -> bool;
    fn framework(&mut self) -> Option<Token>;
    fn register_service(&mut self, framework: Token, service: Token) -> bool;
    fn register_update(&mut self, framework: Token, service: Token) -> bool;
    fn register_hooks(&mut self, framework: Token, hooks: Identity) -> bool;
    fn manager(&mut self, framework: Token, service: u32, iid: u32, out: &mut Option<Token>) -> bool;
    fn register_factory(&mut self, manager: Token, class: u32, resource_type: u32, factory: Option<Token>) -> bool;
    fn global_com(&mut self) -> Option<Token>;
    fn subordinate(&mut self, which: Subordinate) -> Token;
    fn child_start(&mut self, child: Token, com: Option<Token>) -> bool;
    fn free_allocation(&mut self, allocation: Token);
    fn register_exit(&mut self, owner: Owner) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subordinate {
    ScriptedWindow,
    GDrivers,
    Fonts,
    Drawing,
    Graphics,
    ResMan,
    WindowsResource,
    Window,
    Image,
    ResourceLib,
    Script,
    UiScript,
    Automata,
    Lua,
}

pub const SUBORDINATES: [(Subordinate, bool); 14] = [
    (Subordinate::ScriptedWindow, true),
    (Subordinate::GDrivers, true),
    (Subordinate::Fonts, false),
    (Subordinate::Drawing, true),
    (Subordinate::Graphics, false),
    (Subordinate::ResMan, false),
    (Subordinate::WindowsResource, true),
    (Subordinate::Window, false),
    (Subordinate::Image, true),
    (Subordinate::ResourceLib, true),
    (Subordinate::Script, true),
    (Subordinate::UiScript, true),
    (Subordinate::Automata, true),
    (Subordinate::Lua, true),
];

#[derive(Debug)]
pub struct Director {
    owner: Owner,
    identity: Token,
    static_storage: bool,
    count: u32,
    com: Option<Token>,
    heap: Option<Token>,
    path: Vec<u8>,
    children: Vec<Token>,
    classes: BTreeMap<u32, Registration>,
}

impl Director {
    /// Fresh base state followed by the concrete owner's class insertions.
    /// For Game, use `construct_game` to perform its global-root wiring too.
    pub fn new(owner: Owner, identity: Token, static_storage: bool) -> Self {
        let mut result = Self {
            owner,
            identity,
            static_storage,
            count: 0,
            com: None,
            heap: None,
            path: Vec::new(),
            children: Vec::new(),
            classes: BTreeMap::new(),
        };
        for &(class, factory) in owner.registrations() {
            result.add_class(class, Registration::Factory(factory));
        }
        result
    }

    /// `root` is the result of the global root lookup, never the game director.
    /// COM is captured once only when that lookup succeeded (§7.1).
    pub fn construct_game(
        identity: Token,
        static_storage: bool,
        root: Option<&mut Director>,
        host: &mut impl Host,
    ) -> Self {
        if let Some(root) = root {
            let com = host.global_com();
            for (which, start) in SUBORDINATES {
                let child = host.subordinate(which);
                root.add_director(child, host);
                if start {
                    host.child_start(child, com);
                }
            }
        }
        Self::new(Owner::Game, identity, static_storage)
    }

    pub fn owner(&self) -> Owner {
        self.owner
    }
    pub fn identity(&self, face: Face) -> Identity {
        Identity {
            object: self.identity,
            face,
        }
    }
    pub fn count(&self) -> u32 {
        self.count
    }
    pub fn com(&self) -> Option<Token> {
        self.com
    }
    pub fn heap(&self) -> Option<Token> {
        self.heap
    }
    pub fn path(&self) -> &[u8] {
        &self.path
    }
    pub fn children(&self) -> &[Token] {
        &self.children
    }
    pub fn heap_allocated_size(&self) -> u32 {
        0
    }
    pub fn primary_success_hook(&self) -> bool {
        true
    }
    pub fn library_path(&self, out: &mut Vec<u8>) -> bool {
        out.clone_from(&self.path);
        true
    }
    pub fn query_interface(&mut self, iid: u32, out: &mut Option<Identity>) -> bool {
        let face = match iid {
            1 | DIRECTOR_IID => Face::Primary,
            HOOKS_IID => Face::Hooks,
            _ => return false,
        };
        *out = Some(self.identity(face));
        self.add_ref();
        true
    }
    pub fn add_ref(&mut self) -> u32 {
        self.count = self.count.wrapping_add(1);
        self.count
    }
    pub fn remove_ref(&mut self) -> u32 {
        self.count = self.count.saturating_sub(1);
        self.count
    }
    pub fn release(&mut self) -> u32 {
        self.remove_ref()
    }
    pub fn initialize(&mut self, com: Option<Token>, path: &[u8], host: &mut impl Host) -> bool {
        let Some(com) = com else {
            return false;
        };
        self.com = Some(com);
        self.heap = host.heap(com);
        self.path = path.to_vec();
        for &child in &self.children {
            host.child_initialize(child, Some(com), path);
        }
        true
    }
    /// Valid domain: nonnull children and an acyclic borrowed graph. Duplicates
    /// deliberately survive; child failures do not cancel any insertion.
    pub fn add_director(&mut self, child: Token, host: &mut impl Host) {
        host.child_initialize(child, self.com, &self.path);
        for descendant in host.child_children(child) {
            self.add_director(descendant, host);
        }
        self.children.push(child);
    }
    pub fn add_class(&mut self, class: u32, registration: Registration) -> bool {
        if let std::collections::btree_map::Entry::Vacant(entry) = self.classes.entry(class) {
            entry.insert(registration);
            true
        } else {
            false
        }
    }
    pub fn get_class_object(
        &self,
        class: u32,
        iid: u32,
        out: &mut Option<Token>,
        host: &mut impl Host,
    ) -> Result<bool> {
        for &child in &self.children {
            if host.child_class(child, class, iid, out) {
                return Ok(true);
            }
        }
        match self.classes.get(&class) {
            Some(Registration::Factory(factory)) => {
                let object = host.factory(*factory).ok_or(Boundary::NullFactoryObject)?;
                if host.query(object, iid, out) {
                    return Ok(true);
                }
                host.add_ref(object);
                host.release(object);
                Ok(false)
            }
            Some(Registration::Callback(callback)) => Ok(host.callback(*callback, iid, out)),
            _ => Ok(false),
        }
    }
    /// Local order is intentionally a portable ascending-ID choice, NOT native
    /// hash order (open ADIR1). All set/phase/callback semantics are specified.
    pub fn enum_classes(
        &self,
        callback: &mut dyn FnMut(u32, u32, Token) -> bool,
        context: Token,
        host: &mut impl Host,
    ) {
        for &child in &self.children {
            host.child_enum(child, callback, context);
        }
        for &class in self.classes.keys() {
            callback(class, 0, context);
        }
    }
    pub fn can_unload(&self, host: &mut impl Host) -> bool {
        self.count != 0 || self.children.iter().all(|&child| host.child_unload(child))
    }
    pub fn start(&self, supplied_com: Option<Token>, host: &mut impl Host) -> Result<bool> {
        let (com, class) = match self.owner {
            Owner::Tools | Owner::Game => return Ok(true),
            Owner::ResourceLib => (self.com, 0x252E_A0DF),
            Owner::ResMan => (supplied_com, 0x656B_8F27),
        };
        let com = com.ok_or(Boundary::RequiredCom)?;
        let mut service = None;
        host.create(com, class, SERVICE_IID, &mut service);
        if let Some(service) = service {
            // A boundary fault is not a native false result. Balance our local
            // QI reference even if the foreign framework precondition failed.
            let Some(framework) = host.framework() else {
                host.release(service);
                return Err(Boundary::RequiredFramework);
            };
            host.register_service(framework, service);
            if self.owner == Owner::ResMan {
                host.register_update(framework, service);
            }
            host.release(service);
        }
        if self.owner == Owner::ResourceLib {
            let framework = host.framework().ok_or(Boundary::RequiredFramework)?;
            host.register_hooks(framework, self.identity(Face::Hooks));
        }
        Ok(true)
    }
    pub fn hook(&self, slot: u32, host: &mut impl Host) -> Result<bool> {
        if !(3..=10).contains(&slot) {
            return Err(Boundary::InvalidHooksSlot);
        }
        if self.owner != Owner::ResourceLib || slot != 4 {
            return Ok(true);
        }
        let Some(framework) = host.framework() else {
            return Ok(false);
        };
        let mut manager = None;
        host.manager(framework, 0x056B_906E, 0x656B_8EFC, &mut manager);
        let Some(manager) = manager else {
            return Ok(false);
        };
        host.register_factory(manager, 0x4534_29B3, 0x6534_284A, None);
        host.register_factory(manager, 0x0534_29C8, 0x0534_2861, None);
        host.release(manager);
        Ok(true)
    }
    /// Explicit native base destruction; does not stop/release borrowed objects.
    pub fn destroy(&mut self, flags: u32, host: &mut impl Host) -> Result<Token> {
        if self.static_storage && flags & 1 != 0 {
            return Err(Boundary::StaticStorageFree);
        }
        self.classes = BTreeMap::new();
        self.children = Vec::new();
        self.path = Vec::new();
        if flags & 1 != 0 {
            host.free_allocation(self.identity);
        }
        Ok(self.identity)
    }
}

/// Two-phase getter models the set-before-construction guard and recursive
/// borrowed access without inventing synchronization or exceptional rollback.
pub struct Singleton {
    owner: Owner,
    identity: Token,
    guard: bool,
    director: Option<Director>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Get {
    pub identity: Token,
    pub construct: bool,
}

impl Singleton {
    pub fn new(owner: Owner, identity: Token) -> Self {
        Self {
            owner,
            identity,
            guard: false,
            director: None,
        }
    }
    pub fn begin_get(&mut self) -> Get {
        let construct = !self.guard;
        self.guard = true;
        Get {
            identity: self.identity,
            construct,
        }
    }
    pub fn get(&mut self, global_root: Option<&mut Director>, host: &mut impl Host) -> Token {
        let get = self.begin_get();
        if get.construct {
            self.complete_get(global_root, host);
        }
        get.identity
    }
    /// Caller completes only the construction authorized by begin_get.
    pub fn complete_get(&mut self, global_root: Option<&mut Director>, host: &mut impl Host) {
        let director = if self.owner == Owner::Game {
            Director::construct_game(self.identity, true, global_root, host)
        } else {
            Director::new(self.owner, self.identity, true)
        };
        self.director = Some(director);
        host.register_exit(self.owner);
    }
    pub fn director(&self) -> Option<&Director> {
        self.director.as_ref()
    }
    /// Tools' constructor cleanup destroys the completed base; getter cleanup
    /// clears the guard. Other owners' unwind policy remains ADIR2.
    pub fn construction_failed(&mut self, completed_base: Option<&mut Director>, host: &mut impl Host) -> Result<()> {
        if self.owner != Owner::Tools {
            return Err(Boundary::ExceptionalSingletonConstruction);
        }
        if let Some(base) = completed_base {
            base.destroy(0, host)?;
        }
        self.director = None;
        self.guard = false;
        Ok(())
    }
    pub fn exit(&mut self, host: &mut impl Host) -> Result<()> {
        if let Some(director) = &mut self.director {
            director.destroy(0, host)?;
        }
        Ok(())
    }
}

/// Enabled framework hooks dispatcher rule used by AD-V17. Calls are supplied by the
/// foreign framework; false hook results do not cancel later callbacks.
pub fn dispatch_hooks(hooks: &mut [&mut dyn FnMut() -> bool]) -> bool {
    for hook in hooks {
        hook();
    }
    true
}

#[cfg(test)]
mod tests;
