// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable cSC4WinMiniMap owner, specified by docs/specs/minimap.md.
//! Native interfaces remain behind traits; no C ABI or plugin integration.

use crate::ui_windows::base::{self, Window, Windows};
use crate::widget_controls;

pub use widget_controls::{Handle, Result, Unsupported};

mod city;
mod palette;
mod numeric;
mod terrain;
mod networks;
mod blit;
pub use blit::{rotate_blt, rotate_blt_dual, visible_recursive, Surface, SurfacePixels};
pub use networks::{Network, SurfaceNetworkFilter};
pub use terrain::{resample_block, Terrain};
pub use palette::{interpolate_ramp, TUNING_KEY};
pub mod tz_map;
pub use tz_map::TzMap;

pub const CLASS_ID: u32 = 0xca31_8388;
pub const INTERFACE_ID: u32 = 0xca31_8385;

/// Foreign primitives reuse the widget controls' reference/drawing operations
/// and the actual cGZWin runtime. No base-window algorithm is reimplemented.
pub trait Host: base::Host + widget_controls::Foreign + Terrain + Network + Surface {
    fn city_cells(&mut self) -> [usize; 2];
    /// Animation manager slots 5/6; the registration argument is always zero.
    fn animation_target(&mut self, window: Window, register: bool);
    fn subscribe(&mut self, window: Window, message: u32, subscribe: bool);
    /// Raw exemplar/property values, not a native palette result.
    fn tuning_ramp(&mut self, key: [u32; 3], property: u32) -> Result<Option<Vec<(u32, u32)>>>;
    fn rotation(&mut self) -> u32;
}

#[derive(Debug)]
pub struct MiniMap {
    window: Window,
    flags: u32,
    n: usize,
    cells: [usize; 2],
    surface: Option<Handle>,
    overlay: Option<Handle>,
    elapsed: f32,
    attached: bool,
    map_dirty: bool,
    surface_dirty: bool,
    repaint: bool,
    dragging: bool,
    hooked: bool,
    blink: bool,
    shift: i32,
    zone_colors: [u32; 16],
    dirty: [u32; 16],
    terrain_palette: [u32; 256],
    water_palette: [u32; 256],
    networks: Vec<u32>,
    map: TzMap,
}

impl MiniMap {
    pub fn new(windows: &mut Windows, host: &mut impl Host) -> Self {
        Self {
            window: windows.create_default(host),
            flags: 7,
            n: 0,
            cells: [0; 2],
            surface: None,
            overlay: None,
            elapsed: 0.0,
            attached: false,
            map_dirty: false,
            surface_dirty: false,
            repaint: false,
            dragging: false,
            hooked: false,
            blink: false,
            shift: 0,
            zone_colors: [0; 16],
            dirty: [0; 16],
            terrain_palette: [0; 256],
            water_palette: [0; 256],
            networks: Vec::new(),
            map: TzMap::default(),
        }
    }

    pub fn as_igzwin(&self) -> Window {
        self.window
    }

    pub fn query_interface(&self, windows: &mut Windows, iid: u32, out: &mut Option<Window>) -> bool {
        if iid == INTERFACE_ID {
            windows.add_ref(self.window);
            *out = Some(self.window);
            true
        } else {
            windows.query_interface(self.window, iid, out)
        }
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn map(&self) -> &TzMap {
        &self.map
    }

    pub fn map_size(&self) -> usize {
        self.n
    }

    pub fn scale_shift(&self) -> i32 {
        self.shift
    }
}

#[cfg(test)]
mod tests;
