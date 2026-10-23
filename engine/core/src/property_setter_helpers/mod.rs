// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable normal-call contracts from `docs/specs/property-setter-helpers.md` §§2–5.
//!
//! Addresses are opaque 32-bit identities, never native pointers. The caller supplies
//! valid independent string/buffer storage and normally returning, non-reentrant
//! primitives. Storage, lifetime, allocator and scalar x87 behavior remain foreign.
//! This module neither implements Variant storage nor changes live property owners.

/// Raw final typed-call register bits, with an explicit defined-bit mask.
/// For original bool/u8 primitives only AL is specified (`defined_mask = 0xff`).
/// This is not a success flag; EDX and a 64-bit return value are not specified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Residue {
    pub eax: u32,
    pub defined_mask: u32,
}

/// A portable boundary refusal, not a native failure/status contract.
/// Earlier foreign effects and helper stores are preserved; there is no rollback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Memory,
    Primitive,
    AddressOverflow,
    VectorDomain,
}

pub type Normal<T> = Result<T, Refusal>;

pub const POOL_DESCRIPTOR_POINTER: u32 = 0x00b0_a9ec;
pub const POOL_ALLOCATOR_RECEIVER: u32 = 0x00b0_a9e8;
pub const FIRST_TABLE: u32 = 0x00aa_6c54;
pub const PROPERTY_TABLE: u32 = 0x00aa_6988;
pub const VARIANT_TABLE: u32 = 0x00aa_6788;

/// Individual foreign primitives only. Slot numbers are supplied by the helper,
/// including lookup 9, install 15 and GetVariant 5. Implementations must not
/// synthesize acquisition or a whole-helper result. Copy callbacks own copying;
/// the helper never iterates the input. No refcount primitive is needed.
pub trait Primitives {
    fn lookup(&mut self, holder: u32, slot: u32, id: u32) -> Normal<u32>;
    fn read_word(&mut self, address: u32) -> Normal<u32>;
    fn write_word(&mut self, address: u32, value: u32) -> Normal<()>;
    fn allocate_block(&mut self, receiver: u32) -> Normal<u32>;
    fn construct_variant(&mut self, receiver: u32) -> Normal<()>;
    fn install(&mut self, holder: u32, slot: u32, property: u32, zero: u32) -> Normal<Residue>;
    fn get_variant(&mut self, property: u32, slot: u32) -> Normal<u32>;
    fn set_bool(&mut self, variant: u32, slot: u32, word: u32) -> Normal<Residue>;
    fn set_u8(&mut self, variant: u32, slot: u32, word: u32) -> Normal<Residue>;
    fn set_s32(&mut self, variant: u32, slot: u32, word: u32) -> Normal<Residue>;
    fn set_u32(&mut self, variant: u32, slot: u32, word: u32) -> Normal<Residue>;
    fn set_s64(&mut self, variant: u32, slot: u32, low: u32, high: u32) -> Normal<Residue>;
    fn set_f32(&mut self, variant: u32, slot: u32, word: u32) -> Normal<Residue>;
    fn set_string(&mut self, variant: u32, slot: u32, string: u32) -> Normal<Residue>;
    fn copy_u8(&mut self, variant: u32, slot: u32, begin: u32, count: u32) -> Normal<Residue>;
    fn copy_u32(&mut self, variant: u32, slot: u32, begin: u32, count: u32) -> Normal<Residue>;
    fn copy_f32(&mut self, variant: u32, slot: u32, begin: u32, count: u32) -> Normal<Residue>;
    fn copy_s64(&mut self, variant: u32, slot: u32, begin: u32, count: u32) -> Normal<Residue>;
}

fn offset(base: u32, bytes: u32) -> Normal<u32> {
    base.checked_add(bytes).ok_or(Refusal::AddressOverflow)
}

fn acquire(p: &mut impl Primitives, holder: u32, id: u32) -> Normal<u32> {
    let mut property = p.lookup(holder, 9, id)?;
    if property == 0 {
        let descriptor = p.read_word(POOL_DESCRIPTOR_POINTER)?;
        let head_address = offset(descriptor, 8)?;
        property = p.read_word(head_address)?;
        if property == 0 {
            property = p.allocate_block(POOL_ALLOCATOR_RECEIVER)?;
        }
        let link = p.read_word(property)?;
        p.write_word(head_address, link)?;
        let count_address = offset(descriptor, 12)?;
        // Read after the allocator: its raw mutations are authoritative.
        let count = p.read_word(count_address)?;
        p.write_word(count_address, count.wrapping_add(1))?;
        p.write_word(property, FIRST_TABLE)?;
        let embedded = offset(property, 4)?;
        p.construct_variant(embedded)?;
        p.write_word(property, PROPERTY_TABLE)?;
        p.write_word(embedded, VARIANT_TABLE)?;
        p.write_word(offset(property, 0x18)?, id)?;
        let _ignored = p.install(holder, 15, property, 0)?;
    }
    // No adjustment: an existing/inherited property may return a different identity.
    p.get_variant(property, 5)
}

macro_rules! word_helper {
    ($name:ident, $primitive:ident, $slot:literal, $doc:literal) => {
        #[doc = $doc]
        pub fn $name(p: &mut impl Primitives, holder: u32, id: u32, word: u32) -> Normal<Residue> {
            let variant = acquire(p, holder, id)?;
            p.$primitive(variant, $slot, word)
        }
    };
}

word_helper!(
    set_bool,
    set_bool,
    11,
    "Forward the whole bool stack word; no normalization."
);
word_helper!(set_u8, set_u8, 14, "Forward the whole u8 stack word; no narrowing.");
word_helper!(set_s32, set_s32, 29, "Forward the signed 32-bit raw word.");
word_helper!(set_u32, set_u32, 26, "Forward the unsigned 32-bit raw word.");
word_helper!(
    set_f32,
    set_f32,
    38,
    "Forward raw scalar f32 bits; no helper floating arithmetic."
);
word_helper!(
    set_string,
    set_string,
    56,
    "Forward a valid independent string interface unchanged."
);

/// Forward the original low/high pair; the residue is not a 64-bit return.
pub fn set_s64(p: &mut impl Primitives, holder: u32, id: u32, low: u32, high: u32) -> Normal<Residue> {
    let variant = acquire(p, holder, id)?;
    p.set_s64(variant, 35, low, high)
}

fn vector_input(p: &mut impl Primitives, descriptor: u32, shift: u32) -> Normal<(u32, u32)> {
    let begin = p.read_word(descriptor)?;
    let end = p.read_word(offset(descriptor, 4)?)?;
    // Portable domain refusal only: these checks are not native helper branches.
    // Readability/independence/aliasing are caller preconditions, not host oracles.
    let bytes = end.checked_sub(begin).ok_or(Refusal::VectorDomain)?;
    let alignment = (1_u32 << shift) - 1;
    if bytes > i32::MAX as u32 || (begin | bytes) & alignment != 0 || (begin == 0 && bytes != 0) {
        return Err(Refusal::VectorDomain);
    }
    let difference = end.wrapping_sub(begin);
    Ok((begin, ((difference as i32) >> shift) as u32))
}

macro_rules! vector_helper {
    ($name:ident, $primitive:ident, $slot:literal, $shift:literal) => {
        /// Acquire the Variant, read begin then end (never capacity), and call the
        /// copying primitive once with the original begin, including empty input.
        pub fn $name(p: &mut impl Primitives, holder: u32, id: u32, descriptor: u32) -> Normal<Residue> {
            let variant = acquire(p, holder, id)?;
            let (begin, count) = vector_input(p, descriptor, $shift)?;
            p.$primitive(variant, $slot, begin, count)
        }
    };
}

vector_helper!(set_u8_vector, copy_u8, 94, 0);
vector_helper!(set_u32_vector, copy_u32, 102, 2);
vector_helper!(set_f32_vector, copy_f32, 110, 2);
vector_helper!(set_s64_vector, copy_s64, 108, 3);

#[cfg(test)]
mod tests;
