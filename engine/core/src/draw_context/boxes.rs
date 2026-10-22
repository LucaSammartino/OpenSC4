// SPDX-License-Identifier: GPL-3.0-or-later
//! geometry-utilities.md 6.1-6.2, draw-context.md 9.3.
use super::*;

pub fn bounding_box_vertices(bounds: Bounds, color: Rgba) -> Vec<LitVertex> {
    if bounds.is_empty() {
        return Vec::new();
    }
    let corners: [[f32; 3]; 8] = std::array::from_fn(|corner| {
        std::array::from_fn(|axis| {
            if corner & (1 << axis) == 0 {
                bounds.min[axis]
            } else {
                bounds.max[axis]
            }
        })
    });
    let faces = [
        ([2, 3, 1, 0], -1.0),
        ([4, 5, 7, 6], 1.0),
        ([0, 1, 5, 4], -0.5),
        ([1, 3, 7, 5], 0.25),
        ([3, 2, 6, 7], 0.5),
        ([2, 0, 4, 6], -0.25),
    ];
    let channel = |value: f32| {
        let c = crate::x87::rhe(256.0 * value);
        c.wrapping_sub(c >> 8) as u8
    };
    let mut vertices = Vec::with_capacity(24);
    for (indices, dot) in faces {
        let k = 0.3 + 0.7 * dot;
        let bgra = [
            channel(k * color[2]),
            channel(k * color[1]),
            channel(k * color[0]),
            channel(color[3]),
        ];
        for index in indices {
            vertices.push(LitVertex {
                position: corners[index],
                bgra,
            });
        }
    }
    vertices
}

impl<D: Driver> DrawContext<D> {
    pub fn draw_bounding_box(&mut self, bounds: Bounds, color: Rgba) -> Result<(), Error> {
        self.ready()?;
        let vertices = bounding_box_vertices(bounds, color);
        if vertices.is_empty() {
            return Ok(());
        }
        self.set_render_state(BOX_STATE)?;
        self.emit(Command::InterleavedOwned {
            format: 1,
            stride: 0,
            vertices,
        })?;
        self.draw_arrays(6, 0, 24)
    }
}
