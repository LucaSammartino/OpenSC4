// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The disaster layer (§2): the enable counter (§2.3), begin and end with
//! the emergency pause (§2.4), the damage assessment and dispatch timers
//! (§2.5), the cost (§2.6) and the locations (§2.7).
//!
//! The enable counter only nests (test 6): nothing reads it, so it has no
//! effect on any disaster, the menu or the emergency pause (§2.3, Q5).

use std::collections::BTreeMap;

use super::system::Core;
use super::{msg, occupant, prop, DisasterPost, Target, MANAGER_TABLE, NETWORK_DEFAULT_COST};

/// A wall-clock timer (`cRZTimer`): accumulates while running.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timer {
    started_at: Option<u64>,
    accumulated_ms: u64,
}

impl Timer {
    /// `Reset` + `Start`.
    pub fn restart(&mut self, now: u64) {
        self.accumulated_ms = 0;
        self.started_at = Some(now);
    }
    /// `Start`: no effect while running.
    pub fn start(&mut self, now: u64) {
        if self.started_at.is_none() {
            self.started_at = Some(now);
        }
    }
    /// `Stop`: keeps the time so far.
    pub fn stop(&mut self, now: u64) {
        if let Some(s) = self.started_at.take() {
            self.accumulated_ms += now.saturating_sub(s);
        }
    }
    pub fn running(&self) -> bool {
        self.started_at.is_some()
    }
    pub fn elapsed_ms(&self, now: u64) -> u64 {
        self.accumulated_ms + self.started_at.map_or(0, |s| now.saturating_sub(s))
    }
    /// Units 5 (seconds) through the integer `GetElapsedTime`: whole
    /// seconds, truncated (the riot's gate, §6.5, Q9).
    pub fn elapsed_s(&self, now: u64) -> u64 {
        self.elapsed_ms(now) / 1000
    }
    /// Units 5 through the `float` `GetElapsedTime`: fractional seconds
    /// (the dispatch effectiveness, §2.5, Q9).
    pub fn elapsed_secs_f32(&self, now: u64) -> f32 {
        (self.elapsed_ms(now) as f64 / 1000.0) as f32
    }
}

/// A special-building record (§2.5 step 2.3): the spec does not give its
/// fields; we keep the occupant, its flags and its cost [choice].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpecialBuilding {
    pub occupant: u64,
    pub flags: u32,
    pub cost: i64,
}

/// The damage assessment (§2.1, §2.5).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DamageAssessment {
    pub building_count: u32,
    pub total_cost: i64,
    /// (count, cost) for the flag bits 1, 2, 4 and 0x20 (`+0x60` .. `+0x98`).
    pub kinds: [(u32, i64); 4],
    pub special: Vec<SpecialBuilding>,
}

/// The building-type flag bits of the four kind pairs (§2.5 step 2.2).
pub const KIND_BITS: [u32; 4] = [0x1, 0x2, 0x4, 0x20];
/// Flags that make a special-building record (§2.5 step 2.3).
pub const SPECIAL_BITS: u32 = 0x0180_0400;

/// A location entry (§2.1: 20 bytes {instance, x, z, zoom level, occupant}).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Location {
    pub instance: u32,
    pub x: i32,
    pub z: i32,
    pub zoom: u32,
    /// 0 = none.
    pub occupant: u64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LayerState {
    pub initialised: bool,
    pub disable_counter: u32,
    pub ongoing: u32,
    pub damage: DamageAssessment,
    /// Subscribed to the assessment messages (between start and end).
    pub assessing: bool,
    pub fire_timers: BTreeMap<u32, Timer>,
    pub police_timers: BTreeMap<u32, Timer>,
    pub overall: Timer,
    pub fire_effectiveness: f32,
    pub police_effectiveness: f32,
    pub locations: Vec<Location>,
    /// The `GoToNextDisaster` cursor (`+0xC0`, §2.7); `None` before the
    /// first registration.
    pub cursor: Option<usize>,
}

/// The layer's observable state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayerSnapshot {
    pub initialised: bool,
    pub disable_counter: u32,
    pub ongoing: u32,
    pub assessing: bool,
    pub damage: DamageAssessment,
    pub fire_timers: Vec<(u32, u64)>,
    pub police_timers: Vec<(u32, u64)>,
    pub overall_s: u64,
    pub fire_effectiveness: f32,
    pub police_effectiveness: f32,
    pub locations: Vec<Location>,
    pub cursor: Option<usize>,
    /// Manager types in map order.
    pub managers: Vec<u32>,
}

impl Core {
    // ---- §2.2 ----

    /// `Init`, once: create the managers in table order, `Init` them in map
    /// order (ascending type), reset the cost.
    pub(crate) fn layer_init(&mut self) -> bool {
        if self.layer.initialised {
            return true;
        }
        for (t, class) in MANAGER_TABLE {
            self.create_manager(t, class);
        }
        let types: Vec<u32> = self.managers.keys().copied().collect();
        for t in types {
            self.manager_init(t);
        }
        self.reset_cost();
        self.layer.initialised = true;
        true
    }

    /// `Shutdown`: every manager shut down and released (map order), the
    /// locations (their occupants released) and the timers cleared, the
    /// assessment subscriptions dropped.
    pub(crate) fn layer_shutdown(&mut self) -> bool {
        let types: Vec<u32> = self.managers.keys().copied().collect();
        for t in types {
            self.manager_shutdown(t);
        }
        self.managers.clear();
        self.fire_instance = None;
        self.toxic_instance = None;
        let host = self.host.clone();
        for l in std::mem::take(&mut self.layer.locations) {
            if l.occupant != 0 {
                host.release(l.occupant);
            }
        }
        self.layer.cursor = None;
        self.layer.fire_timers.clear();
        self.layer.police_timers.clear();
        self.layer.overall = Timer::default();
        if self.layer.assessing {
            self.layer.assessing = false;
            for m in msg::ASSESSMENT {
                host.unsubscribe(Target::Layer, m);
            }
        }
        self.layer.initialised = false;
        true
    }

    // ---- §2.3 ----

    pub(crate) fn enable_disasters(&mut self, enable: bool) -> bool {
        let c = &mut self.layer.disable_counter;
        if enable {
            *c = c.saturating_sub(1);
        } else {
            *c = c.saturating_add(1);
        }
        self.disasters_enabled()
    }

    pub(crate) fn disasters_enabled(&self) -> bool {
        self.layer.disable_counter == 0
    }

    // ---- §2.4 ----

    /// `DisasterHasBegun(instance)` (slot 8).
    pub(crate) fn disaster_has_begun(&mut self, instance: u32) {
        let host = self.host.clone();
        self.layer.ongoing = self.layer.ongoing.saturating_add(1);
        if self.layer.ongoing == 1 {
            host.emergency_pause();
            self.start_assessment();
            host.disable_save();
        }
        let (x, z) = self.most_recent_location(Some(instance)).unwrap_or((0, 0));
        host.post(&DisasterPost {
            msg_type: msg::DISASTER_PERIOD_STARTED,
            data1: u64::from(self.layer.ongoing),
            data2: x as u32,
            data3: z as u32,
            data4: self.instance_type(instance),
            void1: instance,
        });
    }

    /// `DisasterHasEnded()` (slot 9): only while initialised and ongoing.
    pub(crate) fn disaster_has_ended(&mut self) {
        if !self.layer.initialised || self.layer.ongoing == 0 {
            return;
        }
        self.layer.ongoing -= 1;
        if self.layer.ongoing == 0 {
            let host = self.host.clone();
            host.enable_save();
            host.emergency_resume();
            self.end_assessment();
            host.post(&DisasterPost::new(msg::DISASTER_PERIOD_ENDED));
        }
    }

    pub(crate) fn is_disaster_ongoing(&self) -> bool {
        self.layer.ongoing != 0
    }

    // ---- §2.5 ----

    fn start_assessment(&mut self) {
        let host = self.host.clone();
        if !self.layer.assessing {
            for m in msg::ASSESSMENT {
                host.subscribe(Target::Layer, m);
            }
            self.layer.assessing = true;
        }
        self.reset_cost();
        self.layer.fire_timers.clear();
        self.layer.police_timers.clear();
        self.layer.overall.restart(host.now_ms());
    }

    fn end_assessment(&mut self) {
        let host = self.host.clone();
        if self.layer.assessing {
            for m in msg::ASSESSMENT {
                host.unsubscribe(Target::Layer, m);
            }
            self.layer.assessing = false;
        }
        self.end_dispatch_timers();
    }

    /// `EndDispatchTimers` (§2.5): every timer stopped; per map
    /// `Σ seconds / (float(N) × overall seconds)` in fractional seconds, `N`
    /// being the protection simulator's `GetTotalDispatchCount()` (fire slot
    /// 25, police slot 26). `N = 0` gives 0, an empty map gives 0, and a
    /// zero overall time gives 0 [choice, §2.5]. The values are stored
    /// (`+0xAC` / `+0xB0`) and passed to nobody (Q11).
    pub(crate) fn end_dispatch_timers(&mut self) {
        let host = self.host.clone();
        let now = host.now_ms();
        self.layer.overall.stop(now);
        for t in self.layer.fire_timers.values_mut() {
            t.stop(now);
        }
        for t in self.layer.police_timers.values_mut() {
            t.stop(now);
        }
        let overall = self.layer.overall.elapsed_secs_f32(now);
        let eff = |m: &BTreeMap<u32, Timer>, n: u32| -> f32 {
            let sum: f32 = m.values().map(|t| t.elapsed_secs_f32(now)).sum();
            let den = n as f32 * overall;
            if n == 0 || den == 0.0 {
                0.0
            } else {
                sum / den
            }
        };
        self.layer.fire_effectiveness = eff(&self.layer.fire_timers, host.fire_dispatch_count());
        self.layer.police_effectiveness = eff(&self.layer.police_timers, host.police_dispatch_count());
    }

    /// `DoOnOccupantRemoval` (only while assessing and established).
    pub(crate) fn on_occupant_removed(&mut self, o: u64) {
        let host = self.host.clone();
        if !self.layer.assessing || o == 0 || !host.city_established() {
            return;
        }
        let t = host.occupant_type(o);
        let prop_cost = host.occupant_prop_i64(o, prop::DEMOLITION_COST);
        let mut cost = prop_cost.unwrap_or(0);
        if t == occupant::BUILDING {
            let flags = host.building_type_flags(o);
            let d = &mut self.layer.damage;
            d.building_count = d.building_count.wrapping_add(1);
            for (i, bit) in KIND_BITS.iter().enumerate() {
                if flags & bit != 0 {
                    d.kinds[i].0 = d.kinds[i].0.wrapping_add(1);
                    d.kinds[i].1 = d.kinds[i].1.wrapping_add(cost);
                }
            }
            if flags & SPECIAL_BITS != 0 {
                d.special.push(SpecialBuilding {
                    occupant: o,
                    flags,
                    cost,
                });
            }
        } else if t == occupant::NETWORK && prop_cost.is_none() {
            cost = NETWORK_DEFAULT_COST;
        }
        self.layer.damage.total_cost = self.layer.damage.total_cost.wrapping_add(cost);
    }

    /// `DoDispatchMessage`: data1 = the dispatch type, data2 = the dispatch ID
    /// (the source, dispatch.md §3). Any of the four messages creates the
    /// source's entry [choice, Q12]; `0xE94671EC` starts its timer, the
    /// others stop it.
    pub(crate) fn on_dispatch(&mut self, msg_type: u32, dispatch_type: i64, source: i64) {
        if !self.layer.assessing {
            return;
        }
        let now = self.host.now_ms();
        let map = match dispatch_type {
            1 | 5 => &mut self.layer.fire_timers,
            0 | 6 => &mut self.layer.police_timers,
            _ => return,
        };
        let t = map.entry(source as u32).or_default();
        if msg_type == msg::DISPATCH_ACTIVE {
            t.start(now);
        } else {
            t.stop(now);
        }
    }

    // ---- §2.6 ----

    pub(crate) fn reset_cost(&mut self) {
        self.layer.damage = DamageAssessment::default();
    }

    // ---- §2.7 ----

    /// `RegisterDisasterLocation` (slot 13): the cursor goes to the new
    /// entry (§2.7).
    pub(crate) fn register_location(&mut self, instance: u32, x: i32, z: i32, occ: u64, zoom: u32, focus: bool) {
        let host = self.host.clone();
        if occ != 0 {
            host.add_ref(occ);
        }
        self.layer.locations.push(Location {
            instance,
            x,
            z,
            zoom,
            occupant: occ,
        });
        self.layer.cursor = Some(self.layer.locations.len() - 1);
        if focus && host.follow_disasters() {
            self.go_to_next_disaster();
        }
    }

    /// `UnregisterDisasterLocations(instance)` (slot 14).
    pub(crate) fn unregister_locations(&mut self, instance: u32) {
        self.remove_locations(|l| l.instance == instance);
    }

    /// `UnregisterDisasterLocationByOccupant(instance, occupant)` (slot 15).
    pub(crate) fn unregister_location_by_occupant(&mut self, instance: u32, occ: u64) {
        self.remove_locations(|l| l.instance == instance && l.occupant == occ);
    }

    fn remove_locations(&mut self, f: impl Fn(&Location) -> bool) {
        let host = self.host.clone();
        let mut kept = Vec::with_capacity(self.layer.locations.len());
        for l in std::mem::take(&mut self.layer.locations) {
            if f(&l) {
                if l.occupant != 0 {
                    host.release(l.occupant);
                }
            } else {
                kept.push(l);
            }
        }
        self.layer.locations = kept;
    }

    /// `ResolveDisasterLocation` (`0x00577540`, §2.7): the entry's stored x
    /// and z change **only here**. With an occupant the cell of its position
    /// is written back while the occupant manager still holds it; without one
    /// the instance's `GetLocation` (instance slot 6) writes its x and z when
    /// it answers true. A gone occupant or a false `GetLocation` changes
    /// nothing *[quirk]*: an entry keeps the cell of its last resolution,
    /// (0, 0) if never resolved.
    pub(crate) fn resolve_location(&mut self, index: usize) -> Option<(i32, i32)> {
        let l = *self.layer.locations.get(index)?;
        let resolved = if l.occupant != 0 {
            if self.host.occupant_in_manager(l.occupant) {
                self.host.occupant_cell(l.occupant)
            } else {
                None
            }
        } else {
            // Instance slot 6 `GetLocation`; false changes nothing (§3.2).
            self.instance_info(l.instance).and_then(|i| i.location)
        };
        if let Some((x, z)) = resolved {
            if let Some(e) = self.layer.locations.get_mut(index) {
                e.x = x;
                e.z = z;
            }
            Some((x, z))
        } else {
            Some((l.x, l.z))
        }
    }

    /// `GetMostRecentDisasterLocation` (slot 12): the newest entry overall
    /// (`None`) or of the instance, resolved (§2.7).
    pub(crate) fn most_recent_location(&mut self, instance: Option<u32>) -> Option<(i32, i32)> {
        let index = self
            .layer
            .locations
            .iter()
            .enumerate()
            .rev()
            .find(|(_, l)| instance.map_or(true, |i| l.instance == i))
            .map(|(i, _)| i)?;
        self.resolve_location(index)
    }

    /// `GetDisasterLocationByIndex` (slot 17): the stored values, **not**
    /// resolved (§2.7) *[quirk, DS35]*.
    pub(crate) fn location_by_index(&self, i: usize) -> Option<Location> {
        self.layer.locations.get(i).copied()
    }

    /// `GetNextDisasterLocation` (§2.7): the entry at the cursor (clamped
    /// to the newest), resolved, then the cursor steps back, wrapping to the
    /// newest below 0 (Q10).
    pub(crate) fn next_location(&mut self) -> Option<Location> {
        let n = self.layer.locations.len();
        if n == 0 {
            return None;
        }
        let c = self.layer.cursor.map_or(n - 1, |c| c.min(n - 1));
        self.layer.cursor = Some(if c == 0 { n - 1 } else { c - 1 });
        let l = *self.layer.locations.get(c)?;
        let (x, z) = self.resolve_location(c)?;
        Some(Location { x, z, ..l })
    }

    /// `GoToNextDisaster` (view-side): the next location, then the camera.
    pub(crate) fn go_to_next_disaster(&mut self) {
        if let Some(l) = self.next_location() {
            self.host.go_to_location(l.x, l.z, l.zoom);
        }
    }

    pub(crate) fn layer_snapshot(&self) -> LayerSnapshot {
        let now = self.host.now_ms();
        let l = &self.layer;
        LayerSnapshot {
            initialised: l.initialised,
            disable_counter: l.disable_counter,
            ongoing: l.ongoing,
            assessing: l.assessing,
            damage: l.damage.clone(),
            fire_timers: l.fire_timers.iter().map(|(k, t)| (*k, t.elapsed_s(now))).collect(),
            police_timers: l.police_timers.iter().map(|(k, t)| (*k, t.elapsed_s(now))).collect(),
            overall_s: l.overall.elapsed_s(now),
            fire_effectiveness: l.fire_effectiveness,
            police_effectiveness: l.police_effectiveness,
            locations: l.locations.clone(),
            cursor: l.cursor,
            managers: self.managers.keys().copied().collect(),
        }
    }

    /// An instance's `GetID` (its manager's type); 0 for an unknown ID.
    pub(crate) fn instance_type(&self, instance: u32) -> u32 {
        self.instances.get(&instance).map_or(0, |i| i.mtype)
    }
}
