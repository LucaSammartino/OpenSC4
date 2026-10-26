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
//! type**, fields not counted: a missing one reads 0), the `PropCount` line, then exactly `PropCount` property lines
//! `ID:{"name"}=Type:reps:{values}`. Numbers follow rule N ([`rule_n`]); type names are case-sensitive (no
//! `Float64`) and compared exactly; `reps` is not trimmed; `reps` 0 is a scalar, `n ≥ 1` an array of `n` values,
//! missing values read as 0 and there is no cap (*\[choice\]* above 16 MiB of values the read fails); array values
//! have their leading spaces and tabs trimmed, a scalar's value does not; 64-bit values are always hexadecimal,
//! without a sign; `Float32` uses the game's own digit-by-digit parser at 24-bit precision ([`parse_float`]);
//! `Bool` is true when the value, its leading spaces and tabs trimmed, begins with `true` (any case); a `String` is
//! everything up to the first `}`, trimmed of spaces and tabs, then one quote removed at each end. See
//! [`Exemplar::parse`] for the details.
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
//! missing value *\[choice\]*), holder slot 5 ([`Property::get_void`]: a void array only) and the converting
//! getters of §5.1 (`get_u32`, `get_f32`, `get_string_into`, ...): same type as is; a scalar converts through
//! int64 or float64 (floats truncated and wrapped, Char and RZChar signed, a Uint64 and a Uint32 from 2³¹ kept to
//! 24 significant bits); an array, a string and an Empty value go through a text round trip ([`Property::text`],
//! parsed as `ConvertFromString` does). An Empty value (tag 0, §4.2) is a property like any other: it is present,
//! has no value bytes, and reads as 0. An object (kind-1) value is a missing property *\[choice\]*.
//!
//! # Choices
//!
//! Places marked *\[choice\]* below are FreeCity4 decisions where the spec is silent or leaves it to us. The main
//! ones: cycles and chains deeper than [`MAX_PARENT_DEPTH`] fail the load (§4.4 *\[choice\]*); strings are read
//! with the standalone §2.3 rules (quirks not reproduced, foundations §2.3 *\[choice\]*); binary array counts that
//! cannot fit in the record are rejected; text `reps` above 16 MiB of values fail the read (§4.3 *\[choice\]*); an
//! object value is a missing property for every getter (§5.1 *\[choice\]*); a raw accessor on the wrong type is a
//! missing value (§5.2 *\[choice\]*).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use crate::dbpf::{Package, Source};
use crate::error::{corrupt, Error, Result};
use crate::reader::Reader;
use crate::tgi::Tgi;
pub use crate::tgi::{TYPE_COHORT, TYPE_EXEMPLAR};
use crate::x87;

/// Deepest nesting of loads (parent cohorts, and kind-1 resources that are exemplars or cohorts) that
/// [`ExemplarStore::load`] follows below the record being loaded. The original has no limit and recurses without
/// end on a cycle (§4.4); *\[choice\]* FreeCity4 stops at the first repeated key and below this depth, and treats
/// the record as failing to load. Depth counts the nested loads below the record requested, so at most 32
/// ancestors load. A failure is not remembered: a record that failed only on depth loads when it is requested at a
/// shallower depth, as the original, which caches only successful loads, would load it again (§4.4, EX19).
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

/// Generates the raw accessors of §5.2 for one element type. The original does not check the type: on a mismatch
/// `AsX()` returns the stored bytes reinterpreted (a Float32 2.75 read with `AsUint32()` gives `0x40300000`, an
/// array its heap address) and `RefX()` the data pointer. That is garbage for the caller, so *\[choice\]* (§5.2,
/// EX20) a type mismatch, scalar against array included, is a missing value here.
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

    /// `GetValBool(bool&)` (§5.1): a Bool as is; another scalar's int64 ≠ 0 (a float is truncated first, so 0.5
    /// gives false and NaN true); through the text round trip, the parsed number ≠ 0 (the text `0.5` gives true).
    pub fn get_bool(&self) -> Option<bool> {
        Some(match self.form()? {
            Form::Scalar(s) => s.to_bool(),
            Form::Text(t) => parse_round_trip(&t).to_bool(),
        })
    }
    /// `GetValUint8(uint8&)` (§5.1): converted through int64, then the low 8 bits.
    pub fn get_u8(&self) -> Option<u8> {
        Some(self.int()? as u8)
    }
    /// `GetValSint8(int8&)` (§5.1): converted through int64, then the low 8 bits.
    pub fn get_i8(&self) -> Option<i8> {
        Some(self.int()? as i8)
    }
    /// `GetValUint16(uint16&)` (§5.1): converted through int64, then the low 16 bits.
    pub fn get_u16(&self) -> Option<u16> {
        Some(self.int()? as u16)
    }
    /// `GetValSint16(int16&)` (§5.1): converted through int64, then the low 16 bits.
    pub fn get_i16(&self) -> Option<i16> {
        Some(self.int()? as i16)
    }
    /// `GetValUint32(uint32&)` and `GetProperty(id, uint32&)` (§5.1): converted through int64, then the low 32 bits.
    pub fn get_u32(&self) -> Option<u32> {
        Some(self.int()? as u32)
    }
    /// `GetValSint32(int32&)` (§5.1): converted through int64, then the low 32 bits.
    pub fn get_i32(&self) -> Option<i32> {
        Some(self.int()? as i32)
    }
    /// `GetValUint64(uint64&)` (§5.1): converted through int64.
    pub fn get_u64(&self) -> Option<u64> {
        Some(self.int()? as u64)
    }
    /// `GetValSint64(int64&)` (§5.1): converted through int64.
    pub fn get_i64(&self) -> Option<i64> {
        self.int()
    }
    /// `GetValChar(char&)` and `GetValRZChar(char&)` (§5.1): a scalar through int64, then the low 8 bits; through
    /// the text round trip (arrays, strings, Empty) always 0, and the getter still succeeds.
    pub fn get_char(&self) -> Option<u8> {
        Some(match self.form()? {
            Form::Scalar(s) => s.to_i64() as u8,
            Form::Text(_) => 0,
        })
    }
    /// `GetValRZUnicodeChar(uint16&)` (§5.1): as [`Property::get_char`], with the low 16 bits.
    pub fn get_unicode_char(&self) -> Option<u16> {
        Some(match self.form()? {
            Form::Scalar(s) => s.to_i64() as u16,
            Form::Text(_) => 0,
        })
    }
    /// `GetValFloat32(float&)` (§5.1): a Float32 as is; another scalar → float64 → rounded to float32; through the
    /// text round trip, the parsed number rounded to float32 (so the round trip of a float is not exact).
    pub fn get_f32(&self) -> Option<f32> {
        Some(match self.form()? {
            Form::Scalar(Scalar::F32(v)) => v,
            Form::Scalar(s) => s.to_f64() as f32,
            Form::Text(t) => parse_round_trip(&t).to_f32(),
        })
    }
    /// `GetValFloat64(double&)` (§5.1): a scalar → float64 (unsigned 64-bit values and Uint32 from 2³¹ keep 24
    /// significant bits); through the text round trip, the parsed number.
    pub fn get_f64(&self) -> Option<f64> {
        Some(match self.form()? {
            Form::Scalar(s) => s.to_f64(),
            Form::Text(t) => parse_round_trip(&t).to_f64(),
        })
    }

    /// `ConvertToString` (§5.1): the text of the value, as the text round trip and the string getter use it.
    ///
    /// - A numeric or Bool array gives its **first element** (`0` when it has none); a string (RZChar array) and a
    ///   Char array all their characters; an RZUnicodeChar array all its units, each below `0x80` as that byte and
    ///   any other as `?`; a void array the empty text (EX6).
    /// - A scalar: integers in decimal, Bool `0` / `1`, an RZChar its one character (EX7).
    /// - Floats: `RZFtoaEnglishA` (fixed-point, 17 significant digits rounded half-up, cut at 63 characters; EX8 as
    ///   corrected by EX29; −0.0 prints `0`, EX24; a signalling NaN prints `1.#QNAN`, EX28).
    /// - An Empty value: the empty text (EX5).
    ///
    /// `None` for an object (kind-1) value: the original prints its text or its address, *\[choice\]* FreeCity4
    /// treats it as a missing property.
    pub fn text(&self) -> Option<Vec<u8>> {
        Some(match self.form()? {
            Form::Scalar(s) => s.text(),
            Form::Text(t) => t,
        })
    }

    /// `GetValString` and `GetProperty(id, cIGZString&)` (§5.1): writes the value's text ([`Property::text`]) into
    /// `out`, replacing it, and returns true only for a **string** (an RZChar array). Every other type, Empty
    /// included, has its text written and returns false: a Uint32 7 gives `7` and false (EX5). *\[choice\]* An
    /// object (kind-1) value is a missing property: false, `out` unchanged.
    pub fn get_string_into(&self, out: &mut Vec<u8>) -> bool {
        match self.text() {
            Some(t) => {
                *out = t;
                self.string_bytes().is_some()
            }
            None => false,
        }
    }

    /// The string getter for callers that check its result: the text of a string property, decoded as UTF-8, or
    /// Latin-1 when it is not valid UTF-8. `None` for every other type, for which the getter returns false
    /// ([`Property::get_string_into`] gives the text it writes).
    pub fn get_string(&self) -> Option<String> {
        self.string_lossy()
    }

    /// Holder slot 5 `GetProperty(id, void* buffer, uint32& length)` through `GetValVoid` (§5.2, EX17): only a
    /// **void array** answers. For one of `n` bytes, `length` is lowered to `n` when it is larger, the first
    /// `length` bytes are copied into `buffer`, and the call returns true. Any other type, strings and other arrays
    /// included, returns false and leaves both unchanged (no text round trip). No shipped record has a void array.
    /// The caller's buffer must hold `length` bytes, as in the original; *\[choice\]* a shorter one receives only
    /// the bytes that fit. The original (`0x00915714`) cannot know the buffer's size and overruns a shorter one when
    /// the array is longer; that is not observable on shipped data, so the choice stands (§5.2, EX30).
    pub fn get_void(&self, buffer: &mut [u8], length: &mut u32) -> bool {
        let Values::Void(v) = &self.values else {
            return false;
        };
        if *length as usize > v.len() {
            *length = v.len() as u32;
        }
        let n = (*length as usize).min(buffer.len());
        buffer[..n].copy_from_slice(&v[..n]);
        true
    }

    /// The int64 a converting getter computes before keeping the target's low bits.
    fn int(&self) -> Option<i64> {
        Some(match self.form()? {
            Form::Scalar(s) => s.to_i64(),
            Form::Text(t) => parse_round_trip(&t).to_i64(),
        })
    }

    /// The value as a converting getter sees it (§5.1, `ConvertTypes`): a scalar of a type Bool to RZChar converts
    /// directly; an array, a string and an Empty value go through the text round trip. `None` for an object
    /// (kind-1) value *\[choice\]* (the original's result would be a heap address).
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
            // §5.1 (EX5): memory type 0 takes the text round trip as the empty text, which parses as 0.
            Values::Empty => return Some(Form::Text(Vec::new())),
            Values::Resource(_) => return None,
            // Always arrays (see stored_as_array).
            Values::Char(_) | Values::UnicodeChar(_) | Values::Void(_) => return Some(Form::Text(self.array_text())),
        }))
    }

    /// The text of an array or a string for the text round trip (§5.1): see [`Property::text`].
    fn array_text(&self) -> Vec<u8> {
        fn first<T: Copy>(v: &[T], f: impl Fn(T) -> String) -> Vec<u8> {
            v.first().map_or_else(|| b"0".to_vec(), |&x| f(x).into_bytes())
        }
        match &self.values {
            Values::String(v) | Values::Char(v) => v.clone(),
            Values::UnicodeChar(v) => v.iter().map(|&u| if u < 0x80 { u as u8 } else { b'?' }).collect(),
            // A void array (and a VoidPtr array, not readable here) gives the empty text (EX6).
            Values::Void(_) | Values::Empty | Values::Resource(_) => Vec::new(),
            Values::Uint8(v) => first(v, |x| x.to_string()),
            Values::Bool(v) => first(v, |x| u8::from(x).to_string()),
            Values::Sint8(v) => first(v, |x| x.to_string()),
            Values::Uint16(v) => first(v, |x| x.to_string()),
            Values::Sint16(v) => first(v, |x| x.to_string()),
            Values::Uint32(v) => first(v, |x| x.to_string()),
            Values::Sint32(v) => first(v, |x| x.to_string()),
            Values::Uint64(v) => first(v, |x| x.to_string()),
            Values::Sint64(v) => first(v, |x| x.to_string()),
            Values::Float32(v) => first(v, format_f32),
            Values::Float64(v) => first(v, format_float),
        }
    }
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
    /// The int64 step of the converting getters (§5.1, `0x00915AA5`): unsigned types zero-extended, signed types
    /// sign-extended, an RZChar **sign-extended** as a signed byte (EX4), floats truncated toward zero by `_ftol`,
    /// which wraps rather than saturates: NaN, ±∞ and magnitudes ≥ 2⁶³ give `0x8000000000000000` (EX3).
    fn to_i64(self) -> i64 {
        match self {
            Scalar::Bool(b) => i64::from(b),
            Scalar::U8(v) => i64::from(v),
            Scalar::Char(v) => i64::from(v as i8),
            Scalar::I8(v) => i64::from(v),
            Scalar::U16(v) => i64::from(v),
            Scalar::I16(v) => i64::from(v),
            Scalar::U32(v) => i64::from(v),
            Scalar::I32(v) => i64::from(v),
            Scalar::U64(v) => v as i64,
            Scalar::I64(v) => v,
            Scalar::F32(v) => x87::ftol(f64::from(v)),
            Scalar::F64(v) => x87::ftol(v),
        }
    }

    /// The to-double step (§5.1, `0x00915B85`): signed values exactly (a Sint64 rounded to 53 bits), an RZChar as a
    /// signed byte (EX4), a Uint32 below 2³¹ exactly, and a Uint32 from 2³¹ and every Uint64 to **24 significant
    /// bits** (the unsigned correction is an x87 add at the main thread's precision, EX2).
    fn to_f64(self) -> f64 {
        match self {
            Scalar::Bool(b) => f64::from(u8::from(b)),
            Scalar::U8(v) => f64::from(v),
            Scalar::Char(v) => f64::from(v as i8),
            Scalar::I8(v) => f64::from(v),
            Scalar::U16(v) => f64::from(v),
            Scalar::I16(v) => f64::from(v),
            Scalar::U32(v) => x87::u32_to_f64(v),
            Scalar::I32(v) => f64::from(v),
            Scalar::U64(v) => x87::u64_to_f64(v),
            Scalar::I64(v) => v as f64,
            Scalar::F32(v) => f64::from(v),
            Scalar::F64(v) => v,
        }
    }

    /// A Bool as is; any other type int64 ≠ 0.
    fn to_bool(self) -> bool {
        match self {
            Scalar::Bool(b) => b,
            s => s.to_i64() != 0,
        }
    }

    /// The text of a scalar (§5.1): integers in decimal, Bool `0` / `1`, floats by [`format_float`], an RZChar its
    /// one character (a zero byte gives a one-character text holding NUL).
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
            Scalar::F32(v) => format_f32(v).into_bytes(),
            Scalar::F64(v) => format_float(v).into_bytes(),
            Scalar::Char(c) => vec![c],
        }
    }
}

/// The number the text round trip parses (§5.1).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Parsed {
    /// Hexadecimal, `t`/`true`/`f`/`false`, or decimal without a `-`.
    Unsigned(u64),
    /// Decimal with a `-`: the same bits as unsigned, but converted exactly for a float target.
    Signed(i64),
    /// The 24-bit value of the game's float parser ([`parse_float`]).
    Float(f64),
}

impl Parsed {
    /// Integer targets keep the low bits; a float is truncated first (`_ftol`, wrapping).
    fn to_i64(self) -> i64 {
        match self {
            Parsed::Unsigned(v) => v as i64,
            Parsed::Signed(v) => v,
            Parsed::Float(v) => x87::ftol(v),
        }
    }

    /// Bool is value ≠ 0, a float not truncated first: `0.5` gives true, and so does NaN.
    fn to_bool(self) -> bool {
        match self {
            Parsed::Unsigned(v) => v != 0,
            Parsed::Signed(v) => v != 0,
            Parsed::Float(v) => v != 0.0,
        }
    }

    /// An unsigned number takes the 24-bit path of the to-double converter; a signed one is converted exactly,
    /// then rounded to the target; a float is the parser's 24-bit value.
    fn to_f64(self) -> f64 {
        match self {
            Parsed::Unsigned(v) => x87::u64_to_f64(v),
            Parsed::Signed(v) => v as f64,
            Parsed::Float(v) => v,
        }
    }

    /// As [`Parsed::to_f64`], rounded to float32. A signed number is rounded **once**, from its exact value (`fild`
    /// then `fstp dword`, §5.1 step 4, EX25): `-16777217` gives −16,777,216.0. (A Sint64 *scalar* rounds twice,
    /// through the double the converter stores: see [`Scalar::to_f64`].)
    fn to_f32(self) -> f32 {
        match self {
            Parsed::Unsigned(v) => x87::u64_to_f64(v) as f32,
            Parsed::Signed(v) => v as f32,
            Parsed::Float(v) => x87::store_f32(v),
        }
    }
}

/// The parsing half of the text round trip (§5.1, `ConvertFromString`, EX9), in this order:
///
/// 1. the **whole** text `t` or `true` gives 1 and `f` or `false` 0, case-sensitive;
/// 2. text that starts with a lower-case `0x` followed by a hex digit is unsigned hexadecimal ([`atou64_hex`]);
/// 3. otherwise a scan from the start sets *dot* at any `.`, *minus* at a `-`, and *exponent* at an `e` / `E` that
///    follows some decimal digit, anywhere before it. It stops at an `e` / `E` with no digit before it and at a
///    second `-`; the flags already set decide. (Its other stops, after the exponent letter, cannot change the
///    outcome: the text is floating point by then.)
/// 4. *dot* or *exponent*: floating point ([`parse_float`]); otherwise *minus*: signed decimal; otherwise unsigned
///    decimal (both [`atoi64`]). There is no hexadecimal without the `0x` prefix.
///
/// So `0X1F` gives 0, `1A` 1, `TRUE` 0, `-5` −5, `1-2` 1, `e5` 0, `1e3` 1000.0, `.5` 0.5, `1.2.3` 1.2 and
/// `12 apples` 12.0 (the `e` of `apples` follows a digit).
fn parse_round_trip(t: &[u8]) -> Parsed {
    match t {
        b"t" | b"true" => return Parsed::Unsigned(1),
        b"f" | b"false" => return Parsed::Unsigned(0),
        _ => {}
    }
    if t.starts_with(b"0x") && t.get(2).is_some_and(u8::is_ascii_hexdigit) {
        return Parsed::Unsigned(atou64_hex(t));
    }
    let (mut digit, mut minus) = (false, false);
    for &c in t {
        match c {
            b'0'..=b'9' => digit = true,
            b'.' => return Parsed::Float(parse_float(t)),
            b'e' | b'E' if digit => return Parsed::Float(parse_float(t)),
            b'e' | b'E' => break,
            b'-' if minus => break,
            b'-' => minus = true,
            _ => {}
        }
    }
    if minus {
        Parsed::Signed(atoi64(t))
    } else {
        Parsed::Unsigned(atoi64(t) as u64)
    }
}

/// `RZAtoi64A` (§5.1): leading white space, one optional `+` or `-`, then decimal digits up to the first other
/// character, accumulated in 64 bits with wraparound.
fn atoi64(s: &[u8]) -> i64 {
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
    let mut v: u64 = 0;
    for &c in s[i..].iter().take_while(|c| c.is_ascii_digit()) {
        v = v.wrapping_mul(10).wrapping_add(u64::from(c - b'0'));
    }
    if neg {
        (v as i64).wrapping_neg()
    } else {
        v as i64
    }
}

/// `RZAtoUint64HexA` (§4.3, §5.1): leading white space, an optional `0x` or `0X`, then at most **16** hexadecimal
/// digits up to the first other character (digits after the sixteenth are ignored). There is no sign: `-1` gives 0
/// and `10` gives 16.
fn atou64_hex(s: &[u8]) -> u64 {
    let mut i = s.iter().take_while(|&&b| is_space(b)).count();
    if s.get(i) == Some(&b'0') && matches!(s.get(i + 1), Some(b'x' | b'X')) {
        i += 2;
    }
    let mut v: u64 = 0;
    for d in s[i.min(s.len())..]
        .iter()
        .map_while(|&c| char::from(c).to_digit(16))
        .take(16)
    {
        v = v << 4 | u64::from(d);
    }
    v
}

/// The game's own decimal parser (`RZStrtodEnglishA`, §4.3 Float32, EX14); also the floating-point case of the
/// text round trip (§5.1). Leading white space and one optional sign are skipped, then the value is built **digit
/// by digit in x87 arithmetic at the main thread's 24-bit precision**, every operation rounded once: an integer
/// digit gives value × 10 + digit, a fraction digit multiplies a running scale by 0.1 and adds digit × scale. An
/// `e` / `E` exponent is read as a 32-bit integer; 10ⁿ is computed in 32-bit integer arithmetic, **wrapping**, and
/// loaded as a signed integer, then multiplied in. A negative exponent is a **reciprocal and then a multiply**
/// (EX35): 1/10ⁿ is rounded to the 24-bit precision first, then multiplied in, two roundings, so `9e-1` gives
/// `0x3F666667` where one division gives `0x3F666666`. There is no hexadecimal, `inf` or `nan`: parsing stops at
/// the first character that does not fit, so `inf` gives 0 and `1.#INF` 1.
///
/// The result keeps a 24-bit significand and the register's exponent range; a Float32 target rounds it once more
/// when it is stored (a Float64 target gets it as is). Worked examples (§4.3): `0.05` gives `0x3D4CCCCC` (a correctly
/// rounded parse gives `0x3D4CCCCD`), `0.001` `0x3A83126E`, `3.14159` `0x40490FCE`, `1e10` 1,410,065,408, `1e12`
/// −727,379,968, `1e32` 0, `1e-32` +∞ and `0e-32` the default NaN.
///
/// The arithmetic, as §4.3 "Details of the arithmetic" states it from the exe:
/// - 10.0 and 0.1 are **double** constants used as double operands (EX21): each scale step rounds the exact
///   product scale × 0.1 once. With `0.1f`, `0.05` would give `0x3D4CCCCE`.
/// - An integer digit rounds **twice**, value × 10 and then + digit (EX22), so `33573415` gives 33,573,412.0 (one
///   rounding would give 33,573,416.0). A fraction digit rounds three times: scale × 0.1, digit × scale, + value.
/// - After the mantissa, whatever it consumed, an `e` / `E` starts the exponent (EX23). It takes one optional `+`
///   or `-`, then decimal digits (none: exponent 0). So `e5` and `.e5` give 0, `-e5` −0.0, `2.5e+3` 2500, `1e` 1,
///   and `e-32` the default NaN (`0xFFC00000` as a Float32), as `0e-32` does: 10³² wraps to 0, 1/0 is +∞, and
///   0 × ∞ is invalid.
/// - The sign is applied last (`fchs` after the exponent step, EX36), so it flips a NaN's sign too: `-e-32` gives
///   `0x7FC00000` and `-1e-32` gives −∞.
pub fn parse_float(s: &[u8]) -> f64 {
    let digit = |i: usize| s.get(i).filter(|c| c.is_ascii_digit()).map(|&c| f64::from(c - b'0'));
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
    let mut v = 0.0;
    while let Some(d) = digit(i) {
        v = x87::add(x87::mul(v, 10.0), d);
        i += 1;
    }
    if s.get(i) == Some(&b'.') {
        i += 1;
        let mut scale = 1.0;
        while let Some(d) = digit(i) {
            scale = x87::mul(scale, 0.1);
            v = x87::add(v, x87::mul(d, scale));
            i += 1;
        }
    }
    if matches!(s.get(i), Some(b'e' | b'E')) {
        i += 1;
        let exp_neg = match s.get(i) {
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
        let mut n: u32 = 0;
        while let Some(&c) = s.get(i).filter(|c| c.is_ascii_digit()) {
            n = n.wrapping_mul(10).wrapping_add(u32::from(c - b'0'));
            i += 1;
        }
        let n = if exp_neg { (n as i32).wrapping_neg() } else { n as i32 };
        // 10^|n| mod 2^32 (0 from n = 32 on), loaded as a signed 32-bit integer.
        let power = f64::from(10u32.wrapping_pow(n.unsigned_abs()) as i32);
        // EX35: a positive exponent is one multiply; a negative one rounds the reciprocal 1/10ⁿ first (`fld1` /
        // `fdivrp`), then multiplies it in (`fmul`): two roundings, not one divide.
        v = if n >= 0 {
            x87::mul(v, power)
        } else {
            x87::mul(v, x87::div(1.0, power))
        };
        // The only NaN this parser makes is 0 with a negative exponent whose 10ⁿ wrapped to 0: 1/0 is +∞ and 0 × ∞
        // is invalid, the x87 default NaN (EX23).
        if v.is_nan() {
            v = x87::default_nan();
        }
    }
    if neg {
        -v
    } else {
        v
    }
}

/// The float formatter of the text round trip (`RZFtoaEnglishA`, §5.1 "Floats as text"; EX8 as corrected by EX29):
/// **fixed-point, never an exponent**. `-` for a negative value, the integer digits in full (`0` below 1), then,
/// when a non-zero fraction digit exists, `.`, the fraction's leading zeros and the digits up to the last non-zero
/// one. The digits are those of `_fcvt(x, 64)` ([`fcvt64`]): **17 significant digits, rounded half-up**, padded with
/// zeros, so a value never prints more than 17 non-zero digits and an integer part longer than 17 digits ends in
/// zeros. The printer's own limit of 24 digits after the fraction's leading zeros is kept, but it never takes effect
/// (EX29). A text longer than 63 characters is **cut** to 63 (a 64-byte buffer; the callers ignore the printer's
/// false), whatever the type (EX29). When the cut falls among the significant fraction digits (the 65th character
/// is a digit after the fraction's leading zeros), trailing zeros are **stripped** from the 63 characters kept; a
/// cut anywhere else, and a text of exactly 64 characters, is not stripped (EX34).
///
/// - Zero, −0.0 included, prints `0`: the runtime drops a zero's sign (EX24). A tiny negative value whose digits
///   all round away keeps its sign: −4e-65 prints `-0` (EX34).
/// - NaN and the infinities print as the C runtime prints them: `1.#INF`, `-1.#INF`, `1.#QNAN` / `-1.#QNAN`, and
///   `-1.#IND` for the default NaN. The value is loaded onto the x87 stack first, which quiets a signalling NaN and
///   keeps its sign and payload, so a Float64 sNaN prints `1.#QNAN` too, never `1.#SNAN` (EX28). They parse back as
///   ±1.
///
/// Examples (§5.1): 2.75 gives `2.75`; Float32 0.1 `0.10000000149011612`; Float32 0.001 `0.0010000000474974513`;
/// Float32 3.14159 `3.1415901184082031`; Float32 1e30 `1000000015047466200000000000000`; Float32 2⁻²⁵ (a tie at the
/// 18th digit) `0.000000029802322387695313`; Float32 `0x80000001` `-0.`, 44 zeros and `1401298464324817` (63
/// characters, the 17th digit cut); Float64 1.0000000000000002e-47 `0.`, 46 zeros and `1` (cut, then stripped).
fn format_float(x: f64) -> String {
    if x.is_nan() {
        // EX28: the x87 load quiets a signalling NaN (the quiet bit set, sign and payload kept).
        let bits = x.to_bits() | (1 << 51);
        return if bits == 0xFFF8_0000_0000_0000 {
            "-1.#IND".into()
        } else if bits >> 63 != 0 {
            "-1.#QNAN".into()
        } else {
            "1.#QNAN".into()
        };
    }
    if x.is_infinite() {
        return if x < 0.0 { "-1.#INF" } else { "1.#INF" }.into();
    }
    if x == 0.0 {
        // EX24: `$I10_OUTPUT` resets a zero's sign, so −0.0 prints `0` as well.
        return "0".into();
    }
    let (digits, decpt) = fcvt64(x.abs());
    let mut out = String::new();
    if x < 0.0 {
        out.push('-');
    }
    // The digits stand for 0.d₁d₂… × 10^decpt, and there are decpt + 64 of them (64 places after the point).
    let fraction: Vec<u8> = if decpt > 0 {
        let int = decpt as usize;
        out.push_str(&String::from_utf8_lossy(&digits[..int]));
        digits[int..].to_vec()
    } else {
        out.push('0');
        let mut f = vec![b'0'; decpt.unsigned_abs() as usize];
        f.extend_from_slice(&digits);
        f
    };
    let lead = fraction.iter().take_while(|&&b| b == b'0').count();
    // Where the significant fraction digits start in `out` (none: past the end).
    let mut significant = usize::MAX;
    if lead < fraction.len() {
        let rest = &fraction[lead..];
        let kept = &rest[..rest.iter().rposition(|&b| b != b'0').map_or(0, |i| i + 1)];
        out.push('.');
        out.push_str(&String::from_utf8_lossy(&fraction[..lead]));
        significant = out.len();
        // At most 24 digits after the leading zeros, cut (never reached: at most 17 digits are non-zero, EX29).
        let shown = if kept.len() <= 24 { kept } else { &rest[..24] };
        out.push_str(&String::from_utf8_lossy(shown));
    }
    // EX34: a significant digit still to be written when 64 characters are there (the 65th character) means the
    // cut falls among the significant fraction digits, and the 63 characters kept lose their trailing zeros.
    let strip = out.len() > 64 && significant <= 64;
    out.truncate(63);
    if strip {
        // [literal reading] Only `0` characters are stripped, so a cut whose 63 characters hold no significant
        // digit (the first one at the 64th or 65th character) leaves `0.` (§5.1 does not say whether the `.` goes).
        let len = out.trim_end_matches('0').len();
        out.truncate(len);
    }
    out
}

/// `_fcvt(x, 64)` for a finite `x > 0` (§5.1, EX29): the digits and `decpt` (x = 0.d₁d₂… × 10^decpt).
///
/// 1. The runtime's decimal conversion (`$I10_OUTPUT`, asked for 17 digits by `_fltout2`) gives the first **17
///    significant digits, rounded half-up**: from the exact value, a tail of half an ulp or more rounds up, a tie
///    included (away from zero).
/// 2. `_fptostr` then writes `decpt + 64` digits (64 places after the point): the 17 digits, padded with zeros.
///    When that is fewer than 17 (decpt ≤ −48, a 17-digit value below 10⁻⁴⁸; from 10⁻⁴⁸ up to 10⁻⁴⁷ all 17 still
///    fit) the kept digits are rounded **half-up a second time** on the next one, a carry running through 9s
///    (EX34). At decpt = −64 no digit is kept and the first one rounds; below that nothing is kept and nothing
///    rounds. So 2e-49 prints `0.`, 48 zeros and `2`, and 4e-65 prints `0`. A Float32 never gets there.
fn fcvt64(x: f64) -> (Vec<u8>, i32) {
    /// Adds one at the last digit; true when it carried out of the first (all nines), leaving all zeros.
    fn round_up(d: &mut [u8]) -> bool {
        for c in d.iter_mut().rev() {
            if *c == b'9' {
                *c = b'0';
            } else {
                *c += 1;
                return false;
            }
        }
        true
    }
    // The exact decimal value: an f64 has at most 767 significant digits, so 801 are exact (zeros at the end).
    let sci = format!("{x:.800e}");
    let (mant, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let mut decpt = exp.parse::<i32>().unwrap_or(0) + 1;
    let exact: Vec<u8> = mant.bytes().filter(u8::is_ascii_digit).collect();
    let mut sig = exact[..17].to_vec();
    if exact[17] >= b'5' && round_up(&mut sig) {
        sig[0] = b'1';
        decpt += 1;
    }
    let Ok(n) = usize::try_from(decpt + 64) else {
        return (Vec::new(), decpt);
    };
    let mut out: Vec<u8> = (0..n).map(|i| sig.get(i).copied().unwrap_or(b'0')).collect();
    if n < sig.len() && sig[n] >= b'5' && round_up(&mut out) {
        out.insert(0, b'1');
        decpt += 1;
    }
    (out, decpt)
}

/// [`format_float`] of a Float32, widened to double first (§5.1). The x87 load quiets a signalling NaN.
fn format_f32(x: f32) -> String {
    if x.is_nan() {
        let bits = x.to_bits();
        let wide = u64::from(bits >> 31) << 63 | 0x7FF8_0000_0000_0000 | u64::from(bits & 0x3F_FFFF) << 29;
        return format_float(f64::from_bits(wide));
    }
    format_float(f64::from(x))
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

/// The text reader trims spaces and tabs (§4.3, `0x0067E0B0`).
fn is_blank(b: u8) -> bool {
    matches!(b, b' ' | b'\t')
}

fn trim_blank_start(s: &[u8]) -> &[u8] {
    let n = s.iter().take_while(|&&b| is_blank(b)).count();
    &s[n..]
}

fn trim_blank(s: &[u8]) -> &[u8] {
    let s = trim_blank_start(s);
    let n = s.iter().rev().take_while(|&&b| is_blank(b)).count();
    &s[..s.len() - n]
}

/// Rule N (§4.3) for number text: hexadecimal when it begins with `0x` or `0X`, or when a letter `a`–`f` / `A`–`F`
/// appears before the first character that is not a hex digit; decimal otherwise. The base is chosen on the raw
/// text (`RZConvertToUint32A`): a leading space stops the scan with base 10. The conversion is C `strtoul` in that
/// base (32 bits): it skips leading white space, stops at the first character that is not a digit, a leading `-`
/// negates modulo 2³², and a value above `0xFFFFFFFF` gives `0xFFFFFFFF`, with or without a `-`. So `16` = 16,
/// `1A` = 26, `0x10` = 16, `-1` = `0xFFFFFFFF`, `0x00000010:{"Exemplar Type"}` = 16, ` 16` = 16, ` 1A` = 1,
/// ` 0x10` = 0 and `4294967296` = `-4294967296` = `0xFFFFFFFF`.
pub use fc4_core::number_text::rule_n;

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
    /// Every `name=value` line has its name and value trimmed of spaces and tabs.
    ///
    /// 1. `ParentCohort=Key:{group,instance,type}`: the name is compared case-insensitively, the value must begin
    ///    `Key:{` and end `}` (case-sensitive). The fields are not counted (EX10): the group is read with [`rule_n`]
    ///    from the text after the `{`, the instance and the type from the text after each next `,`; when no `,` is
    ///    left, that field and the later ones read 0, and fields after the third are ignored. Nothing inside the
    ///    braces is trimmed, so a space before a `0x` field makes it 0.
    /// 2. `PropCount=<number>` (name case-insensitive, rule N).
    /// 3. Exactly `PropCount` property lines `ID:{"name"}=Type:reps:{values}`. The ID is the text left of the
    ///    **first** `=`, read with rule N (so a name containing `=` breaks the line *\[quirk\]*). The type is the
    ///    value's text before the first `:`, compared exactly (EX11: `Uint32 :0:{1}` fails); `reps` is read with
    ///    rule N from the text right after it, **not trimmed** (` 0x3` gives 0, a scalar); the `{` must follow the
    ///    second `:` directly. Values are separated by commas; an **array's** values have their leading spaces and
    ///    tabs trimmed, a scalar's value does not (EX13: `Uint32:0:{ 0x10}` gives 0, `Uint32:1:{ 0x10}` gives {16}).
    ///    Integers up to 32 bits: rule N, truncated to the type's width; `Uint64` / `Sint64`: hexadecimal, at most
    ///    16 digits, no sign (`RZAtoUint64HexA`); `Float32`: the game's parser ([`parse_float`]); `Bool`: true when the
    ///    lower-cased value, leading spaces trimmed, begins with `true`; `String`: up to the first `}`, trimmed, one
    ///    `"` removed at each end, `reps` ignored. The reader takes `reps` values (one for `reps` 0): missing values
    ///    read as 0 and extra ones are ignored, with no cap (EX12); *\[choice\]* above 16 MiB of values
    ///    ([`MAX_TEXT_VALUE_BYTES`]) the read fails, where the original crashes.
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
        ex.properties.reserve(count as usize);
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
            ex.set(parse_text_property(line).map_err(err(n))?);
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
    /// hexadecimal, Float32 values as text the game's parser reads back to the same bits. Fails for what the text
    /// form cannot express (§4.3): Empty, Float64, Char, UnicodeChar, Void and resource-key values, a single RZChar,
    /// an empty array (`reps` 0 reads back as a scalar), a string containing `}` or a line break, a non-finite float,
    /// and a Float32 that the game's 24-bit parser cannot reproduce from any text tried (see `float32_text`).
    ///
    /// *\[choice\]* (§4.3 "Writing text", EX32): the original's writer never fails and is lossy. It prints each
    /// Float32 with `%f` and trims the trailing fraction zeros, so 3.14159 is written `3.14159` and reads back 2 ulps
    /// low, and 1e-7 is written `0`. The reader accepts any text, so this writer emits text that reads back exactly
    /// and refuses a value it cannot reproduce (`Unsupported`) instead of copying the loss; a caller can write the
    /// binary form ([`Exemplar::to_binary`]), which the reader also accepts.
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
                Values::Float32(v) => {
                    let texts: Option<Vec<String>> = v.iter().map(|&x| float32_text(x)).collect();
                    let texts =
                        texts.ok_or_else(|| unsupported("a Float32 value the game's text parser cannot reproduce"))?;
                    s.extend_from_slice(texts.join(",").as_bytes());
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
    // Package records commonly arrive in ID order: avoid a search for every
    // append while retaining last-wins replacement for duplicates.
    if !matches!(list.last(), Some(last) if last.id >= p.id) {
        list.push(p);
        return;
    }
    match list.binary_search_by_key(&p.id, |q| q.id) {
        Ok(i) => list[i] = p,
        Err(i) => list.insert(i, p),
    }
}

/// *\[choice\]* (§4.3, EX12): the most value bytes (`reps` × element size) one text property may ask for. The
/// original has no cap: it allocates `reps` × element size bytes, computed in 32 bits and unchecked, so a huge `reps`
/// crashes it. FreeCity4 fails the read above this.
pub const MAX_TEXT_VALUE_BYTES: u64 = 16 << 20;

/// A `name=value` line (§4.3, `0x0067E0B0`): the text left of the first `=` and the rest, both trimmed of spaces and
/// tabs.
fn name_value(line: &[u8]) -> Option<(&[u8], &[u8])> {
    let eq = line.iter().position(|&b| b == b'=')?;
    Some((trim_blank(&line[..eq]), trim_blank(&line[eq + 1..])))
}

/// `ParentCohort=Key:{group,instance,type}` (§4.3, `0x0067DFD0`).
fn parse_parent_line(line: &[u8]) -> std::result::Result<Tgi, String> {
    let (name, value) = name_value(line).ok_or("expected ParentCohort=Key:{...}")?;
    if !name.eq_ignore_ascii_case(b"ParentCohort") {
        return Err(format!("expected ParentCohort, found {:?}", decode_text(name)));
    }
    if !(value.starts_with(b"Key:{") && value.ends_with(b"}")) {
        return Err(format!(
            "ParentCohort value {:?} is not Key:{{...}}",
            decode_text(value)
        ));
    }
    // The fields are not counted (EX10): the text up to the '{' is dropped and the group read with rule N, then the
    // text up to the next ',' for the instance, then the same for the type. With no ',' left, the rest is dropped and
    // that field and the later ones read 0; fields after the third are ignored. Nothing is trimmed, so a space after
    // '{' or ',' turns a 0x field into 0 (rule N then chooses base 10).
    let mut rest = Some(&value[5..]);
    let mut field = || {
        let text = rest?;
        rest = text.iter().position(|&b| b == b',').map(|i| &text[i + 1..]);
        Some(rule_n(text))
    };
    let group = field().unwrap_or(0);
    let instance = field().unwrap_or(0);
    let type_id = field().unwrap_or(0);
    // Text keys are written group, instance, type (§4.3).
    Ok(Tgi::new(type_id, group, instance))
}

/// `PropCount=<number>` (§4.3, `0x0067E250`): name case-insensitive, the value read with rule N.
fn parse_prop_count(line: &[u8]) -> std::result::Result<u32, String> {
    let (name, value) = name_value(line).ok_or("expected PropCount=<number>")?;
    if !name.eq_ignore_ascii_case(b"PropCount") {
        return Err(format!("expected PropCount, found {:?}", decode_text(name)));
    }
    Ok(rule_n(value))
}

/// `0x00000010:{"Exemplar Type"}=Uint32:0:{0x00000002}` (§4.3, `0x0067E780`).
fn parse_text_property(line: &[u8]) -> std::result::Result<Property, String> {
    // The ID is the text left of the first '=' (a '=' inside the name breaks the line [quirk]).
    let (id_text, value) = name_value(line).ok_or("missing '='")?;
    let id = rule_n(id_text);
    // The type is the text before the first ':', compared exactly: nothing is trimmed around the ':' (EX11).
    let colon = value.iter().position(|&b| b == b':').ok_or("missing the reps field")?;
    let ty_name = &value[..colon];
    let ty = ValueType::from_text_name(ty_name).ok_or_else(|| {
        format!(
            "unknown value type {:?} (names are case-sensitive and not trimmed)",
            decode_text(ty_name)
        )
    })?;
    // reps is the text right after the first ':', not trimmed (EX11); rule N stops at the next ':'.
    let after = &value[colon + 1..];
    let colon = after.iter().position(|&b| b == b':').ok_or("missing the values")?;
    let reps_text = &after[..colon];
    // The '{' must follow the second ':' directly.
    let open = after[colon + 1..]
        .strip_prefix(b"{")
        .ok_or("the values must start with '{' right after the second ':'")?;
    if ty == ValueType::String {
        // Everything up to the first '}', trimmed of spaces and tabs at both ends (`cRZString::Trim`, EX27), then one
        // leading and one trailing '"' removed, so white space inside the quotes stays; reps ignored. Only a string
        // is cut at '}' (EX13).
        let end = open.iter().position(|&b| b == b'}').unwrap_or(open.len());
        let s = trim_blank(&open[..end]);
        let s = s.strip_prefix(b"\"").unwrap_or(s);
        let s = s.strip_suffix(b"\"").unwrap_or(s);
        return Ok(Property::array(id, Values::String(s.to_vec())));
    }
    let reps = rule_n(reps_text);
    // No cap and no length check (EX12): the reader takes `reps` values, missing ones read as 0 and extra ones are
    // ignored. [choice] above 16 MiB of values the read fails (the original crashes).
    let bytes = u64::from(reps) * ty.elem_size() as u64;
    if bytes > MAX_TEXT_VALUE_BYTES {
        return Err(format!(
            "reps {reps} asks for {bytes} bytes of values, more than {MAX_TEXT_VALUE_BYTES}"
        ));
    }
    let array = reps != 0;
    let n = (reps as usize).max(1);
    // Values are separated by commas; an array's values have their leading spaces and tabs trimmed, a scalar's
    // value does not (which matters for rule N, EX13). Each is read as a prefix, so the closing '}' after the last
    // value ends it like any other character that cannot continue a number.
    let mut tokens = open.split(|&b| b == b',');
    let mut next = || {
        let w = tokens.next().unwrap_or(&[]);
        if array {
            trim_blank_start(w)
        } else {
            w
        }
    };
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
        // Always hexadecimal, no sign: `10` gives 16 and `-1` gives 0.
        ValueType::Uint64 => each!(Uint64, atou64_hex),
        ValueType::Sint64 => each!(Sint64, |w| atou64_hex(w) as i64),
        // The game's parser at 24-bit precision, stored as Float32 (EX14; a NaN keeps its sign, EX23).
        ValueType::Float32 => each!(Float32, |w| x87::store_f32(parse_float(w))),
        // Leading spaces and tabs trimmed (`cRZString::LTrim`, the same set as for array values, EX26), lower-cased:
        // true exactly when it begins with "true" (`1` gives false, `\ttrue` true).
        ValueType::Bool => each!(Bool, |w: &[u8]| {
            let w = trim_blank_start(w);
            w.len() >= 4 && w[..4].eq_ignore_ascii_case(b"true")
        }),
        _ => return Err(format!("{} has no text form", ty.name())),
    };
    Ok(Property {
        id,
        is_array: array,
        values,
    })
}

/// A Float32 as text that the game's parser ([`parse_float`]) reads back to the same bits, for the text writer:
/// the shortest decimal forms first, then an integer mantissa below 2²⁴ (whose digits add up exactly) with a
/// power of ten up to 10⁹ (one rounded multiply or divide). `None` when no such text is found, or for a non-finite
/// value: the text form cannot express it. *\[choice\]* EX32: the original writes `%f`, trimmed, and loses bits.
fn float32_text(v: f32) -> Option<String> {
    if !v.is_finite() {
        return None;
    }
    let x = f64::from(v);
    let mut candidates = vec![format!("{v}"), format!("{v:?}"), format!("{x:.0}")];
    for n in 1..=9 {
        let p = 10f64.powi(n);
        for (m, e) in [((x * p).round(), -n), ((x / p).round(), n)] {
            if m.abs() < 16_777_216.0 {
                candidates.push(format!("{m}e{e}"));
            }
        }
    }
    candidates
        .into_iter()
        .find(|s| x87::store_f32(parse_float(s.as_bytes())).to_bits() == v.to_bits())
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

    /// `GetProperty(id, uint32&)` (holder slot 8, §5.1): `None` (false, output unchanged) when the property is
    /// missing; see [`Property::get_u32`].
    pub fn get_u32(&self, id: u32) -> Option<u32> {
        self.get(id)?.get_u32()
    }

    /// `GetProperty(id, cIGZString&)` (holder slot 7, §5.1) for callers that check its result: the text of a string
    /// property; `None` when the property is missing or is not a string (see [`Property::get_string`]).
    pub fn get_string(&self, id: u32) -> Option<String> {
        self.get(id)?.get_string()
    }

    /// `GetProperty(id, cIGZString&)` (holder slot 7, §5.1) exactly: false and `out` unchanged when the property is
    /// missing; otherwise as [`Property::get_string_into`] (the text is written for every type, true only for a
    /// string).
    pub fn get_string_into(&self, id: u32, out: &mut Vec<u8>) -> bool {
        self.get(id).is_some_and(|p| p.get_string_into(out))
    }

    /// `GetProperty(id, void* buffer, uint32& length)` (holder slot 5, §5.2): false and both unchanged when the
    /// property is missing; otherwise as [`Property::get_void`] (a void array only).
    pub fn get_void(&self, id: u32, buffer: &mut [u8], length: &mut u32) -> bool {
        self.get(id).is_some_and(|p| p.get_void(buffer, length))
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
    /// The latest package holding this key has a record that does not read: the key does not load, whatever an
    /// earlier package held (§3: only the winning segment is read; EX31).
    Unreadable(Tgi),
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
            LoadError::Unreadable(k) => write!(f, "{k}: the latest package's record does not read"),
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
/// makes later packages override earlier ones (each package is registered at the front, §3.1).
///
/// A kind-1 property's resource (§4.6, EX15) loads when it is an exemplar or cohort record that loads, or a key of
/// another type that resolves: one the caller registered as held by a resource cache ([`ExemplarStore::add_resource`]),
/// or one a loaded package holds whose type has a factory ([`ExemplarStore::add_factory`]) that read it
/// successfully. Presence in a package index alone is not enough.
///
/// # When records are read (§3, EX31)
///
/// The original reads lazily: a factory runs at the first request for a key that no cache holds, on the record of
/// the segment that wins at that moment, and a failure is read again at the next request. *\[choice\]* This store
/// reads each record **eagerly**, when its package is loaded, which §3 allows when three conditions hold. They hold
/// here as follows:
///
/// 1. *A later package's record replaces an earlier read of the same key.* Each package's record replaces the one
///    before it, also when it fails to read: the key then does not load ([`LoadError::Unreadable`]), as the
///    original reads only the winning segment and never falls back to an earlier one. Inside one package the first
///    index entry with a key is the one read (§2.3). The store keeps no cache of looked-up records, so a lookup
///    always sees every package loaded so far; the original keeps a looked-up record (§3 "Cache", EX37), so the two
///    agree when every package is loaded before the first lookup, as at the game's startup (§3.1). *\[choice\]* No
///    cache is kept (EX37): a store that registered packages after lookups would have to keep the first record of
///    each key looked up.
/// 2. *A record that failed is read again later.* Parsing a record depends on its bytes only, so only a later
///    package's record can change it, and that one is read. A failure that depends on other records (a parent
///    cohort, a kind-1 resource, the depth limit) is decided at each [`ExemplarStore::load`], never remembered.
/// 3. *A factory registered later applies to earlier packages.* The store does not keep the packages, so it cannot
///    read their records again: [`ExemplarStore::add_factory`] refuses a type that a loaded package already holds,
///    and logs the refusal as an error ([`ExemplarStore::errors`], §3 "A factory added late", EX38).
#[derive(Default)]
pub struct ExemplarStore {
    map: HashMap<Tgi, Exemplar>,
    /// Exemplar and cohort keys whose winning record (the latest package's) failed to read (EX31 condition 1).
    unreadable: HashSet<Tgi>,
    /// Keys of other resources that a resource cache holds ([`ExemplarStore::add_resource`], §4.6).
    resources: HashSet<Tgi>,
    /// Keys of other resources read by a registered factory: whether the winning record's read succeeded (§4.6).
    factory_reads: HashMap<Tgi, bool>,
    /// Factories for resource types other than exemplars and cohorts, by type ID: each says whether its read of a
    /// record succeeds.
    factories: HashMap<u32, Factory>,
    /// The types other than exemplars and cohorts that the loaded packages hold (EX31 condition 3).
    seen_types: HashSet<u32>,
    /// The store's error log: one line per refused factory (§3 "A factory added late", EX38).
    errors: Vec<String>,
}

/// A registered factory's read of one record: true when it succeeds ([`ExemplarStore::add_factory`]).
type Factory = Box<dyn Fn(&[u8]) -> bool>;

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

    /// Adds or replaces the record under `tgi` (it then replaces an unreadable one as well).
    pub fn insert(&mut self, tgi: Tgi, ex: Exemplar) -> Option<Exemplar> {
        self.unreadable.remove(&tgi);
        self.map.insert(tgi, ex)
    }

    /// The record under `tgi`, as parsed (own properties only; it may fail to load).
    pub fn get(&self, tgi: &Tgi) -> Option<&Exemplar> {
        self.map.get(tgi)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Tgi, &Exemplar)> {
        self.map.iter()
    }

    /// Registers a resource that a resource cache already holds (§4.6, `0x009732EB`), so a kind-1 property naming it
    /// loads whatever its type.
    pub fn add_resource(&mut self, tgi: Tgi) -> bool {
        self.resources.insert(tgi)
    }

    /// Registers a factory for resources of `type_id` (§4.6, `0x00972EAE`): from then on,
    /// [`ExemplarStore::load_package`] runs `read` on every entry of that type, and an entry whose read succeeds
    /// loads as a kind-1 resource. The exemplar and cohort types have the store's own factories and cannot be
    /// registered (returns false). This crate registers no other factory: the set the original registers was not
    /// enumerated (§4.6 *\[inferred\]*). Registering a type again replaces its factory.
    ///
    /// In the original a factory registered late applies at once, to the records of segments registered before it
    /// too (§3, EX31). This store reads eagerly and does not keep the packages, so *\[choice\]* it refuses (returns
    /// false, nothing registered) a type that an already loaded package holds: every factory the store accepts has
    /// seen every record of its type (§3 condition 3). Register factories before loading packages.
    ///
    /// EX38: the original never refuses a late factory, so the refusal stands only while every factory is added
    /// before the first package; a refused factory is logged as an error ([`ExemplarStore::errors`]), never ignored
    /// silently. *\[literal reading\]* A factory for the exemplar or cohort type, which the store's own factories
    /// serve, is refused and logged too.
    pub fn add_factory(&mut self, type_id: u32, read: impl Fn(&[u8]) -> bool + 'static) -> bool {
        if Kind::from_type_id(type_id).is_some() {
            self.errors.push(format!(
                "error: factory for type {type_id:08X} refused: the store's own exemplar and cohort factories serve it"
            ));
            return false;
        }
        if self.seen_types.contains(&type_id) {
            self.errors.push(format!(
                "error: factory for type {type_id:08X} refused: added after a loaded package holding the type (EX38)"
            ));
            return false;
        }
        self.factories.insert(type_id, Box::new(read));
        true
    }

    /// The store's error log, oldest first: one line per factory that [`ExemplarStore::add_factory`] refused (EX38).
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Parses every exemplar and cohort entry of a package into the store, with the factory's type check
    /// ([`Exemplar::parse_record`]), and runs the registered factory ([`ExemplarStore::add_factory`]) on every entry
    /// of another type that has one. Other entries are ignored: an index entry alone does not make a kind-1
    /// resource load (EX15).
    ///
    /// The package is registered at the front (§3.1): its records replace those of earlier packages, and a record
    /// that fails to read replaces them too, so its key does not load (§3, EX31 condition 1; the failure is in
    /// the report). Inside the package only the first index entry with a key is read (§2.3).
    pub fn load_package<S: Source>(&mut self, pkg: &Package<S>) -> LoadReport {
        let mut report = LoadReport::default();
        for e in pkg.entries() {
            // §2.3: a later duplicate of a key in the same package is never read.
            if !pkg.find(&e.tgi).is_some_and(|first| std::ptr::eq(first, e)) {
                continue;
            }
            let ty = e.tgi.type_id;
            if Kind::from_type_id(ty).is_none() {
                self.seen_types.insert(ty);
                if let Some(read) = self.factories.get(&ty) {
                    let ok = pkg.read(e).is_ok_and(|d| read(&d));
                    self.factory_reads.insert(e.tgi, ok);
                }
                continue;
            }
            match pkg.read(e).and_then(|d| Exemplar::parse_record(ty, &d)) {
                Ok(ex) => {
                    if ty == TYPE_EXEMPLAR {
                        report.exemplars += 1;
                    } else {
                        report.cohorts += 1;
                    }
                    self.insert(e.tgi, ex);
                }
                Err(err) => {
                    // The winning record does not read: no fallback to an earlier package's record (EX31).
                    self.map.remove(&e.tgi);
                    self.unreadable.insert(e.tgi);
                    report.failures.push((e.tgi, err));
                }
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
        let record = store.map.get(&key).ok_or_else(|| {
            if store.unreadable.contains(&key) {
                LoadError::Unreadable(key)
            } else {
                LoadError::NotFound(key)
            }
        })?;
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
            if store.map.contains_key(&resource) || store.unreadable.contains(&resource) {
                self.load(resource).map_err(fail)?;
            } else if !store.resources.contains(&resource) && store.factory_reads.get(&resource) != Some(&true) {
                return Err(fail(LoadError::NotFound(resource)));
            }
        }
        Ok(Rc::new(loaded))
    }
}

#[cfg(test)]
mod tests;
