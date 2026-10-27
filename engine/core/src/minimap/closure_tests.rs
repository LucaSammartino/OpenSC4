// SPDX-License-Identifier: GPL-3.0-or-later
use super::tests::fixture;
use super::*;
use base::Rect;

#[test]
fn terrain_live_blocks_zone_overwrite_and_open_water_boundary() {
    let (mut w, mut h, mut m) = fixture();
    h.cells = [32, 16];
    h.terrain = Some(([32, 16], 100.0));
    m.set_area(&mut w, &mut h, Rect(0, 0, 32, 32)).unwrap();
    m.on_city_init(&mut h).unwrap();
    m.flags = 1;
    m.terrain_palette.fill(0xffffff);
    assert_eq!(m.update_terrain_and_zone_map(&mut h), Err(Unsupported::Open("MMW1")));
    assert!(m.map_dirty);
    h.water = Some(Box::new([0.0; 256]));
    h.zone = 2;
    m.set_zone_color(2, 0x112233);
    h.height_rects.clear();
    m.update_terrain_and_zone_map(&mut h).unwrap();
    assert_eq!(h.height_rects, [Rect(0, 0, 15, 15), Rect(16, 0, 31, 15)]);
    let p = m.map.pixels().unwrap();
    assert_eq!([p[0], p[31 * 32], p[31 * 32 + 15]], [0xff112233; 3]);
    assert_eq!(p[16], 0);
    assert!(!m.map_dirty);
    assert_eq!(m.dirty, [0; 16]);
    m.set_default_zone_color_to_trans();
    m.invalidate_tz_map_rect(Rect(17, 2, 17, 2)).unwrap();
    h.height_rects.clear();
    m.update_terrain_and_zone_map(&mut h).unwrap();
    assert_eq!(h.height_rects, [Rect(16, 0, 31, 15)]);
    assert_eq!(m.map.pixels().unwrap()[16 * 32], 0xc8c8c8);
    assert_eq!(m.map.pixels().unwrap()[0], 0xff112233);
    h.zone = -1;
    m.invalidate_tz_map();
    assert!(matches!(
        m.update_terrain_and_zone_map(&mut h),
        Err(Unsupported::Precondition(_))
    ));
    assert!(m.map_dirty);
}

#[test]
fn palettes_load_alpha_policy_and_terrain_tail_skips_incomplete_blocks() {
    let (mut w, mut h, mut m) = fixture();
    h.ramps.insert(0xea3573af, vec![(0, 0xff102030)]);
    h.ramps.insert(0x6b733233, vec![(0, 0x80112233)]);
    m.load_tunable_values(&mut h).unwrap();
    assert_eq!(m.terrain_palette, [0x00102030; 256]);
    assert_eq!(m.water_palette, [0x80112233; 256]);
    h.cells = [16, 16];
    m.set_area(&mut w, &mut h, Rect(0, 0, 16, 16)).unwrap();
    m.on_city_init(&mut h).unwrap();
    h.terrain = Some(([17, 15], 100.0));
    m.update_terrain_and_zone_map(&mut h).unwrap();
    assert!(h.height_rects.is_empty());
    assert!(!m.map_dirty);
    assert_eq!(m.dirty, [0; 16]);
}

#[test]
fn overlay_only_bypasses_terrain_and_networks() {
    let (mut w, mut h, mut m) = fixture();
    m.n = 1;
    m.map.init(1, 1).unwrap();
    m.surface = Some(10);
    m.set_overlay(&mut h, Some(20));
    m.flags = 15;
    m.map_dirty = true;
    h.terrain = Some(([16, 16], 100.0));
    h.surfaces.insert(
        10,
        SurfacePixels {
            pixels: vec![0],
            pitch: 1,
        },
    );
    h.surfaces.insert(
        20,
        SurfacePixels {
            pixels: vec![0x80445566],
            pitch: 1,
        },
    );
    m.update(&mut w, &mut h).unwrap();
    assert_eq!(h.surfaces[&10].pixels, [0xff445566]);
    assert_eq!(h.finds, 0);
    assert!(h.height_rects.is_empty());
    assert!(m.map_dirty && !m.surface_dirty);
}

#[test]
fn paint_geometry_blit_frustum_marker_blink_and_unlock_failure() {
    let (mut w, mut h, mut m) = fixture();
    m.n = 2;
    m.surface = Some(10);
    m.flags = 4 | 0x10;
    assert_eq!(m.paint(&mut w, &mut h), Err(Unsupported::Open("MMW2")));
    h.geometry = Some(PaintGeometry {
        origin: [3, 5],
        copy_area: Rect(0, 0, 2, 2),
        marker_area: Rect(0, 0, 100, 200),
    });
    h.buffer = Some(PaintPixels {
        format: 9,
        pitch: 0,
        pixels: vec![],
    });
    h.surfaces.insert(
        10,
        SurfacePixels {
            pixels: vec![0x888888; 4],
            pitch: 2,
        },
    );
    assert_eq!(m.paint(&mut w, &mut h), Err(Unsupported::Open("MMW3")));
    assert_eq!(h.events.iter().filter(|e| e.starts_with("draw:Line")).count(), 4);
    assert!(h.events.iter().any(|e| e.starts_with("blit:Rect(3, 5, 5, 7)")));
    h.marker = Some(Some(300));
    assert!(m.paint(&mut w, &mut h).unwrap());
    assert!(m.blink);
    assert!(h.events.contains(&"marker:ffc00000".into()));
    m.paint(&mut w, &mut h).unwrap();
    assert!(!m.blink);
    assert!(h.events.contains(&"marker:ffff4040".into()));
    assert!(!h.events.iter().any(|e| e.starts_with("release:")));
    m.flags = 0;
    h.geometry.as_mut().unwrap().copy_area = Rect(0, 0, 3, 2);
    h.buffer = Some(PaintPixels {
        format: 7,
        pitch: 3,
        pixels: vec![0; 6],
    });
    h.events.clear();
    assert!(matches!(m.paint(&mut w, &mut h), Err(Unsupported::Precondition(_))));
    assert_eq!(&h.events[h.events.len() - 2..], ["unlock:10", "buffer-unlock"]);
}

#[test]
fn paint_recreates_freed_map_and_tick_uses_raw_marker_service() {
    let (mut w, mut h, mut m) = fixture();
    h.factory = Some(Some(10));
    h.init_surface_ok = true;
    h.cells = [16, 16];
    m.set_area(&mut w, &mut h, Rect(0, 0, 16, 16)).unwrap();
    m.init(&mut w, &mut h).unwrap();
    w.get_mut(m.window).flags &= !1;
    m.update(&mut w, &mut h).unwrap();
    assert!(m.map.pixels().is_none());
    w.get_mut(m.window).flags |= 1;
    m.flags = 0;
    h.geometry = Some(PaintGeometry {
        origin: [0; 2],
        copy_area: Rect(0, 0, 16, 16),
        marker_area: Rect(0, 0, 16, 16),
    });
    m.paint(&mut w, &mut h).unwrap();
    assert_eq!(m.map.pixels().unwrap().len(), 256);
    m.flags = 0x10;
    assert_eq!(
        m.tick(&mut w, &mut h, 1001f32.to_bits()),
        Err(Unsupported::Open("MMW3"))
    );
    h.marker = Some(Some(300));
    h.active_marker = true;
    h.events.clear();
    m.tick(&mut w, &mut h, 1001f32.to_bits()).unwrap();
    assert_eq!(h.events, [format!("repaint:{}", m.window)]);
    m.on_city_shutdown(&mut h);
    m.shutdown(&mut w, &mut h);
    m.deleting_destructor(&mut w, &mut h, 1);
}

#[test]
fn known_message_lifecycle_and_network_payload_routes() {
    let (mut w, mut h, mut m) = fixture();
    assert!(m
        .do_message(
            &mut w,
            &mut h,
            Message {
                kind: 0x26d31ec1,
                ..Message::default()
            }
        )
        .unwrap());
    assert!(m.attached);
    assert!(m
        .do_message(
            &mut w,
            &mut h,
            Message {
                kind: 0x99ef1142,
                ..Message::default()
            }
        )
        .unwrap());
    m.do_message(
        &mut w,
        &mut h,
        Message {
            kind: 0xa6b79612,
            ..Message::default()
        },
    )
    .unwrap();
    assert!(m.repaint);
    m.do_message(
        &mut w,
        &mut h,
        Message {
            kind: 0x86ad10ee,
            word19: 0.0f32.to_bits(),
            ..Message::default()
        },
    )
    .unwrap();
    assert!(!m.repaint);
    m.do_message(
        &mut w,
        &mut h,
        Message {
            kind: 0x26d31ec2,
            ..Message::default()
        },
    )
    .unwrap();
    assert!(!m.attached);
}
