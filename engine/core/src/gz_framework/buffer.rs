// SPDX-License-Identifier: GPL-3.0-or-later
use super::{framework::Face, Result, Unsupported};
/// The allocator supplies storage with unspecified contents. The core never
/// clears it; safe Rust hosts may initialize it to any bytes of their choice.
pub trait Allocator {
    fn allocate(&mut self, bytes: usize) -> Option<Vec<u8>>;
}
#[derive(Default, Debug)]
pub struct Buffer {
    refs: u32,
    pub destroyed: bool,
    key: u32,
    key_enabled: bool,
    pixels: Option<Vec<u8>>,
    width: u32,
    height: u32,
    colour_type: u32,
    bpp: u32,
    stride: u32,
    pub nesting: u16,
    pub lock_flags: u32,
    pub modifications: u32,
}
impl Buffer {
    pub fn query_interface(&mut self, iid: u32, out: &mut Option<Face>) -> bool {
        let face = match iid {
            1 | 0x20732180 => Face::Primary,
            0x86D72B57 => Face::Secondary,
            _ => return false,
        };
        self.add_ref();
        *out = Some(face);
        true
    }
    pub fn add_ref(&mut self) -> u32 {
        self.refs = self.refs.wrapping_add(1);
        self.refs
    }
    pub fn release(&mut self) -> u32 {
        self.refs = self.refs.wrapping_sub(1);
        if self.refs == 0 {
            self.destroyed = true;
            self.pixels = None;
        }
        self.refs
    }
    pub fn init(
        &mut self,
        allocator: &mut impl Allocator,
        width: u32,
        height: u32,
        colour_type: u32,
        bpp: u32,
    ) -> Result<bool> {
        if self.pixels.is_some() || width == 0 || height == 0 {
            return Ok(false);
        }
        let stride = width
            .checked_mul(bpp >> 3)
            .and_then(|v| v.checked_add(3))
            .map(|v| v & !3)
            .ok_or(Unsupported("GZF6"))?;
        let size = stride.checked_mul(height).ok_or(Unsupported("GZF6"))? as usize;
        let pixels = allocator
            .allocate(size)
            .filter(|p| p.len() == size)
            .ok_or(Unsupported("GZF6"))?;
        self.width = width;
        self.height = height;
        self.colour_type = colour_type;
        self.bpp = bpp;
        self.stride = stride;
        self.pixels = Some(pixels);
        Ok(true)
    }
    pub fn lock(&mut self, flags: u32) -> bool {
        if self.pixels.is_none() {
            return false;
        }
        self.nesting = self.nesting.wrapping_add(1);
        self.lock_flags |= flags;
        if flags & 0x8000 != 0 {
            self.modifications = self.modifications.wrapping_add(1);
        }
        true
    }
    pub fn unlock(&mut self, flags: u32) -> bool {
        self.nesting = self.nesting.wrapping_sub(1);
        self.lock_flags &= !flags;
        true
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn rect(&self) -> [i32; 4] {
        [0, 0, self.width as i32, self.height as i32]
    }
    pub fn colour_type(&self) -> (u32, u32) {
        (self.colour_type, self.bpp)
    }
    pub fn bytes_per_pixel(&self) -> u32 {
        self.bpp >> 3
    }
    pub fn stride(&self) -> u32 {
        self.stride
    }
    pub fn pixels(&self) -> Option<&[u8]> {
        self.pixels.as_deref()
    }
    pub fn pixels_mut(&mut self) -> Option<&mut [u8]> {
        self.pixels.as_deref_mut()
    }
    pub fn map_rgb(&self, r: u32, g: u32, b: u32) -> Result<u32> {
        // span-render-4444 §4 bridge: inputs use only their low bytes.
        let (r, g, b) = (r & 255, g & 255, b & 255);
        Ok(match self.colour_type {
            4 | 6 => 0x8000 | (r & 0xF8) << 7 | (g & 0xF8) << 2 | b >> 3,
            5 => (r & 0xF8) << 8 | (g & 0xFC) << 3 | b >> 3,
            7 => 0xF000 | (r >> 4) << 8 | (g >> 4) << 4 | b >> 4,
            8 | 9 => 0xFF000000 | r << 16 | g << 8 | b,
            _ => 0,
        })
    }
    pub fn colour_key(&self, out: &mut u32) -> Result<bool> {
        if !self.key_enabled {
            return Ok(false);
        }
        *out = self.key;
        Ok(true)
    }
    /// Fixture injection, not a claim about the uncovered native key setter.
    pub fn attach_colour_key(&mut self, key: u32, enabled: bool) {
        self.key = key;
        self.key_enabled = enabled;
    }
    pub fn uncovered_storage_operation(&self) -> Result<()> {
        Err(Unsupported("GZF6"))
    }
}
