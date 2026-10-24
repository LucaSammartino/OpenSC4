// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::ui_windows::win::WinStore;
include!("resolved_tests.rs");
include!("trigger_load_tests.rs");
include!("closure_tests.rs");
include!("closure_round3_tests.rs");

#[derive(Debug, PartialEq)]
enum Op {
    Action(ActionCall),
    Tool(ToolSpec),
    City(CityCall),
    Gui(GuiCall),
    Lifecycle(LifecycleCall),
    Window(MessageCall),
    Command(u32),
    Control(u32, ControlCall),
    Change(Option<u32>, Option<u32>),
    Cursor(u32),
    Capture(bool),
    Position(i32, i32),
    Jump([f32; 3]),
    View(ViewChange),
    Scroll(f32, f32),
    Message(u32),
    Begin,
    End,
    Pause,
    Resume,
    Restart,
    ResetPaint,
    Cost(f32),
    Time(f32),
    Draw,
    Tail,
    DeferredRef(u32, bool),
    Repost(u32, u32),
    Send(u32, u32, u32),
    HoldFilter(u32),
    ReleaseFilter(u32),
    CreateFilter([bool; 3]),
    Size(f32),
    Property(String, String),
    Recorder(bool),
    Contour(bool, u32),
    CostDisplay(CostDisplay),
    Clock(u32, u32, u32),
    Rating(i32, i32),
    Alert(Option<u32>),
    Speed(u32, bool, bool),
    Mission(String, bool),
    Panel(u32, bool),
    Tab(u32, bool),
    Close(bool),
}

struct Host {
    mission_status: Option<MissionStatus>,
    mission_updates: Vec<(u32, String, bool, Option<String>)>,
    advice_calls: Vec<AdviceCall>,
    advice_available: bool,
    budget_window: Option<u32>,
    coverage: CoverageRecorder,
    graph: GraphRecorder,
    registry: Vec<(u32, &'static str, u32)>,
    funds_value: i32,
    hover: Vec<(bool, u32, u32)>,
    dialog_calls: Vec<DialogCall>,
    widgets: Vec<WidgetCall>,
    lighting: Vec<f32>,
    popup_calls: Vec<PopupCall>,
    popup_items: Vec<CatalogItem>,
    selection: Option<CatalogItem>,
    reference_calls: Vec<(u64, u32)>,
    written_records: Vec<Option<ObjectRecord>>,
    read_record: Option<ObjectRecord>,
    helper_calls: Vec<(u32, u32)>,
    pictures: Vec<BackgroundPicture>,
    background_requests: Vec<(bool, String)>,
    cost_visibility: Vec<bool>,
    ops: Vec<Op>,
    takers: Vec<u32>,
    init_fail: Option<u32>,
    capture: bool,
    tutorial: bool,
    rewrite: Option<(i32, i32)>,
    zoom: i32,
    changed: bool,
    deferred_enabled: Option<bool>,
    decimal_separator: u8,
    date: Option<[i32; 3]>,
    moved: bool,
    simulator: bool,
    scroll_control: Option<u32>,
    occupant: Option<[f32; 3]>,
    terrain: Option<[f32; 3]>,
    recorder_exists: bool,
    filter_allocation: bool,
    tabs: std::collections::BTreeMap<u32, bool>,
    established: bool,
    rejected_gui: Vec<GuiCall>,
    rejected_city: Vec<CityCall>,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            mission_status: None,
            mission_updates: Vec::new(),
            advice_calls: Vec::new(),
            advice_available: true,
            budget_window: Some(0x41),
            coverage: CoverageRecorder::default(),
            graph: GraphRecorder::default(),
            registry: Vec::new(),
            funds_value: 100,
            hover: Vec::new(),
            dialog_calls: Vec::new(),
            widgets: Vec::new(),
            lighting: Vec::new(),
            popup_calls: Vec::new(),
            popup_items: Vec::new(),
            selection: None,
            reference_calls: Vec::new(),
            written_records: Vec::new(),
            read_record: None,
            helper_calls: Vec::new(),
            pictures: Vec::new(),
            background_requests: Vec::new(),
            cost_visibility: Vec::new(),
            ops: Vec::new(),
            takers: Vec::new(),
            init_fail: None,
            capture: false,
            tutorial: true,
            rewrite: None,
            zoom: 3,
            changed: true,
            deferred_enabled: None,
            decimal_separator: b'.',
            date: None,
            moved: false,
            simulator: true,
            scroll_control: None,
            occupant: None,
            terrain: Some([10.0, 20.0, 30.0]),
            recorder_exists: false,
            filter_allocation: true,
            tabs: std::collections::BTreeMap::new(),
            established: true,
            rejected_gui: Vec::new(),
            rejected_city: Vec::new(),
        }
    }
}

impl Host {
    fn events(&self) -> Vec<u32> {
        self.ops
            .iter()
            .filter_map(|op| match op {
                Op::Control(id, ControlCall::Event(_)) => Some(*id),
                _ => None,
            })
            .collect()
    }
}

impl WidgetHost for Host {
    fn status_snapshot(&self) -> StatusSnapshot {
        StatusSnapshot {
            date: self.date.unwrap_or([12003, 4, 5]),
            start_year: 2000,
            population: 1234,
            city_name: "City".into(),
            budget_visible: true,
            visual_time: (13, 45),
            ..StatusSnapshot::default()
        }
    }
    fn widget(&mut self, call: WidgetCall) {
        self.widgets.push(call);
    }
    fn ambient_light(&mut self, hour: f32) -> bool {
        self.lighting.push(hour);
        self.changed
    }
}

impl DialogHost for Host {
    fn establish_preferences(&self) -> EstablishData {
        EstablishData {
            difficulty: 2,
            city: "é".repeat(520),
            mayor: "Mayor".into(),
        }
    }
    fn dialog(&mut self, call: DialogCall) -> Option<EstablishData> {
        let reply = match &call {
            DialogCall::Establish { data, .. } if self.changed => Some(data.clone()),
            _ => None,
        };
        self.dialog_calls.push(call);
        reply
    }
    fn budget_dialog_enabled(&self) -> bool {
        self.filter_allocation
    }
}

impl PopupHost for Host {
    fn popup(&mut self, call: PopupCall) -> Option<u32> {
        let reply = if self.filter_allocation {
            match call {
                PopupCall::Script { instance, .. } => Some(instance),
                PopupCall::Catalog { .. } => Some(600),
                PopupCall::Query(token, _) => Some(token + 1),
                _ => None,
            }
        } else {
            None
        };
        self.popup_calls.push(call);
        reply
    }
    fn catalog_items(&mut self, _: &CatalogQuery) -> Vec<CatalogItem> {
        self.popup_items.clone()
    }
    fn popup_state(&mut self, _: u32) -> [u32; 2] {
        [7, 9]
    }
    fn popup_selected(&mut self, _: u32) -> Option<CatalogItem> {
        self.selection
    }
    fn popup_height(&self) -> i32 {
        if self.moved {
            601
        } else {
            600
        }
    }
    fn tutorial_running(&self) -> bool {
        !self.tutorial
    }
    fn utility_extra_enabled(&self, _: u32) -> bool {
        self.changed
    }
    fn situation_visible(&self) -> bool {
        self.changed
    }
}

impl ReferenceHost for Host {
    fn read_view_record(&mut self, _: u32) -> Option<ObjectRecord> {
        self.read_record.clone()
    }
    fn reference_body_available(&mut self, _: u32) -> bool {
        self.changed
    }
    fn write_view_record(&mut self, _: u32, record: Option<ObjectRecord>) {
        self.written_records.push(record);
    }
    fn save_identity(&mut self, object: u32, offset: u32) -> u64 {
        self.reference_calls.push((u64::from(object), offset));
        u64::from(object) + 0x10000
    }
    fn resolve_identity(&mut self, identity: u64, iid: u32) -> Option<u32> {
        self.reference_calls.push((identity, iid));
        self.filter_allocation.then_some((identity - 0x10000) as u32)
    }
}

impl HelperHost for Host {
    fn background_candidates(&mut self, three_d: bool, basename: &str) -> Vec<String> {
        self.background_requests.push((three_d, basename.into()));
        self.background_files(three_d)
    }
    fn background_zoom(&self) -> u8 {
        self.zoom as u8
    }
    fn background_mode(&self) -> bool {
        self.moved
    }
    fn background_files(&mut self, _: bool) -> Vec<String> {
        (0..self.pictures.len()).map(|i| i.to_string()).collect()
    }
    fn load_background(&mut self, path: &str) -> Option<BackgroundPicture> {
        self.pictures.get(path.parse::<usize>().unwrap()).cloned()
    }
    fn renderer_background(&mut self, picture: u32, three_d: bool) {
        self.helper_calls.push((picture, u32::from(three_d)));
    }
    fn release_picture(&mut self, picture: u32) {
        self.helper_calls.push((picture, 0xFFFF));
    }
    fn coverage_clear(&mut self) {
        self.helper_calls.push((0, 0));
    }
    fn coverage_effect(&mut self, effect: u32, radius: u32) {
        self.helper_calls.push((effect, radius));
    }
    fn stop_helper(&mut self, helper: u32) -> bool {
        self.helper_calls.push((helper, 4));
        self.changed
    }
    fn helper_stopped(&mut self, helper: u32, value: u32) {
        self.helper_calls.push((helper, value));
    }
    fn renderer_pick(&mut self, _: bool, _: i32, _: i32, _: u32) -> Option<[f32; 3]> {
        self.terrain
    }
    fn acquire_occupant_interface(&mut self, iid: u32) -> bool {
        self.helper_calls.push((iid, 1));
        self.filter_allocation
    }
    fn release_occupant_interface(&mut self) {
        self.helper_calls.push((0x26BB_767E, 0));
    }
}

impl LifecycleHost for Host {
    fn register_view_command(&mut self, id: u32, name: &'static str, flags: u32) {
        self.registry.push((id, name, flags));
    }
    fn lifecycle(&mut self, call: LifecycleCall) -> LifecycleReply {
        let success = !matches!(call, LifecycleCall::BaseInit) || self.init_fail != Some(0);
        let object = match call {
            LifecycleCall::LoadKeyMap { .. } if self.filter_allocation => Some(11),
            LifecycleCall::Create { class, .. } if self.filter_allocation => Some(class),
            LifecycleCall::CreateProgress if self.filter_allocation => Some(12),
            _ => None,
        };
        self.ops.push(Op::Lifecycle(call));
        LifecycleReply { success, object }
    }
    fn cheat_manager_exists(&self) -> bool {
        true
    }
    fn development_cheats(&self) -> bool {
        self.established
    }
    fn loading_city(&self) -> bool {
        self.moved
    }
}

impl MessageHost for Host {
    fn message_call(&mut self, call: MessageCall) {
        self.ops.push(Op::Window(call));
    }
    fn tutorial_message(&mut self, message: &mut WindowMessage) -> bool {
        if let Some((kind, data)) = self.rewrite {
            message.kind = kind as u32;
            message.data[0] = data as u32;
        }
        !self.tutorial
    }
    fn base_message(&mut self, _: &WindowMessage) -> bool {
        self.changed
    }
    fn open_url(&mut self, _: &str) -> bool {
        self.changed
    }
    fn focus_loss_stops_scroll(&self) -> bool {
        self.changed
    }
    fn accelerator(&mut self, _: u32, key: u32, _: u32, _: bool) -> Option<u32> {
        (key == 32).then_some(0x6A93_5CAD)
    }
    fn post_command(&mut self, command: u32) {
        self.ops.push(Op::Command(command));
    }
    fn modal_safe(&self) -> bool {
        self.changed
    }
    fn city_view_hidden(&self) -> bool {
        self.moved
    }
    fn paused_overlay_exists(&self) -> bool {
        self.simulator
    }
    fn filter_window_passthrough(&self) -> bool {
        false
    }
    fn advice_resource(&mut self, manager: u32, id: u32, iid: u32) -> Option<u32> {
        self.advice_calls.push(AdviceCall::Resource(manager, id, iid));
        self.advice_available.then_some(77)
    }
    fn advice_activate(&mut self, resource: u32, first: u32, second: u32) {
        self.advice_calls.push(AdviceCall::Activate(resource, first, second));
    }
    fn advice_release(&mut self, resource: u32) {
        self.advice_calls.push(AdviceCall::Release(resource));
    }
    fn hide_city_dialog(&mut self) {
        self.advice_calls.push(AdviceCall::HideDialog);
    }
    fn show_graph_type(&mut self, procedure: u32, selector: u32) {
        self.advice_calls.push(AdviceCall::Graph(procedure, selector));
    }
    fn budget_window(&mut self, procedure: u32) -> Option<u32> {
        self.advice_calls.push(AdviceCall::Budget(procedure));
        self.budget_window
    }
    fn dispatch_child(&mut self, window: u32, message: WindowMessage) {
        self.advice_calls.push(AdviceCall::Dispatch(window, message));
    }
}

impl GuiHost for Host {
    fn gui(&mut self, call: GuiCall) -> Option<u32> {
        let token = if self.filter_allocation && !self.rejected_gui.contains(&call) {
            match call {
                GuiCall::Script { id, .. } => Some(id),
                GuiCall::Create { class, .. } => Some(class),
                GuiCall::Helper(kind) => Some(100 + kind as u32),
                GuiCall::Query(token, iid) => Some(token.wrapping_add(iid)),
                GuiCall::Child { id, .. }
                | GuiCall::RecursiveChild { id, .. }
                | GuiCall::RecursivePanelChild { id, .. } => Some(id),
                GuiCall::Image { instance, .. } => Some(instance),
                GuiCall::CreateBuffer => Some(200),
                _ => None,
            }
        } else {
            None
        };
        self.ops.push(Op::Gui(call));
        token
    }
    fn client_size(&self) -> (i32, i32) {
        (1920, 600)
    }
    fn window_size(&self, _: u32) -> (i32, i32) {
        (100, 20)
    }
}

impl CityHost for Host {
    fn city(&mut self, call: CityCall) -> Option<u32> {
        let token = if self.filter_allocation && !self.rejected_city.contains(&call) {
            match call {
                CityCall::Create(kind) | CityCall::Construct(kind) => Some(kind.class_id()),
                CityCall::QueryGraphics(iid) => Some(iid),
                CityCall::AcquireSimulator => Some(0x8695_664E),
                CityCall::CreateSituationControl => Some(1234),
                _ => None,
            }
        } else {
            None
        };
        self.ops.push(Op::City(call));
        token
    }
    fn city_size(&self) -> u32 {
        1024
    }
    fn tunable_values(&mut self) -> TunableValues {
        TunableValues {
            edge_speed: Some(0.8),
            render_properties: vec![("trees".into(), "false".into())],
            ..TunableValues::default()
        }
    }
    fn tip_ready(&self) -> bool {
        self.changed
    }
}

impl ActionHost for Host {
    fn action(&mut self, call: ActionCall) {
        self.ops.push(Op::Action(call));
    }
    fn create_tool(&mut self, spec: ToolSpec) -> Option<u32> {
        self.ops.push(Op::Tool(spec));
        self.filter_allocation.then_some(90)
    }
    fn expanded_tooltips(&self) -> bool {
        self.changed
    }
    fn signs_shown(&self) -> bool {
        self.changed
    }
    fn animation_paused(&self) -> bool {
        self.changed
    }
    fn main_panel_visible(&self) -> bool {
        self.changed
    }
    fn alt_down(&self, modifiers: u32) -> bool {
        modifiers == 4
    }
    fn cursor_in_view(&self) -> (i32, i32) {
        (50, 60)
    }
    fn disaster_properties(&mut self, _: u32) -> DisasterProperties {
        if self.changed {
            DisasterProperties {
                effect_group: Some(42),
                manager: 12,
                ..DisasterProperties::default()
            }
        } else {
            DisasterProperties::default()
        }
    }
}

#[test]
fn command_save_quit_and_animation_guards_preserve_parameters() {
    let mut v = view();
    let mut host = Host::default();
    v.owner.modifiers = 4;
    assert!(!v.process_command(&mut host, 0x6A93_5E36, [0, 9]));
    assert!(!v.process_command(&mut host, 0x6A93_5E3C, [2, 9]));
    v.process_command(&mut host, 0x6A97_53EA, [0, 0]);
    assert_eq!(
        host.ops,
        vec![
            Op::Action(ActionCall::Save {
                prompt: true,
                quick: true
            }),
            Op::Action(ActionCall::Quit {
                prompt: false,
                parameter1: 9
            })
        ]
    );
    v.owner.view.animation_debug = true;
    v.process_command(&mut host, 0x6A97_53EA, [0, 0]);
    v.process_command(&mut host, 0x6A97_53ED, [0, 0]);
    assert_eq!(
        &host.ops[2..],
        &[
            Op::Action(ActionCall::AnimationPause(false)),
            Op::Action(ActionCall::AnimationStep)
        ]
    );
    host.ops.clear();
    v.process_command(&mut host, 0x6A93_5E2D, [0, 0]);
    assert_eq!(
        host.ops,
        vec![
            Op::Action(ActionCall::GraphicOptions),
            Op::Action(ActionCall::FocusView)
        ]
    );
}

#[test]
fn catalog_item_guards_network_intersections_lot_move_and_null_allocation() {
    let mut v = view();
    let mut host = Host {
        established: false,
        ..Host::default()
    };
    v.invoke_catalog_item(&mut host, 0x4A2B_2467, 0x8C32_9937, false);
    v.invoke_zoning_tool(&mut host, 15);
    assert!(host.ops.is_empty());
    host.established = true;
    v.invoke_zoning_tool(&mut host, 16);
    assert!(host.ops.is_empty());
    v.set_tertiary(Some(100), 1);
    v.invoke_catalog_item(&mut host, 0x4A2B_2467, 0x6A47_1234, true);
    assert_eq!(host.ops[0], Op::Close(true));
    assert_eq!(host.ops[1], Op::Tool(ToolSpec::Intersection(0x1234)));
    host.ops.clear();
    v.owner.mouse_inside = true;
    v.invoke_catalog_item(&mut host, 0x6A2B_245F, 42, false);
    let movement = Op::Control(90, ControlCall::Event(InputEvent::Move(50, 60, 0)));
    assert!(host.ops.contains(&movement));
    assert_eq!(host.ops.last(), Some(&Op::Control(90, ControlCall::Release)));
    host.filter_allocation = false;
    host.ops.clear();
    v.invoke_zoning_tool(&mut host, 1);
    assert_eq!(v.input_control(), None);
    assert!(!host.ops.contains(&movement));
}

#[test]
fn disaster_properties_and_global_terrain_close_level_are_literal() {
    let mut v = view();
    let mut host = Host {
        changed: false,
        ..Host::default()
    };
    v.invoke_disaster(&mut host, 42);
    assert!(host.ops.is_empty());
    v.set_secondary(Some(100), 1);
    v.invoke_catalog_item(&mut host, 0x6A5A_C478, 6, true);
    assert_eq!(
        host.ops,
        vec![Op::Close(false), Op::Action(ActionCall::GlobalTerrain(6))]
    );
    host.ops.clear();
    v.invoke_disaster(&mut host, 0xA9DE_8784);
    assert_eq!(host.ops[0], Op::Tool(ToolSpec::Torch));
}

#[test]
fn city_pre_post_initialization_orders_camera_and_gui_then_releases_progress() {
    let mut v = view();
    let mut host = Host::default();
    v.owner.lifecycle.progress = Some(99);
    v.pre_city_init(&mut host);
    assert!(v.owner.motion.city_initializing);
    assert_eq!(v.owner.view.filter.unwrap().realistic_flags, Some([true; 3]));
    assert_eq!(v.owner.motion.edge_speed, 0.8);
    assert_eq!(
        &host.ops[..4],
        &[
            Op::City(CityCall::Create(CityObject::Camera)),
            Op::City(CityCall::Create(CityObject::CameraControl)),
            Op::City(CityCall::Viewport(0xC9C6_28EC, 1920, 600)),
            Op::City(CityCall::CameraInit(0xC9C6_28EC, [512.0, 0.0, 512.0])),
        ]
    );
    let existing = v.owner.view.filter;
    host.ops.clear();
    v.post_city_init(&mut host);
    assert!(!v.owner.motion.city_initializing);
    assert_eq!(v.persistent_controls(), 1);
    assert_eq!(v.lifecycle_state().progress, None);
    assert!(host.ops.contains(&Op::City(CityCall::CitySize(0xC9C6_28EC, 1024))));
    assert_eq!(
        host.ops
            .iter()
            .filter(|op| matches!(op, Op::City(CityCall::DemandGraph { .. })))
            .count(),
        11
    );
    assert_eq!(
        host.ops
            .iter()
            .filter(|op| matches!(op, Op::City(CityCall::Gauge { .. })))
            .count(),
        6
    );
    assert_eq!(
        &host.ops[host.ops.len() - 2..],
        &[
            Op::Lifecycle(LifecycleCall::ProgressShutdown(99)),
            Op::Lifecycle(LifecycleCall::Release(99)),
        ]
    );
    host.ops.clear();
    v.pre_city_init(&mut host);
    assert_eq!(v.owner.view.filter, existing);
    assert!(!host.ops.iter().any(|op| matches!(op, Op::CreateFilter(_))));
}

#[test]
fn city_allocation_failure_is_fatal_and_obliteration_hides_panel4() {
    let mut v = view();
    let mut host = Host {
        filter_allocation: false,
        established: false,
        ..Host::default()
    };
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| v.pre_city_init(&mut host))).is_err());
    assert!(v.owner.lifecycle.objects.is_empty());
    assert_eq!(v.persistent_controls(), 0);
    assert_eq!(v.owner.view.filter, None);
    v.owner.panels.current = Some(4);
    v.owner.panels.panel4_shown = true;
    v.city_obliterated(&mut host);
    assert_eq!(v.owner.panels.current, None);
    assert!(!v.owner.panels.panel4_shown);
}

#[test]
fn gui_lifetime_removes_filters_before_destruction_and_releases_queries() {
    let mut v = view();
    let mut host = Host {
        established: false,
        ..Host::default()
    };
    v.add_gui(&mut host);
    assert_eq!(v.owner.panels.current, Some(0));
    assert_eq!(v.gui_state().windows.len(), 8);
    assert_eq!(
        &host.ops[..6],
        &[
            Op::Gui(GuiCall::Script {
                instance: 0xC973_B411,
                id: 0x0987_B48F,
                iid: 0x22BA_0121
            }),
            Op::Gui(GuiCall::Query(0x0987_B48F, 0x5386_D516)),
            Op::Gui(GuiCall::Filter(0x0987_B48F, true)),
            Op::Gui(GuiCall::Move(0x0987_B48F, 645, 591)),
            Op::Gui(GuiCall::Flag(0x0987_B48F, 0x800, true)),
            Op::Gui(GuiCall::ClearFocus(0x0987_B48F)),
        ]
    );
    assert!(host.ops.contains(&Op::Gui(GuiCall::Move(0x0987_B48F, 645, 591))));
    let fields = v.gui_state().fields.clone();
    let windows = v.gui_state().windows.clone();
    let overlay = 0xCA5D_3294;
    // Creation reference released as soon as the parent owns the overlay.
    assert!(host.ops.contains(&Op::Gui(GuiCall::Release(overlay))));
    host.ops.clear();
    v.remove_gui(&mut host);
    for token in windows {
        let filter = host
            .ops
            .iter()
            .position(|o| *o == Op::Gui(GuiCall::Filter(token, false)))
            .unwrap();
        assert_eq!(
            &host.ops[filter..filter + 4],
            &[
                Op::Gui(GuiCall::Filter(token, false)),
                Op::Gui(GuiCall::Destroy(token)),
                Op::Gui(GuiCall::Shutdown(token)),
                Op::Gui(GuiCall::Release(token)),
            ]
        );
    }
    for token in fields.values() {
        assert_eq!(
            host.ops
                .iter()
                .filter(|o| **o == Op::Gui(GuiCall::Release(*token)))
                .count(),
            1
        );
    }
    assert!(!host.ops.contains(&Op::Gui(GuiCall::Destroy(overlay))));
    assert!(!host.ops.contains(&Op::Gui(GuiCall::Release(overlay))));
    assert!(v.gui_state().fields.is_empty());
    assert!(v.gui_state().windows.is_empty());
    assert!(v.gui_state().panels.iter().all(Vec::is_empty));
}

#[test]
fn gui_failed_creations_and_shutdown_progress_order() {
    let mut v = view();
    let mut host = Host {
        filter_allocation: false,
        ..Host::default()
    };
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| v.add_gui(&mut host))).is_err());
    assert!(v.gui_state().fields.is_empty());
    v.remove_gui(&mut host);
    assert!(!host
        .ops
        .iter()
        .any(|op| matches!(op, Op::Gui(GuiCall::Destroy(_) | GuiCall::Release(_)))));
    host.filter_allocation = true;
    v.init(&mut host);
    v.add_gui(&mut host);
    v.owner.lifecycle.progress = Some(42);
    host.ops.clear();
    v.shutdown(&mut host);
    let gui_end = host
        .ops
        .iter()
        .rposition(|o| matches!(o, Op::Gui(call) if *call != GuiCall::BaseShutdown))
        .unwrap();
    let progress = host
        .ops
        .iter()
        .position(|o| *o == Op::Lifecycle(LifecycleCall::ProgressShutdown(42)))
        .unwrap();
    assert!(gui_end < progress);
    assert_eq!(host.ops.last(), Some(&Op::Gui(GuiCall::BaseShutdown)));
}

#[test]
fn child_filter_uses_existing_tab_state_and_special_type12_branch() {
    let mut v = view();
    let mut host = Host::default();
    host.tabs.insert(0xC988_BC79, true);
    assert!(v.filter_message(
        &mut host,
        WindowMessage {
            kind: 3,
            data: [0x2872_59F7, 0xC988_BC79, 0],
            ..WindowMessage::default()
        }
    ));
    assert_eq!(v.owner.panels.current, Some(1));
    assert!(host.tabs[&0xC988_BC79]);
    host.ops.clear();
    assert!(v.filter_message(
        &mut host,
        WindowMessage {
            kind: 0x12,
            ..WindowMessage::default()
        }
    ));
    assert!(host.ops.is_empty());
    host.established = false;
    host.changed = false;
    v.filter_message(
        &mut host,
        WindowMessage {
            kind: 3,
            data: [0x2872_59F7, 0xC988_BC79, 0],
            ..WindowMessage::default()
        },
    );
    assert!(!host.tabs[&0xC988_BC79]);
}

#[test]
fn init_orders_registration_keeps_base_failure_and_tolerates_keymap_failure() {
    let mut host = Host {
        established: false,
        init_fail: Some(0),
        moved: true,
        ..Host::default()
    };
    let mut v = view();
    assert!(!v.init(&mut host));
    assert_eq!(host.ops[0], Op::Lifecycle(LifecycleCall::Subscribe));
    assert_eq!(host.ops[1], Op::Lifecycle(LifecycleCall::RegisterCommands));
    let cheats: Vec<_> = host
        .ops
        .iter()
        .filter_map(|op| match op {
            Op::Lifecycle(LifecycleCall::Cheat(id, _, true)) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(cheats, CHEATS[..10].iter().map(|c| c.0).collect::<Vec<_>>());
    assert_eq!(host.ops.last(), Some(&Op::Lifecycle(LifecycleCall::InitTail)));
    assert_eq!(v.lifecycle_state().progress, Some(12));
    assert!(host.ops.contains(&Op::Lifecycle(LifecycleCall::Release(11))));
    host.init_fail = None;
    assert!(v.init(&mut host));
    host.ops.clear();
    assert!(v.init(&mut host));
    assert!(host.ops.is_empty());
    let mut failed = view();
    host.filter_allocation = false;
    assert!(failed.init(&mut host));
    assert!(!failed.lifecycle_state().objects.contains_key(&0x120));
    assert_eq!(failed.lifecycle_state().progress, None);
}

#[test]
fn vw_t2_shutdown_unregisters_all_cheats_and_drops_both_control_lists() {
    let mut host = Host {
        established: false,
        ..Host::default()
    };
    let mut v = view();
    v.init(&mut host);
    v.set_current_control(&mut host, Some(71), 0);
    v.add_persistent_control(&mut host, 72);
    host.ops.clear();
    v.shutdown_helpers(&mut host);
    v.shutdown_tail(&mut host);
    assert_eq!(v.input_control(), None);
    assert_eq!(v.persistent_controls(), 0);
    let ids: Vec<_> = host
        .ops
        .iter()
        .filter_map(|op| match op {
            Op::Lifecycle(LifecycleCall::Cheat(id, _, false)) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(ids, CHEATS.iter().map(|c| c.0).collect::<Vec<_>>());
    for id in [71, 72] {
        assert!(host.ops.contains(&Op::Control(id, ControlCall::Shutdown)));
        assert!(host.ops.contains(&Op::Control(id, ControlCall::Release)));
    }
    assert_eq!(host.ops.last(), Some(&Op::Lifecycle(LifecycleCall::UnregisterCommands)));
}

#[test]
fn vw_t3_deferred_budget_deduplicates_and_command_qi_has_priority() {
    let mut v = view();
    let mut host = Host {
        changed: false,
        ..Host::default()
    };
    let message = Notification {
        kind: 0xAA5C_F1E3,
        data: [1, 2],
        ..Notification::default()
    };
    assert!(v.notification(&mut host, message.clone()));
    v.notification(&mut host, message.clone());
    assert_eq!(v.owner.deferred, vec![message.clone()]);
    v.notification(
        &mut host,
        Notification {
            command_interface: true,
            ..message
        },
    );
    assert_eq!(v.owner.deferred.len(), 1);
    assert_eq!(host.ops, [Op::DeferredRef(0xAA5C_F1E3, true)]);
    v.replay_deferred(&mut host);
    assert_eq!(v.owner.deferred.len(), 1);
    host.ops.clear();
    host.changed = true;
    v.replay_deferred(&mut host);
    assert!(v.owner.deferred.is_empty());
    assert_eq!(
        host.ops,
        vec![
            Op::DeferredRef(0xAA5C_F1E3, true),
            Op::DeferredRef(0xAA5C_F1E3, false),
            Op::Repost(0xAA5C_F1E3, 0),
            Op::DeferredRef(0xAA5C_F1E3, false),
            Op::Tail,
        ]
    );
    assert!(host.dialog_calls.is_empty());
}

#[test]
fn vw_t4_url_success_is_false_and_tutorial_precedes_every_window_route() {
    let mut v = view();
    let mut host = Host::default();
    let message = WindowMessage {
        kind: 0xE4A8_5D91,
        text: "example".into(),
        ..WindowMessage::default()
    };
    assert!(!v.window_message(&mut host, message.clone()));
    host.changed = false;
    assert!(v.window_message(&mut host, message.clone()));
    host.tutorial = false;
    assert!(v.window_message(&mut host, message));
    assert!(v.filter_message(&mut host, WindowMessage::default()));
    assert!(host.ops.is_empty());
}

#[test]
fn window_focus_accelerator_button_filter_and_clock_routes() {
    let mut v = view();
    let mut host = Host::default();
    v.init(&mut host);
    host.ops.clear();
    v.owner.motion.keyboard = [1, -1];
    v.owner.motion.edge = [-1, 1];
    assert!(v.window_message(
        &mut host,
        WindowMessage {
            kind: 0x10,
            ..WindowMessage::default()
        }
    ));
    assert_eq!(v.owner.motion.keyboard, [0, 0]);
    assert_eq!(v.owner.motion.edge, [0, 0]);
    assert!(host.ops.contains(&Op::Window(MessageCall::MouseExit)));
    assert!(v.window_message(
        &mut host,
        WindowMessage {
            kind: 0x1B,
            data: [5, 32, 0],
            ..WindowMessage::default()
        }
    ));
    assert!(host.ops.contains(&Op::Command(0x6A93_5CAD)));
    assert!(!v.window_message(
        &mut host,
        WindowMessage {
            kind: 0x1B,
            data: [6, 33, 0],
            ..WindowMessage::default()
        }
    ));
    host.ops.clear();
    let button = WindowMessage {
        kind: 3,
        data: [0x2872_59F6, 19, 0],
        ..WindowMessage::default()
    };
    assert!(v.window_message(&mut host, button.clone()));
    assert!(host.ops.is_empty());
    assert!(v.filter_message(&mut host, button));
    assert!(host.ops.is_empty()); // unknown button has no invented action
    assert!(host.popup_calls.is_empty());
    for kind in [0x13, 0x0D, 0x14] {
        v.window_message(
            &mut host,
            WindowMessage {
                kind,
                ..WindowMessage::default()
            },
        );
    }
    assert!(!v.owner.mouse_inside);
}

#[test]
fn notification_view_hidden_pause_mission_and_progress_guards() {
    let mut v = view();
    let mut host = Host {
        moved: true,
        ..Host::default()
    };
    v.notification(
        &mut host,
        Notification {
            kind: 0xA6B7_9602,
            ..Notification::default()
        },
    );
    assert!(host.ops.is_empty());
    host.moved = false;
    v.notification(
        &mut host,
        Notification {
            kind: 0xA6B7_9602,
            ..Notification::default()
        },
    );
    assert_eq!(
        &host.ops[..2],
        &[
            Op::Window(MessageCall::Refresh(Refresh::Zoom)),
            Op::Window(MessageCall::Refresh(Refresh::Background))
        ]
    );
    assert!(host.ops.contains(&Op::Pause));
    host.ops.clear();
    v.notification(
        &mut host,
        Notification {
            kind: 0xABBB_CA64,
            data: [7, 9],
            ..Notification::default()
        },
    );
    assert_eq!(
        host.ops,
        [
            Op::Window(MessageCall::Refresh(Refresh::PausedOverlay)),
            Op::Window(MessageCall::Refresh(Refresh::Speed)),
            Op::Speed(0x6A4F_BB31, true, false),
            Op::Speed(0x4A4F_BB60, false, false),
            Op::Speed(0x8998_BBDF, false, false),
            Op::Window(MessageCall::Refresh(Refresh::Alert)),
            Op::Alert(None),
            Op::Window(MessageCall::Refresh(Refresh::Mission(0, 7))),
        ]
    );
    host.ops.clear();
    v.owner.lifecycle.progress = Some(99);
    v.notification(
        &mut host,
        Notification {
            kind: 0x4A44_667F,
            ..Notification::default()
        },
    );
    assert!(host.ops.is_empty());
    v.notification(
        &mut host,
        Notification {
            kind: 0x4A44_667F,
            progress_interface: true,
            ..Notification::default()
        },
    );
    assert_eq!(host.ops, vec![Op::Window(MessageCall::Progress([0, 0]))]);
    assert_eq!(
        NOTIFICATIONS
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        33
    );
}

impl MotionHost for Host {
    fn virtual_zoom(&self) -> i32 {
        self.zoom
    }
    fn camera_zoom(&self) -> f32 {
        self.zoom as f32
    }
    fn view_change(&mut self, c: ViewChange) -> bool {
        self.ops.push(Op::View(c));
        self.changed
    }
    fn scroll_camera(&mut self, a: f32, d: f32) -> bool {
        self.ops.push(Op::Scroll(a, d));
        self.moved
    }
    fn post_scroll_message(&mut self, m: u32) {
        self.ops.push(Op::Message(m));
    }
    fn begin_scroll_render_settings(&mut self) {
        self.ops.push(Op::Begin);
    }
    fn end_scroll_render_settings(&mut self) {
        self.ops.push(Op::End);
    }
    fn simulator_present(&self) -> bool {
        self.simulator
    }
    fn hidden_pause(&mut self) {
        self.ops.push(Op::Pause);
    }
    fn hidden_resume(&mut self) {
        self.ops.push(Op::Resume);
    }
    fn restart_hidden_timer(&mut self) {
        self.ops.push(Op::Restart);
    }
    fn reset_paint_timer(&mut self) {
        self.ops.push(Op::ResetPaint);
    }
    fn animation_time(&mut self) -> f32 {
        7.0
    }
    fn set_draw_context_time(&mut self, t: f32) {
        self.ops.push(Op::Time(t));
    }
    fn draw(&mut self) {
        self.ops.push(Op::Draw);
    }
    fn update_cost_indicator(&mut self, dt: f32) {
        self.ops.push(Op::Cost(dt));
    }
    fn paint_tail(&mut self) {
        self.ops.push(Op::Tail);
    }
    fn deferred_safe(&self) -> bool {
        self.changed
    }
    fn deferred_enabled(&self) -> bool {
        self.deferred_enabled.unwrap_or(self.changed)
    }
    fn deferred_reference(&mut self, m: &Notification, acquire: bool) {
        self.ops.push(Op::DeferredRef(m.kind, acquire));
    }
    fn repost_deferred(&mut self, m: &Notification, delivery: u32) {
        self.ops.push(Op::Repost(m.kind, delivery));
    }
}

impl InputHost for Host {
    fn control(&mut self, id: u32, call: ControlCall) -> ControlReply {
        self.ops.push(Op::Control(id, call));
        ControlReply {
            taken: match call {
                ControlCall::Init => self.init_fail != Some(id),
                ControlCall::Event(_) => self.takers.contains(&id),
                ControlCall::CanStack => true,
                _ => false,
            },
            cursor: (call == ControlCall::Cursor).then_some(id + 100),
            tool_id: (call == ControlCall::ToolId).then_some(0xC7AF_928E),
        }
    }
    fn control_changed(&mut self, old: Option<u32>, new: Option<u32>) {
        self.ops.push(Op::Change(old, new));
    }
    fn set_cursor(&mut self, c: u32) {
        self.ops.push(Op::Cursor(c));
    }
    fn has_capture(&self) -> bool {
        self.capture
    }
    fn set_capture(&mut self, c: bool) {
        self.capture = c;
        self.ops.push(Op::Capture(c));
    }
    fn tutorial_mouse(&mut self, x: &mut i32, y: &mut i32, _: i32) -> bool {
        if let Some((a, b)) = self.rewrite {
            *x = a;
            *y = b;
        }
        self.tutorial
    }
    fn cursor_position(&mut self, x: i32, y: i32) {
        self.ops.push(Op::Position(x, y));
    }
    fn create_mouse_scroll(&mut self) -> Option<u32> {
        self.scroll_control
    }
    fn occupant_centre(&mut self, _: i32, _: i32) -> Option<[f32; 3]> {
        self.occupant
    }
    fn terrain_point(&mut self, _: i32, _: i32) -> Option<[f32; 3]> {
        self.terrain
    }
    fn camera_jump(&mut self, p: [f32; 3]) {
        self.ops.push(Op::Jump(p));
    }
}

fn view() -> View3DWin {
    let mut store = WinStore::new();
    let token = store.create(1);
    View3DWin::new(0x2000, token)
}

fn context() -> MouseContext {
    MouseContext {
        width: 1024,
        height: 768,
        edge_preference: true,
        tutorial_edge_allowed: true,
        fullscreen: true,
    }
}

fn controls() -> (View3DWin, Host) {
    let mut v = view();
    let mut h = Host::default();
    v.add_persistent_control(&mut h, 1);
    v.add_persistent_control(&mut h, 2);
    v.set_current_control(&mut h, Some(3), 0);
    v.set_current_control(&mut h, Some(4), 0);
    h.ops.clear();
    (v, h)
}

#[test]
fn vw_t1_qi_exact_interfaces_and_shared_count() {
    let mut v = view();
    for (iid, offset) in [
        (1, 0),
        (0xFA47_B3F9, 0),
        (0x4522_94AA, 4),
        (0xC6AE_7085, 8),
        (0x22BA_0121, 12),
    ] {
        assert_eq!(v.query_interface(iid), Some(0x2000 + offset));
    }
    assert_eq!(v.refs(), 6);
    assert_eq!(v.query_interface(0x5386_D516), None);
    assert_eq!(v.refs(), 6);
}

#[test]
fn vw_t7_and_t10_keys_broadcast_persistent_after_stack_taker() {
    let (mut v, mut h) = controls();
    h.takers = vec![4];
    assert!(v.process_key(&mut h, 42, 3, true));
    assert_eq!(h.events(), [4, 2, 1]);
    assert_eq!(v.owner_state().modifiers, 3);
    h.ops.clear();
    h.takers = vec![1];
    assert!(v.process_key(&mut h, 42, 2, false));
    assert_eq!(h.events(), [4, 3, 2, 1]);
    h.takers.clear();
    assert!(!v.process_key(&mut h, 42, 0, true));
}

#[test]
fn vw_t10_buttons_visit_persistent_before_stack_and_stop() {
    let (mut v, mut h) = controls();
    v.on_mouse(&mut h, InputEvent::DownL(40, 40, 0), context());
    assert_eq!(h.events(), [2, 1, 4, 3]);
    assert!(!h.capture);
    h.ops.clear();
    h.takers = vec![1];
    v.on_mouse(&mut h, InputEvent::DownL(40, 40, 0), context());
    assert_eq!(h.events(), [2, 1]);
    for pair in h.ops.chunks(3) {
        assert!(matches!(
            pair,
            [
                Op::Control(_, ControlCall::AddRef),
                Op::Control(_, ControlCall::Event(_)),
                Op::Control(_, ControlCall::Release)
            ]
        ));
    }
}

#[test]
fn vw_t8_right_click_drag_threshold_and_modifiers() {
    for (point, mods, jumps) in [((102, 103), 0, true), ((104, 100), 0, false), ((100, 100), 1, false)] {
        let mut v = view();
        let mut h = Host::default();
        v.on_mouse(&mut h, InputEvent::DownR(100, 100, 0), context());
        assert!(h.capture);
        v.on_mouse(&mut h, InputEvent::Move(point.0, point.1, mods), context());
        v.on_mouse(&mut h, InputEvent::UpR(point.0, point.1, mods), context());
        assert!(!h.capture);
        assert_eq!(h.ops.iter().any(|op| matches!(op, Op::Jump(_))), jumps);
    }
}

#[test]
fn vw_t9_zero_wheel_zooms_out_and_stack_can_suppress_everything() {
    let (mut v, mut h) = controls();
    v.on_mouse(&mut h, InputEvent::Wheel(10, 10, 2, 0), context());
    assert_eq!(h.events(), [4, 3, 2, 1]);
    assert!(h.ops.contains(&Op::View(ViewChange::ZoomOut)));
    h.ops.clear();
    h.takers = vec![4];
    v.on_mouse(&mut h, InputEvent::Wheel(10, 10, 0, 1), context());
    assert_eq!(h.events(), [4]);
    assert!(!h.ops.iter().any(|op| matches!(op, Op::View(_))));
}

#[test]
fn character_persistent_pass_has_stack_size_bound() {
    let (mut v, mut h) = controls();
    v.on_character(&mut h, 65);
    assert_eq!(h.events(), [4, 3, 2, 1]);
    v.remove_current_control(&mut h, true);
    h.ops.clear();
    v.on_character(&mut h, 65);
    assert_eq!(h.events(), [3]);
}

#[test]
fn tutorial_blocks_move_before_drag_and_rewrites_button_coordinates() {
    let (mut v, mut h) = controls();
    h.tutorial = false;
    v.on_mouse(&mut h, InputEvent::Move(100, 100, 4), context());
    assert!(h.ops.is_empty());
    assert!(!v.owner_state().dragged);
    assert_eq!(v.owner_state().modifiers, 4);
    h.tutorial = true;
    h.rewrite = Some((12, 13));
    v.on_mouse(&mut h, InputEvent::DownL(1, 2, 0), context());
    assert!(h
        .ops
        .contains(&Op::Control(2, ControlCall::Event(InputEvent::DownL(12, 13, 0)))));
}

#[test]
fn exit_calls_only_top_without_reference_and_broadcasts_persistent() {
    let (mut v, mut h) = controls();
    v.on_mouse(&mut h, InputEvent::Exit, context());
    assert_eq!(h.events(), [4, 2, 1]);
    assert!(h.ops.contains(&Op::Position(-10000, -10000)));
    assert!(!h.ops.contains(&Op::Control(4, ControlCall::AddRef)));
}

#[test]
fn control_modes_init_failure_and_persistent_lifetime() {
    let (mut v, mut h) = controls();
    h.init_fail = Some(5);
    v.set_current_control(&mut h, Some(5), 1);
    assert_eq!(v.input_control(), Some(3));
    assert_eq!(v.persistent_controls(), 2);
    assert!(!h.ops.contains(&Op::Control(5, ControlCall::AddRef)));
    assert!(h.ops.contains(&Op::Control(4, ControlCall::Shutdown)));
    assert!(h.ops.contains(&Op::Control(3, ControlCall::Deactivate)));
    v.set_current_control(&mut h, None, 2);
    assert_eq!(v.input_control(), None);
    assert_eq!(v.persistent_controls(), 2);
    assert!(v.remove_persistent_control(&mut h, 1));
    assert!(!v.remove_persistent_control(&mut h, 1));
    v.remove_all_persistent_controls(&mut h);
    assert_eq!(v.persistent_controls(), 0);
}

#[test]
fn remove_control_sets_default_cursor_and_releases_capture() {
    let mut v = view();
    let mut h = Host::default();
    v.set_current_control(&mut h, Some(1), 0);
    h.capture = true;
    h.ops.clear();
    assert!(v.remove_current_control(&mut h, false));
    assert_eq!(
        h.ops,
        [
            Op::Capture(false),
            Op::Control(1, ControlCall::Deactivate),
            Op::Control(1, ControlCall::Shutdown),
            Op::Control(1, ControlCall::Release),
            Op::Cursor(0xC16C_4E1B)
        ]
    );
    assert!(!v.remove_current_control(&mut h, false));
}

#[test]
fn vw_t11_t12_keyboard_cannot_add_diagonal_and_any_stop_stops_both() {
    let mut v = view();
    let mut h = Host::default();
    v.keyboard_scroll(&mut h, [-1, 0], true);
    v.keyboard_scroll(&mut h, [0, -1], true);
    let m = &v.owner_state().motion;
    assert_eq!(m.keyboard, [-1, 0]);
    assert_eq!(m.angle, std::f32::consts::PI);
    assert!((m.speed - 0.3456).abs() < 0.000_001);
    v.keyboard_scroll(&mut h, [0, 0], true);
    assert!(v.owner_state().motion.stopping);
}

#[test]
fn edge_recalculation_forgets_edge_and_keyboard_replaces_axis() {
    let mut v = view();
    let mut h = Host::default();
    v.motion_mut().edge = [1, 1];
    v.motion_mut().keyboard = [-1, 0];
    v.recalc_scroll_velocity(&mut h, true);
    assert_eq!(v.owner_state().motion.edge, [0, 0]);
    assert!((v.owner_state().motion.angle - 3.0 * std::f32::consts::PI / 4.0).abs() < 0.000_001);
}

#[test]
fn vw_t13_zoom_limits_compare_against_unclamped_maximum() {
    let mut v = view();
    let mut h = Host {
        zoom: 5,
        ..Host::default()
    };
    v.set_zoom_limits(&mut h, 0, 9, Some(false));
    assert_eq!(v.owner_state().motion.zoom_max, 4);
    assert!(h.ops.is_empty());
    v.set_zoom(&mut h, 99);
    assert!(h.ops.contains(&Op::View(ViewChange::Zoom(4))));
}

#[test]
fn paint_gates_preserve_tail_and_write_time_before_draw() {
    let mut v = view();
    let mut h = Host::default();
    assert!(v.paint(&mut h, 10, 0));
    assert_eq!(h.ops, [Op::Tail]);
    h.ops.clear();
    v.motion_mut().paint_enabled = true;
    v.motion_mut().paint_period_us = 100.0;
    assert!(v.paint(&mut h, 99, 0));
    assert_eq!(h.ops, [Op::Tail]);
    h.ops.clear();
    assert!(v.paint(&mut h, 1_000_000, 0));
    assert_eq!(
        h.ops,
        [Op::ResetPaint, Op::Cost(0.5), Op::Time(7.0), Op::Draw, Op::Tail]
    );
    h.ops.clear();
    v.motion_mut().city_initializing = true;
    v.paint(&mut h, 250_000, 0);
    assert!(h.ops.contains(&Op::Time(0.25)));
}

#[test]
fn scroll_decay_and_hidden_pause_restart_do_not_resume_in_same_paint() {
    let mut v = view();
    let mut h = Host {
        moved: true,
        ..Host::default()
    };
    v.motion_mut().paint_enabled = true;
    v.motion_mut().reduced_render_settings = true;
    v.set_scrolling(&mut h, true, 1.0, 2.0);
    v.scroll_stop();
    v.paint(&mut h, 100_000, 5000);
    assert!(h.ops.contains(&Op::Scroll(1.0, 5.0)));
    assert!(!h.ops.contains(&Op::Resume));
    v.paint(&mut h, 100_000, 5000);
    assert!(!v.owner_state().motion.scrolling);
    assert!(h.ops.contains(&Op::End));
    assert_eq!(h.ops.iter().filter(|op| **op == Op::Pause).count(), 1);
    h.moved = false;
    v.paint(&mut h, 100_000, 2000);
    assert!(!h.ops.contains(&Op::Resume));
    v.paint(&mut h, 100_000, 2001);
    assert!(h.ops.contains(&Op::Resume));
}

#[test]
fn failed_view_change_does_not_pause_and_no_simulator_does_not_own_pause() {
    let mut v = view();
    let mut h = Host {
        changed: false,
        ..Host::default()
    };
    assert!(!v.change_view(&mut h, ViewChange::RotateLeft));
    assert_eq!(h.ops, [Op::View(ViewChange::RotateLeft)]);
    h.simulator = false;
    v.temporary_hidden_pause(&mut h);
    assert!(!v.owner_state().motion.hidden_paused);
    assert!(!h.ops.contains(&Op::Pause));
}

impl StateHost for Host {
    fn send(&mut self, m: u32, a: u32, b: u32) {
        self.ops.push(Op::Send(m, a, b));
    }
    fn hold_filter(&mut self, t: u32) {
        self.ops.push(Op::HoldFilter(t));
    }
    fn release_filter(&mut self, t: u32) {
        self.ops.push(Op::ReleaseFilter(t));
    }
    fn add_contour_layer(&mut self, t: u32) -> Option<i32> {
        self.ops.push(Op::Contour(true, t));
        Some(42)
    }
    fn remove_contour_layer(&mut self, id: i32) {
        self.ops.push(Op::Contour(false, id as u32));
    }
}

impl CheatHost for Host {
    fn decimal_separator(&self) -> u8 {
        self.decimal_separator
    }
    fn create_cheat_control(&mut self, _: CheatTool) -> Option<u32> {
        self.scroll_control
    }
    fn create_realistic_filter(&mut self, flags: [bool; 3]) -> Option<FilterInfo> {
        self.ops.push(Op::CreateFilter(flags));
        self.filter_allocation.then_some(FilterInfo {
            token: 9,
            realistic_flags: Some(flags),
        })
    }
    fn camera_size(&mut self, size: f32) {
        self.ops.push(Op::Size(size));
    }
    fn render_property(&mut self, name: &str, value: &str) -> bool {
        self.ops.push(Op::Property(name.into(), value.into()));
        self.changed
    }
    fn post_render_property_change(&mut self, m: u32) {
        self.ops.push(Op::Message(m));
    }
    fn recorder_child_exists(&self) -> bool {
        self.recorder_exists
    }
    fn create_recorder_child(&mut self, id: u32, path: &str) {
        assert_eq!(id, 0xA802_B4EB);
        assert_eq!(path, "RecordedAnimations/");
        self.recorder_exists = true;
        self.ops.push(Op::Recorder(true));
    }
    fn remove_recorder_child(&mut self, id: u32) {
        assert_eq!(id, 0xA802_B4EB);
        self.recorder_exists = false;
        self.ops.push(Op::Recorder(false));
    }
}

#[test]
fn vw_t5_sizeof_clamps_arguments_and_needs_one() {
    let mut v = view();
    let mut h = Host::default();
    for arg in ["100", "0.5", "abc", "5", "12junk", "2e1tail", "3e+"] {
        assert!(v.process_cheat(&mut h, 0xCA7B_C5B5, &["SizeOf", arg, "ignored"]));
    }
    assert!(!v.process_cheat(&mut h, 0xCA7B_C5B5, &["SizeOf"]));
    assert_eq!(
        h.ops,
        [
            Op::Size(64.0),
            Op::Size(0.9),
            Op::Size(0.9),
            Op::Size(5.0),
            Op::Size(12.0),
            Op::Size(20.0),
            Op::Size(3.0)
        ]
    );
}

#[test]
fn vw_t6_filter_cheat_false_results_and_terrain_query() {
    let mut v = view();
    let mut h = Host::default();
    v.set_filter(
        &mut h,
        Some(FilterInfo {
            token: 1,
            realistic_flags: Some([true, true, true]),
        }),
    );
    assert!(!v.process_cheat(&mut h, 0xAA7A_2746, &["TastyZots"]));
    assert_eq!(
        v.owner_state().view.filter.unwrap().realistic_flags,
        Some([true, false, true])
    );
    assert!(!v.process_cheat(&mut h, 0xAA7A_2746, &["TastyZots"]));
    assert_eq!(
        v.owner_state().view.filter.unwrap().realistic_flags,
        Some([true, true, true])
    );
    assert!(!v.process_cheat(&mut h, 0x6A7A_276E, &["Zoneria"]));
    assert_eq!(
        v.owner_state().view.filter.unwrap().realistic_flags,
        Some([false, true, false])
    );
    assert!(v.process_cheat(&mut h, 0x8AB7_30AE, &["TerrainQuery", "OFF"]));
    assert!(!v.owner_state().view.terrain_query);
    assert!(v.process_cheat(&mut h, 0x8AB7_30AE, &["TerrainQuery"]));
    assert!(v.owner_state().view.terrain_query);
}

#[test]
fn filter_messages_and_reference_order_are_synchronous() {
    let mut v = view();
    let mut h = Host::default();
    v.set_filter(
        &mut h,
        Some(FilterInfo {
            token: 1,
            realistic_flags: None,
        }),
    );
    h.ops.clear();
    v.set_filter(
        &mut h,
        Some(FilterInfo {
            token: 2,
            realistic_flags: None,
        }),
    );
    assert_eq!(
        h.ops,
        [
            Op::Send(0x06CD_B65A, 1, 2),
            Op::HoldFilter(2),
            Op::ReleaseFilter(1),
            Op::Send(0x06CD_B65B, 0, 0)
        ]
    );
    h.ops.clear();
    assert_eq!(v.get_filter(&mut h).unwrap().token, 2);
    assert_eq!(h.ops, [Op::HoldFilter(2)]);
    h.filter_allocation = false;
    assert!(!v.process_cheat(&mut h, 0xAA7A_2746, &["TastyZots"]));
    assert_eq!(v.owner_state().view.filter.unwrap().token, 2);
}

#[test]
fn render_cheat_recorder_and_debug_flags_keep_their_quirks() {
    let mut v = view();
    let mut h = Host::default();
    assert!(!v.process_cheat(&mut h, 0x88F3_D4BB, &["RP"]));
    assert!(h.ops.is_empty());
    assert!(v.process_cheat(&mut h, 0x88F3_D4BC, &["RP", "property"]));
    assert_eq!(
        h.ops,
        [Op::Property("property".into(), "".into()), Op::Message(0xC9F7_75BB)]
    );
    h.ops.clear();
    h.changed = false;
    assert!(v.process_cheat(&mut h, 0x88F3_D4BB, &["RenderProp", "unknown", "1"]));
    assert_eq!(h.ops, [Op::Property("unknown".into(), "1".into())]);
    h.ops.clear();
    v.recorder(&mut h, true);
    v.recorder(&mut h, true);
    v.recorder(&mut h, false);
    v.recorder(&mut h, false);
    assert_eq!(h.ops, [Op::Recorder(true), Op::Recorder(false)]);
    assert!(v.process_cheat(&mut h, 0x4A97_2F37, &["WatchMeMove"]));
    assert!(v.process_cheat(&mut h, 0x4A82_253E, &["You don't deserve it"]));
    assert!(v.owner_state().view.animation_debug);
    assert!(v.owner_state().view.all_rewards);
    assert!(!v.process_cheat(&mut h, 0, &["unknown"]));
}

#[test]
fn cheat_control_allocation_failure_still_replaces_stack() {
    let (mut v, mut h) = controls();
    assert!(v.process_cheat(&mut h, 0x8973_A69C, &["GOL"]));
    assert_eq!(v.input_control(), None);
    assert_eq!(v.persistent_controls(), 2);
    h.scroll_control = Some(7);
    assert!(v.process_cheat(&mut h, 0x4AC0_96CA, &["BuildingPlop"]));
    assert_eq!(v.input_control(), Some(7));
    assert_eq!(h.ops.last(), Some(&Op::Control(7, ControlCall::Release)));
}

#[test]
fn vw_t16_trigger_transitions_and_four_reference_save_roundtrip() {
    let mut v = view();
    let mut h = Host::default();
    v.assert_global_trigger(&mut h, 123);
    v.assert_global_trigger(&mut h, 123);
    v.retract_global_trigger(&mut h, 123);
    v.retract_global_trigger(&mut h, 123);
    v.retract_global_trigger(&mut h, 123);
    assert_eq!(h.ops, [Op::Send(0xAA37_5FE8, 123, 1), Op::Send(0xAA37_5FE8, 123, 0)]);
    assert!(!v.global_trigger_state(123));
    v.assert_global_trigger(&mut h, 123);
    v.assert_global_trigger(&mut h, 123);
    let refs = [0xC9C6_28EC, 0xE9C6_22D8, 0xA9C7_41B3, 0x8695_664E].map(SaveRef::of);
    v.view_state_mut().references = refs;
    let record = v.save_record();
    assert_eq!(record.triggers, [(123, 2)]);
    let mut loaded = view();
    loaded.load_record(&record).unwrap();
    assert_eq!(loaded.owner_state().view.references, refs);
    assert!(!loaded.owner_state().view.not_loaded);
    assert_eq!(loaded.save_record(), record);
    for version in 0..5 {
        let mut older = record.clone();
        older.header.version = version;
        assert!(loaded.load_record(&older).is_ok());
    }
    let mut newer = record;
    newer.header.version = 5;
    assert!(loaded.load_record(&newer).is_err());
}

#[test]
fn level_only_notifies_changes_and_contour_pair_removes_returned_layer() {
    let mut v = view();
    let mut h = Host::default();
    v.set_view_level(&mut h, 1);
    assert!(h.ops.is_empty());
    v.set_view_level(&mut h, 3);
    assert_eq!(h.ops, [Op::Send(0xC6CD_BF1A, 0, 0), Op::Send(0xC6CD_BF1B, 0, 0)]);
    h.ops.clear();
    v.toggle_contours(&mut h);
    v.toggle_contours(&mut h);
    assert_eq!(h.ops, [Op::Contour(true, 0x07F9_4205), Op::Contour(false, 42)]);
    assert_eq!(v.owner_state().view.contour_layer, None);
}

#[test]
fn widgets_refresh_from_notifications_and_visual_clock() {
    let mut v = view();
    let mut h = Host::default();
    v.set_cost_indicator(&mut h, -101, 101);
    for kind in [0x0AB9_9380, 0x0772_FAD4, 0x6695_6814, 0x4477_8233] {
        v.notification(
            &mut h,
            Notification {
                kind,
                ..Notification::default()
            },
        );
    }
    assert_eq!(
        h.widgets,
        [
            WidgetCall::CityName {
                window: 2,
                name: "City".into()
            },
            WidgetCall::Funds(100, [0x44, 0x55, 0x66]),
            WidgetCall::RefreshBudget,
            WidgetCall::Date([3, 4, 5]),
            WidgetCall::Population(1234),
            WidgetCall::RefreshRenderer,
        ]
    );
    assert_eq!(h.lighting, [13.75]);
    h.changed = false;
    h.widgets.clear();
    v.update_ambient_light(&mut h);
    assert!(h.widgets.is_empty());
}

#[test]
fn popup_restores_state_reinvokes_selected_tool_and_balances_teardown() {
    let mut v = view();
    let mut h = Host {
        selection: Some(CatalogItem {
            kind: 0x4A2B_2466,
            item: 1,
        }),
        ..Host::default()
    };
    v.set_current_control(&mut h, Some(77), 0);
    h.ops.clear();
    assert!(v.handle_button_activated(&mut h, 0x2992_0899));
    assert!(h.ops.contains(&Op::Control(77, ControlCall::Shutdown)));
    assert!(h.ops.contains(&Op::Tool(ToolSpec::Zone(1))));
    let layout = h
        .popup_calls
        .iter()
        .find_map(|call| match call {
            PopupCall::Catalog { layout, .. } => Some(layout),
            _ => None,
        })
        .unwrap();
    assert_eq!(layout.remembered, [0, 1]);
    assert_eq!(layout.rows, 1);
    assert_eq!(layout.cell, [44, 44]);
    assert_eq!(layout.spacing, 5);
    assert_eq!(layout.callout, [0xA85F_1A83, 0xE9C8_6C9D]);
    assert_eq!(layout.frame_id, 0x8A6E_61E0);
    h.popup_calls.clear();
    v.handle_button_activated(&mut h, 0x2992_0899); // same kind toggles
    assert_eq!(v.tertiary_kind(), 0);
    assert_eq!(v.owner.lifecycle.remembered_catalogs[&0x20C], [7, 9]);
    assert_eq!(
        h.popup_calls,
        [
            PopupCall::Query(600, 0x8A2C_AD8B),
            PopupCall::Release(601),
            PopupCall::Hide(600),
            PopupCall::Release(600),
            PopupCall::ChildDelete {
                parent: v.as_igzwin(),
                window: 600
            },
        ]
    );
    assert!(!h
        .ops
        .iter()
        .any(|op| matches!(op, Op::Control(77, ControlCall::Event(_)))));
}

#[test]
fn popup_row_limits_tutorial_and_view_level_gate_selection() {
    for (tall, secondary, rows) in [(false, false, 6), (true, false, 8), (true, true, 6)] {
        let mut v = view();
        let mut h = Host {
            moved: tall,
            popup_items: vec![
                CatalogItem {
                    kind: 0x4A2B_2467,
                    item: 0x2992_37BF
                };
                20
            ],
            selection: Some(CatalogItem {
                kind: 0x4A2B_2467,
                item: 0x2992_37BF,
            }),
            ..Host::default()
        };
        v.invoke_catalog_popup(&mut h, secondary, 88, 99, 0x290, CatalogQuery::default());
        assert!(h
            .popup_calls
            .iter()
            .any(|call| matches!(call, PopupCall::Catalog { layout, .. } if layout.rows == rows)));
        assert!(!h.ops.iter().any(|op| matches!(op, Op::Tool(_)))); // subway needs 2
        if secondary {
            v.close_owner_secondary(&mut h);
        } else {
            v.close_owner_tertiary(&mut h);
        }
        v.owner.view.level = 2;
        h.ops.clear();
        v.invoke_catalog_popup(&mut h, secondary, 88, 99, 0x290, CatalogQuery::default());
        assert!(h.ops.contains(&Op::Tool(ToolSpec::Network(7))));
        if secondary {
            v.close_owner_secondary(&mut h);
        } else {
            v.close_owner_tertiary(&mut h);
        }
        h.ops.clear();
        h.tutorial = false;
        v.invoke_catalog_popup(&mut h, secondary, 88, 99, 0x290, CatalogQuery::default());
        assert!(!h.ops.iter().any(|op| matches!(op, Op::Tool(_))));
    }
}

#[test]
fn scripted_popup_filters_alignment_failure_and_secondary_close_order() {
    let mut v = view();
    let mut h = Host::default();
    v.handle_button_activated(&mut h, 0x6991_EE42);
    v.handle_button_activated(&mut h, 0xAB95_37B7);
    assert!(h.popup_calls.contains(&PopupCall::Align {
        window: 0xCB95_403E,
        anchor: 0xAAAA,
        target: 0xAB95_37B7,
        flags: 0,
        margin: -75
    }));
    assert!(h.popup_calls.contains(&PopupCall::Filter(0xCB95_403E, true)));
    h.popup_calls.clear();
    v.close_owner_secondary(&mut h);
    let hidden: Vec<_> = h
        .popup_calls
        .iter()
        .filter_map(|call| match call {
            PopupCall::Hide(token) => Some(*token),
            _ => None,
        })
        .collect();
    assert_eq!(hidden, [0xCB95_403E, 0x8993_02FC]);
    assert_eq!((v.secondary_kind(), v.tertiary_kind()), (0, 0));
    h.filter_allocation = false;
    v.handle_button_activated(&mut h, 3);
    assert_eq!(v.tertiary_win(), None);
    assert!(v.owner.popups.iter().all(Option::is_none));
}

#[test]
fn button_filter_executes_commands_and_day_night_preferences_in_order() {
    let mut v = view();
    let mut h = Host::default();
    assert!(v.filter_message(
        &mut h,
        WindowMessage {
            kind: 3,
            data: [0x2872_59F6, 0x2A4F_BB08, 0],
            ..WindowMessage::default()
        }
    ));
    assert_eq!(h.popup_calls, [PopupCall::Execute(0x6A93_5CC9)]);
    h.popup_calls.clear();
    v.handle_button_activated(&mut h, 0xCA35_CB76);
    assert_eq!(
        h.popup_calls,
        [
            PopupCall::DayNightNotifications,
            PopupCall::DayNightPreference(1),
            PopupCall::DayNightButton(0xCA35_CB74, false),
            PopupCall::DayNightButton(0xCA35_CB76, true),
            PopupCall::DayNightButton(0xCA35_CB78, false)
        ]
    );
    assert_eq!(h.ops, [Op::Action(ActionCall::VisualTimeMode(1))]);
}

#[test]
fn save_uses_opaque_identities_subobjects_and_load_interfaces_before_init() {
    let mut v = view();
    let mut h = Host::default();
    for (field, token) in [(0xF0, 10), (0xF4, 20), (0xF8, 30), (0x154, 40)] {
        v.owner.lifecycle.objects.insert(field, token);
    }
    *v.owner.view.triggers.entry(99, || 2) = 2;
    assert_eq!(v.save_object_record(&mut h, false), None);
    assert!(h.reference_calls.is_empty());
    let record = v.save_object_record(&mut h, true).unwrap();
    assert_eq!(h.reference_calls, [(10, 0), (20, 8), (30, 4), (40, 0)]);
    assert_eq!(
        record.identities,
        [Some(0x1000A), Some(0x10014), Some(0x1001E), Some(0x10028)]
    );
    let mut loaded = view();
    h.reference_calls.clear();
    loaded.load_object_record(&mut h, &record).unwrap();
    assert_eq!(
        h.reference_calls,
        [
            (0x1000A, 0xC9C6_28EC),
            (0x10014, 0xE9C6_22D8),
            (0x1001E, 0xA9C7_41B3),
            (0x10028, 0x8695_664E)
        ]
    );
    assert_eq!(loaded.renderer(), Some(20));
    assert_eq!(loaded.model_maker(), Some(30));
    assert!(!loaded.owner.view.not_loaded);
    assert_eq!(loaded.owner.view.triggers.get(99), Some(&2));
    let mut bad = record.clone();
    bad.header.3 = 5;
    h.reference_calls.clear();
    assert!(loaded.load_object_record(&mut h, &bad).is_err());
    assert!(h.reference_calls.is_empty());
    h.read_record = Some(record);
    loaded.notification(
        &mut h,
        Notification {
            kind: 0x26C6_3341,
            data: [33, 0],
            ..Notification::default()
        },
    );
    loaded.notification(
        &mut h,
        Notification {
            kind: 0x26C6_3344,
            data: [33, 0],
            ..Notification::default()
        },
    );
    assert_eq!(h.written_records.len(), 1);
    assert_eq!(h.written_records[0].as_ref().unwrap().triggers, [(99, 2)]);
}

#[test]
fn background_first_valid_image_releases_all_loads_and_picking_is_null_safe() {
    let mut v = view();
    let mut h = Host {
        pictures: vec![
            BackgroundPicture {
                token: 1,
                width: 63,
                height: 63,
            },
            BackgroundPicture {
                token: 2,
                width: 64,
                height: 32,
            },
            BackgroundPicture {
                token: 3,
                width: 64,
                height: 64,
            },
            BackgroundPicture {
                token: 4,
                width: 128,
                height: 128,
            },
        ],
        ..Host::default()
    };
    assert!(!v.add_background_image(&mut h, true));
    assert_eq!(v.owner.background_mode, 0);
    v.owner.lifecycle.objects.extend([(0x150, 10), (0xF4, 20)]);
    assert!(v.add_background_image(&mut h, true));
    assert_eq!(v.owner.background_mode, 2);
    assert_eq!(h.helper_calls, [(1, 0xFFFF), (2, 0xFFFF), (3, 1), (3, 0xFFFF)]);
    let mut point = [1.0; 3];
    v.owner.lifecycle.objects.remove(&0xF4);
    assert!(!v.pick_terrain(&mut h, 1, 2, 0, &mut point));
    assert_eq!(point, [0.0; 3]);
    v.owner.lifecycle.objects.insert(0xF4, 20);
    assert!(v.pick_occupant(&mut h, 1, 2, &mut point));
    assert_eq!(point, [10.0, 20.0, 30.0]);
    assert_eq!(
        &h.helper_calls[h.helper_calls.len() - 2..],
        [(0x26BB_767E, 1), (0x26BB_767E, 0)]
    );
    h.terrain = None;
    assert!(!v.pick_occupant(&mut h, 1, 2, &mut point));
    assert_eq!(point, [0.0; 3]);
}

#[test]
fn helper_stop_branch() {
    let v = view();
    let mut h = Host::default();
    v.stop_foreign_helper(&mut h, 99);
    assert_eq!(h.helper_calls, [(99, 4), (99, 0)]);
    h.helper_calls.clear();
    h.changed = false;
    v.stop_foreign_helper(&mut h, 99);
    assert_eq!(h.helper_calls, [(99, 4)]);
}

#[test]
fn establish_result_character_limit_and_bankruptcy_gate_does_not_save() {
    let mut v = view();
    let mut h = Host::default();
    assert!(v.do_establish_city_dialog(&mut h));
    assert_eq!(
        &h.dialog_calls[1..],
        [
            DialogCall::Difficulty(2),
            DialogCall::CityName("é".repeat(511)),
            DialogCall::CityNamePreference("é".repeat(511)),
            DialogCall::NameChanged,
            DialogCall::MayorPreference("Mayor".into()),
            DialogCall::SavePreferences,
            DialogCall::EstablishCity,
        ]
    );
    h.dialog_calls.clear();
    h.changed = false;
    assert!(!v.do_establish_city_dialog(&mut h));
    assert_eq!(h.dialog_calls.len(), 1);
    h.dialog_calls.clear();
    h.filter_allocation = false;
    v.do_budget_failure(&mut h);
    assert!(h.dialog_calls.is_empty());
    assert_eq!(h.ops, [Op::Action(ActionCall::Region { prompt: false })]);
}

#[test]
fn accelerator_replacement_acquires_before_release_and_null_clears() {
    let mut v = view();
    let mut h = Host::default();
    v.set_key_accelerator(&mut h, Some(5));
    h.ops.clear();
    v.set_key_accelerator(&mut h, Some(5));
    assert_eq!(
        h.ops,
        [
            Op::Lifecycle(LifecycleCall::AddRef(5)),
            Op::Lifecycle(LifecycleCall::Release(5))
        ]
    );
    assert_eq!(v.key_accelerator(), Some(5));
    v.set_key_accelerator(&mut h, None);
    assert_eq!(v.key_accelerator(), None);
}

impl StatusHost for Host {
    fn cost_visibility(&mut self, visible: bool) {
        self.cost_visibility.push(visible);
    }
    fn funds(&self) -> i32 {
        self.funds_value
    }
    fn draw_cost(&mut self, display: CostDisplay) {
        self.ops.push(Op::CostDisplay(display));
    }
    fn cursor_callout(&mut self, _: Option<&CursorText>, _: (i32, i32)) {}
    fn project(&mut self, point: [f32; 3]) -> (i32, i32) {
        (point[0] as i32, point[2] as i32)
    }
    fn clock_time(&self) -> (u32, u32) {
        (3, 15)
    }
    fn clock_widgets(&mut self, h: u32, m: u32, f: u32) {
        self.ops.push(Op::Clock(h, m, f));
    }
    fn clock_hover_active(&self) -> bool {
        self.changed
    }
    fn clock_hover_widgets(&mut self, visible: bool, hour: u32, minute: u32) {
        self.hover.push((visible, hour, minute));
    }
    fn rating_widgets(&mut self, col: i32, delta: i32) {
        self.ops.push(Op::Rating(col, delta));
    }
    fn alert_picture(&mut self, picture: Option<u32>) {
        self.ops.push(Op::Alert(picture));
    }
    fn speed_button(&mut self, id: u32, toggle: bool, lower: bool) {
        self.ops.push(Op::Speed(id, toggle, lower));
    }
    fn create_mission_panel(&mut self, id: u32, margin: i32) -> Option<u32> {
        assert_eq!(margin, 8);
        Some(id)
    }
    fn delete_mission_panel(&mut self, _: u32) {}
    fn mission_text(&mut self, _: u32, time: &str, red: bool, _: &str) {
        self.ops.push(Op::Mission(time.into(), red));
    }
    fn active_mission_status(&mut self) -> Option<MissionStatus> {
        self.mission_status.clone()
    }
    fn mission_fallback(&mut self, group: u32, instance: u32) -> String {
        assert_eq!((group, instance), (0x6A23_1EA4, 0xAC2B_D618));
        "localized fallback".into()
    }
    fn mission_update(&mut self, panel: u32, time: &str, red: bool, progress: Option<&str>) {
        self.mission_updates
            .push((panel, time.into(), red, progress.map(str::to_owned)));
    }
}

#[test]
fn vw_t17_clock_frame_and_out_of_range_hour_fallback() {
    let v = view();
    let mut h = Host::default();
    v.update_24hour_clock(&mut h, 13, 45);
    v.update_24hour_clock(&mut h, 24, 59);
    assert_eq!(h.ops, [Op::Clock(13, 45, 27), Op::Clock(3, 15, 6)]);
}

#[test]
fn vw_t18_cost_red_when_above_funds_fades_frozen_for_1500ms() {
    let mut v = view();
    let mut h = Host::default();
    v.update_cursor_position(&mut h, 100, 200);
    v.set_cost_indicator(&mut h, -200, 101);
    let cost = v.owner_state().display.cost;
    assert_eq!(cost.colour, [255, 40, 40]);
    assert_eq!(cost.alpha, 255);
    assert_eq!(cost.position, (85, 185));
    let old_len = h.ops.len();
    v.set_cost_indicator(&mut h, -200, 101);
    assert_eq!(h.ops.len(), old_len);
    v.clear_cost_indicator(&mut h, true);
    v.update_cursor_position(&mut h, 300, 400);
    v.update_cost_indicator(&mut h, 0.75);
    let halfway = v.owner_state().display.cost;
    assert_eq!(halfway.mode, CostMode::Fading);
    assert_eq!(halfway.position, (100, 99));
    assert_eq!(halfway.colour, [255; 3]);
    assert!(halfway.alpha < cost.alpha);
    v.update_cost_indicator(&mut h, 0.75);
    assert_eq!(v.owner_state().display.cost.mode, CostMode::Hidden);
    assert_eq!(v.owner_state().display.cost.alpha, 0);
    v.set_cost_amount(&mut h, 100);
    assert_eq!(v.owner_state().display.cost.alpha, 150);
}

#[test]
fn rating_clamps_and_alert_priority_disaster_situation_pause() {
    let mut v = view();
    let mut h = Host::default();
    v.update_mayor_rating(&mut h, 127, 100);
    v.update_mayor_rating(&mut h, -128, 100);
    v.update_mayor_rating(&mut h, 0, 0);
    assert_eq!(h.ops, [Op::Rating(99, 21), Op::Rating(0, -21)]);
    h.ops.clear();
    v.update_alert_border(&mut h, true, true, true);
    v.update_alert_border(&mut h, false, true, true);
    v.update_alert_border(&mut h, false, false, true);
    v.update_alert_border(&mut h, false, false, false);
    assert_eq!(
        h.ops,
        [
            Op::Alert(Some(0x1431_5E60)),
            Op::Alert(Some(0x1431_5E62)),
            Op::Alert(Some(0x1431_5E61)),
            Op::Alert(None)
        ]
    );
}

#[test]
fn mission_warning_thresholds_and_speed_paused_picture_half() {
    let mut v = view();
    let mut h = Host::default();
    v.show_mission_status_panel(&mut h, 42);
    v.show_mission_status_panel(&mut h, 43);
    assert_eq!(v.owner_state().display.mission_panel, Some(42));
    for seconds in [60, 59, 21, 20, 19, 0] {
        v.update_mission_status_panel(&mut h, seconds, "target");
    }
    assert_eq!(
        h.ops,
        [
            Op::Mission("01:00".into(), false),
            Op::Mission("00:59".into(), true),
            Op::Mission("00:21".into(), true),
            Op::Mission("00:20".into(), false),
            Op::Mission("00:19".into(), true),
            Op::Mission("00:00".into(), false)
        ]
    );
    v.hide_mission_status_panel(&mut h);
    assert_eq!(v.owner_state().display.mission_panel, None);
    h.ops.clear();
    v.update_speed_buttons(&mut h, 1, true);
    assert_eq!(
        h.ops,
        [
            Op::Speed(0x6A4F_BB31, false, true),
            Op::Speed(0x4A4F_BB60, true, true),
            Op::Speed(0x8998_BBDF, false, true)
        ]
    );
}

impl CommandHost for Host {
    fn city_established(&self) -> bool {
        self.established
    }
    fn edge_scrolling_preference(&self) -> bool {
        true
    }
    fn simulator_paused(&self) -> bool {
        false
    }
    fn pause_simulator(&mut self) {}
    fn resume_simulator(&mut self) {}
    fn set_sim_speed(&mut self, _: u32) {}
    fn refresh_speed_buttons(&mut self) {}
    fn close_secondary(&mut self) {
        self.ops.push(Op::Close(false));
    }
    fn close_tertiary(&mut self) {
        self.ops.push(Op::Close(true));
    }
    fn tab_toggled(&self, id: u32) -> bool {
        self.tabs.get(&id).copied().unwrap_or(false)
    }
    fn toggle_tab(&mut self, id: u32, on: bool) {
        self.tabs.insert(id, on);
        self.ops.push(Op::Tab(id, on));
    }
    fn panel_buttons(&mut self, id: u32, on: bool) {
        self.ops.push(Op::Panel(id, on));
    }
    fn mayor_subpanel(&mut self, _: u32, _: bool) {}
    fn mayor_subpanel_visible(&self, _: u32) -> bool {
        self.changed
    }
    fn mysim_visible(&mut self, _: bool) {}
    fn advisor_visible(&mut self, _: bool) {}
    fn news_ticker_visible(&mut self, _: bool) {}
    fn control_panel_visible(&mut self, _: bool) {}
    fn restore_button_visible(&mut self, _: bool) {}
    fn auxiliary_ui_visible(&mut self, _: bool) {}
    fn rotation_ui_visible(&mut self, _: bool) {}
    fn create_query_tool(&mut self, _: QueryTool) -> Option<u32> {
        Some(7)
    }
}

#[test]
fn vw_t14_god_panel_override_and_toggle_close() {
    for (mods, established, expected) in [(0, true, 4), (7, true, 0), (0, false, 0)] {
        let mut v = view();
        let mut h = Host {
            established,
            ..Host::default()
        };
        v.owner.modifiers = mods;
        assert!(!v.process_motion_panel_command(&mut h, 0x6A93_5C34));
        assert_eq!(v.owner_state().panels.current, Some(expected));
        assert!(h.ops.contains(&Op::Panel(expected, true)));
        assert!(!v.process_motion_panel_command(&mut h, 0x6A93_5C34));
        assert_eq!(v.owner_state().panels.current, None);
        assert!(h.ops.contains(&Op::Panel(expected, false)));
    }
}

#[test]
fn vw_t15_query_tool_twice_is_one_tool_and_cancel_closes_both_menus() {
    let mut v = view();
    let mut h = Host::default();
    assert!(!v.process_motion_panel_command(&mut h, 0x6A93_5CF3));
    assert!(!v.process_motion_panel_command(&mut h, 0x6A93_5CF3));
    assert_eq!(v.owner.stack, [7]);
    assert_eq!(
        h.ops
            .iter()
            .filter(|op| **op == Op::Control(7, ControlCall::Init))
            .count(),
        1
    );
    h.ops.clear();
    v.set_secondary(Some(100), 1);
    v.set_tertiary(Some(101), 2);
    assert!(!v.process_motion_panel_command(&mut h, 0x6A93_5CF1));
    assert!(v.owner.stack.is_empty());
    assert_eq!(&h.ops[h.ops.len() - 2..], &[Op::Close(true), Op::Close(false)]);
    h.established = false;
    v.process_motion_panel_command(&mut h, 0x6A93_5CF3);
    assert!(v.owner.stack.is_empty());
}

#[test]
fn rotate_cw_is_left_and_stop_command_stops_any_axis() {
    let mut v = view();
    let mut h = Host::default();
    v.process_motion_panel_command(&mut h, 0x6A93_5CC9);
    v.process_motion_panel_command(&mut h, 0x6A93_5CCC);
    assert!(h.ops.contains(&Op::View(ViewChange::RotateLeft)));
    assert!(h.ops.contains(&Op::View(ViewChange::RotateRight)));
    v.process_motion_panel_command(&mut h, 0x6A93_5CD8);
    v.process_motion_panel_command(&mut h, 0x6A93_5CDD);
    assert_eq!(v.owner_state().motion.keyboard, [-1, 0]);
    v.process_motion_panel_command(&mut h, 0x2A94_8272);
    assert_eq!(v.owner_state().motion.keyboard, [0, 0]);
    assert!(v.owner_state().motion.stopping);
}
