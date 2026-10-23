// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned integer byte-grid interpolation (public grid-interpolation spec, sections 2-6).
//!
//! Axes are outer/inner, with independently backed outer elements and no common
//! pitch or image orientation. Callers allocate all storage. These functions
//! neither allocate nor resize storage or change descriptors. Safe Rust borrows
//! require source/destination and mutable destination elements to be disjoint;
//! there is no in-place snapshot behavior.
//!
//! Shape/domain errors are a checked portable extension, not native recovery or
//! native empty-input behavior. GINT1 (caller allocation), GINT2 (larger native
//! counts), and GINT3 (overlap/malformed descriptors) remain unspecified here.

/// Logical counts, independent of the backing slices' possibly larger lengths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub outer: usize,
    pub inner: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grid {
    Source,
    Destination,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Outer,
    Inner,
}

/// Checked-extension errors, always returned before any destination write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnsupportedCount {
        grid: Grid,
        axis: Axis,
        count: usize,
    },
    OuterStorageTooShort {
        grid: Grid,
        required: usize,
        available: usize,
    },
    InnerStorageTooShort {
        grid: Grid,
        outer: usize,
        required: usize,
        available: usize,
    },
}

/// Zero-extend unsigned source bytes and store the interpolated low byte.
///
/// Counts must be in 1..=32767. Each grid must expose at least `outer`
/// elements, each backed by at least `inner` bytes. Extra storage is untouched.
/// Validation of both grids finishes before numerical work or writes begin.
pub fn interpolate_unsigned(
    source: &[&[u8]],
    source_shape: Shape,
    destination: &mut [&mut [u8]],
    destination_shape: Shape,
) -> Result<(), Error> {
    interpolate(source, source_shape, destination, destination_shape, 0)
}

/// Sign-extend signed source bytes, interpolate, then add 128 once at the store.
///
/// Storage and checked-domain requirements are the same as
/// [`interpolate_unsigned`]. Output bytes are unsigned indices, not a signed grid.
pub fn interpolate_signed(
    source: &[&[i8]],
    source_shape: Shape,
    destination: &mut [&mut [u8]],
    destination_shape: Shape,
) -> Result<(), Error> {
    interpolate(source, source_shape, destination, destination_shape, 128)
}

fn validate_shape(shape: Shape, grid: Grid) -> Result<(), Error> {
    for (axis, count) in [(Axis::Outer, shape.outer), (Axis::Inner, shape.inner)] {
        if !(1..=32767).contains(&count) {
            return Err(Error::UnsupportedCount { grid, axis, count });
        }
    }
    Ok(())
}

fn validate_storage(lengths: impl ExactSizeIterator<Item = usize>, shape: Shape, grid: Grid) -> Result<(), Error> {
    if lengths.len() < shape.outer {
        return Err(Error::OuterStorageTooShort {
            grid,
            required: shape.outer,
            available: lengths.len(),
        });
    }
    for (outer, available) in lengths.take(shape.outer).enumerate() {
        if available < shape.inner {
            return Err(Error::InnerStorageTooShort {
                grid,
                outer,
                required: shape.inner,
                available,
            });
        }
    }
    Ok(())
}

// Section 4.2: arithmetic shift, including negative deltas; ties go upward.
fn lerp(x: i32, y: i32, weight: i32) -> i32 {
    x.wrapping_add(y.wrapping_sub(x).wrapping_mul(weight).wrapping_add(32768) >> 16)
}

fn step(source: usize, destination: usize) -> i32 {
    // Validated counts make this signed 32-bit shifted dividend positive.
    (source as i32).wrapping_shl(16) / destination as i32
}

fn interpolate<T: Copy + Into<i32>>(
    source: &[&[T]],
    source_shape: Shape,
    destination: &mut [&mut [u8]],
    destination_shape: Shape,
    bias: i32,
) -> Result<(), Error> {
    validate_shape(source_shape, Grid::Source)?;
    validate_shape(destination_shape, Grid::Destination)?;
    validate_storage(source.iter().map(|row| row.len()), source_shape, Grid::Source)?;
    validate_storage(
        destination.iter().map(|row| row.len()),
        destination_shape,
        Grid::Destination,
    )?;

    let outer_step = step(source_shape.outer, destination_shape.outer);
    let inner_step = step(source_shape.inner, destination_shape.inner);
    let outer_high = ((source_shape.outer - 1) as i32).wrapping_shl(16);
    let inner_high = ((source_shape.inner - 1) as i32).wrapping_shl(16);
    let inner_start = (inner_step >> 1).wrapping_sub(32768);
    let mut u = (outer_step >> 1).wrapping_sub(32768);

    for row in destination.iter_mut().take(destination_shape.outer) {
        // Section 4.1: outer clamps compare signed coordinates.
        let (a, next_a, f) = if u < 0 {
            (0, 0, 0)
        } else if u >= outer_high {
            let last = source_shape.outer - 1;
            (last, last, 0)
        } else {
            let a = (u >> 16) as usize;
            (a, a + 1, u & 65535)
        };
        let first = source[a];
        let second = source[next_a];
        let mut v = inner_start;
        for byte in row.iter_mut().take(destination_shape.inner) {
            // Section 4.2: raw unsigned comparison rejects negative v as well.
            let value = if (v as u32) < inner_high as u32 {
                let b = (v >> 16) as usize;
                let g = v & 65535;
                let t0 = lerp(first[b].into(), first[b + 1].into(), g);
                let t1 = lerp(second[b].into(), second[b + 1].into(), g);
                lerp(t0, t1, f)
            } else {
                let b = if (v >> 16) < 0 { 0 } else { source_shape.inner - 1 };
                lerp(first[b].into(), second[b].into(), f)
            };
            *byte = value.wrapping_add(bias) as u8;
            // Even the unused terminal advances have machine-width semantics.
            v = v.wrapping_add(inner_step);
        }
        u = u.wrapping_add(outer_step);
    }
    Ok(())
}
