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
//! The in-memory field order (pollution.md, the visual-effect `SetTransform`: `cS3DTransform` is 0x38 bytes:
//! revision byte `+0`, flags byte `+1`, matrix `+0x04`, translation `+0x28`,
//! scale `+0x34`) is kept only as documentation; the core stores the parts.
//!
//! Not specified (question NO15): which mask `operator<<` writes for a given
//! live transform, and what an absent part holds after a read. The codec
//! keeps the mask as data and, literally, leaves an absent part at the
//! identity value (matrix identity, translation 0, scale 1).
//!
//! The legacy form `S3DTransformLegacyLoad` (`0x005DA930`, records below v5)
//! has no byte layout in the specs (question NO15); it is not implemented.

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
    /// The mask byte, written as stored (NO15).
    pub mask: u8,
    pub matrix: [f32; 9],
    pub translation: [f32; 3],
    pub scale: f32,
}

impl Default for Xform {
    fn default() -> Self {
        Xform {
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

    /// The current-form read (`0x005DA710`).
    pub fn read(inp: &mut dyn InStream) -> Option<Xform> {
        let mut x = Xform {
            mask: inp.get_u8()?,
            ..Xform::default()
        };
        if x.mask & XFORM_MATRIX != 0 {
            for v in x.matrix.iter_mut() {
                *v = inp.get_f32()?;
            }
        }
        if x.mask & XFORM_TRANSLATION != 0 {
            for v in x.translation.iter_mut() {
                *v = inp.get_f32()?;
            }
        }
        if x.mask & XFORM_SCALE != 0 {
            x.scale = inp.get_f32()?;
        }
        Some(x)
    }
}
