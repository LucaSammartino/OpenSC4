// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::{flag, TextureBinding, TextureSource};
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

/// Nine floats are preserved verbatim. Their meaning/order is open under OV1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement(pub [f32; 9]);

/// Internal half-open cell interval, not a claim about original endpoint layout
/// (OV8). Clipping and intersection are owned by this module, not a terrain host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRect {
    pub min: [u32; 2],
    pub max: [u32; 2],
}

impl CellRect {
    pub fn clipped(self, city_cells: [u32; 2]) -> Self {
        let min = [self.min[0].min(city_cells[0]), self.min[1].min(city_cells[1])];
        Self {
            min,
            max: [
                self.max[0].min(city_cells[0]).max(min[0]),
                self.max[1].min(city_cells[1]).max(min[1]),
            ],
        }
    }

    pub fn is_empty(self) -> bool {
        self.min[0] >= self.max[0] || self.min[1] >= self.max[1]
    }

    pub fn contains(self, cell: [u32; 2]) -> bool {
        (0..2).all(|axis| self.min[axis] <= cell[axis] && cell[axis] < self.max[axis])
    }

    pub(super) fn cells(self) -> impl Iterator<Item = [u32; 2]> {
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
    pub target: Target,
}

pub(super) struct Overlay {
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

/// Free-list storage. [choice, OV5] Freed slots are reused LIFO; stale handles
/// are rejected until reuse. No generation bits are invented in the public ID.
pub(super) struct Arena<T> {
    slots: Vec<Option<T>>,
    free: Vec<u32>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }
}

impl<T> Arena<T> {
    // Construction stays private until OV1/OV2 supply placement contracts. Tests
    // seed records here to exercise setter logic without a fake placement host.
    #[cfg(test)]
    pub fn insert(&mut self, value: T) -> u32 {
        if let Some(index) = self.free.pop() {
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
        self.slots.get(index as usize)?.as_ref()
    }

    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        self.slots.get_mut(index as usize)?.as_mut()
    }

    pub fn remove(&mut self, index: u32) {
        if let Some(slot) = self.slots.get_mut(index as usize) {
            if slot.take().is_some() {
                self.free.push(index);
            }
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.free.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, o)| o.as_ref().map(|o| (i as u32, o)))
    }
}
