// SPDX-License-Identifier: GPL-3.0-or-later
use super::ui::*;
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Call {
    Query(Query),
    Effect(Effect),
}
#[derive(Default)]
struct Foreign {
    fonts: bool,
    controls: bool,
    create_fonts: bool,
    setting: u32,
    names: Vec<Vec<u8>>,
    keys: Vec<Key>,
    replies: VecDeque<(Query, Reply)>,
    calls: Vec<Call>,
}
impl Foreign {
    fn reply(&mut self, q: Query, r: Reply) {
        self.replies.push_back((q, r));
    }
    fn effects(&self) -> Vec<Effect> {
        self.calls
            .iter()
            .filter_map(|c| match c {
                Call::Effect(e) => Some(e.clone()),
                _ => None,
            })
            .collect()
    }
}
impl UiHost for Foreign {
    fn ui_query(&mut self, query: Query) -> Reply {
        self.calls.push(Call::Query(query.clone()));
        if let Some(index) = self.replies.iter().position(|(q, _)| *q == query) {
            return self.replies.remove(index).unwrap().1;
        }
        match query {
            Query::FontService => present(self.fonts),
            Query::ControlManager => present(self.controls),
            Query::FontNames(64) => Reply {
                names: self.names.clone(),
                ..Reply::default()
            },
            Query::CreateFont {
                name,
                zero,
                size,
                weight,
                flags,
            } => {
                assert_eq!((name.as_slice(), zero, weight, flags), (b"Arta".as_slice(), 0, 2, 1));
                Reply {
                    token: self.create_fonts.then_some(u64::from(size)),
                    ..Reply::default()
                }
            }
            Query::Setting(15) => Reply {
                number: self.setting,
                ..Reply::default()
            },
            Query::CursorKeys(0xAA5C3144) => Reply {
                keys: self.keys.clone(),
                ..Reply::default()
            },
            Query::CreateCursor(key) => token(u64::from(key.instance) + 100),
            Query::LocalizedString(0x6A231EAA, instance) => Reply {
                text: instance.to_le_bytes().to_vec(),
                ..Reply::default()
            },
            _ => Reply::default(),
        }
    }
    fn ui_effect(&mut self, effect: Effect) {
        if let Effect::Setting(15, value) = effect {
            self.setting = value;
        }
        self.calls.push(Call::Effect(effect));
    }
}
fn present(value: bool) -> Reply {
    Reply {
        present: value,
        ..Reply::default()
    }
}
fn token(value: u64) -> Reply {
    Reply {
        token: Some(value),
        ..Reply::default()
    }
}
fn text(value: &[u8]) -> Reply {
    Reply {
        text: value.to_vec(),
        ..Reply::default()
    }
}

#[test]
fn font_filter_preserves_trailing_space_distinction_limits_snapshot_and_repeats_removals() {
    let mut app = super::tests::app();
    app.directories.sku_cc = b"S".to_vec();
    app.directories.data_7c = b"D".to_vec();
    let mut h = Foreign {
        fonts: true,
        names: vec![
            b"MHei GB".to_vec(),
            b"MHei GB Regular".to_vec(),
            b"Arta ARIAL MT".to_vec(),
        ],
        ..Foreign::default()
    };
    h.names.extend((3..64).map(|_| b"safe".to_vec()));
    h.names.push(b"Arta only record 65".to_vec());
    app.setup_font_set(0x0F, &mut h);
    assert_eq!(
        h.effects(),
        [
            Effect::AddFontDirectory(b"S".to_vec()),
            Effect::AddFontDirectory(b"D\\Fonts".to_vec()),
            Effect::RemoveFont(b"MHei GB Regular".to_vec()),
            Effect::RemoveFont(b"Arta ARIAL MT".to_vec()),
            Effect::RemoveFont(b"Arta ARIAL MT".to_vec())
        ]
    );
    assert_eq!(
        h.calls
            .iter()
            .filter(|c| **c == Call::Query(Query::FontNames(64)))
            .count(),
        1
    );
}

#[test]
fn conditional_font_is_offered_before_names_and_failed_add_keeps_conditional_names() {
    let app = super::tests::app();
    let mut h = Foreign {
        fonts: true,
        names: vec![b"cordiaupc".to_vec(), b"Arta".to_vec()],
        ..Foreign::default()
    };
    h.reply(Query::SystemDirectory, text(b"SYS"));
    h.reply(Query::FileExists(b"SYS\\micross.ttf".to_vec()), present(true));
    h.reply(Query::AddFont(b"SYS\\micross.ttf".to_vec()), present(false));
    app.setup_font_set(0x13, &mut h);
    assert_eq!(
        h.effects()
            .iter()
            .filter(|e| matches!(e, Effect::RemoveFont(_)))
            .collect::<Vec<_>>(),
        [&Effect::RemoveFont(b"Arta".to_vec())]
    );
    let add = h
        .calls
        .iter()
        .position(|c| *c == Call::Query(Query::AddFont(b"SYS\\micross.ttf".to_vec())))
        .unwrap();
    let names = h
        .calls
        .iter()
        .position(|c| *c == Call::Query(Query::FontNames(64)))
        .unwrap();
    assert!(add < names);
}

#[test]
fn control_colours_default_font_bindings_picture_skips_and_captions_keep_order() {
    let app = super::tests::app();
    let mut h = Foreign {
        fonts: true,
        controls: true,
        create_fonts: true,
        ..Foreign::default()
    };
    h.reply(
        Query::LoadPicture(Key {
            resource_type: 0x856DDBAC,
            group: 0x46A006B0,
            instance: 0x46A006A2,
        }),
        token(80),
    );
    h.reply(
        Query::LoadPicture(Key {
            resource_type: 0x856DDBAC,
            group: 0x46A006B0,
            instance: 10,
        }),
        token(81),
    );
    assert_eq!(app.setup_window_controls(1, None, None, &mut h), Ok(()));
    let effects = h.effects();
    let colours: Vec<_> = effects.iter().filter(|e| matches!(e, Effect::Colour(..))).collect();
    assert_eq!(colours.len(), 19);
    assert_eq!(colours[1], &Effect::Colour(0x68963F6F, [189, 188, 188]));
    assert_eq!(colours[18], &Effect::Colour(0xBB5B2EC6, [124, 144, 255]));
    let fonts: Vec<_> = effects
        .iter()
        .filter(|e| matches!(e, Effect::BindFont(..) | Effect::Release(12 | 20)))
        .collect();
    assert_eq!(
        fonts,
        [
            &Effect::BindFont(12, 0x68963C4C),
            &Effect::BindFont(12, 0xC8963C5C),
            &Effect::BindFont(12, 0x68963C54),
            &Effect::BindFont(12, 0xA2CAE9DF),
            &Effect::Release(12),
            &Effect::BindFont(20, 0x28963C70),
            &Effect::Release(20)
        ]
    );
    let pictures: Vec<_> = effects
        .iter()
        .filter(|e| matches!(e, Effect::BindPicture(..) | Effect::Release(80 | 81)))
        .collect();
    assert_eq!(
        pictures,
        [
            &Effect::BindPicture(80, 0xA8963EB8),
            &Effect::BindPicture(80, 0xE8963EF1),
            &Effect::Release(80),
            &Effect::BindPicture(81, 0xC8963EF6),
            &Effect::Release(81)
        ]
    );
    let captions: Vec<_> = effects.iter().filter(|e| matches!(e, Effect::Caption(..))).collect();
    assert_eq!(captions.len(), 8);
    assert_eq!(
        captions[0],
        &Effect::Caption(0x5301814C, 0x0A56B09Eu32.to_le_bytes().to_vec())
    );
    assert_eq!(captions[7], &Effect::Caption(0x5377BE31, vec![0xE2, 0x80, 0xA6]));
}

#[test]
fn failed_font_export_skips_callbacks_and_removes_cleared_path() {
    let app = super::tests::app();
    let mut h = Foreign {
        fonts: true,
        controls: true,
        ..Foreign::default()
    };
    h.reply(Query::TempPath, text(b"temporary"));
    h.reply(
        Query::Export(
            Key {
                resource_type: 0,
                group: 0x4A87BFE8,
                instance: 0x2A87BFFC,
            },
            b"temporary".to_vec(),
        ),
        Reply {
            number: u32::MAX,
            ..Reply::default()
        },
    );
    app.setup_window_controls(1, None, None, &mut h).unwrap();
    assert!(h.effects().contains(&Effect::RemoveFile(Vec::new())));
    assert!(!h.calls.iter().any(|c| matches!(c, Call::Query(Query::IniEntries(..)))));
}

#[test]
fn font_styles_aliases_precede_gate_and_parser_supplies_numeric_id_with_balanced_references() {
    let mut app = super::tests::app();
    app.directories.sku_cc = b"S".to_vec();
    let path = b"S\\FontStyle.ini".to_vec();
    let mut h = Foreign {
        fonts: true,
        controls: true,
        ..Foreign::default()
    };
    h.reply(Query::FileExists(path.clone()), present(true));
    h.reply(
        Query::IniEntries(path.clone(), b"Font Aliases"),
        Reply {
            entries: vec![(b"A".to_vec(), b"B".to_vec())],
            ..Reply::default()
        },
    );
    h.reply(Query::StyleObject, token(90));
    h.reply(
        Query::Create {
            class: 0xBA2E7954,
            interface: 0x1A2BDEBB,
        },
        token(91),
    );
    h.reply(
        Query::IniEntries(path.clone(), b"Font Styles"),
        Reply {
            entries: vec![
                (b"bad".to_vec(), b"invalid".to_vec()),
                (b"not-a-hex-id".to_vec(), b"valid".to_vec()),
            ],
            ..Reply::default()
        },
    );
    h.reply(
        Query::ParseStyle(b"valid".to_vec()),
        Reply {
            token: Some(92),
            number: 7,
            ..Reply::default()
        },
    );
    app.setup_window_controls(1, None, None, &mut h).unwrap();
    let effects = h.effects();
    let start = effects.iter().position(|e| matches!(e, Effect::FontAlias(..))).unwrap();
    assert_eq!(
        &effects[start..start + 10],
        [
            Effect::FontAlias(b"B".to_vec(), b"A".to_vec()),
            Effect::Acquire(90),
            Effect::InstallNameTable(90, 91),
            Effect::BindFont(92, 7),
            Effect::FontStyleName(7, b"not-a-hex-id".to_vec()),
            Effect::NameTableAdd(91, b"not-a-hex-id".to_vec(), 7),
            Effect::Release(92),
            Effect::Release(91),
            Effect::ClearGlobalNameTable,
            Effect::Release(90)
        ]
    );
    assert!(!effects.iter().any(|e| matches!(e, Effect::RemoveFile(_))));
    let mut h = Foreign {
        fonts: true,
        controls: true,
        ..Foreign::default()
    };
    h.reply(Query::FileExists(path.clone()), present(true));
    h.reply(
        Query::IniEntries(path, b"Font Aliases"),
        Reply {
            entries: vec![(b"A".to_vec(), b"B".to_vec())],
            ..Reply::default()
        },
    );
    app.setup_window_controls(1, None, None, &mut h).unwrap();
    assert!(h.effects().contains(&Effect::FontAlias(b"B".to_vec(), b"A".to_vec())));
    assert!(!h
        .calls
        .iter()
        .any(|c| matches!(c, Call::Query(Query::IniEntries(_, b"Font Styles")))));
}

#[test]
fn cursor_options_preserve_native_fullcolor_defect_and_filter_only_custom_instances() {
    let app = super::tests::app();
    for (option, setting, group) in [
        (b"OFF".as_slice(), 0, 0),
        (b"BW", 1, 1),
        (b"COLOR16", 2, 4),
        (b"COLOR256", 3, 8),
        (b"fullColor", 4, 32),
    ] {
        let key = Key {
            resource_type: 0xAA5C3144,
            group,
            instance: 2,
        };
        let mut h = Foreign {
            setting: 4,
            keys: vec![Key { instance: 1, ..key }, key, Key { group: 99, ..key }],
            ..Foreign::default()
        };
        app.setup_cursors(Some(b"DISABLED"), Some(option), &mut h);
        assert_eq!(h.setting, setting);
        assert_eq!(
            h.effects()
                .iter()
                .filter(|e| matches!(e, Effect::RegisterCursor(..)))
                .collect::<Vec<_>>(),
            [&Effect::RegisterCursor(102, 2)]
        );
        if option == b"fullColor" {
            assert!(!h.effects().iter().any(|e| matches!(e, Effect::Setting(..))));
        }
    }
    assert_eq!(cursor_group(5), 0x4A708F00);
    assert_eq!(cursor_group(u32::MAX), 0x4A708EFA);
}

#[test]
fn absent_font_service_does_not_skip_pictures_or_captions_and_missing_controls_is_invalid() {
    let app = super::tests::app();
    let mut h = Foreign {
        controls: true,
        ..Foreign::default()
    };
    app.setup_window_controls(1, None, None, &mut h).unwrap();
    assert_eq!(
        h.calls
            .iter()
            .filter(|c| matches!(c, Call::Query(Query::LoadPicture(_))))
            .count(),
        15
    );
    assert!(!h
        .calls
        .iter()
        .any(|c| matches!(c, Call::Query(Query::CreateFont { .. }))));
    assert_eq!(
        app.setup_window_controls(1, None, None, &mut Foreign::default()),
        Err(InvalidInput::MissingControlManager)
    );
}
