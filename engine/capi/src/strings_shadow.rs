// SPDX-License-Identifier: GPL-3.0-or-later
//! Private, stateless transport for constructed ORIGINAL string comparisons.
//! Native outputs never enter the production core. No public DLL ABI additions.
use fc4_core::language::{FormatArg, FormatPrimitives, RzString, StringFault, StringSource};

struct NoFloatingPoint;
impl FormatPrimitives for NoFloatingPoint {
    fn float_digits(&self, _: u8, _: f64, _: Option<u32>, _: bool) -> Result<Vec<u8>, StringFault> {
        // RZS7 finite decimal extraction is deliberately outside this shadow.
        Err(StringFault::Unsupported("RZS7 excluded"))
    }
    fn pointer_digits(&self, _: u32, _: Option<u32>) -> Result<Vec<u8>, StringFault> {
        Err(StringFault::Unsupported("pointer formatting excluded"))
    }
}

fn run(op: u32, input: &[u8], other: &[u8], n: u32) -> Result<(Vec<u8>, i32), StringFault> {
    let mut s = RzString::from_bytes(input);
    let mut scalar = 0;
    match op {
        0 => s.from_char(Some(input), input.len() as u32),
        1 => s.copy(&RzString::from_bytes(input)),
        2 => s.resize(n),
        3 => {
            s.append(other);
        }
        4 => {
            s.insert(n, other)?;
        }
        5 => {
            s.replace(n, other, 3)?;
        }
        6 => {
            s.erase(n, 3)?;
        }
        7 | 8 => scalar = i32::from(s.is_equal(other, other.len() as u32, op == 7)?),
        9 | 10 => scalar = s.compare_to(other, other.len() as u32, op == 9)?,
        11 | 12 => scalar = s.find(other, n, op == 11)? as i32,
        13 | 14 => scalar = s.rfind(other, n, op == 13)? as i32,
        15 => {
            s.sprintf(
                b"%s|%+08d|%#010x|%%",
                &[
                    FormatArg::Bytes(input),
                    FormatArg::Signed(i64::from(n)),
                    FormatArg::Unsigned(u64::from(n)),
                ],
                &NoFloatingPoint,
            )?;
        }
        _ => return Err(StringFault::Argument),
    }
    Ok((s.bytes().to_vec(), scalar))
}

// Safety: the private plugin caller supplies live counted buffers (at most 96
// bytes), output capacity 512, and valid scalar/length pointers. No retained state.
#[no_mangle]
unsafe extern "C" fn fc4_strings_shadow_call(
    op: u32,
    input: *const u8,
    input_len: usize,
    other: *const u8,
    other_len: usize,
    n: u32,
    out: *mut u8,
    capacity: usize,
    length: *mut usize,
    scalar: *mut i32,
) -> i32 {
    if input.is_null()
        || other.is_null()
        || out.is_null()
        || length.is_null()
        || scalar.is_null()
        || input_len > 96
        || other_len > 96
        || n > 4096
    {
        return 0;
    }
    let a = unsafe { std::slice::from_raw_parts(input, input_len) };
    let b = unsafe { std::slice::from_raw_parts(other, other_len) };
    let Ok((bytes, value)) = run(op, a, b, n) else {
        return 0;
    };
    if bytes.len() > capacity {
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
        *length = bytes.len();
        *scalar = value;
    }
    1
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn counted_nul_and_c_string_replace_remain_distinct() {
        assert_eq!(run(3, b"A\0B", b"X\0Y", 0).unwrap().0, b"A\0BX\0Y");
        assert_eq!(run(5, b"abcdef", b"X\0Y", 1).unwrap().0, b"aXef");
        assert_eq!(run(7, b"A\0X", b"A\0Y", 0).unwrap().1, 1);
        assert_eq!(run(8, b"A\0X", b"A\0Y", 0).unwrap().1, 0);
    }

    #[test]
    fn search_boundaries_and_integer_format_use_production_core() {
        assert_eq!(run(11, b"abAB", b"AB", 0).unwrap().1, 2);
        assert_eq!(run(12, b"abAB", b"AB", 0).unwrap().1, 0);
        assert_eq!(run(13, b"abAB", b"", 99).unwrap().1, 4);
        assert_eq!(run(15, b"Oak", b"", 42).unwrap().0, b"Oak|+0000042|0x0000002a|%");
    }
}
