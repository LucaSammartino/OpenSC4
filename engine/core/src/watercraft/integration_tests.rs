// SPDX-License-Identifier: GPL-3.0-or-later
use super::tests::{boat, Water};
use super::*;
use crate::automata::{State, Vec3};
use std::sync::{Arc, Mutex};

struct Bounds {
    outside: Option<Cell>,
    calls: Mutex<Vec<Cell>>,
}
impl Terrain for Bounds {
    fn contains(&self, cell: Cell) -> bool {
        self.calls.lock().unwrap().push(cell);
        Some(cell) != self.outside
    }
    fn cell_at(&self, _: Vec3) -> Cell {
        panic!("bounds helper must not convert runtime position")
    }
    fn terrain_height(&self, _: Vec3) -> f32 {
        panic!("bounds helper must not sample terrain")
    }
    fn terrain_height_at_cell(&self, _: Cell) -> f32 {
        panic!("bounds helper must not sample terrain")
    }
    fn sea_level(&self) -> f32 {
        panic!("bounds helper must not sample terrain")
    }
    fn water_height(&self, _: Vec3) -> f32 {
        panic!("bounds helper must not sample terrain")
    }
    fn navigable(&self, _: Cell, _: u32, _: Option<&Resource>) -> bool {
        panic!("bounds helper must not test navigability")
    }
}

#[test]
fn outside_corner_order_and_movement_flag_and_top_state_gates() {
    let rectangle = [
        Cell { x: 1, z: 2 },
        Cell { x: 1, z: 4 },
        Cell { x: 3, z: 2 },
        Cell { x: 3, z: 4 },
    ];
    let ordered = [rectangle[0], rectangle[2], rectangle[1], rectangle[3]];
    for end in 0..=4 {
        let terrain = Bounds {
            outside: ordered.get(end).copied(),
            calls: Mutex::default(),
        };
        assert_eq!(Watercraft::outside_city(rectangle, &terrain), end < 4);
        assert_eq!(*terrain.calls.lock().unwrap(), ordered[..(end + 1).min(4)]);
        terrain.calls.lock().unwrap().clear();
        let mut f = Fleet::default();
        let mut b = boat();
        b.record.base.automata_flags |= 1;
        f.insert(1, b).unwrap();
        assert!(!f.movement_possible(1, rectangle, &terrain, &mut None).unwrap());
        assert!(terrain.calls.lock().unwrap().is_empty());
        f.get_mut(1).unwrap().record.base.automata_flags &= !1;
        assert_eq!(f.movement_possible(1, rectangle, &terrain, &mut None), Ok(end == 4));
        assert_eq!(
            f.get(1).unwrap().top_state(),
            (end < 4).then_some(state::IMMEDIATE_REMOVE)
        );
        assert_eq!(*terrain.calls.lock().unwrap(), ordered[..(end + 1).min(4)]);
    }
    for kind in [state::RUN_AGROUND, state::SINK] {
        let mut f = Fleet::default();
        let mut b = boat();
        b.record.base.states = vec![State::new(kind)];
        f.insert(1, b).unwrap();
        assert_eq!(f.movement_possible(1, rectangle, &Water, &mut None), Ok(false));
        f.get_mut(1).unwrap().record.base.states.push(State::new(state::IDLE));
        assert_eq!(f.movement_possible(1, rectangle, &Water, &mut None), Ok(true));
    }
}

#[test]
fn wc3_h02_movement_top_queries_short_circuit_and_reread() {
    for (answers, expected, calls) in [
        ([Some(state::RUN_AGROUND), None], false, 1),
        ([Some(0), Some(state::SINK)], false, 2),
        ([Some(0), Some(0)], true, 2),
        ([Some(state::SINK), Some(0)], true, 2),
    ] {
        let mut count = 0;
        let allowed = super::movement::movement_state_allows(|| {
            let answer = answers[count];
            count += 1;
            answer
        });
        assert_eq!((allowed, count), (expected, calls));
    }
}

#[test]
fn end_towing_keeps_link_during_slot53_and_clears_its_reentrant_replacement_and_tug() {
    for replacement in [None, Some(3)] {
        let mut f = Fleet::default();
        let mut b = boat();
        b.towed = Some(2);
        b.tug = Some(4);
        b.speed = 0.25;
        b.still_elapsed = 123.0;
        f.insert(1, b).unwrap();
        let mut calls = Vec::new();
        f.end_towing_with(1, |fleet, target| {
            calls.push(target);
            assert_eq!(fleet.get(1).unwrap().towed, Some(2));
            fleet.get_mut(1).unwrap().towed = replacement;
            Ok(())
        })
        .unwrap();
        assert_eq!(calls, [2]);
        let b = f.get(1).unwrap();
        assert_eq!((b.towed, b.tug), (None, None));
        assert_eq!((b.speed, b.still_elapsed), (0.25, 123.0));
        assert!(b.events.is_empty());
        f.end_towing_with(1, |_, _| panic!("cleared link must not call slot 53 again"))
            .unwrap();
    }
}

#[test]
fn ordinary_tow_end_lets_towed_state_exit_clear_the_reciprocal_link() {
    let mut f = Fleet::default();
    let mut tug = boat();
    tug.towed = Some(2);
    f.insert(1, tug).unwrap();
    let mut follower = boat();
    follower.tug = Some(1);
    f.insert(2, follower).unwrap();
    f.push_state(2, State::new(state::TOWED), &mut None).unwrap();
    f.end_towing(1, &mut None).unwrap();
    assert_eq!(f.get(1).unwrap().towed, None);
    assert_eq!(f.get(2).unwrap().tug, None);
    assert_eq!(f.get(2).unwrap().top_state(), None);
}

#[test]
fn init_clears_each_previous_effect_before_its_factory_call() {
    struct Effects {
        old: Vec<std::sync::Weak<Resource>>,
        names: Vec<&'static str>,
    }
    impl EffectResources for Effects {
        fn create(&mut self, name: &'static str) -> Option<Arc<Resource>> {
            let index = self.names.len();
            assert!(self.old[index].upgrade().is_none());
            assert!(self.old[index + 1..].iter().all(|old| old.upgrade().is_some()));
            self.names.push(name);
            None
        }
    }
    let mut b = boat();
    b.effects = std::array::from_fn(|index| Some(Arc::new(Resource(index as u64))));
    let mut effects = Effects {
        old: b
            .effects
            .iter()
            .map(|effect| Arc::downgrade(effect.as_ref().unwrap()))
            .collect(),
        names: vec![],
    };
    assert_eq!(b.initialize_runtime([0.0; 3], [1.0, 0.0, 0.0], &mut effects), Ok(true));
    assert_eq!(effects.names, EFFECTS);
    assert!(b.effects.iter().all(Option::is_none));
}

#[test]
#[should_panic(expected = "collision size requires a group")]
fn collision_size_has_a_required_group_instead_of_an_invented_class_one_fallback() {
    let mut f = Fleet::default();
    let mut a = boat();
    a.group = None;
    a.speed = 1.0;
    f.insert(1, a).unwrap();
    f.insert(2, boat()).unwrap();
    let _ = f.respond_to_boat_collision(1, 2, [Cell { x: 1, z: 1 }; 4], &Water, &mut None);
}

#[derive(Debug, PartialEq)]
enum RopeCall {
    Transform(Transform),
    Start(bool),
    Stop(bool),
}
#[derive(Default)]
struct Rope {
    running: bool,
    calls: Vec<RopeCall>,
}
impl RopeEffect for Rope {
    fn set_transform(&mut self, transform: &Transform) {
        self.calls.push(RopeCall::Transform(transform.clone()));
    }
    fn running(&self) -> bool {
        self.running
    }
    fn start(&mut self, control: bool) {
        self.calls.push(RopeCall::Start(control));
        self.running = true;
    }
    fn stop(&mut self, control: bool) {
        self.calls.push(RopeCall::Stop(control));
        self.running = false;
    }
}
#[derive(Default)]
struct Player(Option<Rope>);
impl PlayerEffects for Player {
    fn play_fire(&mut self, _: &'static str) -> bool {
        false
    }
    fn fire_model_hook(&mut self) {}
    fn has_waterski(&self) -> bool {
        false
    }
    fn waterski_transform(&mut self, _: &Transform) {}
    fn waterski(&mut self, _: bool) {}
    fn command_event(&mut self, _: usize, _: u32) {}
    fn rope_throw(&mut self) -> Option<&mut dyn RopeEffect> {
        self.0.as_mut().map(|rope| rope as &mut dyn RopeEffect)
    }
}
fn tug() -> Watercraft {
    let mut b = boat();
    b.group = Some(Arc::new(Group {
        id: 0x4410,
        parameters: None,
        aems_value: Some(1.0),
        parent: None,
    }));
    b.transform.translation = [7.0, 8.0, 9.0];
    b.transform.scale = 3.0;
    b.transform.revision = 19;
    b
}

#[test]
fn tow_throw_start_and_independent_refresh_copy_the_complete_transform() {
    for present in [false, true] {
        let mut f = Fleet::default();
        let b = tug();
        let transform = b.transform.clone();
        f.insert(1, b).unwrap();
        let mut effects = Player(present.then(Rope::default));
        let mut command = DriveCommand {
            tow: true,
            ..DriveCommand::default()
        };
        f.player_commands(1, 10.0, &mut command, &mut effects, &mut None)
            .unwrap();
        assert!(f.get(1).unwrap().tow_armed);
        assert!(command.tow);
        if let Some(rope) = effects.0.as_mut() {
            assert_eq!(
                rope.calls,
                [
                    RopeCall::Transform(transform.clone()),
                    RopeCall::Start(false),
                    RopeCall::Transform(transform)
                ]
            );
            rope.calls.clear();
            command.tow = false;
            let b = f.get_mut(1).unwrap();
            b.speed = 0.1;
            b.transform.translation[0] = 42.0;
            let transform = b.transform.clone();
            f.player_commands(1, 10.0, &mut command, &mut effects, &mut None)
                .unwrap();
            assert_eq!(effects.0.as_ref().unwrap().calls, [RopeCall::Transform(transform)]);
        }
    }
}

#[test]
fn rope_connection_refresh_has_distinct_tow_start_refresh_stop_and_missing_effect_paths() {
    let mut b = boat();
    let mut rope = Rope::default();
    b.towed = Some(2);
    b.refresh_rope(Some(&mut rope));
    b.refresh_rope(Some(&mut rope));
    b.towed = None;
    b.refresh_rope(Some(&mut rope));
    b.refresh_rope(Some(&mut rope));
    assert_eq!(
        rope.calls,
        [
            RopeCall::Transform(b.transform.clone()),
            RopeCall::Start(false),
            RopeCall::Transform(b.transform.clone()),
            RopeCall::Stop(false),
        ]
    );
    b.towed = Some(2);
    b.refresh_rope(None); // Nullable only on the towed-reference branch.
}

#[test]
#[should_panic(expected = "no-tow refresh requires the connection effect")]
fn missing_rope_connection_without_a_tow_is_a_required_effect_precondition() {
    boat().refresh_rope(None);
}
