// SPDX-License-Identifier: GPL-3.0-or-later
//! Â§2.1 non-atomic load transaction; typed acquired references use Arc.
use super::{numeric::X, *};

pub const MODEL_HEADER_IID: u32 = 0x29a5d1ec;
pub const TEXTURE_SERVICE_ID: u32 = 0x1ac0e11a;
pub const TEXTURE_SERVICE_IID: u32 = 0xfac0e219;

#[derive(Debug)]
pub struct ModelHeader {
    pub texture_key: ResourceKey,
    pub zoom_type: u32,
    pub zoom_group: u32,
    pub instances: [u32; 5],
    pub scales: [f32; 5],
    pub frame_count: u32,
}
#[derive(Debug)]
pub struct TextureBinding {
    pub table: Vec<TextureHandle>,
}
pub trait SpriteTextureService {
    fn binding(&self, key: ResourceKey) -> Option<Arc<TextureBinding>>;
}
pub trait OwnerResourceServices {
    fn header(&mut self, key: ResourceKey, iid: u32, options: [u32; 2]) -> Option<Arc<ModelHeader>>;
    fn zoom(&mut self, key: ResourceKey, iid: u32, options: [u32; 2]) -> Option<Arc<RawSpriteHolder>>;
    fn texture_service(&mut self, service: u32, iid: u32) -> Option<Arc<dyn SpriteTextureService>>;
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HolderMetrics {
    pub steps: [f32; 3],
    pub bases: [f32; 2],
    pub p: u32,
    pub r: u32,
    pub product: u32,
    pub divisor: u32,
}
impl RawSpriteHolder {
    pub fn derive_metrics(&self) -> Result<HolderMetrics, Unsupported> {
        if self.divisors.contains(&0) || self.angle_limits.iter().any(|x| !x.is_finite()) {
            return Err(Unsupported::Holder);
        }
        let steps = self
            .divisors
            .map(|d| X::f(f32::from_bits(0x40c90fdb)).div(X::integer(i64::from(d))).store());
        if steps.iter().any(|s| !s.is_finite()) {
            return Err(Unsupported::Arithmetic);
        }
        let count = |lo, hi, step| {
            let value = X::f(hi).sub(X::f(lo)).div(X::f(step)).add(X::f(0.1));
            if value.valid() {
                Ok(value.trunc().wrapping_add(1) as u32)
            } else {
                Err(Unsupported::Arithmetic)
            }
        };
        let p = count(self.angle_limits[0], self.angle_limits[1], steps[1])?;
        let r = count(self.angle_limits[2], self.angle_limits[3], steps[2])?;
        Ok(HolderMetrics {
            steps,
            bases: [self.angle_limits[0], self.angle_limits[2]],
            p,
            r,
            product: p.wrapping_mul(r),
            divisor: self.divisors[0],
        })
    }
}
#[derive(Debug, Default)]
pub struct ZoomSlot {
    pub resource: Option<Arc<RawSpriteHolder>>,
    // None represents the constructor's deliberately uninitialized words.
    pub scale: Option<f32>,
    pub metrics: Option<HolderMetrics>,
}
#[derive(Debug, Default)]
pub struct OwnerResources {
    pub frame_count: u32,
    pub zooms: [ZoomSlot; 5],
    pub binding: Option<Arc<TextureBinding>>,
}
impl<O> AnimatedSprite<O> {
    pub fn resources(&self) -> &OwnerResources {
        &self.loaded
    }
    pub fn load(&mut self, key: ResourceKey, services: &mut impl OwnerResourceServices) -> Result<bool, Unsupported> {
        let Some(header) = services.header(key, MODEL_HEADER_IID, [0, 0]) else {
            return Ok(false);
        };
        // The temporary acquired header is released after the borrowed load.
        self.load_from_resource(Some(&header), services)
    }
    pub fn load_from_resource(
        &mut self,
        header: Option<&ModelHeader>,
        services: &mut impl OwnerResourceServices,
    ) -> Result<bool, Unsupported> {
        let Some(header) = header else {
            return Ok(false);
        };
        self.loaded.frame_count = header.frame_count;
        let mut successes = 0;
        let mut unsupported = None;
        for i in 0..5 {
            self.loaded.zooms[i].scale = Some(header.scales[i]);
            let key = ResourceKey {
                resource_type: header.zoom_type,
                group: header.zoom_group,
                instance: header.instances[i],
            };
            let Some(resource) = services.zoom(key, SPRITE_TABLE_INTERFACE, [0, 0]) else {
                break;
            };
            let slot = &mut self.loaded.zooms[i];
            // Release old before retaining new, including on a self-reload.
            slot.resource = None;
            slot.resource = Some(Arc::clone(&resource));
            match resource.derive_metrics() {
                Ok(metrics) => slot.metrics = Some(metrics),
                Err(error) => unsupported = Some(error),
            }
            successes += 1;
            if unsupported.is_some() {
                break;
            }
        }
        if successes == 5 {
            if let Some(service) = services.texture_service(TEXTURE_SERVICE_ID, TEXTURE_SERVICE_IID) {
                self.loaded.binding = service.binding(header.texture_key);
                if self.loaded.binding.is_some() {
                    return Ok(true);
                }
            }
        }
        for i in (0..successes).rev() {
            self.loaded.zooms[i].resource = None;
        }
        if let Some(error) = unsupported {
            Err(error)
        } else {
            Ok(false)
        }
    }

    pub fn draw_loaded(&mut self, input: DrawInputs, context: &mut impl SpriteDrawContext) -> Result<u32, Unsupported> {
        // Clone references to let the owner mutate its caches, not resource data.
        let resources: [Option<RawSpriteHolder>; 5] = std::array::from_fn(|i| {
            self.loaded.zooms[i].resource.as_ref().map(|resource| {
                let mut record = (**resource).clone();
                record.scale = self.loaded.zooms[i].scale.unwrap_or(record.scale);
                record
            })
        });
        let binding = self.loaded.binding.clone();
        let textures = binding.as_ref().map_or(&[][..], |b| b.table.as_slice());
        self.draw(
            input,
            DrawResources {
                holders: std::array::from_fn(|i| resources[i].as_ref()),
                textures,
            },
            context,
        )
    }
}

/// Â§2's sprite route. The other factory's resource-manager IID/return interface
/// is not specified (AS11); no fabricated successful foreign-model factory.
pub fn create_sprite_from_key(
    key: ResourceKey,
    services: &mut impl OwnerResourceServices,
) -> Result<Option<AnimatedSprite<()>>, Unsupported> {
    if creation_route(key) != CreationRoute::SpriteInstance {
        return Err(Unsupported::Holder);
    }
    let mut sprite = AnimatedSprite::default();
    if sprite.load(key, services)? {
        Ok(Some(sprite))
    } else {
        Ok(None)
    }
}
