// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/foundations.md` §9 (variants), §10
//! (property holders in saves) and §12 (the C runtime `rand()`). See
//! README.md ("Foundations ops").

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use fc4_core::property::{ExemplarHolder, ExemplarSource, PropertyHolder};
use fc4_core::rng::CrtRand;
use fc4_core::stream::{ByteReader, ByteWriter, InStream, OutStream, RefResolver, RefValue, ResKey, SaveRegistry};
use fc4_core::variant::{mem, Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;

type R<T> = Result<T, String>;

fn field<'a>(op: &'a Json, key: &str) -> R<&'a Json> {
    op.get(key).ok_or_else(|| format!("missing field '{key}'"))
}

fn int(j: &Json, what: &str) -> R<i64> {
    j.as_i64().ok_or_else(|| format!("'{what}' must be an integer"))
}

fn hex_field(op: &Json, key: &str) -> R<Vec<u8>> {
    hex_decode(
        field(op, key)?
            .as_str()
            .ok_or_else(|| format!("'{key}' must be a hex string"))?,
    )
}

fn f32_of(j: &Json) -> R<f32> {
    if let Some(h) = j.as_str().and_then(|s| s.strip_prefix("0x")) {
        return u32::from_str_radix(h, 16)
            .map(f32::from_bits)
            .map_err(|e| e.to_string());
    }
    j.as_f64().map(|v| v as f32).ok_or_else(|| format!("bad float {j:?}"))
}

fn f64_of(j: &Json) -> R<f64> {
    if let Some(h) = j.as_str().and_then(|s| s.strip_prefix("0x")) {
        return u64::from_str_radix(h, 16)
            .map(f64::from_bits)
            .map_err(|e| e.to_string());
    }
    j.as_f64().ok_or_else(|| format!("bad float {j:?}"))
}

/// Element type names used in vectors.
fn elem_type(name: &str) -> R<u16> {
    Ok(match name {
        "bool" => mem::BOOL,
        "u8" => mem::UINT8,
        "i8" => mem::SINT8,
        "u16" => mem::UINT16,
        "i16" => mem::SINT16,
        "u32" => mem::UINT32,
        "i32" => mem::SINT32,
        "u64" => mem::UINT64,
        "i64" => mem::SINT64,
        "f32" => mem::FLOAT32,
        "f64" => mem::FLOAT64,
        "char" => mem::CHAR,
        "unichar" => mem::UNICODE_CHAR,
        "rzchar" => mem::RZ_CHAR,
        "voidptr" => mem::VOID_PTR,
        "object" => mem::IGZ_UNKNOWN,
        other => return Err(format!("unknown variant type '{other}'")),
    })
}

fn elem_of(t: u16, j: &Json) -> R<Elem> {
    let i = || int(j, "value");
    Ok(match t {
        mem::BOOL => Elem::Bool(j.as_bool().ok_or("bool value")?),
        mem::UINT8 => Elem::U8(i()? as u8),
        mem::SINT8 => Elem::I8(i()? as i8),
        mem::UINT16 => Elem::U16(i()? as u16),
        mem::SINT16 => Elem::I16(i()? as i16),
        mem::UINT32 => Elem::U32(i()? as u32),
        mem::SINT32 => Elem::I32(i()? as i32),
        mem::UINT64 => Elem::U64(i()? as u64),
        mem::SINT64 => Elem::I64(i()?),
        mem::FLOAT32 => Elem::F32(f32_of(j)?),
        mem::FLOAT64 => Elem::F64(f64_of(j)?),
        mem::CHAR => Elem::Char(i()? as u8),
        mem::UNICODE_CHAR => Elem::UnicodeChar(i()? as u16),
        mem::RZ_CHAR => Elem::RzChar(i()? as u8),
        mem::VOID_PTR => Elem::VoidPtr(i()? as u64),
        _ => return Err(format!("no elements of type 0x{t:x}")),
    })
}

/// A variant in vector notation: `{"type": "u32", "value": 7}`, `{"type":
/// "u16[]", "values": [..]}`, `{"type": "string", "value": "Abc"}`, `{"type":
/// "bytes", "hex": ".."}` (void array), `{"type": "empty"}`, `{"type":
/// "object", "id"?, "class"?}` (no id = null).
pub(crate) fn variant_of(j: &Json) -> R<Variant> {
    let ty = field(j, "type")?.as_str().ok_or("'type' must be a string")?;
    Ok(match ty {
        "empty" => Variant::Empty,
        "string" => Variant::string(field(j, "value")?.as_str().ok_or("string value")?.as_bytes()),
        "bytes" => Variant::array(0, hex_field(j, "hex")?.into_iter().map(Elem::U8).collect()),
        "object" => match j.get("id") {
            None => Variant::Object(RefValue::Null),
            Some(id) => Variant::Object(RefValue::Host {
                token: int(id, "id")? as u64,
                class_id: int(field(j, "class")?, "class")? as u32,
            }),
        },
        t if t.ends_with("[]") => {
            let et = elem_type(&t[..t.len() - 2])?;
            let items = field(j, "values")?
                .as_array()
                .ok_or("'values' must be an array")?
                .iter()
                .map(|v| elem_of(et, v))
                .collect::<R<Vec<_>>>()?;
            Variant::array(et, items)
        }
        t => Variant::Scalar(elem_of(elem_type(t)?, field(j, "value")?)?),
    })
}

fn same(got: &Variant, want: &Variant) -> bool {
    match (got, want) {
        (Variant::Scalar(Elem::F32(a)), Variant::Scalar(Elem::F32(b))) => a.to_bits() == b.to_bits(),
        (Variant::Scalar(Elem::F64(a)), Variant::Scalar(Elem::F64(b))) => a.to_bits() == b.to_bits(),
        _ => got == want,
    }
}

/// Resolves every reference to a host object whose token is its object ID.
struct IdResolver;

impl RefResolver for IdResolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: u64::from(object_id),
            class_id,
        })
    }
}

/// A writer whose registry hands out IDs equal to the object tokens used in
/// the vector (lower IDs are taken by placeholder objects).
fn writer_for(v: &[&Variant]) -> ByteWriter {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let max = v
        .iter()
        .filter_map(|x| match x {
            Variant::Object(RefValue::Host { token, .. }) => Some(*token),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    for t in 1..max {
        reg.borrow_mut().register(&RefValue::Host {
            token: 0xFFFF_0000 + t,
            class_id: 1,
        });
    }
    ByteWriter::with_registry(reg)
}

/// Executes a §9/§10/§12 op; `None` if `name` is not one.
pub fn exec(name: &str, op: &Json) -> Option<R<()>> {
    let r = match name {
        "variant_write" => variant_write(op),
        "variant_read" => variant_read(op),
        "holder_write" => holder_write(op),
        "holder_read" => holder_read(op),
        "exemplar_holder" => exemplar_holder(op),
        "crt_rand" => crt_rand(op),
        "variant_convert" => variant_convert(op),
        _ => return None,
    };
    Some(r)
}

/// An integer in a vector: a number, or a `"0x…"` bit pattern (for 64-bit values).
fn int_or_hex(j: &Json, what: &str) -> R<i64> {
    if let Some(h) = j.as_str().and_then(|s| s.strip_prefix("0x")) {
        return u64::from_str_radix(h, 16)
            .map(|v| v as i64)
            .map_err(|e| format!("'{what}': {e}"));
    }
    int(j, what)
}

/// `variant_convert`: `variant` (a scalar), `cases`: `[{to, expect}]`. A scalar
/// element converted as a property reader does (data-formats §5.1): integer
/// targets and Bool through the element's int64, float targets through its
/// double (a Float32 element read as Float32 as is).
fn variant_convert(op: &Json) -> R<()> {
    let e = match variant_of(field(op, "variant")?)? {
        Variant::Scalar(e) => e,
        v => return Err(format!("variant_convert needs a scalar, got {v:?}")),
    };
    for c in field(op, "cases")?.as_array().ok_or("'cases' must be a list")? {
        let to = field(c, "to")?.as_str().ok_or("'to' must be a type name")?;
        let w = field(c, "expect")?;
        let i = e.as_i64();
        let ok = match to {
            "bool" => (i != 0) == w.as_bool().ok_or("bool expect")?,
            "f64" => {
                let (g, w) = (e.as_f64(), f64_of(w)?);
                g.to_bits() == w.to_bits() || (g.is_nan() && w.is_nan())
            }
            "f32" => {
                let g = match e {
                    Elem::F32(x) => x,
                    other => other.as_f64() as f32,
                };
                let w = f32_of(w)?;
                g.to_bits() == w.to_bits() || (g.is_nan() && w.is_nan())
            }
            t => {
                // The low bits of the int64, as the target type holds them.
                let got = match t {
                    "u8" => i64::from(i as u8),
                    "i8" => i64::from(i as i8),
                    "u16" => i64::from(i as u16),
                    "i16" => i64::from(i as i16),
                    "u32" => i64::from(i as u32),
                    "i32" => i64::from(i as i32),
                    "u64" | "i64" => i,
                    other => return Err(format!("unknown target type '{other}'")),
                };
                got == int_or_hex(w, "expect")?
            }
        };
        if !ok {
            return Err(format!(
                "{e:?} as {to}: int64 {i:#x}, double {:?}; expected {w:?}",
                e.as_f64()
            ));
        }
    }
    Ok(())
}

fn check_tail(op: &Json, pos: usize, error: i32) -> R<()> {
    if let Some(p) = op.get("position") {
        if pos as i64 != int(p, "position")? {
            return Err(format!("position {pos}, expected {p:?}"));
        }
    }
    if let Some(e) = op.get("error") {
        let want = e.as_bool().ok_or("'error' must be a boolean")?;
        if (error != 0) != want {
            return Err(format!("stream error state {}, expected {want}", error != 0));
        }
    }
    Ok(())
}

fn variant_write(op: &Json) -> R<()> {
    let v = variant_of(field(op, "variant")?)?;
    let mut w = writer_for(&[&v]);
    let ok = v.write(&mut w);
    let want_ok = op.get("ok").and_then(Json::as_bool).unwrap_or(true);
    if ok != want_ok {
        return Err(format!("write returned {ok}, expected {want_ok}"));
    }
    let want = hex_field(op, "hex")?;
    if w.bytes() != want.as_slice() {
        return Err(format!(
            "bytes {} != expected {}",
            hex_encode(w.bytes()),
            hex_encode(&want)
        ));
    }
    check_tail(op, w.bytes().len(), w.error())?;
    // Round trip for every readable value.
    if ok && op.get("round_trip").and_then(Json::as_bool).unwrap_or(true) {
        let mut res = IdResolver;
        let mut r = ByteReader::with_resolver(&want, &mut res);
        let back = Variant::read(&mut r).ok_or("the written bytes do not read back")?;
        if !same(&back, &v) || r.position() != want.len() {
            return Err(format!("read back {back:?} at {}, expected {v:?}", r.position()));
        }
    }
    Ok(())
}

fn variant_read(op: &Json) -> R<()> {
    let bytes = hex_field(op, "hex")?;
    let mut res = IdResolver;
    let mut r = ByteReader::with_resolver(&bytes, &mut res);
    let mut dest = match op.get("preset") {
        Some(p) => variant_of(p)?,
        None => Variant::Empty,
    };
    let ok = Variant::read_into(&mut r, &mut dest);
    match field(op, "expect")? {
        Json::Null => {
            if ok {
                return Err(format!("read succeeded with {dest:?}, expected a failure"));
            }
        }
        w => {
            if !ok {
                return Err("read failed".into());
            }
            let want = variant_of(w)?;
            if !same(&dest, &want) {
                return Err(format!("read {dest:?}, expected {want:?}"));
            }
        }
    }
    if let Some(p) = op.get("unchanged") {
        // After a failure: the destination keeps its preset value.
        let want = variant_of(p)?;
        if !same(&dest, &want) {
            return Err(format!("destination {dest:?}, expected unchanged {want:?}"));
        }
    }
    check_tail(op, r.position(), r.error())
}

fn holder_of(list: &Json) -> R<PropertyHolder> {
    let mut h = PropertyHolder::new();
    for p in list.as_array().ok_or("props must be a list")? {
        h.add(int(field(p, "id")?, "id")? as u32, variant_of(field(p, "value")?)?);
    }
    Ok(h)
}

fn compare_props(got: &PropertyHolder, want: &Json) -> R<()> {
    let want = want.as_array().ok_or("expect must be a list of {id, value}")?;
    let ids = got.ids();
    if ids.len() != want.len() {
        return Err(format!("{} properties {ids:x?}, expected {}", ids.len(), want.len()));
    }
    for (id, w) in ids.iter().zip(want) {
        let wid = int(field(w, "id")?, "id")? as u32;
        let wv = variant_of(field(w, "value")?)?;
        let gv = got.get(*id).expect("listed");
        if *id != wid || !same(gv, &wv) {
            return Err(format!("property 0x{id:08X} = {gv:?}, expected 0x{wid:08X} = {wv:?}"));
        }
    }
    Ok(())
}

fn holder_write(op: &Json) -> R<()> {
    let h = holder_of(field(op, "props")?)?;
    let mut w = ByteWriter::new();
    if !h.write(&mut w) {
        return Err("write failed".into());
    }
    let want = hex_field(op, "hex")?;
    if w.bytes() != want.as_slice() {
        return Err(format!(
            "bytes {} != expected {}",
            hex_encode(w.bytes()),
            hex_encode(&want)
        ));
    }
    Ok(())
}

fn holder_read(op: &Json) -> R<()> {
    let bytes = hex_field(op, "hex")?;
    let mut h = match op.get("preset") {
        Some(p) => holder_of(p)?,
        None => PropertyHolder::new(),
    };
    let mut res = IdResolver;
    let mut r = ByteReader::with_resolver(&bytes, &mut res);
    let ok = h.read(&mut r);
    let want_ok = op.get("ok").and_then(Json::as_bool).unwrap_or(true);
    if ok != want_ok {
        return Err(format!("read returned {ok}, expected {want_ok}"));
    }
    if let Some(e) = op.get("expect") {
        compare_props(&h, e)?;
    }
    if op.get("rewrite").and_then(Json::as_bool) == Some(true) {
        let mut w = ByteWriter::new();
        h.write(&mut w);
        if w.bytes() != bytes.as_slice() {
            return Err(format!("rewrite {} != {}", hex_encode(w.bytes()), hex_encode(&bytes)));
        }
    }
    check_tail(op, r.position(), r.error())
}

/// Exemplars a stub resource manager knows, keyed by (type, group, instance).
struct Exemplars(HashMap<(u32, u32, u32), Rc<PropertyHolder>>);

impl ExemplarSource for Exemplars {
    fn exemplar(&self, k: ResKey) -> Option<Rc<PropertyHolder>> {
        self.0.get(&(k.type_id, k.group, k.instance)).cloned()
    }
}

fn tgi(j: &Json) -> R<ResKey> {
    Ok(ResKey {
        type_id: int(field(j, "type")?, "type")? as u32,
        group: int(field(j, "group")?, "group")? as u32,
        instance: int(field(j, "instance")?, "instance")? as u32,
    })
}

/// `exemplar_holder`: `exemplars` (stub resource manager: `[{key, props}]`),
/// `hex`; reads (with `preset_parent?`), checks `ok`, `own`, `parent`
/// (key or null) and `lookup` (`[{id, value | null}]`), and with `rewrite`
/// writes it back.
fn exemplar_holder(op: &Json) -> R<()> {
    let mut known = HashMap::new();
    for e in op.get("exemplars").and_then(Json::as_array).unwrap_or(&[]) {
        let k = tgi(field(e, "key")?)?;
        known.insert(
            (k.type_id, k.group, k.instance),
            Rc::new(holder_of(field(e, "props")?)?),
        );
    }
    let src = Exemplars(known);
    let mut h = ExemplarHolder::default();
    if let Some(p) = op.get("preset_parent") {
        let k = tgi(p)?;
        let parent = src.exemplar(k).ok_or("preset parent unknown")?;
        h.parent = Some((k, parent));
    }
    let bytes = hex_field(op, "hex")?;
    let mut r = ByteReader::new(&bytes);
    let ok = h.read(&mut r, &src);
    let want_ok = op.get("ok").and_then(Json::as_bool).unwrap_or(true);
    if ok != want_ok {
        return Err(format!("read returned {ok}, expected {want_ok}"));
    }
    if let Some(e) = op.get("own") {
        compare_props(&h.own, e)?;
    }
    if let Some(p) = op.get("parent") {
        let got = h.parent_key();
        let want = match p {
            Json::Null => None,
            k => Some(tgi(k)?),
        };
        if got != want {
            return Err(format!("parent {got:?}, expected {want:?}"));
        }
    }
    for l in op.get("lookup").and_then(Json::as_array).unwrap_or(&[]) {
        let id = int(field(l, "id")?, "id")? as u32;
        let got = h.get(id);
        match (got, field(l, "value")?) {
            (None, Json::Null) => {}
            (Some(g), Json::Null) => return Err(format!("0x{id:08X} = {g:?}, expected absent")),
            (None, _) => return Err(format!("0x{id:08X} absent")),
            (Some(g), w) => {
                let w = variant_of(w)?;
                if !same(g, &w) {
                    return Err(format!("0x{id:08X} = {g:?}, expected {w:?}"));
                }
            }
        }
    }
    if op.get("rewrite").and_then(Json::as_bool) == Some(true) {
        let mut w = ByteWriter::new();
        h.write(&mut w);
        if w.bytes() != bytes.as_slice() {
            return Err(format!("rewrite {} != {}", hex_encode(w.bytes()), hex_encode(&bytes)));
        }
    }
    check_tail(op, r.position(), r.error())
}

/// `crt_rand`: `seed?` (default 1), `srand?`, `expect` (list of outputs) and/or
/// `shuffle` (`{input, expect, calls}`).
fn crt_rand(op: &Json) -> R<()> {
    let mut g = CrtRand::new(op.get("seed").map(|s| int(s, "seed")).transpose()?.unwrap_or(1) as u32);
    if let Some(s) = op.get("srand") {
        g.srand(int(s, "srand")? as u32);
    }
    if let Some(e) = op.get("expect") {
        for (i, w) in e.as_array().ok_or("expect must be a list")?.iter().enumerate() {
            let got = g.rand();
            if i64::from(got) != int(w, "expect")? {
                return Err(format!("rand #{i} = {got}, expected {w:?}"));
            }
        }
    }
    if let Some(s) = op.get("shuffle") {
        let mut a: Vec<i64> = field(s, "input")?
            .as_array()
            .ok_or("input")?
            .iter()
            .map(|x| int(x, "input"))
            .collect::<R<_>>()?;
        let before = g;
        g.random_shuffle(&mut a);
        let want: Vec<i64> = field(s, "expect")?
            .as_array()
            .ok_or("expect")?
            .iter()
            .map(|x| int(x, "expect"))
            .collect::<R<_>>()?;
        if a != want {
            return Err(format!("shuffle gave {a:?}, expected {want:?}"));
        }
        if let Some(c) = s.get("calls") {
            // Count the calls by replaying the generator.
            let mut probe = before;
            let mut n = 0;
            while probe != g {
                probe.rand();
                n += 1;
                if n > 1_000_000 {
                    return Err("call count not found".into());
                }
            }
            if n != int(c, "calls")? {
                return Err(format!("shuffle made {n} calls, expected {c:?}"));
            }
        }
    }
    Ok(())
}
