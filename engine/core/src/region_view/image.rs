// SPDX-License-Identifier: GPL-3.0-or-later
use super::{Result, Surface, Unsupported};
use crate::ui_windows::base::Rect;

pub trait SurfaceHost {
    fn dimensions(&self, image: Surface) -> (i32, i32);
    fn lock(&mut self, image: Surface, code: u32) -> bool;
    fn unlock(&mut self, image: Surface, code: u32);
    fn pixel(&self, image: Surface, x: i32, y: i32) -> u32;
    fn set_pixel(&mut self, image: Surface, x: i32, y: i32, colour: u32);
    fn fill(&mut self, image: Surface, colour: u32);
    fn tiled_background(&mut self, destination: Surface, source: Surface, offset: (i32, i32));
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coverage {
    pub x: i32,
    pub y: i32,
    pub values: Vec<u32>,
}

pub fn hit(runs: &[u32], x: i32, y: i32) -> bool {
    if !(0..=65535).contains(&x) || !(0..=65535).contains(&y) {
        return false;
    }
    let key = ((y as u32) << 16) + x as u32 + 1;
    runs.partition_point(|&word| word < key) % 2 == 1
}

pub(crate) fn rect(host: &impl SurfaceHost, image: Surface) -> Rect {
    let (w, h) = host.dimensions(image);
    Rect(0, 0, w, h)
}

fn over(under: u32, top: u32) -> u32 {
    let inverse = 255 - (top >> 24);
    let mut result = 0;
    for shift in [0, 8, 16, 24] {
        let channel = ((((under >> shift) & 255) * inverse + 128) >> 8) + ((top >> shift) & 255);
        result |= channel.min(255) << shift;
    }
    result
}

/// Mutates edge pixels, and splits runs whenever opacity class changes (§4.1).
pub fn build_runs(host: &mut impl SurfaceHost, image: Surface, runs: &mut Vec<u32>) -> Result<()> {
    let (w, h) = host.dimensions(image);
    if w == 0 || h == 0 {
        return Ok(());
    }
    if w > 65535 || h > 65535 {
        return Err(Unsupported::Precondition("16-bit image runs"));
    }
    runs.clear();
    if !host.lock(image, 0x8040) {
        return Err(Unsupported::Precondition("image lock"));
    }
    for y in 0..h {
        let mut class = 0;
        for x in 0..w {
            let mut c = host.pixel(image, x, y);
            if c < 0xff00_0000 {
                let right = if x + 1 < w { host.pixel(image, x + 1, y) } else { 0 };
                let below = if y + 1 < h { host.pixel(image, x, y + 1) } else { 0 };
                c = over(over(right, below), c);
                host.set_pixel(image, x, y, c);
            }
            let next = match c >> 24 {
                0 => 0,
                255 => 2,
                _ => 1,
            };
            if next != class {
                let word = ((y as u32) << 16) | x as u32;
                if class != 0 {
                    runs.push(word);
                }
                if next != 0 {
                    runs.push(word);
                }
                class = next;
            }
        }
        if class != 0 {
            runs.push(((y as u32) << 16) | w as u32);
        }
    }
    host.unlock(image, 0x8040);
    Ok(())
}

fn tinted(pixel: u32, colour: u32, a: u32) -> u32 {
    let mut result = pixel & 0xff00_0000;
    for shift in [0, 8, 16] {
        let d = i64::from((pixel >> shift) & 255);
        let c = i64::from((colour >> shift) & 255);
        let value = d + (((c - d) * i64::from(a) + 128) >> 8);
        result |= ((value as u32) & 255) << shift;
    }
    result
}

pub fn copy_and_tint(
    host: &mut impl SurfaceHost,
    src: Surface,
    dst: Surface,
    spans: Option<&[Coverage]>,
    colour: u32,
) -> Result<()> {
    let (w, h) = host.dimensions(src);
    let (dw, dh) = host.dimensions(dst);
    // SPEC-GAP: the native unchecked write is unsupported when the destination is too small.
    if dw < w || dh < h {
        return Err(Unsupported::Precondition("CopyAndTint destination size"));
    }
    if spans
        .into_iter()
        .flatten()
        .any(|s| s.x < 0 || s.y < 0 || s.y >= dh || i64::from(s.x) + s.values.len() as i64 > i64::from(dw))
    {
        return Err(Unsupported::Precondition("CopyAndTint coverage bounds"));
    }
    if !host.lock(dst, 0x8080) {
        return Err(Unsupported::Precondition("destination lock"));
    }
    if !host.lock(src, 0x800) {
        host.unlock(dst, 0x8080);
        return Err(Unsupported::Precondition("source lock"));
    }
    for y in 0..h {
        for x in 0..w {
            let c = host.pixel(src, x, y);
            host.set_pixel(dst, x, y, c);
        }
    }
    for span in spans.into_iter().flatten() {
        for (i, &coverage) in span.values.iter().enumerate() {
            let x = span.x + i as i32;
            let c = host.pixel(dst, x, span.y);
            host.set_pixel(dst, x, span.y, tinted(c, colour, (coverage & 255) >> 1));
        }
    }
    host.unlock(dst, 0x8080); // Compatibility: the source is deliberately left locked.
    Ok(())
}

pub fn fill_coverage(
    host: &mut impl SurfaceHost,
    dst: Surface,
    bounds: Rect,
    spans: &[Coverage],
    colour: u32,
) -> Result<()> {
    if !host.lock(dst, 0x8080) {
        return Err(Unsupported::Precondition("coverage lock"));
    }
    let target = rect(host, dst);
    for span in spans {
        let y = bounds.1 + span.y;
        for (i, &a) in span.values.iter().enumerate() {
            let x = bounds.0 + span.x + i as i32;
            if a != 0 && bounds.contains(x, y) && target.contains(x, y) {
                let d = host.pixel(dst, x, y);
                host.set_pixel(dst, x, y, tinted(d, colour, a));
            }
        }
    }
    host.unlock(dst, 0x8040); // The differing unlock code is specified.
    Ok(())
}

pub fn blit_image_spans(
    host: &mut impl SurfaceHost,
    src: Surface,
    dst: Surface,
    source: Rect,
    destination: Rect,
    runs: &[u32],
) -> Result<()> {
    let sr = rect(host, src);
    let dr = rect(host, dst);
    let offset = (destination.0 - source.0, destination.1 - source.1);
    let clip = Rect(
        source.0.max(sr.0).max(dr.0 - offset.0),
        source.1.max(sr.1).max(dr.1 - offset.1),
        source.2.min(sr.2).min(dr.2 - offset.0).min(destination.2 - offset.0),
        source.3.min(sr.3).min(dr.3 - offset.1).min(destination.3 - offset.1),
    );
    if clip.0 >= clip.2 || clip.1 >= clip.3 {
        return Ok(());
    }
    if !host.lock(dst, 0x8080) {
        return Err(Unsupported::Precondition("destination lock"));
    }
    if !host.lock(src, 0x40) {
        host.unlock(dst, 0x8080);
        return Err(Unsupported::Precondition("source lock"));
    }
    for run in runs.chunks_exact(2) {
        let y = (run[0] >> 16) as i32;
        if y < clip.1 || y >= clip.3 {
            continue;
        }
        let start = (run[0] & 65535) as i32;
        let end = (run[1] & 65535) as i32;
        let opaque = start < sr.2 && host.pixel(src, start, y) >> 24 == 255;
        for x in start.max(clip.0)..end.min(clip.2) {
            let s = host.pixel(src, x, y);
            let (dx, dy) = (x + offset.0, y + offset.1);
            if opaque {
                host.set_pixel(dst, dx, dy, s);
            } else if s >> 24 != 0 {
                let d = host.pixel(dst, dx, dy);
                host.set_pixel(dst, dx, dy, (d & 0xff00_0000) | (over(d, s) & 0x00ff_ffff));
            }
        }
    }
    host.unlock(src, 0x40);
    host.unlock(dst, 0x8080);
    Ok(())
}
