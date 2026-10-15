// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Boundary textures of empty lots, slots 42 and 43 (§17.7).

use super::terrain::TerrainView;
use super::{msg, LotDeveloperHost, TextureRecord, OCCUPANT_TYPE_BASE_TEXTURE};

/// One boundary rule (§17.7): the texture instance, its rotation and flip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundaryRule {
    pub texture: u32,
    pub rotation: u8,
    pub flip: bool,
}

/// The 47 rules `InitBoundaryRules` (`0x006CA160`) puts in the boundary map
/// (§17.7), as `(key, texture, rotation, flip)`. The key is the side code
/// `s0·0x1000 + s1·0x100 + s2·0x10 + s3`.
pub const BOUNDARY_RULES: [(i32, u32, u8, bool); 47] = [
    (0x0000, 0x3000, 0, false),
    (0x1000, 0x2000, 0, false),
    (0x0100, 0x2000, 1, false),
    (0x0010, 0x2000, 2, false),
    (0x0001, 0x2000, 3, false),
    (0x2000, 0x2600, 0, false),
    (0x0200, 0x2600, 1, false),
    (0x0020, 0x2600, 2, false),
    (0x0002, 0x2600, 3, false),
    (0x1001, 0x2100, 0, false),
    (0x1100, 0x2100, 1, false),
    (0x0110, 0x2100, 2, false),
    (0x0011, 0x2100, 3, false),
    (0x2001, 0x2400, 0, false),
    (0x1200, 0x2400, 1, false),
    (0x0120, 0x2400, 2, false),
    (0x0012, 0x2400, 3, false),
    (0x2100, 0x2400, 0, true),
    (0x0210, 0x2400, 1, true),
    (0x0021, 0x2400, 2, true),
    (0x1002, 0x2400, 3, true),
    (0x1010, 0x2500, 0, false),
    (0x0101, 0x2500, 1, false),
    (0x2010, 0x2900, 0, false),
    (0x0201, 0x2900, 1, false),
    (0x1020, 0x2900, 2, false),
    (0x0102, 0x2900, 3, false),
    (0x1101, 0x2200, 0, false),
    (0x1110, 0x2200, 1, false),
    (0x0111, 0x2200, 2, false),
    (0x1011, 0x2200, 3, false),
    (0x2011, 0x2700, 0, false),
    (0x1201, 0x2700, 1, false),
    (0x1120, 0x2700, 2, false),
    (0x0112, 0x2700, 3, false),
    (0x2110, 0x2700, 0, true),
    (0x0211, 0x2700, 1, true),
    (0x1021, 0x2700, 2, true),
    (0x1102, 0x2700, 3, true),
    (0x2101, 0x2800, 0, false),
    (0x1210, 0x2800, 1, false),
    (0x0121, 0x2800, 2, false),
    (0x1012, 0x2800, 3, false),
    (0x2111, 0x2300, 0, false),
    (0x1211, 0x2300, 1, false),
    (0x1121, 0x2300, 2, false),
    (0x1112, 0x2300, 3, false),
];

/// The rule for `key`, if the map has one.
fn rule(key: i32) -> Option<BoundaryRule> {
    BOUNDARY_RULES
        .iter()
        .find(|r| r.0 == key)
        .map(|&(_, texture, rotation, flip)| BoundaryRule {
            texture,
            rotation,
            flip,
        })
}

/// The zones whose lots get no empty-lot texture (§17.7 slot 43 step 1):
/// none, airport, seaport, plopped.
pub fn zone_has_no_empty_texture(zone: i32) -> bool {
    matches!(zone, 0 | 11 | 12 | 15)
}

/// The five bytes after the flip of an empty-lot record (§17.7): the zone's
/// drag colour (top three bytes), the display alpha and 3.
pub fn empty_lot_extra(colour: u32, alpha: u8) -> [u8; 5] {
    [
        (colour >> 24) as u8,
        ((colour >> 16) & 0xFF) as u8,
        ((colour >> 8) & 0xFF) as u8,
        alpha,
        3,
    ]
}

impl TerrainView {
    /// Slot 42 `GetBoundaryLotTexture(sides, texture, rotation, flip)`
    /// (§17.7): the rule of `key := ((s0·16 + s1)·16 + s2)·16 + s3` (int32,
    /// no masking), else the rule of key 0. `None` only when the boundary
    /// map is empty (before `Init`, after `Shutdown`), where the original
    /// reads the map's end node *[choice: nothing is written]*.
    pub fn boundary_lot_texture(&self, sides: [i32; 4]) -> Option<BoundaryRule> {
        if !self.boundary_ready {
            return None;
        }
        let key = sides[0]
            .wrapping_mul(16)
            .wrapping_add(sides[1])
            .wrapping_mul(16)
            .wrapping_add(sides[2])
            .wrapping_mul(16)
            .wrapping_add(sides[3]);
        rule(key).or_else(|| rule(0))
    }

    /// Slot 43 `SetEmptyLotBaseTexture(lot)` (§17.7). Zones 0, 11, 12 and
    /// 15 lose their base-texture occupants (`RemoveLotBaseTextures`); every
    /// other zone gets one boundary record per cell of the lot, handed to the
    /// lot's base-texture occupant (created when missing) in one
    /// `SetTextureSpecification` call, even with no record.
    pub fn set_empty_lot_base_texture(&self, host: &dyn LotDeveloperHost, lot: u64) {
        let zone = host.lot_zone_type(lot);
        if zone_has_no_empty_texture(zone) {
            self.remove_lot_base_textures(host, lot);
            return;
        }
        let colour = host.zone_drag_color(zone);
        let alpha = host.zone_display_alpha();
        let extra = empty_lot_extra(colour, alpha);
        let (cells_x, cells_z) = host.city_cell_counts();
        let (max_x, max_z) = (cells_x.wrapping_sub(1), cells_z.wrapping_sub(1));
        let r = host.lot_bounding_rect(lot);
        let boundaries = !(matches!(zone, 7..=9) && host.lot_size(lot) == (1, 1));
        let mut records: Vec<TextureRecord> = Vec::new();
        let mut x = i64::from(r.x1);
        while x <= i64::from(r.x2) {
            let mut z = i64::from(r.z1);
            while z <= i64::from(r.z2) {
                let (cx, cz) = (x as i32, z as i32);
                z += 1;
                if cx >= self.cell_count_x || cz >= self.cell_count_z {
                    continue;
                }
                let mut sides = [0i32; 4];
                if boundaries {
                    // Neighbour order (x, z−1) → s0, (x, z+1) → s2,
                    // (x−1, z) → s3, (x+1, z) → s1.
                    let order: [(i32, i32, usize); 4] = [
                        (cx, cz.wrapping_sub(1), 0),
                        (cx, cz.wrapping_add(1), 2),
                        (cx.wrapping_sub(1), cz, 3),
                        (cx.wrapping_add(1), cz, 1),
                    ];
                    for (nx, nz, side) in order {
                        let inside = (0..=max_x).contains(&nx) && (0..=max_z).contains(&nz);
                        let n = if inside { host.get_lot(nx, nz, false) } else { None };
                        sides[side] = if n == Some(lot) {
                            0
                        } else if host.lot_facing(lot) == side as i32 {
                            2
                        } else {
                            1
                        };
                    }
                }
                // [choice] With no boundary map (before `Init`) the record
                // carries texture 0, rotation 0, no flip.
                let rule = self.boundary_lot_texture(sides).unwrap_or(BoundaryRule {
                    texture: 0,
                    rotation: 0,
                    flip: false,
                });
                records.push(TextureRecord {
                    texture: rule.texture,
                    x: cx,
                    z: cz,
                    orientation: rule.rotation,
                    byte3: u8::from(rule.flip),
                    extra,
                    frame: 0xFF,
                });
            }
            x += 1;
        }
        let Some(occ) = host.base_texture_occupant(lot, true) else {
            return;
        };
        host.base_texture_set(occ, &records);
        if host.occupant_is_flag_set(occ, 1) {
            host.occupant_changed(occ);
            host.occupant_post_message(occ, msg::OCCUPANT_CHANGED);
        } else {
            host.occupant_insert(occ);
        }
    }

    /// `RemoveLotBaseTextures(lot)` (`0x006CAAF0`, §17.7): every
    /// base-texture occupant on the lot's cells is removed (when inserted)
    /// and shut down.
    pub fn remove_lot_base_textures(&self, host: &dyn LotDeveloperHost, lot: u64) {
        let r = host.lot_bounding_rect(lot);
        for o in host.occupants_in_cells(r, OCCUPANT_TYPE_BASE_TEXTURE) {
            if host.occupant_is_flag_set(o, 1) {
                host.occupant_remove(o, 0, 0);
            }
            host.occupant_shutdown(o);
        }
    }
}
