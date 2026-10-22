// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// External model transform data, not a same-owner projection result.
#[derive(Clone, Copy, Debug)]
pub struct ProjectionTransform {
    pub linear: [[f32; 3]; 3],
    pub translation: Vector,
    pub flags: u32,
}

/// The fully specified least-squares subproblem of §5.6. The caller supplies
/// coordinates in the perpendicular plane (two components per point), not a fit.
/// Returns u and v as affine functions of those coordinates, in [x,y,constant] order.
/// Uses the literal signed determinant cutoff, not its absolute value.
pub fn fit_shadow_plane(points: &[[f32; 2]], uvs: &[[f32; 2]]) -> Result<[[f32; 3]; 2]> {
    if points.len() != uvs.len() {
        return Err(LightingError::BufferTooShort);
    }
    let mut normal = [[0.0_f64; 3]; 3];
    let mut rhs = [[0.0_f64; 3]; 2];
    for (point, uv) in points.iter().zip(uvs) {
        let p = [f64::from(point[0]), f64::from(point[1]), 1.0];
        for i in 0..3 {
            for j in 0..3 {
                normal[i][j] += p[i] * p[j];
            }
            for k in 0..2 {
                rhs[k][i] += p[i] * f64::from(uv[k]);
            }
        }
    }
    let determinant = det(normal);
    if determinant < 0.01 || !determinant.is_finite() {
        return Err(LightingError::DegenerateProjection);
    }
    let mut coefficients = [[0.0; 3]; 2];
    for k in 0..2 {
        for column in 0..3 {
            let mut replaced = normal;
            for row in 0..3 {
                replaced[row][column] = rhs[k][row];
            }
            coefficients[k][column] = (det(replaced) / determinant) as f32;
        }
    }
    Ok(coefficients)
}
fn det(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

impl LightingManager {
    /// LT10: §5.6 leaves the transform flags/composition inferred and gives no
    /// plane-basis convention. Never delegate these missing own-class operations.
    /// The output is untouched on this explicit failure.
    pub fn calc_shadow_projection(
        &self,
        _points: &[Vector],
        _uvs: &[[f32; 2]],
        _transform: ProjectionTransform,
        _inout: &mut [[f32; 4]; 2],
    ) -> Result<bool> {
        Err(LightingError::ContractGap("LT10"))
    }
    /// LT11: no per-axis traversal, top-ray origins or terrain stop rule are
    /// specified beyond the overview. An exact result cannot be composed from
    /// the box rectangle alone; leave the output untouched until clarified.
    pub fn calc_shadow_terrain_rect(&self, _bounds: Bounds, _out: &mut Rect) -> Result<bool> {
        Err(LightingError::ContractGap("LT11"))
    }
}
