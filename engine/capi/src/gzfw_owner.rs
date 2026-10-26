// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link command registry and buffer transport. No DLL exports.
use fc4_core::commands::{parse_delimited_text, quote_argument};
use fc4_core::gz_framework::{
    buffer::{Allocator, Buffer},
    registry::Registry,
};
use std::ffi::{c_char, CStr};
use std::sync::Mutex;

type TextOut = unsafe extern "C" fn(*mut std::ffi::c_void, u32, *const u8, usize);

unsafe fn text<'a>(p: *const c_char) -> Option<&'a [u8]> {
    (!p.is_null()).then(|| CStr::from_ptr(p).to_bytes())
}
#[no_mangle]
pub extern "C" fn fc4_gzfw_registry_new() -> *mut Mutex<Registry> {
    Box::into_raw(Box::new(Mutex::new(Registry::default())))
}
#[no_mangle]
pub unsafe extern "C" fn fc4_gzfw_registry_free(p: *mut Mutex<Registry>) {
    if !p.is_null() {
        drop(Box::from_raw(p));
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc4_gzfw_registry_change(
    p: *mut Mutex<Registry>,
    op: u32,
    id: u32,
    name: *const c_char,
    info: *const c_char,
) -> bool {
    let Some(p) = p.as_ref() else { return false };
    let Ok(mut r) = p.lock() else { return false };
    match op {
        0 => r.register(id, text(name), text(info)),
        1 => r.parameter_info(id, text(info)),
        2 => r.unregister(id),
        3 => {
            r.clear();
            true
        }
        _ => false,
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc4_gzfw_registry_lookup(
    p: *const Mutex<Registry>,
    bytes: *const u8,
    length: usize,
    parse: bool,
) -> u32 {
    let Some(p) = p.as_ref() else { return 0 };
    if bytes.is_null() {
        return 0;
    }
    let Ok(r) = p.lock() else { return 0 };
    let bytes = std::slice::from_raw_parts(bytes, length);
    if parse {
        r.parse_id(bytes)
    } else {
        r.lookup(bytes)
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc4_gzfw_registry_info(
    p: *const Mutex<Registry>,
    id: u32,
    context: *mut std::ffi::c_void,
    out: TextOut,
) -> bool {
    let Some(p) = p.as_ref() else { return false };
    let info = {
        let Ok(r) = p.lock() else { return false };
        r.info(id).cloned()
    };
    let Some(info) = info else { return false };
    // Host callbacks can reenter the server: never hold its lock across them.
    out(context, 0, info.name.as_ptr(), info.name.len());
    out(context, 1, info.parameters.as_ptr(), info.parameters.len());
    true
}
#[no_mangle]
pub unsafe extern "C" fn fc4_gzfw_registry_ids(p: *const Mutex<Registry>, ids: *mut u32, capacity: usize) -> usize {
    let Some(p) = p.as_ref() else { return 0 };
    let Ok(r) = p.lock() else { return 0 };
    let values = r.ids();
    if !ids.is_null() && capacity >= values.len() {
        std::ptr::copy_nonoverlapping(values.as_ptr(), ids, values.len());
    }
    values.len()
}
#[no_mangle]
pub unsafe extern "C" fn fc4_gzfw_words(bytes: *const u8, length: usize, context: *mut std::ffi::c_void, out: TextOut) {
    if bytes.is_null() {
        return;
    }
    let bytes = std::slice::from_raw_parts(bytes, length);
    let (_, words) = parse_delimited_text(bytes, b' ', 256);
    for (i, word) in words.iter().enumerate() {
        out(context, i as u32, word.as_ptr(), word.len());
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc4_gzfw_quote(bytes: *const u8, length: usize, context: *mut std::ffi::c_void, out: TextOut) {
    if bytes.is_null() {
        return;
    }
    let quoted = quote_argument(std::slice::from_raw_parts(bytes, length));
    out(context, 0, quoted.as_ptr(), quoted.len());
}

struct Storage;
impl Allocator for Storage {
    fn allocate(&mut self, bytes: usize) -> Option<Vec<u8>> {
        let mut v = Vec::new();
        v.try_reserve_exact(bytes).ok()?;
        // SPEC-GAP: safe Rust initializes unspecified buffer bytes to 0xA5.
        v.resize(bytes, 0xa5);
        Some(v)
    }
}
#[no_mangle]
pub extern "C" fn fc4_gzfw_buffer_new() -> *mut Mutex<Buffer> {
    Box::into_raw(Box::new(Mutex::new(Buffer::default())))
}
#[no_mangle]
pub unsafe extern "C" fn fc4_gzfw_buffer_free(p: *mut Mutex<Buffer>) {
    if !p.is_null() {
        drop(Box::from_raw(p));
    }
}
// Metadata access is serialized. Borrowed pixels expire on storage disposal.
#[no_mangle]
pub unsafe extern "C" fn fc4_gzfw_buffer_call(
    p: *mut Mutex<Buffer>,
    op: u32,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    out: *mut u32,
) -> usize {
    let Some(p) = p.as_ref() else { return 0 };
    let Ok(mut v) = p.lock() else { return 0 };
    match op {
        0 => v.add_ref() as usize,
        1 => v.release() as usize,
        2 => v.init(&mut Storage, a, b, c, d).unwrap_or(false) as usize,
        3 => v.lock(a) as usize,
        4 => v.unlock(a) as usize,
        5 => v.width() as usize,
        6 => v.height() as usize,
        7 if !out.is_null() => {
            std::ptr::copy_nonoverlapping(v.rect().as_ptr().cast::<u32>(), out, 4);
            out as usize
        }
        8 if !out.is_null() => {
            let (kind, bpp) = v.colour_type();
            *out = kind;
            *out.add(1) = bpp;
            out as usize
        }
        9 => v.bytes_per_pixel() as usize,
        10 if !out.is_null() => v.colour_key(&mut *out).unwrap_or(false) as usize,
        11 => v.map_rgb(a, b, c).unwrap_or(0) as usize,
        12 => v.pixels_mut().map_or(0, |p| p.as_mut_ptr() as usize),
        13 => v.stride() as usize,
        14 => v.modifications as usize,
        15 => v.nesting as usize,
        16 => {
            v.shutdown();
            1
        }
        17 => v.initialized() as usize,
        18 => v.size() as usize,
        19 => {
            if b != 0 {
                v.attach_colour_key(a, true);
            } else {
                v.clear_key();
            }
            1
        }
        20 => v.key_enabled() as usize,
        21 => fc4_core::gz_framework::buffer::map_rgba(v.colour_type().0, a, b, c, d) as usize,
        22 => {
            let Some(rgba) = fc4_core::gz_framework::buffer::unmap_rgba(v.colour_type().0, a) else {
                return 0;
            };
            for (i, value) in rgba.into_iter().enumerate() {
                *out.add(i) = value.into();
            }
            1
        }
        23 => {
            v.set_format(a, b);
            1
        }
        24 => {
            v.allocate_pixels(&mut Storage);
            1
        }
        25 => {
            v.modified();
            1
        }
        26 => {
            v.free_pixels();
            1
        }
        27 => v.set_drawing_color([a, b, c, d]) as usize,
        28 => {
            let Some(rgba) = v.drawing_color() else {
                return 0;
            };
            for (i, value) in rgba.into_iter().enumerate() {
                *out.add(i) = value.into();
            }
            1
        }
        _ => 0,
    }
}
#[no_mangle]
pub extern "C" fn fc4_gzfw_buffer_convert(source: u32, destination: u32, pixel: u32) -> u32 {
    fc4_core::gz_framework::buffer::convert_pixel(source, destination, pixel)
}
