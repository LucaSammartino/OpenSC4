// SPDX-License-Identifier: GPL-3.0-or-later
use crate::{catch_unwind, AssertUnwindSafe};

#[no_mangle]
pub unsafe extern "C" fn fc4_drawstream_raster(
    cells: *const u8,
    outer: u32,
    inner: u32,
    width: u32,
    height: u32,
    palette: *const u32,
    rotation: u32,
    signed: bool,
    pixels: *mut u32,
) -> bool {
    catch_unwind(AssertUnwindSafe(|| {
        let count = u64::from(outer) * u64::from(inner);
        let area = u64::from(width) * u64::from(height);
        if cells.is_null()
            || palette.is_null()
            || pixels.is_null()
            || count == 0
            || count > 4_194_304
            || area == 0
            || area > 4_194_304
        {
            return false;
        }
        let cells = std::slice::from_raw_parts(cells, count as usize);
        let palette = &*(palette as *const [u32; 256]);
        let pixels = std::slice::from_raw_parts_mut(pixels, area as usize);
        fc4_core::map_view::raster_bytes(
            cells,
            outer as usize,
            inner as usize,
            [width as usize, height as usize],
            palette,
            rotation,
            signed,
            pixels,
        )
        .is_ok()
    }))
    .unwrap_or(false)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_drawstream_notification(
    active: bool,
    dirty: bool,
    message: u32,
    subscriptions: *const u32,
    count: u32,
    values: *const u32,
    output: *mut u32,
) -> bool {
    catch_unwind(AssertUnwindSafe(|| {
        if output.is_null() || count > 4096 || (count != 0 && subscriptions.is_null()) {
            return false;
        }
        let subscriptions = if count == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(subscriptions, count as usize)
        };
        let values = if values.is_null() {
            None
        } else {
            Some([*values, *values.add(1)])
        };
        let Ok(result) = fc4_core::map_view::replay_notification(active, dirty, message, subscriptions, values) else {
            return false;
        };
        std::ptr::copy_nonoverlapping(result.as_ptr(), output, 3);
        true
    }))
    .unwrap_or(false)
}
