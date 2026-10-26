use super::*;

fn descriptor() -> ConnectionDescriptor {
    ConnectionDescriptor {
        textures: [1, 2, 3],
        models: [9, 0],
        vertices: [-8.0, 8.0]
            .map(|x| Vertex {
                pos: [x, 7.0, 0.0],
                ..Default::default()
            })
            .to_vec(),
        upper: vec![0],
        lower: vec![1],
        view_rotation: false,
        segment_mode: false,
        lower_offset: 3.0,
    }
}
impl ConnectionGeometryHost for ConnHost {
    fn discrete_view_rotation(&mut self) -> u32 {
        self.h.log.push("view-rotation".into());
        self.view_rotation
    }
    fn occupant_y(&mut self, _: &RefValue) -> f32 {
        100.0
    }
    fn surface_occupant(&mut self, _: (i32, i32)) -> RefValue {
        RefValue::Null
    }
    fn model_resource(&mut self, key: [u32; 3], iid: u32) -> RefValue {
        assert_eq!((key[0], key[1], iid), (0x5AD0_E817, 0xBADB_57F1, 0x9AD0_E7E3));
        self.h.log.push(format!("resource/{}", key[2]));
        reference(400 + u64::from(key[2]))
    }
    fn create_instance(&mut self, resource: &RefValue, index: u32) -> RefValue {
        assert_eq!(index, 0);
        reference(token(resource) + 100)
    }
    fn assign_instance_owner(&mut self, instance: &RefValue) {
        self.h.log.push(format!("owner/{}", token(instance)));
    }
    fn occupied_cell(&mut self, _: &RefValue) -> (i32, i32) {
        self.queried_cell
    }
    fn inserted_attached_refresh(&mut self, value: &RefValue, propagate: bool) {
        assert!(propagate);
        self.h.log.push(format!("inserted-refresh/{}", token(value)));
    }
}
impl ConnectionDrawHost for ConnHost {
    fn selected_shade(&mut self, _: i32) -> Option<[f32; 3]> {
        Some([0.25, 0.5, 0.75])
    }
    fn self_lit_shade(&mut self, shade: [f32; 3]) {
        assert_eq!(shade, [0.25, 0.5, 0.75]);
        self.h.log.push("self-lit-shade".into());
    }
    fn shade(&mut self, shade: [f32; 3]) {
        assert_eq!(shade, [0.2, 0.4, 0.6]);
        self.h.log.push("shade".into());
    }
    fn reset_shade(&mut self) {
        self.h.log.push("reset-shade".into());
    }
    fn default_transform(&mut self) {
        self.h.log.push("default-transform".into());
    }
    fn matrix(&mut self, m: [f32; 16]) {
        self.h.log.push(format!("matrix/{}", m[7]));
    }
    fn state(&mut self, value: u32, mask: u32) {
        self.h.log.push(format!("state/{value:x}/{mask:x}"));
    }
    fn bind_texture(&mut self, value: &RefValue) {
        self.h.log.push(format!("bind/{}", token(value)));
    }
    fn wrap(&mut self, modes: [u32; 3]) {
        assert!(modes == [2, 2, 0] || modes == [3, 3, 0]);
    }
    fn filter(&mut self, modes: [u32; 3]) {
        assert_eq!(modes, [1, 1, 0]);
    }
    fn draw_vertices(&mut self, primitive: u32, format: u32, vertices: &[Vertex]) {
        assert_eq!(format, 10);
        self.draws.push((primitive, vertices.to_vec()));
    }
    fn triangle_normal(&mut self, _: [[f32; 3]; 3]) -> [f32; 3] {
        [0.0, 2.0, 0.0]
    }
    fn light_normal(&mut self, normal: [f32; 3]) -> [f32; 3] {
        assert_eq!(normal, [0.0, 1.0, 0.0]);
        [0.2, 0.4, 0.6]
    }
    fn instance_attributes(&mut self, _: &RefValue) -> u32 {
        17
    }
    fn mesh_collection(&mut self, instance: &RefValue, iid: u32) -> RefValue {
        assert_eq!(iid, 0x26A6_8448);
        reference(token(instance) + 1000)
    }
    fn collection_count(&mut self, collection: &RefValue) -> u32 {
        u32::from(token(collection) == 1509)
    }
    fn material(&mut self, _: &RefValue, index: u32) -> RefValue {
        assert_eq!(index, 0);
        reference(1600)
    }
    fn mesh(&mut self, _: &RefValue, index: u32) -> RefValue {
        assert_eq!(index, 0);
        reference(1700)
    }
    fn material_flags(&mut self, _: &RefValue) -> u32 {
        3
    }
    fn set_material_flags(&mut self, _: &RefValue, flags: u32) {
        assert_eq!(flags, 0x13);
        self.h.log.push("material-flags".into());
    }
    fn material_blend(&mut self, _: &RefValue, source: u8, destination: u8) {
        assert_eq!((source, destination), (4, 5));
        self.h.log.push("blend".into());
    }
    fn model_shade(&mut self, _: &RefValue) {
        self.h.log.push("model-shade".into());
    }
    fn material_state(&mut self, _: &RefValue, attributes: u32) {
        assert_eq!(attributes, 17);
        self.h.log.push("material-state".into());
    }
    fn render_mesh(&mut self, _: &RefValue) {
        self.h.log.push("render-mesh".into());
    }
}

#[test]
fn descriptor_deformation_bounds_and_latch_worked_cases() {
    let mut c = connection();
    c.record.position = [100.0, 100.0, 200.0];
    c.record.saved_170 = 100.0;
    c.record.model_state = false;
    c.record.base.flags |= 1;
    let mut h = ConnHost::default();
    let mut d = descriptor();
    c.create_geometry(&d, &mut h).unwrap();
    let vertices = &c.record.walls[6];
    assert_eq!(
        vertices.iter().map(|v| v.pos).collect::<Vec<_>>(),
        [[92.0, 100.0, 200.0], [108.0, 81.0, 200.0]]
    );
    assert_eq!(
        vertices.iter().map(|v| (v.u, v.v, v.color)).collect::<Vec<_>>(),
        [(0.0, 0.0, u32::MAX), (1.0, 1.1875, u32::MAX)]
    );
    assert!(h.h.log.contains(&"resource/0".into()));
    assert_eq!(h.h.log.iter().filter(|s| *s == "bounds-changed").count(), 1);
    d.segment_mode = true;
    c.create_geometry(&d, &mut h).unwrap();
    assert!(c.record.walls[6].iter().all(|v| v.u == 0.0));
    c.record.position = [8.0, 100.0, 8.0];
    c.record.heights = [100.0, 101.0, 102.0, 103.0];
    c.surface = std::array::from_fn(|i| Vertex {
        pos: [
            if i >= 2 { 16.0 } else { 0.0 },
            80.0 + i as f32,
            if i == 1 || i == 2 { 16.0 } else { 0.0 },
        ],
        ..Default::default()
    });
    assert_eq!(c.compute_box(), [[0.0, 80.0, 0.0], [16.0, 103.0, 16.0]]);
    c.record.walls[6][0].pos = [-1000.0; 3];
    c.bottom_y = -9000.0;
    assert_eq!(c.compute_box(), [[0.0, 80.0, 0.0], [16.0, 103.0, 16.0]]);
    d.view_rotation = true;
    assert_eq!(
        c.create_geometry(&d, &mut h),
        Err(ConnectionUnsupported::RuntimeObservationNO40C)
    );
    d.view_rotation = false;
    c.record.attached = reference(1);
    assert_eq!(
        c.create_geometry(&d, &mut h),
        Err(ConnectionUnsupported::RuntimeObservationNO40C)
    );
}
fn message(id: u32, payload: RefValue) -> ConnectionMessage {
    ConnectionMessage {
        id,
        payload,
        payload_is_primary: false,
        context_words: [7, 7],
    }
}

#[test]
fn connection_attachment_and_removal_messages_conformance_60_61() {
    let mut c = connection();
    c.record.model_state = true;
    let mut h = ConnHost {
        queried_cell: c.record.cell,
        ..Default::default()
    };
    let d = descriptor();
    for _ in 0..2 {
        h.h.log.clear();
        assert_eq!(
            c.do_message(message(0x8B87_A974, reference(4)), ConnectionKind::Pipe, &d, &mut h),
            Ok(true)
        );
        assert_eq!(token(&c.record.attached), 300);
        assert!(!h.h.log.contains(&"bounds-changed".into()));
        assert!(h.h.log.ends_with(&["add300".into(), "release300".into()]));
    }
    assert_eq!(h.h.log.iter().filter(|s| *s == "release300").count(), 3); // two QI temporaries plus stored ref
    h.queried_cell = (999, 999);
    h.h.log.clear();
    c.do_message(message(0x8B87_A974, reference(5)), ConnectionKind::Pipe, &d, &mut h)
        .unwrap();
    assert_eq!(token(&c.record.attached), 300);
    assert!(!h.h.log.contains(&"add300".into()));
    h.query_fail = true;
    let mut own = message(0x8B87_A974, reference(999));
    own.payload_is_primary = true;
    c.do_message(own, ConnectionKind::Pipe, &d, &mut h).unwrap();
    assert_eq!(h.h.log.last().unwrap(), "inserted-refresh/300");
    h.query_fail = false;
    c.record.position[1] = 40.0;
    c.record.saved_170 = 91.0;
    h.h.log.clear();
    c.do_message(message(0x8B87_A975, reference(4)), ConnectionKind::Pipe, &d, &mut h)
        .unwrap();
    assert!(matches!(c.record.attached, RefValue::Null));
    assert_eq!(
        (c.bottom_y, c.record.saved_170, c.record.model_state),
        (24.0, 91.0, true)
    );
    assert!(!h.h.log.contains(&"view-rotation".into()));
    c.record.linked = reference(77);
    c.record.attached = reference(33);
    h.h.log.clear();
    c.do_message(message(0x8B8B_2CEE, reference(78)), ConnectionKind::Pipe, &d, &mut h)
        .unwrap();
    assert!(h.h.log.is_empty());
    c.do_message(message(0x8B8B_2CEE, reference(77)), ConnectionKind::Pipe, &d, &mut h)
        .unwrap();
    assert!(h.h.log.starts_with(&[
        "release77".into(),
        "remove-self-without-destroy".into(),
        "refresh33".into()
    ]));
    assert_eq!(h.h.log.last().unwrap(), "shutdown");
    h.h.log.clear();
    c.do_message(message(0xA6B7_9602, RefValue::Null), ConnectionKind::Pipe, &d, &mut h)
        .unwrap();
    assert!(h.h.log.is_empty());
    let mut changed = message(0xA6B7_9602, RefValue::Null);
    changed.context_words = [1, 2];
    c.do_message(changed, ConnectionKind::Pipe, &d, &mut h).unwrap();
    assert_eq!(h.h.log.iter().filter(|s| *s == "view-rotation").count(), 1);
    assert_eq!(
        c.do_message(message(42, RefValue::Null), ConnectionKind::Pipe, &d, &mut h),
        Ok(true)
    );
}

#[test]
fn connection_draw_pass_latch_windows_and_material_replay_worked_case() {
    let mut c = connection();
    c.record.model_state = true;
    let mut d = descriptor();
    d.vertices.clear();
    d.upper.clear();
    d.lower.clear();
    let mut h = ConnHost::default();
    for _ in 0..2 {
        assert_eq!(c.draw(0, false, &d, &mut h), Ok(2));
        assert!(c.record.model_state);
    }
    assert_eq!(h.h.log.iter().filter(|s| *s == "view-rotation").count(), 2);
    c.draw(1, false, &d, &mut h).unwrap();
    assert!(!c.record.model_state);
    assert!(h.h.log.contains(&"release1509".into()));
    assert!(h.h.log.contains(&"release1500".into()));
    let material_order = [
        "material-flags",
        "blend",
        "model-shade",
        "material-state",
        "render-mesh",
        "reset-shade",
    ];
    assert!(h
        .h
        .log
        .windows(material_order.len())
        .any(|w| w.iter().map(String::as_str).eq(material_order)));
    h.h.log.clear();
    c.draw(1, false, &d, &mut h).unwrap();
    assert!(!h.h.log.contains(&"view-rotation".into()));
    c.record.model_state = true;
    c.draw(0, true, &d, &mut h).unwrap();
    assert!(c.record.model_state);
    c.record.model_state = false;
    c.record.walls[6] = (0..10)
        .map(|i| Vertex {
            pos: [i as f32, 0.0, 0.0],
            ..Default::default()
        })
        .collect();
    c.bindings[2] = reference(9);
    c.meshes = [RefValue::Null, RefValue::Null];
    d.segment_mode = true;
    h.draws.clear();
    c.draw(1, false, &d, &mut h).unwrap();
    assert_eq!(
        h.draws.iter().map(|(_, v)| v[0].pos[0]).collect::<Vec<_>>(),
        [0.0, 2.0, 4.0, 6.0]
    );
    assert!(h.draws.iter().all(|(p, v)| *p == 1 && v.len() == 4));
    d.segment_mode = false;
    h.draws.clear();
    c.draw(1, false, &d, &mut h).unwrap();
    assert_eq!(h.draws.len(), 1);
    assert_eq!(h.draws[0].1.len(), 10);
}
