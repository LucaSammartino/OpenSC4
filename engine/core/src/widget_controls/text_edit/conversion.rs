// SPDX-License-Identifier: GPL-3.0-or-later
//! TE5 counted conversion. Selectors 16 count words; all other selectors count bytes.
//! Windows mapping tables and API success are explicit environment inputs.
use super::{encoding::Encoding, Result, Unsupported};

pub trait CodePages {
    fn system_code_page(&mut self) -> i32;
    fn multi_byte_to_wide(&mut self, page: i32, flags: u32, source: &[u8], output: &mut [u16]) -> usize;
    fn wide_to_multi_byte(&mut self, page: i32, flags: u32, source: &[u16], output: &mut [u8], default: u8) -> usize;
    /// Uninitialized fourth word read by the native three-word temporary.
    /// Supply a controlled memory value when that particular branch is reached.
    fn composition_tail(&mut self) -> u16;
    fn temporary_buffer(&mut self, _bytes: u32, _heap: bool) {}
    fn free_temporary(&mut self) {}
}

fn bad() -> Unsupported {
    Unsupported::Precondition("TE5 readable/writable counted conversion span")
}
fn resolve(selector: i32, host: &mut (impl CodePages + ?Sized)) -> i32 {
    match selector {
        -2 => 8,
        -1 => host.system_code_page(),
        other => other,
    }
}
fn words(source: &[u8], count: usize) -> Result<Vec<u16>> {
    Ok(source
        .get(..count.checked_mul(2).ok_or_else(bad)?)
        .ok_or_else(bad)?
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect())
}

/// `aliased` models pointer identity without manufacturing overlapping Rust borrows.
/// In that case `source` is the pre-call snapshot, and `output` is the same storage.
#[allow(clippy::too_many_arguments)] // Native counted selector pair plus alias/environment inputs.
pub fn convert(
    source: &[u8],
    count: usize,
    from: i32,
    output: &mut [u8],
    capacity: &mut usize,
    to: i32,
    aliased: bool,
    host: &mut (impl CodePages + ?Sized),
) -> Result<usize> {
    if count == 0 {
        *capacity = 0;
        return Ok(0);
    }
    let mut from = resolve(from, host);
    let to = resolve(to, host);
    if from != 16 && source.get(..count).ok_or_else(bad)?.iter().all(|&b| b <= 0x7f) {
        from = 0;
    }
    if from == to || (from == 0 && to != 16) || (from != 16 && to == 0) {
        if aliased {
            *capacity = count;
            return Ok(count);
        }
        let n = count.min(*capacity);
        let bytes = n.checked_mul(if to == 16 { 2 } else { 1 }).ok_or_else(bad)?;
        output
            .get_mut(..bytes)
            .ok_or_else(bad)?
            .copy_from_slice(source.get(..bytes).ok_or_else(bad)?);
        *capacity = n;
        return Ok(n);
    }
    if from == 16 && to == 0 {
        let n = count.min(*capacity);
        for (out, word) in output.get_mut(..n).ok_or_else(bad)?.iter_mut().zip(words(source, n)?) {
            *out = word as u8;
        }
        *capacity = n;
        return Ok(n);
    }
    if aliased {
        return Ok(0);
    }
    let limit = *capacity;
    let (consumed, produced) = match (from, to) {
        (0, 16) => {
            let n = count.min(limit);
            for (i, &byte) in source.get(..n).ok_or_else(bad)?.iter().enumerate() {
                output
                    .get_mut(i * 2..i * 2 + 2)
                    .ok_or_else(bad)?
                    .copy_from_slice(&(byte as u16).to_le_bytes());
            }
            (n, n)
        }
        (8, 16) => decode(source.get(..count).ok_or_else(bad)?, output, limit)?,
        (16, 8) => encode(&words(source, count)?, output, limit)?,
        (page, 16) => {
            let mut wide = words(output, limit)?;
            let n = host.multi_byte_to_wide(page, 0, source.get(..count).ok_or_else(bad)?, &mut wide);
            if n > limit {
                return Err(bad());
            }
            for (i, word) in wide.iter().enumerate() {
                output
                    .get_mut(i * 2..i * 2 + 2)
                    .ok_or_else(bad)?
                    .copy_from_slice(&word.to_le_bytes());
            }
            (if n == 0 { 0 } else { count }, n)
        }
        (16, page) => {
            let n = host.wide_to_multi_byte(
                page,
                0,
                &words(source, count)?,
                output.get_mut(..limit).ok_or_else(bad)?,
                b' ',
            );
            if n > limit {
                return Err(bad());
            }
            (if n == 0 { 0 } else { count }, n)
        }
        (8, _) | (_, 8) => {
            // Native stack/heap choice is based on this wrapping byte quantity,
            // which is also (quirkily) passed as a word capacity.
            let temporary = (count as u32).wrapping_mul(2).wrapping_add(4) as usize;
            host.temporary_buffer(temporary as u32, temporary > 1024);
            // The scratch model has room for the advertised word capacity;
            // temporary_buffer records the actual native allocation in bytes.
            let mut wide = std::mem::ManuallyDrop::new(vec![0; temporary.checked_mul(2).ok_or_else(bad)?]);
            let mut n = temporary;
            if convert(source, count, from, &mut wide, &mut n, 16, false, host)? != count {
                drop(std::mem::ManuallyDrop::into_inner(wide));
                if temporary > 1024 {
                    host.free_temporary();
                }
                *capacity = 0;
                return Ok(0);
            }
            let mut produced = limit;
            if convert(&wide, n, 16, output, &mut produced, to, false, host)? != n {
                drop(std::mem::ManuallyDrop::into_inner(wide));
                if temporary > 1024 {
                    host.free_temporary();
                }
                *capacity = 0;
                return Ok(0);
            }
            drop(std::mem::ManuallyDrop::into_inner(wide));
            if temporary > 1024 {
                host.free_temporary();
            }
            (count, produced)
        }
        _ => (0, 0),
    };
    *capacity = produced;
    Ok(consumed)
}

fn decode(source: &[u8], output: &mut [u8], capacity: usize) -> Result<(usize, usize)> {
    let (mut at, mut n) = (0, 0);
    while at < source.len() && n < capacity {
        let first = source[at];
        let length = match first {
            0..=0x7f => 1,
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf7 => 4,
            0xf8..=0xfb => 5,
            0xfc..=0xfd => 6,
            _ => break,
        };
        let span = match source.get(at..at + length) {
            Some(span) => span,
            None => break,
        };
        if span[1..].iter().any(|&b| !(0x80..=0xbf).contains(&b)) {
            break;
        }
        if length > 1
            && matches!(first, 0xe0 | 0xf0 | 0xf8 | 0xfc)
            && span[1]
                < match first {
                    0xe0 => 0xa0,
                    0xf0 => 0x90,
                    0xf8 => 0x88,
                    _ => 0x84,
                }
        {
            break;
        }
        let mut value = (first & (0x7f >> length.min(6))) as u32;
        if length == 1 {
            value = first as u32;
        }
        for &byte in &span[1..] {
            value = (value << 6) | (byte & 0x3f) as u32;
        }
        output
            .get_mut(n * 2..n * 2 + 2)
            .ok_or_else(bad)?
            .copy_from_slice(&(value as u16).to_le_bytes());
        at += length;
        n += 1;
    }
    Ok((at, n))
}

fn encode(source: &[u16], output: &mut [u8], capacity: usize) -> Result<(usize, usize)> {
    let (mut consumed, mut n) = (0, 0);
    for &word in source {
        consumed += 1; // Native increments before checking whether this word fits.
        let bytes = if word < 0x80 {
            vec![word as u8]
        } else if word < 0x800 {
            vec![0xc0 | (word >> 6) as u8, 0x80 | (word & 63) as u8]
        } else {
            vec![
                0xe0 | (word >> 12) as u8,
                0x80 | ((word >> 6) & 63) as u8,
                0x80 | (word & 63) as u8,
            ]
        };
        if n + bytes.len() > capacity {
            break;
        }
        output
            .get_mut(n..n + bytes.len())
            .ok_or_else(bad)?
            .copy_from_slice(&bytes);
        n += bytes.len();
    }
    Ok((consumed, n))
}

pub fn character(source: &[u8], count: &mut usize, selector: i32, host: &mut (impl CodePages + ?Sized)) -> Result<u16> {
    if *count == 0 {
        return Ok(0);
    }
    if selector != 16 && *source.first().ok_or_else(bad)? < 0x7f {
        return Ok(source[0] as u16);
    }
    let mut output = [0; 2];
    let mut capacity = 1;
    *count = convert(source, *count, selector, &mut output, &mut capacity, 16, false, host)?;
    Ok(u16::from_le_bytes(output))
}

pub fn previous_character(
    prefix: &[u8],
    count: &mut usize,
    selector: i32,
    host: &mut (impl CodePages + ?Sized),
) -> Result<u16> {
    if *count == 0 {
        return Ok(0);
    }
    if selector != 16 && *prefix.last().ok_or_else(bad)? < 0x7f {
        return Ok(*prefix.last().ok_or_else(bad)? as u16);
    }
    let unit = if selector == 16 { 2 } else { 1 };
    let available = prefix
        .get(prefix.len().checked_sub(*count * unit).ok_or_else(bad)?..)
        .ok_or_else(bad)?;
    let encoding = Encoding {
        selector,
        system_selector: if selector == -1 { host.system_code_page() } else { 0 },
    };
    *count = encoding.previous_length(available);
    let start = prefix.len().checked_sub(*count * unit).ok_or_else(bad)?;
    let mut output = [0; 2];
    let mut capacity = 1;
    *count = convert(
        &prefix[start..],
        *count,
        selector,
        &mut output,
        &mut capacity,
        16,
        false,
        host,
    )?;
    Ok(u16::from_le_bytes(output))
}
