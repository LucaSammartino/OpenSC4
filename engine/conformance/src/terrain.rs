// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/terrain.md` (`ter_…`) and the nation's
//! economy modifier (`nation.md`, `nat_economy`): a scripted world around
//! `fc4_core::terrain` (the terrain, its bodies of water and one lava) with
//! a logged host (queued posts, fire calls, random draws). See README.md
//! ("Terrain ops").

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use fc4_core::curve::ResponseCurve;
use fc4_core::demand::NationEconomy;
use fc4_core::terrain::{
    shipped_brush, BodiesOfWater, Bridge, BrushBitmap, BrushComponent, BrushDescription, BrushParams, FireMode, Lava,
    LavaHost, LavaTuning, NavigationHost, Terrain, TerrainTuning, VertRect,
};

use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, i64_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

#[derive(Default)]
struct Nav {
    /// (cell, lot)
    lots: Vec<((i32, i32), u64)>,
    bridges: Vec<((i32, i32), Option<f32>)>,
    poles: Vec<(i32, i32)>,
}

#[derive(Default)]
struct Fire {
    /// Cells that are not flammable (every other cell is).
    not_flammable: Vec<(i32, i32)>,
    /// The burn iterator's draws (`null` = exhausted).
    burn_cells: VecDeque<Option<(i32, i32)>>,
}

pub(crate) struct World {
    events: EventLog,
    terrain: RefCell<Option<Terrain>>,
    bodies: RefCell<BodiesOfWater>,
    lava: RefCell<Option<Lava>>,
    nav: RefCell<Nav>,
    fire: RefCell<Fire>,
    random: RefCell<VecDeque<u32>>,
}

impl World {
    fn new() -> Rc<World> {
        Rc::new(World {
            events: EventLog::default(),
            terrain: RefCell::new(None),
            bodies: RefCell::new(BodiesOfWater::default()),
            lava: RefCell::new(None),
            nav: RefCell::new(Nav::default()),
            fire: RefCell::new(Fire::default()),
            random: RefCell::new(VecDeque::new()),
        })
    }

    /// Runs `f` on the terrain, then logs the messages it queued.
    fn with<T>(&self, f: impl FnOnce(&mut Terrain) -> R<T>) -> R<T> {
        let mut b = self.terrain.borrow_mut();
        let t = b.as_mut().ok_or("no terrain (ter_create)")?;
        let r = f(t);
        for p in t.take_posts() {
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
        r
    }
}

impl NavigationHost for Nav {
    fn lot_at_cell(&self, x: i32, z: i32) -> u64 {
        self.lots.iter().find(|(c, _)| *c == (x, z)).map_or(0, |(_, l)| *l)
    }
    fn bridge_at(&self, x: i32, z: i32) -> Bridge {
        match self.bridges.iter().find(|(c, _)| *c == (x, z)) {
            None => Bridge::None,
            Some((_, None)) => Bridge::UnknownDeck,
            Some((_, Some(d))) => Bridge::Deck(*d),
        }
    }
    fn has_power_pole(&self, x: i32, z: i32) -> bool {
        self.poles.contains(&(x, z))
    }
}

struct FireHost<'a> {
    fire: &'a mut Fire,
    events: &'a EventLog,
}

impl LavaHost for FireHost<'_> {
    fn is_cell_flammable(&mut self, x: i32, z: i32) -> bool {
        !self.fire.not_flammable.contains(&(x, z))
    }
    fn ignite_cell(&mut self, x: i32, z: i32) {
        self.events.push("ignite", vec![("x", ji(x)), ("z", ji(z))]);
    }
    fn quick_burn_cell(&mut self, x: i32, z: i32) {
        self.events.push("quick_burn", vec![("x", ji(x)), ("z", ji(z))]);
    }
    fn set_cause(&mut self, cause: u32) {
        self.events.push("cause", vec![("cause", ji(cause))]);
    }
    fn next_burn_cell(&mut self) -> Option<(i32, i32)> {
        self.fire.burn_cells.pop_front().flatten()
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "terrain", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    if name == "nat_economy" {
        return Some(nation(op));
    }
    let rest = name.strip_prefix("ter_")?;
    let w = world(host);
    Some(match rest {
        "create" => create(&w, op),
        "env" => env(&w, op),
        "set" => set(&w, op),
        "call" => call(&w, op),
        "add_brush" => add_brush(&w, op),
        "apply_brush" => apply_brush(&w, op),
        "expect" => expect(&w, op),
        "expect_vertices" => expect_vertices(&w, op),
        "lava_start" => lava_start(&w, op),
        "lava_set" => lava_set(&w, op),
        "lava_call" => lava_call(&w, op),
        "record" => record(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- values -------------------------------------------------------------------------------------

/// A float: a number or `"nan"`.
fn fl(j: &Json, what: &str) -> R<f32> {
    match j.as_str() {
        Some("nan") => Ok(f32::NAN),
        _ => scalar::f32_of(j, what),
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

fn rect(j: &Json, what: &str) -> R<VertRect> {
    let a = arr(j, what)?;
    if a.len() != 4 {
        return Err(format!("'{what}' must be [x0, z0, x1, z1]"));
    }
    Ok(VertRect::new(
        int(&a[0], what)?,
        int(&a[1], what)?,
        int(&a[2], what)?,
        int(&a[3], what)?,
    ))
}

fn jrect(r: VertRect) -> Json {
    Json::Array(vec![ji(r.x0), ji(r.z0), ji(r.x1), ji(r.z1)])
}

/// A float result; NaN as `"nan"`.
fn jnum(v: f32) -> Json {
    if v.is_nan() {
        Json::Str("nan".into())
    } else {
        jf(v)
    }
}

/// `"shipped"` or `{max_height, min_height, …}` over the shipped values.
fn tuning_of(j: Option<&Json>) -> R<TerrainTuning> {
    let mut t = TerrainTuning::SHIPPED;
    let Some(j) = j else { return Ok(t) };
    if j.as_str() == Some("shipped") {
        return Ok(t);
    }
    let fields: [(&str, &mut f32); 9] = [
        ("max_height", &mut t.max_height),
        ("min_height", &mut t.min_height),
        ("new_sea_level", &mut t.new_sea_level),
        ("new_flat_height", &mut t.new_flat_height),
        ("image_height_scale", &mut t.image_height_scale),
        ("sea_level_min", &mut t.sea_level_min),
        ("sea_level_max", &mut t.sea_level_max),
        ("cliff_slope", &mut t.cliff_slope),
        ("saved_78", &mut t.saved_78),
    ];
    for (k, dst) in fields {
        if let Some(v) = opt(j, k) {
            *dst = fl(v, k)?;
        }
    }
    Ok(t)
}

// ---- setup --------------------------------------------------------------------------------------

/// `ter_create`: `cells` ([2, 2]), `altitudes` (index order) or `flat`
/// (300), `sea` (250), `tuning`, `view_ready` (true), or `new: true` for
/// `Create` (the tuning's flat height and sea level). Replaces the world's
/// terrain, bodies of water and lava.
fn create(w: &World, op: &Json) -> R<()> {
    let (cx, cz) = opt(op, "cells").map_or(Ok((2, 2)), |v| pair(v, "cells"))?;
    let tuning = tuning_of(op.get("tuning"))?;
    let t = if opt(op, "new").map_or(Ok(false), |v| bool_of(v, "new"))? {
        Terrain::create(cx as u32, cz as u32, tuning)
    } else {
        let n = ((cx + 1) * (cz + 1)) as usize;
        let alts = match opt(op, "altitudes") {
            Some(a) => arr(a, "altitudes")?
                .iter()
                .map(|v| fl(v, "altitudes"))
                .collect::<R<Vec<f32>>>()?,
            None => vec![opt(op, "flat").map_or(Ok(300.0), |v| fl(v, "flat"))?; n],
        };
        let sea = opt(op, "sea").map_or(Ok(250.0), |v| fl(v, "sea"))?;
        Terrain::from_altitudes(cx as u32, cz as u32, &alts, sea, tuning)
    };
    let mut t = t.ok_or("bad terrain size or altitude count")?;
    if opt(op, "view_ready").map_or(Ok(true), |v| bool_of(v, "view_ready"))? {
        t.post_city_init();
    }
    *w.bodies.borrow_mut() = BodiesOfWater::new(t.cells_x(), t.cells_z());
    *w.terrain.borrow_mut() = Some(t);
    *w.lava.borrow_mut() = None;
    Ok(())
}

/// `ter_env`: `lots` ([[x, z, lot]]), `bridges` ([[x, z, deck or null]]),
/// `poles` ([[x, z]]), `not_flammable` ([[x, z]]), `burn_cells` ([[x, z] or
/// null]), `random` (queued `RandomUint32Uniform` results; else 0).
fn env(w: &World, op: &Json) -> R<()> {
    let triples = |k: &str| -> R<Vec<Vec<Json>>> {
        Ok(match opt(op, k) {
            Some(v) => arr(v, k)?
                .iter()
                .map(|e| arr(e, k).map(|a| a.to_vec()))
                .collect::<R<_>>()?,
            None => Vec::new(),
        })
    };
    if op.get("lots").is_some() {
        w.nav.borrow_mut().lots = triples("lots")?
            .iter()
            .map(|e| {
                Ok((
                    (int(&e[0], "lots")?, int(&e[1], "lots")?),
                    i64_of(&e[2], "lots")? as u64,
                ))
            })
            .collect::<R<_>>()?;
    }
    if op.get("bridges").is_some() {
        w.nav.borrow_mut().bridges = triples("bridges")?
            .iter()
            .map(|e| {
                let deck = match &e[2] {
                    Json::Null => None,
                    v => Some(fl(v, "deck")?),
                };
                Ok(((int(&e[0], "bridges")?, int(&e[1], "bridges")?), deck))
            })
            .collect::<R<_>>()?;
    }
    if let Some(v) = opt(op, "poles") {
        w.nav.borrow_mut().poles = arr(v, "poles")?.iter().map(|e| pair(e, "poles")).collect::<R<_>>()?;
    }
    if let Some(v) = opt(op, "not_flammable") {
        w.fire.borrow_mut().not_flammable = arr(v, "not_flammable")?
            .iter()
            .map(|e| pair(e, "not_flammable"))
            .collect::<R<_>>()?;
    }
    if let Some(v) = opt(op, "burn_cells") {
        w.fire.borrow_mut().burn_cells = arr(v, "burn_cells")?
            .iter()
            .map(|e| match e {
                Json::Null => Ok(None),
                e => pair(e, "burn_cells").map(Some),
            })
            .collect::<R<_>>()?;
    }
    if let Some(v) = opt(op, "random") {
        *w.random.borrow_mut() = arr(v, "random")?
            .iter()
            .map(|e| u32_of(e, "random"))
            .collect::<R<_>>()?;
    }
    Ok(())
}

/// `ter_set` (test support, no messages): `vertex` [x, z] with any of
/// `altitude`, `depth`, `flags`; or `vertices` (a rectangle [x0, z0, x1,
/// z1]) with the same keys; or `cell` [x, z] with `depths` for its corners
/// `(x, z)`, `(x + 1, z)`, `(x, z + 1)`, `(x + 1, z + 1)`; `cached` [min,
/// max] sets the cached heights.
fn set(w: &World, op: &Json) -> R<()> {
    w.with(|t| {
        let apply = |t: &mut Terrain, x: i32, z: i32| -> R<()> {
            if let Some(v) = opt(op, "altitude") {
                // No setter rules.
                if !t.set_raw_altitude(x, z, fl(v, "altitude")?) {
                    return Err("vertex out of bounds".into());
                }
            }
            if let Some(v) = opt(op, "depth") {
                if !t.set_water_depth(x, z, fl(v, "depth")?) {
                    return Err("vertex out of bounds".into());
                }
            }
            if let Some(v) = opt(op, "flags") {
                if !t.set_vertex_flags(x, z, u32_of(v, "flags")? as u16) {
                    return Err("vertex out of bounds".into());
                }
            }
            Ok(())
        };
        if let Some(v) = opt(op, "vertex") {
            let (x, z) = pair(v, "vertex")?;
            apply(t, x, z)?;
        }
        if let Some(v) = opt(op, "vertices") {
            let r = rect(v, "vertices")?;
            for z in r.z0..=r.z1 {
                for x in r.x0..=r.x1 {
                    apply(t, x, z)?;
                }
            }
        }
        if let Some(v) = opt(op, "cell") {
            let (x, z) = pair(v, "cell")?;
            let d = arr(field(op, "depths")?, "depths")?;
            if d.len() != 4 {
                return Err("'depths' must have 4 values".into());
            }
            for (k, (dx, dz)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
                if !t.set_water_depth(x + dx, z + dz, fl(&d[k], "depths")?) {
                    return Err("vertex out of bounds".into());
                }
            }
        }
        if let Some(v) = opt(op, "cached") {
            let a = arr(v, "cached")?;
            t.set_min_max_cache(fl(&a[0], "cached")?, fl(&a[1], "cached")?);
        }
        Ok(())
    })
}

// ---- calls --------------------------------------------------------------------------------------

/// `ter_call`: `method`, `args`, `expect`, `expect_bits` (a float result's
/// bits).
fn call(w: &World, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    let a = |i| arg(op, i);
    let f = |i| -> R<f32> { fl(a(i)?, "argument") };
    let n = |i| -> R<i32> { int(a(i)?, "argument") };
    let u = |i| -> R<u32> { u32_of(a(i)?, "argument") };
    let b = |i| -> R<bool> { bool_of(a(i)?, "argument") };
    let r4 = |i: usize| -> R<VertRect> { Ok(VertRect::new(n(i)?, n(i + 1)?, n(i + 2)?, n(i + 3)?)) };
    let got = if method == "update_bodies" || method == "body" || method == "bodies_map" {
        let tb = w.terrain.borrow();
        let t = tb.as_ref().ok_or("no terrain (ter_create)")?;
        let mut bodies = w.bodies.borrow_mut();
        match method {
            "update_bodies" => {
                bodies.update(t, &*w.nav.borrow());
                Json::Null
            }
            "body" => ji(bodies.get(n(0)?, n(1)?)),
            _ => Json::Array(bodies.map().iter().map(|&v| ji(v)).collect()),
        }
    } else {
        w.with(|t| {
            Ok(match method {
                "post_city_init" => {
                    t.post_city_init();
                    Json::Null
                }
                "altitude" => jnum(t.altitude(f(0)?, f(1)?)),
                "altitude_at_index" => jnum(t.altitude_at_index(u(0)?)),
                "altitude_at_vertex" => jnum(t.altitude_at_vertex(n(0)?, n(1)?)),
                "altitude_at_nearest_grid" => jnum(t.altitude_at_nearest_grid(f(0)?, f(1)?)),
                "altitude_checksum" => jnum(t.altitude_checksum(r4(0)?)),
                "cell_corner_altitudes" => match t.cell_corner_altitudes(n(0)?, n(1)?) {
                    Some(c) => Json::Array(c.iter().map(|&v| jnum(v)).collect()),
                    None => Json::Null,
                },
                "location_is_in_bounds" => Json::Bool(t.location_is_in_bounds(f(0)?, f(1)?)),
                "face_normal" => match t.face_normal(f(0)?, f(1)?) {
                    Some(c) => Json::Array(c.iter().map(|&v| jnum(v)).collect()),
                    None => Json::Null,
                },
                "min_height" => jnum(t.min_height()),
                "max_height" => jnum(t.max_height()),
                "min_allowed_height" => jnum(t.min_allowed_height()),
                "max_allowed_height" => jnum(t.max_allowed_height()),
                "vert_index" => ji(t.vert_index(n(0)?, n(1)?)),
                "sea_level" => jnum(t.sea_level()),
                "water_level" => jnum(t.water_level(f(0)?, f(1)?)),
                "water_depth_at_vertex" => jnum(t.water_depth_at_vertex(n(0)?, n(1)?)),
                "water_depth_at_index" => jnum(t.water_depth_at_index(u(0)?)),
                "max_surface_level" => jnum(t.max_surface_level(n(0)?, n(1)?)),
                "cell_has_water" => Json::Bool(t.cell_has_water(n(0)?, n(1)?)),
                "cell_has_land" => Json::Bool(t.cell_has_land(n(0)?, n(1)?)),
                "is_land_vertex" => Json::Bool(t.is_land_vertex(n(0)?, n(1)?)),
                "is_water_vertex" => Json::Bool(t.is_water_vertex(n(0)?, n(1)?)),
                "is_sea_vertex" => Json::Bool(t.is_sea_vertex(n(0)?, n(1)?)),
                "water_depth_for_cell" => jnum(t.water_depth_for_cell(n(0)?, n(1)?)),
                "has_draft_clearance" => Json::Bool(t.has_draft_clearance(n(0)?, n(1)?)),
                "water_type" => ji(t.water_type(n(0)?, n(1)?)),
                "is_navigable" => Json::Bool(t.is_navigable(
                    n(0)?,
                    n(1)?,
                    n(2)?,
                    a(3).map_or(Ok(0), |v| i64_of(v, "lot"))? as u64,
                    &*w.nav.borrow(),
                )),
                "raise_vertex" => {
                    t.raise_vertex(u(0)?, f(1)?);
                    Json::Null
                }
                "lower_vertex" => {
                    t.lower_vertex(u(0)?, f(1)?);
                    Json::Null
                }
                "set_altitude_at_vertex" => {
                    t.set_altitude_at_vertex(u(0)?, f(1)?);
                    Json::Null
                }
                "set_altitude" => {
                    t.set_altitude(n(0)?, n(1)?, f(2)?);
                    Json::Null
                }
                // [x0, z0, x1, z1, h, redisplay, data4]
                "set_altitude_rect" => {
                    t.set_altitude_rect(r4(0)?, f(4)?, b(5)?, u(6)?);
                    Json::Null
                }
                // [x0, z0, x1, z1, lock]
                "lock_altitude" => {
                    t.lock_altitude(r4(0)?, b(4)?);
                    Json::Null
                }
                "set_sea_level" => Json::Bool(t.set_sea_level(f(0)?)),
                "set_terrain_type" => {
                    t.set_terrain_type(n(0)?);
                    Json::Null
                }
                "terrain_type" => ji(t.terrain_type()),
                // [terrain, water, x0, z0, x1, z1, data4]
                "redisplay" => {
                    t.redisplay_terrain(b(0)?, b(1)?, r4(2)?, u(6)?);
                    Json::Null
                }
                // [terrain, water, ax, az, bx, bz, data4]
                "redisplay_cells" => {
                    t.redisplay_terrain_cells(b(0)?, b(1)?, (n(2)?, n(3)?), (n(4)?, n(5)?), u(6)?);
                    Json::Null
                }
                "begin_long_terrain_operation" => {
                    t.begin_long_terrain_operation();
                    Json::Null
                }
                "end_long_terrain_operation" => {
                    t.end_long_terrain_operation();
                    Json::Null
                }
                "is_triangulation_flipped" => Json::Bool(t.is_triangulation_flipped(n(0)?, n(1)?)),
                "cell_flags" => ji(t.cell_flags(n(0)?, n(1)?)),
                // Round 2 (re-answers17, ABI 66).
                "lerped_water_level" => jnum(t.lerped_water_level(f(0)?, f(1)?)),
                "flooding_altitude" => jnum(t.altitude_at_which_flooding_will_occur(n(0)?, n(1)?)),
                "water_velocity" => Json::Array(t.water_velocity(n(0)?, n(1)?).iter().map(|&v| jnum(v)).collect()),
                "sediment_at" => jnum(t.sediment_at(f(0)?, f(1)?)),
                "water_display_state" => ji(t.water_display_state()),
                "set_water_display_state" => Json::Bool(t.set_water_display_state(u(0)? as u8)),
                "default_brush_level" => ji(t.default_brush_level()),
                "neighbor_mask" => ji(t.neighbor_mask(n(0)?, n(1)?)),
                "talus_angle" => jnum(t.talus_angle()),
                "set_talus_angle" => {
                    t.set_talus_angle(f(0)?);
                    Json::Null
                }
                // [id] -> [walk, stroke] or null
                "brush_angles" => match t.brush(u(0)?) {
                    Some(d) => Json::Array(vec![jnum(d.walk_angle), jnum(d.stroke_angle)]),
                    None => Json::Null,
                },
                // [id, walk, stroke]
                "set_brush_angles" => Json::Bool(t.set_brush_angles(u(0)?, f(1)?, f(2)?)),
                // [cx, cz, w, h, c, s] -> [[x, z] x 4]
                "brush_corners" => {
                    let c = Terrain::brush_rectangle_corners(f(0)?, f(1)?, f(2)?, f(3)?, f(4)?, f(5)?);
                    Json::Array(c.iter().map(|&(x, z)| Json::Array(vec![jnum(x), jnum(z)])).collect())
                }
                // [x0, z0, x1, z1, x2, z2, x3, z3] -> [[z, start, end]]
                "spans" => {
                    let c = [(f(0)?, f(1)?), (f(2)?, f(3)?), (f(4)?, f(5)?), (f(6)?, f(7)?)];
                    let rows = t.compute_spans_for_rectangle(&c);
                    Json::Array(
                        rows.iter()
                            .map(|&(z, s, e)| Json::Array(vec![ji(z), ji(s), ji(e)]))
                            .collect(),
                    )
                }
                other => return Err(format!("unknown method '{other}'")),
            })
        })?
    };
    if let Some(bits) = opt(op, "expect_bits") {
        let want = u32_of(bits, "expect_bits")?;
        let got_bits = match &got {
            Json::Float(v) => (*v as f32).to_bits(),
            other => return Err(format!("expect_bits on a non-float result {}", scalar::show(other))),
        };
        if got_bits != want {
            return Err(format!("bits 0x{got_bits:08X}, expected 0x{want:08X}"));
        }
    }
    check_expect(op, got)
}

// ---- brushes ------------------------------------------------------------------------------------

/// A bitmap: `{width, height, fill}` or `{width, height, pixels: [...]}`
/// (row-major, `v × width + u`).
fn bitmap_of(j: &Json) -> R<BrushBitmap> {
    let wd = u32_of(field(j, "width")?, "width")?;
    let ht = u32_of(field(j, "height")?, "height")?;
    let px = match opt(j, "pixels") {
        Some(p) => arr(p, "pixels")?
            .iter()
            .map(|v| u32_of(v, "pixels").map(|x| x as u8))
            .collect::<R<Vec<u8>>>()?,
        None => vec![u32_of(field(j, "fill")?, "fill")? as u8; (wd * ht) as usize],
    };
    BrushBitmap::new(wd, ht, px).ok_or_else(|| "bad bitmap".to_string())
}

/// Five per-level values: a number (every level) or an array of five.
fn levels(j: &Json, what: &str) -> R<[f32; 5]> {
    match j {
        Json::Array(a) => {
            if a.len() != 5 {
                return Err(format!("'{what}' must have 5 values"));
            }
            let mut v = [0.0; 5];
            for (k, x) in a.iter().enumerate() {
                v[k] = fl(x, what)?;
            }
            Ok(v)
        }
        v => Ok([fl(v, what)?; 5]),
    }
}

/// `ter_add_brush`: `brush` and either `bitmap` (`brush` one of the shipped
/// IDs `0x11`, `0x12`, `0x14`, which walk with the 0.5° default step) or
/// `components` (`[{kind, radius, strength, oriented?, neutral?, bitmap?}]`,
/// `radius` / `strength` a number or five per-level values; `oriented`
/// false, `neutral` 0) with `step?` (the random-walk step in radians, 0);
/// `walk_angle?` (`angle?` is the same) and `stroke_angle?` (0); `expect?`.
fn add_brush(w: &World, op: &Json) -> R<()> {
    let id = u32_of(field(op, "brush")?, "brush")?;
    let mut d = match opt(op, "components") {
        Some(cs) => BrushDescription {
            components: arr(cs, "components")?
                .iter()
                .map(|c| {
                    Ok(BrushComponent {
                        kind: u32_of(field(c, "kind")?, "kind")?,
                        radius: levels(field(c, "radius")?, "radius")?,
                        strength: levels(field(c, "strength")?, "strength")?,
                        oriented: opt(c, "oriented").map_or(Ok(false), |v| bool_of(v, "oriented"))?,
                        neutral: opt(c, "neutral").map_or(Ok(0), |v| u32_of(v, "neutral"))? as u8,
                        bitmap: opt(c, "bitmap").map(bitmap_of).transpose()?,
                    })
                })
                .collect::<R<_>>()?,
            random_walk_step: opt(op, "step").map_or(Ok(0.0), |v| fl(v, "step"))?,
            walk_angle: 0.0,
            stroke_angle: 0.0,
        },
        None => {
            let bmp = bitmap_of(field(op, "bitmap")?)?;
            shipped_brush(id, Some(bmp)).ok_or("not a shipped brush")?
        }
    };
    if let Some(a) = opt(op, "walk_angle").or_else(|| opt(op, "angle")) {
        d.walk_angle = fl(a, "walk_angle")?;
    }
    if let Some(a) = opt(op, "stroke_angle") {
        d.stroke_angle = fl(a, "stroke_angle")?;
    }
    let ok = w.with(|t| Ok(t.add_brush(id, d)))?;
    check_expect(op, Json::Bool(ok))
}

/// `ter_apply_brush`: `brush`, `from` / `to` [x, z] metres, `level` (−1),
/// `redisplay` (true), `strength` (1), `radius` (1), `data4` (3), `target`
/// (100); `expect` (the result), `out` (the touched rectangle or null).
fn apply_brush(w: &World, op: &Json) -> R<()> {
    let id = u32_of(field(op, "brush")?, "brush")?;
    let pt = |k: &str| -> R<(f32, f32)> {
        let a = arr(field(op, k)?, k)?;
        Ok((fl(&a[0], k)?, fl(&a[1], k)?))
    };
    let (fx, fz) = pt("from")?;
    let (tx, tz) = pt("to")?;
    let num = |k: &str, d: f32| opt(op, k).map_or(Ok(d), |v| fl(v, k));
    let mut p = BrushParams {
        from_x: fx,
        from_z: fz,
        to_x: tx,
        to_z: tz,
        level: opt(op, "level").map_or(Ok(-1), |v| int(v, "level"))?,
        redisplay: opt(op, "redisplay").map_or(Ok(true), |v| bool_of(v, "redisplay"))?,
        strength_scale: num("strength", 1.0)?,
        radius_scale: num("radius", 1.0)?,
        data4: opt(op, "data4").map_or(Ok(3), |v| u32_of(v, "data4"))?,
        out: None,
        target_height: num("target", 100.0)?,
    };
    let random = &w.random;
    let events = &w.events;
    let ok = w.with(|t| {
        Ok(t.apply_brush(id, &mut p, &mut |n| {
            events.push("random", vec![("n", ji(n))]);
            random.borrow_mut().pop_front().unwrap_or(0)
        }))
    })?;
    check_expect(op, Json::Bool(ok))?;
    if let Some(want) = op.get("out") {
        let got = p.out.map_or(Json::Null, jrect);
        if !scalar::matches(&got, want, false) {
            return Err(format!("out {}, expected {}", scalar::show(&got), scalar::show(want)));
        }
    }
    Ok(())
}

// ---- checks -------------------------------------------------------------------------------------

/// `ter_expect`: `state` (partial: `cells`, `sea_level`, `min_height`,
/// `max_height`, `view_ready`, `terrain_type`, `saved_78`, `posts_pending`).
fn expect(w: &World, op: &Json) -> R<()> {
    let want = field(op, "state")?;
    let tb = w.terrain.borrow();
    let t = tb.as_ref().ok_or("no terrain (ter_create)")?;
    let got = vec![
        ("cells".to_owned(), Json::Array(vec![ji(t.cells_x()), ji(t.cells_z())])),
        ("sea_level".to_owned(), jnum(t.sea_level())),
        ("min_height".to_owned(), jnum(t.min_height())),
        ("max_height".to_owned(), jnum(t.max_height())),
        ("view_ready".to_owned(), Json::Bool(t.view_ready())),
        ("terrain_type".to_owned(), ji(t.terrain_type())),
        ("saved_78".to_owned(), jnum(t.saved_78())),
        ("posts_pending".to_owned(), ji(t.pending_posts() as i64)),
    ];
    check_state(&got, want)
}

/// `ter_expect_vertices`: over `rect` (default the whole terrain), every
/// vertex's altitude lies in [`min`, `max`]; with `baseline`, every vertex
/// whose altitude differs from it lies inside `changed_within` (a
/// rectangle) and `changed` (if given) is their count; `vertex` [x, z] with
/// `altitude?`, `depth?`, `flags?` checks one vertex (`depth` exact bits
/// with `exact: true`).
fn expect_vertices(w: &World, op: &Json) -> R<()> {
    let tb = w.terrain.borrow();
    let t = tb.as_ref().ok_or("no terrain (ter_create)")?;
    if let Some(v) = opt(op, "vertex") {
        let (x, z) = pair(v, "vertex")?;
        let wv = t.water_vertex(x, z).ok_or("vertex out of bounds")?;
        let got = vec![
            ("altitude".to_owned(), jnum(t.altitude_at_vertex(x, z))),
            ("depth".to_owned(), jnum(wv.depth)),
            ("flags".to_owned(), ji(t.vertex_flags(x, z))),
        ];
        let mut want = Vec::new();
        for k in ["altitude", "depth", "flags"] {
            if let Some(v) = op.get(k) {
                want.push((k.to_owned(), v.clone()));
            }
        }
        check_state(&got, &Json::Object(want))?;
        return Ok(());
    }
    let r = match opt(op, "rect") {
        Some(v) => rect(v, "rect")?,
        None => t.whole_city(),
    };
    let lo = opt(op, "min").map(|v| fl(v, "min")).transpose()?;
    let hi = opt(op, "max").map(|v| fl(v, "max")).transpose()?;
    let base = opt(op, "baseline").map(|v| fl(v, "baseline")).transpose()?;
    let within = opt(op, "changed_within")
        .map(|v| rect(v, "changed_within"))
        .transpose()?;
    let mut changed = 0i64;
    for z in r.z0..=r.z1 {
        for x in r.x0..=r.x1 {
            let a = t.altitude_at_vertex(x, z);
            if lo.is_some_and(|m| a < m) || hi.is_some_and(|m| a > m) {
                return Err(format!("vertex ({x}, {z}) at {a}, outside [{lo:?}, {hi:?}]"));
            }
            if let Some(b) = base {
                if a != b {
                    changed += 1;
                    if let Some(c) = within {
                        if x < c.x0 || x > c.x1 || z < c.z0 || z > c.z1 {
                            return Err(format!("vertex ({x}, {z}) changed outside {c:?}"));
                        }
                    }
                }
            }
        }
    }
    if let Some(n) = opt(op, "changed") {
        let n = i64_of(n, "changed")?;
        if n != changed {
            return Err(format!("{changed} vertices changed, expected {n}"));
        }
    }
    Ok(())
}

// ---- lava ---------------------------------------------------------------------------------------

/// `ter_lava_start`: `at` [x, z] metres, `radius` (20); `expect?` the
/// rectangles `{source, active, burn}`.
fn lava_start(w: &World, op: &Json) -> R<()> {
    let a = arr(field(op, "at")?, "at")?;
    let (x, z) = (fl(&a[0], "at")?, fl(&a[1], "at")?);
    let radius = opt(op, "radius").map_or(Ok(20.0), |v| fl(v, "radius"))?;
    let tb = w.terrain.borrow();
    let t = tb.as_ref().ok_or("no terrain (ter_create)")?;
    let l = Lava::start(t, x, z, radius, LavaTuning::SHIPPED);
    let got = Json::Object(vec![
        ("source".into(), jrect(l.source)),
        ("active".into(), jrect(l.active)),
        ("burn".into(), jrect(l.burn)),
    ]);
    *w.lava.borrow_mut() = Some(l);
    check_expect(op, got)
}

/// `ter_lava_set`: `vertex` [x, z] and `level`; or `around` [x, z] and
/// `level` (its eight neighbours); `production` / `solidification` rates.
fn lava_set(w: &World, op: &Json) -> R<()> {
    let tb = w.terrain.borrow();
    let t = tb.as_ref().ok_or("no terrain (ter_create)")?;
    let mut lb = w.lava.borrow_mut();
    let l = lb.as_mut().ok_or("no lava (ter_lava_start)")?;
    if let Some(v) = opt(op, "vertex") {
        let (x, z) = pair(v, "vertex")?;
        l.set_level(t, x, z, fl(field(op, "level")?, "level")?);
    }
    if let Some(v) = opt(op, "around") {
        let (x, z) = pair(v, "around")?;
        let lv = fl(field(op, "level")?, "level")?;
        for (dx, dz) in fc4_core::terrain::NEIGHBOURS {
            l.set_level(t, x + dx, z + dz, lv);
        }
    }
    if let Some(v) = opt(op, "production") {
        l.production_rate = fl(v, "production")?;
    }
    if let Some(v) = opt(op, "solidification") {
        l.solidification_rate = fl(v, "solidification")?;
    }
    Ok(())
}

/// `ter_lava_call`: `flow_amount [x, z, dt]` (`{amount, to}`, `to` null at
/// the edge or with no flow), `transfer [x, z, dt]` (the flow step for one
/// vertex in isolation: `SetThingsOnFire` on the neighbour, then the levels
/// move; returns `{amount, to, mode}`), `level [x, z]`, `flow [dt]`
/// (`DoLavaFlow`), `burn` (`BurnSurroundingStuff`), `rects`,
/// `lowest_neighbour [x, z]` (`[x, z]` or null, §11.3) and
/// `set_things_on_fire [x, z, old, amount]` (the mode, §11.5).
fn lava_call(w: &World, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    let a = |i| arg(op, i);
    let n = |i| -> R<i32> { int(a(i)?, "argument") };
    let f = |i| -> R<f32> { fl(a(i)?, "argument") };
    let mut lb = w.lava.borrow_mut();
    let l = lb.as_mut().ok_or("no lava (ter_lava_start)")?;
    let got = match method {
        "flow" => {
            let mut fire = w.fire.borrow_mut();
            let dt = f(0)?;
            w.with(|t| {
                l.flow(
                    t,
                    dt,
                    &mut FireHost {
                        fire: &mut fire,
                        events: &w.events,
                    },
                );
                Ok(())
            })?;
            Json::Null
        }
        "burn" => {
            let mut fire = w.fire.borrow_mut();
            l.burn_surrounding(&mut FireHost {
                fire: &mut fire,
                events: &w.events,
            });
            Json::Null
        }
        "rects" => Json::Object(vec![
            ("source".into(), jrect(l.source)),
            ("active".into(), jrect(l.active)),
            ("burn".into(), jrect(l.burn)),
        ]),
        _ => {
            let tb = w.terrain.borrow();
            let t = tb.as_ref().ok_or("no terrain (ter_create)")?;
            match method {
                "level" => jnum(l.level(t, n(0)?, n(1)?)),
                "lowest_neighbour" => match l.lowest_neighbour(t, n(0)?, n(1)?) {
                    Some((x, z)) => Json::Array(vec![ji(x), ji(z)]),
                    None => Json::Null,
                },
                "set_things_on_fire" => {
                    let mut fire = w.fire.borrow_mut();
                    let mode = l.set_things_on_fire(
                        t,
                        n(0)?,
                        n(1)?,
                        f(2)?,
                        f(3)?,
                        &mut FireHost {
                            fire: &mut fire,
                            events: &w.events,
                        },
                    );
                    Json::Str(mode_name(mode).into())
                }
                "flow_amount" | "transfer" => {
                    let (x, z) = (n(0)?, n(1)?);
                    let (amount, to) = l.flow_amount(t, x, z, f(2)?);
                    let jto = to.map_or(Json::Null, |(a, b)| Json::Array(vec![ji(a), ji(b)]));
                    let mut kv = vec![("amount".to_owned(), jnum(amount)), ("to".to_owned(), jto)];
                    if method == "transfer" {
                        let mut mode = FireMode::None;
                        if amount > 0.0 {
                            if let Some((nx, nz)) = to {
                                let old = l.level(t, nx, nz);
                                let mut fire = w.fire.borrow_mut();
                                mode = l.set_things_on_fire(
                                    t,
                                    nx,
                                    nz,
                                    old,
                                    amount,
                                    &mut FireHost {
                                        fire: &mut fire,
                                        events: &w.events,
                                    },
                                );
                                l.set_level(t, nx, nz, old + amount);
                            }
                            let me = l.level(t, x, z);
                            l.set_level(t, x, z, me - amount);
                        }
                        kv.push(("mode".to_owned(), Json::Str(mode_name(mode).into())));
                    }
                    Json::Object(kv)
                }
                other => return Err(format!("unknown method '{other}'")),
            }
        }
    };
    check_expect(op, got)
}

fn mode_name(mode: FireMode) -> &'static str {
    match mode {
        FireMode::None => "none",
        FireMode::Ignite => "ignite",
        FireMode::Burn => "burn",
    }
}

// ---- records ------------------------------------------------------------------------------------

/// `ter_record`: `sizes` (partial `{misc, altitudes, properties, edges:
/// [4]}`) of the payloads; `round_trip: true` loads the three records into
/// a new terrain and checks altitudes, flags, the cached heights and the
/// misc record are the same; `sea_after_load` loads them and checks the
/// loaded sea level (the tuning's, §13.1, TR10); `edge` `{side, values?:
/// [..], count?}`.
fn record(w: &World, op: &Json) -> R<()> {
    let tb = w.terrain.borrow();
    let t = tb.as_ref().ok_or("no terrain (ter_create)")?;
    let (misc, alts, props) = (t.write_misc(), t.write_altitudes(), t.write_properties());
    if let Some(want) = opt(op, "sizes") {
        let edges = (0..4)
            .map(|s| ji(t.write_edge(s).map_or(0, |e| e.len() as i64)))
            .collect();
        let got = vec![
            ("misc".to_owned(), ji(misc.len() as i64)),
            ("altitudes".to_owned(), ji(alts.len() as i64)),
            ("properties".to_owned(), ji(props.len() as i64)),
            ("edges".to_owned(), Json::Array(edges)),
        ];
        check_state(&got, want)?;
    }
    if opt(op, "round_trip").map_or(Ok(false), |v| bool_of(v, "round_trip"))? {
        let u = Terrain::load(&misc, &alts, &props, *t.tuning()).map_err(|e| format!("load: {e:?}"))?;
        if u.altitudes() != t.altitudes() || u.flag_words() != t.flag_words() {
            return Err("altitudes or flags differ after the round trip".into());
        }
        if u.write_misc() != misc {
            return Err("the misc record differs after the round trip".into());
        }
        if u.view_ready() {
            return Err("a loaded terrain is view-ready".into());
        }
    }
    if let Some(want) = opt(op, "sea_after_load") {
        let u = Terrain::load(&misc, &alts, &props, *t.tuning()).map_err(|e| format!("load: {e:?}"))?;
        let got = jnum(u.sea_level());
        if !scalar::matches(&got, want, false) {
            return Err(format!(
                "sea level after load {}, expected {}",
                scalar::show(&got),
                scalar::show(want)
            ));
        }
    }
    if let Some(e) = opt(op, "edge") {
        let side = u32_of(field(e, "side")?, "side")?;
        let bytes = t.write_edge(side).ok_or("bad side")?;
        let v = Terrain::read_edge(&bytes).map_err(|e| format!("{e:?}"))?;
        if let Some(c) = opt(e, "count") {
            if v.len() as i64 != i64_of(c, "count")? {
                return Err(format!("edge has {} values", v.len()));
            }
        }
        if let Some(want) = opt(e, "values") {
            let got = Json::Array(v.iter().map(|&x| jnum(x)).collect());
            if !scalar::matches(&got, want, false) {
                return Err(format!("edge {}, expected {}", scalar::show(&got), scalar::show(want)));
            }
        }
    }
    Ok(())
}

// ---- nation -------------------------------------------------------------------------------------

/// `nat_economy`: `difficulty` [3], `curves` (`{"id": [x0, y0, x1, y1, …]}`)
/// and `checks` (`[[demandID, months, difficulty or null (no city),
/// expected]]`): `GetEconomyModifier` (nation.md §2).
fn nation(op: &Json) -> R<()> {
    let mut n = NationEconomy::default();
    if let Some(d) = opt(op, "difficulty") {
        for (k, v) in arr(d, "difficulty")?.iter().take(3).enumerate() {
            n.difficulty[k] = fl(v, "difficulty")?;
        }
    }
    if let Some(c) = opt(op, "curves") {
        for (k, v) in c.as_object().ok_or("'curves' must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("curve keys must be IDs")? as u32;
            let pts = arr(v, "curve")?
                .iter()
                .map(|x| fl(x, "curve"))
                .collect::<R<Vec<f32>>>()?;
            n.curves
                .insert(id, ResponseCurve::from_flat(&pts).ok_or("empty curve")?);
        }
    }
    for (i, c) in arr(field(op, "checks")?, "checks")?.iter().enumerate() {
        let c = arr(c, "check")?;
        let id = u32_of(&c[0], "demand ID")?;
        let months = i64_of(&c[1], "months")? as i32;
        let diff = match &c[2] {
            Json::Null => None,
            v => Some(u32_of(v, "difficulty")?),
        };
        let got = jf(n.modifier_with_city(id, months, diff));
        if !scalar::matches(&got, &c[3], false) {
            return Err(format!(
                "check #{i}: got {}, expected {}",
                scalar::show(&got),
                scalar::show(&c[3])
            ));
        }
    }
    Ok(())
}
