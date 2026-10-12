// SPDX-License-Identifier: GPL-3.0-or-later
//! Sweeps the installed game's packages, when present, and parses everything this crate understands.
//!
//! The install is found through `FC4_GAME_DIR`, or the default GOG path. The test is skipped (it passes with a
//! note) when neither exists. Game files are only opened for reading.
//!
//! Run with output: `cargo test -p fc4-formats --release --test game_data -- --nocapture`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use fc4_formats::exemplar::{ChainEnd, Encoding, Exemplar, ExemplarStore, Kind};
use fc4_formats::tgi::{TYPE_COHORT, TYPE_EXEMPLAR, TYPE_LTEXT};
use fc4_formats::{ltext, qfs, Error, Package};

const DEFAULT_DIR: &str = r"C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition";

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FC4_GAME_DIR")
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
        } else if group_of(&p).is_some() {
            out.push(p);
        }
    }
}

/// Report group by extension: game data (`.dat`), city saves (`.sc4`), or plugin files.
fn group_of(p: &Path) -> Option<&'static str> {
    let ext = p.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "dat" => Some("dat"),
        "sc4" => Some("sc4"),
        "sc4lot" | "sc4desc" | "sc4model" => Some("plugin"),
        _ => None,
    }
}

#[derive(Default, Debug)]
struct Stats {
    files: usize,
    not_dbpf: Vec<String>,
    entries: usize,
    bytes_unpacked: u64,
    compressed: usize,
    dir_listed_not_qfs: usize,
    qfs_not_listed: usize,
    dir_size_mismatch: usize,
    exemplars_binary: usize,
    exemplars_text: usize,
    cohorts: usize,
    reencode_identical: usize,
    reencode_different: usize,
    ltext: usize,
    ltext_type_other: BTreeMap<&'static str, usize>,
    qfs_roundtrips: usize,
    warnings: usize,
    failures: Vec<String>,
}

#[test]
fn sweep_installed_game_data() {
    let Some(dir) = game_dir() else {
        eprintln!("game data not found (set FC4_GAME_DIR); skipping");
        return;
    };
    let mut files = Vec::new();
    collect(&dir, &mut files);
    let mut by_group: BTreeMap<&str, Stats> = BTreeMap::new();
    let mut store = ExemplarStore::new();
    let mut type_counts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut value_types: BTreeMap<String, usize> = BTreeMap::new();

    for path in &files {
        let group = group_of(path).unwrap_or("other");
        let s = by_group.entry(group).or_default();
        let name = path.strip_prefix(&dir).unwrap_or(path).display().to_string();
        let pkg = match Package::open(path) {
            Ok(p) => p,
            Err(Error::BadMagic { .. }) => {
                s.not_dbpf.push(name);
                continue;
            }
            Err(e) => {
                s.failures.push(format!("{name}: open: {e}"));
                continue;
            }
        };
        s.files += 1;
        s.warnings += pkg.warnings().len();
        for w in pkg.warnings().iter().take(3) {
            eprintln!("warning {name}: {w}");
        }
        for (n, e) in pkg.entries().iter().enumerate() {
            s.entries += 1;
            if group == "dat" {
                *type_counts.entry(e.tgi.type_id).or_default() += 1;
            }
            let raw = match pkg.read_raw(e) {
                Ok(r) => r,
                Err(err) => {
                    s.failures.push(format!("{name} {}: read: {err}", e.tgi));
                    continue;
                }
            };
            let listed = pkg.dir_size(e);
            let has_qfs = qfs::has_header(&raw);
            match (listed.is_some(), has_qfs) {
                (true, false) => s.dir_listed_not_qfs += 1,
                (false, true) => {
                    s.qfs_not_listed += 1;
                    eprintln!("note {name} {}: QFS header but not in the DIR (read as stored)", e.tgi);
                }
                _ => {}
            }
            let data = match pkg.read(e) {
                Ok(d) => d,
                Err(err) => {
                    s.failures.push(format!("{name} {}: decompress: {err}", e.tgi));
                    continue;
                }
            };
            s.bytes_unpacked += data.len() as u64;
            if listed.is_some() && has_qfs {
                s.compressed += 1;
                if listed != Some(data.len() as u32) {
                    s.dir_size_mismatch += 1;
                }
                // Re-compress a sample of real entries and check the round trip.
                if n % 97 == 0 {
                    match qfs::compress(&data).and_then(|c| qfs::decompress(&c)) {
                        Ok(back) if back == data => s.qfs_roundtrips += 1,
                        Ok(_) => s.failures.push(format!("{name} {}: QFS round trip differs", e.tgi)),
                        Err(err) => s.failures.push(format!("{name} {}: QFS round trip: {err}", e.tgi)),
                    }
                }
            }
            match e.tgi.type_id {
                TYPE_EXEMPLAR | TYPE_COHORT => match Exemplar::parse(&data) {
                    Ok(ex) => {
                        match (ex.kind, ex.encoding) {
                            (Kind::Cohort, _) => s.cohorts += 1,
                            (Kind::Exemplar, Encoding::Binary) => s.exemplars_binary += 1,
                            (Kind::Exemplar, Encoding::Text) => {
                                s.exemplars_text += 1;
                                eprintln!(
                                    "note {name} {}: text exemplar, {} properties",
                                    e.tgi,
                                    ex.properties.len()
                                );
                            }
                        }
                        if ex.encoding == Encoding::Binary {
                            if ex.to_binary() == data {
                                s.reencode_identical += 1;
                            } else {
                                s.reencode_different += 1;
                            }
                        }
                        for p in &ex.properties {
                            let shape = if p.is_array { "[]" } else { "" };
                            *value_types
                                .entry(format!("{}{shape}", p.value_type().name()))
                                .or_default() += 1;
                        }
                        if group == "dat" {
                            store.insert(e.tgi, ex);
                        }
                    }
                    Err(err) => s.failures.push(format!("{name} {}: exemplar: {err}", e.tgi)),
                },
                TYPE_LTEXT if ltext::is_ltext(&data) => match ltext::parse(&data) {
                    Ok(_) => s.ltext += 1,
                    Err(err) => s.failures.push(format!("{name} {}: LTEXT: {err}", e.tgi)),
                },
                TYPE_LTEXT => {
                    // The LTEXT type is shared with audio clips and the odd bare-text entry.
                    let kind = if data.starts_with(b"XA") {
                        "XA audio"
                    } else if data.starts_with(b"RIFF") {
                        "RIFF audio"
                    } else if data
                        .iter()
                        .all(|&b| b"\r\n\t".contains(&b) || (0x20..0x7F).contains(&b))
                    {
                        "bare 8-bit text"
                    } else {
                        "other"
                    };
                    *s.ltext_type_other.entry(kind).or_default() += 1;
                    if kind == "bare 8-bit text" || kind == "other" {
                        eprintln!(
                            "note {name} {}: LTEXT type without LTEXT header ({kind}, {} bytes)",
                            e.tgi,
                            data.len()
                        );
                    }
                }
                _ => {}
            }
        }
    }

    // Cohort inheritance across all game .dat files.
    let mut chains: BTreeMap<String, usize> = BTreeMap::new();
    let mut max_depth = 0;
    for (_, ex) in store.iter() {
        let (chain, end) = store.chain(ex);
        max_depth = max_depth.max(chain.len() - 1);
        let k = match end {
            ChainEnd::Root if chain.len() == 1 => "no parent",
            ChainEnd::Root => "parent chain resolved",
            ChainEnd::Missing(t) => {
                eprintln!("note: parent cohort {t} not found in the game data");
                "parent missing"
            }
            ChainEnd::Cycle(_) => "cycle",
            ChainEnd::TooDeep => "too deep",
        };
        *chains.entry(k.to_string()).or_default() += 1;
    }

    eprintln!("game data: {}", dir.display());
    let mut total_failures = 0;
    for (g, s) in &by_group {
        eprintln!(
            "[{g}] files {} (not DBPF: {:?}), entries {}, unpacked {} MB, compressed {}, \
             DIR-listed but not QFS {}, QFS but not DIR-listed {}, DIR size mismatches {}, open warnings {}",
            s.files,
            s.not_dbpf,
            s.entries,
            s.bytes_unpacked / 1_000_000,
            s.compressed,
            s.dir_listed_not_qfs,
            s.qfs_not_listed,
            s.dir_size_mismatch,
            s.warnings
        );
        eprintln!(
            "[{g}] exemplars binary {}, text {}, cohorts {}, binary re-encode identical {} / different {}, \
             LTEXT {} (same type, not LTEXT: {:?}), QFS re-compress round trips {}, failures {}",
            s.exemplars_binary,
            s.exemplars_text,
            s.cohorts,
            s.reencode_identical,
            s.reencode_different,
            s.ltext,
            s.ltext_type_other,
            s.qfs_roundtrips,
            s.failures.len()
        );
        for f in s.failures.iter().take(20) {
            eprintln!("  FAIL {f}");
        }
        total_failures += s.failures.len();
    }
    eprintln!("cohort chains over .dat exemplars: {chains:?}, deepest {max_depth}");
    eprintln!("exemplar property value types: {value_types:?}");
    let top: Vec<String> = {
        let mut v: Vec<_> = type_counts.iter().collect();
        v.sort_by(|a, b| b.1.cmp(a.1));
        v.iter().take(12).map(|(t, n)| format!("{t:08X}:{n}")).collect()
    };
    eprintln!("most common .dat types: {}", top.join(" "));
    assert_eq!(total_failures, 0, "see FAIL lines above");
}
