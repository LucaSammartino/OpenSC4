// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl Windows {
    pub fn calc_absolute(&mut self, w: Window) {
        let mut r = self.get(w).area;
        let mut p = self.get(w).parent;
        while let Some(a) = p {
            let s = self.get(a);
            r = r.offset(s.area.0, s.area.1);
            p = s.parent;
        }
        self.get_mut(w).absolute = r;
        let list = self.read_children(w);
        for &c in &list {
            self.calc_absolute(c);
        }
        self.unlock_children(list);
    }
    pub fn set_area(&mut self, host: &mut impl Host, w: Window, area: Rect) {
        let old = self.get(w).area;
        self.get_mut(w).area = area;
        if (old.width(), old.height()) != (area.width(), area.height()) && self.get(w).flag(0x10000) {
            self.private_buffer(host, w, true);
        }
        self.calc_absolute(w);
        self.set_draw_area_recursive(host, w);
    }
    pub fn move_to(&mut self, host: &mut impl Host, w: Window, x: i32, y: i32) {
        let r = self.get(w).area;
        host.set_area(
            self,
            w,
            Rect(x, y, x.wrapping_add(r.width()), y.wrapping_add(r.height())),
        );
    }
    pub fn offset(&mut self, host: &mut impl Host, w: Window, x: i32, y: i32) {
        host.set_area(self, w, self.get(w).area.offset(x, y));
    }
    pub fn screen_to_window(&self, host: &mut impl Host, w: Window, point: (i32, i32)) -> (i32, i32) {
        let origin = host.window_to_screen(self, w, (0, 0));
        (point.0.wrapping_sub(origin.0), point.1.wrapping_sub(origin.1))
    }
    pub fn window_to_window(
        &self,
        host: &mut impl Host,
        w: Window,
        other: Option<Window>,
        point: &mut (i32, i32),
    ) -> bool {
        let Some(other) = other else {
            return false;
        };
        let screen = host.window_to_screen(self, w, *point);
        *point = self.screen_to_window(host, other, screen);
        true
    }
    pub fn point_transparent(&self, host: &mut impl Host, w: Window, x: i32, y: i32) -> bool {
        let Some(buffer) = self.get(w).private_buffer else {
            return false;
        };
        if !host.buffer_info(buffer).usable {
            return false;
        }
        host.event(NativeOp::BufferLock(buffer, 0x800));
        if !host.lock_buffer(buffer, 0x800) {
            return false;
        }
        let result = host.pixel_transparent(buffer, x, y);
        host.event(NativeOp::BufferUnlock(buffer, 0x800));
        result
    }
    pub fn point_window(&self, host: &mut impl Host, w: Window, x: i32, y: i32) -> bool {
        self.get(w).area.local().contains(x, y)
            && (!self.get(w).flag(0x80000) || !self.point_transparent(host, w, x, y))
    }
    pub fn point_parent(&self, w: Window, x: i32, y: i32) -> bool {
        self.get(w).area.contains(x, y)
    }
    pub fn point_screen(&self, host: &mut impl Host, w: Window, x: i32, y: i32) -> bool {
        if !self.get(w).absolute.contains(x, y) {
            return false;
        }
        let p = self.screen_to_window(host, w, (x, y));
        !self.get(w).flag(0x80000) || !self.point_transparent(host, w, p.0, p.1)
    }
    pub fn child_from_point(&mut self, host: &mut impl Host, w: Window, x: i32, y: i32) -> Option<Window> {
        let list = self.read_children(w);
        let mut result = None;
        for &c in &list {
            if self.get(c).flag(1) {
                result = self.child_from_point(host, c, x, y);
                if result.is_some() {
                    break;
                }
            }
        }
        self.unlock_children(list);
        result.or_else(|| self.point_screen(host, w, x, y).then_some(w))
    }
    pub fn set_draw_area(&mut self, host: &mut impl Host, w: Window) {
        let s = self.get(w);
        let own = s.private_buffer.is_some();
        let overlay = s.flag(0x8000000);
        let mut area = s.area;
        let mut blit = s.blit;
        if own {
            area = area.local();
            blit = s.area;
            let mut p = s.parent;
            while let Some(a) = p {
                let s = self.get(a);
                if s.private_buffer.is_some() {
                    break;
                }
                blit = blit.offset(s.area.0, s.area.1);
                p = s.parent;
            }
        } else {
            let mut p = s.parent;
            while let Some(a) = p {
                let s = self.get(a);
                if s.private_buffer.is_some() {
                    break;
                }
                if overlay {
                    area = self.get(w).absolute;
                    break;
                }
                area = area.offset(s.area.0, s.area.1);
                p = s.parent;
            }
            let present = *self.draw_service.get_or_insert_with(|| host.draw_service_present());
            if present {
                area = area.local();
            }
        }
        let mut origin = area;
        if !own {
            let mut p = Some(w);
            while let Some(a) = p {
                let s = self.get(a);
                if s.private_buffer.is_some() {
                    break;
                }
                if s.flag(0x8000000) {
                    origin = origin.offset(s.absolute.0, s.absolute.1);
                    break;
                }
                origin = origin.offset(s.area.0, s.area.1);
                p = s.parent;
            }
        }
        let s = self.get_mut(w);
        s.draw_area = area;
        s.draw_origin = origin;
        s.blit = blit;
    }
    pub fn set_draw_area_recursive(&mut self, host: &mut impl Host, w: Window) {
        self.set_draw_area(host, w);
        let list = self.read_children(w);
        for &c in &list {
            self.set_draw_area_recursive(host, c);
        }
        self.unlock_children(list);
    }
    pub fn set_buffer(&mut self, host: &mut impl Host, w: Window) {
        let mut buffer = self.get(w).private_buffer;
        let overlay = self.get(w).flag(0x8000000);
        let mut p = self.get(w).parent;
        while buffer.is_none() {
            let Some(a) = p else {
                break;
            };
            buffer = self.get(a).private_buffer;
            if overlay {
                break;
            }
            p = self.get(a).parent;
        }
        let buffered = buffer.is_some();
        if buffer.is_none() {
            buffer = host.screen_buffer();
        }
        let context = buffer.and_then(|b| host.buffer_info(b).context);
        let s = self.get_mut(w);
        s.draw_buffer = buffer;
        s.buffered = buffered;
        s.draw_context = context;
        self.set_draw_area_recursive(host, w);
    }
    pub fn set_buffer_recursive(&mut self, host: &mut impl Host, w: Window) {
        self.set_buffer(host, w);
        let list = self.read_children(w);
        for &c in &list {
            self.set_buffer_recursive(host, c);
        }
        self.unlock_children(list);
    }
    pub fn invalidate_self(&mut self, w: Window) {
        self.get_mut(w).dirty = true;
    }
    pub fn invalidate_all_parents(&mut self, w: Window) {
        let mut p = self.get(w).parent;
        while let Some(a) = p {
            self.invalidate_self(a);
            if self.get(a).flag(0x8000000) {
                break;
            }
            p = self.get(a).parent;
        }
    }
    pub fn invalidate_self_and_parents(&mut self, w: Window) {
        self.invalidate_self(w);
        if !self.get(w).flag(0x8000000) {
            self.invalidate_all_parents(w);
        }
    }
    pub fn center(&mut self, host: &mut impl Host, w: Window, rect: Option<Rect>) {
        let Some(r) = rect else {
            return;
        };
        let a = self.get(w).area;
        self.move_to(
            host,
            w,
            r.0.wrapping_add(r.width().wrapping_sub(a.width()) / 2),
            r.1.wrapping_add(r.height().wrapping_sub(a.height()) / 2),
        );
    }
    pub fn fit_rect(&self, w: Window, r: &mut Rect, margin: i32) -> bool {
        let a = self.get(w).area;
        let (width, height) = (a.width(), a.height());
        if r.0 < 0 {
            r.0 = margin;
        }
        if r.1 < 0 {
            r.1 = margin;
        }
        if r.2 > width {
            r.0 = r.0.wrapping_add(width.wrapping_sub(r.2).wrapping_sub(margin));
            r.2 = width.wrapping_sub(margin);
        }
        if r.3 > height {
            r.1 = r.1.wrapping_add(height.wrapping_sub(r.3).wrapping_sub(margin));
            r.3 = height.wrapping_sub(margin);
        }
        r.width().wrapping_add(margin) <= width && r.height().wrapping_add(margin) <= height
    }
    pub fn make_key_equivalent(key: u32, modifiers: u32) -> u32 {
        modifiers.wrapping_shl(16) | (key & 0xffff)
    }
    pub fn check_key_equivalent(&self, host: &mut impl Host, w: Window, key: u32, modifiers: u32) -> bool {
        host.make_key_equivalent(key, modifiers) == self.get(w).key_equivalent
    }
    pub fn child_key_equivalent(&self, host: &mut impl Host, w: Window, key: u32, modifiers: u32) -> Option<Window> {
        self.get(w)
            .children
            .iter()
            .copied()
            .find(|&c| self.check_key_equivalent(host, c, key, modifiers))
    }
}
