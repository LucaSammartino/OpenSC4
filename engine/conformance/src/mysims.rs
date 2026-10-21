// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/mysims.md`: the scripted MySim host of
//! the core (`fc4_core::mysims::scripted::ScriptedMySimHost`) next to the
//! advisor world of `advisors.rs` (the MySims are advisors), and ops that
//! drive the agent simulator and the MySims. See README.md ("MySim ops").

use std::cell::RefCell;
use std::rc::Rc;

use fc4_core::advisors::{clsid as adv_clsid, Advisor, AdvisorEntry};
use fc4_core::mysims::scripted::ScriptedMySimHost;
use fc4_core::mysims::{
    clsid, register, Building, Dispatch, Field, Grid, JobDestination, Lot, MySim, MySimAgent, MySimState, MY_STREET_ID,
};
use fc4_core::rng::{FixedSeeds, RzRandom};
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, ResKey, SaveObject, SaveRegistry};

use crate::advisors::{self, jstr, message_of, subject_json, MsPart, Obj, World};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, R};

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("mysim_")?;
    if let Some(r) = crate::mysims_pool::exec(rest, op) {
        return Some(r);
    }
    let w = advisors::world(host);
    Some(match rest {
        "create" => create(&w, op),
        "env" => with_h(&w, |h| env(h, op)),
        "building" => with_h(&w, |h| building(h, op)),
        "position" => with_h(&w, |h| position(h, op)),
        "rect_lot" => with_h(&w, |h| rect_lot(h, op)),
        "dispatch" => with_h(&w, |h| dispatch(h, op)),
        "call" => call(&w, op),
        "simulate" => simulate(&w, op),
        "message" => message(&w, op),
        "set" => set(&w, op),
        "reseed" => reseed(&w, op),
        "expect" => expect(&w, op),
        "expect_subject" => expect_subject(&w, op),
        "expect_fields" => expect_fields(&w, op),
        "agent" => agent(&w, op),
        "expect_agent" => expect_agent(&w, op),
        "write" => write(&w, op),
        "read" => read(&w, op),
        "expect_record" => expect_record(&w, op),
        "compose" => compose(&w, op),
        "expect_refs" => with_h(&w, |h| expect_refs(h, op)),
        "run" => run(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

fn with_h(w: &World, f: impl FnOnce(&ScriptedMySimHost) -> R<()>) -> R<()> {
    let ms = w.ms.borrow();
    let ms = ms.as_ref().ok_or("no MySims (mysim_create)")?;
    f(&ms.h)
}

fn host(w: &World) -> R<Rc<ScriptedMySimHost>> {
    w.ms.borrow()
        .as_ref()
        .map(|m| m.h.clone())
        .ok_or_else(|| "no MySims (mysim_create)".into())
}

fn agent_of(w: &World) -> R<Rc<MySimAgent>> {
    w.ms.borrow()
        .as_ref()
        .map(|m| m.agent.clone())
        .ok_or_else(|| "no MySims (mysim_create)".into())
}

/// The MySim advisor at vector index `sim`, or the named advisor `sim`.
fn sim_adv(w: &World, op: &Json) -> R<Rc<Advisor>> {
    let j = field(op, "sim")?;
    if let Some(n) = j.as_str().filter(|n| Json::Str((*n).to_owned()).as_i64().is_none()) {
        return match w.named.borrow().get(n) {
            Some(Obj::Advisor(a)) => Ok(a.clone()),
            _ => Err(format!("no advisor named '{n}'")),
        };
    }
    let i = i64_of(j, "sim")? as usize;
    agent_of(w)?.sim(i).ok_or_else(|| format!("no MySim at index {i}"))
}

fn with_sim<T>(adv: &Rc<Advisor>, f: impl FnOnce(&MySim) -> T) -> R<T> {
    MySim::with(adv, f).ok_or_else(|| "not a MySim".into())
}

// ---- parsing --------------------------------------------------------------------------------------------

fn tok(j: &Json, what: &str) -> R<u64> {
    Ok(i64_of(j, what)? as u64)
}

fn opt_tok(op: &Json, k: &str) -> R<Option<u64>> {
    opt(op, k).map(|v| tok(v, k)).transpose()
}

fn cell_of(j: &Json, what: &str) -> R<(i32, i32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be [x, z]"));
    }
    Ok((i64_of(&a[0], what)? as i32, i64_of(&a[1], what)? as i32))
}

fn pos_of(j: &Json, what: &str) -> R<[f32; 3]> {
    let a = arr(j, what)?;
    if a.len() != 3 {
        return Err(format!("'{what}' must be [x, y, z]"));
    }
    Ok([f32_of(&a[0], what)?, f32_of(&a[1], what)?, f32_of(&a[2], what)?])
}

fn u32s(j: &Json, what: &str) -> R<Vec<u32>> {
    arr(j, what)?.iter().map(|x| u32_of(x, what)).collect()
}

fn toks(j: &Json, what: &str) -> R<Vec<u64>> {
    arr(j, what)?.iter().map(|x| tok(x, what)).collect()
}

fn grid_of(j: &Json) -> R<Grid> {
    let s = str_of(j, "grid")?;
    Ok(match s {
        "crime" => Grid::Crime,
        "air" => Grid::Pollution(0),
        "water" => Grid::Pollution(1),
        "garbage" => Grid::Pollution(2),
        "radiation" => Grid::Pollution(3),
        "aura" => Grid::Aura,
        "average_eq" => Grid::AverageEq,
        "average_hq" => Grid::AverageHq,
        "congestion" => Grid::Congestion,
        _ => {
            let d = s
                .strip_prefix("desirability:")
                .ok_or_else(|| format!("unknown grid '{s}'"))?;
            Grid::Desirability(u32_of(&Json::Str(d.to_owned()), "grid")?)
        }
    })
}

fn key_json(k: &ResKey) -> Json {
    Json::Array(vec![ji(k.type_id), ji(k.group), ji(k.instance)])
}

fn key_of(j: &Json) -> R<ResKey> {
    let a = u32s(j, "key")?;
    if a.len() != 3 {
        return Err("a key is [type, group, instance]".into());
    }
    Ok(ResKey {
        type_id: a[0],
        group: a[1],
        instance: a[2],
    })
}

fn f32s6(j: &Json, what: &str) -> R<[f32; 6]> {
    let a = arr(j, what)?;
    if a.len() != 6 {
        return Err(format!("'{what}' needs 6 values"));
    }
    let mut r = [0.0f32; 6];
    for (i, x) in a.iter().enumerate() {
        r[i] = f32_of(x, what)?;
    }
    Ok(r)
}

fn tok_json(t: Option<u64>) -> Json {
    t.map_or(Json::Null, |v| ji(v as i64))
}

fn field_json(f: &Field) -> Json {
    match f {
        Field::Num(n) => Json::Float(*n),
        Field::Str(s) => jstr(s),
        Field::Building(b) => Json::Object(vec![("building".into(), tok_json(*b))]),
        Field::Subject(s) => Json::Object(vec![(
            "subject".into(),
            s.as_ref().map_or(Json::Null, |x| subject_json(&x.state())),
        )]),
    }
}

// ---- setup ----------------------------------------------------------------------------------------------

fn shipped_entry(class_id: u32, id: i32) -> AdvisorEntry {
    AdvisorEntry {
        class_id,
        id,
        caption: b"MySim".to_vec(),
        name: format!("Sim{id}").into_bytes(),
        advice_type: 1,
    }
}

/// `mysim_create`: the advisors table (`table`: "shipped" (default: a plain
/// advisor 23, MySims 1–5, the My Street sim 30), or `[{class, id}]`), a new
/// advisor system with the MySim classes registered, `Init` (unless `init:
/// false`), and the agent's `Init`. The logs are cleared.
fn create(w: &World, op: &Json) -> R<()> {
    let table = match opt(op, "table") {
        None => None,
        Some(Json::Str(s)) if s == "shipped" => None,
        Some(t) => Some(t),
    };
    let entries = match table {
        None => {
            let mut v = vec![shipped_entry(adv_clsid::ADVISOR, 23)];
            for id in 1..=5 {
                v.push(shipped_entry(clsid::MYSIM, id));
            }
            v.push(shipped_entry(clsid::MY_STREET_SIM, MY_STREET_ID));
            v
        }
        Some(t) => arr(t, "table")?
            .iter()
            .map(|e| {
                Ok(shipped_entry(
                    advisors::class_of(field(e, "class")?)?,
                    i64_of(field(e, "id")?, "id")? as i32,
                ))
            })
            .collect::<R<_>>()?,
    };
    *w.h.advisors.borrow_mut() = Some(entries);
    if let Some(v) = opt(op, "session_seed") {
        w.seed.set(u32_of(v, "session_seed")?);
    }
    let h = ScriptedMySimHost::new();
    let sys = w.new_sys();
    register(&sys, h.clone(), w.seed.get());
    let agent = MySimAgent::new(h.clone());
    *w.ms.borrow_mut() = Some(MsPart {
        h: h.clone(),
        agent: agent.clone(),
    });
    if opt(op, "init").map_or(Ok(true), |v| bool_of(v, "init"))? {
        if !sys.init() {
            return Err("advisor system Init failed".into());
        }
        if !agent.init(&sys) {
            return Err("agent Init failed".into());
        }
    }
    w.sync();
    w.events.clear();
    Ok(())
}

/// `mysim_env`: the scripted host's answers (see README).
fn env(h: &ScriptedMySimHost, op: &Json) -> R<()> {
    let o32 = |k: &str| opt(op, k).map(|v| f32_of(v, k)).transpose();
    let ou8 = |k: &str| opt(op, k).map(|v| u32_of(v, k).map(|x| x as u8)).transpose();
    if let Some(v) = opt(op, "cells") {
        let (x, z) = cell_of(v, "cells")?;
        h.cells.set((x as u32, z as u32));
    }
    for (k, c) in [
        ("ui_blocked", &h.ui_blocked),
        ("disaster_ongoing", &h.disaster_ongoing),
        ("no_lua", &h.no_lua),
        ("no_query", &h.no_query),
        ("change_job", &h.change_job),
        ("no_trip_simulators", &h.no_trip_simulators),
    ] {
        if let Some(v) = opt(op, k) {
            c.set(bool_of(v, k)?);
        }
    }
    for (k, c) in [
        ("eq", &h.eq),
        ("hq", &h.hq),
        ("travel_ratio", &h.travel_ratio),
        ("commute", &h.commute),
    ] {
        if op.get(k).is_some() {
            c.set(o32(k)?);
        }
    }
    if op.get("life_expectancy").is_some() {
        h.life_expectancy.set(ou8("life_expectancy")?);
    }
    if op.get("new_age").is_some() {
        h.new_age.set(ou8("new_age")?);
    }
    if let Some(v) = opt(op, "disaster_locations") {
        let mut l = Vec::new();
        for e in arr(v, "disaster_locations")? {
            let a = arr(e, "disaster_locations")?;
            l.push((u32_of(&a[0], "type")?, cell_of(&a[1], "cell")?));
        }
        *h.disaster_locations.borrow_mut() = l;
    }
    if let Some(v) = opt(op, "tuning") {
        for (k, x) in v.as_object().ok_or("tuning must be an object")? {
            let n = crate::scalar::num(x).ok_or("tuning values are numbers")?;
            h.tuning.borrow_mut().insert(k.clone(), n);
        }
    }
    if let Some(v) = opt(op, "average_age") {
        for e in arr(v, "average_age")? {
            let a = arr(e, "average_age")?;
            let c = (i64_of(&a[0], "x")? as i32, i64_of(&a[1], "z")? as i32);
            h.average_age.borrow_mut().insert(c, i64_of(&a[2], "age")? as i32);
        }
    }
    if let Some(v) = opt(op, "grids") {
        for e in arr(v, "grids")? {
            let a = arr(e, "grids")?;
            let g = grid_of(&a[0])?;
            let (x, z) = (i64_of(&a[1], "x")? as i32, i64_of(&a[2], "z")? as i32);
            h.grids.borrow_mut().insert((g, x, z), i64_of(&a[3], "value")? as i32);
        }
    }
    if let Some(v) = opt(op, "grids_at") {
        for (k, x) in v.as_object().ok_or("grids_at must be an object")? {
            h.grids_at
                .borrow_mut()
                .insert(grid_of(&Json::Str(k.clone()))?, i64_of(x, k)? as i32);
        }
    }
    if let Some(v) = opt(op, "coverage") {
        for (k, x) in v.as_object().ok_or("coverage must be an object")? {
            h.coverage.borrow_mut().insert(k == "police", i64_of(x, k)? as i32);
        }
    }
    for (k, m) in [("powered", &h.powered), ("watered", &h.watered)] {
        if let Some(v) = opt(op, k) {
            for (t, x) in v.as_object().ok_or("an object of tokens")? {
                m.borrow_mut().insert(tok(&Json::Str(t.clone()), k)?, bool_of(x, k)?);
            }
        }
    }
    if let Some(v) = opt(op, "stations") {
        for (k, x) in v.as_object().ok_or("stations must be an object")? {
            let mut l = Vec::new();
            for e in arr(x, "stations")? {
                let a = arr(e, "stations")?;
                l.push((tok(&a[0], "station")?, f32_of(&a[1], "funding")?));
            }
            h.stations.borrow_mut().insert(k == "police", l);
        }
    }
    if let Some(v) = opt(op, "find_job") {
        for x in arr(v, "find_job")? {
            h.find_job.borrow_mut().push_back(bool_of(x, "find_job")?);
        }
    }
    if let Some(v) = op.get("job_destination") {
        *h.job_destination.borrow_mut() = match v {
            Json::Null => None,
            _ => Some(destination_of(v)?),
        };
    }
    if let Some(v) = opt(op, "neighbor_names") {
        for e in arr(v, "neighbor_names")? {
            let a = arr(e, "neighbor_names")?;
            let c = (i64_of(&a[0], "x")? as i32, i64_of(&a[1], "z")? as i32);
            h.neighbor_names
                .borrow_mut()
                .insert(c, str_of(&a[2], "name")?.as_bytes().to_vec());
        }
    }
    for (k, l) in [
        ("special", &h.special),
        ("schools", &h.schools),
        ("hospitals", &h.hospitals),
    ] {
        if let Some(v) = opt(op, k) {
            *l.borrow_mut() = toks(v, k)?;
        }
    }
    for (k, m) in [("grades", &h.grades), ("landmark_funding", &h.landmark_funding)] {
        if let Some(v) = opt(op, k) {
            for (t, x) in v.as_object().ok_or("an object of tokens")? {
                m.borrow_mut().insert(tok(&Json::Str(t.clone()), k)?, f32_of(x, k)?);
            }
        }
    }
    Ok(())
}

/// A job destination: `{building, cell}` or `{neighbor: [x, z]}`.
fn destination_of(v: &Json) -> R<JobDestination> {
    Ok(match opt(v, "neighbor") {
        Some(c) => JobDestination::Neighbor {
            cell: cell_of(c, "neighbor")?,
        },
        None => JobDestination::Lot {
            building: tok(field(v, "building")?, "building")?,
            cell: opt(v, "cell").map_or(Ok((0, 0)), |c| cell_of(c, "cell"))?,
        },
    })
}

fn lot_of(j: &Json) -> R<Lot> {
    let cell = cell_of(field(j, "cell")?, "cell")?;
    let rect = match opt(j, "rect") {
        Some(r) => {
            let a = arr(r, "rect")?;
            let g = |i: usize| i64_of(&a[i], "rect").map(|x| x as i32);
            (g(0)?, g(1)?, g(2)?, g(3)?)
        }
        None => (cell.0, cell.1, cell.0, cell.1),
    };
    Ok(Lot {
        building: opt_tok(j, "building")?,
        cell,
        rect,
        condition: opt(j, "condition").map_or(Ok(1), |v| u32_of(v, "condition"))? as u8,
        road_access: opt(j, "road_access").map_or(Ok(true), |v| bool_of(v, "road_access"))?,
        wealth: opt(j, "wealth").map_or(Ok(2), |v| u32_of(v, "wealth"))? as u8,
    })
}

/// `mysim_building`: `token`, `type?` (0x10), `name?` ("House"),
/// `groups?` ([0x1000]), `completion?` (1.0), `purpose?` (1: a residence),
/// `position` ([x, y, z]), `lot?` (`{cell, rect?, condition? 1,
/// road_access? true, wealth? 2, building?}`); `remove: true` removes it.
fn building(h: &ScriptedMySimHost, op: &Json) -> R<()> {
    let t = tok(field(op, "token")?, "token")?;
    if opt(op, "remove").map_or(Ok(false), |v| bool_of(v, "remove"))? {
        h.buildings.borrow_mut().remove(&t);
        h.lots.borrow_mut().remove(&t);
        return Ok(());
    }
    let b = Building {
        building_type: opt(op, "type").map_or(Ok(0x10), |v| u32_of(v, "type"))?,
        name: opt(op, "name").map_or(Ok(b"House".to_vec()), |v| {
            str_of(v, "name").map(|s| s.as_bytes().to_vec())
        })?,
        groups: opt(op, "groups").map_or(Ok(vec![0x1000]), |v| u32s(v, "groups"))?,
        completion: opt(op, "completion").map_or(Ok(1.0), |v| f32_of(v, "completion"))?,
        purpose: opt(op, "purpose").map_or(Ok(1), |v| u32_of(v, "purpose"))?,
        position: opt(op, "position").map_or(Ok([0.0; 3]), |v| pos_of(v, "position"))?,
    };
    let lot = opt(op, "lot").map(lot_of).transpose()?;
    h.add_building(t, b, lot);
    Ok(())
}

/// `mysim_position`: a non-building occupant (a fire) at `position`.
fn position(h: &ScriptedMySimHost, op: &Json) -> R<()> {
    h.positions.borrow_mut().insert(
        tok(field(op, "occupant")?, "occupant")?,
        pos_of(field(op, "position")?, "position")?,
    );
    Ok(())
}

/// `mysim_rect_lot`: a lot `lots_in_rect` reports (as `mysim_building`'s
/// `lot`).
fn rect_lot(h: &ScriptedMySimHost, op: &Json) -> R<()> {
    h.rect_lots.borrow_mut().push(lot_of(op)?);
    Ok(())
}

/// `mysim_dispatch`: the dispatch of MySim advisor `id` (null `state`:
/// none).
fn dispatch(h: &ScriptedMySimHost, op: &Json) -> R<()> {
    let id = i64_of(field(op, "id")?, "id")? as i32;
    if op.get("state") == Some(&Json::Null) {
        h.dispatch.borrow_mut().remove(&id);
        return Ok(());
    }
    let b = |k: &str| opt(op, k).map_or(Ok(false), |v| bool_of(v, k));
    let d = Dispatch {
        state: opt(op, "state").map_or(Ok(0), |v| i64_of(v, "state"))? as i32,
        flags: opt(op, "flags").map_or(Ok(0), |v| u32_of(v, "flags"))?,
        cell: opt(op, "cell").map_or(Ok((0, 0)), |v| cell_of(v, "cell"))?,
        position: opt(op, "position").map_or(Ok([0.0; 3]), |v| pos_of(v, "position"))?,
        travel_type: opt(op, "travel_type").map_or(Ok(0xFF), |v| u32_of(v, "travel_type"))?,
        wait_minutes: opt(op, "wait_minutes").map_or(Ok(0), |v| u32_of(v, "wait_minutes"))?,
        destination: opt_tok(op, "destination")?,
        location: opt_tok(op, "location")?,
        at_home: b("at_home")?,
        at_work: b("at_work")?,
        by_commute: b("by_commute")?,
    };
    h.dispatch.borrow_mut().insert(id, d);
    Ok(())
}

// ---- calls ----------------------------------------------------------------------------------------------

fn bytes(op: &Json, k: &str, d: &[u8]) -> R<Vec<u8>> {
    opt(op, k).map_or(Ok(d.to_vec()), |v| str_of(v, k).map(|s| s.as_bytes().to_vec()))
}

fn u(op: &Json, k: &str, d: u32) -> R<u32> {
    opt(op, k).map_or(Ok(d), |v| u32_of(v, k))
}

fn b(op: &Json, k: &str, d: bool) -> R<bool> {
    opt(op, k).map_or(Ok(d), |v| bool_of(v, k))
}

/// `mysim_call`: `method` on MySim `sim` (an index) with named arguments
/// (README); `expect` checks the result.
fn call(w: &World, op: &Json) -> R<()> {
    let adv = sim_adv(w, op)?;
    let method = str_of(field(op, "method")?, "method")?.to_owned();
    let building = || tok(field(op, "building")?, "building");
    let xz = |k: &str| -> R<(f32, f32)> {
        let a = arr(field(op, k)?, k)?;
        Ok((f32_of(&a[0], k)?, f32_of(&a[1], k)?))
    };
    let got: Json = match method.as_str() {
        "move_in" => {
            let (slot, name) = (u(op, "slot", 0)? as u8, bytes(op, "name", b"Ann")?);
            let (male, sign) = (b(op, "is_male", false)?, u(op, "sign", 3)? as u8);
            let (bio, bid, path) = (
                bytes(op, "bio", b"bio")?,
                u(op, "bitmap_id", 7)?,
                bytes(op, "bitmap_path", b"p.png")?,
            );
            with_sim(&adv, |m| m.move_in(slot, &name, male, sign, &bio, bid, &path))?;
            Json::Null
        }
        "set_residence" => {
            let r = opt_tok(op, "building")?;
            Json::Bool(with_sim(&adv, |m| m.set_residence(r))?)
        }
        "change_residence" => {
            let r = building()?;
            Json::Bool(with_sim(&adv, |m| m.change_residence(r))?)
        }
        "find_new_residence" => Json::Bool(with_sim(&adv, |m| m.find_new_residence())?),
        "test_for_residence" => {
            let r = building()?;
            let (f, mw, lh) = (
                b(op, "filter", false)?,
                b(op, "match_wealth", false)?,
                b(op, "lives_here", false)?,
            );
            Json::Bool(with_sim(&adv, |m| m.test_for_residence(r, f, mw, lh))?)
        }
        "change_job" => {
            let r = building()?;
            Json::Bool(with_sim(&adv, |m| m.change_job(r))?)
        }
        "find_new_job" => Json::Bool(with_sim(&adv, |m| m.find_new_job())?),
        "job_found" => {
            let d = destination_of(op)?;
            with_sim(&adv, |m| m.job_found(d))?;
            Json::Null
        }
        "check_commute" => unit(with_sim(&adv, |m| m.check_commute())?),
        "fail_commute" => unit(with_sim(&adv, |m| m.fail_commute())?),
        "quit_job" => unit(with_sim(&adv, |m| m.quit_job())?),
        "neighbor_connection_removed" => unit(with_sim(&adv, |m| m.neighbor_connection_removed())?),
        "move_out" => Json::Bool(with_sim(&adv, |m| m.move_out())?),
        "leave_city" => unit(with_sim(&adv, |m| m.leave_city())?),
        "delete_from_game" => Json::Bool(with_sim(&adv, |m| m.delete_from_game())?),
        "announce_move_in" => unit(with_sim(&adv, |m| m.announce_move_in())?),
        "update_variables" => unit(with_sim(&adv, |m| m.update_variables())?),
        "update_quick_variables" => unit(with_sim(&adv, |m| m.update_quick_variables())?),
        "update_local_buildings" => unit(with_sim(&adv, |m| m.update_local_buildings())?),
        "do_script_command" => {
            let c = u32_of(field(op, "command")?, "command")?;
            unit(with_sim(&adv, |m| m.do_script_command(c))?)
        }
        "distance_to_closest" => {
            let g = u32_of(field(op, "group")?, "group")?;
            ji(with_sim(&adv, |m| m.distance_to_closest(g))?)
        }
        "distance_to_closest_special" => {
            let t = u32_of(field(op, "type")?, "type")?;
            ji(with_sim(&adv, |m| m.distance_to_closest_special(t))?)
        }
        "local_grade" => {
            let g = u32_of(field(op, "group")?, "group")?;
            let hosp = b(op, "hospital", false)?;
            jf(with_sim(&adv, |m| m.local_grade(g, hosp))?)
        }
        "trip_area" => {
            let r = arr(field(op, "rect")?, "rect")?;
            let g = |i: usize| i64_of(&r[i], "rect").map(|x| x as i32);
            let (x0, z0, x1, z1) = (g(0)?, g(1)?, g(2)?, g(3)?);
            unit(with_sim(&adv, |m| m.trip_area(x0, z0, x1, z1))?)
        }
        "update_trip_variables" => {
            let g = |k: &str| opt(op, k).map_or(Ok(-1), |v| i64_of(v, k).map(|x| x as i32));
            let (ns, os, nf, of) = (g("new_state")?, g("old_state")?, g("new_flags")?, g("old_flags")?);
            let force = b(op, "force", false)?;
            unit(with_sim(&adv, |m| m.update_trip_variables(ns, os, nf, of, force))?)
        }
        "disaster" => {
            let t = u32_of(field(op, "type")?, "type")?;
            let (cx, cz) = cell_of(field(op, "cell")?, "cell")?;
            unit(with_sim(&adv, |m| m.disaster(t, cx, cz))?)
        }
        "building_added" => {
            let r = building()?;
            unit(with_sim(&adv, |m| m.building_added(r))?)
        }
        "building_removed" => {
            let r = building()?;
            let cause = u(op, "cause", 0)?;
            unit(with_sim(&adv, |m| m.building_removed(r, cause))?)
        }
        "fire_added" | "pipe_burst" | "tornado" => {
            let (x, z) = xz("position")?;
            with_sim(&adv, |m| match method.as_str() {
                "fire_added" => m.fire_added(x, z),
                "pipe_burst" => m.pipe_burst(x, z),
                _ => m.tornado(x, z),
            })?;
            Json::Null
        }
        "command_would_trigger" => {
            let c = u32_of(field(op, "command")?, "command")?;
            Json::Bool(adv.command_would_trigger(c))
        }
        "init" => {
            let i = adv.state().table_index;
            Json::Bool(adv.init(i))
        }
        other => return Err(format!("unknown MySim method '{other}'")),
    };
    check_expect(op, got)
}

fn unit(_: ()) -> Json {
    Json::Null
}

/// `mysim_simulate`: sets the date (`day?`, a day number), the advisor's
/// month counter (`month_counter?`, before the step), then `Simulate(kind)`
/// on MySim `sim`.
fn simulate(w: &World, op: &Json) -> R<()> {
    let adv = sim_adv(w, op)?;
    if let Some(d) = opt(op, "day") {
        w.h.set_day(i64_of(d, "day")? as i32);
    }
    if let Some(m) = opt(op, "month_counter") {
        let m = i64_of(m, "month_counter")? as i32;
        adv.with_state(|s| s.month_counter = m);
    }
    let k = i64_of(field(op, "kind")?, "kind")? as i32;
    check_expect(op, Json::Bool(adv.simulate(k)))
}

/// `mysim_message`: to the agent (default) or to MySim `sim`'s
/// `DoMessage`.
fn message(w: &World, op: &Json) -> R<()> {
    let m = message_of(op)?;
    let got = match opt(op, "sim") {
        Some(_) => sim_adv(w, op)?.do_message(&m),
        None => agent_of(w)?.do_message(&m),
    };
    check_expect(op, Json::Bool(got))
}

/// `mysim_set`: pokes MySim `sim`'s fields (README).
fn set(w: &World, op: &Json) -> R<()> {
    let adv = sim_adv(w, op)?;
    if let Some(v) = opt(op, "month_counter") {
        let m = i64_of(v, "month_counter")? as i32;
        adv.with_state(|s| s.month_counter = m);
    }
    if opt(op, "queue").is_some() {
        adv.with_state(|s| s.queue.clear());
    }
    let mut s = with_sim(&adv, |m| m.state())?;
    let g8 = |k: &str| opt(op, k).map(|v| u32_of(v, k).map(|x| x as u8)).transpose();
    if let Some(v) = g8("state")? {
        s.state = v;
    }
    if let Some(v) = g8("slot")? {
        s.slot = v;
    }
    if let Some(v) = g8("wealth")? {
        s.wealth = v;
    }
    if let Some(v) = g8("age")? {
        s.age = v;
    }
    if let Some(v) = g8("birth_month")? {
        s.birth_month = v;
    }
    if let Some(v) = g8("sign")? {
        s.sign = v;
    }
    if let Some(v) = g8("eq_index")? {
        s.eq_index = v;
    }
    if let Some(v) = g8("hq_index")? {
        s.hq_index = v;
    }
    if let Some(v) = opt(op, "generation") {
        s.generation = u32_of(v, "generation")? as u16;
    }
    for (k, f) in [
        ("retired", &mut s.retired as &mut bool),
        ("is_male", &mut s.is_male),
        ("unnamed_flag", &mut s.unnamed_flag),
        ("commute_check", &mut s.commute_check),
    ] {
        if let Some(v) = opt(op, k) {
            *f = bool_of(v, k)?;
        }
    }
    for (k, f) in [
        ("jobless_days", &mut s.jobless_days as &mut u32),
        ("last_local_disaster", &mut s.last_local_disaster),
        ("last_commute_time", &mut s.last_commute_time),
        ("bitmap_id", &mut s.bitmap_id),
    ] {
        if let Some(v) = opt(op, k) {
            *f = u32_of(v, k)?;
        }
    }
    if let Some(v) = opt(op, "last_player_trip_day") {
        s.last_player_trip_day = i64_of(v, "last_player_trip_day")? as i32;
    }
    for (k, f) in [
        ("neighbor_cell", &mut s.neighbor_cell as &mut (i32, i32)),
        ("unnamed_pair", &mut s.unnamed_pair),
    ] {
        if let Some(v) = opt(op, k) {
            *f = cell_of(v, k)?;
        }
    }
    for (k, f) in [
        ("vehicle_key", &mut s.vehicle_key as &mut ResKey),
        ("car_key", &mut s.car_key),
        ("pedestrian_key", &mut s.pedestrian_key),
    ] {
        if let Some(v) = opt(op, k) {
            *f = key_of(v)?;
        }
    }
    for (k, f) in [("eq_delta", &mut s.eq_delta as &mut f32), ("hq_delta", &mut s.hq_delta)] {
        if let Some(v) = opt(op, k) {
            *f = f32_of(v, k)?;
        }
    }
    for (k, f) in [
        ("eq_ring", &mut s.eq_ring as &mut [f32; 6]),
        ("hq_ring", &mut s.hq_ring),
    ] {
        if let Some(v) = opt(op, k) {
            *f = f32s6(v, k)?;
        }
    }
    for (k, f) in [
        ("dispatch", &mut s.dispatch as &mut Option<u64>),
        ("dispatch_tool", &mut s.dispatch_tool),
    ] {
        if let Some(v) = op.get(k) {
            *f = match v {
                Json::Null => None,
                _ => Some(tok(v, k)?),
            };
        }
    }
    if let Some(v) = opt(op, "pylon") {
        let a = arr(v, "pylon")?;
        s.pylon = (
            u32_of(&a[0], "pylon")? as u8,
            tok(&a[1], "pylon")?,
            u32_of(&a[2], "pylon")? as u8,
        );
    }
    if let Some(v) = opt(op, "bio") {
        s.bio = str_of(v, "bio")?.as_bytes().to_vec();
    }
    with_sim(&adv, |m| m.with_state(|st| *st = s))
}

/// `mysim_reseed`: MySim `sim`'s generator restarts from `seed` (fixed
/// clock seeds 1).
fn reseed(w: &World, op: &Json) -> R<()> {
    let adv = sim_adv(w, op)?;
    let seed = u32_of(field(op, "seed")?, "seed")?;
    with_sim(&adv, |m| {
        m.with_random(|g| {
            *g = RzRandom::new(
                seed,
                &FixedSeeds {
                    wall_clock: 1,
                    process_clock: 1,
                },
            )
        })
    })
}

// ---- checks ---------------------------------------------------------------------------------------------

fn mysim_state(adv: &Rc<Advisor>, m: &MySim) -> Vec<(String, Json)> {
    let s: MySimState = m.state();
    let f6 = |r: &[f32; 6]| Json::Array(r.iter().map(|v| jf(*v)).collect());
    let pair = |p: (i32, i32)| Json::Array(vec![ji(p.0), ji(p.1)]);
    let st = adv.state();
    vec![
        ("id".into(), ji(adv.id())),
        ("class".into(), ji(SaveObject::class_id(&**adv))),
        ("personal_name".into(), jstr(&adv.personal_name())),
        ("month_counter".into(), ji(st.month_counter)),
        (
            "events".into(),
            Json::Array(st.queue.iter().map(|(e, _)| ji(*e)).collect()),
        ),
        ("lcg".into(), ji(m.with_random(|g| g.lcg_state()))),
        ("active".into(), Json::Bool(m.is_active())),
        ("state".into(), ji(s.state)),
        ("slot".into(), ji(s.slot)),
        ("residence".into(), tok_json(s.residence)),
        (
            "home_cell".into(),
            Json::Array(vec![ji(s.home_cell.0), ji(s.home_cell.1)]),
        ),
        ("job".into(), tok_json(s.job)),
        ("neighbor_job".into(), Json::Bool(s.neighbor_job)),
        ("neighbor_cell".into(), pair(s.neighbor_cell)),
        ("commute_cell".into(), pair(s.commute_cell)),
        ("jobless_days".into(), ji(s.jobless_days)),
        ("vehicle_key".into(), key_json(&s.vehicle_key)),
        ("wealth".into(), ji(s.wealth)),
        ("generation".into(), ji(s.generation)),
        ("sign".into(), ji(s.sign)),
        ("is_male".into(), Json::Bool(s.is_male)),
        ("bio".into(), jstr(&s.bio)),
        ("age".into(), ji(s.age)),
        ("birth_month".into(), ji(s.birth_month)),
        ("bitmap_id".into(), ji(s.bitmap_id)),
        ("bitmap_path".into(), jstr(&s.bitmap_path)),
        ("last_commute_time".into(), ji(s.last_commute_time)),
        ("retired".into(), Json::Bool(s.retired)),
        ("last_local_disaster".into(), ji(s.last_local_disaster)),
        (
            "home_position".into(),
            Json::Array(s.home_position.iter().map(|v| jf(*v)).collect()),
        ),
        ("eq".into(), jf(s.eq)),
        ("hq".into(), jf(s.hq)),
        ("eq_index".into(), ji(s.eq_index)),
        ("eq_delta".into(), jf(s.eq_delta)),
        ("eq_ring".into(), f6(&s.eq_ring)),
        ("hq_index".into(), ji(s.hq_index)),
        ("hq_delta".into(), jf(s.hq_delta)),
        ("hq_ring".into(), f6(&s.hq_ring)),
        ("car_key".into(), key_json(&s.car_key)),
        ("pedestrian_key".into(), key_json(&s.pedestrian_key)),
        ("unnamed_pair".into(), pair(s.unnamed_pair)),
        ("dispatch".into(), tok_json(s.dispatch)),
        (
            "pylon".into(),
            Json::Array(vec![ji(s.pylon.0), ji(s.pylon.1 as i64), ji(s.pylon.2)]),
        ),
        ("dispatch_tool".into(), tok_json(s.dispatch_tool)),
        ("unnamed_flag".into(), Json::Bool(s.unnamed_flag)),
        ("last_player_trip_day".into(), ji(s.last_player_trip_day)),
        ("commute_check".into(), Json::Bool(s.commute_check)),
        ("last_trip_update_day".into(), ji(s.last_trip_update_day)),
        (
            "local".into(),
            Json::Object(s.local.iter().map(|(k, v)| (format!("0x{k:X}"), ji(*v))).collect()),
        ),
        ("lua_table".into(), Json::Bool(s.lua_table)),
        ("subscribed".into(), Json::Bool(s.subscribed)),
        ("local_area_size".into(), ji(s.local_area_size)),
        (
            "tuning".into(),
            Json::Object(s.tuning.iter().map(|(k, v)| (k.clone(), Json::Float(*v))).collect()),
        ),
    ]
}

/// `mysim_expect`: MySim `sim`'s partial `state`.
fn expect(w: &World, op: &Json) -> R<()> {
    let adv = sim_adv(w, op)?;
    let st = with_sim(&adv, |m| mysim_state(&adv, m))?;
    check_state(&st, field(op, "state")?)
}

/// `mysim_expect_subject`: the subject of the first queued `event` of
/// MySim `sim` (partial `state`; null: no subject).
fn expect_subject(w: &World, op: &Json) -> R<()> {
    let adv = sim_adv(w, op)?;
    let ev = u32_of(field(op, "event")?, "event")?;
    let q = adv.state().queue;
    let (_, s) = q
        .iter()
        .find(|(e, _)| *e == ev)
        .ok_or_else(|| format!("event 0x{ev:X} not queued"))?;
    let want = field(op, "state")?;
    match s {
        None if *want == Json::Null => Ok(()),
        None => Err("no subject".into()),
        Some(s) => {
            let Json::Object(kv) = subject_json(&s.state()) else {
                unreachable!()
            };
            check_state(&kv, want)
        }
    }
}

/// `mysim_expect_fields`: MySim `sim`'s Lua fields (`{"name": value}`; null:
/// never written).
fn expect_fields(w: &World, op: &Json) -> R<()> {
    let adv = sim_adv(w, op)?;
    let h = host(w)?;
    let id = adv.id();
    let mut errors = Vec::new();
    for (k, v) in field(op, "fields")?.as_object().ok_or("fields must be an object")? {
        let got = h.field(id, k).map_or(Json::Null, |f| field_json(&f));
        if !crate::scalar::matches(&got, v, false) {
            errors.push(format!(
                "{k}: got {}, expected {}",
                crate::scalar::show(&got),
                crate::scalar::show(v)
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// `mysim_agent`: the agent's `method`: `init`, `shutdown`, `count`,
/// `active` (advisor IDs), `move_in` (as `mysim_call`; the advisor ID or
/// null), `lives_here` (`building`), `set_active_ui_panel_sim` (`slot`),
/// `get_active_ui_panel_sim` (`index` in, default −7; `{sim, index}` out),
/// `write` (`name`), `read` (`hex`).
fn agent(w: &World, op: &Json) -> R<()> {
    let a = agent_of(w)?;
    let id = |x: Option<Rc<Advisor>>| x.map_or(Json::Null, |a| ji(a.id()));
    let got = match str_of(field(op, "method")?, "method")? {
        "init" => Json::Bool(a.init(&w.sys()?)),
        "shutdown" => Json::Bool(a.shutdown()),
        "count" => ji(a.count() as i64),
        "active" => Json::Array(a.get_active_mysims().iter().map(|x| ji(x.id())).collect()),
        "move_in" => id(a.move_in(
            &bytes(op, "name", b"Ann")?,
            b(op, "is_male", false)?,
            u(op, "sign", 3)? as u8,
            &bytes(op, "bio", b"bio")?,
            u(op, "bitmap_id", 7)?,
            &bytes(op, "bitmap_path", b"p.png")?,
            u(op, "slot", 0)? as u8,
        )),
        "lives_here" => id(a.my_sim_lives_here(tok(field(op, "building")?, "building")?)),
        "set_active_ui_panel_sim" => {
            a.set_active_ui_panel_sim(i64_of(field(op, "slot")?, "slot")? as i32);
            Json::Null
        }
        "get_active_ui_panel_sim" => {
            let mut i = opt(op, "index").map_or(Ok(-7), |v| i64_of(v, "index"))? as i32;
            let r = a.get_active_ui_panel_sim(&mut i);
            Json::Object(vec![("sim".into(), id(r)), ("index".into(), ji(i))])
        }
        "write" => {
            let mut out = ByteWriter::new();
            let ok = a.write(&mut out);
            w.records
                .borrow_mut()
                .insert(str_of(field(op, "name")?, "name")?.to_owned(), out.into_bytes());
            Json::Bool(ok)
        }
        "read" => {
            let bytes = advisors::record_input(w, op)?;
            Json::Bool(a.read(&mut ByteReader::new(&bytes)))
        }
        other => return Err(format!("unknown agent method '{other}'")),
    };
    check_expect(op, got)
}

/// `mysim_expect_agent`: partial `state`: `initialised`, `slots` (advisor
/// IDs or null), `last_disaster_type`, `active_ui_index`, `count`.
fn expect_agent(w: &World, op: &Json) -> R<()> {
    let a = agent_of(w)?;
    let s = a.state();
    check_state(
        &[
            ("initialised".into(), Json::Bool(s.initialised)),
            (
                "slots".into(),
                Json::Array(s.slots.iter().map(|x| x.map_or(Json::Null, ji)).collect()),
            ),
            ("last_disaster_type".into(), ji(s.last_disaster_type)),
            ("active_ui_index".into(), ji(s.active_ui_index)),
            ("count".into(), ji(a.count() as i64)),
        ],
        field(op, "state")?,
    )
}

// ---- records --------------------------------------------------------------------------------------------

/// References of a MySim record resolve to host objects whose token is
/// `base + object ID`.
struct Ids(u64);

impl RefResolver for Ids {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: u64::from(object_id) + self.0,
            class_id,
        })
    }
}

/// `mysim_write`: MySim `sim`'s record (references numbered from 1) as
/// `name`; with `advisor_part: true`, only the advisor part
/// (`WriteRecord`).
fn write(w: &World, op: &Json) -> R<()> {
    let adv = sim_adv(w, op)?;
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(reg);
    let ok = if b(op, "advisor_part", false)? {
        adv.write_record(&mut out);
        true
    } else {
        SaveObject::write(&*adv, &mut out)
    };
    w.records
        .borrow_mut()
        .insert(str_of(field(op, "name")?, "name")?.to_owned(), out.into_bytes());
    check_expect(op, Json::Bool(ok))
}

/// `mysim_read`: reads `hex` (or record `record`) into MySim `sim`, or into
/// a new MySim of class `new` named `as`; references resolve to host tokens
/// `token_base` (0x1000) + ID.
fn read(w: &World, op: &Json) -> R<()> {
    let bytes = advisors::record_input(w, op)?;
    let adv = match opt(op, "new") {
        Some(c) => {
            let a = w
                .sys()?
                .create_advisor(advisors::class_of(c)?)
                .ok_or("cannot create that class")?;
            w.named
                .borrow_mut()
                .insert(str_of(field(op, "as")?, "as")?.to_owned(), Obj::Advisor(a.clone()));
            a
        }
        None => sim_adv(w, op)?,
    };
    let mut bytes = bytes;
    if let Some(p) = opt(op, "patch") {
        let a = arr(p, "patch")?;
        let at = i64_of(&a[0], "patch")? as usize;
        let hx = crate::foundations::hex_decode(str_of(&a[1], "patch")?)?;
        if at + hx.len() > bytes.len() {
            return Err("patch beyond the record".into());
        }
        bytes[at..at + hx.len()].copy_from_slice(&hx);
    }
    let base = opt(op, "token_base").map_or(Ok(0x1000), |v| tok(v, "token_base"))?;
    let mut ids = Ids(base);
    let ok = SaveObject::read(&*adv, &mut ByteReader::with_resolver(&bytes, &mut ids));
    check_expect(op, Json::Bool(ok))
}

/// `mysim_expect_record`: record `name` equals `hex` followed by the record
/// `then` (when given: for example the advisor part).
fn expect_record(w: &World, op: &Json) -> R<()> {
    let mut want = crate::foundations::hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    if let Some(t) = opt(op, "then") {
        let r = w
            .records
            .borrow()
            .get(str_of(t, "then")?)
            .cloned()
            .ok_or("no such record")?;
        want.extend(r);
    }
    let name = str_of(field(op, "name")?, "name")?;
    let got = w
        .records
        .borrow()
        .get(name)
        .cloned()
        .ok_or_else(|| format!("no record '{name}'"))?;
    let got = if opt(op, "prefix").map_or(Ok(false), |v| bool_of(v, "prefix"))? {
        got[..want.len().min(got.len())].to_vec()
    } else {
        got
    };
    if got != want {
        return Err(format!(
            "record differs:\n    got      {}\n    expected {}",
            crate::foundations::hex_encode(&got),
            crate::foundations::hex_encode(&want)
        ));
    }
    Ok(())
}

/// `mysim_compose`: record `name` from `parts`: `{"hex": …}` or
/// `{"record": name, "from"?: offset, "to"?: offset}`.
fn compose(w: &World, op: &Json) -> R<()> {
    let mut out = Vec::new();
    for p in arr(field(op, "parts")?, "parts")? {
        if let Some(h) = opt(p, "hex") {
            out.extend(crate::foundations::hex_decode(str_of(h, "hex")?)?);
            continue;
        }
        let n = str_of(field(p, "record")?, "record")?;
        let r = w
            .records
            .borrow()
            .get(n)
            .cloned()
            .ok_or_else(|| format!("no record '{n}'"))?;
        let from = opt(p, "from").map_or(Ok(0), |v| i64_of(v, "from"))? as usize;
        let to = opt(p, "to").map_or(Ok(r.len() as i64), |v| i64_of(v, "to"))? as usize;
        out.extend(r.get(from..to).ok_or("range beyond the record")?);
    }
    w.records
        .borrow_mut()
        .insert(str_of(field(op, "name")?, "name")?.to_owned(), out);
    Ok(())
}

/// `mysim_expect_refs`: `refs` (`{"token": count}`) the MySims hold.
fn expect_refs(h: &ScriptedMySimHost, op: &Json) -> R<()> {
    let mut errors = Vec::new();
    for (k, v) in field(op, "refs")?.as_object().ok_or("refs must be an object")? {
        let t = tok(&Json::Str(k.clone()), "token")?;
        let got = i64::from(h.refs.borrow().get(&t).copied().unwrap_or(0));
        if got != i64_of(v, k)? {
            errors.push(format!("{k}: {got}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("references differ: {}", errors.join(", ")))
    }
}

/// `mysim_run`: `months` months of `days` (28) days from `start_day`: each
/// day sets the date and runs `Simulate(0)` of MySims 0–4, preceded on the
/// last day of a month by `Simulate(1)`.
fn run(w: &World, op: &Json) -> R<()> {
    let a = agent_of(w)?;
    let months = i64_of(field(op, "months")?, "months")? as i32;
    let days = opt(op, "days").map_or(Ok(28), |v| i64_of(v, "days"))? as i32;
    let start = i64_of(field(op, "start_day")?, "start_day")? as i32;
    for month in 0..months {
        for d in 0..days {
            w.h.set_day(start + month * days + d);
            for i in 0..5 {
                if let Some(adv) = a.sim(i) {
                    if d == days - 1 {
                        adv.simulate(1);
                    }
                    adv.simulate(0);
                }
            }
        }
    }
    Ok(())
}
