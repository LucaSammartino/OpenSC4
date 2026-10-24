// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! effects.md §17 (the ten conformance tests) and the rest of the spec's
//! rules.

use std::rc::Rc;

use super::directory::{ComponentRecord, EffectDescription, EffectDirectory};
use super::shipped::{self, BRUSHES, CRATER_BRUSH, VOLCANO_CREATORS};
use super::*;
use crate::rng::{FixedSeeds, RzRandomSuper};
use crate::terrain::BrushParams;

#[derive(Clone, Debug, PartialEq)]
enum Ev {
    Brush {
        id: u32,
        from: (f32, f32),
        to: (f32, f32),
        level: i32,
        strength: f32,
        radius: f32,
        redisplay: bool,
        data4: u32,
        target: f32,
    },
    BeginLong,
    EndLong,
    Ignite(i32, i32, i32, i32),
    Cause(u32),
    Toxic(i32, i32),
    Demolish(EffectRect, u32, bool),
    Extinguish(i32, i32, i32),
    Post(EffectPost),
    Pause,
    Resume,
    HiddenPause,
    HiddenResume,
    Clock(bool),
    Map(u32, (i32, i32), u64),
    RemoveMap(u64),
}

struct Host {
    dilation: Option<f32>,
    cells: (i32, i32),
    events: Vec<Ev>,
}

impl Host {
    fn new() -> Host {
        Host {
            dilation: Some(1.0),
            cells: (64, 64),
            events: Vec::new(),
        }
    }
    fn brush_calls(&self) -> usize {
        self.events.iter().filter(|e| matches!(e, Ev::Brush { .. })).count()
    }
    fn posts(&self) -> Vec<EffectPost> {
        self.events
            .iter()
            .filter_map(|e| match e {
                Ev::Post(p) => Some(*p),
                _ => None,
            })
            .collect()
    }
}

impl EffectsHost for Host {
    fn simulator(&mut self) -> Option<f32> {
        self.dilation
    }
    fn sim_pause(&mut self) {
        self.events.push(Ev::Pause);
    }
    fn sim_resume(&mut self) {
        self.events.push(Ev::Resume);
    }
    fn sim_hidden_pause(&mut self) {
        self.events.push(Ev::HiddenPause);
    }
    fn sim_hidden_resume(&mut self) {
        self.events.push(Ev::HiddenResume);
    }
    fn clock_pause(&mut self, on: bool) {
        self.events.push(Ev::Clock(on));
    }
    fn city_cells(&mut self) -> (i32, i32) {
        self.cells
    }
    fn location_in_bounds(&mut self, x: f32, z: f32) -> bool {
        x >= 0.0 && z >= 0.0 && x < self.cells.0 as f32 * 16.0 && z < self.cells.1 as f32 * 16.0
    }
    fn apply_brush(&mut self, brush_id: u32, p: &mut BrushParams) -> bool {
        self.events.push(Ev::Brush {
            id: brush_id,
            from: (p.from_x, p.from_z),
            to: (p.to_x, p.to_z),
            level: p.level,
            strength: p.strength_scale,
            radius: p.radius_scale,
            redisplay: p.redisplay,
            data4: p.data4,
            target: p.target_height,
        });
        true
    }
    fn begin_long_terrain_operation(&mut self) {
        self.events.push(Ev::BeginLong);
    }
    fn end_long_terrain_operation(&mut self) {
        self.events.push(Ev::EndLong);
    }
    fn ignite_cell_radius(&mut self, x: i32, z: i32, r0: i32, r1: i32) {
        self.events.push(Ev::Ignite(x, z, r0, r1));
    }
    fn set_fire_cause(&mut self, c: u32) {
        self.events.push(Ev::Cause(c));
    }
    fn toxic_attempt_start(&mut self, x: i32, z: i32) {
        self.events.push(Ev::Toxic(x, z));
    }
    fn demolish_region(&mut self, rect: EffectRect, flags: u32, dezone: bool, _f: &DemolishFilter) {
        self.events.push(Ev::Demolish(rect, flags, dezone));
    }
    fn extinguish_cell(&mut self, x: i32, z: i32, a: i32) {
        self.events.push(Ev::Extinguish(x, z, a));
    }
    fn post(&mut self, m: EffectPost) {
        self.events.push(Ev::Post(m));
    }
    fn apply_effect_map(&mut self, record: &GameRecord, cell: (i32, i32)) -> u64 {
        let token = self.events.len() as u64 + 100;
        self.events.push(Ev::Map(record.map_kind, cell, token));
        token
    }
    fn remove_effect_map(&mut self, token: u64) {
        self.events.push(Ev::RemoveMap(token));
    }
}

#[path = "owner_tests.rs"]
mod owner_closure;

/// A directory of one effect `name` with the given components (shipped
/// brush and game records).
fn one_effect(name: &str, comps: Vec<ComponentRecord>) -> Rc<EffectDirectory> {
    Rc::new(EffectDirectory::from_parts(
        BRUSHES.to_vec(),
        shipped::game_records(),
        vec![EffectDescription::new(1, comps)],
        vec![(name.to_owned(), 0)],
    ))
}

fn manager(dir: Rc<EffectDirectory>) -> EffectsManager {
    let mut m = EffectsManager::new(0x1234_5678);
    m.set_directory(dir);
    m
}

/// Creates `name`, places it at world `(x, z)` and starts it.
fn started(m: &mut EffectsManager, h: &mut Host, name: &str, x: f32, z: f32) -> EffectId {
    let id = m.create(name).expect("created");
    assert!(m.set_position(id, [x, 0.0, z]));
    assert!(m.start(id, 0, h));
    id
}

fn brush_of(m: &EffectsManager, id: EffectId) -> BrushEffect {
    match m.component_of_kind(id, component_type::BRUSH, 0) {
        Some(Component::Brush(b)) => b.clone(),
        other => panic!("no brush: {other:?}"),
    }
}

fn game_of(m: &EffectsManager, id: EffectId) -> GameEffect {
    match m.component_of_kind(id, component_type::GAME, 0) {
        Some(Component::Game(g)) => g.clone(),
        other => panic!("no game: {other:?}"),
    }
}

#[test]
fn legacy_component_offsets_survive_creation_and_parent_moves() {
    use super::transform::{ParentTransform, Placement};
    let placement = Placement {
        flags: 1,
        transform: ParentTransform {
            translation: [17., 9., 33.],
            ..ParentTransform::default()
        },
        ..Placement::default()
    }
    .encode();
    let mut brush = ComponentRecord::new(component_type::BRUSH, 0);
    let mut game = ComponentRecord::new(component_type::GAME, 0);
    brush.placement = placement;
    game.placement = placement;
    let mut m = manager(one_effect("offset", vec![brush, game]));
    let id = m.create("offset").unwrap();
    assert_eq!(brush_of(&m, id).cur, (17., 33.));
    assert_eq!(game_of(&m, id).position, [17., 9., 33.]);
    m.set_position(id, [32., 10., 64.]);
    assert_eq!(brush_of(&m, id).cur, (49., 97.));
    assert_eq!(game_of(&m, id).position, [49., 19., 97.]);
    assert_eq!(game_of(&m, id).cell, (3, 6));
}

/// Ticks until the brush is inactive: (ticks, per-tick application counts).
fn run_stroke(record: u32, us: u32, dilation: f32) -> (usize, Vec<usize>, BrushEffect) {
    let mut m = manager(one_effect(
        "stroke",
        vec![ComponentRecord::new(component_type::BRUSH, record)],
    ));
    let mut h = Host::new();
    h.dilation = Some(dilation);
    let id = started(&mut m, &mut h, "stroke", 100.0, 100.0);
    let mut per_tick = Vec::new();
    for tick in 1..10_000 {
        let before = h.brush_calls();
        m.tick(us, &mut h);
        per_tick.push(h.brush_calls() - before);
        if !brush_of(&m, id).active {
            return (tick, per_tick, brush_of(&m, id));
        }
    }
    panic!("stroke never ended");
}

// ---- §17 test 1, 2: the brush stroke --------------------------------------------------------------

#[test]
fn t01_brush_stroke_count() {
    let (ticks, n, b) = run_stroke(2, 33_333, 1.0);
    assert_eq!(&n[..4], &[0, 1, 1, 1]);
    assert_eq!(ticks, 91);
    assert_eq!(n.iter().sum::<usize>(), 90);
    assert!((b.t - 3.0333).abs() < 1e-4, "t = {}", b.t);
    assert!((b.acc - 0.99909).abs() < 1e-4, "acc = {}", b.acc);
    assert_eq!(b.applications, 90);
}

#[test]
fn t02_dilation() {
    let (ticks, n, _) = run_stroke(2, 33_333, 1.5);
    assert_eq!(&n[..5], &[1, 1, 2, 1, 2]);
    assert_eq!(ticks, 61);
    assert_eq!(n.iter().sum::<usize>(), 91);
    let (ticks, n, _) = run_stroke(2, 100_000, 1.0);
    assert!(n.iter().all(|&k| k == 3));
    assert_eq!(ticks, 31);
    assert_eq!(n.iter().sum::<usize>(), 93);
}

#[test]
fn tick_dilation_and_total() {
    let mut m = manager(shipped::volcano_directory().into());
    let mut h = Host::new();
    h.dilation = Some(1.5);
    m.tick(33_333, &mut h);
    assert_eq!(m.total_time().0, 49_999);
    h.dilation = None;
    m.tick(10, &mut h);
    assert_eq!(m.total_time().0, 50_009);
}

// ---- §17 test 3: the select draw ------------------------------------------------------------------

#[test]
fn t03_select_draw() {
    let mut g = RzRandomSuper::new(
        SELECT_SEED,
        &FixedSeeds {
            wall_clock: 0,
            process_clock: 0,
        },
    );
    assert_eq!(g.next_u32(), 0xAAE6_4AC3);
    let mut g = RzRandomSuper::new(
        SELECT_SEED,
        &FixedSeeds {
            wall_clock: 0,
            process_clock: 0,
        },
    );
    let r: Vec<u32> = (0..6).map(|_| g.uniform_u32(SELECT_RANGE)).collect();
    assert_eq!(r, vec![43750, 24188, 47502, 45072, 37594, 53135]);
    let weights: Vec<u16> = VOLCANO_CREATORS.iter().map(|c| c.weight).collect();
    let picks: Vec<usize> = r
        .iter()
        .map(|&r| shipped::select_member(&weights, r).unwrap() + 1)
        .collect();
    assert_eq!(picks, vec![7, 4, 7, 7, 6, 8]);

    // A fresh manager's first volcano_control builds volcanocreator4a.
    let mut m = manager(shipped::volcano_directory().into());
    let id = m.create(shipped::VOLCANO_CONTROL).unwrap();
    let metas: Vec<EffectId> = m
        .components(id)
        .unwrap()
        .iter()
        .flatten()
        .filter_map(|c| match c {
            Component::Meta(child) => Some(*child),
            _ => None,
        })
        .collect();
    assert_eq!(metas.len(), 1);
    let child = m.effect(metas[0]).unwrap();
    assert_eq!(child.name, "volcanocreator4a");
    let b = brush_of(&m, metas[0]);
    assert_eq!((b.record.rate, b.record.length, b.radius), (65.0, 3.0, 1.0));
    assert_eq!(m.stats().select_draws, 1);
    // The next ones follow the draws above.
    let names: Vec<String> = (0..5)
        .map(|_| {
            let id = m.create(shipped::VOLCANO_CONTROL).unwrap();
            let child = m
                .components(id)
                .unwrap()
                .iter()
                .flatten()
                .find_map(|c| match c {
                    Component::Meta(c) => Some(*c),
                    _ => None,
                })
                .unwrap();
            m.effect(child).unwrap().name
        })
        .collect();
    assert_eq!(
        names,
        [
            "volcanocreator2b",
            "volcanocreator4a",
            "volcanocreator4a",
            "volcanocreator3b",
            "volcanocreator4b"
        ]
    );
}

// ---- §17 test 4: the one-shot crater --------------------------------------------------------------

#[test]
fn t04_one_shot_crater() {
    let mut m = manager(shipped::volcano_directory().into());
    let mut h = Host::new();
    let id = started(&mut m, &mut h, "meteorcrater_b", 320.0, 480.0);
    h.events.clear();
    m.tick(33_333, &mut h);
    let call = |redisplay| Ev::Brush {
        id: CRATER_BRUSH,
        from: (320.0, 480.0),
        to: (320.0, 480.0),
        level: 3,
        strength: 6.0,
        radius: 0.7,
        redisplay,
        data4: 3,
        target: 100.0,
    };
    assert_eq!(h.events, vec![call(false), call(true)]);
    let b = brush_of(&m, id);
    assert!(!b.active && !b.long_op);
    h.events.clear();
    m.tick(33_333, &mut h);
    assert!(h.events.is_empty());
}

// ---- §17 test 5: out of bounds --------------------------------------------------------------------

#[test]
fn t05_out_of_bounds() {
    let mut m = manager(one_effect(
        "stroke",
        vec![ComponentRecord::new(component_type::BRUSH, 10)],
    ));
    let mut h = Host::new();
    // prev outside the city (64 × 64 cells = 1024 m).
    let id = started(&mut m, &mut h, "stroke", -50.0, 100.0);
    m.set_position(id, [100.0, 0.0, 100.0]);
    for _ in 0..3 {
        m.tick(100_000, &mut h);
    }
    assert_eq!(h.brush_calls(), 0);
    let b = brush_of(&m, id);
    assert_eq!(b.prev, (-50.0, 100.0));
    assert!(b.acc < 1.0, "acc still drains: {}", b.acc);
    // Only a Start (prev := cur) brings prev back inside.
    m.start(id, 0, &mut h);
    m.tick(100_000, &mut h);
    assert_eq!(h.brush_calls(), 13);
    assert_eq!(brush_of(&m, id).prev, (100.0, 100.0));
}

// ---- §17 test 6: the game record's order ----------------------------------------------------------

#[test]
fn t06_game_record_order() {
    let mut m = manager(one_effect("g17", vec![ComponentRecord::new(component_type::GAME, 17)]));
    let mut h = Host::new();
    started(&mut m, &mut h, "g17", 40.0 * 16.0 + 8.0, 40.0 * 16.0 + 8.0);
    h.events.clear();
    m.tick(33_333, &mut h);
    assert_eq!(
        h.events,
        vec![
            Ev::Ignite(40, 40, 0, 14),
            Ev::Cause(4),
            Ev::Demolish(
                EffectRect {
                    x0: 30,
                    z0: 30,
                    x1: 50,
                    z1: 50
                },
                2,
                false
            ),
        ]
    );
    let mut m = manager(one_effect("g17", vec![ComponentRecord::new(component_type::GAME, 17)]));
    started(&mut m, &mut h, "g17", 2.0 * 16.0, 60.0 * 16.0);
    h.events.clear();
    m.tick(33_333, &mut h);
    assert_eq!(
        h.events[2],
        Ev::Demolish(
            EffectRect {
                x0: 0,
                z0: 50,
                x1: 12,
                z1: 63
            },
            2,
            false
        )
    );
}

#[test]
fn game_actions_in_order() {
    // burn, toxic, demolish, extinguish, message: a record with all of them.
    let mut g = GameRecord {
        burn_radius: 2,
        toxic: 20,
        demolish_radius: 1,
        demolish_flags: 0x1302,
        extinguish: 7,
        message_type: msg::EFFECT_LOCATION,
        message_data: 5,
        flags: game_flag::DEZONE,
        ..GameRecord::default()
    };
    g.version = 1;
    let dir = EffectDirectory::from_parts(
        BRUSHES.to_vec(),
        vec![g],
        vec![EffectDescription::new(
            1,
            vec![ComponentRecord::new(component_type::GAME, 0)],
        )],
        vec![("all".into(), 0)],
    );
    let mut m = manager(dir.into());
    let mut h = Host::new();
    let id = started(&mut m, &mut h, "all", 16.0 * 3.5, 16.0 * 4.5);
    assert!(m.forward_parameter(id, 13, 0xABCD));
    h.events.clear();
    m.tick(1000, &mut h);
    assert_eq!(
        h.events,
        vec![
            Ev::Ignite(3, 4, 0, 2),
            Ev::Cause(4),
            Ev::Toxic(3, 4),
            Ev::Demolish(
                EffectRect {
                    x0: 2,
                    z0: 3,
                    x1: 4,
                    z1: 5
                },
                0x1302,
                true
            ),
            Ev::Extinguish(3, 4, 7),
            Ev::Post(EffectPost {
                msg_type: msg::EFFECT_LOCATION,
                data1: 0xABCD,
                data2: 3,
                data3: 4
            }),
        ]
    );
}

// ---- §17 test 7: moving to a new cell -------------------------------------------------------------

#[test]
fn t07_move_to_new_cell() {
    let mut m = manager(one_effect("walk", vec![ComponentRecord::new(component_type::GAME, 18)]));
    let mut h = Host::new();
    let id = started(&mut m, &mut h, "walk", 100.0, 100.0);
    m.tick(1000, &mut h);
    assert_eq!(game_of(&m, id).count, 1);
    m.set_position(id, [110.0, 0.0, 105.0]); // same cell (6, 6)
    m.tick(1000, &mut h);
    assert_eq!(game_of(&m, id).count, 1);
    m.set_position(id, [113.0, 0.0, 100.0]); // cell (7, 6)
    m.tick(1000, &mut h);
    assert_eq!(game_of(&m, id).count, 2);
    assert_eq!(h.events.iter().filter(|e| matches!(e, Ev::Demolish(..))).count(), 2);
}

// ---- §17 test 8: the pause record -----------------------------------------------------------------

#[test]
fn t08_pause_record() {
    let mut m = manager(one_effect("pause", vec![ComponentRecord::new(component_type::GAME, 3)]));
    let mut h = Host::new();
    let id = started(&mut m, &mut h, "pause", 100.0, 100.0);
    assert_eq!(h.events, vec![Ev::HiddenPause]);
    h.events.clear();
    for k in 1..=5 {
        m.tick(1_000_000, &mut h);
        assert_eq!(game_of(&m, id).timer, k as f32);
        assert!(h.events.is_empty(), "tick {k}: {:?}", h.events);
    }
    m.tick(1_000_000, &mut h);
    assert_eq!(h.events, vec![Ev::HiddenResume]);
    assert!(!game_of(&m, id).started);
    h.events.clear();
    m.stop(id, 0, &mut h);
    assert!(h.events.is_empty());
}

#[test]
fn game_start_needs_a_simulator() {
    let mut m = manager(one_effect("pause", vec![ComponentRecord::new(component_type::GAME, 3)]));
    let mut h = Host::new();
    h.dilation = None;
    let id = started(&mut m, &mut h, "pause", 100.0, 100.0);
    assert!(h.events.is_empty());
    assert!(!game_of(&m, id).started);
}

#[test]
fn single_record_applies_once_and_keeps_pauses() {
    let mut g = shipped::game_record(1).unwrap();
    g.flags |= game_flag::PAUSE_SIM;
    let dir = EffectDirectory::from_parts(
        BRUSHES.to_vec(),
        vec![g],
        vec![EffectDescription::new(
            1,
            vec![ComponentRecord::new(component_type::GAME, 0)],
        )],
        vec![("s".into(), 0)],
    );
    let mut m = manager(dir.into());
    let mut h = Host::new();
    let id = started(&mut m, &mut h, "s", 100.0, 100.0);
    m.tick(1000, &mut h);
    m.tick(1000, &mut h);
    assert_eq!(h.events.iter().filter(|e| matches!(e, Ev::Demolish(..))).count(), 1);
    assert!(!game_of(&m, id).started);
    m.stop(id, 1, &mut h);
    // [quirk] The pause is not undone.
    assert!(!h.events.contains(&Ev::Resume));
}

// ---- §17 test 9: the priority filter --------------------------------------------------------------

#[test]
fn t09_priority_filter() {
    let dir = EffectDirectory::from_parts(
        BRUSHES.to_vec(),
        shipped::game_records(),
        vec![
            EffectDescription::new(1, vec![ComponentRecord::new(component_type::BRUSH, 0)]),
            EffectDescription::new(3, vec![ComponentRecord::new(component_type::BRUSH, 1)]),
        ],
        vec![("x".into(), 0), ("X".into(), 1)],
    );
    let mut m = manager(dir.into());
    m.set_max_priority(2);
    let id = m.create("x").unwrap();
    assert_eq!(m.effect(id).unwrap().index, 0);
    m.set_max_priority(5);
    let id = m.create("X").unwrap();
    assert_eq!(m.effect(id).unwrap().index, 1);
    assert!(m.create("y").is_none());
    m.set_max_priority(0);
    let mut h = Host::new();
    let id = m.create("x").unwrap();
    assert_eq!(m.effect(id).unwrap().index, -1);
    m.set_end_message(id, 0x1234, 7);
    m.start(id, 0, &mut h);
    for _ in 0..5 {
        m.tick(100_000, &mut h);
        m.finish_hook(&mut h);
    }
    assert!(h.posts().is_empty());
    assert!(m.effect(id).is_some(), "held: it stays");
}

#[test]
fn priority_ties_take_the_first_in_the_name_map() {
    let dir = EffectDirectory::from_parts(
        Vec::new(),
        Vec::new(),
        vec![EffectDescription::new(3, vec![]), EffectDescription::new(3, vec![])],
        vec![("t".into(), 1), ("t".into(), 0)],
    );
    assert_eq!(dir.find("t", 5), Some(1));
    assert_eq!(dir.find("T", 2), Some(-1));
}

// ---- §17 test 10: no end without the draw pass ----------------------------------------------------

fn volcano(mode: FinishMode) -> (EffectsManager, Host, EffectId) {
    let mut m = manager(shipped::volcano_directory().into());
    m.set_finish_mode(mode);
    let mut h = Host::new();
    let id = m.create(shipped::VOLCANO_CONTROL).unwrap();
    m.set_end_message(id, shipped::VOLCANO_END_MESSAGE, 0x77);
    m.set_position(id, [500.0, 0.0, 500.0]);
    m.start(id, 0, &mut h);
    (m, h, id)
}

#[test]
fn t10_no_end_without_the_draw_pass() {
    for mode in [FinishMode::DrawPass, FinishMode::Hook] {
        let (mut m, mut h, id) = volcano(mode);
        for _ in 0..200 {
            m.tick(33_333, &mut h);
        }
        assert!(h.brush_calls() > 0);
        assert!(h.posts().is_empty(), "{mode:?}");
        assert!(m.is_started(id));
        // In the compat mode the hook does nothing either.
        if mode == FinishMode::DrawPass {
            assert!(!m.finish_hook(&mut h));
            assert!(h.posts().is_empty());
        }
    }
}

#[test]
fn volcano_finishes_in_the_draw_pass_or_the_hook() {
    for mode in [FinishMode::DrawPass, FinishMode::Hook] {
        let (mut m, mut h, id) = volcano(mode);
        let mut ticks = 0;
        loop {
            m.tick(33_333, &mut h);
            ticks += 1;
            if mode == FinishMode::DrawPass {
                assert!(!m.finish_hook(&mut h));
                m.draw_pass(&mut h);
            } else {
                assert!(!m.draw_pass(&mut h));
                m.finish_hook(&mut h);
            }
            if !h.posts().is_empty() || ticks > 1000 {
                break;
            }
        }
        assert_eq!(
            h.posts(),
            vec![EffectPost {
                msg_type: shipped::VOLCANO_END_MESSAGE,
                data1: 0x77,
                data2: 0,
                data3: 0
            }]
        );
        assert!(!m.is_started(id));
        // The volcano holds its reference: the effect stays, and never posts
        // again.
        for _ in 0..3 {
            m.tick(33_333, &mut h);
            m.draw_pass(&mut h);
            m.finish_hook(&mut h);
        }
        assert_eq!(h.posts().len(), 1);
        assert!(m.effect(id).is_some());
        // The long terrain operation was begun once, and ended twice: when
        // the creator finished, and again when volcano_control's Stop(1)
        // reached it through the meta component (`longOp` stays set, EF8).
        assert_eq!(h.events.iter().filter(|e| **e == Ev::BeginLong).count(), 1);
        assert_eq!(h.events.iter().filter(|e| **e == Ev::EndLong).count(), 2);
        // Released: recycled with its child at the next sweep.
        m.release(id);
        m.draw_pass(&mut h);
        m.finish_hook(&mut h);
        assert_eq!(m.effect_count(), 0);
        // volcanocreator4a: rate 65 for 3 s, its child updated twice per
        // tick (EF9) so the stroke takes half the ticks.
        let applied = h.brush_calls();
        assert!((190..=200).contains(&applied), "{applied}");
        assert!(ticks < 60, "{ticks}");
    }
}

#[test]
fn nested_update_off_updates_children_once() {
    let mut m = manager(shipped::volcano_directory().into());
    m.set_nested_update(false);
    let mut h = Host::new();
    let id = m.create(shipped::VOLCANO_CONTROL).unwrap();
    m.set_position(id, [500.0, 0.0, 500.0]);
    m.start(id, 0, &mut h);
    let mut ticks = 0;
    while h.posts().is_empty() && ticks < 1000 {
        m.set_end_message(id, 1, 0);
        m.tick(33_333, &mut h);
        m.finish_hook(&mut h);
        ticks += 1;
    }
    assert!((90..=92).contains(&ticks), "{ticks}");
}

#[test]
fn volcano_game_record_applies_once_at_the_vent() {
    let (mut m, mut h, _) = volcano(FinishMode::Hook);
    for _ in 0..10 {
        m.tick(33_333, &mut h);
    }
    let c = (500 / 16, 500 / 16);
    let fires: Vec<&Ev> = h.events.iter().filter(|e| matches!(e, Ev::Ignite(..))).collect();
    assert_eq!(fires, vec![&Ev::Ignite(c.0, c.1, 0, 12)]);
    let dem: Vec<&Ev> = h.events.iter().filter(|e| matches!(e, Ev::Demolish(..))).collect();
    assert_eq!(
        dem,
        vec![&Ev::Demolish(
            EffectRect {
                x0: c.0 - 8,
                z0: c.1 - 8,
                x1: c.0 + 8,
                z1: c.1 + 8
            },
            2,
            false
        )]
    );
}

// ---- the rest of §4–§7 ------------------------------------------------------------------------------

#[test]
fn creation_draws_even_without_start() {
    let mut m = manager(shipped::volcano_directory().into());
    let before = m.brush_rng_mut().mul_state();
    let id = m.create("meteorcrater_a").unwrap();
    assert_ne!(m.brush_rng_mut().mul_state(), before, "two draws made");
    // Released without a start: done at once, recycled by the next sweep.
    m.release(id);
    let mut h = Host::new();
    m.finish_hook(&mut h);
    assert_eq!(m.effect_count(), 0);
    assert!(h.events.is_empty());
}

#[test]
fn brush_draws_strength_then_radius() {
    let mut rng = crate::rng::RzRandom::new(
        99,
        &FixedSeeds {
            wall_clock: 0,
            process_clock: 0,
        },
    );
    let mut copy = rng.clone();
    let rec = BrushRecord {
        strength: (1.0, 3.0),
        radius: (0.5, 0.75),
        ..BRUSHES[2]
    };
    let b = BrushEffect::new(&rec, &mut rng);
    let s = copy.range_f64(1.0, 3.0) as f32;
    let r = copy.range_f64(0.5, 0.75) as f32;
    assert_eq!((b.strength, b.radius), (s, r));
}

#[test]
fn finish_test_rules() {
    // A brush with flag 0 holds the effect; a game component never does
    // (flag 1); flag bit 1 holds it forever; flag bit 7 needs a soft stop.
    let mut game = ComponentRecord::new(component_type::GAME, 3);
    game.flags = 1;
    let mut dir = EffectDirectory::from_parts(
        BRUSHES.to_vec(),
        shipped::game_records(),
        vec![
            EffectDescription::new(1, vec![ComponentRecord::new(component_type::BRUSH, 1)]),
            EffectDescription::new(1, vec![game.clone()]),
            EffectDescription {
                flags: effect_flag::NO_AUTO_STOP,
                ..EffectDescription::new(1, vec![])
            },
            EffectDescription {
                flags: effect_flag::FINISH_AFTER_SOFT_STOP,
                ..EffectDescription::new(1, vec![])
            },
        ],
        vec![
            ("stroke".into(), 0),
            ("game".into(), 1),
            ("hold".into(), 2),
            ("soft".into(), 3),
        ],
    );
    dir.reindex();
    let mut m = manager(dir.into());
    let mut h = Host::new();
    let ids: Vec<EffectId> = ["stroke", "game", "hold", "soft"]
        .iter()
        .enumerate()
        .map(|(k, n)| {
            let id = started(&mut m, &mut h, n, 100.0, 100.0);
            m.set_end_message(id, 0x100 + k as u32, k as u64);
            id
        })
        .collect();
    m.finish_hook(&mut h);
    // The game effect is still running (pause record) but carries flag 1.
    assert_eq!(h.posts().iter().map(|p| p.msg_type).collect::<Vec<_>>(), vec![0x101]);
    assert!(h.events.contains(&Ev::HiddenResume), "Stop(1) undid the pause");
    // The stroke (rate 2, 0.5 s) ends after its length.
    for _ in 0..6 {
        m.tick(100_000, &mut h);
    }
    m.finish_hook(&mut h);
    assert_eq!(
        h.posts().iter().map(|p| p.msg_type).collect::<Vec<_>>(),
        vec![0x101, 0x100]
    );
    // Soft stop: the bit-7 effect finishes on the next sweep.
    m.stop(ids[3], 0, &mut h);
    assert!(m.is_stopping(ids[3]) && m.is_started(ids[3]));
    m.finish_hook(&mut h);
    assert_eq!(h.posts().last().map(|p| p.msg_type), Some(0x103));
    assert!(m.is_started(ids[2]), "no auto stop");
}

#[test]
fn hard_stop_flag_turns_a_soft_stop_hard() {
    let dir = EffectDirectory::from_parts(
        BRUSHES.to_vec(),
        Vec::new(),
        vec![EffectDescription {
            flags: effect_flag::HARD_STOP,
            ..EffectDescription::new(1, vec![])
        }],
        vec![("h".into(), 0)],
    );
    let mut m = manager(dir.into());
    let mut h = Host::new();
    let id = started(&mut m, &mut h, "h", 0.0, 0.0);
    m.stop(id, 0, &mut h);
    assert!(!m.is_started(id) && !m.is_stopping(id));
}

#[test]
fn lod_ranges_and_zoom() {
    let mut far = ComponentRecord::new(component_type::BRUSH, 0);
    far.lod_min = 4;
    far.lod_max = 6;
    let dir = one_effect("z", vec![far, ComponentRecord::new(component_type::GAME, 0)]);
    let mut m = manager(dir);
    let mut h = Host::new();
    let id = started(&mut m, &mut h, "z", 100.0, 100.0);
    assert_eq!(m.effect(id).unwrap().components, 1);
    // max(zoom − offset + 1, 1) (§2; EF10: zoom 5 would give 6).
    m.set_zoom(4, 0, &mut h);
    assert_eq!(m.level_of_detail(), 5);
    assert_eq!(m.effect(id).unwrap().components, 2);
    // Entered while started: Start(1) — the brush is active.
    assert!(brush_of(&m, id).active);
    m.set_zoom(0, 0, &mut h);
    assert_eq!(m.effect(id).unwrap().components, 1);
    m.set_zoom(5, 0, &mut h);
    assert_eq!((m.level_of_detail(), m.effect(id).unwrap().components), (6, 0));
    m.set_zoom(-3, 2, &mut h);
    assert_eq!(m.level_of_detail(), 1);
}

#[test]
fn nesting_is_limited_per_activation() {
    // An effect that contains itself: ten levels per creation.
    let dir = one_effect("loop", vec![ComponentRecord::meta("loop")]);
    let mut m = manager(dir);
    m.create("loop").unwrap();
    assert_eq!(m.effect_count(), 11);
}

#[test]
fn forward_parameter_reaches_nested_game_records() {
    let mut dir = shipped::volcano_directory();
    dir.effects
        .push(EffectDescription::new(1, vec![ComponentRecord::meta("loc")]));
    dir.effects.push(EffectDescription::new(
        1,
        vec![ComponentRecord::new(component_type::GAME, 19)],
    ));
    let n = dir.effects.len() as u32;
    dir.names.push(("outer".into(), n - 2));
    dir.names.push(("loc".into(), n - 1));
    dir.reindex();
    let mut m = manager(dir.into());
    let mut h = Host::new();
    let id = started(&mut m, &mut h, "outer", 40.0, 56.0);
    m.forward_parameter(id, shipped::INSTANCE_PARAMETER, 9);
    m.tick(1000, &mut h);
    // Updated twice (nested and from the list) but applied once.
    assert_eq!(
        h.posts(),
        vec![EffectPost {
            msg_type: msg::EFFECT_LOCATION,
            data1: 9,
            data2: 2,
            data3: 3
        }]
    );
}

#[test]
fn demolish_filter() {
    let f = DemolishFilter {
        no_networks: true,
        no_flora: false,
        min_size: 2.0,
        max_size: 48.0,
    };
    let o = OccupantFacts {
        extents: [10.0, 5.0, 10.0],
        ..OccupantFacts::default()
    };
    assert!(!f.skips(&o));
    assert!(f.skips(&OccupantFacts { signpost: true, ..o }));
    assert!(f.skips(&OccupantFacts { text_decal: true, ..o }));
    assert!(f.skips(&OccupantFacts { network: true, ..o }));
    assert!(!f.skips(&OccupantFacts { flora: true, ..o }));
    assert!(f.skips(&OccupantFacts {
        extents: [48.0, 5.0, 10.0],
        ..o
    }));
    assert!(f.skips(&OccupantFacts {
        extents: [10.0, 2.0, 10.0],
        ..o
    }));
    let none = DemolishFilter::default();
    assert!(!none.skips(&OccupantFacts {
        extents: [999.0, 0.0, 0.0],
        ..o
    }));
}

// ---- the directory -------------------------------------------------------------------------------

#[test]
fn directory_round_trip_and_locator() {
    let mut dir = shipped::volcano_directory();
    dir.attractors.push(AttractorRecord {
        version: 0,
        name: "volcano_disaster".into(),
        value: 1,
    });
    let bytes = dir.to_bytes();
    let back = EffectDirectory::parse(&bytes).unwrap();
    assert_eq!(back.brushes, dir.brushes);
    assert_eq!(back.games, dir.games);
    assert_eq!(back.attractors, dir.attractors);
    assert_eq!(back.effects, dir.effects);
    assert_eq!(back.names, dir.names);
    assert_eq!(back.end_index, Some(0xFFFF_FFFF));
    assert_eq!(back.to_bytes(), bytes);
    // Unknown render sections before the brushes and before the effects.
    let o = back.offsets;
    let mut padded = bytes[..2].to_vec();
    padded.extend_from_slice(&[0x55; 37]);
    padded.extend_from_slice(&bytes[2..o.games_end]);
    padded.extend_from_slice(&[0xEE; 23]);
    padded.extend_from_slice(&bytes[o.games_end..]);
    let p = EffectDirectory::parse(&padded).unwrap();
    assert_eq!(p.offsets.brushes, 39);
    assert_eq!(p.offsets.effects, o.games_end + 37 + 23);
    assert_eq!(p.effects, dir.effects);
    let q = EffectDirectory::parse_at(&padded, 39, o.games_end + 60).unwrap();
    assert_eq!(q.names, dir.names);
    // Record sizes.
    let mut w = Vec::new();
    directory::write_brush(&mut w, &BRUSHES[0]);
    assert_eq!(w.len(), directory::BRUSH_RECORD_BYTES);
    w.clear();
    directory::write_game(&mut w, &shipped::game_record(0).unwrap());
    assert_eq!(w.len(), directory::GAME_RECORD_BYTES);
    assert!(EffectDirectory::parse(&bytes[..bytes.len() - 3]).is_err());
}

#[test]
fn shipped_tables() {
    assert_eq!(shipped::game_records().len(), 57);
    let g = shipped::game_record(26).unwrap();
    assert_eq!(
        (g.flags, g.demolish_radius, g.demolish_flags, g.max_demolish_size),
        (1, 4, 0x1302, 48.0)
    );
    assert_eq!(g.message_type, msg::TORNADO_LOCATION);
    assert_eq!(shipped::game_record(55).unwrap().toxic, 20);
    assert!(shipped::game_record(57).is_none());
    let t = shipped::ToolEffects::of("tornado");
    assert_eq!(
        (t.control.as_str(), t.button_trigger.as_str()),
        ("tornado_control", "tornado_button_trigger")
    );
    assert_eq!(VOLCANO_CREATORS.iter().map(|c| u32::from(c.weight)).sum::<u32>(), 65535);
    assert_eq!(
        shipped::METEOR_CRATER_WEIGHTS
            .iter()
            .map(|&w| u32::from(w))
            .sum::<u32>(),
        65535
    );
    assert_eq!(
        shipped::LIGHTNING_BOLT_WEIGHTS
            .iter()
            .map(|&w| u32::from(w))
            .sum::<u32>(),
        65535
    );
    assert_eq!(shipped::select_member(&[1, 1], 3), None);
}

#[test]
fn shutdown_stops_without_posting() {
    let (mut m, mut h, _) = volcano(FinishMode::Hook);
    m.tick(33_333, &mut h);
    m.shutdown(&mut h);
    assert!(h.posts().is_empty());
    assert_eq!(m.effect_count(), 0);
    assert!(h.events.contains(&Ev::EndLong));
}

// ---- re-answers34 (EF1, EF4, EF5) -------------------------------------------------------------

#[test]
fn t17_dilation_table() {
    use super::dilation::{animation_time_dilation, SimTimeState};
    let s = |emergency_paused, driving, paused, hidden_paused, speed| SimTimeState {
        emergency_paused,
        driving,
        paused,
        hidden_paused,
        speed,
    };
    assert_eq!(animation_time_dilation(&s(false, false, true, false, 2)), 0.0);
    assert_eq!(animation_time_dilation(&s(true, false, true, false, 2)), 1.0);
    assert_eq!(animation_time_dilation(&s(false, true, true, false, 1)), 1.0);
    assert_eq!(animation_time_dilation(&s(false, false, false, false, 0)), 1.0);
    assert_eq!(animation_time_dilation(&s(false, false, false, false, 1)), 1.5);
    assert_eq!(animation_time_dilation(&s(false, false, false, false, 2)), 2.0);
    // A hidden pause changes nothing (EF5).
    for speed in 0..3 {
        assert_eq!(
            animation_time_dilation(&s(false, false, false, true, speed)),
            animation_time_dilation(&s(false, false, false, false, speed))
        );
    }
}

#[test]
fn paused_tick_runs_with_zero_dt() {
    // Paused: dilation 0.0, the tick still counts but nothing advances.
    let dir = one_effect("stroke", vec![ComponentRecord::new(component_type::BRUSH, 2)]);
    let mut m = manager(dir);
    let mut h = Host::new();
    h.dilation = Some(0.0);
    let id = started(&mut m, &mut h, "stroke", 100.0, 100.0);
    let before = brush_of(&m, id);
    for _ in 0..10 {
        m.tick(33_333, &mut h);
    }
    assert_eq!(m.total_time().0, 0);
    assert_eq!(m.stats().ticks, 10);
    assert_eq!(brush_of(&m, id).t, before.t);
    assert!(m.effect(id).unwrap().active);
}

#[test]
fn effects_tick_activates_nothing_without_a_queued_reload() {
    // §6.1 / EF12: DoEffectsTick neither activates nor rebuilds on an
    // ordinary tick; the draw-pass sweep ends in ActivateNewVisualEffects.
    let dir = one_effect("loop", vec![ComponentRecord::meta("loop")]);
    let mut m = manager(dir);
    let mut h = Host::new();
    m.create("loop").unwrap();
    assert_eq!((m.effect_count(), m.active_list().len()), (11, 10));
    assert_eq!(m.effects_tick(&mut h), 0);
    assert_eq!((m.effect_count(), m.active_list().len()), (11, 10));
    assert!(m.finish_hook(&mut h));
    assert_eq!((m.effect_count(), m.active_list().len()), (21, 20));
    let dir = one_effect("stroke", vec![ComponentRecord::new(component_type::BRUSH, 2)]);
    let mut m = manager(dir);
    m.create("stroke").unwrap();
    assert_eq!(m.effects_tick(&mut h), 0);
}

fn grouped(kind: u8, index: u32, group: u16, weight: u16, lod: (u8, u8)) -> ComponentRecord {
    let mut c = ComponentRecord::new(kind, index);
    c.select_group = group;
    c.select_weight = weight;
    c.lod_min = lod.0;
    c.lod_max = lod.1;
    c
}

#[test]
fn queued_reload_destroys_and_redraws_every_group() {
    // EF12: only a queued-file reload rebuilds on the effects tick: every
    // component is hard-stopped and released, every group redrawn.
    let comps = vec![
        grouped(component_type::BRUSH, 0, 1, 0x8000, (1, 6)),
        grouped(component_type::BRUSH, 1, 1, 0x7FFF, (1, 6)),
    ];
    let mut m = manager(one_effect("g", comps));
    let mut h = Host::new();
    let id = started(&mut m, &mut h, "g", 0.0, 0.0);
    assert_eq!((m.stats().select_draws, m.effect(id).unwrap().components), (1, 1));
    for _ in 0..3 {
        assert_eq!(m.effects_tick(&mut h), 0);
    }
    assert_eq!(m.stats().select_draws, 1);
    m.queue_effect_files();
    assert_eq!(m.effects_tick(&mut h), 0);
    assert_eq!((m.stats().select_draws, m.effect(id).unwrap().components), (2, 1));
    // Recreated while started and not stopping: Start(1).
    assert!(m.effect(id).unwrap().active);
    // The pending flag is consumed.
    assert_eq!(m.effects_tick(&mut h), 0);
    assert_eq!(m.stats().select_draws, 2);
    // DoRenderPropertiesChanged reloads only on a change.
    m.render_properties_changed(5, &mut h);
    assert_eq!(m.stats().select_draws, 2);
    m.render_properties_changed(4, &mut h);
    assert_eq!((m.max_priority(), m.stats().select_draws), (4, 3));
}

#[test]
fn zoom_draws_only_for_members_entering_the_range() {
    // §5.3 rebuild test (EF12). A (4..6) takes the whole weight, B (1..6)
    // none. New build at L = 1 draws; the draw lands on A, out of range, so
    // nothing in the group is created (B is skipped after the pick).
    let comps = vec![
        grouped(component_type::BRUSH, 0, 1, 0xFFFF, (4, 6)),
        grouped(component_type::BRUSH, 1, 1, 0, (1, 6)),
    ];
    let mut m = manager(one_effect("g", comps));
    let mut h = Host::new();
    let id = started(&mut m, &mut h, "g", 0.0, 0.0);
    assert_eq!((m.stats().select_draws, m.effect(id).unwrap().components), (1, 0));
    assert_eq!(m.previous_level_of_detail(), -1);
    // L 1 -> 1: nothing enters, no draw.
    m.set_zoom(0, 0, &mut h);
    assert_eq!((m.stats().select_draws, m.effect(id).unwrap().components), (1, 0));
    // L 1 -> 5: A enters and draws alone; B (in range at both) does not
    // take part.
    m.set_zoom(4, 0, &mut h);
    assert_eq!(m.previous_level_of_detail(), 1);
    assert_eq!((m.stats().select_draws, m.effect(id).unwrap().components), (2, 1));
    assert!(m.components(id).unwrap()[0].is_some());
    // L 5 -> 4: A stays (in range), nothing enters.
    m.set_zoom(3, 0, &mut h);
    assert_eq!((m.stats().select_draws, m.effect(id).unwrap().components), (2, 1));
    // L 4 -> 3: A leaves (hard stop); L 3 -> 4: A enters again and redraws.
    m.set_zoom(2, 0, &mut h);
    assert_eq!(m.effect(id).unwrap().components, 0);
    m.set_zoom(3, 0, &mut h);
    assert_eq!((m.stats().select_draws, m.effect(id).unwrap().components), (3, 1));
}

#[test]
fn a_group_summing_past_ffff_creates_a_second_member() {
    // §5.3: after a pick r := 0x10000, unsigned "sum >= r".
    let comps = vec![
        grouped(component_type::BRUSH, 0, 1, 0xFFFF, (1, 6)),
        grouped(component_type::BRUSH, 1, 1, 0xFFFF, (1, 6)),
    ];
    let mut m = manager(one_effect("g", comps));
    let id = m.create("g").unwrap();
    assert_eq!(m.effect(id).unwrap().components, 2);
}

#[test]
fn services_gate_blocks_update_active() {
    // §6.1: UpdateActiveVisualEffects is a no-op while +0x1AD is clear.
    let mut far = ComponentRecord::new(component_type::BRUSH, 0);
    far.lod_min = 4;
    far.lod_max = 6;
    let mut m = manager(one_effect("z", vec![far]));
    let mut h = Host::new();
    m.set_services_ready(false);
    let id = m.create("z").unwrap();
    m.set_zoom(4, 0, &mut h);
    assert_eq!(m.effect(id).unwrap().components, 0);
    m.post_app_services_init_complete(&mut h);
    assert_eq!(m.effect(id).unwrap().components, 1);
}
