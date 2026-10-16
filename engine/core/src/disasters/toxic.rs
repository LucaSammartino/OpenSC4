// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The toxic spill (§9, short form).

use super::layer::Timer;
use super::system::{Core, InstanceState};
use super::{instance_class, kind, FIRE_PERIOD_MS, TOXIC_TICK_PRIORITY};

/// `AttemptDisasterStart(x, z)` spills this much (§9).
pub const ATTEMPT_AMOUNT: i32 = 120;

/// One toxic occupant (§9).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ToxicSpill {
    pub occupant: u64,
    pub x: i32,
    pub z: i32,
    pub amount: i32,
    pub hosed: u32,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ToxicState {
    pub timer: Timer,
    pub next_ms: u64,
    pub spills: Vec<ToxicSpill>,
}

/// The toxic instance's observable state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToxicSnapshot {
    pub id: u32,
    pub phase: u32,
    pub spills: Vec<ToxicSpill>,
    pub elapsed_ms: u64,
    pub next_ms: u64,
}

impl Core {
    fn ts(&mut self, id: u32) -> Option<&mut ToxicState> {
        match &mut self.instances.get_mut(&id)?.state {
            InstanceState::Toxic(t) => Some(t),
            _ => None,
        }
    }

    fn ts_ref(&self, id: u32) -> Option<&ToxicState> {
        match &self.instances.get(&id)?.state {
            InstanceState::Toxic(t) => Some(t),
            _ => None,
        }
    }

    /// `AttemptDisasterStart(x, z)`: 120 units at the cell; the instance
    /// is returned [choice].
    pub(crate) fn toxic_attempt_start(&mut self, x: i32, z: i32) -> Option<u32> {
        self.spill_toxic_waste(x, z, ATTEMPT_AMOUNT)
    }

    /// `SpillToxicWaste(x, z, amount)`, creating the one instance on first
    /// use.
    pub(crate) fn spill_toxic_waste(&mut self, x: i32, z: i32, amount: i32) -> Option<u32> {
        if !self.managers.contains_key(&kind::TOXIC) {
            return None;
        }
        let id = match self.toxic_instance {
            Some(id) => id,
            None => {
                let id = self.new_instance(kind::TOXIC, instance_class::TOXIC, InstanceState::Toxic(Box::default()));
                self.toxic_instance = Some(id);
                self.toxic_init(id);
                id
            }
        };
        let host = self.host.clone();
        let existing = self.ts(id).and_then(|t| {
            let s = t.spills.iter_mut().find(|s| s.x == x && s.z == z)?;
            s.amount = s.amount.saturating_add(amount);
            Some((s.occupant, s.amount))
        });
        match existing {
            Some((o, a)) => host.set_toxic_amount(o, a),
            None => {
                if let Some(o) = host.create_toxic_occupant(id, x, z, amount) {
                    if let Some(t) = self.ts(id) {
                        t.spills.push(ToxicSpill {
                            occupant: o,
                            x,
                            z,
                            amount,
                            hosed: 0,
                        });
                    }
                    self.register_location(id, x, z, o, 0, false);
                }
            }
        }
        Some(id)
    }

    /// Instance `Init` (§9): the millisecond timer (first update after
    /// 1000 ms [choice]), the manager, the tick (`0xFF`), the begin. The
    /// phase is 1 while it runs [choice, Q3].
    fn toxic_init(&mut self, id: u32) {
        let now = self.host.now_ms();
        if let Some(t) = self.ts(id) {
            t.timer.restart(now);
            t.next_ms = FIRE_PERIOD_MS;
        }
        if let Some(i) = self.instances.get_mut(&id) {
            i.phase = 1;
        }
        self.add_instance_to_list(kind::TOXIC, id);
        self.register_tick(id, TOXIC_TICK_PRIORITY);
        self.disaster_has_begun(id);
    }

    pub(crate) fn toxic_tick(&mut self, id: u32) {
        let now = self.host.now_ms();
        let due = self.ts_ref(id).is_some_and(|t| t.timer.elapsed_ms(now) >= t.next_ms);
        if due {
            if let Some(t) = self.ts(id) {
                t.next_ms += FIRE_PERIOD_MS;
            }
            self.toxic_update(id);
        }
    }

    /// One update (every 1000 ms): each spill loses 1, or 4 × hosed, floored
    /// at 0; at 0 it goes. With none left the instance ends.
    pub(crate) fn toxic_update(&mut self, id: u32) {
        let host = self.host.clone();
        let list: Vec<ToxicSpill> = self.ts_ref(id).map_or(Vec::new(), |t| t.spills.clone());
        for s in list {
            let dec = if s.hosed == 0 { 1 } else { 4i64 * i64::from(s.hosed) };
            let amount = (i64::from(s.amount) - dec).max(0) as i32;
            if amount == 0 {
                if let Some(t) = self.ts(id) {
                    t.spills.retain(|x| x.occupant != s.occupant);
                }
                host.remove_toxic_occupant(s.occupant);
                self.unregister_location_by_occupant(id, s.occupant);
            } else {
                if let Some(x) = self
                    .ts(id)
                    .and_then(|t| t.spills.iter_mut().find(|x| x.occupant == s.occupant))
                {
                    x.amount = amount;
                }
                host.set_toxic_amount(s.occupant, amount);
            }
        }
        // None left: the instance shuts down (§9; §3.1 ends the disaster).
        if self.ts_ref(id).is_some_and(|t| t.spills.is_empty()) {
            self.unregister_tick(id);
            self.instance_finish(id);
        }
    }

    /// The toxic occupant's hoses (`+n / −n`, clamped at 0).
    pub(crate) fn toxic_set_hosed(&mut self, o: u64, delta: i32) -> bool {
        let Some(id) = self.toxic_instance else { return false };
        match self.ts(id).and_then(|t| t.spills.iter_mut().find(|s| s.occupant == o)) {
            Some(s) => {
                s.hosed = (i64::from(s.hosed) + i64::from(delta)).clamp(0, i64::from(u32::MAX)) as u32;
                true
            }
            None => false,
        }
    }

    /// Instance `Shutdown` (§3.1, Q13): the remaining toxic occupants
    /// removed [choice], `DisasterHasEnded`, then the locations go; the
    /// caller removes the instance from its manager.
    pub(crate) fn toxic_shutdown(&mut self, id: u32) {
        let host = self.host.clone();
        let list = self.ts(id).map_or(Vec::new(), |t| std::mem::take(&mut t.spills));
        for s in list {
            host.remove_toxic_occupant(s.occupant);
            self.unregister_location_by_occupant(id, s.occupant);
        }
        self.disaster_has_ended();
        self.unregister_locations(id);
    }

    pub(crate) fn toxic_snapshot(&self) -> Option<ToxicSnapshot> {
        let id = self.toxic_instance?;
        let i = self.instances.get(&id)?;
        let t = self.ts_ref(id)?;
        let now = self.host.now_ms();
        Some(ToxicSnapshot {
            id,
            phase: i.phase,
            spills: t.spills.clone(),
            elapsed_ms: t.timer.elapsed_ms(now),
            next_ms: t.next_ms,
        })
    }
}
