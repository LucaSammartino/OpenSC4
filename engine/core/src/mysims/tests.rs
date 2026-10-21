// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The §12 tests and the behaviours of `docs/specs/mysims.md`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::advisors::scripted::{HostCall, ScriptedHost};
use crate::advisors::{clsid as adv_clsid, Advisor, AdvisorEntry, AdvisorMessage, AdvisorSystem, LuaValue};
use crate::rng::{FixedSeeds, RzRandom};
use crate::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue, ResKey, SaveObject, SaveRegistry};

use super::scripted::{MsCall, ScriptedMySimHost, HOST_CLASS};
use super::*;

const HOME: u64 = 0x900;
const HOME_CELL: (i32, i32) = (6, 6);
const G: u32 = 0x1500;

fn entry(class_id: u32, id: i32) -> AdvisorEntry {
    AdvisorEntry {
        class_id,
        id,
        caption: b"MySim".to_vec(),
        name: format!("Sim{id}").into_bytes(),
        advice_type: 1,
    }
}

struct F {
    ah: Rc<ScriptedHost>,
    h: Rc<ScriptedMySimHost>,
    sys: Rc<AdvisorSystem>,
    agent: Rc<MySimAgent>,
}

fn residence(position: [f32; 3]) -> Building {
    Building {
        building_type: 0x10,
        name: b"House".to_vec(),
        groups: vec![0x1000],
        completion: 1.0,
        purpose: 1,
        position,
    }
}

fn workplace() -> Building {
    Building {
        purpose: 2,
        ..residence([0.0; 3])
    }
}

fn lot(cell: (i32, i32), wealth: u8, condition: u8) -> Lot {
    Lot {
        building: None,
        cell,
        rect: (cell.0, cell.1, cell.0, cell.1),
        condition,
        road_access: true,
        wealth,
    }
}

impl F {
    /// The shipped advisors table: MySims 1–5 and the My Street sim (30),
    /// after a plain advisor (23).
    fn new() -> F {
        let ah = ScriptedHost::new();
        let mut table = vec![entry(adv_clsid::ADVISOR, 23)];
        for id in 1..=5 {
            table.push(entry(clsid::MYSIM, id));
        }
        table.push(entry(clsid::MY_STREET_SIM, MY_STREET_ID));
        *ah.advisors.borrow_mut() = Some(table);
        let sys = AdvisorSystem::new(ah.clone(), 0);
        let h = ScriptedMySimHost::new();
        register(&sys, h.clone(), 0);
        assert!(sys.init());
        let agent = MySimAgent::new(h.clone());
        assert!(agent.init(&sys));
        h.add_building(HOME, residence([100.0, 0.0, 100.0]), Some(lot(HOME_CELL, 2, 1)));
        h.take_log();
        ah.take_log();
        F { ah, h, sys, agent }
    }
    /// The MySim at vector index `i`.
    fn adv(&self, i: usize) -> Rc<Advisor> {
        self.agent.sim(i).expect("mysim")
    }
    fn m<R>(&self, i: usize, f: impl FnOnce(&MySim) -> R) -> R {
        MySim::with(&self.adv(i), f).expect("a MySim")
    }
    /// Moves MySim `i` into `home` (state 1).
    fn place(&self, i: usize, home: u64) {
        self.h.eq.set(Some(50.0));
        self.h.hq.set(Some(60.0));
        self.m(i, |m| {
            m.move_in(i as u8, b"Ann", false, 3, b"bio", 7, b"p.png");
            assert!(m.set_residence(Some(home)));
        });
    }
    /// Places MySim `i` at home (state 2).
    fn at_home(&self, i: usize) {
        self.place(i, HOME);
        self.m(i, |m| m.with_state(|s| s.state = 2));
    }
    fn id(&self, i: usize) -> i32 {
        self.adv(i).id()
    }
    fn num(&self, i: usize, name: &str) -> Option<f64> {
        self.h.num(self.id(i), name)
    }
    fn events(&self, i: usize) -> Vec<u32> {
        self.adv(i).state().queue.iter().map(|(e, _)| *e).collect()
    }
    fn clear_events(&self, i: usize) {
        self.adv(i).with_state(|s| s.queue.clear());
    }
    fn subject(&self, i: usize, ev: u32) -> crate::advisors::SubjectState {
        let q = self.adv(i).state().queue;
        let (_, s) = q.iter().find(|(e, _)| *e == ev).expect("event");
        s.as_ref().expect("subject").state()
    }
    fn agent_msg(&self, m: AdvisorMessage) {
        assert!(self.agent.do_message(&m));
    }
    fn reseed(&self, i: usize, seed: u32) {
        self.m(i, |m| {
            m.with_random(|g| {
                *g = RzRandom::new(
                    seed,
                    &FixedSeeds {
                        wall_clock: 1,
                        process_clock: 1,
                    },
                )
            })
        });
    }
}

fn message(msg_type: u32) -> AdvisorMessage {
    AdvisorMessage::of_type(msg_type)
}

fn posts(log: &[MsCall], msg_type: u32) -> usize {
    log.iter()
        .filter(|c| matches!(c, MsCall::Post(p) if p.msg_type == msg_type))
        .count()
}

// ================================================================ §12.1

#[test]
fn t01_ownership() {
    let f = F::new();
    let ids: Vec<Option<i32>> = (0..SLOTS).map(|i| f.agent.sim(i).map(|a| a.id())).collect();
    assert_eq!(ids, vec![Some(1), Some(2), Some(3), Some(4), Some(5), Some(30)]);
    assert_eq!(
        f.agent.state().slots,
        [Some(1), Some(2), Some(3), Some(4), Some(5), Some(30)]
    );
    assert_eq!(f.agent.count(), 6);
    // The agent's subscriptions, in order, and the Lua functions.
    let h = ScriptedMySimHost::new();
    let a = MySimAgent::new(h.clone());
    assert!(a.init(&f.sys));
    let log = h.take_log();
    let subs: Vec<u32> = log
        .iter()
        .filter_map(|c| match c {
            MsCall::Subscribe(m, true) => Some(*m),
            _ => None,
        })
        .collect();
    assert_eq!(subs, msg::AGENT_SUBSCRIPTIONS.to_vec());
    assert_eq!(log.last(), Some(&MsCall::AgentLua(true)));
    // The agent is not a saved object: its record is only `u16 1`.
    let mut w = ByteWriter::new();
    assert!(a.write(&mut w));
    assert_eq!(w.into_bytes(), vec![1, 0]);
    assert!(a.read(&mut ByteReader::new(&[1, 0])));
    assert!(!a.read(&mut ByteReader::new(&[2, 0])));
    assert!(a.shutdown());
    assert!(h.subscribed.borrow().is_empty());
    assert!(a.sim(0).is_none());
}

#[test]
fn t01b_extra_mysims_ignored() {
    let ah = ScriptedHost::new();
    let table: Vec<AdvisorEntry> = (1..=7).map(|id| entry(clsid::MYSIM, id)).collect();
    *ah.advisors.borrow_mut() = Some(table);
    let sys = AdvisorSystem::new(ah, 0);
    let h = ScriptedMySimHost::new();
    register(&sys, h.clone(), 0);
    assert!(sys.init());
    let agent = MySimAgent::new(h);
    assert!(agent.init(&sys));
    assert_eq!(agent.state().slots, [Some(1), Some(2), Some(3), Some(4), Some(5), None]);
}

// ================================================================ §12.2

#[test]
fn t02_move_in_age() {
    for (grid, age) in [(12, 18), (40, 40)] {
        let f = F::new();
        f.h.average_age.borrow_mut().insert(HOME_CELL, grid);
        f.place(0, HOME);
        let s = f.m(0, |m| m.state());
        assert_eq!(s.age, age);
        assert_eq!((s.state, s.wealth, s.generation), (1, 2, 1));
        assert_eq!(s.home_cell, (6, 6));
        assert_eq!(s.home_position, [100.0, 0.0, 100.0]);
        // MySimsChanged posted, a job searched, the commute started.
        let log = f.h.take_log();
        assert_eq!(posts(&log, msg::MYSIMS_CHANGED), 1);
        assert!(log.contains(&MsCall::Dispatch(1, "start_commute".into())));
        assert!(log.contains(&MsCall::Dispatch(1, "move_in_control".into())));
    }
}

#[test]
fn t02b_move_in_picks_first_inactive() {
    let f = F::new();
    f.at_home(0);
    let a = f.agent.move_in(b"Bob", true, 1, b"", 0, b"", 4).expect("a free MySim");
    assert_eq!(a.id(), 2);
    assert_eq!(a.personal_name(), b"Bob".to_vec());
    let s = f.m(1, |m| m.state());
    assert_eq!((s.slot, s.is_male, s.generation, s.state), (4, true, 1, 0));
    // All five active: nothing.
    for i in 1..5 {
        f.m(i, |m| m.with_state(|s| s.state = 2));
    }
    assert!(f.agent.move_in(b"C", true, 1, b"", 0, b"", 0).is_none());
}

// ================================================================ §12.3

#[test]
fn t03_eq_ring() {
    let f = F::new();
    f.h.eq.set(Some(87.5));
    f.m(0, |m| {
        m.move_in(0, b"A", false, 0, b"", 0, b"");
        m.set_residence(Some(HOME));
    });
    let s = f.m(0, |m| m.state());
    assert_eq!((s.eq_ring, s.eq_index, s.eq_delta), ([87.5; 6], 0, 0.0));
    let adv = f.adv(0);
    for (k, eq) in [90.0f32, 95.0, 100.0, 100.0, 100.0, 110.0, 110.0].iter().enumerate() {
        f.h.eq.set(Some(*eq));
        adv.simulate(1);
        let s = f.m(0, |m| m.state());
        if k == 5 {
            assert_eq!(s.eq_delta, 22.5);
            // The refresh runs before the ring moves: step 5's delta.
            assert_eq!(f.num(0, "delta_eq"), Some(12.0));
        }
        if k == 6 {
            assert_eq!(s.eq_delta, 20.0);
            assert_eq!(f.num(0, "delta_eq"), Some(22.0));
        }
    }
    // The next refresh shows 20.
    f.m(0, |m| m.update_variables());
    assert_eq!(f.num(0, "delta_eq"), Some(20.0));
}

// ================================================================ §12.4

#[test]
fn t04_eq_not_positive_ignored() {
    let f = F::new();
    f.place(0, HOME);
    assert_eq!(f.num(0, "eq"), Some(50.0));
    f.h.eq.set(Some(0.0));
    f.m(0, |m| m.update_variables());
    assert_eq!(f.num(0, "eq"), Some(50.0));
    assert_eq!(f.m(0, |m| m.state().eq), 50.0);
}

// ================================================================ §12.5

#[test]
fn t05_ageing() {
    let f = F::new();
    f.at_home(0);
    f.m(0, |m| m.with_state(|s| s.birth_month = 3));
    let age0 = f.m(0, |m| m.state().age);
    let adv = f.adv(0);
    let mut ages = vec![];
    for mc in [14, 15, 27] {
        adv.with_state(|s| s.month_counter = mc - 1);
        adv.simulate(1);
        ages.push(f.m(0, |m| m.state().age) - age0);
    }
    assert_eq!(ages, vec![0, 1, 2]);
}

#[test]
fn t05b_skipped_when_blocked_or_empty() {
    let f = F::new();
    f.at_home(0);
    f.m(0, |m| m.with_state(|s| s.birth_month = 0));
    f.h.ui_blocked.set(true);
    let adv = f.adv(0);
    adv.with_state(|s| s.month_counter = 11);
    adv.simulate(1);
    assert_eq!(f.m(0, |m| m.state().age), 18);
    // State 0: nothing either.
    let adv2 = f.adv(1);
    f.h.ui_blocked.set(false);
    f.h.take_log();
    adv2.simulate(1);
    adv2.simulate(0);
    assert!(f.h.take_log().is_empty());
}

// ================================================================ §12.6

#[test]
fn t06_job_search() {
    let f = F::new();
    f.at_home(0);
    let adv = f.adv(0);
    for d in 1..=3 {
        f.ah.set_day(1000 + d);
        adv.simulate(0);
    }
    assert_eq!(f.m(0, |m| m.state().jobless_days), 3);
    f.m(0, |m| m.update_variables());
    assert_eq!(f.num(0, "unemployed_days"), Some(3.0));
    // Retired: no search.
    f.m(0, |m| m.with_state(|s| s.retired = true));
    adv.simulate(0);
    assert_eq!(f.m(0, |m| m.state().jobless_days), 3);
    f.m(0, |m| m.with_state(|s| s.retired = false));
    // The dispatch reports a job.
    let job = 0x500;
    f.h.add_building(job, residence([0.0; 3]), Some(lot((9, 9), 2, 1)));
    *f.h.job_destination.borrow_mut() = Some(JobDestination::Lot {
        building: job,
        cell: (9, 9),
    });
    f.clear_events(0);
    let mut m = message(msg::DISPATCH_NOTIFICATION);
    m.data1 = dispatch_note::JOB_FOUND;
    assert!(adv.do_message(&m));
    assert_eq!(f.num(0, "unemployed_days"), Some(0.0));
    assert_eq!(f.num(0, "employed"), Some(1.0));
    assert_eq!(f.events(0), vec![event::NEW_JOB]);
    assert_eq!(f.m(0, |m| m.job()), Some(job));
    assert_eq!(*f.h.refs.borrow().get(&job).unwrap(), 1);
}

#[test]
fn t06b_neighbor_job() {
    let f = F::new();
    f.at_home(0);
    f.h.neighbor_names.borrow_mut().insert((-1, 5), b"Next".to_vec());
    f.m(0, |m| m.job_found(JobDestination::Neighbor { cell: (-1, 5) }));
    assert_eq!(f.num(0, "job_is_neighbor"), Some(1.0));
    assert_eq!(f.num(0, "job_has_water"), Some(1.0));
    assert_eq!(f.h.field(f.id(0), "job_city"), Some(Field::Str(b"Next".to_vec())));
    // Neighbour connection removed → the next day checks the commute.
    f.agent_msg(message(msg::NEIGHBOR_CONNECTION_REMOVED));
    assert!(f.m(0, |m| m.state().commute_check));
    f.h.commute.set(None);
    f.clear_events(0);
    f.h.find_job_calls.borrow_mut().clear();
    f.adv(0).simulate(0);
    let s = f.m(0, |m| m.state());
    assert!(!s.commute_check && !s.neighbor_job);
    // FailCommute: a new job search (FindNewJob's call), no reset command.
    assert!(!f.h.take_log().contains(&MsCall::Dispatch(1, "reset_commute".into())));
    assert_eq!(*f.h.find_job_calls.borrow(), vec![1]);
}

#[test]
fn t06c_commute_time() {
    let f = F::new();
    f.at_home(0);
    f.m(0, |m| m.job_found(JobDestination::Neighbor { cell: (1, 1) }));
    f.h.commute.set(Some(-3.0));
    f.m(0, |m| m.check_commute());
    assert_eq!(f.m(0, |m| m.state().last_commute_time), 0);
    f.h.commute.set(Some(42.9));
    f.m(0, |m| m.check_commute());
    assert_eq!(f.m(0, |m| m.state().last_commute_time), 42);
    f.clear_events(0);
    f.h.find_job_calls.borrow_mut().clear();
    f.h.commute.set(None);
    f.m(0, |m| m.check_commute());
    assert_eq!(f.events(0), vec![event::COMMUTE_FAILED]);
    // FailCommute order: QuitJob, the event, then the job search.
    assert_eq!(*f.h.find_job_calls.borrow(), vec![f.id(0)]);
    assert_eq!(f.m(0, |m| m.state().state), 2);
}

// ================================================================ §12.7

#[test]
fn t07_home_destroyed() {
    let f = F::new();
    f.at_home(0);
    for (t, x) in [(0x101u64, 10.0f32), (0x102, 20.0), (0x103, 30.0)] {
        f.h.add_building(t, residence([x, 0.0, 0.0]), Some(lot((1, 1), 2, 1)));
    }
    f.reseed(0, 1);
    f.clear_events(0);
    let mut m = message(msg::OCCUPANT_REMOVED);
    m.data3 = OCCUPANT_TYPE_BUILDING;
    m.void1 = Some(HOME);
    f.agent_msg(m);
    assert_eq!(f.events(0), vec![event::HOME_DESTROYED]);
    // One draw: n = 4 → 0 → the first candidate.
    let mut want = RzRandom::new(
        1,
        &FixedSeeds {
            wall_clock: 1,
            process_clock: 1,
        },
    );
    assert_eq!(want.uniform_u32(4), 0);
    assert_eq!(f.m(0, |m| m.with_random(|g| g.lcg_state())), want.lcg_state());
    let s = f.m(0, |m| m.state());
    assert_eq!((f.m(0, |m| m.residence()), s.state), (Some(0x101), 1));
}

#[test]
fn t07b_find_new_residence_none() {
    let f = F::new();
    f.at_home(0);
    f.clear_events(0);
    // Only the home: unchanged → move out, can't find a home; false.
    assert!(!f.m(0, |m| m.find_new_residence()));
    assert_eq!(f.events(0), vec![event::CANT_FIND_HOME]);
    assert_eq!(f.m(0, |m| m.residence()), None);
    assert_eq!(*f.h.refs.borrow().get(&HOME).unwrap(), 0);
}

#[test]
fn t07c_residence_taken_by_other_sim() {
    let f = F::new();
    f.at_home(0);
    let other = 0x101;
    f.h.add_building(other, residence([0.0; 3]), Some(lot((1, 1), 3, 1)));
    f.place(1, other);
    assert!(!f.m(0, |m| m.change_residence(other)));
    assert!(f.agent.my_sim_lives_here(other).is_some_and(|a| a.id() == 2));
    // Free again: the move works; the wealth follows the lot.
    f.m(1, |m| m.move_out());
    assert!(f.m(0, |m| m.change_residence(other)));
    let s = f.m(0, |m| m.state());
    assert_eq!((s.wealth, s.state), (3, 1));
    assert!(
        f.m(0, |m| !m.test_for_residence(HOME, false, true, false)),
        "wealth 3 vs lot 2"
    );
}

// ================================================================ §12.8

#[test]
fn t08_home_destroyed_by_disaster() {
    let f = F::new();
    f.at_home(0);
    let mut d = message(msg::DISASTER_STARTED);
    d.data2 = 10;
    d.data3 = 20;
    d.data4 = disaster::RIOT;
    f.agent_msg(d);
    assert_eq!(f.agent.state().last_disaster_type, disaster::RIOT);
    f.clear_events(0);
    f.h.disaster_ongoing.set(true);
    let mut m = message(msg::OCCUPANT_REMOVED);
    m.data3 = OCCUPANT_TYPE_BUILDING;
    m.data4 = 4;
    m.void1 = Some(HOME);
    f.agent_msg(m);
    let s = f.m(0, |m| m.state());
    assert_eq!(s.last_local_disaster, disaster::RIOT);
    assert_eq!(s.state, 2);
    assert_eq!(f.m(0, |m| m.residence()), None);
    assert_eq!(f.events(0), vec![event::HOME_DESTROYED_BY_DISASTER]);
    // No ongoing disaster: cause 0.
    f.at_home(1);
    f.h.disaster_ongoing.set(false);
    f.agent_msg(m);
    assert_eq!(f.m(1, |m| m.state().last_local_disaster), 0);
}

// ================================================================ §12.9

#[test]
fn t09_job_destroyed() {
    let f = F::new();
    f.at_home(0);
    let job = 0x500;
    f.h.add_building(job, residence([0.0; 3]), Some(lot((9, 9), 2, 1)));
    f.m(0, |m| {
        m.job_found(JobDestination::Lot {
            building: job,
            cell: (9, 9),
        });
        m.with_state(|s| s.state = 3);
    });
    f.clear_events(0);
    let mut m = message(msg::OCCUPANT_REMOVED);
    m.data3 = OCCUPANT_TYPE_BUILDING;
    m.void1 = Some(job);
    f.agent_msg(m);
    assert_eq!(f.events(0), vec![event::JOB_DESTROYED]);
    assert_eq!(f.num(0, "employed"), Some(0.0));
    assert_eq!(f.m(0, |m| m.state().state), 2);
    assert_eq!(f.m(0, |m| m.job()), None);
}

// ================================================================ §12.10

#[test]
fn t10_script_commands() {
    let f = F::new();
    for i in 0..3 {
        f.at_home(i);
    }
    f.h.take_log();
    let mut m = message(msg::ADVICE_COMMAND);
    m.data1 = command::DIE;
    m.unknown = Some(2);
    f.agent_msg(m);
    assert_eq!(f.m(1, |m| m.state().state), 0);
    assert_eq!(f.m(0, |m| m.state().state), 2);
    assert_eq!(f.m(2, |m| m.state().state), 2);
    let log = f.h.take_log();
    assert_eq!(posts(&log, msg::MYSIMS_CHANGED), 1);
    let triggers: Vec<&MsCall> = log.iter().filter(|c| matches!(c, MsCall::Trigger(..))).collect();
    assert_eq!(
        triggers,
        vec![&MsCall::Trigger(DIED_TRIGGER.into(), [100.0, 0.0, 100.0])]
    );
    // State 0: ignored.
    f.agent_msg(m);
    assert!(f.h.take_log().is_empty());
    // IncreaseWealth.
    f.m(0, |m| m.with_state(|s| s.wealth = 3));
    f.reseed(0, 1);
    let lcg = f.m(0, |m| m.with_random(|g| g.lcg_state()));
    f.m(0, |m| m.do_script_command(command::INCREASE_WEALTH));
    assert_eq!(f.m(0, |m| m.state().wealth), 3);
    assert_eq!(f.m(0, |m| m.with_random(|g| g.lcg_state())), lcg);
    let job = 0x500;
    f.h.add_building(job, workplace(), None);
    f.m(0, |m| {
        m.with_state(|s| s.wealth = 1);
        m.job_found(JobDestination::Lot {
            building: job,
            cell: (0, 0),
        });
        m.do_script_command(command::INCREASE_WEALTH);
    });
    assert_eq!(f.m(0, |m| m.state().wealth), 2);
    assert_eq!(f.m(0, |m| m.job()), None);
    assert_ne!(
        f.m(0, |m| m.with_random(|g| g.lcg_state())),
        lcg,
        "one FindNewResidence draw"
    );
}

#[test]
fn t10b_retire_quit_leave() {
    let f = F::new();
    f.at_home(0);
    f.m(0, |m| m.do_script_command(command::RETIRE));
    assert!(f.m(0, |m| m.state().retired));
    f.m(0, |m| m.do_script_command(command::LEAVE_CITY));
    assert_eq!(f.m(0, |m| m.state().state), 0);
    assert_eq!(f.m(0, |m| m.residence()), None);
    assert!(f.h.take_log().contains(&MsCall::Dispatch(1, "shutdown".into())));
}

// ================================================================ §12.11

#[test]
fn t11_spawn() {
    let f = F::new();
    f.at_home(0);
    f.m(0, |m| {
        m.with_state(|s| {
            s.retired = true;
            s.age = 80;
        })
    });
    f.h.new_age.set(Some(30));
    f.h.take_log();
    f.m(0, |m| m.do_script_command(command::SPAWN));
    let s = f.m(0, |m| m.state());
    assert_eq!((s.age, s.generation, s.retired, s.bio.len()), (30, 2, false, 0));
    assert_eq!(posts(&f.h.take_log(), msg::MYSIMS_CHANGED), 1);
}

// ================================================================ §12.12

#[test]
fn t12_building_added() {
    let f = F::new();
    f.at_home(0);
    let b = 0x700;
    f.h.add_building(b, residence([130.0, 0.0, 140.0]), None);
    f.clear_events(0);
    let mut m = message(msg::OCCUPANT_INSERTED);
    m.data3 = OCCUPANT_TYPE_BUILDING;
    m.void1 = Some(b);
    f.agent_msg(m);
    assert_eq!(f.events(0), vec![event::BUILDING_ADD]);
    let s = f.subject(0, event::BUILDING_ADD);
    assert_eq!((s.data1, s.object), (50, Some(b)));
    // A fire occupant: a fire-started event at its position.
    f.clear_events(0);
    f.h.positions.borrow_mut().insert(0x800, [100.0, 0.0, 103.0]);
    let mut m = message(msg::OCCUPANT_INSERTED);
    m.data3 = OCCUPANT_TYPE_FIRE;
    m.void1 = Some(0x800);
    f.agent_msg(m);
    assert_eq!(f.subject(0, event::FIRE_STARTED).data1, 3);
}

// ================================================================ §12.13

#[test]
fn t13_riot_subject() {
    let f = F::new();
    f.at_home(0);
    f.clear_events(0);
    f.m(0, |m| m.disaster(disaster::RIOT, 10, 20));
    let s = f.subject(0, event::RIOT);
    assert_eq!(s.location, Some((10.0, 20.0)));
    let (x, z) = f.h.cell_center(10, 20);
    assert_eq!(s.data1, distance([100.0, 0.0, 100.0], x, z));
    assert_eq!(s.data1, 237);
    f.clear_events(0);
    f.m(0, |m| m.disaster(disaster::FIRE, 10, 20));
    assert!(f.events(0).is_empty());
}

#[test]
fn t13b_pipe_and_tornado() {
    let f = F::new();
    f.at_home(0);
    f.clear_events(0);
    let mut m = message(msg::PIPE_BURST);
    m.data1 = 6;
    m.data2 = 6;
    f.agent_msg(m);
    let mut t = message(msg::TORNADO);
    t.data2 = 6;
    t.data3 = 7;
    f.agent_msg(t);
    assert_eq!(f.events(0), vec![event::PIPE_BURST, event::TORNADO]);
    assert_eq!(f.subject(0, event::PIPE_BURST).data1, 5);
    assert_eq!(f.subject(0, event::TORNADO).data1, 20);
}

// ================================================================ §12.14

#[test]
fn t14_trip_percentages() {
    let f = F::new();
    f.at_home(0);
    for (i, (c, w)) in [(2u8, 1u8), (2, 2), (4, 3)].iter().enumerate() {
        let mut l = lot((i as i32, 0), *w, *c);
        l.building = Some(0x600 + i as u64);
        f.h.rect_lots.borrow_mut().push(l);
    }
    // A lot without a building does not count.
    f.h.rect_lots.borrow_mut().push(lot((0, 1), 3, 4));
    f.m(0, |m| m.trip_area(-5, -5, 5, 5));
    let want = [
        ("trip_distress1_p", 66.0),
        ("trip_distress2_p", 0.0),
        ("trip_distress3_p", 33.0),
        ("trip_wealth_low_p", 33.0),
        ("trip_wealth_med_p", 33.0),
        ("trip_wealth_high_p", 33.0),
        ("trip_landmark_nearby", 0.0),
        ("trip_landmark_funding_p", 0.0),
        ("trip_any_disaster_nearby", 0.0),
    ];
    for (n, v) in want {
        assert_eq!(f.num(0, n), Some(v), "{n}");
    }
}

#[test]
fn t14b_trip_landmark_and_disasters() {
    let f = F::new();
    f.at_home(0);
    let lm = 0x650;
    f.h.add_building(
        lm,
        Building {
            groups: vec![GROUP_LANDMARK],
            name: b"Tower".to_vec(),
            ..residence([0.0; 3])
        },
        None,
    );
    f.h.landmark_funding.borrow_mut().insert(lm, 0.75);
    let mut l = lot((0, 0), 0, 0);
    l.building = Some(lm);
    f.h.rect_lots.borrow_mut().push(l);
    *f.h.disaster_locations.borrow_mut() = vec![(disaster::FIRE, (1, 1)), (0x1234, (50, 50))];
    f.m(0, |m| m.trip_area(-5, -5, 5, 5));
    assert_eq!(f.num(0, "trip_landmark_nearby"), Some(1.0));
    assert_eq!(f.num(0, "trip_landmark_funding_p"), Some(75.0));
    assert_eq!(
        f.h.field(f.id(0), "trip_landmark_name"),
        Some(Field::Str(b"Tower".to_vec()))
    );
    assert_eq!(f.num(0, "trip_fire_nearby"), Some(1.0));
    assert_eq!(f.num(0, "trip_riot_nearby"), Some(0.0));
}

#[test]
fn t14c_trip_state_changes() {
    let f = F::new();
    f.place(0, HOME);
    let id = f.id(0);
    f.h.dispatch.borrow_mut().insert(
        id,
        Dispatch {
            state: 3,
            flags: 0x8000,
            at_home: true,
            travel_type: 6,
            ..Dispatch::default()
        },
    );
    f.clear_events(0);
    f.h.take_log();
    // Arrival home (state 1 → 2) announces the move-in.
    let mut m = message(msg::DISPATCH_NOTIFICATION);
    m.data1 = dispatch_note::TRIP_STATE_B;
    m.data2 = 2;
    m.data3 = 3;
    assert!(f.adv(0).do_message(&m));
    assert_eq!(f.m(0, |m| m.state().state), 2);
    assert_eq!(f.num(0, "trip_at_location"), Some(1.0));
    let log = f.h.take_log();
    assert_eq!(posts(&log, msg::MYSIM_MOVED_IN), 1);
    assert_eq!(
        posts(&log, msg::MYSIM_UPDATE),
        1,
        "trip state changed, flags block not run"
    );
    assert_eq!(f.events(0), vec![event::MOVE_IN]);
    // Flags: bit 15 set → approaching; travel type 6 → subway.
    f.clear_events(0);
    let mut m = message(msg::DISPATCH_NOTIFICATION);
    m.data1 = dispatch_note::TRIP_STATE_A;
    m.data2 = 0;
    m.data3 = 0x8011;
    assert!(f.adv(0).do_message(&m));
    assert_eq!(f.events(0), vec![event::APPROACHING_LOCATION]);
    assert_eq!(f.num(0, "trip_on_subway"), Some(1.0));
    assert_eq!(f.num(0, "last_trip_type"), Some(6.0));
    assert_eq!(f.num(0, "trip_idle"), Some(1.0));
    assert_eq!(f.num(0, "trip_at_transit_switch"), Some(1.0));
    // Only bit 0x100 changed: nothing.
    f.clear_events(0);
    f.h.fields.borrow_mut().clear();
    f.m(0, |m| m.update_trip_variables(-1, -1, 0x100, 0, false));
    assert_eq!(f.num(0, "trip_idle"), None);
}

// ================================================================ §12.15

#[test]
fn t15_local_distances() {
    let f = F::new();
    // The home first (token order), then a building 37 m away.
    let home = 0x100;
    f.h.add_building(
        home,
        Building {
            groups: vec![G],
            ..residence([100.0, 0.0, 100.0])
        },
        Some(lot(HOME_CELL, 2, 1)),
    );
    f.h.add_building(
        0x101,
        Building {
            groups: vec![G],
            ..residence([137.0, 0.0, 100.0])
        },
        Some(lot((8, 6), 2, 1)),
    );
    // Not finished / abandoned: filtered out.
    f.h.add_building(
        0x102,
        Building {
            groups: vec![G],
            completion: 0.5,
            ..residence([101.0, 0.0, 100.0])
        },
        None,
    );
    f.h.add_building(
        0x103,
        Building {
            groups: vec![G],
            ..residence([102.0, 0.0, 100.0])
        },
        Some(lot((7, 6), 2, 4)),
    );
    // Outside the box.
    f.h.add_building(
        0x104,
        Building {
            groups: vec![0x777],
            ..residence([0.0; 3])
        },
        Some(lot((40, 40), 2, 1)),
    );
    f.place(0, home);
    assert_eq!(f.m(0, |m| m.distance_to_closest(G)), 37);
    assert_eq!(f.m(0, |m| m.distance_to_closest(0x4444)), NO_DISTANCE);
    assert_eq!(f.m(0, |m| m.distance_to_closest(0x777)), NO_DISTANCE);
    // State 0: none.
    f.m(0, |m| m.with_state(|s| s.state = 0));
    assert_eq!(f.m(0, |m| m.distance_to_closest(G)), NO_DISTANCE);
}

#[test]
fn t15b_closest_special() {
    let f = F::new();
    f.at_home(0);
    f.h.add_building(
        0x300,
        Building {
            building_type: 0xAB,
            ..residence([100.0, 0.0, 160.0])
        },
        None,
    );
    f.h.add_building(
        0x301,
        Building {
            building_type: 0xAB,
            ..residence([100.0, 0.0, 110.0])
        },
        None,
    );
    *f.h.special.borrow_mut() = vec![0x300, 0x301];
    assert_eq!(f.m(0, |m| m.distance_to_closest_special(0xAB)), 10);
    assert_eq!(f.m(0, |m| m.distance_to_closest_special(0xAC)), NO_DISTANCE);
}

// ================================================================ §12.16

#[test]
fn t16_school_grade() {
    let f = F::new();
    f.at_home(0);
    for (t, z, g) in [(0x400u64, 220.0f32, 0.6f32), (0x401, 180.0, 0.9)] {
        f.h.add_building(
            t,
            Building {
                groups: vec![G],
                ..residence([100.0, 0.0, z])
            },
            None,
        );
        f.h.grades.borrow_mut().insert(t, g);
    }
    *f.h.schools.borrow_mut() = vec![0x400, 0x401];
    assert_eq!(f.m(0, |m| m.local_grade(G, false)), 0.9);
    assert_eq!(f.m(0, |m| m.local_grade(G, true)), 0.0);
    assert_eq!(f.m(0, |m| m.local_grade(0x99, false)), 0.0);
    // Lua: × 100 with the MySim as the context advisor.
    let a = f.sys.create_advice(adv_clsid::ADVICE).unwrap();
    a.set_advisor_id(f.id(0));
    f.sys.push_context_advice(a);
    let r = f
        .sys
        .call_lua("mysim_local_school_grade", &[LuaValue::Number(f64::from(G))])
        .unwrap();
    assert_eq!(r, vec![LuaValue::Number(90.0)]);
    let r = f
        .sys
        .call_lua("mysim_distance_to_closest_building", &[LuaValue::Number(1.0)])
        .unwrap();
    assert_eq!(r, vec![LuaValue::Number(f64::from(NO_DISTANCE))]);
}

// ================================================================ §12.17

#[test]
fn t17_active_ui_panel_sim() {
    let f = F::new();
    for (i, slot) in [4u8, 3, 2, 1, 0].iter().enumerate() {
        f.m(i, |m| m.with_state(|s| s.slot = *slot));
    }
    f.agent.set_active_ui_panel_sim(1);
    let mut i = -7;
    let a = f.agent.get_active_ui_panel_sim(&mut i).unwrap();
    assert_eq!((a.id(), i), (4, 3));
    f.agent.set_active_ui_panel_sim(7);
    let mut i = -7;
    assert!(f.agent.get_active_ui_panel_sim(&mut i).is_none());
    assert_eq!(i, -7);
}

// ================================================================ §12.18, §12.19

struct Ids;
impl RefResolver for Ids {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: u64::from(object_id) + 0x1000,
            class_id,
        })
    }
}

fn record(adv: &Rc<Advisor>) -> Vec<u8> {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg);
    assert!(adv.write(&mut w));
    w.into_bytes()
}

fn advisor_part(adv: &Rc<Advisor>) -> Vec<u8> {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg);
    adv.write_record(&mut w);
    w.into_bytes()
}

fn key(n: u32) -> ResKey {
    ResKey {
        type_id: n,
        group: n + 1,
        instance: n + 2,
    }
}

fn full_state(f: &F) {
    f.at_home(0);
    let job = 0x500;
    f.h.add_building(job, residence([0.0; 3]), None);
    f.m(0, |m| {
        m.job_found(JobDestination::Lot {
            building: job,
            cell: (9, 8),
        });
        m.with_state(|s| {
            s.neighbor_cell = (-1, -2);
            s.jobless_days = 4;
            s.vehicle_key = key(10);
            s.is_male = true;
            s.age = 33;
            s.birth_month = 5;
            s.last_commute_time = 17;
            s.retired = true;
            s.last_local_disaster = 0x77;
            s.eq_index = 2;
            s.eq_delta = 1.5;
            s.eq_ring = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
            s.hq_index = 4;
            s.hq_delta = -2.5;
            s.hq_ring = [7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
            s.car_key = key(20);
            s.pedestrian_key = key(30);
            s.unnamed_pair = (-5, 6);
            s.dispatch = Some(0x5555);
            s.pylon = (1, 0x0102_0304_0506_0708, 1);
            s.dispatch_tool = Some(0x6666);
            s.unnamed_flag = true;
            s.last_player_trip_day = 1234;
        });
    });
}

#[test]
fn t18_record_round_trip() {
    let f = F::new();
    full_state(&f);
    let adv = f.adv(0);
    let rec = record(&adv);
    // The prefix, field by field.
    let mut w = ByteWriter::new();
    w.put_u16(17);
    w.put_u8(2);
    w.put_u8(0);
    w.put_u32(1);
    w.put_u32(HOST_CLASS);
    w.put_u32(6);
    w.put_u32(6);
    w.put_u32(2);
    w.put_u32(HOST_CLASS);
    for v in [-1i32, -2, 9, 8] {
        w.put_i32(v);
    }
    w.put_u32(4);
    w.put_res_key(key(10));
    w.put_u8(2);
    w.put_u16(1);
    w.put_u8(3);
    w.put_bool(true);
    w.put_str(b"bio");
    w.put_u8(33);
    w.put_u8(5);
    w.put_u32(7);
    w.put_str(b"p.png");
    w.put_u32(17);
    w.put_bool(true);
    w.put_u32(0x77);
    for v in [100.0f32, 0.0, 100.0] {
        w.put_f32(v);
    }
    w.put_u8(2);
    w.put_f32(1.5);
    for v in 1..=6 {
        w.put_f32(v as f32);
    }
    w.put_u8(4);
    w.put_f32(-2.5);
    for v in 7..=12 {
        w.put_f32(v as f32);
    }
    w.put_res_key(key(20));
    w.put_res_key(key(30));
    w.put_i32(-5);
    w.put_i32(6);
    w.put_u32(3);
    w.put_u32(HOST_CLASS);
    w.put_u8(1);
    w.put_u64(0x0102_0304_0506_0708);
    w.put_u8(1);
    w.put_bool(true);
    w.put_u32(4);
    w.put_u32(HOST_CLASS);
    w.put_bool(true);
    w.put_i32(1234);
    let mut want = w.into_bytes();
    want.extend(advisor_part(&adv));
    assert_eq!(rec, want);
    // Read into a fresh MySim, write again: identical bytes.
    let adv2 = f.sys.create_advisor(clsid::MYSIM).unwrap();
    assert!(adv2.read(&mut ByteReader::with_resolver(&rec, &mut Ids)));
    let s = MySim::with(&adv2, |m| m.state()).unwrap();
    assert_eq!(
        (s.residence, s.job, s.dispatch, s.dispatch_tool),
        (Some(0x1001), Some(0x1002), Some(0x1003), Some(0x1004))
    );
    assert_eq!(s.last_player_trip_day, 1234);
    assert_eq!(adv2.personal_name(), b"Ann".to_vec());
    assert_eq!(record(&adv2), rec);
}

#[test]
fn t18b_state_zero_record() {
    let f = F::new();
    let adv = f.adv(1);
    let rec = record(&adv);
    let mut want = vec![0x11, 0x00, 0x00];
    want.extend(advisor_part(&adv));
    assert_eq!(rec, want);
    let adv2 = f.sys.create_advisor(clsid::MYSIM).unwrap();
    assert!(adv2.read(&mut ByteReader::with_resolver(&rec, &mut Ids)));
    assert_eq!(record(&adv2), rec);
    // Other versions are rejected [choice].
    let mut bad = rec.clone();
    bad[0] = 18;
    assert!(!adv2.read(&mut ByteReader::with_resolver(&bad, &mut Ids)));
    bad[0] = 10;
    assert!(!adv2.read(&mut ByteReader::with_resolver(&bad, &mut Ids)));
}

#[test]
fn t19_version_14() {
    let f = F::new();
    full_state(&f);
    let adv = f.adv(0);
    let rec = record(&adv);
    // Rows 1–23 of the v17 record (the same bytes up to the keys).
    let rows_1_23 =
        2 + 1 + 1 + 8 + 8 + 8 + 16 + 4 + 12 + 1 + 2 + 1 + 1 + (4 + 3) + 2 + 4 + (4 + 5) + 4 + 1 + 4 + 12 + 29 + 29;
    let mut b = rec[..rows_1_23].to_vec();
    b[0] = 14;
    let mut w = ByteWriter::new();
    for v in 0..5 {
        w.put_i32(v);
    }
    w.put_u32(0); // dispatch: null
    w.put_u8(1);
    w.put_u64(99);
    w.put_u8(0);
    w.put_bool(false);
    w.put_bool(true);
    b.extend(w.into_bytes());
    b.extend(advisor_part(&adv));
    let adv2 = f.sys.create_advisor(clsid::MYSIM).unwrap();
    assert!(adv2.read(&mut ByteReader::with_resolver(&b, &mut Ids)));
    let s = MySim::with(&adv2, |m| m.state()).unwrap();
    assert_eq!((s.car_key, s.pedestrian_key), (ResKey::default(), ResKey::default()));
    assert_eq!((s.last_player_trip_day, s.unnamed_pair), (0, (0, 0)));
    assert_eq!((s.age, s.hq_ring[5], s.pylon), (33, 12.0, (1, 99, 0)));
    assert!(s.unnamed_flag);
    assert_eq!(s.dispatch, None);
    // Written back as version 17.
    assert_eq!(&record(&adv2)[..2], &[17, 0]);
}

// ================================================================ §12.20

#[test]
fn t20_no_feedback() {
    let f = F::new();
    for i in 0..5 {
        let t = 0x200 + i as u64;
        f.h.add_building(
            t,
            residence([i as f32 * 10.0, 0.0, 0.0]),
            Some(lot((i as i32, 0), 1, 1)),
        );
        f.place(i, t);
        f.m(i, |m| m.with_state(|s| s.state = 2));
    }
    f.h.take_log();
    f.ah.take_log();
    for month in 0..24 {
        for d in 0..28 {
            f.ah.set_day(2000 + month * 28 + d);
            for i in 0..5 {
                let adv = f.adv(i);
                if d == 27 {
                    adv.simulate(1);
                }
                adv.simulate(0);
            }
        }
    }
    // The MySims only talk to the Lua table, the dispatch and the UI.
    for c in f.h.take_log() {
        assert!(
            matches!(c, MsCall::Activate(_) | MsCall::Post(_) | MsCall::Dispatch(..)),
            "{c:?}"
        );
    }
    // No simulator-facing advisor-host call.
    for c in f.ah.take_log() {
        assert!(
            !matches!(
                c,
                HostCall::History { .. }
                    | HostCall::Cheat { .. }
                    | HostCall::CancelDeal { .. }
                    | HostCall::SetOrdinance { .. }
            ),
            "{c:?}"
        );
    }
}

// ================================================================ more behaviours

#[test]
fn t21_quick_and_full_variables() {
    let f = F::new();
    f.h.life_expectancy.set(Some(77));
    f.h.grids.borrow_mut().insert((Grid::Crime, 6, 6), 200);
    f.h.grids.borrow_mut().insert((Grid::Pollution(0), 6, 6), -3);
    f.h.grids.borrow_mut().insert((Grid::Desirability(0x1020), 6, 6), 55);
    f.h.grids.borrow_mut().insert((Grid::Congestion, 5, 7), 9);
    f.h.grids.borrow_mut().insert((Grid::Congestion, 8, 8), 99);
    f.h.coverage.borrow_mut().insert(true, 40);
    f.h.powered.borrow_mut().insert(HOME, true);
    let (p1, p2) = (0x310u64, 0x311u64);
    f.h.positions.borrow_mut().insert(p1, [100.0, 0.0, 300.0]);
    f.h.positions.borrow_mut().insert(p2, [100.0, 0.0, 150.0]);
    f.h.stations.borrow_mut().insert(true, vec![(p1, 0.5), (p2, 0.8)]);
    f.place(0, HOME);
    for (n, v) in [
        ("state", 1.0),
        ("age", 18.0),
        ("wealth", 2.0),
        ("generation", 1.0),
        ("zodiac", 3.0),
        ("eq", 50.0),
        ("hq", 60.0),
        ("le", 77.0),
        ("home_condition", 1.0),
        ("home_has_road", 1.0),
        ("home_has_power", 1.0),
        ("local_crime", 200.0),
        ("home_police_coverage", 40.0),
        ("local_air_pollution", -3.0),
        ("desirability", 55.0),
        ("home_occupant_wealth", 2.0),
        ("local_police_funding", 80.0),
        ("local_fire_funding", 0.0),
        ("local_traffic_congestion", 9.0),
        ("employed", 0.0),
    ] {
        assert_eq!(f.num(0, n), Some(v), "{n}");
    }
    assert_eq!(f.num(0, "home_has_water"), None, "missing source: not written");
    assert_eq!(f.h.field(f.id(0), "name"), Some(Field::Str(b"Ann".to_vec())));
    assert_eq!(f.h.field(f.id(0), "home_location"), Some(Field::Building(Some(HOME))));
}

#[test]
fn t22_init_tuning_and_table() {
    let f = F::new();
    let s = f.m(0, |m| m.state());
    assert!(s.lua_table);
    assert_eq!(s.local_area_size, 10);
    assert_eq!(s.tuning["MYSIM_BALLOON_SHOWN_DURATION"], 1500.0);
    // Tuning from Lua; a second Init keeps the table.
    f.h.tuning.borrow_mut().insert("MYSIM_TRIP_LOCAL_AREA_SIZE".into(), 7.9);
    let idx = f.adv(0).state().table_index;
    assert!(f.adv(0).init(idx));
    assert_eq!(f.adv(0).id(), 1);
    assert_eq!(f.m(0, |m| m.state().local_area_size), 7);
    assert!(!f.h.take_log().iter().any(|c| matches!(c, MsCall::CreateTable(_))));
}

#[test]
fn t23_command_would_trigger() {
    let f = F::new();
    f.h.take_log();
    assert!(!f.adv(0).command_would_trigger(command::MOVE_OUT));
    assert!(f.h.take_log().is_empty(), "state 0: no Lua context");
    f.at_home(0);
    f.h.take_log();
    assert!(!f.adv(0).command_would_trigger(command::MOVE_OUT));
    assert_eq!(f.h.take_log(), vec![MsCall::Activate(1)]);
}

#[test]
fn t24_delete_from_game() {
    let f = F::new();
    f.at_home(0);
    f.h.take_log();
    assert!(f.m(0, |m| m.delete_from_game()));
    let s = f.m(0, |m| m.state());
    assert_eq!(s.state, 0);
    assert_eq!(f.m(0, |m| m.residence()), None);
    assert_eq!(f.adv(0).personal_name(), b"Ann".to_vec());
    let log = f.h.take_log();
    assert_eq!(log[0], MsCall::Dispatch(1, "shutdown".into()));
    assert_eq!(posts(&log, msg::MYSIMS_CHANGED), 1);
}

#[test]
fn t25_hour_and_view_messages() {
    let f = F::new();
    f.at_home(0);
    f.h.take_log();
    for (h, what) in [(8, "leave_for_work"), (17, "return_home"), (22, "night_sound")] {
        let mut m = message(msg::HOUR_CHANGED);
        m.data2 = h;
        f.agent_msg(m);
        // All five MySims (active or not).
        let log = f.h.take_log();
        assert_eq!(log.len(), 5);
        assert_eq!(log[0], MsCall::Dispatch(1, what.into()));
    }
    let mut m = message(msg::HOUR_CHANGED);
    m.data2 = 9;
    f.agent_msg(m);
    f.agent_msg(message(msg::VIEW_CHANGED_1));
    assert_eq!(f.h.take_log().len(), 5);
}
