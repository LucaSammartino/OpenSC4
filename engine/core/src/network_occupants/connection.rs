//! Underground connection owner (§3.6/3.6.1; underground-occupants §4).
//! Descriptor geometry, bounds, drawing and full messages remain UG5/NO40.
use crate::{automata::OccupantBase, stream::{InStream, OutStream, RefValue}};
use super::{record::Vertex, underground::write_wall, pylon::{References, RenderStateArgs, same_ref, clear_ref}};

pub const CONNECTION_RECORD_VERSION: u16 = 2;
pub const CONNECTION_MESSAGES: [u32;4] = [0x8B87_A974,0x8B87_A975,0x8B8B_2CEE,0xA6B7_9602];
pub const CONNECTION_RENDER_STATES: [RenderStateArgs;3] = [
    RenderStateArgs([1,1,1,0,0,1,0,3,-1,-1,-1],-1.0,[1,-1]),
    RenderStateArgs([1,1,1,0,1,1,0,3,4,5,-1],-1.0,[1,-1]),
    RenderStateArgs([1,1,1,0,1,0,0,3,4,5,-1],-1.0,[1,-1]),
];
pub const NEIGHBORS: [(i32,i32);8] = [(-1,0),(0,-1),(1,0),(0,1),(-1,-1),(1,-1),(1,1),(-1,1)];
pub const IDENTITY4: [f32;16] = [1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0,0.0,0.0,1.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionKind { Pipe, Subway }
impl ConnectionKind {
    pub fn class_id(self) -> u32 { match self { Self::Pipe => 0xEA0A_5ACF, Self::Subway => 0xCA15_ED6B } }
    pub fn occupant_class(self) -> u32 { match self { Self::Pipe => 0x49C0_5B9F, Self::Subway => 0xCA16_374F } }
    pub fn network_type(self) -> u32 { match self { Self::Pipe => 4, Self::Subway => 7 } }
    pub fn occupant_type(self) -> u32 { match self { Self::Pipe => 0x2A18_3167, Self::Subway => 0x8A13_918A } }
}

/// An adjustment of a host interface view, without changing reference ownership.
pub trait ConnectionRecordHost: References {
    fn adjust_view(&mut self, value: RefValue, bytes: i32) -> RefValue;
}

#[derive(Clone, Debug)]
pub struct ConnectionRecord {
    pub base: OccupantBase,
    pub saved_68: u32,
    pub walls: [Vec<Vertex>;7],
    pub cell: (i32,i32),
    pub heights: [f32;4],
    pub saved_170: f32,
    pub saved_178: i32,
    pub model_state: bool,
    pub attached: RefValue,
    pub linked: RefValue,
    pub saved_240: u32,
    pub position: [f32;3],
    pub loaded: bool,
}

impl ConnectionRecord {
    pub fn write(&self, out: &mut dyn OutStream, host: &mut impl ConnectionRecordHost) -> bool {
        out.put_u16(2);
        if self.base.write(out) {
            out.put_u32(self.saved_68);
            for wall in &self.walls { write_wall(wall,out); }
            out.put_u32(self.cell.0 as u32); out.put_u32(self.cell.1 as u32);
            for height in self.heights { out.put_f32(height); }
            out.put_f32(self.saved_170);
            out.put_u32(self.saved_178 as u32);
            out.put_bool(self.model_state);
            // The adapter serializes this network occupant through its serializable interface.
            let attached = if matches!(self.attached,RefValue::Null) { RefValue::Null }
                else { host.adjust_view(self.attached.clone(),4) };
            out.put_ref(&attached);
            out.put_ref(&self.linked);
            out.put_u32(self.saved_240);
            for v in self.position { out.put_f32(v); }
        }
        out.error() == 0
    }
    pub fn read_into(&mut self, inp: &mut dyn InStream, host: &mut impl ConnectionRecordHost) -> bool {
        if inp.get_u16() != Some(2) { return false; }
        if let Some(base) = OccupantBase::read(inp) {
            self.base = base;
            if let Some(v) = inp.get_u32() { self.saved_68 = v; }
            for wall in &mut self.walls { read_wall_into(wall,inp); }
            if let Some(v) = inp.get_u32() { self.cell.0 = v as i32; }
            if let Some(v) = inp.get_u32() { self.cell.1 = v as i32; }
            for v in &mut self.heights { if let Some(n) = inp.get_f32() { *v = n; } }
            if let Some(v) = inp.get_f32() { self.saved_170 = v; }
            if let Some(v) = inp.get_u32() { self.saved_178 = v as i32; }
            if let Some(v) = inp.get_bool() { self.model_state = v; }
            clear_ref(&mut self.attached,host);
            if let Some(v) = inp.get_ref_as(0x26BB_767E) {
                self.attached = if matches!(v,RefValue::Null) { v } else { host.adjust_view(v,-4) };
            }
            clear_ref(&mut self.linked,host);
            if let Some(v) = inp.get_ref_as(0x26BB_767E) { self.linked = v; }
            if let Some(v) = inp.get_u32() { self.saved_240 = v; }
            for v in &mut self.position { if let Some(n) = inp.get_f32() { *v = n; } }
            self.model_state = true;
            self.loaded = true;
        }
        inp.error() == 0
    }
}

/// Wall-helper reads retain reached mutations even on a later stream failure.
pub(super) fn read_wall_into(wall: &mut Vec<Vertex>, inp: &mut dyn InStream) {
    if let Some(count) = inp.get_u32() {
        wall.resize(count as usize,Vertex::default());
        for vertex in wall {
            for v in &mut vertex.pos { if let Some(n) = inp.get_f32() { *v = n; } }
            if let Some(n) = inp.get_f32() { vertex.u = n; }
            if let Some(n) = inp.get_f32() { vertex.v = n; }
            for shift in [16,8,0,24] {
                if let Some(n) = inp.get_u8() { vertex.color = (vertex.color & !(255 << shift)) | (u32::from(n) << shift); }
            }
        }
    }
}

pub trait ConnectionHost: ConnectionRecordHost {
    fn acquire_resource_manager(&mut self) -> RefValue;
    fn register_shutdown(&mut self);
    fn renderer_exists(&self) -> bool;
    fn renderer_probe(&mut self);
    fn pack(&mut self, args: RenderStateArgs) -> u64;
    fn occupant_init(&mut self, base: &mut OccupantBase);
    fn occupant_shutdown(&mut self, base: &mut OccupantBase);
    fn message(&mut self, id: u32, subscribe: bool);
    /// Three bindings in the shutdown order, indexed 0..3; no native offsets
    /// are inferred from the saved-word/list offsets.
    fn clear_texture_binding(&mut self, slot: u32);
    fn position_to_cell(&mut self, position: [f32;3]) -> (i32,i32);
    fn cell_origin(&mut self, cell: (i32,i32)) -> [f32;2];
    fn cell_extents(&mut self) -> [f32;2];
    fn cell_size(&mut self) -> f32;
    fn corner_heights(&mut self, cell: (i32,i32)) -> [f32;4];
    /// Search results own one reference; network_interface returns a borrowed one.
    fn find_occupant(&mut self, cell: (i32,i32), class: u32) -> RefValue;
    fn find_connection(&mut self, cell: (i32,i32), class: u32) -> RefValue;
    fn network_interface(&mut self, network_type: u32) -> RefValue;
    fn begin_change(&mut self, network: &RefValue);
    fn connect(&mut self, network: &RefValue, coordinates: [i32;4], tail: [i32;4]);
    fn end_change(&mut self, network: &RefValue);
    fn refresh(&mut self, occupant: &RefValue, propagate: bool);
    fn remove_self(&mut self, destroy: bool, argument: u32);
    /// Foreign QI returns a counted interface, or Null.
    fn query_interface(&mut self, occupant: &RefValue, iid: u32) -> RefValue;
    /// Borrowed adjusted model-instance interface, not the primary occupant.
    fn model_instance(&mut self) -> RefValue;
    fn post_occupant_message(&mut self, message: u32, value: u32);
}

#[derive(Clone, Debug)]
pub struct ConnectionStatics {
    pub latched: bool,
    pub resource_manager: RefValue,
    pub packed_states: [u64;3],
}
impl Default for ConnectionStatics {
    fn default() -> Self { Self { latched: false, resource_manager: RefValue::Null, packed_states: [0;3] } }
}
impl ConnectionStatics {
    pub fn initialize(&mut self, host: &mut impl ConnectionHost) {
        if !self.latched {
            self.resource_manager = host.acquire_resource_manager();
            host.register_shutdown();
            self.latched = true;
        }
    }
    pub fn setup_packed_states(&mut self, host: &mut impl ConnectionHost) {
        if host.renderer_exists() {
            host.renderer_probe();
            for (state,args) in self.packed_states.iter_mut().zip(CONNECTION_RENDER_STATES) { *state = host.pack(args); }
        }
    }
    pub fn shutdown(&mut self, refs: &mut impl References) -> bool {
        clear_ref(&mut self.resource_manager,refs);
        true // The static latch deliberately stays set.
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionUnsupported { GeometryBoundsDrawTransformMessagesUG5NO40 }

/// Unpublished construction inputs must be supplied; creation fills specified state.
#[derive(Clone, Debug)]
pub struct UndergroundConnection {
    pub record: ConnectionRecord,
    pub initialized: bool,
    pub surface: [Vertex;4],
    pub matrices: [[f32;16];2],
    pub bottom_y: f32,
}
impl UndergroundConnection {
    pub fn hidden_triangles(&self) -> [u32;3] { [0,1,1] }
    pub fn init(&mut self, statics: &mut ConnectionStatics, host: &mut impl ConnectionHost) -> bool {
        statics.initialize(host);
        if !self.initialized {
            host.occupant_init(&mut self.record.base);
            self.initialized = true;
            statics.setup_packed_states(host);
            for id in CONNECTION_MESSAGES { host.message(id,true); }
        }
        true
    }
    pub fn create(&mut self, kind: ConnectionKind, statics: &mut ConnectionStatics, host: &mut impl ConnectionHost) -> Result<(),ConnectionUnsupported> {
        self.record.base.occupant_type = kind.occupant_type();
        statics.initialize(host);
        if self.initialized { return Ok(()); }
        host.occupant_init(&mut self.record.base);
        self.initialized = true;
        let r = &mut self.record;
        r.cell = host.position_to_cell(r.position);
        let origin = host.cell_origin(r.cell);
        let extents = host.cell_extents().map(|v| v as i32 as f32);
        let size = host.cell_size();
        r.saved_170 = r.position[1];
        self.bottom_y = (f64::from(r.position[1])-f64::from(size)) as f32;
        r.heights = host.corner_heights(r.cell);
        for (i,v) in self.surface.iter_mut().enumerate() {
            v.pos = [if i >= 2 { origin[0]+extents[0] } else { origin[0] },
                (f64::from(r.heights[i])-f64::from(size)) as f32,
                if i == 1 || i == 2 { origin[1]+extents[1] } else { origin[1] }];
            v.u = if i >= 2 { 1.0 } else { 0.0 };
            v.v = if i == 1 || i == 2 { 1.0 } else { 0.0 };
        }
        self.matrices = [IDENTITY4;2];
        r.saved_178 = 1;
        r.model_state = true;
        statics.setup_packed_states(host);
        clear_ref(&mut r.attached,host);
        r.attached = host.find_occupant(r.cell,kind.occupant_class());
        if matches!(r.attached,RefValue::Null) {
            for (dx,dz) in NEIGHBORS {
                let cell = (r.cell.0.wrapping_add(dx),r.cell.1.wrapping_add(dz));
                let mut neighbor = host.find_occupant(cell,kind.occupant_class());
                if matches!(neighbor,RefValue::Null) { neighbor = host.find_connection(cell,kind.class_id()); }
                if !matches!(neighbor,RefValue::Null) {
                    host.release(&neighbor);
                    if !r.loaded {
                        let network = host.network_interface(kind.network_type());
                        if !matches!(network,RefValue::Null) {
                            host.add_ref(&network);
                            host.begin_change(&network);
                            host.connect(&network,[r.cell.0,r.cell.1,cell.0,cell.1],[0,0,1,-1]);
                            host.end_change(&network);
                            host.release(&network);
                        }
                    }
                }
            }
            r.attached = host.find_occupant(r.cell,kind.occupant_class());
        }
        if !matches!(r.attached,RefValue::Null) { host.refresh(&r.attached,true); }
        // ComputeBox is still explicitly unspecified in UG5/NO40. Continue
        // subscriptions, but expose that the creation geometry is incomplete.
        for id in CONNECTION_MESSAGES { host.message(id,true); }
        Err(ConnectionUnsupported::GeometryBoundsDrawTransformMessagesUG5NO40)
    }
    pub fn set_linked_occupant(&mut self, value: RefValue, host: &mut impl ConnectionHost) {
        if same_ref(&self.record.linked,&value) { return; }
        clear_ref(&mut self.record.linked,host); // Release old BEFORE store/AddRef new.
        self.record.linked = value;
        if matches!(self.record.linked,RefValue::Null) { host.remove_self(true,0); }
        else { host.add_ref(&self.record.linked); }
    }
    pub fn refresh_attached(&self, host: &mut impl ConnectionHost) {
        if !matches!(self.record.attached,RefValue::Null) { host.refresh(&self.record.attached,true); }
    }
    pub fn model_state_changed(&mut self) -> Result<(),ConnectionUnsupported> {
        if self.record.model_state { return Ok(()); }
        self.record.model_state = true;
        // Must compute bounds before renderer notification. Do not counterfeit
        // that owner computation with a foreign result callback (UG5/NO40).
        Err(ConnectionUnsupported::GeometryBoundsDrawTransformMessagesUG5NO40)
    }
    pub fn set_position(&mut self, position: [f32;3]) { self.record.position = position; }
    pub fn position(&self) -> [f32;3] { self.record.position }
    pub fn model_instances(&self, host: &mut impl ConnectionHost) -> [RefValue;1] {
        let instance = host.model_instance();
        host.add_ref(&instance);
        [instance]
    }
    pub fn query_occupant_interface(&self, kind: ConnectionKind, occupant: &RefValue, host: &mut impl ConnectionHost) -> RefValue {
        let matched = host.query_interface(occupant,kind.occupant_class());
        if matches!(matched,RefValue::Null) { return RefValue::Null; }
        let answer = host.query_interface(occupant,0x4A10_AFC2);
        host.release(&matched);
        answer
    }
    pub fn sort_value(&self, kind: ConnectionKind) -> u32 { kind.class_id() }
    pub fn post_occupant_message(&self, message: u32, value: u32, host: &mut impl ConnectionHost) {
        host.post_occupant_message(message,value);
        match message {
            0x99EF_1142 => host.post_occupant_message(0x8B87_A974,value),
            0x99EF_1143 => host.post_occupant_message(0x8B87_A975,value),
            _ => (),
        }
    }
    pub fn shutdown(&mut self, host: &mut impl ConnectionHost) {
        for slot in 0..3 { host.clear_texture_binding(slot); }
        for wall in &mut self.record.walls { wall.clear(); }
        clear_ref(&mut self.record.attached,host);
        clear_ref(&mut self.record.linked,host);
        for id in CONNECTION_MESSAGES { host.message(id,false); }
        host.occupant_shutdown(&mut self.record.base);
        self.initialized = false;
    }
}
