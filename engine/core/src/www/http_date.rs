//! Fixed-position CRT date parser, web-network-http-wire §4.
use super::{
    cookies::decimal,
    notification::{time_to_seconds, Calendar, Runtime},
    Contract, Unsupported,
};

fn tokens(bytes: &[u8]) -> Vec<&[u8]> {
    bytes.split(u8::is_ascii_whitespace).filter(|p| !p.is_empty()).collect()
}
fn index(value: &[u8], names: &[&[u8]]) -> i32 {
    names
        .iter()
        .position(|name| value.eq_ignore_ascii_case(name))
        .map_or(0, |i| i as i32 + 1)
}
pub fn date_type(text: &[u8]) -> u32 {
    if text.len() >= 48 {
        return 0;
    }
    match text.iter().position(|b| *b == b',') {
        Some(3) if text.len() == 29 => {
            let mut copy = text.to_vec();
            copy[7] = b' ';
            copy[11] = b' ';
            if tokens(text).len() >= 6 || tokens(&copy).len() >= 6 {
                1
            } else {
                0
            }
        }
        Some(c) if c != 3 && (30..=35).contains(&text.len()) && tokens(text).len() >= 4 => {
            match tokens(text)[1].len() {
                9 => 2,
                11 => 3,
                _ => 0,
            }
        }
        None if text.len() == 24 && tokens(text).len() >= 5 => 4,
        _ => 0,
    }
}
pub fn parse_date(text: &[u8]) -> Contract<Option<(u32, Calendar)>> {
    let kind = date_type(text);
    if kind == 0 {
        return Ok(None);
    }
    let mut copy = text.to_vec();
    let positions = match kind {
        1 => vec![3, 7, 11, 19, 22],
        2 | 3 => {
            let c = text.iter().position(|b| *b == b',').expect("classified comma");
            if c + 17 >= text.len() {
                return Ok(None);
            }
            if kind == 2 {
                vec![c, c + 4, c + 8, c + 14, c + 17]
            } else {
                vec![c, c + 4, c + 8, c + 16, c + 19]
            }
        }
        _ => {
            let Some(p) = text.iter().position(|b| *b == b':') else {
                return Err(Unsupported::NativeInputDomain("type-4 date substitution"));
            };
            vec![p, p + 2]
        }
    };
    for p in positions {
        let Some(byte) = copy.get_mut(p) else {
            return Err(Unsupported::NativeInputDomain("date substitution outside allocation"));
        };
        *byte = b' ';
    }
    let words = tokens(&copy);
    if words.len() < if kind == 4 { 7 } else { 8 } {
        return Ok(None);
    }
    let (day, month, year, hour, minute, second) = if kind == 4 {
        (words[2], words[1], words[6], words[3], words[4], words[5])
    } else {
        (words[1], words[2], words[3], words[4], words[5], words[6])
    };
    let weekday = if kind == 2 || kind == 3 {
        index(
            words[0],
            &[
                b"Monday",
                b"Tuesday",
                b"Wednesday",
                b"Thursday",
                b"Friday",
                b"Saturday",
                b"Sunday",
            ],
        )
    } else {
        index(words[0], &[b"Mon", b"Tue", b"Wed", b"Thu", b"Fri", b"Sat", b"Sun"])
    };
    let month = index(
        month,
        &[
            b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
        ],
    );
    let mut year = decimal(year) as u32;
    if year < 50 {
        year += 2000;
    } else if year < 100 {
        year += 1900;
    }
    let preceding: [u32; 13] = [0x00636544, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let day = decimal(day);
    let yearday = preceding[month as usize]
        .wrapping_add(day as u32)
        .wrapping_add(1)
        .wrapping_add(u32::from(leap));
    Ok(Some((
        kind,
        Calendar {
            weekday,
            day,
            month,
            year: year as i32,
            yearday: yearday as i32,
            hour: decimal(hour),
            minute: decimal(minute),
            second: decimal(second),
        },
    )))
}
pub fn seconds(runtime: &mut impl Runtime, text: &[u8]) -> Contract<u32> {
    Ok(parse_date(text)?.map_or(0, |(_, calendar)| time_to_seconds(runtime, calendar)))
}
