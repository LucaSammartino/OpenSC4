// SPDX-License-Identifier: GPL-3.0-or-later
//! Complete owner sizing, restoration and directory/popup ordering (§8.1.3–6).
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridOp {
    Column(u32, u32),
    Style(u32, i32, i32),
    Text(i32, i32, Vec<u8>, u32, bool, bool),
    RowHeights(u32, u32),
    FirstVisible(u32),
    Select(i32, i32, bool),
    Update,
    BringIntoView(i32, i32),
    Area(Rect),
    Activate,
}

pub trait PopulationIo: BrowserIo + ControlIo + PropertyIo {
    fn grid(&mut self, grid: Handle, operation: GridOp);
    fn window_rect(&mut self, control: Handle) -> Rect;
    fn cell_visible(&mut self, grid: Handle, column: i32, row: i32) -> bool;
    fn split_components(&mut self, path: &[u8], capacity: usize, component_bytes: usize) -> Vec<Vec<u8>>;
    fn focus_grid(&mut self, manager: Handle, grid: Handle);
    fn background_rect(&mut self, buffer: Handle, area: Rect, options: [u32; 2]) -> Rect;
}

impl FileBrowser {
    pub fn populate_file_list(&mut self, f: &mut impl PopulationIo) -> Result<bool> {
        self.populate_files([None; 3], &[], self.directory_order, f)?;
        let Some(grid) = self.children[5] else {
            return Ok(true);
        };
        let area = f.window_rect(grid);
        let row_height = self.word(0x224);
        if row_height == 0 {
            return Err(Unsupported::Precondition("unsigned file-row divide by zero"));
        }
        let capacity = (area[3].wrapping_sub(area[1]) as u32) / row_height;
        let mut width = (area[2].wrapping_sub(area[0]) as u32).wrapping_sub(2);
        if self.entries.len() as u32 > capacity {
            width = width.wrapping_sub(self.word(0x314));
        }
        for id in [0, 5, 2] {
            if self.column_enabled(id) {
                width = width.wrapping_sub(self.word(0x1e4 + 12 * id));
            }
        }
        self.store_word(0x1f0, width);
        for id in 0..6 {
            if self.column_enabled(id) {
                f.grid(
                    grid,
                    GridOp::Column(self.word(0x1e0 + 12 * id), self.word(0x1e4 + 12 * id)),
                );
            }
        }
        for (row, entry) in self.entries.iter().enumerate() {
            let entry = entry.as_ref().expect("populated record");
            let row = row as i32;
            f.grid(grid, GridOp::Style(0x280, self.word(0x21c) as i32, row));
            if self.column_enabled(0) {
                let column = self.word(0x1e0) as i32;
                let icon = match entry.tag {
                    PARENT => self.icons[0],
                    DIRECTORY => self.icons[1],
                    _ => entry.icon.or(self.icons[2]),
                };
                if let Some(icon) = icon {
                    f.grid_set_bitmap(grid, column, row, icon);
                }
                f.grid(grid, GridOp::Style(0x280, column, row));
            }
            if self.column_enabled(2) {
                let column = self.word(0x1f8) as i32;
                if !matches!(entry.tag, PARENT | DIRECTORY) {
                    let mut extension = f.path_extension(&entry.name);
                    f.lowercase(&mut extension);
                    if extension.len() > 1 && extension[0] == b'.' {
                        extension.remove(0);
                    }
                    fit_string(
                        &mut extension,
                        self.word(0x1fc).wrapping_sub(2) as i32,
                        self.style_fonts[1].or(self.common_font),
                        f,
                    )?;
                    f.grid(grid, GridOp::Text(column, row, extension, u32::MAX, true, false));
                }
                f.grid(grid, GridOp::Style(0x25c, column, row));
            }
            let mut name = entry.name.clone();
            fit_string(
                &mut name,
                width.wrapping_sub(2) as i32,
                self.style_fonts[0].or(self.common_font),
                f,
            )?;
            f.grid(
                grid,
                GridOp::Text(self.word(0x1ec) as i32, row, name, u32::MAX, true, false),
            );
            f.grid(grid, GridOp::Style(0x238, self.word(0x1ec) as i32, row));
        }
        let mut candidate = 0;
        for (row, entry) in self.entries.iter().enumerate() {
            let entry = entry.as_ref().expect("populated record");
            let eligible = match self.pending_word {
                1 => entry.tag == DIRECTORY,
                2 => !matches!(entry.tag, PARENT | DIRECTORY),
                _ => false,
            };
            if eligible {
                let (mut name, mut pending) = (entry.name.clone(), self.pending_name.clone());
                f.lowercase(&mut name);
                f.lowercase(&mut pending);
                if pending.is_empty() || name == pending {
                    candidate = row as i32;
                    break;
                }
            }
        }
        f.grid(grid, GridOp::RowHeights(self.entries.len() as u32, row_height));
        f.grid(grid, GridOp::FirstVisible(0));
        f.grid(grid, GridOp::Select(0, candidate, true));
        f.grid(grid, GridOp::Update);
        if !f.cell_visible(grid, 0, candidate) {
            f.grid(grid, GridOp::BringIntoView(0, candidate));
        }
        if !self.entries.is_empty() {
            self.selection_change(candidate as usize, f)?;
        }
        self.name_column = Some(self.word(0x1ec) as i32);
        Ok(true)
    }

    fn column_enabled(&self, id: u32) -> bool {
        self.raw_bytes.get(&(0x1dc + 12 * id)).copied().unwrap_or(0) != 0
    }

    pub fn populate_directory_list(&mut self, f: &mut impl PopulationIo) -> Result<bool> {
        self.drive_records.retain(|r| r.depth == 0);
        self.store_word(0x19c, 0);
        self.store_word(0x234, 2);
        let mut current = vec![self.current.first().copied().unwrap_or(0)];
        f.lowercase(&mut current);
        let mut matched = None;
        for (row, record) in self.drive_records.iter().enumerate() {
            let mut initial = vec![record.name.first().copied().unwrap_or(0)];
            f.lowercase(&mut initial);
            if initial == current {
                matched = Some(row);
                break;
            }
        }
        self.store_word(0x19c, matched.unwrap_or(self.drive_records.len()) as u32);
        let Some(drive) = matched else {
            return Ok(true);
        };
        self.store_word(0x19c, drive as u32);
        let parts = f.split_components(&self.current, 48, 260);
        if parts.len() > 48 {
            return Err(Unsupported::Precondition("SplitDirectory output within capacity"));
        }
        let mut last = drive;
        for (depth, mut name) in parts.into_iter().rev().skip(1).enumerate() {
            if name.last().is_some_and(|&b| f.path_separator(b)) {
                name.pop();
            }
            last += 1;
            self.drive_records.insert(
                last,
                DriveRecord {
                    name,
                    kind: 5,
                    icon: self.icons[1],
                    depth: depth as u32 + 1,
                },
            );
            self.store_word(0x19c, last as u32);
            self.store_word(0x234, self.word(0x234).wrapping_add(1));
        }
        if let Some(grid) = self.children[0] {
            let record = &self.drive_records[last];
            f.clear_grid(grid);
            if let Some(icon) = record.icon {
                f.grid_set_bitmap(grid, 0, 0, icon);
                f.grid(grid, GridOp::Style(0x280, 0, 0));
                let width = f.buffer_width(icon).wrapping_add(2) as u32;
                f.grid(grid, GridOp::Column(0, width));
            }
            let area = f.window_rect(grid);
            let width = area[2]
                .wrapping_sub(area[0])
                .wrapping_sub(3)
                .wrapping_sub(self.word(0x1e4) as i32);
            let mut name = record.name.clone();
            fit_string(&mut name, width, self.style_fonts[3].or(self.common_font), f)?;
            let count = name.len() as u32;
            f.grid(grid, GridOp::Text(1, 0, name, count, true, false));
            f.grid(grid, GridOp::Style(0x2a4, 1, 0));
        }
        Ok(true)
    }

    pub fn open_popup(&mut self, f: &mut impl PopulationIo) -> Result<bool> {
        let grid = obtain_control(1, f)?.ok_or(Unsupported::Precondition("required popup grid"))?;
        self.children[1] = Some(grid);
        let count = self.drive_records.len() as u32;
        let maximum = self.word(0x230);
        let capacity = count.min(maximum);
        f.control(grid, ControlOp::Id(FIRST_CHILD_ID + 1));
        f.control(grid, ControlOp::Dimensions(self.word(0x234).wrapping_add(8), 1, false));
        if self.common_font.is_some() {
            f.control(grid, ControlOp::Font(self.common_font));
        }
        f.control(grid, ControlOp::SelectionWords(0x4000_0000, 0x4000_0000));
        let white = f.convert_rgb(self.window(), [255, 255, 255]);
        f.control(grid, ControlOp::Fill(white));
        f.control(grid, ControlOp::Borders([1; 4]));
        f.control(grid, ControlOp::Outline(self.raw_bytes[&0x308], 0));
        for flag in [0x800, 2] {
            f.control(grid, ControlOp::WindowFlag(flag, false));
        }
        for flag in [0x20000, 0x80000] {
            f.control(grid, ControlOp::WindowFlag(flag, true));
        }
        let collapsed = self.children[0].ok_or(Unsupported::Precondition("popup collapsed grid"))?;
        let area = f.window_rect(collapsed);
        let top = area[3].wrapping_sub(1);
        let height = self.word(0x224).wrapping_mul(capacity).wrapping_add(2);
        f.grid(
            grid,
            GridOp::Area([area[0], top, area[2], top.wrapping_add(height as i32)]),
        );
        let span = self.word(0x234);
        let icon_width = self.word(0x1e4);
        for column in 0..span {
            f.grid(grid, GridOp::Column(column, icon_width.wrapping_mul(9) >> 4));
        }
        f.grid(grid, GridOp::Column(span, 300));
        for (row, record) in self.drive_records.iter().enumerate() {
            let row = row as i32;
            f.grid(grid, GridOp::Style(0x2a4, -1, row));
            if let Some(icon) = record.icon {
                f.grid_set_bitmap(grid, record.depth as i32, row, icon);
                f.grid(grid, GridOp::Style(0x280, record.depth as i32, row));
            }
            let mut width = (area[2].wrapping_sub(area[0]) as u32)
                .wrapping_sub(2)
                .wrapping_sub(record.depth.wrapping_add(1).wrapping_mul(icon_width));
            if count > maximum {
                width = width.wrapping_sub(self.word(0x314));
            }
            let mut name = record.name.clone();
            fit_string(&mut name, width as i32, self.style_fonts[3].or(self.common_font), f)?;
            let length = name.len() as u32;
            f.grid(
                grid,
                GridOp::Text(record.depth.wrapping_add(2) as i32, row, name, length, true, false),
            );
        }
        f.grid(grid, GridOp::RowHeights(count, self.word(0x224)));
        f.control(grid, ControlOp::AddChild(self.window()));
        f.grid(grid, GridOp::FirstVisible(0));
        let current = self.word(0x19c);
        f.grid(grid, GridOp::Select(0, current as i32, true));
        f.grid(grid, GridOp::Update);
        if !f.cell_visible(grid, 0, current as i32) {
            f.grid(
                grid,
                GridOp::FirstVisible(current.wrapping_sub(capacity).wrapping_add(1)),
            );
        }
        let manager = f.manager(self.window());
        f.focus_grid(manager, grid);
        f.grid(grid, GridOp::Activate);
        Ok(true)
    }

    pub fn close_popup(&mut self, f: &mut impl BrowserForeign) -> bool {
        if let Some(popup) = self.children[1] {
            let window = f.child_window(popup, 0x14);
            f.remove_child(self.window(), window);
            f.release(popup);
            self.children[1] = None;
        }
        true
    }

    pub fn select_directory<H: BrowserHost>(
        &mut self,
        index: i32,
        f: &mut H,
        dispatch: &mut impl BrowserOverrides<H>,
    ) -> Result<bool> {
        if index == self.word(0x19c) as i32 {
            return Ok(true);
        }
        if index >= 0 {
            for record in self.drive_records.iter().take(index as usize + 1) {
                let path = [&record.name[..], b"\\"].concat();
                if record.depth == 0 {
                    self.current = path;
                } else {
                    self.current.extend_from_slice(&path);
                }
            }
        }
        self.update_path_live(f, dispatch)
    }

    pub fn remove_subdirectory<H: BrowserHost>(
        &mut self,
        path: &[u8],
        f: &mut H,
        dispatch: &mut impl BrowserOverrides<H>,
    ) -> Result<bool> {
        if path.len() == self.current.len() || !path.starts_with(&self.current) || !f.remove_directory(path, 1) {
            return Ok(false);
        }
        self.update_path_live(f, dispatch)
    }

    pub fn paint_live<H: BrowserHost>(&mut self, f: &mut H, dispatch: &mut impl BrowserOverrides<H>) -> Result<bool> {
        if self.raw_bytes[&0x309] != 0 {
            let area = f.window_rect(self.window());
            if let Some(buffer) = self.extra_reference {
                let rect = f.background_rect(buffer, area, [0, 0]);
                f.draw(Draw::Blit { source: buffer, rect });
            } else {
                f.draw(Draw::Fill {
                    rect: area,
                    color: self.word(0x30c),
                });
            }
        }
        self.paint(None, f, dispatch)
    }

    pub fn create_directory_live<H: BrowserHost>(
        &mut self,
        f: &mut H,
        dispatch: &mut impl BrowserOverrides<H>,
    ) -> Result<bool> {
        let Some(leaf) = self.create_directory_candidate(f) else {
            return Ok(false);
        };
        for refresh in [Refresh::DirectoryList, Refresh::FileList] {
            dispatch.refresh(self, refresh, f)?;
        }
        if let Some(grid) = self.children[5] {
            let row = self.get_row_of_entry_by_name(&leaf, f);
            f.grid_scroll_to(grid, row);
            if !f.grid_selected(grid, row) {
                f.grid_select(grid, row);
            }
        }
        Ok(true)
    }
}
