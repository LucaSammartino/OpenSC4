// SPDX-License-Identifier: GPL-3.0-or-later
use super::types::*;

/// cImageARGB8 (§1.5): owned pixels, explicit width/height/row stride in pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageArgb8 {
    pixels: Vec<u32>,
    width: usize,
    height: usize,
    stride: usize,
}
impl ImageArgb8 {
    pub fn new(width: usize, height: usize, stride: usize, pixels: Vec<u32>) -> Result<Self> {
        if stride < width {
            return Err(LightingError::InvalidDimensions);
        }
        let required = stride.checked_mul(height).ok_or(LightingError::InvalidDimensions)?;
        if pixels.len() < required {
            return Err(LightingError::BufferTooShort);
        }
        Ok(Self {
            pixels,
            width,
            height,
            stride,
        })
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn stride(&self) -> usize {
        self.stride
    }
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    /// Copy decoded resource pixels, not a foreign same-owner image method.
    pub fn copy_from(source: &dyn TexturePixels) -> Result<Self> {
        let (width, height) = source.dimensions();
        let len = width.checked_mul(height).ok_or(LightingError::InvalidDimensions)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(len)
            .map_err(|_| LightingError::InvalidDimensions)?;
        for y in 0..height {
            for x in 0..width {
                pixels.push(source.pixel(x, y).ok_or(LightingError::BufferTooShort)?);
            }
        }
        Self::new(width, height, width, pixels)
    }
    pub fn make_test_pattern() -> Self {
        let pixels = (0..64 * 64)
            .map(|i| 0xff00_0000 | if i % 64 > 32 { 0xff0000 } else { 0 } | if i / 64 > 32 { 0xff00 } else { 0 })
            .collect();
        Self {
            pixels,
            width: 64,
            height: 64,
            stride: 64,
        }
    }
    pub fn find_min_red_component_value(&self) -> f32 {
        let mut minimum = 255;
        for pixel in &self.pixels {
            minimum = minimum.min((pixel >> 16) & 255);
        }
        // LT7: this product of an 8-bit integer and a binary32 constant is
        // exact in binary64; the sole final binary32 store is Init's boundary.
        (f64::from(minimum) * f64::from(f32::from_bits(0x3b808081))) as f32
    }
    fn color_linear(&self, index: usize) -> Result<Color> {
        let p = *self.pixels.get(index).ok_or(LightingError::SampleOutsideImage)?;
        Ok([
            ((p >> 16) & 255) as f32 / 255.0,
            ((p >> 8) & 255) as f32 / 255.0,
            (p & 255) as f32 / 255.0,
        ])
    }
    /// Deliberately permits x == width to spill into the following row (§2.1).
    pub fn sample(&self, u: f32, v: f32) -> Result<Color> {
        let x = self.width as f32 * u;
        let y = (self.height as f32 * v).floor();
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 || self.width == 0 || self.height == 0 {
            return Err(LightingError::SampleOutsideImage);
        }
        let row = (y as usize)
            .checked_mul(self.stride)
            .ok_or(LightingError::SampleOutsideImage)?;
        let left = row
            .checked_add(x.floor() as usize)
            .ok_or(LightingError::SampleOutsideImage)?;
        let right = row
            .checked_add(x.ceil() as usize)
            .ok_or(LightingError::SampleOutsideImage)?;
        Ok(lerp(self.color_linear(left)?, self.color_linear(right)?, x - x.floor()))
    }
    pub(crate) fn normal_pixel(&self, normal: Vector, rotation: u8) -> Result<(usize, usize)> {
        if self.width == 0 || self.height == 0 {
            return Err(LightingError::SampleOutsideImage);
        }
        // §3's 2x2 matrix is the inverse of the discrete direction rotation.
        let n = rotate(normal, (4 - rotation % 4) % 4);
        let x = (self.width as f32 * (n[0] + 1.0) * 0.5).floor();
        let y = (self.height as f32 * (n[2] + 1.0) * 0.5).floor();
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
            return Err(LightingError::SampleOutsideImage);
        }
        Ok(((x as usize).min(self.width - 1), (y as usize).min(self.height - 1)))
    }
    pub fn pixel_color(&self, x: usize, y: usize) -> Result<Color> {
        // §4.2 also preserves linear addressing when map sizes differ (LT7).
        let index = y
            .checked_mul(self.stride)
            .and_then(|i| i.checked_add(x))
            .ok_or(LightingError::SampleOutsideImage)?;
        self.color_linear(index)
    }
}
pub trait TexturePixels {
    fn dimensions(&self) -> (usize, usize);
    fn pixel(&self, x: usize, y: usize) -> Option<u32>;
}
pub trait Resources {
    fn image(&mut self, group: u32, instance: u32) -> Option<Box<dyn TexturePixels>>;
}
pub fn create_color_map(resources: &mut dyn Resources, instance: u32) -> Result<Option<ImageArgb8>> {
    resources
        .image(0xa917_9251, instance)
        .map(|image| ImageArgb8::copy_from(image.as_ref()))
        .transpose()
}
