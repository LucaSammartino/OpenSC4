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

/// Bounded source-zero Update profile. Foreign queries are replayed once from
/// the invocation's tape; commands go to its recorder, never to the game.
pub fn replay_clear_update(
    state: [u32; 4],
    mut boundary: impl FnMut(u32) -> Result<u32, OwnerError>,
) -> Result<[u32; 2], OwnerError> {
    struct Host<F>(F);
    impl<F: FnMut(u32) -> Result<u32, OwnerError>> Renderer for Host<F> {
        fn default_zone_alpha(&mut self) -> Result<u8, OwnerError> {
            Err(OwnerError::Foreign(0))
        }
        fn child_bitmap_present(&mut self) -> Result<bool, OwnerError> {
            (self.0)(0).map(|v| v != 0)
        }
        fn render(&mut self, command: RenderCommand<'_>) -> Result<(), OwnerError> {
            if !matches!(command, RenderCommand::ClearMap) {
                return Err(OwnerError::Foreign(1));
            }
            (self.0)(1).map(|_| ())
        }
        fn clock_ms(&mut self) -> u32 {
            // The tape's validity is also checked by the caller: failure cannot
            // turn an absent native clock query into complete method credit.
            (self.0)(2).unwrap_or(0)
        }
    }
    struct NoSimulators;
    impl Simulators for NoSimulators {
        fn grid(&mut self, _: SourceSelection) -> Result<RawMapGrid, OwnerError> {
            Err(OwnerError::Foreign(2))
        }
        fn city_cells(&mut self) -> Result<[usize; 2], OwnerError> {
            Err(OwnerError::Foreign(3))
        }
        fn buildings(&mut self, _: HighlightRequest) -> Result<Vec<RawBuilding>, OwnerError> {
            Err(OwnerError::Foreign(4))
        }
    }
    let [active, dirty, last_clock, overlay_enabled] = state;
    let mut owner = MapView::new(u64::from(last_clock), 0);
    owner.active = active != 0;
    owner.dirty = dirty != 0;
    owner.overlay_enabled = overlay_enabled != 0;
    // The adapter admits only disabled or already-registered overlays.
    owner.overlay.plan(true, &[])?;
    owner.draw_with(&mut Host(&mut boundary), &mut NoSimulators)?;
    Ok([u32::from(owner.dirty), owner.last_redraw_ms as u32])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clear_update_replays_queries_and_clock_in_order() {
        let mut calls = Vec::new();
        let state = replay_clear_update([1, 1, 5000, 1], |operation| {
            calls.push(operation);
            Ok(if operation == 2 { 6007 } else { 1 })
        })
        .unwrap();
        assert_eq!(calls, [0, 1, 2]);
        assert_eq!(state, [0, 6007]);
        calls.clear();
        assert_eq!(
            replay_clear_update([0, 1, 5000, 1], |operation| {
                calls.push(operation);
                Ok(0)
            })
            .unwrap(),
            [1, 5000]
        );
        assert!(calls.is_empty());
        let mut missing = Vec::new();
        assert_eq!(
            replay_clear_update([1, 1, 5000, 0], |operation| {
                missing.push(operation);
                Ok(if operation == 2 { 7000 } else { 0 })
            })
            .unwrap(),
            [0, 7000]
        );
        assert_eq!(missing, [0, 2]);
    }
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
