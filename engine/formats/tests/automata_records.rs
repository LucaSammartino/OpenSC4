// SPDX-License-Identifier: GPL-3.0-or-later
//! The save sweep's automata records (`docs/specs/automata.md` §14 test 12): for each city save, every entry of
//! the class records of components 40, 41 and 43–46 and of the objects they reference (generators, attractors,
//! pedestrians, vehicles, patrol cars, trailers) is read with `fc4_core::automata`, must parse to its exact end, and
//! is written back and compared with the saved bytes.
//!
//! References go through as host references whose token is `classID << 32 | objectID`, written back as the same
//! object ID and class, so a record needs no other record of the save. The classes behind the references of the
//! managers' and the controller manager's lists are counted, and a referenced class without a record here (an
//! aircraft or watercraft automaton, Q4) is reported.
//!
//! The saves are found as `tests/save_records.rs` finds them (`FC4_SAVES_DIR`, else `FC4_GAME_DIR`, else the default
//! GOG path); files with the same content are checked once; the test passes with a note when no save is found.
//!
//! `cargo test -p fc4-formats --release --test automata_records -- --nocapture`

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use fc4_core::automata::{
    self, AutomataManager, Automaton, Controller, ControllerManager, ManagerKind, ManagerPart, NoControllerHost,
    ScriptSystem,
};
use fc4_core::store::{crc32_mpeg2, CRC_LIMIT, STORE_GROUP};
use fc4_core::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue, SaveObject};
use fc4_formats::tgi::Tgi;
use fc4_formats::{Package, Source};

const DEFAULT_DIR: &str = r"C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition";

/// Resolves every reference to a host token `class << 32 | id`, and counts the classes.
#[derive(Default)]
struct Tokens {
    classes: BTreeMap<u32, usize>,
}

impl RefResolver for Tokens {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        *self.classes.entry(class_id).or_insert(0) += 1;
        Some(RefValue::Host {
            token: (u64::from(class_id) << 32) | u64::from(object_id),
            class_id,
        })
    }
}

/// A byte writer that writes a host reference back as its object ID and class.
struct RawOut(ByteWriter);

impl OutStream for RawOut {
    fn put_u8(&mut self, v: u8) -> bool {
        self.0.put_u8(v)
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.0.put_u16(v)
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.0.put_u32(v)
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.0.put_u64(v)
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.0.put_f32(v)
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.0.put_f64(v)
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.0.put_bytes(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.0.put_str(v)
    }
    fn put_ref(&mut self, r: &RefValue) -> bool {
        match r {
            RefValue::Null => self.0.put_u32(0),
            RefValue::Host { token, class_id } => self.0.put_u32(*token as u32) && self.0.put_u32(*class_id),
            RefValue::Object(_) => {
                self.0.set_error();
                false
            }
        }
    }
    fn error(&self) -> i32 {
        self.0.error()
    }
    fn set_error(&mut self) {
        self.0.set_error()
    }
}

/// One object of a swept class.
enum Obj {
    Script(ScriptSystem),
    ControllerManager(ControllerManager),
    Manager(AutomataManager),
    Saved(Rc<dyn SaveObject>),
}

fn object_for(class: u32) -> Option<Obj> {
    if class == automata::CLSID_SCRIPT_SYSTEM {
        return Some(Obj::Script(ScriptSystem::new()));
    }
    if class == automata::CLSID_CONTROLLER_MANAGER {
        return Some(Obj::ControllerManager(ControllerManager::new(Box::new(
            NoControllerHost,
        ))));
    }
    if let Some(k) = ManagerKind::from_class(class) {
        return Some(Obj::Manager(AutomataManager::new(k)));
    }
    if let Some(c) = Controller::for_class(class) {
        return Some(Obj::Saved(c));
    }
    Automaton::for_class(class).map(|a| Obj::Saved(a as Rc<dyn SaveObject>))
}

#[derive(Default)]
struct Stats {
    records: usize,
    entries: usize,
    identical: usize,
    /// Entries whose re-encode differs from the save only because a state's
    /// `+0x0C` was renormalised (see [`renormalised_index_only`]).
    renormalised: usize,
    versions: BTreeMap<u16, usize>,
    sizes: BTreeMap<usize, usize>,
}

#[derive(Default)]
struct Report {
    by_class: BTreeMap<u32, Stats>,
    /// Classes referenced from the managers' and the controller manager's lists.
    listed: BTreeMap<u32, usize>,
    state_types: BTreeMap<u32, usize>,
    base_versions: BTreeMap<u16, usize>,
    template_names: BTreeMap<String, usize>,
    failures: Vec<String>,
}

/// Whether two encodings of one automaton parse to the same record.
///
/// `+0x0C` (the state stack index) is recomputed by `PushState` / `PopState`
/// but stored as read by Read (automata.md §12 quirk 10, §14 test 29), so a
/// re-save keeps the original's stale values: 14 of the corpus's states sit
/// alone on their stack carrying `+0x0C == 1`. This tolerance (same length,
/// same parsed record) would accept a re-encode that changed only the index;
/// the sweep asserts it is never needed. Anything else is a real failure.
fn renormalised_index_only(class: u32, ours: &[u8], saved: &[u8]) -> bool {
    if ours.len() != saved.len() {
        return false;
    }
    let parse = |payload: &[u8]| -> Option<automata::AutomatonRecord> {
        let obj = Automaton::for_class(class)?;
        let mut tokens = Tokens::default();
        let mut r = ByteReader::with_resolver(payload, &mut tokens);
        if !obj.read(&mut r) || r.remaining() != 0 {
            return None;
        }
        Some(obj.record())
    };
    matches!((parse(ours), parse(saved)), (Some(a), Some(b)) if a == b)
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

fn class_of(r: &RefValue) -> Option<u32> {
    match r {
        RefValue::Host { class_id, .. } => Some(*class_id),
        _ => None,
    }
}

/// Reads one payload and writes it back: `Ok(bytes)` or what failed.
fn round_trip(class: u32, payload: &[u8], rep: &mut Report) -> Result<Vec<u8>, String> {
    let Some(mut obj) = object_for(class) else {
        return Err("no object for the class".into());
    };
    let mut tokens = Tokens::default();
    let mut r = ByteReader::with_resolver(payload, &mut tokens);
    let ok = match &mut obj {
        Obj::Script(s) => s.read(&mut r),
        Obj::ControllerManager(c) => c.read(&mut r),
        Obj::Manager(m) => m.read(&mut r),
        Obj::Saved(o) => o.read(&mut r),
    };
    let left = r.remaining();
    if !ok {
        return Err(format!("Read failed ({left} of {} bytes left)", payload.len()));
    }
    if left != 0 {
        return Err(format!("{left} bytes left after Read"));
    }
    // Census.
    match &obj {
        Obj::ControllerManager(c) => {
            for e in &c.record().entries {
                for x in &e.controllers {
                    if let Some(k) = class_of(x) {
                        *rep.listed.entry(k).or_insert(0) += 1;
                    }
                }
            }
        }
        Obj::Manager(m) => {
            for x in m.automata() {
                if let Some(k) = class_of(x) {
                    *rep.listed.entry(k).or_insert(0) += 1;
                }
            }
            if let ManagerPart::Aircraft { pairs, .. } = &m.record().part {
                for (_, a) in pairs {
                    if let Some(k) = class_of(a) {
                        *rep.listed.entry(k).or_insert(0) += 1;
                    }
                }
            }
        }
        Obj::Saved(o) => {
            if let Ok(a) = o.clone().as_any_rc().downcast::<Automaton>() {
                let rec = a.record();
                *rep.base_versions.entry(rec.base().version).or_insert(0) += 1;
                for s in &rec.base().states {
                    *rep.state_types.entry(s.state_type).or_insert(0) += 1;
                }
            } else if let Ok(c) = o.clone().as_any_rc().downcast::<Controller>() {
                *rep.template_names.entry(c.template_name()).or_insert(0) += 1;
            }
        }
        Obj::Script(_) => {}
    }
    let mut out = RawOut(ByteWriter::new());
    let wrote = match &obj {
        Obj::Script(s) => s.write(&mut out),
        Obj::ControllerManager(c) => c.write(&mut out),
        Obj::Manager(m) => m.write(&mut out),
        Obj::Saved(o) => o.write(&mut out),
    };
    if !wrote {
        return Err("Write failed".into());
    }
    Ok(out.0.into_bytes())
}

/// One class record split into its entries (foundations §3.3), with the CRC check of §3.4.
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
            return Err(format!("entry at byte {p}: size {size} outside the record"));
        }
        let span = (size - 8).min(CRC_LIMIT);
        if crc32_mpeg2(&record[p + 8..p + 8 + span]) != crc {
            return Err(format!("entry at byte {p} (object {id:#x}): CRC mismatch"));
        }
        out.push((id, &record[p + 12..p + size]));
        p += size;
    }
    Ok(out)
}

fn check_package<S: Source>(name: &str, pkg: &Package<S>, rep: &mut Report) {
    for class in automata::RECORD_CLASSES {
        let Some(e) = pkg.find(&Tgi::new(class, STORE_GROUP, 0)) else {
            continue;
        };
        let record = match pkg.read(e) {
            Ok(r) => r,
            Err(err) => {
                rep.failures.push(format!("{name}: class {class:#010x}: {err}"));
                continue;
            }
        };
        let entries = match split_entries(&record) {
            Ok(v) => v,
            Err(err) => {
                rep.failures.push(format!("{name}: class {class:#010x}: {err}"));
                continue;
            }
        };
        rep.by_class.entry(class).or_default().records += 1;
        for (id, payload) in entries {
            let result = round_trip(class, payload, rep);
            let s = rep.by_class.entry(class).or_default();
            s.entries += 1;
            if payload.len() >= 2 {
                *s.versions
                    .entry(u16::from_le_bytes([payload[0], payload[1]]))
                    .or_insert(0) += 1;
            }
            *s.sizes.entry(payload.len()).or_insert(0) += 1;
            match result {
                Ok(ours) if ours == payload => s.identical += 1,
                Ok(ours) if renormalised_index_only(class, &ours, payload) => s.renormalised += 1,
                Ok(ours) => rep.failures.push(format!(
                    "{name}: class {class:#010x} object {id:#x}: {}",
                    first_difference(&ours, payload)
                )),
                Err(why) => rep
                    .failures
                    .push(format!("{name}: class {class:#010x} object {id:#x}: {why}")),
            }
        }
    }
}

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

fn label(class: u32) -> &'static str {
    match class {
        automata::CLSID_SCRIPT_SYSTEM => "script system",
        automata::CLSID_CONTROLLER_MANAGER => "controller manager",
        automata::CLSID_VEHICLE_MANAGER => "vehicle manager",
        automata::CLSID_PEDESTRIAN_MANAGER => "pedestrian manager",
        automata::CLSID_AIRCRAFT_MANAGER => "aircraft manager",
        automata::CLSID_WATERCRAFT_MANAGER => "watercraft manager",
        automata::CLSID_GENERATOR => "generator",
        automata::CLSID_ATTRACTOR => "attractor",
        automata::CLSID_PEDESTRIAN => "pedestrian",
        automata::CLSID_VEHICLE => "vehicle",
        automata::CLSID_PATROL_CAR => "patrol car",
        automata::CLSID_TRAILER => "trailer",
        _ => "?",
    }
}

fn summary(m: &BTreeMap<impl std::fmt::Display, usize>, hex: bool, limit: usize) -> String {
    let mut v: Vec<String> = m
        .iter()
        .take(limit)
        .map(|(k, n)| if hex { format!("{k} {n}") } else { format!("{k}:{n}") })
        .collect();
    if m.len() > limit {
        v.push(format!("… ({} more)", m.len() - limit));
    }
    v.join(", ")
}

#[test]
fn sweep_automata_records() {
    let Some(dir) = saves_dir() else {
        println!("automata_records: no saves found (set FC4_SAVES_DIR or FC4_GAME_DIR); skipped");
        return;
    };
    let mut files = Vec::new();
    collect_saves(&dir, &mut files);
    let mut rep = Report::default();
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
        "automata_records: {} ({} .sc4 files, {} distinct)",
        dir.display(),
        files.len(),
        distinct
    );
    for (class, s) in &rep.by_class {
        let versions: BTreeMap<String, usize> = s.versions.iter().map(|(v, n)| (format!("v{v}"), *n)).collect();
        println!(
            "  {} ({class:#010x}): {} records, {} entries, {} byte-identical, {} index-renormalised; versions {}; sizes {}",
            label(*class),
            s.records,
            s.entries,
            s.identical,
            s.renormalised,
            summary(&versions, true, 8),
            summary(&s.sizes, false, 6)
        );
    }
    let listed: BTreeMap<String, usize> = rep.listed.iter().map(|(c, n)| (format!("{c:#010x}"), *n)).collect();
    println!("  classes listed by the managers: {}", summary(&listed, true, 20));
    let st: BTreeMap<String, usize> = rep.state_types.iter().map(|(c, n)| (format!("{c:#x}"), *n)).collect();
    println!("  automaton state types: {}", summary(&st, true, 40));
    let bv: BTreeMap<String, usize> = rep.base_versions.iter().map(|(c, n)| (format!("v{c}"), *n)).collect();
    println!("  automaton base versions: {}", summary(&bv, true, 10));
    println!("  controller templates: {}", summary(&rep.template_names, true, 30));
    for c in rep.listed.keys() {
        if !automata::RECORD_CLASSES.contains(c) {
            rep.failures
                .push(format!("a listed class without a record here: {c:#010x}"));
        }
    }
    for f in rep.failures.iter().take(40) {
        println!("FAILURE {f}");
    }
    println!("automata_records: {} failures", rep.failures.len());
    // [AT2] The corpus has 14 states carrying a stale `+0x0C` (a state alone on
    // its stack still holding the index it had when another state sat below
    // it). Read stores the streamed value unchanged and only PushState /
    // PopState recompute it (automata.md §12 quirk 10 and §14 test 29, as
    // corrected by A1), so those 14 re-save byte-identically: none may need the
    // index-only tolerance. Was 14 while Read re-derived the index.
    let renormalised: usize = rep.by_class.values().map(|s| s.renormalised).sum();
    println!("automata_records: {renormalised} index-renormalised entries");
    assert_eq!(
        renormalised, 0,
        "a re-save changed a state's `+0x0C`: Read must keep the streamed stack index (automata.md §12 quirk 10)"
    );
    assert!(
        rep.failures.is_empty(),
        "{} failures (see the output)",
        rep.failures.len()
    );
}

// ---- synthetic records (no corpus needed) ---------------------------------------------------------------------

fn hex(s: &str) -> Vec<u8> {
    let c: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    (0..c.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&c[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn synthetic_records_round_trip_through_the_sweep() {
    let mut rep = Report::default();
    let cm = hex("0100 1893010A29F9E7A7 000000000000 000000000000 00000000 00000000");
    assert_eq!(
        round_trip(automata::CLSID_CONTROLLER_MANAGER, &cm, &mut rep).unwrap(),
        cm
    );
    let vm = hex("0200 0400 50010000 00 00 00000000 01000000 01000000");
    assert_eq!(round_trip(automata::CLSID_VEHICLE_MANAGER, &vm, &mut rep).unwrap(), vm);
    let mut cut = vm.clone();
    cut.truncate(20);
    assert!(round_trip(automata::CLSID_VEHICLE_MANAGER, &cut, &mut rep).is_err());
    let mut long = vm;
    long.push(0);
    assert!(round_trip(automata::CLSID_VEHICLE_MANAGER, &long, &mut rep)
        .unwrap_err()
        .contains("1 bytes left"));
}
