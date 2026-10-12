// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The developer exemplar's tuning (§4.8): `ReadSimulatorTunableValues`,
//! the 19 desirability curves (§4.2, "delete" pattern), the stage curves
//! (§5.5) and the shipped exemplars (§4.9).

use std::collections::BTreeMap;

use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::variant::{mem, Elem, Variant};
use crate::x87::{r24, u32_to_float};

use super::{prop, Curve, CURVE_COUNT, CURVE_PROPS};

/// `0.01f` (`0x3C23D70A`).
pub(crate) const PCT: f64 = 0.009_999_999_776_482_582;

/// The developer's tuning and identity fields (§2, §4.8). Constructed with
/// the constructor's values ([`DeveloperTuning::constructed`]).
#[derive(Clone, Debug, PartialEq)]
pub struct DeveloperTuning {
    /// Property `0x21`.
    pub developer_type: u32,
    /// `0x87CD6300`: 1 R, 2 C, 3 I.
    pub target_zone: u8,
    /// `0x87CD6332`, `0x87CD6330`.
    pub purpose: u8,
    pub wealth: u8,
    /// `0x87CD6333` bytes 0 and 1.
    pub land_value_min: u8,
    pub land_value_max: u8,
    /// `0x87CD633F`.
    pub use_intrinsic_land_value: bool,
    /// `0x87CD6310` (constructor 64).
    pub tract_size: u8,
    /// The 19 desirability curves by [`Curve`] (absent = `None`).
    pub curves: [Option<ResponseCurve>; CURVE_COUNT],
    /// The stage curves (`0x87CD6360` of them, §5.5).
    pub stage_curves: Vec<ResponseCurve>,
    pub baseline: i32,
    pub growth_threshold: i32,
    pub decline_threshold: i32,
    /// `0x87CD6356` (constructor true; every read resets it to false).
    pub phantom_demand: bool,
    pub power_cap: u8,
    pub water_cap: u8,
    pub fire_cap: u8,
    pub park_cap: u8,
    /// Abandonment thresholds by key (§4.8, slot 24 inserts 0.0 for an
    /// unknown key).
    pub abandonment: BTreeMap<i32, f32>,
    pub monopoly_use: u8,
    /// `0x87CD63A0`: group → fraction (`SC4Percentage`).
    pub styles: BTreeMap<u32, f32>,
    pub subjective_min: Vec<i32>,
    pub subjective_max: Vec<i32>,
}

/// What a tuning read asks the simulator to do with the host (§4.8): the
/// source demand node and a tract size change, in that order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TuningRead {
    /// `0x87CD6301` when present: release the old node and fetch
    /// `GetDemand(id, 0x20000)`.
    pub source_demand: Option<u32>,
    /// `0x87CD6310` when present and different from the stored size.
    pub new_tract_size: Option<u8>,
}

/// The first value of a variant as an integer (the property getters convert
/// the first element, foundations §9; floats truncate).
fn first_i64(v: &Variant) -> Option<i64> {
    match v {
        Variant::Scalar(e) => Some(e.as_i64()),
        Variant::Array { elem, items } if *elem != mem::RZ_CHAR => items.first().map(Elem::as_i64),
        _ => None,
    }
}

/// The property's data as raw little-endian bytes ("raw" = the data
/// pointer, §4.8). Strings and objects give nothing.
pub(crate) fn raw_bytes(v: &Variant) -> Vec<u8> {
    let one = |e: &Elem, out: &mut Vec<u8>| match *e {
        Elem::Bool(b) => out.push(u8::from(b)),
        Elem::U8(x) | Elem::Char(x) | Elem::RzChar(x) => out.push(x),
        Elem::I8(x) => out.push(x as u8),
        Elem::U16(x) | Elem::UnicodeChar(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::I16(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::U32(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::I32(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::U64(x) | Elem::VoidPtr(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::I64(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::F32(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::F64(x) => out.extend_from_slice(&x.to_le_bytes()),
    };
    let mut out = Vec::new();
    match v {
        Variant::Scalar(e) => one(e, &mut out),
        Variant::Array { items, .. } => items.iter().for_each(|e| one(e, &mut out)),
        _ => {}
    }
    out
}

/// The numeric values of a variant as `f32` (curve points).
fn f32_values(v: &Variant) -> Option<Vec<f32>> {
    let items = match v {
        Variant::Scalar(e) => vec![*e],
        Variant::Array { elem, items } if *elem != mem::RZ_CHAR => items.clone(),
        _ => return None,
    };
    Some(
        items
            .iter()
            .map(|e| match *e {
                Elem::F32(x) => x,
                other => other.as_f64() as f32,
            })
            .collect(),
    )
}

/// A curve as the property-pointer loader builds it (§4.2): `count >> 1`
/// points; `None` when the property is not numeric.
fn load_curve(v: &Variant) -> Option<ResponseCurve> {
    let values = f32_values(v)?;
    Some(ResponseCurve::from_flat(&values).unwrap_or_default())
}

/// All values of a variant as `i32` (the `vector<int32>` getter).
fn i32_values(v: &Variant) -> Option<Vec<i32>> {
    match v {
        Variant::Scalar(e) => Some(vec![e.as_i64() as i32]),
        Variant::Array { elem, items } if *elem != mem::RZ_CHAR => {
            Some(items.iter().map(|e| e.as_i64() as i32).collect())
        }
        _ => None,
    }
}

impl DeveloperTuning {
    /// The constructor's values (§2).
    pub fn constructed() -> DeveloperTuning {
        DeveloperTuning {
            developer_type: 0,
            target_zone: 0,
            purpose: 0,
            wealth: 0,
            land_value_min: 0,
            land_value_max: 0,
            use_intrinsic_land_value: false,
            tract_size: 64,
            curves: Default::default(),
            stage_curves: Vec::new(),
            baseline: 80,
            growth_threshold: 80,
            decline_threshold: 80,
            phantom_demand: true,
            power_cap: 0,
            water_cap: 3,
            fire_cap: 3,
            park_cap: 6,
            abandonment: BTreeMap::new(),
            monopoly_use: 1,
            styles: BTreeMap::new(),
            subjective_min: Vec::new(),
            subjective_max: Vec::new(),
        }
    }

    /// The curve of a layer, if present and not empty (§4.2 [choice]: an
    /// empty curve counts as absent).
    pub fn curve(&self, c: Curve) -> Option<&ResponseCurve> {
        self.curves[c as usize].as_ref().filter(|c| !c.is_empty())
    }

    /// `ReadSimulatorTunableValues` (§4.8) from the developer exemplar, in
    /// the original's order. Returns the two host-visible effects for the
    /// simulator to carry out.
    pub fn read(&mut self, h: &PropertyHolder) -> TuningRead {
        let mut effects = TuningRead::default();
        let int = |id: u32| h.get(id).and_then(first_i64);
        if let Some(v) = int(prop::DEVELOPER_TYPE) {
            self.developer_type = v as u32;
        }
        if let Some(v) = int(prop::TARGET_ZONE) {
            self.target_zone = v as u8;
        }
        if let Some(v) = int(prop::SOURCE_DEMAND) {
            effects.source_demand = Some(v as u32);
        }
        if let Some(v) = int(prop::TRACT_SIZE) {
            let s = v as u8;
            if s != self.tract_size {
                self.tract_size = s;
                effects.new_tract_size = Some(s);
            }
        }
        if let Some(v) = int(prop::WEALTH) {
            self.wealth = v as u8;
        }
        if let Some(v) = int(prop::PURPOSE) {
            self.purpose = v as u8;
        }
        // Raw bytes 0 and 1; [choice] fewer than two bytes keeps both (§13.1 quirk 9).
        if let Some(v) = h.get(prop::LAND_VALUE_RANGE) {
            if let [lo, hi, ..] = raw_bytes(v)[..] {
                self.land_value_min = lo;
                self.land_value_max = hi;
            }
        }
        self.use_intrinsic_land_value = int(prop::USE_INTRINSIC).is_some_and(|v| v != 0);
        // The 19 curves, "delete" pattern (§4.2).
        for (i, id) in CURVE_PROPS.iter().enumerate() {
            self.curves[i] = h.get(*id).and_then(load_curve);
        }
        self.baseline = int(prop::BASELINE).map_or(80, |v| v as i32);
        self.growth_threshold = int(prop::GROWTH_THRESHOLD).map_or(80, |v| v as i32);
        self.decline_threshold = int(prop::DECLINE_THRESHOLD).map_or(self.growth_threshold, |v| v as i32);
        self.phantom_demand = int(prop::PHANTOM_DEMAND).is_some_and(|v| v != 0);
        let k = int(prop::STAGE_COUNT).map_or(0, |v| v as u8) as usize;
        self.stage_curves.resize_with(k, ResponseCurve::default);
        for (i, c) in self.stage_curves.iter_mut().enumerate() {
            // Present → loaded in place; missing → the single point (0, 0).
            // [choice] A present property that gives an empty curve also
            // becomes (0, 0) (its evaluation would crash, §4.2).
            let loaded = h.get(prop::STAGE_CURVE_0 + i as u32).and_then(load_curve);
            *c = match loaded {
                Some(curve) if !curve.is_empty() => curve,
                _ => ResponseCurve::from_points(&[(0.0, 0.0)]),
            };
        }
        if let Some(v) = int(prop::POWER_CAP) {
            self.power_cap = v as u8;
        }
        if let Some(v) = int(prop::WATER_CAP) {
            self.water_cap = v as u8;
        }
        if let Some(v) = int(prop::FIRE_CAP) {
            self.fire_cap = v as u8;
        }
        if let Some(v) = int(prop::PARK_CAP) {
            self.park_cap = v as u8;
        }
        let default_threshold = f32::from_bits(0x3E4C_CCCD);
        for key in 1..=3 {
            self.abandonment.insert(key, default_threshold);
        }
        if let Some(v) = h.get(prop::ABANDONMENT) {
            // Raw floats [0], [1], [2]; [choice] values beyond the data keep 0.2.
            let bytes = raw_bytes(v);
            for (i, c) in bytes.chunks_exact(4).take(3).enumerate() {
                let f = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                self.abandonment.insert(i as i32 + 1, f);
            }
        }
        self.monopoly_use = int(prop::MONOPOLY).map_or(1, |v| v as u8);
        self.styles.clear();
        if let Some(v) = h.get(prop::STYLES) {
            let values: Vec<u32> = match v {
                Variant::Scalar(e) => vec![e.as_i64() as u32],
                Variant::Array { items, .. } => items.iter().map(|e| e.as_i64() as u32).collect(),
                _ => Vec::new(),
            };
            // [choice] An odd count drops the last group (§13.1 quirk 9).
            for pair in values.chunks_exact(2) {
                self.styles.insert(pair[0], style_fraction(pair[1]));
            }
        }
        if let Some(v) = h.get(prop::SUBJECTIVE_MIN).and_then(i32_values) {
            self.subjective_min = v;
        }
        if let Some(v) = h.get(prop::SUBJECTIVE_MAX).and_then(i32_values) {
            self.subjective_max = v;
        }
        effects
    }

    /// `GetAbandonmentThreshold(key)` (slot 24): inserts 0.0 for an unknown
    /// key (§10.1, [compat]).
    pub fn abandonment_threshold(&mut self, key: i32) -> f32 {
        *self.abandonment.entry(key).or_insert(0.0)
    }

    /// `GetSubjectiveThresholds(i)` (slot 30): each output from its vector
    /// when `0 ≤ i < size`, else −4 and 20.
    pub fn subjective_thresholds(&self, i: i32) -> (i32, i32) {
        let at = |v: &Vec<i32>, d: i32| usize::try_from(i).ok().and_then(|i| v.get(i).copied()).unwrap_or(d);
        (at(&self.subjective_min, -4), at(&self.subjective_max, 20))
    }
}

/// `r24(float(pct) × 0.01f)` with `pct` an unsigned 32-bit value (§4.8).
pub fn style_fraction(pct: u32) -> f32 {
    r24(u32_to_float(pct as i32) * PCT) as f32
}

/// A value of the shipped exemplar table ([`super::shipped`]).
#[derive(Clone, Copy, Debug)]
pub enum ShippedValue {
    U8(u8),
    U8s(&'static [u8]),
    U32(u32),
    U32s(&'static [u32]),
    I32(i32),
    I32s(&'static [i32]),
    F32s(&'static [f32]),
    Bool(bool),
    Str(&'static str),
}

impl ShippedValue {
    fn variant(self) -> Variant {
        match self {
            ShippedValue::U8(v) => Variant::Scalar(Elem::U8(v)),
            ShippedValue::U8s(v) => Variant::array(mem::UINT8, v.iter().map(|&x| Elem::U8(x)).collect()),
            ShippedValue::U32(v) => Variant::Scalar(Elem::U32(v)),
            ShippedValue::U32s(v) => Variant::array(mem::UINT32, v.iter().map(|&x| Elem::U32(x)).collect()),
            ShippedValue::I32(v) => Variant::Scalar(Elem::I32(v)),
            ShippedValue::I32s(v) => Variant::array(mem::SINT32, v.iter().map(|&x| Elem::I32(x)).collect()),
            ShippedValue::F32s(v) => Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect()),
            ShippedValue::Bool(v) => Variant::Scalar(Elem::Bool(v)),
            ShippedValue::Str(s) => Variant::string(s.as_bytes()),
        }
    }
}

/// The shipped developer exemplar of a developer type (§4.9), every
/// property as in `SimCity_1.dat`; `None` for other types.
pub fn shipped_exemplar(developer_type: u32) -> Option<PropertyHolder> {
    let (_, props) = super::shipped::ALL.iter().find(|(t, _)| *t == developer_type)?;
    let mut h = PropertyHolder::new();
    for (id, v) in props.iter() {
        h.add(*id, v.variant());
    }
    Some(h)
}

/// The shipped developer types in ascending order.
pub fn shipped_types() -> [u32; 12] {
    let mut out = [0u32; 12];
    for (o, (t, _)) in out.iter_mut().zip(super::shipped::ALL.iter()) {
        *o = *t;
    }
    out
}
