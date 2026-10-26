// SPDX-License-Identifier: GPL-3.0-or-later
//! §1 Release, §3.1 Init, §3.3 Shutdown and §3.4 destruction.
//! PostCityInit (§3.2) depends on unfinished occupant handlers and AddModels dispatch.
use super::{Handle, ModelMaker, Port};

pub const INIT_MESSAGES: [u32; 3] = [0x26D3_1EC1, 0x26C6_3343, 0x26C6_3345];
const CITY_MESSAGES: [u32; 25] = [
    0x99EF_1140,
    0x99EF_1141,
    0x99EF_1142,
    0x99EF_1143,
    0x99EF_1144,
    0x99EF_1145,
    0x99EF_1146,
    0xE6E8_5114,
    0xE99B_3D4D,
    0x09D8_E0B6,
    0x2977_AE2F,
    0xE9C8_6DDB,
    0x06CD_B65B,
    0x49D9_68C4,
    0xA6B7_9601,
    0xA6B7_9602,
    0xA6B7_9620,
    0xA6B7_9611,
    0xCAD6_1E00,
    0xEAD6_1E16,
    0xC9DA_96EA,
    0xBA44_EB51,
    0xC9F7_75BB,
    0x4AB0_4CBE,
    0x4AB0_4CBF,
];

/// A returned handle carries one counted reference. Filter construction is a
/// foreign occupant-filter service, supplied the exact §6.1 acceptance lists.
pub trait LifecycleHost {
    fn acquire(&mut self, port: Port) -> Option<Handle>;
    fn create_filter(&mut self, port: Port, types: &[u32]) -> Option<Handle>;
    fn renderer_aux(&mut self, renderer: Handle) -> Option<Handle>;
    fn release(&mut self, handle: Handle);
    fn shutdown_service(&mut self, port: Port, handle: Handle);
    fn subscribe(&mut self, message: u32, enable: bool);
    fn unregister_tick(&mut self, value: u32);
    fn memory_mib(&mut self) -> Option<u32>;
    fn detail_option(&mut self) -> i32;
}

pub const OCCUPANT_TYPES: [u32; 11] = [
    0x2781_28A0,
    0x68FD_0C69,
    0x895D_1169,
    0x7475_8926,
    0xA823_821E,
    0x497F_6D9D,
    0xC772_BF98,
    0x2890_D4DE,
    0x2A18_3167,
    0x8A13_918A,
    0x6A11_BD2B,
];
pub const HIDING_TYPES: [u32; 5] = [0x68FD_0C69, 0x497F_6D9D, 0xC772_BF98, 0x2A18_3167, 0x8A13_918A];

impl ModelMaker {
    fn acquire_missing(&mut self, port: Port, host: &mut impl LifecycleHost) {
        if self.port(port).is_none() {
            if let Some(handle) = host.acquire(port) {
                self.ports.insert(port, handle);
            }
        }
    }

    pub fn init(&mut self, host: &mut impl LifecycleHost) -> bool {
        if self.initialized {
            return true;
        }
        self.acquire_missing(Port::ViewFilter, host);
        for (port, types) in [
            (Port::TypeFilter, OCCUPANT_TYPES.as_slice()),
            (Port::HidingFilter, HIDING_TYPES.as_slice()),
            (Port::ZoomRotationFilter, &[][..]),
        ] {
            if self.port(port).is_none() {
                if let Some(handle) = host.create_filter(port, types) {
                    self.ports.insert(port, handle);
                }
            }
        }
        for id in INIT_MESSAGES {
            host.subscribe(id, true);
        }
        self.acquire_missing(Port::ResourceManager, host);
        self.acquire_missing(Port::Renderer, host);
        self.renderer_aux = self.port(Port::Renderer).and_then(|r| host.renderer_aux(r));
        self.light_colour = [0.0; 3];
        self.detail = host.detail_option().clamp(0, 2);
        self.determine_load_radii(host.memory_mib());
        self.initialized = true;
        true
    }

    fn release_port(&mut self, port: Port, host: &mut impl LifecycleHost) {
        if let Some(handle) = self.ports.remove(&port) {
            host.release(handle);
        }
    }

    pub fn shutdown(&mut self, host: &mut impl LifecycleHost) -> bool {
        if !self.initialized {
            return true;
        }
        for port in [Port::AnimationCoordinator, Port::BackgroundLoader] {
            if let Some(handle) = self.port(port) {
                host.shutdown_service(port, handle);
                self.release_port(port, host);
            }
        }
        for port in [
            Port::TickManager,
            Port::Terrain,
            Port::Development,
            Port::LotManager,
            Port::OccupantManager,
        ] {
            self.release_port(port, host);
        }
        self.renderer_aux = None;
        self.release_port(Port::Renderer, host);
        self.release_port(Port::ResourceManager, host);
        // §3.3 specifies nulling these slots; §3.4 only releases those remaining.
        self.textures.fill(None);
        if self.city_subscribed {
            self.city_subscribed = false;
            host.unregister_tick(0xC0);
            for id in CITY_MESSAGES {
                host.subscribe(id, false);
            }
        }
        for id in INIT_MESSAGES {
            host.subscribe(id, false);
        }
        for port in [
            Port::ViewFilter,
            Port::ZoomRotationFilter,
            Port::HidingFilter,
            Port::TypeFilter,
        ] {
            self.release_port(port, host);
        }
        for entry in self.deferred.drain(..) {
            host.release(entry.occupant);
        }
        self.initialized = false;
        true
    }

    pub fn destroy(&mut self, host: &mut impl LifecycleHost) {
        for entry in self.deferred.drain(..) {
            host.release(entry.occupant);
        }
        for texture in self.textures.iter_mut().rev() {
            if let Some(handle) = texture.take() {
                host.release(handle);
            }
        }
        self.disposed = true;
    }

    pub fn release(&mut self, host: &mut impl LifecycleHost) -> u32 {
        if self.references > 1 {
            self.references -= 1;
            return self.references;
        }
        self.shutdown(host);
        self.destroy(host);
        self.references = 0;
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        acquired: Vec<Port>,
        released: Vec<Handle>,
        subscriptions: Vec<(u32, bool)>,
        filters: Vec<(Port, Vec<u32>)>,
    }
    impl LifecycleHost for Host {
        fn acquire(&mut self, p: Port) -> Option<Handle> {
            self.acquired.push(p);
            Some(10 + p as u64)
        }
        fn create_filter(&mut self, p: Port, t: &[u32]) -> Option<Handle> {
            self.filters.push((p, t.to_vec()));
            Some(10 + p as u64)
        }
        fn renderer_aux(&mut self, _: Handle) -> Option<Handle> {
            Some(99)
        }
        fn release(&mut self, h: Handle) {
            self.released.push(h);
        }
        fn shutdown_service(&mut self, _: Port, _: Handle) {}
        fn subscribe(&mut self, id: u32, b: bool) {
            self.subscriptions.push((id, b));
        }
        fn unregister_tick(&mut self, _: u32) {}
        fn memory_mib(&mut self) -> Option<u32> {
            Some(512)
        }
        fn detail_option(&mut self) -> i32 {
            5
        }
    }
    #[test]
    fn m2_final_release_runs_shutdown_init_is_idempotent_and_keeps_saved_refs() {
        let mut m = ModelMaker::default();
        let mut h = Host::default();
        m.ports.insert(Port::Renderer, 123);
        m.init(&mut h);
        m.init(&mut h);
        assert_eq!(h.acquired, [Port::ViewFilter, Port::ResourceManager]);
        assert_eq!(h.filters[0].1, OCCUPANT_TYPES);
        assert_eq!(h.filters[1].1, HIDING_TYPES);
        assert_eq!(h.subscriptions.len(), 3);
        assert_eq!(m.detail, 2);
        assert_eq!(m.load_radii, [256, 128, 64, 32, 16]);
        m.add_ref();
        m.add_ref();
        assert_eq!(m.release(&mut h), 1);
        assert!(h.released.is_empty());
        assert_eq!(m.release(&mut h), 0);
        assert!(m.disposed());
        assert!(!m.initialized);
        assert_eq!(h.released, [123, 10, 14, 17, 16, 15]);
        assert!(!h.released.contains(&99)); // Borrowed renderer aux.
    }
    #[test]
    fn destructor_releases_only_remaining_textures_in_reverse_order() {
        let mut m = ModelMaker {
            textures: [Some(1), None, Some(3), Some(4), Some(5)],
            ..Default::default()
        };
        let mut h = Host::default();
        m.deferred.push(super::super::DeferredEntry {
            occupant: 9,
            operation: 3,
            arguments: [0; 2],
        });
        m.destroy(&mut h);
        assert_eq!(h.released, [9, 5, 4, 3, 1]);
        assert_eq!(m.textures, [None; 5]);
    }
}
