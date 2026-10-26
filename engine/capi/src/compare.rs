// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link compare transport. Buffers are owned copies, never game pointers.
use std::ffi::{c_char, CStr};

#[repr(C)]
struct Buffer {
    data: *mut u8,
    size: u32,
    base: u32, // 0 = receiver, 1 + stack argument index
    offset: u32,
}

#[repr(C)]
#[derive(Default)]
struct ResultValue {
    eax: u32,
    edx: u32,
    float_bits: u32,
}

/// A new binding needs a registry row and a native-layout adapter here.
/// # Safety
/// Entry is NUL terminated. Args has 16 words; buffers are initialized, disjoint,
/// writable copies with valid sizes. No raw argument pointer is dereferenced.
#[no_mangle]
unsafe extern "C" fn fc4_compare_adapter(
    entry: *const c_char,
    args: *const u32,
    buffers: *mut Buffer,
    count: usize,
    result: *mut ResultValue,
) -> bool {
    let Ok(entry) = CStr::from_ptr(entry).to_str() else {
        return false;
    };
    let args = &*args.cast::<[u32; 16]>();
    let buffers = if count == 0 {
        &mut []
    } else {
        std::slice::from_raw_parts_mut(buffers, count)
    };
    let result = &mut *result;
    let get = |base: u32| {
        buffers
            .iter()
            .find(|b| b.base == base && b.offset == 0)
            .map(|b| (b.data, b.size as usize))
    };
    let language_op = match entry {
        "language-from-system" => Some(0),
        "system-from-language" => Some(1),
        "language-alias" => Some(2),
        "primary-language" => Some(3),
        "default-country" => Some(4),
        // SPEC-GAP: current-system-country receiver fields are unpublished.
        "system-country" => return false,
        _ => None,
    };
    if let Some(op) = language_op {
        if op > 0 && args[0] == 0 {
            return false;
        }
        result.eax = super::inplace3::fc4_inplace3_language(op, args[0]);
        return true;
    }
    let lighting_op = match entry {
        "lighting-vertices" => Some(0),
        "lighting-global" => Some(1),
        "lighting-shadow" => Some(2),
        "lighting-night" => Some(3),
        _ => None,
    };
    if let Some(op) = lighting_op {
        let Some((owner, size)) = get(0) else {
            return false;
        };
        if size != 0x12c {
            return false;
        }
        let (normals, output, matrix) = if op == 0 {
            let (Some(n), Some(o), Some(m)) = (get(2), get(4), get(6)) else {
                return false;
            };
            let (nlen, clen) = (
                (args[0] as usize).saturating_sub(1) * args[2] as usize + 12,
                (args[0] as usize).saturating_sub(1) * args[4] as usize + 4,
            );
            if args[0] == 0 || args[0] > 4096 || args[2] < 12 || args[4] < 4 || n.1 != nlen || o.1 != clen || m.1 != 36
            {
                return false;
            }
            (n.0, o.0, m.0)
        } else if op < 3 {
            let Some((o, 12)) = get(1) else {
                return false;
            };
            (std::ptr::null_mut(), o, std::ptr::null_mut())
        } else {
            (std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
        };
        result.eax = super::inplace3::fc4_inplace3_lighting(
            owner.cast(),
            op,
            args[0] as usize,
            normals,
            args[2] as usize,
            output,
            args[4] as usize,
            matrix.cast(),
        );
        return true;
    }
    let (op, cached) = match entry {
        "clip2" => (0x7f6620, true),
        "zoom-rotation" => (0x7f66a0, true),
        "view-viewport" => (0x7f66c0, true),
        "fixed" => (0x7f6710, true),
        "view-box" => (0x7f6770, true),
        "city-corner" => (0x7f6ba0, true),
        "clip3" => (0x7f7710, true),
        "visible-edges" => (0x7f7750, true),
        "world-view" => (0x7f77d0, true),
        "world-viewport" => (0x7f7850, true),
        "world-viewport2" => (0x7f78e0, true),
        "view-world" => (0x7f7920, true),
        "viewport-world" => (0x7f79b0, true),
        "box-rectangle" => (0x7f7a40, true),
        "view-point" => (0x7f7e50, true),
        "viewport-fixed" => (0x7f8390, true),
        "rotation" => (0x7f6cc0, false),
        "ground" => (0x7f6ab0, false),
        "cross" => (0x5d6f10, false),
        "primitive-indices" => (0x80aef0, false),
        _ => return false,
    };
    let mut input = [0u32; 32];
    let mut output = [0u32; 32];
    let mut owner = [0u32; 0x470 / 4];
    let copy_in = |base, dest: &mut [u32], at: usize, bytes| -> bool {
        let Some((data, size)) = get(base) else {
            return false;
        };
        if size != bytes || at * 4 + bytes > dest.len() * 4 {
            return false;
        }
        std::ptr::copy_nonoverlapping(data, dest.as_mut_ptr().add(at).cast(), bytes);
        true
    };
    if cached && !copy_in(0, &mut owner, 0, 0x470) {
        return false;
    }
    match entry {
        "zoom-rotation" | "visible-edges" => {
            if !copy_in(1, &mut output, 0, 4) || !copy_in(2, &mut output, 1, 4) {
                return false;
            }
        }
        "view-box" | "box-rectangle" | "ground" | "cross" => {
            if !copy_in(1, &mut input, 0, 12) || !copy_in(2, &mut input, 3, 12) {
                return false;
            }
            if entry == "box-rectangle" {
                if !copy_in(3, &mut output, 0, 8) || !copy_in(4, &mut output, 2, 8) {
                    return false;
                }
            } else if entry != "view-box" && !copy_in(3, &mut output, 0, 12) {
                return false;
            }
            if entry == "cross" {
                input[6] = 2;
            }
        }
        "city-corner" => {
            input[0] = args[0];
            if !copy_in(2, &mut output, 0, 12) {
                return false;
            }
        }
        "primitive-indices" => {
            input[..2].copy_from_slice(&args[..2]);
            if !copy_in(3, &mut output, 0, 12) {
                return false;
            }
        }
        _ => {
            let size = if entry == "clip2" { 8 } else { 12 };
            if !copy_in(1, &mut input, 0, size) {
                return false;
            }
            output[..size / 4].copy_from_slice(&input[..size / 4]);
            if entry == "rotation" {
                input[3] = args[1];
            }
        }
    }
    if cached {
        super::viewutil_shadow::fc4_viewutil_shadow_cached(op, owner.as_ptr(), input.as_ptr(), output.as_mut_ptr());
    } else {
        super::viewutil_shadow::fc4_viewutil_shadow_local(op, input.as_ptr(), output.as_mut_ptr());
    }
    if entry == "view-box" || entry == "view-point" {
        result.eax = output[0];
        return true;
    }
    for buffer in buffers {
        if buffer.base == 0 {
            continue;
        }
        let word = match entry {
            "zoom-rotation" | "visible-edges" => (buffer.base - 1) as usize,
            "box-rectangle" if buffer.base >= 3 => ((buffer.base - 3) * 2) as usize,
            "box-rectangle" | "ground" | "cross" if buffer.base < 3 => continue,
            _ => 0,
        };
        std::ptr::copy_nonoverlapping(
            output.as_ptr().add(word).cast::<u8>(),
            buffer.data,
            buffer.size as usize,
        );
    }
    true
}

/// Retained hand fixtures, alongside natural and recorded input sources.
/// # Safety
/// Same owned-buffer contract as the adapter. The first call has no buffers and
/// fills scalar arguments needed to evaluate the registry's counted windows.
#[no_mangle]
unsafe extern "C" fn fc4_compare_fixture(
    entry: *const c_char,
    sample: u32,
    args: *mut u32,
    buffers: *mut Buffer,
    count: usize,
) -> bool {
    let Ok(entry) = CStr::from_ptr(entry).to_str() else {
        return false;
    };
    let args = &mut *args.cast::<[u32; 16]>();
    if count == 0 {
        match entry {
            "language-from-system" => args[0] = (sample << 10) | 9,
            "system-from-language" | "language-alias" | "primary-language" | "default-country" => args[0] = sample + 1,
            "city-corner" => args[0] = sample % 5,
            "rotation" => args[1] = sample % 4,
            "primitive-indices" => {
                args[0] = sample % 6;
                args[1] = sample;
            }
            "lighting-vertices" => {
                args[0] = 4;
                args[2] = 12;
                args[4] = 8;
            }
            "clip2" | "zoom-rotation" | "view-viewport" | "fixed" | "view-box" | "clip3" | "visible-edges"
            | "world-view" | "world-viewport" | "world-viewport2" | "view-world" | "viewport-world"
            | "box-rectangle" | "view-point" | "viewport-fixed" | "ground" | "cross" | "lighting-global"
            | "lighting-shadow" | "lighting-night" | "system-country" => {}
            _ => return false,
        }
        return true;
    }
    let buffers = std::slice::from_raw_parts_mut(buffers, count);
    for buffer in buffers {
        let bytes = std::slice::from_raw_parts_mut(buffer.data, buffer.size as usize);
        bytes.fill(0);
        let word = |bytes: &mut [u8], at: usize, value: u32| bytes[at..at + 4].copy_from_slice(&value.to_ne_bytes());
        if buffer.base == 0 {
            if bytes.len() == 0x470 {
                word(bytes, 0, 1);
                word(bytes, 0x0c, 4);
                word(bytes, 0x10, sample % 4);
                for at in [0x20, 0x24] {
                    word(bytes, at, 4096f32.to_bits());
                }
                for at in [0x108, 0x110] {
                    word(bytes, at, (-256f32).to_bits());
                }
                for at in [0x10c, 0x114] {
                    word(bytes, at, 256f32.to_bits());
                }
                word(bytes, 0x118, 1f32.to_bits());
                word(bytes, 0x11c, 8192f32.to_bits());
                for at in [0x44c, 0x454, 0x458, 0x45c, 0x460, 0x464, 0x468, 0x46c] {
                    word(bytes, at, 1f32.to_bits());
                }
            } else if bytes.len() == 0x12c {
                bytes[0x24] = (sample & 1) as u8;
                for j in 0..3 {
                    word(bytes, 0x64 + j * 4, (j as f32 - 1.).to_bits());
                    word(bytes, 0x8c + j * 4, ((j + 1) as f32 / 4.).to_bits());
                }
                for j in 0..18 {
                    word(bytes, 0xe4 + j * 4, (((j as u32 + sample) % 5) as f32 / 8.).to_bits());
                }
            }
            continue;
        }
        if entry.starts_with("lighting-") {
            if buffer.base == 2 {
                for j in 0..12 {
                    word(
                        bytes,
                        j * 4,
                        ((((j as u32 + sample) % 3) as i32 - 1) as f32 / 2.).to_bits(),
                    );
                }
            } else if buffer.base == 6 {
                for j in 0..9 {
                    word(bytes, j * 4, (if j % 4 == 0 { 1f32 } else { 0f32 }).to_bits());
                }
            }
            continue;
        }
        for j in 0..bytes.len() / 4 {
            let value = if matches!(entry, "zoom-rotation" | "visible-edges") {
                0.
            } else if entry == "city-corner" || entry == "primitive-indices" || buffer.base >= 3 {
                10.
            } else if buffer.base == 2 {
                sample as f32 + j as f32 + 1.
            } else {
                sample as f32 - 8. + j as f32
            };
            word(bytes, j * 4, value.to_bits());
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cross_replay_preserves_input_windows() {
        let mut a = [1f32, 2., 3.];
        let mut b = [4f32, 5., 6.];
        let mut output = [0f32; 3];
        let mut buffers = [
            Buffer {
                data: a.as_mut_ptr().cast(),
                size: 12,
                base: 1,
                offset: 0,
            },
            Buffer {
                data: b.as_mut_ptr().cast(),
                size: 12,
                base: 2,
                offset: 0,
            },
            Buffer {
                data: output.as_mut_ptr().cast(),
                size: 12,
                base: 3,
                offset: 0,
            },
        ];
        let mut result = ResultValue::default();
        unsafe {
            assert!(fc4_compare_adapter(
                c"cross".as_ptr(),
                [0u32; 16].as_ptr(),
                buffers.as_mut_ptr(),
                3,
                &mut result
            ));
        }
        assert_eq!(a, [1., 2., 3.]);
        assert_eq!(b, [4., 5., 6.]);
        assert_eq!(output, [-3., 6., -3.]);
    }
    #[test]
    fn unpublished_current_language_fields_refuse_replay() {
        let mut dummy = [0u8; 4];
        let mut buffer = Buffer {
            data: dummy.as_mut_ptr(),
            size: 4,
            base: 0,
            offset: 0,
        };
        let mut result = ResultValue::default();
        unsafe {
            assert!(!fc4_compare_adapter(
                c"system-from-language".as_ptr(),
                [0u32; 16].as_ptr(),
                &mut buffer,
                1,
                &mut result
            ));
        }
    }

    #[test]
    fn scalar_arguments_need_no_memory_windows() {
        let mut args = [0u32; 16];
        args[0] = 9;
        let mut result = ResultValue::default();
        unsafe {
            assert!(fc4_compare_adapter(
                c"language-from-system".as_ptr(),
                args.as_ptr(),
                std::ptr::null_mut(),
                0,
                &mut result
            ));
        }
        assert_eq!(result.eax, 1); // Published English system-language mapping.
    }
}
