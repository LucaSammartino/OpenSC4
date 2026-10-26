//! The published base-owner contract, network-occupants §§2.1–2.11.
//! Foreign services supply resources, lighting and geometric ray tests, never
//! a native network occupant's geometry, draw list or cached owner result.
use super::{
    identity::NetOccClass,
    model::Bounds,
    piece::{NetworkPiece, PieceHost, FLAG_LIT_COMPUTED, FLAG_PATHS_BUILT, LIFECYCLE_INIT},
    pylon::{clear_ref, References},
    record::{CellInfo, NetworkBaseRecord, Triangle, Vertex},
    surface,
    xform::Xform,
};
use crate::{
    automata::OccupantBase,
    stream::{RefValue, ResKey},
    variant::{mem, Elem, Variant},
};

mod render;
pub use render::{NetworkDrawHost, NetworkStatics, NetworkTextureHost};
mod paths;
pub use paths::NetworkPathHost;

/// Base services, with an explicit failure result for cSC4Occupant::Init.
pub trait NetworkHost: PieceHost + References {
    fn initialize_occupant(&mut self) -> bool;
    /// PostOccupantMessage(ChangeOccupant, 0), distinct from the hand-built
    /// SetVariation/SetUnderTexture message exposed by PieceHost.
    fn post_translucency_change(&mut self);
    /// The geometry utility's RayIntersectsBoundingBox, not occupant Pick.
    fn ray_box_distance(&mut self, bounds: Bounds) -> Option<f32>;
}

/// A live base occupant. NetworkPiece supplies the existing flag, path-byte,
/// orientation and change-message algorithms; these fields complete its owner.
pub struct NetworkOccupant {
    pub piece: NetworkPiece,
    pub base: OccupantBase,
    pub transform: Xform,
    pub quad: [Vertex; 4],
    pub parts: Vec<Triangle>,
    pub edges: CellInfo,
    pub bounds: Bounds,
    pub piece_id: u32,
    pub draw_attributes: u8,
    pub opacity: f32,
    pub path_info: RefValue,
    pub path_info_key: ResKey,
    pub path_origin: [f32; 2],
}

impl Default for NetworkOccupant {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkOccupant {
    pub fn new() -> Self {
        Self {
            piece: NetworkPiece::new(NetOccClass::Network),
            base: OccupantBase::new(0xC772_BF98),
            transform: Xform::default(),
            quad: [Vertex::default(); 4],
            parts: Vec::new(),
            edges: CellInfo::default(),
            bounds: Bounds {
                min: [0.0; 3],
                max: [0.0; 3],
            },
            piece_id: 0,
            draw_attributes: 0,
            opacity: 1.0,
            path_info: RefValue::Null,
            path_info_key: ResKey::default(),
            path_origin: [0.0; 2],
        }
    }

    /// Main Init(corners) and occupant Init. A failing base Init leaves the
    /// corners/box alone. StaticInit is called even on a repeated Init.
    pub fn init(
        &mut self,
        corners: [[f32; 3]; 4],
        statics: &mut NetworkStatics,
        host: &mut (impl NetworkHost + NetworkTextureHost),
    ) -> bool {
        statics.initialize(host);
        if !self.piece.initialised {
            self.piece.clear_network_flag(FLAG_PATHS_BUILT, host);
            self.piece.lifecycle = LIFECYCLE_INIT;
            if self.check_for_retrofit() {
                host.add_to_retrofit_list();
            }
            if !host.initialize_occupant() {
                return false;
            }
            self.piece.initialised = true;
        } else {
            for flag in [0x1000_0000, 0x0080_0000] {
                if self.piece.network_flags & flag != 0 {
                    self.piece.network_flags &= !flag;
                    self.piece.set_network_flag(flag, host);
                }
            }
        }
        for (v, pos) in self.quad.iter_mut().zip(corners) {
            v.pos = pos;
        }
        self.compute_box();
        true
    }

    pub fn shutdown(&mut self, host: &mut impl NetworkHost) {
        // §2.5: Shutdown delegates only. The owned cache is destroyed separately.
        host.occupant_shutdown();
    }

    pub fn destroy(&mut self, refs: &mut impl References) {
        self.release_path_info(refs);
        self.parts.clear();
    }

    pub fn release_path_info(&mut self, refs: &mut impl References) {
        clear_ref(&mut self.path_info, refs);
        self.path_info_key = ResKey::default();
    }

    /// Installs a resource returned with one acquired/created reference.
    /// GetPathInfo returns a borrow of it, never a second reference (§2.6/TG24).
    pub fn cache_path_info(&mut self, acquired: RefValue, refs: &mut impl References) {
        self.release_path_info(refs);
        self.path_info = acquired;
    }
    pub fn path_info(&self) -> &RefValue {
        &self.path_info
    }

    pub fn set_quad_draw(&mut self, piece: u32, rotation: u8, refs: &mut impl References) {
        if self.piece.network_flags & 0x0020_0000 == 0 {
            self.piece.network_flags |= 0x0600_0000;
        }
        self.release_path_info(refs);
        self.piece.network_flags &= !FLAG_PATHS_BUILT;
        self.piece_id = piece;
        self.piece.orientation = rotation;
        self.set_texture_coordinates(rotation);
        for v in &mut self.quad {
            v.force_alpha();
        }
    }

    pub fn set_texture_coordinates(&mut self, rotation: u8) {
        for (v, [u, w]) in self.quad.iter_mut().zip(surface::texture_coordinates(rotation)) {
            v.u = u;
            v.v = w;
        }
    }

    pub fn add_part(&mut self, positions: &[[f32; 3]], uv: &[[f32; 2]], indices: [usize; 3], texture: u32) {
        self.parts.push(Triangle {
            word0: texture,
            verts: indices.map(|i| Vertex {
                pos: positions[i],
                color: u32::MAX,
                u: uv[i][0],
                v: uv[i][1],
            }),
        });
        self.compute_box();
        self.piece.network_flags &= !FLAG_LIT_COMPUTED;
    }

    pub fn position(&self) -> [f32; 3] {
        let mut p = [0.0, -f32::MAX, 0.0];
        for v in &self.quad {
            p[0] += v.pos[0];
            p[2] += v.pos[2];
            if v.pos[1] > p[1] {
                p[1] = v.pos[1];
            }
        }
        p[0] *= 0.25;
        p[2] *= 0.25;
        p
    }

    pub fn compute_box(&mut self) -> Bounds {
        let p = self.position();
        self.piece.position = p;
        let mut bounds = Bounds { min: p, max: p };
        for v in self.quad.iter().chain(self.parts.iter().flat_map(|t| t.verts.iter())) {
            for i in 0..3 {
                if v.pos[i] < bounds.min[i] {
                    bounds.min[i] = v.pos[i];
                }
                if v.pos[i] > bounds.max[i] {
                    bounds.max[i] = v.pos[i];
                }
            }
        }
        self.bounds = bounds;
        bounds
    }

    pub fn vertex_heights(&self) -> [f32; 4] {
        self.quad.map(|v| v.pos[1])
    }
    pub fn set_vertex_heights(&mut self, heights: [f32; 4]) {
        for (v, y) in self.quad.iter_mut().zip(heights) {
            v.pos[1] = y;
        }
        // Deliberately no box recomputation (§2.4 slots 34–36).
    }
    pub fn occupied_cell(&self) -> (i32, i32) {
        (
            super::world_to_cell(self.quad[0].pos[0]),
            super::world_to_cell(self.quad[0].pos[2]),
        )
    }
    pub fn altitude(&self, x: f32, z: f32) -> f32 {
        let [a, b, c, d] = self.vertex_heights();
        let dx = x - self.quad[0].pos[0];
        let dz = z - self.quad[0].pos[2];
        if dx <= dz {
            a + (dx * (c - b) + dz * (b - a)) * 0.0625
        } else {
            a + (dx * (d - a) + dz * (c - d)) * 0.0625
        }
    }
    pub fn normal(&self, x: f32, z: f32) -> [f32; 3] {
        let [a, b, c, d] = self.vertex_heights();
        if z - self.quad[0].pos[2] > x - self.quad[0].pos[0] {
            [a - d, 16.0, a - b]
        } else {
            [b - c, 16.0, d - c]
        }
    }
    pub fn pick(&self, best: &mut f32, host: &mut impl NetworkHost) -> bool {
        let mut bounds = self.bounds;
        bounds.max[1] += 0.1;
        if let Some(d) = host.ray_box_distance(bounds) {
            if d <= *best {
                *best = d;
                return true;
            }
        }
        false
    }
    pub fn set_transform(&mut self, transform: Xform) {
        let counter = self.transform.counter.wrapping_add(1);
        self.transform = transform;
        self.transform.counter = counter;
    }
    pub fn world_bounds(&self) -> Bounds {
        self.bounds
    }
    /// The bounds in the instance's frame (§2.1), obtained by inverse-transforming
    /// all eight world-box corners. S3D rotation is orthonormal, with uniform scale.
    pub fn local_bounds(&self) -> Bounds {
        let mut out = Bounds {
            min: [f32::MAX; 3],
            max: [-f32::MAX; 3],
        };
        for bits in 0..8 {
            let point: [f32; 3] = std::array::from_fn(|i| {
                (if bits & (1 << i) == 0 {
                    self.bounds.min[i]
                } else {
                    self.bounds.max[i]
                } - self.transform.translation[i])
                    / self.transform.scale
            });
            for i in 0..3 {
                let v = (0..3).map(|j| point[j] * self.transform.matrix[i * 3 + j]).sum::<f32>();
                if v < out.min[i] {
                    out.min[i] = v;
                }
                if v > out.max[i] {
                    out.max[i] = v;
                }
            }
        }
        out
    }
    pub fn get_model_instance(&mut self, refs: &mut impl References, own_model_view: &RefValue) -> Xform {
        self.transform.translation = self.position();
        self.transform.mask |= 1;
        self.transform.counter = self.transform.counter.wrapping_add(1);
        refs.add_ref(own_model_view);
        self.transform
    }
    pub fn sort_value(&self) -> u32 {
        if self.piece.network_flags & 0x0600_0000 != 0 {
            self.piece_id
        } else {
            self.parts.first().map_or(0xA821_EF94, |t| t.word0)
        }
    }
    pub fn set_translucency(&mut self, on: bool, alpha: u8, host: &mut impl NetworkHost) {
        if on {
            self.draw_attributes |= 2;
        } else {
            self.draw_attributes &= !2;
        }
        for v in &mut self.quad {
            v.color = (v.color & 0x00FF_FFFF) | (u32::from(if on { alpha } else { 255 }) << 24);
        }
        self.opacity = f32::from(self.quad[0].alpha()) / 255.0;
        host.post_translucency_change();
    }
    pub fn set_network_flag(&mut self, flags: u32, host: &mut impl NetworkHost) {
        // Handle primary slot 58 in this owner rather than forwarding it to
        // another network occupant through the prefix helper's callback.
        self.piece.set_network_flag(flags & !0x4000, host);
        self.piece.network_flags |= flags & 0x4000;
        if flags & 0x4000 != 0 {
            self.set_translucency(true, 0x80, host);
        }
    }
    /// Base CanBeLit is false (§2.7), so SetLit cannot toggle or post.
    pub fn set_lit(&mut self, _on: bool) {}
    pub fn is_lit(&self) -> bool {
        self.piece.network_flags & 0x2000_0000 != 0
    }
    pub fn is_intersection(&self, statics: &NetworkStatics) -> bool {
        self.piece.is_of_type(6) && statics.intersections.contains(&self.piece_id)
    }

    fn scalar(&self, id: u32) -> Option<Elem> {
        match self.base.holder.get(id) {
            Some(Variant::Scalar(v)) => Some(*v),
            _ => None,
        }
    }
    fn set_scalar(&mut self, id: u32, value: Elem) {
        self.base.holder.own.add(id, Variant::Scalar(value));
    }
    pub fn has_property(&self, id: u32) -> bool {
        id == 0x8869_6402 || self.base.holder.has(id)
    }
    pub fn check_for_retrofit(&self) -> bool {
        self.has_property(0x4835_C6AC)
    }
    pub fn set_retrofit(&mut self, on: bool) {
        if on {
            self.base.holder.own.add(0x4835_C6AC, Variant::Empty);
        } else {
            self.base.holder.own.remove(0x4835_C6AC);
        }
    }
    pub fn connection_node_edges(&self) -> Option<[u32; 2]> {
        match self.base.holder.get(0x4835_C6A7) {
            Some(Variant::Array { items, .. }) if items.len() >= 2 => {
                Some([items[0].as_i64() as u32, items[1].as_i64() as u32])
            }
            _ => None,
        }
    }
    pub fn set_connection_node_edges(&mut self, edges: [u32; 2]) {
        self.base.holder.own.add(
            0x4835_C6A7,
            Variant::Array {
                elem: mem::UINT32,
                items: edges.map(Elem::U32).to_vec(),
            },
        );
    }
    pub fn connection_path_override(&self) -> u32 {
        self.scalar(0x4835_C6A9).map_or(0, |v| v.as_i64() as u32)
    }
    pub fn set_connection_path_override(&mut self, id: u32, refs: &mut impl References) {
        if id == 0 {
            self.base.holder.own.remove(0x4835_C6A9);
        } else {
            self.set_scalar(0x4835_C6A9, Elem::U32(id));
        }
        self.piece.path_id_override = id;
        self.release_path_info(refs);
        self.piece.network_flags &= !FLAG_PATHS_BUILT;
    }
    pub fn passage_ceiling(&self) -> Option<f32> {
        self.scalar(0x4835_C6A8).map(|v| v.as_f64() as f32)
    }
    pub fn set_passage_ceiling(&mut self, value: f32) {
        self.set_scalar(0x4835_C6A8, Elem::F32(value));
    }
    pub fn one_way_directions(&self) -> u8 {
        self.scalar(0x4835_C6AA).map_or(0, |v| v.as_i64() as u8)
    }
    pub fn set_one_way_directions(&mut self, value: u8) {
        if value != 0 || self.has_property(0x4835_C6AA) {
            self.set_scalar(0x4835_C6AA, Elem::U8(value));
        }
    }
    pub fn complex_origin(&self) -> Option<(u32, u32)> {
        self.scalar(0x4835_C6AB).map(|v| {
            let v = v.as_i64() as u32;
            (v & 0xFFFF, v >> 16)
        })
    }
    pub fn set_complex_origin(&mut self, x: u32, z: u32) {
        self.set_scalar(0x4835_C6AB, Elem::U32((z << 16).wrapping_add(x)));
    }
    pub fn same_structure(&self, other: &Self) -> bool {
        self.complex_origin()
            .is_some_and(|origin| other.complex_origin() == Some(origin))
    }

    pub fn record(&self) -> NetworkBaseRecord {
        NetworkBaseRecord {
            base: self.base.clone(),
            transform: self.transform,
            quad: self.quad,
            word_fc: self.piece_id,
            rotation: self.piece.rotation,
            variation: self.piece.variation,
            orientation: self.piece.orientation,
            cell_info: self.edges.clone(),
            triangles: self.parts.clone(),
            bbox_min: self.bounds.min,
            bbox_max: self.bounds.max,
            network_flags: self.piece.network_flags,
            path_id: self.piece.path_id_override,
            path_info_key: self.path_info_key,
            int64: self.piece.demolition_cost,
            ..NetworkBaseRecord::default()
        }
    }
    /// Apply decoded state without accessing city services (Read precedes Init).
    pub fn from_record(mut record: NetworkBaseRecord) -> Self {
        let effects = record.load_effects();
        record.finish_read_alpha(false);
        let mut owner = Self::new();
        owner.base = record.base;
        owner.piece.in_city = owner.base.flags & 1 != 0;
        owner.transform = record.transform;
        owner.quad = record.quad;
        owner.parts = record.triangles;
        owner.edges = record.cell_info;
        owner.bounds = Bounds {
            min: record.bbox_min,
            max: record.bbox_max,
        };
        owner.piece_id = record.word_fc;
        owner.piece.rotation = record.rotation;
        owner.piece.variation = record.variation;
        owner.piece.orientation = record.orientation;
        owner.piece.network_flags = record.network_flags;
        owner.piece.path_id_override = record.path_id;
        owner.piece.demolition_cost = record.int64;
        owner.piece.position = owner.position();
        owner.path_info_key = record.path_info_key;
        owner.path_origin = record.legacy_offsets.unwrap_or([0.0; 2]);
        if let Some((x, z)) = effects.legacy_cell {
            owner.set_complex_origin(x as u32, z as u32);
        }
        if effects.add_legacy_mark {
            owner.set_retrofit(true);
        }
        owner
    }
}

/// §2.10's first entry is retained even when cleared; array removals swap last.
impl CellInfo {
    pub fn set_edge_types(&mut self, kind: u8, flags: u32) {
        if kind == self.byte_10c {
            self.word_110 = flags;
        } else if self.word_110 == 0 {
            self.byte_10c = kind;
            self.word_110 = flags;
        } else if let Some(i) = self.entries.iter().position(|&(t, _)| t == kind) {
            if flags == 0 {
                self.entries.swap_remove(i);
            } else {
                self.entries[i].1 = flags;
            }
        } else if flags != 0 {
            self.entries.push((kind, flags));
        }
    }
    pub fn edge_types(&self, kind: u8) -> u32 {
        if kind == self.byte_10c {
            self.word_110
        } else {
            self.entries.iter().find(|&&(t, _)| t == kind).map_or(0, |&(_, f)| f)
        }
    }
    pub fn set_edge_type(&mut self, kind: u8, side: u8, edge: u8) {
        let shift = u32::from(side) * 8;
        self.set_edge_types(
            kind,
            (self.edge_types(kind) & !(255 << shift)) | (u32::from(edge) << shift),
        );
    }
    pub fn all_edges(&self) -> u32 {
        self.entries.iter().fold(self.word_110, |f, &(_, e)| f | e)
    }
    fn mask(flags: u32, bits: u32) -> u8 {
        (0..4).fold(0, |m, i| m | if (flags >> (i * 8)) & bits != 0 { 1 << i } else { 0 })
    }
    pub fn active_edges(&self) -> u8 {
        Self::mask(self.all_edges(), 7)
    }
    pub fn active_edges_for_type(&self, kind: u8) -> u8 {
        Self::mask(self.edge_types(kind), 7)
    }
    pub fn active_edges_no_group(&self, kind: u8) -> u8 {
        Self::mask(self.edge_types(kind), 3)
    }
    pub fn grouped_edges(&self) -> u8 {
        Self::mask(self.all_edges(), 4)
    }
    pub fn connects_to_neighbor(&self, flags: u32, side: u8, kind: u8) -> bool {
        super::is_of_type(flags, u32::from(kind)) && (self.edge_types(kind) >> (side * 8)) & 3 != 0
    }
}

#[cfg(test)]
mod tests;
