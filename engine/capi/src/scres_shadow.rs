// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link transport for native adapter and command-token observations.
//! Partial sc-resources coverage; no image, graph writer or parser-state parity claim.
use fc4_core::persistence_runtime::{Field, Key, SerialRecord};
use fc4_core::sc_resources::token::{BinaryIo, Descriptor, TextIo};
use std::io::Cursor;

/// Length-delimited copies of borrowed native strings; never native STL layouts.
#[repr(C)]
pub struct ParserBytes {
    pub bytes: *const u8,
    pub length: usize,
}

fn compare_parser(line: &[u8], tokens: &[&[u8]]) -> u32 {
    // Compare at ParseCommandArguments ENTRY, before variable substitution.
    // parse_arguments also substitutes, so use the production lexical service
    // used by CommandParser::parse, retaining unresolved variable references.
    match fc4_core::sc_resources::lexical::split(line) {
        Ok(expected) => {
            u32::from(expected.len() != tokens.len() || expected.iter().zip(tokens).any(|(a, b)| a.bytes != *b))
        }
        Err(_) => 2, // Native reached argument dispatch with a lexical error.
    }
}

/// Private static-link observer, not part of fc4.h or the public DLL exports.
/// # Safety
/// All byte spans and the array must remain readable throughout this call.
#[no_mangle]
pub unsafe extern "C" fn fc4_scres_parser_compare(line: ParserBytes, tokens: *const ParserBytes, count: usize) -> u32 {
    unsafe fn bytes<'a>(value: &ParserBytes) -> Option<&'a [u8]> {
        if value.length > 1024 * 1024 || (value.length != 0 && value.bytes.is_null()) {
            None
        } else if value.length == 0 {
            Some(&[])
        } else {
            Some(std::slice::from_raw_parts(value.bytes, value.length))
        }
    }
    if count > 4096 || (count != 0 && tokens.is_null()) {
        return 16;
    }
    let Some(line) = bytes(&line) else { return 16 };
    let native = if count == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(tokens, count)
    };
    let Some(tokens) = native.iter().map(|t| bytes(t)).collect::<Option<Vec<_>>>() else {
        return 16;
    };
    compare_parser(line, &tokens)
}

#[repr(C)]
pub struct Observation {
    pub text: u32,
    pub slot: u32,
    pub counter: u32,
    pub total: u32,
    pub after_counter: u32,
    pub status: u32,
    pub before: [u8; 12],
    pub after: [u8; 12],
    pub bytes: *const u8,
    pub length: usize,
    pub consumed: u32,
}

fn compare(o: &Observation, bytes: &[u8]) -> u32 {
    // persistence §3.1: every shipped package/save uses native order 1.
    let mut record = SerialRecord::new(Cursor::new(bytes.to_vec()), true);
    let mut output = o.before;
    let mut binary = BinaryIo::new(&mut record);
    binary.counter = u64::from(o.counter);
    binary.total = u64::from(o.total);
    let mut text_record = SerialRecord::new(Cursor::new(bytes.to_vec()), true);
    let mut text = TextIo::new(&mut text_record);
    text.counter = u64::from(o.counter);
    text.total = u64::from(o.total);
    macro_rules! number {
        ($ty:ty) => {{
            let n = std::mem::size_of::<$ty>();
            let mut value = <$ty>::decode(&o.before[..n]);
            let status = if o.text != 0 {
                text.read_number(&mut value)
            } else {
                binary.read_number(&mut value)
            };
            value.encode(&mut output[..n]);
            (status, n)
        }};
    }
    let (status, width) = match o.slot {
        1 => {
            let mut d = Descriptor {
                id: u32::decode(&o.before[..4]),
                ty: u16::decode(&o.before[4..6]),
                count: u16::decode(&o.before[6..8]),
            };
            let status = if o.text != 0 {
                text.read_definition(&mut d)
            } else {
                binary.read_definition(&mut d)
            };
            d.id.encode(&mut output[..4]);
            d.ty.encode(&mut output[4..6]);
            d.count.encode(&mut output[6..8]);
            (status, 8)
        }
        2 => {
            let mut key = Key {
                ty: u32::decode(&o.before[..4]),
                group: u32::decode(&o.before[4..8]),
                instance: u32::decode(&o.before[8..12]),
            };
            let status = if o.text != 0 {
                text.read_key(&mut key)
            } else {
                binary.read_key(&mut key)
            };
            key.ty.encode(&mut output[..4]);
            key.group.encode(&mut output[4..8]);
            key.instance.encode(&mut output[8..12]);
            (status, 12)
        }
        4 => number!(f64),
        5 => number!(f32),
        6 => number!(i64),
        7 => number!(u64),
        8 => number!(i32),
        9 => number!(u32),
        10 => number!(i16),
        11 => number!(u16),
        12 => number!(i8),
        13 => number!(u8),
        _ => return 16,
    };
    let (counter, consumed) = if o.text != 0 {
        (text.counter, text.record.raw().position())
    } else {
        (binary.counter, binary.record.raw().position())
    };
    u32::from(status != (o.status != 0))
        | (u32::from(output[..width] != o.after[..width]) << 1)
        | (u32::from(counter != u64::from(o.after_counter)) << 2)
        | (u32::from(consumed != u64::from(o.consumed)) << 3)
}

/// # Safety
/// `observation` and its captured bytes must remain readable for this call.
#[no_mangle]
pub unsafe extern "C" fn fc4_scres_shadow_compare(observation: *const Observation) -> u32 {
    let Some(o) = observation.as_ref() else {
        return 16;
    };
    if o.length > 1024 * 1024 || (o.length != 0 && o.bytes.is_null()) {
        return 16;
    }
    let bytes = if o.length == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(o.bytes, o.length)
    };
    compare(o, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_command_entry_preserves_variables_and_grouped_tokens() {
        assert_eq!(
            compare_parser(b"set x (hello world) # ignored", &[b"set", b"x", b"hello world"]),
            0
        );
        assert_eq!(compare_parser(b"set y ${x}", &[b"set", b"y", b"${x}"]), 0);
        assert_eq!(compare_parser(b"set x 12", &[b"set", b"x", b"13"]), 1);
        assert_eq!(compare_parser(b"set x (unterminated", &[b"set"]), 2);
        assert_eq!(compare_parser(b"# empty", &[]), 0);
    }

    fn observation(text: bool, slot: u32, bytes: &[u8], after: &[u8]) -> Observation {
        let mut o = Observation {
            text: u32::from(text),
            slot,
            counter: 4,
            total: 1000,
            after_counter: 4 + bytes.len() as u32,
            status: 1,
            before: [0xA5; 12],
            after: [0xA5; 12],
            bytes: bytes.as_ptr(),
            length: bytes.len(),
            consumed: bytes.len() as u32,
        };
        o.after[..after.len()].copy_from_slice(after);
        o
    }

    #[test]
    fn compares_native_byte_values_and_independent_failure_channels() {
        let b = [0x78, 0x56, 0x34, 0x12];
        let mut o = observation(false, 9, &b, &b);
        assert_eq!(compare(&o, &b), 0);
        o.after[0] ^= 1;
        assert_eq!(compare(&o, &b), 2);
        o.status = 0;
        o.after_counter += 1;
        assert_eq!(compare(&o, &b), 7);
    }

    #[test]
    fn text_narrowing_and_definition_read_use_real_delimited_bytes() {
        let b = b";comment\r-257, ";
        let mut o = observation(true, 12, &b[..b.len() - 1], &[255]);
        assert_eq!(compare(&o, &b[..b.len() - 1]), 0);
        let b = b"name[ 42, 65539, 2 ]=";
        o = observation(true, 1, b, &[42, 0, 0, 0, 3, 0, 2, 0]);
        assert_eq!(compare(&o, b), 0);
    }

    #[test]
    fn excluded_final_byte_preserves_output_and_advances_counter() {
        let mut o = observation(false, 9, &[], &[0xA5; 4]);
        o.total = 8;
        o.after_counter = 8;
        o.status = 0;
        assert_eq!(compare(&o, &[]), 0);
    }

    #[test]
    fn constructed_inputs_compare_cursor_without_discarding_unconsumed_bytes() {
        let bytes = b"12suffix, trailing";
        let mut o = observation(true, 9, bytes, &12_u32.to_le_bytes());
        o.consumed = 9;
        o.after_counter = o.counter + o.consumed;
        assert_eq!(compare(&o, bytes), 0);
        o.consumed += 1;
        assert_eq!(compare(&o, bytes), 8);
    }
}
