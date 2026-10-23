// SPDX-License-Identifier: GPL-3.0-or-later
//! Public lexical/expression vectors. Stateful owner vectors are executable
//! integration scenarios in tests/sc_resources.rs (SCR-V05–21, V25–30).
use crate::json::Json;
use fc4_core::sc_resources::{lexical::*, Error};

fn text<'a>(op: &'a Json, name: &str) -> Result<&'a str, String> {
    op.get(name)
        .and_then(Json::as_str)
        .ok_or_else(|| format!("missing string {name}"))
}
fn number(v: f64) -> Json {
    Json::Float(v)
}
fn array(v: impl IntoIterator<Item = f32>) -> Json {
    Json::Array(v.into_iter().map(|v| number(f64::from(v))).collect())
}
pub fn exec(name: &str, op: &Json) -> Option<Result<(), String>> {
    if name != "scr_lexical" {
        return None;
    }
    Some((|| {
        let input = text(op, "input")?.as_bytes();
        let actual: Result<Json, Error> = match text(op, "method")? {
            "split" => split(input).map(|v| {
                Json::Array(
                    v.iter()
                        .map(|t| Json::Str(String::from_utf8_lossy(&t.bytes).into()))
                        .collect(),
                )
            }),
            "float" => parse_float(input).map(number),
            "int" => parse_int(input).map(|n| Json::Int(i64::from(n))),
            "uint" => parse_uint(input).map(|n| Json::Int(i64::from(n))),
            "bool" => parse_bool(input).map(Json::Bool),
            "color" => color(input).map(array),
            "color_alpha" => color_alpha(input).map(array),
            "vector2" => vector2(input).map(array),
            "vector3" => vector3(input).map(array),
            other => return Err(format!("unknown lexical method {other}")),
        };
        if let Some(expected) = op.get("error") {
            return match actual {
                Err(Error::Parse { code, .. })
                    if expected.as_i64() == Some(i64::from(code)) || expected.as_str() == Some("parse") =>
                {
                    Ok(())
                }
                _ => Err(format!("expected error {expected:?}, got {actual:?}")),
            };
        }
        let actual = actual.map_err(|e| format!("{e:?}"))?;
        let expected = op.get("expect").ok_or("missing expect")?;
        fn equal(a: &Json, b: &Json) -> bool {
            if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
                return a == b;
            }
            if let (Some(a), Some(b)) = (a.as_array(), b.as_array()) {
                return a.len() == b.len() && a.iter().zip(b).all(|(a, b)| equal(a, b));
            }
            a == b
        }
        if equal(&actual, expected) {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, got {actual:?}"))
        }
    })())
}
