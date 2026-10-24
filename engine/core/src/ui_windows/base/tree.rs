// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Debug)]
pub struct ChildRead {
    owner: Window,
    generation: u64,
    list: Vec<Window>,
}
impl std::ops::Deref for ChildRead {
    type Target = [Window];
    fn deref(&self) -> &Self::Target {
        &self.list
    }
}
impl<'a> IntoIterator for &'a ChildRead {
    type Item = &'a Window;
    type IntoIter = std::slice::Iter<'a, Window>;
    fn into_iter(self) -> Self::IntoIter {
        self.list.iter()
    }
}

impl Windows {
    /// A read lock owns a snapshot and references to its entries. Mutating the
    /// installed list never changes a running iterator (the §4.1 COW contract).
    pub fn read_children(&mut self, w: Window) -> ChildRead {
        let list = self.get(w).children.clone();
        let generation = self.get(w).child_generation;
        *self.child_readers.entry((w, generation)).or_default() += 1;
        ChildRead {
            owner: w,
            generation,
            list,
        }
    }
    pub fn unlock_children(&mut self, host: &mut impl Host, read: ChildRead) {
        let key = (read.owner, read.generation);
        let count = self.child_readers.get_mut(&key).expect("paired read unlock");
        *count -= 1;
        if *count == 0 {
            self.child_readers.remove(&key);
            if let Some(old) = self.retired_children.remove(&key) {
                for c in old {
                    if self.valid(c) {
                        self.release(host, c);
                    }
                }
            }
        }
    }
    fn write_children(&mut self, w: Window) {
        let key = (w, self.get(w).child_generation);
        if self.child_readers.contains_key(&key) {
            let list = self.get(w).children.clone();
            for &c in &list {
                if self.valid(c) {
                    self.add_ref(c);
                }
            }
            self.retired_children.insert(key, list);
            self.get_mut(w).child_generation += 1;
        }
    }
    pub fn set_parent(&mut self, host: &mut impl Host, w: Window, parent: Option<Window>) -> bool {
        let old = self.get(w).parent;
        if old == parent {
            return true;
        }
        if let (Some(old), Some(_)) = (old, parent) {
            if !self.child_remove(host, old, w) {
                return false;
            }
            if self.get(w).notification == Some(old) {
                self.get_mut(w).notification = None;
            }
        }
        if self.get(w).notification.is_none() {
            self.get_mut(w).notification = parent;
        }
        self.get_mut(w).parent = parent;
        true
    }
    pub fn child_add(&mut self, host: &mut impl Host, p: Window, c: Window) -> bool {
        if !self.valid(c) || self.get(p).children.contains(&c) {
            return false;
        }
        if !self.set_parent(host, c, Some(p)) {
            return false;
        }
        if !host.init(self, c) {
            host.shutdown(self, c);
            self.set_parent(host, c, None);
            return false;
        }
        self.write_children(p);
        self.get_mut(p).children.insert(0, c);
        self.add_ref(c);
        self.calc_absolute(host, c);
        self.set_buffer_recursive(host, c);
        true
    }
    fn drop_manager_references(&mut self, host: &mut impl Host, w: Window) {
        host.event(NativeOp::DropManagerReferences(w));
        if self.focus == Some(w) {
            self.focus = None;
        }
        if self.capture == Some(w) {
            self.capture = None;
        }
        if self.mouse_window == Some(w) {
            self.mouse_window = None;
        }
    }
    pub fn child_remove(&mut self, host: &mut impl Host, p: Window, c: Window) -> bool {
        let valid = self.valid(c);
        let Some(i) = self.get(p).children.iter().position(|&v| v == c) else {
            return false;
        };
        self.write_children(p);
        if valid {
            self.drop_manager_references(host, c);
            self.set_parent(host, c, None);
            self.release(host, c);
        }
        self.get_mut(p).children.remove(i);
        true
    }
    pub fn child_delete(&mut self, host: &mut impl Host, _p: Window, c: Window) {
        self.destroy(host, c);
    }
    pub fn child_delete_absolute(&mut self, host: &mut impl Host, p: Window, c: Window) -> bool {
        let valid = self.valid(c);
        let shutdown = if valid {
            self.add_ref(c);
            host.shutdown(self, c)
        } else {
            false
        };
        let removed = self.child_remove(host, p, c);
        self.destroy(host, c);
        if valid && self.valid(c) {
            self.release(host, c);
        }
        removed && shutdown
    }
    pub fn child_delete_all(&mut self, host: &mut impl Host, p: Window) -> bool {
        let mut ok = true;
        while let Some(&c) = self.get(p).children.first() {
            // WT5: reread the current head. A persistently busy manager never
            // detaches it, so the native loop intentionally need not terminate.
            ok &= host.destroy_window(self, c);
        }
        ok
    }
    pub fn begin_busy(&mut self) {
        self.busy = self.busy.wrapping_add(1);
    }
    pub fn end_busy(&mut self, host: &mut impl Host) {
        self.busy = self.busy.saturating_sub(1);
        if self.busy <= 0 {
            for w in std::mem::take(&mut self.condemned) {
                self.destroy(host, w);
            }
        }
    }
    pub fn destroy(&mut self, host: &mut impl Host, w: Window) -> bool {
        if self.busy > 0 {
            let mut ancestor = Some(w);
            while let Some(a) = ancestor {
                if self.condemned.contains(&a) {
                    return true;
                }
                ancestor = self.windows.get(&a).and_then(|s| s.parent);
            }
            if !self.condemned.contains(&w) {
                self.condemned.push(w);
            }
            return true;
        }
        if !self.valid(w) {
            return false;
        }
        if let Some(p) = self.get(w).parent.filter(|&p| self.valid(p)) {
            return self.child_delete_absolute(host, p, w);
        }
        self.add_ref(w);
        host.shutdown(self, w);
        self.release(host, w);
        true
    }
    pub fn deleting_destructor(&mut self, host: &mut impl Host, w: Window, flags: u32) {
        if !self.valid(w) {
            return;
        }
        self.child_delete_all(host, w);
        self.shutdown(host, w);
        self.drop_manager_references(host, w);
        host.event(NativeOp::Unregister(w));
        self.get_mut(w).registered = false;
        // Direct nondeleting teardown is not a way to preserve live references.
        // Keep allocation storage only when bit zero is clear.
        self.get_mut(w).children.clear();
        if flags & 1 != 0 {
            self.windows.remove(&w);
        }
    }
    pub fn child_from_id(&self, p: Window, id: u32) -> Option<Window> {
        self.get(p)
            .children
            .iter()
            .copied()
            .find(|&c| self.valid(c) && self.get(c).id == id)
    }
    pub fn child_from_id_recursive(&self, w: Window, id: u32) -> Option<Window> {
        for &c in &self.get(w).children {
            if let Some(found) = self.child_from_id_recursive(c, id) {
                return Some(found);
            }
        }
        (self.get(w).id == id).then_some(w)
    }
    pub fn in_parent_chain(&self, w: Window, other: Window) -> bool {
        let mut p = self.get(w).parent;
        while let Some(a) = p {
            if a == other {
                return true;
            }
            p = self.get(a).parent;
        }
        false
    }
    pub fn in_child_chain(&self, w: Window, other: Window) -> bool {
        self.get(w)
            .children
            .iter()
            .any(|&c| c == other || self.in_child_chain(c, other))
    }
    pub fn child_to_front(&mut self, p: Window, c: Window) -> bool {
        if !self.valid(c) || !self.get(p).children.contains(&c) {
            return false;
        }
        if let Some(a) = self.get(p).parent {
            self.child_to_front(a, p);
        }
        if !self.get(c).flag(0x800) {
            return false;
        }
        self.write_children(p);
        let list = &mut self.get_mut(p).children;
        list.retain(|&v| v != c);
        list.insert(0, c);
        true
    }
    pub fn child_to_front_id(&mut self, p: Window, id: u32) -> bool {
        self.child_from_id(p, id).is_some_and(|c| self.child_to_front(p, c))
    }
    pub fn child_to_back(&mut self, p: Window, c: Window) -> bool {
        if !self.valid(c) || !self.get(p).children.contains(&c) || !self.get(c).flag(0x800) {
            return false;
        }
        self.write_children(p);
        let list = &mut self.get_mut(p).children;
        list.retain(|&v| v != c);
        list.push(c);
        true
    }
    pub fn pull_to_front(&mut self, w: Window) -> bool {
        self.get(w).parent.is_some_and(|p| self.child_to_front(p, w))
    }
    pub fn send_to_back(&mut self, w: Window) -> bool {
        self.get(w).parent.is_some_and(|p| self.child_to_back(p, w))
    }
    pub fn child_step(&mut self, p: Window, c: Window, front: bool) -> bool {
        if !self.valid(c) {
            return false;
        }
        self.write_children(p);
        let list = &mut self.get_mut(p).children;
        let Some(i) = list.iter().position(|&v| v == c) else {
            return false;
        };
        let j = if front {
            i.saturating_sub(1)
        } else {
            (i + 1).min(list.len() - 1)
        };
        list.swap(i, j);
        true
    }
    pub fn child_move_relative(&mut self, p: Window, a: Window, b: Window, before: bool) -> bool {
        self.write_children(p);
        let list = &mut self.get_mut(p).children;
        if !list.contains(&a) || !list.contains(&b) {
            return false;
        }
        if a != b {
            list.retain(|&v| v != a);
            let i = list.iter().position(|&v| v == b).expect("present");
            list.insert(i + usize::from(!before), a);
        }
        true
    }
    pub fn move_relative_to(&mut self, w: Window, other: Window, before: bool) -> bool {
        self.get(w)
            .parent
            .is_some_and(|p| self.child_move_relative(p, w, other, before))
    }
    pub fn sort_children(
        &mut self,
        p: Window,
        move_all: bool,
        mut less: impl FnMut(&Self, Window, Window) -> bool,
    ) -> bool {
        if self.get(p).children.len() < 2 {
            return false;
        }
        self.write_children(p);
        let mut i = 1;
        while i < self.get(p).children.len() {
            let list = &self.get(p).children;
            let c = list[i];
            let prev = list[i - 1];
            if (move_all || !self.get(c).flag(0x800)) && less(self, c, prev) {
                let at = list.iter().position(|&n| less(self, c, n)).unwrap_or(list.len());
                self.get_mut(p).children.remove(i);
                self.get_mut(p).children.insert(if at > i { at - 1 } else { at }, c);
                i = 1;
            } else {
                i += 1;
            }
        }
        true
    }
    pub fn sort_children_by_id(&mut self, p: Window, move_all: bool) -> bool {
        self.sort_children(p, move_all, |s, a, b| s.get(a).id < s.get(b).id)
    }
    pub fn enum_children(
        &mut self,
        host: &mut impl Host,
        p: Window,
        iid: u32,
        cb: Option<impl FnMut(&mut Self, Window, u32, Object) -> bool>,
    ) -> bool {
        let Some(mut cb) = cb else {
            return false;
        };
        let list = self.read_children(p);
        let mut ok = !list.is_empty();
        for &c in &list {
            let obj = if iid == IID_WIN {
                Some(c)
            } else {
                host.query_interface(c, iid)
            };
            if let Some(obj) = obj {
                ok &= cb(self, p, self.get(c).id, obj);
                if iid != IID_WIN {
                    host.event(NativeOp::Release(c));
                }
            }
        }
        self.unlock_children(host, list);
        ok
    }
}
