// SPDX-License-Identifier: GPL-3.0-or-later
//! Widget-facing GZ graphics, from gz-graphics.md. See README.md for closure.
//! Arc owns foreign references; getters borrow them. No native layouts or ABI.
mod alpha;
mod canvas_contract;
mod context;
mod driver_contract;
mod engine;
mod geometry;
mod image_contract;
mod services;
pub use alpha::{alpha_blt, alpha_blt_no_clip, AlphaSource};
pub use alpha::{clipped_alpha_rectangles, generic_buffer_clip, generic_buffer_clip_stretch, AlphaFileIo};
pub use canvas_contract::*;
pub use context::*;
pub use driver_contract::{DriverSettings, UiArgument};
pub use engine::*;
pub use geometry::{cubic_segments, fixed_point, polygon_spans, PixelSpan};
pub use image_contract::{ImageBinding, ImageCache, ImageSource, ImageTexture};
pub use services::*;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Detached,
    InvalidInput,
    Foreign,
    Unsupported(&'static str),
}
pub type Result<T> = std::result::Result<T, Error>;
pub type Point = [f32; 2];
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect(pub i32, pub i32, pub i32, pub i32);
impl Rect {
    pub fn intersect(self, b: Self) -> Self {
        let r = Self(self.0.max(b.0), self.1.max(b.1), self.2.min(b.2), self.3.min(b.3));
        if r.0 >= r.2 || r.1 >= r.3 {
            Self::default()
        } else {
            r
        }
    }
    pub fn width(self) -> i32 {
        self.2.wrapping_sub(self.0)
    }
    pub fn height(self) -> i32 {
        self.3.wrapping_sub(self.1)
    }
    pub fn points(self) -> [Point; 2] {
        [[self.0 as f32, self.1 as f32], [self.2 as f32, self.3 as f32]]
    }
}

/// Query failure deliberately preserves the output. Arc cloning is AddRef;
/// dropping a retained Arc is Release. Service-base IIDs belong to the host.
pub fn query<T: ?Sized>(object: &Arc<T>, iid: u32, supported: &[u32], out: &mut Option<Arc<T>>) -> bool {
    if supported.contains(&iid) {
        *out = Some(Arc::clone(object));
        true
    } else {
        false
    }
}
pub const CONTEXT_IIDS: &[u32] = &[1, 0x2ae63219];
pub const ENGINE_IIDS: &[u32] = &[1, 0x2ae78a07];
pub const GRAPHICS_IIDS: &[u32] = &[0x0073283c, 0xc47b747c, 0x03fa40bf, 0xe98b2f57];
pub const CANVAS_IIDS: &[u32] = &[0x66b217a1, 0xe46397db, 0x3ac7a210];
pub const ALPHA_IIDS: &[u32] = &[0xe3260321];
pub const GRAPHICS_CLASS: u32 = 0xc416025c;
pub const GRAPHICS_PRIORITY: u32 = 1_500_000;
pub const ALPHA_CLASS: u32 = 0x44136fdc;
pub const ALPHA_PRIORITY: u32 = 0xfff0bdc0;
pub const CANVAS_CLASS: u32 = 0xa3460deb;
pub const CANVAS_PRIORITY: u32 = 0x1312cf;

#[cfg(test)]
mod tests;
