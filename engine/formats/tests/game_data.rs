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
                // The factory read, with its type check (data-formats §4.1).
                TYPE_EXEMPLAR | TYPE_COHORT => match Exemplar::parse_record(e.tgi.type_id, &data) {
                    Ok(ex) => {
                        match (ex.kind, ex.encoding) {
                            (Kind::Cohort, _) => s.cohorts += 1,
                            (Kind::Exemplar, Encoding::Binary) => s.exemplars_binary += 1,
                            (Kind::Exemplar, Encoding::Text) => {
                                s.exemplars_text += 1;
                                eprintln!(
                                    "note {name} {}: text exemplar, {} properties",
                                    e.tgi,
                                    ex.properties().len()
                                );
                                // Lines with an empty value list: `reps` values are read and missing ones are 0
                                // (data-formats §4.3), so `Type:0:{}` is a scalar 0, not an empty array.
                                for line in data
                                    .split(|&b| b == b'\n')
                                    .filter(|l| l.windows(3).any(|w| w == b":{}"))
                                {
                                    eprintln!("note   empty value list: {}", String::from_utf8_lossy(line).trim_end());
                                }
                            }
                        }
                        if ex.encoding == Encoding::Binary {
                            if ex.to_binary() == data {
                                s.reencode_identical += 1;
                            } else {
                                s.reencode_different += 1;
                            }
                        }
                        for p in ex.properties() {
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
            ChainEnd::NotCohort(t) => {
                eprintln!("note: parent {t} is not a cohort");
                "parent not a cohort"
            }
            ChainEnd::Cycle(_) => "cycle",
            ChainEnd::TooDeep => "too deep",
        };
        *chains.entry(k.to_string()).or_default() += 1;
    }
    // The game's load (data-formats §4.2, §4.4): a parent that does not load as a cohort fails the exemplar.
    let loaded = store.load_all();

    eprintln!("game data: {}", dir.display());
    let mut total_failures = 0;
    eprintln!(
        "[load] .dat exemplars and cohorts loaded {}, failed to load {}",
        loaded.exemplars.len(),
        loaded.failures.len()
    );
    for (k, e) in loaded.failures.iter().take(20) {
        eprintln!("  FAIL load {k}: {e}");
    }
    total_failures += loaded.failures.len();
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

/// FSH and S3D counts of one sweep (graphics-formats §8.4).
#[derive(Default, Debug)]
struct GraphicsStats {
    fsh_records: usize,
    fsh_compressed: usize,
    fsh_entries: usize,
    fsh_uncreated_type: usize,
    fsh_codes: BTreeMap<u8, usize>,
    fsh_zero_filled: usize,
    fsh_size_not_record_length: usize,
    fsh_decoded_levels: usize,
    fsh_dxt1_a: BTreeMap<i16, usize>,
    fsh_mip_counts: BTreeMap<(u8, u8), usize>,
    fsh_attachments: BTreeMap<u8, usize>,
    fsh_directory_ids: BTreeMap<String, usize>,
    fsh_by_file: BTreeMap<String, usize>,
    s3d_records: usize,
    s3d_compressed: usize,
    s3d_other_group: usize,
    s3d_versions: BTreeMap<(u16, u16), usize>,
    vertex_blocks: usize,
    vertex_formats: BTreeMap<(u32, u32), usize>,
    index_blocks: usize,
    empty_index_blocks: usize,
    primitive_blocks: usize,
    primitives: usize,
    primitive_types: BTreeMap<u32, usize>,
    materials: usize,
    material_flags: BTreeMap<u16, usize>,
    stored_texture_counts: BTreeMap<usize, usize>,
    texture_ids_zero: usize,
    meshes_kept: usize,
    meshes_dropped: usize,
    multi_frame_models: usize,
    meshes_out_of_range: usize,
    prop_models: usize,
    regp_models: usize,
    s3d_by_file: BTreeMap<String, usize>,
    texture_steps: BTreeMap<String, usize>,
    shipped_checks: Vec<String>,
    failures: Vec<String>,
}

/// graphics-formats §8.4 (tests 42, 43) and the shipped records of tests 19 and 20: every FSH and S3D record of
/// the install's `.dat` files (for each key the first index entry of its package, data-formats §2.3) loads, with
/// the spec's counts. Decodes every level of every FSH frame in release builds (every 50th record otherwise).
#[test]
fn sweep_fsh_and_s3d() {
    use std::collections::HashSet;

    use fc4_formats::fsh::{Fsh, TextureFormat};
    use fc4_formats::keys::{resolve_texture, TextureStep};
    use fc4_formats::s3d::Model;
    use fc4_formats::tgi::{GROUP_S3D, GROUP_TEXTURES, TYPE_FSH, TYPE_FSH_UNCREATED, TYPE_S3D};
    use fc4_formats::Tgi;

    let Some(dir) = game_dir() else {
        eprintln!("game data not found (set FC4_GAME_DIR); skipping");
        return;
    };
    let mut files = Vec::new();
    collect(&dir, &mut files);
    files.retain(|p| group_of(p) == Some("dat"));
    let decode_all = !cfg!(debug_assertions);
    let mut g = GraphicsStats::default();
    let mut texture_keys: HashSet<Tgi> = HashSet::new();
    let mut texture_refs: Vec<(u32, u32)> = Vec::new();

    for path in &files {
        let name = path.strip_prefix(&dir).unwrap_or(path).display().to_string();
        let file = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let pkg = match Package::open(path) {
            Ok(p) => p,
            Err(Error::BadMagic { .. }) => continue,
            Err(e) => {
                g.failures.push(format!("{name}: open: {e}"));
                continue;
            }
        };
        let first = |e: &fc4_formats::IndexEntry| pkg.find(&e.tgi).is_some_and(|f| std::ptr::eq(f, e));
        g.fsh_uncreated_type += pkg.entries_of_type(TYPE_FSH_UNCREATED).count();
        for e in pkg.entries_of_type(TYPE_FSH).filter(|e| first(e)) {
            g.fsh_records += 1;
            *g.fsh_by_file.entry(file.clone()).or_default() += 1;
            if pkg.is_compressed(e) {
                g.fsh_compressed += 1;
            }
            let data = match pkg.read(e) {
                Ok(d) => d,
                Err(err) => {
                    g.failures.push(format!("{name} {}: read: {err}", e.tgi));
                    continue;
                }
            };
            let f = match Fsh::parse_record(TYPE_FSH, &data) {
                Ok(f) => f,
                Err(err) => {
                    g.failures.push(format!("{name} {}: FSH: {err}", e.tgi));
                    continue;
                }
            };
            texture_keys.insert(e.tgi);
            if f.data().len() != data.len() {
                g.fsh_size_not_record_length += 1;
            }
            *g.fsh_directory_ids
                .entry(String::from_utf8_lossy(&f.directory_id).into_owned())
                .or_default() += 1;
            for (i, fr) in f.frames.iter().enumerate() {
                g.fsh_entries += 1;
                *g.fsh_codes.entry(fr.header.code).or_default() += 1;
                *g.fsh_mip_counts
                    .entry((fr.header.code, fr.header.mip_count()))
                    .or_default() += 1;
                if fr.header.code == 0x60 {
                    *g.fsh_dxt1_a.entry(fr.header.a()).or_default() += 1;
                }
                if fr.zero_filled {
                    g.fsh_zero_filled += 1;
                }
                for a in f.attachments(i) {
                    *g.fsh_attachments.entry(a.code).or_default() += 1;
                }
            }
            if decode_all || g.fsh_records % 50 == 0 {
                for frame in 0..f.frames.len() {
                    for level in 0..f.levels.len() {
                        match f.decode(frame, level) {
                            Ok(img) if img.rgba.len() == (img.width * img.height * 4) as usize => {
                                g.fsh_decoded_levels += 1
                            }
                            Ok(_) => g.failures.push(format!("{name} {}: decoded size", e.tgi)),
                            Err(err) => g.failures.push(format!("{name} {}: decode: {err}", e.tgi)),
                        }
                    }
                }
            }
            // §8.1 test 19: the unpadded mips of SimCity_2.dat 0x10EC0000.
            if file == "SimCity_2.dat" && e.tgi == Tgi::new(TYPE_FSH, GROUP_TEXTURES, 0x10EC_0000) {
                let fr = &f.frames[0];
                let offsets: Vec<u64> = f.levels.iter().map(|l| l.offset).collect();
                let ok = fr.header.code == 0x60
                    && (f.width, f.height, f.mip_count(), fr.header.a()) == (128, 128, 7, 1)
                    && offsets == [0, 8192, 10240, 10752, 10880, 10912, 10928, 10944]
                    && fr.data_end - fr.pixel_start() == 10944
                    && fr.zero_filled;
                g.shipped_checks
                    .push(format!("test 19 {}: {}", e.tgi, if ok { "ok" } else { "FAIL" }));
                if !ok {
                    g.failures.push(format!("test 19: {fr:?} levels {offsets:?}"));
                }
            }
            // §8.1 test 20: the fallback texture of §4.1.
            if file == "SimCity_3.dat" && e.tgi == fc4_formats::keys::FALLBACK_TEXTURE {
                let header = [
                    0x53, 0x48, 0x50, 0x49, 0x40, 0x02, 0, 0, 1, 0, 0, 0, 0x47, 0x32, 0x36, 0x34,
                ];
                let ok = data.len() == 576
                    && data[..16] == header
                    && f.frames.len() == 1
                    && (f.code(), f.width, f.height, f.mip_count()) == (0x60, 32, 32, 0)
                    && f.format().0 == TextureFormat::Dxt1Rgb;
                g.shipped_checks
                    .push(format!("test 20 {}: {}", e.tgi, if ok { "ok" } else { "FAIL" }));
                if !ok {
                    g.failures.push(format!("test 20: {:?}", f.frames[0]));
                }
            }
        }
        for e in pkg.entries_of_type(TYPE_S3D).filter(|e| first(e)) {
            g.s3d_records += 1;
            *g.s3d_by_file.entry(file.clone()).or_default() += 1;
            if pkg.is_compressed(e) {
                g.s3d_compressed += 1;
            }
            if e.tgi.group != GROUP_S3D {
                g.s3d_other_group += 1;
            }
            let m = match pkg.read(e).and_then(|d| Model::parse_record(TYPE_S3D, &d)) {
                Ok(m) => m,
                Err(err) => {
                    g.failures.push(format!("{name} {}: S3D: {err}", e.tgi));
                    continue;
                }
            };
            *g.s3d_versions.entry((m.version.major, m.version.minor)).or_default() += 1;
            g.vertex_blocks += m.vertex_blocks.len();
            for v in &m.vertex_blocks {
                *g.vertex_formats.entry((v.format, v.stride)).or_default() += 1;
            }
            g.index_blocks += m.index_blocks.len();
            g.empty_index_blocks += m.index_blocks.iter().filter(|b| b.indices.is_empty()).count();
            g.primitive_blocks += m.primitive_blocks.len();
            for p in m.primitive_blocks.iter().flatten() {
                g.primitives += 1;
                *g.primitive_types.entry(p.kind).or_default() += 1;
            }
            g.materials += m.materials.len();
            for mat in &m.materials {
                *g.material_flags.entry(mat.flags).or_default() += 1;
                *g.stored_texture_counts.entry(mat.textures.len()).or_default() += 1;
                for t in &mat.textures {
                    if t.id == 0 {
                        g.texture_ids_zero += 1;
                    } else {
                        texture_refs.push((e.tgi.group, t.id));
                    }
                }
            }
            g.meshes_kept += m.anim.meshes.len();
            g.meshes_dropped += m.anim.dropped;
            if m.anim.frame_count > 1 {
                g.multi_frame_models += 1;
            }
            for f in 0..usize::from(m.anim.frame_count) {
                for mesh in m.frame(f).unwrap_or_default() {
                    if !mesh.ranges_fit() {
                        g.meshes_out_of_range += 1;
                    }
                }
            }
            if !m.props.is_empty() {
                g.prop_models += 1;
            }
            if !m.regions.is_empty() {
                g.regp_models += 1;
            }
        }
    }
    // §4.1 over every material texture ID, against every FSH key that loaded.
    for (group, id) in &texture_refs {
        let step = resolve_texture(*group, *id, |k| texture_keys.contains(k)).map(|b| b.step);
        let k = match step {
            Some(TextureStep::SharedGroup) => "shared group",
            Some(TextureStep::ModelGroup) => "model group",
            Some(TextureStep::Fallback) => "fallback",
            None => "none",
        };
        *g.texture_steps.entry(k.into()).or_default() += 1;
    }

    eprintln!("game data: {}", dir.display());
    eprintln!(
        "[fsh] records {} (compressed {}), entries {}, type 0x7AB50E45 entries {}, size != record length {}, \
         zero-filled entries {}, decoded levels {}",
        g.fsh_records,
        g.fsh_compressed,
        g.fsh_entries,
        g.fsh_uncreated_type,
        g.fsh_size_not_record_length,
        g.fsh_zero_filled,
        g.fsh_decoded_levels
    );
    let hexmap = |m: &BTreeMap<u8, usize>| m.iter().map(|(k, v)| format!("{k:#04X}:{v}")).collect::<Vec<_>>();
    eprintln!("[fsh] by file {:?}", g.fsh_by_file);
    eprintln!(
        "[fsh] codes {:?}; attachments {:?}",
        hexmap(&g.fsh_codes),
        hexmap(&g.fsh_attachments)
    );
    eprintln!(
        "[fsh] directory IDs {:?}; DXT1 A {:?}",
        g.fsh_directory_ids, g.fsh_dxt1_a
    );
    eprintln!(
        "[fsh] (code, M > 0) {:?}",
        g.fsh_mip_counts
            .iter()
            .filter(|((_, m), _)| *m > 0)
            .map(|((c, m), n)| format!("{c:#04X}/{m}:{n}"))
            .collect::<Vec<_>>()
    );
    eprintln!(
        "[s3d] records {} (compressed {}, outside group 0xBADB57F1 {}), versions {:?}, by file {:?}",
        g.s3d_records, g.s3d_compressed, g.s3d_other_group, g.s3d_versions, g.s3d_by_file
    );
    eprintln!(
        "[s3d] vertex blocks {} {:?}; index blocks {} ({} empty); primitive blocks {}, primitives {} {:?}",
        g.vertex_blocks,
        g.vertex_formats
            .iter()
            .map(|((f, s), n)| format!("{f:#X}/{s}:{n}"))
            .collect::<Vec<_>>(),
        g.index_blocks,
        g.empty_index_blocks,
        g.primitive_blocks,
        g.primitives,
        g.primitive_types
    );
    eprintln!(
        "[s3d] materials {}, flags {:?}, stored texture counts {:?}, texture IDs 0: {}; texture lookup {:?}",
        g.materials,
        g.material_flags
            .iter()
            .map(|(f, n)| format!("{f:#06X}:{n}"))
            .collect::<Vec<_>>(),
        g.stored_texture_counts,
        g.texture_ids_zero,
        g.texture_steps
    );
    eprintln!(
        "[s3d] meshes kept {}, dropped {}, out of range {}; models with F > 1: {}, PROP {}, REGP {}",
        g.meshes_kept, g.meshes_dropped, g.meshes_out_of_range, g.multi_frame_models, g.prop_models, g.regp_models
    );
    for c in &g.shipped_checks {
        eprintln!("[shipped] {c}");
    }
    for f in g.failures.iter().take(20) {
        eprintln!("  FAIL {f}");
    }
    assert!(
        g.failures.is_empty(),
        "{} failures, see FAIL lines above",
        g.failures.len()
    );

    // The spec's counts (graphics-formats §2.13, §3.14, §8.4) for the SimCity 4 Deluxe (GOG) install.
    assert_eq!(g.fsh_records, 40_549, "FSH records");
    assert_eq!(g.fsh_compressed, 36_177, "compressed FSH records");
    assert_eq!(g.fsh_entries, 45_387, "FSH entries");
    assert_eq!(g.fsh_uncreated_type, 0);
    assert_eq!(g.fsh_size_not_record_length, 0);
    let codes: Vec<(u8, usize)> = g.fsh_codes.iter().map(|(c, n)| (*c, *n)).collect();
    assert_eq!(
        codes,
        [(0x60, 19_357), (0x61, 25_957), (0x7B, 24), (0x7D, 40), (0x7F, 9)]
    );
    assert_eq!(g.fsh_zero_filled, 167, "DXT1 entries that need the §2.8 zero-fill");
    assert_eq!(g.shipped_checks.len(), 2, "tests 19 and 20 found their records");
    assert_eq!(g.s3d_records, 42_915, "S3D records");
    assert_eq!(g.s3d_compressed, 42_868, "compressed S3D records");
    assert_eq!(g.s3d_other_group, 0);
    assert_eq!(g.vertex_blocks, 167_070);
    assert_eq!(g.index_blocks, 155_661);
    assert_eq!(g.empty_index_blocks, 15_311);
    assert_eq!(g.primitive_blocks, 154_046);
    assert_eq!(g.primitives, 138_735);
    assert_eq!(g.materials, 155_827);
    assert_eq!(g.meshes_kept + g.meshes_dropped, 155_827);
    assert_eq!(g.meshes_dropped, 71);
    assert_eq!(g.meshes_out_of_range, 0);
    assert_eq!(texture_refs.len(), 155_822, "non-zero texture IDs");
    assert_eq!(g.texture_ids_zero, 2);
    assert_eq!(
        g.texture_steps.get("shared group"),
        Some(&155_822),
        "every ID resolves at step 1"
    );
    assert_eq!((g.prop_models, g.regp_models), (58, 1_810));
}
