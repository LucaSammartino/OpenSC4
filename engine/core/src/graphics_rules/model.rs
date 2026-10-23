// SPDX-License-Identifier: GPL-3.0-or-later
//! Rule records and the ordered, typed property-service boundary (§§1, 4, 5).

use crate::render_properties::RenderProperties;

/// Only renderer property operations cross this boundary, never parser results.
pub trait Properties {
    fn find_bool(&self, name: &str) -> Option<u32>;
    fn find_int(&self, name: &str) -> Option<u32>;
    fn find_float(&self, name: &str) -> Option<u32>;
    fn get_bool(&self, index: u32) -> Option<bool>;
    fn set_bool(&mut self, index: u32, value: bool) -> bool;
    fn set_int(&mut self, index: u32, value: i32) -> bool;
    fn set_float(&mut self, index: u32, value: f32) -> bool;
    /// Ascending indices within each type; bools, then ints, then floats (§6).
    /// Names are foreign CRT bytes, not UTF-8; the logger applies C-string NUL termination.
    fn dump(&self) -> Vec<(Vec<u8>, PropertyValue)>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropertyValue {
    Bool(bool),
    Int(i32),
    Float(f32),
}

impl Properties for RenderProperties {
    fn find_bool(&self, name: &str) -> Option<u32> {
        u32::try_from(self.find_bool(name)).ok()
    }
    fn find_int(&self, name: &str) -> Option<u32> {
        u32::try_from(self.find_int(name)).ok()
    }
    fn find_float(&self, name: &str) -> Option<u32> {
        u32::try_from(self.find_float(name)).ok()
    }
    fn get_bool(&self, index: u32) -> Option<bool> {
        i32::try_from(index).ok().and_then(|i| self.get_bool(i))
    }
    fn set_bool(&mut self, index: u32, value: bool) -> bool {
        i32::try_from(index).is_ok_and(|i| self.set_bool(i, value))
    }
    fn set_int(&mut self, index: u32, value: i32) -> bool {
        i32::try_from(index).is_ok_and(|i| self.set_int(i, value))
    }
    fn set_float(&mut self, index: u32, value: f32) -> bool {
        i32::try_from(index).is_ok_and(|i| self.set_float(i, value))
    }
    fn dump(&self) -> Vec<(Vec<u8>, PropertyValue)> {
        self.bool_properties()
            .iter()
            .map(|p| (p.name_bytes().to_vec(), PropertyValue::Bool(*p.value())))
            .chain(
                self.int_properties()
                    .iter()
                    .map(|p| (p.name_bytes().to_vec(), PropertyValue::Int(*p.value()))),
            )
            .chain(
                self.float_properties()
                    .iter()
                    .map(|p| (p.name_bytes().to_vec(), PropertyValue::Float(*p.value()))),
            )
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PropertySet {
    pub bools: Vec<(u32, bool)>,
    pub ints: Vec<(u32, i32)>,
    pub floats: Vec<(u32, f32)>,
}

impl PropertySet {
    /// Do not coalesce writes or stop after a failed setter (§1).
    pub fn apply(&self, properties: &mut impl Properties) {
        for &(i, v) in &self.bools {
            properties.set_bool(i, v);
        }
        for &(i, v) in &self.ints {
            properties.set_int(i, v);
        }
        for &(i, v) in &self.floats {
            properties.set_float(i, v);
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OptionGroup {
    pub recommendation: u32,
    pub options: Vec<PropertySet>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub name: String,
    pub depth: u32,
    pub partial: bool,
    pub any: bool,
    pub flag_mask: u32,
    pub flag_expected: u32,
    pub lower: Vec<(usize, i32)>,
    pub upper: Vec<(usize, i32)>,
    pub positive: Vec<(usize, String)>,
    pub negative: Vec<(usize, String)>,
    pub properties: PropertySet,
    pub recommendations: Vec<(u32, u32)>,
}

impl Default for Rule {
    fn default() -> Self {
        Self {
            name: "unknown".into(),
            depth: 0,
            partial: false,
            any: false,
            flag_mask: 0,
            flag_expected: 0,
            lower: Vec::new(),
            upper: Vec::new(),
            positive: Vec::new(),
            negative: Vec::new(),
            properties: PropertySet::default(),
            recommendations: Vec::new(),
        }
    }
}

/// The shared lowercase service is allowed to supply non-ASCII behavior (§4.2).
/// It supplies only lowercase bytes, never a wildcard-match decision.
pub trait Lowercase {
    fn lowercase(&self, bytes: &[u8]) -> Result<Vec<u8>, MatchError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchError;

/// Explicit ASCII-only embedding: non-ASCII requires a real shared string service.
pub struct AsciiLowercase;
impl Lowercase for AsciiLowercase {
    fn lowercase(&self, bytes: &[u8]) -> Result<Vec<u8>, MatchError> {
        if !bytes.is_ascii() {
            return Err(MatchError);
        }
        Ok(bytes.to_ascii_lowercase())
    }
}

/// MatchPattern's third argument is zero: lowercase copies of *both* operands.
pub fn wildcard_match(value: &str, pattern: &str, strings: &impl Lowercase) -> Result<bool, MatchError> {
    let value = strings.lowercase(value.as_bytes())?;
    let pattern = strings.lowercase(pattern.as_bytes())?;
    let (mut v, mut p, mut star) = (0, 0, None);
    while v < value.len() {
        if p < pattern.len() && pattern[p] != b'*' && (pattern[p] == b'?' || pattern[p] == value[v]) {
            v += 1;
            p += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = Some((p, v));
            p += 1;
        } else if let Some((s, end)) = star {
            v = end + 1;
            p = s + 1;
            star = Some((s, v));
        } else {
            return Ok(false);
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    Ok(p == pattern.len())
}

impl Rule {
    pub fn matches(&self, env: &super::Environment, strings: &impl Lowercase) -> Result<bool, MatchError> {
        let agrees = (env.flags ^ self.flag_expected) & self.flag_mask;
        if self.any {
            if agrees != self.flag_mask {
                return Ok(true);
            }
            for &(i, bound) in &self.lower {
                if f64::from(env.numbers[i]) >= f64::from(bound) {
                    return Ok(true);
                }
            }
            for &(i, bound) in &self.upper {
                if f64::from(env.numbers[i]) <= f64::from(bound) {
                    return Ok(true);
                }
            }
            for (i, pattern) in &self.positive {
                if wildcard_match(&env.strings[*i], pattern, strings)? {
                    return Ok(true);
                }
            }
            for (i, pattern) in &self.negative {
                if !wildcard_match(&env.strings[*i], pattern, strings)? {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        if agrees != 0 {
            return Ok(false);
        }
        // GR5: literal §4.2 behavior; V1 contradicts it. NaN passes all mode.
        for &(i, bound) in &self.lower {
            if f64::from(env.numbers[i]) < f64::from(bound) {
                return Ok(false);
            }
        }
        for &(i, bound) in &self.upper {
            if f64::from(env.numbers[i]) > f64::from(bound) {
                return Ok(false);
            }
        }
        for (i, pattern) in &self.positive {
            if !wildcard_match(&env.strings[*i], pattern, strings)? {
                return Ok(false);
            }
        }
        for (i, pattern) in &self.negative {
            if wildcard_match(&env.strings[*i], pattern, strings)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// _ftol2 converts to signed 64-bit, then only EAX is retained (§4.2).
pub fn threshold_from_real(value: f64) -> i32 {
    if !value.is_finite() || !(-9223372036854775808.0..9223372036854775808.0).contains(&value) {
        return 0; // integer-indefinite's low word
    }
    value.trunc() as i64 as i32
}

pub(super) fn grow<T: Default + Clone>(values: &mut Vec<T>, index: u32) -> Result<(), String> {
    // GR6: reject index+1 overflow rather than reproduce undefined allocation behavior.
    let count = index.checked_add(1).ok_or("Index overflow")? as usize;
    if count > values.len() {
        values
            .try_reserve(count - values.len())
            .map_err(|_| "Allocation failed")?;
        values.resize(count, T::default());
    }
    Ok(())
}
