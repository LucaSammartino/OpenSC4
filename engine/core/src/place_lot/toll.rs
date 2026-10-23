// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

const SIDES: [(i32, i32); 4] = [(-1, 0), (0, -1), (1, 0), (0, 1)];

/// Every branch is independent: cycles deliberately repeat until depth two.
pub fn seek_compatible(h: &mut impl Host, x: i32, z: i32, depth: u8, kind: u32) -> bool {
    let Some(n) = h.network(x, z, 0x1045, true) else {
        return true;
    };
    if n.flags & (((!(1 << kind)) & 0x1FFF) | 0x400000) != 0 {
        return false;
    }
    depth == 2
        || SIDES
            .iter()
            .enumerate()
            .all(|(i, &(dx, dz))| n.connections & (1 << i) == 0 || seek_compatible(h, x + dx, z + dz, depth + 1, kind))
}

pub fn pick_toll(h: &mut impl Host, point: Vec3) -> TollPick {
    let x = h.position_to_cell(point.x);
    let z = h.position_to_cell(point.z);
    let (world_x, world_z) = h.cell_corner(x, z);
    let mut out = TollPick {
        configuration: 0xEBF9_A440,
        rotation: 2,
        coverage: Coverage {
            point: Vec3 {
                x: world_x,
                y: point.y,
                z: world_z + 8.0,
            },
            rect: Rect {
                x0: x,
                z0: z,
                x1: x,
                z1: z + 1,
            },
            valid: false,
        },
    };
    let Some(n) = h.network(x, z, 0x1045, true) else {
        return out;
    };
    let Some(kind) = [0, 2, 6, 12].into_iter().find(|&k| n.flags & (1 << k) != 0) else {
        return out;
    };
    if n.flags & 0x1F4F & !(1 << kind) != 0 {
        return out;
    }
    let mut connected = 0u8;
    for i in 0..4 {
        match (n.sides >> (8 * i)) & 3 {
            0 => {}
            2 => connected |= 1 << i,
            _ => return out,
        }
    }
    if connected != 5 && connected != 10 {
        return out;
    }
    out.configuration = match kind {
        0 => 0xEBF9_A440,
        2 => 0x8BF9_B356,
        6 => 0xABF9_A45C,
        _ => 0x2BF9_B222,
    };
    let r = &mut out.coverage.rect;
    let p = &mut out.coverage.point;
    if connected == 5 {
        out.rotation = 1;
        r.x1 = x + 1;
        r.z1 = z;
        p.x = world_x + 8.0;
        p.z = world_z;
        if kind != 0 {
            if n.sides & 0x400 != 0 {
                r.z0 -= 1;
                p.z -= 8.0;
            } else {
                r.z1 += 1;
                p.z += 8.0;
            }
        }
    } else if kind != 0 {
        if n.sides & 4 != 0 {
            r.x0 -= 1;
            p.x -= 8.0;
        } else {
            r.x1 += 1;
            p.x += 8.0;
            p.z -= 16.0;
        }
    }
    out.coverage.valid = (r.z0..=r.z1).all(|cz| (r.x0..=r.x1).all(|cx| seek_compatible(h, cx, cz, 0, kind)));
    out
}

impl PlaceLot {
    pub fn change_configuration(&mut self, h: &mut impl Host, id: u32) {
        if self.configuration != id {
            self.release_configuration_effects(h);
            self.configuration = id;
            self.build_preview(h);
        }
    }
}
