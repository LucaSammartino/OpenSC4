//! Published power-pole state and initialization (network-occupants §4).
//! Unpublished connection/render algorithms and the conflicting record layout
//! are tracked by NO33-NO35; this is not a closed owner.

use crate::stream::ResKey;
use crate::{automata::OccupantBase, stream::RefValue};

use super::{identity::OCCUPANT_TYPE_POWER_POLE, xform::Xform};

/// Native pool allocation size, not the size of this portable Rust state.
pub const POWER_POLE_POOL_OBJECT_SIZE: usize = 0x124;
pub const PROP_POLE_MODEL: u32 = 0x2781_2821;
pub const POLE_MODEL_RESOURCE_TYPE: u32 = 0x9AD0_E7E3;

/// The selection is known, but the table's numeric vertices are not (NO33).
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
    pub vertices: [Vec<[f32; 3]>; 2],
}

/// State with published meanings. Unspecified constructor values are supplied
/// by the caller rather than invented here (NO33). No native offset aliasing
/// or live-to-record mapping is asserted (NO35).
#[derive(Clone, Debug)]
pub struct PoleFields {
    pub transform: Xform,
    pub position: [f32; 3],
    pub cell: (i32, i32),
    pub model_y: f32,
    pub distressed: u8,
    pub ignored: u8,
    pub field_118: u32,
    pub delegate: RefValue,
    pub connections: Vec<PoleConnection>,
}

/// Foreign primitives used by §4.3. In particular, there is no host-owned
/// pole Init, connection refresh, drawing, or geometry callback.
pub trait PoleInitHost {
    fn occupant_init(&mut self, base: &mut OccupantBase);
    fn set_default_exemplar(&mut self, base: &mut OccupantBase, exemplar: ResKey);
    fn model_property(&mut self, base: &OccupantBase, property: u32) -> u32;
    fn model_resource(&mut self, resource_type: u32, model_id: u32) -> RefValue;
}

/// Shared StaticInit inputs whose derivation is outside the published text.
/// Cost is a signed int64 assembled verbatim, never a cast from the control
/// float: the relationship requested by conformance test 17 is missing (NO34).
#[derive(Clone, Debug)]
pub struct PoleStatics {
    pub control: PowerLineControl,
    pub cell_size: [f32; 2],
    pub model_ids: [u32; 2],
    cost_words: [u32; 2],
    initialized: bool,
}

impl PoleStatics {
    pub fn new(cost_low: u32, cost_high: u32) -> Self {
        Self {
            control: PowerLineControl::default(),
            cell_size: [0.0; 2],
            model_ids: [0; 2],
            cost_words: [cost_low, cost_high],
            initialized: false,
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
}

#[derive(Clone, Debug)]
pub struct PowerPole {
    pub base: OccupantBase,
    pub fields: PoleFields,
    pub wire_exemplar_index: u32,
    pub model_ids: [u32; 2],
    pub model: RefValue,
    pub connection_point_table: Option<ConnectionPointTable>,
    pub geometry: [Vec<[f32; 3]>; 3],
    pub bytes_54_55: [u8; 2],
    pub scale_64: f32,
}

impl PowerPole {
    pub fn new(mut fields: PoleFields) -> Self {
        // The only specified matrix default is identity (§4.1). Preserve all
        // other explicitly supplied state; undocumented ctor defaults: NO33.
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

    pub fn is_distressed(&self) -> bool {
        self.fields.distressed != 0
    }

    pub fn ignored(&self) -> bool {
        self.fields.ignored != 0
    }

    pub fn set_field_118(&mut self, value: u32) {
        self.fields.field_118 = value;
    }

    /// The published selection portion of InitConnectionPoints. No numeric
    /// point coordinates or unlisted-subtype fallback is fabricated (NO33).
    pub fn select_connection_point_table(&mut self, subtype_nibble: u8) {
        self.connection_point_table = match subtype_nibble {
            1 | 2 => Some(ConnectionPointTable::First),
            3 | 4 => Some(ConnectionPointTable::Second),
            _ => None,
        };
    }

    /// §4.3's specified prefix. Caller must perform shared init_once first.
    /// This deliberately does not masquerade as full Init: LoadModel internals,
    /// point coordinates and SetDistressed side effects remain NO33.
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
