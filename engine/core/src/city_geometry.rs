// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded city geometry: terrain §2.1 and budget §17 item 6.
//! HG8: city persistence and the remaining owner methods are unspecified.

pub const CELL_WIDTH: f32 = 16.0;

/// World origin is the city's (0, 0) corner; conversion uses floor.
/// The shadow only supplies finite small coordinates, including negative edges.
pub fn position_to_cell(x: f32, z: f32) -> (i32, i32) {
    ((x / CELL_WIDTH).floor() as i32, (z / CELL_WIDTH).floor() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_at_both_sides_of_cell_edges() {
        assert_eq!(position_to_cell(-0.25, 15.75), (-1, 0));
        assert_eq!(position_to_cell(-16.25, 16.0), (-2, 1));
        assert_eq!(position_to_cell(1024.0, 4096.0), (64, 256));
    }
}
