// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The turbo clock (tooling, fc4_core::Turbo) switched off changes nothing:
//! every conformance vector passes again with turbo explicitly configured off
//! (with a frame budget and an end date that would matter if it were on) as
//! its first step. The vectors compare every post and delivery with its fake
//! clock time, so a changed number of clock reads would show up too.

use std::path::PathBuf;

use fc4_conformance::json::Json;

const OFF_STEP: &str = r#"{"op": "set_turbo", "on": false, "frame_ms": 7, "until": 2451600},"#;

fn with_turbo_off(text: &str) -> Option<String> {
    let k = text.find("\"steps\"")?;
    let b = k + text[k..].find('[')?;
    let out = format!("{}{}{}", &text[..=b], OFF_STEP, &text[b + 1..]);
    // The injected step must be the first top-level step (not a nested one).
    let v = Json::parse(&out).ok()?;
    let first = v.get("steps")?.as_array()?.first()?;
    (first.get("op")?.as_str()? == "set_turbo").then_some(out)
}

#[test]
fn every_vector_passes_with_turbo_configured_off() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vectors");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("vectors dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let mut ran = 0usize;
    let mut failures = Vec::new();
    for f in &files {
        let name = f.file_name().unwrap().to_string_lossy().into_owned();
        // The turbo vectors themselves configure turbo and check its fields.
        if name.starts_with("ht") {
            continue;
        }
        let text = std::fs::read_to_string(f).expect("read vector");
        let Some(patched) = with_turbo_off(&text) else {
            failures.push(format!("{name}: could not inject the turbo-off step"));
            continue;
        };
        if let Err(e) = fc4_conformance::run_vector_text(&patched) {
            failures.push(format!("{name}: {e}"));
        }
        ran += 1;
    }
    assert!(ran > 600, "only {ran} vectors ran");
    assert!(
        failures.is_empty(),
        "{} vector(s) failed with turbo off:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn injection_targets_the_top_level_steps() {
    let v = r#"{"format":1,"name":"x","steps":[{"op":"init"}]}"#;
    let out = with_turbo_off(v).expect("injected");
    assert!(out.contains(r#"[{"op": "set_turbo""#));
    assert!(fc4_conformance::run_vector_text(&out).is_ok());
}
