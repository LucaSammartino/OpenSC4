// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable cGZWinTextEdit owner. Byte-stage entry points retain explicit repair
//! records for fixtures; live callers execute them with `execute_repairs`.
//! Layout, input, painting and foreign service orchestration are separate modules.
//! No plugin or ABI uses this owner.
pub mod edits;
pub mod encoding;
pub mod input;
pub mod layout;
pub mod paint;
pub mod runtime;

use super::{Handle, Result, Unsupported};
use encoding::Encoding;

pub const IID: u32 = 0x231a_1c57;
pub const TEXT_VIEW_OFFSET: u32 = 0xd8;
pub const WINDOW_VTABLE: u32 = 0x00ad_fbd0;
pub const TEXT_VTABLE: u32 = 0x00ad_fa68;
pub const TEXT_CHANGED: u32 = 0x9b15_52d8;

/// Five words copied literally into history, including reversed endpoints.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub column: u32,
    pub index: u32,
    pub anchor: u32,
    pub active: u32,
}

/// Native owner work not executed by this byte/state stage. These are explicit
/// outstanding work, not opaque foreign callbacks claiming layout closure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repair {
    RecalculateAll,
    CalculateLines,
    ScrollValues,
    InsertionInView,
    AutomaticBars,
}

/// Only the individually specified foreign conversion/composition primitives.
/// The erase/keep/previous-character loop remains in the owner below.
pub trait TextForeign {
    fn previous_utf16(&mut self, context: &[u8], index: u32, selector: i32) -> u32;
    fn decode_utf16(&mut self, bytes: &[u8], selector: i32) -> (u32, usize);
    fn thai_level(&mut self, current: u16, previous: u16, argument: u32) -> u32;
    fn composed_length(&mut self, bytes: &[u8], selector: i32) -> usize;
    fn correction(&mut self, reason: u32);
    fn invalidate(&mut self, window: Handle);
    fn notify(&mut self, text_view: Handle, kind: u32);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub at: u32,
    /// On replay, erase these currently present bytes, then reinsert `restore`.
    pub erase: u32,
    pub restore: Vec<u8>,
    pub position: Position,
}

#[derive(Debug)]
pub struct TextEdit {
    pub window: Handle,
    pub text_view: Handle,
    pub text: Vec<u8>,
    pub lines: Vec<u32>,
    pub position: Position,
    pub flags: u32,
    pub encoding: Encoding,
    pub maximum: u32,
    pub history_limit: u32,
    pub undo: Vec<Action>,
    pub redo: Vec<Action>,
    pub preferred_x: u32,
    pub pending: Vec<u8>,
    pub gutters: [u32; 2],
    pub first_visible_line: u32,
    pub mask: u8,
    pub caret_period: u32,
    pub separate_interval: u32,
    pub repairs: Vec<Repair>,
    pub initialized: bool,
    pub runtime: runtime::Runtime,
}

impl TextEdit {
    /// Max-undo and preferred-X constructor defaults are not assigned in the
    /// incorporated contract. Require resolved values, rather than inventing
    /// native constructor defaults (TE9).
    pub fn with_resolved_defaults(
        window: Handle,
        text_view: Handle,
        flags: u32,
        encoding: Encoding,
        history_limit: u32,
        preferred_x: u32,
    ) -> Self {
        Self {
            window,
            text_view,
            text: Vec::new(),
            lines: Vec::new(),
            position: Position::default(),
            flags: runtime::normalized_flags(flags),
            encoding,
            maximum: u32::MAX,
            history_limit,
            undo: Vec::new(),
            redo: Vec::new(),
            preferred_x,
            pending: Vec::new(),
            gutters: [5, 0],
            first_visible_line: 0,
            mask: b'*',
            caret_period: 1000,
            separate_interval: 500,
            repairs: Vec::new(),
            initialized: false,
            runtime: runtime::Runtime::default(),
        }
    }

    pub fn set_maximum(&mut self, maximum: u32) {
        self.maximum = maximum;
    }

    pub fn set_history_limit(&mut self, limit: u32) {
        self.history_limit = limit;
    }

    pub fn insert_mode(&self) -> bool {
        self.flags & 0x4000 == 0
    }

    pub fn set_insert_mode(&mut self, insert: bool) -> bool {
        if insert {
            self.flags &= !0x4000;
        } else {
            self.flags |= 0x4000;
        }
        true
    }

    /// Requires coherent line starts when nonempty. No implicit line wrapping.
    pub fn line_column(&self, index: u32) -> Result<(u32, u32)> {
        if index as usize > self.text.len() {
            return Err(Unsupported::Precondition("TextEdit valid byte index"));
        }
        if self.lines.is_empty() || self.text.is_empty() || index == 0 {
            return Ok((0, 0));
        }
        for (line, pair) in self.lines.windows(2).enumerate() {
            if pair[0] <= index && index < pair[1] {
                return Ok((line as u32, index - pair[0]));
            }
        }
        let last = self.lines.len() - 1;
        let start = self.lines[last];
        if start > index {
            return Err(Unsupported::Precondition("TextEdit coherent line starts"));
        }
        let column = if self.text[index as usize - 1] == b'\n' {
            0
        } else {
            index - start
        };
        Ok((last as u32, column))
    }

    pub fn index_from_line_column(&self, line: u32, column: u32) -> (bool, u32) {
        if self.text.is_empty() || self.lines.is_empty() {
            return (false, 0);
        }
        let line = (line as usize).min(self.lines.len() - 1);
        let index = self.lines[line].wrapping_add(column);
        let length = self.text.len() as u32;
        (index <= length, index.min(length))
    }

    pub fn line_byte_count(&self, line: u32) -> Result<u32> {
        if self.text.is_empty() {
            return Ok(0);
        }
        if self.lines.is_empty() {
            return Err(Unsupported::Precondition("TextEdit nonempty text needs line starts"));
        }
        let line = (line as usize).min(self.lines.len() - 1);
        let end = self.lines.get(line + 1).copied().unwrap_or(self.text.len() as u32);
        end.checked_sub(self.lines[line])
            .ok_or(Unsupported::Precondition("TextEdit coherent line starts"))
    }

    fn cache_coordinates(&mut self) -> Result<()> {
        (self.position.line, self.position.column) = self.line_column(self.position.index)?;
        Ok(())
    }

    pub fn set_selection(&mut self, anchor: u32, active: u32, f: &mut impl TextForeign) -> Result<bool> {
        let length = self.text.len() as u32;
        self.position.anchor = anchor.min(length);
        self.position.active = active.min(length);
        self.position.index = self.position.active;
        self.cache_coordinates()?;
        f.invalidate(self.window);
        Ok(true)
    }

    pub fn set_insertion_index(&mut self, index: u32, f: &mut impl TextForeign) -> Result<bool> {
        self.position.index = index.min(self.text.len() as u32);
        self.cache_coordinates()?;
        f.invalidate(self.window);
        Ok(true)
    }

    pub fn set_insertion_line_column(&mut self, line: u32, column: u32) -> Result<bool> {
        if self.lines.is_empty() {
            self.position.line = 0;
            self.position.column = 0;
            self.position.index = 0;
        } else {
            self.position.line = line.min(self.lines.len() as u32 - 1);
            self.position.column = column.min(self.line_byte_count(self.position.line)?);
            self.position.index = self.index_from_line_column(self.position.line, self.position.column).1;
        }
        Ok(true)
    }

    pub fn fix_insertion(&mut self) -> Result<()> {
        if self.text.is_empty() {
            self.position = Position::default();
        } else {
            self.position.index = self.position.index.min(self.text.len() as u32);
            self.cache_coordinates()?;
            self.position.active = self.position.active.min(self.text.len() as u32);
        }
        Ok(())
    }

    /// Copy exactly min(requested,length) bytes; no terminator is appended.
    pub fn copy_bytes(&self, output: &mut [u8], count: &mut u32) -> Result<()> {
        let copied = (*count as usize).min(self.text.len());
        if output.len() < copied {
            return Err(Unsupported::Precondition("TextEdit writable copy destination"));
        }
        output[..copied].copy_from_slice(&self.text[..copied]);
        *count = copied as u32;
        Ok(())
    }

    /// String overload, not the unresolved lib::_strncpy buffer adapter.
    pub fn remove_bad_characters(&self, candidate: &mut Vec<u8>) -> Result<usize> {
        if self.flags & 0x200 == 0 {
            return Ok(0);
        }
        let (mut at, mut removed) = (0, 0);
        let mut kept = Vec::new();
        while at < candidate.len() {
            let length = self.encoding.char_length(&candidate[at..], candidate.len() - at)?;
            let chunk = candidate
                .get(at..at + length)
                .ok_or(Unsupported::Precondition("TE1 filtering readable character span"))?;
            if chunk[0] == b'\n' {
                removed += length;
            } else {
                kept.extend_from_slice(chunk);
            }
            at += length;
        }
        *candidate = kept;
        Ok(removed)
    }

    pub fn fix_composition(
        &self,
        candidate: &mut Vec<u8>,
        context: &[u8],
        index: u32,
        f: &mut impl TextForeign,
    ) -> Result<usize> {
        // Even an empty candidate calls the previous-character service.
        let mut previous = f.previous_utf16(context, index, self.encoding.selector);
        let (mut at, mut removed) = (0, 0);
        while at < candidate.len() {
            let (current, consumed) = f.decode_utf16(&candidate[at..], self.encoding.selector);
            if consumed == 0 {
                removed += candidate.len() - at;
                candidate.truncate(at);
                break;
            }
            if consumed > candidate.len() - at {
                return Err(Unsupported::Precondition("TE5 conversion consumed readable span"));
            }
            if f.thai_level(current as u16, previous as u16, 0) == 0 {
                candidate.drain(at..at + consumed);
                removed += consumed;
            } else {
                at += consumed;
                previous = current;
            }
        }
        Ok(removed)
    }

    fn filtered(&self, source: &[u8], context: &[u8], index: u32, f: &mut impl TextForeign) -> Result<Vec<u8>> {
        let mut candidate = source.to_vec();
        let mut removed = self.remove_bad_characters(&mut candidate)?;
        removed += self.fix_composition(&mut candidate, context, index, f)?;
        if removed != 0 {
            f.correction(0);
        }
        Ok(candidate)
    }

    fn retained(&self, candidate_len: usize, stepping_source: &[u8], existing: usize) -> Result<usize> {
        if existing
            .checked_add(candidate_len)
            .filter(|&n| n <= u32::MAX as usize)
            .is_none()
        {
            return Err(Unsupported::Precondition("TE3 nonoverflowing string length"));
        }
        if existing + candidate_len <= self.maximum as usize {
            return Ok(candidate_len);
        }
        if self.encoding.selector != 8 && self.encoding.selector != 100 {
            if existing > self.maximum as usize {
                // Native wraps M-L and reaches string/source failures. Do not
                // turn that invalid domain into a successful zero insertion.
                return Err(Unsupported::Precondition(
                    "TE3 lowered ordinary limit wraps retained count",
                ));
            }
            return Ok(self.maximum as usize - existing);
        }
        let mut retained = 0;
        while existing + retained < self.maximum as usize {
            let source = stepping_source
                .get(retained..)
                .filter(|source| !source.is_empty())
                .ok_or(Unsupported::Precondition("TE1 truncation readable source"))?;
            retained += self.encoding.char_length(source, source.len())?;
        }
        if retained > candidate_len {
            return Err(Unsupported::Precondition("TE1 retained prefix within filtered string"));
        }
        Ok(retained)
    }

    /// Whole-text byte stage. Count zero requests strlen; explicit nonzero
    /// count may include NUL bytes. Old text is erased before correction.
    pub fn set_text_bytes(
        &mut self,
        source: Option<&[u8]>,
        count: u32,
        notify: bool,
        f: &mut impl TextForeign,
    ) -> Result<bool> {
        let empty = [0];
        let source = source.unwrap_or(&empty);
        let length = if count == 0 {
            source.iter().position(|&b| b == 0).unwrap_or(source.len())
        } else {
            count as usize
        };
        let source = source
            .get(..length)
            .ok_or(Unsupported::Precondition("TextEdit SetText readable count"))?;
        self.text.clear();
        let candidate = self.filtered(source, &[], 0, f)?;
        let retained = self.retained(candidate.len(), source, 0)?;
        if candidate.len() > self.maximum as usize {
            f.correction(1);
        }
        self.text.extend_from_slice(&candidate[..retained]);
        if candidate.is_empty() {
            self.repairs.push(Repair::RecalculateAll);
        } else {
            self.repairs.extend([Repair::CalculateLines, Repair::ScrollValues]);
            self.position = Position::default();
            self.repairs.push(Repair::AutomaticBars);
        }
        f.invalidate(self.window);
        self.undo.clear();
        self.redo.clear();
        if notify {
            f.notify(self.text_view, TEXT_CHANGED);
        }
        Ok(true)
    }

    pub fn insert_bytes(&mut self, source: &[u8], index: u32, f: &mut impl TextForeign) -> Result<u32> {
        let at = (index as usize).min(self.text.len());
        let candidate = self.filtered(source, &self.text, at as u32, f)?;
        if candidate.is_empty() {
            return Ok(0);
        }
        let retained = self.retained(candidate.len(), &candidate, self.text.len())?;
        if self.text.len() + candidate.len() > self.maximum as usize {
            f.correction(1);
        }
        self.text.splice(at..at, candidate[..retained].iter().copied());
        self.position.index = at as u32 + self.encoding.glyph_count(&candidate[..retained]) as u32;
        if self.initialized {
            self.repairs
                .extend([Repair::CalculateLines, Repair::InsertionInView, Repair::AutomaticBars]);
        }
        Ok(retained as u32)
    }

    /// Leaves selection endpoints untouched, even if now beyond text length.
    pub fn remove_bytes(&mut self, start: u32, count: u32, f: &mut impl TextForeign) -> Vec<u8> {
        let at = start as usize;
        if at >= self.text.len() {
            return Vec::new();
        }
        let length = (count as usize).min(self.text.len() - at);
        let removed = self.text.drain(at..at + length).collect();
        self.position.index = start;
        self.repairs
            .extend([Repair::CalculateLines, Repair::InsertionInView, Repair::AutomaticBars]);
        f.invalidate(self.window);
        removed
    }

    pub fn remove_character(&mut self, index: u32, ordinary: bool, f: &mut impl TextForeign) -> Result<Vec<u8>> {
        if self.text.is_empty() {
            return Err(Unsupported::Precondition("TE3 RemoveChar empty text underflows"));
        }
        let at = (index as usize).min(self.text.len() - 1);
        let length = if ordinary {
            self.encoding.char_length(&self.text[at..], self.text.len() - at)?
        } else {
            f.composed_length(&self.text[at..], self.encoding.selector)
        };
        if length > self.text.len() - at {
            return Err(Unsupported::Precondition("TE3 RemoveChar valid character span"));
        }
        Ok(self.remove_bytes(at as u32, length as u32, f))
    }

    pub fn record_insertion(&mut self, at: u32, inserted: u32, replaced: Vec<u8>, before: Position) -> bool {
        if self.flags & 0x100 == 0 {
            return false;
        }
        let n = self.undo.len();
        if n >= self.history_limit as usize {
            self.undo.drain(..n / 4);
        }
        self.undo.push(Action {
            at,
            erase: inserted,
            restore: replaced,
            position: before,
        });
        self.redo.clear();
        true
    }

    pub fn record_deletion(&mut self, at: u32, removed: Vec<u8>, before: Position) -> bool {
        if self.flags & 0x100 == 0 {
            return false;
        }
        self.undo.push(Action {
            at,
            erase: 0,
            restore: removed,
            position: before,
        });
        self.redo.clear();
        true
    }

    fn replay(&mut self, redo: bool, f: &mut impl TextForeign) -> Result<bool> {
        if self.flags & 0x100 == 0 {
            return Ok(false);
        }
        let stack = if redo { &self.redo } else { &self.undo };
        let action = match stack.last() {
            Some(action) => action.clone(),
            None => return Ok(false),
        };
        let at = action.at as usize;
        let end = at
            .checked_add(action.erase as usize)
            .filter(|&end| at <= end && end <= self.text.len());
        let end = end.ok_or(Unsupported::Precondition("TextEdit coherent replay range"))?;
        let inverse = Action {
            at: action.at,
            erase: action.restore.len() as u32,
            restore: self.text[at..end].to_vec(),
            position: self.position,
        };
        self.text.splice(at..end, action.restore.iter().copied());
        self.position = action.position;
        if redo {
            self.undo.push(inverse);
            self.redo.pop();
        } else {
            self.redo.push(inverse);
            self.undo.pop();
        }
        self.repairs.extend([Repair::RecalculateAll, Repair::InsertionInView]);
        f.invalidate(self.window);
        Ok(true)
    }

    pub fn undo_bytes(&mut self, f: &mut impl TextForeign) -> Result<bool> {
        self.replay(false, f)
    }

    pub fn redo_bytes(&mut self, f: &mut impl TextForeign) -> Result<bool> {
        self.replay(true, f)
    }

    fn collapse(&mut self) {
        self.position.anchor = self.position.index;
        self.position.active = self.position.index;
    }

    pub fn insert_high_level_bytes(&mut self, candidate: &[u8], f: &mut impl TextForeign) -> Result<bool> {
        if self.flags & 0x200 != 0 && candidate.first() == Some(&b'\n') {
            self.collapse();
            f.invalidate(self.window);
            return Ok(true);
        }
        let before = self.position;
        let mut removed = Vec::new();
        if before.anchor != before.active {
            let low = before.anchor.min(before.active);
            removed = self.remove_bytes(low, before.anchor.abs_diff(before.active), f);
            self.position.index = low;
        }
        if !self.insert_mode() && removed.is_empty() && (self.position.index as usize) < self.text.len() {
            removed = self.remove_character(self.position.index, false, f)?;
        }
        let at = self.position.index;
        let inserted = self.insert_bytes(candidate, at, f)?;
        if inserted == 0 {
            f.correction(1);
        } else {
            self.record_insertion(at, inserted, removed, before);
        }
        self.preferred_x = u32::MAX;
        self.collapse();
        self.repairs.push(Repair::InsertionInView);
        if self.flags & 0x800 != 0 {
            f.notify(self.text_view, TEXT_CHANGED);
        }
        Ok(true)
    }

    /// Manager key-state queries belong to the still-unimplemented window
    /// adapter; the byte accumulation and high-level edit gates live here.
    pub fn character_bytes(&mut self, byte: u8, capturing: bool, f: &mut impl TextForeign) -> Result<bool> {
        if self.flags & 1 == 0 {
            return Ok(false);
        }
        let byte = if byte == b'\r' { b'\n' } else { byte };
        if byte < 0x20 && byte != b'\n' {
            return Ok(true);
        }
        if self.encoding.accumulates() {
            if self.pending.is_empty() && self.encoding.input_lead(byte) {
                self.pending.push(byte);
                return Ok(true);
            }
            self.pending.push(byte);
            if self.pending.len() < 1 + self.encoding.trailing_count(self.pending[0]) {
                return Ok(true);
            }
        } else {
            self.pending.clear();
            self.pending.push(byte);
        }
        if !capturing {
            let pending = self.pending.clone();
            self.insert_high_level_bytes(&pending, f)?;
        }
        self.pending.clear();
        Ok(false)
    }

    pub fn correct_lf(&mut self) {
        if self.position.index > 0
            && self.position.column > 0
            && self.text.get(self.position.index as usize - 1) == Some(&b'\n')
        {
            self.position.index -= 1;
            self.position.column -= 1;
        }
    }

    pub fn word_step(&mut self, delta: i32) -> Result<()> {
        if self.text.is_empty() || delta == 0 {
            return Ok(());
        }
        let length = self.text.len();
        let mut at = (self.position.index as usize).min(length);
        for _ in 0..delta.unsigned_abs() {
            if delta > 0 {
                while at < length {
                    if at + 1 == length {
                        at = length;
                        break;
                    }
                    if separator(self.text[at]) && !separator(self.text[at + 1]) {
                        at += 1;
                        break;
                    }
                    at += 1;
                }
            } else if at > 0 {
                at -= 1;
                while at > 0 && !(!separator(self.text[at]) && separator(self.text[at - 1])) {
                    at -= 1;
                }
            }
            if (delta > 0 && at == length) || (delta < 0 && at == 0) {
                break;
            }
        }
        self.position.index = at as u32;
        self.cache_coordinates()
    }
}

pub fn separator(byte: u8) -> bool {
    byte == b' ' || byte == b'\n'
}

#[cfg(test)]
mod tests;
