// SPDX-License-Identifier: GPL-3.0-or-later
//! cGZPlayList (§9). File IO and locale conversion remain foreign.
use super::Unsupported;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    ExtendedM3u,
    Pls,
    M3u,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub title: String,
    pub path: String,
    pub duration: i32,
}
#[derive(Default)]
pub struct PlayList {
    pub entries: Vec<Entry>,
    pub stored_path: String,
    pub directory: String,
    pub remaining_input: Vec<u8>,
}
/// File IO, framework encoding conversion and C-library long conversion are
/// foreign. In particular, do not substitute Rust's strict decimal parser.
pub trait PlaylistInput {
    fn converted_lines(&mut self) -> Result<Vec<String>, Unsupported>;
    fn decimal_long(&mut self, text: &str) -> i32;
}
pub trait PlaylistIo {
    fn resolve_path(&mut self, supplied: &str) -> Option<String>;
    fn open(&mut self, supplied: &str, a: u32, b: u32, c: u32) -> bool;
    fn size(&mut self) -> usize;
    fn allocate(&mut self, size: usize) -> Option<Vec<u8>>;
    fn read(&mut self, buffer: &mut [u8]) -> bool;
    fn close(&mut self);
    fn convert(&mut self, line: &mut Vec<u8>, source: i32, destination: u32) -> bool;
    fn decimal_long(&mut self, text: &str) -> i32;
}
trait Records {
    fn line(&mut self) -> Result<Option<String>, Unsupported>;
    fn decimal_long(&mut self, text: &str) -> i32;
    fn plain_first(&mut self, first: String) -> Result<Option<String>, Unsupported> {
        Ok(Some(first))
    }
}
struct Converted<'a, T> {
    lines: std::vec::IntoIter<String>,
    input: &'a mut T,
}
impl<T: PlaylistInput> Records for Converted<'_, T> {
    fn line(&mut self) -> Result<Option<String>, Unsupported> {
        Ok(self
            .lines
            .by_ref()
            .map(|line| line.trim().to_owned())
            .find(|line| !line.is_empty()))
    }
    fn decimal_long(&mut self, text: &str) -> i32 {
        self.input.decimal_long(text)
    }
}
struct Raw<'a> {
    saved: Vec<u8>,
    remaining: &'a mut Vec<u8>,
    io: &'a mut dyn PlaylistIo,
}
impl Records for Raw<'_> {
    fn plain_first(&mut self, _: String) -> Result<Option<String>, Unsupported> {
        *self.remaining = std::mem::take(&mut self.saved);
        self.line()
    }
    fn line(&mut self) -> Result<Option<String>, Unsupported> {
        PlayList::get_line(self.remaining, |line, source, destination| {
            self.io.convert(line, source, destination)
        })
        .map(|line| String::from_utf8(line).map_err(|_| Unsupported("SS11 runtime encoding bytes")))
        .transpose()
    }
    fn decimal_long(&mut self, text: &str) -> i32 {
        self.io.decimal_long(text)
    }
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
    /// Continuation for converted logical lines. Open uses the literal byte
    /// cursor, including its non-consuming final-line quirk.
    pub fn parse_converted(&mut self, base: &str, input: &mut impl PlaylistInput) -> Result<bool, Unsupported> {
        let lines = input.converted_lines()?;
        self.parse_records(
            base,
            &mut Converted {
                lines: lines.into_iter(),
                input,
            },
        )
    }
    fn parse_records(&mut self, base: &str, input: &mut dyn Records) -> Result<bool, Unsupported> {
        let Some(first) = input.line()? else {
            return Ok(true);
        };
        let (format, _) = Self::select_format(&first);
        match format {
            Format::Pls => self.parse_pls(base, input),
            Format::M3u => {
                let mut next = input.plain_first(first)?;
                while let Some(path) = next {
                    let filename = path.rsplit(['/', '\\']).next().unwrap_or(&path);
                    let title = filename.rsplit_once('.').map_or(filename, |(stem, _)| stem).to_owned();
                    self.entries.push(Entry {
                        title,
                        path: Self::make_absolute_path(base, &path)?,
                        duration: 0,
                    });
                    next = input.line()?;
                }
                Ok(true)
            }
            Format::ExtendedM3u => {
                while let Some(header) = input.line()? {
                    let Some(metadata) = header.strip_prefix("#EXTINF:") else {
                        return Ok(false);
                    };
                    let Some(path) = input.line()? else {
                        return Ok(false);
                    };
                    let (duration, title) = metadata.split_once(',').unwrap_or(("", metadata));
                    let duration = input.decimal_long(duration);
                    self.entries.push(Entry {
                        title: title.to_owned(),
                        path: Self::make_absolute_path(base, &path)?,
                        duration,
                    });
                }
                Ok(true)
            }
        }
    }
    fn parse_pls(&mut self, base: &str, input: &mut dyn Records) -> Result<bool, Unsupported> {
        let Some(mut line) = input.line()? else {
            return Ok(true);
        };
        let mut count = 0;
        loop {
            if line.starts_with("NumberOfEntries") {
                return Ok(input.decimal_long(line.get(16..).unwrap_or("")) == count);
            }
            if !line.starts_with("File") {
                let Some(next) = input.line()? else {
                    return Ok(false);
                };
                line = next;
                continue;
            }
            let path = Self::after_equals(&line).to_owned();
            let Some(mut title) = input.line()? else {
                return Ok(false);
            };
            let mut lookahead = None;
            let mut duration = 0;
            if title.starts_with("Title") {
                title = Self::after_equals(&title).to_owned();
            } else {
                lookahead = Some(title.clone());
            }
            if lookahead.is_none() {
                let Some(length) = input.line()? else {
                    return Ok(false);
                };
                if length.starts_with("Length") {
                    duration = input.decimal_long(Self::after_equals(&length));
                } else {
                    duration = input.decimal_long(&length);
                    lookahead = Some(length);
                }
            }
            while lookahead.as_ref().map_or(true, String::is_empty) {
                let Some(next) = input.line()? else {
                    return Ok(false);
                };
                let empty = next.is_empty();
                lookahead = Some(next);
                if !empty || path.starts_with("File") {
                    break;
                }
            }
            self.entries.push(Entry {
                title,
                path: Self::make_absolute_path(base, &path)?,
                duration,
            });
            count += 1;
            line = lookahead.unwrap_or_default();
        }
    }
    fn after_equals(text: &str) -> &str {
        text.split_once('=').map_or(text, |(_, tail)| tail)
    }
    pub fn is_drive_spec(path: &str) -> bool {
        path.as_bytes().get(1) == Some(&b':')
    }
    pub fn make_absolute_path(base: &str, path: &str) -> Result<String, Unsupported> {
        let mut result = base.to_owned();
        let mut remaining = path;
        while !remaining.is_empty() {
            let specification = (remaining.starts_with('\\') && remaining.len() >= 2) || Self::is_drive_spec(remaining);
            let search = if specification && remaining.starts_with('\\') {
                2
            } else {
                0
            };
            let end = remaining.as_bytes()[search..]
                .iter()
                .position(|byte| *byte == b'\\')
                .map_or(remaining.len(), |index| search + index + 1);
            let segment = &remaining[..end];
            remaining = &remaining[end..];
            if specification {
                result = segment.to_owned();
            } else if segment.starts_with('.') {
                let dots = segment.bytes().take_while(|byte| *byte == b'.').count();
                for _ in 1..dots {
                    if result.ends_with('\\') {
                        result.pop();
                    }
                    result.truncate(result.rfind('\\').map_or(0, |index| index + 1));
                }
            } else {
                result.push_str(segment);
            }
        }
        Ok(result)
    }
    /// Byte-level helper: an unterminated final line is deliberately not erased.
    pub fn get_line(input: &mut Vec<u8>, convert: impl FnOnce(&mut Vec<u8>, i32, u32) -> bool) -> Option<Vec<u8>> {
        let whitespace = |byte: &u8| matches!(byte, b' ' | b'\t' | b'\r' | b'\n');
        let start = input.iter().position(|byte| !whitespace(byte)).unwrap_or(input.len());
        input.drain(..start);
        if input.is_empty() {
            return None;
        }
        let end = input.iter().position(|byte| matches!(byte, b'\r' | b'\n'));
        let mut line = input[..end.unwrap_or(input.len())].to_vec();
        if let Some(end) = end {
            input.drain(..=end);
        }
        while line.last().is_some_and(whitespace) {
            line.pop();
        }
        convert(&mut line, -1, 8);
        Some(line)
    }
    pub fn open(&mut self, filename: &str, io: &mut dyn PlaylistIo) -> Result<bool, Unsupported> {
        self.stored_path = io.resolve_path(filename).unwrap_or_else(|| filename.to_owned());
        let end = self.stored_path.rfind(['/', '\\']).map_or_else(
            || if Self::is_drive_spec(&self.stored_path) { 2 } else { 0 },
            |index| index + 1,
        );
        self.directory = self.stored_path[..end].to_owned();
        if !io.open(filename, 1, 2, 1) {
            return Ok(false);
        }
        let size = io.size();
        self.remaining_input.resize(size, 0);
        let Some(mut buffer) = io.allocate(size) else {
            // SPEC-GAP: SS11 reserves the unchecked allocation outcome for runtime observation.
            io.close();
            return Err(Unsupported("SS11 allocation observation"));
        };
        let read = io.read(&mut buffer);
        io.close();
        if !read {
            return Ok(false);
        }
        if size == 0 {
            // SPEC-GAP: SS11 reserves the unchecked size-1 scan outcome for runtime observation.
            return Err(Unsupported("SS11 zero-byte observation"));
        }
        for byte in &mut buffer[..size - 1] {
            if *byte == 0 {
                *byte = b' ';
            }
        }
        self.remaining_input = buffer;
        let mut remaining = std::mem::take(&mut self.remaining_input);
        let result = self.parse_records(
            &self.directory.clone(),
            &mut Raw {
                saved: remaining.clone(),
                remaining: &mut remaining,
                io,
            },
        );
        self.remaining_input = remaining;
        result
    }
}
