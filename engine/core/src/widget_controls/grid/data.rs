// SPDX-License-Identifier: GPL-3.0-or-later
//! Grid data-stage adapters (win-grid §§5.5–5.8,5.11,5.13). As with the
//! parent state helpers, the native recalculation hook is not executed here.
use super::*;
use std::collections::BTreeSet;

pub trait GridDataForeign: GridForeign {
    /// A successful query returns an acquired reference, released after use.
    fn object_query(&mut self, object: Handle, iid: u32, output: &mut Option<Handle>) -> bool;
    fn copy_text(&mut self, text_interface: Handle, output: &mut Vec<u8>) -> bool;
    fn window_id(&mut self, window: Handle) -> u32;
}

impl Grid {
    pub fn cell_extra(&self, cell: Coordinate) -> u32 {
        self.cells.get(&cell).map_or(0, |record| record.extra)
    }

    pub fn extra_object(&self, iid: u32, output: &mut Option<Handle>, f: &mut impl GridDataForeign) -> bool {
        self.extra != 0 && f.object_query(self.extra as Handle, iid, output)
    }

    pub fn cell_extra_object(
        &mut self,
        cell: Coordinate,
        iid: u32,
        output: &mut Option<Handle>,
        f: &mut impl GridDataForeign,
    ) -> bool {
        self.extra_mode = 1;
        let value = self.cell_extra(cell);
        self.extra_mode = 2; // Always, including absent cell or null data.
        value != 0 && f.object_query(value as Handle, iid, output)
    }

    pub fn set_owner_draw(&mut self, target: Option<Handle>, f: &mut impl Foreign) -> bool {
        replace_ref(&mut self.owner_draw, target, f);
        true
    }

    pub fn make_drop_down(&mut self) -> bool {
        self.flags |= 0x40000;
        true
    }

    /// Failure leaves caller output untouched. Window text is obtained only
    /// through its actual text interface, not a second private editor model.
    pub fn cell_text_value(&self, cell: Coordinate, output: &mut Vec<u8>, f: &mut impl GridDataForeign) -> bool {
        match self.cells.get(&cell).map(|record| &record.value) {
            Some(Value::Text(text)) => {
                *output = text.clone();
                true
            }
            Some(Value::Window { handle, .. }) => {
                let mut view = None;
                if !f.object_query(*handle, super::super::text_edit::IID, &mut view) {
                    return false;
                }
                let Some(view) = view else { return false };
                let result = f.copy_text(view, output);
                f.release(view);
                result
            }
            _ => false,
        }
    }

    #[allow(clippy::too_many_arguments)] // Native rectangular search arguments.
    pub fn find_text(
        &self,
        text: Option<&[u8]>,
        count: i32,
        first: Coordinate,
        extent: Coordinate,
        output: &mut Coordinate,
        exact_length: bool,
    ) -> Result<bool> {
        let Some(text) = text else { return Ok(false) };
        let length = if matches!(count, -1 | 0) {
            text.iter().position(|&b| b == 0).unwrap_or(text.len())
        } else {
            usize::try_from(count).map_err(|_| Unsupported::Precondition("Grid FindCellText count"))?
        };
        let text = text
            .get(..length)
            .ok_or(Unsupported::Precondition("Grid FindCellText readable source"))?;
        let inside = |value: i32, first: i32, count: i32| {
            value >= first && (count == i32::MAX || value < first.wrapping_add(count))
        };
        for (&cell, record) in &self.cells {
            if !inside(cell.0, first.0, extent.0) || !inside(cell.1, first.1, extent.1) {
                continue;
            }
            if let Value::Text(value) = &record.value {
                if (!exact_length || value.len() == length)
                    && (text.is_empty() || value.windows(length).any(|candidate| candidate == text))
                {
                    *output = cell;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn is_cell_window(
        &self,
        window: Option<Handle>,
        id: u32,
        output: &mut Coordinate,
        f: &mut impl GridDataForeign,
    ) -> bool {
        for (&handle, &(cell, _, _)) in &self.windows {
            if window.map_or_else(|| f.window_id(handle) == id, |window| window == handle) {
                *output = cell;
                return true;
            }
        }
        false
    }

    pub fn make_room_for_column(&mut self, column: i32, count: i32, mode: u32, f: &mut impl GridForeign) -> bool {
        self.make_room_for_axis(0, column, count, if mode == 0 { 2 } else { mode }, f)
    }

    pub fn make_room_for_row(&mut self, row: i32, count: i32, mode: u32, f: &mut impl GridForeign) -> bool {
        self.make_room_for_axis(1, row, count, if mode == 0 { 3 } else { mode }, f)
    }

    fn make_room_for_axis(&mut self, axis: usize, first: i32, count: i32, mode: u32, f: &mut impl GridForeign) -> bool {
        if !(if axis == 0 { [1, 2, 4] } else { [1, 3, 5] }).contains(&mode) {
            return false;
        }
        let cross: BTreeSet<_> = self.cells.keys().map(|&cell| component(cell, 1 - axis)).collect();
        for other in cross {
            self.make_room(coordinate(axis, first, other), count, mode, f);
        }
        true
    }

    pub fn delete_columns(&mut self, first: i32, count: i32, mode: u32, f: &mut impl GridForeign) -> bool {
        self.delete_axis(0, first, count, mode, f)
    }

    pub fn delete_rows(&mut self, first: i32, count: i32, mode: u32, f: &mut impl GridForeign) -> bool {
        self.delete_axis(1, first, count, mode, f)
    }

    fn delete_axis(&mut self, axis: usize, first: i32, count: i32, mut mode: u32, f: &mut impl GridForeign) -> bool {
        if count < 0 {
            return false;
        }
        if count == 0 {
            return true;
        }
        let forward = if axis == 0 { 2 } else { 3 };
        let backward = if axis == 0 { 4 } else { 5 };
        if mode == 0 {
            mode = forward;
        } else if ![1, forward, backward].contains(&mode) {
            mode = 1;
        }
        let end = first.wrapping_add(count);
        let remove: Vec<_> = self
            .cells
            .keys()
            .copied()
            .filter(|&cell| (first..end).contains(&component(cell, axis)))
            .collect();
        for cell in remove {
            if let Some(record) = self.cells.remove(&cell) {
                self.dispose(cell, record, f);
            }
        }
        if mode == 1 {
            self.reset_cached(axis, first, 1, -1);
        } else if mode == forward {
            self.reset_cached(axis, first, i32::MAX, -1);
            self.make_room_for_axis(axis, first, -count, mode, f);
        } else {
            self.reset_cached(axis, 0, first.wrapping_add(1), -1);
            self.make_room_for_axis(axis, first, -count, mode, f);
        }
        self.dirty = true;
        true
    }

    pub fn copy_columns(
        &mut self,
        from: i32,
        to: i32,
        count: u32,
        move_cells: bool,
        f: &mut impl GridForeign,
    ) -> Result<bool> {
        self.copy_axis(0, from, to, count, move_cells, f)
    }

    pub fn copy_rows(
        &mut self,
        from: i32,
        to: i32,
        count: u32,
        move_cells: bool,
        f: &mut impl GridForeign,
    ) -> Result<bool> {
        self.copy_axis(1, from, to, count, move_cells, f)
    }

    fn copy_axis(
        &mut self,
        axis: usize,
        from: i32,
        to: i32,
        count: u32,
        move_cells: bool,
        f: &mut impl GridForeign,
    ) -> Result<bool> {
        if from == to || count == 0 {
            return Ok(true);
        }
        let count = i32::try_from(count).map_err(|_| Unsupported::Precondition("Grid representable copy count"))?;
        let end = from
            .checked_add(count)
            .ok_or(Unsupported::Precondition("Grid representable copy source"))?;
        let to_end = to
            .checked_add(count)
            .ok_or(Unsupported::Precondition("Grid representable copy destination"))?;
        let forward = to >= end || from >= to_end || to_end < end;
        for offset in 0..count {
            let offset = if forward { offset } else { count - offset - 1 };
            let source = from + offset;
            let target = to + offset;
            let mut range = self
                .cells
                .keys()
                .filter(|&&cell| component(cell, axis) == source)
                .map(|&cell| component(cell, 1 - axis));
            if let Some(first) = range.next() {
                let (low, high) = range.fold((first, first), |(low, high), n| (low.min(n), high.max(n)));
                for other in low..=high {
                    self.reset_cached(1 - axis, other, 1, -1);
                    self.copy_cell(
                        coordinate(axis, source, other),
                        coordinate(axis, target, other),
                        !move_cells,
                        f,
                    );
                }
            }
            let extra = self.get_axis_extra(axis, source);
            self.extra_maps[axis].insert(target, extra);
            let heading = self.heading(axis, source).to_vec();
            self.headings[axis].insert(target, heading);
            if move_cells {
                self.extra_maps[axis].remove(&source);
                self.headings[axis].remove(&source);
            }
            self.dirty = true;
        }
        Ok(true)
    }

    /// Default/per-cell format replacement only. Axis-format size words remain
    /// explicitly open WG6; no invented struct-copy semantics.
    pub fn set_cell_format_state(&mut self, cell: Coordinate, format: &Format, f: &mut impl Foreign) -> Result<bool> {
        if cell == (-1, -1) {
            if let Some(font) = format.font {
                f.add_ref(font);
            }
            if let Some(font) = self.default_format.font {
                f.release(font);
            }
            self.default_format = format.clone();
            self.default_colors.copy_from_slice(&format.words[..6]);
            return Ok(true);
        }
        if cell.0 == -1 || cell.1 == -1 {
            return Err(Unsupported::Open("WG6"));
        }
        if !self.within(cell) {
            return Ok(false);
        }
        if let Some(font) = format.font {
            f.add_ref(font);
        }
        let exists = self.cells.contains_key(&cell);
        let record = self.cells.entry(cell).or_default();
        let old = record.format.replace(format.clone());
        if !exists || old.is_none() || old.as_ref().and_then(|format| format.font).is_some() || format.font.is_some() {
            self.dirty = true;
        }
        if let Some(font) = old.and_then(|format| format.font) {
            f.release(font);
        }
        Ok(true)
    }
}

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
