use super::*;
use crate::lot_records::Bounds;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Call {
    Effect(Effect),
    Factory,
    Dimensions,
    Altitude,
    Level,
    Lookup(u32),
    Add(Handle, Handle, [f32; 2], f32, f32),
    Terrain([f32; 2]),
    BasePosition([f32; 3]),
    BaseInit,
    BaseShutdown,
    BaseFlags(u32),
    Initialized,
    TempFilter,
    Filter(Handle),
    Config,
    Message(u32),
}
#[derive(Clone, Debug)]
struct Seen {
    call: Call,
    ids: [u32; 2],
    managers: [Option<Handle>; 2],
    source: Option<Handle>,
    size: f32,
    bounds: Bounds,
}
struct Raw {
    seen: Rc<RefCell<Vec<Seen>>>,
    dimensions: (u32, u32),
    altitude: f64,
    level: u32,
    managers: [Option<Handle>; 4],
    adds: [u32; 2],
    add_index: usize,
    factory: Option<Handle>,
    filter: Option<Handle>,
    accepts: bool,
    config: bool,
}
impl Raw {
    fn record(&self, state: &State, call: Call) {
        self.seen.borrow_mut().push(Seen {
            call,
            ids: state.ids,
            managers: state.managers,
            source: state.source,
            size: state.size,
            bounds: state.prop.bounding_box(),
        });
    }
    fn base_record(&self, call: Call) {
        self.seen.borrow_mut().push(Seen {
            call,
            ids: [0; 2],
            managers: [None; 2],
            source: None,
            size: 0.0,
            bounds: ([0.0; 3], [0.0; 3]),
        });
    }
    fn calls(&self) -> Vec<Call> {
        self.seen.borrow().iter().map(|s| s.call).collect()
    }
    fn clear(&mut self) {
        self.seen.borrow_mut().clear();
        self.add_index = 0;
    }
}
impl Host for Raw {
    fn effect(&mut self, s: &State, e: Effect) {
        self.record(s, Call::Effect(e));
    }
    fn create_source(&mut self, s: &State, class: u32) -> Option<Handle> {
        assert_eq!(class, 0xCC1F_D62C);
        self.record(s, Call::Factory);
        self.factory
    }
    fn dimensions(&mut self, s: &State, _: Handle) -> (u32, u32) {
        self.record(s, Call::Dimensions);
        self.dimensions
    }
    fn altitude(&mut self, s: &State, _: [f32; 2]) -> f64 {
        self.record(s, Call::Altitude);
        self.altitude
    }
    fn level(&mut self, s: &State) -> u32 {
        self.record(s, Call::Level);
        self.level
    }
    fn manager(&mut self, s: &State, i: u32) -> Option<Handle> {
        self.record(s, Call::Lookup(i));
        self.managers[i as usize]
    }
    fn add(&mut self, s: &State, m: Handle, q: Handle, c: [f32; 2], size: f32, turns: f32) -> u32 {
        self.record(s, Call::Add(m, q, c, size, turns));
        let id = self.adds[self.add_index % 2];
        self.add_index += 1;
        id
    }
    fn terrain_position(&mut self, s: &State, c: [f32; 2]) {
        self.record(s, Call::Terrain(c));
    }
    fn initialized(&mut self, s: &State) -> bool {
        self.record(s, Call::Initialized);
        s.base.status & 1 != 0
    }
    fn base_init(&mut self, b: &mut OccupantBase, _: &PropOccupant) {
        self.base_record(Call::BaseInit);
        b.status |= 1;
    }
    fn base_shutdown(&mut self, b: &mut OccupantBase, _: &PropOccupant) {
        self.base_record(Call::BaseShutdown);
        b.status &= !1;
    }
    fn base_flags(&mut self, b: &mut OccupantBase, _: &PropOccupant, flags: u32) {
        self.base_record(Call::BaseFlags(flags));
        b.flags = flags as u8;
    }
    fn base_position(&mut self, p: &PropOccupant, v: [f32; 3]) {
        self.base_record(Call::BasePosition(v));
        // Typed raw base primitive, deliberately distinguishable from owner box.
        p.set_bounding_box([9.0, 97.0, -10.0], [13.0, 101.0, -4.0]);
    }
    fn base_message(&mut self, s: &State, message: u32) -> u32 {
        self.record(s, Call::Message(message));
        0x1234_0081
    }
    fn temporary_filter(&mut self, s: &State) -> Option<Handle> {
        self.record(s, Call::TempFilter);
        self.filter
    }
    fn filter_accepts(&mut self, s: &State, f: Handle) -> bool {
        self.record(s, Call::Filter(f));
        self.accepts
    }
    fn city_config(&mut self, s: &State) -> bool {
        self.record(s, Call::Config);
        self.config
    }
}
fn owner() -> Owner<Raw> {
    let host = Raw {
        seen: Rc::default(),
        dimensions: (6, 4),
        altitude: 1.5,
        level: 2,
        managers: [Some(Handle(10)), Some(Handle(20)), Some(Handle(30)), Some(Handle(40))],
        adds: [17, 29],
        add_index: 0,
        factory: None,
        filter: None,
        accepts: true,
        config: true,
    };
    Owner::new(
        host,
        PropOccupant::default(),
        OccupantBase::default(),
        Arithmetic::Nearest24Normal,
    )
}
fn g1() -> Owner<Raw> {
    let mut o = owner();
    o.base.flags = 2;
    o.source = Some(Handle(100));
    o.managers = [Some(Handle(10)), Some(Handle(20))];
    o.center = [11.0, -7.0];
    o.height = 0.5;
    o.rgb = [2.0, 3.0, 5.0];
    o.alpha = 0.25;
    o
}
fn e(e: Effect) -> Call {
    Call::Effect(e)
}

#[test]
fn constructor_and_g1_literal_order_and_before_callback_state() {
    let fresh = owner();
    assert_eq!(
        (fresh.height, fresh.size, fresh.orientation, fresh.dynamic),
        (1.0, 100.0, 1, 0)
    );
    assert_eq!(fresh.ids, [ABSENT; 2]);
    assert_eq!(fresh.managers, [None; 2]);
    assert_eq!(fresh.source, None);
    assert_eq!(fresh.rgb, [255.0; 3]);
    assert_eq!(fresh.prop.bounding_box().1.map(f32::to_bits), [0x3DCC_CCCD; 3]);
    let mut o = g1();
    let (s, h) = o.parts();
    s.rebuild(h).unwrap();
    assert_eq!(o.size.to_bits(), 0x4140_0000);
    assert_eq!(
        o.prop.bounding_box().0.map(f32::to_bits),
        [0x40A0_0000, 0x3FB3_3333, 0xC130_0000]
    );
    assert_eq!(
        o.prop.bounding_box().1.map(f32::to_bits),
        [0x4188_0000, 0x3FCC_CCCD, 0xC040_0000]
    );
    assert_eq!(
        o.host.calls(),
        vec![
            Call::Dimensions,
            Call::Altitude,
            Call::Level,
            Call::Add(Handle(10), Handle(100), [11.0, -7.0], 12.0, 0.5),
            e(Effect::Color(Handle(10), 17, [2.0, 3.0, 5.0])),
            e(Effect::Alpha(Handle(10), 17, 0.25)),
            Call::Add(Handle(20), Handle(100), [11.0, -7.0], 12.0, 0.5),
            e(Effect::Color(Handle(20), 29, [2.0, 3.0, 5.0])),
            e(Effect::Alpha(Handle(20), 29, 0.25)),
            e(Effect::NotifyMove)
        ]
    );
    let seen = o.host.seen.borrow();
    assert_eq!(seen[3].ids, [ABSENT; 2]);
    assert_eq!(seen[3].size, 12.0);
    assert_eq!(seen[3].bounds, o.prop.bounding_box());
    assert_eq!(seen[4].ids, [17, ABSENT]);
}

#[test]
fn g2_g3_swapped_extents_and_table() {
    let mut o = g1();
    o.height = 0.75;
    o.center = [-2.5, 3.25];
    o.orientation = 0;
    o.ids = [17, 29];
    o.host.dimensions = (5, 8);
    o.host.altitude = -2.0;
    o.host.level = 3;
    o.host.adds = [41, 43];
    let (s, h) = o.parts();
    s.rebuild(h).unwrap();
    assert_eq!(o.size.to_bits(), 0x40F0_0000);
    assert_eq!(o.prop.bounding_box().0, [-8.5, f32::from_bits(0xC006_6666), -0.5]);
    assert_eq!(o.prop.bounding_box().1, [3.5, f32::from_bits(0xBFF3_3333), 7.0]);
    let seen = o.host.seen.borrow();
    assert_eq!(seen[0].ids, [17, 29]);
    assert_eq!(seen[1].ids, [ABSENT, 29]);
    assert_eq!(
        seen[5].call,
        Call::Add(Handle(10), Handle(100), [-2.5, 3.25], 7.5, 0.25)
    );
    drop(seen);
    for (orientation, level, turns) in [(2, 0, 0.125), (3, 1, 0.875)] {
        let mut p = g1();
        p.orientation = orientation;
        p.host.level = level;
        let (s, h) = p.parts();
        s.rebuild(h).unwrap();
        assert_eq!(
            p.host.calls()[3],
            Call::Add(Handle(10), Handle(100), [11.0, -7.0], 12.0, turns)
        );
        assert_eq!(p.prop.bounding_box().0[0], 5.0);
    }
}

#[test]
fn g4_early_exits_preserve_cache_and_remove_first() {
    for absent_source in [true, false] {
        let mut o = g1();
        o.ids = [17, 29];
        o.size = 100.0;
        let saved = o.prop.bounding_box();
        if absent_source {
            o.source = None;
        } else {
            o.base.flags = 0;
        }
        let (s, h) = o.parts();
        s.rebuild(h).unwrap();
        assert_eq!(
            o.host.calls(),
            [e(Effect::Remove(Handle(10), 17)), e(Effect::Remove(Handle(20), 29))]
        );
        assert_eq!(o.ids, [ABSENT; 2]);
        assert_eq!(o.size, 100.0);
        assert_eq!(o.prop.bounding_box(), saved);
    }
}

#[test]
fn r1_r2_routing_b_before_a_publish_before_release() {
    let mut o = g1();
    o.ids = [17, 29];
    let (s, h) = o.parts();
    s.set_dynamic(h, 1).unwrap();
    assert_eq!(
        &o.host.calls()[..8],
        &[
            e(Effect::Remove(Handle(10), 17)),
            e(Effect::Remove(Handle(20), 29)),
            Call::Lookup(3),
            e(Effect::AddRef(Handle(40))),
            e(Effect::Release(Handle(20))),
            Call::Lookup(2),
            e(Effect::AddRef(Handle(30))),
            e(Effect::Release(Handle(10)))
        ]
    );
    assert_eq!(o.host.seen.borrow()[3].managers[1], Some(Handle(20)));
    assert_eq!(o.host.seen.borrow()[4].managers[1], Some(Handle(40)));
    o.host.clear();
    let (s, h) = o.parts();
    s.set_dynamic(h, 1).unwrap();
    assert!(o.host.calls().is_empty());
    let (s, h) = o.parts();
    s.set_dynamic(h, 2).unwrap();
    assert_eq!(
        &o.host.calls()[..4],
        &[
            e(Effect::Remove(Handle(30), 17)),
            e(Effect::Remove(Handle(40), 29)),
            Call::Lookup(3),
            Call::Lookup(2)
        ]
    );
    assert_eq!(o.dynamic, 2);
}

#[test]
fn r3_source_failure_and_equal_text_do_not_rollback() {
    let mut o = g1();
    o.ids = [17, 29];
    let (s, h) = o.parts();
    s.set_text(h, b"new").unwrap();
    assert_eq!(o.host.calls(), [e(Effect::Release(Handle(100))), Call::Factory]);
    assert_eq!(o.host.seen.borrow()[0].source, None);
    assert_eq!(o.ids, [17, 29]);
    assert_eq!(o.source, None);
    assert_eq!(o.text, b"new");
    o.host.clear();
    o.host.factory = Some(Handle(101));
    let (s, h) = o.parts();
    s.set_text(h, b"new").unwrap();
    assert_eq!(
        &o.host.calls()[..3],
        &[
            Call::Factory,
            e(Effect::Remove(Handle(10), 17)),
            e(Effect::Remove(Handle(20), 29))
        ]
    );
    assert!(!o.host.calls().contains(&e(Effect::AddRef(Handle(101)))));
}

#[test]
fn r4_highbit_and_partial_add_sentinels() {
    for adds in [[ABSENT, 29], [17, ABSENT], [ABSENT; 2]] {
        let mut o = g1();
        o.ids = [0x8000_0005, ABSENT];
        o.host.adds = adds;
        let (s, h) = o.parts();
        s.rebuild(h).unwrap();
        assert_eq!(o.host.calls()[0], e(Effect::Remove(Handle(10), 0x8000_0005)));
        assert_eq!(o.ids, adds);
        assert_eq!(o.host.calls().last(), Some(&e(Effect::NotifyMove)));
        assert_eq!(o.host.calls().iter().filter(|c| matches!(c, Call::Add(..))).count(), 2);
        assert_eq!(
            o.host
                .calls()
                .iter()
                .filter(|c| matches!(c, Call::Effect(Effect::Color(..))))
                .count(),
            adds.iter().filter(|&&id| id != ABSENT).count()
        );
    }
}

#[test]
fn r5_null_routing_and_invalid_manager_domain() {
    let mut o = g1();
    o.source = None;
    o.host.managers = [None; 4];
    let (s, h) = o.parts();
    s.set_dynamic(h, 1).unwrap();
    assert_eq!(
        o.host.calls(),
        [
            Call::Lookup(3),
            e(Effect::Release(Handle(20))),
            Call::Lookup(2),
            e(Effect::Release(Handle(10)))
        ]
    );
    assert_eq!(o.managers, [None; 2]);
    o.ids[0] = 17;
    let (s, h) = o.parts();
    assert_eq!(s.remove(h), Err(Unsupported::MissingManager));
}

#[test]
fn numeric_and_table_unsupported_are_not_success() {
    for (dims, orient, policy, error) in [
        ((0, 4), 1, Arithmetic::Nearest24Normal, Unsupported::NumericDomain),
        ((6, 4), 4, Arithmetic::Nearest24Normal, Unsupported::TableAccess),
        ((6, 4), 1, Arithmetic::Unsupported, Unsupported::ArithmeticPolicy),
    ] {
        let mut o = g1();
        o.host.dimensions = dims;
        o.orientation = orient;
        o.arithmetic = policy;
        let (s, h) = o.parts();
        assert_eq!(s.rebuild(h), Err(error));
        assert!(!o.host.calls().iter().any(|c| matches!(c, Call::Add(..))));
    }
}

#[test]
fn setters_literal_boundaries_nan_epsilon_zero_and_raw_byte() {
    for value in [0.0, -0.0, -1.0, 8.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        let mut o = g1();
        let (s, h) = o.parts();
        s.set_height(h, value).unwrap();
        assert_eq!(o.height, 0.5);
        assert!(o.host.calls().is_empty());
    }
    for value in [0.5, f32::from_bits(1), f32::from_bits(0x40FF_FFFF)] {
        let mut o = g1();
        o.source = None;
        o.ids = [17, 29];
        let (s, h) = o.parts();
        s.set_height(h, value).unwrap();
        assert_eq!(o.height.to_bits(), value.to_bits());
        assert_eq!(o.ids, [ABSENT; 2]);
    }
    for (bits, changed) in [
        (0x38D1_B716, false),
        (0x38D1_B717, true),
        (0x38D1_B718, true),
        (0x7FC0_0001, true),
    ] {
        let mut o = g1();
        o.dynamic = 1;
        o.ids = [17, 29];
        o.rgb = [0.0; 3];
        let value = [f32::from_bits(bits), 0.0, 0.0];
        let (s, h) = o.parts();
        s.set_color(h, value).unwrap();
        assert_eq!(o.host.calls().len(), if changed { 2 } else { 0 });
        assert_eq!(o.rgb[0].to_bits(), if changed { bits } else { 0 });
    }
    let mut o = g1();
    o.dynamic = 1;
    o.ids = [ABSENT, 29];
    o.alpha = 0.0;
    let (s, h) = o.parts();
    s.set_alpha(h, -0.0).unwrap();
    assert_eq!(o.alpha.to_bits(), 0);
    assert!(o.host.calls().is_empty());
    let nan = f32::from_bits(0x7FC0_0001);
    let (s, h) = o.parts();
    s.set_alpha(h, nan).unwrap();
    assert_eq!(o.alpha.to_bits(), 0x7FC0_0001);
    assert!(matches!(o.host.calls()[0], Call::Effect(Effect::Alpha(Handle(20), 29, a)) if a.to_bits() == 0x7FC0_0001));
    o.host.clear();
    let (s, h) = o.parts();
    s.set_orientation(h, 1).unwrap();
    assert!(o.host.calls().is_empty());
    let (s, h) = o.parts();
    assert_eq!(s.set_orientation(h, 0x102), Err(Unsupported::TableAccess));
    assert_eq!(o.orientation, 0x102);
}

#[test]
fn p1_position_dynamic_retains_raw_base_translation_and_repeats() {
    let mut o = g1();
    o.dynamic = 1;
    o.ids = [17, 29];
    for _ in 0..2 {
        let (s, h) = o.parts();
        s.set_position(h, [11.0, 99.0, -7.0]).unwrap();
    }
    let prefix = [
        Call::BasePosition([11.0, 99.0, -7.0]),
        Call::Terrain([11.0, -7.0]),
        e(Effect::Position(Handle(10), 17, [11.0, -7.0])),
        e(Effect::Position(Handle(20), 29, [11.0, -7.0])),
    ];
    assert_eq!(o.host.calls(), prefix.repeat(2));
    assert_eq!(o.size, 100.0);
    assert_eq!(o.prop.bounding_box(), ([9.0, 97.0, -10.0], [13.0, 101.0, -4.0]));
    o.dynamic = 0;
    o.host.clear();
    let (s, h) = o.parts();
    s.set_position(h, [11.0, 99.0, -7.0]).unwrap();
    assert_eq!(&o.host.calls()[..2], &prefix[..2]);
    assert_eq!(o.prop.bounding_box().0[0], 5.0);
    assert_eq!(o.prop.bounding_box().0[1].to_bits(), 0x3FB3_3333);
}

#[test]
fn r6_shutdown_order_state_repeat_and_destruction_are_distinct() {
    let mut o = g1();
    o.base.status = 1;
    o.ids = [ABSENT, 29];
    let trace = o.host.seen.clone();
    let (s, h) = o.parts();
    assert_eq!(s.shutdown(h), Ok(true));
    assert_eq!(
        o.host.calls(),
        [
            Call::Initialized,
            e(Effect::Unsubscribe(0xA6B7_9602)),
            e(Effect::Unsubscribe(0x06CD_B65B)),
            e(Effect::Unsubscribe(0x26D3_1EC1)),
            e(Effect::Release(Handle(10))),
            e(Effect::Remove(Handle(20), 29)),
            e(Effect::Release(Handle(20))),
            e(Effect::Release(Handle(100))),
            Call::BaseShutdown
        ]
    );
    assert_eq!(trace.borrow()[4].managers[0], None);
    assert_eq!(trace.borrow()[5].ids[1], 29);
    assert_eq!(trace.borrow()[6].ids[1], ABSENT);
    assert_eq!(trace.borrow()[6].managers[1], None);
    assert_eq!(trace.borrow()[7].source, None);
    o.host.clear();
    let (s, h) = o.parts();
    s.shutdown(h).unwrap();
    assert_eq!(o.host.calls(), [Call::Initialized]);
    trace.borrow_mut().clear();
    drop(o);
    assert_eq!(
        trace.borrow().iter().map(|s| s.call).collect::<Vec<_>>(),
        [e(Effect::DestroyText), e(Effect::DestroyBase)]
    );

    let mut o = g1();
    o.ids = [17, 29];
    let trace = o.host.seen.clone();
    drop(o);
    assert_eq!(
        trace.borrow().iter().map(|s| s.call).collect::<Vec<_>>(),
        [
            e(Effect::Release(Handle(20))),
            e(Effect::Release(Handle(10))),
            e(Effect::Release(Handle(100))),
            e(Effect::DestroyText),
            e(Effect::DestroyBase)
        ]
    );
    assert_eq!(trace.borrow()[0].managers[1], Some(Handle(20)));
}

#[test]
fn shutdown_null_manager_keeps_id() {
    let mut o = g1();
    o.base.status = 1;
    o.managers[0] = None;
    o.ids = [17, ABSENT];
    let (s, h) = o.parts();
    s.shutdown(h).unwrap();
    assert_eq!(o.ids[0], 17);
}

#[test]
fn init_loaded_guard_filter_and_subscription_order() {
    let mut o = owner();
    o.base.status = 2;
    let (s, h) = o.parts();
    s.init(h).unwrap();
    assert_eq!(
        o.host.calls(),
        [
            Call::Initialized,
            e(Effect::SetPropType(0xCB79_919B)),
            Call::BaseInit,
            e(Effect::Subscribe(0xA6B7_9602)),
            e(Effect::Subscribe(0x06CD_B65B)),
            e(Effect::Subscribe(0x26D3_1EC1))
        ]
    );
    o.host.clear();
    let (s, h) = o.parts();
    s.init(h).unwrap();
    assert_eq!(o.host.calls(), [Call::Initialized]);
    let mut o = owner();
    o.host.filter = Some(Handle(200));
    let (s, h) = o.parts();
    s.init(h).unwrap();
    assert_eq!(
        o.host.calls(),
        [
            Call::Initialized,
            e(Effect::SetPropType(0xCB79_919B)),
            Call::BaseInit,
            Call::Lookup(1),
            e(Effect::AddRef(Handle(20))),
            Call::Lookup(0),
            e(Effect::AddRef(Handle(10))),
            Call::TempFilter,
            Call::Filter(Handle(200)),
            Call::Config,
            Call::Config,
            Call::BaseFlags(2),
            e(Effect::Release(Handle(200))),
            e(Effect::Subscribe(0xA6B7_9602)),
            e(Effect::Subscribe(0x06CD_B65B)),
            e(Effect::Subscribe(0x26D3_1EC1))
        ]
    );
    let (s, h) = o.parts();
    s.shutdown(h).unwrap();
}

#[test]
fn flags_config_veto_and_routing_before_source_factory() {
    let mut o = owner();
    o.host.config = false;
    let (s, h) = o.parts();
    s.set_flags(h, 0x102).unwrap();
    assert_eq!(o.host.calls(), [Call::Config, Call::BaseFlags(0x100)]);
    assert_eq!(o.base.flags & 2, 0);
    o.host.clear();
    o.host.config = true;
    o.text = b"new".to_vec();
    o.host.factory = Some(Handle(100));
    let (s, h) = o.parts();
    s.set_flags(h, 2).unwrap();
    assert_eq!(
        &o.host.calls()[..7],
        &[
            Call::Config,
            Call::BaseFlags(2),
            Call::Lookup(1),
            e(Effect::AddRef(Handle(20))),
            Call::Lookup(0),
            e(Effect::AddRef(Handle(10))),
            Call::Factory
        ]
    );
    assert_eq!(o.ids, [17, 29]);
    o.host.clear();
    let (s, h) = o.parts();
    s.set_flags(h, 0).unwrap();
    assert_eq!(
        o.host.calls(),
        [
            Call::BaseFlags(0),
            e(Effect::Remove(Handle(10), 17)),
            e(Effect::Remove(Handle(20), 29))
        ]
    );
}

#[test]
fn messages_borrowed_filter_no_release_and_base_result_passthrough() {
    let mut o = g1();
    o.host.accepts = false;
    let (s, h) = o.parts();
    assert_eq!(s.message(h, 0x06CD_B65B, Some(Handle(200))), Ok(0x1234_0081));
    assert_eq!(
        o.host.calls(),
        [
            Call::Filter(Handle(200)),
            Call::BaseFlags(0),
            Call::Message(0x06CD_B65B)
        ]
    );
    assert_eq!(o.base.flags & 2, 0);
    o.host.clear();
    let (s, h) = o.parts();
    s.message(h, 0x06CD_B65B, Some(Handle(200))).unwrap();
    assert_eq!(o.host.calls(), [Call::Filter(Handle(200)), Call::Message(0x06CD_B65B)]);
    o.host.clear();
    o.host.filter = Some(Handle(200));
    let (s, h) = o.parts();
    s.message(h, 0x26D3_1EC1, None).unwrap();
    assert_eq!(
        o.host.calls(),
        [
            Call::Lookup(1),
            Call::Lookup(0),
            Call::TempFilter,
            Call::Filter(Handle(200)),
            e(Effect::Release(Handle(200))),
            Call::Message(0x26D3_1EC1)
        ]
    );
    o.host.clear();
    let (s, h) = o.parts();
    s.message(h, 0xDEAD, None).unwrap();
    assert_eq!(o.host.calls(), [Call::Message(0xDEAD)]);
}

// Source-backed S1 extension, verbatim §4 fixture bytes. No generated expected
// values, native layout emulation, or owner implementation used as an oracle.
const S1: &str =
    "0100000041 00003041 0000E0C0 0000003F 00004041 00003041 0000E0C0 00000040 00004040 0000A040 0000803E 02";
fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .flat_map(|word| {
            (0..word.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&word[i..i + 2], 16).unwrap())
                .collect::<Vec<_>>()
        })
        .collect()
}
struct RawBase(bool);
impl BaseCodec for RawBase {
    fn write(&mut self, _: &OccupantBase, _: &PropOccupant, _: &mut dyn OutStream) -> bool {
        self.0
    }
    fn read(&mut self, _: &mut OccupantBase, _: &PropOccupant, _: &mut dyn InStream) -> Result<bool> {
        Ok(self.0)
    }
}

#[test]
fn s1_literal_write_read_second_position_wins_and_foreign_fields_untouched() {
    use crate::stream::{ByteReader, ByteWriter};
    let mut o = g1();
    o.text = b"A".to_vec();
    o.orientation = 0x102;
    o.dynamic = 1;
    o.ids = [17, 29];
    o.size = 12.0;
    let mut out = ByteWriter::new();
    assert!(o.write(&mut RawBase(true), &mut out));
    assert_eq!(out.bytes(), [vec![3, 0], hex(S1)].concat());
    // Actual production prop codec is reused, not a second private base model.
    let mut actual = ByteWriter::new();
    assert!(o.write(&mut PropCodec, &mut actual));
    assert_eq!(&actual.bytes()[actual.len() - 46..], hex(S1));
    let mut receiver = g1();
    let mut input = ByteReader::new(actual.bytes());
    assert_eq!(receiver.read(&mut PropCodec, &mut input), Ok(true));
    assert_eq!(receiver.orientation, 2);
    let mut bytes = [vec![3, 0], hex(S1)].concat();
    bytes[7..15].copy_from_slice(&hex("0000803F 00000040"));
    let mut input = ByteReader::new(&bytes);
    let (s, _) = o.parts();
    assert_eq!(s.read(&mut RawBase(true), &mut input), Ok(true));
    assert_eq!(o.center, [11.0, -7.0]);
    assert_eq!(o.ids, [17, 29]);
    assert_eq!(o.dynamic, 1);
    assert_eq!(o.orientation, 2);
    assert_eq!(o.source, Some(Handle(100)));
    assert_eq!(o.managers, [Some(Handle(10)), Some(Handle(20))]);
    assert!(o.host.calls().is_empty());
}

#[test]
fn s2_legacy_unsigned_height_and_extra_word_vs_v3_raw_bits_and_future_version() {
    use crate::stream::ByteReader;
    for (version, heightword, expected) in [
        (1u16, "01000000", 0x3F80_0000),
        (1, "FFFFFFFF", 0x4F80_0000),
        (2, "01000000", 1),
        (3, "01000000", 1),
        (4, "01000000", 1),
    ] {
        let mut extension = hex(S1);
        extension[13..17].copy_from_slice(&hex(heightword));
        if version < 3 {
            extension.splice(5..5, hex("78563412"));
        }
        let bytes = [version.to_le_bytes().to_vec(), extension].concat();
        let mut o = g1();
        let mut input = ByteReader::new(&bytes);
        assert_eq!(o.read(&mut RawBase(true), &mut input), Ok(true));
        assert_eq!(o.height.to_bits(), expected);
        assert_eq!(o.center, [11.0, -7.0]);
        assert_eq!(input.remaining(), 0);
        assert!(o.host.calls().is_empty());
    }
}

// Only error/status primitives differ from the production byte streams.
struct StatusIn<'a>(crate::stream::ByteReader<'a>, i32);
impl InStream for StatusIn<'_> {
    fn get_u8(&mut self) -> Option<u8> {
        self.0.get_u8()
    }
    fn get_u16(&mut self) -> Option<u16> {
        self.0.get_u16()
    }
    fn get_u32(&mut self) -> Option<u32> {
        self.0.get_u32()
    }
    fn get_u64(&mut self) -> Option<u64> {
        self.0.get_u64()
    }
    fn get_f32(&mut self) -> Option<f32> {
        self.0.get_f32()
    }
    fn get_f64(&mut self) -> Option<f64> {
        self.0.get_f64()
    }
    fn get_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        self.0.get_bytes(n)
    }
    fn get_str(&mut self) -> Option<Vec<u8>> {
        self.0.get_str()
    }
    fn get_ref(&mut self) -> Option<crate::stream::RefValue> {
        self.0.get_ref()
    }
    fn skip(&mut self, n: usize) -> bool {
        self.0.skip(n)
    }
    fn error(&self) -> i32 {
        self.1
    }
}
struct StatusOut(crate::stream::ByteWriter, i32);
impl OutStream for StatusOut {
    fn put_u8(&mut self, v: u8) -> bool {
        self.0.put_u8(v)
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.0.put_u16(v)
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.0.put_u32(v)
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.0.put_u64(v)
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.0.put_f32(v)
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.0.put_f64(v)
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.0.put_bytes(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.0.put_str(v)
    }
    fn put_ref(&mut self, v: &crate::stream::RefValue) -> bool {
        self.0.put_ref(v)
    }
    fn error(&self) -> i32 {
        self.1
    }
}
#[test]
fn s3_lowbyte_error_and_base_false_skips_suffix_not_final_success() {
    use crate::stream::{ByteReader, ByteWriter};
    for base in [false, true] {
        for (error, expected) in [(0x1234_0000, true), (0x1234_0001, false)] {
            let bytes = [vec![3, 0], hex(S1)].concat();
            let mut o = g1();
            let mut input = StatusIn(ByteReader::new(&bytes), error);
            assert_eq!(o.read(&mut RawBase(base), &mut input), Ok(expected));
            assert_eq!(input.0.position(), if base { bytes.len() } else { 2 });
            let mut out = StatusOut(ByteWriter::new(), error);
            assert_eq!(o.write(&mut RawBase(base), &mut out), expected);
            if !base {
                assert_eq!(out.0.bytes(), [3, 0]);
            }
        }
    }
    let mut o = g1();
    let mut input = ByteReader::new(&[3, 0, 1, 0, 0, 0]);
    assert_eq!(
        o.read(&mut RawBase(true), &mut input),
        Err(Unsupported::TruncatedRecord)
    );
}
