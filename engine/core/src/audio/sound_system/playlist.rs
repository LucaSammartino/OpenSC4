// SPDX-License-Identifier: GPL-3.0-or-later
//! cGZPlayList (§9). Only confirmed parsing; unresolved corners are SS11.
use super::Unsupported;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    ExtendedM3u,
    Pls,
    M3u,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub decimal_metadata: Option<i32>,
}
#[derive(Default)]
pub struct PlayList {
    pub entries: Vec<Entry>,
}
/// File IO, framework encoding conversion and C-library long conversion are
/// foreign. In particular, do not substitute Rust's strict decimal parser.
pub trait PlaylistInput {
    fn converted_lines(&mut self) -> Result<Vec<String>, Unsupported>;
    fn decimal_long(&mut self, text: &str) -> i32;
}
impl PlayList {
    /// Pure content selection. Plain M3U starts at line zero again.
    pub fn select_format(first_line: &str) -> (Format, usize) {
        match first_line.trim() {
            "#EXTM3U" => (Format::ExtendedM3u, 1),
            "[playlist]" => (Format::Pls, 1),
            _ => (Format::M3u, 0),
        }
    }
    /// Parser continuation appends without rollback. This deliberately does
    /// not claim Open's unsettled reset/append or file-failure behavior (SS11).
    pub fn parse_converted(&mut self, base: &str, input: &mut impl PlaylistInput) -> Result<bool, Unsupported> {
        let lines = input.converted_lines()?;
        let (format, mut position) = Self::select_format(lines.first().map_or("", String::as_str));
        match format {
            Format::Pls => Err(Unsupported("SS11")),
            Format::M3u => {
                for line in &lines[position..] {
                    let path = line.trim();
                    // Comments, URL handling and empty record behavior SS11.
                    if path.is_empty() || path.starts_with('#') || path.contains("://") {
                        return Err(Unsupported("SS11"));
                    }
                    self.entries.push(Entry {
                        path: Self::make_absolute_path(base, path)?,
                        decimal_metadata: None,
                    });
                }
                Ok(true)
            }
            Format::ExtendedM3u => {
                while position < lines.len() {
                    let header = lines[position].trim();
                    let Some(metadata) = header.strip_prefix("#EXTINF:") else {
                        return Ok(false);
                    };
                    position += 1;
                    let Some(path) = lines.get(position) else {
                        return Ok(false);
                    };
                    position += 1;
                    let metadata = input.decimal_long(metadata);
                    self.entries.push(Entry {
                        path: Self::make_absolute_path(base, path.trim())?,
                        decimal_metadata: Some(metadata),
                    });
                }
                Ok(true)
            }
        }
    }
    pub fn is_drive_spec(path: &str) -> bool {
        path.as_bytes().get(1) == Some(&b':')
    }
    pub fn make_absolute_path(base: &str, path: &str) -> Result<String, Unsupported> {
        // Explicit unsettled path cases; never use the host OS's normalization
        // for Windows playlists. Literal slash/backslash joining is SS12.
        if path.starts_with("\\\\")
            || path.starts_with("//")
            || path.contains("://")
            || path.is_empty()
            || path.split(['/', '\\']).any(|segment| segment == "." || segment == "..")
        {
            return Err(Unsupported("SS11"));
        }
        if Self::is_drive_spec(path) || path.starts_with(['/', '\\']) {
            return Ok(path.to_owned());
        }
        if base.is_empty() {
            return Ok(path.to_owned());
        }
        let separator = if base.contains('\\') { "\\" } else { "/" };
        Ok(format!("{}{}{}", base.trim_end_matches(['/', '\\']), separator, path))
    }
    pub fn open(&mut self, _filename: &str) -> Result<bool, Unsupported> {
        Err(Unsupported("SS11"))
    }
}
