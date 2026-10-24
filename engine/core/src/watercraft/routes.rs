// SPDX-License-Identifier: GPL-3.0-or-later
//! Compressed route reader consumed by watercraft §5.4.1.
use super::{math, Cell};
use crate::automata::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteError {
    Truncated,
}

/// Safe storage boundary: native truncated buffers are invalid input, not exhaustion.
#[derive(Clone, Debug, Default)]
pub struct RouteReader {
    cells: Vec<Cell>,
    next: usize,
}

impl RouteReader {
    pub fn from_bytes(bytes: &[u8]) -> std::result::Result<Self, RouteError> {
        let mut result = Self::default();
        if bytes.is_empty() {
            return Ok(result);
        }
        if bytes.len() < 3 {
            return Err(RouteError::Truncated);
        }
        let mut x = i16::from(bytes[1]);
        let mut z = i16::from(bytes[2]);
        result.cells.push(Cell {
            x: i32::from(x),
            z: i32::from(z),
        });
        let mut cursor = 3;
        while cursor < bytes.len() {
            let descriptor = bytes[cursor];
            cursor += 1;
            if descriptor & 0x40 != 0 {
                let count = usize::from(*bytes.get(cursor).ok_or(RouteError::Truncated)?);
                cursor += 1;
                if count == 0 {
                    break;
                }
                let packed = bytes
                    .get(cursor..cursor + count.div_ceil(4))
                    .ok_or(RouteError::Truncated)?;
                for i in 0..count {
                    match (packed[i / 4] >> (2 * (i % 4))) & 3 {
                        0 => x = x.wrapping_sub(1),
                        1 => z = z.wrapping_sub(1),
                        2 => x = x.wrapping_add(1),
                        _ => z = z.wrapping_add(1),
                    }
                    result.cells.push(Cell {
                        x: i32::from(x),
                        z: i32::from(z),
                    });
                }
                cursor += count.div_ceil(4);
            } else {
                x = i16::from(*bytes.get(cursor).ok_or(RouteError::Truncated)?);
                z = i16::from(*bytes.get(cursor + 1).ok_or(RouteError::Truncated)?);
                cursor += 2;
                result.cells.push(Cell {
                    x: i32::from(x),
                    z: i32::from(z),
                });
            }
        }
        Ok(result)
    }
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }
    pub fn reset(&mut self) {
        self.next = 0;
    }
    pub fn get_next(&mut self) -> Option<Cell> {
        let cell = self.cells.get(self.next).copied()?;
        self.next += 1;
        Some(cell)
    }
}

pub fn sign(value: f32) -> f32 {
    if value < 0.0 {
        -1.0
    } else {
        1.0
    }
}

impl super::Watercraft {
    /// First failure in a search round removes points and marks the survivor's
    /// own direction, not the rejected direction (WC7-R02).
    pub fn backtrack_travel(&mut self) {
        let count = self.travel.len();
        let keep = if count >= 6 {
            count - 5
        } else if count >= 4 {
            count - 3
        } else {
            0
        };
        self.travel.truncate(keep);
        if let Some(last) = self.travel.last_mut() {
            last.tried |= 1_u32.wrapping_shl(last.direction & 31) as u8;
        }
    }
}

impl super::Fleet {
    /// Owner search (§5.2). Only cell conversion/navigation are foreign inputs.
    pub fn pick_location(
        &mut self,
        id: super::BoatId,
        terrain: &dyn super::Terrain,
        mut centre: impl FnMut(Cell) -> Vec3,
        manager: &mut Option<&mut dyn super::DockManager>,
    ) -> super::Result<bool> {
        if self.boat(id)?.neighbor.is_some() || self.boat(id)?.ferry {
            return self.fill_route_points(id, terrain, manager);
        }
        let initial = terrain.cell_at(self.boat(id)?.position);
        let start = self.boat(id)?.travel.last().map_or(initial, |p| p.cell);
        if !terrain.contains(start) {
            return Ok(false);
        }
        let radius = match self.boat(id)?.group.id {
            0x4414 => 3,
            0x4413 => 2,
            _ => 1,
        };
        for _ in 0..15 {
            if self.boat(id)?.travel.len() >= 10 {
                break;
            }
            let boat = self.boat(id)?;
            let (cell, direction, tried) = boat.travel.last().map_or_else(
                || (initial, quantize_heading(boat.heading), 0),
                |p| (p.cell, p.direction, p.tried),
            );
            let mut backed_up = false;
            // Literal §5.2: a round scans this point's ordered candidates;
            // only the first eligible failure performs the backtrack.
            for direction in super::candidate_directions(direction, tried) {
                let candidate = direction_step(cell, direction);
                let offset = math::sub(centre(candidate), self.boat(id)?.position);
                if terrain.navigable(candidate, radius, None) && self.find_overlapping_boat(id, offset, 1.0)?.is_none()
                {
                    self.boat_mut(id)?.travel.push(super::TravelPoint {
                        cell: candidate,
                        tried: 0,
                        direction: direction as u32,
                    });
                    break;
                }
                if !backed_up {
                    self.boat_mut(id)?.backtrack_travel();
                    backed_up = true;
                }
            }
        }
        Ok(!self.boat(id)?.travel.is_empty())
    }

    /// Fill the settled neighbor/ferry branches. Wandering has no reader and
    /// is deliberately rejected here rather than mistaken for exhaustion.
    pub fn fill_route_points(
        &mut self,
        id: super::BoatId,
        terrain: &dyn super::Terrain,
        manager: &mut Option<&mut dyn super::DockManager>,
    ) -> super::Result<bool> {
        let boat = self.boat(id)?;
        let start = boat
            .travel
            .last()
            .map_or_else(|| terrain.cell_at(boat.position), |p| p.cell);
        if !terrain.contains(start) {
            return Ok(false);
        }
        if boat.neighbor.is_none() && !boat.ferry {
            return Err(super::ContractGap::MissingRouteWc7);
        }
        while self.boat(id)?.travel.len() < 10 {
            let Some(cell) = self.next_route_cell(id, manager)? else {
                break;
            };
            self.boat_mut(id)?.travel.push(super::TravelPoint {
                cell,
                tried: 0,
                direction: u32::MAX,
            });
        }
        Ok(!self.boat(id)?.travel.is_empty())
    }
}

pub fn direction_step(cell: Cell, direction: i32) -> Cell {
    let delta = match direction {
        0 => (-1, -1),
        1 => (0, -1),
        2 => (1, -1),
        3 => (1, 0),
        4 => (1, 1),
        5 => (0, 1),
        6 => (-1, 1),
        7 => (-1, 0),
        _ => (0, 0),
    };
    Cell {
        x: cell.x.wrapping_add(delta.0),
        z: cell.z.wrapping_add(delta.1),
    }
}

/// Normalize only a local copy, including its y component (§§3.4,5.4.1).
pub fn quantize_heading(heading: Vec3) -> u32 {
    let [x, _, z] = math::normalize(heading);
    if x == 0.0 {
        return if z > 0.0 { 5 } else { 1 };
    }
    if z == 0.0 {
        return if x > 0.0 { 3 } else { 7 };
    }
    let angle = x.abs().acos().to_degrees();
    if angle < 22.5 {
        if x > 0.0 {
            3
        } else {
            7
        }
    } else if angle < 67.5 {
        match (x > 0.0, z > 0.0) {
            (true, true) => 4,
            (true, false) => 2,
            (false, true) => 6,
            (false, false) => 0,
        }
    } else if z > 0.0 {
        5
    } else {
        1
    }
}
