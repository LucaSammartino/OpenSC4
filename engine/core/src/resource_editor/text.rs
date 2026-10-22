// SPDX-License-Identifier: GPL-3.0-or-later
use crate::number_text::rule_n;
use crate::variant::{mem, Elem, Variant, ARRAY};

use super::{EditorError, Ingredients};

/// Runtime conversion primitives, not editor behavior. In particular `atol32`
/// is the 32-bit C runtime call, not RZAtoi64A. A formats/native adapter can
/// supply its existing English parser without core depending on formats.
/// RE1/RE3: the spec has no default-parser range checks or CRT-overflow rule.
pub trait TextPrimitives {
    fn english_float32(&mut self, text: &str) -> f32;
    fn atol32(&mut self, text: &str) -> i32;
}

/// Formatting which §7 has not published. No guessed float/string/int64 or
/// malformed-pointer rendering is silently credited to cSCPropertyView (RE4).
pub trait UnpublishedFormatting {
    fn format(&mut self, property: u32, value: &Variant) -> Result<String, EditorError>;
}

pub fn decimal64(text: &str) -> i64 {
    let mut bytes = text.trim_start().bytes().peekable();
    let negative = bytes.peek() == Some(&b'-');
    if matches!(bytes.peek(), Some(b'-' | b'+')) { bytes.next(); }
    let value = bytes.take_while(u8::is_ascii_digit)
        .fold(0u64, |n, b| n.wrapping_mul(10).wrapping_add(u64::from(b - b'0')));
    if negative { (value as i64).wrapping_neg() } else { value as i64 }
}

/// §7's inferred item syntax: literal whitespace-separated items, exact count,
/// checked integer ranges, BOOL true/false or 0/1. RE1 asks for the actual
/// delimiters, coercions and range rules. No missing words are synthesized.
pub fn default_value(ty: u16, count: u32, text: &str, numbers: &mut impl TextPrimitives) -> Result<Variant, EditorError> {
    if count == 0 { return Ok(Variant::array(mem::UINT32, Vec::new())); }
    if ty == (ARRAY | mem::RZ_CHAR) {
        return if count == 1 { Ok(Variant::string(text.as_bytes())) } else { Err(EditorError::InvalidValue) };
    }
    let words: Vec<_> = text.split_ascii_whitespace().collect();
    if words.len() != count as usize { return Err(EditorError::InvalidValue); }
    let mut items = Vec::with_capacity(words.len());
    for word in words {
        let invalid = || EditorError::InvalidValue;
        // Validate the whole token for defaults; interactive grid reads are
        // intentionally prefix conversions instead (§11.2).
        let unsigned = || {
            let parsed = word.strip_prefix("0x").or_else(|| word.strip_prefix("0X"))
                .map_or_else(|| word.parse::<u32>(), |s| u32::from_str_radix(s, 16));
            parsed.map_err(|_| invalid())
        };
        let signed = || word.parse::<i64>().map_err(|_| invalid());
        let item = match ty {
            mem::BOOL => match word.to_ascii_lowercase().as_str() {
                "true" | "1" => Elem::Bool(true), "false" | "0" => Elem::Bool(false), _ => return Err(invalid()),
            },
            mem::UINT8 => Elem::U8(u8::try_from(unsigned()?).map_err(|_| invalid())?),
            mem::SINT8 => Elem::I8(i8::try_from(signed()?).map_err(|_| invalid())?),
            mem::UINT16 => Elem::U16(u16::try_from(unsigned()?).map_err(|_| invalid())?),
            mem::SINT16 => Elem::I16(i16::try_from(signed()?).map_err(|_| invalid())?),
            mem::UINT32 => Elem::U32(unsigned()?),
            mem::SINT32 => Elem::I32(i32::try_from(signed()?).map_err(|_| invalid())?),
            mem::SINT64 => Elem::I64(signed()?),
            mem::FLOAT32 => {
                word.parse::<f64>().map_err(|_| invalid())?;
                Elem::F32(numbers.english_float32(word))
            }
            _ => return Err(EditorError::UnsupportedType(ty)),
        };
        items.push(item);
    }
    Ok(if count == 1 { Variant::Scalar(items[0]) } else { Variant::array(ty, items) })
}

pub fn property_value_text(ingredients: &Ingredients, id: u32, value: &Variant,
    unknown: &mut impl UnpublishedFormatting) -> Result<String, EditorError> {
    if matches!(value, Variant::Array { items, .. } if items.is_empty()) {
        return Ok("<empty list>".into());
    }
    let mut text = Vec::new();
    for item in value.items() {
        // §3.3/§10: known map labels are used for combo item texts (RE2).
        let mapped = match item {
            Elem::U8(v) => ingredients.value_map(id, mem::UINT8).and_then(|m| m.get(&u32::from(v))),
            Elem::U32(v) => ingredients.value_map(id, mem::UINT32).and_then(|m| m.get(&v)),
            _ => None,
        };
        let next = if let Some(label) = mapped { label.clone() } else {
            match item {
                Elem::Bool(v) => if v { "True".into() } else { "False".into() },
                Elem::U8(v) => format!("0x{v:02X}"),
                Elem::I8(v) => format!("0x{:02X}", v as u8),
                Elem::U16(v) => format!("0x{v:04X}"),
                Elem::I16(v) => format!("0x{:04X}", v as u16),
                Elem::U32(v) => format!("0x{v:08X}"),
                Elem::I32(v) => format!("0x{:08X}", v as u32),
                _ => return unknown.format(id, value),
            }
        };
        text.push(next);
    }
    if text.is_empty() { unknown.format(id, value) } else { Ok(text.join(", ")) }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInput {
    pub child_id: u32,
    pub x: i32,
    pub y: i32,
    pub text: String,
}

/// §8.1: all eight typed layouts; only SINT64 GetValue is published.
pub struct TypedTextEditor {
    pub element_type: u16,
    pub value: Variant,
    pub inputs: Vec<TextInput>,
    initialized: bool,
}

impl TypedTextEditor {
    pub fn new(element_type: u16, value: Variant) -> Self {
        Self { element_type, value, inputs: Vec::new(), initialized: false }
    }

    pub fn init(&mut self, ingredients: &Ingredients, id: u32, unknown: &mut impl UnpublishedFormatting) -> Result<(), EditorError> {
        if self.initialized { return Ok(()); }
        let base: u32 = match self.element_type {
            mem::UINT8 | mem::SINT8 | mem::UINT32 | mem::SINT32 => 0x5A29_3904,
            mem::FLOAT32 => 0xDA4F_B6F6,
            mem::BOOL => 0x67C3_5A67,
            mem::RZ_CHAR => 0xF9E2_E9E4,
            mem::SINT64 => 0x490E_039A,
            ty => return Err(EditorError::UnsupportedType(ty)),
        };
        // A STRING is one input, not one input per RZChar byte.
        let values = if self.element_type == mem::RZ_CHAR {
            vec![self.value.clone()]
        } else { self.value.items().into_iter().map(Variant::Scalar).collect() };
        let mut inputs = Vec::with_capacity(values.len());
        for (i, value) in values.iter().enumerate() {
            let text = match value {
                Variant::Scalar(Elem::Bool(v)) => if *v { "true".into() } else { "false".into() },
                _ => property_value_text(ingredients, id, value, unknown)?,
            };
            inputs.push(TextInput { child_id: base.wrapping_add(i as u32), x: 5, y: 5 + 20 * i as i32, text });
        }
        self.inputs = inputs;
        self.initialized = true;
        Ok(())
    }

    pub fn read(&self, numbers: &mut impl TextPrimitives) -> Result<Variant, EditorError> {
        if self.element_type != mem::SINT64 {
            // §8.1 explicitly says the other GetValue bodies are unknown; RE3.
            return Err(EditorError::UnsupportedType(self.element_type));
        }
        if self.inputs.len() != self.value.count() { return Err(EditorError::MissingControl(self.inputs.len())); }
        if matches!(self.value, Variant::Scalar(Elem::I64(_))) {
            Ok(Variant::Scalar(Elem::I64(i64::from(numbers.atol32(&self.inputs[0].text)))))
        } else {
            Ok(Variant::array(mem::SINT64, self.inputs.iter().map(|c| Elem::I64(decimal64(&c.text))).collect()))
        }
    }
}

pub(crate) fn grid_parse(ty: u16, text: &str, numbers: &mut impl TextPrimitives) -> Result<Elem, EditorError> {
    Ok(match ty {
        mem::UINT8 => Elem::U8(rule_n(text.as_bytes()) as u8),
        mem::UINT16 => Elem::U16(rule_n(text.as_bytes()) as u16),
        mem::UINT32 => Elem::U32(rule_n(text.as_bytes())),
        mem::SINT8 => Elem::I8(decimal64(text) as i8),
        mem::SINT16 => Elem::I16(decimal64(text) as i16),
        mem::SINT32 => Elem::I32(decimal64(text) as i32),
        mem::FLOAT32 => Elem::F32(numbers.english_float32(text)),
        _ => return Err(EditorError::UnsupportedType(ty)),
    })
}
