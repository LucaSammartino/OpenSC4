// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::{flag, TextureBinding, TextureSource};
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle(pub i32);

impl Handle {
    pub const NONE: Self = Self(-1);

    pub(super) fn is_indirect(self) -> bool {
        self.0 < 0 && self != Self::NONE
    }

    pub(super) fn index(self) -> u32 {
        self.0 as u32 & 0x7FFF_FFFF
    }
}

/// cx, cz, full width, turns, stretch, repeatU, repeatV, offset, retained (§3.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement(pub [f32; 9]);

/// Raw signed half-open cell rectangle `(x0, z0, x1, z1)` (§2.2). Inverted
/// bounds are kept as supplied; they are simply empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRect {
    pub min: [i32; 2],
    pub max: [i32; 2],
}

impl CellRect {
    pub const ZERO: Self = Self {
        min: [0, 0],
        max: [0, 0],
    };

    /// Signed intersection with `(0, 0, nx, nz)` (`0x0048ED90`): a strictly
    /// separated rectangle becomes all zeros, a touching edge is retained.
    pub fn clipped(self, city_cells: [u32; 2]) -> Self {
        let city = city_cells.map(i64::from);
        let [x0, z0] = self.min.map(i64::from);
        let [x1, z1] = self.max.map(i64::from);
        if x1 < 0 || z1 < 0 || x0 > city[0] || z0 > city[1] {
            return Self::ZERO;
        }
        Self {
            min: [x0.max(0) as i32, z0.max(0) as i32],
            max: [x1.min(city[0]) as i32, z1.min(city[1]) as i32],
        }
    }

    pub fn is_empty(self) -> bool {
        self.min[0] >= self.max[0] || self.min[1] >= self.max[1]
    }

    pub fn contains(self, cell: [i32; 2]) -> bool {
        (0..2).all(|axis| self.min[axis] <= cell[axis] && cell[axis] < self.max[axis])
    }

    pub(super) fn cells(self) -> impl Iterator<Item = [i32; 2]> {
        (self.min[1]..self.max[1]).flat_map(move |z| (self.min[0]..self.max[0]).map(move |x| [x, z]))
    }
}

/// Slot 21's specified transform in semantic texture axes. It deliberately
/// avoids guessing which world-matrix elements encode the second axis (OV7).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextureScale {
    pub scale: [f32; 2],
    pub second_axis_offset: f32,
}

impl TextureScale {
    pub fn new(size: f32, offset: f32) -> Self {
        Self {
            scale: [1.0 / size, -1.0 / size],
            second_axis_offset: 1.0 + offset / size,
        }
    }
}

#[derive(Clone, Copy)]
// Only tests construct records until OV1/OV2 publish the add geometry.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) enum Target {
    Overlay(u32),
    Group(u32),
}

pub(super) struct HandleRecord {
    pub placement: Placement,
    pub aspect: f32,
    pub target: Target,
}

pub(super) struct Overlay {
    pub stamp: u16,
    pub secondary: Option<[f32; 16]>,
    pub primary: [f32; 16],
    pub flags: u32,
    pub cells: CellRect,
    pub alpha: f32,
    pub color: [f32; 3],
    pub draw_state: u8,
    pub texture_scale: Option<TextureScale>,
    // Stored ownership, even while drawing is incomplete. Releasing an overlay
    // drops only its own Arc; caller-owned references remain valid.
    pub binding: Option<Arc<dyn TextureBinding>>,
    pub source: Option<Arc<dyn TextureSource>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlaySnapshot {
    pub secondary: Option<[f32; 16]>,
    pub primary: [f32; 16],
    pub flags: u32,
    pub cells: CellRect,
    pub alpha: f32,
    pub color: [f32; 3],
    pub draw_state: u8,
    pub texture_scale: Option<TextureScale>,
    pub has_binding: bool,
    pub has_source: bool,
}

impl Overlay {
    pub fn snapshot(&self) -> OverlaySnapshot {
        OverlaySnapshot {
            secondary: self.secondary,
            primary: self.primary,
            flags: self.flags,
            cells: self.cells,
            alpha: self.alpha,
            color: self.color,
            draw_state: self.draw_state,
            texture_scale: self.texture_scale,
            has_binding: self.binding.is_some(),
            has_source: self.source.is_some(),
        }
    }

    pub fn decal_visible(&self, snapshot: bool) -> bool {
        self.flags & (flag::SHOWN | flag::PART) == flag::SHOWN | flag::PART
            && (!snapshot || self.flags & flag::SNAPSHOT != 0)
    }
}

/// FIFO free queue with retained payloads (§2.3). Portable bounds/stale checks
/// prevent unsafe Rust access; native validation is not claimed. No generations.
pub(super) struct Arena<T> {
    slots: Vec<Option<T>>,
    free: VecDeque<u32>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            free: VecDeque::new(),
        }
    }
}

impl<T> Arena<T> {
    pub fn insert(&mut self, value: T) -> u32 {
        if let Some(index) = self.free.pop_front() {
            self.slots[index as usize] = Some(value);
            index
        } else {
            let index = self.slots.len() as u32;
            assert!(index < 0x7FFF_FFFF);
            self.slots.push(Some(value));
            index
        }
    }

    pub fn get(&self, index: u32) -> Option<&T> {
        if self.free.contains(&index) {
            return None;
        }
        self.slots.get(index as usize)?.as_ref()
    }

    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        if self.free.contains(&index) {
            return None;
        }
        self.slots.get_mut(index as usize)?.as_mut()
    }

    pub fn payload_mut(&mut self, index: u32) -> Option<&mut T> {
        self.slots.get_mut(index as usize)?.as_mut()
    }

    pub fn remove(&mut self, index: u32) {
        if self.get(index).is_some() {
            // Native removal links the slot but retains payload until reuse/range destruction.
            self.free.push_back(index);
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.free.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, o)| {
            if self.free.contains(&(i as u32)) {
                None
            } else {
                o.as_ref().map(|o| (i as u32, o))
            }
        })
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        // Logical range/reuse reference protocol; allocation/reentrancy remains unqualified.
        drop(self.source.take());
        drop(self.binding.take());
    }
}
