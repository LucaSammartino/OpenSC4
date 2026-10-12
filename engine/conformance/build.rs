// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Generates one `#[test]` per conformance vector so `cargo test` reports
//! each vector by name.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("vectors");
    println!("cargo:rerun-if-changed={}", dir.display());

    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("engine/conformance/vectors must exist")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();

    let mut out = String::new();
    for f in &files {
        let stem = f.file_stem().unwrap().to_string_lossy();
        let ident: String = stem
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();
        writeln!(
            out,
            "#[test]\nfn {ident}() {{\n    match fc4_conformance::run_vector_file(std::path::Path::new(r\"{}\")) {{\n        Ok(_) => {{}}\n        Err(e) => panic!(\"{{e}}\"),\n    }}\n}}\n",
            f.display()
        )
        .unwrap();
    }
    writeln!(out, "pub const VECTOR_COUNT: usize = {};", files.len()).unwrap();

    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("vectors_generated.rs");
    std::fs::write(dest, out).unwrap();
}
