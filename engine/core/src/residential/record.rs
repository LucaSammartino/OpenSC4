// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The save record (§9): `Write` writes version 7; `Read` accepts every
//! version (§9.2, RS9).
//!
//! `Read` parses into a copy of the state, field by field, in record order,
//! then runs the fix-ups of §9.2 and commits the copy:
//!
//! - department blocks: a version < 5 record converts f0, max funding,
//!   strike efficiency and init funding with `× 0.01f`, a version < 6 record
//!   min grade;
//! - version < 3: iterator `Reset(0, 0)`, leSum := 0, running population :=
//!   0, state 4 → 11; version 3 keeps the running population it had (RS17:
//!   the constructor's 0); version < 7: both auto-budget flags 0;
//! - always: state 1 → 13, 2 → 14, 3 → 15, tuningLoaded := false,
//!   cityStarted := true.
//!
//! *[choice]* (§9.2, RS9 / RS11 / RS14 / RS15 / EH14) A stream error or a
//! **damaged** record makes `Read` return false and leave the simulator as it
//! was (on the original's only path, as constructed, so cityStarted stays
//! false): no field is committed and every reference the parse acquired is
//! released (occupants with `release`, departments with `release_department`;
//! grids are dropped). A record is damaged when a coverage or building-list
//! vector count is not 7 (school) / 3 (hospital), the saved state is above
//! 15, or a tract size is outside 1…256 read as **signed** (so 0 and every
//! negative value are rejected): P, E, or the loaded R$ population grid's own
//! `GetTractSize()`, which is state 4's S (§5.5). The original keeps them all
//! unchecked (EH14).
//!
//! *[choice]* (RS12) Null list entries are dropped, not appended: a null
//! reference or one the host cannot resolve never reaches a list, so no pass
//! sees one and the counts leave them out.
//!
//! *[choice]* (RS10) The references a successful `Read` replaces are released
//! after the commit (`Read` normally runs on a fresh object, where there is
//! nothing to release): the departments with `release_department` (as
//! `Shutdown` releases them), then the occupant list entries with `release`.

use std::any::Any;
use std::rc::Rc;

use crate::grid::{GridElem, GridProvider, ScanIterator, ScanState, TractGrid};
use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::simulator::ResidentialState;
use super::tuning::pct;
use super::{
    state as st, DepartmentBlock, ResidentialHost, ResidentialSim, CLSID_RESIDENTIAL_SIMULATOR, IID_DEPARTMENT_BUDGET,
    IID_OCCUPANT, IID_SIM_GRID, MAX_LIST_COUNT,
};

/// The version `Write` produces (§9.1).
pub const RECORD_VERSION: u16 = 7;

/// The school and hospital vector sizes (§9.2 RS11): any other count is a
/// damaged record.
pub const SCHOOL_VECTOR_LEN: u32 = 7;
pub const HOSPITAL_VECTOR_LEN: u32 = 3;
/// The largest saved state (§5.1, RS14); above it the record is damaged.
pub const MAX_SAVED_STATE: u32 = 15;
/// The largest tract size `Read` accepts for P, E and the loaded R$
/// population grid (§5.5 RS15, EH14); below 1 is damaged too.
pub const MAX_SAVED_TRACT_SIZE: i32 = 256;

/// A tract size `Read` accepts (§5.5 *[choice]*, EH14): 1…256 as signed.
fn tract_size_ok(s: i32) -> bool {
    (1..=MAX_SAVED_TRACT_SIZE).contains(&s)
}

fn put_grid<T: GridElem>(out: &mut dyn OutStream, g: &Option<Rc<dyn TractGrid<T>>>) {
    out.put_ref(&g.as_ref().map_or(RefValue::Null, |g| g.save_ref()));
}

fn put_token(h: &dyn ResidentialHost, out: &mut dyn OutStream, t: Option<u64>) {
    out.put_ref(&t.map_or(RefValue::Null, |t| h.object_ref(t)));
}

fn put_block(h: &dyn ResidentialHost, out: &mut dyn OutStream, b: &DepartmentBlock) {
    out.put_f32(b.f0);
    out.put_f32(b.max_funding);
    out.put_f32(b.strike_efficiency);
    out.put_i32(b.strike_duration);
    out.put_f32(b.min_grade);
    out.put_u8(b.strike_flag);
    out.put_i32(b.strike_start);
    put_token(h, out, b.department1);
    put_token(h, out, b.department2);
    out.put_f32(b.init_funding);
    out.put_f32(b.hundred);
    out.put_u32(b.id);
}

fn put_list(h: &dyn ResidentialHost, out: &mut dyn OutStream, list: &[Option<u64>]) {
    out.put_u32(list.len() as u32);
    for &occ in list {
        put_token(h, out, occ);
    }
}

/// The iterator as §9.1 lays it out: s8 state (0 fresh, 1 running, 2 done),
/// s32 x, s32 z, u32 step, s32 x0, z0, x1, z1.
fn put_iterator(out: &mut dyn OutStream, it: &ScanIterator) {
    out.put_i8(match it.state {
        ScanState::Fresh => 0,
        ScanState::Running => 1,
        ScanState::Done => 2,
    });
    out.put_i32(it.x);
    out.put_i32(it.z);
    out.put_u32(it.step as u32);
    for v in [it.x0, it.z0, it.x1, it.z1] {
        out.put_i32(v);
    }
}

/// The parse in progress: the state copy, the references it acquired and
/// the references it replaced.
struct Parse<'a> {
    h: &'a (dyn ResidentialHost + 'static),
    s: ResidentialState,
    version: u16,
    /// Occupant and department references resolved by the parse: released
    /// when the record is rejected.
    acquired: Vec<u64>,
    acquired_departments: Vec<u64>,
    /// Occupant references the record replaced, released after the commit.
    replaced: Vec<u64>,
    /// Departments the record replaced, released after the commit with
    /// [`ResidentialHost::release_department`], as `Shutdown` releases them.
    replaced_departments: Vec<u64>,
}

impl Parse<'_> {
    fn grid<T: GridElem>(&self, inp: &mut dyn InStream) -> Option<Option<Rc<dyn TractGrid<T>>>>
    where
        dyn ResidentialHost: GridProvider<T>,
    {
        let r = inp.get_ref_as(IID_SIM_GRID)?;
        Some(if r.is_null() {
            None
        } else {
            GridProvider::<T>::resolve_grid(self.h, &r)
        })
    }

    fn grids3<T: GridElem>(&self, inp: &mut dyn InStream) -> Option<[Option<Rc<dyn TractGrid<T>>>; 3]>
    where
        dyn ResidentialHost: GridProvider<T>,
    {
        Some([self.grid(inp)?, self.grid(inp)?, self.grid(inp)?])
    }

    /// `u32 n, n × ref` with n = `N` (any other count is damaged, RS11).
    fn grid_vector<const N: usize>(&self, inp: &mut dyn InStream) -> Option<[Option<Rc<dyn TractGrid<u8>>>; N]> {
        if inp.get_u32()? != N as u32 {
            return None;
        }
        let mut v: [Option<Rc<dyn TractGrid<u8>>>; N] = std::array::from_fn(|_| None);
        for slot in v.iter_mut() {
            *slot = self.grid(inp)?;
        }
        Some(v)
    }

    fn token(&mut self, inp: &mut dyn InStream, iid: u32) -> Option<Option<u64>> {
        let r = inp.get_ref_as(iid)?;
        let t = if r.is_null() { None } else { self.h.resolve_ref(&r, iid) };
        if let Some(t) = t {
            if iid == IID_DEPARTMENT_BUDGET {
                self.acquired_departments.push(t);
            } else {
                self.acquired.push(t);
            }
        }
        Some(t)
    }

    /// One department block, with the version < 5 / < 6 conversions (`×
    /// 0.01f`, RS1).
    fn block(&mut self, inp: &mut dyn InStream, health: bool) -> Option<()> {
        let (old5, old6) = (self.version < 5, self.version < 6);
        let conv = |x: f32, old: bool| if old { pct(x) } else { x };
        let mut b = if health { self.s.health } else { self.s.education };
        b.f0 = conv(inp.get_f32()?, old5);
        b.max_funding = conv(inp.get_f32()?, old5);
        b.strike_efficiency = conv(inp.get_f32()?, old5);
        b.strike_duration = inp.get_i32()?;
        b.min_grade = conv(inp.get_f32()?, old6);
        b.strike_flag = inp.get_u8()?;
        b.strike_start = inp.get_i32()?;
        for second in [false, true] {
            let t = self.token(inp, IID_DEPARTMENT_BUDGET)?;
            let slot = if second { &mut b.department2 } else { &mut b.department1 };
            if let Some(old) = std::mem::replace(slot, t) {
                self.replaced_departments.push(old);
            }
        }
        b.init_funding = conv(inp.get_f32()?, old5);
        b.hundred = inp.get_f32()?;
        b.id = inp.get_u32()?;
        if health {
            self.s.health = b;
        } else {
            self.s.education = b;
        }
        Some(())
    }

    /// `u32 count, count × ref occupant`: a count of 0 or ≥ 10,000,000 reads
    /// nothing more; a null or unresolved reference is dropped (RS12).
    fn list(&mut self, inp: &mut dyn InStream) -> Option<Vec<Option<u64>>> {
        let n = inp.get_u32()?;
        let mut list = Vec::new();
        if n == 0 || n >= MAX_LIST_COUNT {
            return Some(list);
        }
        for _ in 0..n {
            if let Some(t) = self.token(inp, IID_OCCUPANT)? {
                list.push(Some(t));
            }
        }
        Some(list)
    }

    /// Replaces a list of the copy, keeping its old entries for release.
    fn set_list(&mut self, list: Vec<Option<u64>>, which: impl FnOnce(&mut ResidentialState) -> &mut Vec<Option<u64>>) {
        let old = std::mem::replace(which(&mut self.s), list);
        self.replaced.extend(old.into_iter().flatten());
    }

    /// `u32 n, n × list` with n = `N` (RS11).
    fn list_vector<const N: usize>(&mut self, inp: &mut dyn InStream) -> Option<[Vec<Option<u64>>; N]> {
        if inp.get_u32()? != N as u32 {
            return None;
        }
        let mut v: [Vec<Option<u64>>; N] = std::array::from_fn(|_| Vec::new());
        for slot in v.iter_mut() {
            *slot = self.list(inp)?;
        }
        Some(v)
    }

    fn iterator(&mut self, inp: &mut dyn InStream) -> Option<()> {
        let mut it = self.s.iterator.clone();
        // [choice] A state byte other than 0/1/2 reads as done.
        it.state = match inp.get_i8()? {
            0 => ScanState::Fresh,
            1 => ScanState::Running,
            _ => ScanState::Done,
        };
        it.x = inp.get_i32()?;
        it.z = inp.get_i32()?;
        // [choice] A step below 1 is raised to 1 (simgrid §7.4's iterator).
        it.set_step(inp.get_u32()? as i32);
        it.x0 = inp.get_i32()?;
        it.z0 = inp.get_i32()?;
        it.x1 = inp.get_i32()?;
        it.z1 = inp.get_i32()?;
        self.s.iterator = it;
        Some(())
    }

    /// The record in order (§9.1); `None` at the first stream failure or
    /// damaged field.
    fn record(&mut self, inp: &mut dyn InStream) -> Option<()> {
        self.version = inp.get_u16()?;
        self.s.proximity_size = inp.get_i32()?;
        self.s.grids.proximity = self.grids3(inp)?;
        let p = inp.get_i32()?;
        if !tract_size_ok(p) {
            return None; // RS15, EH14
        }
        self.s.population_size = p;
        self.s.grids.population = self.grids3(inp)?;
        // EH14: state 4's S is the R$ population grid's own size.
        if let Some(g) = &self.s.grids.population[0] {
            if !tract_size_ok(g.geometry().tract_size) {
                return None;
            }
        }
        self.s.grids.age = self.grid(inp)?;
        self.s.grids.school_coverage = self.grid_vector::<{ SCHOOL_VECTOR_LEN as usize }>(inp)?;
        self.s.grids.hospital_coverage = self.grid_vector::<{ HOSPITAL_VECTOR_LEN as usize }>(inp)?;
        self.s.grids.school_grade = self.grid(inp)?;
        self.s.grids.hospital_grade = self.grid(inp)?;
        let e = inp.get_i32()?;
        if !tract_size_ok(e) {
            return None; // EH14
        }
        self.s.eq_size = e;
        self.s.grids.eq = self.grids3(inp)?;
        self.s.grids.hq = self.grids3(inp)?;
        self.s.unused_float1 = inp.get_f32()?;
        self.s.unused_float2 = inp.get_f32()?;
        let state = inp.get_u32()?;
        if state > MAX_SAVED_STATE {
            return None; // RS14
        }
        self.s.state = state;
        self.s.dirty = inp.get_u8()? != 0;
        self.s.population = inp.get_i32()?;
        self.block(inp, true)?;
        self.block(inp, false)?;
        let l = self.list(inp)?;
        self.set_list(l, |s| &mut s.lists.pending_school);
        let l = self.list(inp)?;
        self.set_list(l, |s| &mut s.lists.pending_hospital);
        let l = self.list(inp)?;
        self.set_list(l, |s| &mut s.lists.other);
        let v = self.list_vector::<{ SCHOOL_VECTOR_LEN as usize }>(inp)?;
        for (i, l) in v.into_iter().enumerate() {
            self.set_list(l, |s| &mut s.lists.school_groups[i]);
        }
        let v = self.list_vector::<{ HOSPITAL_VECTOR_LEN as usize }>(inp)?;
        for (i, l) in v.into_iter().enumerate() {
            self.set_list(l, |s| &mut s.lists.hospital_groups[i]);
        }
        let l = self.list(inp)?;
        self.set_list(l, |s| &mut s.lists.health_affecting);
        if self.version >= 3 {
            self.iterator(inp)?;
            self.s.le_sum = inp.get_u64()?;
        }
        // Version 3 keeps the running population it had (RS17).
        if self.version >= 4 {
            self.s.running_population = inp.get_i32()?;
        }
        if self.version >= 7 {
            self.s.school_auto_budget = inp.get_u8()? != 0;
            self.s.hospital_auto_budget = inp.get_u8()? != 0;
        }
        Some(())
    }

    /// The fix-ups of §9.2 after a complete parse.
    fn finish(&mut self) {
        let s = &mut self.s;
        if self.version < 3 {
            s.iterator.reset(0, 0);
            s.le_sum = 0;
            s.running_population = 0;
            if s.state == st::POPULATION {
                s.state = st::HISTOGRAM_RESET;
            }
        }
        if self.version < 7 {
            s.school_auto_budget = false;
            s.hospital_auto_budget = false;
        }
        // The temporary grid is not saved: the proximity states re-seed.
        s.state = match s.state {
            st::PROXIMITY_LOW => st::SEED_LOW,
            st::PROXIMITY_MEDIUM => st::SEED_MEDIUM,
            st::PROXIMITY_HIGH => st::SEED_HIGH,
            other => other,
        };
        s.tuning_loaded = false;
        s.city_started = true;
    }
}

impl SaveObject for ResidentialSim {
    fn class_id(&self) -> u32 {
        CLSID_RESIDENTIAL_SIMULATOR
    }

    /// The version-7 record of §9.1; returns true (AL = 1).
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let h = &*self.host;
        let s = self.snapshot();
        let g = &s.grids;
        out.put_u16(RECORD_VERSION);
        out.put_i32(s.proximity_size);
        g.proximity.iter().for_each(|x| put_grid(out, x));
        out.put_i32(s.population_size);
        g.population.iter().for_each(|x| put_grid(out, x));
        put_grid(out, &g.age);
        out.put_u32(g.school_coverage.len() as u32);
        g.school_coverage.iter().for_each(|x| put_grid(out, x));
        out.put_u32(g.hospital_coverage.len() as u32);
        g.hospital_coverage.iter().for_each(|x| put_grid(out, x));
        put_grid(out, &g.school_grade);
        put_grid(out, &g.hospital_grade);
        out.put_i32(s.eq_size);
        g.eq.iter().for_each(|x| put_grid(out, x));
        g.hq.iter().for_each(|x| put_grid(out, x));
        out.put_f32(s.unused_float1);
        out.put_f32(s.unused_float2);
        out.put_u32(s.state);
        out.put_bool(s.dirty || s.state != st::IDLE);
        out.put_i32(s.population);
        put_block(h, out, &s.health);
        put_block(h, out, &s.education);
        let l = &s.lists;
        put_list(h, out, &l.pending_school);
        put_list(h, out, &l.pending_hospital);
        put_list(h, out, &l.other);
        out.put_u32(l.school_groups.len() as u32);
        l.school_groups.iter().for_each(|x| put_list(h, out, x));
        out.put_u32(l.hospital_groups.len() as u32);
        l.hospital_groups.iter().for_each(|x| put_list(h, out, x));
        put_list(h, out, &l.health_affecting);
        put_iterator(out, &s.iterator);
        out.put_u64(s.le_sum);
        out.put_i32(s.running_population);
        out.put_bool(s.school_auto_budget);
        out.put_bool(s.hospital_auto_budget);
        true
    }

    /// §9.2: every version is accepted; a stream error or a damaged record
    /// returns false and leaves the simulator unchanged (RS9 *[choice]*).
    fn read(&self, inp: &mut dyn InStream) -> bool {
        let mut p = Parse {
            h: &*self.host,
            s: self.snapshot(),
            version: 0,
            acquired: Vec::new(),
            acquired_departments: Vec::new(),
            replaced: Vec::new(),
            replaced_departments: Vec::new(),
        };
        if p.record(inp).is_none() {
            let Parse {
                s,
                acquired,
                acquired_departments,
                ..
            } = p;
            // The copy holds the acquired grids; drop it before releasing.
            drop(s);
            for t in acquired_departments {
                self.host.release_department(t);
            }
            for t in acquired {
                self.host.release(t);
            }
            return false;
        }
        p.finish();
        let Parse {
            s,
            replaced,
            replaced_departments,
            ..
        } = p;
        *self.s.borrow_mut() = s;
        for t in replaced_departments {
            self.host.release_department(t);
        }
        for t in replaced {
            self.host.release(t);
        }
        true
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
