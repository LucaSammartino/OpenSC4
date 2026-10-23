// SPDX-License-Identifier: GPL-3.0-or-later
use super::{RawStream, SerialRecord};
use crate::variant::{self, mem, Elem, Variant};

fn encode(value: &Elem) -> Option<(Vec<u8>, bool)> {
    let integer = !matches!(value, Elem::F32(_) | Elem::F64(_));
    let bytes = match *value {
        Elem::Bool(v) => vec![u8::from(v)],
        Elem::U8(v) | Elem::Char(v) | Elem::RzChar(v) => vec![v],
        Elem::I8(v) => v.to_le_bytes().to_vec(),
        Elem::U16(v) | Elem::UnicodeChar(v) => v.to_le_bytes().to_vec(),
        Elem::I16(v) => v.to_le_bytes().to_vec(),
        Elem::U32(v) => v.to_le_bytes().to_vec(),
        Elem::I32(v) => v.to_le_bytes().to_vec(),
        Elem::U64(v) => v.to_le_bytes().to_vec(),
        Elem::I64(v) => v.to_le_bytes().to_vec(),
        Elem::F32(v) => v.to_le_bytes().to_vec(),
        Elem::F64(v) => v.to_le_bytes().to_vec(),
        Elem::VoidPtr(_) => return None,
    };
    Some((bytes, integer))
}
fn tag_type(tag: u32) -> Option<(u16, usize)> {
    Some(match tag {
        0 => (mem::EMPTY, 1),
        1 => (mem::UINT8, 1),
        2 => (mem::UINT16, 2),
        3 => (mem::UINT32, 4),
        4 => (mem::UINT64, 8),
        5 => (mem::SINT8, 1),
        6 => (mem::SINT16, 2),
        7 => (mem::SINT32, 4),
        8 => (mem::SINT64, 8),
        9 => (mem::FLOAT32, 4),
        10 => (mem::FLOAT64, 8),
        11 => (mem::BOOL, 1),
        12 => (mem::RZ_CHAR, 1),
        16 => (mem::UNICODE_CHAR, 2),
        17 => (mem::CHAR, 1),
        _ => return None,
    })
}
fn decode(ty: u16, bytes: &[u8]) -> Elem {
    let mut padded = [0; 8];
    padded[..bytes.len()].copy_from_slice(bytes);
    let bits = u64::from_le_bytes(padded);
    match ty {
        mem::EMPTY | mem::UINT8 => Elem::U8(bits as u8),
        mem::SINT8 => Elem::I8(bits as i8),
        mem::UINT16 => Elem::U16(bits as u16),
        mem::SINT16 => Elem::I16(bits as i16),
        mem::UINT32 => Elem::U32(bits as u32),
        mem::SINT32 => Elem::I32(bits as i32),
        mem::UINT64 => Elem::U64(bits),
        mem::SINT64 => Elem::I64(bits as i64),
        mem::FLOAT32 => Elem::F32(f32::from_bits(bits as u32)),
        mem::FLOAT64 => Elem::F64(f64::from_bits(bits)),
        mem::BOOL => Elem::Bool(bits != 0),
        mem::RZ_CHAR => Elem::RzChar(bits as u8),
        mem::CHAR => Elem::Char(bits as u8),
        mem::UNICODE_CHAR => Elem::UnicodeChar(bits as u16),
        _ => unreachable!(),
    }
}
impl<S: RawStream> SerialRecord<S> {
    /// §3.6: reuse authoritative memory types/tag mapping, omit stream marker.
    pub fn set_variant(&mut self, value: &Variant) -> bool {
        let tag = if matches!(value, Variant::Object(_)) {
            0x0d
        } else {
            let Some(tag) = variant::disk_tag(value.mem_type()) else {
                return false;
            };
            u32::from(tag)
        };
        if !self.set(&tag) {
            return false;
        }
        if matches!(tag, 0x0d | 0x12 | 0x800d | 0x8012) {
            return false;
        }
        let items: &[Elem] = match value {
            Variant::Empty => return true,
            Variant::Object(_) => return false,
            Variant::Scalar(item) => std::slice::from_ref(item),
            Variant::Array { elem, items } => {
                if *elem == mem::RZ_CHAR {
                    let bytes: Vec<_> = items
                        .iter()
                        .map(|e| if let Elem::RzChar(b) = e { *b } else { 0 })
                        .collect();
                    return self.set_string(&bytes);
                }
                let Ok(count) = u32::try_from(items.len()) else {
                    return false;
                };
                if !self.set(&count) {
                    return false;
                }
                items
            }
        };
        let mut bytes = Vec::new();
        for item in items {
            let Some((mut encoded, integer)) = encode(item) else {
                return false;
            };
            if integer && !self.native_order() {
                encoded.reverse();
            }
            bytes.extend_from_slice(&encoded);
        }
        self.raw_mut().write(&bytes)
    }
    /// A fresh owned value is published only on complete success. Rust drops
    /// failed temporary allocations; no variant pointer is leaked or replaced.
    pub fn get_variant(&mut self, out: &mut Option<Variant>) -> bool {
        let mut tag = 0u32;
        if !self.get(&mut tag) {
            return false;
        }
        if tag == 0 {
            *out = Some(Variant::Empty);
            return true;
        }
        if tag == 0x800c {
            let mut bytes = Vec::new();
            if !self.get_string(&mut bytes) {
                return false;
            }
            *out = Some(Variant::string(&bytes));
            return true;
        }
        let array = tag & 0x8000 != 0;
        if (!array && tag > 12) || (array && !matches!(tag, 0x8000..=0x800c | 0x8010 | 0x8011)) {
            return false;
        }
        let Some((ty, width)) = tag_type(tag & !0x8000) else {
            return false;
        };
        let mut count = 1u32;
        if array && !self.get(&mut count) {
            return false;
        }
        let Some(len) = (count as usize).checked_mul(width) else {
            return false;
        };
        // foundations §9.2 explicit FreeCity4 bound, not native allocation behaviour.
        if array && len as u64 > self.raw().size().saturating_sub(self.raw().position()) {
            return false;
        }
        let mut bytes = vec![0; len];
        if !self.raw_mut().read(&mut bytes) {
            return false;
        }
        let mut items = Vec::with_capacity(count as usize);
        for bytes in bytes.chunks_exact_mut(width) {
            if !self.native_order() && ty != mem::FLOAT32 && ty != mem::FLOAT64 {
                bytes.reverse();
            }
            items.push(decode(ty, bytes));
        }
        *out = Some(if array {
            Variant::Array { elem: ty, items }
        } else {
            Variant::Scalar(items.remove(0))
        });
        true
    }
}
