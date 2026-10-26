//! Power-pole state, records, connection cleanup and published geometry (§4).
//! Native model/draw interfaces are supplied by the plugin host.

use crate::stream::ResKey;
use crate::{automata::OccupantBase, stream::RefValue};

use super::{
    connection::read_wall_into,
    pylon::{clear_ref, same_ref, References},
    record::Vertex,
    underground::write_wall,
};
use super::{identity::OCCUPANT_TYPE_POWER_POLE, xform::Xform};
use crate::stream::{InStream, OutStream};

/// Native pool allocation size, not the size of this portable Rust state.
pub const POWER_POLE_POOL_OBJECT_SIZE: usize = 0x124;
pub const PROP_POLE_MODEL: u32 = 0x2781_2821;
pub const POLE_MODEL_RESOURCE_TYPE: u32 = 0x9AD0_E7E3;

/// Shared attachment-table selection (§4.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionPointTable {
    First,
    Second,
}

/// §4.1: one logical connection, including both vertex lists. The unnamed
/// words/bytes deliberately retain their spec names instead of guessed roles.
#[derive(Clone, Debug)]
pub struct PoleConnection {
    pub pole: RefValue,
    pub wire_points: Vec<RefValue>,
    pub flags: [u8; 2],
    pub words: [u32; 2],
    pub vertices: [Vec<Vec<[f32; 3]>>; 2],
}

/// DrawPowerlines output (4.7.5): xyz+uv only, DrawPrims(6,2,4).
#[derive(Clone, Debug, PartialEq)]
pub struct WireQuad {
    pub texture: u32,
    pub vertices: [[f32; 5]; 4],
}

impl Default for PoleConnection {
    fn default() -> Self {
        Self {
            pole: RefValue::Null,
            wire_points: Vec::new(),
            flags: [0; 2],
            words: [0; 2],
            vertices: Default::default(),
        }
    }
}

/// Live semantic fields; the saved +110 byte has no inferred role (NO42).
#[derive(Clone, Debug)]
pub struct PoleFields {
    pub transform: Xform,
    pub position: [f32; 3],
    pub cell: (i32, i32),
    pub model_y: f32,
    pub distressed: u8,
    pub ignored: u8,
    pub field_118: u32,
    pub connections: Vec<PoleConnection>,
}

impl Default for PoleFields {
    fn default() -> Self {
        Self {
            transform: Xform::default(),
            position: [0.0; 3],
            cell: (0, 0),
            model_y: 0.0,
            distressed: 0,
            ignored: 0,
            field_118: 0,
            connections: Vec::new(),
        }
    }
}

/// Shared writable tables, including the record reader's first-four overwrite.
#[derive(Clone, Debug)]
pub struct AttachmentTables(pub [[[[f32; 3]; 4]; 4]; 2]);

impl Default for AttachmentTables {
    fn default() -> Self {
        let mut tables = [[[[0.0; 3]; 4]; 4]; 2];
        for (t, table) in tables.iter_mut().enumerate() {
            for (r, points) in table.iter_mut().enumerate() {
                let distance = if t == 0 {
                    [4.8, 6.5, 4.8, 6.5][r]
                } else {
                    [3.3, 4.5, 4.5, 3.3][r]
                };
                for (i, p) in points.iter_mut().enumerate() {
                    let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
                    *p = [0.0, if i < 2 { 31.7 } else { 38.7 }, 0.0];
                    if t == 0 {
                        if r < 2 {
                            p[2] = sign * distance;
                        } else {
                            p[0] = -sign * distance;
                        }
                    } else {
                        p[0] = -sign * distance;
                        p[2] = if r < 2 { sign * distance } else { -sign * distance };
                    }
                }
            }
        }
        Self(tables)
    }
}

impl ConnectionPointTable {
    fn index(self) -> usize {
        match self {
            Self::First => 0,
            Self::Second => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoleUnsupported {
    AttachmentInvariant,
    DiagonalFloorNO43,
    CellWalkNO33NO43,
    ModelBoundsPickDrawNO33NO43,
}

/// External occupant manager, effects and reciprocal pole calls only.
pub trait PoleHost: References {
    fn cell_size(&mut self) -> [f32; 2];
    fn insert_line(&mut self, line: &RefValue, argument: u32);
    fn remove_line(&mut self, line: &RefValue, destroy: bool, argument: u32);
    fn reciprocal_remove(&mut self, other: &RefValue, this: &RefValue, quiet: bool);
    fn post_message(&mut self, message: u32, data: u32);
    fn lines_to(&mut self, other: &RefValue, this: &RefValue, out: &mut Vec<(i32, i32)>) -> bool;
    fn create_effect(&mut self, name: &str) -> RefValue;
    fn place_effect(&mut self, effect: &RefValue, position: [f32; 3], rotation: [f32; 9], scale: f32);
    fn start_effect(&mut self, effect: &RefValue);
    fn effect_running(&mut self, effect: &RefValue) -> bool;
    fn stop_effect(&mut self, effect: &RefValue, immediate: bool);
    fn subscription(&mut self, message: u32, subscribe: bool);
    fn occupant_shutdown(&mut self, base: &mut OccupantBase);
}

/// Foreign primitives used by §4.3. In particular, there is no host-owned
/// pole Init, connection refresh, drawing, or geometry callback.
pub trait PoleInitHost {
    fn occupant_init(&mut self, base: &mut OccupantBase);
    fn set_default_exemplar(&mut self, base: &mut OccupantBase, exemplar: ResKey);
    fn model_property(&mut self, base: &OccupantBase, property: u32) -> u32;
    fn model_resource(&mut self, resource_type: u32, model_id: u32) -> RefValue;
}

/// Shared StaticInit state. Quote conversion is foreign variant machinery;
/// copying its signed result and the one-time guard are owned here (§4.3).
#[derive(Clone, Debug)]
pub struct PoleStatics {
    pub control: PowerLineControl,
    pub cell_size: [f32; 2],
    pub model_ids: [u32; 2],
    cost_words: [u32; 2],
    initialized: bool,
    pub widths: [f32; 5],
    pub resource_manager: RefValue,
    pub texture_manager: RefValue,
    pub texture_bindings: [RefValue; 4],
    pub packed_state: u64,
}

impl PoleStatics {
    pub fn new(cost_low: u32, cost_high: u32) -> Self {
        Self {
            control: PowerLineControl::default(),
            cell_size: [0.0; 2],
            model_ids: [0; 2],
            cost_words: [cost_low, cost_high],
            initialized: false,
            widths: [0.3; 5],
            resource_manager: RefValue::Null,
            texture_manager: RefValue::Null,
            texture_bindings: std::array::from_fn(|_| RefValue::Null),
            packed_state: 0,
        }
    }

    /// StaticInit once. City dimensions and model IDs are copied from the
    /// foreign city/model manager; the six property defaults are owned here.
    pub fn init_once(
        &mut self,
        cell_size: [f32; 2],
        model_ids: [u32; 2],
        float: impl Fn(u32) -> Option<f32>,
        uint: impl Fn(u32) -> Option<u32>,
    ) {
        if !self.initialized {
            self.control = PowerLineControl::load(float, uint);
            self.cell_size = cell_size;
            self.model_ids = model_ids;
            self.initialized = true;
        }
    }

    pub fn demolition_cost(&self) -> i64 {
        ((u64::from(self.cost_words[1]) << 32) | u64::from(self.cost_words[0])) as i64
    }

    pub fn initialize(&mut self, host: &mut impl PoleStaticHost) {
        if self.initialized {
            return;
        }
        self.resource_manager = host.acquire_resource_manager();
        host.register_shutdown();
        self.texture_manager = host.acquire_texture_manager(0x1AC0_E11A, 0xFAC0_E219);
        self.control = PowerLineControl::default();
        if host.control_exemplar(CONTROL_EXEMPLAR) {
            self.control = PowerLineControl::load(|id| host.float_property(id), |_| None);
            if let Some(v) = host.uint_property(PROP_89A1E8FB) {
                self.control.v_89a1e8fb = v;
            }
        }
        if host.control_exemplar(CONTROL_EXEMPLAR) {
            if let Some(widths) = host.width_property(0xE9AC_43E7) {
                self.widths = widths;
            }
            if let Some(quote) = host.sint64_property(0x0A2E_CED8) {
                self.cost_words = [quote as u32, (quote as u64 >> 32) as u32];
            }
        }
        self.packed_state = host.pack(super::pylon::RenderStateArgs(
            [1, 1, 1, 1, 0, 1, 0, 3, 4, 5, 4],
            0.1,
            [1, -1],
        ));
        self.initialized = true;
    }
    pub fn shutdown(&mut self, host: &mut impl References) {
        for binding in &mut self.texture_bindings {
            clear_ref(binding, host);
        }
    }
}

pub const POLE_TEXTURES: [u32; 4] = [0xAA9D_A78E, 0xC9AF_0FCD, 0x0912_220E, 0x0808_0004];

pub trait PoleStaticHost {
    fn acquire_resource_manager(&mut self) -> RefValue;
    fn acquire_texture_manager(&mut self, class: u32, iid: u32) -> RefValue;
    fn register_shutdown(&mut self);
    fn control_exemplar(&mut self, key: ResKey) -> bool;
    fn float_property(&self, property: u32) -> Option<f32>;
    fn uint_property(&self, property: u32) -> Option<u32>;
    fn width_property(&self, property: u32) -> Option<[f32; 5]>;
    /// None includes failed variant conversion; destination stays unchanged.
    fn sint64_property(&self, property: u32) -> Option<i64>;
    fn pack(&mut self, args: super::pylon::RenderStateArgs) -> u64;
}

/// Foreign exemplar lookup; table selection itself stays in this owner.
pub trait PoleRecordHost: References + PoleStaticHost {
    fn exemplar_model_instance(&mut self, base: &OccupantBase) -> u32;
}

#[derive(Clone, Debug)]
pub struct PowerPole {
    pub base: OccupantBase,
    pub fields: PoleFields,
    pub wire_exemplar_index: u32,
    pub model_ids: [u32; 2],
    pub model: RefValue,
    pub connection_point_table: Option<ConnectionPointTable>,
    pub geometry: [Vec<Vertex>; 3],
    pub bytes_54_55: [u8; 2],
    pub scale_64: f32,
    pub stale: bool,
    pub subscribed: bool,
    pub effect: RefValue,
}

impl PowerPole {
    pub fn new(mut fields: PoleFields) -> Self {
        // Explicit-state construction for fixtures/loaded state. `fresh`
        // supplies the complete published §4.1 constructor defaults.
        fields.transform.matrix = super::xform::IDENTITY_MATRIX;
        Self {
            base: OccupantBase::new(OCCUPANT_TYPE_POWER_POLE),
            fields,
            wire_exemplar_index: 0,
            model_ids: [0; 2],
            model: RefValue::Null,
            connection_point_table: None,
            geometry: Default::default(),
            bytes_54_55: [0; 2],
            scale_64: 1.0,
            stale: true,
            subscribed: false,
            effect: RefValue::Null,
        }
    }

    pub fn connection_count(&self) -> usize {
        self.fields.connections.len()
    }

    pub fn connection(&self, i: usize) -> Option<&PoleConnection> {
        self.fields.connections.get(i)
    }

    pub fn connected_pole(&self, i: usize) -> Option<RefValue> {
        self.connection(i).map(|c| c.pole.clone())
    }

    pub fn get_cell(&self) -> (i32, i32) {
        self.fields.cell
    }

    /// SetPosition's reciprocal and products stay extended until conversion (4.7).
    pub fn position_cell(position: [f32; 3], terrain_width: f32) -> (i32, i32) {
        let reciprocal = 1.0 / f64::from(terrain_width);
        (
            (f64::from(position[0]) * reciprocal) as i32,
            (f64::from(position[2]) * reciprocal) as i32,
        )
    }

    /// Terrain inputs are obtained by the host at `position_cell`. Moving a pole
    /// does not invalidate its cached bounds, refresh wires or move its own transform.
    pub fn set_position(
        &mut self,
        position: [f32; 3],
        terrain_width: f32,
        corners: Option<[f32; 4]>,
        diagonal: bool,
        rotation: usize,
        host: &mut impl PoleHost,
    ) -> Result<(), PoleUnsupported> {
        self.fields.position = position;
        self.fields.cell = Self::position_cell(position, terrain_width);
        self.fields.model_y = match corners {
            Some(corners) => {
                let mut lowest = corners[0];
                for corner in &corners[1..] {
                    if *corner < lowest {
                        lowest = *corner;
                    }
                }
                lowest
            }
            None => position[1], // 4.7's off-map choice, instead of uninitialized corners.
        };
        self.rebuild_geometry(diagonal, rotation)?;
        if !matches!(self.effect, RefValue::Null) {
            host.place_effect(&self.effect, position, super::xform::IDENTITY_MATRIX, 1.0);
        }
        Ok(())
    }

    pub fn rebuild_geometry(&mut self, diagonal: bool, rotation: usize) -> Result<(), PoleUnsupported> {
        self.create_floor(diagonal)?;
        self.create_walls(rotation);
        Ok(())
    }

    pub fn is_distressed(&self) -> bool {
        self.fields.distressed != 0
    }

    pub fn ignored(&self) -> bool {
        self.fields.ignored != 0
    }

    pub fn set_field_118(&mut self, value: u32) {
        self.fields.field_118 = value;
    }

    /// Unlisted subtypes retain the old table, including null (§4.6).
    pub fn select_connection_point_table(&mut self, subtype_nibble: u8) {
        self.connection_point_table = match subtype_nibble {
            1 | 2 => Some(ConnectionPointTable::First),
            3 | 4 => Some(ConnectionPointTable::Second),
            _ => self.connection_point_table,
        };
    }

    pub fn init_connection_points(&mut self, instance: u32) {
        self.select_connection_point_table(((instance >> 8) & 15) as u8);
    }

    /// §4.3's specified prefix. Caller must perform shared init_once first.
    /// This deliberately does not masquerade as full Init: full model/resource
    /// decisions remain NO33. `set_distressed` and attachment tables are owned.
    pub fn init_published_state(
        &mut self,
        statics: &PoleStatics,
        exemplar: ResKey,
        wire_index: u32,
        subtype_nibble: u8,
        host: &mut impl PoleInitHost,
    ) {
        host.occupant_init(&mut self.base);
        self.model_ids = statics.model_ids;
        self.wire_exemplar_index = wire_index;
        host.set_default_exemplar(&mut self.base, exemplar);
        let id = host.model_property(&self.base, PROP_POLE_MODEL);
        self.model = host.model_resource(POLE_MODEL_RESOURCE_TYPE, id);
        self.select_connection_point_table(subtype_nibble);
        for vector in &mut self.geometry {
            vector.clear();
        }
        self.bytes_54_55 = [0; 2];
        self.scale_64 = 1.0;
    }

    /// Fresh constructor, distinct from the explicit supplied-state constructor.
    pub fn fresh() -> Self {
        let mut pole = Self::new(PoleFields::default());
        pole.model_ids = [u32::MAX; 2]; // cached signed rotation/zoom = -1
        pole
    }

    pub fn set_preview(&mut self, preview: bool) {
        self.fields.field_118 = u32::from(preview);
    }
    pub fn is_preview(&self) -> bool {
        self.fields.field_118 != 0
    }
    pub fn set_highlight(&mut self, highlight: u8) {
        self.bytes_54_55[1] = highlight;
        self.bytes_54_55[0] = (self.bytes_54_55[0] & !1) | u8::from(highlight != 0);
    }
    pub fn sort_value(&self) -> u32 {
        0xAA9D_A78E
    }
    pub fn model_instances(&mut self, instance: &RefValue, host: &mut impl References) -> [RefValue; 1] {
        self.fields.transform.translation = self.fields.position;
        self.fields.transform.mask |= 1;
        self.fields.transform.counter = self.fields.transform.counter.wrapping_add(1);
        host.add_ref(instance);
        [instance.clone()]
    }
    pub fn attachment_points(
        &self,
        rotation: u32,
        tables: &AttachmentTables,
    ) -> Result<[[f32; 3]; 4], PoleUnsupported> {
        let table = self
            .connection_point_table
            .ok_or(PoleUnsupported::AttachmentInvariant)?;
        let mut points = tables.0[table.index()][rotation as usize % 4];
        for point in &mut points {
            for (v, offset) in point.iter_mut().zip(self.fields.position) {
                *v += offset;
            }
        }
        Ok(points)
    }
    pub fn insert_lines(&mut self, host: &mut impl PoleHost) {
        for record in &mut self.fields.connections {
            if record.flags[0] == 0 {
                for line in &record.wire_points {
                    host.insert_line(line, 0);
                }
                record.flags[0] = 1;
            }
        }
    }
    pub fn remove_connection_record(&mut self, index: usize, quiet: bool, host: &mut impl PoleHost) {
        let record = &mut self.fields.connections[index];
        clear_ref(&mut record.pole, host);
        for line in record.wire_points.drain(..) {
            host.remove_line(&line, true, 0);
            if !matches!(line, RefValue::Null) {
                host.release(&line);
            }
        }
        record.flags[0] = 0;
        if record.flags[1] != 0 && !quiet {
            host.post_message(0x99EF_1145, 0);
        }
        self.stale = true;
    }
    pub fn remove_connection(&mut self, pole: &RefValue, quiet: bool, host: &mut impl PoleHost) {
        if let Some(index) = self.fields.connections.iter().position(|r| same_ref(&r.pole, pole)) {
            self.remove_connection_record(index, quiet, host);
            self.fields.connections.remove(index);
        }
    }
    pub fn break_all_connections(&mut self, this: &RefValue, quiet: bool, host: &mut impl PoleHost) {
        for i in 0..self.fields.connections.len() {
            let other = &self.fields.connections[i].pole;
            if !matches!(other, RefValue::Null) {
                host.reciprocal_remove(other, this, false);
            }
            self.remove_connection_record(i, quiet, host);
        }
    }
    pub fn line_cells(
        &self,
        index: usize,
        this: &RefValue,
        out: &mut Vec<(i32, i32)>,
        host: &mut impl PoleHost,
    ) -> Result<bool, PoleUnsupported> {
        let Some(record) = self.fields.connections.get(index) else {
            return Ok(false);
        };
        if record.flags[1] == 0 && !matches!(record.pole, RefValue::Null) {
            return Ok(host.lines_to(&record.pole, this, out));
        }
        let [width, depth] = host.cell_size();
        let mut point = [self.fields.position[0] / width, self.fields.position[2] / depth];
        let delta = [record.words[0] as f32 - point[0], record.words[1] as f32 - point[1]];
        let [a, b] = delta.map(f32::abs);
        let count = a.max(b).trunc() as usize + 1;
        let mut step = if a > b {
            [1.0, b / a]
        } else {
            [if b == 0.0 { 0.0 } else { a / b }, 1.0]
        };
        for (step, delta) in step.iter_mut().zip(delta) {
            if delta < 0.0 {
                *step = -*step;
            }
        }
        out.clear();
        for _ in 0..count {
            out.push((point[0] as i32, point[1] as i32));
            for (point, step) in point.iter_mut().zip(step) {
                *point += step;
            }
        }
        Ok(true)
    }
    pub fn line_cells_to(
        &self,
        pole: &RefValue,
        this: &RefValue,
        out: &mut Vec<(i32, i32)>,
        host: &mut impl PoleHost,
    ) -> Result<bool, PoleUnsupported> {
        match self.fields.connections.iter().position(|r| same_ref(&r.pole, pole)) {
            Some(i) => self.line_cells(i, this, out, host),
            None => Ok(false),
        }
    }
    pub fn set_distressed(&mut self, distressed: bool, host: &mut impl PoleHost) {
        self.fields.distressed = u8::from(distressed);
        if distressed {
            if matches!(self.effect, RefValue::Null) {
                self.effect = host.create_effect("powerpolezap");
                if !matches!(self.effect, RefValue::Null) {
                    host.place_effect(&self.effect, self.fields.position, super::xform::IDENTITY_MATRIX, 1.0);
                    host.start_effect(&self.effect);
                }
            }
        } else {
            self.stop_distress_effect(false, host);
        }
    }
    fn stop_distress_effect(&mut self, immediate: bool, host: &mut impl PoleHost) {
        if !matches!(self.effect, RefValue::Null) {
            if immediate || host.effect_running(&self.effect) {
                host.stop_effect(&self.effect, immediate);
            }
            clear_ref(&mut self.effect, host);
        }
    }
    pub fn on_inserted_message(&mut self, message: u32, is_this: bool, host: &mut impl PoleHost) {
        if message == 0x99EF_1142 && is_this {
            host.subscription(message, false);
            self.subscribed = false;
            self.insert_lines(host);
        }
    }
    pub fn shutdown(&mut self, this: &RefValue, host: &mut impl PoleHost) {
        self.break_all_connections(this, true, host);
        self.stale = false;
        clear_ref(&mut self.model, host);
        for list in &mut self.geometry {
            list.clear();
        }
        if self.subscribed {
            host.subscription(0x99EF_1142, false);
            self.subscribed = false;
        }
        self.stop_distress_effect(true, host);
        host.occupant_shutdown(&mut self.base);
    }
    pub fn create_floor(&mut self, diagonal: bool) -> Result<(), PoleUnsupported> {
        self.geometry[0].clear();
        for (i, (x, z)) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, 1.0), (1.0, -1.0)]
            .into_iter()
            .enumerate()
        {
            let (x, z) = (x * 5.0, z * 5.0);
            let (x, z) = if diagonal {
                let c = f64::from(f32::from_bits(0x3F35_0554));
                let s = f64::from(f32::from_bits(0x3F35_0493));
                ((-s * z + c * x) as f32, s * x + c * z)
            } else {
                (x as f32, z)
            };
            self.geometry[0].push(Vertex {
                pos: [
                    x + self.fields.position[0],
                    self.fields.position[1],
                    (z + f64::from(self.fields.position[2])) as f32,
                ],
                color: u32::MAX,
                u: [0.0, 0.0, 1.0, 1.0][i],
                v: [0.0, 1.0, 1.0, 0.0][i],
            });
        }
        Ok(())
    }
    pub fn wall_quad(a: Vertex, b: Vertex, bottom_y: f32) -> Vec<Vertex> {
        [(a, false, 0.0), (a, true, 0.0), (b, true, 1.0), (b, false, 1.0)]
            .into_iter()
            .map(|(p, bottom, u)| {
                let mut v = p;
                v.color |= 0x00FF_FFFF;
                v.u = u;
                v.v = if bottom {
                    ((f64::from(p.pos[1]) - f64::from(bottom_y)) * 0.0625) as f32
                } else {
                    0.0
                };
                if bottom {
                    v.pos[1] = bottom_y;
                }
                v
            })
            .collect()
    }
    pub fn create_walls(&mut self, rotation: usize) {
        let pairs = [[(2, 3), (1, 2)], [(1, 2), (0, 1)], [(0, 1), (3, 0)], [(3, 0), (2, 3)]][rotation];
        for (i, (a, b)) in pairs.into_iter().enumerate() {
            self.geometry[i + 1] = Self::wall_quad(self.geometry[0][a], self.geometry[0][b], self.fields.model_y);
        }
    }

    pub fn wire_quads(&self, width: f32) -> Vec<WireQuad> {
        let mut out = Vec::new();
        if width <= 0.0 {
            return out;
        }
        let width = f64::from(width);
        for record in &self.fields.connections {
            if record.flags[1] == 0 {
                continue;
            }
            for curve in &record.vertices[0] {
                for pair in curve.windows(2) {
                    let [a, b] = [pair[0], pair[1]];
                    let dx = f64::from(b[0]) - f64::from(a[0]);
                    let dz = f64::from(b[2]) - f64::from(a[2]);
                    let inverse = 1.0 / (dx * dx + dz * dz).sqrt();
                    let nx = f64::from((inverse * dx) as f32);
                    let nz = f64::from((inverse * dz) as f32);
                    let dx = f64::from(dx as f32);
                    let dz = f64::from(dz as f32);
                    let v = ((dx * dx + dz * dz).sqrt() * 0.125) as f32;
                    out.push(WireQuad {
                        texture: 0xAA9D_A78E,
                        vertices: [
                            (a, -1.0, 0.0, 0.0),
                            (a, 1.0, 1.0, 0.0),
                            (b, 1.0, 1.0, v),
                            (b, -1.0, 0.0, v),
                        ]
                        .map(|(p, sign, u, v)| {
                            [
                                (f64::from(p[0]) + sign * width * nz) as f32,
                                p[1],
                                (f64::from(p[2]) - sign * width * nx) as f32,
                                u,
                                v,
                            ]
                        }),
                    });
                }
            }
            for curve in &record.vertices[1] {
                for pair in curve.windows(2) {
                    let p = pair[0];
                    out.push(WireQuad {
                        texture: 0xC9AF_0FCD,
                        vertices: [
                            (-1.0, -1.0, 0.0, 0.0),
                            (-1.0, 1.0, 1.0, 0.0),
                            (1.0, 1.0, 1.0, 1.0),
                            (1.0, -1.0, 0.0, 1.0),
                        ]
                        .map(|(x, z, u, v)| {
                            [
                                (f64::from(p[0]) + x * width * 4.0) as f32,
                                p[1],
                                (f64::from(p[2]) + z * width * 4.0) as f32,
                                u,
                                v,
                            ]
                        }),
                    });
                }
            }
        }
        out
    }

    pub fn write(&self, tables: &AttachmentTables, out: &mut dyn OutStream) -> Result<bool, PoleUnsupported> {
        let table = self
            .connection_point_table
            .ok_or(PoleUnsupported::AttachmentInvariant)?;
        out.put_u16(5);
        if self.base.write(out) {
            self.fields.transform.write(out);
            write_xyz(self.fields.position, out);
            out.put_u32(self.fields.connections.len() as u32);
            for c in &self.fields.connections {
                out.put_ref(&c.pole); // adapter passes pole serializable view +0xC
                out.put_u32(c.wire_points.len() as u32);
                for line in &c.wire_points {
                    out.put_ref(line);
                } // line view +8
                for flag in c.flags {
                    out.put_bool(flag != 0);
                }
                for word in c.words {
                    out.put_u32(word);
                }
                for collection in &c.vertices {
                    out.put_u32(collection.len() as u32);
                    for points in collection {
                        out.put_u32(points.len() as u32);
                        for point in points {
                            write_xyz(*point, out);
                        }
                    }
                }
            }
            out.put_u32(self.wire_exemplar_index);
            for word in self.model_ids {
                out.put_u32(word);
            }
            out.put_u32(self.fields.cell.0 as u32);
            out.put_u32(self.fields.cell.1 as u32);
            for point in tables.0[table.index()][0] {
                write_xyz(point, out);
            }
            for list in &self.geometry {
                write_wall(list, out);
            }
            out.put_f32(self.fields.model_y);
            out.put_bool(self.fields.ignored != 0);
            for byte in self.bytes_54_55 {
                out.put_u8(byte);
            }
            out.put_f32(self.scale_64);
            out.put_u8(self.fields.distressed);
        }
        Ok(out.error() == 0)
    }

    pub fn read_into(
        &mut self,
        tables: &mut AttachmentTables,
        statics: &mut PoleStatics,
        inp: &mut dyn InStream,
        host: &mut impl PoleRecordHost,
    ) -> Result<bool, PoleUnsupported> {
        statics.initialize(host);
        let Some(version) = inp.get_u16() else {
            return Ok(false);
        };
        if version < 4 {
            return Ok(false);
        }
        if let Some(base) = OccupantBase::read(inp) {
            self.base = base;
            if version == 4 {
                self.fields.transform.legacy_load_into(inp);
            } else {
                self.fields.transform.read_into(inp);
            }
            read_xyz(&mut self.fields.position, inp);
            if let Some(count) = inp.get_u32() {
                // Native resize releases old owned refs when shrinking.
                for c in self.fields.connections.iter_mut().skip(count as usize) {
                    clear_ref(&mut c.pole, host);
                    for line in &mut c.wire_points {
                        clear_ref(line, host);
                    }
                }
                self.fields
                    .connections
                    .resize_with(count as usize, PoleConnection::default);
                for c in &mut self.fields.connections {
                    clear_ref(&mut c.pole, host);
                    if let Some(v) = inp.get_ref_as(0x09C0_5C6A) {
                        c.pole = v;
                    }
                    if let Some(n) = inp.get_u32() {
                        for line in &mut c.wire_points {
                            clear_ref(line, host);
                        }
                        c.wire_points.resize_with(n as usize, || RefValue::Null);
                        for line in &mut c.wire_points {
                            if let Some(v) = inp.get_ref_as(0xC9C0_5C5D) {
                                *line = v;
                            }
                        }
                    }
                    for flag in &mut c.flags {
                        if let Some(v) = inp.get_bool() {
                            *flag = u8::from(v);
                        }
                    }
                    for word in &mut c.words {
                        if let Some(v) = inp.get_u32() {
                            *word = v;
                        }
                    }
                    for collection in &mut c.vertices {
                        if let Some(n) = inp.get_u32() {
                            collection.resize_with(n as usize, Vec::new);
                            for points in collection {
                                if let Some(n) = inp.get_u32() {
                                    points.resize(n as usize, [0.0; 3]);
                                    for point in points {
                                        read_xyz(point, inp);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(v) = inp.get_u32() {
                self.wire_exemplar_index = v;
            }
            for word in &mut self.model_ids {
                if let Some(v) = inp.get_u32() {
                    *word = v;
                }
            }
            if let Some(v) = inp.get_u32() {
                self.fields.cell.0 = v as i32;
            }
            if let Some(v) = inp.get_u32() {
                self.fields.cell.1 = v as i32;
            }
            let instance = host.exemplar_model_instance(&self.base);
            self.select_connection_point_table(((instance >> 8) & 15) as u8);
            let table = self.connection_point_table;
            if let Some(table) = table {
                for point in &mut tables.0[table.index()][0] {
                    read_xyz(point, inp);
                }
                for wall in &mut self.geometry {
                    read_wall_into(wall, inp);
                }
                if let Some(v) = inp.get_f32() {
                    self.fields.model_y = v;
                }
                if let Some(v) = inp.get_bool() {
                    self.fields.ignored = u8::from(v);
                }
                for byte in &mut self.bytes_54_55 {
                    if let Some(v) = inp.get_u8() {
                        *byte = v;
                    }
                }
                if let Some(v) = inp.get_f32() {
                    self.scale_64 = v;
                }
                if let Some(v) = inp.get_u8() {
                    self.fields.distressed = v;
                }
            }
            clear_ref(&mut self.model, host);
            if table.is_none() {
                return Err(PoleUnsupported::AttachmentInvariant);
            }
        }
        Ok(inp.error() == 0)
    }
}

fn write_xyz(point: [f32; 3], out: &mut dyn OutStream) {
    for v in point {
        out.put_f32(v);
    }
}
fn read_xyz(point: &mut [f32; 3], inp: &mut dyn InStream) {
    for v in point {
        if let Some(value) = inp.get_f32() {
            *v = value;
        }
    }
}

/// The exemplar the control variables come from (§4.3).
pub const CONTROL_EXEMPLAR: ResKey = ResKey {
    type_id: 0x6534_284A,
    group: 0xE7E2_C2DB,
    instance: 0xC911_E35B,
};

pub const PROP_E9A18D3F: u32 = 0xE9A1_8D3F;
pub const PROP_49A1E8C4: u32 = 0x49A1_E8C4;
pub const PROP_89A18FC4: u32 = 0x89A1_8FC4;
pub const PROP_E9A1E8D4: u32 = 0xE9A1_E8D4;
pub const PROP_49A195C0: u32 = 0x49A1_95C0;
pub const PROP_89A1E8FB: u32 = 0x89A1_E8FB;

/// The six control variables (§4.3). Their meanings are not named by the
/// spec; the fields carry the property IDs and the static addresses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerLineControl {
    /// `0xE9A18D3F` → `0xB46754`, default 16.0.
    pub v_e9a18d3f: f32,
    /// `0x49A1E8C4` → `0xB46750`, default 10.0.
    pub v_49a1e8c4: f32,
    /// `0x89A18FC4` → `0xB4674C`, default 10.0.
    pub v_89a18fc4: f32,
    /// `0xE9A1E8D4` → `0xB46748`, default 0.15 (`0x3E19999A`).
    pub v_e9a1e8d4: f32,
    /// `0x49A195C0` → `0xB46744`, default 8.0.
    pub v_49a195c0: f32,
    /// `0x89A1E8FB` → `0xB46740`, default 5 (an integer default; the
    /// spec does not give the static's type, so it is kept as u32).
    pub v_89a1e8fb: u32,
}

impl Default for PowerLineControl {
    fn default() -> Self {
        PowerLineControl {
            v_e9a18d3f: 16.0,
            v_49a1e8c4: 10.0,
            v_89a18fc4: 10.0,
            v_e9a1e8d4: f32::from_bits(0x3E19_999A),
            v_49a195c0: 8.0,
            v_89a1e8fb: 5,
        }
    }
}

impl PowerLineControl {
    /// Reads the variables from the control exemplar: each property that
    /// `float`/`uint` answers replaces its default; a missing one keeps it.
    pub fn load(float: impl Fn(u32) -> Option<f32>, uint: impl Fn(u32) -> Option<u32>) -> Self {
        let d = PowerLineControl::default();
        PowerLineControl {
            v_e9a18d3f: float(PROP_E9A18D3F).unwrap_or(d.v_e9a18d3f),
            v_49a1e8c4: float(PROP_49A1E8C4).unwrap_or(d.v_49a1e8c4),
            v_89a18fc4: float(PROP_89A18FC4).unwrap_or(d.v_89a18fc4),
            v_e9a1e8d4: float(PROP_E9A1E8D4).unwrap_or(d.v_e9a1e8d4),
            v_49a195c0: float(PROP_49A195C0).unwrap_or(d.v_49a195c0),
            v_89a1e8fb: uint(PROP_89A1E8FB).unwrap_or(d.v_89a1e8fb),
        }
    }
}
