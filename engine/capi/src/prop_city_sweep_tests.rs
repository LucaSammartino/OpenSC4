// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The save sweep for the prop developer (`0x89C48F47`, prop-developer.md
//! §7) and the city detail manager (`0xEAD4BA36`, city-detail.md §4): every
//! class-record entry of those classes in every distinct city save is read
//! into a fresh object over the C ABI and written again.
//!
//! - prop developer: a version-5 record must come back byte-identical; a
//!   version-4 record (no signs) comes back as version 5: the version 4 → 5
//!   and `01 00000000` (signs visible, an empty sign list) appended; that
//!   re-write read again must give itself back;
//! - city detail: byte-identical.
//!
//! Records, references and saves as `save_sweep_tests.rs` (foundations §3.2,
//! §3.3; `FC4_SAVES_DIR`, `FC4_SAVE_DIR`, `FC4_GAME_DIR` or the GOG default;
//! the test passes with a note when there is no directory). Run with output:
//!
//! ```text
//! cargo test -p fc4-capi --release prop_city_save_sweep -- --nocapture
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use fc4_formats::{Package, Tgi};

const CLSID_PROP_DEVELOPER: u32 = 0x89C4_8F47;
const CLSID_CITY_DETAIL: u32 = 0xEAD4_BA36;
const CLASS_GROUP: u32 = 0x299B_2D1B;
const DEFAULT_DIR: &str = r"C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition";

fn save_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FC4_SAVES_DIR")
        .or_else(|| std::env::var_os("FC4_SAVE_DIR"))
        .or_else(|| std::env::var_os("FC4_GAME_DIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DIR));
    dir.is_dir().then_some(dir)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut items: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    items.sort();
    for p in items {
        if p.is_dir() {
            collect(&p, out);
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("sc4"))
        {
            out.push(p);
        }
    }
}

fn fnv64(b: &[u8]) -> u64 {
    let mut h = 0xCBF2_9CE4_8422_2325u64;
    for &x in b {
        h = (h ^ u64::from(x)).wrapping_mul(0x0100_0000_01B3);
    }
    h
}

/// Class-record entries (foundations §3.3): (objectID, payload).
fn entries(rec: &[u8]) -> Result<Vec<(u32, &[u8])>, String> {
    let mut v = Vec::new();
    let mut p = 0;
    while p < rec.len() {
        let size = rec
            .get(p..p + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
            .ok_or("truncated entry header")?;
        if size < 12 || p + size > rec.len() {
            return Err(format!("entry size {size} at {p}"));
        }
        let id = u32::from_le_bytes(rec[p + 8..p + 12].try_into().unwrap());
        v.push((id, &rec[p + 12..p + size]));
        p += size;
    }
    Ok(v)
}

fn round_trip(class: u32, payload: &[u8]) -> Result<Vec<u8>, String> {
    if class == CLSID_PROP_DEVELOPER {
        crate::prop_developer_tests::sweep_round_trip(payload)
    } else {
        crate::city_detail_tests::sweep_round_trip(payload)
    }
}

/// §7: a version-4 prop record as version 5 writes it.
fn prop_v5_of_v4(payload: &[u8]) -> Vec<u8> {
    let mut v = payload.to_vec();
    v[0] = 5;
    v[1] = 0;
    v.extend_from_slice(&[1, 0, 0, 0, 0]);
    v
}

#[derive(Default)]
struct Tally {
    records: usize,
    identical: usize,
    upgraded: usize,
    second: usize,
    versions: BTreeMap<u32, usize>,
    sizes: BTreeMap<usize, usize>,
    failures: Vec<String>,
}

#[test]
fn prop_city_save_sweep() {
    let Some(dir) = save_dir() else {
        eprintln!(
            "prop/city detail save sweep: no save directory (FC4_SAVES_DIR / FC4_GAME_DIR / the GOG default); skipped"
        );
        return;
    };
    let mut files = Vec::new();
    collect(&dir, &mut files);
    let classes = [("propdev", CLSID_PROP_DEVELOPER), ("citydetail", CLSID_CITY_DETAIL)];
    let mut seen = BTreeSet::new();
    let mut distinct = 0;
    let mut tallies: BTreeMap<&str, Tally> = BTreeMap::new();
    let mut absent: BTreeMap<&str, usize> = BTreeMap::new();
    let mut open_failures = Vec::new();
    for f in &files {
        let Ok(bytes) = std::fs::read(f) else {
            open_failures.push(format!("{}: unreadable", f.display()));
            continue;
        };
        if !seen.insert(fnv64(&bytes)) {
            continue;
        }
        distinct += 1;
        let name = f.strip_prefix(&dir).unwrap_or(f).display().to_string();
        let pkg = match Package::from_bytes(bytes) {
            Ok(p) => p,
            Err(e) => {
                open_failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        for (label, class) in classes {
            let t = tallies.entry(label).or_default();
            // city-detail.md §4: most saves (the templates) have no record.
            let Ok(rec) = pkg.read_tgi(&Tgi::new(class, CLASS_GROUP, 0)) else {
                *absent.entry(label).or_default() += 1;
                continue;
            };
            let list = match entries(&rec) {
                Ok(l) => l,
                Err(e) => {
                    t.failures.push(format!("{name}: {e}"));
                    continue;
                }
            };
            for (id, payload) in list {
                t.records += 1;
                *t.sizes.entry(payload.len()).or_default() += 1;
                let version = if class == CLSID_PROP_DEVELOPER {
                    payload
                        .get(0..2)
                        .map_or(0, |b| u32::from(u16::from_le_bytes([b[0], b[1]])))
                } else {
                    payload
                        .get(0..4)
                        .map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
                };
                *t.versions.entry(version).or_default() += 1;
                let once = match round_trip(class, payload) {
                    Ok(b) => b,
                    Err(e) => {
                        t.failures.push(format!("{name} object {id:08X}: {e}"));
                        continue;
                    }
                };
                let expected = if class == CLSID_PROP_DEVELOPER && version == 4 {
                    prop_v5_of_v4(payload)
                } else {
                    payload.to_vec()
                };
                if once != expected {
                    let at = once.iter().zip(expected.iter()).position(|(a, b)| a != b);
                    t.failures.push(format!(
                        "{name} object {id:08X}: re-write differs (v{version}, {} vs {} bytes, first at {at:?})",
                        once.len(),
                        expected.len()
                    ));
                    continue;
                }
                if once == payload {
                    t.identical += 1;
                } else {
                    t.upgraded += 1;
                }
                match round_trip(class, &once) {
                    Ok(b) if b == once => t.second += 1,
                    Ok(_) => t
                        .failures
                        .push(format!("{name} object {id:08X}: second round trip differs")),
                    Err(e) => t
                        .failures
                        .push(format!("{name} object {id:08X}: second round trip: {e}")),
                }
            }
        }
    }
    eprintln!(
        "prop/city detail save sweep: {} .sc4 files below {}, {} distinct",
        files.len(),
        dir.display(),
        distinct
    );
    for f in &open_failures {
        eprintln!("prop/city detail save sweep: {f}");
    }
    let mut failed = !open_failures.is_empty();
    for (label, _) in classes {
        let t = tallies.remove(label).unwrap_or_default();
        let sizes: Vec<String> = t.sizes.iter().map(|(s, n)| format!("{s} B x{n}")).collect();
        let versions: Vec<String> = t.versions.iter().map(|(v, n)| format!("v{v} x{n}")).collect();
        eprintln!(
            "save sweep {label}: records={} identical={} v4-written-as-v5={} second-trip-identical={} failures={} \
             (saves without the record: {}); versions: {}; sizes: {}",
            t.records,
            t.identical,
            t.upgraded,
            t.second,
            t.failures.len(),
            absent.get(label).copied().unwrap_or(0),
            versions.join(", "),
            sizes.join(", ")
        );
        for f in t.failures.iter().take(20) {
            eprintln!("  {f}");
        }
        failed |= !t.failures.is_empty();
    }
    assert!(!failed, "prop/city detail save sweep: failures above");
}

#[test]
fn prop_v4_upgrade_helper() {
    let v4 = vec![4, 0, 1, 2, 3];
    assert_eq!(prop_v5_of_v4(&v4), vec![5, 0, 1, 2, 3, 1, 0, 0, 0, 0]);
}
