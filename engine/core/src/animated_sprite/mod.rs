// SPDX-License-Identifier: GPL-3.0-or-later
//! Specified parts of the animated sprite instance owner.
//!
//! Source: `docs/specs/animated-sprite.md` §§1–6, 10 and the published
//! basic-instance/foreign-resource contracts in `model-instances.md` §§1, 6.
//! This module is WIP, not a complete native-owner replacement. See README.md
//! for the public method inventory and AS3–AS9 for blocking contract holes.
//! Getters use shared borrows; no RefCell or original-owner callbacks.

mod frames;
mod geometry;
mod lifetime;
mod playback;

pub use frames::*;
pub use geometry::*;
pub use lifetime::*;
pub use playback::*;

use std::sync::Arc;

pub const FLAG_HIGHLIGHT: u8 = 1;
pub const FLAG_FADE: u8 = 2;
pub const FLAG_SELF_LIT: u8 = 4;
pub const BASIC_INSTANCE_IID: u32 = 0x26a6_8449;

/// Constructor defaults other than the draw state are unpublished (AS3).
/// An explicit snapshot avoids claiming invented native initial values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceState {
    pub playback: Playback,
    pub zoom: u8,
    pub rotation: u8,
    pub flags: u8,
    pub alpha: f32,
    pub highlight: u32,
    pub custom_data: u32,
    pub properties: u32,
    pub draw_attributes: [u8; 20],
    pub transform: Transform,
    pub bounds: Bounds,
}

/// Owned instance state. Resource records and owner references are typed foreign
/// values; they do not supply animation decisions or computed quad vertices.
#[derive(Debug)]
pub struct AnimatedSprite<O> {
    state: InstanceState,
    owner: Option<Arc<O>>,
    zooms: Vec<LoadedZoom>,
    frame_dirty: bool,
}

impl<O> AnimatedSprite<O> {
    pub fn from_snapshot(state: InstanceState, owner: Option<Arc<O>>) -> Self {
        Self {
            state,
            owner,
            zooms: Vec::new(),
            frame_dirty: true,
        }
    }

    pub fn state(&self) -> &InstanceState {
        &self.state
    }
    pub fn owner(&self) -> Option<&Arc<O>> {
        self.owner.as_ref()
    }
    pub fn set_owner(&mut self, owner: Option<Arc<O>>) {
        self.owner = owner;
    }
    pub fn custom_data(&self) -> u32 {
        self.state.custom_data
    }
    pub fn set_custom_data(&mut self, value: u32) {
        self.state.custom_data = value;
    }
    pub fn properties(&self) -> u32 {
        self.state.properties
    }
    pub fn set_properties(&mut self, value: u32) {
        self.state.properties = value;
    }
    pub fn draw_attributes(&self) -> &[u8; 20] {
        &self.state.draw_attributes
    }
    pub fn set_draw_attributes(&mut self, value: [u8; 20]) {
        self.state.draw_attributes = value;
    }
    pub fn transform(&self) -> Transform {
        self.state.transform
    }
    pub fn frame_dirty(&self) -> bool {
        self.frame_dirty
    }
    pub fn loaded_zooms(&self) -> &[LoadedZoom] {
        &self.zooms
    }

    /// Adopts a *prefix*, not a successful native Load result. AS4: the spec
    /// does not publish the load return or state changes at the first missing zoom.
    pub fn adopt_zoom_prefix(&mut self, prefix: ZoomPrefix) {
        self.zooms = prefix.zooms;
        self.frame_dirty = true;
    }

    pub fn set_view(&mut self, zoom: u8, rotation: u8) {
        if (self.state.zoom, self.state.rotation) != (zoom, rotation) {
            self.state.zoom = zoom;
            self.state.rotation = rotation;
            self.frame_dirty = true;
        }
    }

    pub fn set_frame(&mut self, frame: u32, sequence_length: u32) -> Result<(), PlaybackError> {
        self.state.playback.set_frame(frame, sequence_length)?;
        self.frame_dirty = true;
        Ok(())
    }

    pub fn advance_unsequenced(&mut self, frame_count: u32) -> Result<bool, PlaybackError> {
        let changed = self.state.playback.advance_unsequenced(frame_count)?;
        self.frame_dirty |= changed;
        Ok(changed)
    }

    pub fn tick_unsequenced(&mut self, elapsed: f32, frame_count: u32) -> Result<TickResult, PlaybackError> {
        let result = self.state.playback.tick_unsequenced(elapsed, frame_count)?;
        self.frame_dirty |= result.frame_changed;
        Ok(result)
    }

    pub fn set_play_mode(&mut self, mode: u8) {
        self.state.playback.play_mode = mode;
    }

    /// Raw rate. Sentinel resolution is deliberately separate: §4 only infers
    /// what SetRate(-200) means, and does not define the resource field (AS5).
    pub fn set_explicit_rate(&mut self, rate: f32) -> Result<(), PlaybackError> {
        if rate == -100.0 || rate == -200.0 {
            return Err(PlaybackError::RateSentinel);
        }
        if !rate.is_finite() {
            return Err(PlaybackError::NonFinite);
        }
        self.state.playback.rate = rate;
        Ok(())
    }

    pub fn set_fade(&mut self, on: bool, alpha: f32) {
        // AS8: §3 says "within range" without endpoints. Literal [0,1],
        // including endpoints; invalid alpha leaves both state fields intact.
        if (0.0..=1.0).contains(&alpha) {
            self.state.alpha = alpha;
            self.set_flag(FLAG_FADE, on);
        }
    }

    pub fn set_highlight(&mut self, index: u32) {
        self.state.highlight = index;
        self.set_flag(FLAG_HIGHLIGHT, index != 0);
    }

    pub fn set_self_lit(&mut self, on: bool) {
        self.set_flag(FLAG_SELF_LIT, on);
    }

    fn set_flag(&mut self, flag: u8, on: bool) {
        if on {
            self.state.flags |= flag;
        } else {
            self.state.flags &= !flag;
        }
    }

    pub fn drawing_suppressed(&self) -> bool {
        self.state.flags & FLAG_HIGHLIGHT != 0 || self.state.zoom <= 1
    }

    pub fn material(&self) -> MaterialState {
        MaterialState {
            draw: DrawState::SPRITE,
            self_lit: self.state.flags & FLAG_SELF_LIT != 0,
            transparency: (self.state.flags & FLAG_FADE != 0).then_some(self.state.alpha),
        }
        .with_fade_test()
    }
}

/// §1: symbolic constants avoid inventing native driver enum encodings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawState {
    pub colour_write: bool,
    pub depth_write: bool,
    pub depth_test: DepthTest,
    pub alpha_test: AlphaTest,
    pub alpha_reference: f32,
    pub source_blend: Blend,
    pub destination_blend: Blend,
    pub texture_count: u8,
    pub mode: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepthTest {
    LessOrEqual,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaTest {
    Greater,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blend {
    SourceAlpha,
    OneMinusSourceAlpha,
}

impl DrawState {
    pub const SPRITE: Self = Self {
        colour_write: true,
        depth_write: true,
        depth_test: DepthTest::LessOrEqual,
        alpha_test: AlphaTest::Greater,
        alpha_reference: 0.5,
        source_blend: Blend::SourceAlpha,
        destination_blend: Blend::OneMinusSourceAlpha,
        texture_count: 1,
        mode: 1,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialState {
    pub draw: DrawState,
    pub self_lit: bool,
    pub transparency: Option<f32>,
}
impl MaterialState {
    fn with_fade_test(mut self) -> Self {
        if self.transparency.is_some() {
            self.draw.alpha_reference = 0.0;
        }
        self
    }
}

#[cfg(test)]
mod tests;
