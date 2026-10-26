// SPDX-License-Identifier: GPL-3.0-or-later
//! Construction state (§1.3). Names for unknown fields retain their offsets.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn nonempty(self) -> bool {
        self.left < self.right && self.top < self.bottom
    }
}

#[derive(Debug, PartialEq)]
pub struct State {
    pub background_enabled: bool,
    pub background_tiled: bool,
    pub transparent: bool,
    pub supports_partial_copy: bool,
    pub allows_partial_copy: bool,
    pub backing_saved: bool,
    pub viewport_size: [i32; 2],
    pub viewport: Rect,
    pub subpixel_bias: [f32; 2],
    pub camera_derived: i32,
    pub camera_view: i32,
    pub camera_mode: i32,
    pub lighting: bool,
    pub fog: bool,
    pub fog_values: [f32; 5],
    pub fog_distances: [f32; 2],
    pub city_extent: [f32; 2],
    pub unknown_1a4: u32,
    pub clear_color: [f32; 4],
    pub clear_color_pushed: bool,
    pub clear_depth: f32,
    pub dirty_area_limit: i32,
    pub unknown_1f4: i32,
    pub cleanup_allowed: bool,
    pub cleanup_band_width: i32,
    pub cleanup_threshold: u32,
    pub map_scale: i32,
    pub full_redraw: bool,
    pub draw_reason: i32,
    pub frustum_valid: bool,
    pub visible_valid: bool,
    pub borrowed: bool,
    pub shake_offsets: [i32; 2],
}

impl Default for State {
    fn default() -> Self {
        Self {
            background_enabled: true,
            background_tiled: true,
            transparent: false,
            supports_partial_copy: true,
            allows_partial_copy: true,
            backing_saved: false,
            viewport_size: [0; 2],
            viewport: Rect::default(),
            subpixel_bias: [0.0; 2],
            camera_derived: -1,
            camera_view: -1,
            camera_mode: 0,
            lighting: true,
            fog: false,
            fog_values: [1.0; 5],
            fog_distances: [10000.0, 15000.0],
            city_extent: [4096.0; 2],
            unknown_1a4: 0x5000,
            clear_color: [0.0; 4],
            clear_color_pushed: false,
            clear_depth: 1.0,
            dirty_area_limit: i32::MAX,
            unknown_1f4: i32::MAX,
            cleanup_allowed: true,
            cleanup_band_width: 64,
            cleanup_threshold: 10000,
            map_scale: 1,
            full_redraw: false,
            draw_reason: 0,
            frustum_valid: false,
            visible_valid: false,
            borrowed: false,
            shake_offsets: [0; 2],
        }
    }
}
