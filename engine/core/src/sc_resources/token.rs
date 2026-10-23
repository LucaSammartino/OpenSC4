// SPDX-License-Identifier: GPL-3.0-or-later
use super::{Error, Result};
use crate::persistence_runtime::{Key, RawStream, SerialRecord};
use std::{collections::BTreeMap, sync::{Arc, Mutex}};

pub const TEXT_COOKIE: u32 = u32::from_le_bytes(*b"TKT1");
pub const BINARY_COOKIE: u32 = u32::from_le_bytes(*b"TKB1");
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor { pub id: u32, pub ty: u16, pub count: u16 }
pub const END: Descriptor = Descriptor { id: 1, ty: 3, count: 1 };

/// Twelve typed payloads, distinct from the unsettled wire-number mapping.
#[derive(Clone, Debug, PartialEq)]
pub enum Values {
    U8(Vec<u8>), I8(Vec<i8>), U16(Vec<u16>), I16(Vec<i16>), U32(Vec<u32>), I32(Vec<i32>),
    U64(Vec<u64>), I64(Vec<i64>), F32(Vec<f32>), F64(Vec<f64>), String(Vec<Vec<u8>>), Key(Vec<Key>),
}
impl Values {
    pub fn len(&self) -> usize {
        match self { Self::U8(v) => v.len(), Self::I8(v) => v.len(), Self::U16(v) => v.len(), Self::I16(v) => v.len(), Self::U32(v) => v.len(), Self::I32(v) => v.len(), Self::U64(v) => v.len(), Self::I64(v) => v.len(), Self::F32(v) => v.len(), Self::F64(v) => v.len(), Self::String(v) => v.len(), Self::Key(v) => v.len() }
    }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}
pub trait TextNumber: Copy + Default {
    fn scan(bytes: &[u8]) -> Option<Self>;
    fn render(self) -> String;
}
fn integer_prefix(bytes: &[u8]) -> Option<&str> {
    let text = std::str::from_utf8(bytes).ok()?.trim_start();
    let sign = usize::from(text.starts_with(['+', '-']));
    let digits = text.as_bytes()[sign..].iter().take_while(|b| b.is_ascii_digit()).count();
    (digits != 0).then_some(&text[..sign + digits])
}
macro_rules! integers { ($($ty:ty),*) => { $(impl TextNumber for $ty {
    fn scan(bytes: &[u8]) -> Option<Self> {
        // Literal runtime-prefix reading; overflow wraps to caller width.
        integer_prefix(bytes)?.parse::<i128>().ok().map(|v| v as Self)
    }
    fn render(self) -> String { self.to_string() }
})* }; }
integers!(u8, i8, u16, i16, u32, i32, u64, i64);
fn float_prefix(bytes: &[u8]) -> Option<f64> {
    let text = std::str::from_utf8(bytes).ok()?.trim_start();
    (1..=text.len()).rev().filter(|&n| text.is_char_boundary(n)).find_map(|n| text[..n].parse().ok())
}
impl TextNumber for f32 {
    fn scan(bytes: &[u8]) -> Option<Self> { float_prefix(bytes).map(|v| v as f32) }
    fn render(self) -> String { format!("{:.6}", f64::from(self)) }
}
impl TextNumber for f64 {
    fn scan(bytes: &[u8]) -> Option<Self> { float_prefix(bytes) }
    fn render(self) -> String { crate::rz_streams::format_g(self, b'.') }
}

/// §6 adapter. Cursor is deliberately independent of the raw stream position.
pub struct TextIo<'a, S: RawStream> { pub record: &'a mut SerialRecord<S>, pub counter: u64, pub total: u64 }
impl<'a, S: RawStream> TextIo<'a, S> {
    pub fn new(record: &'a mut SerialRecord<S>) -> Self { let total = record.raw().size(); Self { record, counter: 4, total } }
    pub fn character(&mut self, byte: &mut u8) -> bool {
        self.counter = self.counter.wrapping_add(1);
        self.counter < self.total && self.record.get(byte)
    }
    pub fn delimited(&mut self, delimiter: u8, out: &mut Vec<u8>) -> bool {
        out.clear(); let mut comment = false; let mut b = 0;
        while self.character(&mut b) {
            if comment { if b != b'\r' { continue; } comment = false; }
            if b == b';' { comment = true; continue; }
            if b == b'\n' { continue; }
            if b == delimiter {
                let start = out.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(out.len());
                let end = out.iter().rposition(|b| !b.is_ascii_whitespace()).map_or(start, |n| n + 1);
                *out = out[start..end].to_vec(); return true;
            }
            out.push(if matches!(b, b'\r' | b'\t') { b' ' } else { b });
        }
        false
    }
    pub fn read_number<T: TextNumber>(&mut self, out: &mut T) -> bool {
        let mut text = Vec::new(); if !self.delimited(b',', &mut text) { return false; }
        if let Some(v) = T::scan(&text) { *out = v; true } else { *out = T::default(); false }
    }
    pub fn read_string(&mut self, out: &mut Vec<u8>) -> bool { self.delimited(b',', out) }
    pub fn read_key(&mut self, out: &mut Key) -> bool {
        let mut words = [0u32; 3];
        for w in &mut words { if !self.read_number(w) { return false; } }
        *out = Key::new(words[0], words[1], words[2]); true
    }
    pub fn read_definition(&mut self, out: &mut Descriptor) -> bool {
        let mut text = Vec::new(); let mut id = 0; let mut ty = 0; let mut count = 0;
        if !self.delimited(b'[', &mut text) || !self.read_number(&mut id) || !self.read_number(&mut ty) || !self.delimited(b']', &mut text) { return false; }
        let Some(c) = u32::scan(&text) else { return false }; count += c;
        if !self.delimited(b'=', &mut text) { return false; }
        *out = Descriptor { id, ty: ty as u16, count: count as u16 }; true
    }
    pub fn write_definition(&mut self, name: &[u8], descriptor: Descriptor) -> bool {
        let mut bytes = b"\r\n".to_vec(); bytes.extend(name);
        bytes.extend(format!("[ {}, {}, {} ]= ", descriptor.id, descriptor.ty, descriptor.count).as_bytes());
        self.record.set_string(&bytes)
    }
    pub fn write_number<T: TextNumber>(&mut self, value: T) -> bool { self.record.set_string(format!("{}, ", value.render()).as_bytes()) }
    pub fn write_string(&mut self, value: &[u8]) -> bool {
        let mut bytes = value.to_vec(); bytes.extend(b", "); self.record.set_string(&bytes)
    }
    pub fn write_key(&mut self, key: Key) -> bool { self.write_number(key.ty) && self.write_number(key.group) && self.write_number(key.instance) }
    pub fn write_values(&mut self, values: &Values) -> bool {
        macro_rules! numbers { ($v:expr) => { $v.iter().all(|v| self.write_number(*v)) }; }
        match values {
            Values::U8(v) => numbers!(v), Values::I8(v) => numbers!(v), Values::U16(v) => numbers!(v), Values::I16(v) => numbers!(v), Values::U32(v) => numbers!(v), Values::I32(v) => numbers!(v), Values::U64(v) => numbers!(v), Values::I64(v) => numbers!(v), Values::F32(v) => numbers!(v), Values::F64(v) => numbers!(v),
            Values::String(v) => v.iter().all(|v| self.write_string(v)), Values::Key(v) => v.iter().all(|v| self.write_key(*v)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TokenField { pub descriptor: Descriptor, pub name: Vec<u8>, pub values: Values }
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
    fn default() -> Self { Self { references: 0, serial: None, cookie: 0, initialized: false, text: false, write: false, fields: BTreeMap::new() } }
}
impl<S: RawStream> TokenRecord<S> {
    pub fn query_interface(&mut self, iid: u32, out: Option<&mut bool>) -> bool {
        let Some(out) = out else { return false };
        if !matches!(iid, 1 | 0xa2ac_e662) { return false; }
        self.add_ref(); *out = true; true
    }
    pub fn add_ref(&mut self) -> u32 { self.references = self.references.wrapping_add(1); self.references }
    pub fn release(&mut self) -> u32 { self.references = self.references.wrapping_sub(1); if self.references == 0 { self.shutdown(); } self.references }
    pub fn cookie(&self) -> u32 { if self.initialized { self.cookie } else { 0 } }
    pub fn is_text(&self) -> bool { self.text }
    pub fn key(&self) -> Option<Key> { self.serial.as_ref().map(|s| s.lock().unwrap_or_else(|e| e.into_inner()).record.key()) }
    pub fn available(&self, id: u32) -> bool { self.fields.contains_key(&id) }
    pub fn field(&self, descriptor: Descriptor) -> Option<&TokenField> { self.fields.get(&descriptor.id).filter(|f| f.descriptor == descriptor) }
    pub fn get(&self, descriptor: Descriptor) -> Option<&Values> { self.field(descriptor).map(|f| &f.values) }
    pub fn create_field(&mut self, descriptor: Descriptor) -> Result<bool> {
        if descriptor.count == 0 || !(1..=12).contains(&descriptor.ty) { return Err(Error::Io); }
        if self.field(descriptor).is_some() { return Ok(false); }
        self.fields.remove(&descriptor.id);
        // SCR4: only wire type 3 is specified. Never derive it from getter slots.
        if descriptor.ty != 3 { return Err(Error::Unsupported("SCR4: wire type mapping")); }
        self.fields.insert(descriptor.id, TokenField { descriptor, name: Vec::new(), values: Values::U32(vec![0; usize::from(descriptor.count)]) }); Ok(true)
    }
    pub fn set(&mut self, id: u32, values: Values) -> Result<()> {
        let count = u16::try_from(values.len()).map_err(|_| Error::Io)?;
        let ty = if matches!(values, Values::U32(_)) { 3 } else { return Err(Error::Unsupported("SCR4: wire type mapping")); };
        let d = Descriptor { id, ty, count }; self.create_field(d)?;
        self.fields.get_mut(&id).expect("created field").values = values; Ok(())
    }
    pub fn set_token_string(&mut self, id: u32, text: &[u8]) -> bool {
        let Some(field) = self.fields.get_mut(&id) else { return false };
        // Literal association with the token's definition name; see SCR7.
        field.name = text.to_vec(); true
    }
    fn attach(&mut self, serial: SerialHandle<S>, cookie: u32, write: bool) -> bool {
        if self.initialized { return false; }
        serial.lock().unwrap_or_else(|e| e.into_inner()).record.add_ref();
        self.serial = Some(serial); self.cookie = cookie; self.text = cookie == TEXT_COOKIE; self.write = write; self.initialized = true; true
    }
    pub fn init_write(&mut self, serial: SerialHandle<S>, text: bool) -> bool { self.attach(serial, if text { TEXT_COOKIE } else { BINARY_COOKIE }, true) }
    pub fn init_read(&mut self, serial: SerialHandle<S>, explicit_cookie: Option<u32>) -> Result<bool> {
        if self.initialized { return Ok(false); }
        let cookie = if let Some(c) = explicit_cookie { c } else {
            let mut c = 0; if !serial.lock().unwrap_or_else(|e| e.into_inner()).get(&mut c) { return Ok(false); } c
        };
        self.attach(serial, cookie, false);
        let result = self.read_tokens();
        if !matches!(result, Ok(true)) { self.shutdown(); }
        result
    }
    pub fn read_tokens(&mut self) -> Result<bool> {
        if !self.text { return Err(Error::Unsupported("SCR4: binary field encoding")); }
        let serial = self.serial.clone().ok_or(Error::InvalidContext)?;
        let mut record = serial.lock().unwrap_or_else(|e| e.into_inner()); let mut io = TextIo::new(&mut record);
        loop {
            let mut d = END;
            if !io.read_definition(&mut d) { return Ok(false); }
            if d.id == 1 { return Ok(true); }
            self.create_field(d)?;
            let field = self.fields.get_mut(&d.id).expect("created field");
            if let Values::U32(values) = &mut field.values { for value in values { if !io.read_number(value) { return Ok(false); } } }
        }
    }
    pub fn write_tokens(&mut self) -> Result<bool> {
        if !self.text { return Err(Error::Unsupported("SCR4: binary field encoding")); }
        let serial = self.serial.as_ref().ok_or(Error::InvalidContext)?;
        let mut record = serial.lock().unwrap_or_else(|e| e.into_inner());
        if !record.set(&self.cookie) { return Ok(false); }
        let mut io = TextIo::new(&mut record);
        for field in self.fields.values() {
            if !io.write_definition(&field.name, field.descriptor) || !io.write_values(&field.values) { return Ok(false); }
        }
        Ok(io.write_definition(b"TK_TKR_End", END) && io.write_number(0u32))
    }
    /// Deliberately void: no durable-write success may be inferred.
    pub fn shutdown(&mut self) {
        if self.initialized && self.write { let _ = self.write_tokens(); }
        if let Some(serial) = self.serial.take() { serial.lock().unwrap_or_else(|e| e.into_inner()).record.release(); }
        self.fields.clear(); self.initialized = false;
    }
}
impl<S: RawStream> Drop for TokenRecord<S> { fn drop(&mut self) { self.shutdown(); } }
