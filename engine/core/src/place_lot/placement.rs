// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl PlaceLot {
    pub fn coverage(&self, h: &mut impl Host, point: Vec3, rotation: u8) -> Option<Coverage> {
        let cfg = h.configuration(self.configuration)?;
        Some(cell_coverage(cfg.size, h.terrain(), point, rotation))
    }

    fn demolition(&self, h: &mut impl Host, rect: Rect) -> (bool, i64) {
        h.demolish(
            Demolition {
                commit: false,
                rect,
                privilege: 1,
                flags: if self.coverage_view == 0xF2 { 0x800 } else { 0x2800 },
                option: false,
            },
            demolish_accepts,
        )
    }

    pub fn can_place(&self, h: &mut impl Host, point: Vec3, rotation: u8) -> Validation {
        let mut out = Validation {
            allowed: false,
            coverage: None,
            creation_code: 0,
            cost: 0,
            text: String::new(),
        };
        let Some(coverage) = self.coverage(h, point, rotation) else {
            return out;
        };
        out.coverage = Some(coverage);
        if !coverage.valid {
            return out;
        }
        // -1 is the signed constructor sentinel; all edges are inclusive.
        let p = [coverage.point.x as i32, coverage.point.z as i32];
        let r = self.last_placed.map(|v| v as i32);
        if p[0] >= r[0] && p[1] >= r[1] && p[0] <= r[2] && p[1] <= r[3] {
            return out;
        }
        out.cost = if self.plop_cost == 0 {
            0
        } else {
            self.plop_cost + h.development_cost(coverage.rect)
        };
        let code = h.can_create(coverage.rect, rotation, self.configuration);
        out.creation_code = code;
        if code == 8 {
            let (ok, cost) = self.demolition(h, coverage.rect);
            if ok {
                out.cost += cost;
                out.allowed = true;
            } else {
                out.text = h.localize(0x0A41_9226, 0x6A71_1969);
            }
        } else if (0..=7).contains(&code) {
            let message = CREATION_MESSAGES[code as usize];
            out.text = h.localize(0x0A41_9226, message);
        }
        out
    }

    pub fn street_count(&self, h: &mut impl Host, rect: Rect, rotation: u8) -> i32 {
        let (start_x, start_z, dx, dz, count) = match rotation {
            0 => (rect.x0, rect.z0 - 1, 1, 0, rect.width()),
            1 => (rect.x1 + 1, rect.z0, 0, 1, rect.height()),
            2 => (rect.x0, rect.z1 + 1, 1, 0, rect.width()),
            _ => (rect.x0 - 1, rect.z0, 0, 1, rect.height()),
        };
        let terrain = h.terrain();
        let mut score = 0;
        for i in 0..count {
            let (x, z) = (start_x + i * dx, start_z + i * dz);
            if x < 0 || z < 0 || x >= terrain.width || z >= terrain.height {
                continue;
            }
            let flags = h.network_flags(x, z, 0x449);
            score += if flags & 0x40 != 0 {
                4
            } else if flags & 1 != 0 {
                2
            } else if flags & 0x408 != 0 {
                1
            } else {
                0
            };
        }
        score
    }

    pub fn automatic_facing(&self, h: &mut impl Host, point: Vec3) -> Facing {
        let invalid = Coverage {
            point,
            rect: Rect {
                x0: -1000,
                z0: -1000,
                x1: -1000,
                z1: -1000,
            },
            valid: false,
        };
        let mut choices = [invalid; 4];
        let mut scores = [-8; 4];
        for rotation in 0..4 {
            if let Some(c) = self.coverage(h, point, rotation) {
                choices[usize::from(rotation)] = c;
                if c.valid && h.can_create(c.rect, rotation, self.configuration) == 8 && self.demolition(h, c.rect).0 {
                    scores[usize::from(rotation)] = self.street_count(h, c.rect, rotation);
                }
            }
        }
        let rotation = choose_facing(scores, self.player_rotation);
        Facing {
            rotation,
            coverage: choices[usize::from(rotation)],
            any: scores.iter().any(|&s| s != -8),
            scores,
        }
    }

    pub fn place_lot(&mut self, h: &mut impl Host, point: &mut Vec3, mut rotation: u8, effects: bool) -> Placement {
        let mut out = Placement {
            rect: Rect::INVALID,
            built: false,
        };
        if self.cost == UNKNOWN_COST {
            let validation = self.can_place(h, *point, rotation);
            self.cost = validation.cost;
            self.cursor_text = validation.text;
            if !validation.allowed {
                return out;
            }
        }
        let Some(mut cfg) = h.configuration(self.configuration) else {
            return out;
        };
        let c = if self.toll_booth {
            let picked = pick_toll(h, *point);
            if !picked.coverage.valid {
                return out;
            }
            self.change_configuration(h, picked.configuration);
            let Some(selected) = h.configuration(self.configuration) else {
                return out;
            };
            cfg = selected;
            rotation = picked.rotation;
            self.rotation = rotation;
            picked.coverage
        } else {
            cell_coverage(cfg.size, h.terrain(), *point, rotation)
        };
        *point = c.point;
        if !c.valid {
            return out;
        }
        out.rect = c.rect;
        if h.tutorial_rectangle().is_some_and(|r| r != c.rect) {
            return out;
        }
        if self.cost != 0 && h.total_funds() < self.cost {
            return out;
        }
        if !h
            .demolish(
                Demolition {
                    commit: true,
                    rect: c.rect,
                    privilege: 1,
                    flags: 0x2000,
                    option: false,
                },
                demolish_accepts,
            )
            .0
        {
            return out;
        }
        if !h.place_zone(Zone {
            rect: c.rect,
            kind: 0,
            commit: true,
            skip_funds: self.cost == 0,
            check_slope: true,
            skip_size: true,
            check_lot_inside: false,
            outputs: [None; 3],
        }) {
            return out;
        }
        let info = self
            .previews
            .iter()
            .find(|p| !p.prop)
            .map_or_else(BuildingInfo::default, |p| h.occupant_info(p.token));
        let kind = if info.groups.contains(&0x1508) {
            11
        } else if info.groups.contains(&0x1509) || cfg.water_compatible {
            12
        } else {
            15
        };
        if !h.place_zone(Zone {
            rect: c.rect,
            kind,
            commit: true,
            skip_funds: true,
            check_slope: true,
            skip_size: true,
            check_lot_inside: false,
            outputs: [None; 3],
        }) {
            return out;
        }
        let Some(lot) = h.create_lot(c.rect, rotation, true, cfg.water_compatible, self.configuration) else {
            return out;
        };
        let building = h.construct_default_lot(lot, self.configuration, self.exemplar, cfg.construction);
        if let Some(building) = building {
            if self.cost != 0 {
                h.effect(Effect::Withdraw(self.cost));
            }
            h.effect(Effect::Aura(2, building.position));
            let info = h.occupant_info(building.token);
            let terrain = h.terrain();
            let x = (c.point.x / terrain.cell_width) as i32;
            let z = (c.point.z / terrain.cell_width) as i32;
            if info.water_production {
                h.effect(Effect::InsertConnection(Connection::Pipe, x, z));
            }
            if info.subway_station {
                h.effect(Effect::InsertConnection(Connection::Subway, x, z));
            }
            out.built = true;
        } else {
            // Compatibility quirk: refund despite no preceding withdrawal.
            h.effect(Effect::Deposit(self.cost));
        }
        if effects {
            // Construction may change terrain height. section 3.12 requires the
            // created lot's position, not the pre-construction cursor point.
            let position = h.lot_position(lot);
            let name = match c.rect.width().max(c.rect.height()) {
                1 => "placement_effects_1x1",
                2 => "placement_effects_2x2",
                3 => "placement_effects_3x3",
                _ => "placement_effects_4x4",
            };
            h.effect(Effect::PlacementEffect(name, position));
            h.effect(Effect::Ring(position, [c.rect.width(), c.rect.height()], 0x49D5_B83F));
        }
        out
    }
}

pub const CREATION_MESSAGES: [u32; 8] = [
    0xCA71_1914,
    0xAA71_194E,
    0xAA71_1938,
    0x6A71_193F,
    0xCABC_118F,
    0x6A71_1955,
    0xAA71_195C,
    0x6A71_1944,
];

pub fn choose_facing(scores: [i32; 4], player: u8) -> u8 {
    let mut best = 0;
    for i in 1..4 {
        if scores[i] > scores[best] {
            best = i;
        }
    }
    let own = scores[usize::from(player)];
    // CT24: equality at four more points keeps the player's rotation.
    if (own != 0 || scores[best] < 1) && own >= scores[best] - 4 {
        player
    } else {
        best as u8
    }
}

pub fn cell_coverage(mut size: [i32; 2], terrain: Terrain, mut point: Vec3, rotation: u8) -> Coverage {
    if rotation % 2 == 1 {
        size.swap(0, 1);
    }
    let axis = |s: i32, p: f32| {
        let w = terrain.cell_width;
        if s % 2 != 0 {
            let c = (p / w) as i32;
            ((c as f32 + 0.5) * w, c - s / 2, c + s / 2)
        } else {
            let c = ((p + w / 2.0) / w) as i32;
            (c as f32 * w, c - s / 2, c + s / 2 - 1)
        }
    };
    let (x, x0, x1) = axis(size[0], point.x);
    let (z, z0, z1) = axis(size[1], point.z);
    point.x = x;
    point.z = z;
    Coverage {
        point,
        rect: Rect { x0, z0, x1, z1 },
        valid: x0 >= 0 && z0 >= 0 && x1 < terrain.width && z1 < terrain.height,
    }
}
