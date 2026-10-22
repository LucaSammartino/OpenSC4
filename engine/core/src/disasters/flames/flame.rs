// SPDX-License-Identifier: GPL-3.0-or-later
use crate::camera::Vec3;
use crate::network_occupants::xform::{Xform, XFORM_MATRIX, XFORM_TRANSLATION};
use crate::stream::InStream;

/// Only the five published table entries can be requested (§1). Out-of-range
/// native stages, including the occupant's initial -1, remain FF2.
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
    fn start(&mut self);
    fn stop(&mut self);
    fn set_transform(&mut self, transform: &Xform);
}

/// The effects manager's by-name creation primitive, not an owner algorithm.
pub trait EffectFactory {
    type Effect: VisualEffect;
    fn create(&mut self, name: &str) -> Option<Self::Effect>;
}

/// FF2: the constructor defaults are unpublished. Require explicit input rather
/// than inventing them. These are construction inputs, not host-computed output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlameInitial {
    pub made: u8,
    pub visible: bool,
    pub wall: bool,
    pub side: u32,
}

pub struct Flame<E: VisualEffect> {
    made: u8,
    visible: bool,
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
            visible: initial.visible,
            wall: initial.wall,
            side: initial.side,
            position: [0.0; 3],
            transform: Xform::default(),
            effect: None,
        }
    }

    pub fn made(&self) -> u8 {
        self.made
    }
    pub fn visible(&self) -> bool {
        self.visible
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
    pub fn update(&mut self, stage: Stage, factory: &mut impl EffectFactory<Effect = E>) {
        if let Some(mut old) = self.effect.take() {
            old.stop();
            drop(old);
        }
        if let Some(mut effect) = factory.create(stage.effect_name(self.wall)) {
            effect.set_transform(&self.transform);
            if self.visible {
                effect.start();
            }
            self.effect = Some(effect);
        }
    }

    /// §1: side*90 is a radian argument, not a degrees-to-radians conversion.
    pub fn update_transform(&mut self) {
        // FF4: positive-y right-handed signs and f64 trig -> f32 stores. The
        // spec publishes the transform's value, not its native change counter.
        let angle = f64::from(self.side) * 90.0;
        let (sin, cos) = angle.sin_cos();
        let (sin, cos) = (sin as f32, cos as f32);
        self.transform = Xform {
            mask: XFORM_MATRIX | XFORM_TRANSLATION,
            matrix: [cos, 0.0, sin, 0.0, 1.0, 0.0, -sin, 0.0, cos],
            translation: self.position,
            ..Xform::default()
        };
        if let Some(effect) = self.effect.as_mut() {
            effect.set_transform(&self.transform);
        }
    }

    pub(crate) fn place(&mut self, position: Vec3, wall: bool, side: u32, view_level: u32) {
        self.position = position;
        self.wall = wall;
        self.side = side;
        self.update_transform();
        if view_level != 1 && self.visible {
            // FF4: hiding clears the flag as well as stopping the live effect.
            self.visible = false;
            if let Some(effect) = self.effect.as_mut() {
                effect.stop();
            }
        }
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
