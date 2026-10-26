// SPDX-License-Identifier: GPL-3.0-or-later
//! cWinBMPViewGVExtra, picture-view.md section 8. The picture view is a foreign
//! dependency; this owner chooses the size, settings, timer and notification.

use crate::ui_windows::base::{Host, Rect, Window, Windows};

use super::Result;

pub const SIZE_CHANGED: u32 = 0x9127_484a;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewOp {
    BitmapFlag { flag: u32, on: bool },
    Fit(bool),
    Centred(u32),
    ZoomKeys(bool),
    Scrolling(bool),
}

/// Primitive calls into the separate cWinBMPView, input and timer services.
/// `elapsed_ms` is None when the timer has not been started or was reset.
pub trait PictureHost {
    fn init_view(&mut self, window: Window) -> Result<()>;
    fn view_op(&mut self, window: Window, op: ViewOp);
    fn view_left_click(&mut self, window: Window, point: [i32; 2]);
    fn view_message(&mut self, window: Window, message: [u32; 4]) -> bool;
    fn scroll_lock(&mut self) -> bool;
    fn elapsed_ms(&mut self, window: Window) -> Option<u32>;
    fn restart_timer(&mut self, window: Window);
    fn reset_timer(&mut self, window: Window);
    fn send_parent(&mut self, parent: Window, message: u32, words: [u32; 3]);
}

#[derive(Clone, Debug)]
pub struct AlbumPicture {
    pub window: Window,
    should_fit: bool,
    large: bool,
    large_area: Rect,
    small_area: Rect,
}

impl AlbumPicture {
    /// The spec gives no initial fit byte; callers supply it explicitly.
    pub fn new(window: Window, should_fit: bool) -> Self {
        Self {
            window,
            should_fit,
            large: true,
            large_area: Rect(0, 0, 16, 16),
            small_area: Rect(0, 0, 32, 32),
        }
    }

    pub fn is_large(&self) -> bool {
        self.large
    }

    pub fn should_fit(&self) -> bool {
        self.should_fit
    }

    /// UI-script geometry is an input, not a second window implementation.
    pub fn set_size_areas(&mut self, large: Rect, small: Rect) {
        self.large_area = large;
        self.small_area = small;
    }

    pub fn init(&mut self, host: &mut impl PictureHost) -> Result<()> {
        host.init_view(self.window)?;
        host.view_op(self.window, ViewOp::BitmapFlag { flag: 2, on: true });
        // Section 8 quirk: constructor is already large; SetSizeInOut(1) has no effects.
        Ok(())
    }

    fn apply_fit(&self, host: &mut impl PictureHost) {
        let fit = self.should_fit && (self.large || !host.scroll_lock());
        host.view_op(self.window, ViewOp::Fit(fit));
    }

    pub fn set_should_fit(&mut self, host: &mut impl PictureHost, fit: bool) {
        self.should_fit = fit;
        self.apply_fit(host);
    }

    pub fn set_size(&mut self, windows: &mut Windows, host: &mut (impl Host + PictureHost), large: bool) -> bool {
        if self.large == large {
            return true;
        }
        self.large = large;
        windows.pull_to_front(self.window);
        self.apply_fit(host);
        host.view_op(self.window, ViewOp::Centred(1));
        host.view_op(self.window, ViewOp::ZoomKeys(false));
        host.view_op(self.window, ViewOp::Scrolling(false));
        windows.set_area(host, self.window, if large { self.large_area } else { self.small_area });
        let base = windows.get(self.window);
        if let Some(parent) = base.parent {
            host.send_parent(parent, SIZE_CHANGED, [base.id, u32::from(large), 0]);
        }
        true
    }

    pub fn left_click(&mut self, windows: &mut Windows, host: &mut (impl Host + PictureHost), point: [i32; 2]) {
        if host.elapsed_ms(self.window).is_some_and(|elapsed| elapsed < 500) {
            self.set_size(windows, host, !self.large);
            host.reset_timer(self.window);
        } else {
            host.restart_timer(self.window);
            host.view_left_click(self.window, point);
        }
    }

    pub fn message(&self, host: &mut impl PictureHost, message: [u32; 4]) -> bool {
        host.view_message(self.window, message)
    }
}

#[cfg(test)]
mod tests;
