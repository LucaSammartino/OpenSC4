// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/weather.md`: a scripted city and terrain
//! (heights, water, normals, sea level, hemisphere, region water
//! preferences), a scriptable random generator, and ops that drive the
//! weather simulator. See README.md ("Weather ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::{Rc, Weak};

use fc4_core::julian::julian_from_civil;
use fc4_core::property::PropertyHolder;
use fc4_core::rng::{CrtRand, FixedSeeds};
use fc4_core::stream::{RefValue, SaveObject};
use fc4_core::weather::{
    msg, shipped_tuning, CellIterator, Grid, IteratorState, RngState, WeatherData, WeatherHost, WeatherRandom,
    WeatherRng, WeatherSim, CLSID_WEATHER_SIMULATOR,
};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, Saved, R,
};

#[test]
fn we1_daily_and_accessor_bit_patterns() {
    let w = World::new();
    let s = WeatherSim::new(w.clone() as Rc<dyn WeatherHost>, 1, w.rng(None).unwrap());
    {
        let mut d = s.data_mut();
        d.wind_month = 3;
        d.prev_wind_speed = 10.0;
        d.prev_wind_dir = [1.0, 0.0];
        d.target_wind_speed = 20.0;
        d.target_wind_dir = [0.0, 1.0];
    }
    // Weather §14.13: exact bits distinguish reciprocal products from division.
    for (day, direction, speed) in [
        (2, [0x3F7F_64B5, 0x3D8C_E81D], 0x4125_5555),
        (8, [0x3F5A_ACC1, 0x3F05_1B3D], 0x4145_5555),
        (31, [0, 0x3F80_0000], 0x41A0_0000),
    ] {
        s.do_daily(julian_from_civil(2001, 3, day));
        let d = s.snapshot();
        assert_eq!(d.wind_dir.map(f32::to_bits), direction);
        assert_eq!(d.wind_speed.to_bits(), speed);
        if day == 2 {
            assert_eq!(s.ambient_wind().1.map(f32::to_bits), [0x4124_F10A, 0x3F36_0125]);
        }
    }
    // §14.14: strict threshold, rescale measured length, no mutation.
    for (direction, expected) in [
        ([0x3F80_0000, 0x3F80_0000], [0x40E2_4630, 0x40E2_4630]),
        ([0x3C23_D70A, 0], [0x3C23_D70A, 0]),
        ([0x3CA3_D70A, 0], [0x4120_0000, 0]),
        ([0, 0], [0, 0]),
    ] {
        {
            let mut d = s.data_mut();
            d.wind_dir = direction.map(f32::from_bits);
            d.wind_speed = 10.0;
        }
        assert_eq!(s.ambient_wind().1.map(f32::to_bits), expected);
        assert_eq!(s.wind_at_cell(0, 0).1.map(f32::to_bits), expected);
        assert_eq!(s.snapshot().wind_dir.map(f32::to_bits), direction);
    }
    {
        let mut d = s.data_mut();
        d.target_wind_speed = 10.0;
        d.target_wind_dir = [-1.0, 0.0];
    }
    s.do_daily(julian_from_civil(2001, 3, 16));
    assert_eq!(s.snapshot().wind_dir.map(f32::to_bits), [0, 0]);
    let before = s.snapshot();
    s.do_daily(julian_from_civil(2001, 4, 2));
    assert_eq!(s.snapshot().wind_dir, before.wind_dir);
    assert_eq!(s.snapshot().wind_speed, before.wind_speed);
}

/// A scripted generator: queued values, the last one repeating.
struct Stub {
    world: Weak<World>,
    uniform: VecDeque<f64>,
    gaussian: VecDeque<f64>,
    ints: VecDeque<i32>,
    last: (f64, f64, i32),
    state: RngState,
}

impl Stub {
    fn log(&self, kind: &str, v: Json) {
        if let Some(w) = self.world.upgrade() {
            w.events
                .push("rng", vec![("kind", Json::Str(kind.into())), ("value", v)]);
        }
    }
}

impl WeatherRandom for Stub {
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        let u = self.uniform.pop_front().unwrap_or(self.last.0);
        self.last.0 = u;
        self.log("uniform", Json::Float(u));
        // RandomDoubleRangeUniform's 24-bit sequence (foundations §6.1).
        let r24 = fc4_core::x87::r24;
        r24(r24(u * r24(b - a)) + a)
    }
    fn gaussian(&mut self) -> f64 {
        let g = self.gaussian.pop_front().unwrap_or(self.last.1);
        self.last.1 = g;
        self.log("gaussian", Json::Float(g));
        g
    }
    fn range_i32_gaussian_fast(&mut self, lo: i32, hi: i32) -> i32 {
        let i = self.ints.pop_front().unwrap_or(self.last.2);
        self.last.2 = i;
        if let Some(w) = self.world.upgrade() {
            w.events.push(
                "rng",
                vec![
                    ("kind", Json::Str("int".into())),
                    ("value", Json::Int(i.into())),
                    ("lo", Json::Int(lo.into())),
                    ("hi", Json::Int(hi.into())),
                ],
            );
        }
        i
    }
    fn state(&self) -> RngState {
        self.state
    }
    fn set_state(&mut self, s: RngState) {
        self.state = s;
    }
}

#[derive(Default)]
pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<Rc<WeatherSim>>>,
    next_token: Cell<u64>,
    saves: RefCell<BTreeMap<String, Saved>>,
    size: Cell<(f32, f32)>,
    cell_width: Cell<(f32, f32)>,
    hemisphere: Cell<i32>,
    month: Cell<i32>,
    water_prefs: Cell<Option<(u8, u8)>>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    terrain: Cell<bool>,
    flat: Cell<f32>,
    heights: RefCell<BTreeMap<(i32, i32), f32>>,
    water: RefCell<BTreeSet<(i32, i32)>>,
    normals: RefCell<BTreeMap<(i32, i32), [f32; 3]>>,
    sea_level: Cell<f32>,
    seed: Cell<u32>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            next_token: Cell::new(0x100),
            size: Cell::new((256.0, 256.0)),
            cell_width: Cell::new((16.0, 16.0)),
            hemisphere: Cell::new(1),
            month: Cell::new(1),
            terrain: Cell::new(true),
            sea_level: Cell::new(-1.0),
            seed: Cell::new(1),
            ..Default::default()
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn sim(&self) -> R<Rc<WeatherSim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no weather simulator (weather_create)".into())
    }

    fn token(&self) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        t
    }

    fn rng(&self, spec: Option<&Json>) -> R<Box<dyn WeatherRandom>> {
        let seeds = FixedSeeds {
            wall_clock: self.seed.get(),
            process_clock: 1,
        };
        let Some(spec) = spec else {
            return Ok(Box::new(WeatherRng::new(&seeds)));
        };
        if let Some(s) = opt(spec, "seed") {
            return Ok(Box::new(WeatherRng::with_seed(u32_of(s, "seed")?, &seeds)));
        }
        let stub = field(spec, "stub")?;
        let floats = |k: &str| -> R<VecDeque<f64>> {
            opt(stub, k).map_or(Ok(VecDeque::new()), |v| {
                arr(v, k)?
                    .iter()
                    .map(|x| scalar::num(x).ok_or_else(|| format!("{k}: number")))
                    .collect()
            })
        };
        let ints = opt(stub, "int").map_or(Ok(VecDeque::new()), |v| {
            arr(v, "int")?
                .iter()
                .map(|x| i64_of(x, "int").map(|i| i as i32))
                .collect::<R<VecDeque<i32>>>()
        })?;
        Ok(Box::new(Stub {
            world: self.me.borrow().clone(),
            uniform: floats("uniform")?,
            gaussian: floats("gaussian")?,
            ints,
            last: (0.5, 0.0, 0),
            state: RngState::default(),
        }))
    }

    fn height(&self, x: i32, z: i32) -> f32 {
        self.heights.borrow().get(&(x, z)).copied().unwrap_or(self.flat.get())
    }
}

impl WeatherHost for World {
    fn city_size(&self) -> (f32, f32) {
        self.size.get()
    }
    fn city_cell_width(&self) -> (f32, f32) {
        self.cell_width.get()
    }
    fn world_hemisphere(&self) -> i32 {
        self.hemisphere.get()
    }
    fn sim_month(&self) -> i32 {
        self.month.get()
    }
    fn water_prefs(&self) -> Option<(u8, u8)> {
        self.water_prefs.get()
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn terrain_available(&self) -> bool {
        self.terrain.get()
    }
    fn altitude_at_vertex(&self, x: i32, z: i32) -> f32 {
        if self.terrain.get() {
            self.height(x, z)
        } else {
            0.0
        }
    }
    fn vertex_normal(&self, x: i32, z: i32) -> [f32; 3] {
        self.normals.borrow().get(&(x, z)).copied().unwrap_or([0.0, 1.0, 0.0])
    }
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        self.terrain.get() && self.water.borrow().contains(&(x, z))
    }
    fn altitude(&self, world_x: f32, world_z: f32) -> f32 {
        self.altitude_at_vertex((world_x / 16.0).floor() as i32, (world_z / 16.0).floor() as i32)
    }
    fn sea_level(&self) -> f32 {
        self.sea_level.get()
    }
    fn post(&self, msg_type: u32, data2: u32) {
        let name = match msg_type {
            msg::SEASON_CHANGED => "SeasonChanged",
            msg::WEATHER_CYCLE_COMPLETED => "WeatherCycleCompleted",
            msg::MOISTURE_MAP_UPDATED => "MoistureMapUpdated",
            _ => "other",
        };
        self.events
            .push("post", vec![("type", Json::Str(name.into())), ("data2", ji(data2))]);
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "weather", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("weather_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "tuning" => tuning(&w, op),
        "create" => (|| {
            if let Some(s) = opt(op, "seed") {
                w.seed.set(u32_of(s, "seed")?);
            }
            let rng = w.rng(opt(op, "rng"))?;
            *w.sim.borrow_mut() = Some(WeatherSim::new(w.rc() as Rc<dyn WeatherHost>, w.token(), rng));
            Ok(())
        })(),
        "rng" => w.sim().and_then(|s| {
            s.set_random(w.rng(Some(op))?);
            Ok(())
        }),
        "init" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => message(&w, op),
        "call" => call(&w, op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "expect_grid" => expect_grid(&w, op),
        "iterator" => iterator(op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect_save" => expect_save(&w, op),
        "patch" => patch(&w, op),
        "expect_payload" => expect_payload(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- environment ----------------------------------------------------------------------------------

/// `{x, z, w?, d?}` → the cells of the rectangle.
fn cells_of(e: &Json) -> R<Vec<(i32, i32)>> {
    let x = i64_of(field(e, "x")?, "x")? as i32;
    let z = i64_of(field(e, "z")?, "z")? as i32;
    let w = opt(e, "w").map_or(Ok(1), |v| i64_of(v, "w"))? as i32;
    let d = opt(e, "d").map_or(Ok(1), |v| i64_of(v, "d"))? as i32;
    Ok((x..x + w).flat_map(|i| (z..z + d).map(move |j| (i, j))).collect())
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "size") {
        let a = arr(v, "size")?;
        let (sx, sz) = (f32_of(&a[0], "size")?, f32_of(&a[1], "size")?);
        w.size.set((sx * 16.0, sz * 16.0));
    }
    if let Some(v) = opt(op, "cell_width") {
        let a = arr(v, "cell_width")?;
        w.cell_width
            .set((f32_of(&a[0], "cell_width")?, f32_of(&a[1], "cell_width")?));
    }
    if let Some(v) = opt(op, "hemisphere") {
        w.hemisphere.set(i64_of(v, "hemisphere")? as i32);
    }
    if let Some(v) = opt(op, "month") {
        w.month.set(i64_of(v, "month")? as i32);
    }
    if let Some(v) = op.get("water_prefs") {
        w.water_prefs.set(match v {
            Json::Null => None,
            v => {
                let a = arr(v, "water_prefs")?;
                Some((u32_of(&a[0], "lo")? as u8, u32_of(&a[1], "hi")? as u8))
            }
        });
    }
    if let Some(v) = opt(op, "terrain") {
        w.terrain.set(bool_of(v, "terrain")?);
    }
    if let Some(v) = opt(op, "flat") {
        w.flat.set(f32_of(v, "flat")?);
        w.heights.borrow_mut().clear();
    }
    if let Some(v) = opt(op, "heights") {
        for e in arr(v, "heights")? {
            let h = f32_of(field(e, "h")?, "h")?;
            for c in cells_of(e)? {
                w.heights.borrow_mut().insert(c, h);
            }
        }
    }
    if let Some(v) = op.get("water") {
        let mut set = BTreeSet::new();
        if !matches!(v, Json::Null) {
            for e in arr(v, "water")? {
                set.extend(cells_of(e)?);
            }
        }
        *w.water.borrow_mut() = set;
    }
    if let Some(v) = opt(op, "normals") {
        for e in arr(v, "normals")? {
            let n = arr(field(e, "n")?, "n")?;
            let n = [f32_of(&n[0], "n")?, f32_of(&n[1], "n")?, f32_of(&n[2], "n")?];
            for c in cells_of(e)? {
                w.normals.borrow_mut().insert(c, n);
            }
        }
    }
    if let Some(v) = opt(op, "sea_level") {
        w.sea_level.set(f32_of(v, "sea_level")?);
    }
    Ok(())
}

/// `weather_tuning`: `shipped` (the §9 shipped values), `props` (merged
/// over them when `shipped`), or `none`.
fn tuning(w: &World, op: &Json) -> R<()> {
    if opt(op, "none").and_then(crate::runner::boolean).unwrap_or(false) {
        *w.tuning.borrow_mut() = None;
        return Ok(());
    }
    let mut h = if opt(op, "shipped").and_then(crate::runner::boolean).unwrap_or(false) {
        shipped_tuning()
    } else {
        PropertyHolder::new()
    };
    if let Some(p) = opt(op, "props") {
        let extra = scalar::holder_of(p)?;
        for id in extra.ids() {
            if let Some(v) = extra.get(id) {
                h.add(id, v.clone());
            }
        }
    }
    *w.tuning.borrow_mut() = Some(Rc::new(h));
    Ok(())
}

// ---- calls ------------------------------------------------------------------------------------------

fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let t = match field(op, "type")?.as_str() {
        Some("SimNewDay") => msg::SIM_NEW_DAY,
        Some("SimNewWeek") => msg::SIM_NEW_WEEK,
        Some("SimNewMonth") => msg::SIM_NEW_MONTH,
        Some("PostCityInit") => msg::POST_CITY_INIT,
        _ => u32_of(field(op, "type")?, "type")?,
    };
    let mut data1 = opt(op, "data1").map_or(Ok(0), |v| u32_of(v, "data1"))?;
    if let Some(d) = opt(op, "date") {
        let g = |k: &str| -> R<i32> { Ok(i64_of(field(d, k)?, k)? as i32) };
        data1 = julian_from_civil(g("year")?, g("month")?, g("day")?) as u32;
    }
    let data3 = opt(op, "data3").map_or(Ok(0), |v| u32_of(v, "data3"))?;
    check_expect(op, Json::Bool(s.handle_message(t, data1, data3)))
}

fn a_f32(op: &Json, i: usize) -> R<f32> {
    f32_of(arg(op, i)?, "argument")
}

fn a_i32(op: &Json, i: usize) -> R<i32> {
    Ok(i64_of(arg(op, i)?, "argument")? as i32)
}

fn pair_i(a: (i32, i32)) -> Json {
    Json::Array(vec![ji(a.0), ji(a.1)])
}

fn vec2(v: [f32; 2]) -> Json {
    Json::Array(vec![jf(v[0]), jf(v[1])])
}

fn grid_json(g: &Grid) -> Json {
    Json::Array(
        g.iter()
            .map(|r| Json::Array(r.iter().map(|&b| ji(b)).collect()))
            .collect(),
    )
}

fn call(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let m = str_of(field(op, "method")?, "method")?;
    let got = match m {
        "do_post_city_init" => Json::Bool(s.do_post_city_init()),
        "do_complete_simulation" => {
            s.do_complete_simulation();
            Json::Null
        }
        "do_monthly_simulation" => {
            s.do_monthly_simulation(a_i32(op, 0)?);
            Json::Null
        }
        "do_weekly_simulation" => Json::Bool(s.do_weekly_simulation()),
        "do_daily" => {
            s.do_daily(a_i32(op, 0)?);
            Json::Null
        }
        "new_current_ambient_values" => {
            s.new_current_ambient_values(a_i32(op, 0)?);
            Json::Null
        }
        "onshore_direction" => pair_i(s.onshore_direction()),
        "simulate_cell_temperature" => {
            s.simulate_cell_temperature(a_i32(op, 0)?, a_i32(op, 1)?);
            Json::Null
        }
        "simulate_moisture" => {
            s.simulate_moisture(a_i32(op, 0)?, a_i32(op, 1)?, a_i32(op, 2)?, a_i32(op, 3)?);
            Json::Null
        }
        "update_moisture" => {
            s.update_moisture(a_i32(op, 0)?, a_i32(op, 1)?, a_i32(op, 2)?, a_i32(op, 3)?);
            Json::Null
        }
        "generate_moisture_variation" => {
            s.generate_moisture_variation();
            Json::Null
        }
        "rebuild_water" => {
            s.rebuild_water();
            Json::Null
        }
        "set_manual_moisture_addition" => {
            s.set_manual_moisture_addition(
                a_f32(op, 0)?,
                a_f32(op, 1)?,
                a_f32(op, 2)?,
                a_f32(op, 3)?,
                a_f32(op, 4)?,
            );
            Json::Null
        }
        "cell_width_x" => jf(s.cell_width_x()),
        "cell_width_z" => jf(s.cell_width_z()),
        "cell_count_x" => ji(s.cell_count_x()),
        "cell_count_z" => ji(s.cell_count_z()),
        "position_to_cell" => {
            let (r, x, z) = s.position_to_cell(a_f32(op, 0)?, a_f32(op, 1)?);
            Json::Array(vec![ji(r), ji(x), ji(z)])
        }
        "cell_to_position" => {
            let (x, z) = s.cell_to_position(a_i32(op, 0)?, a_i32(op, 1)?);
            Json::Array(vec![jf(x), jf(z)])
        }
        "cell_to_standard_city_cell" => pair_i(s.cell_to_standard_city_cell(a_i32(op, 0)?, a_i32(op, 1)?)),
        "standard_city_cell_to_cell" => pair_i(s.standard_city_cell_to_cell(a_i32(op, 0)?, a_i32(op, 1)?)),
        "current_season" => ji(s.current_season()),
        "current_season_month" => ji(s.current_season_month()),
        "moisture_value" => ji(s.moisture_value(a_f32(op, 0)?, a_f32(op, 1)?)),
        "moisture_type" => ji(s.moisture_type(a_f32(op, 0)?, a_f32(op, 1)?)),
        "moisture_map" => grid_json(&s.moisture_grid()),
        "humidity" => jf(s.humidity(a_f32(op, 0)?, a_f32(op, 1)?)),
        "ambient_wind" => {
            let (sp, v) = s.ambient_wind();
            Json::Object(vec![("speed".into(), jf(sp)), ("v".into(), vec2(v))])
        }
        "wind_at_cell" => {
            let (sp, v) = s.wind_at_cell(a_i32(op, 0)?, a_i32(op, 1)?);
            Json::Object(vec![("speed".into(), jf(sp)), ("v".into(), vec2(v))])
        }
        other => return Err(format!("unknown weather method '{other}'")),
    };
    check_expect(op, got)
}

fn f2(v: &Json, k: &str) -> R<[f32; 2]> {
    let a = arr(v, k)?;
    Ok([f32_of(&a[0], k)?, f32_of(&a[1], k)?])
}

/// `weather_set`: pokes state fields (test setup).
fn set(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let mut d = s.data_mut();
    for (k, v) in field(op, "state")?.as_object().ok_or("state must be an object")? {
        let fl = || f32_of(v, k);
        let int = || i64_of(v, k).map(|i| i as i32);
        let byte = || u32_of(v, k).map(|i| i as u8);
        let (nx, nz) = (d.count_x.max(0), d.count_z.max(0));
        let fill = |b: u8| vec![vec![b; nz as usize]; nx as usize];
        match k.as_str() {
            "ambient_temp" => d.ambient_temp = fl()?,
            "ambient_moisture" => d.ambient_moisture = fl()?,
            "base_temp" => d.base_temp = fl()?,
            "base_moisture" => d.base_moisture = fl()?,
            "wind_speed" => d.wind_speed = fl()?,
            "wind_dir" => d.wind_dir = f2(v, k)?,
            "prev_wind_speed" => d.prev_wind_speed = fl()?,
            "prev_wind_dir" => d.prev_wind_dir = f2(v, k)?,
            "target_wind_speed" => d.target_wind_speed = fl()?,
            "target_wind_dir" => d.target_wind_dir = f2(v, k)?,
            "wind_month" => d.wind_month = int()?,
            "wind_spread" => d.wind_spread = fl()?,
            "last_season" => d.last_season = int()?,
            "blend_radius" => d.blend_radius = int()?,
            "sea_weight" => d.sea_weight = fl()?,
            "height_factor" => d.height_factor = fl()?,
            "wind_factor" => d.wind_factor = fl()?,
            "high_moisture" => d.high_moisture = fl()?,
            "curvature_threshold" => d.curvature_threshold = fl()?,
            "slope_k" => d.slope_k = fl()?,
            "cells_per_week" => d.cells_per_week = int()?,
            "temperature_fill" => d.temperature = fill(byte()?),
            "moisture_fill" => d.moisture = fill(byte()?),
            "moisture_base_fill" => d.moisture_base = fill(byte()?),
            "sea_mask_fill" => d.sea_mask = fill(byte()?),
            "sea_mask_cells" | "moisture_base_cells" | "moisture_cells" | "temperature_cells" => {
                for e in arr(v, k)? {
                    let b = opt(e, "v").map_or(Ok(1), |x| u32_of(x, "v"))? as u8;
                    for (x, z) in cells_of(e)? {
                        let g = match k.as_str() {
                            "sea_mask_cells" => &mut d.sea_mask,
                            "moisture_base_cells" => &mut d.moisture_base,
                            "moisture_cells" => &mut d.moisture,
                            _ => &mut d.temperature,
                        };
                        if let Some(c) = g.get_mut(x as usize).and_then(|r| r.get_mut(z as usize)) {
                            *c = b;
                        }
                    }
                }
            }
            other => return Err(format!("weather_set: unknown field '{other}'")),
        }
    }
    Ok(())
}

fn iterator_json(it: &CellIterator) -> Json {
    let state = match it.state {
        IteratorState::Running => "running",
        IteratorState::Exhausted => "exhausted",
    };
    Json::Object(vec![
        ("state".into(), Json::Str(state.into())),
        ("mode".into(), ji(it.mode)),
        (
            "rect".into(),
            Json::Array(vec![ji(it.x0), ji(it.z0), ji(it.x1), ji(it.z1)]),
        ),
        ("count".into(), ji(it.count)),
        ("index".into(), ji(it.index)),
        ("current".into(), ji(it.current)),
        ("step".into(), ji(it.step)),
    ])
}

fn state_json(d: &WeatherData, rng: RngState) -> Vec<(String, Json)> {
    let f = |k: &str, v: f32| (k.to_owned(), jf(v));
    let i = |k: &str, v: i64| (k.to_owned(), ji(v));
    let b = |k: &str, v: bool| (k.to_owned(), Json::Bool(v));
    vec![
        i("count_x", d.count_x.into()),
        i("count_z", d.count_z.into()),
        f("cell_width_x", d.cell_width_x),
        f("cell_width_z", d.cell_width_z),
        f("inv_cell_width_x", d.inv_cell_width_x),
        f("size_x", d.size_x),
        f("size_z", d.size_z),
        i("city_cells_per_cell_x", d.city_cells_per_cell_x.into()),
        i("city_cells_per_cell_z", d.city_cells_per_cell_z.into()),
        i("subdiv_x", d.subdiv_x.into()),
        i("subdiv_z", d.subdiv_z.into()),
        i("blend_radius", d.blend_radius.into()),
        i("last_season", d.last_season.into()),
        f("base_temp", d.base_temp),
        f("temp_amp_a", d.temp_amp_a),
        f("temp_amp_b", d.temp_amp_b),
        f("ambient_temp", d.ambient_temp),
        (
            "water_rects".into(),
            Json::Array(
                d.water_rects
                    .iter()
                    .map(|r| Json::Array(r.iter().map(|&v| ji(v)).collect()))
                    .collect(),
            ),
        ),
        ("iterator".into(), iterator_json(&d.iterator)),
        i("cells_per_week", d.cells_per_week.into()),
        i("sea_radius", d.sea_radius.into()),
        f("sea_weight", d.sea_weight),
        f("height_factor", d.height_factor),
        f("wind_factor", d.wind_factor),
        f("ambient_moisture", d.ambient_moisture),
        f("base_moisture", d.base_moisture),
        f("moist_amp_a", d.moist_amp_a),
        f("moist_amp_b", d.moist_amp_b),
        f("slope_k", d.slope_k),
        f("high_moisture", d.high_moisture),
        f("curvature_threshold", d.curvature_threshold),
        i("tuning80", d.tuning80.into()),
        i("tuning81", d.tuning81.into()),
        f("default_wind_speed", d.default_wind_speed),
        ("default_wind_dir".into(), vec2(d.default_wind_dir)),
        f("wind_speed", d.wind_speed),
        ("wind_dir".into(), vec2(d.wind_dir)),
        f("prev_wind_speed", d.prev_wind_speed),
        ("prev_wind_dir".into(), vec2(d.prev_wind_dir)),
        f("target_wind_speed", d.target_wind_speed),
        ("target_wind_dir".into(), vec2(d.target_wind_dir)),
        i("wind_month", d.wind_month.into()),
        f("wind_spread", d.wind_spread),
        b("initialized", d.initialized),
        b("loaded_from_save", d.loaded_from_save),
        b("post_init_done", d.post_init_done),
        (
            "rng".into(),
            Json::Object(vec![
                ("lcg".into(), ji(rng.lcg)),
                ("mul".into(), ji(rng.mul)),
                ("has_cached".into(), ji(rng.has_cached_gaussian)),
                ("cached".into(), Json::Float(rng.cached_gaussian)),
            ]),
        ),
    ]
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let got = state_json(&s.snapshot(), s.rng_state());
    let want = field(op, "state")?;
    // The nested `iterator` and `rng` objects are matched partially (only
    // the fields given are checked); everything else exactly.
    let (nested, flat): (Vec<_>, Vec<_>) = want
        .as_object()
        .ok_or("'state' must be an object")?
        .iter()
        .cloned()
        .partition(|(k, _)| k == "iterator" || k == "rng");
    for (k, v) in nested {
        let g = got
            .iter()
            .find(|(gk, _)| *gk == k)
            .map(|e| &e.1)
            .ok_or("no such field")?;
        if !scalar::matches(g, &v, true) {
            return Err(format!("{k}: got {}, expected {}", scalar::show(g), scalar::show(&v)));
        }
    }
    // `exact: true`: numbers must be bit-equal (the float32 value widened to
    // a double), for precision tests (§6.1, §6.2).
    if op.get("exact").and_then(crate::runner::boolean) == Some(true) {
        for (k, v) in &flat {
            let g = got
                .iter()
                .find(|(gk, _)| gk == k)
                .map(|e| &e.1)
                .ok_or_else(|| format!("no such field '{k}'"))?;
            if !exactly(g, v) {
                return Err(format!(
                    "{k}: got {}, expected exactly {}",
                    scalar::show(g),
                    scalar::show(v)
                ));
            }
        }
    }
    check_state(&got, &Json::Object(flat))
}

/// Numbers bit-equal as doubles, arrays element by element.
fn exactly(got: &Json, want: &Json) -> bool {
    match (got, want) {
        (Json::Array(a), Json::Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(x, y)| exactly(x, y)),
        (g, w) => match (scalar::num(g), scalar::num(w)) {
            (Some(x), Some(y)) => x == y,
            _ => scalar::matches(g, w, false),
        },
    }
}

/// `weather_expect_grid`: `grid` (`temperature`, `moisture`,
/// `moisture_base`, `sea_mask`) and any of `rows` (x-major), `all`,
/// `cells` (`[{x, z, w?, d?, v}]`), `rect` + `inside` / `outside` (a value
/// every cell inside / outside `[x0, z0, x1, z1)` has), `min`, `max`, `size`.
fn expect_grid(w: &World, op: &Json) -> R<()> {
    let d = w.sim()?.snapshot();
    let name = str_of(field(op, "grid")?, "grid")?;
    let g = match name {
        "temperature" => &d.temperature,
        "moisture" => &d.moisture,
        "moisture_base" => &d.moisture_base,
        "sea_mask" => &d.sea_mask,
        other => return Err(format!("unknown grid '{other}'")),
    };
    let at = |x: i32, z: i32| -> R<u8> {
        g.get(x as usize)
            .and_then(|r| r.get(z as usize))
            .copied()
            .ok_or_else(|| format!("{name}[{x}][{z}] is outside the grid"))
    };
    let all_cells = || {
        g.iter()
            .enumerate()
            .flat_map(|(x, r)| r.iter().enumerate().map(move |(z, &v)| (x as i32, z as i32, v)))
    };
    if let Some(v) = opt(op, "size") {
        let want = arr(v, "size")?;
        let got = (g.len() as i64, g.first().map_or(0, Vec::len) as i64);
        if got != (i64_of(&want[0], "size")?, i64_of(&want[1], "size")?) {
            return Err(format!("{name}: size {got:?}"));
        }
    }
    if let Some(v) = opt(op, "rows") {
        let got = grid_json(g);
        if !scalar::matches(&got, v, false) {
            return Err(format!("{name} differs:\n  got      {}", scalar::show(&got)));
        }
    }
    if let Some(v) = opt(op, "all") {
        let want = u32_of(v, "all")? as u8;
        if let Some((x, z, b)) = all_cells().find(|c| c.2 != want) {
            return Err(format!("{name}[{x}][{z}] = {b}, expected {want} everywhere"));
        }
    }
    if let Some(v) = opt(op, "cells") {
        for e in arr(v, "cells")? {
            let want = u32_of(field(e, "v")?, "v")? as u8;
            for (x, z) in cells_of(e)? {
                let b = at(x, z)?;
                if b != want {
                    return Err(format!("{name}[{x}][{z}] = {b}, expected {want}"));
                }
            }
        }
    }
    if let Some(r) = opt(op, "rect") {
        let r: Vec<i32> = arr(r, "rect")?
            .iter()
            .map(|v| i64_of(v, "rect").map(|i| i as i32))
            .collect::<R<_>>()?;
        let inside = |x: i32, z: i32| x >= r[0] && z >= r[1] && x < r[2] && z < r[3];
        for (key, want_in) in [("inside", true), ("outside", false)] {
            if let Some(v) = opt(op, key) {
                let want = u32_of(v, key)? as u8;
                if let Some((x, z, b)) = all_cells().find(|c| inside(c.0, c.1) == want_in && c.2 != want) {
                    return Err(format!("{name}[{x}][{z}] = {b}, expected {want} {key} the rectangle"));
                }
            }
        }
    }
    if let Some(v) = opt(op, "range") {
        let r = arr(v, "range")?;
        let (lo, hi) = (u32_of(&r[0], "range")? as u8, u32_of(&r[1], "range")? as u8);
        if let Some((x, z, b)) = all_cells().find(|c| c.2 < lo || c.2 > hi) {
            return Err(format!("{name}[{x}][{z}] = {b}, outside [{lo}, {hi}]"));
        }
    }
    for (key, is_min) in [("min", true), ("max", false)] {
        if let Some(v) = opt(op, key) {
            let want = u32_of(v, key)? as u8;
            let got = if is_min {
                all_cells().map(|c| c.2).min()
            } else {
                all_cells().map(|c| c.2).max()
            };
            if got != Some(want) {
                return Err(format!("{name}: {key} is {got:?}, expected {want}"));
            }
        }
    }
    Ok(())
}

/// `weather_iterator`: a free-standing prime-step iterator over `rect`
/// (`[x0, z0, x1, z1]`, inclusive) with its own `rand()` (`crt_seed`,
/// default 1). Checks `step`, `count`, `first` (the first cells),
/// `full_pass` (every cell exactly once, then `next` is false) and
/// `rand_calls` (the `rand()` calls the reset made).
fn iterator(op: &Json) -> R<()> {
    let r: Vec<i32> = arr(field(op, "rect")?, "rect")?
        .iter()
        .map(|v| i64_of(v, "rect").map(|i| i as i32))
        .collect::<R<_>>()?;
    let seed = opt(op, "crt_seed").map_or(Ok(1), |v| u32_of(v, "crt_seed"))?;
    let shuffle = opt(op, "shuffle").map_or(Ok(true), |v| bool_of(v, "shuffle"))?;
    let mut crt = CrtRand::new(seed);
    let mut it = CellIterator::new(r[0], r[1], r[2], r[3]);
    it.reset(shuffle, &mut crt);
    if let Some(v) = opt(op, "rand_calls") {
        let mut probe = CrtRand::new(seed);
        let mut n = 0;
        while probe != crt && n < 1000 {
            probe.rand();
            n += 1;
        }
        if n != i64_of(v, "rand_calls")? {
            return Err(format!("reset made {n} rand() calls"));
        }
    }
    for (k, got) in [("step", it.step), ("count", it.count), ("current", it.current)] {
        if let Some(v) = opt(op, k) {
            if i64::from(got) != i64_of(v, k)? {
                return Err(format!("iterator {k} = {got}, expected {}", i64_of(v, k)?));
            }
        }
    }
    if let Some(v) = opt(op, "first") {
        for (i, e) in arr(v, "first")?.iter().enumerate() {
            let want = arr(e, "cell")?;
            let want = (i64_of(&want[0], "x")? as i32, i64_of(&want[1], "z")? as i32);
            let got = it.next_cell();
            if got != Some(want) {
                return Err(format!("cell {i}: got {got:?}, expected {want:?}"));
            }
        }
    }
    if opt(op, "full_pass").and_then(crate::runner::boolean).unwrap_or(false) {
        let (w, h) = ((r[2] - r[0] + 1) as usize, (r[3] - r[1] + 1) as usize);
        let mut seen = vec![0u32; w * h];
        let mut n = 0usize;
        while let Some((x, z)) = it.next_cell() {
            let i = (z - r[1]) as usize * w + (x - r[0]) as usize;
            seen[i] += 1;
            n += 1;
            if n > w * h + 1 {
                return Err("the iterator does not stop".into());
            }
        }
        if let Some(i) = seen.iter().position(|&c| c != 1) {
            return Err(format!("cell #{i} visited {} times", seen[i]));
        }
        if it.next_cell().is_some() {
            return Err("next after the last cell returned a cell".into());
        }
    }
    Ok(())
}

// ---- persistence ------------------------------------------------------------------------------------

fn slot(op: &Json) -> R<String> {
    Ok(str_of(field(op, "slot")?, "slot")?.to_owned())
}

fn save_now(w: &World) -> R<Saved> {
    let s = w.sim()?;
    scalar::save(&[RefValue::Object(s as Rc<dyn SaveObject>)])
}

fn save(w: &World, op: &Json) -> R<()> {
    let s = save_now(w)?;
    w.saves.borrow_mut().insert(slot(op)?, s);
    Ok(())
}

fn load(w: &World, op: &Json) -> R<()> {
    let name = slot(op)?;
    let saved = w.saves.borrow().get(&name).cloned().ok_or("no such slot")?;
    let me = w.rc();
    let rng = w.rng(opt(op, "rng"))?;
    let mut rng = Some(rng);
    let mut factory = |class: u32, _id: u32| -> Option<RefValue> {
        if class != CLSID_WEATHER_SIMULATOR {
            return None;
        }
        let r = rng.take()?;
        Some(RefValue::Object(
            WeatherSim::new(me.clone() as Rc<dyn WeatherHost>, me.token(), r) as Rc<dyn SaveObject>,
        ))
    };
    let (ok, roots) = scalar::load(&saved, &mut factory);
    let want = opt(op, "expect").map_or(Ok(true), |v| bool_of(v, "expect"))?;
    if ok != want {
        return Err(format!("store load clean = {ok}, expected {want}"));
    }
    let sim = roots
        .first()
        .and_then(|r| r.object::<WeatherSim>())
        .ok_or("the root did not resolve to a weather simulator")?;
    *w.sim.borrow_mut() = Some(sim);
    Ok(())
}

fn expect_save(w: &World, op: &Json) -> R<()> {
    let want = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    scalar::same_save(&save_now(w)?, &want)
}

fn patch(w: &World, op: &Json) -> R<()> {
    let mut saves = w.saves.borrow_mut();
    let s = saves.get_mut(&slot(op)?).ok_or("no such slot")?;
    scalar::patch(s, CLSID_WEATHER_SIMULATOR, op)
}

fn expect_payload(w: &World, op: &Json) -> R<()> {
    let saves = w.saves.borrow();
    let s = saves.get(&slot(op)?).ok_or("no such slot")?;
    let got = scalar::entry_payload(s, CLSID_WEATHER_SIMULATOR, 0)?;
    if let Some(h) = opt(op, "hex") {
        let want = hex_decode(str_of(h, "hex")?)?;
        if got != want {
            return Err(format!(
                "payload differs:\n    got      {}\n    expected {}",
                hex_encode(&got),
                hex_encode(&want)
            ));
        }
    }
    // `at`: [{offset, hex}] checks slices of the payload.
    if let Some(v) = opt(op, "at") {
        for e in arr(v, "at")? {
            let off = i64_of(field(e, "offset")?, "offset")? as usize;
            let want = hex_decode(str_of(field(e, "hex")?, "hex")?)?;
            let slice = got.get(off..off + want.len()).ok_or("slice outside the payload")?;
            if slice != want.as_slice() {
                return Err(format!(
                    "payload at {off}: got {}, expected {}",
                    hex_encode(slice),
                    hex_encode(&want)
                ));
            }
        }
    }
    if let Some(v) = opt(op, "length") {
        if got.len() as i64 != i64_of(v, "length")? {
            return Err(format!("payload length {}", got.len()));
        }
    }
    Ok(())
}
