//! cRZCmdLine: a command-line text and the arguments split from it (docs/specs/command-line.md §1-§6).
//!
//! Pure Rust and byte oriented (the game's strings are 8-bit `cRZString`s). Every slot of the vtable
//! (§3) has a method here, named after the slot. Where the original reads or writes before the
//! argument list (a negative index in slots 4, 14 and 16, or an erase on an empty list) this core
//! returns [`CmdLineError`] instead and leaves the object unchanged: the original behaviour is memory
//! corruption, which a reimplementation must not reproduce.
//!
//! Open questions: CL1 (whether slot 16 is part of the published interface) and CL2-CL3 in
//! docs/specs/questions/command-line.md. No C ABI is published from here.

use std::fmt;

/// IID of `cIGZUnknown` (§3 slot 0).
pub const IID_IGZUNKNOWN: u32 = 0x0000_0001;
/// IID of `cIGZCmdLine` (§3 slot 0).
pub const IID_IGZCMDLINE: u32 = 0xA31E_539C;

/// The three switch prefix characters (§4).
pub const SWITCH_PREFIXES: [u8; 3] = [b'/', b'-', b'\\'];

/// Errors for the cases the original leaves undefined (§3 slots 4, 14, 16; §6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmdLineError {
    /// A negative index: the original reads (slots 4, 16) or removes (slot 14) before the list.
    NegativeIndex(i32),
    /// `EraseArgument` on an empty list: the original removes before the list.
    EraseFromEmpty,
}

impl fmt::Display for CmdLineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CmdLineError::NegativeIndex(i) => write!(f, "negative argument index {i}"),
            CmdLineError::EraseFromEmpty => write!(f, "erase from an empty argument list"),
        }
    }
}

impl std::error::Error for CmdLineError {}

/// Separators of the split (§2.1 rule 1): space, tab and newline only; `\r` is ordinary.
fn is_separator(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n'
}

fn is_switch(arg: &[u8]) -> bool {
    arg.first().is_some_and(|b| SWITCH_PREFIXES.contains(b))
}

/// "Any case" comparison after `MakeLower` (§3). `tolower` lowers `A`-`Z` only; bytes 0x80-0xFF are
/// left as they are here (literal reading of "ASCII text lowers with tolower"; question CL3).
fn lower(s: &[u8]) -> Vec<u8> {
    s.to_ascii_lowercase()
}

/// Split a text into arguments (§2.1).
pub fn split(text: &[u8]) -> Vec<Vec<u8>> {
    // Rule 2: trim separators at both ends.
    let start = match text.iter().position(|&b| !is_separator(b)) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let end = text.iter().rposition(|&b| !is_separator(b)).unwrap() + 1;
    let t = &text[start..end];

    let mut raw: Vec<&[u8]> = Vec::new();
    let mut arg_start = 0usize;
    let mut quotes = 0u32;
    let mut i = 0usize;
    while i < t.len() {
        let b = t[i];
        if is_separator(b) && quotes % 2 == 0 {
            // Rule 3: an even count ends the argument; a run of separators is one break.
            raw.push(&t[arg_start..i]);
            while i < t.len() && is_separator(t[i]) {
                i += 1;
            }
            arg_start = i;
            quotes = 0;
            continue;
        }
        if b == b'"' {
            quotes += 1;
        }
        i += 1;
    }
    // Rule 4: the text after the last break (non-empty: the end was trimmed).
    raw.push(&t[arg_start..]);

    raw.into_iter().map(unquote_split_arg).collect()
}

/// §2.1 rule 5: an argument that starts and ends with `"` loses its last character, then, if it is
/// not empty, its first.
fn unquote_split_arg(arg: &[u8]) -> Vec<u8> {
    let mut v = arg.to_vec();
    if v.first() == Some(&b'"') && v.last() == Some(&b'"') {
        v.pop();
        if !v.is_empty() {
            v.remove(0);
        }
    }
    v
}

/// Join arguments into a text (§2.2): arguments holding a separator are written between `"`,
/// inner quotes are not escaped, empty arguments write nothing; one space between arguments.
pub fn join(args: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for (k, a) in args.iter().enumerate() {
        if k > 0 {
            out.push(b' ');
        }
        if a.iter().any(|&b| is_separator(b)) {
            out.push(b'"');
            out.extend_from_slice(a);
            out.push(b'"');
        } else {
            out.extend_from_slice(a);
        }
    }
    out
}

/// The `cRZCmdLine` object (§1.1). `empty` is not stored: an out-of-range index yields `&[]`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CmdLine {
    args: Vec<Vec<u8>>,
    text: Vec<u8>,
    ref_count: u32,
}

impl CmdLine {
    /// `cRZCmdLine(char const*)` (§1.2): `text := s` verbatim, `args := split(text)`.
    pub fn new(text: &[u8]) -> Self {
        CmdLine {
            args: split(text),
            text: text.to_vec(),
            ref_count: 0,
        }
    }

    /// `cRZCmdLine(cIGZCmdLine const&)` (§1.2): the source's text and arguments, the text is not
    /// split again. The new object's count starts at 0.
    pub fn from_parts<'a, I>(text: &[u8], args: I) -> Self
    where
        I: IntoIterator<Item = &'a [u8]>,
    {
        CmdLine {
            args: args.into_iter().map(|a| a.to_vec()).collect(),
            text: text.to_vec(),
            ref_count: 0,
        }
    }

    /// The copy constructor applied to another `CmdLine` (through its slots 3, 4 and 5).
    pub fn copy_of(other: &CmdLine) -> Self {
        Self::from_parts(&other.text, other.args.iter().map(|a| a.as_slice()))
    }

    /// The arguments (read-only view).
    pub fn args(&self) -> &[Vec<u8>] {
        &self.args
    }

    // ---- slots 0-2: reference counting -------------------------------------------------------

    /// Slot 0: IIDs 1 and `cIGZCmdLine` succeed and AddRef; anything else fails.
    pub fn query_interface(&mut self, iid: u32) -> bool {
        if iid == IID_IGZUNKNOWN || iid == IID_IGZCMDLINE {
            self.add_ref();
            true
        } else {
            false
        }
    }

    /// Slot 1: increments and returns the count.
    pub fn add_ref(&mut self) -> u32 {
        self.ref_count = self.ref_count.wrapping_add(1);
        self.ref_count
    }

    /// Slot 2: above 1, decrements and returns the new count; otherwise returns 0 and the caller
    /// must destroy the object (slot 15).
    pub fn release(&mut self) -> u32 {
        if self.ref_count > 1 {
            self.ref_count -= 1;
            self.ref_count
        } else {
            0
        }
    }

    /// The current reference count (starts at 0, §1.1).
    pub fn ref_count(&self) -> u32 {
        self.ref_count
    }

    // ---- slots 3-7: arguments, text, find ----------------------------------------------------

    /// Slot 3: the argument count.
    pub fn argc(&self) -> u32 {
        self.args.len() as u32
    }

    /// Slot 4: argument `index`, or the empty string when `index >= n`. A negative index is
    /// unchecked in the original (reads before the list); here it is an error.
    pub fn argv(&self, index: i32) -> Result<&[u8], CmdLineError> {
        if index < 0 {
            return Err(CmdLineError::NegativeIndex(index));
        }
        Ok(self.args.get(index as usize).map_or(&[][..], |a| a.as_slice()))
    }

    /// Slot 5: the command-line text (rebuilt by the join after an insert or erase).
    pub fn get_command_line(&self) -> &[u8] {
        &self.text
    }

    /// Slot 6: the first index whose argument equals `s` (any case when not case sensitive); −1.
    pub fn find(&self, s: &[u8], case_sensitive: bool) -> i32 {
        let key = if case_sensitive { s.to_vec() } else { lower(s) };
        for (i, a) in self.args.iter().enumerate() {
            let eq = if case_sensitive {
                a.as_slice() == key.as_slice()
            } else {
                lower(a) == key
            };
            if eq {
                return i as i32;
            }
        }
        -1
    }

    /// Slot 7: the first index whose argument contains `s`; −1. An empty `s` is found in the first
    /// non-empty argument (an empty argument never matches).
    pub fn find_sub_string(&self, s: &[u8], case_sensitive: bool) -> i32 {
        let key = if case_sensitive { s.to_vec() } else { lower(s) };
        for (i, a) in self.args.iter().enumerate() {
            if a.is_empty() {
                continue;
            }
            let hay = if case_sensitive { a.clone() } else { lower(a) };
            if key.is_empty() || hay.windows(key.len()).any(|w| w == key.as_slice()) {
                return i as i32;
            }
        }
        -1
    }

    // ---- slots 8-12: switches ----------------------------------------------------------------

    /// Slot 8: the `-name:value` form (§4.1). On a match writes `value` and returns true; otherwise
    /// returns false and leaves `value` alone.
    pub fn is_switch_present_value(&self, name: &[u8], value: &mut Vec<u8>, unquote: bool) -> bool {
        if name.is_empty() {
            return false;
        }
        let ln = name.len();
        let lname = lower(name);
        for a in &self.args {
            if !is_switch(a) || a.len() < ln + 3 || a[ln + 1] != b':' {
                continue;
            }
            if lower(&a[1..=ln]) != lname {
                continue;
            }
            let mut v = a[ln + 2..].to_vec();
            let is_q = |b: Option<&u8>| matches!(b, Some(b'"') | Some(b'\''));
            if unquote && is_q(v.first()) && is_q(v.last()) {
                v.remove(0);
                // A one-quote value is empty now: the original throws out_of_range on the second
                // removal (§4.1 rule 4); *[choice]* an empty value.
                v.pop();
            }
            *value = v;
            return true;
        }
        false
    }

    /// Slot 9: `GetIndexOfSwitch(name, 0) >= 0`, else slot 8 with a scratch value and unquote true.
    pub fn is_switch_present(&self, name: &[u8]) -> bool {
        if self.get_index_of_switch(name, 0) >= 0 {
            return true;
        }
        let mut scratch = Vec::new();
        self.is_switch_present_value(name, &mut scratch, true)
    }

    /// Slot 10: the `-cvalue` form (§4.2). A bare two-character `-c` returns true without writing
    /// `value`.
    pub fn is_switch_present_char(&self, c: u8, value: &mut Vec<u8>, unquote: bool) -> bool {
        for a in &self.args {
            if !is_switch(a) || a.len() < 2 || !a[1].eq_ignore_ascii_case(&c) {
                continue;
            }
            if a.len() == 2 {
                return true;
            }
            let n = a.len();
            *value = if unquote && n >= 4 && a[2] == b'"' && a[n - 1] == b'"' {
                a[3..n - 1].to_vec()
            } else {
                a[2..].to_vec()
            };
            return true;
        }
        false
    }

    /// Slot 11: the bare `-name` form (§4.3), from `max(start, 0)`; −1.
    ///
    /// An empty `name` is not special-cased in §4.3; literally it matches a lone prefix character
    /// (`-`), length 1 (question CL2).
    pub fn get_index_of_switch(&self, name: &[u8], start: i32) -> i32 {
        let lname = lower(name);
        self.scan_from(start, |a| {
            is_switch(a) && a.len() == name.len() + 1 && lower(&a[1..]) == lname
        })
    }

    /// Slot 12: the bare `-c` form (§4.3), from `max(start, 0)`; −1.
    pub fn get_index_of_switch_char(&self, c: u8, start: i32) -> i32 {
        self.scan_from(start, |a| is_switch(a) && a.len() == 2 && a[1].eq_ignore_ascii_case(&c))
    }

    fn scan_from(&self, start: i32, pred: impl Fn(&[u8]) -> bool) -> i32 {
        let s = start.max(0) as usize;
        if s >= self.args.len() {
            return -1;
        }
        self.args[s..]
            .iter()
            .position(|a| pred(a))
            .map_or(-1, |p| (s + p) as i32)
    }

    // ---- slots 13-16: edit, destroy, argument --------------------------------------------------

    /// Slot 13: inserts `s` at `index` clamped to `0 ..= n` (0 when the list is empty or the index is
    /// negative), then joins (§2.2). Always succeeds (true in the original).
    pub fn insert_argument(&mut self, s: &[u8], index: i32) {
        let n = self.args.len();
        let at = if n == 0 || index < 0 {
            0
        } else {
            (index as usize).min(n)
        };
        self.args.insert(at, s.to_vec());
        self.text = join(&self.args);
    }

    /// Slot 14: `index >= n` becomes `n − 1`; removes that argument and joins. The original removes
    /// before the list for a negative index or an empty list; here those are errors and nothing
    /// changes.
    pub fn erase_argument(&mut self, index: i32) -> Result<(), CmdLineError> {
        let n = self.args.len();
        if n == 0 {
            return Err(CmdLineError::EraseFromEmpty);
        }
        if index < 0 {
            return Err(CmdLineError::NegativeIndex(index));
        }
        let at = (index as usize).min(n - 1);
        self.args.remove(at);
        self.text = join(&self.args);
        Ok(())
    }

    // Slot 15 (deleting destructor) is Drop.

    /// Slot 16: argument `index` as a string, or the empty string; same unchecked negative index as
    /// slot 4 in the original (an error here). Whether plugins can reach it is question CL1.
    pub fn get_argument(&self, index: i32) -> Result<&[u8], CmdLineError> {
        self.argv(index)
    }
}

#[cfg(test)]
mod tests;
