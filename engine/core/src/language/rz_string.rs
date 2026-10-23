//! Owned counted framework string, rz-strings.md §§1–5. This is portable state,
//! not the native 0x14-byte ABI object. An adapter owns that layout and allocator.

use super::{Released, IID_UNKNOWN};

pub const IID_STRING: u32 = 0x089B_7DC8;
pub const NOT_FOUND: u32 = u32::MAX;

/// Only explicitly unsettled operations return Unsupported. Range errors remain
/// range errors; allocation failure is never a successful empty edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringFault {
    Range,
    Unsupported(&'static str),
    /// A typed core caller supplied an argument of the wrong kind or too few.
    Argument,
}

/// Foreign string interfaces provide their counted bytes; Copy does not retain
/// the interface. Adapters obtain length and const buffer through slots 7 and 6.
pub trait StringSource {
    fn bytes(&self) -> &[u8];
}

#[derive(Debug)]
pub struct RzString {
    // Includes exactly one allocation terminator outside the logical range.
    buffer: Vec<u8>,
    refs: u32,
}

impl Default for RzString {
    fn default() -> Self {
        let mut buffer = Vec::with_capacity(8);
        buffer.push(0);
        Self { buffer, refs: 0 }
    }
}

impl Clone for RzString {
    fn clone(&self) -> Self {
        Self::from_bytes(self.bytes())
    }
}

impl StringSource for RzString {
    fn bytes(&self) -> &[u8] {
        &self.buffer[..self.buffer.len() - 1]
    }
}

pub(super) fn c_prefix(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())]
}

pub(super) fn ascii_case(bytes: &[u8], upper: bool) -> Result<Vec<u8>, StringFault> {
    if bytes.iter().any(|&b| b >= 0x7f) {
        // RZS2: do not substitute Unicode case folding or platform ANSI tables.
        return Err(StringFault::Unsupported("RZS2"));
    }
    Ok(bytes
        .iter()
        .map(|b| {
            if upper {
                b.to_ascii_uppercase()
            } else {
                b.to_ascii_lowercase()
            }
        })
        .collect())
}

impl RzString {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert!(bytes.len() < u32::MAX as usize, "unrepresentable string length");
        let mut result = Self::new();
        result.from_char(Some(bytes), bytes.len() as u32);
        result
    }

    /// Non-null C-string construction is distinct from the null-accepting setter.
    pub fn from_c_string(bytes: &[u8]) -> Self {
        Self::from_bytes(c_prefix(bytes))
    }
    pub fn strlen(&self) -> u32 {
        (self.buffer.len() - 1) as u32
    }
    pub fn allocation_bytes(&self) -> usize {
        self.buffer.capacity()
    }
    pub fn to_char(&self) -> &[u8] {
        &self.buffer
    }
    /// Mutating a byte does not change the logical end. The allocation terminator
    /// is excluded so safe callers cannot break the invariant.
    pub fn to_char_mut(&mut self) -> &mut [u8] {
        let len = self.buffer.len() - 1;
        &mut self.buffer[..len]
    }
    pub fn ref_count(&self) -> u32 {
        self.refs
    }
    pub fn query_interface(&mut self, iid: u32) -> bool {
        if iid == IID_STRING || iid == IID_UNKNOWN {
            self.add_ref();
            true
        } else {
            false
        }
    }
    /// The adapter writes its receiver identity only on success.
    pub fn query_interface_into<T>(&mut self, iid: u32, out: &mut T, identity: T) -> bool {
        if self.query_interface(iid) {
            *out = identity;
            true
        } else {
            false
        }
    }
    pub fn add_ref(&mut self) -> u32 {
        self.refs = self.refs.wrapping_add(1);
        self.refs
    }
    /// Destroy means the owner must drop this object, including at count zero.
    pub fn release(&mut self) -> Released {
        if self.refs > 1 {
            self.refs -= 1;
            Released::Alive(self.refs)
        } else {
            Released::Destroy
        }
    }
    pub fn from_char(&mut self, input: Option<&[u8]>, count: u32) {
        let bytes = input.map(|b| &b[..count as usize]).unwrap_or_default();
        assert!(bytes.len() < u32::MAX as usize, "unrepresentable string length");
        self.buffer.clear();
        self.buffer.extend_from_slice(bytes);
        self.buffer.push(0);
    }
    pub fn set_c_string(&mut self, input: Option<&[u8]>) {
        let bytes = c_prefix(input.unwrap_or_default());
        self.from_char(Some(bytes), bytes.len() as u32);
    }
    pub fn copy(&mut self, input: &impl StringSource) {
        let bytes = input.bytes();
        self.from_char(Some(bytes), bytes.len() as u32);
    }
    pub fn assign(&mut self, input: &impl StringSource) -> &mut Self {
        self.copy(input);
        self
    }
    pub fn resize(&mut self, len: u32) {
        assert!(len < u32::MAX, "unrepresentable string length");
        self.buffer.pop();
        self.buffer.resize(len as usize, 0);
        self.buffer.push(0);
    }
    pub fn append(&mut self, bytes: &[u8]) -> &mut Self {
        self.insert(self.strlen(), bytes).expect("append at end");
        self
    }
    pub fn append_string(&mut self, other: &impl StringSource) -> &mut Self {
        self.append(other.bytes())
    }
    pub fn insert(&mut self, position: u32, bytes: &[u8]) -> Result<&mut Self, StringFault> {
        if position > self.strlen() {
            return Err(StringFault::Range);
        }
        assert!(self
            .bytes()
            .len()
            .checked_add(bytes.len())
            .is_some_and(|n| n < u32::MAX as usize));
        self.buffer
            .splice(position as usize..position as usize, bytes.iter().copied());
        Ok(self)
    }
    pub fn insert_string(&mut self, position: u32, other: &impl StringSource) -> Result<&mut Self, StringFault> {
        self.insert(position, other.bytes())
    }
    pub fn erase(&mut self, position: u32, count: u32) -> Result<&mut Self, StringFault> {
        if position > self.strlen() {
            return Err(StringFault::Range);
        }
        let end = position as usize + count.min(self.strlen() - position) as usize;
        self.buffer.drain(position as usize..end);
        Ok(self)
    }
    pub fn replace(&mut self, position: u32, source: &[u8], erase: u32) -> Result<&mut Self, StringFault> {
        if position > self.strlen() {
            return Err(StringFault::Range);
        }
        let bytes = c_prefix(source);
        let end = position as usize + erase.min(self.strlen() - position) as usize;
        assert!((self.strlen() as usize - (end - position as usize))
            .checked_add(bytes.len())
            .is_some_and(|n| n < u32::MAX as usize));
        self.buffer.splice(position as usize..end, bytes.iter().copied());
        Ok(self)
    }
    pub fn replace_string(&mut self, position: u32, source: &impl StringSource) -> Result<&mut Self, StringFault> {
        self.replace(position, source.bytes(), source.bytes().len() as u32)
    }
    pub fn strcat(&mut self, source: &[u8]) -> &mut Self {
        self.append(c_prefix(source))
    }
    pub fn strncpy(&mut self, source: &[u8], count: u32) {
        let source = c_prefix(source);
        self.from_char(Some(source), count.min(source.len() as u32));
    }
    fn operand(bytes: &[u8], count: u32) -> &[u8] {
        if count == u32::MAX {
            c_prefix(bytes)
        } else {
            &bytes[..count as usize]
        }
    }
    pub fn is_equal(&self, bytes: &[u8], count: u32, sensitive: bool) -> Result<bool, StringFault> {
        let other = Self::operand(bytes, count);
        if other.len() != self.bytes().len() {
            return Ok(false);
        }
        Ok(self.compare_bytes(other, sensitive)? == 0)
    }
    pub fn is_equal_string(&self, other: &impl StringSource, sensitive: bool) -> Result<bool, StringFault> {
        self.is_equal(other.bytes(), other.bytes().len() as u32, sensitive)
    }
    pub fn compare_to(&self, bytes: &[u8], count: u32, sensitive: bool) -> Result<i32, StringFault> {
        self.compare_bytes(Self::operand(bytes, count), sensitive)
    }
    pub fn compare_string(&self, other: &impl StringSource, sensitive: bool) -> Result<i32, StringFault> {
        self.compare_bytes(other.bytes(), sensitive)
    }
    fn compare_bytes(&self, other: &[u8], sensitive: bool) -> Result<i32, StringFault> {
        let left;
        let right;
        let (a, b) = if sensitive {
            (self.bytes(), other)
        } else {
            left = ascii_case(self.bytes(), false)?;
            right = ascii_case(other, false)?;
            (&left[..], &right[..])
        };
        for (&x, &y) in a.iter().zip(b) {
            if x != y {
                return Ok(x as i32 - y as i32);
            }
            if sensitive && x == 0 {
                break;
            }
        }
        Ok((self.strlen() as i32).wrapping_sub(other.len() as i32))
    }
    pub fn find(&self, needle: &[u8], start: u32, sensitive: bool) -> Result<u32, StringFault> {
        self.search(needle, start, sensitive, false)
    }
    pub fn rfind(&self, needle: &[u8], start: u32, sensitive: bool) -> Result<u32, StringFault> {
        self.search(needle, start, sensitive, true)
    }
    pub fn find_string(&self, needle: &impl StringSource, start: u32, sensitive: bool) -> Result<u32, StringFault> {
        self.find(needle.bytes(), start, sensitive)
    }
    pub fn rfind_string(&self, needle: &impl StringSource, start: u32, sensitive: bool) -> Result<u32, StringFault> {
        self.rfind(needle.bytes(), start, sensitive)
    }
    fn search(&self, needle: &[u8], start: u32, sensitive: bool, reverse: bool) -> Result<u32, StringFault> {
        let folded_hay;
        let folded_needle;
        let (hay, needle) = if sensitive {
            (self.bytes(), c_prefix(needle))
        } else {
            folded_hay = ascii_case(self.bytes(), false)?;
            folded_needle = ascii_case(needle, false)?;
            (&folded_hay[..], c_prefix(&folded_needle))
        };
        if needle.len() > hay.len() {
            return Ok(NOT_FOUND);
        }
        let last = hay.len() - needle.len();
        let found = if reverse {
            (0..=last.min(start as usize))
                .rev()
                .find(|&i| &hay[i..i + needle.len()] == needle)
        } else {
            (start as usize..=last).find(|&i| &hay[i..i + needle.len()] == needle)
        };
        Ok(found.map(|i| i as u32).unwrap_or(NOT_FOUND))
    }
    pub fn begins_with(&self, bytes: &[u8]) -> bool {
        self.bytes().starts_with(bytes)
    }
    pub fn ends_with(&self, bytes: &[u8]) -> bool {
        self.bytes().ends_with(bytes)
    }
    pub fn ltrim(&mut self) -> &mut Self {
        let n = self.bytes().iter().take_while(|&&b| b == b' ' || b == b'\t').count();
        self.buffer.drain(..n);
        self
    }
    pub fn rtrim(&mut self) -> &mut Self {
        let n = self
            .bytes()
            .iter()
            .rev()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        self.resize(self.strlen() - n as u32);
        self
    }
    pub fn trim(&mut self) -> &mut Self {
        self.ltrim().rtrim()
    }
    pub fn mid(&self, start: u32, count: u32) -> Self {
        let start = start.min(self.strlen());
        Self::from_bytes(&self.bytes()[start as usize..(start + count.min(self.strlen() - start)) as usize])
    }
    pub fn left(&self, count: u32) -> Self {
        self.mid(0, count)
    }
    pub fn right(&self, count: u32) -> Self {
        self.mid(self.strlen() - count.min(self.strlen()), count)
    }
    pub fn split_token(&mut self, delimiter: Option<&[u8]>) -> Self {
        let delimiter = c_prefix(delimiter.unwrap_or_default());
        if delimiter.is_empty() {
            return self.clone();
        }
        let at = self.find(delimiter, 0, true).expect("sensitive search");
        if at == NOT_FOUND {
            return Self::new();
        }
        let result = self.left(at);
        self.erase(0, at + delimiter.len() as u32).expect("found range");
        result
    }
    pub fn split_token_delimited(&mut self) -> Result<Self, StringFault> {
        Err(StringFault::Unsupported("RZS4"))
    }
    pub fn split_token_separated(&mut self) -> Result<Self, StringFault> {
        Err(StringFault::Unsupported("RZS4"))
    }
    pub fn make_lower(&mut self) -> Result<&mut Self, StringFault> {
        let bytes = ascii_case(self.bytes(), false)?;
        self.from_char(Some(&bytes), bytes.len() as u32);
        Ok(self)
    }
    pub fn make_upper(&mut self) -> Result<&mut Self, StringFault> {
        let bytes = ascii_case(self.bytes(), true)?;
        self.from_char(Some(&bytes), bytes.len() as u32);
        Ok(self)
    }
}
