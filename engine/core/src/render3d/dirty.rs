// SPDX-License-Identifier: GPL-3.0-or-later
//! Dirty-rectangle storage, filtering and merging (§6.1).
use super::{Handle, Rect, References, Render3d, ViewServices};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirtyRequest {
    pub mode: i32,
    pub rect: Rect,
    pub model: Option<Handle>,
    pub delay: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirtyNode {
    pub stamp: i32,
    pub mode: i32,
    pub rect: Rect,
    pub model: Option<Handle>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirtyPolicy {
    pub merge: bool,
    pub node_threshold: u32,
    pub merge_threshold: f32,
    pub area_limit: i32,
}

impl Default for DirtyPolicy {
    fn default() -> Self {
        Self {
            merge: true,
            node_threshold: 16,
            merge_threshold: -2500.0,
            area_limit: i32::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirtyAdd {
    Queued,
    FullUpdate,
}

#[derive(Debug, Default)]
pub struct DirtyQueue {
    nodes: Vec<DirtyNode>,
    area: i32,
    deadline_base: i32,
}

impl Rect {
    pub fn area(self) -> i32 {
        self.bottom
            .wrapping_sub(self.top)
            .wrapping_mul(self.right.wrapping_sub(self.left))
    }

    pub fn union(self, other: Self) -> Self {
        Self::new(
            self.left.min(other.left),
            self.top.min(other.top),
            self.right.max(other.right),
            self.bottom.max(other.bottom),
        )
    }

    /// Deliberately accepts an inverted rectangle when its edges pass (§6.1).
    pub fn filter_to_viewport(&mut self, width: i32, height: i32) -> bool {
        if self.right <= 0 || self.left >= width || self.bottom <= 0 || self.top >= height {
            return false;
        }
        self.left = self.left.max(0);
        self.top = self.top.max(0);
        self.right = self.right.min(width);
        self.bottom = self.bottom.min(height);
        true
    }
}

impl DirtyQueue {
    pub fn nodes(&self) -> &[DirtyNode] {
        &self.nodes
    }
    pub fn accumulated_area(&self) -> i32 {
        self.area
    }
    pub fn deadline_base(&self) -> i32 {
        self.deadline_base
    }

    /// Returns a request for the owner's full-update transition, not a callback
    /// into the original renderer. All retained models remain owned until clear.
    pub fn add(
        &mut self,
        host: &mut impl References,
        mut request: DirtyRequest,
        time: [i32; 2],
        policy: DirtyPolicy,
    ) -> DirtyAdd {
        if request.mode != 0 {
            request.model = None;
        }
        let area = request.rect.area();
        if self.nodes.is_empty() {
            self.deadline_base = time[0].wrapping_sub(request.delay);
        } else if policy.merge {
            self.deadline_base = self.deadline_base.wrapping_sub(request.delay);
            let mut best = policy.area_limit.wrapping_neg();
            let mut best_index = 0;
            let mut growth = 0;
            for (index, node) in self.nodes.iter().enumerate() {
                let node_area = node.rect.area();
                let union_area = node.rect.union(request.rect).area();
                let mut score = node_area.wrapping_add(area).wrapping_sub(union_area);
                if request.mode == 0 || node.mode == 0 {
                    score = score.wrapping_sub(area.min(node_area));
                }
                if score > best {
                    best = score;
                    best_index = index;
                    growth = union_area.wrapping_sub(node_area);
                }
            }
            if self.nodes.len() as u32 >= policy.node_threshold || best as f32 >= policy.merge_threshold {
                self.area = self.area.wrapping_add(growth);
                if self.area >= policy.area_limit {
                    return DirtyAdd::FullUpdate;
                }
                let node = &mut self.nodes[best_index];
                let union = node.rect.union(request.rect);
                if let Some(model) = node.model.take() {
                    host.release(model);
                }
                if growth > 0 && self.nodes.len() > 1 {
                    self.nodes.remove(best_index);
                    return self.add(
                        host,
                        DirtyRequest {
                            mode: 6,
                            rect: union,
                            model: None,
                            delay: 0,
                        },
                        time,
                        policy,
                    );
                }
                let node = &mut self.nodes[best_index];
                node.mode = 6;
                node.rect = union;
                return DirtyAdd::Queued;
            }
        }
        self.area = self.area.wrapping_add(area);
        if self.area >= policy.area_limit {
            return DirtyAdd::FullUpdate;
        }
        if let Some(model) = request.model {
            host.add_ref(model);
        }
        self.nodes.push(DirtyNode {
            stamp: time[1],
            mode: request.mode,
            rect: request.rect,
            model: request.model,
        });
        DirtyAdd::Queued
    }

    pub fn clear(&mut self, host: &mut impl References) {
        if self.nodes.is_empty() {
            return;
        }
        for node in self.nodes.drain(..) {
            if let Some(model) = node.model {
                host.release(model);
            }
        }
        self.area = 0;
    }
}

impl Render3d {
    pub fn dirty_queue(&self) -> &DirtyQueue {
        &self.dirty
    }

    pub fn add_dirty_rectangle(&mut self, host: &mut impl ViewServices, mut request: DirtyRequest) {
        if self.state.full_redraw {
            return;
        }
        let [width, height] = self.state.viewport_size;
        if !request.rect.filter_to_viewport(width, height) {
            return;
        }
        let policy = DirtyPolicy {
            area_limit: self.state.dirty_area_limit,
            ..DirtyPolicy::default()
        };
        if self.dirty.add(
            host,
            request,
            [self.state.draw_counter, self.state.view_change_counter],
            policy,
        ) == DirtyAdd::FullUpdate
        {
            self.do_full_view_update(host, 4);
        }
    }

    pub fn clear_dirty_rectangle_queue(&mut self, host: &mut impl References) {
        self.dirty.clear(host);
    }
}
