// SPDX-License-Identifier: GPL-3.0-or-later
//! Exemplars and cohorts (types [`TYPE_EXEMPLAR`] and [`TYPE_COHORT`]) as the original's factories read them,
//! and the property getters (`docs/specs/data-formats.md` §4 and §5).
//!
//! # Record header (§4.1)
//!
//! An exemplar is stored under type `0x6534284A` with signature `EQZB` (binary) or `EQZT` (text), a cohort under
//! `0x05342861` with `CQZB` / `CQZT`. The reader takes 8 signature bytes (a shorter record fails) and compares
//! **only the first four**: bytes 5–8 (`1###` in every shipped record) are not checked. A cohort record with an
//! `EQZ` signature, or the reverse, fails. [`Exemplar::parse_record`] is the factory read (type check included);
//! [`Exemplar::parse`] takes the kind from the signature, for tools that have no key.
//!
//! # Binary form (§4.2)
//!
//! ```text
//! char[8]  "EQZB1###" / "CQZB1###"
//! u32 × 3  parent cohort key: type, group, instance (all zero = no parent)
//! u32      property count
//! property × count:
//!   u32    property ID
//!   u8     kind: 0 = value, 1 = resource key; anything else fails the read
//!   kind 0: u32 disk tag (foundations §9.2), then the value:
//!           scalar tags 0x00–0x0C: one element (0x00 Empty: nothing; 0x0C: one RZChar byte, not a string)
//!           array tags 0x8000–0x800B, 0x8010, 0x8011: u32 count, then count elements
//!           array tag 0x800C: a string, foundations §2.3 (its 32-bit header is the count; top byte 0x10 = UTF-16)
//!   kind 1: u32 type, u32 group, u32 instance of a resource that must load (§4.6)
//! ```
//!
//! The usual description ("`u16` value type, `u16` key type, one unused byte") is the same bytes: the low byte of
//! the value type is the kind and the next four bytes are the little-endian tag, so the "unused" byte and the high
//! byte of the key type must be zero or the tag is invalid. Any tag outside the reader set fails the read.
//! Properties are added in record order; a later property with an ID replaces the earlier one (§4.5). Bytes after
//! the last property are ignored; a record that ends early fails.
//!
//! # Text form (§4.3)
//!
//! ```text
//! EQZT1###
//! ParentCohort=Key:{0xca386e22,0x0a64ed55,0x05342861}
//! PropCount=0x00000003
//! 0x00000010:{"Exemplar Type"}=Uint32:0:{0x00000002}
//! 0x099afacd:{"Bulldoze Cost"}=Sint64:0:{0x0000000000001356}
//! 0x27812810:{"Occupant Size"}=Float32:3:{80,52,80}
//! ```
//!
//! After the 8 signature bytes the reader **skips 2 bytes unchecked**, splits the rest into lines at `\n` (one
//! trailing `\r` removed, empty lines skipped) and reads: the `ParentCohort` line (key order **group, instance,
//! type**), the `PropCount` line, then exactly `PropCount` property lines `ID:{"name"}=Type:reps:{values}`. Numbers
//! follow rule N ([`rule_n`]); type names are case-sensitive (no `Float64`); `reps` 0 is a scalar, `n ≥ 1` an array
//! of `n` values, missing values read as 0; 64-bit values are always hexadecimal; `Bool` is true when the value
//! begins with `true` (any case); a `String` is everything up to the first `}`, trimmed, one quote removed at each
//! end. See [`Exemplar::parse`] for the details.
//!
//! # Parents and inheritance (§4.4)
//!
//! [`Exemplar`] is one record: its **own** properties, one per ID, sorted by ascending unsigned ID. Loading through
//! an [`ExemplarStore`] gives a [`LoadedExemplar`], which also keeps the **effective** list (the parent cohort's
//! effective list with the own properties applied on top), built when the parent is set: a snapshot. A parent key
//! that does not load as a cohort fails the whole exemplar ([`LoadError`]); so does a kind-1 resource that does
//! not load.
//!
//! # Getters (§5)
//!
//! [`Property`] has the raw accessors of §5.2 (`as_u32`, `u32s`, `count`, ...: no conversion, a type mismatch is a
//! missing value *\[choice\]*) and the converting getters of §5.1 (`get_u32`, `get_f32`, `get_string`, ...): same
//! type as is; a scalar converts through int64 or float64; an array or a string goes through a text round trip of
//! its first element.
//!
//! # Choices
//!
//! Places marked *\[choice\]* below are FreeCity4 decisions where the spec is silent or leaves it to us. The main
//! ones: cycles and chains deeper than [`MAX_PARENT_DEPTH`] fail the load (§4.4 *\[choice\]*); strings are read
//! with the standalone §2.3 rules (quirks not reproduced, foundations §2.3 *\[choice\]*); array counts and text
//! `reps` that cannot fit in the record are rejected.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use crate::dbpf::{Package, Source};
use crate::error::{corrupt, Error, Result};
use crate::reader::Reader;
use crate::tgi::Tgi;
pub use crate::tgi::{TYPE_COHORT, TYPE_EXEMPLAR};

/// Deepest nesting of loads (parent cohorts, and kind-1 resources that are exemplars or cohorts) that
/// [`ExemplarStore::load`] follows below the record being loaded. The original has no limit and recurses without
/// end on a cycle (§4.4); *\[choice\]* FreeCity4 stops at the first repeated key and below this depth, and treats
/// the record as failing to load.
pub const MAX_PARENT_DEPTH: usize = 32;

/// Exemplar or cohort.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Exemplar,
    Cohort,
}

impl Kind {
    /// The resource type a record of this kind must be stored under (§4.1).
    pub fn type_id(self) -> u32 {
        match self {
            Kind::Exemplar => TYPE_EXEMPLAR,
            Kind::Cohort => TYPE_COHORT,
        }
    }

    /// The kind whose records are stored under `type_id`, if any.
    pub fn from_type_id(type_id: u32) -> Option<Kind> {
        match type_id {
            TYPE_EXEMPLAR => Some(Kind::Exemplar),
            TYPE_COHORT => Some(Kind::Cohort),
            _ => None,
        }
    }

    fn letter(self) -> u8 {
        match self {
            Kind::Exemplar => b'E',
            Kind::Cohort => b'C',
        }
    }

    fn name(self) -> &'static str {
        match self {
            Kind::Exemplar => "exemplar",
            Kind::Cohort => "cohort",
        }
    }
}

/// Binary or text encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Binary,
    Text,
}

/// The element type of a property value (foundations §9.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueType {
    /// No value (disk tag `0x0000`, memory type `0x00`).
    Empty,
    Bool,
    Uint8,
    Sint8,
    Uint16,
    Sint16,
    Uint32,
    Sint32,
    Uint64,
    Sint64,
    Float32,
    Float64,
    /// 8-bit characters (memory type RZChar `0x0E`). An array of them is a **string** (disk tag `0x800C`); a
    /// scalar is **one character** (disk tag `0x000C`, §4.2).
    String,
    /// `Char` elements (memory type `0x0C`). Only the array form is readable (disk tag `0x8011`).
    Char,
    /// UTF-16 code units (memory type RZUnicodeChar `0x0D`). Only the array form is readable (disk tag `0x8010`).
    UnicodeChar,
    /// Raw bytes, the "void array" (memory type `0x8000`, disk tag `0x8000`).
    Void,
    /// A kind-1 resource-key property (§4.6; in memory an `IGZUnknown`, `0x10`).
    Resource,
}

impl ValueType {
    /// The types a text property line can name (§4.3), in no particular order. Names are case-sensitive.
    pub const TEXT: [ValueType; 11] = [
        ValueType::Bool,
        ValueType::Uint8,
        ValueType::Sint8,
        ValueType::Uint16,
        ValueType::Sint16,
        ValueType::Uint32,
        ValueType::Sint32,
        ValueType::Uint64,
        ValueType::Sint64,
        ValueType::Float32,
        ValueType::String,
    ];

    /// The disk tag of one element, without the array bit `0x8000` (foundations §9.2). `None` for
    /// [`ValueType::Resource`], which is kind 1 rather than a tag. Empty and Void share element tag 0 (Empty is the
    /// scalar `0x0000`, Void the array `0x8000`).
    pub fn disk_tag(self) -> Option<u32> {
        Some(match self {
            ValueType::Empty | ValueType::Void => 0x00,
            ValueType::Uint8 => 0x01,
            ValueType::Uint16 => 0x02,
            ValueType::Uint32 => 0x03,
            ValueType::Uint64 => 0x04,
            ValueType::Sint8 => 0x05,
            ValueType::Sint16 => 0x06,
            ValueType::Sint32 => 0x07,
            ValueType::Sint64 => 0x08,
            ValueType::Float32 => 0x09,
            ValueType::Float64 => 0x0A,
            ValueType::Bool => 0x0B,
            ValueType::String => 0x0C,
            ValueType::UnicodeChar => 0x10,
            ValueType::Char => 0x11,
            ValueType::Resource => return None,
        })
    }

    /// The type and shape (`true` = array) of a disk tag the reader accepts: scalars `0x00`–`0x0C`, arrays
    /// `0x8000`–`0x800C`, `0x8010`, `0x8011` (§4.2, foundations §9.2). `None` for every other tag, including any tag
    /// with a bit set above bit 15 (the "unused" byte or the key type's high byte).
    pub fn from_disk_tag(tag: u32) -> Option<(ValueType, bool)> {
        let array = tag & 0x8000 != 0;
        let ty = match (tag & !0x8000, array) {
            (0x00, false) => ValueType::Empty,
            (0x00, true) => ValueType::Void,
            (0x01, _) => ValueType::Uint8,
            (0x02, _) => ValueType::Uint16,
            (0x03, _) => ValueType::Uint32,
            (0x04, _) => ValueType::Uint64,
            (0x05, _) => ValueType::Sint8,
            (0x06, _) => ValueType::Sint16,
            (0x07, _) => ValueType::Sint32,
            (0x08, _) => ValueType::Sint64,
            (0x09, _) => ValueType::Float32,
            (0x0A, _) => ValueType::Float64,
            (0x0B, _) => ValueType::Bool,
            (0x0C, _) => ValueType::String,
            (0x10, true) => ValueType::UnicodeChar,
            (0x11, true) => ValueType::Char,
            _ => return None,
        };
        Some((ty, array))
    }

    /// The in-memory type of one element (foundations §9.1); a property's [`Property::memory_type`] adds `0x8000`
    /// when it is an array.
    pub fn memory_type(self) -> u16 {
        match self {
            ValueType::Empty | ValueType::Void => 0x00,
            ValueType::Bool => 0x01,
            ValueType::Uint8 => 0x02,
            ValueType::Sint8 => 0x03,
            ValueType::Uint16 => 0x04,
            ValueType::Sint16 => 0x05,
            ValueType::Uint32 => 0x06,
            ValueType::Sint32 => 0x07,
            ValueType::Uint64 => 0x08,
            ValueType::Sint64 => 0x09,
            ValueType::Float32 => 0x0A,
            ValueType::Float64 => 0x0B,
            ValueType::Char => 0x0C,
            ValueType::UnicodeChar => 0x0D,
            ValueType::String => 0x0E,
            ValueType::Resource => 0x10,
        }
    }

    /// The name used in text exemplars (for the types in [`ValueType::TEXT`]); a descriptive name otherwise.
    pub fn name(self) -> &'static str {
        match self {
            ValueType::Empty => "Empty",
            ValueType::Bool => "Bool",
            ValueType::Uint8 => "Uint8",
            ValueType::Sint8 => "Sint8",
            ValueType::Uint16 => "Uint16",
            ValueType::Sint16 => "Sint16",
            ValueType::Uint32 => "Uint32",
            ValueType::Sint32 => "Sint32",
            ValueType::Uint64 => "Uint64",
            ValueType::Sint64 => "Sint64",
            ValueType::Float32 => "Float32",
            ValueType::Float64 => "Float64",
            ValueType::String => "String",
            ValueType::Char => "Char",
            ValueType::UnicodeChar => "UnicodeChar",
            ValueType::Void => "Void",
            ValueType::Resource => "Resource",
        }
    }

    /// The type a text property line names: exactly one of the [`ValueType::TEXT`] names, case-sensitive (§4.3).
    pub fn from_name(name: &str) -> Option<ValueType> {
        Self::from_text_name(name.as_bytes())
    }

    fn from_text_name(name: &[u8]) -> Option<ValueType> {
        Self::TEXT.into_iter().find(|t| t.name().as_bytes() == name)
    }

    /// Bytes per element in binary form (1 for String, whose elements are bytes; 0 for Empty and Resource).
    pub fn elem_size(self) -> usize {
        match self {
            ValueType::Empty | ValueType::Resource => 0,
            ValueType::Uint8
            | ValueType::Sint8
            | ValueType::Bool
            | ValueType::String
            | ValueType::Char
            | ValueType::Void => 1,
            ValueType::Uint16 | ValueType::Sint16 | ValueType::UnicodeChar => 2,
            ValueType::Uint32 | ValueType::Sint32 | ValueType::Float32 => 4,
            ValueType::Uint64 | ValueType::Sint64 | ValueType::Float64 => 8,
        }
    }
}

/// The values of one property.
#[derive(Clone, Debug, PartialEq)]
pub enum Values {
    /// No value (disk tag 0).
    Empty,
    Bool(Vec<bool>),
    Uint8(Vec<u8>),
    Sint8(Vec<i8>),
    Uint16(Vec<u16>),
    Sint16(Vec<i16>),
    Uint32(Vec<u32>),
    Sint32(Vec<i32>),
    Uint64(Vec<u64>),
    Sint64(Vec<i64>),
    Float32(Vec<f32>),
    Float64(Vec<f64>),
    /// RZChar elements: as an array, a string's bytes (no terminator; a UTF-16 string is stored converted to
    /// UTF-8, foundations §2.3); as a scalar, one 8-bit character.
    String(Vec<u8>),
    /// `Char` array elements (disk tag `0x8011`).
    Char(Vec<u8>),
    /// RZUnicodeChar array elements (disk tag `0x8010`).
    UnicodeChar(Vec<u16>),
    /// Raw bytes of a void array (disk tag `0x8000`).
    Void(Vec<u8>),
    /// The key (type, group, instance) of a kind-1 property's resource (§4.6).
    Resource(Tgi),
}

impl Values {
    pub fn value_type(&self) -> ValueType {
        match self {
            Values::Empty => ValueType::Empty,
            Values::Bool(_) => ValueType::Bool,
            Values::Uint8(_) => ValueType::Uint8,
            Values::Sint8(_) => ValueType::Sint8,
            Values::Uint16(_) => ValueType::Uint16,
            Values::Sint16(_) => ValueType::Sint16,
            Values::Uint32(_) => ValueType::Uint32,
            Values::Sint32(_) => ValueType::Sint32,
            Values::Uint64(_) => ValueType::Uint64,
            Values::Sint64(_) => ValueType::Sint64,
            Values::Float32(_) => ValueType::Float32,
            Values::Float64(_) => ValueType::Float64,
            Values::String(_) => ValueType::String,
            Values::Char(_) => ValueType::Char,
            Values::UnicodeChar(_) => ValueType::UnicodeChar,
            Values::Void(_) => ValueType::Void,
            Values::Resource(_) => ValueType::Resource,
        }
    }

    /// Element count (string length in bytes for String; 0 for Empty, 1 for Resource).
    pub fn len(&self) -> usize {
        match self {
            Values::Empty => 0,
            Values::Resource(_) => 1,
            Values::Bool(v) => v.len(),
            Values::Uint8(v) | Values::String(v) | Values::Char(v) | Values::Void(v) => v.len(),
            Values::Sint8(v) => v.len(),
            Values::Uint16(v) | Values::UnicodeChar(v) => v.len(),
            Values::Sint16(v) => v.len(),
            Values::Uint32(v) => v.len(),
            Values::Sint32(v) => v.len(),
            Values::Uint64(v) => v.len(),
            Values::Sint64(v) => v.len(),
            Values::Float32(v) => v.len(),
            Values::Float64(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Reads `n` elements of `ty` (not Empty, Resource or a §2.3 string).
    fn read(r: &mut Reader<'_>, ty: ValueType, n: usize) -> Result<Values> {
        // [choice] foundations §9.2: a count whose elements would run past the record fails before allocating.
        r.ensure(n as u64, ty.elem_size() as u64)?;
        fn many<T>(n: usize, mut f: impl FnMut() -> Result<T>) -> Result<Vec<T>> {
            (0..n).map(|_| f()).collect()
        }
        Ok(match ty {
            ValueType::Empty => Values::Empty,
            ValueType::Uint8 => Values::Uint8(r.bytes(n)?.to_vec()),
            ValueType::Sint8 => Values::Sint8(r.bytes(n)?.iter().map(|&b| b as i8).collect()),
            // foundations §9.2: any non-zero byte is true.
            ValueType::Bool => Values::Bool(r.bytes(n)?.iter().map(|&b| b != 0).collect()),
            ValueType::String => Values::String(r.bytes(n)?.to_vec()),
            ValueType::Char => Values::Char(r.bytes(n)?.to_vec()),
            ValueType::Void => Values::Void(r.bytes(n)?.to_vec()),
            ValueType::Uint16 => Values::Uint16(many(n, || r.u16())?),
            ValueType::UnicodeChar => Values::UnicodeChar(many(n, || r.u16())?),
            ValueType::Sint16 => Values::Sint16(many(n, || r.u16().map(|v| v as i16))?),
            ValueType::Uint32 => Values::Uint32(many(n, || r.u32())?),
            ValueType::Sint32 => Values::Sint32(many(n, || r.u32().map(|v| v as i32))?),
            ValueType::Float32 => Values::Float32(many(n, || r.u32().map(f32::from_bits))?),
            ValueType::Uint64 => Values::Uint64(many(n, || r.u64())?),
            ValueType::Sint64 => Values::Sint64(many(n, || r.u64().map(|v| v as i64))?),
            ValueType::Float64 => Values::Float64(many(n, || r.u64().map(f64::from_bits))?),
            ValueType::Resource => return Err(corrupt("binary exemplar", "a resource key is not a tagged value")),
        })
    }

    /// The value of a kind-0 property with this tag shape (§4.2).
    fn read_tagged(r: &mut Reader<'_>, ty: ValueType, array: bool) -> Result<Values> {
        if !array {
            return Values::read(r, ty, usize::from(ty != ValueType::Empty));
        }
        if ty == ValueType::String {
            // Tag 0x800C: the §2.3 string header is the count. Standalone [choice] (foundations §2.3): the header
            // decides the encoding and the §2.3 read quirks are errors (crate::reader::Reader::gzstr).
            return Ok(Values::String(r.gzstr()?));
        }
        let n = r.u32()?;
        Values::read(r, ty, n as usize)
    }

    /// The elements in binary form (no count).
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Values::Empty | Values::Resource(_) => {}
            Values::Uint8(v) | Values::String(v) | Values::Char(v) | Values::Void(v) => out.extend_from_slice(v),
            Values::Sint8(v) => out.extend(v.iter().map(|&x| x as u8)),
            Values::Bool(v) => out.extend(v.iter().map(|&x| x as u8)),
            Values::Uint16(v) | Values::UnicodeChar(v) => {
                v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes()))
            }
            Values::Sint16(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
            Values::Uint32(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
            Values::Sint32(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
            Values::Float32(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_bits().to_le_bytes())),
            Values::Uint64(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
            Values::Sint64(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
            Values::Float64(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_bits().to_le_bytes())),
        }
    }
}

/// One exemplar property.
#[derive(Clone, Debug, PartialEq)]
pub struct Property {
    pub id: u32,
    /// Stored as an array (binary tag bit `0x8000`, text `reps` > 0) rather than a single value. See
    /// [`Property::stored_as_array`] for the shape the property actually has.
    pub is_array: bool,
    pub values: Values,
}

/// Generates the raw accessors of §5.2 for one element type.
macro_rules! raw_accessors {
    ($($scalar:ident, $slice:ident, $variant:ident, $t:ty;)*) => {
        $(
            #[doc = concat!("`As", stringify!($variant), "()` (§5.2): the value of a **scalar** ",
                stringify!($variant), " property, without conversion. `None` for any other stored type, arrays \
                included *\\[choice\\]*.")]
            pub fn $scalar(&self) -> Option<$t> {
                match &self.values {
                    Values::$variant(v) if !self.stored_as_array() => v.first().copied(),
                    _ => None,
                }
            }
            #[doc = concat!("`Ref", stringify!($variant), "()` (§5.2): the elements of a ", stringify!($variant),
                " **array**, without conversion. `None` for any other stored type, scalars included *\\[choice\\]*.")]
            pub fn $slice(&self) -> Option<&[$t]> {
                match &self.values {
                    Values::$variant(v) if self.stored_as_array() => Some(v),
                    _ => None,
                }
            }
        )*
    };
}

impl Property {
    /// A scalar property (it should hold exactly one value; see [`Property::stored_as_array`]).
    pub fn single(id: u32, values: Values) -> Property {
        Property {
            id,
            is_array: false,
            values,
        }
    }

    /// An array property.
    pub fn array(id: u32, values: Values) -> Property {
        Property {
            id,
            is_array: true,
            values,
        }
    }

    /// A kind-1 resource-key property (§4.6).
    pub fn resource(id: u32, key: Tgi) -> Property {
        Property::single(id, Values::Resource(key))
    }

    pub fn value_type(&self) -> ValueType {
        self.values.value_type()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// True when the property has the array shape: always for Void, Char and UnicodeChar (their scalar tags are
    /// not readable), never for Empty and Resource, otherwise when [`Property::is_array`] is set or the element
    /// count is not 1 (a scalar holds exactly one element). The binary writer and the getters use this shape.
    pub fn stored_as_array(&self) -> bool {
        match self.values {
            Values::Empty | Values::Resource(_) => false,
            Values::Void(_) | Values::Char(_) | Values::UnicodeChar(_) => true,
            _ => self.is_array || self.len() != 1,
        }
    }

    /// `GetCount()` (§5.2): 0 for a scalar, the element count for an array (the byte length for a string).
    pub fn count(&self) -> usize {
        if self.stored_as_array() {
            self.len()
        } else {
            0
        }
    }

    /// `GetType()`: the in-memory type (foundations §9.1), `0x8000` set for an array. A string is `0x800E`.
    pub fn memory_type(&self) -> u16 {
        self.value_type().memory_type() | if self.stored_as_array() { 0x8000 } else { 0 }
    }

    /// The binary disk tag (foundations §9.2), `None` for a kind-1 resource property.
    pub fn disk_tag(&self) -> Option<u32> {
        let tag = self.value_type().disk_tag()?;
        Some(tag | if self.stored_as_array() { 0x8000 } else { 0 })
    }

    raw_accessors! {
        as_bool, bools, Bool, bool;
        as_u8, u8s, Uint8, u8;
        as_i8, i8s, Sint8, i8;
        as_u16, u16s, Uint16, u16;
        as_i16, i16s, Sint16, i16;
        as_u32, u32s, Uint32, u32;
        as_i32, i32s, Sint32, i32;
        as_u64, u64s, Uint64, u64;
        as_i64, i64s, Sint64, i64;
        as_f32, f32s, Float32, f32;
        as_f64, f64s, Float64, f64;
    }

    /// `AsRZChar()` (§5.2): the character of a scalar RZChar property (disk tag `0x000C`).
    pub fn as_char(&self) -> Option<u8> {
        match &self.values {
            Values::String(v) if !self.stored_as_array() => v.first().copied(),
            _ => None,
        }
    }

    /// The bytes of a string property (an RZChar array, disk tag `0x800C`).
    pub fn string_bytes(&self) -> Option<&[u8]> {
        match &self.values {
            Values::String(v) if self.stored_as_array() => Some(v),
            _ => None,
        }
    }

    /// A string property as UTF-8 (`None` if not a string or not valid UTF-8).
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(self.string_bytes()?).ok()
    }

    /// A string property decoded as UTF-8, or Latin-1 when it is not valid UTF-8.
    pub fn string_lossy(&self) -> Option<String> {
        Some(decode_text(self.string_bytes()?))
    }

    /// The key of a kind-1 resource property (§4.6).
    pub fn resource_key(&self) -> Option<Tgi> {
        match self.values {
            Values::Resource(k) => Some(k),
            _ => None,
        }
    }

    /// `GetValBool(bool&)` (§5.1): converted to Bool (int64 ≠ 0).
    pub fn get_bool(&self) -> Option<bool> {
        Some(self.convert(Target::Int)?.int() != 0)
    }
    /// `GetValUint8(uint8&)` (§5.1): converted, then the low 8 bits. The `Char` and `RZChar` targets of §5.1 give
    /// the same 8 bits (as [`Property::get_u16`] does for `RZUnicodeChar`).
    pub fn get_u8(&self) -> Option<u8> {
        Some(self.convert(Target::Int)?.int() as u8)
    }
    /// `GetValSint8(int8&)` (§5.1): converted, then the low 8 bits.
    pub fn get_i8(&self) -> Option<i8> {
        Some(self.convert(Target::Int)?.int() as i8)
    }
    /// `GetValUint16(uint16&)` (§5.1): converted, then the low 16 bits.
    pub fn get_u16(&self) -> Option<u16> {
        Some(self.convert(Target::Int)?.int() as u16)
    }
    /// `GetValSint16(int16&)` (§5.1): converted, then the low 16 bits.
    pub fn get_i16(&self) -> Option<i16> {
        Some(self.convert(Target::Int)?.int() as i16)
    }
    /// `GetValUint32(uint32&)` and `GetProperty(id, uint32&)` (§5.1): converted, then the low 32 bits.
    pub fn get_u32(&self) -> Option<u32> {
        Some(self.convert(Target::Int)?.int() as u32)
    }
    /// `GetValSint32(int32&)` (§5.1): converted, then the low 32 bits.
    pub fn get_i32(&self) -> Option<i32> {
        Some(self.convert(Target::Int)?.int() as i32)
    }
    /// `GetValUint64(uint64&)` (§5.1): converted through int64.
    pub fn get_u64(&self) -> Option<u64> {
        Some(self.convert(Target::Int)?.int() as u64)
    }
    /// `GetValSint64(int64&)` (§5.1): converted through int64.
    pub fn get_i64(&self) -> Option<i64> {
        Some(self.convert(Target::Int)?.int())
    }
    /// `GetValFloat32(float&)` (§5.1): a scalar → float64 → rounded to float32; arrays and strings by text.
    pub fn get_f32(&self) -> Option<f32> {
        match self.convert(Target::F32)? {
            Num::F32(v) => Some(v),
            n => Some(n.f64() as f32),
        }
    }
    /// `GetValFloat64(double&)` (§5.1): a scalar → float64; arrays and strings by text.
    pub fn get_f64(&self) -> Option<f64> {
        Some(self.convert(Target::F64)?.f64())
    }

    /// `GetProperty(id, cIGZString&)` (§5.1): a string as is; anything else through the text of its value (an
    /// array's first element, `0` for an empty array). Decoded as UTF-8, or Latin-1 when not valid UTF-8.
    pub fn get_string(&self) -> Option<String> {
        let text = match self.form()? {
            Form::Scalar(s) => s.text(),
            Form::Text(t) => t,
        };
        Some(decode_text(&text))
    }

    /// The value as a converting getter sees it: a scalar, or the text of an array or string.
    fn form(&self) -> Option<Form> {
        if self.stored_as_array() {
            return Some(Form::Text(self.array_text()));
        }
        Some(Form::Scalar(match &self.values {
            Values::Bool(v) => Scalar::Bool(*v.first()?),
            Values::Uint8(v) => Scalar::U8(*v.first()?),
            Values::Sint8(v) => Scalar::I8(*v.first()?),
            Values::Uint16(v) => Scalar::U16(*v.first()?),
            Values::Sint16(v) => Scalar::I16(*v.first()?),
            Values::Uint32(v) => Scalar::U32(*v.first()?),
            Values::Sint32(v) => Scalar::I32(*v.first()?),
            Values::Uint64(v) => Scalar::U64(*v.first()?),
            Values::Sint64(v) => Scalar::I64(*v.first()?),
            Values::Float32(v) => Scalar::F32(*v.first()?),
            Values::Float64(v) => Scalar::F64(*v.first()?),
            Values::String(v) => Scalar::Char(*v.first()?),
            // [choice] §5.1 does not say what an Empty value or an object (kind-1) value converts to: the getter
            // fails, as for a missing property. Char, UnicodeChar and Void are always arrays.
            Values::Empty | Values::Resource(_) | Values::Char(_) | Values::UnicodeChar(_) | Values::Void(_) => {
                return None
            }
        }))
    }

    /// The text of an array or string for the text round trip (§5.1): the first element only (`0` for count 0);
    /// a string gives its text.
    fn array_text(&self) -> Vec<u8> {
        fn first<T: Copy>(v: &[T], f: impl Fn(T) -> String) -> Vec<u8> {
            v.first().map_or_else(|| b"0".to_vec(), |&x| f(x).into_bytes())
        }
        match &self.values {
            Values::String(v) => v.clone(),
            // [choice] §5.1 lists the text of integer, float and Bool elements and of strings. Char and
            // RZUnicodeChar arrays are treated as text like an RZChar string; a void array as unsigned bytes.
            Values::Char(v) => v.clone(),
            Values::UnicodeChar(v) => String::from_utf16_lossy(v).into_bytes(),
            Values::Void(v) | Values::Uint8(v) => first(v, |x| x.to_string()),
            Values::Bool(v) => first(v, |x| u8::from(x).to_string()),
            Values::Sint8(v) => first(v, |x| x.to_string()),
            Values::Uint16(v) => first(v, |x| x.to_string()),
            Values::Sint16(v) => first(v, |x| x.to_string()),
            Values::Uint32(v) => first(v, |x| x.to_string()),
            Values::Sint32(v) => first(v, |x| x.to_string()),
            Values::Uint64(v) => first(v, |x| x.to_string()),
            Values::Sint64(v) => first(v, |x| x.to_string()),
            Values::Float32(v) => first(v, |x| format_g24(f64::from(x))),
            Values::Float64(v) => first(v, format_g24),
            Values::Empty | Values::Resource(_) => b"0".to_vec(),
        }
    }

    fn convert(&self, target: Target) -> Option<Num> {
        Some(match self.form()? {
            Form::Scalar(s) => s.convert(target),
            Form::Text(t) => parse_round_trip(&t).convert(target),
        })
    }
}

/// What a converting getter asks for: an integer type, Bool or a char type (through int64), or a float.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Int,
    F32,
    F64,
}

/// A converted value before the target width is applied.
#[derive(Clone, Copy)]
enum Num {
    Int(i64),
    F32(f32),
    F64(f64),
}

impl Num {
    fn int(self) -> i64 {
        match self {
            Num::Int(v) => v,
            Num::F32(v) => truncate(f64::from(v)),
            Num::F64(v) => truncate(v),
        }
    }

    fn f64(self) -> f64 {
        match self {
            Num::Int(v) => v as f64,
            Num::F32(v) => f64::from(v),
            Num::F64(v) => v,
        }
    }
}

/// A float truncated toward zero to int64 (§5.1). *\[choice\]* §5.1 does not say what happens out of the int64
/// range: values saturate and NaN gives 0.
fn truncate(v: f64) -> i64 {
    v as i64
}

enum Form {
    Scalar(Scalar),
    Text(Vec<u8>),
}

/// One scalar value, by stored type.
#[derive(Clone, Copy)]
enum Scalar {
    Bool(bool),
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
    /// RZChar (disk tag `0x000C`).
    Char(u8),
}

impl Scalar {
    /// §5.1: "value → int64 (unsigned types zero-extended, signed sign-extended, floats truncated toward zero)".
    /// *\[choice\]* RZChar is zero-extended (§5.1 does not say whether a char is signed).
    fn to_i64(self) -> i64 {
        match self {
            Scalar::Bool(b) => i64::from(b),
            Scalar::U8(v) | Scalar::Char(v) => i64::from(v),
            Scalar::I8(v) => i64::from(v),
            Scalar::U16(v) => i64::from(v),
            Scalar::I16(v) => i64::from(v),
            Scalar::U32(v) => i64::from(v),
            Scalar::I32(v) => i64::from(v),
            Scalar::U64(v) => v as i64,
            Scalar::I64(v) => v,
            Scalar::F32(v) => truncate(f64::from(v)),
            Scalar::F64(v) => truncate(v),
        }
    }

    /// §5.1: "value → float64 (Uint32 ≥ 2³¹ corrected to unsigned)". *\[choice\]* Uint64 is converted as unsigned
    /// too (§5.1 names only Uint32), and RZChar as unsigned.
    fn to_f64(self) -> f64 {
        match self {
            Scalar::Bool(b) => f64::from(u8::from(b)),
            Scalar::U8(v) | Scalar::Char(v) => f64::from(v),
            Scalar::I8(v) => f64::from(v),
            Scalar::U16(v) => f64::from(v),
            Scalar::I16(v) => f64::from(v),
            Scalar::U32(v) => f64::from(v),
            Scalar::I32(v) => f64::from(v),
            Scalar::U64(v) => v as f64,
            Scalar::I64(v) => v as f64,
            Scalar::F32(v) => f64::from(v),
            Scalar::F64(v) => v,
        }
    }

    fn convert(self, target: Target) -> Num {
        match (self, target) {
            // Same type: as is.
            (Scalar::F32(v), Target::F32) => Num::F32(v),
            (Scalar::F64(v), Target::F64) => Num::F64(v),
            // Float32: value → float64 → rounded to float32.
            (_, Target::F32) => Num::F32(self.to_f64() as f32),
            (_, Target::F64) => Num::F64(self.to_f64()),
            (_, Target::Int) => Num::Int(self.to_i64()),
        }
    }

    /// The text of a scalar for a string target (a requested non-scalar type, §5.1): formatted as an array element
    /// would be. *\[choice\]* An RZChar scalar gives its one character.
    fn text(self) -> Vec<u8> {
        match self {
            Scalar::Bool(b) => vec![b'0' + u8::from(b)],
            Scalar::U8(v) => v.to_string().into_bytes(),
            Scalar::I8(v) => v.to_string().into_bytes(),
            Scalar::U16(v) => v.to_string().into_bytes(),
            Scalar::I16(v) => v.to_string().into_bytes(),
            Scalar::U32(v) => v.to_string().into_bytes(),
            Scalar::I32(v) => v.to_string().into_bytes(),
            Scalar::U64(v) => v.to_string().into_bytes(),
            Scalar::I64(v) => v.to_string().into_bytes(),
            Scalar::F32(v) => format_g24(f64::from(v)).into_bytes(),
            Scalar::F64(v) => format_g24(v).into_bytes(),
            Scalar::Char(c) => vec![c],
        }
    }
}

/// The number the text round trip parses (§5.1).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Parsed {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
}

impl Parsed {
    fn convert(self, target: Target) -> Num {
        match target {
            Target::F32 => Num::F32(self.f64() as f32),
            Target::F64 => Num::F64(self.f64()),
            Target::Int => Num::Int(match self {
                Parsed::Unsigned(v) => v as i64,
                Parsed::Signed(v) => v,
                Parsed::Float(v) => truncate(v),
            }),
        }
    }

    fn f64(self) -> f64 {
        match self {
            Parsed::Unsigned(v) => v as f64,
            Parsed::Signed(v) => v as f64,
            Parsed::Float(v) => v,
        }
    }
}

/// The parsing half of the text round trip (§5.1): `t` / `true` → 1 and `f` / `false` → 0 (the whole text,
/// case-sensitive); text beginning `0x` followed by a hex digit → unsigned hexadecimal; text with `.`, `e` or `E`
/// after a digit → floating point; text containing `-` → signed int64; otherwise unsigned int64. Each number is read
/// as C `strtoull` / `strtod` / `strtoll` read it (a numeric prefix; no digits gives 0; out of range saturates).
/// *\[choice\]* the literal reading: only a lower-case `0x` selects hexadecimal, and "after a digit" means anywhere
/// after the first digit.
fn parse_round_trip(t: &[u8]) -> Parsed {
    match t {
        b"t" | b"true" => return Parsed::Unsigned(1),
        b"f" | b"false" => return Parsed::Unsigned(0),
        _ => {}
    }
    if t.starts_with(b"0x") && t.get(2).is_some_and(u8::is_ascii_hexdigit) {
        return Parsed::Unsigned(strtoul(t, 16, u64::MAX));
    }
    if let Some(d) = t.iter().position(u8::is_ascii_digit) {
        if t[d..].iter().any(|b| matches!(b, b'.' | b'e' | b'E')) {
            return Parsed::Float(strtod(t));
        }
    }
    if t.contains(&b'-') {
        return Parsed::Signed(strtoll(t));
    }
    Parsed::Unsigned(strtoul(t, 10, u64::MAX))
}

/// The float formatter of the text round trip: up to 24 significant digits, `%g` style (fixed notation for
/// exponents −4 to 23, otherwise `d.ddde±XX`), trailing zeros removed. §5.1 *\[inferred\]*: every float value
/// survives the round trip exactly. *\[choice\]* the exponent has at least two digits; NaN and infinities print as
/// `nan` / `inf` (and so parse back as 0).
fn format_g24(v: f64) -> String {
    const DIGITS: usize = 24;
    if v.is_nan() {
        return "nan".into();
    }
    if v.is_infinite() {
        return if v < 0.0 { "-inf" } else { "inf" }.into();
    }
    if v == 0.0 {
        return if v.is_sign_negative() { "-0" } else { "0" }.into();
    }
    let sci = format!("{:.*e}", DIGITS - 1, v);
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let mut out = String::new();
    if v < 0.0 {
        out.push('-');
    }
    if exp < -4 || exp >= DIGITS as i32 {
        let d = digits.trim_end_matches('0');
        out.push_str(&d[..1]);
        if d.len() > 1 {
            out.push('.');
            out.push_str(&d[1..]);
        }
        out.push_str(&format!(
            "e{}{:02}",
            if exp < 0 { '-' } else { '+' },
            exp.unsigned_abs()
        ));
    } else if exp >= 0 {
        let (int, frac) = digits.split_at(exp as usize + 1);
        out.push_str(int);
        let frac = frac.trim_end_matches('0');
        if !frac.is_empty() {
            out.push('.');
            out.push_str(frac);
        }
    } else {
        out.push_str("0.");
        for _ in 0..(-exp - 1) {
            out.push('0');
        }
        out.push_str(digits.trim_end_matches('0'));
    }
    out
}

/// Text bytes as a Rust string: UTF-8 when valid, otherwise Latin-1 (one byte per character).
fn decode_text(b: &[u8]) -> String {
    match std::str::from_utf8(b) {
        Ok(s) => s.to_owned(),
        Err(_) => b.iter().map(|&c| c as char).collect(),
    }
}

/// C `isspace` in the "C" locale.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

fn trim_start(s: &[u8]) -> &[u8] {
    let n = s.iter().take_while(|&&b| is_space(b)).count();
    &s[n..]
}

fn trim(s: &[u8]) -> &[u8] {
    let s = trim_start(s);
    let n = s.iter().rev().take_while(|&&b| is_space(b)).count();
    &s[..s.len() - n]
}

/// C `strtoul` / `strtoull` in `base` 10 or 16: leading white space, an optional sign, in base 16 an optional
/// `0x`/`0X`, then digits up to the first character that is not one. No digits gives 0; a value above `max` gives
/// `max`; a leading `-` negates modulo `max + 1` (`max` is `u32::MAX` or `u64::MAX`).
fn strtoul(s: &[u8], base: u32, max: u64) -> u64 {
    let mut i = s.iter().take_while(|&&b| is_space(b)).count();
    let neg = match s.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    if base == 16
        && s.get(i) == Some(&b'0')
        && matches!(s.get(i + 1), Some(b'x' | b'X'))
        && s.get(i + 2).is_some_and(u8::is_ascii_hexdigit)
    {
        i += 2;
    }
    let mut v: u64 = 0;
    let mut over = false;
    while let Some(d) = s.get(i).and_then(|&c| char::from(c).to_digit(base)) {
        match v.checked_mul(u64::from(base)).and_then(|v| v.checked_add(u64::from(d))) {
            Some(n) if n <= max => v = n,
            _ => over = true,
        }
        i += 1;
    }
    if over {
        max
    } else if neg {
        v.wrapping_neg() & max
    } else {
        v
    }
}

/// C `strtoll` in base 10: as [`strtoul`], saturating at the int64 limits.
fn strtoll(s: &[u8]) -> i64 {
    let t = trim_start(s);
    let (neg, digits) = match t.first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let mut v: u64 = 0;
    let mut over = false;
    for &c in digits.iter().take_while(|c| c.is_ascii_digit()) {
        match v.checked_mul(10).and_then(|v| v.checked_add(u64::from(c - b'0'))) {
            Some(n) => v = n,
            None => over = true,
        }
    }
    let limit = if neg { 1u64 << 63 } else { i64::MAX as u64 };
    if over || v > limit {
        return if neg { i64::MIN } else { i64::MAX };
    }
    if neg {
        (v as i64).wrapping_neg()
    } else {
        v as i64
    }
}

/// C `strtod` on decimal text, locale-independent: leading white space, an optional sign, digits with an optional
/// `.` fraction and an optional exponent. No digits gives 0.0. *\[choice\]* no hexadecimal floats, `inf` or `nan`.
fn strtod(s: &[u8]) -> f64 {
    let digits_from = |i: usize| s[i.min(s.len())..].iter().take_while(|b| b.is_ascii_digit()).count();
    let start = s.iter().take_while(|&&b| is_space(b)).count();
    let mut i = start;
    if matches!(s.get(i), Some(b'+' | b'-')) {
        i += 1;
    }
    let int_digits = digits_from(i);
    i += int_digits;
    let mut frac_digits = 0;
    if s.get(i) == Some(&b'.') {
        frac_digits = digits_from(i + 1);
        if int_digits + frac_digits > 0 {
            i += 1 + frac_digits;
        }
    }
    if int_digits + frac_digits == 0 {
        return 0.0;
    }
    if matches!(s.get(i), Some(b'e' | b'E')) {
        let mut k = i + 1;
        if matches!(s.get(k), Some(b'+' | b'-')) {
            k += 1;
        }
        let n = digits_from(k);
        if n > 0 {
            i = k + n;
        }
    }
    std::str::from_utf8(&s[start..i])
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(0.0)
}

/// Rule N (§4.3) for number text: hexadecimal when it begins with `0x` or `0X`, or when a letter `a`–`f` / `A`–`F`
/// appears before the first character that is not a hex digit; decimal otherwise. The conversion is C `strtoul` in
/// that base (32 bits): it stops at the first character that is not a digit, and a leading `-` negates modulo
/// 2³². So `16` = 16, `1A` = 26, `0x10` = 16, `-1` = `0xFFFFFFFF`, `0x00000010:{"Exemplar Type"}` = 16.
pub fn rule_n(s: &[u8]) -> u32 {
    let hex = s.starts_with(b"0x")
        || s.starts_with(b"0X")
        || s.iter()
            .take_while(|b| b.is_ascii_hexdigit())
            .any(u8::is_ascii_alphabetic);
    strtoul(s, if hex { 16 } else { 10 }, u64::from(u32::MAX)) as u32
}

/// A parsed exemplar or cohort record: its kind, parent key and **own** properties (§4.4).
#[derive(Clone, Debug, PartialEq)]
pub struct Exemplar {
    pub kind: Kind,
    pub encoding: Encoding,
    /// Parent cohort key; all zero = none.
    pub parent: Tgi,
    /// The own properties: one per ID (the last one read wins), sorted by ascending unsigned ID.
    properties: Vec<Property>,
    /// Kind-1 resource keys met while reading, in record order (each must load, §4.6).
    resources: Vec<(u32, Tgi)>,
}

impl Exemplar {
    pub fn new(kind: Kind) -> Exemplar {
        Exemplar {
            kind,
            encoding: Encoding::Binary,
            parent: Tgi::default(),
            properties: Vec::new(),
            resources: Vec::new(),
        }
    }

    /// True if `data` starts with an exemplar or cohort signature: `E` or `C`, `QZ`, then `B` or `T`. Only these
    /// four bytes are compared (§4.1).
    pub fn is_exemplar(data: &[u8]) -> bool {
        data.len() >= 4 && matches!(data[0], b'E' | b'C') && &data[1..3] == b"QZ" && matches!(data[3], b'B' | b'T')
    }

    /// Reads 8 signature bytes and compares the first four (§4.1).
    fn signature(data: &[u8]) -> Result<(Kind, Encoding)> {
        if data.len() < 8 {
            return Err(Error::Truncated {
                what: "exemplar signature",
                needed: 8,
                available: data.len() as u64,
            });
        }
        if !Self::is_exemplar(data) {
            return Err(Error::BadMagic {
                what: "exemplar",
                found: data[..8].to_vec(),
            });
        }
        let kind = if data[0] == b'E' { Kind::Exemplar } else { Kind::Cohort };
        let encoding = if data[3] == b'B' {
            Encoding::Binary
        } else {
            Encoding::Text
        };
        Ok((kind, encoding))
    }

    /// The factory read of a record stored under `type_id` (§4.1): the type must be the exemplar type with an
    /// `EQZ` signature or the cohort type with a `CQZ` signature, otherwise the read fails. Then as
    /// [`Exemplar::parse`]. Parent cohorts and kind-1 resources are resolved by [`ExemplarStore::load`].
    pub fn parse_record(type_id: u32, data: &[u8]) -> Result<Exemplar> {
        let want = Kind::from_type_id(type_id).ok_or_else(|| Error::Unsupported {
            what: "exemplar",
            detail: format!("type {type_id:#010X} is neither the exemplar nor the cohort type"),
        })?;
        let (kind, encoding) = Self::signature(data)?;
        if kind != want {
            return Err(Error::BadMagic {
                what: want.name(),
                found: data[..8].to_vec(),
            });
        }
        Self::parse_body(kind, encoding, data)
    }

    /// Parses either form, taking the kind from the signature (no type check; see [`Exemplar::parse_record`]).
    ///
    /// Binary form: see the module documentation. Text form (§4.3), line by line:
    ///
    /// 1. `ParentCohort=Key:{group,instance,type}`: the name is compared case-insensitively, name and value are
    ///    trimmed, the value must begin `Key:{` and end `}` (case-sensitive) and hold three numbers.
    /// 2. `PropCount=<number>` (name case-insensitive).
    /// 3. Exactly `PropCount` property lines `ID:{"name"}=Type:reps:{values}`. The ID is the text left of the
    ///    **first** `=`, trimmed, read with [`rule_n`] (so a name containing `=` breaks the line *\[quirk\]*); the
    ///    rest is split at the first two `:`. Integer values up to 32 bits: rule N, truncated to the type's width;
    ///    `Uint64` / `Sint64`: hexadecimal with or without `0x`; `Float32`: decimal; `Bool`: true when the
    ///    lower-cased value begins with `true`; `String`: up to the first `}`, trimmed, one `"` removed at each end,
    ///    the `reps` field ignored. Values are separated by commas and their leading spaces trimmed; the reader
    ///    takes `reps` values (one for `reps` 0) and missing values read as 0.
    pub fn parse(data: &[u8]) -> Result<Exemplar> {
        let (kind, encoding) = Self::signature(data)?;
        Self::parse_body(kind, encoding, data)
    }

    fn parse_body(kind: Kind, encoding: Encoding, data: &[u8]) -> Result<Exemplar> {
        match encoding {
            Encoding::Binary => Self::parse_binary(kind, data),
            Encoding::Text => Self::parse_text(kind, data),
        }
    }

    fn parse_binary(kind: Kind, data: &[u8]) -> Result<Exemplar> {
        let mut r = Reader::new(data, "binary exemplar");
        r.bytes(8)?;
        let mut ex = Exemplar::new(kind);
        ex.parent = Tgi::new(r.u32()?, r.u32()?, r.u32()?);
        let count = r.u32()?;
        // The smallest property (an Empty value: ID, kind, tag) takes 9 bytes; reject impossible counts early.
        r.ensure(u64::from(count), 9)?;
        for _ in 0..count {
            let at = r.pos();
            let id = r.u32()?;
            let property = match r.u8()? {
                0 => {
                    let tag = r.u32()?;
                    let (ty, array) = ValueType::from_disk_tag(tag).ok_or_else(|| {
                        corrupt(
                            "binary exemplar",
                            format!("property {id:#010X} at {at}: invalid disk tag {tag:#010X}"),
                        )
                    })?;
                    Property {
                        id,
                        is_array: array,
                        values: Values::read_tagged(&mut r, ty, array)?,
                    }
                }
                1 => {
                    let key = Tgi::new(r.u32()?, r.u32()?, r.u32()?);
                    ex.resources.push((id, key));
                    Property::resource(id, key)
                }
                k => {
                    return Err(corrupt(
                        "binary exemplar",
                        format!("property {id:#010X} at {at}: kind {k} (only 0 and 1 are read)"),
                    ))
                }
            };
            ex.set(property);
        }
        Ok(ex)
    }

    fn parse_text(kind: Kind, data: &[u8]) -> Result<Exemplar> {
        let mut ex = Exemplar::new(kind);
        ex.encoding = Encoding::Text;
        // The 2 bytes after the signature are skipped unchecked (`\r\n` in every shipped record). Lines are numbered
        // as if they were that line break: the signature is line 1.
        let body = data.get(10..).unwrap_or(&[]);
        let mut lines = body
            .split(|&b| b == b'\n')
            .enumerate()
            .map(|(i, l)| (i + 2, l.strip_suffix(b"\r").unwrap_or(l)))
            .filter(|(_, l)| !l.is_empty());
        let mut last = 1;
        let mut next = |what: &str| {
            let (n, line) = lines.next().ok_or_else(|| Error::Text {
                line: last + 1,
                detail: format!("missing {what}"),
            })?;
            last = n;
            Ok::<_, Error>((n, line))
        };
        let err = |line: usize| move |detail: String| Error::Text { line, detail };

        let (n, line) = next("the ParentCohort line")?;
        ex.parent = parse_parent_line(line).map_err(err(n))?;
        let (n, line) = next("the PropCount line")?;
        let count = parse_prop_count(line).map_err(err(n))?;
        for i in 0..count {
            let (n, line) = next(&format!("property line {} of {count}", i + 1))?;
            ex.set(parse_text_property(line, data.len()).map_err(err(n))?);
        }
        Ok(ex)
    }

    /// The own properties: one per ID, sorted by ascending unsigned ID.
    pub fn properties(&self) -> &[Property] {
        &self.properties
    }

    /// The own property with this ID (see [`ExemplarStore::load`] for inheritance).
    pub fn get(&self, id: u32) -> Option<&Property> {
        let i = self.properties.binary_search_by_key(&id, |p| p.id).ok()?;
        Some(&self.properties[i])
    }

    /// Adds a property: one with the same ID is replaced in place (§4.5), otherwise it is inserted in ID order.
    pub fn set(&mut self, p: Property) {
        insert_sorted(&mut self.properties, p);
    }

    /// Removes the property with this ID.
    pub fn remove(&mut self, id: u32) -> Option<Property> {
        let i = self.properties.binary_search_by_key(&id, |p| p.id).ok()?;
        Some(self.properties.remove(i))
    }

    pub fn has_parent(&self) -> bool {
        !self.parent.is_zero()
    }

    /// The kind-1 resource keys met while reading the record, with their property IDs, in record order. The
    /// original loads each one during the read, so each must load even if a later property replaced it (§4.6).
    pub fn resource_keys(&self) -> &[(u32, Tgi)] {
        &self.resources
    }

    /// Binary encoding (`EQZB1###` / `CQZB1###`), properties in ID order. Strings are written as 8-bit §2.3
    /// strings (a UTF-16 string read from data comes back as its UTF-8 bytes); a string longer than 200,000 bytes
    /// is written but cannot be read back.
    pub fn to_binary(&self) -> Vec<u8> {
        let mut out = vec![self.kind.letter()];
        out.extend_from_slice(b"QZB1###");
        for w in [
            self.parent.type_id,
            self.parent.group,
            self.parent.instance,
            self.properties.len() as u32,
        ] {
            out.extend_from_slice(&w.to_le_bytes());
        }
        for p in &self.properties {
            out.extend_from_slice(&p.id.to_le_bytes());
            if let Values::Resource(k) = p.values {
                out.push(1);
                for w in [k.type_id, k.group, k.instance] {
                    out.extend_from_slice(&w.to_le_bytes());
                }
                continue;
            }
            out.push(0);
            let array = p.stored_as_array();
            out.extend_from_slice(&p.disk_tag().unwrap_or(0).to_le_bytes());
            if array {
                // For a string this is the §2.3 header of 8-bit text (encoding byte 0).
                out.extend_from_slice(&(p.len() as u32).to_le_bytes());
            }
            p.values.write(&mut out);
        }
        out
    }

    /// Text encoding (`EQZT1###` / `CQZT1###`), CRLF line ends, property names left empty, 64-bit values in
    /// hexadecimal. Fails for what the text form cannot express (§4.3): Empty, Float64, Char, UnicodeChar, Void and
    /// resource-key values, a single RZChar, an empty array (`reps` 0 reads back as a scalar), a string containing
    /// `}` or a line break, and a non-finite float.
    pub fn to_text(&self) -> Result<Vec<u8>> {
        let mut s = vec![self.kind.letter()];
        s.extend_from_slice(b"QZT1###\r\n");
        s.extend_from_slice(
            format!(
                "ParentCohort=Key:{{0x{:08x},0x{:08x},0x{:08x}}}\r\nPropCount=0x{:08x}\r\n",
                self.parent.group,
                self.parent.instance,
                self.parent.type_id,
                self.properties.len()
            )
            .as_bytes(),
        );
        for p in &self.properties {
            let unsupported = |why: &str| Error::Unsupported {
                what: "text exemplar",
                detail: format!("property {:#010X}: {why}", p.id),
            };
            let ty = p.value_type();
            if !ValueType::TEXT.contains(&ty) {
                return Err(unsupported(&format!("{} values have no text form", ty.name())));
            }
            let array = p.stored_as_array();
            if !array && ty == ValueType::String {
                return Err(unsupported("a single RZChar has no text form"));
            }
            if array && p.is_empty() && ty != ValueType::String {
                return Err(unsupported("an empty array has no text form"));
            }
            match &p.values {
                Values::String(v) if v.contains(&b'}') || v.contains(&b'\n') => {
                    return Err(unsupported("a string containing '}' or a line break"))
                }
                Values::Float32(v) if v.iter().any(|x| !x.is_finite()) => {
                    return Err(unsupported("a non-finite float"))
                }
                _ => {}
            }
            let reps = if array { p.len() } else { 0 };
            s.extend_from_slice(format!("0x{:08x}:{{\"\"}}={}:{}:{{", p.id, ty.name(), reps).as_bytes());
            match &p.values {
                Values::String(v) => {
                    s.push(b'"');
                    s.extend_from_slice(v);
                    s.push(b'"');
                }
                v => s.extend_from_slice(TextValues(v).to_string().as_bytes()),
            }
            s.extend_from_slice(b"}\r\n");
        }
        Ok(s)
    }
}

/// Replaces the property with `p.id` in place, or inserts `p` in ID order.
fn insert_sorted(list: &mut Vec<Property>, p: Property) {
    match list.binary_search_by_key(&p.id, |q| q.id) {
        Ok(i) => list[i] = p,
        Err(i) => list.insert(i, p),
    }
}

/// `ParentCohort=Key:{group,instance,type}` (§4.3).
fn parse_parent_line(line: &[u8]) -> std::result::Result<Tgi, String> {
    let eq = line
        .iter()
        .position(|&b| b == b'=')
        .ok_or("expected ParentCohort=Key:{...}")?;
    let name = trim(&line[..eq]);
    if !name.eq_ignore_ascii_case(b"ParentCohort") {
        return Err(format!("expected ParentCohort, found {:?}", decode_text(name)));
    }
    let value = trim(&line[eq + 1..]);
    let inner = value
        .strip_prefix(b"Key:{")
        .and_then(|v| v.strip_suffix(b"}"))
        .ok_or_else(|| format!("ParentCohort value {:?} is not Key:{{...}}", decode_text(value)))?;
    // [choice] §4.3 says the key "holds three numbers separated by commas": any other count fails the read. Each
    // number has its leading spaces trimmed (as property values do), then rule N.
    let words: Vec<&[u8]> = inner.split(|&b| b == b',').map(trim_start).collect();
    if words.len() != 3 {
        return Err(format!("ParentCohort key has {} fields, not 3", words.len()));
    }
    // Text keys are written group, instance, type (§4.3).
    Ok(Tgi::new(rule_n(words[2]), rule_n(words[0]), rule_n(words[1])))
}

/// `PropCount=<number>` (§4.3).
fn parse_prop_count(line: &[u8]) -> std::result::Result<u32, String> {
    let eq = line
        .iter()
        .position(|&b| b == b'=')
        .ok_or("expected PropCount=<number>")?;
    let name = trim(&line[..eq]);
    if !name.eq_ignore_ascii_case(b"PropCount") {
        return Err(format!("expected PropCount, found {:?}", decode_text(name)));
    }
    // [choice] the value is trimmed on both sides like the ParentCohort value, then rule N.
    Ok(rule_n(trim(&line[eq + 1..])))
}

/// `0x00000010:{"Exemplar Type"}=Uint32:0:{0x00000002}` (§4.3). `limit` is the record length in bytes.
fn parse_text_property(line: &[u8], limit: usize) -> std::result::Result<Property, String> {
    // The ID is the text left of the first '=' (a '=' inside the name breaks the line [quirk]).
    let eq = line.iter().position(|&b| b == b'=').ok_or("missing '='")?;
    let id = rule_n(trim(&line[..eq]));
    let mut parts = line[eq + 1..].splitn(3, |&b| b == b':');
    let ty_name = parts.next().unwrap_or(&[]);
    // [choice] the type name is compared exactly, with no trimming: §4.3 lists the names, "anything else fails".
    let ty = ValueType::from_text_name(ty_name).ok_or_else(|| {
        format!(
            "unknown value type {:?} (names are case-sensitive)",
            decode_text(ty_name)
        )
    })?;
    let reps_text = parts.next().ok_or("missing the reps field")?;
    let values_text = parts.next().ok_or("missing the values")?;
    let open = values_text.strip_prefix(b"{").ok_or("the values must start with '{'")?;
    if ty == ValueType::String {
        // Everything up to the first '}', trimmed, then one leading and one trailing '"' removed; reps ignored.
        let end = open.iter().position(|&b| b == b'}').unwrap_or(open.len());
        let s = trim(&open[..end]);
        let s = s.strip_prefix(b"\"").unwrap_or(s);
        let s = s.strip_suffix(b"\"").unwrap_or(s);
        return Ok(Property::array(id, Values::String(s.to_vec())));
    }
    // [choice] reps is trimmed like the other fields. The reader allocates `reps` values even when fewer are
    // written (missing ones read as 0); a count larger than the record itself is rejected rather than allocated.
    let reps = rule_n(trim(reps_text)) as usize;
    if reps > limit {
        return Err(format!("reps {reps} exceeds the record length {limit}"));
    }
    let n = reps.max(1);
    // Values are separated by commas, leading spaces trimmed; each is read as a numeric prefix, so the closing
    // '}' after the last value is ignored. [choice] the values are not cut at the first '}' (§4.3 only says that
    // of strings).
    let mut tokens = open.split(|&b| b == b',').map(trim_start);
    let mut next = || tokens.next().unwrap_or(&[]);
    macro_rules! each {
        ($variant:ident, $f:expr) => {
            Values::$variant((0..n).map(|_| $f(next())).collect())
        };
    }
    let values = match ty {
        ValueType::Uint8 => each!(Uint8, |w| rule_n(w) as u8),
        ValueType::Sint8 => each!(Sint8, |w| rule_n(w) as i8),
        ValueType::Uint16 => each!(Uint16, |w| rule_n(w) as u16),
        ValueType::Sint16 => each!(Sint16, |w| rule_n(w) as i16),
        ValueType::Uint32 => each!(Uint32, rule_n),
        ValueType::Sint32 => each!(Sint32, |w| rule_n(w) as i32),
        // Always hexadecimal, with or without 0x (§4.3; the no-prefix case is [inferred] there).
        ValueType::Uint64 => each!(Uint64, |w| strtoul(w, 16, u64::MAX)),
        ValueType::Sint64 => each!(Sint64, |w| strtoul(w, 16, u64::MAX) as i64),
        // [choice] decimal text read as float64, then rounded to float32.
        ValueType::Float32 => each!(Float32, |w| strtod(w) as f32),
        ValueType::Bool => each!(Bool, |w: &[u8]| w.to_ascii_lowercase().starts_with(b"true")),
        _ => return Err(format!("{} has no text form", ty.name())),
    };
    Ok(Property {
        id,
        is_array: reps != 0,
        values,
    })
}

/// Formats values as in text exemplars (64-bit integers in hexadecimal, as the text reader expects). Values the
/// text form cannot express are shown for display only.
pub struct TextValues<'a>(pub &'a Values);

impl fmt::Display for TextValues<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn list<T>(
            f: &mut fmt::Formatter<'_>,
            v: &[T],
            g: impl Fn(&mut fmt::Formatter<'_>, &T) -> fmt::Result,
        ) -> fmt::Result {
            for (i, x) in v.iter().enumerate() {
                if i > 0 {
                    f.write_str(",")?;
                }
                g(f, x)?;
            }
            Ok(())
        }
        match self.0 {
            Values::Empty => Ok(()),
            Values::Uint8(v) | Values::Void(v) => list(f, v, |f, x| write!(f, "0x{x:02x}")),
            Values::Uint16(v) => list(f, v, |f, x| write!(f, "0x{x:04x}")),
            Values::Uint32(v) => list(f, v, |f, x| write!(f, "0x{x:08x}")),
            Values::Uint64(v) => list(f, v, |f, x| write!(f, "0x{x:016x}")),
            Values::Sint8(v) => list(f, v, |f, x| write!(f, "{x}")),
            Values::Sint16(v) => list(f, v, |f, x| write!(f, "{x}")),
            Values::Sint32(v) => list(f, v, |f, x| write!(f, "{x}")),
            Values::Sint64(v) => list(f, v, |f, x| write!(f, "0x{:016x}", *x as u64)),
            Values::Float32(v) => list(f, v, |f, x| write!(f, "{x:?}")),
            Values::Float64(v) => list(f, v, |f, x| write!(f, "{x:?}")),
            Values::Bool(v) => list(f, v, |f, x| f.write_str(if *x { "True" } else { "False" })),
            Values::String(v) | Values::Char(v) => write!(f, "\"{}\"", decode_text(v)),
            Values::UnicodeChar(v) => write!(f, "\"{}\"", String::from_utf16_lossy(v)),
            Values::Resource(k) => write!(f, "Key:{{0x{:08x},0x{:08x},0x{:08x}}}", k.type_id, k.group, k.instance),
        }
    }
}

/// An exemplar or cohort as the game holds it once loaded (§4.4): its own properties and its **effective** list
/// (the parent cohort's effective list with the own properties applied on top), both sorted by ascending unsigned
/// ID. The effective list is built when the parent is set and updated on every add: it is a snapshot, so changing
/// the cohort afterwards does not change this exemplar.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedExemplar {
    key: Tgi,
    kind: Kind,
    parent: Tgi,
    own: Vec<Property>,
    effective: Vec<Property>,
}

impl LoadedExemplar {
    /// A record under `key` with no parent applied yet.
    fn from_record(key: Tgi, record: &Exemplar) -> LoadedExemplar {
        LoadedExemplar {
            key,
            kind: record.kind,
            parent: Tgi::default(),
            own: record.properties.clone(),
            effective: record.properties.clone(),
        }
    }

    pub fn key(&self) -> Tgi {
        self.key
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The key of the parent cohort, all zero when there is none.
    pub fn parent(&self) -> Tgi {
        self.parent
    }

    /// The own properties (those of the record), sorted by ID.
    pub fn own_properties(&self) -> &[Property] {
        &self.own
    }

    /// `GetPropertyList` / `EnumProperties`: the effective list, sorted by ascending unsigned ID.
    pub fn properties(&self) -> &[Property] {
        &self.effective
    }

    /// `GetProperty(id)`: the property in the effective list.
    pub fn get(&self, id: u32) -> Option<&Property> {
        let i = self.effective.binary_search_by_key(&id, |p| p.id).ok()?;
        Some(&self.effective[i])
    }

    /// `HasProperty(id)` on the effective list.
    pub fn has_property(&self, id: u32) -> bool {
        self.get(id).is_some()
    }

    /// `IsPropertyLocal(id)` (`cISCResExemplar` slot 8): the own list only.
    pub fn is_property_local(&self, id: u32) -> bool {
        self.own.binary_search_by_key(&id, |p| p.id).is_ok()
    }

    /// `GetProperty(id, uint32&)` (§5.1): `None` when the property is missing.
    pub fn get_u32(&self, id: u32) -> Option<u32> {
        self.get(id)?.get_u32()
    }

    /// `GetProperty(id, cIGZString&)` (§5.1): `None` when the property is missing.
    pub fn get_string(&self, id: u32) -> Option<String> {
        self.get(id)?.get_string()
    }

    /// `AddProperty`: adds to the own list and updates the effective list (same ID replaced in place).
    pub fn set(&mut self, p: Property) {
        insert_sorted(&mut self.effective, p.clone());
        insert_sorted(&mut self.own, p);
    }

    /// `SetCohort`: sets the parent and rebuilds the effective list from the cohort's effective list with the own
    /// properties on top (a snapshot of the cohort as it is now). `None` removes the parent. Returns false, changing
    /// nothing, when `cohort` is an exemplar rather than a cohort.
    pub fn set_cohort(&mut self, cohort: Option<&LoadedExemplar>) -> bool {
        let Some(c) = cohort else {
            self.parent = Tgi::default();
            self.effective = self.own.clone();
            return true;
        };
        if c.kind != Kind::Cohort {
            return false;
        }
        // Both lists are sorted: merge them, an own property replacing the cohort's one with the same ID.
        let mut merged = Vec::with_capacity(c.effective.len() + self.own.len());
        let (mut a, mut b) = (c.effective.iter().peekable(), self.own.iter().peekable());
        loop {
            let next = match (a.peek().map(|p| p.id), b.peek().map(|p| p.id)) {
                (None, None) => break,
                (Some(_), None) => a.next(),
                (None, Some(_)) => b.next(),
                (Some(x), Some(y)) if x < y => a.next(),
                (Some(x), Some(y)) if x > y => b.next(),
                _ => {
                    a.next();
                    b.next()
                }
            };
            merged.extend(next.cloned());
        }
        self.parent = c.key;
        self.effective = merged;
        true
    }
}

/// Why an exemplar or cohort did not load (§4.2, §4.4, §4.6). The original then has no resource under the key:
/// lookups behave as if it did not exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
    /// No exemplar or cohort record (or, for a kind-1 property, no resource) under this key.
    NotFound(Tgi),
    /// The record under this key has the other kind's signature, or the key's type is neither the exemplar nor
    /// the cohort type (§4.1).
    WrongType(Tgi),
    /// A parent key names a record that is not a cohort: only cohorts answer the cohort interface (§4.2).
    NotCohort(Tgi),
    /// Loading this key needs itself (a parent or resource cycle) *\[choice\]*.
    Cycle(Tgi),
    /// Loading this key is nested more than [`MAX_PARENT_DEPTH`] levels below the record asked for *\[choice\]*.
    TooDeep(Tgi),
    /// The parent cohort did not load (§4.2): the whole read fails.
    Parent {
        key: Tgi,
        parent: Tgi,
        cause: Box<LoadError>,
    },
    /// A kind-1 property's resource did not load (§4.6): the whole read fails.
    Resource {
        key: Tgi,
        property: u32,
        resource: Tgi,
        cause: Box<LoadError>,
    },
}

impl LoadError {
    /// True when the failure comes from the depth limit (it depends on where the load started).
    fn too_deep(&self) -> bool {
        match self {
            LoadError::TooDeep(_) => true,
            LoadError::Parent { cause, .. } | LoadError::Resource { cause, .. } => cause.too_deep(),
            _ => false,
        }
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::NotFound(k) => write!(f, "{k} not found"),
            LoadError::WrongType(k) => write!(f, "{k}: the record's signature does not match its type"),
            LoadError::NotCohort(k) => write!(f, "{k} is not a cohort"),
            LoadError::Cycle(k) => write!(f, "{k} needs itself to load (cycle)"),
            LoadError::TooDeep(k) => write!(f, "{k}: loads nested more than {MAX_PARENT_DEPTH} deep"),
            LoadError::Parent { key, parent, cause } => {
                write!(f, "{key}: parent cohort {parent} did not load: {cause}")
            }
            LoadError::Resource {
                key,
                property,
                resource,
                cause,
            } => write!(
                f,
                "{key}: property {property:#010X} resource {resource} did not load: {cause}"
            ),
        }
    }
}

impl std::error::Error for LoadError {}

/// What following an exemplar's parent chain found (a diagnostic; [`ExemplarStore::load`] applies the rules).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChainEnd {
    /// The last exemplar has no parent.
    Root,
    /// A parent key that the store does not contain.
    Missing(Tgi),
    /// A parent key whose record is not a cohort stored under the cohort type.
    NotCohort(Tgi),
    /// A parent key already visited.
    Cycle(Tgi),
    /// More than [`MAX_PARENT_DEPTH`] parents.
    TooDeep,
}

/// A set of exemplar and cohort records by key: the resource manager's view of them.
///
/// \[choice\] Inserting a key that is already present replaces it, so loading packages in the game's load order
/// makes later packages override earlier ones.
#[derive(Default)]
pub struct ExemplarStore {
    map: HashMap<Tgi, Exemplar>,
    /// Keys of other resources, which kind-1 properties may name (§4.6).
    resources: HashSet<Tgi>,
}

/// Result of [`ExemplarStore::load_package`].
#[derive(Debug, Default)]
pub struct LoadReport {
    pub exemplars: usize,
    pub cohorts: usize,
    pub failures: Vec<(Tgi, Error)>,
}

/// Result of [`ExemplarStore::load_all`].
#[derive(Debug, Default)]
pub struct LoadedSet {
    /// Every record that loaded.
    pub exemplars: BTreeMap<Tgi, LoadedExemplar>,
    /// Every record that did not, in key order.
    pub failures: Vec<(Tgi, LoadError)>,
}

impl ExemplarStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Adds or replaces the record under `tgi`.
    pub fn insert(&mut self, tgi: Tgi, ex: Exemplar) -> Option<Exemplar> {
        self.map.insert(tgi, ex)
    }

    /// The record under `tgi`, as parsed (own properties only; it may fail to load).
    pub fn get(&self, tgi: &Tgi) -> Option<&Exemplar> {
        self.map.get(tgi)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Tgi, &Exemplar)> {
        self.map.iter()
    }

    /// Registers the key of a resource other than an exemplar or cohort, so a kind-1 property naming it loads.
    /// *\[choice\]* (§4.6): a kind-1 resource loads when it is an exemplar or cohort that loads, or a registered
    /// key; this crate cannot tell whether the original has a factory for other types.
    pub fn add_resource(&mut self, tgi: Tgi) -> bool {
        self.resources.insert(tgi)
    }

    /// Parses every exemplar and cohort entry of a package into the store, with the factory's type check
    /// ([`Exemplar::parse_record`]), and registers every other entry's key ([`ExemplarStore::add_resource`]).
    pub fn load_package<S: Source>(&mut self, pkg: &Package<S>) -> LoadReport {
        let mut report = LoadReport::default();
        for e in pkg.entries() {
            let ty = e.tgi.type_id;
            if Kind::from_type_id(ty).is_none() {
                self.resources.insert(e.tgi);
                continue;
            }
            match pkg.read(e).and_then(|d| Exemplar::parse_record(ty, &d)) {
                Ok(ex) => {
                    if ty == TYPE_EXEMPLAR {
                        report.exemplars += 1;
                    } else {
                        report.cohorts += 1;
                    }
                    self.map.insert(e.tgi, ex);
                }
                Err(err) => report.failures.push((e.tgi, err)),
            }
        }
        report
    }

    /// Loads the record under `key` as the original's resource manager would (§4.2, §4.4, §4.6): its type must
    /// match its signature; a parent key must load as a cohort (its own parent first, recursively); every kind-1
    /// resource must load. Any failure fails the whole record. The result is a snapshot of the store as it is now.
    pub fn load(&self, key: &Tgi) -> std::result::Result<LoadedExemplar, LoadError> {
        Loader::new(self).load(*key).map(|l| (*l).clone())
    }

    /// Loads every record (each record and parent once), in key order.
    pub fn load_all(&self) -> LoadedSet {
        let mut keys: Vec<Tgi> = self.map.keys().copied().collect();
        keys.sort_unstable();
        let mut loader = Loader::new(self);
        let mut set = LoadedSet::default();
        for key in keys {
            match loader.load(key) {
                Ok(l) => {
                    set.exemplars.insert(key, (*l).clone());
                }
                Err(e) => set.failures.push((key, e)),
            }
        }
        set
    }

    /// `id` in the effective list of the record under `tgi`, or `None` when it is missing or the record does not
    /// load (§4.2: an exemplar whose parent fails does not exist).
    pub fn property(&self, tgi: &Tgi, id: u32) -> Option<&Property> {
        self.load(tgi).ok()?;
        let (chain, _) = self.chain(self.map.get(tgi)?);
        chain.into_iter().find_map(|e| e.get(id))
    }

    /// `SC4GetExemplarProperty(instance, propertyID)` (§5.3): property `id` of the exemplar (`0x6534284A`, group 0,
    /// `instance`), or `None` when the exemplar does not load or lacks the property.
    pub fn exemplar_property(&self, instance: u32, id: u32) -> Option<&Property> {
        self.property(&Tgi::new(TYPE_EXEMPLAR, 0, instance), id)
    }

    /// `ex` followed by its parent cohorts, and how the chain ended. A diagnostic: unlike
    /// [`ExemplarStore::load`] it reports how far a broken chain gets and ignores kind-1 resources.
    pub fn chain<'a>(&'a self, ex: &'a Exemplar) -> (Vec<&'a Exemplar>, ChainEnd) {
        let mut out = vec![ex];
        let mut seen = Vec::new();
        let mut cur = ex;
        loop {
            if !cur.has_parent() {
                return (out, ChainEnd::Root);
            }
            if seen.contains(&cur.parent) {
                return (out, ChainEnd::Cycle(cur.parent));
            }
            if out.len() > MAX_PARENT_DEPTH {
                return (out, ChainEnd::TooDeep);
            }
            seen.push(cur.parent);
            match self.map.get(&cur.parent) {
                Some(p) if cur.parent.type_id == TYPE_COHORT && p.kind == Kind::Cohort => {
                    out.push(p);
                    cur = p;
                }
                Some(_) => return (out, ChainEnd::NotCohort(cur.parent)),
                None => return (out, ChainEnd::Missing(cur.parent)),
            }
        }
    }

    /// Every property visible on `ex` through as much of its chain as exists (own ones override inherited ones),
    /// sorted by ID. Each item also says how many levels up it came from (0 = own). A diagnostic, see
    /// [`ExemplarStore::chain`].
    pub fn resolved_properties<'a>(&'a self, ex: &'a Exemplar) -> Vec<(usize, &'a Property)> {
        let (chain, _) = self.chain(ex);
        let mut map: BTreeMap<u32, (usize, &Property)> = BTreeMap::new();
        for (depth, e) in chain.iter().enumerate().rev() {
            for p in &e.properties {
                map.insert(p.id, (depth, p));
            }
        }
        map.into_values().collect()
    }
}

/// A memoised result of [`Loader::load`].
enum Memo {
    /// Independent of where the load started.
    Done(std::result::Result<Rc<LoadedExemplar>, LoadError>),
    /// Failed on the depth limit when loaded at this nesting depth; holds for any depth at least as deep.
    TooDeepAt(usize, LoadError),
}

/// One run of loads sharing a cache, like the resource manager's.
struct Loader<'a> {
    store: &'a ExemplarStore,
    memo: HashMap<Tgi, Memo>,
    /// Keys being loaded, outermost first.
    stack: Vec<Tgi>,
}

impl<'a> Loader<'a> {
    fn new(store: &'a ExemplarStore) -> Self {
        Loader {
            store,
            memo: HashMap::new(),
            stack: Vec::new(),
        }
    }

    fn load(&mut self, key: Tgi) -> std::result::Result<Rc<LoadedExemplar>, LoadError> {
        let depth = self.stack.len();
        match self.memo.get(&key) {
            Some(Memo::Done(r)) => return r.clone(),
            Some(Memo::TooDeepAt(d, e)) if depth >= *d => return Err(e.clone()),
            _ => {}
        }
        // [choice] §4.4: the original recurses without end on a cycle and has no depth limit.
        if self.stack.contains(&key) {
            return Err(LoadError::Cycle(key));
        }
        if depth > MAX_PARENT_DEPTH {
            return Err(LoadError::TooDeep(key));
        }
        self.stack.push(key);
        let r = self.read(key);
        self.stack.pop();
        let memo = match &r {
            Err(e) if e.too_deep() => Memo::TooDeepAt(depth, e.clone()),
            _ => Memo::Done(r.clone()),
        };
        self.memo.insert(key, memo);
        r
    }

    fn read(&mut self, key: Tgi) -> std::result::Result<Rc<LoadedExemplar>, LoadError> {
        let store: &'a ExemplarStore = self.store;
        let record = store.map.get(&key).ok_or(LoadError::NotFound(key))?;
        if Kind::from_type_id(key.type_id) != Some(record.kind) {
            return Err(LoadError::WrongType(key));
        }
        let mut loaded = LoadedExemplar::from_record(key, record);
        if record.has_parent() {
            // The parent is requested with the cohort interface: it must load, and be a cohort (§4.2).
            let parent = record.parent;
            let cohort = self
                .load(parent)
                .and_then(|c| {
                    if c.kind == Kind::Cohort {
                        Ok(c)
                    } else {
                        Err(LoadError::NotCohort(parent))
                    }
                })
                .map_err(|cause| LoadError::Parent {
                    key,
                    parent,
                    cause: Box::new(cause),
                })?;
            loaded.set_cohort(Some(&cohort));
        }
        for &(property, resource) in &record.resources {
            let fail = |cause| LoadError::Resource {
                key,
                property,
                resource,
                cause: Box::new(cause),
            };
            if store.map.contains_key(&resource) {
                self.load(resource).map_err(fail)?;
            } else if !store.resources.contains(&resource) {
                return Err(fail(LoadError::NotFound(resource)));
            }
        }
        Ok(Rc::new(loaded))
    }
}

#[cfg(test)]
mod tests;
