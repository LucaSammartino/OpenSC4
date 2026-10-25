// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::neighbors::{deal_type, DealData};
use crate::stream::{ByteReader, ByteWriter, InStream, OutStream, ResKey};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct Life(Vec<(u32, bool)>);
impl Lifecycle for Life {
    fn subscribe(&mut self, m: u32, on: bool) {
        self.0.push((m, on));
    }
}
fn context() -> ReadContext<'static> {
    ReadContext {
        application_default_difficulty: 1,
        default_city_name: b"New City",
        localized_city_name: None,
        localized_mayor_name: None,
        map_tutorial: |v| v,
    }
}
fn bytes(c: &RegionalCity) -> Vec<u8> {
    let mut out = ByteWriter::new();
    assert!(c.write_record(&mut out));
    out.into_bytes()
}
fn change(t: u32, x: i32, z: i32, add: u8) -> PendingChange {
    PendingChange {
        conn_type: t,
        x,
        z,
        add,
    }
}
fn deal(a: (i32, i32), b: (i32, i32), t: u32, amount: i32) -> Rc<NeighborDeal> {
    NeighborDeal::from_data(DealData {
        city_side: a,
        other_side: b,
        deal_type: t,
        amount,
        flags: 0,
        ..DealData::default()
    })
}

#[test]
fn defaults_identity_life_and_simple_slots() {
    let mut c = RegionalCity::new(1);
    let mut h = Life::default();
    assert_eq!(c.difficulty, 1);
    assert_eq!(c.size, (1, 1));
    assert_eq!(c.tax_rates, [7.0; 12]);
    assert_eq!(c.commodities.len(), 3);
    assert_eq!(*c.get_workforce_percentage(), 0.5);
    assert!(std::ptr::eq(c.get_workforce_percentage(), &c.workforce));
    c.set_world_position(12.0, 34.0);
    c.set_world_base_elevation(56.0);
    assert_eq!(c.get_world_position(), (12.0, 34.0));
    assert_eq!(c.get_world_base_elevation(), 56.0);
    assert_eq!(c.get_world_hemisphere(), 1);
    c.set_world_position(78.0, 90.0);
    assert_eq!(c.get_world_base_elevation(), 56.0);
    c.set_city_name(b"Copied");
    let mut copy = Vec::new();
    c.get_city_name(&mut copy);
    copy[0] = b'X';
    assert_eq!(c.city_name, b"Copied");
    for iid in [1, INTERFACE_ID, 0x452294AA, 0xE4FDA3D4] {
        assert!(c.query_interface(iid));
    }
    assert!(!c.query_interface(55));
    assert_eq!(c.references, 4);
    c.init(&mut h);
    c.init(&mut h);
    assert_eq!(h.0.len(), 5);
    c.connections.push(NeighborConnection::new());
    c.local_deals.push(NeighborDeal::new());
    for n in (0..4).rev() {
        assert_eq!(c.release(&mut h), n);
    }
    assert_eq!(h.0.len(), 10);
    assert!(c.connections.is_empty());
    assert!(c.local_deals.is_empty());
    c.shutdown(&mut h);
    assert_eq!(h.0.len(), 10);
    c.world[1] = f32::NAN;
    assert_eq!(c.world_hemisphere(), 2);
    c.world[1] = 1.0;
    assert_eq!(c.world_hemisphere(), 1);
    assert!(!c.is_tutorial());
    c.tutorial_id = 0;
    assert!(c.is_tutorial());
    let mut locale = (7, 7);
    assert!(c.get_original_language_and_country(&mut locale));
    assert_eq!(locale, (0, 0));
    let p = RegionalCity::placeholder(2, (3, 4), 1, b"New City", &mut h);
    assert!(p.initialized);
    assert_eq!(p.size, (2, 2));
    assert_eq!(p.position, (3, 4));
    assert!(!p.moved);
    assert!(p.file_path.is_empty());
    assert_eq!(p.serial, 0);
}

#[test]
fn statistics_conformance_and_insertion_quirks() {
    let mut c = RegionalCity::new(2);
    c.potential.insert(1, 1500);
    c.extrapolated.insert(1, 1800);
    assert_eq!(c.get_allowable_extrapolation(1), 0);
    c.extrapolated.insert(1, 1200);
    assert_eq!(c.get_allowable_extrapolation(1), 300);
    assert_eq!(c.get_population(99), 0);
    assert!(!c.populations.contains_key(&99));
    c.populations.insert(99, 500);
    assert_eq!(c.get_extrapolated_population(99), 500);
    c.extrapolate_growth(99, 3.9);
    assert_eq!(c.extrapolated[&99], 3);
    c.get_allowable_extrapolation(88);
    assert_eq!(c.potential[&88], 0);
    assert_eq!(c.extrapolated[&88], 0);
    c.populations = [(1, 500), (2, 2000), (3, 0), (4, -1)].into();
    c.reset_growth();
    assert_eq!(
        (c.potential[&1], c.potential[&2], c.potential[&3], c.potential[&4]),
        (1000, 2200, 0, 0)
    );
    c.set_produced(1, 10.0);
    c.set_imported(1, 5.0);
    c.set_demanded(1, 8.0);
    c.set_exported(1, 2.0);
    assert_eq!(c.get_commodity_balance(1), 5.0);
    c.set_exported(1, -3.0);
    assert_eq!(c.get_exported(1), 0.0);
    c.set_produced(44, f64::NAN);
    assert!(c.get_produced(44).is_nan());
    assert_eq!(c.get_imported(45), 0.0);
    assert!(c.commodities.contains_key(&45));
    c.tax_rates[6] = 9.0;
    assert_eq!(c.get_tax_rate(0x3320), 9.0);
    assert_eq!(c.get_tax_rate(0x9999), 7.0);
}

#[test]
fn commodity_balance_rounds_operations_without_rounding_loaded_doubles() {
    let mut c = RegionalCity::new(2);
    // The intermediate produced+imported is rounded before subtracting.
    c.set_produced(3, 16_777_217.0);
    c.set_imported(3, 0.0);
    c.set_demanded(3, 16_777_216.0);
    c.set_exported(3, 0.0);
    assert_eq!(c.get_commodity_balance(3), 0.0);
    // A decoded record may contain signed values, without setter clamping.
    // Rounding these loads before adding would give zero instead of one.
    c.commodity(3).imported = -16_777_216.0;
    c.set_demanded(3, 0.0);
    assert_eq!(c.get_commodity_balance(3), 1.0);
    c.set_produced(3, 16_777_216.0);
    c.set_imported(3, 1.0);
    c.set_demanded(3, 16_777_216.0);
    assert_eq!(c.get_commodity_balance(3), 0.0);
    c.set_produced(3, 0.0);
    c.set_imported(3, 0.0);
    c.set_demanded(3, 11_584.583251953125);
    assert_eq!(c.get_commodity_balance(3), -11_584.5830078125);
}

#[test]
fn moving_and_copy_outputs() {
    let mut c = RegionalCity::new(2);
    c.position = (3, 4);
    let d = deal((3, 4), (2, 4), deal_type::BUY_POWER, 1);
    c.other_deals.push(d.clone());
    let conn = NeighborConnection::with(2, -1, 5);
    c.connections.push(conn.clone());
    c.set_position(5, 4, true);
    assert!(c.moved);
    assert_eq!(d.city_side(), (5, 4));
    assert_eq!(d.other_side(), (4, 4));
    assert_eq!(conn.other_region_cell(), (4, 4));
    assert!(Rc::ptr_eq(c.find_connection(2, -1, 5).unwrap(), &conn));
    assert!(c.find_connection(1, -1, 5).is_none());
    let mut list = vec![NeighborConnection::new()];
    c.get_all_connections(&mut list);
    assert!(Rc::ptr_eq(&list[0], &conn));
    let mut points = vec![[1.0; 3]];
    c.get_points_of_interest(99, &mut points);
    assert_eq!(points, [[1.0; 3]]);
    c.points.insert(0, vec![[2.0; 3]]);
    c.get_points_of_interest(0, &mut points);
    assert_eq!(points, [[2.0; 3]]);
}

#[derive(Default)]
struct Region {
    current: Option<CityId>,
    cells: BTreeMap<(i32, i32), CityId>,
}
impl DealRegion for Region {
    fn current_city(&self) -> Option<CityId> {
        self.current
    }
    fn city_at(&self, p: (i32, i32)) -> Option<CityId> {
        self.cells.get(&p).copied()
    }
}
#[test]
fn claiming_and_import_export_vectors() {
    let h = Region {
        current: Some(1),
        cells: [((0, 0), 1), ((1, 0), 2), ((2, 0), 3)].into(),
    };
    let xc = deal((0, 0), (1, 0), deal_type::BUY_POWER, 100);
    let cd = deal((1, 0), (2, 0), deal_type::BUY_WATER, 50);
    let xd = deal((0, 0), (2, 0), deal_type::BUY_GARBAGE, 25);
    let broken = deal((0, 0), (99, 0), deal_type::BUY_POWER, 9);
    let mut c = RegionalCity::new(2);
    c.other_deals = vec![xc.clone(), cd.clone(), broken.clone()];
    for d in &c.other_deals {
        d.set_flags(flag::SILENT, true);
    }
    c.update_local_deals(2, &h);
    assert_eq!(c.local_deals.len(), 1);
    assert_eq!(c.other_deals.len(), 1);
    assert!(Rc::ptr_eq(&c.other_deals[0], &cd));
    assert!(!cd.is_silent());
    xc.set_flags(flag::SILENT, false);
    c.update_import_export(2, &h);
    assert_eq!(c.get_exported(1), 100.0);
    let mut x = RegionalCity::new(2);
    let mut src = vec![xc.clone(), cd.clone(), xd.clone(), broken];
    x.set_local_deals(&mut src, 1, &h);
    assert_eq!(src.len(), 3);
    assert_eq!(x.local_deals.len(), 2);
    assert_eq!(x.get_imported(1), 100.0);
    assert_eq!(x.get_imported(3), 25.0);
    xc.set_flags(flag::CANCELLED, true);
    x.update_import_export(1, &h);
    assert_eq!(x.get_imported(1), 0.0);
    x.return_local_deals();
    assert!(Rc::ptr_eq(&x.other_deals[0], &xd));
    assert!(Rc::ptr_eq(&x.other_deals[1], &xc));
    x.local_deals = vec![deal((0, 0), (1, 0), 99, 100)];
    x.other_deals.clear();
    x.update_import_export(1, &h);
    assert_eq!(x.get_imported(1), 0.0);
    let none = Region::default();
    let mut list = vec![cd];
    x.set_local_deals(&mut list, 1, &none);
    assert!(x.local_deals.is_empty());
    assert_eq!(list.len(), 1);
}

#[test]
fn wire_round_trip_header_and_object_read() {
    let mut c = RegionalCity::new(2);
    c.established = true;
    c.position = (6, 5);
    c.size = (2, 2);
    c.population = 2564;
    c.commercial_jobs = 297;
    c.industrial_jobs = 652;
    c.workforce = 0.48;
    c.mayor_rating = 51;
    c.serial = 0x53E3CBFB;
    c.city_name = b"Town".to_vec();
    c.unknown_strings[1] = b"opaque".to_vec();
    c.connections = vec![NeighborConnection::with(2, -1, 5), NeighborConnection::with(9, 64, 10)];
    c.pending = vec![change(9, 100, 128, 1), change(9, 99, 128, 1)];
    c.commodities.remove(&3);
    c.set_imported(1, 5.0);
    c.set_exported(1, 7.0);
    c.other_deals = vec![deal((6, 5), (4, 5), deal_type::BUY_POWER, 100)];
    c.local_deals = vec![deal((6, 5), (6, 3), deal_type::SELL_WATER, 30)];
    c.points = [(0, vec![[1.0, 2.0, 3.0]]), (1, vec![[4.0, 5.0, 6.0]])].into();
    let b = bytes(&c);
    let decoded = RegionalCity::decode_record(&mut ByteReader::new(&b), 1).unwrap();
    assert_eq!(bytes(&decoded), b);
    assert_eq!(decoded.local_deals.len(), 1);
    let mut with_prefix = vec![1, 0];
    with_prefix.extend(&b);
    assert_eq!(
        &with_prefix[..43],
        &[
            1, 0, 13, 0, 6, 0, 0, 0, 5, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 4, 10, 0, 0, 41, 1, 0, 0, 140, 2, 0, 0, 143,
            194, 245, 62, 51, 2, 0, 251, 203, 227, 83
        ]
    );
    let mut life = Life::default();
    let obj = RegionalCity::read_record(&mut ByteReader::new(&b), &context(), &mut life).unwrap();
    assert_eq!(obj.other_deals.len(), 2);
    assert!(obj.local_deals.is_empty());
    assert!(obj.other_deals.iter().all(|d| d.is_silent()));
    assert_eq!(life.0.len(), 5);
    for end in 0..b.len() {
        assert!(
            RegionalCity::decode_record(&mut ByteReader::new(&b[..end]), 1).is_err(),
            "prefix {end}"
        );
    }
    let mut life = Life::default();
    let mut c = RegionalCity::new(2);
    c.tutorial_id = 99;
    let ctx = ReadContext {
        localized_city_name: Some(b"Localized"),
        localized_mayor_name: Some(b"Mayor"),
        map_tutorial: |v| v + 1,
        ..context()
    };
    let c = RegionalCity::read_record(&mut ByteReader::new(&bytes(&c)), &ctx, &mut life).unwrap();
    assert_eq!(c.city_name, b"Localized");
    assert_eq!(c.mayor_name, b"Mayor");
    assert_eq!(c.difficulty, 1);
    assert_eq!(c.tutorial_id, 100);
}

#[test]
fn version_gates_and_collection_limits() {
    let base = bytes(&RegionalCity::new(2));
    for version in [9_u16, 10, 11, 12, 13, 14] {
        let mut b = base.clone();
        b[..2].copy_from_slice(&version.to_le_bytes());
        if version < 13 {
            b.remove(35);
        }
        if version == 10 {
            b[30..34].copy_from_slice(&48_f32.to_le_bytes());
        }
        if version < 10 {
            b.drain(30..34);
        }
        if version < 12 {
            b.truncate(b.len() - 4);
        }
        let result = RegionalCity::decode_record(&mut ByteReader::new(&b), 1);
        if version == 14 {
            assert_eq!(result.unwrap_err(), RecordError::Version);
        } else {
            let c = result.unwrap();
            assert_eq!(c.difficulty, 2);
            assert_eq!(c.workforce, if version == 10 { 48.0 * 0.01 } else { 0.5 });
        }
    }
    // Empty record field offsets are derived by parsing, rather than magic map offsets.
    let mut inp = ByteReader::new(&base);
    inp.skip(2 + 16 + 12 + 4 + 3 + 4 + 16 + 4 + 1);
    for _ in 0..7 {
        inp.get_str().unwrap();
    }
    inp.skip(16);
    let map_start = inp.position();
    for (offset, bad) in [
        (map_start, 0x20000001_u32),
        (map_start + 12, 13),
        (map_start + 64, 0x100001),
        (map_start + 68, 0x100001),
        (map_start + 88, 0x100001),
    ] {
        let mut b = base.clone();
        b[offset..offset + 4].copy_from_slice(&bad.to_le_bytes());
        assert_eq!(
            RegionalCity::decode_record(&mut ByteReader::new(&b), 1).unwrap_err(),
            RecordError::Limit,
            "offset {offset}"
        );
    }
}

#[derive(Default)]
struct Pack {
    records: BTreeMap<ResKey, Vec<u8>>,
    unreadable: BTreeSet<ResKey>,
    writes: usize,
    closed: bool,
    finish: bool,
}
impl Package for Pack {
    fn open(&mut self, _: &[u8], _: bool) -> bool {
        true
    }
    fn exists(&mut self, k: ResKey) -> bool {
        self.records.contains_key(&k) || self.unreadable.contains(&k)
    }
    fn read(&mut self, k: ResKey) -> Option<Vec<u8>> {
        if self.unreadable.contains(&k) {
            None
        } else {
            self.records.get(&k).cloned()
        }
    }
    fn write(&mut self, k: ResKey, b: &[u8]) -> bool {
        self.writes += 1;
        self.records.insert(k, b.to_vec());
        true
    }
    fn finish(&mut self) -> bool {
        self.finish
    }
    fn close(&mut self) {
        self.closed = true;
    }
}
#[test]
fn path_gate_output_preservation_finish_and_one_record() {
    let mut c = RegionalCity::new(2);
    c.serial = 123;
    for v in [61_u32, 62, 65, 66] {
        let mut p = Pack {
            finish: true,
            ..Pack::default()
        };
        p.records.insert(VERSION_KEY, v.to_le_bytes().to_vec());
        assert_eq!(write_path(&mut p, b"city", &c), (62..=65).contains(&v));
        assert_eq!(p.records[&VERSION_KEY], v.to_le_bytes());
        assert_eq!(p.writes, usize::from((62..=65).contains(&v)));
    }
    let mut p = Pack {
        finish: true,
        ..Pack::default()
    };
    assert!(write_path(&mut p, b"city", &c));
    let mut out = None;
    let mut life = Life::default();
    assert!(read_path(&mut p, b"city", &mut out, &context(), &mut life));
    assert_eq!(out.as_ref().unwrap().serial, 123);
    assert_eq!(out.as_ref().unwrap().references, 1);
    assert!(out.as_ref().unwrap().file_path.is_empty());
    p.records.remove(&RECORD_KEY);
    assert!(!read_package(&mut p, &mut out, &context(), &mut life));
    assert_eq!(out.as_ref().unwrap().serial, 123);
    p.unreadable.insert(VERSION_KEY);
    assert!(!write_path(&mut p, b"city", &c));
    p.unreadable.clear();
    p.finish = false;
    p.closed = false;
    assert!(!write_path(&mut p, b"city", &c));
    assert!(!p.closed);
    assert!(p.records.contains_key(&RECORD_KEY));
    assert!(!read_path(&mut p, b"city", &mut out, &context(), &mut life));
    assert_eq!(out.unwrap().serial, 123);
}

#[test]
fn pending_layout_conversion_duplicates_sort_and_pairs() {
    let mut c = RegionalCity::new(2);
    c.size = (2, 2);
    c.position = (2, 6);
    let road = NeighborConnection::with(2, 34, -1);
    c.change_symmetric_connection(&road, (2, 8), (2, 2), 1);
    c.change_symmetric_connection(&road, (2, 8), (2, 2), 0);
    assert_eq!(c.pending, [change(2, 34, 128, 1), change(2, 34, 128, 0)]);
    let mut out = ByteWriter::new();
    out.put_u32(2);
    for p in [change(9, 100, 128, 1), change(9, 99, 128, 1)] {
        assert!(p.write(&mut out));
    }
    assert_eq!(
        out.bytes(),
        &[2, 0, 0, 0, 9, 0, 0, 0, 100, 0, 0, 0, 128, 0, 0, 0, 1, 9, 0, 0, 0, 99, 0, 0, 0, 128, 0, 0, 0, 1]
    );
    c.size = (1, 1);
    for flags in [vec![1, 0], vec![0, 1], vec![1, 1, 0]] {
        c.pending = flags.iter().map(|&f| change(2, -1, 5, f)).collect();
        let p = c.prepare_changes(true);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].change.add, *flags.last().unwrap());
    }
    c.pending = vec![
        change(2, 64, 10, 1),
        change(2, -1, 5, 1),
        change(2, 3, -1, 1),
        change(2, 7, 64, 1),
    ];
    assert_eq!(
        c.prepare_changes(true)
            .iter()
            .map(|p| (p.change.x, p.change.z))
            .collect::<Vec<_>>(),
        [(-1, 5), (3, -1), (64, 10), (7, 64)]
    );
    c.pending = vec![
        change(3, -1, 10, 1),
        change(3, -1, 11, 1),
        change(3, 10, -1, 1),
        change(3, 11, -1, 1),
    ];
    assert_eq!(
        c.prepare_changes(true)
            .iter()
            .map(|p| (p.pair, p.direction))
            .collect::<Vec<_>>(),
        [(false, false), (true, true), (false, true), (true, false)]
    );
    assert!(c
        .prepare_changes(true)
        .iter()
        .zip(c.prepare_changes(false))
        .all(|(a, b)| a.direction != b.direction));
}

#[derive(Default)]
struct Services {
    existing: BTreeSet<(u32, i32, i32)>,
    words: BTreeMap<u64, u32>,
    cells: BTreeMap<(u32, i32, i32), Vec<u64>>,
    events: Vec<String>,
    incoming: Vec<IncomingCity>,
    no_network: bool,
    pole: bool,
    occupied: BTreeSet<(i32, i32)>,
}
impl ConnectionHost for Services {
    fn drive_side(&self) -> bool {
        true
    }
    fn other_city_established(&mut self, _: &NeighborConnection) -> bool {
        true
    }
    fn adjacent_connections(&mut self) -> Vec<IncomingCity> {
        std::mem::take(&mut self.incoming)
    }
    fn points_to_this_city(&mut self, _: &NeighborConnection) -> bool {
        true
    }
    fn network_exists(&mut self, _: u32) -> bool {
        !self.no_network
    }
    fn find_connection(&mut self, p: PendingChange) -> bool {
        self.existing.contains(&(p.conn_type, p.x, p.z))
    }
    fn create_connection(&mut self, p: PendingChange, _: bool) {
        self.events.push("create".into());
        self.existing.insert((p.conn_type, p.x, p.z));
    }
    fn delete_connection(&mut self, p: PendingChange) {
        self.events.push("delete".into());
        self.existing.remove(&(p.conn_type, p.x, p.z));
    }
    fn traffic_present(&mut self, p: (i32, i32)) -> bool {
        self.occupied.contains(&p)
    }
    fn power_present(&mut self, p: (i32, i32)) -> bool {
        self.occupied.contains(&p)
    }
    fn demolish(&mut self, p: &[(i32, i32)], layer: u32) {
        self.events.push(format!("demolish:{layer}:{p:?}"));
    }
    fn dezone(&mut self, _: &[(i32, i32)]) {
        self.events.push("dezone".into());
    }
    fn place_segment(&mut self, _: u32, _: (i32, i32), _: (i32, i32)) {
        self.events.push("place".into());
    }
    fn pole_present(&mut self, _: (i32, i32)) -> bool {
        self.pole
    }
    fn occupants(&mut self, n: u32, p: (i32, i32)) -> Vec<u64> {
        self.cells.get(&(n, p.0, p.1)).cloned().unwrap_or_default()
    }
    fn connection_word(&mut self, o: u64) -> u32 {
        self.words[&o]
    }
    fn set_connection_word(&mut self, o: u64, word: u32) {
        self.words.insert(o, word);
    }
    fn post(&mut self, m: u32) {
        assert_eq!(m, CONNECTIONS_READY);
        self.events.push("ready".into());
    }
}
impl CacheHost for Services {
    fn mayor_name(&mut self) -> Vec<u8> {
        self.events.push("mayor".into());
        b"Mayor".to_vec()
    }
    fn city_name(&mut self) -> Vec<u8> {
        self.events.push("city".into());
        b"Town".to_vec()
    }
    fn difficulty(&mut self) -> u8 {
        self.events.push("difficulty".into());
        1
    }
    fn file_path(&mut self) -> Vec<u8> {
        self.events.push("path".into());
        b"C:/Town.sc4".to_vec()
    }
    fn serial(&mut self) -> u32 {
        99
    }
    fn established(&mut self) -> bool {
        true
    }
    fn mayor_rating(&mut self) -> i8 {
        -5
    }
    fn tax_rate(&mut self, i: usize) -> f32 {
        i as f32
    }
    fn connections(&mut self) -> Vec<Rc<NeighborConnection>> {
        vec![NeighborConnection::with(2, -1, 10)]
    }
    fn update_commodity_stats(&mut self, c: &mut RegionalCity) {
        self.events.push("commodities".into());
        c.set_produced(1, 75.0);
    }
    fn tutorial_id(&mut self) -> Option<i32> {
        Some(7)
    }
    fn residential(&mut self) -> Option<(i32, f32)> {
        Some((500, 0.48))
    }
    fn population_summary(&mut self) -> BTreeMap<u32, i32> {
        [(0x3111, 100), (0x4101, 50)].into()
    }
    fn jobs_per_unit(&mut self, _: u32) -> [f32; 4] {
        [99.0, 0.2, 0.3, 0.5]
    }
    fn lots(&mut self, _: u32) -> Vec<(bool, [f32; 3])> {
        vec![(true, [1.0, 2.0, 3.0]), (false, [9.0; 3])]
    }
}

#[test]
fn apply_connections_ferry_rollback_skip_pairs_and_poles() {
    let mut c = RegionalCity::new(2);
    let mut h = Services::default();
    c.pending.push(change(9, -1, 4, 0));
    c.create_symmetric_connections(&mut h);
    assert!(h.existing.contains(&(9, -1, 4)));
    assert_eq!(h.events, ["create", "ready"]);
    h.events.clear();
    c.pending.push(change(2, -1, 4, 1));
    c.create_symmetric_connections(&mut h);
    assert!(!h.existing.contains(&(2, -1, 4)));
    assert_eq!(
        h.events,
        [
            "demolish:1:[(0, 4), (1, 4), (2, 4)]",
            "create",
            "place",
            "delete",
            "place",
            "ready"
        ]
    );
    h.events.clear();
    h.no_network = true;
    c.pending.push(change(2, -1, 4, 1));
    c.create_symmetric_connections(&mut h);
    assert_eq!(h.events, ["ready"]);
    h.no_network = false;
    h.events.clear();
    c.pending = vec![
        change(3, -1, 10, 1),
        change(3, -1, 11, 1),
        change(3, 10, -1, 1),
        change(3, 11, -1, 1),
    ];
    c.create_symmetric_connections(&mut h);
    assert_eq!(h.events.iter().filter(|s| s.starts_with("demolish")).count(), 2);
    assert!(h.existing.contains(&(3, -1, 10)));
    assert!(h.existing.contains(&(3, 10, -1)));
    h.events.clear();
    h.occupied.insert((1, 7));
    h.pole = true;
    c.pending.push(change(4, -1, 7, 1));
    c.create_symmetric_connections(&mut h);
    assert_eq!(h.events[0], "demolish:1:[(0, 7), (2, 7)]");
    assert_eq!(h.events[1], "dezone");
    assert!(h.existing.contains(&(4, -1, 7)));
}

#[test]
fn edge_bytes_and_remove_cleanup() {
    let mut c = RegionalCity::new(2);
    let mut h = Services::default();
    h.cells.insert((0, 0, 5), vec![1]);
    h.words.insert(1, 0x0400);
    h.cells.insert((0, 0, 4), vec![2]);
    h.words.insert(2, 0x12345678);
    c.pending.push(change(2, -1, 5, 1));
    c.create_symmetric_connections(&mut h);
    assert_eq!(h.words[&1], 0x0402);
    assert_eq!(h.words[&2], 0x12340278);
    h.events.clear();
    c.pending.push(change(2, -1, 5, 0));
    c.create_symmetric_connections(&mut h);
    assert_eq!(h.words[&1], 0x0400);
    assert_eq!(h.words[&2], 0x12340078);
    assert_eq!(h.events, ["delete", "place", "ready"]);
    h.events.clear();
    c.create_symmetric_connections(&mut h);
    assert_eq!(h.events, ["ready"]);
}

#[test]
fn message_handlers_cache_preservation_and_reconciliation_cases() {
    let mut c = RegionalCity::new(2);
    let mut h = Services::default();
    c.budget = 123.0;
    c.income = 4.0;
    c.birth_date = 8;
    c.world = [9.0; 3];
    c.original_locale = (2, 3);
    c.unknown_strings[0] = b"opaque".to_vec();
    c.potential.insert(1, 55);
    c.populations.insert(9, 99);
    let current = MessageContext {
        application_object: true,
        city_exists: true,
        established: true,
        ..MessageContext::default()
    };
    c.do_message(0x26D31EC1, MessageContext::default(), &mut h);
    assert!(h.events.is_empty());
    c.do_message(0x26D31EC1, current, &mut h);
    assert_eq!(&h.events[..4], ["mayor", "city", "difficulty", "path"]);
    assert_eq!(c.commercial_jobs, 100);
    assert_eq!(c.industrial_jobs, 50);
    assert_eq!(c.population, 500);
    assert!(!c.populations.contains_key(&9));
    assert_eq!(c.points[&0], [[1.0, 2.0, 3.0]]);
    assert_eq!((c.budget, c.income, c.birth_date), (123.0, 4.0, 8));
    assert_eq!(c.potential[&1], 55);
    assert_eq!(c.world, [9.0; 3]);
    assert_eq!(c.original_locale, (2, 3));
    assert_eq!(c.unknown_strings[0], b"opaque");
    assert_eq!(c.get_produced(1), 75.0);
    assert_eq!(c.tutorial_id, 7);
    c.do_message(0x26C63343, current, &mut h);
    assert_eq!(c.potential[&0x3111], 1000);
    assert_eq!(c.extrapolated[&0x3111], 100);
    for (established, forced, moved, expect_old) in [
        (true, false, false, true),
        (false, false, false, false),
        (true, true, false, false),
        (true, false, true, false),
    ] {
        c.moved = moved;
        c.pending = vec![change(9, -1, 5, 1)];
        h.events.clear();
        h.existing.clear();
        h.no_network = true;
        h.incoming = vec![IncomingCity {
            position: (0, 1),
            size: (1, 1),
            connections: vec![NeighborConnection::with(9, 7, -1)],
        }];
        let msg = MessageContext {
            established,
            forced,
            ..current
        };
        c.do_message(0xCA63D517, msg, &mut h);
        assert_eq!(h.existing.contains(&(9, -1, 5)), expect_old);
        assert_eq!(h.existing.contains(&(9, 7, 64)), !expect_old);
        assert!(!c.moved);
        assert!(c.pending.is_empty());
        assert_eq!(h.events.last().unwrap(), "ready");
    }
    let d1 = NeighborDeal::new();
    let d2 = NeighborDeal::new();
    c.local_deals = vec![d1.clone(), d2.clone()];
    c.do_message(0x26D31EC3, MessageContext::default(), &mut h);
    assert_eq!(c.local_deals.len(), 2);
    c.do_message(0x26C63342, MessageContext::default(), &mut h);
    assert!(Rc::ptr_eq(&c.other_deals[0], &d2));
    c.local_deals.push(d1);
    c.do_message(
        0x26D31EC3,
        MessageContext {
            names_current_city: true,
            ..current
        },
        &mut h,
    );
    assert!(c.local_deals.is_empty());
    assert!(c.do_message(0xFFFFFFFF, current, &mut h));
}

#[test]
fn preferences_and_setup_city_order() {
    #[derive(Default)]
    struct Setup(Vec<String>);
    impl CitySetup for Setup {
        fn set_mayor_name(&mut self, n: &[u8]) {
            self.0.push(format!("mayor:{n:?}"));
        }
        fn set_city_name(&mut self, n: &[u8]) {
            self.0.push(format!("city:{n:?}"));
        }
        fn set_difficulty(&mut self, d: u8) {
            self.0.push(format!("difficulty:{d}"));
        }
        fn create_save_path(&mut self, _: &[u8]) -> Vec<u8> {
            self.0.push("create_path".into());
            b"generated.sc4".to_vec()
        }
        fn set_file_path(&mut self, p: &[u8]) {
            assert_eq!(p, b"generated.sc4");
            self.0.push("path".into());
        }
        fn set_size(&mut self, s: (i32, i32)) {
            assert_eq!(s, (2048, 4096));
            self.0.push("size".into());
        }
    }
    let mut c = RegionalCity::new(2);
    c.file_path = b"C:\\Region\\old.sc4".to_vec();
    c.update_preferences(&Preferences {
        city_name: b"New\0ignored".to_vec(),
        mayor_name: b"M".to_vec(),
        difficulty: 1,
    });
    assert_eq!(c.file_path, b"C:\\Region\\New.sc4");
    assert_eq!(c.setup_preferences().city_name, b"New");
    c.size = (2, 4);
    let mut h = Setup::default();
    c.setup_city(&mut h);
    assert_eq!(
        h.0,
        [
            "mayor:[77]",
            "city:[78, 101, 119]",
            "difficulty:1",
            "create_path",
            "path",
            "size"
        ]
    );
}

#[test]
fn non_serializable_element_counts_are_not_adjusted() {
    struct Missing;
    impl RecordElements for Missing {
        fn connection(&mut self, _: &NeighborConnection, _: &mut dyn OutStream) -> Option<bool> {
            None
        }
        fn deal(&mut self, _: &NeighborDeal, _: &mut dyn OutStream) -> Option<bool> {
            None
        }
    }
    let mut c = RegionalCity::new(2);
    let baseline = bytes(&c);
    c.connections.push(NeighborConnection::new());
    c.local_deals.push(NeighborDeal::new());
    let mut out = ByteWriter::new();
    assert!(c.write_record_with(&mut out, &mut Missing));
    assert_eq!(out.bytes().len(), baseline.len());
    assert_eq!(out.bytes().iter().zip(&baseline).filter(|(a, b)| a != b).count(), 2);
    assert!(RegionalCity::decode_record(&mut ByteReader::new(out.bytes()), 2).is_err());
}
