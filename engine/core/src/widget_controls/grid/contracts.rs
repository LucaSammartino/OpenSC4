// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled WG5–WG9 dependency and owner contracts, win-grid §§5.15–5.16.
use super::*;
use std::cmp::Ordering;

impl Format {
    fn merge(&mut self, source: &Self) {
        if source.font.is_some() {
            self.font = source.font;
        }
        for i in [0, 1, 2, 3, 4, 5, 7] {
            if source.words[i] != 0x4000_0000 {
                self.words[i] = source.words[i];
            }
        }
        let mut bytes = self.words[6].to_le_bytes();
        let other = source.words[6].to_le_bytes();
        for i in 0..4 {
            if other[i] != if i == 0 { 0xff } else { 0 } {
                bytes[i] = other[i];
            }
        }
        self.words[6] = u32::from_le_bytes(bytes);
    }
}

impl Grid {
    /// Borrowed-font value copy; no bounds check or ownership changes.
    pub fn cell_format(&self, cell: Coordinate) -> Format {
        let mut format = self.default_format.clone();
        for (axis, index) in [(1, cell.1), (0, cell.0)] {
            if let Some(entry) = self.sizes[axis].get(&index) {
                format.merge(&entry.format);
            }
        }
        if let Some(source) = self.cells.get(&cell).and_then(|record| record.format.as_ref()) {
            format.merge(source);
        }
        format
    }

    pub fn default_formatting(&mut self, f: &mut impl GridHost) {
        let window = self.window();
        for header in [false, true] {
            let format = if header {
                &mut self.header_format
            } else {
                &mut self.default_format
            };
            let font = &mut format.font;
            if font.is_none() {
                *font = f.default_interface(0x6896_3c4c);
                if let Some(font) = *font {
                    f.add_ref(font);
                }
            }
            let colors = if header {
                [
                    [0, 0, 0],
                    [255, 255, 255],
                    [128, 128, 128],
                    [220, 220, 220],
                    [0, 0, 0],
                    [220, 220, 255],
                    [0, 0, 0],
                ]
            } else {
                [
                    [0, 0, 0],
                    [255, 255, 255],
                    [192, 192, 192],
                    [255, 255, 255],
                    [255, 255, 255],
                    [0, 0, 0],
                    [0, 0, 0],
                ]
            };
            for (index, rgb) in [0, 1, 2, 3, 4, 5, 7].into_iter().zip(colors) {
                if format.words[index] == 0x4000_0000 {
                    format.words[index] = f.grid_rgb(window, rgb);
                }
            }
            let mut bytes = format.words[6].to_le_bytes();
            if bytes[0] == 0xff {
                bytes[0] = 1;
            }
            if bytes[1] == 0 {
                bytes[1] = 2;
            }
            if bytes[2] == 0 {
                bytes[2] = if header { 6 } else { 5 };
            }
            format.words[6] = u32::from_le_bytes(bytes);
        }
        self.default_colors.copy_from_slice(&self.default_format.words[..6]);
        self.outline = f.grid_rgb(window, [0, 0, 0]);
        self.line_colors = [0xff00_0000; 2];
    }

    /// Header slots 72/73. None denotes the native false result.
    pub fn header_rect(&mut self, axis: usize, index: i32, f: &mut impl GridForeign) -> Option<Rect> {
        if self.flags & (8 << axis) == 0 {
            return None;
        }
        let size = f.window_size(self.window());
        if index == -1 {
            return Some(if axis == 0 {
                [
                    self.gutters[0],
                    self.gutters[1],
                    size[0].wrapping_sub(self.gutters[2]),
                    self.gutters[1].wrapping_add(self.header_size[0]),
                ]
            } else {
                // Whole row band deliberately uses header height, not width.
                [
                    self.gutters[0],
                    self.gutters[1],
                    self.gutters[0].wrapping_add(self.header_size[0]),
                    size[1].wrapping_sub(self.gutters[3]),
                ]
            });
        }
        let cell = if axis == 0 { (index, 0) } else { (0, index) };
        if !self.within(cell) {
            return None;
        }
        let mut rect = self.cell_rect(cell, f);
        if axis == 0 {
            rect[1] = self.gutters[1];
            rect[3] = rect[1].wrapping_add(self.header_size[0]);
        } else {
            rect[0] = self.gutters[0];
            rect[2] = rect[0].wrapping_add(self.header_size[1]);
        }
        Some(rect)
    }

    fn partial_format(&mut self, cell: Coordinate) -> Option<&mut Format> {
        if !self.within(cell) {
            return None;
        }
        Some(
            self.cells
                .entry(cell)
                .or_default()
                .format
                .get_or_insert_with(Format::default),
        )
    }

    pub fn set_cell_colors(
        &mut self,
        cell: Coordinate,
        index: u32,
        color: u32,
        context: &mut LayoutContext,
        f: &mut impl GridHost,
    ) -> Result<bool> {
        if !self.within(cell) {
            return Ok(false);
        }
        let format = self.partial_format(cell).expect("checked bounds");
        // Native unchecked indexing can overwrite packed bytes/border colour.
        // Beyond the 36-byte format is a memory precondition, never false.
        let slot = format
            .words
            .get_mut(index as usize)
            .ok_or(Unsupported::Precondition("Grid color write outside format storage"))?;
        *slot = color;
        self.recalculate(context, f);
        Ok(true)
    }

    pub fn set_cell_wrapping(
        &mut self,
        cell: Coordinate,
        value: u32,
        context: &mut LayoutContext,
        f: &mut impl GridHost,
    ) -> bool {
        let Some(format) = self.partial_format(cell) else {
            return false;
        };
        format.words[6] = (format.words[6] & !0xff) | (value & 0xff);
        self.recalculate(context, f);
        true
    }

    pub fn set_cell_alignment(
        &mut self,
        cell: Coordinate,
        horizontal: u32,
        vertical: u32,
        context: &mut LayoutContext,
        f: &mut impl GridHost,
    ) -> bool {
        let Some(format) = self.partial_format(cell) else {
            return false;
        };
        format.words[6] = (format.words[6] & 0xff00_00ff) | ((horizontal & 0xff) << 8) | ((vertical & 0xff) << 16);
        self.recalculate(context, f);
        true
    }

    pub fn set_cell_border(
        &mut self,
        cell: Coordinate,
        style: u32,
        color: u32,
        context: &mut LayoutContext,
        f: &mut impl GridHost,
    ) -> bool {
        let Some(format) = self.partial_format(cell) else {
            return false;
        };
        format.words[6] = (format.words[6] & 0x00ff_ffff) | ((style & 0xff) << 24);
        format.words[7] = color;
        self.recalculate(context, f);
        true
    }

    pub fn set_cell_extra(&mut self, cell: Coordinate, value: u32) -> bool {
        self.extra_mode = 1;
        if !self.within(cell) {
            return false;
        }
        if value != 0 || self.cells.contains_key(&cell) {
            self.cells.entry(cell).or_default().extra = value;
        }
        true
    }

    pub fn resize_columns_to_fit_windows(
        &mut self,
        start: i32,
        count: i32,
        context: &mut LayoutContext,
        f: &mut impl GridHost,
    ) {
        let mut changed = false;
        // Native decrement-to-zero semantics also apply to negative counts.
        let mut remaining = count;
        let mut column = start;
        while remaining != 0 {
            let maximum = self
                .cells
                .iter()
                .filter_map(|(&(c, _), record)| match &record.value {
                    Value::Window { handle, .. } if c == column && *handle != 0 => Some(f.window_size(*handle)[0]),
                    _ => None,
                })
                .max();
            if let Some(width) = maximum {
                let entry = self.sizes[0].entry(column).or_default();
                if entry.cached == -1 || i32::from(entry.cached) < width {
                    entry.cached = width as i16;
                    changed = true;
                }
            }
            column = column.wrapping_add(1);
            remaining = remaining.wrapping_sub(1);
        }
        if changed {
            self.reset_cached(1, 0, i32::MAX, -1);
            self.dirty = true;
            self.recalculate(context, f);
        }
    }

    /// axis 0 = SortColumn, axis 1 = SortRow. Comparator must match host CRT
    /// CTYPE locale; compare_c_locale supplies the default/C locale behavior.
    #[allow(clippy::too_many_arguments)]
    pub fn sort_axis(
        &mut self,
        axis: usize,
        key: i32,
        direction: u32,
        whole_grid: bool,
        mut compare: Option<&mut CellComparator<'_>>,
        context: &mut LayoutContext,
        f: &mut impl GridHost,
    ) {
        let old_range = self.used_range();
        let mut sequence: Vec<_> = self
            .cells
            .iter()
            .filter(|(cell, _)| component(**cell, axis) == key)
            .map(|(&cell, _)| component(cell, 1 - axis))
            .collect();
        sequence.sort_unstable();
        sequence.sort_by(|&a, &b| {
            let a = &self.cells[&coordinate(axis, key, a)];
            let b = &self.cells[&coordinate(axis, key, b)];
            let (a, b) = if direction == 0 { (a, b) } else { (b, a) };
            if let Some(compare) = &mut compare {
                compare(a, b)
            } else {
                compare_cell_values(Some(a), Some(b), |a, b| f.compare_grid_text(a, b))
            }
        });
        let ranks: BTreeMap<_, _> = sequence
            .iter()
            .enumerate()
            .map(|(rank, &index)| (index, rank as i32))
            .collect();
        let mut affected: BTreeMap<i32, Vec<(i32, Cell)>> = BTreeMap::new();
        let cells = std::mem::take(&mut self.cells);
        for (cell, record) in cells {
            let index = component(cell, axis);
            if whole_grid || index == key {
                affected
                    .entry(index)
                    .or_default()
                    .push((component(cell, 1 - axis), record));
            } else {
                self.cells.insert(cell, record);
            }
        }
        for (index, mut records) in affected {
            records.sort_by_key(|&(other, _)| other);
            let mut trailing = sequence.len() as i32;
            for (old, record) in records {
                let new = ranks.get(&old).copied().unwrap_or_else(|| {
                    let result = trailing;
                    trailing = trailing.wrapping_add(1);
                    result
                });
                self.cells.insert(coordinate(axis, index, new), record);
            }
        }
        let mut trailing = sequence.len() as i32;
        for cell in &mut self.selection {
            if whole_grid || component(*cell, axis) == key {
                // SortRow intentionally looks up old row, not old column.
                let new = ranks.get(&cell.1).copied().unwrap_or_else(|| {
                    let result = trailing;
                    trailing = trailing.wrapping_add(1);
                    result
                });
                if axis == 0 {
                    cell.1 = new;
                } else {
                    cell.0 = new;
                }
            }
        }
        let new_range = self.used_range();
        for (&row, entry) in &mut self.sizes[1] {
            if row >= old_range[1].min(new_range[1]) && row <= old_range[3].max(new_range[3]) {
                entry.cached = -1;
            }
        }
        self.dirty = true;
        self.recalculate(context, f);
    }

    pub fn sort_column(
        &mut self,
        column: i32,
        direction: u32,
        whole_grid: bool,
        compare: Option<&mut CellComparator<'_>>,
        context: &mut LayoutContext,
        f: &mut impl GridHost,
    ) {
        self.sort_axis(0, column, direction, whole_grid, compare, context, f);
    }

    pub fn sort_row(
        &mut self,
        row: i32,
        direction: u32,
        whole_grid: bool,
        compare: Option<&mut CellComparator<'_>>,
        context: &mut LayoutContext,
        f: &mut impl GridHost,
    ) {
        self.sort_axis(1, row, direction, whole_grid, compare, context, f);
    }
}

pub type CellComparator<'a> = dyn FnMut(&Cell, &Cell) -> Ordering + 'a;

fn component(cell: Coordinate, axis: usize) -> i32 {
    if axis == 0 {
        cell.0
    } else {
        cell.1
    }
}
fn coordinate(axis: usize, value: i32, other: i32) -> Coordinate {
    if axis == 0 {
        (value, other)
    } else {
        (other, value)
    }
}

/// CRT _stricmp with C/default CTYPE: unsigned narrow bytes, stop at NUL.
pub fn compare_c_locale(a: &Cell, b: &Cell) -> Ordering {
    compare_cell_values(Some(a), Some(b), |a, b| {
        a.iter()
            .copied()
            .take_while(|&c| c != 0)
            .map(|c| c.to_ascii_lowercase())
            .cmp(
                b.iter()
                    .copied()
                    .take_while(|&c| c != 0)
                    .map(|c| c.to_ascii_lowercase()),
            )
    })
}
