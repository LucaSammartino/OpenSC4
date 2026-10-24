// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The brush effect (effects.md §8.2, §8.3): a terrain brush applied along
//! the effect's path, `floor(acc)` times per update with `acc += rate × dt`,
//! or once with `trunc(rate)` applications for a one-shot record.

use crate::rng::RzRandom;
use crate::terrain::BrushParams;
use crate::x87::r24;

use super::directory::BrushRecord;
use super::EffectsHost;

/// `tBrushParameters.data4` of an effect brush (§8.3).
pub const BRUSH_DATA4: u32 = 3;

/// One brush component (vtable `0x00A9EDD0`, §8.3).
#[derive(Clone, Debug, PartialEq)]
pub struct BrushEffect {
    pub record: BrushRecord,
    /// The drawn strength and radius scale (§8.2), stored as `float`.
    pub strength: f32,
    pub radius: f32,
    pub active: bool,
    /// A long terrain operation was begun (terrain slot 87).
    pub long_op: bool,
    pub acc: f32,
    /// Elapsed effect-seconds.
    pub t: f32,
    pub prev: (f32, f32),
    pub cur: (f32, f32),
    /// Diagnostics: `ApplyBrush` calls made.
    pub applications: u64,
}

impl BrushEffect {
    /// Creation (`0x00581490`, §8.2): two `RandomDoubleRangeUniform` draws on
    /// the brush generator, strength first, both stored as `float` (the
    /// draws happen even for degenerate ranges).
    pub fn new(record: &BrushRecord, rng: &mut RzRandom) -> BrushEffect {
        let strength = rng.range_f64(f64::from(record.strength.0), f64::from(record.strength.1)) as f32;
        let radius = rng.range_f64(f64::from(record.radius.0), f64::from(record.radius.1)) as f32;
        BrushEffect {
            record: *record,
            strength,
            radius,
            active: false,
            long_op: false,
            acc: 0.0,
            t: 0.0,
            prev: (0.0, 0.0),
            cur: (0.0, 0.0),
            applications: 0,
        }
    }

    /// `SetTransform` (§8.3) with an identity local placement (EF7): `cur`
    /// is the parent translation's `(x, z)`.
    pub fn set_position(&mut self, x: f32, z: f32) {
        self.cur = (x, z);
    }

    /// `SetTransform(local, parent)` at the semantic transform boundary (§8.3).
    pub fn set_transform(&mut self, local_offset: [f32; 3], parent: &super::transform::ParentTransform) {
        use super::transform::{ParentTransform, Placement, Precision};
        let local = Placement {
            flags: 1,
            transform: ParentTransform {
                translation: local_offset,
                ..ParentTransform::default()
            },
            ..Placement::default()
        };
        let parent = Placement {
            flags: u8::from(*parent != ParentTransform::default()),
            transform: *parent,
            ..Placement::default()
        };
        self.set_placement(&local, &parent, Precision::Single);
    }

    pub fn set_placement(
        &mut self,
        local: &super::transform::Placement,
        parent: &super::transform::Placement,
        precision: super::transform::Precision,
    ) {
        let position = local.world_position(parent, false, precision);
        self.set_position(position[0], position[2]);
    }

    /// `Start(mode)` (`0x005812C0`).
    pub fn start(&mut self) {
        self.prev = self.cur;
        self.active = true;
        self.long_op = false;
        self.acc = 0.0;
        self.t = 0.0;
    }

    /// `Stop(mode)` (`0x00581530`): ends the long operation if one was
    /// begun, then `active := 0`. [choice, EF8] `longOp` is not cleared (the
    /// spec does not clear it), so a second stop ends the operation again.
    pub fn stop(&mut self, host: &mut dyn EffectsHost) {
        if self.long_op {
            host.end_long_terrain_operation();
        }
        self.active = false;
    }

    /// `IsActive`.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// `Update(dt)` (`0x005817F0`), `dt` in effect-seconds.
    pub fn update(&mut self, dt: f32, host: &mut dyn EffectsHost) {
        if self.record.one_shot {
            self.active = false;
            let n = self.record.rate.trunc();
            self.apply(if n > 0.0 { n as u64 } else { 0 }, host);
            return;
        }
        if !self.long_op {
            host.begin_long_terrain_operation();
            self.long_op = true;
        }
        let dt64 = f64::from(dt);
        self.acc = r24(r24(dt64 * f64::from(self.record.rate)) + f64::from(self.acc)) as f32;
        self.t = r24(dt64 + f64::from(self.t)) as f32;
        let n = f64::from(self.acc).floor();
        self.apply(if n > 0.0 { n as u64 } else { 0 }, host);
        self.acc = r24(f64::from(self.acc) - n) as f32;
        if self.record.length > 0.0 && self.t > self.record.length {
            self.active = false;
        }
    }

    /// `Apply(n)` (`0x00581640`): nothing when either end is outside the
    /// city (and `prev` stays); else `n` brush calls from `prev` to `cur`,
    /// the last one redisplaying, then `prev := cur` (also for `n = 0`).
    fn apply(&mut self, n: u64, host: &mut dyn EffectsHost) {
        if !host.location_in_bounds(self.prev.0, self.prev.1) || !host.location_in_bounds(self.cur.0, self.cur.1) {
            return;
        }
        for i in 0..n {
            let mut p = BrushParams {
                from_x: self.prev.0,
                from_z: self.prev.1,
                to_x: self.cur.0,
                to_z: self.cur.1,
                level: self.record.level as i32,
                redisplay: i + 1 == n,
                strength_scale: self.strength,
                radius_scale: self.radius,
                data4: BRUSH_DATA4,
                out: None,
                target_height: self.record.target_height,
            };
            host.apply_brush(self.record.brush_id, &mut p);
            self.applications += 1;
        }
        self.prev = self.cur;
    }
}
