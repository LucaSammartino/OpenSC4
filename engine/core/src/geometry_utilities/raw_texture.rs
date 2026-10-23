// SPDX-License-Identifier: GPL-3.0-or-later
//! Public §6.4 primitive boundary. Handles are borrowed identities, not COM owners.
//! Native unchecked pointers, nonterminating sizes and uninitialized descriptors
//! are caller-domain exclusions, represented by `Unsupported`, never dereferenced.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    UnterminatedText,
    NonterminatingDimension,
    RequiredHandle,
    Descriptor,
    Backing,
    Allocation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceDescriptor {
    pub width: i32,
    pub height: i32,
    pub byte_pitch: i32,
    pub bytes_per_pixel: i32,
    pub pixel_code: u32,
    pub base: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UploadDescriptor {
    pub width: i32,
    pub height: i32,
    pub row_stride: i32,
    pub format: u32,
    pub element_type: u32,
    /// GU7: only the compressed helper's unit auxiliary dimensions are public.
    /// None leaves the plain helper's unestablished fields out of this model.
    pub auxiliary: Option<[u32; 2]>,
    pub source_offset: usize,
}

/// Only supported unsigned codes initialize outputs. The auxiliary input is unused.
pub fn texture_format(code: u32, _auxiliary: u32, format: &mut u32, ty: &mut u32) -> bool {
    let t = match code {
        0 | 2 | 3 | 5 => 14,
        4 | 6 => 10,
        7 => 13,
        8 | 9 => 1,
        _ => return false,
    };
    *format = 3;
    *ty = t;
    true
}

/// Raw calls only: the implementation owns layout, descriptors, bytes and cleanup.
/// No method may return a completed text layout, compressed image or final result.
pub trait RawTextureHost {
    fn default_font(&mut self, id: u32) -> Option<u32>;
    fn font_add_ref(&mut self, font: u32);
    fn font_release(&mut self, font: u32);
    fn font_mode(&mut self, font: u32) -> u32;
    fn font_set_mode(&mut self, font: u32, mode: u32);
    fn font_width(&mut self, font: u32, text: &[u8]) -> i32;
    fn font_height(&mut self, font: u32, text: &[u8]) -> i32;
    fn font_draw(&mut self, font: u32, text: &[u8], surface: u32, xy: [i32; 2], colour: u32) -> bool;
    fn surface_line(&mut self, surface: u32, from: [i32; 2], to: [i32; 2], colour: u32) -> bool;
    fn graphics_system(&mut self) -> bool;
    /// A false result can still return a nonnull surface requiring Release.
    fn surface_factory(&mut self) -> (bool, Option<u32>);
    fn surface_create(&mut self, surface: u32, width: i32, height: i32, code: u32) -> bool;
    fn surface_release(&mut self, surface: u32);
    fn descriptor(&mut self, surface: u32) -> SurfaceDescriptor;
    fn access(&mut self, surface: u32, flags: u32) -> bool;
    fn end_access(&mut self, surface: u32, flags: u32);
    fn bytes(&mut self, surface: u32) -> &mut [u8];
    fn acquire_service(&mut self) -> bool;
    fn cleanup_service(&mut self);
    fn upload(&mut self, texture: u32, descriptor: UploadDescriptor, bytes: &[u8]) -> bool;
    fn allocate_temporary(&mut self, len: usize) -> Option<Vec<u8>>;
    fn free_temporary(&mut self, bytes: Vec<u8>);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextLayout {
    pub size: [i32; 2],
    pub origin: [i32; 2],
    pub far: [i32; 2],
}

pub fn text_layout(w: i32, h: i32) -> Result<TextLayout, Unsupported> {
    fn dimension(n: i32) -> Result<i32, Unsupported> {
        let target = n.wrapping_add(2);
        if target > (1 << 30) {
            return Err(Unsupported::NonterminatingDimension);
        }
        let mut result = 1i32;
        while result < target {
            result = result.wrapping_mul(2);
        }
        Ok(result)
    }
    let size = [dimension(w)?, dimension(h)?];
    let origin = [
        (size[0].wrapping_sub(w) / 2).wrapping_add(1),
        (size[1].wrapping_sub(h) / 2).wrapping_sub(1),
    ];
    Ok(TextLayout {
        size,
        origin,
        far: [origin[0].wrapping_add(w), origin[1].wrapping_add(h)],
    })
}

/// `text` must contain its byte NUL. Unsupported native domains are errors,
/// not claims about native false/rollback behavior. Size/rect are written only
/// for defined executions, in the common epilogue after mode restoration.
#[allow(clippy::too_many_arguments)]
pub fn create_text_raw<H: RawTextureHost>(
    host: &mut H,
    text: &[u8],
    texture: u32,
    font: Option<u32>,
    size: Option<&mut [i32; 2]>,
    rect: Option<&mut [f32; 4]>,
    compressed: bool,
) -> Result<bool, Unsupported> {
    let length = text.iter().position(|&b| b == 0).ok_or(Unsupported::UnterminatedText)?;
    let text = &text[..length];
    let Some(font) = font.or_else(|| host.default_font(0x6896_3c4c)) else {
        return Ok(false);
    };
    host.font_add_ref(font);
    let mode = host.font_mode(font);
    host.font_set_mode(font, 3);
    let w = host.font_width(font, text);
    let h = host.font_height(font, text);
    let layout = text_layout(w, h);
    let mut surface = None;
    let result = (|| {
        let layout = layout?;
        if !host.graphics_system() {
            return Ok(false);
        }
        let (created, acquired) = host.surface_factory();
        surface = acquired;
        if !created {
            return Ok(false);
        }
        let s = surface.ok_or(Unsupported::RequiredHandle)?;
        if !host.surface_create(s, layout.size[0], layout.size[1], 9) {
            return Ok(false);
        }
        for (flags, border) in [(0x8001, false), (0x8004, true)] {
            if border {
                let _ = host.font_draw(font, text, s, layout.origin, u32::MAX);
            }
            if host.access(s, flags) {
                let d = host.descriptor(s);
                let cleared = if border {
                    let [width, height] = layout.size;
                    for (from, to) in [
                        ([0, 0], [width - 1, 0]),
                        ([0, height - 1], [width - 1, height - 1]),
                        ([0, 0], [0, height - 1]),
                        ([width - 1, 0], [width - 1, height - 1]),
                    ] {
                        let _ = host.surface_line(s, from, to, 0);
                    }
                    Ok(())
                } else {
                    clear_pixels(host.bytes(s), d)
                };
                host.end_access(s, flags);
                cleared?;
            }
        }
        if compressed {
            upload_compressed_raw(host, s, texture)
        } else {
            upload_plain_raw(host, Some(s), Some(texture), None)
        }
    })();
    host.font_set_mode(font, mode);
    if let (Ok(layout), Ok(_)) = (layout, &result) {
        if let Some(size) = size {
            *size = layout.size;
        }
        if let Some(rect) = rect {
            rect[2] = layout.origin[0] as f32;
            rect[3] = layout.origin[1] as f32;
            rect[0] = layout.far[0] as f32;
            rect[1] = layout.far[1] as f32;
        }
    }
    if let Some(surface) = surface {
        host.surface_release(surface);
    }
    host.font_release(font);
    result
}

fn row_range(
    d: SurfaceDescriptor,
    y: i32,
    x: i32,
    width: i32,
    len: usize,
) -> Result<std::ops::Range<usize>, Unsupported> {
    if width < 0 || d.bytes_per_pixel <= 0 {
        return Err(Unsupported::Backing);
    }
    let start = i64::try_from(d.base).map_err(|_| Unsupported::Backing)?
        + i64::from(y) * i64::from(d.byte_pitch)
        + i64::from(x) * i64::from(d.bytes_per_pixel);
    let end = start + i64::from(width) * i64::from(d.bytes_per_pixel);
    if start < 0 || end < start || end > len as i64 || start > i64::from(i32::MAX) {
        return Err(Unsupported::Backing);
    }
    Ok(start as usize..end as usize)
}

fn clear_pixels(bytes: &mut [u8], d: SurfaceDescriptor) -> Result<(), Unsupported> {
    if d.width <= 0 || d.height <= 0 || d.pixel_code != 9 || d.bytes_per_pixel != 4 {
        return Err(Unsupported::Descriptor);
    }
    for y in 0..d.height {
        let r = row_range(d, y, 0, d.width, bytes.len())?;
        bytes[r].fill(0);
    }
    Ok(())
}

pub fn upload_plain_raw<H: RawTextureHost>(
    host: &mut H,
    surface: Option<u32>,
    texture: Option<u32>,
    rect: Option<[i32; 4]>,
) -> Result<bool, Unsupported> {
    if !host.acquire_service() {
        return Err(Unsupported::RequiredHandle);
    }
    let result = (|| {
        let (Some(s), Some(texture)) = (surface, texture) else {
            return Ok(false);
        };
        if !host.access(s, 0x8040) {
            return Ok(false);
        }
        let d = host.descriptor(s);
        let result = (|| {
            let mut format = 0;
            let mut element_type = 0;
            if !texture_format(d.pixel_code, 0, &mut format, &mut element_type)
                || d.bytes_per_pixel == 0
                || (d.byte_pitch == i32::MIN && d.bytes_per_pixel == -1)
            {
                // Native ignores the helper boolean and reads unset fields. Do not do that in Rust.
                return Err(Unsupported::Descriptor);
            }
            let [left, top, right, bottom] = rect.unwrap_or([0, 0, d.width, d.height]);
            let width = right.wrapping_sub(left);
            let height = bottom.wrapping_sub(top);
            if width <= 0 || height <= 0 {
                return Err(Unsupported::Backing);
            }
            let bytes = host.bytes(s).to_vec();
            for y in 0..height {
                row_range(d, top.wrapping_add(y), left, width, bytes.len())?;
            }
            let source_offset = row_range(d, top, left, width, bytes.len())?.start;
            let descriptor = UploadDescriptor {
                width,
                height,
                row_stride: d.byte_pitch / d.bytes_per_pixel,
                format,
                element_type,
                auxiliary: None, // GU7: not specified for plain uploads.
                source_offset,
            };
            Ok(host.upload(texture, descriptor, &bytes))
        })();
        host.end_access(s, 0x8040);
        result
    })();
    host.cleanup_service();
    result
}

pub fn upload_compressed_raw<H: RawTextureHost>(host: &mut H, surface: u32, texture: u32) -> Result<bool, Unsupported> {
    let mut d = host.descriptor(surface);
    if d.pixel_code != 9 || d.width & 3 != 0 || d.height & 3 != 0 {
        return Ok(false);
    }
    if !host.access(surface, 0x8040) {
        return Ok(false);
    }
    let result = (|| {
        if d.width <= 0 || d.height <= 0 || d.bytes_per_pixel != 4 {
            return Err(Unsupported::Descriptor);
        }
        d.byte_pitch = ((d.byte_pitch as u32 >> 2) * 4) as i32;
        if d.byte_pitch <= 0 {
            return Err(Unsupported::Backing);
        }
        let bytes = host.bytes(surface).to_vec();
        for y in 0..d.height {
            row_range(d, y, 0, d.width, bytes.len())?;
        }
        let len = usize::try_from(i64::from(d.width) * i64::from(d.height)).map_err(|_| Unsupported::Allocation)?;
        let mut packed = host.allocate_temporary(len).ok_or(Unsupported::Allocation)?;
        if packed.len() != len {
            host.free_temporary(packed);
            return Err(Unsupported::Allocation);
        }
        let mut offset = 0;
        for by in (0..d.height).step_by(4) {
            for bx in (0..d.width).step_by(4) {
                let mut alpha = 0u64;
                for y in 0..4 {
                    let row = row_range(d, by + y, bx, 4, bytes.len())?.start;
                    for x in 0..4 {
                        alpha |= u64::from(bytes[row + x * 4 + 3] >> 4) << (4 * (y as usize * 4 + x));
                    }
                }
                packed[offset..offset + 8].copy_from_slice(&alpha.to_le_bytes());
                packed[offset + 8..offset + 16].copy_from_slice(&[0, 0, 255, 255, 0x55, 0x55, 0x55, 0x55]);
                offset += 16;
            }
        }
        let result = host.upload(
            texture,
            UploadDescriptor {
                width: d.width,
                height: d.height,
                row_stride: d.width,
                format: 9,
                element_type: 0,
                auxiliary: Some([1, 1]),
                source_offset: 0,
            },
            &packed,
        );
        host.free_temporary(packed);
        Ok(result)
    })();
    host.end_access(surface, 0x8040);
    result
}
