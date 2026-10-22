// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
mod support;
use crate::{
    terrain::{Terrain, TerrainTuning, VertRect},
    view_utilities::{PixelRect, ViewUtilities},
};
use support::{Codec, Event as E, Host};

fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.00001, "{a} != {b}");
}
fn motion(host: &Host) -> ScrollMotion {
    match host.events.last().unwrap() {
        E::Motion(m) => *m,
        _ => panic!("no scrolling output"),
    }
}

#[test]
fn scroll_thresholds_raw_magnitude_inclusive_cursor_and_release_order() {
    let mut host = Host::default();
    let mut tool = CityScroll::default();
    tool.init(&mut host, ScrollTunables::default()).unwrap();
    tool.set_mouse_scroll(&mut host, Pixel::new(100, 100)).unwrap();
    assert_eq!(
        &host.events[3..],
        &[
            E::LoadPicture,
            E::Transparent(7, (255, 0, 255)),
            E::PictureWindow(0x48E9_45B4, 7, PixelRect::new(90, 95, 110, 105)),
            E::Flag(8, 2, false),
            E::Flag(8, 4, true),
            E::ShowWindow(8),
            E::Child(8),
            E::Front(8),
        ]
    );
    for (at, stored, cursor, on, speed, angle) in [
        (Pixel::new(124, 100), (0, 0), 0xC2A6_76AC, false, 12.0 / 68.0, 0.0),
        (
            Pixel::new(126, 124),
            (13, 0),
            0x22A6_76A0,
            true,
            313.0_f32.sqrt() / 68.0,
            (12.0_f32 / 13.0).atan(),
        ),
        (Pixel::new(300, 100), (80, 0), 0xC2A6_76A4, true, 1.0, 0.0),
        (Pixel::new(99, 99), (0, 0), 0xC2A6_76AC, false, 0.0, 0.0),
        (
            Pixel::new(74, 74),
            (-13, -13),
            0x02A6_7691,
            true,
            338.0_f32.sqrt() / 68.0,
            5.0 * std::f32::consts::PI / 4.0,
        ),
        (
            Pixel::new(100, 74),
            (0, -13),
            0x62A6_7606,
            true,
            13.0 / 68.0,
            3.0 * std::f32::consts::FRAC_PI_2,
        ),
    ] {
        host.clear();
        assert!(!tool.mouse_move(&mut host, at, true));
        assert_eq!(tool.stored_speed(), stored);
        assert_eq!(tool.cursor(), cursor);
        let m = motion(&host);
        assert_eq!(m.on, on);
        near(m.speed, speed);
        near(m.angle, angle);
    }
    assert!(tool.has_scrolled());
    assert!(tool.is_self_scrolling_view());
    host.clear();
    assert!(tool.mouse_move(&mut host, Pixel::new(500, 500), false));
    assert_eq!(host.events, vec![E::Stop, E::DeleteChild(0x48E9_45B4), E::Top(false)]);
    host.clear();
    tool.shutdown(&mut host);
    assert_eq!(
        host.events,
        vec![E::Stop, E::DeleteChild(0x48E9_45B4), E::Release, E::ReleasePicture(7)]
    );
    host.clear();
    tool.shutdown(&mut host);
    assert!(host.events.is_empty());
}

#[test]
fn scroll_tunables_capture_resource_failure_and_unknown_cursor() {
    let mut host = Host {
        capturing: true,
        child_add: false,
        ..Default::default()
    };
    let mut tool = CityScroll::default();
    assert_eq!(
        tool.init(
            &mut host,
            ScrollTunables {
                inner: Some(10),
                outer: Some(10)
            }
        ),
        Err(Unsupported::PrimitiveData)
    );
    assert!(host.events.is_empty());
    tool.init(
        &mut host,
        ScrollTunables {
            inner: Some(2),
            outer: Some(10),
        },
    )
    .unwrap();
    assert!(!host.events.contains(&E::Capture));
    tool.set_mouse_scroll(&mut host, Pixel::new(0, 0)).unwrap();
    assert_eq!(host.events.last(), Some(&E::Destroy(8)));
    host.cursor_found = false;
    host.clear();
    tool.mouse_move(&mut host, Pixel::new(12, 0), true);
    near(motion(&host).speed, 0.75);
    assert_eq!(tool.cursor(), 0xC2A6_76AC);
    assert!(tool.has_scrolled());
    host.clear();
    tool.shutdown(&mut host);
    assert!(!host.events.contains(&E::Release));
    let mut host = Host {
        picture_found: false,
        ..Default::default()
    };
    let mut tool = CityScroll::default();
    assert_eq!(
        tool.set_mouse_scroll(&mut host, Pixel::new(0, 0)),
        Err(Unsupported::ResourceUnavailable)
    );
}

#[test]
fn leveling_uses_start_vertex_extends_clamps_and_restores() {
    let mut terrain = Terrain::create(8, 8, TerrainTuning::default()).unwrap();
    terrain.set_altitude_at_vertex(39, 117.0);
    terrain.set_altitude_at_vertex(37, 9.0);
    let mut host = Host::default();
    let mut tool = LevelTerrain::new(&terrain).unwrap();
    let pixel = Pixel::new(1, 2);
    assert!(!tool.mouse_move(&mut host, pixel));
    assert!(!tool.mouse_up(&mut host, &mut terrain, pixel).unwrap());
    host.hit(Vec3::new(48.0, 0.0, 64.0));
    assert!(tool.mouse_down(&mut host, pixel));
    assert_eq!(host.events, vec![E::Pick(pixel), E::Capture, E::Auto(false)]);
    host.clear();
    host.hit(Vec3::new(16.0, 0.0, 96.0));
    assert!(tool.mouse_move(&mut host, pixel));
    assert_eq!(
        host.events,
        vec![E::Pick(pixel), E::Mark(VertRect::new(1, 4, 3, 6), 2, true)]
    );
    host.clear();
    assert!(tool.mouse_up(&mut host, &mut terrain, pixel).unwrap()); // miss uses last end
    assert_eq!(
        host.events,
        vec![
            E::Pick(pixel),
            E::Mark(VertRect::new(1, 4, 3, 6), 2, true),
            E::Clear,
            E::Auto(true),
            E::Release
        ]
    );
    for z in 4..=7 {
        for x in 1..=4 {
            assert_eq!(terrain.altitude_at_vertex(x, z), 117.0);
        }
    }
    assert_ne!(terrain.altitude_at_vertex(5, 7), 117.0);
    assert!(!tool.dragging());
    assert!(!tool.key_down(0x41, 0));
    let mut terrain = Terrain::create(4, 6, TerrainTuning::default()).unwrap();
    terrain.set_altitude(3, 4, 88.0);
    let mut tool = LevelTerrain::new(&terrain).unwrap();
    host.hit(Vec3::new(48.0, 0.0, 64.0));
    tool.mouse_down(&mut host, pixel);
    host.hit(Vec3::new(64.0, 0.0, 96.0));
    tool.mouse_up(&mut host, &mut terrain, pixel).unwrap();
    assert_eq!(terrain.altitude_at_vertex(4, 6), 88.0);
}

#[test]
fn leveling_miss_no_view_and_invalid_range_are_distinct() {
    let mut terrain = Terrain::create(4, 4, TerrainTuning::default()).unwrap();
    let mut tool = LevelTerrain::new(&terrain).unwrap();
    let mut host = Host::default();
    assert!(tool.mouse_down(&mut host, Pixel::new(0, 0)));
    assert!(!tool.dragging());
    assert_eq!(host.events, vec![E::Pick(Pixel::new(0, 0))]);
    host.city_view = false;
    host.clear();
    assert!(!tool.mouse_down(&mut host, Pixel::new(0, 0)));
    assert!(host.events.is_empty());
    host.city_view = true;
    host.hit(Vec3::new(-32.0, 0.0, 0.0));
    tool.mouse_down(&mut host, Pixel::new(0, 0));
    host.clear();
    assert_eq!(
        tool.mouse_up(&mut host, &mut terrain, Pixel::new(0, 0)),
        Err(Unsupported::PrimitiveData)
    );
    assert!(host.events.ends_with(&[E::Clear, E::Auto(true), E::Release]));
    assert!(!tool.dragging());
}

#[test]
fn label_placement_priorities_exact_off_network_and_network_masks() {
    let p = Vec3::new(19.5, 7.0, 33.5);
    let mut host = Host {
        water: true,
        tile: Some(NetworkTile {
            flags: 0x40,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(
        label_placement(&host, p, 3).unwrap(),
        LabelPlacement {
            position: p,
            orientation: 3,
            color: (0.5, 0.5, 1.0),
            height: 2.0
        }
    );
    host.water = false;
    host.tile = None;
    assert_eq!(
        label_placement(&host, p, 2).unwrap(),
        LabelPlacement {
            position: p,
            orientation: 2,
            color: (0.5, 1.0, 0.5),
            height: 2.0
        }
    );
    for (flags, edges, height, orientation, position) in [
        (8, [0, 2, 0, 2], 0.64, 0, Vec3::new(24.0, 7.0, 40.0)),
        (1, [1, 3, 0, 0], 0.8, 2, Vec3::new(20.0, 7.0, 36.0)),
        (0x400, [0, 0, 3, 1], 0.8, 2, Vec3::new(28.0, 7.0, 44.0)),
    ] {
        let tile = NetworkTile {
            flags,
            road: edges,
            one_way: edges,
            street: edges,
            ..Default::default()
        };
        host.tile = Some(tile);
        host.road_tile = Some(tile);
        let out = label_placement(&host, p, 3).unwrap();
        assert_eq!(
            out,
            LabelPlacement {
                position,
                orientation,
                color: (1.0, 0.902, 0.0),
                height
            }
        );
    }
    host.tile = Some(NetworkTile {
        flags: 3,
        ..Default::default()
    });
    host.road_tile = Some(NetworkTile {
        flags: 3,
        rail: [0, 2, 0, 2],
        road: [2, 0, 2, 0],
        ..Default::default()
    });
    assert_eq!(label_placement(&host, p, 1).unwrap().orientation, 0);
    host.tile = Some(NetworkTile {
        flags: 0x40,
        ..Default::default()
    });
    host.avenue = None;
    assert_eq!(
        label_placement(&host, p, 3).unwrap(),
        LabelPlacement {
            position: p,
            orientation: 1,
            color: (1.0, 0.902, 0.0),
            height: 2.0
        }
    );
    host.avenue = Some([3, 0, 0, 0]);
    assert_eq!(
        label_placement(&host, p, 1).unwrap().position,
        Vec3::new(16.0, 7.0, 48.0)
    );
    host.tile = Some(NetworkTile {
        flags: 1,
        ..Default::default()
    });
    host.road_tile = None;
    assert_eq!(label_placement(&host, p, 1), Err(Unsupported::PrimitiveData));
}

#[test]
fn label_selection_drag_ignores_offset_and_sound_order() {
    let mut host = Host::default();
    let a = host.insert(LABEL_TYPE, b"a", Vec3::new(9.0, 0.0, 9.0));
    let b = host.insert(LABEL_TYPE, b"b", Vec3::new(40.0, 0.0, 50.0));
    let mut tool = LabelTool::default();
    tool.select(&mut host, Some(a));
    host.clear();
    tool.select(&mut host, Some(b));
    assert_eq!(host.events, vec![E::Alpha(b, 0.7), E::Alpha(a, 0.5)]);
    host.hit(Vec3::new(20.0, 0.0, 30.0));
    host.occupant_picks.push_back(Some(b));
    tool.mouse_down(&mut host, Pixel::new(3, 4)).unwrap();
    assert_eq!(tool.press_offset(), Vec3::new(20.0, 0.0, 20.0));
    assert_eq!(tool.state(), MouseState::Pressed);
    host.clear();
    host.hit(Vec3::new(21.0, 2.0, 31.0));
    tool.mouse_move(&mut host, Pixel::new(4, 5)).unwrap();
    assert_eq!(host.events[0], E::Dynamic(b, true));
    assert_eq!(host.position(b), Vec3::new(21.0, 2.0, 31.0));
    host.clear();
    tool.mouse_up(&mut host).unwrap();
    assert_eq!(
        host.events,
        vec![E::Dynamic(b, false), E::Sound(0xEA45_1903, 0x2C16_DA29)]
    );
    assert_eq!(tool.state(), MouseState::Idle);
    host.clear();
    tool.shutdown(&mut host);
    assert_eq!(host.events, vec![E::Alpha(b, 0.5)]);
}

#[test]
fn label_dialog_results_cancel_cleanup_keys_and_rotation() {
    let mut host = Host::default();
    let a = host.insert(LABEL_TYPE, b"old", Vec3::ZERO);
    let mut tool = LabelTool::default();
    assert!(tool.edit_label(&mut host, None).unwrap());
    assert!(host.events.is_empty());
    host.modal_result = LABEL_CANCEL;
    assert!(!tool.edit_label(&mut host, Some(a)).unwrap());
    assert_eq!(host.text(a), Text::from_single_bytes(b"old"));
    assert_eq!(
        host.events,
        vec![
            E::Dialog((0x96A0_06B0, 0x6B70_4690), 0x8A8D_FCF5),
            E::Field(Text::from_single_bytes(b"old")),
            E::SelectAll,
            E::Modal,
            E::Focus,
            E::ShutdownDialog
        ]
    );
    host.modal_result = 123;
    assert!(tool.edit_label(&mut host, Some(a)).unwrap());
    assert_eq!(host.text(a), Text::from_single_bytes(b"old"));
    host.modal_result = LABEL_OK;
    host.modal_text = Text::default();
    assert!(!tool.edit_label(&mut host, Some(a)).unwrap());
    assert!(host.text(a).is_empty());
    host.hit(Vec3::ZERO);
    host.occupant_picks.push_back(Some(a));
    tool.mouse_down(&mut host, Pixel::new(0, 0)).unwrap();
    host.clear();
    tool.mouse_up(&mut host).unwrap();
    assert_eq!(tool.selected(), None);
    assert!(host.events.ends_with(&[E::Alpha(a, 0.5), E::Remove(a)]));
    host.modal_text = Text::from_single_bytes(b"new");
    host.rotation = 1;
    host.hit(Vec3::new(2.0, 3.0, 4.0));
    tool.mouse_down(&mut host, Pixel::new(0, 0)).unwrap();
    let p = tool.selected().unwrap();
    assert_eq!(tool.orientation(), 0);
    host.clear();
    assert!(!tool.key_down(&mut host, Key::Home).unwrap());
    assert_eq!(tool.orientation(), 3);
    assert!(host.events.contains(&E::Orientation(p, 3)));
    assert!(!tool.key_down(&mut host, Key::End).unwrap());
    assert_eq!(tool.orientation(), 0);
    assert!(tool.key_down(&mut host, Key::Delete).unwrap());
    assert_eq!(tool.selected(), None);
}

#[test]
fn label_message_filter_checks_type_id_and_widget() {
    for id in [LABEL_OK, LABEL_CANCEL, LABEL_CLOSE] {
        assert_eq!(
            label_message_filter(DialogMessage {
                kind: 3,
                message: 0x2872_59F6,
                widget: id,
                key: 0
            }),
            FilterAction::EndModal { kind: 15, result: id }
        );
    }
    assert_eq!(
        label_message_filter(DialogMessage {
            kind: 0x1A,
            key: 13,
            ..Default::default()
        }),
        FilterAction::EndModal {
            kind: 15,
            result: LABEL_OK
        }
    );
    assert_eq!(
        label_message_filter(DialogMessage {
            kind: 3,
            message: 0,
            widget: LABEL_OK,
            key: 0
        }),
        FilterAction::None
    );
    assert_eq!(
        label_message_filter(DialogMessage {
            kind: 3,
            message: 0x2872_59F6,
            widget: 0,
            key: 0
        }),
        FilterAction::None
    );
}

fn utilities() -> ViewUtilities {
    use crate::view_utilities::{CameraState, ViewVolume};
    let mut view = ViewUtilities::new();
    view.set_renderer_present(true);
    assert!(view.update_camera_position(Some(&CameraState {
        position: Vec3::ZERO,
        look_at: Vec3::new(0.0, -1.0, -1.0),
        zoom: 4,
        rotation: 0,
        viewport_width: 200,
        viewport_height: 100,
        volume: ViewVolume {
            left: -10.0,
            right: 10.0,
            bottom: -5.0,
            top: 5.0,
            near: 0.0,
            far: 100.0
        },
        ..Default::default()
    })));
    view
}

fn new_sign(host: &mut Host, tool: &mut SignTool) -> PropId {
    host.hit(Vec3::new(10.0, 99.0, 20.0));
    assert!(tool.mouse_down(host, &utilities(), Pixel::new(0, 0)).unwrap());
    tool.selected().unwrap()
}

#[test]
fn sign_new_placement_and_empty_enter_remove_without_stale_access() {
    let mut host = Host::default();
    let mut tool = SignTool::new(0);
    assert!(!tool.on_character(&mut host, &Codec, b'a').unwrap());
    let p = new_sign(&mut host, &mut tool);
    assert_eq!(tool.state(), MouseState::Typing);
    assert_eq!(host.position(p), Vec3::new(10.0, 5.0, 20.0));
    assert!(host.events.ends_with(&[
        E::Text(p, Text::default()),
        E::Edit(p, true),
        E::Sound(0xEA45_1903, 0x2C16_DA29)
    ]));
    host.clear();
    assert!(tool.on_character(&mut host, &Codec, 13).unwrap());
    assert_eq!(host.events, vec![E::Edit(p, false), E::Dynamic(p, true), E::Remove(p)]);
    assert_eq!(tool.state(), MouseState::Idle);
    assert_eq!(tool.selected(), None);
}

#[test]
fn sign_glyph_limit_newline_budget_and_multibyte_backspace() {
    let mut host = Host::default();
    let mut tool = SignTool::new(8);
    let p = new_sign(&mut host, &mut tool);
    tool.append_text(&mut host, &Text::from_single_bytes(&[b'a'; 32]));
    tool.on_character(&mut host, &Codec, b'b').unwrap();
    assert_eq!(tool.edit_text().glyph_count(), 33);
    tool.on_character(&mut host, &Codec, b'c').unwrap();
    assert_eq!(host.text(p).glyph_count(), 33);
    tool.on_character(&mut host, &Codec, 8).unwrap();
    tool.on_character(&mut host, &Codec, 8).unwrap();
    assert_eq!(tool.edit_text().glyph_count(), 31);
    tool.append_text(&mut host, &Text::from_single_bytes(b"\nAB"));
    assert_eq!(tool.edit_text().glyph_count(), 32);
    assert_eq!(tool.edit_text().0.last(), Some(&vec![b'A']));
    tool.on_character(&mut host, &Codec, 0x81).unwrap();
    assert_eq!(tool.pending_bytes(), &[0x81]);
    assert_eq!(tool.edit_text().glyph_count(), 32);
    tool.on_character(&mut host, &Codec, 0x40).unwrap();
    assert!(tool.pending_bytes().is_empty());
    assert_eq!(tool.edit_text().glyph_count(), 33);
    assert_eq!(tool.edit_text().0.last(), Some(&vec![0x81, 0x40]));
    tool.on_character(&mut host, &Codec, 8).unwrap();
    assert_eq!(tool.edit_text().glyph_count(), 32);
    assert_eq!(
        tool.on_character(&mut host, &Codec, 0x82),
        Err(Unsupported::CharacterEncoding)
    );
    for cp in [0, 7, 9, 99] {
        assert!(!SignTool::new(cp).is_multibyte());
    }
    for cp in [8, 100, 1252] {
        assert!(SignTool::new(cp).is_multibyte());
    }
}

#[test]
fn sign_backup_escape_ctrl_v_and_message_projection() {
    let view = utilities();
    let mut host = Host::default();
    let p = host.insert(SIGN_TYPE, b"saved", Vec3::ZERO);
    // Zero x/z bounds yield an independent projection: x=100,
    // y=50-60*cos(pi/4); the second corner's y is 6, not centre y=3.
    host.prop_mut(p).bounds = (Vec3::ZERO, Vec3::new(0.0, 6.0, 0.0));
    let mut tool = SignTool::new(0);
    host.renderer_picks.push_back(Some(p));
    tool.mouse_down(&mut host, &view, Pixel::new(100, 50)).unwrap();
    tool.mouse_up(&mut host);
    assert_eq!(tool.state(), MouseState::Typing);
    tool.on_character(&mut host, &Codec, b'!').unwrap();
    assert!(!tool.key_down(&mut host, Key::Escape, 0, ServiceText::Unbound).unwrap());
    assert_eq!(tool.state(), MouseState::Typing);
    assert_eq!(host.text(p), Text::from_single_bytes(b"saved"));
    for m in [0, 1, 3, 4, 6, 7] {
        assert!(tool
            .key_down(&mut host, Key::V, m, ServiceText::Text(Text::from_single_bytes(b"x")))
            .unwrap());
        assert_eq!(tool.edit_text(), &Text::from_single_bytes(b"saved"));
    }
    assert_eq!(
        tool.key_down(&mut host, Key::V, 2, ServiceText::Unbound),
        Err(Unsupported::ServiceProtocol)
    );
    tool.key_down(&mut host, Key::V, 2, ServiceText::NoText).unwrap();
    tool.key_down(
        &mut host,
        Key::V,
        10,
        ServiceText::Text(Text::from_single_bytes(b"paste")),
    )
    .unwrap();
    assert_eq!(host.text(p), Text::from_single_bytes(b"savedpaste"));
    let reply = tool.position_message(&host, &view, 0x0BE1_0A20).unwrap();
    assert!(reply.answered);
    assert!(!reply.consumed);
    near(reply.screen.x, 100.0);
    near(reply.screen.y, 50.0 - 60.0 * std::f32::consts::FRAC_1_SQRT_2);
    assert_eq!(tool.position_message(&host, &view, 0x0BE1_0A21), None);
    host.clear();
    tool.shutdown(&mut host);
    assert_eq!(
        host.events,
        vec![E::Edit(p, false), E::Dynamic(p, true), E::Dynamic(p, false)]
    );
    assert_eq!(tool.selected(), None);
}

#[test]
fn sign_drag_offset_nonempty_release_empty_drag_typing_and_selection() {
    let mut host = Host::default();
    let mut tool = SignTool::new(0);
    let view = utilities();
    let a = host.insert(SIGN_TYPE, b"a", Vec3::ZERO);
    let b = host.insert(SIGN_TYPE, b"", Vec3::ZERO);
    tool.select(&mut host, Some(b));
    host.clear();
    tool.select(&mut host, Some(a));
    assert_eq!(host.events, vec![E::Remove(b), E::Dynamic(a, true)]);
    host.renderer_picks.push_back(Some(a));
    tool.mouse_down(&mut host, &view, Pixel::new(90, 45)).unwrap();
    host.clear();
    host.hit(Vec3::new(8.0, 9.0, 10.0));
    tool.mouse_move(&mut host, Pixel::new(100, 50));
    assert_eq!(
        host.events,
        vec![E::Pick(Pixel::new(110, 55)), E::Position(a, Vec3::new(8.0, 9.0, 10.0))]
    );
    host.clear();
    tool.mouse_up(&mut host);
    assert_eq!(tool.state(), MouseState::Idle);
    assert_eq!(host.events, vec![E::Sound(0xEA45_1903, 0x2C16_DA29)]);
    let empty = host.insert(SIGN_TYPE, b"", Vec3::ZERO);
    host.renderer_picks.push_back(Some(empty));
    tool.mouse_down(&mut host, &view, Pixel::new(100, 50)).unwrap();
    tool.mouse_move(&mut host, Pixel::new(1, 2));
    tool.mouse_up(&mut host);
    assert_eq!(tool.state(), MouseState::Typing);
    host.clear();
    tool.mouse_down(&mut host, &view, Pixel::new(0, 0)).unwrap();
    assert_eq!(tool.state(), MouseState::Idle);
    assert_eq!(host.events, vec![E::Edit(empty, false), E::Dynamic(empty, true)]);
    assert!(tool.key_down(&mut host, Key::Delete, 0, ServiceText::Unbound).unwrap());
    assert_eq!(tool.selected(), None);
}

#[test]
fn filters_require_prop_and_use_interface_vs_exact_type() {
    for filter in [PropFilter::Label, PropFilter::Sign, PropFilter::SignOrLabel] {
        assert!(!filter.accepts(PropInfo {
            is_prop: false,
            prop_type: SIGN_TYPE,
            text_decal: true,
            signpost: true
        }));
    }
    let foreign = PropInfo {
        is_prop: true,
        prop_type: 123,
        text_decal: true,
        signpost: true,
    };
    assert!(PropFilter::Label.accepts(foreign));
    assert!(PropFilter::Sign.accepts(foreign));
    assert!(!PropFilter::SignOrLabel.accepts(foreign));
    let typed = PropInfo {
        is_prop: true,
        prop_type: LABEL_TYPE,
        ..Default::default()
    };
    assert!(PropFilter::SignOrLabel.accepts(typed));
    assert!(!PropFilter::Label.accepts(typed));
}

#[test]
fn demolition_renderer_first_fallback_highlight_order_and_removal() {
    let mut host = Host::default();
    let label = host.insert(LABEL_TYPE, b"label", Vec3::ZERO);
    let sign = host.insert(SIGN_TYPE, b"sign", Vec3::ZERO);
    let mut tool = SignLabelDemolish::default();
    host.renderer_picks.push_back(Some(label));
    tool.mouse_move(&mut host, Pixel::new(3, 4));
    assert_eq!(
        host.events,
        vec![
            E::PickProp(Pixel::new(3, 4), PropFilter::SignOrLabel),
            E::Alpha(label, 0.7)
        ]
    );
    host.clear();
    host.hit(Vec3::new(1.0, 2.0, 3.0));
    host.occupant_picks.push_back(Some(sign));
    tool.mouse_move(&mut host, Pixel::new(5, 6));
    assert_eq!(
        host.events,
        vec![
            E::PickProp(Pixel::new(5, 6), PropFilter::SignOrLabel),
            E::Pick(Pixel::new(5, 6)),
            E::First(Vec3::new(1.0, 2.0, 3.0), PropFilter::SignOrLabel),
            E::Alpha(label, 0.5),
            E::Dynamic(sign, true)
        ]
    );
    host.clear();
    assert!(tool.mouse_up(&mut host));
    assert_eq!(host.events, vec![E::Remove(sign), E::Sound(0xEA45_1903, 0x8A5E_C047)]);
    assert_eq!(tool.selected(), None);
    host.clear();
    tool.mouse_up(&mut host);
    assert!(host.events.is_empty());
}

#[test]
fn label_miss_and_dialog_resource_boundary() {
    let mut host = Host {
        renderer: false,
        ..Default::default()
    };
    let mut tool = LabelTool::default();
    assert!(tool.mouse_down(&mut host, Pixel::new(0, 0)).unwrap());
    assert!(host.events.is_empty());
    host.renderer = true;
    assert!(!tool.mouse_down(&mut host, Pixel::new(0, 0)).unwrap());
    host.script_found = false;
    host.hit(Vec3::ZERO);
    assert_eq!(
        tool.mouse_down(&mut host, Pixel::new(0, 0)),
        Err(Unsupported::ResourceUnavailable)
    );
    assert!(host.props.iter().all(Option::is_none));
}

#[test]
fn independent_road_and_avenue_vectors() {
    for (edges, orientation, offset) in [
        ([0, 2, 0, 2], 0, (0.0, 0.0)),
        ([1, 3, 0, 0], 2, (-4.0, -4.0)),
        ([2, 1, 2, 0], 1, (0.0, 0.0)),
        ([0, 0, 0, 2], 0, (0.0, 0.0)),
        ([0, 0, 2, 0], 1, (0.0, 0.0)),
        ([0, 0, 2, 3], 2, (4.0, 4.0)),
        ([2, 0, 0, 3], 3, (-4.0, 4.0)),
        ([0, 2, 3, 0], 3, (4.0, -4.0)),
        ([2, 2, 2, 2], 1, (0.0, 0.0)),
        ([0, 0, 0, 0], 1, (0.0, 0.0)),
    ] {
        assert_eq!(road_center_and_orientation(edges), (orientation, offset));
    }
    for (edges, orientation, offset) in [
        ([1, 1, 1, 1], 2, (0.0, 0.0)),
        ([3, 0, 0, 0], 3, (-8.0, 8.0)),
        ([2, 1, 1, 2], 2, (-8.0, 8.0)),
        ([4, 2, 0, 0], 0, (-8.0, 0.0)),
        ([3, 3, 3, 3], 3, (0.0, 0.0)),
        ([1, 0, 0, 0], 2, (-8.0, -8.0)),
        ([0, 0, 1, 0], 2, (8.0, 8.0)),
        ([0, 0, 3, 0], 3, (8.0, -8.0)),
        ([2, 2, 1, 1], 3, (-8.0, -8.0)),
        ([1, 1, 2, 2], 3, (8.0, 8.0)),
        ([1, 2, 2, 1], 2, (8.0, -8.0)),
        ([1, 2, 1, 2], 1, (0.0, 0.0)),
        ([6, 4, 4, 4], 1, (0.0, 0.0)),
        ([0, 0, 0, 0], 1, (0.0, 0.0)),
    ] {
        assert_eq!(avenue_center_and_orientation(edges), (orientation, offset));
    }
}
