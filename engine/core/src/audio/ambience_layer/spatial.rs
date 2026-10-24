// SPDX-License-Identifier: GPL-3.0-or-later
//! Group lifetime/intersection and 3x3 sample reduction, ambience §8.4–8.5.
use super::{Position, Rectangle};

#[derive(Clone, Debug)]
pub struct Group {
    pub center: Position,
    pub radius: f32,
    pub count: u32,
    pub peak: u32,
}

#[derive(Clone, Debug)]
pub struct GroupLayer {
    pub radius: f32,
    pub groups: Vec<Group>,
}

pub fn distance(a: Position, b: Position) -> f64 {
    let [dx, dy, dz] = std::array::from_fn(|i| f64::from(a[i]) - f64::from(b[i]));
    ((dz * dz + dy * dy) + dx * dx).sqrt()
}

impl GroupLayer {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            groups: Vec::new(),
        }
    }
    pub fn insert(&mut self, position: Position, extra_radius: f32) {
        for group in &mut self.groups {
            if distance(group.center, position) < f64::from(extra_radius) + f64::from(group.radius) {
                group.count = group.count.wrapping_add(1);
                group.peak = group.peak.max(group.count);
                return;
            }
        }
        self.groups.push(Group {
            center: position,
            radius: self.radius,
            count: 1,
            peak: 1,
        });
    }
    pub fn begin(&mut self) {
        for group in &mut self.groups {
            group.peak = group.count;
            group.count = 0;
        }
    }
    pub fn end(&mut self) {
        self.groups.retain(|group| group.count != 0);
    }
    pub fn intersect(&mut self, a: &Self, b: &Self, swap: bool) {
        let (inner, outer) = if swap { (b, a) } else { (a, b) };
        self.groups.clear();
        let mut used = vec![false; inner.groups.len()];
        for o in &outer.groups {
            for (index, i) in inner.groups.iter().enumerate() {
                if !used[index] && distance(o.center, i.center) < f64::from(o.radius) + f64::from(i.radius) {
                    self.groups.push(Group {
                        center: i.center,
                        radius: i.radius,
                        count: 1,
                        peak: 1,
                    });
                    used[index] = true;
                }
            }
        }
    }
}

/// _ftol2's masked invalid low-word result is specified here as INT_MIN
/// (§8.5 grid), rather than x87::ftol32's separate runtime convention.
fn grid_index(value: f64) -> usize {
    let integer = if !value.is_finite() || value < i32::MIN as f64 || value >= 2147483648.0 {
        i32::MIN
    } else {
        value as i32
    };
    integer.clamp(0, 2) as usize
}

pub fn distributed_positions(positions: &[Position], rectangle: Rectangle) -> Vec<Position> {
    if positions.len() <= 9 {
        return positions.to_vec();
    }
    let sx = crate::x87::ftol32((f64::from(rectangle[2]) - f64::from(rectangle[0])) * f64::from(0.33333334f32));
    let sz = crate::x87::ftol32((f64::from(rectangle[3]) - f64::from(rectangle[1])) * f64::from(0.33333334f32));
    let mut occupied = [false; 9];
    let mut output = Vec::new();
    for position in positions {
        let ix = grid_index((f64::from(position[0]) - f64::from(rectangle[0])) / f64::from(sx));
        let iz = grid_index((f64::from(position[2]) - f64::from(rectangle[1])) / f64::from(sz));
        if !occupied[ix + 3 * iz] {
            occupied[ix + 3 * iz] = true;
            output.push(*position);
        }
    }
    output
}
