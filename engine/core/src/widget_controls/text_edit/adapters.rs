// SPDX-License-Identifier: GPL-3.0-or-later
//! Text-interface adapters. Handles returned here are borrowed.
use super::{runtime::TextHost, *};
use crate::widget_controls::Rect;

/// Separate writes let a native adapter preserve aliased output ordering.
pub trait WordOutputs {
    fn write(&mut self, output: Handle, value: u32);
}

impl TextEdit {
    /// The owner filters chunks; only lib::_strncpy itself is a TE8 foreign
    /// input. In particular the count passed to it is the original count.
    pub fn remove_bad_buffer(
        &self,
        buffer: &mut [u8],
        count: &mut u32,
        copy: impl FnOnce(&mut [u8], &[u8], u32),
    ) -> Result<u32> {
        if self.flags & 0x200 == 0 {
            return Ok(0);
        }
        let original = *count;
        let mut at = 0usize;
        let mut kept = Vec::new();
        let mut removed = 0u32;
        while at < original as usize {
            let tail = buffer
                .get(at..)
                .ok_or(Unsupported::Precondition("TextEdit filter readable source"))?;
            let length = self.encoding.char_length(tail, original as usize - at)?;
            let chunk = tail
                .get(..length)
                .ok_or(Unsupported::Precondition("TextEdit filter readable character"))?;
            if chunk[0] == b'\n' {
                removed = removed.wrapping_add(length as u32);
            } else {
                kept.extend_from_slice(chunk);
            }
            at += length;
        }
        if removed != 0 {
            copy(buffer, &kept, original);
            *count = kept.len() as u32;
        }
        Ok(removed)
    }
    /// Export GetTextLength is a string copy adapter, despite its name.
    pub fn copy_to_string<R>(&self, assign: impl FnOnce(&[u8]) -> R) -> R {
        assign(&self.text)
    }
    pub fn text_length(&self) -> u32 {
        self.text.len() as u32
    }
    pub fn caption(&self) -> &[u8] {
        &self.text
    }
    pub fn set_caption(&mut self, text: &[u8], f: &mut impl TextHost) -> Result<bool> {
        self.set_text(Some(text), text.len() as u32, false, f)
    }
    pub fn get_text_buffer(&self, output: &mut [u8], count: &mut u32) -> Result<()> {
        *count = (*count).min(self.text_length());
        let destination = output
            .get_mut(..*count as usize)
            .ok_or(Unsupported::Precondition("TextEdit writable copy destination"))?;
        destination.copy_from_slice(&self.text[..*count as usize]);
        Ok(())
    }
    pub fn get_selection(&self, anchor: Handle, active: Handle, out: &mut impl WordOutputs) -> bool {
        out.write(anchor, self.position.anchor);
        out.write(active, self.position.active);
        true
    }
    pub fn get_insertion(&self, line: Handle, column: Handle, out: &mut impl WordOutputs) -> bool {
        out.write(line, self.position.line);
        out.write(column, self.position.column);
        true
    }
    pub fn get_font(&self) -> Option<Handle> {
        self.runtime.font
    }
    pub fn get_font_style(&self, output: &mut u32) -> bool {
        if let Some(style) = self.runtime.font_style {
            *output = style;
            true
        } else {
            false
        }
    }
    pub fn get_bar_image(&self, axis: usize, image: &mut Option<Handle>, rect: &mut Rect) -> bool {
        *image = self.runtime.images[axis];
        *rect = self.runtime.image_rects[axis];
        image.is_some()
    }
    pub fn get_background_image(&self, image: &mut Option<Handle>, option: &mut u8) {
        *image = self.runtime.background;
        *option = self.runtime.background_option;
    }
    pub fn get_caret_period(&self, output: &mut u32) -> bool {
        *output = self.caret_period;
        true
    }
    pub fn set_flag(&mut self, mask: u32, f: &mut impl TextHost) -> Result<()> {
        self.set_flags(self.flags | mask, f)
    }
}
