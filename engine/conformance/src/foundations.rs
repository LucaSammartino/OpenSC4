// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/foundations.md`: streams, CRC, the
//! class-object store entry format, response curves and random generators.
//! See README.md ("Foundations ops").

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use fc4_core::curve::ResponseCurve;
use fc4_core::rng::{derive_seed as derive, file_seed, FixedSeeds, RzRandom, RzRandomSuper};
use fc4_core::store::{crc32_mpeg2, save_store, Package, INDEX_TYPE, STORE_GROUP};
use fc4_core::stream::{ByteReader, ByteWriter, InStream, OutStream, RefValue, ResKey, SaveObject, SaveRegistry};

use crate::json::Json;

type R<T> = Result<T, String>;

fn field<'a>(op: &'a Json, key: &str) -> R<&'a Json> {
    op.get(key).ok_or_else(|| format!("missing field '{key}'"))
}

fn int(j: &Json, what: &str) -> R<i64> {
    j.as_i64().ok_or_else(|| format!("'{what}' must be an integer"))
}

pub(crate) fn hex_decode(s: &str) -> R<Vec<u8>> {
    let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if clean.len() % 2 != 0 {
        return Err("hex string has odd length".into());
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

pub(crate) fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn close(got: f64, want: f64, rel: f64) -> bool {
    if want == 0.0 {
        got.abs() <= 1e-12
    } else {
        ((got - want) / want).abs() <= rel
    }
}

/// Floats in vectors may be numbers or "0x…" bit patterns (for NaN / -0.0).
fn f32_of(j: &Json) -> R<f32> {
    if let Some(s) = j.as_str() {
        if let Some(h) = s.strip_prefix("0x") {
            return u32::from_str_radix(h, 16)
                .map(f32::from_bits)
                .map_err(|e| e.to_string());
        }
    }
    j.as_f64().map(|v| v as f32).ok_or_else(|| format!("bad float {j:?}"))
}

fn f64_of(j: &Json) -> R<f64> {
    if let Some(s) = j.as_str() {
        if let Some(h) = s.strip_prefix("0x") {
            return u64::from_str_radix(h, 16)
                .map(f64::from_bits)
                .map_err(|e| e.to_string());
        }
    }
    j.as_f64().ok_or_else(|| format!("bad float {j:?}"))
}

/// A store object with a fixed payload (for entry-format vectors).
struct RawObject {
    class: u32,
    payload: Vec<u8>,
}

impl SaveObject for RawObject {
    fn class_id(&self) -> u32 {
        self.class
    }
    fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_bytes(&self.payload)
    }
    fn read(&self, _: &mut dyn InStream) -> bool {
        true
    }
    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

/// Executes a foundations op; `None` if `name` is not one.
pub fn exec(name: &str, op: &Json) -> Option<R<()>> {
    let r = match name {
        "stream_write" => stream_write(op),
        "stream_read" => stream_read(op),
        "crc" => crc(op),
        "store_entry" => store_entry(op),
        "curve" => curve(op),
        "rng" => rng(op),
        "derive_seed" => derive_seed(op),
        _ => return crate::variants::exec(name, op),
    };
    Some(r)
}

fn stream_write(op: &Json) -> R<()> {
    let mut w = ByteWriter::new();
    for item in field(op, "values")?.as_array().ok_or("'values' must be an array")? {
        let (k, v) = item
            .as_object()
            .and_then(|o| o.first())
            .ok_or("each value must be a one-key object")?;
        let ok = match k.as_str() {
            "u8" => w.put_u8(int(v, k)? as u8),
            "i8" => w.put_i8(int(v, k)? as i8),
            "u16" => w.put_u16(int(v, k)? as u16),
            "i16" => w.put_i16(int(v, k)? as i16),
            "u32" => w.put_u32(int(v, k)? as u32),
            "i32" => w.put_i32(int(v, k)? as i32),
            "u64" => w.put_u64(int(v, k)? as u64),
            "i64" => w.put_i64(int(v, k)?),
            "f32" => w.put_f32(f32_of(v)?),
            "f64" => w.put_f64(f64_of(v)?),
            "bool" => w.put_bool(v.as_bool().ok_or("bool")?),
            "str" => w.put_str(v.as_str().ok_or("str")?.as_bytes()),
            "bytes" => w.put_bytes(&hex_decode(v.as_str().ok_or("bytes")?)?),
            "res_key" => {
                let a = v.as_array().ok_or("res_key must be [type, group, instance]")?;
                w.put_res_key(ResKey {
                    type_id: int(&a[0], "type")? as u32,
                    group: int(&a[1], "group")? as u32,
                    instance: int(&a[2], "instance")? as u32,
                })
            }
            "null_ref" => w.put_ref(&RefValue::Null),
            other => return Err(format!("unknown value kind '{other}'")),
        };
        if !ok {
            return Err(format!("write of {k} failed"));
        }
    }
    let want = hex_decode(field(op, "hex")?.as_str().ok_or("'hex' must be a string")?)?;
    if w.bytes() != want.as_slice() {
        return Err(format!(
            "bytes {} != expected {}",
            hex_encode(w.bytes()),
            hex_encode(&want)
        ));
    }
    Ok(())
}

fn stream_read(op: &Json) -> R<()> {
    let data = hex_decode(field(op, "hex")?.as_str().ok_or("'hex' must be a string")?)?;
    let mut r = ByteReader::new(&data);
    for (i, item) in field(op, "reads")?
        .as_array()
        .ok_or("'reads' must be an array")?
        .iter()
        .enumerate()
    {
        let kind = field(item, "get")?.as_str().ok_or("'get' must be a string")?;
        let want = item.get("expect");
        let ctx = |e: String| format!("read #{i} ({kind}): {e}");
        // `expect: null` means the read must fail.
        let must_fail = matches!(want, Some(Json::Null));
        let got: Option<Json> = match kind {
            "u8" => r.get_u8().map(|v| Json::Int(v.into())),
            "u16" => r.get_u16().map(|v| Json::Int(v.into())),
            "u32" => r.get_u32().map(|v| Json::Int(v.into())),
            "i32" => r.get_i32().map(|v| Json::Int(v.into())),
            "u64" => r.get_u64().map(|v| Json::Int(v as i64)),
            "i64" => r.get_i64().map(Json::Int),
            "f32" => r.get_f32().map(|v| Json::Str(format!("0x{:08x}", v.to_bits()))),
            "f64" => r.get_f64().map(|v| Json::Str(format!("0x{:016x}", v.to_bits()))),
            "str" => r.get_str().map(|v| Json::Str(String::from_utf8_lossy(&v).into_owned())),
            "rz_char_str" => {
                let max = int(field(item, "max_bytes")?, "max_bytes")? as usize;
                r.get_rz_char_str(max).map(|v| Json::Str(hex_encode(&v)))
            }
            "skip" => {
                let n = int(field(item, "n")?, "n")? as usize;
                r.skip(n).then_some(Json::Bool(true))
            }
            other => return Err(ctx(format!("unknown read kind '{other}'"))),
        };
        match (got, want) {
            (None, _) if must_fail => {}
            (None, _) => return Err(ctx("read failed".into())),
            (Some(g), _) if must_fail => return Err(ctx(format!("read succeeded with {g:?}, expected failure"))),
            (Some(_), None) => {}
            (Some(g), Some(w)) => {
                let ok = match (kind, &g) {
                    ("f32", Json::Str(bits)) => *bits == format!("0x{:08x}", f32_of(w).map_err(ctx)?.to_bits()),
                    ("f64", Json::Str(bits)) => *bits == format!("0x{:016x}", f64_of(w).map_err(ctx)?.to_bits()),
                    (_, Json::Int(v)) => w.as_i64() == Some(*v),
                    (_, Json::Str(s)) => w.as_str() == Some(s.as_str()),
                    (_, other) => w == other,
                };
                if !ok {
                    return Err(ctx(format!("got {g:?}, expected {w:?}")));
                }
            }
        }
        if let Some(p) = item.get("position") {
            if r.position() as i64 != int(p, "position")? {
                return Err(ctx(format!("position {} != {p:?}", r.position())));
            }
        }
    }
    if let Some(e) = op.get("error") {
        let want = e.as_bool().ok_or("'error' must be a boolean")?;
        if (r.error() != 0) != want {
            return Err(format!("stream error state {} != expected {want}", r.error()));
        }
    }
    Ok(())
}

fn crc(op: &Json) -> R<()> {
    let data = if let Some(a) = op.get("ascii") {
        a.as_str().ok_or("'ascii' must be a string")?.as_bytes().to_vec()
    } else {
        hex_decode(field(op, "hex")?.as_str().ok_or("'hex' must be a string")?)?
    };
    let got = crc32_mpeg2(&data);
    let want = int(field(op, "expect")?, "expect")? as u32;
    if got != want {
        return Err(format!("crc 0x{got:08X} != 0x{want:08X}"));
    }
    Ok(())
}

fn store_entry(op: &Json) -> R<()> {
    let class = int(field(op, "class_id")?, "class_id")? as u32;
    let payload = hex_decode(field(op, "payload_hex")?.as_str().ok_or("payload_hex")?)?;
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    reg.borrow_mut()
        .register(&RefValue::Object(Rc::new(RawObject { class, payload })));
    let mut pkg = Package::new();
    save_store(&reg, &mut pkg);
    let entry = pkg.get(&(class, STORE_GROUP, 0)).ok_or("no class record written")?;
    let want = hex_decode(field(op, "entry_hex")?.as_str().ok_or("entry_hex")?)?;
    if *entry != want {
        return Err(format!("entry {} != expected {}", hex_encode(entry), hex_encode(&want)));
    }
    if let Some(ix) = op.get("index_hex") {
        let want = hex_decode(ix.as_str().ok_or("index_hex")?)?;
        let got = pkg.get(&(INDEX_TYPE, STORE_GROUP, 0)).ok_or("no index written")?;
        if *got != want {
            return Err(format!("index {} != expected {}", hex_encode(got), hex_encode(&want)));
        }
    }
    Ok(())
}

fn curve(op: &Json) -> R<()> {
    let flat: Vec<f32> = field(op, "points")?
        .as_array()
        .ok_or("'points' must be an array")?
        .iter()
        .map(f32_of)
        .collect::<R<_>>()?;
    let c = ResponseCurve::from_flat(&flat);
    if let Some(a) = op.get("absent") {
        let want = a.as_bool().ok_or("'absent' must be a boolean")?;
        if c.is_none() != want {
            return Err(format!("curve absent = {}, expected {want}", c.is_none()));
        }
    }
    let c = c.unwrap_or_default();
    let tol = op.get("tolerance").and_then(Json::as_f64).unwrap_or(1e-6);
    for pair in op.get("eval").and_then(Json::as_array).unwrap_or(&[]) {
        let p = pair.as_array().ok_or("each eval entry must be [x, y]")?;
        let (x, want) = (f32_of(&p[0])?, f64_of(&p[1])?);
        let got = f64::from(c.eval(x));
        // A NaN expectation (any NaN bit pattern) matches any NaN.
        let ok = if want.is_nan() {
            got.is_nan()
        } else {
            close(got, want, tol)
        };
        if !ok {
            return Err(format!("f({x}) = {got}, expected {want}"));
        }
    }
    Ok(())
}

fn rng(op: &Json) -> R<()> {
    let seeds = FixedSeeds {
        wall_clock: op.get("wall_clock_seed").and_then(Json::as_i64).unwrap_or(0) as u32,
        process_clock: op.get("process_clock").and_then(Json::as_i64).unwrap_or(0) as u32,
    };
    let seed = int(field(op, "seed")?, "seed")? as u32;
    let calls = field(op, "calls")?.as_array().ok_or("'calls' must be an array")?;
    match field(op, "generator")?.as_str() {
        Some("RzRandom") => {
            let mut g = RzRandom::new(seed, &seeds);
            for (i, c) in calls.iter().enumerate() {
                let m = field(c, "method")?.as_str().ok_or("method")?;
                let args = rng_args(c);
                let arg = |k: usize| {
                    args.get(k)
                        .copied()
                        .ok_or_else(|| format!("call #{i}: missing args[{k}]"))
                };
                let got: f64 = match m {
                    "next_raw" => f64::from(g.next_raw()),
                    "uniform_u32" => f64::from(g.uniform_u32(arg(0)? as i64 as u32)),
                    "range_i32" => f64::from(g.range_i32(arg(0)? as i64 as i32, arg(1)? as i64 as i32)),
                    "uniform_f64" => g.uniform_f64(),
                    "range_f64" => g.range_f64(arg(0)?, arg(1)?),
                    "gaussian" => g.gaussian(),
                    "range_gaussian" => g.range_gaussian(arg(0)?, arg(1)?),
                    "range_gaussian_fast" => g.range_gaussian_fast(arg(0)?, arg(1)?),
                    "range_i32_gaussian_fast" => {
                        f64::from(g.range_i32_gaussian_fast(arg(0)? as i64 as i32, arg(1)? as i64 as i32))
                    }
                    // Pure mappings (foundations §6.1 / §7 test 16).
                    "uniform_of" => fc4_core::rng::double_uniform_of(arg(0)? as i64 as i32),
                    "gaussian_to_range" => fc4_core::rng::gaussian_to_range(arg(0)?, arg(1)?, arg(2)?),
                    other => return Err(format!("call #{i}: unknown method '{other}'")),
                };
                check_value(i, m, got, c)?;
                if let Some(v) = c.get("lcg") {
                    if i64::from(g.lcg_state()) != int(v, "lcg")? {
                        return Err(format!("call #{i}: lcg 0x{:08X}", g.lcg_state()));
                    }
                }
                if let Some(v) = c.get("mul") {
                    if i64::from(g.mul_state()) != int(v, "mul")? {
                        return Err(format!("call #{i}: mul 0x{:08X}", g.mul_state()));
                    }
                }
            }
        }
        Some("RzRandomSuper") => {
            let mut g = RzRandomSuper::new(seed, &seeds);
            for (i, c) in calls.iter().enumerate() {
                let m = field(c, "method")?.as_str().ok_or("method")?;
                let got: f64 = match m {
                    "next_u32" => f64::from(g.next_u32()),
                    "uniform_u32" => f64::from(
                        g.uniform_u32(int(&c.get("args").and_then(Json::as_array).ok_or("args")?[0], "n")? as u32),
                    ),
                    "uniform_f64" => g.uniform_f64(),
                    "range_f64" => {
                        let a = rng_args(c);
                        g.range_f64(
                            *a.first().ok_or("range_f64: args")?,
                            *a.get(1).ok_or("range_f64: args")?,
                        )
                    }
                    "weighted_choice" => {
                        let w: Vec<f64> = field(c, "weights")?
                            .as_array()
                            .ok_or("weights")?
                            .iter()
                            .filter_map(Json::as_f64)
                            .collect();
                        f64::from(g.weighted_choice(&w))
                    }
                    other => return Err(format!("call #{i}: unknown method '{other}'")),
                };
                check_value(i, m, got, c)?;
            }
        }
        _ => return Err("'generator' must be RzRandom or RzRandomSuper".into()),
    }
    Ok(())
}

/// The `K` of a named file-local generator (foundations §6.3 table).
fn file_seed_of(name: &str) -> R<u32> {
    Ok(match name {
        "building_development" => file_seed::BUILDING_DEVELOPMENT,
        "growth_developer" => file_seed::GROWTH_DEVELOPER,
        "tract_developer" => file_seed::TRACT_DEVELOPER,
        "lot_developer" => file_seed::LOT_DEVELOPER,
        "zone_developer" => file_seed::ZONE_DEVELOPER,
        "lot" => file_seed::LOT,
        "budget" => file_seed::BUDGET,
        "crime" => file_seed::CRIME,
        "pollution" => file_seed::POLLUTION,
        "wealth_requester" => file_seed::WEALTH_REQUESTER,
        "network_lot_manager" => file_seed::NETWORK_LOT_MANAGER,
        "network_construction_crew" => file_seed::NETWORK_CONSTRUCTION_CREW,
        "construction_occupant" => file_seed::CONSTRUCTION_OCCUPANT,
        "base_texture_occupant" => file_seed::BASE_TEXTURE_OCCUPANT,
        "retaining_wall_occupant" => file_seed::RETAINING_WALL_OCCUPANT,
        "prop_occupant" => file_seed::PROP_OCCUPANT,
        "fire_occupant" => file_seed::FIRE_OCCUPANT,
        "earthquake" => file_seed::EARTHQUAKE,
        "pipe_burst" => file_seed::PIPE_BURST,
        "riot" => file_seed::RIOT,
        "automata" => file_seed::AUTOMATA,
        "region_creation" => file_seed::REGION_CREATION,
        "advice" => file_seed::ADVICE,
        other => return Err(format!("unknown generator '{other}'")),
    })
}

/// `derive_seed`: `cases` of `{session, k | generator, expect, next_raw?}`
/// (foundations §6.3); `next_raw` checks the first `NextRaw` words of a
/// `cRZRandom` seeded with the derived value.
fn derive_seed(op: &Json) -> R<()> {
    let cases = field(op, "cases")?.as_array().ok_or("'cases' must be an array")?;
    for (i, c) in cases.iter().enumerate() {
        let session = int(field(c, "session")?, "session")? as u32;
        let k = match (c.get("k"), c.get("generator").and_then(Json::as_str)) {
            (Some(k), None) => int(k, "k")? as u32,
            (None, Some(name)) => {
                let k = file_seed_of(name)?;
                if let Some(want) = c.get("k_expect") {
                    let want = int(want, "k_expect")? as u32;
                    if k != want {
                        return Err(format!("case #{i}: K of {name} is 0x{k:08X}, expected 0x{want:08X}"));
                    }
                }
                k
            }
            _ => return Err(format!("case #{i}: give exactly one of 'k' and 'generator'")),
        };
        let got = derive(session, k);
        let want = int(field(c, "expect")?, "expect")? as u32;
        if got != want {
            return Err(format!(
                "case #{i}: derive(0x{session:08X}, 0x{k:08X}) = 0x{got:08X}, expected 0x{want:08X}"
            ));
        }
        if let Some(words) = c.get("next_raw").and_then(Json::as_array) {
            let seeds = FixedSeeds {
                wall_clock: 0xDEAD_BEEF,
                process_clock: 0xDEAD_BEEF,
            };
            let mut g = RzRandom::new(got, &seeds);
            for (j, w) in words.iter().enumerate() {
                let want = int(w, "next_raw")? as u32;
                let v = g.next_raw();
                if v != want {
                    return Err(format!("case #{i}: NextRaw #{j} = 0x{v:08X}, expected 0x{want:08X}"));
                }
            }
        }
    }
    Ok(())
}

/// A call's numeric `args` (integers given as hex strings are accepted).
fn rng_args(c: &Json) -> Vec<f64> {
    c.get("args")
        .and_then(Json::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_f64().or_else(|| int(v, "arg").ok().map(|n| n as f64)))
                .collect()
        })
        .unwrap_or_default()
}

fn check_value(i: usize, m: &str, got: f64, c: &Json) -> R<()> {
    if let Some(w) = c.get("expect") {
        let want = w
            .as_f64()
            .or_else(|| w.as_i64().map(|v| v as f64))
            .ok_or("expect must be a number")?;
        if got != want {
            return Err(format!("call #{i} ({m}): got {got}, expected {want}"));
        }
    }
    Ok(())
}
