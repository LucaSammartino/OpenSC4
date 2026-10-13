// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The save record (§9): `Write` writes version 7; `Read` accepts every
//! version, never fails and ignores stream errors (§9.2).
//!
//! `Read` parses into a copy of the state, field by field, in record order.
//! At the first stream failure it stops and **keeps what it read** (a field
//! is either read whole or left as it was; a list keeps the references read
//! before the failure). Then the fix-ups of §9.2 run and the copy replaces
//! the state:
//!
//! - department blocks: a version < 5 record converts f0, max funding,
//!   strike efficiency and init funding with `× 0.01f`, a version < 6 record
//!   min grade — each field as it is read, so a field the stream did not
//!   deliver is not converted ([choice]);
//! - version < 3: iterator `Reset(0, 0)`, leSum := 0, running population :=
//!   0, state 4 → 11; version < 7: both auto-budget flags 0;
//! - always: state 1 → 13, 2 → 14, 3 → 15, tuningLoaded := false,
//!   cityStarted := true.
//!
//! [choice] A record whose version cannot be read gets only the "always"
//! fix-ups. References the record replaces (the occupant list entries, a
//! department) are released ([choice]: §9.2's "cleared first" leaves this
//! open; `Read` normally runs on a fresh object, where there is nothing to
//! release). A coverage or building-list vector with more entries than the
//! simulator has (7 school, 3 hospital) reads the extra ones and drops them;
//! one with fewer leaves the rest null / empty ([choice]).

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

/// The parse in progress: the state copy plus the references it replaced.
struct Parse<'a> {
    h: &'a (dyn ResidentialHost + 'static),
    s: ResidentialState,
    version: Option<u16>,
    /// Host references the record replaced, released after the commit.
    replaced: Vec<u64>,
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

    fn grids3<T: GridElem>(&self, inp: &mut dyn InStream, dest: &mut [Option<Rc<dyn TractGrid<T>>>; 3]) -> Option<()>
    where
        dyn ResidentialHost: GridProvider<T>,
    {
        for slot in dest.iter_mut() {
            *slot = self.grid(inp)?;
        }
        Some(())
    }

    /// `u32 n, n × ref`: the first `N` go to the vector, the rest are
    /// dropped; missing entries are null.
    fn grid_vector<const N: usize>(&self, inp: &mut dyn InStream) -> Option<[Option<Rc<dyn TractGrid<u8>>>; N]> {
        let n = inp.get_u32()?;
        let mut v: [Option<Rc<dyn TractGrid<u8>>>; N] = std::array::from_fn(|_| None);
        for i in 0..n {
            let g = self.grid(inp)?;
            if let Some(slot) = v.get_mut(i as usize) {
                *slot = g;
            }
        }
        Some(v)
    }

    fn token(&mut self, inp: &mut dyn InStream, iid: u32) -> Option<Option<u64>> {
        let r = inp.get_ref_as(iid)?;
        Some(if r.is_null() { None } else { self.h.resolve_ref(&r, iid) })
    }

    /// One department block, field by field, with the version < 5 / < 6
    /// conversions (`× 0.01f`, RS1) applied to each converted field as it
    /// is read.
    fn block(&mut self, inp: &mut dyn InStream, health: bool) -> Option<()> {
        let v = self.version.unwrap_or(RECORD_VERSION);
        let (old5, old6) = (v < 5, v < 6);
        let conv = |x: f32, old: bool| if old { pct(x) } else { x };
        let mut b = if health { self.s.health } else { self.s.education };
        // Each field is stored the moment it is read, so a failure keeps the
        // fields read so far.
        let result = (|| -> Option<()> {
            b.f0 = conv(inp.get_f32()?, old5);
            b.max_funding = conv(inp.get_f32()?, old5);
            b.strike_efficiency = conv(inp.get_f32()?, old5);
            b.strike_duration = inp.get_i32()?;
            b.min_grade = conv(inp.get_f32()?, old6);
            b.strike_flag = inp.get_u8()?;
            b.strike_start = inp.get_i32()?;
            Some(())
        })();
        let store = |p: &mut Parse<'_>, b: DepartmentBlock| {
            if health {
                p.s.health = b;
            } else {
                p.s.education = b;
            }
        };
        store(self, b);
        result?;
        for second in [false, true] {
            let t = self.token(inp, IID_DEPARTMENT_BUDGET)?;
            let slot = if second { &mut b.department2 } else { &mut b.department1 };
            if let Some(old) = std::mem::replace(slot, t) {
                self.replaced.push(old);
            }
            store(self, b);
        }
        let result = (|| -> Option<()> {
            b.init_funding = conv(inp.get_f32()?, old5);
            b.hundred = inp.get_f32()?;
            b.id = inp.get_u32()?;
            Some(())
        })();
        store(self, b);
        result
    }

    /// `u32 count, count × ref occupant`: the list is cleared first (its old
    /// references released); a count of 0 or ≥ 10,000,000 reads nothing
    /// more; null references are appended.
    fn list(&mut self, inp: &mut dyn InStream, which: ListId) -> Option<()> {
        let old = self.list_opt(which).map(std::mem::take).unwrap_or_default();
        self.replaced.extend(old.into_iter().flatten());
        let n = inp.get_u32()?;
        if n == 0 || n >= MAX_LIST_COUNT {
            return Some(());
        }
        for _ in 0..n {
            let t = self.token(inp, IID_OCCUPANT)?;
            if let Some(list) = self.list_opt(which) {
                list.push(t);
            }
        }
        Some(())
    }

    fn list_opt(&mut self, which: ListId) -> Option<&mut Vec<Option<u64>>> {
        let l = &mut self.s.lists;
        match which {
            ListId::PendingSchool => Some(&mut l.pending_school),
            ListId::PendingHospital => Some(&mut l.pending_hospital),
            ListId::Other => Some(&mut l.other),
            ListId::School(i) => l.school_groups.get_mut(i),
            ListId::Hospital(i) => l.hospital_groups.get_mut(i),
            ListId::HealthAffecting => Some(&mut l.health_affecting),
        }
    }

    /// `u32 n, n × list`: lists past the simulator's `len` are read and their
    /// references released; lists the record does not have are emptied.
    fn list_vector(&mut self, inp: &mut dyn InStream, len: usize, make: fn(usize) -> ListId) -> Option<()> {
        let n = inp.get_u32()?;
        for i in n as usize..len {
            let old = self.list_opt(make(i)).map(std::mem::take).unwrap_or_default();
            self.replaced.extend(old.into_iter().flatten());
        }
        for i in 0..n as usize {
            if i < len {
                self.list(inp, make(i))?;
            } else {
                self.extra_list(inp)?;
            }
        }
        Some(())
    }

    /// A list beyond the simulator's vector: read and released.
    fn extra_list(&mut self, inp: &mut dyn InStream) -> Option<()> {
        let n = inp.get_u32()?;
        if n == 0 || n >= MAX_LIST_COUNT {
            return Some(());
        }
        for _ in 0..n {
            if let Some(t) = self.token(inp, IID_OCCUPANT)? {
                self.replaced.push(t);
            }
        }
        Some(())
    }

    fn iterator(&mut self, inp: &mut dyn InStream) -> Option<()> {
        let it = &mut self.s.iterator;
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
        Some(())
    }

    /// The record in order (§9.1); `None` at the first stream failure.
    fn record(&mut self, inp: &mut dyn InStream) -> Option<()> {
        let version = inp.get_u16()?;
        self.version = Some(version);
        self.s.proximity_size = inp.get_i32()?;
        let mut g = self.s.grids.proximity.clone();
        let r = self.grids3(inp, &mut g);
        self.s.grids.proximity = g;
        r?;
        self.s.population_size = inp.get_i32()?;
        let mut g = self.s.grids.population.clone();
        let r = self.grids3(inp, &mut g);
        self.s.grids.population = g;
        r?;
        self.s.grids.age = self.grid(inp)?;
        self.s.grids.school_coverage = self.grid_vector::<7>(inp)?;
        self.s.grids.hospital_coverage = self.grid_vector::<3>(inp)?;
        self.s.grids.school_grade = self.grid(inp)?;
        self.s.grids.hospital_grade = self.grid(inp)?;
        self.s.eq_size = inp.get_i32()?;
        let mut g = self.s.grids.eq.clone();
        let r = self.grids3(inp, &mut g);
        self.s.grids.eq = g;
        r?;
        let mut g = self.s.grids.hq.clone();
        let r = self.grids3(inp, &mut g);
        self.s.grids.hq = g;
        r?;
        self.s.unused_float1 = inp.get_f32()?;
        self.s.unused_float2 = inp.get_f32()?;
        self.s.state = inp.get_u32()?;
        self.s.dirty = inp.get_u8()? != 0;
        self.s.population = inp.get_i32()?;
        self.block(inp, true)?;
        self.block(inp, false)?;
        self.list(inp, ListId::PendingSchool)?;
        self.list(inp, ListId::PendingHospital)?;
        self.list(inp, ListId::Other)?;
        self.list_vector(inp, 7, ListId::School)?;
        self.list_vector(inp, 3, ListId::Hospital)?;
        self.list(inp, ListId::HealthAffecting)?;
        if version >= 3 {
            self.iterator(inp)?;
            self.s.le_sum = inp.get_u64()?;
        }
        if version >= 4 {
            self.s.running_population = inp.get_i32()?;
        }
        if version >= 7 {
            self.s.school_auto_budget = inp.get_u8()? != 0;
            self.s.hospital_auto_budget = inp.get_u8()? != 0;
        }
        Some(())
    }

    /// The fix-ups of §9.2 after the parse.
    fn finish(&mut self) {
        let s = &mut self.s;
        if let Some(v) = self.version {
            if v < 3 {
                s.iterator.reset(0, 0);
                s.le_sum = 0;
                s.running_population = 0;
                if s.state == st::POPULATION {
                    s.state = st::HISTOGRAM_RESET;
                }
            }
            if v < 7 {
                s.school_auto_budget = false;
                s.hospital_auto_budget = false;
            }
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

/// Which occupant list a record entry fills (§9.1 order).
#[derive(Clone, Copy)]
enum ListId {
    PendingSchool,
    PendingHospital,
    Other,
    School(usize),
    Hospital(usize),
    HealthAffecting,
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

    /// §9.2: every version, never fails (AL = 1), stream errors ignored.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        let mut p = Parse {
            h: &*self.host,
            s: self.snapshot(),
            version: None,
            replaced: Vec::new(),
        };
        let _ = p.record(inp);
        p.finish();
        let Parse { s, replaced, .. } = p;
        *self.s.borrow_mut() = s;
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
