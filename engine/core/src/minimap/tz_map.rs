// SPDX-License-Identifier: GPL-3.0-or-later
//! Nested cSC4WinMiniMap::cTZMap owner (minimap §3).
use super::{Result, Unsupported};

#[derive(Debug, Default)]
pub struct TzMap {
    pub(super) pixels: Option<Vec<u32>>,
    pub(super) width: usize,
    pub(super) height: usize,
}

impl TzMap {
    pub fn init(&mut self, width: usize, height: usize) -> Result<()> {
        if self.pixels.is_some() && (self.width, self.height) == (width, height) {
            return Ok(());
        }
        let len = width.checked_mul(height).ok_or(Unsupported::Precondition("map allocation size"))?;
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(len).map_err(|_| Unsupported::Precondition("map allocation"))?;
        // SPEC-GAP: newly allocated pixel contents are unspecified; use zero until rebuilt.
        pixels.resize(len, 0);
        self.pixels = Some(pixels);
        self.width = width;
        self.height = height;
        Ok(())
    }

    pub fn free(&mut self) {
        self.pixels = None;
    }

    pub fn pixels(&self) -> Option<&[u32]> {
        self.pixels.as_deref()
    }

    pub fn dimensions(&self) -> [usize; 2] {
        [self.width, self.height]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_dimensions_keep_pixels_and_free_allows_reallocation() {
        let mut map = TzMap::default();
        map.init(128, 128).unwrap();
        map.pixels.as_mut().unwrap()[10] = 9;
        let address = map.pixels.as_ref().unwrap().as_ptr();
        map.init(128, 128).unwrap();
        assert_eq!(map.pixels.as_ref().unwrap().as_ptr(), address);
        assert_eq!(map.pixels().unwrap()[10], 9);
        map.free();
        assert!(map.pixels().is_none());
        map.init(64, 64).unwrap();
        assert_eq!(map.pixels().unwrap().len(), 4096);
    }
}
