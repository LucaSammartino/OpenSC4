// SPDX-License-Identifier: GPL-3.0-or-later
//! Exemplars and cohorts (types [`TYPE_EXEMPLAR`] and [`TYPE_COHORT`]).
//!
//! Binary form:
//!
//! ```text
//! char[8]  "EQZB1###" (exemplar) or "CQZB1###" (cohort)
//! u32 × 3  parent cohort key: type, group, instance (all zero = no parent)
//! u32      property count
//! property × count:
//!   u32    property ID
//!   u16    value type: 0x0100 Uint8, 0x0200 Uint16, 0x0300 Uint32, 0x0700 Sint32, 0x0800 Sint64,
//!          0x0900 Float32, 0x0B00 Bool, 0x0C00 String
//!   u16    key type: 0x0000 single value, 0x0080 array
//!   u8     unused
//!   single: one value
//!   array:  u32 rep count, then that many values (String: the rep count is the length in bytes)
//! ```
//!
//! Text form (`EQZT1###` / `CQZT1###`), one item per line:
//!
//! ```text
//! EQZT1###
//! ParentCohort=Key:{0xa8fbd372,0xaa1b5001,0x05342861}
//! PropCount=0x00000003
//! 0x00000010:{"Exemplar Type"}=Uint32:0:{0x00000010}
//! 0x00000020:{"Exemplar Name"}=String:0:{"Some name"}
//! 0x88edc790:{"LotConfigPropertySize"}=Uint8:2:{0x06,0x06}
//! ```
//!
//! `ID:{"name"}=Type:reps:{values}`: `reps` 0 means a single value, otherwise an array; values are
//! comma-separated (hex or decimal integers, decimal floats, `True`/`False`, a quoted string); `{}` is no value.
//!
//! **Key order differs between the forms.** The binary parent key is type, group, instance (as documented).
//! The text `ParentCohort` key is **group, instance, type**: in both text exemplars the shipped game has
//! (`Sku_Data/Sku1_US/Plugin_002_TheLivingMall.dat`) the last word is the cohort type `0x05342861`, and read
//! in that order every parent resolves to a cohort in the game data. Resource keys inside property values
//! (for example `Uint32:3:{0x2026960b,...}`) stay in type, group, instance order.
//!
//! Choices made where the documentation is silent *\[choice\]*:
//! - The value-type table also accepts the codes of foundations §9.2's tag numbering that the exemplar
//!   documentation does not list (`0x0400` Uint64, `0x0500` Sint8, `0x0600` Sint16, `0x0A00` Float64).
//! - Bool values: any non-zero byte is true.
//! - Strings are kept as bytes ([`Values::String`]); [`Property::as_str`] decodes them as UTF-8 and
//!   [`Property::string_lossy`] falls back to Latin-1.
//! - Duplicate property IDs: lookups return the last one (as foundations §10.5 does for property holders).
//! - Bytes after the last property are ignored; the declared count is authoritative in binary form. In text
//!   form every property line is read and a `PropCount` mismatch is not an error.
//! - Cohort inheritance: a property missing from an exemplar is looked up in its parent cohort, then in that
//!   cohort's parent, and so on (at most [`MAX_PARENT_DEPTH`] levels; a cycle ends the walk).

use std::collections::HashMap;
use std::fmt::{self, Write as _};

use crate::dbpf::{Package, Source};
use crate::error::{corrupt, Error, Result};
use crate::reader::Reader;
use crate::tgi::Tgi;
pub use crate::tgi::{TYPE_COHORT, TYPE_EXEMPLAR};

/// Deepest parent chain the resolver follows.
pub const MAX_PARENT_DEPTH: usize = 32;

/// Exemplar or cohort.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Exemplar,
    Cohort,
}

/// Binary or text encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Binary,
    Text,
}

/// A property value type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueType {
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Sint8,
    Sint16,
    Sint32,
    Sint64,
    Float32,
    Float64,
    Bool,
    String,
}

impl ValueType {
    const ALL: [ValueType; 12] = [
        ValueType::Uint8,
        ValueType::Uint16,
        ValueType::Uint32,
        ValueType::Uint64,
        ValueType::Sint8,
        ValueType::Sint16,
        ValueType::Sint32,
        ValueType::Sint64,
        ValueType::Float32,
        ValueType::Float64,
        ValueType::Bool,
        ValueType::String,
    ];

    /// The binary type code.
    pub fn code(self) -> u16 {
        match self {
            ValueType::Uint8 => 0x0100,
            ValueType::Uint16 => 0x0200,
            ValueType::Uint32 => 0x0300,
            ValueType::Uint64 => 0x0400,
            ValueType::Sint8 => 0x0500,
            ValueType::Sint16 => 0x0600,
            ValueType::Sint32 => 0x0700,
            ValueType::Sint64 => 0x0800,
            ValueType::Float32 => 0x0900,
            ValueType::Float64 => 0x0A00,
            ValueType::Bool => 0x0B00,
            ValueType::String => 0x0C00,
        }
    }

    pub fn from_code(code: u16) -> Option<ValueType> {
        Self::ALL.into_iter().find(|t| t.code() == code)
    }

    /// The name used in text exemplars.
    pub fn name(self) -> &'static str {
        match self {
            ValueType::Uint8 => "Uint8",
            ValueType::Uint16 => "Uint16",
            ValueType::Uint32 => "Uint32",
            ValueType::Uint64 => "Uint64",
            ValueType::Sint8 => "Sint8",
            ValueType::Sint16 => "Sint16",
            ValueType::Sint32 => "Sint32",
            ValueType::Sint64 => "Sint64",
            ValueType::Float32 => "Float32",
            ValueType::Float64 => "Float64",
            ValueType::Bool => "Bool",
            ValueType::String => "String",
        }
    }

    pub fn from_name(name: &str) -> Option<ValueType> {
        Self::ALL
            .into_iter()
            .find(|t| t.name().eq_ignore_ascii_case(name.trim()))
    }

    /// Bytes per element in binary form (1 for String, whose elements are bytes).
    pub fn elem_size(self) -> usize {
        match self {
            ValueType::Uint8 | ValueType::Sint8 | ValueType::Bool | ValueType::String => 1,
            ValueType::Uint16 | ValueType::Sint16 => 2,
            ValueType::Uint32 | ValueType::Sint32 | ValueType::Float32 => 4,
            ValueType::Uint64 | ValueType::Sint64 | ValueType::Float64 => 8,
        }
    }
}

/// The values of one property.
#[derive(Clone, Debug, PartialEq)]
pub enum Values {
    Uint8(Vec<u8>),
    Uint16(Vec<u16>),
    Uint32(Vec<u32>),
    Uint64(Vec<u64>),
    Sint8(Vec<i8>),
    Sint16(Vec<i16>),
    Sint32(Vec<i32>),
    Sint64(Vec<i64>),
    Float32(Vec<f32>),
    Float64(Vec<f64>),
    Bool(Vec<bool>),
    /// The raw string bytes (no terminator).
    String(Vec<u8>),
}

impl Values {
    pub fn value_type(&self) -> ValueType {
        match self {
            Values::Uint8(_) => ValueType::Uint8,
            Values::Uint16(_) => ValueType::Uint16,
            Values::Uint32(_) => ValueType::Uint32,
            Values::Uint64(_) => ValueType::Uint64,
            Values::Sint8(_) => ValueType::Sint8,
            Values::Sint16(_) => ValueType::Sint16,
            Values::Sint32(_) => ValueType::Sint32,
            Values::Sint64(_) => ValueType::Sint64,
            Values::Float32(_) => ValueType::Float32,
            Values::Float64(_) => ValueType::Float64,
            Values::Bool(_) => ValueType::Bool,
            Values::String(_) => ValueType::String,
        }
    }

    /// Element count (string length in bytes for String).
    pub fn len(&self) -> usize {
        match self {
            Values::Uint8(v) => v.len(),
            Values::Uint16(v) => v.len(),
            Values::Uint32(v) => v.len(),
            Values::Uint64(v) => v.len(),
            Values::Sint8(v) => v.len(),
            Values::Sint16(v) => v.len(),
            Values::Sint32(v) => v.len(),
            Values::Sint64(v) => v.len(),
            Values::Float32(v) => v.len(),
            Values::Float64(v) => v.len(),
            Values::Bool(v) => v.len(),
            Values::String(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn read(r: &mut Reader<'_>, ty: ValueType, n: usize) -> Result<Values> {
        r.ensure(n as u64, ty.elem_size() as u64)?;
        fn many<T>(n: usize, mut f: impl FnMut() -> Result<T>) -> Result<Vec<T>> {
            (0..n).map(|_| f()).collect()
        }
        Ok(match ty {
            ValueType::Uint8 => Values::Uint8(r.bytes(n)?.to_vec()),
            ValueType::Sint8 => Values::Sint8(r.bytes(n)?.iter().map(|&b| b as i8).collect()),
            ValueType::Bool => Values::Bool(r.bytes(n)?.iter().map(|&b| b != 0).collect()),
            ValueType::String => Values::String(r.bytes(n)?.to_vec()),
            ValueType::Uint16 => Values::Uint16(many(n, || r.u16())?),
            ValueType::Sint16 => Values::Sint16(many(n, || r.u16().map(|v| v as i16))?),
            ValueType::Uint32 => Values::Uint32(many(n, || r.u32())?),
            ValueType::Sint32 => Values::Sint32(many(n, || r.u32().map(|v| v as i32))?),
            ValueType::Float32 => Values::Float32(many(n, || r.u32().map(f32::from_bits))?),
            ValueType::Uint64 => Values::Uint64(many(n, || r.u64())?),
            ValueType::Sint64 => Values::Sint64(many(n, || r.u64().map(|v| v as i64))?),
            ValueType::Float64 => Values::Float64(many(n, || r.u64().map(f64::from_bits))?),
        })
    }

    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Values::Uint8(v) | Values::String(v) => out.extend_from_slice(v),
            Values::Sint8(v) => out.extend(v.iter().map(|&x| x as u8)),
            Values::Bool(v) => out.extend(v.iter().map(|&x| x as u8)),
            Values::Uint16(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
            Values::Sint16(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
            Values::Uint32(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
            Values::Sint32(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
            Values::Float32(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_bits().to_le_bytes())),
            Values::Uint64(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
            Values::Sint64(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes())),
            Values::Float64(v) => v.iter().for_each(|x| out.extend_from_slice(&x.to_bits().to_le_bytes())),
        }
    }

    /// Element `i` as an integer (integer and Bool types only).
    pub fn int(&self, i: usize) -> Option<i64> {
        match self {
            Values::Uint8(v) => v.get(i).map(|&x| x as i64),
            Values::Uint16(v) => v.get(i).map(|&x| x as i64),
            Values::Uint32(v) => v.get(i).map(|&x| x as i64),
            Values::Uint64(v) => v.get(i).and_then(|&x| i64::try_from(x).ok()),
            Values::Sint8(v) => v.get(i).map(|&x| x as i64),
            Values::Sint16(v) => v.get(i).map(|&x| x as i64),
            Values::Sint32(v) => v.get(i).map(|&x| x as i64),
            Values::Sint64(v) => v.get(i).copied(),
            Values::Bool(v) => v.get(i).map(|&x| x as i64),
            _ => None,
        }
    }

    /// Element `i` as a float (any numeric or Bool type).
    pub fn float(&self, i: usize) -> Option<f64> {
        match self {
            Values::Float32(v) => v.get(i).map(|&x| x as f64),
            Values::Float64(v) => v.get(i).copied(),
            Values::Uint64(v) => v.get(i).map(|&x| x as f64),
            _ => self.int(i).map(|x| x as f64),
        }
    }
}

/// One exemplar property.
#[derive(Clone, Debug, PartialEq)]
pub struct Property {
    pub id: u32,
    /// Stored as an array (binary key type `0x80`, text rep count > 0) rather than a single value.
    pub is_array: bool,
    pub values: Values,
}

impl Property {
    pub fn single(id: u32, values: Values) -> Property {
        Property {
            id,
            is_array: false,
            values,
        }
    }

    pub fn array(id: u32, values: Values) -> Property {
        Property {
            id,
            is_array: true,
            values,
        }
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

    /// First element of a Uint8 property.
    pub fn as_u8(&self) -> Option<u8> {
        self.u8s()?.first().copied()
    }
    /// First element of a Uint16 property.
    pub fn as_u16(&self) -> Option<u16> {
        self.u16s()?.first().copied()
    }
    /// First element of a Uint32 property.
    pub fn as_u32(&self) -> Option<u32> {
        self.u32s()?.first().copied()
    }
    /// First element of a Sint32 property.
    pub fn as_i32(&self) -> Option<i32> {
        self.i32s()?.first().copied()
    }
    /// First element of a Sint64 property.
    pub fn as_i64(&self) -> Option<i64> {
        self.i64s()?.first().copied()
    }
    /// First element of a Float32 property.
    pub fn as_f32(&self) -> Option<f32> {
        self.f32s()?.first().copied()
    }
    /// First element of a Bool property.
    pub fn as_bool(&self) -> Option<bool> {
        self.bools()?.first().copied()
    }
    /// A String property as UTF-8 (None if not a string or not valid UTF-8).
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(self.string_bytes()?).ok()
    }
    /// A String property decoded as UTF-8, or Latin-1 when it is not valid UTF-8.
    pub fn string_lossy(&self) -> Option<String> {
        let b = self.string_bytes()?;
        Some(match std::str::from_utf8(b) {
            Ok(s) => s.to_owned(),
            Err(_) => b.iter().map(|&c| c as char).collect(),
        })
    }
    pub fn string_bytes(&self) -> Option<&[u8]> {
        match &self.values {
            Values::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn u8s(&self) -> Option<&[u8]> {
        match &self.values {
            Values::Uint8(v) => Some(v),
            _ => None,
        }
    }
    pub fn u16s(&self) -> Option<&[u16]> {
        match &self.values {
            Values::Uint16(v) => Some(v),
            _ => None,
        }
    }
    pub fn u32s(&self) -> Option<&[u32]> {
        match &self.values {
            Values::Uint32(v) => Some(v),
            _ => None,
        }
    }
    pub fn i32s(&self) -> Option<&[i32]> {
        match &self.values {
            Values::Sint32(v) => Some(v),
            _ => None,
        }
    }
    pub fn i64s(&self) -> Option<&[i64]> {
        match &self.values {
            Values::Sint64(v) => Some(v),
            _ => None,
        }
    }
    pub fn f32s(&self) -> Option<&[f32]> {
        match &self.values {
            Values::Float32(v) => Some(v),
            _ => None,
        }
    }
    pub fn bools(&self) -> Option<&[bool]> {
        match &self.values {
            Values::Bool(v) => Some(v),
            _ => None,
        }
    }

    /// Element `i` widened to an integer, whatever the integer type (for callers that do not care).
    pub fn int(&self, i: usize) -> Option<i64> {
        self.values.int(i)
    }
    /// Element `i` widened to a float, whatever the numeric type.
    pub fn float(&self, i: usize) -> Option<f64> {
        self.values.float(i)
    }
}

/// A parsed exemplar or cohort.
#[derive(Clone, Debug, PartialEq)]
pub struct Exemplar {
    pub kind: Kind,
    pub encoding: Encoding,
    /// Parent cohort key; all zero = none.
    pub parent: Tgi,
    /// Properties in file order.
    pub properties: Vec<Property>,
}

impl Exemplar {
    pub fn new(kind: Kind) -> Exemplar {
        Exemplar {
            kind,
            encoding: Encoding::Binary,
            parent: Tgi::default(),
            properties: Vec::new(),
        }
    }

    /// True if `data` starts with an exemplar or cohort signature.
    pub fn is_exemplar(data: &[u8]) -> bool {
        data.len() >= 4 && matches!(data[0], b'E' | b'C') && &data[1..3] == b"QZ" && matches!(data[3], b'B' | b'T')
    }

    /// Parses either form.
    pub fn parse(data: &[u8]) -> Result<Exemplar> {
        if data.len() < 8 || !Self::is_exemplar(data) {
            return Err(Error::BadMagic {
                what: "exemplar",
                found: data.iter().take(8).copied().collect(),
            });
        }
        let kind = if data[0] == b'E' { Kind::Exemplar } else { Kind::Cohort };
        if data[4] != b'1' {
            return Err(Error::Unsupported {
                what: "exemplar",
                detail: format!("version {:?}", String::from_utf8_lossy(&data[4..8])),
            });
        }
        if data[3] == b'B' {
            Self::parse_binary(kind, data)
        } else {
            Self::parse_text(kind, data)
        }
    }

    fn parse_binary(kind: Kind, data: &[u8]) -> Result<Exemplar> {
        let mut r = Reader::new(data, "binary exemplar");
        r.bytes(8)?;
        let parent = Tgi::new(r.u32()?, r.u32()?, r.u32()?);
        let count = r.u32()?;
        // Each property takes at least 9 bytes; reject impossible counts before allocating.
        r.ensure(count as u64, 9)?;
        let mut properties = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let at = r.pos();
            let id = r.u32()?;
            let code = r.u16()?;
            let ty = ValueType::from_code(code).ok_or_else(|| {
                corrupt(
                    "binary exemplar",
                    format!("property {id:#010X} at {at}: value type {code:#06X}"),
                )
            })?;
            let key = r.u16()?;
            let _unused = r.u8()?;
            let (is_array, n) = match key {
                0x00 => (false, 1usize),
                0x80 => (true, r.u32()? as usize),
                k => {
                    return Err(corrupt(
                        "binary exemplar",
                        format!("property {id:#010X} at {at}: key type {k:#06X}"),
                    ))
                }
            };
            let values = Values::read(&mut r, ty, n)?;
            properties.push(Property { id, is_array, values });
        }
        Ok(Exemplar {
            kind,
            encoding: Encoding::Binary,
            parent,
            properties,
        })
    }

    fn parse_text(kind: Kind, data: &[u8]) -> Result<Exemplar> {
        // [choice] text exemplars are read as UTF-8, falling back to Latin-1.
        let text: String = match std::str::from_utf8(data) {
            Ok(s) => s.to_owned(),
            Err(_) => data.iter().map(|&b| b as char).collect(),
        };
        let mut ex = Exemplar {
            kind,
            encoding: Encoding::Text,
            parent: Tgi::default(),
            properties: Vec::new(),
        };
        let mut lines = text.lines().enumerate().skip(1);
        while let Some((i, line)) = lines.next() {
            let mut line = line.trim().to_owned();
            if line.is_empty() || line.trim_matches('\0').is_empty() {
                continue;
            }
            // A value list may continue on following lines until its braces balance.
            while !braces_balanced(&line) {
                match lines.next() {
                    Some((_, more)) => {
                        line.push(' ');
                        line.push_str(more.trim());
                    }
                    None => {
                        return Err(Error::Text {
                            line: i + 1,
                            detail: "unterminated value list".into(),
                        })
                    }
                }
            }
            let err = |d: String| Error::Text { line: i + 1, detail: d };
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("parentcohort") {
                let words = brace_body(&line).ok_or_else(|| err("ParentCohort without {..}".into()))?;
                let k: Vec<u32> = split_values(words)
                    .iter()
                    .map(|w| parse_u32(w))
                    .collect::<Option<_>>()
                    .ok_or_else(|| err(format!("bad key {words:?}")))?;
                if k.len() != 3 {
                    return Err(err(format!("ParentCohort has {} words", k.len())));
                }
                // Text keys are written group, instance, type (see the module documentation).
                ex.parent = Tgi::new(k[2], k[0], k[1]);
            } else if lower.starts_with("propcount") {
                // informational only
            } else {
                ex.properties.push(parse_text_property(&line).map_err(err)?);
            }
        }
        Ok(ex)
    }

    /// The last property with this ID (own properties only; see [`ExemplarStore`] for inheritance).
    pub fn get(&self, id: u32) -> Option<&Property> {
        self.properties.iter().rev().find(|p| p.id == id)
    }

    /// Replaces or appends a property.
    pub fn set(&mut self, p: Property) {
        if let Some(slot) = self.properties.iter_mut().find(|q| q.id == p.id) {
            *slot = p;
        } else {
            self.properties.push(p);
        }
    }

    pub fn has_parent(&self) -> bool {
        !self.parent.is_zero()
    }

    /// Binary encoding (`EQZB1###` / `CQZB1###`).
    pub fn to_binary(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(if self.kind == Kind::Exemplar {
            b"EQZB1###"
        } else {
            b"CQZB1###"
        });
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
            out.extend_from_slice(&p.value_type().code().to_le_bytes());
            // Strings are always stored as arrays (their length is the rep count).
            let array = p.is_array || p.value_type() == ValueType::String || p.len() != 1;
            out.extend_from_slice(&(if array { 0x80u16 } else { 0 }).to_le_bytes());
            out.push(0);
            if array {
                out.extend_from_slice(&(p.len() as u32).to_le_bytes());
            }
            p.values.write(&mut out);
        }
        out
    }

    /// Text encoding (`EQZT1###` / `CQZT1###`), CRLF line ends. Property names are left empty.
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        s.push_str(if self.kind == Kind::Exemplar {
            "EQZT1###\r\n"
        } else {
            "CQZT1###\r\n"
        });
        let _ = write!(
            s,
            "ParentCohort=Key:{{0x{:08x},0x{:08x},0x{:08x}}}\r\nPropCount=0x{:08x}\r\n",
            self.parent.group,
            self.parent.instance,
            self.parent.type_id,
            self.properties.len()
        );
        for p in &self.properties {
            let array = p.is_array || p.len() != 1;
            let rep = if p.value_type() == ValueType::String || array {
                p.len()
            } else {
                0
            };
            let _ = write!(
                s,
                "0x{:08x}:{{\"\"}}={}:{}:{{{}}}\r\n",
                p.id,
                p.value_type().name(),
                rep,
                TextValues(&p.values)
            );
        }
        s
    }
}

/// Formats values as in text exemplars.
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
            Values::Uint8(v) => list(f, v, |f, x| write!(f, "0x{x:02x}")),
            Values::Uint16(v) => list(f, v, |f, x| write!(f, "0x{x:04x}")),
            Values::Uint32(v) => list(f, v, |f, x| write!(f, "0x{x:08x}")),
            Values::Uint64(v) => list(f, v, |f, x| write!(f, "0x{x:016x}")),
            Values::Sint8(v) => list(f, v, |f, x| write!(f, "{x}")),
            Values::Sint16(v) => list(f, v, |f, x| write!(f, "{x}")),
            Values::Sint32(v) => list(f, v, |f, x| write!(f, "{x}")),
            Values::Sint64(v) => list(f, v, |f, x| write!(f, "{x}")),
            Values::Float32(v) => list(f, v, |f, x| write!(f, "{x:?}")),
            Values::Float64(v) => list(f, v, |f, x| write!(f, "{x:?}")),
            Values::Bool(v) => list(f, v, |f, x| f.write_str(if *x { "True" } else { "False" })),
            Values::String(v) => {
                f.write_str("\"")?;
                for &b in v {
                    f.write_char(b as char)?;
                }
                f.write_str("\"")
            }
        }
    }
}

fn braces_balanced(s: &str) -> bool {
    let mut depth = 0i32;
    let mut in_str = false;
    for c in s.chars() {
        match c {
            '"' => in_str = !in_str,
            '{' if !in_str => depth += 1,
            '}' if !in_str => depth -= 1,
            _ => {}
        }
    }
    // An odd number of quotes inside a string value must not swallow the following lines.
    (depth <= 0 && !in_str) || (in_str && s.trim_end().ends_with('}'))
}

/// The text between the first `{` and the last `}`.
fn brace_body(s: &str) -> Option<&str> {
    let a = s.find('{')?;
    let b = s.rfind('}')?;
    (b > a).then(|| &s[a + 1..b])
}

fn split_values(s: &str) -> Vec<&str> {
    s.split(',').map(str::trim).filter(|w| !w.is_empty()).collect()
}

fn parse_u64(w: &str) -> Option<u64> {
    let w = w.trim();
    if let Some(h) = w.strip_prefix("0x").or_else(|| w.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).ok()
    } else {
        w.parse().ok()
    }
}

fn parse_u32(w: &str) -> Option<u32> {
    parse_u64(w).and_then(|v| u32::try_from(v).ok())
}

fn parse_i64(w: &str) -> Option<i64> {
    let w = w.trim();
    if let Some(rest) = w.strip_prefix('-') {
        return parse_u64(rest).and_then(|v| i64::try_from(v).ok()).map(|v| -v);
    }
    // hex values are bit patterns: 0xFFFFFFFF in a Sint32 is -1
    parse_u64(w).map(|v| v as i64)
}

fn parse_float(w: &str) -> Option<f64> {
    let w = w.trim();
    if w.starts_with("0x") || w.starts_with("0X") {
        return parse_u64(w).map(|v| v as f64);
    }
    w.trim_end_matches(['f', 'F']).parse().ok()
}

fn parse_bool(w: &str) -> Option<bool> {
    match w.trim().to_ascii_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        other => parse_u64(other).map(|v| v != 0),
    }
}

/// `0x00000010:{"Exemplar Type"}=Uint32:0:{0x00000002}`; the `:{"name"}` part is optional.
fn parse_text_property(line: &str) -> std::result::Result<Property, String> {
    let eq = find_outside_quotes(line, '=').ok_or("missing '='")?;
    let (lhs, rhs) = (&line[..eq], &line[eq + 1..]);
    let id_str = lhs.split(':').next().unwrap_or("").trim();
    let id = parse_u32(id_str).ok_or_else(|| format!("bad property id {id_str:?}"))?;
    let mut parts = rhs.splitn(3, ':');
    let ty_str = parts.next().unwrap_or("");
    let ty = ValueType::from_name(ty_str).ok_or_else(|| format!("unknown value type {ty_str:?}"))?;
    let rep_str = parts.next().ok_or("missing rep count")?.trim();
    let rep = parse_u32(rep_str).ok_or_else(|| format!("bad rep count {rep_str:?}"))? as usize;
    let body = brace_body(parts.next().ok_or("missing values")?).ok_or("values without {..}")?;
    let bad = |w: &str| format!("bad {} value {w:?}", ty.name());
    let words = split_values(body);
    macro_rules! each {
        ($variant:ident, $f:expr) => {
            Values::$variant(
                words
                    .iter()
                    .map(|w| $f(w).ok_or_else(|| bad(w)))
                    .collect::<std::result::Result<_, _>>()?,
            )
        };
    }
    let values = match ty {
        ValueType::String => {
            let t = body.trim();
            let s = t.strip_prefix('"').and_then(|t| t.strip_suffix('"')).unwrap_or(t);
            // [choice] text made only of U+0000..=U+00FF is stored one byte per character, other text as UTF-8.
            let bytes: Vec<u8> = if s.chars().all(|c| (c as u32) < 0x100) {
                s.chars().map(|c| c as u8).collect()
            } else {
                s.as_bytes().to_vec()
            };
            Values::String(bytes)
        }
        ValueType::Uint8 => each!(Uint8, |w: &str| parse_i64(w).map(|v| v as u8)),
        ValueType::Uint16 => each!(Uint16, |w: &str| parse_i64(w).map(|v| v as u16)),
        ValueType::Uint32 => each!(Uint32, |w: &str| parse_i64(w).map(|v| v as u32)),
        ValueType::Uint64 => each!(Uint64, |w: &str| parse_u64(w)),
        ValueType::Sint8 => each!(Sint8, |w: &str| parse_i64(w).map(|v| v as i8)),
        ValueType::Sint16 => each!(Sint16, |w: &str| parse_i64(w).map(|v| v as i16)),
        ValueType::Sint32 => each!(Sint32, |w: &str| parse_i64(w).map(|v| v as i32)),
        ValueType::Sint64 => each!(Sint64, |w: &str| parse_i64(w)),
        ValueType::Float32 => each!(Float32, |w: &str| parse_float(w).map(|v| v as f32)),
        ValueType::Float64 => each!(Float64, |w: &str| parse_float(w)),
        ValueType::Bool => each!(Bool, |w: &str| parse_bool(w)),
    };
    let is_array = rep > 0 || values.len() != 1 || ty == ValueType::String;
    Ok(Property { id, is_array, values })
}

fn find_outside_quotes(s: &str, needle: char) -> Option<usize> {
    let mut in_str = false;
    for (i, c) in s.char_indices() {
        if c == '"' {
            in_str = !in_str;
        } else if c == needle && !in_str {
            return Some(i);
        }
    }
    None
}

/// Resolves `id` on `ex`, falling back to its parent cohorts through `lookup`.
pub fn resolve<'a>(ex: &'a Exemplar, id: u32, lookup: impl Fn(&Tgi) -> Option<&'a Exemplar>) -> Option<&'a Property> {
    let mut cur = ex;
    let mut seen: Vec<Tgi> = Vec::new();
    for _ in 0..=MAX_PARENT_DEPTH {
        if let Some(p) = cur.get(id) {
            return Some(p);
        }
        if !cur.has_parent() || seen.contains(&cur.parent) {
            return None;
        }
        seen.push(cur.parent);
        cur = lookup(&cur.parent)?;
    }
    None
}

/// What happened when following an exemplar's parent chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChainEnd {
    /// The last exemplar has no parent.
    Root,
    /// A parent key that the store does not contain.
    Missing(Tgi),
    /// A parent key already visited.
    Cycle(Tgi),
    /// [`MAX_PARENT_DEPTH`] reached.
    TooDeep,
}

/// A set of exemplars and cohorts by key, with cohort inheritance.
///
/// \[choice\] Inserting a key that is already present replaces it, so loading packages in the game's load
/// order makes later packages override earlier ones.
#[derive(Default)]
pub struct ExemplarStore {
    map: HashMap<Tgi, Exemplar>,
}

/// Result of [`ExemplarStore::load_package`].
#[derive(Debug, Default)]
pub struct LoadReport {
    pub exemplars: usize,
    pub cohorts: usize,
    pub failures: Vec<(Tgi, Error)>,
}

impl ExemplarStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn insert(&mut self, tgi: Tgi, ex: Exemplar) -> Option<Exemplar> {
        self.map.insert(tgi, ex)
    }

    pub fn get(&self, tgi: &Tgi) -> Option<&Exemplar> {
        self.map.get(tgi)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Tgi, &Exemplar)> {
        self.map.iter()
    }

    /// Parses every exemplar and cohort entry of a package into the store.
    pub fn load_package<S: Source>(&mut self, pkg: &Package<S>) -> LoadReport {
        let mut report = LoadReport::default();
        for e in pkg.entries() {
            let ty = e.tgi.type_id;
            if ty != TYPE_EXEMPLAR && ty != TYPE_COHORT {
                continue;
            }
            match pkg.read(e).and_then(|d| Exemplar::parse(&d)) {
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

    /// `id` on `ex` or, when missing, on its nearest parent cohort that has it.
    pub fn resolve<'a>(&'a self, ex: &'a Exemplar, id: u32) -> Option<&'a Property> {
        resolve(ex, id, |t| self.map.get(t))
    }

    /// `id` on the exemplar stored under `tgi`, with inheritance.
    pub fn property(&self, tgi: &Tgi, id: u32) -> Option<&Property> {
        self.resolve(self.map.get(tgi)?, id)
    }

    /// `ex` followed by its parent cohorts, and how the chain ended.
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
                Some(p) => {
                    out.push(p);
                    cur = p;
                }
                None => return (out, ChainEnd::Missing(cur.parent)),
            }
        }
    }

    /// Every property visible on `ex` (own ones override inherited ones), sorted by ID. Each item also says
    /// how many levels up it came from (0 = own).
    pub fn resolved_properties<'a>(&'a self, ex: &'a Exemplar) -> Vec<(usize, &'a Property)> {
        let (chain, _) = self.chain(ex);
        let mut map: HashMap<u32, (usize, &Property)> = HashMap::new();
        for (depth, e) in chain.iter().enumerate().rev() {
            for p in &e.properties {
                map.insert(p.id, (depth, p));
            }
        }
        let mut v: Vec<_> = map.into_values().collect();
        v.sort_by_key(|(_, p)| p.id);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbpf::{Compression, PackageBuilder};

    fn sample() -> Exemplar {
        let mut ex = Exemplar::new(Kind::Exemplar);
        ex.parent = Tgi::new(TYPE_COHORT, 0x1234, 0x5678);
        ex.properties = vec![
            Property::single(0x10, Values::Uint32(vec![2])),
            Property::array(0x20, Values::String(b"Test Exemplar".to_vec())),
            Property::single(0x21, Values::Uint8(vec![200])),
            Property::array(0x22, Values::Uint16(vec![1, 65535])),
            Property::array(0x23, Values::Sint32(vec![-5, 7, i32::MIN])),
            Property::single(0x24, Values::Sint64(vec![-1_234_567_890_123])),
            Property::array(0x25, Values::Float32(vec![1.5, -0.25, 1e-3])),
            Property::single(0x26, Values::Bool(vec![true])),
            Property::array(0x27, Values::Bool(vec![false, true])),
            Property::array(0x28, Values::Uint32(vec![])),
            Property::array(0x29, Values::Uint32(vec![7])),
        ];
        ex
    }

    #[test]
    fn binary_layout() {
        let mut ex = Exemplar::new(Kind::Cohort);
        ex.properties.push(Property::single(0x10, Values::Uint32(vec![0x2A])));
        ex.properties
            .push(Property::array(0x20, Values::String(b"ab".to_vec())));
        let b = ex.to_binary();
        let mut want = b"CQZB1###".to_vec();
        want.extend_from_slice(&[0; 12]);
        want.extend_from_slice(&[2, 0, 0, 0]);
        want.extend_from_slice(&[0x10, 0, 0, 0, 0x00, 0x03, 0x00, 0x00, 0x00, 0x2A, 0, 0, 0]);
        want.extend_from_slice(&[0x20, 0, 0, 0, 0x00, 0x0C, 0x80, 0x00, 0x00, 2, 0, 0, 0, b'a', b'b']);
        assert_eq!(b, want);
        let back = Exemplar::parse(&b).unwrap();
        assert_eq!(back.kind, Kind::Cohort);
        assert_eq!(back.get(0x10).unwrap().as_u32(), Some(0x2A));
        assert_eq!(back.get(0x20).unwrap().as_str(), Some("ab"));
    }

    #[test]
    fn binary_round_trip_and_typed_api() {
        let ex = sample();
        let back = Exemplar::parse(&ex.to_binary()).unwrap();
        assert_eq!(back, ex);
        assert_eq!(back.parent, ex.parent);
        assert_eq!(back.get(0x10).unwrap().as_u32(), Some(2));
        assert_eq!(back.get(0x10).unwrap().as_f32(), None);
        assert_eq!(back.get(0x20).unwrap().as_str(), Some("Test Exemplar"));
        assert_eq!(back.get(0x21).unwrap().as_u8(), Some(200));
        assert_eq!(back.get(0x22).unwrap().u16s(), Some(&[1, 65535][..]));
        assert_eq!(back.get(0x23).unwrap().i32s().unwrap()[2], i32::MIN);
        assert_eq!(back.get(0x24).unwrap().as_i64(), Some(-1_234_567_890_123));
        assert_eq!(back.get(0x25).unwrap().f32s().unwrap().len(), 3);
        assert_eq!(back.get(0x26).unwrap().as_bool(), Some(true));
        assert_eq!(back.get(0x27).unwrap().bools(), Some(&[false, true][..]));
        assert!(back.get(0x28).unwrap().is_empty());
        assert!(back.get(0x29).unwrap().is_array);
        assert_eq!(back.get(0x23).unwrap().int(0), Some(-5));
        assert_eq!(back.get(0x25).unwrap().float(0), Some(1.5));
    }

    #[test]
    fn text_round_trip() {
        let ex = sample();
        let text = ex.to_text();
        let back = Exemplar::parse(text.as_bytes()).unwrap();
        assert_eq!(back.encoding, Encoding::Text);
        assert_eq!(back.parent, ex.parent);
        assert_eq!(back.properties, ex.properties);
    }

    #[test]
    fn text_maxis_style() {
        let text = "EQZT1###\r\n\
ParentCohort=Key:{0x8A5971C5,0xA4D9D8E4,0x05342861}\r\n\
PropCount=0x00000005\r\n\
0x00000010:{\"Exemplar Type\"}=Uint32:0:{0x00000002}\r\n\
0x00000020:{\"Exemplar Name\"}=String:1:{\"Fire, Station {Small}\"}\r\n\
0x27812810:{\"Occupant Size\"}=Float32:3:{12.5,6.0,-3}\r\n\
0x4A0D2F8C:{\"Flag\"}=Bool:0:{True}\r\n\
0xEA54D283:{\"Signed\"}=Sint32:2:{-1,\r\n  0xFFFFFFFE}\r\n\
0x88edc795:{\"LotConfigPropertyWealthTypes\"}=Uint8:0:{}\r\n\
0x00000021:{\"Name\"}=String:0:{\"x\"}\r\n\
0x099afacd:{\"Bulldoze Cost\"}=Sint64:0:{0x0000000000001356}\r\n\r\n";
        let ex = Exemplar::parse(text.as_bytes()).unwrap();
        // text key order is group, instance, type
        assert_eq!(ex.parent, Tgi::new(0x0534_2861, 0x8A59_71C5, 0xA4D9_D8E4));
        assert!(ex
            .to_text()
            .contains("ParentCohort=Key:{0x8a5971c5,0xa4d9d8e4,0x05342861}"));
        assert_eq!(ex.properties.len(), 8);
        assert!(ex.get(0x88ED_C795).unwrap().is_empty());
        assert_eq!(ex.get(0x21).unwrap().as_str(), Some("x"));
        assert_eq!(ex.get(0x099A_FACD).unwrap().as_i64(), Some(4950));
        assert_eq!(ex.get(0x10).unwrap().as_u32(), Some(2));
        assert!(!ex.get(0x10).unwrap().is_array);
        assert_eq!(ex.get(0x20).unwrap().as_str(), Some("Fire, Station {Small}"));
        assert_eq!(ex.get(0x2781_2810).unwrap().f32s(), Some(&[12.5, 6.0, -3.0][..]));
        assert_eq!(ex.get(0x4A0D_2F8C).unwrap().as_bool(), Some(true));
        assert_eq!(ex.get(0xEA54_D283).unwrap().i32s(), Some(&[-1, -2][..]));
    }

    #[test]
    fn bad_exemplars_are_errors() {
        assert!(matches!(Exemplar::parse(b"XQZB1###"), Err(Error::BadMagic { .. })));
        assert!(matches!(Exemplar::parse(b"EQZB2###"), Err(Error::Unsupported { .. })));
        let good = sample().to_binary();
        for n in 0..good.len() {
            assert!(Exemplar::parse(&good[..n]).is_err(), "prefix {n}");
        }
        let mut huge = good.clone();
        huge[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Exemplar::parse(&huge).is_err());
        let mut bad_type = good.clone();
        bad_type[28] = 0x0D;
        bad_type[29] = 0x0D;
        assert!(matches!(Exemplar::parse(&bad_type), Err(Error::Corrupt { .. })));
        assert!(matches!(
            Exemplar::parse(b"EQZT1###\nParentCohort=Key:{1,2}\n"),
            Err(Error::Text { line: 2, .. })
        ));
        assert!(matches!(
            Exemplar::parse(b"EQZT1###\n0x10:{\"x\"}=Nope:0:{1}\n"),
            Err(Error::Text { .. })
        ));
        assert!(matches!(
            Exemplar::parse(b"EQZT1###\n0x10:{\"x\"}=Uint32:0:{1\n"),
            Err(Error::Text { .. })
        ));
    }

    #[test]
    fn cohort_inheritance() {
        let grand = Tgi::new(TYPE_COHORT, 1, 1);
        let parent = Tgi::new(TYPE_COHORT, 1, 2);
        let child = Tgi::new(TYPE_EXEMPLAR, 1, 3);
        let mut g = Exemplar::new(Kind::Cohort);
        g.set(Property::single(0xA, Values::Uint32(vec![1])));
        g.set(Property::single(0xB, Values::Uint32(vec![1])));
        g.set(Property::single(0xC, Values::Uint32(vec![1])));
        let mut p = Exemplar::new(Kind::Cohort);
        p.parent = grand;
        p.set(Property::single(0xB, Values::Uint32(vec![2])));
        p.set(Property::single(0xC, Values::Uint32(vec![2])));
        let mut c = Exemplar::new(Kind::Exemplar);
        c.parent = parent;
        c.set(Property::single(0xC, Values::Uint32(vec![3])));

        // through a package, so load_package is covered too
        let mut b = PackageBuilder::new();
        b.add(grand, &g.to_binary(), Compression::Auto).unwrap();
        b.add(parent, &p.to_text().into_bytes(), Compression::Auto).unwrap();
        b.add(child, &c.to_binary(), Compression::Always).unwrap();
        let pkg = Package::from_bytes(b.build().unwrap()).unwrap();
        let mut store = ExemplarStore::new();
        let rep = store.load_package(&pkg);
        assert_eq!((rep.exemplars, rep.cohorts, rep.failures.len()), (1, 2, 0));

        assert_eq!(store.property(&child, 0xA).unwrap().as_u32(), Some(1));
        assert_eq!(store.property(&child, 0xB).unwrap().as_u32(), Some(2));
        assert_eq!(store.property(&child, 0xC).unwrap().as_u32(), Some(3));
        assert!(store.property(&child, 0xD).is_none());
        let (chain, end) = store.chain(store.get(&child).unwrap());
        assert_eq!((chain.len(), end), (3, ChainEnd::Root));
        let all = store.resolved_properties(store.get(&child).unwrap());
        let summary: Vec<(u32, usize)> = all.iter().map(|(d, p)| (p.id, *d)).collect();
        assert_eq!(summary, vec![(0xA, 2), (0xB, 1), (0xC, 0)]);

        // missing parent and cycles end the walk without looping
        let mut orphan = Exemplar::new(Kind::Exemplar);
        orphan.parent = Tgi::new(TYPE_COHORT, 9, 9);
        assert_eq!(store.chain(&orphan).1, ChainEnd::Missing(orphan.parent));
        assert!(store.resolve(&orphan, 0xA).is_none());
        let mut a = Exemplar::new(Kind::Cohort);
        a.parent = Tgi::new(TYPE_COHORT, 5, 2);
        let mut b2 = Exemplar::new(Kind::Cohort);
        b2.parent = Tgi::new(TYPE_COHORT, 5, 1);
        store.insert(Tgi::new(TYPE_COHORT, 5, 1), a);
        store.insert(Tgi::new(TYPE_COHORT, 5, 2), b2);
        let start = store.get(&Tgi::new(TYPE_COHORT, 5, 1)).unwrap();
        assert!(store.resolve(start, 0xA).is_none());
        assert!(matches!(store.chain(start).1, ChainEnd::Cycle(_)));
    }
}
