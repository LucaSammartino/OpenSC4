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
