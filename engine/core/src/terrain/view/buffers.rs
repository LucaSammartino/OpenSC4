// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use crate::map_view::ring_float::F;
use crate::map_view::RingContext;

/// The edge array is independent of the cell flags and the terrain's row grid J.
/// terrain.md §15.5.15.5. The count argument remains 6(W-1) for every edge.
pub fn edge_triangle_indices(verts_x: u32) -> Vec<u16> {
    (0..verts_x.saturating_sub(1))
        .flat_map(|i| {
            let a = i as u16;
            let b = a.wrapping_add(verts_x as u16);
            [a, b, b.wrapping_add(1), a, b.wrapping_add(1), a.wrapping_add(1)]
        })
        .collect()
}

/// Format 11's complete 32-byte record, kept in little-endian draw-time form.
/// The caller supplies height/colour producer results; this does not invent
/// resource-derived water colours or rebuild the edge strips (§15.5.15.8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewVertex(pub [u8; 32]);

impl ViewVertex {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        let mut vertex = Self([0; 32]);
        vertex.set_float(0, x);
        vertex.set_float(4, y);
        vertex.set_float(8, z);
        vertex.0[15] = 255;
        vertex
    }

    /// The source's consecutive RGB bytes become BGR; alpha is untouched,
    /// including on a floor pass following the textured pass.
    pub fn set_rgb(&mut self, rgb: [u8; 3]) {
        self.0[12..15].copy_from_slice(&[rgb[2], rgb[1], rgb[0]]);
    }

    /// Three terrain layers in stored order; zero terminates the search.
    /// The low byte of the matching weight is stored, not a normalized float.
    pub fn set_layer_alpha(&mut self, key: u32, layers: [(u32, u16); 3]) {
        self.0[15] = layers
            .into_iter()
            .take_while(|&(id, _)| id != 0)
            .find_map(|(id, weight)| (id == key).then_some(weight as u8))
            .unwrap_or(0);
    }

    fn float(&self, offset: usize) -> f32 {
        f32::from_le_bytes(self.0[offset..offset + 4].try_into().unwrap())
    }

    fn set_float(&mut self, offset: usize, value: f32) {
        self.0[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}

/// UV refresh has a zoom/rotation key, not a tuning-factor key (§15.5.15.5).
/// A context carries PC/RC explicitly; the native host must supply its actual
/// numeric controls rather than assume the qualified vector's 0x037F.
#[derive(Clone, Debug, Default)]
pub struct TextureCoordinateCache {
    key: Option<(u32, u32)>,
    factor: f32,
}

impl TextureCoordinateCache {
    pub fn factor(&self) -> f32 {
        self.factor
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        context: RingContext,
        zoom: u32,
        rotation: u32,
        factor_operands: [f32; 2],
        size: [f32; 2],
        land: &mut [ViewVertex],
        water: &mut [ViewVertex],
    ) -> bool {
        if self.key == Some((zoom, rotation)) {
            return false;
        }
        // SPEC-GAP: reject nonfinite producer inputs atomically; only finite arithmetic is specified here.
        if !factor_operands.into_iter().chain(size).all(f32::is_finite)
            || !land
                .iter()
                .chain(water.iter())
                .all(|v| v.float(0).is_finite() && v.float(8).is_finite())
        {
            return false;
        }
        let [a, b] = factor_operands.map(F::float);
        let factor = context.store(context.mul(a, b)).stored_float();
        if !factor.is_finite() {
            return false;
        }
        self.key = Some((zoom, rotation));
        self.factor = factor;
        // An invalid rotation still changes both keys and the stored factor.
        if rotation > 3 {
            return true;
        }
        let [size_x, size_z] = size.map(F::float);
        let f = F::float(self.factor);
        for vertex in land.iter_mut().chain(water) {
            let (x, z) = (F::float(vertex.float(0)), F::float(vertex.float(8)));
            let (u, v) = match rotation {
                0 => (context.mul(f, x), context.mul(f, z)),
                1 => (context.mul(f, z), context.mul(context.sub(size_x, x), f)),
                2 => (
                    context.mul(context.sub(size_x, x), f),
                    context.mul(context.sub(size_z, z), f),
                ),
                _ => (context.mul(context.sub(size_z, z), f), context.mul(f, x)),
            };
            vertex.set_float(16, context.store(u).stored_float());
            vertex.set_float(20, context.store(v).stored_float());
        }
        true
    }
}
