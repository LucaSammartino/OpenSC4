// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

impl TreeView {
    /// RecalculateAll's owner work, given base-window geometry. The bar
    /// requirements use content overflow, solving both coupled axes first.
    pub fn recalculate_all(&mut self, area: Rect, f: &mut impl TreeForeign) -> Result<()> {
        self.require_live_root()?;
        self.recalculate();
        self.layout_area = Some(area);
        let content = [
            self.content_width(f),
            (self.visible.len() as u32).wrapping_mul(self.row_height as u32),
        ];
        let thickness = f.tree_bar_thickness(self.window());
        let client = [
            (area[2].wrapping_sub(area[0]) as u32).wrapping_sub((self.raw_style[2] as u32).wrapping_mul(2)),
            (area[3].wrapping_sub(area[1]) as u32).wrapping_sub((self.raw_style[3] as u32).wrapping_mul(2)),
        ];
        // SPEC-GAP: exact buffer/bar allocation policy is silent; use content size and overflow.
        let mut need = [false; 2];
        for _ in 0..3 {
            need = [
                content[0] > client[0].wrapping_sub(if need[1] { thickness[1] } else { 0 }),
                content[1] > client[1].wrapping_sub(if need[0] { thickness[0] } else { 0 }),
            ];
        }
        let window = self.window();
        for (axis, slot) in [&mut self.horizontal_bar, &mut self.vertical_bar]
            .into_iter()
            .enumerate()
        {
            if need[axis] && slot.is_none() {
                *slot = f.create_tree_bar(window, axis == 0);
            } else if !need[axis] {
                if let Some(bar) = slot.take() {
                    f.release(bar);
                }
            }
        }
        if let Some(buffer) = self.working_buffer {
            f.resize_tree_buffer(buffer, content);
            self.layout_viewport(area, f.buffer_size(buffer), thickness)?;
        }
        for (axis, bar) in [self.horizontal_bar, self.vertical_bar].into_iter().enumerate() {
            if let Some(bar) = bar {
                let (maximum, page, value) = if axis == 0 {
                    (
                        content[0].wrapping_sub(self.viewport[2] as u32),
                        self.viewport[2] as u32,
                        self.horizontal_position,
                    )
                } else {
                    (
                        (self.visible.len() as u32).saturating_sub(self.viewport_rows),
                        self.viewport_rows,
                        self.first,
                    )
                };
                f.bar_range(bar, maximum as i32, page as i32, value);
                let rect = if axis == 0 {
                    [area[0], area[3].wrapping_sub(thickness[0] as i32), area[2], area[3]]
                } else {
                    [area[2].wrapping_sub(thickness[1] as i32), area[1], area[2], area[3]]
                };
                f.place_bar(bar, rect);
            }
        }
        Ok(())
    }
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
                let crop = self.image_rectangles[index as usize];
                let width = crop[2].wrapping_sub(crop[0]) as u32;
                let height = crop[3].wrapping_sub(crop[1]) as u32;
                // Native centers with logical halving, including undersized reservations.
                let x = image[0].wrapping_add((self.image_width.wrapping_sub(width) >> 1) as i32);
                let y = image[1].wrapping_add(((self.row_height as u32).wrapping_sub(height) >> 1) as i32);
                let destination = [x, y, x.wrapping_add(width as i32), y.wrapping_add(height as i32)];
                if f.access_mode(buffer, 0x8010, true) {
                    f.draw_subimage(source, crop, destination);
                    self.nodes.get_mut(&h).unwrap().image_rect = destination;
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
    pub(super) fn connectors(&mut self, h: i32, x: i32, y: i32, buffer: Handle, f: &mut impl TreeForeign) -> Rect {
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
                        let c = if self.flags & 4 != 0 {
                            self.image_width.wrapping_sub(b as u32) >> 1
                        } else {
                            0
                        } as i32;
                        let ax = b
                            .wrapping_add(self.raw_style[1])
                            .wrapping_add(c)
                            .wrapping_mul(self.depth(a) as i32)
                            .wrapping_add(c)
                            .wrapping_add(q);
                        self.stroke(buffer, [ax, y, ax, y.wrapping_add(height)], f);
                    }
                }
            }
            ancestor = node.parent;
        }
        let last = match n.parent {
            Some(p) => self.nodes[&p].children.last() == Some(&h),
            None => true,
        };
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
        if self.layout_dirty || self.layout_area != Some(area) {
            self.recalculate_all(area, f)?;
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
        self.layout_viewport(area, size, f.tree_bar_thickness(self.window()))?;
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
        f.blit_viewport(
            buffer,
            [
                self.horizontal_position,
                0,
                self.horizontal_position.wrapping_add(self.viewport[2]),
                self.viewport[3],
            ],
            [
                self.viewport[0],
                self.viewport[1],
                self.viewport[0].wrapping_add(self.viewport[2]),
                self.viewport[1].wrapping_add(self.viewport[3]),
            ],
        );
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
