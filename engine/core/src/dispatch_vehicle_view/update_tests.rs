// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl WindowServices for Host {
    fn main_window(&mut self) -> Option<Window> {
        self.main
    }
    fn child_windows(&mut self, _: Token) -> Vec<Window> {
        self.children.clone()
    }
    fn window_exclusion(&mut self, child: Token) -> (bool, Option<Token>) {
        self.window_queries.push(child);
        (self.exclusions.contains(&child), Some(child + 2000))
    }
    fn release_window_object(&mut self, object: Token) {
        self.window_releases.push(object);
    }
}
impl FrameServices for Host {
    fn stop_and_sample_timer(&mut self) -> f32 {
        self.stops += 1;
        self.elapsed
    }
    fn start_timer(&mut self) {
        self.starts += 1;
    }
    fn context_blocked(&mut self) -> bool {
        self.blocked
    }
    fn render_allowed(&mut self) -> bool {
        !self.denied
    }
    fn simulator_state(&mut self) -> u8 {
        self.simulator
    }
    fn position(&mut self, _: Token) -> Vec3 {
        self.position
    }
    fn terrain_height(&mut self, _: Vec3) -> Option<f32> {
        self.terrain
    }
    fn project(&mut self, p: Vec3) -> Vec3 {
        [self.origin[0] + p[0], self.origin[1] - p[1], p[2]]
    }
    fn draw_op(&mut self, op: DrawOp) {
        self.draw.push(op);
    }
    fn icon_texture(&mut self, icon: Option<Icon>) -> Option<Token> {
        if let Some(icon) = icon {
            self.icon_reads.push(icon.identity);
        }
        self.icon_override.or_else(|| icon.and_then(|i| i.texture))
    }
    fn progress(&mut self, _: Token) -> Option<Progress> {
        self.progress.clone()
    }
    fn model_transform(&mut self, _: Token) -> Transform {
        self.transform
    }
}
fn host() -> Host {
    Host {
        main: Some(Window {
            token: 1,
            id: 1,
            area: Rect::new(0, 0, 800, 600),
            visible: true,
        }),
        origin: [400.0, 300.0],
        elapsed: 500.0,
        ..Host::default()
    }
}
fn quads(h: &Host) -> usize {
    h.draw.iter().filter(|d| matches!(d, DrawOp::Quad(_))).count()
}

#[test]
fn closed_conflicts_probe_order_and_blocked_fallback() {
    let bounds = Rect::new(0, 0, 100, 100);
    let mut obstacles = [Obstacle {
        rectangle: Rect::new(10, 10, 20, 20),
        category: 1,
    }];
    let candidate = Obstacle {
        rectangle: Rect::new(20, 12, 30, 18),
        category: 1,
    };
    assert_eq!(conflict(candidate, &obstacles, bounds), Some(obstacles[0]));
    assert_eq!(
        conflict(
            Obstacle {
                rectangle: Rect::new(21, 12, 31, 18),
                ..candidate
            },
            &obstacles,
            bounds
        ),
        None
    );
    obstacles[0].category = 2;
    assert_eq!(conflict(candidate, &obstacles, bounds), None);
    obstacles[0].category = 0xff;
    assert_eq!(conflict(candidate, &obstacles, bounds), Some(obstacles[0]));
    assert_eq!(
        conflict(
            Obstacle {
                rectangle: Rect::new(-1, 0, 10, 10),
                ..candidate
            },
            &obstacles,
            bounds
        ),
        Some(Obstacle {
            rectangle: bounds,
            category: 0xff
        })
    );
    assert_eq!(
        conflict(
            Obstacle {
                rectangle: Rect::new(0, 0, 10, 9),
                ..candidate
            },
            &obstacles,
            bounds
        ),
        None
    );
    let original = Obstacle {
        rectangle: Rect::new(10, 10, 52, 52),
        category: 1,
    };
    let obstacle = Obstacle {
        rectangle: Rect::new(20, 20, 40, 40),
        category: 1,
    };
    assert_eq!(
        relocate(original, &[obstacle], Rect::new(0, 0, 200, 200)).rectangle,
        Rect::new(20, 63, 40, 83)
    );
    let blocked = Obstacle {
        rectangle: bounds,
        category: 0xff,
    };
    assert_eq!(relocate(original, &[blocked], bounds), original);
    let second = Obstacle {
        rectangle: Rect::new(0, 0, 99, 99),
        category: 1,
    };
    assert_eq!(conflict(original, &[obstacle, second], bounds), Some(obstacle));
}

#[test]
fn window_filter_uses_or_dimensions_and_releases_exclusion_result() {
    let mut h = host();
    let child = |token, id, area, visible| Window {
        token,
        id,
        area,
        visible,
    };
    h.children = vec![
        child(2, 0, Rect::new(0, 0, 800, 100), true),
        child(3, 0, Rect::new(0, 0, 100, 600), true),
        child(4, 0, Rect::new(0, 0, 800, 600), true),
        child(5, 0xaa32_bce6, Rect::new(0, 0, 10, 10), true),
        child(6, 0, Rect::new(0, 0, 10, 10), false),
        child(7, 0, Rect::new(0, 0, 10, 10), true),
        child(0, 0, Rect::new(0, 0, 10, 10), true),
    ];
    h.exclusions = vec![7];
    let mut v = view(&mut h);
    v.rebuild_windows(&mut h);
    assert_eq!(v.windows.len(), 3);
    assert!(v.windows.iter().all(|w| w.category == 0xff));
    assert_eq!(h.window_queries, [2, 3, 7]);
    assert_eq!(h.window_releases, [2002, 2003, 2007]);
    h.main = None;
    v.rebuild_windows(&mut h);
    assert_eq!(v.windows.len(), 3);
}

#[test]
fn gates_delay_removal_and_hidden_fade_finishes_strictly_below_threshold() {
    let mut h = host();
    let mut v = view(&mut h);
    let id = v.track(1, 5, 0, None, &mut h).unwrap();
    v.find_mut(id).unwrap().fade = 0.01;
    v.untrack(id, false, &mut h);
    h.blocked = true;
    v.update(&mut h).unwrap();
    assert_eq!((h.stops, h.starts, v.counts[5]), (1, 0, 1));
    h.blocked = false;
    h.denied = true;
    v.update(&mut h).unwrap();
    assert_eq!((h.stops, h.starts, v.counts[5]), (2, 0, 1));
    h.denied = false;
    h.elapsed = 0.0;
    v.show_all(5, false);
    v.update(&mut h).unwrap();
    assert_eq!(v.counts[5], 1);
    h.elapsed = 0.000001;
    v.update(&mut h).unwrap();
    assert_eq!((v.counts[5], h.starts), (0, 2));
    assert!(h.draw.is_empty());
}

#[test]
fn flash_strict_period_lock_and_nan_angle() {
    let mut h = host();
    let mut v = view(&mut h);
    let id = v.track(1, 0, 1, None, &mut h).unwrap();
    v.flash(id, true, 1.0);
    v.find_mut(id).unwrap().flash_accumulator = 0.5;
    v.lock(id, true);
    v.find_mut(id).unwrap().rectangle = Rect::new(-3, -5, 40, 38);
    v.find_mut(id).unwrap().anchor = [18.5, 16.5, 0.0];
    v.update(&mut h).unwrap();
    assert_eq!(v.find(id).unwrap().flash_accumulator, 1.0);
    near(v.find(id).unwrap().fade, 0.0);
    assert_eq!(v.find(id).unwrap().rectangle, Rect::new(-3, -5, 40, 38));
    assert_eq!(v.find(id).unwrap().anchor, [18.5, 16.5, 0.0]);
    h.elapsed = 1.0;
    v.update(&mut h).unwrap();
    assert_eq!(
        (v.find(id).unwrap().flash_accumulator, v.find(id).unwrap().fade),
        (0.0, 1.0)
    );
    v.find_mut(id).unwrap().flash_accumulator = 5.0;
    v.flash(id, true, 2.0);
    assert_eq!(v.find(id).unwrap().flash_accumulator, 2.0);
    v.flash(id, false, 99.0);
    assert_eq!((v.find(id).unwrap().flash_period, v.find(id).unwrap().fade), (2.0, 1.0));
    let info = v.find(id).unwrap();
    let (icon, number) = screen_quads(info, info.anchor);
    assert!(icon.iter().all(|v| v.position[0].is_nan() && v.position[1].is_nan()));
    assert!(number.iter().all(|v| v.position.iter().all(|v| v.is_finite())));
}

#[test]
fn worked_building_ring_and_clamp() {
    let mut h = host();
    h.building = true;
    h.origin = [100.0, 100.0];
    let mut v = view(&mut h);
    let ids: Vec<_> = (0..3).map(|_| v.track(1, 0, 0, None, &mut h).unwrap()).collect();
    v.update(&mut h).unwrap();
    for (n, id) in ids.into_iter().enumerate() {
        let theta = f64::from(72 * (n as i32 + 1) - 90) * f64::from(f32::from_bits(0x3c8e_fa35));
        let left = (theta.cos() * 50.0 + 79.0) as f32;
        let top = (theta.sin() * 50.0 + 79.0) as f32;
        assert_eq!(v.find(id).unwrap().rectangle.left, crate::x87::ftol32(f64::from(left)));
        assert_eq!(v.find(id).unwrap().rectangle.top, crate::x87::ftol32(f64::from(top)));
    }
    let (r, moved) = clamp_rectangle(
        FloatRect {
            left: 10.0,
            top: 40.0,
            right: 52.0,
            bottom: 82.0,
        },
        Rect::new(32, 32, 768, 568),
    );
    assert_eq!(
        r,
        FloatRect {
            left: 32.0,
            top: 40.0,
            right: 74.0,
            bottom: 82.0
        }
    );
    assert!(moved);
    assert_eq!(quads(&h), 6);
}

#[test]
fn world_two_passes_do_not_become_placement_obstacles() {
    let mut h = host();
    let mut v = view(&mut h);
    let world = v.track(1, 5, 0, None, &mut h).unwrap();
    h.building = true;
    let building = v.track(2, 5, 0, None, &mut h).unwrap();
    h.building = false;
    h.terrain = Some(0.0);
    v.update(&mut h).unwrap();
    assert_eq!(v.find(world).unwrap().anchor, [0.0; 3]);
    assert_eq!(v.find(world).unwrap().rectangle, Rect::new(368, 216, 432, 280));
    assert_eq!(v.find(building).unwrap().rectangle, Rect::new(426, 263, 468, 305));
    assert_eq!(
        &h.draw[..12],
        &[
            DrawOp::ClearTexTransform(0),
            DrawOp::ModelTransform(Some(Transform::default())),
            DrawOp::Transparency(0.5),
            DrawOp::State(101),
            DrawOp::Texture(Some(200), 0),
            DrawOp::Quad(v.find(world).unwrap().icon_quad),
            DrawOp::Transparency(1.0),
            DrawOp::State(100),
            DrawOp::Texture(Some(200), 0),
            DrawOp::Quad(v.find(world).unwrap().icon_quad),
            DrawOp::ResetTransparency,
            DrawOp::ModelTransform(None)
        ]
    );
}

#[test]
fn identity_draw_order_null_number_texture_visibility_limits_and_simulator() {
    let mut h = host();
    h.image = Some(Image {
        token: 88,
        width: 200,
        height: 50,
    });
    let mut v = view(&mut h);
    let a = v.track(1, 3, 1, None, &mut h).unwrap();
    let b = v.track(2, 4, 1, None, &mut h).unwrap();
    v.update(&mut h).unwrap();
    let textures: Vec<_> = h
        .draw
        .iter()
        .filter_map(|op| if let DrawOp::Texture(t, _) = op { Some(*t) } else { None })
        .collect();
    assert_eq!(textures, [Some(99), Some(200), Some(200), Some(99)]);
    assert_ne!(
        v.find(a).unwrap().icon.unwrap().identity,
        v.find(b).unwrap().icon.unwrap().identity
    );
    h.draw.clear();
    h.simulator = 1;
    v.update(&mut h).unwrap();
    assert_eq!(quads(&h), 2);
    h.draw.clear();
    v.set_indicator_max(3, 0).unwrap();
    v.update(&mut h).unwrap();
    assert!(h.draw.is_empty());
    h.simulator = 0;
    v.set_indicator_max(4, 1).unwrap();
    v.track(3, 4, 2, None, &mut h).unwrap(); // rejected by tracking limit
    assert_eq!(v.counts[4], 1);
    let null = v.track(4, 0, 3, None, &mut h).unwrap();
    assert_eq!(v.find(null).unwrap().texture, None);
    v.update(&mut h).unwrap();
    assert!(h.draw.contains(&DrawOp::Texture(None, 0)));
}

#[test]
fn model_progress_parent_walk_material_spin_and_call_order() {
    let mut h = host();
    h.model = Some(30);
    h.processor = Some(31);
    h.progress = Some(Progress {
        numerator: 20,
        chain: vec![
            ProgressNode {
                has_resource: false,
                duration: 1.0,
            },
            ProgressNode {
                has_resource: true,
                duration: 40.0,
            },
        ],
    });
    let mut v = view(&mut h);
    let id = v.track(1, 6, 0, None, &mut h).unwrap();
    h.model_ops.clear();
    v.update(&mut h).unwrap();
    let material = h
        .model_ops
        .iter()
        .find_map(|(_, op)| if let ModelOp::Material(m) = op { Some(*m) } else { None })
        .unwrap();
    assert_eq!(material.rgba, [269.0 / 512.0, 243.0 / 512.0, 0.078125, 1.0]);
    let transform = h
        .model_ops
        .iter()
        .find_map(|(_, op)| if let ModelOp::Transform(t) = op { Some(*t) } else { None })
        .unwrap();
    assert_eq!(transform.translation, [0.0, 16.0, 0.0]);
    assert_eq!(transform.flags, [4, 5]);
    near(transform.rotation[0][0], 7.0_f32.to_radians().cos());
    near(transform.rotation[0][2], 7.0_f32.to_radians().sin());
    assert_eq!(
        &h.model_ops[h.model_ops.len() - 4..],
        &[
            (30, ModelOp::Fade(false, 1.0)),
            (30, ModelOp::Advance(500.0)),
            (30, ModelOp::Frame(1)),
            (30, ModelOp::Draw(0))
        ]
    );
    assert!(h.ops.contains(&ResourceOp::Release(Reference::Automaton, 1001)));
    assert!(h.draw.is_empty());
    h.model_ops.clear();
    h.elapsed = 0.0;
    h.complete = true;
    v.update(&mut h).unwrap();
    assert!(h.model_ops.contains(&(
        30,
        ModelOp::Material(Material {
            flags: 1,
            rgba: [0.921875, 0.0546875, 0.05859375, 1.0]
        })
    )));
    assert!(h.model_ops.contains(&(30, ModelOp::Transform(transform))));
    assert_eq!(v.find(id).unwrap().fade, 1.0);
}

#[test]
fn messages_rebuild_without_retracking_and_open_states_are_explicit() {
    let mut h = host();
    h.state_missing = true;
    let mut v = DispatchVehicleView::default();
    assert_eq!(v.init(&mut h), Err(Unsupported::OpenRenderStates));
    h.state_missing = false;
    v.init(&mut h).unwrap();
    let id = v.track(1, 0, 2, None, &mut h).unwrap();
    v.find_mut(id).unwrap().texture = Some(91);
    h.ops.clear();
    v.do_message(0, &mut h).unwrap();
    assert!(h.ops.is_empty());
    v.do_message(VIEW_CHANGED, &mut h).unwrap();
    assert_eq!(h.ops[0], ResourceOp::ReleaseTexture(91));
    assert_eq!((v.tracks()[0].handle, v.counts[0]), (id, 1));
}

#[test]
fn ambiguous_spin_row2_is_an_explicit_unsupported_boundary() {
    let a = 7.0 * f64::from(f32::from_bits(0x3c8e_fa35));
    let c = f64::from(a.cos() as f32);
    let s = a.sin();
    let values = [0.1_f32, 0.3, 1.1, 1.5, std::f32::consts::PI, 10.1, 100.3];
    let (x, z) = values
        .into_iter()
        .flat_map(|x| values.into_iter().map(move |z| (x, z)))
        .find(|&(x, z)| {
            let xs = f64::from(x) * s;
            let zc = f64::from(z) * c;
            (xs + f64::from(zc as f32)) as f32 != (f64::from(xs as f32) + zc) as f32
        })
        .expect("worked finite matrix distinguishes the two spill positions");
    let mut h = host();
    h.model = Some(30);
    h.processor = Some(31);
    h.transform.rotation[2] = [x, 0.0, z];
    let mut v = view(&mut h);
    v.track(1, 6, 0, None, &mut h).unwrap();
    h.model_ops.clear();
    h.ops.clear();
    assert_eq!(v.update(&mut h), Err(Unsupported::OpenSpinRow2));
    assert!(!h
        .model_ops
        .iter()
        .any(|(_, op)| matches!(op, ModelOp::Transform(_) | ModelOp::Draw(_))));
    assert_eq!(h.ops.last(), Some(&ResourceOp::Release(Reference::Automaton, 1001)));
}

#[test]
fn texture_resolution_follows_the_live_handle_on_each_world_pass() {
    let mut h = host();
    let mut v = view(&mut h);
    let id = v.track(1, 5, 0, None, &mut h).unwrap();
    let identity = v.find(id).unwrap().icon.unwrap().identity;
    h.icon_override = Some(888);
    v.update(&mut h).unwrap();
    let textures: Vec<_> = h
        .draw
        .iter()
        .filter_map(|op| if let DrawOp::Texture(t, _) = op { Some(*t) } else { None })
        .collect();
    assert_eq!(textures, [Some(888), Some(888)]);
    assert_eq!(h.icon_reads, [identity, identity]);
}
