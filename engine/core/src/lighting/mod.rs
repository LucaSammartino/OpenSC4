// SPDX-License-Identifier: GPL-3.0-or-later
//! Lighting and hill shadows, implemented solely from docs/specs/lighting.md.
//! Typed providers expose external pixels, properties and terrain, never original
//! lighting methods. See README.md for owner closure and the LT12/LT13 numeric
//! limits; no specified owner algorithm is delegated to a provider.
//! Safe API errors for invalid buffers/indices replace original unchecked reads;
//! they do not clamp valid samples or conceal missing own-class logic (LT7).

mod colors;
#[cfg(test)]
mod contracts;
mod image;
mod projection;
mod properties;
mod register;
mod shadows;
#[cfg(test)]
mod tests;
mod types;

pub use image::*;
pub use projection::{fit_shadow_plane, ProjectionTransform};
pub use properties::*;
pub use register::RegisterPrecision;
pub use shadows::LightingMessage;
use std::sync::Arc;
pub use types::*;

/// §2.4: delivery-time visual reads, not the posted tick payload or saved time.
/// The view calls slot 6 only if its renderer and lighting interface exist.
pub fn visual_time_argument(hour: u32, minute: u32, precision: RegisterPrecision) -> f32 {
    let number = |value| register::Number::new(value, precision);
    (number(hour as f32) + number(minute as f32) * number(f32::from_bits(0x3c88_8889))).store()
}

pub const CLASS_ID: u32 = 0x681b_d52b;
pub const LIGHTING_IID: u32 = 0x681b_d52a;
pub const MESSAGE_TARGET_IID: u32 = 0x4522_94aa;
pub const SUBSCRIPTIONS: [u32; 7] = [
    0xc53d10aa, 0xc9f775bb, 0x2b96b3ea, 0xa6b79601, 0xea3b0af1, 0x6a3b0afd, 0x278e6003,
];

pub struct LightingManager {
    statics: LightingStatics,
    register_precision: RegisterPrecision,
    refs: u32,
    released: bool,
    initialized: bool,
    initialization_interrupted: bool,
    view_applied: bool,
    exemplar: Option<Arc<dyn PropertyHolder>>,
    terrain: Option<Arc<dyn Terrain>>,
    city_terrain: Option<Arc<dyn Terrain>>,
    time: f32,
    month: u8,
    night: bool,
    day_keys: Vec<Color>,
    strength_keys: Vec<f32>,
    fade_keys: Vec<Color>,
    fill: Vector,
    key: Vector,
    shadow_direction: Vector,
    scaled_light: Color,
    rotation: u8,
    map_rotation_initialized: bool,
    zoom: u8,
    global_light: Color,
    day_light: Color,
    announced_global: Color,
    announced_day: Color,
    day_fraction: f32,
    night_fraction: f32,
    maps: [Option<ImageArgb8>; 4],
    minimum_red: f32,
    sun: Vector,
    axes: [Color; 6],
    cells: (usize, usize),
    fade_origin: Vector,
    fade_gradient: Vector,
    fade_strength: f32,
    fade_color: Color,
    tint: Option<(Color, f32)>,
    heights: Vec<f32>,
    shadows: Vec<f32>,
    long_operation: bool,
    dirty: Rect,
    changed: Rect,
    shadows_on: bool,
}

impl LightingManager {
    pub fn new(statics: LightingStatics, register_precision: RegisterPrecision) -> Self {
        // LT5: geometry/map slots without native defaults use safe empty storage.
        // LT12: the caller supplies inherited register precision; no guessed CW.
        Self {
            statics,
            register_precision,
            refs: 0,
            released: false,
            initialized: false,
            initialization_interrupted: false,
            view_applied: false,
            exemplar: None,
            terrain: None,
            city_terrain: None,
            time: 12.0,
            month: 0,
            night: false,
            day_keys: Vec::new(),
            strength_keys: Vec::new(),
            fade_keys: Vec::new(),
            fill: [0.0; 3],
            key: [0.0; 3],
            shadow_direction: [0.0; 3],
            scaled_light: [0.0; 3],
            rotation: 0,
            map_rotation_initialized: false,
            zoom: 0,
            global_light: WHITE,
            day_light: WHITE,
            announced_global: [0.0; 3],
            announced_day: [0.0; 3],
            day_fraction: 1.0,
            night_fraction: 0.0,
            maps: std::array::from_fn(|_| None),
            minimum_red: 0.0,
            sun: [1.0, -1.0, -1.0],
            axes: [[0.0; 3]; 6],
            cells: (0, 0),
            fade_origin: [0.0; 3],
            fade_gradient: [0.0; 3],
            fade_strength: 0.0,
            fade_color: [0.0; 3],
            tint: None,
            heights: Vec::new(),
            shadows: Vec::new(),
            long_operation: false,
            dirty: Rect::EMPTY,
            changed: Rect::EMPTY,
            shadows_on: false,
        }
    }
    pub fn query_interface(&mut self, iid: u32) -> Result<Option<Interface>> {
        let interface = match iid {
            1 | LIGHTING_IID => Interface::Lighting,
            MESSAGE_TARGET_IID => Interface::MessageTarget,
            _ => return Ok(None),
        };
        // LT5: successful QueryInterface acquires one reference.
        self.add_ref()?;
        Ok(Some(interface))
    }
    pub fn add_ref(&mut self) -> Result<u32> {
        if self.released {
            return Err(LightingError::Released);
        }
        self.refs = self.refs.checked_add(1).ok_or(LightingError::ReferenceOverflow)?;
        Ok(self.refs)
    }
    pub fn release(&mut self) -> u32 {
        if self.refs > 1 {
            self.refs -= 1;
        } else {
            self.destroy();
        }
        self.refs
    }
    fn destroy(&mut self) {
        self.refs = 0;
        self.released = true;
        self.city_terrain = None;
        self.terrain = None;
        self.exemplar = None;
        // LT7: native destruction is NOT Shutdown. Un-shut-down maps leak;
        // do not invent map cleanup or unsubscribe in the destructor.
        for map in &mut self.maps {
            if let Some(map) = map.take() {
                std::mem::forget(map);
            }
        }
        self.day_keys = Vec::new();
        self.strength_keys = Vec::new();
        self.fade_keys = Vec::new();
        self.heights = Vec::new();
        self.shadows = Vec::new();
    }
    /// §1.3: resources are destroyed for either flag. Bit 0 frees the owned
    /// allocation; otherwise return the destroyed storage to its owner.
    pub fn deleting_destructor(mut self: Box<Self>, flags: u32) -> Option<Box<Self>> {
        self.destroy();
        if flags & 1 != 0 {
            None
        } else {
            Some(self)
        }
    }

    pub fn init(
        &mut self,
        shadow_setting: u32,
        exemplar: Option<Arc<dyn PropertyHolder>>,
        resources: &mut dyn Resources,
        events: &mut dyn EventSink,
    ) -> Result<bool> {
        if self.released {
            return Err(LightingError::Released);
        }
        if self.initialization_interrupted {
            return Err(LightingError::InitializationInterrupted);
        }
        if self.initialized {
            return Ok(false);
        }
        self.shadows_on = shadow_setting >= 2;
        self.exemplar = exemplar;
        self.read_properties();
        if self.statics.snapshot().test_pattern {
            self.maps[0] = Some(ImageArgb8::make_test_pattern());
            self.maps[1] = Some(ImageArgb8::make_test_pattern());
        } else {
            for (index, instance) in [0x0917660c, 0x0917660d, 0x0917660e, 0x0917660f].into_iter().enumerate() {
                self.maps[index] = match create_color_map(resources, instance) {
                    Ok(map) => map,
                    Err(error) => {
                        // LT7: invalid foreign decoder output interrupts Init;
                        // safe Rust latches it, rather than inventing a retry.
                        self.initialization_interrupted = true;
                        return Err(error);
                    }
                };
            }
            if let Some(image) = &self.maps[2] {
                self.minimum_red = image.find_min_red_component_value();
            }
        }
        for id in SUBSCRIPTIONS {
            events.emit(Event::Subscribe(id));
        }
        self.initialized = true;
        Ok(true)
    }
    pub fn shutdown(&mut self, events: &mut dyn EventSink) -> bool {
        if !self.initialized {
            return false;
        }
        for id in SUBSCRIPTIONS {
            events.emit(Event::Unsubscribe(id));
        }
        self.view_applied = false;
        self.exemplar = None;
        self.terrain = None;
        self.maps = std::array::from_fn(|_| None);
        self.initialized = false;
        true
    }
    pub fn read_properties(&mut self) {
        if let Some(holder) = &self.exemplar {
            self.statics.update(|p| p.read(holder.as_ref()));
        }
        let p = self.statics.snapshot();
        self.day_light = p.day;
        self.global_light = p.day;
        self.scaled_light = scale(p.day, p.light_scale);
        self.fade_strength = p.fade_strength;
        self.day_keys = vec![p.night, p.night, p.day, p.day, p.day, p.day, p.night, p.night];
        self.strength_keys = vec![0.0, p.fade_strength, p.fade_strength, 0.0];
        self.fade_keys = vec![p.fade_color];
        let a = p.azimuth * 0.017453292;
        let e = p.elevation * 0.017453292;
        self.sun = [a.sin() * e.cos(), -e.sin(), -a.cos() * e.cos()];
    }
    pub fn get_time_of_day(&self) -> f32 {
        self.time
    }
    fn map_normal(&self, normal: Vector) -> Vector {
        if self.map_rotation_initialized {
            normal
        } else {
            // LT5: the constructor's stored 2x2 is zero, not identity. Preserve
            // zero products (and hence nonfinite propagation) before first view.
            let zero = 0.0 * normal[0] + 0.0 * normal[2];
            [zero, normal[1], zero]
        }
    }
    pub fn month(&self) -> u8 {
        self.month
    }
    pub fn is_night(&self) -> bool {
        self.night
    }
    pub fn get_global_light(&self) -> Color {
        self.global_light
    }
    pub fn day_light(&self) -> Color {
        self.day_light
    }
    pub fn fractions(&self) -> (f32, f32) {
        (self.day_fraction, self.night_fraction)
    }
    pub fn get_shadow_direction(&self) -> Vector {
        self.shadow_direction
    }
    pub fn initialized(&self) -> bool {
        self.initialized
    }
    pub fn view_applied(&self) -> bool {
        self.view_applied
    }
    pub fn shadows_enabled(&self) -> bool {
        self.shadows_on
    }

    pub fn set_time_dependent_lighting_values(&mut self, suppress_fade: bool) -> Result<()> {
        let f = self.time * (1.0 / 24.0);
        let v = (f32::from(self.month) + 0.5) * (1.0 / 12.0);
        self.day_light = match &self.maps[2] {
            Some(map) => map.sample(f, v)?,
            None => color_key(&self.day_keys, f)?,
        };
        if suppress_fade {
            self.fade_color = [0.0; 3];
            self.fade_strength = 0.0;
        } else {
            self.fade_color = match &self.maps[3] {
                Some(map) => map.sample(f, v)?,
                None => color_key(&self.fade_keys, f)?,
            };
            self.fade_strength = scalar_key(&self.strength_keys, f)?;
        }
        self.global_light = self.day_light;
        if let Some((color, amount)) = self.tint {
            self.global_light = lerp(self.global_light, color, amount);
            self.fade_color = lerp(self.fade_color, color, amount);
        }
        self.scaled_light = scale(self.global_light, self.statics.snapshot().light_scale);
        Ok(())
    }
    /// month_one_based is the external city clock's month, read only on wrap.
    /// suppress_fade is the raw settings flag in LT1, not a guessed setting name.
    pub fn set_time_of_day(
        &mut self,
        time: f32,
        month_one_based: u8,
        suppress_fade: bool,
        events: &mut dyn EventSink,
    ) -> Result<bool> {
        if !time.is_finite() || !(0.0..=24.0).contains(&time) {
            return Err(LightingError::InvalidTime);
        }
        if time < self.time {
            if !(1..=12).contains(&month_one_based) {
                return Err(LightingError::InvalidMonth);
            }
            self.month = month_one_based - 1;
        }
        self.time = time;
        self.set_time_dependent_lighting_values(suppress_fade)?;
        let p = self.statics.snapshot();
        // LT6: all differences/squares/sums and sqrt remain in registers.
        let r = |x| register::Number::new(x, self.register_precision);
        let delta: [_; 3] = std::array::from_fn(|i| r(self.announced_global[i]) - r(self.global_light[i]));
        let distance = ((delta[2] * delta[2] + delta[1] * delta[1]) + delta[0] * delta[0]).sqrt();
        if distance.partial_cmp(&r(p.change_threshold)) != Some(std::cmp::Ordering::Greater) {
            return Ok(false);
        }
        if self.terrain.is_some() {
            let was_night = self.announced_day[0] < p.night_threshold;
            let is_night = self.day_light[0] < p.night_threshold;
            if was_night != is_night && !suppress_fade {
                events.emit(Event::Post(if was_night { 0xc9da96e8 } else { 0xc9da96e9 }));
            }
            let red = self.day_light[0];
            self.day_fraction = ((red - p.night_threshold) / (1.0 - p.night_threshold)).max(0.0);
            let mut denominator = p.night_threshold - self.minimum_red;
            if denominator == 0.0 {
                denominator = 0.0001;
            }
            self.night_fraction = (1.0 - (red - self.minimum_red) / denominator).max(0.0);
            events.emit(Event::Fractions {
                day: self.day_fraction,
                night: self.night_fraction,
            });
            self.announced_global = self.global_light;
            self.announced_day = self.day_light;
            self.night = is_night;
        }
        Ok(true)
    }
    pub fn set_lighting_tint(
        &mut self,
        color: Color,
        amount: f32,
        suppress_fade: bool,
        events: &mut dyn EventSink,
    ) -> Result<()> {
        self.tint = Some((color, amount));
        if self.set_time_of_day(self.time, self.month + 1, suppress_fade, events)? {
            events.emit(Event::Redraw);
        }
        Ok(())
    }
    pub fn reset_lighting_tint(&mut self, suppress_fade: bool, events: &mut dyn EventSink) -> Result<()> {
        self.tint = None;
        if self.set_time_of_day(self.time, self.month + 1, suppress_fade, events)? {
            events.emit(Event::Redraw);
        }
        Ok(())
    }
}

impl Drop for LightingManager {
    fn drop(&mut self) {
        self.destroy();
    }
}

fn scalar_key(keys: &[f32], f: f32) -> Result<f32> {
    let (i, t) = key_position(keys.len(), f)?;
    Ok(if t <= 0.0 {
        keys[i]
    } else {
        (1.0 - t) * keys[i] + t * keys[i + 1]
    })
}
fn color_key(keys: &[Color], f: f32) -> Result<Color> {
    let (i, t) = key_position(keys.len(), f)?;
    Ok(if t <= 0.0 {
        keys[i]
    } else {
        lerp(keys[i], keys[i + 1], t)
    })
}
fn key_position(n: usize, f: f32) -> Result<(usize, f32)> {
    if n == 0 || !f.is_finite() || !(0.0..=1.0).contains(&f) {
        return Err(LightingError::InvalidTime);
    }
    if n == 1 {
        return Ok((0, 0.0));
    }
    let p = (n - 1) as f32 * f;
    Ok((p.floor() as usize, p - p.floor()))
}
