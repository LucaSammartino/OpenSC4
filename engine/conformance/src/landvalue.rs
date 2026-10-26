// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/landvalue.md`: the land value simulator
//! against a fake terrain, zone manager, lot developer, growth developers,
//! grids and history warehouse. See README.md ("Land value ops").

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use fc4_core::curve::ResponseCurve;
use fc4_core::grid::{CityDims, GridElem, SimGrid, TractGeometry, TractGrid};
use fc4_core::landvalue::{desirability_term, AltitudeParams, GridKind, LandValueHost, LandValueSim, TuningInput};
use fc4_core::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, check_expect_bool, field, int, opt_int, HostRef};
use crate::simgrid::{close, num_of, nums, triples, AnyGrid};

type R<T> = Result<T, String>;

/// The shipped "Land Value Sim" exemplar values (landvalue.md §8).
const SHIPPED_ALTITUDE_CURVE: [f32; 14] = [
    0.0, 0.0, 25.0, 5.0, 50.0, 10.0, 75.0, 21.0, 100.0, 50.0, 125.0, 30.0, 150.0, 0.0,
];
const SHIPPED_DESIRABILITY_CURVE: [f32; 4] = [-255.0, -255.0, 255.0, 255.0];

fn shipped_tuning() -> TuningInput {
    TuningInput {
        tract_size: Some(2),
        intrinsic: Some(vec![1, 64]),
        wealth: Some(vec![70, 120]),
        water: Some(vec![50, 10]),
        altitude_curve: Some(SHIPPED_ALTITUDE_CURVE.to_vec()),
        developer_ids: Some(vec![0x1030]),
        developer_weights: Some(vec![2.5]),
        desirability_curve: Some(SHIPPED_DESIRABILITY_CURVE.to_vec()),
    }
}

struct FakeGrid {
    grid: AnyGrid,
    label: String,
    refs: i32,
}

/// The fake game behind the [`LandValueHost`].
struct Data {
    city: CityDims,
    land_default: bool,
    /// Vertex rectangles `[x0, z0, x1, z1]` (inclusive), applied in order.
    land_rects: Vec<([i32; 4], bool)>,
    alt_default: f32,
    /// Altitude by terrain vertex (the sample position divided by the cell width).
    altitudes: HashMap<(i32, i32), f32>,
    min_height: f32,
    max_height: f32,
    /// Cell rectangles `[x0, z0, x1, z1]` (inclusive) with a zone type.
    zones: Vec<([i32; 4], u32)>,
    /// Developer type → developer token.
    developers: HashMap<u32, u64>,
    /// Developer token → desirability grid token (absent = no map).
    maps: HashMap<u64, u64>,
    grids: HashMap<u64, FakeGrid>,
    next_token: u64,
    events: Vec<String>,
    #[cfg(test)]
    reads: Vec<&'static str>,
    #[cfg(test)]
    slope_input: Option<f32>,
}

impl Data {
    fn new() -> Data {
        Data {
            city: CityDims::default(),
            land_default: true,
            land_rects: Vec::new(),
            alt_default: 100.0,
            altitudes: HashMap::new(),
            min_height: 20.0,
            max_height: 2500.0,
            zones: Vec::new(),
            developers: HashMap::new(),
            maps: HashMap::new(),
            grids: HashMap::new(),
            next_token: 0x100,
            events: Vec::new(),
            #[cfg(test)]
            reads: Vec::new(),
            #[cfg(test)]
            slope_input: None,
        }
    }

    fn token(&mut self) -> u64 {
        self.next_token += 1;
        self.next_token
    }

    fn add_grid(&mut self, grid: AnyGrid, label: String) -> u64 {
        let t = self.token();
        self.grids.insert(t, FakeGrid { grid, label, refs: 1 });
        t
    }

    fn label(&self, t: u64) -> String {
        self.grids.get(&t).map_or_else(|| format!("#{t}"), |g| g.label.clone())
    }
}

fn in_rect(r: &[i32; 4], x: i32, z: i32) -> bool {
    x >= r[0] && z >= r[1] && x <= r[2] && z <= r[3]
}

/// The class IDs the resolver accepts.
const CLSID_U8: u32 = <u8 as GridElem>::CLSID;
const CLSID_F32: u32 = <f32 as GridElem>::CLSID;

fn read_into<T: GridElem>(g: &SimGrid<T>, out: &mut [T]) {
    for (o, v) in out.iter_mut().zip(g.values()) {
        *o = v;
    }
}

struct FakeHost(Rc<RefCell<Data>>);

impl LandValueHost for FakeHost {
    fn terrain_cells(&self) -> (i32, i32) {
        let d = self.0.borrow();
        (d.city.cells_x, d.city.cells_z)
    }

    fn is_land_vertex(&self, vx: u32, vz: u32) -> bool {
        #[cfg(test)]
        self.0.borrow_mut().reads.push("land");
        let d = self.0.borrow();
        let (x, z) = (vx as i32, vz as i32);
        let mut land = d.land_default;
        for (r, v) in &d.land_rects {
            if in_rect(r, x, z) {
                land = *v;
            }
        }
        land
    }

    fn altitude(&self, px: f32, pz: f32) -> f32 {
        #[cfg(test)]
        self.0.borrow_mut().reads.push("altitude");
        let d = self.0.borrow();
        let vx = (px / d.city.cell_width_x).round() as i32;
        let vz = (pz / d.city.cell_width_z).round() as i32;
        d.altitudes.get(&(vx, vz)).copied().unwrap_or(d.alt_default)
    }

    fn min_allowed_height(&self) -> f32 {
        self.0.borrow().min_height
    }

    fn max_allowed_height(&self) -> f32 {
        self.0.borrow().max_height
    }

    fn zone_type(&self, x: i32, z: i32) -> u32 {
        #[cfg(test)]
        self.0.borrow_mut().reads.push("zone");
        let d = self.0.borrow();
        let mut zone = 0;
        for (r, t) in &d.zones {
            if in_rect(r, x, z) {
                zone = *t;
            }
        }
        zone
    }

    /// `x1 + 1000·z1 + (x2 − x1 + 1)/16 + (z2 − z1 + 1)/64`: exact in f32 and
    /// encodes the whole rectangle (an 8 × 8 rectangle adds 0.625).
    fn area_slope(&self, x1: i32, z1: i32, x2: i32, z2: i32) -> f32 {
        #[cfg(test)]
        {
            let mut d = self.0.borrow_mut();
            d.reads.push("slope");
            if let Some(v) = d.slope_input {
                return v;
            }
        }
        x1 as f32 + 1000.0 * z1 as f32 + (x2 - x1 + 1) as f32 / 16.0 + (z2 - z1 + 1) as f32 / 64.0
    }

    fn developer(&self, developer_type: u32) -> u64 {
        self.0.borrow().developers.get(&developer_type).copied().unwrap_or(0)
    }

    fn desirability_map(&self, developer: u64) -> u64 {
        self.0.borrow().maps.get(&developer).copied().unwrap_or(0)
    }

    fn create_grid(&self, kind: GridKind, tract_size: i32, instance_id: u32) -> u64 {
        let mut d = self.0.borrow_mut();
        let city = d.city;
        // Init, SetTractSize, SetInstanceID (simgrid §6.2).
        let (grid, name) = match kind {
            GridKind::U8 => (AnyGrid::U8(SimGrid::create(&city, instance_id, tract_size)), "u8"),
            GridKind::F32 => (AnyGrid::F32(SimGrid::create(&city, instance_id, tract_size)), "f32"),
        };
        d.events.push(format!("create {name} {tract_size} 0x{instance_id:08X}"));
        d.add_grid(grid, format!("0x{instance_id:08X}"))
    }

    fn grid_geometry(&self, grid: u64) -> TractGeometry {
        self.0
            .borrow()
            .grids
            .get(&grid)
            .map(|g| g.grid.geometry())
            .unwrap_or_default()
    }

    fn grid_set_tract_size(&self, grid: u64, tract_size: i32) {
        let mut d = self.0.borrow_mut();
        let city = d.city;
        let label = d.label(grid);
        d.events.push(format!("set_tract_size {label} {tract_size}"));
        if let Some(g) = d.grids.get(&grid) {
            g.grid.set_tract_size(tract_size, &city);
        }
    }

    fn grid_read_u8(&self, grid: u64, out: &mut [u8]) {
        if let Some(AnyGrid::U8(g)) = self.0.borrow().grids.get(&grid).map(|g| &g.grid) {
            read_into(g, out);
        }
    }

    fn grid_read_i16(&self, grid: u64, out: &mut [i16]) {
        if let Some(AnyGrid::I16(g)) = self.0.borrow().grids.get(&grid).map(|g| &g.grid) {
            read_into(g, out);
        }
    }

    fn grid_write_u8(&self, grid: u64, values: &[u8]) {
        if let Some(AnyGrid::U8(g)) = self.0.borrow().grids.get(&grid).map(|g| &g.grid) {
            g.set_values(values);
        }
    }

    fn grid_write_f32(&self, grid: u64, values: &[f32]) {
        if let Some(AnyGrid::F32(g)) = self.0.borrow().grids.get(&grid).map(|g| &g.grid) {
            g.set_values(values);
        }
    }

    fn grid_shutdown(&self, grid: u64) {
        let mut d = self.0.borrow_mut();
        let label = d.label(grid);
        d.events.push(format!("grid_shutdown {label}"));
    }

    fn release(&self, token: u64) {
        let mut d = self.0.borrow_mut();
        let label = d.label(token);
        d.events.push(format!("release {label}"));
        let gone = match d.grids.get_mut(&token) {
            Some(g) => {
                g.refs -= 1;
                g.refs <= 0
            }
            None => false,
        };
        if gone {
            d.grids.remove(&token);
        }
    }

    fn set_history_queue_length(&self, series: u32, length: u32) {
        self.0
            .borrow_mut()
            .events
            .push(format!("history_length 0x{series:08X} {length}"));
    }

    fn record_history(&self, series: u32, value: i32) {
        self.0
            .borrow_mut()
            .events
            .push(format!("history 0x{series:08X} {value}"));
    }

    fn post(&self, msg_type: u32) {
        self.0.borrow_mut().events.push(format!("post 0x{msg_type:08X}"));
    }
}

pub(crate) struct World {
    data: Rc<RefCell<Data>>,
    sim: RefCell<Option<LandValueSim>>,
}

fn world(host: &HostRef) -> Rc<World> {
    let hs = &host.0;
    if let Some(w) = hs.landvalue.borrow().as_ref() {
        return w.clone();
    }
    let w = Rc::new(World {
        data: Rc::new(RefCell::new(Data::new())),
        sim: RefCell::new(None),
    });
    *hs.landvalue.borrow_mut() = Some(w.clone());
    w
}

/// Executes a land value op; `None` if `name` is not one.
pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    if !name.starts_with("lv_") {
        return None;
    }
    let w = world(host);
    Some(run(&w, name, op))
}

fn rects(j: &Json) -> R<Vec<[i32; 4]>> {
    let mut out = Vec::new();
    for r in j.as_array().ok_or("expected a list of [x0, z0, x1, z1]")? {
        let v = nums(r)?;
        match v.len() {
            2 => out.push([v[0] as i32, v[1] as i32, v[0] as i32, v[1] as i32]),
            4 => out.push([v[0] as i32, v[1] as i32, v[2] as i32, v[3] as i32]),
            _ => return Err(format!("bad rectangle {r:?}")),
        }
    }
    Ok(out)
}

fn curve_arg(j: Option<&Json>) -> R<Option<Vec<f32>>> {
    match j {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Str(s)) if s == "shipped_altitude" => Ok(Some(SHIPPED_ALTITUDE_CURVE.to_vec())),
        Some(Json::Str(s)) if s == "shipped_desirability" => Ok(Some(SHIPPED_DESIRABILITY_CURVE.to_vec())),
        Some(j) => Ok(Some(nums(j)?.into_iter().map(|v| v as f32).collect())),
    }
}

fn ints(j: Option<&Json>) -> R<Option<Vec<i64>>> {
    match j {
        None | Some(Json::Null) => Ok(None),
        Some(j) => Ok(Some(nums(j)?.into_iter().map(|v| v as i64).collect())),
    }
}

/// `"shipped"`, or an object of properties (absent or null = property absent),
/// optionally starting from the shipped values with `"shipped": true`.
fn parse_tuning(j: &Json) -> R<TuningInput> {
    if j.as_str() == Some("shipped") {
        return Ok(shipped_tuning());
    }
    let obj = j.as_object().ok_or("'tuning' must be \"shipped\" or an object")?;
    let mut t = if j.get("shipped").and_then(boolean) == Some(true) {
        shipped_tuning()
    } else {
        TuningInput::default()
    };
    for (k, v) in obj {
        match k.as_str() {
            "shipped" | "description" => {}
            "tract_size" => t.tract_size = v.as_i64(),
            "intrinsic" => t.intrinsic = ints(Some(v))?,
            "wealth" => t.wealth = ints(Some(v))?,
            "water" => t.water = ints(Some(v))?,
            "altitude_curve" => t.altitude_curve = curve_arg(Some(v))?,
            "desirability_curve" => t.desirability_curve = curve_arg(Some(v))?,
            "developer_ids" => t.developer_ids = ints(Some(v))?.map(|v| v.into_iter().map(|x| x as u32).collect()),
            "developer_weights" => {
                t.developer_weights = match v {
                    Json::Null => None,
                    _ => Some(nums(v)?.into_iter().map(|x| x as f32).collect()),
                }
            }
            other => return Err(format!("unknown tuning key '{other}'")),
        }
    }
    Ok(t)
}

fn configure(d: &mut Data, op: &Json) -> R<()> {
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        match k.as_str() {
            "op" | "description" => {}
            "cells" => {
                let c = nums(v)?;
                d.city.cells_x = c[0] as i32;
                d.city.cells_z = c[1] as i32;
            }
            "cell_width" => {
                let w = num_of(v)? as f32;
                d.city.cell_width_x = w;
                d.city.cell_width_z = w;
            }
            "land" => d.land_default = boolean(v).ok_or("'land' must be a boolean")?,
            "water" => {
                d.land_rects.extend(rects(v)?.into_iter().map(|r| (r, false)));
            }
            "land_vertices" => {
                d.land_rects.extend(rects(v)?.into_iter().map(|r| (r, true)));
            }
            "clear_land" => d.land_rects.clear(),
            "altitude" => d.alt_default = num_of(v)? as f32,
            "altitudes" => {
                for (x, z, a) in triples(v)? {
                    d.altitudes.insert((x, z), a as f32);
                }
            }
            "heights" => {
                let h = nums(v)?;
                d.min_height = h[0] as f32;
                d.max_height = h[1] as f32;
            }
            "zoned" => d.zones.extend(rects(v)?.into_iter().map(|r| (r, 1))),
            "clear_zones" => d.zones.clear(),
            "developers" => {
                for (id, spec) in v.as_object().ok_or("'developers' must be an object")? {
                    let id = Json::Str(id.clone())
                        .as_i64()
                        .ok_or_else(|| format!("bad developer id '{id}'"))? as u32;
                    let dev = d.token();
                    d.developers.insert(id, dev);
                    if matches!(spec, Json::Null) {
                        continue;
                    }
                    // `cells: [x, z]`: the map covers a city of that size
                    // (smaller or larger than the land value city, §7.4).
                    let mut city = d.city;
                    if let Some(c) = spec.get("cells") {
                        let c = c
                            .as_array()
                            .filter(|c| c.len() == 2)
                            .ok_or("developer 'cells' must be [x, z]")?;
                        city.cells_x = num_of(&c[0])? as i32;
                        city.cells_z = num_of(&c[1])? as i32;
                    }
                    let g = AnyGrid::I16(SimGrid::create(&city, 0, opt_int(spec, "size")?.unwrap_or(2) as i32));
                    if let Some(all) = spec.get("all") {
                        g.set_all(num_of(all)?);
                    }
                    if let Some(vals) = spec.get("values") {
                        for (x, z, val) in triples(vals)? {
                            g.set_tract_value(x, z, val);
                        }
                    }
                    let grid = d.add_grid(g, format!("dev:0x{id:04X}"));
                    d.maps.insert(dev, grid);
                }
            }
            other => return Err(format!("unknown world key '{other}'")),
        }
    }
    Ok(())
}

fn with_sim<T>(w: &World, f: impl FnOnce(&mut LandValueSim) -> R<T>) -> R<T> {
    let mut s = w.sim.borrow_mut();
    let sim = s.as_mut().ok_or("no land value simulator (lv_create first)")?;
    f(sim)
}

fn grid_token(sim: &LandValueSim, name: &str) -> R<u64> {
    Ok(match name {
        "intrinsic" => sim.intrinsic_grid(),
        "total" => sim.total_grid(),
        "slope" => sim.slope_grid(),
        other => return Err(format!("unknown grid '{other}'")),
    })
}

fn run(w: &World, name: &str, op: &Json) -> R<()> {
    match name {
        "lv_world" => configure(&mut w.data.borrow_mut(), op),
        "lv_create" => {
            *w.sim.borrow_mut() = Some(LandValueSim::new(Box::new(FakeHost(w.data.clone()))));
            Ok(())
        }
        "lv_init" => {
            let t = op.get("tuning").map(parse_tuning).transpose()?;
            let ok = with_sim(w, |s| Ok(s.init(t.as_ref())))?;
            check_expect_bool(op, ok)
        }
        "lv_shutdown" => {
            let ok = with_sim(w, |s| Ok(s.shutdown()))?;
            check_expect_bool(op, ok)
        }
        "lv_step" => with_sim(w, |s| {
            let budget = opt_int(op, "budget")?.unwrap_or(1000) as u32;
            for _ in 0..opt_int(op, "count")?.unwrap_or(1) {
                s.step(budget);
            }
            if let Some(want) = opt_int(op, "expect_state")? {
                if i64::from(s.pipeline_state()) != want {
                    return Err(format!("state {}, expected {want}", s.pipeline_state()));
                }
            }
            Ok(())
        }),
        "lv_run" => with_sim(w, |s| {
            let budget = opt_int(op, "budget")?.unwrap_or(1000) as u32;
            let max = opt_int(op, "max")?.unwrap_or(1000);
            let mut calls = 0;
            loop {
                s.step(budget);
                calls += 1;
                if s.pipeline_state() == fc4_core::landvalue::STATE_FINISHED {
                    break;
                }
                if calls >= max {
                    return Err(format!(
                        "no finished pass after {calls} calls (state {})",
                        s.pipeline_state()
                    ));
                }
            }
            if let Some(want) = opt_int(op, "expect_calls")? {
                if calls != want {
                    return Err(format!("pass took {calls} Step calls, expected {want}"));
                }
            }
            Ok(())
        }),
        "lv_message" => {
            let t = int(op, "type")? as u32;
            let data3 = opt_int(op, "data3")?.unwrap_or(0);
            let ok = with_sim(w, |s| Ok(s.handle_message(t, data3)))?;
            check_expect_bool(op, ok)
        }
        "lv_set" => with_sim(w, |s| {
            let f = |k: &str| -> R<Option<f32>> { op.get(k).map(|v| num_of(v).map(|x| x as f32)).transpose() };
            s.debug_set(
                op.get("dirty").and_then(boolean),
                opt_int(op, "state")?.map(|v| v as u32),
                opt_int(op, "average")?.map(|v| v as u8),
                f("mean_alt")?,
            );
            if op.get("min_alt").is_some() {
                let st = s.snapshot();
                s.set_altitude_stats(
                    f("min_alt")?.unwrap_or(st.min_alt),
                    f("max_alt")?.unwrap_or(st.max_alt),
                    f("mode_alt")?.unwrap_or(st.mode_alt),
                    f("mean_alt")?.unwrap_or(st.mean_alt),
                );
            }
            Ok(())
        }),
        "lv_expect" => with_sim(w, |s| expect_state(s, op)),
        "lv_set_grid" => {
            let token = with_sim(w, |s| {
                grid_token(s, field(op, "grid")?.as_str().ok_or("'grid' must be a name")?)
            })?;
            let d = w.data.borrow();
            let g = &d.grids.get(&token).ok_or("grid not created")?.grid;
            if !matches!(g, AnyGrid::U8(_) | AnyGrid::F32(_)) {
                return Err("unexpected grid type".into());
            }
            if let Some(all) = op.get("all") {
                g.set_all(num_of(all)?);
            }
            for (x, z, v) in op.get("values").map(triples).transpose()?.unwrap_or_default() {
                g.set_tract_value(x, z, v);
            }
            Ok(())
        }
        "lv_expect_grid" => expect_grid(w, op),
        "lv_query" => with_sim(w, |s| query(w, s, op)),
        "lv_altitude" => altitude(op),
        "lv_desirability" => {
            let weight = num_of(field(op, "weight")?)? as f32;
            let curve = curve_arg(op.get("curve"))?.and_then(|c| ResponseCurve::from_flat(&c));
            for (v, d, want) in triples(field(op, "cases")?)? {
                let got = desirability_term(v as u8, d as i16, weight, curve.as_ref());
                if f64::from(got) != want {
                    return Err(format!("(v {v}, d {d}): got {got}, expected {want}"));
                }
            }
            Ok(())
        }
        "lv_save" => with_sim(w, |s| save(s, op)),
        "lv_load" => {
            let mut bytes = hex_decode(field(op, "hex")?.as_str().ok_or("'hex' must be a string")?)?;
            if let Some(n) = opt_int(op, "zero_tail")? {
                bytes.extend(std::iter::repeat(0u8).take(n as usize));
            }
            let mut resolver = Resolver(w.data.clone());
            let mut rd = ByteReader::with_resolver(&bytes, &mut resolver);
            let ok = with_sim(w, |s| Ok(s.read(&mut rd)))?;
            if let Some(p) = opt_int(op, "position")? {
                if rd.position() as i64 != p {
                    return Err(format!("read {} bytes, expected {p}", rd.position()));
                }
            }
            check_expect_bool(op, ok)
        }
        "lv_expect_events" => {
            let want: Vec<String> = field(op, "expect")?
                .as_array()
                .ok_or("'expect' must be a list of strings")?
                .iter()
                .map(|e| e.as_str().map(str::to_owned).ok_or_else(|| format!("bad event {e:?}")))
                .collect::<R<_>>()?;
            let mut d = w.data.borrow_mut();
            let got = std::mem::take(&mut d.events);
            if got != want {
                return Err(format!("events {got:?}, expected {want:?}"));
            }
            Ok(())
        }
        "lv_clear_events" => {
            w.data.borrow_mut().events.clear();
            Ok(())
        }
        other => Err(format!("unknown op '{other}'")),
    }
}

fn expect_state(s: &LandValueSim, op: &Json) -> R<()> {
    let st = s.snapshot();
    let tu = s.tuning();
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        let fl = |got: f32| -> R<()> {
            let want = num_of(v)?;
            if close(f64::from(got), want, true) {
                Ok(())
            } else {
                Err(format!("{k}: got {got}, expected {want}"))
            }
        };
        let it = |got: i64| -> R<()> {
            let want = v
                .as_i64()
                .or_else(|| boolean(v).map(i64::from))
                .ok_or_else(|| format!("'{k}' must be an integer"))?;
            if got == want {
                Ok(())
            } else {
                Err(format!("{k}: got {got}, expected {want}"))
            }
        };
        let pair = |got: [u8; 2]| -> R<()> {
            let want = nums(v)?;
            if want.len() == 2 && want[0] == f64::from(got[0]) && want[1] == f64::from(got[1]) {
                Ok(())
            } else {
                Err(format!("{k}: got {got:?}, expected {want:?}"))
            }
        };
        match k.as_str() {
            "op" | "description" => Ok(()),
            "initialized" => it(st.initialized.into()),
            "dirty" => it(st.dirty.into()),
            "state" => it(st.pipeline_state.into()),
            "sum" => it(st.sum.into()),
            "count" => it(st.count.into()),
            "ignore_zoning" => it(st.ignore_zoning.into()),
            "average" => it(st.average_land_value.into()),
            "bins" => it(st.histogram.len() as i64),
            "hist_shift" => it(st.hist_shift.into()),
            "min_alt" => fl(st.min_alt),
            "max_alt" => fl(st.max_alt),
            "mean_alt" => fl(st.mean_alt),
            "mean_alt_bits" => it(st.mean_alt.to_bits().into()),
            "mode_alt" => fl(st.mode_alt),
            "pivot" => fl(st.altitude.pivot),
            "low_range" => fl(st.altitude.low_range),
            "high_range" => fl(st.altitude.high_range),
            "top_index" => it(st.altitude.top_index.into()),
            "histogram" => {
                let want = v.as_object().ok_or("'histogram' must be an object {bin: count}")?;
                let mut expected = vec![0i64; st.histogram.len()];
                for (b, n) in want {
                    let b: usize = b.parse().map_err(|_| format!("bad bin '{b}'"))?;
                    *expected.get_mut(b).ok_or_else(|| format!("bin {b} out of range"))? =
                        n.as_i64().ok_or("bin counts must be integers")?;
                }
                let got: Vec<i64> = st.histogram.iter().map(|&n| n.into()).collect();
                if got == expected {
                    Ok(())
                } else {
                    Err(format!("histogram {got:?}, expected {expected:?}"))
                }
            }
            "developers" => it(st.developers.len() as i64),
            "developer_weights" => {
                let want = nums(v)?;
                let got: Vec<f64> = st.developers.iter().map(|d| f64::from(d.1)).collect();
                if got == want {
                    Ok(())
                } else {
                    Err(format!("developer weights {got:?}, expected {want:?}"))
                }
            }
            "tract_size" => it(tu.tract_size.into()),
            "base" => it(tu.base.into()),
            "intrinsic_max" => it(tu.intrinsic_max.into()),
            "wealth" => pair(tu.wealth),
            "water" => pair(tu.water),
            "altitude_curve" => it(tu.altitude_curve.is_some().into()),
            "desirability_curve" => it(tu.desirability_curve.is_some().into()),
            "has_grids" => it((st.intrinsic != 0 && st.total != 0 && st.slope != 0).into()),
            other => Err(format!("unknown lv_expect key '{other}'")),
        }?;
    }
    Ok(())
}

fn expect_grid(w: &World, op: &Json) -> R<()> {
    let name = field(op, "grid")?.as_str().ok_or("'grid' must be a name")?;
    let check = |get: &dyn Fn(i32, i32) -> f64, cx: i32, cz: i32, float: bool| -> R<()> {
        if let Some(all) = op.get("all") {
            let want = num_of(all)?;
            for x in 0..cx {
                for z in 0..cz {
                    let got = get(x, z);
                    if !close(got, want, float) {
                        return Err(format!(
                            "{name} tract ({x}, {z}): got {got}, expected {want} everywhere"
                        ));
                    }
                }
            }
        }
        if let Some(vals) = op.get("values") {
            for (x, z, want) in triples(vals)? {
                let got = get(x, z);
                if !close(got, want, float) {
                    return Err(format!("{name} tract ({x}, {z}): got {got}, expected {want}"));
                }
            }
        }
        if let Some(rows) = op.get("rows") {
            // rows[z][x], z outer, for readable vectors.
            for (z, row) in rows.as_array().ok_or("'rows' must be a list")?.iter().enumerate() {
                for (x, want) in nums(row)?.into_iter().enumerate() {
                    let got = get(x as i32, z as i32);
                    if !close(got, want, float) {
                        return Err(format!("{name} tract ({x}, {z}): got {got}, expected {want}"));
                    }
                }
            }
        }
        Ok(())
    };
    let sim = w.sim.borrow();
    let sim = sim.as_ref().ok_or("no land value simulator")?;
    if name == "scratch" {
        let s = sim.scratch();
        let geo = s.geometry();
        let get = |x: i32, z: i32| geo.index(x, z).map_or(f64::NAN, |_| s.get(x, z).into());
        return check(&get, geo.count_x, geo.count_z, false);
    }
    let token = grid_token(sim, name)?;
    let d = w.data.borrow();
    let g = &d
        .grids
        .get(&token)
        .ok_or_else(|| format!("grid '{name}' does not exist"))?
        .grid;
    let geom = g.geometry();
    if let Some(s) = opt_int(op, "tract_size")? {
        if i64::from(geom.tract_size) != s {
            return Err(format!("{name}: tract size {}, expected {s}", geom.tract_size));
        }
    }
    let get = |x: i32, z: i32| g.tract_value(x, z);
    check(&get, geom.count_x, geom.count_z, matches!(g, AnyGrid::F32(_)))
}

fn query(w: &World, s: &LandValueSim, op: &Json) -> R<()> {
    let method = field(op, "method")?.as_str().ok_or("'method' must be a string")?;
    let got: f64 = match method {
        // GetLandValue / GetLandValueType(x, z): the total grid's GetCellValue.
        "land_value" | "land_value_type" => {
            let c = nums(field(op, "cell")?)?;
            let d = w.data.borrow();
            let v = match d.grids.get(&s.total_grid()).map(|g| &g.grid) {
                Some(AnyGrid::U8(g)) => g.get_cell_value(c[0] as i32, c[1] as i32),
                _ => return Err("no total grid".into()),
            };
            if method == "land_value" {
                v.into()
            } else {
                s.land_value_type(v).into()
            }
        }
        "average_land_value" => s.average_land_value().into(),
        "mode_altitude" => s.mode_altitude().into(),
        other => return Err(format!("unknown query '{other}'")),
    };
    let want = num_of(field(op, "expect")?)?;
    if close(got, want, method == "mode_altitude") {
        Ok(())
    } else {
        Err(format!("{method}: got {got}, expected {want}"))
    }
}

fn altitude(op: &Json) -> R<()> {
    let p = nums(field(op, "setup")?)?;
    let max_allowed = op.get("max_allowed").map(num_of).transpose()?.unwrap_or(2500.0) as f32;
    let a = AltitudeParams::setup(p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32, max_allowed);
    if let Some(e) = op.get("expect") {
        let checks = [
            ("pivot", f64::from(a.pivot)),
            ("low_range", f64::from(a.low_range)),
            ("high_range", f64::from(a.high_range)),
            ("top_index", f64::from(a.top_index)),
        ];
        for (k, got) in checks {
            if let Some(want) = e.get(k) {
                let want = num_of(want)?;
                if !close(got, want, true) {
                    return Err(format!("{k}: got {got}, expected {want}"));
                }
            }
        }
    }
    let curve = curve_arg(op.get("curve"))?.and_then(|c| ResponseCurve::from_flat(&c));
    if let Some(effects) = op.get("effects") {
        for pair in effects
            .as_array()
            .ok_or("'effects' must be a list of [altitude, effect]")?
        {
            let v = nums(pair)?;
            let got = a.effect(curve.as_ref(), v[0] as f32);
            if f64::from(got) != v[1] {
                return Err(format!("AltitudeEffect({}): got {got}, expected {}", v[0], v[1]));
            }
        }
    }
    Ok(())
}

/// A byte writer whose references carry fixed object IDs per host token.
struct FixedIdWriter {
    inner: ByteWriter,
    ids: HashMap<u64, u32>,
}

impl OutStream for FixedIdWriter {
    fn put_u8(&mut self, v: u8) -> bool {
        self.inner.put_u8(v)
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.inner.put_u16(v)
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.inner.put_u32(v)
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.inner.put_u64(v)
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.inner.put_f32(v)
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.inner.put_f64(v)
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.inner.put_bytes(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.inner.put_str(v)
    }
    fn put_ref(&mut self, r: &RefValue) -> bool {
        match r {
            RefValue::Null => self.inner.put_u32(0),
            RefValue::Host { token, class_id } => {
                let Some(&id) = self.ids.get(token) else {
                    self.inner.set_error();
                    return false;
                };
                self.inner.put_u32(id) && self.inner.put_u32(*class_id)
            }
            RefValue::Object(_) => {
                self.inner.set_error();
                false
            }
        }
    }
    fn error(&self) -> i32 {
        self.inner.error()
    }
}

fn save(s: &LandValueSim, op: &Json) -> R<()> {
    let mut ids = HashMap::new();
    if let Some(m) = op.get("ids") {
        for (k, v) in m.as_object().ok_or("'ids' must be an object")? {
            ids.insert(grid_token(s, k)?, v.as_i64().ok_or("ids must be integers")? as u32);
        }
    }
    let mut wr = FixedIdWriter {
        inner: ByteWriter::new(),
        ids,
    };
    let ok = s.write(&mut wr);
    check_expect_bool(op, ok)?;
    let want = hex_decode(field(op, "hex")?.as_str().ok_or("'hex' must be a string")?)?;
    if wr.inner.bytes() != want.as_slice() {
        return Err(format!(
            "wrote {}, expected {}",
            hex_encode(wr.inner.bytes()),
            hex_encode(&want)
        ));
    }
    Ok(())
}

/// Creates a fake grid of the referenced class, sized as the shipped game
/// saves them (uint8 grids S = 2, float grids S = 8), standing in for the
/// grid's own class record; its label is the object ID.
struct Resolver(Rc<RefCell<Data>>);

#[cfg(test)]
#[path = "landvalue_terrain_tests.rs"]
mod terrain_tests;

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        let city = self.0.borrow().city;
        let grid = match class_id {
            CLSID_U8 => {
                let g = SimGrid::<u8>::new();
                g.set_tract_size(&city, 2);
                AnyGrid::U8(g)
            }
            CLSID_F32 => {
                let g = SimGrid::<f32>::new();
                g.set_tract_size(&city, 8);
                AnyGrid::F32(g)
            }
            _ => return None,
        };
        let mut d = self.0.borrow_mut();
        d.events.push(format!("load 0x{class_id:08X} 0x{object_id:X}"));
        let token = d.add_grid(grid, format!("obj:0x{object_id:X}"));
        Some(RefValue::Host { token, class_id })
    }
}
