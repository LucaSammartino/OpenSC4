// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent expected ledger: docs/specs/grid-interpolation.md section 7.
//! These literals were recorded before the production kernel was written or run.
//! They are proposed public-contract vectors, never executed-original goldens.

use fc4_core::grid_interpolation::{interpolate_signed, interpolate_unsigned, Axis, Error, Grid, Shape};

fn unsigned(source: &[&[u8]], expected: &[&[u8]]) {
    let source_shape = Shape {
        outer: source.len(),
        inner: source[0].len(),
    };
    let destination_shape = Shape {
        outer: expected.len(),
        inner: expected[0].len(),
    };
    let mut storage = vec![vec![213; destination_shape.inner]; destination_shape.outer];
    let mut destination: Vec<_> = storage.iter_mut().map(Vec::as_mut_slice).collect();
    interpolate_unsigned(source, source_shape, &mut destination, destination_shape).unwrap();
    for (actual, expected) in storage.iter().zip(expected) {
        assert_eq!(actual, expected);
    }
}

fn signed(source: &[&[i8]], expected: &[&[u8]]) {
    let source_shape = Shape {
        outer: source.len(),
        inner: source[0].len(),
    };
    let destination_shape = Shape {
        outer: expected.len(),
        inner: expected[0].len(),
    };
    let mut storage = vec![vec![213; destination_shape.inner]; destination_shape.outer];
    let mut destination: Vec<_> = storage.iter_mut().map(Vec::as_mut_slice).collect();
    interpolate_signed(source, source_shape, &mut destination, destination_shape).unwrap();
    for (actual, expected) in storage.iter().zip(expected) {
        assert_eq!(actual, expected);
    }
}

#[test]
fn g1_unsigned_expansion_clamps_and_asymmetry() {
    unsigned(
        &[&[0, 100], &[200, 255]],
        &[
            &[0, 25, 75, 100],
            &[50, 72, 117, 139],
            &[150, 167, 200, 216],
            &[200, 214, 241, 255],
        ],
    );
}

#[test]
fn g2_unsigned_reduction() {
    unsigned(
        &[&[0, 20, 80], &[100, 160, 200], &[220, 240, 255]],
        &[&[33, 96], &[198, 236]],
    );
}

#[test]
fn g3_quantized_step_middle_is_127() {
    unsigned(&[&[0, 20, 80], &[100, 160, 200]], &[&[5, 65], &[60, 127], &[115, 190]]);
}

#[test]
fn g4_two_rounds_produce_one() {
    unsigned(&[&[0, 0], &[0, 1]], &[&[1]]);
}

#[test]
fn g5_unsigned_equal_copy() {
    unsigned(&[&[0, 255, 7], &[19, 3, 200]], &[&[0, 255, 7], &[19, 3, 200]]);
}

#[test]
fn g6_signed_expansion() {
    signed(
        &[&[-128, -1], &[0, 127]],
        &[
            &[0, 32, 95, 127],
            &[32, 64, 127, 159],
            &[96, 128, 191, 223],
            &[128, 160, 223, 255],
        ],
    );
}

#[test]
fn g7_signed_equal_copy_bias() {
    signed(&[&[-128, -1, 0, 127]], &[&[0, 127, 128, 255]]);
}

#[test]
fn g8_signed_full_range_half() {
    signed(&[&[-128, 127]], &[&[128]]);
}

#[test]
fn g9_signed_negative_to_zero_tie() {
    signed(&[&[-1, 0]], &[&[128]]);
}

#[test]
fn g10_unsigned_singleton_axes() {
    unsigned(&[&[7]], &[&[7, 7, 7], &[7, 7, 7]]);
}

#[test]
fn g10_signed_singleton_axes() {
    signed(&[&[-1]], &[&[127, 127, 127], &[127, 127, 127]]);
}

#[test]
fn g11_single_destination_outer_samples_middle() {
    unsigned(&[&[0], &[100], &[255]], &[&[100, 100, 100, 100]]);
}

#[test]
fn g12_negative_half_rounds_up() {
    signed(&[&[-2, -1]], &[&[127]]);
}

#[test]
fn maximum_inner_count_single_output_terminal_advance_wraps() {
    // q=32767<<16, v0=16383<<16: only the central byte is sampled.
    // v0+q exceeds i32::MAX, but that advance has no subsequent consumer.
    // Source is just 32767 bytes, output is one byte, not a giant grid.
    let mut source = vec![9; 32767];
    source[16383] = 77;
    unsigned(&[&source], &[&[77]]);
}

#[test]
fn maximum_outer_count_two_outputs_quarters_and_terminal_wrap() {
    // p=1073709056; u0=8191.25 and u1=24574.75 in source units.
    // L(0,255,1/4)=64; L(255,0,3/4)=64. Final u+p wraps.
    // 32767 singleton outer elements, but only two output bytes.
    let mut source = vec![[13_u8]; 32767];
    source[8191][0] = 0;
    source[8192][0] = 255;
    source[24574][0] = 255;
    source[24575][0] = 0;
    let rows: Vec<_> = source.iter().map(|row| row.as_slice()).collect();
    unsigned(&rows, &[&[64], &[64]]);
}

#[test]
fn maximum_inner_signed_quarters_and_terminal_wrap() {
    // Same two coordinates as the preceding outer case, now inner.
    // L(127,-128,1/4)=63; L(-128,127,3/4)=63; final bias ->191.
    let mut source = vec![0_i8; 32767];
    source[8191] = 127;
    source[8192] = -128;
    source[24574] = -128;
    source[24575] = 127;
    signed(&[&source], &[&[191, 191]]);
}

#[test]
fn independently_backed_elements_and_descriptors_are_preserved() {
    let first = [93, 0, 100, 94];
    let second = [200, 255];
    let source: &[&[u8]] = &[&first[1..], &second];
    let mut a = [219; 5];
    let mut b = [219; 7];
    let mut c = [219; 4];
    let mut d = [219; 6];
    let mut spare = [219; 2];
    let mut destination: [&mut [u8]; 5] = [&mut a, &mut b[2..], &mut c, &mut d, &mut spare];
    let descriptors: Vec<_> = destination.iter().map(|row| (row.as_ptr(), row.len())).collect();
    interpolate_unsigned(
        source,
        Shape { outer: 2, inner: 2 },
        &mut destination,
        Shape { outer: 4, inner: 4 },
    )
    .unwrap();
    assert_eq!(
        destination
            .iter()
            .map(|row| (row.as_ptr(), row.len()))
            .collect::<Vec<_>>(),
        descriptors
    );
    assert_eq!(a, [0, 25, 75, 100, 219]);
    assert_eq!(b, [219, 219, 50, 72, 117, 139, 219]);
    assert_eq!(c, [150, 167, 200, 216]);
    assert_eq!(d, [200, 214, 241, 255, 219, 219]);
    assert_eq!(spare, [219, 219]);
    assert_eq!(first, [93, 0, 100, 94]);
    assert_eq!(second, [200, 255]);
}

#[test]
fn checked_extension_rejects_counts_before_writes() {
    // Explicit portable errors, not assertions about native faults or recovery.
    for grid in [Grid::Source, Grid::Destination] {
        for axis in [Axis::Outer, Axis::Inner] {
            for count in [0, 32768, usize::MAX] {
                let mut source_shape = Shape { outer: 1, inner: 1 };
                let mut destination_shape = source_shape;
                let shape = if grid == Grid::Source {
                    &mut source_shape
                } else {
                    &mut destination_shape
                };
                match axis {
                    Axis::Outer => shape.outer = count,
                    Axis::Inner => shape.inner = count,
                }
                let mut storage = [213];
                assert_eq!(
                    interpolate_unsigned(&[&[7]], source_shape, &mut [&mut storage], destination_shape),
                    Err(Error::UnsupportedCount { grid, axis, count })
                );
                assert_eq!(storage, [213]);
            }
        }
    }
}

#[test]
fn checked_extension_validates_all_elements_before_first_write() {
    let shape = Shape { outer: 2, inner: 2 };
    let mut first = [213, 213];
    let mut short = [213];
    assert_eq!(
        interpolate_signed(&[&[0, 1], &[2, 3]], shape, &mut [&mut first, &mut short], shape),
        Err(Error::InnerStorageTooShort {
            grid: Grid::Destination,
            outer: 1,
            required: 2,
            available: 1,
        })
    );
    assert_eq!(first, [213, 213]);
    assert_eq!(short, [213]);
    let mut destination = [[213; 2]; 2];
    let mut rows: Vec<_> = destination.iter_mut().map(|row| row.as_mut_slice()).collect();
    assert_eq!(
        interpolate_unsigned(&[&[0, 1], &[2]], shape, &mut rows, shape),
        Err(Error::InnerStorageTooShort {
            grid: Grid::Source,
            outer: 1,
            required: 2,
            available: 1,
        })
    );
    assert_eq!(
        interpolate_unsigned(&[&[0, 1]], shape, &mut rows, shape),
        Err(Error::OuterStorageTooShort {
            grid: Grid::Source,
            required: 2,
            available: 1,
        })
    );
    assert_eq!(
        interpolate_unsigned(&[&[0, 1], &[2, 3]], shape, &mut rows[..1], shape),
        Err(Error::OuterStorageTooShort {
            grid: Grid::Destination,
            required: 2,
            available: 1,
        })
    );
    assert_eq!(destination, [[213, 213], [213, 213]]);
}
