// SPDX-License-Identifier: GPL-3.0-or-later
//! cGZWinGrid state, selection and sparse size/format tables (win-grid §§1–5).
mod data;
mod paint;
mod runtime;
use super::*;
pub use data::GridDataForeign;
pub use paint::{PaintHost, PaintOp, PaintRecord};
pub use runtime::{compare_cell_values, equal_record, signed_gcd, stable_sort_values};
pub use runtime::{GridHost, Input, Mutation, PendingOwner};
use std::collections::BTreeMap;

pub const IID: u32 = 0xdaa6_b9be;
pub const PRIMARY_SLOTS: usize = 142;
pub const WINDOW_SLOTS: usize = 151;
pub type Coordinate = (i32, i32);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Format {
    pub font: Option<Handle>,
    /// The other eight words are preserved even when their public meanings
    /// remain unproved. They are not a Rust/native memory-layout promise.
    // WG6: zero initialization is provisional until SetDefaultFormatting is
    // specified; compatible drawing requires a resolved supplied format.
    pub words: [u32; 8],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SizeEntry {
    pub format: Format,
    pub explicit: i16,
    pub cached: i16,
}
impl Default for SizeEntry {
    fn default() -> Self {
        Self {
            format: Format::default(),
            explicit: -1,
            cached: -1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Empty,
    Window {
        handle: Handle,
        keep_width: bool,
        keep_height: bool,
    },
    Text(Vec<u8>),
    Bitmap(Handle),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub value: Value,
    pub format: Option<Format>,
    pub extra: u32,
}
impl Default for Cell {
    fn default() -> Self {
        Self {
            value: Value::Empty,
            format: None,
            extra: 0,
        }
    }
}

/// Context-wide recursion state, explicitly shared by all grids in a toolkit.
/// It is not per-window: nested foreign callbacks must reuse the same context.
#[derive(Default, Debug)]
pub struct LayoutContext {
    pub depth: u32,
}

pub trait GridForeign: Foreign {
    fn language_value(&mut self) -> Option<u32>;
    fn window_size(&mut self, window: Handle) -> [i32; 2];
    fn set_area(&mut self, window: Handle, rect: Rect);
    fn child_add(&mut self, parent: Handle, child: Handle);
    fn child_remove(&mut self, parent: Handle, child: Handle);
    fn child_delete(&mut self, parent: Handle, child: Handle);
    fn window_flag(&mut self, window: Handle, flag: u32, set: bool);
    fn send_to_back(&mut self, window: Handle);
    fn move_window(&mut self, window: Handle, left: i32, top: i32);
    fn set_width(&mut self, window: Handle, width: i32);
    fn set_height(&mut self, window: Handle, height: i32);
    fn line_count(&mut self, font: Handle, text: &[u8], width: i32) -> i32;
    fn grid_notification(&mut self, window: Handle, id: u32, data: &[i32]);
    fn sound_instance(&mut self, class: u32, args: [u32; 5]) -> Option<Handle>;
    fn play_instance(&mut self, instance: Handle) -> bool;
}

#[derive(Debug)]
pub struct Grid {
    pub primary: Handle,
    pub initialized: bool,
    pub flags: u32,
    pub language: u32,
    pub dirty: bool,
    pub pause: i32,
    pub gutters: [i32; 4],
    pub cell_gutters: [i32; 4],
    pub header_size: [i32; 2],
    pub grid_size: [i32; 2],
    pub cell_size: [i32; 2],
    pub position: [i32; 2],
    pub visible: [f32; 2],
    pub limits: [i32; 2],
    pub default_size: [i32; 2],
    pub outline: u32,
    pub last_set: Coordinate,
    pub background: Option<Handle>,
    pub owner_draw: Option<Handle>,
    pub header_font: Option<Handle>,
    pub default_format: Format,
    pub default_colors: [u32; 6],
    pub selection_mode: u32,
    pub selection_max: i32,
    pub selection: Vec<Coordinate>,
    pub cells: BTreeMap<Coordinate, Cell>,
    /// Independent set keyed by the child identity (§5.7). Moving cell data
    /// does not update these coordinates: DoCellWindowMove is a no-op (§5.11).
    pub windows: BTreeMap<Handle, (Coordinate, bool, bool)>,
    /// Axis 0 = columns, axis 1 = rows.
    pub sizes: [BTreeMap<i32, SizeEntry>; 2],
    pub headings: [BTreeMap<i32, Vec<u8>>; 2],
    pub extra_maps: [BTreeMap<i32, u32>; 2],
    pub extra_mode: u32,
    pub extra: u32,
    /// Native shutdown order is vertical then horizontal.
    pub bars: [Option<Handle>; 2],
    pub bar_images: [Option<Handle>; 2],
    pub sound_ids: [u32; 3],
    pub clicked: [i32; 3],
    pub hovered: [i32; 3],
    pub line_colors: [u32; 2],
    pub line_widths: [u32; 2],
    pub bar_image_rects: [Rect; 2],
    pub click_started: Option<u64>,
}

impl Grid {
    pub fn new(primary: Handle, flags: u32) -> Self {
        Self {
            primary,
            initialized: false,
            flags,
            language: 0,
            dirty: true,
            pause: 0,
            gutters: [5; 4],
            cell_gutters: [1; 4],
            header_size: [20, 80],
            grid_size: [1; 2],
            cell_size: [1; 2],
            position: [0; 2],
            visible: [1.0; 2],
            limits: [-1; 2],
            default_size: [100, 20],
            outline: 0,
            last_set: (-1, -1),
            background: None,
            owner_draw: None,
            header_font: None,
            default_format: Format::default(),
            default_colors: [0; 6],
            selection_mode: 1,
            selection_max: i32::MAX,
            selection: Vec::new(),
            cells: BTreeMap::new(),
            windows: BTreeMap::new(),
            sizes: Default::default(),
            headings: Default::default(),
            extra_maps: Default::default(),
            extra_mode: 0,
            extra: 0,
            bars: [None; 2],
            bar_images: [None; 2],
            sound_ids: [0; 3],
            clicked: [0, -1, -1],
            hovered: [0, -1, -1],
            line_colors: [0; 2],
            line_widths: [1; 2],
            bar_image_rects: [[0; 4]; 2],
            click_started: None,
        }
    }
    pub fn window(&self) -> Handle {
        self.primary + 4
    }
    pub fn query_interface(&self, iid: u32, out: &mut Option<Handle>, f: &mut impl Foreign) -> bool {
        if iid == IID {
            *out = Some(self.primary);
            f.add_ref(self.primary);
            true
        } else {
            f.base_query(self.window(), iid, out)
        }
    }
    pub fn pause_recalculations(&mut self, pause: bool) -> i32 {
        self.pause = self.pause.wrapping_add(if pause { 1 } else { -1 });
        self.pause
    }
    pub fn set_default_color(&mut self, index: u32, color: u32) -> bool {
        if let Some(slot) = self.default_colors.get_mut(index as usize) {
            *slot = color;
            self.default_format.words[index as usize] = color;
            true
        } else {
            false
        }
    }
    pub fn default_color(&self, index: u32, out: &mut u32) -> bool {
        if let Some(color) = self.default_colors.get(index as usize) {
            *out = *color;
            true
        } else {
            false
        }
    }
    pub fn set_default_font(&mut self, font: Option<Handle>, f: &mut impl Foreign) -> bool {
        if self.default_format.font == font {
            return false;
        }
        replace_ref(&mut self.default_format.font, font, f);
        self.dirty = true;
        true
    }
    pub fn set_wrapping(&mut self, on: bool) -> bool {
        let changed = (self.flags & 0x800 != 0) != on;
        if changed {
            self.flags ^= 0x800;
            self.dirty = true;
        }
        changed
    }
    pub fn set_gutters(&mut self, gutters: [i32; 4]) {
        self.gutters = gutters;
        self.dirty = true;
    }
    pub fn set_cell_gutters(&mut self, gutters: [i32; 4]) {
        self.cell_gutters = gutters;
    }
    pub fn set_outline(&mut self, on: bool, color: u32) {
        self.flags = (self.flags & !0x200) | if on { 0x200 } else { 0 };
        self.outline = color;
    }
    pub fn enable_fixed_ranges(&mut self, columns: bool, rows: bool) {
        for (axis, on) in [columns, rows].into_iter().enumerate() {
            let bit = 0x2000 << axis;
            if on {
                self.flags |= bit;
            } else {
                self.flags &= !bit;
                self.limits[axis] = -1;
            }
        }
    }
    pub fn set_selection_rules(&mut self, mode: u32, maximum: i32) {
        self.selection_mode = mode;
        self.selection_max = maximum;
        self.selection.clear();
    }
    pub fn selected(&self, cell: Coordinate) -> bool {
        self.selection.contains(&cell)
    }
    /// Returns whether data changed. A caller must run the recalculation hook
    /// even for false, including when the list was already in the desired state.
    pub fn add_selection(&mut self, cell: Coordinate, select: bool) -> bool {
        if self.selected(cell) == select {
            return false;
        }
        if select {
            if (self.selection.len() as i32) >= self.selection_max {
                return false;
            }
            self.selection.push(cell);
            true
        } else {
            let len = self.selection.len();
            let mode = self.selection_mode;
            self.selection.retain(|c| match mode {
                0 => c.0 != cell.0,
                1 => c.1 != cell.1,
                _ => *c != cell,
            });
            len != self.selection.len()
        }
    }
    pub fn set_selection(&mut self, cell: Coordinate) -> bool {
        if self.selection_max <= 0 || (self.selection.len() == 1 && self.selected(cell)) {
            return false;
        }
        self.selection.clear();
        self.add_selection(cell, true);
        true
    }
    pub fn act_like_listbox(&mut self, columns: i32, maximum: i32, keep_empty: bool) {
        self.limits = [columns, -1];
        self.flags = (self.flags & 0xfffc_2ba6) | 0x2ba6;
        self.selection_mode = 1;
        self.selection_max = maximum;
        if keep_empty {
            self.flags |= 0x1000;
        } else if maximum < 1 {
            self.selection.clear();
        } else if self.selection.is_empty() {
            self.selection.push((0, 0));
        }
    }
    /// Bound helper is a literal fixed-range reading; WG5 asks for the body
    /// contract for negative indices and limits other than -1.
    pub fn within_axis(&self, axis: usize, index: i32) -> bool {
        index >= 0 && (self.flags & (0x2000 << axis) == 0 || self.limits[axis] == -1 || index < self.limits[axis])
    }
    pub fn within(&self, cell: Coordinate) -> bool {
        self.within_axis(0, cell.0) && self.within_axis(1, cell.1)
    }
    pub fn used_range(&self) -> Rect {
        let Some(&(c, r)) = self.cells.keys().next() else {
            return [0; 4];
        };
        self.cells.keys().fold([c, r, c, r], |a, &(c, r)| {
            [a[0].min(c), a[1].min(r), a[2].max(c), a[3].max(r)]
        })
    }
    pub fn get_axis_extra(&mut self, axis: usize, index: i32) -> u32 {
        *self.extra_maps[axis].entry(index).or_default()
    }
    pub fn set_axis_extra(&mut self, axis: usize, index: i32, value: u32) -> bool {
        self.extra_mode = 1;
        if !self.within_axis(axis, index) {
            return false;
        }
        self.extra_maps[axis].insert(index, value);
        true
    }
    pub fn set_extra(&mut self, value: u32) {
        self.extra = value;
        self.extra_mode = 1;
    }
    pub fn set_extra_object(&mut self, value: Option<Handle>, f: &mut impl Foreign) {
        self.extra_mode = 2;
        if let Some(value) = value {
            f.add_ref(value);
        }
        if self.extra != 0 {
            f.release(self.extra as Handle);
        }
        self.extra = value.unwrap_or(0) as u32;
    }
    pub fn heading(&mut self, axis: usize, index: i32) -> &[u8] {
        self.headings[axis].entry(index).or_default()
    }
    pub fn set_heading(&mut self, axis: usize, index: i32, text: &[u8]) {
        self.headings[axis].insert(index, text.to_vec());
    }
    pub fn set_size(&mut self, axis: usize, first: i32, count: i32, value: i32) {
        for i in 0..count {
            self.sizes[axis].entry(first.wrapping_add(i)).or_default().explicit = value.max(0) as i16;
        }
    }
    pub fn reset_cached(&mut self, axis: usize, first: i32, count: i32, value: i32) {
        if count == i32::MAX && value == -1 {
            for entry in self.sizes[axis].values_mut() {
                entry.cached = -1;
            }
            return;
        }
        let count = if count == i32::MAX {
            let limit = self.limits[axis];
            (if matches!(limit, -1 | 0) { i32::MAX } else { limit }).wrapping_sub(first)
        } else {
            count
        };
        for i in 0..count {
            let index = first.wrapping_add(i);
            if value != -1 {
                self.sizes[axis].entry(index).or_default().cached = value as i16;
            } else if let Some(entry) = self.sizes[axis].get_mut(&index) {
                entry.cached = -1;
            }
        }
    }
    pub fn column_width(&mut self, column: i32) -> i32 {
        if let Some(entry) = self.sizes[0].get(&column) {
            if entry.explicit != -1 {
                return entry.explicit as i32;
            }
            if entry.cached != -1 {
                return entry.cached as i32;
            }
        } else if !self.cells.keys().any(|c| c.0 == column) {
            return self.default_size[0];
        }
        let width = self.default_size[0];
        self.reset_cached(0, column, 1, width);
        width
    }
    pub fn grid_area(&self, size: [i32; 2]) -> Rect {
        [
            self.gutters[0],
            self.gutters[1],
            size[0].wrapping_sub(self.gutters[2]),
            size[1].wrapping_sub(self.gutters[3]),
        ]
    }
    pub fn cell_area(&self, size: [i32; 2], f: &mut impl GridForeign) -> Rect {
        let mut rect = self.grid_area(size);
        if self.flags & 8 != 0 {
            rect[1] = rect[1].wrapping_add(self.header_size[0]);
        }
        if self.flags & 16 != 0 {
            rect[0] = rect[0].wrapping_add(self.header_size[1]);
        }
        if let Some(bar) = self.bars[0] {
            rect[2] = rect[2].wrapping_sub(f.window_size(bar)[0]);
        }
        if let Some(bar) = self.bars[1] {
            rect[3] = rect[3].wrapping_sub(f.window_size(bar)[1]);
        }
        rect
    }
    pub fn scroll_to(&mut self, axis: usize, mut index: i32) {
        if index == self.position[axis] {
            return;
        }
        if self.flags & (0x8000 << axis) != 0 {
            if self.limits[axis] > 0 {
                index = index.min(self.limits[axis] - 1);
            }
        } else {
            index = index.min(self.used_range()[axis + 2]);
        }
        self.position[axis] = index;
    }
    pub fn scroll_pages(&mut self, axis: usize, pages: i32) {
        self.scroll_cells(axis, pages.wrapping_mul(4));
    }
    /// Literal relative-scroll interpretation. The slot bodies are pending in
    /// win-grid's inventory (WG5); unlike ScrollTo, its lower clamp is unproved.
    pub fn scroll_cells(&mut self, axis: usize, cells: i32) {
        self.scroll_to(axis, self.position[axis].wrapping_add(cells));
    }
    pub fn wheel(&mut self, delta: i32) {
        self.scroll_cells(1, delta / -100);
    }
    pub fn scroll_step(&mut self, axis: usize, back: bool) {
        let count = self.visible[axis] as i32;
        self.scroll_cells(axis, if back { count.wrapping_neg() } else { count });
    }
    pub fn do_message(&mut self, message: Message, f: &mut impl Foreign) -> bool {
        if message.kind == 3 && message.words[0] as u32 == 0x8871_13a3 {
            let axis = match message.words[1] as u32 {
                0x1a7c_4511 => Some(1),
                0x1a7c_4510 => Some(0),
                _ => None,
            };
            if let Some(axis) = axis {
                if let Some(bar) = self.bars[1 - axis] {
                    let value = f.bar_value(bar);
                    if self.position[axis] != value {
                        self.position[axis] = value;
                        self.dirty = true;
                    }
                }
            }
        }
        f.base_message(self.window(), message)
    }
    pub fn set_sound(&mut self, index: i32, id: u32) -> Result<bool> {
        if index >= 3 {
            return Ok(false);
        }
        if index < 0 {
            return Err(Unsupported::Precondition(
                "negative sound index writes outside sound array",
            ));
        }
        self.sound_ids[index as usize] = id;
        Ok(true)
    }
    pub fn play_sound(&self, index: usize, f: &mut impl GridForeign) -> Result<bool> {
        let id = *self
            .sound_ids
            .get(index)
            .ok_or(Unsupported::Precondition("sound index in raw array"))?;
        if id == 0 {
            return Ok(false);
        }
        let Some(instance) = f.sound_instance(0x5b52_3ba4, [5, id, 0, 0, 0]) else {
            return Ok(false);
        };
        let result = f.play_instance(instance);
        f.release(instance);
        Ok(result)
    }

    pub fn shutdown(&mut self, f: &mut impl GridForeign) {
        let cells = std::mem::take(&mut self.cells);
        for (coordinate, cell) in cells {
            self.dispose(coordinate, cell, f);
        }
        for slot in [
            &mut self.background,
            &mut self.owner_draw,
            &mut self.default_format.font,
            &mut self.header_font,
        ] {
            replace_ref(slot, None, f);
        }
        for slot in &mut self.bars {
            replace_ref(slot, None, f);
        }
        for slot in &mut self.bar_images {
            replace_ref(slot, None, f);
        }
        f.base_shutdown(self.window());
        self.initialized = false;
    }

    fn dispose(&mut self, _: Coordinate, cell: Cell, f: &mut impl GridForeign) {
        match cell.value {
            Value::Window { handle, .. } => {
                if self.windows.remove(&handle).is_some() {
                    f.child_remove(self.window(), handle);
                    f.release(handle);
                }
            }
            Value::Bitmap(handle) => f.release(handle),
            _ => {}
        }
        if let Some(format) = cell.format {
            if let Some(font) = format.font {
                f.release(font);
            }
        }
        if self.extra_mode == 2 && cell.extra != 0 {
            f.release(cell.extra as Handle);
        }
    }

    pub fn clear_cell(&mut self, cell: Coordinate, f: &mut impl GridForeign) -> bool {
        self.add_selection(cell, false);
        if let Some(record) = self.cells.remove(&cell) {
            self.dispose(cell, record, f);
            self.dirty = true;
        }
        true
    }

    pub fn resolve_append(&self, mut cell: Coordinate) -> Coordinate {
        if cell.0 == i32::MAX {
            cell.0 = if cell.1 == i32::MAX {
                self.cells.keys().map(|c| c.0).max().unwrap_or(0)
            } else {
                self.cells
                    .keys()
                    .filter(|c| c.1 == cell.1)
                    .map(|c| c.0)
                    .max()
                    .map_or(0, |c| c.wrapping_add(1))
            };
        }
        if cell.1 == i32::MAX {
            cell.1 = self
                .cells
                .keys()
                .filter(|c| c.0 == cell.0)
                .map(|c| c.1)
                .max()
                .map_or(0, |r| r.wrapping_add(1));
        }
        cell
    }

    pub fn copy_cell(
        &mut self,
        source: Coordinate,
        mut destination: Coordinate,
        copy: bool,
        f: &mut impl GridForeign,
    ) -> bool {
        if !self.within(source) {
            return false;
        }
        if source == destination {
            return true;
        }
        if !self.within(destination) {
            if copy {
                return true;
            }
            destination = source;
        }
        if destination == source {
            return true;
        }
        let Some(mut record) = (if copy {
            self.cells.get(&source).cloned()
        } else {
            self.cells.remove(&source)
        }) else {
            self.clear_cell(destination, f);
            return true;
        };
        if let Some(previous) = self.cells.remove(&destination) {
            self.dispose(destination, previous, f);
        }
        if copy {
            if matches!(record.value, Value::Window { .. }) {
                self.cells.insert(destination, Cell::default());
                return true;
            }
            if let Value::Bitmap(bitmap) = record.value {
                f.add_ref(bitmap);
            }
            if let Some(font) = record.format.as_ref().and_then(|fmt| fmt.font) {
                f.add_ref(font);
            }
            record.extra = 0;
        }
        self.cells.insert(destination, record);
        self.dirty = true;
        true
    }

    /// §5.11's five movement modes. The owner runs the loops in the specified
    /// direction so overlap cannot silently change which record is moved.
    pub fn make_room(&mut self, cell: Coordinate, n: i32, mut mode: u32, f: &mut impl GridForeign) -> bool {
        if mode == 0 {
            mode = 1;
        }
        if !(1..=5).contains(&mode) {
            return false;
        }
        self.reset_cached(1, cell.1, 1, -1);
        if mode == 1 {
            if n > 0 {
                if let Some(record) = self.cells.remove(&cell) {
                    self.dispose(cell, record, f);
                    // §5.11 retains the matrix entry until its caller overwrites
                    // it; represent the disposed payload safely as empty.
                    self.cells.insert(cell, Cell::default());
                }
            }
        } else {
            let axis = if matches!(mode, 2 | 4) { 0 } else { 1 };
            let position = if axis == 0 { cell.0 } else { cell.1 };
            let mut indices = self
                .cells
                .keys()
                .filter(|c| if axis == 0 { c.1 == cell.1 } else { c.0 == cell.0 })
                .map(|c| if axis == 0 { c.0 } else { c.1 });
            if let Some(first) = indices.next() {
                let (min, max) = indices.fold((first, first), |(min, max), i| (min.min(i), max.max(i)));
                let coord = |i| if axis == 0 { (i, cell.1) } else { (cell.0, i) };
                let mut moves = Vec::new();
                if matches!(mode, 2 | 3) {
                    if n > 0 {
                        for c in min.wrapping_sub(n).max(0)..position {
                            moves.push((c.wrapping_add(n), c));
                        }
                    } else if n < 0 {
                        for c in position.wrapping_sub(n)..=max {
                            moves.push((c, c.wrapping_add(n)));
                        }
                    }
                } else if n > 0 {
                    let end = max.wrapping_add(n);
                    let end = if self.limits[axis] == -1 {
                        end
                    } else {
                        end.min(self.limits[axis])
                    };
                    if end >= position {
                        for c in (position..=end).rev() {
                            moves.push((c.wrapping_sub(n), c));
                        }
                    }
                } else if n < 0 && position > min {
                    for c in (min..position).rev() {
                        moves.push((c, c.wrapping_sub(n)));
                    }
                }
                for (source, destination) in moves {
                    self.reset_cached(axis, source, 1, -1);
                    self.reset_cached(axis, destination, 1, -1);
                    self.copy_cell(coord(source), coord(destination), false, f);
                }
            }
        }
        self.dirty = true;
        true
    }

    pub fn set_text(
        &mut self,
        cell: Coordinate,
        text: Option<&[u8]>,
        count: i32,
        mode: u32,
        f: &mut impl GridForeign,
    ) -> Result<bool> {
        if self
            .cells
            .get(&cell)
            .is_some_and(|c| matches!(c.value, Value::Window { .. }))
        {
            self.clear_cell(cell, f);
        }
        let cell = self.resolve_append(cell);
        if !self.within(cell) {
            self.clear_cell(cell, f);
            return Ok(true);
        }
        let Some(text) = text else {
            self.clear_cell(cell, f);
            return Ok(true);
        };
        let count = if matches!(count, -1 | 0) {
            text.iter().position(|&b| b == 0).unwrap_or(text.len())
        } else {
            usize::try_from(count).map_err(|_| Unsupported::Precondition("text count nonnegative or -1"))?
        };
        let text = text
            .get(..count)
            .ok_or(Unsupported::Precondition("text source covers count"))?;
        self.make_room(cell, 1, mode, f);
        self.cells.insert(
            cell,
            Cell {
                value: Value::Text(text.to_vec()),
                ..Cell::default()
            },
        );
        self.last_set = cell;
        self.dirty = true;
        Ok(true)
    }

    pub fn set_bitmap(
        &mut self,
        cell: Coordinate,
        bitmap: Option<Handle>,
        mode: u32,
        f: &mut impl GridForeign,
    ) -> bool {
        let cell = self.resolve_append(cell);
        if !self.within(cell) {
            self.clear_cell(cell, f);
            return false;
        }
        let Some(bitmap) = bitmap else {
            self.clear_cell(cell, f);
            return true;
        };
        f.add_ref(bitmap);
        self.make_room(cell, 1, mode, f);
        self.cells.insert(
            cell,
            Cell {
                value: Value::Bitmap(bitmap),
                ..Cell::default()
            },
        );
        self.last_set = cell;
        self.dirty = true;
        true
    }

    #[allow(clippy::too_many_arguments)] // Native SetCellWindow argument list.
    pub fn set_window(
        &mut self,
        cell: Coordinate,
        window: Option<Handle>,
        keep_width: bool,
        keep_height: bool,
        mode: u32,
        f: &mut impl GridForeign,
    ) -> bool {
        if let Some(window) = window {
            if self.windows.contains_key(&window) {
                return false;
            }
        }
        let cell = self.resolve_append(cell);
        if !self.within(cell) {
            self.clear_cell(cell, f);
            return false;
        }
        let Some(window) = window else {
            self.clear_cell(cell, f);
            return true;
        };
        f.add_ref(window);
        self.make_room(cell, 1, mode, f);
        self.cells.insert(
            cell,
            Cell {
                value: Value::Window {
                    handle: window,
                    keep_width,
                    keep_height,
                },
                ..Cell::default()
            },
        );
        f.add_ref(window);
        self.windows.insert(window, (cell, keep_width, keep_height));
        f.send_to_back(window);
        f.window_flag(window, 0x800, false);
        f.child_add(self.window(), window);
        self.place_window(cell, window, keep_width, keep_height, true, f);
        f.release(window);
        self.last_set = cell;
        self.dirty = true;
        true
    }

    pub fn cell_height(&mut self, cell: Coordinate, f: &mut impl GridForeign) -> i32 {
        let mut height = self.default_size[1];
        if let Some(record) = self.cells.get(&cell).cloned() {
            match &record.value {
                Value::Window { handle, .. } => height = f.window_size(*handle)[1],
                Value::Bitmap(handle) => height = f.buffer_height(*handle),
                Value::Text(text) if !text.is_empty() => {
                    // Literal precedence: explicit cell, row, column, default.
                    // The GetCellFormat slot's missing merge rules are WG5.
                    let font = record
                        .format
                        .as_ref()
                        .and_then(|fmt| fmt.font)
                        .or_else(|| self.sizes[1].get(&cell.1).and_then(|s| s.format.font))
                        .or_else(|| self.sizes[0].get(&cell.0).and_then(|s| s.format.font))
                        .or(self.default_format.font);
                    if let Some(font) = font {
                        let line = f.font_height(font);
                        if line != 0 {
                            let lines = if self.flags & (0x800 | 0x20000) != 0 {
                                1
                            } else {
                                let width = self.column_width(cell.0);
                                f.line_count(font, text, width)
                            };
                            height = lines.wrapping_mul(line);
                        }
                    }
                }
                _ => {}
            }
        }
        self.cell_gutters[1]
            .wrapping_add(self.cell_gutters[3])
            .wrapping_add(height)
    }

    pub fn row_height(&mut self, row: i32, f: &mut impl GridForeign) -> i32 {
        if let Some(entry) = self.sizes[1].get(&row) {
            if entry.explicit != -1 {
                return entry.explicit as i32;
            }
            if entry.cached != -1 {
                return entry.cached as i32;
            }
        } else if !self.cells.keys().any(|c| c.1 == row) {
            return self.default_size[1];
        }
        let mut height = if matches!(self.limits[1], -1 | 0) {
            self.default_size[1]
        } else {
            self.cell_size[1].wrapping_div(self.limits[1])
        };
        let cells: Vec<_> = self.cells.keys().copied().filter(|c| c.1 == row).collect();
        for cell in cells {
            height = height.max(self.cell_height(cell, f));
        }
        self.reset_cached(1, row, 1, height);
        height
    }

    pub fn size_sum(&mut self, axis: usize, first: i32, count: i32, f: &mut impl GridForeign) -> i32 {
        (0..count).fold(0_i32, |sum, i| {
            sum.wrapping_add(if axis == 0 {
                self.column_width(first.wrapping_add(i))
            } else {
                self.row_height(first.wrapping_add(i), f)
            })
        })
    }

    pub fn cell_rect(&mut self, cell: Coordinate, f: &mut impl GridForeign) -> Rect {
        let size = f.window_size(self.window());
        let area = self.cell_area(size, f);
        let left = self
            .size_sum(0, 0, cell.0, f)
            .wrapping_sub(self.size_sum(0, 0, self.position[0], f))
            .wrapping_add(area[0]);
        let top = self
            .size_sum(1, 0, cell.1, f)
            .wrapping_sub(self.size_sum(1, 0, self.position[1], f))
            .wrapping_add(area[1]);
        [
            left,
            top,
            left.wrapping_add(self.column_width(cell.0)),
            top.wrapping_add(self.row_height(cell.1, f)),
        ]
    }

    fn place_window(
        &mut self,
        cell: Coordinate,
        window: Handle,
        keep_width: bool,
        keep_height: bool,
        final_area: bool,
        f: &mut impl GridForeign,
    ) {
        if !self.initialized {
            return;
        }
        let visible = cell.0 >= self.position[0]
            && cell.1 >= self.position[1]
            && (cell.0 as f32) < self.position[0] as f32 + self.visible[0]
            && (cell.1 as f32) < self.position[1] as f32 + self.visible[1];
        if !visible {
            f.show(window, false);
            return;
        }
        let mut rect = self.cell_rect(cell, f);
        for axis in 0..2 {
            rect[axis] = rect[axis].wrapping_add(self.cell_gutters[axis]);
            rect[axis + 2] = rect[axis + 2].wrapping_sub(self.cell_gutters[axis + 2]);
        }
        f.show(window, true);
        f.move_window(window, rect[0], rect[1]);
        if keep_width {
            f.set_width(window, rect[2].wrapping_sub(rect[0]));
        }
        if keep_height {
            f.set_height(window, rect[3].wrapping_sub(rect[1]));
        }
        if final_area {
            f.set_area(window, rect);
        }
    }

    pub fn place_all_windows(&mut self, f: &mut impl GridForeign) {
        let windows: Vec<_> = self
            .windows
            .iter()
            .map(|(&handle, &(cell, width, height))| (cell, handle, width, height))
            .collect();
        for (cell, handle, width, height) in windows {
            self.place_window(cell, handle, width, height, false, f);
        }
    }

    /// Layout's content counts before the scrollbar addition/removal pass.
    /// The caller passes the shared context; no static mutable counter is used.
    pub fn recalculate_content(&mut self, context: &mut LayoutContext, f: &mut impl GridForeign) {
        if context.depth >= 2 {
            return;
        }
        context.depth += 1;
        if self.pause < 1 && self.initialized {
            self.calculate_visible(f);
            if context.depth == 1 {
                self.place_all_windows(f);
            }
        }
        context.depth -= 1;
        self.dirty = false;
    }

    fn calculate_visible(&mut self, f: &mut impl GridForeign) {
        let size = f.window_size(self.window());
        let grid = self.grid_area(size);
        let cell = self.cell_area(size, f);
        self.grid_size = [grid[2].wrapping_sub(grid[0]), grid[3].wrapping_sub(grid[1])];
        self.cell_size = [cell[2].wrapping_sub(cell[0]), cell[3].wrapping_sub(cell[1])];
        let used = self.used_range();
        for axis in 0..2 {
            let mut bound = if matches!(self.limits[axis], -1 | 0) {
                i32::MAX
            } else {
                self.limits[axis]
            };
            if self.flags & (0x8000 << axis) == 0 && used[axis + 2] <= bound {
                bound = used[axis + 2].wrapping_add(1);
            }
            let mut total = 0_i32;
            let mut visible = 0_f32;
            for index in self.position[axis]..bound {
                let size = if axis == 0 {
                    self.column_width(index)
                } else {
                    self.row_height(index, f)
                };
                let previous = total;
                total = total.wrapping_add(size);
                if total <= self.cell_size[axis] {
                    visible += 1.0;
                } else {
                    let fraction = (self.cell_size[axis].wrapping_sub(previous) as f64
                        / total.wrapping_sub(previous) as f64) as f32;
                    if fraction.abs() > 0.05_f32 {
                        visible += fraction;
                    }
                    break;
                }
            }
            self.visible[axis] = visible;
        }
    }
}
