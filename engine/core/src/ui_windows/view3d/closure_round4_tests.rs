// SPDX-License-Identifier: GPL-3.0-or-later
// Integration vectors for the remaining settled owner branches.

#[test]
fn registry_tool_commands_use_literal_arguments_and_control_lifetime() {
    // VW9 / §6g, independently specified command -> zone/network/lot vectors.
    let zones = [
        (0x6A93_5CF7, 1),
        (0x6A93_5D0B, 2),
        (0x6A93_5D0E, 3),
        (0x6A93_5D11, 4),
        (0x6A93_5D17, 5),
        (0x6A93_5D1A, 6),
        (0x6A93_5D1D, 7),
        (0x6A93_5D25, 8),
        (0x6A93_5D2B, 9),
        (0x6A93_5D28, 14),
        (0x6A93_5D2E, 0),
    ];
    let networks = [
        (0x6A93_5DC0, 0),
        (0x6A93_5DDA, 1),
        (0x6A93_5DC5, 2),
        (0x6A93_5DC2, 3),
        (0x6A93_5DEE, 4),
        (0x6A93_5DEB, 5),
        (0x6B73_0F78, 6),
        (0x6A93_5DE3, 7),
        (0x2B79_E72B, 8),
        (0x8BE0_98F4, 9),
        (0x4BE0_98F7, 10),
        (0x6BE0_98FA, 11),
        (0x4BE0_98FD, 12),
    ];
    let lots = [
        (0x6A93_5DC9, 0x0500_0001),
        (0x6A93_5DDD, 0x6A8A_358F),
        (0x6A93_5DDF, 0x6A38_55D4),
        (0x6A93_5DE6, 0x0500_0002),
        (0x6A93_5DF1, 0x69DE_426B),
        (0x6A93_5DF4, 0x0100_0005),
        (0x6A93_5DF7, 0x0100_0003),
        (0x6A93_5DFA, 0xA9DD_1E4C),
        (0x6A93_5DFD, 0x0100_0007),
        (0x6A93_5E0A, 0x2A27_35A0),
        (0x6A93_5E0D, 0xEA27_3867),
        (0x6A93_5E14, 0x89DD_094E),
        (0x6A93_5E11, 0x4A2A_F441),
        (0x6A93_5E17, 0x4A28_29D1),
        (0x2BFE_11C9, 0x4BAF_BA9F),
        (0x2BFE_1095, 0x0B96_AE64),
        (0x2BFE_0BEA, 0x6BB9_42D7),
    ];
    let vectors = zones
        .into_iter()
        .map(|(id, z)| (id, ToolSpec::Zone(z)))
        .chain(networks.into_iter().map(|(id, n)| (id, ToolSpec::Network(n))))
        .chain(lots.into_iter().map(|(id, lot)| (id, ToolSpec::Lot(lot))));
    for (command, tool) in vectors {
        for established in [false, true] {
            let mut v = view();
            let mut h = Host {
                established,
                ..Host::default()
            };
            v.owner.stack.push(77);
            v.owner.mouse_inside = true;
            assert!(!v.process_command(&mut h, command, [0, 0]));
            if established {
                assert_eq!(h.ops.first(), Some(&Op::Tool(tool.clone())), "{command:08X}");
                assert_eq!(v.input_control(), Some(90));
                assert!(h.ops.contains(&Op::Control(77, ControlCall::Shutdown)));
                assert!(h.ops.contains(&Op::Control(77, ControlCall::Release)));
                assert!(h.ops.contains(&Op::Control(90, ControlCall::Init)));
                assert!(h.ops.contains(&Op::Control(90, ControlCall::AddRef)));
                assert_eq!(h.ops.last(), Some(&Op::Control(90, ControlCall::Release)));
                assert_eq!(
                    h.ops
                        .contains(&Op::Control(90, ControlCall::Event(InputEvent::Move(50, 60, 0)))),
                    matches!(tool, ToolSpec::Lot(_))
                );
            } else {
                assert!(h.ops.is_empty(), "{command:08X}");
                assert_eq!(v.input_control(), Some(77));
            }
        }
    }
    for command in [0x6A93_5CD5, 0x6A93_5CD1, 0x2A94_96C9, 0x6A97_57C2, u32::MAX] {
        let mut v = view();
        let mut h = Host::default();
        assert!(!v.process_command(&mut h, command, [1, 2]));
        assert!(h.ops.is_empty());
    }
}

#[test]
fn ui_visibility_command_runs_one_minimize_operation() {
    let mut v = view();
    let mut h = Host::default();
    assert!(!v.process_command(&mut h, 0x6A93_5E3F, [0, 0]));
    assert_eq!(h.ui_visibility.iter().filter(|(name, _)| *name == "control").count(), 1);
    assert!(h.ui_visibility.contains(&("restore", true)));
    assert!(h.ui_visibility.contains(&("rotation", false)));
    assert!(v.owner.panels.minimized);
}

#[test]
fn vw_t36_fresh_background_falls_through_to_3d_and_latches_it() {
    let mut v = view();
    let mut h = Host {
        background_2d_absent: true,
        pictures: vec![BackgroundPicture {
            token: 40,
            width: 128,
            height: 128,
        }],
        ..Host::default()
    };
    for field in [0x150, 0xF4] {
        v.owner.lifecycle.objects.insert(field, field);
    }
    assert!(v.refresh_background(&mut h));
    assert_eq!(
        h.background_requests,
        [(false, "Background2D".into()), (true, "Background3D3".into())]
    );
    assert_eq!(v.owner.background_mode, 2);
    assert_eq!(h.helper_calls, [(40, 1), (40, 0xFFFF)]);
    h.background_requests.clear();
    h.pictures.clear();
    h.helper_calls.clear();
    assert!(!v.refresh_background(&mut h));
    assert_eq!(h.background_requests, [(true, "Background3D3".into())]);
    assert_eq!(v.owner.background_mode, 2);
    assert!(h.helper_calls.is_empty());
}

#[test]
fn city_required_fallbacks_initialize_and_optional_loader_does_not_abort() {
    for kind in [
        CityObject::Camera,
        CityObject::CameraControl,
        CityObject::Renderer,
        CityObject::AnimationManager,
        CityObject::ModelMaker,
    ] {
        let mut v = view();
        let mut h = Host {
            rejected_city: vec![CityCall::Create(kind), CityCall::Create(CityObject::ImageLoader)],
            ..Host::default()
        };
        v.pre_city_init(&mut h);
        let fallback = h
            .ops
            .iter()
            .position(|o| *o == Op::City(CityCall::Construct(kind)))
            .unwrap();
        assert_eq!(
            h.ops[fallback + 1],
            Op::Lifecycle(LifecycleCall::AddRef(kind.class_id()))
        );
        for initialized in [CityObject::AnimationManager, CityObject::ModelMaker] {
            assert!(h
                .ops
                .contains(&Op::City(CityCall::InitObject(initialized, initialized.class_id()))));
        }
        assert!(v.gui_state().fields.contains_key(&0x18C));
        assert!(!v.owner.lifecycle.objects.contains_key(&0x150));
        h.rejected_city.push(CityCall::Construct(kind));
        let mut v = view();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| v.pre_city_init(&mut h))).is_err());
    }
}

#[test]
fn gui_missing_main_fails_at_required_lookup_and_optional_auxiliaries_skip() {
    let mut v = view();
    let mut h = Host {
        rejected_gui: vec![GuiCall::Script {
            instance: 0xC973_B411,
            id: 0x0987_B48F,
            iid: 0x22BA_0121,
        }],
        ..Host::default()
    };
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| v.add_gui(&mut h))).is_err());
    assert!(h.ops.contains(&Op::Gui(GuiCall::Helper(HelperKind::NewsTicker))));
    assert!(!h.ops.iter().any(|o| matches!(o, Op::Gui(GuiCall::Child { id: 1, .. }))));

    let mut v = view();
    let mut h = Host {
        rejected_gui: vec![
            GuiCall::CreateBuffer,
            GuiCall::Image {
                type_id: 0x856D_DBAC,
                group: 0x46A0_06B0,
                instance: 0x1441_6190,
            },
            GuiCall::Image {
                type_id: 0x856D_DBAC,
                group: 0x46A0_06B0,
                instance: 0x1441_6160,
            },
            GuiCall::Create {
                class: 0xCA5D_3294,
                iid: 0xCA5D_3290,
            },
            GuiCall::RecursiveChild {
                id: 0xEA8C_AD14,
                iid: 0x5386_D516,
            },
        ],
        ..Host::default()
    };
    v.add_gui(&mut h);
    assert!(!v.gui_state().fields.contains_key(&0x3D0));
    assert!(!v.gui_state().fields.contains_key(&0x3E0));
    assert!(h
        .ops
        .contains(&Op::Gui(GuiCall::AddChild(v.gui_state().fields[&0x3D4]))));
    assert!(h
        .ops
        .contains(&Op::Gui(GuiCall::AddChild(v.gui_state().fields[&0x3D8]))));
    assert!(!h
        .ops
        .iter()
        .any(|o| matches!(o, Op::Gui(GuiCall::SurfaceSize(..) | GuiCall::CalloutImage(..)))));
    v.shutdown(&mut h);
    assert!(v.gui_state().fields.is_empty());
    assert_eq!(h.ops.last(), Some(&Op::Gui(GuiCall::BaseShutdown)));
}

#[test]
fn edge_and_capture_branches_preserve_fullscreen_and_clear_windowed_scroll() {
    // Recalc consumes the edge dwords with the preference enabled. Observe the
    // resulting scrolling/angle, plus the absence of a recompute on keep paths.
    for (fullscreen, self_scroll, capturing, preference, tutorial, expected_scroll) in [
        (true, false, false, true, true, true),
        (false, false, true, true, true, true),
        (false, false, false, true, true, false),
        (false, true, true, true, true, false),
        (true, false, false, false, true, false),
        (true, false, false, true, false, false),
    ] {
        let mut v = view();
        let mut h = Host::default();
        v.owner.stack.push(9);
        if self_scroll {
            h.self_scrolling.push(9);
        }
        if capturing {
            h.capturing_controls.push(9);
        }
        let c = MouseContext {
            width: 100,
            height: 60,
            edge_preference: preference,
            tutorial_edge_allowed: tutorial,
            fullscreen,
        };
        v.on_mouse(&mut h, InputEvent::Move(1, 59, 0), c);
        assert_eq!(v.owner.motion.scrolling, expected_scroll);
        assert_eq!(v.owner.motion.edge, [0, 0]);
        if expected_scroll {
            assert!((v.owner.motion.angle - 3.0 * std::f32::consts::FRAC_PI_4).abs() < 1e-6);
        }
    }
    let mut v = view();
    let mut h = Host {
        self_scrolling: vec![9],
        ..Host::default()
    };
    v.owner.stack.push(9);
    v.owner.motion.edge = [-1, 1];
    let mut c = MouseContext {
        width: 100,
        height: 60,
        edge_preference: true,
        tutorial_edge_allowed: true,
        fullscreen: true,
    };
    v.on_mouse(&mut h, InputEvent::Move(50, 30, 0), c);
    assert_eq!(v.owner.motion.edge, [-1, 1]); // fullscreen self-scroll keeps state
    c.fullscreen = false;
    v.on_mouse(&mut h, InputEvent::Move(50, 30, 0), c);
    assert_eq!(v.owner.motion.edge, [0, 0]);
    assert!(v.owner.motion.stopping || !v.owner.motion.scrolling);

    // Left release recomputes only for a captured, non-self-scrolling control.
    h.capture = true;
    h.self_scrolling.clear();
    h.capturing_controls.push(9);
    v.owner.motion.edge = [1, 0];
    v.on_mouse(&mut h, InputEvent::UpL(99, 30, 4), c);
    assert!(v.owner.motion.scrolling);
    assert!(h.capture); // left release never drops capture
    assert_eq!(v.owner.modifiers, 4);
    v.on_mouse(&mut h, InputEvent::Exit, c);
    assert!(h.ops.contains(&Op::Position(-10000, -10000)));
    assert_eq!(v.owner.motion.edge, [0, 0]);
}

#[test]
fn mayor_bookkeeping_retains_none_shown_and_ticker_tracks_panel_visibility() {
    let ids = [0x49ED_F9B7, 0x41, 0x9988_7755, 0x1520_0002, 0x1520_0003, 0xABC5_4125];
    for selected in ids {
        let mut v = view();
        let mut h = Host {
            mayor_visible: Some(vec![selected]),
            ..Host::default()
        };
        v.show_mayor_subpanel(&mut h, selected);
        assert_eq!(h.mayor_calls, ids.map(|id| (id, id == selected)));
        v.show_panel(&mut h, 1);
        assert!(v.owner.panels.mayor_shown);
        assert_eq!(h.ui_visibility.last(), Some(&("ticker", true)));
        v.hide_panel(&mut h, 1);
        assert!(!v.owner.panels.mayor_none_shown);
        assert_eq!(h.ui_visibility.last(), Some(&("ticker", false)));
        assert!(ids.into_iter().all(|id| h.mayor_calls.contains(&(id, false))));
        h.mayor_visible = Some(Vec::new());
        v.hide_panel(&mut h, 1);
        assert!(v.owner.panels.mayor_none_shown);
        h.mayor_calls.clear();
        v.show_mayor_subpanel(&mut h, selected);
        assert_eq!(v.owner.panels.mayor_subpanel, selected);
        assert!(h.mayor_calls.is_empty());
    }
    let mut v = view();
    let mut h = Host::default();
    v.show_mayor_subpanel(&mut h, 0);
    assert!(v.owner.panels.mayor_none_shown);
    assert!(h.mayor_calls.is_empty());
    v.show_panel(&mut h, 2);
    assert!(h.ui_visibility.contains(&("advisor", false)));
    assert!(v.owner.panels.mysim_shown);
    v.show_panel(&mut h, 4);
    assert!(!v.owner.panels.mysim_shown);
    assert!(v.owner.panels.panel4_shown);
    v.hide_panel(&mut h, 4);
    assert_eq!(h.ui_visibility.last(), Some(&("ticker", false)));
}

#[test]
fn inert_buttons_still_notify_tutorial_and_global_terrain_stays_in_owner() {
    for button in [0xABB2_7B91, 0x09DE_87AE, 0xE9DE_87A7] {
        let mut v = view();
        let mut h = Host::default();
        assert!(v.filter_message(
            &mut h,
            WindowMessage {
                kind: 3,
                data: [0x2872_59F6, button, 0],
                ..WindowMessage::default()
            }
        ));
        assert_eq!(h.popup_calls, [PopupCall::TutorialButton(button)]);
        assert!(h.ops.is_empty());
    }
    for (item, effect) in [
        (3, PopupCall::TerrainLevel(true)),
        (4, PopupCall::TerrainLevel(false)),
        (5, PopupCall::TerrainPost(0x470B_D214, 0)),
        (6, PopupCall::TerrainPost(0x470B_D214, 1)),
    ] {
        let mut v = view();
        let mut h = Host::default();
        v.invoke_catalog_item(&mut h, 0x6A5A_C478, item, false);
        assert_eq!(h.popup_calls, [PopupCall::TerrainSound, effect]);
        assert!(h.ops.is_empty());
    }
    let mut v = view();
    let mut h = Host::default();
    v.run_global_terrain_tool(&mut h, 7);
    assert_eq!(h.ops.first(), Some(&Op::Tool(ToolSpec::LevelTerrain)));
    assert_eq!(v.input_control(), Some(90));
    assert_eq!(h.ops.last(), Some(&Op::Control(90, ControlCall::Release)));
}

#[test]
fn catalog_groups_resource_ids_and_reward_override_reach_foreign_catalog() {
    // Each family has independently specified catalog/background identities.
    for (button, catalog, background, groups) in [
        (0x2992_0899, 0x2A38_58E4, 0x1421_5ED1, vec![0x8A3F_1505]),
        (0xA998_AF42, 0x2A38_58E4, 0x1421_5ED1, vec![0x8A3F_14DC]),
        (0xC998_AF00, 0x2A38_58E4, 0x1421_5ED1, vec![0x8A3F_1502]),
        (0x35, 0xC8DB_CCBA, 0x1421_5ED3, vec![0x1400, 0x1402, 0x6A65_0FFE]),
        (0x39, 0xC8DB_CCBA, 0x1421_5ED3, vec![0x1401]),
        (0x40, 0xC8DB_CCBA, 0x1421_5ED3, vec![0x1402, 0x1401]),
        (3, 0xC8DB_CCBA, 0x1421_5ED4, vec![0x1006]),
        (0x34, 0xC8DB_CCBA, 0x1421_5ED4, vec![0x150B]),
        (0x37, 0xC8DB_CCBA, 0x1421_5ED4, vec![0x1500, 0x1501]),
        (0x38, 0xC8DB_CCBA, 0x1421_5ED4, vec![0x1502]),
        (0x42, 0xC8DB_CCBA, 0x1421_5ED4, vec![0x1503, 0x1504, 0x1505, 0x1506]),
        (0x0993_0709, 0xC8DB_CCBA, 0x1421_5ED4, vec![0x150A]),
        (0x89DD_5405, 0xC8DB_CCBA, 0x1421_5ED4, vec![0x1507]),
        (0x4A22_EA06, 0x6A01_FC2A, 0x1421_5ED0, vec![0x8A26_02BB]),
        (0xC992_31DA, 0x6A01_FC2A, 0x1421_5ED0, vec![0x6A33_D682]),
        (0x1522_31F3, 0x6A01_FC2A, 0x1421_5ED0, vec![0xEA33_D694]),
        (0x6A3D_638C, 0x6A01_FC2A, 0x1421_5ED0, vec![0x8A3F_131B]),
        (0x2992_3215, 0x6A01_FC2A, 0x1421_5ED0, vec![0x8A31_43B7]),
    ] {
        let mut v = view();
        let mut h = Host::default();
        v.process_cheat(&mut h, 0x4A82_253E, &["You don't deserve it"]);
        assert!(v.handle_button_activated(&mut h, button));
        assert_eq!(h.catalog_queries.len(), 1);
        let q = &h.catalog_queries[0];
        assert_eq!(q.catalog, catalog, "{button:08X}");
        assert_eq!(q.groups, groups);
        assert!(q.all_rewards);
        assert!(h
            .popup_calls
            .iter()
            .any(|call| matches!(call, PopupCall::Catalog { layout, .. }
            if layout.background == background)));
        assert_eq!(h.popup_calls.last(), Some(&PopupCall::TutorialButton(button)));
    }
}

#[test]
fn transport_script_vehicle_and_disaster_menus_have_literal_identity() {
    for (button, groups, networks) in [
        (0x29, vec![0x1300], vec![0x4001]),
        (0x31, vec![], vec![0x4005]),
        (0x2992_37BF, vec![0x130A], vec![0x4002]),
        (0x6999_BF56, vec![], vec![0x4000]),
        (0x2B73_0C5B, vec![], vec![0x4000]),
        (0xA992_34A6, vec![0x1519], vec![]),
        (0xE992_34B3, vec![0x1508], vec![]),
        (0x2B79_E398, vec![], vec![]),
        (0xEA6E_379D, vec![], vec![]),
    ] {
        let mut v = view();
        let mut h = Host::default();
        v.handle_button_activated(&mut h, button);
        let q = &h.catalog_queries[0];
        assert_eq!(q.catalog, 0x07BD_DF1C);
        assert_eq!(q.kind, 0x4A2B_2467);
        assert_eq!(q.groups, groups);
        assert_eq!(q.networks, networks);
        assert!(h
            .popup_calls
            .iter()
            .any(|c| matches!(c, PopupCall::Catalog { layout, .. }
            if layout.background == 0x1421_5ED2)));
    }
    for (button, instance, kind, secondary) in [
        (0x6991_EE42, 0x8993_02FC, 0x0992_FD17, true),
        (0x0991_EE39, 0xC993_0681, 0x6993_06ED, true),
        (0xA994_824D, 0xE992_37FF, 0xC992_37A0, true),
        (0xA9ED_5617, 0xAA35_6502, 0xCA35_CBED, true),
        (0xE991_EE2F, 0x4992_F764, 0xE992_F711, true),
        (0x0991_EE13, 0xE994_9936, 0x6992_3479, true),
        (0xABB2_7A7A, 0x6BB2_7447, 0x8BB2_7C12, true),
        (0x49E9_5D2B, 0xE992_3283, 0x4992_3239, true),
        (0x8991_EE08, 0x0992_3283, 0x4992_3239, true),
        (1, 0x09DE_3002, 0x09DE_8798, false),
        (0xAB95_37B7, 0xCB95_403E, 0xAB95_4023, false),
    ] {
        let mut v = view();
        let mut h = Host::default();
        v.owner.stack.push(10);
        h.capture = true;
        v.handle_button_activated(&mut h, button);
        assert!(h.ops.contains(&Op::Control(10, ControlCall::Shutdown)));
        assert!(h.ops.contains(&Op::Capture(false)));
        assert!(h.popup_calls.contains(&PopupCall::Script {
            parent: v.as_igzwin(),
            group: 0x96A0_06B0,
            instance,
            data: button,
        }));
        assert_eq!(
            if secondary {
                v.secondary_kind()
            } else {
                v.tertiary_kind()
            },
            kind
        );
        h.ops.clear();
        v.owner.stack.push(11);
        v.handle_button_activated(&mut h, button);
        assert!(h.ops.contains(&Op::Control(11, ControlCall::Shutdown))); // before same-kind toggle
        assert_eq!(
            if secondary {
                v.secondary_kind()
            } else {
                v.tertiary_kind()
            },
            0
        );
    }
    for (button, catalog, kind, secondary) in [
        (0xEBFA_C717, 0x00B4_3D44, 0x0BB2_E43E, false),
        (0xEBFA_C718, 0x00B4_3D4C, 0x0BB2_E43E, false),
        (0xEBFA_C719, 0x00B4_3D50, 0x0BB2_E43E, false),
        (0x69B9_324A, 0x29B6_C670, 0x4A2B_2464, true),
    ] {
        let mut v = view();
        let mut h = Host::default();
        v.handle_button_activated(&mut h, button);
        assert_eq!(h.catalog_queries[0].catalog, catalog);
        assert_eq!(h.catalog_queries[0].kind, kind);
        assert_eq!(
            if secondary {
                v.secondary_kind()
            } else {
                v.tertiary_kind()
            },
            button
        );
    }
}

#[test]
fn shutdown_orders_services_gui_progress_and_registry_and_consumes_owned_fields() {
    let mut v = view();
    let mut h = Host::default();
    v.pre_city_init(&mut h);
    v.owner.lifecycle.progress = Some(777);
    v.owner.lifecycle.objects.insert(0x410, 778);
    let gui_refs: Vec<_> = v.gui_state().fields.values().copied().collect();
    let service_refs: Vec<_> = v
        .owner
        .lifecycle
        .objects
        .iter()
        .filter_map(|(&field, &id)| (field != 0x410).then_some(id))
        .collect();
    h.ops.clear();
    v.shutdown(&mut h);
    let remove_gui = h
        .ops
        .iter()
        .position(|o| matches!(o, Op::Gui(GuiCall::Destroy(_))))
        .unwrap();
    for token in service_refs {
        let release = h
            .ops
            .iter()
            .position(|o| *o == Op::Lifecycle(LifecycleCall::Release(token)))
            .unwrap();
        assert!(release < remove_gui);
    }
    for token in gui_refs {
        assert_eq!(
            h.ops.iter().filter(|o| **o == Op::Gui(GuiCall::Release(token))).count(),
            1
        );
    }
    let progress = h
        .ops
        .iter()
        .position(|o| *o == Op::Lifecycle(LifecycleCall::ProgressShutdown(777)))
        .unwrap();
    let unregister = h
        .ops
        .iter()
        .position(|o| *o == Op::Lifecycle(LifecycleCall::UnregisterCommands))
        .unwrap();
    assert!(remove_gui < progress && progress < unregister);
    assert_eq!(h.ops.last(), Some(&Op::Gui(GuiCall::BaseShutdown)));
    assert!(v.gui_state().fields.is_empty());
    assert!(v.gui_state().windows.is_empty());
    assert!(v.gui_state().panels.iter().all(Vec::is_empty));
    assert_eq!(v.owner.lifecycle.objects.len(), 1);
    assert!(!h.ops.contains(&Op::Lifecycle(LifecycleCall::Release(778))));
    v.destroy(&mut h);
    assert_eq!(h.ops.last(), Some(&Op::Lifecycle(LifecycleCall::Release(778))));
}
