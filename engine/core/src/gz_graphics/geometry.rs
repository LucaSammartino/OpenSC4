// SPDX-License-Identifier: GPL-3.0-or-later
//! The settled generic geometry routes in gz-graphics §§3.2.1–3.2.5.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelSpan {
    pub y: i32,
    pub left: i32,
    pub right: i32,
}

fn emit(engine: &mut dyn RenderEngine, clip: Rect, y: i32, left: i32, right: i32) -> Result<()> {
    let (left, right) = (left.max(clip.0), right.min(clip.2));
    if y >= clip.1 && y < clip.3 && left < right {
        engine.spans(&[PixelSpan { y, left, right }])?;
    }
    Ok(())
}

/// Stored binary32 vertices, separately rounded row limits, and signed packed x.
pub fn polygon_spans(points: &[Point], clip: Rect) -> Vec<PixelSpan> {
    if points.len() < 3 {
        return Vec::new();
    }
    let vertices: Vec<_> = points
        .iter()
        .map(|p| {
            (
                (p[0] as f64 - 0.5) as f32,
                (p[1] as f64 - 0.5) as f32,
                (p[1] as f64 - 0.5).ceil() as i32,
            )
        })
        .collect();
    let mut first = 0;
    for i in 1..points.len() {
        if points[i][1] < points[first][1] {
            first = i;
        }
    }
    struct Chain {
        index: usize,
        visited: usize,
        end: i32,
        x: i32,
        step: i32,
    }
    fn advance(c: &mut Chain, direction: usize, row: i32, v: &[(f32, f32, i32)]) -> bool {
        while c.end == row {
            if c.visited == v.len() {
                return false;
            }
            let a = v[c.index];
            c.index = (c.index + direction) % v.len();
            c.visited += 1;
            let b = v[c.index];
            c.end = b.2;
            if c.end < row {
                return false;
            }
            if c.end > row {
                let slope = (b.0 as f64 - a.0 as f64) / (b.1 as f64 - a.1 as f64);
                c.x = ((((row as f64 - a.1 as f64) * slope + a.0 as f64) * 65536. + 0.5).floor() as i32)
                    .wrapping_add(65535);
                c.step = (slope * 65536. + 0.5).floor() as i32;
            }
        }
        true
    }
    let start = vertices[first].2;
    let chain = || Chain {
        index: first,
        visited: 0,
        end: start,
        x: 0,
        step: 0,
    };
    let (mut left, mut right) = (chain(), chain());
    let mut spans = Vec::new();
    let mut y = start;
    loop {
        if !advance(&mut left, 1, y, &vertices) || !advance(&mut right, vertices.len() - 1, y, &vertices) {
            break;
        }
        let x0 = i32::from((left.x >> 16) as i16).max(clip.0);
        let x1 = i32::from((right.x >> 16) as i16).min(clip.2);
        if y >= clip.1 && y < clip.3 && x0 < x1 {
            spans.push(PixelSpan { y, left: x0, right: x1 });
        }
        left.x = left.x.wrapping_add(left.step);
        right.x = right.x.wrapping_add(right.step);
        let Some(next) = y.checked_add(1) else { break };
        y = next;
    }
    spans
}

pub fn fixed_point(p: Point) -> [i32; 2] {
    p.map(|v| (v as f64 * 256. + 0.5).floor() as i32)
}

/// Exact forward differences, including the specified extra segment beyond P3.
pub fn cubic_segments(points: &[Point]) -> Vec<([i32; 2], [i32; 2])> {
    let mut segments = Vec::new();
    for p in points.chunks_exact(4) {
        let p: Vec<_> = p.iter().copied().map(fixed_point).collect();
        let coefficients: Vec<_> = (0..2)
            .map(|i| {
                let a = p[3][i]
                    .wrapping_sub(p[2][i].wrapping_mul(3))
                    .wrapping_add(p[1][i].wrapping_mul(3))
                    .wrapping_sub(p[0][i]);
                let b = p[2][i]
                    .wrapping_sub(p[1][i].wrapping_mul(2))
                    .wrapping_add(p[0][i])
                    .wrapping_mul(3);
                let c = p[1][i].wrapping_sub(p[0][i]).wrapping_mul(3);
                (a, b, c)
            })
            .collect();
        let bound = coefficients
            .iter()
            .flat_map(|&(a, b, _)| {
                [
                    b.wrapping_mul(2).unsigned_abs(),
                    b.wrapping_add(a.wrapping_mul(3)).wrapping_mul(2).unsigned_abs(),
                ]
            })
            .max()
            .unwrap_or(0);
        let mut n = 1_i64;
        while n < 1024 && 4 * n * n < i64::from(bound) {
            n *= 2;
        }
        let (h1, h2, h3) = ((1_i64 << 30) / n, (1_i64 << 30) / (n * n), (1_i64 << 30) / (n * n * n));
        let mut position = p[0].map(|v| (i64::from(v) << 30) + (1 << 29));
        let mut d1 = [0_i64; 2];
        let mut d2 = [0_i64; 2];
        let mut d3 = [0_i64; 2];
        for i in 0..2 {
            let (a, b, c) = coefficients[i];
            let (a, b, c) = (i64::from(a), i64::from(b), i64::from(c));
            d1[i] = c * h1 + b * h2 + a * h3;
            d2[i] = 2 * b * h2 + 6 * a * h3;
            d3[i] = 6 * a * h3;
        }
        let mut previous = p[0];
        for _ in 0..=n {
            for i in 0..2 {
                position[i] = position[i].wrapping_add(d1[i]);
                d1[i] = d1[i].wrapping_add(d2[i]);
                d2[i] = d2[i].wrapping_add(d3[i]);
            }
            let next = position.map(|v| (v >> 30) as i32);
            segments.push((previous, next));
            previous = next;
        }
    }
    segments
}

fn line(engine: &mut dyn RenderEngine, a: [i32; 2], b: [i32; 2], clip: Rect) -> Result<()> {
    if a == b {
        return Ok(());
    }
    // Only the directional horizontal integer-centre contract is settled.
    // Diagonal ties, clipping phase and thin-clip behavior remain GZG1.
    if a[1] != b[1]
        || a[0] & 255 != 128
        || b[0] & 255 != 128
        || a[1] & 255 != 128
        || clip.width() < 2
        || clip.height() < 2
    {
        return Err(Error::Unsupported("GZG1"));
    }
    let (start, end) = if a[0] < b[0] {
        (a[0] >> 8, b[0] >> 8)
    } else {
        ((b[0] >> 8) + 1, (a[0] >> 8) + 1)
    };
    for x in start..end {
        emit(engine, clip, a[1] >> 8, x, x + 1)?;
    }
    Ok(())
}

fn submit(engine: &mut dyn RenderEngine, kind: u32, p: &[Point], state: &EngineState) -> Result<()> {
    let result = engine.draw(kind, p)?;
    match result {
        1 => execute(engine, GeometryRoute::Tessellate, kind, p, state),
        2 => execute(engine, GeometryRoute::Rasterize, kind, p, state),
        _ => Ok(()),
    }
}

fn ellipse_cubics(points: &[Point]) -> Vec<Point> {
    let mut out = Vec::new();
    for p in points.chunks_exact(2) {
        let cx = (p[0][0] as f64 + p[1][0] as f64) * 0.5;
        let cy = (p[0][1] as f64 + p[1][1] as f64) * 0.5;
        let rx = ((p[1][0] as f64 - p[0][0] as f64).abs() * 0.5) as f32;
        let ry = ((p[1][1] as f64 - p[0][1] as f64).abs() * 0.5) as f32;
        let k = 0.552_284_8_f32;
        let (kx, ky) = ((k as f64 * rx as f64) as f32, (k as f64 * ry as f64) as f32);
        for [x, y] in [
            [rx, 0.],
            [rx, ky],
            [kx, ry],
            [0., ry],
            [0., ry],
            [-kx, ry],
            [-rx, ky],
            [-rx, 0.],
            [-rx, 0.],
            [-rx, -ky],
            [-kx, -ry],
            [0., -ry],
            [0., -ry],
            [kx, -ry],
            [rx, -ky],
            [rx, 0.],
        ] {
            out.push([(cx + x as f64) as f32, (cy + y as f64) as f32]);
        }
    }
    out
}

pub(super) fn execute(
    engine: &mut dyn RenderEngine,
    route: GeometryRoute,
    kind: u32,
    points: &[Point],
    state: &EngineState,
) -> Result<()> {
    if route == GeometryRoute::Outline {
        return submit(engine, kind, points, state);
    }
    if kind == 4 && (route == GeometryRoute::Rasterize || state.aa != 0) {
        return submit(engine, 2, &ellipse_cubics(points), state);
    }
    if route == GeometryRoute::Tessellate {
        match kind {
            2 => {
                for p in points.chunks_exact(4) {
                    let mut previous = p[0];
                    let mut quads = Vec::new();
                    for i in 1..=16 {
                        let t = i as f64 / 16.;
                        let u = 1. - t;
                        let next = [0, 1].map(|axis| {
                            (u * u * u * p[0][axis] as f64
                                + 3. * u * u * t * p[1][axis] as f64
                                + 3. * u * t * t * p[2][axis] as f64
                                + t * t * t * p[3][axis] as f64) as f32
                        });
                        let dx = next[0] as f64 - previous[0] as f64;
                        let dy = next[1] as f64 - previous[1] as f64;
                        let length = dx.hypot(dy);
                        // Literal stroke quad: perpendicular half-width, no joins.
                        // GZG7 asks for native corner/degenerate/rounding details.
                        let scale = if length == 0. {
                            0.
                        } else {
                            state.thickness as f64 / (2. * length)
                        };
                        let (nx, ny) = (-dy * scale, dx * scale);
                        quads.extend_from_slice(&[
                            [(previous[0] as f64 + nx) as f32, (previous[1] as f64 + ny) as f32],
                            [(next[0] as f64 + nx) as f32, (next[1] as f64 + ny) as f32],
                            [(next[0] as f64 - nx) as f32, (next[1] as f64 - ny) as f32],
                            [(previous[0] as f64 - nx) as f32, (previous[1] as f64 - ny) as f32],
                        ]);
                        previous = next;
                    }
                    submit(engine, 7, &quads, state)?;
                }
                return Ok(());
            }
            6 => return Ok(()),
            7 => {
                for p in points.chunks_exact(4) {
                    execute(engine, route, 8, p, state)?;
                }
                return Ok(());
            }
            8 => {
                if points.len() < 3 {
                    return Err(Error::InvalidInput);
                }
                let mut fan = Vec::new();
                for i in 1..points.len() - 1 {
                    fan.extend_from_slice(&[points[0], points[i], points[i + 1]]);
                }
                engine.draw(6, &fan)?;
                return Ok(());
            }
            5 => {
                if points.len() < 2 {
                    return Ok(());
                }
                let a = points[0];
                let b = points[1];
                let radii = [
                    ((a[0] as f64 - b[0] as f64).abs() * 0.5) as f32,
                    ((a[1] as f64 - b[1] as f64).abs() * 0.5) as f32,
                ];
                let centre = [(a[0] as f64 + b[0] as f64) * 0.5, (a[1] as f64 + b[1] as f64) * 0.5];
                let r = radii[0].max(radii[1]);
                let n = if r <= 1. { 1 } else { (r as f64).sqrt().ceil() as usize };
                // Spec's original constant, intentionally not mathematical TAU.
                #[allow(clippy::approx_constant)]
                let step = (6.28318_f64 / (4 * n) as f64) as f32;
                for _ in points.chunks_exact(2) {
                    let mut angle = -step;
                    let mut polygon = Vec::new();
                    for _ in 0..4 * n {
                        polygon.push([
                            (centre[0] + radii[0] as f64 * (angle as f64).cos()) as f32,
                            (centre[1] + radii[1] as f64 * (angle as f64).sin()) as f32,
                        ]);
                        angle = (angle as f64 - step as f64) as f32;
                    }
                    submit(engine, 8, &polygon, state)?;
                }
                return Ok(());
            }
            // Non-AA outline geometry and stroke-quad corners remain GZG1.
            _ => return Err(Error::Unsupported("GZG1")),
        }
    }
    match kind {
        1 => {
            for p in points.chunks_exact(2) {
                line(engine, fixed_point(p[0]), fixed_point(p[1]), state.clip)?;
            }
        }
        2 => {
            for (a, b) in cubic_segments(points) {
                line(engine, a, b, state.clip)?;
            }
        }
        5 => {
            for p in points.chunks_exact(2) {
                let left = (p[0][0] as f64).min(p[1][0] as f64);
                let right = (p[0][0] as f64).max(p[1][0] as f64);
                let top = (p[0][1] as f64).min(p[1][1] as f64);
                let bottom = (p[0][1] as f64).max(p[1][1] as f64);
                let (rx, ry) = ((right - left) * 0.5, (bottom - top) * 0.5);
                if rx == 0. || ry == 0. {
                    continue;
                }
                let (cx, cy) = (((left + right) * 0.5) as f32, ((top + bottom) * 0.5) as f32);
                let ry2 = (ry * ry) as f32;
                let ratio = (rx / ry) as f32;
                for y in ((top - 0.5).ceil() as i32).max(state.clip.1)..((bottom - 0.5).ceil() as i32).min(state.clip.3)
                {
                    let d = y as f64 + 0.5 - cy as f64;
                    let q = ry2 as f64 - d * d;
                    if q <= 0. {
                        continue;
                    }
                    let half = q.sqrt() * ratio as f64;
                    emit(
                        engine,
                        state.clip,
                        y,
                        (cx as f64 - half - 0.5).ceil() as i32,
                        (cx as f64 + half - 0.5).ceil() as i32,
                    )?;
                }
            }
        }
        6 | 7 | 8 => {
            let size = match kind {
                6 => 3,
                7 => 4,
                _ => points.len().max(1),
            };
            for p in points.chunks_exact(size) {
                engine.spans(&polygon_spans(p, state.clip))?;
            }
        }
        // Point and rectangle coverage beyond their specified input conversion
        // is not described in this spec; retain the GZG1 coverage boundary.
        _ => return Err(Error::Unsupported("GZG1")),
    }
    Ok(())
}
