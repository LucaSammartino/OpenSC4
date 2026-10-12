// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `development_eval`: the shared growth helpers of `fc4_core::development`
//! (building-development.md §4.7.3).

use fc4_core::development::{
    building_fits_in_zone, building_map_key, developer_type, growth_stage_fits_in_zone, wealth_of,
};

use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{arg, check_expect, i64_of, ji, str_of, R};

fn a_i64(op: &Json, i: usize) -> R<i64> {
    i64_of(arg(op, i)?, "args")
}

/// `development_eval`: `fn` (`developer_type [p, w]`, `wealth [devType]`,
/// `building_map_key [p, w]`, `building_fits_in_zone [bz, cz]`,
/// `growth_stage_fits_in_zone [stage, zone]`), `args`, `expect`.
fn eval(op: &Json) -> R<()> {
    let f = str_of(field(op, "fn")?, "fn")?;
    let got = match f {
        "developer_type" => ji(developer_type(a_i64(op, 0)? as u32, a_i64(op, 1)? as u32)),
        "wealth" => ji(wealth_of(a_i64(op, 0)? as u32)),
        "building_map_key" => ji(building_map_key(a_i64(op, 0)? as u32, a_i64(op, 1)? as u32)),
        "building_fits_in_zone" => Json::Bool(building_fits_in_zone(a_i64(op, 0)? as i32, a_i64(op, 1)? as i32)),
        "growth_stage_fits_in_zone" => {
            Json::Bool(growth_stage_fits_in_zone(a_i64(op, 0)? as u32, a_i64(op, 1)? as i32))
        }
        other => return Err(format!("unknown development fn '{other}'")),
    };
    check_expect(op, got)
}

pub(crate) fn exec(_host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    match name {
        "development_eval" => Some(eval(op)),
        _ => None,
    }
}
