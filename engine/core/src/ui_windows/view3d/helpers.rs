// SPDX-License-Identifier: GPL-3.0-or-later
//! Owner decisions for the foreign helpers in view3d-window §§6f, 6l.
use super::{View3DWin, ViewChange};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackgroundPicture {
    pub token: u32,
    pub width: u32,
    pub height: u32,
}

pub trait HelperHost {
    fn background_mode(&self) -> bool;
    /// Preserve the foreign directory enumeration order. No sorting by owner.
    fn background_files(&mut self, three_d: bool) -> Vec<String>;
    fn background_candidates(&mut self, three_d: bool, _basename: &str) -> Vec<String> {
        self.background_files(three_d)
    }
    fn background_zoom(&self) -> u8 {
        0
    }
    /// One owned reference on successful load, released even for invalid sizes.
    fn load_background(&mut self, path: &str) -> Option<BackgroundPicture>;
    /// Renderer retains its own reference; the view releases its load reference.
    fn renderer_background(&mut self, picture: u32, three_d: bool);
    fn release_picture(&mut self, picture: u32);
    fn coverage_clear(&mut self);
    fn coverage_effect(&mut self, effect: u32, radius: u32);
    fn stop_helper(&mut self, helper: u32) -> bool;
    fn helper_stopped(&mut self, helper: u32, value: u32);
    fn renderer_pick(&mut self, occupant: bool, x: i32, y: i32, flags: u32) -> Option<[f32; 3]>;
    fn acquire_occupant_interface(&mut self, iid: u32) -> bool;
    fn release_occupant_interface(&mut self);
}

pub trait GraphHost {
    fn graph_resource(&mut self, instance: u32, group: u32) -> Option<u32>;
    fn graph_reset(&mut self, resource: u32, index: i32, value: u32);
    fn graph_prepare(&mut self, resource: u32);
    fn graph_series(&mut self, resource: u32, series: u32, colour: u32);
    fn graph_release(&mut self, resource: u32);
}

impl View3DWin {
    pub fn setup_graph(&self, host: &mut impl GraphHost) {
        if let Some(resource) = host.graph_resource(0x4203, 0xCA31_8385) {
            host.graph_reset(resource, -1, 0);
            host.graph_prepare(resource);
            for (index, colour) in [
                0x00FF00, 0x44CC22, 0x1E911E, 0x5B81D0, 0x3D6EB8, 0x3C42AD, 0xD0D030, 0xC6B733, 0xBC9F37,
            ]
            .into_iter()
            .enumerate()
            {
                host.graph_series(resource, index as u32 + 1, colour);
            }
            host.graph_release(resource);
        }
    }
    pub fn add_background_image(&mut self, host: &mut impl HelperHost, three_d: bool) -> bool {
        if !self.owner.lifecycle.objects.contains_key(&0x150) || !self.owner.lifecycle.objects.contains_key(&0xF4) {
            return false;
        }
        let basename = if three_d {
            format!("Background3D{}", char::from(b'0'.wrapping_add(host.background_zoom())))
        } else {
            "Background2D".into()
        };
        for path in host.background_candidates(three_d, &basename) {
            if let Some(picture) = host.load_background(&path) {
                let valid = picture.width == picture.height && picture.width & picture.width.wrapping_sub(1) == 0;
                if valid {
                    host.renderer_background(picture.token, three_d);
                    self.owner.background_mode = if three_d { 2 } else { 1 };
                }
                host.release_picture(picture.token);
                if valid {
                    return true;
                }
            }
        }
        false
    }

    pub fn refresh_background(&mut self, host: &mut impl HelperHost) -> bool {
        match self.owner.background_mode {
            0 => self.add_background_image(host, false) || self.add_background_image(host, true),
            1 => self.add_background_image(host, false),
            2 => self.add_background_image(host, true),
            _ => false,
        }
    }

    /// Slot 38 is an empty two-argument void method (§6l).
    pub fn display_terrain_pick_debug_string(&self, _a: u32, _b: u32) {}

    pub fn stop_foreign_helper(&self, host: &mut impl HelperHost, helper: u32) {
        if host.stop_helper(helper) {
            host.helper_stopped(helper, 0);
        }
    }

    pub fn stop_auxiliary(&self, host: &mut impl HelperHost, helper: u32, active: &mut bool) -> bool {
        if *active {
            self.stop_foreign_helper(host, helper);
            *active = false;
        }
        true
    }

    pub fn pick_terrain(&self, host: &mut impl HelperHost, x: i32, y: i32, flags: u32, point: &mut [f32; 3]) -> bool {
        *point = [0.0; 3];
        if self.owner.lifecycle.objects.contains_key(&0xF4) {
            if let Some(hit) = host.renderer_pick(false, x, y, flags) {
                *point = hit;
                return true;
            }
        }
        false
    }

    pub fn pick_occupant(&self, host: &mut impl HelperHost, x: i32, y: i32, point: &mut [f32; 3]) -> bool {
        *point = [0.0; 3];
        if self.owner.lifecycle.objects.contains_key(&0xF4) && host.acquire_occupant_interface(0x26BB_767E) {
            let hit = host.renderer_pick(true, x, y, 0);
            host.release_occupant_interface();
            if let Some(hit) = hit {
                *point = hit;
                return true;
            }
        }
        false
    }

    pub fn set_zoom_and_rotation(&mut self, host: &mut impl super::MotionHost, zoom: i32, rotation: i32) -> bool {
        self.change_view(host, ViewChange::ZoomAndRotation(zoom, rotation))
    }
}
