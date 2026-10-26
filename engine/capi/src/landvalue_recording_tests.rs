// SPDX-License-Identifier: GPL-3.0-or-later
//! landvalue §6.1.6 / §16.36: preserved numeric observations, not Step inputs.

use super::*;

#[test]
fn lv10_march6_base1_selected_native_bytes_are_not_replaced_by_sampled_replay() {
    // lvterrain27-repro-base1, March 6 pass 3: full-map intrinsic 930/4096,
    // total 637/4096, max delta 45; slope/types and scalars exact. Overall 3/4.
    // These are only the logged cells. Unknown bytes are excluded from this
    // reduced comparison, never filled into an asserted native golden map.
    let g = GridGeometry::for_city(
        &CityCells {
            cells_x: 2,
            cells_z: 16,
            cell_width_x: 16.0,
            cell_width_z: 16.0,
        },
        2,
    );
    let intrinsic = diff_u8(
        &g,
        &[1, 1, 14, 15, 11, 10, 10, 10],
        &[46, 46, 54, 50, 41, 35, 30, 25],
        8,
    );
    assert_eq!((intrinsic.mismatched, intrinsic.max), (8, 45.0));
    assert_eq!(intrinsic.first[0], "(0,0) 1/46");
    // Total's final logged cell is (1,0), rather than (0,7).
    let mut total = Diff::new(4096);
    for (x, z, native, sampled) in [
        (0, 0, 16u8, 61u8),
        (0, 1, 16, 61),
        (0, 2, 24, 64),
        (0, 3, 25, 60),
        (0, 4, 8, 38),
        (0, 5, 7, 32),
        (0, 6, 1, 5),
        (1, 0, 16, 15),
    ] {
        total.note(x, z, native, sampled, f64::from(native.abs_diff(sampled)), 8);
    }
    assert_eq!((total.mismatched, total.max), (8, 45.0));
    assert_eq!(total.first[7], "(1,0) 16/15");
}

#[test]
fn lv10_march6_recordings_keep_separate_scalars_and_probe_diagnostic() {
    // The named recordings have different averages despite equal mean/mode.
    // base1: March 6 (72), April 6 exact (67); inputs1: March 6 exact (73),
    // but only 1/4 exact overall. Neither has an eligible native capture host.
    let base1 = (270.02063f32, 272.0f32, 72u8);
    let inputs1 = (270.02063f32, 272.0f32, 73u8);
    assert!(same_f32(base1.0, inputs1.0));
    assert!(same_f32(base1.1, inputs1.1));
    assert_ne!(base1.2, inputs1.2);
    // inputs1 February 6: the day-3 diagnostic must not erase this mismatch.
    assert!(!same_f32(270.02023, 270.02063));
}
