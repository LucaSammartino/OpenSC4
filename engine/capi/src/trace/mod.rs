// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in record/replay at the C ABI host boundary. Tooling, not simulation.
//! SPEC-GAP: trace transport is a versioned little-endian event stream;
//! opaque host tokens retain their numeric identity and are never dereferenced.
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{c_char, c_void},
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
mod hash;
mod replay;
mod wire;
pub use replay::run;
use wire::*;
const MAGIC: &[u8] = b"FC4TRACE\x01\0\0\0";
const MAX_EVENT: usize = 128 * 1024 * 1024;
const CALLBACK: u8 = 1;
const OP: u8 = 2;
const RESULT: u8 = 3;
const END: u8 = 4;

#[derive(Clone)]
struct Config {
    dir: PathBuf,
    scenario: String,
}
static CONFIG: OnceLock<Mutex<Option<Config>>> = OnceLock::new();
static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
thread_local! { static SESSIONS:RefCell<HashMap<&'static str,Session>>=RefCell::new(HashMap::new()); }
#[cfg(test)]
thread_local! { static TEST_CONFIG:RefCell<Option<Config>>=const{RefCell::new(None)}; }
struct Session {
    mode: Mode,
    misses: Vec<String>,
    recorded_mismatches: u64,
    comparisons: u64,
    incomplete: u64,
}
enum Mode {
    Record {
        writer: BufWriter<File>,
        temp: PathBuf,
        config: Config,
        failed: bool,
    },
    Replay {
        reader: BufReader<File>,
    },
}
fn config() -> Option<Config> {
    #[cfg(test)]
    if let Some(c) = TEST_CONFIG.with(|c| c.borrow().clone()) {
        return Some(c);
    }
    CONFIG.get().and_then(|c| c.lock().ok()?.clone())
}
fn configured() -> bool {
    config().is_some()
}
fn active(key: &str) -> bool {
    SESSIONS.with(|s| s.borrow().contains_key(key))
}
fn replaying(key: &str) -> bool {
    SESSIONS.with(|s| {
        s.borrow()
            .get(key)
            .is_some_and(|s| matches!(s.mode, Mode::Replay { .. }))
    })
}
fn miss(key: &str, error: String) {
    SESSIONS.with(|s| {
        if let Some(s) = s.borrow_mut().get_mut(key) {
            if s.misses.len() < 16 {
                s.misses.push(error);
            }
        }
    });
}
fn decode_callback<T: Default>(key: &str, v: Result<T, String>) -> T {
    v.unwrap_or_else(|e| {
        miss(key, e);
        T::default()
    })
}
fn event(kind: u8, name: &str, data: &[u8]) -> Vec<u8> {
    let mut b = vec![kind];
    name.to_owned().put(&mut b);
    b.extend_from_slice(data);
    b
}
fn write_event(s: &mut Session, b: &[u8]) {
    if let Mode::Record { writer, failed, .. } = &mut s.mode {
        if b.len() > MAX_EVENT
            || writer
                .write_all(&(b.len() as u32).to_le_bytes())
                .and_then(|_| writer.write_all(b))
                .is_err()
        {
            *failed = true;
        }
    }
}
fn read_event(reader: &mut impl Read) -> Result<Vec<u8>, String> {
    let mut len = [0u8; 4];
    reader.read_exact(&mut len).map_err(|e| format!("trace frame: {e}"))?;
    let n = u32::from_le_bytes(len) as usize;
    if n > MAX_EVENT {
        return Err("oversized trace event".into());
    }
    let mut b = vec![0; n];
    reader.read_exact(&mut b).map_err(|e| format!("trace payload: {e}"))?;
    Ok(b)
}
fn unpack(b: &[u8]) -> Result<(u8, String, &[u8]), String> {
    let mut r = Reader(b);
    let kind = u8::get(&mut r)?;
    let name = String::get(&mut r)?;
    Ok((kind, name, r.0))
}
fn put_event(key: &str, kind: u8, name: &str, data: &[u8]) {
    SESSIONS.with(|sessions| {
        if let Some(s) = sessions.borrow_mut().get_mut(key) {
            write_event(s, &event(kind, name, data));
        }
    });
}
fn exchange(key: &str, name: &str, input: Vec<u8>, call: impl FnOnce() -> Vec<u8>) -> Vec<u8> {
    if replaying(key) {
        let answer = SESSIONS.with(|sessions| {
            let mut sessions = sessions.borrow_mut();
            let s = sessions.get_mut(key).ok_or("no replay")?;
            let Mode::Replay { reader } = &mut s.mode else {
                return Err("not replaying".to_owned());
            };
            let b = read_event(reader)?;
            let (kind, expected, data) = unpack(&b)?;
            let mut r = Reader(data);
            let args = Vec::<u8>::get(&mut r)?;
            let answer = Vec::<u8>::get(&mut r)?;
            r.end()?;
            if kind != CALLBACK || expected != name || args != input {
                return Err(format!(
                    "replay miss: expected {expected}, requested {name}; arguments_equal={}",
                    args == input
                ));
            }
            Ok::<_, String>(answer)
        });
        match answer {
            Ok(a) => a,
            Err(e) => {
                miss(key, e);
                Vec::new()
            }
        }
    } else {
        let answer = call();
        let mut b = packed(&input);
        answer.put(&mut b);
        put_event(key, CALLBACK, name, &b);
        answer
    }
}
fn op(key: &str, name: &str, data: &[u8]) {
    if !replaying(key) {
        put_event(key, OP, name, data);
    }
}
fn result<T: Wire>(key: &str, name: &str, value: &T) {
    let b = packed(value);
    if replaying(key) {
        let got = SESSIONS.with(|sessions| {
            let mut sessions = sessions.borrow_mut();
            let s = sessions.get_mut(key).ok_or("no replay")?;
            let Mode::Replay { reader } = &mut s.mode else {
                return Err("not replaying".to_owned());
            };
            read_event(reader)
        });
        match got.and_then(|got| {
            let (kind, n, d) = unpack(&got)?;
            if kind == RESULT && n == name && d == b {
                Ok(())
            } else {
                Err(format!("output differs: {name}"))
            }
        }) {
            Ok(()) => (),
            Err(e) => miss(key, e),
        }
    } else {
        put_event(key, RESULT, name, &b);
    }
}
fn start(key: &'static str) {
    if replaying(key) {
        return;
    }
    finish(key);
    let Some(config) = config() else {
        return;
    };
    let opened = (|| -> std::io::Result<_> {
        fs::create_dir_all(&config.dir)?;
        let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let temp = config
            .dir
            .join(format!(".{}-{key}-{serial}.partial", std::process::id()));
        let mut writer = BufWriter::new(OpenOptions::new().create_new(true).write(true).open(&temp)?);
        writer.write_all(MAGIC)?;
        Ok((writer, temp))
    })();
    match opened {
        Ok((writer, temp)) => SESSIONS.with(|s| {
            s.borrow_mut().insert(
                key,
                Session {
                    mode: Mode::Record {
                        writer,
                        temp,
                        config,
                        failed: false,
                    },
                    misses: Vec::new(),
                    recorded_mismatches: 0,
                    comparisons: 0,
                    incomplete: 0,
                },
            );
        }),
        Err(e) => eprintln!("fc4 trace: cannot record {key}: {e}"),
    }
}
fn finish(key: &str) {
    if replaying(key) {
        return;
    }
    let Some(mut s) = SESSIONS.with(|sessions| sessions.borrow_mut().remove(key)) else {
        return;
    };
    write_event(&mut s, &event(END, "end", &[]));
    if let Mode::Record {
        mut writer,
        temp,
        config,
        failed,
    } = s.mode
    {
        let saved = (|| -> std::io::Result<_> {
            writer.flush()?;
            drop(writer);
            if failed {
                return Err(std::io::Error::other("trace write failed"));
            }
            let sha = hash::sha256(BufReader::new(File::open(&temp)?))?;
            let dest = config.dir.join(&sha);
            if dest.exists() {
                fs::remove_file(&temp)?;
            } else {
                fs::rename(&temp, &dest)?;
            }
            // Serialize manifest appends across shadow keys/threads in this process.
            let _lock = CONFIG
                .get_or_init(|| Mutex::new(None))
                .lock()
                .map_err(|_| std::io::Error::other("trace config poisoned"))?;
            let mut manifest = OpenOptions::new()
                .create(true)
                .append(true)
                .open(config.dir.join("manifest.tsv"))?;
            writeln!(
                manifest,
                "{}\t{}_shadow\t{}\t{}\t{}\t{}",
                config.scenario, key, sha, s.comparisons, s.recorded_mismatches, s.incomplete
            )?;
            Ok(sha)
        })();
        match saved {
            Ok(sha) => eprintln!("fc4 trace: {key} {sha}"),
            Err(e) => eprintln!("fc4 trace: incomplete {key}: {e}"),
        }
    }
}
fn compared(key: &str, r: i32) {
    SESSIONS.with(|s| {
        if let Some(s) = s.borrow_mut().get_mut(key) {
            if r < 0 {
                s.incomplete += 1;
            } else {
                s.comparisons += 1;
                s.recorded_mismatches += r as u64;
            }
        }
    });
}

/// Configure once before shadow creation; empty directory disables recording.
/// The paths and scenario are UTF-8 byte slices, not nul-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn fc4_trace_configure(
    dir: *const u8,
    dir_len: usize,
    scenario: *const u8,
    scenario_len: usize,
) -> bool {
    crate::guard(false, || {
        if dir.is_null() || scenario.is_null() {
            return false;
        }
        let Ok(dir) = std::str::from_utf8(std::slice::from_raw_parts(dir, dir_len)) else {
            return false;
        };
        let Ok(scenario) = std::str::from_utf8(std::slice::from_raw_parts(scenario, scenario_len)) else {
            return false;
        };
        if scenario.is_empty() || scenario.chars().any(|c| c.is_control()) {
            return false;
        }
        let Ok(mut config) = CONFIG.get_or_init(|| Mutex::new(None)).lock() else {
            return false;
        };
        *config = if dir.is_empty() {
            None
        } else {
            Some(Config {
                dir: PathBuf::from(dir),
                scenario: scenario.to_owned(),
            })
        };
        true
    })
}

pub(crate) fn power_create(
    h: crate::power::fc4_power_host,
    o: Option<crate::power_shadow::fc4_power_shadow_options>,
) -> (
    crate::power::fc4_power_host,
    Option<crate::power_shadow::fc4_power_shadow_options>,
) {
    if !configured() && !replaying("power") {
        return (h, o);
    }
    start("power");
    let mut b = Vec::new();
    wire::put_power(&h, &mut b);
    o.is_some().put(&mut b);
    if let Some(o) = o {
        o.max_cells.put(&mut b);
        o.max_lines.put(&mut b);
        o.flags.put(&mut b);
        o.connection_visited.is_some().put(&mut b);
        VISITED.with(|f| *f.borrow_mut() = o.connection_visited);
    }
    op("power", "create", &b);
    (
        wire::wrap_power(h),
        o.map(|mut o| {
            o.connection_visited = o.connection_visited.map(|_| visited as _);
            o
        }),
    )
}
thread_local! {static VISITED:RefCell<Option<unsafe extern "C" fn(*mut c_void,u64)->bool>>=const{RefCell::new(None)};}
unsafe extern "C" fn visited(user: *mut c_void, token: u64) -> bool {
    let answer = exchange("power", "power.connection_visited", packed(&token), || {
        packed(&VISITED.with(|f| f.borrow().unwrap()(user, token)))
    });
    decode_callback("power", bool::get(&mut Reader(&answer)))
}
pub(crate) fn pollution_create(
    h: crate::pollution::fc4_pollution_host,
    max_cells: u32,
) -> crate::pollution::fc4_pollution_host {
    if !configured() && !replaying("pollution") {
        return h;
    }
    start("pollution");
    let mut b = Vec::new();
    wire::put_pollution(&h, &mut b);
    max_cells.put(&mut b);
    op("pollution", "create", &b);
    wire::wrap_pollution(h)
}

unsafe fn put_power_capture(p: *const crate::power_shadow::fc4_power_capture, b: &mut Vec<u8>) {
    p.as_ref().is_some().put(b);
    if let Some(c) = p.as_ref() {
        c.ok.put(b);
        c.len.put(b);
        put_ptr(c.bytes, c.len, b);
        c.ref_count.put(b);
        put_ptr(c.refs, c.ref_count, b);
    }
}
unsafe fn put_pollution_capture(p: *const crate::pollution_shadow::fc4_pollution_capture, b: &mut Vec<u8>) {
    p.as_ref().is_some().put(b);
    if let Some(c) = p.as_ref() {
        c.ok.put(b);
        c.len.put(b);
        put_ptr(c.bytes, c.len, b);
        c.ref_count.put(b);
        put_ptr(c.refs, c.ref_count, b);
    }
}
pub(crate) unsafe fn power_step(
    name: &str,
    msg: u32,
    julian: i32,
    c: *const crate::power_shadow::fc4_power_capture,
    n: *const crate::power_shadow::fc4_power_shadow_counts,
) {
    if !active("power") {
        return;
    }
    let mut b = packed(&msg);
    julian.put(&mut b);
    put_power_capture(c, &mut b);
    n.as_ref().copied().put(&mut b);
    op("power", name, &b);
}
pub(crate) unsafe fn pollution_step(
    name: &str,
    msg: u32,
    julian: i32,
    c: *const crate::pollution_shadow::fc4_pollution_capture,
) {
    if !active("pollution") {
        return;
    }
    let mut b = packed(&msg);
    julian.put(&mut b);
    put_pollution_capture(c, &mut b);
    op("pollution", name, &b);
}
pub(crate) fn step_result(key: &str, name: &str, r: i32, lines: &[String]) {
    if !active(key) {
        return;
    }
    let mut b = packed(&r);
    lines.to_vec().put(&mut b);
    result(key, name, &b);
    if name == "finish" {
        compared(key, r);
    }
}
pub(crate) fn power_message(m: &crate::power::fc4_power_message) {
    op("power", "message", &packed(m));
}
pub(crate) fn power_wants(msg: u32, event: bool) {
    let mut b = packed(&msg);
    event.put(&mut b);
    op("power", "wants", &b);
}
pub(crate) fn bool_result(key: &str, name: &str, r: bool) {
    result(key, name, &r);
}
pub(crate) fn will_destroy(key: &str) {
    op(key, "destroy", &[]);
}
pub(crate) fn destroy(key: &str) {
    finish(key);
}

pub(crate) unsafe fn budget_create(
    h: crate::budget::fc4_budget_host,
    tuning: u64,
    fix: bool,
    difficulty: u32,
    rates: *const f32,
) -> crate::budget::fc4_budget_host {
    if !configured() && !replaying("budget") {
        return h;
    }
    start("budget");
    let mut b = Vec::new();
    wire::put_budget(&h, &mut b);
    tuning.put(&mut b);
    fix.put(&mut b);
    difficulty.put(&mut b);
    put_ptr(rates, 12, &mut b);
    op("budget", "create", &b);
    wire::wrap_budget(h)
}

/// The original values and observability used by FC4BudgetShadow::Report.
#[no_mangle]
pub unsafe extern "C" fn fc4_trace_budget_expected(
    year: i32,
    month: i32,
    day: i32,
    wa: *const i64,
    tax: *const i64,
    total: i64,
    income_tax: i64,
    compare_income: bool,
    expense_mismatch: bool,
    recorded_mismatches: i32,
) {
    crate::guard((), || {
        if !active("budget") || wa.is_null() || tax.is_null() {
            return;
        }
        let mut b = Vec::new();
        year.put(&mut b);
        month.put(&mut b);
        day.put(&mut b);
        for p in [wa, tax] {
            for &v in std::slice::from_raw_parts(p, 12) {
                v.put(&mut b);
            }
        }
        total.put(&mut b);
        income_tax.put(&mut b);
        compare_income.put(&mut b);
        expense_mismatch.put(&mut b);
        recorded_mismatches.put(&mut b);
        op("budget", "expected", &b);
        compared("budget", recorded_mismatches);
        finish("budget");
    });
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) fn test_record(scenario: &str, f: impl FnOnce()) -> PathBuf {
    let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../build-runs/trace-tests/{}-{serial}-{scenario}",
        std::process::id()
    ));
    TEST_CONFIG.with(|c| {
        *c.borrow_mut() = Some(Config {
            dir: dir.clone(),
            scenario: scenario.into(),
        })
    });
    f();
    TEST_CONFIG.with(|c| *c.borrow_mut() = None);
    dir
}
