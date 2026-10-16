// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/network-rules.md`: a rule database
//! loaded from vector text, a scratch tile evaluator, a cell grid and the
//! solve state, and ops that drive the rule stages. See README.md
//! ("Network rules ops").

use std::cell::RefCell;
use std::rc::Rc;

use fc4_core::network_rules::{
    adjust_tile_subsets, apply_network_rules, assign_edge_subtypes, cell_key, edge_token,
    identify_highway_intersections, key_cell, mark_highway_intersection_pieces, mirror_edge_byte, mirror_edges,
    neighbourhood, network_name_to_id, patch_tile_pair, remove_bridge_pieces_from_solve_set,
    remove_highway_intersection_pieces_from_solve_set, resolve_one_way_conflicts, rotate_edges, rotate_meta,
    rotate_orientation, solve_multi_type_intersections, solve_multi_type_separation, solve_network,
    verify_tile_subsets, CellGrid, CellInfo, DryHost, Evaluator, IntRule, LoadReport, LoadStatus, NetworkRules,
    OccupantView, ResourceId, SolveState, SolvedRecord, TilePiece, TYPE_COUNT, TYPE_NAMES,
};
use fc4_core::stl::StlHashSet;

use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{self, arr, bool_of, check_expect, check_state, i64_of, ji, opt, str_of, u32_of, R};

pub(crate) struct World {
    rules: RefCell<NetworkRules>,
    ev: RefCell<Evaluator>,
    grid: RefCell<CellGrid>,
    state: RefCell<SolveState>,
}

impl World {
    fn new() -> Rc<World> {
        Rc::new(World {
            rules: RefCell::new(NetworkRules::new()),
            ev: RefCell::new(Evaluator::new()),
            grid: RefCell::new(CellGrid::new(64, 64)),
            state: RefCell::new(SolveState::new()),
        })
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "network_rules", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("nr_")?;
    Some(dispatch(&world(host), name, rest, op))
}

fn dispatch(w: &World, name: &str, rest: &str, op: &Json) -> R<()> {
    match rest {
        "reset" => reset(w, op),
        "edge_token" => check_expect(op, ji(edge_token(i64_of(field(op, "value")?, "value")? as i32))),
        "mirror_byte" => check_expect(op, ji(mirror_edge_byte(u32_of(field(op, "byte")?, "byte")? as u8))),
        "mirror_edges" => check_expect(op, ji(mirror_edges(u32_of(field(op, "edges")?, "edges")?))),
        "rotate_edges" => turns(op).and_then(|r| {
            let e = u32_of(field(op, "edges")?, "edges")?;
            check_expect(op, ji(rotate_edges(e, r)))
        }),
        "rotate_meta" => turns(op).and_then(|r| {
            let m = i64_of(field(op, "m")?, "m")? as i32;
            check_expect(op, ji(rotate_meta(m, r)))
        }),
        "rotate_orientation" => turns(op).and_then(|r| {
            let o = u32_of(field(op, "orientation")?, "orientation")? as u8;
            check_expect(op, ji(rotate_orientation(o, r)))
        }),
        "tiles" => tiles(w, op),
        "expect_rules" => expect_rules(w, op),
        "load" => load(w, op),
        "cell" => cell(w, op),
        "evaluate" => evaluate(w, op),
        "set_s" => keys_of(op, "cells").map(|k| {
            w.state.borrow_mut().s = set_of(&k);
        }),
        "set_valid_count" => i64_of(field(op, "count")?, "count").map(|v| w.state.borrow_mut().valid_count = v as i32),
        "append_solved" => append_solved(w, op),
        "apply" => apply(w, op),
        "multi" => multi(w, op),
        "subtypes" => subtypes(w, op),
        "patch" => patch(w, op),
        "adjust" => {
            let resolve = opt(op, "resolve")
                .map(|v| bool_of(v, "resolve"))
                .transpose()?
                .unwrap_or(false);
            let r = w.rules.borrow();
            let ok = adjust_tile_subsets(&r, &mut w.grid.borrow_mut(), &mut w.state.borrow_mut(), resolve);
            check_expect(op, Json::Bool(ok))
        }
        "verify" => {
            let r = w.rules.borrow();
            let ok = verify_tile_subsets(&r, &mut w.grid.borrow_mut(), &mut w.state.borrow_mut());
            check_expect(op, Json::Bool(ok))
        }
        "solve" => solve(w, op),
        "int_rule" => int_rule(w, op),
        "int_test" => int_test(w, op),
        "identify" => identify(w, op),
        "one_way" => one_way(w, op),
        "separation" => separation(w, op),
        "tile_info" => tile_info(w, op),
        "pieces" => pieces(w, op),
        "mark_pieces" => mark_pieces(w, op),
        "remove_pieces" => remove_pieces(w, op),
        "expect_solved" => expect_solved(w, op),
        "expect_s" => {
            let st = w.state.borrow();
            let got: Vec<Json> = st.s.iter_order().into_iter().map(xz_json).collect();
            want_array(op, "cells", Json::Array(got))
        }
        "expect_potential" => {
            let st = w.state.borrow();
            let got: Vec<Json> = st
                .potential
                .iter()
                .map(|&(x, z, t)| Json::Array(vec![ji(x), ji(z), ji(t)]))
                .collect();
            want_array(op, "entries", Json::Array(got))
        }
        "expect_cell" => expect_cell(w, op),
        _ => Err(format!("unknown op '{name}'")),
    }
}

// ---- helpers ------------------------------------------------------------------------------------

fn turns(op: &Json) -> R<u8> {
    Ok(i64_of(field(op, "turns")?, "turns")? as u8)
}

fn type_of_json(j: &Json, what: &str) -> R<u8> {
    if let Some(s) = j.as_str() {
        if let Some(t) = network_name_to_id(s) {
            return Ok(t);
        }
    }
    let t = i64_of(j, what)?;
    if (0..TYPE_COUNT as i64).contains(&t) {
        Ok(t as u8)
    } else {
        Err(format!("'{what}': unknown network type"))
    }
}

fn xz_of(j: &Json, what: &str) -> R<(i32, i32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be [x, z]"));
    }
    Ok((i64_of(&a[0], what)? as i32, i64_of(&a[1], what)? as i32))
}

fn keys_of(op: &Json, key: &str) -> R<Vec<u32>> {
    arr(field(op, key)?, key)?
        .iter()
        .map(|c| xz_of(c, key).map(|(x, z)| cell_key(x, z)))
        .collect()
}

fn set_of(keys: &[u32]) -> StlHashSet {
    let mut s = StlHashSet::with_hint(100);
    for &k in keys {
        s.insert(k);
    }
    s
}

fn xz_json(key: u32) -> Json {
    let (x, z) = key_cell(key);
    Json::Array(vec![ji(x), ji(z)])
}

fn want_array(op: &Json, key: &str, got: Json) -> R<()> {
    let want = field(op, key)?;
    if scalar::matches(&got, want, false) {
        Ok(())
    } else {
        Err(format!(
            "{key}: got {}, expected {}",
            scalar::show(&got),
            scalar::show(want)
        ))
    }
}

/// The rule text: a string, or an array of lines joined with `\n`.
fn text_of(op: &Json) -> R<Vec<u8>> {
    let t = field(op, "text")?;
    if let Some(s) = t.as_str() {
        return Ok(s.as_bytes().to_vec());
    }
    let lines: R<Vec<&str>> = arr(t, "text")?.iter().map(|l| str_of(l, "text")).collect();
    Ok(lines?.join("\n").into_bytes())
}

fn status_name(s: LoadStatus) -> &'static str {
    match s {
        LoadStatus::Missing => "missing",
        LoadStatus::Complete => "complete",
        LoadStatus::Rejected => "rejected",
        LoadStatus::BadCentre => "bad_centre",
        LoadStatus::Truncated => "truncated",
    }
}

fn report_state(r: &LoadReport) -> Vec<(String, Json)> {
    vec![
        (
            "status".into(),
            r.status.map_or(Json::Null, |s| Json::Str(status_name(s).into())),
        ),
        ("transmogrify".into(), Json::Bool(r.transmogrify)),
        ("source_rules".into(), ji(r.source_rules as i64)),
        ("rules".into(), ji(r.rules as i64)),
        ("dropped".into(), ji(r.dropped_variants as i64)),
        ("entries".into(), ji(r.entries as i64)),
        ("ignored".into(), ji(r.ignored as i64)),
        ("sections".into(), ji(r.sections as i64)),
    ]
}

fn records_json(st: &SolveState) -> Json {
    Json::Array(
        st.solved
            .iter()
            .map(|r| {
                let (x, z) = key_cell(r.key);
                Json::Array(vec![ji(r.id), ji(r.orientation), ji(x), ji(z)])
            })
            .collect(),
    )
}

fn piece_of(j: &Json, what: &str) -> R<TilePiece> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be [id, orientation]"));
    }
    Ok(TilePiece {
        id: u32_of(&a[0], what)?,
        orientation: u32_of(&a[1], what)? as u8,
    })
}

fn piece_json(p: TilePiece) -> Json {
    Json::Array(vec![ji(p.id), ji(p.orientation)])
}

// ---- ops ----------------------------------------------------------------------------------------

/// `nr_reset`: a new rule database, scratch evaluator, grid (`width`,
/// `height`, default 64) and solve state.
fn reset(w: &World, op: &Json) -> R<()> {
    let wd = opt(op, "width").map(|v| u32_of(v, "width")).transpose()?.unwrap_or(64);
    let ht = opt(op, "height")
        .map(|v| u32_of(v, "height"))
        .transpose()?
        .unwrap_or(64);
    *w.rules.borrow_mut() = NetworkRules::new();
    *w.ev.borrow_mut() = Evaluator::new();
    *w.grid.borrow_mut() = CellGrid::new(wd, ht);
    *w.state.borrow_mut() = SolveState::new();
    Ok(())
}

/// `nr_tiles`: read a tile rule file (`text`) into the scratch evaluator
/// (replacing it, or adding to it with `"append": true`); with `type`, the
/// file is instead appended to that type's evaluator list in the database.
/// `expect` checks the load report fields.
fn tiles(w: &World, op: &Json) -> R<()> {
    let bytes = text_of(op)?;
    let report = if let Some(t) = opt(op, "type") {
        let t = type_of_json(t, "type")?;
        w.rules.borrow_mut().add_tile_file(t, Some(&bytes))
    } else {
        let append = opt(op, "append")
            .map(|v| bool_of(v, "append"))
            .transpose()?
            .unwrap_or(false);
        let mut ev = w.ev.borrow_mut();
        if append {
            ev.add_rules(&bytes);
        } else {
            *ev = Evaluator::from_bytes(&bytes);
        }
        LoadReport {
            status: Some(ev.status),
            transmogrify: ev.transmogrify,
            source_rules: ev.source_rules,
            rules: ev.rule_count(),
            dropped_variants: ev.dropped_variants,
            ..LoadReport::default()
        }
    };
    if let Some(want) = opt(op, "expect") {
        check_state(&report_state(&report), want)?;
    }
    Ok(())
}

/// `nr_expect_rules`: the scratch evaluator's rules in map order, each
/// `{key, transform, conditions: [[m, value, mask]], outputs: [[id, m,
/// orientation]]}` (fields left out are not compared).
fn expect_rules(w: &World, op: &Json) -> R<()> {
    let ev = w.ev.borrow();
    let got: Vec<Json> = ev
        .rules()
        .map(|r| {
            Json::Object(vec![
                ("key".into(), ji(r.key)),
                ("transform".into(), ji(r.transform)),
                (
                    "conditions".into(),
                    Json::Array(
                        r.conditions
                            .iter()
                            .map(|c| Json::Array(vec![ji(c.m), ji(c.value), ji(c.mask)]))
                            .collect(),
                    ),
                ),
                (
                    "outputs".into(),
                    Json::Array(
                        r.outputs
                            .iter()
                            .map(|o| Json::Array(vec![ji(o.id), ji(o.m), ji(o.orientation)]))
                            .collect(),
                    ),
                ),
            ])
        })
        .collect();
    let want = field(op, "rules")?;
    let got = Json::Array(got);
    let ok = match (&got, want) {
        (Json::Array(g), Json::Array(wt)) => {
            g.len() == wt.len() && g.iter().zip(wt).all(|(a, b)| scalar::matches(a, b, true))
        }
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "rules: got {}, expected {}",
            scalar::show(&got),
            scalar::show(want)
        ))
    }
}

/// `nr_load`: load one rule resource from text into the database:
/// `resource` is `"multi"` (RUL1), `"overrides"` (RUL2), `"intersections"`
/// (RUL0), `"network"` (the network INI) or an instance number (a tile
/// file). `expect` checks the load report.
fn load(w: &World, op: &Json) -> R<()> {
    let bytes = text_of(op)?;
    let res = field(op, "resource")?;
    let id = match res.as_str() {
        Some("multi") => ResourceId::Rule(fc4_core::network_rules::MULTI_TYPE_INSTANCE),
        Some("overrides") => ResourceId::Rule(fc4_core::network_rules::OVERRIDES_INSTANCE),
        Some("intersections") => ResourceId::Rule(fc4_core::network_rules::INTERSECTION_INSTANCE),
        Some("network") => ResourceId::NetworkIni,
        _ => ResourceId::Rule(u32_of(res, "resource")?),
    };
    let report = w.rules.borrow_mut().load_resource(id, &bytes);
    if let Some(want) = opt(op, "expect") {
        check_state(&report_state(&report), want)?;
    }
    Ok(())
}

/// `nr_cell`: put a cell info at (`x`, `z`): `edges` maps network names to
/// edge words (each sets its type bit); `flags` ORs extra flag bits;
/// `in_run`, `step`, `fixed`, `node`, `intersection`; `occupant` `{piece,
/// orientation, network_flags, node_edges: [a, b], anchor: [x, z]}`.
fn cell(w: &World, op: &Json) -> R<()> {
    let x = i64_of(field(op, "x")?, "x")? as i32;
    let z = i64_of(field(op, "z")?, "z")? as i32;
    let mut c = CellInfo::new(x, z);
    if let Some(e) = opt(op, "edges") {
        for (name, v) in e.as_object().ok_or("'edges' must be an object")? {
            let t = type_of_json(&Json::Str(name.clone()), "edges")?;
            c = c.with_type(t, u32_of(v, "edges")?);
        }
    }
    if let Some(f) = opt(op, "flags") {
        c.flags |= u32_of(f, "flags")?;
    }
    if let Some(v) = opt(op, "in_run") {
        c.in_run = bool_of(v, "in_run")?;
    }
    if let Some(v) = opt(op, "step") {
        c.step = i64_of(v, "step")? as i32;
    }
    if let Some(v) = opt(op, "fixed") {
        c.fixed = bool_of(v, "fixed")?;
    }
    if let Some(v) = opt(op, "node") {
        c.node = bool_of(v, "node")?;
    }
    if let Some(v) = opt(op, "intersection") {
        c.intersection = bool_of(v, "intersection")?;
    }
    if let Some(v) = opt(op, "one_way") {
        c.one_way = u32_of(v, "one_way")? as u8;
    }
    if let Some(v) = opt(op, "marked") {
        c.marked = bool_of(v, "marked")?;
    }
    if let Some(o) = opt(op, "occupant") {
        let mut occ = OccupantView::default();
        if let Some(v) = opt(o, "piece") {
            occ.piece_id = u32_of(v, "piece")?;
        }
        if let Some(v) = opt(o, "orientation") {
            occ.orientation = u32_of(v, "orientation")? as u8;
        }
        if let Some(v) = opt(o, "network_flags") {
            occ.network_flags = u32_of(v, "network_flags")?;
        }
        if let Some(v) = opt(o, "node_edges") {
            let a = arr(v, "node_edges")?;
            if a.len() != 2 {
                return Err("'node_edges' must be [a, b]".into());
            }
            occ.node_edge_types = Some((u32_of(&a[0], "node_edges")?, u32_of(&a[1], "node_edges")?));
        }
        if let Some(v) = opt(o, "anchor") {
            occ.anchor = Some(xz_of(v, "anchor")?);
        }
        if let Some(v) = opt(o, "paired_portal") {
            occ.paired_portal = Some(xz_of(v, "paired_portal")?);
        }
        c.occupant = Some(occ);
    }
    w.grid.borrow_mut().insert(c);
    Ok(())
}

/// `nr_evaluate`: the scratch evaluator at (`x`, `z`) for `type`; `expect`
/// is null (no rule) or the outputs `[[id, m, orientation]]`.
fn evaluate(w: &World, op: &Json) -> R<()> {
    let (x, z) = (
        i64_of(field(op, "x")?, "x")? as i32,
        i64_of(field(op, "z")?, "z")? as i32,
    );
    let t = type_of_json(field(op, "type")?, "type")?;
    let g = w.grid.borrow();
    let ev = w.ev.borrow();
    let got = match ev.evaluate(&neighbourhood(&g, cell_key(x, z), t)) {
        None => Json::Null,
        Some(outs) => Json::Array(
            outs.iter()
                .map(|o| Json::Array(vec![ji(o.id), ji(o.m), ji(o.orientation)]))
                .collect(),
        ),
    };
    check_expect(op, got)
}

/// `nr_append_solved`: append `{id, orientation, (x, z)}` to the solved list.
fn append_solved(w: &World, op: &Json) -> R<()> {
    let (x, z) = (
        i64_of(field(op, "x")?, "x")? as i32,
        i64_of(field(op, "z")?, "z")? as i32,
    );
    let rec = SolvedRecord {
        id: u32_of(field(op, "id")?, "id")?,
        orientation: u32_of(field(op, "orientation")?, "orientation")? as u8,
        key: cell_key(x, z),
    };
    w.state.borrow_mut().append_solved(&mut w.grid.borrow_mut(), rec);
    Ok(())
}

/// `nr_apply`: `ApplyNetworkRules` with the scratch evaluator, `P` from
/// `p` (in the given insertion order) and an empty `W`, for `type`.
/// `expect_p` / `expect_w` check the sets' iteration order afterwards.
fn apply(w: &World, op: &Json) -> R<()> {
    let mut p = set_of(&keys_of(op, "p")?);
    let mut wset = StlHashSet::with_hint(100);
    let t = type_of_json(field(op, "type")?, "type")?;
    apply_network_rules(
        &mut w.grid.borrow_mut(),
        &mut w.state.borrow_mut(),
        &mut p,
        &mut wset,
        &w.ev.borrow(),
        t,
    );
    if opt(op, "expect_p").is_some() {
        let got = Json::Array(p.iter_order().into_iter().map(xz_json).collect());
        want_array(op, "expect_p", got)?;
    }
    if opt(op, "expect_w").is_some() {
        let got = Json::Array(wset.iter_order().into_iter().map(xz_json).collect());
        want_array(op, "expect_w", got)?;
    }
    Ok(())
}

/// `nr_multi`: `SolveMultiTypeIntersections` for tool type `type`;
/// `expect` is the output set's cells in order.
fn multi(w: &World, op: &Json) -> R<()> {
    let t = type_of_json(field(op, "type")?, "type")?;
    let mut out = Vec::new();
    let r = w.rules.borrow();
    solve_multi_type_intersections(&r, &mut w.grid.borrow_mut(), &mut w.state.borrow_mut(), &mut out, t);
    check_expect(op, Json::Array(out.into_iter().map(xz_json).collect()))
}

/// `nr_subtypes`: `AssignEdgeSubtypes` with `P` from `p`, `reset` (default
/// true); `expect` is the failure set `R` in iteration order, `expect_p`
/// the set `P` afterwards.
fn subtypes(w: &World, op: &Json) -> R<()> {
    let mut p = set_of(&keys_of(op, "p")?);
    let reset = opt(op, "reset")
        .map(|v| bool_of(v, "reset"))
        .transpose()?
        .unwrap_or(true);
    let mut r = StlHashSet::with_hint(100);
    assign_edge_subtypes(&mut w.grid.borrow_mut(), &mut p, &mut r, reset);
    check_expect(op, Json::Array(r.iter_order().into_iter().map(xz_json).collect()))?;
    if opt(op, "expect_p").is_some() {
        let got = Json::Array(p.iter_order().into_iter().map(xz_json).collect());
        want_array(op, "expect_p", got)?;
    }
    Ok(())
}

/// `nr_patch`: `PatchTilePair(a, b, side)` with `a`, `b` `[id,
/// orientation]`; `expect` the result, `expect_a` / `expect_b` the pieces.
fn patch(w: &World, op: &Json) -> R<()> {
    let mut a = piece_of(field(op, "a")?, "a")?;
    let mut b = piece_of(field(op, "b")?, "b")?;
    let s = i64_of(field(op, "side")?, "side")? as usize;
    let n = patch_tile_pair(&w.rules.borrow(), &mut a, &mut b, s);
    check_expect(op, ji(n))?;
    if let Some(want) = opt(op, "expect_a") {
        if !scalar::matches(&piece_json(a), want, false) {
            return Err(format!("a: got {}", scalar::show(&piece_json(a))));
        }
    }
    if let Some(want) = opt(op, "expect_b") {
        if !scalar::matches(&piece_json(b), want, false) {
            return Err(format!("b: got {}", scalar::show(&piece_json(b))));
        }
    }
    Ok(())
}

/// `nr_solve`: the rule stages of `SolveNetwork` (tool `type`, `resolve`
/// default false); `expect` the result.
fn solve(w: &World, op: &Json) -> R<()> {
    let t = type_of_json(field(op, "type")?, "type")?;
    let resolve = opt(op, "resolve")
        .map(|v| bool_of(v, "resolve"))
        .transpose()?
        .unwrap_or(false);
    let r = w.rules.borrow();
    let v = solve_network(&r, &mut w.grid.borrow_mut(), &mut w.state.borrow_mut(), t, resolve);
    check_expect(op, ji(v))
}

fn int_rule_of(w: &World, op: &Json) -> R<IntRule> {
    let id = u32_of(field(op, "id")?, "id")?;
    let r = w.rules.borrow();
    let mut rule = r
        .int_rules
        .get(id)
        .cloned()
        .ok_or_else(|| format!("no intersection rule {id:#x}"))?;
    if let Some(v) = opt(op, "rotate") {
        rule.rotate(i64_of(v, "rotate")? as i32);
    }
    if opt(op, "transpose").map(|v| bool_of(v, "transpose")).transpose()? == Some(true) {
        rule.transpose();
    }
    Ok(rule)
}

/// `nr_int_rule`: one intersection rule (a copy, optionally turned by
/// `rotate` or `transpose`d); `state` checks `occupied`, `tagged` (`[[x,
/// z, tag]]`), `new_cells`, `bounds` (`[minX, maxX, minZ, maxZ]`),
/// `one_way_dir`, `type_mask`, `ring` (`[prev, next]`), `add` and
/// `edges` (`{tag: [edges1, mask1, edges2, mask2]}`).
fn int_rule(w: &World, op: &Json) -> R<()> {
    let r = int_rule_of(w, op)?;
    let cells = |v: &[(i32, i32)]| Json::Array(v.iter().map(|&(x, z)| Json::Array(vec![ji(x), ji(z)])).collect());
    let b = r.bounds;
    let got = vec![
        ("occupied".to_string(), cells(&r.occupied)),
        ("new_cells".to_string(), cells(&r.new_cells)),
        (
            "tagged".to_string(),
            Json::Array(
                r.tagged
                    .iter()
                    .map(|t| Json::Array(vec![ji(t.x), ji(t.z), Json::Str((t.tag as char).to_string())]))
                    .collect(),
            ),
        ),
        (
            "bounds".to_string(),
            Json::Array(vec![ji(b.min_x), ji(b.max_x), ji(b.min_z), ji(b.max_z)]),
        ),
        ("one_way_dir".to_string(), ji(r.one_way_dir)),
        ("type_mask".to_string(), ji(r.type_mask)),
        ("ring".to_string(), Json::Array(vec![ji(r.ring_prev), ji(r.ring_next)])),
        ("add".to_string(), Json::Array(vec![ji(r.add_prev), ji(r.add_next)])),
        (
            "edges".to_string(),
            Json::Object(
                r.check_types
                    .iter()
                    .map(|(&tag, ct)| {
                        (
                            (tag as char).to_string(),
                            Json::Array(vec![ji(ct.edges1), ji(ct.mask1), ji(ct.edges2), ji(ct.mask2)]),
                        )
                    })
                    .collect(),
            ),
        ),
    ];
    check_state(&got, field(op, "state")?)
}

/// `nr_int_test`: `cIntRule::Test` of rule `id` (optionally turned) at
/// (`x`, `z`) on a dry city; `expect` 0, 1 or 2.
fn int_test(w: &World, op: &Json) -> R<()> {
    let r = int_rule_of(w, op)?;
    let (x, z) = (
        i64_of(field(op, "x")?, "x")? as i32,
        i64_of(field(op, "z")?, "z")? as i32,
    );
    check_expect(op, ji(r.test(x, z, &w.grid.borrow(), &DryHost)))
}

/// `nr_identify`: `IdentifyHighwayIntersections(v, type)` over `steps`
/// (each step a list of `[x, z]`); `expect` the result.
fn identify(w: &World, op: &Json) -> R<()> {
    let steps: R<Vec<Vec<(i32, i32)>>> = arr(field(op, "steps")?, "steps")?
        .iter()
        .map(|s| arr(s, "steps")?.iter().map(|c| xz_of(c, "steps")).collect())
        .collect();
    let v = i64_of(field(op, "v")?, "v")? as i32;
    let t = u32_of(field(op, "type")?, "type")?;
    let r = w.rules.borrow();
    let got = identify_highway_intersections(
        &r.int_rules,
        &mut w.grid.borrow_mut(),
        &mut w.state.borrow_mut(),
        &steps?,
        v,
        t,
        &DryHost,
    );
    check_expect(op, ji(got))?;
    if opt(op, "expect_intersections").is_some() {
        let st = w.state.borrow();
        let got = Json::Array(
            st.intersections
                .iter()
                .map(|i| {
                    Json::Array(vec![
                        ji(i.rule_id),
                        ji(i.x),
                        ji(i.z),
                        ji(i.start_step),
                        ji(i.step_a),
                        ji(i.step_b),
                    ])
                })
                .collect(),
        );
        want_array(op, "expect_intersections", got)?;
    }
    Ok(())
}

/// `nr_expect_solved`: the solved list `[[id, orientation, x, z]]`.
fn expect_solved(w: &World, op: &Json) -> R<()> {
    want_array(op, "records", records_json(&w.state.borrow()))
}

/// `nr_expect_cell`: the cell at (`x`, `z`): `state` checks `flags`,
/// `union`, `solved`, `intersection` and `edges` (`{name: word}`, every
/// type with a non-zero word).
/// `nr_one_way`: `ResolveOneWayConflicts` over `steps` (one array of `[x, z]`
/// per dragged step). Check the one-way bytes with `nr_expect_cell` and the
/// solve set with `nr_expect_s`.
fn one_way(w: &World, op: &Json) -> R<()> {
    let steps: R<Vec<Vec<(i32, i32)>>> = arr(field(op, "steps")?, "steps")?
        .iter()
        .map(|s| arr(s, "steps")?.iter().map(|c| xz_of(c, "steps")).collect())
        .collect();
    let steps = steps?;
    resolve_one_way_conflicts(&mut w.grid.borrow_mut(), &mut w.state.borrow_mut(), &steps);
    Ok(())
}

/// `nr_separation`: `SolveMultiTypeSeparation` at (`x`, `z`) for `type`, over
/// the rule database's evaluators of that type. `expect` is null (no piece)
/// or `[id, rot, flip]`.
fn separation(w: &World, op: &Json) -> R<()> {
    let (x, z) = (
        i64_of(field(op, "x")?, "x")? as i32,
        i64_of(field(op, "z")?, "z")? as i32,
    );
    let t = type_of_json(field(op, "type")?, "type")?;
    let r = w.rules.borrow();
    let g = w.grid.borrow();
    let got = match solve_multi_type_separation(r.evaluators[t as usize].iter(), &g, x, z, t) {
        None => Json::Null,
        Some((id, rot, flip)) => Json::Array(vec![ji(id), ji(rot), ji(flip)]),
    };
    check_expect(op, got)
}

/// `nr_tile_info`: `GetTileInfo(dx, dz)` of the rule. `expect` is null or
/// `[tile, orientation, new_cell]`.
fn tile_info(w: &World, op: &Json) -> R<()> {
    let r = int_rule_of(w, op)?;
    let (dx, dz) = xz_of(field(op, "offset")?, "offset")?;
    let got = match r.get_tile_info(dx, dz) {
        None => Json::Null,
        Some(t) => Json::Array(vec![ji(t.tile), ji(t.orientation), ji(t.new_cell)]),
    };
    check_expect(op, got)
}

/// `nr_pieces`: the pieces an intersection at (`x`, `z`) places (§10.9), one
/// per occupied cell: `[ox, oz, piece_id, orientation, path, ax, az, new_cell]`.
fn pieces(w: &World, op: &Json) -> R<()> {
    let r = int_rule_of(w, op)?;
    let (x, z) = (
        i64_of(field(op, "x")?, "x")? as i32,
        i64_of(field(op, "z")?, "z")? as i32,
    );
    let g = w.grid.borrow();
    let got = Json::Array(
        r.intersection_pieces(x, z, &g)
            .iter()
            .map(|p| {
                Json::Array(vec![
                    ji(p.offset.0),
                    ji(p.offset.1),
                    ji(p.piece_id),
                    ji(p.orientation),
                    ji(p.path),
                    ji(p.anchor.0),
                    ji(p.anchor.1),
                    ji(p.new_cell),
                ])
            })
            .collect(),
    );
    check_expect(op, got)
}

/// `nr_mark_pieces`: `MarkHighwayIntersectionPieces` over the recorded
/// intersections, for the records whose start step is below `valid_count`.
fn mark_pieces(w: &World, op: &Json) -> R<()> {
    let on = bool_of(field(op, "on")?, "on")?;
    let valid_count = i64_of(field(op, "valid_count")?, "valid_count")? as i32;
    let r = w.rules.borrow();
    let mut g = w.grid.borrow_mut();
    let st = w.state.borrow();
    mark_highway_intersection_pieces(&r.int_rules, &mut g, &st.intersections, valid_count, on);
    Ok(())
}

/// `nr_remove_pieces`: `RemoveHighwayIntersectionPiecesFromSetOfCellsToBeSolved`
/// when `bridge` is absent or false, else `RemoveBridgePiecesFromSetOfCellsToBeSolved`
/// over `bridge_cells` `[[end_step, [key, ...]], ...]`.
fn remove_pieces(w: &World, op: &Json) -> R<()> {
    let valid_count = i64_of(field(op, "valid_count")?, "valid_count")? as i32;
    if opt(op, "bridge").map(|v| bool_of(v, "bridge")).transpose()? == Some(true) {
        let cells: R<Vec<(i32, Vec<u32>)>> = arr(field(op, "bridge_cells")?, "bridge_cells")?
            .iter()
            .map(|e| {
                let a = arr(e, "bridge_cells")?;
                if a.len() != 2 {
                    return Err("'bridge_cells' entries are [end_step, [key, ...]]".into());
                }
                let keys: R<Vec<u32>> = arr(&a[1], "bridge_cells")?
                    .iter()
                    .map(|k| u32_of(k, "bridge_cells"))
                    .collect();
                Ok((i64_of(&a[0], "bridge_cells")? as i32, keys?))
            })
            .collect();
        remove_bridge_pieces_from_solve_set(&mut w.state.borrow_mut(), &cells?, valid_count);
        return Ok(());
    }
    let r = w.rules.borrow();
    let intersections = w.state.borrow().intersections.clone();
    remove_highway_intersection_pieces_from_solve_set(
        &r.int_rules,
        &mut w.state.borrow_mut(),
        &intersections,
        valid_count,
    );
    Ok(())
}

fn expect_cell(w: &World, op: &Json) -> R<()> {
    let (x, z) = (
        i64_of(field(op, "x")?, "x")? as i32,
        i64_of(field(op, "z")?, "z")? as i32,
    );
    let g = w.grid.borrow();
    let c = g.at(x, z).ok_or_else(|| format!("no cell info at ({x}, {z})"))?;
    let edges = Json::Object(
        (0..TYPE_COUNT)
            .filter(|&t| c.edges[t] != 0)
            .map(|t| (TYPE_NAMES[t].to_string(), ji(c.edges[t])))
            .collect(),
    );
    let got = vec![
        ("flags".to_string(), ji(c.flags)),
        ("union".to_string(), ji(c.union)),
        ("solved".to_string(), ji(c.solved)),
        ("intersection".to_string(), Json::Bool(c.intersection)),
        ("one_way".to_string(), ji(c.one_way)),
        ("marked".to_string(), Json::Bool(c.marked)),
        ("in_run".to_string(), Json::Bool(c.in_run)),
        ("edges".to_string(), edges),
    ];
    check_state(&got, field(op, "state")?)
}
