// SPDX-License-Identifier: GPL-3.0-or-later
//! cGZWinTreeView, widget-controls §6. Node handles, visible rows and selection
//! positions are distinct. Native memory/lifetime preconditions stay explicit.
use super::*;
#[path = "tree_view/adapters.rs"]
mod adapters;
#[path = "tree_view/input.rs"]
mod input;
#[path = "tree_view/rendering.rs"]
mod rendering;
#[path = "tree_view/settled.rs"]
mod settled;
pub use input::DragHelper;
pub use settled::{ClickState, DragSession, NodeDescriptor};
use std::collections::BTreeMap;

pub const IID: u32 = 0x3ae8_bad2;
pub const WINDOW_OFFSET: u64 = 4;
pub const PRIMARY_SLOTS: usize = 60;
pub const WINDOW_SLOTS: usize = 151;
pub const SELECTION: u32 = 0x1af5_a13a;
pub const ACTIVATION: u32 = 0x1af5_a13b;
pub const EXPAND: u32 = 0x1af5_a13c;
pub const COLLAPSE: u32 = 0x1af5_a13d;

pub type NodeHandle = i32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub parent: NodeHandle,
    pub reference: i32,
    pub text: Vec<u8>,
    pub data: u32,
    pub images: [i32; 4],
    pub font: Option<Handle>,
    pub colors: [u32; 4],
    pub expanded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub handle: NodeHandle,
    pub reference: i32,
    pub text: Vec<u8>,
    pub data: u32,
    pub images: [i32; 4],
    pub font: Option<Handle>,
    pub colors: [u32; 4],
    pub expanded: bool,
    pub force_branch: bool,
    pub selected: bool,
    pub flattened: bool,
    pub visible_index: Option<usize>,
    pub parent: Option<NodeHandle>,
    pub children: Vec<NodeHandle>,
    pub text_rect: Rect,
    pub image_rect: Rect,
    pub expander_rect: Rect,
}

/// Only individual graphics/font resource operations cross this boundary.
pub trait TreeForeign: Foreign {
    fn convert_color(&mut self, window: Handle, color: u32) -> u32;
    fn initialize_buffer(&mut self, buffer: Handle) -> bool;
    fn buffer_access(&mut self, buffer: Handle, acquire: bool) -> bool;
    fn measure_text(&mut self, font: Handle, text: &[u8]) -> i32;
    fn draw_image(&mut self, image: Handle, rect: Rect);
    fn set_bar_value(&mut self, bar: Handle, value: i32);
    fn buffer_size(&mut self, _buffer: Handle) -> [u32; 2] {
        [4096, 4096]
    }
    fn access_mode(&mut self, buffer: Handle, _mode: u32, take: bool) -> bool {
        self.buffer_access(buffer, take)
    }
    fn graphics_available(&mut self) -> bool {
        true
    }
    fn manager_available(&mut self) -> bool {
        true
    }
    /// Base-window geometry and scrollbar thickness supplied by the host.
    fn tree_area(&mut self, _window: Handle) -> Option<Rect> {
        None
    }
    fn tree_bar_thickness(&mut self, _window: Handle) -> [u32; 2] {
        [0, 0]
    }
    fn resize_tree_buffer(&mut self, _buffer: Handle, _size: [u32; 2]) {}
    fn create_tree_bar(&mut self, window: Handle, _horizontal: bool) -> Option<Handle> {
        self.create_bar(window)
    }
    fn initialize_image(&mut self, buffer: Handle, _slot: usize, _size: [u32; 2]) -> bool {
        self.initialize_buffer(buffer)
    }
    fn triangle(&mut self, _buffer: Handle, _color: u32, _background: u32) {}
    fn control_down(&mut self) -> bool {
        false
    }
    fn drag_request(&mut self, _window: Handle, _accept: &mut u8) -> bool {
        false
    }
    fn run_drag(&mut self, _tree: &mut TreeView, _session: &DragSession) {}
    fn dispose_surface(&mut self, _buffer: Handle) {}
    fn draw_subimage(&mut self, image: Handle, _source: Rect, destination: Rect) {
        self.draw_image(image, destination);
    }
    fn blit_viewport(&mut self, buffer: Handle, source: Rect, _destination: Rect) {
        self.draw(Draw::Blit {
            source: buffer,
            rect: source,
        });
    }
}

#[derive(Debug)]
pub struct TreeView {
    pub primary: Handle,
    pub nodes: BTreeMap<NodeHandle, Node>,
    pub visible: Vec<NodeHandle>,
    pub selection: Vec<NodeHandle>,
    pub next_handle: NodeHandle,
    pub root: Option<NodeHandle>,
    pub default_font: Option<Handle>,
    pub flags: u32,
    pub internal_initialized: bool,
    pub layout_dirty: bool,
    pub selection_dirty: bool,
    pub row_height: i32,
    pub base_color: u32,
    pub colors: [u32; 4],
    pub raw_style: [i32; 6],
    pub working_buffer: Option<Handle>,
    pub horizontal_bar: Option<Handle>,
    pub vertical_bar: Option<Handle>,
    pub first: i32,
    pub horizontal_position: i32,
    pub viewport_rows: u32,
    pub images: Vec<Option<Handle>>,
    pub image_rectangles: Vec<Rect>,
    pub image_width: u32,
    pub page_count: u32,
    pub viewport: Rect,
    pub insertion_side: bool,
    pub line_style: [u32; 2],
    /// Native leaked nodes remain outside map/topology after failed insertion.
    pub abandoned: Vec<Node>,
    pub fallback_text: Vec<u8>,
    /// Host-provided readable/writable words outside the modeled properties.
    /// Keys are the signed O40/O41 indices, not native addresses.
    pub raw_property_words: BTreeMap<i32, u32>,
    pub layout_area: Option<Rect>,
}

impl TreeView {
    pub fn new(primary: Handle) -> Self {
        Self {
            primary,
            nodes: BTreeMap::new(),
            visible: Vec::new(),
            selection: Vec::new(),
            next_handle: 0,
            root: None,
            default_font: None,
            flags: 0xb,
            internal_initialized: false,
            layout_dirty: false,
            selection_dirty: false,
            row_height: 16,
            base_color: SENTINEL,
            colors: [SENTINEL; 4],
            raw_style: [9, 3, 3, 3, 44, 1],
            working_buffer: None,
            horizontal_bar: None,
            vertical_bar: None,
            first: 0,
            horizontal_position: 0,
            viewport_rows: 0,
            images: Vec::new(),
            image_rectangles: Vec::new(),
            image_width: 0,
            page_count: 0,
            viewport: [0; 4],
            insertion_side: false,
            line_style: [0, 1],
            abandoned: Vec::new(),
            fallback_text: Vec::new(),
            raw_property_words: BTreeMap::new(),
            layout_area: None,
        }
    }

    pub fn window(&self) -> Handle {
        self.primary + WINDOW_OFFSET
    }

    /// Own-IID null output is a native caller precondition, unlike ListBox.
    pub fn query_interface(&self, iid: u32, output: Option<&mut Option<Handle>>, f: &mut impl Foreign) -> Result<bool> {
        if iid == IID {
            let output = output.ok_or(Unsupported::Precondition("TreeView own-IID output"))?;
            *output = Some(self.primary);
            f.add_ref(self.window());
            Ok(true)
        } else if let Some(output) = output {
            Ok(f.base_query(self.window(), iid, output))
        } else {
            Ok(false)
        }
    }

    pub fn internal_init(&mut self, f: &mut impl TreeForeign) {
        if self.internal_initialized {
            return;
        }
        self.internal_initialized = true;
        if self.base_color == SENTINEL {
            self.base_color = f.convert_color(self.window(), u32::MAX);
        }
        let window = self.window();
        for (slot, color) in self.colors.iter_mut().zip([0, 0xff_ffff, 0xff_ffff, 0]) {
            if *slot == SENTINEL {
                *slot = f.convert_color(window, color);
            }
        }
        if self.default_font.is_none() {
            let font = f.default_interface(0x6896_3c4c);
            replace_ref(&mut self.default_font, font, f);
        }
        self.initialize_images(f);
        self.create_root();
        if self.working_buffer.is_none() {
            self.working_buffer = if f.graphics_available() {
                f.create(Resource::Buffer)
            } else {
                None
            };
            if let Some(buffer) = self.working_buffer {
                if !f.initialize_buffer(buffer) {
                    f.release(buffer);
                    self.working_buffer = None;
                }
            }
        }
    }

    pub fn init(&mut self, f: &mut impl TreeForeign) -> bool {
        // Safe convenience wrapper; false is not a claimed native restart result.
        self.checked_init(f).unwrap_or(false)
    }

    pub fn checked_init(&mut self, f: &mut impl TreeForeign) -> Result<bool> {
        f.base_init(self.window());
        self.internal_init(f);
        self.require_live_root()?;
        if let Some(area) = f.tree_area(self.window()) {
            self.recalculate_all(area, f)?;
        } else {
            self.recalculate();
        }
        Ok(true)
    }

    fn require_live_root(&self) -> Result<()> {
        if self.root.is_some_and(|root| !self.nodes.contains_key(&root)) {
            return Err(Unsupported::Precondition(
                "live TreeView root (Shutdown is not restart)",
            ));
        }
        Ok(())
    }

    pub fn create_root(&mut self) {
        if self.root.is_some() {
            return;
        }
        let root = Node {
            handle: 0,
            reference: -1,
            text: Vec::new(),
            data: 0,
            images: [0, 1, 2, 3],
            font: self.default_font,
            colors: self.colors,
            expanded: true,
            force_branch: true,
            selected: false,
            flattened: false,
            visible_index: None,
            parent: None,
            children: Vec::new(),
            text_rect: [0; 4],
            image_rect: [0; 4],
            expander_rect: [0; 4],
        };
        self.nodes.insert(0, root);
        self.root = Some(0);
        self.next_handle += 1;
        self.layout_dirty = true;
    }

    pub fn shutdown(&mut self, f: &mut impl Foreign) {
        self.visible.clear();
        self.selection.clear();
        self.nodes.clear();
        self.image_rectangles.clear();
        for image in self.images.drain(..).flatten() {
            f.release(image);
        }
        replace_ref(&mut self.default_font, None, f);
        replace_ref(&mut self.working_buffer, None, f);
        replace_ref(&mut self.horizontal_bar, None, f);
        replace_ref(&mut self.vertical_bar, None, f);
        f.base_shutdown(self.window());
        // WC2: no invented restart protocol; internal_initialized is preserved.
    }

    pub fn node(&self, handle: NodeHandle) -> Option<&Node> {
        self.nodes.get(&handle)
    }
    pub fn parent(&self, handle: NodeHandle) -> NodeHandle {
        self.nodes.get(&handle).and_then(|node| node.parent).unwrap_or(-1)
    }
    pub fn insertion_reference(&self, handle: NodeHandle) -> i32 {
        self.nodes.get(&handle).map_or(-1, |node| node.reference)
    }

    /// WC2 ordering includes the consumed/leaked node on a missing reference.
    pub fn insert(&mut self, descriptor: Descriptor, f: &mut impl TreeForeign) -> Result<NodeHandle> {
        self.internal_init(f);
        if !self.nodes.contains_key(&descriptor.parent) {
            return Ok(-1);
        }
        let children = &self.nodes[&descriptor.parent].children;
        let position = match descriptor.reference {
            -2 => 0,
            -3 => children.len(),
            -4 => children
                .iter()
                .position(|h| self.nodes[h].text >= descriptor.text)
                .unwrap_or(children.len()),
            r => {
                if !self.nodes.contains_key(&r) {
                    let handle = self.insert_at_inner(descriptor, children.len(), false, f);
                    let node = self.nodes.remove(&handle).unwrap();
                    self.nodes
                        .get_mut(&node.parent.unwrap())
                        .unwrap()
                        .children
                        .retain(|h| *h != handle);
                    self.abandoned.push(node);
                    return Ok(-1);
                }
                children.iter().position(|h| *h == r).unwrap_or(children.len())
            }
        };
        Ok(self.insert_at(descriptor, position, f))
    }

    /// Ordered-list primitive; `insert` implements the native sentinel mapping.
    pub fn insert_at(&mut self, descriptor: Descriptor, position: usize, f: &mut impl TreeForeign) -> NodeHandle {
        self.insert_at_inner(descriptor, position, true, f)
    }
    fn insert_at_inner(
        &mut self,
        descriptor: Descriptor,
        position: usize,
        commit: bool,
        f: &mut impl TreeForeign,
    ) -> NodeHandle {
        self.internal_init(f);
        let Some(parent) = self.nodes.get(&descriptor.parent) else {
            return -1;
        };
        if position > parent.children.len() {
            return -1;
        }
        let handle = self.next_handle;
        self.next_handle += 1;
        let font = descriptor.font.or(self.default_font);
        if let Some(font) = font {
            self.row_height = self.row_height.max(f.font_height(font));
        }
        self.nodes.insert(
            handle,
            Node {
                handle,
                reference: descriptor.reference,
                text: descriptor.text,
                data: descriptor.data,
                images: descriptor.images,
                font,
                colors: descriptor.colors,
                expanded: descriptor.expanded,
                force_branch: false,
                selected: false,
                flattened: false,
                visible_index: None,
                parent: Some(descriptor.parent),
                children: Vec::new(),
                text_rect: [0; 4],
                image_rect: [0; 4],
                expander_rect: [0; 4],
            },
        );
        self.nodes
            .get_mut(&descriptor.parent)
            .unwrap()
            .children
            .insert(position, handle);
        if commit {
            self.dirty(f);
        }
        handle
    }

    fn dirty(&mut self, f: &mut impl Foreign) {
        self.layout_dirty = true;
        f.invalidate(self.window());
    }
    pub fn emit(&self, id: u32, handle: NodeHandle, f: &mut impl Foreign) {
        let payload = f.convert_payload(self.window(), handle);
        f.deliver(
            self.window(),
            Delivery::Send,
            Message {
                kind: 3,
                words: [id as i32, payload, 0, 0],
            },
        );
    }
    pub fn set_expanded(&mut self, handle: NodeHandle, expanded: bool, f: &mut impl Foreign) -> bool {
        let Some(node) = self.nodes.get_mut(&handle) else {
            return false;
        };
        if node.expanded != expanded {
            node.expanded = expanded;
            self.dirty(f);
            self.emit(if expanded { EXPAND } else { COLLAPSE }, handle, f);
        }
        true
    }
    pub fn set_font(&mut self, handle: NodeHandle, font: Option<Handle>, f: &mut impl TreeForeign) -> bool {
        let font = font.or(self.default_font);
        let Some(node) = self.nodes.get_mut(&handle) else {
            return false;
        };
        if node.font != font {
            node.font = font;
            if let Some(font) = font {
                self.row_height = self.row_height.max(f.font_height(font));
            }
            self.dirty(f);
        }
        // No per-node AddRef is observed (§6.3).
        true
    }

    pub fn recalculate(&mut self) {
        for node in self.nodes.values_mut() {
            node.flattened = false;
            node.visible_index = None;
        }
        self.visible.clear();
        if let Some(root) = self.root {
            let mut pending = vec![root];
            while let Some(handle) = pending.pop() {
                let Some(node) = self.nodes.get_mut(&handle) else {
                    continue;
                };
                let hidden_root = handle == root && self.flags & 2 == 0;
                if !hidden_root {
                    node.flattened = true;
                    node.visible_index = Some(self.visible.len());
                    self.visible.push(handle);
                }
                if node.expanded || hidden_root {
                    pending.extend(node.children.iter().rev().copied());
                }
            }
        }
        self.layout_dirty = false;
    }
    pub fn get_visible(&mut self, handle: NodeHandle) -> bool {
        if self.layout_dirty {
            self.recalculate();
        }
        self.nodes
            .get(&handle)
            .and_then(|node| node.visible_index)
            .is_some_and(|row| {
                row as i64 >= i64::from(self.first)
                    && (row as i64) < i64::from(self.first) + i64::from(self.viewport_rows)
            })
    }
    pub fn make_visible(&mut self, handle: NodeHandle, f: &mut impl TreeForeign) -> bool {
        if !self.nodes.contains_key(&handle) {
            return false;
        }
        if self.layout_dirty {
            self.recalculate();
        }
        let mut parent = self.nodes[&handle].parent;
        while let Some(ancestor) = parent {
            parent = self.nodes[&ancestor].parent;
            if !self.nodes[&ancestor].expanded {
                self.set_expanded(ancestor, true, f);
            }
        }
        self.recalculate();
        let Some(row) = self.nodes[&handle].visible_index else {
            return false;
        };
        if let Some(bar) = self.vertical_bar {
            if (row as i64) < i64::from(self.first) {
                self.first = row as i32;
            } else if row as i64 >= i64::from(self.first) + i64::from(self.viewport_rows) {
                self.first = row as i32 - self.viewport_rows as i32 + 1;
            }
            f.set_bar_value(bar, self.first);
        }
        true
    }

    pub fn select(&mut self, handle: NodeHandle, replacement: bool, f: &mut impl TreeForeign) -> bool {
        if !self.nodes.contains_key(&handle) {
            return false;
        }
        let mut changed = false;
        if replacement || self.flags & 0x10 == 0 {
            for other in self.selection.clone() {
                if other != handle {
                    if let Some(node) = self.nodes.get_mut(&other) {
                        node.selected = false;
                    }
                    self.selection.retain(|&item| item != other);
                    changed = true;
                }
            }
        }
        if !self.nodes[&handle].selected {
            self.make_visible(handle, f);
            self.nodes.get_mut(&handle).unwrap().selected = true;
            self.selection.push(handle);
            changed = true;
        }
        if changed {
            f.invalidate(self.window());
            self.emit(SELECTION, 0, f);
        }
        true
    }
    pub fn deselect(&mut self, handle: NodeHandle, f: &mut impl Foreign) -> bool {
        let Some(index) = self.selection.iter().position(|&item| item == handle) else {
            return false;
        };
        self.selection.remove(index);
        if let Some(node) = self.nodes.get_mut(&handle) {
            node.selected = false;
        }
        f.invalidate(self.window());
        self.emit(SELECTION, 0, f);
        true
    }
    pub fn clear_selection(&mut self, f: &mut impl Foreign) -> bool {
        if !self.selection.is_empty() {
            for handle in self.selection.drain(..) {
                if let Some(node) = self.nodes.get_mut(&handle) {
                    node.selected = false;
                }
            }
            f.invalidate(self.window());
            self.emit(SELECTION, 0, f);
        }
        true
    }
    /// Semantic descriptor transfer. Native descriptor output/child-copy ABI
    /// remains WC2; this value has no claimed native memory layout.
    pub fn set_descriptor(
        &mut self,
        handle: NodeHandle,
        value: Descriptor,
        visible: bool,
        selected: bool,
        f: &mut impl TreeForeign,
    ) -> bool {
        self.apply_descriptor(
            handle,
            NodeDescriptor {
                text: value.text,
                data: value.data,
                images: value.images,
                font: value.font,
                colors: value.colors,
                expanded: value.expanded,
                force_branch: false,
                visible,
                selected,
                children: Vec::new(),
            },
            f,
        )
    }

    pub fn do_message(&mut self, message: Message, f: &mut impl Foreign) -> Result<bool> {
        if message.kind == 3 && message.words[0] as u32 == 0x8871_13a3 {
            match message.words[1] as u32 {
                0x1756_bac7 => {
                    let bar = self.horizontal_bar.ok_or(Unsupported::Precondition("horizontal bar"))?;
                    self.horizontal_position = f.bar_value(bar);
                }
                0x1756_bac6 => {
                    let bar = self.vertical_bar.ok_or(Unsupported::Precondition("vertical bar"))?;
                    self.first = f.bar_value(bar);
                }
                _ => {}
            }
        }
        Ok(f.base_message(self.window(), message))
    }
    pub fn wheel(&mut self, delta: i32, f: &mut impl TreeForeign) -> bool {
        if let Some(bar) = self.vertical_bar {
            // Native comparisons are unsigned. Zero takes the upward branch.
            let first = self.first as u32;
            if delta < 0 {
                if first.wrapping_add(self.viewport_rows) < self.visible.len() as u32 {
                    self.first = first.wrapping_add(1) as i32;
                    f.set_bar_value(bar, self.first);
                }
            } else if first > 0 {
                self.first = first.wrapping_sub(1) as i32;
                f.set_bar_value(bar, self.first);
            }
        }
        true
    }
    fn activate(&mut self, handle: NodeHandle, f: &mut impl Foreign) {
        self.emit(ACTIVATION, handle, f);
        if let Some(node) = self.nodes.get(&handle) {
            if node.force_branch || !node.children.is_empty() {
                self.toggle_from_input(handle, f);
            }
        }
    }
    pub fn key_up(&self, _key: u32, _modifiers: u32) -> bool {
        false
    }
    pub fn key_down(&mut self, key: u32, f: &mut impl TreeForeign) -> Result<bool> {
        match key {
            0x1b => {
                self.clear_selection(f);
                return Ok(true);
            }
            0x0d => {
                for handle in self.selection.clone() {
                    self.activate(handle, f);
                }
                return Ok(true);
            }
            0x21 | 0x22 => {}
            0x23..=0x28 => {}
            _ => return Ok(true),
        }
        if self.visible.is_empty() {
            return Err(Unsupported::Precondition("nonempty cached navigation sequence"));
        }
        let current = self.selection.last().copied();
        let index = current
            .and_then(|handle| self.nodes.get(&handle)?.visible_index)
            .unwrap_or(0);
        let target = match key {
            0x21 | 0x22 => {
                let delta = self.page_count as i32;
                let candidate = if key == 0x21 {
                    (index as i32).wrapping_sub(delta)
                } else {
                    (index as i32).wrapping_add(delta)
                };
                self.visible[(candidate.max(0) as usize).min(self.visible.len() - 1)]
            }
            0x24 => self.visible[0],
            0x23 => *self.visible.last().unwrap(),
            0x26 => self.visible[index.saturating_sub(1)],
            0x28 => self.visible[(index + 1).min(self.visible.len() - 1)],
            0x25 => {
                let handle = self.visible[index];
                if current.is_none() {
                    self.select(handle, true, f);
                    return Ok(true);
                }
                if self.nodes[&handle].expanded {
                    self.set_expanded(handle, false, f);
                    return Ok(true);
                }
                self.nodes[&handle].parent.unwrap_or(handle)
            }
            0x27 => {
                let handle = self.visible[index];
                if current.is_none() {
                    self.select(handle, true, f);
                    return Ok(true);
                }
                if !self.nodes[&handle].expanded {
                    self.set_expanded(handle, true, f);
                    return Ok(true);
                }
                self.visible[(index + 1).min(self.visible.len() - 1)]
            }
            _ => unreachable!(),
        };
        self.select(target, true, f);
        Ok(true)
    }
    pub fn mouse_down(&mut self, x: i32, y: i32, f: &mut impl TreeForeign) -> Result<bool> {
        // Host adapters use mouse_button with the application-wide ClickState.
        // This convenience entry is a single-click operation.
        self.mouse_button(x, y, false, &mut ClickState::default(), f)
    }

    /// Established DrawItem interface contract for already resolved hit areas.
    /// Expander/connector/gradient/layout pixels stay WC3, rather than guessed.
    pub fn draw_item(
        &mut self,
        handle: NodeHandle,
        text_rect: Rect,
        image_rect: Rect,
        expander_rect: Rect,
        f: &mut impl TreeForeign,
    ) -> Result<bool> {
        self.render_item(handle, text_rect, image_rect, expander_rect, f)
    }
    pub fn paint(&mut self, viewport: Rect, f: &mut impl TreeForeign) -> Result<()> {
        self.paint_tree(viewport, f)
    }
}

fn contains(rect: Rect, x: i32, y: i32) -> bool {
    rect[2] > rect[0] && rect[3] > rect[1] && x >= rect[0] && x <= rect[2] && y >= rect[1] && y <= rect[3]
}
