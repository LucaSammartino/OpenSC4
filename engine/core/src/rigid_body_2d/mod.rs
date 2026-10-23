// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded portable `cSC4RigidBodySimulator2D`, from rigid-body-2d.md §§3–7.
//!
//! Seven published methods are supported within the domains below: primary
//! sector, support point, contact velocity, collision test, sweep, dead sweep,
//! bounded removal. No integration, response, pair cycle, registration, packed
//! mapping, allocation, reference ownership, or destruction is inferred.
//!
//! Numeric policy: finite normal binary32 inputs (signed zeros allowed), valid
//! independent body/output records, nonnegative extents, caller-supplied
//! orthonormal axes, stable state, round-to-nearest/ties-even, no FTZ/DAZ.
//! Every unstored operation must be an EXACT dyadic with <=53 significant bits.
//! Thus the accepted operations agree with x87 PC=53 OR PC=64, without claiming
//! general binary64 evaluation is bit-exact x87. Binary32 stores are explicit.
//! Nonzero subnormal inputs/stores, overflow and inexact unstored arithmetic
//! return Unsupported; these errors are never collision false results. On an
//! error after accepted candidates, already performed output writes remain.
//! Rust borrows enforce nonaliasing; raw foreign pointers are not dereferenced.

mod numeric;
pub mod storage;

use numeric::{add, dot, mul, store, sub, valid};

pub type Pair = [f32; 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    NonFiniteOrSubnormal,
    NegativeExtent,
    InexactUnstoredArithmetic,
    Binary32StoreRange,
    /// RB7: transverse projection staging is unresolved when offsets round.
    CandidateOffsetStage,
    InvalidStableList,
    /// RB1: allocation, copying, registration and destruction are unproved.
    RegistrationLifetime,
    /// RB2: packed sector mapping and sector insertion/removal internals.
    SectorMapping,
    /// RB3: pair enumeration, persistent pairs and retry/termination.
    CollisionCycle,
    /// RB4: coincident random primitive and lifetime; response not implemented.
    CollisionResponse,
    /// RB5: angle/trigonometry, drag and coordinate conversion remain partial.
    InnerAdvance,
}

pub type Result<T> = std::result::Result<T, Unsupported>;

/// Raw public fields only; no mass conversion, normalization or COM ownership.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub position: Pair,
    pub velocity: Pair,
    pub angular_velocity: f32,
    pub extents: Pair,
    pub axis: Pair,
}

impl Body {
    fn validate(&self) -> Result<()> {
        for v in self
            .position
            .into_iter()
            .chain(self.velocity)
            .chain(self.extents)
            .chain(self.axis)
            .chain([self.angular_velocity])
        {
            valid(f64::from(v))?;
        }
        if self.extents.iter().any(|&v| v < 0.0) {
            return Err(Unsupported::NegativeExtent);
        }
        Ok(())
    }
}

fn wide(v: Pair) -> [f64; 2] {
    v.map(f64::from)
}
fn perp(v: Pair) -> Pair {
    [-v[1], v[0]]
}
fn difference(a: Pair, b: Pair) -> Result<[f64; 2]> {
    Ok([
        sub(f64::from(a[0]), f64::from(b[0]))?,
        sub(f64::from(a[1]), f64::from(b[1]))?,
    ])
}
fn stored_pair(v: [f64; 2]) -> Result<Pair> {
    Ok([store(v[0])?, store(v[1])?])
}

/// §3 unsigned component-min, not body priority or same-body rejection.
pub fn primary_sector(current: u32, a: u32, b: u32) -> bool {
    current == (a & 0xffff_0000).min(b & 0xffff_0000) + (a & 0xffff).min(b & 0xffff)
}

/// §4 directional corner selection. Ties, including either signed zero, do not
/// negate either extent. Y's first addition is stored; X's is not.
pub fn support_point(reference: Pair, direction: Pair, body: &Body) -> Result<Pair> {
    body.validate()?;
    let direction = wide(direction);
    let d = f64::from(store(dot(difference(body.position, reference)?, direction)?)?);
    let [c, s] = wide(body.axis);
    let [ex, ey] = wide(body.extents);
    let mut u = [f64::from(store(mul(ex, c)?)?), f64::from(store(mul(ex, s)?)?)];
    let mut v = [mul(ey, -s)?, f64::from(store(mul(ey, c)?)?)];
    if mul(dot(u, direction)?, d)? > 0.0 {
        u = u.map(|x| -x);
    }
    if mul(dot(v, direction)?, d)? > 0.0 {
        v = v.map(|x| -x);
    }
    let x = store(add(add(f64::from(body.position[0]), u[0])?, v[0])?)?;
    let y_first = store(add(f64::from(body.position[1]), u[1])?)?;
    let y = store(add(f64::from(y_first), v[1])?)?;
    Ok([x, y])
}

/// §5 B-at-point minus A-at-point, with the asymmetric published stores.
pub fn relative_velocity(a: &Body, b: &Body, point: Pair) -> Result<Pair> {
    a.validate()?;
    b.validate()?;
    let bo = wide(stored_pair(difference(point, b.position)?)?);
    let ao = difference(point, a.position)?;
    let wa = f64::from(a.angular_velocity);
    let wb = f64::from(b.angular_velocity);
    let ay_rot = store(mul(wa, ao[0])?)?;
    let ay = store(add(f64::from(a.velocity[1]), f64::from(ay_rot))?)?;
    let ax = add(f64::from(a.velocity[0]), mul(wa, -ao[1])?)?;
    let bx_rot = store(mul(wb, -bo[1])?)?;
    let bx = store(add(f64::from(b.velocity[0]), f64::from(bx_rot))?)?;
    let by = store(add(f64::from(b.velocity[1]), mul(wb, bo[0])?)?)?;
    stored_pair([sub(f64::from(bx), ax)?, sub(f64::from(by), f64::from(ay))?])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputField {
    PointY,
    PointX,
    NormalX,
    NormalY,
}

/// Optional instrumentation reports each actual store, after that store.
/// No callback computes a candidate or supplies any collision answer.
pub trait OutputObserver {
    fn stored(&mut self, field: OutputField, value: f32);
}
impl OutputObserver for () {
    fn stored(&mut self, _: OutputField, _: f32) {}
}

pub fn test_for_collision(a: &Body, b: &Body, normal: &mut Pair, point: &mut Pair) -> Result<bool> {
    test_for_collision_observed(a, b, normal, point, &mut ())
}

/// §6.1. Early separation/no eligible candidate preserves output sentinels.
/// An accepted candidate ALWAYS writes point then normal, even if the final
/// binary64 1e35 threshold returns false. Unsupported is a separate outcome.
pub fn test_for_collision_observed(
    a: &Body,
    b: &Body,
    normal: &mut Pair,
    point: &mut Pair,
    observer: &mut impl OutputObserver,
) -> Result<bool> {
    a.validate()?;
    b.validate()?;
    let aw = wide(a.axis);
    let ap = wide(perp(a.axis));
    let bw = wide(b.axis);
    let bp = wide(perp(b.axis));
    let delta = difference(b.position, a.position)?;
    let u = f64::from(store(dot(delta, aw)?)?);
    let v = f64::from(store(dot(delta, ap)?)?);
    let c = f64::from(store(dot(aw, bw)?)?);
    let s = f64::from(store(sub(mul(aw[0], bw[1])?, mul(aw[1], bw[0])?)?)?);
    let [ax, ay] = wide(a.extents);
    let [bx, by] = wide(b.extents);
    // Evaluate in CFG order: do not eagerly evaluate later separations.
    if sub(u.abs(), add(add(mul(c, bx)?.abs(), mul(-s, by)?.abs())?, ax)?)? > 0.0 {
        return Ok(false);
    }
    if sub(v.abs(), add(add(mul(c, by)?.abs(), mul(s, bx)?.abs())?, ay)?)? > 0.0 {
        return Ok(false);
    }
    if sub(
        add(mul(c, u)?, mul(s, v)?)?.abs(),
        add(add(mul(c.abs(), ax)?, mul(s.abs(), ay)?)?, bx)?,
    )? > 0.0
    {
        return Ok(false);
    }
    if sub(
        add(mul(c, v)?, mul(-s, u)?)?.abs(),
        add(add(mul(c.abs(), ay)?, mul(s.abs(), ax)?)?, by)?,
    )? > 0.0
    {
        return Ok(false);
    }
    // All four support calls precede ALL candidate evaluation.
    let qs = [
        support_point(a.position, a.axis, b)?,
        support_point(a.position, perp(a.axis), b)?,
        support_point(b.position, b.axis, a)?,
        support_point(b.position, perp(b.axis), a)?,
    ];
    let mut score = f32::from_bits(0x7cf0_bdc2);
    for (i, q) in qs.into_iter().enumerate() {
        let (reference, n, transverse, extent) = match i {
            0 => (a.position, a.axis, ap, ay),
            1 => (a.position, perp(a.axis), aw, ax),
            2 => (b.position, b.axis, bp, by),
            _ => (b.position, perp(b.axis), bw, bx),
        };
        let raw_r = difference(q, reference)?;
        let r = wide(stored_pair(raw_r)?);
        // §6.1 explicitly stores offsets before t, but does not explicitly
        // stage h relative to those stores (RB7). Admit only exact stores so
        // either reading gives the identical transverse gate; never guess.
        if r != raw_r {
            return Err(Unsupported::CandidateOffsetStage);
        }
        if dot(r, transverse)?.abs() > extent {
            continue;
        }
        let t = store(dot(r, wide(n))?)?;
        let velocity = relative_velocity(a, b, q)?;
        let gate = mul(dot(wide(n), wide(velocity))?, f64::from(t))?;
        if (i < 2 && gate > 0.0) || (i >= 2 && gate < 0.0) {
            continue;
        }
        if i != 0 && t.abs() >= score {
            continue;
        }
        let fields = if i == 0 {
            [OutputField::PointY, OutputField::PointX]
        } else {
            [OutputField::PointX, OutputField::PointY]
        };
        for field in fields {
            let component = if field == OutputField::PointX { 0 } else { 1 };
            point[component] = q[component];
            observer.stored(field, q[component]);
        }
        normal[0] = n[0];
        observer.stored(OutputField::NormalX, n[0]);
        normal[1] = n[1];
        observer.stored(OutputField::NormalY, n[1]);
        score = t.abs();
    }
    Ok(f64::from(score) < 1e35_f64)
}

/// Named missing owner bodies, never successful no-ops or host full answers.
pub fn resolve_collision() -> Result<()> {
    Err(Unsupported::CollisionResponse)
}
pub fn add_object() -> Result<()> {
    Err(Unsupported::RegistrationLifetime)
}
pub fn map_sector() -> Result<()> {
    Err(Unsupported::SectorMapping)
}
pub fn do_collisions() -> Result<()> {
    Err(Unsupported::CollisionCycle)
}
pub fn advance_inner(_root_offset: u32, _dt_bits: u32) -> Result<()> {
    Err(Unsupported::InnerAdvance)
}

/// Exact wrapper phase plan, NOT an implemented inner timestep or pair cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvancePhase {
    Inner { root_offset: u32, dt_bits: u32 },
    Collisions,
}
pub fn advance_phases(dt_bits: u32) -> [AdvancePhase; 3] {
    [
        AdvancePhase::Inner {
            root_offset: 0,
            dt_bits,
        },
        AdvancePhase::Inner {
            root_offset: 4,
            dt_bits,
        },
        AdvancePhase::Collisions,
    ]
}
/// Stops at the first missing body; does not pretend all planned calls ran.
pub fn advance_time(dt_bits: u32) -> Result<()> {
    for phase in advance_phases(dt_bits) {
        match phase {
            AdvancePhase::Inner { root_offset, dt_bits } => advance_inner(root_offset, dt_bits)?,
            AdvancePhase::Collisions => do_collisions()?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
