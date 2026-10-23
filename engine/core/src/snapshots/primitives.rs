// SPDX-License-Identifier: GPL-3.0-or-later
//! Primitive boundaries and usable, deterministic in-memory file/UI hosts.
use std::collections::{BTreeMap, BTreeSet};

use crate::paths::helpers::is_path_separator;
use crate::rz_files::host::{OS_ERROR_ACCESS_DENIED, OS_ERROR_ALREADY_EXISTS};

use super::{CaptureTarget, Error, Result};

/// Validated path in a virtual namespace. It cannot address the OS filesystem.
/// Exact UTF-8 component equality is the portable policy pending SN3/SN9.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VPath(Vec<String>);

impl VPath {
    pub fn new(path: &str) -> Result<Self> {
        if path.is_empty() {
            return Ok(Self(vec![]));
        }
        let components: Vec<_> = path.split('/').collect();
        if components.iter().any(|s| !valid_component(s)) {
            return Err(Error::InvalidPath);
        }
        Ok(Self(components.into_iter().map(str::to_owned).collect()))
    }

    pub fn join(&self, name: &str) -> Result<Self> {
        if !valid_component(name) {
            return Err(Error::InvalidPath);
        }
        let mut result = self.clone();
        result.0.push(name.to_owned());
        Ok(result)
    }

    pub fn parent(&self) -> Option<Self> {
        (!self.0.is_empty()).then(|| Self(self.0[..self.0.len() - 1].to_vec()))
    }

    pub fn file_name(&self) -> Option<&str> {
        self.0.last().map(String::as_str)
    }

    pub fn companion(&self, extension: &str) -> Result<Self> {
        if !matches!(extension, "info" | "txt") {
            return Err(Error::InvalidPath);
        }
        let name = self.file_name().ok_or(Error::InvalidPath)?;
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        self.parent()
            .ok_or(Error::InvalidPath)?
            .join(&format!("{stem}.{extension}"))
    }

    pub fn text(&self) -> String {
        self.0.join("/")
    }
}

pub(crate) fn valid_component(s: &str) -> bool {
    !s.is_empty() && !matches!(s, "." | "..") && !s.bytes().any(|b| b == 0 || is_path_separator(b) || b == b':')
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
}

/// Immediate-directory enumeration and byte IO only. No album filtering, naming,
/// ordering, metadata, selection or native owner methods may be implemented here.
pub trait FileSystem {
    fn entries(&mut self, directory: &VPath) -> Result<Vec<Entry>>;
    fn create_dir(&mut self, directory: &VPath) -> Result<()>;
    fn remove_dir(&mut self, directory: &VPath) -> Result<()>;
    fn read(&mut self, file: &VPath) -> Result<Option<Vec<u8>>>;
    /// May leave partial output on error (the capture workflow removes it).
    fn write(&mut self, file: &VPath, bytes: &[u8]) -> Result<()>;
    /// Missing files are a successful no-op, including optional companions.
    fn remove_file(&mut self, file: &VPath) -> Result<()>;
}

/// Production virtual filesystem; deliberately has no disk/import/export methods.
#[derive(Clone, Debug)]
pub struct MemoryFs {
    directories: BTreeSet<VPath>,
    files: BTreeMap<VPath, Vec<u8>>,
}

impl Default for MemoryFs {
    fn default() -> Self {
        Self {
            directories: BTreeSet::from([VPath(vec![])]),
            files: BTreeMap::new(),
        }
    }
}

impl FileSystem for MemoryFs {
    fn entries(&mut self, directory: &VPath) -> Result<Vec<Entry>> {
        if !self.directories.contains(directory) {
            return Err(Error::MissingFile);
        }
        let dirs = self
            .directories
            .iter()
            .filter(|p| p.parent().as_ref() == Some(directory))
            .map(|p| Entry {
                name: p.file_name().expect("non-root child").to_owned(),
                kind: EntryKind::Directory,
            });
        let files = self
            .files
            .keys()
            .filter(|p| p.parent().as_ref() == Some(directory))
            .map(|p| Entry {
                name: p.file_name().expect("non-root child").to_owned(),
                kind: EntryKind::File,
            });
        Ok(dirs.chain(files).collect())
    }

    fn create_dir(&mut self, directory: &VPath) -> Result<()> {
        if self.directories.contains(directory) {
            return Ok(());
        }
        if self.files.contains_key(directory) {
            return Err(Error::Io(OS_ERROR_ALREADY_EXISTS));
        }
        if !directory.parent().is_some_and(|p| self.directories.contains(&p)) {
            return Err(Error::MissingFile);
        }
        self.directories.insert(directory.clone());
        Ok(())
    }

    fn remove_dir(&mut self, directory: &VPath) -> Result<()> {
        if directory.0.is_empty() || !self.entries(directory)?.is_empty() {
            return Err(Error::Io(OS_ERROR_ACCESS_DENIED));
        }
        self.directories.remove(directory);
        Ok(())
    }

    fn read(&mut self, file: &VPath) -> Result<Option<Vec<u8>>> {
        if self.directories.contains(file) {
            return Err(Error::Io(OS_ERROR_ACCESS_DENIED));
        }
        Ok(self.files.get(file).cloned())
    }

    fn write(&mut self, file: &VPath, bytes: &[u8]) -> Result<()> {
        if self.directories.contains(file) {
            return Err(Error::Io(OS_ERROR_ACCESS_DENIED));
        }
        if !file.parent().is_some_and(|p| self.directories.contains(&p)) {
            return Err(Error::MissingFile);
        }
        self.files.insert(file.clone(), bytes.to_vec());
        Ok(())
    }

    fn remove_file(&mut self, file: &VPath) -> Result<()> {
        if self.directories.contains(file) {
            return Err(Error::Io(OS_ERROR_ACCESS_DENIED));
        }
        self.files.remove(file);
        Ok(())
    }
}

/// Raw simulation clock; language date formatting is a separate primitive.
pub trait SnapshotClock {
    fn simulation_day(&mut self) -> i32;
}

pub trait DateText {
    fn format_day(&mut self, day: i32) -> String;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageDepth {
    Bits16,
    Bits32,
    Other(u8),
}

/// Surface/codec primitive, never an album or filename provider. An opaque
/// operation belongs to the image boundary because SN8 leaves masks unpublished.
pub trait ImagePrimitives {
    type Image;
    fn capture(&mut self, target: CaptureTarget) -> Result<Self::Image>;
    fn depth(&self, image: &Self::Image) -> ImageDepth;
    fn force_opaque(&mut self, image: &mut Self::Image) -> Result<()>;
    fn encode_png(&mut self, image: &Self::Image) -> Result<Vec<u8>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zoom {
    In,
    Out,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiEvent {
    FrameVisible(bool),
    PostCapture {
        message: u32,
        counter: u8,
        close_afterwards: bool,
    },
    LuaEvent(u32),
}

/// Primitive UI effects. Lists, fields, count text, selection and modal state are
/// derived by the owners, not returned by a native snapshot window.
pub trait UiPrimitives {
    fn paused(&self) -> bool;
    fn set_paused(&mut self, paused: bool);
    fn set_album_visible(&mut self, visible: bool);
    fn set_frame_visible(&mut self, visible: bool);
    fn post_capture_message(&mut self, message: u32, counter: u8, close_afterwards: bool);
    fn raise_lua_event(&mut self, event: u32);
    fn zoom_picture(&mut self, direction: Zoom);
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryUi {
    pub paused: bool,
    pub album_visible: bool,
    pub frame_visible: bool,
    pub zoom_requests: Vec<Zoom>,
    pub events: Vec<UiEvent>,
}

impl UiPrimitives for MemoryUi {
    fn paused(&self) -> bool {
        self.paused
    }
    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }
    fn set_album_visible(&mut self, visible: bool) {
        self.album_visible = visible;
    }
    fn set_frame_visible(&mut self, visible: bool) {
        self.frame_visible = visible;
        self.events.push(UiEvent::FrameVisible(visible));
    }
    fn post_capture_message(&mut self, message: u32, counter: u8, close_afterwards: bool) {
        self.events.push(UiEvent::PostCapture {
            message,
            counter,
            close_afterwards,
        });
    }
    fn raise_lua_event(&mut self, event: u32) {
        self.events.push(UiEvent::LuaEvent(event));
    }
    fn zoom_picture(&mut self, direction: Zoom) {
        self.zoom_requests.push(direction);
    }
}
