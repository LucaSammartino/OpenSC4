// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cRZOFStream`, the file sink (§4). The `ofstream` is modelled here (its
//! state bits, stickiness, flush) over a typed file provider ([`OfFile`]).

use super::c_str;
use super::com::{StreamCore, IID_GZ_OSTREAM};
use super::ostream::{base_set_int, GzOStream};
use super::text::{format_g, text_set};

/// The STL stream state bits.
pub const BADBIT: u8 = 1;
pub const EOFBIT: u8 = 2;
pub const FAILBIT: u8 = 4;

/// The open mode (§4.1): always `out`; `binary` when asked; `app` when
/// appending, else `trunc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OfOpenMode {
    pub binary: bool,
    pub append: bool,
}

/// An open file as the `ofstream`'s buffer sees it.
pub trait OfFile {
    /// Transfers `p`; returns how many bytes were taken (fewer than `p.len()`
    /// is a partial transfer, which sets badbit).
    fn write(&mut self, p: &[u8]) -> usize;
    /// Pushes buffered bytes to the destination; false is a failed sync.
    fn sync(&mut self) -> bool;
}

/// Opens the file for an `RzOFStream`; `None` is a failed open (failbit).
pub trait OfFileOpener {
    type File: OfFile;
    fn open(&mut self, path: &[u8], mode: OfOpenMode) -> Option<Self::File>;
}

/// `cRZOFStream` (§4).
pub struct RzOFStream<F: OfFile> {
    core: StreamCore,
    file: Option<F>,
    state: u8,
    binary: bool,
    auto_flush: bool,
    /// The C runtime's decimal point for text-mode floats *[compat-limit]*.
    pub decimal_point: u8,
}

impl<F: OfFile> RzOFStream<F> {
    /// `cRZOFStream(path, binary, littleEndian, autoFlush, append)` (§4.1).
    /// Both game constructors (`cIGZString const&` and `char const*`) read the
    /// path as a C string, so `path` stops at its first NUL. The `ofstream`
    /// allocation failure of `+0x10` has no Rust counterpart (an allocation
    /// failure aborts) and is not modelled.
    pub fn open<O>(
        opener: &mut O,
        path: &[u8],
        binary: bool,
        little_endian: bool,
        auto_flush: bool,
        append: bool,
    ) -> Self
    where
        O: OfFileOpener<File = F>,
    {
        let file = opener.open(c_str(path), OfOpenMode { binary, append });
        let state = if file.is_some() { 0 } else { FAILBIT };
        RzOFStream {
            core: StreamCore::new(little_endian, IID_GZ_OSTREAM),
            file,
            state,
            binary,
            auto_flush,
            decimal_point: b'.',
        }
    }

    pub fn is_binary(&self) -> bool {
        self.binary
    }

    /// The `ofstream`'s state bits.
    pub fn state(&self) -> u8 {
        self.state
    }

    pub fn file(&self) -> Option<&F> {
        self.file.as_ref()
    }

    /// `ofstream.write(p, n)` / `ofstream << s`: refused with badbit once the
    /// state is not good (sticky); a partial transfer sets badbit. True when
    /// the state is good afterwards.
    fn put(&mut self, p: &[u8]) -> bool {
        if self.state != 0 {
            self.state |= BADBIT;
            return false;
        }
        match self.file.as_mut() {
            Some(f) => {
                if f.write(p) < p.len() {
                    self.state |= BADBIT;
                }
            }
            None => self.state |= BADBIT,
        }
        self.state == 0
    }

    fn after_text(&mut self, ok: bool) -> bool {
        if self.auto_flush {
            self.flush();
        }
        ok
    }
}

impl<F: OfFile> GzOStream for RzOFStream<F> {
    fn core(&self) -> &StreamCore {
        &self.core
    }
    fn core_mut(&mut self) -> &mut StreamCore {
        &mut self.core
    }
    /// `ofstream.flush()`: a failed sync sets badbit. Without an open file
    /// there is nothing to sync and nothing changes (OS4).
    fn flush(&mut self) {
        if let Some(f) = self.file.as_mut() {
            if !f.sync() {
                self.state |= BADBIT;
            }
        }
    }
    fn set_sint8(&mut self, v: i8) -> bool {
        if self.binary {
            self.set_void(&[v as u8])
        } else {
            text_set(self, v.to_string())
        }
    }
    fn set_uint8(&mut self, v: u8) -> bool {
        if self.binary {
            self.set_void(&[v])
        } else {
            text_set(self, v.to_string())
        }
    }
    fn set_sint16(&mut self, v: i16) -> bool {
        if self.binary {
            base_set_int(self, v.to_le_bytes())
        } else {
            text_set(self, v.to_string())
        }
    }
    fn set_uint16(&mut self, v: u16) -> bool {
        if self.binary {
            base_set_int(self, v.to_le_bytes())
        } else {
            text_set(self, v.to_string())
        }
    }
    fn set_sint32(&mut self, v: i32) -> bool {
        if self.binary {
            base_set_int(self, v.to_le_bytes())
        } else {
            text_set(self, v.to_string())
        }
    }
    fn set_uint32(&mut self, v: u32) -> bool {
        if self.binary {
            base_set_int(self, v.to_le_bytes())
        } else {
            text_set(self, v.to_string())
        }
    }
    fn set_sint64(&mut self, v: i64) -> bool {
        if self.binary {
            base_set_int(self, v.to_le_bytes())
        } else {
            text_set(self, v.to_string())
        }
    }
    fn set_uint64(&mut self, v: u64) -> bool {
        if self.binary {
            base_set_int(self, v.to_le_bytes())
        } else {
            text_set(self, v.to_string())
        }
    }
    fn set_float32(&mut self, f: f32) -> bool {
        if self.binary {
            self.set_void(&f.to_le_bytes())
        } else {
            let dp = self.decimal_point;
            text_set(self, format_g(f as f64, dp))
        }
    }
    fn set_float64(&mut self, d: f64) -> bool {
        if self.binary {
            self.set_void(&d.to_le_bytes())
        } else {
            let dp = self.decimal_point;
            text_set(self, format_g(d, dp))
        }
    }
    /// Binary: `SetVoid(s, strlen(s))`, no length and no terminator; text:
    /// `ofstream << s`, the bytes as given (no CR LF translation here, OS2).
    /// The result is the state after the write; `autoFlush` then flushes
    /// without changing the result.
    fn set_rz_char_str(&mut self, s: &[u8]) -> bool {
        let s = c_str(s);
        let ok = if self.binary { self.set_void(s) } else { self.put(s) };
        self.after_text(ok)
    }
    /// `cRZTextOStream`'s: `SetRZCharStr(s.ToChar())`, stopping at a NUL.
    fn set_gz_str(&mut self, s: &[u8]) -> bool {
        self.set_rz_char_str(c_str(s))
    }
    /// `ofstream.write(p, n)` in both modes.
    fn set_void(&mut self, p: &[u8]) -> bool {
        self.put(p)
    }
    /// Any of bad, fail or eof.
    fn get_error(&self) -> bool {
        self.state & (BADBIT | FAILBIT | EOFBIT) != 0
    }
}

/// The host file system: `std::fs` with a write buffer (the `ofstream`'s
/// own buffering); dropping the file flushes and closes it.
#[derive(Default, Clone, Copy, Debug)]
pub struct StdFileOpener;

pub struct StdFile(std::io::BufWriter<std::fs::File>);

impl OfFile for StdFile {
    fn write(&mut self, p: &[u8]) -> usize {
        use std::io::Write;
        match self.0.write_all(p) {
            Ok(()) => p.len(),
            Err(_) => 0,
        }
    }
    fn sync(&mut self) -> bool {
        use std::io::Write;
        self.0.flush().is_ok()
    }
}

impl OfFileOpener for StdFileOpener {
    type File = StdFile;
    /// The path bytes are taken as UTF-8 (lossy); text mode writes the bytes
    /// unchanged (OS2).
    fn open(&mut self, path: &[u8], mode: OfOpenMode) -> Option<StdFile> {
        let path = String::from_utf8_lossy(path).into_owned();
        let mut o = std::fs::OpenOptions::new();
        o.create(true);
        if mode.append {
            o.append(true);
        } else {
            o.write(true).truncate(true);
        }
        o.open(path).ok().map(|f| StdFile(std::io::BufWriter::new(f)))
    }
}
