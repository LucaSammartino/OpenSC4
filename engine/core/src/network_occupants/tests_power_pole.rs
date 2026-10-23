use super::power_pole::*;
use super::xform::Xform;
use crate::automata::OccupantBase;
use crate::stream::{RefValue, ResKey};

fn fields() -> PoleFields {
    PoleFields {
        transform: Xform::default(),
        position: [2.0, 3.0, 4.0],
        cell: (-7, 9),
        model_y: 3.5,
        distressed: 2,
        ignored: 3,
        field_118: 123,
        delegate: RefValue::Null,
        connections: (0..2)
            .map(|i| PoleConnection {
                pole: RefValue::Host {
                    token: i + 1,
                    class_id: 0x09C0_5C6A,
                },
                wire_points: vec![RefValue::Null],
                flags: [i as u8, 4],
                words: [5, 6],
                vertices: [vec![[1.0, 2.0, 3.0]], vec![]],
            })
            .collect(),
    }
}

#[derive(Default)]
struct Host(Vec<&'static str>);

impl PoleInitHost for Host {
    fn occupant_init(&mut self, base: &mut OccupantBase) {
        assert_eq!(base.occupant_type, 0x2890_D4DE);
        self.0.push("base");
    }

    fn set_default_exemplar(&mut self, _: &mut OccupantBase, key: ResKey) {
        assert_eq!(key.instance, 77);
        self.0.push("exemplar");
    }

    fn model_property(&mut self, _: &OccupantBase, property: u32) -> u32 {
        assert_eq!(property, 0x2781_2821);
        self.0.push("property");
        0x1234
    }

    fn model_resource(&mut self, resource_type: u32, model_id: u32) -> RefValue {
        assert_eq!((resource_type, model_id), (0x9AD0_E7E3, 0x1234));
        self.0.push("resource");
        RefValue::Host {
            token: 42,
            class_id: resource_type,
        }
    }
}

#[test]
fn published_init_copies_models_and_clears_geometry_in_order() {
    let mut statics = PoleStatics::new(0x89AB_CDEF, 0xFEDC_BA98);
    statics.init_once([16.0, 32.0], [10, 20], |_| None, |_| None);
    let mut pole = PowerPole::new(fields());
    pole.geometry = [vec![[1.0; 3]], vec![[2.0; 3]], vec![[3.0; 3]]];
    pole.bytes_54_55 = [4, 5];
    pole.scale_64 = 6.0;
    let mut host = Host::default();
    pole.init_published_state(
        &statics,
        ResKey {
            instance: 77,
            ..ResKey::default()
        },
        99,
        3,
        &mut host,
    );
    assert_eq!(host.0, ["base", "exemplar", "property", "resource"]);
    assert_eq!(pole.model_ids, [10, 20]);
    assert_eq!(pole.wire_exemplar_index, 99);
    assert!(matches!(pole.model, RefValue::Host { token: 42, .. }));
    assert_eq!(pole.connection_point_table, Some(ConnectionPointTable::Second));
    assert!(pole.geometry.iter().all(Vec::is_empty));
    assert_eq!(pole.bytes_54_55, [0, 0]);
    assert_eq!(pole.scale_64, 1.0);
    // Init does not clear the separately specified connection vector.
    assert_eq!(pole.connection_count(), 2);
    assert!(pole.is_distressed());
}

#[test]
fn static_controls_load_once_and_cost_preserves_both_words() {
    let mut statics = PoleStatics::new(0x89AB_CDEF, 0xFEDC_BA98);
    statics.init_once(
        [16.0, 32.0],
        [10, 20],
        |id| (id == PROP_E9A18D3F).then_some(31.0),
        |_| None,
    );
    statics.init_once([1.0; 2], [1; 2], |_| panic!("second read"), |_| panic!("second read"));
    assert_eq!(statics.control.v_e9a18d3f, 31.0);
    assert_eq!(statics.control.v_49a1e8c4, 10.0);
    assert_eq!(statics.cell_size, [16.0, 32.0]);
    assert_eq!(statics.model_ids, [10, 20]);
    assert_eq!(statics.demolition_cost() as u64, 0xFEDC_BA98_89AB_CDEF);
    assert_eq!(PoleStatics::new(7, 0).demolition_cost(), 7);
}

#[test]
fn connection_views_and_published_state() {
    let mut pole = PowerPole::new(fields());
    // §13 test 2.
    assert_eq!(pole.base.occupant_type, 0x2890_D4DE);
    // Metadata only; the native pool is not implemented by this Rust type.
    assert_eq!(POWER_POLE_POOL_OBJECT_SIZE, 0x124);
    assert_eq!(pole.get_cell(), (-7, 9));
    assert!(pole.is_distressed());
    assert!(pole.ignored());
    pole.set_field_118(456);
    assert_eq!(pole.fields.field_118, 456);
    assert_eq!(pole.connection(0).unwrap().vertices[0], [[1.0, 2.0, 3.0]]);
    assert!(matches!(pole.connected_pole(1), Some(RefValue::Host { token: 2, .. })));
    assert!(pole.connection(2).is_none());
    for (nibble, expected) in [
        (1, ConnectionPointTable::First),
        (2, ConnectionPointTable::First),
        (3, ConnectionPointTable::Second),
        (4, ConnectionPointTable::Second),
    ] {
        pole.select_connection_point_table(nibble);
        assert_eq!(pole.connection_point_table, Some(expected));
    }
}
