// SPDX-License-Identifier: GPL-3.0-or-later
//! The proved population and icon-loader portions of win-filebrowser §8.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Enumeration {
    Directories,
    Files,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Column {
    pub index: i32,
    pub width: i32,
    pub font: Option<Handle>,
}

/// Filesystem enumeration and individual grid operations. Column geometry is
/// an input because the browser's complete sizing algorithm remains FBQ5.
pub trait BrowserIo: BrowserForeign {
    fn enumerate(&mut self, path: &[u8], kind: Enumeration) -> Vec<Vec<u8>>;
    fn wildcard_match(&mut self, pattern: &[u8], name: &[u8], option: u32) -> bool;
    fn clear_grid(&mut self, grid: Handle);
    fn clear_columns(&mut self, grid: Handle);
    fn grid_set_text(&mut self, grid: Handle, column: i32, row: i32, text: &[u8]);
    fn grid_set_bitmap(&mut self, grid: Handle, column: i32, row: i32, bitmap: Handle);
    fn combo_select(&mut self, combo: Handle, index: u32);
    fn base_set_area(&mut self, window: Handle, area: Rect);
}

/// Buffer methods preserve the factory/fallback, borrowed resource and locking
/// boundaries. Procedural geometry itself is explicitly unproved (FBQ1).
pub trait IconIo: BrowserForeign {
    fn buffer_bounds(&mut self, buffer: Handle) -> Rect;
    fn factory_buffer(&mut self, component: u32, iid: u32) -> Option<Handle>;
    fn graphics_buffer(&mut self) -> Option<Handle>;
    fn init_buffer(&mut self, buffer: Handle, width: i32, height: i32, format: [u32; 2]) -> bool;
    fn icon_strip(&mut self, resource: u32) -> Option<Handle>;
    fn lock_buffer(&mut self, buffer: Handle, flags: u32) -> bool;
    fn copy_buffer(&mut self, destination: Handle, source: Handle, from: Rect, to: Rect);
    fn unlock_buffer(&mut self, buffer: Handle);
    fn convert_rgb(&mut self, window: Handle, rgb: [u8; 3]) -> u32;
    fn set_color_key(&mut self, buffer: Handle, color: u32);
    fn fill_buffer(&mut self, buffer: Handle, area: Rect, color: u32);
    fn line_buffer(&mut self, buffer: Handle, from: [i32; 2], to: [i32; 2], color: u32, option: u32);
}

impl FileBrowser {
    /// §8: repeated valid indices are deliberately not optimized away.
    pub fn select_filter(&mut self, index: u32, f: &mut impl BrowserIo) -> bool {
        if index as usize >= self.filter_labels.len() {
            return false;
        }
        self.filter_index = index;
        if let Some(combo) = self.children[7].or(self.filter_combo) {
            f.combo_select(combo, index);
        }
        self.mark_dirty(f);
        true
    }

    /// §8.1's specified enumeration/sort/cell installation. The caller supplies
    /// resolved column metrics, never a callback implementing browser logic.
    /// Full native sizing and pending-selection restoration remain FBQ5.
    pub fn populate_files(
        &mut self,
        columns: [Option<Column>; 3],
        icons_by_tag: &[(u32, Handle)],
        directory_mode: u32,
        f: &mut impl BrowserIo,
    ) -> Result<()> {
        let grid = self.children[5];
        if let Some(grid) = grid {
            f.clear_grid(grid);
            f.clear_columns(grid);
        }
        self.clear_file_entries();
        if self.flags & 1 != 0 && f.split_directory(&self.current, 2, 260).0 >= 2 {
            self.entries.push(Some(Entry {
                icon: None,
                tag: PARENT,
                id: 0,
                name: self.parent_label.clone(),
            }));
        }
        if self.flags & 4 != 0 {
            if let Some(button) = self.children[3] {
                let can_parent = f.split_directory(&self.current, 2, 260).0 >= 2;
                f.set_window_flag(button, 2, can_parent);
            }
            let mut directories = f.enumerate(&self.current, Enumeration::Directories);
            directories.sort();
            self.entries.extend(directories.into_iter().map(|name| {
                Some(Entry {
                    icon: None,
                    tag: DIRECTORY,
                    id: 0,
                    name,
                })
            }));
        }
        if self.flags & 8 != 0 {
            let patterns = if self.filter_labels.is_empty() {
                &[][..]
            } else {
                self.filter_patterns
                    .get(self.filter_index as usize)
                    .ok_or(Unsupported::Precondition("valid selected filter group"))?
                    .as_slice()
            };
            for name in f.enumerate(&self.current, Enumeration::Files) {
                if patterns.is_empty()
                    || patterns.iter().any(|p| {
                        let (mut pattern, mut candidate) = (p.clone(), name.clone());
                        let option = self.raw_bytes[&0x15c];
                        if option == 0 {
                            f.lowercase(&mut pattern);
                            f.lowercase(&mut candidate);
                        }
                        f.wildcard_match(&pattern, &candidate, u32::from(option))
                    })
                {
                    self.entries.push(Some(Entry {
                        icon: None,
                        tag: FILE,
                        id: 0,
                        name,
                    }));
                }
            }
        }
        // The callback is not a strict weak ordering on duplicate parents.
        // Insertion sorting calls the specified Boolean predicate directly.
        for i in 1..self.entries.len() {
            let mut j = i;
            while j > 0
                && precedes(
                    self.entries[j].as_ref().expect("new entry"),
                    self.entries[j - 1].as_ref().expect("new entry"),
                    self.sort_words[0],
                    self.sort_words[1],
                    directory_mode,
                    f,
                )
            {
                self.entries.swap(j, j - 1);
                j -= 1;
            }
        }
        if let Some(grid) = grid {
            for (row, entry) in self.entries.iter().enumerate() {
                let entry = entry.as_ref().expect("new entry");
                if let Some(column) = columns[0] {
                    if let Some((_, icon)) = icons_by_tag.iter().find(|(tag, _)| *tag == entry.tag) {
                        f.grid_set_bitmap(grid, column.index, row as i32, *icon);
                    }
                }
                for (field, column) in columns.iter().enumerate().skip(1) {
                    if let Some(column) = column {
                        let mut text = if field == 1 {
                            entry.name.clone()
                        } else {
                            f.path_extension(&entry.name)
                        };
                        if field == 2 {
                            if text.len() > 1 && text[0] == b'.' {
                                text.remove(0);
                            }
                            f.lowercase(&mut text);
                        }
                        if let Some(font) = column.font.or(self.common_font) {
                            fit_string(&mut text, column.width, Some(font), f)?;
                        }
                        f.grid_set_text(grid, column.index, row as i32, &text);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn set_area(&mut self, area: Rect, f: &mut impl BrowserIo) {
        self.mark_dirty(f);
        f.base_set_area(self.window(), area);
    }

    /// §8.3, FB-C25/C26. A failed lock is still a successful loader. The
    /// procedural glyph branch is the one genuine open contract here.
    pub fn load_default_icon(&mut self, index: u32, f: &mut impl IconIo) -> Result<bool> {
        let Some(buffer) = f
            .factory_buffer(0xc470_d325, 0x2073_2180)
            .or_else(|| f.graphics_buffer())
        else {
            return Ok(false);
        };
        if !f.init_buffer(buffer, 18, 17, [3, 16]) {
            f.release(buffer);
            return Ok(false);
        }
        if let Some(strip) = f.icon_strip(0xa896_3eea) {
            if f.lock_buffer(buffer, 0x8010) {
                let left = (index as i32).wrapping_mul(18);
                f.copy_buffer(buffer, strip, [left, 0, left.wrapping_add(18), 17], [0, 0, 18, 17]);
                f.unlock_buffer(buffer);
            }
            let key = f.convert_rgb(self.window(), [255, 0, 255]);
            f.set_color_key(buffer, key);
        } else if let Err(error) = self.draw_default_icon(index, Some(buffer), [0, 0, 18, 17], f) {
            f.release(buffer);
            return Err(error);
        }
        self.set_icon(index, Some(buffer), f);
        f.release(buffer);
        Ok(true)
    }

    /// FB-C20 is independent of the unsettled nonnull glyph recipe.
    pub fn create_default_icon(buffer: Option<Handle>) -> Result<bool> {
        if buffer.is_none() {
            Ok(true)
        } else {
            Err(Unsupported::Open("FBQ1"))
        }
    }

    /// FB-C21. This operation only promises allocation/initialization. A
    /// successful initialization cannot establish the unproved draw/result.
    pub fn create_default_button_buffer(output: &mut Option<Handle>, f: &mut impl IconIo) -> Result<()> {
        *output = f
            .factory_buffer(0xc470_d325, 0x2073_2180)
            .or_else(|| f.graphics_buffer());
        if let Some(buffer) = *output {
            if !f.init_buffer(buffer, 84, 22, [3, 16]) {
                f.release(buffer);
                *output = None;
                return Ok(());
            }
        }
        Err(Unsupported::Open("FBQ1"))
    }
}
