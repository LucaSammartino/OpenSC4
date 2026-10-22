// SPDX-License-Identifier: GPL-3.0-or-later
use super::register::Number;
use super::*;

/// Raw model-frame fields (§5.8); rows of R, not a precomputed inverse.
#[derive(Clone, Copy, Debug)]
pub struct ProjectionTransform {
    pub linear: [[f32; 3]; 3],
    pub translation: Vector,
    pub scale: f32,
    pub flags: u8,
}

fn basis(direction: [Number; 3]) -> [[Number; 3]; 3] {
    let [x, y, z] = direction;
    let zero = Number::new(0.0, x.precision());
    let one = Number::new(1.0, x.precision());
    let h = (x * x + (-z) * (-z)).sqrt();
    if -h < zero {
        let k = one / -h;
        let a = [(-z * k).q(), (zero * k).q(), (x * k).q()];
        let b = [
            (y * a[2] - z * a[1]).q(),
            (z * a[0] - x * a[2]).q(),
            (x * a[1] - y * a[0]).q(),
        ];
        [a, b, direction]
    } else {
        [[zero, zero, one], [one, zero, zero], direction]
    }
}

/// LT10's mixed register/memory sums and cofactor solve, not a generic fit.
fn fit(points: &[[Number; 2]], uvs: &[[f32; 2]], p: RegisterPrecision) -> Option<[[Number; 3]; 2]> {
    let r = |x| Number::new(x, p);
    let (mut x2, mut xye, mut xe, mut ye, mut n) = (r(0.0), r(0.0), r(0.0), r(0.0), r(0.0));
    let (mut xyf, mut y2, mut xf, mut yf) = (r(0.0), r(0.0), r(0.0), r(0.0));
    let mut rhs = [[r(0.0); 3]; 2];
    for ([a, b], uv) in points.iter().copied().zip(uvs) {
        x2 = x2 + (a * a).q();
        xye = xye + (a * b).q();
        xe = xe + a;
        ye = ye + b;
        n = n + r(1.0);
        xyf = ((a * b).q() + xyf).q();
        y2 = ((b * b).q() + y2).q();
        xf = (a + xf).q();
        yf = (b + yf).q();
        for (sums, w) in rhs.iter_mut().zip(uv.iter().copied()) {
            let w = r(w);
            sums[0] = (a * w + sums[0]).q();
            sums[1] = ((b * w).q() + sums[1]).q();
            sums[2] = (w + sums[2]).q();
        }
    }
    let c = [
        [
            (y2 * n - ye * yf).q(),
            (ye * xf - n * xyf).q(),
            (yf * xyf - y2 * xf).q(),
        ],
        [
            (xe * yf - n * xye).q(),
            (x2 * n - xe * xf).q(),
            (xye * xf - x2 * yf).q(),
        ],
        [
            (ye * xye - xe * y2).q(),
            (xe * xyf - x2 * ye).q(),
            (x2 * y2 - xye * xyf).q(),
        ],
    ];
    let determinant = (c[0][2] * xe + c[0][1] * xye) + c[0][0] * x2;
    // Unordered proceeds; no finite/absolute-value/singular-transform filter.
    if determinant < Number::double(0.01, p) {
        return None;
    }
    let reciprocal = r(1.0) / determinant;
    let k0 = c.map(|row| row[0] * reciprocal);
    let k1 = c.map(|row| (row[1] * reciprocal).q());
    let k2 = c.map(|row| (row[2] * reciprocal).q());
    let mut out = [[r(0.0); 3]; 2];
    for (index, (out, [wa, wb, w0])) in out.iter_mut().zip(rhs).enumerate() {
        *out = [
            (k0[2] * w0 + k0[1] * wb) + k0[0] * wa,
            (k1[0] * wa + k1[2] * w0) + k1[1] * wb,
            ((k2[0] * wa + k2[2] * w0) + k2[1] * wb).q(),
        ];
        if index == 0 {
            out[0] = out[0].q();
            out[1] = out[1].q();
        }
    }
    Some(out)
}

/// Independently usable plane subproblem, with the same native spills as slot 17.
pub fn fit_shadow_plane(points: &[[f32; 2]], uvs: &[[f32; 2]], precision: RegisterPrecision) -> Result<[[f32; 3]; 2]> {
    if points.len() != uvs.len() {
        return Err(LightingError::BufferTooShort);
    }
    let points: Vec<_> = points.iter().map(|p| p.map(|v| Number::new(v, precision))).collect();
    fit(&points, uvs, precision)
        .map(|rows| rows.map(|row| row.map(Number::store)))
        .ok_or(LightingError::DegenerateProjection)
}

impl LightingManager {
    /// §5.8. Explicit inherited precision (LT12); nearest-even, masked exceptions.
    /// Writes all sixteen column-major floats only after success.
    pub fn calc_shadow_projection(
        &self,
        points: &[Vector],
        uvs: &[[f32; 2]],
        transform: ProjectionTransform,
        inout: &mut [f32; 16],
    ) -> Result<bool> {
        if points.len() != uvs.len() {
            return Err(LightingError::BufferTooShort);
        }
        let precision = self.register_precision;
        let r = |x| Number::new(x, precision);
        let mut a = transform.linear.map(|row| row.map(r));
        if transform.flags & 4 != 0 {
            a = std::array::from_fn(|i| std::array::from_fn(|j| a[j][i]));
        }
        let [b0, b1, _] = model_basis(self.shadow_direction, a, precision);
        let plane: Vec<_> = points
            .iter()
            .map(|point| {
                let [x, y, z] = point.map(r);
                [
                    ((b0[0] * x + b0[1] * y) + b0[2] * z).q(),
                    ((b1[2] * z + b1[1] * y) + b1[0] * x).q(),
                ]
            })
            .collect();
        let Some([[ua, ub, u0], [va, vb, v0]]) = fit(&plane, uvs, precision) else {
            return Ok(false);
        };
        let u = [
            (ua * b0[0] + ub * b1[0]).q(),
            ((ua * b0[1]).q() + ub * b1[1]).q(),
            ((ua * b0[2]).q() + (ub * b1[2]).q()).q(),
        ];
        let v = [
            (va * b0[0] + (vb * b1[0]).q()).q(),
            (va * b0[1] + (vb * b1[1]).q()).q(),
            ((va * b0[2]).q() + vb * b1[2]).q(),
        ];
        let mut rows = [
            [u[0], u[1], u[2], u0],
            [v[0], v[1], v[2], v0],
            [r(0.0), r(0.0), r(1.0), r(0.0)],
            [r(0.0), r(0.0), r(0.0), r(1.0)],
        ];
        if transform.flags != 0 {
            let mut t = transform.translation.map(|v| r(-v));
            if transform.flags & 4 != 0 {
                t = a.map(|row| ((row[0] * t[0] + row[2] * t[2]) + row[1] * t[1]).q());
            }
            // A zero scale is consumed only on bit 2's reciprocal path.
            let s = if transform.flags & 2 != 0 {
                (r(1.0) / r(transform.scale)).q()
            } else {
                r(transform.scale)
            };
            if transform.flags & 2 != 0 {
                t = t.map(|v| (v * s).q());
            }
            for j in 0..3 {
                rows[0][j] = ((a[2][j] * u[2] + a[1][j] * u[1]) + u[0] * a[0][j]).q();
                rows[1][j] = ((a[2][j] * v[2] + a[1][j] * v[1]) + v[0] * a[0][j]).q();
                let w = (a[1][j] + a[0][j]) * r(0.0) + a[2][j];
                rows[2][j] = if j == 1 { w } else { w.q() };
            }
            rows[0][3] = ((((t[2] * u[2] + t[1] * u[1]) + u[0] * t[0]).q()) + u0).q();
            rows[1][3] = (((t[2] * v[2] + t[1] * v[1]) + v[0] * t[0]) + v0).q();
            rows[2][3] = ((t[1] + t[0]) * r(0.0) + t[2]).q();
            if transform.flags & 2 != 0 {
                for row in rows.iter_mut().take(3) {
                    for value in row.iter_mut().take(3) {
                        *value = (*value * s).q();
                    }
                }
            }
        }
        *inout = std::array::from_fn(|index| rows[index % 4][index / 4].store());
        Ok(true)
    }

    /// §5.7. Only raw cached terrain heights are consumed; no host rectangle fit.
    pub fn calc_shadow_terrain_rect(&self, bounds: Bounds, out: &mut Rect) -> Result<bool> {
        let precision = self.register_precision;
        let r = |x| Number::new(x, precision);
        let floor = |x: f32| native_endpoint(r(x) * r(1.0 / 16.0), false, precision);
        let ceil = |x: f32| native_endpoint(r(x) * r(1.0 / 16.0), true, precision);
        let mut edges = [
            floor(bounds.min[0]).max(0),
            floor(bounds.min[2]).max(0),
            ceil(bounds.max[0]).min(self.cells.0 as i32),
            ceil(bounds.max[2]).min(self.cells.1 as i32),
        ];
        if edges[0] > edges[2] || edges[1] > edges[3] {
            edges = [0; 4];
        }
        let [dx, dy, dz] = self.shadow_direction.map(r);
        let slopes = [(dy / dx).q(), (dy / dz).q()];
        let (first, probes, last, last_probe) = if dx > r(0.0) {
            if dz > r(0.0) {
                (3, [0, 2], 2, 3)
            } else {
                (2, [3, 1], 1, 2)
            }
        } else if dz > r(0.0) {
            (0, [1, 3], 3, 0)
        } else {
            (1, [2, 0], 0, 1)
        };
        for (moving, probes) in [(first, &probes[..]), (last, &[last_probe][..])] {
            let axis = moving % 2;
            let increasing = moving >= 2;
            let face = if increasing {
                bounds.max[axis * 2]
            } else {
                bounds.min[axis * 2]
            };
            let mut height = r(bounds.max[1]) + (r(edges[moving] as f32) * r(16.0) - r(face)) * slopes[axis];
            let step = r(16.0) * slopes[axis];
            for &probe in probes {
                loop {
                    let limit = if axis == 0 { self.cells.0 } else { self.cells.1 } as i32;
                    if if increasing {
                        edges[moving] >= limit
                    } else {
                        edges[moving] <= 0
                    } {
                        break;
                    }
                    let (x, z) = if axis == 0 {
                        (edges[moving], edges[probe])
                    } else {
                        (edges[probe], edges[moving])
                    };
                    let index = self.grid_index(x as usize, z as usize)?;
                    let terrain = *self.heights.get(index).ok_or(LightingError::LocationOutsideGrid)?;
                    if height.partial_cmp(&r(terrain)) != Some(std::cmp::Ordering::Greater) {
                        break;
                    }
                    edges[moving] += if increasing { 1 } else { -1 };
                    height = if increasing { height + step } else { height - step };
                }
            }
        }
        *out = Rect {
            min_x: edges[0],
            min_z: edges[1],
            max_x: edges[2],
            max_z: edges[3],
        };
        Ok(true)
    }
}
fn native_endpoint(q: Number, ceil: bool, p: RegisterPrecision) -> i32 {
    let shifted = q + Number::new(if ceil { 0.5 } else { -0.5 }, p);
    let stored = (shifted + Number::new(12582912.0, p)).store();
    let mut integer = stored.to_bits().wrapping_sub(0x4b400000) as i32;
    // Correct the ties at integral q, without replacing the native bit rounding.
    if ceil && Number::new(integer.wrapping_sub(1) as f32, p) == q {
        integer -= 1;
    }
    if !ceil && Number::new(integer.wrapping_add(1) as f32, p) == q {
        integer += 1;
    }
    integer
}

#[cfg(test)]
pub(super) fn direction_basis(
    direction: Vector,
    transform: ProjectionTransform,
    precision: RegisterPrecision,
) -> [[f32; 3]; 3] {
    let r = |x| Number::new(x, precision);
    let a = std::array::from_fn::<_, 3, _>(|i| {
        std::array::from_fn::<_, 3, _>(|j| {
            r(if transform.flags & 4 != 0 {
                transform.linear[j][i]
            } else {
                transform.linear[i][j]
            })
        })
    });
    model_basis(direction, a, precision).map(|row| row.map(Number::store))
}

fn model_basis(direction: Vector, a: [[Number; 3]; 3], precision: RegisterPrecision) -> [[Number; 3]; 3] {
    let s = direction.map(|v| Number::new(-v, precision));
    basis(a.map(|row| ((row[0] * s[0] + row[2] * s[2]) + row[1] * s[1]).q()))
}
