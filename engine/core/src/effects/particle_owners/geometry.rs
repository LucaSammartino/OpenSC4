// SPDX-License-Identifier: GPL-3.0-or-later
//! particles.md §14: retained geometry resources and inherited dispatch.
use super::Unsupported;

pub type Reference = u64;
pub trait Resources {
    fn retain(&mut self, reference: Reference);
    fn release(&mut self, reference: Reference);
    fn group_count(&mut self, source: Reference) -> usize;
    fn group(&mut self, source: Reference, index: usize) -> Reference;
    fn material(&mut self, source: Reference, group: Reference) -> (bool, Option<Reference>);
    /// Returns a temporary owned reference, released after SetGeometrySource.
    fn lookup(&mut self, key: [u32; 3], interface: u32) -> Option<Reference>;
    fn model_interface(&mut self, model: Reference, interface: u32) -> Option<Reference>;
    fn ordinary_shutdown(&mut self);
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeometryParticles {
    pub description: Option<Reference>,
    pub initialized: bool,
    pub source: Option<Reference>,
    pub textures: Vec<Reference>,
    pub built: bool,
    pub enabled: bool,
    pub center: Option<[f32; 3]>,
    pub threshold: f32,
    pub parameter_101: f32,
}
impl Default for GeometryParticles {
    fn default() -> Self {
        Self {
            description: None,
            initialized: false,
            source: None,
            textures: Vec::new(),
            built: false,
            enabled: true,
            center: None,
            threshold: 10.,
            parameter_101: 0.,
        }
    }
}
impl GeometryParticles {
    /// §21.2 parameter 9: two vectors and a source, never three vertices.
    pub fn project_source(
        &mut self,
        vectors: [[f32; 3]; 2],
        resources: &mut impl Resources,
    ) -> Result<bool, Unsupported> {
        let Some(source) = self.source else {
            return Ok(true);
        };
        if resources.group_count(source) == 0 {
            return Ok(true);
        }
        let _ = vectors;
        // SPEC-GAP: nonempty projection/dicing requires §21.2's literal mesh capture.
        Err(Unsupported::PT8)
    }
    pub fn set_source(&mut self, source: Option<Reference>, resources: &mut impl Resources) {
        if self.source == source {
            return;
        }
        if let Some(new) = source {
            resources.retain(new);
        }
        let old = std::mem::replace(&mut self.source, source);
        if let Some(old) = old {
            resources.release(old);
        }
        for old in self.textures.drain(..) {
            resources.release(old);
        }
        if let Some(source) = source {
            for index in 0..resources.group_count(source) {
                let group = resources.group(source, index);
                let (enabled, binding) = resources.material(source, group);
                if enabled {
                    if let Some(binding) = binding {
                        resources.retain(binding);
                        self.textures.push(binding);
                    }
                }
            }
        }
    }
    pub fn set_resource(&mut self, id: u32, resources: &mut impl Resources) {
        if id == 0 {
            return;
        }
        if let Some(source) = resources.lookup([0x5ad0_e817, 0xbadb_57f1, id], 0x9ad0_e7e3) {
            self.set_source(Some(source), resources);
            resources.release(source);
        }
    }
    pub fn set_model(
        &mut self,
        model: Reference,
        resources: &mut impl Resources,
    ) -> Result<(), Unsupported> {
        let source = resources.model_interface(model, 0x26a6_8448);
        self.set_source(source, resources);
        if let Some(source) = source {
            resources.release(source);
        }
        // Bounds-to-center arithmetic is explicitly PT8.
        Err(Unsupported::PT8)
    }
    /// None means delegate the parameter at its original slot to the ordinary
    /// handler, rather than invent an unknown-ID result for the derived class.
    pub fn set_float(&mut self, id: u32, values: &[f32]) -> Option<Result<bool, Unsupported>> {
        match id {
            8 => {
                self.center = Some([values[0], values[1], values[2]]);
                Some(Ok(true))
            }
            9 => {
                // Valid six-value input is a precondition, not advertised count.
                let _vectors = &values[..6];
                Some(if self.source.is_none() {
                    Ok(true)
                } else {
                    Err(Unsupported::PT8)
                })
            }
            0x101 => {
                self.parameter_101 = values[0];
                Some(Ok(true))
            }
            _ => None,
        }
    }
    pub fn set_description(
        &mut self,
        description: Reference,
        flags_b: u32,
        resource: u32,
        resources: &mut impl Resources,
    ) -> Result<bool, Unsupported> {
        if flags_b & 1 == 0 {
            return Ok(false);
        }
        if self.description != Some(description) {
            resources.retain(description);
            if let Some(old) = self.description.replace(description) {
                resources.release(old);
            }
        }
        self.built = false;
        if resource != 0 && resource != u32::MAX {
            self.set_resource(resource, resources);
        }
        // Draw mode/cache setup and threshold/dicing remain PT8, not an
        // ordinary source-box replacement.
        Err(Unsupported::PT8)
    }
    pub fn emit(
        &mut self,
        delayed_stop: bool,
        countdown: &mut f32,
        dt: f32,
        live_count: usize,
    ) -> Result<bool, Unsupported> {
        if self.source.is_none() || delayed_stop {
            return Ok(false);
        }
        if *countdown > 0. {
            *countdown -= dt;
            if *countdown > 0. {
                return Ok(false);
            }
        }
        if !self.built {
            return Err(Unsupported::PT8);
        }
        // true requests component Stop(0), the delayed simulator stop.
        Ok(live_count == 0)
    }
    pub fn physics_is_ordinary(&self, wave_field: f32) -> Result<bool, Unsupported> {
        if wave_field == 0. {
            Ok(true)
        } else {
            Err(Unsupported::PT8)
        }
    }
    pub fn shutdown(&mut self, resources: &mut impl Resources) {
        if !self.initialized {
            return;
        }
        if let Some(old) = self.source.take() {
            resources.release(old);
        }
        resources.ordinary_shutdown();
        self.initialized = false;
    }
    pub fn teardown(&mut self, resources: &mut impl Resources) {
        self.shutdown(resources);
        // Source assignment may precede Init. Final teardown still owns that
        // retained reference even though uninitialized Shutdown does no work.
        if let Some(source) = self.source.take() {
            resources.release(source);
        }
        // Texture bindings are independent references, not source aliases.
        for old in self.textures.drain(..) {
            resources.release(old);
        }
        if let Some(description) = self.description.take() {
            resources.release(description);
        }
    }
}

/// Only the qualified wave prefix; displacement/damping remain PT8.
pub fn wave_prefix(
    cache: &mut [f32; 2],
    dt: f32,
    field: f32,
    curve: &[f32],
    clock: f64,
) -> Option<f32> {
    if field == 0. {
        return None;
    }
    if cache[0] == cache[1] {
        cache[0] = (f64::from(cache[0]) + f64::from(dt) * f64::from(field)) as f32;
    }
    Some(super::decal::sample(curve, clock))
}

/// Numeric mesh generation remains PT8, but already-generated groups retain
/// their material order and bind only when nonempty.
pub trait PrimitiveRenderer {
    fn bind_texture(&mut self, texture: Reference);
    fn draw_pool(&mut self, group: usize);
}
impl GeometryParticles {
    pub fn draw_groups(&self, primitive_counts: &[usize], renderer: &mut impl PrimitiveRenderer) {
        assert_eq!(primitive_counts.len(), self.textures.len());
        for (index, (&texture, &count)) in self.textures.iter().zip(primitive_counts).enumerate() {
            if count != 0 {
                renderer.bind_texture(texture);
                renderer.draw_pool(index);
            }
        }
    }
}
