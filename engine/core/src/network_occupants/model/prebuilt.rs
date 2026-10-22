//! Resource boundary and the §10.1 draw-query/cache lifetime.
use super::*;
use crate::network_occupants::record::Triangle;
use crate::network_occupants::xform::Xform;
use std::sync::RwLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKey {
    /// The main model's ID is resolved by the external resource service;
    /// §10.1 publishes a fixed type/group only for the second model.
    MainInstance(u32),
    Tgi {
        kind: u32,
        group: u32,
        instance: u32,
    },
}

impl ResourceKey {
    pub fn network(instance: u32) -> Self {
        Self::Tgi {
            kind: 0x5AD0_E817,
            group: 0xBADB_57F1,
            instance,
        }
    }

    pub fn instance_id(self) -> u32 {
        match self {
            Self::MainInstance(id) | Self::Tgi { instance: id, .. } => id,
        }
    }
}

/// Only external model/resource interfaces: no Bind/Deform/Clip/GetModelInstances
/// result, renderer callback, cache callback, or null-cache default.
pub trait ModelResource: Send + Sync {
    fn meshes(&self, lod: u16) -> Option<Vec<Mesh>>;
    fn bounds(&self) -> Bounds;
}

pub trait ModelProvider {
    /// External cSC4ModelMakerUtility, not the network owner's method.
    fn model_instance_id(&self, rotation: u8, alternate: bool) -> u32;
    fn load(&self, key: ResourceKey) -> Option<Arc<dyn ModelResource>>;
}

/// The plain piece always returns this actual embedded instance, including
/// when no exemplar model exists. Getters can run during a draw query.
pub struct EmbeddedInstance {
    pub transform: RwLock<Xform>,
    pub quad: [Vertex; 4],
    pub triangles: Vec<Triangle>,
}

pub enum PieceRender {
    Embedded(Arc<EmbeddedInstance>),
    Model(Arc<NetworkModelInstance>),
}

pub struct DrawQuery {
    pub instances: Vec<PieceRender>,
    /// Primary slot 81 receives each successfully appended model instance.
    /// This records the dispatch, not an invented appendix-method result.
    pub appendix: Vec<usize>,
    pub update_shadow: bool,
}

pub struct DrawState {
    pub position: [f32; 3],
    pub model_offset: [f32; 3],
    pub rotation: u8,
    pub orientation_byte: u8,
    pub network_flags: u32,
    pub corners: CornerHeights,
    pub terrain_heights: CornerHeights,
    pub path_id: u32,
    pub path_origin: [f32; 2],
    /// SC4ViewUtilities' external yaw when the position-query flag is true.
    pub object_yaw: f32,
    pub use_object_yaw: bool,
    pub view_yaw: f32,
    pub option_27812821: bool,
    pub option_6a845768: bool,
    pub alpha: u8,
    pub shadow_byte: u8,
    pub second_id: u32,
    pub second_yaw: f32,
    pub second_offset: [f32; 3],
}

pub struct PrebuiltPiece {
    pub embedded: Arc<EmbeddedInstance>,
    main_cache: Option<Arc<NetworkModelInstance>>,
    second_cache: Option<Arc<NetworkModelInstance>>,
    pub shadow_byte: u8,
}

impl PrebuiltPiece {
    pub fn new(quad: [Vertex; 4], triangles: Vec<Triangle>, shadow_byte: u8) -> Self {
        Self {
            embedded: Arc::new(EmbeddedInstance {
                transform: RwLock::new(Xform::default()),
                quad,
                triangles,
            }),
            main_cache: None,
            second_cache: None,
            shadow_byte,
        }
    }

    pub fn cached_main(&self) -> Option<&Arc<NetworkModelInstance>> {
        self.main_cache.as_ref()
    }
    pub fn cached_second(&self) -> Option<&Arc<NetworkModelInstance>> {
        self.second_cache.as_ref()
    }

    pub fn plain_instances(&self, position: [f32; 3]) -> DrawQuery {
        let mut transform = self.embedded.transform.write().expect("embedded transform poisoned");
        transform.translation = position;
        transform.mask |= 1;
        transform.counter = transform.counter.wrapping_add(1);
        DrawQuery {
            instances: vec![PieceRender::Embedded(Arc::clone(&self.embedded))],
            appendix: Vec::new(),
            update_shadow: false,
        }
    }

    pub fn get_model_instances(
        &mut self,
        provider: &dyn ModelProvider,
        state: &DrawState,
        constants: ProcessorConstants,
    ) -> DrawQuery {
        let mut query = self.plain_instances(state.position);
        let id = provider.model_instance_id(state.rotation, !state.option_27812821);
        if id == 0 {
            return query;
        } // also skips second model and shadow update.
        self.append(provider, ResourceKey::MainInstance(id), state, constants, &mut query);
        if state.second_id != 0 {
            self.append(
                provider,
                ResourceKey::network(state.second_id),
                state,
                constants,
                &mut query,
            );
        }
        if state.shadow_byte != self.shadow_byte {
            query.update_shadow = true;
            self.shadow_byte = state.shadow_byte;
        }
        query
    }

    fn append(
        &mut self,
        provider: &dyn ModelProvider,
        key: ResourceKey,
        state: &DrawState,
        constants: ProcessorConstants,
        query: &mut DrawQuery,
    ) {
        let second = matches!(key, ResourceKey::Tgi { .. });
        // Load failure preserves the old cache; SetModel failure clears it.
        let Some(resource) = provider.load(key) else {
            return;
        };
        let Some(mut instance) = NetworkModelInstance::create(key, resource) else {
            if second {
                self.second_cache = None;
            } else {
                self.main_cache = None;
            }
            return;
        };
        let flip = flip_bit(state.orientation_byte);
        let yaw = if state.use_object_yaw {
            state.object_yaw
        } else {
            -state.view_yaw
        };
        let yaw = if second { yaw + state.second_yaw } else { yaw };
        let translation = std::array::from_fn(|i| {
            let position = state.position[i] + state.model_offset[i];
            if second {
                position + state.second_offset[i]
            } else {
                position
            }
        });
        instance.transform = ModelTransform::new(yaw, translation, flip);
        let clipped = !second && state.network_flags & 0x40000 != 0;
        if clipped {
            let mut lowest = f32::MAX;
            for h in state.terrain_heights {
                if h < lowest {
                    lowest = h;
                }
            }
            let h = (f64::from(lowest) - f64::from(state.position[1])) as f32;
            let b = Bounds {
                min: [-f32::MAX, h, -f32::MAX],
                max: [f32::MAX; 3],
            };
            instance.clip(Some(b), 1).expect("network mode is specified");
        }
        // Literal option exception: Clip still runs; the other passes skip.
        // NO30 asks to disambiguate the clipped/option rows of §10.2.
        if !state.option_6a845768 {
            instance.bind(constants);
            if !second && !clipped {
                let mut deformation =
                    Deformation::from_corners(state.corners, state.position[1], state.network_flags & 0x1000000 != 0);
                deformation.rotation = state.rotation;
                deformation.flip = flip;
                if state.path_id != 0 {
                    deformation.origin = state.path_origin.map(|v| -v);
                }
                instance.deform(deformation);
            }
            instance.flip_and_light(flip);
        }
        if state.alpha != 0xFF {
            instance.alpha = Some(state.alpha as f32 * 0.003_921_569);
            instance.draw_flags |= 2;
        }
        // Fresh Arc is the construction reference. Cloning into the caller
        // then moving into AutoRef leaves exactly caller + cache, not a leak.
        // Safe stable Arc allocation aborts on OOM: native new-failure cache
        // clearing is an exact remaining branch, not a fabricated host input.
        let instance = Arc::new(instance);
        query.appendix.push(query.instances.len());
        query.instances.push(PieceRender::Model(Arc::clone(&instance)));
        if second {
            self.second_cache = Some(instance);
        } else {
            self.main_cache = Some(instance);
        }
    }

    /// NO26: inactive returns 1 without touching either cache or shadow byte.
    /// Active clears main then second; the caller continues with base Shutdown.
    pub fn shutdown(&mut self, active: bool) -> bool {
        if active {
            self.main_cache = None;
            self.second_cache = None;
        }
        true
    }
}

impl Drop for PrebuiltPiece {
    fn drop(&mut self) {
        // Destructor order is the reverse of successful Shutdown (NO26).
        drop(self.second_cache.take());
        drop(self.main_cache.take());
    }
}
