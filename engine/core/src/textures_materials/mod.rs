// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable owners from texture-bindings.md §§1–3, with graphics-formats.md §§2.10/3.8.
//!
//! No native pointers or layouts are reproduced. Factory handles are never reused, so
//! FlushAll invalidates even outstanding handles without exposing freed Rust memory.
//! Getters use shared borrows; a native adapter must synchronize concurrent mutation.
//! Providers supply foreign services and image primitives, never cache or upload policy.

mod binding;
mod factory;
mod material;

pub use binding::{Binding, Release};
pub use factory::{BindingFactory, BindingId, BindingInfo, Stats};
pub use material::{material_flags, Material, MaterialManager, MaterialRecord, MaterialTexture, TextureSlot};

#[cfg(test)]
mod tests;

pub const FACTORY_CLASS: u32 = 0x9AC0_E23C;
pub const FACTORY_SERVICE: u32 = 0x1AC0_E11A;
pub const MATERIAL_CLASS: u32 = 0x9AC0_E23D;
pub const MATERIAL_SERVICE: u32 = 0x1AC0_E11B;
pub const SERVICE_PRIORITY: i32 = 0x0013_0EE8;
pub const IMAGE_TYPE: u32 = 0x7AB5_0E44;
pub const DEFAULT_GROUP: u32 = 0x1ABE_787D;
pub const RESTORE_TEXTURES: u32 = 0x8ACA_A95F;

/// Foreign cRZSystemService operations inherited by these owners (§§1.1, 3).
/// The base owns its count, interface set and deletion decision; not texture policy.
pub trait SystemService {
    fn query_interface(&mut self, iid: u32) -> Option<u32>;
    fn add_ref(&mut self) -> u32;
    fn release(&mut self) -> u32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceKey {
    pub type_id: u32,
    pub group: u32,
    pub instance: u32,
}

/// Counts returned by the foreign FindBitCounts primitive, not inferred from FSH codes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BitCounts {
    pub red: u32,
    pub green: u32,
    pub blue: u32,
    pub alpha: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageDescription {
    pub frames: u32,
    pub levels: u32,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub component_type: u32,
    pub bits: BitCounts,
}

/// A borrowed frame/level table; existing format readers provide the actual bytes.
pub trait ImageInfo {
    fn description(&self) -> ImageDescription;
    fn level_data(&self, frame: u32, level: u32) -> Option<&[u8]>;
}

pub trait ImageResource {
    fn image_info(&self) -> Option<&dyn ImageInfo>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BindingParams {
    pub frames: u32,
    pub levels: u32,
    pub width: u32,
    pub height: u32,
    pub internal_format: u32,
    pub bits_per_texel: u32,
    pub bytes: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    NoLevels,
    Overflow,
}

impl BindingParams {
    /// §1.4. Halving for byte accounting intentionally does not clamp to one.
    pub fn for_image(image: ImageDescription, lod: u32, depth: u32) -> Result<Self, LayoutError> {
        // TB5: zero levels and unrepresentable 32-bit sizes have no published
        // numeric contract. Reject them as a safe portable boundary, not native parity.
        if image.levels == 0 {
            return Err(LayoutError::NoLevels);
        }
        let d = lod.min(image.levels - 1);
        let width = image
            .width
            .max(1)
            .checked_next_power_of_two()
            .ok_or(LayoutError::Overflow)?;
        let height = image
            .height
            .max(1)
            .checked_next_power_of_two()
            .ok_or(LayoutError::Overflow)?;
        let width = width.checked_shr(d).unwrap_or(0).max(1);
        let height = height.checked_shr(d).unwrap_or(0).max(1);
        let internal_format = match image.format {
            7 | 8 => 5,
            9 => 6,
            10 => 7,
            _ if depth >= 24 && image.bits.alpha == 0 => 1,
            _ if depth >= 24 => 4,
            _ if image.bits.alpha == 0 => 0,
            _ if image.bits.alpha == 1 => 3,
            _ => 2,
        };
        let bits_per_texel = [16, 24, 16, 16, 32, 4, 8, 8][internal_format as usize];
        let levels = image.levels - d;
        let (mut w, mut h, mut texels) = (width, height, 0_u64);
        for _ in 0..levels {
            texels = texels
                .checked_add(u64::from(w) * u64::from(h))
                .ok_or(LayoutError::Overflow)?;
            w >>= 1;
            h >>= 1;
            if w == 0 || h == 0 {
                break;
            }
        }
        let bytes = texels
            .checked_mul(u64::from(bits_per_texel))
            .and_then(|n| n.checked_mul(u64::from(image.frames)))
            .ok_or(LayoutError::Overflow)?
            / 8;
        Ok(Self {
            frames: image.frames,
            levels,
            width,
            height,
            internal_format,
            bits_per_texel,
            bytes: u32::try_from(bytes).map_err(|_| LayoutError::Overflow)?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Service {
    ResourceManager,
    GraphicSystem,
    Driver,
    MessageServer,
}

/// Acquisitions return owned references; loads return owned resources. All release
/// operations below correspond to references acquired by this factory, never borrows.
pub trait Provider: SystemService {
    type Resource: ImageResource;
    fn acquire(&mut self, service: Service, graphic_system: Option<u64>) -> Option<u64>;
    fn retain(&mut self, service: Service, id: u64);
    fn release_service(&mut self, service: Service, id: u64);
    fn subscribe(&mut self, server: u64, message: u32);
    fn unsubscribe(&mut self, server: u64, message: u32);
    fn load(&mut self, manager: u64, key: ResourceKey) -> Option<Self::Resource>;
    fn release_resource(&mut self, resource: Self::Resource);
    fn create_texture(&mut self, driver: u64, params: BindingParams, usage: u32) -> u32;
    fn driver_error(&mut self, driver: u64) -> bool;
    fn delete_texture(&mut self, driver: u64, texture: u32);
    fn upload(&mut self, driver: u64, upload: Upload<'_>);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Upload<'a> {
    pub texture: u32,
    pub level: u32,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub component_type: u32,
    pub data: &'a [u8],
}
