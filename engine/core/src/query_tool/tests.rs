// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct Fake {
    events: Vec<Effect>,
    objects: BTreeMap<Token, Occupant>,
    lots: BTreeMap<Token, Lot>,
    reads: BTreeMap<Read, f64>,
    tree: BTreeMap<Token, Vec<Token>>,
    ids: BTreeMap<Token, u32>,
    values: BTreeMap<Token, Value>,
    properties: BTreeMap<Token, (u32, i64, Vec<u32>)>,
    picked: Option<Token>,
    automaton: Option<Token>,
    terrain: Option<Vec3>,
    zone: u32,
    no_view: bool,
    terrain_query: bool,
    water: bool,
    debug: bool,
    visible: bool,
    result: u32,
    scripts: Vec<Script>,
    active: Option<(Token, Subject)>,
    day: u32,
    categories: u32,
    occupant_categories: u32,
    fail_dialog: bool,
}
impl World for Fake {
    fn city_size(&self) -> i32 {
        64
    }
    fn occupant(&self, id: Token) -> Occupant {
        self.objects.get(&id).cloned().unwrap_or_default()
    }
    fn lot(&self, id: Token) -> Lot {
        self.lots.get(&id).cloned().unwrap_or_default()
    }
    fn lot_for_occupant(&self, id: Token) -> Option<Token> {
        self.lots
            .iter()
            .find(|(_, l)| l.building == Some(id) || l.occupants.contains(&id))
            .map(|(id, _)| *id)
    }
    fn lot_at(&self, _: Cell) -> Option<Token> {
        self.lots.keys().next().copied()
    }
    fn position_to_cell(&self, p: Vec3) -> Cell {
        Cell {
            x: p.x as i32,
            z: p.z as i32,
        }
    }
    fn read(&self, r: Read, _: Subject) -> f64 {
        *self.reads.get(&r).unwrap_or(&0.0)
    }
    fn zone(&self, _: Cell) -> u32 {
        self.zone
    }
    fn cell_has_water(&self, _: Cell) -> bool {
        self.water
    }
    fn localize(&self, g: u32, i: u32) -> String {
        format!("{g:08X}:{i:08X}")
    }
    fn money(&self, v: i64) -> String {
        format!("${v}")
    }
    fn decimal(&self, v: f64) -> String {
        format!("{v:.2}")
    }
    fn money_value(&self, r: Read, s: Subject) -> i64 {
        self.read(r, s) as i64
    }
    fn debug_query_enabled(&self) -> bool {
        self.debug
    }
}
impl ControlHost for Fake {
    fn lookup_cursor(&mut self, r: u32) -> Option<u64> {
        Some(u64::from(r))
    }
    fn apply_window_cursor(&mut self, _: u64, _: bool) {}
    fn am_capturing(&self) -> bool {
        false
    }
    fn capture(&mut self) -> bool {
        true
    }
    fn release_capture(&mut self) -> bool {
        true
    }
    fn remove_top_control(&mut self, _: bool) {}
    fn post_sound(&mut self, _: u32, _: u32) {}
}
impl Host for Fake {
    fn effect(&mut self, e: Effect) {
        match &e {
            Effect::Show(_) => self.visible = true,
            Effect::Hide(_) => self.visible = false,
            Effect::PublishDialog(v) => self.active = *v,
            Effect::SliderProperties(w, i, c, r) => {
                self.properties.insert(*w, (*i, *c, r.clone()));
            }
            Effect::Categories(v) => self.categories = *v,
            Effect::Highlight(o, v) => {
                self.objects.entry(*o).or_default().highlight = *v;
            }
            _ => {}
        }
        self.events.push(e);
    }
    fn has_view(&self) -> bool {
        !self.no_view
    }
    fn terrain_query_enabled(&self) -> bool {
        self.terrain_query
    }
    fn default_point(&self) -> Vec3 {
        Vec3::default()
    }
    fn pick(&mut self, _: Pixel, f: Filter, predicate: fn(Filter, &Occupant) -> bool) -> Option<Token> {
        let hit = if f == Filter::Automaton {
            self.automaton
        } else {
            self.picked
        };
        hit.filter(|id| predicate(f, &self.occupant(*id)))
    }
    fn pick_terrain(&mut self, _: Pixel) -> Option<Vec3> {
        self.terrain
    }
    fn create_callout(&mut self) -> Option<Token> {
        Some(90)
    }
    fn visible(&self, _: Token) -> bool {
        self.visible
    }
    fn cursor_position(&self, _: bool) -> Pixel {
        Pixel { x: 4, y: 5 }
    }
    fn day(&self) -> u32 {
        self.day
    }
    fn active_dialog(&self) -> Option<(Token, Subject)> {
        self.active
    }
    fn categories(&self) -> u32 {
        self.categories
    }
    fn occupant_categories(&self, _: Token) -> u32 {
        self.occupant_categories
    }
    fn create_dialog(&mut self, s: Script) -> Option<Token> {
        self.scripts.push(s);
        (!self.fail_dialog).then_some(100)
    }
    fn supports_proc(&self, _: Token) -> bool {
        true
    }
    fn children(&self, w: Token) -> Vec<Token> {
        self.tree.get(&w).cloned().unwrap_or_default()
    }
    fn control_id(&self, w: Token) -> u32 {
        self.ids.get(&w).copied().unwrap_or(0)
    }
    fn modal(&mut self, _: Token, _: Subject) -> u32 {
        self.result
    }
    fn control_value(&self, w: Token) -> Value {
        self.values.get(&w).cloned().unwrap_or(Value::Number(0.0))
    }
    fn slider_properties(&self, w: Token) -> (u32, i64, Vec<u32>) {
        self.properties.get(&w).cloned().unwrap_or_default()
    }
    fn child_by_id(&self, _: Token, id: u32) -> Option<Token> {
        self.ids.iter().find(|(_, i)| **i == id).map(|(w, _)| *w)
    }
}
fn tool(mode: Mode) -> QueryTool {
    QueryTool::new(mode, 50, Vec3::default())
}
fn building() -> Occupant {
    Occupant {
        kind: Kind::Building,
        ..Occupant::default()
    }
}
fn world() -> Fake {
    let mut f = Fake::default();
    f.objects.insert(1, building());
    f.lots.insert(
        10,
        Lot {
            building: Some(1),
            occupants: vec![1],
            ..Lot::default()
        },
    );
    f
}
fn has_text(f: &Fake, w: Token, text: &str) -> bool {
    f.events.contains(&Effect::Text(w, text.into()))
}

#[test]
fn ct6_creation_lifecycle_slots_and_holds() {
    for mode in [Mode::Query, Mode::Route] {
        let mut f = Fake::default();
        let mut q = tool(mode);
        assert_eq!(q.unused, 3);
        assert!(q.tip_allowed);
        assert!(!q.clicking);
        assert_eq!(
            Mode::from_command(if mode == Mode::Query { 0x6A93_5CF3 } else { 0x6A93_5CF4 }, mode.id()),
            None
        );
        q.init(&mut f);
        q.init(&mut f);
        assert_eq!(f.events.iter().filter(|e| **e == Effect::AcquireView).count(), 1);
        assert_eq!(
            f.events
                .iter()
                .filter_map(|e| if let Effect::Subscribe(v) = e { Some(*v) } else { None })
                .collect::<Vec<_>>(),
            MESSAGES
        );
        assert!(f.events.contains(&Effect::AcquireCursor(u64::from(mode.cursor()))));
        assert_eq!(q.should_stack(), true);
        assert!(!q.unhandled_input());
        assert!(!q.is_self_scrolling());
        assert_eq!(
            f.events.contains(&Effect::Unsettled(Open::LegendCt7)),
            mode == Mode::Route
        );
        q.shutdown(&mut f);
        q.shutdown(&mut f);
        assert_eq!(
            f.events
                .iter()
                .filter_map(|e| if let Effect::Unsubscribe(v) = e { Some(*v) } else { None })
                .collect::<Vec<_>>(),
            MESSAGES
        );
        let n = f.events.len();
        assert_eq!(
            &f.events[n - 3..],
            &[
                Effect::ReleaseWindowManager,
                Effect::ReleaseView,
                Effect::ReleaseCursor(u64::from(mode.cursor()))
            ]
        );
        q.destroy(&mut f);
        assert_eq!(f.events.last(), Some(&Effect::DestroyTipTimer));
    }
}
#[test]
fn ct7_filter_truth_table_and_names() {
    assert!(accepts(
        Filter::Hover,
        &Occupant {
            kind: Kind::PowerPole,
            ..Default::default()
        }
    ));
    let mut prop = Occupant {
        kind: Kind::Prop,
        ..Default::default()
    };
    assert!(!accepts(Filter::Hover, &prop));
    prop.properties.insert(0x8A41_6A99, Property::Key(3, 4));
    assert!(accepts(Filter::Hover, &prop));
    let f = Fake::default();
    assert_eq!(tips::display_name(&f, &prop), "00000003:00000004");
    prop.properties.insert(0x899A_FBAD, Property::Text("direct".into()));
    assert_eq!(tips::display_name(&f, &prop), "direct");
    prop.properties.insert(0xCAA9_AB92, Property::Bool(false));
    assert!(!accepts(Filter::Describe, &prop));
    for t in 0..=12 {
        assert_eq!(
            accepts(
                Filter::Route,
                &Occupant {
                    network_interface: true,
                    network_types: vec![t],
                    ..Default::default()
                }
            ),
            t != 4 && t != 5
        );
    }
    prop.groups = vec![0x5002];
    assert!(accepts(Filter::Stoplight, &prop));
}
#[test]
fn ct8_hover_restore_exit_and_shutdown_message() {
    let mut f = world();
    let mut q = tool(Mode::Query);
    q.init(&mut f);
    f.objects.get_mut(&1).unwrap().highlight = 3;
    f.picked = Some(1);
    q.mouse_move(&mut f, Pixel::default());
    assert_eq!(q.saved_highlight, 3);
    assert_eq!(q.highlighted, Some(1));
    assert!(q.queriable);
    f.objects.insert(
        2,
        Occupant {
            highlight: 5,
            ..building()
        },
    );
    f.picked = Some(2);
    q.mouse_move(&mut f, Pixel::default());
    assert_eq!(f.objects[&1].highlight, 3);
    assert_eq!(f.objects[&2].highlight, 7);
    q.message(&mut f, 0x26D3_1EC2);
    let n = f.events.len();
    q.message(&mut f, MESSAGES[0]);
    assert_eq!(n, f.events.len());
    q.mouse_move(&mut f, Pixel::default());
    assert!(q.tip_allowed);
    q.mouse_exit(&mut f);
    assert_eq!(q.highlighted, None);
    assert_eq!(f.objects[&2].highlight, 5);
    assert!(!q.queriable);
}
#[test]
fn hover_property_lot_highlight_and_terrain_fallback() {
    let mut f = world();
    let mut q = tool(Mode::Query);
    let mut p = Occupant {
        kind: Kind::Prop,
        ..Default::default()
    };
    p.properties.insert(0x6A95_E503, Property::Bool(true));
    p.properties.insert(0x8A41_6A99, Property::Text("p".into()));
    f.objects.insert(2, p);
    f.lots.get_mut(&10).unwrap().occupants.push(2);
    f.picked = Some(2);
    q.mouse_move(&mut f, Pixel::default());
    assert_eq!(q.highlighted, Some(1));
    assert_eq!(f.objects[&2].highlight, 7);
    q.mouse_exit(&mut f);
    assert_eq!(f.objects[&2].highlight, 0);
    f.picked = None;
    f.terrain = Some(Vec3 { x: 1.0, y: 2.0, z: 3.0 });
    q.mouse_move(&mut f, Pixel::default());
    assert_eq!(q.highlighted, None);
    assert!(q.queriable);
    assert_eq!(q.point, f.terrain.unwrap());
    f.objects.get_mut(&1).unwrap().groups = vec![0x1006];
    q.mouse_move(&mut f, Pixel::default());
    assert_eq!(q.highlighted, Some(1));
    q.clicking = true;
    f.events.clear();
    q.mouse_move(&mut f, Pixel::default());
    assert!(!f.events.iter().any(|e| matches!(e, Effect::StartTipTimer(_))));
}
#[test]
fn ct9_tips_ratio_terrain_network_and_display() {
    for (u, c, v) in [(80, 100, 1.0), (90, 100, 0.5), (120, 100, 0.0), (5, 0, 0.0)] {
        assert!((load_ratio(u, c) - v).abs() < 0.00001);
    }
    let mut f = Fake::default();
    let mut q = tool(Mode::Query);
    let mut t = Tip::default();
    q.terrain_tip(&f, &mut t);
    assert_eq!(t.title, "EA5524EB:AA5BBDE4");
    assert!(t.body.is_empty());
    f.water = true;
    let original = t.clone();
    q.terrain_tip(&f, &mut t);
    assert_eq!(t, original);
    f.objects.insert(
        2,
        Occupant {
            network_interface: true,
            network_types: vec![0],
            ..Default::default()
        },
    );
    q.network_tip(&f, 2, &mut t);
    assert_eq!(t.title, "EA5524EB:6A5589D4");
    q.callout = Some(90);
    q.display_tip(&mut f, &t);
    assert!(f.events.contains(&Effect::Hide(90)));
    assert!(f.events.iter().any(|e| matches!(e, Effect::CursorText(_, _, _))));
    q.set_tip(&mut f, "", "");
    assert_eq!(f.events.last(), Some(&Effect::Hide(90)));
    t.icon = 7;
    q.display_tip(&mut f, &t);
    assert!(f.events.contains(&Effect::CalloutImages(90, 7, 0, 0.5)));
    assert!(f.visible);
}
#[test]
fn building_tip_group_priority_utilities_and_headline_limit() {
    let mut f = world();
    let q = tool(Mode::Query);
    f.objects
        .get_mut(&1)
        .unwrap()
        .properties
        .insert(0xAA1D_D396, Property::Words(vec![0x1500, 0x1502]));
    f.reads.insert(Read::PoliceEfficiency, 0.7);
    f.lots.get_mut(&10).unwrap().headlines = vec!["a".into(), "b".into(), "c".into(), "d".into()];
    let mut t = Tip::default();
    q.building_tip(&mut f, 1, &mut t);
    assert_eq!(t.icon, 0x1441_6207);
    assert_eq!(t.fraction, 0.7);
    assert_eq!(t.body, "a\nb\nc");
    f.objects
        .get_mut(&1)
        .unwrap()
        .properties
        .insert(0xAA1D_D396, Property::Words(vec![0x1004, 0x1402]));
    f.reads.insert(Read::RecyclingMaximum, 0.0);
    f.reads.insert(Read::RecyclingCurrent, 1.0);
    q.building_tip(&mut f, 1, &mut t);
    assert_eq!(t.fraction, 1.0);
    f.objects
        .get_mut(&1)
        .unwrap()
        .properties
        .insert(0xAA1D_D396, Property::Bool(true));
    t = Tip::default();
    q.building_tip(&mut f, 1, &mut t);
    assert_eq!(t.icon, 0);
}
#[test]
fn ct10_modal_script_walk_cancel_coverage_and_ownership() {
    let mut f = world();
    let q = tool(Mode::Query);
    f.objects
        .get_mut(&1)
        .unwrap()
        .properties
        .insert(0x2A49_9F85, Property::U32(123));
    f.categories = 3;
    f.occupant_categories = 9;
    f.result = 1;
    f.tree.insert(100, vec![11, 12, 13]);
    f.tree.insert(12, vec![14]);
    f.tree.insert(13, vec![15]);
    for (w, id) in [(11, 0x1001), (12, 0x1005), (13, 5), (14, 0x1001), (15, 0x1001)] {
        f.ids.insert(w, id);
    }
    f.reads.insert(Read::FireEfficiency, 0.75);
    assert!(!q.do_query_dialog(&mut f, 1));
    assert_eq!(f.scripts, vec![Script::query(123)]);
    assert_eq!(f.categories, 3);
    for w in [11, 14, 15] {
        assert!(f.events.contains(&Effect::Meter(w, 0.75)));
    }
    assert!(!f.events.iter().any(|e| matches!(e, Effect::Write(..))));
    assert!(f.events.contains(&Effect::CoverageTimer(true, 330)));
    assert!(f.events.contains(&Effect::CoverageTimer(false, 330)));
    for id in [1, 100] {
        assert_eq!(
            f.events.iter().filter(|e| **e == Effect::Acquire(id)).count() + usize::from(id == 100),
            f.events.iter().filter(|e| **e == Effect::Release(id)).count()
        );
    }
    assert!(f.active.is_none());
}
#[test]
fn click_sound_script_zone_and_shift_miss() {
    let mut f = world();
    let mut q = tool(Mode::Query);
    f.picked = Some(1);
    f.terrain = Some(Vec3::default());
    let p = &mut f.objects.get_mut(&1).unwrap().properties;
    p.insert(0x2A49_9F85, Property::U32(77));
    p.insert(0xAA1D_D397, Property::U32(88));
    let out = q.describe_pick(&mut f, Pixel::default(), false);
    assert_eq!(out.extra2, "[Building ] [Lot: _00000000]\n");
    assert!(f.events.contains(&Effect::Sound(88, Some(1))));
    assert_eq!(f.scripts[0], Script::query(77));
    f.scripts.clear();
    q.mouse_down(&mut f, Pixel::default(), 1);
    assert_eq!(f.scripts.len(), 1);
    assert!(f.events.contains(&Effect::PostAt(0x2ACD_2CF3, Pixel::default())));
    f.picked = None;
    f.lots.clear();
    for (zone, instance) in [(11, 0x6A56_1B3A), (12, 0x4A56_2DA5), (14, 0xAA56_1F93)] {
        f.zone = zone;
        q.describe_pick(&mut f, Pixel::default(), false);
        assert_eq!(f.scripts.last(), Some(&Script::query(instance)));
    }
    f.zone = 0;
    f.events.clear();
    q.describe_pick(&mut f, Pixel::default(), false);
    assert!(f.events.contains(&Effect::Sound(0xAA62_C9D5, None)));
    f.no_view = true;
    assert_eq!(q.describe_pick(&mut f, Pixel::default(), false), Readout::default());
}
#[test]
fn conformance_rating_and_live_funding_slider() {
    assert_eq!(rating_level(33.2), 0x4A5F_E266);
    assert_eq!(rating_level(33.3), 0x0A5F_E260);
    assert_eq!(rating_level(66.6), 0xAA5F_E249);
    let mut f = world();
    f.ids.insert(11, 0x1002);
    f.values.insert(11, Value::Number(80.0));
    f.properties.insert(11, (0x1002, 123, vec![]));
    assert!(dialog_notification(
        &mut f,
        100,
        Subject::Occupant(1),
        Notification {
            message_type: 3,
            from_dialog: true,
            code: 0x8871_13A3,
            value: 0,
            slider: Some(11)
        }
    ));
    assert!(f.events.contains(&Effect::Write(
        Subject::Occupant(1),
        Write::FireFunding,
        Value::Number(0.8)
    )));
    assert!(f.events.contains(&Effect::MoneyChild(11, 0x1102, "$98".into())));
}
#[test]
fn filler_table_all_slots_and_set_pass() {
    let mut f = world();
    let s = Subject::Occupant(1);
    for i in 0..127 {
        let expected = matches!(i,0..=4|6..=37|41..=71|73..=74|81..=92|94..=99|107..=109|115..=126);
        assert_eq!(fill_control(&mut f, 11, 0x1000 | i, true, s), expected, "slot {i}");
        f.events.clear();
        assert_eq!(fill_control(&mut f, 11, 0x1000 | i, false, s), expected, "set {i}");
        if !matches!(i, 0 | 2 | 9 | 24 | 25 | 33 | 34 | 99 | 122) {
            assert!(f.events.is_empty(), "set mutated slot {i}");
        }
    }
    assert_eq!(filler_index(0x107F), None);
    assert_eq!(filler_index(1), None);
}
#[test]
fn filler_numeric_formulas_and_zero_divisors() {
    let mut f = world();
    let s = Subject::Occupant(1);
    for (r, v) in [
        (Read::JailCapacity, 100.0),
        (Read::JailedFraction, 0.9),
        (Read::PowerUsed, 25.0),
        (Read::PowerProduced, 50.0),
        (Read::PowerCost, 125.0),
        (Read::WaterCurrent, 50.0),
        (Read::WaterUsed, 25.0),
        (Read::WaterCost, 75.0),
    ] {
        f.reads.insert(r, v);
    }
    for (id, text) in [
        (0x100D, "80"),
        (0x100E, "90"),
        (0x1035, "50%"),
        (0x1039, "2.50"),
        (0x103B, "50%"),
        (0x103F, "1.50"),
    ] {
        fill_control(&mut f, 11, id, true, s);
        assert!(has_text(&f, 11, text), "id {id:x}");
    }
    fill_control(&mut f, 11, 0x100C, true, s);
    assert!(f
        .events
        .iter()
        .any(|e| matches!(e,Effect::Meter(11,v) if (*v-0.5).abs()<0.00001)));
    f.reads.clear();
    for id in [0x102A, 0x102E, 0x103B, 0x1042, 0x105F] {
        f.events.clear();
        fill_control(&mut f, 11, id, true, s);
        assert!(has_text(&f, 11, "0%"));
    }
}
#[test]
fn building_values_edit_toggle_and_cost_sum() {
    let mut f = world();
    let s = Subject::Occupant(1);
    let l = f.lots.get_mut(&10).unwrap();
    l.capacity = 100;
    l.building_type = 1;
    l.occupancy = 0.75;
    assert_eq!(current_occupancy(&f, 1), 75);
    fill_control(&mut f, 11, 0x1051, true, s);
    assert!(has_text(&f, 11, "75/100"));
    f.reads.insert(Read::Aura, 127.0);
    assert_eq!(mayor_percent(&f, 1), 100.0);
    f.values.insert(11, Value::Text("renamed".into()));
    fill_control(&mut f, 11, 0x1000, false, s);
    assert!(f
        .events
        .contains(&Effect::Write(s, Write::Name, Value::Text("renamed".into()))));
    f.values.insert(11, Value::Bool(true));
    fill_control(&mut f, 11, 0x107A, false, s);
    assert!(f
        .events
        .contains(&Effect::Write(s, Write::Historical, Value::Bool(true))));
    f.objects
        .get_mut(&1)
        .unwrap()
        .properties
        .insert(0xEA54_D285, Property::Words(vec![7, 8]));
    f.reads.insert(Read::BudgetCost(7), 12.0);
    f.reads.insert(Read::BudgetCost(8), 13.0);
    fill_control(&mut f, 11, 0x106C, true, s);
    assert!(has_text(&f, 11, "$25"));
}
#[test]
fn messages_callbacks_and_route_open_boundary() {
    let mut f = world();
    let mut q = tool(Mode::Route);
    q.highlighted = Some(1);
    q.get_occupant_paths(&mut f);
    assert!(q.path_rect.is_some());
    assert!(f.events.contains(&Effect::Unsettled(Open::RouteCt15)));
    q.path_view = Some(200);
    q.message(&mut f, 0x6924_7DC7);
    assert!(f.events.contains(&Effect::ClearPaths(200)));
    q.message(&mut f, MESSAGES[1]);
    assert!(f.events.contains(&Effect::RecalcCoverage));
    QueryTool::tip_callback(&mut f);
    QueryTool::coverage_callback(&mut f);
    assert!(f.events.contains(&Effect::SendTipMessage(MESSAGES[0])));
    assert!(f.events.contains(&Effect::PostMessage(MESSAGES[1])));
    f.active = Some((100, Subject::Occupant(1)));
    f.tree.insert(100, vec![11]);
    f.ids.insert(11, 0x1001);
    f.day = 9;
    f.events.clear();
    q.message(&mut f, 0x6695_6814);
    assert!(f.events.is_empty());
    f.day = 10;
    q.message(&mut f, 0x6695_6814);
    assert!(f.events.contains(&Effect::Meter(11, 0.0)));
}

#[test]
fn filler_source_matrix() {
    // Distinct values catch swapped service selectors and aliases, independently
    // of the table-presence test. Formula fields have separate fixtures.
    let cases: &[(u32, Read, f64, &str)] = &[
        (0x1003, Read::FireDispatch, 7.0, "7"),
        (0x1004, Read::FireCost, 21.0, "$21"),
        (0x1007, Read::Arrests, 8.0, "8"),
        (0x1008, Read::Crimes, 9.0, "9"),
        (0x100A, Read::PoliceDispatch, 6.0, "6"),
        (0x100B, Read::PoliceCost, 22.0, "$22"),
        (0x100F, Read::JailCost, 23.0, "$23"),
        (0x1013, Read::AirportCost, 24.0, "$24"),
        (0x1016, Read::PollutionWater, 10.0, "10"),
        (0x101A, Read::EstimatedStaff, 11.0, "11"),
        (0x1023, Read::EstimatedStaff, 12.0, "12"),
        (0x101B, Read::EstimatedCapacity, 13.0, "13"),
        (0x101D, Read::EstimatedCapacity, 14.0, "14"),
        (0x101F, Read::EstimatedCapacity, 15.0, "15"),
        (0x1024, Read::EstimatedCapacity, 16.0, "16"),
        (0x101C, Read::SchoolServed, 17.0, "17"),
        (0x101E, Read::SchoolServed, 18.0, "18"),
        (0x1025, Read::HospitalServed, 19.0, "19"),
        (0x102C, Read::PollutionAir, 20.0, "20"),
        (0x102F, Read::PowerMaximum, 31.0, "31"),
        (0x1036, Read::PowerMaximum, 32.0, "32"),
        (0x1030, Read::PowerProduced, 33.0, "33"),
        (0x1037, Read::PowerProduced, 34.0, "34"),
        (0x1031, Read::PowerUsed, 35.0, "35"),
        (0x1038, Read::PowerUsed, 36.0, "36"),
        (0x103C, Read::WaterMaximum, 37.0, "37"),
        (0x103D, Read::WaterCurrent, 38.0, "38"),
        (0x103E, Read::WaterUsed, 39.0, "39"),
        (0x1043, Read::StationCost, 40.0, "$40"),
        (0x1044, Read::StationUsage, 41.0, "41"),
        (0x1045, Read::StationMaximum, 42.0, "42"),
        (0x1046, Read::TransitFunding, 0.8, "80%"),
        (0x1047, Read::TransitLocalCost, 43.0, "$43"),
        (0x1078, Read::SeaportCost, 44.0, "$44"),
        (0x107B, Read::AirportUsage, 45.0, "45"),
        (0x107C, Read::AirportCapacity, 46.0, "46"),
        (0x107D, Read::SeaportUsage, 47.0, "47"),
        (0x107E, Read::SeaportCapacity, 48.0, "48"),
        (0x1061, Read::PowerCondition, 0.73, "73%"),
        (0x1062, Read::PowerCondition, 0.74, "74%"),
        (0x1073, Read::WaterCondition, 0.75, "75%"),
    ];
    for (id, r, v, text) in cases {
        let mut f = world();
        f.reads.insert(*r, *v);
        assert!(fill_control(&mut f, 11, *id, true, Subject::Occupant(1)));
        assert!(has_text(&f, 11, text), "{id:x}: {:?}", f.events);
    }
    for (id, r) in [
        (0x1001, Read::FireEfficiency),
        (0x1006, Read::PoliceEfficiency),
        (0x1017, Read::SchoolGrade),
        (0x1020, Read::HospitalGrade),
        (0x102D, Read::PowerEfficiency),
        (0x1034, Read::PowerEfficiency),
        (0x103A, Read::WaterEfficiency),
        (0x106D, Read::WaterEfficiency),
    ] {
        let mut f = world();
        f.reads.insert(r, 0.625);
        fill_control(&mut f, 11, id, true, Subject::Occupant(1));
        assert!(f.events.contains(&Effect::Meter(11, 0.625)), "{id:x}");
    }
}
#[test]
fn terrain_zones_and_network_priority() {
    let mut f = Fake::default();
    let q = tool(Mode::Query);
    for z in 0..=16 {
        f.zone = z;
        let mut t = Tip::default();
        q.terrain_tip(&f, &mut t);
        assert_eq!(t.body.is_empty(), matches!(z, 0 | 15 | 16));
        assert!(!t.title.is_empty());
    }
    f.objects.insert(
        1,
        Occupant {
            network_interface: true,
            network_types: vec![0, 2, 12],
            ..Default::default()
        },
    );
    let mut t = Tip::default();
    q.network_tip(&f, 1, &mut t);
    assert_eq!(t.title, "EA5524EB:AA5589FB");
    f.objects.get_mut(&1).unwrap().network_types = vec![4];
    f.debug = true;
    f.reads.insert(Read::PipeDistressed, 2.0);
    f.reads.insert(Read::PipeCount, 10.0);
    q.network_tip(&f, 1, &mut t);
    assert_eq!(t.body, "Distressed Pipes: 2 / 10\n");
}
#[test]
fn building_tip_rows_and_transport() {
    let rows = [
        (0x1000, 0x1441_6208),
        (0x1001, 0x1441_6202),
        (0x1002, 0x1441_6206),
        (0x1003, 0x1441_6209),
        (0x1004, 0x1441_620A),
        (0x1501, 0x1441_6207),
        (0x1500, 0x1441_6207),
        (0x1502, 0x1441_6203),
        (0x1503, 0x1441_6201),
        (0x1504, 0x1441_6201),
        (0x1505, 0x1441_6201),
        (0x1506, 0x1441_6201),
        (0x1507, 0x1441_6201),
        (0x150A, 0x1441_620C),
        (0x150B, 0x1441_620C),
        (0x1006, 0x1441_6204),
        (0xFFFF, 0x1441_6200),
    ];
    let q = tool(Mode::Query);
    for (g, icon) in rows {
        let mut f = world();
        f.objects
            .get_mut(&1)
            .unwrap()
            .properties
            .insert(0xAA1D_D396, Property::Words(vec![g]));
        let mut t = Tip::default();
        q.building_tip(&mut f, 1, &mut t);
        assert_eq!(t.icon, icon);
    }
    let mut f = world();
    f.objects
        .get_mut(&1)
        .unwrap()
        .properties
        .insert(0xAA1D_D396, Property::Words(vec![0x1003, 0x1508]));
    f.reads.insert(Read::AirportUsage, 90.0);
    f.reads.insert(Read::AirportCapacity, 100.0);
    let mut t = Tip::default();
    q.building_tip(&mut f, 1, &mut t);
    assert_eq!(t.secondary, 0x1441_6214);
    assert!((t.fraction - 0.5).abs() < 0.00001);
    f.objects
        .get_mut(&1)
        .unwrap()
        .properties
        .insert(0xAA1D_D396, Property::Words(vec![0x1000]));
    f.reads.insert(Read::DeveloperType, 0x1020 as f64);
    q.building_tip(&mut f, 1, &mut t);
    assert_eq!(t.body, "EA5524EB:6A95F2B0");
}
#[test]
fn click_modifiers_and_route_latch() {
    let mut f = world();
    let mut q = tool(Mode::Query);
    f.automaton = Some(2);
    f.objects.insert(
        2,
        Occupant {
            vehicle: true,
            ..Default::default()
        },
    );
    f.terrain = Some(Vec3 {
        x: -7.0,
        y: 0.0,
        z: 9.0,
    });
    q.mouse_down(&mut f, Pixel::default(), 3);
    assert!(f.events.contains(&Effect::ModifierHit(2, Open::ModifierActionCt9)));
    assert!(f.events.contains(&Effect::Residential59(Cell { x: -1, z: 2 })));
    assert!(f.scripts.is_empty());
    assert!(!q.clicking);
    q = tool(Mode::Route);
    q.path_view = Some(200);
    q.highlighted = Some(1);
    q.mouse_down(&mut f, Pixel::default(), 0);
    assert!(q.clicking);
    assert!(f.events.contains(&Effect::ShutdownPath(200)));
    q.mouse_up(&mut f);
    assert!(!q.clicking);
}
#[test]
fn accepted_dialog_set_pass_and_failures() {
    let mut f = world();
    let q = tool(Mode::Query);
    assert!(!q.do_query_dialog(&mut f, 1));
    f.objects
        .get_mut(&1)
        .unwrap()
        .properties
        .insert(0x2A49_9F85, Property::U32(12));
    f.fail_dialog = true;
    assert!(!q.do_query_dialog(&mut f, 1));
    assert!(!f.events.iter().any(|e| matches!(e, Effect::PublishDialog(_))));
    f.fail_dialog = false;
    f.tree.insert(100, vec![11]);
    f.ids.insert(11, 0x1000);
    f.values.insert(11, Value::Text("new name".into()));
    assert!(q.do_query_dialog(&mut f, 1));
    assert!(f.events.contains(&Effect::Write(
        Subject::Occupant(1),
        Write::Name,
        Value::Text("new name".into())
    )));
}
#[test]
fn dialog_notifications_and_condition_sounds() {
    let mut f = world();
    let q = tool(Mode::Query);
    let p = &mut f.objects.get_mut(&1).unwrap().properties;
    p.insert(0x0A90_2434, Property::U32(1));
    p.insert(0xAA90_5AB9, Property::U32(2));
    p.insert(0xAA1D_D397, Property::U32(3));
    for (condition, sound) in [(4, 1), (3, 2), (2, 2), (0, 3)] {
        f.lots.get_mut(&10).unwrap().condition = condition;
        q.play_occupant_sound(&mut f, 1);
        assert_eq!(f.events.last(), Some(&Effect::Sound(sound, Some(1))));
    }
    for value in 0..3 {
        assert_eq!(
            dialog_notification(
                &mut f,
                100,
                Subject::Occupant(1),
                Notification {
                    message_type: 3,
                    from_dialog: true,
                    code: 0x2872_59F6,
                    value,
                    slider: None
                }
            ),
            value < 2
        );
    }
    assert!(!dialog_notification(
        &mut f,
        100,
        Subject::Occupant(1),
        Notification {
            message_type: 2,
            from_dialog: true,
            code: 0x2872_59F6,
            value: 0,
            slider: None
        }
    ));
}
#[test]
fn building_value_thresholds() {
    let mut f = world();
    let s = Subject::Occupant(1);
    f.lots.get_mut(&10).unwrap().fallback_capacity = 35;
    assert_eq!(occupant_capacity(&f, 1), 35);
    f.reads.insert(Read::RatingMinimum(1), 10.0);
    f.reads.insert(Read::RatingMaximum(1), 20.0);
    for (value, id) in [(9.0, 0xAA5F_E249), (10.0, 0x0A5F_E260), (20.0, 0x4A5F_E266)] {
        f.reads.insert(Read::RatingCurve(1, 0), value);
        assert_eq!(subjective_level(&f, s, 1), id);
    }
    assert_eq!(goofy_stat(&[(256.0, 256.0)], -1.0), Some(256));
    assert_eq!(goofy_stat(&[(0.0, 0.0), (1.0, 1.0)], 0.5), None);
}
#[test]
fn network_describe_format() {
    let mut f = Fake::default();
    f.picked = Some(1);
    f.objects.insert(
        1,
        Occupant {
            kind: Kind::Network,
            network_interface: true,
            network_types: vec![0],
            network_tile: Some(NetworkTile {
                id: 0x1234,
                rotation: 2,
                flipped: 1,
            }),
            ..Default::default()
        },
    );
    f.reads.insert(Read::TrafficCapacity2, 100.0);
    f.reads.insert(Read::TrafficVolumeByte, 3.0);
    let out = tool(Mode::Query).describe_pick(&mut f, Pixel::default(), false);
    assert!(out.text.contains("Piece Id: 00001234, Rotation:2, Flipped:1"));
    assert!(out.text.contains("cap(100) vol(30) cong(0.300000)"));
    assert!(out.open.contains(&Open::DebugWritersCt14));
    f.objects.get_mut(&1).unwrap().cell = Cell { x: 5, z: 5 };
    f.events.clear();
    assert_eq!(tool(Mode::Query).do_neighbor_dialog(&mut f, 1), Ok(false));
    assert!(f.events.is_empty());
}
