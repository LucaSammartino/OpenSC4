use super::*;
use crate::stream::{ByteReader, ByteWriter};

fn token(id: u64) -> RefValue {
    RefValue::Host { token: id, class_id: 0 }
}

fn id(value: &RefValue) -> u64 {
    match value {
        RefValue::Host { token, .. } => *token,
        _ => 0,
    }
}

#[derive(Debug, PartialEq)]
enum Event {
    State((u32, u32)),
    Bind(u64),
    Vertices(Vec<Vertex>),
    Triangle([usize; 3]),
    Shade(Option<[f32; 3]>),
    Transform,
    Wrap([u32; 3]),
    Filter([u32; 3]),
}

#[derive(Default)]
struct Host {
    refs: Vec<(bool, u64)>,
    loads: Vec<ResKey>,
    missing: Vec<u32>,
    events: Vec<Event>,
    normals: Vec<[f32; 3]>,
    corner_queries: usize,
    init_ok: bool,
    init_calls: usize,
    registered: usize,
    retrofit: usize,
    usable: Vec<bool>,
    changes: usize,
    shutdown: usize,
    ray: Option<f32>,
    picked_box: Option<Bounds>,
    water: bool,
    path_fail: bool,
    path_loads: Vec<ResKey>,
    path_calls: Vec<&'static str>,
    clip: Option<([f32; 3], [f32; 4])>,
}

impl References for Host {
    fn add_ref(&mut self, value: &RefValue) {
        self.refs.push((true, id(value)));
    }
    fn release(&mut self, value: &RefValue) {
        self.refs.push((false, id(value)));
    }
}

impl PieceHost for Host {
    fn post_usable_change(&mut self, usable: bool) {
        self.usable.push(usable);
    }
    fn set_translucency(&mut self, _: bool, _: u8) {}
    fn build_path_info(&mut self) {
        panic!("the integrated owner builds its own path info");
    }
    fn add_to_retrofit_list(&mut self) {
        self.retrofit += 1;
    }
    fn occupant_init(&mut self) {}
    fn occupant_shutdown(&mut self) {
        self.shutdown += 1;
    }
    fn post_piece_changed(&mut self) {
        self.changes += 1;
    }
}

impl NetworkHost for Host {
    fn initialize_occupant(&mut self) -> bool {
        self.init_calls += 1;
        self.init_ok
    }
    fn post_translucency_change(&mut self) {
        self.changes += 1;
    }
    fn ray_box_distance(&mut self, bounds: Bounds) -> Option<f32> {
        self.picked_box = Some(bounds);
        self.ray
    }
}

impl NetworkTextureHost for Host {
    fn acquire_resource_manager(&mut self) -> RefValue {
        token(1)
    }
    fn acquire_texture_manager(&mut self, service: u32, iid: u32) -> RefValue {
        assert_eq!((service, iid), (0x1AC0_E11A, 0xFAC0_E219));
        token(2)
    }
    fn register_static_shutdown(&mut self) {
        self.registered += 1;
    }
    fn load_texture(&mut self, manager: &RefValue, key: ResKey) -> RefValue {
        assert_eq!(id(manager), 2);
        assert_eq!((key.type_id, key.group), (0x7AB5_0E44, 0x1ABE_787D));
        self.loads.push(key);
        if self.missing.contains(&key.instance) {
            RefValue::Null
        } else {
            token(u64::from(key.instance))
        }
    }
}

impl surface::PartLighting for Host {
    fn triangle_normal(&mut self, p: [[f32; 3]; 3]) -> [f32; 3] {
        let a: [f32; 3] = std::array::from_fn(|i| p[1][i] - p[0][i]);
        let b: [f32; 3] = std::array::from_fn(|i| p[2][i] - p[0][i]);
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
    fn rgb(&mut self, _: [f32; 3], normal: [f32; 3]) -> [u8; 3] {
        self.normals.push(normal);
        [17, 34, 51]
    }
}

impl NetworkDrawHost for Host {
    fn corner_rgb(&mut self, _: [f32; 3]) -> [u8; 3] {
        self.corner_queries += 1;
        [3, 5, 7]
    }
    fn set_state(&mut self, state: (u32, u32)) {
        self.events.push(Event::State(state));
    }
    fn set_model_transform(&mut self, _: Xform) {
        self.events.push(Event::Transform);
    }
    fn set_shade(&mut self, shade: Option<[f32; 3]>) {
        self.events.push(Event::Shade(shade));
    }
    fn bind_texture(&mut self, value: &RefValue) {
        self.events.push(Event::Bind(id(value)));
    }
    fn wrap(&mut self, modes: [u32; 3]) {
        self.events.push(Event::Wrap(modes));
    }
    fn filtering(&mut self, modes: [u32; 3]) {
        self.events.push(Event::Filter(modes));
    }
    fn vertices(&mut self, vertices: &[Vertex], format: u32) {
        assert_eq!(format, 10);
        self.events.push(Event::Vertices(vertices.to_vec()));
    }
    fn triangle(&mut self, indices: [usize; 3]) {
        self.events.push(Event::Triangle(indices));
    }
    fn water_visible(&mut self) -> bool {
        self.water
    }
    fn water_level(&mut self, _: f32, _: f32) -> f32 {
        10.0
    }
}

impl NetworkPathHost for Host {
    fn map_path(&mut self, path: u32) -> u32 {
        path + 1
    }
    fn load_path(&mut self, key: ResKey, iid: u32) -> RefValue {
        assert_eq!(iid, 0xA966_7793);
        self.path_loads.push(key);
        if self.path_fail {
            RefValue::Null
        } else {
            token(10)
        }
    }
    fn uses_elevation(&mut self, _: &RefValue) -> bool {
        self.path_calls.push("uses");
        false
    }
    fn clone_path(&mut self, _: &RefValue) -> RefValue {
        self.path_calls.push("clone");
        token(11)
    }
    fn set_elevation(&mut self, _: &RefValue) {
        self.path_calls.push("elevation");
    }
    fn transformed_copy(&mut self, _: &RefValue, occupant: &NetworkOccupant) -> RefValue {
        assert_eq!(occupant.piece.orientation, 0x81);
        self.path_calls.push("transform");
        token(12)
    }
    fn offset_and_scissor(&mut self, _: &RefValue, offset: [f32; 3], cell: [f32; 4]) -> RefValue {
        self.path_calls.push("scissor");
        self.clip = Some((offset, cell));
        token(13)
    }
    fn remove_wrong_way(&mut self, _: &RefValue, directions: u8) {
        assert_eq!(directions, 1);
        self.path_calls.push("wrong-way");
    }
    fn verify_edges(&mut self, _: &RefValue, _: &NetworkOccupant) {
        self.path_calls.push("edges");
    }
    fn path_connections(&mut self, _: &RefValue) -> Vec<(u8, u8, u8)> {
        self.path_calls.push("bitmap");
        vec![(3, 0, 2)]
    }
    fn persist_key(&mut self, _: &RefValue) -> ResKey {
        ResKey {
            type_id: 1,
            group: 2,
            instance: 3,
        }
    }
}

fn corners() -> [[f32; 3]; 4] {
    [
        [32.0, 0.0, 64.0],
        [32.0, 16.0, 80.0],
        [48.0, 32.0, 80.0],
        [48.0, 16.0, 64.0],
    ]
}

fn initialized() -> (NetworkOccupant, NetworkStatics, Host) {
    let mut owner = NetworkOccupant::new();
    let mut statics = NetworkStatics::default();
    let mut host = Host {
        init_ok: true,
        ..Host::default()
    };
    assert!(owner.init(corners(), &mut statics, &mut host));
    owner.set_quad_draw(0x1234_5000, 0x81, &mut host);
    (owner, statics, host)
}

#[test]
fn base_init_failure_reinit_static_lifetime_and_geometry_worked_28() {
    let mut owner = NetworkOccupant::new();
    assert_eq!(owner.quad, [Vertex::default(); 4]);
    assert_eq!(owner.piece.network_flags, 0x0600_0000);
    assert_eq!(owner.transform, Xform::default());
    let mut statics = NetworkStatics::default();
    let mut host = Host::default();
    owner.set_retrofit(true);
    assert!(!owner.init(corners(), &mut statics, &mut host));
    assert_eq!(owner.quad[2].pos, [0.0; 3]);
    assert!(!owner.piece.initialised);
    assert_eq!((host.registered, host.retrofit), (1, 1));
    host.init_ok = true;
    assert!(owner.init(corners(), &mut statics, &mut host));
    assert_eq!(owner.piece.lifecycle, 0x1A);
    assert_eq!(owner.position(), [40.0, 32.0, 72.0]);
    assert_eq!(owner.occupied_cell(), (2, 4));
    assert_eq!(owner.bounds.min, [32.0, 0.0, 64.0]);
    assert_eq!(owner.bounds.max, [48.0, 32.0, 80.0]);
    assert_eq!(owner.altitude(36.0, 72.0), 12.0);
    assert_eq!(owner.altitude(40.0, 68.0), 12.0);
    owner.piece.network_flags |= 0x1100_0000 | 0x0080_0000;
    owner.piece.in_city = true;
    assert!(owner.init(corners(), &mut statics, &mut host));
    assert_eq!(host.init_calls, 2);
    assert_eq!(host.usable, vec![true]);
    assert_eq!(statics.intersections.len(), 13);
    owner.piece_id = 0x0400_3500;
    owner.piece.network_flags |= 0x40;
    assert!(owner.is_intersection(&statics));
    owner.piece.network_flags &= !0x40;
    assert!(!owner.is_intersection(&statics));
    statics.shutdown(&mut host);
    assert_eq!(host.refs, vec![(false, 2), (false, 1)]);
    statics.shutdown(&mut host);
    assert_eq!(host.refs.len(), 2);
}

#[test]
fn geometry_parts_box_pick_and_transform_do_not_recompute_height_box() {
    let (mut owner, _, mut host) = initialized();
    let old_bounds = owner.bounds;
    owner.set_vertex_heights([0.0, 4.0, 20.0, 7.0]);
    assert_eq!(owner.bounds, old_bounds);
    assert_eq!(owner.normal(36.0, 72.0), [-7.0, 16.0, -4.0]);
    assert_eq!(owner.normal(40.0, 68.0), [-16.0, 16.0, -13.0]);
    let altitude = owner.altitude(36.0, 72.0);
    owner.piece.network_flags ^= 0x0100_0000;
    assert_eq!(owner.altitude(36.0, 72.0), altitude);
    assert_eq!(owner.normal(f32::NAN, 70.0), [-16.0, 16.0, -13.0]);
    owner.piece.network_flags |= FLAG_LIT_COMPUTED;
    owner.add_part(
        &[[0.0, 50.0, 0.0], [64.0, -5.0, 100.0], [0.0; 3]],
        &[[0.0, 0.0], [0.5, 1.0], [1.0, 0.0]],
        [1, 0, 2],
        0x123,
    );
    assert_eq!(owner.parts[0].verts[0].u, 0.5);
    assert_eq!(owner.parts[0].verts[0].color, u32::MAX);
    assert_eq!(owner.bounds.min, [0.0, -5.0, 0.0]);
    assert_eq!(owner.bounds.max, [64.0, 50.0, 100.0]);
    assert_eq!(owner.piece.network_flags & FLAG_LIT_COMPUTED, 0);
    let mut best = 5.0;
    host.ray = Some(6.0);
    assert!(!owner.pick(&mut best, &mut host));
    host.ray = Some(5.0);
    assert!(owner.pick(&mut best, &mut host));
    assert_eq!(host.picked_box.unwrap().max[1], 50.1);
    host.ray = None;
    assert!(!owner.pick(&mut best, &mut host));
    let xform = Xform {
        translation: [10.0, 20.0, 30.0],
        scale: 2.0,
        ..Xform::default()
    };
    owner.set_transform(xform);
    assert_eq!(owner.transform.counter, 1);
    assert_eq!(owner.local_bounds().min, [-5.0, -12.5, -15.0]);
    assert_eq!(owner.world_bounds(), owner.bounds);
    let view = token(99);
    let xform = owner.get_model_instance(&mut host, &view);
    assert_eq!(xform.translation, owner.position());
    assert_eq!((xform.mask & 1, xform.counter), (1, 2));
    assert_eq!(host.refs, vec![(true, 99)]);
}

#[test]
fn property_and_edge_store_worked_29_and_base_lit_gate() {
    let (mut owner, _, mut host) = initialized();
    assert!(!owner.check_for_retrofit());
    owner.set_retrofit(true);
    assert!(owner.check_for_retrofit());
    owner.set_retrofit(false);
    assert!(!owner.check_for_retrofit());
    assert!(owner.has_property(0x8869_6402));
    assert_eq!(owner.connection_node_edges(), None);
    owner.set_connection_node_edges([7, 9]);
    assert_eq!(owner.connection_node_edges(), Some([7, 9]));
    assert_eq!(owner.passage_ceiling(), None);
    owner.set_passage_ceiling(15.5);
    assert_eq!(owner.passage_ceiling(), Some(15.5));
    owner.set_one_way_directions(0);
    assert!(!owner.has_property(0x4835_C6AA));
    owner.set_one_way_directions(9);
    owner.set_one_way_directions(0);
    assert!(owner.has_property(0x4835_C6AA));
    assert_eq!(owner.one_way_directions(), 0);
    let mut other = NetworkOccupant::new();
    assert!(!owner.same_structure(&other));
    owner.set_complex_origin(12, 34);
    other.set_complex_origin(12, 34);
    assert!(owner.same_structure(&other));
    assert_eq!(owner.complex_origin(), Some((12, 34)));
    owner.edges.set_edge_types(0, 0x0002_0002);
    assert_eq!(owner.edges.active_edges(), 5);
    assert_eq!(owner.edges.active_edges_no_group(0), 5);
    owner.edges.set_edge_type(0, 1, 4);
    assert_eq!(owner.edges.grouped_edges(), 2);
    assert_eq!(owner.edges.active_edges_for_type(0), 7);
    assert!(owner.edges.connects_to_neighbor(1, 0, 0));
    assert!(!owner.edges.connects_to_neighbor(1, 1, 0));
    owner.edges.set_edge_types(1, 1);
    owner.edges.set_edge_types(2, 2);
    owner.edges.set_edge_types(3, 3);
    owner.edges.set_edge_types(1, 0);
    assert_eq!(owner.edges.entries, vec![(3, 3), (2, 2)]);
    let copy = owner.edges.clone();
    owner.edges.set_edge_types(0, 0);
    owner.edges.set_edge_types(7, 9);
    assert_eq!((owner.edges.byte_10c, owner.edges.word_110), (7, 9));
    assert_ne!(copy, owner.edges);
    owner.set_lit(true);
    assert!(!owner.is_lit());
    assert_eq!(host.changes, 0);
    owner.set_network_flag(0x4000, &mut host);
    assert_eq!(owner.quad[0].alpha(), 128);
    assert_eq!(host.changes, 1);
}

#[test]
fn quad_uv_translucency_and_live_record_worked_31_33() {
    let (mut owner, _, mut host) = initialized();
    owner.cache_path_info(token(10), &mut host);
    owner.piece.network_flags |= FLAG_PATHS_BUILT;
    owner.set_quad_draw(0x1234_5000, 1, &mut host);
    assert_eq!((owner.quad[0].u, owner.quad[0].v), (0.0, 1.0));
    assert_eq!(host.refs, vec![(false, 10)]);
    assert_eq!(owner.piece.network_flags & FLAG_PATHS_BUILT, 0);
    owner.set_quad_draw(0x1234_5000, 0x81, &mut host);
    assert_eq!((owner.quad[0].u, owner.quad[0].v), (0.0, 0.0));
    owner.set_translucency(true, 128, &mut host);
    assert_eq!(owner.quad.map(|v| v.alpha()), [128; 4]);
    assert_eq!(owner.opacity, 128.0 / 255.0);
    assert_eq!(owner.draw_attributes & 2, 2);
    owner.set_one_way_directions(1);
    owner.set_complex_origin(2, 4);
    owner.piece.demolition_cost = 1_i64 << 40;
    let mut bytes = ByteWriter::new();
    assert!(super::super::record::write_network_record(&owner.record(), &mut bytes));
    let data = bytes.into_bytes();
    let decoded = super::super::record::read_network_record(&mut ByteReader::new(&data)).unwrap();
    let loaded = NetworkOccupant::from_record(decoded);
    assert_eq!(loaded.quad.map(|v| v.alpha()), [255; 4]);
    assert_eq!((loaded.draw_attributes & 2, loaded.opacity), (0, 1.0));
    assert_eq!(loaded.one_way_directions(), 1);
    assert_eq!(loaded.complex_origin(), Some((2, 4)));
    assert_eq!(loaded.piece.demolition_cost, 1_i64 << 40);
    assert_eq!(loaded.bounds, owner.bounds);
    owner.set_translucency(false, 0, &mut host);
    assert_eq!(owner.opacity, 1.0);
    assert_eq!(host.changes, 2);
}

#[test]
fn path_pipeline_override_refs_bitmap_cache_borrow_and_failed_retry() {
    let (mut owner, _, mut host) = initialized();
    owner.path_origin = [4.0, -8.0];
    owner.set_one_way_directions(1);
    owner.set_connection_path_override(0x123, &mut host);
    assert_eq!(owner.connection_path_override(), 0x123);
    assert_eq!(owner.path_connection(3, 0, 2, &mut host), 1);
    assert_eq!(owner.piece.path_connection(0, 0, 2, &mut host), 1);
    assert_eq!(owner.path_connection(3, 255, 255, &mut host), 1);
    assert_eq!(
        host.path_calls,
        vec![
            "uses",
            "clone",
            "elevation",
            "transform",
            "scissor",
            "wrong-way",
            "edges",
            "bitmap"
        ]
    );
    assert_eq!(
        host.path_loads,
        vec![ResKey {
            type_id: 0x2966_78F7,
            group: 0xA966_883F,
            instance: 0x124
        }]
    );
    assert_eq!(host.clip, Some(([4.0, 0.0, -8.0], [32.0, 64.0, 48.0, 80.0])));
    assert_eq!(host.refs, vec![(false, 10), (false, 11), (false, 12)]);
    assert_eq!(id(owner.get_path_info(&mut host)), 13);
    assert_eq!(id(owner.path_info()), 13);
    assert_eq!(
        owner.record().path_info_key,
        ResKey {
            type_id: 1,
            group: 2,
            instance: 3
        }
    );
    assert_eq!(host.path_loads.len(), 1);
    assert_eq!(host.refs.len(), 3);
    owner.shutdown(&mut host);
    assert_eq!(id(owner.path_info()), 13);
    owner.set_connection_path_override(0, &mut host);
    assert_eq!(owner.record().path_info_key, ResKey::default());
    assert!(!owner.has_property(0x4835_C6A9));
    assert_eq!(host.refs.last(), Some(&(false, 13)));
    host.path_fail = true;
    assert_eq!(id(owner.get_path_info(&mut host)), 0);
    assert_eq!(id(owner.get_path_info(&mut host)), 0);
    assert_eq!(host.path_loads.len(), 3);
    assert_eq!(host.path_loads[2].group, 0x6966_8828);
    host.path_fail = false;
    assert_eq!(id(owner.get_path_info(&mut host)), 12);
    owner.destroy(&mut host);
    assert_eq!(host.refs.last(), Some(&(false, 12)));
}

#[test]
fn no32_integrated_normal_rgb_alpha_and_degenerate_worked_case() {
    let (mut owner, _, mut host) = initialized();
    owner.add_part(
        &[[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        &[[0.0; 2]; 3],
        [0, 1, 2],
        1,
    );
    owner.parts[0].verts[0].color = 0x4412_3456;
    owner.update_lighting(&mut host);
    assert_eq!(host.corner_queries, 4);
    assert_eq!(host.normals, vec![[1.0, 0.0, 0.0]]);
    assert_eq!(owner.parts[0].verts[0].color, 0x4411_2233);
    assert_eq!(owner.parts[0].verts[1].color, 0xFF11_2233);
    assert_eq!(owner.quad[0].color, 0xFF03_0507);
    for v in &mut owner.parts[0].verts {
        v.pos = [0.0; 3];
    }
    owner.update_lighting(&mut host);
    assert!(host.normals[1].iter().all(|v| v.is_nan()));
    assert_eq!(host.normals.len(), 2);
    assert_eq!(owner.parts[0].verts[0].color, 0x4411_2233);
    assert_ne!(owner.piece.network_flags & FLAG_LIT_COMPUTED, 0);
}

#[test]
fn no32_draw_half_masks_zoom_cache_fallback_and_static_releases() {
    let (mut owner, mut statics, mut host) = initialized();
    for (mask, triangles) in [
        (0, vec![]),
        (0x0200_0000, vec![[0, 1, 2]]),
        (0x0400_0000, vec![[0, 2, 3]]),
        (0x0600_0000, vec![[0, 1, 2], [0, 2, 3]]),
    ] {
        owner.piece.network_flags = mask | FLAG_LIT_COMPUTED;
        host.events.clear();
        assert_eq!(owner.draw(3, false, &mut statics, &mut host), 1);
        let actual: Vec<_> = host
            .events
            .iter()
            .filter_map(|e| match e {
                Event::Triangle(t) => Some(*t),
                _ => None,
            })
            .collect();
        assert_eq!(actual, triangles);
    }
    assert_eq!(
        host.loads.iter().map(|k| k.instance).collect::<Vec<_>>(),
        vec![0x1234_5003]
    );
    assert_eq!(id(&statics.texture(0x1234_5002, 0, &mut host)), 0x1234_5002);
    owner.piece.rotation = 1;
    host.missing.push(0x1234_5013);
    host.events.clear();
    owner.draw(3, false, &mut statics, &mut host);
    assert!(host.events.contains(&Event::Bind(0x1234_5003)));
    let released = host.refs.len();
    statics.set_texture_zoom(3, &mut host);
    assert_eq!(host.refs.len() - released, 2);
    assert!(statics.textures.is_empty());
    owner.draw(4, false, &mut statics, &mut host);
    statics.shutdown(&mut host);
    assert_eq!(&host.refs[host.refs.len() - 2..], &[(false, 2), (false, 1)]);
}

#[test]
fn draw_self_lit_early_out_under_piece_order_water_and_state_b() {
    let (mut owner, mut statics, mut host) = initialized();
    owner.piece.variation = 0x5678_9000;
    owner.draw(5, true, &mut statics, &mut host);
    assert_eq!(host.corner_queries, 4);
    assert_eq!(
        host.events,
        vec![Event::State(surface::RENDER_STATES[0]), Event::Transform]
    );
    assert!(host.loads.is_empty());
    owner.base.highlight = 1;
    owner.piece.network_flags |= 0x0008_0000;
    owner.add_part(&[[0.0, 9.0, 0.0]; 3], &[[0.0; 2]; 3], [0, 1, 2], 0x444);
    owner.add_part(&[[0.0, 10.0, 0.0]; 3], &[[0.0; 2]; 3], [0, 1, 2], 0x555);
    host.water = true;
    host.events.clear();
    owner.draw(5, true, &mut statics, &mut host);
    let binds: Vec<_> = host
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Bind(v) => Some(*v),
            _ => None,
        })
        .collect();
    assert_eq!(binds, vec![0x5678_9005, 0x1234_5005, 0x555]);
    assert_eq!(host.events.first(), Some(&Event::State(surface::RENDER_STATES[1])));
    assert_eq!(host.events.last(), Some(&Event::Shade(None)));
    assert_eq!(
        host.events.iter().filter(|e| matches!(e, Event::Triangle(_))).count(),
        10
    );
    owner.edges.set_edge_types(0, 1);
    host.events.clear();
    owner.draw(5, false, &mut statics, &mut host);
    assert_eq!(host.events.first(), Some(&Event::State(surface::RENDER_STATES[0])));
    owner.piece.network_flags &= !0x0600_0000;
    assert_eq!(owner.sort_value(), 0x444);
    owner.parts.clear();
    assert_eq!(owner.sort_value(), 0xA821_EF94);
}

#[test]
fn one_way_arrow_worked_32_rotates_restores_uvs_and_replays_state() {
    let (mut owner, mut statics, mut host) = initialized();
    owner.edges.set_edge_types(10, 0x0002_0002);
    let original = owner.quad;
    for (directions, expected_uv) in [(1, [0.0, 1.0]), (2, [1.0, 0.0])] {
        host.events.clear();
        owner.draw_one_way_arrow(false, surface::RENDER_STATES[1], directions, &mut statics, &mut host);
        assert_eq!(host.events[0], Event::State(surface::RENDER_STATES[0]));
        assert_eq!(host.events[1], Event::Bind(0x0BE3_5EB5));
        let vertices = host
            .events
            .iter()
            .find_map(|e| match e {
                Event::Vertices(v) => Some(v),
                _ => None,
            })
            .unwrap();
        assert_eq!([vertices[0].u, vertices[0].v], expected_uv);
        assert_eq!(
            &host.events[host.events.len() - 2..],
            &[
                Event::State(surface::RENDER_STATES[1]),
                Event::State(surface::RENDER_STATES[1])
            ]
        );
        assert_eq!(owner.quad, original);
    }
    owner.edges.set_edge_type(10, 1, 4);
    host.events.clear();
    owner.draw_one_way_arrow(true, surface::RENDER_STATES[0], 1, &mut statics, &mut host);
    assert!(host.events.is_empty());
}
