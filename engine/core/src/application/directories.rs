// SPDX-License-Identifier: GPL-3.0-or-later
//! Byte paths and lazy caches from application.md section 9. No filesystem access.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserFolder {
    Plugins,
    Regions,
    MySim,
    Albums,
    HttpCache,
    Temp,
    ExceptionReports,
    Scripts,
}

impl UserFolder {
    fn index(self) -> usize {
        self as usize
    }

    fn name(self) -> &'static [u8] {
        match self {
            Self::Plugins => b"Plugins",
            Self::Regions => b"Regions",
            Self::MySim => b"MySim",
            Self::Albums => b"Albums",
            Self::HttpCache => b"HTTPCache",
            Self::Temp => b"Temp",
            Self::ExceptionReports => b"Exception Reports",
            Self::Scripts => b"Scripts",
        }
    }
}

/// Stored paths keep the spec's offset names where export names disagree (AP3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Directories {
    pub data_7c: Vec<u8>,
    pub cd_data_90: Vec<u8>,
    pub plugin_a4: Vec<u8>,
    pub sku_cc: Vec<u8>,
    pub user_data_e0: Vec<u8>,
    pub slot_e4_108: Vec<u8>,
    pub user_base_11c: Vec<u8>,
    cache: [Vec<u8>; 8],
}

impl Directories {
    /// Negative index: unmodified base. Nonnegative: the selected language,
    /// or English when no language name is supplied (section 9).
    pub fn plugin_directory(&self, index: i32, language: Option<&[u8]>) -> Vec<u8> {
        let mut out = self.plugin_a4.clone();
        if index >= 0 {
            append_folder(&mut out, language.unwrap_or(b"English"));
        }
        out
    }

    /// Caches survive UserDir changes: the spec checks only whether each
    /// cached string is empty; it specifies no invalidation.
    pub fn user_folder(&mut self, folder: UserFolder) -> &[u8] {
        let index = folder.index();
        if self.cache[index].is_empty() {
            let mut path = if folder == UserFolder::Scripts {
                self.plugin_directory(-1, None)
            } else {
                self.user_base_11c.clone()
            };
            append_folder(&mut path, folder.name());
            self.cache[index] = path;
        }
        &self.cache[index]
    }
}

fn append_folder(base: &mut Vec<u8>, name: &[u8]) {
    // AP7: literal Windows folder append; avoid a doubled separator when
    // the supplied base already ends in either Windows separator.
    if !base.is_empty() && !matches!(base.last(), Some(b'\\' | b'/')) {
        base.push(b'\\');
    }
    base.extend_from_slice(name);
    base.push(b'\\');
}
