//! `cSC4NetworkOccupantModelInstance`, the model side of a network piece
//! (docs/specs/network-occupants.md §10, conformance test 24).
//!
//! What the spec publishes, and what is implemented here:
//! - the instance is a `cS3DModelInstance` subclass with exactly four
//!   overrides ([`ModelOverride`]): `Deform`, `FlipAndLight`, `Clip`,
//!   `Prelight`;
//! - `GetModelInstances` (primary slot 33, [`GET_MODEL_INSTANCES_SLOT`])
//!   returns it ([`PieceRender::model_instance`]);
//! - `Deform` bends the flat piece model to the four corner heights, corner
//!   order (x, z), (x, z+1), (x+1, z+1), (x+1, z) (network-geometry.md §8.1)
//!   ([`NetworkModelInstance::deform`]);
//! - `FlipAndLight` reads the flip, bit 7 of `+0x104` ([`flip_bit`]);
//! - with no model, the piece renders from the §2.2 quad and triangle list
//!   ([`PieceRender::Fallback`]).
//!
//! Not published (questions NO21-NO23): the interpolation and frame details
//! of `Deform`, what the flip and the lighting, clipping and pre-light passes
//! compute, and when the instance is created and released. The four passes
//! other than the corner bend are handed to a typed [`ModelRenderer`]
//! provider; the core never calls an original model-instance method for an
//! output.

use super::record::{Triangle, Vertex};

/// `GetModelInstances`: primary `cISC4NetworkOccupant` slot 33 (§10).
pub const GET_MODEL_INSTANCES_SLOT: u32 = 33;

/// The 16-metre cell edge (§2.4's quantiser divides world units by 16).
pub const CELL_SIZE: f32 = 16.0;

/// The four `cS3DModelInstance` overrides of §10, in the spec's table order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelOverride {
    Deform,
    FlipAndLight,
    Clip,
    Prelight,
}

impl ModelOverride {
    pub const ALL: [ModelOverride; 4] = [
        ModelOverride::Deform,
        ModelOverride::FlipAndLight,
        ModelOverride::Clip,
        ModelOverride::Prelight,
    ];
}

/// The flip `FlipAndLight` reads: bit 7 of the `+0x104` orientation byte.
pub fn flip_bit(orientation_byte: u8) -> bool {
    orientation_byte & 0x80 != 0
}

/// Four corner heights in network-geometry.md §8.1 order:
/// `[h(x, z), h(x, z+1), h(x+1, z+1), h(x+1, z)]`.
pub type CornerHeights = [f32; 4];

/// Where the cell lies in the model's local x/z: corner (x, z) at `origin`,
/// the far corner at `origin + extent` on both axes.
///
/// The spec does not give the model's local frame (question NO21 c); the
/// default is the most literal reading of "the cell": origin (0, 0), one
/// 16-metre cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeformFrame {
    pub origin: [f32; 2],
    pub extent: f32,
}

impl Default for DeformFrame {
    fn default() -> DeformFrame {
        DeformFrame {
            origin: [0.0, 0.0],
            extent: CELL_SIZE,
        }
    }
}

/// The corner height at local (x, z): bilinear over the four corners.
///
/// Literal reading of "the four corner heights interpolated per quad
/// position" (§13 test 24); bilinear rather than a two-triangle split and
/// no clamping outside the cell are choices (question NO21 a, d).
pub fn corner_height(c: &CornerHeights, frame: &DeformFrame, x: f32, z: f32) -> f32 {
    let s = (x - frame.origin[0]) / frame.extent;
    let t = (z - frame.origin[1]) / frame.extent;
    c[0] * (1.0 - s) * (1.0 - t) + c[1] * (1.0 - s) * t + c[2] * s * t + c[3] * s * (1.0 - t)
}

/// The resource side: the flat piece model for a key (the exemplar's model).
pub trait ModelProvider {
    /// The flat model's vertices, or `None` when the piece has no model.
    fn flat_model(&self, key: u64) -> Option<Vec<Vertex>>;
}

/// The renderer side: the three passes whose bodies the spec does not
/// publish (question NO22). They act on the deformed vertices in place.
pub trait ModelRenderer {
    /// `FlipAndLight` with the flip bit of `+0x104`.
    fn flip_and_light(&mut self, verts: &mut [Vertex], flip: bool);
    /// `Clip`: the underground/clipping view.
    fn clip(&mut self, verts: &mut [Vertex]);
    /// `Prelight`: the pre-light pass.
    fn prelight(&mut self, verts: &mut [Vertex]);
}

/// One `cSC4NetworkOccupantModelInstance`.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkModelInstance {
    /// The model key it was built from.
    pub key: u64,
    /// The flat model as loaded.
    pub flat: Vec<Vertex>,
    /// The vertices after the last pass; equal to `flat` until `deform`.
    pub verts: Vec<Vertex>,
    /// The corner heights of the last `deform`.
    pub corners: Option<CornerHeights>,
    pub frame: DeformFrame,
}

impl NetworkModelInstance {
    /// Builds the instance from the provider's flat model; `None` when there
    /// is no model (the piece then renders by [`PieceRender::Fallback`]).
    /// When the game creates it is not published (question NO23).
    pub fn create(provider: &dyn ModelProvider, key: u64) -> Option<NetworkModelInstance> {
        let flat = provider.flat_model(key)?;
        Some(NetworkModelInstance {
            key,
            verts: flat.clone(),
            flat,
            corners: None,
            frame: DeformFrame::default(),
        })
    }

    /// `Deform`: bend the flat model to `corners`. Each vertex's y is its
    /// flat y plus the interpolated corner height at its x/z, so a flat
    /// (y = 0) model ends with y equal to the interpolation (test 24).
    /// Adding to the flat y rather than replacing it, and starting again
    /// from the flat model on every call, are choices (question NO21 b).
    pub fn deform(&mut self, corners: CornerHeights) {
        self.verts.clear();
        for v in &self.flat {
            let mut d = *v;
            d.pos[1] = v.pos[1] + corner_height(&corners, &self.frame, v.pos[0], v.pos[2]);
            self.verts.push(d);
        }
        self.corners = Some(corners);
    }

    /// `FlipAndLight` through the renderer, with the flip of `+0x104`.
    pub fn flip_and_light(&mut self, renderer: &mut dyn ModelRenderer, orientation_byte: u8) {
        renderer.flip_and_light(&mut self.verts, flip_bit(orientation_byte));
    }

    /// `Clip` through the renderer.
    pub fn clip(&mut self, renderer: &mut dyn ModelRenderer) {
        renderer.clip(&mut self.verts);
    }

    /// `Prelight` through the renderer.
    pub fn prelight(&mut self, renderer: &mut dyn ModelRenderer) {
        renderer.prelight(&mut self.verts);
    }

    /// The rendered vertices.
    pub fn rendered(&self) -> &[Vertex] {
        &self.verts
    }
}

/// What a piece renders from (§10's last sentence): the deformed model when
/// there is one, otherwise the §2.2 quad and triangle soup.
#[derive(Clone, Copy, Debug)]
pub enum PieceRender<'a> {
    Model(&'a NetworkModelInstance),
    Fallback {
        quad: &'a [Vertex; 4],
        triangles: &'a [Triangle],
    },
}

impl<'a> PieceRender<'a> {
    pub fn select(
        instance: Option<&'a NetworkModelInstance>,
        quad: &'a [Vertex; 4],
        triangles: &'a [Triangle],
    ) -> PieceRender<'a> {
        match instance {
            Some(m) => PieceRender::Model(m),
            None => PieceRender::Fallback { quad, triangles },
        }
    }

    /// `GetModelInstances` (slot 33): the instance, or none on the fallback
    /// path (what the slot returns with no model is question NO23 c).
    pub fn model_instance(&self) -> Option<&'a NetworkModelInstance> {
        match *self {
            PieceRender::Model(m) => Some(m),
            PieceRender::Fallback { .. } => None,
        }
    }
}
