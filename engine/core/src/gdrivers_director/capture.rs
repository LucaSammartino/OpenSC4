// SPDX-License-Identifier: GPL-3.0-or-later
//! Front/software bitmap capture algorithms; bitmap and GL primitives stay foreign.
use super::{
    call,
    software::{address, strided_copy, Frame},
    zero, Allocation, Host, Op, Result,
};

fn clipped([x, y, width, height]: [i32; 4], bounds: [i32; 2]) -> [i32; 4] {
    let right = x.wrapping_add(width).min(bounds[0]);
    let bottom = y.wrapping_add(height).min(bounds[1]);
    let x = x.max(0);
    let y = y.max(0);
    [x, y, right.wrapping_sub(x), bottom.wrapping_sub(y)]
}
fn initialized(bitmap: u32, host: &mut impl Host) -> bool {
    call(host, bitmap, Op::BitmapInitialized, &[]) as u8 != 0
}
fn format_ok(bitmap: u32, host: &mut impl Host) -> bool {
    let mut descriptor = [3, 16];
    host.call(bitmap, Op::BitmapFormat, &[], &[], &mut descriptor);
    descriptor[0] == 9
}
fn limit(bitmap: u32, width: &mut i32, height: &mut i32, host: &mut impl Host) {
    *width = (*width).min(call(host, bitmap, Op::BitmapWidth, &[]) as i32);
    *height = (*height).min(call(host, bitmap, Op::BitmapHeight, &[]) as i32);
}
fn init(bitmap: u32, width: i32, height: i32, host: &mut impl Host) -> bool {
    call(
        host,
        bitmap,
        Op::BitmapInitialize,
        &[width as u32, height as u32, 9, 32],
    ) as u8
        != 0
}
pub fn front(rectangle: [i32; 4], mut bitmap: u32, host: &mut impl Host) -> Result<u32> {
    let mut viewport = [0; 4];
    host.call(0, Op::GlViewport, &[0x0ba2], &[], &mut viewport);
    let [x, y, mut width, mut height] = clipped(rectangle, [viewport[2] as i32, viewport[3] as i32]);
    let size = (width as u32).wrapping_mul(height as u32).wrapping_mul(3);
    let scratch = host.alloc(size, Allocation::Heap);
    if scratch == 0 {
        return Ok(bitmap);
    }
    let service = call(host, 0, Op::GraphicsService, &[]);
    if bitmap != 0 && initialized(bitmap, host) {
        if !format_ok(bitmap, host) {
            return Ok(0);
        } // Native scratch leak, deliberately preserved.
        limit(bitmap, &mut width, &mut height, host);
    }
    if bitmap == 0 && service != 0 {
        host.call(service, Op::BitmapCreate, &[], &[], std::slice::from_mut(&mut bitmap));
    }
    if bitmap == 0 {
        host.free(scratch, Allocation::Heap);
        return Ok(0);
    }
    zero(host, scratch, size)?;
    call(host, 0, Op::GlReadBuffer, &[0x0405]);
    call(host, 0, Op::GlPackAlignment, &[0x0d05, 1]);
    call(
        host,
        0,
        Op::GlReadPixels,
        &[
            x as u32,
            (viewport[3] as i32).wrapping_sub(y).wrapping_sub(height) as u32,
            width as u32,
            height as u32,
            0x1907,
            0x1401,
            scratch,
        ],
    );
    if !initialized(bitmap, host) && !init(bitmap, width, height, host) {
        host.free(scratch, Allocation::Heap);
        return Ok(bitmap);
    }
    if call(host, bitmap, Op::BitmapLock, &[0x8080]) as u8 == 0 {
        host.free(scratch, Allocation::Heap);
        return Ok(bitmap);
    }
    for row in 0..height {
        for column in 0..width {
            let offset = (row as u32)
                .wrapping_mul(width as u32)
                .wrapping_add(column as u32)
                .wrapping_mul(3);
            let mut rgb = [0; 3];
            host.read(scratch.wrapping_add(offset), &mut rgb)?;
            let pixel = 0xff00_0000 | ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | rgb[2] as u32;
            call(
                host,
                bitmap,
                Op::BitmapPixel,
                &[column as u32, height.wrapping_sub(1).wrapping_sub(row) as u32, pixel],
            );
        }
    }
    call(host, bitmap, Op::BitmapUnlock, &[0x8080]);
    host.free(scratch, Allocation::Heap);
    Ok(bitmap)
}
pub fn software(frame: Frame, rectangle: [i32; 4], mut bitmap: u32, host: &mut impl Host) -> Result<u32> {
    let [x, y, mut width, mut height] = clipped(rectangle, [frame.width, frame.height]);
    call(host, frame.identity, Op::Synchronize, &[]);
    if bitmap != 0 && initialized(bitmap, host) {
        if !format_ok(bitmap, host) {
            return Ok(0);
        }
        limit(bitmap, &mut width, &mut height, host);
    }
    if bitmap == 0 {
        // No checked accessor failure in the native path. A null foreign accessor is
        // a Host fault/precondition, not an invented clean return.
        let service = call(host, 0, Op::ComService, &[]);
        if service == 0 {
            return Err(super::Fault::NullRead);
        }
        host.call(
            service,
            Op::ComBitmapCreate,
            &[0xc470_d325, 0x2073_2180],
            &[],
            std::slice::from_mut(&mut bitmap),
        );
        if bitmap == 0 {
            return Ok(0);
        }
    }
    if (!initialized(bitmap, host) && !init(bitmap, width, height, host))
        || call(host, bitmap, Op::BitmapLock, &[0x8040]) as u8 == 0
    {
        call(host, bitmap, Op::Release, &[]);
        return Ok(0);
    }
    let storage = call(
        host,
        bitmap,
        Op::BitmapStorageRequest,
        &[frame.pitches[0], height as u32],
    );
    let destination = call(host, bitmap, Op::BitmapStorageAddress, &[storage]);
    strided_copy(
        host,
        address(frame.planes[0], x, y, 4, frame.pitches[0]),
        destination,
        width.wrapping_mul(4),
        frame.pitches[0],
        frame.pitches[0],
        height,
    );
    call(host, bitmap, Op::BitmapUnlock, &[0x8040]);
    Ok(bitmap)
}
