//! Surface rendering boundaries specified by §§2.5, 2.7 and 2.11 (NO32).
use super::record::{Triangle, Vertex};

/// Packed value/mask pairs; no stage-1 state is changed by these states.
pub const RENDER_STATES: [(u32, u32); 3] = [
    (0x024A_91FF, 0x1FFF_FFFF),
    (0x0200_01E7, 0x1E00_03FF),
    (0x0242_317F, 0x1FFF_FFFF),
];

pub fn texture_id(id: u32, zoom: u32) -> u32 {
    if id & 15 == 0 {
        id.wrapping_add(zoom)
    } else {
        id
    }
}

/// Rotation is a byte: bit 7 selects the mirrored UV permutations.
pub fn texture_coordinates(rotation: u8) -> [[f32; 2]; 4] {
    let u = [0.0, 0.0, 1.0, 1.0];
    let v = [0.0, 1.0, 1.0, 0.0];
    let r = usize::from(rotation & 3);
    let mut uv = std::array::from_fn(|k| [u[(r + k) & 3], v[(r + k) & 3]]);
    if rotation & 0x80 != 0 {
        let component = if r & 1 != 0 { 1 } else { 0 };
        for (a, b) in [(0, 3), (1, 2)] {
            let old = uv[a][component];
            uv[a][component] = uv[b][component];
            uv[b][component] = old;
        }
    }
    uv
}

/// HasAnyNetworkFlag selects the path; each half then has its own gate.
pub fn surface_triangles(flags: u32) -> Vec<[usize; 3]> {
    let indices = if flags & 0x0100_0000 == 0 {
        [[0, 1, 2], [0, 2, 3]]
    } else {
        [[0, 1, 3], [3, 1, 2]]
    };
    indices
        .into_iter()
        .enumerate()
        .filter_map(|(i, tri)| (flags & (0x0200_0000 << i) != 0).then_some(tri))
        .collect()
}

/// Normalize the external geometry helper's unnormalized normal. No fallback
/// is inserted for zero, unordered or infinite normals (masked exceptions).
pub fn normalize(normal: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = normal.map(f64::from);
    let reciprocal = 1.0 / ((z * z + y * y) + x * x).sqrt();
    normal.map(|v| (f64::from(v) * reciprocal) as f32)
}

/// Foreign geometry and lighting providers, not cached native owner results.
pub trait PartLighting {
    fn triangle_normal(&mut self, positions: [[f32; 3]; 3]) -> [f32; 3];
    fn rgb(&mut self, position: [f32; 3], normal: [f32; 3]) -> [u8; 3];
}

/// §2.7: one query per triangle, copy provider RGB, preserve each alpha.
pub fn update_part_lighting(parts: &mut [Triangle], flags: &mut u32, host: &mut impl PartLighting) {
    for triangle in parts {
        let normal = normalize(host.triangle_normal(triangle.verts.map(|v| v.pos)));
        let [r, g, b] = host.rgb(triangle.verts[0].pos, normal);
        let rgb = (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
        for Vertex { color, .. } in &mut triangle.verts {
            *color = (*color & 0xFF00_0000) | rgb;
        }
    }
    *flags |= 0x8000;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_tables_zoom_and_independent_halves() {
        assert_eq!(RENDER_STATES[1], (0x0200_01E7, 0x1E00_03FF));
        assert_eq!(texture_id(0x1234_5000, 3), 0x1234_5003);
        assert_eq!(texture_id(0x1234_5002, 3), 0x1234_5002);
        assert_eq!(texture_id(0xFFFF_FFF0, 16), 0);
        assert_eq!(texture_coordinates(1), [[0., 1.], [1., 1.], [1., 0.], [0., 0.]]);
        assert_eq!(texture_coordinates(0x81), [[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
        assert_eq!(texture_coordinates(0x80), [[1., 0.], [1., 1.], [0., 1.], [0., 0.]]);
        for (flags, expected) in [
            (0, vec![]),
            (0x0200_0000, vec![[0, 1, 2]]),
            (0x0400_0000, vec![[0, 2, 3]]),
            (0x0600_0000, vec![[0, 1, 2], [0, 2, 3]]),
        ] {
            assert_eq!(surface_triangles(flags), expected);
        }
        assert_eq!(surface_triangles(0x0700_0000), [[0, 1, 3], [3, 1, 2]]);
    }

    #[test]
    fn lighting_copies_rgb_and_preserves_alpha_even_for_degenerate_normal() {
        struct Host {
            degenerate: bool,
            queries: usize,
        }
        impl PartLighting for Host {
            fn triangle_normal(&mut self, _: [[f32; 3]; 3]) -> [f32; 3] {
                if self.degenerate {
                    [0.; 3]
                } else {
                    [0., 3., 4.]
                }
            }
            fn rgb(&mut self, position: [f32; 3], normal: [f32; 3]) -> [u8; 3] {
                self.queries += 1;
                assert_eq!(position, [1., 2., 3.]);
                if self.degenerate {
                    assert!(normal.into_iter().all(f32::is_nan));
                } else {
                    assert_eq!(normal, [0., 0.6, 0.8]);
                }
                [17, 34, 51]
            }
        }
        for degenerate in [false, true] {
            let mut host = Host { degenerate, queries: 0 };
            let mut part = Triangle::default();
            for (i, vertex) in part.verts.iter_mut().enumerate() {
                vertex.pos = [1., 2., 3.];
                vertex.color = ((i as u32 + 1) << 24) | 0xABCDEF;
            }
            let mut flags = 0x6000000;
            update_part_lighting(std::slice::from_mut(&mut part), &mut flags, &mut host);
            assert_eq!(host.queries, 1);
            assert_eq!(part.verts.map(|v| v.color), [0x01112233, 0x02112233, 0x03112233]);
            assert_eq!(flags, 0x6008000);
        }
    }
}
