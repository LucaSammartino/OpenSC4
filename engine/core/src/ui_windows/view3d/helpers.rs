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

impl View3DWin {
    pub fn add_background_image(&mut self, host: &mut impl HelperHost, three_d: bool) -> bool {
        if !self.owner.lifecycle.objects.contains_key(&0x150)
            || !self.owner.lifecycle.objects.contains_key(&0xF4)
        {
            return false;
        }
        for path in host.background_files(three_d) {
            if let Some(picture) = host.load_background(&path) {
                let valid = picture.width == picture.height && picture.width.is_power_of_two();
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

    /// The helper owns the spatial set; the owner specifies effect identities
    /// and radius. Placement/recalculation remain foreign helper operations.
    pub fn show_coverage(&self, host: &mut impl HelperHost) {
        host.coverage_clear();
        for effect in [0x3AE4_BEA3, 0x3AE4_BEAB] {
            host.coverage_effect(effect, 32);
        }
    }

    pub fn stop_foreign_helper(&self, host: &mut impl HelperHost, helper: u32) {
        if host.stop_helper(helper) {
            host.helper_stopped(helper, 0);
        }
    }

    pub fn pick_terrain(
        &self,
        host: &mut impl HelperHost,
        x: i32,
        y: i32,
        flags: u32,
        point: &mut [f32; 3],
    ) -> bool {
        *point = [0.0; 3];
        if self.owner.lifecycle.objects.contains_key(&0xF4) {
            if let Some(hit) = host.renderer_pick(false, x, y, flags) {
                *point = hit;
                return true;
            }
        }
        false
    }

    pub fn pick_occupant(
        &self,
        host: &mut impl HelperHost,
        x: i32,
        y: i32,
        point: &mut [f32; 3],
    ) -> bool {
        *point = [0.0; 3];
        if self.owner.lifecycle.objects.contains_key(&0xF4)
            && host.acquire_occupant_interface(0x26BB_767E)
        {
            let hit = host.renderer_pick(true, x, y, 0);
            host.release_occupant_interface();
            if let Some(hit) = hit {
                *point = hit;
                return true;
            }
        }
        false
    }

    pub fn set_zoom_and_rotation(
        &mut self,
        host: &mut impl super::MotionHost,
        zoom: i32,
        rotation: i32,
    ) -> bool {
        self.change_view(host, ViewChange::ZoomAndRotation(zoom, rotation))
    }
}
