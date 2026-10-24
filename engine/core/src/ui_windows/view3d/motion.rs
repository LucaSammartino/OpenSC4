// SPDX-License-Identifier: GPL-3.0-or-later
//! Window-owned motion and paint scheduling (view3d-window §6f, view3d §5).

use super::View3DWin;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewChange {
    Zoom(i32),
    ZoomIn,
    ZoomOut,
    Rotation(i32),
    RotateLeft,
    RotateRight,
    ZoomAndRotation(i32, i32),
}

/// Camera, renderer, simulator, timer and drawing remain foreign interfaces.
pub trait MotionHost {
    fn virtual_zoom(&self) -> i32;
    fn camera_zoom(&self) -> f32;
    fn view_change(&mut self, change: ViewChange) -> bool;
    fn scroll_camera(&mut self, angle: f32, distance: f32) -> bool;
    fn post_scroll_message(&mut self, message: u32);
    fn begin_scroll_render_settings(&mut self);
    fn end_scroll_render_settings(&mut self);
    fn simulator_present(&self) -> bool;
    fn hidden_pause(&mut self);
    fn hidden_resume(&mut self);
    fn restart_hidden_timer(&mut self);
    fn reset_paint_timer(&mut self);
    fn animation_time(&mut self) -> f32;
    fn set_draw_context_time(&mut self, time: f32);
    fn draw(&mut self);
    fn update_cost_indicator(&mut self, dt: f32);
    fn paint_tail(&mut self);
    fn deferred_safe(&self) -> bool;
    fn deferred_enabled(&self) -> bool;
    fn deferred_reference(&mut self, message: &super::Notification, acquire: bool);
    fn repost_deferred(&mut self, message: &super::Notification, delivery: u32);
}

#[derive(Debug)]
pub struct Motion {
    pub keyboard: [i32; 2],
    pub edge: [i32; 2],
    pub angle: f32,
    pub speed: f32,
    pub scrolling: bool,
    pub stopping: bool,
    pub stop_step: f32,
    pub proportional_stop: bool,
    pub active_step: f32,
    pub edge_speed: f32,
    pub keyboard_speed: f32,
    pub zoom_min: i32,
    pub zoom_max: i32,
    pub hidden_paused: bool,
    pub resume_delay_ms: u64,
    pub paint_enabled: bool,
    pub city_initializing: bool,
    /// In timer microseconds, matching the timer/period comparison (§5.1).
    pub paint_period_us: f32,
    pub reduced_render_settings: bool,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            keyboard: [0, 0],
            edge: [0, 0],
            angle: 0.0,
            speed: 0.0,
            scrolling: false,
            stopping: false,
            stop_step: 1.0,
            proportional_stop: false,
            active_step: 0.0,
            edge_speed: 0.2,
            keyboard_speed: 0.2,
            zoom_min: 0,
            zoom_max: 5,
            hidden_paused: false,
            resume_delay_ms: 2000,
            paint_enabled: false,
            city_initializing: false,
            // §6f: zero microseconds is always due.
            paint_period_us: 0.0,
            reduced_render_settings: false,
        }
    }
}

impl View3DWin {
    pub fn motion_mut(&mut self) -> &mut Motion {
        &mut self.owner.motion
    }

    pub fn temporary_hidden_pause(&mut self, host: &mut impl MotionHost) {
        let motion = &mut self.owner.motion;
        if !motion.hidden_paused && host.simulator_present() {
            motion.hidden_paused = true;
            host.hidden_pause();
        }
        host.restart_hidden_timer();
    }

    pub fn change_view(&mut self, host: &mut impl MotionHost, change: ViewChange) -> bool {
        let changed = host.view_change(change);
        if changed {
            self.temporary_hidden_pause(host);
        }
        changed
    }

    pub fn set_zoom(&mut self, host: &mut impl MotionHost, zoom: i32) -> bool {
        let m = &self.owner.motion;
        // §6f: minimum wins first, even for inverted limits.
        let zoom = if zoom < m.zoom_min {
            m.zoom_min
        } else if zoom > m.zoom_max {
            m.zoom_max
        } else {
            zoom
        };
        self.change_view(host, ViewChange::Zoom(zoom))
    }

    pub fn can_zoom_in(&self, host: &impl MotionHost) -> bool {
        host.virtual_zoom() < self.owner.motion.zoom_max
    }

    pub fn can_zoom_out(&self, host: &impl MotionHost) -> bool {
        host.virtual_zoom() > self.owner.motion.zoom_min
    }

    pub fn zoom_step(&mut self, host: &mut impl MotionHost, inward: bool) -> bool {
        if inward && self.can_zoom_in(host) {
            self.change_view(host, ViewChange::ZoomIn)
        } else if !inward && self.can_zoom_out(host) {
            self.change_view(host, ViewChange::ZoomOut)
        } else {
            false
        }
    }

    pub fn set_zoom_limits(
        &mut self,
        host: &mut impl MotionHost,
        min: i32,
        max: i32,
        extra_zoom_preference: Option<bool>,
    ) -> bool {
        self.owner.motion.zoom_min = min;
        self.owner.motion.zoom_max = if max > 4 {
            if extra_zoom_preference.unwrap_or(true) {
                5
            } else {
                4
            }
        } else {
            max
        };
        let zoom = host.virtual_zoom();
        // The bounds test deliberately uses the *unclamped* maximum (§6m.5).
        if zoom < min {
            self.set_zoom(host, min);
        } else if zoom > max {
            self.set_zoom(host, max);
        }
        true
    }

    pub fn set_scrolling(&mut self, host: &mut impl MotionHost, on: bool, angle: f32, speed: f32) {
        if on != self.owner.motion.scrolling {
            host.post_scroll_message(if on { 0xCAD6_1E00 } else { 0xEAD6_1E16 });
            if self.owner.motion.reduced_render_settings {
                if on {
                    host.begin_scroll_render_settings();
                } else {
                    host.end_scroll_render_settings();
                }
            }
        }
        let m = &mut self.owner.motion;
        m.scrolling = on;
        m.angle = angle;
        m.speed = speed;
        m.stopping = false;
    }

    pub fn scroll_stop(&mut self) {
        let m = &mut self.owner.motion;
        if m.scrolling {
            m.stopping = true;
            m.active_step = m.stop_step * if m.proportional_stop { m.speed } else { 1.0 };
        }
    }

    pub fn recalc_scroll_velocity(&mut self, host: &mut impl MotionHost, edge_preference: bool) {
        let m = &mut self.owner.motion;
        let keyboard = m.keyboard != [0, 0];
        let mut direction = m.edge;
        if edge_preference {
            m.edge = [0, 0];
        }
        for (axis, &key) in direction.iter_mut().zip(&m.keyboard) {
            if key != 0 {
                *axis = key;
            }
        }
        if direction == [0, 0] {
            if !m.stopping {
                self.scroll_stop();
            }
            return;
        }
        let angle = (direction[1] as f32)
            .atan2(direction[0] as f32)
            .rem_euclid(std::f32::consts::TAU);
        let speed = if keyboard { m.keyboard_speed } else { m.edge_speed } * 1.2_f32.powf(host.camera_zoom());
        self.set_scrolling(host, true, angle, speed);
    }

    /// Arrow commands only start when neither axis is held (§6g, §6m.2).
    pub fn keyboard_scroll(&mut self, host: &mut impl MotionHost, direction: [i32; 2], edge: bool) {
        if self.owner.motion.keyboard == [0, 0] || direction == [0, 0] {
            self.owner.motion.keyboard = direction;
            self.recalc_scroll_velocity(host, edge);
        }
    }

    pub(super) fn paint_deferred(&mut self, host: &mut impl MotionHost) {
        if !self.owner.deferred.is_empty() && host.deferred_safe() {
            host.deferred_reference(&self.owner.deferred[0], true);
            let message = self.owner.deferred.remove(0);
            host.deferred_reference(&message, false);
            // Recheck after unlink/release, which may change modal state.
            if host.deferred_enabled() {
                host.repost_deferred(&message, 0);
            }
            host.deferred_reference(&message, false);
        }
        host.paint_tail();
    }

    /// GZPaint always returns true, including disabled and throttled paints.
    pub fn paint(&mut self, host: &mut impl MotionHost, elapsed_us: u64, hidden_elapsed_ms: u64) -> bool {
        if !self.owner.motion.paint_enabled {
            self.paint_deferred(host);
            return true;
        }
        let period = self.owner.motion.paint_period_us;
        if period != 0.0 && (elapsed_us as f64) <= f64::from(period) {
            self.paint_deferred(host);
            return true;
        }
        host.reset_paint_timer();
        let dt = ((elapsed_us as f64) * 0.000_001).min(0.5) as f32;
        host.update_cost_indicator(dt);
        let mut restarted = false;
        if self.owner.motion.scrolling {
            let m = &self.owner.motion;
            let moved = host.scroll_camera(m.angle, dt * m.speed * 25.0);
            if m.stopping {
                self.owner.motion.speed -= self.owner.motion.active_step;
                if self.owner.motion.speed <= 0.0 {
                    let m = &self.owner.motion;
                    self.set_scrolling(host, false, m.angle, m.speed);
                }
            }
            if moved {
                self.temporary_hidden_pause(host);
                restarted = true;
            }
        }
        // A pause restarted above has age zero, not the caller's old timer age.
        // The host timer can be restarted by a scroll on this very paint.
        let m = &mut self.owner.motion;
        if m.hidden_paused && !restarted && hidden_elapsed_ms > m.resume_delay_ms {
            host.hidden_resume();
            m.hidden_paused = false;
            host.restart_hidden_timer();
        }
        let time = if m.city_initializing { dt } else { host.animation_time() };
        host.set_draw_context_time(time);
        host.draw();
        self.paint_deferred(host);
        true
    }
}
