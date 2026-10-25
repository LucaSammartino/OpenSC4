use super::identity::*;
use super::piece::*;
use super::power_line::*;
use super::power_pole::*;
use crate::automata::OccupantBase;
use crate::stream::{ByteReader, ByteWriter, RefValue};

#[derive(Default)]
struct Rec {
    usable: Vec<bool>,
    translucency: Vec<(bool, u8)>,
    path_builds: u32,
    retrofit: u32,
    occ_init: u32,
    occ_shutdown: u32,
    changed: u32,
    set_model: u32,
    release_model: u32,
}

impl PieceHost for Rec {
    fn post_usable_change(&mut self, usable: bool) {
        self.usable.push(usable);
    }
    fn set_translucency(&mut self, on: bool, alpha: u8) {
        self.translucency.push((on, alpha));
    }
    fn build_path_info(&mut self) {
        self.path_builds += 1;
    }
    fn add_to_retrofit_list(&mut self) {
        self.retrofit += 1;
    }
    fn occupant_init(&mut self) {
        self.occ_init += 1;
    }
    fn occupant_shutdown(&mut self) {
        self.occ_shutdown += 1;
    }
    fn post_piece_changed(&mut self) {
        self.changed += 1;
    }
    fn prebuilt_set_model(&mut self) {
        self.set_model += 1;
    }
    fn prebuilt_release_model(&mut self) {
        self.release_model += 1;
    }
}

// Conformance 1 (§1.2).
#[test]
fn road_piece_identity() {
    let c = NetOccClass::Network;
    assert_eq!(query_interface(c, IID_NETWORK_OCCUPANT), QiAnswer::View(0));
    assert_eq!(query_interface(c, IID_FAMILY_CLASS), QiAnswer::View(0));
    assert_eq!(query_interface(c, IID_OCCUPANT), QiAnswer::View(4));
    assert_eq!(query_interface(c, IID_UNKNOWN), QiAnswer::View(4));
    assert_eq!(query_interface(c, IID_SERIALIZABLE), QiAnswer::View(8));
    assert_eq!(
        query_interface(c, 0x1234_5678),
        QiAnswer::Fallthrough {
            holder: true,
            model: true
        }
    );
    assert_eq!(c.gzclsid(), Some(0xC9C0_5C6E));
}

#[test]
fn subclass_qi_chains() {
    use NetOccClass::*;
    for c in [
        PreBuilt,
        Bridge,
        Tunnel,
        Pipe,
        Subway,
        PipeConnection,
        SubwayConnection,
        Underground,
    ] {
        assert_eq!(query_interface(c, c.class_id()), QiAnswer::View(0), "{c:?}");
    }
    // A bridge does not answer the tunnel's class ID; it falls through.
    assert_eq!(
        query_interface(Bridge, Tunnel.class_id()),
        QiAnswer::Fallthrough {
            holder: true,
            model: true
        }
    );
    // A bridge answers the prebuilt's and the network occupant interface.
    assert_eq!(query_interface(Bridge, PreBuilt.class_id()), QiAnswer::View(0));
    assert_eq!(query_interface(Bridge, IID_NETWORK_OCCUPANT), QiAnswer::View(0));
    // Pipe and subway are underground occupants; a pipe is not a subway.
    assert_eq!(query_interface(Subway, Underground.class_id()), QiAnswer::View(0));
    assert_eq!(query_interface(Pipe, Underground.class_id()), QiAnswer::View(0));
    assert_ne!(query_interface(Pipe, Subway.class_id()), QiAnswer::View(0));
    // Connections: the occupant view is at +0x1C, serializable at +4.
    assert_eq!(query_interface(PipeConnection, IID_OCCUPANT), QiAnswer::View(0x1C));
    assert_eq!(query_interface(SubwayConnection, IID_SERIALIZABLE), QiAnswer::View(4));
    assert_eq!(
        query_interface(PipeConnection, UndergroundConnection.class_id()),
        QiAnswer::View(0)
    );
    assert_ne!(query_interface(PipeConnection, IID_NETWORK_OCCUPANT), QiAnswer::View(0));
}

#[test]
fn pole_line_pylon_qi() {
    use NetOccClass::*;
    assert_eq!(query_interface(PowerPole, IID_POWER_POLE), QiAnswer::View(0));
    assert_eq!(query_interface(PowerPole, IID_MESSAGE_TARGET2), QiAnswer::View(4));
    assert_eq!(query_interface(PowerPole, IID_POLE_DELEGATE), QiAnswer::PoleDelegate);
    assert_eq!(query_interface(PowerPole, IID_OCCUPANT), QiAnswer::View(8));
    assert_eq!(query_interface(PowerPole, IID_SERIALIZABLE), QiAnswer::View(0xC));
    assert!(null_this_returns_true(PowerPole, IID_POWER_POLE));
    assert!(null_this_returns_true(PowerPole, IID_MESSAGE_TARGET2));
    assert!(!null_this_returns_true(PowerPole, IID_OCCUPANT));
    // The line has no model fallback (§1.2).
    assert_eq!(query_interface(PowerLine, IID_POWER_LINE), QiAnswer::View(0));
    assert_eq!(query_interface(PowerLine, PowerLine.class_id()), QiAnswer::View(0));
    assert_eq!(
        query_interface(PowerLine, 0x1234_5678),
        QiAnswer::Fallthrough {
            holder: true,
            model: false
        }
    );
    assert_eq!(query_interface(Pylon, IID_PYLON), QiAnswer::View(0));
    assert_eq!(query_interface(Pylon, IID_MESSAGE_TARGET2), QiAnswer::View(4));
    assert!(null_this_returns_true(Pylon, IID_MESSAGE_TARGET2));
    assert!(!null_this_returns_true(Pylon, IID_PYLON));
}

// §1.3 and conformance 2.
#[test]
fn class_ids_and_types() {
    use NetOccClass::*;
    for c in NetOccClass::ALL {
        assert_eq!(NetOccClass::from_class_id(c.class_id()), Some(c));
        assert_eq!(c.gzclsid().is_none(), c == Underground);
    }
    assert_eq!(Network.occupant_type(false), 0xC772_BF98);
    assert_eq!(Tunnel.occupant_type(false), 0xC772_BF98);
    assert_eq!(PowerPole.occupant_type(false), 0x2890_D4DE);
    assert_eq!(PowerLine.occupant_type(false), 0x0990_B2EA);
    assert_eq!(Pylon.occupant_type(false), 0xA9B6_D7DE);
    assert_eq!(PipeConnection.occupant_type(false), OCCUPANT_TYPE_BARE);
    assert_eq!(PipeConnection.occupant_type(true), 0x2A18_3167);
    assert_eq!(SubwayConnection.occupant_type(true), 0x8A13_918A);
    assert_eq!(UndergroundConnection.occupant_type(true), OCCUPANT_TYPE_BARE);
}

// Conformance 3.
#[test]
fn is_of_type_is_a_flag_test() {
    let mut p = NetworkPiece::new(NetOccClass::Network);
    p.network_flags |= 1;
    assert!(p.is_of_type(0));
    assert!(!p.is_of_type(1));
    for t in 0..13 {
        assert_eq!(p.is_of_type(t), p.network_flags & (1 << t) != 0);
    }
    assert!(!is_of_type(u32::MAX, 13));
}

// Conformance 5 and quirk 9.
#[test]
fn occupied_cell_rounding() {
    let mut p = NetworkPiece::new(NetOccClass::Network);
    p.position = [800.0, 3.0, 240.0];
    assert_eq!(p.get_occupied_cell(), (50, 15));
    assert_eq!(world_to_cell(-4096.0), -256);
    assert_eq!(world_to_cell(-800.0), -50);
    // Round to nearest, ties to even (the float32 magic add).
    assert_eq!(world_to_cell(808.0), 50);
    assert_eq!(world_to_cell(824.0), 52);
    assert_eq!(world_to_cell(809.0), 51);
    assert_eq!(world_to_cell(7.0), 0);
}

// Conformance 6 (slot 25 is `GetRotation`; see NO14).
#[test]
fn orientation_flip_rotation() {
    let mut h = Rec::default();
    let mut p = NetworkPiece::new(NetOccClass::Network);
    p.set_rotation(2, &mut h);
    assert_eq!(p.get_rotation(), 2);
    p.orientation = 0x05 | 0x80;
    assert_eq!(p.get_flip(), 1);
    assert_eq!(p.get_orientation(), 5);
    p.set_variation(7, &mut h);
    assert_eq!(p.variation, 7);
    assert_eq!(h.changed, 2);
}

// Conformance 7 and network-manager.md test 22.
#[test]
fn usable_message() {
    let mut h = Rec::default();
    let mut p = NetworkPiece::new(NetOccClass::Network);
    p.in_city = true;
    p.set_network_flag(FLAG_USABLE, &mut h);
    p.set_network_flag(FLAG_USABLE, &mut h);
    assert_eq!(h.usable, vec![true]);
    p.clear_network_flag(FLAG_USABLE, &mut h);
    p.clear_network_flag(FLAG_USABLE, &mut h);
    assert_eq!(h.usable, vec![true, false]);
    // Not in the city: set posts nothing, clear posts.
    let mut h = Rec::default();
    let mut q = NetworkPiece::new(NetOccClass::Network);
    q.set_network_flag(FLAG_USABLE, &mut h);
    assert!(h.usable.is_empty());
    q.clear_network_flag(FLAG_USABLE, &mut h);
    assert_eq!(h.usable, vec![false]);
    // 0x4000 calls slot 58.
    q.set_network_flag(FLAG_TRANSLUCENT, &mut h);
    assert_eq!(h.translucency, vec![(true, 0x80)]);
}

// Conformance 8.
#[test]
fn managed_globally() {
    use NetOccClass::*;
    assert!(!NetworkPiece::new(Network).is_managed_globally());
    for c in [PreBuilt, Bridge, Tunnel] {
        assert!(NetworkPiece::new(c).is_managed_globally(), "{c:?}");
    }
    assert_eq!(PowerLine.is_managed_globally(), None);
}

#[test]
fn constructor_flags() {
    use NetOccClass::*;
    assert_eq!(NetworkPiece::new(Network).network_flags, 0x0600_0000);
    assert_eq!(NetworkPiece::new(PreBuilt).network_flags, 0x0640_0000);
    assert_eq!(NetworkPiece::new(Bridge).network_flags, 0x0640_0000);
    // Literal `&= ~0x400 | 0x10C00000` (NO11).
    assert_eq!(NetworkPiece::new(Tunnel).network_flags, 0x16C0_0000);
}

#[test]
#[should_panic]
fn connection_is_not_a_network_piece() {
    // §3.6: the connection occupants derive directly from cSC4Occupant.
    let _ = NetworkPiece::new(NetOccClass::PipeConnection);
}

// Conformance 9.
#[test]
fn path_connections() {
    let mut h = Rec::default();
    let mut p = NetworkPiece::new(NetOccClass::Network);
    assert_eq!(p.path_connection(1, 0, 2, &mut h), 0);
    assert_eq!(h.path_builds, 1);
    p.add_path_connection(1, 0, 2, &mut h);
    assert!(p.has_all_network_flags(FLAG_PATHS_BUILT));
    assert_eq!(p.connections[0][2], 0b11);
    assert_eq!(p.path_connection(1, 0, 2, &mut h), 1);
    assert_eq!(p.path_connection(0, 0, 2, &mut h), 1);
    assert_eq!(p.path_connection(2, 0, 2, &mut h), 0);
    // Flag 0x2000 is set now: no more path-info builds.
    assert_eq!(h.path_builds, 1);
    p.add_path_connection(1, 3, 1, &mut h);
    assert_eq!(p.path_connection(1, PATH_WILDCARD, PATH_WILDCARD, &mut h), 2);
    assert_eq!(p.path_connection(PATH_WILDCARD, 0, 2, &mut h), 2);
    assert_eq!(p.path_connection(1, 0, PATH_WILDCARD, &mut h), 1);
}

// Conformance 10.
#[test]
fn init_lifecycle() {
    let mut h = Rec::default();
    let mut p = NetworkPiece::new(NetOccClass::Network);
    p.in_city = true;
    p.init(&mut h);
    assert_eq!(p.lifecycle, 0x1A);
    assert!(p.has_all_network_flags(FLAG_PATHS_BUILT));
    assert_eq!((h.retrofit, h.occ_init), (1, 1));
    p.network_flags |= FLAG_USABLE | FLAG_REINIT_CLEAR;
    p.init(&mut h);
    assert!(!p.has_any_network_flag(FLAG_USABLE | FLAG_REINIT_CLEAR));
    assert_eq!(h.occ_init, 1);
    assert_eq!(h.usable, vec![false]);
    p.shutdown(&mut h);
    assert_eq!((h.occ_shutdown, h.release_model), (1, 0));
}

#[test]
fn prebuilt_and_tunnel_init() {
    let mut h = Rec::default();
    let mut p = NetworkPiece::new(NetOccClass::PreBuilt);
    p.init(&mut h);
    assert_eq!(p.lifecycle, (0x1A & !0x10) | 0x40);
    assert_eq!((h.retrofit, h.set_model), (0, 1));
    p.shutdown(&mut h);
    assert_eq!(h.release_model, 1);
    let mut t = NetworkPiece::new(NetOccClass::Tunnel);
    t.network_flags |= FLAG_TRANSLUCENT;
    t.init(&mut h);
    assert!(!t.has_any_network_flag(FLAG_TRANSLUCENT));
}

// Conformance 18.
#[test]
fn power_line_slots() {
    let mut l = PowerLine::new();
    assert_eq!(l.base.occupant_type, OCCUPANT_TYPE_POWER_LINE);
    let a = RefValue::Host {
        token: 1,
        class_id: NetOccClass::PowerPole.class_id(),
    };
    let b = RefValue::Host {
        token: 2,
        class_id: NetOccClass::PowerPole.class_id(),
    };
    l.set_connected_poles(a, b, 1);
    let (x, y, f) = l.get_connected_poles();
    assert!(matches!(x, RefValue::Host { token: 1, .. }));
    assert!(matches!(y, RefValue::Host { token: 2, .. }));
    assert_eq!(f, 1);
    assert!(!l.is_preview());
    l.set_preview(1);
    assert!(l.is_preview());
    l.set_position([pole_position_axis(10), 4.0, pole_position_axis(3)]);
    assert_eq!(l.get_cell(), (10, 3));
    assert_eq!(l.get_bounding_box(), (l.position, l.position));
    l.shutdown();
    assert!(l.poles.iter().all(|p| p.is_null()));
}

#[test]
fn power_line_record_round_trip() {
    let rec = PowerLineRecord {
        version: POWER_LINE_RECORD_VERSION,
        base: OccupantBase::new(OCCUPANT_TYPE_POWER_LINE),
        position_bits: [168.0f32.to_bits(), 4.5f32.to_bits(), 56.0f32.to_bits()],
        cell: [10, -3],
        flag: true,
        refs: [RefValue::Null, RefValue::Null],
    };
    let mut w = ByteWriter::new();
    assert!(write_power_line_record(&rec, &mut w));
    let bytes = w.into_bytes();
    assert_eq!(&bytes[..2], &[2, 0]);
    // NO13: three floats, two signed cells, one bool, two null refs.
    let tail = &bytes[2 + base_len()..];
    assert_eq!(tail.len(), 29);
    assert_eq!(&tail[..4], &168.0f32.to_le_bytes());
    assert_eq!(&tail[12..16], &10i32.to_le_bytes());
    assert_eq!(&tail[16..20], &(-3i32).to_le_bytes());
    assert_eq!(tail[20], 1);
    assert_eq!(&tail[21..], &[0; 8]);
    let mut r = ByteReader::new(&bytes);
    let back = read_power_line_record(&mut r).expect("read");
    assert_eq!(back.version, 2);
    // The base read marks the status "loaded, not initialised" (status 2).
    assert_eq!(back.base.occupant_type, rec.base.occupant_type);
    assert_eq!(back.base.flags, rec.base.flags);
    assert_eq!(back.base.status, 2);
    assert_eq!(back.position_bits, rec.position_bits);
    assert_eq!(back.cell, [10, -3]);
    assert!(back.flag);
    assert!(back.refs.iter().all(|r| r.is_null()));
    // Truncated input fails.
    let mut r = ByteReader::new(&bytes[..bytes.len() - 1]);
    assert!(read_power_line_record(&mut r).is_none());
    let mut wrong_version = bytes;
    wrong_version[0] = 3;
    let mut input = ByteReader::new(&wrong_version);
    assert!(read_power_line_record(&mut input).is_none());
    assert_eq!(input.position(), 2);
}

#[test]
fn power_line_live_record_keeps_saved_cells_and_float_bits() {
    let poles = [
        RefValue::Host {
            token: 17,
            class_id: 0x09C0_5C6A,
        },
        RefValue::Host {
            token: 29,
            class_id: 0x09C0_5C6A,
        },
    ];
    let rec = PowerLineRecord {
        version: POWER_LINE_RECORD_VERSION,
        base: OccupantBase::new(OCCUPANT_TYPE_POWER_LINE),
        position_bits: [0x8000_0000, 0x7FC0_0017, 56.0f32.to_bits()],
        // Read must retain these cells, not recompute them from position.
        cell: [-7, 111],
        flag: true,
        refs: poles.clone(),
    };
    let mut line = PowerLine::from_record(rec.clone());
    assert_eq!(line.cell, (-7, 111));
    assert!(!line.is_preview());
    let before = line.record();
    assert_eq!(before.base, rec.base);
    assert_eq!(before.position_bits, rec.position_bits);
    assert_eq!(before.cell, rec.cell);
    assert!(before.flag);
    line.set_preview(1);
    let after = line.record();
    assert_eq!(after.position_bits, before.position_bits);
    assert_eq!(after.cell, before.cell);
    assert_eq!(after.base, before.base);
    assert_eq!(after.flag, before.flag, "preview is not saved");
    let (first, second, flag) = line.get_connected_poles();
    assert!(matches!(
        first,
        RefValue::Host {
            token: 17,
            class_id: 0x09C0_5C6A
        }
    ));
    assert!(matches!(
        second,
        RefValue::Host {
            token: 29,
            class_id: 0x09C0_5C6A
        }
    ));
    assert_eq!(flag, 1);
    line.set_position([-17.0, 4.0, 31.0]);
    assert_eq!(line.record().cell, [-1, 1]);
    line.shutdown();
    assert!(line.record().refs.iter().all(RefValue::is_null));
}

fn base_len() -> usize {
    let mut w = ByteWriter::new();
    assert!(OccupantBase::new(OCCUPANT_TYPE_POWER_LINE).write(&mut w));
    w.into_bytes().len()
}

#[test]
fn control_variables() {
    let d = PowerLineControl::default();
    assert_eq!(d.v_e9a18d3f, 16.0);
    assert_eq!(d.v_e9a1e8d4.to_bits(), 0x3E19_999A);
    assert_eq!(d.v_89a1e8fb, 5);
    let l = PowerLineControl::load(
        |id| (id == PROP_E9A18D3F).then_some(20.0),
        |id| (id == PROP_89A1E8FB).then_some(7),
    );
    assert_eq!(l.v_e9a18d3f, 20.0);
    assert_eq!(l.v_49a1e8c4, 10.0);
    assert_eq!(l.v_89a1e8fb, 7);
    assert_eq!(CONTROL_EXEMPLAR.instance, 0xC911_E35B);
}
