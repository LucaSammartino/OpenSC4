use super::connection::*;
use super::record::Vertex;
use crate::automata::OccupantBase;
use crate::stream::{ByteReader, ByteWriter, InStream, RefValue};

fn record() -> ConnectionRecord {
    ConnectionRecord {
        version_read: 1,
        base: OccupantBase::new(0x7475_8925),
        field_64: 1,
        walls: std::array::from_fn(|i| {
            vec![Vertex {
                pos: [i as f32, 2.0, 3.0],
                color: 0x1234_5678,
                u: 4.0,
                v: 5.0,
            }]
        }),
        words_14c_150: [2, 3],
        words_15c: [4, 5, 6, 7],
        field_16c: 8,
        field_174: 9,
        flag_178: true,
        references: [RefValue::Null, RefValue::Null],
        field_23c: 10,
        words_22c: [11, 12, 13],
    }
}

#[test]
fn seven_wall_record_order_round_trip_and_truncation() {
    let source = record();
    let mut out = ByteWriter::new();
    assert!(source.write(&mut out));
    let bytes = out.into_bytes();
    let mut input = ByteReader::new(&bytes);
    assert_eq!(input.get_u16(), Some(2));
    let base = OccupantBase::read(&mut input).unwrap();
    assert_eq!(base.occupant_type, 0x7475_8925);
    let tail = input.position();
    assert_eq!(input.get_u32(), Some(1));
    for wall in 0..7 {
        assert_eq!(input.get_u32(), Some(1));
        assert_eq!(input.get_f32(), Some(wall as f32));
        assert_eq!(input.get_f32(), Some(2.0));
        assert_eq!(input.get_f32(), Some(3.0));
        assert_eq!(input.get_f32(), Some(4.0));
        assert_eq!(input.get_f32(), Some(5.0));
        assert_eq!(input.get_bytes(4).unwrap(), [0x34, 0x56, 0x78, 0x12]);
    }
    for value in 2..=9 {
        assert_eq!(input.get_u32(), Some(value));
    }
    assert_eq!(input.get_u8(), Some(1));
    assert_eq!(input.get_u32(), Some(0));
    assert_eq!(input.get_u32(), Some(0));
    for value in 10..=13 {
        assert_eq!(input.get_u32(), Some(value));
    }
    assert_eq!(input.remaining(), 0);
    assert_eq!(bytes.len() - tail, 4 + 7 * 28 + 8 * 4 + 1 + 8 + 4 * 4);
    let back = ConnectionRecord::read(&mut ByteReader::new(&bytes)).unwrap();
    assert_eq!(back.version_read, 2);
    assert_eq!(back.walls, source.walls);
    assert_eq!(back.words_15c, source.words_15c);
    assert_eq!(back.words_22c, source.words_22c);
    assert_eq!(back.field_23c, 10);
    assert!(back.flag_178);
    for end in 0..bytes.len() {
        assert!(ConnectionRecord::read(&mut ByteReader::new(&bytes[..end])).is_none());
    }
}

#[derive(Default)]
struct Host(Vec<u32>);

impl ConnectionLifecycleHost for Host {
    fn occupant_init(&mut self, _: &mut OccupantBase) {
        self.0.push(1);
    }
    fn subscribe_message(&mut self, message: u32) {
        self.0.push(message);
    }
    fn clear_texture_binding(&mut self, offset: u32) {
        self.0.push(offset);
    }
    fn unsubscribe_message(&mut self, message: u32) {
        self.0.push(message);
    }
    fn occupant_shutdown(&mut self, _: &mut OccupantBase) {
        self.0.push(2);
    }
}

#[test]
fn hidden_triangles_subscriptions_and_shutdown_order() {
    let reference = || RefValue::Host {
        token: 1,
        class_id: 0xC9C0_5C6E,
    };
    let mut connection = UndergroundConnection {
        base: OccupantBase::new(0x7475_8925),
        vectors_70: std::array::from_fn(|_| vec![[1.0, 2.0, 3.0]]),
        vector_a4: vec![[1.0; 3]],
        vector_b0: vec![[2.0; 3]],
        vector_c0: vec![[3.0; 3]],
        reference_158: reference(),
        reference_15c: reference(),
        reference_c8: reference(),
        reference_cc: reference(),
    };
    assert_eq!(connection.hidden_triangles(), [0, 1, 1]);
    let mut host = Host::default();
    connection.init_published_subscriptions(&mut host);
    assert_eq!(host.0, [1, 0x8B87_A974, 0x8B87_A975, 0x8B8B_2CEE, 0xA6B7_9602]);
    host.0.clear();
    connection.shutdown(&mut host);
    assert_eq!(
        host.0,
        [0x9C, 0x64, 0xB8, 0x8B87_A974, 0x8B87_A975, 0x8B8B_2CEE, 0xA6B7_9602, 2]
    );
    assert!(connection.vectors_70.iter().all(Vec::is_empty));
    assert!(connection.vector_a4.is_empty());
    assert!(connection.vector_b0.is_empty());
    assert!(connection.vector_c0.is_empty());
    assert!(connection.reference_158.is_null());
    assert!(connection.reference_15c.is_null());
    assert!(connection.reference_c8.is_null());
    assert!(connection.reference_cc.is_null());
}
