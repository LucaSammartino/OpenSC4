// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl Windows {
    fn buffer_color(&self, host: &mut impl Host, w: Window, alpha: u32) {
        if let Some(b) = self.get(w).private_buffer {
            host.event(NativeOp::BufferColor(b, (alpha << 24) | 0xffffff));
        }
    }
    pub fn set_fade(&mut self, host: &mut impl Host, w: Window, alpha: i32, period: i32) {
        let time = host.clock();
        for (key, value) in [(FADE_ALPHA, alpha), (FADE_TIME, time as i32), (FADE_PERIOD, period)] {
            self.set_param(host, w, key, Some(Param::Integer(value)));
        }
        self.buffer_color(host, w, alpha as u32);
    }
    pub fn fade_advance(&mut self, host: &mut impl Host, w: Window) {
        let period = self.get(w).integer(FADE_PERIOD);
        if period == 0 {
            return;
        }
        let elapsed = host.clock().wrapping_sub(self.get(w).integer(FADE_TIME) as u32);
        let alpha = f64::from(self.get(w).integer(FADE_ALPHA)) + f64::from(elapsed) * 255.0 / f64::from(period);
        let alpha = if !(0.0..256.0).contains(&alpha) {
            self.set_param(host, w, FADE_PERIOD, Some(Param::Integer(0)));
            if alpha < 0.0 {
                0
            } else {
                255
            }
        } else {
            alpha as u32
        };
        self.buffer_color(host, w, alpha);
    }
    fn ancestors_visible(&self, w: Window) -> bool {
        let mut p = self.get(w).parent;
        while let Some(a) = p {
            if !self.get(a).flag(1) {
                return false;
            }
            p = self.get(a).parent;
        }
        true
    }
    pub fn set_transparency(&self, host: &mut impl Host, w: Window, on: bool) -> bool {
        let Some(b) = self.get(w).private_buffer else {
            return false;
        };
        let color = if on {
            let Some(c) = host.transparent_color(0x0896_3f6c) else {
                return false;
            };
            Some(c)
        } else {
            None
        };
        host.event(NativeOp::TransparentColor(b, color));
        true
    }
    pub fn fill_transparent(&self, host: &mut impl Host, w: Window) -> bool {
        let Some(b) = self.get(w).private_buffer else {
            return false;
        };
        host.event(NativeOp::BufferLock(b, 0x8001));
        if !host.lock_buffer(b, 0x8001) {
            return false;
        }
        if let Some(c) = host.buffer_info(b).transparent {
            host.event(NativeOp::Fill(b, self.get(w).draw_area, c));
        }
        host.event(NativeOp::BufferUnlock(b, 0x8001));
        true
    }
    pub fn packed_to_native(&self, host: &mut impl Host, w: Window, color: u32) -> u32 {
        host.rgb_to_native(
            self.get(w).draw_buffer,
            [(color >> 16) as u8, (color >> 8) as u8, color as u8],
        )
        .unwrap_or(0)
    }
    pub fn native_to_packed(&self, host: &mut impl Host, w: Window, color: u32) -> u32 {
        let [r, g, b] = host.native_to_rgb(self.get(w).draw_buffer, color).unwrap_or([0; 3]);
        0xff000000 | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
    }
    pub fn private_buffer(&mut self, host: &mut impl Host, w: Window, on: bool) -> bool {
        let mut ok = true;
        if on && self.ancestors_visible(w) {
            self.get_mut(w).flags |= 0x10000;
            let r = self.get(w).area;
            let current_size = self.get(w).private_buffer.map(|b| host.buffer_info(b).area);
            if current_size.map(|r| (r.width(), r.height())) != Some((r.width(), r.height())) {
                // Concrete slot 150 owns buffer allocation/replacement; its internals
                // are explicitly unsettled in §12. Host returns the resulting buffer.
                if let Some(buffer) = host.create_private_buffer(w, r.width(), r.height()) {
                    self.get_mut(w).private_buffer = Some(buffer);
                    if self.get(w).flag(0x20000) {
                        self.set_transparency(host, w, true);
                    }
                    host.set_area(self, w, r);
                } else {
                    ok = false;
                }
            }
        } else if let Some(buffer) = self.get(w).private_buffer {
            let content = host.buffer_info(buffer).content;
            let fade = content && self.get(w).flag(0x20) && self.get(w).flag(0x400000);
            if fade {
                self.get_mut(w).flags &= !0x400000;
            }
            if fade && !self.get(w).params.contains_key(&FADE_TEMP) && self.ancestors_visible(w) {
                self.steal_fade_buffer(host, w, buffer);
            } else if content {
                host.event(NativeOp::BufferReleaseSurface(buffer));
            }
            host.event(NativeOp::Release(buffer));
            self.get_mut(w).private_buffer = None;
        }
        self.set_buffer_recursive(host, w);
        self.child_private_buffers(host, w, self.get(w).flag(1));
        self.invalidate_self_and_parents(w);
        ok
    }
    pub fn child_private_buffers(&mut self, host: &mut impl Host, w: Window, on: bool) -> bool {
        let children = self.read_children(w);
        let mut ok = true;
        for &c in &children {
            if self.get(c).flag(0x10000) {
                if !on || self.get(c).flag(1) {
                    ok &= self.private_buffer(host, c, on);
                } else {
                    self.child_private_buffers(host, c, false);
                }
            } else {
                self.child_private_buffers(host, c, on);
            }
            if on {
                let r = self.get(c).area;
                self.move_to(host, c, r.0, r.1);
            }
        }
        self.unlock_children(children);
        ok
    }
    fn steal_fade_buffer(&mut self, host: &mut impl Host, src: Window, buffer: Object) -> Window {
        let s = self.get(src).clone();
        let tmp = self.create(host, s.manager, s.keyboard);
        self.set_param(host, tmp, FADE_TEMP, Some(Param::Integer(1)));
        self.set_flag(host, tmp, 4, true);
        if let Some(p) = s.parent {
            self.child_add(host, p, tmp);
            host.set_area(self, tmp, s.area);
            self.move_relative_to(tmp, src, false);
        } else if let Some(main) = self.main_window {
            self.child_add(host, main, tmp);
            host.set_area(self, tmp, s.absolute);
        }
        self.set_flag(host, tmp, 0x40000, false);
        self.set_flag(host, tmp, 0x230021, true);
        host.event(NativeOp::AddRef(buffer));
        self.get_mut(tmp).private_buffer = Some(buffer);
        self.set_buffer(host, tmp);
        self.set_fade(host, tmp, 255, s.fade_periods.1.wrapping_neg());
        tmp
    }
    pub fn queue_plot(&mut self, host: &mut impl Host, w: Option<Window>) -> bool {
        let Some(w) = w else {
            return false;
        };
        if !self.queue_enabled {
            return false;
        }
        self.plots.push_back(w);
        host.event(NativeOp::QueuePlot(w));
        true
    }
    pub fn flush_plots(&mut self, host: &mut impl Host) -> bool {
        if !self.queue_enabled {
            return false;
        }
        while let Some(w) = self.plots.pop_front() {
            if self.valid(w) {
                self.plot(host, w);
            }
        }
        true
    }
    fn queue_overlay(&mut self, host: &mut impl Host, w: Window) {
        self.get_mut(w).flags |= 0x4000000;
        self.queue_plot(host, Some(w));
    }
    fn clean_overlays(&mut self, host: &mut impl Host, w: Window) {
        let list = self.read_children(w);
        for &c in &list {
            if !self.get(c).flag(1) {
                continue;
            }
            if self.get(c).flag(0x8000000) {
                self.queue_overlay(host, c);
            } else {
                self.clean_overlays(host, c);
            }
        }
        self.unlock_children(list);
    }
    pub fn plot(&mut self, host: &mut impl Host, w: Window) -> bool {
        let temporary = self.get(w).params.contains_key(&FADE_TEMP);
        if temporary && !self.get(w).fade_running() {
            self.destroy(host, w);
            return true;
        }
        let ok = self.plot_composite(host, w) && self.plot_present(host, w);
        if !self.get(w).buffered {
            self.flush_plots(host);
        }
        if temporary && self.valid(w) {
            self.invalidate_self_and_parents(w);
        }
        ok
    }
    pub fn plot_composite(&mut self, host: &mut impl Host, w: Window) -> bool {
        if !self.get(w).flag(1) && (!self.get(w).flag(0x20) || !self.get(w).fade_running()) {
            return false;
        }
        if self.get(w).flag(0x20) {
            self.fade_advance(host, w);
        }
        if self.run_filters(host, w, Message::new(0x16, 0, 0, 0, 0)) {
            return true;
        }
        if !self.get(w).dirty {
            self.clean_overlays(host, w);
            return true;
        }
        if self.get(w).private_buffer.is_some() {
            if self.get(w).parent.is_some() {
                self.get_mut(w).dirty = false;
            }
            if self.get(w).flags & 0x60000 == 0x60000 {
                self.fill_transparent(host, w);
            }
        }
        let s = self.get(w).clone();
        if let Some(c) = s.draw_context {
            host.event(NativeOp::AddRef(c));
            host.event(NativeOp::Context(c, 3, vec![]));
            host.event(NativeOp::ContextRect(c, 8, s.draw_origin));
            host.event(NativeOp::ContextRect(c, 6, s.draw_origin));
            for slot in [4, 15, 10, 19] {
                host.event(NativeOp::Context(c, slot, vec![]));
            }
            let mode = u32::from(s.flag(4) && !s.flag(0x10000));
            for slot in [22, 23] {
                host.event(NativeOp::Context(c, slot, vec![mode]));
            }
            host.event(NativeOp::Context(
                c,
                21,
                vec![if s.flag(8) && !s.flag(0x10000) {
                    s.shade
                } else {
                    u32::MAX
                }],
            ));
            host.event(NativeOp::Context(c, 43, vec![]));
        }
        host.event(NativeOp::Paint(w));
        host.paint(self, w);
        if self.get(w).private_buffer.is_some() {
            self.get_mut(w).flags |= 0x400000;
        }
        if let Some(c) = s.draw_context {
            for slot in [42, 11, 9] {
                host.event(NativeOp::Context(c, slot, vec![]));
            }
            host.event(NativeOp::Release(c));
        }
        let list = self.read_children(w);
        let mut deferred = Vec::<Rect>::new();
        for &c in list.iter().rev() {
            if self.get(w).flag(0x8000000) {
                self.queue_overlay(host, c);
                continue;
            }
            if !self.get(c).flag(1) {
                continue;
            }
            let r = self.get(c).absolute;
            if self.get(c).flag(0x8000000) || deferred.iter().any(|&a| a.overlaps(r)) {
                deferred.push(r);
                self.queue_overlay(host, c);
            } else {
                self.plot(host, c);
            }
        }
        self.unlock_children(list);
        true
    }
    pub fn plot_present(&mut self, host: &mut impl Host, w: Window) -> bool {
        let s = self.get(w).clone();
        let Some(source) = s.private_buffer else {
            return true;
        };
        let (destination, rect) = if s.flag(0x4000000) && s.parent.is_some() {
            self.get_mut(w).flags &= !0x4000000;
            (host.screen_buffer(), s.absolute)
        } else {
            (
                s.parent
                    .and_then(|p| self.get(p).draw_buffer)
                    .or_else(|| host.screen_buffer()),
                s.blit,
            )
        };
        let Some(destination) = destination else {
            return true;
        };
        if let Some(c) = host.buffer_info(destination).present_context {
            host.event(NativeOp::Context(c, 4, vec![]));
            host.event(NativeOp::Context(c, 8, vec![0, 0]));
            host.event(NativeOp::Context(c, 10, vec![]));
            for slot in [22, 23] {
                host.event(NativeOp::Context(c, slot, vec![u32::from(s.flag(4))]));
            }
            host.event(NativeOp::Context(
                c,
                21,
                vec![if s.flag(8) { s.shade } else { u32::MAX }],
            ));
            if let Some(color) = host.buffer_info(source).color {
                let color = if s.flag(8) {
                    (s.shade & 0xffffff) | (color & 0xff000000)
                } else {
                    color
                };
                host.event(NativeOp::Context(c, 21, vec![color]));
            }
            host.event(NativeOp::Blit {
                destination,
                source,
                from: s.draw_area,
                to: rect,
                context: true,
            });
            host.event(NativeOp::Context(c, 11, vec![]));
            host.event(NativeOp::ReleasePresentInterface(destination));
        } else {
            host.event(NativeOp::BufferLock(destination, 0x8010));
            if host.lock_buffer(destination, 0x8010) {
                host.event(NativeOp::Blit {
                    destination,
                    source,
                    from: s.draw_area,
                    to: rect,
                    context: false,
                });
                host.event(NativeOp::BufferUnlock(destination, 0x8010));
            }
        }
        true
    }
}
