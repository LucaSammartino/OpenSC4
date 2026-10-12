// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The save record (§9.1, version 6).

use std::any::Any;
use std::rc::Rc;

use crate::grid::{Bitmap, GridProvider};
use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::simulator::{PlantNode, PowerState, G};
use super::{
    PowerSim, CLSID_POWER_SIMULATOR, IID_DEPARTMENT_BUDGET, IID_OCCUPANT, IID_POWER_POLE, IID_SIM_GRID, MAX_MAP_CELLS,
    PLANT_TYPES, RECORD_VERSION,
};

/// Preallocation cap for lists read from a stream (the stream bounds the
/// real length).
const PREALLOC: usize = 1 << 12;

impl PowerSim {
    /// Parses the record after the version into a copy of `base`. Host
    /// references taken from the stream are pushed to `taken`.
    fn parse(
        &self,
        inp: &mut dyn InStream,
        version: u16,
        base: &PowerState,
        taken: &mut Vec<u64>,
    ) -> Option<(PowerState, Option<G>)> {
        let h = &*self.host;
        let mut s = base.clone();
        let resolve = |r: RefValue, iid: u32, taken: &mut Vec<u64>| -> Option<u64> {
            if r.is_null() {
                return None;
            }
            let t = h.resolve_ref(&r, iid);
            if let Some(t) = t {
                taken.push(t);
            }
            t
        };
        let r = inp.get_ref_as(IID_DEPARTMENT_BUDGET)?;
        s.department = resolve(r, IID_DEPARTMENT_BUDGET, taken);
        s.ring_index = inp.get_i32()?;
        s.production = inp.get_u32()?;
        s.consumption = inp.get_u32()?;
        s.deficit = if version < 6 {
            // An integer percentage.
            inp.get_i32()? as f32 * 0.01f32
        } else {
            inp.get_f32()?
        };
        s.consumers = inp.get_u32()?;
        s.powered = inp.get_u32()?;
        s.unpowered = inp.get_u32()?;
        s.requirement = inp.get_u32()?;
        s.cost_base = inp.get_i64()?;
        s.last_production = inp.get_u32()?;
        s.last_consumption = inp.get_u32()?;
        s.last_import = inp.get_u32()?;
        s.last_export = inp.get_u32()?;
        s.last_demand = inp.get_u32()?;
        let r = inp.get_ref_as(IID_SIM_GRID)?;
        let grid = if r.is_null() {
            None
        } else {
            GridProvider::<u8>::resolve_grid(h, &r)
        };
        let (nx, nz) = (inp.get_u32()?, inp.get_u32()?);
        if u64::from(nx) * u64::from(nz) > MAX_MAP_CELLS {
            return None;
        }
        s.consumption_map = Vec::with_capacity((nx as usize).min(PREALLOC));
        for _ in 0..nx {
            let mut row = Vec::with_capacity((nz as usize).min(PREALLOC));
            for _ in 0..nz {
                row.push(inp.get_i16()?);
            }
            s.consumption_map.push(row);
        }
        for row in s.ring.iter_mut() {
            for v in row.iter_mut() {
                *v = inp.get_i32()?;
            }
        }
        for v in s.annual.iter_mut() {
            *v = inp.get_i32()?;
        }
        s.plant_count = inp.get_i32()?;
        for v in s.per_type.iter_mut() {
            *v = inp.get_i32()?;
        }
        s.plants = Vec::with_capacity((s.plant_count.max(0) as usize).min(PREALLOC));
        for _ in 0..s.plant_count.max(0) {
            let visited = inp.get_u32()?;
            let r = inp.get_ref_as(IID_OCCUPANT)?;
            if let Some(occupant) = resolve(r, IID_OCCUPANT, taken) {
                s.plants.push(PlantNode { visited, occupant });
            }
        }
        s.pole_count = inp.get_u32()?;
        s.broken_count = inp.get_u32()?;
        s.poles = Vec::with_capacity((s.pole_count as usize).min(PREALLOC));
        for _ in 0..s.pole_count {
            let r = inp.get_ref_as(IID_POWER_POLE)?;
            if let Some(p) = resolve(r, IID_POWER_POLE, taken) {
                s.poles.push(p);
            }
        }
        let words = inp.get_u32()?;
        let (cx, cz) = h.cell_counts();
        let mut maps = Vec::with_capacity(4);
        for _ in 0..4 {
            let mut w = Vec::with_capacity((words as usize).min(PREALLOC));
            for _ in 0..words {
                w.push(inp.get_u32()?);
            }
            maps.push(Bitmap::from_words(cx, cz, &w));
        }
        let mut maps = maps.into_iter();
        s.mark = maps.next()?;
        s.wire = maps.next()?;
        s.powered_current = maps.next()?;
        s.powered_next = maps.next()?;
        s.unused_flag = inp.get_u8()?;
        s.strike_flag = inp.get_u8()?;
        s.strike_start = inp.get_i32()?;
        s.recompute = inp.get_u8()? != 0;
        s.age_plants = inp.get_u8()? != 0;
        s.state = inp.get_u32()?;
        if inp.error() != 0 {
            return None;
        }
        s.strike_flag = 0;
        Some((s, grid))
    }
}

fn put_bitmap(out: &mut dyn OutStream, b: &Bitmap, words: usize) {
    for i in 0..words {
        out.put_u32(b.words().get(i).copied().unwrap_or(0));
    }
}

impl SaveObject for PowerSim {
    fn class_id(&self) -> u32 {
        CLSID_POWER_SIMULATOR
    }

    /// The version-6 record (§9.1); always true.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let h = &*self.host;
        let inner = self.inner.borrow();
        let s = &inner.s;
        out.put_u16(RECORD_VERSION);
        out.put_ref(&s.department.map_or(RefValue::Null, |d| h.object_ref(d)));
        out.put_i32(s.ring_index);
        out.put_u32(s.production);
        out.put_u32(s.consumption);
        out.put_f32(s.deficit);
        out.put_u32(s.consumers);
        out.put_u32(s.powered);
        out.put_u32(s.unpowered);
        out.put_u32(s.requirement);
        out.put_i64(s.cost_base);
        for v in [
            s.last_production,
            s.last_consumption,
            s.last_import,
            s.last_export,
            s.last_demand,
        ] {
            out.put_u32(v);
        }
        out.put_ref(&inner.grid.as_ref().map_or(RefValue::Null, |g| g.save_ref()));
        let nx = s.consumption_map.len();
        let nz = s.consumption_map.first().map_or(0, Vec::len);
        out.put_u32(nx as u32);
        out.put_u32(nz as u32);
        for row in &s.consumption_map {
            for z in 0..nz {
                out.put_i16(row.get(z).copied().unwrap_or(0));
            }
        }
        for row in &s.ring {
            for &v in row {
                out.put_i32(v);
            }
        }
        for &v in &s.annual {
            out.put_i32(v);
        }
        out.put_i32(s.plant_count);
        for &v in &s.per_type[..PLANT_TYPES] {
            out.put_i32(v);
        }
        for p in &s.plants {
            out.put_u32(p.visited);
            out.put_ref(&h.object_ref(p.occupant));
        }
        out.put_u32(s.pole_count);
        out.put_u32(s.broken_count);
        for i in 0..s.pole_count as usize {
            out.put_ref(&s.poles.get(i).map_or(RefValue::Null, |&p| h.object_ref(p)));
        }
        let words = s.mark.words().len();
        out.put_u32(words as u32);
        for b in [&s.mark, &s.wire, &s.powered_current, &s.powered_next] {
            put_bitmap(out, b, words);
        }
        out.put_u8(s.unused_flag);
        out.put_u8(s.strike_flag);
        out.put_i32(s.strike_start);
        out.put_bool(s.recompute);
        out.put_bool(s.age_plants);
        out.put_u32(s.state);
        true
    }

    /// §9.1: sets `running`; false only for a version above 6.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        self.inner.borrow_mut().s.running = true;
        let Some(version) = inp.get_u16() else {
            return true;
        };
        if version > RECORD_VERSION {
            return false;
        }
        let base = self.inner.borrow().s.clone();
        let mut taken = Vec::new();
        match self.parse(inp, version, &base, &mut taken) {
            Some((new, grid)) => {
                if let Some(d) = base.department {
                    self.host.release(d);
                }
                for p in &base.plants {
                    self.host.release(p.occupant);
                }
                for &p in &base.poles {
                    self.host.release(p);
                }
                let mut inner = self.inner.borrow_mut();
                inner.s = new;
                inner.grid = grid;
            }
            None => {
                for t in taken {
                    self.host.release(t);
                }
            }
        }
        true
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token()
    }
}
