// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! WR8's bounded normal caller domain. This is process storage, not defaults
//! installed on every instance. Foreign conversion and invalid backing remain explicit.
use crate::variant::Elem;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Bool,
    U32,
    F32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scalar {
    Bool(bool),
    U32(u32),
    F32(f32),
}

/// Pointer/count observations, represented by a safely borrowed backing slice.
/// Tags are deliberately independent: the raw-array helper does not convert elements.
#[derive(Clone, Copy, Debug)]
pub struct Property<'a> {
    pub tag: u16,
    pub scalar: Option<Elem>,
    pub backing: &'a [u8],
    pub count: usize,
}

pub trait RawTuningSource {
    fn acquire(&self, key: [u32; 3]) -> bool;
    fn property(&self, id: u32) -> Option<Property<'_>>;
    /// Only a single foreign primitive conversion, never owner state or arrays.
    fn foreign_scalar(&self, _property: u32, _target: Target) -> Result<Option<Scalar>, Unsupported> {
        Err(Unsupported::ForeignConversion)
    }
    fn release(&self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    InvalidBacking,
    PostTransformIndices,
    ForeignConversion,
    NumericDomain,
    FloatingMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadEvent {
    SetFlag,
    RegisterReset,
    Acquire(bool),
    Lookup(u32),
    Temporary(Target),
    DestroyTemporary(Target),
    Release,
}

/// Numeric mode is explicit; shared Elem::as_f64 implements the published 24-bit
/// unsigned correction. Other inherited modes are not silently assumed equivalent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericMode {
    Published24,
    Other,
}

#[derive(Clone, Copy, Debug)]
pub struct ReadArithmetic {
    pub mode: NumericMode,
}

/// §2.6/WR11: stored binary32 factor, not a host-computed pi/180.
pub const DEGREE_FACTOR_BITS: u32 = 0x3C8E_FA35;

#[derive(Clone, Debug, PartialEq)]
pub struct RetainedTuning {
    pub enabled: bool,
    pub minimum: u32,
    pub height: f32,
    pub period: u32,
    pub edit_area: u32,
    pub fade: u32,
    /// Suffix order: 02,04,05,06,0A,0B. Every copied element is retained.
    pub floats: [Vec<f32>; 6],
    pub bytes: [Vec<u8>; 2],
    pub step: [f32; 5],
    pub period_bounds: [u32; 2],
}

impl Default for RetainedTuning {
    fn default() -> Self {
        Self {
            enabled: true,
            minimum: 10,
            height: 2.0,
            period: 1_000_000,
            edit_area: 850,
            fade: 5,
            floats: Default::default(),
            bytes: Default::default(),
            step: [0.0; 5],
            period_bounds: [0; 2],
        }
    }
}

#[derive(Default, Debug)]
pub struct RippleProcess {
    pub tuning: RetainedTuning,
    /// `None` represents an unbound fixture, not executable startup. Use
    /// `startup` once per process, or inject a captured native RNG state.
    pub random: Option<super::mesh::NativeRandom>,
    pub render_state: Option<u32>,
    read: bool,
    pub events: Vec<ReadEvent>,
}

impl RippleProcess {
    /// §1.5: process static initialization precedes all ripple owners.
    pub fn startup(host: &mut dyn super::mesh::RandomHost) -> Self {
        Self {
            random: Some(super::mesh::NativeRandom::startup(host)),
            ..Self::default()
        }
    }
    pub fn was_read(&self) -> bool {
        self.read
    }
    pub fn reset(&mut self) -> bool {
        self.read = false;
        true
    }
    /// Call only after the instance's already-initialized gate. Flag and callback
    /// registration precede the read; failures do not roll back earlier mutations.
    pub fn eligible_init(
        &mut self,
        source: &dyn RawTuningSource,
        arithmetic: ReadArithmetic,
    ) -> Result<(), Unsupported> {
        if !self.read {
            self.read = true;
            self.events.push(ReadEvent::SetFlag);
            self.events.push(ReadEvent::RegisterReset);
            self.tuning.read(source, arithmetic, &mut self.events)?;
        }
        Ok(())
    }
}

fn numeric(value: Elem, target: Target, mode: NumericMode) -> Result<Scalar, Unsupported> {
    if matches!(value, Elem::VoidPtr(_)) {
        return Err(Unsupported::ForeignConversion);
    }
    match target {
        Target::Bool | Target::U32 => {
            let f = match value {
                Elem::F32(f) => Some(f64::from(f)),
                Elem::F64(f) => Some(f),
                _ => None,
            };
            if f.is_some_and(|f| !f.is_finite() || !(-9223372036854775808.0..9223372036854775808.0).contains(&f)) {
                return Err(Unsupported::NumericDomain);
            }
            let bits = value.as_i64();
            Ok(if target == Target::Bool {
                Scalar::Bool(bits != 0)
            } else {
                Scalar::U32(bits as u32)
            })
        }
        Target::F32 => {
            if mode != NumericMode::Published24 && matches!(value, Elem::U64(_) | Elem::U32(0x8000_0000..)) {
                return Err(Unsupported::FloatingMode);
            }
            Ok(Scalar::F32(value.as_f64() as f32))
        }
    }
}

impl RetainedTuning {
    pub fn read(
        &mut self,
        source: &dyn RawTuningSource,
        arithmetic: ReadArithmetic,
        events: &mut Vec<ReadEvent>,
    ) -> Result<(), Unsupported> {
        let acquired = source.acquire(super::RIPPLE_EXEMPLAR);
        events.push(ReadEvent::Acquire(acquired));
        if !acquired {
            return Ok(());
        }
        // Unsupported means the normal domain ended, not native helper false.
        // Therefore no fictional native release is emitted on an unchecked path.
        for suffix in 0..14 {
            let id = 0x491C_9C00 + suffix;
            events.push(ReadEvent::Lookup(id));
            let p = source.property(id);
            let target = match suffix {
                0 => Some(Target::Bool),
                1 | 7 | 12 | 13 => Some(Target::U32),
                3 => Some(Target::F32),
                _ => None,
            };
            if let (Some(p), Some(target)) = (p, target) {
                let direct_tag = match target {
                    Target::Bool => 1,
                    Target::U32 => 6,
                    Target::F32 => 10,
                };
                let temporary = p.tag != direct_tag;
                if temporary {
                    events.push(ReadEvent::Temporary(target));
                }
                let result = if (1..=14).contains(&p.tag) {
                    match p.scalar {
                        Some(Elem::Bool(v)) if !temporary => Ok(Some(Scalar::Bool(v))),
                        Some(Elem::U32(v)) if !temporary => Ok(Some(Scalar::U32(v))),
                        Some(Elem::F32(v)) if !temporary => Ok(Some(Scalar::F32(v))),
                        Some(value) if value.mem_type() == p.tag => numeric(value, target, arithmetic.mode).map(Some),
                        _ => Err(Unsupported::InvalidBacking),
                    }
                } else {
                    source.foreign_scalar(id, target)
                };
                if temporary {
                    events.push(ReadEvent::DestroyTemporary(target));
                }
                match (suffix, result?) {
                    (0, Some(Scalar::Bool(v))) => self.enabled = v,
                    (1, Some(Scalar::U32(v))) => self.minimum = v,
                    (3, Some(Scalar::F32(v))) => self.height = v,
                    (7, Some(Scalar::U32(v))) => self.period = v,
                    (12, Some(Scalar::U32(v))) => self.edit_area = v,
                    (13, Some(Scalar::U32(v))) => self.fade = v,
                    (_, None) => (),
                    _ => return Err(Unsupported::ForeignConversion),
                }
            } else if let Some(p) = p {
                if matches!(suffix, 8 | 9) {
                    let raw = p.backing.get(..p.count).ok_or(Unsupported::InvalidBacking)?;
                    self.bytes[(suffix - 8) as usize] = raw.to_vec();
                } else {
                    let index = match suffix {
                        2 => 0,
                        4 => 1,
                        5 => 2,
                        6 => 3,
                        10 => 4,
                        11 => 5,
                        _ => unreachable!(),
                    };
                    let len = p.count.checked_mul(4).ok_or(Unsupported::InvalidBacking)?;
                    let raw = p.backing.get(..len).ok_or(Unsupported::InvalidBacking)?;
                    self.floats[index] = raw
                        .chunks_exact(4)
                        .map(|b| f32::from_bits(u32::from_le_bytes(b.try_into().unwrap())))
                        .collect();
                }
            }
            match suffix {
                1 => self.minimum = self.minimum.max(2),
                2 => {
                    let first = self.floats[0].get_mut(..5).ok_or(Unsupported::PostTransformIndices)?;
                    for (step, v) in self.step.iter_mut().zip(first) {
                        *step = *v / 64.0;
                        *v *= *v;
                    }
                }
                7 => {
                    // Exact products in this bounded domain (24+24 <= 53 bits).
                    // Larger periods / inherited precision remain unsupported.
                    if self.period > (1 << 24) {
                        return Err(Unsupported::NumericDomain);
                    }
                    self.period_bounds = [
                        (f64::from(self.period) * f64::from(0.7_f32)).floor() as u32,
                        (f64::from(self.period) * f64::from(1.3_f32)).floor() as u32,
                    ];
                }
                11 => {
                    let first = self.floats[5].get_mut(..5).ok_or(Unsupported::PostTransformIndices)?;
                    for v in first {
                        *v *= f32::from_bits(DEGREE_FACTOR_BITS);
                    }
                }
                _ => (),
            }
        }
        source.release();
        events.push(ReadEvent::Release);
        Ok(())
    }
}
