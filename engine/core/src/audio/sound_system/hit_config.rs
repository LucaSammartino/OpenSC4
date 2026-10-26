// SPDX-License-Identifier: GPL-3.0-or-later
//! §5.1 HIT initialization. INI lexing, COM operations and filesystem IO are foreign.
use super::hit::{HitIniFile, IniFile, IniServices};
use super::Unsupported;
use std::collections::{BTreeMap, BTreeSet};

/// RZConvertToUint32A, including base selection before whitespace/sign handling.
pub fn unsigned_number(text: &str) -> u32 {
    let bytes = text.as_bytes();
    let hex = bytes.starts_with(b"0x")
        || bytes.starts_with(b"0X")
        || bytes
            .iter()
            .take_while(|c| c.is_ascii_hexdigit())
            .any(|c| c.is_ascii_alphabetic());
    let mut value = text.trim_start();
    let negative = value.starts_with('-');
    if value.starts_with(['-', '+']) {
        value = &value[1..];
    }
    if hex && (value.starts_with("0x") || value.starts_with("0X")) {
        value = &value[2..];
    }
    let radix = if hex { 16 } else { 10 };
    let mut result = 0u32;
    let mut overflow = false;
    for digit in value.chars().map_while(|c| c.to_digit(radix)) {
        match result.checked_mul(radix).and_then(|n| n.checked_add(digit)) {
            Some(n) => result = n,
            None => overflow = true,
        }
    }
    if overflow {
        u32::MAX
    } else if negative {
        result.wrapping_neg()
    } else {
        result
    }
}

pub fn split_token(text: &mut String, delimiter: char) -> String {
    match text.find(delimiter) {
        Some(index) => {
            let token = text[..index].to_owned();
            text.drain(..index + delimiter.len_utf8());
            token
        }
        None => String::new(),
    }
}

fn comma_tokens(mut text: String) -> Vec<String> {
    let mut result = Vec::new();
    loop {
        let token = split_token(&mut text, ',');
        if token.is_empty() {
            result.push(text);
            break;
        }
        result.push(token);
    }
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IniAction {
    Packed {
        group: u32,
        class: u32,
        path: String,
    },
    DirectoryCreate {
        group: u32,
        class: u32,
        allow_text: bool,
    },
    DirectoryBind {
        group: u32,
        kind: u32,
        extension: String,
    },
    DirectoryInit {
        group: u32,
        path: String,
    },
    TypeGroup {
        kind: u32,
        group: u32,
    },
    LoadTrack(String),
    DataCreator {
        class: u32,
        creator: u32,
    },
    DataDefinition {
        creator: u32,
        sound: u32,
        definition: String,
    },
    SoundCreator {
        class: u32,
    },
    SoundDefinition {
        class: u32,
        sound: u32,
        definition: String,
    },
    ResourceFactory {
        kind: u32,
        class: u32,
    },
}

impl<I: IniFile> HitIniFile<I> {
    pub fn new(ini: I) -> Self {
        Self {
            ini,
            exclusions: BTreeSet::new(),
            allow_text_as_key: false,
            creator_sections: BTreeMap::new(),
        }
    }
    pub(super) fn numeric_entries(&mut self, section: &str) -> BTreeMap<u32, String> {
        self.ini
            .enum_entries(section)
            .into_iter()
            .map(|(key, value)| (unsigned_number(&key), value))
            .collect()
    }
    fn expand_path(path: &str, host: &dyn IniServices) -> String {
        let path = path.trim();
        path.strip_prefix("%DataDirectory%")
            .map_or_else(|| path.to_owned(), |tail| format!("{}{tail}", host.data_directory()))
    }
    pub fn enum_default_entries(&mut self, host: &mut dyn IniServices) -> Result<(), Unsupported> {
        self.init_defaults(host);
        for (group, mut value) in self.numeric_entries("PackedDbSegments") {
            let class = unsigned_number(&value);
            split_token(&mut value, ';');
            let section = split_token(&mut value, ';');
            let path = Self::expand_path(&value, host);
            if host.file_exists(&path) && host.apply(&IniAction::Packed { group, class, path }) {
                let covered = self.numeric_entries(&section);
                self.exclusions.extend(covered.into_keys());
            }
        }
        self.init_db_segments(host);
        for value in self.numeric_entries("TrackDataBlocks").into_values() {
            host.apply(&IniAction::LoadTrack(format!("{}{value}", host.data_directory())));
        }
        self.init_type_groups(host);
        // Creator callbacks preserve the inherited reader's file order/duplicates.
        for (section, mut value) in self.ini.enum_entries("SndDataSourceCreators") {
            let class = unsigned_number(&split_token(&mut value, ','));
            let creator = unsigned_number(&value);
            host.apply(&IniAction::DataCreator { class, creator });
            self.creator_sections.insert(creator, section);
        }
        for (creator, section) in self.creator_sections.clone() {
            for (sound, definition) in self.numeric_entries(&section) {
                host.apply(&IniAction::DataDefinition {
                    creator,
                    sound,
                    definition,
                });
            }
        }
        for (class, mut section) in self.ini.enum_entries("SndCreators") {
            let class = unsigned_number(&class);
            if section.contains(',') {
                section = split_token(&mut section, ',');
            }
            if host.apply(&IniAction::SoundCreator { class }) && !section.is_empty() {
                for (sound, definition) in self.ini.enum_entries(&section) {
                    host.apply(&IniAction::SoundDefinition {
                        class,
                        sound: unsigned_number(&sound),
                        definition,
                    });
                }
            }
        }
        self.init_resources(host);
        // Earlier local failures are intentionally not aggregated (§5.1).
        Ok(())
    }
    pub fn init_db_segments(&mut self, host: &mut dyn IniServices) -> bool {
        if let Some(value) = self.ini.read_entry("Config", "AllowTextAsKey") {
            self.allow_text_as_key = host.text_is_true(&value);
        }
        let segments = self.numeric_entries("DBSegments");
        for (&group, value) in &segments {
            if !self.exclusions.contains(&group) {
                host.apply(&IniAction::DirectoryCreate {
                    group,
                    class: unsigned_number(value),
                    allow_text: self.allow_text_as_key,
                });
            }
        }
        for (group, value) in self.numeric_entries("DBSegmentDirectoryFiles") {
            if self.exclusions.contains(&group) {
                continue;
            }
            for mut binding in comma_tokens(value) {
                let kind = unsigned_number(&split_token(&mut binding, ':'));
                host.apply(&IniAction::DirectoryBind {
                    group,
                    kind,
                    extension: binding,
                });
                host.apply(&IniAction::TypeGroup { kind, group });
            }
        }
        for (group, mut value) in segments {
            if !self.exclusions.contains(&group) {
                split_token(&mut value, ';');
                host.apply(&IniAction::DirectoryInit {
                    group,
                    path: Self::expand_path(&value, host),
                });
            }
        }
        true
    }
    pub fn init_type_groups(&mut self, host: &mut dyn IniServices) {
        for (kind, value) in self.numeric_entries("HitManTypeGroups") {
            for group in comma_tokens(value) {
                host.apply(&IniAction::TypeGroup {
                    kind,
                    group: unsigned_number(&group),
                });
            }
        }
    }
    pub fn init_resources(&mut self, host: &mut dyn IniServices) -> bool {
        for (kind, value) in self.numeric_entries("ResourceFactories") {
            host.apply(&IniAction::ResourceFactory {
                kind,
                class: unsigned_number(&value),
            });
        }
        true
    }
}
