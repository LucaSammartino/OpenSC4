// SPDX-License-Identifier: GPL-3.0-or-later
//! First portable game-data registry, from game-data-registry.md §§1–7.
//!
//! Tokens simulate stable 32-bit allocation/pointer identities, not native addresses
//! or an ABI. The caller supplies successful allocation tokens; this module owns
//! records, both indexes, lookup decisions, traversals and effect ordering. No
//! primitive may return a completed tree, canonical entry, or removal result.
//!
//! Callbacks receive only `&Registry`: the published nonmutating callback domain.
//! GDR1 factory/native construction, GDR2 failed/invalid allocation inputs, GDR3
//! destructive reentry and GDR4 diagnostic formatting are unsupported. This is
//! neither a native layout nor a restart/invalid-memory compatibility promise.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

macro_rules! token {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(u32);
        impl $name {
            pub const fn new(word: u32) -> Option<Self> {
                if word == 0 {
                    None
                } else {
                    Some(Self(word))
                }
            }
            pub const fn word(self) -> u32 {
                self.0
            }
        }
    };
}
token!(EntryId);
token!(Object);
token!(Target);
token!(Callback);
token!(Message);
token!(ServiceIdentity);

/// Explicitly successful record allocation, not an emulated allocator/factory.
#[derive(Clone, Copy, Debug)]
pub struct Allocation(pub EntryId);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    InvalidRecord,
    ReusedAllocation,
    NullName,
    NullNodeKey,
    NullNodeParent,
    NodeParentIsLeaf,
    AnonymousKeyCollision,
    CallbackSignature,
    FailedAllocation,
    MutatingReentry,
    FactoryOrNativeLayout,
    DiagnosticOutput,
    AfterShutdown,
    Destroyed,
    CounterOverflow,
}
pub type Domain<T> = Result<T, Unsupported>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    Typed,
    Message,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reference {
    Object(Object),
    Target(TargetKind, Target),
    Message(Message),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Registry,
    SystemService,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryOutcome {
    pub succeeded: bool,
    pub output: Option<Object>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub code: u32,
    pub entry: Option<EntryId>,
}
pub const REGISTRY_IID: u32 = 0xE898_ED03;
pub const SYSTEM_SERVICE_IID: u32 = 0x287F_B697;
pub const SERVICE_ID: u32 = 0xEBE0_E860;
pub const SERVICE_PRIORITY: i32 = -1_000_000;
pub const MESSAGE_TYPE: u32 = 0x2BDF_E414;

/// Foreign primitives only. Output parameter contents and callback effects belong
/// to the publisher. Integer arguments are opaque words, never stored variants.
pub trait Primitives {
    fn add_ref(&mut self, reference: Reference);
    fn release(&mut self, reference: Reference);
    fn query_unknown(&mut self, object: Object) -> QueryOutcome;
    fn framework_register(&mut self, identity: ServiceIdentity);
    fn framework_unregister(&mut self, identity: ServiceIdentity);
    fn typed_event(&mut self, target: Target, event: Event, registry: &Registry) -> bool;
    fn allocate_message(&mut self) -> Domain<Message>;
    fn message_type(&mut self, message: Message, value: u32);
    fn message_registry(&mut self, message: Message, registry: ServiceIdentity);
    fn message_code(&mut self, message: Message, code: u32);
    fn message_entry(&mut self, message: Message, entry: EntryId);
    fn message_deliver(&mut self, target: Target, message: Message, registry: &Registry) -> bool;
    fn value_call(
        &mut self,
        callback: Callback,
        key: u32,
        input: u32,
        output: u32,
        registry: &Registry,
    ) -> bool;
    fn value_exchange(
        &mut self,
        callback: Callback,
        key: u32,
        variant: u32,
        set: bool,
        registry: &Registry,
    ) -> bool;
    /// Logical destructor/deleting-wrapper effect; no host allocation is inferred.
    fn service_destroyed(&mut self, identity: ServiceIdentity, free_allocation: bool);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeafKind {
    Exchange,
    Call,
}
#[derive(Debug)]
enum Kind {
    Node(Object),
    Leaf {
        key: u32,
        callback: Option<Callback>,
    },
}
#[derive(Debug)]
struct Entry {
    name: String,
    parent: Option<EntryId>,
    context: u32,
    flags: u32,
    kind: Kind,
    children: Vec<EntryId>,
}
/// A logical borrowed view, including allocated-but-unindexed records during
/// removal callbacks and the preserved root after shutdown. No AddRef occurs.
#[derive(Debug, PartialEq, Eq)]
pub struct EntryView<'a> {
    pub name: &'a str,
    pub parent: Option<EntryId>,
    pub context: u32,
    pub flags: u32,
    pub object: Option<Object>,
    pub raw_value: Option<u32>,
    pub children: &'a [EntryId],
}
#[derive(Debug)]
pub struct Registry {
    identity: ServiceIdentity,
    root: EntryId,
    records: BTreeMap<EntryId, Entry>,
    allocated: BTreeSet<EntryId>,
    nodes: BTreeMap<u32, EntryId>,
    leaves: BTreeMap<u32, EntryId>,
    typed: Vec<Target>,
    messages: Vec<Target>,
    count: u32,
    shutdown: bool,
    destroyed: bool,
}

/// Names are nullable solely to distinguish GDR2 from defined negative calls.
#[derive(Clone, Copy, Debug)]
pub struct Create<'a> {
    pub allocation: Allocation,
    pub name: Option<&'a str>,
    pub parent: Option<EntryId>,
    pub flags: u32,
    pub context: u32,
}

impl Registry {
    pub fn new(
        identity: ServiceIdentity,
        root: Allocation,
        root_key: Object,
        host: &mut impl Primitives,
    ) -> Self {
        host.add_ref(Reference::Object(root_key));
        let root = root.0;
        Self {
            identity,
            root,
            records: BTreeMap::from([(
                root,
                Entry {
                    name: "__root__".into(),
                    parent: None,
                    context: 0,
                    flags: 0,
                    kind: Kind::Node(root_key),
                    children: Vec::new(),
                },
            )]),
            allocated: BTreeSet::from([root]),
            nodes: BTreeMap::from([(root_key.word(), root)]),
            leaves: BTreeMap::new(),
            typed: Vec::new(),
            messages: Vec::new(),
            count: 0,
            shutdown: false,
            destroyed: false,
        }
    }
    pub fn identity(&self) -> ServiceIdentity {
        self.identity
    }
    fn alive(&self) -> Domain<()> {
        if self.destroyed {
            Err(Unsupported::Destroyed)
        } else {
            Ok(())
        }
    }
    fn active(&self) -> Domain<()> {
        self.alive()?;
        if self.shutdown {
            Err(Unsupported::AfterShutdown)
        } else {
            Ok(())
        }
    }
    fn record(&self, id: Option<EntryId>) -> Domain<Option<&Entry>> {
        self.alive()?;
        id.map(|id| self.records.get(&id).ok_or(Unsupported::InvalidRecord))
            .transpose()
    }
    pub fn root(&self) -> Domain<EntryId> {
        self.alive()?;
        Ok(self.root)
    }
    pub fn view(&self, id: Option<EntryId>) -> Domain<Option<EntryView<'_>>> {
        Ok(self.record(id)?.map(|entry| EntryView {
            name: &entry.name,
            parent: entry.parent,
            context: entry.context,
            flags: entry.flags,
            object: match entry.kind {
                Kind::Node(key) => Some(key),
                _ => None,
            },
            raw_value: match entry.kind {
                Kind::Leaf { key, .. } => Some(key),
                _ => None,
            },
            children: &entry.children,
        }))
    }
    pub fn node(&self, id: Option<EntryId>) -> Domain<Option<Object>> {
        Ok(self.view(id)?.and_then(|v| v.object))
    }
    pub fn value(&self, id: Option<EntryId>) -> Domain<Option<u32>> {
        Ok(self.view(id)?.and_then(|v| v.raw_value))
    }
    pub fn name(&self, id: Option<EntryId>) -> Domain<Option<&str>> {
        Ok(self.record(id)?.map(|e| e.name.as_str()))
    }
    pub fn parent(&self, id: Option<EntryId>) -> Domain<Option<EntryId>> {
        Ok(self.record(id)?.and_then(|e| e.parent))
    }
    pub fn context(&self, id: Option<EntryId>) -> Domain<Option<u32>> {
        Ok(self.record(id)?.map(|e| e.context))
    }
    pub fn is_node(&self, id: Option<EntryId>) -> Domain<bool> {
        Ok(self.node(id)?.is_some())
    }
    pub fn check_flags(&self, id: Option<EntryId>, mask: u32) -> Domain<bool> {
        Ok(self.record(id)?.is_some_and(|e| e.flags & mask == mask))
    }
    pub fn set_context(&mut self, id: Option<EntryId>, context: u32) -> Domain<bool> {
        self.alive()?;
        let Some(id) = id else {
            return Ok(false);
        };
        self.records
            .get_mut(&id)
            .ok_or(Unsupported::InvalidRecord)?
            .context = context;
        Ok(true)
    }
    /// Value scan, deliberately independent of record dereference (§3.2).
    pub fn is_valid(&self, id: Option<EntryId>) -> Domain<bool> {
        self.alive()?;
        Ok(id.is_some_and(|id| {
            self.nodes
                .values()
                .chain(self.leaves.values())
                .any(|&v| v == id)
        }))
    }
    pub fn lookup_raw(&self, key: u32) -> Domain<Option<EntryId>> {
        self.alive()?;
        Ok(self.leaves.get(&key).copied())
    }
    pub fn lookup_object(
        &self,
        key: Option<Object>,
        host: &mut impl Primitives,
    ) -> Domain<Option<EntryId>> {
        self.alive()?;
        let Some(key) = key else {
            return Ok(None);
        };
        if let Some(&id) = self.nodes.get(&key.word()) {
            return Ok(Some(id));
        }
        let qi = host.query_unknown(key);
        let found = if qi.succeeded {
            qi.output
                .and_then(|key| self.nodes.get(&key.word()).copied())
        } else {
            None
        };
        if let Some(key) = qi.output {
            host.release(Reference::Object(key));
        }
        Ok(found)
    }
    fn prepare(&self, create: Create<'_>) -> Domain<(EntryId, String)> {
        if self.allocated.contains(&create.allocation.0) {
            return Err(Unsupported::ReusedAllocation);
        }
        Ok((
            create.allocation.0,
            create.name.ok_or(Unsupported::NullName)?.to_owned(),
        ))
    }
    pub fn add_node(
        &mut self,
        key: Option<Object>,
        create: Create<'_>,
        host: &mut impl Primitives,
    ) -> Domain<EntryId> {
        self.active()?;
        if let Some(id) = self.lookup_object(key, host)? {
            return Ok(id);
        }
        let key = key.ok_or(Unsupported::NullNodeKey)?;
        let parent = create.parent.ok_or(Unsupported::NullNodeParent)?;
        if !self.is_node(Some(parent))? {
            return Err(Unsupported::NodeParentIsLeaf);
        }
        let (id, name) = self.prepare(create)?;
        host.add_ref(Reference::Object(key));
        self.records.insert(
            id,
            Entry {
                name,
                parent: Some(parent),
                context: create.context,
                flags: create.flags,
                kind: Kind::Node(key),
                children: Vec::new(),
            },
        );
        self.allocated.insert(id);
        self.nodes.insert(key.word(), id);
        self.records
            .get_mut(&parent)
            .expect("validated parent")
            .children
            .push(id);
        self.broadcast(
            Event {
                code: 3,
                entry: Some(id),
            },
            host,
        );
        Ok(id)
    }
    pub fn add_leaf(
        &mut self,
        key: u32,
        callback: Option<Callback>,
        kind: LeafKind,
        create: Create<'_>,
        host: &mut impl Primitives,
    ) -> Domain<Option<EntryId>> {
        self.active()?;
        if self.leaves.contains_key(&key) {
            return Ok(None);
        }
        if !self.is_node(create.parent)? {
            return Ok(None);
        }
        let (id, name) = self.prepare(create)?;
        let key = if key == 0 { id.word() } else { key };
        // GDR5: no invented overwrite/rebalance semantics for anonymous collisions.
        if self.leaves.contains_key(&key) {
            return Err(Unsupported::AnonymousKeyCollision);
        }
        let flags = match kind {
            LeafKind::Call => create.flags | 1,
            LeafKind::Exchange => create.flags & !1,
        };
        self.records.insert(
            id,
            Entry {
                name,
                parent: create.parent,
                context: create.context,
                flags,
                kind: Kind::Leaf { key, callback },
                children: Vec::new(),
            },
        );
        self.allocated.insert(id);
        self.leaves.insert(key, id);
        self.records
            .get_mut(&create.parent.expect("node parent"))
            .expect("validated parent")
            .children
            .push(id);
        self.broadcast(
            Event {
                code: 3,
                entry: Some(id),
            },
            host,
        );
        Ok(Some(id))
    }
    fn callback(&self, id: Option<EntryId>) -> Domain<Option<(Callback, u32)>> {
        match self.record(id)?.map(|e| &e.kind) {
            None | Some(Kind::Leaf { callback: None, .. }) => Ok(None),
            Some(Kind::Leaf {
                key,
                callback: Some(callback),
            }) => Ok(Some((*callback, *key))),
            Some(Kind::Node(_)) => Err(Unsupported::CallbackSignature),
        }
    }
    pub fn value_call(
        &self,
        id: Option<EntryId>,
        input: u32,
        output: u32,
        host: &mut impl Primitives,
    ) -> Domain<bool> {
        Ok(self
            .callback(id)?
            .is_some_and(|(cb, key)| host.value_call(cb, key, input, output, self)))
    }
    pub fn value_exchange(
        &self,
        id: Option<EntryId>,
        variant: u32,
        set: bool,
        host: &mut impl Primitives,
    ) -> Domain<bool> {
        if set && self.check_flags(id, 2)? {
            return Ok(false);
        }
        Ok(self
            .callback(id)?
            .is_some_and(|(cb, key)| host.value_exchange(cb, key, variant, set, self)))
    }
    pub fn visit_immediate(
        &self,
        start: Option<EntryId>,
        context: u32,
        mut visit: impl FnMut(EntryId, u32, &Registry) -> bool,
    ) -> Domain<bool> {
        let Some(entry) = self.record(start)? else {
            return Ok(true);
        };
        for &child in &entry.children {
            if !visit(child, context, self) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub fn visit_breadth_first(
        &self,
        start: Option<EntryId>,
        context: u32,
        mut visit: impl FnMut(EntryId, u32, &Registry) -> bool,
    ) -> Domain<bool> {
        self.record(start)?;
        let mut queue: VecDeque<_> = start.into_iter().collect();
        while let Some(id) = queue.pop_front() {
            if !visit(id, context, self) {
                return Ok(false);
            }
            queue.extend(self.records[&id].children.iter().copied());
        }
        Ok(true)
    }
    fn replay(&self, target: Target, kind: TargetKind, host: &mut impl Primitives) -> Domain<()> {
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            match kind {
                TargetKind::Typed => {
                    host.typed_event(
                        target,
                        Event {
                            code: 3,
                            entry: Some(id),
                        },
                        self,
                    );
                }
                TargetKind::Message => {
                    let message = host.allocate_message()?;
                    host.message_type(message, MESSAGE_TYPE);
                    host.message_registry(message, self.identity);
                    host.message_code(message, 3);
                    host.message_entry(message, id);
                    host.message_deliver(target, message, self);
                    host.release(Reference::Message(message));
                }
            }
            stack.extend(self.records[&id].children.iter().copied());
        }
        Ok(())
    }
    fn targets(&mut self, kind: TargetKind) -> &mut Vec<Target> {
        match kind {
            TargetKind::Typed => &mut self.typed,
            TargetKind::Message => &mut self.messages,
        }
    }
    pub fn subscribe(
        &mut self,
        kind: TargetKind,
        target: Option<Target>,
        host: &mut impl Primitives,
    ) -> Domain<bool> {
        self.active()?;
        let target = target.ok_or(Unsupported::InvalidRecord)?;
        let reference = Reference::Target(kind, target);
        host.add_ref(reference);
        if self.targets(kind).contains(&target) {
            host.release(reference);
            return Ok(false);
        }
        host.add_ref(reference);
        self.targets(kind).push(target);
        host.release(reference);
        // No transactional recovery is promised for failed message allocations (GDR2).
        self.replay(target, kind, host)?;
        Ok(true)
    }
    pub fn unsubscribe(
        &mut self,
        kind: TargetKind,
        target: Option<Target>,
        host: &mut impl Primitives,
    ) -> Domain<bool> {
        self.alive()?;
        let Some(target) = target else {
            return Ok(false);
        };
        let list = self.targets(kind);
        let Some(index) = list.iter().position(|&value| value == target) else {
            return Ok(false);
        };
        list.remove(index);
        host.release(Reference::Target(kind, target));
        Ok(true)
    }
    fn broadcast(&self, event: Event, host: &mut impl Primitives) {
        for &target in &self.typed {
            host.typed_event(target, event, self);
        }
        // §5 ordinary message branch is skipped, including with zero typed targets.
    }
    fn postorder(&self, id: EntryId, out: &mut Vec<EntryId>) {
        for &child in &self.records[&id].children {
            self.postorder(child, out);
        }
        out.push(id);
    }
    fn destroy_tree(&mut self, id: EntryId, host: &mut impl Primitives) {
        let entry = self.records.remove(&id).expect("owned record");
        if let Kind::Node(key) = entry.kind {
            host.release(Reference::Object(key));
        }
        for child in entry.children {
            self.destroy_tree(child, host);
        }
    }
    fn remove_entry(&mut self, id: EntryId, host: &mut impl Primitives) -> bool {
        if id == self.root {
            return false;
        }
        let mut postorder = Vec::new();
        self.postorder(id, &mut postorder);
        for victim in postorder {
            self.broadcast(
                Event {
                    code: 2,
                    entry: Some(victim),
                },
                host,
            );
            match self.records[&victim].kind {
                Kind::Node(key) => {
                    self.nodes.remove(&key.word());
                }
                Kind::Leaf { key, .. } => {
                    self.leaves.remove(&key);
                }
            }
        }
        let parent = self.records[&id].parent.expect("nonroot parent");
        self.destroy_tree(id, host);
        let children = &mut self
            .records
            .get_mut(&parent)
            .expect("owned parent")
            .children;
        let position = children
            .iter()
            .position(|&child| child == id)
            .expect("linked entry");
        children.remove(position);
        true
    }
    pub fn remove_raw(&mut self, key: u32, host: &mut impl Primitives) -> Domain<bool> {
        self.active()?;
        Ok(match self.lookup_raw(key)? {
            Some(id) => self.remove_entry(id, host),
            None => false,
        })
    }
    pub fn remove_object(
        &mut self,
        key: Option<Object>,
        host: &mut impl Primitives,
    ) -> Domain<bool> {
        self.active()?;
        Ok(match self.lookup_object(key, host)? {
            Some(id) => self.remove_entry(id, host),
            None => false,
        })
    }
    pub fn init(&self, host: &mut impl Primitives) -> Domain<bool> {
        self.active()?;
        host.framework_register(self.identity);
        Ok(true)
    }
    fn clear_targets(&mut self, host: &mut impl Primitives) {
        for target in self.typed.drain(..) {
            host.release(Reference::Target(TargetKind::Typed, target));
        }
        for target in self.messages.drain(..) {
            host.release(Reference::Target(TargetKind::Message, target));
        }
    }
    pub fn shutdown(&mut self, host: &mut impl Primitives) -> Domain<bool> {
        self.active()?;
        host.framework_unregister(self.identity);
        self.broadcast(
            Event {
                code: 1,
                entry: None,
            },
            host,
        );
        self.nodes.clear();
        self.leaves.clear();
        let children =
            std::mem::take(&mut self.records.get_mut(&self.root).expect("root").children);
        for child in children {
            self.destroy_tree(child, host);
        }
        self.clear_targets(host);
        self.shutdown = true;
        Ok(true)
    }
    /// Explicit effectful destruction. Rust Drop cannot take reference callbacks;
    /// dropping this safe model alone makes no claims about native Release effects.
    pub fn destroy(&mut self, free_allocation: bool, host: &mut impl Primitives) -> Domain<()> {
        self.alive()?;
        self.destroy_tree(self.root, host);
        self.clear_targets(host);
        self.nodes.clear();
        self.leaves.clear();
        self.destroyed = true;
        host.service_destroyed(self.identity, free_allocation);
        Ok(())
    }
    /// Published deleting-wrapper flag, modeled as an effect rather than freeing
    /// a host allocation. Only the low bit controls that effect (§2).
    pub fn delete_wrapper(&mut self, flags: u32, host: &mut impl Primitives) -> Domain<()> {
        self.destroy(flags & 1 != 0, host)
    }
    pub fn query_interface(&mut self, iid: u32) -> Domain<Option<Face>> {
        self.alive()?;
        let face = match iid {
            REGISTRY_IID => Face::Registry,
            SYSTEM_SERVICE_IID => Face::SystemService,
            1 => Face::Unknown,
            _ => return Ok(None),
        };
        self.add_ref()?;
        Ok(Some(face))
    }
    pub fn ref_count(&self) -> Domain<u32> {
        self.alive()?;
        Ok(self.count)
    }
    pub fn add_ref(&mut self) -> Domain<u32> {
        self.alive()?;
        self.count = self
            .count
            .checked_add(1)
            .ok_or(Unsupported::CounterOverflow)?;
        Ok(self.count)
    }
    pub fn release(&mut self, host: &mut impl Primitives) -> Domain<u32> {
        self.alive()?;
        if self.count > 1 {
            self.count -= 1;
            Ok(self.count)
        } else {
            self.count = 0;
            self.destroy(true, host)?;
            Ok(0)
        }
    }
    pub fn debug_dump(&self, id: Option<EntryId>) -> Domain<()> {
        self.record(id)?;
        if id.is_none() {
            Ok(())
        } else {
            Err(Unsupported::DiagnosticOutput)
        }
    }
}

#[cfg(test)]
mod tests;
