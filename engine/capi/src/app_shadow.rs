// SPDX-License-Identifier: GPL-3.0-or-later
//! Private, main-thread-only static-link application shadow. No DLL ABI exports.
//! AP17: paths and lifecycle message state only; no guessed timers or startup defaults.
use fc4_core::application::{
    Application, Directories, Effect, Host, HostFailure, Object, Preferences, Service, UserFolder,
};
use fc4_core::gz_messages::Token;

struct Shadow {
    app: Application,
    seeded: bool,
}

fn new_shadow() -> Shadow {
    Shadow {
        app: Application::new(
            0,
            Preferences {
                ime: false,
                width: 0,
                height: 0,
                depth: 0,
                fullscreen: false,
                pause_on_focus_loss: false,
                audio_04: false,
                audio_69: false,
                radio: 0,
                language: Vec::new(),
                simulation: 0.0,
                rdd: false,
                driver_f13: 0,
                driver_f0c: 0,
            },
            Directories::default(),
        ),
        seeded: false,
    }
}

// Only the four section 6.2 mode messages enter this host. Any accidental
// expansion of that scope fails instead of pretending an effect was observed.
struct ModeHost;
impl Host for ModeHost {
    fn effect(&mut self, _: Effect) {
        unreachable!("mode messages have no host effects")
    }
    fn create(&mut self, _: Object) -> Result<Token, HostFailure> {
        unreachable!()
    }
    fn register_service(&mut self, _: Service, _: Token) -> Result<(), HostFailure> {
        unreachable!()
    }
    fn release_created(&mut self, _: Token) {
        unreachable!()
    }
    fn install_directory(&mut self) -> Option<Vec<u8>> {
        unreachable!()
    }
    fn graphics_driver(&mut self) -> Option<Token> {
        unreachable!()
    }
    fn check_graphics_driver(&mut self, _: Token) -> bool {
        unreachable!()
    }
    fn directory_exists(&mut self, _: &[u8]) -> bool {
        unreachable!()
    }
    fn watcher_running(&mut self, _: Token) -> bool {
        unreachable!()
    }
    fn watched_directory(&mut self, _: Token) -> Vec<u8> {
        unreachable!()
    }
    fn install_exception_filter(&mut self) -> Token {
        unreachable!()
    }
}

fn expected(shadow: &mut Shadow, field: u32) -> Option<Vec<u8>> {
    let app = &mut shadow.app;
    let dirs = &mut app.directories;
    Some(match field {
        0 => dirs.data_7c.clone(),
        1 => dirs.cd_data_90.clone(),
        2 => dirs.plugin_directory(-1, None),
        3 => dirs.language_directory(true, -1, None),
        4 => dirs.sku_cc.clone(),
        5 => dirs.user_data_e0.clone(),
        6 => dirs.slot_e4_108.clone(),
        7 => dirs.user_base_11c.clone(),
        8..=15 => dirs
            .user_folder(
                [
                    UserFolder::Plugins,
                    UserFolder::Regions,
                    UserFolder::MySim,
                    UserFolder::Albums,
                    UserFolder::HttpCache,
                    UserFolder::Temp,
                    UserFolder::ExceptionReports,
                    UserFolder::Scripts,
                ][(field - 8) as usize],
            )
            .to_vec(),
        16 | 17 => {
            let mut out = b"app-shadow-sentinel".to_vec();
            app.app_file_name(if field == 16 { b".ini" } else { b".cfg" }, &mut out);
            out
        }
        _ => return None,
    })
}

#[no_mangle]
extern "C" fn fc4_app_shadow_new() -> *mut Shadow {
    Box::into_raw(Box::new(new_shadow()))
}

#[no_mangle]
unsafe extern "C" fn fc4_app_shadow_free(shadow: *mut Shadow) {
    if !shadow.is_null() {
        // SAFETY: opaque handle is returned once by new and released once by Stop.
        drop(unsafe { Box::from_raw(shadow) });
    }
}

#[no_mangle]
unsafe extern "C" fn fc4_app_shadow_input(shadow: *mut Shadow, field: u32, bytes: *const u8, len: u32) -> u32 {
    if shadow.is_null() || bytes.is_null() || len > 32768 {
        return 1;
    }
    // SAFETY: plugin passes a live exclusive handle and len readable bytes.
    let shadow = unsafe { &mut *shadow };
    let value = unsafe { std::slice::from_raw_parts(bytes, len as usize) }.to_vec();
    if shadow.seeded {
        return 1;
    } // Never resync expected output from a sample.
    match field {
        0 => shadow.app.directories.data_7c = value,
        1 => shadow.app.directories.cd_data_90 = value,
        2 => shadow.app.directories.plugin_a4 = value,
        3 => shadow.app.directories.cd_plugin_b8 = value,
        4 => shadow.app.directories.sku_cc = value,
        5 => shadow.app.directories.user_data_e0 = value,
        6 => shadow.app.directories.slot_e4_108 = value,
        7 => shadow.app.directories.user_base_11c = value,
        8 => {
            shadow.app.app_name = value;
            shadow.seeded = true;
        }
        _ => return 1,
    }
    0
}

#[no_mangle]
unsafe extern "C" fn fc4_app_shadow_message(shadow: *mut Shadow, kind: u32) {
    if !shadow.is_null() && matches!(kind, 0x26D3_1EC2 | 0x8BB5_BB46 | 0xCBB5_BB45 | 0xEA8A_E29A) {
        // SAFETY: observer callbacks serialize access to this owned handle.
        let _ = unsafe { &mut *shadow }.app.message_v2(kind, &mut ModeHost);
    }
}

#[no_mangle]
unsafe extern "C" fn fc4_app_shadow_mode(shadow: *const Shadow, native: u32) -> u32 {
    if shadow.is_null() {
        return 1;
    }
    // SAFETY: observer owns this live handle; never shared with game getter threads.
    u32::from(unsafe { &*shadow }.app.mode != native)
}

#[no_mangle]
unsafe extern "C" fn fc4_app_shadow_compare(shadow: *mut Shadow, field: u32, bytes: *const u8, len: u32) -> u32 {
    if shadow.is_null() || bytes.is_null() || len > 32768 {
        return 1;
    }
    // SAFETY: caller supplies exclusive live handle and len readable bytes.
    let shadow = unsafe { &mut *shadow };
    let sample = unsafe { std::slice::from_raw_parts(bytes, len as usize) };
    u32::from(!shadow.seeded || expected(shadow, field).as_deref() != Some(sample))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_detects_corruption_and_keeps_message_state() {
        let shadow = fc4_app_shadow_new();
        unsafe {
            for field in 0..8 {
                assert_eq!(fc4_app_shadow_input(shadow, field, b"C:\\base\\".as_ptr(), 8), 0);
            }
            assert_eq!(fc4_app_shadow_input(shadow, 8, b"SimCity 4".as_ptr(), 9), 0);
            let path = b"C:\\base\\Plugins\\";
            assert_eq!(fc4_app_shadow_compare(shadow, 8, path.as_ptr(), path.len() as u32), 0);
            assert_eq!(fc4_app_shadow_compare(shadow, 8, b"wrong".as_ptr(), 5), 1);
            assert_eq!(fc4_app_shadow_compare(shadow, 16, b"SimCity 4.ini".as_ptr(), 13), 0);
            assert_eq!(fc4_app_shadow_input(shadow, 7, b"wrong".as_ptr(), 5), 1);
            fc4_app_shadow_message(shadow, 0xCBB5_BB45);
            assert_eq!(fc4_app_shadow_mode(shadow, 1), 0);
            fc4_app_shadow_message(shadow, 0xEA8A_E29A);
            assert_eq!(fc4_app_shadow_mode(shadow, 1), 1);
            assert_eq!(fc4_app_shadow_mode(shadow, 2), 0);
            fc4_app_shadow_message(shadow, 0x26D3_1EC2);
            assert_eq!(fc4_app_shadow_mode(shadow, 0), 0);
            fc4_app_shadow_free(shadow);
        }
    }
}
