// SPDX-License-Identifier: GPL-3.0-or-later
// Expected inputs/effects fixed from legacy-control §§1–10 before production code.
use super::*;
use crate::variant::{Elem, Variant};
use std::collections::{BTreeMap, VecDeque};

mod conformance;

#[derive(Debug, Clone, PartialEq)]
enum Event {
    Create(u32, u32),
    Query(Handle, u32),
    Add(Handle),
    Release(Handle),
    Word(Handle, String, u32),
    Read(Handle, Read),
    Write(Handle, Write),
    Widget(Handle, u32, Vec<Value>),
    FontId(String),
    Font(u32),
    Colour([u8; 3]),
    ColourFour([u8; 3], u32),
    GridColour(Handle, [u8; 3]),
    NotifyCell(Handle, i32, i32, CellDescriptor),
    NotifyAxis(Handle, Notifier, [i32; 3]),
    Image(u32, u32),
    StringResource(u32, u32),
    ImageRect(Handle),
    ManagerSlot3,
    NewString,
    Append(Handle, String),
    NewProcedure,
}

#[derive(Default)]
struct Fake {
    events: Vec<Event>,
    creates: VecDeque<Option<Handle>>,
    queries: BTreeMap<(Handle, u32), Handle>,
    reads: BTreeMap<(Handle, Read), Value>,
    raw: BTreeMap<(Handle, u32), Value>,
    release_counts: BTreeMap<Handle, u32>,
    raw_sequences: BTreeMap<(Handle, u32), VecDeque<Value>>,
    images: BTreeMap<(u32, u32), Handle>,
    rects: BTreeMap<Handle, Rect>,
    strings: BTreeMap<(u32, u32), String>,
}
impl Foreign for Fake {
    fn new_default_procedure(&mut self) -> Option<Handle> {
        self.events.push(Event::NewProcedure);
        Some(900)
    }
    fn create(&mut self, c: u32, i: u32) -> Option<Handle> {
        self.events.push(Event::Create(c, i));
        self.creates.pop_front().flatten()
    }
    fn query(&mut self, h: Handle, i: u32) -> Option<Handle> {
        self.events.push(Event::Query(h, i));
        self.queries.get(&(h, i)).copied()
    }
    fn add_ref(&mut self, h: Handle) {
        self.events.push(Event::Add(h));
    }
    fn release(&mut self, h: Handle) -> u32 {
        self.events.push(Event::Release(h));
        self.release_counts.get(&h).copied().unwrap_or(1)
    }
    fn vocabulary_add(&mut self, h: Handle, s: &str, id: u32) {
        self.events.push(Event::Word(h, s.into(), id));
    }
    fn read(&mut self, h: Handle, r: Read) -> Value {
        self.events.push(Event::Read(h, r.clone()));
        self.reads.get(&(h, r)).cloned().unwrap_or(Value::None)
    }
    fn write(&mut self, h: Handle, w: Write) {
        self.events.push(Event::Write(h, w));
    }
    fn widget(&mut self, h: Handle, slot: u32, args: Vec<Value>) -> Value {
        self.events.push(Event::Widget(h, slot, args));
        self.raw_sequences
            .get_mut(&(h, slot))
            .and_then(|v| v.pop_front())
            .or_else(|| self.raw.get(&(h, slot)).cloned())
            .unwrap_or(Value::None)
    }
    fn font_id(&mut self, n: &str) -> u32 {
        self.events.push(Event::FontId(n.into()));
        7
    }
    fn font(&mut self, id: u32) -> Handle {
        self.events.push(Event::Font(id));
        700
    }
    fn colour(&mut self, c: [u8; 3]) -> u32 {
        self.events.push(Event::Colour(c));
        u32::from_le_bytes([c[0], c[1], c[2], 0])
    }
    fn colour_four(&mut self, c: [u8; 3], fourth: u32) -> u32 {
        self.events.push(Event::ColourFour(c, fourth));
        0
    }
    fn grid_colour(&mut self, h: Handle, c: [u8; 3]) -> u32 {
        self.events.push(Event::GridColour(h, c));
        u32::from_le_bytes([c[0], c[1], c[2], 0])
    }
    fn load_image(&mut self, g: u32, i: u32) -> Option<Handle> {
        self.events.push(Event::Image(g, i));
        self.images.get(&(g, i)).copied()
    }
    fn load_string(&mut self, g: u32, i: u32) -> Option<String> {
        self.events.push(Event::StringResource(g, i));
        self.strings.get(&(g, i)).cloned()
    }
    fn image_rect(&mut self, h: Handle) -> Rect {
        self.events.push(Event::ImageRect(h));
        self.rects.get(&h).copied().unwrap_or([0, 0, 16, 20])
    }
    fn notify_cell(&mut self, h: Handle, col: i32, row: i32, d: CellDescriptor) {
        self.events.push(Event::NotifyCell(h, col, row, d));
    }
    fn notify_axis(&mut self, h: Handle, kind: Notifier, v: [i32; 3]) {
        self.events.push(Event::NotifyAxis(h, kind, v));
    }
    fn winmgr_slot3(&mut self) -> Handle {
        self.events.push(Event::ManagerSlot3);
        900
    }
    fn new_string(&mut self) -> Handle {
        self.events.push(Event::NewString);
        800
    }
    fn append_string(&mut self, h: Handle, text: &str) {
        self.events.push(Event::Append(h, text.into()));
    }
}
fn u(k: u32, v: u32) -> Property {
    Property {
        key: k,
        value: Some(Variant::Scalar(Elem::U32(v))),
    }
}
fn s(k: u32, v: &str) -> Property {
    Property {
        key: k,
        value: Some(Variant::string(v.as_bytes())),
    }
}
fn win(h: Handle, w: Write) -> Event {
    Event::Write(h, w)
}
fn call(h: Handle, slot: u32, args: Vec<Value>) -> Event {
    Event::Widget(h, slot, args)
}
fn n(v: u32) -> Value {
    Value::U32(v)
}
fn i(v: i32) -> Value {
    Value::I32(v)
}
fn text(v: &str) -> Value {
    Value::Text(v.into())
}
fn initialized() -> (LegacyControl, Globals, Fake) {
    let mut f = Fake::default();
    f.creates.push_back(Some(99));
    let mut g = Globals::default();
    let c = LegacyControl::new(&mut g, &mut f);
    f.events.clear();
    (c, g, f)
}

#[test]
fn property_scan_skips_wrong_types_and_preserves_numeric_output() {
    let p = vec![
        Property { key: 5, value: None },
        s(5, "wrong"),
        u(5, 0xffff_fffe),
        u(5, 9),
    ];
    let p = Properties(&p);
    let mut number = 42;
    let mut string = "old".into();
    assert!(p.has(5));
    assert!(p.uint32(5, &mut number));
    assert_eq!(number, 0xffff_fffe);
    let mut signed = 0;
    assert!(p.sint32(5, &mut signed));
    assert_eq!(signed, -2);
    assert!(!p.uint32(9, &mut number));
    assert_eq!(number, 0xffff_fffe);
    assert!(p.string(5, &mut string));
    assert_eq!(string, "wrong");
    assert!(!p.string(9, &mut string));
    assert_eq!(string, "");
}

#[test]
fn exact_vocabulary_order_ids_aliases_and_maps() {
    let mut f = Fake::default();
    f.creates.push_back(Some(99));
    let mut g = Globals::default();
    let _ = LegacyControl::new(&mut g, &mut f);
    let expected = include_str!("vocabulary.expected");
    let mut entries = vec![];
    let mut expected_classes = BTreeMap::new();
    let mut expected_iids = vec![];
    for line in expected.lines() {
        if line.starts_with('[') {
            let fields: Vec<_> = line.trim_matches(['[', ']']).split_whitespace().collect();
            let cls = u32::from_str_radix(fields[1], 16).unwrap();
            let iid = u32::from_str_radix(fields[2], 16).unwrap();
            if cls != 0 {
                expected_classes.insert(cls, iid);
                expected_iids.push(iid);
            }
        } else {
            let (name, id) = line.split_once(' ').unwrap();
            entries.push((name.to_string(), u32::from_str_radix(id, 16).unwrap()));
        }
    }
    assert_eq!(g.classes, expected_classes);
    assert_eq!(g.interpreters.len(), 20);
    for iid in expected_iids {
        assert!(g.interpreters.contains_key(&iid));
    }
    assert_eq!(g.words(), entries);
    assert_eq!(entries.len(), 393);
    let mut effects = vec![Event::Create(0xba2e7954, 0x1a2bdebb)];
    effects.extend(entries.into_iter().map(|(s, id)| Event::Word(99, s, id)));
    assert_eq!(f.events, effects);
    f.events.clear();
    // A fresh constructor must reuse, not register again.
    let _second = LegacyControl::new(&mut g, &mut f);
    assert_eq!(f.events, vec![Event::Add(99)]);
}

#[test]
fn constructor_failure_retry_maps_survive_and_zero_release_destroys() {
    let mut f = Fake::default();
    let mut g = Globals::default();
    let mut c = LegacyControl::new(&mut g, &mut f);
    assert_eq!(c.ref_count(), 0);
    assert_eq!(c.control(), None);
    assert!(!c.owns_control());
    assert!(g.classes.is_empty());
    assert_eq!(c.release(&mut g, &mut f), 0);
    assert!(c.destroyed());
    f.creates.push_back(Some(99));
    let mut c = LegacyControl::new(&mut g, &mut f);
    f.release_counts.insert(99, 0);
    f.events.clear();
    c.release(&mut g, &mut f);
    assert_eq!(f.events, vec![Event::Release(99)]);
    assert_eq!(g.vocabulary(), None);
    assert_eq!(g.classes.len(), 20);
    f.creates.push_back(Some(98));
    let _ = LegacyControl::new(&mut g, &mut f);
    assert_eq!(g.classes.len(), 20);
}

#[test]
fn lifecycle_identity_self_assignment_parent_and_one_shot_procedure() {
    let (mut c, mut g, mut f) = initialized();
    let mut out = Some(Interface::Legacy);
    assert!(!c.query_interface(123, &mut out));
    assert_eq!(out, Some(Interface::Legacy));
    assert!(c.query_interface(0x5a356159, &mut out));
    assert_eq!(out, Some(Interface::Area));
    assert_eq!(c.ref_count(), 1);
    assert!(c.implement_interface([1, 2, 3, 4]));
    c.force_control(Some(10), &mut f);
    c.force_control(Some(10), &mut f);
    c.use_win_proc(Some(20), &mut f);
    c.use_win_proc(Some(20), &mut f);
    assert!(!c.owns_control());
    assert_eq!(f.events, vec![Event::Add(10), Event::Add(20)]);
    f.events.clear();
    f.queries.insert((10, 0x5386d516), 11);
    c.do_exceptions(10, false, &mut f);
    c.do_exceptions(10, true, &mut f);
    c.do_exceptions(10, true, &mut f);
    assert_eq!(
        f.events,
        vec![
            Event::Query(10, 0x5386d516),
            call(11, 0x1c, vec![Value::Handle(20)]),
            Event::Release(20),
            Event::Release(11),
            Event::Query(10, 0x5386d516),
            Event::Release(11)
        ]
    );
    f.events.clear();
    f.queries.insert((50, 0x9a4527cb), 51);
    f.reads.insert((51, Read::LegacyControl), Value::Handle(52));
    assert!(c.set_parent(Some(50), &mut f));
    assert!(c.set_parent(Some(60), &mut f));
    assert_eq!(
        f.events,
        vec![
            Event::Query(50, 0x9a4527cb),
            Event::Read(51, Read::LegacyControl),
            Event::Add(52),
            Event::Release(51),
            Event::Release(52),
            Event::Query(60, 0x9a4527cb)
        ]
    );
    f.events.clear();
    c.set_own_control(true);
    f.reads.insert((10, Read::Parent), Value::Handle(30));
    assert_eq!(c.release(&mut g, &mut f), 0);
    assert_eq!(
        f.events,
        vec![
            Event::Read(10, Read::Parent),
            win(30, Write::ChildRemove(10)),
            win(10, Write::Shutdown),
            Event::Release(10),
            Event::Release(99)
        ]
    );
}

#[test]
fn fallback_attempts_one_lowest_class_then_custom_and_retains_requested_ids() {
    let (mut c, mut g, mut f) = initialized();
    g.classes.insert(1, 0x678d1e84);
    g.classes.insert(2, 0x678d1e84);
    f.creates.extend([None, None, Some(10)]);
    let result = c.set_parameters(
        Properties(&[u(0xffffffff, 0x1234), u(0xfffffffe, 0x678d1e84)]),
        &g,
        &mut f,
    );
    assert_eq!(result, Ok(false));
    assert_eq!(
        &f.events[..5],
        &[
            Event::Create(0x1234, 0x22ba0121),
            Event::Create(1, 0x22ba0121),
            Event::Create(0x478d1e6f, 0x22ba0121),
            win(10, Write::Param(0xe85eebde, n(0x1234))),
            win(10, Write::Param(0x08d6e462, n(0x678d1e84)))
        ]
    );
    assert_eq!(c.control(), Some(10));
}

#[test]
fn missing_zero_clsid_and_second_build_leak() {
    let (mut c, g, mut f) = initialized();
    c.force_control(Some(5), &mut f);
    f.events.clear();
    for props in [vec![], vec![u(0xffffffff, 0)], vec![s(0xffffffff, "wrong")]] {
        assert_eq!(c.set_parameters(Properties(&props), &g, &mut f), Ok(false));
        assert_eq!(c.control(), Some(5));
        assert!(f.events.is_empty());
    }
    f.creates.extend([Some(10), Some(20)]);
    for _ in 0..2 {
        assert_eq!(
            c.set_parameters(Properties(&[u(0xffffffff, 0x478d1e6f)]), &g, &mut f),
            Ok(false)
        );
    }
    assert_eq!(c.control(), Some(20));
    assert!(!f.events.contains(&Event::Release(5)));
    assert!(!f.events.contains(&Event::Release(10)));
}

#[test]
fn exact_two_pass_slider_common_init_parent_front_order() {
    let (mut c, g, mut f) = initialized();
    f.queries.insert((50, 0x9a4527cb), 51);
    f.reads.insert((51, Read::LegacyControl), Value::Handle(52));
    c.set_parent(Some(50), &mut f);
    f.events.clear();
    f.creates.push_back(Some(10));
    f.queries.insert((10, 0x21325207), 12);
    let props = [
        u(0xffffffff, 0x21325208),
        s(0xc00, "(100,0)"),
        u(0xc01, 0xc03),
        u(0x40a, 30),
        u(0xf01b, 0),
    ];
    assert_eq!(c.set_parameters(Properties(&props), &g, &mut f), Ok(false));
    assert_eq!(
        f.events,
        vec![
            Event::Create(0x21325208, 0x22ba0121),
            win(10, Write::Param(0xe85eebde, n(0x21325208))),
            win(10, Write::Param(0x08d6e462, n(0x21325207))),
            Event::Query(10, 0x21325207),
            call(12, 0x20, vec![i(100)]),
            call(12, 0x18, vec![i(0)]),
            call(12, 0x28, vec![n(2)]),
            Event::Release(12),
            win(10, Write::Flag(1, true)),
            win(10, Write::Flag(0x10000, true)),
            win(10, Write::Flag(0x800, false)),
            win(10, Write::Flag(0x20000, true)),
            win(10, Write::Flag(0x40000, true)),
            win(10, Write::Flag(4, false)),
            win(10, Write::Flag(0x8000, true)),
            win(10, Write::Param(0x3aca6906, Value::Pair(0, 0))),
            win(
                10,
                Write::Param(0xbaca6879, Value::Comments(std::array::from_fn(|_| String::new())))
            ),
            Event::Query(10, 0x5386d516),
            win(10, Write::Init),
            win(52, Write::ChildAdd(10)),
            Event::Release(52),
            win(10, Write::Front),
            Event::Query(10, 0x21325207),
            call(12, 0x14, vec![i(30)]),
            Event::Release(12),
            win(10, Write::Flag(2, false))
        ]
    );
    assert_eq!(c.pending_parent(), None);
}

#[test]
fn text_build_reaches_common_and_attach() {
    let (mut c, g, mut f) = initialized();
    f.creates.push_back(Some(10));
    f.queries.insert((10, 0x212cdc1f), 11);
    assert_eq!(
        c.set_parameters(Properties(&[u(0xffffffff, 0x592)]), &g, &mut f),
        Ok(false)
    );
    assert_eq!(
        &f.events[..4],
        &[
            Event::Create(0x592, 0x22ba0121),
            win(10, Write::Param(0xe85eebde, n(0x592))),
            win(10, Write::Param(0x08d6e462, n(0x212cdc1f))),
            Event::Query(10, 0x212cdc1f),
        ]
    );
    assert!(f.events.contains(&win(10, Write::Init)));
    assert!(f.events.contains(&win(10, Write::Front)));
    assert!(c.owns_control());
    assert_eq!(c.control(), Some(10));
}

#[test]
fn common_geometry_caption_resource_and_unconditional_parameters() {
    let mut f = Fake::default();
    f.strings.insert((1, 2), "Loaded".into());
    let p = [
        s(0x101, "(1,2,3,4)"),
        s(0x102, "(8,9)"),
        s(0x103, "(9,9)"),
        s(0x105, "literal"),
        s(0x106, "{1,2}"),
        Property {
            key: 0x107,
            value: None,
        },
    ];
    interpret_common(10, Properties(&p), true, &mut f).unwrap();
    assert_eq!(
        &f.events[7..],
        &[
            win(10, Write::Area([1, 2, 3, 4])),
            win(10, Write::Caption("literal".into())),
            Event::StringResource(1, 2),
            win(10, Write::Caption("Loaded".into())),
            win(
                10,
                Write::Param(
                    0x0b31771a,
                    Value::Key(ResourceKey {
                        kind: 0x2026960b,
                        group: 1,
                        instance: 2
                    })
                )
            ),
            win(10, Write::Param(0x3aca6906, Value::Pair(1, 2))),
            win(10, Write::Flag(0x20000, true)),
            win(
                10,
                Write::Param(0xbaca6879, Value::Comments(std::array::from_fn(|_| String::new())))
            )
        ]
    );
}

#[test]
fn flat_rect_outline_folders_and_slider_boundary_effects() {
    let mut f = Fake::default();
    for iid in [0xc2afa76f, 0xe303e5ae, 0xa13351b8, 0x21325207] {
        f.queries.insert((10, iid), 11);
    }
    interpret(
        Kind::FlatRect,
        10,
        Properties(&[
            u(0xf005, 0x204),
            u(0xf005, 0x205),
            u(0xf005, 0xf00b),
            s(0x201, "(266,-1,30)"),
        ]),
        true,
        &mut f,
    )
    .unwrap();
    assert_eq!(
        f.events,
        vec![
            Event::Query(10, 0xc2afa76f),
            call(11, 0x30, vec![n(0x2a)]),
            call(11, 0x28, vec![n(1), n(10), n(255), n(30)]),
            Event::Release(11)
        ]
    );
    f.events.clear();
    interpret(Kind::Outline, 10, Properties(&[]), true, &mut f).unwrap();
    assert_eq!(f.events, vec![Event::Query(10, 0xe303e5ae), Event::Release(11)]);
    f.events.clear();
    let p: Vec<_> = (0..34).map(|v| s(0x1102, &format!("({v},2)"))).collect();
    interpret(Kind::Outline, 10, Properties(&p), true, &mut f).unwrap();
    assert_eq!(
        f.events[1],
        call(
            11,
            0x34,
            vec![n(32), Value::Points((0..32).map(|v| [v, 2]).collect()), n(1), n(1)]
        )
    );
    f.events.clear();
    interpret(
        Kind::Folders,
        10,
        Properties(&[s(0x502, "A"), s(0x502, "B")]),
        false,
        &mut f,
    )
    .unwrap();
    assert_eq!(
        f.events,
        vec![
            Event::Query(10, 0xa13351b8),
            call(11, 0x1c, vec![]),
            call(11, 0x10, vec![text("A"), n(0)]),
            call(11, 0x10, vec![text("B"), n(0)]),
            Event::Release(11)
        ]
    );
    f.events.clear();
    interpret(
        Kind::Slider,
        10,
        Properties(&[s(0xc00, "(-5,-5)"), u(0xc01, 999)]),
        true,
        &mut f,
    )
    .unwrap();
    assert_eq!(
        f.events,
        vec![
            Event::Query(10, 0x21325207),
            call(11, 0x20, vec![i(-5)]),
            call(11, 0x18, vec![i(-5)]),
            call(11, 0x28, vec![n(1)]),
            Event::Release(11)
        ]
    );
}

#[test]
fn tree_position_retention_and_depth_first_search() {
    let mut f = Fake::default();
    f.queries.insert((10, 0x3ae8bad2), 11);
    f.raw.insert((11, 0x60), n(40));
    f.raw_sequences.insert((11, 0x60), [n(40), n(41), n(42)].into());
    let props = [
        s(0x1102, "b:Beta:a:first::::::::::no:yes"),
        s(0x1102, "a:Alpha:Root:invalid::::::::::no:no"),
        s(0x1102, "c:Gamma:Root:invalid::::::::::no:no"),
    ];
    interpret(Kind::TreeView, 10, Properties(&props), false, &mut f).unwrap();
    let adds: Vec<_> = f
        .events
        .iter()
        .filter(|e| matches!(e, Event::Widget(_, 0x60, _)))
        .cloned()
        .collect();
    assert_eq!(
        adds,
        vec![
            call(11, 0x60, vec![n(0), i(-2), text("Alpha"), n(0)]),
            call(11, 0x60, vec![n(40), i(-2), text("Beta"), n(0)]),
            call(11, 0x60, vec![n(0), i(-2), text("Gamma"), n(0)])
        ]
    );
    f.events.clear();
    interpret(
        Kind::TreeView,
        10,
        Properties(&[s(0x1102, "z:Z:Root:bad::::::::::no:no")]),
        false,
        &mut f,
    )
    .unwrap();
    assert!(f.events.contains(&call(11, 0x60, vec![n(0), i(-3), text("Z"), n(0)])));
}

#[test]
fn grid_shared_width_header_and_column_only_sizing() {
    let mut f = Fake::default();
    f.queries.insert((10, 0xdaa6b9be), 11);
    f.raw.insert((11, 0x14), Value::Handle(12));
    let p = [u(0x1607, 1), s(0x1626, "2,4,60"), s(0x1627, "2,4,18")];
    interpret(Kind::Grid, 10, Properties(&p), true, &mut f).unwrap();
    assert!(f.events.contains(&call(11, 0x128, vec![n(1), n(0)])));
    assert!(f.events.contains(&call(11, 0x104, vec![i(2), i(1), i(60)])));
    assert!(f.events.contains(&call(11, 0x10c, vec![i(2), i(3), i(18)])));
    f.events.clear();
    interpret(Kind::Grid, 10, Properties(&[]), true, &mut f).unwrap();
    assert!(f.events.contains(&call(11, 0x128, vec![n(0), n(0)])));
    f.events.clear();
    let p = [s(0x1625, "0,0,2000,1,2")];
    assert_eq!(
        interpret(Kind::Grid, 10, Properties(&p), true, &mut f),
        Err(Unsupported::Malformed("Grid style scratch uninitialized"))
    );
    assert_eq!(f.events.last(), Some(&Event::Release(11)));
}

#[test]
fn release_only_order_refcounts_and_accessor_channels() {
    let (mut c, mut g, mut f) = initialized();
    assert_eq!(c.get_id(&mut f), Ok(0));
    let mut area = [9; 4];
    assert_eq!(c.get_area(&mut area, &mut f), Ok(false));
    assert_eq!(area, [9; 4]);
    assert!(!c.set_area([1, 2, 3, 4], &mut f));
    c.set_id(9, &mut f);
    assert!(f.events.is_empty());
    for (iid, interface) in [
        (1, Interface::Primary),
        (0x1a356083, Interface::Primary),
        (0x9a4527cb, Interface::Legacy),
    ] {
        let mut out = None;
        assert!(c.query_interface(iid, &mut out));
        assert_eq!(out, Some(interface));
    }
    assert_eq!(c.ref_count(), 3);
    assert_eq!(c.release(&mut g, &mut f), 2);
    assert!(f.events.is_empty());
    c.force_control(Some(10), &mut f);
    c.use_win_proc(Some(20), &mut f);
    f.queries.insert((30, 0x9a4527cb), 31);
    f.reads.insert((31, Read::LegacyControl), Value::Handle(32));
    c.set_parent(Some(30), &mut f);
    f.events.clear();
    f.reads.insert((10, Read::Id), n(100));
    f.reads.insert((10, Read::Area), Value::Rect([1, 2, 3, 4]));
    assert_eq!(c.get_id(&mut f), Ok(100));
    assert_eq!(c.get_area(&mut area, &mut f), Ok(true));
    assert_eq!(area, [1, 2, 3, 4]);
    c.set_id(101, &mut f);
    assert!(c.set_area([4, 3, 2, 1], &mut f));
    assert_eq!(c.get_control_vocabulary(&g), Some(99));
    f.events.clear();
    assert_eq!(c.release(&mut g, &mut f), 1);
    assert!(f.events.is_empty());
    assert_eq!(c.release(&mut g, &mut f), 0);
    assert_eq!(
        f.events,
        vec![
            Event::Release(10),
            Event::Release(32),
            Event::Release(99),
            Event::Release(20)
        ]
    );
}

#[test]
fn failed_parent_qi_and_procedure_qi_preserve_only_the_procedure() {
    let (mut c, _g, mut f) = initialized();
    c.use_win_proc(Some(20), &mut f);
    f.events.clear();
    c.do_exceptions(10, true, &mut f);
    assert_eq!(c.procedure(), Some(20));
    assert!(c.set_parent(Some(30), &mut f));
    assert_eq!(c.pending_parent(), None);
    f.queries.insert((30, 0x9a4527cb), 31);
    c.set_parent(Some(30), &mut f);
    assert_eq!(
        &f.events[2..],
        &[
            Event::Query(30, 0x9a4527cb),
            Event::Read(31, Read::LegacyControl),
            Event::Release(31)
        ]
    );
}

#[test]
fn default_parent_is_exact_opaque_manager_slot_and_all_factory_failures() {
    let (mut c, g, mut f) = initialized();
    f.creates.extend([None, None, None]);
    assert_eq!(
        c.set_parameters(Properties(&[u(0xffffffff, 0x21325208)]), &g, &mut f),
        Ok(false)
    );
    assert_eq!(
        f.events,
        vec![
            Event::Create(0x21325208, 0x22ba0121),
            Event::Create(0x21325208, 0x22ba0121),
            Event::Create(0x478d1e6f, 0x22ba0121)
        ]
    );
    assert_eq!(c.control(), None);
    f.events.clear();
    f.creates.push_back(Some(10));
    c.set_parameters(Properties(&[u(0xffffffff, 0x478d1e6f)]), &g, &mut f)
        .unwrap();
    let init = f.events.iter().position(|e| *e == win(10, Write::Init)).unwrap();
    assert_eq!(
        &f.events[init..],
        &[
            win(10, Write::Init),
            Event::ManagerSlot3,
            win(900, Write::ChildAdd(10)),
            win(10, Write::Front)
        ]
    );
}

#[test]
fn grid_style_scratch_is_last_value_not_cell_flags_and_partial_scan_is_stale() {
    let mut f = Fake::default();
    f.queries.insert((10, 0xdaa6b9be), 11);
    f.raw.insert((11, 0x14), Value::Handle(12));
    let p = [
        u(0x160d, 0),
        s(0x1625, "1,2,0,00000011,00000022"),
        s(0x1625, "3,4,0,00000033,bad!"),
    ];
    interpret(Kind::Grid, 10, Properties(&p), true, &mut f).unwrap();
    assert!(f.events.contains(&Event::NotifyCell(
        10,
        1,
        2,
        CellDescriptor {
            flags: 0,
            group: 0x11,
            instance: 0x22,
            text: String::new()
        }
    )));
    // 'bad' is valid hex, so the conversion assigns 0xbad before '!'.
    assert!(f.events.contains(&Event::NotifyCell(
        10,
        3,
        4,
        CellDescriptor {
            flags: 0,
            group: 0x33,
            instance: 0xbad,
            text: String::new()
        }
    )));
    f.events.clear();
    let p = [
        u(0x160d, 0),
        s(0x1625, "1,2,40000,00000011,00000022"),
        s(0x1625, "3,4,0,00000033,nothex"),
    ];
    interpret(Kind::Grid, 10, Properties(&p), true, &mut f).unwrap();
    assert!(f.events.contains(&Event::NotifyCell(
        10,
        3,
        4,
        CellDescriptor {
            flags: 0,
            group: 0x33,
            instance: 0x22,
            text: String::new()
        }
    )));
    f.events.clear();
    let p = [u(0x160d, 0x40000), s(0x1625, "1,2,0,00000011,00000022")];
    assert_eq!(
        interpret(Kind::Grid, 10, Properties(&p), true, &mut f),
        Err(Unsupported::Malformed("Grid cell uninitialized resource group"))
    );
}

#[test]
fn file_browser_keeps_raw_getter_values_and_filter_uses_pair_position() {
    let mut f = Fake::default();
    f.queries.insert((10, 0x3aa52e64), 11);
    for (slot, values) in [
        (0x88, vec![7, 8]),
        (0x98, vec![3, 4, 5]),
        (0x108, vec![1, 0x55]),
        (0x110, vec![0, 0x66]),
        (0x118, vec![0x77, 0x88]),
    ] {
        f.raw.insert((11, slot), Value::Words(values));
    }
    let p = [
        u(0x130c, 0x1309),
        u(0x131a, 0x1316),
        s(0x1342, "All:12:{1,2}"),
        u(0x99, 7),
        s(0x1342, "Other:bad:{3,4}"),
    ];
    interpret(Kind::FileBrowser, 10, Properties(&p), true, &mut f).unwrap();
    assert_eq!(
        f.events,
        vec![
            Event::Query(10, 0x3aa52e64),
            call(11, 0x88, vec![]),
            call(11, 0x8c, vec![n(1), n(8)]),
            call(11, 0x98, vec![]),
            call(11, 0xa0, vec![n(3), n(5), n(5)]),
            call(11, 0x108, vec![]),
            call(11, 0x10c, vec![n(1), n(0x55)]),
            call(11, 0x110, vec![]),
            call(11, 0x114, vec![n(0), n(0x66)]),
            call(11, 0x118, vec![]),
            call(11, 0x11c, vec![n(0x77), n(0x88)]),
            Event::Image(1, 2),
            win(10, Write::Param(0xdaa3b863, Value::Pair(1, 2))),
            call(11, 0x54, vec![text("All"), Value::None, i(12), n(0)]),
            Event::Image(3, 4),
            win(10, Write::Param(0xdaa3b865, Value::Pair(3, 4))),
            call(11, 0x54, vec![text("Other"), Value::None, i(0), n(0)]),
            Event::Release(11)
        ]
    );
}

#[test]
fn tree_images_maps_root_defaults_and_numeric_font_precedes_string() {
    let mut f = Fake::default();
    f.queries.insert((10, 0x3ae8bad2), 11);
    f.images.insert((1, 2), 60);
    f.raw.insert((11, 0x50), n(5));
    let p = [
        s(0xf016, "folder:{1,2}:(0,0,0,0)"),
        s(0x1408, "folder"),
        s(0x1409, "missing"),
        s(0x1410, "font-name"),
        u(0x1410, 3),
        u(0x1411, 2),
        u(0x1412, 2),
    ];
    interpret(Kind::TreeView, 10, Properties(&p), false, &mut f).unwrap();
    assert_eq!(
        &f.events[..5],
        &[
            Event::Query(10, 0x3ae8bad2),
            Event::Image(1, 2),
            Event::ImageRect(60),
            call(11, 0x50, vec![Value::Handle(60), Value::Rect([0, 0, 16, 20])]),
            Event::Release(60)
        ]
    );
    assert!(f.events.contains(&call(11, 0x90, vec![n(0), n(0), n(5)])));
    assert!(f.events.contains(&call(11, 0x90, vec![n(0), n(1), n(1)])));
    assert!(f.events.contains(&call(11, 0x9c, vec![n(0), Value::Handle(700)])));
    assert!(f.events.contains(&call(11, 0xb4, vec![n(0), n(0)])));
    assert!(!f.events.iter().any(|e| matches!(e, Event::Widget(_, 0xe0, _))));
}

fn fragment_fields(f: &mut Fake, cls: u32, iid: u32) {
    for (r, v) in [
        (Read::Param(0xe85eebde), n(cls)),
        (Read::Param(0x08d6e462), n(iid)),
        (Read::Id, n(0x1234)),
        (Read::Area, Value::Rect([0, 0, 100, 20])),
        (Read::Fill, Value::Colour([255, 255, 255])),
        (Read::Caption, text("OK")),
    ] {
        f.reads.insert((10, r), v);
    }
    for mask in [
        1, 2, 0x100, 0x200, 0x800, 0x10000, 0x20000, 0x40000, 0x100000, 4, 0x8000, 0x80000, 0x200000,
    ] {
        f.reads
            .insert((10, Read::Flag(mask)), n((mask == 1 || mask == 2) as u32));
    }
}

#[test]
fn fragment_wrapper_exact_common_text_and_lc8_partial() {
    let (mut c, _g, mut f) = initialized();
    let mut out = None;
    assert_eq!(c.get_script_fragment(&mut out, &mut f), Ok(false));
    assert!(f.events.is_empty());
    c.force_control(Some(10), &mut f);
    f.events.clear();
    fragment_fields(&mut f, 0x592, 0x212cdc1f);
    f.queries.insert((10, 0x212cdc1f), 11);
    assert_eq!(c.get_script_fragment(&mut out, &mut f), Ok(true));
    let fragment = out.unwrap();
    assert_eq!(fragment.string, 800);
    assert_eq!(
        fragment.coverage,
        FragmentCoverage::PartialLc8 { interface: 0x212cdc1f }
    );
    assert_eq!(fragment.text,"<LEGACY clsid=GZWinText iid=IGZWinText id=0x00001234 area=(0,0,100,20) fillcolor=(255,255,255) caption=\"OK\" winflag_visible=yes winflag_enabled=yes winflag_moveable=no winflag_sizeable=no winflag_sortable=no winflag_pbuff=no winflag_pbufftrans=no winflag_pbufferase=no winflag_pbuffvid=no winflag_alphablend=no winflag_acceptfocus=no winflag_mousetrans=no winflag_ignoremouse=no >\n");
    assert_eq!(&f.events[..2], &[Event::NewString, Event::Add(800)]);
    assert_eq!(
        &f.events[f.events.len() - 3..],
        &[
            Event::Query(10, 0x212cdc1f),
            Event::Release(11),
            Event::Append(800, fragment.text)
        ]
    );
}

#[test]
fn fragment_resolution_scans_all_21_rows_and_preserves_stored_class() {
    let (mut c, _g, mut f) = initialized();
    c.force_control(Some(10), &mut f);
    fragment_fields(&mut f, 0x12345678, 0);
    f.queries.insert((10, 0xc2afa76f), 11);
    f.events.clear();
    let mut out = None;
    c.get_script_fragment(&mut out, &mut f).unwrap();
    let fragment = out.unwrap();
    assert!(fragment
        .text
        .starts_with("<LEGACY clsid=0x12345678 iid=IGZWinFlatRect "));
    let queries: Vec<_> = f
        .events
        .iter()
        .filter_map(|e| if let Event::Query(_, iid) = e { Some(*iid) } else { None })
        .collect();
    assert_eq!(
        queries,
        vec![
            0xc2afa76f, 0x212cdc1f, 0x231a1c57, 0x4132242b, 0x21335c59, 0x412ce496, 0x8810, 0xc12cea13, 0x61325a2d,
            0x21325207, 0xa1336cc0, 0x612ce0c3, 0xe303e5ae, 0x032f0b3c, 0x5386d516, 0x3aa52e64, 0x3ae8bad2, 0xa13351b8,
            0xdaa6b9be, 0x678d1e84, 0x22ba0121, 0xc2afa76f
        ]
    );
    // Wrapper failure leaves caller's old output unchanged and releases new string.
    let mut out = Some(fragment.clone());
    f.reads.remove(&(10, Read::Area));
    f.events.clear();
    assert_eq!(
        c.get_script_fragment(&mut out, &mut f),
        Err(Unsupported::ForeignResult("fragment area"))
    );
    assert_eq!(out, Some(fragment));
    assert_eq!(f.events.last(), Some(&Event::Release(800)));
}

#[test]
fn every_published_type_traverses_the_production_builder() {
    for (cls, iid) in [
        (0xc2afa76e, 0xc2afa76f),
        (0x21325208, 0x21325207),
        (0x3ae8bae1, 0x3ae8bad2),
        (0x1aa52ea4, 0x3aa52e64),
        (0xdaa6b9bf, 0xdaa6b9be),
        (0x078e8055, 0xa13351b8),
        (0x4303e5b6, 0xe303e5ae),
        (0x592, 0x212cdc1f),
        (0x231a1c58, 0x231a1c57),
        (0x598, 0x4132242b),
        (0x59b, 0x412ce496),
        (0x22ecfc47, 0x8810),
        (0x82fe68c4, 0xc12cea13),
        (0x61325a2e, 0x61325a2d),
        (0x21335c5a, 0x21335c59),
        (0x612ce0c4, 0x612ce0c3),
        (0xe32f0b31, 0x032f0b3c),
        (0xa1336cc1, 0xa1336cc0),
        (0x4386d516, 0x5386d516),
    ] {
        let (mut c, g, mut f) = initialized();
        f.creates.push_back(Some(10));
        f.queries.insert((10, iid), 11);
        f.raw.insert((11, 0x14), Value::Handle(12));
        f.raw.insert((11, 0x140), Value::U32(0));
        for (slot, values) in [
            (0x88, vec![7, 8]),
            (0x98, vec![3, 4, 5]),
            (0x108, vec![1, 0x55]),
            (0x110, vec![0, 0x66]),
            (0x118, vec![0x77, 0x88]),
        ] {
            f.raw.insert((11, slot), Value::Words(values));
        }
        let p = [u(0xffffffff, cls), u(0xf01b, 0)];
        assert_eq!(c.set_parameters(Properties(&p), &g, &mut f), Ok(false));
        assert_eq!(c.control(), Some(10));
        let type_queries: Vec<_> = f
            .events
            .iter()
            .enumerate()
            .filter_map(|(k, e)| (*e == Event::Query(10, iid)).then_some(k))
            .collect();
        assert_eq!(type_queries.len(), if iid == GEN_IID { 3 } else { 2 });
        let common = f
            .events
            .iter()
            .position(|e| *e == win(10, Write::Flag(1, true)))
            .unwrap();
        let init = f.events.iter().position(|e| *e == win(10, Write::Init)).unwrap();
        let front = f.events.iter().position(|e| *e == win(10, Write::Front)).unwrap();
        let second = *type_queries.last().unwrap();
        assert!(type_queries[0] < common && common < init && init < front && front < second);
        assert_eq!(f.events.last(), Some(&win(10, Write::Flag(2, false))));
    }
}

#[test]
fn image_reference_and_parameter_effects_are_owned_by_core() {
    let mut f = Fake::default();
    f.queries.insert((10, 0x21325207), 11);
    f.images.insert((1, 2), 60);
    let p = [s(0xf016, "{1,2}"), s(0xf018, "(2,3,8,9)")];
    interpret(Kind::Slider, 10, Properties(&p), true, &mut f).unwrap();
    assert_eq!(
        f.events,
        vec![
            Event::Query(10, 0x21325207),
            Event::Image(1, 2),
            call(11, 0x30, vec![Value::Handle(60), Value::Rect([2, 3, 8, 9])]),
            win(
                10,
                Write::Param(
                    0x6b4bcbd4,
                    Value::Key(ResourceKey {
                        kind: 0x856ddbac,
                        group: 1,
                        instance: 2
                    })
                )
            ),
            win(10, Write::Param(0xdaa3b860, Value::Pair(1, 2))),
            Event::Release(60),
            Event::Release(11)
        ]
    );
    f.events.clear();
    interpret(Kind::Slider, 10, Properties(&[s(0xf016, "{1,}")]), true, &mut f).unwrap();
    assert_eq!(f.events, vec![Event::Query(10, 0x21325207), Event::Release(11)]);
    f.queries.insert((10, 0xa13351b8), 11);
    f.events.clear();
    let p = [s(0xf000, "name"), u(0xf000, 3), s(0xf016, "{1,}")];
    interpret(Kind::Folders, 10, Properties(&p), true, &mut f).unwrap();
    assert_eq!(
        f.events,
        vec![
            Event::Query(10, 0xa13351b8),
            Event::Font(3),
            call(11, 0x28, vec![Value::Handle(700)]),
            win(10, Write::Param(0xfaa3be85, n(3))),
            Event::Image(1, 0),
            win(10, Write::Param(0xdaa3b860, Value::Pair(1, 0))),
            Event::Release(11)
        ]
    );
}
