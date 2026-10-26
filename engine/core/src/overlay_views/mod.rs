// SPDX-License-Identifier: GPL-3.0-or-later
//! Clean-room overlay owners. See README.md for supported boundaries and closure.
mod marked;
mod network;
mod path;
mod train;
mod vertex_buffer;

pub use marked::*;
pub use network::*;
pub use path::*;
pub use train::*;
pub use vertex_buffer::*;
pub use MarkedCellView as SC4MarkedCellView;
pub use PathInfoView as SC4PathInfoView;
pub use PathView as cSC4PathView;
pub use TrainJunctionView as SC4TrainJunctionView;
pub use VertexBufferExtension as cSTEVertBufExtn;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    pub fn plus(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }
    pub fn minus(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }
    pub fn times(self, k: f32) -> Self {
        Self::new(self.x * k, self.y * k, self.z * k)
    }
    pub fn dot(self, b: Self) -> f32 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }
    pub fn length(self) -> f32 {
        (f64::from(self.x).powi(2) + f64::from(self.y).powi(2) + f64::from(self.z).powi(2)).sqrt() as f32
    }
    pub fn horizontal_length(self) -> f32 {
        (f64::from(self.x).powi(2) + f64::from(self.z).powi(2)).sqrt() as f32
    }
}

/// None represents native uninitialized scratch alpha, never an invented default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: Option<u8>,
}
impl Colour {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a: Some(a) }
    }
    pub fn packed(bits: u32) -> Self {
        Self::rgba((bits >> 16) as u8, (bits >> 8) as u8, bits as u8, (bits >> 24) as u8)
    }
    pub fn bgra(self) -> Option<[u8; 4]> {
        self.a.map(|a| [self.b, self.g, self.r, a])
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub colour: Option<Colour>,
    /// None means no UV field (formats 1), or an unwritten path repeat (format 10).
    pub uv: Option<[f32; 2]>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Draw {
    pub primitive: u8,
    pub format: u8,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepthTest {
    Off,
    LessEqual,
    Greater,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blend {
    Off,
    SourceAlpha,
    Additive,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderState {
    pub colour_write: bool,
    pub depth_write: bool,
    pub depth_test: DepthTest,
    pub culling: bool,
    pub blend: Blend,
    pub textures: u8,
    pub alpha_test: bool,
    /// Exact published packing arguments; absent when only semantic fields are specified.
    pub tuple: Option<[i32; 14]>,
}
pub(crate) fn overlay_state(depth_test: DepthTest, textures: u8, blend: Blend) -> RenderState {
    RenderState {
        colour_write: true,
        depth_write: false,
        depth_test,
        culling: true,
        blend,
        textures,
        alpha_test: false,
        tuple: None,
    }
}

pub type Handle = u64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceKey {
    pub kind: u32,
    pub group: u32,
    pub instance: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Identity,
    DepthOffset(i32),
    State(RenderState),
    Texture(Handle),
    RepeatWrap,
    Transparency(f32),
    ResetTransparency,
    Draw(Draw),
    /// Adjusted model interface slot 10; unwritten record padding is not represented.
    ModelState {
        model: Handle,
        state: MarkerState,
    },
    /// Primary model slot 18, with the native integer argument and transparency.
    ModelTransparency {
        model: Handle,
        mode: u32,
        value: f32,
    },
    /// Adjusted model interface slot 12; the Renderer supplies its current context.
    DrawModel {
        model: Handle,
        flags: u32,
    },
}
pub trait Renderer {
    fn depth_offset(&self) -> i32;
    /// Driver draw return values are intentionally ignored by these owners.
    fn emit(&mut self, command: Command);
}
/// Only foreign resource operations; no same-owner geometry is delegated.
pub trait Resources {
    fn render_service_available(&mut self) -> bool;
    fn resource_service_available(&mut self) -> bool;
    /// Returned bindings are owned references.
    fn binding(&mut self, key: ResourceKey) -> Option<Handle>;
    /// Returned model is a temporary acquired reference.
    fn model(&mut self, key: ResourceKey) -> Option<Handle>;
    fn lot_filter(&mut self, mask: u32) -> Option<Handle>;
    fn retain(&mut self, handle: Handle);
    fn release(&mut self, handle: Handle);
    fn base_cleanup(&mut self, primary: u32);
    fn free_object(&mut self, primary: u32);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    /// Native undefined/caller-unsafe inputs are not translated to successful no-ops.
    Undefined(&'static str),
    Unsupported(&'static str),
}
pub type Result<T> = std::result::Result<T, Boundary>;

pub trait ViewOwner {
    fn deleting_cleanup(&mut self, resources: &mut impl Resources);
}
/// Portable model of §§3.4–3.5, not a native ABI or an atomic reference implementation.
pub struct View<T: ViewOwner> {
    pub owner: T,
    primary: u32,
    count: u32,
    deleted: bool,
}
impl<T: ViewOwner> View<T> {
    pub fn new(primary: u32, owner: T) -> Self {
        Self {
            owner,
            primary,
            count: 0,
            deleted: false,
        }
    }
    pub fn count(&self) -> u32 {
        self.count
    }
    pub fn deleted(&self) -> bool {
        self.deleted
    }
    pub fn query_interface(&mut self, id: u32, output: &mut u32) -> Result<bool> {
        if self.deleted {
            return Err(Boundary::Undefined("deleted view"));
        }
        if id != 1 {
            return Ok(false);
        }
        *output = self.primary.wrapping_add(4);
        self.add_ref()?;
        Ok(true)
    }
    pub fn add_ref(&mut self) -> Result<u32> {
        if self.deleted {
            return Err(Boundary::Undefined("deleted view"));
        }
        self.count = self.count.wrapping_add(1);
        Ok(self.count)
    }
    /// Secondary slot 4 differs from Release: even 1 -> 0 never deletes the owner.
    pub fn remove_ref(&mut self) -> Result<u32> {
        if self.deleted {
            return Err(Boundary::Undefined("deleted view"));
        }
        self.count = self.count.saturating_sub(1);
        Ok(self.count)
    }
    pub fn release(&mut self, resources: &mut impl Resources) -> Result<u32> {
        if self.deleted {
            return Err(Boundary::Undefined("deleted view"));
        }
        if self.count > 1 {
            self.count -= 1;
            return Ok(self.count);
        }
        self.delete(1, resources)?;
        Ok(0)
    }
    pub fn delete(&mut self, flags: u32, resources: &mut impl Resources) -> Result<u32> {
        if self.deleted {
            return Err(Boundary::Undefined("deleted view"));
        }
        self.owner.deleting_cleanup(resources);
        resources.base_cleanup(self.primary);
        if flags & 1 != 0 {
            resources.free_object(self.primary);
        }
        self.deleted = true;
        Ok(self.primary)
    }
    pub fn pick<O>(&self, _output: &mut O) -> bool {
        false
    }
}

#[cfg(test)]
mod tests;
