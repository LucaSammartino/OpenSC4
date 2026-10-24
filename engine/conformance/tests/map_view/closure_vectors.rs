// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn v36_traffic_eligibility_duplicates_bias_cap_and_all_networks() {
    let (_, _, _, _, mut sims) = inputs(76);
    sims.traffic.capacities.extend([(0, 1), (1, 1)]);
    let mut h0 = [0; 17];
    h0[16] = 3;
    h0[2] = 10;
    let mut h1 = [0; 17];
    h1[16] = 5;
    h1[7] = 4;
    let mut h99 = [0; 17];
    h99[16] = 7;
    sims.traffic.densities.extend([(0, h0), (1, h1), (99, h99)]);
    let g = reconstruct_traffic([1, 1], 0x4c, &[0, 1, 1, 8, 99], 256, false, &mut sims).unwrap();
    assert_eq!((g.volume, g.x, g.z), (vec![22], vec![20], vec![16]));
    assert_eq!(sims.traffic.network_reads, [0, 1, 1, 5, 1]);
    assert_eq!(
        sims.traffic.density_reads,
        [(0, false), (1, false), (1, false), (99, false)]
    );
    assert_eq!(
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 99].map(traffic_network),
        [0, 1, 1, 2, 1, 2, 3, 4, 5, 1]
    );
    assert_eq!(
        reconstruct_traffic([1, 1], 1, &[], 256, false, &mut sims)
            .unwrap()
            .volume,
        [0]
    );
    sims.traffic.capacities.insert(0, -1);
    assert_eq!(
        reconstruct_traffic([1, 1], 1, &[0, 8], 256, false, &mut sims)
            .unwrap()
            .volume,
        [0]
    );
    sims.traffic.capacities.insert(0, 999999);
    h0[16] = 2000;
    sims.traffic.densities.insert(0, h0);
    assert_eq!(
        reconstruct_traffic([1, 1], 1, &[0], 1024, false, &mut sims)
            .unwrap()
            .volume,
        [1020]
    );
}

#[test]
fn v37_unsigned_records_final_balance_clamp_alternate_and_fresh_grids() {
    let (_, _, _, _, mut sims) = inputs(76);
    sims.traffic.capacities.insert(1, 1);
    let mut h = [0; 17];
    h[2] = 65535;
    h[13] = 65535;
    sims.traffic.densities.insert(1, h);
    for alternate in [false, true] {
        let g = reconstruct_traffic([1, 1], 1, &[1], 256, alternate, &mut sims).unwrap();
        assert_eq!((g.volume, g.x, g.z), (vec![2], vec![32767], vec![-32768]));
    }
    assert_eq!(sims.traffic.density_reads, [(1, false), (1, true)]);
}

fn path(count: u16, alternate: u16, direction: u8) -> FerryPath {
    FerryPath {
        normal: count,
        alternate,
        points: vec![([0, 0], direction)],
    }
}
#[test]
fn v38_ferry_wrapping_per_point_clamp_path_order_terminal_and_alternate() {
    let (_, _, _, _, mut sims) = inputs(76);
    sims.traffic.routes = vec![
        FerryRoute {
            paths: [path(40000, 0, 2), path(40000, 0, 0)],
        },
        FerryRoute {
            paths: [path(40000, 0, 0), path(7, 0, 255)],
        },
    ];
    let g = reconstruct_traffic([1, 1], 0x54, &[], 256, false, &mut sims).unwrap();
    assert_eq!((g.volume, g.x, g.z), (vec![54471], vec![-32768], vec![0]));
    sims.traffic.routes[1].paths[1].points.clear();
    assert_eq!(
        reconstruct_traffic([1, 1], 0x54, &[], 256, false, &mut sims)
            .unwrap()
            .volume,
        [54464]
    );
    sims.traffic.routes = vec![FerryRoute {
        paths: [path(3, 7, 255), path(5, 11, 255)],
    }];
    for (alternate, total) in [(false, 8), (true, 18)] {
        assert_eq!(
            reconstruct_traffic([1, 1], 0x54, &[], 0, alternate, &mut sims)
                .unwrap()
                .volume,
            [total]
        );
    }
    assert!(sims.traffic.network_reads.is_empty());
}

#[test]
fn v39_animation_overlay_palette_overrides_sprites_and_standard_factors() {
    let mut g = TrafficGrids {
        size: [16, 5],
        volume: vec![0; 80],
        x: vec![0; 80],
        z: vec![0; 80],
    };
    for (x, z, bx, bz, v) in [
        (1, 4, 20, 0, 20),
        (14, 4, -20, 0, 20),
        (4, 1, 20, 20, 20),
        (2, 0, 20, 0, 10),
        (1, 3, 20, 20, 20),
    ] {
        let i = x * 5 + z;
        g.volume[i] = v;
        g.x[i] = bx;
        g.z[i] = bz;
    }
    g.animate(1, 256);
    for cell in [[1, 4], [14, 4], [4, 1]] {
        assert_eq!(g.volume[cell[0] * 5 + cell[1]], 256);
    }
    assert_eq!(g.volume[10], 10);
    assert_eq!(g.volume[8], 20);
    for explicit in [false, true] {
        let (mut view, mut res, mut win, mut render, mut sims) = inputs(76);
        if explicit {
            set_property(res.raw.as_mut().unwrap(), SCALE, RawValue::U32(256));
        }
        set_property(res.raw.as_mut().unwrap(), TRAVEL_TYPES, RawValue::U32List(vec![1]));
        sims.traffic.capacities.insert(1, 1);
        let mut h = [0; 17];
        h[16] = 18;
        sims.traffic.densities.insert(1, h);
        sims.traffic.damaged = vec![[0, 0]];
        render.damaged_icon = Some(IncidentIcon {
            handle: 77,
            width: 1,
            height: 1,
            pixels: vec![0xff00ff00],
        });
        view.set_overlay_enabled(true);
        view.select_with(0x4c, &mut res, &mut win, &mut render, &mut sims)
            .unwrap();
        let p = view.owned_palettes().unwrap();
        assert_eq!(
            (p.straight.lookup(255), p.premultiplied.lookup(255)),
            (0xcfffffff, 0xffffffff)
        );
        assert_eq!(render.cells.last().unwrap().colors, [0, p.straight.lookup(19)]);
        assert_eq!(
            render.bitmaps.last().unwrap()[1],
            p.premultiplied.lookup(if explicit { 19 } else { 0 })
        );
        assert_eq!(render.damaged_loads, 1);
        assert_eq!(render.released_icons, [77]);
        view.draw_with(&mut render, &mut sims).unwrap();
        assert_eq!(render.cells.len(), 2); // overlay restored after the volume draw
    }
}

fn flag(win: &Windows, control: u32, bit: u8) -> bool {
    *win.flags.get(&(control, bit)).unwrap()
}
fn first_show(view: &mut MapView, res: &mut Resource, win: &mut Windows, render: &mut Render, sims: &mut Sims) {
    view.show_with(PanelRequest::Show, res, win, render, sims).unwrap();
}
fn press(
    view: &mut MapView,
    button: u32,
    res: &mut Resource,
    win: &mut Windows,
    render: &mut Render,
    sims: &mut Sims,
) -> u32 {
    view.button_with(button, res, win, render, sims).unwrap();
    view.requested_view().unwrap_or(0)
}

#[test]
fn v40_panel_matrix_preserves_dashes_and_legend_flag() {
    let (mut view, mut res, mut win, _, _) = inputs(0);
    let ids = [
        0x8a2871b1, 0x8a2871b2, 0xaa32bcd4, 0xaa32bcd5, 0x8a2871c3, 0x8a2871c4, 0xcb72f247, 0x8100, 0x8101,
    ];
    for id in ids {
        win.flags.insert((id, 1), true);
    }
    for (layout, expected) in [
        (
            PanelRequest::Small,
            [true, true, false, false, false, false, true, true, true],
        ),
        (
            PanelRequest::Desirability,
            [false, true, false, true, true, false, false, false, false],
        ),
        (
            PanelRequest::Traffic,
            [false, true, false, true, true, false, true, true, true],
        ),
        (
            PanelRequest::Large,
            [false, true, true, false, true, false, true, false, false],
        ),
    ] {
        view.layout_with(layout, &mut win).unwrap();
        assert_eq!(ids.map(|id| flag(&win, id, 1)), expected);
    }
    view.display_legend(1, &mut res, &mut win).unwrap();
    view.layout_with(PanelRequest::Large, &mut win).unwrap();
    assert!(flag(&win, 0x8a2871c4, 1));
    win.denied_controls.push(0x8100);
    win.flags.insert((0x8100, 1), true);
    view.layout_with(PanelRequest::Large, &mut win).unwrap();
    assert!(flag(&win, 0x8100, 1));
}

#[test]
fn v41_independent_requested_memories_aliases_and_failed_selection() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(0);
    first_show(&mut view, &mut res, &mut win, &mut render, &mut sims);
    let mut requests = Vec::new();
    for button in [0x5005, 0x6002, 0x5002, 0x5105] {
        requests.push(press(&mut view, button, &mut res, &mut win, &mut render, &mut sims));
        if button == 0x6002 {
            assert_eq!(
                view.memories(),
                ViewMemories {
                    desirability: 0x29,
                    traffic: 0x4c,
                    mode: 0x28
                }
            );
        }
    }
    assert_eq!(requests, [0x28, 0x29, 5, 0x29]);
    assert_eq!(
        view.memories(),
        ViewMemories {
            desirability: 0x29,
            traffic: 0x4c,
            mode: 0x29
        }
    );
    assert!(win.buttons[&0x5005]);
    assert!(!win.buttons[&0x5002]);
    res.raw = None;
    press(&mut view, 0x6003, &mut res, &mut win, &mut render, &mut sims);
    assert_eq!(view.memories().desirability, 0x2a);
    assert_eq!(res.keys.last().unwrap(), &exemplar_key(0x2a).unwrap());
}

#[test]
fn v42_traffic_group_memories_enable_flags_and_absent_button() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(0);
    first_show(&mut view, &mut res, &mut win, &mut render, &mut sims);
    press(&mut view, 0x5002, &mut res, &mut win, &mut render, &mut sims);
    assert_eq!(press(&mut view, 0x5008, &mut res, &mut win, &mut render, &mut sims), 9);
    assert_eq!(view.memories().mode, 9);
    assert_eq!(
        press(&mut view, 0x8002, &mut res, &mut win, &mut render, &mut sims),
        0x4c
    );
    assert!((0x8003..=0x800c).all(|id| flag(&win, id, 2)));
    assert_eq!(view.memories().mode, 9);
    press(&mut view, 0x800b, &mut res, &mut win, &mut render, &mut sims);
    press(&mut view, 0x5002, &mut res, &mut win, &mut render, &mut sims);
    assert_eq!(
        press(&mut view, 0x5008, &mut res, &mut win, &mut render, &mut sims),
        0x54
    );
    assert_eq!(view.memories().mode, 0x54);
    press(&mut view, 0x8001, &mut res, &mut win, &mut render, &mut sims);
    assert!((0x8003..=0x800c).all(|id| !flag(&win, id, 2)));
    assert_eq!(
        press(&mut view, 0x800c, &mut res, &mut win, &mut render, &mut sims),
        0x55
    );
    win.denied_controls.push(0x8001);
    assert_eq!(
        press(&mut view, 0x5008, &mut res, &mut win, &mut render, &mut sims),
        0x55
    );
}

#[test]
fn v43_alternate_saved_queue_reset_precedence_and_same_view_guard() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(0);
    view.set_reset_requests(0x54, vec![]);
    first_show(&mut view, &mut res, &mut win, &mut render, &mut sims);
    let memories = view.memories();
    let cleared = render.clears;
    press(&mut view, 0x8101, &mut res, &mut win, &mut render, &mut sims);
    assert_eq!(view.two_way_toggle(), Some(true));
    assert_eq!(render.clears, cleared);
    assert!(flag(&win, 0x8a2871d6, 1));
    assert!(!flag(&win, 0x8a2871c5, 1));
    view.show_with(PanelRequest::Hide, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    assert_eq!(view.current_view(), Some(0));
    assert!(!flag(&win, 0x8a2871d6, 1));
    assert!(flag(&win, 0x8a2871c5, 1));
    assert!(win.effects.contains(&WindowEffect::InvalidateControl(0x8a2871c5)));
    view.set_reset_requests(0x54, vec![7, 8]);
    view.reset_with(&mut res, &mut win, &mut render, &mut sims).unwrap();
    assert_eq!(view.current_view(), Some(7));
    press(&mut view, 0x8100, &mut res, &mut win, &mut render, &mut sims);
    assert_eq!(view.two_way_toggle(), Some(false));
    assert_eq!(view.memories(), memories);
}

#[test]
fn v44_first_show_expand_precedence_and_cancel_tutorial_dispatch() {
    let (mut view, mut res, mut win, mut render, mut sims) = inputs(0);
    assert_eq!(
        view.memories(),
        ViewMemories {
            desirability: 99,
            traffic: 0x4c,
            mode: 0
        }
    );
    assert_eq!(view.two_way_toggle(), Some(false));
    first_show(&mut view, &mut res, &mut win, &mut render, &mut sims);
    for id in [0x5001, 0x6001, 0x8001, 0x8100, 0x8003] {
        assert!(win.buttons[&id]);
    }
    assert_eq!(
        win.effects
            .iter()
            .filter(|e| matches!(e, WindowEffect::MessageTarget(_)))
            .count(),
        82
    );
    assert!(view.map_enabled());
    press(&mut view, 0x6002, &mut res, &mut win, &mut render, &mut sims);
    press(&mut view, 0x8101, &mut res, &mut win, &mut render, &mut sims);
    first_show(&mut view, &mut res, &mut win, &mut render, &mut sims);
    assert_eq!(view.memories().desirability, 0x29);
    assert_eq!(view.two_way_toggle(), Some(true));
    win.buttons.insert(0x5005, true);
    set_property(res.raw.as_mut().unwrap(), SOURCE, RawValue::U32(76));
    view.select_with(99, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    assert!(!view
        .message_with(0xea8b80a6, None, &mut res, &mut win, &mut render, &mut sims)
        .unwrap());
    assert_eq!(view.requested_view(), Some(0x29));
    win.buttons.insert(0x5005, false);
    view.message_with(0xea8b80a6, None, &mut res, &mut win, &mut render, &mut sims)
        .unwrap();
    assert_eq!(view.requested_view(), Some(0x4c));
    let memories = view.memories();
    let count = win.effects.len();
    win.deny = true;
    press(&mut view, 0x8a2871d5, &mut res, &mut win, &mut render, &mut sims);
    assert_eq!(win.effects.len(), count);
    win.deny = false;
    press(&mut view, 0x8a2871d5, &mut res, &mut win, &mut render, &mut sims);
    assert_eq!(
        win.effects.last(),
        Some(&WindowEffect::DispatchCommand {
            command: 0x6a935cf1,
            arguments: [0, 0]
        })
    );
    assert_eq!(view.memories(), memories);
}

fn ring_pixel(args: [f32; 3], precision: RingPrecision, rounding: RingRounding) -> u32 {
    let mut pixels = vec![0xff000000; 64];
    paint_ring_with_context(
        &mut pixels,
        8,
        8,
        ClipRect {
            left: 0,
            top: 0,
            right: 8,
            bottom: 8,
        },
        args,
        0x00ff0000,
        RingContext { precision, rounding },
    )
    .unwrap();
    pixels[4 * 8] // bitmap helper pixel (x=0, y=4), row-major buffer
}
#[test]
fn v45_ring_pc24_differs_from_pc53_and_pc64() {
    assert_eq!(
        [RingPrecision::Bits24, RingPrecision::Bits53, RingPrecision::Bits64].map(|p| ring_pixel(
            [f32::from_bits(0xbeff8000), -0.5, 513.0 / 128.0],
            p,
            RingRounding::Nearest
        )),
        [0xfffd0000, 0xfffe0000, 0xfffe0000]
    );
}
#[test]
fn v46_ring_pc64_retains_unspilled_distance() {
    assert_eq!(
        [RingPrecision::Bits24, RingPrecision::Bits53, RingPrecision::Bits64].map(|p| ring_pixel(
            [f32::from_bits(0xbeffffff), -0.5, 513.0 / 128.0],
            p,
            RingRounding::Nearest
        )),
        [0xfffd0000, 0xfffd0000, 0xfffe0000]
    );
}
#[test]
fn v47_ring_floor_lower_ceil_upper_candidates() {
    let mut pixels = vec![0xff000000; 16];
    paint_ring(
        &mut pixels,
        4,
        4,
        ClipRect {
            left: 0,
            top: 0,
            right: 4,
            bottom: 4,
        },
        [0.0, 0.0, 1.0],
        0x00ff0000,
    )
    .unwrap();
    assert_eq!(pixels[10], 0xff700000);
}
#[test]
fn v48_ring_inherited_rounding_changes_bias_conversion() {
    assert_eq!(
        [
            RingRounding::Nearest,
            RingRounding::Down,
            RingRounding::Up,
            RingRounding::TowardZero
        ]
        .map(|rc| ring_pixel([-0.5, -0.5, 2049.0 / 512.0], RingPrecision::Bits24, rc)),
        [0xfffe0000, 0xffff0000, 0xfffe0000, 0xffff0000]
    );
}
