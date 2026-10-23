// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded portable owner of `effects-parser.md` §§2–7.
//!
//! This is a primitive-input model, not a lexer, native binding or full parser.
//! Every finder/converter is foreign. Unsupported exits retain prefix writes and
//! calls; `Ok(())` denotes a completed void routine, never a manufactured AL.
//! Allocation failure, error transport, reentry, rotations and destruction are
//! unsupported (EFP1–6 remain open). Rust drop frees memory only: it does not
//! implement the unknown logical reference cleanup at refcount zero.

use crate::effects::ComponentRecord;

#[cfg(test)]
mod tests;

/// Logical identities, deliberately not native pointers or pointer adjustments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Index(pub u32);
/// Portable string identity: exact UTF-8 payload, with no case/grammar policy.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text(pub String);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub id: Index,
    pub text: Text,
}

/// Pre-store scalar reply. This is not a decimal parser or a stored f32 result.
/// Dyadic(c,e) means exactly c * 2^e. Other replies stop at the conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawFloat {
    Dyadic(i32, i16),
    Unsupported,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericPolicy {
    /// Explicitly qualified nearest-even stores, x87 precision at least 53 bits.
    /// No assertion that this is the game's current control word is implied.
    NearestAtLeast53,
    Unqualified,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unsupported {
    Rotation(&'static str),
    Numeric,
    MissingIndex,
    InvalidToken,
    LodInSelect,
    ProbabilityOutsideSelect,
    NamedValidation,
    NotInEffectsBlock,
    Allocation,
    ForeignOperation,
    PhysicalQueryAdjustment,
}

/// Reuses the existing component payload. `type_word` is the authoritative +0C
/// word; `component.kind` is its low-byte projection for the existing effects
/// representation, whose binary reader has a narrower type domain.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub component: ComponentRecord,
    pub type_word: u32,
}
impl Record {
    /// Supporting foreign shared-constructor facts (§5.3), not parser-exclusive
    /// work. Existing ComponentRecord::new uses shipped defaults/layout, so the
    /// transform and LOD are explicitly projected here without changing it.
    pub fn shared_constructor_projection() -> Self {
        let mut component = ComponentRecord::new(0, u32::MAX);
        component.placement.fill(0);
        for at in [4, 20, 36, 52] {
            component.placement[at..at + 4].copy_from_slice(&1f32.to_bits().to_le_bytes());
        }
        component.lod_min = 0;
        component.lod_max = 0;
        Self { component, type_word: 0 }
    }
    fn word(&mut self, at: usize, bits: u32) {
        self.component.placement[at..at + 4].copy_from_slice(&bits.to_le_bytes());
    }
    fn increment(&mut self) {
        self.component.placement[0] = self.component.placement[0].wrapping_add(1);
    }
    fn initial_bit(&mut self, bit: bool) {
        self.component.flags = (self.component.flags & !1) | u32::from(bit);
    }
    fn set_type(&mut self, word: u32) {
        self.type_word = word;
        self.component.kind = word as u8;
    }
}

/// Each call is one raw primitive. AL is tested only for zero; scratch length
/// is independent of the requested count. No acceptance/consumption is inferred.
pub trait OptionPrimitives {
    fn find(&mut self, tokens: &[Token], key: &'static str, request: u8, scratch: &mut Vec<Index>) -> u8;
    fn float(&mut self, token: &Token) -> RawFloat;
    fn uint(&mut self, token: &Token) -> u32;
    fn vector(&mut self, token: &Token) -> [u32; 3];
}
fn token<'a>(tokens: &'a [Token], scratch: &[Index], n: usize) -> Result<&'a Token, Unsupported> {
    let index = scratch.get(n).ok_or(Unsupported::MissingIndex)?.0 as usize;
    tokens.get(index).ok_or(Unsupported::InvalidToken)
}
fn exact(raw: RawFloat, policy: NumericPolicy) -> Result<f64, Unsupported> {
    if policy != NumericPolicy::NearestAtLeast53 { return Err(Unsupported::Numeric); }
    match raw {
        // Every i32 is exact in binary64. These powers of two and products are
        // exact normal binary64 values. The final binary32 cast is nearest-even.
        // This deliberately bounds the domain; it is not general x87 emulation.
        RawFloat::Dyadic(c, e) if (-149..=104).contains(&e) => {
            let x = f64::from(c) * 2f64.powi(i32::from(e));
            if (x as f32).is_finite() { Ok(x) } else { Err(Unsupported::Numeric) }
        }
        _ => Err(Unsupported::Numeric),
    }
}
fn scalar(raw: RawFloat, policy: NumericPolicy) -> Result<f32, Unsupported> {
    Ok(exact(raw, policy)? as f32)
}
fn probability(raw: RawFloat, policy: NumericPolicy) -> Result<u16, Unsupported> {
    let x = exact(raw, policy)?;
    // Exact c*65535, then addition to 12582912, require at most 49 significant
    // bits for 0<=x<=1 with a denominator <=2^24. Thus >=53-bit x87 arithmetic
    // has no intermediate rounding here. Broader PC/RC/input ranges stay out.
    match raw {
        RawFloat::Dyadic(_, e) if e >= -24 && (0.0..=1.0).contains(&x) => {
            let product = x * 65535.0;
            let sum = product + 12582912.0;
            let stored = sum as f32;
            Ok(stored.to_bits() as u16)
        }
        _ => Err(Unsupported::Numeric),
    }
}

/// Finite owner lookup order; no preflight, rollback, token scan or AL result.
pub fn parse_options<P: OptionPrimitives>(
    tokens: &[Token], scratch: &mut Vec<Index>, select: u32, record: &mut Record,
    numeric: NumericPolicy, host: &mut P,
) -> Result<(), Unsupported> {
    if host.find(tokens, "offset", 1, scratch) != 0 {
        let words = host.vector(token(tokens, scratch, 0)?);
        for (at, bits) in [40, 44, 48].into_iter().zip(words) { record.word(at, bits); }
        record.component.placement[1] |= 1;
        record.increment();
    }
    for (key, count) in [("rotateX",1),("rotateY",1),("rotateZ",1),("rotateXYZ",3),("rotateZXY",3)] {
        if host.find(tokens, key, count, scratch) != 0 { return Err(Unsupported::Rotation(key)); }
    }
    if host.find(tokens, "scale", 1, scratch) != 0 {
        let original = exact(host.float(token(tokens, scratch, 0)?), numeric)?;
        record.word(52, (original as f32).to_bits()); // store BEFORE original comparison
        if original == 1.0 { record.component.placement[1] &= !2; }
        else { record.component.placement[1] |= 2; }
        record.increment();
    }
    if host.find(tokens, "lodRange", 2, scratch) != 0 {
        if select != 0 { return Err(Unsupported::LodInSelect); }
        let first = host.uint(token(tokens, scratch, 0)?);
        record.component.lod_min = first as u8;
        let second = host.uint(token(tokens, scratch, 1)?);
        record.component.lod_max = (second as u8).wrapping_add(1);
    } else if host.find(tokens, "lod", 1, scratch) != 0 {
        if select != 0 { return Err(Unsupported::LodInSelect); }
        let value = host.uint(token(tokens, scratch, 0)?) as u8;
        record.component.lod_min = value;
        record.component.lod_max = value.wrapping_add(1);
    } else {
        record.component.lod_min = 1;
        record.component.lod_max = 6;
    }
    for (key, at) in [("emitScale",0),("sizeScale",2)] {
        if host.find(tokens, key, 1, scratch) != 0 {
            let first = scalar(host.float(token(tokens, scratch, 0)?), numeric)?;
            record.component.render[at] = first;
            // The actual scratch count, not the request, selects conversion two.
            record.component.render[at + 1] = if scratch.len() < 2 { first }
                else { scalar(host.float(token(tokens, scratch, 1)?), numeric)? };
        }
    }
    if host.find(tokens, "prob", 1, scratch) != 0 {
        if select == 0 { return Err(Unsupported::ProbabilityOutsideSelect); }
        record.component.select_weight = probability(host.float(token(tokens, scratch, 0)?), numeric)?;
    }
    if host.find(tokens, "ignoreLength", 0, scratch) != 0 { record.component.flags |= 1; }
    if host.find(tokens, "respectLength", 0, scratch) != 0 { record.component.flags &= !1; }
    Ok(())
}

/// Returning normal foreign calls only. String allocation/copy/cleanup and raw
/// insertion remain foreign; no host method chooses an owner branch or returns
/// a completed component. `construct` reports only the raw successful call.
pub trait AppendPrimitives: OptionPrimitives {
    fn construct(&mut self) -> Result<(), Unsupported>;
    fn validate(&mut self, manager: Identity, name: &Text, kind: u32, scratch: &mut Vec<Index>) -> u8;
    fn assign_name(&mut self, name: &Text);
    fn register_anonymous(&mut self, manager: Identity, object: Identity) -> u32;
    fn object_type(&mut self, object: Identity) -> u32;
    fn copy_record(&mut self, record: &Record) -> Result<(), Unsupported>;
    /// Raw (end,&D,tag,1,1) insertion boundary; native pointers are not modeled.
    fn insert_record(&mut self, record: &Record) -> Result<(), Unsupported>;
    fn cleanup_string(&mut self);
    fn find_non_switch(&mut self, tokens: &[Token], request: u8, scratch: &mut Vec<Index>) -> u8;
}

/// Vec capacity is a portable choice, not the native pointer layout/allocator.
/// `pending` exposes truthful partial temporary writes on unsupported paths;
/// resuming such a path is unsupported. No cleanup/unwind is inferred (EFP3).
#[derive(Debug)]
pub struct Parser {
    pub manager: Option<Identity>,
    pub interface: Option<Identity>,
    pub reset: u8,
    pub select: u32,
    pub current_block: Text,
    pub scratch: Vec<Index>,
    pub components: Vec<Record>,
    pub pending: Option<Record>,
}
impl Parser {
    pub fn with_capacity(capacity: usize) -> Self {
        Self { manager: None, interface: None, reset: 0, select: 0,
            current_block: Text::default(), scratch: Vec::new(), components: Vec::with_capacity(capacity), pending: None }
    }
    fn begin<P: AppendPrimitives>(&mut self, host: &mut P) -> Result<(), Unsupported> {
        if self.pending.is_some() { return Err(Unsupported::ForeignOperation); }
        host.construct()?;
        self.pending = Some(Record::shared_constructor_projection());
        Ok(())
    }
    fn finish<P: AppendPrimitives>(&mut self, tokens: &[Token], initial: bool, numeric: NumericPolicy, host: &mut P) -> Result<(), Unsupported> {
        let record = self.pending.as_mut().ok_or(Unsupported::ForeignOperation)?;
        record.component.select_group = self.select as u16;
        record.initial_bit(initial);
        parse_options(tokens, &mut self.scratch, self.select, record, numeric, host)?;
        if self.components.len() == self.components.capacity() {
            host.insert_record(record)?;
            self.components.try_reserve(1).map_err(|_| Unsupported::Allocation)?;
        } else { host.copy_record(record)?; }
        // Owned Vec insertion after the returning foreign copy boundary. Its
        // allocation/copy details are not claimed as native capacity mechanics.
        self.components.push(record.clone());
        host.cleanup_string(); // shared ctor's successful buffer is nonnull
        self.pending = None;
        Ok(())
    }
    pub fn add_named<P: AppendPrimitives>(&mut self, tokens: &[Token], initial: bool, numeric: NumericPolicy, host: &mut P) -> Result<(), Unsupported> {
        let Some(manager) = self.manager else { return Ok(()); };
        self.begin(host)?;
        let name = &token(tokens, &self.scratch, 0)?.text;
        if host.validate(manager, name, 6, &mut self.scratch) == 0 { return Err(Unsupported::NamedValidation); }
        let name = &token(tokens, &self.scratch, 0)?.text; // reread shared index
        host.assign_name(name);
        let record = self.pending.as_mut().ok_or(Unsupported::ForeignOperation)?;
        record.component.name.clone_from(&name.0);
        record.set_type(6);
        self.finish(tokens, initial, numeric, host)
    }
    pub fn add_anonymous<P: AppendPrimitives>(&mut self, tokens: &[Token], object: Identity, initial: bool, numeric: NumericPolicy, host: &mut P) -> Result<(), Unsupported> {
        let Some(manager) = self.manager else { return Ok(()); };
        self.begin(host)?;
        let record = self.pending.as_mut().ok_or(Unsupported::ForeignOperation)?;
        record.component.name.clear();
        record.component.index = host.register_anonymous(manager, object);
        record.set_type(host.object_type(object));
        self.finish(tokens, initial, numeric, host)
    }
    pub fn sequence_effect<P: AppendPrimitives>(&mut self, tokens: &[Token], numeric: NumericPolicy, host: &mut P) -> Result<(), Unsupported> {
        if self.current_block.0.is_empty() { return Err(Unsupported::NotInEffectsBlock); }
        let _ignored_al = host.find_non_switch(tokens, 1, &mut self.scratch);
        let _name = token(tokens, &self.scratch, 0)?; // command reads even with null manager
        self.add_named(tokens, false, numeric, host)
    }
    /// Adjusted-entry fields H/K are logical state, not main-object offsets.
    pub fn set_manager<R: References>(&mut self, manager: Option<Identity>, refs: &mut R) {
        if manager != self.manager {
            if let Some(m) = manager { refs.add_ref(m); }
            let old = self.manager;
            self.manager = manager;
            refs.store_h(manager);
            if let Some(old) = old { refs.release(old); }
        }
        if let Some(m) = manager {
            let mut temporary = None;
            let _ignored_bool = refs.query_manager(m, 0xC97CF5CE, &mut temporary);
            if let Some(t) = temporary { refs.add_ref(t); refs.release(t); }
            if temporary != self.interface {
                if let Some(t) = temporary { refs.add_ref(t); }
                let old = self.interface;
                self.interface = temporary;
                refs.store_k(temporary);
                if let Some(old) = old { refs.release(old); }
            }
            if let Some(t) = temporary { refs.release(t); }
        }
        self.reset = 0;
        refs.reset_byte();
    }
}

/// Foreign reference bodies; store notifications make assignment order visible.
/// Foreign QI supplies its output and independent boolean, never a final state.
pub trait References {
    fn add_ref(&mut self, id: Identity);
    fn release(&mut self, id: Identity);
    fn store_h(&mut self, id: Option<Identity>);
    fn store_k(&mut self, id: Option<Identity>);
    fn query_manager(&mut self, id: Identity, iid: u32, out: &mut Option<Identity>) -> bool;
    fn reset_byte(&mut self);
    fn store_query_output(&mut self, id: Identity);
    fn base_query(&mut self, iid: u32, out: &mut Option<Identity>) -> bool;
}
/// Raw table-adjusted logical identities supplied by the embedding boundary.
/// No arithmetic on physical addresses or invented native binding is performed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryIdentities {
    pub adjusted_interface: Identity,
    pub canonical_unknown: Identity,
}
pub fn query_interface<R: References>(iid: u32, out: Option<&mut Option<Identity>>, identities: Option<QueryIdentities>, refs: &mut R) -> Result<bool, Unsupported> {
    let Some(out) = out else { return Ok(false); };
    if iid != 0x6956BABC { return Ok(refs.base_query(iid, out)); }
    let q = identities.ok_or(Unsupported::PhysicalQueryAdjustment)?;
    *out = Some(q.adjusted_interface);
    refs.store_query_output(q.adjusted_interface);
    refs.add_ref(q.canonical_unknown);
    Ok(true)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hooks { Direct(u32), Block(u32, u32) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Registration { pub name: &'static str, pub descriptor: u32, pub hooks: Hooks }
use Hooks::{Block, Direct};
/// Metadata establishes only registration associations, not handler semantics.
pub const ROOT_COMMANDS: [Registration; 21] = [
    Registration { name:"renderprop",descriptor:0x00A9F85C,hooks:Direct(0x00596630) },
    Registration { name:"textureID",descriptor:0x00A9F86C,hooks:Direct(0x0059F890) },
    Registration { name:"modelID",descriptor:0x00A9F87C,hooks:Direct(0x0059FE50) },
    Registration { name:"soundID",descriptor:0x00A9F88C,hooks:Direct(0x0059FF40) },
    Registration { name:"brushID",descriptor:0x00A9F89C,hooks:Direct(0x005A0030) },
    Registration { name:"effectID",descriptor:0x00A9F8AC,hooks:Direct(0x0059A640) },
    Registration { name:"effectGroup",descriptor:0x00A9F8BC,hooks:Block(0x00596750,0x00648F10) },
    Registration { name:"instance",descriptor:0x00A9FA08,hooks:Direct(0x0059A730) },
    Registration { name:"testEffect",descriptor:0x00A9F8D8,hooks:Direct(0x0059D1D0) },
    Registration { name:"messageTrigger",descriptor:0x00A9F8E8,hooks:Direct(0x0059E820) },
    Registration { name:"camera",descriptor:0x00A9F8F8,hooks:Direct(0x0059A2A0) },
    Registration { name:"setPriority",descriptor:0x00A9F908,hooks:Direct(0x005A24C0) },
    Registration { name:"effectsResource",descriptor:0x00A9F918,hooks:Block(0x00596810,0x00596970) },
    Registration { name:"loadResource",descriptor:0x00A9F934,hooks:Direct(0x00596AB0) },
    Registration { name:"effect",descriptor:0x00A9F944,hooks:Block(0x005A0D40,0x0059EB50) },
    Registration { name:"particles",descriptor:0x00A9F960,hooks:Block(0x005A1A50,0x0059D9B0) },
    Registration { name:"dynamicParticle",descriptor:0x00A9F97C,hooks:Block(0x0059DB00,0x0059DCF0) },
    Registration { name:"decal",descriptor:0x00A9F998,hooks:Block(0x0059DD80,0x0059DF50) },
    Registration { name:"shake",descriptor:0x00A9F9B4,hooks:Block(0x0059DFE0,0x0059E260) },
    Registration { name:"light",descriptor:0x00A9F9D0,hooks:Block(0x0059E2F0,0x0059E4D0) },
    Registration { name:"sequence",descriptor:0x00A9F9EC,hooks:Block(0x0059E600,0x0059E780) },
];
pub const SEQUENCE_EFFECT: Registration = Registration { name:"sequenceEffect",descriptor:0x00A9FDFC,hooks:Direct(0x005A3890) };
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor(pub u32);
pub trait Registrar {
    fn base_register(&mut self);
    fn allocate(&mut self, bytes: u32) -> Option<Descriptor>;
    fn shared_block_construct(&mut self, descriptor: Descriptor);
    fn install_vptr(&mut self, descriptor: Descriptor, vptr: u32);
    fn register(&mut self, descriptor: Descriptor, registration: Registration) -> u8;
}
fn register_one<R: Registrar>(r: Registration, host: &mut R) -> Result<(), Unsupported> {
    let block = matches!(r.hooks, Block(..));
    let d = host.allocate(if block { 0x18 } else { 4 }).ok_or(Unsupported::Allocation)?;
    if block { host.shared_block_construct(d); }
    host.install_vptr(d, r.descriptor);
    let _ignored_result = host.register(d, r);
    Ok(())
}
pub fn register_commands<R: Registrar>(host: &mut R) -> Result<(), Unsupported> {
    host.base_register();
    for registration in ROOT_COMMANDS { register_one(registration, host)?; }
    Ok(())
}
/// Only this nested association is published, not the whole effect dictionary.
pub fn register_sequence_effect<R: Registrar>(host: &mut R) -> Result<(), Unsupported> {
    register_one(SEQUENCE_EFFECT, host)
}
