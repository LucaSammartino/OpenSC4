// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4WinRegionView, from region-view.md. Native objects are opaque handles.
//! The caller owns city entries; this owner retains only their portable values.

pub mod canvas;
mod cities;
pub mod image;
mod signposts;
#[cfg(test)]
mod tests;

use crate::regional_city::RegionalCity;
use crate::ui_windows::base::{self, Rect, Window, Windows};
use crate::widget_controls::{Draw, Handle};
pub use canvas::{Canvas, CanvasHost, Surface};
pub use signposts::{Animator, SignpostData};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    Open(&'static str),
    Precondition(&'static str),
}
pub type Result<T> = std::result::Result<T, Unsupported>;
pub type RegionKey = u64;
pub type CityId = u64;
/// No Rc/RefCell in view state: an adapter copies metadata from the city interface.
pub type CityRecord = RegionalCity<(), ()>;

#[derive(Clone, Debug, PartialEq)]
pub struct Icon {
    pub position: (i32, i32),
    pub image: Surface,
}

#[derive(Clone, Debug)]
pub struct CityEntry {
    pub id: CityId,
    pub cell: (i32, i32),
    pub position: (f32, f32),
    pub size_code: u8,
    pub image: Surface,
    pub alternate_image: Option<Surface>,
    pub composed: Option<Surface>,
    pub current: bool,
    pub runs: Vec<u32>,
    pub hit_runs: Vec<u32>,
    pub grid: Vec<image::Coverage>,
    pub selection: Vec<image::Coverage>,
    pub label_anchor: (i32, i32),
    pub effects: Vec<Handle>,
    pub icons: Vec<Icon>,
    pub signpost: Option<Window>,
}

impl CityEntry {
    /// RGV5: only the consumer format is specified; callers must supply built masks.
    pub fn new(id: CityId, cell: (i32, i32), size_code: u8, image: Surface) -> Self {
        Self {
            id,
            cell,
            size_code,
            image,
            position: (0.0, 0.0),
            alternate_image: None,
            composed: None,
            current: false,
            runs: Vec::new(),
            hit_runs: Vec::new(),
            grid: Vec::new(),
            selection: Vec::new(),
            label_anchor: (0, 0),
            effects: Vec::new(),
            icons: Vec::new(),
            signpost: None,
        }
    }
}

/// Window, renderer, resource and service calls only; policy stays in RegionView.
pub trait Host: base::Host + CanvasHost {
    /// Returns a created reference. RegionView releases each successful load once.
    fn load_image(&mut self, key: (u32, u32, u32, u32, u32)) -> Option<Surface>;
    fn register_canvas(&mut self, register: bool);
    fn redraw(&mut self);
    fn child_at_screen(&mut self, parent: Window, point: (i32, i32)) -> bool;
    fn city_record(&mut self, region: RegionKey, cell: (i32, i32)) -> Option<CityRecord>;
    fn hide_tutorial_mayor(&mut self, tutorial: i32) -> bool;
    fn tutorial_tile(&mut self, region: RegionKey) -> Option<(i32, i32)>;
    fn create_effect(&mut self, name: &str) -> Option<Handle>;
    fn start_effect(&mut self, effect: Handle, position: [f32; 3]);
    fn stop_release_effect(&mut self, effect: Handle);
    fn remove_release_signpost(&mut self, parent: Window, signpost: Window);
    fn create_signpost(&mut self, parent: Window, instance: u32, id: u32) -> Option<Window>;
    fn signpost_size(&mut self, signpost: Window) -> (i32, i32);
    fn move_signpost(&mut self, signpost: Window, position: (i32, i32));
    /// Adapter applies these values through existing script-created widget controls.
    fn fill_signpost(&mut self, signpost: Window, data: &SignpostData);
    fn font(&mut self, id: u32) -> Option<Handle>;
    fn font_height(&mut self, font: Handle) -> i32;
    fn layout_text(&mut self, font: Handle, text: &mut Vec<u8>, bounds: Rect, style: u32);
    fn text_width(&mut self, font: Handle, text: &[u8]) -> i32;
    /// Reuse widget_controls drawing vocabulary; do not implement another text widget.
    fn draw_control(&mut self, destination: Surface, operation: Draw);
}

#[derive(Debug)]
pub struct RegionView {
    pub window: Window,
    pub region_key: Option<RegionKey>,
    pub grid_colour: u32,
    pub selection_colour: u32,
    pub scroll: (i32, i32),
    pub selected: Option<CityId>,
    pub signpost_type: (u32, u32),
    pub signposts: (bool, bool),
    pub grid: bool,
    pub labels: (bool, bool),
    pub tutorial_arrow: bool,
    pub scrolling: bool,
    pub map_mode: u32,
    pub cities: Vec<CityEntry>,
    pub canvas: Option<Canvas>,
    background: Option<Surface>,
    icons: [Option<Surface>; 2],
}

impl RegionView {
    pub fn new(window: Window) -> Self {
        Self {
            window,
            region_key: None,
            grid_colour: 0,
            selection_colour: 0,
            scroll: (0, 0),
            selected: None,
            signpost_type: (0, 0),
            signposts: (true, false),
            grid: true,
            labels: (false, false),
            tutorial_arrow: true,
            scrolling: false,
            map_mode: 0,
            cities: Vec::new(),
            canvas: None,
            background: None,
            icons: [None; 2],
        }
    }

    pub fn key(&self) -> Result<RegionKey> {
        // SPEC-GAP: RGV1 leaves the owner's region key unknown; require it explicitly.
        self.region_key.ok_or(Unsupported::Open("RGV1"))
    }

    pub fn init(&mut self, windows: &mut Windows, host: &mut impl Host) -> bool {
        if windows.get(self.window).flag(0x4000) {
            return true;
        }
        windows.init(host, self.window);
        windows.get_mut(self.window).id = 0x2ba6_bb97;
        let mut canvas = Canvas::default();
        let area = windows.get(self.window).area;
        let ok = canvas.init(host, area.width(), area.height(), 256, 256);
        self.canvas = Some(canvas);
        if !ok {
            return false;
        } // The base stays initialised, including on retry.
        host.register_canvas(true);
        windows.set_flag(host, self.window, 0x10000, false);
        windows.set_flag(host, self.window, 0x8000, false);
        self.background = host.load_image((0x856d_dbac, 0x6a1e_ed2c, 0x4a28_05ff, 1, 0));
        for (index, instance) in [0xebab_b1b0, 0xebab_b1b1].into_iter().enumerate() {
            self.icons[index] = host.load_image((0x856d_dbac, 0x46a0_06b0, instance, 1, 0));
        }
        true
    }

    pub fn set_area(&mut self, windows: &mut Windows, host: &mut impl Host, area: Rect) {
        windows.set_area(host, self.window, area);
        if let Some(canvas) = &mut self.canvas {
            let _ = canvas.init(host, area.width(), area.height(), 256, 256);
        }
    }

    pub fn shutdown(&mut self, windows: &mut Windows, host: &mut impl Host) -> bool {
        if !windows.get(self.window).flag(0x4000) {
            return true;
        }
        self.clear_cities(host);
        for icon in &mut self.icons {
            if let Some(image) = icon.take() {
                host.release_surface(image);
            }
        }
        if let Some(mut canvas) = self.canvas.take() {
            host.register_canvas(false);
            canvas.shutdown(host);
        }
        if let Some(image) = self.background.take() {
            host.release_surface(image);
        }
        windows.shutdown(host, self.window)
    }

    pub fn point_transparent(&self, windows: &Windows, host: &mut impl Host, point: (i32, i32)) -> bool {
        let screen = host.window_to_screen(windows, self.window, point);
        !host.child_at_screen(self.window, screen)
    }
}
