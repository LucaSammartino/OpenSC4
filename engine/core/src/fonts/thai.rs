// SPDX-License-Identifier: GPL-3.0-or-later
//! Thai presentation forms (drawing-fonts.md §11).

/// `GetSubstitutionChar(ch, form)` (`0x0089645A`): Private Use glyph or `ch`.
pub fn substitution_char(ch: u16, form: u32) -> u16 {
    match (ch, form) {
        (0x0E0D, 1) => 0xF70F,
        (0x0E31, 1) => 0xF710,
        (0x0E34..=0x0E37, 1) => 0xF701 + (ch - 0x0E34),
        (0x0E47, 1) => 0xF712,
        (0x0E48..=0x0E4C, 1) => 0xF713 + (ch - 0x0E48),
        (0x0E48..=0x0E4C, 2) => 0xF70A + (ch - 0x0E48),
        (0x0E48..=0x0E4C, 3) => 0xF705 + (ch - 0x0E48),
        (0x0E4D, 1) => 0xF711,
        _ => ch,
    }
}

/// `UTF16GetThaiCharType(ch)` (`0x0091169F`).
pub fn thai_char_type(ch: u16) -> u32 {
    match ch {
        0x0E01..=0x0E23 | 0x0E25 | 0x0E27..=0x0E2E => 1,
        0x0E40..=0x0E44 => 2,
        0x0E30 | 0x0E32 | 0x0E33 => 3,
        0x0E45 => 4,
        0x0E24 | 0x0E26 => 5,
        0x0E38 => 6,
        0x0E39 => 7,
        0x0E3A => 8,
        0x0E48..=0x0E4B => 9,
        0x0E4C | 0x0E4D => 10,
        0x0E47 => 11,
        0x0E4E => 12,
        0x0E34 => 13,
        0x0E31 | 0x0E36 => 14,
        0x0E35 | 0x0E37 => 15,
        0x0E50..=0x0E59 => 16,
        0x0E2F | 0x0E3F | 0x0E46 | 0x0E4F | 0x0E5A | 0x0E5B => 17,
        _ => 0,
    }
}

fn is_tall(prev: u16) -> bool {
    matches!(prev, 0x0E1B | 0x0E1D | 0x0E1F | 0x0E2C)
}

/// `MakeSubstitutedString` (`0x00899665`): returns the output units and, per output unit, the index
/// of its source unit in `text`.
pub fn make_substituted_string(text: &[u16]) -> (Vec<u16>, Vec<u32>) {
    // First loop: split SARA AM.
    let mut split = Vec::with_capacity(text.len() + 4);
    let mut map = Vec::with_capacity(text.len() + 4);
    for (i, &c) in text.iter().enumerate() {
        if c == 0x0E33 {
            split.push(0x0E4D);
            split.push(0x0E32);
            map.push(i as u32);
            map.push(i as u32);
        } else {
            split.push(c);
            map.push(i as u32);
        }
    }
    // Second loop, decisions on the split (unsubstituted) characters.
    let orig = split.clone();
    let mut carry = 0u32;
    for i in 0..orig.len() {
        let cur = orig[i];
        let prev = if i == 0 { 0 } else { orig[i - 1] };
        let next = orig.get(i + 1).copied().unwrap_or(0);
        let cc = thai_char_type(cur);
        let tall = is_tall(prev);
        if cur == 0x0E0D && (6..=8).contains(&thai_char_type(next)) {
            split[i] = substitution_char(0x0E0D, 1);
            carry = 0;
        } else if cc == 6 || cc == 7 {
            carry = u32::from(tall);
        } else if cc == 8 {
            carry = 0;
        } else if (9..=11).contains(&cc) || (13..=15).contains(&cc) {
            let mut form = if carry == 1 || tall { 1 } else { 0 };
            if (cc == 9 || cur == 0x0E4C) && thai_char_type(prev) < 10 {
                form += 2;
            }
            if form != 0 {
                split[i] = substitution_char(cur, form);
            }
            carry = form;
        } else {
            carry = 0;
        }
    }
    (split, map)
}
