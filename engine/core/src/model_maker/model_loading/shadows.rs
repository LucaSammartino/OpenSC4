// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone, Debug)]
pub struct ShadowMesh {
    pub material_state: u8,
    /// Borrowed from the mesh material.
    pub binding: Option<Handle>,
    pub positions: Vec<Vector>,
    pub coordinates: Vec<[f32; 2]>,
}

/// resource_model returns one owned reference. Meshes and bindings borrow from
/// it. The overlay manager creates IDs, which are stored in the occupant holder.
pub trait ShadowHost: ModelHost {
    fn shadow_detail(&mut self) -> i32;
    fn shadow_threshold(&mut self) -> f32;
    fn shadow_model_id(
        &mut self,
        occupant: Handle,
        zoom: i32,
        view_rotation: i32,
        orientation: i32,
        flag: bool,
        key: &mut ModelKey,
    );
    fn resource_model(&mut self, resource: Handle) -> Option<Handle>;
    fn model_bounds(&mut self, model: Handle) -> Bounds;
    fn shadow_meshes(&mut self, model: Handle) -> Vec<ShadowMesh>;
    fn add_model_shadow(&mut self, mesh: &ShadowMesh, bounds: Bounds, transform: Transform) -> i32;
    fn holder_add_shadow(&mut self, holder: Handle, shadow: i32);
}

impl ModelMaker {
    pub fn load_models_with_shadow(
        &self,
        occupant: Handle,
        zoom: i32,
        rotation: i32,
        type_id: u32,
        host: &mut impl ShadowHost,
    ) -> Result<bool, LoadError> {
        let result = self.load_models_common(occupant, zoom, rotation, type_id, host)?;
        if result {
            self.create_occupant_shadow(occupant, zoom, rotation, host)?;
        }
        Ok(result)
    }

    pub fn load_prop_models(
        &self,
        occupant: Handle,
        zoom: i32,
        rotation: i32,
        type_id: u32,
        host: &mut impl ShadowHost,
    ) -> Result<bool, LoadError> {
        if !host.uses_building_style(occupant) {
            return Ok(false);
        }
        self.load_models_with_shadow(occupant, zoom, rotation, type_id, host)
    }

    pub fn create_occupant_shadow(
        &self,
        occupant: Handle,
        zoom: i32,
        view_rotation: i32,
        host: &mut impl ShadowHost,
    ) -> Result<bool, LoadError> {
        let ex = host.exemplar(occupant);
        if host.property_bool(ex, 0x0292_44DB) {
            return Ok(false);
        }
        let detail = host.shadow_detail();
        let rotation = if let Some(building) = host.query_interface(occupant, 0x87DD_A3A9) {
            let disabled = host.property_bool(ex, 0xEA32_09F9);
            let rotation = if disabled {
                0
            } else {
                host.interface_word(building, 14) as i32
            };
            host.release(building);
            if disabled {
                return Ok(false);
            }
            rotation
        } else if let Some(prop) = host.query_interface(occupant, 0xE979_3A65) {
            let disabled = detail < 3 || !host.property_bool(ex, 0x8A5E_5DB8);
            let rotation = if disabled {
                0
            } else {
                host.interface_word(prop, 9) as i32
            };
            host.release(prop);
            if disabled {
                return Ok(false);
            }
            rotation
        } else if host.type_id(occupant) == POWER_POLE {
            0
        } else {
            // §12.12's exclusion gate precedes the flora orientation fallback.
            return Ok(false);
        };
        let property1 = host.property_present(ex, 0xE83A_081D);
        let mut selected = None;
        for z in (0..=zoom).rev() {
            let mut key = ModelKey::s3d(0);
            host.shadow_model_id(
                occupant,
                z,
                view_rotation.wrapping_add(1) % 4,
                rotation,
                property1,
                &mut key,
            );
            let Some(resource) = host.load_s3d(key) else { continue };
            if let Some(model) = host.resource_model(resource) {
                let meshes = host.shadow_meshes(model);
                if !meshes.is_empty() && (detail >= 4 || meshes.len() == 1) {
                    let bounds = host.resource_bounds(resource);
                    if (bounds.max[1] - bounds.min[1]) * host.camera_scale() < host.shadow_threshold() {
                        host.release(model);
                        host.release(resource);
                        return Ok(false);
                    }
                    selected = Some((resource, model, meshes));
                    break;
                }
                host.release(model);
            }
            host.release(resource);
        }
        let Some((resource, model, meshes)) = selected else {
            return Ok(false);
        };
        let mut transform = Transform::default();
        let position = host.position(occupant);
        let tilt_to_terrain = host.property_bool(ex, 0x69F1_4D33);
        find_model_transform(
            TransformInput {
                flag: property1,
                orientation: rotation,
                position,
                offset: None,
                tilt_to_terrain,
            },
            &mut transform,
            host,
        );
        transform.rotate_shadow();
        let bounds = if detail < 5 {
            host.occupant_bounds(occupant)
        } else {
            host.model_bounds(model).transformed(&transform)
        };
        let mut result = Ok(true);
        for mesh in meshes {
            if mesh.material_state < 1 || mesh.positions.len() < 3 {
                continue;
            }
            let id = host.add_model_shadow(&mesh, bounds, transform);
            if id != -1 {
                if let Some(holder) = host.holder(occupant) {
                    host.holder_add_shadow(holder, id);
                } else {
                    result = Err(LoadError::MissingHolder);
                    break;
                }
            }
        }
        host.release(model);
        host.release(resource);
        result
    }
}
