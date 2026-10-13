// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Shared pieces of the scalar-simulator vectors (demand, ordinance,
//! weather): an event log with partial matching, tolerant value comparison,
//! property maps written as JSON, and class-object store save/load/patch
//! helpers. See README.md ("Scalar simulator ops").

use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use fc4_core::property::PropertyHolder;
use fc4_core::store::{crc32_mpeg2, load_store, save_store, LoadRegistry, Package, CRC_LIMIT, STORE_GROUP};
use fc4_core::stream::{ByteReader, ByteWriter, OutStream, RefValue, SaveObject, SaveRegistry};
use fc4_core::variant::{mem, Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};

pub(crate) type R<T> = Result<T, String>;

/// Relative tolerance of float comparisons (demand spec §14: 1e-5).
pub(crate) const REL_TOL: f64 = 1e-5;

/// The per-vector world of a scalar simulator, created on first use.
pub(crate) fn world<T: Any>(host: &HostRef, key: &'static str, make: impl FnOnce() -> Rc<T>) -> Rc<T> {
    let mut ext = host.0.ext.borrow_mut();
    if let Some(w) = ext.get(key) {
        if let Ok(w) = w.clone().downcast::<T>() {
            return w;
        }
    }
    let w = make();
    ext.insert(key, w.clone() as Rc<dyn Any>);
    w
}

/// A number from JSON: integers, floats, or `"0x…"` / decimal strings.
pub(crate) fn num(j: &Json) -> Option<f64> {
    match j {
        Json::Float(f) => Some(*f),
        _ => j.as_i64().map(|v| v as f64),
    }
}

pub(crate) fn u32_of(j: &Json, what: &str) -> R<u32> {
    j.as_i64()
        .map(|v| v as u32)
        .ok_or_else(|| format!("'{what}' must be an integer"))
}

pub(crate) fn i64_of(j: &Json, what: &str) -> R<i64> {
    j.as_i64().ok_or_else(|| format!("'{what}' must be an integer"))
}

pub(crate) fn f32_of(j: &Json, what: &str) -> R<f32> {
    num(j)
        .map(|v| v as f32)
        .ok_or_else(|| format!("'{what}' must be a number"))
}

pub(crate) fn bool_of(j: &Json, what: &str) -> R<bool> {
    crate::runner::boolean(j).ok_or_else(|| format!("'{what}' must be a boolean"))
}

pub(crate) fn str_of<'a>(j: &'a Json, what: &str) -> R<&'a str> {
    j.as_str().ok_or_else(|| format!("'{what}' must be a string"))
}

pub(crate) fn arr<'a>(j: &'a Json, what: &str) -> R<&'a [Json]> {
    j.as_array().ok_or_else(|| format!("'{what}' must be an array"))
}

pub(crate) fn opt<'a>(op: &'a Json, key: &str) -> Option<&'a Json> {
    match op.get(key) {
        Some(Json::Null) | None => None,
        Some(v) => Some(v),
    }
}

/// The i-th call argument.
pub(crate) fn arg(op: &Json, i: usize) -> R<&Json> {
    op.get("args")
        .and_then(Json::as_array)
        .and_then(|a| a.get(i))
        .ok_or_else(|| format!("missing argument {i}"))
}

pub(crate) fn close(got: f64, want: f64) -> bool {
    if got == want {
        return true;
    }
    if want == 0.0 {
        got.abs() <= 1e-6
    } else {
        ((got - want) / want).abs() <= REL_TOL
    }
}

/// Whether `got` matches `want`: numbers with the float tolerance (exactly
/// when both are integers, including an integer `want` written as a decimal
/// or `"0x…"` string, so IDs never match a neighbour), strings and booleans
/// exactly, arrays element by element, objects key by key (keys compared as
/// integers when both parse). With `partial`, objects only check the keys
/// `want` gives.
pub(crate) fn matches(got: &Json, want: &Json, partial: bool) -> bool {
    match (got, want) {
        (Json::Int(a), Json::Int(b)) => a == b,
        (Json::Int(a), Json::Str(_)) if want.as_i64().is_some() => want.as_i64() == Some(*a),
        (Json::Bool(a), w) => crate::runner::boolean(w) == Some(*a),
        (Json::Str(a), Json::Str(b)) => a == b,
        (Json::Null, Json::Null) => true,
        (Json::Array(a), Json::Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(x, y)| matches(x, y, partial)),
        (Json::Object(a), Json::Object(b)) => {
            let key_eq = |x: &str, y: &str| {
                x == y
                    || matches!((Json::Str(x.into()).as_i64(), Json::Str(y.into()).as_i64()), (Some(p), Some(q)) if p == q)
            };
            if !partial && a.len() != b.len() {
                return false;
            }
            b.iter().all(|(k, v)| {
                a.iter()
                    .find(|(gk, _)| key_eq(gk, k))
                    .is_some_and(|(_, gv)| matches(gv, v, partial))
            })
        }
        (g, w) => match (num(g), num(w)) {
            (Some(x), Some(y)) => close(x, y),
            _ => false,
        },
    }
}

/// Checks the `expect` field of an op against a result.
pub(crate) fn check_expect(op: &Json, got: Json) -> R<()> {
    if let Some(w) = op.get("expect") {
        if !matches(&got, w, false) {
            return Err(format!("returned {}, expected {}", show(&got), show(w)));
        }
    }
    Ok(())
}

/// Compact JSON text for error messages.
pub(crate) fn show(j: &Json) -> String {
    match j {
        Json::Null => "null".into(),
        Json::Bool(b) => b.to_string(),
        Json::Int(i) => i.to_string(),
        Json::Float(f) => format!("{f}"),
        Json::Str(s) => format!("\"{s}\""),
        Json::Array(a) => format!("[{}]", a.iter().map(show).collect::<Vec<_>>().join(", ")),
        Json::Object(kv) => format!(
            "{{{}}}",
            kv.iter()
                .map(|(k, v)| format!("\"{k}\": {}", show(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub(crate) fn jf(v: f32) -> Json {
    Json::Float(f64::from(v))
}

pub(crate) fn ji(v: impl Into<i64>) -> Json {
    Json::Int(v.into())
}

pub(crate) fn hex_key(k: u32) -> String {
    format!("0x{k:X}")
}

/// Compares a partial state object: `want` keys must all be present in
/// `got` and match.
pub(crate) fn check_state(got: &[(String, Json)], want: &Json) -> R<()> {
    let want = want.as_object().ok_or("'state' must be an object")?;
    let mut errors = Vec::new();
    for (k, v) in want {
        match got.iter().find(|(gk, _)| gk == k) {
            None => errors.push(format!("unknown state field '{k}'")),
            Some((_, g)) => {
                if !matches(g, v, false) {
                    errors.push(format!("{k}: got {}, expected {}", show(g), show(v)));
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

// ---- events -------------------------------------------------------------------------------------

/// Recorded events, each a JSON object with an `event` kind.
#[derive(Default)]
pub(crate) struct EventLog(RefCell<Vec<Json>>);

impl EventLog {
    pub(crate) fn push(&self, kind: &str, fields: Vec<(&str, Json)>) {
        let mut kv = vec![("event".to_owned(), Json::Str(kind.to_owned()))];
        kv.extend(fields.into_iter().map(|(k, v)| (k.to_owned(), v)));
        self.0.borrow_mut().push(Json::Object(kv));
    }

    fn filtered(&self, filter: Option<&Json>) -> Vec<Json> {
        self.0
            .borrow()
            .iter()
            .filter(|e| filter.map_or(true, |f| matches(e, f, true)))
            .cloned()
            .collect()
    }

    /// `*_expect_events`: the events since the last check (after `filter`)
    /// match `events` one to one, in order. Clears the log.
    pub(crate) fn expect_events(&self, op: &Json) -> R<()> {
        let want = arr(field(op, "events")?, "events")?;
        let got = self.filtered(op.get("filter"));
        self.0.borrow_mut().clear();
        let ok = got.len() == want.len() && got.iter().zip(want).all(|(g, w)| matches(g, w, true));
        if ok {
            Ok(())
        } else {
            Err(format!(
                "events differ:\n  got      {}\n  expected {}",
                got.iter().map(show).collect::<Vec<_>>().join("\n           "),
                want.iter().map(show).collect::<Vec<_>>().join("\n           ")
            ))
        }
    }

    /// `*_expect_event_count`: number of matching events (does not clear).
    pub(crate) fn expect_count(&self, op: &Json) -> R<()> {
        let n = self.filtered(op.get("filter")).len() as i64;
        for (k, ok) in [("count", 0), ("min", 1), ("max", 2)] {
            if let Some(w) = op.get(k).and_then(Json::as_i64) {
                let good = match ok {
                    0 => n == w,
                    1 => n >= w,
                    _ => n <= w,
                };
                if !good {
                    return Err(format!("event count {n}, {k} {w}"));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn clear(&self) {
        self.0.borrow_mut().clear();
    }
}

// ---- property maps ------------------------------------------------------------------------------

fn elems(kind: &str, v: &Json) -> R<Vec<Elem>> {
    let one = |j: &Json| -> R<Elem> {
        Ok(match kind {
            "u8" => Elem::U8(u32_of(j, kind)? as u8),
            "u16" => Elem::U16(u32_of(j, kind)? as u16),
            "u32" => Elem::U32(u32_of(j, kind)?),
            "i32" => Elem::I32(i64_of(j, kind)? as i32),
            "i64" => Elem::I64(i64_of(j, kind)?),
            "f32" => Elem::F32(f32_of(j, kind)?),
            "bool" => Elem::Bool(bool_of(j, kind)?),
            _ => return Err(format!("unknown property type '{kind}'")),
        })
    };
    match v {
        Json::Array(a) => a.iter().map(one).collect(),
        other => Ok(vec![one(other)?]),
    }
}

fn mem_of(kind: &str) -> u16 {
    match kind {
        "u8" => mem::UINT8,
        "u16" => mem::UINT16,
        "u32" => mem::UINT32,
        "i32" => mem::SINT32,
        "i64" => mem::SINT64,
        "f32" => mem::FLOAT32,
        _ => mem::BOOL,
    }
}

/// One property value: `{"u32": 5}` (a scalar), `{"f32": [1, 2]}` (an
/// array), `{"str": "text"}`; types u8, u16, u32, i32, i64, f32, bool.
pub(crate) fn variant_of(v: &Json) -> R<Variant> {
    let kv = v
        .as_object()
        .ok_or("a property value must be an object like {\"u32\": 1}")?;
    let [(kind, val)] = kv else {
        return Err("a property value has exactly one type key".into());
    };
    if kind == "str" {
        return Ok(Variant::string(str_of(val, "str")?.as_bytes()));
    }
    let items = elems(kind, val)?;
    Ok(match val {
        Json::Array(_) => Variant::array(mem_of(kind), items),
        _ => Variant::Scalar(items[0]),
    })
}

/// A property map `{"0xID": value, …}`.
pub(crate) fn holder_of(j: &Json) -> R<PropertyHolder> {
    let mut h = PropertyHolder::new();
    for (k, v) in j.as_object().ok_or("properties must be an object")? {
        let id = Json::Str(k.clone())
            .as_i64()
            .ok_or_else(|| format!("bad property ID '{k}'"))? as u32;
        h.add(id, variant_of(v).map_err(|e| format!("property {k}: {e}"))?);
    }
    Ok(h)
}

// ---- class-object store ---------------------------------------------------------------------------

/// One saved city: the store package plus a root record holding references
/// to the roots.
#[derive(Clone)]
pub(crate) struct Saved {
    pub(crate) pkg: Package,
    pub(crate) root: Vec<u8>,
}

/// Saves `roots` (references in a root record) through the class-object store.
pub(crate) fn save(roots: &[RefValue]) -> R<Saved> {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut root = ByteWriter::with_registry(reg.clone());
    for r in roots {
        root.put_ref(r);
    }
    let mut pkg = Package::new();
    let rep = save_store(&reg, &mut pkg);
    if rep.aborted || rep.entries_discarded != 0 {
        return Err(format!("save failed: {rep:?}"));
    }
    Ok(Saved {
        pkg,
        root: root.into_bytes(),
    })
}

/// Loads a saved city: creates objects through `factory`, reads the store,
/// then resolves the root references. Returns (clean store load, roots).
pub(crate) fn load(saved: &Saved, factory: &mut dyn FnMut(u32, u32) -> Option<RefValue>) -> (bool, Vec<RefValue>) {
    let mut f = |c: u32, o: u32| factory(c, o);
    let mut reg = LoadRegistry::new(BTreeMap::new(), Box::new(&mut f));
    let report = load_store(&saved.pkg, &mut reg);
    let ok = report.index_ok && report.failed_classes.is_empty();
    let mut r = ByteReader::with_resolver(&saved.root, &mut reg);
    let mut roots = Vec::new();
    while r.remaining() > 0 {
        match fc4_core::stream::InStream::get_ref(&mut r) {
            Some(v) => roots.push(v),
            None => break,
        }
    }
    (ok, roots)
}

/// Byte-compares two saved cities.
pub(crate) fn same_save(got: &Saved, want: &Saved) -> R<()> {
    if got.root != want.root {
        return Err("root records differ".into());
    }
    for (k, v) in &want.pkg {
        match got.pkg.get(k) {
            Some(g) if g == v => {}
            Some(g) => {
                return Err(format!(
                    "record {:08X} differs:\n    got      {}\n    expected {}",
                    k.0,
                    hex_encode(g),
                    hex_encode(v)
                ))
            }
            None => return Err(format!("record {:08X} missing", k.0)),
        }
    }
    if got.pkg.len() != want.pkg.len() {
        return Err("record sets differ".into());
    }
    Ok(())
}

/// The payload of entry `entry` of class record `class`.
pub(crate) fn entry_payload(saved: &Saved, class: u32, entry: usize) -> R<Vec<u8>> {
    let rec = saved
        .pkg
        .get(&(class, STORE_GROUP, 0))
        .ok_or("no record for that class")?;
    let (pos, size) = entry_span(rec, entry)?;
    Ok(rec[pos + 12..pos + size].to_vec())
}

fn entry_span(rec: &[u8], entry: usize) -> R<(usize, usize)> {
    let mut pos = 0usize;
    for i in 0..=entry {
        let b = rec.get(pos..pos + 4).ok_or("entry beyond the record")?;
        let size = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize;
        if size < 12 || pos + size > rec.len() {
            return Err("bad entry size".into());
        }
        if i == entry {
            return Ok((pos, size));
        }
        pos += size;
    }
    unreachable!()
}

/// `*_patch`: overwrites (or, with `insert`, inserts; with `delete` n,
/// removes) bytes at `offset` of an entry's payload, then fixes the entry
/// size and the CRC.
pub(crate) fn patch(saved: &mut Saved, class: u32, op: &Json) -> R<()> {
    let entry = op.get("entry").and_then(Json::as_i64).unwrap_or(0) as usize;
    let offset = crate::runner::int(op, "offset")? as usize;
    let bytes = match op.get("hex") {
        Some(h) => hex_decode(str_of(h, "hex")?)?,
        None => Vec::new(),
    };
    let insert = op.get("insert").and_then(crate::runner::boolean).unwrap_or(false);
    let delete = op.get("delete").and_then(Json::as_i64).unwrap_or(0) as usize;
    let rec = saved
        .pkg
        .get_mut(&(class, STORE_GROUP, 0))
        .ok_or("no record for that class")?;
    let (pos, size) = entry_span(rec, entry)?;
    let start = pos + 12 + offset;
    let mut payload_end = pos + size;
    if delete > 0 {
        if start + delete > payload_end {
            return Err("delete beyond the entry".into());
        }
        rec.drain(start..start + delete);
        payload_end -= delete;
    }
    if insert {
        if start > payload_end {
            return Err("insert beyond the entry".into());
        }
        rec.splice(start..start, bytes.iter().copied());
        payload_end += bytes.len();
    } else if !bytes.is_empty() {
        if start + bytes.len() > payload_end {
            return Err("patch beyond the entry".into());
        }
        rec[start..start + bytes.len()].copy_from_slice(&bytes);
    }
    let new_size = payload_end - pos;
    rec[pos..pos + 4].copy_from_slice(&(new_size as u32).to_le_bytes());
    let span = (new_size - 8).min(CRC_LIMIT);
    let crc = crc32_mpeg2(&rec[pos + 8..pos + 8 + span]);
    rec[pos + 4..pos + 8].copy_from_slice(&crc.to_le_bytes());
    Ok(())
}

/// Writes one object's record into bytes (no references allowed to resolve).
pub(crate) fn record_bytes(obj: &dyn SaveObject) -> Vec<u8> {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg);
    obj.write(&mut w);
    w.into_bytes()
}
