// SPDX-License-Identifier: GPL-3.0-or-later
//! cS3DTextureBindingFactory, texture-bindings.md §1.

use std::collections::{BTreeMap, VecDeque};

use super::{
    Binding, BindingParams, BitCounts, ImageInfo, ImageResource, Provider, Release, ResourceKey, Service, Upload,
    DEFAULT_GROUP, IMAGE_TYPE, RESTORE_TEXTURES,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindingId(u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BindingInfo {
    pub group: u32,
    pub instance: u32,
    pub levels: u32,
    pub width: u32,
    pub height: u32,
    pub bits: BitCounts,
    pub bits_per_texel: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub in_use: u32,
    pub pooled: u32,
    // TB5: wide aggregate counters avoid inventing native overflow semantics.
    pub in_use_bytes: u64,
    pub pooled_bytes: u64,
}

impl Stats {
    pub fn format(self) -> String {
        let mb = |n: u64| n as f64 * 2_f64.powi(-20);
        format!(
            "Textures: {} ({:.1} MB) in use, {} ({:.1} MB) awaiting reuse, {} ({:.1} MB) total\n",
            self.in_use,
            mb(self.in_use_bytes),
            self.pooled,
            mb(self.pooled_bytes),
            u64::from(self.in_use) + u64::from(self.pooled),
            mb(self.in_use_bytes + self.pooled_bytes)
        )
    }
}

struct Record {
    binding: Binding,
    params: BindingParams,
    bits: BitCounts,
    in_use: bool,
    group: u32,
    instance: u32,
}

pub struct BindingFactory<P: Provider> {
    provider: P,
    records: BTreeMap<BindingId, Record>,
    keys: BTreeMap<(u32, u32), BindingId>,
    free: VecDeque<BindingId>,
    stats: Stats,
    free_pool_max: u32,
    display_depth: u32,
    lod: u32,
    group: u32,
    manager: Option<u64>,
    graphic_system: Option<u64>,
    driver: Option<u64>,
    message_server: Option<u64>,
    next_id: u64,
    alive: bool,
}

impl<P: Provider> BindingFactory<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            records: BTreeMap::new(),
            keys: BTreeMap::new(),
            free: VecDeque::new(),
            stats: Stats::default(),
            free_pool_max: 2_000_000,
            display_depth: 32,
            lod: 0,
            group: DEFAULT_GROUP,
            manager: None,
            graphic_system: None,
            driver: None,
            message_server: None,
            next_id: 1,
            alive: true,
        }
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Exclusively borrowed for driver/resource setup; no owner policy is delegated.
    pub fn provider_mut(&mut self) -> &mut P {
        &mut self.provider
    }

    pub fn query_interface(&mut self, iid: u32) -> Option<u32> {
        if !self.alive {
            return None;
        }
        let offset = match iid {
            1 | 0xFAC0_E219 => 0,
            0x088C_EB67 => 4,
            0x287F_B697 => 8,
            _ => return None,
        };
        self.provider.add_ref();
        Some(offset)
    }

    pub fn add_ref(&mut self) -> Option<u32> {
        self.alive.then(|| self.provider.add_ref())
    }

    pub fn release(&mut self) -> Option<u32> {
        if !self.alive {
            return None;
        }
        let count = self.provider.release();
        if count == 0 {
            self.deleting_destructor(1);
        }
        Some(count)
    }

    /// Returns whether the native outer allocation would be freed (bit 0).
    pub fn deleting_destructor(&mut self, flags: u32) -> bool {
        self.shutdown();
        self.alive = false;
        flags & 1 != 0
    }

    /// §1.3: acquired references on an early failure live until Shutdown/next Init.
    pub fn init(&mut self) -> bool {
        if !self.alive {
            return false;
        }
        self.shutdown();
        self.manager = self.provider.acquire(Service::ResourceManager, None);
        if self.manager.is_none() {
            return false;
        }
        self.graphic_system = self.provider.acquire(Service::GraphicSystem, None);
        if self.graphic_system.is_none() {
            return false;
        }
        self.driver = self.provider.acquire(Service::Driver, self.graphic_system);
        if self.driver.is_none() {
            return false;
        }
        self.message_server = self.provider.acquire(Service::MessageServer, None);
        if let Some(server) = self.message_server {
            self.provider.subscribe(server, RESTORE_TEXTURES);
        }
        true
    }

    pub fn shutdown(&mut self) -> bool {
        self.flush_all();
        for (service, id) in [
            (Service::Driver, self.driver.take()),
            (Service::GraphicSystem, self.graphic_system.take()),
            (Service::ResourceManager, self.manager.take()),
        ] {
            if let Some(id) = id {
                self.provider.release_service(service, id);
            }
        }
        if let Some(server) = self.message_server.take() {
            self.provider.unsubscribe(server, RESTORE_TEXTURES);
            self.provider.release_service(Service::MessageServer, server);
        }
        true
    }

    /// Borrowed incoming driver: retain before storing, release only our old reference.
    pub fn set_driver(&mut self, driver: Option<u64>) -> bool {
        if !self.alive || self.driver == driver {
            return false;
        }
        self.flush_all();
        if let Some(old) = self.driver.take() {
            self.provider.release_service(Service::Driver, old);
        }
        if let Some(new) = driver {
            self.provider.retain(Service::Driver, new);
        }
        self.driver = driver;
        true
    }

    pub fn driver(&self) -> Option<u64> {
        self.driver
    }
    pub fn lod_offset(&self) -> u32 {
        self.lod
    }
    pub fn set_lod_offset(&mut self, lod: u32) {
        self.lod = lod;
    }
    pub fn display_depth(&self) -> u32 {
        self.display_depth
    }
    pub fn set_display_depth(&mut self, depth: u32) {
        self.display_depth = depth;
    }
    pub fn resource_group(&self) -> u32 {
        self.group
    }
    pub fn free_pool_max(&self) -> u32 {
        self.free_pool_max
    }
    pub fn stats(&self) -> Stats {
        self.stats
    }
    pub fn stats_string(&self) -> String {
        self.stats.format()
    }

    pub fn set_resource_group(&mut self, group: u32) {
        if self.group != group {
            for id in self.keys.values() {
                if let Some(record) = self.records.get_mut(id) {
                    record.instance = 0;
                }
            }
            self.keys.clear();
        }
        self.group = group;
    }

    pub fn binding(&self, id: BindingId) -> Option<&Binding> {
        self.records.get(&id).map(|record| &record.binding)
    }

    pub fn get_binding_info(&self, id: BindingId, out: &mut BindingInfo) {
        if let Some(record) = self.records.get(&id) {
            *out = BindingInfo {
                group: record.group,
                instance: record.instance,
                levels: record.params.levels,
                width: record.params.width,
                height: record.params.height,
                bits: record.bits,
                bits_per_texel: record.params.bits_per_texel,
            };
        }
    }

    pub fn add_binding_ref(&mut self, id: BindingId) -> Option<u32> {
        self.records.get_mut(&id)?.binding.add_ref()
    }

    pub fn release_binding(&mut self, id: BindingId) -> Option<u32> {
        let result = self.records.get_mut(&id)?.binding.release()?;
        match result {
            Release::FreeToPool => self.free_binding(id),
            Release::Destroyed => self.destroy(id),
            Release::Live(_) => {}
        }
        // Return 1 even when trimming has immediately destroyed the pooled object.
        Some(result.count())
    }

    pub fn get_binding_from_image(&mut self, image: &dyn ImageInfo) -> Option<BindingId> {
        if !self.alive {
            return None;
        }
        let params = BindingParams::for_image(image.description(), self.lod, self.display_depth).ok()?;
        let id = self.reuse_or_create(params, image.description().bits)?;
        self.set_binding_image_data(id, image);
        self.add_binding_ref(id)?;
        Some(id)
    }

    /// §1.5 quirk: ignores the configured resource group.
    pub fn get_binding_by_id(&mut self, instance: u32) -> Option<BindingId> {
        self.get_binding_by_key(ResourceKey {
            type_id: IMAGE_TYPE,
            group: DEFAULT_GROUP,
            instance,
        })
    }

    pub fn get_binding_by_key(&mut self, key: ResourceKey) -> Option<BindingId> {
        // TB6: instance zero is the unkeyed sentinel; its cache contract is unpublished.
        if !self.alive || key.instance == 0 {
            return None;
        }
        if let Some(&id) = self.keys.get(&(key.group, key.instance)) {
            self.take_from_pool(id);
            self.add_binding_ref(id)?;
            return Some(id);
        }
        let resource = self.provider.load(self.manager?, key)?;
        let result = resource.image_info().and_then(|image| {
            let params = BindingParams::for_image(image.description(), self.lod, self.display_depth).ok()?;
            let id = self.reuse_or_create(params, image.description().bits)?;
            let record = self.records.get_mut(&id)?;
            record.group = key.group;
            record.instance = key.instance;
            self.keys.insert((key.group, key.instance), id);
            self.set_binding_image_data(id, image);
            self.add_binding_ref(id)?;
            Some(id)
        });
        self.provider.release_resource(resource);
        result
    }

    fn reuse_or_create(&mut self, params: BindingParams, bits: BitCounts) -> Option<BindingId> {
        let reusable = self.free.iter().rev().copied().find(|id| {
            let p = self.records[id].params;
            (p.frames, p.levels, p.width, p.height, p.internal_format)
                == (
                    params.frames,
                    params.levels,
                    params.width,
                    params.height,
                    params.internal_format,
                )
        });
        if let Some(id) = reusable {
            self.take_from_pool(id);
            let record = self.records.get_mut(&id)?;
            if record.instance != 0 {
                self.keys.remove(&(record.group, record.instance));
            }
            record.instance = 0;
            record.bits = bits;
            return Some(id);
        }
        let driver = self.driver?;
        let id = BindingId(self.next_id);
        let next = self.next_id.checked_add(1)?;
        let mut textures = Vec::new();
        for _ in 0..params.frames {
            let texture = self.provider.create_texture(driver, params, 2);
            textures.push(texture);
            if self.provider.driver_error(driver) {
                // TB7: includes the just-returned handle among textures made so far.
                for texture in textures {
                    self.provider.delete_texture(driver, texture);
                }
                return None;
            }
        }
        self.next_id = next;
        self.records.insert(
            id,
            Record {
                binding: Binding::new(textures, true),
                params,
                bits,
                in_use: true,
                group: 0,
                instance: 0,
            },
        );
        self.stats.in_use += 1;
        self.stats.in_use_bytes += u64::from(params.bytes);
        Some(id)
    }

    /// §1.5 and graphics-formats §2.10: last n source levels, per frame, no clamping.
    pub fn set_binding_image_data(&mut self, id: BindingId, image: &dyn ImageInfo) {
        let Some(record) = self.records.get(&id) else {
            return;
        };
        let Some(driver) = self.driver else {
            return;
        };
        let description = image.description();
        // TB7: malformed/mismatched level tables are outside the published domain.
        let Some(first) = description.levels.checked_sub(record.params.levels) else {
            return;
        };
        for (frame, texture) in record.binding.textures().iter().enumerate() {
            for level in 0..record.params.levels {
                let Some(data) = image.level_data(frame as u32, first + level) else {
                    return;
                };
                self.provider.upload(
                    driver,
                    Upload {
                        texture: *texture,
                        level,
                        width: record.params.width.checked_shr(level).unwrap_or(0),
                        height: record.params.height.checked_shr(level).unwrap_or(0),
                        format: description.format,
                        component_type: description.component_type,
                        data,
                    },
                );
            }
        }
    }

    fn take_from_pool(&mut self, id: BindingId) {
        let Some(record) = self.records.get_mut(&id) else {
            return;
        };
        if record.in_use {
            return;
        }
        self.free.retain(|entry| *entry != id);
        record.in_use = true;
        self.stats.pooled -= 1;
        self.stats.pooled_bytes = self.stats.pooled_bytes.saturating_sub(u64::from(record.params.bytes));
        self.stats.in_use += 1;
        self.stats.in_use_bytes += u64::from(record.params.bytes);
    }

    pub fn free_binding(&mut self, id: BindingId) {
        let Some(record) = self.records.get_mut(&id) else {
            return;
        };
        // Invalid duplicate frees are a safe no-op, not a claim about native misuse.
        if !record.in_use {
            return;
        }
        record.in_use = false;
        self.stats.in_use -= 1;
        self.stats.in_use_bytes -= u64::from(record.params.bytes);
        self.stats.pooled += 1;
        self.stats.pooled_bytes += u64::from(record.params.bytes);
        self.free.push_front(id);
        self.adjust_free_pool_size();
    }

    fn adjust_free_pool_size(&mut self) {
        while self.stats.pooled_bytes > u64::from(self.free_pool_max) {
            let Some(&oldest) = self.free.back() else {
                break;
            };
            self.destroy(oldest);
        }
    }

    fn destroy(&mut self, id: BindingId) {
        let Some(record) = self.records.remove(&id) else {
            return;
        };
        if record.instance != 0 {
            self.keys.remove(&(record.group, record.instance));
        }
        self.free.retain(|entry| *entry != id);
        if record.in_use {
            self.stats.in_use -= 1;
            self.stats.in_use_bytes -= u64::from(record.params.bytes);
        } else {
            self.stats.pooled -= 1;
            self.stats.pooled_bytes = self.stats.pooled_bytes.saturating_sub(u64::from(record.params.bytes));
        }
        if let Some(driver) = self.driver {
            for texture in record.binding.textures() {
                self.provider.delete_texture(driver, *texture);
            }
        }
        // Dropping the record frees binding storage even if caller references survive.
    }

    pub fn set_free_pool_max(&mut self, bytes: u32) {
        self.free_pool_max = bytes;
        self.adjust_free_pool_size();
    }

    pub fn flush_free_pool(&mut self) {
        let limit = self.free_pool_max;
        self.free_pool_max = 0;
        self.adjust_free_pool_size();
        self.free_pool_max = limit;
    }

    pub fn flush_all(&mut self) {
        self.free.clear();
        let ids: Vec<_> = self.records.keys().copied().collect();
        for id in ids {
            self.destroy(id);
        }
        self.keys.clear();
    }

    pub fn do_message(&mut self, message: u32) -> bool {
        if message == RESTORE_TEXTURES {
            if let Some(manager) = self.manager {
                // The spec publishes membership, not native hash traversal order.
                let keys: Vec<_> = self.keys.iter().map(|(&key, &id)| (key, id)).collect();
                for ((group, instance), id) in keys {
                    let key = ResourceKey {
                        type_id: IMAGE_TYPE,
                        group,
                        instance,
                    };
                    if let Some(resource) = self.provider.load(manager, key) {
                        if let Some(image) = resource.image_info() {
                            self.set_binding_image_data(id, image);
                        }
                        self.provider.release_resource(resource);
                    }
                }
            }
        }
        true
    }
}

impl<P: Provider> Drop for BindingFactory<P> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
