// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4MySimAgentSimulator` (§1.3, §2.1, §3.2, §6.1, §7.1).

use std::cell::RefCell;
use std::rc::Rc;

use crate::advisors::{Advisor, AdvisorMessage, AdvisorSystem};
use crate::stream::{InStream, OutStream};

use super::sim::MySim;
use super::{msg, MySimHost, AGENT_RECORD_VERSION, MY_STREET_ID, OCCUPANT_TYPE_BUILDING, OCCUPANT_TYPE_FIRE, SLOTS};

/// The agent's fields (§2.1); the slots by advisor ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentState {
    pub initialised: bool,
    pub slots: [Option<i32>; SLOTS],
    pub last_disaster_type: u32,
    pub active_ui_index: i32,
}

impl Default for AgentState {
    fn default() -> AgentState {
        AgentState {
            initialised: false,
            slots: [None; SLOTS],
            last_disaster_type: 0,
            active_ui_index: -1,
        }
    }
}

/// The agent simulator.
pub struct MySimAgent {
    host: Rc<dyn MySimHost>,
    sys: RefCell<Option<Rc<AdvisorSystem>>>,
    sims: RefCell<Vec<Option<Rc<Advisor>>>>,
    s: RefCell<AgentState>,
}

impl MySimAgent {
    pub fn new(host: Rc<dyn MySimHost>) -> Rc<MySimAgent> {
        Rc::new(MySimAgent {
            host,
            sys: RefCell::new(None),
            sims: RefCell::new(vec![None; SLOTS]),
            s: RefCell::new(AgentState::default()),
        })
    }

    pub fn state(&self) -> AgentState {
        self.s.borrow().clone()
    }

    /// The MySim advisor at vector index `i`.
    pub fn sim(&self, i: usize) -> Option<Rc<Advisor>> {
        self.sims.borrow().get(i).cloned().flatten()
    }

    /// `get_count()` (Lua): the vector size.
    pub fn count(&self) -> usize {
        SLOTS
    }

    /// The MySims at indices 0–4, skipping nulls, in index order.
    fn five(&self) -> Vec<Rc<Advisor>> {
        self.sims.borrow()[..5].iter().flatten().cloned().collect()
    }

    fn each(&self, f: impl Fn(&MySim)) {
        for a in self.five() {
            MySim::with(&a, &f);
        }
    }

    /// Slot 3 `Init` (§1.3), once.
    pub fn init(&self, sys: &Rc<AdvisorSystem>) -> bool {
        if self.s.borrow().initialised {
            return true;
        }
        *self.sys.borrow_mut() = Some(sys.clone());
        let mut sims: Vec<Option<Rc<Advisor>>> = vec![None; SLOTS];
        let mut next = 0usize;
        for (id, adv) in sys.advisors() {
            let is_mysim = adv.ext().is_some_and(|e| e.is_mysim());
            if !is_mysim {
                continue;
            }
            if id == MY_STREET_ID {
                sims[5] = Some(adv);
            } else {
                // [choice] Beyond index 4: ignored.
                if next < 5 {
                    sims[next] = Some(adv);
                }
                next += 1;
            }
        }
        {
            let mut s = self.s.borrow_mut();
            for (i, a) in sims.iter().enumerate() {
                s.slots[i] = a.as_ref().map(|a| a.id());
            }
            s.initialised = true;
        }
        *self.sims.borrow_mut() = sims;
        for m in msg::AGENT_SUBSCRIPTIONS {
            self.host.subscribe(m, true);
        }
        self.host.register_agent_lua(true);
        true
    }

    /// Slot 4 `Shutdown`.
    pub fn shutdown(&self) -> bool {
        if !self.s.borrow().initialised {
            return true;
        }
        self.host.register_agent_lua(false);
        for m in msg::AGENT_SUBSCRIPTIONS {
            self.host.subscribe(m, false);
        }
        *self.sims.borrow_mut() = vec![None; SLOTS];
        *self.sys.borrow_mut() = None;
        *self.s.borrow_mut() = AgentState::default();
        true
    }

    /// Slot 5 `GetActiveMySims`: the active MySims among 0–4.
    pub fn get_active_mysims(&self) -> Vec<Rc<Advisor>> {
        self.five()
            .into_iter()
            .filter(|a| MySim::with(a, |m| m.is_active()).unwrap_or(false))
            .collect()
    }

    /// Slot 6 `MoveIn` (§4.1): the first inactive MySim of 0–4.
    #[allow(clippy::too_many_arguments)]
    pub fn move_in(
        &self,
        name: &[u8],
        is_male: bool,
        sign: u8,
        bio: &[u8],
        bitmap_id: u32,
        bitmap_path: &[u8],
        slot: u8,
    ) -> Option<Rc<Advisor>> {
        let a = self
            .five()
            .into_iter()
            .find(|a| MySim::with(a, |m| !m.is_active()).unwrap_or(false))?;
        MySim::with(&a, |m| {
            m.move_in(slot, name, is_male, sign, bio, bitmap_id, bitmap_path)
        });
        Some(a)
    }

    /// Slot 9 `MySimLivesHere(b)`.
    pub fn my_sim_lives_here(&self, b: u64) -> Option<Rc<Advisor>> {
        self.five()
            .into_iter()
            .find(|a| MySim::with(a, |m| m.residence() == Some(b)).unwrap_or(false))
    }

    /// Slot 11 `SetActiveUIPanelSim(slot)`.
    pub fn set_active_ui_panel_sim(&self, slot: i32) {
        let sims = self.sims.borrow();
        let idx = sims[..5]
            .iter()
            .position(|a| {
                a.as_ref()
                    .and_then(|a| MySim::with(a, |m| i32::from(m.slot()) == slot))
                    .unwrap_or(false)
            })
            .map_or(-1, |i| i as i32);
        self.s.borrow_mut().active_ui_index = idx;
    }

    /// Slot 12 `GetActiveUIPanelSim(int32& index)`: the MySim and its vector
    /// index; `index` unchanged without one.
    pub fn get_active_ui_panel_sim(&self, index: &mut i32) -> Option<Rc<Advisor>> {
        let i = self.s.borrow().active_ui_index;
        if i < 0 || i as usize >= SLOTS {
            return None;
        }
        *index = i;
        self.sim(i as usize)
    }

    /// `DoMessage` (§3.2); always true. AdviceCommand names the advisor by
    /// ID in `unknown` [choice].
    pub fn do_message(&self, m: &AdvisorMessage) -> bool {
        let h = self.host.clone();
        match m.msg_type {
            msg::OCCUPANT_INSERTED | msg::OCCUPANT_CHANGED => {
                let Some(o) = m.void1 else { return true };
                if m.msg_type == msg::OCCUPANT_INSERTED {
                    if m.data3 == OCCUPANT_TYPE_BUILDING && h.building(o).is_some() {
                        self.each(|s| s.building_added(o));
                    } else if m.data3 == OCCUPANT_TYPE_FIRE {
                        if let Some(p) = h.occupant_position(o) {
                            self.each(|s| s.fire_added(p[0], p[2]));
                        }
                    }
                }
                if m.data3 == OCCUPANT_TYPE_BUILDING && h.building(o).is_some() {
                    self.each(|s| s.building_changed(o));
                }
            }
            msg::OCCUPANT_REMOVED => {
                let Some(o) = m.void1 else { return true };
                if m.data3 == OCCUPANT_TYPE_BUILDING && h.building(o).is_some() {
                    let cause = if m.data4 == 4 && h.disaster_ongoing() {
                        self.s.borrow().last_disaster_type
                    } else {
                        0
                    };
                    self.each(|s| s.building_removed(o, cause));
                }
            }
            msg::DISASTER_STARTED => {
                self.s.borrow_mut().last_disaster_type = m.data4;
                self.each(|s| s.disaster(m.data4, m.data2 as i32, m.data3 as i32));
            }
            msg::PIPE_BURST => {
                let (x, z) = h.cell_center(m.data1 as i32, m.data2 as i32);
                self.each(|s| s.pipe_burst(x, z));
            }
            msg::TORNADO => {
                let (x, z) = h.cell_center(m.data2 as i32, m.data3 as i32);
                self.each(|s| s.tornado(x, z));
            }
            msg::NEIGHBOR_CONNECTION_REMOVED => self.each(|s| s.neighbor_connection_removed()),
            msg::HOUR_CHANGED => {
                let what = match m.data2 {
                    8 => "leave_for_work",
                    17 => "return_home",
                    22 => "night_sound",
                    _ => return true,
                };
                for a in self.five() {
                    h.dispatch_command(a.id(), what);
                }
            }
            msg::ADVICE_COMMAND => {
                let sys = self.sys.borrow().clone();
                let adv = m.unknown.and_then(|id| sys.and_then(|s| s.get_advisor(id as i32)));
                if let Some(a) = adv {
                    MySim::with(&a, |s| s.do_script_command(m.data1));
                }
            }
            msg::VIEW_CHANGED_1 | msg::VIEW_CHANGED_2 => {
                for a in self.five() {
                    h.dispatch_command(a.id(), "update_residence");
                }
            }
            _ => {}
        }
        true
    }

    /// `Write`: `u16 1` (§6.1).
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(AGENT_RECORD_VERSION);
        out.error() == 0
    }

    /// `Read`: version 1 only.
    pub fn read(&self, inp: &mut dyn InStream) -> bool {
        inp.get_u16() == Some(AGENT_RECORD_VERSION)
    }
}
