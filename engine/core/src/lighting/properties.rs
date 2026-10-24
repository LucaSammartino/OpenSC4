// SPDX-License-Identifier: GPL-3.0-or-later
use super::types::*;
use std::sync::{Arc, RwLock};

/// Typed reads from an external exemplar. None means the property is absent.
pub trait PropertyHolder: Send + Sync {
    fn float(&self, _id: u32) -> Option<f32> {
        None
    }
    fn boolean(&self, _id: u32) -> Option<bool> {
        None
    }
    fn color(&self, _id: u32) -> Option<Color> {
        None
    }
    fn pair(&self, _id: u32) -> Option<[f32; 2]> {
        None
    }
    fn unsigned(&self, _id: u32) -> Option<u32> {
        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    pub light_scale: f32,
    pub use_maps: bool,
    pub test_pattern: bool,
    pub terrain_weight: f32,
    pub debug_model: bool,
    pub day: Color,
    pub night: Color,
    pub flora_weight: f32,
    pub model_weight: f32,
    pub mobile_weight: f32,
    pub fade_color: Color,
    pub fade_strength: f32,
    pub change_threshold: f32,
    pub night_threshold: f32,
    pub decal: DecalShadowParams,
    pub azimuth: f32,
    pub elevation: f32,
    pub normal_bend: f32,
    pub slope_factor: f32,
    pub depth_fade: f32,
    pub night_reduction: f32,
}
impl Parameters {
    /// LT5: image-seeded shared configuration, before any exemplar reads.
    /// Construct one LightingStatics per process; managers share it explicitly.
    pub fn initial() -> Self {
        Self {
            terrain_weight: 0.6,
            night: WHITE,
            flora_weight: 0.6,
            model_weight: 0.6,
            mobile_weight: 0.6,
            fade_color: [0.0, 0.5, 1.0],
            fade_strength: 0.5,
            decal: DecalShadowParams {
                texture: 0x0917660e,
                a: 0.5,
                b: 0.0,
                c: 8.0,
                pair: [0.0; 2],
                color: [0.0; 3],
            },
            azimuth: 90.0,
            normal_bend: 1.0,
            ..Self::shipped()
        }
    }
    /// §1.4's shipped exemplar values, with the explicit initial values for
    /// absent properties. This is NOT a claim about unspecified static defaults (LT5).
    pub fn shipped() -> Self {
        Self {
            light_scale: 0.1,
            use_maps: true,
            test_pattern: false,
            terrain_weight: 0.8,
            debug_model: false,
            day: WHITE,
            night: [0.5, 0.5, 0.78],
            flora_weight: 0.9,
            model_weight: 0.4,
            mobile_weight: 0.7,
            fade_color: [0.875, 0.949, 1.0],
            fade_strength: 0.15,
            change_threshold: 1e-4,
            night_threshold: 0.8,
            decal: DecalShadowParams {
                texture: 0x69d3_a23b, // LT15: corrected shipped exemplar value.
                a: 0.4,
                b: 0.55,
                c: 12.0,
                pair: [0.0; 2],
                color: [0.08, 0.06, 0.23],
            },
            azimuth: 67.5,
            elevation: 45.0,
            normal_bend: 2.0,
            slope_factor: 0.7,
            depth_fade: 50.0,
            night_reduction: 0.6,
        }
    }
    pub fn read(&mut self, holder: &dyn PropertyHolder) {
        macro_rules! read {
            ($method:ident, $($field:ident = $id:expr),+ $(,)?) => {
                $(if let Some(value) = holder.$method($id) { self.$field = value; })+
            };
        }
        read!(
            float,
            light_scale = 0x29189beb,
            terrain_weight = 0x29189bee,
            flora_weight = 0x29189bf1,
            model_weight = 0x29189bf2,
            mobile_weight = 0x29189bf3,
            fade_strength = 0x29189bf5,
            change_threshold = 0x29189bf6,
            night_threshold = 0x29189bf7,
            azimuth = 0x085489ce,
            elevation = 0x085489cf,
            normal_bend = 0x085489d0,
            slope_factor = 0x085489d1,
            depth_fade = 0x085489d2,
            night_reduction = 0x085489d3
        );
        read!(
            boolean,
            use_maps = 0x29189bec,
            test_pattern = 0x29189bed,
            debug_model = 0x29189bef
        );
        read!(color, day = 0x29189bf0, night = 0x09b6d6ae, fade_color = 0x29189bf4);
        self.decal = self.decal.overridden(holder);
    }
}

/// A process can share this between managers, matching §1.4's persistent statics.
/// Getters use an RwLock; no RefCell or thread-local state.
#[derive(Clone)]
pub struct LightingStatics(Arc<RwLock<Parameters>>);
impl LightingStatics {
    pub fn new(initial: Parameters) -> Self {
        Self(Arc::new(RwLock::new(initial)))
    }
    pub fn snapshot(&self) -> Parameters {
        self.0.read().unwrap_or_else(|poison| poison.into_inner()).clone()
    }
    pub(crate) fn update(&self, f: impl FnOnce(&mut Parameters)) {
        f(&mut self.0.write().unwrap_or_else(|poison| poison.into_inner()));
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecalShadowParams {
    pub texture: u32,
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub pair: [f32; 2],
    pub color: Color,
}
impl DecalShadowParams {
    pub fn overridden(mut self, holder: &dyn PropertyHolder) -> Self {
        if let Some(value) = holder.unsigned(0x085489c8) {
            self.texture = value;
        }
        if let Some(value) = holder.float(0x085489cc) {
            self.a = value;
        }
        if let Some(value) = holder.float(0x085489cd) {
            self.b = value;
        }
        if let Some(value) = holder.float(0x085489ca) {
            self.c = value;
        }
        if let Some(value) = holder.pair(0x085489c9) {
            self.pair = value;
        }
        if let Some(value) = holder.color(0x085489cb) {
            self.color = value;
        }
        self
    }
}
