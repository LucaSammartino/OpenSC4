// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for mysims.md §4.7.1 (the dispatch job search, on a
//! recording pathfinder) and §4.12 (the Sim template pool, on the core's
//! in-memory pool host). See README.md ("MySim ops").

use std::cell::Cell;
use std::rc::Rc;

use fc4_core::grid::CellRect;
use fc4_core::mysims::dispatch::{
    set_current_commute_destination, DispatchLocation, JobSearchState, JobSearchWorld, LotAt, RecordingJobPathfinder,
    world_to_cell, FLAG_HAS_HOME,
};
use fc4_core::mysims::pool::{
    is_male_text, member_face, member_key, portrait_source, IniFile, MemoryPoolHost, MySimPool, MySimTemplate, PortraitSource, TemplateExemplar,
    TemplateKey, MALE_TEXT, ZODIAC_GROUP, ZODIAC_INSTANCES,
};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::scalar::{arr, bool_of, check_expect, i64_of, ji, opt, str_of, u32_of, R};

pub(crate) fn exec(rest: &str, op: &Json) -> Option<R<()>> {
    Some(match rest {
        "job_search" => job_search(op),
        "location_arithmetic" => location_arithmetic(op),
        "member_format" => member_format(op),
        "pool" => pool(op),
        "portrait" => portrait(op),
        "gender" => check_expect(
            op,
            Json::Bool(is_male_text(
                str_of(op.get("value")?, "value").ok()?.as_bytes(),
                str_of(op.get("male_text")?, "male_text").ok()?.as_bytes(),
            )),
        ),
        _ => return None,
    })
}

fn location_arithmetic(op: &Json) -> R<()> {
    let cells = arr(op.get("bits").ok_or("bits")?, "bits")?
        .iter()
        .map(|j| {
            let b = match j {
                Json::Str(s) => u32::from_str_radix(s, 16).map_err(|e| e.to_string()),
                _ => u32_of(j, "bits"),
            }?;
            Ok(ji(world_to_cell(f32::from_bits(b))))
        })
        .collect::<R<Vec<_>>>()?;
    let (x, z) = pair(op.get("cell").ok_or("cell")?, "cell")?;
    let mut loc = DispatchLocation {
        occupant: Some(77), by_commute: true, in_city: false,
        neighbor_city: (2, 9), reserved: (3, 4), ..DispatchLocation::default()
    };
    let before = loc;
    loc.set_cell_position(x, z);
    let preserved = loc.occupant == before.occupant && loc.by_commute == before.by_commute
        && loc.in_city == before.in_city && loc.neighbor_city == before.neighbor_city
        && loc.reserved == before.reserved;
    check_expect(op, obj(vec![
        ("cells", Json::Array(cells)),
        ("cell", Json::Array(vec![ji(loc.cell.0), ji(loc.cell.1)])),
        ("world", Json::Array(vec![Json::Float(f64::from(loc.world.0)), Json::Float(f64::from(loc.world.1))])),
        ("preserved", Json::Bool(preserved)),
    ]))
}

fn member_format(op: &Json) -> R<()> {
    let k = u32_of(op.get("index").ok_or("index")?, "index")?;
    check_expect(op, obj(vec![("key", Json::Str(member_key(k, "name"))), ("face", Json::Str(member_face(k)))]))
}

fn pair(j: &Json, what: &str) -> R<(i32, i32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("{what}: expected [x, z]"));
    }
    Ok((i64_of(&a[0], what)? as i32, i64_of(&a[1], what)? as i32))
}

/// `[x, z]` floats (an occupant's `GetPosition` x, z).
fn fpair(j: &Json, what: &str) -> R<(f32, f32)> {
    let a = arr(j, what)?;
    let f = |e: &Json| match e {
        Json::Float(v) => Ok(*v as f32),
        Json::Int(v) => Ok(*v as f32),
        _ => Err(format!("{what}: expected a number")),
    };
    match a {
        [x, z] => Ok((f(x)?, f(z)?)),
        _ => Err(format!("{what}: expected [x, z]")),
    }
}

fn rect(j: &Json, what: &str) -> R<CellRect> {
    let a = arr(j, what)?;
    if a.len() != 4 {
        return Err(format!("{what}: expected [x1, z1, x2, z2]"));
    }
    let v: Vec<i32> = a.iter().map(|e| i64_of(e, what).map(|n| n as i32)).collect::<R<_>>()?;
    Ok(CellRect::new(v[0], v[1], v[2], v[3]))
}

fn rect_json(r: CellRect) -> Json {
    Json::Array(vec![ji(r.x1), ji(r.z1), ji(r.x2), ji(r.z2)])
}

fn obj(kv: Vec<(&str, Json)>) -> Json {
    Json::Object(kv.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn text(b: &[u8]) -> Json {
    Json::Str(String::from_utf8_lossy(b).into_owned())
}

/// The world of one job-search case.
struct World {
    wealth: Cell<u32>,
    cells: (i32, i32),
    home_lot: Option<CellRect>,
    lot: Cell<LotAt>,
    city: Cell<Option<(i32, i32)>>,
    /// The found building's `GetPosition` x, z.
    end_position: Cell<(f32, f32)>,
    /// Occupant positions of `location` inputs.
    positions: std::cell::RefCell<Vec<(u64, (f32, f32))>>,
    /// The region city at a position and its connection types.
    region: Option<((i32, i32), Vec<u32>)>,
}

/// The opaque handle of the one region city of a case.
const REGION_CITY: u64 = 0xC1;

const HOME_OCCUPANT: u64 = 0x500;
const END_BUILDING: u64 = 0x900;

impl JobSearchWorld for World {
    fn wealth(&self) -> u32 {
        self.wealth.get()
    }
    fn occupant_lot_rect(&self, occupant: u64) -> Option<CellRect> {
        (occupant == HOME_OCCUPANT).then_some(self.home_lot).flatten()
    }
    fn lot_rect_at(&self, _x: i32, _z: i32) -> Option<CellRect> {
        None
    }
    fn cell_is_in_bounds(&self, x: i32, z: i32) -> bool {
        x >= 0 && z >= 0 && x < self.cells.0 && z < self.cells.1
    }
    fn cell_counts(&self) -> (i32, i32) {
        self.cells
    }
    fn lot_at(&self, _x: i32, _z: i32) -> LotAt {
        self.lot.get()
    }
    fn city_across_connection(&self, _x: i32, _z: i32) -> Option<(i32, i32)> {
        self.city.get()
    }
    fn occupant_position(&self, occupant: u64) -> (f32, f32) {
        if occupant == END_BUILDING {
            return self.end_position.get();
        }
        let p = self.positions.borrow();
        p.iter().find(|(o, _)| *o == occupant).map_or((0.0, 0.0), |(_, xz)| *xz)
    }
    fn region_city(&self, x: i32, z: i32) -> Option<u64> {
        self.region.as_ref().filter(|(p, _)| *p == (x, z)).map(|_| REGION_CITY)
    }
    fn connections_of_type_to_city(&self, connection_type: u32, city: u64) -> usize {
        self.region.as_ref().map_or(0, |(_, types)| {
            types
                .iter()
                .filter(|t| city == REGION_CITY && **t == connection_type)
                .count()
        })
    }
}

/// `mysim_job_search`: `cells?` ([64, 64]), `home` (`{"rect": [..]}` an
/// occupant home with that lot, `{"cell": [x, z]}` no lot, null = flag
/// `0x80` clear), `state?` (`+0x40`), `region?` (`{"city": [x, z],
/// "connections": [types]}`), `searches` (each `wealth`, `end_cell`,
/// `end_position?` (the found building's position, [0.0, 0.0]),
/// `find_result?` (1), `lot?` (`building` | `no_building` | `no_lot`),
/// `city?` ([x, z]), `notify?` (true), `location?` (`default`,
/// `{"occupant": n, "position": [x, z]}` = `SetByOccupant`, or
/// `{"neighbor_city": [x, z]}` = in-city flag clear)); one recording pathfinder for
/// all searches. Result: `results`, `notifies`, `dest_flags` (after each
/// search), `flags`, `filter`, `density`, `start_rect`, `travel`
/// ([allowed, starting]), `find_path` (calls), `calls`, `destination`.
fn job_search(op: &Json) -> R<()> {
    let cells = match opt(op, "cells") {
        Some(j) => pair(j, "cells")?,
        None => (64, 64),
    };
    let mut st = JobSearchState::default();
    let mut home_lot = None;
    if let Some(h) = opt(op, "home").filter(|h| !matches!(h, Json::Null)) {
        st.flags |= FLAG_HAS_HOME;
        if let Some(r) = h.get("rect") {
            home_lot = Some(rect(r, "home.rect")?);
            // The home's position is not read by the search (its lot is).
            st.home = DispatchLocation::by_occupant(Some((HOME_OCCUPANT, (0.0, 0.0))));
        } else {
            let (x, z) = pair(h.get("cell").ok_or("home: rect or cell")?, "home.cell")?;
            st.home = DispatchLocation::by_cell(x, z);
        }
    }
    if let Some(s) = opt(op, "state") {
        st.state = i64_of(s, "state")? as i32;
    }
    let w = World {
        wealth: Cell::new(0),
        cells,
        home_lot,
        lot: Cell::new(LotAt::Building(END_BUILDING)),
        city: Cell::new(None),
        end_position: Cell::new((0.0, 0.0)),
        positions: std::cell::RefCell::new(Vec::new()),
        region: match opt(op, "region") {
            Some(r) => Some((
                pair(r.get("city").ok_or("region.city")?, "region.city")?,
                arr(r.get("connections").ok_or("region.connections")?, "region.connections")?
                    .iter()
                    .map(|t| u32_of(t, "region.connections"))
                    .collect::<R<Vec<u32>>>()?,
            )),
            None => None,
        },
    };
    let mut pf = RecordingJobPathfinder::default();
    let (mut results, mut notifies, mut dest) = (Vec::new(), Vec::new(), Vec::new());
    for s in arr(op.get("searches").ok_or("searches")?, "searches")? {
        w.wealth.set(match opt(s, "wealth") {
            Some(j) => u32_of(j, "wealth")?,
            None => 0,
        });
        pf.find_result = match opt(s, "find_result") {
            Some(j) => u32_of(j, "find_result")?,
            None => 1,
        };
        pf.end_cell = match opt(s, "end_cell") {
            Some(j) => pair(j, "end_cell")?,
            None => (0, 0),
        };
        w.lot.set(match opt(s, "lot").map(|j| str_of(j, "lot")).transpose()? {
            None | Some("building") => LotAt::Building(END_BUILDING),
            Some("no_building") => LotAt::NoBuilding,
            Some("no_lot") => LotAt::NoLot,
            Some(o) => return Err(format!("lot: unknown '{o}'")),
        });
        w.city.set(opt(s, "city").map(|j| pair(j, "city")).transpose()?);
        w.end_position.set(match opt(s, "end_position") {
            Some(j) => fpair(j, "end_position")?,
            None => (0.0, 0.0),
        });
        let notify = match opt(s, "notify") {
            Some(j) => bool_of(j, "notify")?,
            None => true,
        };
        let loc = match opt(s, "location") {
            Some(Json::Str(d)) if d == "default" => DispatchLocation::default(),
            Some(j) if j.get("occupant").is_some() => {
                let o = u32_of(j.get("occupant").ok_or("location.occupant")?, "location.occupant")? as u64;
                let pos = fpair(j.get("position").ok_or("location.position")?, "location.position")?;
                w.positions.borrow_mut().push((o, pos));
                DispatchLocation::by_occupant(Some((o, pos)))
            }
            Some(j) => DispatchLocation {
                in_city: false,
                neighbor_city: pair(
                    j.get("neighbor_city")
                        .ok_or("location: default, occupant or neighbor_city")?,
                    "location.neighbor_city",
                )?,
                ..DispatchLocation::default()
            },
            None => DispatchLocation {
                by_commute: true,
                ..DispatchLocation::default()
            },
        };
        let (ok, fire) = set_current_commute_destination(&mut st, &mut pf, &w, loc, notify);
        results.push(Json::Bool(ok));
        notifies.push(Json::Bool(fire));
        dest.push(ji(pf.dest_flags));
    }
    let d = st.commute_destination;
    let destination = obj(vec![
        ("occupant", d.occupant.map_or(Json::Null, |o| ji(o as i64))),
        (
            "world",
            Json::Array(vec![
                Json::Float(f64::from(d.world.0)),
                Json::Float(f64::from(d.world.1)),
            ]),
        ),
        ("cell", Json::Array(vec![ji(d.cell.0), ji(d.cell.1)])),
        (
            "neighbor_city",
            Json::Array(vec![ji(d.neighbor_city.0), ji(d.neighbor_city.1)]),
        ),
        ("in_city", Json::Bool(d.in_city)),
        ("by_commute", Json::Bool(d.by_commute)),
        ("reserved", Json::Array(vec![ji(d.reserved.0), ji(d.reserved.1)])),
    ]);
    let got = obj(vec![
        ("results", Json::Array(results)),
        ("notifies", Json::Array(notifies)),
        ("dest_flags", Json::Array(dest)),
        ("flags", ji(pf.flags)),
        ("filter", ji(pf.filter)),
        ("density", ji(pf.density)),
        ("start_rect", pf.start_rect.map_or(Json::Null, rect_json)),
        ("travel", Json::Array(vec![ji(pf.allowed), ji(pf.starting)])),
        (
            "find_path",
            ji(pf.calls.iter().filter(|c| **c == "FindPath").count() as i64),
        ),
        (
            "calls",
            Json::Array(pf.calls.iter().map(|c| Json::Str(c.to_string())).collect()),
        ),
        ("destination", destination),
        ("dispatch_flags", ji(st.flags)),
    ]);
    check_expect(op, got)
}

fn template_json(t: &MySimTemplate) -> Json {
    let mut fields = vec![
        ("name", text(&t.name)),
        ("male", Json::Bool(t.is_male)),
        ("sign", ji(t.sign)),
        ("bitmap", ji(t.bitmap_id)),
        ("portrait", text(&t.portrait)),
    ];
    // Absent exemplar properties (MS12 a): only shown when some are.
    if t.indeterminate != 0 {
        fields.push(("indeterminate", ji(t.indeterminate)));
    }
    obj(fields)
}

fn bytes(j: &Json, what: &str) -> R<Vec<u8>> {
    Ok(str_of(j, what)?.as_bytes().to_vec())
}

/// `mysim_pool`: `mysim_dir`, `dat_hex?` (an existing `MySims.dat`),
/// `builtins?` (each `type?`, `instance`, `exemplar_type?`, `bitmap?`,
/// `localized?`, `name?`, `male?`, `sign?`), `install?` (`registry`
/// {value: path}, `dirs` [paths], `male_text?`, `zodiac?` [12 names],
/// `families?` [{`export` dir, `ini` path, `entries` [[section, key,
/// value]]}]), `steps` (`get` | `import` | `find` | `drop_families`).
/// Result: `results` (templates / count / bool / null per step), `loaded`,
/// `cached`, `installation`, `file_hex`, `converted`, `deleted`.
fn pool(op: &Json) -> R<()> {
    let dir = bytes(op.get("mysim_dir").ok_or("mysim_dir")?, "mysim_dir")?;
    let h = Rc::new(MemoryPoolHost {
        dir: dir.clone(),
        ..MemoryPoolHost::default()
    });
    let dat_path = [dir.as_slice(), b"MySims.dat"].concat();
    if let Some(j) = opt(op, "dat_hex") {
        h.files
            .borrow_mut()
            .insert(dat_path.clone(), hex_decode(str_of(j, "dat_hex")?)?);
    }
    if let Some(list) = opt(op, "builtins") {
        let mut keys = Vec::new();
        for b in arr(list, "builtins")? {
            let num = |k: &str| opt(b, k).map(|j| u32_of(j, k)).transpose();
            let s = |k: &str| opt(b, k).map(|j| bytes(j, k)).transpose();
            let instance = num("instance")?.ok_or("builtin instance")?;
            keys.push(TemplateKey {
                type_id: num("type")?.unwrap_or(fc4_core::mysims::pool::TEMPLATE_KEY_TYPE),
                group: 0,
                instance,
            });
            let ex = TemplateExemplar {
                exemplar_type: num("exemplar_type")?,
                bitmap_id: num("bitmap")?,
                localized_name: s("localized")?,
                name: s("name")?,
                is_male: opt(b, "male").map(|j| bool_of(j, "male")).transpose()?,
                sign: num("sign")?.map(|v| v as u8),
            };
            h.exemplars.borrow_mut().insert(instance, ex);
        }
        *h.keys.borrow_mut() = Some(keys);
    }
    if let Some(inst) = opt(op, "install") {
        if let Some(Json::Object(kv)) = inst.get("registry") {
            for (k, v) in kv {
                h.registry
                    .borrow_mut()
                    .insert(k.as_bytes().to_vec(), bytes(v, "registry")?);
            }
        }
        if let Some(d) = inst.get("dirs") {
            for p in arr(d, "dirs")? {
                h.dirs.borrow_mut().push(bytes(p, "dirs")?);
            }
        }
        h.converter.set(true);
        if let Some(m) = inst.get("male_text") {
            h.strings.borrow_mut().insert(MALE_TEXT, bytes(m, "male_text")?);
        }
        if let Some(z) = inst.get("zodiac") {
            for (i, n) in arr(z, "zodiac")?.iter().enumerate().take(12) {
                h.strings
                    .borrow_mut()
                    .insert((ZODIAC_GROUP, ZODIAC_INSTANCES[i]), bytes(n, "zodiac")?);
            }
        }
        if let Some(f) = inst.get("families") {
            for fam in arr(f, "families")? {
                let export = bytes(fam.get("export").ok_or("family export")?, "export")?;
                let ini_path = bytes(fam.get("ini").ok_or("family ini")?, "ini")?;
                let pub_name = ini_path
                    .rsplit(|b| *b == b'\\')
                    .next()
                    .map(|n| [n.strip_suffix(b".ini").unwrap_or(n), b".pub"].concat())
                    .unwrap_or_default();
                let pub_name = match opt(fam, "listed_name") {
                    Some(j) => bytes(j, "listed_name")?,
                    None => pub_name,
                };
                h.listings.borrow_mut().entry(export).or_default().push(pub_name);
                let mut ini = IniFile::default();
                for e in arr(fam.get("entries").ok_or("family entries")?, "entries")? {
                    let e = arr(e, "entry")?;
                    if e.len() != 3 {
                        return Err("entry: [section, key, value]".into());
                    }
                    ini.entries
                        .push((bytes(&e[0], "section")?, bytes(&e[1], "key")?, bytes(&e[2], "value")?));
                }
                h.inis.borrow_mut().insert(ini_path, ini);
            }
        }
    }
    let pool = MySimPool::new(h.clone());
    let mut results = Vec::new();
    for s in arr(op.get("steps").ok_or("steps")?, "steps")? {
        results.push(match str_of(s, "step")? {
            "get" => Json::Array(pool.templates().iter().map(template_json).collect()),
            "import" => ji(pool.import()),
            "find" => Json::Bool(pool.find_the_sims_installation()),
            "drop_families" => {
                h.listings.borrow_mut().clear();
                Json::Null
            }
            o => return Err(format!("unknown pool step '{o}'")),
        });
    }
    let file = h
        .files
        .borrow()
        .get(&dat_path)
        .map_or(Json::Null, |b| Json::Str(hex_encode(b)));
    let pairs =
        |v: &[(Vec<u8>, Vec<u8>)]| Json::Array(v.iter().map(|(a, b)| Json::Array(vec![text(a), text(b)])).collect());
    let got = obj(vec![
        ("results", Json::Array(results)),
        ("loaded", Json::Bool(pool.is_loaded())),
        ("cached", ji(pool.cached().len() as i64)),
        ("installation", text(&pool.installation())),
        ("file_hex", file),
        ("converted", pairs(&h.converted.borrow())),
        (
            "deleted",
            Json::Array(h.deleted.borrow().iter().map(|d| text(d)).collect()),
        ),
    ]);
    check_expect(op, got)
}

/// `mysim_portrait`: `bitmap`, `portrait`, `mysim_dir` → `{"resource": id}`
/// or `{"file": path}` (`PickHead`).
fn portrait(op: &Json) -> R<()> {
    let t = MySimTemplate {
        bitmap_id: u32_of(op.get("bitmap").ok_or("bitmap")?, "bitmap")?,
        portrait: bytes(op.get("portrait").ok_or("portrait")?, "portrait")?,
        ..MySimTemplate::default()
    };
    let dir = bytes(op.get("mysim_dir").ok_or("mysim_dir")?, "mysim_dir")?;
    let got = match portrait_source(&t, &dir) {
        PortraitSource::Resource(id) => obj(vec![("resource", ji(id))]),
        PortraitSource::File(p) => obj(vec![("file", text(&p))]),
    };
    check_expect(op, got)
}
