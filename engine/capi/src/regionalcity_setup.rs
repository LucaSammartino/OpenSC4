// SPDX-License-Identifier: GPL-3.0-or-later
use super::Owner;
use fc4_core::application::save_path::{create_save_path, SavePathHost};
use fc4_core::neighbors::flag;
use fc4_core::regional_city::DealHandle;
use std::ffi::c_void;

#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_finish_read(
    owner: *mut Owner,
    difficulty: u8,
    default_name: *const u8,
    len: u32,
) -> bool {
    let Some(owner) = owner.as_ref() else { return false };
    if default_name.is_null() && len != 0 {
        return false;
    }
    let deals = {
        let Ok(mut c) = owner.0.lock() else { return false };
        if !c.established {
            c.difficulty = difficulty;
            c.city_name = if len == 0 {
                vec![]
            } else {
                std::slice::from_raw_parts(default_name, len as usize).to_vec()
            };
        }
        // tutorial-registry §4.1: the two legacy save IDs.
        c.tutorial_id = match c.tutorial_id {
            0 => 0x8A5B_7A6Cu32 as i32,
            1 => 0xEA5D_6DC8u32 as i32,
            id => id,
        };
        let local = std::mem::take(&mut c.local_deals);
        c.other_deals.extend(local);
        c.other_deals.clone()
    };
    for d in deals {
        d.set_flags(flag::SILENT, true);
    }
    true
}
type Sink = unsafe extern "C" fn(*mut c_void, *const u8, u32);
type Exists = unsafe extern "C" fn(*mut c_void, *const u8, u32) -> bool;
type Roundtrip = unsafe extern "C" fn(*mut c_void, *const u8, u32, *mut c_void, Sink);
struct PathHost {
    context: *mut c_void,
    exists: Exists,
    roundtrip: Roundtrip,
}
unsafe extern "C" fn bytes(p: *mut c_void, s: *const u8, n: u32) {
    if n != 0 && !s.is_null() {
        (*p.cast::<Vec<u8>>()).extend_from_slice(std::slice::from_raw_parts(s, n as usize));
    }
}
impl SavePathHost for PathHost {
    fn save_directory(&mut self, _: u64) -> Option<Vec<u8>> {
        None
    }
    fn city_name(&mut self, _: u64) -> Vec<u8> {
        vec![]
    }
    fn city_id(&mut self, _: u64) -> u32 {
        0
    }
    fn filename_system_roundtrip(&mut self, b: &[u8]) -> Vec<u8> {
        let mut out = vec![];
        unsafe {
            (self.roundtrip)(
                self.context,
                b.as_ptr(),
                b.len() as u32,
                (&mut out as *mut Vec<u8>).cast(),
                bytes,
            )
        };
        out
    }
    fn filename_encoding_valid(&mut self, b: &[u8]) -> bool {
        std::str::from_utf8(b).is_ok()
    }
    fn file_exists(&mut self, p: &[u8]) -> bool {
        unsafe { (self.exists)(self.context, p.as_ptr(), p.len() as u32) }
    }
    fn existing_city_id(&mut self, _: &[u8]) -> Option<u32> {
        None
    } // SetupCity always uses cityID 0.
}
#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_save_path(
    owner: *const Owner,
    base: *const u8,
    len: u32,
    context: *mut c_void,
    exists: Exists,
    roundtrip: Roundtrip,
    output: *mut c_void,
    emit: Sink,
) -> bool {
    let Some(owner) = owner.as_ref() else { return false };
    if base.is_null() && len != 0 {
        return false;
    }
    let name = match owner.0.lock() {
        Ok(c) => c.city_name.clone(),
        Err(_) => return false,
    };
    let base = if len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(base, len as usize)
    };
    let path = create_save_path(
        base,
        &name,
        0,
        &mut PathHost {
            context,
            exists,
            roundtrip,
        },
    );
    emit(output, path.as_ptr(), path.len() as u32);
    true
}
