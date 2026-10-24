// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeDescriptor {
    pub text: Vec<u8>,
    pub data: u32,
    pub images: [i32; 4],
    pub font: Option<Handle>,
    pub colors: [u32; 4],
    pub expanded: bool,
    pub force_branch: bool,
    pub visible: bool,
    pub selected: bool,
    pub children: Vec<NodeHandle>,
}

/// One host-owned instance is shared across controls, as the native static state is.
#[derive(Default, Debug)]
pub struct ClickState {
    pub handle: i32,
    pub timestamp: u32,
    pub rectangle: Rect,
}
#[derive(Debug)]
pub struct DragSession {
    pub nodes: Vec<NodeHandle>,
    pub buffer: Handle,
    pub size: [u32; 2],
}

impl TreeView {
    pub(super) fn toggle_from_input(&mut self, h: i32, f: &mut impl Foreign) {
        let expanded = !self.nodes[&h].expanded;
        self.set_expanded(h, expanded, f);
        if !expanded {
            let mut pending = self.nodes[&h].children.clone();
            while let Some(h) = pending.pop() {
                let n = self.nodes.get_mut(&h).unwrap();
                n.selected = false;
                pending.extend_from_slice(&n.children);
            }
            self.selection_dirty = true;
        }
    }
    pub fn set_style_bits(&mut self, mask: u32, enabled: bool, f: &mut impl Foreign) {
        let old = self.raw_style[4] as u32;
        if (old & mask != 0) == enabled {
            return;
        }
        self.raw_style[4] = if enabled { old | mask } else { old & !mask } as i32;
        self.dirty(f);
    }
    pub fn style_bits(&self, mask: u32) -> bool {
        self.raw_style[4] as u32 & mask != 0
    }
    pub fn set_margins(&mut self, x: u32, y: u32, f: &mut impl Foreign) {
        if self.raw_style[2..4] != [x as i32, y as i32] {
            self.raw_style[2] = x as i32;
            self.raw_style[3] = y as i32;
            self.dirty(f);
        }
    }
    pub fn set_spacing(&mut self, value: u32, f: &mut impl Foreign) {
        if self.raw_style[1] != value as i32 {
            self.raw_style[1] = value as i32;
            self.dirty(f);
        }
    }
    pub fn set_row_height(&mut self, value: u32, f: &mut impl Foreign) {
        if self.row_height != value as i32 {
            self.row_height = value as i32;
            self.dirty(f);
        }
    }
    pub fn set_expander_size(&mut self, value: u32, f: &mut impl Foreign) {
        let value = value.min(self.flags) as i32;
        if self.raw_style[0] != value {
            self.raw_style[0] = value;
            self.dirty(f);
        }
    }
    pub fn set_base_color(&mut self, value: u32, f: &mut impl TreeForeign) {
        let value = f.convert_color(self.window(), value);
        if self.base_color != value {
            self.base_color = value;
            self.dirty(f);
        }
    }
    pub fn set_selection_color(&mut self, value: u32, f: &mut impl Foreign) {
        if self.colors[3] != value {
            self.colors[3] = value;
            if !self.selection.is_empty() {
                self.dirty(f);
            }
        }
    }
    pub fn set_default_font(&mut self, font: Option<Handle>, f: &mut impl Foreign) {
        if self.default_font != font {
            replace_ref(&mut self.default_font, font, f);
        }
    }
    pub fn default_color(&self, index: i32) -> Result<u32> {
        if index >= 4 {
            return Ok(0);
        } // Negative native indices alias preceding object words.
        if index < 0 {
            return Err(Unsupported::Precondition(
                "default-color index aliases raw object storage",
            ));
        }
        Ok(self.colors[index as usize])
    }
    pub fn set_default_color(&mut self, index: i32, value: u32) -> Result<()> {
        if index < 0 {
            return Err(Unsupported::Precondition(
                "default-color index aliases raw object storage",
            ));
        }
        if index < 4 {
            self.colors[index as usize] = value;
        }
        Ok(())
    }
    pub fn descriptor(&mut self, h: i32) -> Option<NodeDescriptor> {
        let visible = self.get_visible(h);
        let n = self.nodes.get(&h)?;
        Some(NodeDescriptor {
            text: n.text.clone(),
            data: n.data,
            images: n.images,
            font: n.font,
            colors: n.colors,
            expanded: n.expanded,
            force_branch: n.force_branch,
            visible,
            selected: n.selected,
            children: n.children.clone(),
        })
    }
    pub fn apply_descriptor(&mut self, h: i32, value: NodeDescriptor, f: &mut impl TreeForeign) -> bool {
        if !self.nodes.contains_key(&h) {
            return false;
        }
        self.set_expanded(h, value.expanded, f);
        if value.visible {
            self.make_visible(h, f);
        }
        if self.nodes[&h].selected != value.selected {
            if value.selected {
                self.select(h, false, f);
            } else {
                self.deselect(h, f);
            }
        }
        self.set_font(h, value.font, f);
        let n = self.nodes.get_mut(&h).unwrap();
        n.text = value.text;
        n.data = value.data;
        n.images = value.images;
        n.colors = value.colors;
        self.dirty(f);
        true
    }
    pub fn text(&self, h: i32) -> &[u8] {
        self.nodes
            .get(&h)
            .map_or(self.fallback_text.as_slice(), |n| n.text.as_slice())
    }
    pub fn set_text(&mut self, h: i32, text: Vec<u8>, f: &mut impl Foreign) -> bool {
        let Some(n) = self.nodes.get_mut(&h) else {
            return false;
        };
        n.text = text;
        self.dirty(f);
        true
    }
    pub fn data(&self, h: i32) -> u32 {
        self.nodes.get(&h).map_or(0, |n| n.data)
    }
    pub fn set_data(&mut self, h: i32, data: u32) -> bool {
        let Some(n) = self.nodes.get_mut(&h) else {
            return false;
        };
        n.data = data;
        true
    }
    pub fn node_image(&self, h: i32, index: i32) -> i32 {
        self.nodes
            .get(&h)
            .and_then(|n| n.images.get(index as usize))
            .copied()
            .unwrap_or(-1)
    }
    pub fn set_node_image(&mut self, h: i32, index: i32, value: i32, f: &mut impl Foreign) -> bool {
        let Some(slot) = self.nodes.get_mut(&h).and_then(|n| n.images.get_mut(index as usize)) else {
            return false;
        };
        *slot = value;
        self.dirty(f);
        true
    }
    pub fn node_color(&self, h: i32, index: i32) -> u32 {
        let order = [0, 2, 1, 3];
        self.nodes
            .get(&h)
            .and_then(|n| order.get(index as usize).map(|i| n.colors[*i]))
            .unwrap_or(0)
    }
    pub fn set_node_color(&mut self, h: i32, index: i32, value: u32, f: &mut impl Foreign) -> bool {
        let order = [0, 2, 1, 3];
        let Some(slot) = self
            .nodes
            .get_mut(&h)
            .and_then(|n| order.get(index as usize).map(|i| &mut n.colors[*i]))
        else {
            return false;
        };
        *slot = value;
        self.dirty(f);
        true
    }
    pub fn children(&self, h: i32, output: Option<&mut Vec<i32>>) -> bool {
        let (Some(n), Some(out)) = (self.nodes.get(&h), output) else {
            return false;
        };
        out.clear();
        out.extend_from_slice(&n.children);
        true
    }
    pub fn selected_handles(&self) -> &[i32] {
        &self.selection
    }
    pub fn first_selected(&self) -> i32 {
        self.selection.first().copied().unwrap_or(-1)
    }
    pub fn move_node(&mut self, h: i32, parent: i32, reference: i32, f: &mut impl Foreign) {
        let Some(old) = self.nodes.get(&h).and_then(|n| n.parent) else {
            return;
        };
        if !self.nodes.contains_key(&parent) {
            return;
        }
        self.nodes.get_mut(&old).unwrap().children.retain(|n| *n != h);
        let children = &mut self.nodes.get_mut(&parent).unwrap().children;
        let position = match reference {
            -3 => children.len(),
            r if r > 0 => children.iter().position(|n| *n == r).map_or(0, |p| p + 1),
            _ => 0,
        };
        children.insert(position, h);
        self.nodes.get_mut(&h).unwrap().parent = Some(parent);
        self.dirty(f);
    }
    pub fn remove_node(&mut self, h: i32, f: &mut impl Foreign) -> bool {
        let Some(n) = self.nodes.get(&h) else {
            return false;
        };
        let children = n.children.clone();
        let parent = n.parent;
        for child in children {
            self.remove_node(child, f);
        }
        self.deselect(h, f);
        if Some(h) != self.root {
            if let Some(p) = parent {
                if let Some(n) = self.nodes.get_mut(&p) {
                    n.children.retain(|v| *v != h);
                }
            }
            self.nodes.remove(&h);
        } else {
            self.nodes.get_mut(&h).unwrap().children.clear();
        }
        self.dirty(f);
        true
    }
    pub fn set_image(&mut self, index: i32, buffer: Option<Handle>, mut rect: Rect, f: &mut impl TreeForeign) -> bool {
        if !(0..4).contains(&index) {
            return false;
        }
        let index = index as usize;
        self.install_image(index, buffer, &mut rect, f);
        true
    }
    fn install_image(&mut self, index: usize, buffer: Option<Handle>, rect: &mut Rect, f: &mut impl TreeForeign) {
        self.images.resize(self.images.len().max(index + 1), None);
        self.image_rectangles
            .resize(self.image_rectangles.len().max(index + 1), [0; 4]);
        replace_ref(&mut self.images[index], buffer, f);
        if let Some(b) = buffer {
            if rect[2] <= rect[0] || rect[3] <= rect[1] {
                let [w, h] = f.buffer_size(b);
                *rect = [0, 0, w as i32, h as i32];
            }
        }
        self.image_rectangles[index] = *rect;
        self.image_width = self.image_width.max(rect[2].wrapping_sub(rect[0]) as u32);
        let height = rect[3].wrapping_sub(rect[1]) as u32;
        if height > self.row_height as u32 {
            self.row_height = height.wrapping_add(2) as i32;
        }
    }
    pub fn append_image(&mut self, buffer: Option<Handle>, mut rect: Rect, f: &mut impl TreeForeign) -> i32 {
        let Some(b) = buffer else {
            return -1;
        };
        let i = self.images.len();
        self.install_image(i, Some(b), &mut rect, f);
        i as i32
    }
    pub fn image(&self, index: u32) -> Option<(Option<Handle>, Rect)> {
        Some((
            *self.images.get(index as usize)?,
            *self.image_rectangles.get(index as usize)?,
        ))
    }
    pub fn truncate_images(&mut self, f: &mut impl Foreign) {
        if self.images.len() > 4 {
            for b in self.images.drain(4..).flatten() {
                f.release(b);
            }
            self.image_rectangles.truncate(4);
        }
    }
    pub fn initialize_images(&mut self, f: &mut impl TreeForeign) {
        if !f.graphics_available() || !f.manager_available() {
            return;
        }
        for i in 0..4 {
            if self.images.get(i).copied().flatten().is_some() {
                continue;
            }
            let Some(buffer) = f.create(Resource::Buffer) else {
                continue;
            };
            let color = if i % 2 == 0 { 0xff } else { 0xff0000 };
            let background = f.default_color(0x0896_3f6c).unwrap_or(0);
            if f.initialize_image(buffer, i, [9, 9]) {
                if f.access_mode(buffer, 0x8001, true) {
                    f.draw(Draw::Fill {
                        rect: [0, 0, 9, 9],
                        color: if i < 2 { color } else { background },
                    });
                    f.access_mode(buffer, 0x8001, false);
                }
                if i >= 2 {
                    f.triangle(buffer, color, background);
                }
            }
            self.set_image(i as i32, Some(buffer), [0, 0, 9, 9], f);
            f.release(buffer);
        }
    }
}
