// SPDX-License-Identifier: GPL-3.0-or-later
use super::{ControlHost, Pixel, Unsupported};
use crate::view_utilities::PixelRect;

pub const ID: u32 = 0x48E2_A1F8;
pub const NEUTRAL: u32 = 0xC2A6_76AC;
pub const ANCHOR_WINDOW: u32 = 0x48E9_45B4;
pub const TUNING_KEY: (u32, u32, u32) = (0x6534_284A, 0x690F_693F, 0xE90F_6C84);
pub const INNER_PROPERTY: u32 = 0xE90F_5DE4;
pub const OUTER_PROPERTY: u32 = 0xA90F_5DEC;
pub const PICTURE_KEY: (u32, u32, u32) = (0x856D_DBAC, 0x46A0_06B0, 0x094A_C89A);

#[derive(Clone, Copy, Debug, Default)]
pub struct ScrollTunables {
    pub inner: Option<i32>,
    pub outer: Option<i32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollMotion {
    pub on: bool,
    pub angle: f32,
    pub speed: f32,
}

/// Resource handles are opaque tokens, not native tool results. Picture-window
/// flags are set individually so host adapters cannot choose owner policy.
pub trait ScrollHost: ControlHost {
    fn scroll(&mut self, motion: ScrollMotion);
    fn scroll_stop(&mut self);
    fn load_picture(&mut self, key: (u32, u32, u32)) -> Option<u64>;
    fn picture_size(&self, picture: u64) -> (i32, i32);
    fn picture_transparent_color(&mut self, picture: u64, rgb: (u8, u8, u8));
    fn create_picture_window(&mut self, id: u32, picture: u64, rect: PixelRect) -> Option<u64>;
    fn window_flag(&mut self, window: u64, flag: u32, on: bool);
    fn window_show(&mut self, window: u64);
    fn child_add(&mut self, window: u64) -> bool;
    fn window_front(&mut self, window: u64);
    fn window_destroy(&mut self, window: u64);
    fn child_delete_id(&mut self, id: u32);
    fn release_picture(&mut self, picture: u64);
}

#[derive(Debug)]
pub struct CityScroll {
    anchor: Pixel,
    inner: i32,
    outer: i32,
    stored_speed: (i32, i32),
    cursor: u32,
    has_scrolled: bool,
    was_capturing: bool,
    initialized: bool,
    picture: Option<u64>,
}

impl Default for CityScroll {
    fn default() -> Self {
        Self {
            anchor: Pixel { x: 0, y: 0 },
            inner: 12,
            outer: 80,
            stored_speed: (0, 0),
            cursor: NEUTRAL,
            has_scrolled: false,
            was_capturing: false,
            initialized: false,
            picture: None,
        }
    }
}

impl CityScroll {
    pub fn init(&mut self, host: &mut impl ScrollHost, tuning: ScrollTunables) -> Result<(), Unsupported> {
        let inner = tuning.inner.unwrap_or(12);
        let outer = tuning.outer.unwrap_or(80);
        // VIC3: negative/equal/reversed thresholds are not specified. Reject
        // instead of inventing a denominator, wraparound or cursor policy.
        if inner < 0 || outer <= inner {
            return Err(Unsupported::PrimitiveData);
        }
        if !self.initialized {
            self.inner = inner;
            self.outer = outer;
            if host.set_cursor(NEUTRAL) {
                self.cursor = NEUTRAL;
            }
            self.was_capturing = host.am_capturing();
            if !self.was_capturing {
                host.capture();
            }
            self.initialized = true;
        }
        Ok(())
    }
    pub fn is_self_scrolling_view(&self) -> bool {
        true
    }
    pub fn stored_speed(&self) -> (i32, i32) {
        self.stored_speed
    }
    pub fn has_scrolled(&self) -> bool {
        self.has_scrolled
    }
    pub fn cursor(&self) -> u32 {
        self.cursor
    }
    pub fn set_mouse_scroll(&mut self, host: &mut impl ScrollHost, at: Pixel) -> Result<(), Unsupported> {
        self.anchor = at;
        if self.picture.is_some() {
            return Ok(());
        }
        let pic = host.load_picture(PICTURE_KEY).ok_or(Unsupported::ResourceUnavailable)?;
        self.picture = Some(pic);
        host.picture_transparent_color(pic, (255, 0, 255));
        let (w, h) = host.picture_size(pic);
        let x = at.x - w / 2;
        let y = at.y - h / 2;
        let win = host
            .create_picture_window(ANCHOR_WINDOW, pic, PixelRect::new(x, y, x + w, y + h))
            .ok_or(Unsupported::ResourceUnavailable)?;
        host.window_flag(win, 2, false);
        host.window_flag(win, 4, true);
        host.window_show(win);
        if host.child_add(win) {
            host.window_front(win);
        } else {
            host.window_destroy(win);
        }
        Ok(())
    }
    /// `physical_button_down` is the right button, or left when swapped, sampled
    /// by the input adapter. It is not inferred from event modifiers.
    pub fn mouse_move(&mut self, host: &mut impl ScrollHost, at: Pixel, physical_button_down: bool) -> bool {
        if !physical_button_down {
            return self.mouse_up_right(host);
        }
        // Widen subtraction to avoid debug overflow on extreme input pixels.
        let dx = -((i64::from(self.anchor.x) - i64::from(at.x)) / 2);
        let dz = -((i64::from(self.anchor.y) - i64::from(at.y)) / 2);
        let i = i64::from(self.inner);
        let o = i64::from(self.outer);
        let axis = |d: i64| {
            if d.abs() > o {
                (d.signum() * o) as i32
            } else if d.abs() > i {
                d as i32
            } else {
                0
            }
        };
        self.stored_speed = (axis(dx), axis(dz));
        let moving = dx.abs() > i || dz.abs() > i;
        let cursor = if !moving {
            NEUTRAL
        } else {
            let row = if dz <= -i {
                0
            } else if dz >= i {
                2
            } else {
                1
            };
            let col = if dx <= -i {
                0
            } else if dx >= i {
                2
            } else {
                1
            };
            [
                [0x02A6_7691, 0x62A6_7606, 0x82A6_76A8],
                [0x62A6_7694, NEUTRAL, 0xC2A6_76A4],
                [0xA2A6_7698, 0x62A6_769B, 0x22A6_76A0],
            ][row][col]
        };
        if cursor != NEUTRAL {
            self.has_scrolled = true;
        }
        if cursor != self.cursor && host.set_cursor(cursor) {
            self.cursor = cursor;
        }
        let (dx, dz) = (dx as f32, dz as f32);
        let speed = ((dx * dx + dz * dz).sqrt() / (self.outer - self.inner) as f32).min(1.0);
        let angle = if !moving {
            0.0
        } else if dx > 0.0 {
            (dz / dx).atan()
        } else if dx < 0.0 {
            (dz / dx).atan() + std::f32::consts::PI
        } else if dz < 0.0 {
            3.0 * std::f32::consts::FRAC_PI_2
        } else {
            std::f32::consts::FRAC_PI_2
        };
        host.scroll(ScrollMotion {
            on: moving,
            angle,
            speed,
        });
        false
    }
    pub fn end_input(&mut self, host: &mut impl ScrollHost) {
        host.scroll_stop();
        host.child_delete_id(ANCHOR_WINDOW);
        host.remove_top_control(false);
    }
    pub fn mouse_up_right(&mut self, host: &mut impl ScrollHost) -> bool {
        self.end_input(host);
        true
    }
    pub fn shutdown(&mut self, host: &mut impl ScrollHost) {
        if !self.initialized {
            return;
        }
        host.scroll_stop();
        host.child_delete_id(ANCHOR_WINDOW);
        if !self.was_capturing {
            host.release_capture();
        }
        if let Some(pic) = self.picture.take() {
            host.release_picture(pic);
        }
        self.initialized = false;
    }
}
