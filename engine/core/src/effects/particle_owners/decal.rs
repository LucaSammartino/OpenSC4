// SPDX-License-Identifier: GPL-3.0-or-later
//! particles.md §15. Integer renderer handles and settled timing/parameters.
use super::ordinary::Transform;
use super::Unsupported;
use crate::rng::RzRandom;

pub const INVALID_HANDLE: u32 = u32::MAX;

#[derive(Clone, Debug, PartialEq)]
pub struct Description {
    pub flags: u32,
    pub resource: u32,
    pub layer: u8,
    pub lifetime_mode: u8,
    pub duration: f32,
    pub rotation: Vec<f32>,
    pub size: Vec<f32>,
    pub opacity: Vec<f32>,
    pub colour: Vec<[f32; 3]>,
    pub aspect: Vec<f32>,
    /// Opacity, size, rotation variance at description +58/+5C/+60.
    pub variances: [f32; 3],
    pub mapping_scale: f32,
    pub uv: [f32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Binding {
    Resource(u32),
    Texture(u64),
    Alternate(u32),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Creation {
    pub binding: Binding,
    pub center: [f32; 2],
    pub size: f32,
    pub angle: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub center: [f32; 2],
    pub size: f32,
    pub angle: f32,
    pub projection: [u32; 7],
}
/// Only foreign service operations, never a supplied owner tick/phase answer.
pub trait Renderer {
    fn terrain_available(&mut self) -> bool;
    fn acquire_service(&mut self) -> bool;
    fn release_service(&mut self);
    fn create(&mut self, creation: Creation) -> u32;
    fn get_projection(&mut self, handle: u32) -> [u32; 7];
    fn set_projection(&mut self, handle: u32, projection: [u32; 7]);
    /// Abstract +34 placement update; native placement-record packing remains foreign.
    fn place(&mut self, handle: u32, placement: Placement);
    fn enable(&mut self, handle: u32, value: bool);
    fn city_size_z(&mut self) -> f64;
    fn city_size_x(&mut self) -> f64;
    fn remove(&mut self, handle: u32);
    fn opacity(&mut self, handle: u32, value: f32);
    fn colour(&mut self, handle: u32, value: [f32; 3]);
    fn layer(&mut self, handle: u32, value: u8);
    fn retain_texture(&mut self, texture: u64);
    fn release_texture(&mut self, texture: u64);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Decal {
    pub description: Description,
    pub started: bool,
    pub handle: u32,
    pub service_retained: bool,
    pub texture: Option<u64>,
    pub center: [f32; 2],
    pub size_scale: f32,
    pub angle: f32,
    pub mapping: [f32; 2],
    pub external_opacity: f32,
    pub external_size: f32,
    pub explicit: bool,
    /// Start stores q itself, in seconds, rather than q times duration.
    pub elapsed: Option<f32>,
    pub inverse_lifetime: f32,
    /// Multipliers from the separate decal RNG.
    pub variance: Option<[f32; 3]>,
}
impl Decal {
    pub fn new(description: Description) -> Self {
        let inverse_lifetime = 1. / description.duration.max(0.0001);
        Self {
            description,
            started: false,
            handle: INVALID_HANDLE,
            service_retained: false,
            texture: None,
            center: [0.; 2],
            size_scale: 1.,
            angle: 0.,
            mapping: [1.; 2],
            external_opacity: 1.,
            external_size: 1.,
            explicit: false,
            elapsed: None,
            inverse_lifetime,
            variance: None,
        }
    }
    pub fn set_description(
        &mut self,
        description: &Description,
        renderer: &mut impl Renderer,
        rng: &mut RzRandom,
    ) -> Result<(), Unsupported> {
        self.description.clone_from(description);
        self.inverse_lifetime = 1. / description.duration.max(0.0001);
        if let Some(old) = self.texture.take() {
            renderer.release_texture(old);
        }
        self.external_opacity = 1.;
        self.external_size = 1.;
        self.explicit = false;
        let [opacity, size, rotation] = description.variances.map(f64::from);
        let opacity = rng.range_f64(1. - opacity, 1. + opacity) as f32;
        let size = rng.range_f64(1. - size, 1. + size) as f32;
        let rotation = if rotation > 0. {
            rng.range_f64(-rotation, rotation) as f32
        } else {
            1.
        };
        self.variance = Some([opacity, size, rotation]);
        if description.flags & 0x10 != 0 {
            self.explicit = true;
            self.center[1] = (renderer.city_size_z() * 0.5) as f32;
            self.center[0] = (renderer.city_size_x() * 0.5) as f32;
            self.size_scale = renderer.city_size_x() as f32;
        }
        Ok(())
    }
    pub fn set_float(&mut self, id: u32, values: &[f32]) -> bool {
        match id {
            3 => self.external_opacity = values[0],
            0x102 => self.external_size = values[1],
            12 => {
                let [x0, z0, x1, z1] = [values[0], values[1], values[2], values[3]].map(f64::from);
                self.center = [((x0 + x1) * 8.) as f32, ((z0 + z1) * 8.) as f32];
                self.size_scale = ((x1 - x0) * 16.) as f32;
                self.angle = 0.;
                self.mapping[1] = ((z1 - z0) / (x1 - x0)) as f32;
                if self.description.flags & 8 != 0 {
                    self.mapping[0] = (x1 - x0) as f32;
                }
                self.explicit = true;
            }
            _ => return false,
        }
        true
    }
    pub fn set_pointer(&mut self, id: u32, texture: Option<u64>, renderer: &mut impl Renderer) -> bool {
        if id != 6 {
            return false;
        }
        if self.texture != texture {
            if let Some(new) = texture {
                renderer.retain_texture(new);
            }
            let old = std::mem::replace(&mut self.texture, texture);
            if let Some(old) = old {
                renderer.release_texture(old);
            }
        }
        true
    }
    pub fn stop(&mut self, _mode: u32, renderer: &mut impl Renderer) {
        if !self.started {
            return;
        }
        self.started = false;
        if self.handle != INVALID_HANDLE {
            renderer.remove(self.handle);
            self.handle = INVALID_HANDLE;
            self.service_retained = false;
            renderer.release_service();
        }
    }
    /// §15.4 finite nonempty-curve creation and ordered projection trace.
    pub fn start(&mut self, mode: u32, renderer: &mut impl Renderer) -> Result<(), Unsupported> {
        if self.started || (mode != 0 && self.description.lifetime_mode == 2) {
            return Ok(());
        }
        if !renderer.terrain_available() {
            return Ok(());
        }
        if self.handle != INVALID_HANDLE {
            return Err(Unsupported::PT9);
        }
        if !self.service_retained {
            self.service_retained = renderer.acquire_service();
            if !self.service_retained {
                return Ok(());
            }
        }
        let phase = if mode == 0 {
            0.
        } else {
            initial_phase(u32::from(self.description.lifetime_mode))
        };
        let factors = self.variance.ok_or(Unsupported::PT9)?;
        let binding = if self.description.flags & 0x20 != 0 {
            Binding::Alternate(self.description.resource)
        } else if let Some(texture) = self.texture {
            Binding::Texture(texture)
        } else {
            Binding::Resource(self.description.resource)
        };
        self.handle = renderer.create(Creation {
            binding,
            center: self.center,
            size: (f64::from(self.size_scale) * f64::from(sample(&self.description.size, phase))) as f32,
            angle: (f64::from(self.angle)
                + f64::from(factors[2]) * f64::from(sample(&self.description.rotation, phase)))
                as f32,
        });
        if self.handle == INVALID_HANDLE {
            return Ok(());
        }
        let projection = renderer.get_projection(self.handle);
        renderer.set_projection(self.handle, self.projection(projection, phase));
        if self.description.flags & 2 != 0 {
            renderer.enable(self.handle, true);
        }
        renderer.opacity(self.handle, self.sample_opacity(phase)?);
        renderer.colour(self.handle, sample_colour(&self.description.colour, phase));
        renderer.layer(self.handle, self.description.layer);
        self.elapsed = Some(phase as f32);
        self.started = true;
        Ok(())
    }
    pub fn set_transforms(
        &mut self,
        a: Transform,
        b: Transform,
        renderer: &mut impl Renderer,
    ) -> Result<(), Unsupported> {
        if self.explicit {
            return Ok(());
        }
        let factors = self.variance.ok_or(Unsupported::PT9)?;
        let mut t = if a.flags != 0 && b.flags == 0 { a } else { b };
        let mut r00 = f64::from(t.rotation[0][0]);
        if a.flags != 0 && b.flags != 0 {
            let dot = |row: [f32; 3], p: [f32; 3]| {
                let row = row.map(f64::from);
                let p = p.map(f64::from);
                (row[0] * p[0] + row[2] * p[2]) + row[1] * p[1]
            };
            for i in [0, 2] {
                let scaled = (f64::from(b.scale) * dot(b.rotation[i], a.translation)) as f32;
                t.translation[i] = (f64::from(scaled) + f64::from(b.translation[i])) as f32;
            }
            let column = [a.rotation[0][0], a.rotation[1][0], a.rotation[2][0]];
            r00 = dot(b.rotation[0], column);
            t.rotation[2][0] = dot(b.rotation[2], column) as f32;
            t.scale = (f64::from(b.scale) * f64::from(a.scale)) as f32;
        }
        self.center = [t.translation[0], t.translation[2]];
        self.size_scale = (f64::from(t.scale) * f64::from(factors[1])) as f32;
        self.angle = ((-f64::from(t.rotation[2][0])).atan2(r00) * f64::from(f32::from_bits(0x3e22f983))) as f32;
        if self.handle != INVALID_HANDLE && self.description.flags & 0x40 != 0 {
            let projection = renderer.get_projection(self.handle);
            renderer.place(self.handle, self.placement(projection, 0.)?);
        }
        Ok(())
    }
    fn projection(&self, mut record: [u32; 7], phase: f64) -> [u32; 7] {
        let aspect = (f64::from(sample(&self.description.aspect, phase)) * f64::from(self.mapping[1])) as f32;
        record[2] = aspect.to_bits();
        let mapping = f64::from(self.mapping[0]) * f64::from(self.description.mapping_scale);
        record[3] = if self.description.flags & 8 == 0 {
            1_f32.to_bits()
        } else {
            (mapping as f32).to_bits()
        };
        record[4] = if self.description.flags & 8 == 0 {
            1_f32.to_bits()
        } else {
            ((mapping * f64::from(aspect)) as f32).to_bits()
        };
        record[5] = self.description.uv[0].to_bits();
        record[6] = self.description.uv[1].to_bits();
        record
    }
    fn placement(&self, record: [u32; 7], phase: f64) -> Result<Placement, Unsupported> {
        let factors = self.variance.ok_or(Unsupported::PT9)?;
        // SPEC-GAP: native placement record offsets beyond §15.4's projection are not specified; expose logical fields.
        Ok(Placement {
            center: self.center,
            size: (f64::from(self.size_scale)
                * f64::from(sample(&self.description.size, phase))
                * f64::from(self.external_size)) as f32,
            angle: (f64::from(self.angle)
                + f64::from(factors[2]) * f64::from(sample(&self.description.rotation, phase)))
                as f32,
            projection: self.projection(record, phase),
        })
    }
    fn sample_opacity(&self, phase: f64) -> Result<f32, Unsupported> {
        let factors = self.variance.ok_or(Unsupported::PT9)?;
        Ok((f64::from(sample(&self.description.opacity, phase))
            * f64::from(factors[0])
            * f64::from(self.external_opacity)) as f32)
    }
    pub fn update(&mut self, dt: f32, renderer: &mut impl Renderer) -> Result<(), Unsupported> {
        if self.handle == INVALID_HANDLE {
            return Ok(());
        }
        let elapsed = self.elapsed.as_mut().ok_or(Unsupported::PT9)?;
        let sum = f64::from(*elapsed) + f64::from(dt);
        *elapsed = sum as f32;
        let mut phase = f64::from((sum * f64::from(self.inverse_lifetime)) as f32);
        if self.description.lifetime_mode == 1 {
            phase -= phase.floor();
        } else if phase > 1. {
            if self.description.lifetime_mode == 2 {
                self.stop(0, renderer);
                return Ok(());
            }
            phase = 1.;
        }
        if self.description.flags & 0x40 == 0 {
            renderer.place(self.handle, self.placement([0; 7], phase)?);
        }
        renderer.opacity(self.handle, self.sample_opacity(phase)?);
        if self.description.flags & 0x40 != 0 {
            return Ok(());
        }
        // Colour has a settled weighted interpolation, unlike projection.
        if self.description.colour.len() > 1 {
            renderer.colour(self.handle, sample_colour(&self.description.colour, phase));
        }
        Ok(())
    }
    pub fn teardown(&mut self, renderer: &mut impl Renderer) {
        self.stop(0, renderer);
        if self.handle != INVALID_HANDLE {
            renderer.remove(self.handle);
            self.handle = INVALID_HANDLE;
        }
        if let Some(texture) = self.texture.take() {
            renderer.release_texture(texture);
        }
        if self.service_retained {
            self.service_retained = false;
            renderer.release_service();
        }
    }
}

pub fn initial_phase(mode: u32) -> f64 {
    if mode == 3 {
        1.
    } else {
        0.
    }
}
pub fn sample(values: &[f32], phase: f64) -> f32 {
    assert!(!values.is_empty(), "nonempty spec curve precondition");
    if values.len() == 1 {
        return values[0];
    }
    let x = phase * (values.len() - 1) as f64;
    let i = x.floor() as usize;
    let fraction = x - i as f64;
    if fraction <= 0. {
        values[i]
    } else {
        (f64::from(values[i]) + fraction * (f64::from(values[i + 1]) - f64::from(values[i]))) as f32
    }
}
pub fn sample_colour(values: &[[f32; 3]], phase: f64) -> [f32; 3] {
    assert!(!values.is_empty(), "nonempty spec curve precondition");
    if values.len() == 1 {
        return values[0];
    }
    let x = phase * (values.len() - 1) as f64;
    let i = x.floor() as usize;
    let fraction = x - i as f64;
    if fraction <= 0. {
        values[i]
    } else {
        std::array::from_fn(|j| {
            ((1. - fraction) * f64::from(values[i][j]) + fraction * f64::from(values[i + 1][j])) as f32
        })
    }
}
