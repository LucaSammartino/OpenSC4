// SPDX-License-Identifier: GPL-3.0-or-later
//! Literal ordered traces from fixtures.md, committed before implementation.
use super::*;
use crate::view_input_controls::ControlHost;
use std::num::{NonZeroU32, NonZeroU64};

const P: Vec3 = Vec3::new(33.75, 4.9, 65.25);
const Q: Vec3 = Vec3::new(112.0, 8.0, 144.0);
const M: Pixel = Pixel::new(10, 20);

fn resource(value: u64) -> Resource {
    Resource(NonZeroU64::new(value).unwrap())
}

fn indicator(value: u32) -> Option<Indicator> {
    Some(Indicator(NonZeroU32::new(value).unwrap()))
}

struct Host {
    trace: Vec<String>,
    pick: Option<Vec3>,
    funds: i64,
    inserted: bool,
    rewritten: Option<Vec3>,
    withdrawal: bool,
    water: bool,
    location: bool,
    next_effect: u64,
    create_error: Option<PrimitiveFailure>,
    started: bool,
    level: u32,
    managers: bool,
    hit: Option<Indicator>,
    occupant: Option<Resource>,
    cursor_available: bool,
}

impl Host {
    fn new() -> Self {
        Self {
            trace: vec![],
            pick: Some(P),
            funds: 100,
            inserted: true,
            rewritten: None,
            withdrawal: true,
            water: false,
            location: true,
            next_effect: 21,
            create_error: None,
            started: true,
            level: 1,
            managers: true,
            hit: indicator(7),
            occupant: Some(resource(31)),
            cursor_available: true,
        }
    }

    fn expect(&mut self, expected: &[&str]) {
        assert_eq!(std::mem::take(&mut self.trace), expected);
    }

    fn flora(&mut self) -> Flora {
        let flora = Flora::init(
            7,
            100,
            resource(11),
            FloraInputState {
                pressed: false,
                hover_valid: None,
                out_of_money: false,
            },
            self,
        )
        .unwrap();
        self.expect(&["price 100"]);
        flora
    }

    fn torch(&mut self) -> Torch {
        let torch = Torch::init(resource(12), resource(13), self);
        self.expect(&["timer-start 13", "audio 2a106113 unpublished"]);
        torch
    }

    fn prepared_torch(&mut self) -> Torch {
        let mut torch = self.torch();
        assert_eq!(torch.mouse_move(self, M, 0), Ok(true));
        self.expect(&[
            "pick 10 20",
            "create torch_inactive",
            "start 21 0",
            "transform 21 33.75 4.9 65.25 identity 1",
            "count 13",
            "audio 4a106126 10 20 123",
        ]);
        torch
    }
}

impl FloraHost for Host {
    fn pick_terrain(&mut self, mouse: Pixel) -> Option<Vec3> {
        self.trace.push(format!("pick {} {}", mouse.x, mouse.y));
        self.pick
    }
    fn funds(&mut self) -> i64 {
        self.trace.push(format!("funds {}", self.funds));
        self.funds
    }
    fn insert_flora(&mut self, simulator: Resource, kind: u32, p: &mut Vec3, force: bool) -> bool {
        self.trace
            .push(format!("insert {} {kind} {} {} {} {force}", simulator.0, p.x, p.y, p.z));
        if let Some(rewritten) = self.rewritten {
            *p = rewritten;
        }
        self.inserted
    }
    fn location_ok(&mut self, simulator: Resource, kind: u32, x: f32, z: f32, exclude: Option<Resource>) -> bool {
        assert_eq!(exclude, None);
        self.trace.push(format!("location {} {kind} {x} {z} none", simulator.0));
        self.location
    }
    fn is_water_vertex(&mut self, vertex: Cell) -> bool {
        self.trace.push(format!("water {} {}", vertex.x, vertex.z));
        self.water
    }
    fn new_sound_instance(&mut self) -> Result<u32, PrimitiveFailure> {
        self.trace.push("instance 9".into());
        Ok(9)
    }
    fn plant_sound(&mut self, message: PlantSound) {
        self.trace.push(match message {
            PlantSound::Start { event, sound, instance } => format!("sound-start {event:08x} {sound:08x} {instance}"),
            PlantSound::Position { event, instance, xyz } => {
                format!("sound-position {event:08x} {instance} {} {} {}", xyz[0], xyz[1], xyz[2])
            }
        });
    }
    fn result_sound(&mut self, id: u32) {
        self.trace.push(format!("result {id:08x}"));
    }
    fn withdraw(&mut self, price: i64) -> bool {
        self.trace.push(format!("withdraw {price}"));
        self.withdrawal
    }
    fn cost_indicator(&mut self, intent: CostIndicator) {
        self.trace.push(match intent {
            CostIndicator::Price(price) => format!("price {price}"),
            CostIndicator::Stroke { total, price } => format!("cost {total} {price}"),
            CostIndicator::ClearPositiveStroke => "clear positive unresolved-flag".into(),
            CostIndicator::ClearShutdown { flag } => format!("clear shutdown {flag}"),
        });
    }
    fn transient_aura(&mut self, x: f32, z: f32, kind: u32) {
        self.trace.push(format!("aura {x} {z} {kind}"));
    }
    fn release_simulator(&mut self, simulator: Resource) {
        self.trace.push(format!("release-flora {}", simulator.0));
    }
}

impl ControlHost for Host {
    fn lookup_cursor(&mut self, id: u32) -> Option<u64> {
        self.trace.push(format!("lookup {id:08x}"));
        self.cursor_available.then_some(u64::from(id))
    }
    fn apply_window_cursor(&mut self, cursor: u64, force: bool) {
        self.trace.push(format!("cursor {cursor:08x} {force}"));
    }
    fn am_capturing(&self) -> bool {
        panic!("not a capture owner")
    }
    fn capture(&mut self) -> bool {
        panic!("not a capture owner")
    }
    fn release_capture(&mut self) -> bool {
        panic!("not a capture owner")
    }
    fn remove_top_control(&mut self, _: bool) {
        panic!("not a stack owner")
    }
    fn post_sound(&mut self, _: u32, _: u32) {
        panic!("not a result-sound or CT2 primitive")
    }
}

impl TorchHost for Host {
    fn pick_terrain(&mut self, mouse: Pixel) -> Option<Vec3> {
        FloraHost::pick_terrain(self, mouse)
    }
    fn create_effect(&mut self, name: &'static str) -> Result<Resource, PrimitiveFailure> {
        self.trace.push(format!("create {name}"));
        if let Some(error) = self.create_error {
            return Err(error);
        }
        let effect = resource(self.next_effect);
        self.next_effect += 1;
        Ok(effect)
    }
    fn effect_started(&mut self, effect: Resource) -> bool {
        self.trace.push(format!("started {}", effect.0));
        self.started
    }
    fn start_effect(&mut self, effect: Resource, flags: u32) {
        self.trace.push(format!("start {} {flags}", effect.0));
    }
    fn stop_effect(&mut self, effect: Resource, flags: u32) {
        self.trace.push(format!("stop {} {flags}", effect.0));
    }
    fn release_effect(&mut self, effect: Resource) {
        self.trace.push(format!("release-effect {}", effect.0));
    }
    fn transform_effect(&mut self, effect: Resource, p: Vec3, rotation: [f32; 9], scale: f32) {
        assert_eq!(rotation, [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
        self.trace.push(format!(
            "transform {} {} {} {} identity {scale}",
            effect.0, p.x, p.y, p.z
        ));
    }
    fn start_timer(&mut self, timer: Resource) {
        self.trace.push(format!("timer-start {}", timer.0));
    }
    fn stop_timer(&mut self, timer: Resource) {
        self.trace.push(format!("timer-stop {}", timer.0));
    }
    fn timer_count(&mut self, timer: Resource) -> u64 {
        self.trace.push(format!("count {}", timer.0));
        123
    }
    fn torch_audio(&mut self, intent: TorchAudio) {
        self.trace.push(match intent.data {
            TorchAudioData::Unpublished => format!("audio {:08x} unpublished", intent.message),
            TorchAudioData::Active(on) => format!("audio {:08x} {on}", intent.message),
            TorchAudioData::Mouse { mouse, count } => {
                format!("audio {:08x} {} {} {count}", intent.message, mouse.x, mouse.y)
            }
        });
    }
    fn register_tick(&mut self, value: u32) {
        self.trace.push(format!("register {value:02x}"));
    }
    fn unregister_tick(&mut self) {
        self.trace.push("unregister".into());
    }
    fn ignite_cell(&mut self, fire: Resource, cell: Cell, probability: bool, teleport: bool, fire_id: u32) {
        self.trace.push(format!(
            "ignite {} {} {} {probability} {teleport} {fire_id}",
            fire.0, cell.x, cell.z
        ));
    }
    fn set_cause(&mut self, fire: Resource, cause: u32) {
        self.trace.push(format!("cause {} {cause}", fire.0));
    }
    fn release_fire_manager(&mut self, fire: Resource) {
        self.trace.push(format!("release-fire {}", fire.0));
    }
}

impl SituationsHost for Host {
    fn subscribe(&mut self, id: u32) {
        self.trace.push(format!("subscribe {id:08x}"));
    }
    fn view_level(&mut self) -> u32 {
        self.trace.push(format!("level {}", self.level));
        self.level
    }
    fn both_managers_present(&mut self) -> bool {
        self.trace.push(format!("managers {}", self.managers));
        self.managers
    }
    fn find_indicator(&mut self, mouse: Pixel) -> Option<Indicator> {
        self.trace.push(format!("find {} {}", mouse.x, mouse.y));
        self.hit
    }
    fn lock_indicator(&mut self, handle: Indicator, on: bool) {
        self.trace.push(format!("lock {} {on}", handle.0));
    }
    fn set_active_indicator(&mut self, handle: Option<Indicator>, state: u32) {
        self.trace
            .push(format!("indicator {} {state}", handle.map_or(0, |h| h.0.get())));
    }
    fn active_occupant(&mut self) -> Option<Resource> {
        self.trace.push("occupant".into());
        self.occupant
    }
    fn initiate_situation(&mut self, occupant: Resource, advice: Option<Resource>) -> bool {
        assert_eq!(advice, None);
        self.trace.push(format!("initiate {} none", occupant.0));
        false // Refusal is not the tool's handler return.
    }
}

#[test]
fn flora_poor_press_latches_and_zero_release_resets() {
    let mut h = Host::new();
    let mut f = h.flora();
    h.funds = 50;
    assert_eq!(f.press(&mut h, M), Ok(true));
    h.expect(&["pick 10 20", "funds 50"]);
    assert!(f.input().pressed);
    assert!(!f.input().out_of_money);
    assert_eq!(f.total(), 0);
    assert_eq!(f.release(&mut h), Ok(true));
    h.expect(&["cost 100 100"]);
    assert_eq!(
        f.input(),
        FloraInputState {
            pressed: false,
            hover_valid: None,
            out_of_money: false
        }
    );
}

#[test]
fn flora_shortage_repeats_on_every_move_even_miss_and_recovery() {
    let mut h = Host::new();
    let mut f = h.flora();
    h.funds = 50;
    f.press(&mut h, M).unwrap();
    h.expect(&["pick 10 20", "funds 50"]);
    for _ in 0..2 {
        assert_eq!(f.mouse_move(&mut h, M), Ok(true));
        h.expect(&["pick 10 20", "funds 50", "result aa62c9d5"]);
    }
    h.pick = None;
    f.mouse_move(&mut h, M).unwrap();
    h.expect(&["pick 10 20", "result aa62c9d5"]);
    h.pick = Some(P);
    h.funds = 100;
    f.mouse_move(&mut h, M).unwrap();
    h.expect(&[
        "pick 10 20",
        "funds 100",
        "insert 11 7 33.75 4.9 65.25 false",
        "instance 9",
        "sound-start ea451903 49dc0c90 9",
        "sound-position ca45190f 9 33 4 65",
        "withdraw 100",
        "cost 100 100",
        "aura 33.75 65.25 1",
        "result aa62c9d5",
    ]);
    assert!(f.input().out_of_money);
    assert_eq!(f.release(&mut h), Ok(true));
    h.expect(&["clear positive unresolved-flag"]);
    assert_eq!(f.total(), 0);
    assert!(!f.input().out_of_money);
    f.mouse_move(&mut h, M).unwrap();
    h.expect(&["pick 10 20", "location 11 7 33.75 65.25 none"]);
}

#[test]
fn flora_failed_withdrawal_press_and_drag_are_counterexamples() {
    let mut h = Host::new();
    let mut f = h.flora();
    h.withdrawal = false;
    f.press(&mut h, M).unwrap();
    h.expect(&[
        "pick 10 20",
        "funds 100",
        "insert 11 7 33.75 4.9 65.25 false",
        "instance 9",
        "sound-start ea451903 49dc0c90 9",
        "sound-position ca45190f 9 33 4 65",
        "withdraw 100",
        "aura 33.75 65.25 1",
    ]);
    assert_eq!(f.total(), 0);
    f.mouse_move(&mut h, M).unwrap();
    h.expect(&[
        "pick 10 20",
        "funds 100",
        "insert 11 7 33.75 4.9 65.25 false",
        "instance 9",
        "sound-start ea451903 49dc0c90 9",
        "sound-position ca45190f 9 33 4 65",
        "withdraw 100",
        "cost 100 100",
        "aura 33.75 65.25 1",
    ]);
    assert_eq!(f.total(), 100);
    f.release(&mut h).unwrap();
    h.expect(&["clear positive unresolved-flag"]);
}

#[test]
fn flora_success_uses_insertion_rewritten_position_and_press_total() {
    let mut h = Host::new();
    let mut f = h.flora();
    h.rewritten = Some(Vec3::new(48.9, 6.8, 80.7));
    f.press(&mut h, M).unwrap();
    h.expect(&[
        "pick 10 20",
        "funds 100",
        "insert 11 7 33.75 4.9 65.25 false",
        "instance 9",
        "sound-start ea451903 49dc0c90 9",
        "sound-position ca45190f 9 48 6 80",
        "withdraw 100",
        "cost 100 100",
        "aura 48.9 80.7 1",
    ]);
    assert_eq!(f.total(), 100);
}

#[test]
fn flora_failed_plant_water_vertex_truncates_toward_zero() {
    let mut h = Host::new();
    let mut f = h.flora();
    h.inserted = false;
    h.pick = Some(Vec3::new(-31.5, 4.9, -47.75));
    h.water = true;
    f.press(&mut h, M).unwrap();
    h.expect(&[
        "pick 10 20",
        "funds 100",
        "insert 11 7 -31.5 4.9 -47.75 false",
        "water -1 -2",
        "result aa62c9d5",
    ]);
    assert_eq!(f.total(), 0);
    h.rewritten = Some(Vec3::new(48.9, 6.8, 80.7));
    h.water = false;
    f.mouse_move(&mut h, M).unwrap();
    h.expect(&[
        "pick 10 20",
        "funds 100",
        "insert 11 7 -31.5 4.9 -47.75 false",
        "water 3 5",
    ]);
}

#[test]
fn flora_hover_miss_and_shutdown_do_not_invent_state_or_releases() {
    let mut h = Host::new();
    let mut f = h.flora();
    h.location = false;
    f.mouse_move(&mut h, M).unwrap();
    h.expect(&["pick 10 20", "location 11 7 33.75 65.25 none"]);
    assert_eq!(f.input().hover_valid, Some(false));
    h.pick = None;
    f.press(&mut h, M).unwrap();
    f.mouse_move(&mut h, M).unwrap();
    h.expect(&["pick 10 20", "pick 10 20"]);
    assert!(!f.input().pressed);
    assert_eq!(f.input().hover_valid, Some(false));
    f.shutdown(&mut h).unwrap();
    h.expect(&["release-flora 11", "clear shutdown true"]);
    assert_eq!(f.shutdown(&mut h), Err(Unsupported::InactiveLifetime));
    assert_eq!(f.press(&mut h, M), Err(Unsupported::InactiveLifetime));
    h.expect(&[]);
}

#[test]
fn flora_int64_total_and_overflow_preserve_truthful_prefix() {
    let mut h = Host::new();
    let state = FloraInputState {
        pressed: false,
        hover_valid: None,
        out_of_money: false,
    };
    let mut f = Flora::init(7, 3_000_000_000, resource(11), state, &mut h).unwrap();
    h.expect(&["price 3000000000"]);
    h.funds = 3_000_000_000;
    f.press(&mut h, M).unwrap();
    h.expect(&[
        "pick 10 20",
        "funds 3000000000",
        "insert 11 7 33.75 4.9 65.25 false",
        "instance 9",
        "sound-start ea451903 49dc0c90 9",
        "sound-position ca45190f 9 33 4 65",
        "withdraw 3000000000",
        "cost 3000000000 3000000000",
        "aura 33.75 65.25 1",
    ]);
    assert_eq!(f.total(), 3_000_000_000);
    let mut f = Flora::init(7, i64::MAX, resource(11), state, &mut h).unwrap();
    h.expect(&["price 9223372036854775807"]);
    h.funds = i64::MAX;
    f.press(&mut h, M).unwrap();
    h.expect(&[
        "pick 10 20",
        "funds 9223372036854775807",
        "insert 11 7 33.75 4.9 65.25 false",
        "instance 9",
        "sound-start ea451903 49dc0c90 9",
        "sound-position ca45190f 9 33 4 65",
        "withdraw 9223372036854775807",
        "cost 9223372036854775807 9223372036854775807",
        "aura 33.75 65.25 1",
    ]);
    assert_eq!(f.mouse_move(&mut h, M), Err(Unsupported::ArithmeticOverflow));
    h.expect(&[
        "pick 10 20",
        "funds 9223372036854775807",
        "insert 11 7 33.75 4.9 65.25 false",
        "instance 9",
        "sound-start ea451903 49dc0c90 9",
        "sound-position ca45190f 9 33 4 65",
        "withdraw 9223372036854775807",
    ]);
    assert_eq!(f.total(), i64::MAX);
    for price in [0, -1] {
        assert_eq!(
            Flora::init(7, price, resource(11), state, &mut h).unwrap_err(),
            Unsupported::NonpositivePrice
        );
    }
    h.expect(&[]);
}

#[test]
fn torch_press_without_effect_or_hit_does_not_manufacture_activation() {
    let mut h = Host::new();
    let mut t = h.torch();
    assert_eq!(t.position(), None);
    assert_eq!(t.last_active_move(), Cell { x: 0, z: 0 });
    h.pick = None;
    assert_eq!(t.press(&mut h, M), Ok(true));
    h.expect(&["count 13", "audio 8a10611d 10 20 123", "pick 10 20"]);
    h.pick = Some(P);
    t.press(&mut h, M).unwrap();
    h.expect(&[
        "count 13",
        "audio 8a10611d 10 20 123",
        "pick 10 20",
        "register c0",
        "ignite 12 2 4 false true 0",
        "cause 12 1",
        "lookup 0a08c54e",
        "cursor 0a08c54e true",
    ]);
    assert!(!t.active());
    assert_eq!(t.position(), Some(P));
    assert_eq!(t.effect(), None);
    t.animation_tick(&mut h).unwrap();
    h.expect(&[]);
}

#[test]
fn torch_press_hit_and_first_tick_use_different_cells() {
    let mut h = Host::new();
    let mut t = h.prepared_torch();
    h.pick = Some(Q);
    t.press(&mut h, M).unwrap();
    h.expect(&[
        "count 13",
        "audio 8a10611d 10 20 123",
        "pick 10 20",
        "stop 21 0",
        "release-effect 21",
        "create torch_active",
        "start 22 0",
        "audio aa96fe8f true",
        "transform 22 112 8 144 identity 1",
        "register c0",
        "ignite 12 7 9 false true 0",
        "cause 12 1",
        "lookup 0a08c54e",
        "cursor 0a08c54e true",
    ]);
    assert!(t.active());
    assert_eq!(t.last_active_move(), Cell { x: 0, z: 0 });
    t.animation_tick(&mut h).unwrap();
    h.expect(&["ignite 12 0 0 true true 0", "cause 12 1"]);
}

#[test]
fn torch_release_replacement_and_next_tick_retain_last_active_move() {
    let mut h = Host::new();
    let mut t = h.torch();
    t.mouse_move(&mut h, M, 0x10).unwrap();
    h.expect(&[
        "pick 10 20",
        "create torch_inactive",
        "start 21 0",
        "stop 21 0",
        "release-effect 21",
        "create torch_active",
        "start 22 0",
        "audio aa96fe8f true",
        "transform 22 33.75 4.9 65.25 identity 1",
        "count 13",
        "audio 4a106126 10 20 123",
    ]);
    assert_eq!(t.last_active_move(), Cell { x: 2, z: 4 });
    t.mouse_move(&mut h, M, 0).unwrap();
    h.expect(&[
        "pick 10 20",
        "transform 22 33.75 4.9 65.25 identity 1",
        "count 13",
        "audio 4a106126 10 20 123",
    ]);
    assert!(t.active());
    assert_eq!(t.release(&mut h, M), Ok(true));
    h.expect(&[
        "count 13",
        "audio 0a106122 10 20 123",
        "unregister",
        "stop 22 0",
        "release-effect 22",
        "create torch_inactive",
        "start 23 0",
        "count 13",
        "audio 0a106122 10 20 123",
        "audio aa96fe8f false",
        "lookup 4a08c57e",
        "cursor 4a08c57e true",
    ]);
    t.animation_tick(&mut h).unwrap();
    h.expect(&[]);
    h.pick = Some(Q);
    t.press(&mut h, M).unwrap();
    h.expect(&[
        "count 13",
        "audio 8a10611d 10 20 123",
        "pick 10 20",
        "stop 23 0",
        "release-effect 23",
        "create torch_active",
        "start 24 0",
        "audio aa96fe8f true",
        "transform 24 112 8 144 identity 1",
        "register c0",
        "ignite 12 7 9 false true 0",
        "cause 12 1",
        "lookup 0a08c54e",
        "cursor 0a08c54e true",
    ]);
    t.animation_tick(&mut h).unwrap();
    h.expect(&["ignite 12 2 4 true true 0", "cause 12 1"]);
}

#[test]
fn torch_raw_modifier_bit_and_same_state_gate_are_literal() {
    for modifiers in [1, 8, 0x10, 0x90] {
        let mut h = Host::new();
        let mut t = h.prepared_torch();
        t.mouse_move(&mut h, M, modifiers).unwrap();
        if modifiers == 1 || modifiers == 8 {
            h.expect(&[
                "pick 10 20",
                "transform 21 33.75 4.9 65.25 identity 1",
                "count 13",
                "audio 4a106126 10 20 123",
            ]);
            assert!(!t.active());
        } else {
            h.expect(&[
                "pick 10 20",
                "stop 21 0",
                "release-effect 21",
                "create torch_active",
                "start 22 0",
                "audio aa96fe8f true",
                "transform 22 33.75 4.9 65.25 identity 1",
                "count 13",
                "audio 4a106126 10 20 123",
            ]);
            assert!(t.active());
        }
        t.activate(&mut h, t.active(), M).unwrap();
        h.expect(&[]);
    }
}

#[test]
fn torch_miss_exit_and_shutdown_order_distinguish_stop_flags_and_releases() {
    let mut h = Host::new();
    let mut t = h.prepared_torch();
    h.pick = None;
    t.mouse_move(&mut h, M, 0).unwrap();
    h.expect(&[
        "pick 10 20",
        "started 21",
        "stop 21 1",
        "release-effect 21",
        "audio aa96fe8f false",
        "count 13",
        "audio 4a106126 10 20 123",
    ]);
    h.pick = Some(P);
    t.mouse_move(&mut h, M, 0).unwrap();
    h.expect(&[
        "pick 10 20",
        "create torch_inactive",
        "start 22 0",
        "transform 22 33.75 4.9 65.25 identity 1",
        "count 13",
        "audio 4a106126 10 20 123",
    ]);
    h.started = false;
    t.mouse_exit(&mut h).unwrap();
    h.expect(&["started 22", "release-effect 22", "audio aa96fe8f false"]);
    t.mouse_exit(&mut h).unwrap();
    h.expect(&["audio aa96fe8f false"]);
    t.mouse_move(&mut h, M, 0).unwrap();
    h.expect(&[
        "pick 10 20",
        "create torch_inactive",
        "start 23 0",
        "transform 23 33.75 4.9 65.25 identity 1",
        "count 13",
        "audio 4a106126 10 20 123",
    ]);
    h.started = true;
    t.shutdown(&mut h).unwrap();
    h.expect(&[
        "timer-stop 13",
        "started 23",
        "stop 23 1",
        "release-effect 23",
        "audio aa96fe8f false",
        "audio 8a10611a unpublished",
        "release-fire 12",
        "unregister",
    ]);
    assert_eq!(t.shutdown(&mut h), Err(Unsupported::InactiveLifetime));
    h.expect(&[]);
}

#[test]
fn torch_foreign_creation_failure_preserves_prefix_without_fallback() {
    let mut h = Host::new();
    let mut t = h.prepared_torch();
    h.create_error = Some(PrimitiveFailure::Unavailable);
    assert_eq!(
        t.activate(&mut h, true, M),
        Err(Unsupported::ForeignResource(PrimitiveFailure::Unavailable))
    );
    h.expect(&["stop 21 0", "release-effect 21", "create torch_active"]);
    assert_eq!(t.effect(), None);
    assert!(!t.active());
    t.shutdown(&mut h).unwrap();
    h.expect(&[
        "timer-stop 13",
        "audio aa96fe8f false",
        "audio 8a10611a unpublished",
        "release-fire 12",
        "unregister",
    ]);
}

#[test]
fn coordinates_outside_portable_domain_are_residual_not_saturated() {
    let mut h = Host::new();
    let mut f = h.flora();
    h.pick = Some(Vec3::new(f32::NAN, 0.0, 0.0));
    assert_eq!(f.press(&mut h, M), Err(Unsupported::CoordinateConversion));
    h.expect(&["pick 10 20", "funds 100"]);
    assert!(f.input().pressed);
    assert_eq!(trunc_i32(2_147_483_648.0), Err(Unsupported::CoordinateConversion));
    assert_eq!(trunc_i32(f32::NEG_INFINITY), Err(Unsupported::CoordinateConversion));
    assert_eq!(trunc_i32(-2_147_483_648.0), Ok(i32::MIN));
}

#[test]
fn situations_hover_transition_unlocks_without_immediate_relock() {
    let mut h = Host::new();
    let mut s = CitySituations::new(None, None);
    s.init(&mut h);
    h.expect(&["subscribe a6b79602"]);
    assert_eq!(s.mouse_move(&mut h, M), Ok(false));
    h.expect(&["find 10 20", "lock 7 true", "indicator 7 3"]);
    assert_eq!(s.locked(), indicator(7));
    s.mouse_move(&mut h, M).unwrap();
    h.expect(&["find 10 20"]);
    h.hit = indicator(8);
    s.mouse_move(&mut h, M).unwrap();
    h.expect(&["find 10 20", "lock 7 false"]);
    assert_eq!(s.locked(), None);
    s.mouse_move(&mut h, M).unwrap();
    h.expect(&["find 10 20", "lock 8 true", "indicator 8 3"]);
    h.hit = None;
    s.mouse_move(&mut h, M).unwrap();
    h.expect(&["find 10 20", "lock 8 false"]);
    assert_eq!(s.mouse_move(&mut h, M), Err(Unsupported::EmptyHover));
    h.expect(&["find 10 20"]);
}

#[test]
fn situations_level_gate_refuses_and_release_clears_pressed() {
    let mut h = Host::new();
    let mut s = CitySituations::new(None, indicator(8));
    h.level = 2;
    assert!(!s.press(&mut h, M));
    h.expect(&["level 2"]);
    assert_eq!(s.pressed(), indicator(8));
    assert!(!s.release(&mut h, M));
    h.expect(&["level 2"]);
    assert_eq!(s.pressed(), None);
    h.level = 1;
    assert!(s.press(&mut h, M));
    h.expect(&["level 1", "find 10 20", "indicator 7 2"]);
    assert_eq!(s.pressed(), indicator(7));
}

#[test]
fn situations_release_sets_state_before_matched_mission_and_ignores_manager_result() {
    let mut h = Host::new();
    let mut s = CitySituations::new(None, indicator(7));
    assert!(s.release(&mut h, M));
    h.expect(&[
        "level 1",
        "managers true",
        "find 10 20",
        "indicator 7 1",
        "occupant",
        "initiate 31 none",
    ]);
    assert_eq!(s.pressed(), indicator(7)); // CT16 literal reading.
    h.hit = indicator(8);
    assert!(s.release(&mut h, M));
    h.expect(&["level 1", "managers true", "find 10 20", "indicator 8 1"]);
    h.hit = indicator(7);
    h.occupant = None;
    assert!(s.release(&mut h, M));
    h.expect(&["level 1", "managers true", "find 10 20", "indicator 7 1", "occupant"]);
}

#[test]
fn situations_missing_indicator_or_manager_refuses_and_forgets_press() {
    let mut h = Host::new();
    let mut s = CitySituations::new(None, indicator(7));
    h.hit = None;
    assert!(!s.release(&mut h, M));
    h.expect(&["level 1", "managers true", "find 10 20"]);
    assert_eq!(s.pressed(), None);
    assert!(!s.press(&mut h, M));
    h.expect(&["level 1", "find 10 20"]);
    h.managers = false;
    let mut s = CitySituations::new(None, indicator(7));
    assert!(!s.release(&mut h, M));
    h.expect(&["level 1", "managers false"]);
    assert_eq!(s.pressed(), None);
}

#[test]
fn situations_all_three_reset_messages_have_identical_order_despite_one_subscription() {
    let mut h = Host::new();
    for message in [0x06CD_B65B, 0xA6B7_9602, 0xA6B7_9612] {
        let mut s = CitySituations::new(indicator(7), indicator(8));
        assert_eq!(s.message(&mut h, message), Ok(true));
        h.expect(&["lock 7 false", "indicator 0 1"]);
        assert_eq!(s.locked(), None);
        assert_eq!(s.pressed(), None);
    }
    let mut s = CitySituations::new(None, None);
    assert_eq!(s.message(&mut h, 0xA6B7_9602), Ok(true));
    h.expect(&["indicator 0 1"]);
    assert_eq!(s.message(&mut h, 0), Err(Unsupported::UnknownMessage));
    h.expect(&[]);
}
