use super::{
    connection::*,
    pylon::{References, RenderStateArgs},
    record::Vertex,
    tests_pylon::{reference, token, writer, Host, Reader},
};
use crate::{
    automata::OccupantBase,
    stream::{ByteReader, InStream, RefValue},
};

fn record() -> ConnectionRecord {
    ConnectionRecord {
        base: OccupantBase::new(0x2A18_3167),
        saved_68: 17,
        walls: Default::default(),
        cell: (10, 20),
        heights: [20.0, 21.0, 22.0, 23.0],
        saved_170: 12.0,
        saved_178: 1,
        model_state: false,
        attached: RefValue::Null,
        linked: RefValue::Null,
        saved_240: 7,
        position: [32.0, 12.0, 64.0],
        loaded: false,
    }
}
fn connection() -> UndergroundConnection {
    UndergroundConnection {
        record: record(),
        initialized: false,
        surface: [Vertex::default(); 4],
        matrices: [[9.0; 16]; 2],
        bottom_y: 99.0,
    }
}
#[derive(Default)]
struct ConnHost {
    h: Host,
    queries: Vec<(bool, (i32, i32), u32)>,
    requests: Vec<([i32; 4], [i32; 4])>,
    own_hit: bool,
}
impl References for ConnHost {
    fn add_ref(&mut self, v: &RefValue) {
        self.h.add_ref(v);
    }
    fn release(&mut self, v: &RefValue) {
        self.h.release(v);
    }
}
impl ConnectionRecordHost for ConnHost {
    fn adjust_view(&mut self, v: RefValue, bytes: i32) -> RefValue {
        self.h.log.push(format!("adjust{bytes}"));
        match v {
            RefValue::Host { token, class_id } => RefValue::Host {
                token: (token as i64 + i64::from(bytes)) as u64,
                class_id,
            },
            _ => panic!("unexpected adjustment"),
        }
    }
}
impl ConnectionHost for ConnHost {
    fn acquire_resource_manager(&mut self) -> RefValue {
        self.h.log.push("resource".into());
        reference(88)
    }
    fn register_shutdown(&mut self) {
        self.h.log.push("callback".into());
    }
    fn renderer_exists(&self) -> bool {
        self.h.renderer
    }
    fn renderer_probe(&mut self) {
        self.h.log.push("probe".into());
    }
    fn pack(&mut self, args: RenderStateArgs) -> u64 {
        self.h.packs.push(args);
        self.h.packs.len() as u64
    }
    fn occupant_init(&mut self, _: &mut OccupantBase) {
        self.h.log.push("init".into());
    }
    fn occupant_shutdown(&mut self, _: &mut OccupantBase) {
        self.h.log.push("shutdown".into());
    }
    fn message(&mut self, id: u32, subscribe: bool) {
        self.h.log.push(format!("message{id:x}/{subscribe}"));
    }
    fn clear_texture_binding(&mut self, slot: u32) {
        self.h.log.push(format!("binding{slot}"));
    }
    fn position_to_cell(&mut self, p: [f32; 3]) -> (i32, i32) {
        assert_eq!(p, [32.0, 12.0, 64.0]);
        (10, 20)
    }
    fn cell_origin(&mut self, cell: (i32, i32)) -> [f32; 2] {
        assert_eq!(cell, (10, 20));
        [32.0, 64.0]
    }
    fn cell_extents(&mut self) -> [f32; 2] {
        [16.9, 16.1]
    }
    fn cell_size(&mut self) -> f32 {
        16.0
    }
    fn corner_heights(&mut self, _: (i32, i32)) -> [f32; 4] {
        [20.0, 21.0, 22.0, 23.0]
    }
    fn find_occupant(&mut self, cell: (i32, i32), class: u32) -> RefValue {
        self.queries.push((false, cell, class));
        if cell == (11, 20) || (cell == (10, 20) && self.own_hit) {
            reference(101)
        } else {
            RefValue::Null
        }
    }
    fn find_connection(&mut self, cell: (i32, i32), class: u32) -> RefValue {
        self.queries.push((true, cell, class));
        if cell == (9, 21) {
            reference(102)
        } else {
            RefValue::Null
        }
    }
    fn network_interface(&mut self, network: u32) -> RefValue {
        assert!(network == 4 || network == 7);
        reference(200)
    }
    fn begin_change(&mut self, _: &RefValue) {
        self.h.log.push("begin".into());
    }
    fn connect(&mut self, _: &RefValue, coordinates: [i32; 4], tail: [i32; 4]) {
        self.requests.push((coordinates, tail));
        self.h.log.push("connect".into());
    }
    fn end_change(&mut self, _: &RefValue) {
        self.h.log.push("end".into());
    }
    fn refresh(&mut self, v: &RefValue, propagate: bool) {
        assert!(propagate);
        self.h.log.push(format!("refresh{}", token(v)));
    }
    fn remove_self(&mut self, destroy: bool, arg: u32) {
        assert!(destroy);
        assert_eq!(arg, 0);
        self.h.log.push("remove-self".into());
    }
    fn query_interface(&mut self, _: &RefValue, iid: u32) -> RefValue {
        self.h.log.push(format!("qi{iid:x}"));
        reference(300)
    }
    fn model_instance(&mut self) -> RefValue {
        reference(999)
    }
    fn post_occupant_message(&mut self, id: u32, value: u32) {
        self.h.log.push(format!("post{id:x}/{value}"));
    }
}

#[test]
fn connection_record_exact_version_seven_lists_refs_and_partial_failure() {
    let mut r = record();
    for (i, count) in [0, 1, 2, 0, 1, 0, 3].into_iter().enumerate() {
        r.walls[i] = (0..count)
            .map(|j| Vertex {
                pos: [i as f32, j as f32, 3.0],
                u: 0.25,
                v: 0.5,
                color: 0x4411_2233,
            })
            .collect();
    }
    r.attached = reference(96);
    r.linked = reference(200);
    let mut h = ConnHost::default();
    let mut w = writer();
    assert!(r.write(&mut w, &mut h));
    let bytes = w.into_bytes();
    let mut peek = ByteReader::new(&bytes);
    assert_eq!(peek.get_u16(), Some(2));
    OccupantBase::read(&mut peek).unwrap();
    assert_eq!(peek.get_u32(), Some(17));
    let body = peek.position();
    for count in [0, 1, 2, 0, 1, 0, 3] {
        assert_eq!(peek.get_u32(), Some(count));
        for _ in 0..count {
            peek.skip(20);
            assert_eq!(peek.get_bytes(4).unwrap(), [0x11, 0x22, 0x33, 0x44]);
        }
    }
    let mut back = record();
    let mut input = Reader::new(&bytes);
    assert!(back.read_into(&mut input, &mut h));
    assert_eq!(input.inner.remaining(), 0);
    assert_eq!(input.iids, [0x26BB_767E; 2]);
    assert_eq!((token(&back.attached), token(&back.linked)), (96, 200));
    assert!(back.loaded && back.model_state);
    assert_eq!(back.walls, r.walls);
    for version in [1u16, 3] {
        let mut bytes = bytes.clone();
        bytes[..2].copy_from_slice(&version.to_le_bytes());
        let mut input = Reader::new(&bytes);
        assert!(!back.read_into(&mut input, &mut h));
        assert_eq!(input.inner.position(), 2);
    }
    // Reached body still releases refs and sets the post-read latches.
    let mut back = record();
    back.attached = reference(8);
    back.linked = reference(9);
    h.h.log.clear();
    assert!(!back.read_into(&mut Reader::new(&bytes[..body + 9]), &mut h));
    assert_eq!(h.h.log, ["release8", "release9"]);
    assert!(back.loaded && back.model_state);
    assert_eq!(back.walls[1].len(), 1); // resize survived failed vertex reads
    let mut null = record();
    let mut w = writer();
    null.write(&mut w, &mut h);
    let bytes = w.into_bytes();
    h.h.log.clear();
    assert!(null.read_into(&mut Reader::new(&bytes), &mut h));
    assert!(h.h.log.is_empty());
}

#[test]
fn connection_creation_search_order_fresh_loaded_and_own_hit_conformance_10_11() {
    for loaded in [false, true] {
        let mut c = connection();
        c.record.loaded = loaded;
        let mut s = ConnectionStatics::default();
        let mut h = ConnHost::default();
        assert_eq!(
            c.create(ConnectionKind::Pipe, &mut s, &mut h),
            Err(ConnectionUnsupported::GeometryBoundsDrawTransformMessagesUG5NO40)
        );
        let expected = [
            (10, 20),
            (9, 20),
            (10, 19),
            (11, 20),
            (10, 21),
            (9, 19),
            (11, 19),
            (11, 21),
            (9, 21),
            (10, 20),
        ];
        assert_eq!(
            h.queries.iter().filter(|q| !q.0).map(|q| q.1).collect::<Vec<_>>(),
            expected
        );
        for (i, q) in h.queries.iter().enumerate() {
            if !q.0 && q.1 != (10, 20) && q.1 != (11, 20) {
                assert_eq!(h.queries[i + 1], (true, q.1, 0xEA0A_5ACF));
            }
        }
        assert!(matches!(c.record.attached, RefValue::Null));
        if loaded {
            assert!(h.requests.is_empty());
        } else {
            assert_eq!(
                h.requests,
                [([10, 20, 11, 20], [0, 0, 1, -1]), ([10, 20, 9, 21], [0, 0, 1, -1])]
            );
        }
        assert_eq!(h.h.log.iter().filter(|s| *s == "release101").count(), 1);
        assert_eq!(h.h.log.iter().filter(|s| *s == "release102").count(), 1);
        if !loaded {
            let index = h.h.log.iter().position(|s| s == "add200").unwrap();
            assert_eq!(
                &h.h.log[index..index + 5],
                ["add200", "begin", "connect", "end", "release200"]
            );
        }
        assert_eq!(
            c.surface.map(|v| v.pos),
            [
                [32.0, 4.0, 64.0],
                [32.0, 5.0, 80.0],
                [48.0, 6.0, 80.0],
                [48.0, 7.0, 64.0]
            ]
        );
        assert_eq!(
            c.surface.map(|v| [v.u, v.v]),
            [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]
        );
        assert_eq!(c.matrices, [IDENTITY4; 2]);
        assert_eq!((c.record.saved_170, c.bottom_y), (12.0, -4.0));
    }
    let mut c = connection();
    let mut s = ConnectionStatics::default();
    let mut h = ConnHost {
        own_hit: true,
        ..ConnHost::default()
    };
    assert!(c.create(ConnectionKind::Subway, &mut s, &mut h).is_err());
    assert_eq!(h.queries, [(false, (10, 20), 0xCA16_374F)]);
    assert!(h.requests.is_empty());
    assert_eq!(h.h.log.iter().filter(|s| *s == "refresh101").count(), 1);
    assert_eq!(c.record.base.occupant_type, 0x8A13_918A);
}

#[test]
fn connection_statics_latch_packing_enumeration_and_position_conformance_13() {
    let mut c = connection();
    let mut s = ConnectionStatics {
        packed_states: [11, 12, 13],
        ..ConnectionStatics::default()
    };
    let mut h = ConnHost::default();
    h.h.renderer = false;
    assert!(c.init(&mut s, &mut h));
    assert_eq!(s.packed_states, [11, 12, 13]);
    let count = h.h.log.len();
    assert!(c.init(&mut s, &mut h));
    assert_eq!(h.h.log.len(), count);
    h.h.renderer = true;
    s.setup_packed_states(&mut h);
    assert_eq!(h.h.packs, CONNECTION_RENDER_STATES);
    assert!(s.shutdown(&mut h));
    assert!(s.shutdown(&mut h));
    s.initialize(&mut h);
    assert!(s.latched);
    assert!(matches!(s.resource_manager, RefValue::Null));
    assert_eq!(h.h.log.iter().filter(|s| *s == "release88").count(), 1);
    assert_eq!(h.h.log.iter().filter(|s| *s == "resource").count(), 1);
    h.h.log.clear();
    let instances = c.model_instances(&mut h);
    assert_eq!(token(&instances[0]), 999);
    assert_eq!(h.h.log, ["add999"]);
    h.h.log.clear();
    let cell = c.record.cell;
    let surface = c.surface;
    c.set_position([4.0, 5.0, 6.0]);
    assert_eq!(c.position(), [4.0, 5.0, 6.0]);
    assert_eq!(c.record.cell, cell);
    assert_eq!(c.surface, surface);
    assert!(h.h.log.is_empty());
}

#[test]
fn connection_linked_refs_refresh_messages_query_and_shutdown() {
    let mut c = connection();
    let mut h = ConnHost::default();
    c.record.linked = reference(1);
    c.set_linked_occupant(reference(2), &mut h);
    c.set_linked_occupant(reference(2), &mut h);
    c.set_linked_occupant(RefValue::Null, &mut h);
    c.set_linked_occupant(RefValue::Null, &mut h);
    assert_eq!(h.h.log, ["release1", "add2", "release2", "remove-self"]);
    assert_eq!(
        c.model_state_changed(),
        Err(ConnectionUnsupported::GeometryBoundsDrawTransformMessagesUG5NO40)
    );
    assert_eq!(c.model_state_changed(), Ok(()));
    c.record.attached = reference(3);
    h.h.log.clear();
    c.refresh_attached(&mut h);
    assert_eq!(h.h.log, ["refresh3"]);
    h.h.log.clear();
    c.post_occupant_message(0x99EF_1142, 7, &mut h);
    assert_eq!(h.h.log, ["post99ef1142/7", "post8b87a974/7"]);
    h.h.log.clear();
    let answer = c.query_occupant_interface(ConnectionKind::Subway, &reference(5), &mut h);
    assert_eq!(token(&answer), 300);
    assert_eq!(h.h.log, ["qica16374f", "qi4a10afc2", "release300"]);
    assert_eq!(c.sort_value(ConnectionKind::Pipe), 0xEA0A_5ACF);
    assert_eq!(c.hidden_triangles(), [0, 1, 1]);
    c.record.linked = reference(4);
    c.record.walls[0] = vec![Vertex::default()];
    h.h.log.clear();
    c.shutdown(&mut h);
    assert_eq!(
        &h.h.log[..5],
        ["binding0", "binding1", "binding2", "release3", "release4"]
    );
    assert!(c.record.walls.iter().all(Vec::is_empty));
    assert_eq!(h.h.log.last().unwrap(), "shutdown");
}
