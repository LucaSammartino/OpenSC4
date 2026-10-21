//! Retaining walls: build, remove and refresh (`docs/specs/lot-records.md`
//! §7.1, §7.2; LR5).
//!
//! The wall record is [`RetainingWallRecord`] (§7). Everything outside the
//! lot and the wall record goes through [`WallHost`]: the lot manager, the
//! occupant manager, terrain, the lot's wall type draw, BDS property loading
//! and the visual model build of `AddRetainingWallModel` (whose only effect
//! on saved state, the edge mask, is applied here).

use std::rc::Rc;

use super::lot::{lot_flag, Lot};
use super::{RecordObject, RetainingWallRecord};
use crate::stream::{RefValue, SaveObject};

/// A retaining-wall occupant held by the core.
pub type Wall = Rc<RecordObject<RetainingWallRecord>>;

/// A neighbour lot as `GetLotWallEdgeFlags` sees it (lot slots 12, 33).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NeighbourLot {
    pub flags: u32,
    pub state: u32,
}

/// Occupant manager slot 33 `GetFirstOccupantByStandardCityCell(&w, x, z,
/// 0x895D1169)`.
#[derive(Clone, Debug)]
pub enum WallLookup {
    /// Returned false.
    NotFound,
    /// Returned true with `w` null.
    Null,
    /// Returned true; `w` does not answer QI `0x295D1162` (host handle).
    NotAWall(u64),
    /// Returned true with a wall.
    Wall(Wall),
}

/// What the wall code needs from the rest of the city.
pub trait WallHost {
    /// Lot manager slot 12 `GetNeighborCellsAtEdgeOfLot(R, edge_flag, C)`:
    /// `Some(C)` (x1, z1, x2, z2 inclusive) when it returns true.
    fn neighbour_cells_at_edge(&mut self, rect: [i32; 4], edge_flag: u32) -> Option<[i32; 4]>;
    /// Lot manager slot 11 `GetVerticesAtEdgeOfLot(R, edge_flag, V)`: the
    /// vertex rect V (x1, z1, x2, z2 inclusive). Its return is not tested;
    /// spec silent on V after a false return (LR6): the host always fills it.
    fn vertices_at_edge(&mut self, rect: [i32; 4], edge_flag: u32) -> [i32; 4];
    /// Lot manager slot 6 `GetLot(x, z, false)`.
    fn lot_at(&mut self, x: i32, z: i32) -> Option<NeighbourLot>;
    /// Terrain slot 14 `GetAltitudeAtVertex(x, z)`.
    fn altitude_at_vertex(&mut self, x: i32, z: i32) -> f64;
    /// `GetRetainingWallType(lot)` (`0x005E2ED0`, lot-configuration.md: one
    /// draw, 0 without a configuration or list).
    fn retaining_wall_type(&mut self, lot: &Rc<Lot>) -> u32;
    /// BDS slot 29 `LoadLotRetainingWallProperties(wall, t)`.
    fn load_wall_properties(&mut self, wall: &Wall, wall_type: u32) -> bool;
    /// Occupant manager slot 20: insert the new wall.
    fn insert_occupant(&mut self, wall: &Wall);
    /// Occupant manager slot 33 at cell (x, z), type `0x895D1169`.
    fn first_wall_at_cell(&mut self, x: i32, z: i32) -> WallLookup;
    /// Occupant manager slot 21 `RemoveOccupant(w, 1, 0)` for a found occupant.
    fn remove_occupant(&mut self, w: &WallLookup);
    /// `AddRetainingWallModel(w, W, edge, false, false)` visual part: find the
    /// edge's model, remove it from the renderer when inserted, detach it.
    fn remove_wall_model(&mut self, wall: &Wall, edge: u32);
    /// `AddRetainingWallModel(w, W, edge, true, insert)` visual part: `None`
    /// when the property holder or the placeable object is null (return before
    /// the mask is touched), else `Some(ok)` of the model's vertex setup.
    fn add_wall_model(&mut self, wall: &Wall, edge: u32, insert: bool) -> Option<bool>;
    /// Lot manager slot 9 `GetLotsSurroundingLot(lot, list, 0xF)`: the ring
    /// one cell outside the rect in zones.md §8.3 row order, each lot once,
    /// the lot itself excluded.
    fn lots_surrounding(&mut self, lot: &Rc<Lot>, mask: u32) -> Vec<Rc<Lot>>;
}

fn lot_rect(lot: &Lot) -> [i32; 4] {
    lot.rec.borrow().rect.map(i32::from)
}

/// `lot.SetFlags(f)` for a change of bit 2 only: bit 0 is unchanged, so
/// `SetFlags` only stores (§3.5) and no terrain view call can follow.
fn store_flags(lot: &Lot, f: u8) {
    lot.rec.borrow_mut().flags = f;
}

/// `round(y)` of `0x005E2E6A`–`0x005E2E87`: `bits(f32(y + 12582912.0)) −
/// 0x4B400000` (round to nearest, ties to even, exact for |y| < 2^22).
pub fn round_altitude(y: f64) -> i32 {
    // x87 adds in extended precision before the float store; f64 here.
    (((y + 12_582_912.0) as f32).to_bits() as i32).wrapping_sub(0x4B40_0000)
}

/// `GetLotWallEdgeFlags(lot, a, R, h[4], &top)` (`0x005E2CE0`, §7.1): the
/// four edge hashes, `top` and `found`.
pub fn lot_wall_edge_flags(a: f32, rect: [i32; 4], host: &mut dyn WallHost) -> ([i32; 4], f32, bool) {
    // Float 0.1 at `0x00A8C950`, added in extended precision, stored as float.
    let mut top = (f64::from(a) + f64::from(0.1f32)) as f32;
    let mut found = false;
    let mut h = [0i32; 4];
    for (i, hi) in h.iter_mut().enumerate() {
        let flag = 1u32 << i;
        let mut need = false;
        let mut bits: u32 = 0;
        let mut b: u32 = 0x8000_0000;
        let Some([cx1, cz1, cx2, cz2]) = host.neighbour_cells_at_edge(rect, flag) else {
            continue;
        };
        for x in cx1..=cx2 {
            for z in cz1..=cz2 {
                let l = host.lot_at(x, z);
                match l {
                    None => need = true,
                    Some(n) => {
                        if n.flags & 1 == 0 {
                            need = true;
                        }
                        if n.state != 6 {
                            bits |= b;
                        }
                    }
                }
                b >>= 1;
                if b == 0 {
                    // The 33rd cell reuses bit 31.
                    b = 0x8000_0000;
                }
            }
        }
        if !need {
            continue;
        }
        let mut hash = bits as i32;
        let [vx1, vz1, vx2, vz2] = host.vertices_at_edge(rect, flag);
        for x in vx1..=vx2 {
            for z in vz1..=vz2 {
                found = true;
                let y = host.altitude_at_vertex(x, z);
                // x87 compare; NaN is not greater.
                if y > f64::from(top) {
                    top = y as f32;
                }
                hash = hash
                    .wrapping_add(round_altitude(y).wrapping_mul(x).wrapping_mul(z))
                    .wrapping_add(1);
            }
        }
        *hi = hash;
    }
    (h, top, found)
}

/// Edge mask of §7.1 step 5: bit i when `h[i] ≠ 0`.
pub fn mask_from_hashes(h: &[i32; 4]) -> i32 {
    h.iter().enumerate().fold(0, |m, (i, &v)| m | (i32::from(v != 0) << i))
}

/// `AddRetainingWallOccupant(lot, true)` (`0x005E3070`, §7.1). Returns AL,
/// which is 0 on every path *[quirk]*; the built wall (if any) is the second
/// value, for the caller's bookkeeping.
pub fn add_retaining_wall_occupant(lot: &Rc<Lot>, host: &mut dyn WallHost) -> (u8, Option<Wall>) {
    // Step 1.
    if lot.rec.borrow().flags & lot_flag::TERRAIN_LOCKED == 0 {
        return (0, None);
    }
    // Step 2.
    let rect = lot_rect(lot);
    let a = lot.rec.borrow().altitude;
    // Step 3.
    let (h, top, found) = lot_wall_edge_flags(a, rect, host);
    if !found {
        return (0, None);
    }
    // Step 4.
    let wall: Wall = RecordObject::new(<RetainingWallRecord as super::OccupantRecord>::new());
    let t = host.retaining_wall_type(lot);
    if t != 0 && !host.load_wall_properties(&wall, t) {
        return (0, None);
    }
    {
        let mut w = wall.rec.borrow_mut();
        // Step 5.
        w.edge_mask = mask_from_hashes(&h);
        // Step 6.
        let [x1, z1, x2, z2] = rect;
        w.position = [(16 * (x1 + x2 + 1)) as f32 * 0.5, a, (16 * (z1 + z2 + 1)) as f32 * 0.5];
        w.size = [(16 * (x2 - x1 + 1)) as f32, top - a, (16 * (z2 - z1 + 1)) as f32];
        // Step 7: lot, hashes, type (`t`, 0 when `t = 0`; the record's
        // constructor value is 0 as well, so both readings agree).
        w.lot = RefValue::Object(lot.clone() as Rc<dyn SaveObject>);
        w.edge_hashes = h;
        w.wall_type = t;
    }
    host.insert_occupant(&wall);
    let f = lot.rec.borrow().flags | lot_flag::RETAINING_WALLS;
    store_flags(lot, f);
    // Step 8: AL = 0 even on success.
    (0, Some(wall))
}

/// `AddRetainingWallOccupant(lot, false)` (removal, §7.1): remove the first
/// wall at the location cell, then always clear lot flag bit 2. Returns AL
/// (1 when a wall was removed).
pub fn remove_retaining_wall_occupant(lot: &Rc<Lot>, host: &mut dyn WallHost) -> u8 {
    let [x, z] = lot.rec.borrow().location.map(i32::from);
    let w = host.first_wall_at_cell(x, z);
    let result = match w {
        WallLookup::NotFound | WallLookup::Null => 0,
        WallLookup::NotAWall(_) | WallLookup::Wall(_) => {
            host.remove_occupant(&w);
            1
        }
    };
    let f = lot.rec.borrow().flags & !lot_flag::RETAINING_WALLS;
    store_flags(lot, f);
    result
}

/// `AddRetainingWallModel(w, W, edge, add, insert)` (`0x005E3400`, §7.2):
/// the visual work is the host's; removal leaves the mask, a build sets
/// (ok) or clears (not ok) mask bit `edge`.
pub fn add_retaining_wall_model(wall: &Wall, edge: u32, add: bool, insert: bool, host: &mut dyn WallHost) {
    if !add {
        // `insert` is unused on removal; the mask bit is left as it was.
        host.remove_wall_model(wall, edge);
        return;
    }
    let Some(ok) = host.add_wall_model(wall, edge, insert) else {
        return;
    };
    let mut w = wall.rec.borrow_mut();
    if ok {
        w.edge_mask |= 1 << edge;
    } else {
        w.edge_mask &= !(1 << edge);
    }
}

/// `UpdateRetainingWalls(lot)` (`0x005E37F0`, §7.2, no return).
pub fn update_retaining_walls(lot: &Rc<Lot>, host: &mut dyn WallHost) {
    // Step 1.
    if lot.rec.borrow().flags & lot_flag::TERRAIN_LOCKED == 0 {
        return;
    }
    // Step 2.
    let [x, z] = lot.rec.borrow().location.map(i32::from);
    let wall = match host.first_wall_at_cell(x, z) {
        WallLookup::NotFound => {
            add_retaining_wall_occupant(lot, host);
            return;
        }
        // Step 3: the original calls QueryInterface through `w` without a
        // null test; no guard is synthesised here (LR7).
        WallLookup::Null => panic!("UpdateRetainingWalls: occupant lookup returned true with a null wall"),
        WallLookup::NotAWall(_) => {
            add_retaining_wall_occupant(lot, host);
            return;
        }
        WallLookup::Wall(w) => w,
    };
    // Step 4.
    let rect = lot_rect(lot);
    let a = lot.rec.borrow().altitude;
    let old = wall.rec.borrow().edge_hashes;
    // Step 5: `found` and `top` are ignored.
    let (new, _top, _found) = lot_wall_edge_flags(a, rect, host);
    // Step 6.
    let mut changed = false;
    for i in 0..4 {
        if old[i] != new[i] {
            if old[i] != 0 {
                add_retaining_wall_model(&wall, i as u32, false, false, host);
            }
            if new[i] != 0 {
                add_retaining_wall_model(&wall, i as u32, true, true, host);
            }
            changed = true;
        }
    }
    // Step 7: a plain copy; the mask is not touched.
    if changed {
        wall.rec.borrow_mut().edge_hashes = new;
    }
}

/// `UpdateSurroundingRetainingWalls(lot)` (`0x005E3960`, §7.2): update every
/// lot of the surrounding ring in list order, never the lot itself.
pub fn update_surrounding_retaining_walls(lot: &Rc<Lot>, host: &mut dyn WallHost) {
    let list = host.lots_surrounding(lot, 0xF);
    for l in &list {
        update_retaining_walls(l, host);
    }
    // The list's references are released when it drops.
}
