// SPDX-License-Identifier: GPL-3.0-or-later
//! Camera/control forwarding (§3) and full-redraw coalescing (§4).
use super::{Handle, Rect, References, Render3d, State};
use crate::ui_windows::view3d::ViewChange;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlCall {
    Zoom(f32),
    Change(ViewChange),
    ReturnRenderSystem(i32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ContextCall {
    Camera(Option<Handle>),
    UiMode(bool),
    UiMatrix([f32; 16]),
    CommitUi,
    ModelViewChanged,
    DefaultRenderState,
    RedrawReason(i32),
}

/// Camera, driver and utility operations, rather than owner callbacks.
pub trait ViewServices: References {
    fn driver_depth_query(&mut self);
    fn set_camera_depth(&mut self, camera: Handle, depth: f32);
    fn camera_derived(&self, camera: Handle) -> i32;
    fn camera_view_state(&self, camera: Option<Handle>) -> i32;
    fn update_camera_position(&mut self, control: Handle);
    fn control(&mut self, control: Option<Handle>, call: ControlCall) -> bool;
    fn context(&mut self, call: ContextCall);
    fn build_draw_frustum(&mut self, rect: Rect, state: &State);
}

/// This is process state supplied by the host, shared by all renderer owners.
pub trait BackingSwitch {
    fn backing_buffer_present(&self) -> bool;
    fn backing_enabled(&self) -> bool;
    fn store_backing_enabled(&mut self, enabled: bool);
}

impl Render3d {
    pub fn set_map_scale_factor(&mut self, scale: i32) {
        self.state.map_scale = scale;
    }
    pub fn viewport_size(&self) -> [i32; 2] {
        self.state.viewport_size
    }
    pub fn set_viewport_size(&mut self, width: i32, height: i32) {
        self.state.viewport_size = [width, height];
        self.state.viewport.right = width;
        self.state.viewport.bottom = height;
    }
    pub fn set_subpixel_image_bias(&mut self, x: f32, y: f32) {
        self.state.subpixel_bias = [x, y];
    }
    pub fn camera(&self) -> Option<Handle> {
        self.camera
    }
    pub fn camera_control(&self) -> Option<Handle> {
        self.control
    }
    pub fn lighting_manager(&self) -> Option<Handle> {
        self.lighting_manager
    }

    pub fn set_camera(&mut self, host: &mut impl ViewServices, camera: Option<Handle>) {
        if camera == self.camera {
            return;
        }
        if let Some(new) = camera {
            host.add_ref(new);
        }
        if let Some(old) = self.camera {
            host.release(old);
        }
        self.camera = camera;
        if let Some(new) = camera {
            host.driver_depth_query();
            host.set_camera_depth(new, 65536.0);
            self.state.camera_derived = host.camera_derived(new).wrapping_sub(1);
        }
    }

    pub fn set_camera_control(&mut self, host: &mut impl ViewServices, control: Option<Handle>) {
        if control == self.control {
            return;
        }
        if let Some(new) = control {
            host.add_ref(new);
        }
        if let Some(old) = self.control {
            host.release(old);
        }
        self.control = control;
        if let Some(new) = control {
            host.update_camera_position(new);
        }
    }

    /// Reuses the window's existing command boundary, including integer rotation.
    pub fn change_view(&mut self, host: &mut impl ViewServices, change: ViewChange) -> bool {
        let call = match change {
            ViewChange::Zoom(value) => ControlCall::Zoom(value as f32),
            other => ControlCall::Change(other),
        };
        host.control(self.control, call)
    }

    pub fn set_3d_camera_view(&mut self, host: &mut impl ViewServices) {
        if self.state.camera_mode == 2 {
            return;
        }
        host.context(ContextCall::Camera(self.camera));
        self.state.camera_mode = 2;
        self.state.camera_view = host.camera_view_state(self.camera);
    }

    pub fn set_2d_camera_view(&mut self, host: &mut impl ViewServices) {
        if self.state.camera_mode == 1 {
            return;
        }
        host.context(ContextCall::Camera(self.camera_2d));
        self.state.camera_mode = 1;
    }

    pub fn set_ui_camera_view(&mut self, host: &mut impl ViewServices) {
        if self.state.camera_mode == 3 {
            return;
        }
        host.context(ContextCall::UiMode(true));
        host.context(ContextCall::UiMatrix(self.ui_matrix));
        host.context(ContextCall::UiMode(false));
        host.context(ContextCall::CommitUi);
        host.context(ContextCall::ModelViewChanged);
        self.state.camera_mode = 3;
    }

    pub fn borrow_render_system(&mut self) -> bool {
        self.state.borrowed = true;
        true
    }
    pub fn return_render_system(&mut self, host: &mut impl ViewServices) -> bool {
        self.state.borrowed = false;
        host.control(self.control, ControlCall::ReturnRenderSystem(5));
        let mode = self.state.camera_mode;
        self.state.camera_mode = 0;
        match mode {
            1 => self.set_2d_camera_view(host),
            2 => self.set_3d_camera_view(host),
            3 => self.set_ui_camera_view(host),
            _ => (),
        }
        host.context(ContextCall::DefaultRenderState);
        true
    }

    pub fn shake_offsets(&self) -> [f32; 2] {
        self.state.shake_offsets.map(|v| v as f32)
    }
    pub fn lighting_enabled(&self) -> bool {
        self.state.lighting
    }
    pub fn view_fog_enabled(&self) -> bool {
        self.state.fog
    }
    pub fn set_view_fog_enabled(&mut self, host: &mut impl ViewServices, enabled: bool) -> bool {
        if self.state.fog != enabled {
            self.state.fog = enabled;
            self.force_full_redraw(host);
        }
        true
    }
    pub fn set_lighting_enabled(&mut self, host: &mut impl ViewServices, enabled: bool) -> bool {
        if self.state.lighting != enabled {
            self.force_full_redraw(host);
            self.state.lighting = enabled;
        }
        true
    }
    pub fn set_viewport_clear_color(&mut self, rgba: [f32; 4]) {
        self.state.clear_color = rgba;
        self.state.clear_color_pushed = false;
    }
    pub fn enable_background_picture(&mut self, enabled: bool) {
        self.state.background_enabled = enabled;
    }
    pub fn background_picture(&self, tiled: Option<&mut bool>) -> Option<Handle> {
        if let Some(out) = tiled {
            *out = self.state.background_tiled;
        }
        self.picture
    }
    pub fn enable_transparent_background(&mut self, enabled: bool) {
        self.state.transparent = enabled;
    }
    pub fn backing_store_enabled(&self, globals: &impl BackingSwitch) -> bool {
        globals.backing_enabled()
    }
    pub fn set_backing_store_enabled(&mut self, globals: &mut impl BackingSwitch, enabled: bool) {
        let enabled = enabled && globals.backing_buffer_present();
        if globals.backing_enabled() != enabled {
            globals.store_backing_enabled(enabled);
        }
    }
    pub fn force_full_redraw(&mut self, host: &mut impl ViewServices) {
        self.do_full_view_update(host, 10);
    }
    pub fn do_full_view_update(&mut self, host: &mut impl ViewServices, reason: i32) {
        self.state.draw_reason = reason;
        host.context(ContextCall::RedrawReason(reason));
        if self.state.full_redraw {
            return;
        }
        self.state.full_redraw = true;
        self.state.backing_saved = false;
        host.build_draw_frustum(self.state.viewport, &self.state);
        self.state.frustum_valid = false;
        self.state.visible_valid = false;
        self.clear_dirty_rectangle_queue(host);
    }
}
