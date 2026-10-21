//! Behaviour tests for retaining walls (lot-records §7.1, §7.2, §17.3
//! T7, T21–T25).

use std::collections::HashMap;
use std::rc::Rc;

use super::lot::{lot_flag, Lot};
use super::tests::{read_obj, write_obj};
use super::walls::*;
use super::*;
use crate::stream::RefValue;

/// A fake city. Edge i has a neighbour row when `edge_alt[i]` is `Some`;
/// that row is one cell (0, 0) with `cell_lot`, and its vertex rect is the
/// one vertex (1, 1) at `edge_alt[i]`, so the edge hash is `bits + round(y)
/// + 1`. `geometry` replaces this with an explicit setup.
#[derive(Default)]
struct Host {
    edge_alt: [Option<f64>; 4],
    cell_lot: Option<NeighbourLot>,
    cur_edge: usize,
    geometry: Option<Geometry>,
    wall_type: u32,
    props_ok: bool,
    lookup: Option<WallLookup>,
    /// Per edge: `None` = add returns early, `Some(ok)`.
    build: [Option<bool>; 4],
    surrounding: Vec<Rc<Lot>>,
    log: Vec<String>,
    inserted: Vec<Wall>,
}

struct Geometry {
    cells: HashMap<u32, [i32; 4]>,
    verts: HashMap<u32, [i32; 4]>,
    lots: HashMap<(i32, i32), NeighbourLot>,
    alt: HashMap<(i32, i32), f64>,
}

impl WallHost for Host {
    fn neighbour_cells_at_edge(&mut self, rect: [i32; 4], edge_flag: u32) -> Option<[i32; 4]> {
        self.log.push(format!("cells {rect:?} {edge_flag}"));
        if let Some(g) = &self.geometry {
            return g.cells.get(&edge_flag).copied();
        }
        let i = edge_flag.trailing_zeros() as usize;
        self.edge_alt[i].map(|_| [0, 0, 0, 0])
    }
    fn vertices_at_edge(&mut self, _rect: [i32; 4], edge_flag: u32) -> [i32; 4] {
        self.cur_edge = edge_flag.trailing_zeros() as usize;
        if let Some(g) = &self.geometry {
            return g.verts[&edge_flag];
        }
        [1, 1, 1, 1]
    }
    fn lot_at(&mut self, x: i32, z: i32) -> Option<NeighbourLot> {
        if let Some(g) = &self.geometry {
            return g.lots.get(&(x, z)).copied();
        }
        self.cell_lot
    }
    fn altitude_at_vertex(&mut self, x: i32, z: i32) -> f64 {
        if let Some(g) = &self.geometry {
            return g.alt[&(x, z)];
        }
        self.edge_alt[self.cur_edge].unwrap()
    }
    fn retaining_wall_type(&mut self, _lot: &Rc<Lot>) -> u32 {
        self.log.push("type".into());
        self.wall_type
    }
    fn load_wall_properties(&mut self, _wall: &Wall, t: u32) -> bool {
        self.log.push(format!("props {t:#x}"));
        self.props_ok
    }
    fn insert_occupant(&mut self, wall: &Wall) {
        self.log.push("insert".into());
        self.inserted.push(wall.clone());
    }
    fn first_wall_at_cell(&mut self, x: i32, z: i32) -> WallLookup {
        self.log.push(format!("find {x},{z}"));
        self.lookup.clone().unwrap_or(WallLookup::NotFound)
    }
    fn remove_occupant(&mut self, _w: &WallLookup) {
        self.log.push("remove occupant".into());
    }
    fn remove_wall_model(&mut self, _wall: &Wall, edge: u32) {
        self.log.push(format!("model remove {edge}"));
    }
    fn add_wall_model(&mut self, _wall: &Wall, edge: u32, insert: bool) -> Option<bool> {
        self.log.push(format!("model add {edge} {insert}"));
        self.build[edge as usize]
    }
    fn lots_surrounding(&mut self, _lot: &Rc<Lot>, mask: u32) -> Vec<Rc<Lot>> {
        self.log.push(format!("surrounding {mask:#x}"));
        self.surrounding.clone()
    }
}

fn lot(flags: u8, rect: [u8; 4], altitude: f32) -> Rc<Lot> {
    let l = Lot::constructed();
    {
        let mut r = l.rec.borrow_mut();
        r.flags = flags;
        r.rect = rect;
        r.location = [rect[0], rect[1]];
        r.altitude = altitude;
    }
    l
}

fn wall(hashes: [i32; 4], mask: i32) -> Wall {
    let w = RecordObject::new(<RetainingWallRecord as OccupantRecord>::new());
    {
        let mut r = w.rec.borrow_mut();
        r.edge_hashes = hashes;
        r.edge_mask = mask;
        r.wall_type = 0xC96D_2137;
        r.position = [8.0, 100.0, 8.0];
        r.size = [16.0, 2.5, 16.0];
    }
    w
}

/// Altitude giving the edge hash `h` in the default geometry (no lot cell).
fn alt_for(h: i32) -> Option<f64> {
    (h != 0).then(|| f64::from(h - 1))
}

fn models(h: &Host) -> Vec<&str> {
    h.log
        .iter()
        .filter(|s| s.starts_with("model"))
        .map(String::as_str)
        .collect()
}

#[test]
fn round_altitude_is_nearest_even() {
    assert_eq!(round_altitude(102.4), 102);
    assert_eq!(round_altitude(103.6), 104);
    assert_eq!(round_altitude(2.5), 2);
    assert_eq!(round_altitude(3.5), 4);
    assert_eq!(round_altitude(-1.5), -2);
}

#[test]
fn t21_bit_0_clear_builds_nothing() {
    let l = lot(0, [10, 20, 10, 20], 100.0);
    let mut h = Host::default();
    let (al, w) = add_retaining_wall_occupant(&l, &mut h);
    assert_eq!((al, w.is_none()), (0, true));
    assert!(h.log.is_empty(), "no draw, no query");
}

#[test]
fn t21_single_edge_wall_hash_top_size_and_al_zero() {
    let l = lot(lot_flag::TERRAIN_LOCKED, [10, 20, 10, 20], 100.0);
    let g = Geometry {
        cells: HashMap::from([(1, [10, 19, 10, 19])]),
        verts: HashMap::from([(1, [10, 20, 11, 20])]),
        lots: HashMap::new(),
        alt: HashMap::from([((10, 20), 102.4), ((11, 20), 103.6)]),
    };
    let mut h = Host {
        geometry: Some(g),
        wall_type: 0xC96D_2137,
        props_ok: true,
        ..Host::default()
    };
    let (al, w) = add_retaining_wall_occupant(&l, &mut h);
    assert_eq!(al, 0, "AL = 0 even on success");
    let w = w.unwrap();
    let r = w.rec.borrow();
    assert_eq!(r.edge_hashes, [43_282, 0, 0, 0]);
    assert_eq!(r.edge_mask, 1);
    assert_eq!(r.position, [168.0, 100.0, 328.0]);
    assert_eq!(r.size, [16.0, 103.6f32 - 100.0f32, 16.0]);
    assert_eq!(r.wall_type, 0xC96D_2137);
    assert!(r.lot.object::<Lot>().is_some_and(|x| Rc::ptr_eq(&x, &l)));
    assert_eq!(
        l.rec.borrow().flags,
        lot_flag::TERRAIN_LOCKED | lot_flag::RETAINING_WALLS
    );
    assert_eq!(h.inserted.len(), 1);
    assert!(h.log.iter().any(|s| s == "props 0xc96d2137"));
}

#[test]
fn add_fails_on_props_and_skips_props_for_type_zero() {
    let l = lot(1, [0, 0, 0, 0], 10.0);
    let mut h = Host {
        edge_alt: [Some(4.0), None, None, None],
        wall_type: 7,
        props_ok: false,
        ..Host::default()
    };
    assert!(add_retaining_wall_occupant(&l, &mut h).1.is_none());
    assert!(h.inserted.is_empty());
    assert_eq!(l.rec.borrow().flags, 1, "no flag bit 2 on failure");
    h.wall_type = 0;
    let w = add_retaining_wall_occupant(&l, &mut h).1.unwrap();
    assert_eq!(w.rec.borrow().wall_type, 0);
    assert_eq!(h.log.iter().filter(|s| s.starts_with("props")).count(), 1);
    // No edge with a vertex: found = false, nothing built.
    let mut h = Host::default();
    assert!(add_retaining_wall_occupant(&l, &mut h).1.is_none());
    assert!(!h.log.iter().any(|s| s == "type"));
}

#[test]
fn neighbour_bits_and_need_rules() {
    // Every neighbour cell a terrain-locked lot: no wall on that edge.
    let mut h = Host {
        edge_alt: [Some(4.0); 4],
        cell_lot: Some(NeighbourLot { flags: 1, state: 6 }),
        ..Host::default()
    };
    let (hs, _, found) = lot_wall_edge_flags(0.0, [0, 0, 0, 0], &mut h);
    assert_eq!((hs, found), ([0; 4], false));
    // An unlocked lot not in state 6: bit 31 set, need set.
    h.cell_lot = Some(NeighbourLot { flags: 0, state: 2 });
    let (hs, top, found) = lot_wall_edge_flags(0.0, [0, 0, 0, 0], &mut h);
    assert!(found);
    assert_eq!(hs[0], (0x8000_0000u32 as i32).wrapping_add(5));
    assert_eq!(top, 4.0);
    // top is at least f32(a + 0.1).
    h.edge_alt = [Some(-3.0), None, None, None];
    let (_, top, _) = lot_wall_edge_flags(1.0, [0, 0, 0, 0], &mut h);
    assert_eq!(top, (1.0f64 + f64::from(0.1f32)) as f32);
}

#[test]
fn bit_cursor_wraps_at_the_33rd_cell() {
    // 33 cells in a row, only the 33rd holds a non-state-6 lot.
    let g = Geometry {
        cells: HashMap::from([(1, [0, 0, 32, 0])]),
        verts: HashMap::from([(1, [0, 0, 0, 0])]),
        lots: HashMap::from([((32, 0), NeighbourLot { flags: 0, state: 1 })]),
        alt: HashMap::from([((0, 0), 0.0)]),
    };
    let mut h = Host {
        geometry: Some(g),
        ..Host::default()
    };
    let (hs, _, _) = lot_wall_edge_flags(0.0, [0, 1, 0, 1], &mut h);
    assert_eq!(hs[0], (0x8000_0000u32 as i32).wrapping_add(1));
}

#[test]
fn removal_takes_the_first_wall_and_always_clears_bit_2() {
    let l = lot(1 | lot_flag::RETAINING_WALLS, [3, 4, 5, 6], 0.0);
    let mut h = Host {
        lookup: Some(WallLookup::Wall(wall([1, 0, 0, 0], 1))),
        ..Host::default()
    };
    assert_eq!(remove_retaining_wall_occupant(&l, &mut h), 1);
    assert_eq!(h.log, vec!["find 3,4", "remove occupant"]);
    assert_eq!(l.rec.borrow().flags, 1);
    l.rec.borrow_mut().flags |= lot_flag::RETAINING_WALLS;
    for (lk, al) in [
        (WallLookup::NotFound, 0),
        (WallLookup::Null, 0),
        (WallLookup::NotAWall(9), 1),
    ] {
        let mut h = Host {
            lookup: Some(lk),
            ..Host::default()
        };
        l.rec.borrow_mut().flags |= lot_flag::RETAINING_WALLS;
        assert_eq!(remove_retaining_wall_occupant(&l, &mut h), al);
        assert_eq!(l.rec.borrow().flags, 1);
    }
}

#[test]
fn t22_update_changes_hashes_and_mask_only() {
    let l = lot(1, [0, 0, 0, 0], 100.0);
    let w = wall([5, 0, 7, 9], 0b1101);
    let before = w.rec.borrow().clone();
    let mut h = Host {
        edge_alt: [alt_for(5), alt_for(4), alt_for(0), alt_for(8)],
        lookup: Some(WallLookup::Wall(w.clone())),
        build: [Some(true); 4],
        ..Host::default()
    };
    update_retaining_walls(&l, &mut h);
    assert_eq!(
        models(&h),
        vec![
            "model add 1 true",
            "model remove 2",
            "model remove 3",
            "model add 3 true"
        ]
    );
    let r = w.rec.borrow();
    assert_eq!(r.edge_hashes, [5, 4, 0, 8]);
    assert_eq!(r.edge_mask, 0b1111, "bit 2 stays");
    assert_eq!(
        (r.position, r.size, r.wall_type),
        (before.position, before.size, before.wall_type)
    );
    assert!(!h.log.iter().any(|s| s == "remove occupant" || s == "insert"));
    assert_eq!(l.rec.borrow().flags, 1);
}

#[test]
fn t23_equal_hashes_no_query_without_bit_0_and_add_when_missing() {
    let l = lot(1, [0, 0, 0, 0], 100.0);
    let w = wall([5, 0, 7, 9], 0b1101);
    let (_, bytes_before, _) = {
        let o = RefValue::Object(w.clone());
        (0, write_obj(&o), 0)
    };
    let mut h = Host {
        edge_alt: [alt_for(5), alt_for(0), alt_for(7), alt_for(9)],
        lookup: Some(WallLookup::Wall(w.clone())),
        ..Host::default()
    };
    update_retaining_walls(&l, &mut h);
    assert!(models(&h).is_empty());
    assert_eq!(write_obj(&RefValue::Object(w.clone())), bytes_before);
    // Bit 0 clear: no occupant query.
    let l0 = lot(0, [0, 0, 0, 0], 100.0);
    let mut h = Host::default();
    update_retaining_walls(&l0, &mut h);
    assert!(h.log.is_empty());
    // No wall at the location cell, or not a wall: one add.
    for lk in [WallLookup::NotFound, WallLookup::NotAWall(3)] {
        let mut h = Host {
            edge_alt: [Some(1.0), None, None, None],
            lookup: Some(lk),
            ..Host::default()
        };
        update_retaining_walls(&l, &mut h);
        assert_eq!(h.inserted.len(), 1);
        assert_eq!(h.log.iter().filter(|s| s.starts_with("find")).count(), 1);
    }
}

#[test]
fn all_zero_update_keeps_the_wall_and_its_mask() {
    let l = lot(1, [0, 0, 0, 0], 100.0);
    let w = wall([5, 6, 0, 0], 0b0011);
    let mut h = Host {
        lookup: Some(WallLookup::Wall(w.clone())),
        ..Host::default()
    };
    update_retaining_walls(&l, &mut h);
    assert_eq!(models(&h), vec!["model remove 0", "model remove 1"]);
    let r = w.rec.borrow();
    assert_eq!((r.edge_hashes, r.edge_mask), ([0; 4], 0b0011));
    assert!(!h.log.iter().any(|s| s == "remove occupant"));
}

#[test]
fn t24_failed_build_clears_the_bit_and_read_restores_it() {
    let l = lot(1, [0, 0, 0, 0], 100.0);
    let w = wall([0, 0, 0, 0], 0b0001);
    let mut h = Host {
        edge_alt: [alt_for(6), None, None, None],
        lookup: Some(WallLookup::Wall(w.clone())),
        build: [Some(false), None, None, None],
        ..Host::default()
    };
    update_retaining_walls(&l, &mut h);
    assert_eq!((w.rec.borrow().edge_hashes[0], w.rec.borrow().edge_mask), (6, 0));
    // An early return (no property holder / placeable object) leaves the mask.
    w.rec.borrow_mut().edge_mask = 0b0001;
    let mut h = Host {
        edge_alt: [alt_for(8), None, None, None],
        lookup: Some(WallLookup::Wall(w.clone())),
        ..Host::default()
    };
    update_retaining_walls(&l, &mut h);
    assert_eq!((w.rec.borrow().edge_hashes[0], w.rec.borrow().edge_mask), (8, 1));
    // Save and load: Read ORs bit 0 back in (§7, T7).
    w.rec.borrow_mut().edge_mask = 0;
    w.rec.borrow_mut().lot = RefValue::Object(l.clone());
    let bytes = write_obj(&RefValue::Object(w.clone()));
    let (o, ok, _) = read_obj(CLSID_RETAINING_WALL_OCCUPANT, &bytes);
    assert!(ok);
    let back = o.object::<RecordObject<RetainingWallRecord>>().unwrap();
    assert_eq!(back.rec.borrow().edge_mask, 1);
}

#[test]
#[should_panic]
fn update_has_no_null_guard() {
    let l = lot(1, [0, 0, 0, 0], 0.0);
    let mut h = Host {
        lookup: Some(WallLookup::Null),
        ..Host::default()
    };
    update_retaining_walls(&l, &mut h);
}

#[test]
fn t25_surrounding_in_ring_order_not_self_refcounts_kept() {
    let b = lot(1, [5, 5, 6, 6], 0.0);
    let c = lot(1, [5, 4, 5, 4], 0.0);
    let a = lot(1, [4, 5, 4, 6], 0.0);
    let (rc_a, rc_c) = (Rc::strong_count(&a), Rc::strong_count(&c));
    let mut h = Host {
        surrounding: vec![c.clone(), a.clone()],
        ..Host::default()
    };
    update_surrounding_retaining_walls(&b, &mut h);
    let finds: Vec<&str> = h
        .log
        .iter()
        .filter(|s| s.starts_with("find"))
        .map(String::as_str)
        .collect();
    assert_eq!(finds, vec!["find 5,4", "find 4,5"]);
    assert_eq!(h.log[0], "surrounding 0xf");
    h.surrounding.clear();
    assert_eq!((Rc::strong_count(&a), Rc::strong_count(&c)), (rc_a, rc_c));
}
