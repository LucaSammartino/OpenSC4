// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl TreeView {
    fn raw_click(&self, id: u32, h: i32, f: &mut impl Foreign) {
        f.deliver(
            self.window(),
            Delivery::Send,
            Message {
                kind: 3,
                words: [id as i32, h, 0, 0],
            },
        );
    }
    pub fn mouse_button(
        &mut self,
        x: i32,
        y: i32,
        right: bool,
        state: &mut ClickState,
        f: &mut impl TreeForeign,
    ) -> Result<bool> {
        let Some(h) = self.cursor_row(x, y)? else {
            return Ok(false);
        };
        self.make_visible(h, f);
        let x = x.wrapping_add(self.horizontal_position).wrapping_sub(self.raw_style[2]);
        let y = y.wrapping_sub(self.raw_style[3]);
        let n = &self.nodes[&h];
        let expander = n.expander_rect;
        let image = n.image_rect;
        let text = n.text_rect;
        let elapsed = f.double_click_interval();
        // The timer's elapsed value is a service operation. The shared state is host-owned.
        let now = f.timer_elapsed(0);
        let double = state.timestamp != 0 && state.handle == h && now.wrapping_sub(state.timestamp) < elapsed;
        if state.timestamp == 0 {
            f.restart_timer(0);
            state.timestamp = 1;
        } else {
            state.timestamp = now;
        }
        state.handle = h;
        if right {
            if contains(image, x, y) || contains(text, x, y) {
                self.select(h, true, f);
                self.raw_click(0x1af5_a13e, h, f);
                self.raw_click(0x1af5_a13f, h, f);
            }
            return Ok(true);
        }
        if contains(expander, x, y) {
            self.toggle_from_input(h, f);
            state.rectangle = expander;
            self.raw_click(0x1af5_a13f, h, f);
            return Ok(true);
        }
        let rectangle = if contains(image, x, y) {
            Some(image)
        } else if contains(text, x, y) {
            Some(text)
        } else {
            None
        };
        if let Some(rect) = rectangle {
            let activation = double && state.rectangle == rect;
            state.rectangle = rect;
            if activation {
                self.emit(ACTIVATION, h, f);
                if self.nodes[&h].force_branch || !self.nodes[&h].children.is_empty() {
                    self.toggle_from_input(h, f);
                    self.raw_click(0x1af5_a13f, h, f);
                    return Ok(true);
                }
            }
            if f.control_down() && self.flags & 0x10 != 0 {
                if self.nodes[&h].selected {
                    self.deselect(h, f);
                } else {
                    self.select(h, false, f);
                }
            } else {
                self.select(h, true, f);
            }
        }
        self.raw_click(0x1af5_a13f, h, f);
        Ok(true)
    }
    pub fn notify_drag(&mut self, x: i32, y: i32, copied: &[i32], f: &mut impl TreeForeign) -> Result<()> {
        let h = self.row_height as u32;
        if h == 0 {
            return Err(Unsupported::Precondition("nonzero drag row height"));
        }
        let adjusted = y.wrapping_add((h >> 2) as i32);
        let target = self.cursor_row(x, adjusted)?;
        let side = (adjusted.wrapping_sub(self.raw_style[3]) as u32) % h < (h >> 1);
        if self.selection.first().copied() == target && self.insertion_side == side {
            return Ok(());
        }
        let mut accepted = target;
        let mut ancestor = target;
        while let Some(a) = ancestor {
            if copied.contains(&a) {
                accepted = None;
                break;
            }
            ancestor = self.nodes[&a].parent;
        }
        self.insertion_side = side;
        if let Some(target) = accepted {
            self.select(target, true, f);
        } else {
            self.clear_selection(f);
        }
        f.invalidate(self.window());
        Ok(())
    }
    pub fn mouse_move(&mut self, x: i32, y: i32, bits: u32, f: &mut impl TreeForeign) -> Result<bool> {
        if self.selection.is_empty() || bits & 0x10 == 0 {
            return Ok(true);
        }
        if self.cursor_row(x, y)?.is_some_and(|h| self.nodes[&h].selected) {
            return Ok(true);
        }
        let mut accept = 0;
        if !f.drag_request(self.window(), &mut accept) || accept == 0 {
            return Ok(true);
        }
        let copied = self.selection.clone();
        let (mut left, mut right) = (0i32, 0i32);
        for h in &copied {
            let n = &self.nodes[h];
            for rect in [n.expander_rect, n.image_rect, n.text_rect] {
                if rect[2] > rect[0] && rect[3] > rect[1] {
                    left = left.min(rect[0]);
                    right = right.max(rect[2]);
                }
            }
        }
        let size = [
            right.wrapping_sub(left) as u32,
            (copied.len() as u32).wrapping_mul(self.row_height as u32),
        ];
        let Some(buffer) = f.create(Resource::Buffer) else {
            return Ok(true);
        };
        if !f.initialize_image(buffer, usize::MAX, size) {
            f.release(buffer);
            return Ok(true);
        }
        if f.access_mode(buffer, 0x8001, true) {
            f.draw(Draw::Fill {
                rect: [0, 0, size[0] as i32, size[1] as i32],
                color: u32::MAX,
            });
            f.access_mode(buffer, 0x8001, false);
        }
        self.clear_selection(f);
        let old_buffer = self.working_buffer.replace(buffer);
        for (row, h) in copied.iter().enumerate() {
            let rect = self.nodes[h].text_rect;
            let y = (row as i32).wrapping_mul(self.row_height);
            let text = [
                rect[0].wrapping_sub(left),
                y,
                rect[2].wrapping_sub(left),
                y.wrapping_add(self.row_height),
            ];
            self.render_item(*h, text, [0; 4], [0; 4], f)?;
        }
        self.working_buffer = old_buffer;
        let session = DragSession {
            nodes: copied.clone(),
            buffer,
            size,
        };
        f.run_drag(self, &session);
        if let Some(target) = self.selection.first().copied() {
            let pair = if !self.insertion_side {
                (target, -2)
            } else {
                let parent = self.nodes[&target].parent.unwrap_or(-1);
                let previous = self
                    .nodes
                    .get(&parent)
                    .and_then(|n| {
                        n.children
                            .iter()
                            .position(|h| *h == target)
                            .and_then(|i| i.checked_sub(1).map(|i| n.children[i]))
                    })
                    .unwrap_or(-2);
                (parent, previous)
            };
            for h in &copied {
                self.select(*h, false, f);
            }
            let a = f.convert_payload(self.window(), pair.0);
            let b = f.convert_payload(self.window(), pair.1);
            f.deliver(
                self.window(),
                Delivery::Send,
                Message {
                    kind: 3,
                    words: [0x1af5_a141, a, b, 0],
                },
            );
        }
        self.insertion_side = false;
        f.invalidate(self.window());
        f.dispose_surface(buffer);
        f.release(buffer);
        Ok(true)
    }
}

/// Borrowed drag helper state; the enclosing runner owns its lifetime.
#[derive(Debug)]
pub struct DragHelper<'a> {
    pub source: Handle,
    pub session: &'a DragSession,
    pub position: [i32; 2],
    pub captured: bool,
}
impl DragHelper<'_> {
    pub fn init(&mut self, cursor: [i32; 2], f: &mut impl Foreign) -> bool {
        if !f.base_init(self.source) {
            return false;
        }
        self.position = cursor;
        f.capture(self.source, true);
        self.captured = true;
        true
    }
    pub fn move_to(
        &mut self,
        tree: &mut TreeView,
        cursor: [i32; 2],
        source_cursor: [i32; 2],
        f: &mut impl TreeForeign,
    ) -> Result<()> {
        self.position = cursor;
        f.invalidate(self.source);
        tree.notify_drag(source_cursor[0], source_cursor[1], &self.session.nodes, f)
    }
    pub fn paint(&self, f: &mut impl Foreign) {
        f.draw(Draw::Blit {
            source: self.session.buffer,
            rect: [0, 0, self.session.size[0] as i32, self.session.size[1] as i32],
        });
    }
    pub fn mouse_up(&self, f: &mut impl Foreign) -> bool {
        f.drop_complete(self.source, 0xf, 0);
        true
    }
    pub fn shutdown(&mut self, f: &mut impl Foreign) {
        f.capture(self.source, false);
        self.captured = false;
        f.base_shutdown(self.source);
    }
}
