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
        if files.is_null() {
            return std::ptr::null_mut();
        }
        let mut input = Vec::with_capacity(32);
        for file in std::slice::from_raw_parts(files, 32) {
            if file.present > 1 || file.len > isize::MAX as usize || (file.len != 0 && file.data.is_null()) {
                return std::ptr::null_mut();
            }
            input.push(if file.present == 0 {
                None
            } else if file.len == 0 {
                Some(Vec::new())
            } else {
                Some(std::slice::from_raw_parts(file.data, file.len).to_vec())
            });
        }
        let process = FeatureProcess::new();
        let mut manager = FeatureManager::new(process.clone());
        manager.init(&mut Files(input));
        // Published lifecycle, independent of native counts: app construction
        // AddRef, followed by the verifier's one QI-owned reference.
        manager.add_ref();
        let mut owned = None;
        manager.query_interface(0x0B56_2673, &mut owned);
        Box::into_raw(Box::new(fc4_feature_snapshot {
            manager,
            ids: process.mentioned_features(),
        }))
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_feature_snapshot_destroy(snapshot: *mut fc4_feature_snapshot) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !snapshot.is_null() {
            drop(Box::from_raw(snapshot));
        }
    }));
}

/// Portable lifetime mirror: 0 AddRef, 1 Release, 2 QI. QI returns bit 0
/// success, bit 1 identity preserved (sentinel is the existing identity).
#[no_mangle]
pub unsafe extern "C" fn fc4_feature_snapshot_lifetime(
    snapshot: *mut fc4_feature_snapshot,
    operation: u32,
    iid: u32,
) -> u32 {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(s) = snapshot.as_mut() else {
            return u32::MAX;
        };
        match operation {
            0 => s.manager.add_ref().unwrap_or(u32::MAX),
            1 => s.manager.release().unwrap_or(u32::MAX),
            2 => {
                let identity = s.manager.interface();
                let mut out = identity.clone();
                let ok = s.manager.query_interface(iid, &mut out);
                u32::from(ok) | (u32::from(out == identity) << 1)
            }
            _ => u32::MAX,
        }
    }))
    .unwrap_or(u32::MAX)
}

/// kind 0 feature, 1 pack (all u32 arguments), 2 current pack, 3 ID count, 4 ID at index.
#[no_mangle]
pub unsafe extern "C" fn fc4_feature_snapshot_query(snapshot: *const fc4_feature_snapshot, kind: u32, arg: u32) -> u32 {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(s) = snapshot.as_ref() else {
            return u32::MAX;
        };
        match kind {
            0 => s.manager.is_feature_enabled(arg).map(u32::from).unwrap_or(u32::MAX),
            1 => s
                .manager
                .is_expansion_pack_installed(arg)
                .map(u32::from)
                .unwrap_or(u32::MAX),
            2 => s.manager.get_current_expansion_pack().unwrap_or(u32::MAX),
            3 => s.ids.len() as u32,
            4 => s.ids.get(arg as usize).copied().unwrap_or(u32::MAX),
            _ => u32::MAX,
        }
    }))
    .unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_capture_is_copied_and_queried_without_native_answers() {
        let mut ini = b"[Features]\n0x10=on\n20=off\n1A=false\n".to_vec();
        let mut files: [fc4_feature_file; 32] = std::array::from_fn(|_| fc4_feature_file {
            data: std::ptr::null(),
            len: 0,
            present: 0,
        });
        files[2] = fc4_feature_file {
            data: ini.as_ptr(),
            len: ini.len(),
            present: 1,
        };
        files[31].present = 1; // Successfully acquired empty file installs pack 32.
        unsafe {
            let snapshot = fc4_feature_snapshot_create(files.as_ptr());
            assert!(!snapshot.is_null());
            ini.fill(b'!'); // Creation owns its independent input copy.
            assert_eq!(fc4_feature_snapshot_query(snapshot, 0, 16), 1);
            assert_eq!(fc4_feature_snapshot_query(snapshot, 0, 20), 0);
            assert_eq!(fc4_feature_snapshot_query(snapshot, 0, 26), 1);
            assert_eq!(fc4_feature_snapshot_query(snapshot, 1, 0), 1);
            assert_eq!(fc4_feature_snapshot_query(snapshot, 1, 35), 1);
            assert_eq!(fc4_feature_snapshot_query(snapshot, 1, 33), 0);
            assert_eq!(fc4_feature_snapshot_query(snapshot, 2, 0), 32);
            assert_eq!(fc4_feature_snapshot_query(snapshot, 3, 0), 3);
            assert_eq!(fc4_feature_snapshot_query(snapshot, 4, 0), 16);
            assert_eq!(fc4_feature_snapshot_query(snapshot, 4, 1), 20);
            assert_eq!(fc4_feature_snapshot_query(snapshot, 4, 2), 26);
            assert_eq!(fc4_feature_snapshot_query(snapshot, 4, 3), u32::MAX);
            assert_eq!(fc4_feature_snapshot_lifetime(snapshot, 0, 0), 3);
            assert_eq!(fc4_feature_snapshot_lifetime(snapshot, 1, 0), 2);
            assert_eq!(fc4_feature_snapshot_lifetime(snapshot, 2, 1), 3);
            assert_eq!(fc4_feature_snapshot_lifetime(snapshot, 1, 0), 2);
            assert_eq!(fc4_feature_snapshot_lifetime(snapshot, 2, 0x1234_5678), 2);
            assert_eq!(fc4_feature_snapshot_lifetime(snapshot, 1, 0), 1);
            fc4_feature_snapshot_destroy(snapshot);
            assert!(fc4_feature_snapshot_create(std::ptr::null()).is_null());
            files[0].present = 2; // Incomplete/unsupported capture cannot masquerade as absent.
            assert!(fc4_feature_snapshot_create(files.as_ptr()).is_null());
            assert_eq!(fc4_feature_snapshot_query(std::ptr::null(), 0, 0), u32::MAX);
            fc4_feature_snapshot_destroy(std::ptr::null_mut());
        }
    }
}
