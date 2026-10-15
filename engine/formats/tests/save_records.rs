// SPDX-License-Identifier: GPL-3.0-or-later
//! The save sweep's record mode: for each city save, the history warehouse record (class `0x89EFA536`) and the
//! 24-hour clock record (class `0xA7E7F929`) of the class-object store (foundations §3.3) are read with the
//! FreeCity4 cores (`fc4_core::history`, `fc4_core::clock24`), written back, and compared with the saved bytes.
//!
//! For every entry of the two class records: the entry framing and its CRC (foundations §3.4) are checked, the
//! payload is read by a fresh core (it must read to the end, nothing left over) and written again.
//! - History (history §7): a version-2 payload must come back byte-identical. An older one is converted (the
//!   writer always writes version 2; version 1 reads a queue length of 100 as 0; an unset or repeated date is
//!   folded), so it is reported as converted, and the conversion must be stable: the rewritten bytes read and
//!   written again must not change.
//! - Clock (clock24 §7): the writer always writes 26 bytes of version 2. The saved bytes are compared with the
//!   version field set to 2 and, for a version below 2, the 10-byte timer record after the time of day removed.
//!
//! The saves are found through `FC4_SAVES_DIR` (every `.sc4` below it), else the installed game (`FC4_GAME_DIR`,
//! or the default GOG path), as `tests/game_data.rs` does. Files with the same content are checked once. The
//! test passes with a note when no save is found. Game files are only opened for reading.
//!
//! Run with output:
//! `cargo test -p fc4-formats --release --test save_records -- --nocapture`
//! (`FC4_SAVES_DIR=<folder>` to sweep another corpus; `FC4_SAVE_RECORDS_VERBOSE=1` prints every record).

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use fc4_core::clock24::Clock24;
use fc4_core::clock24::{ClockHost, ClockTuning, CLSID_24_HOUR_CLOCK};
use fc4_core::history::{HistoryHost, HistoryWarehouse, CLSID_HISTORY_WAREHOUSE};
use fc4_core::store::{crc32_mpeg2, CRC_LIMIT, STORE_GROUP};
use fc4_core::stream::{ByteReader, ByteWriter};
use fc4_formats::dbpf::Compression;
use fc4_formats::tgi::Tgi;
use fc4_formats::{Package, PackageBuilder, Source};

const DEFAULT_DIR: &str = r"C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition";

// ---- hosts: the record round trip calls none of them ------------------------------------------------------------

struct NoHistoryHost;

impl HistoryHost for NoHistoryHost {
    fn sim_date(&self) -> u32 {
        0
    }
    fn now_ms(&self) -> u32 {
        0
    }
    fn subscribe(&self, _: u32) {}
    fn unsubscribe(&self, _: u32) {}
    fn add_agent(&self, _: u32, _: &str, _: u32) {}
    fn remove_agent(&self) {}
    fn send(&self, _: u32, _: u32, _: u32) {}
}

struct NoClockHost;

impl ClockHost for NoClockHost {
    fn now_micros(&self) -> u64 {
        0
    }
    fn post(&self, _: u32, _: u32, _: u32) {}
    fn add_system_service(&self) {}
    fn remove_system_service(&self) {}
    fn add_to_on_idle(&self) {}
    fn remove_from_on_idle(&self) {}
    fn has_cheat_manager(&self) -> bool {
        false
    }
    fn register_cheat(&self, _: u32, _: &str) {}
    fn add_cheat_notification(&self, _: u32) {}
    fn remove_cheat_notification(&self, _: u32) {}
    fn unregister_cheat(&self, _: u32) {}
    fn tuning(&self) -> Option<ClockTuning> {
        None
    }
}

// ---- the checks ------------------------------------------------------------------------------------------------

/// What one payload's round trip gave.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// Written back byte-identical (the clock: after its version normalisation).
    Identical,
    /// An older history version, converted to version 2; the conversion is stable.
    Converted,
    /// A failure, with its description.
    Failed(String),
}

fn first_difference(ours: &[u8], saved: &[u8]) -> String {
    let n = ours.len().min(saved.len());
    match (0..n).find(|&i| ours[i] != saved[i]) {
        Some(i) => format!(
            "first difference at byte {i} (written {:#04x}, saved {:#04x}; {} vs {} bytes)",
            ours[i],
            saved[i],
            ours.len(),
            saved.len()
        ),
        None => format!("lengths differ ({} written, {} saved)", ours.len(), saved.len()),
    }
}

fn version_of(payload: &[u8]) -> Option<u16> {
    payload.get(..2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

/// Reads `payload` into a fresh warehouse and writes it again: the bytes, the series and the points.
fn history_round_trip(payload: &[u8]) -> Result<(Vec<u8>, usize, usize), String> {
    let mut w = HistoryWarehouse::new(Box::new(NoHistoryHost));
    let mut r = ByteReader::new(payload);
    if !w.read(&mut r) {
        return Err(format!(
            "Read failed ({} of {} bytes left)",
            r.remaining(),
            payload.len()
        ));
    }
    if r.remaining() != 0 {
        return Err(format!("{} bytes left after Read", r.remaining()));
    }
    let mut out = ByteWriter::new();
    if !w.write(&mut out) {
        return Err("Write failed".into());
    }
    Ok((out.into_bytes(), w.series_count(), w.total_points()))
}

fn check_history(payload: &[u8], stats: &mut Stats) -> Outcome {
    let Some(version) = version_of(payload) else {
        return Outcome::Failed("no version".into());
    };
    *stats.versions.entry(version).or_insert(0) += 1;
    let (ours, series, points) = match history_round_trip(payload) {
        Ok(v) => v,
        Err(e) => return Outcome::Failed(format!("version {version}: {e}")),
    };
    stats.series += series;
    stats.points += points;
    if ours == payload {
        return Outcome::Identical;
    }
    if version >= 2 {
        return Outcome::Failed(format!("version {version}: {}", first_difference(&ours, payload)));
    }
    // An older version: the rewrite (version 2) must itself round-trip.
    match history_round_trip(&ours) {
        Ok((again, _, _)) if again == ours => Outcome::Converted,
        Ok((again, _, _)) => Outcome::Failed(format!(
            "version {version}: the converted record does not round-trip: {}",
            first_difference(&again, &ours)
        )),
        Err(e) => Outcome::Failed(format!("version {version}: the converted record does not read: {e}")),
    }
}

/// The bytes the clock's writer must produce from `payload` (clock24 §7): version 2, and below version 2 the
/// timer record (10 bytes after the time of day, at offset 18) left out.
fn clock_expected(payload: &[u8]) -> Option<Vec<u8>> {
    let version = version_of(payload)?;
    let mut e = Vec::with_capacity(26);
    e.extend_from_slice(&2u16.to_le_bytes());
    if version < 2 {
        e.extend_from_slice(payload.get(2..18)?);
        e.extend_from_slice(payload.get(28..36)?);
    } else {
        e.extend_from_slice(payload.get(2..26)?);
    }
    Some(e)
}

fn check_clock(payload: &[u8], stats: &mut Stats) -> Outcome {
    let Some(version) = version_of(payload) else {
        return Outcome::Failed("no version".into());
    };
    *stats.versions.entry(version).or_insert(0) += 1;
    let mut c = Clock24::new(Box::new(NoClockHost));
    let mut r = ByteReader::new(payload);
    if !c.read(&mut r) {
        return Outcome::Failed(format!(
            "version {version}: Read failed ({} of {} bytes left)",
            r.remaining(),
            payload.len()
        ));
    }
    if r.remaining() != 0 {
        return Outcome::Failed(format!("version {version}: {} bytes left after Read", r.remaining()));
    }
    let mut out = ByteWriter::new();
    if !c.write(&mut out) {
        return Outcome::Failed(format!("version {version}: Write failed"));
    }
    let ours = out.into_bytes();
    let Some(expected) = clock_expected(payload) else {
        return Outcome::Failed(format!("version {version}: {} bytes, too short", payload.len()));
    };
    if ours == expected {
        Outcome::Identical
    } else {
        Outcome::Failed(format!("version {version}: {}", first_difference(&ours, &expected)))
    }
}

// ---- the class records -----------------------------------------------------------------------------------------

/// One class record split into its entries (foundations §3.3): `(objectID, payload)`, or what is wrong with the
/// framing or a CRC (§3.4).
fn split_entries(record: &[u8]) -> Result<Vec<(u32, &[u8])>, String> {
    let mut out = Vec::new();
    let mut p = 0usize;
    while p < record.len() {
        let word = |at: usize| -> Option<u32> {
            record
                .get(at..at + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let (Some(size), Some(crc), Some(id)) = (word(p), word(p + 4), word(p + 8)) else {
            return Err(format!("a cut entry header at byte {p}"));
        };
        let size = size as usize;
        if size < 12 || p + size > record.len() {
            return Err(format!(
                "entry at byte {p}: size {size} outside the record ({} bytes)",
                record.len()
            ));
        }
        let span = (size - 8).min(CRC_LIMIT);
        let want = crc32_mpeg2(&record[p + 8..p + 8 + span]);
        if want != crc {
            return Err(format!(
                "entry at byte {p} (object {id:#x}): CRC {crc:#010x}, computed {want:#010x}"
            ));
        }
        out.push((id, &record[p + 12..p + size]));
        p += size;
    }
    Ok(out)
}

#[derive(Default)]
struct Stats {
    records: usize,
    entries: usize,
    identical: usize,
    converted: usize,
    versions: BTreeMap<u16, usize>,
    series: usize,
    points: usize,
    missing: usize,
}

#[derive(Default)]
struct Report {
    history: Stats,
    clock: Stats,
    failures: Vec<String>,
    verbose: bool,
}

/// Checks both records of one package. `name` labels the lines.
fn check_package<S: Source>(name: &str, pkg: &Package<S>, rep: &mut Report) {
    for (class, label) in [(CLSID_HISTORY_WAREHOUSE, "history"), (CLSID_24_HOUR_CLOCK, "clock")] {
        let tgi = Tgi::new(class, STORE_GROUP, 0);
        let is_history = class == CLSID_HISTORY_WAREHOUSE;
        let Some(e) = pkg.find(&tgi) else {
            let stats = if is_history { &mut rep.history } else { &mut rep.clock };
            stats.missing += 1;
            continue;
        };
        let record = match pkg.read(e) {
            Ok(r) => r,
            Err(err) => {
                rep.failures
                    .push(format!("{name}: {label} record does not read: {err}"));
                continue;
            }
        };
        let entries = match split_entries(&record) {
            Ok(v) => v,
            Err(err) => {
                rep.failures.push(format!("{name}: {label} record: {err}"));
                continue;
            }
        };
        let stats = if is_history { &mut rep.history } else { &mut rep.clock };
        stats.records += 1;
        for (id, payload) in entries {
            stats.entries += 1;
            let outcome = if is_history {
                check_history(payload, stats)
            } else {
                check_clock(payload, stats)
            };
            if rep.verbose {
                println!("  {name}: {label} object {id:#x}, {} bytes: {outcome:?}", payload.len());
            }
            match outcome {
                Outcome::Identical => stats.identical += 1,
                Outcome::Converted => stats.converted += 1,
                Outcome::Failed(why) => rep.failures.push(format!("{name}: {label} object {id:#x}: {why}")),
            }
        }
    }
}

fn print_stats(label: &str, s: &Stats) {
    let versions: Vec<String> = s.versions.iter().map(|(v, n)| format!("v{v} {n}")).collect();
    println!(
        "{label}: {} records ({} saves without one), {} entries ({}): {} byte-identical, {} converted (stable){}",
        s.records,
        s.missing,
        s.entries,
        versions.join(", "),
        s.identical,
        s.converted,
        if s.series > 0 {
            format!("; {} series, {} points", s.series, s.points)
        } else {
            String::new()
        }
    );
}

// ---- the sweep -------------------------------------------------------------------------------------------------

fn saves_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FC4_SAVES_DIR")
        .or_else(|| std::env::var_os("FC4_GAME_DIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DIR));
    dir.is_dir().then_some(dir)
}

fn collect_saves(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut items: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    items.sort();
    for p in items {
        if p.is_dir() {
            collect_saves(&p, out);
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("sc4"))
        {
            out.push(p);
        }
    }
}

#[test]
fn sweep_history_and_clock_records() {
    let Some(dir) = saves_dir() else {
        println!("save_records: no saves found (set FC4_SAVES_DIR or FC4_GAME_DIR); skipped");
        return;
    };
    let mut files = Vec::new();
    collect_saves(&dir, &mut files);
    let mut rep = Report {
        verbose: std::env::var_os("FC4_SAVE_RECORDS_VERBOSE").is_some_and(|v| v == "1"),
        ..Report::default()
    };
    let mut seen = HashSet::new();
    let mut distinct = 0usize;
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            rep.failures.push(format!("{}: unreadable", path.display()));
            continue;
        };
        let mut h = DefaultHasher::new();
        bytes.hash(&mut h);
        if !seen.insert((bytes.len(), h.finish())) {
            continue;
        }
        distinct += 1;
        let name = path.strip_prefix(&dir).unwrap_or(path).display().to_string();
        match Package::from_bytes(bytes) {
            Ok(pkg) => check_package(&name, &pkg, &mut rep),
            Err(err) => rep.failures.push(format!("{name}: does not open: {err}")),
        }
    }
    println!(
        "save_records: {} ({} .sc4 files, {} distinct)",
        dir.display(),
        files.len(),
        distinct
    );
    print_stats("history warehouse (0x89EFA536)", &rep.history);
    print_stats("24-hour clock (0xA7E7F929)", &rep.clock);
    for f in &rep.failures {
        println!("FAILURE {f}");
    }
    println!("save_records: {} failures", rep.failures.len());
    assert!(
        rep.failures.is_empty(),
        "{} failures (see the output)",
        rep.failures.len()
    );
}

// ---- synthetic records -----------------------------------------------------------------------------------------

/// A class record of `(objectID, payload)` entries with their sizes and CRCs (foundations §3.3, §3.4).
fn class_record(entries: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (id, payload) in entries {
        let p = out.len();
        out.extend_from_slice(&((12 + payload.len()) as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(payload);
        let span = (out.len() - p - 8).min(CRC_LIMIT);
        let crc = crc32_mpeg2(&out[p + 8..p + 8 + span]);
        out[p + 4..p + 8].copy_from_slice(&crc.to_le_bytes());
    }
    out
}

/// A history payload: version, then `(key, queue length, [(date, value)])` series.
/// One synthetic series: key, queue length, `(date, value)` points.
type SeriesSpec<'a> = (u32, u32, &'a [(u32, i32)]);

fn history_payload(version: u16, series: &[SeriesSpec]) -> Vec<u8> {
    let mut out = version.to_le_bytes().to_vec();
    out.extend_from_slice(&(series.len() as u32).to_le_bytes());
    for (key, max, points) in series {
        out.extend_from_slice(&key.to_le_bytes());
        out.extend_from_slice(&max.to_le_bytes());
        out.extend_from_slice(&(points.len() as u32).to_le_bytes());
        for (d, v) in *points {
            out.extend_from_slice(&d.to_le_bytes());
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    out
}

/// A clock payload; version 1 carries the 10-byte timer record.
fn clock_payload(version: u16, seconds: u32) -> Vec<u8> {
    let mut out = version.to_le_bytes().to_vec();
    out.extend_from_slice(&1i32.to_le_bytes()); // pause count
    out.extend_from_slice(&135.0f32.to_le_bytes());
    out.extend_from_slice(&2000.0f32.to_le_bytes());
    out.extend_from_slice(&seconds.to_le_bytes());
    if version < 2 {
        out.push(3);
        out.extend_from_slice(&123_456u64.to_le_bytes());
        out.push(1);
    }
    out.extend_from_slice(&20u32.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out
}

fn package_of(history: &[(u32, Vec<u8>)], clock: &[(u32, Vec<u8>)], compression: Compression) -> Vec<u8> {
    let mut b = PackageBuilder::new();
    if !history.is_empty() {
        b.add(
            Tgi::new(CLSID_HISTORY_WAREHOUSE, STORE_GROUP, 0),
            &class_record(history),
            compression,
        )
        .unwrap();
    }
    if !clock.is_empty() {
        b.add(
            Tgi::new(CLSID_24_HOUR_CLOCK, STORE_GROUP, 0),
            &class_record(clock),
            compression,
        )
        .unwrap();
    }
    b.build().unwrap()
}

fn sweep_bytes(bytes: Vec<u8>) -> Report {
    let pkg = Package::from_bytes(bytes).unwrap();
    let mut rep = Report::default();
    check_package("synthetic", &pkg, &mut rep);
    rep
}

#[test]
fn crc_rule_check_value() {
    assert_eq!(crc32_mpeg2(b"123456789"), 0x0376_E6E7);
}

#[test]
fn synthetic_current_versions_round_trip_identical() {
    let points: &[(u32, i32)] = &[(2_450_000, 5), (2_450_031, -7), (2_450_059, 1_000_000)];
    let h = history_payload(
        2,
        &[(0x1000, 0, points), (0x2000, 120, &[(2_450_100, 3)]), (0x3000, 0, &[])],
    );
    let c = clock_payload(2, 43_200);
    assert_eq!(c.len(), 26);
    for compression in [Compression::None, Compression::Always] {
        let rep = sweep_bytes(package_of(&[(7, h.clone())], &[(9, c.clone())], compression));
        assert!(rep.failures.is_empty(), "{:?}", rep.failures);
        assert_eq!(
            (rep.history.records, rep.history.entries, rep.history.identical),
            (1, 1, 1)
        );
        assert_eq!((rep.history.series, rep.history.points), (3, 4));
        assert_eq!((rep.clock.records, rep.clock.entries, rep.clock.identical), (1, 1, 1));
    }
}

#[test]
fn synthetic_old_versions_are_converted() {
    // Version 1: a queue length of 100 (read as 0), an unset date and a repeated one.
    let points: &[(u32, i32)] = &[(2_450_000, 5), (0xCDCD_CDCD, 6), (2_450_001, 7), (2_450_010, 8)];
    let h = history_payload(1, &[(0x1000, 100, points)]);
    let c = clock_payload(1, 3_600);
    assert_eq!(c.len(), 36);
    let rep = sweep_bytes(package_of(&[(1, h)], &[(2, c)], Compression::None));
    assert!(rep.failures.is_empty(), "{:?}", rep.failures);
    assert_eq!((rep.history.entries, rep.history.converted), (1, 1));
    assert_eq!(rep.history.points, 3);
    assert_eq!(rep.clock.identical, 1);
    // A clock version above 2 reads as 2 and is written as version 2.
    let mut c3 = clock_payload(2, 100);
    c3[0] = 3;
    let rep = sweep_bytes(package_of(&[], &[(2, c3)], Compression::None));
    assert!(rep.failures.is_empty(), "{:?}", rep.failures);
    assert_eq!((rep.clock.identical, rep.history.missing), (1, 1));
}

#[test]
fn synthetic_damage_is_reported() {
    let good = history_payload(2, &[(0x1000, 0, &[(2_450_000, 5)])]);
    // A wrong CRC.
    let mut record = class_record(&[(1, good.clone())]);
    record[4] ^= 1;
    let mut b = PackageBuilder::new();
    b.add(
        Tgi::new(CLSID_HISTORY_WAREHOUSE, STORE_GROUP, 0),
        &record,
        Compression::None,
    )
    .unwrap();
    let rep = sweep_bytes(b.build().unwrap());
    assert_eq!(rep.failures.len(), 1);
    assert!(rep.failures[0].contains("CRC"), "{}", rep.failures[0]);
    // Trailing bytes after the history payload.
    let mut long = good.clone();
    long.extend_from_slice(&[0, 0]);
    let rep = sweep_bytes(package_of(&[(1, long)], &[], Compression::None));
    assert!(
        rep.failures.len() == 1 && rep.failures[0].contains("2 bytes left"),
        "{:?}",
        rep.failures
    );
    // A version-2 history record the writer cannot reproduce (a repeated date folds into one point).
    let folded = history_payload(2, &[(0x1000, 0, &[(2_450_000, 5), (2_450_000, 6)])]);
    let rep = sweep_bytes(package_of(&[(1, folded)], &[], Compression::None));
    assert!(
        rep.failures.len() == 1 && rep.failures[0].contains("first difference"),
        "{:?}",
        rep.failures
    );
    // A cut clock record.
    let mut cut = clock_payload(2, 0);
    cut.truncate(20);
    let rep = sweep_bytes(package_of(&[], &[(1, cut)], Compression::None));
    assert!(
        rep.failures.len() == 1 && rep.failures[0].contains("Read failed"),
        "{:?}",
        rep.failures
    );
    // A cut entry header.
    let mut record = class_record(&[(1, good)]);
    record.extend_from_slice(&[1, 2, 3]);
    let mut b = PackageBuilder::new();
    b.add(
        Tgi::new(CLSID_HISTORY_WAREHOUSE, STORE_GROUP, 0),
        &record,
        Compression::None,
    )
    .unwrap();
    let rep = sweep_bytes(b.build().unwrap());
    assert!(
        rep.failures.len() == 1 && rep.failures[0].contains("cut entry header"),
        "{:?}",
        rep.failures
    );
}
