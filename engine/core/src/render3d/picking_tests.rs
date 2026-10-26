// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::{BTreeMap, VecDeque};

#[derive(Default)]
struct Host {
    refs: tests::Host,
    projection: container_tests::Projection,
    eye: Ray,
    eye_ok: bool,
    extended: Option<Handle>,
    hits: VecDeque<Option<[f32; 3]>>,
    calls: Vec<(TerrainTarget, Ray, u32)>,
    ground: Option<[f32; 3]>,
    ground_rays: Vec<Ray>,
    sizes: Vec<Handle>,
    visible: Vec<(WorldBox, bool)>,
    model_distances: BTreeMap<Handle, f32>,
    records: Vec<(Handle, PickRecord)>,
    reject_filter: Option<Handle>,
}

impl References for Host {
    fn create_helper(&mut self, kind: Helper) -> Option<Handle> {
        self.refs.create_helper(kind)
    }
    fn destroy_helper(&mut self, handle: Handle) {
        self.refs.destroy_helper(handle);
    }
    fn add_ref(&mut self, handle: Handle) {
        self.refs.add_ref(handle);
    }
    fn release(&mut self, handle: Handle) {
        self.refs.release(handle);
    }
}

impl ContainerServices for Host {
    fn rotation(&self) -> u32 {
        self.projection.rotation()
    }
    fn project_fixed(&mut self, point: [f32; 3]) -> [f32; 2] {
        self.projection.project_fixed(point)
    }
    fn viewport_to_fixed(&mut self, point: [f32; 2]) -> [f32; 2] {
        self.projection.viewport_to_fixed(point)
    }
    fn viewport_to_world(&mut self, point: [f32; 3]) -> [f32; 3] {
        self.projection.viewport_to_world(point)
    }
    fn utility_viewport_size(&self) -> [f32; 2] {
        self.projection.utility_viewport_size()
    }
    fn utility_draw_rect(&self) -> Rect {
        self.projection.utility_draw_rect()
    }
    fn model_world_box(&mut self, model: Handle, out: &mut WorldBox) {
        self.projection.model_world_box(model, out);
    }
    fn bind_model_grid_element(&mut self, model: Handle, present: bool) {
        self.projection.bind_model_grid_element(model, present);
    }
}

impl PickingServices for Host {
    fn bounding_box_in_frustum(&mut self, bounds: WorldBox, view: bool) -> bool {
        self.visible.push((bounds, view));
        view
    }
    fn eye_ray(&mut self, _camera: Handle, _screen: [f32; 2], out: &mut Ray) -> bool {
        *out = self.eye;
        self.eye_ok
    }
    fn query_terrain_view(&mut self, _view: Handle) -> Option<Handle> {
        self.extended
    }
    fn terrain_hit(
        &mut self,
        target: TerrainTarget,
        ray: Ray,
        context: Option<Handle>,
        out: &mut [f32; 3],
        extra: u32,
    ) -> bool {
        assert_eq!(context, Some(Handle(2)));
        self.calls.push((target, ray, extra));
        if let Some(hit) = self.hits.pop_front().flatten() {
            *out = hit;
            true
        } else {
            false
        }
    }
    fn ground_plane_hit(&mut self, ray: Ray, out: &mut [f32; 3]) -> bool {
        self.ground_rays.push(ray);
        if let Some(hit) = self.ground {
            *out = hit;
            true
        } else {
            false
        }
    }
    fn terrain_size(&mut self, terrain: Handle) -> [i32; 2] {
        self.sizes.push(terrain);
        [64, 64]
    }
    fn model_object(&mut self, model: Handle) -> Option<Handle> {
        Some(model)
    }
    fn query_filter_object(&mut self, object: Handle) -> Option<Handle> {
        Some(Handle(object.0 + 100))
    }
    fn filter_accepts(&mut self, _filter: Handle, object: Handle) -> bool {
        Some(object) != self.reject_filter
    }
    fn model_hit_test(&mut self, model: Handle, record: &mut PickRecord) -> bool {
        self.records.push((model, *record));
        if let Some(&distance) = self.model_distances.get(&model) {
            if distance <= record.max_distance {
                record.max_distance = distance;
                return true;
            }
        }
        false
    }
}

fn ray() -> Ray {
    Ray {
        origin: [10.0, 100.0, 10.0],
        direction: [0.0, -1.0, 0.0],
    }
}

fn renderer(host: &mut Host) -> Render3d {
    let mut r = Render3d::new(host);
    r.camera = Some(Handle(10));
    r.objects.terrain_view = Some(Handle(20));
    r.objects.terrain = Some(Handle(30));
    r
}

#[test]
fn view_objects_p1_unsigned_stable_keys_p2_invalid_type_and_backing_invalidation() {
    let mut host = tests::Host::default();
    let mut r = Render3d::new(&mut host);
    for (object, key) in [(1, 5), (2, 2), (3, 5), (4, u32::MAX)] {
        assert!(r.add_view_object(&mut host, Handle(object), 3, key));
    }
    assert_eq!(
        r.view_objects(3)
            .unwrap()
            .iter()
            .map(|n| n.object.0)
            .collect::<Vec<_>>(),
        [2, 1, 3, 4]
    );
    r.state.backing_saved = true;
    let events = host.events.clone();
    assert!(!r.add_view_object(&mut host, Handle(1), 1, 0));
    assert_eq!(host.events, events);
    assert!(r.state.backing_saved);
    assert!(r.add_view_object(&mut host, Handle(1), 0, 0));
    assert!(!r.state.backing_saved);
}

#[test]
fn view_objects_p3_remove_first_per_list_duplicates_and_null_terrain_quirk() {
    let mut host = tests::Host::default();
    let mut r = Render3d::new(&mut host);
    assert!(r.find_view_object(None));
    assert!(r.remove_view_object(&mut host, None));
    for kind in [3, 3, 0] {
        r.add_view_object(&mut host, Handle(1), kind, 0);
    }
    r.post_city_init_terrain_view(&mut host, Some(Handle(1)));
    r.post_city_init_terrain_view(&mut host, Some(Handle(99)));
    host.events.clear();
    r.state.backing_saved = true;
    assert!(r.remove_view_object(&mut host, Some(Handle(1))));
    assert_eq!(host.events, ["release 1", "release 1", "release 1"]);
    assert!(!r.state.backing_saved);
    assert_eq!(r.view_objects(3).unwrap().len(), 1);
    assert!(r.find_view_object(Some(Handle(1))));
    assert!(r.remove_view_object(&mut host, Some(Handle(1))));
    assert!(!r.remove_view_object(&mut host, Some(Handle(1))));
}

#[test]
fn visibility_uses_current_model_box_and_only_kind_two_selects_view_frustum() {
    let mut host = Host::default();
    let r = renderer(&mut host);
    host.projection.bounds = WorldBox {
        min: [2.0; 3],
        max: [4.0; 3],
    };
    assert!(r.is_model_visible(&mut host, Handle(1), 2));
    assert!(!r.is_grid_element_visible(&mut host, &GridElement::new(Handle(1)), 1));
    assert_eq!(
        host.visible,
        [(host.projection.bounds, true), (host.projection.bounds, false)]
    );
}

#[test]
fn terrain_p4_retry_raise_and_original_ray_fallback_release_extended_interface() {
    let mut host = Host {
        extended: Some(Handle(21)),
        ..Host::default()
    };
    let r = renderer(&mut host);
    host.hits.extend([None, Some([10.0, 90.0, 10.0])]);
    let mut out = [0.0; 3];
    assert!(r.pick_terrain_ray(&mut host, ray(), &mut out, false, 7));
    assert_eq!(
        host.calls.iter().map(|c| c.1.origin[1]).collect::<Vec<_>>(),
        [100.0, 104.0]
    );
    assert!(host
        .calls
        .iter()
        .all(|c| c.0 == TerrainTarget::Extended(Handle(21)) && c.2 == 7));
    assert_eq!(host.refs.events, ["release 21"]);
    host.calls.clear();
    host.extended = None;
    assert!(!r.pick_terrain_ray(&mut host, ray(), &mut out, false, 7));
    assert!(host.ground_rays.is_empty());
    host.ground = Some([10.0, 0.0, 10.0]);
    assert!(r.pick_terrain_ray(&mut host, ray(), &mut out, true, 7));
    assert_eq!(host.ground_rays, [ray()]);
    assert!(host
        .calls
        .iter()
        .all(|c| c.0 == TerrainTarget::Base(Handle(20)) && c.2 == 0));
}

#[test]
fn picking_p5_terrain_only_occludes_and_p6_screen_terrain_ray_normalizes() {
    let mut host = Host {
        eye: Ray {
            direction: [0.0, -3.0, 4.0],
            ..ray()
        },
        eye_ok: true,
        ..Host::default()
    };
    let mut r = renderer(&mut host);
    host.hits.push_back(Some([10.0, 90.0, 10.0]));
    let mut model = Some(Handle(99));
    assert!(!r.pick_ray(&mut host, [0, 0], ray(), None, &mut model));
    assert_eq!(model, None);
    let mut out = [0.0; 3];
    host.hits.push_back(Some([10.0, 90.0, 10.0]));
    assert!(r.pick_terrain(&mut host, [0, 0], &mut out, false, 0));
    assert_eq!(host.calls.last().unwrap().1.direction, [0.0, -0.6, 0.8]);
    host.eye_ok = false;
    let count = host.calls.len();
    assert!(!r.pick_terrain(&mut host, [0, 0], &mut out, false, 0));
    assert_eq!(host.calls.len(), count);
    assert!(!r.pick(&mut host, [0, 0], None, &mut model));
    assert!(host.calls.len() > count);
}

#[test]
fn model_pick_deduplicates_uses_mutable_distance_releases_previous_and_filter_faces() {
    let mut host = Host::default();
    let mut r = renderer(&mut host);
    let mut grid = QuadGrid::for_map_scale(1);
    for id in 1..=3 {
        grid.add_element(
            GridElement::new(Handle(id)),
            ContainerRect::new(90.0, 90.0, 110.0, 110.0),
            1,
        );
    }
    r.grid = Some(grid);
    host.projection.fixed_results.push_back([100.0, -100.0]);
    host.hits.push_back(Some([10.0, 90.0, 10.0]));
    host.model_distances = [(Handle(1), 9.0), (Handle(2), 5.0), (Handle(3), 7.0)].into();
    let mut out = None;
    assert!(r.pick_ray(&mut host, [1, 2], ray(), Some(Handle(99)), &mut out));
    assert_eq!(out, Some(Handle(2)));
    assert_eq!(
        host.records.iter().map(|r| r.1.max_distance).collect::<Vec<_>>(),
        [10.0, 9.0, 5.0]
    );
    assert_eq!(
        host.refs.events,
        [
            "release 101",
            "retain 1",
            "release 102",
            "release 1",
            "retain 2",
            "release 103"
        ]
    );
    for id in 1..=3 {
        assert_eq!(r.grid.as_ref().unwrap().element(Handle(id)).unwrap().flags & 2, 0);
    }
    host.records.clear();
    host.projection.fixed_results.push_back([100.0, -100.0]);
    host.reject_filter = Some(Handle(102));
    r.pick_ray(&mut host, [1, 2], ray(), Some(Handle(99)), &mut out);
    assert_eq!(
        host.records.iter().map(|r| r.0).collect::<Vec<_>>(),
        [Handle(1), Handle(3)]
    );
    assert_eq!(r.grid.as_ref().unwrap().element(Handle(2)).unwrap().flags & 2, 0);
}

#[test]
fn terrain_always_worked_polyline_and_final_fallback_slide_onto_original_ray() {
    let mut host = Host::default();
    let mut r = renderer(&mut host);
    let ray = Ray {
        origin: [10.0, 50.0, 10.0],
        direction: [-1.0, -1.0, 0.0],
    };
    let points = picking::terrain_polyline(ray, [64, 64]);
    assert_eq!(points.len(), 3);
    assert!((points[1][0] - 0.01).abs() < 0.0001);
    assert!((points[1][1] - 40.01).abs() < 0.0001);
    assert!((points[2][0] - 0.01).abs() < 0.0001);
    assert_eq!(points[2][1], 0.0);
    host.ground = Some([0.01, 0.0, 10.0]);
    let mut out = [0.0; 3];
    assert!(r.pick_terrain_always_ray(&mut host, ray, &mut out, 4));
    assert_eq!(host.calls.len(), 6);
    assert_eq!(host.ground_rays.len(), 1);
    assert_eq!(out, [-40.0, 0.0, 10.0]);
    assert_eq!(host.sizes, [Handle(30)]);
    r.objects.terrain = None;
    let calls = host.calls.len();
    assert!(!r.pick_terrain_always_ray(&mut host, ray, &mut out, 4));
    assert_eq!(host.calls.len(), calls);
}

#[test]
fn terrain_always_rejects_hit_past_intermediate_end_and_accepts_final_hit() {
    let mut host = Host::default();
    let r = renderer(&mut host);
    let ray = Ray {
        origin: [10.0, 50.0, 10.0],
        direction: [-1.0, -1.0, 0.0],
    };
    host.hits
        .extend([None, None, Some([0.01, 20.0, 10.0]), Some([0.01, 10.0, 10.0])]);
    let mut out = [0.0; 3];
    assert!(r.pick_terrain_always_ray(&mut host, ray, &mut out, 0));
    assert_eq!(host.calls.len(), 4);
    assert_eq!(out, [-30.0, 10.0, 10.0]);
}

#[test]
fn terrain_polyline_outside_start_projects_but_crosses_from_original_point() {
    let ray = Ray {
        origin: [-10.0, 10.0, 10.0],
        direction: [2.0, -1.0, 0.0],
    };
    let points = picking::terrain_polyline(ray, [64, 64]);
    assert_eq!(points.len(), 3);
    assert!((points[0][0] - 0.01).abs() < 0.0001);
    assert_eq!(points[0][1], 10.0);
    assert!((points[1][0] - 0.01).abs() < 0.0001);
    assert!((points[1][1] - 4.995).abs() < 0.0001);
    assert_eq!(points[2], [10.0, 0.0, 10.0, 1.0]);
}

#[test]
fn terrain_always_direction_guard_after_first_attempt_and_screen_camera_guards() {
    let mut host = Host::default();
    let mut r = renderer(&mut host);
    let mut up = ray();
    up.direction[1] = 1.0;
    let mut out = [99.0; 3];
    assert!(!r.pick_terrain_always_ray(&mut host, up, &mut out, 0));
    assert_eq!(host.calls.len(), 2);
    assert!(host.sizes.is_empty());
    host.hits.push_back(Some([1.0; 3]));
    assert!(r.pick_terrain_always_ray(&mut host, up, &mut out, 0));
    assert_eq!(out, [1.0; 3]);
    r.camera = None;
    let calls = host.calls.len();
    assert!(!r.pick_terrain(&mut host, [0; 2], &mut out, true, 0));
    assert!(!r.pick_terrain_always(&mut host, [0; 2], &mut out, 0));
    let mut model = Some(Handle(99));
    assert!(!r.pick(&mut host, [0; 2], None, &mut model));
    assert_eq!(model, Some(Handle(99)));
    assert_eq!(host.calls.len(), calls);
}
