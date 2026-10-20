// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/network-geometry.md` (`ng_…`): a city
//! (a `fc4_core::terrain::Terrain`), a scripted host (occupants, existing
//! pieces, connection nodes, bridge types) and a `NetworkGeometry`. See
//! README.md ("Network geometry ops").

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use fc4_core::network_geometry::extras::{self, Removal};
use fc4_core::network_geometry::smoothing::{terrain_vertex_constraint, TerrainVertex};
use fc4_core::network_geometry::{
    break_into_segments, BridgeChoice, CellOccupant, ExistingPiece, GeometryHost, NetworkGeometry, PlacementParams,
    Run, TypeInfo, TypeTable, VertexHtConstraintSatisfier,
};
use fc4_core::terrain::{Terrain, TerrainTuning};

use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{self, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, EventLog, R};

#[derive(Default)]
struct Host {
    landfill: Vec<(i32, i32)>,
    immovable: Vec<((i32, i32), bool, u32)>,
    nodes: Vec<(i32, i32)>,
    existing: Vec<((i32, i32), ExistingPiece)>,
    extend: Vec<(i32, i32)>,
    bridge_types: u32,
    bridge_min_span: u32,
    highlight_cut: Option<u32>,
}

impl GeometryHost for Host {
    fn is_connection_node(&self, x: i32, z: i32) -> bool {
        self.nodes.contains(&(x, z))
    }
    fn cell_occupants(&self, x: i32, z: i32) -> Vec<CellOccupant> {
        let mut v = Vec::new();
        if self.landfill.contains(&(x, z)) {
            v.push(CellOccupant::Landfill);
        }
        for &(c, node, t) in &self.immovable {
            if c == (x, z) {
                v.push(CellOccupant::Network {
                    immovable: true,
                    connection_node: node,
                    type_id: t,
                });
            }
        }
        v
    }
    fn existing_piece(&self, x: i32, z: i32) -> Option<ExistingPiece> {
        self.existing.iter().find(|(c, _)| *c == (x, z)).map(|(_, p)| p.clone())
    }
    fn extends_diagonal_end(&self, x: i32, z: i32, _t: u32) -> bool {
        self.extend.contains(&(x, z))
    }
    fn update_occupant_highlights(&mut self, _run: &Run, v: u32) -> u32 {
        self.highlight_cut.map_or(v, |c| c.min(v))
    }
    fn bridge_type_count(&self, _t: u32) -> u32 {
        self.bridge_types
    }
    fn find_best_bridge_type(&mut self, _run: &Run, _s: u32, _e: u32) -> Option<BridgeChoice> {
        (self.bridge_types > 0).then_some(BridgeChoice {
            bridge_type: 0,
            min_span: self.bridge_min_span,
        })
    }
}

pub(crate) struct World {
    events: EventLog,
    terrain: RefCell<Terrain>,
    host: RefCell<Host>,
    geo: RefCell<NetworkGeometry>,
    resolve: RefCell<BTreeSet<i32>>,
}

impl World {
    fn new() -> Rc<World> {
        let mut t = Terrain::create(64, 64, TerrainTuning::SHIPPED).expect("64 x 64");
        t.set_view_ready(true);
        Rc::new(World {
            events: EventLog::default(),
            terrain: RefCell::new(t),
            host: RefCell::new(Host::default()),
            geo: RefCell::new(NetworkGeometry::new(TypeTable::shipped())),
            resolve: RefCell::new(BTreeSet::new()),
        })
    }

    fn drain_posts(&self) {
        for p in self.terrain.borrow_mut().take_posts() {
            self.events.push(
                "post",
                vec![
                    ("msg", ji(p.msg)),
                    ("data1", ji(p.data1)),
                    ("data2", ji(p.data2)),
                    ("data3", ji(p.data3)),
                    ("data4", ji(p.data4)),
                ],
            );
        }
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "network_geometry", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("ng_")?;
    let w = world(host);
    let r = match rest {
        "city" => city(&w, op),
        "types" => types(&w, op),
        "host" => set_host(&w, op),
        "segments" => segments(op),
        "draw" => draw(&w, op),
        "validate" => validate(&w, op),
        "cell_infos" => cell_infos(&w, op),
        "smoothen" => smoothen(&w, op),
        "support" => support(&w, op),
        "place" => place(&w, op),
        "expect" => expect(&w, op),
        "type_info" => type_info(op),
        "sat" => sat(&w, op),
        "terrain_vertex" => terrain_vertex(op),
        "support_level" => support_level(op),
        "occupant_flags" => occupant_flags(op),
        "cost" => cost(&w, op),
        "power_line_cost" => power_line_cost(op),
        "destroy" => destroy(&w, op),
        "grade" => grade(&w, op),
        "expect_events" => w.events.expect_events(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        _ => Err(format!("unknown op '{name}'")),
    };
    w.drain_posts();
    Some(r)
}

// ---- values -------------------------------------------------------------------------------------

/// A float: a number, `"max"` (FLT_MAX) or `"-max"`.
fn fl(j: &Json, what: &str) -> R<f32> {
    match j.as_str() {
        Some("max") => Ok(f32::MAX),
        Some("-max") => Ok(-f32::MAX),
        _ => f32_of(j, what),
    }
}

fn int(j: &Json, what: &str) -> R<i32> {
    Ok(i64_of(j, what)? as i32)
}

fn pair(j: &Json, what: &str) -> R<(i32, i32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be [x, z]"));
    }
    Ok((int(&a[0], what)?, int(&a[1], what)?))
}

fn pairs(j: &Json, what: &str) -> R<Vec<(i32, i32)>> {
    arr(j, what)?.iter().map(|p| pair(p, what)).collect()
}

fn floats<const N: usize>(j: &Json, what: &str) -> R<[f32; N]> {
    let a = arr(j, what)?;
    if a.len() != N {
        return Err(format!("'{what}' must have {N} values"));
    }
    let mut out = [0.0; N];
    for (o, v) in out.iter_mut().zip(a) {
        *o = fl(v, what)?;
    }
    Ok(out)
}

fn jpair(p: (i32, i32)) -> Json {
    Json::Array(vec![ji(p.0), ji(p.1)])
}

fn type_of(op: &Json) -> R<u32> {
    Ok(opt(op, "type").map(|j| int(j, "type")).transpose()?.unwrap_or(0) as u32)
}

// ---- setup --------------------------------------------------------------------------------------

/// `ng_city`: a 64 × 64 city (or `cells`), flat at `altitude` (300), the sea
/// at `sea` (250); `columns` [[x0, x1, alt]] set vertex columns,
/// `vertices` [[x, z, alt]] single vertices; `view_ready` (true).
fn city(w: &World, op: &Json) -> R<()> {
    let (cx, cz) = opt(op, "cells")
        .map(|j| pair(j, "cells"))
        .transpose()?
        .unwrap_or((64, 64));
    let base = opt(op, "altitude")
        .map(|j| fl(j, "altitude"))
        .transpose()?
        .unwrap_or(300.0);
    let sea = opt(op, "sea").map(|j| fl(j, "sea")).transpose()?.unwrap_or(250.0);
    let (vx, vz) = (cx + 1, cz + 1);
    let mut a = vec![base; (vx * vz) as usize];
    if let Some(cols) = opt(op, "columns") {
        for c in arr(cols, "columns")? {
            let c = arr(c, "columns")?;
            if c.len() != 3 {
                return Err("a column is [x0, x1, alt]".into());
            }
            let (x0, x1, h) = (int(&c[0], "x0")?, int(&c[1], "x1")?, fl(&c[2], "alt")?);
            for z in 0..vz {
                for x in x0.max(0)..=x1.min(vx - 1) {
                    a[(z * vx + x) as usize] = h;
                }
            }
        }
    }
    if let Some(vs) = opt(op, "slope_x") {
        // alt = base + slope_x × x
        let s = fl(vs, "slope_x")?;
        for z in 0..vz {
            for x in 0..vx {
                a[(z * vx + x) as usize] = base + s * x as f32;
            }
        }
    }
    if let Some(vs) = opt(op, "vertices") {
        for v in arr(vs, "vertices")? {
            let v = arr(v, "vertices")?;
            if v.len() != 3 {
                return Err("a vertex is [x, z, alt]".into());
            }
            let (x, z, h) = (int(&v[0], "x")?, int(&v[1], "z")?, fl(&v[2], "alt")?);
            a[(z * vx + x) as usize] = h;
        }
    }
    let mut t = Terrain::from_altitudes(cx as u32, cz as u32, &a, sea, TerrainTuning::SHIPPED).ok_or("bad city")?;
    t.set_view_ready(
        opt(op, "view_ready")
            .map(|j| bool_of(j, "view_ready"))
            .transpose()?
            .unwrap_or(true),
    );
    *w.terrain.borrow_mut() = t;
    Ok(())
}

/// `ng_types`: `type` with `params` (a partial `PlacementParams` over the
/// shipped values).
fn types(w: &World, op: &Json) -> R<()> {
    let t = type_of(op)?;
    let mut p = PlacementParams::shipped(t);
    if let Some(ps) = opt(op, "params") {
        for (k, v) in ps.as_object().ok_or("'params' must be an object")? {
            let f = fl(v, k)?;
            match k.as_str() {
                "max_slope" => p.max_slope = f,
                "max_slope_change" => p.max_slope_change = f,
                "max_raise" => p.max_raise = f,
                "max_cut" => p.max_cut = f,
                "resolve_tolerance" => p.resolve_tolerance = f,
                "tunnel_trigger_slope" => p.tunnel_trigger_slope = f,
                "tunnel_end_slope" => p.tunnel_end_slope = f,
                "new_cell_tolerance" => p.new_cell_tolerance = f,
                "support_threshold" => p.support_threshold = f,
                "max_support_height" => p.max_support_height = f,
                "tunnel_clearance" => p.tunnel_clearance = vec![f],
                other => return Err(format!("unknown param '{other}'")),
            }
        }
    }
    let mut g = w.geo.borrow_mut();
    g.types.infos[t as usize] = TypeInfo::new(t, p);
    Ok(())
}

/// `ng_host`: `landfill` [[x, z]], `immovable` [[x, z, node, type]],
/// `nodes` [[x, z]], `existing` [{cell, immovable?, flags?, corners,
/// supports?}], `extend` [[x, z]], `bridge_types`, `bridge_min_span`,
/// `highlight_cut`.
fn set_host(w: &World, op: &Json) -> R<()> {
    let mut h = w.host.borrow_mut();
    if let Some(j) = opt(op, "landfill") {
        h.landfill = pairs(j, "landfill")?;
    }
    if let Some(j) = opt(op, "nodes") {
        h.nodes = pairs(j, "nodes")?;
    }
    if let Some(j) = opt(op, "extend") {
        h.extend = pairs(j, "extend")?;
    }
    if let Some(j) = opt(op, "immovable") {
        h.immovable.clear();
        for e in arr(j, "immovable")? {
            let e = arr(e, "immovable")?;
            if e.len() != 4 {
                return Err("immovable is [x, z, node, type]".into());
            }
            h.immovable.push((
                (int(&e[0], "x")?, int(&e[1], "z")?),
                bool_of(&e[2], "node")?,
                int(&e[3], "type")? as u32,
            ));
        }
    }
    if let Some(j) = opt(op, "existing") {
        h.existing.clear();
        for e in arr(j, "existing")? {
            let cell = pair(field(e, "cell")?, "cell")?;
            let p = ExistingPiece {
                flags: opt(e, "flags").map(|j| int(j, "flags")).transpose()?.unwrap_or(1) as u32,
                immovable: opt(e, "immovable")
                    .map(|j| bool_of(j, "immovable"))
                    .transpose()?
                    .unwrap_or(false),
                corners: floats::<4>(field(e, "corners")?, "corners")?,
                has_supports: opt(e, "supports")
                    .map(|j| bool_of(j, "supports"))
                    .transpose()?
                    .unwrap_or(false),
                ..ExistingPiece::default()
            };
            h.existing.push((cell, p));
        }
    }
    if let Some(j) = opt(op, "bridge_types") {
        h.bridge_types = int(j, "bridge_types")? as u32;
    }
    if let Some(j) = opt(op, "bridge_min_span") {
        h.bridge_min_span = int(j, "bridge_min_span")? as u32;
    }
    if let Some(j) = opt(op, "highlight_cut") {
        h.highlight_cut = Some(int(j, "highlight_cut")? as u32);
    }
    Ok(())
}

// ---- the drag -----------------------------------------------------------------------------------

/// `ng_segments`: `start`, `end`, `snap?`, `straight_only?`, `widths?`
/// [w1, w2] (1, 1); `state` checks `end`, `dragged_count`,
/// `straight_count`, `axis`, `pen_direction`, `end_direction`.
fn segments(op: &Json) -> R<()> {
    let start = pair(field(op, "start")?, "start")?;
    let end = pair(field(op, "end")?, "end")?;
    let snap = opt(op, "snap")
        .map(|j| bool_of(j, "snap"))
        .transpose()?
        .unwrap_or(false);
    let so = opt(op, "straight_only")
        .map(|j| bool_of(j, "straight_only"))
        .transpose()?
        .unwrap_or(false);
    let (w1, w2) = opt(op, "widths")
        .map(|j| pair(j, "widths"))
        .transpose()?
        .unwrap_or((1, 1));
    let s = break_into_segments(start, end, snap, so, w1 as u32, w2 as u32);
    let got = vec![
        ("end".to_owned(), jpair(s.end)),
        ("dragged_count".to_owned(), ji(s.dragged_count)),
        ("straight_count".to_owned(), ji(s.straight_count)),
        ("axis".to_owned(), Json::Array(vec![ji(s.axis.0), ji(s.axis.1)])),
        ("pen_direction".to_owned(), ji(s.pen_direction)),
        ("end_direction".to_owned(), ji(s.end_direction)),
    ];
    check_state(&got, field(op, "state")?)
}

/// `ng_draw`: `type?` (0), `start`, `end`, `snap?` (false), `expect?` (the
/// result); a fresh run (the solve state is reset).
fn draw(w: &World, op: &Json) -> R<()> {
    let t = type_of(op)?;
    let start = pair(field(op, "start")?, "start")?;
    let end = pair(field(op, "end")?, "end")?;
    let snap = opt(op, "snap")
        .map(|j| bool_of(j, "snap"))
        .transpose()?
        .unwrap_or(false);
    let mut g = w.geo.borrow_mut();
    g.reset();
    let ok = g.draw_network_line(t, start, end, snap, &*w.terrain.borrow(), &*w.host.borrow());
    check_expect(op, Json::Bool(ok))
}

/// `ng_validate`: the passes; `expect?` the valid count.
fn validate(w: &World, op: &Json) -> R<()> {
    let v = {
        let mut g = w.geo.borrow_mut();
        g.validate(&*w.terrain.borrow(), &mut *w.host.borrow_mut()).valid
    };
    check_expect(op, ji(v))
}

/// `ng_cell_infos`: the cell infos of the valid steps.
fn cell_infos(w: &World, _op: &Json) -> R<()> {
    let mut g = w.geo.borrow_mut();
    g.create_cell_infos(&*w.terrain.borrow(), &*w.host.borrow());
    Ok(())
}

/// `ng_smoothen`: `level`; `expect?` the solve's result.
fn smoothen(w: &World, op: &Json) -> R<()> {
    let level = int(field(op, "level")?, "level")?;
    let ok = w
        .geo
        .borrow_mut()
        .smoothen(level, &*w.terrain.borrow(), &mut *w.host.borrow_mut());
    check_expect(op, Json::Bool(ok))
}

/// `ng_support`: slot 56 on the last smoothing; `expect?`.
fn support(w: &World, op: &Json) -> R<()> {
    let ok = w
        .geo
        .borrow_mut()
        .can_tiles_be_supported(&*w.terrain.borrow(), &mut *w.host.borrow_mut());
    check_expect(op, Json::Bool(ok))
}

/// `ng_place`: the passes, the cell infos and the smoothing loop; `expect?`
/// the valid count.
fn place(w: &World, op: &Json) -> R<()> {
    let v = w
        .geo
        .borrow_mut()
        .place(&*w.terrain.borrow(), &mut *w.host.borrow_mut());
    check_expect(op, ji(v))
}

/// `ng_expect`: `state` (partial): `valid`, `failure`, `level` (null when
/// none), `end`, `dragged_count`, `straight_count`, `pen_direction`,
/// `end_direction`, `cells` ([[x, z]]), `kinds` (per step), `tunnels`
/// ([[start, end]]), `bridges` ([[start, end]]), `tunnel_searches`,
/// `heights` (`{"vertex": h}` from the network satisfier, null =
/// unsolved), `terrain_heights` (the same from the terrain satisfier),
/// `smoothing_cells`, `boundary`.
fn expect(w: &World, op: &Json) -> R<()> {
    let want = field(op, "state")?;
    let g = w.geo.borrow();
    let r = &g.run;
    let heights = |sat: &VertexHtConstraintSatisfier, key: &str| -> R<Json> {
        let mut out = Vec::new();
        if let Some(o) = want.get(key).and_then(Json::as_object) {
            for (k, _) in o {
                let v: i32 = k.parse().map_err(|_| format!("bad vertex '{k}'"))?;
                out.push((k.clone(), sat.height(v).map_or(Json::Null, jf)));
            }
        }
        Ok(Json::Object(out))
    };
    let got = vec![
        ("valid".to_owned(), ji(g.valid)),
        ("failure".to_owned(), ji(g.failure)),
        ("level".to_owned(), g.level.map_or(Json::Null, ji)),
        ("end".to_owned(), jpair(r.end)),
        ("dragged_count".to_owned(), ji(r.dragged_count)),
        ("straight_count".to_owned(), ji(r.straight_count)),
        ("pen_direction".to_owned(), ji(r.pen_direction)),
        ("end_direction".to_owned(), ji(r.end_direction)),
        (
            "cells".to_owned(),
            Json::Array(r.cells.iter().map(|&c| jpair(c)).collect()),
        ),
        (
            "kinds".to_owned(),
            Json::Array(r.steps.iter().map(|s| ji(s.kind as u32)).collect()),
        ),
        (
            "tunnels".to_owned(),
            Json::Array(
                r.tunnels
                    .iter()
                    .map(|t| Json::Array(vec![ji(t.start), ji(t.end)]))
                    .collect(),
            ),
        ),
        (
            "bridges".to_owned(),
            Json::Array(
                r.bridges
                    .iter()
                    .map(|b| Json::Array(vec![ji(b.start), ji(b.end)]))
                    .collect(),
            ),
        ),
        (
            "tunnel_searches".to_owned(),
            Json::Array(r.tunnel_searches.iter().map(|&s| ji(s)).collect()),
        ),
        ("heights".to_owned(), heights(&g.network, "heights")?),
        ("terrain_heights".to_owned(), heights(&g.terrain, "terrain_heights")?),
        (
            "smoothing_cells".to_owned(),
            Json::Array(g.smoothing.cells.iter().map(|&k| ji(k)).collect()),
        ),
        (
            "boundary".to_owned(),
            Json::Array(g.smoothing.boundary.iter().map(|&k| ji(k)).collect()),
        ),
    ];
    check_state(&got, want)
}

// ---- the limits and the satisfier ---------------------------------------------------------------

/// `ng_type_info`: `type`; `state` checks `slope_rise`,
/// `diagonal_slope_rise`, `smoothness`, `diagonal_smoothness`,
/// `tunnel_trigger_rise`, `widths`, `straight_only` (the shipped values).
fn type_info(op: &Json) -> R<()> {
    let i = TypeInfo::shipped(type_of(op)?);
    let got = vec![
        ("slope_rise".to_owned(), jf(i.slope_rise)),
        ("diagonal_slope_rise".to_owned(), jf(i.diagonal_slope_rise)),
        ("smoothness".to_owned(), jf(i.smoothness)),
        ("diagonal_smoothness".to_owned(), jf(i.diagonal_smoothness)),
        ("tunnel_trigger_rise".to_owned(), jf(i.tunnel_trigger_rise)),
        ("widths".to_owned(), Json::Array(vec![ji(i.widths.0), ji(i.widths.1)])),
        ("straight_only".to_owned(), Json::Bool(i.params.straight_only)),
    ];
    check_state(&got, field(op, "state")?)
}

/// `ng_sat`: `sat?` (`"network"` (default) or `"terrain"`), `call`, `args?`,
/// `expect?`. Calls: `init` [lo, hi], `shutdown`, `add` [v, h], `fixed`
/// [v, h], `range` [v, a, b], `equality` [a, b], `slope` [a, b, s],
/// `smoothness` [a, b, c, s], `preferred` [v, h], `preferred_range`
/// [v, a, b], `solve` (→ bool), `height` [v] (→ h or null), `range_of` [v]
/// (→ [min, max] or null), `solution` (→ [[v, h]]).
fn sat(w: &World, op: &Json) -> R<()> {
    let which = opt(op, "sat")
        .map(|j| str_of(j, "sat"))
        .transpose()?
        .unwrap_or("network");
    let call = str_of(field(op, "call")?, "call")?;
    let args: Vec<Json> = opt(op, "args")
        .map(|j| arr(j, "args").map(<[Json]>::to_vec))
        .transpose()?
        .unwrap_or_default();
    let f = |i: usize| -> R<f32> { fl(args.get(i).ok_or_else(|| format!("missing argument {i}"))?, "arg") };
    let v = |i: usize| -> R<i32> { int(args.get(i).ok_or_else(|| format!("missing argument {i}"))?, "arg") };
    let mut g = w.geo.borrow_mut();
    let s = match which {
        "network" => &mut g.network,
        "terrain" => &mut g.terrain,
        other => return Err(format!("unknown satisfier '{other}'")),
    };
    let got = match call {
        "init" => {
            s.init(f(0)?, f(1)?);
            Json::Null
        }
        "shutdown" => {
            s.shutdown();
            Json::Null
        }
        "add" => Json::Bool(s.add_vertex(v(0)?, f(1)?)),
        "fixed" => {
            s.insert_fixed(v(0)?, f(1)?);
            Json::Null
        }
        "range" => {
            s.insert_range(v(0)?, f(1)?, f(2)?);
            Json::Null
        }
        "equality" => {
            s.insert_equality(v(0)?, v(1)?);
            Json::Null
        }
        "slope" => {
            s.insert_slope(v(0)?, v(1)?, f(2)?);
            Json::Null
        }
        "smoothness" => {
            s.insert_smoothness(v(0)?, v(1)?, v(2)?, f(3)?);
            Json::Null
        }
        "preferred" => {
            s.set_preferred_value(v(0)?, f(1)?);
            Json::Null
        }
        "preferred_range" => {
            s.set_preferred_range(v(0)?, f(1)?, f(2)?);
            Json::Null
        }
        "solve" => Json::Bool(s.solve()),
        "height" => s.height(v(0)?).map_or(Json::Null, jf),
        "range_of" => s
            .range(v(0)?)
            .map_or(Json::Null, |(a, b)| Json::Array(vec![jf(a), jf(b)])),
        "solution" => Json::Array(
            s.solution()
                .iter()
                .map(|&(id, h)| Json::Array(vec![ji(id), jf(h)]))
                .collect(),
        ),
        other => return Err(format!("unknown satisfier call '{other}'")),
    };
    check_expect(op, got)
}

/// `ng_terrain_vertex`: `n`, `t`, `supports`, `fixed?`, `type?`; `expect`
/// `{"fix": h}`, `{"range": [lo, hi], "preferred": p}` or `"unsupported"`.
fn terrain_vertex(op: &Json) -> R<()> {
    let info = TypeInfo::shipped(type_of(op)?);
    let n = fl(field(op, "n")?, "n")?;
    let t = fl(field(op, "t")?, "t")?;
    let sup = bool_of(field(op, "supports")?, "supports")?;
    let fixed = opt(op, "fixed")
        .map(|j| bool_of(j, "fixed"))
        .transpose()?
        .unwrap_or(false);
    let got = match terrain_vertex_constraint(n, t, sup, fixed, &info) {
        TerrainVertex::Unsupported => Json::Str("unsupported".into()),
        TerrainVertex::Fix(h) => Json::Object(vec![("fix".into(), jf(h))]),
        TerrainVertex::Range { lo, hi, preferred } => Json::Object(vec![
            ("range".into(), Json::Array(vec![jf(lo), jf(hi)])),
            ("preferred".into(), jf(preferred)),
        ]),
    };
    check_expect(op, got)
}

// ---- §10, §11, §13 ------------------------------------------------------------------------------

/// `ng_support_level`: `corners` [4], `terrain` [4], `threshold`; `expect`
/// `{level, flags}`.
fn support_level(op: &Json) -> R<()> {
    let c = floats::<4>(field(op, "corners")?, "corners")?;
    let t = floats::<4>(field(op, "terrain")?, "terrain")?;
    let th = fl(field(op, "threshold")?, "threshold")?;
    let l = extras::support_level(c, t, th);
    check_expect(
        op,
        Json::Object(vec![
            ("level".into(), ji(l)),
            ("flags".into(), ji(extras::support_flags(l))),
        ]),
    )
}

/// `ng_occupant_flags`: `flags`, `real`, `tunnel?`; `expect` the flags.
fn occupant_flags(op: &Json) -> R<()> {
    let f = i64_of(field(op, "flags")?, "flags")? as u32;
    let real = bool_of(field(op, "real")?, "real")?;
    let tunnel = opt(op, "tunnel")
        .map(|j| bool_of(j, "tunnel"))
        .transpose()?
        .unwrap_or(false);
    check_expect(op, ji(extras::occupant_flags(f, real, tunnel)))
}

/// `ng_cost`: `GetCostOfSolution` of the placed run, `demolition?` (0).
fn cost(w: &World, op: &Json) -> R<()> {
    let d = opt(op, "demolition")
        .map(|j| int(j, "demolition"))
        .transpose()?
        .unwrap_or(0);
    let c = w.geo.borrow().cost_inputs(&*w.terrain.borrow(), d);
    check_expect(op, ji(extras::cost_of_solution(&c)))
}

/// `ng_power_line_cost`: `poles`, `wire`, `per_pole?` (15), `per_wire?` (2.0).
fn power_line_cost(op: &Json) -> R<()> {
    let poles = int(field(op, "poles")?, "poles")? as u32;
    let wire = int(field(op, "wire")?, "wire")? as u32;
    let pp = opt(op, "per_pole")
        .map(|j| int(j, "per_pole"))
        .transpose()?
        .unwrap_or(15);
    let pw = opt(op, "per_wire")
        .map(|j| fl(j, "per_wire"))
        .transpose()?
        .unwrap_or(2.0);
    check_expect(op, ji(extras::power_line_cost(poles, wire, pp, pw)))
}

/// `ng_destroy`: `destroyed` [[x, z]], `pieces` [[x, z]] (in removal
/// order), `replaced` [[x, z]] (where `RemoveOrReplaceOccupant` puts a piece
/// back); `expect` `{removals: ["remove" | "replace"], resolve: [[x, z]]}`
/// (the re-solve set, sorted by key).
fn destroy(w: &World, op: &Json) -> R<()> {
    let key = |(x, z): (i32, i32)| extras::destroy_key(x, z);
    let destroyed: BTreeSet<i32> = pairs(field(op, "destroyed")?, "destroyed")?
        .into_iter()
        .map(key)
        .collect();
    let replaced = pairs(field(op, "replaced")?, "replaced")?;
    let mut resolve = w.resolve.borrow_mut();
    resolve.clear();
    let mut removals = Vec::new();
    for p in pairs(field(op, "pieces")?, "pieces")? {
        let k = key(p);
        let how = extras::removal_for(k, &destroyed);
        if how == Removal::RemoveOrReplace {
            extras::update_resolve_set(&mut resolve, k, replaced.contains(&p));
        }
        removals.push(Json::Str(
            match how {
                Removal::Remove => "remove",
                Removal::RemoveOrReplace => "replace",
            }
            .into(),
        ));
    }
    let res = resolve.iter().map(|&k| jpair((k >> 16, k & 0xFFFF))).collect();
    check_expect(
        op,
        Json::Object(vec![
            ("removals".into(), Json::Array(removals)),
            ("resolve".into(), Json::Array(res)),
        ]),
    )
}

/// `ng_grade`: slot 57 on the terrain satisfier; `expect?` `{changed: [ids],
/// rect: [x0, z0, x1, z1] or null}`. The terrain's posts go to the event
/// log.
fn grade(w: &World, op: &Json) -> R<()> {
    let g = w.geo.borrow_mut().update_terrain_heights(&mut *w.terrain.borrow_mut());
    check_expect(
        op,
        Json::Object(vec![
            (
                "changed".into(),
                Json::Array(g.changed.iter().map(|&v| ji(v)).collect()),
            ),
            (
                "rect".into(),
                g.rect.map_or(Json::Null, |r| {
                    Json::Array(vec![ji(r.x0), ji(r.z0), ji(r.x1), ji(r.z1)])
                }),
            ),
        ]),
    )
}
