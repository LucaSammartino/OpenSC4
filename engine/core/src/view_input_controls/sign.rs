// SPDX-License-Identifier: GPL-3.0-or-later
use super::{label::screen_offset, project, sound, Key, MouseState, Pixel, PropFilter, PropId, PropWorld, Text, Unsupported, Vec3, PICK_UP, PUT_DOWN, SIGN_TYPE};
use crate::view_utilities::ViewUtilities;

pub const SIGN_POSITION_MESSAGE: u32 = 0x0BE1_0A20;
pub const MESSAGE_TARGET_INTERFACE: u32 = 0xE98B_2F57;
pub const TEXT_SERVICE: u32 = 0x27EB_FFFD;
pub const TEXT_SERVICE_INTERFACE: u32 = 0x07EC_0010;

pub trait SignWorld: PropWorld {
    fn ground_altitude(&self, p: Vec3) -> f32;
    fn sea_level(&self) -> f32;
    fn bounds_corners(&self, prop: PropId) -> (Vec3, Vec3);
}
/// External code-page decoding is a primitive text/input operation, never a
/// sign-tool append, length, editing or backspace algorithm.
pub trait CharacterCodec {
    fn decode(&self, code_page: u32, bytes: &[u8]) -> DecodedCharacter;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodedCharacter { Pending, Complete(Vec<u8>), Unsupported }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceText { Unbound, NoText, Text(Text) }

/// Typed core reply only. VIC2 leaves native message/service binding unsupported;
/// this does not invent a name, wire encoding or notification registration API.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SignPositionReply { pub answered: bool, pub screen: Vec3, pub consumed: bool }

#[derive(Debug)]
pub struct SignTool {
    state: MouseState,
    selected: Option<PropId>,
    offset: Vec3,
    edit: Text,
    backup: Text,
    pending: Vec<u8>,
    code_page: u32,
}
impl SignTool {
    pub const ID: u32 = 0x8B4C_E5A0;
    pub const CURSOR: u32 = 0x6B72_FECF;
    pub fn new(code_page: u32) -> Self {
        Self { state: MouseState::Idle, selected: None, offset: Vec3::ZERO,
            edit: Text::default(), backup: Text::default(), pending: Vec::new(), code_page }
    }
    /// Supported initialization only. Service acquisition/notification lifetime
    /// is an explicit VIC2 boundary, not emulated with an invented messaging API.
    pub fn init(&self, host: &mut impl SignWorld) { host.show_signs_and_labels(true); host.set_cursor(Self::CURSOR); }
    pub fn state(&self) -> MouseState { self.state }
    pub fn selected(&self) -> Option<PropId> { self.selected }
    pub fn edit_text(&self) -> &Text { &self.edit }
    pub fn pending_bytes(&self) -> &[u8] { &self.pending }
    pub fn is_multibyte(&self) -> bool { self.code_page == 8 || self.code_page >= 100 }
    pub fn select(&mut self, host: &mut impl SignWorld, prop: Option<PropId>) {
        if self.selected == prop { return; }
        if let Some(old) = self.selected {
            if host.text(old).is_empty() { host.remove_prop(old); } else { host.set_dynamic(old, false); }
        }
        self.selected = prop;
        if let Some(new) = prop { host.set_dynamic(new, true); }
    }
    pub fn mouse_down(&mut self, host: &mut impl SignWorld, utilities: &ViewUtilities, at: Pixel) -> Result<bool, Unsupported> {
        if self.state == MouseState::Typing {
            if let Some(prop) = self.selected { host.set_edit_mode(prop, false); host.set_dynamic(prop, true); }
            self.state = MouseState::Idle;
            return Ok(true);
        }
        if !host.renderer_available() { return Ok(true); }
        if let Some(prop) = host.pick_prop(at, PropFilter::Sign) {
            if !host.prop_info(prop).signpost { return Ok(true); }
            self.state = MouseState::Pressed;
            self.select(host, Some(prop));
            self.offset = screen_offset(utilities, host.position(prop), at);
            sound(host, PICK_UP);
        } else {
            let Some(p) = host.pick_terrain(at) else { return Ok(false); };
            let prop = host.add_city_prop(SIGN_TYPE).ok_or(Unsupported::ResourceUnavailable)?;
            let height = host.ground_altitude(p).max(host.sea_level());
            self.select(host, Some(prop));
            host.set_position(prop, Vec3::new(p.x, height, p.z));
            self.state = MouseState::Typing;
            self.edit = Text::default();
            self.backup = Text::default();
            host.set_text(prop, &self.edit);
            host.set_edit_mode(prop, true);
            sound(host, PUT_DOWN);
        }
        Ok(true)
    }
    pub fn mouse_move(&mut self, host: &mut impl SignWorld, at: Pixel) -> bool {
        if self.state == MouseState::Pressed && self.selected.is_some() { self.state = MouseState::Dragging; }
        if self.state == MouseState::Dragging {
            // VIC3: use the literal mouse+saved-offset rule. The native rounding
            // contract is unpublished; the core boundary truncates to pixels.
            let at = Pixel::new((at.x as f32 + self.offset.x) as i32, (at.y as f32 + self.offset.y) as i32);
            if let (Some(prop), Some(p)) = (self.selected, host.pick_terrain(at)) { host.set_position(prop, p); }
        } else if self.state == MouseState::Idle {
            let prop = host.pick_prop(at, PropFilter::Sign);
            self.select(host, prop);
        }
        true
    }
    pub fn mouse_up(&mut self, host: &mut impl SignWorld) -> bool {
        if self.state == MouseState::Dragging { sound(host, PUT_DOWN); }
        let typing = self.state == MouseState::Pressed || (self.state == MouseState::Dragging
            && self.selected.is_some_and(|p| host.text(p).is_empty()));
        if typing {
            if let Some(prop) = self.selected {
                self.state = MouseState::Typing;
                self.edit = host.text(prop);
                self.backup = self.edit.clone();
                host.set_edit_mode(prop, true);
            }
        } else { self.state = MouseState::Idle; }
        true
    }
    pub fn append_text(&mut self, host: &mut impl SignWorld, text: &Text) {
        let mut count = self.edit.glyph_count();
        for glyph in &text.0 {
            if count > 32 { break; }
            // VIC4: literal newline (LF); CR/CRLF normalization is unpublished.
            if glyph.as_slice() != b"\n" { self.edit.0.push(glyph.clone()); }
            count += 1;
        }
        if let Some(prop) = self.selected { host.set_text(prop, &self.edit); }
    }
    pub fn on_character(&mut self, host: &mut impl SignWorld, codec: &impl CharacterCodec, c: u8) -> Result<bool, Unsupported> {
        if self.state != MouseState::Typing { return Ok(false); }
        match c {
            0x1B => {},
            0x0D => {
                if let Some(prop) = self.selected { host.set_edit_mode(prop, false); host.set_dynamic(prop, true); }
                self.select(host, None);
                self.state = MouseState::Idle;
            }
            0x08 => {
                self.edit.0.pop();
                if let Some(prop) = self.selected { host.set_text(prop, &self.edit); }
            }
            0..=0x1F => {},
            _ if self.is_multibyte() => {
                self.pending.push(c);
                match codec.decode(self.code_page, &self.pending) {
                    DecodedCharacter::Pending => {},
                    DecodedCharacter::Complete(glyph) => {
                        self.pending.clear();
                        self.append_text(host, &Text(vec![glyph]));
                    }
                    DecodedCharacter::Unsupported => return Err(Unsupported::CharacterEncoding),
                }
            }
            _ => self.append_text(host, &Text(vec![vec![c]])),
        }
        Ok(true)
    }
    pub fn key_down(&mut self, host: &mut impl SignWorld, key: Key, modifiers: u32, service_text: ServiceText) -> Result<bool, Unsupported> {
        if self.state == MouseState::Typing {
            if key == Key::Escape {
                self.edit = self.backup.clone();
                if let Some(prop) = self.selected { host.set_text(prop, &self.edit); }
                return Ok(false);
            }
            if key == Key::V && modifiers & 7 == 2 {
                match service_text {
                    ServiceText::Unbound => return Err(Unsupported::ServiceProtocol),
                    ServiceText::NoText => {},
                    ServiceText::Text(text) => self.append_text(host, &text),
                }
            }
            return Ok(true);
        }
        if key == Key::Delete {
            if let Some(prop) = self.selected.take() { host.remove_prop(prop); self.state = MouseState::Idle; return Ok(true); }
        }
        Ok(false)
    }
    pub fn position_message(&self, host: &impl SignWorld, utilities: &ViewUtilities, message: u32) -> Option<SignPositionReply> {
        if message != SIGN_POSITION_MESSAGE || self.state != MouseState::Typing { return None; }
        let (a, b) = host.bounds_corners(self.selected?);
        let p = Vec3::new((a.x + b.x) / 2.0, b.y, (a.z + b.z) / 2.0);
        Some(SignPositionReply { answered: true, screen: project(utilities, p), consumed: false })
    }
    pub fn shutdown(&mut self, host: &mut impl SignWorld) {
        if self.state == MouseState::Typing {
            if let Some(prop) = self.selected { host.set_edit_mode(prop, false); host.set_dynamic(prop, true); }
        }
        self.select(host, None);
        // VIC2: no acquired service or registered notification exists here.
    }
}
