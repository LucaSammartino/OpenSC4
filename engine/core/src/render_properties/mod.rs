// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned rendering-settings registry (`docs/specs/render-properties.md` §§1–3).
//!
//! Each construction copies the registered defaults into independent tables. These are
//! portable core records, not the game's 32-bit entry layout. Rust ownership frees all
//! three tables on drop; COM identity/refcount slots are not modelled (question RP3).
//! Getters borrow immutably and setters require exclusive access, so there is no interior
//! mutable state for a second thread to race. A glue owner can supply its own lock.

mod registry;

#[cfg(test)]
mod tests;

/// One property, including the two uninterpreted extra values of spec §1.2.
/// For booleans these represent the two initially zero bytes (RP2).
#[derive(Clone, Debug, PartialEq)]
pub struct Property<T> {
    index: i32,
    name: String,
    short_name: String,
    value: T,
    extra_values: [T; 2],
}

impl<T> Property<T> {
    pub fn index(&self) -> i32 {
        self.index
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn short_name(&self) -> &str {
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
        Self {
            bools: copy_registrations(&registry::BOOLS),
            ints: copy_registrations(&registry::INTS),
            floats: copy_registrations(&registry::FLOATS),
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

    /// Slots 12–14: exact case-insensitive full-name OR acronym match, in table order.
    pub fn find_bool(&self, name: &str) -> i32 {
        find(&self.bools, name)
    }

    pub fn find_int(&self, name: &str) -> i32 {
        find(&self.ints, name)
    }

    pub fn find_float(&self, name: &str) -> i32 {
        find(&self.floats, name)
    }

    /// Slots 15–17: return the stored lowercase full name, without the `k`.
    pub fn bool_name(&self, index: i32) -> Option<&str> {
        entry(&self.bools, index).map(Property::name)
    }

    pub fn int_name(&self, index: i32) -> Option<&str> {
        entry(&self.ints, index).map(Property::name)
    }

    pub fn float_name(&self, index: i32) -> Option<&str> {
        entry(&self.floats, index).map(Property::name)
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

/// Spec §1.3 helper: capital letters only, converted to lowercase.
fn short_name(name_without_prefix: &str) -> String {
    name_without_prefix
        .bytes()
        .filter(u8::is_ascii_uppercase)
        .map(|ch| char::from(ch.to_ascii_lowercase()))
        .collect()
}

fn copy_registrations<T: Copy>(registrations: &[registry::Registration<T>]) -> Vec<Property<T>> {
    registrations
        .iter()
        .enumerate()
        .map(|(index, registration)| {
            // All public registrations have an ASCII `k` prefix (§3).
            let name = &registration.name[1..];
            Property {
                index: index as i32,
                name: name.to_ascii_lowercase(),
                short_name: short_name(name),
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

fn find<T>(table: &[Property<T>], query: &str) -> i32 {
    // All registry names are ASCII. Literal ASCII folding leaves non-ASCII bytes
    // unmatched; locale-dependent folding and embedded NULs are unresolved (RP5).
    let query = query.to_ascii_lowercase();
    table
        .iter()
        .find(|p| p.name == query || p.short_name == query)
        .map_or(-1, |p| p.index)
}
