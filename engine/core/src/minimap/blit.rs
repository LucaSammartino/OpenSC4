// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// A lock's raw words and pitch in words. The adapter retains lock ownership
/// until unlock, and copies modifications back. No format interpretation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfacePixels {
    pub pixels: Vec<u32>,
    pub pitch: usize,
}

pub trait Surface {
    /// Framework service 0xC416025C, IID 0x0073283C, slot 3. Owned output.
    fn create_surface(&mut self, _service: u32, _iid: u32) -> Result<Option<Handle>> {
        Err(Unsupported::Open("MMW4"))
    }
    fn init_surface(&mut self, _surface: Handle, _n: usize, _format: [u32; 2]) -> Result<bool> {
        Err(Unsupported::Open("MMW4"))
    }
    fn shutdown_surface(&mut self, surface: Handle);
    /// Raw slot 6 lock code. None means lock failure, distinct from Unsupported.
    fn surface_lock(&mut self, _surface: Handle, _code: u32) -> Result<Option<SurfacePixels>> {
        Err(Unsupported::Open("MMW4"))
    }
    fn surface_unlock(&mut self, surface: Handle, pixels: SurfacePixels);
    /// Slots 30 and 18, in order, during Init. Kept raw under MMW4.
    fn surface_clear(&mut self, surface: Handle, color: [u8; 4], lock: &mut SurfacePixels);
}

pub fn visible_recursive(windows: &Windows, window: Window) -> bool {
    let mut current = Some(window);
    while let Some(w) = current {
        if !windows.get(w).flag(1) {
            return false;
        }
        current = windows.get(w).parent;
    }
    true
}

fn valid(p: &[u32], pitch: usize, n: usize) -> bool {
    n == 0
        || (pitch >= n
            && (n - 1)
                .checked_mul(pitch)
                .and_then(|v| v.checked_add(n))
                .is_some_and(|v| v <= p.len()))
}

fn offset(x: usize, y: usize, pitch: usize, n: usize, rotation: u32) -> usize {
    match rotation {
        1 => (n - 1) + x * pitch - y,
        2 => (n - 1 - y) * pitch + (n - 1 - x),
        3 => (n - 1 - x) * pitch + y,
        _ => y * pitch + x,
    }
}

fn blit(
    dst: &mut SurfacePixels,
    src: &[u32],
    pitch: usize,
    n: usize,
    rotation: u32,
    ov: Option<&SurfacePixels>,
) -> Result<()> {
    if !valid(src, pitch, n) || !valid(&dst.pixels, dst.pitch, n) || ov.is_some_and(|o| !valid(&o.pixels, o.pitch, n)) {
        return Err(Unsupported::Precondition("blit source/destination extent"));
    }
    for y in 0..n {
        for x in 0..n {
            let t = src[offset(x, y, pitch, n, rotation)];
            let mut pixel = t | 0xff00_0000;
            if let Some(ov) = ov {
                let v = ov.pixels[offset(x, y, ov.pitch, n, rotation)];
                let k = 256 - (v >> 24);
                pixel = 0xff00_0000;
                for shift in [0, 8, 16] {
                    let c = ((v >> shift) & 255) + ((((t >> shift) & 255) * k + 128) >> 8);
                    pixel |= c.min(255) << shift;
                }
            }
            dst.pixels[y * dst.pitch + x] = pixel;
        }
    }
    Ok(())
}

pub fn rotate_blt(
    host: &mut impl Surface,
    dst: Handle,
    src: &[u32],
    pitch: usize,
    n: usize,
    rotation: u32,
) -> Result<bool> {
    let Some(mut lock) = host.surface_lock(dst, 0x8040)? else {
        return Ok(false);
    };
    let result = blit(&mut lock, src, pitch, n, rotation, None);
    host.surface_unlock(dst, lock);
    result.map(|()| true)
}

pub fn rotate_blt_dual(
    host: &mut impl Surface,
    dst: Handle,
    src: &[u32],
    pitch: usize,
    n: usize,
    overlay: Handle,
    rotation: u32,
) -> Result<bool> {
    // SPEC-GAP: dual blits use the same opaque lock code as the single blit (MMW4).
    let Some(ov) = host.surface_lock(overlay, 0x8040)? else {
        return Ok(false);
    };
    let result = (|| {
        let Some(mut lock) = host.surface_lock(dst, 0x8040)? else {
            return Ok(false);
        };
        let result = blit(&mut lock, src, pitch, n, rotation, Some(&ov));
        host.surface_unlock(dst, lock);
        result.map(|()| true)
    })();
    host.surface_unlock(overlay, ov);
    result
}

impl MiniMap {
    pub(super) fn request_repaint(&self, windows: &mut Windows, host: &mut impl Host) {
        windows.invalidate_self_and_parents(self.window);
        host.invalidate(self.window);
    }

    pub fn update(&mut self, windows: &mut Windows, host: &mut impl Host) -> Result<()> {
        if !visible_recursive(windows, self.window) {
            self.map.free();
        }
        if self.map.pixels.is_none() || !self.surface_dirty {
            return Ok(());
        }
        let surface = self
            .surface
            .ok_or(Unsupported::Precondition("minimap surface before Update"))?;
        let rotation = host.rotation();
        if let Some(overlay) = self.overlay.filter(|_| self.flags & 8 != 0) {
            if let Some(ov) = host.surface_lock(overlay, 0x8040)? {
                let result = rotate_blt(host, surface, &ov.pixels, ov.pitch, self.n, rotation);
                host.surface_unlock(overlay, ov);
                result?;
            }
        } else {
            if self.map_dirty {
                self.update_terrain_and_zone_map(host)?;
            }
            if self.flags & 2 != 0 {
                self.update_networks(host)?;
            }
            let pixels = self.map.pixels.as_deref().unwrap();
            if let Some(overlay) = self.overlay {
                rotate_blt_dual(host, surface, pixels, self.map.width, self.n, overlay, rotation)?;
            } else {
                rotate_blt(host, surface, pixels, self.map.width, self.n, rotation)?;
            }
        }
        self.surface_dirty = false;
        self.request_repaint(windows, host);
        self.repaint = false;
        self.hook_occupant_messages(host);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::fixture;
    use super::*;

    #[test]
    fn mm_c7_all_rotations_pitches_and_premultiplied_overlay() {
        let (_, mut h, _) = fixture();
        h.surfaces.insert(
            10,
            SurfacePixels {
                pixels: vec![0; 8],
                pitch: 4,
            },
        );
        let src = [1, 2, 99, 3, 4];
        for (r, expected) in [
            (0, [1, 2, 3, 4]),
            (1, [2, 4, 1, 3]),
            (2, [4, 3, 2, 1]),
            (3, [3, 1, 4, 2]),
            (9, [1, 2, 3, 4]),
        ] {
            rotate_blt(&mut h, 10, &src, 3, 2, r).unwrap();
            let p = &h.surfaces[&10].pixels;
            assert_eq!([p[0], p[1], p[4], p[5]], expected.map(|v| v | 0xff000000));
            assert_eq!([p[2], p[3], p[6], p[7]], [0; 4]);
        }
        h.surfaces.insert(
            20,
            SurfacePixels {
                pixels: vec![0x80646464],
                pitch: 1,
            },
        );
        rotate_blt_dual(&mut h, 10, &[0xff808080], 1, 1, 20, 0).unwrap();
        assert_eq!(h.surfaces[&10].pixels[0], 0xffa4a4a4);
        h.fail_lock = Some(10);
        assert!(!rotate_blt_dual(&mut h, 10, &[0], 1, 1, 20, 0).unwrap());
        assert_eq!(h.events.last().unwrap(), "unlock:20");
        h.fail_lock = None;
        assert!(rotate_blt(&mut h, 10, &[], 0, 2, 0).is_err());
        assert_eq!(h.events.last().unwrap(), "unlock:10");
    }

    #[test]
    fn invisible_parent_frees_pixels_and_update_tail_rehooks() {
        let (mut w, mut h, mut m) = fixture();
        let parent = w.create_default(&mut h);
        w.get_mut(m.window).parent = Some(parent);
        m.n = 1;
        m.map.init(1, 1).unwrap();
        w.get_mut(parent).flags &= !1;
        m.update(&mut w, &mut h).unwrap();
        assert!(m.map.pixels().is_none());
        w.get_mut(parent).flags |= 1;
        m.map.init(1, 1).unwrap();
        m.attached = true;
        m.surface = Some(10);
        m.surface_dirty = true;
        m.repaint = true;
        m.flags = 0;
        h.surfaces.insert(
            10,
            SurfacePixels {
                pixels: vec![0],
                pitch: 1,
            },
        );
        m.update(&mut w, &mut h).unwrap();
        assert!(!m.surface_dirty && !m.repaint && m.hooked);
        assert_eq!(h.surfaces[&10].pixels, [0xff000000]);
    }
}
