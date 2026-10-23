// SPDX-License-Identifier: GPL-3.0-or-later
//! cGZWinListBox, widget-controls §5. Native L slots and W extensions are
//! separate operations; this safe model does not fabricate unresolved ABIs.
use super::*;

pub const IID: u32 = 0x4132_242b;
pub const LIST_OFFSET: u64 = 0xd8;
pub const LIST_SLOTS: usize = 66;
pub const WINDOW_SLOTS: usize = 169;
pub const SELECTION: u32 = 0x0871_0e0f;
pub const ACTIVATION: u32 = 0x0871_0e10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub text: Vec<u8>,
    pub id: u32,
}

#[derive(Debug)]
pub struct ListBox {
    pub window: Handle,
    pub entries: Vec<Entry>,
    pub selected: i32,
    pub first: i32,
    pub whole_rows: i32,
    pub row_height: i32,
    pub minimum_height: u8,
    pub intensity: f32,
    pub selection_color: u32,
    pub font_colors: [u32; 3],
    pub outline_colors: [u32; 2],
    pub font: Option<Handle>,
    pub background: Option<Handle>,
    pub border: Option<Handle>,
    pub services: [Option<Handle>; 2],
    pub bar: Option<Handle>,
    pub timer: Option<Handle>,
    pub flat: bool,
    pub three_d: bool,
    pub drop_mode: bool,
    pub suppression: bool,
    pub drop_resize: [bool; 2],
    pub highlight: bool,
    pub x_gutter: i8,
    pub y_gutter: i8,
    pub bar_gap: i32,
    pub bar_width: i32,
    pub area: Rect,
    pub fill_color: u32,
    pub delivery: Delivery,
    pub previous_click: i32,
    pub pending_double_click: bool,
    pub captured: bool,
    pub initialized: bool,
}

impl ListBox {
    pub fn new(window: Handle, drop_mode: bool, f: &mut impl Foreign) -> Self {
        Self {
            window,
            entries: Vec::new(),
            selected: -1,
            first: 0,
            whole_rows: 0,
            row_height: 10,
            minimum_height: 0,
            intensity: 1.0,
            selection_color: SENTINEL,
            font_colors: [SENTINEL; 3],
            outline_colors: [SENTINEL; 2],
            font: None,
            background: None,
            border: None,
            services: [None; 2],
            bar: None,
            timer: f.create(Resource::Timer),
            flat: false,
            three_d: false,
            drop_mode,
            suppression: false,
            drop_resize: [true, false],
            highlight: true,
            x_gutter: 3,
            y_gutter: 1,
            bar_gap: 3,
            // Window geometry/bar width are supplied by the base window/style.
            bar_width: 0,
            area: [0; 4],
            fill_color: 0,
            delivery: Delivery::Send,
            previous_click: -1,
            pending_double_click: false,
            captured: false,
            initialized: false,
        }
    }

    /// Both native W and L query paths arrive here after view normalization.
    pub fn query_interface(&self, iid: u32, output: Option<&mut Option<Handle>>, f: &mut impl Foreign) -> bool {
        if iid == IID {
            let Some(output) = output else { return false };
            *output = Some(self.window + LIST_OFFSET);
            f.add_ref(self.window);
            true
        } else if let Some(output) = output {
            f.base_query(self.window, iid, output)
        } else {
            false
        }
    }

    pub fn init(&mut self, f: &mut impl Foreign) -> bool {
        if self.initialized {
            return true;
        }
        if self.font.is_none() {
            let font = f.default_interface(0x6896_3c4c);
            replace_ref(&mut self.font, font, f);
        }
        for (slot, key) in self.outline_colors.iter_mut().zip([0xa896_3fad, 0x0896_3faf]) {
            if *slot == SENTINEL {
                if let Some(color) = f.default_color(key) {
                    *slot = color;
                }
            }
        }
        for (slot, key) in self.font_colors.iter_mut().take(2).zip([0x0896_3fb9, 0x2896_3fbc]) {
            if *slot == SENTINEL {
                if let Some(color) = f.default_color(key) {
                    *slot = color;
                }
            }
        }
        if self.selection_color == SENTINEL {
            if let Some(color) = f.default_color(0x8896_3fb4) {
                self.selection_color = color;
            }
        }
        if self.drop_mode {
            self.first = 0;
            self.select(self.selected, false, f);
            f.capture(self.window, true);
            self.captured = true;
            if let Some(border) = f.default_interface(0xe896_3ee5) {
                replace_ref(&mut self.border, Some(border), f);
            }
        }
        self.recompute_metrics(f);
        self.update_bar(f);
        let result = f.base_init(self.window);
        self.initialized = result;
        result
    }

    pub fn shutdown(&mut self, f: &mut impl Foreign) {
        f.show(self.window, false);
        replace_ref(&mut self.border, None, f);
        replace_ref(&mut self.background, None, f);
        replace_ref(&mut self.font, None, f);
        for service in &mut self.services {
            replace_ref(service, None, f);
        }
        f.base_shutdown(self.window);
        self.initialized = false;
    }

    /// Timer/container destruction is distinct from Shutdown's five releases.
    pub fn destroy(&mut self, f: &mut impl Foreign) {
        self.shutdown(f);
        if let Some(timer) = self.timer.take() {
            f.release(timer);
        }
        self.entries.clear();
    }

    pub fn set_font(&mut self, font: Option<Handle>, f: &mut impl Foreign) {
        replace_ref(&mut self.font, font, f);
        self.recompute_metrics(f);
    }
    pub fn set_background(&mut self, buffer: Option<Handle>, f: &mut impl Foreign) {
        replace_ref(&mut self.background, buffer, f);
    }
    pub fn set_border(&mut self, buffer: Option<Handle>, f: &mut impl Foreign) {
        replace_ref(&mut self.border, buffer, f);
    }
    pub fn set_flat(&mut self, value: bool, f: &mut impl Foreign) {
        self.flat = value;
        self.three_d = false;
        f.invalidate(self.window);
    }
    pub fn set_three_d(&mut self, value: bool, f: &mut impl Foreign) {
        self.three_d = value;
        self.flat = false;
        f.invalidate(self.window);
    }
    pub fn set_font_color(&mut self, index: usize, color: u32, f: &mut impl Foreign) -> bool {
        let Some(slot) = self.font_colors.get_mut(index) else {
            return false;
        };
        if *slot == color {
            f.invalidate(self.window);
        }
        *slot = color;
        true
    }
    pub fn font_color(&self, index: usize, output: &mut u32) -> bool {
        match self.font_colors.get(index) {
            Some(&color) if color != SENTINEL => {
                *output = color;
                true
            }
            _ => false,
        }
    }

    pub fn recompute_metrics(&mut self, f: &mut impl Foreign) {
        self.row_height = i32::from(self.minimum_height);
        if let Some(h) = self.font {
            self.row_height = self.row_height.max(f.font_height(h));
        }
        if let Some(h) = self.background {
            self.row_height = self.row_height.max(f.buffer_height(h));
        }
        self.whole_rows = if self.row_height == 0 {
            0
        } else {
            (self.height() - 2 * i32::from(self.y_gutter)) / self.row_height
        };
    }
    pub fn height(&self) -> i32 {
        self.area[3] - self.area[1]
    }
    pub fn width(&self) -> i32 {
        self.area[2] - self.area[0]
    }
    pub fn resize_whole_lines(&mut self, f: &mut impl Foreign) -> bool {
        if self.row_height == 0 {
            return false;
        }
        self.area[3] = self.area[1] + self.whole_rows * self.row_height + 2 * i32::from(self.y_gutter);
        self.update_bar(f);
        f.invalidate(self.window);
        true
    }

    pub fn insert(&mut self, index: i32, text: Vec<u8>, id: u32, f: &mut impl Foreign) {
        let count = self.entries.len() as i32;
        let index = if index == -2 || index > count {
            count
        } else {
            index.max(0)
        };
        self.entries.insert(index as usize, Entry { text, id });
        if count == 0 {
            if self.selected != -1 {
                self.selected = 0;
            }
        } else if index <= self.selected {
            self.selected += 1;
        }
        self.update_bar(f);
        f.invalidate(self.window);
    }
    pub fn replace_text(&mut self, index: i32, text: Vec<u8>, f: &mut impl Foreign) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        let last = self.entries.len() as i32 - 1;
        let index = if index == -2 || index > last {
            last
        } else {
            index.max(0)
        };
        self.entries[index as usize].text = text;
        f.invalidate(self.window);
        true
    }
    /// W extension 158: storage removal, deliberately separate from L slot 10.
    pub fn remove_storage(&mut self, index: usize) -> bool {
        if index >= self.entries.len() {
            return false;
        }
        self.entries.remove(index);
        true
    }
    pub fn insert_storage(&mut self, index: usize, entry: Entry) -> bool {
        if index > self.entries.len() {
            return false;
        }
        self.entries.insert(index, entry);
        true
    }
    pub fn remove(&mut self, index: i32, f: &mut impl Foreign) -> Result<bool> {
        if index < 0 {
            return Err(Unsupported::Open("WC1: negative removal"));
        }
        if !self.remove_storage(index as usize) {
            return Ok(false);
        }
        // Literal §5.3: maintain indices of surviving entries and clamp the
        // removed last entry. Exact deleted-selection choice is asked in WC4.
        if self.selected >= index {
            self.selected -= 1;
        }
        if self.first > index {
            self.first -= 1;
        }
        self.update_bar(f);
        f.invalidate(self.window);
        Ok(true)
    }
    pub fn text(&self, index: usize) -> Option<&[u8]> {
        self.entries.get(index).map(|entry| entry.text.as_slice())
    }
    pub fn find_text(&self, text: &[u8], f: &mut impl Foreign) -> Option<usize> {
        self.entries.iter().position(|entry| f.text_equal(&entry.text, text))
    }
    pub fn select_prefix(&mut self, prefix: &[u8], f: &mut impl Foreign) -> bool {
        let index = self
            .entries
            .iter()
            .position(|entry| f.begins_with(&entry.text, prefix, 1));
        if let Some(index) = index {
            self.select(index as i32, true, f)
        } else {
            false
        }
    }
    pub fn select(&mut self, index: i32, notify: bool, f: &mut impl Foreign) -> bool {
        let index = if index == -1 || self.entries.is_empty() {
            -1
        } else {
            index.clamp(0, self.entries.len() as i32 - 1)
        };
        let changed = self.selected != index;
        self.selected = index;
        if changed && index >= 0 && !self.is_visible(index) {
            self.make_visible(index, f);
        }
        if notify && !self.drop_mode {
            self.emit(SELECTION, f);
        }
        f.invalidate(self.window);
        true
    }
    pub fn select_id(&mut self, id: u32, notify: bool, f: &mut impl Foreign) -> bool {
        let Some(index) = self.entries.iter().position(|entry| entry.id == id) else {
            return false;
        };
        self.select(index as i32, notify, f)
    }
    pub fn selected_id(&self) -> u32 {
        self.entries.get(self.selected as usize).map_or(0, |entry| entry.id)
    }
    pub fn is_visible(&self, row: i32) -> bool {
        row >= self.first && row < self.first + self.whole_rows
    }
    pub fn make_visible(&mut self, row: i32, f: &mut impl Foreign) {
        if row < self.first {
            self.first = row;
        } else if row >= self.first + self.whole_rows {
            self.first = row - self.whole_rows + 1;
        }
        self.refresh_bar(f);
        f.invalidate(self.window);
    }
    pub fn set_first(&mut self, row: i32, f: &mut impl Foreign) {
        self.first = row.max(0);
        let count = self.entries.len() as i32;
        if count > self.whole_rows {
            self.first = self.first.min(count - self.whole_rows);
        }
        self.refresh_bar(f);
        f.invalidate(self.window);
    }
    pub fn previous(&mut self, f: &mut impl Foreign) -> bool {
        if self.selected <= 0 || self.entries.is_empty() {
            return false;
        }
        self.select(self.selected - 1, false, f)
    }
    pub fn next(&mut self, f: &mut impl Foreign) -> bool {
        if self.selected < 0 || self.selected + 1 >= self.entries.len() as i32 {
            return false;
        }
        self.select(self.selected + 1, false, f)
    }
    pub fn previous_page(&mut self, f: &mut impl Foreign) -> bool {
        let row = (self.first - self.whole_rows + 1).max(0);
        self.set_first(row, f);
        self.select(row, false, f)
    }
    pub fn next_page(&mut self, _f: &mut impl Foreign) -> Result<bool> {
        Err(Unsupported::Open("WC1: next-page arithmetic"))
    }
    pub fn hit_test(&self, _x: i32, mut y: i32, clamp: bool) -> Result<i32> {
        if self.entries.is_empty() {
            return Ok(-1);
        }
        if self.row_height == 0 {
            return Err(Unsupported::Precondition("nonzero row height"));
        }
        if clamp {
            if self.height() <= 0 {
                return Err(Unsupported::Precondition("positive height"));
            }
            y = y.clamp(0, self.height() - 1);
        }
        let row = (y - i32::from(self.y_gutter)) / self.row_height + self.first;
        let count = self.entries.len() as i32;
        Ok(if row >= count {
            if clamp {
                count - 1
            } else {
                -1
            }
        } else {
            row
        })
    }
    pub fn text_area(&self) -> Rect {
        let x = i32::from(self.x_gutter);
        let y = i32::from(self.y_gutter);
        [
            x,
            y,
            self.width()
                - x
                - if self.bar.is_some() {
                    self.bar_width + self.bar_gap
                } else {
                    0
                },
            self.height() - y,
        ]
    }
    fn refresh_bar(&self, f: &mut impl Foreign) {
        if let Some(bar) = self.bar {
            f.bar_range(
                bar,
                (self.entries.len() as i32 - self.whole_rows).max(0),
                self.whole_rows,
                self.first,
            );
        }
    }
    fn update_bar(&mut self, f: &mut impl Foreign) {
        if self.entries.len() as i32 > self.whole_rows {
            if self.bar.is_none() {
                self.bar = f.create_bar(self.window);
            }
        } else if let Some(bar) = self.bar.take() {
            f.destroy_bar(bar);
        }
        if let Some(bar) = self.bar {
            f.place_bar(bar, [self.width() - self.bar_width, 0, self.width(), self.height()]);
        }
        self.refresh_bar(f);
    }

    pub fn emit(&self, id: u32, f: &mut impl Foreign) -> bool {
        let row = f.convert_payload(self.window, self.selected);
        f.deliver(
            self.window,
            self.delivery,
            Message {
                kind: 3,
                words: [id as i32, row, 0, 0],
            },
        );
        true
    }
    pub fn complete_drop(&self, row: i32, f: &mut impl Foreign) -> bool {
        if self.drop_mode {
            f.drop_complete(self.window, 0xf, row);
        }
        self.drop_mode
    }
    pub fn key_up(&self, key: u32, modifiers: u32, f: &mut impl Foreign) -> bool {
        self.drop_mode
            || f.deliver(
                self.window,
                Delivery::Send,
                Message {
                    kind: 6,
                    words: [key as i32, modifiers as i32, 0, 0],
                },
            )
    }
    pub fn key_down(&mut self, key: u32, modifiers: u32, f: &mut impl Foreign) -> Result<bool> {
        let old = self.selected;
        match key {
            0x21 => {
                self.previous_page(f);
            }
            0x22 => {
                self.next_page(f)?;
            }
            0x26 => {
                self.previous(f);
            }
            0x28 => {
                self.next(f);
            }
            0x0d => {
                self.select(self.selected, true, f);
                if self.drop_mode {
                    self.complete_drop(self.selected, f);
                } else {
                    self.emit(ACTIVATION, f);
                }
                return Ok(true);
            }
            0x1b if self.drop_mode => {
                self.complete_drop(-1, f);
                return Ok(true);
            }
            _ => {
                return Ok(self.drop_mode
                    || f.deliver(
                        self.window,
                        Delivery::Send,
                        Message {
                            kind: 5,
                            words: [key as i32, modifiers as i32, 0, 0],
                        },
                    ))
            }
        }
        if old != self.selected {
            self.select(self.selected, true, f);
        }
        Ok(true)
    }
    /// W extension 152 returns false even when prefix navigation selected a row.
    pub fn character(&mut self, prefix: &[u8], f: &mut impl Foreign) -> bool {
        if !self.captured {
            self.select_prefix(prefix, f);
        }
        false
    }
    pub fn mouse_down(&mut self, x: i32, y: i32, f: &mut impl Foreign) -> Result<bool> {
        let row = self.hit_test(x, y, true)?;
        if !self.captured {
            f.capture(self.window, true);
            self.captured = true;
            if row >= 0 && row != self.selected {
                self.select(row, true, f);
            }
        }
        if let Some(timer) = self.timer {
            self.pending_double_click =
                row == self.previous_click && f.timer_elapsed(timer) < f.double_click_interval();
            f.restart_timer(timer);
        }
        self.previous_click = row;
        Ok(true)
    }
    pub fn mouse_up(&mut self, f: &mut impl Foreign) -> Result<bool> {
        if !self.captured {
            return Ok(true);
        }
        if self.drop_mode {
            return Err(Unsupported::Open("WC1: drop bar capture/up"));
        }
        f.capture(self.window, false);
        self.captured = false;
        if self.pending_double_click {
            self.emit(ACTIVATION, f);
        }
        self.previous_click = -1;
        self.pending_double_click = false;
        Ok(true)
    }
    pub fn mouse_move(&mut self, x: i32, y: i32, over_bar: bool, f: &mut impl Foreign) -> Result<bool> {
        if self.drop_mode {
            if over_bar {
                return Err(Unsupported::Open("WC1: drop bar capture/move"));
            }
            let row = self.hit_test(x, y, true)?;
            if row != self.selected {
                self.select(row, false, f);
            }
        }
        Ok(true)
    }
    pub fn paint(&mut self, f: &mut impl Foreign) -> Result<()> {
        if self.font.is_some() {
            return Err(Unsupported::Open("WC1/WC5: escape recognition and ellipsis"));
        }
        self.paint_plain_text(f)
    }

    /// Established drawing operations on the plain-text, already-fitting
    /// subdomain. This is not a substitute for WC1's unproved text preprocessing.
    pub fn paint_plain_text(&mut self, f: &mut impl Foreign) -> Result<()> {
        if let Some(bar) = self.bar {
            self.first = f.bar_value(bar);
        }
        let rect = [
            0,
            0,
            self.width()
                - if self.bar.is_some() {
                    self.bar_width + self.bar_gap
                } else {
                    0
                },
            self.height(),
        ];
        if let Some(source) = self.background {
            f.draw(Draw::Blit { source, rect });
        } else {
            f.draw(Draw::Fill {
                rect,
                color: self.fill_color,
            });
        }
        if self.drop_mode {
            if let Some(source) = self.border {
                f.draw(Draw::Edge { source, rect });
            }
        }
        let Some(font) = self.font else { return Ok(()) };
        let area = self.text_area();
        let mut y = area[1];
        if self.row_height <= 0 {
            return Err(Unsupported::Precondition("positive paint row height"));
        }
        for row in self.first.max(0) as usize..self.entries.len() {
            if y >= area[3] {
                break;
            }
            let line = [area[0], y, area[2], (y + self.row_height).min(area[3])];
            let selected = row as i32 == self.selected;
            if selected && self.highlight && self.intensity != 0.0 {
                if self.intensity != 1.0 {
                    return Err(Unsupported::Open("WC1: fractional intensity"));
                }
                f.draw(Draw::Fill {
                    rect: line,
                    color: self.selection_color,
                });
            }
            let text = &self.entries[row].text;
            f.draw(Draw::Text {
                font,
                text: text.clone(),
                rect: line,
                color: self.font_colors[usize::from(selected)],
            });
            y += self.row_height;
        }
        if self.flat || self.three_d {
            f.draw(Draw::Outline {
                rect,
                colors: self.outline_colors,
                three_d: self.three_d,
            });
        }
        Ok(())
    }
}
