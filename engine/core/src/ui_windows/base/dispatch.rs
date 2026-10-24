// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl Windows {
    pub fn add_filter(&mut self, host: &mut impl Host, w: Window, f: Object) -> bool {
        self.get_mut(w).filters.push(f);
        host.event(NativeOp::AddRef(f));
        true
    }
    pub fn remove_filter(&mut self, host: &mut impl Host, w: Window, f: Object) -> bool {
        let Some(i) = self.get(w).filters.iter().position(|&v| v == f) else {
            return false;
        };
        if self.get(w).filtering {
            // WT10: deferred entries stay in the active list until outermost exit.
            self.get_mut(w).removed_filters.push(f);
        } else {
            self.get_mut(w).filters.remove(i);
            host.event(NativeOp::Release(f));
        }
        true
    }
    pub(super) fn run_filters(&mut self, host: &mut impl Host, w: Window, msg: Message) -> bool {
        let nested = self.get(w).filtering;
        self.get_mut(w).filtering = true;
        let filters = self.get(w).filters.clone();
        let mut handled = false;
        for f in filters {
            if host.filter(self, f, w, msg) {
                handled = true;
                break;
            }
        }
        self.get_mut(w).filtering = nested;
        if !nested {
            let pending = std::mem::take(&mut self.get_mut(w).removed_filters);
            for f in pending {
                self.remove_filter(host, w, f);
            }
        }
        handled
    }
    pub fn accelerate(&mut self, host: &mut impl Host, w: Window, msg: Message) -> bool {
        self.get(w).accelerator.is_some_and(|a| host.accelerate(a, msg))
    }
    pub fn do_message(&mut self, host: &mut impl Host, w: Window, msg: Message) -> bool {
        if self.run_filters(host, w, msg) {
            return true;
        }
        let [kind, a, b, c, _] = msg.0;
        if kind == 1 {
            self.child_delete(host, w, u64::from(a));
            return false;
        } // WT11: void slot's AL is unpublished.
        let buttons = c >> 16;
        let x = (a as i16 as i32) as u32;
        let y = (b as i16 as i32) as u32;
        let (slot, args) = match kind {
            3 => (143, vec![a, b]),
            4 => (129, vec![a & 0xff]),
            5 => (130, vec![a, b]),
            6 => (131, vec![a, b]),
            7 => (134, vec![x, y, buttons]),
            8 => (135, vec![x, y, buttons]),
            10 => (136, vec![x, y, buttons]),
            11 => (137, vec![x, y, buttons]),
            13 => (138, vec![x, y, buttons]),
            14 => (139, vec![a, b, buttons, (c as i16 as i32) as u32]),
            16 => (133, vec![a]),
            17 => (132, vec![a]),
            18 => (140, vec![a, b]),
            19 => (141, vec![a]),
            20 => (142, vec![a]),
            _ => return false,
        };
        if kind == 17 && !self.get(w).flag(1) {
            self.set_focus_to(host, self.get(w).notification);
        }
        host.event(NativeOp::Handler(w, slot, args.clone()));
        if host.handler(self, w, slot, &args) {
            return true;
        }
        (kind == 5 || kind == 6) && self.accelerate(host, w, msg)
    }
    pub fn send(&mut self, host: &mut impl Host, target: Window, msg: Message) -> bool {
        self.valid(target) && self.do_message(host, target, msg)
    }
    pub fn post(&mut self, host: &mut impl Host, target: Window, msg: Message) -> bool {
        host.event(NativeOp::Post(target, msg));
        self.posts.push_back((target, msg));
        true
    }
    pub fn dispatch_posted(&mut self, host: &mut impl Host) {
        while let Some((w, msg)) = self.posts.pop_front() {
            self.send(host, w, msg);
        }
    }
    pub fn update_cursor(&mut self, host: &mut impl Host, w: Window) {
        if self.get(w).cursor.is_none() {
            host.event(NativeOp::CursorLookup(w));
            self.get_mut(w).cursor = host.cursor(w);
        }
        if let Some(c) = self.get(w).cursor {
            host.event(NativeOp::CursorActivate(c));
        }
    }
    pub fn set_mouse_window(&mut self, host: &mut impl Host, w: Option<Window>) {
        if self.mouse_window == w {
            return;
        }
        let old = self.mouse_window.filter(|&old| self.valid(old));
        let new = w.filter(|&w| self.valid(w));
        if let Some(old) = old {
            self.do_message(host, old, Message::new(20, new.unwrap_or(0) as u32, 0, 0, 0));
        }
        if let Some(new) = new {
            self.do_message(host, new, Message::new(19, old.unwrap_or(0) as u32, 0, 0, 0));
        }
        if let Some(old) = old {
            self.release(old);
        }
        if let Some(new) = new {
            self.add_ref(new);
        }
        self.mouse_window = new;
    }
    pub fn process_cursor(&mut self, host: &mut impl Host, w: Window, msg: &mut Message) -> bool {
        let [kind, x, y, _, _] = msg.0;
        let list = self.read_children(w);
        let mut target = None;
        for &c in &list {
            if self.get(c).flag(1)
                && !self.get(c).flag(0x200000)
                && (self.get(c).flag(2) || kind == 13)
                && self.point_screen(host, c, x as i32, y as i32)
            {
                target = Some(c);
                break;
            }
        }
        self.unlock_children(list);
        if let Some(c) = target {
            return self.process_cursor(host, c, msg);
        }
        if self.get(w).flag(0x200000) || !self.get(w).flag(1) {
            return false;
        }
        let enabled = self.get(w).flag(2);
        if !enabled && kind != 13 {
            return false;
        }
        if enabled && matches!(kind, 7..=9) {
            self.pull_to_front(w);
            self.set_focus(host, Some(w));
        }
        let local = self.screen_to_window(host, w, (x as i32, y as i32));
        msg.0[1] = local.0 as u32;
        msg.0[2] = local.1 as u32;
        if enabled && kind == 13 {
            self.update_cursor(host, w);
        }
        self.set_mouse_window(host, Some(w));
        self.do_message(host, w, *msg)
    }
}
