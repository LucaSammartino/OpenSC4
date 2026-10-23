// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::{mesh::Vertex, TextureBinding};

/// Signed cell-grid contact box; inclusive overlap, including edges/corners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect { pub min_x: i32, pub min_z: i32, pub max_x: i32, pub max_z: i32 }
impl Rect {
    pub const DIRTY_EMPTY: Self = Self { min_x: i32::MAX, min_z: i32::MAX, max_x: i32::MIN, max_z: i32::MIN };
    pub const SHORE_EMPTY: Self = Self { min_x: i32::MAX, min_z: i32::MAX, max_x: i32::MAX, max_z: i32::MAX };
    pub fn union(self, b: Self) -> Self {
        Self { min_x: self.min_x.min(b.min_x), min_z: self.min_z.min(b.min_z), max_x: self.max_x.max(b.max_x), max_z: self.max_z.max(b.max_z) }
    }
    pub fn overlaps(self, b: Self) -> bool {
        self.min_x <= b.max_x && b.min_x <= self.max_x && self.min_z <= b.max_z && b.min_z <= self.max_z
    }
    pub fn area(self) -> i32 {
        self.max_x.wrapping_sub(self.min_x).wrapping_mul(self.max_z.wrapping_sub(self.min_z))
    }
    pub fn from_packed(x: u32, z: u32) -> Self {
        Self { min_x: (x >> 16) as i32, max_x: (x & 65535) as i32, min_z: (z >> 16) as i32, max_z: (z & 65535) as i32 }
    }
    pub fn render_box(self, water: f32) -> RenderBox {
        RenderBox { min: [self.min_x as f32 * 16.0, water, self.min_z as f32 * 16.0], max: [self.max_x as f32 * 16.0, water, self.max_z as f32 * 16.0] }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderBox { pub min: [f32; 3], pub max: [f32; 3] }

pub const RIPPLE_STATE: [u32; 14] = [1, 0, 1, 0, 1, 1, 1, 3, 4, 5, u32::MAX, 0xBF80_0000, 1, u32::MAX];

/// All operations are foreign renderer primitives; geometry and ordering stay owned.
pub trait Renderer {
    fn visible(&mut self, bounds: RenderBox) -> bool;
    fn pack_state(&mut self, arguments: [u32; 14]) -> u32;
    fn set_state(&mut self, state: u32);
    fn default_model_transform(&mut self);
    fn bind_frame(&mut self, binding: &dyn TextureBinding, frame: u32, unit: u32);
    fn submit(&mut self, primitive: u32, format: u32, vertices: &[Vertex]);
}

pub fn contact_rectangles(vertices: &[Vertex]) -> Vec<Rect> {
    vertices.chunks(20).map(|chunk| {
        let mut min_x = f32::INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_z = f32::NEG_INFINITY;
        for v in chunk {
            min_x = min_x.min(v.position[0]); min_z = min_z.min(v.position[2]);
            max_x = max_x.max(v.position[0]); max_z = max_z.max(v.position[2]);
        }
        // Literal floor/ceil in the normal representable conversion domain.
        Rect { min_x: (min_x / 16.0).floor() as i32, min_z: (min_z / 16.0).floor() as i32,
            max_x: (max_x / 16.0).ceil() as i32, max_z: (max_z / 16.0).ceil() as i32 }
    }).collect()
}
