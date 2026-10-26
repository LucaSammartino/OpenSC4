// SPDX-License-Identifier: GPL-3.0-or-later
//! Private FC4 factory transport, not the native overlay interface/Init ABI.
use fc4_core::terrain_overlays::{BindingLookup, Handle, OverlayManager, TerrainProvider, TextureBinding};
use fc4_core::terrain_overlays::{Placement, TextureFactory, ViewProvider};
use std::sync::{Arc, Mutex};

struct Inputs {
    cells: [u32; 2],
    width: f32,
}
impl TerrainProvider for Inputs {
    fn cells(&self) -> [u32; 2] {
        self.cells
    }
    fn cell_width(&self) -> f32 {
        self.width
    }
}
impl TextureFactory for Inputs {
    // SPEC-GAP: native resource/Init routing is not asserted by the FC4 factory;
    // explicit dimension-bearing bindings are the only supported input here.
    fn binding(&self, _: u32, _: BindingLookup) -> Option<Arc<dyn TextureBinding>> {
        None
    }
    fn shadow_binding_slot6(&self, _: [u32; 3]) -> Option<Arc<dyn TextureBinding>> {
        None
    }
    fn key_word_slot11(&self) -> u32 {
        0
    }
}
impl ViewProvider for Inputs {
    fn terrain(&self) -> Option<Arc<dyn TerrainProvider>> {
        Some(Arc::new(Self {
            cells: self.cells,
            width: self.width,
        }))
    }
    fn texture_factory(&self) -> Option<Arc<dyn TextureFactory>> {
        Some(Arc::new(Self {
            cells: self.cells,
            width: self.width,
        }))
    }
    fn shadow_quality(&self) -> i32 {
        2
    }
    fn shadow_debug(&self) -> i32 {
        0
    }
    fn subscribe(&self, _: u32) {}
    fn unsubscribe(&self, _: u32) {}
}
struct Dimensions([u32; 2]);
impl TextureBinding for Dimensions {
    fn size(&self) -> [u32; 2] {
        self.0
    }
}

pub struct Owner(Mutex<OverlayManager>);

// The pointers below are private handles returned by create, valid until destroy.
// A mutex also covers const getters: the host reads on a second thread.
#[no_mangle]
pub extern "C" fn fc4_overlay_owner_create() -> *mut Owner {
    Box::into_raw(Box::new(Owner(Mutex::new(OverlayManager::new()))))
}
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_destroy(owner: *mut Owner) {
    if !owner.is_null() {
        drop(Box::from_raw(owner));
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_init(
    owner: *mut Owner,
    nx: u32,
    nz: u32,
    width: f32,
    dynamic: u32,
    water: u32,
) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(mut core) = owner.0.lock() else { return 0 };
    u32::from(
        core.init(Arc::new(Inputs { cells: [nx, nz], width }), dynamic != 0, water != 0)
            .is_ok(),
    )
}
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_shutdown(owner: *mut Owner) {
    if let Some(owner) = owner.as_ref() {
        if let Ok(mut core) = owner.0.lock() {
            core.shutdown();
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_add(
    owner: *mut Owner,
    tw: u32,
    th: u32,
    x: f32,
    z: f32,
    width: f32,
    turns: f32,
) -> u32 {
    let Some(owner) = owner.as_ref() else {
        return u32::MAX;
    };
    let Ok(mut core) = owner.0.lock() else {
        return u32::MAX;
    };
    core.add_decal_binding(Some(Arc::new(Dimensions([tw, th]))), [x, z], width, turns)
        .map_or(u32::MAX, |h| h.0 as u32)
}
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_remove(owner: *mut Owner, id: u32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(mut core) = owner.0.lock() else { return 0 };
    u32::from(core.remove(Handle(id as i32)).is_ok())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_set_position(owner: *mut Owner, id: u32, x: f32, z: f32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(mut core) = owner.0.lock() else { return 0 };
    u32::from(core.set_position(Handle(id as i32), [x, z]).is_ok())
}

// Private portable boundary: input/output address nine aligned binary32 words.
// These are not declarations of the native slots' calling conventions.
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_set_placement(owner: *mut Owner, id: u32, input: *const f32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    if input.is_null() {
        return 0;
    }
    let Ok(mut core) = owner.0.lock() else { return 0 };
    if id == u32::MAX || (id as i32) >= 0 {
        return 1;
    }
    let mut words = [0.0; 9];
    std::ptr::copy_nonoverlapping(input, words.as_mut_ptr(), 9);
    u32::from(core.set_placement(Handle(id as i32), Placement(words)).is_ok())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_placement(owner: *const Owner, id: u32, output: *mut f32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    if output.is_null() {
        return 0;
    }
    let Ok(core) = owner.0.lock() else { return 0 };
    // Slots 12-14 ignore direct IDs and the sentinel, leaving output untouched.
    if id == u32::MAX || (id as i32) >= 0 {
        return 1;
    }
    let mut placement = Placement([0.0; 9]);
    if core.get_placement(Handle(id as i32), &mut placement).is_err() {
        return 0;
    }
    std::ptr::copy_nonoverlapping(placement.0.as_ptr(), output, 9);
    1
}
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_set_alpha(owner: *mut Owner, id: u32, alpha: f32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(mut core) = owner.0.lock() else { return 0 };
    u32::from(core.set_alpha(Handle(id as i32), alpha).is_ok())
}
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_alpha(owner: *const Owner, id: u32) -> f32 {
    let Some(owner) = owner.as_ref() else { return 0.0 };
    let Ok(core) = owner.0.lock() else { return 0.0 };
    core.alpha(Handle(id as i32)).unwrap_or(0.0)
}
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_initialized(owner: *const Owner) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(core) = owner.0.lock() else { return 0 };
    u32::from(core.is_initialized())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_set_shown(owner: *mut Owner, id: u32, shown: u32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(mut core) = owner.0.lock() else { return 0 };
    u32::from(core.set_shown(Handle(id as i32), shown != 0).is_ok())
}
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_shown(owner: *const Owner, id: u32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(core) = owner.0.lock() else { return 0 };
    u32::from(core.shown(Handle(id as i32)).ok().flatten().unwrap_or(false))
}
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_enable_self_lighting(owner: *mut Owner, id: u32, on: u32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(mut core) = owner.0.lock() else { return 0 };
    u32::from(core.enable_self_lighting(Handle(id as i32), on).is_ok())
}
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_self_lit(owner: *const Owner, id: u32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(core) = owner.0.lock() else { return 0 };
    u32::from(core.self_lit(Handle(id as i32)).unwrap_or(false))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_set_color(owner: *mut Owner, id: u32, r: f32, g: f32, b: f32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(mut core) = owner.0.lock() else { return 0 };
    u32::from(core.set_color(Handle(id as i32), [r, g, b]).is_ok())
}

// Private portable boundary: rgb addresses three writable floats. Native slot
// 24 has a different signature (output first, pointer return), per spec §2.3.
#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_color(owner: *const Owner, id: u32, rgb: *mut f32) -> u32 {
    if rgb.is_null() {
        return 0;
    }
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(core) = owner.0.lock() else { return 0 };
    let mut value = [0.0; 3];
    if core.color_into(Handle(id as i32), &mut value).is_err() {
        return 0;
    }
    std::ptr::copy_nonoverlapping(value.as_ptr(), rgb, 3);
    1
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_set_draw_state(owner: *mut Owner, id: u32, index: u32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(mut core) = owner.0.lock() else { return 0 };
    // §2.3: only the low byte is stored, independently of the overlay flags.
    u32::from(core.set_draw_state(Handle(id as i32), index as u8).is_ok())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_owner_draw_state(owner: *const Owner, id: u32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(core) = owner.0.lock() else { return 0 };
    u32::from(core.draw_state(Handle(id as i32)).ok().flatten().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_transport_preserves_words_and_ignored_outputs() {
        unsafe {
            let owner = fc4_overlay_owner_create();
            assert_eq!(fc4_overlay_owner_init(owner, 64, 64, 16.0, 0, 0), 1);
            let id = fc4_overlay_owner_add(owner, 64, 32, 128.0, 128.0, 32.0, 0.0);
            let mut output = [9.0; 9];
            assert_eq!(fc4_overlay_owner_placement(owner, id, output.as_mut_ptr()), 1);
            assert_eq!(output, [128.0, 128.0, 32.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0]);
            assert_eq!(fc4_overlay_owner_set_position(owner, id, 128.00005, 128.0), 1);
            assert_eq!(fc4_overlay_owner_placement(owner, id, output.as_mut_ptr()), 1);
            assert_eq!(output[0], 128.0);
            let input = [
                160.0,
                176.0,
                64.0,
                0.0,
                2.0,
                3.0,
                1.0,
                -0.0,
                f32::from_bits(0x7fc0_1234),
            ];
            assert_eq!(fc4_overlay_owner_set_placement(owner, id, input.as_ptr()), 1);
            assert_eq!(fc4_overlay_owner_placement(owner, id, output.as_mut_ptr()), 1);
            assert_eq!(output.map(f32::to_bits), input.map(f32::to_bits));
            assert_eq!(fc4_overlay_owner_set_position(owner, id, 192.0, 224.0), 1);
            assert_eq!(fc4_overlay_owner_placement(owner, id, output.as_mut_ptr()), 1);
            assert_eq!(&output[..2], &[192.0, 224.0]);
            assert_eq!(output[8].to_bits(), input[8].to_bits());
            for ignored in [0, 0x7fff_ffff, u32::MAX] {
                assert_eq!(fc4_overlay_owner_set_position(owner, ignored, 1.0, 2.0), 1);
                assert_eq!(fc4_overlay_owner_set_placement(owner, ignored, input.as_ptr()), 1);
                output = [9.0; 9];
                assert_eq!(fc4_overlay_owner_placement(owner, ignored, output.as_mut_ptr()), 1);
                assert_eq!(output, [9.0; 9]);
            }
            assert_eq!(fc4_overlay_owner_placement(owner, id, std::ptr::null_mut()), 0);
            assert_eq!(fc4_overlay_owner_set_placement(owner, id, std::ptr::null()), 0);
            assert_eq!(
                fc4_overlay_owner_placement(std::ptr::null(), id, output.as_mut_ptr()),
                0
            );
            assert_eq!(fc4_overlay_owner_set_position(std::ptr::null_mut(), id, 1.0, 2.0), 0);
            assert_eq!(fc4_overlay_owner_remove(owner, id), 1);
            assert_eq!(fc4_overlay_owner_placement(owner, id, output.as_mut_ptr()), 0);
            assert_eq!(output, [9.0; 9]);
            assert_eq!(fc4_overlay_owner_set_position(owner, id, 1.0, 2.0), 0);
            assert_eq!(fc4_overlay_owner_set_placement(owner, id, input.as_ptr()), 0);
            fc4_overlay_owner_shutdown(owner);
            fc4_overlay_owner_destroy(owner);
        }
    }

    #[test]
    fn color_words_and_draw_byte_survive_repeat_init_but_not_shutdown() {
        unsafe {
            let owner = fc4_overlay_owner_create();
            assert_eq!(fc4_overlay_owner_init(owner, 64, 64, 16.0, 0, 0), 1);
            let id = fc4_overlay_owner_add(owner, 64, 32, 128.0, 128.0, 32.0, 0.0);
            // No clamping or float arithmetic: preserve signed zero and NaN payload.
            let words = [0x8000_0000, 0x7fc0_1234, 0x3fe0_0000];
            assert_eq!(
                fc4_overlay_owner_set_color(
                    owner,
                    id,
                    f32::from_bits(words[0]),
                    f32::from_bits(words[1]),
                    f32::from_bits(words[2]),
                ),
                1
            );
            assert_eq!(fc4_overlay_owner_set_draw_state(owner, id, 0x0012_3403), 1);
            assert_eq!(fc4_overlay_owner_init(owner, 64, 64, 16.0, 1, 1), 1);
            let mut rgb = [9.0; 3];
            assert_eq!(fc4_overlay_owner_color(owner, id, rgb.as_mut_ptr()), 1);
            assert_eq!(rgb.map(f32::to_bits), words);
            assert_eq!(fc4_overlay_owner_draw_state(owner, id), 3);
            assert_eq!(fc4_overlay_owner_shown(owner, id), 1);
            assert_eq!(fc4_overlay_owner_self_lit(owner, id), 0);
            assert_eq!(fc4_overlay_owner_set_color(owner, u32::MAX, 2.0, 3.0, 4.0), 1);
            assert_eq!(fc4_overlay_owner_set_draw_state(owner, u32::MAX, 255), 1);
            assert_eq!(fc4_overlay_owner_color(owner, id, rgb.as_mut_ptr()), 1);
            assert_eq!(rgb.map(f32::to_bits), words);
            assert_eq!(fc4_overlay_owner_color(owner, u32::MAX, rgb.as_mut_ptr()), 1);
            assert_eq!(rgb.map(f32::to_bits), [0; 3]);
            assert_eq!(fc4_overlay_owner_draw_state(owner, u32::MAX), 0);
            assert_eq!(fc4_overlay_owner_color(owner, id, std::ptr::null_mut()), 0);
            rgb = [9.0; 3];
            assert_eq!(fc4_overlay_owner_color(std::ptr::null(), id, rgb.as_mut_ptr()), 0);
            assert_eq!(rgb, [9.0; 3]);
            assert_eq!(fc4_overlay_owner_remove(owner, id), 1);
            assert_eq!(fc4_overlay_owner_color(owner, id, rgb.as_mut_ptr()), 0);
            assert_eq!(rgb, [9.0; 3]);
            assert_eq!(fc4_overlay_owner_set_color(owner, id, 1.0, 2.0, 3.0), 0);
            assert_eq!(fc4_overlay_owner_set_draw_state(owner, id, 1), 0);
            fc4_overlay_owner_shutdown(owner);
            assert_eq!(fc4_overlay_owner_init(owner, 64, 64, 16.0, 0, 0), 1);
            assert_eq!(fc4_overlay_owner_add(owner, 64, 32, 128.0, 128.0, 32.0, 0.0), id);
            assert_eq!(fc4_overlay_owner_color(owner, id, rgb.as_mut_ptr()), 1);
            assert_eq!(rgb, [1.0; 3]);
            assert_eq!(fc4_overlay_owner_draw_state(owner, id), 0);
            fc4_overlay_owner_destroy(owner);
        }
    }

    #[test]
    fn visibility_and_sticky_self_lighting_survive_reinit_but_not_reset() {
        unsafe {
            let owner = fc4_overlay_owner_create();
            assert_eq!(fc4_overlay_owner_init(owner, 64, 64, 16.0, 1, 0), 1);
            let id = fc4_overlay_owner_add(owner, 64, 32, 128.0, 128.0, 32.0, 0.0);
            assert_ne!(id, u32::MAX);
            assert_eq!(fc4_overlay_owner_shown(owner, id), 1);
            assert_eq!(fc4_overlay_owner_set_shown(owner, id, 0), 1);
            assert_eq!(fc4_overlay_owner_shown(owner, id), 0);
            assert_eq!(fc4_overlay_owner_enable_self_lighting(owner, id, 2), 1);
            assert_eq!(fc4_overlay_owner_self_lit(owner, id), 0);
            assert_eq!(fc4_overlay_owner_enable_self_lighting(owner, id, 3), 1);
            assert_eq!(fc4_overlay_owner_enable_self_lighting(owner, id, 0), 1);
            assert_eq!(fc4_overlay_owner_self_lit(owner, id), 1);
            assert_eq!(fc4_overlay_owner_init(owner, 64, 64, 16.0, 0, 1), 1);
            assert_eq!(fc4_overlay_owner_shown(owner, id), 0);
            assert_eq!(fc4_overlay_owner_self_lit(owner, id), 1);
            assert_eq!(fc4_overlay_owner_shown(owner, u32::MAX), 0);
            assert_eq!(fc4_overlay_owner_self_lit(owner, u32::MAX), 0);
            fc4_overlay_owner_shutdown(owner);
            assert_eq!(fc4_overlay_owner_init(owner, 64, 64, 16.0, 0, 0), 1);
            assert_eq!(fc4_overlay_owner_add(owner, 64, 32, 128.0, 128.0, 32.0, 0.0), id);
            assert_eq!(fc4_overlay_owner_shown(owner, id), 1);
            assert_eq!(fc4_overlay_owner_self_lit(owner, id), 0);
            fc4_overlay_owner_destroy(owner);
        }
    }

    #[test]
    fn invalid_init_preserves_the_cores_published_prefix() {
        unsafe {
            let owner = fc4_overlay_owner_create();
            assert_eq!(fc4_overlay_owner_initialized(owner), 0);
            assert_eq!(fc4_overlay_owner_init(owner, 0, 64, 16.0, 0, 0), 0);
            // §1.1: Init publishes initialized before consulting services.
            assert_eq!(fc4_overlay_owner_initialized(owner), 1);
            fc4_overlay_owner_shutdown(owner);
            assert_eq!(fc4_overlay_owner_initialized(owner), 0);
            assert_eq!(fc4_overlay_owner_init(owner, 64, 64, 16.0, 1, 0), 1);
            let id = fc4_overlay_owner_add(owner, 64, 32, 128.0, 128.0, 32.0, 0.0);
            assert_eq!(id, 0x8000_0000);
            assert_eq!(fc4_overlay_owner_set_alpha(owner, id, 0.51), 1);
            assert_eq!(fc4_overlay_owner_init(owner, 64, 64, 16.0, 0, 0), 1);
            assert_eq!(fc4_overlay_owner_alpha(owner, id), 0.51);
            // Static alpha changes within tolerance are ignored by the owner.
            assert_eq!(fc4_overlay_owner_set_alpha(owner, id, 0.52), 1);
            assert_eq!(fc4_overlay_owner_alpha(owner, id), 0.51);
            fc4_overlay_owner_shutdown(owner);
            fc4_overlay_owner_shutdown(owner);
            assert_eq!(fc4_overlay_owner_init(owner, 64, 64, 16.0, 0, 0), 1);
            assert_eq!(fc4_overlay_owner_add(owner, 64, 32, 128.0, 128.0, 32.0, 0.0), id);
            assert_eq!(fc4_overlay_owner_alpha(owner, id), 1.0);
            fc4_overlay_owner_destroy(owner);
        }
    }
}
