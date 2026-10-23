// SPDX-License-Identifier: GPL-3.0-or-later
use super::placement::{cell_coverage, choose_facing, CREATION_MESSAGES};
use super::*;
use std::collections::{BTreeMap, VecDeque};

#[path = "closure_tests.rs"]
mod closure;

#[derive(Debug, PartialEq)]
enum Call {
    Effect(Effect),
    Cursor(u64, bool),
    Sound(u32, u32),
    End(bool),
    Filter(Filter),
    Pick(String, Vec<(u32, String)>, u32),
    Development(Rect),
    CreateTest(Rect, u8, u32),
    Demolish(Demolition),
    Network(i32, i32, u32),
    Zone(Zone),
    Lot(Rect, u8, bool, bool, u32),
    Build(u64, u32, u32, u32),
    Key(u32, u32, u32),
    EffectCreate(String),
    TollNetwork(i32, i32, u32, bool),
    PreviewBuilding(u32),
    PreviewProp(u32),
}
struct Fake {
    calls: Vec<Call>,
    cfg: Option<Configuration>,
    candidates: Vec<u32>,
    picks: VecDeque<Option<u32>>,
    hit: Option<Vec3>,
    top: bool,
    command: Option<u32>,
    price: Option<i64>,
    development: i64,
    demolition: (bool, i64),
    codes: [i32; 4],
    flags: BTreeMap<(i32, i32), u32>,
    groups: Vec<u32>,
    occupant: BuildingInfo,
    funds: i64,
    task: Option<Rect>,
    lot: Option<u64>,
    builder: bool,
    hidden: bool,
    wm: bool,
    entries: Vec<Entry>,
    networks: BTreeMap<(i32, i32), Network>,
    visual: bool,
    next_effect: u64,
    running: Vec<u64>,
    preview: PreviewInfo,
    radii: BTreeMap<u32, (f32, u32)>,
    zone_results: VecDeque<bool>,
    profile: bool,
    candidates_building: Vec<u32>,
    building_entry: Option<Entry>,
    prop_allowed: bool,
    city_hidden: bool,
    building_creation: bool,
    denied_props: Vec<u64>,
    cell_width: f32,
}
impl Default for Fake {
    fn default() -> Self {
        Self {
            calls: vec![],
            cfg: Some(Configuration {
                size: [2, 2],
                first_building: 7,
                water_compatible: false,
                construction: 123,
            }),
            candidates: vec![10, 11],
            picks: VecDeque::new(),
            hit: Some(point(40.0, 40.0)),
            top: true,
            command: None,
            price: Some(500),
            development: 25,
            demolition: (true, 10),
            codes: [8; 4],
            flags: BTreeMap::new(),
            groups: vec![],
            occupant: BuildingInfo::default(),
            funds: 10000,
            task: None,
            lot: Some(99),
            builder: true,
            hidden: false,
            wm: true,
            entries: Vec::new(),
            networks: BTreeMap::new(),
            visual: false,
            next_effect: 300,
            running: vec![200],
            preview: PreviewInfo::default(),
            radii: BTreeMap::new(),
            zone_results: VecDeque::new(),
            profile: true,
            candidates_building: vec![7],
            building_entry: Some(Entry::default()),
            prop_allowed: true,
            city_hidden: false,
            building_creation: true,
            denied_props: Vec::new(),
            cell_width: 16.0,
        }
    }
}
fn point(x: f32, z: f32) -> Vec3 {
    Vec3 { x, y: 3.0, z }
}
fn lot_point() -> Vec3 {
    Vec3 {
        x: 48.0,
        y: 30.0,
        z: 48.0,
    }
}
fn mouse() -> Pixel {
    Pixel { x: 10, y: 20 }
}
fn rect() -> Rect {
    Rect {
        x0: 2,
        z0: 2,
        x1: 3,
        z1: 3,
    }
}
impl ControlHost for Fake {
    fn lookup_cursor(&mut self, resource: u32) -> Option<u64> {
        if resource == CURSOR || resource == 0x417E_E36B {
            Some(if resource == CURSOR { 8 } else { 9 })
        } else {
            None
        }
    }
    fn apply_window_cursor(&mut self, cursor: u64, force: bool) {
        self.calls.push(Call::Cursor(cursor, force));
    }
    fn am_capturing(&self) -> bool {
        false
    }
    fn capture(&mut self) -> bool {
        true
    }
    fn release_capture(&mut self) -> bool {
        true
    }
    fn remove_top_control(&mut self, stack: bool) {
        self.calls.push(Call::End(stack));
    }
    fn post_sound(&mut self, event: u32, sound: u32) {
        self.calls.push(Call::Sound(event, sound));
    }
}
impl Host for Fake {
    fn effect(&mut self, effect: Effect) {
        self.calls.push(Call::Effect(effect));
    }
    fn has_view(&self) -> bool {
        true
    }
    fn on_top(&self) -> bool {
        self.top
    }
    fn translate_key(&mut self, key: u32, modifiers: u32, context: u32) -> Option<u32> {
        self.calls.push(Call::Key(key, modifiers, context));
        self.command
    }
    fn configuration(&mut self, _: u32) -> Option<Configuration> {
        self.cfg.clone()
    }
    fn configurations(&mut self, filter: Filter) -> Vec<u32> {
        self.calls.push(Call::Filter(filter));
        self.candidates.clone()
    }
    fn building_types(&mut self) -> Vec<u32> {
        vec![7, 8]
    }
    fn last_building(&self) -> u32 {
        0
    }
    fn last_configuration(&self) -> u32 {
        11
    }
    fn pick_list(&mut self, title: &str, rows: &[(u32, String)], initial: u32) -> Option<u32> {
        self.calls.push(Call::Pick(title.into(), rows.to_vec(), initial));
        self.picks.pop_front().unwrap_or(Some(initial))
    }
    fn plop_cost(&mut self, _: u32) -> Option<i64> {
        self.price
    }
    fn occupant_info(&mut self, token: u64) -> BuildingInfo {
        if token == 90 {
            BuildingInfo {
                groups: self.groups.clone(),
                ..BuildingInfo::default()
            }
        } else {
            self.occupant.clone()
        }
    }
    fn terrain(&self) -> Terrain {
        Terrain {
            cell_width: self.cell_width,
            width: 32,
            height: 32,
        }
    }
    fn pick_terrain(&mut self, _: Pixel) -> Option<Vec3> {
        self.hit
    }
    fn terrain_grid_hidden(&self) -> bool {
        self.hidden
    }
    fn localize(&mut self, g: u32, i: u32) -> String {
        format!("{g:08x}:{i:08x}")
    }
    fn development_cost(&mut self, r: Rect) -> i64 {
        self.calls.push(Call::Development(r));
        self.development
    }
    fn can_create(&mut self, r: Rect, rot: u8, cfg: u32) -> i32 {
        self.calls.push(Call::CreateTest(r, rot, cfg));
        self.codes[usize::from(rot)]
    }
    fn demolish(&mut self, request: Demolition, filter: fn(u32) -> bool) -> (bool, i64) {
        assert!(!filter(0x2890_D4DE));
        assert!(!filter(0x0990_B2EA));
        assert!(filter(0x2781_28A0));
        self.calls.push(Call::Demolish(request));
        self.demolition
    }
    fn network_flags(&mut self, x: i32, z: i32, mask: u32) -> u32 {
        self.calls.push(Call::Network(x, z, mask));
        *self.flags.get(&(x, z)).unwrap_or(&0)
    }
    fn tutorial_rectangle(&self) -> Option<Rect> {
        self.task
    }
    fn total_funds(&self) -> i64 {
        self.funds
    }
    fn place_zone(&mut self, zone: Zone) -> bool {
        self.calls.push(Call::Zone(zone));
        self.zone_results.pop_front().unwrap_or(true)
    }
    fn create_lot(&mut self, r: Rect, rotation: u8, enabled: bool, water: bool, cfg: u32) -> Option<u64> {
        self.calls.push(Call::Lot(r, rotation, enabled, water, cfg));
        self.lot
    }
    fn construct_default_lot(&mut self, lot: u64, cfg: u32, exemplar: u32, construction: u32) -> Option<Building> {
        self.calls.push(Call::Build(lot, cfg, exemplar, construction));
        self.builder.then_some(Building {
            token: 100,
            position: point(49.0, 50.0),
        })
    }
    fn effect_running(&self, token: u64) -> bool {
        self.running.contains(&token)
    }
    fn lot_position(&mut self, lot: u64) -> Vec3 {
        assert_eq!(lot, 99);
        lot_point()
    }
    fn has_window_manager(&self) -> bool {
        self.wm
    }
    fn city_grid_hidden(&self) -> bool {
        self.city_hidden
    }
    fn entries(&mut self, _: u32) -> Vec<Entry> {
        self.entries.clone()
    }
    fn first_building_entry(&mut self, _: u32) -> Option<Entry> {
        self.building_entry.clone()
    }
    fn building_candidates(&mut self, _: &[u32]) -> Vec<u32> {
        self.candidates_building.clone()
    }
    fn has_profile(&mut self, _: u32) -> bool {
        self.profile
    }
    fn create_preview_building(&mut self, exemplar: u32) -> Option<u64> {
        self.calls.push(Call::PreviewBuilding(exemplar));
        self.building_creation.then_some(90)
    }
    fn create_preview_prop(&mut self, exemplar: u32) -> Option<u64> {
        self.calls.push(Call::PreviewProp(exemplar));
        self.prop_allowed.then_some(u64::from(exemplar) + 1000)
    }
    fn preview_info(&mut self, token: u64) -> PreviewInfo {
        let mut info = self.preview;
        if self.denied_props.contains(&token) {
            info.prop_preview = false;
        }
        info
    }
    fn coverage_radius(&mut self, _: u64, category: u32) -> Option<(f32, u32)> {
        self.radii.get(&category).copied()
    }
    fn create_effect(&mut self, name: &str) -> Option<u64> {
        self.calls.push(Call::EffectCreate(name.into()));
        if !self.visual {
            return None;
        }
        self.next_effect += 1;
        Some(self.next_effect)
    }
    fn create_ring(&mut self) -> Option<u64> {
        self.visual.then_some(299)
    }
    fn footprint_texture(&mut self, _: u32) -> Option<(u64, f32)> {
        self.visual.then_some((500, 64.0))
    }
    fn position_to_cell(&self, p: f32) -> i32 {
        (p / 16.0).floor() as i32
    }
    fn cell_corner(&self, x: i32, z: i32) -> (f32, f32) {
        (x as f32 * 16.0, z as f32 * 16.0)
    }
    fn network(&mut self, x: i32, z: i32, mask: u32, option: bool) -> Option<Network> {
        self.calls.push(Call::TollNetwork(x, z, mask, option));
        self.networks.get(&(x, z)).copied()
    }
}
fn tool(h: &mut Fake) -> PlaceLot {
    let mut t = PlaceLot::new(0, 10, 0, false);
    assert!(t.init(h));
    h.calls.clear();
    t
}
fn effects(h: &Fake) -> Vec<Effect> {
    h.calls
        .iter()
        .filter_map(|c| if let Call::Effect(e) = c { Some(e.clone()) } else { None })
        .collect()
}

#[test]
fn construction_and_ct3_lifecycle() {
    let mut t = PlaceLot::new(0, 10, 0, true);
    assert_eq!((t.rotation, t.player_rotation, t.shown_rotation), (0, 0, -1));
    assert_eq!((t.mouse.x, t.mouse.y), (0, 0));
    assert_eq!(t.last_placed, [0, 0, u32::MAX, u32::MAX]);
    assert_eq!(
        t.position,
        Vec3 {
            x: 0.0,
            y: -f32::MAX,
            z: 0.0
        }
    );
    assert!(!t.placeable && !t.feedback_flag && !t.toll_booth);
    assert_eq!((t.cost, t.plop_cost, t.plops_left), (-100000, -100000, 1000));
    assert!(t.should_stack());
    assert!(!t.unhandled_input());
    let mut h = Fake {
        groups: vec![0x1401],
        ..Fake::default()
    };
    assert!(t.init(&mut h));
    assert!(t.init(&mut h));
    assert_eq!((t.control_id, t.cursor_id), (ID, CURSOR));
    assert_eq!(
        effects(&h)
            .iter()
            .filter(|e| **e == Effect::Acquire(Service::View))
            .count(),
        1
    );
    let subscriptions: Vec<_> = effects(&h)
        .into_iter()
        .filter_map(|e| if let Effect::Subscribe(m) = e { Some(m) } else { None })
        .collect();
    assert_eq!(subscriptions, MESSAGES);
    assert_eq!(t.coverage_view, 10);
    assert_eq!(t.plop_cost, 500);
    assert!(!t.set_cursor_id(&mut h, 999));
    assert_eq!(t.cursor_id, CURSOR);
    h.calls.clear();
    t.coverage_raised = true;
    assert!(t.shutdown(&mut h));
    let calls = h.calls.len();
    assert!(t.shutdown(&mut h));
    assert_eq!(calls, h.calls.len());
    let es = effects(&h);
    assert_eq!(
        &es[..3],
        &[
            Effect::ClearCoverageOverlay,
            Effect::RecalculateCoverage,
            Effect::UnregisterView(10)
        ]
    );
    let unsub: Vec<_> = es
        .iter()
        .filter_map(|e| if let Effect::Unsubscribe(m) = e { Some(*m) } else { None })
        .collect();
    assert_eq!(unsub, MESSAGES);
    assert_eq!(
        &es[es.len() - 3..],
        &[
            Effect::Release(Service::WindowManager),
            Effect::Release(Service::View),
            Effect::ReleaseCursor(8)
        ]
    );
    assert_eq!(t.cursor_id, CURSOR);
}

#[test]
fn activation_pickers_filters_and_failure_paths() {
    let mut h = Fake::default();
    let mut t = PlaceLot::new(0, 0, 0, true);
    assert!(t.init(&mut h));
    assert_eq!(t.configuration, 10);
    assert!(h.calls.contains(&Call::Filter(Filter::Monopoly(true))));
    assert!(h.calls.contains(&Call::Pick(
        "Lot Plop".into(),
        vec![(10, "0x0000000a".into()), (11, "0x0000000b".into())],
        10
    )));
    let mut h = Fake::default();
    let mut t = PlaceLot::new(1, 0, 0, false);
    assert!(t.init(&mut h));
    assert_eq!((t.exemplar, t.configuration), (7, 11));
    assert!(h.calls.contains(&Call::Filter(Filter::Building(7))));
    assert!(effects(&h).contains(&Effect::StoreLastBuilding(7)));
    assert!(effects(&h).contains(&Effect::StoreLastConfiguration(11)));
    let mut h = Fake::default();
    h.picks.push_back(None);
    let mut t = PlaceLot::new(1, 0, 0, false);
    assert!(!t.init(&mut h));
    assert!(t.initialized);
    t.shutdown(&mut h);
    let mut h = Fake::default();
    h.candidates.clear();
    let mut t = PlaceLot::new(1, 0, 7, false);
    assert!(!t.init(&mut h));
    assert!(!t.initialized);
    assert!(effects(&h).contains(&Effect::Unsubscribe(MESSAGES[5])));
    let mut h = Fake {
        price: None,
        groups: vec![0x1302],
        ..Fake::default()
    };
    let mut t = PlaceLot::new(0, 0xEBF9_A440, 0, false);
    assert!(t.init(&mut h));
    assert!(t.toll_booth);
    assert_eq!(t.plop_cost, UNKNOWN_COST);
    assert_eq!(t.coverage_view, 0xF2);
}

#[test]
fn ct4_rotation_keys_top_translation_and_manual_path() {
    let mut h = Fake::default();
    let mut t = tool(&mut h);
    h.command = Some(0x6A93_5CD5);
    for _ in 0..4 {
        assert!(t.key_down(&mut h, 1, 2));
    }
    assert_eq!(t.player_rotation, 0);
    h.command = Some(0x6A93_5CD1);
    assert!(t.key_down(&mut h, 1, 2));
    h.command = Some(0x6A93_5CD5);
    assert!(t.key_down(&mut h, 1, 2));
    assert_eq!((t.rotation, t.player_rotation), (0, 0));
    assert!(h.calls.contains(&Call::Key(1, 2, 1)));
    assert!(!h.calls.iter().any(|c| matches!(c, Call::Network(..))));
    h.command = Some(7);
    assert!(!t.key_down(&mut h, 1, 2));
    h.command = None;
    assert!(!t.key_down(&mut h, 1, 2));
    h.top = false;
    h.calls.clear();
    assert!(!t.key_down(&mut h, 1, 2));
    assert!(h.calls.is_empty());
}

#[test]
fn conformance_odd_even_rotated_cells_and_edges() {
    let terrain = Fake::default().terrain();
    let odd = cell_coverage([3, 3], terrain, point(40.0, 40.0), 0);
    assert_eq!(
        odd.rect,
        Rect {
            x0: 1,
            z0: 1,
            x1: 3,
            z1: 3
        }
    );
    assert_eq!(odd.point, point(40.0, 40.0));
    assert!(odd.valid);
    let even = cell_coverage([2, 2], terrain, point(40.0, 40.0), 0);
    assert_eq!(even.rect, rect());
    assert_eq!(even.point, point(48.0, 48.0));
    assert!(even.valid);
    let rotated = cell_coverage([2, 3], terrain, point(40.0, 40.0), 1);
    assert_eq!(
        rotated.rect,
        Rect {
            x0: 1,
            z0: 2,
            x1: 3,
            z1: 3
        }
    );
    assert!(!cell_coverage([3, 3], terrain, point(0.0, 0.0), 0).valid);
    assert!(!cell_coverage([3, 3], terrain, point(511.0, 511.0), 0).valid);
    // Truncation (not floor) on negative fractional positions is published.
    assert_eq!(
        cell_coverage([1, 1], terrain, point(-1.0, -1.0), 0).point,
        point(8.0, 8.0)
    );
}

#[test]
fn validation_precedence_price_codes_and_demolition_flags() {
    let mut h = Fake::default();
    let mut t = tool(&mut h);
    let v = t.can_place(&mut h, point(40.0, 40.0), 0);
    assert!(v.allowed);
    assert_eq!(v.cost, 535);
    assert_eq!(v.creation_code, 8);
    assert_eq!(
        h.calls,
        vec![
            Call::Development(rect()),
            Call::CreateTest(rect(), 0, 10),
            Call::Demolish(Demolition {
                commit: false,
                rect: rect(),
                privilege: 1,
                flags: 0x2800,
                option: false
            })
        ]
    );
    t.plop_cost = 0;
    h.calls.clear();
    assert_eq!(t.can_place(&mut h, point(40.0, 40.0), 0).cost, 10);
    assert!(!h.calls.iter().any(|c| matches!(c, Call::Development(_))));
    t.coverage_view = 0xF2;
    h.calls.clear();
    t.can_place(&mut h, point(40.0, 40.0), 0);
    assert!(h.calls.contains(&Call::Demolish(Demolition {
        commit: false,
        rect: rect(),
        privilege: 1,
        flags: 0x800,
        option: false
    })));
    for (code, expected) in CREATION_MESSAGES.iter().enumerate() {
        h.codes = [code as i32; 4];
        h.calls.clear();
        let v = t.can_place(&mut h, point(40.0, 40.0), 0);
        assert!(!v.allowed);
        assert_eq!(v.text, format!("0a419226:{expected:08x}"));
        assert!(!h.calls.iter().any(|c| matches!(c, Call::Demolish(_))));
    }
    h.codes = [9; 4];
    assert!(t.can_place(&mut h, point(40.0, 40.0), 0).text.is_empty());
    h.codes = [8; 4];
    h.demolition.0 = false;
    assert_eq!(t.can_place(&mut h, point(40.0, 40.0), 0).text, "0a419226:6a711969");
    h.calls.clear();
    t.last_placed = [32, 32, 64, 64];
    let v = t.can_place(&mut h, point(48.0, 48.0), 0);
    assert!(!v.allowed);
    assert_eq!(v.cost, 0);
    assert!(h.calls.is_empty());
    t.last_placed = [48, 48, 48, 48];
    assert!(!t.can_place(&mut h, point(48.0, 48.0), 0).allowed);
    h.calls.clear();
    assert!(!t.can_place(&mut h, point(511.0, 511.0), 0).allowed);
    assert!(h.calls.is_empty());
    h.cfg = None;
    assert!(t.can_place(&mut h, point(40.0, 40.0), 0).coverage.is_none());
}

#[test]
fn conformance_frontage_hysteresis_ties_and_unplaceable_rotations() {
    assert_eq!(choose_facing([3, 6, 0, 0], 0), 0);
    assert_eq!(choose_facing([3, 8, 0, 0], 0), 1);
    assert_eq!(choose_facing([0, 1, 0, 0], 0), 1);
    assert_eq!(choose_facing([3, 7, 0, 0], 0), 0);
    assert_eq!(choose_facing([0; 4], 2), 2);
    let mut h = Fake::default();
    let t = tool(&mut h);
    let wide = Rect {
        x0: 2,
        z0: 2,
        x1: 4,
        z1: 4,
    };
    h.flags.insert((2, 1), 0x40);
    h.flags.insert((3, 1), 1);
    h.flags.insert((4, 1), 1);
    assert_eq!(t.street_count(&mut h, wide, 0), 8);
    assert!(h.calls.contains(&Call::Network(2, 1, 0x449)));
    h.flags.insert((4, 2), 0x400);
    h.flags.insert((4, 3), 8);
    assert_eq!(t.street_count(&mut h, rect(), 1), 2);
    h.codes = [1, 8, 1, 1];
    let facing = t.automatic_facing(&mut h, point(40.0, 40.0));
    assert!(facing.any);
    assert_eq!(facing.rotation, 1);
    assert_eq!(facing.scores, [-8, 2, -8, -8]);
    h.codes = [1; 4];
    let f = t.automatic_facing(&mut h, point(40.0, 40.0));
    assert!(!f.any);
    h.calls.clear();
    assert_eq!(
        t.street_count(
            &mut h,
            Rect {
                x0: 0,
                z0: 0,
                x1: 1,
                z1: 1
            },
            0
        ),
        0
    );
    assert!(h.calls.is_empty());
}

#[test]
fn mouse_feedback_text_cost_exit_and_failed_coverage() {
    let mut h = Fake::default();
    let mut t = tool(&mut h);
    t.mouse_move(&mut h, mouse(), 0x100);
    assert!(t.placeable);
    assert_eq!(t.cost, 535);
    assert_eq!(t.position, point(48.0, 48.0));
    assert_eq!(t.shown_rotation, 0);
    assert!(effects(&h).contains(&Effect::ShowCost(535)));
    h.calls.clear();
    t.mouse_move(&mut h, mouse(), 0x100);
    assert!(!effects(&h).iter().any(|e| matches!(e, Effect::FeedbackState(..))));
    h.hit = None;
    h.calls.clear();
    t.mouse_move(&mut h, mouse(), 0);
    assert!(!t.placeable);
    assert_eq!(t.cursor_text, "0a419226:ca711914");
    assert!(effects(&h).contains(&Effect::CursorText(TEXT_KEY, t.cursor_text.clone())));
    assert!(effects(&h).contains(&Effect::ClearCost(false)));
    h.calls.clear();
    t.mouse_exit(&mut h);
    assert!(t.cursor_text.is_empty());
    assert!(effects(&h).contains(&Effect::CursorText(TEXT_KEY, String::new())));
    h.hit = Some(point(511.0, 511.0));
    t.rotation = 2;
    t.player_rotation = 1;
    t.mouse_move(&mut h, mouse(), 0);
    assert!(!t.placeable);
    assert_eq!(t.rotation, 1);
    h.hit = Some(point(40.0, 40.0));
    t.plop_cost = 0;
    h.demolition.1 = 0;
    t.mouse_move(&mut h, mouse(), 0x100);
    assert!(t.placeable);
    assert_eq!(t.cost, 0);
    assert!(effects(&h).contains(&Effect::FlushInputQueue));
}

#[test]
fn ct5_click_sound_repeat_rectangle_and_plop_limit() {
    let mut h = Fake::default();
    let mut t = tool(&mut h);
    assert!(!t.mouse_down(&mut h, mouse()));
    assert!(h.calls.contains(&Call::Sound(0xEA45_1903, 0xAA62_C9D5)));
    t.mouse_move(&mut h, mouse(), 0x100);
    h.calls.clear();
    t.plops_left = 1;
    assert!(t.mouse_down(&mut h, mouse()));
    assert_eq!(t.last_placed, [32, 32, 64, 64]);
    assert!(!t.placeable);
    assert_eq!(t.plops_left, 0);
    assert!(h.calls.contains(&Call::End(false)));
    assert!(!h.calls.iter().any(|c| matches!(c, Call::Sound(..))));
    assert!(effects(&h).contains(&Effect::ClearCost(true)));
    assert!(!t.can_place(&mut h, point(48.0, 48.0), 0).allowed);
    h.hit = Some(point(100.0, 100.0));
    t.mouse_move(&mut h, mouse(), 0x100);
    assert!(t.placeable);
    assert_eq!(t.last_placed, [0, 0, u32::MAX, u32::MAX]);
    h.hit = None;
    assert!(!t.mouse_down(&mut h, mouse()));
}

#[test]
fn commit_order_money_aura_connections_and_effects() {
    let mut h = Fake::default();
    let mut t = tool(&mut h);
    t.cost = 500;
    h.occupant.water_production = true;
    h.occupant.subway_station = true;
    let result = t.place_lot(&mut h, &mut point(40.0, 40.0), 0, true);
    assert!(result.built);
    assert_eq!(result.rect, rect());
    assert_eq!(
        &h.calls[..5],
        &[
            Call::Demolish(Demolition {
                commit: true,
                rect: rect(),
                privilege: 1,
                flags: 0x2000,
                option: false
            }),
            Call::Zone(Zone {
                rect: rect(),
                kind: 0,
                commit: true,
                skip_funds: false,
                check_slope: true,
                skip_size: true,
                check_lot_inside: false,
                outputs: [None; 3]
            }),
            Call::Zone(Zone {
                rect: rect(),
                kind: 15,
                commit: true,
                skip_funds: true,
                check_slope: true,
                skip_size: true,
                check_lot_inside: false,
                outputs: [None; 3]
            }),
            Call::Lot(rect(), 0, true, false, 10),
            Call::Build(99, 10, 0, 123),
        ]
    );
    assert_eq!(
        effects(&h),
        vec![
            Effect::Withdraw(500),
            Effect::Aura(2, point(49.0, 50.0)),
            Effect::InsertConnection(Connection::Pipe, 3, 3),
            Effect::InsertConnection(Connection::Subway, 3, 3),
            Effect::PlacementEffect("placement_effects_2x2", lot_point()),
            Effect::Ring(lot_point(), [2, 2], 0x49D5_B83F)
        ]
    );
}

#[test]
fn conformance_builder_failure_deposits_without_withdrawal_and_keeps_lot() {
    let mut h = Fake::default();
    let mut t = tool(&mut h);
    t.cost = 500;
    h.builder = false;
    assert!(!t.place_lot(&mut h, &mut point(40.0, 40.0), 0, true).built);
    assert_eq!(
        effects(&h),
        vec![
            Effect::Deposit(500),
            Effect::PlacementEffect("placement_effects_2x2", lot_point()),
            Effect::Ring(lot_point(), [2, 2], 0x49D5_B83F)
        ]
    );
    assert_eq!(h.calls.iter().filter(|c| matches!(c, Call::Lot(..))).count(), 1);
}

#[test]
fn commit_guards_unknown_cost_free_zones_and_sizes() {
    let mut h = Fake::default();
    let mut t = tool(&mut h);
    h.codes = [0; 4];
    assert!(!t.place_lot(&mut h, &mut point(40.0, 40.0), 0, true).built);
    assert!(!h.calls.iter().any(|c| matches!(c, Call::Lot(..))));
    t.cost = 500;
    h.task = Some(Rect::INVALID);
    h.calls.clear();
    assert!(!t.place_lot(&mut h, &mut point(40.0, 40.0), 0, false).built);
    assert!(h.calls.is_empty());
    h.task = Some(rect());
    h.funds = 499;
    assert!(!t.place_lot(&mut h, &mut point(40.0, 40.0), 0, false).built);
    assert!(h.calls.is_empty());
    h.funds = 500;
    h.lot = None;
    assert!(!t.place_lot(&mut h, &mut point(40.0, 40.0), 0, true).built);
    assert!(!h.calls.iter().any(|c| matches!(c, Call::Build(..))));
    assert!(effects(&h).is_empty());
    h.lot = Some(99);
    h.task = None;
    h.funds = -10;
    t.cost = 0;
    for (groups, water, kind) in [
        (vec![0x1508], false, 11),
        (vec![0x1509], false, 12),
        (vec![], true, 12),
        (vec![], false, 15),
    ] {
        h.groups = groups;
        h.cfg.as_mut().unwrap().water_compatible = water;
        h.calls.clear();
        assert!(t.place_lot(&mut h, &mut point(40.0, 40.0), 0, false).built);
        assert!(h.calls.contains(&Call::Zone(Zone {
            rect: rect(),
            kind,
            commit: true,
            skip_funds: true,
            check_slope: true,
            skip_size: true,
            check_lot_inside: false,
            outputs: [None; 3]
        })));
        assert!(!effects(&h)
            .iter()
            .any(|e| matches!(e, Effect::Withdraw(_) | Effect::PlacementEffect(..))));
    }
    for (size, name) in [
        ([1, 1], "placement_effects_1x1"),
        ([3, 2], "placement_effects_3x3"),
        ([4, 5], "placement_effects_4x4"),
    ] {
        h.cfg.as_mut().unwrap().size = size;
        h.calls.clear();
        assert!(t.place_lot(&mut h, &mut point(120.0, 120.0), 0, true).built);
        assert!(effects(&h)
            .iter()
            .any(|e| matches!(e, Effect::PlacementEffect(n, _) if *n == name)));
    }
}

#[test]
fn messages_stop_only_running_release_both_and_open_boundaries() {
    let mut h = Fake::default();
    let mut t = tool(&mut h);
    assert!(!t.message(&mut h, 0x277A_9364));
    assert_eq!(effects(&h), vec![Effect::GridVisible(true)]);
    h.hidden = true;
    h.calls.clear();
    t.message(&mut h, 0x277A_9364);
    assert_eq!(effects(&h), vec![Effect::GridVisible(false)]);
    h.calls.clear();
    t.message(&mut h, 0x99EF_1142);
    t.message(&mut h, 0x99EF_1143);
    assert_eq!(effects(&h), vec![Effect::RecalculateCoverage; 2]);
    t.coverage_raised = true;
    h.calls.clear();
    t.message(&mut h, 0x99EF_1142);
    assert!(h.calls.is_empty());
    for message in [0xA6B7_9602, 0xC9F7_75BB] {
        t.preview_effects = [Some(200), Some(201)];
        h.calls.clear();
        assert!(!t.message(&mut h, message));
        assert_eq!(
            &effects(&h)[..3],
            &[
                Effect::StopEffect(200, 0),
                Effect::ReleaseCreatedEffect(200),
                Effect::ReleaseCreatedEffect(201)
            ]
        );
        assert_eq!(t.preview_effects, [None; 2]);
        assert!(effects(&h).iter().any(|e| matches!(e, Effect::FeedbackState(..))));
    }
    h.calls.clear();
    t.message(&mut h, 0x49D5_B83F);
    assert_eq!(effects(&h), vec![Effect::RecalculateCoverage]);
    assert!(!t.coverage_raised);
    h.calls.clear();
    assert!(!t.message(&mut h, 123));
    assert!(h.calls.is_empty());
    t.toll_booth = true;
    t.mouse_move(&mut h, mouse(), 0);
    assert!(!t.placeable);
    assert!(h.calls.contains(&Call::TollNetwork(2, 2, 0x1045, true)));
    h.calls.clear();
    t.cost = 500;
    assert!(!t.place_lot(&mut h, &mut point(40.0, 40.0), 0, false).built);
    assert!(effects(&h).is_empty());
}
