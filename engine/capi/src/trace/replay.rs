// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::{budget::*, pollution_shadow::*, power_shadow::*};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

fn values<T: Wire>(r: &mut Reader<'_>, n: usize) -> Result<Option<Vec<T>>, String> {
    if n > MAX_EVENT || n > r.0.len() {
        return Err("invalid capture size".into());
    }
    if !bool::get(r)? {
        return Ok(None);
    }
    Ok(Some((0..n).map(|_| T::get(r)).collect::<Result<_, _>>()?))
}
struct Capture<T> {
    ok: u32,
    bytes: Option<Vec<u8>>,
    refs: Option<Vec<T>>,
    len: usize,
    count: usize,
}
fn capture<T: Wire>(r: &mut Reader<'_>) -> Result<Option<Capture<T>>, String> {
    if !bool::get(r)? {
        return Ok(None);
    }
    let ok = u32::get(r)?;
    let len = usize::get(r)?;
    let bytes = values(r, len)?;
    let count = usize::get(r)?;
    let refs = values(r, count)?;
    Ok(Some(Capture {
        ok,
        bytes,
        refs,
        len,
        count,
    }))
}
fn power_capture(c: &Option<Capture<fc4_power_capture_ref>>) -> Option<fc4_power_capture> {
    c.as_ref().map(|c| fc4_power_capture {
        struct_size: std::mem::size_of::<fc4_power_capture>() as u32,
        ok: c.ok,
        bytes: c.bytes.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
        len: c.len,
        refs: c.refs.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
        ref_count: c.count,
    })
}
fn pollution_capture(c: &Option<Capture<fc4_pollution_capture_ref>>) -> Option<fc4_pollution_capture> {
    c.as_ref().map(|c| fc4_pollution_capture {
        struct_size: std::mem::size_of::<fc4_pollution_capture>() as u32,
        ok: c.ok,
        bytes: c.bytes.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
        len: c.len,
        refs: c.refs.as_ref().map_or(std::ptr::null(), |v| v.as_ptr()),
        ref_count: c.count,
    })
}
fn ptr<T>(v: &Option<T>) -> *const T {
    v.as_ref().map_or(std::ptr::null(), |v| v)
}

#[derive(Clone, Debug)]
struct Verdict {
    equal: bool,
    comparisons: u64,
    mismatches: u64,
    incomplete: u64,
    errors: Vec<String>,
}
impl Wire for Verdict {
    fn put(&self, b: &mut Vec<u8>) {
        self.equal.put(b);
        self.comparisons.put(b);
        self.mismatches.put(b);
        self.incomplete.put(b);
        self.errors.put(b);
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        Ok(Self {
            equal: bool::get(r)?,
            comparisons: u64::get(r)?,
            mismatches: u64::get(r)?,
            incomplete: u64::get(r)?,
            errors: Vec::<String>::get(r)?,
        })
    }
}
unsafe fn replay_file(path: &Path, key: &'static str) -> Result<Verdict, String> {
    let mut reader = BufReader::new(File::open(path).map_err(|e| e.to_string())?);
    let mut magic = vec![0; MAGIC.len()];
    reader.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if magic != MAGIC {
        return Err("unsupported trace version".into());
    }
    SESSIONS.with(|s| {
        s.borrow_mut().insert(
            key,
            Session {
                mode: Mode::Replay { reader },
                misses: Vec::new(),
                recorded_mismatches: 0,
                comparisons: 0,
                incomplete: 0,
            },
        );
    });
    let mut power: *mut fc4_power_shadow = std::ptr::null_mut();
    let mut pollution: *mut fc4_pollution_shadow = std::ptr::null_mut();
    let mut scan: Option<fc4_tax_scan> = None;
    let outcome = (|| -> Result<(), String> {
        loop {
            let b = SESSIONS.with(|s| {
                let mut s = s.borrow_mut();
                let Mode::Replay { reader } = &mut s.get_mut(key).unwrap().mode else {
                    unreachable!()
                };
                read_event(reader)
            })?;
            let (kind, name, data) = unpack(&b)?;
            if kind == END {
                let mut tail = [0u8; 1];
                let n = SESSIONS
                    .with(|s| {
                        let mut s = s.borrow_mut();
                        let Mode::Replay { reader } = &mut s.get_mut(key).unwrap().mode else {
                            unreachable!()
                        };
                        reader.read(&mut tail)
                    })
                    .map_err(|e| e.to_string())?;
                if n != 0 {
                    return Err("bytes after trace end".into());
                }
                break;
            }
            if kind != OP {
                return Err(format!("unconsumed callback/result before {name}"));
            }
            let mut r = Reader(data);
            match (key, name.as_str()) {
                ("power", "create") => {
                    let h = wire::get_power(&mut r)?;
                    let o = if bool::get(&mut r)? {
                        Some(fc4_power_shadow_options {
                            struct_size: std::mem::size_of::<fc4_power_shadow_options>() as u32,
                            max_cells: u32::get(&mut r)?,
                            max_lines: u32::get(&mut r)?,
                            flags: u32::get(&mut r)?,
                            connection_visited: if bool::get(&mut r)? { Some(visited) } else { None },
                        })
                    } else {
                        None
                    };
                    if !power.is_null() {
                        return Err("duplicate power create".into());
                    }
                    power = fc4_power_shadow_create(&h, ptr(&o));
                    if power.is_null() {
                        return Err("power create failed".into());
                    }
                }
                ("pollution", "create") => {
                    let h = wire::get_pollution(&mut r)?;
                    let cells = u32::get(&mut r)?;
                    if !pollution.is_null() {
                        return Err("duplicate pollution create".into());
                    }
                    pollution = fc4_pollution_shadow_create(&h, cells);
                    if pollution.is_null() {
                        return Err("pollution create failed".into());
                    }
                }
                ("power", "begin" | "finish") => {
                    if power.is_null() {
                        return Err("power step before create".into());
                    }
                    let msg = u32::get(&mut r)?;
                    let julian = i32::get(&mut r)?;
                    let c = capture(&mut r)?;
                    let c = power_capture(&c);
                    let counts = Option::<fc4_power_shadow_counts>::get(&mut r)?;
                    if name == "begin" {
                        fc4_power_shadow_begin(power, msg, julian, ptr(&c), ptr(&counts));
                    } else {
                        fc4_power_shadow_finish(power, ptr(&c), ptr(&counts), None, std::ptr::null_mut());
                    }
                }
                ("pollution", "begin" | "finish") => {
                    if pollution.is_null() {
                        return Err("pollution step before create".into());
                    }
                    let msg = u32::get(&mut r)?;
                    let julian = i32::get(&mut r)?;
                    let c = capture(&mut r)?;
                    let c = pollution_capture(&c);
                    if name == "begin" {
                        fc4_pollution_shadow_begin(pollution, msg, julian, ptr(&c));
                    } else {
                        fc4_pollution_shadow_finish(pollution, ptr(&c), None, std::ptr::null_mut());
                    }
                }
                ("power", "message") => {
                    let m = crate::power::fc4_power_message::get(&mut r)?;
                    fc4_power_shadow_message(power, &m);
                }
                ("power", "wants") => {
                    let msg = u32::get(&mut r)?;
                    let event = bool::get(&mut r)?;
                    fc4_power_shadow_wants(power, msg, event);
                }
                ("budget", "create") => {
                    let h = wire::get_budget(&mut r)?;
                    let tuning = u64::get(&mut r)?;
                    let fix = bool::get(&mut r)?;
                    let difficulty = u32::get(&mut r)?;
                    let mut rates = [0f32; 12];
                    get_ptr(rates.as_mut_ptr(), 12, &mut r)?;
                    let mut out = fc4_tax_scan {
                        struct_size: std::mem::size_of::<fc4_tax_scan>() as u32,
                        ..Default::default()
                    };
                    if !fc4_budget_shadow_tax(
                        &h,
                        tuning,
                        fix,
                        difficulty,
                        rates.as_ptr(),
                        None,
                        std::ptr::null_mut(),
                        &mut out,
                    ) {
                        return Err("budget scan failed".into());
                    }
                    scan = Some(out);
                }
                ("budget", "expected") => {
                    let _date = <[i32; 3]>::get(&mut r)?;
                    let wa = <[i64; 12]>::get(&mut r)?;
                    let tax = <[i64; 12]>::get(&mut r)?;
                    let total = i64::get(&mut r)?;
                    let income_tax = i64::get(&mut r)?;
                    let compare_income = bool::get(&mut r)?;
                    let expense_mismatch = bool::get(&mut r)?;
                    let recorded = i32::get(&mut r)?;
                    let scan = scan.as_ref().ok_or("budget observation before scan")?;
                    let mismatch = (0..12)
                        .filter(|&i| wa[i] != scan.weighted_assessed[i] || tax[i] != scan.tax_income[i])
                        .count() as i32
                        + i32::from(total != scan.total_tax)
                        + i32::from(compare_income && income_tax != scan.total_tax)
                        + i32::from(expense_mismatch);
                    compared(key, mismatch);
                    if mismatch != recorded {
                        miss(
                            key,
                            format!("output differs: budget shadow recorded={recorded} replay={mismatch}"),
                        );
                    }
                }
                ("power", "destroy") => {
                    fc4_power_shadow_destroy(power);
                    power = std::ptr::null_mut();
                }
                ("pollution", "destroy") => {
                    fc4_pollution_shadow_destroy(pollution);
                    pollution = std::ptr::null_mut();
                }
                _ => return Err(format!("unsupported trace operation {key}/{name}")),
            }
            r.end()?;
            if SESSIONS.with(|s| !s.borrow().get(key).unwrap().misses.is_empty()) {
                return Err("replay stopped after first divergent operation".into());
            }
        }
        Ok(())
    })();
    // Destroy on failure too, while callback context is still valid.
    if !power.is_null() {
        fc4_power_shadow_destroy(power);
    }
    if !pollution.is_null() {
        fc4_pollution_shadow_destroy(pollution);
    }
    let mut s = SESSIONS.with(|s| s.borrow_mut().remove(key)).unwrap();
    if let Err(e) = outcome {
        s.misses.push(e);
    }
    Ok(Verdict {
        equal: s.misses.is_empty(),
        comparisons: s.comparisons,
        mismatches: s.recorded_mismatches,
        incomplete: s.incomplete,
        errors: s.misses,
    })
}

fn owner(key: &str) -> Result<&'static str, String> {
    match key {
        "budget_shadow" => Ok("budget"),
        "power_shadow" => Ok("power"),
        "pollution_shadow" => Ok("pollution"),
        _ => Err(format!("unsupported replay key: {key}")),
    }
}
#[derive(Clone)]
struct Row {
    scenario: String,
    key: String,
    sha: String,
    comparisons: u64,
    mismatches: u64,
    incomplete: u64,
}
fn rows(manifest: &Path) -> Result<Vec<Row>, String> {
    let text = fs::read_to_string(manifest).map_err(|e| e.to_string())?;
    text.lines()
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .map(|s| {
            let c: Vec<_> = s.split('\t').collect();
            if c.len() != 6 {
                return Err("manifest requires scenario,key,sha,comparisons,mismatches,incomplete TSV columns".into());
            }
            owner(c[1])?;
            if c[2].len() != 64 || !c[2].bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
                return Err("invalid trace SHA-256".into());
            }
            Ok(Row {
                scenario: c[0].to_owned(),
                key: c[1].to_owned(),
                sha: c[2].to_owned(),
                comparisons: c[3].parse().map_err(|_| "comparison count")?,
                mismatches: c[4].parse().map_err(|_| "mismatch count")?,
                incomplete: c[5].parse().map_err(|_| "incomplete count")?,
            })
        })
        .collect()
}

/// Replay every unique trace on the available CPUs; write machine-readable
/// verdicts next to the manifest. A cache hit still verifies the trace digest.
pub fn run(manifest: &Path, no_cache: bool, jobs: usize) -> Result<bool, String> {
    let start = Instant::now();
    let rows = rows(manifest)?;
    if rows.is_empty() {
        return Err("empty replay manifest".into());
    }
    let dir = manifest.parent().unwrap_or(Path::new("."));
    let cache = dir.join("verdict-cache");
    fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let mut unique = HashMap::new();
    for row in &rows {
        if let Some(previous) = unique.insert(row.sha.clone(), row.clone()) {
            if previous.key != row.key {
                return Err("one trace assigned to different keys".into());
            }
        }
    }
    let work: Arc<Vec<_>> = Arc::new(unique.into_values().collect());
    let next = Arc::new(AtomicUsize::new(0));
    let results = Arc::new(Mutex::new(HashMap::new()));
    let source = env!("FC4_REPLAY_SOURCE_HASH");
    let jobs = jobs.max(1).min(work.len());
    std::thread::scope(|scope| {
        for _ in 0..jobs {
            let work = work.clone();
            let next = next.clone();
            let results = results.clone();
            let cache = &cache;
            scope.spawn(move || loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(row) = work.get(i) else {
                    break;
                };
                let outcome = (|| -> Result<_, String> {
                    let path = dir.join(&row.sha);
                    let digest = hash::sha256(BufReader::new(File::open(&path).map_err(|e| e.to_string())?))
                        .map_err(|e| e.to_string())?;
                    if digest != row.sha {
                        return Err("trace content digest mismatch".into());
                    }
                    let cached = cache.join(format!("{}-{source}", row.sha));
                    if !no_cache {
                        if let Ok(bytes) = fs::read(&cached) {
                            let mut r = Reader(&bytes);
                            if let Ok(v) = Verdict::get(&mut r) {
                                if r.end().is_ok() {
                                    return Ok((v, true));
                                }
                            }
                        }
                    }
                    let v = unsafe { replay_file(&path, owner(&row.key)?) }?;
                    fs::write(&cached, packed(&v)).map_err(|e| e.to_string())?;
                    Ok((v, false))
                })();
                results.lock().unwrap().insert(row.sha.clone(), outcome);
            });
        }
    });
    let results = results.lock().unwrap();
    let mut passed = true;
    let mut report=String::from("scenario\tkey\ttrace_sha\tsource_sha\tverdict\tshadow_equal\tcomparisons\tmismatches\tincomplete\tcache\terrors\n");
    for row in rows {
        let (v, cached) = match &results[&row.sha] {
            Ok(v) => v.clone(),
            Err(e) => (
                Verdict {
                    equal: false,
                    comparisons: 0,
                    mismatches: 0,
                    incomplete: 0,
                    errors: vec![e.clone()],
                },
                false,
            ),
        };
        let equal = v.equal
            && v.comparisons == row.comparisons
            && v.mismatches == row.mismatches
            && v.incomplete == row.incomplete;
        let verdict = if !equal || v.mismatches > 0 {
            "inexact"
        } else if v.comparisons == 0 || v.incomplete > 0 {
            "incomplete"
        } else {
            "exact"
        };
        passed &= equal && verdict == "exact";
        let errors = v.errors.join("; ").replace(['\t', '\r', '\n'], " ");
        println!(
            "{} {} {verdict} shadow_equal={equal} comparisons={} mismatches={} incomplete={} cache={cached} {errors}",
            row.scenario, row.key, v.comparisons, v.mismatches, v.incomplete
        );
        report += &format!(
            "{}\t{}\t{}\t{source}\t{verdict}\t{equal}\t{}\t{}\t{}\t{cached}\t{errors}\n",
            row.scenario, row.key, row.sha, v.comparisons, v.mismatches, v.incomplete
        );
    }
    fs::write(dir.join("replay-verdicts.tsv"), report).map_err(|e| e.to_string())?;
    println!(
        "fc4-replay traces={} jobs={jobs} wall_seconds={:.3} source_sha={source}",
        work.len(),
        start.elapsed().as_secs_f64()
    );
    Ok(passed)
}
