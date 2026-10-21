// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.
//! The `S3DTransform` stream form (`xform`, `docs/specs/lot-records.md`
//! §2.1; `operator<<` `0x005DA8C0`, read `0x005DA710`), used by the network
//! base record's field 3 (`network-occupants.md` §2.3).
//!
//! | Bytes | Present when | Field |
//! |---|---|---|
//! | u8 | always | the mask |
//! | 9 × f32 | `mask & 4` | the 3×3 matrix, row by row |
//! | 3 × f32 | `mask & 1` | the translation x, y, z |
//! | 1 × f32 | `mask & 2` | the uniform scale |
//!
//! In the object the transform is 0x38 bytes (network-occupants.md §2.2):
//! u8 change counter `+0`, u8 part mask `+1`, 9 f32 rotation, 3 f32
//! translation, f32 scale. The writer emits the live mask unchanged (NO15).
//!
//! Read rules (§2.3 "Transform read rules", NO15): [`Xform::read_into`]
//! resets to identity only when the live mask is non-zero, ORs each present
//! bit, increments the counter per reset and per part, and sets the scale
//! bit only when the scale read differs from 1.0. [`Xform::legacy_load_into`]
//! is the 56-byte `S3DTransformLegacyLoad` of v4 records.

use crate::stream::{InStream, OutStream};

/// Mask bit: the translation follows.
pub const XFORM_TRANSLATION: u8 = 1;
/// Mask bit: the scale follows.
pub const XFORM_SCALE: u8 = 2;
/// Mask bit: the 3×3 matrix follows.
pub const XFORM_MATRIX: u8 = 4;

/// The identity rotation matrix, row by row.
pub const IDENTITY_MATRIX: [f32; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

/// A transform as stored in a record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xform {
    /// `+0`: the change counter (not saved, except by the legacy form).
    pub counter: u8,
    /// `+1`: the mask byte, written as stored (NO15).
    pub mask: u8,
    pub matrix: [f32; 9],
    pub translation: [f32; 3],
    pub scale: f32,
}

impl Default for Xform {
    fn default() -> Self {
        Xform {
            counter: 0,
            mask: 0,
            matrix: IDENTITY_MATRIX,
            translation: [0.0; 3],
            scale: 1.0,
        }
    }
}

impl Xform {
    /// Byte length of the stream form.
    pub fn encoded_len(&self) -> usize {
        let mut n = 1;
        if self.mask & XFORM_MATRIX != 0 {
            n += 36;
        }
        if self.mask & XFORM_TRANSLATION != 0 {
            n += 12;
        }
        if self.mask & XFORM_SCALE != 0 {
            n += 4;
        }
        n
    }

    /// `operator<<` (`0x005DA8C0`): the mask, then the parts it names in the
    /// order matrix, translation, scale (lot-records.md §2.1).
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u8(self.mask);
        if self.mask & XFORM_MATRIX != 0 {
            for &v in &self.matrix {
                out.put_f32(v);
            }
        }
        if self.mask & XFORM_TRANSLATION != 0 {
            for &v in &self.translation {
                out.put_f32(v);
            }
        }
        if self.mask & XFORM_SCALE != 0 {
            out.put_f32(self.scale);
        }
        out.error() == 0
    }

    /// Back to identity (rotation identity, translation 0, scale 1.0),
    /// mask 0, counter + 1.
    pub fn reset(&mut self) {
        self.matrix = IDENTITY_MATRIX;
        self.translation = [0.0; 3];
        self.scale = 1.0;
        self.mask = 0;
        self.counter = self.counter.wrapping_add(1);
    }

    /// Store a scale with the 1.0 test: bit 2 set when it differs from 1.0
    /// (a NaN differs), cleared when equal.
    fn set_scale_bit(&mut self, scale: f32) {
        self.scale = scale;
        if scale == 1.0 {
            self.mask &= !XFORM_SCALE;
        } else {
            self.mask |= XFORM_SCALE;
        }
    }

    /// `operator>>` (`0x005DA710`) into a live transform (NO15).
    pub fn read_into(&mut self, inp: &mut dyn InStream) -> Option<()> {
        let mask = inp.get_u8()?;
        if self.mask != 0 {
            self.reset();
        }
        if mask & XFORM_MATRIX != 0 {
            for v in self.matrix.iter_mut() {
                *v = inp.get_f32()?;
            }
            self.mask |= XFORM_MATRIX;
            self.counter = self.counter.wrapping_add(1);
        }
        if mask & XFORM_TRANSLATION != 0 {
            for v in self.translation.iter_mut() {
                *v = inp.get_f32()?;
            }
            self.mask |= XFORM_TRANSLATION;
            self.counter = self.counter.wrapping_add(1);
        }
        if mask & XFORM_SCALE != 0 {
            let s = inp.get_f32()?;
            self.set_scale_bit(s);
            self.counter = self.counter.wrapping_add(1);
        }
        Some(())
    }

    /// The current-form read into a fresh (identity, mask 0) transform.
    pub fn read(inp: &mut dyn InStream) -> Option<Xform> {
        let mut x = Xform::default();
        x.read_into(inp)?;
        Some(x)
    }

    /// `S3DTransformLegacyLoad` (`0x005DA930`, v4 records), 56 bytes: 9 f32
    /// rotation (mask |= 4), 3 f32 translation (mask |= 1), f32 scale (bit 2
    /// by the 1.0 test), u32 whose low byte replaces the counter. The mask is
    /// never reset first (NO15).
    pub fn legacy_load_into(&mut self, inp: &mut dyn InStream) -> Option<()> {
        for v in self.matrix.iter_mut() {
            *v = inp.get_f32()?;
        }
        self.mask |= XFORM_MATRIX;
        for v in self.translation.iter_mut() {
            *v = inp.get_f32()?;
        }
        self.mask |= XFORM_TRANSLATION;
        let s = inp.get_f32()?;
        self.set_scale_bit(s);
        self.counter = inp.get_u32()? as u8;
        Some(())
    }
}
