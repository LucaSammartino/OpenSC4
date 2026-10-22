// SPDX-License-Identifier: GPL-3.0-or-later
//! Text texture generation (§6.4), with all external interfaces injected.
//! GU4: the host owns native reference/resource handling; core owns pixel data.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Surface {
    pub width: u32,
    pub height: u32,
    pub pixel_code: u32,
    /// Packed BGRA pixels, row-major. GU4: no native pitch is inferred.
    pub pixels: Vec<[u8; 4]>,
}
impl Surface {
    pub fn new(width: u32, height: u32) -> Option<Self> {
        let count = usize::try_from(u64::from(width) * u64::from(height)).ok()?;
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(count).ok()?;
        pixels.resize(count, [0; 4]);
        Some(Self {
            width,
            height,
            pixel_code: 9,
            pixels,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureImage {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub element_type: u32,
    pub bytes: Vec<u8>,
}
pub trait TextFont {
    fn mode(&self) -> u32;
    fn set_mode(&mut self, mode: u32);
    fn width(&mut self, text: &str) -> i32;
    fn height(&mut self) -> i32;
    fn draw(&mut self, text: &str, surface: &mut Surface, x: i32, y: i32, colour: u32);
}
pub trait TextureHost {
    type Font: TextFont;
    type Texture;
    /// Framework font source font 0x68963C4C; the handle manages its ownership.
    fn default_font(&mut self, id: u32) -> Option<Self::Font>;
    /// Graphics-system surface request, pixel code 9.
    fn create_surface(&mut self, width: u32, height: u32, pixel_code: u32) -> Option<Surface>;
    /// Texture-binding factory slot 4; returns the actual upload result.
    fn upload(&mut self, image: &TextureImage, texture: &mut Self::Texture) -> bool;
}

#[allow(clippy::too_many_arguments)]
pub fn create_texture_from_text<H: TextureHost>(
    text: &str,
    texture: &mut H::Texture,
    font: Option<&mut H::Font>,
    size: Option<&mut [u32; 2]>,
    rect: Option<&mut [f32; 4]>,
    compressed: bool,
    host: &mut H,
) -> bool {
    if let Some(font) = font {
        create_with_font(text, texture, font, size, rect, compressed, host)
    } else if let Some(mut font) = host.default_font(0x6896_3C4C) {
        create_with_font(text, texture, &mut font, size, rect, compressed, host)
    } else {
        false
    }
}

#[allow(clippy::too_many_arguments)]
fn create_with_font<H: TextureHost>(
    text: &str,
    texture: &mut H::Texture,
    font: &mut H::Font,
    size: Option<&mut [u32; 2]>,
    rect: Option<&mut [f32; 4]>,
    compressed: bool,
    host: &mut H,
) -> bool {
    let mode = font.mode();
    font.set_mode(3);
    let w = font.width(text);
    let h = font.height();
    // GU4: reject negative/unrepresentable dimensions, no guessed wraparound.
    let dimension = |n: i32| u32::try_from(n).ok()?.checked_add(2)?.checked_next_power_of_two();
    let dimensions = dimension(w).zip(dimension(h));
    let Some((width, height)) = dimensions else {
        font.set_mode(mode);
        return false;
    };
    let x = ((i64::from(width) - i64::from(w)) / 2 + 1) as i32;
    let y = ((i64::from(height) - i64::from(h)) / 2 - 1) as i32;
    let surface = host.create_surface(width, height, 9);
    let Some(mut surface) = surface else {
        font.set_mode(mode);
        return false;
    };
    if surface.width != width
        || surface.height != height
        || surface.pixel_code != 9
        || surface.pixels.len() as u64 != u64::from(width) * u64::from(height)
    {
        font.set_mode(mode);
        return false;
    }
    surface.pixels.fill([0; 4]);
    font.draw(text, &mut surface, x, y, 0xFFFF_FFFF);
    font.set_mode(mode);
    let width_usize = width as usize;
    for row in 0..height as usize {
        for column in 0..width_usize {
            if row == 0 || row == height as usize - 1 || column == 0 || column == width_usize - 1 {
                surface.pixels[row * width_usize + column] = [0; 4];
            }
        }
    }
    let Some(image) = encode_surface(&surface, compressed) else {
        return false;
    };
    let result = host.upload(&image, texture);
    // GU4: step 5 outputs are written even when upload reports failure.
    if let Some(size) = size {
        *size = [width, height];
    }
    if let Some(rect) = rect {
        *rect = [
            (i64::from(x) + i64::from(w)) as f32,
            (i64::from(y) + i64::from(h)) as f32,
            x as f32,
            y as f32,
        ];
    }
    result
}

fn encode_surface(surface: &Surface, compressed: bool) -> Option<TextureImage> {
    let mut bytes = Vec::new();
    let (format, element_type) = if compressed {
        if surface.pixel_code != 9 || surface.width % 4 != 0 || surface.height % 4 != 0 {
            return None;
        }
        for by in (0..surface.height as usize).step_by(4) {
            for bx in (0..surface.width as usize).step_by(4) {
                let mut alpha = 0u64;
                for y in 0..4 {
                    for x in 0..4 {
                        let a = surface.pixels[(by + y) * surface.width as usize + bx + x][3] >> 4;
                        alpha |= u64::from(a) << (4 * (y * 4 + x));
                    }
                }
                bytes.extend_from_slice(&alpha.to_le_bytes());
                bytes.extend_from_slice(&[0, 0, 255, 255, 0x55, 0x55, 0x55, 0x55]);
            }
        }
        (9, 1)
    } else {
        // This public helper always requests code 9, whose type is bytes (1).
        // No assumptions about other pixel-code storage are needed here.
        for pixel in &surface.pixels {
            bytes.extend_from_slice(pixel);
        }
        (3, 1)
    };
    Some(TextureImage {
        width: surface.width,
        height: surface.height,
        format,
        element_type,
        bytes,
    })
}

#[cfg(test)]
#[path = "texture_tests.rs"]
mod tests;
