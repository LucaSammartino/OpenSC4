//! Behaviour tests for the lot life cycle and business units (lot-records
//! §3.3, §3.5, §4, §17.3 T13 / T14; lot-developer.md §5.7 Add / Remove).

use std::rc::Rc;

use super::lifecycle::*;
use super::lot::*;
use super::tests::{hex, read_obj, write_obj, T1};
use super::*;
use crate::stream::{ByteReader, ByteWriter, RefValue};
use crate::traffic_simulator::{read_trip, write_trip, TripData};

#[derive(Default)]
struct Host {
    draws: Vec<u32>,
    next_draw: u32,
    configs: Vec<u32>,
    lookups: Vec<u32>,
    locked: Vec<(u8, u8, f32)>,
    unlocked: Vec<(u8, u8)>,
    altitude_calls: usize,
    /// (x, z) of each terrain altitude query.
    altitude_at: Vec<(f32, f32)>,
    /// Terrain answer; `None` = 300.5.
    terrain: Option<f64>,
    /// Slope answer; `None` = 1.25.
    slope_answer: Option<f64>,
    slope_calls: Vec<[u8; 4]>,
    no_view: bool,
}

impl LotHost for Host {
    fn terrain_altitude(&mut self, x: f32, z: f32) -> f64 {
        self.altitude_calls += 1;
        self.altitude_at.push((x, z));
        self.terrain.unwrap_or(300.5)
    }
    fn lot_slope(&mut self, rect: [u8; 4]) -> f64 {
        self.slope_calls.push(rect);
        self.slope_answer.unwrap_or(1.25)
    }
    fn has_terrain_view(&mut self) -> bool {
        !self.no_view
    }
    fn random_uint32_uniform(&mut self, n: u32) -> u32 {
        self.draws.push(n);
        self.next_draw
    }
    fn has_lot_configuration(&mut self, id: u32) -> bool {
        self.lookups.push(id);
        self.configs.contains(&id)
    }
    fn lock_cell(&mut self, x: u8, z: u8, altitude: f32) {
        self.locked.push((x, z, altitude));
    }
    fn unlock_cell(&mut self, x: u8, z: u8) {
        self.unlocked.push((x, z));
    }
}

fn t1_lot() -> Rc<Lot> {
    let (o, ok, _) = read_obj(CLSID_LOT, &hex(T1));
    assert!(ok);
    o.object::<Lot>().unwrap()
}

#[test]
fn t13_init_after_t1_draws_nothing_and_keeps_altitude() {
    let lot = t1_lot();
    let alt = lot.rec.borrow().altitude;
    let mut h = Host::default();
    assert!(lot_init(&lot, &mut h));
    assert!(h.draws.is_empty());
    assert_eq!(h.altitude_calls, 0);
    assert!(h.lookups.is_empty(), "configuration 0 is not looked up");
    assert_eq!(lot.configuration.get(), None);
    let r = lot.rec.borrow();
    assert_eq!(r.altitude, alt);
    assert_eq!(r.strategy, 74);
    drop(r);
    // Init changes nothing that is saved.
    assert_eq!(write_obj(&RefValue::Object(lot.clone())), hex(T1));
}

#[test]
fn t13_strategy_ff_draws_exactly_once() {
    let lot = t1_lot();
    lot.rec.borrow_mut().strategy = STRATEGY_UNSET;
    let mut h = Host {
        next_draw: 42,
        ..Host::default()
    };
    lot_init(&lot, &mut h);
    assert_eq!(h.draws, vec![100]);
    assert_eq!(lot.rec.borrow().strategy, 42);
    lot_init(&lot, &mut h);
    assert_eq!(h.draws.len(), 1);
}

#[test]
fn t9_rejected_lot_skips_the_altitude_step() {
    let mut b = hex(T1);
    b[0] = 9;
    let (o, ok, _) = read_obj(CLSID_LOT, &b);
    assert!(!ok);
    let lot = o.object::<Lot>().unwrap();
    let mut h = Host::default();
    lot_init(&lot, &mut h);
    assert_eq!(h.altitude_calls, 0);
    assert_eq!(lot.rec.borrow().altitude, 0.0);
    // Its strategy is 0 (not the constructor value), so no draw either.
    assert!(h.draws.is_empty());
}

#[test]
fn init_of_a_new_lot_sets_altitude_slopes_jobs_and_draws() {
    let lot = Lot::constructed();
    lot.rec.borrow_mut().travel_jobs = [1.0, 2.0, 3.0];
    let mut h = Host {
        next_draw: 7,
        ..Host::default()
    };
    lot_init(&lot, &mut h);
    assert!(lot.initialised.get());
    let r = lot.rec.borrow();
    assert_eq!((r.altitude, r.slope_at_altitude, r.slope), (300.5, 1.25, 1.25));
    assert_eq!(r.travel_jobs, [0.0; 3]);
    assert_eq!(r.strategy, 7);
    assert_eq!(h.draws, vec![100]);
}

#[test]
fn init_resolves_configuration_and_write_saves_its_id() {
    let lot = t1_lot();
    lot.rec.borrow_mut().configuration_id = 0x1234_5678;
    let mut h = Host {
        configs: vec![0x1234_5678],
        ..Host::default()
    };
    lot_init(&lot, &mut h);
    assert_eq!(h.lookups, vec![0x1234_5678]);
    assert_eq!(lot.configuration.get(), Some(0x1234_5678));
    assert_eq!(lot.rec.borrow().configuration_id, 0);
    let out = write_obj(&RefValue::Object(lot.clone()));
    assert_eq!(&out[2..6], &0x1234_5678u32.to_le_bytes());
    assert_eq!(&out[6..], &hex(T1)[6..]);
}

#[test]
fn init_drops_a_missing_configuration_and_saves_zero() {
    let lot = t1_lot();
    lot.rec.borrow_mut().configuration_id = 0x0BAD_0001;
    let mut h = Host::default();
    lot_init(&lot, &mut h);
    assert_eq!(h.lookups, vec![0x0BAD_0001]);
    assert_eq!(lot.configuration.get(), None);
    assert_eq!(write_obj(&RefValue::Object(lot.clone())), hex(T1));
}

#[test]
fn t20_read_without_init_writes_zero_configuration() {
    let mut b = hex(T1);
    b[2..6].copy_from_slice(&0x1234_5678u32.to_le_bytes());
    let (o, ok, _) = read_obj(CLSID_LOT, &b);
    assert!(ok);
    let out = write_obj(&o);
    assert_eq!(&out[2..6], &[0, 0, 0, 0], "LR4: the ID read is never written");
    assert_eq!(&out[6..], &b[6..]);
    // After Init with the manager answering that ID: 78 56 34 12.
    let lot = o.object::<Lot>().unwrap();
    let mut h = Host {
        configs: vec![0x1234_5678],
        ..Host::default()
    };
    lot_init(&lot, &mut h);
    assert_eq!(&write_obj(&o)[2..6], &[0x78, 0x56, 0x34, 0x12]);
    // Answering null: 0.
    let (o, _, _) = read_obj(CLSID_LOT, &b);
    lot_init(&o.object::<Lot>().unwrap(), &mut Host::default());
    assert_eq!(&write_obj(&o)[2..6], &[0, 0, 0, 0]);
}

#[test]
fn t17_init_centre_altitude_and_both_slopes() {
    let lot = Lot::constructed();
    lot.rec.borrow_mut().rect = [10, 20, 12, 21];
    lot.rec.borrow_mut().travel_jobs = [1.0, 2.0, 3.0];
    let mut h = Host {
        terrain: Some(300.0),
        slope_answer: Some(2.5),
        ..Host::default()
    };
    lot_init(&lot, &mut h);
    assert_eq!(h.altitude_at, vec![(184.0, 336.0)]);
    assert_eq!(h.slope_calls, vec![[10, 20, 12, 21]]);
    let r = lot.rec.borrow();
    assert_eq!((r.altitude, r.slope_at_altitude, r.slope), (300.0, 2.5, 2.5));
    assert_eq!(r.travel_jobs, [0.0; 3]);
    drop(r);
    // Within 0.001 of the constructor altitude 0: nothing changes, no slope query.
    let lot = Lot::constructed();
    lot.rec.borrow_mut().rect = [10, 20, 12, 21];
    let mut h = Host {
        terrain: Some(0.0005),
        ..Host::default()
    };
    lot_init(&lot, &mut h);
    assert!(h.slope_calls.is_empty());
    let r = lot.rec.borrow();
    assert_eq!((r.altitude, r.slope_at_altitude, r.slope), (0.0, 0.0, 0.0));
}

#[test]
fn set_altitude_threshold_nan_and_update_slope() {
    let mut r = LotRecord {
        altitude: 10.0,
        ..LotRecord::default()
    };
    let mut h = Host::default();
    // The f32 nearest 10.001 sits on one side of the double threshold.
    lot_set_altitude(&mut r, 10.001, &mut h);
    let moved = r.altitude != 10.0;
    assert_eq!(moved, (f64::from(10.001f32) - 10.0).abs() > 0.001);
    assert_eq!(h.slope_calls.len(), usize::from(moved));
    let mut h = Host::default();
    let mut r = LotRecord {
        altitude: 10.0,
        ..LotRecord::default()
    };
    lot_set_altitude(&mut r, f32::NAN, &mut h);
    assert_eq!(r.altitude, 10.0);
    lot_set_altitude(&mut r, 10.0, &mut h);
    assert!(h.slope_calls.is_empty());
    r.slope_at_altitude = 9.0;
    lot_update_slope(&mut r, &mut h);
    assert_eq!((r.slope_at_altitude, r.slope), (9.0, 1.25));
}

#[test]
fn t18_set_flags_cell_order_and_no_view() {
    let lot = Lot::constructed();
    {
        let mut r = lot.rec.borrow_mut();
        r.rect = [10, 20, 11, 21];
        r.altitude = 310.5;
    }
    let mut h = Host::default();
    lot_set_flags(&lot, 0x01, &mut h);
    let cells = [(10, 20), (10, 21), (11, 20), (11, 21)];
    assert_eq!(h.locked, cells.map(|(x, z)| (x, z, 310.5)).to_vec());
    lot_set_flags(&lot, 0x00, &mut h);
    assert_eq!(h.unlocked, cells.to_vec());
    lot_set_flags(&lot, 0x01, &mut h);
    let n = (h.locked.len(), h.unlocked.len());
    lot_set_flags(&lot, 0x03, &mut h);
    assert_eq!((h.locked.len(), h.unlocked.len()), n);
    // No terrain view: flags stored, no cell touched.
    let mut h = Host {
        no_view: true,
        ..Host::default()
    };
    lot_set_flags(&lot, 0x00, &mut h);
    assert_eq!(lot.rec.borrow().flags, 0);
    assert!(h.unlocked.is_empty() && h.locked.is_empty());
}

#[test]
fn t19_remove_non_member_still_clears_and_add_is_idempotent() {
    let unit = Rc::new(BusinessUnit::default());
    let (a, b, c) = (Lot::constructed(), Lot::constructed(), Lot::constructed());
    business_unit_add(&unit, &a);
    business_unit_add(&unit, &b);
    let other = Rc::new(BusinessUnit::default());
    c.rec.borrow_mut().business_unit = RefValue::Object(other.clone());
    c.rec.borrow_mut().travel_designate = RefValue::Object(a.clone());
    business_unit_remove(&unit, &c);
    assert!(c.rec.borrow().business_unit.is_null());
    assert!(c.rec.borrow().travel_designate.is_null());
    assert_eq!(Rc::strong_count(&other), 1, "C's real unit is released");
    let ids = |u: &BusinessUnit| -> Vec<*const Lot> {
        business_unit_members(u)
            .iter()
            .map(|m| Rc::as_ptr(&m.object::<Lot>().unwrap()))
            .collect()
    };
    assert_eq!(ids(&unit), vec![Rc::as_ptr(&a), Rc::as_ptr(&b)]);
    a.rec.borrow_mut().business_unit = RefValue::Null;
    business_unit_add(&unit, &a);
    assert_eq!(ids(&unit), vec![Rc::as_ptr(&a), Rc::as_ptr(&b)]);
    assert!(a.rec.borrow().business_unit.object::<BusinessUnit>().is_some());
    business_unit_add(&unit, &c);
    assert_eq!(ids(&unit), vec![Rc::as_ptr(&a), Rc::as_ptr(&b), Rc::as_ptr(&c)]);
    // Remove erases every equal node.
    unit.rec.borrow_mut().lots.push(RefValue::Object(a.clone()));
    business_unit_remove(&unit, &a);
    assert_eq!(ids(&unit), vec![Rc::as_ptr(&b), Rc::as_ptr(&c)]);
}

#[test]
fn t14_post_city_init_relocks_terrain_only_with_bit_0() {
    let lot = t1_lot();
    {
        let mut r = lot.rec.borrow_mut();
        r.flags = 0x5B;
        r.rect = [10, 20, 11, 22];
    }
    let mut h = Host::default();
    lot_post_city_init(&lot, &mut h);
    assert_eq!(lot.rec.borrow().flags, 0x5B);
    let alt = lot.rec.borrow().altitude;
    let mut cells: Vec<_> = h
        .locked
        .iter()
        .map(|&(x, z, a)| {
            assert_eq!(a, alt);
            (x, z)
        })
        .collect();
    cells.sort();
    assert_eq!(cells, vec![(10, 20), (10, 21), (10, 22), (11, 20), (11, 21), (11, 22)]);
    assert!(h.unlocked.is_empty());

    let mut h = Host::default();
    lot.rec.borrow_mut().flags = 0x50;
    lot_post_city_init(&lot, &mut h);
    assert_eq!(lot.rec.borrow().flags, 0x50);
    assert!(h.locked.is_empty() && h.unlocked.is_empty());
}

#[test]
fn set_flags_unlocks_when_bit_0_clears_and_ignores_other_bits() {
    let lot = t1_lot();
    lot.rec.borrow_mut().flags = 0x11;
    let mut h = Host::default();
    lot_set_flags(&lot, 0x13, &mut h);
    assert!(h.locked.is_empty() && h.unlocked.is_empty());
    lot_set_flags(&lot, 0x12, &mut h);
    assert_eq!(h.unlocked, vec![(73, 100)]);
    assert_eq!(lot.rec.borrow().flags, 0x12);
}

#[test]
fn business_unit_add_is_idempotent_and_remove_clears_unit_and_designate() {
    let unit = Rc::new(BusinessUnit::default());
    let a = t1_lot();
    let b = t1_lot();
    business_unit_add(&unit, &a);
    business_unit_add(&unit, &b);
    business_unit_add(&unit, &a);
    assert_eq!(business_unit_members(&unit).len(), 2);
    assert!(a.rec.borrow().business_unit.object::<BusinessUnit>().is_some());
    b.rec.borrow_mut().travel_designate = RefValue::Object(a.clone());
    business_unit_remove(&unit, &b);
    assert!(b.rec.borrow().business_unit.is_null());
    assert!(b.rec.borrow().travel_designate.is_null());
    let m = business_unit_members(&unit);
    assert_eq!(m.len(), 1);
    assert!(m[0].object::<Lot>().is_some_and(|l| Rc::ptr_eq(&l, &a)));
    business_unit_remove(&unit, &b);
    assert_eq!(business_unit_members(&unit).len(), 1);
    // Remove then re-add appends at the end (lot-developer.md §5.7).
    business_unit_add(&unit, &b);
    business_unit_remove(&unit, &a);
    business_unit_add(&unit, &a);
    let m = business_unit_members(&unit);
    assert!(m[1].object::<Lot>().is_some_and(|l| Rc::ptr_eq(&l, &a)));
}

#[test]
fn business_unit_written_after_add_names_its_lots() {
    let unit = Rc::new(BusinessUnit::default());
    let a = t1_lot();
    business_unit_add(&unit, &a);
    let out = write_obj(&RefValue::Object(unit.clone()));
    assert_eq!(out.len(), 2 + 4 + 8);
    assert_eq!(&out[..6], &[1, 0, 1, 0, 0, 0]);
    assert_eq!(&out[10..14], &CLSID_LOT.to_le_bytes());
}

#[test]
fn shutdown_clears_runtime_state_and_leaves_the_unit() {
    let unit = Rc::new(BusinessUnit::default());
    let a = t1_lot();
    business_unit_add(&unit, &a);
    {
        let mut r = a.rec.borrow_mut();
        r.populations = vec![PopulationMap::from([(0x1010, 3)])];
        r.travel_populations.insert(0x1010, 2);
        r.travel_designate = RefValue::Object(a.clone());
    }
    a.configuration.set(Some(5));
    assert!(lot_shutdown(&a));
    assert!(!a.initialised.get());
    let r = a.rec.borrow();
    assert!(r.populations.is_empty() && r.travel_populations.is_empty());
    assert!(r.travel_designate.is_null() && r.business_unit.is_null());
    assert_eq!(a.configuration.get(), None);
    assert!(business_unit_members(&unit).is_empty());
    drop(r);
    // A second Shutdown does nothing.
    business_unit_add(&unit, &a);
    assert!(lot_shutdown(&a));
    assert_eq!(business_unit_members(&unit).len(), 1);
}

/// §3.3: uninitialised trip fields (`0xCCCC` destinations, `0xCCCCCCCC`
/// travel types) are written back byte for byte, version 2 always.
#[test]
fn trip_with_uninitialised_fields_round_trips() {
    let mut b = vec![2, 0, 0, 0, 3, 0, 0, 0, 1, 2, 3, 0, 0, 0, 0];
    b.extend(5u32.to_le_bytes());
    b.extend([3, 2, 11, 5, 0xCC, 0xCC, 0xCC, 0xCC]);
    b.extend(12.74f32.to_le_bytes());
    b.extend([0xCC; 4]);
    let mut t = TripData::default();
    let mut rd = ByteReader::new(&b);
    assert!(read_trip(&mut rd, &mut t));
    assert_eq!(rd.remaining(), 0);
    assert_eq!((t.dest_x, t.dest_z), (-13_108, -13_108));
    assert_eq!(t.travel_types_used, 0xCCCC_CCCC);
    assert_eq!((t.trip_type, t.strategy, t.flags, t.status), (3, 2, 11, 5));
    let mut w = ByteWriter::new();
    assert!(write_trip(&mut w, &t));
    assert_eq!(w.into_bytes(), b);
}

#[test]
fn lot_property_ids_match_section_3_6() {
    let lot = t1_lot();
    assert_eq!(lot.rec.borrow().properties.ids(), vec![PROP_TRIP_STATISTICS]);
    assert_eq!(PROP_LOCAL_FUNDING_LAST - PROP_LOCAL_FUNDING_FIRST, 3);
}
