// SPDX-License-Identifier: GPL-3.0-or-later
use super::{ControlHost, Pixel, Vec3};
use crate::view_utilities::ViewUtilities;

pub const LABEL_TYPE: u32 = 0xCB79_919B;
pub const SIGN_TYPE: u32 = 0xAB72_FBB3;
pub const PROP_INTERFACE: u32 = 0xA823_821E;
pub const LABEL_INTERFACE: u32 = 0x2B3B_7D86;
pub const SIGN_INTERFACE: u32 = 0x4B44_FBE2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PropId(pub u64);

/// One complete code-page character; raw bytes remain intact. Input adapters
/// segment strings with a code-page codec, not UTF-8 or byte-count guesses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text(pub Vec<Vec<u8>>);
impl Text {
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn glyph_count(&self) -> usize { self.0.len() }
    pub fn from_single_bytes(bytes: &[u8]) -> Self { Self(bytes.iter().map(|b| vec![*b]).collect()) }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PropInfo {
    pub is_prop: bool,
    pub prop_type: u32,
    pub text_decal: bool,
    pub signpost: bool,
}

/// These predicates are the filter owners, independent of renderer traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropFilter { Label, Sign, SignOrLabel }
impl PropFilter {
    pub fn accepts(self, p: PropInfo) -> bool {
        p.is_prop && match self {
            Self::Label => p.text_decal,
            Self::Sign => p.signpost,
            Self::SignOrLabel => p.prop_type == SIGN_TYPE || p.prop_type == LABEL_TYPE,
        }
    }
}

/// Primitive object/world boundary. Picking traverses renderer/occupant data
/// using the supplied *core-owned* predicate; no native tool/filter method.
pub trait PropWorld: ControlHost {
    fn renderer_available(&self) -> bool;
    /// Renderer slot 60, PickTerrain(...,0,1).
    fn pick_terrain(&mut self, at: Pixel) -> Option<Vec3>;
    fn pick_prop(&mut self, at: Pixel, filter: PropFilter) -> Option<PropId>;
    fn first_prop_at(&mut self, p: Vec3, filter: PropFilter) -> Option<PropId>;
    fn prop_info(&self, prop: PropId) -> PropInfo;
    fn position(&self, prop: PropId) -> Vec3;
    fn set_position(&mut self, prop: PropId, position: Vec3);
    fn text(&self, prop: PropId) -> Text;
    fn set_text(&mut self, prop: PropId, text: &Text);
    fn set_dynamic(&mut self, prop: PropId, on: bool);
    fn set_edit_mode(&mut self, prop: PropId, on: bool);
    fn set_alpha(&mut self, prop: PropId, alpha: f32);
    fn add_city_prop(&mut self, prop_type: u32) -> Option<PropId>;
    fn remove_prop(&mut self, prop: PropId);
    fn show_signs_and_labels(&mut self, show: bool);
}

/// Projection reuses the current production view utility rather than a host's
/// precomputed tool offset or IME answer.
pub fn project(utilities: &ViewUtilities, point: Vec3) -> Vec3 {
    utilities.project_point_to_viewport_coordinates_2d(point)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MouseState { #[default] Idle, Pressed, Dragging, Typing }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key { Delete, Home, End, Escape, V, Other(u32) }
