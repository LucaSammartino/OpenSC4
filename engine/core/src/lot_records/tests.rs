//! lot-records §17 conformance tests for the records of this module.

use super::lot::*;
use super::*;
use crate::automata::OccupantBase;
use crate::store::crc32_mpeg2;
use crate::stream::{ByteReader, ByteWriter, RefResolver};
use crate::traffic_simulator::{write_trip, TripData};

fn hex(s: &str) -> Vec<u8> {
    let c: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    (0..c.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&c[i..i + 2], 16).unwrap())
        .collect()
}

/// References resolve to host tokens carrying the object ID.
struct Tokens;

impl RefResolver for Tokens {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: u64::from(object_id),
            class_id,
        })
    }
}

/// Writes references back as the object ID and class they were read with.
struct RawOut(ByteWriter);

impl OutStream for RawOut {
    fn put_u8(&mut self, v: u8) -> bool {
        self.0.put_u8(v)
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.0.put_u16(v)
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.0.put_u32(v)
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.0.put_u64(v)
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.0.put_f32(v)
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.0.put_f64(v)
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.0.put_bytes(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.0.put_str(v)
    }
    fn put_ref(&mut self, r: &RefValue) -> bool {
        match r {
            RefValue::Null => self.0.put_u32(0),
            RefValue::Host { token, class_id } => self.0.put_u32(*token as u32) && self.0.put_u32(*class_id),
            RefValue::Object(o) => self.0.put_u32(o.host_token() as u32) && self.0.put_u32(o.class_id()),
        }
    }
    fn error(&self) -> i32 {
        self.0.error()
    }
}

/// Reads `bytes` into a fresh object of `class`: (object, result, remaining).
fn read_obj(class: u32, bytes: &[u8]) -> (RefValue, bool, usize) {
    let o = create(class).expect("class");
    let RefValue::Object(so) = &o else { unreachable!() };
    let mut t = Tokens;
    let mut r = ByteReader::with_resolver(bytes, &mut t);
    let ok = so.read(&mut r);
    let rem = r.remaining();
    (o, ok, rem)
}

fn write_obj(o: &RefValue) -> Vec<u8> {
    let RefValue::Object(so) = o else { unreachable!() };
    let mut w = RawOut(ByteWriter::new());
    assert!(so.write(&mut w));
    w.0.into_bytes()
}

fn rec<R: OccupantRecord>(o: &RefValue) -> R {
    o.object::<RecordObject<R>>().unwrap().rec.borrow().clone()
}

fn entry_crc(id: u32, payload: &[u8]) -> u32 {
    let mut d = id.to_le_bytes().to_vec();
    d.extend_from_slice(payload);
    crc32_mpeg2(&d)
}

fn token(r: &RefValue) -> Option<(u64, u32)> {
    match r {
        RefValue::Host { token, class_id } => Some((*token, *class_id)),
        _ => None,
    }
}

const T1: &str = "08 00 00 00 00 00 10 49 64 49 64 49 64 4C FE 99 43 80 5F 03 40 80 5F 03 40
01 01 01 01 00 08 00 74 68 25 00 00 00 00 00 CD 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00
02 00 01 00 00 00 00 41 0B 8A 00 41 0B 8A 00 00 00 00 02 80 00 00 0C 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 4A";

#[test]
fn t1_lot_round_trips_byte_exact() {
    let b = hex(T1);
    assert_eq!(b.len(), 118);
    assert_eq!(entry_crc(0x0DFA_3C64, &b), 0x1C85_D610);
    let (o, ok, rem) = read_obj(CLSID_LOT, &b);
    assert!(ok);
    assert_eq!(rem, 0);
    let lot = o.object::<Lot>().unwrap();
    assert!(lot.initialised.get());
    {
        let r = lot.rec.borrow();
        assert_eq!(r.configuration_id, 0);
        assert_eq!(r.flags, lot_flag::HAS_POWER);
        assert_eq!(r.rect, [73, 100, 73, 100]);
        assert_eq!(r.location, [73, 100]);
        assert!((r.altitude - 307.9867).abs() < 1e-3);
        assert!((r.slope_at_altitude - 2.0527).abs() < 1e-3);
        assert_eq!(r.slope, r.slope_at_altitude);
        assert_eq!(r.size, [1, 1]);
        assert_eq!((r.facing, r.state, r.condition, r.zone_type, r.wealth), (1, 1, 0, 8, 0));
        assert_eq!(r.state_day, 2_451_572);
        assert_eq!(r.building_type, 0);
        assert_eq!(r.vacate_reason, 205);
        assert!(r.business_unit.is_null() && r.travel_designate.is_null());
        assert!(r.populations.is_empty() && r.travel_populations.is_empty());
        assert_eq!(r.travel_jobs, [0.0; 3]);
        assert_eq!(r.properties.ids(), vec![0x8A0B_4100]);
        assert!(r.trips.is_empty());
        assert_eq!(r.strategy, 74);
    }
    assert_eq!(write_obj(&o), b);
}

#[test]
fn t2_business_unit_round_trips_byte_exact() {
    let b = hex("01 00 02 00 00 00 0C 40 DA 13 4A 5D BD C9 44 CA B1 11 4A 5D BD C9");
    assert_eq!(entry_crc(0x10BD_D698, &b), 0x73D2_BA7A);
    let (o, ok, rem) = read_obj(CLSID_BUSINESS_UNIT, &b);
    assert!(ok && rem == 0);
    let bu = o.object::<BusinessUnit>().unwrap();
    let lots: Vec<_> = bu.rec.borrow().lots.iter().map(|l| token(l).unwrap()).collect();
    assert_eq!(lots, vec![(0x13DA_400C, CLSID_LOT), (0x11B1_CA44, CLSID_LOT)]);
    assert_eq!(write_obj(&o), b);
}

#[test]
fn business_unit_read_appends_and_rejects_other_versions() {
    let b = hex("01 00 01 00 00 00 0C 40 DA 13 4A 5D BD C9");
    let mut r = BusinessUnitRecord::default();
    let mut t = Tokens;
    assert!(r.read(&mut ByteReader::with_resolver(&b, &mut t)));
    assert!(r.read(&mut ByteReader::with_resolver(&b, &mut t)));
    assert_eq!(r.lots.len(), 2);
    let mut bad = b.clone();
    bad[0] = 2;
    let mut rd = ByteReader::with_resolver(&bad, &mut t);
    assert!(!r.read(&mut rd));
    assert_eq!(rd.position(), 2);
    assert_eq!(r.lots.len(), 2);
}

/// T5 rebuilt from the decoded values of §17.2 (removal 0, highlight 0, flags
/// 5 as observed for every building, no own properties). The spec gives the
/// bounds to 3 decimals; the exact f32 bits here are the unique patterns
/// within 0.0005 of them for which the entry CRC matches (T6 likewise).
fn t5_bytes() -> Vec<u8> {
    hex(&format!(
        "02 00 04 00 00 00 00 05 A0 28 81 27 54 4F 54 4F 02 00 02 00 00 00 00 00
         0C DF BD 67 4A 28 34 65 00 00 73 00  00 00 73 00 00 {} 02 00 00 80 3F",
        "9F43A444 111D9943 C6267544 9F03A544 111D9D43 C6267744"
    ))
}

#[test]
fn t5_building_round_trips() {
    let b = t5_bytes();
    assert_eq!(b.len(), 70);
    assert_eq!(entry_crc(0x00C3_CF20, &b), 0x9175_D64A);
    let (o, ok, rem) = read_obj(CLSID_BUILDING_OCCUPANT, &b);
    assert!(ok && rem == 0);
    let r: BuildingRecord = rec(&o);
    assert_eq!(r.base.occupant_type, OCCUPANT_TYPE_BUILDING);
    assert_eq!(r.base.cell_rect, [84, 79, 84, 79]);
    assert_eq!(r.base.status, 2);
    let k = r.base.holder.parent_key().unwrap();
    assert_eq!(
        (k.group, k.type_id, k.instance),
        (0x67BD_DF0C, 0x6534_284A, 0x0073_0000)
    );
    assert_eq!(r.building_type, 0x0073_0000);
    assert_eq!(r.building_flags, 0);
    assert!((r.bounds.1[1] - 314.227).abs() < 5e-4);
    assert_eq!(r.orientation, 2);
    assert_eq!(r.completion, 1.0);
    assert_eq!(write_obj(&o), b);
}

#[test]
fn building_read_rejects_bad_version_before_base() {
    let mut b = t5_bytes();
    b[0] = 3;
    let mut r = BuildingRecord::new();
    let mut rd = ByteReader::new(&b);
    assert!(!r.read(&mut rd));
    assert_eq!(rd.position(), 2);
    assert_eq!(r, BuildingRecord::new());
    // A bad base version: false after the base version.
    let mut b = t5_bytes();
    b[2] = 3;
    let mut rd = ByteReader::new(&b);
    assert!(!r.read(&mut rd));
    assert_eq!(rd.position(), 4);
}

#[test]
fn t6_base_texture_round_trips() {
    let b = hex(&format!(
        "02 00 04 00 00 00 00 05 9D 6D 7F 49 52 5B 52 5B 02 00 02 00 00 00 00 00
         00 00 00 00 00 00 00 00 00 00 00 00 {} 02 01 00 00 00
         00 30 00 00 4A 6C 00 00 FF B2 33 AA 03 00",
        "33039444 890F9943 3303D844 CDFC9544 B2339943 CDFCD944"
    ));
    assert_eq!(b.len(), 79);
    assert_eq!(entry_crc(0x00BF_7198, &b), 0xB4A1_1718);
    let (o, ok, rem) = read_obj(CLSID_BASE_TEXTURE_OCCUPANT, &b);
    assert!(ok && rem == 0);
    let r: BaseTextureRecord = rec(&o);
    assert_eq!(r.base.occupant_type, OCCUPANT_TYPE_BASE_TEXTURE);
    assert!(r.base.holder.parent_key().is_none());
    assert_eq!(r.usage, 2);
    assert_eq!(
        r.textures,
        vec![TextureRecord {
            texture_instance: 0x3000,
            cell_x: 74,
            cell_z: 108,
            orientation: 0,
            config_byte: 0,
            unnamed: [0xFF, 0xB2, 0x33, 0xAA, 0x03],
            frame: 0,
        }]
    );
    assert_eq!(write_obj(&o), b);
}

#[test]
fn base_texture_set_draws_once_per_unset_multi_frame_record() {
    let mut r = BaseTextureRecord::new();
    let t = |inst, frame| TextureRecord {
        texture_instance: inst,
        frame,
        ..TextureRecord::default()
    };
    let mut draws = Vec::new();
    r.set_textures(
        vec![t(1, 0xFF), t(2, 0xFF), t(3, 5), t(1, 0xFF)],
        &|inst| if inst == 1 { 8 } else { 1 },
        &mut |n| {
            draws.push(n);
            (draws.len() as u32) + 2
        },
    );
    assert_eq!(draws, vec![8, 8]);
    let frames: Vec<u8> = r.textures.iter().map(|t| t.frame).collect();
    assert_eq!(frames, vec![3, 0, 5, 4]);
    assert_eq!(TextureRecord::default().unnamed, [0xFF; 5]);
}

#[test]
fn base_texture_short_read_pads_to_n() {
    let mut r = BaseTextureRecord::new();
    let mut w = RawOut(ByteWriter::new());
    r.textures = vec![TextureRecord::default(); 2];
    assert!(r.write(&mut w));
    let mut b = w.0.into_bytes();
    b.truncate(b.len() - 3);
    let mut back = BaseTextureRecord::new();
    assert!(!back.read(&mut ByteReader::new(&b)));
    assert_eq!(back.textures.len(), 2);
}

fn wall_base() -> OccupantBase {
    let mut b = OccupantBase::new(OCCUPANT_TYPE_RETAINING_WALL);
    b.flags = 5;
    b.cell_rect = [90, 79, 91, 80];
    b
}

#[test]
fn t7_retaining_wall_mask_grows_on_load() {
    let mut w0 = RetainingWallRecord::new();
    w0.base = wall_base();
    w0.wall_type = 0xC96D_2135;
    w0.edge_mask = 9;
    w0.edge_hashes = [0x00F4_39E9, 0x00CF_A447, 0x010D_EF29, 0xE0C0_42C7_u32 as i32];
    w0.position = [1664.0, 299.696, 960.0];
    w0.size = [128.0, 0.2437, 96.0];
    w0.lot = RefValue::Host {
        token: 0x11DE_AF54,
        class_id: CLSID_LOT,
    };
    let mut w = RawOut(ByteWriter::new());
    assert!(w0.write(&mut w));
    let b = w.0.into_bytes();
    assert_eq!(b.len(), 92);
    assert_eq!(&b[40..44], &[9, 0, 0, 0]);
    let (o, ok, rem) = read_obj(CLSID_RETAINING_WALL_OCCUPANT, &b);
    assert!(ok && rem == 0);
    let r: RetainingWallRecord = rec(&o);
    assert_eq!(r.edge_mask, 15);
    assert_eq!(token(&r.lot), Some((0x11DE_AF54, CLSID_LOT)));
    let again = write_obj(&o);
    assert_eq!(&again[40..44], &[15, 0, 0, 0]);
    assert_eq!(&again[..40], &b[..40]);
    assert_eq!(&again[44..], &b[44..]);
}

#[test]
fn retaining_wall_mask_keeps_bits_of_zero_hashes() {
    let mut w0 = RetainingWallRecord::new();
    w0.edge_mask = 0b1010;
    w0.edge_hashes = [7, 0, 0, 0];
    let mut w = RawOut(ByteWriter::new());
    assert!(w0.write(&mut w));
    let b = w.0.into_bytes();
    let mut r = RetainingWallRecord::new();
    r.lot = RefValue::Host {
        token: 1,
        class_id: CLSID_LOT,
    };
    assert!(r.read(&mut ByteReader::new(&b)));
    assert_eq!(r.edge_mask, 0b1011);
    assert!(r.lot.is_null());
    // A rejected version still releases the old lot (literal order, §7).
    r.lot = RefValue::Host {
        token: 1,
        class_id: CLSID_LOT,
    };
    let mut bad = b.clone();
    bad[0] = 1;
    assert!(!r.read(&mut ByteReader::new(&bad)));
    assert!(r.lot.is_null());
}

#[test]
fn t8_foundation_round_trips() {
    let mut f0 = FoundationRecord::new();
    f0.base.flags = 5;
    f0.base.cell_rect = [89, 88, 90, 88];
    f0.foundation_type = 0x890B_7313;
    f0.max_height = 32.0;
    f0.position = [1656.0, 304.792, 1560.0];
    f0.size = [16.0, 4.1668, 48.0];
    f0.usage = 2;
    f0.orientation = 2;
    let mut w = RawOut(ByteWriter::new());
    assert!(f0.write(&mut w));
    let b = w.0.into_bytes();
    assert_eq!(b.len(), 70);
    assert_eq!(&b[..2], &[2, 0]);
    assert_eq!(&b[36..40], &[0x13, 0x73, 0x0B, 0x89]);
    assert_eq!(&b[68..], &[2, 2]);
    let (o, ok, rem) = read_obj(CLSID_FOUNDATION_OCCUPANT, &b);
    assert!(ok && rem == 0);
    let mut r: FoundationRecord = rec(&o);
    assert_eq!(r.base.status, 2);
    r.base.status = 0;
    assert_eq!(r, f0);
    assert_eq!(write_obj(&o), b);
}

#[test]
fn bare_occupant_is_the_base_block() {
    let mut r0 = BareOccupantRecord::new();
    r0.base.flags = 5;
    r0.base.cell_rect = [64, 64, 64, 64];
    let mut w = RawOut(ByteWriter::new());
    assert!(r0.write(&mut w));
    let b = w.0.into_bytes();
    assert_eq!(b.len(), 34);
    assert_eq!(&b[..12], &hex("04 00 00 00 00 05 25 89 75 74 40 40"));
    let (o, ok, rem) = read_obj(CLSID_OCCUPANT, &b);
    assert!(ok && rem == 0);
    assert_eq!(rec::<BareOccupantRecord>(&o).base.occupant_type, OCCUPANT_TYPE_BARE);
    assert_eq!(write_obj(&o), b);
}

#[test]
fn occupant_flags_write_clears_bits_1_and_5() {
    let mut r0 = FoundationRecord::new();
    r0.base.flags = 0xFF;
    let mut w = RawOut(ByteWriter::new());
    assert!(r0.write(&mut w));
    assert_eq!(w.0.bytes()[7], 0xDD);
}

#[test]
fn occupant_init_after_load_and_after_create() {
    let mut fresh = OccupantBase::new(OCCUPANT_TYPE_BUILDING);
    assert_eq!(occupant_init(&mut fresh), InitOutcome::PostCreateOccupant);
    assert_eq!(occupant_init(&mut fresh), InitOutcome::AlreadyInitialised);
    let b = t5_bytes();
    let (o, _, _) = read_obj(CLSID_BUILDING_OCCUPANT, &b);
    let obj = o.object::<RecordObject<BuildingRecord>>().unwrap();
    let mut r = obj.rec.borrow_mut();
    r.base.highlight = 3;
    r.base.status = (3 << 4) | STATUS_LOADED;
    assert_eq!(occupant_init(r.base_mut()), InitOutcome::HighlightCleared);
    assert_eq!(
        (r.base.highlight, r.base.status),
        (0, STATUS_LOADED | STATUS_INITIALISED)
    );
    assert_eq!(occupant_init(r.base_mut()), InitOutcome::AlreadyInitialised);
}

#[test]
fn t9_lot_rejects_version_9_after_two_bytes() {
    let mut b = hex(T1);
    b[0] = 9;
    let (o, ok, rem) = read_obj(CLSID_LOT, &b);
    assert!(!ok);
    assert_eq!(rem, b.len() - 2);
    let lot = o.object::<Lot>().unwrap();
    assert!(lot.initialised.get());
    assert_eq!(lot.rec.borrow().altitude, 0.0);
}

/// T1 with one population map, one travel map entry, two trips and a
/// business unit, rebuilt through the writer.
fn rich_lot() -> LotRecord {
    let mut r = LotRecord {
        state: 3,
        building_type: 0x0073_0000,
        business_unit: RefValue::Host {
            token: 0x10BD_D698,
            class_id: CLSID_BUSINESS_UNIT,
        },
        travel_designate: RefValue::Host {
            token: 0x0DFA_3C64,
            class_id: CLSID_LOT,
        },
        travel_jobs: [12.5, 1.0, 0.25],
        strategy: 3,
        ..LotRecord::default()
    };
    r.populations = vec![[(0x1020, 7), (0x1010, 3)].into_iter().collect()];
    r.travel_populations = [(0x1010, 2)].into_iter().collect();
    let mut t = TripData::new(4, 1, 1, 0, 0);
    t.route = vec![1, 2, 3];
    t.travel_types_used = 0xCCCC_CCCC;
    r.trips = vec![t.clone(), TripData::new(9, 3, 2, 1, 2)];
    r
}

fn write_lot(r: &LotRecord) -> Vec<u8> {
    let mut w = RawOut(ByteWriter::new());
    assert!(r.write(&mut w));
    w.0.into_bytes()
}

#[test]
fn lot_round_trips_maps_refs_and_trips() {
    let r0 = rich_lot();
    let b = write_lot(&r0);
    let (o, ok, rem) = read_obj(CLSID_LOT, &b);
    assert!(ok && rem == 0);
    let lot = o.object::<Lot>().unwrap();
    {
        let r = lot.rec.borrow();
        assert_eq!(r.populations, r0.populations);
        assert_eq!(r.travel_populations, r0.travel_populations);
        assert_eq!(r.trips, r0.trips);
        assert_eq!(token(&r.business_unit), Some((0x10BD_D698, CLSID_BUSINESS_UNIT)));
        assert_eq!(token(&r.travel_designate), Some((0x0DFA_3C64, CLSID_LOT)));
    }
    assert_eq!(write_obj(&o), b);
    // Map entries are written in ascending key order.
    let pos = b.windows(4).position(|w| w == [0x10, 0x10, 0, 0]).unwrap();
    assert_eq!(&b[pos - 2..pos], &[1, 2]);
}

#[test]
fn t10_lot_read_appends_trips_and_merges_maps() {
    let b = write_lot(&rich_lot());
    let lot = Lot::new(LotRecord {
        trips: vec![TripData::new(1, 0, 0, 0, 0)],
        populations: vec![[(0x1010, 99), (0x4100, 5)].into_iter().collect(), PopulationMap::new()],
        travel_populations: [(0x3110, 1)].into_iter().collect(),
        ..LotRecord::default()
    });
    let mut t = Tokens;
    assert!(lot.read(&mut ByteReader::with_resolver(&b, &mut t)));
    let r = lot.rec.borrow();
    assert_eq!(r.trips.len(), 3);
    assert_eq!(r.trips[0].count, 1);
    // Resized to P = 1; the kept map gets the read entries inserted.
    assert_eq!(r.populations.len(), 1);
    let m: Vec<_> = r.populations[0].iter().map(|(&k, &v)| (k, v)).collect();
    assert_eq!(m, vec![(0x1010, 3), (0x1020, 7), (0x4100, 5)]);
    assert_eq!(r.travel_populations.len(), 2);
}

#[test]
fn lot_trip_read_failure_returns_false() {
    let mut b = write_lot(&rich_lot());
    b.truncate(b.len() - 3);
    let (_, ok, _) = read_obj(CLSID_LOT, &b);
    assert!(!ok);
}

#[test]
fn lot_versions_6_and_7_skip_trips_and_strategy() {
    // Version 7: trips but no strategy byte.
    let r0 = LotRecord {
        strategy: 0,
        ..rich_lot()
    };
    let mut b = write_lot(&r0);
    b.pop();
    b[0] = 7;
    let (o, ok, rem) = read_obj(CLSID_LOT, &b);
    assert!(ok && rem == 0);
    assert_eq!(o.object::<Lot>().unwrap().rec.borrow().trips.len(), 2);
    // Version 6: neither (the trip count and trips are absent too).
    let mut b6 = write_lot(&LotRecord::default());
    b6.truncate(b6.len() - 5);
    b6[0] = 6;
    let (_, ok, rem) = read_obj(CLSID_LOT, &b6);
    assert!(ok && rem == 0);
}

#[test]
fn lot_legacy_layout_below_version_6() {
    let mut w = RawOut(ByteWriter::new());
    w.put_u16(5);
    w.put_u32(0x1234);
    w.put_u8(0x10);
    for v in [3, 4, 5, 6, 3, 4] {
        w.put_i32(v);
    }
    for f in [300.0f32, 1.0, 2.0] {
        w.put_f32(f);
    }
    w.put_i32(3);
    w.put_i32(3);
    for v in [1u8, 3, 0, 1, 2] {
        w.put_u8(v);
    }
    w.put_bool(true); // vertices locked
    w.put_u32(2_451_545);
    w.put_u32(0x55);
    w.put_bool(true); // needs power
    w.put_bool(false); // has power: clears bit 4
    w.put_bool(true); // historical
    w.put_u8(0);
    w.put_u32(0);
    w.put_u32(0);
    w.put_u8(0);
    w.put_u8(0);
    for _ in 0..3 {
        w.put_f32(0.0);
    }
    PropertyHolderWrite::write_empty(&mut w);
    let b = w.0.into_bytes();
    let (o, ok, rem) = read_obj(CLSID_LOT, &b);
    assert!(ok && rem == 0);
    let lot = o.object::<Lot>().unwrap();
    let r = lot.rec.borrow();
    assert_eq!(r.rect, [3, 4, 5, 6]);
    assert_eq!(r.location, [3, 4]);
    assert_eq!(r.size, [3, 3]);
    assert_eq!(
        r.flags,
        lot_flag::VERTICES_LOCKED | lot_flag::NEEDS_POWER | lot_flag::HISTORICAL
    );
    assert_eq!(r.building_type, 0x55);
}

struct PropertyHolderWrite;

impl PropertyHolderWrite {
    fn write_empty(out: &mut dyn OutStream) {
        assert!(crate::property::PropertyHolder::new().write(out));
    }
}

#[test]
fn lot_write_stops_before_strategy_on_trip_failure() {
    // A stream that fails after a byte budget.
    struct Budget(ByteWriter, usize);
    impl OutStream for Budget {
        fn put_u8(&mut self, v: u8) -> bool {
            self.put_bytes(&[v])
        }
        fn put_u16(&mut self, v: u16) -> bool {
            self.put_bytes(&v.to_le_bytes())
        }
        fn put_u32(&mut self, v: u32) -> bool {
            self.put_bytes(&v.to_le_bytes())
        }
        fn put_u64(&mut self, v: u64) -> bool {
            self.put_bytes(&v.to_le_bytes())
        }
        fn put_f32(&mut self, v: f32) -> bool {
            self.put_bytes(&v.to_le_bytes())
        }
        fn put_f64(&mut self, v: f64) -> bool {
            self.put_bytes(&v.to_le_bytes())
        }
        fn put_bytes(&mut self, v: &[u8]) -> bool {
            if self.0.len() + v.len() > self.1 {
                self.1 = 0;
                return false;
            }
            self.0.put_bytes(v)
        }
        fn put_str(&mut self, v: &[u8]) -> bool {
            self.put_bytes(v)
        }
        fn put_ref(&mut self, _r: &RefValue) -> bool {
            self.put_u32(0)
        }
        fn error(&self) -> i32 {
            i32::from(self.1 == 0)
        }
    }
    let r = LotRecord {
        trips: vec![TripData::default()],
        strategy: 0x4A,
        ..LotRecord::default()
    };
    let full = write_lot(&r);
    let mut tw = RawOut(ByteWriter::new());
    assert!(write_trip(&mut tw, &TripData::default()));
    let before_trip = full.len() - 1 - tw.0.len();
    let mut s = Budget(ByteWriter::new(), before_trip + 2);
    assert!(!r.write(&mut s));
    assert!(s.0.len() <= before_trip + 2);
    assert!(!s.0.bytes().ends_with(&[0x4A]));
}

#[test]
fn create_covers_the_module_classes_only() {
    for c in [
        CLSID_LOT,
        CLSID_BUSINESS_UNIT,
        CLSID_OCCUPANT,
        CLSID_BUILDING_OCCUPANT,
        CLSID_BASE_TEXTURE_OCCUPANT,
        CLSID_RETAINING_WALL_OCCUPANT,
        CLSID_FOUNDATION_OCCUPANT,
    ] {
        let RefValue::Object(o) = create(c).unwrap() else {
            panic!()
        };
        assert_eq!(o.class_id(), c);
    }
    assert!(create(0x2977_AA47).is_none());
}
