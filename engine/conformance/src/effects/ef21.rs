// SPDX-License-Identifier: GPL-3.0-or-later
//! Language-neutral EF21 observation and retained-memory fixtures. These are
//! synthetic contract tests, not native interception or real-game goldens.
use crate::json::Json;
use crate::runner::field;
use crate::scalar::{arr, bool_of, opt, str_of, u32_of, R};
use fc4_core::effects::capture::observation::{Event, Observation, Operation};
use fc4_core::effects::capture::payload::{copy_builtins, Value};
use fc4_core::effects::capture::Memory;

fn number(j: &Json, key: &str) -> R<u32> {
    u32_of(field(j, key)?, key)
}
fn flag(j: &Json, key: &str) -> R<bool> {
    bool_of(field(j, key)?, key)
}
fn bytes(j: &Json, key: &str) -> R<Vec<u8>> {
    Ok(str_of(field(j, key)?, key)?.as_bytes().to_vec())
}
fn operation(j: &Json) -> R<Operation> {
    Ok(match str_of(field(j, "scope")?, "scope")? {
        "init" => Operation::Init,
        "bootstrap" => Operation::Bootstrap,
        "tick" => Operation::Tick,
        "begin" => Operation::Begin,
        "parse" => Operation::Parse,
        "end" => Operation::End,
        other => return Err(format!("unknown observation scope {other}")),
    })
}

pub(super) fn observation(op: &Json) -> R<()> {
    let owner = number(op, "owner")?;
    let mut o = Observation::attach(owner, flag(op, "late")?);
    for step in arr(field(op, "actions")?, "actions")? {
        let thread = opt(step, "thread").map_or(Ok(owner), |j| u32_of(j, "thread"))?;
        let method = str_of(field(step, "method")?, "method")?;
        let event = match method {
            "enter" => Event::Enter(operation(step)?),
            "leave" => Event::Leave(operation(step)?),
            "request" => Event::Request {
                key: [number(step, "type")?, number(step, "group")?, number(step, "instance")?],
                iid: number(step, "iid")?,
                success: flag(step, "success")?,
                resource: opt(step, "resource").map(|j| u32_of(j, "resource")).transpose()?,
            },
            "read" => Event::Read {
                resource: number(step, "resource")?,
                success: flag(step, "success")?,
            },
            "release" => Event::Release(number(step, "resource")?),
            "publish_before" => Event::PublishBefore(number(step, "resource")?),
            "publish_after" => Event::PublishAfter(number(step, "resource")?),
            "register" => Event::Register {
                path: bytes(step, "path")?,
                flags: number(step, "flags")?,
            },
            "parse_result" => Event::ParseResult {
                success: flag(step, "success")?,
                diagnostic: bytes(step, "diagnostic")?,
            },
            "message" => Event::Message(number(step, "id")?),
            "modified" => Event::Modified(flag(step, "value")?),
            "expect" => {
                let available = o.retained_available(thread, flag(step, "writers_excluded")?);
                if available != flag(step, "retained")?
                    || o.history_available() != flag(step, "history")?
                    || o.generation != number(step, "generation")? as u64
                {
                    return Err(format!("observation state differs: {o:?}"));
                }
                if let Some(r) = opt(step, "resource") {
                    if o.borrowed(u32_of(r, "resource")?) != flag(step, "borrowed")? {
                        return Err("borrowed lifetime differs".into());
                    }
                }
                if let Some(n) = opt(step, "events") {
                    if o.trace.len() != u32_of(n, "events")? as usize {
                        return Err("event count differs".into());
                    }
                }
                if let Some(flags) = opt(step, "registration_flags") {
                    let actual = Json::Array(
                        o.trace
                            .iter()
                            .filter_map(|(_, e)| match e {
                                Event::Register { flags, .. } => Some(Json::Int(*flags as i64)),
                                _ => None,
                            })
                            .collect(),
                    );
                    if &actual != flags {
                        return Err("registration flags/order differ".into());
                    }
                }
                if let Some(reads) = opt(step, "reads") {
                    let actual = Json::Array(
                        o.trace
                            .iter()
                            .filter_map(|(_, e)| match e {
                                Event::Read { success, .. } => Some(Json::Bool(*success)),
                                _ => None,
                            })
                            .collect(),
                    );
                    if &actual != reads {
                        return Err("stream read history differs".into());
                    }
                }
                continue;
            }
            other => return Err(format!("unknown observation method {other}")),
        };
        o.record(thread, event);
    }
    Ok(())
}

struct Image(Vec<u8>);
impl Memory for Image {
    fn read(&mut self, address: u32, out: &mut [u8]) -> bool {
        let Some(end) = (address as usize).checked_add(out.len()) else {
            return false;
        };
        let Some(b) = self.0.get(address as usize..end) else {
            return false;
        };
        out.copy_from_slice(b);
        true
    }
}

fn json_value(value: &Value) -> Json {
    match value {
        Value::Byte(n) => Json::Int(*n as i64),
        Value::Half(n) => Json::Int(*n as i64),
        Value::Word(n) => Json::Int(*n as i64),
        Value::Indeterminate(n) => Json::Object(vec![("indeterminate".into(), Json::Int(*n as i64))]),
        Value::Bytes(b) => Json::Array(b.iter().map(|n| Json::Int(*n as i64)).collect()),
        Value::Words(w) => Json::Array(w.iter().map(|n| Json::Int(*n as i64)).collect()),
        Value::Rows(rows) => Json::Array(
            rows.iter()
                .map(|r| Json::Object(r.iter().map(|(k, v)| (k.to_string(), json_value(v))).collect()))
                .collect(),
        ),
    }
}

pub(super) fn payload(op: &Json) -> R<()> {
    let mut image = Image(vec![0; number(op, "size")? as usize]);
    for write in arr(field(op, "writes")?, "writes")? {
        let address = number(write, "at")? as usize;
        let data = if let Some(v) = opt(write, "words") {
            arr(v, "words")?
                .iter()
                .map(|j| u32_of(j, "word"))
                .collect::<R<Vec<_>>>()?
                .into_iter()
                .flat_map(u32::to_le_bytes)
                .collect::<Vec<_>>()
        } else {
            bytes(write, "text")?
        };
        let end = address.checked_add(data.len()).ok_or("write overflow")?;
        image
            .0
            .get_mut(address..end)
            .ok_or("write outside image")?
            .copy_from_slice(&data);
    }
    let result = copy_builtins(image, number(op, "collection")?);
    if result.is_some() != flag(op, "readable")? {
        return Err("payload readability differs".into());
    }
    let Some(result) = result else {
        return Ok(());
    };
    if result.supported() != flag(op, "supported")? {
        return Err("payload support differs".into());
    }
    for check in arr(field(op, "checks")?, "checks")? {
        let family = result
            .families
            .get(number(check, "family")? as usize)
            .ok_or("family missing")?;
        if let Some(n) = opt(check, "slots") {
            if family.slots.len() != u32_of(n, "slots")? as usize {
                return Err("slot count differs".into());
            }
        }
        if let Some(v) = opt(check, "names") {
            let names = Json::Array(
                family
                    .names
                    .iter()
                    .map(|(n, i)| {
                        Json::Array(vec![
                            Json::String(String::from_utf8_lossy(n).into_owned()),
                            Json::Int(*i as i64),
                        ])
                    })
                    .collect(),
            );
            if &names != v {
                return Err("names differ".into());
            }
        }
        if let Some(slot) = opt(check, "slot") {
            let slot = family.slots.get(u32_of(slot, "slot")? as usize).ok_or("slot missing")?;
            if let Some(expected) = opt(check, "value") {
                let value = slot
                    .payload
                    .as_ref()
                    .and_then(|p| p.get(&number(check, "offset").ok()?));
                let actual = value.map(json_value).ok_or("field missing")?;
                if &actual != expected {
                    return Err(format!("payload field differs: {actual:?} != {expected:?}"));
                }
            }
            if let Some(offset) = opt(check, "absent") {
                if slot
                    .payload
                    .as_ref()
                    .is_some_and(|p| p.contains_key(&u32_of(offset, "absent").unwrap_or(u32::MAX)))
                {
                    return Err("excluded payload field retained".into());
                }
            }
        }
    }
    Ok(())
}
