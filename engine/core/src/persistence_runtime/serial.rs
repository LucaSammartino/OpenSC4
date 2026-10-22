// SPDX-License-Identifier: GPL-3.0-or-later
use super::{DbRecord, Key, RecordInterface, INPUT_IID, OUTPUT_IID, SERIAL_IID};

/// Only raw operations are foreign. No sticky-error/atomic-read assumption is
/// imposed on a host: failed reads may have modified any prefix of `out`.
pub trait RawStream {
    fn read(&mut self, out: &mut [u8]) -> bool;
    fn write(&mut self, bytes: &[u8]) -> bool;
    fn position(&self) -> u64;
    fn size(&self) -> u64;
    fn seek(&mut self, position: u64) -> bool;
    fn close(&mut self) -> bool;
}

/// Standard portable in-memory byte/seek primitive. read_exact may update a
/// prefix before failing; typed readers preserve those effects.
impl RawStream for std::io::Cursor<Vec<u8>> {
    fn read(&mut self, out: &mut [u8]) -> bool {
        std::io::Read::read_exact(self, out).is_ok()
    }
    fn write(&mut self, bytes: &[u8]) -> bool {
        std::io::Write::write_all(self, bytes).is_ok()
    }
    fn position(&self) -> u64 {
        std::io::Cursor::position(self)
    }
    fn size(&self) -> u64 {
        self.get_ref().len() as u64
    }
    fn seek(&mut self, position: u64) -> bool {
        std::io::Seek::seek(self, std::io::SeekFrom::Start(position)).is_ok()
    }
    fn close(&mut self) -> bool {
        true
    }
}

mod sealed {
    pub trait Sealed {}
}
/// Bit-preserving typed field, portable LE host representation. Integers alone
/// obey native_order=false; floats retain their LE IEEE bytes (§3.3).
pub trait Field: sealed::Sealed + Copy {
    const WIDTH: usize;
    const INTEGER: bool;
    fn encode(self, bytes: &mut [u8]);
    fn decode(bytes: &[u8]) -> Self;
}
macro_rules! fields {
    ($integer:expr; $($ty:ty),+ $(,)?) => { $(
        impl sealed::Sealed for $ty {}
        impl Field for $ty {
            const WIDTH: usize = std::mem::size_of::<Self>();
            const INTEGER: bool = $integer;
            fn encode(self, bytes: &mut [u8]) { bytes.copy_from_slice(&self.to_le_bytes()); }
            fn decode(bytes: &[u8]) -> Self {
                let mut array = [0; std::mem::size_of::<Self>()];
                array.copy_from_slice(bytes);
                Self::from_le_bytes(array)
            }
        }
    )+ };
}
fields!(true; u8, i8, u16, i16, u32, i32, u64, i64);
fields!(false; f32, f64);

pub struct SerialRecord<S> {
    pub record: DbRecord,
    raw: S,
    native_order: bool,
}
impl<S: RawStream> SerialRecord<S> {
    /// Convenience owner with an explicit logical reference, no segment. Use
    /// with_record for segment-backed records and other constructor counts.
    pub fn new(raw: S, native_order: bool) -> Self {
        Self::with_record(raw, native_order, DbRecord::new(Key::default(), 1, 0, None))
    }
    pub fn with_record(raw: S, native_order: bool, record: DbRecord) -> Self {
        Self {
            record,
            raw,
            native_order,
        }
    }
    pub fn raw(&self) -> &S {
        &self.raw
    }
    pub fn raw_mut(&mut self) -> &mut S {
        &mut self.raw
    }
    pub fn native_order(&self) -> bool {
        self.native_order
    }
    pub fn query_interface(&mut self, iid: u32) -> Option<RecordInterface> {
        let view = match iid {
            SERIAL_IID => RecordInterface::Serial,
            INPUT_IID => RecordInterface::Input,
            OUTPUT_IID => RecordInterface::Output,
            _ => return self.record.query_interface(iid),
        };
        self.record.add_ref();
        Some(view)
    }
    pub fn adapter(&mut self) -> StreamAdapter<'_, S> {
        StreamAdapter { serial: self }
    }
    pub fn get<T: Field>(&mut self, value: &mut T) -> bool {
        self.get_array(std::slice::from_mut(value))
    }
    pub fn set<T: Field>(&mut self, value: &T) -> bool {
        self.set_array(std::slice::from_ref(value))
    }
    pub fn get_array<T: Field>(&mut self, values: &mut [T]) -> bool {
        let Some(len) = values.len().checked_mul(T::WIDTH) else {
            return false;
        };
        let mut bytes = vec![0; len];
        // Seed with existing bits so failed raw reads retain their own effects.
        for (value, bytes) in values.iter().zip(bytes.chunks_exact_mut(T::WIDTH)) {
            value.encode(bytes);
        }
        let result = self.raw.read(&mut bytes);
        for (value, bytes) in values.iter_mut().zip(bytes.chunks_exact_mut(T::WIDTH)) {
            if !self.native_order && T::INTEGER {
                bytes.reverse();
            }
            *value = T::decode(bytes);
        }
        result
    }
    pub fn set_array<T: Field>(&mut self, values: &[T]) -> bool {
        let Some(len) = values.len().checked_mul(T::WIDTH) else {
            return false;
        };
        let mut bytes = vec![0; len];
        for (value, bytes) in values.iter().zip(bytes.chunks_exact_mut(T::WIDTH)) {
            value.encode(bytes);
            if !self.native_order && T::INTEGER {
                bytes.reverse();
            }
        }
        self.raw.write(&bytes)
    }
    pub fn set_string(&mut self, bytes: &[u8]) -> bool {
        // Published writer domain (foundations §2.3). Invalid writer inputs
        // are rejected as a portable API precondition, not a native claim.
        if bytes.len() > 200_000 || bytes.contains(&0) {
            return false;
        }
        self.set(&(bytes.len() as u32)) && self.raw.write(bytes)
    }
    pub fn get_string(&mut self, out: &mut Vec<u8>) -> bool {
        let remaining = self.raw.size().saturating_sub(self.raw.position());
        if remaining < 4 {
            out.clear();
            for _ in 0..remaining {
                let mut byte = [0];
                self.raw.read(&mut byte);
                out.push(byte[0]);
            }
            return true;
        }
        let start = self.raw.position();
        let mut header = 0u32;
        if !self.get(&mut header) {
            return false;
        }
        let count = (header & 0xffffff) as usize;
        if count > 200_000 {
            return false;
        }
        if count == 0 {
            out.clear();
            return true;
        }
        match header >> 24 {
            0 | 0x10 => {
                let utf16 = header >> 24 == 0x10;
                let mut bytes = vec![0; count * if utf16 { 2 } else { 1 }];
                let result = self.raw.read(&mut bytes);
                if !result {
                    return false;
                } // failure destination unpublished (PR10)
                if utf16 || (bytes.len() >= 2 && bytes[1] == 0) {
                    // PR10: the guessed UTF-16 native path can read outside
                    // the input. Only complete, valid code units are supported.
                    if bytes.len() % 2 != 0 {
                        return false;
                    }
                    let units: Vec<_> = bytes
                        .chunks_exact(2)
                        .map(|b| {
                            if self.native_order {
                                u16::from_le_bytes([b[0], b[1]])
                            } else {
                                u16::from_be_bytes([b[0], b[1]])
                            }
                        })
                        .collect();
                    let Ok(text) = String::from_utf16(&units) else {
                        return false;
                    };
                    *out = text.into_bytes();
                } else {
                    *out = bytes;
                }
                result
            }
            _ => {
                self.raw.seek(start);
                out.clear();
                // Failed-read byte contents are unspecified. The host owns
                // those bytes; retaining the preceding byte is not a native
                // guarantee (PR10), but count and return channel are exact.
                let mut byte = [0];
                for _ in 0..self.raw.size() {
                    self.raw.read(&mut byte);
                    out.push(byte[0]);
                }
                true
            }
        }
    }
    pub fn set_key(&mut self, key: Key) -> bool {
        self.set(&key.ty) && self.set(&key.group) && self.set(&key.instance)
    }
    pub fn get_key(&mut self, key: &mut Key) -> bool {
        self.get(&mut key.ty) && self.get(&mut key.group) && self.get(&mut key.instance)
    }
}

/// Borrowed views share the record position and logical count. QI acquisition
/// is performed by SerialRecord, not implicitly by constructing this borrow.
pub struct StreamAdapter<'a, S> {
    serial: &'a mut SerialRecord<S>,
}
impl<S: RawStream> StreamAdapter<'_, S> {
    pub fn set_void(&mut self, bytes: &[u8]) -> bool {
        self.serial.raw.write(bytes)
    }
    pub fn get_void(&mut self, bytes: &mut [u8]) -> bool {
        self.serial.raw.read(bytes)
    }
    pub fn set_gz_str(&mut self, bytes: &[u8]) -> bool {
        self.serial.set_string(bytes)
    }
    pub fn set_rz_char_str(&mut self, bytes: &[u8]) -> bool {
        self.serial.set_string(crate::rz_streams::c_str(bytes))
    }
}
