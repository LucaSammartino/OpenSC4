// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
#[path = "update_tests.rs"]
mod frame;

#[derive(Default)]
pub(super) struct Host {
    ops: Vec<ResourceOp>,
    model_ops: Vec<(Token, ModelOp)>,
    camera: Camera,
    sizes: [i32; 2],
    image: Option<Image>,
    surface: Option<Token>,
    surface_ok: bool,
    lock_ok: bool,
    sides: Vec<u32>,
    building: bool,
    model: Option<Token>,
    processor: Option<Token>,
    state_missing: bool,
    draw: Vec<DrawOp>,
    elapsed: f32,
    blocked: bool,
    denied: bool,
    simulator: u8,
    starts: usize,
    stops: usize,
    main: Option<Window>,
    children: Vec<Window>,
    exclusions: Vec<Token>,
    window_queries: Vec<Token>,
    window_releases: Vec<Token>,
    origin: [f32; 2],
    position: Vec3,
    terrain: Option<f32>,
    progress: Option<Progress>,
    complete: bool,
    removed: bool,
    transform: Transform,
    icon_override: Option<Token>,
    icon_reads: Vec<Token>,
}
impl DisplayServices for Host {
    fn camera(&mut self) -> Camera {
        self.camera
    }
    fn resource_op(&mut self, op: ResourceOp) {
        self.ops.push(op);
    }
    fn font(&mut self, _: u32) -> Option<Token> {
        Some(10)
    }
    fn font_style(&mut self, _: Token) -> u32 {
        7
    }
    fn measure_text(&mut self, _: Token, _: &str) -> [i32; 2] {
        self.sizes
    }
    fn create_surface(&mut self) -> Option<Token> {
        self.surface
    }
    fn init_surface(&mut self, _: Token, side: u32, descriptor: u32) -> bool {
        assert_eq!(descriptor, 9);
        self.sides.push(side);
        self.surface_ok
    }
    fn lock_surface(&mut self, _: Token, flags: u32) -> bool {
        assert_eq!(flags, 0x8001);
        self.lock_ok
    }
    fn texture_service_present(&mut self) -> bool {
        true
    }
    fn setup_texture(&mut self, _: Token) -> Option<Token> {
        Some(99)
    }
    fn resource_image(&mut self, _: u32) -> Option<(Token, Image)> {
        self.image.map(|i| (80, i))
    }
    fn image_factory(&mut self) -> Option<Token> {
        Some(81)
    }
    fn factory_image(&mut self, _: Token, _: &str, flags: u32) -> Option<Image> {
        assert_eq!(flags, 8);
        self.image
    }
}
impl Services for Host {
    fn icon_handle(&mut self, instance: u32) -> Option<Icon> {
        Some(Icon {
            identity: u64::from(instance),
            texture: Some(200),
        })
    }
    fn render_states(&mut self) -> Option<[Token; 3]> {
        if self.state_missing {
            None
        } else {
            Some([100, 101, 102])
        }
    }
    fn subscribe(&mut self, _: u32, _: bool) {}
    fn occupant_type(&mut self, _: Token) -> u32 {
        if self.building {
            0x2781_28a0
        } else {
            0x496e_5783
        }
    }
    fn automaton(&mut self, occupant: Token) -> Option<Token> {
        Some(occupant + 1000)
    }
    fn automaton_flag(&mut self, _: Token, flag: u32) -> bool {
        if flag == 1 {
            self.removed
        } else if flag == 0x0800_0000 {
            self.complete
        } else {
            false
        }
    }
    fn set_automaton_flag(&mut self, _: Token, flag: u32, _: bool) {
        assert_eq!(flag, 0x200000);
    }
    fn load_model(&mut self, key: [u32; 3]) -> Option<Token> {
        assert_eq!(key, MODEL_KEY);
        self.model
    }
    fn model_op(&mut self, model: Token, op: ModelOp) {
        self.model_ops.push((model, op));
    }
    fn allocate_processor(&mut self) -> Option<Token> {
        self.processor
    }
    fn model_base(&mut self, _: Token) -> Token {
        500
    }
    fn model_bounds(&mut self, _: Token) -> [Vec3; 2] {
        [[0.0, -5.0, 0.0], [0.0, 7.0, 0.0]]
    }
    fn model_material(&mut self, _: Token) -> Material {
        Material {
            flags: 1,
            rgba: [0.0; 4],
        }
    }
}
pub(super) fn view(host: &mut Host) -> DispatchVehicleView {
    let mut v = DispatchVehicleView::default();
    v.init(host).unwrap();
    v
}
pub(super) fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.0001, "{a} != {b}");
}

#[test]
fn worked_strip_and_quarter_camera_rotation() {
    let mut h = Host {
        image: Some(Image {
            token: 88,
            width: 200,
            height: 50,
        }),
        ..Host::default()
    };
    let mut t = TrackInfo::for_tracking(1, 4, 123, None, None);
    t.frame = 2;
    build_display(&mut t, &mut h).unwrap();
    assert_eq!(t.display_size, [35.0; 2]);
    assert_eq!(t.display_quad[0].uv, [100.0 / 256.0, 0.0]);
    assert_eq!(t.display_quad[2].uv, [150.0 / 256.0, 50.0 / 256.0]);
    t.frame = 259;
    build_display(&mut t, &mut h).unwrap();
    assert_eq!(t.display_quad[2].uv[0], 200.0 / 256.0);
    let q = t.icon_quad;
    h.camera.forward_x = -1.0;
    h.camera.forward_z = 0.0;
    build_display(&mut t, &mut h).unwrap();
    for (a, b) in q.iter().zip(t.icon_quad) {
        near(b.position[0], a.position[2]);
        near(b.position[1], a.position[1]);
        near(b.position[2], -a.position[0]);
        assert_eq!(a.uv, b.uv);
    }
    assert_eq!(
        h.ops
            .iter()
            .filter(|op| **op == ResourceOp::Release(Reference::Resource, 80))
            .count(),
        3
    );
}

#[test]
fn number_surface_sizing_failure_and_worked_geometry() {
    for (sizes, side) in [([31, 32], 32), ([32, 32], 32), ([33, 14], 64), ([65, 14], 128)] {
        let mut h = Host {
            sizes,
            surface: Some(50),
            surface_ok: true,
            ..Host::default()
        };
        let mut t = TrackInfo::for_tracking(1, 0, u32::MAX, None, None);
        build_display(&mut t, &mut h).unwrap();
        assert_eq!(h.sides, [side]);
        assert!(h.ops.contains(&ResourceOp::DrawText {
            font: 10,
            surface: 50,
            text: "-1".into(),
            colour: 0xff00_0000,
            flags: 0x0800_0000
        }));
        assert!(!h.ops.iter().any(|op| matches!(op, ResourceOp::ClearSurface(..))));
        assert_eq!(t.texture, Some(99));
    }
    let mut h = Host {
        sizes: [10, 12],
        ..Host::default()
    };
    let mut t = TrackInfo::for_tracking(1, 0, 1, None, None);
    t.texture = Some(42);
    build_display(&mut t, &mut h).unwrap();
    assert_eq!(
        h.ops,
        [
            ResourceOp::ReleaseTexture(42),
            ResourceOp::Acquire(Reference::Font, 10),
            ResourceOp::FontStyle(10, 3),
            ResourceOp::FontStyle(10, 7),
            ResourceOp::Release(Reference::Font, 10)
        ]
    );
    assert_eq!(t.texture, None);
    assert_eq!(t.display_size, [10.0, 14.0]);
    assert_eq!(t.icon_quad[0].position, [-32.0, 84.0, 0.0]);
    assert_eq!(t.icon_quad[2].position, [32.0, 20.0, 0.0]);
    assert_eq!(t.display_quad[0].position, [-5.0, 58.0, 0.3]);
    assert_eq!(t.display_quad[2].position, [5.0, 46.0, 0.3]);
    assert_eq!(t.display_quad[2].uv, [0.3125, 0.375]);
    t.kind = 3;
    let old = t.display_quad;
    build_display(&mut t, &mut h).unwrap();
    assert_eq!(t.display_quad, old);
    t.flags |= flags::MODEL;
    t.texture = Some(43);
    h.ops.clear();
    build_display(&mut t, &mut h).unwrap();
    assert!(h.ops.is_empty());
    assert_eq!(t.texture, Some(43));
}

#[test]
fn priority_selection_active_frames_and_hidden_fallback() {
    let mut h = Host::default();
    let mut v = view(&mut h);
    let low = v.track(1, 1, 2, None, &mut h).unwrap();
    let high = v.track(1, 3, 3, None, &mut h).unwrap();
    assert!(!v.find(low).unwrap().has(flags::TOP));
    assert!(v.find(high).unwrap().has(flags::TOP));
    v.show_all(3, false);
    assert!(v.find(low).unwrap().has(flags::TOP));
    let a = v.track(2, 4, 5, None, &mut h).unwrap();
    let b = v.track(3, 4, 6, None, &mut h).unwrap();
    for id in [a, b] {
        let t = v.find_mut(id).unwrap();
        t.fade = 0.9;
        t.rectangle = Rect::new(1, 2, 9, 10);
    }
    assert_eq!(v.find_indicator_handle(9, 10), b);
    v.find_mut(b).unwrap().fade = 0.89;
    assert_eq!(v.find_indicator_handle(9, 10), a);
    v.find_mut(b).unwrap().fade = f32::NAN;
    assert!(!v.is_selectable(v.find(b).unwrap()));
    v.set_active(a, 3, &mut h).unwrap();
    assert_eq!(v.active_occupant(), Some(2));
    v.set_active(a, 9, &mut h).unwrap();
    assert_eq!(v.find(a).unwrap().frame, 3);
    v.set_active(b, 2, &mut h).unwrap();
    assert_eq!((v.active, v.find(a).unwrap().frame), (0, 1));
    v.find_mut(a).unwrap().rectangle = Rect::new(9, 8, 1, 2);
    assert!(v.last_screen_icon_rect(a).is_some());
    v.find_mut(a).unwrap().rectangle.right = 9;
    assert!(v.last_screen_icon_rect(a).is_none());
    assert!(v.use_alternate_model(4));
    assert!(v.find(a).unwrap().has(flags::ALTERNATE));
    assert!(!v.use_alternate_model(6));
}

#[test]
fn track_limits_global_handles_model_lifecycle_and_removal() {
    let mut h = Host {
        model: Some(30),
        processor: Some(31),
        ..Host::default()
    };
    let mut v = view(&mut h);
    v.set_indicator_max(5, 1).unwrap();
    let a = v.track(1, 5, 0, None, &mut h).unwrap();
    assert_eq!(v.track(1, 5, 0, None, &mut h), Ok(0));
    v.set_indicator_max(5, 0).unwrap();
    assert_ne!(v.track(1, 5, 0, None, &mut h), Ok(0));
    let mut other = view(&mut h);
    let b = other.track(1, 5, 0, None, &mut h).unwrap();
    assert_ne!(a, b);
    let m = v.track(2, 6, 0, None, &mut h).unwrap();
    assert_eq!(v.find(m).unwrap().display_size[1], 12.0);
    assert!(v.find(m).unwrap().has(flags::PROGRESS));
    assert!(h.model_ops.contains(&(
        30,
        ModelOp::Material(Material {
            flags: 0x11,
            rgba: [0.0; 4]
        })
    )));
    assert!(v.untrack(m, false, &mut h));
    assert_eq!(v.counts[6], 1);
    h.ops.clear();
    assert!(v.untrack(m, true, &mut h));
    assert_eq!(
        h.ops,
        [
            ResourceOp::Acquire(Reference::Occupant, 2),
            ResourceOp::Release(Reference::Model, 30),
            ResourceOp::Release(Reference::Occupant, 2),
            ResourceOp::Release(Reference::Automaton, 1002),
            ResourceOp::Release(Reference::Occupant, 2)
        ]
    );
    assert!(h.model_ops.contains(&(0, ModelOp::DestroyProcessor(31))));
    h.processor = None;
    assert_eq!(
        v.track(2, 6, 0, None, &mut h),
        Err(Unsupported::NativeProcessorAllocation)
    );
    v.shutdown(&mut h);
    assert!(v.tracks().is_empty());
    assert_eq!(v.counts, [0; 7]);
}
