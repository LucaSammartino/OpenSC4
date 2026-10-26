// SPDX-License-Identifier: GPL-3.0-or-later
//! Counted texture assignment and material walks, model-maker §§12.3–12.4.
use super::{Handle, ModelKey, ModelMaker, ObjectHost};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Material {
    pub state: u8,
    pub name_id: u32,
    pub swapped_instance: u32,
    pub binding: Option<Handle>,
    pub sampler: [u8; 4],
    pub byte_2c: u8,
    pub byte_2d: u8,
    pub byte_2e: u8,
    pub word_30: u32,
}

/// Holder/instance/material getters borrow. A material list, resource service,
/// or successfully loaded texture carries one owned reference. `write_material`
/// stores the snapshot without further reference-count changes.
pub trait MaterialHost: ObjectHost {
    fn holder(&mut self, occupant: Handle) -> Option<Handle>;
    fn occupant_type(&mut self, occupant: Handle) -> u32;
    fn instances(&mut self, holder: Handle) -> Vec<Handle>;
    fn renderer_contains(&mut self, instance: Handle) -> bool;
    fn renderer_change(&mut self, instance: Handle, mode: u32);
    fn material_list(&mut self, instance: Handle) -> Option<Handle>;
    fn materials(&mut self, list: Handle) -> Vec<Handle>;
    fn material(&mut self, material: Handle) -> Material;
    fn write_material(&mut self, material: Handle, value: Material);
    fn frame_counter(&mut self) -> u32;
    fn resource_service(&mut self) -> Option<Handle>;
    fn texture_name(&mut self, service: Handle, name: u32) -> (u32, u32);
    fn load_texture(&mut self, service: Handle, key: ModelKey) -> Option<Handle>;
}

/// §12.3: acquire the replacement before releasing the old value; equality is a no-op.
pub fn assign_counted(slot: &mut Option<Handle>, new: Option<Handle>, host: &mut impl ObjectHost) {
    if *slot != new {
        if let Some(object) = new {
            host.retain(object);
        }
        if let Some(object) = *slot {
            host.release(object);
        }
        *slot = new;
    }
}

fn walk_materials<H: MaterialHost>(holder: Handle, host: &mut H, mut visit: impl FnMut(&mut H, Handle)) {
    for instance in host.instances(holder) {
        if let Some(list) = host.material_list(instance) {
            for material in host.materials(list) {
                visit(host, material);
            }
            host.release(list);
        }
    }
}

fn sampler(repeat: bool) -> [u8; 4] {
    if repeat {
        [2, 2, 0, 0]
    } else {
        [3, 3, 1, 1]
    }
}

impl ModelMaker {
    pub fn change_models(&self, occupant: Handle, flag: u32, mode: u32, host: &mut impl MaterialHost) -> bool {
        let mode = if mode == 0 { 3 } else { mode };
        if flag == 0 {
            host.occupant_type(occupant);
        }
        let Some(holder) = host.holder(occupant) else {
            return false;
        };
        host.retain(holder);
        for instance in host.instances(holder) {
            if host.renderer_contains(instance) {
                host.renderer_change(instance, mode);
            }
        }
        host.release(holder);
        true
    }

    pub fn restore_static_textures(&self, holder: Handle, host: &mut impl MaterialHost) -> bool {
        let mut changed = false;
        walk_materials(holder, host, |host, id| {
            let mut material = host.material(id);
            if material.state > 1 {
                material.state = 1;
                assign_counted(&mut material.binding, None, host);
                host.write_material(id, material);
                changed = true;
            }
        });
        changed
    }

    pub fn apply_texture_to_models(
        &self,
        holder: Handle,
        texture: Option<Handle>,
        a: u8,
        b: bool,
        host: &mut impl MaterialHost,
    ) {
        let frame = host.frame_counter();
        walk_materials(holder, host, |host, id| {
            let mut material = host.material(id);
            material.state = 2;
            assign_counted(&mut material.binding, texture, host);
            material.byte_2c = a;
            material.byte_2e = 1;
            material.word_30 = frame;
            material.sampler = sampler(b);
            host.write_material(id, material);
        });
    }

    pub fn swap_textures_by_offset(
        &self,
        holder: Handle,
        delta: u32,
        a: u8,
        b: bool,
        host: &mut impl MaterialHost,
    ) -> bool {
        let Some(service) = host.resource_service() else {
            return false;
        };
        let mut changed = false;
        walk_materials(holder, host, |host, id| {
            let mut material = host.material(id);
            if material.state == 0 || material.name_id == 0 {
                return;
            }
            material.state = 1;
            assign_counted(&mut material.binding, None, host);
            let (group, instance) = host.texture_name(service, material.name_id);
            if instance != 0 {
                let instance = instance.wrapping_add(delta);
                let key = ModelKey {
                    type_id: 0x7AB5_0E44,
                    group,
                    instance,
                };
                if let Some(texture) = host.load_texture(service, key) {
                    material.swapped_instance = instance;
                    material.state = 2;
                    assign_counted(&mut material.binding, Some(texture), host);
                    material.byte_2c = a;
                    material.byte_2d = 0;
                    material.byte_2e = 0;
                    material.word_30 = 1.0f32.to_bits();
                    material.sampler = sampler(b);
                    host.release(texture);
                    changed = true;
                }
            }
            host.write_material(id, material);
        });
        host.release(service);
        changed
    }
}

#[cfg(test)]
mod tests;
