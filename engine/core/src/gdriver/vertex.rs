// SPDX-License-Identifier: GPL-3.0-or-later
//! graphics-formats §3.5 and renderer-driver §8.5. Pure shared-format forwards.
use super::backend::{Error, Result};
pub const SHIFTS: [u32; 10] = [0, 2, 5, 6, 7, 8, 10, 14, 18, 22];
pub const MASKS: [u32; 10] = [3, 7, 1, 1, 1, 3, 15, 15, 15, 15];
pub const SIZES: [u32; 10] = [12, 4, 4, 12, 4, 4, 4, 8, 12, 16];
pub fn pack(format: u32) -> u32 {
    if format & 0x8000_0000 != 0 {
        return format;
    }
    match format {
        1 => 0x8000_0101,
        2 => 0x8000_4001,
        3 => 0x8000_8001,
        10 => 0x8000_4101,
        11 => 0x8000_8101,
        32 => 0x8000_0001,
        33 => 0x8000_0041,
        34 => 0x8000_0141,
        35 => 0x8000_4041,
        36 => 0x8000_8041,
        37 => 0x8000_4141,
        38 => 0x8000_8141,
        _ => 0,
    }
}
pub fn elements(format: u32, kind: usize) -> Result<u32> {
    if kind >= 10 {
        return Err(Error::InvalidInput);
    }
    Ok((pack(format) >> SHIFTS[kind]) & MASKS[kind])
}
pub fn make(kinds: &[u8]) -> Result<u32> {
    let mut counts = [0u32; 10];
    for &k in kinds {
        let count = counts.get_mut(k as usize).ok_or(Error::InvalidInput)?;
        *count = count.wrapping_add(1);
    }
    Ok(counts
        .iter()
        .enumerate()
        .fold(0x8000_0000, |v, (k, n)| v | n.wrapping_shl(SHIFTS[k])))
}
pub fn stride(format: u32) -> u32 {
    let f = pack(format);
    (0..10)
        .filter(|k| f & 15 != 0 || matches!(k, 0 | 5 | 6 | 7))
        .map(|k| ((f >> SHIFTS[k]) & MASKS[k]) * SIZES[k])
        .sum()
}
pub fn offset(format: u32, kind: usize, index: u32) -> Result<u32> {
    if kind >= 10 {
        return Err(Error::InvalidInput);
    }
    // graphics-formats §3.5 explicitly chooses plain kind-order layout.
    Ok((0..kind)
        .map(|k| elements(format, k).unwrap_or(0) * SIZES[k])
        .sum::<u32>()
        + index.wrapping_mul(SIZES[kind]))
}
pub fn fvf(format: u32) -> u32 {
    let f = pack(format);
    (u32::from(f & 3 != 0) * 2)
        | (u32::from(f & 0x300 != 0) * 0x40)
        | match (f >> 14) & 15 {
            0 => 0,
            1 => 0x100,
            _ => 0x200,
        }
}
