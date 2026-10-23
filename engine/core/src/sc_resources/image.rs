// SPDX-License-Identifier: GPL-3.0-or-later
use super::Result;
use crate::persistence_runtime::{Buffer, BufferHandle, Format, RawStream, Resource, BUFFER_IID};
use std::sync::{Arc, Mutex};

pub const IMAGE_RESOURCE_IID: u32 = 0x06f1_568e;
pub trait Aggregator: Send {
    fn set_outer(&mut self, attached: bool);
    fn query_interface(&mut self, iid: u32) -> bool;
    fn release(&mut self);
}
pub type AggregatorHandle = Arc<Mutex<dyn Aggregator>>;
pub struct CreatedBuffer {
    pub buffer: BufferHandle,
    pub aggregator: Option<AggregatorHandle>,
}
pub trait BufferProvider {
    fn create(&mut self, class: u32, iid: u32, flags: u32) -> Option<CreatedBuffer>;
    /// Newly acquired aggregation interface, never a borrowed pointer. The
    /// default transfers a pre-acquired QI result supplied by the provider.
    fn query_aggregator(&mut self, created: &mut CreatedBuffer, iid: u32) -> Option<AggregatorHandle> {
        debug_assert_eq!(iid, 0x12f);
        created.aggregator.take()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordBmpOffset {
    Unsettled,
    Sequential,
    Absolute(u64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageInterface {
    Resource,
    ImageResource,
    Buffer,
    Aggregator,
}
pub struct PersistBufferResource {
    pub base: Resource,
    pub target: Format,
    buffer: Option<BufferHandle>,
    aggregator: Option<AggregatorHandle>,
    scratch: Vec<u8>,
    create_flags: u32,
}
impl PersistBufferResource {
    pub fn new(target: Format, flags: u32, provider: &mut impl BufferProvider) -> Self {
        let first = provider.create(0xc470_d326, BUFFER_IID, flags);
        let first_ok = first.is_some();
        let created = first.or_else(|| provider.create(0xc470_d325, BUFFER_IID, flags));
        let (buffer, aggregator) = if let Some(mut c) = created {
            let aggregator = if first_ok {
                provider.query_aggregator(&mut c, 0x12f)
            } else {
                None
            };
            (Some(c.buffer), aggregator)
        } else {
            (None, None)
        };
        if let Some(a) = &aggregator {
            a.lock().unwrap_or_else(|e| e.into_inner()).set_outer(true);
        }
        // No image resource type constant is assigned by §7; base key remains 0.
        Self {
            base: Resource::new(0),
            target,
            buffer,
            aggregator,
            scratch: Vec::new(),
            create_flags: flags,
        }
    }
    pub fn buffer(&self) -> Option<BufferHandle> {
        let b = self.buffer.as_ref()?;
        b.lock().unwrap_or_else(|e| e.into_inner()).add_ref();
        Some(b.clone())
    }
    pub fn resource_type(&self) -> u32 { self.base.key().ty }
    pub fn set_buffer(&mut self, new: Option<BufferHandle>) {
        if let Some(b) = &new {
            b.lock().unwrap_or_else(|e| e.into_inner()).add_ref();
        }
        if let Some(old) = self.buffer.take() {
            old.lock().unwrap_or_else(|e| e.into_inner()).release();
        }
        self.buffer = new;
    }
    pub fn buffer_interface(&self, iid: u32) -> bool {
        self.buffer
            .as_ref()
            .is_some_and(|b| b.lock().unwrap_or_else(|e| e.into_inner()).query_interface(iid))
    }
    pub fn query_interface(&self, iid: u32) -> Option<ImageInterface> {
        if iid == IMAGE_RESOURCE_IID {
            self.base.add_ref();
            return Some(ImageInterface::ImageResource);
        }
        if self.base.query_interface(iid) {
            return Some(ImageInterface::Resource);
        }
        if let Some(a) = &self.aggregator {
            if a.lock().unwrap_or_else(|e| e.into_inner()).query_interface(iid) {
                return Some(ImageInterface::Aggregator);
            }
        } else if self.buffer_interface(iid) {
            return Some(ImageInterface::Buffer);
        }
        None
    }
    pub fn release(&mut self) -> u32 {
        let count = self.base.release();
        if count == 0 {
            self.clear();
        }
        count
    }
    fn clear(&mut self) {
        if let Some(a) = self.aggregator.take() {
            let mut a = a.lock().unwrap_or_else(|e| e.into_inner());
            a.set_outer(false);
            a.release();
        }
        if let Some(b) = self.buffer.take() {
            b.lock().unwrap_or_else(|e| e.into_inner()).release();
        }
        self.scratch.clear();
    }
    pub fn load_record(&mut self, raw: &mut impl RawStream) -> Result<bool> {
        let result = self.bmp(raw, false, RecordBmpOffset::Sequential);
        if matches!(result, Ok(true)) {
            return result;
        }
        if !raw.seek(0) {
            return Ok(false);
        }
        self.tga(raw, 0x8080)
    }
    pub fn load_file_bmp(&mut self, raw: &mut impl RawStream) -> Result<bool> {
        self.bmp(raw, true, RecordBmpOffset::Sequential)
    }
    /// Explicit alternate cursor policy for callers; the owner uses Sequential.
    pub fn load_record_bmp_with_policy(&mut self, raw: &mut impl RawStream, policy: RecordBmpOffset) -> Result<bool> {
        self.bmp(raw, false, policy)
    }
    pub fn load_file_tga(&mut self, raw: &mut impl RawStream) -> Result<bool> {
        self.tga(raw, 0x8080)
    }
    fn bmp(&mut self, raw: &mut impl RawStream, file: bool, policy: RecordBmpOffset) -> Result<bool> {
        let mut header = [0; 14];
        let mut info = [0; 40];
        if !raw.read(&mut header) || &header[..2] != b"BM" || !raw.read(&mut info) {
            return Ok(false);
        }
        let width = le32(&info[4..]);
        let height = le32(&info[8..]);
        let depth = le16(&info[14..]);
        let compression = le32(&info[16..]);
        if le32(&info) != 40
            || width == 0
            || height == 0
            || width > i32::MAX as u32
            || height > i32::MAX as u32
            || !matches!(depth, 4 | 8 | 24)
            || compression > 1
        {
            return Ok(false);
        }
        if !bmp_translation(depth, self.target.bits) {
            return Ok(false);
        }
        if !file && le32(&header[10..]).wrapping_add(14) < 54 {
            return Ok(false);
        }
        let colours = if depth <= 8 {
            let n = le32(&info[32..]);
            if n == 0 {
                1usize << depth
            } else {
                n as usize
            }
        } else {
            0
        };
        if colours > 256 {
            return Ok(false);
        }
        let mut palette = vec![0; colours * 4];
        if !raw.read(&mut palette) {
            return Ok(false);
        }
        let seek = if file {
            Some(u64::from(le32(&header[10..])))
        } else {
            match policy {
                RecordBmpOffset::Absolute(at) => Some(at),
                _ => None,
            }
        };
        if seek.is_some_and(|at| !raw.seek(at)) {
            return Ok(false);
        }
        let Some(handle) = &self.buffer else { return Ok(false); };
        let mut b = handle.lock().unwrap_or_else(|e| e.into_inner());
        if !b.initialize_with_flags(width, height, self.target, self.create_flags) || !b.lock_with_flags(0x8040) {
            return Ok(false);
        }
        let result = (|| {
        let pixels = if compression == 1 {
            if depth != 8 {
                return Ok(false);
            }
            let size = if le32(&info[20..]) == 0 {
                let size = le32(&header[2..]).wrapping_sub(le32(&header[10..]));
                if !file && size >= le32(&header[2..]) { return Ok(false); }
                size
            } else { le32(&info[20..]) };
            let mut compressed = vec![0; size as usize];
            if !raw.read(&mut compressed) { return Ok(false); }
            let Some(p) = rle8(&mut std::io::Cursor::new(compressed), width, height) else {
                return Ok(false);
            };
            p
        } else {
            let Some(stride) = bmp_stride(width, depth, file) else {
                return Ok(false);
            };
            // Invalid bounds have no successful contract. The 4-bpp record stride
            // quirk yields zero; reject safely instead of indexing empty rows.
            if stride == 0 {
                return Ok(false);
            }
            let Some(size) = stride.checked_mul(height as usize) else {
                return Ok(false);
            };
            self.scratch.resize(size, 0);
            if !raw.read(&mut self.scratch) {
                return Ok(false);
            }
            let mut pixels = Vec::new();
            for y in 0..height as usize {
                for x in 0..width as usize {
                    if depth == 24 {
                        let at = y * stride + x * 3;
                        pixels.extend_from_slice(&self.scratch[at..at + 3]);
                    } else {
                        let b = self.scratch[y * stride + if depth == 4 { x / 2 } else { x }];
                        pixels.push(if depth == 4 {
                            if x % 2 == 0 {
                                b >> 4
                            } else {
                                b & 15
                            }
                        } else {
                            b
                        });
                    }
                }
            }
            pixels
        };
        if depth == 24 && self.target.colour_type == 1 && b.layout().format == Format::new(7, 16) {
            let mut rgba = Vec::with_capacity((width*height) as usize);
            for y in (0..height as usize).rev() {
                for x in 0..width as usize {
                    let at = (y*width as usize+x)*3;
                    rgba.push([pixels[at], pixels[at+1], pixels[at+2], 255]);
                }
            }
            return Ok(store_4444(&mut *b, width, height, &diffuse_4444(&rgba, width as usize, false)));
        }
        let mut success = true;
        'rows: for y in 0..height {
            for x in 0..width {
                let at = (y * width + x) as usize;
                let (rgb, index) = if depth == 24 {
                    let at = at * 3;
                    ([pixels[at + 2], pixels[at + 1], pixels[at]], None)
                } else {
                    let index = pixels[at];
                    let p = usize::from(index) * 4;
                    if p + 3 >= palette.len() {
                        success = false;
                        break 'rows;
                    }
                    ([palette[p + 2], palette[p + 1], palette[p]], Some(index))
                };
                if !b.rgb(x, height - 1 - y, rgb, index) {
                    success = false;
                    break 'rows;
                }
            }
        }
        Ok(success)
        })();
        b.unlock();
        result
    }
    fn tga(&mut self, raw: &mut impl RawStream, flags: u32) -> Result<bool> {
        if !raw.seek(0) {
            return Ok(false);
        }
        let mut h = [0u8; 18];
        if !raw.read(&mut h) || !matches!(h[2], 2 | 10) {
            return Ok(false);
        }
        let (width, height) = (u32::from(le16(&h[12..])), u32::from(le16(&h[14..])));
        let palette = u64::from(le16(&h[5..])) * u64::from(h[7]).div_ceil(8);
        if !raw.seek(18 + u64::from(h[0]) + palette) {
            return Ok(false);
        }
        let Some(handle) = &self.buffer else {
            return Ok(false);
        };
        let mut b = handle.lock().unwrap_or_else(|e| e.into_inner());
        if !b.initialize_with_flags(width, height, self.target, self.create_flags) || !b.lock_with_flags(flags) {
            return Ok(false);
        }
        // Unsupported source depth still succeeds once creation and locking did.
        let result = if matches!(h[16], 16 | 24 | 32) {
            decode_tga(raw, &mut *b, &h, width, height)
        } else {
            true
        };
        b.unlock();
        Ok(result)
    }
}
impl Drop for PersistBufferResource {
    fn drop(&mut self) {
        self.clear();
    }
}
pub fn bmp_translation(source: u16, target: u32) -> bool {
    matches!((source, target), (4, 16) | (8, 8 | 16 | 32) | (24, 16 | 24 | 32))
}
pub fn bmp_stride(width: u32, depth: u16, file: bool) -> Option<usize> {
    let row = if depth == 4 && file {
        (width as usize).div_ceil(2)
    } else {
        (width as usize).checked_mul(usize::from(depth >> 3))?
    };
    Some(row.checked_add(3)? & !3)
}
fn le16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}
fn le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}
fn rle8(raw: &mut impl RawStream, width: u32, height: u32) -> Option<Vec<u8>> {
    let mut pixels = vec![0; (width as usize).checked_mul(height as usize)?];
    let (mut x, mut y) = (0u32, 0u32);
    loop {
        let mut pair = [0; 2];
        if !raw.read(&mut pair) {
            return None;
        }
        let (n, v) = (pair[0], pair[1]);
        if n != 0 {
            for _ in 0..n {
                if x >= width || y >= height {
                    return None;
                }
                pixels[(y * width + x) as usize] = v;
                x += 1;
            }
        } else {
            match v {
                0 => {
                    x = 0;
                    y += 1;
                }
                1 => return Some(pixels),
                2 => {
                    let mut delta = [0; 2];
                    if !raw.read(&mut delta) {
                        return None;
                    }
                    x += u32::from(delta[0]);
                    y += u32::from(delta[1]);
                }
                n => {
                    let mut bytes = vec![0; usize::from(n) + usize::from(n & 1)];
                    if !raw.read(&mut bytes) {
                        return None;
                    }
                    for &value in &bytes[..usize::from(n)] {
                        if x >= width || y >= height {
                            return None;
                        }
                        pixels[(y * width + x) as usize] = value;
                        x += 1;
                    }
                }
            }
        }
    }
}
fn decode_tga(raw: &mut impl RawStream, buffer: &mut dyn Buffer, h: &[u8; 18], width: u32, height: u32) -> bool {
    let depth = usize::from(h[16] / 8);
    let total = u64::from(width) * u64::from(height);
    let mut at = 0;
    let dither = depth == 4 && buffer.layout().format.colour_type == 7;
    let mut rgba = if dither { vec![[0;4]; total as usize] } else { Vec::new() };
    while at < total {
        let mut packet = [0];
        if h[2] == 10 && !raw.read(&mut packet) {
            return false;
        }
        let count = if h[2] == 10 { u64::from(packet[0] & 127) + 1 } else { 1 };
        if at + count > total {
            return false;
        }
        let repeated = h[2] == 10 && packet[0] & 128 != 0;
        let mut p = [0; 4];
        for i in 0..count {
            if (!repeated || i == 0) && !raw.read(&mut p[..depth]) {
                return false;
            }
            let (r, g, b, a) = if depth == 2 {
                let v = le16(&p);
                let scale = |n: u16| u32::from(n) * 255 / 31;
                (
                    scale((v >> 10) & 31),
                    scale((v >> 5) & 31),
                    scale(v & 31),
                    if h[17] & 15 != 0 {
                        if v & 0x8000 != 0 {
                            255
                        } else {
                            0
                        }
                    } else {
                        255
                    },
                )
            } else {
                let bits = (h[17] & 15).min(8);
                let alpha = if depth == 4 && bits != 0 {
                    u32::from(p[3] >> (8 - bits)) * 255 / ((1 << bits) - 1)
                } else {
                    255
                };
                (u32::from(p[2]), u32::from(p[1]), u32::from(p[0]), alpha)
            };
            let y = (at / u64::from(width)) as u32;
            let y = if h[17] & 32 == 0 { height - 1 - y } else { y };
            if dither {
                rgba[(u64::from(y)*u64::from(width)+at%u64::from(width)) as usize] = [b as u8,g as u8,r as u8,a as u8];
            } else if !buffer.pixel32((at % u64::from(width)) as u32, y, (a << 24) | (r << 16) | (g << 8) | b) {
                return false;
            }
            at += 1;
        }
    }
    if dither { store_4444(buffer, width, height, &diffuse_4444(&rgba, width as usize, true)) } else { true }
}

/// Section 7.3: padded, serpentine low-nibble diffusion in B/G/R/A lanes.
pub fn diffuse_4444(pixels: &[[u8;4]], width: usize, alpha: bool) -> Vec<u16> {
    if width == 0 { return Vec::new(); }
    let height = pixels.len()/width;
    let mut current = vec![[0u32;4]; width+2];
    let mut next = current.clone();
    let mut output = vec![0; pixels.len()];
    for y in 0..height {
        let direction = if y%2 == 0 {1isize} else {-1};
        for step in 0..width {
            let x = if direction == 1 {step} else {width-1-step};
            let cell = x+1;
            let forward = (cell as isize+direction) as usize;
            let backward = (cell as isize-direction) as usize;
            let mut word = if alpha {0} else {0xf000};
            for channel in 0..if alpha {4} else {3} {
                let value = (u32::from(pixels[y*width+x][channel]) + current[cell][channel]/16).min(255);
                word |= ((value >> 4) as u16) << (channel*4);
                let error = value & 15;
                current[forward][channel] += error*7;
                next[backward][channel] += error*3;
                next[cell][channel] += error*5;
                next[forward][channel] += error;
            }
            output[y*width+x] = word;
        }
        std::mem::swap(&mut current, &mut next);
        next.fill([0;4]);
    }
    output
}
fn store_4444(buffer: &mut dyn Buffer, width: u32, height: u32, words: &[u16]) -> bool {
    let stride = buffer.layout().stride;
    let bytes = buffer.bytes_mut();
    for y in 0..height as usize {
        for x in 0..width as usize {
            let Some(target) = bytes.get_mut(y*stride+x*2..y*stride+x*2+2) else { return false; };
            target.copy_from_slice(&words[y*width as usize+x].to_le_bytes());
        }
    }
    true
}
