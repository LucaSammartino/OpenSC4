// SPDX-License-Identifier: GPL-3.0-or-later
use std::collections::BTreeMap;

use crate::number_text::rule_n;
use crate::rz_files::ini::{ini_lower, parse_line, IniLine};
use crate::stream::ResKey;
use crate::variant::{mem, Variant, ARRAY};

use super::{default_value, EditorError, TextPrimitives};

pub const STRING_PARAMETER: u32 = 0x073B_6FD3;
pub const RESOURCE_TABLE_CLASS: u32 = 0xA784_7D6A;
pub const RESPONSE_CURVE_CLASS: u32 = 0xE8DD_A013;

pub fn variant_type(name: &str) -> u16 {
    match name.trim().to_ascii_uppercase().as_str() {
        "BOOL" => mem::BOOL,
        "UINT8" => mem::UINT8,
        "SINT8" => mem::SINT8,
        "UINT16" => mem::UINT16,
        "SINT16" => mem::SINT16,
        "UINT32" => mem::UINT32,
        "SINT32" => mem::SINT32,
        "SINT64" => mem::SINT64,
        "FLOAT32" => mem::FLOAT32,
        "STRING" => ARRAY | mem::RZ_CHAR,
        _ => mem::EMPTY,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditorParameter {
    pub kind: u32,
    pub text: String,
    pub argument: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropertyInfo {
    pub id: u32,
    pub value_type: u16,
    pub count: u32,
    pub default: Variant,
    pub filter: u32,
    pub name: String,
    pub help: String,
    pub editor_class: u32,
    pub parameter: EditorParameter,
    pub last: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableDefaults {
    pub parameter: u32,
    pub key: ResKey,
}

/// One process's ingredient vocabulary; callers own the process lifetime.
#[derive(Clone, Debug, Default)]
pub struct Ingredients {
    pub aliases: BTreeMap<String, u32>,
    pub exemplar_filters: BTreeMap<u32, u32>,
    pub properties: BTreeMap<u32, PropertyInfo>,
    pub table_defaults: BTreeMap<u32, TableDefaults>,
    pub uint8_maps: BTreeMap<u32, BTreeMap<u32, String>>,
    pub uint32_maps: BTreeMap<u32, BTreeMap<u32, String>>,
}

/// Load-once state is explicit, rather than a hidden global shared between tests.
#[derive(Default)]
pub struct IngredientCache(Option<Ingredients>);

impl IngredientCache {
    pub fn load_once(&mut self, text: &str, numbers: &mut impl TextPrimitives) -> &Ingredients {
        self.0.get_or_insert_with(|| Ingredients::parse(text, numbers))
    }
}

impl Ingredients {
    pub fn filter_word(&self, word: &str) -> u32 {
        if word.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            rule_n(word.as_bytes())
        } else {
            self.aliases.get(word).copied().unwrap_or(0)
        }
    }

    pub fn filter(&self, text: &str) -> u32 {
        // §3.2: only space separates words; tabs are not separators.
        text.split(' ').fold(0, |mask, word| mask | self.filter_word(word))
    }

    pub fn name(&self, id: u32) -> String {
        self.properties
            .get(&id)
            .filter(|p| !p.name.is_empty())
            .map_or_else(|| format!("0x{id:08X}"), |p| p.name.clone())
    }

    pub fn help(&self, id: u32) -> &str {
        self.properties.get(&id).map_or("", |p| p.help.as_str())
    }

    pub fn variable_length(&self, id: u32) -> bool {
        self.properties
            .get(&id)
            .is_some_and(|p| p.count == 0 && p.editor_class != RESPONSE_CURVE_CLASS)
    }

    pub fn value_map(&self, id: u32, elem: u16) -> Option<&BTreeMap<u32, String>> {
        match elem {
            mem::UINT8 => self.uint8_maps.get(&id),
            mem::UINT32 => self.uint32_maps.get(&id),
            _ => None,
        }
    }

    pub fn parse(text: &str, numbers: &mut impl TextPrimitives) -> Self {
        let mut sections: BTreeMap<Vec<u8>, Vec<(String, String)>> = BTreeMap::new();
        let mut section = None;
        for line in text.lines() {
            match parse_line(line.as_bytes()) {
                IniLine::Header(name) => section = name.map(|s| ini_lower(&s)),
                IniLine::Entry { key, value } => {
                    if let Some(name) = &section {
                        sections.entry(name.clone()).or_default().push((
                            String::from_utf8_lossy(&key).into_owned(),
                            String::from_utf8_lossy(&value).into_owned(),
                        ));
                    }
                }
                IniLine::Ignored => {}
            }
        }
        let entries = |name: &str| sections.get(&ini_lower(name.as_bytes())).cloned().unwrap_or_default();
        let mut result = Self::default();
        // Callbacks run in section order, independent of physical file order.
        for (key, value) in entries("Aliases") {
            result.aliases.insert(value.trim().to_owned(), rule_n(key.as_bytes()));
        }
        for (key, value) in entries("ExemplarFilters") {
            result
                .exemplar_filters
                .insert(rule_n(key.as_bytes()), result.filter(&value));
        }
        for (key, value) in entries("Properties") {
            if let Ok(property) = result.parse_property(rule_n(key.as_bytes()), &value, numbers) {
                result.properties.insert(property.id, property);
            }
        }
        for (key, value) in entries("ResKeyTableEditorDefaults") {
            if value.trim_start().is_empty() {
                continue;
            }
            // RE1: short records are unspecified; never manufacture missing words.
            if let Ok(Variant::Array { items, .. }) = default_value(mem::UINT32, 4, &value, numbers) {
                result.table_defaults.insert(
                    rule_n(key.as_bytes()),
                    TableDefaults {
                        parameter: items[0].as_i64() as u32,
                        key: ResKey {
                            type_id: items[2].as_i64() as u32,
                            group: items[1].as_i64() as u32,
                            instance: items[3].as_i64() as u32,
                        },
                    },
                );
            }
        }
        for &id in result.properties.keys() {
            for (prefix, maps) in [("Uint8", &mut result.uint8_maps), ("Uint32", &mut result.uint32_maps)] {
                let mut map = BTreeMap::new();
                for (key, value) in entries(&format!("{prefix}ValueMap_0x{id:08X}")) {
                    // RE2: key narrowing and name trimming not specified; retain u32 keys and INI text.
                    map.insert(rule_n(key.as_bytes()), value);
                }
                if !map.is_empty() {
                    maps.insert(id, map);
                }
            }
        }
        result
    }

    pub fn parse_property(
        &self,
        id: u32,
        text: &str,
        numbers: &mut impl TextPrimitives,
    ) -> Result<PropertyInfo, EditorError> {
        let mut fields = Fields(text);
        let value_type = variant_type(&fields.read_field()?.0);
        if value_type == mem::EMPTY {
            return Err(EditorError::UnsupportedType(value_type));
        }
        let count = rule_n(fields.read_field()?.0.as_bytes());
        if value_type == ARRAY | mem::RZ_CHAR && count != 1 {
            return Err(EditorError::InvalidValue);
        }
        let default = default_value(value_type, count, &fields.read_field()?.0, numbers)?;
        let filter = self.filter(&fields.read_field()?.0);
        let name = fields.read_field()?.0;
        let help = fields.read_field()?.0;
        let editor_class = self.filter_word(fields.read_field()?.0.trim());
        let (arg, quoted) = fields.read_field()?;
        let mut parameter = EditorParameter::default();
        if quoted {
            parameter.kind = STRING_PARAMETER;
            parameter.text = arg;
        } else if arg.as_bytes().first().is_some_and(u8::is_ascii_digit) {
            parameter.kind = rule_n(arg.as_bytes());
        } else if let Some(&alias) = self.aliases.get(&arg) {
            parameter.kind = alias;
            parameter.argument = id;
        } else if let Some((prefix, number)) = arg.split_once('_') {
            if !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()) {
                parameter.kind = self.aliases.get(prefix).copied().unwrap_or(0);
                parameter.argument = rule_n(number.as_bytes());
            }
        }
        let last = self.filter_word(fields.read_field()?.0.trim());
        Ok(PropertyInfo {
            id,
            value_type,
            count,
            default,
            filter,
            name,
            help,
            editor_class,
            parameter,
            last,
        })
    }
}

/// §7 getUnquotedString: consumes a comma outside quotes, preserving quoted commas.
pub struct Fields<'a>(pub &'a str);

impl Fields<'_> {
    pub fn read_field(&mut self) -> Result<(String, bool), EditorError> {
        let rest = self.0.trim_start();
        if let Some(quoted) = rest.strip_prefix('"') {
            let end = quoted.find('"').ok_or(EditorError::InvalidField)?;
            let tail = quoted[end + 1..].trim_start();
            self.0 = tail.strip_prefix(',').unwrap_or(tail);
            Ok((quoted[..end].to_owned(), true))
        } else {
            let (field, tail) = rest.split_once(',').unwrap_or((rest, ""));
            self.0 = tail;
            Ok((field.to_owned(), false))
        }
    }
}
