// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned rendering-settings registry (`docs/specs/render-properties.md` §§1–3).
//!
//! Each construction copies the registered defaults into independent tables. These are
//! portable core records, not the game's 32-bit entry layout. Rust ownership frees all
//! three tables on drop. `RenderPropertiesOwner` models both interface views and lifetime;
//! it is not a native vtable or a promise of the game's 32-bit memory layout.
//! Getters borrow immutably and setters require exclusive access, so there is no interior
//! mutable state for a second thread to race. A glue owner can supply its own lock.

mod lifetime;
mod registry;

pub use lifetime::{Deletion, InterfaceView, RenderPropertiesOwner};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod contract_tests;

/// One property, including the two uninterpreted extra values of spec §1.2.
/// For booleans these represent the two initially zero bytes (RP2).
#[derive(Clone, Debug, PartialEq)]
pub struct Property<T> {
    index: i32,
    name: Vec<u8>,
    short_name: Vec<u8>,
    value: T,
    extra_values: [T; 2],
}

impl<T> Property<T> {
    pub fn index(&self) -> i32 {
        self.index
    }

    /// UTF-8 convenience view; foreign CRT profiles can produce non-UTF-8 names.
    pub fn name(&self) -> Option<&str> {
        std::str::from_utf8(&self.name).ok()
    }

    pub fn short_name(&self) -> Option<&str> {
        std::str::from_utf8(&self.short_name).ok()
    }

    pub fn name_bytes(&self) -> &[u8] {
        &self.name
    }

    pub fn short_name_bytes(&self) -> &[u8] {
        &self.short_name
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    /// Metadata only: setters never validate against these values (§2, RP1).
    pub fn extra_values(&self) -> &[T; 2] {
        &self.extra_values
    }
}

/// The complete public registry: 38 booleans, 24 integers and 15 floats.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderProperties {
    bools: Vec<Property<bool>>,
    ints: Vec<Property<i32>>,
    floats: Vec<Property<f32>>,
}

impl Default for RenderProperties {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderProperties {
    /// Spec §1.3: copy registrations, remove `k`, derive acronyms, lowercase names.
    pub fn new() -> Self {
        Self::with_ctype(&CLocale)
    }

    /// Construction-time CRT classification/folding (RP5/RP6). The caller supplies
    /// the active foreign CRT service, not original-owner lookup results. No locale
    /// is frozen here: each later `find_*_with_ctype` receives its call-time service.
    pub fn with_ctype(ctype: &dyn ByteCtype) -> Self {
        Self {
            bools: copy_registrations(&registry::BOOLS, ctype),
            ints: copy_registrations(&registry::INTS, ctype),
            floats: copy_registrations(&registry::FLOATS, ctype),
        }
    }

    /// Slots 3–5.
    pub fn num_bool_properties(&self) -> i32 {
        self.bools.len() as i32
    }

    pub fn num_int_properties(&self) -> i32 {
        self.ints.len() as i32
    }

    pub fn num_float_properties(&self) -> i32 {
        self.floats.len() as i32
    }

    /// Slots 6–8. `None` denotes an index outside the specified valid table.
    /// The original does not check indices; this safe core boundary does not emulate
    /// out-of-bounds memory reads or claim an original failure value (RP4).
    pub fn get_bool(&self, index: i32) -> Option<bool> {
        entry(&self.bools, index).map(|p| p.value)
    }

    pub fn get_int(&self, index: i32) -> Option<i32> {
        entry(&self.ints, index).map(|p| p.value)
    }

    pub fn get_float(&self, index: i32) -> Option<f32> {
        entry(&self.floats, index).map(|p| p.value)
    }

    /// Slots 9–11. Return whether the index was valid, without clamping the value.
    /// Invalid indices leave all state untouched at the safe boundary (RP4).
    pub fn set_bool(&mut self, index: i32, value: bool) -> bool {
        set(&mut self.bools, index, value)
    }

    pub fn set_int(&mut self, index: i32, value: i32) -> bool {
        set(&mut self.ints, index, value)
    }

    pub fn set_float(&mut self, index: i32, value: f32) -> bool {
        set(&mut self.floats, index, value)
    }

    /// Slots 12–14 under the explicit C-locale profile. A byte slice is a safe
    /// counted boundary: its end supplies the terminator if no NUL is present.
    /// Only the prefix before the first NUL is copied, folded and compared.
    /// These convenience methods do not follow the game's global CRT locale (RP6).
    pub fn find_bool(&self, name: impl AsRef<[u8]>) -> i32 {
        self.find_bool_with_ctype(name.as_ref(), &CLocale)
    }

    pub fn find_int(&self, name: impl AsRef<[u8]>) -> i32 {
        self.find_int_with_ctype(name.as_ref(), &CLocale)
    }

    pub fn find_float(&self, name: impl AsRef<[u8]>) -> i32 {
        self.find_float_with_ctype(name.as_ref(), &CLocale)
    }

    pub fn find_bool_with_ctype(&self, name: &[u8], ctype: &dyn ByteCtype) -> i32 {
        find(&self.bools, name, ctype)
    }

    pub fn find_int_with_ctype(&self, name: &[u8], ctype: &dyn ByteCtype) -> i32 {
        find(&self.ints, name, ctype)
    }

    pub fn find_float_with_ctype(&self, name: &[u8], ctype: &dyn ByteCtype) -> i32 {
        find(&self.floats, name, ctype)
    }

    /// Slots 15–17: return the stored lowercase full name, without the `k`.
    pub fn bool_name(&self, index: i32) -> Option<&str> {
        entry(&self.bools, index).and_then(Property::name)
    }

    pub fn int_name(&self, index: i32) -> Option<&str> {
        entry(&self.ints, index).and_then(Property::name)
    }

    pub fn float_name(&self, index: i32) -> Option<&str> {
        entry(&self.floats, index).and_then(Property::name)
    }

    /// Byte-exact name getters, including names produced by a foreign CRT profile.
    pub fn bool_name_bytes(&self, index: i32) -> Option<&[u8]> {
        entry(&self.bools, index).map(Property::name_bytes)
    }

    pub fn int_name_bytes(&self, index: i32) -> Option<&[u8]> {
        entry(&self.ints, index).map(Property::name_bytes)
    }

    pub fn float_name_bytes(&self, index: i32) -> Option<&[u8]> {
        entry(&self.floats, index).map(Property::name_bytes)
    }

    pub fn bool_properties(&self) -> &[Property<bool>] {
        &self.bools
    }

    pub fn int_properties(&self) -> &[Property<i32>] {
        &self.ints
    }

    pub fn float_properties(&self) -> &[Property<f32>] {
        &self.floats
    }
}

/// Only these primitive CRT operations may be foreign. The owner itself performs
/// prefix removal, acronym generation, first-NUL scanning, query copying, folding,
/// comparison and first-match selection. Inputs are sign-extended bytes, including
/// negative ints for 0x80–0xFF; only the low byte of `tolower` is stored (RP5).
pub trait ByteCtype {
    fn tolower(&self, signed_byte: i32) -> i32;
    fn isupper(&self, signed_byte: i32) -> bool;
}

/// Spec §1.3's C-locale path, not a resolution of active-locale binding RP6.
#[derive(Clone, Copy, Debug)]
pub struct CLocale;

impl ByteCtype for CLocale {
    fn tolower(&self, signed_byte: i32) -> i32 {
        if (0x41..=0x5a).contains(&signed_byte) {
            signed_byte + 0x20
        } else {
            signed_byte
        }
    }

    fn isupper(&self, signed_byte: i32) -> bool {
        (0x41..=0x5a).contains(&signed_byte)
    }
}

fn signed(byte: u8) -> i32 {
    i32::from(byte as i8)
}

/// Spec §1.3 helper: CRT uppercase bytes only, then CRT lowering.
fn short_name(name_without_prefix: &[u8], ctype: &dyn ByteCtype) -> Vec<u8> {
    name_without_prefix
        .iter()
        .copied()
        .filter(|&ch| ctype.isupper(signed(ch)))
        .map(|ch| ctype.tolower(signed(ch)) as u8)
        .collect()
}

fn copy_registrations<T: Copy>(
    registrations: &[registry::Registration<T>],
    ctype: &dyn ByteCtype,
) -> Vec<Property<T>> {
    registrations
        .iter()
        .enumerate()
        .map(|(index, registration)| {
            // All public registrations have an ASCII `k` prefix (§3).
            let name = &registration.name.as_bytes()[1..];
            let short_name = short_name(name, ctype);
            Property {
                index: index as i32,
                name: name.iter().map(|&ch| ctype.tolower(signed(ch)) as u8).collect(),
                short_name,
                value: registration.value,
                extra_values: registration.extra_values,
            }
        })
        .collect()
}

fn entry<T>(table: &[Property<T>], index: i32) -> Option<&Property<T>> {
    usize::try_from(index).ok().and_then(|i| table.get(i))
}

fn set<T>(table: &mut [Property<T>], index: i32, value: T) -> bool {
    let Some(property) = usize::try_from(index).ok().and_then(|i| table.get_mut(i)) else {
        return false;
    };
    property.value = value;
    true
}

fn find<T>(table: &[Property<T>], query: &[u8], ctype: &dyn ByteCtype) -> i32 {
    let end = query.iter().position(|&ch| ch == 0).unwrap_or(query.len());
    // Own the scratch copy; never write caller storage or fold bytes after NUL.
    let mut query = query[..end].to_vec();
    for ch in &mut query {
        *ch = ctype.tolower(signed(*ch)) as u8;
    }
    table
        .iter()
        .find(|p| p.name == query || p.short_name == query)
        .map_or(-1, |p| p.index)
}

impl Drop for RenderProperties {
    fn drop(&mut self) {
        // RP3: ordinary destruction releases float, integer, then Boolean tables.
        drop(std::mem::take(&mut self.floats));
        drop(std::mem::take(&mut self.ints));
        drop(std::mem::take(&mut self.bools));
    }
}
