// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/simgrid.md`: named grids of any element
//! type, the map utilities and the scanning iterator, on `fc4_core::grid`.
//! See README.md ("Simgrid ops").

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use fc4_core::grid::{
    add_distance_effect, add_radius_effect, process_two_grids, CityDims, GridElem, ScanIterator, SimGrid,
    TractGeometry, TractGrid,
};
use fc4_core::stream::{ByteReader, ByteWriter, SaveObject};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{check_expect_bool, field, int, opt_int, rel_close, HostRef};

type R<T> = Result<T, String>;

/// A core grid of any of the ten element types (shared: cloning an
/// `AnyGrid` shares the grid).
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
    pub(crate) fn new(type_name: &str) -> R<AnyGrid> {
        Ok(match type_name {
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
            other => return Err(format!("unknown grid type '{other}'")),
        })
    }

    pub(crate) fn is_float(&self) -> bool {
        matches!(self, AnyGrid::F32(_) | AnyGrid::F64(_))
    }

    pub(crate) fn geometry(&self) -> TractGeometry {
        each!(self, g => g.geometry())
    }

    pub(crate) fn set_tract_size(&self, s: i32, city: &CityDims) {
        each!(self, g => { g.set_tract_size(city, s); })
    }

    /// `GetTractValue` as a number (out of range reads 0).
    pub(crate) fn tract_value(&self, x: i32, z: i32) -> f64 {
        each!(self, g => g.get(x, z).to_f64())
    }

    pub(crate) fn cell_value(&self, x: i32, z: i32) -> f64 {
        each!(self, g => g.get_cell_value(x, z).to_f64())
    }

    /// `SetTractValue` from a number (cast as by `as`).
    pub(crate) fn set_tract_value(&self, x: i32, z: i32, v: f64) {
        each!(self, g => g.set(x, z, GridElem::from_f64(v)))
    }

    /// `SetTractValues`.
    pub(crate) fn set_all(&self, v: f64) {
        each!(self, g => g.fill(GridElem::from_f64(v)))
    }
}

pub(crate) struct World {
    city: RefCell<CityDims>,
    grids: RefCell<HashMap<String, AnyGrid>>,
}

fn world(host: &HostRef) -> Rc<World> {
    let hs = &host.0;
    if let Some(w) = hs.grids.borrow().as_ref() {
        return w.clone();
    }
    let w = Rc::new(World {
        city: RefCell::new(CityDims::default()),
        grids: RefCell::new(HashMap::new()),
    });
    *hs.grids.borrow_mut() = Some(w.clone());
    w
}

pub(crate) fn num_of(j: &Json) -> R<f64> {
    j.as_f64()
        .or_else(|| j.as_i64().map(|v| v as f64))
        .ok_or_else(|| format!("not a number: {j:?}"))
}

pub(crate) fn nums(j: &Json) -> R<Vec<f64>> {
    j.as_array().ok_or("expected an array")?.iter().map(num_of).collect()
}

fn args(op: &Json) -> R<Vec<f64>> {
    match op.get("args") {
        None => Ok(Vec::new()),
        Some(a) => nums(a),
    }
}

pub(crate) fn close(got: f64, want: f64, float: bool) -> bool {
    if float {
        rel_close(got, want) || (got - want).abs() < 1e-9
    } else {
        got == want
    }
}

/// `[[x, z, v], ...]` triples.
pub(crate) fn triples(j: &Json) -> R<Vec<(i32, i32, f64)>> {
    let mut out = Vec::new();
    for t in j.as_array().ok_or("expected a list of [x, z, value]")? {
        let v = nums(t)?;
        if v.len() != 3 {
            return Err(format!("expected [x, z, value], got {t:?}"));
        }
        out.push((v[0] as i32, v[1] as i32, v[2]));
    }
    Ok(out)
}

fn grid_name(op: &Json, key: &str) -> R<String> {
    Ok(field(op, key)?
        .as_str()
        .ok_or_else(|| format!("'{key}' must be a grid name"))?
        .to_owned())
}

/// Executes a simgrid op; `None` if `name` is not one.
pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    if !name.starts_with("grid_") && name != "iter_scan" {
        return None;
    }
    let w = world(host);
    Some(run(&w, name, op))
}

fn run(w: &World, name: &str, op: &Json) -> R<()> {
    match name {
        "grid_city" => {
            let mut c = w.city.borrow_mut();
            if let Some(cells) = op.get("cells") {
                let v = nums(cells)?;
                if v.len() != 2 {
                    return Err("'cells' is [x, z]".into());
                }
                c.cells_x = v[0] as i32;
                c.cells_z = v[1] as i32;
            }
            if let Some(cw) = op.get("cell_width") {
                let v = num_of(cw)? as f32;
                c.cell_width_x = v;
                c.cell_width_z = v;
            }
            Ok(())
        }
        "grid_new" => {
            let g = grid_name(op, "grid")?;
            let grid = AnyGrid::new(field(op, "type")?.as_str().ok_or("'type' must be a string")?)?;
            let city = *w.city.borrow();
            if let Some(s) = opt_int(op, "size")? {
                grid.set_tract_size(s as i32, &city);
            }
            w.grids.borrow_mut().insert(g, grid);
            Ok(())
        }
        "grid_iter" | "iter_scan" => iter_scan(op),
        "grid_distance_effect" => distance(w, op),
        "grid_two_grids" => two_grids(w, op),
        _ => {
            let g = grid_name(op, "grid")?;
            let city = *w.city.borrow();
            let grid = w
                .grids
                .borrow()
                .get(&g)
                .cloned()
                .ok_or_else(|| format!("no grid '{g}'"))?;
            grid_op(&grid, &city, name, op)
        }
    }
}

fn grid_op(grid: &AnyGrid, city: &CityDims, name: &str, op: &Json) -> R<()> {
    let float = grid.is_float();
    match name {
        "grid_set_tract_size" => {
            grid.set_tract_size(int(op, "size")? as i32, city);
            Ok(())
        }
        "grid_expect_geometry" => {
            let g = grid.geometry();
            let checks: [(&str, f64); 8] = [
                ("size", f64::from(g.tract_size)),
                ("shift", f64::from(g.shift)),
                ("count_x", f64::from(g.count_x)),
                ("count_z", f64::from(g.count_z)),
                ("width_x", f64::from(g.width_x)),
                ("width_z", f64::from(g.width_z)),
                ("inv_width_x", f64::from(g.inv_width_x)),
                ("inv_width_z", f64::from(g.inv_width_z)),
            ];
            for (k, got) in checks {
                if let Some(want) = op.get(k) {
                    let want = num_of(want)?;
                    if !close(got, want, true) {
                        return Err(format!("{k}: got {got}, expected {want} ({g:?})"));
                    }
                }
            }
            Ok(())
        }
        "grid_set" => {
            if let Some(all) = op.get("all") {
                grid.set_all(num_of(all)?);
            }
            if let Some(vals) = op.get("values") {
                for (x, z, v) in triples(vals)? {
                    grid.set_tract_value(x, z, v);
                }
            }
            Ok(())
        }
        "grid_expect" => {
            for (x, z, want) in triples(field(op, "values")?)? {
                let got = grid.tract_value(x, z);
                if !close(got, want, float) {
                    return Err(format!("tract ({x}, {z}): got {got}, expected {want}"));
                }
            }
            Ok(())
        }
        "grid_call" => grid_call(grid, op),
        "grid_write" => {
            let mut wr = ByteWriter::new();
            let ok = each!(grid, g => g.write(&mut wr));
            let mut want = hex_decode(field(op, "hex")?.as_str().ok_or("'hex' must be a string")?)?;
            if let Some(n) = opt_int(op, "zero_tail")? {
                want.extend(std::iter::repeat(0u8).take(n as usize));
            }
            if !ok || wr.bytes() != want.as_slice() {
                return Err(format!(
                    "wrote {} bytes {}…, expected {} bytes {}…",
                    wr.len(),
                    hex_encode(&wr.bytes()[..wr.len().min(64)]),
                    want.len(),
                    hex_encode(&want[..want.len().min(64)])
                ));
            }
            Ok(())
        }
        "grid_read" => {
            let mut data = hex_decode(field(op, "hex")?.as_str().ok_or("'hex' must be a string")?)?;
            if let Some(n) = opt_int(op, "zero_tail")? {
                data.extend(std::iter::repeat(0u8).take(n as usize));
            }
            let mut rd = ByteReader::new(&data);
            let ok = each!(grid, g => g.read(&mut rd));
            if let Some(p) = opt_int(op, "position")? {
                if rd.position() as i64 != p {
                    return Err(format!("position {}, expected {p}", rd.position()));
                }
            }
            check_expect_bool(op, ok)
        }
        "grid_radius_effect" => {
            let c = nums(field(op, "center")?)?;
            if c.len() != 2 {
                return Err("'center' is [x, z]".into());
            }
            let r = num_of(field(op, "radius")?)? as f32;
            let cv = num_of(field(op, "center_value")?)? as f32;
            let ov = num_of(field(op, "outer_value")?)? as f32;
            let (cx, cz) = (c[0] as i32, c[1] as i32);
            let geo = grid.geometry();
            let (nx, nz) = (geo.count_x, geo.count_z);
            match grid {
                AnyGrid::U8(g) => add_radius_effect(&**g, nx, nz, cx, cz, r, cv, ov),
                AnyGrid::I8(g) => add_radius_effect(&**g, nx, nz, cx, cz, r, cv, ov),
                AnyGrid::I16(g) => add_radius_effect(&**g, nx, nz, cx, cz, r, cv, ov),
                _ => return Err("AddRadiusEffect exists for u8, i8 and i16 grids only".into()),
            }
            Ok(())
        }
        other => Err(format!("unknown op '{other}'")),
    }
}

fn grid_call(grid: &AnyGrid, op: &Json) -> R<()> {
    let method = field(op, "method")?.as_str().ok_or("'method' must be a string")?;
    let a = args(op)?;
    let i = |n: usize| -> R<i32> {
        a.get(n)
            .map(|&v| v as i32)
            .ok_or_else(|| format!("missing argument {n}"))
    };
    let f = |n: usize| -> R<f32> {
        a.get(n)
            .map(|&v| v as f32)
            .ok_or_else(|| format!("missing argument {n}"))
    };
    let float = grid.is_float();
    let geo = grid.geometry();
    let got: Json = match method {
        "cell_value" => Json::Float(grid.cell_value(i(0)?, i(1)?)),
        "tract_value" => Json::Float(grid.tract_value(i(0)?, i(1)?)),
        "average_cell_rect" => {
            Json::Float(each!(grid, g => g.average_in_cell_rect(i(0)?, i(1)?, i(2)?, i(3)?).to_f64()))
        }
        "average_tract_rect" => {
            Json::Float(each!(grid, g => g.average_in_tract_rect(i(0)?, i(1)?, i(2)?, i(3)?).to_f64()))
        }
        "tract_in_bounds" => {
            // The arguments are unsigned in the original (§4 slot 18).
            i(1)?;
            // Wrapped to 32 bits: 4294967295 is (u32) −1.
            Json::Bool(geo.in_bounds(a[0] as i64 as i32, a[1] as i64 as i32))
        }
        "position_to_tract" => {
            let (x, z) = geo.position_to_tract(f(0)?, f(1)?);
            Json::Array(vec![Json::Int(x.into()), Json::Int(z.into())])
        }
        "tract_center" | "tract_corner" => {
            let (x, z) = if method == "tract_center" {
                geo.tract_center(i(0)?, i(1)?)
            } else {
                geo.tract_corner(i(0)?, i(1)?)
            };
            Json::Array(vec![Json::Float(x.into()), Json::Float(z.into())])
        }
        "clsid" => Json::Int(each!(grid, g => g.class_id()).into()),
        "init" => Json::Bool(each!(grid, g => g.init())),
        "set_instance_id" => {
            let id = a.first().copied().ok_or("missing argument 0")? as u32;
            each!(grid, g => g.set_instance_id(id));
            Json::Bool(true)
        }
        other => return Err(format!("unknown grid method '{other}'")),
    };
    let Some(want) = op.get("expect") else { return Ok(()) };
    let ok = match (&got, want) {
        (Json::Bool(g), w) => crate::runner::boolean(w) == Some(*g),
        (Json::Int(g), w) => w.as_i64() == Some(*g),
        (Json::Float(g), w) => close(*g, num_of(w)?, float),
        (Json::Array(g), Json::Array(w)) => {
            g.len() == w.len()
                && g.iter().zip(w).all(|(a, b)| {
                    let a = a.as_f64().or_else(|| a.as_i64().map(|v| v as f64)).unwrap_or(f64::NAN);
                    num_of(b).map(|b| close(a, b, true)).unwrap_or(false)
                })
        }
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{method}: got {got:?}, expected {want:?}"))
    }
}

fn distance(w: &World, op: &Json) -> R<()> {
    let target = grid_name(op, "grid")?;
    let mask = op
        .get("mask")
        .and_then(Json::as_str)
        .map_or_else(|| target.clone(), str::to_owned);
    let m = int(op, "mask_value")? as u8;
    let radius = num_of(field(op, "radius")?)? as f32;
    let near = num_of(field(op, "near")?)? as f32;
    let far = num_of(field(op, "far")?)? as f32;
    let grids = w.grids.borrow();
    let Some(AnyGrid::U8(t)) = grids.get(&target) else {
        return Err(format!("target '{target}' must be a u8 grid"));
    };
    let Some(AnyGrid::U8(mk)) = grids.get(&mask) else {
        return Err(format!("mask '{mask}' must be a u8 grid"));
    };
    let geo = t.geometry();
    add_distance_effect(&**t, geo.count_x, geo.count_z, &**mk, m, radius, near, far);
    Ok(())
}

fn two_grids(w: &World, op: &Json) -> R<()> {
    let a = grid_name(op, "a")?;
    let b = grid_name(op, "b")?;
    let grids = w.grids.borrow();
    let Some(AnyGrid::I16(bg)) = grids.get(&b) else {
        return Err(format!("'{b}' must be an i16 grid"));
    };
    let Some(AnyGrid::U8(ag)) = grids.get(&a) else {
        return Err(format!("'{a}' must be a u8 grid"));
    };
    let mut visits = Vec::new();
    process_two_grids(&**ag, &**bg, |x, z, _, v| visits.push((x, z, f64::from(v))));
    let want = triples(field(op, "expect")?)?;
    if visits != want {
        return Err(format!("visits {visits:?}, expected {want:?}"));
    }
    Ok(())
}

fn iter_scan(op: &Json) -> R<()> {
    let r = nums(field(op, "rect")?)?;
    if r.len() != 4 {
        return Err("'rect' is [x0, x1, z0, z1]".into());
    }
    let mut it = ScanIterator::new(r[0] as i32, r[2] as i32, r[1] as i32, r[3] as i32);
    if let Some(s) = opt_int(op, "step")? {
        it.set_step(s as i32);
    }
    let reset = nums(field(op, "reset")?)?;
    if reset.len() != 2 {
        return Err("'reset' is [x, z]".into());
    }
    it.reset(reset[0] as i32, reset[1] as i32);
    let want: Vec<(i32, i32)> = field(op, "expect")?
        .as_array()
        .ok_or("'expect' must be a list")?
        .iter()
        .map(|p| nums(p).map(|v| (v[0] as i32, v[1] as i32)))
        .collect::<R<_>>()?;
    let got: Vec<(i32, i32)> = std::iter::from_fn(|| it.next_cell()).take(want.len() + 8).collect();
    if got != want {
        return Err(format!("visited {got:?}, expected {want:?}"));
    }
    if it.next_cell().is_some() {
        return Err("GetNext returned true after the end".into());
    }
    if let Some(rest) = op.get("rests") {
        let rest = nums(rest)?;
        if rest.len() != 2 {
            return Err("'rests' is [x, z]".into());
        }
        let want = (rest[0] as i32, rest[1] as i32);
        if (it.x, it.z) != want {
            return Err(format!("rests at {:?}, expected {want:?}", (it.x, it.z)));
        }
    }
    Ok(())
}
