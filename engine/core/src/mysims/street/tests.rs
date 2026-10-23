// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! Literal expectations from public §4, before executing the owner. This host
//! answers only individual foreign calls; it never computes a Street outcome.

use super::*;
use crate::stream::{ByteReader, ByteWriter};
use std::collections::VecDeque;

fn t(n: u64) -> Token {
    Token::new(n).unwrap()
}

#[derive(Clone, Debug, PartialEq)]
enum Event {
    W(WordCall),
    H(HandleCall),
    Qi(Token, u32, Option<Token>),
    Init(u32),
    Write,
    Read,
    WriteStatus,
    ReadStatus,
    Location(Token),
    Quantize(Token, f32, f32),
    Real(RealCall),
    Education(bool, Option<Token>),
    Lots(Token, Rect),
    Funding(Token, u32, f32),
    Disaster(i32, Option<Token>),
}

struct Fixture {
    log: Vec<Event>,
    base: u32,
    status: u32,
    table_after_init: Option<u32>,
    table_after_create: Option<u32>,
    qi_al: u32,
    qi_out: Option<Token>,
    group: u32,
    group_al: u32,
    random: u32,
    auto_state: u32,
    control: Option<Token>,
    control_type: u32,
    control_callback: Option<Box<dyn FnMut()>>,
    id: u32,
    data: u32,
    payload: Option<Token>,
    inherited_message: u32,
    ticker: Option<Token>,
    enabled: u32,
    elapsed: u32,
    traffic: bool,
    pollution: bool,
    crime: bool,
    loc: Location,
    lots: Vec<Token>,
    lot_data: Vec<(Token, Option<Token>, i32, i32, u32)>,
    count: i32,
    disasters: VecDeque<(Option<Token>, i32, i32)>,
    disaster_ids: VecDeque<u32>,
    funding: Option<f32>,
    ratio: f64,
    eq: f64,
    hq: f64,
    grid_raw: u32,
    track: u32,
    tool_init: u32,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            log: vec![],
            base: 0,
            status: 0,
            table_after_init: None,
            table_after_create: None,
            qi_al: 0,
            qi_out: None,
            group: 0,
            group_al: 2,
            random: 0,
            auto_state: 7,
            control: None,
            control_type: 0,
            control_callback: None,
            id: 0,
            data: 0,
            payload: None,
            inherited_message: 0xAABB_CC80,
            ticker: Some(t(70)),
            enabled: 1,
            elapsed: 3,
            traffic: false,
            pollution: true,
            crime: true,
            loc: Location {
                retained: Some(t(31)),
                x: 1.5,
                z: -2.25,
                cx: 10,
                cz: -5,
            },
            lots: vec![],
            lot_data: vec![],
            count: -1,
            disasters: VecDeque::new(),
            disaster_ids: VecDeque::new(),
            funding: None,
            ratio: 1.75,
            eq: -2.75,
            hq: 4.5,
            grid_raw: 0x1234_FF80,
            track: 41,
            tool_init: 1,
        }
    }
}

impl Host for Fixture {
    fn word(&mut self, call: WordCall) -> u32 {
        self.log.push(Event::W(call.clone()));
        match call {
            WordCall::IsGroup(o, group) if group == 0x150A => self
                .lot_data
                .iter()
                .find(|(_, j, _, _, _)| *j == Some(o))
                .map_or(0, |row| row.4),
            WordCall::IsGroup(_, group) if group == self.group => self.group_al,
            WordCall::AutomatonState(_) => self.auto_state,
            WordCall::RandomUniform(_) => self.random,
            WordCall::ControlType(_) => self.control_type,
            WordCall::MessageId(_) => self.id,
            WordCall::MessageData(_) => self.data,
            WordCall::InheritedMessage(_) => self.inherited_message,
            WordCall::TickerEnabled(_) => self.enabled,
            WordCall::Elapsed(_) => self.elapsed,
            WordCall::Track(..) => self.track,
            WordCall::LotDistress(p) => self.lot_data.iter().find(|r| r.0 == p).unwrap().2 as u32,
            WordCall::LotWealth(p) => self.lot_data.iter().find(|r| r.0 == p).unwrap().3 as u32,
            WordCall::GridSample(..) => self.grid_raw,
            WordCall::StringType(_, 1) => 0xDEAD_BEEF,
            WordCall::DisasterCount => self.count as u32,
            WordCall::DisasterId(_) => self.disaster_ids.pop_front().unwrap(),
            WordCall::ControlInit(..) => self.tool_init,
            _ => 0,
        }
    }
    fn handle(&mut self, call: HandleCall) -> Option<Token> {
        self.log.push(Event::H(call));
        match call {
            HandleCall::View => Some(t(40)),
            HandleCall::CurrentControl => {
                if let Some(callback) = &mut self.control_callback {
                    callback();
                }
                self.control
            }
            HandleCall::Descriptor(_) => Some(t(30)),
            HandleCall::MessagePayload(_) => self.payload,
            HandleCall::Ticker => self.ticker,
            HandleCall::TrafficGlobal => self.traffic.then_some(t(80)),
            HandleCall::PollutionGlobal => self.pollution.then_some(t(81)),
            HandleCall::CrimeGlobal => self.crime.then_some(t(82)),
            HandleCall::CongestionGrid(_) => Some(t(83)),
            HandleCall::PollutionGrid(_, index) => Some(t(84 + u64::from(index))),
            HandleCall::CrimeGrid(_) => Some(t(87)),
            HandleCall::MayorGrid => Some(t(88)),
            HandleCall::LotManagerGlobal => Some(t(89)),
            HandleCall::Building(p) => self.lot_data.iter().find(|r| r.0 == p).unwrap().1,
            HandleCall::AsOccupant(j) => Some(j),
            HandleCall::Name(_) => Some(t(92)),
            HandleCall::CreateNameHelper(_) => Some(t(93)),
            HandleCall::StringTypeInterface => Some(t(94)),
            HandleCall::PropertyHolder(_) => Some(t(95)),
            HandleCall::Random => Some(t(96)),
            HandleCall::CreateControl => Some(t(97)),
        }
    }
    fn qi(&mut self, object: Token, iid: u32, out: &mut Option<Token>) -> u32 {
        self.log.push(Event::Qi(object, iid, *out));
        *out = if iid == 0x6819_05BD { Some(t(90)) } else { self.qi_out };
        self.qi_al
    }
    fn base_init(&mut self, state: &InheritedState, arg: u32) -> u32 {
        self.log.push(Event::Init(arg));
        if let Some(table) = self.table_after_init {
            let mut s = state.get();
            s.table = table;
            state.set(s);
        }
        self.base
    }
    fn base_write(&mut self, _: &mut dyn OutStream) -> u32 {
        self.log.push(Event::Write);
        self.base
    }
    fn create_lua_table(&mut self, state: &InheritedState) -> u32 {
        self.log.push(Event::W(WordCall::CreateLuaTable));
        if let Some(table) = self.table_after_create {
            let mut s = state.get();
            s.table = table;
            state.set(s);
        }
        0
    }
    fn base_read(&mut self, _: &InheritedState, _: &mut dyn InStream) -> u32 {
        self.log.push(Event::Read);
        self.base
    }
    fn write_status(&mut self, _: &mut dyn OutStream) -> u32 {
        self.log.push(Event::WriteStatus);
        self.status
    }
    fn read_status(&mut self, _: &mut dyn InStream) -> u32 {
        self.log.push(Event::ReadStatus);
        self.status
    }
    fn location(&mut self, occupant: Token) -> Location {
        self.log.push(Event::Location(occupant));
        self.loc
    }
    fn quantize(&mut self, grid: Token, x: f32, z: f32, gx: &mut i32, gz: &mut i32) -> u32 {
        self.log.push(Event::Quantize(grid, x, z));
        *gx = -7;
        *gz = 13;
        0
    }
    fn real(&mut self, call: RealCall) -> f64 {
        self.log.push(Event::Real(call));
        match call {
            RealCall::TimeRatio(..) => self.ratio,
            RealCall::GridSample(g, ..) if g == t(91) => self.hq,
            RealCall::GridSample(..) => self.eq,
        }
    }
    fn education_grid(&mut self, health: bool, out: &mut Option<Token>) -> u32 {
        self.log.push(Event::Education(health, *out));
        *out = Some(t(if health { 91 } else { 98 }));
        0
    }
    fn lots(&mut self, manager: Token, rect: Rect, out: &mut Vec<Token>) -> u32 {
        assert!(out.is_empty());
        self.log.push(Event::Lots(manager, rect));
        *out = self.lots.clone();
        0
    }
    fn funding(&mut self, holder: Token, purpose: u32, funding: &mut f32) -> u32 {
        self.log.push(Event::Funding(holder, purpose, *funding));
        if let Some(f) = self.funding {
            *funding = f;
        }
        0
    }
    fn disaster(&mut self, index: i32, object: &mut Option<Token>, x: &mut i32, z: &mut i32) -> u32 {
        self.log.push(Event::Disaster(index, *object));
        (*object, *x, *z) = self.disasters.pop_front().unwrap();
        0
    }
}

fn seed() -> Inherited {
    Inherited {
        state: 6,
        automaton: None,
        table: 0,
        script: Some(t(20)),
        active: 0,
        extent: 5,
        timer: t(21),
        category: 7,
    }
}
fn make(seed: Inherited, f: &mut Fixture) -> StreetSim {
    StreetSim::new(
        Identities {
            complete: t(1),
            message: t(2),
            street: t(3),
        },
        seed,
        Some(t(10)),
        f,
    )
}
fn words(f: &Fixture) -> Vec<WordCall> {
    f.log
        .iter()
        .filter_map(|e| if let Event::W(w) = e { Some(w.clone()) } else { None })
        .collect()
}
fn numbers(f: &Fixture) -> Vec<(&'static str, i32)> {
    // Observer only: require exact select/push/set triplets, no expected values
    // derived from the owner. Literal expected values occur in assertions below.
    f.log
        .windows(3)
        .filter_map(|w| match (&w[0], &w[1], &w[2]) {
            (
                Event::W(WordCall::Select(_, k)),
                Event::W(WordCall::PushNumber(_, n)),
                Event::W(WordCall::Set(_, -3)),
            ) => Some((*k, *n as i32)),
            _ => None,
        })
        .collect()
}

#[test]
fn l3_l5_init_channels_and_failed_prefix() {
    for (base, table, expected, calls) in [
        (0x100, 0, 0, vec![Event::Init(55)]),
        (2, 0, 1, vec![Event::Init(55), Event::W(WordCall::CreateLuaTable)]),
        (1, 0x1234_5678, 1, vec![Event::Init(55)]),
    ] {
        let mut f = Fixture {
            base,
            ..Fixture::default()
        };
        let mut s = seed();
        s.table = table;
        let owner = make(s, &mut f);
        f.log.clear();
        assert_eq!(owner.init(55, &mut f), expected);
        assert_eq!(f.log, calls);
    }
    let mut f = Fixture {
        base: 0x100,
        table_after_init: Some(0x4444),
        ..Fixture::default()
    };
    let owner = make(seed(), &mut f);
    f.log.clear();
    assert_eq!(owner.init(55, &mut f), 0);
    assert_eq!(owner.state().table, 0x4444);
    assert_eq!(f.log, [Event::Init(55)]);
    f.base = 2;
    f.table_after_init = Some(0);
    f.table_after_create = Some(0x5555);
    f.log.clear();
    assert_eq!(owner.init(55, &mut f), 1);
    assert_eq!(owner.state().table, 0x5555);
    assert_eq!(f.log, [Event::Init(55), Event::W(WordCall::CreateLuaTable)]);
}

#[test]
fn l6_l7_stream_wrapper_ignores_base_and_uses_status_al() {
    let mut f = Fixture {
        base: 0,
        status: 0x100,
        ..Fixture::default()
    };
    let owner = make(seed(), &mut f);
    f.log.clear();
    let mut writer = ByteWriter::new();
    assert_eq!(owner.write(&mut writer, &mut f), 1);
    assert_eq!(f.log, [Event::Write, Event::WriteStatus]);
    f.log.clear();
    f.status = 0x102;
    f.base = 1;
    let mut reader = ByteReader::new(&[]);
    assert_eq!(owner.read(&mut reader, &mut f), 0);
    assert_eq!(f.log, [Event::Read, Event::ReadStatus]);
    f.log.clear();
    f.base = 0;
    f.status = 0;
    assert_eq!(owner.read(&mut reader, &mut f), 1);
    assert_eq!(f.log, [Event::Read, Event::ReadStatus]);
}

#[test]
fn l8_l10_constructor_and_shutdown_are_not_destruction() {
    let mut f = Fixture::default();
    let owner = make(seed(), &mut f);
    assert_eq!(words(&f), [WordCall::AddRef(t(10))]);
    assert_eq!(owner.raw_word(), 0x0BFA_331E);
    assert_eq!(owner.tracking(), 0xFFFF_FFFF);
    assert_eq!(owner.state(), seed());
    assert_eq!(owner.class_id(), 0x0BF6_06A5);
    assert_eq!(
        owner.helper().unwrap().storage,
        [Some(0), None, None, None, None, None, None, None]
    );
    assert_eq!(owner.helper().unwrap().trailing, 0);
    f.log.clear();
    assert_eq!(owner.shutdown(&mut f), 0);
    assert_eq!(words(&f), [WordCall::Shutdown]);
    f.log.clear();
    assert_eq!(owner.destroy(2, &mut f), (t(1), false));
    assert_eq!(words(&f), [WordCall::Release(t(10)), WordCall::InheritedDestroy]);
    f.log.clear();
    let owner = StreetSim::new(
        Identities {
            complete: t(4),
            message: t(5),
            street: t(6),
        },
        seed(),
        None,
        &mut f,
    );
    assert!(owner.helper().is_none());
    assert!(f.log.is_empty());
    assert_eq!(owner.destroy(3, &mut f), (t(4), true));
    assert_eq!(words(&f), [WordCall::InheritedDestroy]);
}

#[test]
fn s1_s2_handle_gates_all_scheduling() {
    let mut f = Fixture::default();
    let mut s = seed();
    s.active = 1;
    let owner = make(s, &mut f);
    f.log.clear();
    owner.day(&mut f);
    owner.day_events(&mut f);
    owner.month(&mut f);
    assert!(f.log.is_empty());
    owner.change(|s| {
        s.automaton = Some(t(11));
        s.active = 0;
        s.state = 0;
    });
    owner.day(&mut f);
    owner.day_events(&mut f);
    owner.month(&mut f);
    assert_eq!(
        words(&f),
        [
            WordCall::ActivateLuaContext,
            WordCall::Balloons(0),
            WordCall::ActivateLuaContext,
            WordCall::DayEvents,
            WordCall::Month
        ]
    );
    assert_eq!(owner.state().active, 0);
}

#[test]
fn d1_d3_and_a1_a2_detach_and_null_attach_prefixes() {
    let mut f = Fixture::default();
    let owner = make(seed(), &mut f);
    f.log.clear();
    for _ in 0..2 {
        assert_eq!(owner.detach(&mut f), Ok(1));
    }
    assert!(f.log.is_empty());
    assert_eq!(owner.state().state, 0);
    assert_eq!(owner.state().category, 7);
    for q in [None, Some(t(12))] {
        owner.change(|s| s.automaton = Some(t(11)));
        owner.tracking.store(41, Ordering::Relaxed);
        f.qi_out = q;
        f.log.clear();
        assert_eq!(owner.attach(None, &mut f), Ok(1));
        let mut expected = vec![
            Event::Qi(t(11), 0x8B99_EE7C, None),
            Event::W(WordCall::RemoveTarget(t(11), t(2))),
            Event::W(WordCall::Release(t(11))),
        ];
        if let Some(q) = q {
            expected.push(Event::W(WordCall::Release(q)));
        }
        expected.extend([Event::H(HandleCall::View), Event::W(WordCall::Untrack(t(40), 41, 0))]);
        assert_eq!(f.log, expected);
        assert_eq!(owner.tracking(), u32::MAX);
    }
    owner.change(|s| s.script = None);
    f.log.clear();
    assert_eq!(owner.attach(Some(t(13)), &mut f), Ok(1));
    assert!(f.log.is_empty());
}

// Literal finite A3–A16 data, independently specified in public §4.1.
type Classification = (u32, u32, u32, i32, i32, &'static [u32], Option<&'static str>);
const CLASSIFICATIONS: &[Classification] = &[
    (
        0,
        0,
        7,
        0,
        9,
        &[0x4001, 0x420B, 0x4200, 0x4201, 0x4215, 0x422C, 0x422D, 0x422E, 0x422F],
        None,
    ),
    (0x4001, 0, 7, 6, 0, &[0x4001], Some("trip_on_foot")),
    (0x4001, 0, 0x107, 6, 0, &[0x4001], Some("trip_on_foot")),
    (0x420B, 0x7FFF_FFFE, 7, 5, 1, &[0x4001, 0x420B], Some("trip_in_car")),
    (0x420B, 0x7FFF_FFFF, 7, 4, 1, &[0x4001, 0x420B], Some("trip_in_car")),
    (0x420B, 0xFFFF_FFFF, 7, 4, 1, &[0x4001, 0x420B], Some("trip_in_car")),
    (0x4200, 0, 7, 5, 2, &[0x4001, 0x420B, 0x4200], Some("trip_on_bus")),
    (
        0x4215,
        0x8000_0000,
        7,
        4,
        3,
        &[0x4001, 0x420B, 0x4200, 0x4201, 0x4215],
        Some("trip_on_train"),
    ),
    (
        0x422D,
        0,
        7,
        5,
        7,
        &[0x4001, 0x420B, 0x4200, 0x4201, 0x4215, 0x422C, 0x422D],
        Some("trip_on_elevated_train"),
    ),
    (
        0x422F,
        0x7FFF_FFFF,
        7,
        4,
        8,
        &[0x4001, 0x420B, 0x4200, 0x4201, 0x4215, 0x422C, 0x422D, 0x422E, 0x422F],
        Some("trip_on_monorail"),
    ),
];

#[test]
fn a3_a4_a6_a14_literal_classification_and_raw_tracking() {
    for &(group, random, auto_state, state, category, groups, key) in CLASSIFICATIONS {
        for q in [None, Some(t(12))] {
            let mut f = Fixture {
                group,
                random,
                auto_state,
                qi_out: q,
                ..Fixture::default()
            };
            let owner = make(seed(), &mut f);
            f.log.clear();
            assert_eq!(owner.attach(Some(t(11)), &mut f), Ok(1));
            assert_eq!(
                (
                    owner.state().state,
                    owner.state().category,
                    owner.state().active,
                    owner.tracking()
                ),
                (state, category, 1, 41)
            );
            let queried: Vec<_> = words(&f)
                .iter()
                .filter_map(|c| {
                    if let WordCall::IsGroup(_, p) = c {
                        Some(*p)
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(queried, groups);
            let mut expected = vec![
                ("wealth", 2),
                ("trip_in_transit", 1),
                ("trip_at_location", 0),
                ("trip_at_transit_switch", 0),
                ("trip_idle", 0),
                ("trip_approaching_location", 0),
                ("trip_on_bus", 0),
                ("trip_on_foot", 0),
                ("trip_on_subway", 0),
                ("trip_in_car", 0),
                ("trip_on_train", 0),
                ("trip_on_elevated_train", 0),
                ("trip_on_monorail", 0),
            ];
            if let Some(key) = key {
                expected.push((key, 1));
            }
            if group == 0x4001 && auto_state == 7 {
                expected.push(("trip_idle", 1));
            }
            expected.push(("state", state));
            assert_eq!(numbers(&f), expected);
            assert!(!words(&f)
                .iter()
                .any(|w| matches!(w, WordCall::ConfigureByte(..) | WordCall::ConfigureFlag(..))));
            assert_eq!(
                words(&f)
                    .iter()
                    .filter(|w| matches!(w, WordCall::RandomUniform(_)))
                    .count(),
                usize::from(category != 0 && category != 9)
            );
            let mut tail = vec![Event::W(WordCall::Track(t(40), t(30), 3, 0x0BFA_331E, 0))];
            if let Some(q) = q {
                tail.push(Event::W(WordCall::Release(q)));
            }
            assert!(f.log.ends_with(&tail));
        }
    }
    let mut f = Fixture {
        track: u32::MAX,
        ..Fixture::default()
    };
    let owner = make(seed(), &mut f);
    owner.attach(Some(t(11)), &mut f).unwrap();
    f.log.clear();
    owner.detach(&mut f).unwrap();
    assert!(!f.log.contains(&Event::H(HandleCall::View)));
}

#[test]
fn a5_true_qi_null_is_unsupported_without_rollback() {
    let mut f = Fixture {
        qi_al: 2,
        ..Fixture::default()
    };
    let owner = make(seed(), &mut f);
    f.log.clear();
    assert_eq!(
        owner.attach(Some(t(11)), &mut f),
        Err(Unsupported::InvalidOutput("true QI null"))
    );
    assert_eq!(owner.state().automaton, Some(t(11)));
    assert_eq!(owner.state().state, 0);
    assert_eq!(
        f.log,
        [Event::W(WordCall::AddRef(t(11))), Event::Qi(t(11), 0x8B99_EE7C, None)]
    );
}

#[test]
fn a15_a16_h4_repeated_same_and_different_show_protects_saved() {
    let mut f = Fixture::default();
    let mut s = seed();
    s.automaton = Some(t(11));
    let owner = make(s, &mut f);
    for (index, incoming) in [t(11), t(11), t(13)].into_iter().enumerate() {
        let old = owner.state().automaton.unwrap();
        f.log.clear();
        owner.attach(Some(incoming), &mut f).unwrap();
        let mut prefix = vec![
            Event::Qi(old, 0x8B99_EE7C, None),
            Event::W(WordCall::RemoveTarget(old, t(2))),
            Event::W(WordCall::Release(old)),
        ];
        if index > 0 {
            prefix.extend([Event::H(HandleCall::View), Event::W(WordCall::Untrack(t(40), 41, 0))]);
        }
        prefix.push(Event::W(WordCall::AddRef(incoming)));
        assert!(f.log.starts_with(&prefix));
        let release = f
            .log
            .iter()
            .position(|e| *e == Event::W(WordCall::Release(old)))
            .unwrap();
        let retain = f
            .log
            .iter()
            .position(|e| *e == Event::W(WordCall::AddRef(incoming)))
            .unwrap();
        assert!(release < retain);
    }
    f.log.clear();
    owner.show_location(&mut f).unwrap();
    assert_eq!(f.log.first(), Some(&Event::W(WordCall::AddRef(t(13)))));
    assert_eq!(f.log.last(), Some(&Event::W(WordCall::Release(t(13)))));
}

#[test]
fn h1_h3_hide_control_full_word_and_restored_inactive_handle() {
    for (control, kind, clear) in [
        (None, 0, false),
        (Some(t(50)), 0xCC23_D564, true),
        (Some(t(50)), 0xCC23_D565, false),
        (Some(t(50)), 0x0000_D564, false),
    ] {
        let mut f = Fixture {
            control,
            control_type: kind,
            ..Fixture::default()
        };
        let mut s = seed();
        s.automaton = Some(t(11));
        s.active = 1;
        let owner = make(s, &mut f);
        f.log.clear();
        owner.hide_location(&mut f).unwrap();
        let mut expected = vec![
            Event::W(WordCall::AddRef(t(11))),
            Event::Qi(t(11), 0x8B99_EE7C, None),
            Event::W(WordCall::RemoveTarget(t(11), t(2))),
            Event::W(WordCall::Release(t(11))),
            Event::H(HandleCall::CurrentControl),
        ];
        if let Some(p) = control {
            expected.extend([Event::W(WordCall::AddRef(p)), Event::W(WordCall::ControlType(p))]);
            if clear {
                expected.push(Event::W(WordCall::ClearControl(0)));
            }
        }
        expected.push(Event::W(WordCall::AddRef(t(11))));
        if let Some(p) = control {
            expected.push(Event::W(WordCall::Release(p)));
        }
        expected.push(Event::W(WordCall::Release(t(11))));
        assert_eq!(f.log, expected);
        assert_eq!(
            (owner.state().automaton, owner.state().state, owner.state().active),
            (Some(t(11)), 0, 0)
        );
    }
}

#[test]
fn hide_rereads_same_different_and_null_current_field() {
    // Controlled field perturbation tests the published reread/comparison,
    // not a claim of general callback reentrancy safety (ST5).
    for current in [None, Some(t(11)), Some(t(13))] {
        let mut f = Fixture::default();
        let mut s = seed();
        s.automaton = Some(t(11));
        let owner = std::sync::Arc::new(make(s, &mut f));
        let target = owner.clone();
        f.control_callback = Some(Box::new(move || {
            target.change(|s| s.automaton = current);
        }));
        f.log.clear();
        owner.hide_location(&mut f).unwrap();
        let start = f
            .log
            .iter()
            .position(|e| *e == Event::H(HandleCall::CurrentControl))
            .unwrap();
        let expected = match current {
            Some(a) if a == t(11) => vec![Event::W(WordCall::Release(t(11)))],
            Some(_) => vec![
                Event::W(WordCall::AddRef(t(11))),
                Event::W(WordCall::Release(t(13))),
                Event::W(WordCall::Release(t(11))),
            ],
            None => vec![Event::W(WordCall::AddRef(t(11))), Event::W(WordCall::Release(t(11)))],
        };
        assert_eq!(&f.log[start + 1..], expected);
        assert_eq!(owner.state().automaton, Some(t(11)));
    }
}

#[test]
fn m1_m3_m9_raw_message_discriminators() {
    for d in [0, 1, 2, 3, 4, 5, 6, 0x100, 0x102, u32::MAX] {
        let mut f = Fixture {
            id: 0x09D9_2FC3,
            data: d,
            ..Fixture::default()
        };
        let mut s = seed();
        s.automaton = Some(t(11));
        let owner = make(s, &mut f);
        f.log.clear();
        assert_eq!(owner.message(t(60), &mut f), Ok(0xAABB_CC80));
        assert_eq!(owner.state().automaton.is_none(), [2, 3, 5].contains(&d));
        assert_eq!(f.log.first(), Some(&Event::W(WordCall::MessageId(t(60)))));
        assert_eq!(f.log.last(), Some(&Event::W(WordCall::InheritedMessage(t(60)))));
        assert_eq!(
            words(&f)
                .iter()
                .filter(|w| matches!(w, WordCall::InheritedMessage(_)))
                .count(),
            1
        );
    }
    for payload in [None, Some(t(1)), Some(t(4)), Some(t(184))] {
        let mut f = Fixture {
            id: 0x4BC4_2C48,
            payload,
            ..Fixture::default()
        };
        let owner = make(seed(), &mut f);
        f.log.clear();
        assert_eq!(owner.message(t(60), &mut f), Ok(0xAABB_CC80));
        assert_eq!(
            f.log,
            [
                Event::W(WordCall::MessageId(t(60))),
                Event::H(HandleCall::MessagePayload(t(60))),
                Event::W(WordCall::InheritedMessage(t(60)))
            ]
        );
    }
    let mut f = Fixture {
        id: 0x1122_3344,
        ..Fixture::default()
    };
    let owner = make(seed(), &mut f);
    f.log.clear();
    assert_eq!(owner.message(t(60), &mut f), Ok(0xAABB_CC80));
    assert_eq!(
        f.log,
        [
            Event::W(WordCall::MessageId(t(60))),
            Event::W(WordCall::InheritedMessage(t(60)))
        ]
    );
}

#[test]
fn m4_m8_timer_strict_unsigned_and_inherited_last() {
    for (a, ticker, enabled, elapsed, updates) in [
        (None, Some(t(70)), 1, 3, false),
        (Some(t(11)), None, 1, 3, false),
        (Some(t(11)), Some(t(70)), 0x100, 3, false),
        (Some(t(11)), Some(t(70)), 2, 2, false),
        (Some(t(11)), Some(t(70)), 2, 3, true),
        (Some(t(11)), Some(t(70)), 2, u32::MAX, true),
    ] {
        let mut f = Fixture {
            id: 0x86AD_10EE,
            ticker,
            enabled,
            elapsed,
            ..Fixture::default()
        };
        let mut s = seed();
        s.automaton = a;
        s.script = None;
        let owner = make(s, &mut f);
        f.log.clear();
        for _ in 0..2 {
            assert_eq!(owner.message(t(60), &mut f), Ok(0xAABB_CC80));
        }
        assert_eq!(
            words(&f).iter().filter(|w| **w == WordCall::Restart(t(21))).count(),
            if updates { 2 } else { 0 }
        );
        let mut one = vec![Event::W(WordCall::MessageId(t(60)))];
        if a.is_some() {
            one.push(Event::H(HandleCall::Ticker));
            if ticker.is_some() {
                one.push(Event::W(WordCall::TickerEnabled(t(70))));
                if enabled as u8 != 0 {
                    one.push(Event::W(WordCall::Elapsed(t(21))));
                }
            }
        }
        if updates {
            one.extend([Event::W(WordCall::Balloons(1)), Event::W(WordCall::Restart(t(21)))]);
        }
        one.push(Event::W(WordCall::InheritedMessage(t(60))));
        let mut both = one.clone();
        both.extend(one);
        assert_eq!(f.log, both);
    }
    let mut f = Fixture {
        id: 0x4BC4_2C48,
        payload: Some(t(3)),
        ..Fixture::default()
    };
    let mut s = seed();
    s.script = None;
    let owner = make(s, &mut f);
    f.log.clear();
    owner.message(t(60), &mut f).unwrap();
    assert_eq!(
        f.log,
        [
            Event::W(WordCall::MessageId(t(60))),
            Event::H(HandleCall::MessagePayload(t(60))),
            Event::W(WordCall::Balloons(1)),
            Event::W(WordCall::InheritedMessage(t(60)))
        ]
    );
}

const EMPTY_TRIP_NUMBERS: &[(&str, i32)] = &[
    ("trip_traffic_congestion", 128),
    ("trip_time_ratio", 1),
    ("trip_wait_length", 0),
    ("trip_pollution_air", -128),
    ("trip_pollution_water", -128),
    ("trip_pollution_garbage", -128),
    ("trip_crime", 128),
    ("trip_mayor_rating", -128),
    ("trip_eq", -2),
    ("trip_hq", 4),
    ("trip_distress1_p", 0),
    ("trip_distress2_p", 0),
    ("trip_distress3_p", 0),
    ("trip_wealth_low_p", 0),
    ("trip_wealth_med_p", 0),
    ("trip_wealth_high_p", 0),
    ("trip_landmark_nearby", 0),
    ("trip_landmark_funding_p", 0),
    ("trip_fire_nearby", 0),
    ("trip_riot_nearby", 0),
    ("trip_any_disaster_nearby", 0),
];

#[test]
fn t1_t6_t9_t12_t16_trip_prefix_integer_widths_and_empty_path() {
    let mut f = Fixture::default();
    let mut s = seed();
    s.script = None;
    let owner = make(s, &mut f);
    f.log.clear();
    assert_eq!(owner.update_trip(&mut f), Ok(TripOutcome::NoScript));
    assert!(f.log.is_empty());
    owner.change(|s| {
        s.script = Some(t(20));
        s.automaton = Some(t(11));
    });
    assert_eq!(owner.update_trip(&mut f), Ok(TripOutcome::MissingGlobal));
    assert_eq!(
        f.log,
        [
            Event::W(WordCall::Context(t(20), 0)),
            Event::H(HandleCall::Descriptor(t(11))),
            Event::Location(t(30)),
            Event::H(HandleCall::TrafficGlobal),
            Event::W(WordCall::Release(t(31)))
        ]
    );
    assert_eq!(Rect::around(10, -5, 5), Rect(8, -7, 12, -3));
    assert_eq!(Rect::around(10, -5, -3), Rect(11, -4, 9, -6));
    f.traffic = true;
    f.log.clear();
    assert_eq!(owner.update_trip(&mut f), Ok(TripOutcome::Complete));
    assert_eq!(numbers(&f), EMPTY_TRIP_NUMBERS);
    assert!(f.log.contains(&Event::Lots(t(90), Rect(8, -7, 12, -3))));
    assert!(f.log.contains(&Event::Qi(t(89), 0x6819_05BD, None)));
    assert!(f.log.contains(&Event::Education(false, None)));
    assert!(f.log.contains(&Event::Education(true, None)));
    assert!(f
        .log
        .contains(&Event::W(WordCall::PushObject(t(20), t(10), 0xDEAD_BEEF))));
    assert!(f.log.ends_with(&[
        Event::W(WordCall::Finish(t(20), 1)),
        Event::W(WordCall::Release(t(90))),
        Event::W(WordCall::Release(t(91))),
        Event::W(WordCall::Release(t(98))),
        Event::W(WordCall::Release(t(31)))
    ]));
    assert!(!f.log.iter().any(|e| matches!(e, Event::Disaster(..))));
}

#[test]
fn t7_reverse_lots_signed_counts_and_percentages() {
    let mut f = Fixture {
        traffic: true,
        lots: vec![t(100), t(101), t(102), t(103)],
        lot_data: vec![
            (t(100), None, 0, 0, 0),
            (t(101), Some(t(111)), 2, 1, 0),
            (t(102), Some(t(112)), 5, 3, 0),
            (t(103), Some(t(113)), 4, 3, 0),
        ],
        ..Fixture::default()
    };
    let mut s = seed();
    s.automaton = Some(t(11));
    let owner = make(s, &mut f);
    f.log.clear();
    assert_eq!(owner.update_trip(&mut f), Ok(TripOutcome::Complete));
    let visits: Vec<_> = f
        .log
        .iter()
        .filter_map(|e| {
            if let Event::H(HandleCall::Building(p)) = e {
                Some(*p)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(visits, [t(103), t(102), t(101), t(100)]);
    assert_eq!(
        &numbers(&f)[10..17],
        [
            ("trip_distress1_p", 33),
            ("trip_distress2_p", 0),
            ("trip_distress3_p", 66),
            ("trip_wealth_low_p", 33),
            ("trip_wealth_med_p", 0),
            ("trip_wealth_high_p", 66),
            ("trip_landmark_nearby", 0)
        ]
    );
}

#[test]
fn t8_t10_t11_first_reversed_landmark_and_independent_budget_output() {
    for (funding, expected) in [(None, 0), (Some(0.25), 25)] {
        let mut f = Fixture {
            traffic: true,
            lots: vec![t(100), t(101)],
            funding,
            lot_data: vec![(t(100), Some(t(110)), -2, -1, 2), (t(101), Some(t(111)), -3, -2, 2)],
            ..Fixture::default()
        };
        let mut s = seed();
        s.automaton = Some(t(11));
        let owner = make(s, &mut f);
        f.log.clear();
        assert_eq!(owner.update_trip(&mut f), Ok(TripOutcome::Complete));
        assert_eq!(
            &numbers(&f)[10..18],
            [
                ("trip_distress1_p", 0),
                ("trip_distress2_p", 0),
                ("trip_distress3_p", 0),
                ("trip_wealth_low_p", 0),
                ("trip_wealth_med_p", 0),
                ("trip_wealth_high_p", 0),
                ("trip_landmark_nearby", 1),
                ("trip_landmark_funding_p", expected)
            ]
        );
        assert!(f.log.contains(&Event::H(HandleCall::Name(t(111)))));
        assert!(!f.log.contains(&Event::H(HandleCall::Name(t(110)))));
        assert!(f.log.contains(&Event::W(WordCall::IsGroup(t(110), 0x150A))));
        assert!(f.log.contains(&Event::Funding(t(95), 0xAA59_670C, 0.0)));
        assert!(f.log.ends_with(&[
            Event::W(WordCall::Finish(t(20), 1)),
            Event::W(WordCall::Release(t(111))),
            Event::W(WordCall::Release(t(90))),
            Event::W(WordCall::Release(t(91))),
            Event::W(WordCall::Release(t(98))),
            Event::W(WordCall::Release(t(31)))
        ]));
    }
}

#[test]
fn t13_t15_disaster_inclusive_edges_repeated_ids_and_release_before_outputs() {
    let cases = [
        (
            vec![(Some(t(120)), 8, -7)],
            vec![0x621C_DA33, 0x621C_DA33],
            vec![0],
            (1, 0, 1),
        ),
        (
            vec![(Some(t(120)), 12, -3), (Some(t(121)), 9, -4), (Some(t(122)), 10, -5)],
            vec![0xCA38_0B06, 0xCA38_0B06, 0x621C_DA33],
            vec![0, 1],
            (1, 1, 1),
        ),
        (vec![(None, 13, -3)], vec![], vec![0], (0, 0, 0)),
    ];
    for (disasters, ids, visits, (fire, riot, any)) in cases {
        let mut f = Fixture {
            traffic: true,
            count: disasters.len() as i32,
            disasters: disasters.into(),
            disaster_ids: ids.into(),
            ..Fixture::default()
        };
        let mut s = seed();
        s.automaton = Some(t(11));
        let owner = make(s, &mut f);
        f.log.clear();
        assert_eq!(owner.update_trip(&mut f), Ok(TripOutcome::Complete));
        let actual: Vec<_> = f
            .log
            .iter()
            .filter_map(|e| {
                if let Event::Disaster(i, None) = e {
                    Some(*i)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(actual, visits);
        assert!(f.disaster_ids.is_empty());
        assert_eq!(
            &numbers(&f)[18..],
            [
                ("trip_fire_nearby", fire),
                ("trip_riot_nearby", riot),
                ("trip_any_disaster_nearby", any)
            ]
        );
        let first_output = f
            .log
            .iter()
            .position(|e| *e == Event::W(WordCall::Select(t(20), "trip_fire_nearby")))
            .unwrap();
        assert!(!f.log[first_output..]
            .iter()
            .any(|e| matches!(e,Event::W(WordCall::Release(d)) if *d==t(120)||*d==t(121))));
    }
}

#[test]
fn t4_reversed_raw_rectangle_is_not_clamped_and_qi_success_configures_individual_methods() {
    let mut f = Fixture {
        traffic: true,
        count: 1,
        disasters: vec![(None, 10, -5)].into(),
        ..Fixture::default()
    };
    let mut s = seed();
    s.automaton = Some(t(11));
    s.extent = -3;
    let owner = make(s, &mut f);
    f.log.clear();
    assert_eq!(owner.update_trip(&mut f), Ok(TripOutcome::Complete));
    assert!(f.log.contains(&Event::Lots(t(90), Rect(11, -4, 9, -6))));
    assert!(!words(&f).iter().any(|w| matches!(w, WordCall::DisasterId(_))));
    assert_eq!(
        &numbers(&f)[18..],
        [
            ("trip_fire_nearby", 0),
            ("trip_riot_nearby", 0),
            ("trip_any_disaster_nearby", 0)
        ]
    );
    f.qi_al = 0x102;
    f.qi_out = Some(t(12));
    f.traffic = false;
    f.log.clear();
    owner.attach(Some(t(13)), &mut f).unwrap();
    assert!(f.log.windows(3).any(|w| w
        == [
            Event::W(WordCall::ConfigureByte(t(12), 255)),
            Event::W(WordCall::ConfigureFlag(t(12), 0x0200_0000, 1)),
            Event::W(WordCall::AddTarget(t(13), t(2)))
        ]));
}

#[test]
fn t17_numeric_and_invalid_output_stay_explicit() {
    for value in [f64::NAN, f64::INFINITY, 1.0e19] {
        let mut f = Fixture {
            traffic: true,
            ratio: value,
            ..Fixture::default()
        };
        let mut s = seed();
        s.automaton = Some(t(11));
        let owner = make(s, &mut f);
        f.log.clear();
        assert_eq!(owner.update_trip(&mut f), Err(Unsupported::NumericDomain));
        assert_eq!(numbers(&f), [("trip_traffic_congestion", 128)]);
    }
    let mut f = Fixture::default();
    let owner = make(seed(), &mut f);
    f.log.clear();
    assert_eq!(
        owner.update_trip(&mut f),
        Err(Unsupported::InvalidOutput("trip automaton"))
    );
    assert_eq!(f.log, [Event::W(WordCall::Context(t(20), 0))]);
    assert_eq!(percentage(3, 1), Ok(33));
    assert_eq!(percentage(3, 2), Ok(66));
    // PC53 rounds the final product to exactly 100 here (ST3).
    assert_eq!(percentage(4096, 4096), Ok(100));
    assert_eq!(percentage(1, 1), Ok(99));
    assert_eq!(native_trunc(-2.75), Ok(-2));
    assert_eq!(native_trunc(2147483648.0), Ok(i32::MIN));
    assert_eq!(native_trunc(4294967298.5), Ok(2));
}

#[test]
fn u1_u2_tool_init_al_and_ignored_install_reply() {
    for (init, expected) in [(0x100, 0), (2, 1)] {
        let mut f = Fixture {
            tool_init: init,
            ..Fixture::default()
        };
        let owner = make(seed(), &mut f);
        f.log.clear();
        assert_eq!(owner.create_tool(&mut f), Ok(expected));
        let mut expected_calls = vec![
            Event::H(HandleCall::CreateControl),
            Event::W(WordCall::AddRef(t(97))),
            Event::W(WordCall::ControlInit(t(97), t(1))),
        ];
        if expected == 1 {
            expected_calls.push(Event::W(WordCall::InstallControl(t(97), 2)));
        }
        expected_calls.push(Event::W(WordCall::Release(t(97))));
        assert_eq!(f.log, expected_calls);
    }
}
