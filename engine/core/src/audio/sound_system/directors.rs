// SPDX-License-Identifier: GPL-3.0-or-later
//! Class factories/lifecycle (§8), sharing existing audio director identities.
use super::Unsupported;
use std::any::Any;
use std::sync::{Arc, OnceLock};

pub const SOUND_CLASSES: [u32; 8] = [
    0xE41C_E9FD,
    0x7AF8_5B98,
    0xDC1F_73D6,
    0x1D07_EB4B,
    0xBB70_51F5,
    0x9D4C_6B95,
    0x7DC4_95CC,
    0xBDF6_13B4,
];
pub const MAC_CLASSES: [u32; 5] = [0xFDED_7F31, 0xFDED_7F34, 0xE9D2_8415, 0x89D3_D983, 0x1C5F_21FE];
pub const FRESH_CLASSES: [u32; 5] = [0x3C1C_F59E, 0x5C1C_F5A3, 0xBDDA_72DD, 0x7DD9_C9F2, 0xBDD9_D7A5];
pub const SERVICE_IID: u32 = 0x287F_B697;
pub type ForeignRef = Arc<dyn Any + Send + Sync>;
pub trait DirectorHost {
    fn register_class(&mut self, class_id: u32);
    fn register_hook(&mut self, director: u32);
    fn create(&mut self, class_id: u32, iid: u32) -> Option<ForeignRef>;
    fn add_service(&mut self, service: ForeignRef);
    fn register_handler(&mut self, system: &ForeignRef, handler: ForeignRef);
    fn panic(&mut self, system: &ForeignRef);
}
#[derive(Default)]
pub struct SoundDirector {
    pub system: Option<ForeignRef>,
    pub manager: Option<ForeignRef>,
}
impl SoundDirector {
    pub fn construct(host: &mut dyn DirectorHost) -> Self {
        for class_id in SOUND_CLASSES {
            host.register_class(class_id);
        }
        Self::default()
    }
    pub fn id(&self) -> u32 {
        crate::audio::director::GZ_SOUND_DIRECTOR.id
    }
    pub fn on_start(&mut self, host: &mut dyn DirectorHost) -> bool {
        host.register_hook(self.id());
        self.system = host.create(SOUND_CLASSES[0], SERVICE_IID);
        if let Some(service) = &self.system {
            host.add_service(Arc::clone(service));
        }
        self.manager = host.create(SOUND_CLASSES[1], SERVICE_IID);
        if let Some(service) = &self.manager {
            host.add_service(Arc::clone(service));
        }
        if let Some(handler) = host.create(0x7DC4_95CC, 0xBB52_3BB2) {
            if let Some(system) = &self.system {
                host.register_handler(system, handler);
            }
        }
        true
    }
    pub fn pre_app_shutdown(&mut self, host: &mut dyn DirectorHost) -> bool {
        if let Some(system) = &self.system {
            host.panic(system);
        }
        true
    }
    pub const POST_INIT_CACHE_VALUE: u32 = 0xCC30_14EE;
    pub fn post_app_init(&mut self) -> Result<bool, Unsupported> {
        if self.system.is_none() {
            Ok(false)
        } else {
            Err(Unsupported("SS10"))
        }
    }
}

pub struct MacSoundDirector;
impl MacSoundDirector {
    pub const ID: u32 = 0xE9D3_A016;
    pub fn construct(host: &mut dyn DirectorHost) -> Self {
        for class_id in MAC_CLASSES {
            host.register_class(class_id);
        }
        Self
    }
    pub fn on_start(&self, host: &mut dyn DirectorHost) -> bool {
        host.register_hook(Self::ID);
        for class_id in [0xE9D2_8415, 0x89D3_D983] {
            if let Some(service) = host.create(class_id, SERVICE_IID) {
                host.add_service(service);
            }
        }
        true
    }
}

pub struct FreshnessDirector {
    pub id: u32,
    pub classes: [u32; 5],
}
static FRESHNESS: OnceLock<FreshnessDirector> = OnceLock::new();
/// The native host arranges process-exit cleanup of acquired factory/native
/// references. Rust's immutable registration metadata has no foreign resources.
pub fn get_freshness_director(register_and_arrange_exit_cleanup: impl FnOnce(&[u32])) -> &'static FreshnessDirector {
    FRESHNESS.get_or_init(|| {
        register_and_arrange_exit_cleanup(&FRESH_CLASSES);
        FreshnessDirector {
            id: crate::audio::director::FRESHNESS_DIRECTOR.id,
            classes: FRESH_CLASSES,
        }
    })
}
pub fn get_freshness_director_alt(
    register_and_arrange_exit_cleanup: impl FnOnce(&[u32]),
) -> &'static FreshnessDirector {
    get_freshness_director(register_and_arrange_exit_cleanup)
}
