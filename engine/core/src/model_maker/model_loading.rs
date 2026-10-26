// SPDX-License-Identifier: GPL-3.0-or-later
//! Model construction, model-maker §12. Foreign model/resource/utility objects
//! remain ports; construction decisions, transform math and cleanup live here.
use super::transform::{find_model_transform, Transform, TransformHost, TransformInput, Vector};
use super::{Handle, ModelKey, ModelMaker, ObjectHost, Rect, TerrainHost};

mod shadows;
pub use shadows::{ShadowHost, ShadowMesh};

pub const BUILDING: u32 = 0x2781_28A0;
pub const FLORA: u32 = 0x7475_8926;
pub const PROP: u32 = 0xA823_821E;
pub const FOUNDATION: u32 = 0x68FD_0C69;
pub const POWER_POLE: u32 = 0x2890_D4DE;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: Vector,
    pub max: Vector,
}

impl Bounds {
    pub fn transformed(self, transform: &Transform) -> Self {
        let mut out = Self {
            min: [f32::INFINITY; 3],
            max: [f32::NEG_INFINITY; 3],
        };
        for corner in 0..8 {
            let point = transform.point(std::array::from_fn(|i| {
                if corner & (1 << i) == 0 {
                    self.min[i]
                } else {
                    self.max[i]
                }
            }));
            for (i, value) in point.into_iter().enumerate() {
                out.min[i] = out.min[i].min(value);
                out.max[i] = out.max[i].max(value);
            }
        }
        out
    }
}

#[derive(Clone, Debug)]
pub struct ModelIds {
    pub type_id: u32,
    pub group: u32,
    pub entries: Vec<(u32, Vector)>,
}

#[derive(Clone, Copy, Debug)]
pub struct Placement {
    /// §12.11's rot argument is unused.
    pub rotation: i32,
    pub orientation: i32,
    pub flip: bool,
    pub position: Vector,
    pub offset: Option<Vector>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstanceSource {
    PowerPole,
    NetworkInterface,
    Underground,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadError {
    /// The native caller dereferences a missing holder. The portable port instead
    /// reports this violated precondition, after releasing its temporaries.
    MissingHolder,
}

/// All getters borrow, except query_interface/load_s3d/resource_instance/
/// animated_model/create_foundation/fill_instances, which return owned references.
/// An animated_instance is borrowed from its animation. holder_add takes its own
/// reference. A returned AddModel instance is borrowed from the holder after add.
/// Property access and model-ID/orientation/style/class utilities are foreign
/// services (graphics-formats §4.2), not delegated model-maker algorithms.
pub trait ModelHost: ObjectHost + TransformHost {
    fn exemplar(&mut self, occupant: Handle) -> Handle;
    fn type_id(&mut self, occupant: Handle) -> u32;
    fn position(&mut self, occupant: Handle) -> Vector;
    fn cells(&mut self, occupant: Handle) -> Rect;
    fn occupant_bounds(&mut self, occupant: Handle) -> Bounds;
    fn query_interface(&mut self, object: Handle, iid: u32) -> Option<Handle>;
    fn interface_word(&mut self, interface: Handle, slot: u32) -> u32;
    fn property_bool(&mut self, exemplar: Handle, id: u32) -> bool;
    fn property_present(&mut self, exemplar: Handle, id: u32) -> bool;
    fn property_byte(&mut self, exemplar: Handle, id: u32) -> u8;
    fn property_float(&mut self, exemplar: Handle, id: u32, default: f32) -> f32;
    fn property_five(&mut self, exemplar: Handle, id: u32) -> Option<[u32; 5]>;
    fn observe_property(&mut self, exemplar: Handle, id: u32);
    fn orientation_and_state(&mut self, occupant: Handle) -> (i32, bool);
    fn model_class(&mut self, occupant: Handle) -> u32;
    fn uses_building_style(&mut self, occupant: Handle) -> bool;
    fn foundation_uses_building_style(&mut self, occupant: Handle) -> bool;
    fn model_ids(&mut self, occupant: Handle, zoom: i32, rotation: i32) -> ModelIds;
    fn property1_instance(&mut self, base: u32, zoom: i32, rotation: i32) -> u32;
    fn holder(&mut self, occupant: Handle) -> Option<Handle>;
    fn holder_add(&mut self, holder: Handle, instance: Handle);
    fn holder_clear(&mut self, holder: Handle);
    fn hiding_filter_accepts(&mut self, type_id: u32) -> bool;
    fn reset_base_texture(&mut self, interface: Handle, argument: u32);
    fn world_to_cell(&mut self, x: f32, z: f32) -> [i32; 2];
    fn lot_at_cell(&mut self, cell: [i32; 2]) -> Option<Handle>;
    fn lot_occupant(&mut self, lot: Handle) -> Option<Handle>;
    fn fill_instances(
        &mut self,
        source: Handle,
        kind: InstanceSource,
        zoom: i32,
        rotation: i32,
        limit: usize,
    ) -> Vec<Handle>;
    fn load_s3d(&mut self, key: ModelKey) -> Option<Handle>;
    fn resource_bounds(&mut self, resource: Handle) -> Bounds;
    fn resource_instance(&mut self, resource: Handle) -> Option<Handle>;
    fn camera_scale(&mut self) -> f32;
    fn model_threshold(&mut self) -> f32;
    fn animated_model(&mut self, key: ModelKey) -> Option<Handle>;
    fn animated_instance(&mut self, animation: Handle) -> Option<Handle>;
    fn animated_view(&mut self, animation: Handle, zoom: i32, rotation: i32);
    fn animated_speed(&mut self, animation: Handle, speed: f32);
    fn animated_byte(&mut self, animation: Handle, value: u8);
    fn set_transform(&mut self, instance: Handle, transform: Transform);
    fn occupant_flag(&mut self, occupant: Handle, flag: u32) -> bool;
    fn instance_flags(&mut self, instance: Handle) -> u32;
    fn set_instance_flags(&mut self, instance: Handle, flags: u32);
    fn instance_byte(&mut self, instance: Handle) -> u8;
    fn set_instance_byte(&mut self, instance: Handle, value: u8);
    fn bind_occupant(&mut self, instance: Handle, occupant: Handle);
    /// Foreign building-development simulator query. The full key is preserved
    /// for the service's two-word key projection; no native ABI mapping here.
    fn development_check(&mut self, category: u32, key: ModelKey) -> i32;
    fn development_accepts(&mut self, occupant: Handle) -> bool;
    fn development_notify(&mut self, building: Handle, argument: u32);
    fn create_foundation(&mut self, argument: u32) -> Handle;
    /// A None array leaves that side's constructed defaults unchanged (§12.5).
    fn foundation_arrays(&mut self, model: Handle, p1: Option<[u32; 5]>, p2: Option<[u32; 5]>);
    fn foundation_dimensions(&mut self, model: Handle, a: f32, b: f32);
    fn foundation_bounds(&mut self, model: Handle, bounds: Bounds);
    fn foundation_adjust_height(&mut self, model: Handle, enabled: bool);
}

impl ModelMaker {
    pub fn remove_models<H: ModelHost + TerrainHost>(&self, occupant: Handle, type_id: u32, host: &mut H) {
        if !self.bulk_reload && host.hiding_filter_accepts(type_id) {
            self.hide_occupant_terrain(occupant, type_id, false, host);
        }
        if let Some(holder) = ModelHost::holder(host, occupant) {
            host.retain(holder);
            host.holder_clear(holder);
            host.release(holder);
        }
    }

    pub fn reset_lot_base_texture(&self, occupant: Handle, host: &mut impl ModelHost) {
        if let Some(interface) = host.query_interface(occupant, 0x097F_6D98) {
            host.reset_base_texture(interface, 0);
            host.release(interface);
        }
    }

    /// §12.5's constructor overwrites out without releasing an earlier value.
    pub fn construct_interface(&self, out: &mut Option<Handle>, source: Handle, iid: u32, host: &mut impl ModelHost) {
        *out = host.query_interface(source, iid);
    }

    pub fn lot_at_occupant(&self, occupant: Handle, host: &mut impl ModelHost) -> Option<Handle> {
        let p = host.position(occupant);
        let [x, z] = host.world_to_cell(p[0], p[2]);
        let r = self.saved.city;
        if x < r.x0 || x > r.x1 || z < r.z0 || z > r.z1 {
            return None;
        }
        host.lot_at_cell([x, z]).and_then(|lot| host.lot_occupant(lot))
    }

    pub fn load_network_models(
        &self,
        occupant: Handle,
        zoom: i32,
        rotation: i32,
        host: &mut impl ModelHost,
    ) -> Result<bool, LoadError> {
        let instances = if host.type_id(occupant) == POWER_POLE {
            host.fill_instances(occupant, InstanceSource::PowerPole, zoom, rotation, 10)
        } else if let Some(interface) = host.query_interface(occupant, 0xA821_EF94) {
            let instances = host.fill_instances(interface, InstanceSource::NetworkInterface, zoom, rotation, 10);
            host.release(interface);
            instances
        } else {
            Vec::new()
        };
        self.keep_filled_instances(occupant, instances, host)
    }

    pub fn load_underground_models(
        &self,
        occupant: Handle,
        zoom: i32,
        rotation: i32,
        host: &mut impl ModelHost,
    ) -> Result<bool, LoadError> {
        let instances = if matches!(host.type_id(occupant), 0x2A18_3167 | 0x8A13_918A) {
            host.fill_instances(occupant, InstanceSource::Underground, zoom, rotation, 10)
        } else {
            Vec::new()
        };
        self.keep_filled_instances(occupant, instances, host)
    }

    fn keep_filled_instances(
        &self,
        occupant: Handle,
        instances: Vec<Handle>,
        host: &mut impl ModelHost,
    ) -> Result<bool, LoadError> {
        let holder = host.holder(occupant);
        if let Some(h) = holder {
            host.retain(h);
        }
        let made = !instances.is_empty();
        for instance in instances {
            if let Some(h) = holder {
                host.holder_add(h, instance);
            }
            host.release(instance);
        }
        if let Some(h) = holder {
            host.release(h);
        }
        if made && holder.is_none() {
            Err(LoadError::MissingHolder)
        } else {
            Ok(made)
        }
    }

    pub fn create_placeholder(
        &self,
        occupant: Handle,
        zoom: i32,
        view_rotation: i32,
        orientation: i32,
        flip: bool,
        host: &mut impl ModelHost,
    ) -> Option<Handle> {
        let cells = host.cells(occupant);
        let size = cells.x1.wrapping_sub(cells.x0).max(cells.z1.wrapping_sub(cells.z0));
        let base = match size {
            i32::MIN..=1 => 0x2A0E_0000,
            2 => 0x2A0C_0000,
            3 => 0x2A0B_0000,
            _ => 0x2A0A_0000,
        };
        let rotation = view_rotation
            .wrapping_mul(3)
            .wrapping_add(if flip { 0 } else { orientation })
            % 4;
        let id = host.property1_instance(base, zoom, rotation);
        let resource = host.load_s3d(ModelKey::s3d(id))?;
        let instance = host.resource_instance(resource);
        host.release(resource);
        instance
    }

    pub fn add_model(
        &self,
        occupant: Handle,
        key: &mut ModelKey,
        placement: Placement,
        host: &mut impl ModelHost,
    ) -> Result<Option<Handle>, LoadError> {
        let ex = host.exemplar(occupant);
        let mut flip = placement.flip;
        let mut instance = None;
        let mut resource = None;
        let class = host.model_class(occupant);
        if matches!(class, 0xEA12_3BE1 | 0xAA12_3BF9) {
            if class == 0xAA12_3BF9 {
                flip = true;
            }
            if let Some(animation) = host.animated_model(*key) {
                instance = host.animated_instance(animation);
                if let Some(model) = instance {
                    host.retain(model);
                    host.animated_view(animation, self.saved.target_view[0], self.saved.target_view[1]);
                    host.animated_speed(animation, 0.01);
                    if host.property_present(ex, 0x6A9E_DD07) {
                        let byte = host.property_byte(ex, 0x6A9E_DD07);
                        host.animated_byte(animation, byte);
                    }
                }
                host.release(animation);
            }
            if instance.is_none() {
                *key = ModelKey::s3d(key.instance);
            }
        }
        if instance.is_none() {
            resource = host.load_s3d(*key);
            if resource.is_none() && *key != ModelKey::s3d(key.instance) {
                *key = ModelKey::s3d(key.instance);
                resource = host.load_s3d(*key);
            }
            if let Some(res) = resource {
                let bounds = host.resource_bounds(res);
                let extent: Vector = std::array::from_fn(|i| bounds.max[i] - bounds.min[i]);
                let length = (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2]).sqrt();
                if host.camera_scale() * length < host.model_threshold() {
                    host.release(res);
                    return Ok(None);
                }
                instance = host.resource_instance(res);
            }
        }
        if instance.is_none() && (!self.ignore_missing_model_data || host.property_present(ex, 0x6A87_1B82)) {
            instance = self.create_placeholder(
                occupant,
                self.saved.target_view[0],
                self.saved.target_view[1],
                placement.orientation,
                flip,
                host,
            );
        }
        let Some(model) = instance else {
            if let Some(res) = resource {
                host.release(res);
            }
            return Ok(None);
        };
        let mut record = Transform::default();
        let tilt_to_terrain = host.property_bool(ex, 0x69F1_4D33);
        find_model_transform(
            TransformInput {
                flag: flip,
                orientation: placement.orientation,
                position: placement.position,
                offset: placement.offset,
                tilt_to_terrain,
            },
            &mut record,
            host,
        );
        host.set_transform(model, record);
        if host.occupant_flag(occupant, 0x10) {
            let flags = host.instance_flags(model) & 1;
            host.set_instance_flags(model, flags);
        }
        host.bind_occupant(model, occupant);
        let holder = host.holder(occupant);
        if let Some(holder) = holder {
            host.holder_add(holder, model);
        }
        host.release(model);
        if let Some(res) = resource {
            host.release(res);
        }
        if holder.is_none() {
            Err(LoadError::MissingHolder)
        } else {
            Ok(Some(model))
        }
    }

    pub fn load_models_common(
        &self,
        occupant: Handle,
        zoom: i32,
        rotation: i32,
        type_id: u32,
        host: &mut impl ModelHost,
    ) -> Result<bool, LoadError> {
        let ex = host.exemplar(occupant);
        let mut position = host.position(occupant);
        if type_id == FOUNDATION {
            position[1] += 0.2;
        }
        let (orientation, flip) = host.orientation_and_state(occupant);
        let ids = host.model_ids(occupant, zoom, rotation);
        let mut result = false;
        for (index, (id, offset)) in ids.entries.into_iter().enumerate() {
            if id == 0 {
                continue;
            }
            let mut key = ModelKey {
                type_id: ids.type_id,
                group: ids.group,
                instance: id,
            };
            let placement = Placement {
                rotation,
                orientation,
                flip,
                position,
                offset: (offset != [0.0; 3]).then_some(offset),
            };
            let model = self.add_model(occupant, &mut key, placement, host)?;
            if index == 0 {
                result = model.is_some();
            }
            if let Some(model) = model {
                let flag_mask = match type_id {
                    BUILDING if index == 0 => {
                        if let Some(building) = host.query_interface(occupant, 0x87DD_A3A9) {
                            if host.development_check(2, key) == 0
                                && host.development_check(1, key) == 0
                                && host.development_accepts(occupant)
                            {
                                host.development_notify(building, 0);
                            }
                            host.release(building);
                        }
                        0x40
                    }
                    FLORA => 0x80,
                    PROP => 0x20,
                    _ => 0,
                };
                if flag_mask != 0 {
                    let mut flags = host.instance_flags(model) | flag_mask;
                    if type_id == PROP && host.property_present(ex, 0x4A70_D491) {
                        flags = (flags & !1) | 2;
                    }
                    host.set_instance_flags(model, flags);
                }
                if host.property_bool(ex, 0xE9F0_FA86) {
                    let byte = host.instance_byte(model) | 4;
                    host.set_instance_byte(model, byte);
                }
            }
        }
        Ok(result)
    }

    pub fn load_foundation_models(
        &self,
        occupant: Handle,
        zoom: i32,
        rotation: i32,
        type_id: u32,
        host: &mut impl ModelHost,
    ) -> Result<bool, LoadError> {
        if host.foundation_uses_building_style(occupant) {
            return self.load_models_common(occupant, zoom, rotation, type_id, host);
        }
        let ex = host.exemplar(occupant);
        let a = host.property_float(ex, 0xC911_EDA0, 16.0);
        let b = host.property_float(ex, 0xC911_EDA1, 16.0);
        let p1 = host.property_five(ex, 0x68FC_FF37);
        let p2 = host.property_five(ex, 0x68FC_FF38);
        host.observe_property(ex, 0x88FC_C3E3);
        let model = host.create_foundation(1);
        host.foundation_arrays(model, p1, p2);
        host.foundation_dimensions(model, a, b);
        let mut bounds = host.occupant_bounds(occupant);
        bounds.max[1] += 0.2;
        host.foundation_bounds(model, bounds);
        host.foundation_adjust_height(model, true);
        host.bind_occupant(model, occupant);
        let holder = host.holder(occupant);
        if let Some(h) = holder {
            host.holder_add(h, model);
        }
        host.release(model);
        if holder.is_none() {
            Err(LoadError::MissingHolder)
        } else {
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests;
