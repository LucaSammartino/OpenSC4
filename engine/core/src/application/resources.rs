// SPDX-License-Identifier: GPL-3.0-or-later
//! Application resource registry and submission policy (application.md §14.2).
use super::{
    compatibility::{append_path, registry_entry},
    Application,
};
use crate::gz_messages::Token;

pub const SEGMENT_INTERFACE: u32 = 0x656B_8F0C;
pub const PACKED_FILE: u32 = 0x086D_4A89;
pub const DIRECTORY: u32 = 0x090F_8619;
pub const FACTORIES: [(u32, u32); 15] = [
    (0xCA63E2A3, 0x0A63DF8C),
    (0x2026960B, 0x00436EB4),
    (0x686AA4B0, 0xC8696797),
    (0x856DDBAC, 0xA2FFB5D3),
    (0x7AB50E44, 0x3AB50E2A),
    (0x7AB50E45, 0x3AB50E2A),
    (0x29A5D1EC, 0x69B6F01C),
    (0x09ADCD75, 0x69B6F01C),
    (0x5AD0E817, 0xFAD0F0B6),
    (0x6534284A, 0x453429B3),
    (0x05342861, 0x053429C8),
    (0xA83479D3, 0xA83479EA),
    (0x296678F7, 0x496678FE),
    (0xEA5118B0, 0xEA5118B5),
    (0xA2E3D533, 0x42E411C2),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub id: u32,
    pub class: u32,
    pub path: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registry {
    pub packed_exemplars: bool,
    pub packed_textures: bool,
    pub sources: Vec<Source>,
    /// Raw CN_DBSDF names and values, never interpreted as class/path entries.
    pub directory_files: Vec<(Vec<u8>, Vec<u8>)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: Vec<u8>,
    pub directory: bool,
}

/// Primitive calls only; enumeration returns native order and immediate children.
/// Factories/resource manager must exist, as required by the native precondition.
pub trait ResourceOwnerHost {
    fn registry_entries(&mut self, file: &[u8], section: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)>;
    fn directory_exists(&mut self, path: &[u8]) -> bool;
    fn create_directory(&mut self, path: &[u8]);
    fn enumerate(&mut self, path: &[u8], pattern: &[u8]) -> Vec<DirectoryEntry>;
    fn is_database_file(&mut self, path: &[u8], mode: u32) -> i32;
    fn create_segment(&mut self, class: u32, interface: u32) -> Option<Token>;
    fn initialize_segment(&mut self, segment: Token) -> bool;
    fn set_segment_path(&mut self, segment: Token, path: &[u8]) -> bool;
    fn open_segment(&mut self, segment: Token, mode: u32, flags: u32) -> bool;
    fn register_segment(&mut self, segment: Token, id: Option<u32>) -> bool;
    fn shutdown_segment(&mut self, segment: Token);
    fn release_segment(&mut self, segment: Token);
    fn register_factory(&mut self, resource_type: u32, class: u32, flags: u32);
    fn resource_count(&mut self) -> u32;
}

/// RZAtoUint32HexA: at most eight digits, no sign, trailing junk ignored.
pub fn hex_id(bytes: &[u8]) -> u32 {
    let bytes = &bytes[bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len())..];
    let bytes = if bytes.starts_with(b"0x") || bytes.starts_with(b"0X") {
        &bytes[2..]
    } else {
        bytes
    };
    let mut value = 0;
    for byte in bytes.iter().take(8) {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => break,
        };
        value = (value << 4) | u32::from(digit);
    }
    value
}

fn group_bit(name: &[u8]) -> u8 {
    match name {
        b"PACKED_EXEMPLARS" => 1,
        b"NON_PACKED_EXEMPLARS" => 2,
        b"PACKED_TEXTURES" => 4,
        b"NON_PACKED_TEXTURES" => 8,
        _ => 0,
    }
}

/// Unmatched END_GROUP has no native valid-input contract. Preserve this as a
/// typed invalid input, rather than silently accepting it or inventing recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnmatchedEndGroup;

impl Registry {
    pub fn load(
        &mut self,
        app: &Application,
        language: Option<&[u8]>,
        packed: [Option<&[u8]>; 2],
        host: &mut impl ResourceOwnerHost,
    ) -> Result<(), UnmatchedEndGroup> {
        for (input, flag) in [
            (packed[0], &mut self.packed_exemplars),
            (packed[1], &mut self.packed_textures),
        ] {
            if let Some(input) = input {
                if input.eq_ignore_ascii_case(b"on") {
                    *flag = true;
                } else if input.eq_ignore_ascii_case(b"off") {
                    *flag = false;
                }
            }
        }
        let file = append_path(&app.directories.data_7c, b"Sys\\Resources.ini");
        let mut stack = Vec::new();
        let mut bits = 0;
        for (name, value) in host.registry_entries(&file, b"DBSegments") {
            if name == b"BEGIN_GROUP" {
                let bit = group_bit(&value);
                stack.push(bit);
                bits |= bit;
            } else if name == b"END_GROUP" {
                bits &= !stack.pop().ok_or(UnmatchedEndGroup)?;
            } else if (bits & 1 == 0 || self.packed_exemplars)
                && (bits & 2 == 0 || !self.packed_exemplars)
                && (bits & 4 == 0 || self.packed_textures)
                && (bits & 8 == 0 || !self.packed_textures)
            {
                // No semicolon means an empty class token and the entire path.
                let entry =
                    registry_entry(&value, &app.directories.data_7c, language).expect("parser accepts every entry");
                self.sources.push(Source {
                    id: hex_id(&name),
                    class: hex_id(&entry.class),
                    path: entry.path,
                });
            }
        }
        self.directory_files
            .extend(host.registry_entries(&file, b"DBSegmentDirectoryFiles"));
        Ok(())
    }
}

fn submit(class: u32, id: Option<u32>, path: &[u8], host: &mut impl ResourceOwnerHost) {
    let Some(segment) = host.create_segment(class, SEGMENT_INTERFACE) else {
        return;
    };
    if host.initialize_segment(segment) {
        if !host.set_segment_path(segment, path)
            || !host.open_segment(segment, 1, 0)
            || !host.register_segment(segment, id)
        {
            host.shutdown_segment(segment);
        }
    }
    host.release_segment(segment);
}

fn scan(path: &[u8], recursive: bool, host: &mut impl ResourceOwnerHost) {
    if recursive && !host.directory_exists(path) {
        host.create_directory(path);
    }
    let entries = host.enumerate(path, if recursive { b"*" } else { b"*.dat" });
    for entry in entries.iter().filter(|entry| !entry.directory) {
        let file = append_path(path, &entry.name);
        if recursive {
            let extension = entry
                .name
                .iter()
                .rposition(|b| *b == b'.')
                .map(|dot| &entry.name[dot..]);
            if extension.is_some_and(|ext| ext.eq_ignore_ascii_case(b".dat")) || host.is_database_file(&file, 1) == -1 {
                continue;
            }
        }
        submit(PACKED_FILE, None, &file, host);
    }
    if recursive {
        for entry in entries
            .iter()
            .filter(|entry| entry.directory && entry.name != b"." && entry.name != b"..")
        {
            scan(&append_path(path, &entry.name), true, host);
        }
    }
}

impl Application {
    pub fn setup_resources(
        &mut self,
        registry: &Registry,
        language: Option<&[u8]>,
        host: &mut impl ResourceOwnerHost,
    ) -> bool {
        for source in &registry.sources {
            submit(source.class, Some(source.id), &source.path, host);
        }
        scan(&self.directories.plugin_directory(-1, language), false, host);
        scan(&self.directories.plugin_directory(0, language), false, host);
        submit(DIRECTORY, None, &self.directories.slot_e4_108, host);
        let roots = [
            self.directories.sku_cc.clone(),
            self.directories.user_folder(super::UserFolder::Plugins).to_vec(),
        ];
        for root in roots {
            scan(&root, true, host);
            submit(DIRECTORY, None, &root, host);
        }
        for (resource_type, class) in FACTORIES {
            host.register_factory(resource_type, class, 0);
        }
        host.resource_count() >= 3
    }
}
