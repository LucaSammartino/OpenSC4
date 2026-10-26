// SPDX-License-Identifier: GPL-3.0-or-later
use super::numeric::E;
use super::*;
use base::Rect;
use std::cmp::Ordering;

/// Raw simulator data. Blocks are indexed local_x*16+local_z; adapters may
/// arrange slot-15/16 rows, but must not shade, blend, zone or resample pixels.
pub trait Terrain {
    /// Slots 7/8 and 29; None is the absent-terrain path.
    fn terrain_info(&mut self) -> Option<([usize; 2], f32)>;
    fn terrain_heights(&mut self, rect: Rect) -> Result<[f32; 256]>;
    /// MMW1 is open: no interpretation as metres/depth is supplied here.
    fn terrain_water_values(&mut self, _rect: Rect) -> Result<[f32; 256]> {
        Err(Unsupported::Open("MMW1"))
    }
    fn terrain_normal(&mut self, x: usize, z: usize) -> Result<[f32; 3]>;
    /// Signed byte rows from zones slot 5, grid slot 25, addressed (x,z).
    fn zone_at(&mut self, x: usize, z: usize) -> Result<i8>;
}

fn index_for_height(height: E, sea: E) -> usize {
    // SPEC-GAP: decimal constants without a stated operand width use binary64.
    (64i32.saturating_add(height.sub(sea).mul(E::constant(0.2)).floor())).clamp(0, 255) as usize
}

fn brightness(height: E, sea: E, normal: [f32; 3], rotation: u32) -> Result<u32> {
    let mut n = if height.cmp(sea) == Ordering::Less {
        [E::constant(0.0), E::constant(1.0), E::constant(0.0)]
    } else {
        [
            E::input(normal[0])?,
            E::input(normal[1])?.mul(E::constant(0.4)),
            E::input(normal[2])?,
        ]
    };
    if height.cmp(sea) != Ordering::Less {
        let length = n[0].mul(n[0]).add(n[1].mul(n[1])).add(n[2].mul(n[2])).sqrt();
        if length.cmp(E::constant(0.0)) == Ordering::Equal {
            return Err(Unsupported::Precondition("nonzero terrain normal"));
        }
        let reciprocal = E::constant(1.0).div(length);
        for v in &mut n {
            *v = v.mul(reciprocal);
        }
    }
    let (lx, lz) = match rotation {
        1 => (0.57735, -0.57735),
        2 => (0.57735, 0.57735),
        3 => (-0.57735, 0.57735),
        _ => (-0.57735, -0.57735),
    };
    let l = [lx, 0.57735026, lz].map(|v| E::constant(v).mul(E::constant(63.75)));
    let d = n[0].mul(l[0]).add(n[1].mul(l[1])).add(n[2].mul(l[2]));
    Ok(d.abs().add(d).add(E::constant(127.75)).rhe() as u32)
}

fn shade(color: u32, intensity: u32, water: u32) -> u32 {
    let alpha = i64::from(water >> 24);
    let mut out = 0;
    for shift in [0, 8, 16] {
        let s = i64::from((((color >> shift) & 255) * intensity + 128) >> 8);
        let w = i64::from((water >> shift) & 255);
        out |= (((s * 256 + (w - s) * alpha + 128) >> 8) as u32) << shift;
    }
    out
}

/// The five specified resamplers. Other shifts skip the block.
pub fn resample_block(block: &[u32; 256], shift: i32) -> Option<(Vec<u32>, usize)> {
    let side = match shift {
        0 => 16,
        1 => 8,
        2 => 4,
        -1 => 32,
        -2 => 64,
        _ => return None,
    };
    let mut out = vec![0; side * side];
    if shift <= 0 {
        let factor = side / 16;
        for x in 0..side {
            for z in 0..side {
                out[x * side + z] = block[(x / factor) * 16 + z / factor];
            }
        }
    } else {
        let factor = 1usize << shift;
        let count = (factor * factor) as u32;
        for x in 0..side {
            for z in 0..side {
                for lane in [0, 8, 16] {
                    let mut sum = 0;
                    for dx in 0..factor {
                        for dz in 0..factor {
                            sum += (block[(x * factor + dx) * 16 + z * factor + dz] >> lane) & 255;
                        }
                    }
                    out[x * side + z] |= ((sum + count / 2) / count) << lane;
                }
            }
        }
    }
    Some((out, side))
}

impl MiniMap {
    pub fn invalidate_tz_map_rect(&mut self, rect: Rect) -> Result<()> {
        if rect.0 < 0 || rect.1 < 0 {
            return Err(Unsupported::Precondition(
                "negative dirty block index (unclamped native lower bound)",
            ));
        }
        for bz in (rect.1 >> 4)..=(rect.3 >> 4).min(15) {
            for bx in (rect.0 >> 4)..=(rect.2 >> 4).min(15) {
                self.dirty[bz as usize] |= 1 << bx;
            }
        }
        self.map_dirty = true;
        self.surface_dirty = true;
        Ok(())
    }

    pub fn update_terrain_and_zone_map(&mut self, host: &mut impl Host) -> Result<()> {
        let Some(([cx, cz], sea)) = host.terrain_info() else {
            return Ok(());
        };
        let sea = E::input(sea)?;
        if cx >> 4 > 16 || cz >> 4 > 16 || self.map.pixels.is_none() {
            return Err(Unsupported::Precondition("terrain dirty-grid bounds and allocated map"));
        }
        let rotation = host.rotation();
        for bz in 0..(cz >> 4) {
            for bx in 0..(cx >> 4) {
                if self.dirty[bz] & (1 << bx) == 0 {
                    continue;
                }
                let (x0, z0) = (bx * 16, bz * 16);
                let rect = Rect(x0 as i32, z0 as i32, (x0 + 15) as i32, (z0 + 15) as i32);
                let heights = host.terrain_heights(rect)?;
                let water = host.terrain_water_values(rect)?;
                let mut block = [0; 256];
                for x in 0..16 {
                    for z in 0..16 {
                        let i = x * 16 + z;
                        let h = E::input(heights[i])?;
                        let normal = if h.cmp(sea) == Ordering::Less {
                            [0.0, 1.0, 0.0]
                        } else {
                            host.terrain_normal(x0 + x, z0 + z)?
                        };
                        let intensity = brightness(h, sea, normal, rotation)?;
                        let wi = E::input(water[i])?.floor().clamp(0, 255) as usize;
                        let mut c = shade(
                            self.terrain_palette[index_for_height(h, sea)],
                            intensity,
                            self.water_palette[wi],
                        );
                        if self.flags & 1 != 0 {
                            let zone = host.zone_at(x0 + x, z0 + z)?;
                            let color = self
                                .zone_colors
                                .get(zone as usize)
                                .ok_or(Unsupported::Precondition("zone signed index outside 0..16"))?;
                            if *color != 0 {
                                c = *color;
                            }
                        }
                        block[i] = c;
                    }
                }
                if let Some((pixels, side)) = resample_block(&block, self.shift) {
                    let (row, col) = if self.shift >= 0 {
                        (x0 >> self.shift, z0 >> self.shift)
                    } else {
                        (x0 << -self.shift, z0 << -self.shift)
                    };
                    let dst = self.map.pixels.as_mut().unwrap();
                    if col + side > self.n || (row + side).checked_mul(self.n).map_or(true, |end| end > dst.len()) {
                        return Err(Unsupported::Precondition("resampled block exceeds pixel allocation"));
                    }
                    for r in 0..side {
                        dst[(row + r) * self.n + col..(row + r) * self.n + col + side]
                            .copy_from_slice(&pixels[r * side..(r + 1) * side]);
                    }
                }
            }
        }
        self.map_dirty = false;
        self.dirty.fill(0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::fixture;
    use super::*;

    #[test]
    fn mm_c4_c5_height_and_underwater_brightness() {
        assert_eq!(index_for_height(E::constant(110.0), E::constant(100.0)), 66);
        assert_eq!(
            brightness(E::constant(90.0), E::constant(100.0), [f32::NAN; 3], 0).unwrap(),
            201
        );
        assert_eq!(shade(0xffffff, 201, 0), 0xc8c8c8);
        assert_eq!(shade(0xffffff, 201, 0xff102030), 0x112131);
    }
    #[test]
    fn dirty_rect_inclusive_upper_clamp_and_negative_boundary() {
        let (_, _, mut m) = fixture();
        m.invalidate_tz_map_rect(Rect(15, 16, 256, 31)).unwrap();
        assert_eq!(m.dirty[1], 0xffff);
        assert_eq!(m.dirty[0], 0);
        assert!(m.map_dirty && m.surface_dirty);
        assert!(m.invalidate_tz_map_rect(Rect(-1, 0, 0, 0)).is_err());
    }
    #[test]
    fn all_five_resamplers_and_skip() {
        let block = std::array::from_fn(|i| 0xff000000 | (i as u32) * 0x010101);
        assert_eq!(resample_block(&block, 0).unwrap().0, block);
        assert_eq!(resample_block(&block, 1).unwrap().0[0], 0x090909);
        assert_eq!(resample_block(&block, 2).unwrap().0[0], 0x1a1a1a);
        for (s, side, repeat) in [(-1, 32, 2), (-2, 64, 4)] {
            let (p, n) = resample_block(&block, s).unwrap();
            assert_eq!(n, side);
            assert_eq!(p[repeat - 1], block[0]);
            assert_eq!(p[repeat], block[1]);
            assert_eq!(p[repeat * side], block[16]);
        }
        assert!(resample_block(&block, 3).is_none());
    }
}
