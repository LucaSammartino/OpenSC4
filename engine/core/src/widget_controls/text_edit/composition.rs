// SPDX-License-Identifier: GPL-3.0-or-later
//! TE5 Thai classes and asymmetric composed-character walks.
use super::{
    conversion::{self, CodePages},
    encoding::Encoding,
    Result, Unsupported,
};

pub fn thai_class(word: u16) -> u32 {
    match word {
        0x0e01..=0x0e23 | 0x0e25 | 0x0e27..=0x0e2e => 1,
        0x0e40..=0x0e44 => 2,
        0x0e30 | 0x0e32 | 0x0e33 => 3,
        0x0e45 => 4,
        0x0e24 | 0x0e26 => 5,
        0x0e38 => 6,
        0x0e39 => 7,
        0x0e3a => 8,
        0x0e48..=0x0e4b => 9,
        0x0e4c | 0x0e4d => 10,
        0x0e47 => 11,
        0x0e4e => 12,
        0x0e34 => 13,
        0x0e31 | 0x0e36 => 14,
        0x0e35 | 0x0e37 => 15,
        0x0e50..=0x0e59 => 16,
        0x0e2f | 0x0e3f | 0x0e46 | 0x0e4f | 0x0e5a | 0x0e5b => 17,
        _ => 0,
    }
}

pub fn thai_level(current: u16, previous: u16, following: u16) -> u32 {
    let previous = thai_class(previous);
    match thai_class(current) {
        0..=5 | 16 | 17 => {
            if current == 0x0e33 && !matches!(previous, 1 | 9) {
                0
            } else {
                4
            }
        }
        6..=8 => {
            if previous == 1 {
                5
            } else {
                0
            }
        }
        9 => match previous {
            13..=15 => 2,
            1 if following == 0x0e33 => 2,
            1 | 6 | 7 => 3,
            _ => 0,
        },
        10..=11 => match previous {
            1 | 6 | 7 => 3,
            13..=15 => 2,
            _ => 0,
        },
        12..=15 => {
            if previous == 1 {
                3
            } else {
                0
            }
        }
        _ => 1,
    }
}

pub fn decoration(word: u16) -> bool {
    matches!(word, 0x0e31 | 0x0e34..=0x0e3a | 0x0e47..=0x0e4e)
}

fn bad() -> Unsupported {
    Unsupported::Precondition("TE5 readable composed-character storage")
}

pub fn thai_group(readable: &[u16], count: usize) -> Result<usize> {
    if count < 2 {
        return Ok(count);
    }
    let second = *readable.get(1).ok_or_else(bad)?;
    if second == 0x0e33 {
        return Ok(2);
    }
    if !decoration(second) {
        return Ok(1);
    }
    if count < 3 {
        return Ok(2);
    }
    let third = *readable.get(2).ok_or_else(bad)?;
    if third == 0x0e33 {
        return Ok(3);
    }
    if !decoration(third) {
        return Ok(2);
    }
    // Literal native quirk: this read is not bounded by count == 3.
    Ok(if *readable.get(3).ok_or_else(bad)? == 0x0e33 {
        4
    } else {
        3
    })
}

fn encoding(selector: i32, host: &mut (impl CodePages + ?Sized)) -> Encoding {
    Encoding {
        selector,
        system_selector: if selector == -1 { host.system_code_page() } else { 0 },
    }
}

pub fn forward(source: &[u8], count: usize, selector: i32, host: &mut (impl CodePages + ?Sized)) -> Result<usize> {
    if count == 0 {
        return Ok(0);
    }
    if selector == 16 {
        let words: Vec<u16> = source
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        let first = *words.first().ok_or_else(bad)?;
        return if (0x0e00..=0x0e7f).contains(&first) {
            thai_group(&words, count)
        } else {
            Ok(1)
        };
    }
    if selector == 8 {
        if *source.first().ok_or_else(bad)? <= 0x7f {
            return Ok(1);
        }
        let mut output = [0u8; 6];
        let mut produced = 3;
        let consumed = conversion::convert(source, count, 8, &mut output, &mut produced, 16, false, host)?;
        let mut words: Vec<u16> = output
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        if consumed != 0 && (0x0e00..=0x0e7f).contains(&words[0]) {
            if produced == 3 && decoration(words[1]) && decoration(words[2]) {
                words.push(host.composition_tail());
            }
            let glyphs = thai_group(&words, produced)?;
            let encoding = encoding(selector, host);
            let mut at = 0;
            for _ in 0..glyphs {
                at += encoding.char_length(source.get(at..).ok_or_else(bad)?, count.saturating_sub(at))?;
            }
            return Ok(at);
        }
    }
    encoding(selector, host).char_length(source, count)
}

pub fn backward(prefix: &[u8], count: usize, selector: i32, host: &mut (impl CodePages + ?Sized)) -> Result<usize> {
    if count == 0 {
        return Ok(0);
    }
    if selector == 16 {
        let words: Vec<u16> = prefix
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        let mut traversed = 0;
        loop {
            let at = words.len().checked_sub(traversed + 1).ok_or_else(bad)?;
            traversed += 1;
            if !decoration(words[at]) || traversed >= count {
                return Ok(traversed);
            }
        }
    }
    let encoding = encoding(selector, host);
    if selector != 8 {
        return Ok(encoding.previous_length(
            prefix
                .get(prefix.len().checked_sub(count).ok_or_else(bad)?..)
                .ok_or_else(bad)?,
        ));
    }
    if *prefix.last().ok_or_else(bad)? <= 0x7f {
        return Ok(1);
    }
    let mut traversed = 0;
    let mut word = ((count as u32) >> 16) as u16;
    loop {
        let end = prefix.len().checked_sub(traversed).ok_or_else(bad)?;
        let start = end.checked_sub(count - traversed).ok_or_else(bad)?;
        let length = encoding.previous_length(&prefix[start..end]);
        let begin = end.checked_sub(length).ok_or_else(bad)?;
        let mut output = word.to_le_bytes(); // Aliases the high half, retaining prior value on no write.
        let mut capacity = 1;
        conversion::convert(
            &prefix[begin..end],
            length,
            8,
            &mut output,
            &mut capacity,
            16,
            false,
            host,
        )?;
        word = u16::from_le_bytes(output);
        traversed += length;
        if !decoration(word) || traversed >= count {
            return Ok(traversed);
        }
    }
}
