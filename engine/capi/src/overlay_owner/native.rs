// SPDX-License-Identifier: GPL-3.0-or-later
//! Acquired native resources and service calls, with all owner logic in core.
use super::Owner;
use fc4_core::terrain_overlays::{
    BindingLookup, CellRect, GeometryToken, Handle, ModelLighting, ProjectionDescriptor, TerrainProvider,
    TextureBinding, TextureFactory, TextureSource, ViewProvider,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

pub(super) type ResourceMap = Arc<Mutex<BTreeMap<usize, (usize, u32)>>>;

#[repr(C)]
#[derive(Default)]
pub struct Resource {
    object: usize,
    context: usize,
    width: u32,
    height: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Services {
    context: usize,
    retain: Option<unsafe extern "C" fn(usize)>,
    release: Option<unsafe extern "C" fn(usize)>,
    nx: u32,
    nz: u32,
    cell_width: f32,
    setting: Option<unsafe extern "C" fn(usize, u32) -> i32>,
    lookup: Option<unsafe extern "C" fn(usize, u32, usize, *mut Resource) -> u32>,
    resource: Option<unsafe extern "C" fn(usize, usize, u32, *mut Resource) -> u32>,
    release_resource: Option<unsafe extern "C" fn(usize, usize, u32)>,
    key_word: Option<unsafe extern "C" fn(usize) -> u32>,
    redisplay: Option<unsafe extern "C" fn(usize, i32, u32, *const i32, i32)>,
    subscription: Option<unsafe extern "C" fn(usize, u32, u32)>,
    projection: Option<unsafe extern "C" fn(usize, i32, *const u8, *mut f32, *const u8, *mut f32) -> u32>,
    terrain_rect: Option<unsafe extern "C" fn(usize, i32, *const u8, *const f32, *mut i32) -> u32>,
    direction: Option<unsafe extern "C" fn(usize, *mut f32)>,
}

// Only the callback context crosses this boundary; core never dereferences it.
// The host owns synchronization for retain/release. Owner serializes all calls.
pub(super) struct Host(Services, ResourceMap);
#[derive(Clone)]
struct Adapter {
    host: Arc<Host>,
}
impl std::ops::Deref for Adapter {
    type Target = Host;
    fn deref(&self) -> &Host {
        &self.host
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        if let Some(release) = self.0.release {
            unsafe { release(self.0.context) };
        }
    }
}
struct NativeResource {
    release: unsafe extern "C" fn(usize, usize, u32),
    value: Resource,
    kind: u32,
    identities: ResourceMap,
}
impl Drop for NativeResource {
    fn drop(&mut self) {
        if let Ok(mut identities) = self.identities.lock() {
            identities.remove(&(self as *const Self as usize));
        }
        unsafe { (self.release)(self.value.context, self.value.object, self.kind) };
    }
}
impl TextureBinding for NativeResource {
    fn size(&self) -> [u32; 2] {
        [self.value.width, self.value.height]
    }
}
impl TextureSource for NativeResource {
    fn size(&self) -> [u32; 2] {
        [self.value.width, self.value.height]
    }
}
impl Host {
    fn acquired(&self, value: Resource, kind: u32) -> Option<Arc<NativeResource>> {
        let resource = Arc::new(NativeResource {
            release: self.0.release_resource?,
            value,
            kind,
            identities: self.1.clone(),
        });
        self.1
            .lock()
            .ok()?
            .insert(Arc::as_ptr(&resource) as usize, (resource.value.object, kind));
        Some(resource)
    }
    fn setting(&self, index: u32) -> i32 {
        self.0.setting.map_or(0, |get| unsafe { get(self.0.context, index) })
    }
    fn import(self: &Arc<Self>, object: usize, kind: u32) -> Option<Arc<NativeResource>> {
        if object == 0 {
            return None;
        }
        let mut value = Resource::default();
        let get = self.0.resource?;
        if unsafe { get(self.0.context, object, kind, &mut value) } == 0 || value.object == 0 {
            return None;
        }
        self.acquired(value, kind)
    }
    fn lookup(self: &Arc<Self>, slot: u32, key: usize) -> Option<Arc<dyn TextureBinding>> {
        let mut value = Resource::default();
        let get = self.0.lookup?;
        if unsafe { get(self.0.context, slot, key, &mut value) } == 0 || value.object == 0 {
            return None;
        }
        self.acquired(value, 0).map(|r| r as Arc<dyn TextureBinding>)
    }
}
impl TerrainProvider for Adapter {
    fn cells(&self) -> [u32; 2] {
        [self.0.nx, self.0.nz]
    }
    fn cell_width(&self) -> f32 {
        self.0.cell_width
    }
    fn redisplay(&self, pass: i32, water: bool, rectangle: CellRect, flags: i32) {
        if let Some(call) = self.0.redisplay {
            let r = [rectangle.min[0], rectangle.min[1], rectangle.max[0], rectangle.max[1]];
            unsafe { call(self.0.context, pass, u32::from(water), r.as_ptr(), flags) };
        }
    }
}
impl TextureFactory for Adapter {
    fn binding(&self, key: u32, lookup: BindingLookup) -> Option<Arc<dyn TextureBinding>> {
        self.host
            .lookup(if lookup == BindingLookup::Slot5 { 5 } else { 6 }, key as usize)
    }
    fn shadow_binding_slot6(&self, key: [u32; 3]) -> Option<Arc<dyn TextureBinding>> {
        self.host.lookup(6, key.as_ptr() as usize)
    }
    fn key_word_slot11(&self) -> u32 {
        self.0.key_word.map_or(0, |get| unsafe { get(self.0.context) })
    }
}
impl ViewProvider for Adapter {
    fn terrain(&self) -> Option<Arc<dyn TerrainProvider>> {
        Some(Arc::new(self.clone()))
    }
    fn texture_factory(&self) -> Option<Arc<dyn TextureFactory>> {
        Some(Arc::new(self.clone()))
    }
    fn shadow_quality(&self) -> i32 {
        self.setting(0)
    }
    fn shadow_debug(&self) -> i32 {
        self.setting(1)
    }
    fn selection_byte(&self) -> u8 {
        self.setting(2) as u8
    }
    fn model_lighting_slot9(&self) -> Option<Arc<dyn ModelLighting>> {
        self.0
            .projection
            .map(|_| Arc::new(self.clone()) as Arc<dyn ModelLighting>)
    }
    fn subscribe(&self, message: u32) {
        if let Some(call) = self.0.subscription {
            unsafe { call(self.0.context, message, 1) };
        }
    }
    fn unsubscribe(&self, message: u32) {
        if let Some(call) = self.0.subscription {
            unsafe { call(self.0.context, message, 0) };
        }
    }
}
impl ModelLighting for Adapter {
    fn projection_slot17(
        &self,
        n: i32,
        source: GeometryToken<'_>,
        pairs: &mut [[f32; 2]],
        descriptor: ProjectionDescriptor<'_>,
        primary: &mut [f32; 16],
    ) -> bool {
        self.0.projection.is_some_and(|call| unsafe {
            call(
                self.0.context,
                n,
                source.0.as_ptr(),
                pairs.as_mut_ptr().cast(),
                descriptor.0.as_ptr(),
                primary.as_mut_ptr(),
            ) != 0
        })
    }
    fn terrain_rect_slot18(&self, n: i32, source: GeometryToken<'_>, world_box: [f32; 6]) -> Option<CellRect> {
        let call = self.0.terrain_rect?;
        let mut r = [0; 4];
        (unsafe { call(self.0.context, n, source.0.as_ptr(), world_box.as_ptr(), r.as_mut_ptr()) } != 0).then_some(
            CellRect {
                min: [r[0], r[1]],
                max: [r[2], r[3]],
            },
        )
    }
    fn direction_slot13(&self) -> [f32; 3] {
        let mut direction = [0.0; 3];
        if let Some(call) = self.0.direction {
            unsafe { call(self.0.context, direction.as_mut_ptr()) };
        }
        direction
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_native_init(
    owner: *mut Owner,
    services: *const Services,
    dynamic: u32,
    water: u32,
) -> u32 {
    let (Some(owner), Some(services)) = (owner.as_ref(), services.as_ref()) else {
        return 0;
    };
    if services.retain.is_none() || services.release.is_none() || services.release_resource.is_none() {
        return 0;
    }
    let Ok(mut core) = owner.0.lock() else { return 0 };
    let Ok(mut saved_host) = owner.1.lock() else { return 0 };
    let host = if let Some(old) = saved_host.as_ref().filter(|old| {
        old.0.context == services.context
            && old.0.nx == services.nx
            && old.0.nz == services.nz
            && old.0.cell_width.to_bits() == services.cell_width.to_bits()
    }) {
        old.clone()
    } else {
        services.retain.unwrap()(services.context);
        Arc::new(Host(*services, owner.2.clone()))
    };
    let result = core.init(Arc::new(Adapter { host: host.clone() }), dynamic != 0, water != 0);
    *saved_host = Some(host);
    u32::from(result.is_ok())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_native_add(
    owner: *mut Owner,
    route: u32,
    input: usize,
    xz: *const f32,
    width: f32,
    turns: f32,
    flip: u32,
) -> u32 {
    let Some(owner) = owner.as_ref() else { return u32::MAX };
    let Ok(mut core) = owner.0.lock() else { return u32::MAX };
    let Ok(saved) = owner.1.lock() else { return u32::MAX };
    let Some(host) = saved.as_ref() else { return u32::MAX };
    if !core.is_initialized() {
        return u32::MAX;
    }
    if (route == 6 || route == 7) && core.shadow_settings()[0] == 0 {
        return u32::MAX;
    }
    let position = if route == 5 {
        [0.0; 2]
    } else {
        if xz.is_null() {
            return u32::MAX;
        }
        [*xz, *xz.add(1)]
    };
    let result = match route {
        0 => {
            let Some(source) = host.import(input, 1) else {
                return u32::MAX;
            };
            core.add_decal_source(source as Arc<dyn TextureSource>, position, width, turns)
        }
        1 => core.add_decal_binding(
            host.import(input, 0).map(|r| r as Arc<dyn TextureBinding>),
            position,
            width,
            turns,
        ),
        2 => core.add_decal_key(input as u32, position, width, turns, BindingLookup::Slot6),
        3 => core.add_decal_key(input as u32, position, width, turns, BindingLookup::Slot5),
        4 => core.add_round_decal(input as u32, position, width, turns),
        5 => core.add_city_decal(input as u32),
        6 => {
            if input == 0 {
                return u32::MAX;
            }
            let key = std::ptr::read_unaligned(input as *const [u32; 3]);
            core.add_shadow_key(key, position, width, turns, flip)
        }
        7 => core.add_shadow_group(input as u32, position, width, turns, flip),
        _ => return u32::MAX,
    };
    result.map_or(u32::MAX, |h| h.0 as u32)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_native_model(
    owner: *mut Owner,
    binding: usize,
    n: i32,
    source: *const u8,
    pairs: *mut f32,
    world_box: *const f32,
    descriptor: *const u8,
) -> u32 {
    let Some(owner) = owner.as_ref() else { return u32::MAX };
    let Ok(mut core) = owner.0.lock() else { return u32::MAX };
    // §3.7: no pointer access or external service calls at quality zero.
    if core.shadow_settings()[0] == 0 {
        return u32::MAX;
    }
    let Ok(saved) = owner.1.lock() else { return u32::MAX };
    let Some(host) = saved.as_ref() else { return u32::MAX };
    if source.is_null() || pairs.is_null() || world_box.is_null() || descriptor.is_null() {
        return u32::MAX;
    }
    // These two opaque inputs are forwarded without decoding or inventing a
    // native geometry/descriptor size. Zero-length slices preserve their address.
    let geometry = GeometryToken(std::slice::from_raw_parts(source, 0));
    let projection = ProjectionDescriptor(std::slice::from_raw_parts(descriptor, 0));
    let count = n.max(1) as usize;
    if count > isize::MAX as usize / std::mem::size_of::<[f32; 2]>() {
        return u32::MAX;
    }
    let pairs = std::slice::from_raw_parts_mut(pairs.cast::<[f32; 2]>(), count);
    let bounds = std::ptr::read_unaligned(world_box.cast::<[f32; 6]>());
    core.add_model_shadow_resolved(
        || host.import(binding, 0).map(|r| r as Arc<dyn TextureBinding>),
        n,
        geometry,
        pairs,
        bounds,
        projection,
    )
    .map_or(u32::MAX, |h| h.0 as u32)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_native_scale(owner: *mut Owner, id: u32, size: f32, offset: f32) {
    if let Some(owner) = owner.as_ref() {
        if let Ok(mut core) = owner.0.lock() {
            let _ = core.set_texture_scale(Handle(id as i32), size, offset);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_native_message(owner: *mut Owner, message: u32) -> u32 {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(mut core) = owner.0.lock() else { return 0 };
    // §5: the native handler returns true for unrelated messages as well.
    // Use the retained view's raw settings, without touching record/index state.
    let _ = core.on_message(message);
    1
}

#[no_mangle]
pub unsafe extern "C" fn fc4_overlay_native_stats(owner: *const Owner, output: *mut u8, size: usize) -> usize {
    let Some(owner) = owner.as_ref() else { return 0 };
    let Ok(core) = owner.0.lock() else { return 0 };
    let text = core.stats_string();
    if !output.is_null() && size > text.len() {
        std::ptr::copy_nonoverlapping(text.as_ptr(), output, text.len());
        *output.add(text.len()) = 0;
    }
    text.len()
}
