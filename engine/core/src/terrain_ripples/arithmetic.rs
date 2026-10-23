// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! §1.3/WR13 ordered arithmetic. Register values never implicitly become f32.
//! The owner supplies the ledger, while `Arithmetic` supplies single operations
//! under the caller's control word and the native math-library boundary.
//! `Portable` uses binary64 and host transcendentals: it is not x87 bit equality.

use super::{mesh::Parameters, Point};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spill {
    W1,
    W2,
    W3,
    D0,
    D1,
    D2,
    Px,
    Pz,
    X,
    Tx,
    Amplitude,
    Phase,
    OuterX,
    OuterZ,
    InnerX,
    InnerZ,
    InnerY,
    DirectionX,
    DirectionZ,
}

/// Primitive arithmetic only: no backend may supply weights, geometry or a
/// turn decision. An exact backend must preserve registers/control word and
/// implement the spec's sine/acos library semantics, including exception modes.
pub trait Arithmetic: Default + Send + Sync {
    type Register: Copy;
    fn load(&self, value: f32) -> Self::Register;
    fn store(&self, at: Spill, value: Self::Register) -> f32;
    fn add(&self, a: Self::Register, b: Self::Register) -> Self::Register;
    fn sub(&self, a: Self::Register, b: Self::Register) -> Self::Register;
    fn mul(&self, a: Self::Register, b: Self::Register) -> Self::Register;
    fn div(&self, a: Self::Register, b: Self::Register) -> Self::Register;
    fn sqrt(&self, a: Self::Register) -> Self::Register;
    fn sin(&self, a: Self::Register) -> Self::Register;
    /// §1.3's acos wrapper: nearest rounding, caller precision preserved, then
    /// restore caller mode. Exceptional handling remains a library boundary.
    fn acos(&self, a: Self::Register) -> Self::Register;
    fn less(&self, a: Self::Register, b: Self::Register) -> bool;
    fn greater(&self, a: Self::Register, b: Self::Register) -> bool;
}

#[derive(Default)]
pub struct Portable;
impl Arithmetic for Portable {
    type Register = f64;
    fn load(&self, v: f32) -> f64 {
        f64::from(v)
    }
    fn store(&self, _: Spill, v: f64) -> f32 {
        v as f32
    }
    fn add(&self, a: f64, b: f64) -> f64 {
        a + b
    }
    fn sub(&self, a: f64, b: f64) -> f64 {
        a - b
    }
    fn mul(&self, a: f64, b: f64) -> f64 {
        a * b
    }
    fn div(&self, a: f64, b: f64) -> f64 {
        a / b
    }
    fn sqrt(&self, a: f64) -> f64 {
        a.sqrt()
    }
    fn sin(&self, a: f64) -> f64 {
        a.sin()
    }
    fn acos(&self, a: f64) -> f64 {
        if a == 1.0 {
            0.0
        } else if a == -1.0 {
            std::f64::consts::PI
        } else {
            ((1.0 + a) * (1.0 - a)).sqrt().atan2(a)
        }
    }
    fn less(&self, a: f64, b: f64) -> bool {
        a < b
    }
    fn greater(&self, a: f64, b: f64) -> bool {
        a > b
    }
}

pub struct Sample<R> {
    pub point: Point,
    pub tx: f32,
    pub tz: R,
}

/// Valid four-point input. There is deliberately no zero-derivative guard.
pub fn spline<A: Arithmetic>(a: &A, points: [Point; 4], parameter: f32) -> Sample<A::Register> {
    let t = a.load(parameter);
    let one = a.load(1.0);
    let two = a.load(2.0);
    let three = a.load(3.0);
    let s = a.load(f32::from_bits(0x3E2A_AAAB));
    let h = a.load(0.5);
    let triple = a.mul(three, t);
    let square = a.mul(t, t);
    let w0 = a.mul(a.add(a.mul(a.sub(a.mul(a.sub(three, t), t), three), t), one), s);
    let w1 = a.load(a.store(
        Spill::W1,
        a.mul(a.add(a.mul(a.mul(a.sub(triple, a.load(6.0)), t), t), a.load(4.0)), s),
    ));
    let w2 = a.load(a.store(
        Spill::W2,
        a.mul(a.add(a.mul(a.add(a.mul(a.sub(three, triple), t), three), t), one), s),
    ));
    let w3 = a.load(a.store(Spill::W3, a.mul(a.mul(t, square), s)));
    let d0 = a.load(a.store(Spill::D0, a.mul(a.sub(a.mul(a.sub(two, t), t), one), h)));
    let d1 = a.load(a.store(Spill::D1, a.mul(a.mul(a.sub(triple, a.load(4.0)), t), h)));
    let d2 = a.load(a.store(Spill::D2, a.mul(a.add(a.mul(a.sub(two, triple), t), one), h)));
    let d3 = a.mul(square, h);
    let [x0, x1, x2, x3] = points.map(|p| a.load(p.x));
    let [z0, z1, z2, z3] = points.map(|p| a.load(p.z));
    let px = a.store(
        Spill::Px,
        a.add(a.add(a.add(a.mul(w2, x2), a.mul(w1, x1)), a.mul(w3, x3)), a.mul(w0, x0)),
    );
    let pz = a.store(
        Spill::Pz,
        a.add(a.add(a.add(a.mul(w3, z3), a.mul(w2, z2)), a.mul(w0, z0)), a.mul(w1, z1)),
    );
    let x = a.load(a.store(
        Spill::X,
        a.add(a.add(a.add(a.mul(d2, x2), a.mul(d1, x1)), a.mul(d3, x3)), a.mul(d0, x0)),
    ));
    let z = a.add(a.add(a.add(a.mul(d3, z3), a.mul(d2, z2)), a.mul(d0, z0)), a.mul(d1, z1));
    let reciprocal = a.div(one, a.sqrt(a.add(a.mul(z, z), a.mul(x, x))));
    Sample {
        point: Point::new(px, pz),
        tx: a.store(Spill::Tx, a.mul(x, reciprocal)),
        tz: a.mul(z, reciprocal),
    }
}

pub fn candidate<A: Arithmetic>(
    a: &A,
    points: [Point; 4],
    t: f32,
    phase: &mut f32,
    water: f32,
    p: &Parameters,
) -> [[f32; 3]; 2] {
    let s = spline(a, points, t);
    // Stored wobble amplitude, followed by register-resident radius.
    let amplitude = a.load(a.store(Spill::Amplitude, a.mul(a.load(p.outer), a.load(p.wobble))));
    let radius = a.add(a.mul(a.sin(a.load(*phase)), amplitude), a.load(p.outer));
    *phase = a.store(Spill::Phase, a.add(a.load(*phase), a.load(f32::from_bits(0x3DCC_CCCD))));
    let tx = a.load(s.tx);
    let px = a.load(s.point.x);
    let pz = a.load(s.point.z);
    let inner = a.load(p.inner);
    [
        [
            a.store(Spill::OuterX, a.add(a.mul(radius, s.tz), px)),
            water,
            a.store(Spill::OuterZ, a.sub(pz, a.mul(radius, tx))),
        ],
        [
            a.store(Spill::InnerX, a.sub(px, a.mul(s.tz, inner))),
            a.store(Spill::InnerY, a.add(a.load(water), a.load(p.height))),
            a.store(Spill::InnerZ, a.add(a.mul(tx, inner), pz)),
        ],
    ]
}

pub fn distance_squared<A: Arithmetic>(a: &A, previous: [f32; 3], candidate: [f32; 3]) -> A::Register {
    let dx = a.sub(a.load(previous[0]), a.load(candidate[0]));
    let dz = a.sub(a.load(previous[2]), a.load(candidate[2]));
    a.add(a.mul(dz, dz), a.mul(dx, dx))
}

pub fn direction<A: Arithmetic>(a: &A, previous: [f32; 3], candidate: [f32; 3]) -> [f32; 2] {
    let dx = a.sub(a.load(candidate[0]), a.load(previous[0]));
    let dz = a.sub(a.load(candidate[2]), a.load(previous[2]));
    let reciprocal = a.div(a.load(1.0), a.sqrt(a.add(a.mul(dz, dz), a.mul(dx, dx))));
    [
        a.store(Spill::DirectionX, a.mul(dx, reciprocal)),
        a.store(Spill::DirectionZ, a.mul(dz, reciprocal)),
    ]
}

pub fn turn_exceeds<A: Arithmetic>(a: &A, previous: [f32; 2], current: [f32; 2], limit: f32) -> bool {
    let dot = a.add(
        a.mul(a.load(previous[1]), a.load(current[1])),
        a.mul(a.load(previous[0]), a.load(current[0])),
    );
    a.greater(a.acos(dot), a.load(limit))
}
