// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link transport. No public DLL exports.
use fc4_core::city_geometry::{position_to_cell, CELL_WIDTH};

#[repr(C)]
struct Sample {
    x: f32,
    z: f32,
    cell_x: i32,
    cell_z: i32,
}

fn compare(samples: &[Sample], width_x: f32, width_z: f32) -> u32 {
    let mut differences = u32::from(width_x != CELL_WIDTH) + u32::from(width_z != CELL_WIDTH);
    for sample in samples {
        let (x, z) = position_to_cell(sample.x, sample.z);
        differences += u32::from(sample.cell_x != x) + u32::from(sample.cell_z != z);
    }
    differences
}

#[no_mangle]
unsafe extern "C" fn fc4_city_shadow_compare(samples: *const Sample, count: u32, wx: f32, wz: f32) -> u32 {
    if samples.is_null() || count != 8 {
        return u32::MAX;
    }
    // SAFETY: the plugin supplies eight readable C-layout samples.
    compare(unsafe { std::slice::from_raw_parts(samples, count as usize) }, wx, wz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_every_output_field_including_both_axes() {
        let samples = [Sample {
            x: -0.25,
            z: 16.0,
            cell_x: 0,
            cell_z: 0,
        }];
        assert_eq!(compare(&samples, 8.0, 32.0), 4);
        let samples = [Sample {
            x: -0.25,
            z: 16.0,
            cell_x: -1,
            cell_z: 1,
        }];
        assert_eq!(compare(&samples, 16.0, 16.0), 0);
    }
}
