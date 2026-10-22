// SPDX-License-Identifier: GPL-3.0-or-later
//! Direct owner vectors: state is already imported. This runner deliberately
//! does not pretend to execute Lua or implement script-system containers.
use std::collections::BTreeMap;
use std::sync::Arc;

use fc4_core::occupant_groups::{GroupState, KeyEffect, OccupantGroupTemplate};

use crate::json::Json;
use crate::runner::field;
use crate::scalar::{arr, num, R};

fn text<'a>(j: &'a Json, key: &str) -> R<&'a str> {
    field(j, key)?.as_str().ok_or_else(|| format!("{key}: expected string"))
}

fn uint(j: &Json) -> R<u32> {
    j.as_i64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| "expected u32".into())
}

fn number(j: &Json) -> R<f64> {
    match j.as_str() {
        Some("NaN") => Ok(f64::NAN),
        Some("+inf") => Ok(f64::INFINITY),
        Some("-inf") => Ok(f64::NEG_INFINITY),
        _ => num(j).ok_or_else(|| "expected number".into()),
    }
}

fn ids(j: &Json) -> R<Vec<u32>> {
    arr(j, "ids")?.iter().map(uint).collect()
}

fn effect(j: &Json) -> R<KeyEffect> {
    Ok(KeyEffect {
        fx: text(j, "fx")?.into(),
        bone: text(j, "bone")?.into(),
        controller: text(j, "controller")?.into(),
        flags: uint(field(j, "flags")?)?,
    })
}

fn set(state: &mut GroupState, j: &Json) -> R<()> {
    for (key, value) in j.as_object().ok_or("expected state object")? {
        match key.as_str() {
            "id" => state.group_id = uint(value)?,
            "class" => state.class_id = uint(value)?,
            "source" => state.source_building = uint(value)?,
            "percentage" => state.percentage = number(value)? as f32,
            _ => return Err(format!("unknown owner state field {key}")),
        }
    }
    Ok(())
}

fn expect(group: &OccupantGroupTemplate, j: &Json) -> R<()> {
    for (key, value) in j.as_object().ok_or("expected query object")? {
        let matches = match key.as_str() {
            "class" => group.get_class_id() == uint(value)?,
            "source" => group.get_source_building_group() == uint(value)?,
            "depth" => OccupantGroupTemplate::get_group_hierarchy_depth(Some(group)) == uint(value)?,
            "ids" => group.get_group_id_variant() == ids(value)?,
            "contains" => ids(value)?.iter().all(|id| group.is_occupant_group(*id)),
            "excludes" => ids(value)?.iter().all(|id| !group.is_occupant_group(*id)),
            _ => return Err(format!("unknown owner query {key}")),
        };
        if !matches {
            return Err(format!("query {key}: expected {value:?}"));
        }
    }
    Ok(())
}

pub(crate) fn run(v: &Json, mut shared: impl FnMut(&Json) -> R<()>) -> R<usize> {
    let steps = arr(field(v, "steps")?, "steps")?;
    let mut groups = BTreeMap::<String, Arc<OccupantGroupTemplate>>::new();
    for (index, step) in steps.iter().enumerate() {
        let result = (|| -> R<()> {
            let op = text(step, "op")?;
            // The workspace's turbo-off check injects this shared harness
            // operation. Execute it in the real harness, not as a no-op.
            if op == "set_turbo" {
                return shared(step);
            }
            let name = text(step, "group")?;
            if op == "new" {
                if groups.contains_key(name) {
                    return Err("duplicate fixture group".into());
                }
                let seed = uint(field(step, "session_seed")?)?;
                let group = Arc::new(OccupantGroupTemplate::new(seed));
                group.edit(|s| set(s, field(step, "state")?))?;
                groups.insert(name.into(), group);
                return Ok(());
            }
            let group = groups.get(name).ok_or_else(|| format!("unknown group {name}"))?;
            match op {
                "set" => group.edit(|s| set(s, field(step, "state")?))?,
                "parent" => {
                    let parent = text(step, "parent")?;
                    let parent = groups.get(parent).ok_or("unknown parent")?;
                    group.edit(|s| s.parent = Arc::downgrade(parent));
                }
                "expect" => expect(group, field(step, "queries")?)?,
                "filter" => {
                    let property = match field(step, "property")? {
                        Json::Null => None,
                        j => Some(ids(j)?),
                    };
                    let draw = number(field(step, "draw")?)?;
                    let mut draws = 0;
                    let actual = group.is_occupant_included_with_draw(property.as_deref(), || {
                        draws += 1;
                        draw
                    });
                    let expected = field(step, "included")?.as_bool().ok_or("included: expected bool")?;
                    if actual != expected || draws != uint(field(step, "draws")?)? {
                        return Err(format!("filter: included {actual}, draws {draws}"));
                    }
                }
                "effect_state" => {
                    let command = uint(field(step, "command")?)?;
                    let value = effect(field(step, "effect")?)?;
                    // Fixture seeding, NOT ImportKeyEffectsTable: no import
                    // filtering, defaulting or duplicate policy is hidden here.
                    group.edit(|s| s.key_effects.insert(command as u8, value));
                }
                "key" => {
                    let sentinel = KeyEffect {
                        fx: "sentinel-fx".into(),
                        bone: "sentinel-bone".into(),
                        controller: "sentinel-controller".into(),
                        flags: u32::MAX,
                    };
                    let mut output = sentinel.clone();
                    let found = group.get_key_effect(uint(field(step, "command")?)?, &mut output);
                    let expected = field(step, "expected")?;
                    let matches = if matches!(expected, Json::Null) {
                        !found && output == sentinel
                    } else {
                        found && output == effect(expected)?
                    };
                    if !matches {
                        return Err(format!("key: found {found}, output {output:?}"));
                    }
                }
                "clear_effects" => group.clear_key_effects(),
                _ => return Err(format!("unknown owner operation {op}")),
            }
            Ok(())
        })();
        result.map_err(|error| format!("owner step {index}: {error}"))?;
    }
    Ok(steps.len())
}

#[cfg(test)]
mod tests {
    #[test]
    fn rejects_incorrect_public_expectations_and_unknown_operations() {
        let prefix = r#"{"format":1,"suite":"occupant_groups","steps":[
            {"op":"new","group":"g","session_seed":0,"state":{"id":3}},"#;
        for tail in [
            r#"{"op":"expect","group":"g","queries":{"ids":[4]}}]}"#,
            r#"{"op":"expect","group":"g","queries":{"unknown":0}}]}"#,
            r#"{"op":"filter","group":"g","property":[3],"draw":0,"included":false,"draws":0}]}"#,
            r#"{"op":"key","group":"g","command":7,"expected":{"fx":"wrong","bone":"","controller":"","flags":0}}]}"#,
            r#"{"op":"unknown","group":"g"}]}"#,
        ] {
            assert!(crate::run_vector_text(&format!("{prefix}{tail}")).is_err());
        }
    }
}
