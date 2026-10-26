//! Settled NO40/UG5 owner work. The remaining NO40-C observation is explicit.
use super::*;
use crate::network_occupants::surface::normalize;

#[derive(Clone, Debug)]
pub struct ConnectionDescriptor {
    pub textures: [u32; 3],
    pub models: [u32; 2],
    /// Bounded descriptor input arrays, not a generated native mesh.
    pub vertices: Vec<Vertex>,
    pub upper: Vec<usize>,
    pub lower: Vec<usize>,
    pub view_rotation: bool,
    pub segment_mode: bool,
    pub lower_offset: f32,
}

pub trait ConnectionGeometryHost: ConnectionHost + ConnectionTextureHost {
    fn discrete_view_rotation(&mut self) -> u32;
    fn occupant_y(&mut self, occupant: &RefValue) -> f32;
    /// Returns an owned occupant, or null; a primitive spatial lookup.
    fn surface_occupant(&mut self, cell: (i32, i32)) -> RefValue;
    fn model_resource(&mut self, key: [u32; 3], iid: u32) -> RefValue;
    fn create_instance(&mut self, resource: &RefValue, index: u32) -> RefValue;
    fn assign_instance_owner(&mut self, instance: &RefValue);
    fn occupied_cell(&mut self, occupant: &RefValue) -> (i32, i32);
    fn inserted_attached_refresh(&mut self, attached: &RefValue, propagate: bool);
}

pub trait ConnectionDrawHost: ConnectionGeometryHost {
    /// The selected bounded shade input at B0CFB0+12*index (NO40-C).
    fn selected_shade(&mut self, index: i32) -> Option<[f32; 3]>;
    fn self_lit_shade(&mut self, shade: [f32; 3]);
    fn shade(&mut self, shade: [f32; 3]);
    fn reset_shade(&mut self);
    fn default_transform(&mut self);
    fn matrix(&mut self, matrix: [f32; 16]);
    fn state(&mut self, value: u32, mask: u32);
    fn bind_texture(&mut self, binding: &RefValue);
    fn wrap(&mut self, modes: [u32; 3]);
    fn filter(&mut self, modes: [u32; 3]);
    fn draw_vertices(&mut self, primitive: u32, format: u32, vertices: &[Vertex]);
    fn triangle_normal(&mut self, positions: [[f32; 3]; 3]) -> [f32; 3];
    fn light_normal(&mut self, normal: [f32; 3]) -> [f32; 3];
    fn instance_attributes(&mut self, instance: &RefValue) -> u32;
    fn mesh_collection(&mut self, instance: &RefValue, iid: u32) -> RefValue;
    fn collection_count(&mut self, collection: &RefValue) -> u32;
    /// Collection accessors borrow material and mesh pointers.
    fn material(&mut self, collection: &RefValue, index: u32) -> RefValue;
    fn mesh(&mut self, collection: &RefValue, index: u32) -> RefValue;
    fn material_flags(&mut self, material: &RefValue) -> u32;
    fn set_material_flags(&mut self, material: &RefValue, flags: u32);
    fn material_blend(&mut self, material: &RefValue, source: u8, destination: u8);
    fn model_shade(&mut self, instance: &RefValue);
    fn material_state(&mut self, material: &RefValue, attributes: u32);
    fn render_mesh(&mut self, mesh: &RefValue);
}

pub struct ConnectionMessage {
    pub id: u32,
    pub payload: RefValue,
    /// Exact primary-pointer comparison, performed by the native adapter.
    pub payload_is_primary: bool,
    /// Values at context+4, not pointer identities.
    pub context_words: [u32; 2],
}

impl UndergroundConnection {
    pub fn compute_box(&self) -> [[f32; 3]; 2] {
        let mut bounds = [self.record.position; 2];
        for (vertex, height) in self.surface.iter().zip(self.record.heights) {
            for i in 0..3 {
                if vertex.pos[i] < bounds[0][i] { bounds[0][i] = vertex.pos[i]; }
                let high = if i == 1 { height } else { vertex.pos[i] };
                if high > bounds[1][i] { bounds[1][i] = high; }
            }
        }
        bounds
    }

    pub fn create_geometry(&mut self, descriptor: &ConnectionDescriptor, host: &mut impl ConnectionGeometryHost) -> Result<(), ConnectionUnsupported> {
        // NO40-C establishes only the unrotated, unattached matrix case. Do not
        // accept an opaque native-owner matrix callback as its implementation.
        if descriptor.view_rotation || host.discrete_view_rotation() != 0 || !matches!(self.record.attached, RefValue::Null) {
            return Err(ConnectionUnsupported::RuntimeObservationNO40C);
        }
        self.matrices = [IDENTITY4; 2];
        for matrix in &mut self.matrices {
            matrix[3] = self.record.position[0]; matrix[7] = self.record.saved_170; matrix[11] = self.record.position[2];
        }
        self.create_piece_model(descriptor, host)?;
        self.create_walls(host)?;
        self.create_floor(host)
    }

    pub fn create_piece_model(&mut self, descriptor: &ConnectionDescriptor, host: &mut impl ConnectionGeometryHost) -> Result<(), ConnectionUnsupported> {
        if !matches!(self.record.attached, RefValue::Null) { return Err(ConnectionUnsupported::RuntimeObservationNO40C); }
        self.record.walls[6].clear();
        clear_ref(&mut self.bindings[2], host);
        for instance in &mut self.meshes { clear_ref(instance, host); }
        self.bindings[2] = host.request_texture_binding([0x7AB5_0E44, 0x891B_0E1A, descriptor.textures[2]]);
        let mut vertices = descriptor.vertices.clone();
        let surface = if matches!(self.record.linked, RefValue::Null) { host.surface_occupant(self.record.cell) } else { RefValue::Null };
        if !matches!(self.record.linked, RefValue::Null) { self.record.saved_170 = host.occupant_y(&self.record.linked); }
        else if !matches!(surface, RefValue::Null) { self.record.saved_170 = host.occupant_y(&surface); }
        if !matches!(surface, RefValue::Null) { host.release(&surface); }
        for (id, instance) in descriptor.models.into_iter().zip(&mut self.meshes) {
            let resource = host.model_resource([0x5AD0_E817, 0xBADB_57F1, id], 0x9AD0_E7E3);
            if !matches!(resource, RefValue::Null) {
                *instance = host.create_instance(&resource, 0);
                if !matches!(instance, RefValue::Null) { host.assign_instance_owner(instance); }
                host.release(&resource);
            }
        }
        for &i in &descriptor.upper { vertices[i].pos[1] = 0.0; }
        self.bottom_y = (f64::from(self.record.saved_170) - f64::from(host.cell_size())) as f32;
        let delta = (f64::from(self.bottom_y) - f64::from(self.record.saved_170)) as f32;
        for &i in &descriptor.lower { vertices[i].pos[1] = (f64::from(delta) - f64::from(descriptor.lower_offset)) as f32; }
        let mut low = [0.0f32; 2]; let mut high = [0.0f32; 2];
        for v in &vertices {
            for (i, value) in [v.pos[0], v.pos[2]].into_iter().enumerate() {
                if value < low[i] { low[i] = value; } if value > high[i] { high[i] = value; }
            }
        }
        let width = ((f64::from(high[0])-f64::from(low[0])).max(f64::from(high[1])-f64::from(low[1]))).abs();
        let reciprocal = 1.0 / width;
        let matrix = &mut self.matrices[1];
        matrix[3] = self.record.position[0]; matrix[7] = self.record.saved_170; matrix[11] = self.record.position[2]; matrix[15] = 1.0;
        let m = matrix.map(f64::from);
        for (i, vertex) in vertices.iter_mut().enumerate() {
            let [x,y,z] = vertex.pos.map(f64::from);
            vertex.u = if descriptor.segment_mode { (i / 2) as f32 } else { ((width*0.5+x)*reciprocal) as f32 };
            vertex.v = (y.abs()*reciprocal) as f32;
            vertex.color = u32::MAX;
            vertex.pos = [(((z*m[2]+y*m[1])+x*m[0])+m[3]) as f32, (((z*m[6]+y*m[5])+x*m[4])+m[7]) as f32, (((x*m[8]+z*m[10])+y*m[9])+m[11]) as f32];
        }
        self.record.walls[6] = vertices;
        for i in [1,0] {
            clear_ref(&mut self.bindings[i], host);
            self.bindings[i] = host.request_texture_binding([0x7AB5_0E44, 0x891B_0E1A, descriptor.textures[i]]);
        }
        self.model_state_changed(host)
    }

    pub fn do_message(&mut self, message: ConnectionMessage, kind: ConnectionKind, descriptor: &ConnectionDescriptor, host: &mut impl ConnectionGeometryHost) -> Result<bool, ConnectionUnsupported> {
        let payload = &message.payload;
        match message.id {
            0x8B87_A974 | 0x8B87_A975 if !matches!(payload, RefValue::Null) => {
                let queried = self.query_occupant_interface(kind, payload, host);
                if !matches!(queried, RefValue::Null) {
                    if message.id == 0x8B87_A974 {
                        if host.occupied_cell(&queried) == self.record.cell {
                            clear_ref(&mut self.record.attached, host);
                            self.record.attached = queried.clone(); host.add_ref(&queried);
                            let result = self.model_state_changed(host);
                            host.release(&queried); result?;
                        } else { host.release(&queried); }
                    } else {
                        if same_ref(&queried, &self.record.attached) {
                            clear_ref(&mut self.record.attached, host);
                            self.record.model_state = true;
                            self.bottom_y = (f64::from(self.record.position[1]) - f64::from(host.cell_size())) as f32;
                        }
                        host.release(&queried);
                    }
                } else if message.id == 0x8B87_A974 && message.payload_is_primary && !matches!(self.record.attached, RefValue::Null) {
                    host.inserted_attached_refresh(&self.record.attached, true);
                }
            }
            0x8B8B_2CEE if !matches!(payload, RefValue::Null) && same_ref(payload, &self.record.linked) => {
                clear_ref(&mut self.record.linked, host); host.remove_self(false, 0); self.refresh_attached(host); self.shutdown(host);
            }
            0xA6B7_9602 if message.context_words[0] != message.context_words[1] => self.create_geometry(descriptor, host)?,
            _ => {}
        }
        Ok(true)
    }

    fn draw_list(&self, binding: &RefValue, vertices: &[Vertex], host: &mut impl ConnectionDrawHost) {
        if matches!(binding, RefValue::Null) || vertices.is_empty() { return; }
        host.default_transform(); host.state(0x0200_01A7,0x1E00_03FF); host.bind_texture(binding); host.wrap([2,2,0]); host.filter([1,1,0]); host.draw_vertices(6,10,vertices);
    }

    pub fn draw(&mut self, pass: u32, self_lit: bool, descriptor: &ConnectionDescriptor, host: &mut impl ConnectionDrawHost) -> Result<u32, ConnectionUnsupported> {
        if self.record.model_state { self.create_geometry(descriptor,host)?; }
        if self_lit {
            if !self.record.walls[5].is_empty() { host.default_transform(); host.state(0x020A_8197,0x1E1F_E3FF); host.draw_vertices(1,10,&self.record.walls[5]); }
            return Ok(1);
        }
        if pass == 0 {
            self.draw_list(&self.bindings[1],&self.record.walls[4],host);
            for face in self.record.walls.iter().take(self.record.saved_68 as usize) { self.draw_list(&self.bindings[0],face,host); }
            return Ok(2);
        }
        if pass == 1 {
            let shade = host.selected_shade(self.record.saved_178).ok_or(ConnectionUnsupported::RuntimeObservationNO40C)?;
            host.self_lit_shade(shade);
            let vertices = &self.record.walls[6];
            if !matches!(self.bindings[2],RefValue::Null) && !vertices.is_empty() {
                host.default_transform(); host.state(0x0200_01A7,0x1E00_03FF); host.bind_texture(&self.bindings[2]); host.wrap([3,3,0]); host.filter([1,1,0]);
                if descriptor.segment_mode {
                    for start in (0..vertices.len().saturating_sub(2)).step_by(2) {
                        let window = &vertices[start..start+4];
                        let normal = normalize(host.triangle_normal([window[0].pos,window[1].pos,window[2].pos]));
                        let shade = host.light_normal(normal); host.shade(shade); host.draw_vertices(1,10,window);
                    }
                } else { host.draw_vertices(1,10,vertices); }
            }
            host.reset_shade();
            for (index,instance) in self.meshes.iter().enumerate() {
                if matches!(instance,RefValue::Null) { continue; }
                self.matrices[0][3]=self.record.position[0]; self.matrices[0][7]=if index==0 { self.record.saved_170 } else { self.bottom_y }; self.matrices[0][11]=self.record.position[2]; self.matrices[0][15]=1.0;
                host.matrix(self.matrices[0]); let attributes=host.instance_attributes(instance);
                let collection=host.mesh_collection(instance,0x26A6_8448);
                if !matches!(collection,RefValue::Null) {
                    for i in 0..host.collection_count(&collection) {
                        let material=host.material(&collection,i); let mesh=host.mesh(&collection,i);
                        let flags=host.material_flags(&material); host.set_material_flags(&material,flags|0x10); host.material_blend(&material,4,5);
                        host.model_shade(instance); host.material_state(&material,attributes); host.render_mesh(&mesh); host.reset_shade();
                    }
                    host.release(&collection);
                }
            }
            host.reset_shade();
        }
        self.record.model_state=false;
        Ok(1)
    }
}
