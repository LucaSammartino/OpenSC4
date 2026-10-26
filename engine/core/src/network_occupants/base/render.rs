//! NO32: static resource lifetime, texture zoom, lighting and ordered Draw.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// Returned handles carry one reference acquired/created by the foreign service.
pub trait NetworkTextureHost: References {
    fn acquire_resource_manager(&mut self) -> RefValue;
    fn acquire_texture_manager(&mut self, service: u32, iid: u32) -> RefValue;
    fn register_static_shutdown(&mut self);
    fn load_texture(&mut self, manager: &RefValue, key: ResKey) -> RefValue;
}

pub struct NetworkStatics {
    pub initialized: bool,
    pub zoom: u32,
    pub intersections: BTreeSet<u32>,
    pub resource_manager: RefValue,
    pub texture_manager: RefValue,
    pub textures: BTreeMap<u32, RefValue>,
}

impl Default for NetworkStatics {
    fn default() -> Self {
        Self {
            initialized: false,
            zoom: 5,
            intersections: BTreeSet::new(),
            resource_manager: RefValue::Null,
            texture_manager: RefValue::Null,
            textures: BTreeMap::new(),
        }
    }
}
impl NetworkStatics {
    pub fn initialize(&mut self, host: &mut impl NetworkTextureHost) {
        if self.initialized {
            return;
        }
        self.resource_manager = host.acquire_resource_manager();
        self.texture_manager = host.acquire_texture_manager(0x1AC0_E11A, 0xFAC0_E219);
        host.register_static_shutdown();
        self.intersections = [
            0x0400_3500,
            0x0400_3600,
            0x0400_5200,
            0x0400_5300,
            0x0400_5400,
            0x0400_6900,
            0x0400_7100,
            0x0400_7400,
            0x0400_7500,
            0x0400_8500,
            0x0400_9000,
            0x0400_9300,
            0x0400_9400,
        ]
        .into_iter()
        .collect();
        self.initialized = true;
    }
    fn clear_textures(&mut self, refs: &mut impl References) {
        for (_, mut binding) in std::mem::take(&mut self.textures) {
            clear_ref(&mut binding, refs);
        }
    }
    pub fn set_texture_zoom(&mut self, zoom: u32, refs: &mut impl References) {
        self.zoom = zoom;
        self.clear_textures(refs);
    }
    pub fn shutdown(&mut self, refs: &mut impl References) {
        self.clear_textures(refs);
        clear_ref(&mut self.texture_manager, refs);
        clear_ref(&mut self.resource_manager, refs);
        self.initialized = false;
    }
    fn lookup(&mut self, id: u32, host: &mut impl NetworkTextureHost) -> RefValue {
        let key = surface::texture_id(id, self.zoom);
        self.textures
            .entry(key)
            .or_insert_with(|| {
                host.load_texture(
                    &self.texture_manager,
                    ResKey {
                        type_id: 0x7AB5_0E44,
                        group: 0x1ABE_787D,
                        instance: key,
                    },
                )
            })
            .clone()
    }
    /// Returned binding is borrowed from the cache: cloning the token does not
    /// acquire another native reference and callers must not release it.
    pub fn texture(&mut self, id: u32, fallback: u32, host: &mut impl NetworkTextureHost) -> RefValue {
        let binding = self.lookup(id, host);
        if matches!(binding, RefValue::Null) && fallback != 0 && fallback != id {
            self.lookup(fallback, host)
        } else {
            binding
        }
    }
}

pub trait NetworkDrawHost: NetworkTextureHost + surface::PartLighting {
    fn corner_rgb(&mut self, position: [f32; 3]) -> [u8; 3];
    fn set_state(&mut self, value_mask: (u32, u32));
    fn set_model_transform(&mut self, transform: Xform);
    fn set_shade(&mut self, shade: Option<[f32; 3]>);
    fn bind_texture(&mut self, binding: &RefValue);
    fn wrap(&mut self, modes: [u32; 3]);
    fn filtering(&mut self, modes: [u32; 3]);
    fn vertices(&mut self, vertices: &[Vertex], format: u32);
    fn triangle(&mut self, indices: [usize; 3]);
    fn water_visible(&mut self) -> bool;
    fn water_level(&mut self, x: f32, z: f32) -> f32;
}

impl NetworkOccupant {
    pub fn update_lighting(&mut self, host: &mut impl NetworkDrawHost) {
        for v in &mut self.quad {
            let [r, g, b] = host.corner_rgb(v.pos);
            v.color = (v.color & 0xFF00_0000) | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
        }
        surface::update_part_lighting(&mut self.parts, &mut self.piece.network_flags, host);
    }
    fn draw_state(&self) -> (u32, u32) {
        let flags = self.piece.network_flags;
        if (flags & 0x0008_0000 != 0 || (flags & 0x4000_0000 != 0 && self.quad[0].alpha() == 255))
            && ![
                0x0301_0000,
                0x0003_0100,
                0x0000_0301,
                0x0100_0003,
                0x0100_0000,
                0x0001_0000,
                0x0000_0100,
                1,
                0x0300_0000,
                0x0003_0000,
                0x0000_0300,
                3,
            ]
            .contains(&self.edges.all_edges())
        {
            surface::RENDER_STATES[1]
        } else {
            surface::RENDER_STATES[0]
        }
    }
    fn draw_quad(&self, host: &mut impl NetworkDrawHost) {
        for triangle in surface::surface_triangles(self.piece.network_flags) {
            host.triangle(triangle);
        }
    }
    fn quad_modes(host: &mut impl NetworkDrawHost) {
        host.wrap([2, 2, 0]);
        host.filtering([1, 1, 0]);
    }
    pub fn draw(
        &mut self,
        zoom: u32,
        self_lit: bool,
        statics: &mut NetworkStatics,
        host: &mut impl NetworkDrawHost,
    ) -> u32 {
        if zoom != statics.zoom {
            statics.set_texture_zoom(zoom, host);
        }
        if self.piece.network_flags & FLAG_LIT_COMPUTED == 0 {
            self.update_lighting(host);
        }
        let state = self.draw_state();
        host.set_state(state);
        host.set_model_transform(self.transform);
        if self_lit {
            if self.base.highlight == 0 {
                return 1;
            }
            host.set_shade(Some([1.0, 0.94, 0.55]));
        } else {
            host.set_shade(None);
        }

        if self.piece.network_flags & 0x0600_0000 != 0 {
            let under = if self.piece.variation != 0 {
                statics.texture(self.piece.variation, 0, host)
            } else {
                RefValue::Null
            };
            let id = self.piece_id.wrapping_add(16 * u32::from(self.piece.rotation));
            let texture = statics.texture(id, self.piece_id, host);
            if !matches!(texture, RefValue::Null) {
                Self::quad_modes(host);
                host.vertices(&self.quad, 10);
                for binding in [&under, &texture] {
                    if matches!(binding, RefValue::Null) {
                        continue;
                    }
                    host.bind_texture(binding);
                    self.draw_quad(host);
                    if self_lit {
                        host.set_state(surface::RENDER_STATES[2]);
                        Self::quad_modes(host);
                        self.draw_quad(host);
                        host.set_state(state);
                    }
                }
                let directions = self.one_way_directions();
                if directions != 0 {
                    self.draw_one_way_arrow(self_lit, state, directions, statics, host);
                }
            }
        }
        host.wrap([3, 3, 0]);
        host.filtering([1, 1, 0]);
        let water = host.water_visible();
        for part in &self.parts {
            let [x, y, z] = part.verts[0].pos;
            if water && y < host.water_level(x, z) {
                continue;
            }
            let texture = statics.texture(part.word0, 0, host);
            if matches!(texture, RefValue::Null) {
                continue;
            }
            host.bind_texture(&texture);
            host.vertices(&part.verts, 10);
            host.triangle([0, 1, 2]);
            if self_lit {
                host.set_state(surface::RENDER_STATES[2]);
                host.triangle([0, 1, 2]);
            }
        }
        if self_lit {
            host.set_shade(None);
        }
        1
    }

    pub fn draw_one_way_arrow(
        &mut self,
        self_lit: bool,
        state: (u32, u32),
        mut directions: u8,
        statics: &mut NetworkStatics,
        host: &mut impl NetworkDrawHost,
    ) {
        let e = self.edges.edge_types(10);
        if e & !0x0303_0303 != 0 {
            return;
        }
        let m = self.edges.active_edges_no_group(10);
        let rotations = match m {
            3 => [0x83, 2],
            5 => [1, 3],
            6 => [3, 0x82],
            9 => [1, 0x80],
            10 => [2, 0],
            12 => [0x81, 0],
            _ => return,
        };
        if ![
            0x0002_0002,
            0x0200_0200,
            0x0100_0003,
            0x0000_0301,
            0x0003_0100,
            0x0301_0000,
        ]
        .contains(&e)
        {
            return;
        }
        let curve = matches!(m, 3 | 6 | 9 | 12);
        let id = if curve { 0x0BE3_5EC0 } else { 0x0BE3_5EB0 };
        if curve {
            for shift in [2, 4] {
                let field = (directions >> shift) & 3;
                let swapped = ((field & 1) << 1) | ((field & 2) >> 1);
                directions = (directions & !(3 << shift)) | (swapped << shift);
            }
        }
        let sum: i32 = (0..4)
            .map(|i| match (directions >> (i * 2)) & 3 {
                1 => 1,
                2 => -1,
                _ => 0,
            })
            .sum();
        let rotation = rotations[usize::from(sum < 0)];
        let binding = statics.texture(id, 0, host);
        if matches!(binding, RefValue::Null) {
            return;
        }
        host.set_state(surface::RENDER_STATES[0]);
        host.bind_texture(&binding);
        self.set_texture_coordinates(rotation);
        host.vertices(&self.quad, 10);
        self.draw_quad(host);
        host.set_state(surface::RENDER_STATES[2]);
        Self::quad_modes(host);
        if self_lit {
            self.draw_quad(host);
        }
        host.set_state(state);
        host.set_state(state);
        self.set_texture_coordinates(self.piece.orientation);
    }
}
