// SPDX-License-Identifier: GPL-3.0-or-later
use super::{RawStream, Resource, ResourceInterface, SerialRecord, BUFFER_IID, BUFFER_RESOURCE_IID, BUFFER_TYPE};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Format {
    pub colour_type: u32,
    pub bits: u32,
}
impl Format {
    pub const fn new(colour_type: u32, bits: u32) -> Self {
        Self { colour_type, bits }
    }
    pub fn stored(colour_type: u32) -> Self {
        Self::new(
            colour_type,
            match colour_type {
                2 => 8,
                3..=7 => 16,
                8 | 9 => 32,
                _ => 0,
            },
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferLayout {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub format: Format,
}
impl BufferLayout {
    fn row_bytes(self) -> Option<usize> {
        (self.width as usize).checked_mul((self.format.bits / 8) as usize)
    }
    fn packed_len(self) -> Option<usize> {
        self.row_bytes()?.checked_mul(self.height as usize)
    }
}

/// The existing formats QFS implementation supplies this byte codec. It
/// cannot parse resource headers, validate pixel dimensions or copy rows.
pub trait ImageCodec {
    fn decompress(&self, bytes: &[u8]) -> Option<Vec<u8>>;
    fn compress(&self, bytes: &[u8]) -> Option<Vec<u8>>;
}

/// Only foreign colour conversion is abstract. No source file/record/header,
/// rows, locks, reference lifetime or factory outcomes cross this boundary.
pub trait PixelConversion: Send {
    fn pixel32(&self, value: u32, target: Format, out: &mut [u8]) -> bool;
    fn rgb(&self, rgb: [u8; 3], target: Format, out: &mut [u8]) -> bool;
    fn rgb16(&self, value: u16, source: Format) -> Option<[u8; 3]>;
    fn convert(&self, bytes: &[u8], source: Format, target: Format) -> Option<Vec<u8>>;
    fn index8(&self, _index: u8, _rgb: [u8; 3], _out: &mut [u8]) -> bool {
        false
    }
}

/// Buffer primitives, suitable for an owned portable allocation or foreign
/// buffer. QI must acquire its returned reference. `release` need not free
/// the Rust handle; the logical reference is independent of Arc ownership.
pub trait Buffer: Send {
    fn add_ref(&mut self);
    fn release(&mut self);
    fn query_interface(&mut self, iid: u32) -> bool;
    fn initialize(&mut self, width: u32, height: u32, format: Format) -> bool;
    /// File/record adapter creation flags. Existing portable buffers have no
    /// external allocation policy; foreign implementations may override it.
    fn initialize_with_flags(&mut self, width: u32, height: u32, format: Format, _flags: u32) -> bool {
        self.initialize(width, height, format)
    }
    fn layout(&self) -> BufferLayout;
    fn lock(&mut self, write: bool) -> bool;
    fn unlock(&mut self);
    fn bytes(&self) -> &[u8];
    fn bytes_mut(&mut self) -> &mut [u8];
    fn pixel32(&mut self, x: u32, y: u32, value: u32) -> bool;
    fn rgb(&mut self, x: u32, y: u32, rgb: [u8; 3], index: Option<u8>) -> bool;
    fn rgb16(&self, value: u16) -> Option<[u8; 3]>;
    fn convert(&mut self, target: Format) -> bool;
}
pub type BufferHandle = Arc<Mutex<dyn Buffer>>;

/// Owned packed pixels. Memory bound is an explicit portable input, not an
/// invented native dimension threshold (PR11). Colour semantics are supplied.
pub struct OwnedBuffer<C> {
    pub references: u32,
    pub locked: bool,
    layout: BufferLayout,
    bytes: Vec<u8>,
    conversion: C,
    max_bytes: usize,
}
impl<C> OwnedBuffer<C> {
    pub fn new(conversion: C, max_bytes: usize) -> Self {
        Self {
            references: 1,
            locked: false,
            layout: BufferLayout {
                width: 0,
                height: 0,
                stride: 0,
                format: Format::new(0, 0),
            },
            bytes: Vec::new(),
            conversion,
            max_bytes,
        }
    }
    fn pixel_range(&self, x: u32, y: u32) -> std::ops::Range<usize> {
        let n = (self.layout.format.bits / 8) as usize;
        let at = y as usize * self.layout.stride + x as usize * n;
        at..at + n
    }
}
impl<C: PixelConversion> Buffer for OwnedBuffer<C> {
    fn add_ref(&mut self) {
        self.references = self.references.wrapping_add(1);
    }
    fn release(&mut self) {
        self.references = self.references.wrapping_sub(1);
    }
    fn query_interface(&mut self, iid: u32) -> bool {
        if iid == BUFFER_IID || iid == 1 {
            self.add_ref();
            true
        } else {
            false
        }
    }
    fn initialize(&mut self, width: u32, height: u32, format: Format) -> bool {
        // Unsupported invalid/native-allocation domains: PR11. Owned storage
        // follows the published colour table and always uses packed stride.
        if width == 0 || height == 0 || format.bits == 0 || Format::stored(format.colour_type) != format {
            return false;
        }
        let mut layout = BufferLayout {
            width,
            height,
            stride: 0,
            format,
        };
        let Some(len) = layout.packed_len() else {
            return false;
        };
        if len > self.max_bytes {
            return false;
        }
        let mut bytes = Vec::new();
        if bytes.try_reserve_exact(len).is_err() {
            return false;
        }
        bytes.resize(len, 0);
        layout.stride = layout.row_bytes().unwrap_or(0);
        self.layout = layout;
        self.bytes = bytes;
        true
    }
    fn layout(&self) -> BufferLayout {
        self.layout
    }
    fn lock(&mut self, _write: bool) -> bool {
        if self.locked {
            false
        } else {
            self.locked = true;
            true
        }
    }
    fn unlock(&mut self) {
        self.locked = false;
    }
    fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
    fn pixel32(&mut self, x: u32, y: u32, value: u32) -> bool {
        let range = self.pixel_range(x, y);
        self.conversion
            .pixel32(value, self.layout.format, &mut self.bytes[range])
    }
    fn rgb(&mut self, x: u32, y: u32, rgb: [u8; 3], index: Option<u8>) -> bool {
        let range = self.pixel_range(x, y);
        if self.layout.format.bits == 8 {
            if let Some(index) = index {
                return self.conversion.index8(index, rgb, &mut self.bytes[range]);
            }
        }
        self.conversion.rgb(rgb, self.layout.format, &mut self.bytes[range])
    }
    fn rgb16(&self, value: u16) -> Option<[u8; 3]> {
        self.conversion.rgb16(value, self.layout.format)
    }
    fn convert(&mut self, target: Format) -> bool {
        if self.layout.format == target {
            return true;
        }
        let Some(bytes) = self.conversion.convert(&self.bytes, self.layout.format, target) else {
            return false;
        };
        let layout = BufferLayout {
            format: target,
            ..self.layout
        };
        if layout.packed_len() != Some(bytes.len()) || bytes.len() > self.max_bytes {
            return false;
        }
        self.bytes = bytes;
        self.layout = BufferLayout {
            stride: layout.row_bytes().unwrap_or(0),
            ..layout
        };
        true
    }
}

pub struct BufferResource {
    pub base: Resource,
    buffer: Option<BufferHandle>,
}
impl BufferResource {
    /// Takes the already-owned creation reference, including null allocation.
    pub fn new(created: Option<BufferHandle>) -> Self {
        Self {
            base: Resource::new(BUFFER_TYPE),
            buffer: created,
        }
    }
    pub fn release(&mut self) -> u32 {
        let count = self.base.release();
        if count == 0 {
            self.clear_buffer();
        }
        count
    }
    fn clear_buffer(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            buffer.lock().unwrap_or_else(|e| e.into_inner()).release();
        }
    }
    pub fn set_buffer(&mut self, buffer: Option<BufferHandle>) {
        if let Some(buffer) = &buffer {
            buffer.lock().unwrap_or_else(|e| e.into_inner()).add_ref();
        }
        self.clear_buffer();
        self.buffer = buffer;
    }
    pub fn get_buffer_as(&self, iid: u32) -> Option<BufferHandle> {
        let buffer = self.buffer.as_ref()?;
        if buffer.lock().unwrap_or_else(|e| e.into_inner()).query_interface(iid) {
            Some(buffer.clone())
        } else {
            None
        }
    }
    pub fn get_buffer(&self) -> Option<BufferHandle> {
        self.get_buffer_as(BUFFER_IID)
    }
    pub fn query_interface(&self, iid: u32) -> Option<(ResourceInterface, Option<BufferHandle>)> {
        if let Some(buffer) = self.get_buffer_as(iid) {
            return Some((ResourceInterface::Buffer, Some(buffer)));
        }
        if iid == BUFFER_RESOURCE_IID {
            self.base.add_ref();
            Some((ResourceInterface::BufferResource, None))
        } else if self.base.query_interface(iid) {
            Some((ResourceInterface::Resource, None))
        } else {
            None
        }
    }
    pub fn load<S: RawStream>(
        &mut self,
        record: &mut SerialRecord<S>,
        target: Format,
        codec: &impl ImageCodec,
    ) -> bool {
        let Some(buffer) = &self.buffer else {
            return false;
        };
        let mut head = [0; 8];
        if !record.raw_mut().read(&mut head) {
            return false;
        }
        let mut buffer = buffer.lock().unwrap_or_else(|e| e.into_inner());
        if head[..2] == *b"BM" {
            let mut rest = [0; 6];
            if !record.raw_mut().read(&mut rest) {
                return false;
            }
            return load_bmp(&mut *buffer, record, target);
        }
        let form = u32::from_le_bytes(head[..4].try_into().unwrap());
        let size_or_width = u32::from_le_bytes(head[4..].try_into().unwrap());
        if form == BUFFER_TYPE {
            let mut height = 0u32;
            if !record.get(&mut height) || !buffer.initialize(size_or_width, height, target) || !buffer.lock(true) {
                return false;
            }
            let len = buffer.layout().width as usize * buffer.layout().height as usize;
            let mut pixels = vec![0u32; len];
            let result = record.get_array(&mut pixels);
            let mut conversion = true;
            for (i, pixel) in pixels.into_iter().enumerate() {
                conversion &= buffer.pixel32(
                    (i % size_or_width as usize) as u32,
                    (i / size_or_width as usize) as u32,
                    pixel,
                );
            }
            buffer.unlock();
            // PR12: unsupported conversion cannot claim native pixel effects.
            return result && conversion;
        }
        if form != 1 && form != 2 {
            return false;
        }
        let (mut width, mut height, mut colour_type) = (0u32, 0u32, 0u32);
        if !record.get(&mut width)
            || !record.get(&mut height)
            || !record.get(&mut colour_type)
            || !buffer.initialize(width, height, Format::stored(colour_type))
            || !buffer.lock(true)
        {
            return false;
        }
        let result = load_pixels(&mut *buffer, record, form, size_or_width as usize, codec);
        buffer.unlock();
        result && buffer.convert(target)
    }
    pub fn save<S: RawStream>(&self, record: &mut SerialRecord<S>, version: u32, codec: &impl ImageCodec) -> bool {
        let Some(buffer) = &self.buffer else {
            return false;
        };
        let mut buffer = buffer.lock().unwrap_or_else(|e| e.into_inner());
        if !buffer.lock(false) {
            return false;
        }
        let result = save_locked(&*buffer, record, version, codec);
        buffer.unlock();
        result
    }
}
impl Drop for BufferResource {
    fn drop(&mut self) {
        self.clear_buffer();
    }
}

fn load_pixels<S: RawStream>(
    buffer: &mut dyn Buffer,
    record: &mut SerialRecord<S>,
    form: u32,
    size: usize,
    codec: &impl ImageCodec,
) -> bool {
    let layout = buffer.layout();
    let Some(packed) = layout.packed_len() else {
        return false;
    };
    let row = layout.row_bytes().unwrap_or(0);
    // v1 mismatched dataSize is an unsupported native-memory domain (PR12).
    if form == 1 && size != packed {
        return false;
    }
    if form == 1 && layout.stride == row {
        let Some(out) = buffer.bytes_mut().get_mut(..size) else {
            return false;
        };
        return record.raw_mut().read(out);
    }
    if size as u64 > record.raw().size().saturating_sub(record.raw().position()) {
        return false;
    }
    let mut bytes = vec![0; size];
    if !record.raw_mut().read(&mut bytes) {
        return false;
    }
    let decoded;
    let pixels = if form == 2 {
        let Some(output) = codec.decompress(&bytes) else {
            return false;
        };
        decoded = output;
        &decoded
    } else {
        &bytes
    };
    if pixels.len() != packed {
        return false;
    }
    for y in 0..layout.height as usize {
        let Some(destination) = buffer.bytes_mut().get_mut(y * layout.stride..y * layout.stride + row) else {
            return false;
        };
        // PR3 explicit choice: correct packed-source, padded-destination copy.
        destination.copy_from_slice(&pixels[y * row..(y + 1) * row]);
    }
    true
}

fn save_locked<S: RawStream>(
    buffer: &dyn Buffer,
    record: &mut SerialRecord<S>,
    version: u32,
    codec: &impl ImageCodec,
) -> bool {
    let layout = buffer.layout();
    if version == 0x55d4fb53 {
        return save_bmp(buffer, record);
    }
    if version != 2 {
        return false;
    }
    let Some(len) = layout.packed_len() else {
        return false;
    };
    let row = layout.row_bytes().unwrap_or(0);
    let mut packed = Vec::with_capacity(len);
    for y in 0..layout.height as usize {
        let Some(bytes) = buffer.bytes().get(y * layout.stride..y * layout.stride + row) else {
            return false;
        };
        packed.extend_from_slice(bytes);
    }
    let Some(compressed) = codec.compress(&packed) else {
        return false;
    };
    let Ok(size) = u32::try_from(compressed.len()) else {
        return false;
    };
    record.set(&2u32)
        && record.set(&size)
        && record.set(&layout.width)
        && record.set(&layout.height)
        && record.set(&layout.format.colour_type)
        && record.raw_mut().write(&compressed)
}

fn load_bmp<S: RawStream>(buffer: &mut dyn Buffer, record: &mut SerialRecord<S>, target: Format) -> bool {
    let mut info = [0; 40];
    if !record.raw_mut().read(&mut info) {
        return false;
    }
    let word = |at| u32::from_le_bytes(info[at..at + 4].try_into().unwrap());
    let (width, height, bits, compression, colours) = (
        word(4),
        word(8),
        u16::from_le_bytes([info[14], info[15]]),
        word(16),
        word(32),
    );
    if compression != 0
        || !matches!(
            (bits, target.bits),
            (4, 16) | (8, 8) | (8, 16) | (24, 16) | (24, 24) | (24, 32)
        )
    {
        return false;
    }
    // PR11: negative dimensions are outside the safely allocated domain. No
    // height-sign-dependent alternate row direction or file-offset seek.
    if width > i32::MAX as u32
        || height > i32::MAX as u32
        || !buffer.initialize(width, height, target)
        || !buffer.lock(true)
    {
        return false;
    }
    let result = load_bmp_locked(buffer, record, width, height, bits, colours);
    buffer.unlock();
    result
}
fn load_bmp_locked<S: RawStream>(
    buffer: &mut dyn Buffer,
    record: &mut SerialRecord<S>,
    width: u32,
    height: u32,
    bits: u16,
    colours: u32,
) -> bool {
    let palette_len = if bits == 24 {
        0
    } else if colours == 0 {
        1usize << bits
    } else {
        colours as usize
    };
    let Some(palette_size) = palette_len.checked_mul(4) else {
        return false;
    };
    if palette_size as u64 > record.raw().size().saturating_sub(record.raw().position()) {
        return false;
    }
    let mut palette = vec![0; palette_size];
    if palette_size != 0 && !record.raw_mut().read(&mut palette) {
        return false;
    }
    let Some(row) = (width as usize)
        .checked_mul(bits as usize)
        .and_then(|n| n.checked_add(31))
        .map(|n| (n / 32) * 4)
    else {
        return false;
    };
    let Some(size) = row.checked_mul(height as usize) else {
        return false;
    };
    if size as u64 > record.raw().size().saturating_sub(record.raw().position()) {
        return false;
    }
    let mut pixels = vec![0; size];
    if !record.raw_mut().read(&mut pixels) {
        return false;
    }
    for y in 0..height {
        for x in 0..width {
            let at = y as usize * row;
            let (rgb, index) = if bits == 24 {
                let at = at + x as usize * 3;
                ([pixels[at + 2], pixels[at + 1], pixels[at]], None)
            } else {
                let index = if bits == 8 {
                    pixels[at + x as usize]
                } else {
                    let byte = pixels[at + x as usize / 2];
                    if x % 2 == 0 {
                        byte >> 4
                    } else {
                        byte & 15
                    }
                };
                let Some(entry) = palette.get(index as usize * 4..index as usize * 4 + 4) else {
                    return false;
                };
                ([entry[2], entry[1], entry[0]], Some(index))
            };
            // Positive BMP rows bottom-up is an explicit choice pending PR11.
            if !buffer.rgb(x, height - 1 - y, rgb, index) {
                return false;
            }
        }
    }
    true
}
fn save_bmp<S: RawStream>(buffer: &dyn Buffer, record: &mut SerialRecord<S>) -> bool {
    let layout = buffer.layout();
    if layout.format.bits != 16 {
        return false;
    }
    let Some(row) = (layout.width as usize)
        .checked_mul(3)
        .and_then(|n| n.checked_add(3))
        .map(|n| n & !3)
    else {
        return false;
    };
    let Some(size) = row.checked_mul(layout.height as usize) else {
        return false;
    };
    let Ok(file_size) = u32::try_from(size.saturating_add(54)) else {
        return false;
    };
    let mut file = vec![b'B', b'M'];
    file.extend_from_slice(&file_size.to_le_bytes());
    file.extend_from_slice(&[0; 4]);
    file.extend_from_slice(&54u32.to_le_bytes());
    let mut info = Vec::new();
    for value in [40, layout.width, layout.height] {
        info.extend_from_slice(&value.to_le_bytes());
    }
    info.extend_from_slice(&[1, 0, 24, 0]);
    for value in [0, size as u32, 0, 0, 0, 0] {
        info.extend_from_slice(&value.to_le_bytes());
    }
    let mut pixels = vec![0; size];
    for y in 0..layout.height as usize {
        for x in 0..layout.width as usize {
            let at = y * layout.stride + x * 2;
            let Some(source) = buffer.bytes().get(at..at + 2) else {
                return false;
            };
            let Some(rgb) = buffer.rgb16(u16::from_le_bytes([source[0], source[1]])) else {
                return false;
            };
            let at = (layout.height as usize - 1 - y) * row + x * 3;
            pixels[at..at + 3].copy_from_slice(&[rgb[2], rgb[1], rgb[0]]);
        }
    }
    record.raw_mut().write(&file);
    record.raw_mut().write(&info);
    record.raw_mut().write(&pixels);
    true // §6.3: all three raw write results deliberately ignored.
}
