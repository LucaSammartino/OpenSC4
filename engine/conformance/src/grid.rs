// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/simgrid.md`: named core grids of any
//! element type, the grid slots, the record, the shared map utilities and
//! the cell iterators. See README.md ("Tract grid ops").

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use fc4_core::grid::{
    add_distance_effect, add_radius_effect, create_for_clsid, process_two_grids, table_distance, CellIterator,
    CityDims, GridElem, IteratorState, ScanIterator, SimGrid, TractGrid,
};
use fc4_core::rng::CrtRand;
use fc4_core::stream::{ByteReader, RefValue, SaveObject};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{self, arr, bool_of, check_expect, f32_of, i64_of, jf, ji, num, opt, show, str_of, u32_of, R};

/// A core grid of any element type.
#[derive(Clone)]
pub(crate) enum AnyGrid {
    U8(Rc<SimGrid<u8>>),
    I8(Rc<SimGrid<i8>>),
    U16(Rc<SimGrid<u16>>),
    I16(Rc<SimGrid<i16>>),
    U32(Rc<SimGrid<u32>>),
    I32(Rc<SimGrid<i32>>),
    U64(Rc<SimGrid<u64>>),
    I64(Rc<SimGrid<i64>>),
    F32(Rc<SimGrid<f32>>),
    F64(Rc<SimGrid<f64>>),
}

macro_rules! each {
    ($g:expr, $v:ident => $e:expr) => {
        match $g {
            AnyGrid::U8($v) => $e,
            AnyGrid::I8($v) => $e,
            AnyGrid::U16($v) => $e,
            AnyGrid::I16($v) => $e,
            AnyGrid::U32($v) => $e,
            AnyGrid::I32($v) => $e,
            AnyGrid::U64($v) => $e,
            AnyGrid::I64($v) => $e,
            AnyGrid::F32($v) => $e,
            AnyGrid::F64($v) => $e,
        }
    };
}

impl AnyGrid {
    fn of_type(t: &str) -> R<AnyGrid> {
        Ok(match t {
            "u8" => AnyGrid::U8(SimGrid::new()),
            "i8" => AnyGrid::I8(SimGrid::new()),
            "u16" => AnyGrid::U16(SimGrid::new()),
            "i16" => AnyGrid::I16(SimGrid::new()),
            "u32" => AnyGrid::U32(SimGrid::new()),
            "i32" => AnyGrid::I32(SimGrid::new()),
            "u64" => AnyGrid::U64(SimGrid::new()),
            "i64" => AnyGrid::I64(SimGrid::new()),
            "f32" => AnyGrid::F32(SimGrid::new()),
            "f64" => AnyGrid::F64(SimGrid::new()),
            _ => return Err(format!("unknown grid type '{t}'")),
        })
    }

    /// Wraps a store-created object (factory or load).
    pub(crate) fn of_object(o: Rc<dyn SaveObject>) -> Option<AnyGrid> {
        let a = o.as_any_rc();
        macro_rules! try_as {
            ($t:ty, $v:ident) => {
                if let Ok(g) = a.clone().downcast::<SimGrid<$t>>() {
                    return Some(AnyGrid::$v(g));
                }
            };
        }
        try_as!(u8, U8);
        try_as!(i8, I8);
        try_as!(u16, U16);
        try_as!(i16, I16);
        try_as!(u32, U32);
        try_as!(i32, I32);
        try_as!(u64, U64);
        try_as!(i64, I64);
        try_as!(f32, F32);
        try_as!(f64, F64);
        None
    }

    fn object(&self) -> Rc<dyn SaveObject> {
        each!(self, g => g.clone() as Rc<dyn SaveObject>)
    }

    fn type_name(&self) -> &'static str {
        fn n<T: GridElem>(_: &Rc<SimGrid<T>>) -> &'static str {
            T::NAME
        }
        each!(self, g => n(g))
    }
}

fn to_json<T: GridElem>(v: T) -> Json {
    if T::FLOAT {
        Json::Float(v.to_f64())
    } else {
        Json::Int(v.to_f64() as i64)
    }
}

fn of_json<T: GridElem>(j: &Json) -> R<T> {
    if let (false, Some(i)) = (T::FLOAT, j.as_i64()) {
        // Exact for 64-bit values.
        return Ok(if T::NAME == "u64" {
            T::from_f64(i as u64 as f64)
        } else {
            T::from_f64(i as f64)
        });
    }
    num(j)
        .map(T::from_f64)
        .ok_or_else(|| format!("bad grid value {}", show(j)))
}

#[derive(Default)]
pub(crate) struct World {
    city: Cell<Option<CityDims>>,
    grids: RefCell<BTreeMap<String, AnyGrid>>,
}

impl World {
    fn new() -> Rc<World> {
        Rc::new(World {
            city: Cell::new(Some(CityDims::square(256))),
            ..Default::default()
        })
    }

    fn city(&self) -> CityDims {
        self.city.get().unwrap_or(CityDims::square(256))
    }

    fn grid(&self, op: &Json, key: &str) -> R<AnyGrid> {
        let name = str_of(field(op, key)?, key)?;
        self.grids
            .borrow()
            .get(name)
            .cloned()
            .ok_or_else(|| format!("no grid '{name}' (tg_create)"))
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "tract_grid", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("tg_")?;
    let w = world(host);
    Some(match rest {
        "city" => city(&w, op),
        "create" => create(&w, op),
        "call" => w.grid(op, "grid").and_then(|g| call(&w, &g, op)),
        "set" => w.grid(op, "grid").and_then(|g| each!(&g, g => set(g, op))),
        "expect" => w.grid(op, "grid").and_then(|g| each!(&g, g => expect(g, op))),
        "record" => w.grid(op, "grid").and_then(|g| record(&g, op)),
        "read" => w.grid(op, "grid").and_then(|g| read(&g, op)),
        "store_round_trip" => w.grid(op, "grid").and_then(|g| store_round_trip(&g, op)),
        "radius_effect" => w.grid(op, "grid").and_then(|g| radius(&g, op)),
        "distance_effect" => distance(&w, op),
        "two_grids" => two_grids(&w, op),
        "table_distance" => (|| {
            let d = table_distance(i64_of(field(op, "dx")?, "dx")?, i64_of(field(op, "dz")?, "dz")?);
            check_expect(op, jf(d))
        })(),
        "scan" => scan(op),
        "random_iterator" => random_iterator(op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

fn pair(op: &Json, key: &str) -> R<(i64, i64)> {
    let a = arr(field(op, key)?, key)?;
    if a.len() != 2 {
        return Err(format!("'{key}' must have 2 elements"));
    }
    Ok((i64_of(&a[0], key)?, i64_of(&a[1], key)?))
}

fn city(w: &World, op: &Json) -> R<()> {
    let mut c = w.city();
    if let Some(v) = opt(op, "cells") {
        let (x, z) = pair(op, "cells").map_err(|e| format!("{e} ({})", show(v)))?;
        c.cell_count_x = x as i32;
        c.cell_count_z = z as i32;
    }
    if let Some(v) = opt(op, "width") {
        let a = arr(v, "width")?;
        c.cell_width_x = f32_of(&a[0], "width")?;
        c.cell_width_z = f32_of(a.get(1).unwrap_or(&a[0]), "width")?;
    }
    w.city.set(Some(c));
    Ok(())
}

/// `grid_create`: by `type`, or by factory `clsid` (§2); optionally
/// `tract_size` / `instance` / `init` as the original simulators do.
fn create(w: &World, op: &Json) -> R<()> {
    let name = str_of(field(op, "name")?, "name")?.to_owned();
    let g = match opt(op, "clsid") {
        Some(c) => match create_for_clsid(u32_of(c, "clsid")?) {
            Some(o) => AnyGrid::of_object(o).ok_or("factory made a non-grid")?,
            None => {
                return check_expect(op, Json::Bool(false));
            }
        },
        None => AnyGrid::of_type(str_of(field(op, "type")?, "type")?)?,
    };
    if let Some(t) = opt(op, "expect_type") {
        if g.type_name() != str_of(t, "expect_type")? {
            return Err(format!("created a {} grid", g.type_name()));
        }
    }
    let city = w.city();
    each!(&g, s => {
        if opt(op, "init").map_or(Ok(false), |v| bool_of(v, "init"))? {
            s.init();
        }
        if let Some(t) = opt(op, "tract_size") {
            s.set_tract_size(&city, i64_of(t, "tract_size")? as i32);
        }
        if let Some(i) = opt(op, "instance") {
            s.set_instance_id(u32_of(i, "instance")?);
        }
    });
    w.grids.borrow_mut().insert(name, g);
    check_expect(op, Json::Bool(true))
}

fn int_arg(op: &Json, i: usize) -> R<i32> {
    i64_of(scalar::arg(op, i)?, "argument").map(|v| v as i32)
}

fn call(w: &World, g: &AnyGrid, op: &Json) -> R<()> {
    let m = str_of(field(op, "method")?, "method")?;
    let city = w.city();
    let got = each!(g, s => call_typed(s, &city, m, op)?);
    check_expect(op, got)
}

fn call_typed<T: GridElem>(g: &Rc<SimGrid<T>>, city: &CityDims, m: &str, op: &Json) -> R<Json> {
    let geo = g.geometry();
    let a = |i| int_arg(op, i);
    Ok(match m {
        "init" => Json::Bool(g.init()),
        "shutdown" => Json::Bool(g.shutdown()),
        "is_init" => Json::Bool(g.data.borrow().init),
        "instance_id" => ji(g.instance_id()),
        "set_instance_id" => {
            g.set_instance_id(u32_of(scalar::arg(op, 0)?, "instance")?);
            Json::Null
        }
        "clsid" => ji(g.class_id()),
        "set_tract_size" => Json::Bool(g.set_tract_size(city, a(0)?)),
        "tract_size" => ji(geo.size),
        "shift" => ji(geo.shift),
        "count_x" => ji(geo.count_x),
        "count_z" => ji(geo.count_z),
        "width_x" => jf(geo.width_x),
        "width_z" => jf(geo.width_z),
        "inv_width_x" => jf(geo.inv_width_x),
        "inv_width_z" => jf(geo.inv_width_z),
        "geometry" => Json::Object(vec![
            ("size".into(), ji(geo.size)),
            ("shift".into(), ji(geo.shift)),
            ("count_x".into(), ji(geo.count_x)),
            ("count_z".into(), ji(geo.count_z)),
            ("width_x".into(), jf(geo.width_x)),
            ("width_z".into(), jf(geo.width_z)),
            ("inv_width_x".into(), jf(geo.inv_width_x)),
            ("inv_width_z".into(), jf(geo.inv_width_z)),
        ]),
        "tract_in_bounds" => Json::Bool(g.tract_in_bounds(a(0)?, a(1)?)),
        "position_to_tract" => {
            let (x, z) = geo.position_to_tract(f32_of(scalar::arg(op, 0)?, "px")?, f32_of(scalar::arg(op, 1)?, "pz")?);
            Json::Array(vec![ji(x), ji(z)])
        }
        "tract_corner" => {
            let (x, z) = geo.tract_corner(a(0)?, a(1)?);
            Json::Array(vec![jf(x), jf(z)])
        }
        "tract_center" => {
            let (x, z) = geo.tract_center(a(0)?, a(1)?);
            Json::Array(vec![jf(x), jf(z)])
        }
        "get_cell_value" => to_json(g.get_cell_value(a(0)?, a(1)?)),
        "get_tract_value" => to_json(g.get(a(0)?, a(1)?)),
        "set_tract_value" => {
            g.set(a(0)?, a(1)?, of_json(scalar::arg(op, 2)?)?);
            Json::Null
        }
        "set_tract_values" => {
            g.fill(of_json(scalar::arg(op, 0)?)?);
            Json::Null
        }
        "average_in_cell_rect" => to_json(g.average_in_cell_rect(a(0)?, a(1)?, a(2)?, a(3)?)),
        "average_in_tract_rect" => to_json(g.average_in_tract_rect(a(0)?, a(1)?, a(2)?, a(3)?)),
        _ => return Err(format!("unknown grid method '{m}'")),
    })
}

/// `grid_set`: `cells` [[x, z, v], …] and/or `fill` v (applied first) and/or
/// `rows` [[z0, z1, …] per x].
fn set<T: GridElem>(g: &Rc<SimGrid<T>>, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "fill") {
        g.fill(of_json(v)?);
    }
    if let Some(rows) = opt(op, "rows") {
        for (x, row) in arr(rows, "rows")?.iter().enumerate() {
            for (z, v) in arr(row, "rows")?.iter().enumerate() {
                g.set(x as i32, z as i32, of_json(v)?);
            }
        }
    }
    if let Some(cells) = opt(op, "cells") {
        for c in arr(cells, "cells")? {
            let c = arr(c, "cell")?;
            if c.len() != 3 {
                return Err("a cell is [x, z, value]".into());
            }
            g.set(i64_of(&c[0], "x")? as i32, i64_of(&c[1], "z")? as i32, of_json(&c[2])?);
        }
    }
    Ok(())
}

/// `grid_expect`: `cells` [[x, z, v], …]; `others` v = every other tract;
/// `rows` = the whole grid, x outer; `counts` [countX, countZ].
fn expect<T: GridElem>(g: &Rc<SimGrid<T>>, op: &Json) -> R<()> {
    let geo = g.geometry();
    let mut errors = Vec::new();
    if let Some(c) = opt(op, "counts") {
        let (x, z) = pair(op, "counts").map_err(|e| format!("{e} ({})", show(c)))?;
        if (i64::from(geo.count_x), i64::from(geo.count_z)) != (x, z) {
            errors.push(format!(
                "counts ({}, {}), expected ({x}, {z})",
                geo.count_x, geo.count_z
            ));
        }
    }
    let mut listed = std::collections::BTreeSet::new();
    if let Some(cells) = opt(op, "cells") {
        for c in arr(cells, "cells")? {
            let c = arr(c, "cell")?;
            if c.len() != 3 {
                return Err("a cell is [x, z, value]".into());
            }
            let (x, z) = (i64_of(&c[0], "x")? as i32, i64_of(&c[1], "z")? as i32);
            listed.insert((x, z));
            let got = to_json(g.get(x, z));
            if !scalar::matches(&got, &c[2], false) {
                errors.push(format!("({x}, {z}) = {}, expected {}", show(&got), show(&c[2])));
            }
        }
    }
    if let Some(o) = opt(op, "others") {
        let mut bad = 0;
        for x in 0..geo.count_x {
            for z in 0..geo.count_z {
                if !listed.contains(&(x, z)) && !scalar::matches(&to_json(g.get(x, z)), o, false) {
                    if bad < 5 {
                        errors.push(format!(
                            "({x}, {z}) = {}, expected {}",
                            show(&to_json(g.get(x, z))),
                            show(o)
                        ));
                    }
                    bad += 1;
                }
            }
        }
        if bad > 5 {
            errors.push(format!("… {bad} cells differ"));
        }
    }
    if let Some(rows) = opt(op, "rows") {
        let got = Json::Array(
            (0..geo.count_x)
                .map(|x| Json::Array((0..geo.count_z).map(|z| to_json(g.get(x, z))).collect()))
                .collect(),
        );
        if !scalar::matches(&got, rows, false) {
            errors.push(format!("rows {}, expected {}", show(&got), show(rows)));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// Hex bytes plus `zeros` trailing zero bytes.
fn bytes_of(op: &Json) -> R<Vec<u8>> {
    let mut b = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    if let Some(z) = opt(op, "zeros") {
        b.resize(b.len() + i64_of(z, "zeros")? as usize, 0);
    }
    Ok(b)
}

fn record(g: &AnyGrid, op: &Json) -> R<()> {
    let got = scalar::record_bytes(&*g.object());
    let want = bytes_of(op)?;
    if got != want {
        let n = got.len().min(64);
        return Err(format!(
            "record differs ({} bytes, expected {}):\n    got      {}…",
            got.len(),
            want.len(),
            hex_encode(&got[..n])
        ));
    }
    Ok(())
}

fn read(g: &AnyGrid, op: &Json) -> R<()> {
    let b = bytes_of(op)?;
    let ok = g.object().read(&mut ByteReader::new(&b));
    check_expect(op, Json::Bool(ok))
}

/// Saves the grid through the class-object store, loads it back through the
/// §2 factories and compares the records; `class` = the expected record
/// class.
fn store_round_trip(g: &AnyGrid, op: &Json) -> R<()> {
    let obj = g.object();
    let saved = scalar::save(&[RefValue::Object(obj.clone())])?;
    if let Some(c) = opt(op, "class") {
        let c = u32_of(c, "class")?;
        scalar::entry_payload(&saved, c, 0)?;
    }
    let (ok, roots) = scalar::load(&saved, &mut |c, _| create_for_clsid(c).map(RefValue::Object));
    if !ok {
        return Err("store load failed".into());
    }
    let loaded = match roots.first() {
        Some(RefValue::Object(o)) => o.clone(),
        _ => return Err("no grid reference after load".into()),
    };
    let lg = AnyGrid::of_object(loaded.clone()).ok_or("loaded a non-grid")?;
    if let Some(t) = opt(op, "expect_type") {
        if lg.type_name() != str_of(t, "expect_type")? {
            return Err(format!("loaded a {} grid", lg.type_name()));
        }
    }
    if scalar::record_bytes(&*loaded) != scalar::record_bytes(&*obj) {
        return Err("loaded record differs".into());
    }
    Ok(())
}

fn radius(g: &AnyGrid, op: &Json) -> R<()> {
    let (cx, cz) = pair(op, "center")?;
    let r = f32_of(field(op, "r")?, "r")?;
    let c = f32_of(field(op, "c")?, "c")?;
    let o = f32_of(field(op, "o")?, "o")?;
    macro_rules! go {
        ($s:expr) => {{
            let geo = $s.geometry();
            let (nx, nz) = match opt(op, "counts") {
                Some(_) => {
                    let (x, z) = pair(op, "counts")?;
                    (x as i32, z as i32)
                }
                None => (geo.count_x, geo.count_z),
            };
            add_radius_effect(&**$s, nx, nz, cx as i32, cz as i32, r, c, o);
            Ok(())
        }};
    }
    match g {
        AnyGrid::U8(s) => go!(s),
        AnyGrid::I8(s) => go!(s),
        AnyGrid::I16(s) => go!(s),
        _ => Err(format!("AddRadiusEffect has no {} variant", g.type_name())),
    }
}

fn u8_grid(w: &World, op: &Json, key: &str) -> R<Rc<SimGrid<u8>>> {
    match w.grid(op, key)? {
        AnyGrid::U8(g) => Ok(g),
        g => Err(format!("'{key}' must be a u8 grid, not {}", g.type_name())),
    }
}

fn distance(w: &World, op: &Json) -> R<()> {
    let t = u8_grid(w, op, "target")?;
    let m = u8_grid(w, op, "mask")?;
    let geo = t.geometry();
    add_distance_effect(
        &*t,
        geo.count_x,
        geo.count_z,
        &*m,
        u32_of(field(op, "value")?, "value")? as u8,
        f32_of(field(op, "radius")?, "radius")?,
        f32_of(field(op, "near")?, "near")?,
        f32_of(field(op, "far")?, "far")?,
    );
    Ok(())
}

/// `grid_two_grids`: A and B grids; records each callback as [x, z, b];
/// with `add`, the callback adds b to the A value (wrapping, as integers).
fn two_grids(w: &World, op: &Json) -> R<()> {
    let a = w.grid(op, "a")?;
    let b = w.grid(op, "b")?;
    let add = opt(op, "add").map_or(Ok(false), |v| bool_of(v, "add"))?;
    let mut calls = Vec::new();
    fn run<A: GridElem, B: GridElem>(a: &SimGrid<A>, b: &SimGrid<B>, add: bool, calls: &mut Vec<Json>) {
        process_two_grids(a, b, |x, z, v, bv| {
            calls.push(Json::Array(vec![ji(x), ji(z), to_json(bv)]));
            if add {
                *v = A::from_f64(v.to_f64() + bv.to_f64());
            }
        });
    }
    each!(&a, sa => each!(&b, sb => run(&**sa, &**sb, add, &mut calls)));
    if let Some(want) = opt(op, "calls") {
        let got = Json::Array(calls);
        if !scalar::matches(&got, want, false) {
            return Err(format!("callbacks {}, expected {}", show(&got), show(want)));
        }
    }
    Ok(())
}

fn rect(op: &Json) -> R<[i32; 4]> {
    let r = arr(field(op, "rect")?, "rect")?;
    if r.len() != 4 {
        return Err("'rect' is [x0, z0, x1, z1]".into());
    }
    let mut out = [0; 4];
    for (o, v) in out.iter_mut().zip(r) {
        *o = i64_of(v, "rect")? as i32;
    }
    Ok(out)
}

fn cells_json(cells: &[(i32, i32)]) -> Json {
    Json::Array(cells.iter().map(|&(x, z)| Json::Array(vec![ji(x), ji(z)])).collect())
}

/// `grid_scan`: a scanning iterator; `resets` is a list of
/// {"at": [x, z], "expect": [[x, z], …]} (all cells until false).
fn scan(op: &Json) -> R<()> {
    let r = rect(op)?;
    let mut it = ScanIterator::new(r[0], r[1], r[2], r[3]);
    if let Some(s) = opt(op, "step") {
        it.set_step(i64_of(s, "step")? as i32);
    }
    for step in arr(field(op, "resets")?, "resets")? {
        let (x, z) = pair(step, "at")?;
        it.reset(x as i32, z as i32);
        let mut got = Vec::new();
        while let Some(c) = it.next_cell() {
            got.push(c);
            if got.len() > 100_000 {
                return Err("scan does not end".into());
            }
        }
        let got = cells_json(&got);
        let want = field(step, "expect")?;
        if !scalar::matches(&got, want, false) {
            return Err(format!("scan {}, expected {}", show(&got), show(want)));
        }
    }
    Ok(())
}

/// `grid_random_iterator`: a random cell iterator in `mode` "table" or
/// "prime" (or "constructed"); `resets` is a list of
/// {"shuffle", "rand_calls", "first", "count", "full_pass"} checks.
fn random_iterator(op: &Json) -> R<()> {
    let seed = opt(op, "crt_seed").map_or(Ok(1), |v| u32_of(v, "crt_seed"))?;
    let mut crt = CrtRand::new(seed);
    let mode = opt(op, "mode").map_or(Ok("table"), |m| str_of(m, "mode"))?;
    let mut it = match mode {
        "table" => {
            let r = rect(op)?;
            CellIterator::new_table(r[0], r[1], r[2], r[3])
        }
        "prime" => {
            let r = rect(op)?;
            CellIterator::new(r[0], r[1], r[2], r[3])
        }
        "constructed" => CellIterator::constructed(),
        _ => return Err(format!("unknown iterator mode '{mode}'")),
    };
    if let Some(v) = opt(op, "first_next") {
        let got = it.next_cell().is_some();
        if got != bool_of(v, "first_next")? {
            return Err(format!("first next() returned {got}"));
        }
    }
    for step in opt(op, "resets").map_or(Ok(&[][..]), |r| arr(r, "resets"))? {
        if let Some(r) = opt(step, "rect") {
            let r = rect(&Json::Object(vec![("rect".into(), r.clone())]))?;
            it.set_rect(r[0], r[1], r[2], r[3]);
        }
        let shuffle = opt(step, "shuffle").map_or(Ok(true), |v| bool_of(v, "shuffle"))?;
        let before = crt;
        it.reset(shuffle, &mut crt);
        if let Some(v) = opt(step, "rand_calls") {
            let want = i64_of(v, "rand_calls")?;
            let mut probe = before;
            let mut n = 0i64;
            while probe != crt && n <= want + 1 {
                probe.rand();
                n += 1;
            }
            if n != want {
                return Err(format!("reset made {n} rand() calls, expected {want}"));
            }
        }
        if let Some(c) = opt(step, "count") {
            if i64::from(it.count) != i64_of(c, "count")? {
                return Err(format!("count {}, expected {}", it.count, show(c)));
            }
        }
        if let Some(t) = opt(step, "table") {
            let got = Json::Array(it.table.iter().map(|&v| ji(v)).collect());
            if !scalar::matches(&got, t, false) {
                return Err(format!("table {}, expected {}", show(&got), show(t)));
            }
        }
        let mut seen = Vec::new();
        if let Some(f) = opt(step, "first") {
            let n = arr(f, "first")?.len();
            for _ in 0..n {
                match it.next_cell() {
                    Some(c) => seen.push(c),
                    None => break,
                }
            }
            let got = cells_json(&seen);
            if !scalar::matches(&got, f, false) {
                return Err(format!("first cells {}, expected {}", show(&got), show(f)));
            }
        }
        if opt(step, "full_pass").map_or(Ok(false), |v| bool_of(v, "full_pass"))? {
            while let Some(c) = it.next_cell() {
                seen.push(c);
            }
            let n = seen.len();
            seen.sort_unstable();
            seen.dedup();
            if seen.len() != n || i64::try_from(n).ok() != Some(i64::from(it.count)) {
                return Err(format!(
                    "full pass visited {n} cells ({} distinct) of {}",
                    seen.len(),
                    it.count
                ));
            }
            if it.state != IteratorState::Exhausted || it.next_cell().is_some() {
                return Err("iterator not exhausted after a full pass".into());
            }
        }
    }
    Ok(())
}
