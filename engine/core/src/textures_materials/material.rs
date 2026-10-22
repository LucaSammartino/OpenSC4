// SPDX-License-Identifier: GPL-3.0-or-later
//! Material construction, texture-bindings.md §3. No draw-context policy lives here.

use super::{BindingId, SystemService};

pub const fn material_flags(stored: u32) -> u16 {
    stored as u16 | 0x0180
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialTexture {
    pub id: u32,
    pub wrap_s: u8,
    pub wrap_t: u8,
    pub mag_filter: u8,
    pub min_filter: u8,
}

/// View of the already-parsed MATS record. The format reader owns versioning,
/// validation and the last-ID-zero texture-count quirk (graphics-formats §3.8).
pub trait MaterialRecord {
    fn stored_flags(&self) -> u32;
    fn alpha_func(&self) -> u8;
    fn depth_func(&self) -> u8;
    fn src_blend(&self) -> u8;
    fn dst_blend(&self) -> u8;
    fn alpha_threshold(&self) -> u16;
    fn texture_count(&self) -> usize;
    fn texture(&self, index: usize) -> MaterialTexture;
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextureSlot {
    /// Unset in the constructor; only active slots receive an ID.
    pub id: Option<u32>,
    pub binding: Option<BindingId>,
    pub wrap_s: u8,
    pub wrap_t: u8,
    pub mag_filter: u8,
    pub min_filter: u8,
    pub environment: u8,
    pub coordinate_set: u8,
    pub transform_switch: u8,
    pub scale_divisor: f32,
}

impl Default for TextureSlot {
    fn default() -> Self {
        Self {
            id: None,
            binding: None,
            wrap_s: 3,
            wrap_t: 3,
            mag_filter: 1,
            min_filter: 1,
            environment: 1,
            coordinate_set: 0,
            transform_switch: 0,
            scale_divisor: 1.0,
        }
    }
}

/// Published construction state only. TB4 requests the material's remaining
/// methods/lifetime; no AddRef/Release or binding-ownership contract is invented.
#[derive(Debug, PartialEq)]
pub struct Material {
    pub first_byte: u8,
    pub references: u16,
    pub flags: u16,
    pub alpha_func: u8,
    pub depth_func: u8,
    pub src_blend: u8,
    pub dst_blend: u8,
    pub alpha_threshold: u8,
    pub textures: [TextureSlot; 2],
}

pub struct MaterialManager<B: SystemService> {
    base: B,
    alive: bool,
}

impl<B: SystemService> MaterialManager<B> {
    pub fn new(base: B) -> Self {
        Self { base, alive: true }
    }

    pub fn query_interface(&mut self, iid: u32) -> Option<u32> {
        if !self.alive {
            return None;
        }
        if iid == 0xFAC0_E21A {
            self.base.add_ref();
            Some(0)
        } else {
            // Base offsets are relative to the +4 system-service subobject.
            self.base.query_interface(iid).map(|offset| offset + 4)
        }
    }

    pub fn add_ref(&mut self) -> Option<u32> {
        self.alive.then(|| self.base.add_ref())
    }

    pub fn release(&mut self) -> Option<u32> {
        if !self.alive {
            return None;
        }
        let count = self.base.release();
        if count == 0 {
            self.alive = false;
        }
        Some(count)
    }

    pub fn deleting_destructor(&mut self, flags: u32) -> bool {
        self.alive = false;
        flags & 1 != 0
    }

    pub fn create_material(&self, record: &dyn MaterialRecord) -> Option<Material> {
        if !self.alive || record.texture_count() > 2 {
            return None;
        }
        let mut textures = [TextureSlot::default(), TextureSlot::default()];
        for (index, slot) in textures.iter_mut().enumerate() {
            if index < record.texture_count() {
                let source = record.texture(index);
                slot.id = Some(source.id);
                slot.wrap_s = source.wrap_s;
                slot.wrap_t = source.wrap_t;
                slot.mag_filter = source.mag_filter;
                slot.min_filter = source.min_filter;
            } else {
                slot.environment = 0xFF;
            }
        }
        Some(Material {
            first_byte: 1,
            references: 1,
            flags: material_flags(record.stored_flags()),
            alpha_func: record.alpha_func(),
            depth_func: record.depth_func(),
            src_blend: record.src_blend(),
            dst_blend: record.dst_blend(),
            alpha_threshold: (record.alpha_threshold() >> 8) as u8,
            textures,
        })
    }

    pub fn create_material_variant(&self) -> bool {
        false
    }

    pub fn stats_string(&self) -> &'static str {
        "Nothing to report"
    }
}
