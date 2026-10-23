use super::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
enum Call {
    Effect(Effect), Factory, Dimensions, Altitude, Level, Lookup(u32),
    Add(Handle, Handle, [f32; 2], f32, f32), Terrain([f32; 2]),
    BasePosition([f32; 3]), BaseInit, BaseShutdown, BaseFlags(u32),
    Initialized, TempFilter, Filter(Handle), Config, Message(u32),
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
        self.seen.borrow_mut().push(Seen { call, ids: state.ids, managers: state.managers,
            source: state.source, size: state.size, bounds: state.prop.bounding_box() });
    }
    fn base_record(&self, call: Call) {
        self.seen.borrow_mut().push(Seen { call, ids: [0; 2], managers: [None; 2],
            source: None, size: 0.0, bounds: ([0.0; 3], [0.0; 3]) });
    }
    fn calls(&self) -> Vec<Call> { self.seen.borrow().iter().map(|s| s.call.clone()).collect() }
    fn clear(&mut self) { self.seen.borrow_mut().clear(); self.add_index = 0; }
}
impl Host for Raw {
    fn effect(&mut self, s: &State, e: Effect) { self.record(s, Call::Effect(e)); }
    fn create_source(&mut self, s: &State, class: u32) -> Option<Handle> {
        assert_eq!(class, 0xCC1F_D62C); self.record(s, Call::Factory); self.factory
    }
    fn dimensions(&mut self, s: &State, _: Handle) -> (u32, u32) {
        self.record(s, Call::Dimensions); self.dimensions
    }
    fn altitude(&mut self, s: &State, _: [f32; 2]) -> f64 {
        self.record(s, Call::Altitude); self.altitude
    }
    fn level(&mut self, s: &State) -> u32 { self.record(s, Call::Level); self.level }
    fn manager(&mut self, s: &State, i: u32) -> Option<Handle> {
        self.record(s, Call::Lookup(i)); self.managers[i as usize]
    }
    fn add(&mut self, s: &State, m: Handle, q: Handle, c: [f32; 2], size: f32, turns: f32) -> u32 {
        self.record(s, Call::Add(m, q, c, size, turns));
        let id = self.adds[self.add_index % 2]; self.add_index += 1; id
    }
    fn terrain_position(&mut self, s: &State, c: [f32; 2]) { self.record(s, Call::Terrain(c)); }
    fn initialized(&mut self, s: &State) -> bool {
        self.record(s, Call::Initialized); s.base.status & 1 != 0
    }
    fn base_init(&mut self, b: &mut OccupantBase, _: &PropOccupant) {
        self.base_record(Call::BaseInit); b.status |= 1;
    }
    fn base_shutdown(&mut self, b: &mut OccupantBase, _: &PropOccupant) {
        self.base_record(Call::BaseShutdown); b.status &= !1;
    }
    fn base_flags(&mut self, b: &mut OccupantBase, _: &PropOccupant, flags: u32) {
        self.base_record(Call::BaseFlags(flags)); b.flags = flags as u8;
    }
    fn base_position(&mut self, p: &PropOccupant, v: [f32; 3]) {
        self.base_record(Call::BasePosition(v));
        // Typed raw base primitive, deliberately distinguishable from owner box.
        p.set_bounding_box([9.0, 97.0, -10.0], [13.0, 101.0, -4.0]);
    }
    fn base_message(&mut self, s: &State, message: u32) -> u32 {
        self.record(s, Call::Message(message)); 0x1234_0081
    }
    fn temporary_filter(&mut self, s: &State) -> Option<Handle> {
        self.record(s, Call::TempFilter); self.filter
    }
    fn filter_accepts(&mut self, s: &State, f: Handle) -> bool {
        self.record(s, Call::Filter(f)); self.accepts
    }
    fn city_config(&mut self, s: &State) -> bool { self.record(s, Call::Config); self.config }
}
fn owner() -> Owner<Raw> {
    let host = Raw { seen: Rc::default(), dimensions: (6, 4), altitude: 1.5, level: 2,
        managers: [Some(Handle(10)), Some(Handle(20)), Some(Handle(30)), Some(Handle(40))],
        adds: [17, 29], add_index: 0, factory: None, filter: None, accepts: true, config: true };
    Owner::new(host, PropOccupant::default(), OccupantBase::default(), Arithmetic::Nearest24Normal)
}
fn g1() -> Owner<Raw> {
    let mut o = owner();
    o.base.flags = 2; o.source = Some(Handle(100)); o.managers = [Some(Handle(10)), Some(Handle(20))];
    o.center = [11.0, -7.0]; o.height = 0.5; o.rgb = [2.0, 3.0, 5.0]; o.alpha = 0.25;
    o
}
fn e(e: Effect) -> Call { Call::Effect(e) }

#[test]
fn constructor_and_g1_literal_order_and_before_callback_state() {
    let fresh = owner();
    assert_eq!((fresh.height, fresh.size, fresh.orientation, fresh.dynamic), (1.0, 100.0, 1, 0));
    assert_eq!(fresh.ids, [ABSENT; 2]); assert_eq!(fresh.managers, [None; 2]);
    assert_eq!(fresh.source, None); assert_eq!(fresh.rgb, [255.0; 3]);
    assert_eq!(fresh.prop.bounding_box().1.map(f32::to_bits), [0x3DCC_CCCD; 3]);
    let mut o = g1(); let (s, h) = o.parts(); s.rebuild(h).unwrap();
    assert_eq!(o.size.to_bits(), 0x4140_0000);
    assert_eq!(o.prop.bounding_box().0.map(f32::to_bits), [0x40A0_0000, 0x3FB3_3333, 0xC130_0000]);
    assert_eq!(o.prop.bounding_box().1.map(f32::to_bits), [0x4188_0000, 0x3FCC_CCCD, 0xC040_0000]);
    assert_eq!(o.host.calls(), vec![Call::Dimensions, Call::Altitude, Call::Level,
        Call::Add(Handle(10), Handle(100), [11.0, -7.0], 12.0, 0.5),
        e(Effect::Color(Handle(10), 17, [2.0, 3.0, 5.0])), e(Effect::Alpha(Handle(10), 17, 0.25)),
        Call::Add(Handle(20), Handle(100), [11.0, -7.0], 12.0, 0.5),
        e(Effect::Color(Handle(20), 29, [2.0, 3.0, 5.0])), e(Effect::Alpha(Handle(20), 29, 0.25)),
        e(Effect::NotifyMove)]);
    let seen = o.host.seen.borrow();
    assert_eq!(seen[3].ids, [ABSENT; 2]); assert_eq!(seen[3].size, 12.0);
    assert_eq!(seen[3].bounds, o.prop.bounding_box()); assert_eq!(seen[4].ids, [17, ABSENT]);
}

#[test]
fn g2_g3_swapped_extents_and_table() {
    let mut o = g1(); o.height = 0.75; o.center = [-2.5, 3.25]; o.orientation = 0;
    o.ids = [17, 29]; o.host.dimensions = (5, 8); o.host.altitude = -2.0; o.host.level = 3;
    o.host.adds = [41, 43]; let (s, h) = o.parts(); s.rebuild(h).unwrap();
    assert_eq!(o.size.to_bits(), 0x40F0_0000);
    assert_eq!(o.prop.bounding_box().0, [-8.5, f32::from_bits(0xC006_6666), -0.5]);
    assert_eq!(o.prop.bounding_box().1, [3.5, f32::from_bits(0xBFF3_3333), 7.0]);
    let seen = o.host.seen.borrow();
    assert_eq!(seen[0].ids, [17, 29]); assert_eq!(seen[1].ids, [ABSENT, 29]);
    assert_eq!(seen[5].call, Call::Add(Handle(10), Handle(100), [-2.5, 3.25], 7.5, 0.25));
    drop(seen);
    for (orientation, level, turns) in [(2, 0, 0.125), (3, 1, 0.875)] {
        let mut p = g1(); p.orientation = orientation; p.host.level = level;
        let (s, h) = p.parts(); s.rebuild(h).unwrap();
        assert_eq!(p.host.calls()[3], Call::Add(Handle(10), Handle(100), [11.0, -7.0], 12.0, turns));
        assert_eq!(p.prop.bounding_box().0[0], 5.0);
    }
}

#[test]
fn g4_early_exits_preserve_cache_and_remove_first() {
    for absent_source in [true, false] {
        let mut o = g1(); o.ids = [17, 29]; o.size = 100.0;
        let saved = o.prop.bounding_box();
        if absent_source { o.source = None; } else { o.base.flags = 0; }
        let (s, h) = o.parts(); s.rebuild(h).unwrap();
        assert_eq!(o.host.calls(), [e(Effect::Remove(Handle(10), 17)), e(Effect::Remove(Handle(20), 29))]);
        assert_eq!(o.ids, [ABSENT; 2]); assert_eq!(o.size, 100.0); assert_eq!(o.prop.bounding_box(), saved);
    }
}

#[test]
fn r1_r2_routing_b_before_a_publish_before_release() {
    let mut o = g1(); o.ids = [17, 29]; let (s, h) = o.parts(); s.set_dynamic(h, 1).unwrap();
    assert_eq!(&o.host.calls()[..8], &[
        e(Effect::Remove(Handle(10), 17)), e(Effect::Remove(Handle(20), 29)), Call::Lookup(3),
        e(Effect::AddRef(Handle(40))), e(Effect::Release(Handle(20))), Call::Lookup(2),
        e(Effect::AddRef(Handle(30))), e(Effect::Release(Handle(10)))]);
    assert_eq!(o.host.seen.borrow()[3].managers[1], Some(Handle(20)));
    assert_eq!(o.host.seen.borrow()[4].managers[1], Some(Handle(40)));
    o.host.clear(); let (s, h) = o.parts(); s.set_dynamic(h, 1).unwrap(); assert!(o.host.calls().is_empty());
    let (s, h) = o.parts(); s.set_dynamic(h, 2).unwrap();
    assert_eq!(&o.host.calls()[..4], &[e(Effect::Remove(Handle(30), 17)),
        e(Effect::Remove(Handle(40), 29)), Call::Lookup(3), Call::Lookup(2)]);
    assert_eq!(o.dynamic, 2);
}

#[test]
fn r3_source_failure_and_equal_text_do_not_rollback() {
    let mut o = g1(); o.ids = [17, 29];
    let (s, h) = o.parts(); s.set_text(h, b"new").unwrap();
    assert_eq!(o.host.calls(), [e(Effect::Release(Handle(100))), Call::Factory]);
    assert_eq!(o.host.seen.borrow()[0].source, None);
    assert_eq!(o.ids, [17, 29]); assert_eq!(o.source, None); assert_eq!(o.text, b"new");
    o.host.clear(); o.host.factory = Some(Handle(101));
    let (s, h) = o.parts(); s.set_text(h, b"new").unwrap();
    assert_eq!(&o.host.calls()[..3], &[Call::Factory, e(Effect::Remove(Handle(10), 17)),
        e(Effect::Remove(Handle(20), 29))]);
    assert!(!o.host.calls().contains(&e(Effect::AddRef(Handle(101)))));
}

#[test]
fn r4_highbit_and_partial_add_sentinels() {
    for adds in [[ABSENT, 29], [17, ABSENT], [ABSENT; 2]] {
        let mut o = g1(); o.ids = [0x8000_0005, ABSENT]; o.host.adds = adds;
        let (s, h) = o.parts(); s.rebuild(h).unwrap();
        assert_eq!(o.host.calls()[0], e(Effect::Remove(Handle(10), 0x8000_0005)));
        assert_eq!(o.ids, adds); assert_eq!(o.host.calls().last(), Some(&e(Effect::NotifyMove)));
        assert_eq!(o.host.calls().iter().filter(|c| matches!(c, Call::Add(..))).count(), 2);
        assert_eq!(o.host.calls().iter().filter(|c| matches!(c, Call::Effect(Effect::Color(..)))).count(),
            adds.iter().filter(|&&id| id != ABSENT).count());
    }
}

#[test]
fn r5_null_routing_and_invalid_manager_domain() {
    let mut o = g1(); o.source = None; o.host.managers = [None; 4];
    let (s, h) = o.parts(); s.set_dynamic(h, 1).unwrap();
    assert_eq!(o.host.calls(), [Call::Lookup(3), e(Effect::Release(Handle(20))),
        Call::Lookup(2), e(Effect::Release(Handle(10)))]);
    assert_eq!(o.managers, [None; 2]);
    o.ids[0] = 17; let (s, h) = o.parts(); assert_eq!(s.remove(h), Err(Unsupported::MissingManager));
}

#[test]
fn numeric_and_table_unsupported_are_not_success() {
    for (dims, orient, policy, error) in [((0, 4), 1, Arithmetic::Nearest24Normal, Unsupported::NumericDomain),
        ((6, 4), 4, Arithmetic::Nearest24Normal, Unsupported::TableAccess),
        ((6, 4), 1, Arithmetic::Unsupported, Unsupported::ArithmeticPolicy)] {
        let mut o = g1(); o.host.dimensions = dims; o.orientation = orient; o.arithmetic = policy;
        let (s, h) = o.parts(); assert_eq!(s.rebuild(h), Err(error));
        assert!(!o.host.calls().iter().any(|c| matches!(c, Call::Add(..))));
    }
}
