// SPDX-License-Identifier: GPL-3.0-or-later
//! Complete-byte verification bridge; no native answer is accepted as input.
use fc4_core::feature_manager::{FeatureManager, FeatureProcess, PackResources, ResourceKey, FIRST_PACK_INSTANCE};
use std::panic::{catch_unwind, AssertUnwindSafe};

#[repr(C)]
pub struct fc4_feature_file {
    pub data: *const u8,
    pub len: usize,
    pub present: u32,
}

pub struct fc4_feature_snapshot {
    manager: FeatureManager,
    ids: Vec<u32>,
}

struct Files(Vec<Option<Vec<u8>>>);
impl PackResources for Files {
    type File = Vec<u8>;
    fn register_static_shutdown(&mut self, _: &FeatureProcess) {}
    fn open_resource(&mut self, key: ResourceKey) -> Option<Self::File> {
        self.0[(key.instance_id - FIRST_PACK_INSTANCE) as usize].take()
    }
    fn release_file(&mut self, _: Self::File) {}
}

/// Exactly 32 independently captured resources; caller retains all byte buffers.
#[no_mangle]
pub unsafe extern "C" fn fc4_feature_snapshot_create(files: *const fc4_feature_file) -> *mut fc4_feature_snapshot {
    catch_unwind(AssertUnwindSafe(|| {
        if files.is_null() { return std::ptr::null_mut(); }
        let mut input = Vec::with_capacity(32);
        for file in std::slice::from_raw_parts(files, 32) {
            if file.present > 1 || file.len > isize::MAX as usize || (file.len != 0 && file.data.is_null()) {
                return std::ptr::null_mut();
            }
            input.push(if file.present == 0 { None } else if file.len == 0 { Some(Vec::new()) }
                else { Some(std::slice::from_raw_parts(file.data, file.len).to_vec()) });
        }
        let process = FeatureProcess::new();
        let mut manager = FeatureManager::new(process.clone());
        manager.init(&mut Files(input));
        Box::into_raw(Box::new(fc4_feature_snapshot { manager, ids: process.mentioned_features() }))
    })).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_feature_snapshot_destroy(snapshot: *mut fc4_feature_snapshot) {
    let _ = catch_unwind(AssertUnwindSafe(|| { if !snapshot.is_null() { drop(Box::from_raw(snapshot)); } }));
}

/// kind 0 feature, 1 pack (all u32 arguments), 2 current pack, 3 ID count, 4 ID at index.
#[no_mangle]
pub unsafe extern "C" fn fc4_feature_snapshot_query(snapshot: *const fc4_feature_snapshot, kind: u32, arg: u32) -> u32 {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(s) = snapshot.as_ref() else { return u32::MAX; };
        match kind {
            0 => s.manager.is_feature_enabled(arg).map(u32::from).unwrap_or(u32::MAX),
            1 => s.manager.is_expansion_pack_installed(arg).map(u32::from).unwrap_or(u32::MAX),
            2 => s.manager.get_current_expansion_pack().unwrap_or(u32::MAX),
            3 => s.ids.len() as u32,
            4 => s.ids.get(arg as usize).copied().unwrap_or(u32::MAX),
            _ => u32::MAX,
        }
    })).unwrap_or(u32::MAX)
}
