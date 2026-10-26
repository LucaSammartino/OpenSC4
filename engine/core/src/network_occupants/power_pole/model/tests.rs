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

struct Host {
    log: Vec<String>,
    exemplar: bool,
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
