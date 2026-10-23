// SPDX-License-Identifier: GPL-3.0-or-later
//! particles.md §15. Integer renderer handles and settled timing/parameters.
use super::Unsupported;

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
}
/// Only foreign service operations, never a supplied owner tick/phase answer.
pub trait Renderer {
    fn terrain_available(&mut self) -> bool;
    fn acquire_service(&mut self) -> bool;
    fn release_service(&mut self);
    fn create(&mut self) -> u32;
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
    /// PT9: no guessed initial elapsed time from Start(mode).
    pub elapsed: Option<f32>,
    pub inverse_lifetime: f32,
    /// PT9: per-instance random factors remain unknown until specified.
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
    ) -> Result<(), Unsupported> {
        self.description.clone_from(description);
        self.inverse_lifetime = 1. / description.duration.max(0.0001);
        if let Some(old) = self.texture.take() {
            renderer.release_texture(old);
        }
        self.external_opacity = 1.;
        self.external_size = 1.;
        self.explicit = false;
        self.variance = None;
        // Does not Stop or recreate an existing renderer handle. Whole-city
        // placement and variance sampling are explicitly unresolved in PT9.
        Err(Unsupported::PT9)
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
    /// Settled early exits, acquisition and failure sentinel. Successful creation
    /// needs PT9 projection/flag records: the pending handle stays owned but
    /// started stays false, and Unsupported explicitly prevents claiming success.
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
        self.handle = renderer.create();
        if self.handle == INVALID_HANDLE {
            return Ok(());
        }
        Err(Unsupported::PT9)
    }
    /// Transform composition is open, but explicit rectangles suppress it.
    pub fn set_transforms(&mut self) -> Result<(), Unsupported> {
        if self.explicit && !(self.description.flags & 0x40 != 0 && self.handle != INVALID_HANDLE) {
            Ok(())
        } else {
            Err(Unsupported::PT9)
        }
    }
    pub fn update(&mut self, dt: f32, renderer: &mut impl Renderer) -> Result<(), Unsupported> {
        if self.handle == INVALID_HANDLE {
            return Ok(());
        }
        let elapsed = self.elapsed.as_mut().ok_or(Unsupported::PT9)?;
        *elapsed = (f64::from(*elapsed) + f64::from(dt)) as f32;
        let mut phase = f64::from(*elapsed) * f64::from(self.inverse_lifetime);
        if self.description.lifetime_mode == 1 {
            phase -= phase.floor();
        } else if phase > 1. {
            if self.description.lifetime_mode == 2 {
                self.stop(0, renderer);
                return Ok(());
            }
            phase = 1.;
        }
        let factors = self.variance.ok_or(Unsupported::PT9)?;
        let opacity = f64::from(sample(&self.description.opacity, phase))
            * f64::from(factors[0])
            * f64::from(self.external_opacity);
        renderer.opacity(self.handle, opacity as f32);
        if self.description.flags & 0x40 != 0 {
            return Ok(());
        }
        // Colour has a settled weighted interpolation, unlike projection.
        if self.description.colour.len() > 1 {
            renderer.colour(self.handle, sample_colour(&self.description.colour, phase));
        }
        Err(Unsupported::PT9)
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
