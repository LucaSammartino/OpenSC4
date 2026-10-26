// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

// Locate the first map using the specified header and seven length-prefixed strings.
// These fixtures have empty scalar maps, connections and pending queues, and twelve taxes.
fn commodity_offset(b: &[u8]) -> usize {
    let mut inp = ByteReader::new(b);
    assert!(inp.skip(2 + 16 + 12 + 4 + 3 + 4 + 16 + 4 + 1));
    for _ in 0..7 {
        inp.get_str().unwrap();
    }
    assert!(inp.skip(16 + 12 + 52 + 8 + 16));
    inp.position()
}

#[test]
fn ry7_absent_keys_duplicate_commodities_and_pois() {
    let mut source = RegionalCity::new(2);
    source.set_produced(1, 10.0);
    source.set_produced(2, 20.0);
    source.commodities.remove(&3);
    source.points.insert(7, vec![[1.0, 2.0, 3.0]]);
    source.points.insert(8, vec![[4.0, 5.0, 6.0]]);
    let mut b = bytes(&source);
    let offset = commodity_offset(&b);
    let decoded = RegionalCity::decode_record(&mut ByteReader::new(&b), 2).unwrap();
    assert_eq!(decoded.commodities.keys().copied().collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(decoded.commodities[&3], Commodity::default());
    assert_eq!(&bytes(&decoded)[offset..offset + 4], &3_u32.to_le_bytes());

    // The second commodity and second POI carry the first entry's key on the wire.
    b[offset + 48..offset + 52].copy_from_slice(&1_i32.to_le_bytes());
    b[offset + 52..offset + 60].copy_from_slice(&25_f64.to_le_bytes());
    let second_poi = offset + 4 + 2 * 44 + 8 + 24;
    b[second_poi..second_poi + 4].copy_from_slice(&7_u32.to_le_bytes());
    let mut receiver = RegionalCity::new(2);
    receiver.commodities.entry(9).or_default().produced = 99.0;
    receiver.populations.insert(42, 123);
    receiver.points.insert(7, vec![[0.0; 3]]);
    receiver
        .decode_record_into_with(&mut ByteReader::new(&b), &mut CoreElements)
        .unwrap();
    assert_eq!(receiver.commodities[&1].produced, 25.0);
    assert_eq!(receiver.commodities[&2].produced, 0.0);
    assert_eq!(receiver.commodities[&9].produced, 99.0);
    assert_eq!(receiver.populations[&42], 123);
    assert_eq!(receiver.points[&7], [[0.0; 3], [1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
}

#[test]
fn ry7_partial_receiver_failure_and_atomic_wrapper_publication() {
    let mut source = RegionalCity::new(2);
    source.position = (2, 3);
    let mut b = bytes(&source);
    let offset = commodity_offset(&b);
    b[offset..offset + 4].copy_from_slice(&0x0010_0001_u32.to_le_bytes());
    let mut receiver = RegionalCity::new(2);
    receiver.position = (8, 9);
    assert_eq!(
        receiver.read_record_into_with(
            &mut ByteReader::new(&b),
            &context(),
            &mut Life::default(),
            &mut CoreElements
        ),
        Err(RecordError::Limit)
    );
    assert_eq!(receiver.position, (2, 3));
    let mut old = RegionalCity::new(2);
    old.serial = 777;
    let mut out = Some(old);
    let mut p = Pack::default();
    let mut prefixed = vec![1, 0];
    prefixed.extend(b);
    p.records.insert(RECORD_KEY, prefixed);
    assert!(!read_package(&mut p, &mut out, &context(), &mut Life::default()));
    assert_eq!(out.unwrap().serial, 777);
}

#[test]
fn ry7_negative_deal_count_attempts_first_element() {
    struct StopAtFirstDeal(usize);
    impl RecordReader<Rc<NeighborConnection>, Rc<NeighborDeal>> for StopAtFirstDeal {
        fn connection(&mut self, _: &mut dyn InStream) -> Option<Rc<NeighborConnection>> {
            None
        }
        fn deal(&mut self, _: &mut dyn InStream) -> Option<Rc<NeighborDeal>> {
            self.0 += 1;
            None
        }
    }
    let mut b = bytes(&RegionalCity::new(2));
    let first_deal = commodity_offset(&b) + 4 + 3 * 44;
    b.truncate(first_deal + 4);
    b[first_deal..].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut reader = StopAtFirstDeal(0);
    assert_eq!(
        RegionalCity::decode_record_with(&mut ByteReader::new(&b), 2, &mut reader).unwrap_err(),
        RecordError::Element
    );
    assert_eq!(reader.0, 1);
}

#[test]
fn rr9_cleanup_preserves_result_and_unrelated_record() {
    let mut p = Pack {
        finish: true,
        close_result: false,
        ..Pack::default()
    };
    let unrelated = ResKey {
        type_id: 1,
        group: 2,
        instance: 3,
    };
    p.records.insert(VERSION_KEY, 65_u32.to_le_bytes().to_vec());
    p.records.insert(unrelated, b"untouched".to_vec());
    assert!(write_path(&mut p, b"city", &RegionalCity::new(2)));
    assert_eq!(p.records[&unrelated], b"untouched");
    assert_eq!(&p.records[&RECORD_KEY][..2], &[1, 0]);
    assert_eq!(p.cleanup, ["is_open", "close", "shutdown", "release"]);
    p.finish = false;
    p.cleanup.clear();
    let mut out = None;
    assert!(read_path(&mut p, b"city", &mut out, &context(), &mut Life::default()));
    assert_eq!(p.cleanup, ["is_open"]);
    p.finish = true;
    p.cleanup.clear();
    p.records.insert(RECORD_KEY, vec![1, 0]);
    assert!(!read_path(&mut p, b"city", &mut out, &context(), &mut Life::default()));
    assert_eq!(p.cleanup, ["is_open", "close", "shutdown", "release"]);
}

#[test]
fn rr3_last_group_mask_and_city_border_byte() {
    for last_word in [0, 0x0000_0500] {
        let mut c = RegionalCity::new(2);
        let mut h = Services::default();
        h.cells.insert((0, 0, 5), vec![1, 2]);
        h.words.insert(1, 0x0400);
        h.words.insert(2, last_word);
        h.cells.insert((0, 0, 4), vec![3]);
        h.words.insert(3, 0x1234_5678);
        c.pending.push(change(2, -1, 5, 1));
        c.create_symmetric_connections(&mut h);
        assert_eq!(h.words[&1], 0x0402);
        assert_eq!(h.words[&2], last_word | 2);
        assert_eq!(h.words[&3], if last_word == 0 { 0x1234_5678 } else { 0x1234_5602 });
    }
}

#[test]
fn rr4_rr5_west_highway_orientation_singleton_and_pair() {
    for count in [1, 2] {
        let mut c = RegionalCity::new(2);
        c.size = (1, 1);
        c.pending = (10..10 + count).map(|z| change(3, -1, z, 1)).collect();
        for property in [true, false] {
            let prepared = c.prepare_changes(property);
            assert_eq!(prepared[0].direction, !property);
            assert!(!prepared[0].pair);
            if count == 2 {
                assert_eq!(prepared[1].direction, property);
                assert!(prepared[1].pair);
            }
        }
        let mut h = Services::default();
        c.create_symmetric_connections(&mut h);
        assert!(c.pending.is_empty());
        assert_eq!(h.events.iter().filter(|e| *e == "create").count(), count as usize);
        assert_eq!(
            h.events.iter().filter(|e| *e == "place").count(),
            if count == 1 { 0 } else { 2 }
        );
        if count == 1 {
            assert_eq!(h.events, ["create", "ready"]);
        } else {
            assert!(h.events.contains(&"demolish:1:[(0, 11), (1, 11), (2, 11)]".into()));
        }
    }
    assert_eq!(connections::network_type(2), Some(0));
    assert_eq!(connections::network_type(3), Some(2));
}

#[test]
fn ry2_rr7_rr8_metadata_preferences_and_opaque_string_preservation() {
    let mut source = RegionalCity::new(2);
    source.established = true;
    source.city_name = b"City".to_vec();
    source.mayor_name = b"Mayor".to_vec();
    source.description = b"Description".to_vec();
    source.unknown_strings = [b"alpha".to_vec(), b"beta".to_vec(), b"gamma".to_vec()];
    source.original_locale = (1, 2);
    source.last_locale = (3, 4);
    source.birth_date = 123;
    source.world = [12.5, -3.0, 7.0];
    let original = bytes(&source);
    let mut c = RegionalCity::read_record(&mut ByteReader::new(&original), &context(), &mut Life::default()).unwrap();
    assert_eq!(bytes(&c), original);
    c.set_city_name(b"Renamed");
    c.set_mayor_name(b"Renamed Mayor");
    assert_eq!(c.unknown_strings, source.unknown_strings);
    c.file_path = b"C:/Regions/R/City - Old.sc4".to_vec();
    let prefs = Preferences {
        city_name: b"New".to_vec(),
        mayor_name: b"Mayor".to_vec(),
        difficulty: 1,
    };
    c.update_preferences(&prefs);
    let mut path = Vec::new();
    c.get_file_path(&mut path);
    assert_eq!(path, b"C:/Regions/R/New.sc4");
    let back = c.setup_preferences();
    assert_eq!(
        (back.city_name, back.mayor_name, back.difficulty),
        (prefs.city_name, prefs.mayor_name, 1)
    );
    c.update_city_cache(&mut Services::default());
    assert_eq!(c.file_path, b"C:/Town.sc4");
    assert_eq!(c.birth_date, 123);
    assert_eq!(c.world, [12.5, -3.0, 7.0]);
    assert_eq!(c.original_locale, (1, 2));
    assert_eq!(c.last_locale, (3, 4));
    assert_eq!(c.unknown_strings, source.unknown_strings);
    c.set_birth_date(456);
    let written = RegionalCity::decode_record(&mut ByteReader::new(&bytes(&c)), 2).unwrap();
    assert_eq!(written.birth_date, 456);
    assert_eq!(written.world, source.world);
    assert_eq!(written.original_locale, source.original_locale);
    assert_eq!(written.last_locale, source.last_locale);
    assert_eq!(written.unknown_strings, source.unknown_strings);
}

#[test]
fn rr6_workforce_getter_returns_address_of_stored_float() {
    let mut c = RegionalCity::new(2);
    c.workforce = f32::from_bits(0x3F00_0000);
    assert!(std::ptr::eq(c.get_workforce_percentage(), &c.workforce));
    assert_eq!(c.get_workforce_percentage().to_bits(), 0x3F00_0000);
    assert_eq!(c.workforce.to_bits(), 0x3F00_0000);
}
