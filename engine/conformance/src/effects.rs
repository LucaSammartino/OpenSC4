// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/effects.md` (`ef_…`): an effects manager
//! (`fc4_core::effects`) over a scripted host that logs every call (terrain
//! brushes, fire, toxic, demolition, extinguishing, pauses, posts). See
//! README.md ("Effects ops").

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use fc4_core::effects::{
    component_type, shipped, Component, ComponentRecord, DemolishFilter, EffectDescription, EffectDirectory, EffectId,
    EffectPost, EffectRect, EffectsHost, EffectsManager, FinishMode, GameRecord, SELECT_RANGE, SELECT_SEED,
};
use fc4_core::rng::{FixedSeeds, RzRandomSuper};
use fc4_core::terrain::BrushParams;

use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, i64_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

mod ef21;

struct Env {
    dilation: Option<f32>,
    cells: (i32, i32),
    render_active: bool,
    brush_calls: usize,
    events: Rc<EventLog>,
}

impl EffectsHost for Env {
    fn simulator(&mut self) -> Option<f32> {
        self.dilation
    }
    fn sim_pause(&mut self) {
        self.events.push("pause", vec![]);
    }
    fn sim_resume(&mut self) {
        self.events.push("resume", vec![]);
    }
    fn sim_hidden_pause(&mut self) {
        self.events.push("hidden_pause", vec![]);
    }
    fn sim_hidden_resume(&mut self) {
        self.events.push("hidden_resume", vec![]);
    }
    fn clock_pause(&mut self, on: bool) {
        self.events.push("clock", vec![("on", Json::Bool(on))]);
    }
    fn city_cells(&mut self) -> (i32, i32) {
        self.cells
    }
    fn location_in_bounds(&mut self, x: f32, z: f32) -> bool {
        x >= 0.0 && z >= 0.0 && x < self.cells.0 as f32 * 16.0 && z < self.cells.1 as f32 * 16.0
    }
    fn apply_brush(&mut self, brush_id: u32, p: &mut BrushParams) -> bool {
        self.brush_calls += 1;
        self.events.push(
            "brush",
            vec![
                ("brush", ji(brush_id)),
                ("from", Json::Array(vec![jf(p.from_x), jf(p.from_z)])),
                ("to", Json::Array(vec![jf(p.to_x), jf(p.to_z)])),
                ("level", ji(p.level)),
                ("strength", jf(p.strength_scale)),
                ("radius", jf(p.radius_scale)),
                ("redisplay", Json::Bool(p.redisplay)),
                ("data4", ji(p.data4)),
                ("target", jf(p.target_height)),
            ],
        );
        true
    }
    fn begin_long_terrain_operation(&mut self) {
        self.events.push("begin_long", vec![]);
    }
    fn end_long_terrain_operation(&mut self) {
        self.events.push("end_long", vec![]);
    }
    fn ignite_cell_radius(&mut self, x: i32, z: i32, r0: i32, r1: i32) {
        self.events.push(
            "ignite",
            vec![("x", ji(x)), ("z", ji(z)), ("r0", ji(r0)), ("r1", ji(r1))],
        );
    }
    fn set_fire_cause(&mut self, cause: u32) {
        self.events.push("cause", vec![("cause", ji(cause))]);
    }
    fn toxic_attempt_start(&mut self, x: i32, z: i32) {
        self.events.push("toxic", vec![("x", ji(x)), ("z", ji(z))]);
    }
    fn demolish_region(&mut self, r: EffectRect, flags: u32, dezone: bool, f: &DemolishFilter) {
        self.events.push(
            "demolish",
            vec![
                ("rect", Json::Array(vec![ji(r.x0), ji(r.z0), ji(r.x1), ji(r.z1)])),
                ("flags", ji(flags)),
                ("dezone", Json::Bool(dezone)),
                ("no_networks", Json::Bool(f.no_networks)),
                ("no_flora", Json::Bool(f.no_flora)),
                ("min_size", jf(f.min_size)),
                ("max_size", jf(f.max_size)),
            ],
        );
    }
    fn extinguish_cell(&mut self, x: i32, z: i32, amount: i32) {
        self.events
            .push("extinguish", vec![("x", ji(x)), ("z", ji(z)), ("amount", ji(amount))]);
    }
    fn post(&mut self, m: EffectPost) {
        self.events.push(
            "post",
            vec![
                ("type", ji(m.msg_type)),
                ("data1", Json::Int(m.data1 as i64)),
                ("data2", ji(m.data2)),
                ("data3", ji(m.data3)),
            ],
        );
    }
    fn render_component_active(&mut self, _effect: EffectId, _component: u32, _kind: u8) -> bool {
        self.render_active
    }
}

pub(crate) struct World {
    events: Rc<EventLog>,
    env: RefCell<Env>,
    mgr: RefCell<Option<EffectsManager>>,
    labels: RefCell<BTreeMap<String, EffectId>>,
}

impl World {
    fn new() -> Rc<World> {
        let events = Rc::new(EventLog::default());
        Rc::new(World {
            events: events.clone(),
            env: RefCell::new(Env {
                dilation: Some(1.0),
                cells: (64, 64),
                render_active: false,
                brush_calls: 0,
                events,
            }),
            mgr: RefCell::new(None),
            labels: RefCell::new(BTreeMap::new()),
        })
    }

    fn with<T>(&self, f: impl FnOnce(&mut EffectsManager, &mut Env) -> R<T>) -> R<T> {
        let mut m = self.mgr.borrow_mut();
        let m = m.as_mut().ok_or("no effects manager (ef_create)")?;
        f(m, &mut self.env.borrow_mut())
    }

    fn label(&self, op: &Json) -> R<EffectId> {
        let l = str_of(field(op, "effect")?, "effect")?;
        self.labels
            .borrow()
            .get(l)
            .copied()
            .ok_or_else(|| format!("unknown effect label '{l}'"))
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "effects", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("ef_")?;
    let w = world(host);
    Some(match rest {
        "create" => create(&w, op),
        "env" => env(&w, op),
        "effect" => effect(&w, op),
        "call" => call(&w, op),
        "run_stroke" => run_stroke(&w, op),
        "child" => child(&w, op),
        "expect" => expect(&w, op),
        "expect_brush" => expect_brush(&w, op),
        "expect_game" => expect_game(&w, op),
        "expect_manager" => expect_manager(&w, op),
        "select" => select(op),
        "dilation" => dilation(op),
        "particles" => particles(op),
        "observation" => ef21::observation(op),
        "payload" => ef21::payload(op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- setup -----------------------------------------------------------------------------------------

fn kind_of(j: &Json) -> R<u8> {
    if let Some(v) = j.as_i64() {
        return Ok(v as u8);
    }
    Ok(match str_of(j, "type")? {
        "particles" => component_type::PARTICLES,
        "decal" => component_type::DECAL,
        "meta" => component_type::META,
        "brush" => component_type::BRUSH,
        "attractor" => component_type::ATTRACTOR,
        "game" => component_type::GAME,
        "sound" => component_type::SOUND,
        other => return Err(format!("unknown component type '{other}'")),
    })
}

fn component(j: &Json) -> R<ComponentRecord> {
    let kind = kind_of(field(j, "type")?)?;
    let mut c = ComponentRecord::new(kind, opt(j, "index").map_or(Ok(0), |v| u32_of(v, "index"))?);
    if let Some(n) = opt(j, "name") {
        c.name = str_of(n, "name")?.to_owned();
    }
    if let Some(f) = opt(j, "flags") {
        c.flags = u32_of(f, "flags")?;
    }
    if let Some(l) = opt(j, "lod") {
        let l = arr(l, "lod")?;
        c.lod_min = u32_of(&l[0], "lod")? as u8;
        c.lod_max = u32_of(&l[1], "lod")? as u8;
    }
    if let Some(g) = opt(j, "group") {
        c.select_group = u32_of(g, "group")? as u16;
    }
    if let Some(g) = opt(j, "weight") {
        c.select_weight = u32_of(g, "weight")? as u16;
    }
    Ok(c)
}

fn game_record(j: &Json) -> R<GameRecord> {
    let mut g = GameRecord::default();
    let o = j.as_object().ok_or("a game record must be an object")?;
    for (k, v) in o {
        match k.as_str() {
            "flags" => g.flags = u32_of(v, k)?,
            "demolish" => g.demolish_radius = i64_of(v, k)? as i32,
            "demolish_flags" => g.demolish_flags = u32_of(v, k)?,
            "min_size" => g.min_demolish_size = scalar::f32_of(v, k)?,
            "max_size" => g.max_demolish_size = scalar::f32_of(v, k)?,
            "burn" => g.burn_radius = i64_of(v, k)? as i32,
            "message" => g.message_type = u32_of(v, k)?,
            "message_data" => g.message_data = u32_of(v, k)?,
            "duration" => g.duration = scalar::f32_of(v, k)?,
            "toxic" => g.toxic = i64_of(v, k)? as i32,
            "extinguish" => g.extinguish = i64_of(v, k)? as i32,
            other => return Err(format!("unknown game record field '{other}'")),
        }
    }
    Ok(g)
}

/// `"volcano"` (shipped::volcano_directory) or an inline directory.
fn directory(j: Option<&Json>) -> R<EffectDirectory> {
    let Some(j) = j else {
        return Ok(shipped::volcano_directory());
    };
    if let Some(s) = j.as_str() {
        return match s {
            "volcano" => Ok(shipped::volcano_directory()),
            other => Err(format!("unknown directory '{other}'")),
        };
    }
    let games = match opt(j, "games") {
        None => shipped::game_records(),
        Some(g) => arr(g, "games")?.iter().map(game_record).collect::<R<_>>()?,
    };
    let mut effects = Vec::new();
    let mut names = Vec::new();
    for e in arr(field(j, "effects")?, "effects")? {
        let comps = arr(field(e, "components")?, "components")?
            .iter()
            .map(component)
            .collect::<R<Vec<_>>>()?;
        let mut d = EffectDescription::new(opt(e, "priority").map_or(Ok(1), |v| u32_of(v, "priority"))?, comps);
        if let Some(f) = opt(e, "flags") {
            d.flags = u32_of(f, "flags")?;
        }
        effects.push(d);
        names.push((
            str_of(field(e, "name")?, "name")?.to_owned(),
            (effects.len() - 1) as u32,
        ));
    }
    Ok(EffectDirectory::from_parts(
        shipped::BRUSHES.to_vec(),
        games,
        effects,
        names,
    ))
}

fn create(w: &World, op: &Json) -> R<()> {
    let dir = directory(opt(op, "directory"))?;
    let seed = opt(op, "seed").map_or(Ok(0x1234_5678), |v| u32_of(v, "seed"))?;
    let mut m = EffectsManager::new(seed);
    m.set_directory(Rc::new(dir));
    if let Some(p) = opt(op, "max_priority") {
        m.set_max_priority(u32_of(p, "max_priority")?);
    }
    if let Some(f) = opt(op, "finish_mode") {
        m.set_finish_mode(match str_of(f, "finish_mode")? {
            "hook" => FinishMode::Hook,
            "draw_pass" => FinishMode::DrawPass,
            other => return Err(format!("unknown finish mode '{other}'")),
        });
    }
    if let Some(n) = opt(op, "nested_update") {
        m.set_nested_update(bool_of(n, "nested_update")?);
    }
    *w.mgr.borrow_mut() = Some(m);
    w.labels.borrow_mut().clear();
    Ok(())
}

fn env(w: &World, op: &Json) -> R<()> {
    let mut e = w.env.borrow_mut();
    if let Some(d) = op.get("dilation") {
        e.dilation = match d {
            Json::Null => None,
            v => Some(scalar::f32_of(v, "dilation")?),
        };
    }
    if let Some(c) = opt(op, "cells") {
        let c = arr(c, "cells")?;
        e.cells = (i64_of(&c[0], "cells")? as i32, i64_of(&c[1], "cells")? as i32);
    }
    if let Some(r) = opt(op, "render_active") {
        e.render_active = bool_of(r, "render_active")?;
    }
    Ok(())
}

/// `ef_effect`: `CreateVisualEffect(name)`, labelled `as`; `expect` is the
/// return value (false: no label).
fn effect(w: &World, op: &Json) -> R<()> {
    let name = str_of(field(op, "name")?, "name")?;
    let id = w.with(|m, _| Ok(m.create(name)))?;
    check_expect(op, Json::Bool(id.is_some()))?;
    if let (Some(id), Some(l)) = (id, opt(op, "as")) {
        w.labels.borrow_mut().insert(str_of(l, "as")?.to_owned(), id);
    }
    if let Some(i) = opt(op, "expect_index") {
        let got = id
            .and_then(|id| w.with(|m, _| Ok(m.effect(id))).ok().flatten())
            .map(|e| e.index);
        if got != Some(i64_of(i, "expect_index")? as i32) {
            return Err(format!("index {got:?}, expected {}", scalar::show(i)));
        }
    }
    Ok(())
}

fn f(op: &Json, i: usize) -> R<f32> {
    scalar::f32_of(arg(op, i)?, "argument")
}

fn call(w: &World, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    let repeat = opt(op, "repeat").map_or(Ok(1), |v| i64_of(v, "repeat"))?;
    let mut last = Json::Null;
    for _ in 0..repeat {
        last = match method {
            "tick" => {
                let us = u32_of(arg(op, 0)?, "us")?;
                w.with(|m, h| {
                    m.tick(us, h);
                    Ok(Json::Null)
                })?
            }
            // A host frame: the tick, then the finishing hook, then the draw
            // pass (only the one of the finish mode does anything).
            "frame" => {
                let us = u32_of(arg(op, 0)?, "us")?;
                w.with(|m, h| {
                    m.tick(us, h);
                    let a = m.finish_hook(h);
                    let b = m.draw_pass(h);
                    Ok(Json::Bool(a || b))
                })?
            }
            "finish_hook" => w.with(|m, h| Ok(Json::Bool(m.finish_hook(h))))?,
            "draw_pass" => w.with(|m, h| Ok(Json::Bool(m.draw_pass(h))))?,
            "effects_tick" => w.with(|m, h| Ok(ji(m.effects_tick(h) as i64)))?,
            "queue_effect_files" => w.with(|m, _| {
                m.queue_effect_files();
                Ok(Json::Null)
            })?,
            "render_properties_changed" => {
                let p = u32_of(arg(op, 0)?, "priority")?;
                w.with(|m, h| {
                    m.render_properties_changed(p, h);
                    Ok(Json::Null)
                })?
            }
            "shutdown" => w.with(|m, h| {
                m.shutdown(h);
                Ok(Json::Null)
            })?,
            "set_zoom" => {
                let (z, o) = (
                    i64_of(arg(op, 0)?, "zoom")? as i32,
                    i64_of(arg(op, 1)?, "offset")? as i32,
                );
                w.with(|m, h| {
                    m.set_zoom(z, o, h);
                    Ok(Json::Null)
                })?
            }
            "set_max_priority" => {
                let p = u32_of(arg(op, 0)?, "priority")?;
                w.with(|m, _| {
                    m.set_max_priority(p);
                    Ok(Json::Null)
                })?
            }
            "has" => {
                let n = str_of(arg(op, 0)?, "name")?;
                w.with(|m, _| Ok(Json::Bool(m.has_visual_effect(n))))?
            }
            _ => {
                let id = w.label(op)?;
                match method {
                    "start" => {
                        let mode = opt(op, "args").map_or(Ok(0), |_| u32_of(arg(op, 0)?, "mode"))?;
                        w.with(|m, h| Ok(Json::Bool(m.start(id, mode, h))))?
                    }
                    "stop" => {
                        let mode = opt(op, "args").map_or(Ok(0), |_| u32_of(arg(op, 0)?, "mode"))?;
                        w.with(|m, h| Ok(Json::Bool(m.stop(id, mode, h))))?
                    }
                    "set_position" => {
                        let p = [f(op, 0)?, f(op, 1)?, f(op, 2)?];
                        w.with(|m, _| Ok(Json::Bool(m.set_position(id, p))))?
                    }
                    "set_end_message" => {
                        let t = u32_of(arg(op, 0)?, "type")?;
                        let d = i64_of(arg(op, 1)?, "data1")? as u64;
                        w.with(|m, _| Ok(Json::Bool(m.set_end_message(id, t, d))))?
                    }
                    "forward_parameter" => {
                        let i = u32_of(arg(op, 0)?, "index")?;
                        let v = i64_of(arg(op, 1)?, "value")? as u64;
                        w.with(|m, _| Ok(Json::Bool(m.forward_parameter(id, i, v))))?
                    }
                    "add_ref" => w.with(|m, _| Ok(Json::Bool(m.add_ref(id))))?,
                    "release" => w.with(|m, _| Ok(Json::Bool(m.release(id))))?,
                    "is_started" => w.with(|m, _| Ok(Json::Bool(m.is_started(id))))?,
                    "is_stopping" => w.with(|m, _| Ok(Json::Bool(m.is_stopping(id))))?,
                    other => return Err(format!("unknown method '{other}'")),
                }
            }
        };
    }
    check_expect(op, last)
}

/// The brush component of an effect (the `n`-th, default 0).
fn brush(w: &World, op: &Json) -> R<fc4_core::effects::BrushEffect> {
    let id = w.label(op)?;
    let n = opt(op, "n").map_or(Ok(0), |v| i64_of(v, "n"))? as usize;
    w.with(|m, _| match m.component_of_kind(id, component_type::BRUSH, n) {
        Some(Component::Brush(b)) => Ok(b.clone()),
        _ => Err("no brush component".into()),
    })
}

/// `ef_run_stroke`: ticks `us` until the effect's brush is inactive
/// (`max?` ticks, default 10000); checks `ticks`, `applications` (the brush
/// calls made) and `first` (the calls of the first ticks).
fn run_stroke(w: &World, op: &Json) -> R<()> {
    let us = u32_of(field(op, "us")?, "us")?;
    let max = opt(op, "max").map_or(Ok(10_000), |v| i64_of(v, "max"))?;
    let calls = || w.env.borrow().brush_calls;
    let mut per_tick = Vec::new();
    let mut ticks = 0i64;
    loop {
        let before = calls();
        w.with(|m, h| {
            m.tick(us, h);
            Ok(())
        })?;
        per_tick.push(calls() - before);
        ticks += 1;
        if !brush(w, op)?.active {
            break;
        }
        if ticks >= max {
            return Err(format!("the stroke did not end in {max} ticks"));
        }
    }
    if let Some(t) = opt(op, "ticks") {
        if ticks != i64_of(t, "ticks")? {
            return Err(format!("ended after {ticks} ticks, expected {}", scalar::show(t)));
        }
    }
    let total: usize = per_tick.iter().sum();
    if let Some(a) = opt(op, "applications") {
        if total as i64 != i64_of(a, "applications")? {
            return Err(format!("{total} applications, expected {}", scalar::show(a)));
        }
    }
    if let Some(first) = opt(op, "first") {
        let want = arr(first, "first")?;
        let got: Vec<Json> = per_tick.iter().take(want.len()).map(|&n| ji(n as i64)).collect();
        if !scalar::matches(&Json::Array(got.clone()), first, false) {
            return Err(format!(
                "per-tick applications {}, expected {}",
                scalar::show(&Json::Array(got)),
                scalar::show(first)
            ));
        }
    }
    if let Some(e) = opt(op, "every") {
        let k = i64_of(e, "every")? as usize;
        if per_tick.iter().any(|&n| n != k) {
            return Err(format!("per-tick applications {per_tick:?}, expected {k} each"));
        }
    }
    Ok(())
}

/// `ef_child`: labels (`as`) the child of the effect's built meta component
/// (the `n`-th, default 0); `name?` checks the child's name.
fn child(w: &World, op: &Json) -> R<()> {
    let id = w.label(op)?;
    let n = opt(op, "n").map_or(Ok(0), |v| i64_of(v, "n"))? as usize;
    let (child, name) = w.with(|m, _| match m.component_of_kind(id, component_type::META, n) {
        Some(Component::Meta(c)) => Ok((*c, m.effect(*c).map(|e| e.name).unwrap_or_default())),
        _ => Err("no meta component".into()),
    })?;
    if let Some(want) = opt(op, "name") {
        if name != str_of(want, "name")? {
            return Err(format!("child '{name}', expected {}", scalar::show(want)));
        }
    }
    if let Some(l) = opt(op, "as") {
        w.labels.borrow_mut().insert(str_of(l, "as")?.to_owned(), child);
    }
    Ok(())
}

fn expect(w: &World, op: &Json) -> R<()> {
    let id = w.label(op)?;
    let info = w.with(|m, _| Ok(m.effect(id)))?;
    let got = match info {
        None => vec![("exists".to_owned(), Json::Bool(false))],
        Some(e) => vec![
            ("exists".to_owned(), Json::Bool(true)),
            ("name".to_owned(), Json::Str(e.name)),
            ("index".to_owned(), ji(e.index)),
            ("refs".to_owned(), ji(e.refs)),
            ("started".to_owned(), Json::Bool(e.started)),
            ("stopping".to_owned(), Json::Bool(e.stopping)),
            ("components".to_owned(), ji(e.components as i64)),
            ("active".to_owned(), Json::Bool(e.active)),
        ],
    };
    check_state(&got, field(op, "state")?)
}

fn pair(p: (f32, f32)) -> Json {
    Json::Array(vec![jf(p.0), jf(p.1)])
}

fn expect_brush(w: &World, op: &Json) -> R<()> {
    let b = brush(w, op)?;
    let got = vec![
        ("brush".to_owned(), ji(b.record.brush_id)),
        ("rate".to_owned(), jf(b.record.rate)),
        ("length".to_owned(), jf(b.record.length)),
        ("one_shot".to_owned(), Json::Bool(b.record.one_shot)),
        ("strength".to_owned(), jf(b.strength)),
        ("radius".to_owned(), jf(b.radius)),
        ("active".to_owned(), Json::Bool(b.active)),
        ("long_op".to_owned(), Json::Bool(b.long_op)),
        ("acc".to_owned(), jf(b.acc)),
        ("t".to_owned(), jf(b.t)),
        ("prev".to_owned(), pair(b.prev)),
        ("cur".to_owned(), pair(b.cur)),
        ("applications".to_owned(), Json::Int(b.applications as i64)),
    ];
    check_state(&got, field(op, "state")?)
}

fn expect_game(w: &World, op: &Json) -> R<()> {
    let id = w.label(op)?;
    let n = opt(op, "n").map_or(Ok(0), |v| i64_of(v, "n"))? as usize;
    let g = w.with(|m, _| match m.component_of_kind(id, component_type::GAME, n) {
        Some(Component::Game(g)) => Ok(g.clone()),
        _ => Err("no game component".into()),
    })?;
    let got = vec![
        ("started".to_owned(), Json::Bool(g.started)),
        ("pending".to_owned(), Json::Bool(g.pending)),
        ("timer".to_owned(), jf(g.timer)),
        ("count".to_owned(), ji(g.count)),
        ("cell".to_owned(), Json::Array(vec![ji(g.cell.0), ji(g.cell.1)])),
        ("data".to_owned(), Json::Int(g.data as i64)),
        ("applies".to_owned(), ji(g.applies)),
    ];
    check_state(&got, field(op, "state")?)
}

fn expect_manager(w: &World, op: &Json) -> R<()> {
    let got = w.with(|m, _| {
        let s = m.stats();
        Ok(vec![
            ("select_draws".to_owned(), Json::Int(s.select_draws as i64)),
            ("end_posts".to_owned(), Json::Int(s.end_posts as i64)),
            ("recycled".to_owned(), Json::Int(s.recycled as i64)),
            ("effect_count".to_owned(), ji(m.effect_count() as i64)),
            ("active_count".to_owned(), ji(m.active_list().len() as i64)),
            ("total_us".to_owned(), Json::Int(m.total_time().0 as i64)),
            ("lod".to_owned(), ji(m.level_of_detail())),
            ("max_priority".to_owned(), ji(m.max_priority())),
        ])
    })?;
    check_state(&got, field(op, "state")?)
}

/// `ef_select`: a fresh select generator (seed `0x1105`): `first_word?`
/// (the first tempered word), `draws?` (the first `RandomUint32Uniform
/// (0x10000)` values), `members?` with `weights` (the 1-based members the
/// draws pick).
fn select(op: &Json) -> R<()> {
    let no_clock = FixedSeeds {
        wall_clock: 0,
        process_clock: 0,
    };
    if let Some(wd) = opt(op, "first_word") {
        let got = RzRandomSuper::new(SELECT_SEED, &no_clock).next_u32();
        if i64::from(got) != i64_of(wd, "first_word")? {
            return Err(format!("first word 0x{got:08X}, expected {}", scalar::show(wd)));
        }
    }
    let mut g = RzRandomSuper::new(SELECT_SEED, &no_clock);
    let n = opt(op, "draws").map_or(0, |d| d.as_array().map_or(0, |a| a.len()));
    let draws: Vec<u32> = (0..n.max(opt(op, "members").map_or(0, |m| m.as_array().map_or(0, |a| a.len()))))
        .map(|_| g.uniform_u32(SELECT_RANGE))
        .collect();
    if let Some(d) = opt(op, "draws") {
        let got = Json::Array(draws.iter().map(|&r| ji(r)).collect());
        if !scalar::matches(&got, d, false) {
            return Err(format!("draws {}, expected {}", scalar::show(&got), scalar::show(d)));
        }
    }
    if let Some(mm) = opt(op, "members") {
        let weights: Vec<u16> = arr(field(op, "weights")?, "weights")?
            .iter()
            .map(|v| u32_of(v, "weight").map(|w| w as u16))
            .collect::<R<_>>()?;
        let got = Json::Array(
            draws
                .iter()
                .map(|&r| shipped::select_member(&weights, r).map_or(Json::Null, |k| ji(k as i64 + 1)))
                .collect(),
        );
        if !scalar::matches(&got, mm, false) {
            return Err(format!("members {}, expected {}", scalar::show(&got), scalar::show(mm)));
        }
    }
    Ok(())
}

/// `ef_dilation` (§6, test 17): `GetAnimationTimeDilation` of a simulator
/// state with the shipped tunables, and optionally the dilated step.
fn dilation(op: &Json) -> R<()> {
    use fc4_core::effects::dilation::{animation_time_dilation, dilated_us, SimTimeState};
    let flag = |k: &str| opt(op, k).map_or(Ok(false), |v| bool_of(v, k));
    let s = SimTimeState {
        emergency_paused: flag("emergency_paused")?,
        driving: flag("driving")?,
        paused: flag("paused")?,
        hidden_paused: flag("hidden_paused")?,
        speed: opt(op, "speed").map_or(Ok(0), |v| u32_of(v, "speed"))?,
    };
    let d = animation_time_dilation(&s);
    let want = field(op, "expect")?;
    if !scalar::matches(&jf(d), want, false) {
        return Err(format!("dilation {d}, expected {}", scalar::show(want)));
    }
    if let Some(us) = opt(op, "us") {
        let got = dilated_us(u32_of(us, "us")?, Some(d));
        let want = field(op, "expect_us")?;
        if !scalar::matches(&ji(got as i64), want, false) {
            return Err(format!("dt' {got}, expected {}", scalar::show(want)));
        }
    }
    Ok(())
}

/// `ef_particles` (§7.1, §7.1.1, test 16): a particle component's simulator
/// run from `Start` with fixed updates; checks when it went inactive and,
/// optionally, how many particles it created and loops it counted.
fn particles(op: &Json) -> R<()> {
    use fc4_core::effects::particles::{EmitterDesc, ParticleSimulator};
    use fc4_core::rng::{FixedSeeds, RzRandom};
    let f = |k: &str, d: f32| opt(op, k).and_then(scalar::num).map_or(d, |v| v as f32);
    let u = |k: &str, d: u32| opt(op, k).map_or(Ok(d), |v| u32_of(v, k));
    let pair = |k: &str| -> R<(f32, f32)> {
        match opt(op, k) {
            None => Ok((-1.0, -1.0)),
            Some(v) => {
                let a = arr(v, k)?;
                let n = |i: usize| {
                    a.get(i)
                        .and_then(scalar::num)
                        .map(|x| x as f32)
                        .ok_or(format!("{k}[{i}]"))
                };
                Ok((n(0)?, n(1)?))
            }
        }
    };
    let mut d = EmitterDesc::new(u("flags", 0)?, f("count", 1.0), f("life", 1.0));
    d.flags_b = u("flags_b", 0)?;
    d.period = f("period", 1.0);
    d.loop_count = u("loop_count", 1)?;
    d.preroll = f("preroll", 0.0);
    // EF14: D+0x10 is both the life upper bound and the burst-age limit.
    d.max_age = f("max_age", d.max_age);
    d.start_delay = pair("start_delay")?;
    d.interval = pair("interval")?;
    d.rate_scale = f("rate_scale", 1.0);
    if let Some(c) = opt(op, "curve") {
        d.curve = arr(c, "curve")?
            .iter()
            .filter_map(scalar::num)
            .map(|x| x as f32)
            .collect();
    }
    if let Some(s) = opt(op, "scales") {
        let s: Vec<f32> = arr(s, "scales")?
            .iter()
            .filter_map(scalar::num)
            .map(|x| x as f32)
            .collect();
        d.scales = [s[0], s[1], s[2]];
    }
    let seeds = FixedSeeds {
        wall_clock: 0,
        process_clock: 7,
    };
    let mut r = RzRandom::new(u("seed", 0x1234)?, &seeds);
    let mut p = ParticleSimulator::new(d);
    p.start(u("start_arg", 0)?, &mut r);
    let dt = f("dt", 1.0 / 30.0);
    let max = u("max", 10_000)?;
    let stop_at = opt(op, "stop_after").map(|v| u32_of(v, "stop_after")).transpose()?;
    let stop_mode = u("stop_mode", 0)?;
    let mut ended: Option<u32> = None;
    for i in 1..=max {
        if stop_at == Some(i - 1) {
            p.stop(stop_mode);
            if !p.is_active() {
                ended = Some(i - 1);
                break;
            }
        }
        p.update(dt, &mut r);
        if !p.is_active() {
            ended = Some(i);
            break;
        }
    }
    let got = ended.map_or(Json::Null, |n| ji(i64::from(n)));
    let want = field(op, "expect_updates")?;
    if !scalar::matches(&got, want, false) {
        return Err(format!(
            "inactive after {}, expected {} (loops {}, live {}, created {})",
            scalar::show(&got),
            scalar::show(want),
            p.loops_emitted(),
            p.live_count(),
            p.particles_created()
        ));
    }
    for (k, v) in [
        ("expect_created", p.particles_created() as i64),
        ("expect_loops", i64::from(p.loops_emitted())),
    ] {
        if let Some(w) = opt(op, k) {
            if !scalar::matches(&ji(v), w, false) {
                return Err(format!("{k}: {v}, expected {}", scalar::show(w)));
            }
        }
    }
    Ok(())
}
