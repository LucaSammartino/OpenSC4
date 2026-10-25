// SPDX-License-Identifier: GPL-3.0-or-later
//! S3 round-3 inputs use their own native layout, not the view-utilities packet.
use fc4_core::{language::LanguageManager, lighting::LightingManager};

#[repr(C)]
struct NativeLighting {
    prefix: [u8; 0x24],
    night: u8,
    gap0: [u8; 0x3f],
    shadow: [f32; 3],
    gap1: [u8; 0x1c],
    global: [f32; 3],
    gap2: [u8; 0x4c],
    axes: [[f32; 3]; 6],
}

#[no_mangle]
extern "C" fn fc4_inplace3_language(op: u32, input: u32) -> u32 {
    let manager = LanguageManager::new();
    match op {
        0 => LanguageManager::language_from_system_language(input),
        // Zero requires unpublished current-language/country field offsets.
        1 => manager.system_language_from_language(input),
        2 => manager.language_id_alias(input),
        3 => manager.primary_language_for_country(input),
        4 => manager.default_country_for_language(input),
        5 => manager.current_system_country(),
        _ => unreachable!(),
    }
}

#[no_mangle]
unsafe extern "C" fn fc4_inplace3_lighting(
    owner: *const NativeLighting,
    op: u32,
    count: usize,
    normals: *const u8,
    normal_stride: usize,
    output: *mut u8,
    color_stride: usize,
    matrix: *const [[f32; 3]; 3],
) -> u32 {
    let owner = &*owner;
    let manager = LightingManager::inplace_cached_inputs(owner.night != 0, owner.global, owner.shadow, owner.axes);
    match op {
        0 => {
            let nlen = if count == 0 {
                0
            } else {
                (count - 1) * normal_stride + 12
            };
            let clen = if count == 0 { 0 } else { (count - 1) * color_stride + 4 };
            let normals = std::slice::from_raw_parts(normals, nlen);
            let colors = std::slice::from_raw_parts_mut(output, clen);
            u32::from(
                manager
                    .get_vertex_colors(count, normals, normal_stride, colors, color_stride, *matrix)
                    .is_ok(),
            )
        }
        1 | 2 => {
            let value = if op == 1 {
                manager.get_global_light()
            } else {
                manager.get_shadow_direction()
            };
            std::ptr::copy_nonoverlapping(value.as_ptr().cast::<u8>(), output, 12);
            1
        }
        3 => u32::from(manager.is_night()),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_lighting_layout() {
        assert_eq!(std::mem::offset_of!(NativeLighting, shadow), 0x64);
        assert_eq!(std::mem::offset_of!(NativeLighting, global), 0x8c);
        assert_eq!(std::mem::offset_of!(NativeLighting, axes), 0xe4);
        assert_eq!(std::mem::size_of::<NativeLighting>(), 0x12c);
    }
}
