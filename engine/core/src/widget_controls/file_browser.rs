// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable cGZWinFileBrowser state and string algorithms, win-filebrowser
//! §§1–7, 8.1.1–8.1.2, 8.2.1, 8.4, 8.6–8.7. Population/resource recipes
//! not yet implemented are not represented by foreign whole-owner callbacks.
use super::*;

mod operations;
pub use operations::{BrowserIo, Column, Enumeration, IconIo};

pub const IID: u32 = 0x3aa5_2e64;
pub const PRIMARY_SLOTS: usize = 107;
pub const WINDOW_SLOTS: usize = 151;
pub const FIRST_CHILD_ID: u32 = 0xbaa5_5d50;
pub const PARENT: u32 = 0xbaa8_8501;
pub const DIRECTORY: u32 = 0xbaa8_8502;
pub const FILE: u32 = 0xbaa8_8503;
pub const CUSTOM_FILE: u32 = 0xbaa8_8513;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub tag: u32,
    pub id: u32,
    pub name: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomEntry {
    pub interface: Option<Handle>,
    pub text: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub kind: u32,
    pub id: u32,
    pub full_path: Vec<u8>,
    pub directory: Vec<u8>,
    pub leaf: Vec<u8>,
}

/// Borrowed string-object identities, not copies of their current contents.
/// A native adapter translates these to the three persistent string objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionString {
    FullPath,
    Directory,
    Leaf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionDescriptor {
    pub kind: u32,
    pub id: u32,
    pub strings: [SelectionString; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refresh {
    DirectoryList,
    FileList,
}

/// Primitive path, grid, font and child operations only. Missing population,
/// resource, message and layout owner contracts remain in the core checklist.
pub trait BrowserForeign: Foreign {
    fn directory_exists(&mut self, path: &[u8]) -> bool;
    fn create_directory(&mut self, path: &[u8]) -> bool;
    fn split_directory(&mut self, path: &[u8], capacity: usize, component_bytes: usize) -> (i32, Vec<u8>);
    fn split_file(&mut self, path: &[u8]) -> [Vec<u8>; 4];
    fn path_separator(&mut self, byte: u8) -> bool;
    fn path_extension(&mut self, path: &[u8]) -> Vec<u8>;
    fn lowercase(&mut self, bytes: &mut [u8]);
    fn right_trim(&mut self, bytes: &mut Vec<u8>);
    fn window_id(&mut self, window: Handle) -> u32;
    fn set_window_flag(&mut self, window: Handle, mask: u32, value: bool);
    fn notify_path(&mut self, id: u32, window_id: u32, path: &[u8], fourth: u32);
    fn notify_selection(
        &mut self,
        id: u32,
        window_id: u32,
        descriptor: SelectionDescriptor,
        strings: &Selection,
        fourth: u32,
    );
    fn set_editor_text(&mut self, editor: Handle, text: &[u8]);
    fn grid_last_row(&mut self, grid: Handle) -> u32;
    fn grid_text(&mut self, grid: Handle, column: i32, row: i32) -> Option<Vec<u8>>;
    fn grid_scroll_to(&mut self, grid: Handle, row: i32);
    fn grid_selected(&mut self, grid: Handle, row: i32) -> bool;
    fn grid_select(&mut self, grid: Handle, row: i32);
    fn child_window(&mut self, control: Handle, accessor: u32) -> Handle;
    fn remove_child(&mut self, parent: Handle, child: Handle);
    fn set_button_buffer(&mut self, button: Handle, buffer: Option<Handle>);
    fn font_fit(&mut self, font: Handle, text: &[u8], width: i32, option: u32) -> usize;
    fn text_resource(&mut self, id: u32) -> Option<Vec<u8>>;
    fn font_rect(&mut self, font: Handle, text: &[u8]) -> Rect;
}

#[derive(Debug)]
pub struct FileBrowser {
    pub primary: Handle,
    pub flags: u32,
    pub children: [Option<Handle>; 13],
    pub enabled: [u8; 13],
    pub entries: Vec<Option<Entry>>,
    pub current: Vec<u8>,
    pub fallback: Vec<u8>,
    pub parent_label: Vec<u8>,
    pub new_directory_label: Vec<u8>,
    pub pending_word: u32,
    pub pending_name: Vec<u8>,
    pub filter_index: u32,
    pub filter_labels: Vec<Vec<u8>>,
    pub filter_patterns: Vec<Vec<Vec<u8>>>,
    /// Resolved combo interface; the incomplete AddControls table does not
    /// establish a retained-field index for this control (FBQ2).
    pub filter_combo: Option<Handle>,
    pub selection: Selection,
    pub selected_row: i32,
    pub dirty: bool,
    pub sort_words: [u32; 2],
    pub wildcard_option: u32,
    pub common_font: Option<Handle>,
    pub icons: [Option<Handle>; 7],
    pub button_buffers: [Option<Handle>; 4],
    pub name_column: Option<i32>,
    /// Required calls are recorded in order, never silently outsourced. Bodies
    /// of these refresh operations still need FBQ5's specified portions added.
    pub refresh_work: Vec<Refresh>,
    pub drive_records: Vec<Vec<u8>>,
    pub custom_groups: Vec<Vec<CustomEntry>>,
    pub style_fonts: [Option<Handle>; 4],
    pub style_words: [[u32; 8]; 4],
    pub extra_reference: Option<Handle>,
    /// Offset-keyed constructor facts whose public property meanings are not
    /// proved (FBQ4). These are values, not a native struct memory layout.
    pub raw_words: std::collections::BTreeMap<u32, u32>,
    pub raw_bytes: std::collections::BTreeMap<u32, u8>,
}

impl FileBrowser {
    /// Constructor creates no child or graphics resources (§8.6). Remaining
    /// raw dimension/style defaults are pending in the closure checklist.
    pub fn new(primary: Handle) -> Self {
        Self {
            primary,
            flags: 0xd,
            children: [None; 13],
            enabled: [1; 13],
            entries: Vec::new(),
            current: Vec::new(),
            fallback: Vec::new(),
            parent_label: b"..".to_vec(),
            new_directory_label: b"New Folder".to_vec(),
            pending_word: 0,
            pending_name: Vec::new(),
            filter_index: 0,
            filter_labels: Vec::new(),
            filter_patterns: Vec::new(),
            filter_combo: None,
            selection: Selection::default(),
            selected_row: 0,
            dirty: false,
            sort_words: [0, 2],
            wildcard_option: 0,
            common_font: None,
            icons: [None; 7],
            button_buffers: [None; 4],
            name_column: None,
            refresh_work: Vec::new(),
            drive_records: Vec::new(),
            custom_groups: Vec::new(),
            style_fonts: [None; 4],
            style_words: [[
                0x4000_0000,
                0x4000_0000,
                0x4000_0000,
                0x4000_0000,
                0x4000_0000,
                0x4000_0000,
                0,
                0x4000_0000,
            ]; 4],
            extra_reference: None,
            raw_words: [
                (0x198, 0),
                (0x19c, 0),
                (0x224, 20),
                (0x2e8, 20),
                (0x2ec, 20),
                (0x314, 20),
                (0x318, 20),
                (0x228, 0),
                (0x22c, 0),
                (0x234, 0),
                (0x300, 0),
                (0x304, 0),
                (0x230, 8),
                (0x30c, u32::MAX),
                (0x164, 0),
                (0x168, 0),
                (0x16c, 0),
                (0x1e4, 10),
                (0x1f0, 10),
                (0x1fc, 10),
                (0x208, 10),
                (0x214, 10),
                (0x1f8, u32::MAX),
                (0x204, u32::MAX),
                (0x210, u32::MAX),
                (0x130, 2),
                (0x1ec, 2),
                (0x220, 2),
                (0x1e0, 0),
                (0x21c, 1),
            ]
            .into_iter()
            .collect(),
            raw_bytes: [
                (0x15c, 0),
                (0x308, 0),
                (0x309, 0),
                (0x31c, 0),
                (0x1dc, 1),
                (0x1e8, 1),
                (0x218, 1),
                (0x1f4, 0),
                (0x200, 0),
                (0x20c, 0),
            ]
            .into_iter()
            .collect(),
        }
    }
    pub fn window(&self) -> Handle {
        self.primary + 4
    }
    pub fn query_interface(&self, iid: u32, output: &mut Option<Handle>, f: &mut impl Foreign) -> bool {
        if iid == IID {
            *output = Some(self.primary);
            f.add_ref(self.primary);
            true
        } else {
            f.base_query(self.window(), iid, output)
        }
    }
    pub fn set_current_directory(&mut self, path: &[u8], f: &mut impl BrowserForeign) -> bool {
        let temporary = path.to_vec();
        if !f.directory_exists(&temporary) {
            return false;
        }
        self.current = temporary;
        self.update_path(f);
        true
    }
    pub fn go_to_parent(&mut self, f: &mut impl BrowserForeign) -> bool {
        let (count, second) = f.split_directory(&self.current, 2, 260);
        if count <= 1 {
            return false;
        }
        self.set_current_directory(&second, f);
        true
    }
    pub fn update_path(&mut self, f: &mut impl BrowserForeign) -> bool {
        if !f.directory_exists(&self.current) {
            self.current.clone_from(&self.fallback);
        }
        let id = f.window_id(self.window());
        f.notify_path(0x7abe_7205, id, &self.current, 0);
        self.refresh_work.extend([Refresh::DirectoryList, Refresh::FileList]);
        true
    }
    pub fn get_row_of_entry_by_name(&self, name: &[u8], f: &mut impl BrowserForeign) -> i32 {
        let (Some(grid), Some(column)) = (self.children[5], self.name_column) else {
            return 0;
        };
        let mut bound = f.grid_last_row(grid).wrapping_add(1);
        if bound == 0 || bound == u32::MAX {
            bound = 1024;
        }
        for row in 0..bound as i32 {
            if let Some(text) = f.grid_text(grid, column, row) {
                if f.text_equal(&text, name) {
                    return row;
                }
            }
        }
        0
    }
    pub fn set_flags(&mut self, flags: u32, f: &mut impl BrowserForeign) {
        let changed = flags ^ self.flags;
        if changed == 0 {
            return;
        }
        self.flags = flags;
        if changed & 2 != 0 {
            if flags & 2 != 0 {
                f.set_window_flag(self.window(), 0x20000, false);
                f.set_window_flag(self.window(), 0x40000, false);
                f.set_window_flag(self.window(), 0x10000, true);
            } else {
                f.set_window_flag(self.window(), 0x20000, true);
                f.set_window_flag(self.window(), 0x40000, true);
            }
            f.invalidate(self.window());
        } else if changed & 0xd != 0 {
            self.update_path(f);
        }
    }
    pub fn add_flags(&mut self, mask: u32, f: &mut impl BrowserForeign) {
        if self.flags & mask == 0 {
            self.set_flags(self.flags | mask, f);
        }
    }
    pub fn clear_flags(&mut self, mask: u32, f: &mut impl BrowserForeign) {
        if self.flags & mask != 0 {
            self.set_flags(self.flags & !mask, f);
        }
    }
    pub fn any_flag(&self, mask: u32) -> u32 {
        u32::from(self.flags & mask != 0)
    }

    fn selection_from(&mut self, row: usize, f: &mut impl BrowserForeign) -> Result<bool> {
        let entry = self
            .entries
            .get(row)
            .ok_or(Unsupported::Precondition("selection row in range"))?;
        let Some(entry) = entry else { return Ok(false) };
        let kind = match entry.tag {
            PARENT => 0,
            DIRECTORY => 1,
            FILE | CUSTOM_FILE => 2,
            _ => return Ok(false),
        };
        let mut leaf = entry.name.clone();
        if kind != 2 && !leaf.last().is_some_and(|&byte| f.path_separator(byte)) {
            leaf.push(b'\\');
        }
        let mut full_path = self.current.clone();
        full_path.extend_from_slice(&leaf);
        self.selection = Selection {
            kind,
            id: if kind == 2 { entry.id } else { 0 },
            full_path,
            directory: self.current.clone(),
            leaf,
        };
        Ok(true)
    }
    pub fn descriptor(&self) -> SelectionDescriptor {
        SelectionDescriptor {
            kind: self.selection.kind,
            id: self.selection.id,
            strings: [
                SelectionString::FullPath,
                SelectionString::Directory,
                SelectionString::Leaf,
            ],
        }
    }
    pub fn borrowed_string(&self, identity: SelectionString) -> &[u8] {
        match identity {
            SelectionString::FullPath => &self.selection.full_path,
            SelectionString::Directory => &self.selection.directory,
            SelectionString::Leaf => &self.selection.leaf,
        }
    }
    fn notify_selection(&self, message: u32, f: &mut impl BrowserForeign) {
        let id = f.window_id(self.window());
        f.notify_selection(message, id, self.descriptor(), &self.selection, 0);
    }
    pub fn selection_change(&mut self, row: usize, f: &mut impl BrowserForeign) -> Result<bool> {
        if !self.selection_from(row, f)? {
            return Ok(false);
        }
        self.selected_row = row as i32;
        if self.selection.kind == 2 {
            if let Some(editor) = self.children[6] {
                f.set_editor_text(editor, &self.selection.leaf);
            }
        }
        self.notify_selection(0x7abe_7203, f);
        Ok(true)
    }
    pub fn selection_double_click(&mut self, row: usize, f: &mut impl BrowserForeign) -> Result<bool> {
        if !self.selection_from(row, f)? {
            return Ok(false);
        }
        self.notify_selection(0x7abe_7204, f);
        Ok(true)
    }
    pub fn file_grid_double_click(&mut self, row: u32, f: &mut impl BrowserForeign) -> bool {
        if row as usize >= self.entries.len() {
            return false;
        }
        if let Some(entry) = &self.entries[row as usize] {
            match entry.tag {
                PARENT => {
                    self.go_to_parent(f);
                }
                DIRECTORY => {
                    let mut path = self.current.clone();
                    path.extend_from_slice(&entry.name);
                    path.push(b'\\');
                    self.set_current_directory(&path, f);
                }
                _ => {}
            }
        }
        true
    }
    pub fn set_selection(&mut self, path: &[u8], f: &mut impl BrowserForeign) -> bool {
        let [drive, directory, basename, extension] = f.split_file(path);
        let directory = [drive, directory].concat();
        if !f.directory_exists(&directory) {
            return false;
        }
        let leaf = [basename, extension].concat();
        self.selection.kind = 2;
        self.selection.full_path = [&directory[..], &leaf[..]].concat();
        self.selection.leaf = leaf;
        self.selection.directory.clone_from(&directory);
        self.current = directory;
        self.mark_dirty(f);
        true
    }
    pub fn create_new_directory(&mut self, f: &mut impl BrowserForeign) -> bool {
        let mut leaf = self.new_directory_label.clone();
        let mut path = [&self.current[..], &leaf[..], b"\\"].concat();
        if f.directory_exists(&path) {
            for suffix in 2..=256 {
                leaf = self.new_directory_label.clone();
                leaf.extend_from_slice(format!(" ({suffix})").as_bytes());
                path = [&self.current[..], &leaf[..], b"\\"].concat();
                if !f.directory_exists(&path) {
                    break;
                }
            }
        }
        if !f.create_directory(&path) {
            return false;
        }
        self.refresh_work.extend([Refresh::DirectoryList, Refresh::FileList]);
        // Refresh bodies must execute before the grid lookup when this partial
        // state layer is integrated; no external population callback is used.
        if let Some(grid) = self.children[5] {
            let row = self.get_row_of_entry_by_name(&leaf, f);
            f.grid_scroll_to(grid, row);
            if !f.grid_selected(grid, row) {
                f.grid_select(grid, row);
            }
        }
        true
    }
    pub fn enable_child(&mut self, id: u32, value: u8, f: &mut impl BrowserForeign) {
        let index = id.wrapping_sub(FIRST_CHILD_ID) as usize;
        if index >= 13 || self.enabled[index] == value {
            return;
        }
        self.enabled[index] = value;
        if value != 0 || matches!(index, 1 | 5) {
            return;
        }
        if let Some(control) = self.children[index].take() {
            let accessor = if matches!(index, 0 | 6) { 0x14 } else { 0x0c };
            let window = f.child_window(control, accessor);
            f.remove_child(self.window(), window);
            f.release(control);
        }
    }
    pub fn set_icon(&mut self, index: u32, buffer: Option<Handle>, f: &mut impl Foreign) {
        if let Some(slot) = self.icons.get_mut(index as usize) {
            if *slot != buffer {
                replace_ref(slot, buffer, f);
            }
        }
    }
    pub fn set_button_buffer(&mut self, index: u32, buffer: Option<Handle>, f: &mut impl BrowserForeign) {
        let Some(slot) = self.button_buffers.get_mut(index as usize) else {
            return;
        };
        if *slot == buffer {
            return;
        }
        replace_ref(slot, buffer, f);
        if let Some(button) = self.children[[8, 9, 3, 4][index as usize]] {
            f.set_button_buffer(button, buffer);
        }
    }
    pub fn mark_dirty(&mut self, f: &mut impl Foreign) {
        self.dirty = true;
        f.invalidate(self.window());
    }
    pub fn set_pending(&mut self, word: u32, first: &[u8], second: &[u8], f: &mut impl Foreign) -> bool {
        self.pending_word = word;
        self.pending_name = [first, second].concat();
        self.mark_dirty(f);
        true
    }
    pub fn pending(&self, word: &mut u32, name: &mut Vec<u8>, _third: &mut u32) -> bool {
        *word = self.pending_word;
        name.clone_from(&self.pending_name);
        true
    }
    pub fn set_sort_words(&mut self, words: [u32; 2], f: &mut impl Foreign) -> bool {
        self.sort_words = words;
        self.mark_dirty(f);
        true
    }
    pub fn set_wildcard_option(&mut self, option: u32, f: &mut impl Foreign) -> bool {
        self.wildcard_option = option;
        self.mark_dirty(f);
        true
    }

    pub fn clear_file_entries(&mut self) -> bool {
        self.entries.clear();
        true
    }
    pub fn clear_drive_records(&mut self) -> bool {
        self.drive_records.clear();
        true
    }
    pub fn clear_custom_groups(&mut self, f: &mut impl Foreign) -> bool {
        for group in self.custom_groups.drain(..) {
            for entry in group {
                if let Some(interface) = entry.interface {
                    f.release(interface);
                }
            }
        }
        self.filter_index = 0;
        true
    }
    /// All inputs in retained fields carry owner-held references. The asymmetric
    /// style-font omission at +0x280 is native, not a symmetric-loop oversight.
    pub fn shutdown(&mut self, f: &mut impl Foreign) {
        self.clear_file_entries();
        self.clear_drive_records();
        self.clear_custom_groups(f);
        for child in &mut self.children {
            replace_ref(child, None, f);
        }
        replace_ref(&mut self.common_font, None, f);
        for icon in &mut self.icons {
            replace_ref(icon, None, f);
        }
        for buffer in &mut self.button_buffers {
            replace_ref(buffer, None, f);
        }
        replace_ref(&mut self.extra_reference, None, f);
        for index in [0, 1, 3] {
            replace_ref(&mut self.style_fonts[index], None, f);
        }
        f.base_shutdown(self.window());
    }
}

/// The original callback is a Boolean predicate. In particular two parent
/// entries both precede one another; do not feed that domain to a strict sort.
pub fn precedes(
    a: &Entry,
    b: &Entry,
    primary: u32,
    secondary: u32,
    directory_mode: u32,
    f: &mut impl BrowserForeign,
) -> bool {
    if a.tag == PARENT {
        return true;
    }
    if b.tag == PARENT {
        return false;
    }
    let (at, bt) = (a.tag & 0xf, b.tag & 0xf);
    if at != bt {
        return if directory_mode <= 1 { at < bt } else { at > bt };
    }
    let mut names = [a.name.clone(), b.name.clone()];
    let mut extensions = [f.path_extension(&names[0]), f.path_extension(&names[1])];
    for text in names.iter_mut().chain(extensions.iter_mut()) {
        f.lowercase(text);
    }
    if a.tag == DIRECTORY {
        return if matches!(directory_mode, 0 | 2) {
            names[0] < names[1]
        } else {
            names[0] > names[1]
        };
    }
    if !matches!(a.tag, FILE | CUSTOM_FILE) || primary > 3 {
        return false;
    }
    let key = if primary <= 1 { &names } else { &extensions };
    let mode = if key[0] == key[1] { secondary } else { primary };
    match mode {
        0 => names[0] < names[1],
        1 => names[0] > names[1],
        2 => extensions[0] < extensions[1],
        3 => extensions[0] > extensions[1],
        _ => false,
    }
}

pub fn fit_string(text: &mut Vec<u8>, width: i32, font: Option<Handle>, f: &mut impl BrowserForeign) -> Result<()> {
    let Some(font) = font else { return Ok(()) };
    if text.is_empty() {
        return Ok(());
    }
    let count = f.font_fit(font, text, width, 0);
    if count > text.len() {
        return Err(Unsupported::Precondition("font fit count within string"));
    }
    if count == 0 {
        text.clear();
        return Ok(());
    }
    if count == text.len() {
        return Ok(());
    }
    let marker = f.text_resource(0x5377_be31).unwrap_or_else(|| b"...".to_vec());
    let rect = if marker.is_empty() {
        [0; 4]
    } else {
        f.font_rect(font, &marker)
    };
    if rect[2].wrapping_sub(rect[0]) >= width {
        text.clear();
        return Ok(());
    }
    let count = f.font_fit(font, text, width.wrapping_add(rect[0]).wrapping_sub(rect[2]), 0);
    if count > text.len() {
        return Err(Unsupported::Precondition("font refit count within string"));
    }
    text.truncate(count);
    f.right_trim(text);
    text.extend_from_slice(&marker);
    Ok(())
}
