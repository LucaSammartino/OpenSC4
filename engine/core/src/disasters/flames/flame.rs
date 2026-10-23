// SPDX-License-Identifier: GPL-3.0-or-later
use super::{PlacementError, Unsupported};
use crate::camera::Vec3;
use crate::network_occupants::xform::{Xform, XFORM_MATRIX, XFORM_TRANSLATION};
use crate::stream::InStream;

/// Only the five published table entries can be requested (§1). Out-of-range
/// native table accesses are Unsupported; equal raw -1 is quiet in FireFlames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stage(u8);

impl Stage {
    pub const fn new(value: u32) -> Option<Self> {
        if value < 5 {
            Some(Self(value as u8))
        } else {
            None
        }
    }

    pub const fn value(self) -> u32 {
        self.0 as u32
    }

    pub fn effect_name(self, wall: bool) -> &'static str {
        const TOP: [&str; 5] = ["fire_1", "fire_2", "fire_3", "fire_4", "fire_5"];
        const WALL: [&str; 5] = ["wfire_1", "wfire_2", "wfire_3", "wfire_4", "wfire_5"];
        if wall {
            WALL[self.0 as usize]
        } else {
            TOP[self.0 as usize]
        }
    }
}

/// A creation reference, owned by the flame. Dropping it releases exactly that
/// reference. Implementations must not release a borrowed game pointer.
pub trait VisualEffect {
    fn start(&mut self, argument: u32);
    fn stop(&mut self, argument: u32);
    fn is_active(&mut self) -> bool;
    fn set_transform(&mut self, transform: &Xform);
}

/// The effects manager's by-name creation primitive, not an owner algorithm.
pub trait EffectFactory {
    type Effect: VisualEffect;
    fn create(&mut self, name: &str) -> Option<Self::Effect>;
}

/// Raw shared constructor word; the public spec does not establish its value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SharedPositionWord(pub f32);

/// Explicit state for restored/test compositions. Use `native` for fresh flames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlameInitial {
    pub made: u8,
    pub visible: bool,
    pub wall: bool,
    pub side: u32,
    pub position: SharedPositionWord,
}

impl FlameInitial {
    pub fn native(position: SharedPositionWord) -> Self {
        Self {
            made: 0,
            visible: true,
            wall: false,
            side: 0,
            position,
        }
    }
}

pub struct Flame<E: VisualEffect> {
    made: u8,
    visible: u8,
    wall: bool,
    side: u32,
    position: Vec3,
    transform: Xform,
    effect: Option<E>,
}

impl<E: VisualEffect> Flame<E> {
    pub fn from_initial(initial: FlameInitial) -> Self {
        Self {
            made: initial.made,
            visible: u8::from(initial.visible),
            wall: initial.wall,
            side: initial.side,
            position: [initial.position.0; 3],
            transform: Xform::default(),
            effect: None,
        }
    }

    pub fn made(&self) -> u8 {
        self.made
    }
    pub fn visible(&self) -> bool {
        self.visible != 0
    }
    pub fn visible_byte(&self) -> u8 {
        self.visible
    }
    pub fn set_visibility(&mut self, byte: u8) {
        if self.visible == byte {
            return;
        }
        self.visible = byte;
        if let Some(effect) = self.effect.as_mut() {
            if byte != 0 {
                effect.start(1);
            } else {
                effect.stop(1);
            }
        }
    }
    pub fn wall(&self) -> bool {
        self.wall
    }
    pub fn side(&self) -> u32 {
        self.side
    }
    pub fn position(&self) -> Vec3 {
        self.position
    }
    pub fn transform(&self) -> &Xform {
        &self.transform
    }
    pub fn has_effect(&self) -> bool {
        self.effect.is_some()
    }

    /// §1: stop and release the old reference before attempting creation. Failed
    /// creation leaves no effect, and never changes the placement/visible flags.
    pub fn update(&mut self, stage: Stage, factory: &mut impl EffectFactory<Effect = E>) -> Result<(), PlacementError> {
        // The full nonzero-angle FSIN/FCOS path is not qualified. Fail before
        // mutating foreign state; no portable libm result is claimed native.
        if self.side != 0 {
            return Err(PlacementError::Unsupported(Unsupported::HardwareTrig));
        }
        if let Some(old) = self.effect.as_mut() {
            old.stop(0);
        }
        // Published order is stop, clear, release, then create (not clear/stop).
        drop(self.effect.take());
        if let Some(effect) = factory.create(stage.effect_name(self.wall)) {
            self.effect = Some(effect);
            self.update_transform()?;
            if self.visible != 0 {
                self.effect.as_mut().unwrap().start(1);
            }
        }
        Ok(())
    }

    /// §1: side*90 is a radian argument, not a degrees-to-radians conversion.
    pub fn update_transform(&mut self) -> Result<(), PlacementError> {
        let Some(effect) = self.effect.as_mut() else {
            return Ok(());
        };
        if self.side != 0 {
            return Err(PlacementError::Unsupported(Unsupported::HardwareTrig));
        }
        // FF4 exact zero-angle path, including the published signed zeros.
        // Xform represents the semantic 56-byte fields; temporary padding +2/+3
        // is unwritten, never serialized/initialized or offered as native bytes.
        let temporary = Xform {
            counter: 2,
            mask: XFORM_MATRIX | XFORM_TRANSLATION,
            matrix: [1.0, -0.0, 0.0, 0.0, 1.0, -0.0, 0.0, 0.0, 1.0],
            translation: self.position,
            ..Xform::default()
        };
        effect.set_transform(&temporary);
        Ok(())
    }

    pub(crate) fn place(
        &mut self,
        position: Vec3,
        wall: bool,
        side: u32,
        view_level: u32,
    ) -> Result<(), PlacementError> {
        self.wall = wall;
        self.side = side;
        self.update_transform()?; // ViewUpdate/box wall: OLD position first
        self.position = position;
        self.update_transform()?;
        if view_level != 1 {
            self.set_visibility(0);
        }
        Ok(())
    }

    pub(crate) fn place_model(
        &mut self,
        position: Vec3,
        wall: bool,
        side: u32,
        view_level: u32,
    ) -> Result<(), PlacementError> {
        self.wall = wall;
        self.position = position;
        self.update_transform()?; // model creation: OLD side first
        self.side = side;
        self.update_transform()?;
        if view_level != 1 {
            self.set_visibility(0);
        }
        Ok(())
    }

    pub(crate) fn shutdown(&mut self) {
        if let Some(mut effect) = self.effect.take() {
            if self.made != 0 && effect.is_active() {
                effect.stop(0);
            }
            drop(effect);
        }
    }

    /// Logical direct destruction transfers the owned creation reference out;
    /// it does not call Shutdown or walk/release the foreign pointee. The caller
    /// must retain/rehome it. Rust RAII drop alone is NOT native destruction.
    pub fn into_effect(mut self) -> Option<E> {
        self.effect.take()
    }

    /// §1: no city-dependent sizes, no effect creation, no transform rebuild.
    /// Existing stream primitives preserve sticky errors and partial field reads.
    pub fn read(&mut self, stream: &mut dyn InStream) -> bool {
        self.read_fields(stream).is_some() && stream.error() == 0
    }

    fn read_fields(&mut self, stream: &mut dyn InStream) -> Option<()> {
        let version = stream.get_u16()?;
        if version < 2 {
            return None;
        }
        self.wall = stream.get_u8()? != 0;
        self.side = stream.get_u32()?;
        for value in &mut self.position {
            *value = stream.get_f32()?;
        }
        if version >= 3 {
            self.transform.read_into(stream)
        } else {
            self.transform.legacy_load_into(stream)
        }
    }
}
