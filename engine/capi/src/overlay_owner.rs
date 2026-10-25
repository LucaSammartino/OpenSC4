// SPDX-License-Identifier: GPL-3.0-or-later
//! Private FC4 factory transport, not the native overlay interface/Init ABI.
use fc4_core::terrain_overlays::{BindingLookup, Handle, OverlayManager, TerrainProvider, TextureBinding};
use fc4_core::terrain_overlays::{TextureFactory, ViewProvider};
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

#[cfg(test)]
mod tests {
    use super::*;

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
