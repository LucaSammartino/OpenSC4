// SPDX-License-Identifier: GPL-3.0-or-later
//! Material construction, texture-bindings.md §3. No draw-context policy lives here.

use std::collections::BTreeMap;

use super::{BindingId, SystemService, Unsupported};

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

/// A nonvirtual value. Reference ownership is handled by MaterialArena; copying
/// this value must use assign_value, never a derived Clone of held bindings.
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
    /// Byte +0x0B is copied by value assignment but not initialized by the
    /// published constructor/CreateMaterial. None retains that unknown value.
    pub header_tail: Option<u8>,
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
            self.deleting_destructor(1);
        }
        Some(count)
    }

    pub fn deleting_destructor(&mut self, flags: u32) -> bool {
        if self.alive {
            self.base.destroy_base();
            self.alive = false;
        }
        flags & 1 != 0
    }

    pub fn create_material(&self, record: &dyn MaterialRecord) -> Option<Material> {
        self.try_create_material(record).ok()
    }

    /// Native creation always succeeds for valid allocated input. Allocation
    /// failure and T > 2 are invalid-memory paths, never native false promises.
    pub fn try_create_material(&self, record: &dyn MaterialRecord) -> Result<Material, Unsupported> {
        if !self.alive {
            return Err(Unsupported::InvalidMaterial);
        }
        if record.texture_count() > 2 {
            return Err(Unsupported::MaterialSlots);
        }
        let mut material = Material::constructor();
        for (index, slot) in material.textures.iter_mut().enumerate() {
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
        material.flags = material_flags(record.stored_flags());
        material.first_byte = 1;
        material.alpha_func = record.alpha_func();
        material.depth_func = record.depth_func();
        material.src_blend = record.src_blend();
        material.dst_blend = record.dst_blend();
        material.alpha_threshold = (record.alpha_threshold() >> 8) as u8;
        material.acquire();
        Ok(material)
    }

    pub fn create_material_variant(&self) -> bool {
        false
    }

    /// Primary slots 6/7: unnamed shared-false bodies; arguments are not read.
    pub fn slot_6(&self, _arg0: u32, _arg1: u32) -> bool {
        false
    }

    pub fn slot_7(&self, _arg0: u32, _arg1: u32) -> bool {
        false
    }

    pub fn stats_string(&self) -> &'static str {
        "Nothing to report"
    }
}

impl<B: SystemService> Drop for MaterialManager<B> {
    fn drop(&mut self) {
        self.deleting_destructor(0);
    }
}

/// Binding operations, not material ownership policy. IDs are safe stand-ins
/// for pointers; using a destroyed ID is outside the native valid-memory domain.
pub trait BindingReferences {
    fn acquire_binding(&mut self, binding: BindingId);
    fn release_binding(&mut self, binding: BindingId);
}

impl TextureSlot {
    pub fn assign(&mut self, source: &Self, bindings: &mut impl BindingReferences) {
        self.id = source.id;
        if self.binding != source.binding {
            if let Some(new) = source.binding {
                bindings.acquire_binding(new);
            }
            let old = self.binding;
            self.binding = source.binding;
            if let Some(old) = old {
                bindings.release_binding(old);
            }
        }
        self.wrap_s = source.wrap_s;
        self.wrap_t = source.wrap_t;
        self.mag_filter = source.mag_filter;
        self.min_filter = source.min_filter;
        self.environment = source.environment;
        self.coordinate_set = source.coordinate_set;
        self.transform_switch = source.transform_switch;
        self.scale_divisor = source.scale_divisor;
    }
}

impl Material {
    /// Constructor's known fields. Header scalars that CreateMaterial overwrites
    /// use Rust zero placeholders; their pre-creation native values are withheld.
    pub fn constructor() -> Self {
        Self {
            first_byte: 0,
            references: 0,
            flags: 0,
            alpha_func: 0,
            depth_func: 0,
            src_blend: 0,
            dst_blend: 0,
            alpha_threshold: 0,
            header_tail: None,
            textures: [TextureSlot::default(), TextureSlot::default()],
        }
    }

    pub fn acquire(&mut self) -> u16 {
        self.references = self.references.wrapping_add(1);
        self.references
    }

    pub fn assign_value(&mut self, source: &Self, bindings: &mut impl BindingReferences) {
        self.first_byte = source.first_byte;
        self.references = source.references;
        self.flags = source.flags;
        self.alpha_func = source.alpha_func;
        self.depth_func = source.depth_func;
        self.src_blend = source.src_blend;
        self.dst_blend = source.dst_blend;
        self.alpha_threshold = source.alpha_threshold;
        self.header_tail = source.header_tail;
        self.textures[0].assign(&source.textures[0], bindings);
        self.textures[1].assign(&source.textures[1], bindings);
    }

    fn release_stages(&self, bindings: &mut impl BindingReferences) {
        for slot in self.textures.iter().rev() {
            if let Some(binding) = slot.binding {
                bindings.release_binding(binding);
            }
        }
    }
}

/// Forward value-range cleanup does not consult counts or free the enclosing
/// range. As in native destruction, the values may not subsequently be reused.
pub fn release_value_range(values: &[Material], bindings: &mut impl BindingReferences) {
    for value in values {
        value.release_stages(bindings);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaterialId(u64);

#[derive(Default)]
pub struct MaterialArena {
    values: BTreeMap<MaterialId, Material>,
    next: u64,
}

impl MaterialArena {
    pub fn insert(&mut self, value: Material) -> MaterialId {
        self.next = self.next.checked_add(1).expect("material token space exhausted");
        let id = MaterialId(self.next);
        self.values.insert(id, value);
        id
    }

    pub fn get(&self, id: MaterialId) -> Option<&Material> {
        self.values.get(&id)
    }

    pub fn get_mut(&mut self, id: MaterialId) -> Option<&mut Material> {
        self.values.get_mut(&id)
    }

    pub fn release(&mut self, id: MaterialId, bindings: &mut impl BindingReferences) -> Result<u32, Unsupported> {
        let value = self.values.get_mut(&id).ok_or(Unsupported::InvalidMaterial)?;
        if value.references > 1 {
            value.references -= 1;
            return Ok(u32::from(value.references));
        }
        value.release_stages(bindings);
        self.values.remove(&id);
        Ok(0)
    }

    pub fn assign_reference(
        &mut self,
        target: &mut Option<MaterialId>,
        source: Option<MaterialId>,
        bindings: &mut impl BindingReferences,
    ) -> Result<(), Unsupported> {
        if *target == source {
            return Ok(());
        }
        if let Some(new) = source {
            self.values.get_mut(&new).ok_or(Unsupported::InvalidMaterial)?.acquire();
        }
        let old = *target;
        *target = source;
        if let Some(old) = old {
            self.release(old, bindings)?;
        }
        Ok(())
    }
}
