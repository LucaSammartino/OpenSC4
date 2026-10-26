// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Default)]
pub(super) struct Projection {
    pub(super) rotation: u32,
    pub(super) points: Vec<[f32; 3]>,
    pub(super) viewport_points: Vec<[f32; 2]>,
    pub(super) bindings: Vec<(Handle, bool)>,
    pub(super) bounds: WorldBox,
    pub(super) fixed_results: std::collections::VecDeque<[f32; 2]>,
}

impl ContainerServices for Projection {
    fn rotation(&self) -> u32 {
        self.rotation
    }
    fn project_fixed(&mut self, point: [f32; 3]) -> [f32; 2] {
        self.points.push(point);
        self.fixed_results.pop_front().unwrap_or([point[0], -point[2]])
    }
    fn viewport_to_fixed(&mut self, point: [f32; 2]) -> [f32; 2] {
        self.viewport_points.push(point);
        [point[0], -point[1]]
    }
    fn viewport_to_world(&mut self, point: [f32; 3]) -> [f32; 3] {
        point
    }
    fn utility_viewport_size(&self) -> [f32; 2] {
        [10_000.0, 9000.0]
    }
    fn utility_draw_rect(&self) -> Rect {
        Rect::new(-10, -20, 9000, 9001)
    }
    fn model_world_box(&mut self, _model: Handle, out: &mut WorldBox) {
        *out = self.bounds;
    }
    fn bind_model_grid_element(&mut self, model: Handle, present: bool) {
        self.bindings.push((model, present));
    }
}

pub(super) fn renderer() -> Render3d {
    let mut r = Render3d::new(&mut tests::Host::default());
    r.grid = Some(QuadGrid::for_map_scale(1));
    r
}

#[test]
fn container_world_box_rotations_padding_and_margins() {
    let mut r = renderer();
    r.set_map_scale_factor(2);
    r.initialize_container_margins();
    assert_eq!(r.container_margins()[0], [6000.0, 2800.0, 10_400.0, 13_600.0]);
    assert_eq!(r.container_margins()[1], [9000.0, 12_800.0, 14_400.0, 10_600.0]);
    r.geometry = ContainerGeometry::default();
    let bounds = WorldBox {
        min: [100.0, 10.0, 200.0],
        max: [140.0, 30.0, 280.0],
    };
    let expected = [
        ([71.71573, 30.0, 211.71573], [168.28427, 10.0, 268.28427]),
        ([134.14214, 30.0, 185.85786], [105.857864, 10.0, 294.14215]),
        ([168.28427, 30.0, 268.28427], [71.71573, 10.0, 211.71573]),
        ([105.857864, 30.0, 294.14215], [134.14214, 10.0, 185.85786]),
    ];
    for (rotation, (first, second)) in expected.into_iter().enumerate() {
        let mut host = Projection::default();
        let rect = r.world_to_container_rect(&mut host, bounds, rotation as u32);
        for (actual, expected) in host.points.iter().flatten().zip([first, second].iter().flatten()) {
            assert!((actual - expected).abs() < 0.0001);
        }
        assert!(rect.right > rect.left && rect.bottom > rect.top);
        if rotation == 0 {
            assert!((rect.left - 69.71573).abs() < 0.0001);
            assert!((rect.bottom - 270.28427).abs() < 0.0001);
        }
    }
}

#[test]
fn container_clamps_in_order_and_turns_half_grid_span_into_sliver() {
    let r = renderer();
    let mut host = Projection::default();
    host.fixed_results.extend([[-10.0, 20.0], [9000.0, -9000.0]]);
    let rect = r.world_to_container_rect(&mut host, WorldBox::default(), 0);
    assert_eq!(rect, ContainerRect::new(0.5, 0.5, 1.5, 1.5));
    host.fixed_results.extend([[100.0, -200.0], [50.0, -100.0]]);
    let rect = r.world_to_container_rect(&mut host, WorldBox::default(), 0);
    assert_eq!(rect, ContainerRect::new(51.9, 101.9, 52.0, 102.0));
}

#[test]
fn container_frustum_caches_separate_draw_and_view_until_invalidation() {
    let mut r = renderer();
    let mut host = Projection::default();
    assert_eq!(
        r.view_frustum_container_rect(&mut host),
        ContainerRect::new(0.0, 0.0, 10_000.0, 9000.0)
    );
    host.viewport_points.clear();
    let draw = r.visible_container_rect(&mut host, 0);
    assert_eq!(draw, ContainerRect::new(0.0, 0.0, 8191.0, 8191.0));
    assert_eq!(r.visible_container_rect(&mut host, 1), draw);
    assert_eq!(host.viewport_points.len(), 2);
    assert_eq!(r.visible_container_rect(&mut host, 2), draw);
    assert_eq!(host.viewport_points.len(), 4);
    r.force_full_redraw(&mut tests::Host::default());
    r.visible_container_rect(&mut host, 0);
    r.visible_container_rect(&mut host, 2);
    assert_eq!(host.viewport_points.len(), 8);
}

#[test]
fn container_ray_clamps_left_without_clamping_large_right() {
    let r = renderer();
    let mut host = Projection::default();
    host.fixed_results.push_back([-10.0, 20.0]);
    assert_eq!(
        r.viewport_ray_container_rect(&mut host, 17, 29),
        ContainerRect::new(0.0, 0.0, 1.0, 1.0)
    );
    host.fixed_results.push_back([9000.0, -10_000.0]);
    assert_eq!(
        r.viewport_ray_container_rect(&mut host, 0, 0),
        ContainerRect::new(8190.0, 8190.0, 9002.0, 10_002.0)
    );
}

#[test]
fn container_move_updates_world_box_marks_only_eligible_quadrants_and_clears_failure() {
    for (flags, expected) in [(0, 1), (8, 0), (0x88, 1)] {
        let mut r = renderer();
        let mut element = GridElement::new(Handle(1));
        element.flags = flags;
        r.grid
            .as_mut()
            .unwrap()
            .add_element(element, ContainerRect::new(1.0, 1.0, 2.0, 2.0), 0);
        let mut host = Projection {
            bounds: WorldBox {
                min: [100.0; 3],
                max: [110.0; 3],
            },
            ..Projection::default()
        };
        assert!(r.move_grid_element(&mut host, Handle(1)));
        let grid = r.grid.as_ref().unwrap();
        assert_eq!(grid.element(Handle(1)).unwrap().bounds, host.bounds);
        let cells = grid.element(Handle(1)).unwrap().cells;
        assert_eq!(grid.quadrant(cells.left, cells.top).0, expected);
        assert!(host.bindings.is_empty());
        assert!(!r.move_grid_element(&mut host, Handle(99)));
        assert_eq!(host.bindings, [(Handle(99), false)]);
    }
}

#[test]
fn container_rebuild_guard_and_new_binding_preserve_flags_bounds_and_model_reference() {
    let mut r = renderer();
    let mut host = Projection {
        bounds: WorldBox {
            min: [100.0; 3],
            max: [110.0; 3],
        },
        ..Projection::default()
    };
    let mut element = GridElement::new(Handle(1));
    element.flags = 0x88;
    let original_box = element.bounds;
    r.grid
        .as_mut()
        .unwrap()
        .add_element(element, ContainerRect::new(1.0, 1.0, 2.0, 2.0), 0);
    r.rebuild_quad_grid(&mut host, 1, 0, 1, 0);
    assert!(host.bindings.is_empty());
    r.rebuild_quad_grid(&mut host, 1, 0, 2, 0);
    assert_eq!(host.bindings, [(Handle(1), true)]);
    let grid = r.grid.as_ref().unwrap();
    let element = grid.element(Handle(1)).unwrap();
    assert_eq!(element.flags, 0x88);
    assert_eq!(element.bounds, original_box);
    assert!(element.reference_count() > 0);
    assert_eq!(grid.element_count(), 1);
}
