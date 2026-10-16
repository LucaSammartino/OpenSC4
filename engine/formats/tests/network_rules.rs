// SPDX-License-Identifier: GPL-3.0-or-later
//! Sweeps the installed game's network rule resources (`docs/specs/network-rules.md` §2) with
//! `fc4_core::network_rules`: every resource of type `0x0A5BCF4B`, group `0xAA5BCF57` in the install's top-folder
//! `.dat` files (the tile rule files, RUL0, RUL1, RUL2) and the network INI (instance `0x8A5993B9`), read at run time
//! and never stored in the repository.
//!
//! For each tile file it reports the load status and the `#Transmogrify` counts (source rules, rules kept, variants
//! dropped as repeats); for the INI resources the entries used and ignored; then it runs the §2.1 load order
//! (`NetworkRules::reload`) over the same bytes and reports the evaluator and rule counts per type. Every tile file
//! must read to its end (`Complete`) and every INI entry must be used.
//!
//! The install is found through `FC4_GAME_DIR`, or the default GOG path; the test passes with a note when neither
//! exists. Game files are only opened for reading. For a key held by more than one package the first package in
//! file-name order is used, and the others are reported.
//!
//! `cargo test -p fc4-formats --release --test network_rules -- --nocapture`

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::path::PathBuf;

use fc4_core::network_rules::{
    tile_instance_owner, Evaluator, LoadStatus, NetworkRules, ResourceId, INTERSECTION_INSTANCE, MULTI_TYPE_INSTANCE,
    NETWORK_INI_INSTANCE, OVERRIDES_INSTANCE, RULE_GROUP, RULE_TYPE, TYPE_NAMES,
};
use fc4_formats::tgi::Tgi;
use fc4_formats::Package;

const DEFAULT_DIR: &str = r"C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition";

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FC4_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DIR));
    dir.is_dir().then_some(dir)
}

fn dat_files(dir: &PathBuf) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|x| x.eq_ignore_ascii_case("dat"))
        })
        .collect();
    files.sort();
    files
}

fn name_of(inst: u32) -> String {
    match inst {
        INTERSECTION_INSTANCE => "RUL0 (intersections)".into(),
        MULTI_TYPE_INSTANCE => "RUL1 (multi-type)".into(),
        OVERRIDES_INSTANCE => "RUL2 (rule overrides)".into(),
        NETWORK_INI_INSTANCE => "network INI".into(),
        i => match tile_instance_owner(i) {
            Some((t, k)) => format!("{} tile file {}", TYPE_NAMES[t as usize], k + 1),
            None => "unknown".into(),
        },
    }
}

#[test]
fn sweep_network_rules() {
    let Some(dir) = game_dir() else {
        eprintln!("game data not found (set FC4_GAME_DIR); skipping");
        return;
    };
    let files = dat_files(&dir);
    if files.is_empty() {
        eprintln!("no .dat files in {}; skipping", dir.display());
        return;
    }

    // instance -> (file, tgi, bytes); later copies are reported only.
    let mut found: BTreeMap<u32, (String, Tgi, Vec<u8>)> = BTreeMap::new();
    let mut duplicates: Vec<String> = Vec::new();
    let mut other_ini_keys: Vec<String> = Vec::new();
    for path in &files {
        let Ok(pkg) = Package::open(path) else { continue };
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        for e in pkg.entries() {
            let rule = e.tgi.type_id == RULE_TYPE && e.tgi.group == RULE_GROUP;
            let ini = e.tgi.instance == NETWORK_INI_INSTANCE;
            if !rule && !ini {
                continue;
            }
            if ini && !rule && e.tgi.group != fc4_core::network_rules::NETWORK_INI_GROUP {
                other_ini_keys.push(format!("{file} {}", e.tgi));
            }
            if !pkg.find(&e.tgi).is_some_and(|f| std::ptr::eq(f, e)) {
                continue;
            }
            let data = pkg.read(e).unwrap_or_else(|err| panic!("{file} {}: {err:?}", e.tgi));
            match found.entry(e.tgi.instance) {
                Entry::Occupied(_) => duplicates.push(format!("{file} {}", e.tgi)),
                Entry::Vacant(v) => {
                    v.insert((file.clone(), e.tgi, data));
                }
            }
        }
    }
    eprintln!(
        "network rule resources: {} found in {} .dat files",
        found.len(),
        files.len()
    );
    for d in &duplicates {
        eprintln!("  later copy (not used): {d}");
    }
    for k in &other_ini_keys {
        eprintln!("  instance 0x8A5993B9 under another group: {k}");
    }
    assert!(!found.is_empty(), "no network rule resource in the install");

    // Each resource on its own.
    let mut failures: Vec<String> = Vec::new();
    let mut totals = (0usize, 0usize, 0usize, 0usize);
    for (&inst, (file, tgi, data)) in &found {
        // The bridge and tunnel evaluators (instances 0x1000 and up) are outside the spec (§1, network-geometry
        // §7.1); they are read as tile rule files for information only.
        if (0x1000..INTERSECTION_INSTANCE).contains(&inst) {
            let ev = Evaluator::from_bytes(data);
            eprintln!(
                "  {file} {tgi} bridge/tunnel evaluator (not in the spec): {} bytes, read as a tile file: {:?}, transmogrify {}, source rules {}, rules {}, dropped {}",
                data.len(),
                ev.status,
                ev.transmogrify,
                ev.source_rules,
                ev.rule_count(),
                ev.dropped_variants
            );
            continue;
        }
        let mut rules = NetworkRules::new();
        let id = if inst == NETWORK_INI_INSTANCE {
            ResourceId::NetworkIni
        } else {
            ResourceId::Rule(inst)
        };
        let rep = rules.load_resource(id, data);
        let what = name_of(inst);
        match rep.status {
            Some(st) => {
                eprintln!(
                    "  {file} {tgi} {what}: {} bytes, {st:?}, transmogrify {}, source rules {}, rules {}, dropped {}",
                    data.len(),
                    rep.transmogrify,
                    rep.source_rules,
                    rep.rules,
                    rep.dropped_variants
                );
                totals.0 += 1;
                totals.1 += rep.source_rules;
                totals.2 += rep.rules;
                totals.3 += rep.dropped_variants;
                if st != LoadStatus::Complete {
                    failures.push(format!("{what} {tgi}: {st:?}"));
                }
            }
            None => {
                eprintln!(
                    "  {file} {tgi} {what}: {} bytes, sections {}, entries {}, ignored {}, rules {}",
                    data.len(),
                    rep.sections,
                    rep.entries,
                    rep.ignored,
                    rep.rules
                );
                // A CopyFrom whose rule is not (yet) in the map does nothing (§10.1); it is reported, with
                // whether the rule is defined later in the file.
                let mut hard = 0usize;
                for (sec, key, why) in &rules.int_rules.failures {
                    let later = why
                        .strip_prefix("CopyFrom: no rule ")
                        .and_then(|r| r.split_whitespace().next())
                        .map(|h| u32::from_str_radix(h.trim_start_matches("0x"), 16).unwrap_or(0))
                        .map(|id| rules.int_rules.get(id).is_some());
                    match later {
                        Some(l) => eprintln!("    [{sec}] {key}: {why}; defined later in the file: {l}"),
                        None => {
                            hard += 1;
                            eprintln!("    failure: [{sec}] {key}: {why}");
                        }
                    }
                }
                if what == "unknown" {
                    failures.push(format!("unknown instance {tgi}"));
                } else if inst == INTERSECTION_INSTANCE {
                    if hard != 0 {
                        failures.push(format!("{what} {tgi}: {hard} entries failed"));
                    }
                } else if rep.ignored != 0 {
                    failures.push(format!("{what} {tgi}: {} entries ignored", rep.ignored));
                }
            }
        }
    }
    eprintln!(
        "tile files: {}, source rules {}, rules after Transmogrify {}, variants dropped {}",
        totals.0, totals.1, totals.2, totals.3
    );

    // The §2.1 load order over the same bytes.
    let mut missing: Vec<String> = Vec::new();
    let (rules, reports) = NetworkRules::reload(&mut |id| {
        let inst = match id {
            ResourceId::Rule(i) => i,
            ResourceId::NetworkIni => NETWORK_INI_INSTANCE,
        };
        let got = found.get(&inst).map(|(_, _, d)| d.clone());
        if got.is_none() {
            missing.push(name_of(inst));
        }
        got
    });
    eprintln!("reload: {} resources asked for, missing: {missing:?}", reports.len());
    for (name, evs) in TYPE_NAMES.iter().zip(&rules.evaluators) {
        let counts: Vec<usize> = evs.iter().map(|e| e.rule_count()).collect();
        eprintln!("  {name:<14} evaluators {} rules {counts:?}", counts.len());
    }
    let solutions: usize = rules.solutions.values().map(|m| m.len()).sum();
    let overrides: usize = rules.rule_overrides.values().map(Vec::len).sum();
    eprintln!(
        "  multi-type solutions {solutions} over {} pairs, allowed overrides {:?}, rule overrides {overrides}, \
         verifiers {}, intersection rules {} (failures {})",
        rules.solutions.len(),
        rules.override_rows,
        rules.verifiers.len(),
        rules.int_rules.len(),
        rules.int_rules.failures.len()
    );
    assert!(
        failures.is_empty(),
        "rule resources that did not load cleanly: {failures:#?}"
    );
}
