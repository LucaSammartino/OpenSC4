//! Primitive reconciliation dependencies. Any attempt to reconcile a host-side
//! configuration instead of the editor's records fails these tests immediately.
use crate::lot_config_manager::*;
use crate::property::PropertyHolder;
use crate::stl::StlHashSet;
use crate::stream::{RefValue, ResKey};
use std::rc::Rc;

pub(super) fn manager() -> Rc<LotConfigManager> {
    LotConfigManager::new(Rc::new(Dependencies), LotConfigCatalogue::shared(), 1)
}
struct Dependencies;
macro_rules! unused_slots {
    ($(fn $name:ident (&self $(, $arg:ident : $ty:ty)*) $(-> $out:ty)?;)*) => {
        $(fn $name(&self $(, $arg: $ty)*) $(-> $out)? {
            $(let _ = $arg;)*
            panic!("unexpected manager slot: {}", stringify!($name))
        })*
    };
}
impl LotConfigHost for Dependencies {
    unused_slots! {
        fn detail_level(&self) -> u32;
        fn exemplar_keys(&self) -> Option<Vec<ResKey>>;
        fn register_shutdown_callback(&self);
        fn unregister_shutdown_callback(&self);
        fn create_configuration(&self) -> u64;
        fn load_configuration(&self, cfg: u64, key: ResKey, detail: u32) -> bool;
        fn config_id(&self, cfg: u64) -> u32;
        fn config_belongs_to_family(&self, cfg: u64, family: u32) -> bool;
        fn config_families(&self, cfg: u64) -> Vec<u32>;
        fn config_size(&self, cfg: u64) -> (u32, u32);
        fn config_objects(&self, cfg: u64) -> Vec<LotConfigObject>;
        fn set_config_objects(&self, cfg: u64, objects: Vec<LotConfigObject>);
        fn config_primary_building(&self, cfg: u64) -> Option<LotConfigObject>;
        fn set_config_building_info(&self, cfg: u64, info: &BuildingInfo);
        fn config_retaining_walls(&self, cfg: u64) -> Vec<u32>;
        fn set_config_retaining_walls(&self, cfg: u64, ids: Vec<u32>);
        fn create_generic_configuration(&self, ty: u32, size: (u32, u32), objects: Vec<LotConfigObject>) -> Option<u64>;
        fn acquire_service(&self, service: LcmService) -> Option<u64>;
        fn subscribe(&self, msg_type: u32);
        fn unsubscribe(&self, msg_type: u32);
        fn add_ref(&self, token: u64);
        fn release(&self, token: u64);
        fn filter_matches(&self, filter: u64, cfg: u64) -> bool;
        fn building_exemplar(&self, ty: u32) -> Option<Rc<PropertyHolder>>;
        fn is_building_type_valid(&self, id: u32) -> bool;
        fn building_types_by_family(&self, vec: &mut Vec<u32>, id: u32) -> u32;
        fn building_types_into_set(&self, set: &mut StlHashSet, id: u32) -> u32;
        fn is_building_in_family(&self, ty: u32, family: u32) -> bool;
        fn building_capacity(&self, ty: u32) -> u16;
        fn object_ref(&self, token: u64) -> RefValue;
        fn resolve_ref(&self, r: &RefValue, iid: u32) -> Option<u64>;
    }
    fn is_building_family_type_valid(&self, id: u32) -> bool {
        id == 10
    }
    fn is_prop_family_type_valid(&self, id: u32) -> bool {
        id == 10
    }
    fn is_prop_type_valid(&self, id: u32) -> bool {
        id == 10
    }
    fn is_flora_type_valid(&self, id: u32) -> bool {
        id == 10
    }
    fn is_retaining_wall_valid(&self, id: u32) -> bool {
        id == 10
    }
    fn flora_types_into_set(&self, _: &mut StlHashSet, _: u32) -> u32 {
        0
    }
    fn prop_types_by_family(&self, ids: &mut Vec<u32>, id: u32) -> u32 {
        ids.push(id);
        0
    }
    fn building_footprint(&self, _: u32) -> Option<(f32, f32)> {
        Some((2.0, 4.0))
    }
    fn prop_size(&self, _: u32) -> Option<(f32, f32)> {
        Some((2.0, 4.0))
    }
    fn flora_radius(&self, _: u32) -> Option<f32> {
        Some(1.0)
    }
}
