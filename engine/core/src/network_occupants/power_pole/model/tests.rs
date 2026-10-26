use super::*;

fn reference(token: u64) -> RefValue {
    RefValue::Host { token, class_id: 0 }
}
fn token(value: &RefValue) -> u64 {
    if let RefValue::Host { token, .. } = value {
        *token
    } else {
        0
    }
}

#[test]
fn occupant_bounds_conformance_54_55() {
    let mut p = PowerPole::fresh();
    p.fields.position = [24., 50., 40.];
    p.fields.model_y = 7.;
    assert_eq!(p.occupant_bounds([16.; 2]), [[16., 7., 32.], [32., 110., 48.]]);
    assert!(!p.stale);
    p.fields.connections.push(PoleConnection {
        words: [9, 9],
        ..Default::default()
    });
    p.stale = true;
    assert_eq!(p.occupant_bounds([16.; 2]), [[16., 7., 32.], [32., 110., 48.]]);
    p.fields.connections[0].words = [3, 1];
    p.fields.connections[0].vertices[0].push(vec![]);
    p.stale = true;
    assert_eq!(p.occupant_bounds([16.; 2]), [[16., 7., 16.], [64., 110., 48.]]);
    p.fields.connections[0].words = [99, 99];
    p.fields.position[1] = 60.;
    assert_eq!(p.occupant_bounds([16.; 2]), [[16., 7., 16.], [64., 120., 48.]]);
    p.fields.connections.clear();
    p.fields.position = [8., 50., 64.];
    p.stale = true;
    assert_eq!(p.occupant_bounds([8., 32.]), [[8., 7., 64.], [16., 110., 96.]]);
    p.fields.position = [24., 50., 40.];
    p.stale = true;
    p.fields.connections.push(PoleConnection {
        words: [u32::MAX, 2],
        vertices: [vec![vec![]], vec![]],
        ..Default::default()
    });
    let bounds = p.occupant_bounds([16.; 2]);
    assert_eq!((bounds[0][0], bounds[1][0]), (16., 0.));
}

#[test]
fn pole_pick_conformance_68() {
    struct Pick {
        seen: Vec<[[f32; 3]; 2]>,
    }
    impl PolePickHost for Pick {
        fn delegate_bounds(&mut self, _: &RefValue, out: &mut [[f32; 3]; 2]) {
            *out = [[0.; 3], [2.; 3]];
        }
        fn ray_box_distance(&mut self, bounds: [[f32; 3]; 2]) -> Option<f64> {
            self.seen.push(bounds);
            Some(4.)
        }
    }
    let mut p = PowerPole::fresh();
    p.fields.position = [10., 20., 30.];
    let mut host = Pick { seen: vec![] };
    let mut best = 5.;
    assert!(!p.pick(&mut best, &mut host));
    p.model = reference(1);
    p.geometry[0] = vec![Vertex {
        pos: [-999.; 3],
        ..Vertex::default()
    }];
    p.geometry[1] = vec![Vertex {
        pos: [15., 4., 31.],
        ..Vertex::default()
    }];
    p.fields.transform.translation = [999.; 3];
    assert!(p.pick(&mut best, &mut host));
    assert_eq!(host.seen[0], [[10., 4., 30.], [15., 22., 32.]]);
    assert_eq!(best, 4.);
    assert!(!p.pick(&mut best, &mut host));
    best = f32::NAN;
    assert!(!p.pick(&mut best, &mut host));
    let mut out = [[91.; 3]; 2];
    p.model = RefValue::Null;
    p.local_bounds(&mut out, &mut host);
    assert_eq!(out, [[91.; 3]; 2]);
}

struct Host {
    log: Vec<String>,
    exemplar: bool,
    base_exemplar: RefValue,
    resource: bool,
    instance: bool,
    property: Option<ResKey>,
    transform: Xform,
    texture_attempts: Vec<ResKey>,
    lights: Vec<[f32; 3]>,
    shades: Vec<[f32; 3]>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            log: vec![],
            exemplar: true,
            base_exemplar: RefValue::Null,
            resource: true,
            instance: true,
            property: None,
            transform: Xform {
                scale: 2.,
                counter: 200,
                ..Default::default()
            },
            texture_attempts: vec![],
            lights: vec![],
            shades: vec![],
        }
    }
}
impl References for Host {
    fn add_ref(&mut self, value: &RefValue) {
        self.log.push(format!("add:{}", token(value)));
    }
    fn release(&mut self, value: &RefValue) {
        self.log.push(format!("release:{}", token(value)));
    }
}

impl super::super::links::PoleLinkHost for Host {
    fn pole_cell(&mut self, _: &RefValue) -> (i32, i32) {
        (1, 0)
    }
    fn attachment_points(&mut self, _: &RefValue, _: u32) -> Result<[[f32; 3]; 4], PoleUnsupported> {
        Ok([[16., 0., 0.]; 4])
    }
    fn cell_size(&mut self) -> [f32; 2] {
        [16.; 2]
    }
    fn is_water(&mut self, _: (i32, i32)) -> bool {
        false
    }
    fn post_message(&mut self, _: u32, _: u32) {
        self.log.push("wire-message".into());
    }
}
impl PoleHost for Host {
    fn cell_size(&mut self) -> [f32; 2] {
        [8., 32.]
    }
    fn insert_line(&mut self, _: &RefValue, _: u32) {}
    fn remove_line(&mut self, _: &RefValue, _: bool, _: u32) {}
    fn reciprocal_remove(&mut self, _: &RefValue, _: &RefValue, _: bool) {
        self.log.push("reciprocal-remove".into());
    }
    fn post_message(&mut self, _: u32, _: u32) {}
    fn lines_to(&mut self, _: &RefValue, _: &RefValue, _: &mut Vec<(i32, i32)>) -> bool {
        false
    }
    fn create_effect(&mut self, _: &str) -> RefValue {
        RefValue::Null
    }
    fn place_effect(&mut self, _: &RefValue, _: [f32; 3], _: [f32; 9], _: f32) {}
    fn start_effect(&mut self, _: &RefValue) {}
    fn effect_running(&mut self, _: &RefValue) -> bool {
        false
    }
    fn stop_effect(&mut self, _: &RefValue, _: bool) {
        self.log.push("stop-effect".into());
    }
    fn subscription(&mut self, _: u32, subscribe: bool) {
        self.log.push(format!("subscribe:{subscribe}"));
    }
    fn occupant_shutdown(&mut self, _: &mut OccupantBase) {
        self.log.push("base-shutdown".into());
    }
}
impl PoleStaticHost for Host {
    fn acquire_resource_manager(&mut self) -> RefValue {
        RefValue::Null
    }
    fn acquire_texture_manager(&mut self, _: u32, _: u32) -> RefValue {
        RefValue::Null
    }
    fn register_shutdown(&mut self) {
        self.log.push("static".into());
    }
    fn control_exemplar(&mut self, _: ResKey) -> bool {
        false
    }
    fn float_property(&self, _: u32) -> Option<f32> {
        None
    }
    fn uint_property(&self, _: u32) -> Option<u32> {
        None
    }
    fn width_property(&self, _: u32) -> Option<[f32; 5]> {
        None
    }
    fn sint64_property(&self, _: u32) -> Option<i64> {
        None
    }
    fn pack(&mut self, _: crate::network_occupants::pylon::RenderStateArgs) -> u64 {
        0
    }
}
impl PoleLifecycleHost for Host {
    fn base_init(&mut self, base: &mut OccupantBase) {
        base.status |= 1;
        self.log.push("base-init".into());
    }
    fn view_zoom_rotation(&mut self) -> [u32; 2] {
        [3, 2]
    }
    fn current_exemplar(&mut self, _: &OccupantBase) -> RefValue {
        self.base_exemplar.clone()
    }
    fn assign_exemplar(&mut self, _: &mut OccupantBase, exemplar: &RefValue) {
        if !exemplar.is_null() {
            self.add_ref(exemplar);
        }
        let old = std::mem::replace(&mut self.base_exemplar, exemplar.clone());
        if !old.is_null() {
            self.release(&old);
        }
        self.log.push("exemplar".into());
    }
    fn exemplar_model_instance(&mut self, _: &OccupantBase) -> u32 {
        0x100
    }
    fn refresh_other_wires(&mut self, _: &RefValue, _: &RefValue) {
        self.log.push("other-refresh".into());
    }
}

#[test]
fn pole_init_guard_exemplar_refresh_and_destruction_conformance_71_73() {
    let mut p = PowerPole::fresh();
    let mut h = Host {
        resource: false,
        ..Default::default()
    };
    let mut statics = PoleStatics::new(0, 0);
    let tables = AttachmentTables::default();
    p.geometry[0].push(Vertex::default());
    assert_eq!(
        p.init(&reference(99), reference(10), 8, &mut statics, &tables, &mut h),
        Ok(true)
    );
    assert_eq!(statics.cell_size, [8., 32.]);
    assert_eq!(p.model_ids, [2, 3]);
    assert_eq!(p.wire_exemplar_index, 8);
    assert!(p.geometry.iter().all(Vec::is_empty));
    assert_eq!((p.bytes_54_55, p.scale_64), ([0; 2], 1.));
    assert!(p.subscribed);
    h.log.clear();
    p.init(&reference(99), reference(11), 1, &mut statics, &tables, &mut h)
        .unwrap();
    assert!(h.log.is_empty());
    p.set_default_exemplar(&reference(99), reference(10), &statics, &tables, &mut h)
        .unwrap();
    assert!(h.log.is_empty());
    p.fields.connections.push(PoleConnection {
        pole: reference(3),
        ..Default::default()
    });
    p.set_default_exemplar(&reference(99), reference(11), &statics, &tables, &mut h)
        .unwrap();
    assert_eq!(&h.log[..2], ["add:11", "release:10"]);
    assert_eq!(h.log.last().unwrap(), "other-refresh");
    p.model = reference(80);
    p.effect = reference(81);
    h.log.clear();
    p.destroy_resources(&mut h);
    assert_eq!(h.log, ["release:80", "release:81", "release:3"]);
    assert_eq!(token(&h.base_exemplar), 11); // Released by base destruction, not twice here.
}

#[test]
fn unchanged_exemplar_uses_base_holder_identity_including_loaded_state() {
    let mut p = PowerPole::fresh();
    let mut host = Host {
        base_exemplar: reference(10),
        resource: false,
        ..Default::default()
    };
    let statics = PoleStatics::new(0, 0);
    let tables = AttachmentTables::default();
    p.set_default_exemplar(&reference(99), reference(10), &statics, &tables, &mut host)
        .unwrap();
    assert!(host.log.is_empty());
    // A base Read can replace the holder without going through a pole setter.
    host.base_exemplar = reference(12);
    p.set_default_exemplar(&reference(99), reference(10), &statics, &tables, &mut host)
        .unwrap();
    assert_eq!(&host.log[..3], ["add:10", "release:12", "exemplar"]);
    assert!(host.log.iter().any(|line| line == "load"));
}

#[test]
fn pole_position_updates_delegate_but_keeps_own_transform_and_clean_bounds() {
    let mut p = PowerPole::fresh();
    // §4.3 Init establishes the cached view before the tool places the pole.
    p.model_ids = [2, 3];
    p.model = reference(80);
    p.fields.transform.translation = [91.; 3];
    p.stale = false;
    p.bounds_cells = [1, 2, 3, 4];
    let mut host = Host::default();
    p.set_position_with_model([10., 20., 30.], 16., Some([7., 8., 9., 10.]), &mut host)
        .unwrap();
    assert_eq!(host.transform.translation, [11., 22., 33.]);
    assert_eq!(p.fields.transform.translation, [91.; 3]);
    assert_eq!(p.fields.model_y, 7.);
    assert_eq!(p.bounds_cells, [1, 2, 3, 4]);
    assert!(!p.stale);
    assert_eq!(p.geometry.map(|v| v.len()), [4; 3]);
}
impl PoleModelHost for Host {
    fn has_exemplar(&mut self, _: &OccupantBase) -> bool {
        self.exemplar
    }
    fn model_key(&mut self, _: &OccupantBase) -> Option<ResKey> {
        self.property
    }
    fn model_instance_id(&mut self, zoom: i32, rotation: i32, alternate: bool) -> u32 {
        self.log.push(format!("id:{zoom}:{rotation}:{alternate}"));
        42
    }
    fn load_resource(&mut self, key: ResKey, iid: u32) -> RefValue {
        assert_eq!(iid, 0x9AD0_E7E3);
        assert_eq!(key.instance, 42);
        if let Some(property) = self.property {
            assert_eq!((key.type_id, key.group), (property.type_id, property.group));
        } else {
            assert_eq!((key.type_id, key.group), (0x5AD0_E817, 0xBADB_57F1));
        }
        self.log.push("load".into());
        if self.resource {
            reference(70)
        } else {
            RefValue::Null
        }
    }
    fn create_instance(&mut self, resource: &RefValue, argument: u32) -> RefValue {
        assert_eq!((token(resource), argument), (70, 0));
        self.log.push("create".into());
        if self.instance {
            reference(80)
        } else {
            RefValue::Null
        }
    }
    fn position_offset(&mut self, _: &OccupantBase) -> [f32; 3] {
        [1., 2., 3.]
    }
    fn get_transform(&mut self, _: &RefValue) -> Xform {
        self.transform
    }
    fn set_transform(&mut self, _: &RefValue, transform: Xform) {
        self.transform = transform;
        self.log.push("transform".into());
    }
    fn set_owner(&mut self, _: &RefValue, occupant: &RefValue) {
        assert_eq!(token(occupant), 99);
        self.log.push("owner".into());
    }
    fn view_yaw(&mut self) -> f32 {
        0.
    }
    fn diagonal_model(&mut self, _: &OccupantBase) -> bool {
        false
    }
    fn load_texture(&mut self, key: ResKey) -> RefValue {
        assert_eq!((key.type_id, key.group), (0x7AB5_0E44, 0x1ABE_787D));
        self.texture_attempts.push(key);
        // Every first attempt fails, so the repeated key must be exact.
        if self.texture_attempts.len() % 2 == 1 {
            RefValue::Null
        } else {
            reference(u64::from(key.instance))
        }
    }
}
impl PoleRenderHost for Host {
    fn delegate_draw(&mut self, instance: &RefValue, attributes: [u8; 2], opacity: f32) {
        assert_eq!(token(instance), 80);
        assert_eq!(attributes, [1, 0]);
        assert_eq!(opacity, 1.);
        self.log.push("delegate:false".into());
    }
    fn highlight_tint(&mut self, index: u8) -> [f32; 3] {
        assert_eq!(index, 0);
        [0.5, 0.25, 0.125]
    }
    fn set_pole_state(&mut self) {
        self.log.push("state".into());
    }
    fn default_model_transform(&mut self) {
        self.log.push("default-transform".into());
    }
    fn triangle_normal(&mut self, _: [[f32; 3]; 3]) -> [f32; 3] {
        [0., 3., 4.]
    }
    fn light(&mut self, normal: [f32; 3], position: [f32; 3]) -> [f32; 3] {
        assert_eq!(normal, [0., 0.6, 0.8]);
        self.lights.push(position);
        [self.lights.len() as f32, 2., 4.]
    }
    fn set_shade(&mut self, rgb: [f32; 3]) {
        self.shades.push(rgb);
        self.log.push("shade".into());
    }
    fn reset_shade(&mut self) {
        self.log.push("reset".into());
    }
    fn bind_texture(&mut self, id: u32, _: &RefValue, stage: u32) {
        assert_eq!(stage, 0);
        self.log.push(format!("bind:{id:08X}"));
    }
    fn filtering(&mut self, min: u32, mag: u32, stage: u32) {
        assert_eq!((min, mag, stage), (1, 1, 0));
        self.log.push("filter".into());
    }
    fn wrap(&mut self, u: u32, v: u32, stage: u32) {
        assert_eq!((u, v, stage), (u, u, 0));
        self.log.push(format!("wrap:{u}"));
    }
    fn draw_face(&mut self, primitive: u32, format: u32, vertices: &[Vertex]) {
        assert_eq!((primitive, format, vertices.len()), (6, 10, 4));
        self.log.push("face".into());
    }
    fn draw_wire(&mut self, primitive: u32, format: u32, _: &[[f32; 5]; 4]) {
        assert_eq!((primitive, format), (6, 2));
        self.log.push("wire".into());
    }
}

fn pole() -> PowerPole {
    let mut p = PowerPole::fresh();
    p.fields.position = [10., 20., 30.];
    p.model_ids = [2, 3];
    p.wire_exemplar_index = 8;
    p.model = reference(60);
    p
}

#[test]
fn model_resource_failure_preserves_old_delegate_success_releases_in_order() {
    let mut p = pole();
    let mut h = Host {
        exemplar: false,
        ..Default::default()
    };
    assert!(!p.load_model(&reference(99), &mut h));
    assert_eq!(token(&p.model), 60);
    assert!(h.log.is_empty());
    h.exemplar = true;
    h.resource = false;
    assert!(!p.load_model(&reference(99), &mut h));
    assert_eq!(token(&p.model), 60);
    assert_eq!(h.log, ["id:3:2:true", "load"]);
    h.log.clear();
    h.resource = true;
    h.property = Some(ResKey {
        type_id: 123,
        group: 456,
        instance: 999,
    });
    assert!(!p.load_model(&reference(99), &mut h));
    assert_eq!(
        h.log,
        [
            "id:3:2:true",
            "load",
            "release:60",
            "create",
            "transform",
            "owner",
            "transform",
            "release:70"
        ]
    );
    assert_eq!(token(&p.model), 80);
    assert_eq!(h.transform.translation, [11., 22., 33.]);
    assert_eq!(h.transform.scale, 2.);
    assert_eq!(h.transform.mask, 5);
    assert_eq!(h.transform.counter, 202);
    assert_eq!(h.transform.matrix, [1., 0., 0., 0., 1., 0., 0., 0., 1.]);
    h.log.clear();
    h.instance = false;
    p.load_model(&reference(99), &mut h);
    assert!(matches!(p.model, RefValue::Null));
    assert_eq!(h.log, ["id:3:2:true", "load", "release:80", "create", "release:70"]);
}

#[test]
fn own_transform_and_model_query_do_not_move_position_or_delegate() {
    let mut p = pole();
    let mut h = Host::default();
    p.fields.transform.counter = 255;
    let input = Xform {
        counter: 77,
        translation: [1., 2., 3.],
        scale: 9.,
        mask: 3,
        ..Default::default()
    };
    p.set_transform(input);
    assert_eq!(p.fields.transform.counter, 0);
    let mut output = Xform {
        counter: 255,
        ..Default::default()
    };
    p.get_transform(&mut output);
    assert_eq!(output.counter, 0);
    assert_eq!(output.translation, input.translation);
    assert_eq!(p.fields.position, [10., 20., 30.]);
    assert!(h.log.is_empty());
    let instances = p.model_instances(&reference(99), &mut h);
    assert_eq!(token(&instances[0]), 99);
    assert_eq!(p.fields.transform.translation, p.fields.position);
    assert_eq!(p.fields.transform.counter, 1);
    assert_eq!(h.log, ["add:99"]);
    p.set_highlight(0x100);
    assert_eq!(p.bytes_54_55, [1, 0]);
    assert_eq!(p.base.highlight, 0);
}

#[test]
fn zoom_always_rebuilds_model_and_geometry_but_textures_only_on_shared_change() {
    let mut p = pole();
    let mut h = Host::default();
    let mut textures = PoleTextures::new(1);
    textures.bindings = std::array::from_fn(|i| reference(i as u64 + 10));
    let context = PoleDrawContext {
        zoom: 3,
        rotation: 2,
        highlight: false,
    };
    p.change_zoom_level(&context, &reference(99), &mut textures, &mut h)
        .unwrap();
    assert_eq!(textures.zoom, 3);
    assert_eq!(h.texture_attempts.len(), 8);
    for (attempts, id) in h.texture_attempts.chunks(2).zip(TEXTURE_IDS) {
        assert_eq!(attempts[0], attempts[1]);
        assert_eq!(attempts[0].instance, id);
    }
    assert!(p.geometry.iter().all(|v| v.len() == 4));
    p.geometry[0].clear();
    h.log.clear();
    p.change_zoom_level(&context, &reference(99), &mut textures, &mut h)
        .unwrap();
    assert_eq!(h.texture_attempts.len(), 8);
    assert!(h.log.iter().any(|v| v == "load"));
    assert_eq!(p.geometry[0].len(), 4);
}

#[test]
fn draw_has_three_lighting_queries_two_shade_resets_and_inherited_wire_state() {
    let mut p = pole();
    let mut h = Host::default();
    let mut textures = PoleTextures::new(3);
    p.model = reference(80);
    p.set_highlight(0x100);
    p.rebuild_geometry(false, 2).unwrap();
    let mut connection = PoleConnection {
        flags: [0, 1],
        ..Default::default()
    };
    connection.vertices[0] = vec![vec![[0., 0., 0.], [8., 0., 0.], [16., 0., 0.]]];
    connection.vertices[1] = vec![vec![[0., 0., 0.]]]; // Bind even though this emits no squares.
    p.fields.connections.push(connection);
    let context = PoleDrawContext {
        zoom: 3,
        rotation: 2,
        highlight: true,
    };
    assert_eq!(
        p.draw(&context, &reference(99), &[0.3; 5], &mut textures, &mut h),
        Ok(true)
    );
    assert_eq!(
        h.log,
        [
            "delegate:false",
            "state",
            "default-transform",
            "shade",
            "bind:0912220E",
            "filter",
            "wrap:2",
            "face",
            "reset",
            "shade",
            "bind:08080004",
            "filter",
            "wrap:3",
            "face",
            "shade",
            "face",
            "reset",
            "bind:AA9DA78E",
            "wire",
            "wire",
            "bind:C9AF0FCD"
        ]
    );
    assert_eq!(h.lights, p.geometry.iter().map(|face| face[0].pos).collect::<Vec<_>>());
    assert_eq!(h.shades, [[0.5, 0.5, 0.5], [1., 0.5, 0.5], [1.5, 0.5, 0.5]]);
}

#[test]
fn failed_model_load_does_not_suppress_faces_and_null_plus_zoom_change_loads_twice() {
    let mut p = pole();
    let mut h = Host {
        resource: false,
        ..Default::default()
    };
    let mut textures = PoleTextures::new(2);
    p.model = RefValue::Null;
    let context = PoleDrawContext {
        zoom: 3,
        rotation: 1,
        highlight: false,
    };
    assert_eq!(
        p.draw(&context, &reference(99), &[0.; 5], &mut textures, &mut h),
        Ok(true)
    );
    assert_eq!(h.log.iter().filter(|v| *v == "load").count(), 2);
    assert!(!h.log.iter().any(|v| v == "delegate:false"));
    assert_eq!(h.lights.len(), 3);
    assert_eq!(p.model_ids, [1, 3]);
}
