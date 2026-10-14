// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The record of §7 (version 6).

use std::any::Any;
use std::rc::Rc;

use crate::stream::{InStream, OutStream, RefValue, SaveObject};

use super::simulator::CivicSim;
use super::{
    CivicHost, ConditionalBuildingStatus, StlHashMap, StlHashSet, CLSID_CIVIC_BUILDING_SIMULATOR,
    IID_DEPARTMENT_BUDGET, IID_OCCUPANT, MAX_LIST_COUNT, RECORD_VERSION,
};

/// Writes one occupant list (pollution.md §6): `u32 count`, then `count`
/// references (a null entry is a null reference).
pub(crate) fn write_list(h: &dyn CivicHost, out: &mut dyn OutStream, list: &[Option<u64>]) {
    out.put_u32(list.len() as u32);
    for e in list {
        out.put_ref(&e.map_or(RefValue::Null, |t| h.object_ref(t)));
    }
}

/// The record as parsed, before it is applied.
struct Parsed {
    exclusions: Vec<u32>,
    statuses: Vec<(u32, ConditionalBuildingStatus)>,
    landmarks: Option<u64>,
    disease_centers: Vec<Option<u64>>,
    maintained: Vec<Option<u64>>,
}

/// Reads references, keeping every token it was handed so a failed read
/// can give them back.
struct Reader<'a> {
    h: &'a dyn CivicHost,
    acquired: Vec<u64>,
}

impl Reader<'_> {
    fn token(&mut self, inp: &mut dyn InStream, iid: u32) -> Option<Option<u64>> {
        let r = inp.get_ref_as(iid)?;
        let t = if r.is_null() { None } else { self.h.resolve_ref(&r, iid) };
        if let Some(t) = t {
            self.acquired.push(t);
        }
        Some(t)
    }

    /// The shared list reader (§7): a count of 10,000,000 or more reads
    /// nothing further; an unresolvable reference appends null.
    fn list(&mut self, inp: &mut dyn InStream) -> Option<Vec<Option<u64>>> {
        let n = inp.get_u32()?;
        let mut v = Vec::new();
        if n >= MAX_LIST_COUNT {
            return Some(v);
        }
        for _ in 0..n {
            v.push(self.token(inp, IID_OCCUPANT)?);
        }
        Some(v)
    }

    fn record(&mut self, inp: &mut dyn InStream) -> Option<Parsed> {
        if inp.get_u16()? != RECORD_VERSION {
            return None;
        }
        let n = inp.get_u32()?;
        let mut exclusions = Vec::new();
        for _ in 0..n {
            exclusions.push(inp.get_u32()?);
        }
        let m = inp.get_u32()?;
        let mut statuses = Vec::new();
        for _ in 0..m {
            let key = inp.get_u32()?;
            let status = inp.get_u8()?;
            let building_count = inp.get_u16()?;
            let tooltip = inp.get_str()?;
            statuses.push((
                key,
                ConditionalBuildingStatus {
                    status,
                    building_count,
                    tooltip,
                },
            ));
        }
        let landmarks = self.token(inp, IID_DEPARTMENT_BUDGET)?;
        let disease_centers = self.list(inp)?;
        let maintained = self.list(inp)?;
        if inp.error() != 0 {
            return None;
        }
        Some(Parsed {
            exclusions,
            statuses,
            landmarks,
            disease_centers,
            maintained,
        })
    }
}

/// The file's entries without repeats, first occurrence kept (STL `insert`
/// keeps the value already present).
fn first_occurrences<T>(entries: Vec<(u32, T)>) -> Vec<(u32, T)> {
    let mut seen = std::collections::HashSet::new();
    entries.into_iter().filter(|(k, _)| seen.insert(*k)).collect()
}

/// Inserts `ids` so that the set iterates them in file order (see the
/// module documentation of `civic`): grow first, then insert in reverse.
fn insert_set_in_order(set: &mut StlHashSet, ids: Vec<u32>) {
    let ids: Vec<u32> = first_occurrences(ids.into_iter().map(|k| (k, ())).collect())
        .into_iter()
        .map(|(k, _)| k)
        .filter(|k| !set.contains(*k))
        .collect();
    set.reserve(set.len() + ids.len());
    for k in ids.into_iter().rev() {
        set.insert(k);
    }
}

/// [`insert_set_in_order`] for the status map.
fn insert_map_in_order(
    map: &mut StlHashMap<ConditionalBuildingStatus>,
    entries: Vec<(u32, ConditionalBuildingStatus)>,
) {
    let entries: Vec<_> = first_occurrences(entries)
        .into_iter()
        .filter(|(k, _)| !map.contains(*k))
        .collect();
    map.reserve(map.len() + entries.len());
    for (k, v) in entries.into_iter().rev() {
        map.entry(k, || v);
    }
}

impl SaveObject for CivicSim {
    fn class_id(&self) -> u32 {
        CLSID_CIVIC_BUILDING_SIMULATOR
    }

    /// §7: version 6, the exclusions and statuses in hash order, the
    /// Landmarks reference and the two lists. True unless the stream
    /// reports an error.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let h = &*self.host;
        let s = self.s.borrow().clone();
        out.put_u16(RECORD_VERSION);
        let ids = s.exclusions.iter_order();
        out.put_u32(ids.len() as u32);
        for id in ids {
            out.put_u32(id);
        }
        out.put_u32(s.statuses.len() as u32);
        for (k, st) in s.statuses.iter() {
            out.put_u32(k);
            out.put_u8(st.status);
            out.put_u16(st.building_count);
            out.put_str(&st.tooltip);
        }
        out.put_ref(&s.landmarks.map_or(RefValue::Null, |t| h.object_ref(t)));
        write_list(h, out, &s.disease_centers);
        write_list(h, out, &s.maintained);
        out.error() == 0
    }

    /// §7: false unless the version is exactly 6. [choice] Atomic: on a
    /// failure nothing changes and the references read are released.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        let h = &*self.host;
        let mut r = Reader {
            h,
            acquired: Vec::new(),
        };
        let Some(p) = r.record(inp) else {
            for t in r.acquired {
                h.release(t);
            }
            return false;
        };
        let old_department = {
            let mut s = self.s.borrow_mut();
            insert_set_in_order(&mut s.exclusions, p.exclusions);
            insert_map_in_order(&mut s.statuses, p.statuses);
            // The lists are replaced without releasing their old entries
            // (the shared list reader, §7).
            s.disease_centers = p.disease_centers;
            s.maintained = p.maintained;
            std::mem::replace(&mut s.landmarks, p.landmarks)
        };
        if let Some(d) = old_department {
            h.release(d);
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
