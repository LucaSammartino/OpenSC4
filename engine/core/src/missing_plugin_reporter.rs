// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable OWN two-body algorithm from `docs/specs/missing-plugin-reporter.md`.
//! No construction, subscription, QI, native layout, or runtime binding is implied.
//! Host identities denote externally live views/references, not Rust ownership.
//! In particular the service cache is weak: store, temporary Release, reload.
//! MPR1–3 remain open. Unsupported stops preserve only the emitted prefix; they
//! are not native false returns, exception recovery, or cleanup guarantees.

pub const POST_CITY_INIT: u32 = 0xEA8A_E29A;
pub const PACK_PROPERTY: u32 = 0x6A87_1B82;

/// Opaque semantic identity. No native dereference/address arithmetic is done.
/// Existing owner-specific identities do not describe these foreign views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Id(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    Allocator,
    Allocation,
    InvalidView,
    SuccessWithoutOutput,
    CountOverflow,
    NullDialog,
    ForeignCall,
}
pub type Result<T> = std::result::Result<T, Unsupported>;

/// Only semantic collection effects; never native bucket geometry or ordering.
/// The observer cannot choose membership, insert results, or cleanup decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Dispatch(Id),
    SetZero,
    SetInit(u32),
    Capacity(u32),
    Insert(u32),
    SaveAl(u8),
    SetNodeFree(u32),
    SetClear,
    SetBucketsFree,
}

/// Borrowed membership only. No sorted/native iteration order is exported.
pub struct PackSet {
    values: Vec<u32>,
}
impl PackSet {
    pub fn contains(&self, value: u32) -> bool {
        self.values.contains(&value)
    }
    pub fn len(&self) -> u32 {
        self.values.len() as u32
    }
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    fn fresh(host: &mut impl PrimitiveHost) -> Result<Self> {
        host.observe(Effect::SetZero);
        host.observe(Effect::SetInit(100));
        let mut values = Vec::new();
        // [MPR4 portable choice] reserve is storage support only, not a claim
        // about native buckets, allocation counts, or container failure policy.
        values.try_reserve(100).map_err(|_| Unsupported::Allocation)?;
        Ok(Self { values })
    }
    fn insert(&mut self, host: &mut impl PrimitiveHost, value: u32) -> Result<()> {
        let requested = self.len().checked_add(1).ok_or(Unsupported::CountOverflow)?;
        host.observe(Effect::Capacity(requested));
        self.values.try_reserve(1).map_err(|_| Unsupported::Allocation)?;
        host.observe(Effect::Insert(value));
        if !self.contains(value) {
            self.values.push(value);
        }
        Ok(())
    }
    fn cleanup(self, host: &mut impl PrimitiveHost) {
        // Portable storage order only. Each unique node is destroyed once.
        for value in self.values {
            host.observe(Effect::SetNodeFree(value));
        }
        host.observe(Effect::SetClear);
        host.observe(Effect::SetBucketsFree);
    }
}

/// Single foreign primitives only. A normal host supplies finite circular lists,
/// valid borrowed views and externally live cached services across every call.
/// Output reference identities from query are consumed by exactly one Release.
/// No host primitive implements collection/filter/dialog gating or ref ledgers.
pub trait PrimitiveHost {
    fn observe(&mut self, effect: Effect);
    fn selector(&mut self, message: Id) -> Result<u32>;
    fn context(&mut self) -> Result<Option<Id>>;
    fn context_word(&mut self, context: Id, offset: u32) -> Result<u32>;
    fn filter_word(&mut self, filter: Id, offset: u32, value: u32) -> Result<()>;
    fn manager(&mut self) -> Result<Option<Id>>;
    fn cache_load(&mut self) -> Result<Option<Id>>;
    fn cache_store(&mut self, value: Option<Id>) -> Result<()>;
    fn framework(&mut self) -> Result<Option<Id>>;
    /// The output starts null. Return bits are ignored, even AL zero with live out.
    fn service_lookup(&mut self, framework: Id, service: u32, iid: u32, output: &mut Option<Id>) -> Result<u32>;
    fn allocate(&mut self, allocator: Id, bytes: u32) -> Result<Option<Id>>;
    fn reset_sentinel(&mut self, sentinel: Id) -> Result<()>;
    fn query(&mut self, manager: Id, head: &mut Id, min: Option<Id>, max: Option<Id>, filter: Id, max_count: u32) -> Result<u32>;
    fn next(&mut self, node: Id) -> Result<Id>;
    fn occupant(&mut self, node: Id) -> Result<Id>;
    fn holder(&mut self, occupant: Id) -> Result<Option<Id>>;
    /// Full pointer channel, never a low-byte boolean.
    fn property(&mut self, holder: Id, property: u32) -> Result<Option<Id>>;
    fn variant(&mut self, property: Id) -> Result<Option<Id>>;
    /// Borrowed variant and scalar output. On success all 32 output bits must
    /// be written; on AL zero the reporter does not inspect the output.
    fn convert(&mut self, variant: Id, output: &mut Option<u32>) -> Result<u32>;
    fn release(&mut self, reference: Id) -> Result<()>;
    fn permission(&mut self) -> Result<u32>;
    fn dialog_allocate(&mut self, bytes: u32) -> Result<Option<Id>>;
    fn dialog_initialize(&mut self, dialog: Id, vptr: u32, refs: u32) -> Result<()>;
    fn add_ref(&mut self, reference: Id) -> Result<u32>;
    fn do_dialog(&mut self, dialog: Id, packs: &PackSet) -> Result<u32>;
    fn free(&mut self, allocator: Id, node: Id) -> Result<()>;
}

/// Supplied complete-owner/filter views, not a native constructor or owned refs.
pub struct Reporter {
    pub owner: Id,
    pub filter: Id,
}
/// Typed complete-owner association replaces physical R+8/minus-eight layout.
/// Producing this token does not QI, AddRef, subscribe, or read native memory.
pub struct Listener<'a> {
    reporter: &'a Reporter,
}
impl Reporter {
    pub fn listener(&self) -> Listener<'_> {
        Listener { reporter: self }
    }

    pub fn post_init(&self, host: &mut impl PrimitiveHost) -> Result<u8> {
        let mut result = 1;
        let mut packs = PackSet::fresh(host)?;
        if let Some(context) = host.context()? {
            // Both source words precede either destination store (alias safe).
            let first = host.context_word(context, 12)?;
            let second = host.context_word(context, 16)?;
            host.filter_word(self.filter, 12, first)?;
            host.filter_word(self.filter, 16, second)?;
        }
        let manager = match host.manager()? {
            Some(manager) => manager,
            None => {
                packs.cleanup(host);
                return Ok(0);
            }
        };
        let service = allocator(host)?;
        let mut head = host.allocate(service, 12)?.ok_or(Unsupported::Allocation)?;
        host.reset_sentinel(head)?;
        let _ignored = host.query(manager, &mut head, None, None, self.filter, u32::MAX)?;
        let sentinel = head;
        let mut node = host.next(sentinel)?;
        // No sorting, type filter, installed-pack lookup or once-only latch.
        // Finite valid links are an external normal-domain precondition (MPR2).
        while node != sentinel {
            let occupant = host.occupant(node)?;
            let holder = host.holder(occupant)?.ok_or(Unsupported::InvalidView)?;
            if let Some(property) = host.property(holder, PACK_PROPERTY)? {
                let variant = host.variant(property)?.ok_or(Unsupported::InvalidView)?;
                // None represents unwritten bytes, not a native initialized slot.
                let mut output = None;
                let bits = host.convert(variant, &mut output)?;
                if bits as u8 != 0 {
                    packs.insert(host, output.ok_or(Unsupported::SuccessWithoutOutput)?)?;
                }
            }
            host.release(occupant)?;
            node = host.next(node)?;
        }
        if !packs.is_empty() && host.permission()? as u8 != 0 {
            let dialog = host.dialog_allocate(8)?;
            // [MPR4 portable choice] null rejoins a native foreign call, whose
            // return is UNKNOWN. Stop before that invalid receiver, not AL false.
            let dialog = dialog.ok_or(Unsupported::NullDialog)?;
            host.dialog_initialize(dialog, 0x00A8_ECE0, 0)?;
            let _ignored = host.add_ref(dialog)?;
            result = host.do_dialog(dialog, &packs)? as u8;
            host.observe(Effect::SaveAl(result));
            host.release(dialog)?;
        }
        let mut node = host.next(sentinel)?;
        while node != sentinel {
            let next = host.next(node)?;
            let service = allocator(host)?;
            host.free(service, node)?;
            node = next;
        }
        host.reset_sentinel(sentinel)?;
        let service = allocator(host)?;
        host.free(service, sentinel)?;
        packs.cleanup(host);
        Ok(result)
    }
}
impl Listener<'_> {
    pub fn do_message(&self, host: &mut impl PrimitiveHost, message: Id) -> Result<u8> {
        if host.selector(message)? == POST_CITY_INIT {
            host.observe(Effect::Dispatch(self.reporter.owner));
            let _post_init_al = self.reporter.post_init(host)?;
        }
        Ok(1)
    }
}

fn allocator(host: &mut impl PrimitiveHost) -> Result<Id> {
    let mut cache = host.cache_load()?;
    if cache.is_none() {
        let mut output = None;
        if let Some(framework) = host.framework()? {
            let _ignored = host.service_lookup(framework, 0x3AE4_BEA3, 0x3AE4_BEAB, &mut output)?;
        }
        host.cache_store(output)?;
        if let Some(temporary) = output {
            host.release(temporary)?;
        }
        cache = host.cache_load()?;
    }
    // External weak-cache liveness required; no invented retained AddRef/Arc.
    cache.ok_or(Unsupported::Allocator)
}

#[cfg(test)]
#[path = "missing_plugin_reporter/tests.rs"]
mod tests;
