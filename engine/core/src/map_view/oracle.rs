// SPDX-License-Identifier: GPL-3.0-or-later
//! Same-invocation bitmap and notification replay. No native objects are called.
use super::*;

/// map-color-buffer.md §§1–2. Logical rows exclude native pitch padding.
#[allow(clippy::too_many_arguments)]
pub fn raster_bytes(
    cells: &[u8],
    outer: usize,
    inner: usize,
    size: [usize; 2],
    palette: &[u32; 256],
    rotation: u32,
    signed: bool,
    pixels: &mut [u32],
) -> Result<(), Unsupported> {
    let [width, height] = size;
    if outer == 0
        || inner == 0
        || rotation > 3
        || width == 0
        || height == 0
        || width > 4096
        || height > 4096
        || outer.checked_mul(inner) != Some(cells.len())
        || width.checked_mul(height) != Some(pixels.len())
    {
        return Err(Unsupported::GridGeometry);
    }
    let (mut w, mut h, mut sx, mut sy) = (width, height, 1usize, 1usize);
    while w > outer {
        w >>= 1;
        sx *= 2;
    }
    while h > inner {
        h >>= 1;
        sy *= 2;
    }
    let (w, h, sx, sy) = (w as i64, h as i64, sx as i64, sy as i64);
    let (x, y, dx, dy, ex, ey) = match rotation {
        0 => (0, 0, sx, 0, 0, sy),
        1 => (0, (h - 1) * sy, 0, -sx, sy, 0),
        2 => ((w - 1) * sx, (h - 1) * sy, -sx, 0, 0, -sy),
        _ => ((w - 1) * sx, 0, 0, sx, -sy, 0),
    };
    let location = |i: usize, j: usize| (x + i as i64 * dx + j as i64 * ex, y + i as i64 * dy + j as i64 * ey);
    // Refuse native out-of-allocation geometry before changing the caller's rows.
    for i in 0..outer {
        for j in 0..inner {
            let (x, y) = location(i, j);
            if x < 0 || y < 0 || x + sx > width as i64 || y + sy > height as i64 {
                return Err(Unsupported::GridGeometry);
            }
        }
    }
    let mut colors = vec![0; cells.len()];
    let table = RawPalette(*palette);
    if signed {
        let values: Vec<i8> = cells.iter().map(|v| *v as i8).collect();
        emit_colors(Values::I8(&values), &table, &mut colors)?;
    } else {
        emit_colors(Values::U8(cells), &table, &mut colors)?;
    }
    for i in 0..outer {
        for j in 0..inner {
            let (x, y) = location(i, j);
            for row in y..y + sy {
                let start = row as usize * width + x as usize;
                pixels[start..start + sx as usize].fill(colors[i * inner + j]);
            }
        }
    }
    Ok(())
}

/// Notification owner fields/effect count, seeded from the live entry snapshot.
/// Animation ticks require the full renderer/simulator adapter and are refused.
pub fn replay_notification(
    active: bool,
    dirty: bool,
    message: u32,
    subscriptions: &[u32],
    values: Option<[u32; 2]>,
) -> Result<[u32; 3], Unsupported> {
    if message == 0x86ad_10ee {
        return Err(Unsupported::HighlightComparison);
    }
    let mut owner = MapView::new(0, 0);
    owner.active = active;
    owner.dirty = dirty;
    let effects = if message == VALUE_CHANGED {
        let [old, new] = values.ok_or(Unsupported::MissingNotificationValue)?;
        owner.notify_values(old, new);
        Vec::new()
    } else {
        owner.notify(message, None)?
    };
    owner.dirty |= subscriptions.contains(&message);
    Ok([u32::from(owner.active), u32::from(owner.dirty), effects.len() as u32])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ds29_developer_layer_two_surface() {
        let factors = ScaleFactors::from_scale(Some(256));
        let cells = [0, 64, 128, 256].map(|v| factors.unsigned_index(v));
        assert_eq!(cells, [0, 63, 127, 255]);
        let palette = std::array::from_fn(|k| premultiply_argb(0xff00_0000 + k as u32));
        let mut pixels = [0; 4];
        raster_bytes(&cells, 2, 2, [2, 2], &palette, 0, false, &mut pixels).unwrap();
        assert_eq!(pixels, [0xff00_0000, 0xff00_007f, 0xff00_003f, 0xff00_00fe]);
    }
    #[test]
    fn signed_rotation_and_refused_geometry() {
        let palette = std::array::from_fn(|k| k as u32);
        let mut pixels = [99; 16];
        raster_bytes(&[128, 0, 127, 255], 2, 2, [4, 4], &palette, 2, true, &mut pixels).unwrap();
        assert_eq!(
            pixels,
            [127, 127, 128, 128, 127, 127, 128, 128, 255, 255, 0, 0, 255, 255, 0, 0]
        );
        let before = pixels;
        assert!(raster_bytes(&[0; 4], 2, 2, [4, 4], &palette, 4, false, &mut pixels).is_err());
        assert_eq!(pixels, before);
    }
    #[test]
    fn notifications_preserve_or_dirty_from_same_input() {
        assert_eq!(replay_notification(true, false, 123, &[123], None), Ok([1, 1, 0]));
        assert_eq!(replay_notification(false, true, 42, &[], None), Ok([0, 1, 0]));
        assert_eq!(
            replay_notification(true, false, VALUE_CHANGED, &[], Some([7, 7])),
            Ok([1, 0, 0])
        );
        assert_eq!(
            replay_notification(true, false, VALUE_CHANGED, &[], Some([7, 8])),
            Ok([1, 1, 0])
        );
    }
}
