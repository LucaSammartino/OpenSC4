// SPDX-License-Identifier: GPL-3.0-or-later
use super::{CityContext, Error, FileSystem, Gap, ImageDepth, ImagePrimitives, Result, SnapshotAlbum, UiPrimitives, VPath};

pub const CAPTURE_MESSAGE: u32 = 0x1a5d_056d;
pub const CAPTURE_LUA_EVENT: u32 = 0xaa8b_82a9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size { pub width: u32, pub height: u32 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect { pub x: i32, pub y: i32, pub width: u32, pub height: u32 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureTarget { WholeView, Rectangle(Rect) }

pub fn capture_target(rect: Rect, view: Size) -> CaptureTarget {
    // SN6: offsets are relative to the 3D view; the literal "at most 5" is inclusive.
    if rect.width == view.width && rect.height == view.height && rect.x.unsigned_abs() <= 5 && rect.y.unsigned_abs() <= 5 {
        CaptureTarget::WholeView
    } else { CaptureTarget::Rectangle(rect) }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers { pub shift: bool, pub ctrl: bool, pub alt: bool }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key { Space, Escape, Digit(u8), Enter, Other }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyResult {
    Ignored,
    Updated,
    Closed,
    PostCapture { message: u32, counter: u8, close_afterwards: bool, lua_event: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureDelivery {
    Repost { message: u32, counter: u8 },
    Ready { target: CaptureTarget, close_afterwards: bool },
}

#[derive(Clone, Copy, Debug)]
enum Phase {
    Idle,
    Waiting { counter: u8, close: bool, target: CaptureTarget },
    Modal { close: bool, target: CaptureTarget },
}

/// Owns the frame's sizes/index/rectangle, percentage, drag anchor, pending
/// capture count and close/restore decision. No native snapshot callback exists.
#[derive(Clone, Debug)]
pub struct SnapshotTaker {
    sizes: Vec<Size>,
    size_index: usize,
    view: Size,
    rect: Rect,
    percentage: Option<u8>,
    visible: bool,
    closed: bool,
    drag_anchor: Option<(i32, i32)>,
    phase: Phase,
    capture_written: bool,
}

impl SnapshotTaker {
    pub const WINDOW_ID: u32 = 0x8520_2c0e;
    pub const FONT: &'static str = "Arial MT";
    pub const HELP_STRINGS: [u32; 3] = [0x2a56_de25, 0xca56_de46, 0x0a56_de50];

    /// SN6: sizes come from caller configuration, never fabricated native defaults.
    /// Percentage has no invented default; it becomes known after a digit key.
    pub fn new(sizes: Vec<Size>, view: Size) -> Result<Self> {
        if !valid_size(view) || sizes.is_empty() || sizes.iter().any(|s| !valid_size(*s)) { return Err(Error::InvalidSize); }
        if sizes.iter().any(|s| s.width > view.width || s.height > view.height) { return Err(Error::Gap(Gap::OversizedFrame)); }
        let size = sizes[0];
        Ok(Self { sizes, size_index: 0, view,
            rect: Rect { x: 0, y: 0, width: size.width, height: size.height },
            percentage: None, visible: true, closed: false, drag_anchor: None,
            phase: Phase::Idle, capture_written: false })
    }

    pub fn create_or_show(existing: &mut Option<Self>, sizes: Vec<Size>, view: Size, index: Option<isize>) -> Result<&mut Self> {
        if existing.as_ref().map_or(true, |t| t.closed) { *existing = Some(Self::new(sizes, view)?); }
        let frame = existing.as_mut().ok_or(Error::InvalidState)?;
        if !matches!(frame.phase, Phase::Idle) { return Err(Error::InvalidState); }
        frame.set_view(view)?;
        if let Some(index) = index { frame.go_to_size_index(index)?; }
        frame.visible = true;
        Ok(frame)
    }

    pub fn size_index(&self) -> usize { self.size_index }
    pub fn sizes(&self) -> &[Size] { &self.sizes }
    pub fn rectangle(&self) -> Rect { self.rect }
    /// No published initial native percentage; None until a digit sets it (SN1/SN6).
    pub fn percentage(&self) -> Option<u8> { self.percentage }
    pub fn visible(&self) -> bool { self.visible }
    pub fn closed(&self) -> bool { self.closed }

    pub fn set_view(&mut self, view: Size) -> Result<()> {
        if !matches!(self.phase, Phase::Idle) { return Err(Error::InvalidState); }
        if !valid_size(view) { return Err(Error::InvalidSize); }
        if self.sizes.iter().any(|s| s.width > view.width || s.height > view.height) { return Err(Error::Gap(Gap::OversizedFrame)); }
        self.view = view;
        self.adjust_position();
        Ok(())
    }

    pub fn go_to_size_index(&mut self, index: isize) -> Result<()> {
        if !matches!(self.phase, Phase::Idle) || self.closed { return Err(Error::InvalidState); }
        self.size_index = (index.max(0) as usize).min(self.sizes.len() - 1);
        let size = self.sizes[self.size_index];
        self.rect.width = size.width;
        self.rect.height = size.height;
        self.adjust_position();
        Ok(())
    }

    fn adjust_position(&mut self) {
        self.rect.x = self.rect.x.clamp(0, (self.view.width - self.rect.width) as i32);
        self.rect.y = self.rect.y.clamp(0, (self.view.height - self.rect.height) as i32);
    }

    pub fn set_position(&mut self, x: i32, y: i32) -> Result<()> {
        if !matches!(self.phase, Phase::Idle) || self.closed { return Err(Error::InvalidState); }
        self.rect.x = x;
        self.rect.y = y;
        self.adjust_position();
        Ok(())
    }

    pub fn key(&mut self, key: Key, modifiers: Modifiers, frame_slot71_passes: bool) -> Result<KeyResult> {
        // SN6/SN7: input while hidden/pending is ignored; slot 71 is explicit input,
        // never inferred from an undocumented toolkit method.
        if self.closed || !self.visible || !matches!(self.phase, Phase::Idle) { return Ok(KeyResult::Ignored); }
        match key {
            Key::Space => {
                self.go_to_size_index(((self.size_index + 1) % self.sizes.len()) as isize)?;
                Ok(KeyResult::Updated)
            }
            Key::Escape => { self.close(); Ok(KeyResult::Closed) }
            Key::Digit(digit) if digit <= 9 => { self.percentage = Some((digit + 1) * 10); Ok(KeyResult::Updated) }
            Key::Digit(_) => Err(Error::InvalidIndex),
            Key::Enter if frame_slot71_passes => {
                let close = !(modifiers.shift || modifiers.ctrl || modifiers.alt);
                self.phase = Phase::Waiting { counter: 0, close, target: capture_target(self.rect, self.view) };
                self.visible = false;
                self.drag_anchor = None;
                self.capture_written = false;
                Ok(KeyResult::PostCapture { message: CAPTURE_MESSAGE, counter: 0, close_afterwards: close, lua_event: CAPTURE_LUA_EVENT })
            }
            _ => Ok(KeyResult::Ignored),
        }
    }

    pub fn left_mouse_down(&mut self, x: i32, y: i32) {
        if self.visible && !self.closed && matches!(self.phase, Phase::Idle) {
            // SN6: preserve the pointer-to-frame offset instead of jumping to the pointer.
            self.drag_anchor = Some((x.saturating_sub(self.rect.x), y.saturating_sub(self.rect.y)));
        }
    }

    pub fn mouse_move(&mut self, x: i32, y: i32) -> Result<()> {
        if let Some((dx, dy)) = self.drag_anchor { self.set_position(x.saturating_sub(dx), y.saturating_sub(dy))?; }
        Ok(())
    }

    pub fn end_drag(&mut self) { self.drag_anchor = None; }
    pub fn right_mouse_down(&mut self) { self.close(); }

    fn close(&mut self) {
        self.visible = false;
        self.closed = true;
        self.drag_anchor = None;
        self.phase = Phase::Idle;
    }

    pub fn deliver_capture(&mut self) -> Result<CaptureDelivery> {
        let Phase::Waiting { counter, close, target } = self.phase else { return Err(Error::InvalidState); };
        if counter < 3 {
            self.phase = Phase::Waiting { counter: counter + 1, close, target };
            return Ok(CaptureDelivery::Repost { message: CAPTURE_MESSAGE, counter: counter + 1 });
        }
        self.phase = Phase::Modal { close, target };
        Ok(CaptureDelivery::Ready { target, close_afterwards: close })
    }

    /// Invoked after the fourth delivery. Capturing/encoding are primitive image
    /// operations; album naming, metadata, cleanup and selection stay in this core.
    /// Success leaves a paused modal album. Call complete_modal after its close.
    pub fn capture_into_album(&mut self, fs: &mut impl FileSystem, images: &mut impl ImagePrimitives,
        ui: &mut impl UiPrimitives, album: &mut SnapshotAlbum, city: &CityContext) -> Result<VPath> {
        let Phase::Modal { target, .. } = self.phase else { return Err(Error::InvalidState); };
        if self.capture_written || album.is_modal() { return Err(Error::InvalidState); }
        ui.set_frame_visible(false);
        let result = (|| {
            let mut image = images.capture(target)?;
            if matches!(images.depth(&image), ImageDepth::Bits16 | ImageDepth::Bits32) { images.force_opaque(&mut image)?; }
            album.go_to_city(fs, city, true)?;
            let destination = album.new_picture_path(fs, city)?;
            let bytes = images.encode_png(&image)?;
            if let Err(write_error) = fs.write(&destination, &bytes) {
                // §2: a failed write removes even partial output. SN9: cleanup errors surface.
                fs.remove_file(&destination)?;
                return Err(write_error);
            }
            album.add_new_graphic_for_current_city(fs, &destination, b"", city, false)?;
            album.begin_modal(ui)?;
            self.capture_written = true;
            Ok(destination)
        })();
        if result.is_err() {
            // SN8: failure is a completed attempt, using the same close/restore decision.
            self.finish_modal();
            ui.set_frame_visible(self.visible);
        }
        result
    }

    pub fn complete_modal(&mut self, fs: &mut impl FileSystem, ui: &mut impl UiPrimitives,
        album: &mut SnapshotAlbum) -> Result<()> {
        if !matches!(self.phase, Phase::Modal { .. }) { return Err(Error::InvalidState); }
        album.end_dialog(fs, ui)?;
        self.finish_modal();
        ui.set_frame_visible(self.visible);
        Ok(())
    }

    pub fn finish_modal(&mut self) {
        if let Phase::Modal { close, .. } = self.phase {
            self.phase = Phase::Idle;
            self.visible = !close;
            self.closed = close;
            self.capture_written = false;
        }
    }
}

fn valid_size(size: Size) -> bool {
    size.width > 0 && size.height > 0 && size.width <= i32::MAX as u32 && size.height <= i32::MAX as u32
}
