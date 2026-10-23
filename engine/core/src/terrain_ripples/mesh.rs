// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Owner strip traversal, including asymmetric pending outer vertices (§1.3).
use super::{
    arithmetic::{self, Arithmetic, Portable},
    retained::RetainedTuning,
    Error, Point, Zoom,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: u32,
    pub uv: [f32; 2],
}

#[derive(Clone, Debug)]
pub struct Parameters {
    pub step: f32,
    pub spacing_squared: f32,
    pub turn_radians: f32,
    pub advance: f32,
    pub outer: f32,
    pub inner: f32,
    pub wobble: f32,
    pub height: f32,
    pub alpha: [u8; 2],
    pub fade: u32,
}
impl Parameters {
    pub fn from_retained(t: &RetainedTuning, zoom: Zoom) -> Result<Self, Error> {
        let z = zoom.index();
        let f = |a: usize| {
            t.floats[a].get(z).copied().ok_or(Error::UnsupportedTuning(
                super::retained::Unsupported::PostTransformIndices,
            ))
        };
        let b = |a: usize| {
            t.bytes[a].get(z).copied().ok_or(Error::UnsupportedTuning(
                super::retained::Unsupported::PostTransformIndices,
            ))
        };
        let p = Self {
            step: t.step[z],
            spacing_squared: f(0)?,
            advance: f(1)?,
            outer: f(2)?,
            inner: f(3)?,
            wobble: f(4)?,
            turn_radians: f(5)?,
            height: t.height,
            alpha: [b(1)?, b(0)?],
            fade: t.fade,
        };
        // Invalid/nonpositive tuning is outside the specified normal domain;
        // report it rather than substitute a sanitized/default value.
        if ![
            p.step,
            p.spacing_squared,
            p.advance,
            p.outer,
            p.inner,
            p.wobble,
            p.turn_radians,
            p.height,
        ]
        .iter()
        .all(|v| v.is_finite())
            || p.step <= 0.0
            || p.spacing_squared <= 0.0
            || p.fade == 0
        {
            return Err(Error::InvalidTuning);
        }
        Ok(p)
    }
    fn candidates<A: Arithmetic>(&self, a: &A, points: [Point; 4], t: f32, phase: &mut f32, water: f32) -> [Vertex; 2] {
        let positions = arithmetic::candidate(a, points, t, phase, water, self);
        [
            Vertex {
                position: positions[0],
                color: 0x00FF_FFFF | (u32::from(self.alpha[0]) << 24),
                uv: [0.0, 1.0],
            },
            Vertex {
                position: positions[1],
                color: 0x00FF_FFFF | (u32::from(self.alpha[1]) << 24),
                uv: [0.0, 0.0],
            },
        ]
    }
}

pub struct StripWalk {
    pub strips: Vec<Vec<Vertex>>,
    current: Option<usize>,
    new_start: bool,
    outer_test: bool,
    followups: u32,
    u: f32,
    positions: [[f32; 3]; 2],
    directions: [[f32; 2]; 2],
}
impl Default for StripWalk {
    fn default() -> Self {
        Self {
            strips: Vec::new(),
            current: None,
            new_start: true,
            outer_test: true,
            followups: 0,
            u: 0.0,
            positions: [[0.0; 3]; 2],
            directions: [[0.0; 2]; 2],
        }
    }
}
impl StripWalk {
    fn start(&mut self, acquire: &mut impl FnMut() -> bool) {
        // Failure nulls the current pointer, but does not erase its old strip.
        // It still follows the null-current append path, not an early return.
        if !acquire() {
            self.current = None;
        }
        if let Some(i) = self.current {
            if self.strips[i].len() < 8 {
                self.strips[i].clear();
                return;
            }
        }
        self.current = Some(self.strips.len());
        self.strips.push(Vec::new());
    }
    fn push(&mut self, mut v: Vertex) {
        v.uv[0] = self.u;
        self.strips[self.current.expect("StartNewRipple precedes append")].push(v);
    }
    /// Public primitive candidate ledger for §6 vectors 9–10. No host-prepared
    /// mesh can be installed into WaterRipple through this diagnostic operation.
    pub fn candidate(&mut self, v: [Vertex; 2], extent: Point, p: &Parameters, acquire: &mut impl FnMut() -> bool) {
        self.candidate_with_arithmetic(&Portable, v, extent, p, acquire);
    }
    pub fn candidate_with_arithmetic<A: Arithmetic>(
        &mut self,
        a: &A,
        v: [Vertex; 2],
        extent: Point,
        p: &Parameters,
        acquire: &mut impl FnMut() -> bool,
    ) {
        let inside = |v: Vertex| {
            v.position[0] >= 0.2_f32
                && v.position[0] < extent.x - 0.2_f32
                && v.position[2] >= 0.2_f32
                && v.position[2] < extent.z - 0.2_f32
        };
        if !inside(v[1]) || !inside(v[0]) {
            self.start(acquire);
            self.new_start = true;
            return;
        }
        if self.new_start {
            self.start(acquire);
            self.u = 0.0;
            self.push(v[0]);
            self.push(v[1]);
            self.positions = [v[0].position, v[1].position];
            self.directions = [[0.0; 2]; 2];
            self.followups = 0;
            self.outer_test = true;
            self.new_start = false;
            return;
        }
        for (side, vertex) in v.into_iter().enumerate() {
            if side == 0 && !self.outer_test {
                continue;
            }
            let squared = arithmetic::distance_squared(a, self.positions[side], vertex.position);
            if a.less(squared, a.load(p.spacing_squared)) {
                return;
            }
            let direction = arithmetic::direction(a, self.positions[side], vertex.position);
            if self.followups != 0 && arithmetic::turn_exceeds(a, self.directions[side], direction, p.turn_radians) {
                self.new_start = true;
                return;
            }
            self.push(vertex);
            self.positions[side] = vertex.position;
            self.directions[side] = direction;
            if side == 0 {
                self.outer_test = false;
            } else {
                self.followups += 1;
                self.u += p.advance;
                self.outer_test = true;
                if self.followups > 1000 {
                    self.new_start = true;
                }
            }
        }
    }
    pub fn finish(mut self) -> Vec<Vec<Vertex>> {
        if self.strips.last().is_some_and(|s| s.len() < 8) {
            self.strips.pop();
        }
        self.strips
    }
}

pub fn traverse(
    points: &[Point],
    phase: &mut f32,
    water: f32,
    extent: Point,
    p: &Parameters,
    acquire: &mut impl FnMut() -> bool,
) -> Result<Vec<Vec<Vertex>>, Error> {
    traverse_with_arithmetic(&Portable, points, phase, water, extent, p, acquire)
}

pub fn traverse_with_arithmetic<A: Arithmetic>(
    a: &A,
    points: &[Point],
    phase: &mut f32,
    water: f32,
    extent: Point,
    p: &Parameters,
    acquire: &mut impl FnMut() -> bool,
) -> Result<Vec<Vec<Vertex>>, Error> {
    let mut walk = StripWalk::default();
    let mut span = 0;
    let mut t = 0.0_f32;
    while span + 3 < points.len() {
        let vertices = p.candidates(a, points[span..span + 4].try_into().unwrap(), t, phase, water);
        walk.candidate_with_arithmetic(a, vertices, extent, p, acquire);
        t += p.step;
        if t >= 1.0 {
            let integral = t.floor();
            t -= integral;
            span += integral as usize;
        }
    }
    Ok(walk.finish())
}

/// Alpha arithmetic keeps binary32 constants in an exact f64 product before
/// truncation (e.g. 100 * 0.9f32 / 6 truncates to 14, not 15).
pub fn fade(vertices: &mut Vec<Vertex>, alpha: [u8; 2], configured: u32) -> Result<(), Error> {
    if configured == 0 {
        return Err(Error::InvalidTuning);
    }
    vertices.truncate(vertices.len() & !1);
    let ramp = (vertices.len() / 4).min(configured as usize);
    for (side, &full) in alpha.iter().enumerate() {
        let first = (f64::from(full) * f64::from(0.1_f32)).trunc() as u32 as u8;
        let increment = ((f64::from(full) * f64::from(0.9_f32) / f64::from(configured)).trunc() as u32 as u8).max(1);
        let mut value = first;
        for pair in 0..ramp {
            let head = pair * 2 + side;
            let tail = vertices.len() - 2 - pair * 2 + side;
            for index in [head, tail] {
                vertices[index].color = 0x00FF_FFFF | (u32::from(value) << 24);
            }
            value = value.wrapping_add(increment).min(full);
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeRandom {
    pub state: u32,
    pub remembered_seed: u32,
    pub third_word: u32,
}

/// Foreign host samples only; startup XOR/sentinel policy belongs to the owner.
pub trait RandomHost {
    fn time_word(&mut self) -> u32;
    fn clock_word(&mut self) -> u32;
}

impl NativeRandom {
    /// An already resolved ordinary seed (or captured fixture seed).
    pub fn from_effective_seed(seed: u32) -> Self {
        Self {
            state: seed,
            remembered_seed: seed,
            third_word: 0,
        }
    }
    pub fn startup(host: &mut dyn RandomHost) -> Self {
        let seed = host.time_word() ^ 0x8C09_C57A;
        let effective = if seed == u32::MAX {
            let time = host.time_word();
            let clock = host.clock_word();
            time ^ (clock | clock.wrapping_shl(16))
        } else {
            seed
        };
        Self::from_effective_seed(effective)
    }
    pub fn raw(&mut self) -> u32 {
        let q = u64::from(self.state) * 0x41C6_4E6D + 0x3039;
        self.state = q as u32;
        (q >> 16) as u32
    }
    pub fn bounded(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            0
        } else {
            ((u64::from(self.raw()) * u64::from(bound)) >> 32) as u32
        }
    }
    pub fn range(&mut self, bounds: [u32; 2]) -> u32 {
        bounds[0].wrapping_add(self.bounded(bounds[1].wrapping_sub(bounds[0])))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Animation {
    pub frame: u32,
    /// Microseconds, modulo 2^32.
    pub clock: u32,
    pub period: u32,
}
impl Animation {
    pub fn prepare(frames: u32, bounds: [u32; 2], random: &mut NativeRandom) -> Self {
        Self {
            frame: random.bounded(frames),
            period: random.range(bounds),
            clock: 0,
        }
    }
    pub fn tick(&mut self, elapsed: u32, frames: u32, bounds: [u32; 2], random: &mut NativeRandom) {
        self.clock = self.clock.wrapping_add(elapsed);
        if self.clock >= self.period {
            self.frame = self.frame.wrapping_add(1);
            self.period = random.range(bounds);
            self.clock = 0;
            if self.frame >= frames {
                self.frame = 0;
            }
        }
    }
}
