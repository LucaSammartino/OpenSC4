// SPDX-License-Identifier: GPL-3.0-or-later
use super::{Error, Result};
use crate::persistence_runtime::{Field, Key, RawStream, SerialRecord};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

pub const TEXT_COOKIE: u32 = u32::from_le_bytes(*b"TKT1");
pub const BINARY_COOKIE: u32 = u32::from_le_bytes(*b"TKB1");
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub id: u32,
    pub ty: u16,
    pub count: u16,
}
pub const END: Descriptor = Descriptor { id: 1, ty: 3, count: 1 };

fn legacy_float(value: f64, fixed: bool) -> String {
    if value.is_finite() {
        return if fixed {
            format!("{value:.6}")
        } else {
            crate::rz_streams::format_g(value, b'.')
        };
    }
    let bits = value.to_bits();
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    let class = if fraction == 0 {
        "INF"
    } else if fraction == 0x0008_0000_0000_0000 && value.is_sign_negative() {
        "IND"
    } else if fraction & 0x0008_0000_0000_0000 != 0 {
        "QNAN"
    } else {
        "SNAN"
    };
    let mut text = format!("{}1.#{class}", if value.is_sign_negative() { "-" } else { "" });
    if fixed {
        for _ in class.len() + 1..6 {
            text.push('0');
        }
    }
    text
}

/// Binary boundary counters are independent of the serial stream's cursor.
pub struct BinaryIo<'a, S: RawStream> {
    pub record: &'a mut SerialRecord<S>,
    pub counter: u64,
    pub total: u64,
}
impl<'a, S: RawStream> BinaryIo<'a, S> {
    pub fn new(record: &'a mut SerialRecord<S>) -> Self {
        let total = record.raw().size();
        Self {
            record,
            counter: 4,
            total,
        }
    }
    fn advance(&mut self, width: u64) -> bool {
        self.counter = self.counter.wrapping_add(width);
        self.counter < self.total
    }
    pub fn read_number<T: Field>(&mut self, out: &mut T) -> bool {
        self.advance(T::WIDTH as u64) && self.record.get(out)
    }
    pub fn read_definition(&mut self, out: &mut Descriptor) -> bool {
        self.advance(8)
            && self.record.get(&mut out.id)
            && self.record.get(&mut out.ty)
            && self.record.get(&mut out.count)
    }
    pub fn read_key(&mut self, out: &mut Key) -> bool {
        self.advance(12) && self.record.get_key(out)
    }
    pub fn read_string(&mut self, out: &mut Vec<u8>) -> bool {
        if !self.record.get_string(out) {
            return false;
        }
        self.counter = self.counter.wrapping_add(out.len() as u64 + 4);
        self.counter <= self.total
    }
}
enum Adapter<'a, S: RawStream> {
    Text(TextIo<'a, S>),
    Binary(BinaryIo<'a, S>),
}
impl<'a, S: RawStream> Adapter<'a, S> {
    fn new(record: &'a mut SerialRecord<S>, text: bool) -> Self {
        if text {
            Self::Text(TextIo::new(record))
        } else {
            Self::Binary(BinaryIo::new(record))
        }
    }
    fn read_definition(&mut self, out: &mut Descriptor) -> bool {
        match self {
            Self::Text(io) => io.read_definition(out),
            Self::Binary(io) => io.read_definition(out),
        }
    }
    fn read_number<T: TextNumber>(&mut self, out: &mut T) -> bool {
        match self {
            Self::Text(io) => io.read_number(out),
            Self::Binary(io) => io.read_number(out),
        }
    }
    fn write_number<T: TextNumber>(&mut self, value: T) -> bool {
        match self {
            Self::Text(io) => io.write_number(value),
            Self::Binary(io) => io.record.set(&value),
        }
    }
    fn write_definition(&mut self, name: &[u8], d: Descriptor) -> bool {
        match self {
            Self::Text(io) => io.write_definition(name, d),
            Self::Binary(io) => io.record.set(&d.id) && io.record.set(&d.ty) && io.record.set(&d.count),
        }
    }
    fn read_values(&mut self, values: &mut Values) -> bool {
        macro_rules! numbers {
            ($v:expr) => {
                $v.iter_mut().all(|v| self.read_number(v))
            };
        }
        match values {
            Values::U8(v) => numbers!(v),
            Values::I8(v) => numbers!(v),
            Values::U16(v) => numbers!(v),
            Values::I16(v) => numbers!(v),
            Values::U32(v) => numbers!(v),
            Values::I32(v) => numbers!(v),
            Values::U64(v) => numbers!(v),
            Values::I64(v) => numbers!(v),
            Values::F32(v) => numbers!(v),
            Values::F64(v) => numbers!(v),
            Values::String(v) => v.iter_mut().all(|v| match self {
                Self::Text(io) => io.read_string(v),
                Self::Binary(io) => io.read_string(v),
            }),
            Values::Key(v) => v.iter_mut().all(|v| match self {
                Self::Text(io) => io.read_key(v),
                Self::Binary(io) => io.read_key(v),
            }),
        }
    }
    fn write_values(&mut self, values: &Values) -> bool {
        if let Self::Text(io) = self {
            return io.write_values(values);
        }
        macro_rules! numbers {
            ($v:expr) => {
                $v.iter().all(|v| self.write_number(*v))
            };
        }
        match values {
            Values::U8(v) => numbers!(v),
            Values::I8(v) => numbers!(v),
            Values::U16(v) => numbers!(v),
            Values::I16(v) => numbers!(v),
            Values::U32(v) => numbers!(v),
            Values::I32(v) => numbers!(v),
            Values::U64(v) => numbers!(v),
            Values::I64(v) => numbers!(v),
            Values::F32(v) => numbers!(v),
            Values::F64(v) => numbers!(v),
            Values::String(v) => v.iter().all(|v| match self {
                Self::Binary(io) => io.record.set_string(v),
                _ => unreachable!(),
            }),
            Values::Key(v) => v.iter().all(|v| match self {
                Self::Binary(io) => io.record.set_key(*v),
                _ => unreachable!(),
            }),
        }
    }
}

/// The twelve payloads use the wire mapping in section 5.2.
#[derive(Clone, Debug, PartialEq)]
pub enum Values {
    U8(Vec<u8>),
    I8(Vec<i8>),
    U16(Vec<u16>),
    I16(Vec<i16>),
    U32(Vec<u32>),
    I32(Vec<i32>),
    U64(Vec<u64>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    String(Vec<Vec<u8>>),
    Key(Vec<Key>),
}
impl Values {
    pub fn wire_type(&self) -> u16 {
        match self {
            Self::U8(_) => 1,
            Self::U16(_) => 2,
            Self::U32(_) => 3,
            Self::I32(_) => 4,
            Self::F32(_) => 5,
            Self::I64(_) => 6,
            Self::String(_) => 7,
            Self::Key(_) => 8,
            Self::I8(_) => 9,
            Self::I16(_) => 10,
            Self::U64(_) => 11,
            Self::F64(_) => 12,
        }
    }
    fn empty(ty: u16, count: u16) -> Self {
        let n = usize::from(count);
        match ty {
            1 => Self::U8(vec![0; n]),
            2 => Self::U16(vec![0; n]),
            3 => Self::U32(vec![0; n]),
            4 => Self::I32(vec![0; n]),
            5 => Self::F32(vec![0.0; n]),
            6 => Self::I64(vec![0; n]),
            7 => Self::String(vec![Vec::new(); n]),
            8 => Self::Key(vec![Key::default(); n]),
            9 => Self::I8(vec![0; n]),
            10 => Self::I16(vec![0; n]),
            11 => Self::U64(vec![0; n]),
            12 => Self::F64(vec![0.0; n]),
            _ => unreachable!("validated wire type"),
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::U8(v) => v.len(),
            Self::I8(v) => v.len(),
            Self::U16(v) => v.len(),
            Self::I16(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::I32(v) => v.len(),
            Self::U64(v) => v.len(),
            Self::I64(v) => v.len(),
            Self::F32(v) => v.len(),
            Self::F64(v) => v.len(),
            Self::String(v) => v.len(),
            Self::Key(v) => v.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
pub trait TextNumber: Field + Default {
    const BINARY_READ: bool = false;
    fn scan(bytes: &[u8]) -> Option<Self>;
    fn render(self) -> String;
}
fn scanner_start(bytes: &[u8]) -> &[u8] {
    &bytes[bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len())..]
}
fn integer_prefix(bytes: &[u8]) -> Option<&[u8]> {
    let text = scanner_start(bytes);
    let sign = usize::from(matches!(text.first(), Some(b'+' | b'-')));
    let digits = text[sign..].iter().take_while(|b| b.is_ascii_digit()).count();
    (digits != 0).then_some(&text[..sign + digits])
}
macro_rules! integers { ($($ty:ty),*) => { $(impl TextNumber for $ty {
    fn scan(bytes: &[u8]) -> Option<Self> {
        // Literal runtime-prefix reading; overflow wraps to caller width.
        let text = integer_prefix(bytes)?;
        let negative = text.starts_with(b"-");
        let n = text.iter().copied().filter(u8::is_ascii_digit).fold(0u64, |n,b| n.wrapping_mul(10).wrapping_add(u64::from(b-b'0')));
        Some(if negative { n.wrapping_neg() } else { n } as Self)
    }
    fn render(self) -> String { self.to_string() }
})* }; }
integers!(u8, i8, u16, i16, u32, i32, i64);
impl TextNumber for u64 {
    const BINARY_READ: bool = true;
    fn scan(bytes: &[u8]) -> Option<Self> {
        i64::scan(bytes).map(|v| v as u64)
    }
    fn render(self) -> String {
        self.to_string()
    }
}
fn float_prefix(bytes: &[u8]) -> Option<f64> {
    let bytes = scanner_start(bytes);
    if !bytes
        .get(usize::from(matches!(bytes.first(), Some(b'+' | b'-'))))
        .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
    {
        return None;
    }
    // Only the scanner's decimal prefix needs to be ASCII; suffix bytes do
    // not participate in the conversion and need not be valid UTF-8.
    let end = bytes
        .iter()
        .position(|b| !matches!(b, b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-'))
        .unwrap_or(bytes.len());
    let text = std::str::from_utf8(&bytes[..end]).ok()?;
    (1..=text.len())
        .rev()
        .filter(|&n| text.is_char_boundary(n))
        .find_map(|n| text[..n].parse().ok())
}
impl TextNumber for f32 {
    fn scan(bytes: &[u8]) -> Option<Self> {
        float_prefix(bytes).map(|v| v as f32)
    }
    fn render(self) -> String {
        legacy_float(f64::from(self), true)
    }
}
impl TextNumber for f64 {
    fn scan(bytes: &[u8]) -> Option<Self> {
        float_prefix(bytes)
    }
    fn render(self) -> String {
        legacy_float(self, false)
    }
}

/// Ãƒâ€šÃ‚Â§6 adapter. Cursor is deliberately independent of the raw stream position.
pub struct TextIo<'a, S: RawStream> {
    pub record: &'a mut SerialRecord<S>,
    pub counter: u64,
    pub total: u64,
}
impl<'a, S: RawStream> TextIo<'a, S> {
    pub fn new(record: &'a mut SerialRecord<S>) -> Self {
        let total = record.raw().size();
        Self {
            record,
            counter: 4,
            total,
        }
    }
    pub fn character(&mut self, byte: &mut u8) -> bool {
        self.counter = self.counter.wrapping_add(1);
        self.counter < self.total && self.record.get(byte)
    }
    pub fn delimited(&mut self, delimiter: u8, out: &mut Vec<u8>) -> bool {
        out.clear();
        let mut comment = false;
        let mut b = 0;
        while self.character(&mut b) {
            if comment {
                if b != b'\r' {
                    continue;
                }
                comment = false;
            }
            if b == b';' {
                comment = true;
                continue;
            }
            if b == b'\n' {
                continue;
            }
            if b == delimiter {
                let start = out.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(out.len());
                let end = out
                    .iter()
                    .rposition(|b| !b.is_ascii_whitespace())
                    .map_or(start, |n| n + 1);
                *out = out[start..end].to_vec();
                return true;
            }
            out.push(if matches!(b, b'\r' | b'\t') { b' ' } else { b });
        }
        false
    }
    pub fn read_number<T: TextNumber>(&mut self, out: &mut T) -> bool {
        if T::BINARY_READ {
            self.counter = self.counter.wrapping_add(T::WIDTH as u64);
            return self.counter < self.total && self.record.get(out);
        }
        let mut text = Vec::new();
        if !self.delimited(b',', &mut text) {
            return false;
        }
        if let Some(v) = T::scan(&text) {
            *out = v;
            true
        } else {
            *out = T::default();
            false
        }
    }
    pub fn read_string(&mut self, out: &mut Vec<u8>) -> bool {
        self.delimited(b',', out)
    }
    pub fn read_key(&mut self, out: &mut Key) -> bool {
        let mut words = [0u32; 3];
        for w in &mut words {
            if !self.read_number(w) {
                return false;
            }
        }
        *out = Key::new(words[0], words[1], words[2]);
        true
    }
    pub fn read_definition(&mut self, out: &mut Descriptor) -> bool {
        let mut text = Vec::new();
        let mut id = 0;
        let mut ty = 0;
        let mut count = 0;
        if !self.delimited(b'[', &mut text)
            || !self.read_number(&mut id)
            || !self.read_number(&mut ty)
            || !self.delimited(b']', &mut text)
        {
            return false;
        }
        let Some(c) = u32::scan(&text) else {
            return false;
        };
        count += c;
        if !self.delimited(b'=', &mut text) {
            return false;
        }
        *out = Descriptor {
            id,
            ty: ty as u16,
            count: count as u16,
        };
        true
    }
    pub fn write_definition(&mut self, name: &[u8], descriptor: Descriptor) -> bool {
        let mut bytes = b"\r\n".to_vec();
        bytes.extend(crate::rz_streams::c_str(name));
        bytes.extend(format!("[ {}, {}, {} ]= ", descriptor.id, descriptor.ty, descriptor.count).as_bytes());
        self.record.set_string(&bytes)
    }
    pub fn write_number<T: TextNumber>(&mut self, value: T) -> bool {
        self.record.set_string(format!("{}, ", value.render()).as_bytes())
    }
    pub fn write_string(&mut self, value: &[u8]) -> bool {
        let mut bytes = crate::rz_streams::c_str(value).to_vec();
        bytes.extend(b", ");
        self.record.set_string(&bytes)
    }
    pub fn write_key(&mut self, key: Key) -> bool {
        self.write_number(key.ty) && self.write_number(key.group) && self.write_number(key.instance)
    }
    pub fn write_values(&mut self, values: &Values) -> bool {
        macro_rules! numbers {
            ($v:expr) => {
                $v.iter().all(|v| self.write_number(*v))
            };
        }
        match values {
            Values::U8(v) => numbers!(v),
            Values::I8(v) => numbers!(v),
            Values::U16(v) => numbers!(v),
            Values::I16(v) => numbers!(v),
            Values::U32(v) => numbers!(v),
            Values::I32(v) => numbers!(v),
            Values::U64(v) => numbers!(v),
            Values::I64(v) => numbers!(v),
            Values::F32(v) => numbers!(v),
            Values::F64(v) => numbers!(v),
            Values::String(v) => v.iter().all(|v| self.write_string(v)),
            Values::Key(v) => v.iter().all(|v| self.write_key(*v)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TokenField {
    pub descriptor: Descriptor,
    pub name: Vec<u8>,
    pub values: Values,
}
pub type SerialHandle<S> = Arc<Mutex<SerialRecord<S>>>;
pub struct TokenRecord<S: RawStream> {
    pub references: u32,
    serial: Option<SerialHandle<S>>,
    cookie: u32,
    initialized: bool,
    text: bool,
    write: bool,
    fields: BTreeMap<u32, TokenField>,
}
impl<S: RawStream> Default for TokenRecord<S> {
    fn default() -> Self {
        Self {
            references: 0,
            serial: None,
            cookie: 0,
            initialized: false,
            text: false,
            write: false,
            fields: BTreeMap::new(),
        }
    }
}
impl<S: RawStream> TokenRecord<S> {
    pub fn query_interface(&mut self, iid: u32, out: Option<&mut bool>) -> bool {
        let Some(out) = out else { return false };
        if !matches!(iid, 1 | 0xa2ac_e662) {
            return false;
        }
        self.add_ref();
        *out = true;
        true
    }
    pub fn add_ref(&mut self) -> u32 {
        self.references = self.references.wrapping_add(1);
        self.references
    }
    pub fn release(&mut self) -> u32 {
        self.references = self.references.wrapping_sub(1);
        if self.references == 0 {
            self.shutdown();
        }
        self.references
    }
    pub fn cookie(&self) -> u32 {
        if self.initialized {
            self.cookie
        } else {
            0
        }
    }
    pub fn is_text(&self) -> bool {
        self.text
    }
    pub fn key(&self) -> Option<Key> {
        self.serial
            .as_ref()
            .map(|s| s.lock().unwrap_or_else(|e| e.into_inner()).record.key())
    }
    pub fn available(&self, id: u32) -> bool {
        self.fields.contains_key(&id)
    }
    pub fn field(&self, descriptor: Descriptor) -> Option<&TokenField> {
        self.fields.get(&descriptor.id).filter(|f| f.descriptor == descriptor)
    }
    pub fn get(&self, descriptor: Descriptor) -> Option<&Values> {
        self.field(descriptor).map(|f| &f.values)
    }
    pub fn create_field(&mut self, descriptor: Descriptor) -> Result<bool> {
        if descriptor.count == 0 || !(1..=12).contains(&descriptor.ty) {
            return Err(Error::Io);
        }
        if self.field(descriptor).is_some() {
            return Ok(false);
        }
        self.fields.remove(&descriptor.id);
        self.fields.insert(
            descriptor.id,
            TokenField {
                descriptor,
                name: Vec::new(),
                values: Values::empty(descriptor.ty, descriptor.count),
            },
        );
        Ok(true)
    }
    pub fn set(&mut self, id: u32, values: Values) -> Result<()> {
        let count = u16::try_from(values.len()).map_err(|_| Error::Io)?;
        let ty = values.wire_type();
        let d = Descriptor { id, ty, count };
        self.create_field(d)?;
        self.fields.get_mut(&id).expect("created field").values = values;
        Ok(())
    }
    pub fn set_token_string(&mut self, id: u32, text: &[u8]) -> bool {
        let Some(field) = self.fields.get_mut(&id) else {
            return false;
        };
        // The name is independent of descriptor matching and payload.
        field.name = text.to_vec();
        true
    }
    fn attach(&mut self, serial: SerialHandle<S>, cookie: u32, write: bool) -> bool {
        if self.initialized {
            return false;
        }
        serial.lock().unwrap_or_else(|e| e.into_inner()).record.add_ref();
        self.serial = Some(serial);
        self.cookie = cookie;
        self.text = cookie == TEXT_COOKIE;
        self.write = write;
        self.initialized = true;
        true
    }
    pub fn init_write(&mut self, serial: SerialHandle<S>, text: bool) -> bool {
        self.attach(serial, if text { TEXT_COOKIE } else { BINARY_COOKIE }, true)
    }
    pub fn init_read(&mut self, serial: SerialHandle<S>, explicit_cookie: Option<u32>) -> Result<bool> {
        if self.initialized {
            return Ok(false);
        }
        let cookie = if let Some(c) = explicit_cookie {
            c
        } else {
            let mut c = 0;
            if !serial.lock().unwrap_or_else(|e| e.into_inner()).get(&mut c) {
                return Ok(false);
            }
            c
        };
        self.attach(serial, cookie, false);
        let result = self.read_tokens();
        if !matches!(result, Ok(true)) {
            self.shutdown();
        }
        result
    }
    pub fn read_tokens(&mut self) -> Result<bool> {
        let serial = self.serial.clone().ok_or(Error::InvalidContext)?;
        let mut record = serial.lock().unwrap_or_else(|e| e.into_inner());
        let mut io = Adapter::new(&mut record, self.text);
        loop {
            let mut d = END;
            if !io.read_definition(&mut d) {
                return Ok(false);
            }
            if d.id == 1 {
                return Ok(true);
            }
            if self.create_field(d).is_err() {
                return Ok(false);
            }
            let field = self.fields.get_mut(&d.id).expect("created field");
            if !io.read_values(&mut field.values) {
                return Ok(false);
            }
        }
    }
    pub fn write_tokens(&mut self) -> Result<bool> {
        let serial = self.serial.as_ref().ok_or(Error::InvalidContext)?;
        let mut record = serial.lock().unwrap_or_else(|e| e.into_inner());
        if !record.set(&self.cookie) {
            return Ok(false);
        }
        let mut io = Adapter::new(&mut record, self.text);
        for field in self.fields.values() {
            if !io.write_definition(&field.name, field.descriptor) || !io.write_values(&field.values) {
                return Ok(false);
            }
        }
        Ok(io.write_definition(b"TK_TKR_End", END) && io.write_number(0u32))
    }
    /// Deliberately void: no durable-write success may be inferred.
    pub fn shutdown(&mut self) {
        if self.initialized && self.write {
            let _ = self.write_tokens();
        }
        if let Some(serial) = self.serial.take() {
            serial.lock().unwrap_or_else(|e| e.into_inner()).record.release();
        }
        self.fields.clear();
        self.initialized = false;
    }
}
impl<S: RawStream> Drop for TokenRecord<S> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
