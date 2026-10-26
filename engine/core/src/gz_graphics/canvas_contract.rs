// SPDX-License-Identifier: GPL-3.0-or-later
//! Native canvas contracts from gz-graphics §7.4. All OS calls remain foreign.
// SPEC-GAP: primary slots 25-28 in §7.1 have no behavior contracts; not exposed here.
use super::*;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeMessage {
    pub handle: usize,
    pub id: u32,
    pub wparam: u32,
    pub lparam: u32,
}

pub trait CanvasListener: Send + Sync {
    /// None is the native handled=false channel; Some supplies the result.
    fn message(&self, message: NativeMessage) -> Option<isize>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasMode {
    pub width: i32,
    pub height: i32,
    pub discriminator: u32,
    pub format: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeCall {
    ClassIcon(u32),
    FreeAuxiliary(usize),
    BufferBounds(Rect),
    MoveWindow(i32, i32, bool),
    BeginPaint,
    EndPaint,
    FillClient(u32),
    Capture(bool),
    Mouse {
        subtype: u32,
        position: [i32; 2],
        amount: i32,
        flags: u32,
    },
    Key {
        subtype: u32,
        value: i32,
        repeat: u16,
        flags: u32,
    },
    Delete(u32),
}

/// Host facts and side effects, rather than invented Windows resource ownership.
pub trait CanvasNativeHost {
    fn call(&mut self, call: NativeCall);
    fn default_message(&mut self, message: NativeMessage) -> isize;
    fn interfaces_available(&self) -> bool;
    fn modes(&self) -> Vec<CanvasMode>;
    /// Creation passes the procedure bridge; release is (-1,null,1,1).
    /// A handle here records the synchronous first procedure message.
    fn change_mode(&mut self, index: i32, creation: bool) -> Option<usize>;
    fn mode_status(&self) -> i32;
    fn windowed(&self) -> bool;
    fn work_area(&self) -> Option<Rect>;
    fn window_rect(&self) -> Option<Rect>;
    fn visible(&self) -> bool;
    fn mapped_client(&self) -> Rect;
    fn has_paint_dc(&self) -> bool;
    fn owns_capture(&self, handle: usize) -> bool;
    /// Already sampled Shift/Control/Alt/left/right/middle in canvas bit order.
    fn key_flags(&self) -> u32;
    fn screen_to_client(&self, position: [i32; 2]) -> [i32; 2];
    fn wheel_lines(&self) -> Option<u32>;
    fn locale_code_page(&self, locale: u16) -> u32;
    fn framework_code_page(&self) -> u32;
    fn additional_bytes(&self, code_page: u32, first: u8) -> usize;
    fn convert_text(&self, from: u32, to: u32, bytes: &[u8]) -> Vec<u8>;
}

impl CanvasW32 {
    pub fn hwnd_word(&self) -> &Option<usize> {
        &self.handle
    }
    pub fn hwnd(&self) -> Option<usize> {
        self.handle
    }
    pub fn native_listener(&mut self, listener: Option<&Arc<dyn CanvasListener>>, enable: bool) -> bool {
        let Some(listener) = listener else { return false };
        let position = self.listeners.iter().position(|old| Arc::ptr_eq(old, listener));
        if enable {
            if position.is_some() {
                return false;
            }
            self.listeners.insert(0, Arc::clone(listener));
            true
        } else if let Some(position) = position {
            self.listeners.remove(position);
            true
        } else {
            false
        }
    }

    pub fn center_native(&self, host: &mut dyn CanvasNativeHost) {
        if !host.windowed() {
            return;
        }
        if let (Some(work), Some(window)) = (host.work_area(), host.window_rect()) {
            host.call(NativeCall::MoveWindow(
                work.0 + (work.width() - window.width()) / 2,
                work.1 + (work.height() - window.height()) / 2,
                host.visible(),
            ));
        }
    }

    pub fn realize_native(&mut self, create: bool, wanted: CanvasMode, host: &mut dyn CanvasNativeHost) -> bool {
        if create && self.handle.is_none() {
            if !host.interfaces_available() {
                return false;
            }
            let Some(index) = host.modes().iter().position(|mode| {
                mode.width == wanted.width
                    && mode.height == wanted.height
                    && mode.discriminator == wanted.discriminator
                    && mode.format & 63 == wanted.format & 63
            }) else {
                return false;
            };
            if let Some(handle) = host.change_mode(index as i32, true) {
                self.handle = Some(handle);
                host.call(NativeCall::ClassIcon(1000));
            }
            if host.mode_status() != 0 {
                return false;
            }
            self.center_native(host);
            self.wheel_message = self
                .host
                .register_wheel_message()
                .filter(|&v| v != 0)
                .unwrap_or(u32::MAX);
            self.host.call(self.handle, CanvasCall::Event(8));
            true
        } else {
            if host.interfaces_available() {
                host.change_mode(-1, false);
            }
            self.handle = None;
            if let Some(auxiliary) = self.auxiliary.take() {
                host.call(NativeCall::FreeAuxiliary(auxiliary));
            }
            self.host.call(None, CanvasCall::Event(9));
            true
        }
    }

    pub fn initialize_native(&mut self, bounds: Rect, wanted: CanvasMode, host: &mut dyn CanvasNativeHost) -> bool {
        if self.realized {
            return true;
        }
        self.bounds = bounds;
        self.auxiliary = None;
        let result = self.realize_native(true, wanted, host);
        self.realized = true; // The native byte is set even when realization failed.
        result
    }

    pub fn suspend_native(&mut self, wanted: CanvasMode, host: &mut dyn CanvasNativeHost) -> bool {
        if self.realized {
            self.suspended = true;
            self.show(false);
            self.host.call(self.handle, CanvasCall::Event(6));
            self.realize_native(false, wanted, host);
            self.realized = false;
        }
        true
    }

    pub fn resume_native(&mut self, bounds: Rect, wanted: CanvasMode, host: &mut dyn CanvasNativeHost) -> bool {
        if self.realized {
            return true;
        }
        self.bounds = bounds;
        let result = self.realize_native(true, wanted, host);
        self.realized = true;
        if !result {
            self.shutdown();
            return false;
        }
        self.suspended = false;
        self.host.call(self.handle, CanvasCall::Event(7));
        self.show(true);
        true
    }

    /// Slot 21 deletes regardless of outstanding references; the host owns deallocation.
    pub fn delete_native(&mut self, host: &mut dyn CanvasNativeHost) -> i32 {
        if self.realized {
            self.shutdown();
        }
        host.call(NativeCall::Delete(1));
        0
    }

    pub fn request_repaint(&mut self) {
        if !self.paint_suppressed {
            self.host.call(self.handle, CanvasCall::Event(4));
        }
    }

    pub fn native_message(&mut self, mut message: NativeMessage, host: &mut dyn CanvasNativeHost) -> isize {
        if self.handle.is_none() {
            self.handle = Some(message.handle);
            host.call(NativeCall::ClassIcon(1000));
        }
        if message.id == self.wheel_message {
            message.id = 522;
            message.wparam = message.wparam.wrapping_shl(16);
        }
        for listener in &self.listeners {
            if let Some(result) = listener.message(message) {
                return result;
            }
        }
        let low = message.lparam as u16;
        let high = (message.lparam >> 16) as u16;
        let position = [i32::from(low as i16), i32::from(high as i16)];
        let emit_key = |host: &mut dyn CanvasNativeHost, subtype, value| {
            host.call(NativeCall::Key {
                subtype,
                value,
                repeat: low,
                flags: host.key_flags() | (message.lparam & 0x40000000),
            });
        };
        match message.id {
            2 => {}
            3 | 5 | 562 => {
                let mapped = host.mapped_client();
                self.bounds = match message.id {
                    3 => Rect(
                        i32::from(low),
                        i32::from(high),
                        i32::from(low) + mapped.width(),
                        i32::from(high) + mapped.height(),
                    ),
                    5 => Rect(
                        mapped.0,
                        mapped.1,
                        mapped.0 + i32::from(low),
                        mapped.1 + i32::from(high),
                    ),
                    _ => mapped,
                };
                host.call(NativeCall::BufferBounds(self.bounds));
                if message.id == 562 {
                    self.host.call(self.handle, CanvasCall::Event(5));
                }
            }
            6 => {
                self.host.call(
                    self.handle,
                    CanvasCall::Event(if message.wparam as u16 != 0 { 2 } else { 3 }),
                );
            }
            15 => {
                if self.handle == Some(message.handle) {
                    host.call(NativeCall::BeginPaint);
                    self.request_repaint();
                    if self.paint_suppressed && host.has_paint_dc() {
                        host.call(NativeCall::FillClient(16));
                    }
                    host.call(NativeCall::EndPaint);
                }
            }
            16 => {
                self.host.call(self.handle, CanvasCall::Event(10));
            }
            32 => return 1,
            81 => {
                self.code_page = host.locale_code_page(low);
                if self.code_page == 0 {
                    self.code_page = 0x10;
                }
                return host.default_message(message);
            }
            256 | 260 | 257 | 261 => {
                if self.input_enabled(1) && message.wparam != 0xe5 {
                    emit_key(
                        host,
                        if matches!(message.id, 256 | 260) { 5 } else { 6 },
                        message.wparam as i32,
                    );
                }
            }
            258 => {
                if self.input_enabled(1) {
                    let byte = message.wparam as u8;
                    let target = host.framework_code_page();
                    if self.code_page == target {
                        emit_key(host, 4, i32::from(byte as i8));
                    } else {
                        if self.pending_text.is_empty() {
                            self.pending_bytes = host.additional_bytes(self.code_page, byte) + 1;
                        }
                        self.pending_text.push(byte);
                        if self.pending_text.len() >= self.pending_bytes {
                            for byte in host.convert_text(self.code_page, target, &self.pending_text) {
                                emit_key(host, 4, i32::from(byte as i8));
                            }
                            self.pending_text.clear();
                            self.pending_bytes = 0;
                        }
                    }
                }
            }
            288 => {
                let result = host.default_message(message);
                return if result == 0 { 0x10000 } else { result };
            }
            512 | 513 | 516 | 514 | 517 | 522 => {
                if message.id != 522 {
                    self.coordinates = [position[0] as u32, position[1] as u32];
                }
                if self.input_enabled(2) {
                    let mut point = position;
                    let (subtype, amount) = match message.id {
                        512 => (13, 0),
                        513 | 516 => {
                            if !host.owns_capture(message.handle) {
                                host.call(NativeCall::Capture(true));
                            }
                            (if message.id == 513 { 7 } else { 8 }, 0)
                        }
                        514 | 517 => {
                            if host.owns_capture(message.handle) && message.wparam & 3 == 0 {
                                host.call(NativeCall::Capture(false));
                            }
                            (if message.id == 514 { 10 } else { 11 }, 0)
                        }
                        _ => {
                            point = host.screen_to_client(point);
                            let mut amount = i32::from((message.wparam >> 16) as i16) * 100 / 120;
                            if amount > 999 {
                                amount = 100;
                            }
                            if let Some(lines) = host.wheel_lines() {
                                amount *= lines.min(10) as i32;
                            }
                            (14, amount)
                        }
                    };
                    let flags = ((message.wparam & 4) >> 2)
                        | ((message.wparam & 8) >> 2)
                        | (host.key_flags() & 4)
                        | ((message.wparam & 1) << 4)
                        | ((message.wparam & 2) << 4)
                        | ((message.wparam & 16) << 2);
                    host.call(NativeCall::Mouse {
                        subtype,
                        position: [
                            point[0].max(0).min(self.width() - 1),
                            point[1].max(0).min(self.height() - 1),
                        ],
                        amount,
                        flags,
                    });
                }
            }
            _ => return host.default_message(message),
        }
        0
    }
}

pub trait CanvasQueue {
    fn peek_remove(&mut self, first: u32, last: u32) -> Option<NativeMessage>;
    fn repost(&mut self, message: NativeMessage) -> bool;
}

pub fn filter_keyboard_queue(host: &mut dyn CanvasQueue) -> u32 {
    let mut saved = Vec::new();
    while let Some(message) = host.peek_remove(0x100, 0x108) {
        if saved.len() == 1024 {
            break;
        } // The extra removal precedes the count test.
        saved.push(message);
    }
    let mut discarded: Vec<_> = saved.iter().map(|m| matches!(m.id, 258 | 259 | 262 | 263)).collect();
    for i in (0..saved.len()).rev() {
        if !matches!(saved[i].id, 257 | 261) {
            continue;
        }
        for j in (0..i).rev() {
            if matches!(saved[j].id, 257 | 261) {
                break;
            }
            if saved[j].wparam == saved[i].wparam {
                discarded[i] = true;
                discarded[j] = true;
            }
        }
    }
    for (message, &discard) in saved.iter().zip(&discarded) {
        if !discard {
            host.repost(*message);
        }
    }
    discarded.iter().filter(|&&discard| discard).count() as u32
}
