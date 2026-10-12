// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Response curves (`docs/specs/foundations.md` §4): piecewise-linear
//! functions built from flat `x0, y0, x1, y1, …` float arrays, clamped at both
//! ends, with the original's segment construction rules.

/// One kept segment: ends at (x, y), starts at its predecessor (px, py).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Segment {
    x: f32,
    y: f32,
    px: f32,
    py: f32,
    slope: f32,
}

/// A response curve (`cSC4ResponseCurve`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResponseCurve {
    segments: Vec<Segment>,
}

impl ResponseCurve {
    /// Builds a curve from a flat `x, y, …` array (§4.1). An odd trailing value
    /// is ignored. Returns `None` when there are no points (the property is
    /// then treated as absent).
    pub fn from_flat(values: &[f32]) -> Option<ResponseCurve> {
        let points: Vec<(f32, f32)> = values.chunks_exact(2).map(|c| (c[0], c[1])).collect();
        if points.is_empty() {
            return None;
        }
        Some(Self::from_points(&points))
    }

    /// Builds a curve from (x, y) points (§4.2).
    pub fn from_points(points: &[(f32, f32)]) -> ResponseCurve {
        let mut segments = Vec::with_capacity(points.len());
        for (i, &(x, y)) in points.iter().enumerate() {
            // The predecessor is always the previous INPUT point (even if dropped).
            let (px, py) = if i == 0 { (-f32::MAX, y) } else { points[i - 1] };
            let dy = y - py;
            let dx = x - px;
            if dy.abs() < 1e-10 {
                segments.push(Segment {
                    x,
                    y,
                    px,
                    py,
                    slope: 0.0,
                });
            } else if dy * 1e-10 < dx {
                segments.push(Segment {
                    x,
                    y,
                    px,
                    py,
                    slope: dy / dx,
                });
            }
            // else: the point is dropped (a vertical step).
        }
        ResponseCurve { segments }
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Evaluates the curve (§4.3): `py + (input − px) · slope` as three `f32`
    /// operations in that order (foundations §18); an empty curve returns 0.0.
    pub fn eval(&self, input: f32) -> f32 {
        let Some(last) = self.segments.len().checked_sub(1) else {
            return 0.0;
        };
        let k = self.segments.iter().position(|s| s.x > input).unwrap_or(last);
        let s = &self.segments[k];
        if k == 0 {
            return self.segments[0].y;
        }
        if k < last || input < self.segments[last].x {
            return s.py + (input - s.px) * s.slope;
        }
        self.segments[last].y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: f32, want: f64) {
        let tol = (want.abs() * 1e-6).max(1e-9);
        assert!((f64::from(got) - want).abs() <= tol, "got {got}, want {want}");
    }

    fn c(v: &[f32]) -> ResponseCurve {
        ResponseCurve::from_flat(v).unwrap()
    }

    #[test]
    fn three_f32_roundings() {
        // foundations §4.3 / §18: (input − px), × slope and py + are each
        // rounded to f32; one rounding of the exact value gives bits 0x3E47EC7F.
        let c = ResponseCurve::from_points(&[(0.0, 0.1), (3.0, 1.1)]);
        assert_eq!(c.eval((2.0f64 / 7.0) as f32).to_bits(), 1_044_900_992);
    }

    #[test]
    fn worked_examples() {
        let police = c(&[0.0, 0.0, 100.0, 100.0, 120.0, 110.0]);
        for (x, y) in [
            (-10.0, 0.0),
            (0.0, 0.0),
            (50.0, 50.0),
            (100.0, 100.0),
            (110.0, 105.0),
            (150.0, 110.0),
        ] {
            close(police.eval(x), y);
        }
        let health = c(&[0.0, 50.0, 20.0, 55.0, 40.0, 60.0, 60.0, 65.0, 200.0, 100.0]);
        for (x, y) in [(10.0, 52.5), (60.0, 65.0), (100.0, 75.0), (250.0, 100.0)] {
            close(health.eval(x), y);
        }
        let crime = c(&[-1.0, 0.1, -0.7, 0.07, 0.0, 0.0, 0.7, -0.07, 1.0, -0.1]);
        for (x, y) in [(-2.0, 0.1f32), (-0.85, 0.085), (0.35, -0.035), (1.5, -0.1)] {
            close(crime.eval(x), f64::from(y));
        }
        let jail = c(&[0.0, 0.0, 80.0, 0.0, 90.0, 10.0, 100.0, 100.0]);
        for (x, y) in [(50.0, 0.0), (85.0, 5.0), (95.0, 55.0), (100.0, 100.0)] {
            close(jail.eval(x), y);
        }
        let air = c(&[0.0, 0.0, 512.0, 0.1, 1024.0, 50.0]);
        close(air.eval(256.0), 0.05);
        assert!((air.eval(768.0) - 25.05).abs() < 1e-3);
        close(air.eval(2000.0), 50.0);
        let flat = c(&[0.0, 0.0, 255.0, 0.0]);
        close(flat.eval(128.0), 0.0);
        let garbage = c(&[0.0, 1e-5, 0.0075, 0.0075, 1.0, 0.075]);
        close(garbage.eval(0.0), 1e-5);
        assert!((garbage.eval(0.5) - 0.040_995).abs() < 1e-6);
        close(garbage.eval(2.0), 0.075);
    }

    #[test]
    fn synthetic_steps() {
        let step = c(&[0.0, 0.0, 5.0, 10.0, 5.0, 20.0, 10.0, 30.0]);
        assert!((step.eval(4.999) - 9.998).abs() < 1e-3);
        close(step.eval(5.0), 20.0);
        close(step.eval(7.5), 25.0);
        let drop = c(&[0.0, 0.0, 5.0, 20.0, 5.0, 10.0, 10.0, 0.0]);
        close(drop.eval(4.9), 19.6);
        close(drop.eval(5.0), 10.0);
        close(drop.eval(7.5), 5.0);
        let ooo = c(&[0.0, 0.0, 10.0, 10.0, 5.0, 20.0, 20.0, 40.0]);
        close(ooo.eval(5.0), 5.0);
        assert!((ooo.eval(10.0) - 26.667).abs() < 1e-3);
        assert!((ooo.eval(15.0) - 33.333).abs() < 1e-3);
        close(ooo.eval(20.0), 40.0);
    }

    #[test]
    fn degenerate_inputs() {
        let one = c(&[3.0, 7.0]);
        for x in [-1e9, 0.0, 3.0, 1e9] {
            close(one.eval(x), 7.0);
        }
        let odd = c(&[0.0, 0.0, 10.0, 10.0, 99.0]); // trailing value ignored
        close(odd.eval(20.0), 10.0);
        assert!(ResponseCurve::from_flat(&[]).is_none());
        assert!(ResponseCurve::from_flat(&[5.0]).is_none());
        assert_eq!(ResponseCurve::default().eval(1.0), 0.0);
    }
}
