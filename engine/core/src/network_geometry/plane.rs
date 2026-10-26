// SPDX-License-Identifier: GPL-3.0-or-later
//! Ordinary surface-run planes, network-geometry section 9.8.

use super::params::{is_underground, CELL_SIZE, POWER_LINE};
use super::{cell_key, CellOccupant, GeometryHost, GeometryTerrain, NetworkGeometry, StepKind};
use crate::x87::r24;

fn add(a: f32, b: f32) -> f32 {
    r24(f64::from(a) + f64::from(b)) as f32
}
fn sub(a: f32, b: f32) -> f32 {
    r24(f64::from(a) - f64::from(b)) as f32
}
fn mul(a: f32, b: f32) -> f32 {
    r24(f64::from(a) * f64::from(b)) as f32
}

fn coefficients(p: [[f32; 3]; 3]) -> [f32; 4] {
    let [[x1, y1, z1], [x2, y2, z2], [x3, y3, z3]] = p;
    let a = add(add(mul(sub(z3, z1), y2), mul(sub(z2, z3), y1)), mul(sub(z1, z2), y3));
    let b = add(add(mul(sub(x2, x3), z1), mul(sub(x3, x1), z2)), mul(sub(x1, x2), z3));
    let c = add(add(mul(sub(y1, y2), x3), mul(sub(y3, y1), x2)), mul(sub(y2, y3), x1));
    let d = sub(
        sub(
            -mul(sub(mul(y2, z3), mul(z2, y3)), x1),
            mul(sub(mul(z1, y3), mul(y1, z3)), x2),
        ),
        mul(sub(mul(y1, z2), mul(z1, y2)), x3),
    );
    [a, b, c, d]
}

fn held(g: &NetworkGeometry, vertex: (i32, i32), t: &dyn GeometryTerrain, host: &dyn GeometryHost) -> bool {
    let (nx, nz) = t.cells();
    let mask = if g.run.type_id == POWER_LINE {
        1 << POWER_LINE
    } else {
        0x1F4F
    };
    let (x, z) = vertex;
    [(x, z), (x, z - 1), (x - 1, z - 1), (x - 1, z)]
        .into_iter()
        .any(|(x, z)| {
            x >= 0
            && z >= 0
            && x < nx as i32
            && z < nz as i32
            && g.smoothing.cells.binary_search(&cell_key(x, z)).is_err()
            && host.cell_occupants(x, z).into_iter().any(|o| {
                matches!(o, CellOccupant::Network { type_id, .. } if type_id < 13 && mask & (1 << type_id) != 0)
            })
        })
}

fn anchor_height(g: &NetworkGeometry, q: [(i32, i32); 2], t: &dyn GeometryTerrain, host: &dyn GeometryHost) -> f32 {
    let stride = t.cells().0 as i32 + 1;
    let h = q.map(|(x, z)| t.vertex_altitude(z * stride + x));
    if held(g, q[0], t, host) {
        h[0]
    } else if held(g, q[1], t, host) {
        h[1]
    } else {
        mul(add(h[1], h[0]), 0.5)
    }
}

/// Structure elevations still belong to the bridge/tunnel host. The empty
/// structure-list path builds its plane even for an isolated one-cell run.
pub(super) fn apply(g: &mut NetworkGeometry, t: &dyn GeometryTerrain, host: &dyn GeometryHost) {
    if is_underground(g.run.type_id) || g.valid == 0 || !g.run.bridges.is_empty() || !g.run.tunnels.is_empty() {
        return;
    }
    let last = g.valid as usize - 1;
    let Some(end_step) = g.run.steps.get(last) else { return };
    let d = if g.run.straight_count > 0 {
        g.run.pen_direction
    } else {
        g.run.end_direction
    } as usize;
    let opposite = (d + 4) & 7;
    // The two direction-indexed edge tables from section 6.1, as coordinates.
    const Q1: [(i32, i32); 8] = [(1, 1), (0, 1), (0, 1), (0, 0), (0, 0), (1, 0), (1, 0), (1, 1)];
    const Q2: [(i32, i32); 8] = [(1, 0), (1, 0), (1, 1), (1, 1), (0, 1), (0, 1), (0, 0), (0, 0)];
    let edge =
        |cell: (i32, i32), direction: usize| [Q1[direction], Q2[direction]].map(|(x, z)| (cell.0 + x, cell.1 + z));
    let near = edge(g.run.cells[g.run.steps[0].first as usize], d);
    let far = edge(g.run.cells[end_step.first as usize], opposite);
    let ha = anchor_height(g, near, t, host);
    let hb = anchor_height(g, far, t, host);
    let point = |(x, z): (i32, i32), h| [mul(x as u32 as f32, CELL_SIZE), h, mul(z as u32 as f32, CELL_SIZE)];
    let [a, b, c, d] = if last == 0 && g.run.straight_count == 0 {
        [0.0, 1.0, 0.0, -ha]
    } else {
        coefficients([point(near[0], ha), point(near[1], ha), point(far[0], hb)])
    };
    if b == 0.0 {
        return;
    }
    let m = r24(-1.0 / f64::from(b)) as f32;
    let tolerance = g.info(g.run.type_id).params.plane_tolerance;
    let stride = t.cells().0 as i32 + 1;
    for step in g
        .run
        .steps
        .iter()
        .take(g.valid as usize)
        .filter(|s| s.kind == StepKind::Regular)
    {
        for index in step.first..=step.last {
            let (x, z) = g.run.cells[index as usize];
            let Some(ci) = g.cells.get_mut(&cell_key(x, z)) else {
                continue;
            };
            for k in 0..4 {
                let v = ci.vertices[k];
                let tx = mul(mul((v % stride) as u32 as f32, CELL_SIZE), a);
                let tz = mul(mul((v / stride) as u32 as f32, CELL_SIZE), c);
                let h = mul(add(add(tx, tz), d), m);
                let current = ci.corners[k];
                let lo = sub(h, tolerance).min(current);
                let hi = add(h, tolerance).max(current);
                let range = &mut ci.allowed[k];
                range.0 = range.0.max(lo);
                range.1 = range.1.min(hi);
                if range.1 < range.0 && sub(range.0, range.1) <= 0.01 {
                    range.1 = range.0;
                }
                ci.preferred[k] = (h.min(current), h.max(current));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tempelhof_coefficients_keep_documented_rounding() {
        let p = [
            [1376.0, 292.0509, 64.0],
            [1392.0, 292.0509, 64.0],
            [1392.0, 290.46765, 48.0],
        ];
        assert_eq!(coefficients(p), [0.0, 256.0, -25.332031, -73144.0]);
    }
}
