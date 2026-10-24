// SPDX-License-Identifier: GPL-3.0-or-later
//! Named texture metadata, surface lifetime, upload addressing (§9).
use super::backend::*;

#[derive(Clone, Debug)]
pub struct Texture {
    pub used: bool,
    pub width: u32,
    pub height: u32,
    pub levels: u32,
    pub priority: f32,
    pub format: u32,
    pub flags: u32,
    pub parameters: [u32; 4],
    pub surface: u32,
}
impl Default for Texture {
    fn default() -> Self {
        Self {
            used: true,
            width: 0,
            height: 0,
            levels: 0,
            priority: 0.5,
            format: 4,
            flags: 0,
            parameters: [0, 0, 3, 3],
            surface: 0,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Format {
    /// alpha, red, green, blue masks, flags, FourCC, bits, converter key.
    pub words: [u32; 8],
    pub alpha_bits: u32,
    pub size: u32,
}
/// §9.6: compressed preference, then stable rank/metric selection.
pub fn select_format(request: u32, candidates: &[Format], _host: &mut impl Device) -> Result<Format> {
    let request = match request {
        5..=7 => {
            let key = [9, 11, 13][(request - 5) as usize];
            if let Some(candidate) = candidates.iter().find(|f| f.words[7] == key) {
                return Ok(*candidate);
            }
            [3, 2, 2][(request - 5) as usize]
        }
        0..=4 => request,
        _ => return Err(Error::InvalidInput),
    };
    let (bits, alpha) = [(16, 0), (24, 0), (16, 4), (16, 1), (32, 8)][request as usize];
    let mut best = Format::default();
    let mut score = (0, 0);
    for f in candidates {
        let (b, a) = (f.words[6], f.alpha_bits);
        let next = if b == bits && a == alpha {
            (7, 0)
        } else if alpha > 0 && a == alpha {
            (6, 0)
        } else if alpha > 0 && a > alpha {
            (5, u32::MAX - a)
        } else if alpha > 0 && a > 0 {
            (4, a)
        } else if b == bits {
            (3, 0)
        } else if b > bits {
            (2, u32::MAX - b)
        } else {
            (1, b)
        };
        if next > score {
            best = *f;
            score = next;
        }
    }
    Ok(best)
}
pub fn converter_available(source: usize, key: u32) -> bool {
    match source {
        0..=9 => matches!(key, 0..=5 | 7),
        10 => matches!(key, 1 | 2 | 4 | 5 | 7 | 9),
        12 => matches!(key, 1 | 2 | 4 | 5 | 7 | 11),
        11 | 13 | 14 => matches!(key, 10..=13),
        _ => false,
    }
}
#[derive(Default, Debug)]
pub struct TextureManager {
    pub refs: u32,
    pub records: Vec<Texture>,
    pub freed: Vec<u32>,
    pub dd: u32,
    pub device: u32,
    pub formats: Option<[Format; 8]>,
    pub converters: Option<[[bool; 8]; 15]>,
}
#[derive(Clone, Copy, Debug)]
pub struct Upload {
    pub name: u32,
    pub level: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub kind: u32,
    pub row_length: u32,
    pub pixels: u32,
}
pub fn source_format(format: u32, kind: u32) -> Option<(usize, u32)> {
    Some(match (kind, format) {
        (0 | 1, 3) => (0, 32),
        (0 | 1, 1) => (1, 32),
        (0 | 1, 2) => (2, 24),
        (0 | 1, 0) => (3, 24),
        (13, 3) => (4, 16),
        (13, 1) => (5, 16),
        (10, 3) => (6, 16),
        (10, 1) => (7, 16),
        (14, 2) => (8, 16),
        (14, 0) => (9, 16),
        (0 | 1, 7 | 8) => (10, 4),
        (0 | 1, 9) => (12, 8),
        (0 | 1, 10) => (14, 8),
        _ => return None,
    })
}
impl TextureManager {
    pub fn destroy(&mut self, host: &mut impl Device) {
        self.shutdown(host);
        self.records = Vec::new();
        self.freed = Vec::new();
        self.formats = None;
        self.converters = None;
    }
    pub fn get(&self, name: u32) -> Option<&Texture> {
        self.records.get(name as usize).filter(|r| r.used)
    }
    pub fn get_mut(&mut self, name: u32) -> Option<&mut Texture> {
        self.records.get_mut(name as usize).filter(|r| r.used)
    }
    pub fn valid(&self, name: u32) -> bool {
        self.get(name).is_some()
    }
    pub fn alpha(&self, name: u32) -> bool {
        self.get(name).is_some_and(|r| r.format != 0 && r.format != 1)
    }
    pub fn surface(&self, name: u32) -> u32 {
        self.get(name).map_or(0, |r| r.surface)
    }
    pub fn new_name(&mut self) -> u32 {
        if let Some(name) = self.freed.pop() {
            let r = &mut self.records[name as usize];
            r.used = true;
            r.width = 0;
            r.height = 0;
            name
        } else {
            let name = self.records.len() as u32;
            self.records.push(Texture::default());
            name
        }
    }
    pub fn free(&mut self, name: u32, host: &mut impl Device) -> bool {
        if let Some(r) = self.get_mut(name) {
            release(host, &mut r.surface);
            r.used = false;
            self.freed.push(name);
            true
        } else {
            false
        }
    }
    pub fn priority(&mut self, name: u32, value: f32) -> bool {
        if let Some(r) = self.get_mut(name) {
            r.priority = value;
            if value == 0.95f32 {
                r.flags |= 1;
            }
            true
        } else {
            false
        }
    }
    pub fn init(&mut self, dd: u32, device: u32, host: &mut impl Device) -> Result<()> {
        if dd == 0 || device == 0 || self.dd != 0 {
            return Ok(());
        }
        acquire(host, dd);
        acquire(host, device);
        self.dd = dd;
        self.device = device;
        if self.formats.is_none() {
            let mut count = [0];
            host.dx(device, Call::TextureFormats, &[], &[], &mut count);
            let mut candidates = Vec::new();
            for i in 0..count[0] {
                let mut values = [0; 10];
                host.dx(device, Call::TextureFormats, &[i], &[], &mut values);
                candidates.push(Format {
                    words: values[..8].try_into().unwrap(),
                    alpha_bits: values[8],
                    size: values[9],
                });
            }
            let mut formats = [Format::default(); 8];
            for (i, format) in formats.iter_mut().enumerate() {
                *format = select_format(i as u32, &candidates, host)?;
            }
            self.formats = Some(formats);
        }
        if self.converters.is_none() {
            let formats = self.formats.unwrap();
            self.converters = Some(std::array::from_fn(|s| {
                std::array::from_fn(|i| converter_available(s, formats[i].words[7]))
            }));
        }
        let name = self.new_name();
        self.create(name, 0, 4, 1, 1, 1, host)?;
        // The known white pixel is an immediate value, not a foreign address.
        // The foreign allocator supplies temporary readable storage; it is never retained.
        let pixel = host.alloc(4, crate::gdrivers_director::Allocation::Heap);
        if pixel == 0 {
            return Err(Error::Foreign(crate::gdrivers_director::Fault::NullWrite));
        }
        let result = host
            .write(pixel, &u32::MAX.to_le_bytes())
            .map_err(Error::from)
            .and_then(|()| {
                self.load(
                    Upload {
                        name,
                        level: 0,
                        x: 0,
                        y: 0,
                        width: 1,
                        height: 1,
                        format: 3,
                        kind: 0,
                        row_length: 0,
                        pixels: pixel,
                    },
                    host,
                )
                .map(|_| ())
            });
        host.free(pixel, crate::gdrivers_director::Allocation::Heap);
        result?;
        Ok(())
    }
    pub fn lose(&mut self, host: &mut impl Device) {
        for r in &mut self.records {
            release(host, &mut r.surface);
        }
        release(host, &mut self.device);
    }
    pub fn init_device(&mut self, device: u32, host: &mut impl Device) -> Result<()> {
        acquire(host, device);
        release(host, &mut self.device);
        self.device = device;
        for i in 0..self.records.len() {
            let r = self.records[i].clone();
            if r.used && r.width != 0 {
                self.create(i as u32, r.flags, r.format, r.width, r.height, r.levels, host)?;
            }
        }
        Ok(())
    }
    pub fn shutdown(&mut self, host: &mut impl Device) {
        self.free(0, host);
        for r in &mut self.records {
            release(host, &mut r.surface);
        }
        self.records.clear();
        self.freed.clear();
        release(host, &mut self.device);
        release(host, &mut self.dd);
    }
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        &mut self,
        name: u32,
        flags: u32,
        format: u32,
        width: u32,
        height: u32,
        levels: u32,
        host: &mut impl Device,
    ) -> Result<bool> {
        if !(1..=4096).contains(&width) || !(1..=4096).contains(&height) {
            return Ok(false);
        }
        let device = self.device;
        let dd = self.dd;
        let chosen = self.formats.and_then(|f| f.get(format as usize).copied());
        let r = match self.get_mut(name) {
            Some(r) => r,
            None => return Ok(false),
        };
        r.width = width;
        r.height = height;
        r.levels = levels;
        r.flags = flags;
        r.format = format;
        release(host, &mut r.surface);
        if device == 0 {
            return Ok(true);
        }
        let chosen = chosen.ok_or(Error::InvalidInput)?;
        let caps2 = if flags & 1 != 0 {
            0x20004
        } else if flags & 2 != 0 {
            0x20080
        } else {
            0x20000
        };
        let mut args = vec![0x7c, 0x21007, width, height, levels.max(1), 0x401008, caps2];
        args.push(chosen.size);
        args.extend(&chosen.words[..7]);
        let mut out = [0];
        let hr = host.dx(dd, Call::CreateSurface, &args, &[], &mut out);
        if failed(hr) {
            return Ok(false);
        }
        r.surface = out[0];
        Ok(true)
    }
    pub fn load(&self, u: Upload, host: &mut impl Device) -> Result<bool> {
        let r = match self.get(u.name) {
            Some(r) if r.surface != 0 && (u.level == 0 || u.level < r.levels) => r,
            _ => return Ok(false),
        };
        let (source, bits) = match source_format(u.format, u.kind) {
            Some(v) => v,
            None => return Ok(false),
        };
        let available = self.converters.ok_or(Error::InvalidInput)?;
        if r.format >= 8 || !available[source][r.format as usize] {
            return Ok(false);
        }
        let mut surface = r.surface;
        let mut owned = false;
        for _ in 0..u.level {
            let mut out = [0];
            let hr = host.dx(surface, Call::AttachedSurface, &[0x400000], &[], &mut out);
            if owned {
                release(host, &mut surface);
            }
            if failed(hr) {
                return Ok(false);
            }
            surface = out[0];
            owned = true;
        }
        let result = self.load_surface(surface, u, source as u32, bits, host);
        if owned {
            release(host, &mut surface);
        }
        result
    }
    fn load_surface(&self, surface: u32, u: Upload, source: u32, bits: u32, host: &mut impl Device) -> Result<bool> {
        let format = self.get(u.name).ok_or(Error::InvalidInput)?.format as usize;
        let key = self
            .formats
            .and_then(|f| f.get(format).copied())
            .ok_or(Error::InvalidInput)?
            .words[7];
        let mut out = [0; 5]; // mapped address, pitch, bits, linear-size flag, linear byte size
        loop {
            let hr = host.dx(surface, Call::LockSurface, &[0x2821], &[], &mut out);
            if hr == LOST && !failed(dx(host, surface, Call::RestoreSurface, &[])) {
                continue;
            }
            if failed(hr) {
                return Ok(false);
            }
            break;
        }
        let row = if u.row_length == 0 { u.width } else { u.row_length };
        let pitch = bits.wrapping_mul(row) / 8;
        let result = if out[3] != 0 && (u.x != 0 || u.y != 0) {
            Ok(())
        } else {
            let destination = if out[3] != 0 {
                out[0]
            } else {
                out[0]
                    .wrapping_add(u.y.wrapping_mul(out[1]))
                    .wrapping_add(u.x.wrapping_mul(out[2] / 8))
            };
            let (width, height, src_pitch, dst_pitch) = if out[3] != 0 {
                (out[4], 1, out[4], out[4])
            } else {
                (u.width, u.height, pitch, out[1])
            };
            convert(
                source,
                key,
                u.pixels,
                destination,
                src_pitch,
                dst_pitch,
                width,
                height,
                host,
            )
        };
        dx(host, surface, Call::UnlockSurface, &[]);
        result.map(|()| true)
    }
}

/// All available §9.6 cells, including raw compressed copies and software decoders.
#[allow(clippy::too_many_arguments)]
pub fn convert(
    source: u32,
    key: u32,
    src: u32,
    dst: u32,
    src_pitch: u32,
    dst_pitch: u32,
    width: u32,
    height: u32,
    host: &mut impl Device,
) -> Result<()> {
    let raw = matches!((source, key), (10, 9) | (12, 11) | (11 | 13 | 14, 10..=13));
    if !converter_available(source as usize, key) {
        return Err(Error::InvalidInput);
    }
    if !raw && matches!(source, 10 | 12) {
        return decode_blocks(source, key, src, dst, dst_pitch, width, height, host);
    }
    for row in 0..height {
        // Section 9.6's compressed byte copier advances rows; expansions walk backward.
        let y = if raw { row } else { height - 1 - row };
        let s = src.wrapping_add(y.wrapping_mul(src_pitch));
        let d = dst.wrapping_add(y.wrapping_mul(dst_pitch));
        if raw {
            let mut bytes = vec![0; width as usize];
            host.read(s, &mut bytes)?;
            host.write(d, &bytes)?;
        } else {
            for x in 0..width {
                let size = match source {
                    0 | 1 => 4,
                    2 | 3 => 3,
                    _ => 2,
                };
                let mut bytes = [0; 4];
                host.read(s.wrapping_add(x.wrapping_mul(size)), &mut bytes[..size as usize])?;
                let v = u32::from_le_bytes(bytes);
                let (mut rgb, bits, alpha, abits) = match source {
                    0 | 1 => ([(v >> 16) & 255, (v >> 8) & 255, v & 255], [8; 3], v >> 24, 8),
                    2 | 3 => ([(v >> 16) & 255, (v >> 8) & 255, v & 255], [8; 3], 1, 1),
                    4 | 5 => ([(v >> 8) & 15, (v >> 4) & 15, v & 15], [4; 3], v >> 12, 4),
                    6 | 7 => ([(v >> 10) & 31, (v >> 5) & 31, v & 31], [5; 3], v >> 15, 1),
                    _ => ([(v >> 11) & 31, (v >> 5) & 63, v & 31], [5, 6, 5], 1, 1),
                };
                if source & 1 != 0 {
                    rgb.swap(0, 2);
                }
                let (outbits, outalpha, size) = destination_layout(key);
                for i in 0..3 {
                    rgb[i] = resize_channel(rgb[i], bits[i], outbits[i]);
                }
                let alpha = resize_channel(alpha, abits, outalpha);
                let pixel = pack_pixel(key, rgb, alpha);
                host.write(
                    d.wrapping_add(x.wrapping_mul(size)),
                    &pixel.to_le_bytes()[..size as usize],
                )?;
            }
        }
    }
    Ok(())
}

fn resize_channel(value: u32, from: u32, to: u32) -> u32 {
    if to <= from {
        return value >> (from - to);
    }
    // §9.6 repeats high source bits, including 1-bit alpha replication.
    let mut result = value;
    let mut bits = from;
    while bits < to {
        let take = (to - bits).min(from);
        result = (result << take) | (value >> (from - take));
        bits += take;
    }
    result
}
fn destination_layout(key: u32) -> ([u32; 3], u32, u32) {
    match key {
        0 => ([8; 3], 0, 3),
        1 | 2 => ([8; 3], 8, 4),
        3 => ([5, 6, 5], 0, 2),
        4 | 5 => ([5; 3], 1, 2),
        7 => ([4; 3], 4, 2),
        _ => unreachable!(),
    }
}
fn pack_pixel(key: u32, rgb: [u32; 3], alpha: u32) -> u32 {
    let [r, g, b] = rgb;
    match key {
        0..=2 => alpha << 24 | r << 16 | g << 8 | b,
        3 => r << 11 | g << 5 | b,
        4 | 5 => alpha << 15 | r << 10 | g << 5 | b,
        7 => alpha << 12 | r << 8 | g << 4 | b,
        _ => unreachable!(),
    }
}
#[allow(clippy::too_many_arguments)]
fn decode_blocks(
    source: u32,
    key: u32,
    src: u32,
    dst: u32,
    pitch: u32,
    width: u32,
    height: u32,
    host: &mut impl Device,
) -> Result<()> {
    let (bits, abits, size) = destination_layout(key);
    let block_size = if source == 10 { 8 } else { 16 };
    for by in 0..height / 4 {
        for bx in 0..width / 4 {
            let mut block = [0; 16];
            host.read(
                src.wrapping_add((by * (width / 4) + bx) * block_size),
                &mut block[..block_size as usize],
            )?;
            let color_offset = if source == 10 { 0 } else { 8 };
            let c0 = u16::from_le_bytes(block[color_offset..color_offset + 2].try_into().unwrap()) as u32;
            let c1 = u16::from_le_bytes(block[color_offset + 2..color_offset + 4].try_into().unwrap()) as u32;
            let mut palette = [[0; 3]; 4];
            for (i, c) in [c0, c1].into_iter().enumerate() {
                palette[i] = [
                    resize_channel(c >> 11, 5, bits[0]),
                    resize_channel((c >> 5) & 63, 6, bits[1]),
                    resize_channel(c & 31, 5, bits[2]),
                ];
            }
            for channel in 0..3 {
                let a = palette[0][channel];
                let b = palette[1][channel];
                if c0 > c1 {
                    let (wa, wb, rounding, shift) = match key {
                        1 | 2 => (171, 85, 128, 8),
                        4 | 5 => (21, 11, 16, 5),
                        _ => (11, 5, 8, 4),
                    };
                    palette[2][channel] = (wa * a + wb * b + rounding) >> shift;
                    palette[3][channel] = (wb * a + wa * b + rounding) >> shift;
                } else {
                    palette[2][channel] = (a + b).div_ceil(2);
                }
            }
            let selectors = u32::from_le_bytes(block[color_offset + 4..color_offset + 8].try_into().unwrap());
            let alpha_bits = u64::from_le_bytes(block[..8].try_into().unwrap());
            for p in 0..16 {
                let selector = (selectors >> (p * 2)) & 3;
                let alpha = if source == 12 {
                    resize_channel(((alpha_bits >> (p * 4)) & 15) as u32, 4, abits)
                } else if c0 <= c1 && selector == 3 {
                    0
                } else {
                    (1 << abits) - 1
                };
                let pixel = pack_pixel(key, palette[selector as usize], alpha);
                host.write(
                    dst.wrapping_add((by * 4 + p / 4) * pitch + (bx * 4 + p % 4) * size),
                    &pixel.to_le_bytes()[..size as usize],
                )?;
            }
        }
    }
    Ok(())
}
