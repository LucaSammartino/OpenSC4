//! Published underground-connection record/shutdown, network-occupants §3.6.
//! Drawing/model construction remains open in underground-occupants §8 UG1.
//! Creation geometry, callback and render-state inputs are tracked by UG5.

use crate::automata::OccupantBase;
use crate::stream::{InStream, OutStream, RefValue};

use super::record::Vertex;
use super::underground::{read_wall, write_wall};

pub const CONNECTION_RECORD_VERSION: u16 = 2;
pub const CONNECTION_MESSAGES: [u32; 4] = [0x8B87_A974, 0x8B87_A975, 0x8B8B_2CEE, 0xA6B7_9602];

/// Stream fields in §3.6 order. This raw record avoids inventing a mapping
/// between serialized +C4/+C8 views and the conflicting live references (UG5).
#[derive(Clone, Debug)]
pub struct ConnectionRecord {
    pub version_read: u16,
    pub base: OccupantBase,
    pub field_64: u32,
    pub walls: [Vec<Vertex>; 7],
    pub words_14c_150: [u32; 2],
    pub words_15c: [u32; 4],
    pub field_16c: u32,
    pub field_174: u32,
    pub flag_178: bool,
    /// Already adjusted serializable views, not native addresses.
    pub references: [RefValue; 2],
    pub field_23c: u32,
    pub words_22c: [u32; 3],
}

impl ConnectionRecord {
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(CONNECTION_RECORD_VERSION);
        if !self.base.write(out) {
            return false;
        }
        out.put_u32(self.field_64);
        for wall in &self.walls {
            write_wall(wall, out);
        }
        for word in self.words_14c_150 {
            out.put_u32(word);
        }
        for word in self.words_15c {
            out.put_u32(word);
        }
        out.put_u32(self.field_16c);
        out.put_u32(self.field_174);
        out.put_bool(self.flag_178);
        for reference in &self.references {
            out.put_ref(reference);
        }
        out.put_u32(self.field_23c);
        for word in self.words_22c {
            out.put_u32(word);
        }
        out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream) -> Option<Self> {
        // The spec says symmetric but states no version rejection predicate.
        // Literal choice: retain the read version, no invented rejection (UG5).
        let version_read = inp.get_u16()?;
        let base = OccupantBase::read(inp)?;
        let field_64 = inp.get_u32()?;
        let mut walls: [Vec<Vertex>; 7] = Default::default();
        for wall in &mut walls {
            *wall = read_wall(inp)?;
        }
        Some(Self {
            version_read,
            base,
            field_64,
            walls,
            words_14c_150: [inp.get_u32()?, inp.get_u32()?],
            words_15c: [inp.get_u32()?, inp.get_u32()?, inp.get_u32()?, inp.get_u32()?],
            field_16c: inp.get_u32()?,
            field_174: inp.get_u32()?,
            flag_178: inp.get_bool()?,
            references: [inp.get_ref()?, inp.get_ref()?],
            field_23c: inp.get_u32()?,
            words_22c: [inp.get_u32()?, inp.get_u32()?, inp.get_u32()?],
        })
    }
}

pub trait ConnectionLifecycleHost {
    fn occupant_init(&mut self, base: &mut OccupantBase);
    fn subscribe_message(&mut self, message: u32);
    /// A foreign texture-binding primitive, supplied the named binding slot.
    fn clear_texture_binding(&mut self, offset: u32);
    fn unsubscribe_message(&mut self, message: u32);
    fn occupant_shutdown(&mut self, base: &mut OccupantBase);
}

/// Runtime-only fields explicitly cleared by Shutdown. Unpublished defaults
/// are supplied by the caller; no RefCell state is used for getters.
#[derive(Clone, Debug)]
pub struct UndergroundConnection {
    pub base: OccupantBase,
    pub vectors_70: [Vec<[f32; 3]>; 4],
    pub vector_a4: Vec<[f32; 3]>,
    pub vector_b0: Vec<[f32; 3]>,
    pub vector_c0: Vec<[f32; 3]>,
    pub reference_158: RefValue,
    pub reference_15c: RefValue,
    pub reference_c8: RefValue,
    pub reference_cc: RefValue,
}

impl UndergroundConnection {
    /// The only specified GetHiddenTriangles behavior (§3.6).
    pub fn hidden_triangles(&self) -> [u32; 3] {
        [0, 1, 1]
    }

    /// This is explicitly a fragment, not full Init: the one-time callback
    /// and SetupPackedRenderState have no specified bodies (UG1/UG5).
    pub fn init_published_subscriptions(&mut self, host: &mut impl ConnectionLifecycleHost) {
        host.occupant_init(&mut self.base);
        for message in CONNECTION_MESSAGES {
            host.subscribe_message(message);
        }
    }

    /// §3.6 shutdown order: bindings, vectors, four held refs, four messages,
    /// then the foreign cSC4Occupant base. No borrowed refs are released.
    pub fn shutdown(&mut self, host: &mut impl ConnectionLifecycleHost) {
        for offset in [0x9C, 0x64, 0xB8] {
            host.clear_texture_binding(offset);
        }
        for vector in &mut self.vectors_70 {
            vector.clear();
        }
        self.vector_a4.clear();
        self.vector_b0.clear();
        self.vector_c0.clear();
        self.reference_158 = RefValue::Null;
        self.reference_15c = RefValue::Null;
        self.reference_c8 = RefValue::Null;
        self.reference_cc = RefValue::Null;
        for message in CONNECTION_MESSAGES {
            host.unsubscribe_message(message);
        }
        host.occupant_shutdown(&mut self.base);
    }
}
