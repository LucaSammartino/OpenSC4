// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Object positions, slots 39 and 40 (§17.6).

use super::terrain::TerrainView;
use super::{object_type, CellRect, CfgObject, LotDeveloperHost, Vec3};
use crate::x87::{ftol32, r24};

impl TerrainView {
    /// Slot 39 `GetPositionForLotConfigurationObject(lot, cfg, obj, pos,
    /// orient)` (§17.6): the world position and orientation of the
    /// configuration object `obj` on `lot`. The original returns `&orient`
    /// in EAX (the plugin does the same); this returns what it writes.
    ///
    /// The lot's facing `f` (lot slot 23) rotates the object's lot-space
    /// offset about the lot's corners; a facing outside 0–3 gives
    /// `(0, ·, 0)` and the footprint `(0, 1, 0, 1)`. The ground is the lot's
    /// altitude when its flags have bit 0, else the lot foundation's top,
    /// else the terrain under the object (types 1, 2, 4) or the highest
    /// altitude under its footprint (slot 16, other types).
    pub fn object_position(&self, host: &dyn LotDeveloperHost, lot: u64, cfg: Option<u64>, obj: &CfgObject) -> (Vec3, i32) {
        // 1. Corners and facing.
        let r: CellRect = host.lot_bounding_rect(lot);
        let (c0x, c0z) = host.cell_corner_to_position(r.x1, r.z1);
        let (c1x, c1z) = host.cell_corner_to_position(r.x2.wrapping_add(1), r.z2.wrapping_add(1));
        let f = host.lot_facing(lot);
        // 2. The offsets; a base texture snaps to its lot-space cell centre.
        let [mut ox, oy, mut oz] = obj.offset;
        if obj.object_type == object_type::TEXTURE {
            let snap = |v: f32| (r24((f64::from(ftol32(r24(f64::from(v) * 0.0625))) + 0.5) * 16.0)) as f32;
            ox = snap(ox);
            oz = snap(oz);
        }
        // 3. Position and footprint, one r24 add or subtract each.
        let add = |a: f32, b: f32| (r24(f64::from(a) + f64::from(b))) as f32;
        let sub = |a: f32, b: f32| (r24(f64::from(a) - f64::from(b))) as f32;
        let [mn_x, mn_z] = obj.footprint_min;
        let [mx_x, mx_z] = obj.footprint_max;
        let (px, pz, foot) = match f {
            0 => (
                sub(c1x, ox),
                sub(c1z, oz),
                [sub(c1x, mx_x), sub(c1z, mx_z), sub(c1x, mn_x), sub(c1z, mn_z)],
            ),
            1 => (
                add(c0x, oz),
                sub(c1z, ox),
                [add(c0x, mn_z), sub(c1z, mx_x), add(c0x, mx_z), sub(c1z, mn_x)],
            ),
            2 => (
                add(c0x, ox),
                add(c0z, oz),
                [add(c0x, mn_x), add(c0z, mn_z), add(c0x, mx_x), add(c0z, mx_z)],
            ),
            3 => (
                sub(c1x, oz),
                add(c0z, ox),
                [sub(c1x, mx_z), add(c0z, mn_x), sub(c1x, mn_z), add(c0z, mx_x)],
            ),
            _ => (0.0, 0.0, [0.0, 1.0, 0.0, 1.0]),
        };
        // 4. The ground.
        let y = if host.lot_flags(lot) & 1 != 0 {
            host.lot_altitude(lot)
        } else if let Some(fnd) = self.lot_foundation(host, lot) {
            host.occupant_position(fnd)[1]
        } else {
            let water = cfg.is_some_and(|c| host.cfg_is_compatible_with_water(c));
            match obj.object_type {
                object_type::PROP | object_type::TEXTURE | object_type::FLORA => host.terrain_altitude_at(px, pz),
                _ => self.lowest_or_highest_area_altitude_f(host, foot, false, water),
            }
        };
        // 5. Height and orientation.
        let py = (r24(f64::from(oy) + f64::from(y))) as f32;
        let orient = i32::from(obj.orientation).wrapping_add(f) & 3;
        ([px, py, pz], orient)
    }

    /// Slot 40 `GetPositionForBuilding(lot, pos, orient)` (§17.6): slot 39
    /// for the configuration's primary building object; `None` (EAX 0,
    /// nothing written) without a configuration or primary object.
    pub fn position_for_building(&self, host: &dyn LotDeveloperHost, lot: u64) -> Option<(Vec3, i32)> {
        let cfg = host.lot_configuration(lot)?;
        let obj = host.cfg_primary_building_object(cfg)?;
        Some(self.object_position(host, lot, Some(cfg), &obj))
    }
}
