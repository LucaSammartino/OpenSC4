// SPDX-License-Identifier: GPL-3.0-or-later
use super::numeric::E;
use super::*;
use base::Rect;

#[derive(Clone, Copy, Debug)]
pub struct PaintGeometry {
    /// Raw +24/+28, +34..+40 and +A8..+B4 words, respectively (MMW2).
    pub origin: [i32; 2],
    pub copy_area: Rect,
    pub marker_area: Rect,
}

#[derive(Clone, Debug)]
pub struct PaintPixels {
    pub format: u32,
    pub pitch: usize,
    pub pixels: Vec<u16>,
}

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
}

pub trait Paint {
    fn paint_geometry(&mut self, _window: Window) -> Result<PaintGeometry> {
        Err(Unsupported::Open("MMW2"))
    }
    fn paint_buffer_lock(&mut self, window: Window) -> Result<Option<PaintPixels>>;
    fn paint_buffer_unlock(&mut self, window: Window, pixels: PaintPixels);
    fn surface_source_rect(&mut self, surface: Handle) -> Rect;
    fn paint_blit(&mut self, window: Window, surface: Handle, source: Rect, destination: Rect);
    fn world_size(&mut self) -> [f32; 2];
    fn main_dimensions(&mut self) -> [i32; 2];
    fn eye_ray(&mut self, screen: [i32; 2]) -> Ray;
    fn pick_terrain_always(&mut self, ray: Ray, mode: u32) -> Result<[f32; 3]>;
    /// Unknown city slot-49 object, not a guessed simulator (MMW3).
    fn marker_service(&mut self) -> Result<Option<Handle>> {
        Err(Unsupported::Open("MMW3"))
    }
    /// Slot 9 returns a borrowed subject; no Release of an unacquired pointer.
    fn marker_subject(&mut self, service: Handle) -> Option<Handle>;
    fn marker_active(&mut self, service: Handle) -> bool;
    fn marker_position(&mut self, subject: Handle) -> [f32; 3];
    fn paint_buffer_color(&mut self, window: Window, color: u32);
    fn paint_buffer_points(&mut self, window: Window, mode: u32, points: [[f32; 2]; 2], count: u32);
}

/// §8.1, using remaining row/column counts for the matrix phase.
pub fn dither_rgb8_to_argb4(
    dst: &mut [u16],
    dst_pitch: usize,
    src: &[u32],
    src_pitch: usize,
    width: usize,
    height: usize,
) -> Result<()> {
    let fits = |len: usize, pitch: usize| {
        width == 0
            || height == 0
            || (pitch >= width
                && (height - 1)
                    .checked_mul(pitch)
                    .and_then(|v| v.checked_add(width))
                    .is_some_and(|v| v <= len))
    };
    if !fits(dst.len(), dst_pitch) || !fits(src.len(), src_pitch) {
        return Err(Unsupported::Precondition("4444 copy exceeds source/destination"));
    }
    const MATRIX: [[u32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    for y in 0..height {
        for x in 0..width {
            let d = MATRIX[(height - y) & 3][(width - x) & 3] << 24;
            let color = src[y * src_pitch + x];
            let mut pixel = 0xf000;
            for (source, target) in [(16, 8), (8, 4), (0, 0)] {
                pixel |= (((((color >> source) & 255) * 0x00f1_f1f1 + d) >> 28) as u16) << target;
            }
            dst[y * dst_pitch + x] = pixel;
        }
    }
    Ok(())
}

pub fn frustum_points(
    hits: [[f32; 3]; 4],
    size: [f32; 2],
    n: usize,
    rotation: u32,
    origin: [i32; 2],
) -> Result<[[i32; 2]; 4]> {
    if size[0] <= 1.0 || size[1] <= 1.0 || n == 0 {
        return Err(Unsupported::Precondition("positive frustum dimensions"));
    }
    let mut points = [[0; 2]; 4];
    for (point, hit) in points.iter_mut().zip(hits) {
        let mut u = E::input(hit[0])?
            .div(E::input(size[0])?.sub(E::constant(1.0)))
            .sub(E::constant(0.5));
        let mut v = E::input(hit[2])?
            .div(E::input(size[1])?.sub(E::constant(1.0)))
            .sub(E::constant(0.5));
        if rotation & 1 != 0 {
            (u, v) = (v, u.neg());
        }
        if rotation & 2 != 0 {
            u = u.neg();
            v = v.neg();
        }
        for (i, value) in [u, v].into_iter().enumerate() {
            let coordinate = value.add(E::constant(0.5)).mul(E::constant((n - 1) as f64)).floor();
            point[i] = origin[i]
                .checked_add(coordinate)
                .ok_or(Unsupported::Precondition("frustum integer extent"))?;
        }
    }
    Ok(points)
}

pub fn marker_points(position: [f32; 3], size: [f32; 2], area: Rect, rotation: u32) -> Result<[[f32; 2]; 2]> {
    if size[0] <= 0.0 || size[1] <= 0.0 {
        return Err(Unsupported::Precondition("positive marker world size"));
    }
    let [sx, sz] = [E::input(size[0])?, E::input(size[1])?];
    let (mut x, mut z) = (E::input(position[0])?, E::input(position[2])?);
    if rotation & 1 != 0 {
        (x, z) = (z, sz.sub(x));
    }
    if rotation & 2 != 0 {
        x = sx.sub(x);
        z = sz.sub(z);
    }
    let px = E::constant(f64::from(area.width())).mul(x).div(sx);
    let pz = E::constant(f64::from(area.height())).mul(z).div(sz);
    Ok([
        [px.sub(E::constant(1.0)).single(), pz.sub(E::constant(1.0)).single()],
        [px.add(E::constant(1.0)).single(), pz.add(E::constant(1.0)).single()],
    ])
}

impl MiniMap {
    fn paint_map(&self, host: &mut impl Host, geometry: PaintGeometry, pixels: &mut PaintPixels) -> Result<()> {
        let surface = self
            .surface
            .ok_or(Unsupported::Precondition("minimap surface before paint"))?;
        let code = if pixels.format == 7 { 0x8040 } else { 0x8010 };
        if let Some(lock) = host.surface_lock(surface, code)? {
            let result = if pixels.format == 7 {
                let Rect(x0, y0, x1, y1) = geometry.copy_area;
                if x0 < 0 || y0 < 0 || x1 < x0 || y1 < y0 || x1 as usize > pixels.pitch {
                    Err(Unsupported::Precondition("4444 destination rectangle"))
                } else {
                    let start = (y0 as usize)
                        .checked_mul(pixels.pitch)
                        .and_then(|v| v.checked_add(x0 as usize));
                    match start.and_then(|i| pixels.pixels.get_mut(i..)) {
                        Some(dst) => dither_rgb8_to_argb4(
                            dst,
                            pixels.pitch,
                            &lock.pixels,
                            lock.pitch,
                            (x1 - x0) as usize,
                            (y1 - y0) as usize,
                        ),
                        None => Err(Unsupported::Precondition("4444 destination offset")),
                    }
                }
            } else {
                let [x, y] = geometry.origin;
                let source = host.surface_source_rect(surface);
                host.paint_blit(
                    self.window,
                    surface,
                    source,
                    Rect(x, y, x + self.n as i32, y + self.n as i32),
                );
                Ok(())
            };
            host.surface_unlock(surface, lock);
            result?;
        }
        Ok(())
    }

    pub fn paint(&mut self, windows: &mut Windows, host: &mut impl Host) -> Result<bool> {
        if self.attached && self.map.pixels.is_none() {
            self.init_tz_map(host)?;
            self.update(windows, host)?;
        }
        let geometry = host.paint_geometry(self.window)?;
        if let Some(mut pixels) = host.paint_buffer_lock(self.window)? {
            let result = self.paint_map(host, geometry, &mut pixels);
            host.paint_buffer_unlock(self.window, pixels);
            result?;
        }
        let rotation = host.rotation();
        if self.flags & 4 != 0 {
            let [w, h] = host.main_dimensions();
            let mut hits = [[0.0; 3]; 4];
            for (hit, screen) in hits.iter_mut().zip([[0, 0], [w - 1, 0], [w - 1, h - 1], [0, h - 1]]) {
                let ray = host.eye_ray(screen);
                *hit = host.pick_terrain_always(ray, 1)?;
            }
            let points = frustum_points(hits, host.world_size(), self.n, rotation, geometry.origin)?;
            for i in 0..4 {
                let a = points[i];
                let b = points[(i + 1) % 4];
                host.draw(widget_controls::Draw::Line {
                    points: [a[0], a[1], b[0], b[1]],
                    color: 0x00ff_ffff,
                    style: 1,
                });
            }
        }
        if self.flags & 0x10 != 0 {
            if let Some(service) = host.marker_service()? {
                if let Some(subject) = host.marker_subject(service) {
                    let points = marker_points(
                        host.marker_position(subject),
                        host.world_size(),
                        geometry.marker_area,
                        rotation,
                    )?;
                    self.blink = !self.blink;
                    host.paint_buffer_color(self.window, if self.blink { 0xffc0_0000 } else { 0xffff_4040 });
                    host.paint_buffer_points(self.window, 3, points, 2);
                }
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dither_uses_remaining_counts_and_padded_rows() {
        let mut dst = [0; 6];
        dither_rgb8_to_argb4(&mut dst, 3, &[0x080808; 4], 2, 2, 2).unwrap();
        assert_eq!(dst, [0xf000, 0xf111, 0, 0xf111, 0xf000, 0]);
        let mut dst = [0; 4];
        dither_rgb8_to_argb4(&mut dst, 4, &[0x101010; 4], 4, 4, 1).unwrap();
        assert_eq!(dst, [0xf111, 0xf111, 0xf111, 0xf111]);
        assert!(dither_rgb8_to_argb4(&mut dst, 4, &[0], 1, 2, 2).is_err());
    }
    #[test]
    fn frustum_size_minus_one_and_marker_size_rotation() {
        let hits = [
            [0.0, 0.0, 0.0],
            [100.0, 0.0, 0.0],
            [100.0, 0.0, 100.0],
            [0.0, 0.0, 100.0],
        ];
        assert_eq!(
            frustum_points(hits, [101.0; 2], 11, 0, [3, 5]).unwrap(),
            [[3, 5], [13, 5], [13, 15], [3, 15]]
        );
        assert_eq!(
            frustum_points(hits, [101.0; 2], 11, 1, [3, 5]).unwrap(),
            [[3, 15], [3, 5], [13, 5], [13, 15]]
        );
        assert_eq!(
            marker_points([25.0, 0.0, 50.0], [100.0; 2], Rect(10, 20, 110, 220), 1).unwrap(),
            [[49.0, 149.0], [51.0, 151.0]]
        );
    }
}
