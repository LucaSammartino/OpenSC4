// SPDX-License-Identifier: GPL-3.0-or-later
//! Typed foreign records, not binary resource/factory implementations (AS1).

use std::sync::Arc;

pub const SPRITE_ANIMATION_TYPE: u32 = 0x29a5_d1ec;
pub const FOREIGN_MODEL_TYPE: u32 = 0x5ad0_e817;
pub const SPRITE_TABLE_INTERFACE: u32 = 0x69b6_f01c;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceKey {
    pub resource_type: u32,
    pub group: u32,
    pub instance: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreationRoute {
    SpriteInstance,
    ForeignResource,
    Unsupported,
}

/// §2: retain the seemingly swapped native resource-key routes.
pub fn creation_route(key: ResourceKey) -> CreationRoute {
    match key.resource_type {
        SPRITE_ANIMATION_TYPE => CreationRoute::SpriteInstance,
        FOREIGN_MODEL_TYPE => CreationRoute::ForeignResource,
        _ => CreationRoute::Unsupported,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureHandle(pub u64);

/// Decoded data only. AS1 still blocks interpretation of the native 8 bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameEntry {
    pub texture: TextureHandle,
    pub page_side: u32,
    pub pixel_origin: [u32; 2],
    pub pixel_size: [u32; 2],
    pub pixel_offset: [f32; 2],
    pub flip_u: bool,
    pub flip_v: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpriteTable {
    pub frames: u32,
    pub directions: u32,
    pub base_angle: f32,
    /// Direction-major decoded entries, not a guessed native layout.
    pub entries: Vec<FrameEntry>,
}

#[derive(Clone, Debug)]
pub struct ZoomRecord {
    pub table_key: ResourceKey,
    pub scale: f32,
}
#[derive(Clone, Debug)]
pub struct AnimationRecord {
    pub zooms: [ZoomRecord; 5],
}
#[derive(Clone, Debug)]
pub struct LoadedZoom {
    pub table: Arc<SpriteTable>,
    pub scale: f32,
    pub angle_step: f32,
}

pub trait SpriteResources {
    /// Must acquire a typed reference to data, not call a sprite-owner method.
    fn sprite_table(&mut self, key: ResourceKey, interface: u32) -> Option<Arc<SpriteTable>>;
}

#[derive(Debug)]
pub struct ZoomPrefix {
    pub zooms: Vec<LoadedZoom>,
    pub end: PrefixEnd,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefixEnd {
    Complete,
    Missing(usize),
    Invalid(usize),
}

/// §2: fetch in zoom order and stop at the first absent zoom. AS4: the native
/// Load return and partial-state lifetime are not specified; this result exposes
/// the prefix and stopping reason instead of reporting placeholder success.
pub fn acquire_zoom_prefix(resource: &AnimationRecord, provider: &mut impl SpriteResources) -> ZoomPrefix {
    let mut zooms = Vec::new();
    for (zoom, record) in resource.zooms.iter().enumerate() {
        let Some(table) = provider.sprite_table(record.table_key, SPRITE_TABLE_INTERFACE) else {
            return ZoomPrefix {
                zooms,
                end: PrefixEnd::Missing(zoom),
            };
        };
        if table.frames == 0 || table.directions == 0 || !table.base_angle.is_finite() || !record.scale.is_finite() {
            return ZoomPrefix {
                zooms,
                end: PrefixEnd::Invalid(zoom),
            };
        }
        zooms.push(LoadedZoom {
            angle_step: std::f32::consts::TAU / table.directions as f32,
            table,
            scale: record.scale,
        });
    }
    ZoomPrefix {
        zooms,
        end: PrefixEnd::Complete,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    Empty,
    NonFinite,
    MissingEntry,
    InvalidPage,
}

/// §4.1's equation. AS6: literal Rust round (ties away from zero); negative
/// directions use Euclidean modulo to keep the result in the table's rows.
pub fn direction_index(directions: u32, yaw: f32, base: f32) -> Result<u32, FrameError> {
    if directions == 0 {
        return Err(FrameError::Empty);
    }
    if !yaw.is_finite() || !base.is_finite() {
        return Err(FrameError::NonFinite);
    }
    let index = (directions as f32 * ((yaw + std::f32::consts::PI) - base) / std::f32::consts::TAU).round();
    if !index.is_finite() {
        return Err(FrameError::NonFinite);
    }
    Ok(index.rem_euclid(directions as f32) as u32)
}

impl SpriteTable {
    pub fn entry(&self, yaw: f32, frame: u32) -> Result<&FrameEntry, FrameError> {
        if self.frames == 0 {
            return Err(FrameError::Empty);
        }
        let direction = direction_index(self.directions, yaw, self.base_angle)?;
        let index = u64::from(direction) * u64::from(self.frames) + u64::from(frame % self.frames);
        let index = usize::try_from(index).map_err(|_| FrameError::MissingEntry)?;
        self.entries.get(index).ok_or(FrameError::MissingEntry)
    }
}

impl FrameEntry {
    /// Lower/upper pixel edges, flipped independently; native vertex winding
    /// and half-texel conventions are not supplied by the spec (AS6).
    pub fn uv_edges(&self) -> Result<[[f32; 2]; 2], FrameError> {
        if !self.page_side.is_power_of_two() {
            return Err(FrameError::InvalidPage);
        }
        let side = self.page_side as f32;
        let mut u = [
            self.pixel_origin[0] as f32 / side,
            (u64::from(self.pixel_origin[0]) + u64::from(self.pixel_size[0])) as f32 / side,
        ];
        let mut v = [
            self.pixel_origin[1] as f32 / side,
            (u64::from(self.pixel_origin[1]) + u64::from(self.pixel_size[1])) as f32 / side,
        ];
        if self.flip_u {
            u.swap(0, 1);
        }
        if self.flip_v {
            v.swap(0, 1);
        }
        Ok([u, v])
    }

    pub fn scaled_metrics(&self, scale: f32) -> ([f32; 2], [f32; 2]) {
        let scale = if (scale - 1.0).abs() > 0.0001 { scale } else { 1.0 };
        (
            self.pixel_size.map(|v| v as f32 * scale),
            self.pixel_offset.map(|v| v * scale),
        )
    }
}
