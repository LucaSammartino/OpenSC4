// SPDX-License-Identifier: GPL-3.0-or-later
// Included in owner_tests to share the foreign-interface recorder.

#[test]
fn vw_t26_dormant_fields_and_panel_extras() {
    let mut v = view();
    let mut h = Host::default();
    v.add_gui(&mut h);
    for field in [0x184, 0x188, 0x2EC, 0x2F0, 0x410] {
        assert!(!v.owner.lifecycle.objects.contains_key(&field));
        assert!(!v.gui_state().fields.contains_key(&field));
    }
    for (panel, child) in [(1, 0xE988_9775), (2, 0xABB2_6B0E)] {
        assert!(v.gui_state().panels[panel].contains(&child));
        let flag = h
            .ops
            .iter()
            .position(|o| *o == Op::Gui(GuiCall::Flag(child, 0x800, true)))
            .unwrap();
        assert_eq!(
            &h.ops[flag..flag + 4],
            &[
                Op::Gui(GuiCall::Flag(child, 0x800, true)),
                Op::Gui(GuiCall::SendToBack(child)),
                Op::Gui(GuiCall::Flag(child, 0x800, false)),
                Op::Gui(GuiCall::ClearFocus(child)),
            ]
        );
        assert!(h.ops.contains(&Op::Gui(GuiCall::Filter(child, true))));
        assert!(h
            .ops
            .contains(&Op::Gui(GuiCall::SetWinProc(child, v.gui_state().fields[&0x2D8]))));
    }
    assert!(h
        .ops
        .contains(&Op::Gui(GuiCall::SetWinProc(0xCA1F_1D9C, v.gui_state().fields[&0x2E4]))));

    let mut v = view();
    h.rejected_gui.push(GuiCall::RecursivePanelChild {
        parent: 0x69E4_0A1F,
        id: 0xE988_9775,
        iid: 0x5386_D516,
    });
    v.add_gui(&mut h);
    assert!(!v.gui_state().panels[1].contains(&0xE988_9775));
    assert!(v.gui_state().panels[2].contains(&0xABB2_6B0E));
}

#[test]
fn vw_t27_factory_fallback_graphics_gate_and_reuse() {
    let mut v = view();
    let mut h = Host {
        rejected_city: vec![
            CityCall::Create(CityObject::Camera),
            CityCall::QueryGraphics(0xC47B_747C),
        ],
        ..Host::default()
    };
    v.pre_city_init(&mut h);
    assert!(h.ops.contains(&Op::City(CityCall::Construct(CityObject::Camera))));
    assert!(!h
        .ops
        .iter()
        .any(|o| matches!(o, Op::City(CityCall::WireRenderer { .. }))));
    assert!(v.gui_state().fields.contains_key(&0x18C));
    assert_eq!(v.renderer(), Some(CityObject::Renderer.class_id()));
    h.ops.clear();
    h.rejected_city.clear();
    v.pre_city_init(&mut h);
    assert!(!h
        .ops
        .iter()
        .any(|o| matches!(o, Op::City(CityCall::Create(_) | CityCall::Construct(_)))));
    assert!(h
        .ops
        .iter()
        .any(|o| matches!(o, Op::City(CityCall::WireRenderer { .. }))));
    assert!(h.ops.contains(&Op::Lifecycle(LifecycleCall::Release(0xC47B_747C))));
}

#[test]
fn vw_t27_required_fallback_failure_is_fatal() {
    let mut v = view();
    let mut h = Host {
        rejected_city: vec![
            CityCall::Create(CityObject::Camera),
            CityCall::Construct(CityObject::Camera),
        ],
        ..Host::default()
    };
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| v.pre_city_init(&mut h))).is_err());
    assert!(!h.ops.iter().any(|o| matches!(o, Op::Gui(_))));
}

#[test]
fn vw_t27_overlay_survives_remove_gui_until_base_shutdown() {
    let mut v = view();
    let mut h = Host::default();
    v.add_gui(&mut h);
    let overlay = 0xCA5D_3294;
    h.ops.clear();
    v.remove_gui(&mut h);
    assert!(!h.ops.contains(&Op::Gui(GuiCall::Destroy(overlay))));
    assert!(!h.ops.contains(&Op::Gui(GuiCall::Release(overlay))));
    assert!(!h.ops.contains(&Op::Gui(GuiCall::BaseShutdown)));
    v.shutdown(&mut h);
    assert_eq!(h.ops.last(), Some(&Op::Gui(GuiCall::BaseShutdown)));
}

#[test]
fn required_gui_helpers_and_panel_qi_are_fatal() {
    for call in [
        GuiCall::Helper(HelperKind::PanelList),
        GuiCall::Helper(HelperKind::NewsTicker),
        GuiCall::Helper(HelperKind::MySim),
        GuiCall::Helper(HelperKind::Budget),
        GuiCall::Helper(HelperKind::Callout),
        GuiCall::Helper(HelperKind::Bmp),
        GuiCall::Helper(HelperKind::AdvisorTracker),
        GuiCall::Query(0x0987_B48F, 0x5386_D516),
        GuiCall::Query(0xC991_EDA8, 0x5386_D516),
    ] {
        let mut v = view();
        let mut h = Host::default();
        h.rejected_gui.push(call);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| v.add_gui(&mut h))).is_err());
    }
}

#[test]
fn dormant_helpers_stop_but_dormant_reference_only_releases() {
    let mut v = view();
    let mut h = Host::default();
    for (offset, token) in [(0x184, 41), (0x188, 42), (0x410, 43)] {
        v.owner.lifecycle.objects.insert(offset, token);
    }
    v.shutdown_helpers(&mut h);
    assert!(h.ops.contains(&Op::Lifecycle(LifecycleCall::Stop(41))));
    assert!(h.ops.contains(&Op::Lifecycle(LifecycleCall::Stop(42))));
    assert!(!h.ops.contains(&Op::Lifecycle(LifecycleCall::Stop(43))));
    assert!(!h.ops.contains(&Op::Lifecycle(LifecycleCall::Release(43))));
    assert_eq!(v.owner.lifecycle.objects.get(&0x410), Some(&43));
    v.destroy(&mut h);
    v.destroy(&mut h); // destruction-only ownership is consumed once
    for token in [41, 42, 43] {
        assert_eq!(
            h.ops
                .iter()
                .filter(|o| **o == Op::Lifecycle(LifecycleCall::Release(token)))
                .count(),
            1
        );
    }
}

#[test]
fn vw_t28_bus_and_pipe_buttons_dispatch_commands_without_parameters() {
    let mut v = view();
    let mut h = Host::default();
    v.handle_button_activated(&mut h, 0xE992_37B4);
    v.handle_button_activated(&mut h, 0x2992_F72C);
    assert_eq!(
        h.popup_calls,
        [
            PopupCall::Execute(0x6A93_5DC9),
            PopupCall::TutorialButton(0xE992_37B4),
            PopupCall::Execute(0x6A93_5DEE),
            PopupCall::TutorialButton(0x2992_F72C)
        ]
    );
    assert!(h.ops.is_empty());
}

#[test]
fn vw_t32_landscape_scripts_keep_caller_ids_and_transport_has_no_memory() {
    for button in [0x36, 0x8A32_DDDB] {
        let mut v = view();
        let mut h = Host::default();
        assert!(v.handle_button_activated(&mut h, button));
        assert!(h.popup_calls.contains(&PopupCall::Script {
            parent: v.as_igzwin(),
            group: 0x96A0_06B0,
            instance: 0xAAA4_4448,
            data: button,
        }));
        assert_eq!(v.secondary_kind(), 0xCA35_CBED);
    }
    let mut v = view();
    let mut h = Host::default();
    assert!(v.handle_button_activated(&mut h, 0xABB2_7B91));
    assert_eq!(h.popup_calls, [PopupCall::TutorialButton(0xABB2_7B91)]);
    assert!(h.ops.is_empty());
    for button in [0x2B79_E398, 0xEA6E_379D] {
        assert!(v.handle_button_activated(&mut h, button));
        assert_eq!(v.tertiary_kind(), button);
        assert_eq!(v.owner.popups[1].unwrap().memory, None);
        assert!(!v.owner.lifecycle.remembered_catalogs.contains_key(&button));
        assert!(h
            .popup_calls
            .iter()
            .any(|c| matches!(c, PopupCall::Catalog { layout, .. }
            if layout.kind == button && layout.background == 0x1421_5ED2)));
    }
}

#[test]
fn cost_fade_redraw_does_not_show_hidden_bmp() {
    let mut v = view();
    let mut h = Host::default();
    v.clear_cost_indicator(&mut h, false);
    assert_eq!(h.cost_visibility, [false]);
    v.clear_cost_indicator(&mut h, true);
    v.update_cost_indicator(&mut h, 0.5);
    assert_eq!(h.cost_visibility, [false]);
    v.update_cost_indicator(&mut h, 1.0);
    assert_eq!(h.cost_visibility, [false, false]);
    v.set_cost_amount(&mut h, 4);
    assert_eq!(h.cost_visibility, [false, false, true]);
}

#[test]
fn vw_t36_background_refresh_latches_mode_and_preserves_it_on_failure() {
    let mut v = view();
    let mut h = Host::default();
    for field in [0x150, 0xF4] {
        v.owner.lifecycle.objects.insert(field, field);
    }
    h.pictures.push(BackgroundPicture {
        token: 1,
        width: 64,
        height: 64,
    });
    assert!(v.refresh_background(&mut h));
    assert_eq!(v.owner.background_mode, 1);
    assert_eq!(h.background_requests, [(false, "Background2D".into())]);
    h.background_requests.clear();
    assert!(v.refresh_background(&mut h));
    assert_eq!(h.background_requests, [(false, "Background2D".into())]);
    v.owner.background_mode = 2;
    h.background_requests.clear();
    assert!(v.refresh_background(&mut h));
    assert_eq!(h.background_requests, [(true, "Background3D3".into())]);
    assert!(h.helper_calls.contains(&(1, 1)));
    h.pictures.clear();
    h.helper_calls.clear();
    assert!(!v.refresh_background(&mut h));
    assert_eq!(v.owner.background_mode, 2);
    assert!(h.helper_calls.is_empty());
}
