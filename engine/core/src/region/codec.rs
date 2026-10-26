// SPDX-License-Identifier: GPL-3.0-or-later
//! Region-owned records: INI, 12-byte location, decoded picture; no invented region save record.

use crate::rz_files::ini::{parse_line, scan_long, IniLine};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordError {
    Truncated,
    Invalid,
    Unsupported,
    TooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    pub x: i32,
    pub z: i32,
    pub size_exp: i32,
}

impl Location {
    pub fn encode(self) -> [u8; 12] {
        let mut out = [0; 12];
        for (chunk, value) in out.chunks_exact_mut(4).zip([self.x, self.z, self.size_exp]) {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        out
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, RecordError> {
        if bytes.len() != 12 {
            return Err(RecordError::Truncated);
        }
        let value = |i| i32::from_le_bytes(bytes[i..i + 4].try_into().expect("four bytes"));
        Ok(Self {
            x: value(0),
            z: value(4),
            size_exp: value(8),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    width: usize,
    height: usize,
    /// Logical RGB, row-major with the top row first. Alpha is irrelevant to region locations.
    pixels: Vec<[u8; 3]>,
}

impl Image {
    pub fn new(width: usize, height: usize, pixels: Vec<[u8; 3]>) -> Result<Self, RecordError> {
        if width > i32::MAX as usize || height > i32::MAX as usize {
            return Err(RecordError::TooLarge);
        }
        if width.checked_mul(height) != Some(pixels.len()) {
            return Err(RecordError::Invalid);
        }
        Ok(Self { width, height, pixels })
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn pixels(&self) -> &[[u8; 3]] {
        &self.pixels
    }
    pub fn locations(&self) -> Vec<Location> {
        let mut visited = vec![false; self.pixels.len()];
        let mut locations = Vec::new();
        for z in 0..self.height {
            for x in 0..self.width {
                if visited[z * self.width + x] {
                    continue;
                }
                let [r, g, b] = self.pixels[z * self.width + x];
                let code = usize::from(r == 255) + 2 * usize::from(g == 255) + 4 * usize::from(b == 255);
                if !matches!(code, 1 | 2 | 4) || x + code > self.width || z + code > self.height {
                    continue;
                }
                for row in z..z + code {
                    visited[row * self.width + x..row * self.width + x + code].fill(true);
                }
                locations.push(Location {
                    x: x as i32,
                    z: z as i32,
                    size_exp: code.trailing_zeros() as i32,
                });
            }
        }
        locations
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub terrain_type: i32,
    pub water_min: i32,
    pub water_max: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            terrain_type: 0,
            water_min: 60,
            water_max: 100,
        }
    }
}

const SECTION: &[u8] = b"Regional Settings";
const KEYS: [&[u8]; 4] = [b"Name", b"Terrain type", b"Water Min", b"Water Max"];

impl Config {
    /// Patch valid keys only. The returned nonempty Name still needs the host's LTEXT loader.
    pub fn apply_ini(&mut self, bytes: &[u8]) -> Option<String> {
        let mut values: [Option<Vec<u8>>; 4] = Default::default();
        let mut in_section = false;
        let mut entered = false;
        for (start, end, _) in line_ranges(bytes) {
            let line = &bytes[start..end];
            match parse_line(line) {
                IniLine::Header(section) => {
                    if entered {
                        break;
                    }
                    in_section = section.is_some_and(|s| s.eq_ignore_ascii_case(SECTION));
                    entered |= in_section;
                }
                IniLine::Entry { key, value } if in_section => {
                    for (i, expected) in KEYS.iter().enumerate() {
                        if key.eq_ignore_ascii_case(expected) && values[i].is_none() {
                            values[i] = Some(value.clone());
                        }
                    }
                }
                _ => {}
            }
        }
        for (index, bound, target) in [
            (1, 2, &mut self.terrain_type),
            (2, 255, &mut self.water_min),
            (3, 255, &mut self.water_max),
        ] {
            if let Some(value) = values[index].as_deref().and_then(scan_long) {
                if (0..bound).contains(&value) {
                    *target = value;
                }
            }
        }
        values[0]
            .take()
            .filter(|v| !v.is_empty())
            .map(|v| String::from_utf8_lossy(&v).into_owned())
    }
    /// The region's sequence of four cRZIniFile writes, for a previously missing or nonempty file.
    pub fn write_ini(&self, previous: &[u8], name: &str) -> Vec<u8> {
        let mut out = self.write_ini_entry((!previous.is_empty()).then_some(previous), name, 0);
        for index in 1..4 {
            out = self.write_ini_entry(Some(&out), name, index);
        }
        out
    }
    /// Byte-exact successful WriteEntry semantics (framework-rz-files.md §3.6); the host retains
    /// responsibility for individual read/write failures. None means a nonexistent file.
    pub fn write_ini_entry(&self, previous: Option<&[u8]>, name: &str, index: usize) -> Vec<u8> {
        let value = match index {
            0 => name.to_owned(),
            1 => self.terrain_type.to_string(),
            2 => self.water_min.to_string(),
            3 => self.water_max.to_string(),
            _ => return previous.unwrap_or_default().to_vec(),
        };
        let entry = [KEYS[index], b" = ", value.as_bytes(), b"\r\n"].concat();
        let header = b"[Regional Settings]\r\n";
        let Some(previous) = previous else {
            return [header.as_slice(), &entry].concat();
        };
        let ranges = line_ranges(previous);
        let section = ranges.iter().position(|&(start, end, _)| matches!(parse_line(&previous[start..end]), IniLine::Header(Some(s)) if s.eq_ignore_ascii_case(SECTION)));
        let Some(section) = section else {
            let separator: &[u8] = if previous.last().is_some_and(|b| matches!(b, b'\r' | b'\n')) {
                b"\r\n"
            } else {
                b"\r\n\r\n"
            };
            return [previous, separator, header, &entry].concat();
        };
        for &(start, end, next) in &ranges[section + 1..] {
            match parse_line(&previous[start..end]) {
                IniLine::Header(_) => return [&previous[..start], &entry, &previous[start..]].concat(),
                IniLine::Entry { key, .. } if key.eq_ignore_ascii_case(KEYS[index]) => {
                    return [&previous[..start], &entry, &previous[next..]].concat();
                }
                _ => {}
            }
        }
        [previous, &entry].concat()
    }
}

pub(super) fn localization_id(name: &str) -> Option<u32> {
    // RM15: at most eight hex digits after optional whitespace and 0x; ignore the tail.
    let text = name.strip_prefix('#')?.trim_start_matches(char::is_whitespace);
    let text = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    let digits: String = text.chars().take(8).take_while(char::is_ascii_hexdigit).collect();
    u32::from_str_radix(&digits, 16).ok().filter(|id| *id != 0)
}

/// The framework reader consumes an entire CR/LF run after each line.
fn line_ranges(bytes: &[u8]) -> Vec<(usize, usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < bytes.len() {
        let mut end = start;
        while end < bytes.len() && !matches!(bytes[end], b'\r' | b'\n') {
            end += 1;
        }
        let mut next = end;
        while next < bytes.len() && matches!(bytes[next], b'\r' | b'\n') {
            next += 1;
        }
        ranges.push((start, end, next));
        start = next;
    }
    ranges
}

/// Standalone BI_RGB BMP codec for the region's shipped 24-bit files and uncompressed 32-bit
/// variants. Other framework image formats are delegated to RegionHost::load_image.
pub fn decode_bmp(bytes: &[u8]) -> Result<Image, RecordError> {
    let u16_at = |i| {
        bytes
            .get(i..i + 2)
            .map(|b| u16::from_le_bytes(b.try_into().expect("two bytes")))
            .ok_or(RecordError::Truncated)
    };
    let u32_at = |i| {
        bytes
            .get(i..i + 4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("four bytes")))
            .ok_or(RecordError::Truncated)
    };
    if bytes.get(..2) != Some(b"BM") {
        return Err(RecordError::Invalid);
    }
    let offset = u32_at(10)? as usize;
    let header = u32_at(14)? as usize;
    if header < 40 {
        return Err(RecordError::Unsupported);
    }
    let width = u32_at(18)? as i32;
    let signed_height = u32_at(22)? as i32;
    let bpp = u16_at(28)?;
    if u16_at(26)? != 1 || !matches!(bpp, 24 | 32) || u32_at(30)? != 0 {
        return Err(RecordError::Unsupported);
    }
    let header_end = 14usize.checked_add(header).ok_or(RecordError::TooLarge)?;
    if width <= 0 || signed_height == 0 || signed_height == i32::MIN || offset < header_end {
        return Err(RecordError::Invalid);
    }
    let width = width as usize;
    let height = signed_height.unsigned_abs() as usize;
    let pixel_size = usize::from(bpp / 8);
    let stride = width
        .checked_mul(pixel_size)
        .and_then(|v| v.checked_add(3))
        .map(|v| v & !3)
        .ok_or(RecordError::TooLarge)?;
    let end = stride
        .checked_mul(height)
        .and_then(|v| offset.checked_add(v))
        .ok_or(RecordError::TooLarge)?;
    if end > bytes.len() {
        return Err(RecordError::Truncated);
    }
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        let row = if signed_height > 0 { height - y - 1 } else { y };
        for x in 0..width {
            let i = offset + row * stride + x * pixel_size;
            pixels.push([bytes[i + 2], bytes[i + 1], bytes[i]]);
        }
    }
    Image::new(width, height, pixels)
}

/// Canonical 24-bit bottom-up BMP (same-input decode/encode/decode is lossless for RGB pixels).
pub fn encode_bmp(image: &Image) -> Result<Vec<u8>, RecordError> {
    if image.width == 0 || image.height == 0 {
        return Err(RecordError::Invalid);
    }
    let stride = image
        .width
        .checked_mul(3)
        .and_then(|v| v.checked_add(3))
        .map(|v| v & !3)
        .ok_or(RecordError::TooLarge)?;
    let len = stride
        .checked_mul(image.height)
        .and_then(|v| v.checked_add(54))
        .ok_or(RecordError::TooLarge)?;
    let len32 = u32::try_from(len).map_err(|_| RecordError::TooLarge)?;
    let mut out = vec![0; len];
    out[..2].copy_from_slice(b"BM");
    for (offset, value) in [
        (2, len32),
        (10, 54),
        (14, 40),
        (18, image.width as u32),
        (22, image.height as u32),
        (34, len32 - 54),
    ] {
        out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    out[26..28].copy_from_slice(&1u16.to_le_bytes());
    out[28..30].copy_from_slice(&24u16.to_le_bytes());
    for y in 0..image.height {
        for x in 0..image.width {
            let [r, g, b] = image.pixels[y * image.width + x];
            let i = 54 + (image.height - y - 1) * stride + x * 3;
            out[i..i + 3].copy_from_slice(&[b, g, r]);
        }
    }
    Ok(out)
}
