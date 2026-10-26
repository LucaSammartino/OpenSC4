// SPDX-License-Identifier: GPL-3.0-or-later
//! SC4QuadGrid and SC43DGridElement (render3d §5.1; small-owners-b9 §2).
use super::{Handle, Rect};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Default for WorldBox {
    fn default() -> Self {
        Self {
            min: [1.0; 3],
            max: [0.0; 3],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ContainerRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl ContainerRect {
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridElement {
    pub model: Handle,
    pub flags: u16,
    pub bounds: WorldBox,
    pub cells: Rect,
    references: u16,
}

impl GridElement {
    pub fn new(model: Handle) -> Self {
        Self {
            model,
            flags: 0,
            bounds: WorldBox::default(),
            cells: Rect::default(),
            references: 0,
        }
    }

    pub fn reference_count(&self) -> u16 {
        self.references
    }

    /// Keep class-independent bits 0–4 and 14–15, derive the rest (§5).
    pub fn update_object_flags(&mut self, properties: u32, attributes: u32) {
        let p = properties;
        let a = attributes;
        self.flags &= 0xC01F;
        if p & 1 != 0 {
            self.flags |= 0x20;
        }
        if a & 4 != 0 {
            self.flags |= 0x101;
        }
        if a & 2 != 0 {
            self.flags |= 0x100;
        }
        if a & 6 == 0 {
            if p & 4 != 0 {
                self.flags |= 0x100;
            }
            if p & 2 != 0 {
                self.flags |= 0x40;
            }
        }
        self.flags |= ((p & 0xF8) << 6) as u16;
        if p & 0xF0 == 0 {
            self.flags |= 0x800;
        }
    }

    pub fn should_go_into_backing_store(&self, zoom: i32, limits: BackingLimits) -> bool {
        let flags = self.flags;
        flags & 0x100 == 0
            && flags & 0x20 == 0
            && flags & 0x40 != 0
            && ((flags & 0x400 != 0 && zoom < limits.terrain)
                || (flags & 0x1800 != 0 && zoom < limits.buildings_props)
                || (flags & 0x2000 != 0 && zoom < limits.flora))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackingLimits {
    pub terrain: i32,
    pub buildings_props: i32,
    pub flora: i32,
}

#[derive(Debug, Default)]
struct Quadrant {
    flags: u16,
    elements: Vec<Handle>,
}

/// Sparse storage preserves the specified per-cell list and swap removal.
/// Model references belong to the renderer, not to these element references.
#[derive(Debug)]
pub struct QuadGrid {
    size: f32,
    cell_size: f32,
    side: i32,
    scale: f32,
    quadrants: BTreeMap<(i32, i32), Quadrant>,
    elements: BTreeMap<Handle, GridElement>,
}

impl QuadGrid {
    pub fn for_map_scale(factor: i32) -> Self {
        Self::new(factor as f32 * 8192.0, factor as f32 * 32.0)
    }

    pub fn new(size: f32, cell_size: f32) -> Self {
        // SPEC-GAP: nonpositive/nonfinite grid dimensions make an empty grid.
        let valid = size.is_finite() && cell_size.is_finite() && size > 0.0 && cell_size > 0.0;
        Self {
            size,
            cell_size,
            side: if valid {
                (size / cell_size + 0.5).trunc() as i32
            } else {
                0
            },
            scale: if valid { 1.0 / cell_size } else { 0.0 },
            quadrants: BTreeMap::new(),
            elements: BTreeMap::new(),
        }
    }

    pub fn size(&self) -> f32 {
        self.size
    }
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }
    pub fn side(&self) -> i32 {
        self.side
    }
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }
    pub fn element(&self, model: Handle) -> Option<&GridElement> {
        self.elements.get(&model)
    }
    pub fn element_mut(&mut self, model: Handle) -> Option<&mut GridElement> {
        self.elements.get_mut(&model)
    }

    pub fn cells_for_rect(&self, rect: ContainerRect) -> Rect {
        let cell = |value: f32| (value * self.scale - 0.5).round_ties_even() as i32;
        Rect::new(cell(rect.left), cell(rect.top), cell(rect.right), cell(rect.bottom))
    }

    fn clamped(&self, rect: Rect) -> Option<Rect> {
        let out = Rect::new(
            rect.left.max(0),
            rect.top.max(0),
            rect.right.min(self.side - 1),
            rect.bottom.min(self.side - 1),
        );
        (self.side > 0 && out.left <= out.right && out.top <= out.bottom).then_some(out)
    }

    fn attach(&mut self, model: Handle, cells: Rect, flags: u16) -> u16 {
        let mut count = 0u16;
        for y in cells.top..=cells.bottom {
            for x in cells.left..=cells.right {
                let quadrant = self.quadrants.entry((y, x)).or_default();
                quadrant.elements.push(model);
                quadrant.flags |= flags;
                count = count.wrapping_add(1);
            }
        }
        count
    }

    pub fn add_element(&mut self, mut element: GridElement, rect: ContainerRect, flags: u16) -> bool {
        // SPEC-GAP: duplicate handles are rejected without changing the grid.
        if self.elements.contains_key(&element.model) {
            return false;
        }
        element.cells = self.cells_for_rect(rect);
        let Some(cells) = self.clamped(element.cells) else {
            return false;
        };
        element.references = element
            .references
            .wrapping_add(self.attach(element.model, cells, flags));
        self.elements.insert(element.model, element);
        true
    }

    pub fn remove_element(&mut self, model: Handle) -> Option<GridElement> {
        let mut element = self.elements.remove(&model)?;
        if let Some(cells) = self.clamped(element.cells) {
            for y in cells.top..=cells.bottom {
                for x in cells.left..=cells.right {
                    if let Some(quadrant) = self.quadrants.get_mut(&(y, x)) {
                        if let Some(index) = quadrant.elements.iter().position(|entry| *entry == model) {
                            quadrant.elements.swap_remove(index);
                            element.references = element.references.wrapping_sub(1);
                        }
                    }
                }
            }
        }
        Some(element)
    }

    pub fn move_element(&mut self, model: Handle, rect: ContainerRect, flags: u16) -> bool {
        let cells = self.cells_for_rect(rect);
        let Some(element) = self.elements.get(&model) else {
            return false;
        };
        if element.cells == cells {
            return true;
        }
        let Some(mut element) = self.remove_element(model) else {
            return false;
        };
        element.cells = cells;
        self.add_element(element, rect, flags)
    }

    pub fn set_quadrant_flags_for_element(&mut self, model: Handle, flags: u16) {
        let Some(element) = self.elements.get(&model) else {
            return;
        };
        let Some(cells) = self.clamped(element.cells) else {
            return;
        };
        for y in cells.top..=cells.bottom {
            for x in cells.left..=cells.right {
                if let Some(quadrant) = self.quadrants.get_mut(&(y, x)) {
                    quadrant.flags |= flags;
                }
            }
        }
    }

    pub fn quadrant(&self, x: i32, y: i32) -> (u16, &[Handle]) {
        self.quadrants
            .get(&(y, x))
            .map_or((0, &[]), |q| (q.flags, q.elements.as_slice()))
    }

    pub fn all_elements(&mut self, limit: usize) -> Vec<Handle> {
        let mut out = Vec::new();
        // SPEC-GAP: cell enumeration order is unspecified; visit rows then columns.
        for quadrant in self.quadrants.values() {
            for model in &quadrant.elements {
                if out.len() == limit {
                    break;
                }
                let element = self.elements.get_mut(model).expect("grid membership");
                if element.flags & 0x4000 == 0 {
                    element.flags |= 0x4000;
                    out.push(*model);
                }
            }
        }
        for model in &out {
            self.elements.get_mut(model).expect("collected element").flags &= !0x4000;
        }
        out
    }

    pub fn remove_all_elements(&mut self) {
        self.quadrants.clear();
        self.elements.clear();
    }
}
