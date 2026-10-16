// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Component 41, the automata controller manager (§3, §7.1): the controllers
//! grouped by occupant, the automaton sets, the two controller times, the
//! record, the template re-binding at `PostCityInit` (§3.4) and the calls of
//! §7.1.

use std::rc::Rc;

use crate::stream::{InStream, OutStream, RefValue};

use super::codec::{
    get_n_refs, get_ref_checked, get_time, get_vec3, put_time, put_vec3, ref_id, ref_lists_equal, refs_equal, same_ref,
    ControllerTime, RefId, Vec3,
};
use super::controller::{ref_is_transient, Controller};
use super::script::{ControllerTemplate, OccupantGroup, ScriptSource};
use super::{IID_24_HOUR_CLOCK, IID_AUTOMATA_CONTROLLER, IID_AUTOMATON, IID_OCCUPANT};

/// The record's version (§3.1; Read does not check it).
pub const CONTROLLER_MANAGER_VERSION: u16 = 1;

/// What the controller manager needs from its host. Every call has a
/// default, so a host implements only what it uses.
pub trait ControllerManagerHost {
    /// A log line (§3.4 step 2).
    fn log(&self, _text: &str) {}
    /// Releases the reference the manager held on a host object it drops (a
    /// controller or occupant read from a save, foundations §2.7: read
    /// references come with one reference owned by the reader).
    fn release(&self, _token: u64) {}
    /// Takes a reference on a host occupant the manager starts listing (a
    /// new occupant entry made by slot 6).
    fn add_ref(&self, _token: u64) {}
    /// A host controller's flags (its `CheckFlag`); core controllers answer
    /// themselves.
    fn controller_flags(&self, _token: u64) -> u32 {
        0
    }
}

/// A host that does nothing.
pub struct NoControllerHost;

impl ControllerManagerHost for NoControllerHost {}

/// The log line of §3.4 step 2.
pub fn missing_template_message(name: &str) -> String {
    format!("Couldn't find automata controller template '{name}' in scripts. Removing controller.")
}

/// An occupant and its controllers (§3.2), in list order.
#[derive(Clone, Debug)]
pub struct OccupantEntry {
    pub occupant: RefValue,
    pub controllers: Vec<RefValue>,
}

impl PartialEq for OccupantEntry {
    fn eq(&self, o: &Self) -> bool {
        refs_equal(&self.occupant, &o.occupant) && ref_lists_equal(&self.controllers, &o.controllers)
    }
}

/// An automaton set (§3.3): occupant, automata, two vectors and a flag
/// (their meaning is Q2).
#[derive(Clone, Debug)]
pub struct AutomatonSet {
    pub occupant: RefValue,
    pub automata: Vec<RefValue>,
    pub v1: Vec3,
    pub v2: Vec3,
    pub flag: bool,
}

impl PartialEq for AutomatonSet {
    fn eq(&self, o: &Self) -> bool {
        refs_equal(&self.occupant, &o.occupant)
            && ref_lists_equal(&self.automata, &o.automata)
            && self.v1 == o.v1
            && self.v2 == o.v2
            && self.flag == o.flag
    }
}

/// The saved state (§3.1).
#[derive(Clone, Debug)]
pub struct ControllerManagerRecord {
    /// The version as read (Read does not check it; written back unchanged
    /// *[choice]*, 1 for a new manager).
    pub version: u16,
    /// The 24-hour clock (read with `0x681BD53A`).
    pub clock: RefValue,
    pub time_a: ControllerTime,
    /// The time slot 24 `GetControllerTime` returns.
    pub time_b: ControllerTime,
    /// The controllers by occupant, in a fixed order: the order of the file,
    /// new occupants appended *[choice]* (§3.2).
    pub entries: Vec<OccupantEntry>,
    pub sets: Vec<AutomatonSet>,
}

impl Default for ControllerManagerRecord {
    fn default() -> Self {
        ControllerManagerRecord {
            version: CONTROLLER_MANAGER_VERSION,
            clock: RefValue::Null,
            time_a: ControllerTime::default(),
            time_b: ControllerTime::default(),
            entries: Vec::new(),
            sets: Vec::new(),
        }
    }
}

impl PartialEq for ControllerManagerRecord {
    fn eq(&self, o: &Self) -> bool {
        self.version == o.version
            && refs_equal(&self.clock, &o.clock)
            && self.time_a == o.time_a
            && self.time_b == o.time_b
            && self.entries == o.entries
            && self.sets == o.sets
    }
}

/// An `InStream` that remembers every host reference it hands out, so a
/// failed or partly discarded read can give the references back.
struct Tracking<'a> {
    inner: &'a mut dyn InStream,
    tokens: Vec<u64>,
}

impl InStream for Tracking<'_> {
    fn get_u8(&mut self) -> Option<u8> {
        self.inner.get_u8()
    }
    fn get_u16(&mut self) -> Option<u16> {
        self.inner.get_u16()
    }
    fn get_u32(&mut self) -> Option<u32> {
        self.inner.get_u32()
    }
    fn get_u64(&mut self) -> Option<u64> {
        self.inner.get_u64()
    }
    fn get_f32(&mut self) -> Option<f32> {
        self.inner.get_f32()
    }
    fn get_f64(&mut self) -> Option<f64> {
        self.inner.get_f64()
    }
    fn get_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        self.inner.get_bytes(n)
    }
    fn get_str(&mut self) -> Option<Vec<u8>> {
        self.inner.get_str()
    }
    fn get_ref(&mut self) -> Option<RefValue> {
        self.get_ref_as(0)
    }
    fn get_ref_as(&mut self, iid: u32) -> Option<RefValue> {
        let r = self.inner.get_ref_as(iid)?;
        if let RefValue::Host { token, .. } = r {
            self.tokens.push(token);
        }
        Some(r)
    }
    fn skip(&mut self, n: usize) -> bool {
        self.inner.skip(n)
    }
    fn error(&self) -> i32 {
        self.inner.error()
    }
    fn set_error(&mut self) {
        self.inner.set_error()
    }
}

/// The controller manager (`0xCA14CA03`, city slot 47).
pub struct ControllerManager {
    rec: ControllerManagerRecord,
    loaded: bool,
    host: Box<dyn ControllerManagerHost>,
}

impl ControllerManager {
    pub fn new(host: Box<dyn ControllerManagerHost>) -> ControllerManager {
        ControllerManager {
            rec: ControllerManagerRecord::default(),
            loaded: false,
            host,
        }
    }

    pub fn record(&self) -> &ControllerManagerRecord {
        &self.rec
    }

    pub fn record_mut(&mut self) -> &mut ControllerManagerRecord {
        &mut self.rec
    }

    /// Read set the loaded flag (§3.4, §12 item 7).
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn set_clock(&mut self, clock: RefValue) {
        self.rec.clock = clock;
    }

    /// Sets the two controller times (they advance with the clock's
    /// `HourChanged` *[inferred]*; how is open, AM5).
    pub fn set_times(&mut self, a: ControllerTime, b: ControllerTime) {
        self.rec.time_a = a;
        self.rec.time_b = b;
    }

    /// Slot 24 `GetControllerTime`: time B (§3.1 field 4).
    pub fn controller_time(&self) -> ControllerTime {
        self.rec.time_b
    }

    /// The controllers of `occupant` (empty when none).
    pub fn controllers_of(&self, occupant: &RefValue) -> Vec<RefValue> {
        self.rec
            .entries
            .iter()
            .find(|e| same_ref(&e.occupant, occupant))
            .map(|e| e.controllers.clone())
            .unwrap_or_default()
    }

    /// Every controller, entry by entry.
    pub fn all_controllers(&self) -> Vec<RefValue> {
        self.rec
            .entries
            .iter()
            .flat_map(|e| e.controllers.iter().cloned())
            .collect()
    }

    fn push_controller(&mut self, occupant: &RefValue, c: RefValue) {
        match self.rec.entries.iter_mut().find(|e| same_ref(&e.occupant, occupant)) {
            Some(e) => e.controllers.push(c),
            None => {
                if let RefValue::Host { token, .. } = occupant {
                    self.host.add_ref(*token);
                }
                self.rec.entries.push(OccupantEntry {
                    occupant: occupant.clone(),
                    controllers: vec![c],
                })
            }
        }
    }

    /// Slot 6 `AddControllerFromTemplate(occupant, template, extra)`: a new
    /// controller of the template's class, bound, `Init`ed, in the grid, at
    /// the end of the occupant's list. `extra`'s meaning is open (AM6); it is
    /// not used.
    pub fn add_controller_from_template(
        &mut self,
        occupant: &RefValue,
        template: &ControllerTemplate,
        _extra: u32,
    ) -> Rc<Controller> {
        let c = Controller::from_template(template, occupant.clone());
        self.push_controller(occupant, RefValue::Object(c.clone()));
        c
    }

    /// Slot 5 `AddControllerFromTemplateName(occupant, name, extra)` (§7.1):
    /// no template of that name → null.
    pub fn add_controller_from_template_name(
        &mut self,
        scripts: &dyn ScriptSource,
        occupant: &RefValue,
        name: &str,
        extra: u32,
    ) -> Option<Rc<Controller>> {
        let t = scripts.controller_template(name)?;
        Some(self.add_controller_from_template(occupant, &t, extra))
    }

    fn add_group(&mut self, scripts: &dyn ScriptSource, group: OccupantGroup, occupant: &RefValue, extra: u32) {
        for name in &group.templates {
            // The group lists templates; a name the scripts no longer define
            // is skipped *[choice]*.
            if let Some(t) = scripts.controller_template(name) {
                self.add_controller_from_template(occupant, &t, extra);
            }
        }
    }

    /// Slot 9 `AddControllersFromOccupantGroup(groupID, occupant, extra)`
    /// (§7.1): no script system or no group → false; otherwise one controller
    /// per template of the group, true.
    pub fn add_controllers_from_occupant_group_id(
        &mut self,
        scripts: Option<&dyn ScriptSource>,
        group_id: u32,
        occupant: &RefValue,
        extra: u32,
    ) -> bool {
        let Some(s) = scripts else { return false };
        let Some(g) = s.occupant_group_by_id(group_id) else {
            return false;
        };
        self.add_group(s, g, occupant, extra);
        true
    }

    /// Slot 10, the same with the group looked up by name.
    pub fn add_controllers_from_occupant_group_name(
        &mut self,
        scripts: Option<&dyn ScriptSource>,
        name: &str,
        occupant: &RefValue,
        extra: u32,
    ) -> bool {
        let Some(s) = scripts else { return false };
        let Some(g) = s.occupant_group_by_name(name) else {
            return false;
        };
        self.add_group(s, g, occupant, extra);
        true
    }

    /// Shuts down and lets go of the controllers for which `drop` is true.
    fn remove_where(&mut self, occupant: &RefValue, drop: impl Fn(&RefValue) -> bool) -> usize {
        let Some(i) = self.rec.entries.iter().position(|e| same_ref(&e.occupant, occupant)) else {
            return 0;
        };
        let (gone, kept): (Vec<RefValue>, Vec<RefValue>) =
            self.rec.entries[i].controllers.drain(..).partition(|c| drop(c));
        self.rec.entries[i].controllers = kept;
        let n = gone.len();
        for c in gone {
            self.release_controller(c);
        }
        if self.rec.entries[i].controllers.is_empty() {
            let e = self.rec.entries.remove(i);
            self.release_ref(&e.occupant);
        }
        n
    }

    fn release_ref(&self, r: &RefValue) {
        if let RefValue::Host { token, .. } = r {
            self.host.release(*token);
        }
    }

    /// Shut down and release (§3.4 step 2, slot 14).
    fn release_controller(&self, c: RefValue) {
        match &c {
            RefValue::Object(o) => {
                if let Ok(ctl) = o.clone().as_any_rc().downcast::<Controller>() {
                    ctl.shutdown();
                }
            }
            RefValue::Host { .. } => self.release_ref(&c),
            RefValue::Null => {}
        }
    }

    /// Slot 14 `RemoveControllers(occupant)`: all its controllers are shut
    /// down and released. Returns how many.
    pub fn remove_controllers(&mut self, occupant: &RefValue) -> usize {
        self.remove_where(occupant, |_| true)
    }

    fn template_of(c: &RefValue) -> Option<String> {
        match c {
            RefValue::Object(o) => o
                .clone()
                .as_any_rc()
                .downcast::<Controller>()
                .ok()
                .map(|c| c.template_name()),
            _ => None,
        }
    }

    fn remove_group(&mut self, group: OccupantGroup, occupant: &RefValue) -> usize {
        let names = group.templates;
        self.remove_where(occupant, move |c| {
            Self::template_of(c).is_some_and(|n| names.contains(&n))
        })
    }

    /// Slot 11 `RemoveControllersFromOccupantGroup(groupID, occupant)`: the
    /// occupant's controllers whose template is one of the group's are removed
    /// (the literal reading of "the controllers that the group added", which
    /// also works after a load, AM7). False when there is no script system
    /// or no such group.
    pub fn remove_controllers_from_occupant_group_id(
        &mut self,
        scripts: Option<&dyn ScriptSource>,
        group_id: u32,
        occupant: &RefValue,
    ) -> bool {
        let Some(g) = scripts.and_then(|s| s.occupant_group_by_id(group_id)) else {
            return false;
        };
        self.remove_group(g, occupant);
        true
    }

    /// Slot 12, by name.
    pub fn remove_controllers_from_occupant_group_name(
        &mut self,
        scripts: Option<&dyn ScriptSource>,
        name: &str,
        occupant: &RefValue,
    ) -> bool {
        let Some(g) = scripts.and_then(|s| s.occupant_group_by_name(name)) else {
            return false;
        };
        self.remove_group(g, occupant);
        true
    }

    /// `RemoveOccupant` (`0x99EF1143`, §10): the occupant takes its
    /// controllers and its automaton sets with it.
    pub fn occupant_removed(&mut self, occupant: &RefValue) {
        self.remove_controllers(occupant);
        let (gone, kept): (Vec<AutomatonSet>, Vec<AutomatonSet>) =
            self.rec.sets.drain(..).partition(|s| same_ref(&s.occupant, occupant));
        self.rec.sets = kept;
        for s in gone {
            self.release_ref(&s.occupant);
            for a in &s.automata {
                self.release_ref(a);
            }
        }
    }

    /// `PostCityInit` (`0x26D31EC1`, §3.4): only when loaded, every saved
    /// controller is looked up by its template name; unknown → logged, shut
    /// down, released; known → `SetTemplate`, `Init`, into the grid. Host
    /// controllers are left alone (they re-bind themselves). Returns the
    /// names of the dropped controllers.
    pub fn post_city_init(&mut self, scripts: &dyn ScriptSource) -> Vec<String> {
        let mut dropped = Vec::new();
        if !self.loaded {
            return dropped;
        }
        let occupants: Vec<RefValue> = self.rec.entries.iter().map(|e| e.occupant.clone()).collect();
        for occ in occupants {
            let mut unknown: Vec<RefId> = Vec::new();
            for c in self.controllers_of(&occ) {
                let RefValue::Object(o) = &c else { continue };
                let Ok(ctl) = o.clone().as_any_rc().downcast::<Controller>() else {
                    continue;
                };
                let name = ctl.template_name();
                match scripts.controller_template(&name) {
                    Some(t) => {
                        ctl.set_template(&t);
                        ctl.init();
                        ctl.add_to_grid();
                    }
                    None => {
                        self.host.log(&missing_template_message(&name));
                        unknown.push(ref_id(&c));
                        dropped.push(name);
                    }
                }
            }
            if !unknown.is_empty() {
                self.remove_where(&occ, |c| unknown.contains(&ref_id(c)));
            }
        }
        dropped
    }

    fn transient(&self, c: &RefValue) -> bool {
        ref_is_transient(c, &|t| self.host.controller_flags(t))
    }

    /// Write (§3.1): version, clock, the two times, the occupant entries with
    /// their controllers without flag 8 (occupants left with none are
    /// skipped), the automaton sets.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        if out.error() != 0 {
            return false;
        }
        out.put_u16(self.rec.version);
        out.put_ref(&self.rec.clock);
        put_time(out, &self.rec.time_a);
        put_time(out, &self.rec.time_b);
        let saved: Vec<(&RefValue, Vec<&RefValue>)> = self
            .rec
            .entries
            .iter()
            .map(|e| {
                (
                    &e.occupant,
                    e.controllers.iter().filter(|c| !self.transient(c)).collect::<Vec<_>>(),
                )
            })
            .filter(|(_, cs)| !cs.is_empty())
            .collect();
        out.put_u32(saved.len() as u32);
        for (occ, cs) in saved {
            out.put_ref(occ);
            out.put_u32(cs.len() as u32);
            for c in cs {
                out.put_ref(c);
            }
        }
        out.put_u32(self.rec.sets.len() as u32);
        for s in &self.rec.sets {
            out.put_ref(&s.occupant);
            out.put_u32(s.automata.len() as u32);
            for a in &s.automata {
                out.put_ref(a);
            }
            put_vec3(out, &s.v1);
            put_vec3(out, &s.v2);
            out.put_bool(s.flag);
        }
        out.error() == 0
    }

    fn parse(inp: &mut Tracking<'_>, duplicates: &mut Vec<RefValue>) -> Option<ControllerManagerRecord> {
        let version = inp.get_u16()?;
        let clock = get_ref_checked(inp, IID_24_HOUR_CLOCK)?;
        let time_a = get_time(inp)?;
        let time_b = get_time(inp)?;
        let n1 = inp.get_u32()?;
        let mut entries: Vec<OccupantEntry> = Vec::new();
        for _ in 0..n1 {
            let occupant = get_ref_checked(inp, IID_OCCUPANT)?;
            let m = inp.get_u32()? as usize;
            let controllers = get_n_refs(inp, m, IID_AUTOMATA_CONTROLLER)?;
            if entries.iter().any(|e| same_ref(&e.occupant, &occupant)) {
                // §12 item 2 [choice]: a repeated occupant's controllers are
                // read, then dropped.
                duplicates.push(occupant);
                duplicates.extend(controllers);
                continue;
            }
            entries.push(OccupantEntry { occupant, controllers });
        }
        let n2 = inp.get_u32()?;
        let mut sets = Vec::new();
        for _ in 0..n2 {
            let occupant = get_ref_checked(inp, IID_OCCUPANT)?;
            let k = inp.get_u32()? as usize;
            let automata = get_n_refs(inp, k, IID_AUTOMATON)?;
            sets.push(AutomatonSet {
                occupant,
                automata,
                v1: get_vec3(inp)?,
                v2: get_vec3(inp)?,
                flag: inp.get_bool()?,
            });
        }
        Some(ControllerManagerRecord {
            version,
            clock,
            time_a,
            time_b,
            entries,
            sets,
        })
    }

    /// Read (§3.1): sets the loaded flag. Atomic *[choice]*: on a short
    /// stream nothing changes, the host references read so far are released
    /// and the result is false.
    pub fn read(&mut self, inp: &mut dyn InStream) -> bool {
        let mut t = Tracking {
            inner: inp,
            tokens: Vec::new(),
        };
        let mut duplicates = Vec::new();
        match Self::parse(&mut t, &mut duplicates) {
            Some(r) if t.error() == 0 => {
                for d in &duplicates {
                    self.release_controller(d.clone());
                }
                self.rec = r;
                self.loaded = true;
                true
            }
            _ => {
                for tok in t.tokens {
                    self.host.release(tok);
                }
                false
            }
        }
    }
}
