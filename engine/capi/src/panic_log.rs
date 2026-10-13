// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The panic report (tooling): a panic hook that hands the panic's message,
//! location, thread and a backtrace to a host sink before the panic goes on
//! (release builds abort with `panic = "abort"`, which ends the process with a
//! fail-fast and no other trace, in-game run 23). The previous hook runs
//! afterwards, so tests keep their usual output.

use std::ffi::c_void;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;

/// Receives one UTF-8 text block (lines separated by `\n`, no NUL).
pub type fc4_log_sink = Option<unsafe extern "C" fn(ctx: *mut c_void, text: *const u8, len: usize)>;

static SINK: AtomicUsize = AtomicUsize::new(0);
static CTX: AtomicUsize = AtomicUsize::new(0);
static HOOK: Once = Once::new();

/// The most backtrace lines written.
const MAX_BACKTRACE_LINES: usize = 80;

fn current_sink() -> fc4_log_sink {
    let f = SINK.load(Ordering::Acquire);
    if f == 0 {
        None
    } else {
        // SAFETY: only ever stored from a valid `fc4_log_sink` function pointer.
        Some(unsafe { std::mem::transmute::<usize, unsafe extern "C" fn(*mut c_void, *const u8, usize)>(f) })
    }
}

/// The report for one panic.
#[allow(deprecated)] // PanicInfo: the name before Rust 1.81 (rust-version 1.75)
pub(crate) fn report(info: &std::panic::PanicInfo<'_>, backtrace: Option<&str>) -> String {
    let payload = info.payload();
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("(non-string payload)");
    let location = info.location().map_or_else(
        || "unknown location".to_string(),
        |l| format!("{}:{}:{}", l.file(), l.line(), l.column()),
    );
    let thread = std::thread::current();
    let mut t = format!(
        "Rust PANIC in FreeCity4 core: '{message}' at {location} (thread {}); release builds abort now (fail-fast)",
        thread.name().unwrap_or("unnamed")
    );
    if let Some(bt) = backtrace {
        let _ = write!(t, "\nRust backtrace:");
        for line in bt.lines().take(MAX_BACKTRACE_LINES) {
            let _ = write!(t, "\n  {}", line.trim_end());
        }
    }
    t
}

/// Sets (or, with a null sink, clears) where panic reports go, and installs
/// the panic hook on the first call. Any thread; the sink must stay valid
/// until it is replaced.
#[no_mangle]
pub unsafe extern "C" fn fc4_set_panic_log(sink: fc4_log_sink, ctx: *mut c_void) {
    CTX.store(ctx as usize, Ordering::Release);
    SINK.store(sink.map_or(0, |f| f as usize), Ordering::Release);
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if let Some(f) = current_sink() {
                let bt = std::backtrace::Backtrace::force_capture().to_string();
                let text = report(info, Some(&bt));
                // SAFETY: the host's sink and context, set together above.
                unsafe { f(CTX.load(Ordering::Acquire) as *mut c_void, text.as_ptr(), text.len()) };
            }
            previous(info);
        }));
    });
}

/// Tooling: a deliberate panic (tests of the report). Release builds abort;
/// builds that unwind return.
#[no_mangle]
pub extern "C" fn fc4_debug_panic() {
    crate::guard((), || {
        panic!("fc4_debug_panic: a deliberate panic (test of the panic report)")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static GOT: Mutex<Vec<String>> = Mutex::new(Vec::new());

    unsafe extern "C" fn collect(_ctx: *mut c_void, text: *const u8, len: usize) {
        let s = String::from_utf8_lossy(std::slice::from_raw_parts(text, len)).into_owned();
        GOT.lock().unwrap().push(s);
    }

    #[test]
    fn a_panic_reaches_the_sink_with_its_location() {
        unsafe { fc4_set_panic_log(Some(collect), std::ptr::null_mut()) };
        let r = std::panic::catch_unwind(|| {
            let v: Vec<u32> = Vec::new();
            let i = v.len() + 3;
            v[i]
        });
        assert!(r.is_err());
        unsafe { fc4_set_panic_log(None, std::ptr::null_mut()) };
        let got = GOT.lock().unwrap();
        let t = got
            .iter()
            .find(|t| t.contains("index out of bounds"))
            .expect("the report");
        assert!(
            t.starts_with("Rust PANIC in FreeCity4 core: 'index out of bounds"),
            "{t}"
        );
        assert!(t.contains("panic_log.rs:"), "{t}");
        assert!(t.contains("\nRust backtrace:"), "{t}");
    }
}
