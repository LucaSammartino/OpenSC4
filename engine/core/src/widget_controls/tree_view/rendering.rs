// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl TreeView {
    pub fn layout_viewport(&mut self, area: Rect, buffer_size: [u32; 2], bar_thickness: [u32; 2]) -> Result<()> {
        let mx = self.raw_style[2] as u32;
        let my = self.raw_style[3] as u32;
        let width = (area[2].wrapping_sub(area[0]) as u32)
            .wrapping_sub(mx.wrapping_mul(2))
            .wrapping_sub(if self.vertical_bar.is_some() {
                bar_thickness[1]
            } else {
                0
            })
            .min(buffer_size[0]);
        let height = (area[3].wrapping_sub(area[1]) as u32)
            .wrapping_sub(my.wrapping_mul(2))
            .wrapping_sub(if self.horizontal_bar.is_some() {
                bar_thickness[0]
            } else {
                0
            })
            .min(buffer_size[1]);
        let h = self.row_height as u32;
        if h == 0 {
            return Err(Unsupported::Precondition("nonzero viewport row height"));
        }
        self.viewport = [
            area[0].wrapping_add(mx as i32),
            area[1].wrapping_add(my as i32),
            width as i32,
            height as i32,
        ];
        self.viewport_rows = height / h;
        self.page_count = (f64::from(height) / f64::from(h)).ceil() as u32;
        Ok(())
    }
    pub fn cursor_row(&self, x: i32, y: i32) -> Result<Option<i32>> {
        let x = x.wrapping_sub(self.raw_style[2]);
        let y = y.wrapping_sub(self.raw_style[3]);
        if x < 0 || y < 0 || x as u32 > self.viewport[2] as u32 || y as u32 > self.viewport[3] as u32 {
            return Ok(None);
        }
        let h = self.row_height as u32;
        if h == 0 {
            return Err(Unsupported::Precondition("nonzero cursor row height"));
        }
        let offset = y as u32 / h;
        if offset > self.page_count.wrapping_sub(1) {
            return Ok(None);
        }
        let index = (self.first as u32).wrapping_add(offset) as usize;
        self.visible
            .get(index)
            .copied()
            .map(Some)
            .ok_or(Unsupported::Precondition(
                "cursor row must be backed by cached sequence",
            ))
    }
    pub fn depth(&self, h: i32) -> u32 {
        let mut depth = 0;
        let mut parent = self.nodes[&h].parent;
        while let Some(p) = parent {
            if Some(p) != self.root {
                depth += 1;
            }
            parent = self.nodes[&p].parent;
        }
        depth
    }
    pub fn content_width(&self, f: &mut impl TreeForeign) -> u32 {
        let b = self.raw_style[0] as u32;
        let g = self.raw_style[1] as u32;
        let i = self.image_width;
        let c = if self.flags & 4 != 0 && i > b { (i - b) / 2 } else { 0 };
        let mut max = 0;
        for h in &self.visible {
            let n = &self.nodes[h];
            let contribution = if Some(*h) == self.root {
                0
            } else {
                b.wrapping_add(g).wrapping_add(c).wrapping_mul(self.depth(*h) + 1)
            };
            let width = n.font.map_or(0, |font| f.measure_text(font, &n.text) as u32);
            max = max.max(
                contribution
                    .wrapping_add(g)
                    .wrapping_add(i)
                    .wrapping_add(3)
                    .wrapping_add(width),
            );
        }
        max
    }
    pub(super) fn render_item(
        &mut self,
        h: i32,
        text: Rect,
        image: Rect,
        expander: Rect,
        f: &mut impl TreeForeign,
    ) -> Result<bool> {
        let Some(n) = self.nodes.get(&h) else {
            return Ok(false);
        };
        let selected = n.selected;
        let font = n.font;
        let colors = n.colors;
        let bytes = n.text.clone();
        let open = n.expanded && !n.children.is_empty();
        let ordinary = if open { 2 } else { 0 };
        let selected_slot = ordinary + 1;
        let index = if selected && n.images[selected_slot] != -1 {
            n.images[selected_slot]
        } else {
            n.images[ordinary]
        };
        let n = self.nodes.get_mut(&h).unwrap();
        n.text_rect = text;
        n.expander_rect = expander;
        n.image_rect = [0; 4];
        if self.flags & 4 != 0 && index >= 0 {
            let image_buffer = self
                .images
                .get(index as usize)
                .copied()
                .ok_or(Unsupported::Precondition("image index within retained image vector"))?;
            if let (Some(buffer), Some(source)) = (self.working_buffer, image_buffer) {
                if f.access_mode(buffer, 0x8010, true) {
                    f.draw_image(source, image);
                    self.nodes.get_mut(&h).unwrap().image_rect = image;
                    f.access_mode(buffer, 0x8010, false);
                }
            }
        }
        if let Some(font) = font {
            if let Some(buffer) = self.working_buffer {
                if f.access_mode(buffer, 0x8001, true) {
                    let color = if selected {
                        if self.insertion_side {
                            average(self.colors[1], self.colors[3])
                        } else {
                            self.colors[3]
                        }
                    } else {
                        colors[2]
                    };
                    f.draw(Draw::Fill { rect: text, color });
                    if selected && self.insertion_side {
                        let width = f.buffer_size(buffer)[0] as i32;
                        let bottom = text[3].wrapping_add(text[1].wrapping_mul(3)).wrapping_add(2) >> 2;
                        f.draw(Draw::Fill {
                            rect: [0, text[1], width, bottom],
                            color: self.colors[3],
                        });
                    }
                    f.access_mode(buffer, 0x8001, false);
                }
            }
            f.draw(Draw::Text {
                font,
                text: bytes,
                rect: [text[0].wrapping_add(3), text[1].wrapping_add(1), text[2], text[3]],
                color: colors[usize::from(selected)],
            });
        }
        Ok(true)
    }
    fn stroke(&self, buffer: Handle, points: Rect, f: &mut impl TreeForeign) {
        if f.access_mode(buffer, 0x8004, true) {
            f.draw(Draw::Line {
                points,
                color: self.line_style[0],
                style: self.line_style[1],
            });
            f.access_mode(buffer, 0x8004, false);
        }
    }
    fn connectors(&mut self, h: i32, x: i32, y: i32, buffer: Handle, f: &mut impl TreeForeign) -> Rect {
        if self.flags & 8 == 0 {
            return [0; 4];
        }
        let b = self.raw_style[0];
        let q = ((b as u32) >> 1) as i32;
        let height = self.row_height;
        let e = y.wrapping_add(height.wrapping_sub(b) >> 1);
        let center = e.wrapping_add(q);
        let n = &self.nodes[&h];
        let branch = !n.children.is_empty() || n.force_branch;
        let expanded = n.expanded;
        let mut ancestor = n.parent;
        while let Some(a) = ancestor {
            let node = &self.nodes[&a];
            if Some(a) != self.root {
                if let Some(p) = node.parent {
                    if self.nodes[&p].children.last() != Some(&a) {
                        let ax = (b.wrapping_add(self.raw_style[1]))
                            .wrapping_mul(self.depth(a) as i32)
                            .wrapping_add(q);
                        self.stroke(buffer, [ax, y, ax, y.wrapping_add(height)], f);
                    }
                }
            }
            ancestor = node.parent;
        }
        let last = n.parent.map_or(true, |p| self.nodes[&p].children.last() == Some(&h));
        let mut rect = [0; 4];
        if branch {
            let outline = [x, e, x.wrapping_add(b), e.wrapping_add(b)];
            if f.access_mode(buffer, 0x8008, true) {
                f.draw(Draw::Outline {
                    rect: outline,
                    colors: [self.line_style[0]; 2],
                    three_d: false,
                });
                f.access_mode(buffer, 0x8008, false);
                rect = outline;
            }
            let length = q - i32::from(b & 1 == q & 1);
            let offset = ((b.wrapping_sub(length) as u32) >> 1) as i32;
            self.stroke(
                buffer,
                [
                    x.wrapping_add(offset),
                    center,
                    x.wrapping_add(offset).wrapping_add(length),
                    center,
                ],
                f,
            );
            if !expanded {
                self.stroke(
                    buffer,
                    [
                        x.wrapping_add(q),
                        e.wrapping_add(offset),
                        x.wrapping_add(q),
                        e.wrapping_add(offset).wrapping_add(length),
                    ],
                    f,
                );
            }
            if expanded {
                self.stroke(
                    buffer,
                    [
                        x.wrapping_add(q),
                        e.wrapping_add(b),
                        x.wrapping_add(q),
                        y.wrapping_add(height),
                    ],
                    f,
                );
            }
        } else {
            let extension = self.raw_style[1].wrapping_add(if self.flags & 4 != 0 { 0 } else { b });
            self.stroke(
                buffer,
                [
                    x.wrapping_add(q),
                    center,
                    x.wrapping_add(b).wrapping_add(extension),
                    center,
                ],
                f,
            );
            if !(self.nodes[&h].visible_index == Some(0) && self.flags & 2 == 0) {
                self.stroke(buffer, [x.wrapping_add(q), y, x.wrapping_add(q), center], f);
            }
            if !last {
                self.stroke(
                    buffer,
                    [x.wrapping_add(q), center, x.wrapping_add(q), y.wrapping_add(height)],
                    f,
                );
            }
        }
        rect
    }
    pub(super) fn paint_tree(&mut self, area: Rect, f: &mut impl TreeForeign) -> Result<()> {
        if self.layout_dirty {
            self.recalculate();
        }
        if self.flags & 0x20 != 0 {
            f.draw(Draw::Fill {
                rect: area,
                color: self.base_color,
            });
        }
        let Some(buffer) = self.working_buffer else {
            return Ok(());
        };
        let size = f.buffer_size(buffer);
        self.layout_viewport(area, size, [0, 0])?;
        if self.visible.is_empty() {
            return Ok(());
        }
        if self.selection_dirty {
            let before = self.selection.len();
            self.selection.retain(|h| self.nodes.get(h).is_some_and(|n| n.selected));
            self.selection_dirty = false;
            if self.selection.len() != before {
                self.emit(SELECTION, 0, f);
            }
        }
        if self.flags & 0x20 != 0 && f.access_mode(buffer, 0x8001, true) {
            f.draw(Draw::Fill {
                rect: [0, 0, size[0] as i32, size[1] as i32],
                color: self.base_color,
            });
            f.access_mode(buffer, 0x8001, false);
        }
        let b = self.raw_style[0] as u32;
        let g = self.raw_style[1] as u32;
        let c = if self.flags & 4 != 0 {
            self.image_width.wrapping_sub(b) >> 1
        } else {
            0
        };
        for h in self.visible.clone() {
            let row = self.nodes[&h].visible_index.unwrap() as u32;
            if row < self.first as u32 || row >= (self.first as u32).wrapping_add(self.page_count) {
                continue;
            }
            {
                let n = self.nodes.get_mut(&h).unwrap();
                n.text_rect = [0; 4];
                n.image_rect = [0; 4];
                n.expander_rect = [0; 4];
            }
            let root = Some(h) == self.root;
            let x = if root {
                0
            } else {
                b.wrapping_add(g)
                    .wrapping_add(c)
                    .wrapping_mul(self.depth(h))
                    .wrapping_add(c)
            } as i32;
            let y = if root {
                0
            } else {
                row.wrapping_sub(self.first as u32).wrapping_mul(self.row_height as u32) as i32
            };
            let a = if root {
                0
            } else {
                x.wrapping_add(b as i32).wrapping_add(g as i32)
            };
            let n = &self.nodes[&h];
            let width = n.font.map_or(0, |font| f.measure_text(font, &n.text));
            let left = a.wrapping_add(g as i32).wrapping_add(self.image_width as i32);
            let text = [
                left,
                y,
                left.wrapping_add(3).wrapping_add(width),
                y.wrapping_add(self.row_height),
            ];
            let expander = self.connectors(h, x, y, buffer, f);
            let image = [
                a,
                y,
                a.wrapping_add(self.image_width as i32),
                y.wrapping_add(self.row_height),
            ];
            self.render_item(h, text, image, expander, f)?;
        }
        f.draw(Draw::Blit {
            source: buffer,
            rect: [
                self.horizontal_position,
                0,
                self.horizontal_position.wrapping_add(self.viewport[2]),
                self.viewport[3],
            ],
        });
        Ok(())
    }
}
fn average(a: u32, b: u32) -> u32 {
    let mut result = 0;
    for shift in [0, 8, 16] {
        result |= ((((a >> shift) & 255) + ((b >> shift) & 255)) >> 1) << shift;
    }
    result
}
