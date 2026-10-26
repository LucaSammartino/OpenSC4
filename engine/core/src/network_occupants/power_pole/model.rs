//! Pole model lifetime and render policy (§§4.2, 4.7.3–4.7.5).
use super::*;
use crate::network_occupants::{surface::normalize, xform::Xform};

/// External exemplar, resource and model-instance interfaces. Returned resource
/// and instance tokens each carry one created/acquired reference. No native
/// LoadModel, ChangeZoomLevel or Draw result is used as an input.
pub trait PoleModelHost: References {
    fn has_exemplar(&mut self, base: &OccupantBase) -> bool;
    fn model_key(&mut self, base: &OccupantBase) -> Option<ResKey>;
    fn model_instance_id(&mut self, zoom: i32, rotation: i32, alternate: bool) -> u32;
    fn load_resource(&mut self, key: ResKey, iid: u32) -> RefValue;
    fn create_instance(&mut self, resource: &RefValue, argument: u32) -> RefValue;
    /// §4.7.3 requires three readable offsets; there is no missing-data default.
    fn position_offset(&mut self, base: &OccupantBase) -> [f32; 3];
    fn get_transform(&mut self, instance: &RefValue) -> Xform;
    fn set_transform(&mut self, instance: &RefValue, transform: Xform);
    fn set_owner(&mut self, instance: &RefValue, occupant: &RefValue);
    fn view_yaw(&mut self) -> f32;
    fn diagonal_model(&mut self, base: &OccupantBase) -> bool;
    fn load_texture(&mut self, key: ResKey) -> RefValue;
}

const TEXTURE_IDS: [u32; 4] = [0xAA9D_A78E, 0xC9AF_0FCD, 0x0912_220E, 0x0808_0004];

pub struct PoleTextures {
    pub zoom: i32,
    pub bindings: [RefValue; 4],
}

impl PoleTextures {
    pub fn new(zoom: i32) -> Self {
        Self {
            zoom,
            bindings: std::array::from_fn(|_| RefValue::Null),
        }
    }

    pub fn reload(&mut self, zoom: i32, host: &mut impl PoleModelHost) {
        self.zoom = zoom;
        for (id, binding) in TEXTURE_IDS.into_iter().zip(&mut self.bindings) {
            clear_ref(binding, host);
            let key = ResKey {
                type_id: 0x7AB5_0E44,
                group: 0x1ABE_787D,
                instance: id,
            };
            *binding = host.load_texture(key);
            if matches!(binding, RefValue::Null) {
                *binding = host.load_texture(key);
            }
        }
    }

    fn binding(&self, id: u32) -> &RefValue {
        &self.bindings[TEXTURE_IDS.iter().position(|&v| v == id).expect("owned texture ID")]
    }
}

pub struct PoleDrawContext {
    pub zoom: i32,
    pub rotation: i32,
    pub highlight: bool,
}

/// Only foreign rendering/lighting calls. The owning pole chooses ordering,
/// primitive/format, texture IDs, normals and per-face lighting queries.
pub trait PoleRenderHost: PoleModelHost {
    fn delegate_draw(&mut self, instance: &RefValue, attributes: [u8; 2], opacity: f32);
    fn highlight_tint(&mut self, index: u8) -> [f32; 3];
    fn set_pole_state(&mut self);
    fn default_model_transform(&mut self);
    fn triangle_normal(&mut self, positions: [[f32; 3]; 3]) -> [f32; 3];
    fn light(&mut self, normal: [f32; 3], position: [f32; 3]) -> [f32; 3];
    fn set_shade(&mut self, rgb: [f32; 3]);
    fn reset_shade(&mut self);
    fn bind_texture(&mut self, id: u32, binding: &RefValue, stage: u32);
    fn filtering(&mut self, min: u32, mag: u32, stage: u32);
    fn wrap(&mut self, u: u32, v: u32, stage: u32);
    fn draw_face(&mut self, primitive: u32, format: u32, vertices: &[Vertex]);
    fn draw_wire(&mut self, primitive: u32, format: u32, vertices: &[[f32; 5]; 4]);
}

impl PowerPole {
    pub fn get_transform(&self, destination: &mut Xform) {
        let counter = destination.counter.wrapping_add(1);
        *destination = self.fields.transform;
        destination.counter = counter;
    }

    pub fn set_transform(&mut self, source: Xform) {
        let counter = self.fields.transform.counter.wrapping_add(1);
        self.fields.transform = source;
        self.fields.transform.counter = counter;
    }

    pub fn set_model_position(&self, host: &mut impl PoleModelHost) {
        if matches!(self.model, RefValue::Null) {
            return;
        }
        let mut transform = host.get_transform(&self.model);
        let offset = host.position_offset(&self.base);
        transform.translation =
            std::array::from_fn(|i| (f64::from(self.fields.position[i]) + f64::from(offset[i])) as f32);
        transform.mask |= 1;
        transform.counter = transform.counter.wrapping_add(1);
        host.set_transform(&self.model, transform);
    }

    /// Returns false on all paths, including successful creation (§4.7.3).
    pub fn load_model(&mut self, occupant: &RefValue, host: &mut impl PoleModelHost) -> bool {
        if !host.has_exemplar(&self.base) {
            return false;
        }
        let mut key = host.model_key(&self.base).unwrap_or(ResKey {
            type_id: 0x5AD0_E817,
            group: 0xBADB_57F1,
            instance: 0,
        });
        key.instance = host.model_instance_id(
            self.model_ids[1] as i32,
            self.model_ids[0] as i32,
            matches!(self.wire_exemplar_index, 1 | 8),
        );
        let resource = host.load_resource(key, POLE_MODEL_RESOURCE_TYPE);
        if matches!(resource, RefValue::Null) {
            return false;
        }
        clear_ref(&mut self.model, host);
        self.model = host.create_instance(&resource, 0);
        if !matches!(self.model, RefValue::Null) {
            self.set_model_position(host);
            host.set_owner(&self.model, occupant);
            let mut transform = host.get_transform(&self.model);
            let yaw = -f64::from(host.view_yaw());
            let (s, c) = yaw.sin_cos();
            transform.matrix = [c as f32, 0., -s as f32, 0., 1., 0., s as f32, 0., c as f32];
            transform.mask |= 4;
            transform.counter = transform.counter.wrapping_add(1);
            host.set_transform(&self.model, transform);
        }
        host.release(&resource);
        false
    }

    pub fn change_zoom_level(
        &mut self,
        context: &PoleDrawContext,
        occupant: &RefValue,
        textures: &mut PoleTextures,
        host: &mut impl PoleModelHost,
    ) -> Result<(), PoleUnsupported> {
        self.model_ids = [context.rotation as u32, context.zoom as u32];
        self.load_model(occupant, host);
        let diagonal = host.diagonal_model(&self.base);
        self.rebuild_geometry(diagonal, context.rotation as usize)?;
        if textures.zoom != context.zoom {
            textures.reload(context.zoom, host);
        }
        Ok(())
    }

    pub fn draw(
        &mut self,
        context: &PoleDrawContext,
        occupant: &RefValue,
        widths: &[f32; 5],
        textures: &mut PoleTextures,
        host: &mut impl PoleRenderHost,
    ) -> Result<bool, PoleUnsupported> {
        if matches!(self.model, RefValue::Null) {
            self.load_model(occupant, host);
            let diagonal = host.diagonal_model(&self.base);
            self.rebuild_geometry(diagonal, self.model_ids[0] as usize)?;
        }
        if textures.zoom != context.zoom || self.model_ids != [context.rotation as u32, context.zoom as u32] {
            self.change_zoom_level(context, occupant, textures, host)?;
        }
        if !matches!(self.model, RefValue::Null) {
            host.delegate_draw(&self.model, self.bytes_54_55, self.scale_64);
        }
        let tint = if context.highlight {
            host.highlight_tint(self.bytes_54_55[1])
        } else {
            [1.; 3]
        };
        host.set_pole_state();
        host.default_model_transform();
        for (index, face) in self.geometry.iter().enumerate() {
            // The native contract requires built lists, not an empty-list fallback.
            if face.len() < 3 {
                return Err(PoleUnsupported::ModelBoundsPickDrawNO33NO43);
            }
            let normal = normalize(host.triangle_normal([face[0].pos, face[1].pos, face[2].pos]));
            let rgb = host.light(normal, face[0].pos);
            host.set_shade(std::array::from_fn(|i| rgb[i] * tint[i]));
            if index == 0 {
                host.bind_texture(0x0912_220E, textures.binding(0x0912_220E), 0);
                host.filtering(1, 1, 0);
                host.wrap(2, 2, 0);
            } else if index == 1 {
                host.bind_texture(0x0808_0004, textures.binding(0x0808_0004), 0);
                host.filtering(1, 1, 0);
                host.wrap(3, 3, 0);
            }
            host.draw_face(6, 10, face);
            if index == 0 || index == 2 {
                host.reset_shade();
            }
        }
        // The published caller supplies a valid cached zoom, without clamping.
        for command in self.wire_submissions(widths[context.zoom as usize]) {
            match command {
                WireSubmission::Bind(id) => host.bind_texture(id, textures.binding(id), 0),
                WireSubmission::Quad(quad) => host.draw_wire(6, 2, &quad.vertices),
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
