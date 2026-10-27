// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::ui_windows::base::{Host as BaseHost, Rect as BaseRect, Windows};

struct Base;
impl BaseHost for Base {}
struct Trace {
    windows: Windows,
    calls: Vec<Call>,
    graph_calls: Vec<(Space, u32, Vec<Arg>)>,
    selects: Vec<(Handle, [u32; 2])>,
    titles: Vec<(String, u32)>,
    definitions: BTreeMap<u32, Definition>,
    states: BTreeMap<Handle, u32>,
    main: Handle,
    collapsed: Handle,
    expanded: Handle,
    today: i32,
    start: i32,
    filter: bool,
    values: BTreeMap<u32, f32>,
}
impl Trace {
    fn new() -> Self {
        let mut windows = Windows::default();
        let root = windows.create(&mut Base, 1, None);
        let main = windows.create(&mut Base, 1, None);
        let collapsed = windows.create(&mut Base, 1, None);
        let expanded = windows.create(&mut Base, 1, None);
        windows.get_mut(collapsed).id = 0x8a8b5b72;
        windows.get_mut(expanded).id = 0x8a8b5b71;
        for child in [main, collapsed, expanded] {
            windows.child_add(&mut Base, root, child);
        }
        let mut result = Self {
            windows,
            calls: vec![],
            graph_calls: vec![],
            selects: vec![],
            titles: vec![],
            definitions: BTreeMap::new(),
            states: BTreeMap::new(),
            main,
            collapsed,
            expanded,
            today: 1000,
            start: 0,
            filter: true,
            values: BTreeMap::new(),
        };
        for (parent, ids) in [
            (main, vec![0x4001, 0x4101, 0x4005, 0x4105]),
            (collapsed, vec![0x0a8b7e01, 0xca8b82f9]),
            (
                expanded,
                vec![0x0a8b7e00, 0xca8b82f9, 0x8a2871d9, 0x8a2871d8, 0xea4a7cf2, 0x8a4b0aa0],
            ),
        ] {
            for id in ids {
                result.child(parent, id);
            }
        }
        result
    }
    fn create(&mut self, id: u32) -> Handle {
        let handle = self.windows.create(&mut Base, 1, None);
        let state = self.windows.get_mut(handle);
        state.id = id;
        state.area = BaseRect(0, 0, 400, 240);
        handle
    }
    fn child(&mut self, parent: Handle, id: u32) -> Handle {
        let child = self.create(id);
        self.windows.child_add(&mut Base, parent, child);
        child
    }
    fn owner(&mut self) -> Owner {
        let main = self.main;
        Owner::new(self, 10000, Some(main)).unwrap()
    }
}
impl Host for Trace {
    fn call(&mut self, call: Call) -> Result<Value> {
        self.calls.push(call.clone());
        Ok(match call {
            Call::AddRef(_) | Call::Release(_) => Value::Unit,
            Call::Window(window, op) => match op {
                WindowOp::Parent => Value::Handle(self.windows.get(window).parent),
                WindowOp::Find { id, .. } => Value::Handle(self.windows.child_from_id(window, id)),
                WindowOp::Query(_) => Value::Handle(Some(window)),
                WindowOp::Children => Value::Handles(self.windows.get(window).children().to_vec()),
                WindowOp::Flag(mask, on) => {
                    self.windows.set_flag(&mut Base, window, mask, on);
                    Value::Unit
                }
                WindowOp::Show(on) => {
                    if on {
                        self.windows.show(&mut Base, window);
                    } else {
                        self.windows.hide(&mut Base, window);
                    }
                    Value::Unit
                }
                WindowOp::Visible => Value::Bool(self.windows.get(window).flag(1)),
                WindowOp::Notification(target) => {
                    self.windows.get_mut(window).notification = Some(target);
                    Value::Unit
                }
                WindowOp::Area => {
                    let BaseRect(x0, y0, x1, y1) = self.windows.get(window).area;
                    Value::Rect([x0, y0, x1, y1])
                }
                WindowOp::Height => Value::Word(10),
                WindowOp::SetArea(rect) => {
                    self.windows.get_mut(window).area = BaseRect(rect[0], rect[1], rect[2], rect[3]);
                    Value::Unit
                }
                WindowOp::AddChild(child) => {
                    self.windows.child_add(&mut Base, window, child);
                    Value::Unit
                }
                WindowOp::ButtonState(1) => Value::Word(self.states.get(&window).copied().unwrap_or(1)),
                WindowOp::SliderValue => Value::Word(0),
                _ => Value::Unit,
            },
            Call::Service(service) => match service {
                Service::Text(id) => Value::Text(format!("text:{id:08x}")),
                Service::Rgb([r, g, b]) => Value::Word((u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)),
                Service::Font(id) => Value::Handle(Some(u64::from(id))),
                Service::Today => Value::Date(self.today),
                Service::StartDate => Value::Date(self.start),
                Service::History { key, .. } => self
                    .values
                    .get(&key)
                    .map(|&v| Value::Float(v))
                    .unwrap_or(Value::Missing),
                Service::CreateWidget { .. } => Value::Handle(Some(self.create(0))),
                Service::CreateCheckbox {
                    id,
                    area,
                    target,
                    visible,
                    ..
                } => {
                    let window = self.create(id);
                    self.windows.get_mut(window).area = BaseRect(area[0], area[1], area[2], area[3]);
                    self.windows.get_mut(window).notification = Some(target);
                    self.states.insert(window, u32::from(visible));
                    Value::Handle(Some(window))
                }
                Service::Filter(_) => Value::Bool(self.filter),
                _ => Value::Unit,
            },
        })
    }
    fn definition(&mut self, graph: u32) -> Result<Option<Definition>> {
        Ok(self.definitions.get(&graph).cloned())
    }
    fn graph(&mut self, _widget: Handle, space: Space, slot: u32, args: Vec<Arg>) -> Result<Value> {
        self.graph_calls.push((space, slot, args));
        Ok(match (space, slot) {
            (Space::Graph, 25 | 34) => Value::Float(250.0),
            (Space::Graph, 26) => Value::Float(10.0),
            (Space::Graph, 49) => Value::Handle(Some(self.create(0))),
            _ => Value::Unit,
        })
    }
    fn select_button(&mut self, button: Handle, args: [u32; 2]) -> Result<()> {
        self.selects.push((button, args));
        Ok(())
    }
    fn axis_title(&mut self, _widget: Handle, text: String, index: u32) -> Result<()> {
        self.titles.push((text, index));
        Ok(())
    }
    fn line_control(&mut self, window: Handle, show: bool) -> Result<()> {
        self.call(Call::Window(window, WindowOp::Show(show)))?;
        Ok(())
    }
}
#[test]
fn section1_constructor_init_repeat_shutdown_and_destructor_reuse_base_windows() {
    let mut host = Trace::new();
    let mut owner = host.owner();
    assert_eq!(owner.graph_id, 1);
    assert!(owner.expanded);
    assert!(!owner.shown);
    assert_eq!(
        owner.axis,
        Axis {
            maximum: 0.0,
            minimum: 0.0,
            step: 0.0
        }
    );
    assert_eq!(host.calls, [Call::AddRef(host.main)]);
    assert_eq!(owner.init(&mut host), Ok(true));
    for window in [host.main, host.collapsed, host.expanded] {
        assert!(!host.windows.get(window).flag(0x800));
    }
    assert!(!host.windows.get(host.collapsed).flag(1));
    assert!(!host.windows.get(host.expanded).flag(1));
    let button = host.windows.child_from_id(host.collapsed, 0x0a8b7e01).unwrap();
    assert_eq!(host.windows.get(button).notification, Some(host.main));
    host.calls.clear();
    owner.init(&mut host).unwrap();
    assert!(!host
        .calls
        .iter()
        .any(|c| matches!(c, Call::Service(Service::Subscribe(..)) | Call::AddRef(_))));
    owner.destroy(&mut host).unwrap();
    for h in [host.main, host.collapsed, host.expanded] {
        assert_eq!(host.calls.iter().filter(|c| **c == Call::Release(h)).count(), 2);
    }
    // One Release is the counted generator QI, one the owned window; main only has the destructor QI here.
    assert!(owner.refresh(&mut host).is_err());
}
#[test]
fn gw_c1_dates_and_all_time_scales_have_one_extra_sample() {
    let (a, samples) = sample_dates(1000, 0).unwrap();
    assert_eq!(a, 630);
    assert_eq!(samples, (660..=1020).step_by(30).collect::<Vec<_>>());
    for (i, (points, step, _)) in TIMES.iter().enumerate() {
        let (a, dates) = sample_dates(1000, i).unwrap();
        assert_eq!(dates.len(), points + 1);
        assert_eq!(dates[0], a + step);
    }
    assert!(sample_dates(0, 6).is_err());
}
#[test]
fn gw_c2_demands_transform_and_negative_and_unset_cases() {
    let limits = Axis {
        maximum: 6000.0,
        minimum: -6000.0,
        step: 2000.0,
    };
    assert!((bar_value(3000.0, 5, limits) - 4261.0).abs() < 0.1);
    assert!((bar_value(-3000.0, 5, limits) + 4261.0).abs() < 0.1);
    assert_eq!(bar_value(0.0, 5, limits), 0.0);
    assert_eq!(bar_value(3000.0, 5, Axis::default()), 3000.0);
}
#[test]
fn gw_c3_axis_range_250_and_fractional_and_explicit_step() {
    let output = axis(Axis::default(), 250.0, 0.0).unwrap();
    assert_eq!(output.step, 50.0);
    assert!(!output.fractional);
    assert!((output.maximum - 250.0 * 20.0 / 19.0).abs() < 0.0001);
    assert_eq!(axis(Axis::default(), 0.5, 0.0).unwrap().step, 0.2);
    let output = axis(
        Axis {
            maximum: 1.2,
            minimum: 0.0,
            step: 7.0,
        },
        9.0,
        8.0,
    )
    .unwrap();
    assert_eq!(
        output,
        AxisOutput {
            maximum: 2.0,
            minimum: 0.0,
            step: 7.0,
            fractional: true
        }
    );
}
#[test]
fn gw_c4_operations_and_division_threshold() {
    assert_eq!(line_value(7.0, 0.0, 4), 0.0);
    assert_eq!(line_value(7.0, 1e-7, 4), 0.0);
    assert_eq!(line_value(7.0, -2.0, 4), -3.5);
    assert_eq!(
        [
            line_value(7.0, 2.0, 1),
            line_value(7.0, 2.0, 2),
            line_value(7.0, 2.0, 3),
            line_value(7.0, 2.0, 99)
        ],
        [9.0, 5.0, 14.0, 7.0]
    );
}
#[test]
fn gw_c5_second_button_range_selects_low_byte_and_returns_false() {
    let mut host = Trace::new();
    let mut owner = host.owner();
    assert_eq!(owner.window_message(&mut host, 99, [3, 0x287259f6, 0x4105]), Ok(false));
    assert_eq!(owner.graph_id, 5);
    assert!(owner.expanded);
    let selected: Vec<_> = host
        .selects
        .iter()
        .map(|(h, a)| (host.windows.get(*h).id, *a))
        .collect();
    assert_eq!(
        selected,
        [(0x4001, [1, 0]), (0x4101, [1, 0]), (0x4005, [1, 1]), (0x4105, [1, 1])]
    );
    host.filter = false;
    let count = host.selects.len();
    assert_eq!(owner.window_message(&mut host, 99, [3, 0x287259f6, 0x4001]), Ok(true));
    assert_eq!(host.selects.len(), count);
}
#[test]
fn section4_line_setup_legend_capture_and_pie_without_population() {
    let mut host = Trace::new();
    let mut owner = host.owner();
    owner.init(&mut host).unwrap();
    host.definitions.insert(
        1,
        Definition {
            resource: 999,
            kind: 1,
            keys: vec![1, 2, 3],
            colors: vec![[1, 2, 3]],
            ..Definition::default()
        },
    );
    owner.visibility.insert(2, 0);
    owner.show_graph(&mut host, 1).unwrap();
    assert_eq!(owner.series.values().len(), 3);
    assert_eq!(owner.series.values()[2].scale, 1.0);
    assert_eq!(owner.series.values()[2].color, [0; 3]);
    assert!(!owner.series.values()[1].visible);
    let boxes: Vec<_> = host
        .calls
        .iter()
        .filter_map(|c| match c {
            Call::Service(Service::CreateCheckbox { id, .. }) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(boxes, [0x04000000, 0x04000001, 0x04000002]);
    assert_eq!(host.titles.iter().map(|(_, index)| *index).collect::<Vec<_>>(), [0, 12]);
    assert!(host
        .graph_calls
        .iter()
        .any(|(_, slot, args)| *slot == 12 && *args == vec![Arg::Rect([45, 20, 290, 220])]));
    host.definitions.insert(
        3,
        Definition {
            resource: 999,
            kind: 3,
            name: "pie".into(),
            ..Definition::default()
        },
    );
    host.graph_calls.clear();
    owner.show_graph(&mut host, 3).unwrap();
    assert!(!host.graph_calls.iter().any(|(space, _, _)| *space != Space::Graph));
    assert!(owner.series.values().is_empty());
    assert!(host
        .calls
        .iter()
        .any(|c| matches!(c, Call::Window(_, WindowOp::DeleteChild(_)))));
}
#[test]
fn section5_bar_vectors_only_fill_their_group_and_make_unused_date() {
    let mut host = Trace::new();
    let mut owner = host.owner();
    owner.init(&mut host).unwrap();
    host.values.insert(1, 8.0);
    host.values.insert(2, 12.0);
    host.definitions.insert(
        2,
        Definition {
            resource: 999,
            kind: 2,
            keys: vec![1, 2],
            groups: vec![1, 0],
            scales: vec![0.5, 2.0],
            ..Definition::default()
        },
    );
    owner.show_graph(&mut host, 2).unwrap();
    let vectors: Vec<_> = host
        .graph_calls
        .iter()
        .filter(|(space, slot, _)| *space == Space::Bar && *slot == 21)
        .map(|(_, _, args)| args.clone())
        .collect();
    assert_eq!(
        vectors,
        [
            vec![Arg::Values(vec![0.0, 24.0]), Arg::U32(0)],
            vec![Arg::Values(vec![4.0, 0.0]), Arg::U32(1)]
        ]
    );
    assert!(host.calls.contains(&Call::Service(Service::MakeDate(940))));
}
#[test]
fn section5_start_date_strict_test_missing_history_and_contiguous_visible_lines() {
    let mut host = Trace::new();
    let mut owner = host.owner();
    let widget = host.create(0);
    owner.widget = Some(widget);
    owner.kind = 1;
    owner.axis = Axis::default();
    owner.series.resize(3);
    for (i, record) in owner.series.values.iter_mut().enumerate() {
        record.key = i as u32 + 1;
        record.scale = 1.0;
    }
    host.start = 660;
    host.values.insert(1, 7.0);
    let hidden = host.child(widget, 0x04000001);
    host.states.insert(hidden, 0);
    owner.refresh(&mut host).unwrap();
    let samples: Vec<_> = host
        .graph_calls
        .iter()
        .filter(|(space, slot, _)| *space == Space::Line && *slot == 20)
        .map(|(_, _, a)| a.clone())
        .collect();
    assert_eq!(samples.len(), 26);
    assert_eq!(samples[0], vec![Arg::Float(0.0), Arg::U32(0), Arg::U32(u32::MAX)]);
    assert_eq!(samples[2][0], Arg::Float(0.0));
    assert_eq!(samples[4][0], Arg::Float(7.0));
    assert_eq!(samples[5], vec![Arg::Float(0.0), Arg::U32(1), Arg::U32(u32::MAX)]);
    assert_eq!(owner.visibility.get(&2), Some(&0));
    assert!(!host
        .calls
        .iter()
        .any(|c| matches!(c, Call::Service(Service::History { key: 2, .. }))));
}
#[test]
fn section7_vector_growth_copy_strings_map_default_and_line_forwarding() {
    let mut records = SeriesRecords::default();
    records.resize(3);
    records.values[0].label = "owned".into();
    records.resize(4);
    assert_eq!(records.capacity(), 6);
    let mut copy = records.clone();
    copy.values[0].label.clear();
    assert_eq!(records.values()[0].label, "owned");
    records.resize(2);
    assert_eq!(records.capacity(), 6);
    let mut map = BTreeMap::<u32, u8>::new();
    assert_eq!(*map.entry(9).or_default(), 0);
    let mut host = Trace::new();
    antialiased_line(&mut host, [1, 2, 3, 4, 5, 6, 7]).unwrap();
    assert_eq!(
        host.calls.last(),
        Some(&Call::Service(Service::AntialiasedLine([1, 2, 3, 4, 5, 6, 7, 0])))
    );
}
#[test]
fn open_graph_contracts_are_explicit_unsupported_boundaries() {
    struct NoContracts;
    impl Host for NoContracts {
        fn call(&mut self, _: Call) -> Result<Value> {
            Ok(Value::Unit)
        }
        fn definition(&mut self, _: u32) -> Result<Option<Definition>> {
            Ok(None)
        }
    }
    let mut host = NoContracts;
    assert_eq!(
        host.graph(1, Space::Graph, 4, vec![]),
        Err(Unsupported::Open("GW1: graph widget slots"))
    );
    assert_eq!(
        host.axis_title(1, String::new(), 0),
        Err(Unsupported::Open("GW2: G17 second argument"))
    );
    assert_eq!(
        host.select_button(1, [1, 0]),
        Err(Unsupported::Open("GW3: SelectOption button slot 5"))
    );
    assert_eq!(
        host.line_control(1, true),
        Err(Unsupported::Open("GW4: line-only control 8A2871D8"))
    );
}
