// SPDX-License-Identifier: GPL-3.0-or-later
//! Invocation-input replay for the View3D shadow (ui-windows §3, view3d-window §6f).
//! Native post-state never enters this adapter. This is not a persistent mirror.

use super::{MotionHost, Notification, View3DWin, ViewChange};
use std::cell::Cell;

pub const WORDS: usize = 22;

struct Host {
    zoom: i32,
    reads: Cell<u32>,
    unexpected: Cell<bool>,
}

impl Host {
    fn unexpected(&self) {
        self.unexpected.set(true);
    }
}

impl MotionHost for Host {
    fn virtual_zoom(&self) -> i32 {
        self.reads.set(self.reads.get() + 1);
        self.zoom
    }
    fn camera_zoom(&self) -> f32 {
        self.unexpected();
        0.0
    }
    fn view_change(&mut self, _: ViewChange) -> bool {
        self.unexpected();
        false
    }
    fn scroll_camera(&mut self, _: f32, _: f32) -> bool {
        self.unexpected();
        false
    }
    fn post_scroll_message(&mut self, _: u32) {
        self.unexpected();
    }
    fn begin_scroll_render_settings(&mut self) {
        self.unexpected();
    }
    fn end_scroll_render_settings(&mut self) {
        self.unexpected();
    }
    fn simulator_present(&self) -> bool {
        self.unexpected();
        false
    }
    fn hidden_pause(&mut self) {
        self.unexpected();
    }
    fn hidden_resume(&mut self) {
        self.unexpected();
    }
    fn restart_hidden_timer(&mut self) {
        self.unexpected();
    }
    fn reset_paint_timer(&mut self) {
        self.unexpected();
    }
    fn animation_time(&mut self) -> f32 {
        self.unexpected();
        0.0
    }
    fn set_draw_context_time(&mut self, _: f32) {
        self.unexpected();
    }
    fn draw(&mut self) {
        self.unexpected();
    }
    fn update_cost_indicator(&mut self, _: f32) {
        self.unexpected();
    }
    fn paint_tail(&mut self) {
        self.unexpected();
    }
    fn deferred_safe(&self) -> bool {
        self.unexpected();
        false
    }
    fn deferred_enabled(&self) -> bool {
        self.unexpected();
        false
    }
    fn deferred_reference(&mut self, _: &Notification, _: bool) {
        self.unexpected();
    }
    fn repost_deferred(&mut self, _: &Notification, _: u32) {
        self.unexpected();
    }
}

/// Returns (predicted return, virtual-zoom call count, output packet).
/// Packet layout is private to the static-link shadow transport.
pub fn replay(slot: u32, input: &[u32; WORDS]) -> Option<(u32, u32, [u32; WORDS])> {
    let mut view = View3DWin::new(input[0], u64::from(input[0].wrapping_add(12)));
    for (offset, value) in [(0xF4, input[1]), (0xF8, input[2]), (0x120, input[3])] {
        if value != 0 {
            view.owner.lifecycle.objects.insert(offset, value);
        }
    }
    let state = view.view_state_mut();
    state.level = input[4];
    state.terrain_query = input[5] != 0;
    state.last_building = (input[7] != 0).then_some(input[6]);
    state.last_lot = (input[9] != 0).then_some(input[8]);
    let motion = view.motion_mut();
    motion.scrolling = input[10] != 0;
    motion.speed = f32::from_bits(input[11]);
    motion.stop_step = f32::from_bits(input[12]);
    motion.proportional_stop = input[13] != 0;
    motion.stopping = input[14] != 0;
    motion.active_step = f32::from_bits(input[15]);
    motion.zoom_min = input[16] as i32;
    motion.zoom_max = input[17] as i32;
    let host = Host {
        zoom: input[18] as i32,
        reads: Cell::new(0),
        unexpected: Cell::new(false),
    };
    let mut output = *input;
    let value = match slot {
        3 => View3DWin::as_igzwin_addr(input[0]),
        4 => view.renderer().unwrap_or(0),
        5 => view.model_maker().unwrap_or(0),
        12 => u32::from(view.can_zoom_in(&host)),
        13 => u32::from(view.can_zoom_out(&host)),
        20 | 22 => {
            let mut level = 0;
            view.get_view_level_info(&mut level);
            level
        }
        31 => view.key_accelerator().unwrap_or(0),
        34 => {
            view.scroll_stop();
            // Preserve the native byte when the method does not write it.
            if input[10] != 0 {
                output[14] = u32::from(view.owner.motion.stopping);
                output[15] = view.owner.motion.active_step.to_bits();
            }
            0 // ui-windows §3.3 gives no return contract for ScrollStop.
        }
        39 => u32::from(view.terrain_query_enabled()),
        51 => view.last_selected_building().unwrap_or(0),
        52 => view.last_selected_lot().unwrap_or(0),
        53 => {
            view.set_last_selected_building(input[20]);
            output[6] = view.last_selected_building()?;
            output[7] = 1;
            0
        }
        54 => {
            view.set_last_selected_lot(input[20]);
            output[8] = view.last_selected_lot()?;
            output[9] = 1;
            0
        }
        143 => u32::from(view.on_command(input[20], input[21])),
        _ => return None,
    };
    (!host.unexpected.get()).then_some((value, host.reads.get(), output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_uses_before_state_and_preserves_idle_bytes() {
        let mut input = [0; WORDS];
        input[11] = 3.0_f32.to_bits();
        input[12] = 0.5_f32.to_bits();
        input[13] = 1;
        input[14] = 7;
        input[15] = 9.0_f32.to_bits();
        assert_eq!(replay(34, &input).unwrap().2, input);
        input[10] = 1;
        let (_, reads, output) = replay(34, &input).unwrap();
        assert_eq!(reads, 0);
        assert_eq!(output[14], 1);
        assert_eq!(output[15], 1.5_f32.to_bits());
        assert_eq!(input[15], 9.0_f32.to_bits());
    }

    #[test]
    fn zoom_tests_use_signed_unsorted_limits_and_one_observed_call() {
        let mut input = [0; WORDS];
        input[16] = 4;
        input[17] = 2;
        input[18] = 3;
        assert_eq!(replay(12, &input), Some((0, 1, input)));
        assert_eq!(replay(13, &input), Some((0, 1, input)));
        input[18] = u32::MAX;
        assert_eq!(replay(12, &input), Some((1, 1, input)));
        assert!(replay(8, &input).is_none());
        assert!(replay(7, &input).is_none());
    }

    #[test]
    fn borrowed_getters_return_the_entry_token_without_writes_or_calls() {
        let mut input = [0xCAFE_BABE; WORDS];
        // Distinguish the renderer, model maker and accelerator, including null.
        input[1] = 0x1234_5678;
        input[2] = 0x8765_4321;
        input[3] = 0;
        for (slot, value) in [(4, input[1]), (5, input[2]), (31, input[3])] {
            assert_eq!(replay(slot, &input), Some((value, 0, input)));
        }
        input[3] = 0xF123_4567;
        assert_eq!(replay(31, &input), Some((input[3], 0, input)));
    }
}
