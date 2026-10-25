// SPDX-License-Identifier: GPL-3.0-or-later
//! Live mutation adapters execute repairs before owner notifications.
use super::{runtime::TextHost, *};

struct Deferred<'a, H> {
    host: &'a mut H,
    invalidations: Vec<Handle>,
}
impl<H: TextHost> conversion::CodePages for Deferred<'_, H> {
    fn system_code_page(&mut self) -> i32 {
        self.host.system_code_page()
    }
    fn multi_byte_to_wide(&mut self, p: i32, flags: u32, s: &[u8], d: &mut [u16]) -> usize {
        self.host.multi_byte_to_wide(p, flags, s, d)
    }
    fn wide_to_multi_byte(&mut self, p: i32, flags: u32, s: &[u16], d: &mut [u8], default: u8) -> usize {
        self.host.wide_to_multi_byte(p, flags, s, d, default)
    }
    fn composition_tail(&mut self) -> u16 {
        self.host.composition_tail()
    }
}
impl<H: TextHost> TextForeign for Deferred<'_, H> {
    fn previous_utf16(&mut self, c: &[u8], i: u32, s: i32) -> u32 {
        self.host.previous_utf16(c, i, s)
    }
    fn decode_utf16(&mut self, b: &[u8], s: i32) -> (u32, usize) {
        self.host.decode_utf16(b, s)
    }
    fn thai_level(&mut self, c: u16, p: u16, a: u32) -> u32 {
        self.host.thai_level(c, p, a)
    }
    fn composed_length(&mut self, b: &[u8], s: i32) -> usize {
        self.host.composed_length(b, s)
    }
    fn correction(&mut self, reason: u32) {
        self.host.correction(reason);
    }
    fn invalidate(&mut self, h: Handle) {
        self.invalidations.push(h);
    }
    fn notify(&mut self, h: Handle, kind: u32) {
        self.host.notify(h, kind);
    }
}

impl TextEdit {
    pub fn set_text(&mut self, source: Option<&[u8]>, count: u32, notify: bool, f: &mut impl TextHost) -> Result<bool> {
        let empty = [0];
        let source = source.unwrap_or(&empty);
        let count = if count == 0 {
            source.iter().position(|&b| b == 0).unwrap_or(source.len())
        } else {
            count as usize
        };
        let source = source
            .get(..count)
            .ok_or(Unsupported::Precondition("TextEdit SetText readable count"))?;
        self.text.clear();
        let candidate = self.filtered(source, &[], 0, f)?;
        let retained = self.retained(candidate.len(), source, 0)?;
        if candidate.len() > self.maximum as usize {
            f.correction(1);
        }
        self.text.extend_from_slice(&candidate[..retained]);
        if candidate.is_empty() {
            self.recalculate(f)?;
        } else {
            self.calculate_lines(f)?;
            self.bar_values(true, f)?;
            self.position = Position::default();
            self.automatic_bars(f)?;
        }
        f.invalidate(self.window);
        self.undo.clear();
        self.redo.clear();
        if notify {
            f.notify(self.text_view, TEXT_CHANGED);
        }
        Ok(true)
    }
    fn raw_live<T, H: TextHost>(
        &mut self,
        f: &mut H,
        call: impl FnOnce(&mut Self, &mut Deferred<'_, H>) -> Result<T>,
    ) -> Result<T> {
        let mut deferred = Deferred {
            host: f,
            invalidations: Vec::new(),
        };
        let result = call(self, &mut deferred)?;
        let invalidations = deferred.invalidations;
        self.execute_repairs(f)?;
        for h in invalidations {
            f.invalidate(h);
        }
        Ok(result)
    }
    pub fn insert_raw(&mut self, bytes: &[u8], index: u32, f: &mut impl TextHost) -> Result<u32> {
        self.raw_live(f, |e, f| e.insert_bytes(bytes, index, f))
    }
    pub fn remove_raw(&mut self, start: u32, count: u32, f: &mut impl TextHost) -> Result<Vec<u8>> {
        self.raw_live(f, |e, f| Ok(e.remove_bytes(start, count, f)))
    }
    pub fn remove_char_raw(&mut self, index: u32, ordinary: bool, f: &mut impl TextHost) -> Result<Vec<u8>> {
        self.raw_live(f, |e, f| e.remove_character(index, ordinary, f))
    }
    pub fn undo(&mut self, f: &mut impl TextHost) -> Result<bool> {
        self.raw_live(f, |e, f| e.undo_bytes(f))
    }
    pub fn redo(&mut self, f: &mut impl TextHost) -> Result<bool> {
        self.raw_live(f, |e, f| e.redo_bytes(f))
    }
    pub fn insert_text(&mut self, candidate: &[u8], f: &mut impl TextHost) -> Result<bool> {
        if self.flags & 0x200 != 0 && candidate.first() == Some(&b'\n') {
            self.collapse();
            f.invalidate(self.window);
            return Ok(true);
        }
        let before = self.position;
        let mut removed = Vec::new();
        if before.anchor != before.active {
            let low = before.anchor.min(before.active);
            removed = self.remove_raw(low, before.anchor.abs_diff(before.active), f)?;
            self.position.index = low;
        }
        if !self.insert_mode() && removed.is_empty() && self.position.index < self.text.len() as u32 {
            removed = self.remove_char_raw(self.position.index, false, f)?;
        }
        let at = self.position.index;
        let inserted = self.insert_raw(candidate, at, f)?;
        if inserted == 0 {
            f.correction(1);
        } else {
            self.record_insertion(at, inserted, removed, before);
        }
        self.preferred_x = u32::MAX;
        self.collapse();
        self.insertion_in_view(f)?;
        if self.flags & 0x800 != 0 {
            f.notify(self.text_view, TEXT_CHANGED);
        }
        Ok(true)
    }
    pub fn adjust_line_positions(&mut self, increment: u32, count: u32) -> Result<()> {
        let count = count as usize;
        if count > self.lines.len() {
            let at = count
                .checked_sub(2)
                .ok_or(Unsupported::Precondition("TextEdit line extension source"))?;
            let fill = self
                .lines
                .get(at)
                .copied()
                .ok_or(Unsupported::Precondition(
                    "TextEdit line extension indexed before resize",
                ))?
                .wrapping_add(increment);
            self.lines.resize(count, fill);
        } else if count > 1 {
            self.lines[count - 1] = self.lines[count - 2].wrapping_add(increment);
        } else {
            *self
                .lines
                .first_mut()
                .ok_or(Unsupported::Precondition("TextEdit adjust empty starts"))? = 0;
        }
        Ok(())
    }
}
