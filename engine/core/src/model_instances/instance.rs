// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::any::Any;

pub type OwnerReference = Reference<Box<dyn Any + Send + Sync>>;

/// Semantic 20-byte record. The low byte of flags is byte 0; higher bytes
/// preserve bytes 1..3, including the constructor's unwritten bytes 2..3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawAttributes {
    pub flags: u32,
    pub opaque: [u8; 16],
}
impl Default for DrawAttributes {
    fn default() -> Self {
        let mut opaque = [0; 16];
        for word in opaque.chunks_exact_mut(4) {
            word.copy_from_slice(&1_f32.to_bits().to_le_bytes());
        }
        // MI3 [choice]: initialize the two unwritten bytes to zero.
        Self { flags: 0, opaque }
    }
}
impl DrawAttributes {
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self {
            flags: u32::from_le_bytes(bytes[..4].try_into().unwrap()),
            opaque: bytes[4..].try_into().unwrap(),
        }
    }
    pub fn to_bytes(self) -> [u8; 20] {
        let mut bytes = [0; 20];
        bytes[..4].copy_from_slice(&self.flags.to_le_bytes());
        bytes[4..].copy_from_slice(&self.opaque);
        bytes
    }
}

pub trait BasicDerived {
    fn world_bounds(&self) -> BoundingBox;
    fn sort_value(&self) -> u32;
}

/// cS3DModelInstanceBasic: abstract bounds/sort are supplied by the derived owner.
pub struct BasicInstance {
    owner: Option<OwnerReference>,
    custom_data: u32,
    properties: u32,
    attributes: DrawAttributes,
}
impl BasicInstance {
    /// Portable convenience constructor with explicitly supplied initial fields.
    pub fn new(custom_data: u32, properties: u32, attributes: DrawAttributes) -> Self {
        Self {
            owner: None,
            custom_data,
            properties,
            attributes,
        }
    }
    pub fn owner(&self) -> Option<&OwnerReference> {
        self.owner.as_ref()
    }
    pub fn set_owner(&mut self, owner: Option<&OwnerReference>) {
        if self.owner.as_ref().zip(owner).is_some_and(|(a, b)| a.ptr_eq(b)) {
            return;
        }
        // Basic releases old before retaining new (MI3).
        self.owner = None;
        self.owner = owner.cloned();
    }
    pub fn custom_data(&self) -> u32 {
        self.custom_data
    }
    pub fn set_custom_data(&mut self, data: u32) {
        self.custom_data = data;
    }
    pub fn properties(&self) -> u32 {
        self.properties
    }
    pub fn set_properties(&mut self, properties: u32) {
        self.properties = properties;
    }
    pub fn draw_attributes(&self) -> DrawAttributes {
        self.attributes
    }
    pub fn set_draw_attributes(&mut self, attributes: DrawAttributes) {
        self.attributes = attributes;
    }
    pub fn bounds(&self, derived: &impl BasicDerived) -> BoundingBox {
        derived.world_bounds()
    }
    pub fn world_bounds(&self, derived: &impl BasicDerived) -> BoundingBox {
        derived.world_bounds()
    }
    pub fn sort_value(&self, derived: &impl BasicDerived) -> u32 {
        derived.sort_value()
    }
    pub fn get_transform(&self, output: &mut Transform) {
        output.make_identity();
    }
    pub fn set_transform(&mut self, _transform: &Transform) {}
    pub fn pick(&self, _context: &mut PickContext) -> bool {
        false
    }
    pub fn draw(&self) {}
    pub fn resource_model(&self) -> Option<&ModelReference> {
        None
    }
}
impl Default for BasicInstance {
    fn default() -> Self {
        Self::new(0, 2, DrawAttributes::default())
    }
}
impl Interfaces for BasicInstance {
    fn interface(&self, iid: u32) -> Option<InterfacePart> {
        match iid {
            IID_UNKNOWN | IID_BASIC => Some(InterfacePart::Basic),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub colour: u32,
    pub uv: [f32; 2],
}

/// Vertex fields not used by the processor can be retained by a foreign provider.
/// Format is the standard format number; the processor accepts exactly 2.
#[derive(Debug)]
pub struct VertexBlock {
    pub format: u32,
    pub vertices: Vec<Vertex>,
    pub bounds: BoundingBox,
}
impl VertexBlock {
    pub fn new(format: u32, vertices: Vec<Vertex>) -> Self {
        let bounds = BoundingBox::from_points(vertices.iter().map(|v| v.position));
        Self {
            format,
            vertices,
            bounds,
        }
    }
    pub fn update_bounds(&mut self) {
        self.bounds = BoundingBox::from_points(self.vertices.iter().map(|v| v.position));
    }
}

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub vertices: Option<Reference<VertexBlock>>,
    pub indices: Option<Reference<Vec<u16>>>,
    pub primitives: Option<Reference<Vec<Primitive>>>,
}
impl Mesh {
    pub fn bounds(&self) -> BoundingBox {
        self.vertices.as_ref().map_or(BoundingBox::EMPTY, |v| v.read().bounds)
    }
}

/// A material's documented semantic fields, plus remaining uninterpreted data.
/// No offsets are invented for its native 52-byte layout.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Material {
    pub flags: u16,
    pub source_blend: u8,
    pub destination_blend: u8,
    pub byte_18: u8,
    pub textures: Vec<u32>,
    pub opaque: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct ModelFrame {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimationHeader {
    pub frame_count: u16,
    pub frame_rate: u16,
    pub mode: u8,
    pub flags: u8,
    pub displacement: f32,
}

/// Only the foreign resource service is a provider. All frame ownership,
/// attributes, playback, transform and processor algorithms are implemented here.
pub trait ModelProvider: Send + Sync {
    fn animation_header(&self) -> AnimationHeader;
    fn bounds(&self) -> BoundingBox;
    fn model_frame(&self, frame: u16) -> Option<ModelFrame>;
}
pub type ModelReference = Reference<Box<dyn ModelProvider>>;

pub trait DrawProvider {
    fn render_model_instance(
        &mut self,
        instance: &StaticInstance,
        transform: &Transform,
        attributes: DrawAttributes,
        option: bool,
    ) -> bool;
}

/// cS3DModelInstance and its mesh-container part share one owning allocation.
pub struct StaticInstance {
    basic: BasicInstance,
    transform: Transform,
    model: Option<ModelReference>,
    frame: ModelFrame,
    bounds: BoundingBox,
    bounds_stale: bool,
    properties: u16,
}
impl StaticInstance {
    pub fn new(attributes: DrawAttributes) -> Self {
        Self {
            basic: BasicInstance::new(0, 2, attributes),
            transform: Transform::identity(),
            model: None,
            frame: ModelFrame::default(),
            bounds: BoundingBox::EMPTY,
            bounds_stale: true,
            properties: 2,
        }
    }
    /// Portable frame-provider convenience. Native SetModel only binds the
    /// reference and inspects existing materials (set_model_reference below);
    /// the resource supplies frame creation separately. The bool and null clear
    /// here are portable choices, not extra native entry points.
    pub fn set_model(&mut self, model: Option<&ModelReference>) -> bool {
        let Some(model) = model else {
            self.frame = ModelFrame::default();
            self.model = None;
            self.bounds = BoundingBox::EMPTY;
            self.bounds_stale = false;
            return true;
        };
        if !self.model.as_ref().is_some_and(|old| old.ptr_eq(model)) {
            let Some(frame) = model.read().model_frame(0) else {
                return false;
            };
            // MI6: material fields are instance-local; block references can share.
            self.frame = frame;
        }
        self.set_model_reference(Some(model));
        true
    }
    pub fn set_model_reference(&mut self, model: Option<&ModelReference>) {
        let same = match (&self.model, model) {
            (Some(old), Some(new)) => old.ptr_eq(new),
            (None, None) => true,
            _ => false,
        };
        if !same {
            // Retain/store new before releasing old; same pointer avoids both.
            let old = std::mem::replace(&mut self.model, model.cloned());
            drop(old);
        }
        // §9.1 [choice]: absent material 0 is safe here, unlike native unchecked
        // access. No frame is built and no native failure return is invented.
        if let Some(material) = self.frame.materials.first_mut() {
            let special = material.source_blend == 8 && material.destination_blend == 1;
            if special || material.flags & 0x10 != 0 {
                self.properties = (self.properties | 4) & !2;
            }
            if special {
                let mut attributes = self.basic.draw_attributes();
                attributes.flags |= 0xE;
                self.basic.set_draw_attributes(attributes);
                material.flags &= !0x80;
                material.byte_18 = 0;
            }
        }
        self.bounds_stale = true;
    }
    pub fn resource_model(&self) -> Option<&ModelReference> {
        self.model.as_ref()
    }
    pub fn owner(&self) -> Option<&OwnerReference> {
        self.basic.owner()
    }
    pub fn set_owner(&mut self, owner: Option<&OwnerReference>) {
        if self.basic.owner.as_ref().zip(owner).is_some_and(|(a, b)| a.ptr_eq(b)) {
            return;
        }
        // Static retains new, stores it, then releases old (MI3).
        let old = std::mem::replace(&mut self.basic.owner, owner.cloned());
        drop(old);
    }
    pub fn custom_data(&self) -> u32 {
        self.basic.custom_data()
    }
    pub fn set_custom_data(&mut self, data: u32) {
        self.basic.set_custom_data(data);
    }
    pub fn mesh_count(&self) -> usize {
        self.frame.meshes.len()
    }
    pub fn mesh(&self, i: usize) -> Option<&Mesh> {
        self.frame.meshes.get(i)
    }
    pub fn mesh_mut(&mut self, i: usize) -> Option<&mut Mesh> {
        self.frame.meshes.get_mut(i)
    }
    pub fn material(&self, i: usize) -> Option<&Material> {
        self.frame.materials.get(i)
    }
    pub fn material_mut(&mut self, i: usize) -> Option<&mut Material> {
        self.frame.materials.get_mut(i)
    }
    pub fn meshes(&self) -> &[Mesh] {
        &self.frame.meshes
    }
    pub fn properties(&self) -> u16 {
        self.properties
    }
    pub fn set_properties(&mut self, properties: u32) {
        self.properties = properties as u16;
    }
    pub fn draw_attributes(&self) -> DrawAttributes {
        self.basic.draw_attributes()
    }
    pub fn set_draw_attributes(&mut self, attributes: DrawAttributes) {
        self.basic.set_draw_attributes(attributes);
    }
    pub fn transform(&self) -> Transform {
        self.transform
    }
    pub fn get_transform(&self, output: &mut Transform) {
        let counter = output.change_counter.wrapping_add(1);
        *output = self.transform;
        output.change_counter = counter;
    }
    pub fn set_transform(&mut self, transform: &Transform) {
        let counter = self.transform.change_counter.wrapping_add(1);
        self.transform = *transform;
        self.transform.change_counter = counter;
    }
    pub fn mark_bounds_stale(&mut self) {
        self.bounds_stale = true;
    }
    pub fn refresh_bounds(&mut self) {
        if let Some(model) = &self.model {
            self.bounds = model.read().bounds();
            self.bounds_stale = false;
        }
    }
    pub fn bounds(&mut self) -> BoundingBox {
        if self.bounds_stale {
            self.refresh_bounds();
        }
        self.bounds
    }
    pub fn world_bounds(&mut self) -> BoundingBox {
        self.bounds().transformed(&self.transform)
    }
    pub fn sort_value(&self) -> u32 {
        self.frame
            .materials
            .first()
            .and_then(|m| m.textures.first())
            .copied()
            .unwrap_or(0)
    }
    pub fn pick(&self, context: &mut PickContext) -> bool {
        context.model_pick(self.meshes(), &self.transform)
    }
    pub fn draw(&self, context: &mut impl DrawProvider, option: bool) -> bool {
        context.render_model_instance(self, &self.transform, self.draw_attributes(), option)
    }
    pub(crate) fn build_frame(&mut self, frame: u16) -> bool {
        let Some(model) = &self.model else {
            return false;
        };
        let Some(frame) = model.read().model_frame(frame) else {
            return false;
        };
        self.frame = frame;
        true
    }
    pub fn animation_header(&self) -> Option<AnimationHeader> {
        self.model.as_ref().map(|m| m.read().animation_header())
    }
}
impl Interfaces for StaticInstance {
    fn interface(&self, iid: u32) -> Option<InterfacePart> {
        match iid {
            IID_BASIC => Some(InterfacePart::Basic),
            IID_UNKNOWN | IID_MESH_CONTAINER => Some(InterfacePart::MeshContainer),
            _ => None,
        }
    }
}
