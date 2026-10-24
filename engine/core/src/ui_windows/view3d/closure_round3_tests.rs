// SPDX-License-Identifier: GPL-3.0-or-later
// Included by owner_tests.rs: exercise owner integration through foreign traits.

#[derive(Debug, PartialEq, Eq)]
enum AdviceCall {
    Resource(u32, u32, u32),
    Activate(u32, u32, u32),
    Release(u32),
    HideDialog,
    Graph(u32, u32),
    Budget(u32),
    Dispatch(u32, WindowMessage),
}

impl GraphHost for Host {
    fn graph_resource(&mut self, instance: u32, group: u32) -> Option<u32> {
        self.graph.graph_resource(instance, group)
    }
    fn graph_reset(&mut self, resource: u32, index: i32, value: u32) {
        self.graph.graph_reset(resource, index, value);
    }
    fn graph_prepare(&mut self, resource: u32) {
        self.graph.graph_prepare(resource);
    }
    fn graph_series(&mut self, resource: u32, series: u32, colour: u32) {
        self.graph.graph_series(resource, series, colour);
    }
    fn graph_release(&mut self, resource: u32) {
        self.graph.graph_release(resource);
    }
}

impl CoverageHost for Host {
    fn occupants(&mut self, category: Category) -> Vec<u32> {
        self.coverage.occupants(category)
    }
    fn radius(&mut self, category: Category, occupant: u32, factor: f32) -> Option<f32> {
        self.coverage.radius(category, occupant, factor)
    }
    fn position(&mut self, occupant: u32) -> Option<[f32; 3]> {
        self.coverage.position(occupant)
    }
    fn release_occupant(&mut self, occupant: u32) {
        self.coverage.release_occupant(occupant);
    }
    fn create_effect(&mut self, name: &'static str) -> Option<u32> {
        self.coverage.create_effect(name)
    }
    fn transform(&mut self, effect: u32, transform: CoverageTransform) {
        self.coverage.transform(effect, transform);
    }
    fn start(&mut self, effect: u32, argument: u32) {
        self.coverage.start(effect, argument);
    }
    fn running(&mut self, effect: u32) -> bool {
        self.coverage.running(effect)
    }
    fn stop(&mut self, effect: u32, argument: u32) {
        self.coverage.stop(effect, argument);
    }
    fn release_effect(&mut self, effect: u32) {
        self.coverage.release_effect(effect);
    }
}

#[test]
fn vw_t33_mission_uses_borrowed_attachment_and_active_situation() {
    let mut v = view();
    let mut h = Host {
        mission_status: Some(MissionStatus {
            seconds: 59,
            progress: "active target".into(),
        }),
        ..Host::default()
    };
    // Neither manager nor panel can be replaced by interpreting attachment A.
    v.update_mission_progress(&mut h, Some(123), 7);
    assert!(h.mission_updates.is_empty());
    v.show_mission_status_panel(&mut h, 42);
    for (kind, attachment, data1, flag, progress) in [
        (0xABBB_CA64, Some(123), 7, 7, Some("active target")),
        (0xABBB_CA64, None, 1, 1, Some("localized fallback")),
        (0xAC08_3D7D, Some(123), 99, 1, Some("active target")),
        (0xABBB_CA64, Some(123), 0, 0, None),
    ] {
        h.ops.clear();
        v.notification(
            &mut h,
            Notification {
                kind,
                attachment,
                data: [data1, 0],
                ..Notification::default()
            },
        );
        assert!(h.ops.contains(&Op::Window(MessageCall::Refresh(Refresh::Mission(
            attachment.unwrap_or(0),
            flag
        )))));
        assert_eq!(
            h.mission_updates.last(),
            Some(&(42, "00:59".into(), true, progress.map(str::to_owned)))
        );
        assert!(!h.ops.iter().any(|op| matches!(
            op,
            Op::Lifecycle(LifecycleCall::AddRef(123) | LifecycleCall::Release(123))
        )));
    }
    h.mission_status = None;
    let count = h.mission_updates.len();
    v.update_mission_progress(&mut h, None, 1);
    assert_eq!(h.mission_updates.len(), count);
}

fn advice_view() -> View3DWin {
    let mut v = view();
    v.owner
        .gui
        .fields
        .extend([(0x18C, 11), (0x300, 12), (0x2F8, 13), (0x2E8, 14)]);
    v
}

#[test]
fn vw_t34_advice_destinations_and_literal_envelopes() {
    for (kind, selector, panel, window, buttons) in [
        (0x6A5B_DFCD, 123, 0x9988_7755, 12, vec![0x4A32_CA92, 123]),
        (0xEA5B_DFBC, 0x4A5B_DFB8, 0x41, 0x41, vec![0xAA3A_C40E]),
        (0xEA5B_DFBC, 0xCA5B_DFB0, 0x41, 0x41, vec![0xAA3A_C40F]),
        (0xEA5B_DFBC, 123, 0x1520_0003, 0x41, vec![123]),
    ] {
        for none_shown in [false, true] {
            let mut v = advice_view();
            let mut h = Host::default();
            v.owner.panels.mayor_none_shown = none_shown;
            assert!(v.window_message(
                &mut h,
                WindowMessage {
                    kind,
                    data: [selector, 999, 888],
                    ..WindowMessage::default()
                }
            ));
            assert_eq!(v.owner.panels.mayor_subpanel, panel);
            assert_eq!(
                &h.advice_calls[..3],
                &[
                    AdviceCall::Resource(11, 0xC988_BC79, 0x8810),
                    AdviceCall::Activate(77, 1, 1),
                    AdviceCall::Release(77)
                ]
            );
            let dispatched: Vec<_> = h
                .advice_calls
                .iter()
                .filter_map(|call| {
                    if let AdviceCall::Dispatch(w, m) = call {
                        Some((*w, m.kind, m.data))
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(
                dispatched,
                buttons
                    .into_iter()
                    .map(|button| (window, 3, [0x2872_59F6, button, 0]))
                    .collect::<Vec<_>>()
            );
            if none_shown {
                assert!(!h.advice_calls.contains(&AdviceCall::HideDialog));
            }
            if kind == 0xEA5B_DFBC && selector == 123 {
                assert!(!h.advice_calls.contains(&AdviceCall::HideDialog));
            } else {
                assert!(!v.owner.panels.mayor_none_shown);
            }
        }
    }
    let mut v = advice_view();
    let mut h = Host::default();
    v.handle_advice_link(&mut h, 0x6A72_0993, 456);
    assert_eq!(v.owner.panels.mayor_subpanel, 0x1520_0002);
    assert_eq!(h.advice_calls.last(), Some(&AdviceCall::Graph(13, 456)));
    h.advice_available = false;
    h.advice_calls.clear();
    h.ops.clear();
    v.handle_advice_link(&mut h, 0x6A5B_DFCD, 123);
    assert_eq!(h.advice_calls, [AdviceCall::Resource(11, 0xC988_BC79, 0x8810)]);
    assert!(h.ops.is_empty());
    v.owner.gui.fields.remove(&0x18C);
    h.advice_calls.clear();
    v.handle_advice_link(&mut h, 0x6A72_0993, 123);
    assert!(h.advice_calls.is_empty());
}

#[test]
fn coverage_init_shutdown_and_graph_add_gui_are_owner_operations() {
    let mut v = view();
    let mut h = Host::default();
    v.owner.coverage.categories = 1;
    h.coverage.occupants = vec![1];
    h.coverage.radius = Some(12.0);
    h.coverage.position = Some([0.0; 3]);
    assert!(v.init(&mut h));
    assert_eq!(h.coverage.listed, [Category::Police]);
    assert_eq!(h.coverage.created, ["PlopMode_Police_Existing"]);
    h.graph.resource = Some(77);
    v.add_gui(&mut h);
    assert_eq!(h.graph.calls.len(), 12);
    assert_eq!(&h.graph.calls[..3], &[(8, -1, 0), (6, 0, 0), (7, 1, 0x00FF00)]);
    assert_eq!(h.graph.calls.last(), Some(&(0, 0, 0)));
    v.shutdown(&mut h);
    assert_eq!(h.coverage.stopped, [77]);
    assert_eq!(h.coverage.released, [77]);
}

#[test]
fn vw_t29_owner_posts_whole_map_requests_without_installing_controls() {
    let mut v = view();
    let mut h = Host::default();
    for (button, data) in [(0x0AA4_4502, 0), (0x0AA4_4503, 1)] {
        h.popup_calls.clear();
        assert!(v.handle_button_activated(&mut h, button));
        assert_eq!(
            h.popup_calls,
            [PopupCall::TerrainSound, PopupCall::TerrainPost(0x470B_D214, data)]
        );
        assert!(v.input_control().is_none());
    }
}
