// SPDX-License-Identifier: GPL-3.0-or-later
//! Text texture generation (§6.4), with all external interfaces injected.
//! Qualified packed-surface/UTF-8 adapter. For observable native reference,
//! access and descriptor choreography use `raw_texture` instead.

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
    let text = text.split('\0').next().unwrap_or("");
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
    // The bool adapter is only qualified for terminating dimensions. Raw API
    // reports a typed exclusion for the native infinite-loop domain.
    let Ok(layout) = super::raw_texture::text_layout(w, h) else {
        font.set_mode(mode);
        return false;
    };
    let [width, height] = layout.size.map(|v| v as u32);
    let [x, y] = layout.origin;
    let result = (|| {
    let surface = host.create_surface(width, height, 9);
    let Some(mut surface) = surface else {
        return false;
    };
    if surface.width != width
        || surface.height != height
        || surface.pixel_code != 9
        || surface.pixels.len() as u64 != u64::from(width) * u64::from(height)
    {
        return false;
    }
    surface.pixels.fill([0; 4]);
    font.draw(text, &mut surface, x, y, 0xFFFF_FFFF);
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
    host.upload(&image, texture)
    })();
    font.set_mode(mode);
    // §6.4.1: common outputs include surface creation and upload failures.
    if let Some(size) = size {
        *size = [width, height];
    }
    if let Some(rect) = rect {
        *rect = [
            layout.far[0] as f32,
            layout.far[1] as f32,
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
        (9, 0)
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
