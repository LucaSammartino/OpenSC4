// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Foundations, slots 48–61 (§17.5). A foundation is a
//! `cSC4FoundationOccupant` (occupant type `0x68FD0C69`, interface IID
//! `0xC8FCE101`); its token is the occupant, as for every occupant the
//! host hands over. Lookups return it without a reference (§17.5.1).
//! Foundations are created by the BDS (slot 23, §17.5.3), a host call.

use super::simulator::{DrawKind, SharedStatics};
use super::terrain::TerrainView;
use super::{CellRect, LotDeveloperHost, Vec3, OCCUPANT_TYPE_BUILDING, OCCUPANT_TYPE_PROP};
use crate::x87::{ftol32, r24};

/// The foundation occupant type (§17.5).
pub const OCCUPANT_TYPE_FOUNDATION: u32 = 0x68FD_0C69;
/// `cISC4FoundationOccupant` (§17.5).
pub const IID_FOUNDATION_OCCUPANT: u32 = 0xC8FC_E101;
/// A building's own foundation type property (slots 58, 59).
pub const PROP_BUILDING_FOUNDATION_TYPE: u32 = 0x88FC_D877;

/// Foundation kinds (`GetKind`, §17.5).
pub mod kind {
    pub const BUILDING: i32 = 1;
    pub const LOT: i32 = 2;
    pub const PROP: i32 = 3;
    pub const OTHER: i32 = 4;
}

/// The lot flags that mean "has a lot foundation" (lot slots 12 / 13,
/// §17.5).
pub const LOT_FOUNDATION_FLAGS: u32 = 3;
/// The half-width of slot 50's query offset (§17.5.1).
const POSITION_PROBE: f64 = 0.05;

/// R8 (§10.2): `Uniform(n)` on the file-local generator.
fn draw_r8(statics: &SharedStatics, n: u32) -> u32 {
    let mut s = statics.borrow_mut();
    let v = s.rng.uniform_u32(n);
    s.record(DrawKind::R8, n, f64::from(v));
    v
}

impl TerrainView {
    /// The first foundation in `list` (list order) that answers
    /// `QueryInterface(0xC8FCE101)` with `GetKind() = kind`.
    fn first_of_kind(host: &dyn LotDeveloperHost, list: Vec<u64>, kind: i32) -> Option<u64> {
        list.into_iter().find(|&o| host.foundation_kind(o) == Some(kind))
    }

    /// Slot 49 `GetFoundationByStandardCityCell(kind, x, z)` (§17.5.1):
    /// the foundations on cell `(x, z)` (occupant manager slot 41).
    pub fn foundation_by_cell(&self, host: &dyn LotDeveloperHost, kind: i32, x: i32, z: i32) -> Option<u64> {
        let list = host.occupants_in_cells(CellRect::new(x, z, x, z), OCCUPANT_TYPE_FOUNDATION);
        Self::first_of_kind(host, list, kind)
    }

    /// Slot 50 `GetFoundationByPosition(kind, x, z)` (§17.5.1): occupant
    /// manager slot 37 with the vertical line at `(x − 0.05, z + 0.05)`
    /// (quirk: not a 0.1 m square).
    pub fn foundation_by_position(&self, host: &dyn LotDeveloperHost, kind: i32, x: f32, z: f32) -> Option<u64> {
        let lx = r24(f64::from(x) - POSITION_PROBE) as f32;
        let lz = r24(f64::from(z) + POSITION_PROBE) as f32;
        let min: Vec3 = [lx, -f32::MAX, lz];
        let max: Vec3 = [lx, f32::MAX, lz];
        let list = host.occupants_in_box(min, max, OCCUPANT_TYPE_FOUNDATION);
        Self::first_of_kind(host, list, kind)
    }

    /// Slot 48 `GetFoundationByOccupant(occ)` (§17.5.1): slot 50 at the
    /// occupant's position, kind 1 for a building, 3 for a prop, else 4.
    pub fn foundation_by_occupant(&self, host: &dyn LotDeveloperHost, occ: u64) -> Option<u64> {
        let p = host.occupant_position(occ);
        let k = match host.occupant_type(occ) {
            OCCUPANT_TYPE_BUILDING => kind::BUILDING,
            OCCUPANT_TYPE_PROP => kind::PROP,
            _ => kind::OTHER,
        };
        self.foundation_by_position(host, k, p[0], p[2])
    }

    /// Slot 51 `GetLotFoundation(lot)` (§17.5.1): slot 49 (kind 2) at the
    /// lot's location.
    pub fn lot_foundation(&self, host: &dyn LotDeveloperHost, lot: u64) -> Option<u64> {
        let (x, z) = host.lot_location(lot);
        self.foundation_by_cell(host, kind::LOT, x, z)
    }

    /// Clears the "has a lot foundation" flags of `lot`.
    fn clear_foundation_flags(host: &dyn LotDeveloperHost, lot: u64) {
        let f = host.lot_flags(lot);
        host.lot_set_flags(lot, f & !LOT_FOUNDATION_FLAGS);
    }

    /// Slot 52 `RemoveFoundation(kind, x, z)` (§17.5.2): removes the
    /// foundation slot 50 finds (occupant manager slot 21 with `(1, 0)`);
    /// for kind 2 the lot at `(trunc(x / 16), trunc(z / 16))` loses its
    /// foundation flags whether or not one was found.
    pub fn remove_foundation(&self, host: &dyn LotDeveloperHost, k: i32, x: f32, z: f32) -> bool {
        let found = self.foundation_by_position(host, k, x, z);
        if let Some(f) = found {
            host.occupant_remove(f, 1, 0);
        }
        if k == kind::LOT {
            let lx = ftol32(r24(f64::from(x) * 0.0625));
            let lz = ftol32(r24(f64::from(z) * 0.0625));
            if let Some(l) = host.get_lot(lx, lz, false) {
                Self::clear_foundation_flags(host, l);
            }
        }
        found.is_some()
    }

    /// Slot 53 `RemoveLotFoundation(x, z)` (§17.5.2): slot 49 (kind 2),
    /// removed as in slot 52; then the lot at `(x, z)` loses its foundation
    /// flags.
    pub fn remove_lot_foundation_at(&self, host: &dyn LotDeveloperHost, x: i32, z: i32) -> bool {
        let found = self.foundation_by_cell(host, kind::LOT, x, z);
        if let Some(f) = found {
            host.occupant_remove(f, 1, 0);
        }
        if let Some(l) = host.get_lot(x, z, false) {
            Self::clear_foundation_flags(host, l);
        }
        found.is_some()
    }

    /// `RemoveLotFoundation(lot)` (`0x006C3F30`, private, §17.5.2): slot 53
    /// at the lot's location, then the lot's foundation flags are cleared
    /// again.
    pub fn remove_lot_foundation(&self, host: &dyn LotDeveloperHost, lot: u64) -> bool {
        let (x, z) = host.lot_location(lot);
        let r = self.remove_lot_foundation_at(host, x, z);
        Self::clear_foundation_flags(host, lot);
        r
    }

    /// Slot 60 `RemoveBuildingFoundation(occ)` (§17.5.2): slot 52 (kind 1)
    /// at the occupant's position.
    pub fn remove_building_foundation(&self, host: &dyn LotDeveloperHost, occ: u64) -> bool {
        let p = host.occupant_position(occ);
        self.remove_foundation(host, kind::BUILDING, p[0], p[2])
    }

    /// Slot 54 `AddLotFoundation(lot, cfg, useLotAltitude, level, walls)`
    /// (§17.5.4): the type is a draw (R8) over the primary foundation
    /// object's IDs, else its `+0x20`, else the tuning's default; type 0 →
    /// false. R8 is drawn before any terrain or occupant change.
    #[allow(clippy::too_many_arguments)]
    pub fn add_lot_foundation(
        &self,
        host: &dyn LotDeveloperHost,
        statics: &SharedStatics,
        lot: u64,
        cfg: Option<u64>,
        use_lot_altitude: bool,
        level: bool,
        walls: bool,
    ) -> bool {
        let Some(cfg) = cfg else {
            return false;
        };
        let mut ftype = self.tuning.default_lot_foundation_type;
        if let Some(obj) = host.cfg_primary_foundation_object(cfg) {
            let n = obj.ids.len();
            if n > 0 {
                let i = draw_r8(statics, n as u32) as usize;
                ftype = obj.ids[i.min(n - 1)];
            } else {
                ftype = obj.foundation;
            }
        }
        if ftype == 0 {
            return false;
        }
        self.add_lot_foundation_type(host, statics, lot, ftype, use_lot_altitude, level, walls)
    }

    /// Slot 55 `AddLotFoundation(lot, type, useLotAltitude, level, walls)`
    /// (§17.5.4): the foundation's top is the lot's altitude or the picked
    /// altitude; with `level` the terrain is levelled to it (after the
    /// bottom and the highest vertex were taken); the BDS creates the
    /// foundation (slot 23); the lot takes the top as its altitude and the
    /// foundation flags; with `walls` and a top below the highest vertex the
    /// retaining walls and the neighbours' foundations are refreshed.
    /// Returns the BDS's AL.
    #[allow(clippy::too_many_arguments)]
    pub fn add_lot_foundation_type(
        &self,
        host: &dyn LotDeveloperHost,
        statics: &SharedStatics,
        lot: u64,
        ftype: u32,
        use_lot_altitude: bool,
        level: bool,
        walls: bool,
    ) -> bool {
        // 1.–2.
        let mut c = host.lot_center(lot);
        let r = host.lot_bounding_rect(lot);
        let f = host.lot_facing(lot);
        let cells = |a: i32, b: i32| b.wrapping_sub(a).wrapping_add(1).wrapping_mul(16) as f32;
        let dx = cells(r.x1, r.x2);
        let dz = cells(r.z1, r.z2);
        // 3.
        let bottom = self.lot_foundation_bottom_altitude(host, lot);
        let hi = self.lowest_or_highest_area_altitude(host, r, false, true);
        // 4.
        let top = if use_lot_altitude {
            host.lot_altitude(lot)
        } else {
            let corners = self.corner_altitudes(host, r, true);
            self.pick_lot_foundation_altitude(host, lot, &corners, f)
        };
        c[1] = top;
        // 5.
        if level {
            if r.x2.wrapping_sub(r.x1) > 1 && r.z2.wrapping_sub(r.z1) > 1 {
                let target = r24(f64::from(top) - 0.5) as f32;
                self.level_area_internal_vertices(host, r, target, true, true, Some(lot));
            }
            self.level_bordering_vertices(host, r, top, Some(lot));
        }
        // 6.
        let ok = host.add_new_foundation_occupant(ftype, kind::LOT, &mut c, dx, dz, f, bottom);
        // 7.
        host.lot_set_altitude(lot, c[1]);
        let flags = host.lot_flags(lot);
        host.lot_set_flags(lot, (flags & !4) | LOT_FOUNDATION_FLAGS);
        // 8.
        if walls && c[1] < hi {
            host.add_retaining_wall_occupant(lot, 1);
            self.update_surrounding_lot_foundations(host, statics, lot);
            host.update_surrounding_retaining_walls(lot);
        }
        ok
    }

    /// `PickLotFoundationAltitude(lot, …, corners)` (`0x006C5360`, private,
    /// §17.5.4): a water-compatible configuration's first water cell (x
    /// outer) gives that cell centre's water level + 0.5; otherwise the
    /// middle of the two corners of the side the lot faces.
    /// *[choice]* A facing outside 0–3 (the original indexes past the
    /// corner array) is taken modulo 4.
    pub fn pick_lot_foundation_altitude(
        &self,
        host: &dyn LotDeveloperHost,
        lot: u64,
        corners: &[f32; 4],
        f: i32,
    ) -> f32 {
        if let Some(cfg) = host.lot_configuration(lot) {
            if host.cfg_is_compatible_with_water(cfg) {
                let r = host.lot_bounding_rect(lot);
                let mut x = i64::from(r.x1);
                while x <= i64::from(r.x2) {
                    let mut z = i64::from(r.z1);
                    while z <= i64::from(r.z2) {
                        if host.terrain_cell_has_water(x as i32, z as i32) {
                            let (px, pz) = host.cell_center_to_position(x as i32, z as i32);
                            return r24(f64::from(host.terrain_water_level_at(px, pz)) + 0.5) as f32;
                        }
                        z += 1;
                    }
                    x += 1;
                }
            }
        }
        let k = (f & 3) as usize;
        let a = corners[k];
        let b = corners[(k + 1) & 3];
        let (lo, hi) = if b < a { (b, a) } else { (a, b) };
        let half = r24(r24(f64::from(hi) - f64::from(lo)) * 0.5);
        r24(f64::from(lo) + half) as f32
    }

    /// `GetLotFoundationBottomAltitude(lot)` (`0x006C7630`, private,
    /// §17.5.4): the lowest vertex of the lot, lowered to the altitude of
    /// any surrounding lot (`GetLotsSurroundingLot(…, 0xF)`) that has a lot
    /// foundation (strict, NaN no).
    pub fn lot_foundation_bottom_altitude(&self, host: &dyn LotDeveloperHost, lot: u64) -> f32 {
        let r = host.lot_bounding_rect(lot);
        let mut b = self.lowest_or_highest_area_altitude(host, r, true, true);
        let mut list = Vec::new();
        host.lots_surrounding_lot(lot, &mut list, 0xF);
        for n in list {
            if host.lot_flags(n) & LOT_FOUNDATION_FLAGS != 0 {
                let a = host.lot_altitude(n);
                if a < b {
                    b = a;
                }
            }
        }
        b
    }

    /// `UpdateSurroundingLotFoundations(lot)` (`0x006C7720`, private,
    /// §17.5.4): slot 61 on every surrounding lot with a lot foundation, in
    /// list order.
    pub fn update_surrounding_lot_foundations(&self, host: &dyn LotDeveloperHost, statics: &SharedStatics, lot: u64) {
        let mut list = Vec::new();
        host.lots_surrounding_lot(lot, &mut list, 0xF);
        for n in list {
            if host.lot_flags(n) & LOT_FOUNDATION_FLAGS != 0 {
                self.update_lot_foundation(host, statics, n);
            }
        }
    }

    /// Slot 61 `UpdateLotFoundation(lot)` (§17.5.4): nothing without the
    /// foundation flags (AL is then the flags' low byte); an existing lot
    /// foundation is replaced by one of the same type at the lot's altitude
    /// (slot 55, no levelling, no walls); otherwise slot 54 with the lot's
    /// configuration (R8 possible).
    pub fn update_lot_foundation(&self, host: &dyn LotDeveloperHost, statics: &SharedStatics, lot: u64) -> bool {
        let flags = host.lot_flags(lot);
        if flags & LOT_FOUNDATION_FLAGS == 0 {
            return flags & 0xFF != 0;
        }
        if let Some(fnd) = self.lot_foundation(host, lot) {
            let t = host.foundation_type(fnd);
            self.remove_lot_foundation(host, lot);
            return self.add_lot_foundation_type(host, statics, lot, t, true, false, false);
        }
        let cfg = host.lot_configuration(lot);
        self.add_lot_foundation(host, statics, lot, cfg, true, false, false)
    }

    /// Slot 56 `IsLotObjectFoundationRequired(occ)` (§17.5.5): the
    /// occupant's slope (with water) above `kMaxSlopeForFlatness`.
    pub fn is_occupant_foundation_required(&self, host: &dyn LotDeveloperHost, occ: u64) -> bool {
        self.occupant_slope(host, occ, true) > self.tuning.max_slope_for_flatness
    }

    /// Slot 57 `IsLotObjectFoundationRequired(min, max)` (§17.5.5): the
    /// slope of the cells of the box, one cell larger in x and z (quirk).
    pub fn is_box_foundation_required(&self, host: &dyn LotDeveloperHost, min: Vec3, max: Vec3) -> bool {
        let (a, b) = host.position_to_cell(min[0], min[2]);
        let (c, d) = host.position_to_cell(max[0], max[2]);
        let rect = CellRect::new(a, b, c.wrapping_add(1), d.wrapping_add(1));
        self.area_slope(host, rect, true) > self.tuning.max_slope_for_flatness
    }

    /// Slot 58 `AddBuildingFoundation(building, cfg)` (§17.5.5): the type is
    /// the primary building object's `+0x20` when non-zero, else the
    /// building's `0x88FCD877` property when present, else the tuning's
    /// default (`0x6916516B`); type 0 → false; else slot 59.
    pub fn add_building_foundation(&self, host: &dyn LotDeveloperHost, building: u64, cfg: u64) -> bool {
        let mut t = self.tuning.default_building_foundation_type;
        // LD7: a null configuration (token 0) reads as having no primary
        // object *[choice]*.
        let primary = if cfg == 0 {
            None
        } else {
            host.cfg_primary_building_object(cfg)
        };
        match primary {
            Some(obj) if obj.foundation != 0 => t = obj.foundation,
            _ => {
                if let Some(v) = host.occupant_property(building, PROP_BUILDING_FOUNDATION_TYPE) {
                    t = v;
                }
            }
        }
        if t == 0 {
            return false;
        }
        self.add_building_foundation_type(host, building, t)
    }

    /// Slot 59 `AddBuildingFoundation(building, type)` (§17.5.5): a kind-1
    /// foundation whose top is the building's position and whose bottom is
    /// the lowest vertex under it (BDS slot 23 with bottom −1). Type 0 reads
    /// `0x88FCD877`, then the tuning's default.
    pub fn add_building_foundation_type(&self, host: &dyn LotDeveloperHost, building: u64, ftype: u32) -> bool {
        let mut pos = host.occupant_position(building);
        let (min, max) = host.occupant_bounding_box(building);
        let o = host.building_orientation(building);
        let mut t = ftype;
        if t == 0 {
            t = host
                .occupant_property(building, PROP_BUILDING_FOUNDATION_TYPE)
                .unwrap_or(0);
            if t == 0 {
                t = self.tuning.default_building_foundation_type;
            }
        }
        let dx = r24(f64::from(max[0]) - f64::from(min[0])) as f32;
        let dz = r24(f64::from(max[2]) - f64::from(min[2])) as f32;
        host.add_new_foundation_occupant(t, kind::BUILDING, &mut pos, dx, dz, o, -1.0)
    }
}
