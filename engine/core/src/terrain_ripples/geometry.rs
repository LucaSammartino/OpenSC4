// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::{Bounds, ContractGap, Error, RandomSource, RippleTuning, Zoom};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub z: f32,
}

impl Point {
    pub fn new(x: f32, z: f32) -> Self {
        Self { x, z }
    }
    fn finite(self) -> bool {
        self.x.is_finite() && self.z.is_finite()
    }
    fn difference(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.z - other.z)
    }
    fn squared_length(self) -> f32 {
        self.x * self.x + self.z * self.z
    }
    fn unit(self) -> Result<Self, Error> {
        let length = self.squared_length().sqrt();
        if !length.is_finite() || length == 0.0 {
            return Err(Error::InvalidGeometry);
        }
        Ok(Self::new(self.x / length, self.z / length))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplinePoint {
    pub point: Point,
    pub tangent: Point,
}

/// §1.3: uniform cubic B-spline. This evaluates a given four-point span;
/// No endpoint padding or wrapping; complete owner traversal lives in `mesh`.
/// Diagnostic projection of §1.3's ledger using portable host math. Unlike the
/// owner pipeline, this public pair of points necessarily spills tangent z.
pub fn spline(points: [Point; 4], t: f32) -> Result<SplinePoint, Error> {
    if !points.iter().all(|p| p.finite()) || !(0.0..=1.0).contains(&t) {
        return Err(Error::InvalidGeometry);
    }
    let sample = super::arithmetic::spline(&super::arithmetic::Portable, points, t);
    let tangent = Point::new(sample.tx, sample.tz as f32);
    if !sample.point.finite() || !tangent.finite() {
        return Err(Error::InvalidGeometry);
    }
    Ok(SplinePoint {
        point: sample.point,
        tangent,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidatePair {
    pub outer: Point,
    pub inner: Point,
    pub height: f32,
}

/// §1.3: normal is exactly (tangent.z, -tangent.x), without assigning
/// shore/water meanings to the flags or orientation (WR2).
pub fn candidate_pair(
    sample: SplinePoint,
    phase: f32,
    water_level: f32,
    tuning: &RippleTuning,
    zoom: Zoom,
) -> Result<CandidatePair, Error> {
    tuning.validate()?;
    if !phase.is_finite() || !water_level.is_finite() || !sample.point.finite() {
        return Err(Error::InvalidGeometry);
    }
    let tangent = sample.tangent.unit()?;
    let normal = Point::new(tangent.z, -tangent.x);
    let i = zoom.index();
    let outer = tuning.outer_distance[i];
    let distance = outer + outer * tuning.wobble[i] * phase.sin();
    let pair = CandidatePair {
        outer: Point::new(
            sample.point.x + distance * normal.x,
            sample.point.z + distance * normal.z,
        ),
        inner: Point::new(
            sample.point.x - tuning.inner_distance[i] * normal.x,
            sample.point.z - tuning.inner_distance[i] * normal.z,
        ),
        height: water_level + tuning.height,
    };
    if !pair.outer.finite() || !pair.inner.finite() || !pair.height.is_finite() {
        return Err(Error::InvalidGeometry);
    }
    Ok(pair)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RippleVertex {
    pub point: Point,
    pub height: f32,
    pub texture_advance: f32,
}

/// Legacy diagnostic whole-pair geometry. The owner uses `mesh::StripWalk`,
/// which retains partial pairs, UV-side coordinates and native alpha ramps.
#[derive(Clone, Debug, PartialEq)]
pub struct StripGeometry {
    pairs: Vec<CandidatePair>,
    advance: f32,
}

impl StripGeometry {
    pub fn pairs(&self) -> &[CandidatePair] {
        &self.pairs
    }
    pub fn vertex_count(&self) -> usize {
        self.pairs.len() * 2
    }

    pub fn vertices(&self) -> Vec<RippleVertex> {
        // Literal outer/inner pair ordering and initial texture advance 0;
        // native order and starting coordinate need confirmation (WR5).
        self.pairs
            .iter()
            .enumerate()
            .flat_map(|(i, p)| {
                let texture_advance = i as f32 * self.advance;
                [
                    RippleVertex {
                        point: p.outer,
                        height: p.height,
                        texture_advance,
                    },
                    RippleVertex {
                        point: p.inner,
                        height: p.height,
                        texture_advance,
                    },
                ]
            })
            .collect()
    }

    pub fn fade_pair_count(&self, tuning: &RippleTuning) -> usize {
        (self.pairs.len() / 2).min(tuning.fade_pairs as usize)
    }

    pub fn prepare_for_display(&self) -> Result<(), Error> {
        // The ramp's endpoint, denominator and integer rounding are not specified.
        Err(Error::Gap(ContractGap::StripPreparationWr5))
    }

    /// §1.3 factory slot 6 and exact resource key. A failed binding keeps no
    /// strip. This owns the binding; it does not delegate geometry to the host.
    pub fn bind_texture(
        self,
        factory: &dyn super::TextureFactory,
        tuning: &RippleTuning,
        random: &mut dyn RandomSource,
    ) -> Result<Option<AnimatedStrip>, Error> {
        let Some(texture) = factory.bind_slot6(super::RIPPLE_TEXTURE) else {
            return Ok(None);
        };
        let animation = FrameAnimation::random_start(texture.frame_count(), tuning, random)?;
        Ok(Some(AnimatedStrip {
            geometry: self,
            texture,
            animation,
        }))
    }
}

/// Legacy diagnostic bound strip; not the WaterRipple owner's display stream.
/// Native display preparation and drawing use `mesh` and `render` instead.
pub struct AnimatedStrip {
    geometry: StripGeometry,
    texture: Arc<dyn super::TextureBinding>,
    animation: FrameAnimation,
}

impl AnimatedStrip {
    pub fn geometry(&self) -> &StripGeometry {
        &self.geometry
    }
    pub fn texture(&self) -> &Arc<dyn super::TextureBinding> {
        &self.texture
    }
    pub fn animation(&self) -> &FrameAnimation {
        &self.animation
    }
    pub fn tick(&mut self, elapsed: u64, tuning: &RippleTuning, random: &mut dyn RandomSource) -> Result<bool, Error> {
        self.animation.tick(elapsed, tuning, random)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairDecision {
    Kept,
    TooClose,
    EndAtEdge,
    EndAtTurn,
    EndAtLimit,
}

/// Legacy whole-pair diagnostic filter. This is not the owner algorithm:
/// `mesh::StripWalk` consumes split candidates without retry and keeps partials.
pub struct PairFilter {
    spacing_squared: f32,
    turn_cosine: f32,
    advance: f32,
    pairs: Vec<CandidatePair>,
    previous_directions: Option<[Point; 2]>,
}

impl PairFilter {
    pub fn new(tuning: &RippleTuning, zoom: Zoom) -> Result<Self, Error> {
        tuning.validate()?;
        let i = zoom.index();
        Ok(Self {
            spacing_squared: tuning.spacing[i] * tuning.spacing[i],
            turn_cosine: tuning.max_turn_degrees[i].to_radians().cos(),
            advance: tuning.texture_advance[i],
            pairs: Vec::new(),
            previous_directions: None,
        })
    }

    pub fn push(&mut self, pair: CandidatePair, bounds: Bounds) -> Result<PairDecision, Error> {
        if !pair.outer.finite()
            || !pair.inner.finite()
            || !pair.height.is_finite()
            || !bounds.min.finite()
            || !bounds.max.finite()
            || bounds.min.x > bounds.max.x
            || bounds.min.z > bounds.max.z
        {
            return Err(Error::InvalidGeometry);
        }
        if self.pairs.len() >= 1000 {
            return Ok(PairDecision::EndAtLimit);
        }
        // Literal either-point edge check; exact 0.2 and priority are WR5.
        if !bounds.contains_away_from_edge(pair.outer) || !bounds.contains_away_from_edge(pair.inner) {
            return Ok(PairDecision::EndAtEdge);
        }
        if let Some(last) = self.pairs.last() {
            let displacements = [pair.outer.difference(last.outer), pair.inner.difference(last.inner)];
            // Both points must pass, per "outer or inner ... only when"; WR5.
            if displacements.iter().any(|d| d.squared_length() < self.spacing_squared) {
                return Ok(PairDecision::TooClose);
            }
            let directions = [displacements[0].unit()?, displacements[1].unit()?];
            if let Some(previous) = self.previous_directions {
                for i in 0..2 {
                    let dot = directions[i].x * previous[i].x + directions[i].z * previous[i].z;
                    if dot < self.turn_cosine {
                        return Ok(PairDecision::EndAtTurn);
                    }
                }
            }
            self.previous_directions = Some(directions);
        }
        self.pairs.push(pair);
        Ok(PairDecision::Kept)
    }

    pub fn finish(self) -> Option<StripGeometry> {
        if self.pairs.len() < 4 {
            return None;
        }
        Some(StripGeometry {
            pairs: self.pairs,
            advance: self.advance,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameAnimation {
    frame: u32,
    frame_count: u32,
    elapsed: u64,
    period: u64,
}

impl FrameAnimation {
    pub fn new(frame: u32, frame_count: u32, period: u64) -> Result<Self, Error> {
        if frame_count == 0 {
            return Err(Error::EmptyTexture);
        }
        if frame >= frame_count || period == 0 {
            return Err(Error::InvalidRandomValue);
        }
        Ok(Self {
            frame,
            frame_count,
            elapsed: 0,
            period,
        })
    }

    pub fn random_start(frame_count: u32, tuning: &RippleTuning, random: &mut dyn RandomSource) -> Result<Self, Error> {
        if frame_count == 0 {
            return Err(Error::EmptyTexture);
        }
        let frame = random.below(frame_count);
        Self::new(frame, frame_count, tuning.random_period(random)?)
    }

    pub fn frame(&self) -> u32 {
        self.frame
    }
    pub fn period(&self) -> u64 {
        self.period
    }
    pub fn elapsed(&self) -> u64 {
        self.elapsed
    }

    /// §6 vector 6 settles equality: advance at exactly the period. Overshoot,
    /// catch-up and remainder behavior are missing (WR6); leave state unchanged.
    pub fn tick(&mut self, elapsed: u64, tuning: &RippleTuning, random: &mut dyn RandomSource) -> Result<bool, Error> {
        let total = self
            .elapsed
            .checked_add(elapsed)
            .ok_or(Error::Gap(ContractGap::AnimationOvershootWr6))?;
        if total > self.period {
            return Err(Error::Gap(ContractGap::AnimationOvershootWr6));
        }
        if total < self.period {
            self.elapsed = total;
            return Ok(false);
        }
        let period = tuning.random_period(random)?;
        self.frame = if self.frame + 1 == self.frame_count {
            0
        } else {
            self.frame + 1
        };
        self.elapsed = 0;
        self.period = period;
        Ok(true)
    }
}
