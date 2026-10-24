// SPDX-License-Identifier: GPL-3.0-or-later
//! Window service boundary and owner input/layout algorithms (win-grid §5).
use super::*;

/// Only individual game-interface operations cross this boundary. References
/// returned by create_grid_bar/create_editor are created, not borrowed.
pub trait GridHost: GridDataForeign {
    fn enabled(&mut self, window: Handle) -> bool;
    fn capturing(&mut self, window: Handle) -> bool;
    fn focused(&mut self) -> Option<Handle>;
    fn focus(&mut self, window: Handle);
    fn base_focus(&mut self, window: Handle) -> bool;
    fn now_ms(&mut self) -> u64;
    fn create_grid_bar(&mut self, class: u32, style: u32, range: [i32; 2]) -> Option<Handle>;
    fn bar_parameters(&mut self, bar: Handle, range: [i32; 2], steps: [i32; 2], position: i32);
    fn image_area(&mut self, image: Handle) -> Rect;
    fn bar_image(&mut self, bar: Handle, image: Handle, source: Rect);
    fn point_in_bar(&mut self, bar: Handle, point: [i32; 2]) -> Option<[i32; 2]>;
    fn mouse_holder(&mut self, window: Handle);
    fn bar_mouse(&mut self, bar: Handle, kind: u32, point: [i32; 2], modifiers: u32) -> bool;
    fn notify(&mut self, source: Handle, id: u32, data: &[i32]) -> bool;
    fn create_editor(&mut self, style: u32) -> Option<Handle>;
    fn editor_text(&mut self, editor: Handle, text: Option<&[u8]>, count: i32);
    /// Borrowed base-window buffer and its existing drawing rectangle.
    fn window_buffer(&mut self, window: Handle) -> (Option<Handle>, Rect);
    fn base_fill_color(&mut self, window: Handle, color: u32);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Down { point: [i32; 2], modifiers: u32 },
    Up { point: [i32; 2], modifiers: u32 },
    Move { point: [i32; 2], modifiers: u32 },
    KeyDown { key: u32, modifiers: u32 },
    KeyUp { key: u32, modifiers: u32 },
    Wheel(i32),
    Character,
    Focus(bool),
}

/// Named owner bodies explicitly pending in the source specification. These
/// cannot be implemented by guessing from similarly named controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingOwner {
    SortColumn,
    SortRow,
    ResizeColumnToFitCellWindows,
    SetCellColors,
    SetCellWrapping,
    SetCellAlignment,
    SetCellBorder,
}

/// Live counterparts of the data-stage methods: unlike direct state edits,
/// these execute the documented recalculation hook before returning.
pub enum Mutation<'a> {
    DefaultFont(Option<Handle>),
    Wrapping(bool),
    Gutters([i32; 4]),
    ScrollTo {
        axis: usize,
        index: i32,
    },
    ScrollCells {
        axis: usize,
        count: i32,
    },
    ScrollPages {
        axis: usize,
        count: i32,
    },
    Selection(Coordinate),
    AddSelection(Coordinate, bool),
    Clear(Coordinate),
    Text {
        cell: Coordinate,
        text: Option<&'a [u8]>,
        count: i32,
        mode: u32,
    },
    Bitmap {
        cell: Coordinate,
        image: Option<Handle>,
        mode: u32,
    },
    Window {
        cell: Coordinate,
        window: Option<Handle>,
        keep: [bool; 2],
        mode: u32,
    },
    Format(Coordinate, &'a Format),
    Delete {
        axis: usize,
        first: i32,
        count: i32,
        mode: u32,
    },
    Copy {
        axis: usize,
        from: i32,
        to: i32,
        count: u32,
        move_cells: bool,
    },
    MakeRoom {
        cell: Coordinate,
        count: i32,
        mode: u32,
    },
}

pub(super) fn inside(r: Rect, p: [i32; 2]) -> bool {
    p[0] >= r[0] && p[0] <= r[2] && p[1] >= r[1] && p[1] <= r[3]
}

impl Grid {
    pub fn pending_owner(&self, _: PendingOwner) -> Result<()> {
        Err(Unsupported::Open("WG8"))
    }

    pub fn set_fill_color(&mut self, color: u32, context: &mut LayoutContext, f: &mut impl GridHost) {
        f.base_fill_color(self.window(), color);
        if self.flags & 0x100 != 0 {
            self.recalculate(context, f);
        }
    }

    pub fn buffer_to_draw_to(&self, f: &mut impl GridHost) -> (Option<Handle>, Rect) {
        f.window_buffer(self.window())
    }

    pub fn first_selection(&self, output: &mut Coordinate) -> bool {
        if let Some(&cell) = self.selection.first() {
            *output = cell;
            true
        } else {
            false
        }
    }

    pub fn mutate(
        &mut self,
        mutation: Mutation<'_>,
        context: &mut LayoutContext,
        f: &mut impl GridHost,
    ) -> Result<bool> {
        let (result, hook) = match mutation {
            Mutation::DefaultFont(font) => {
                let changed = self.set_default_font(font, f);
                (true, changed)
            }
            Mutation::Wrapping(on) => {
                let changed = self.set_wrapping(on);
                (true, changed)
            }
            Mutation::Gutters(gutters) => {
                self.set_gutters(gutters);
                (true, true)
            }
            Mutation::ScrollTo { axis, index } => {
                let changed = self.position[axis] != index;
                self.scroll_to(axis, index);
                (true, changed)
            }
            Mutation::ScrollCells { axis, count } => {
                self.scroll_cells(axis, count);
                (true, true)
            }
            Mutation::ScrollPages { axis, count } => {
                self.scroll_pages(axis, count);
                (true, true)
            }
            Mutation::Selection(cell) => {
                let changed = self.set_selection(cell);
                (changed, changed)
            }
            Mutation::AddSelection(cell, on) => (self.add_selection(cell, on), true),
            Mutation::Clear(cell) => (self.clear_cell(cell, f), true),
            Mutation::Text {
                cell,
                text,
                count,
                mode,
            } => (self.set_text(cell, text, count, mode, f)?, true),
            Mutation::Bitmap { cell, image, mode } => (self.set_bitmap(cell, image, mode, f), true),
            Mutation::Window {
                cell,
                window,
                keep,
                mode,
            } => (self.set_window(cell, window, keep[0], keep[1], mode, f), true),
            Mutation::Format(cell, format) => (
                self.set_cell_format_state(cell, format, f)?,
                cell.0 != -1 && cell.1 != -1,
            ),
            Mutation::Delete {
                axis,
                first,
                count,
                mode,
            } => {
                let result = if axis == 0 {
                    self.delete_columns(first, count, mode, f)
                } else {
                    self.delete_rows(first, count, mode, f)
                };
                (result, count > 0 && result)
            }
            Mutation::Copy {
                axis,
                from,
                to,
                count,
                move_cells,
            } => {
                let result = if axis == 0 {
                    self.copy_columns(from, to, count, move_cells, f)?
                } else {
                    self.copy_rows(from, to, count, move_cells, f)?
                };
                (result, false) // §5.13 copy only marks dirty; no hook.
            }
            Mutation::MakeRoom { cell, count, mode } => (self.make_room(cell, count, mode, f), false),
        };
        if hook {
            self.recalculate(context, f);
        }
        Ok(result)
    }

    /// A safe Rust owner returns zero to its allocator instead of deleting
    /// itself through a native pointer. The caller drops the value at zero.
    pub fn release(&mut self, references: &mut u32, f: &mut impl GridHost) -> u32 {
        if *references > 1 {
            *references -= 1;
        } else {
            self.shutdown(f);
            *references = 0;
        }
        *references
    }

    pub fn set_cell_extra_object(&mut self, cell: Coordinate, object: Option<Handle>, f: &mut impl GridHost) -> bool {
        if !self.within(cell) {
            return false;
        }
        if let Some(object) = object {
            f.add_ref(object);
        }
        let previous = self.cell_extra(cell);
        if previous != 0 {
            f.release(previous as Handle);
        }
        // WG4: preserve the measured store of grid extra, not the argument.
        self.extra_mode = 1;
        self.cells.entry(cell).or_default().extra = self.extra;
        self.extra_mode = 2;
        true
    }
    /// WG6 leaves the default-format initializer unsettled. Initialization
    /// still performs the settled lifecycle; callers must supply resolved
    /// formatting before rendering and cannot claim initializer parity.
    pub fn init(&mut self, context: &mut LayoutContext, f: &mut impl GridHost) -> bool {
        if !self.initialized {
            f.base_init(self.window());
            self.initialized = true;
            self.language = f.language_value().unwrap_or(1);
            let _ = self.default_formatting();
            self.recalculate(context, f);
        }
        true
    }

    pub fn default_formatting(&mut self) -> Result<()> {
        Err(Unsupported::Open("WG6"))
    }

    pub fn set_area_live(&mut self, rect: Rect, context: &mut LayoutContext, f: &mut impl GridHost) {
        f.set_area(self.window(), rect);
        if self.initialized {
            self.dirty = true;
            self.recalculate(context, f);
        }
    }

    pub fn set_window_flag(&self, flag: u32, on: bool, f: &mut impl GridHost) {
        if flag == 2 {
            for bar in self.bars.into_iter().flatten() {
                f.window_flag(bar, flag, on);
            }
        }
        f.window_flag(self.window(), flag, on);
    }

    pub fn set_flags(&mut self, flags: u32, context: &mut LayoutContext, f: &mut impl GridHost) {
        let changed = self.flags ^ flags;
        self.flags = flags;
        if changed & 6 != 0 {
            self.remove_bars([flags & 4 == 0, flags & 2 == 0], f);
            self.dirty = true;
            self.recalculate(context, f);
        }
        if changed & 0x40000 != 0 {
            let on = flags & 0x40000 != 0;
            if f.capturing(self.window()) != on {
                f.capture(self.window(), on);
            }
        }
    }

    pub fn message(&mut self, message: Message, context: &mut LayoutContext, f: &mut impl GridHost) -> bool {
        // The recalc must precede the unconditional base delegation.
        if message.kind == 3 && message.words[0] as u32 == 0x8871_13a3 {
            let bar_index = match message.words[1] as u32 {
                0x1a7c_4511 => Some(0),
                0x1a7c_4510 => Some(1),
                _ => None,
            };
            if let Some(i) = bar_index {
                if let Some(bar) = self.bars[i] {
                    let axis = 1 - i;
                    let value = f.bar_value(bar);
                    if self.position[axis] != value {
                        self.position[axis] = value;
                        self.dirty = true;
                        self.recalculate(context, f);
                    }
                }
            }
        }
        f.base_message(self.window(), message)
    }

    pub fn remove_bars(&mut self, requested: [bool; 2], f: &mut impl GridHost) {
        for (i, remove) in requested.into_iter().enumerate() {
            if remove {
                if let Some(bar) = self.bars[i].take() {
                    f.child_delete(self.window(), bar);
                    f.release(bar);
                    self.dirty = true;
                }
            }
        }
    }

    pub fn add_bars(&mut self, requested: [bool; 2], f: &mut impl GridHost) {
        for (i, add) in requested.into_iter().enumerate() {
            let flag = if i == 0 { 4 } else { 2 };
            if !add || self.flags & flag == 0 || self.bars[i].is_some() {
                continue;
            }
            let focused = f.focused();
            let Some(bar) = f.create_grid_bar(0x1a7c_4511 - i as u32, 2 - i as u32, [0, 100]) else {
                self.flags &= !flag;
                continue;
            };
            self.bars[i] = Some(bar);
            if let Some(image) = self.bar_images[i] {
                let source = self.image_source(i, image, f);
                f.bar_image(bar, image, source);
            }
            f.child_add(self.window(), bar);
            if let Some(window) = focused {
                f.focus(window);
            }
            self.dirty = true;
        }
    }

    fn image_source(&self, i: usize, image: Handle, f: &mut impl GridHost) -> Rect {
        let r = self.bar_image_rects[i];
        if r[2] <= r[0] || r[3] <= r[1] {
            f.image_area(image)
        } else {
            r
        }
    }

    pub fn set_bar_image(&mut self, vertical: bool, image: Option<Handle>, f: &mut impl GridHost) {
        let i = usize::from(!vertical);
        replace_ref(&mut self.bar_images[i], image, f);
        if let (Some(image), Some(bar)) = (image, self.bars[i]) {
            let source = self.image_source(i, image, f);
            f.bar_image(bar, image, source);
        }
    }

    pub fn bar_values(&self, f: &mut impl GridHost) {
        let used = self.used_range();
        for (i, bar) in self.bars.into_iter().enumerate() {
            let axis = 1 - i;
            let visible = self.visible[axis];
            if (i == 0 && visible <= 0.0) || (i == 1 && visible == 0.0) {
                continue;
            }
            if let Some(bar) = bar {
                let unbounded = self.flags & (0x8000 << axis) != 0;
                let (maximum, steps) = if unbounded && matches!(self.limits[axis], -1 | 0) {
                    (visible as i32, [i32::MAX; 2])
                } else {
                    let total = if unbounded {
                        self.limits[axis]
                    } else {
                        used[axis + 2].wrapping_add(1)
                    };
                    let maximum = total.wrapping_sub(visible as i32).max(0);
                    (maximum, [maximum, (total as f64 / visible as f64 + 1.0) as i32])
                };
                f.bar_parameters(bar, [0, maximum], steps, self.position[axis]);
            }
        }
    }

    pub fn bar_positions(&self, f: &mut impl GridHost) {
        let [w, h] = f.window_size(self.window());
        let bw = self.bars[0].map_or(0, |b| f.window_size(b)[0]);
        let bh = self.bars[1].map_or(0, |b| f.window_size(b)[1]);
        if let Some(bar) = self.bars[0] {
            let top = if self.flags & 8 != 0 {
                self.header_size[0] + self.gutters[1] + i32::from(self.flags & 0x200 != 0)
            } else {
                0
            };
            f.set_area(bar, [w - bw, top, w, h - bh]);
        }
        if let Some(bar) = self.bars[1] {
            let left = if self.flags & 0x10 != 0 {
                self.header_size[1] + 1 + self.gutters[0] + i32::from(self.flags & 0x200 != 0)
            } else {
                0
            };
            f.set_area(bar, [left, h - bh, w - bw, h]);
        }
    }

    pub fn test_bars(&mut self, f: &mut impl GridHost) {
        let used = self.used_range();
        let always = self.flags & 0x10000 != 0 && matches!(self.limits[1], -1 | 0);
        let mut wanted = [false; 2];
        for (i, value) in wanted.iter_mut().enumerate() {
            let axis = 1 - i;
            let flag = if i == 0 { 4 } else { 2 };
            let total = if self.flags & (0x8000 << axis) != 0 {
                self.limits[axis]
            } else {
                used[axis + 2].wrapping_add(1)
            };
            // Both axes intentionally consult the row's unbounded state.
            *value = self.flags & flag != 0 && (always || total as f32 > self.visible[axis]);
        }
        self.remove_bars([!wanted[0], !wanted[1]], f);
        self.add_bars(wanted, f);
        if self.bars.iter().any(Option::is_some) {
            self.bar_values(f);
            self.bar_positions(f);
        }
    }

    pub fn recalculate(&mut self, context: &mut LayoutContext, f: &mut impl GridHost) {
        if context.depth >= 2 {
            return;
        }
        context.depth += 1;
        if self.pause < 1 && self.initialized {
            self.calculate_visible(f);
            let bars = self.bars;
            self.test_bars(f);
            if bars != self.bars {
                self.recalculate(context, f);
            }
            if context.depth == 1 {
                self.place_all_windows(f);
            }
        }
        context.depth -= 1;
        self.dirty = false;
    }

    pub fn hit_test(&mut self, point: [i32; 2], f: &mut impl GridHost) -> Option<[i32; 3]> {
        let size = f.window_size(self.window());
        if !inside(self.grid_area(size), point) {
            return Some([1, 0, 0]);
        }
        let region = if inside(self.cell_area(size, f), point) {
            5
        } else if self.flags & 0x18 == 0x18 {
            let col = point[1] >= self.gutters[1] && point[1] < self.header_size[0];
            let row = point[0] >= self.gutters[0] && point[0] < self.header_size[1];
            match (col, row) {
                (true, true) => return Some([4, 0, 0]),
                (true, false) => 3,
                (false, true) => 2,
                _ => return None,
            }
        } else {
            return None;
        };
        let mut hit = [region, 0, 0];
        for axis in 0..2 {
            for i in self.position[axis]..self.position[axis].wrapping_add(self.visible[axis].ceil() as i32) {
                let cell = if axis == 0 { (i, 0) } else { (0, i) };
                if point[axis] < self.cell_rect(cell, f)[axis + 2] {
                    hit[axis + 1] = i;
                    break;
                }
            }
        }
        if region == 3 {
            hit[2] = 0;
        }
        if region == 2 {
            hit[1] = 0;
        }
        Some(hit)
    }

    fn select_live(&mut self, cell: Coordinate, on: bool, context: &mut LayoutContext, f: &mut impl GridHost) {
        self.add_selection(cell, on);
        self.recalculate(context, f);
    }

    fn route_bar(&self, kind: u32, point: [i32; 2], modifiers: u32, f: &mut impl GridHost) -> Option<bool> {
        for bar in self.bars.into_iter().flatten() {
            if let Some(local) = f.point_in_bar(bar, point) {
                let size = f.window_size(bar);
                if local[0] >= 0 && local[1] >= 0 && local[0] < size[0] && local[1] < size[1] {
                    f.mouse_holder(bar);
                    let result = f.bar_mouse(bar, kind, local, modifiers);
                    f.capture(self.window(), true);
                    return Some(result);
                }
            }
        }
        None
    }

    pub fn input(&mut self, input: Input, context: &mut LayoutContext, f: &mut impl GridHost) -> bool {
        match input {
            Input::Character => false,
            Input::Wheel(delta) => {
                self.wheel(delta);
                self.recalculate(context, f);
                true
            }
            Input::Focus(on) => {
                if self.flags & 1 != 0 {
                    self.recalculate(context, f);
                }
                on && f.base_focus(self.window())
            }
            Input::KeyUp { key, modifiers } => key != 0x1b || f.notify(self.primary, 6, &[0x1b, modifiers as i32, 0]),
            Input::KeyDown { key, modifiers } => self.key_down(key, modifiers, context, f),
            Input::Down { point, modifiers } => self.mouse(7, point, modifiers, context, f),
            Input::Up { point, modifiers } => self.mouse(10, point, modifiers, context, f),
            Input::Move { point, modifiers } => self.mouse(13, point, modifiers, context, f),
        }
    }

    fn mouse(
        &mut self,
        kind: u32,
        point: [i32; 2],
        modifiers: u32,
        context: &mut LayoutContext,
        f: &mut impl GridHost,
    ) -> bool {
        if kind == 7 && !f.enabled(self.window()) {
            return true;
        }
        let drop = self.flags & 0x40000 != 0 && f.capturing(self.window());
        if drop {
            let [w, h] = f.window_size(self.window());
            if !inside([0, 0, w, h], point) {
                if kind == 7 {
                    f.notify(self.primary, 0x9a15_52d7, &[]);
                }
                return true;
            }
            if let Some(result) = self.route_bar(kind, point, modifiers, f) {
                return result;
            }
            if kind == 7 {
                return true;
            }
        } else if kind == 10 {
            return true;
        }
        let now = f.now_ms();
        let mut double = false;
        if kind == 7 {
            double = self.click_started.is_some_and(|start| now.wrapping_sub(start) < 500);
            self.click_started = if double { None } else { Some(now) };
        }
        let Some(hit) = self.hit_test(point, f) else {
            return true;
        };
        let cell = (hit[1], hit[2]);
        if kind == 10 {
            self.clicked = hit;
            f.notify(self.primary, 0x9a15_52d3, &self.clicked);
        } else if kind == 13 {
            if drop {
                if hit[0] == 5 && hit != self.clicked {
                    self.selection.clear();
                    self.select_live(cell, true, context, f);
                    self.clicked = hit;
                }
            } else {
                if matches!(hit[0], 2 | 3 | 5) && hit != self.hovered {
                    f.notify(self.primary, 0x9a15_52d6, &hit);
                }
                self.hovered = hit;
            }
        } else {
            match hit[0] {
                2 | 3 => {
                    self.selection.clear();
                    let used = self.used_range();
                    let axis = if hit[0] == 2 { 0 } else { 1 };
                    for i in used[axis]..=used[axis + 2] {
                        self.select_live(if axis == 0 { (i, cell.1) } else { (cell.0, i) }, true, context, f);
                    }
                }
                5 => match modifiers & 7 {
                    1 => {
                        if let Some(&last) = self.selection.last() {
                            for c in last.0.min(cell.0)..=last.0.max(cell.0) {
                                for r in last.1.min(cell.1)..=last.1.max(cell.1) {
                                    self.select_live((c, r), true, context, f);
                                }
                            }
                        } else {
                            self.selection.push(cell);
                        }
                    }
                    2 => self.select_live(cell, !self.selected(cell), context, f),
                    0 => {
                        self.selection.clear();
                        self.select_live(cell, true, context, f);
                    }
                    _ => {}
                },
                _ => {}
            }
            let changed = hit != self.clicked;
            self.clicked = hit;
            if double && !changed {
                f.notify(self.primary, 0x9a15_52d4, &self.clicked);
            } else {
                f.notify(self.primary, 0x9a15_52d3, &self.clicked);
                if changed {
                    f.notify(self.primary, 0x9a15_52d2, &[cell.0, cell.1]);
                    self.recalculate(context, f);
                }
            }
        }
        true
    }

    fn key_down(&mut self, key: u32, modifiers: u32, context: &mut LayoutContext, f: &mut impl GridHost) -> bool {
        if !f.enabled(self.window()) {
            return true;
        }
        match key {
            0x23 => {
                let used = self.used_range();
                for axis in 0..2 {
                    if self.flags & (0x8000 << axis) == 0 {
                        self.scroll_to(axis, used[axis + 2]);
                    } else if !matches!(self.limits[axis], -1 | 0) {
                        self.scroll_to(axis, self.limits[axis].wrapping_sub(1));
                    }
                }
                self.recalculate(context, f);
            }
            0x24 => {
                self.scroll_to(0, 0);
                self.scroll_to(1, 0);
                self.recalculate(context, f);
            }
            0x21 | 0x22 => {
                self.scroll_step(usize::from(modifiers & 4 == 0), key == 0x21);
                self.recalculate(context, f);
            }
            0x25..=0x28 => self.arrow(key, modifiers, context, f),
            0x2d => self.flags ^= 0x80,
            0x1b => {
                f.notify(self.primary, 5, &[0x1b, modifiers as i32, 0]);
            }
            _ => {}
        }
        f.notify(self.primary, 0x9a15_52d5, &[key as i32]);
        true
    }

    pub fn arrow(&mut self, key: u32, modifiers: u32, context: &mut LayoutContext, f: &mut impl GridHost) {
        if !(0x25..=0x28).contains(&key) {
            return;
        }
        let mut cell = self.selection.last().map_or([0, 1], |&(c, r)| [c, r]);
        let axis = usize::from(matches!(key, 0x26 | 0x28));
        let back = key <= 0x26;
        let unlimited = self.flags & (0x8000 << axis) != 0;
        if back {
            if cell[axis] < 1 {
                let _ = self.play_sound(2, f);
                return;
            }
            cell[axis] = cell[axis].wrapping_sub(1);
        } else if unlimited && matches!(self.limits[axis], -1 | 0) {
            cell[axis] = cell[axis].wrapping_add(1);
        } else {
            let bound = if unlimited {
                self.limits[axis]
            } else {
                self.used_range()[axis + 2].wrapping_add(1)
            };
            if cell[axis] >= bound.wrapping_sub(1) {
                if unlimited {
                    let _ = self.play_sound(2, f);
                    return;
                }
            } else {
                cell[axis] = cell[axis].wrapping_add(1);
            }
        }
        if modifiers & 1 == 0 {
            self.selection.clear();
        }
        f.notify(self.primary, 0x9a15_52d2, &cell);
        self.select_live((cell[0], cell[1]), true, context, f);
        if back && cell[axis] < self.position[axis] {
            self.scroll_to(axis, cell[axis]);
        }
        if !back && cell[axis] as f32 >= self.position[axis] as f32 + self.visible[axis] {
            self.scroll_to(axis, (cell[axis] as f32 - self.visible[axis] + 1.0) as i32);
        }
        self.bar_values(f);
        self.dirty = true;
        self.recalculate(context, f);
    }

    pub fn set_editable_text(
        &mut self,
        cell: Coordinate,
        text: Option<&[u8]>,
        count: i32,
        context: &mut LayoutContext,
        f: &mut impl GridHost,
    ) -> Result<bool> {
        if cell.0 != i32::MAX && cell.1 != i32::MAX {
            if let Some(Cell {
                value: Value::Window { handle, .. },
                ..
            }) = self.cells.get(&cell)
            {
                let mut view = None;
                if f.object_query(*handle, super::super::text_edit::IID, &mut view) {
                    if let Some(editor) = view {
                        f.editor_text(editor, text, count);
                        f.release(editor);
                        return Ok(true);
                    }
                }
            }
        }
        self.clear_cell(cell, f);
        let Some(editor) = f.create_editor(0x11ff) else {
            // WG7: spec does not settle the factory failure path.
            return Err(Unsupported::Open("WG7"));
        };
        f.editor_text(editor, text, count);
        let result = self.set_window(cell, Some(editor), true, true, 1, f);
        f.release(editor);
        self.recalculate(context, f);
        Ok(result)
    }

    pub fn set_cell_font(
        &mut self,
        cell: Coordinate,
        font: Option<Handle>,
        context: &mut LayoutContext,
        f: &mut impl GridHost,
    ) -> bool {
        if !self.within(cell) {
            return false;
        }
        if let Some(font) = font {
            f.add_ref(font);
        }
        let format = self
            .cells
            .entry(cell)
            .or_default()
            .format
            .get_or_insert_with(Format::default);
        let old = std::mem::replace(&mut format.font, font);
        if let Some(old) = old {
            f.release(old);
        }
        if font.is_some() || old.is_some() {
            self.recalculate(context, f);
        }
        true
    }
}

/// §5.7's template helpers have no native allocator/layout requirement. Rust
/// containers provide the same insert/erase/lower-bound and stable tie order.
pub fn signed_gcd(mut a: i32, mut b: i32) -> i32 {
    while b != 0 {
        let remainder = a.wrapping_rem(b);
        a = b;
        b = remainder;
    }
    a
}

pub fn equal_record(left: [u32; 3], right: [u32; 3]) -> bool {
    left[1] == right[1] && (left[1] == 0 || left[2] == right[2])
}

pub fn stable_sort_values<T>(values: &mut [T], ascending: bool, mut compare: impl FnMut(&T, &T) -> std::cmp::Ordering) {
    values.sort_by(|a, b| if ascending { compare(a, b) } else { compare(b, a) });
}

pub fn compare_cell_values(
    left: Option<&Cell>,
    right: Option<&Cell>,
    mut text_compare: impl FnMut(&[u8], &[u8]) -> std::cmp::Ordering,
) -> std::cmp::Ordering {
    match (left.map(|c| &c.value), right.map(|c| &c.value)) {
        (Some(Value::Text(a)), Some(Value::Text(b))) => text_compare(a, b),
        _ => std::cmp::Ordering::Equal,
    }
}
