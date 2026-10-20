// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/network-bridges.md` (`nb_…`): a city (a
//! `fc4_core::network_geometry` terrain view), a scripted bridge host and a
//! [`BridgeTool`]. See README.md ("Network bridge ops").

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use fc4_core::network_bridge::{
    self, cost, elev, getters, land, model, pieces, select, span, specialize, types, BridgeHost, BridgeRecord,
    BridgeTerrain, BridgeTool,
};
use fc4_core::network_geometry::smoothing::CellInfo;
use fc4_core::network_geometry::{GeometryTerrain, Run};

use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{self, check_expect, check_state, jf, ji, R};

/// A flat city: `h` everywhere, `sea` as the water level, and a set of wet
/// cells.
#[derive(Clone, Debug)]
struct City {
    cells: (u32, u32),
    h: f32,
    sea: f32,
    wet: Vec<(i32, i32)>,
    /// A per-cell height override.
    over: BTreeMap<(i32, i32), f32>,
}

impl City {
    fn new() -> City {
        City {
            cells: (64, 64),
            h: 300.0,
            sea: 250.0,
            wet: Vec::new(),
            over: BTreeMap::new(),
        }
    }
    fn at(&self, x: i32, z: i32) -> f32 {
        self.over.get(&(x, z)).copied().unwrap_or(self.h)
    }
    fn wet(&self, x: i32, z: i32) -> bool {
        self.wet.contains(&(x, z))
    }
}

impl GeometryTerrain for City {
    fn cells(&self) -> (u32, u32) {
        self.cells
    }
    fn vertex_altitude(&self, _v: i32) -> f32 {
        self.h
    }
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        self.wet(x, z)
    }
    fn surface_water_level(&self, _v: i32) -> f32 {
        self.sea
    }
    fn max_allowed_height(&self) -> f32 {
        1000.0
    }
}

impl BridgeTerrain for City {
    fn lowest_corner(&self, x: i32, z: i32) -> f32 {
        self.at(x, z)
    }
    fn surface_level(&self, x: i32, z: i32) -> f32 {
        if self.wet(x, z) {
            self.sea
        } else {
            self.at(x, z)
        }
    }
    fn water_level(&self, _x: i32, _z: i32) -> f32 {
        self.sea
    }
}

impl span::StepMeasures for City {}

/// A host with a set of blocked rectangles (§4.2's OR filter, §7.3's
/// unfiltered query).
/// A blocked rectangle and the filter mask it blocks under: a mask of
/// None blocks everything (7.3 step 2).
type Block = (((i32, i32), (i32, i32)), Option<u32>);

#[derive(Clone, Default)]
struct Host {
    /// `(rect, mask)`: `mask` `None` blocks everything (§7.3 step 2).
    blocked: Vec<Block>,
}

impl BridgeHost for Host {
    fn occupant_present(&self, rect: ((i32, i32), (i32, i32)), mask: Option<u32>) -> bool {
        self.blocked.iter().any(|(r, m)| {
            let overlap = r.0 .0 <= rect.1 .0 && r.1 .0 >= rect.0 .0 && r.0 .1 <= rect.1 .1 && r.1 .1 >= rect.0 .1;
            overlap && (m.is_none() || mask.is_none() || *m == mask)
        })
    }
}

/// The per-vector world.
struct World {
    city: City,
    host: Host,
    tool: BridgeTool,
    run: Run,
    valid: u32,
    infos: BTreeMap<i32, CellInfo>,
    /// The placement parameter p+0x00, the maximum slope in radians.
    max_slope: f32,
}

impl World {
    fn new() -> Rc<RefCell<World>> {
        Rc::new(RefCell::new(World {
            city: City::new(),
            host: Host::default(),
            tool: BridgeTool::new(types::shipped_records(0)),
            run: Run::default(),
            valid: 0,
            infos: BTreeMap::new(),
            max_slope: 0.610_865_24,
        }))
    }
}

fn world(host: &HostRef) -> Rc<RefCell<World>> {
    scalar::world(host, "network_bridge", World::new)
}

fn int_at(v: &Json, i: usize, j: usize) -> R<i64> {
    v.as_array()
        .and_then(|a| a.get(i))
        .and_then(|p| p.as_array())
        .and_then(|p| p.get(j))
        .and_then(Json::as_i64)
        .ok_or_else(|| format!("missing nested index [{i}][{j}]"))
}

/// A required integer field.
fn ikey(op: &Json, k: &str) -> R<i64> {
    field(op, k)?
        .as_i64()
        .ok_or_else(|| format!("{k}: expected an integer"))
}

fn skey(op: &Json, k: &str) -> R<String> {
    field(op, k)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("{k}: expected a string"))
}
/// A required array field.
fn akey(op: &Json, k: &str) -> R<Vec<Json>> {
    field(op, k)?
        .as_array()
        .map(|a| a.to_vec())
        .ok_or_else(|| format!("{k}: expected an array"))
}

fn need_f(v: &Json) -> R<f64> {
    v.as_f64().ok_or_else(|| "expected a number".to_string())
}
/// A required integer value.
fn need_i(v: &Json) -> R<i64> {
    v.as_i64().ok_or_else(|| "expected an integer".to_string())
}

fn need_a(v: &Json) -> R<Vec<Json>> {
    v.as_array()
        .map(|a| a.to_vec())
        .ok_or_else(|| "expected an array".to_string())
}

///  ops.
pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    name.strip_prefix("nbr_")?;
    Some(dispatch(host, name, op))
}

/// The typed array accessors the ops use.
trait Args {
    fn i64(&self, i: usize, what: &str) -> R<i64>;
    fn f64(&self, i: usize, what: &str) -> R<f64>;
}

impl Args for Json {
    fn i64(&self, i: usize, what: &str) -> R<i64> {
        self.as_array()
            .and_then(|a| a.get(i))
            .and_then(Json::as_i64)
            .ok_or_else(|| format!("{what}[{i}]: expected an integer"))
    }
    fn f64(&self, i: usize, what: &str) -> R<f64> {
        self.as_array()
            .and_then(|a| a.get(i))
            .and_then(Json::as_f64)
            .ok_or_else(|| format!("{what}[{i}]: expected a number"))
    }
}

fn dispatch(host: &HostRef, name: &str, op: &Json) -> R<()> {
    let w = world(host);
    match name {
        "nbr_city" => city(&w, op),
        "nbr_host" => set_host(&w, op),
        "nbr_types" => set_types(&w, op),
        "nbr_drag" => drag(&w, op),
        "nbr_water" => water(&w, op),
        "nbr_land_scan" => land_scan(&w, op),
        "nbr_adjust" => adjust(&w, op),
        "nbr_pad" => pad(&w, op),
        "nbr_best" => best(&w, op),
        "nbr_model" => build_model(&w, op),
        "nbr_pins" => pins(&w, op),
        "nbr_cost" => cost_op(&w, op),
        "nbr_getter" => getter(&w, op),
        "nbr_edge_mask" => edge_mask(op),
        "nbr_orient" => orient(op),
        "nbr_ramp_heights" => ramp_heights(op),
        "nbr_parse" => parse(op),
        "nbr_expect" => expect(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    }
}

fn city(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let mut g = w.borrow_mut();
    if let Some(c) = op.get("cells") {
        g.city.cells = (c.i64(0, "arg0")? as u32, c.i64(1, "arg1")? as u32);
    }
    if let Some(v) = op.get("altitude") {
        g.city.h = need_f(v)? as f32;
    }
    if let Some(v) = op.get("sea") {
        g.city.sea = need_f(v)? as f32;
    }
    g.city.wet.clear();
    g.city.over.clear();
    if let Some(cols) = op.get("wet_columns") {
        for c in need_a(cols)? {
            let x0 = c.i64(0, "arg0")? as i32;
            let x1 = c.i64(1, "arg1")? as i32;
            let z = c
                .as_array()
                .and_then(|a| a.get(2))
                .and_then(|v| v.as_i64())
                .unwrap_or(10) as i32;
            for x in x0..=x1 {
                g.city.wet.push((x, z));
            }
        }
    }
    if let Some(cols) = op.get("heights") {
        for c in need_a(cols)? {
            g.city.over.insert(
                (c.i64(0, "arg0")? as i32, c.i64(1, "arg1")? as i32),
                c.f64(2, "arg2")? as f32,
            );
        }
    }
    Ok(())
}

fn set_host(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let mut g = w.borrow_mut();
    if op.get("clear").and_then(|v| v.as_bool()).unwrap_or(false) {
        g.host.blocked.clear();
    }
    if let Some(b) = op.get("blocked") {
        for e in need_a(b)? {
            let r = e.get("rect").ok_or_else(|| "blocked.rect".to_string())?;
            let rect = (
                (int_at(r, 0, 0)? as i32, int_at(r, 0, 1)? as i32),
                (int_at(r, 1, 0)? as i32, int_at(r, 1, 1)? as i32),
            );
            let mask = e.get("mask").and_then(|v| v.as_i64()).map(|v| v as u32);
            g.host.blocked.push((rect, mask));
        }
    }
    if let Some(v) = op.get("max_slope") {
        g.max_slope = need_f(v)? as f32;
    }
    if let Some(v) = op.get("preferred") {
        g.tool.preferred = need_i(v)? as i32;
    }
    if let Some(v) = op.get("override") {
        g.tool.altitude_override = need_f(v)? as f32;
    }
    Ok(())
}

fn set_types(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let mut g = w.borrow_mut();
    let t = ikey(op, "type")? as u32;
    g.tool = BridgeTool::new(types::shipped_records(t));
    Ok(())
}

/// A straight x-drag of `n` one-cell steps at row `z`, pen 4.
fn drag(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let mut g = w.borrow_mut();
    let n = ikey(op, "steps")? as u32;
    let z = op.get("z").and_then(|v| v.as_i64()).unwrap_or(10) as i32;
    let pen = op.get("pen").and_then(|v| v.as_i64()).unwrap_or(4) as u8;
    let cells: Vec<(i32, i32)> = (0..n as i32).map(|x| (x, z)).collect();
    let steps = (0..n)
        .map(|i| fc4_core::network_geometry::drag::Step {
            first: i,
            last: i,
            kind: fc4_core::network_geometry::drag::StepKind::Regular,
        })
        .collect();
    g.run = Run {
        type_id: 0,
        cells,
        steps,
        dragged_count: n,
        straight_count: n,
        pen_direction: pen,
        end_direction: 5,
        ..Run::default()
    };
    g.valid = n;
    Ok(())
}

fn water(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let want = op.get("expect").cloned();
    let (city, host, mut tool, mut run, valid, straight, max_slope) = {
        let g = w.borrow();
        (
            g.city.clone(),
            g.host.clone(),
            g.tool.clone(),
            g.run.clone(),
            g.valid,
            g.run.straight_count,
            g.max_slope,
        )
    };
    let infos: BTreeMap<i32, CellInfo> = BTreeMap::new();
    let mut wv = span::SpanWorld {
        run: &mut run,
        valid,
        straight,
        type_id: 0,
        terrain: &city,
        host: &host,
        infos: &|x, z| {
            infos
                .get(&fc4_core::network_geometry::smoothing::cell_key(x, z))
                .cloned()
        },
    };
    let mut v = valid;
    let recorded = network_bridge::identify_bridge_cells(&mut tool, &mut wv, &mut v, max_slope);
    {
        let mut g = w.borrow_mut();
        g.tool = tool;
        g.run = run;
        g.valid = v;
    }
    if want.is_some() {
        check_expect(op, ji(recorded))?;
    }
    Ok(())
}

fn land_scan(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let max_h = op.get("max_height").and_then(|v| v.as_f64()).unwrap_or(55.0) as f32;
    let tol = op.get("bank_tolerance").and_then(|v| v.as_f64()).unwrap_or(20.0) as f32;
    let (city, host, mut tool, mut run, valid, straight, city_x) = {
        let g = w.borrow();
        (
            g.city.clone(),
            g.host.clone(),
            g.tool.clone(),
            g.run.clone(),
            g.valid,
            g.run.straight_count,
            g.city.cells.0 as i32,
        )
    };
    let infos: BTreeMap<i32, CellInfo> = BTreeMap::new();
    let mut wv = span::SpanWorld {
        run: &mut run,
        valid,
        straight,
        type_id: 0,
        terrain: &city,
        host: &host,
        infos: &|x, z| {
            infos
                .get(&fc4_core::network_geometry::smoothing::cell_key(x, z))
                .cloned()
        },
    };
    land::identify_land_bridge_cells(&mut tool, &mut wv, valid, max_h, tol, city_x);
    {
        let mut g = w.borrow_mut();
        g.tool = tool;
        g.run = run;
    }
    Ok(())
}

fn adjust(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let start = ikey(op, "start")? as u32;
    let end = ikey(op, "end")? as u32;
    let (city, host, run, valid, straight, slope, over) = {
        let g = w.borrow();
        (
            g.city.clone(),
            g.host.clone(),
            g.run.clone(),
            g.valid,
            g.run.straight_count,
            g.max_slope,
            g.tool.altitude_override,
        )
    };
    let mut run = run;
    let infos: BTreeMap<i32, CellInfo> = BTreeMap::new();
    let wv = span::SpanWorld {
        run: &mut run,
        valid,
        straight,
        type_id: 0,
        terrain: &city,
        host: &host,
        infos: &|x, z| {
            infos
                .get(&fc4_core::network_geometry::smoothing::cell_key(x, z))
                .cloned()
        },
    };
    let a = span::adjust_bridge_span(&wv, start, end, slope, over);
    match a {
        Some(a) => {
            let state = vec![
                ("start".into(), ji(a.start)),
                ("end".into(), ji(a.end)),
                ("clearance".into(), jf(a.clearance)),
                ("reference".into(), jf(a.reference)),
            ];
            check_state(&state, field(op, "expect")?)
        }
        None => Err("adjust_bridge_span failed".to_string()),
    }
}

fn pad(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let mut start = ikey(op, "start")? as u32;
    let mut end = ikey(op, "end")? as u32;
    let min_len = ikey(op, "min_len")? as u32;
    let (city, host, mut run, valid, straight) = {
        let g = w.borrow();
        (
            g.city.clone(),
            g.host.clone(),
            g.run.clone(),
            g.valid,
            g.run.straight_count,
        )
    };
    let infos: BTreeMap<i32, CellInfo> = BTreeMap::new();
    let wv = span::SpanWorld {
        run: &mut run,
        valid,
        straight,
        type_id: 0,
        terrain: &city,
        host: &host,
        infos: &|x, z| {
            infos
                .get(&fc4_core::network_geometry::smoothing::cell_key(x, z))
                .cloned()
        },
    };
    let ok = select::pad_steps_to_make_min_bridge_span(&wv, min_len, &mut start, &mut end);
    let state = vec![
        ("ok".into(), Json::Bool(ok)),
        ("start".into(), ji(start)),
        ("end".into(), ji(end)),
    ];
    check_state(&state, field(op, "expect")?)
}

fn best(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let mut start = ikey(op, "start")? as u32;
    let mut end = ikey(op, "end")? as u32;
    let water_list = op.get("water").and_then(|v| v.as_bool()).unwrap_or(true);
    let h = op.get("h").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let (city, host, mut tool, mut run, valid, straight) = {
        let g = w.borrow();
        (
            g.city.clone(),
            g.host.clone(),
            g.tool.clone(),
            g.run.clone(),
            g.valid,
            g.run.straight_count,
        )
    };
    let infos: BTreeMap<i32, CellInfo> = BTreeMap::new();
    let wv = span::SpanWorld {
        run: &mut run,
        valid,
        straight,
        type_id: 0,
        terrain: &city,
        host: &host,
        infos: &|x, z| {
            infos
                .get(&fc4_core::network_geometry::smoothing::cell_key(x, z))
                .cloned()
        },
    };
    let index = select::find_best_bridge_type_recording(&mut tool, &wv, &mut start, &mut end, water_list, h);
    let cand = tool.candidates.clone();
    {
        let mut g = w.borrow_mut();
        g.tool = tool;
    }
    let state = vec![
        (
            "index".into(),
            match index {
                Some(i) => ji(i),
                None => Json::Null,
            },
        ),
        ("start".into(), ji(start)),
        ("end".into(), ji(end)),
        ("candidates".into(), Json::Array(cand.into_iter().map(ji).collect())),
    ];
    check_state(&state, field(op, "expect")?)
}

fn build_model(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let index = ikey(op, "bridge_type")? as usize;
    let start = ikey(op, "start")? as u32;
    let end = ikey(op, "end")? as u32;
    let deck = op.get("deck").and_then(|v| v.as_f64()).unwrap_or(270.0) as f32;
    let pen = op.get("pen").and_then(|v| v.as_i64()).unwrap_or(4) as u8;
    let z = op.get("z").and_then(|v| v.as_i64()).unwrap_or(10) as i32;
    let mut g = w.borrow_mut();
    let bt = g.tool.types.get(index).cloned().ok_or("bridge_type")?;
    let o = model::Orientation::of(pen);
    let mut rec = BridgeRecord {
        type_id: 0,
        start,
        end,
        bridge_type: index as u32,
        deck,
        reference: deck,
        land: 0,
        pieces: Vec::new(),
    };
    let cells_of = |s: u32| -> Vec<(i32, i32)> { vec![(s as i32, z)] };
    if op.get("specialized").is_some() {
        let entry = specialize::Specialization {
            pieces: Vec::new(),
            heights: vec![0.0],
            height_add: 0.0,
        };
        model::construct_specialized_bridge(&mut rec, &entry, &bt, &o, &cells_of);
    } else {
        model::construct_parametric_bridge(&mut rec, &bt, &o, &cells_of);
    }
    g.tool.records.push(rec);
    Ok(())
}

fn pins(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let valid = op.get("valid").and_then(|v| v.as_i64()).unwrap_or(i64::MAX) as u32;
    let (tool, run, mut infos) = {
        let g = w.borrow();
        (g.tool.clone(), g.run.clone(), g.infos.clone())
    };
    let entries = elev::bridge_elevation_constraints(&tool, &run, valid, &mut infos);
    w.borrow_mut().infos = infos;
    let out: Vec<Json> = entries
        .iter()
        .map(|e| Json::Array(vec![ji(e.start), ji(e.end), jf(e.height), jf(e.height)]))
        .collect();
    check_expect(op, Json::Array(out))
}

fn cost_op(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let per_step = op.get("per_step").and_then(|v| v.as_i64()).unwrap_or(100) as u32;
    let g = w.borrow();
    let total = cost::total_bridge_cost(&g.tool, per_step);
    check_expect(op, ji(total))
}

fn getter(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    use getters::ToolBridgeGetters as G;
    let g = w.borrow();
    let which = skey(op, "slot")?;
    let got = match which.as_str() {
        "major" => ji(G::get_major_bridge(&g.tool)),
        "altitude" => match G::get_major_bridge_altitude(&g.tool) {
            Some(h) => jf(h),
            None => Json::Bool(false),
        },
        "reference" => match G::get_major_bridge_reference_altitude(&g.tool) {
            Some(h) => jf(h),
            None => Json::Bool(false),
        },
        "land" => match G::is_major_bridge_land_bridge(&g.tool) {
            Some(b) => Json::Bool(b),
            None => Json::Bool(false),
        },
        "preferred" => ji(G::get_preferred_bridge(&g.tool)),
        "override" => jf(G::get_bridge_altitude_override(&g.tool)),
        "bitmap" => {
            let i = op.get("index").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
            match G::get_bridge_bitmap_key(&g.tool, i) {
                Some((a, b)) => Json::Array(vec![ji(a), ji(b)]),
                None => Json::Bool(false),
            }
        }
        "candidates" => {
            let mut list = Vec::new();
            let ok = G::get_possible_bridges(&g.tool, &mut list);
            Json::Object(vec![
                ("ok".into(), Json::Bool(ok)),
                ("list".into(), Json::Array(list.into_iter().map(ji).collect())),
            ])
        }
        other => return Err(format!("unknown slot '{other}'")),
    };
    check_expect(op, got)
}

fn edge_mask(op: &Json) -> R<()> {
    let pen = ikey(op, "pen")? as u8;
    let index = ikey(op, "index")? as usize;
    let n = ikey(op, "n")? as usize;
    check_expect(op, ji(span::EdgeMask::allowed(pen, index, n)))
}

fn orient(op: &Json) -> R<()> {
    let pen = ikey(op, "pen")? as u8;
    let o = model::Orientation::of(pen);
    let state = vec![
        ("r".into(), ji(o.r)),
        ("t".into(), ji(o.t)),
        ("t_mirror".into(), ji(o.t_mirror)),
        ("forward".into(), Json::Bool(o.lanes_forward)),
    ];
    check_state(&state, field(op, "expect")?)
}

fn ramp_heights(op: &Json) -> R<()> {
    let pairs: Vec<(f32, f32)> = akey(op, "pairs")?
        .iter()
        .map(|p| Ok((p.f64(0, "arg0")? as f32, p.f64(1, "arg1")? as f32)))
        .collect::<R<Vec<_>>>()?;
    let (drops, offsets) = types::ramp_heights(&pairs);
    let state = vec![
        ("rise".into(), jf(*drops.first().unwrap_or(&0.0))),
        ("drops".into(), Json::Array(drops.iter().map(|&v| jf(v)).collect())),
        ("offsets".into(), Json::Array(offsets.iter().map(|&v| jf(v)).collect())),
        (
            "last_column".into(),
            ji(types::last_column_ramp(&vec![-1.0; pairs.len()])),
        ),
    ];
    check_state(&state, field(op, "expect")?)
}

fn parse(op: &Json) -> R<()> {
    let text = skey(op, "text")?;
    let keys = akey(op, "keys")?
        .iter()
        .map(|v| Ok(need_i(v)? as u32))
        .collect::<R<Vec<_>>>()?;
    let n = keys.len();
    let mins: Vec<u32> = match op.get("min_span") {
        Some(a) => need_a(a)?
            .iter()
            .map(|v| Ok(need_i(v)? as u32))
            .collect::<R<Vec<_>>>()?,
        None => vec![4; n],
    };
    let maxs = vec![types::DEFAULT_MAX_SPAN; n];
    let widths = vec![1u32; n];
    let (maps, ok) = specialize::parse_specializations(&text, &keys, &widths, &mins, &maxs);
    let state = vec![
        ("ok".into(), Json::Bool(ok)),
        (
            "lengths".into(),
            Json::Array(
                maps.iter()
                    .map(|m| Json::Array(m.keys().map(|&k| ji(k)).collect()))
                    .collect(),
            ),
        ),
    ];
    check_state(&state, field(op, "expect")?)
}

fn expect(w: &Rc<RefCell<World>>, op: &Json) -> R<()> {
    let g = w.borrow();
    let mut state: Vec<(String, Json)> = Vec::new();
    if let Some(v) = op.get("valid") {
        state.push(("valid".into(), ji(g.valid)));
        let _ = v;
    }
    if op.get("records").is_some() {
        let recs: Vec<Json> = g
            .tool
            .records
            .iter()
            .map(|r| {
                Json::Array(vec![
                    ji(r.type_id),
                    ji(r.start),
                    ji(r.end),
                    ji(r.bridge_type),
                    jf(r.deck),
                    jf(r.reference),
                    ji(r.land),
                    ji(r.pieces.len() as i64),
                ])
            })
            .collect();
        state.push(("records".into(), Json::Array(recs)));
    }
    if op.get("candidates").is_some() {
        state.push((
            "candidates".into(),
            Json::Array(g.tool.candidates.iter().map(|&i| ji(i)).collect()),
        ));
    }
    if op.get("kinds").is_some() {
        state.push((
            "kinds".into(),
            Json::Array(g.run.steps.iter().map(|s| ji(s.kind as u32)).collect()),
        ));
    }
    if op.get("piece_kinds").is_some() {
        let mut out: Vec<Json> = Vec::new();
        for r in &g.tool.records {
            for p in &r.pieces {
                out.push(Json::Array(vec![
                    ji(p.kind as u32),
                    ji(p.x),
                    ji(p.z),
                    jf(p.depth),
                    jf(p.y),
                    ji(p.rotation),
                ]));
            }
        }
        state.push(("piece_kinds".into(), Json::Array(out)));
    }
    if op.get("columns").is_some() {
        let mut out: Vec<Json> = Vec::new();
        for r in &g.tool.records {
            let Some(bt) = g.tool.type_of_record(r) else {
                continue;
            };
            for p in &r.pieces {
                let (_x, _z, height) = pieces::occupant_position(&p.corners, p.x, p.z, pieces::CELL_SIZE);
                for c in pieces::pillar_columns(p, bt, height, [0.0; 4], -1.0) {
                    out.push(Json::Array(vec![jf(c.bottom), jf(c.top), Json::Bool(c.underwater)]));
                }
            }
        }
        state.push(("columns".into(), Json::Array(out)));
    }
    check_state(&state, field(op, "expect")?)
}
