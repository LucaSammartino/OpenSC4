// SPDX-License-Identifier: GPL-3.0-or-later
//! QV4, QV8 and QV10: owner algorithms over raw foreign data.
use super::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarkedCells {
    pub coordinates: Vec<[usize; 2]>,
    pub colors: Vec<u32>,
    pub cell_size: [f32; 2],
}

pub fn marked_cells(grid: &RawMapGrid, palette: &RawPalette) -> Result<MarkedCells, Unsupported> {
    let indices: Vec<u8> = match &grid.values {
        GridValues::U8(v) => v.clone(),
        GridValues::I8(v) => v.iter().copied().map(i8_index).collect(),
        _ => return Err(Unsupported::SixteenBitConflict),
    };
    if grid.height == 0 || indices.len() != grid.width * grid.height {
        return Err(Unsupported::GridGeometry);
    }
    let mut cells = MarkedCells {
        cell_size: grid.cell_size,
        ..MarkedCells::default()
    };
    for (i, index) in indices.into_iter().enumerate() {
        let color = palette.lookup(index);
        if color >> 24 != 0 {
            cells.coordinates.push([i / grid.height, i % grid.height]);
            cells.colors.push(color);
        }
    }
    Ok(cells)
}

fn axis(j: usize, source: usize, destination: usize) -> (usize, usize, i64) {
    // Literal section 4.3 formula; its truncated step conflicts with vector 21
    // at the 2-to-3 centre (QV16). Do not silently substitute exact ratios.
    let step = (source as i64 * 65536) / destination as i64;
    let position = step / 2 - 32768 + j as i64 * step;
    if position <= 0 {
        return (0, 0, 0);
    }
    if position >= (source as i64 - 1) * 65536 {
        return (source - 1, source - 1, 0);
    }
    let low = (position / 65536) as usize;
    (low, low + 1, position & 65535)
}

fn lerp(a: i64, b: i64, fraction: i64) -> i64 {
    a + ((b - a) * fraction + 32768).div_euclid(65536)
}

/// X-major storage; inner z interpolation is rounded before outer x.
pub fn resample_grid(grid: &RawMapGrid, destination: [usize; 2]) -> Result<RawMapGrid, Unsupported> {
    let [width, height] = destination;
    if width == 0 || height == 0 || grid.width == 0 || grid.height == 0 {
        return Err(Unsupported::GridGeometry);
    }
    let source: Vec<i64> = match &grid.values {
        GridValues::U8(v) => v.iter().map(|v| i64::from(*v)).collect(),
        GridValues::I8(v) => v.iter().map(|v| i64::from(*v)).collect(),
        _ => return Err(Unsupported::SixteenBitConflict),
    };
    if source.len() != grid.width * grid.height
        || width.checked_mul(height).filter(|v| *v <= u32::MAX as usize).is_none()
    {
        return Err(Unsupported::GridGeometry);
    }
    let mut output = Vec::with_capacity(width * height);
    for x in 0..width {
        let (x0, x1, fx) = axis(x, grid.width, width);
        for z in 0..height {
            let (z0, z1, fz) = axis(z, grid.height, height);
            let a = lerp(source[x0 * grid.height + z0], source[x0 * grid.height + z1], fz);
            let b = lerp(source[x1 * grid.height + z0], source[x1 * grid.height + z1], fz);
            let value = lerp(a, b, fx);
            output.push(if matches!(grid.values, GridValues::I8(_)) {
                (value + 128) as u8
            } else {
                value as u8
            });
        }
    }
    Ok(RawMapGrid {
        width,
        height,
        cell_size: [16.0; 2],
        values: GridValues::U8(output),
    })
}

#[derive(Clone, Copy, Debug)]
pub struct MapLot {
    pub lot_type: u32,
    /// Inclusive x0,z0,x1,z1 in the cell grid.
    pub cells: [usize; 4],
}

pub fn block_lots(grid: &mut RawMapGrid, mask: u32, lots: &[MapLot]) -> Result<(), Unsupported> {
    let GridValues::U8(values) = &mut grid.values else {
        return Err(Unsupported::SixteenBitConflict);
    };
    // Means always read the source copy, even for overlapping lot rectangles.
    let source = values.clone();
    for lot in lots.iter().filter(|lot| mask & (1u32 << (lot.lot_type & 31)) != 0) {
        let [x0, z0, x1, z1] = lot.cells;
        if x1 >= grid.width || z1 >= grid.height || x0 > x1 || z0 > z1 || values.len() != grid.width * grid.height {
            return Err(Unsupported::GridGeometry);
        }
        let area = (x1 - x0 + 1) * (z1 - z0 + 1);
        let mut sum = 0i32;
        for x in x0..=x1 {
            for z in z0..=z1 {
                sum = sum.wrapping_add(i32::from(source[x * grid.height + z]));
            }
        }
        // Section 4.3: nearest integer, positive half up, minimum one.
        let value = ((sum + area as i32 / 2) / area as i32).max(1) as u8;
        for x in x0..=x1 {
            for z in z0..=z1 {
                values[x * grid.height + z] = value;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct WordGrid {
    pub width: usize,
    pub height: usize,
    pub shift: u8,
    pub words: Vec<u16>,
}

fn word_at(grid: &WordGrid, x: usize, z: usize, signed: bool) -> Result<i32, Unsupported> {
    let sample = |x: usize, z: usize| -> Result<i32, Unsupported> {
        if x >= grid.width || z >= grid.height {
            return Err(Unsupported::GridGeometry);
        }
        let word = *grid.words.get(x * grid.height + z).ok_or(Unsupported::BufferLength)?;
        Ok(if signed {
            i32::from(word as i16)
        } else {
            i32::from(word)
        })
    };
    if grid.shift >= 1 {
        let delta = grid.shift - 1;
        if delta >= usize::BITS as u8 {
            return Err(Unsupported::GridGeometry);
        }
        sample(x >> delta, z >> delta)
    } else {
        let mut sum = 0i32;
        for dx in 0..2 {
            for dz in 0..2 {
                sum = sum.wrapping_add(sample(x * 2 + dx, z * 2 + dz)?);
            }
        }
        Ok(sum >> 1 >> 1)
    }
}

/// Owned shift-one signed grid, developer slots 15 then 14, prior to QV5 display.
pub fn capacity_percentage(
    population: &WordGrid,
    capacity: &WordGrid,
    cells: [usize; 2],
) -> Result<RawMapGrid, Unsupported> {
    let [width, height] = [cells[0] / 2, cells[1] / 2];
    let mut words = Vec::with_capacity(width * height);
    for x in 0..width {
        for z in 0..height {
            let p = word_at(population, x, z, true)? as i16;
            let c = word_at(capacity, x, z, false)? as u16;
            words.push(if c == 0 {
                0
            } else {
                ((f64::from(p) * 100.0 / f64::from(c)).trunc() as i64) as i16
            });
        }
    }
    Ok(RawMapGrid {
        width,
        height,
        cell_size: [32.0; 2],
        values: GridValues::I16(words),
    })
}

#[derive(Clone, Debug)]
pub struct MapIncident {
    pub cell: [i32; 2],
    pub key: [u32; 3],
}
#[derive(Clone, Debug)]
pub struct IncidentIcon {
    pub handle: u64,
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

pub fn blend_incident(destination: u32, source: u32) -> u32 {
    let alpha = source >> 24;
    let da = destination >> 24;
    let mut result = (da - (da * alpha + 128) / 256 + alpha).min(255) << 24;
    for shift in [0, 8, 16] {
        let d = (destination >> shift) & 255;
        let s = (source >> shift) & 255;
        result |= ((d * (256 - alpha) + s * alpha + 128) / 256) << shift;
    }
    result
}

pub fn incident_center(cell: [i32; 2], map_width: usize, city_width: usize) -> [i32; 2] {
    let ratio = map_width as f32 / city_width as f32;
    [
        (cell[0] as f32 * ratio).trunc() as i32,
        (cell[1] as f32 * ratio).trunc() as i32,
    ]
}

pub fn blit_incident(
    pixels: &mut [u32],
    size: [usize; 2],
    icon: &IncidentIcon,
    center: [i32; 2],
    turn: u32,
) -> Result<(), Unsupported> {
    let [width, height] = size;
    if pixels.len() != width * height || icon.pixels.len() != icon.width * icon.height {
        return Err(Unsupported::BufferLength);
    }
    let turn = turn & 3;
    let (rw, rh) = if turn & 1 == 0 {
        (icon.width, icon.height)
    } else {
        (icon.height, icon.width)
    };
    let left = center[0] - (rw / 2) as i32;
    let top = center[1] - (rh / 2) as i32;
    for u in 0..rw {
        for v in 0..rh {
            let x = left + u as i32;
            let z = top + v as i32;
            if x < 0 || z < 0 || x >= width as i32 || z >= height as i32 {
                continue;
            }
            let (sx, sz) = match turn {
                0 => (u, v),
                1 => (icon.width - 1 - v, u),
                2 => (icon.width - 1 - u, icon.height - 1 - v),
                _ => (v, icon.height - 1 - u),
            };
            let i = x as usize * height + z as usize;
            pixels[i] = blend_incident(pixels[i], icon.pixels[sx * icon.height + sz]);
        }
    }
    Ok(())
}
