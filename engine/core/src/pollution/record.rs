// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The save record (§6).

use std::any::Any;
use std::rc::Rc;

use crate::grid::CellIterator;
use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::simulator::{Counters, Grids, PollutionState};
use super::{
    PollutionSim, CLSID_POLLUTION_SIMULATOR, IID_DEPARTMENT_BUDGET, IID_OCCUPANT, IID_SIM_GRID, RECORD_VERSION,
};

/// A list count at or above this reads no references (§6).
const LIST_SKIP: u32 = 10_000_000;

fn put_counters(out: &mut dyn OutStream, c: &Counters) {
    out.put_i64(c.produced);
    out.put_i64(c.imported);
    out.put_i64(c.exported);
    out.put_f64(c.recycled);
    out.put_i64(c.incinerated);
    out.put_i64(c.wte);
    out.put_i64(c.landfilled);
}

fn get_counters(inp: &mut dyn InStream) -> Option<Counters> {
    Some(Counters {
        produced: inp.get_i64()?,
        imported: inp.get_i64()?,
        exported: inp.get_i64()?,
        recycled: inp.get_f64()?,
        incinerated: inp.get_i64()?,
        wte: inp.get_i64()?,
        landfilled: inp.get_i64()?,
    })
}

impl PollutionSim {
    /// Parses a record into a copy of `base`. Host references taken from
    /// the stream are pushed to `taken` (released by the caller on failure).
    fn parse(&self, inp: &mut dyn InStream, base: &PollutionState, taken: &mut Vec<u64>) -> Option<PollutionState> {
        let h = &*self.host;
        let version = inp.get_u16()?;
        if version > RECORD_VERSION {
            return None;
        }
        let mut s = base.clone();
        let mut grids: Grids = Default::default();
        for g in grids.iter_mut() {
            let r = inp.get_ref_as(IID_SIM_GRID)?;
            *g = if r.is_null() { None } else { h.resolve_grid(&r) };
        }
        s.grids = grids;
        s.cell_x = inp.get_u32()?;
        s.cell_z = inp.get_u32()?;
        for list in s.lists.iter_mut() {
            list.clear();
            let n = inp.get_u32()?;
            if n == 0 || n >= LIST_SKIP {
                continue;
            }
            for _ in 0..n {
                let r = inp.get_ref_as(IID_OCCUPANT)?;
                if r.is_null() {
                    continue;
                }
                if let Some(t) = h.resolve_ref(&r, IID_OCCUPANT) {
                    taken.push(t);
                    list.push(t);
                }
            }
        }
        s.recycling_count = inp.get_u32()?;
        s.avg_air = inp.get_i32()?;
        s.avg_water = inp.get_i32()?;
        s.air_sum = inp.get_i32()?;
        s.water_sum_treated = inp.get_i32()?;
        s.water_sum = inp.get_i32()?;
        s.treated_sum = inp.get_i32()?;
        s.last_water_sum = inp.get_i32()?;
        s.last_treated_sum = inp.get_i32()?;
        s.developed_tracts = inp.get_u32()?;
        if version < 3 {
            inp.get_str()?;
        }
        s.avg_garbage = inp.get_u16()?;
        s.incinerator_capacity = inp.get_i32()?;
        s.wte_capacity = inp.get_i32()?;
        s.recycling_effect = inp.get_f32()?;
        s.garbage_uncollected = inp.get_u8()? != 0;
        s.total_uncollected = inp.get_u32()?;
        s.unused64 = inp.get_i64()?;
        s.last_month_cost = inp.get_i64()?;
        if version >= 4 {
            s.decomposed_last_month = inp.get_u32()?;
        }
        s.last_month = get_counters(inp)?;
        s.month = get_counters(inp)?;
        s.decomposed = inp.get_u32()?;
        s.export_capacity = inp.get_i32()?;
        s.active_landfill_cells = inp.get_u32()?;
        s.tracts_per_day = inp.get_u32()?;
        s.iterator = CellIterator::read(inp)?;
        s.dirty_radiation = inp.get_u8()? != 0;
        s.dirty_air = inp.get_u8()? != 0;
        s.dirty_water = inp.get_u8()? != 0;
        s.dirty_garbage = inp.get_u8()? != 0;
        if version < 3 {
            inp.get_u32()?;
        }
        s.efficiency = inp.get_f32()?;
        s.garbage_supply = inp.get_i32()?;
        s.garbage_capacity = inp.get_i32()?;
        let r = inp.get_ref_as(IID_DEPARTMENT_BUDGET)?;
        s.department = if r.is_null() {
            None
        } else {
            let d = h.resolve_ref(&r, IID_DEPARTMENT_BUDGET);
            if let Some(t) = d {
                taken.push(t);
            }
            d
        };
        if inp.error() != 0 {
            return None;
        }
        s.radiation_sum = 0;
        s.effects_visible = true;
        s.loaded = true;
        Some(s)
    }
}

impl SaveObject for PollutionSim {
    fn class_id(&self) -> u32 {
        CLSID_POLLUTION_SIMULATOR
    }

    /// The §6 record; returns true.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let h = &*self.host;
        let s = self.snapshot();
        out.put_u16(RECORD_VERSION);
        for g in &s.grids {
            out.put_ref(&g.as_ref().map_or(RefValue::Null, |g| g.save_ref()));
        }
        out.put_u32(s.cell_x);
        out.put_u32(s.cell_z);
        for list in &s.lists {
            out.put_u32(list.len() as u32);
            for &occ in list {
                out.put_ref(&h.object_ref(occ));
            }
        }
        out.put_u32(s.recycling_count);
        out.put_i32(s.avg_air);
        out.put_i32(s.avg_water);
        for v in [
            s.air_sum,
            s.water_sum_treated,
            s.water_sum,
            s.treated_sum,
            s.last_water_sum,
            s.last_treated_sum,
        ] {
            out.put_i32(v);
        }
        out.put_u32(s.developed_tracts);
        out.put_u16(s.avg_garbage);
        out.put_i32(s.incinerator_capacity);
        out.put_i32(s.wte_capacity);
        out.put_f32(s.recycling_effect);
        out.put_bool(s.garbage_uncollected);
        out.put_u32(s.total_uncollected);
        out.put_i64(s.unused64);
        out.put_i64(s.last_month_cost);
        out.put_u32(s.decomposed_last_month);
        put_counters(out, &s.last_month);
        put_counters(out, &s.month);
        out.put_u32(s.decomposed);
        out.put_i32(s.export_capacity);
        out.put_u32(s.active_landfill_cells);
        out.put_u32(s.tracts_per_day);
        s.iterator.write(out);
        for d in [s.dirty_radiation, s.dirty_air, s.dirty_water, s.dirty_garbage] {
            out.put_bool(d);
        }
        out.put_f32(s.efficiency);
        out.put_i32(s.garbage_supply);
        out.put_i32(s.garbage_capacity);
        out.put_ref(&s.department.map_or(RefValue::Null, |d| h.object_ref(d)));
        true
    }

    /// The §6 record: versions 0–4. [choice] Nothing changes when the
    /// version is rejected or the stream fails.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        let base = self.snapshot();
        let mut taken = Vec::new();
        match self.parse(inp, &base, &mut taken) {
            Some(new) => {
                for &occ in base.lists.iter().flatten() {
                    self.host.release(occ);
                }
                if let Some(d) = base.department {
                    self.host.release(d);
                }
                *self.gate() = new;
                self.warm_grids();
                true
            }
            None => {
                for t in taken {
                    self.host.release(t);
                }
                false
            }
        }
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
