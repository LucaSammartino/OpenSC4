// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::Error;

pub const RIPPLE_EXEMPLAR: [u32; 3] = [0x6534_284A, 0x88CD_66E9, 2];
pub const RIPPLE_TEXTURE: [u32; 3] = [0x7AB5_0E44, 0x891B_0E1A, 0xE9A4_4A88];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zoom(usize);

impl Zoom {
    pub fn new(value: usize) -> Result<Self, Error> {
        if value < 5 {
            Ok(Self(value))
        } else {
            Err(Error::InvalidTuning)
        }
    }

    pub fn index(self) -> usize {
        self.0
    }
}

/// The provider decodes the exemplar, not the owner's tuning decisions.
pub trait TuningSource {
    fn boolean(&self, exemplar: [u32; 3], property: u32) -> Option<bool>;
    fn integer(&self, exemplar: [u32; 3], property: u32) -> Option<u32>;
    fn scalar(&self, exemplar: [u32; 3], property: u32) -> Option<f32>;
    fn per_zoom(&self, exemplar: [u32; 3], property: u32) -> Option<[f32; 5]>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct RippleTuning {
    pub enabled: bool,
    pub minimum_shore_points: u32,
    pub spacing: [f32; 5],
    pub height: f32,
    pub texture_advance: [f32; 5],
    pub outer_distance: [f32; 5],
    pub inner_distance: [f32; 5],
    /// WR1: raw elapsed units. No conversion to seconds or milliseconds.
    pub frame_period: u64,
    pub inner_alpha: [f32; 5],
    pub outer_alpha: [f32; 5],
    pub wobble: [f32; 5],
    pub max_turn_degrees: [f32; 5],
    pub edit_area_threshold: u32,
    pub fade_pairs: u32,
}

impl Default for RippleTuning {
    fn default() -> Self {
        Self {
            enabled: true,
            minimum_shore_points: 50,
            spacing: [16.0; 5],
            height: 0.3,
            texture_advance: [0.15, 0.3, 0.3, 0.6, 0.75],
            outer_distance: [45.0, 40.0, 35.0, 30.0, 26.0],
            inner_distance: [1.0; 5],
            frame_period: 175_000,
            inner_alpha: [100.0, 150.0, 200.0, 255.0, 255.0],
            outer_alpha: [100.0, 150.0, 200.0, 255.0, 255.0],
            wobble: [0.2; 5],
            max_turn_degrees: [60.0; 5],
            edit_area_threshold: 12,
            fade_pairs: 6,
        }
    }
}

impl RippleTuning {
    /// §1.1. Missing property behavior is unpublished (WR8); `read` reports
    /// missing values instead of silently assuming the shipped values are defaults.
    pub fn read(source: &dyn TuningSource) -> Result<Self, Error> {
        let integer = |id| source.integer(RIPPLE_EXEMPLAR, id).ok_or(Error::InvalidTuning);
        let per_zoom = |id| source.per_zoom(RIPPLE_EXEMPLAR, id).ok_or(Error::InvalidTuning);
        let result = Self {
            enabled: source
                .boolean(RIPPLE_EXEMPLAR, 0x491C_9C00)
                .ok_or(Error::InvalidTuning)?,
            minimum_shore_points: integer(0x491C_9C01)?.max(2),
            spacing: per_zoom(0x491C_9C02)?,
            height: source
                .scalar(RIPPLE_EXEMPLAR, 0x491C_9C03)
                .ok_or(Error::InvalidTuning)?,
            texture_advance: per_zoom(0x491C_9C04)?,
            outer_distance: per_zoom(0x491C_9C05)?,
            inner_distance: per_zoom(0x491C_9C06)?,
            frame_period: u64::from(integer(0x491C_9C07)?),
            inner_alpha: per_zoom(0x491C_9C08)?,
            outer_alpha: per_zoom(0x491C_9C09)?,
            wobble: per_zoom(0x491C_9C0A)?,
            max_turn_degrees: per_zoom(0x491C_9C0B)?,
            edit_area_threshold: integer(0x491C_9C0C)?,
            fade_pairs: integer(0x491C_9C0D)?,
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), Error> {
        // Safe Rust boundary validation only; invalid native tuning is unknown (WR8).
        let finite = [
            &self.spacing,
            &self.texture_advance,
            &self.outer_distance,
            &self.inner_distance,
            &self.inner_alpha,
            &self.outer_alpha,
            &self.wobble,
            &self.max_turn_degrees,
        ]
        .iter()
        .all(|a| a.iter().all(|v| v.is_finite()));
        if !finite
            || !self.height.is_finite()
            || self.frame_period == 0
            || self.frame_period > u64::MAX / 13
            || self.minimum_shore_points < 2
            || self.spacing.iter().any(|v| *v <= 0.0)
            || self.max_turn_degrees.iter().any(|v| !(0.0..=180.0).contains(v))
        {
            return Err(Error::InvalidTuning);
        }
        Ok(())
    }

    pub fn shore_is_long_enough(&self, points: usize) -> bool {
        points > self.minimum_shore_points as usize
    }

    pub fn spline_step(&self, zoom: Zoom) -> f32 {
        self.spacing[zoom.index()] / 64.0
    }

    pub fn edit_count_threshold(&self) -> u64 {
        u64::from(self.minimum_shore_points).pow(2) / 10
    }

    pub fn random_period(&self, random: &mut dyn super::RandomSource) -> Result<u64, Error> {
        self.validate()?;
        // Literal 0.7–1.3 range, integral raw units. Rounding/distribution: WR6.
        let min = self.frame_period * 7 / 10;
        let max = self.frame_period * 13 / 10;
        let value = random.inclusive(min, max);
        if value == 0 || !(min..=max).contains(&value) {
            return Err(Error::InvalidRandomValue);
        }
        Ok(value)
    }
}
