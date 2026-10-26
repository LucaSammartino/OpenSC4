// SPDX-License-Identifier: GPL-3.0-or-later
//! Complete AU43 preferences owner. Foreign UI, resource and file operations are host inputs.
use super::slider_volume;
pub const BASE: [u32; 2] = [0x2A5B_A8A7, 0x3A5B_A8A7];
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub effects_on: u8,
    pub effects: u32,
    pub music_on: u32,
    pub music: u32,
    pub songs: [[u8; 30]; 2],
    pub mode: u8,
}
impl Default for Record {
    fn default() -> Self {
        Self {
            effects_on: 1,
            effects: 1024,
            music_on: 1,
            music: 800,
            songs: [[1; 30]; 2],
            mode: 0,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Song {
    pub id: u32,
    pub file: u32,
    pub title: u32,
    pub default: u8,
}
#[derive(Default, Debug)]
pub struct Process {
    pub loaded: bool,
    pub open: bool,
    pub lists: [Vec<Song>; 2],
    pub folders: [String; 2],
    pub excluded: [Vec<String>; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub enum Ui {
    Notify(u32),
    Enable(u32, bool),
    Check(u32, bool),
    Slider(u32, u32),
    GridSetup,
    Song {
        row: usize,
        id: u32,
        title: String,
        checked: bool,
        enabled: bool,
    },
    Attach,
    Destroy(u64),
    EndModal(u64),
    ClearSelection,
}
pub trait Host {
    fn retain(&mut self, p: u64);
    fn release(&mut self, p: u64);
    fn song_table(&mut self, k: usize) -> Option<Vec<Vec<u32>>>;
    fn plugin_directory(&mut self) -> String;
    fn text(&mut self, id: u32) -> String;
    fn files(&mut self, folder: &str, pattern: &str) -> Vec<String>;
    fn exists(&mut self, path: &str) -> bool;
    fn tuning(&mut self) -> Option<[(u32, u32); 2]>;
    fn create_window(&mut self) -> Option<u64>;
    fn grid(&mut self, root: u64) -> Option<u64>;
    fn window_view(&mut self, holder: u64, grid: bool) -> u64;
    fn window_manager(&mut self, root: u64) -> Option<u64>;
    fn is_modal(&mut self, root: u64) -> bool;
    fn ui(&mut self, window: u64, op: Ui);
    fn checkbox(&mut self, id: u32) -> Option<bool>;
    fn slider(&mut self, id: u32) -> Option<u32>;
    fn selected_row(&mut self, grid: u64) -> Option<i32>;
    fn child_enabled(&mut self, root: u64, id: u32) -> bool;
    fn click_sound(&mut self);
    fn post(&mut self, kind: u32, a: u32, b: u32);
    /// Pointer2 borrows this process-static list, not a refcounted payload.
    fn post_list(&mut self, k: usize, station: u32, list: &[String], mode: u8);
    fn master_volume(&mut self, value: u32);
    fn application(&mut self) -> Option<u64>;
    fn save_preferences(&mut self, app: u64);
    fn radio(&mut self) -> Option<u64>;
    fn current_path(&mut self, radio: u64) -> Option<String>;
    /// Creates and transfers one counted string to the radio message.
    fn post_preview_path(&mut self, path: &str);
    fn modal(&mut self, root: u64);
}
impl Process {
    pub fn load(&mut self, h: &mut impl Host) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        self.open = false;
        let dir = h.plugin_directory();
        self.folders = [
            format!("{dir}Radio\\Stations\\Region\\Music\\"),
            format!("{dir}Radio\\Stations\\Mayor\\Music\\"),
        ];
        for (k, base) in BASE.into_iter().enumerate() {
            if let Some(properties) = h.song_table(k) {
                let mut enabled = 1;
                for (i, p) in properties.iter().take(30).enumerate() {
                    if p.len() < 2 {
                        continue;
                    }
                    if p.len() > 2 {
                        enabled = p[2] as u8;
                    }
                    self.lists[k].push(Song {
                        id: base + i as u32,
                        title: p[0],
                        file: p[1],
                        default: enabled,
                    });
                }
            }
        }
    }
    pub fn defaults(&self, r: &mut Record) {
        for (flags, list) in r.songs.iter_mut().zip(&self.lists) {
            for (f, s) in flags.iter_mut().zip(list) {
                *f = s.default;
            }
        }
    }
    pub fn custom(&self, k: usize, h: &mut impl Host) -> bool {
        if k >= 2 {
            return false;
        }
        let known: Vec<_> = self.lists[k].iter().map(|s| h.text(s.file).to_lowercase()).collect();
        for pattern in ["*.mp3", "*.wav", "*.xa", "*.m3u", "*.pls", "*.lnk"] {
            if h.files(&self.folders[k], pattern)
                .iter()
                .any(|f| !known.contains(&f.to_lowercase()))
            {
                return true;
            }
        }
        false
    }
    pub fn enabled(&self, path: &str, r: &Record, h: &mut impl Host) -> bool {
        for k in 0..2 {
            let mut found = None;
            for (j, s) in self.lists[k].iter().enumerate() {
                if path.eq_ignore_ascii_case(&format!("{}{}", self.folders[k], h.text(s.file))) {
                    found = Some(r.songs[k][j] != 0);
                }
            }
            if let Some(v) = found {
                return v;
            }
        }
        true
    }
    pub fn rebuild(&mut self, k: usize, r: &Record, h: &mut impl Host) {
        self.load(h);
        if k >= 2 {
            return;
        }
        self.excluded[k] = self.lists[k]
            .iter()
            .zip(r.songs[k])
            .filter(|(_, flag)| *flag != r.mode)
            .map(|(s, _)| h.text(s.file))
            .collect();
        h.post_list(k, u32::from(k == 0), &self.excluded[k], r.mode);
    }
    fn row(&self, row: i32) -> Option<(usize, usize)> {
        if row < 0 {
            return None;
        }
        let row = row as usize;
        if row < self.lists[0].len() {
            Some((0, row))
        } else {
            let j = row - self.lists[0].len();
            (j < self.lists[1].len()).then_some((1, j))
        }
    }
}
#[derive(Debug, Default)]
pub struct Owner {
    pub refcount: u32,
    pub window: Option<u64>,
    pub grid: Option<u64>,
    pub preferences: bool,
    pub preview: bool,
    pub saved: Record,
    pub bounds: [(u32, u32); 2],
}
impl Owner {
    pub fn construct() -> Self {
        Self {
            bounds: [(0, 1024); 2],
            ..Self::default()
        }
    }
    pub fn query_interface(&mut self, iid: u32) -> bool {
        if !matches!(iid, 1 | 0x22E8_5D8E) {
            return false;
        }
        self.refcount = self.refcount.wrapping_add(1);
        true
    }
    pub fn shutdown(&mut self, h: &mut impl Host) -> bool {
        self.preferences = false;
        if let Some(p) = self.grid.take() {
            h.release(p);
        }
        true
    }
    pub fn release(&mut self, h: &mut impl Host) -> u32 {
        if self.refcount > 1 {
            self.refcount -= 1;
            return self.refcount;
        }
        self.shutdown(h);
        if let Some(p) = self.window.take() {
            h.release(p);
        }
        self.refcount = 0;
        0
    }
    pub fn show(&mut self, p: &mut Process, r: &Record, h: &mut impl Host) -> bool {
        p.load(h);
        if p.open {
            return true;
        }
        if let Some(bounds) = h.tuning() {
            self.bounds = bounds;
        }
        if let Some(old) = self.window.take() {
            h.release(old);
        }
        self.window = h.create_window();
        let Some(root) = self.window else { return false };
        self.preferences = true;
        p.open = true;
        self.saved = r.clone();
        h.ui(root, Ui::Enable(0x8A55_0C57, false));
        let custom = p.custom(1, h) || p.custom(0, h);
        h.ui(root, Ui::Enable(0x8A55_0C5A, custom));
        for (id, on) in [
            (0x8A55_0C5A, r.mode != 0),
            (0x8A55_0C52, r.effects_on != 0),
            (0x8A55_0C50, r.music_on != 0),
        ] {
            h.ui(root, Ui::Check(id, on));
        }
        for (id, bank) in [(0x8A55_0C53, 0), (0x8A55_0C51, 1)] {
            let v = if bank == 0 {
                &mut self.saved.effects
            } else {
                &mut self.saved.music
            };
            let pos = volume_slider(v, self.bounds[bank]);
            h.ui(root, Ui::Slider(id, pos));
        }
        for id in [
            0x8A55_0C57,
            0x8A55_0C58,
            0x8A55_0C5A,
            0x8A55_0C52,
            0x8A55_0C50,
            0x8A55_0C51,
            0x8A55_0C53,
        ] {
            h.ui(root, Ui::Notify(id));
        }
        if let Some(old) = self.grid.take() {
            h.release(old);
        }
        self.grid = h.grid(root);
        if let Some(grid) = self.grid {
            h.ui(grid, Ui::GridSetup);
            let mut row = 0;
            for (k, base) in BASE.into_iter().enumerate() {
                for (j, s) in p.lists[k].iter().take(30).enumerate() {
                    let path = format!("{}{}", p.folders[k], h.text(s.file));
                    let enabled = h.exists(&path);
                    let title = h.text(s.title);
                    h.ui(
                        grid,
                        Ui::Song {
                            row,
                            id: base + j as u32,
                            title,
                            checked: r.songs[k][j] != 0,
                            enabled,
                        },
                    );
                    row += 1;
                }
            }
            h.ui(grid, Ui::ClearSelection);
            h.ui(grid, Ui::Notify(0x8A55_0C56));
        }
        h.ui(root, Ui::Attach);
        true
    }
    pub fn update(r: &Record, h: &mut impl Host) {
        h.master_volume(if r.effects_on != 0 { r.effects } else { 0 });
        h.post(0xAA65_0860, r.music_on, r.music);
    }
    pub fn end(&mut self, p: &mut Process, h: &mut impl Host) {
        self.refcount = self.refcount.wrapping_add(1);
        if self.preview {
            h.post(0xAA65_0861, 0, 0);
        }
        if let Some(root) = self.window {
            if let Some(manager) = h.window_manager(root) {
                if h.is_modal(root) {
                    h.ui(root, Ui::EndModal(manager));
                }
                h.ui(root, Ui::Destroy(manager));
            }
            self.window = None;
            h.release(root);
        }
        p.open = false;
        self.release(h);
    }
    pub fn do_modal(&mut self, p: &mut Process, h: &mut impl Host) -> bool {
        let root = self.window.expect("unchecked modal root");
        h.retain(root);
        h.modal(root);
        self.end(p, h);
        h.release(root);
        true
    }
    fn preview_start(&mut self, h: &mut impl Host) {
        if !self.preview {
            self.preview = true;
            h.post(0xAA65_0861, 1, 0);
        }
    }
    fn selected(&self, p: &Process, h: &mut impl Host) -> Option<(usize, usize)> {
        p.row(h.selected_row(self.grid?)?)
    }
    pub fn control(&mut self, id: u32, value: u32, p: &mut Process, r: &mut Record, h: &mut impl Host) -> bool {
        match id {
            0x8A55_0C50 => {
                r.music_on = value;
                Self::update(r, h);
                p.rebuild(0, r, h);
                p.rebuild(1, r, h);
            }
            0x8A55_0C52 => {
                r.effects_on = value as u8;
                Self::update(r, h);
            }
            0x8A55_0C51 | 0x8A55_0C53 => {
                if let Some(pos) = h.slider(id) {
                    let bank = usize::from(id == 0x8A55_0C51);
                    let v = slider_volume(pos, bank as u32, self.bounds);
                    let v = if v > self.bounds[bank].0 { v } else { 0 };
                    if bank == 0 {
                        r.effects = v;
                    } else {
                        r.music = v;
                    }
                    Self::update(r, h);
                }
            }
            0x8A55_0C5A => r.mode = value as u8,
            0x8A55_0C56 => {
                if let Some(root) = self.window {
                    h.ui(root, Ui::Enable(0x8A55_0C57, false));
                    h.click_sound();
                    if let Some((k, j)) = self.selected(p, h) {
                        if h.child_enabled(root, BASE[k] + j as u32) {
                            h.ui(root, Ui::Enable(0x8A55_0C57, true));
                        }
                    }
                }
            }
            0x8A55_0C57 => {
                self.preview_start(h);
                if let Some((k, j)) = self.selected(p, h) {
                    let path = format!("{}{}", p.folders[k], h.text(p.lists[k][j].file));
                    h.post_preview_path(&path);
                }
            }
            0x8A55_0C58 => {
                self.preview_start(h);
                h.post(0x2A61_1B51, 0, 0);
            }
            0xEA55_09E3 => {
                *r = self.saved.clone();
                Self::update(r, h);
                h.post(0x231B_BF91, 0, 0);
                self.end(p, h);
            }
            0xEA55_09E4 => {
                if let Some(app) = h.application() {
                    h.retain(app);
                    h.save_preferences(app);
                    h.release(app);
                }
                p.rebuild(0, r, h);
                p.rebuild(1, r, h);
                let radio = h.radio();
                if let Some(radio) = radio {
                    if let Some(path) = h.current_path(radio) {
                        if !p.enabled(&path, r, h) {
                            h.post(0x2A61_1B51, 0, 0);
                        }
                    }
                }
                h.post(0x231B_BF91, 0, 0);
                self.end(p, h);
                if let Some(radio) = radio {
                    h.release(radio);
                }
            }
            _ => {
                for (k, base) in BASE.into_iter().enumerate() {
                    let j = id.wrapping_sub(base);
                    if j <= 30 {
                        if j < 30 {
                            r.songs[k][j as usize] = value as u8;
                        } else if k == 0 {
                            r.mode = value as u8;
                        } else {
                            r.songs[0][0] = value as u8;
                        }
                    }
                }
            }
        }
        true
    }
    pub fn message(
        &mut self,
        source: u64,
        visible: bool,
        m: [u32; 4],
        p: &mut Process,
        r: &mut Record,
        h: &mut impl Host,
    ) -> bool {
        if !visible {
            return false;
        }
        let root = self.window.map(|w| h.window_view(w, false));
        if Some(source) != root && self.grid.map(|w| h.window_view(w, true)) != Some(source) {
            return false;
        }
        let [kind, notification, id, value] = m;
        match kind {
            3 => {
                let special = [0x8A55_0C57, 0x8A55_0C58, 0xEA55_09E3, 0xEA55_09E4];
                let dispatch = match notification {
                    0x8871_13A3 => matches!(id, 0x8A55_0C51 | 0x8A55_0C53),
                    0xE871_1255 => true,
                    0x2872_59F6 => special.contains(&id),
                    0x2872_59F7 => id != 0x8A55_0C52 && !special.contains(&id),
                    _ => false,
                };
                if dispatch {
                    self.control(id, value, p, r, h);
                }
                if notification == 0x2872_59F8 && id == 0x8A55_0C52 && value == 0 {
                    if let Some(on) = h.checkbox(id) {
                        self.control(id, u32::from(on), p, r, h);
                    }
                }
                true
            }
            5 if notification == 27 => {
                self.control(0xEA55_09E3, 0, p, r, h);
                false
            }
            0x9A15_52D2 => {
                self.control(0x8A55_0C56, value, p, r, h);
                false
            }
            _ => false,
        }
    }
}
pub fn volume_slider(saved: &mut u32, (lo, hi): (u32, u32)) -> u32 {
    let v = if *saved > hi {
        hi
    } else if *saved < lo {
        *saved = 0;
        lo
    } else {
        *saved
    };
    if v <= lo {
        0
    } else if v >= 1000 {
        1000
    } else {
        crate::x87::ftol32(f64::from(v.wrapping_sub(lo)) / f64::from(hi.wrapping_sub(lo)) * 1000.0) as u32
    }
}
#[cfg(test)]
mod tests;
