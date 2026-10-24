// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The flora census shadow (fc4.h "flora shadow"; tooling, no spec
//! behaviour): a same-process differential of the ORIGINAL flora simulator
//! against FreeCity4's rules. The host reads the game; this module compares.
//!
//! Two comparisons:
//!
//! 1. **The registry after load** ([`fc4_flora_shadow_registry`], once per
//!    city): the core builds a registry from the same flora exemplars, in the
//!    same resource-manager order (§3.2, §3.3, §6.11: [`FloraRecord::from_exemplar`],
//!    [`FloraRegistry::add`], then the key-0 default record), and compares it
//!    with the original's type array (slot 9), `IsFloraTypeValid(0)` (slot
//!    11) and each record the original returns from `GetStaticFloraData`
//!    (slot 7, read from its block, docs/abi/layouts.md): next type, seed
//!    type, water, the 14 floats (bitwise), cluster radius and height, the
//!    256 table bytes, the unnamed u32, flags bits 0-2, the family list, and
//!    the plop cost when the exemplar has property `0x49CAC341`. Not
//!    compared: flags bits 3-7 and the key-0 record's flags and plop cost
//!    (uninitialised in the original, §3.2), the family map and the type sets
//!    of slots 8, 10, 13 and 14 (they make the game allocate into our
//!    containers; the families are compared per record instead).
//! 2. **The census around each monthly pass** ([`fc4_flora_shadow_month`]):
//!    every flora occupant of the city before and after the original's
//!    SimNewMonth (type, state, birth, last seeding date, position, box, flag
//!    1), and the original's record before and after (its own Write). The
//!    blocks the pass visited are recovered from the record's iterator
//!    (before: `table[index..]`; after a wrap, the reshuffled table's
//!    `[..index]`), the occupants of each visited block found as the game's
//!    cell query finds them (§6.7.1, §6.4.2, FQ15: listed in one of the
//!    block's 64 m occupant-manager cells, from its box with the 0.1 m
//!    inset, and overlapping the block's world box strictly; [choice] (AQ9:
//!    no original behaviour) the listing is taken from the census box before
//!    the pass, as flora boxes change only through `MoveOccupant`, which
//!    lists them again: the original lists an occupant from its box at its
//!    last insert or move, the same box), and
//!    §6.7.2 applied visit by visit with
//!    [`fc4_core::flora::predict_visit`] (the deterministic part: rebirth,
//!    stage, removal, last seeding date), with the original's own records,
//!    `today` = data1, the original's propagation flag, the simulator's day
//!    number, and the score from the moisture type and terrain altitude the
//!    host read at each occupant before the pass, the sea level and our
//!    elevation scale (the tuning exemplar's `0xC9DBDD02`, §7, FQ7: the
//!    original's record does not save it).
//!    Compared: state, birth date, last seeding date, type and position of
//!    every occupant present before and after; predicted removals against
//!    absence; occupants outside the visited blocks must not change; the
//!    record's scalars must not change but `today` (= data1) and the
//!    iterator (must agree with `blocksPerMonth` visits). Not compared (the
//!    random part, §8): the seeds dropped (positions, acceptance, cluster
//!    merges): occupants that vanish while predicted present are counted as
//!    "vanished" (merged into a seedling's cluster), new occupants as "new";
//!    the generator state; wild seeds. A pass that wraps the iterator twice
//!    is not compared.

use std::collections::{BTreeMap, HashMap};
use std::ffi::c_void;
use std::fmt::Write as _;

use fc4_core::flora::{
    cell_query_box, elevation_index, manager_listing, manager_query_returns, predict_visit, prop, score_from_table,
    FloraOccupant, FloraRecord, FloraRegistry, Vec3, MAX_BLOCKS_PER_MONTH,
};
use fc4_core::grid::{CellIterator, IteratorState, MODE_TABLE};
use fc4_core::julian::civil_from_julian;
use fc4_core::property::PropertyHolder;
use fc4_core::props::{f32_of, u32_of};
use fc4_core::stream::{ByteReader, InStream};

use crate::flora::{fc4_flora_exemplar_visitor, fc4_flora_record};
use crate::guard;
use crate::hostgrid::take_props;
use crate::props::fc4_props;
use crate::stream::fc4_bytes_sink;

/// The flora exemplar property of the plop cost (§3.3).
const PROP_PLOP_COST: u32 = 0x49CA_C341;
/// The code default elevation scale (§7).
const DEFAULT_ELEVATION_SCALE: f32 = 0.08;
/// Differences listed per line.
const LISTED: usize = 6;

// ---- ABI structs ------------------------------------------------------------------------------------

/// One flora occupant of the census (fc4.h `fc4_flora_census_entry`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_flora_census_entry {
    /// Its identity (the host keeps it alive across the delivery).
    pub token: u64,
    pub flora_type: u32,
    pub birth_date: u32,
    pub last_seeding_date: u32,
    pub state: i8,
    /// `IsFlagSet(1)`.
    pub in_manager: u8,
    /// Answers `cISC4FloraOccupant` (else the values are 0).
    pub is_flora: u8,
    pub reserved: u8,
    pub pos: [f32; 3],
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    /// The score inputs at its position, read before the pass: the weather's
    /// `GetMoistureType(x, z)` and the terrain's `GetAltitude(x, z)`.
    pub moisture: u32,
    pub altitude: f32,
    pub reserved2: u32,
}

/// One monthly delivery (fc4.h `fc4_flora_shadow_month_in`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_flora_shadow_month_in {
    pub struct_size: u32,
    /// The message's data1 (`today`).
    pub julian: u32,
    /// `GetSimDateNumber()` (read before the pass).
    pub sim_date: u32,
    /// The terrain's `GetSeaLevel()`.
    pub sea_level: f32,
    /// The original's record (its own Write) before and after the pass
    /// (NULL = not captured).
    pub before: *const u8,
    pub before_len: usize,
    pub after: *const u8,
    pub after_len: usize,
    pub census_before: *const fc4_flora_census_entry,
    pub before_count: usize,
    pub census_after: *const fc4_flora_census_entry,
    pub after_count: usize,
    /// The original's records (from its blocks), any order, by `type_id`.
    pub records: *const fc4_flora_record,
    pub record_count: usize,
}

/// Totals since creation (fc4.h `fc4_flora_shadow_stats`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_flora_shadow_stats {
    pub struct_size: u32,
    /// Monthly deliveries compared / not compared (no capture, iterator).
    pub months: u32,
    pub incomplete: u32,
    /// The last delivery's mismatches (-1 = not compared).
    pub last_mismatches: i32,
    pub registry_checks: u32,
    pub registry_mismatches: u32,
    pub occupants: u32,
    pub visited: u32,
    pub matched: u32,
    pub mismatched: u32,
    pub vanished: u32,
    pub appeared: u32,
    pub predicted_reborn: u32,
    pub predicted_aged: u32,
    pub predicted_removed: u32,
    pub predicted_seeded: u32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_flora_census_entry>() == 72);
    assert!(std::mem::size_of::<fc4_flora_shadow_stats>() == 64);
    assert!(
        std::mem::size_of::<fc4_flora_shadow_month_in>() == if cfg!(target_pointer_width = "32") { 56 } else { 96 }
    );
};

// ---- the original's record --------------------------------------------------------------------------

/// The fields of §10.1 the shadow reads (1-15 and the burnt count).
#[derive(Clone, Debug, PartialEq)]
struct Record {
    enabled: bool,
    propagation: bool,
    forced: bool,
    preserve: bool,
    size_x: f32,
    size_z: f32,
    cells_x: i32,
    cells_z: i32,
    today: u32,
    period: u32,
    blocks_per_month: u32,
    wild: u32,
    keep: u8,
    iterator: CellIterator,
    burnt: u32,
}

fn parse_record(bytes: &[u8]) -> Result<Record, &'static str> {
    let mut r = ByteReader::new(bytes);
    if r.get_u16() != Some(2) {
        return Err("the record is not version 2");
    }
    let short = "the record is short";
    let mut b = || r.get_u8().map(|v| v != 0);
    let (enabled, propagation, forced, preserve) = (b(), b(), b(), b());
    let rec = (|| {
        Some(Record {
            enabled: enabled?,
            propagation: propagation?,
            forced: forced?,
            preserve: preserve?,
            size_x: r.get_f32()?,
            size_z: r.get_f32()?,
            cells_x: r.get_i32()?,
            cells_z: r.get_i32()?,
            today: r.get_u32()?,
            period: r.get_u32()?,
            blocks_per_month: r.get_u32()?,
            wild: r.get_u32()?,
            keep: r.get_u8()?,
            iterator: CellIterator::read(&mut r)?,
            burnt: {
                // the generator (16 bytes and a double), then the list count
                r.get_bytes(20)?;
                r.get_u32()?
            },
        })
    })();
    rec.ok_or(short)
}

/// The blocks a pass of `k` blocks visited, from the iterator before and
/// after (§6.7.1); `wrapped` when it reset.
fn visited_blocks(before: &CellIterator, after: &CellIterator, k: u32) -> Result<(Vec<(i32, i32)>, bool), String> {
    if before.mode != MODE_TABLE || after.mode != MODE_TABLE {
        return Err("the iterator is not in table mode".into());
    }
    let k = k.min(MAX_BLOCKS_PER_MONTH) as usize;
    let len = before.table.len();
    let start = if before.state == IteratorState::Exhausted {
        len
    } else {
        usize::try_from(before.index)
            .map_err(|_| "negative iterator index")?
            .min(len)
    };
    let first = (len - start).min(k);
    let mut entries: Vec<u16> = before.table[start..start + first].to_vec();
    let rest = k - first;
    let after_index = usize::try_from(after.index).map_err(|_| "negative iterator index")?;
    let wrapped = rest > 0;
    if wrapped {
        if rest > after.table.len() {
            return Err(format!(
                "the pass wrapped twice ({k} blocks, {} in the table)",
                after.table.len()
            ));
        }
        if after_index != rest {
            return Err(format!(
                "after a wrap the iterator is at {after_index}, expected {rest}"
            ));
        }
        entries.extend_from_slice(&after.table[..rest]);
    } else if after.table != before.table || after_index != start + first {
        return Err(format!(
            "the iterator moved from {start} to {after_index} without a wrap, expected {}",
            start + first
        ));
    }
    Ok((
        entries
            .iter()
            .map(|&e| (i32::from(e >> 8), i32::from(e & 0xFF)))
            .collect(),
        wrapped,
    ))
}

/// The world box of block `(bx, bz)` (cells `2bx..2bx+1`, `2bz..2bz+1`),
/// as the occupant manager's cell query builds it (§6.10.4).
fn block_box(bx: i32, bz: i32) -> (Vec3, Vec3) {
    let (x0, z0) = (bx.wrapping_mul(2), bz.wrapping_mul(2));
    cell_query_box((x0, z0, x0.wrapping_add(1), z0.wrapping_add(1)))
}

/// Whether the block query `b` returns the occupant (§6.4.2, FQ15): listed
/// in one of the query's 64 m cells and strictly overlapping.
fn in_block(e: &fc4_flora_census_entry, b: &(Vec3, Vec3)) -> bool {
    let occ = (e.bbox_min, e.bbox_max);
    manager_query_returns(manager_listing(occ.0, occ.1), occ, *b)
}

// ---- the shadow -------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_flora_shadow`).
#[repr(C)]
pub struct fc4_flora_shadow {
    _private: [u8; 0],
}

struct Shadow {
    elevation_scale: f32,
    stats: fc4_flora_shadow_stats,
}

fn shadow<'a>(p: *mut fc4_flora_shadow) -> Option<&'a mut Shadow> {
    // SAFETY: documented precondition: NULL or a live fc4_flora_shadow*.
    unsafe { (p as *mut Shadow).as_mut() }
}

fn emit(sink: fc4_bytes_sink, ctx: *mut c_void, line: &str) {
    if let Some(f) = sink {
        // SAFETY: the host's sink; the line outlives the call.
        unsafe { f(ctx, line.as_ptr(), line.len()) };
    }
}

fn date(julian: u32) -> String {
    let d = civil_from_julian(julian as i32);
    format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
}

/// SAFETY: `p` NULL or valid for `n` values.
unsafe fn slice<'a, T>(p: *const T, n: usize) -> &'a [T] {
    if p.is_null() || n == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(p, n)
    }
}

/// Creates a shadow (code default elevation scale, no totals).
#[no_mangle]
pub extern "C" fn fc4_flora_shadow_create() -> *mut fc4_flora_shadow {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(Shadow {
            elevation_scale: DEFAULT_ELEVATION_SCALE,
            stats: fc4_flora_shadow_stats {
                struct_size: std::mem::size_of::<fc4_flora_shadow_stats>() as u32,
                last_mismatches: -1,
                ..fc4_flora_shadow_stats::default()
            },
        })) as *mut fc4_flora_shadow
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_flora_shadow_destroy(p: *mut fc4_flora_shadow) {
    if !p.is_null() {
        guard((), || drop(Box::from_raw(p as *mut Shadow)));
    }
}

/// The tuning exemplar (`tuning`: NEW, taken; NULL = missing): the elevation
/// scale `16 / v` of property `0xC9DBDD02` (§7, FQ7), else 0.08. Returns it.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_shadow_set_tuning(p: *mut fc4_flora_shadow, tuning: *mut fc4_props) -> f32 {
    let t = take_props(tuning);
    let Some(s) = shadow(p) else { return 0.0 };
    guard(0.0, || {
        s.elevation_scale = t
            .as_deref()
            .and_then(|h| f32_of(h, prop::ELEVATION_SCALE))
            .map_or(DEFAULT_ELEVATION_SCALE, |v| (16.0 / f64::from(v)) as f32);
        s.elevation_scale
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_flora_shadow_get_stats(
    p: *mut fc4_flora_shadow,
    out: *mut fc4_flora_shadow_stats,
) -> bool {
    match (shadow(p), out.as_mut()) {
        (Some(s), Some(o)) => {
            *o = s.stats;
            true
        }
        _ => false,
    }
}

// ---- 1. the registry --------------------------------------------------------------------------------

struct Loaded {
    out: Vec<Option<std::rc::Rc<PropertyHolder>>>,
}

unsafe extern "C" fn collect_exemplar(ctx: *mut c_void, _t: u32, _g: u32, _i: u32, props: *mut fc4_props, _ex: u64) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let h = take_props(props);
        if let Some(acc) = (ctx as *mut Loaded).as_mut() {
            acc.out.push(h);
        }
    }));
}

/// The differences between one of our records and the original's.
fn record_diffs(ours: &FloraRecord, theirs: &fc4_flora_record, plop: bool, flags: bool, out: &mut Vec<String>) {
    let t = ours.type_id;
    let mut d = |what: &str, a: String, b: String| {
        if a != b {
            out.push(format!("{t:#010x} {what} ours {a} original {b}"));
        }
    };
    d(
        "next",
        format!("{:#x}", ours.next_type),
        format!("{:#x}", theirs.next_type),
    );
    d(
        "seed",
        format!("{:#x}", ours.seed_type.get()),
        format!("{:#x}", theirs.seed_type),
    );
    d("water", ours.water.to_string(), (theirs.water != 0).to_string());
    for (i, (a, b)) in ours.floats().iter().zip(theirs.floats.iter()).enumerate() {
        if a.to_bits() != b.to_bits() {
            d(&format!("float[{i}]"), a.to_string(), b.to_string());
        }
    }
    if ours.cluster_radius.to_bits() != theirs.cluster_radius.to_bits() {
        d(
            "clusterRadius",
            ours.cluster_radius.to_string(),
            theirs.cluster_radius.to_string(),
        );
    }
    if ours.cluster_height.to_bits() != theirs.cluster_height.to_bits() {
        d(
            "clusterHeight",
            ours.cluster_height.to_string(),
            theirs.cluster_height.to_string(),
        );
    }
    if let Some(i) = (0..256).find(|&i| ours.table[i] != theirs.table[i]) {
        let n = (0..256).filter(|&i| ours.table[i] != theirs.table[i]).count();
        d(
            &format!("table ({n} bytes, first [{}][{}])", i / 16, i % 16),
            ours.table[i].to_string(),
            theirs.table[i].to_string(),
        );
    }
    d("unnamed", ours.unnamed.to_string(), theirs.unnamed.to_string());
    if flags {
        d(
            "flags&7",
            format!("{:#x}", ours.flags & 7),
            format!("{:#x}", theirs.flags & 7),
        );
    }
    // SAFETY: the host promised `family_count` values (or NULL).
    let fam = unsafe { slice(theirs.families, theirs.family_count as usize) };
    d("families", format!("{:x?}", ours.families), format!("{fam:x?}"));
    if plop {
        d("plopCost", ours.plop_cost.to_string(), theirs.plop_cost.to_string());
    }
}

/// Compares the original's registry after load with one built from the same
/// exemplars (module docs, 1). `exemplars(user, visitor, ctx)` enumerates
/// them as the host's `flora_exemplars` does (the exemplar reference must be
/// 0: none is kept). `types`: the original's type array (slot 9);
/// `records[i]`: the record of `types[i]` (`type_id` 0 = slot 7 returned
/// null); `key0`: the record under key 0 (NULL = none); `key0_valid`:
/// `IsFloraTypeValid(0)`. Owner and registry lines go to `sink`; returns the number of
/// differences (-1 on bad arguments).
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_shadow_registry(
    p: *mut fc4_flora_shadow,
    exemplars: Option<unsafe extern "C" fn(*mut c_void, fc4_flora_exemplar_visitor, *mut c_void)>,
    user: *mut c_void,
    types: *const u32,
    type_count: usize,
    records: *const fc4_flora_record,
    key0: *const fc4_flora_record,
    key0_valid: bool,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> i32 {
    let Some(s) = shadow(p) else { return -1 };
    let Some(enumerate) = exemplars else { return -1 };
    let theirs_types = slice(types, type_count);
    let theirs = slice(records, if records.is_null() { 0 } else { type_count });
    let mut acc = Loaded { out: Vec::new() };
    enumerate(user, Some(collect_exemplar), &mut acc as *mut Loaded as *mut c_void);
    guard(-1, || {
        // Ours, as the core's load does (§4.1 step 2, §4.1 step 8).
        let mut reg = FloraRegistry::new();
        let mut has_plop: HashMap<u32, bool> = HashMap::new();
        for h in acc.out.iter().flatten() {
            if u32_of(h, prop::EXEMPLAR_TYPE) != Some(fc4_core::flora::FLORA_EXEMPLAR_TYPE) {
                continue;
            }
            let r = FloraRecord::from_exemplar(h);
            has_plop.entry(r.type_id).or_insert(h.has(PROP_PLOP_COST));
            reg.add(std::rc::Rc::new(r));
        }
        let had_zero = reg.contains(0);
        reg.insert_default();
        let mut diffs: Vec<String> = Vec::new();
        let ours_types = reg.types().to_vec();
        if ours_types != theirs_types {
            let i = ours_types.iter().zip(theirs_types).position(|(a, b)| a != b);
            diffs.push(match i {
                Some(i) => format!(
                    "type array differs at {i}: ours {:#x} original {:#x}",
                    ours_types[i], theirs_types[i]
                ),
                None => format!(
                    "type array lengths ours {} original {}",
                    ours_types.len(),
                    theirs_types.len()
                ),
            });
        }
        let mut compared = 0u32;
        // Keep static-owner evidence separate from simulator registry/order checks.
        let mut static_diffs = Vec::new();
        for (i, &t) in theirs_types.iter().enumerate() {
            let Some(ours) = reg.get(t) else {
                static_diffs.push(format!("{t:#010x} is not in ours"));
                continue;
            };
            match theirs.get(i) {
                Some(r) if r.type_id == t => {
                    compared += 1;
                    record_diffs(
                        &ours,
                        r,
                        has_plop.get(&t).copied().unwrap_or(false),
                        true,
                        &mut static_diffs,
                    );
                }
                Some(r) if r.type_id == 0 => static_diffs.push(format!("{t:#010x}: the original returned no record")),
                Some(r) => static_diffs.push(format!("{t:#010x}: the original's record says type {:#x}", r.type_id)),
                None => static_diffs.push(format!("{t:#010x}: record values not captured")),
            }
        }
        // The key-0 default record (§3.2): flags and plop cost uninitialised.
        let key0_note = match (key0.as_ref(), reg.get(0)) {
            (Some(k), Some(ours)) if !had_zero => {
                compared += 1;
                record_diffs(&ours, k, false, false, &mut static_diffs);
                "compared (flags and plop cost not)"
            }
            (Some(_), _) => "a flora exemplar has type 0: not compared",
            (None, _) => {
                static_diffs.push("the original has no key-0 record".into());
                "missing"
            }
        };
        emit(
            sink,
            ctx,
            &format!(
                "flora-static-shadow: compared={compared} mismatches={} owner=cSC4StaticFloraData; flags bits 3-7 and uninitialised key-0 flags/cost excluded",
                static_diffs.len()
            ),
        );
        diffs.extend(static_diffs);
        if !key0_valid {
            diffs.push("IsFloraTypeValid(0) is false in the original".into());
        }
        s.stats.registry_checks += 1;
        s.stats.registry_mismatches += diffs.len() as u32;
        let mut line = format!(
            "flora shadow registry: exemplars {}, types ours {} original {}; records compared {compared}; key 0 {key0_note}; differences {}",
            acc.out.len(),
            ours_types.len(),
            theirs_types.len(),
            diffs.len()
        );
        for d in diffs.iter().take(LISTED * 2) {
            let _ = write!(line, "; {d}");
        }
        if diffs.len() > LISTED * 2 {
            let _ = write!(line, "; ... {} more", diffs.len() - LISTED * 2);
        }
        emit(sink, ctx, &line);
        diffs.len() as i32
    })
}

// ---- 2. the monthly census --------------------------------------------------------------------------

/// What one occupant is predicted to be after the pass.
#[derive(Clone, Copy, Debug)]
struct Predicted {
    fo: FloraOccupant,
    removed: bool,
    visits: u32,
    reborn: bool,
    aged: bool,
    seeded: bool,
}

#[derive(Default)]
struct MonthTally {
    visited: u32,
    matched: u32,
    mismatched: u32,
    vanished: u32,
    appeared: u32,
    reborn: u32,
    aged: u32,
    removed: u32,
    seeded: u32,
    by_kind: BTreeMap<&'static str, u32>,
    listed: Vec<String>,
}

impl MonthTally {
    fn miss(&mut self, kind: &'static str, what: String) {
        self.mismatched += 1;
        *self.by_kind.entry(kind).or_insert(0) += 1;
        if self.listed.len() < LISTED {
            self.listed.push(what);
        }
    }
}

fn record_of(r: &fc4_flora_record) -> FloraRecord {
    // SAFETY: as `fc4_flora_record` documents (families NULL or valid).
    unsafe { r.core_record() }
}

fn fo_of(e: &fc4_flora_census_entry) -> FloraOccupant {
    FloraOccupant {
        flora_type: e.flora_type,
        birth_date: e.birth_date,
        last_seeding_date: e.last_seeding_date,
        state: e.state,
        orientation: 0,
    }
}

fn same_pos(a: &Vec3, b: &Vec3) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| x.to_bits() == y.to_bits())
}

/// Compares one monthly pass of the original (module docs, 2). Owner and month
/// lines go to `sink`. Returns the mismatches, -2 when nothing could be
/// compared (the line says why), -1 on bad arguments.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_shadow_month(
    p: *mut fc4_flora_shadow,
    m: *const fc4_flora_shadow_month_in,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> i32 {
    let Some(s) = shadow(p) else { return -1 };
    let Some(m) = m.as_ref() else { return -1 };
    if (m.struct_size as usize) < std::mem::size_of::<fc4_flora_shadow_month_in>() {
        return -1;
    }
    let before = slice(m.before, m.before_len);
    let after = slice(m.after, m.after_len);
    let cb = slice(m.census_before, m.before_count);
    let ca = slice(m.census_after, m.after_count);
    let recs = slice(m.records, m.record_count);
    guard(-1, || {
        let prefix = format!("flora shadow month {}", date(m.julian));
        let incomplete = |s: &mut Shadow, why: String| {
            s.stats.incomplete += 1;
            s.stats.last_mismatches = -1;
            emit(
                sink,
                ctx,
                &format!(
                    "{prefix}: INCOMPLETE ({why}); census before {} after {}",
                    cb.len(),
                    ca.len()
                ),
            );
            -2
        };
        if before.is_empty() || after.is_empty() {
            return incomplete(s, "the original's record was not captured".into());
        }
        let (rb, ra) = match (parse_record(before), parse_record(after)) {
            (Ok(b), Ok(a)) => (b, a),
            (Err(e), _) => return incomplete(s, format!("before: {e}")),
            (_, Err(e)) => return incomplete(s, format!("after: {e}")),
        };
        let mut t = MonthTally::default();
        // The record's scalars: only today changes (§5: today := data1).
        let mut field = |name: &'static str, ok: bool, what: String| {
            if !ok {
                t.miss(name, format!("record {name}: {what}"));
            }
        };
        field(
            "today",
            ra.today == m.julian,
            format!("{} (data1 {})", ra.today, m.julian),
        );
        field(
            "flags",
            (rb.enabled, rb.propagation, rb.forced, rb.preserve)
                == (ra.enabled, ra.propagation, ra.forced, ra.preserve),
            "enabled/propagation/forced/preserve changed".into(),
        );
        field(
            "sizes",
            (rb.size_x.to_bits(), rb.size_z.to_bits(), rb.cells_x, rb.cells_z)
                == (ra.size_x.to_bits(), ra.size_z.to_bits(), ra.cells_x, ra.cells_z),
            "sizes or cell counts changed".into(),
        );
        field(
            "tuning",
            (rb.period, rb.blocks_per_month, rb.wild, rb.keep) == (ra.period, ra.blocks_per_month, ra.wild, ra.keep),
            "period/blocksPerMonth/wildSeeds changed".into(),
        );
        // The visited blocks.
        let k = if rb.enabled { rb.blocks_per_month } else { 0 };
        let (blocks, wrapped) = if rb.enabled {
            match visited_blocks(&rb.iterator, &ra.iterator, k) {
                Ok(v) => v,
                Err(e) => return incomplete(s, format!("iterator: {e}")),
            }
        } else {
            if ra.iterator != rb.iterator {
                t.miss("iterator", "disabled, but the iterator moved".into());
            }
            (Vec::new(), false)
        };
        let boxes: Vec<_> = blocks.iter().map(|&(x, z)| block_box(x, z)).collect();
        let records: HashMap<u32, FloraRecord> = recs.iter().map(|r| (r.type_id, record_of(r))).collect();
        if let Some(e) = cb
            .iter()
            .find(|e| e.is_flora != 0 && !records.contains_key(&e.flora_type))
        {
            return incomplete(s, format!("static record {:#x} not captured", e.flora_type));
        }
        let after_by: HashMap<u64, &fc4_flora_census_entry> = ca.iter().map(|e| (e.token, e)).collect();
        let before_tokens: std::collections::HashSet<u64> = cb.iter().map(|e| e.token).collect();
        let scale = s.elevation_scale;
        let scalar_mismatches = t.mismatched;
        for e in cb {
            let mut pr = Predicted {
                fo: fo_of(e),
                removed: false,
                visits: 0,
                reborn: false,
                aged: false,
                seeded: false,
            };
            if e.is_flora != 0 && e.in_manager != 0 {
                for b in &boxes {
                    if pr.removed || !in_block(e, b) {
                        continue;
                    }
                    pr.visits += 1;
                    let rec = records.get(&pr.fo.flora_type);
                    let o = predict_visit(rec, &pr.fo, m.julian, rb.propagation, m.sim_date, || {
                        let r = rec.expect("scored only with a record");
                        let ei = elevation_index(e.altitude, m.sea_level, r.water, scale);
                        score_from_table(&r.table, ei, e.moisture)
                    });
                    if !o.known {
                        continue;
                    }
                    if o.reborn {
                        pr.fo.birth_date = m.julian;
                        pr.reborn = true;
                    }
                    if o.state != pr.fo.state && (o.state == 1 || o.state == 2) {
                        pr.aged = true;
                    }
                    pr.fo.state = o.state;
                    if o.seeded {
                        pr.fo.last_seeding_date = m.sim_date;
                        pr.seeded = true;
                    }
                    pr.removed = o.removed;
                }
            }
            if pr.visits > 0 {
                t.visited += 1;
            }
            t.reborn += u32::from(pr.reborn);
            t.aged += u32::from(pr.aged);
            t.removed += u32::from(pr.removed);
            t.seeded += u32::from(pr.seeded);
            let observed = after_by.get(&e.token).filter(|a| a.in_manager != 0);
            let id = format!(
                "{:#x} type {:#x} at ({}, {})",
                e.token, e.flora_type, e.pos[0], e.pos[2]
            );
            match (pr.removed, observed) {
                (true, None) => t.matched += 1,
                (true, Some(_)) => t.miss("removal", format!("{id}: predicted removed, still present")),
                (false, None) if pr.visits > 0 => t.vanished += 1,
                (false, None) => t.miss("outside", format!("{id}: outside the visited blocks, but gone")),
                (false, Some(a)) => {
                    let got = fo_of(a);
                    let kind = if pr.visits == 0 { Some("outside") } else { None };
                    let mut bad: Vec<String> = Vec::new();
                    if got.state != pr.fo.state {
                        bad.push(format!("state {} predicted {}", got.state, pr.fo.state));
                    }
                    if got.birth_date != pr.fo.birth_date {
                        bad.push(format!("birth {} predicted {}", got.birth_date, pr.fo.birth_date));
                    }
                    if got.last_seeding_date != pr.fo.last_seeding_date {
                        bad.push(format!(
                            "lastSeed {} predicted {}",
                            got.last_seeding_date, pr.fo.last_seeding_date
                        ));
                    }
                    if got.flora_type != pr.fo.flora_type {
                        bad.push(format!("type {:#x}", got.flora_type));
                    }
                    if !same_pos(&a.pos, &e.pos) {
                        bad.push(format!("moved to ({}, {}, {})", a.pos[0], a.pos[1], a.pos[2]));
                    }
                    if bad.is_empty() {
                        t.matched += 1;
                    } else {
                        let k = kind.unwrap_or(if got.state != pr.fo.state {
                            "state"
                        } else if got.birth_date != pr.fo.birth_date {
                            "birth"
                        } else if got.last_seeding_date != pr.fo.last_seeding_date {
                            "seed"
                        } else {
                            "other"
                        });
                        t.miss(k, format!("{id} visits {}: {}", pr.visits, bad.join(", ")));
                    }
                }
            }
        }
        t.appeared = ca.iter().filter(|e| !before_tokens.contains(&e.token)).count() as u32;
        // predict_visit is the independent Rust rule; observations come from
        // native occupant getters around the original's delivery. Do not credit
        // the simulator's scalar checks or random disappearances to this owner.
        let occupant_mismatches = t.mismatched - scalar_mismatches;
        emit(
            sink,
            ctx,
            &format!(
                "flora-occupant-shadow: date={} compared={} mismatches={occupant_mismatches} visited={} owner=cSC4FloraOccupant; random vanished={} new={} excluded",
                date(m.julian),
                t.matched + occupant_mismatches,
                t.visited,
                t.vanished,
                t.appeared
            ),
        );
        // Totals and the line.
        let st = &mut s.stats;
        st.months += 1;
        st.last_mismatches = t.mismatched as i32;
        st.occupants += cb.len() as u32;
        st.visited += t.visited;
        st.matched += t.matched;
        st.mismatched += t.mismatched;
        st.vanished += t.vanished;
        st.appeared += t.appeared;
        st.predicted_reborn += t.reborn;
        st.predicted_aged += t.aged;
        st.predicted_removed += t.removed;
        st.predicted_seeded += t.seeded;
        let mut line = format!(
            "{prefix}: blocks {} of {} (k {k}{}, enabled {}, propagation {}); census before {} after {}; visited {}; matched {} mismatched {}",
            blocks.len(),
            rb.iterator.table.len(),
            if wrapped { ", wrapped" } else { "" },
            u8::from(rb.enabled),
            u8::from(rb.propagation),
            cb.len(),
            ca.len(),
            t.visited,
            t.matched,
            t.mismatched,
        );
        if !t.by_kind.is_empty() {
            let kinds: Vec<String> = t.by_kind.iter().map(|(k, n)| format!("{k} {n}")).collect();
            let _ = write!(line, " ({})", kinds.join(", "));
        }
        let _ = write!(
            line,
            "; predicted reborn {} aged {} removed {} seeded {}; not compared (random): vanished {} new {}; burnt list {}",
            t.reborn, t.aged, t.removed, t.seeded, t.vanished, t.appeared, ra.burnt
        );
        for d in &t.listed {
            let _ = write!(line, "; {d}");
        }
        emit(sink, ctx, &line);
        t.mismatched as i32
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iter_with(table: Vec<u16>, index: i32, exhausted: bool) -> CellIterator {
        let mut it = CellIterator::new_table(0, 0, 31, 31);
        it.table = table;
        it.index = index;
        it.state = if exhausted {
            IteratorState::Exhausted
        } else {
            IteratorState::Running
        };
        it
    }

    #[test]
    fn visited_blocks_follow_the_iterator() {
        let a = iter_with(vec![1, 2, 3, 4], 1, false);
        let b = iter_with(vec![1, 2, 3, 4], 3, false);
        assert_eq!(visited_blocks(&a, &b, 2).unwrap(), (vec![(0, 2), (0, 3)], false));
        // A wrap: 3 left, 2 more from the reshuffled table.
        let c = iter_with(vec![9, 8, 7, 6], 2, false);
        assert_eq!(
            visited_blocks(&a, &c, 5).unwrap(),
            (vec![(0, 2), (0, 3), (0, 4), (0, 9), (0, 8)], true)
        );
        assert!(visited_blocks(&a, &c, 4).is_err(), "after.index must be the remainder");
        assert!(visited_blocks(&a, &c, 9).is_err(), "a double wrap is not compared");
        let ex = iter_with(vec![1, 2], 2, true);
        let d = iter_with(vec![5, 6], 1, false);
        assert_eq!(visited_blocks(&ex, &d, 1).unwrap(), (vec![(0, 5)], true));
        assert_eq!(visited_blocks(&a, &a, 0).unwrap(), (vec![], false));
        assert!(visited_blocks(&a, &b, 1).is_err(), "moved too far");
        assert_eq!(block_box(1, 2), ([32.1, -f32::MAX, 64.1], [63.9, f32::MAX, 95.9]));
    }

    /// Test 38 (§6.7.1, FQ15): which blocks visit a plant near a 64 m line.
    #[test]
    fn blocks_visit_as_the_64_m_index_lists() {
        let entry = |x: f32, z: f32, hw: f32| fc4_flora_census_entry {
            bbox_min: [x - hw, 260.0, z - hw],
            bbox_max: [x + hw, 270.0, z + hw],
            ..Default::default()
        };
        let visits = |e: &fc4_flora_census_entry| -> Vec<(i32, i32)> {
            [(53, 9), (54, 9), (53, 10), (54, 10)]
                .into_iter()
                .filter(|&(bx, bz)| in_block(e, &block_box(bx, bz)))
                .collect()
        };
        // Oak Tree clump 5 0x14C279F4: box z from 319.84396, listed from
        // 319.94396 (z cell 69); the blocks below the line do not return it.
        let oak = entry(1726.9807, 328.84396, 9.0);
        assert_eq!(visits(&oak), [(53, 10), (54, 10)]);
        // 0.2 m below the line: listed from 319.9 (z cell 68), all four.
        assert_eq!(
            visits(&entry(1726.9807, 328.8, 9.0)),
            [(53, 9), (54, 9), (53, 10), (54, 10)]
        );
        // 0.8 m wide (no inset), box z 319.86 .. 320.66, listed from z cell 68.
        assert_eq!(visits(&entry(1726.9807, 320.26, 0.4)), [(53, 9), (53, 10)]);
    }

    /// The record our own simulator writes parses (the original's has the
    /// same layout, §10.1).
    #[test]
    fn parse_record_reads_our_write() {
        let rh = crate::flora_tests::empty_registry_host();
        let host = crate::flora_tests::empty_host();
        unsafe {
            let reg = crate::flora::fc4_flora_registry_create(&rh);
            let f = crate::flora::fc4_flora_create(&host, reg, 1, 1);
            let mut o = crate::test_streams::Out::default();
            assert!(crate::flora::fc4_flora_write(f, &crate::test_streams::ostream(&mut o)));
            let r = parse_record(&o.bytes).unwrap();
            assert!(r.enabled && r.propagation && !r.forced && !r.preserve);
            assert_eq!((r.period, r.blocks_per_month, r.wild, r.burnt), (6, 1, 5, 0));
            assert!(parse_record(&o.bytes[..10]).is_err());
            assert!(parse_record(&[3, 0]).is_err());
            crate::flora::fc4_flora_destroy(f);
            crate::flora::fc4_flora_registry_destroy(reg);
        }
    }
}
