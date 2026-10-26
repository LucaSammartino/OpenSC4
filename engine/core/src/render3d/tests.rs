// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Debug, Default)]
struct Host {
    events: Vec<String>,
    contexts: Vec<ContextCall>,
    controls: Vec<ControlCall>,
    frustums: Vec<(Rect, bool, bool)>,
    backing_buffer: bool,
    backing_on: bool,
}

impl ViewServices for Host {
    fn driver_depth_query(&mut self) {
        self.events.push("driver depth query".into());
    }
    fn set_camera_depth(&mut self, camera: Handle, depth: f32) {
        self.events.push(format!("depth {} {depth}", camera.0));
    }
    fn camera_derived(&self, _camera: Handle) -> i32 {
        4
    }
    fn camera_view_state(&self, _camera: Option<Handle>) -> i32 {
        7
    }
    fn update_camera_position(&mut self, control: Handle) {
        self.events.push(format!("update {}", control.0));
    }
    fn control(&mut self, _control: Option<Handle>, call: ControlCall) -> bool {
        self.controls.push(call);
        true
    }
    fn context(&mut self, call: ContextCall) {
        self.contexts.push(call);
    }
    fn build_draw_frustum(&mut self, rect: Rect, state: &State) {
        self.frustums.push((rect, state.lighting, state.fog));
    }
}

impl BackingSwitch for Host {
    fn backing_buffer_present(&self) -> bool {
        self.backing_buffer
    }
    fn backing_enabled(&self) -> bool {
        self.backing_on
    }
    fn store_backing_enabled(&mut self, enabled: bool) {
        self.backing_on = enabled;
    }
}

impl References for Host {
    fn create_helper(&mut self, kind: Helper) -> Option<Handle> {
        Some(Handle(match kind {
            Helper::ViewUtilities => 1,
            Helper::DrawContext => 2,
        }))
    }
    fn destroy_helper(&mut self, handle: Handle) {
        self.events.push(format!("destroy {}", handle.0));
    }
    fn add_ref(&mut self, handle: Handle) {
        self.events.push(format!("retain {}", handle.0));
    }
    fn release(&mut self, handle: Handle) {
        self.events.push(format!("release {}", handle.0));
    }
}

#[test]
fn identity_case_1_and_constructor_defaults() {
    let mut r = Render3d::new(&mut Host::default());
    assert_eq!(r.reference_count(), 0);
    assert_eq!(r.class_id(), CLASS_ID);
    let mut out = None;
    assert!(r.query_interface(0x4522_94AA, &mut out));
    assert_eq!(out.unwrap().offset(), 4);
    assert!(r.query_interface(0xE4FD_A3D4, &mut out));
    assert_eq!(out.unwrap().offset(), 8);
    assert!(!r.query_interface(0x1234_5678, &mut out));
    assert_eq!(out, Some(Interface::Serializable));
    assert_eq!(r.reference_count(), 2);
    let s = r.state();
    assert!(s.background_enabled && s.background_tiled && s.lighting);
    assert!(!s.fog);
    assert_eq!(s.camera_derived, -1);
    assert_eq!(s.fog_values, [1.0; 5]);
    assert_eq!(s.city_extent, [4096.0; 2]);
    assert_eq!(s.map_scale, 1);
}

#[test]
fn serialization_is_explicitly_unsupported_rnd1() {
    let mut r = Render3d::new(&mut Host::default());
    assert_eq!(r.write(), Err(Unsupported::Rnd1StreamSlot8));
    assert_eq!(r.read(&[3, 0]), Err(Unsupported::Rnd1StreamSlot8));
    assert_eq!(r.state(), &State::default());
}

#[test]
fn last_release_case_2_including_initial_zero() {
    for acquire in [false, true] {
        let mut h = Host::default();
        let mut r = Render3d::new(&mut h);
        if acquire {
            assert_eq!(r.add_ref(), 1);
        }
        assert_eq!(r.release(&mut h), 0);
        assert!(r.destroyed());
        assert_eq!(h.events, ["destroy 2", "destroy 1"]);
        assert_eq!(r.draw_context(), None);
        assert_eq!(r.view_utilities(), None);
    }
}

#[test]
fn viewport_and_forwarding_cases_3_4() {
    use crate::ui_windows::view3d::ViewChange;
    let mut h = Host::default();
    let mut r = Render3d::new(&mut h);
    r.state.viewport.left = -2;
    r.state.viewport.top = 3;
    r.set_viewport_size(800, 600);
    assert_eq!(r.viewport_size(), [800, 600]);
    assert_eq!(r.state.viewport, Rect::new(-2, 3, 800, 600));
    assert!(h.frustums.is_empty());
    r.change_view(&mut h, ViewChange::Zoom(3));
    r.change_view(&mut h, ViewChange::Rotation(3));
    assert_eq!(
        h.controls,
        [ControlCall::Zoom(3.0), ControlCall::Change(ViewChange::Rotation(3))]
    );
    r.set_camera(&mut h, Some(Handle(20)));
    r.set_camera(&mut h, Some(Handle(20)));
    r.set_camera(&mut h, Some(Handle(21)));
    assert_eq!(
        h.events,
        [
            "retain 20",
            "driver depth query",
            "depth 20 65536",
            "retain 21",
            "release 20",
            "driver depth query",
            "depth 21 65536"
        ]
    );
    assert_eq!(r.state.camera_derived, 3);
    h.events.clear();
    r.set_camera_control(&mut h, Some(Handle(30)));
    r.set_camera_control(&mut h, Some(Handle(30)));
    r.set_camera_control(&mut h, None);
    assert_eq!(h.events, ["retain 30", "update 30", "release 30"]);
    assert_eq!(r.camera(), Some(Handle(21)));
    assert_eq!(r.camera_control(), None);
}

#[test]
fn toggles_and_global_backing_cases_5_6() {
    let mut h = Host::default();
    let mut r = Render3d::new(&mut h);
    assert!(r.set_lighting_enabled(&mut h, true));
    assert!(h.frustums.is_empty());
    assert!(r.set_lighting_enabled(&mut h, false));
    assert_eq!(h.frustums[0].1, true); // Redraw precedes the lighting store.
    r.state.full_redraw = false;
    assert!(r.set_view_fog_enabled(&mut h, true));
    assert_eq!(h.frustums[1].2, true); // Fog store precedes the redraw.
    r.set_backing_store_enabled(&mut h, true);
    assert!(!r.backing_store_enabled(&h));
    h.backing_buffer = true;
    r.set_backing_store_enabled(&mut h, true);
    let other = Render3d::new(&mut h);
    assert!(other.backing_store_enabled(&h));
}

#[test]
fn borrow_restores_each_camera_mode_and_default_state() {
    for mode in 0..=3 {
        let mut h = Host::default();
        let mut r = Render3d::new(&mut h);
        r.state.camera_mode = mode;
        assert!(r.borrow_render_system());
        assert!(r.return_render_system(&mut h));
        assert!(!r.state.borrowed);
        assert_eq!(r.state.camera_mode, mode);
        assert_eq!(h.contexts.last(), Some(&ContextCall::DefaultRenderState));
        assert_eq!(h.controls, [ControlCall::ReturnRenderSystem(5)]);
        if mode == 3 {
            assert_eq!(
                &h.contexts[..5],
                &[
                    ContextCall::UiMode(true),
                    ContextCall::UiMatrix([0.0; 16]),
                    ContextCall::UiMode(false),
                    ContextCall::CommitUi,
                    ContextCall::ModelViewChanged
                ]
            );
        }
    }
}

#[test]
fn full_redraw_coalesces_case_7_but_always_writes_reason() {
    let mut h = Host::default();
    let mut r = Render3d::new(&mut h);
    r.force_full_redraw(&mut h);
    r.force_full_redraw(&mut h);
    assert_eq!(h.frustums.len(), 1);
    assert_eq!(h.contexts, [ContextCall::RedrawReason(10); 2]);
    r.do_full_view_update(&mut h, 4);
    assert_eq!(r.state.draw_reason, 4);
    assert_eq!(h.frustums.len(), 1);
}
