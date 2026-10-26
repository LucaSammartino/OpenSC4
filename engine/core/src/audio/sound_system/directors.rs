// SPDX-License-Identifier: GPL-3.0-or-later
//! Class factories/lifecycle (§8), sharing existing audio director identities.
use super::NativeFault;
use std::any::Any;
use std::sync::{Arc, Mutex, OnceLock};

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
    fn disable_updates(&mut self, system: &ForeignRef);
    fn sound_system_service(&mut self) -> Option<ForeignRef>;
    /// Borrowed cache getter followed by an acquired reference in the adapter.
    fn acquire_property_cache(&mut self, system: &ForeignRef) -> Option<Arc<dyn PropertyCache>>;
}
pub trait PropertyCache: Send + Sync {
    fn create_instance_set(&self, instance: u32) -> bool;
}
pub trait ConstructionIdentity: Send + Sync {
    fn query(&self, iid: u32, output: &mut Option<ForeignRef>) -> bool;
    fn add_ref(&self);
    fn release(&self);
}
pub trait ClassConstructor {
    fn construct(&mut self, class: u32) -> Option<Arc<dyn ConstructionIdentity>>;
}
pub trait DirectorChild: Send + Sync {
    fn initialize(&self, com: &ForeignRef, path: &str) -> bool;
    fn enum_classes(&self, visit: &mut dyn FnMut(u32));
    fn get_class_object(&self, class: u32, iid: u32, output: &mut Option<ForeignRef>) -> Result<bool, NativeFault>;
    fn can_unload(&self) -> bool;
}
/// Shared 17-slot director contract (§8.1); secondary view uses offset +4.
#[derive(Default)]
pub struct DirectorState {
    refs: u32,
    com: Option<ForeignRef>,
    framework: Option<ForeignRef>,
    path: String,
    pub children: Vec<Arc<dyn DirectorChild>>,
    pub classes: Vec<u32>,
}
impl DirectorState {
    pub fn with_classes(classes: &[u32]) -> Self {
        Self {
            classes: classes.to_vec(),
            ..Self::default()
        }
    }
    pub fn query(&mut self, iid: u32, output_offset: &mut u32) -> bool {
        let offset = match iid {
            1 | 0xA21E_E941 => 0,
            0x03FA_40BF => 4,
            _ => return false,
        };
        *output_offset = offset;
        self.add_ref();
        true
    }
    pub fn add_ref(&mut self) -> u32 {
        self.refs = self.refs.wrapping_add(1);
        self.refs
    }
    pub fn remove_ref(&mut self) -> u32 {
        self.refs = self.refs.saturating_sub(1);
        self.refs
    }
    pub fn release(&mut self) -> u32 {
        self.remove_ref()
    }
    pub fn ref_count(&self) -> u32 {
        self.refs
    }
    pub fn initialize(&mut self, com: Option<ForeignRef>, framework: Option<ForeignRef>, path: &str) -> bool {
        let Some(com) = com else {
            return false;
        };
        self.com = Some(com.clone());
        self.framework = framework;
        self.path = path.to_owned();
        for child in &self.children {
            child.initialize(&com, path);
        }
        true
    }
    pub fn framework(&self) -> Option<&ForeignRef> {
        self.framework.as_ref()
    }
    pub fn com(&self) -> Option<&ForeignRef> {
        self.com.as_ref()
    }
    pub fn library_path(&self, output: &mut String) -> bool {
        *output = self.path.clone();
        true
    }
    pub fn size_query(&self) -> u32 {
        0
    }
    pub fn no_op(&self) -> bool {
        true
    }
    pub fn can_unload(&self) -> bool {
        self.refs != 0 || self.children.iter().all(|child| child.can_unload())
    }
    pub fn enum_classes(&self, visit: &mut dyn FnMut(u32)) {
        for child in &self.children {
            child.enum_classes(visit);
        }
        for class in &self.classes {
            visit(*class);
        }
    }
    pub fn get_class_object(
        &self,
        class: u32,
        iid: u32,
        output: &mut Option<ForeignRef>,
        constructors: &mut dyn ClassConstructor,
    ) -> Result<bool, NativeFault> {
        for child in &self.children {
            if child.get_class_object(class, iid, output)? {
                return Ok(true);
            }
        }
        if !self.classes.contains(&class) {
            return Ok(false);
        }
        let identity = constructors
            .construct(class)
            .ok_or(NativeFault::NullConstructionIdentity)?;
        if identity.query(iid, output) {
            return Ok(true);
        }
        identity.add_ref();
        identity.release();
        Ok(false)
    }
}
/// Backend callback allocation size and returned identity offset (§8.2).
pub const MAC_CONSTRUCTION_LAYOUTS: [(u32, u32, u32); 5] = [
    (0xFDED_7F31, 0xD8, 0),
    (0xFDED_7F34, 0x100, 0xD8),
    (0xE9D2_8415, 0xA0, 8),
    (0x89D3_D983, 0x54, 4),
    (0x1C5F_21FE, 0x14, 0),
];
#[derive(Default)]
pub struct SoundDirector {
    pub system: Option<ForeignRef>,
    pub manager: Option<ForeignRef>,
    pub inherited: DirectorState,
}
impl SoundDirector {
    pub fn construct(host: &mut dyn DirectorHost) -> Self {
        for class_id in SOUND_CLASSES {
            host.register_class(class_id);
        }
        Self {
            inherited: DirectorState::with_classes(&SOUND_CLASSES),
            ..Self::default()
        }
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
            host.disable_updates(system);
        }
        true
    }
    pub const POST_INIT_CACHE_VALUE: u32 = 0xCC30_14EE;
    pub fn post_app_init(&mut self, host: &mut dyn DirectorHost) -> Result<bool, NativeFault> {
        let Some(system) = host.sound_system_service() else {
            return Ok(false);
        };
        let cache = host
            .acquire_property_cache(&system)
            .ok_or(NativeFault::MissingPropertyCache)?;
        cache.create_instance_set(Self::POST_INIT_CACHE_VALUE);
        drop(cache);
        drop(system);
        Ok(true)
    }
}

pub struct MacSoundDirector {
    pub inherited: DirectorState,
}
impl MacSoundDirector {
    pub const ID: u32 = 0xE9D3_A016;
    pub fn construct(host: &mut dyn DirectorHost) -> Self {
        for class_id in MAC_CLASSES {
            host.register_class(class_id);
        }
        Self {
            inherited: DirectorState::with_classes(&MAC_CLASSES),
        }
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
    pub fn pre_app_shutdown(&self, host: &mut dyn DirectorHost) -> bool {
        if let Some(system) = host.sound_system_service() {
            host.disable_updates(&system);
        }
        true
    }
}

pub struct FreshnessDirector {
    pub id: u32,
    pub classes: [u32; 5],
    pub inherited: Mutex<DirectorState>,
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
            inherited: Mutex::new(DirectorState::with_classes(&FRESH_CLASSES)),
        }
    })
}
pub fn get_freshness_director_alt(
    register_and_arrange_exit_cleanup: impl FnOnce(&[u32]),
) -> &'static FreshnessDirector {
    get_freshness_director(register_and_arrange_exit_cleanup)
}
