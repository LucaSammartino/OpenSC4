// SPDX-License-Identifier: GPL-3.0-or-later
//! Font/window-control/cursor orchestration; application.md §14.2.
//! Resource factories, font parsing, rendering and the window system stay foreign.
use super::{compatibility::append_path, Application};
use crate::gz_messages::Token;

pub const COLOURS: [(u32, [u32; 3]); 19] = [
    (0x08963F6C, [255, 0, 255]),
    (0x68963F6F, [189, 188, 188]),
    (0xC8963F7A, [255, 255, 255]),
    (0x68963F7D, [0, 0, 0]),
    (0x28963FC5, [128, 0, 0]),
    (0xA8963F82, [0, 128, 0]),
    (0x48963F84, [128, 128, 64]),
    (0x68963F8A, [0, 0, 128]),
    (0xA8963F8C, [128, 0, 128]),
    (0x68963F8F, [0, 128, 128]),
    (0x08963F91, [128, 128, 128]),
    (0x28963F93, [192, 192, 192]),
    (0xE8963F95, [255, 0, 0]),
    (0x28963F98, [0, 255, 0]),
    (0xE8963F9A, [255, 255, 0]),
    (0xC8963FA2, [0, 0, 255]),
    (0x08963FA5, [0, 255, 255]),
    (0x28963FA8, [255, 255, 255]),
    (0xBB5B2EC6, [124, 144, 255]),
];
pub const PICTURES: [(u32, &[u32]); 15] = [
    (0x46A006A2, &[0xA8963EB8, 0xE8963EF1]),
    (0x144161EA, &[0x68963EBE, 0xA8963EF3]),
    (0xA, &[0xC8963EF6]),
    (0x82B99D9D, &[0x68963ED6]),
    (0x62B99D31, &[0x68963ED8]),
    (0x42E55FD4, &[0x48963EEE]),
    (0xE78FFC90, &[0xE8963EDF]),
    (0xC2B66DAA, &[0xE8963EE5]),
    (0x46A006A4, &[0xC8963ED3]),
    (0x46A006A6, &[0xE8963EC7]),
    (0x46A006A7, &[0x88963EC1]),
    (0x46A006A8, &[0xA8963EC4]),
    (0x46A006A5, &[0xA8963EEA]),
    (0xE2B14588, &[0xE2B14588]),
    (0x62B19CE9, &[0xE2B14589]),
];
pub const CAPTIONS: [(u32, u32); 7] = [
    (0x5301814C, 0x0A56B09E),
    (0x53018149, 0x8A56B0AF),
    (0x5301814A, 0x4A56B0B4),
    (0x53018147, 0x6A56B0BB),
    (0x53018146, 0x2A56B0BE),
    (0x5301814B, 0xAA56B0C3),
    (0x53018148, 0xCA56B0C8),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub resource_type: u32,
    pub group: u32,
    pub instance: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Query {
    FontService,
    ControlManager,
    Service {
        class: u32,
        interface: u32,
    },
    Create {
        class: u32,
        interface: u32,
    },
    SystemDirectory,
    LanguageName(i32),
    FileExists(Vec<u8>),
    AddFont(Vec<u8>),
    FontNames(u32),
    CreateFont {
        name: Vec<u8>,
        zero: u32,
        size: u32,
        weight: u32,
        flags: u32,
    },
    FontColour([u32; 4]),
    TempPath,
    Export(Key, Vec<u8>),
    IniEntries(Vec<u8>, &'static [u8]),
    StyleObject,
    ParseStyle(Vec<u8>),
    LoadPicture(Key),
    Setting(u32),
    CursorKeys(u32),
    CreateCursor(Key),
    LocalizedString(u32, u32),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reply {
    pub present: bool,
    pub token: Option<Token>,
    pub number: u32,
    pub text: Vec<u8>,
    pub names: Vec<Vec<u8>>,
    pub entries: Vec<(Vec<u8>, Vec<u8>)>,
    pub keys: Vec<Key>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    AddFontDirectory(Vec<u8>),
    RemoveFont(Vec<u8>),
    FontSetting(u32),
    InstallFontHelper(Token),
    Release(Token),
    Acquire(Token),
    Colour(u32, [u32; 3]),
    SetFontColour(Token, u32),
    BindFont(Token, u32),
    FontAlias(Vec<u8>, Vec<u8>),
    InstallNameTable(Token, Token),
    ClearGlobalNameTable,
    FontStyleName(u32, Vec<u8>),
    NameTableAdd(Token, Vec<u8>, u32),
    RemoveFile(Vec<u8>),
    BindPicture(Token, u32),
    Setting(u32, u32),
    RegisterCursor(Token, u32),
    Caption(u32, Vec<u8>),
}

pub trait UiHost {
    fn ui_query(&mut self, query: Query) -> Reply;
    fn ui_effect(&mut self, effect: Effect);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidInput {
    MissingControlManager,
}

pub fn cursor_group(setting: u32) -> u32 {
    match setting {
        0 => 0,
        1 => 1,
        2 => 4,
        3 => 8,
        4 => 32,
        _ => setting.wrapping_add(0x4A708EFB),
    }
}

impl Application {
    pub fn setup_font_set(&self, language: u32, host: &mut impl UiHost) {
        if !host.ui_query(Query::FontService).present {
            return;
        }
        host.ui_effect(Effect::AddFontDirectory(self.directories.sku_cc.clone()));
        host.ui_effect(Effect::AddFontDirectory(append_path(
            &self.directories.data_7c,
            b"Fonts",
        )));
        let additional: &[&[u8]] = match language {
            0x0F => &[b"mhei gb ", b"mhei b5", b"hy round gothic", b"cordiaupc"],
            0x11 => &[b"hg-gothic", b"mhei b5", b"hy round gothic", b"cordiaupc"],
            0x12 => &[b"hg-gothic", b"mhei gb", b"hy round gothic", b"cordiaupc"],
            0x13 => &[b"hg-gothic", b"mhei gb", b"mhei b5", b"hy round gothic"],
            0x14 => &[b"hg-gothic", b"mhei gb", b"mhei b5", b"cordiaupc"],
            _ => return,
        };
        let conditional = match language {
            0x13 => Some((b"micross.ttf".as_slice(), b"cordiaupc".as_slice())),
            0x14 => Some((b"Gulim.ttc".as_slice(), b"hy round gothic".as_slice())),
            _ => None,
        };
        let mut exclusions: Vec<&[u8]> = vec![b"arial mt", b"arta", b"courier ld", b"stoclet"];
        exclusions.extend_from_slice(additional);
        if let Some((file, name)) = conditional {
            let directory = host.ui_query(Query::SystemDirectory).text;
            let path = append_path(&directory, file);
            if host.ui_query(Query::FileExists(path.clone())).present && host.ui_query(Query::AddFont(path)).present {
                exclusions.push(name);
            }
        }
        let names = host.ui_query(Query::FontNames(64)).names;
        for name in names.into_iter().take(64) {
            let lowered = name.to_ascii_lowercase();
            for needle in &exclusions {
                if lowered.windows(needle.len()).any(|part| part == *needle) {
                    host.ui_effect(Effect::RemoveFont(name.clone()));
                }
            }
        }
    }

    pub fn setup_window_controls(
        &self,
        language: u32,
        custom: Option<&[u8]>,
        cursors: Option<&[u8]>,
        host: &mut impl UiHost,
    ) -> Result<(), InvalidInput> {
        let controls = host.ui_query(Query::ControlManager).present;
        if controls {
            for (id, rgb) in COLOURS {
                host.ui_effect(Effect::Colour(id, rgb));
            }
        }
        if host.ui_query(Query::FontService).present {
            host.ui_effect(Effect::FontSetting(3));
            if host
                .ui_query(Query::Service {
                    class: 0xEB2EA3CB,
                    interface: 0x4B2EA3B3,
                })
                .present
            {
                if let Some(helper) = host
                    .ui_query(Query::Create {
                        class: 0x8B3255F9,
                        interface: 0xA8BC2A75,
                    })
                    .token
                {
                    host.ui_effect(Effect::InstallFontHelper(helper));
                    host.ui_effect(Effect::Release(helper));
                }
            }
            self.setup_font_set(language, host);
            for (size, ids) in [
                (12, &[0x68963C4C, 0xC8963C5C, 0x68963C54, 0xA2CAE9DF][..]),
                (20, &[0x28963C70][..]),
            ] {
                if let Some(font) = host
                    .ui_query(Query::CreateFont {
                        name: b"Arta".to_vec(),
                        zero: 0,
                        size,
                        weight: 2,
                        flags: 1,
                    })
                    .token
                {
                    if !controls {
                        host.ui_effect(Effect::Release(font));
                        return Err(InvalidInput::MissingControlManager);
                    }
                    let colour = host.ui_query(Query::FontColour([0, 0, 0, 0x08000000])).number;
                    host.ui_effect(Effect::SetFontColour(font, colour));
                    for id in ids {
                        host.ui_effect(Effect::BindFont(font, *id));
                    }
                    host.ui_effect(Effect::Release(font));
                }
            }
            self.setup_font_styles(host);
        }
        if controls {
            for (instance, ids) in PICTURES {
                if let Some(picture) = host
                    .ui_query(Query::LoadPicture(Key {
                        resource_type: 0x856DDBAC,
                        group: 0x46A006B0,
                        instance,
                    }))
                    .token
                {
                    for id in ids {
                        host.ui_effect(Effect::BindPicture(picture, *id));
                    }
                    host.ui_effect(Effect::Release(picture));
                }
            }
        }
        self.setup_cursors(custom, cursors, host);
        if !controls {
            return Err(InvalidInput::MissingControlManager);
        }
        for (id, instance) in CAPTIONS {
            let text = host.ui_query(Query::LocalizedString(0x6A231EAA, instance)).text;
            host.ui_effect(Effect::Caption(id, text));
        }
        host.ui_effect(Effect::Caption(0x5377BE31, vec![0xE2, 0x80, 0xA6]));
        Ok(())
    }

    fn setup_font_styles(&self, host: &mut impl UiHost) {
        let mut path = append_path(&self.directories.sku_cc, b"FontStyle.ini");
        let mut temporary = false;
        if !host.ui_query(Query::FileExists(path.clone())).present {
            let language = host.ui_query(Query::LanguageName(0)).text;
            let language = if language.is_empty() {
                None
            } else {
                Some(language.as_slice())
            };
            path = append_path(&self.directories.plugin_directory(0, language), b"FontStyle.ini");
            if !host.ui_query(Query::FileExists(path.clone())).present {
                temporary = true;
                path = host.ui_query(Query::TempPath).text;
                if host
                    .ui_query(Query::Export(
                        Key {
                            resource_type: 0,
                            group: 0x4A87BFE8,
                            instance: 0x2A87BFFC,
                        },
                        path.clone(),
                    ))
                    .number
                    == u32::MAX
                {
                    path.clear();
                }
            }
        }
        if !path.is_empty() {
            for (name, value) in host.ui_query(Query::IniEntries(path.clone(), b"Font Aliases")).entries {
                host.ui_effect(Effect::FontAlias(value, name));
            }
            if let Some(object) = host.ui_query(Query::StyleObject).token {
                if let Some(table) = host
                    .ui_query(Query::Create {
                        class: 0xBA2E7954,
                        interface: 0x1A2BDEBB,
                    })
                    .token
                {
                    host.ui_effect(Effect::Acquire(object));
                    host.ui_effect(Effect::InstallNameTable(object, table));
                    for (name, value) in host.ui_query(Query::IniEntries(path.clone(), b"Font Styles")).entries {
                        let parsed = host.ui_query(Query::ParseStyle(value));
                        if let Some(font) = parsed.token {
                            host.ui_effect(Effect::BindFont(font, parsed.number));
                            host.ui_effect(Effect::FontStyleName(parsed.number, name.clone()));
                            host.ui_effect(Effect::NameTableAdd(table, name, parsed.number));
                            host.ui_effect(Effect::Release(font));
                        }
                    }
                    host.ui_effect(Effect::Release(table));
                    host.ui_effect(Effect::ClearGlobalNameTable);
                    host.ui_effect(Effect::Release(object));
                }
            }
        }
        if temporary {
            host.ui_effect(Effect::RemoveFile(path));
        }
    }

    pub fn setup_cursors(&self, custom: Option<&[u8]>, option: Option<&[u8]>, host: &mut impl UiHost) {
        let exclude = custom.is_some_and(|v| v.eq_ignore_ascii_case(b"off") || v.eq_ignore_ascii_case(b"disabled"));
        let setting = match option.map(|v| v.to_ascii_lowercase()).as_deref() {
            Some(b"off" | b"disabled") => Some(0),
            Some(b"bw") => Some(1),
            Some(b"color16") => Some(2),
            Some(b"color" | b"color256") => Some(3),
            // The native comparison is mixed-case after lowercasing: unreachable.
            _ => None,
        };
        if let Some(setting) = setting {
            host.ui_effect(Effect::Setting(15, setting));
        }
        let group = cursor_group(host.ui_query(Query::Setting(15)).number);
        for key in host.ui_query(Query::CursorKeys(0xAA5C3144)).keys {
            if key.group != group
                || (exclude
                    && matches!(
                        key.instance,
                        1 | 3 | 15..=18 | 0x43401C71 | 0x68024B0E | 0x88024B02 | 0xC8024B17 | 0xE8024B13
                    ))
            {
                continue;
            }
            if let Some(cursor) = host.ui_query(Query::CreateCursor(key)).token {
                host.ui_effect(Effect::RegisterCursor(cursor, key.instance));
                host.ui_effect(Effect::Release(cursor));
            }
        }
    }
}
