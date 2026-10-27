// SPDX-License-Identifier: GPL-3.0-or-later
//! Private, main-thread-only static-link application shadow. No DLL ABI exports.
//! AP17: paths and lifecycle message state only; no guessed timers or startup defaults.
use fc4_core::application::compatibility::{AppValue, Embedded};
use fc4_core::application::{
    Application, Directories, Effect, Host, HostFailure, Object, Preferences, Service, UserFolder,
};
use fc4_core::gz_messages::Token;

const SCALAR_SLOTS: [u32; 16] = [21, 23, 26, 32, 34, 36, 37, 40, 41, 43, 45, 46, 47, 48, 49, 50];
const METHOD_COUNT: usize = 20 + SCALAR_SLOTS.len();

struct Shadow {
    app: Application,
    seeded: bool,
    inputs_seen: u16,
    methods: [MethodEvidence; METHOD_COUNT],
}

#[derive(Clone, Copy, Default)]
struct MethodEvidence {
    calls: u64,
    complete: u64,
    mismatches: u64,
}

impl MethodEvidence {
    fn record(&mut self, complete: bool, mismatch: bool) -> u32 {
        self.calls += 1;
        self.complete += u64::from(complete);
        self.mismatches += u64::from(mismatch);
        u32::from(mismatch)
    }
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
        inputs_seen: 0,
        methods: [MethodEvidence::default(); METHOD_COUNT],
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
        }
        _ => return 1,
    }
    shadow.inputs_seen |= 1 << field;
    shadow.seeded = shadow.inputs_seen == 0x1FF;
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
unsafe extern "C" fn fc4_app_shadow_mode(shadow: *mut Shadow, native: u32, inputs_observed: u32) -> u32 {
    if shadow.is_null() {
        return 1;
    }
    // SAFETY: observer owns this live handle; never shared with game getter threads.
    let shadow = unsafe { &mut *shadow };
    let mismatch = inputs_observed == 0 || shadow.app.mode != native;
    shadow.methods[18].record(inputs_observed != 0, mismatch)
}

#[no_mangle]
unsafe extern "C" fn fc4_app_shadow_delivery(shadow: *mut Shadow, kind: u32, result: u32, mode: u32) -> u32 {
    if shadow.is_null() {
        return 1;
    }
    // SAFETY: the app-target forwarder owns the serialized live handle. The
    // input transition has already been applied, without reading native mode.
    let shadow = unsafe { &mut *shadow };
    let mode_input = matches!(kind, 0x26D3_1EC2 | 0x8BB5_BB46 | 0xCBB5_BB45 | 0xEA8A_E29A);
    let mismatch = result != 1 || (mode_input && shadow.app.mode != mode);
    // SPEC-GAP: AP19 supplies the pure mode branches, but actual GetType calls
    // are not traced; other branches additionally lack timer/collaborator capture.
    // Count every app delivery, including unsupported branches, as incomplete.
    shadow.methods[19].record(false, mismatch)
}

// These section-2 getters make no collaborator calls. A host that fails on any
// attempted call prevents accidental expansion to region forwarding or I/O.
struct NoGetterHost;
impl fc4_core::application::save_path::SavePathHost for NoGetterHost {
    fn save_directory(&mut self, _: Token) -> Option<Vec<u8>> {
        unreachable!()
    }
    fn city_name(&mut self, _: Token) -> Vec<u8> {
        unreachable!()
    }
    fn city_id(&mut self, _: Token) -> u32 {
        unreachable!()
    }
    fn filename_system_roundtrip(&mut self, _: &[u8]) -> Vec<u8> {
        unreachable!()
    }
    fn filename_encoding_valid(&mut self, _: &[u8]) -> bool {
        unreachable!()
    }
    fn file_exists(&mut self, _: &[u8]) -> bool {
        unreachable!()
    }
    fn existing_city_id(&mut self, _: &[u8]) -> Option<u32> {
        unreachable!()
    }
}
impl fc4_core::application::city::CityHost for NoGetterHost {
    fn query(&mut self, _: fc4_core::application::city::Query) -> fc4_core::application::city::Reply {
        unreachable!()
    }
    fn effect(&mut self, _: fc4_core::application::city::Effect) {
        unreachable!()
    }
    fn create(&mut self, _: fc4_core::application::city::Object) -> Token {
        unreachable!()
    }
    fn regional_geometry(&mut self, _: Token) -> fc4_core::application::city::RegionalCity {
        unreachable!()
    }
    fn all_cities(&mut self, _: Token) -> Vec<fc4_core::application::city::RegionalCity> {
        unreachable!()
    }
    fn city_at(&mut self, _: Token, _: i32, _: i32) -> Token {
        unreachable!()
    }
    fn adjacent_cities(&mut self, _: Token) -> Vec<Token> {
        unreachable!()
    }
}

#[no_mangle]
unsafe extern "C" fn fc4_app_shadow_scalar(
    shadow: *mut Shadow,
    index: u32,
    before: u32,
    result: u32,
    after: u32,
    owner: u32,
) -> u32 {
    let Some(&slot) = SCALAR_SLOTS.get(index as usize) else {
        return 1;
    };
    if shadow.is_null() {
        return 1;
    }
    // SAFETY: independent owner state is captured before the native getter;
    // result is never used to initialize the model. Tokens remain borrowed.
    let shadow = unsafe { &mut *shadow };
    let token = (before != 0).then_some(u64::from(before));
    match slot {
        21 => shadow.app.debug_enabled = before != 0,
        23 => shadow.app.popup_dialogs_enabled = before != 0,
        26 => shadow.app.handles.stored_4c = before,
        32 => shadow.app.field_314 = token,
        34 => shadow.app.city_state.region_owner = token,
        36 => shadow.app.city_state.regional_city = token,
        37 => shadow.app.city_state.city = token,
        40 => shadow.app.handles.debug_console = token,
        41 => shadow.app.handles.gimex_factory = token,
        43 => shadow.app.handles.window_location_saver = token,
        46 => shadow.app.handles.unnamed_660 = token,
        47 => shadow.app.handles.lua_interpreter = token,
        50 => shadow.app.handles.flag_668 = before as u8,
        45 | 48 | 49 => (),
        _ => unreachable!(),
    }
    let expected = match shadow.app.app_interface_getter(slot, None, &mut NoGetterHost) {
        Ok(AppValue::Bool(value)) => u32::from(value),
        Ok(AppValue::Byte(value)) => u32::from(value),
        Ok(AppValue::Word(value)) => value,
        Ok(AppValue::Token(value)) => value.unwrap_or(0) as u32,
        Ok(AppValue::Embedded(Embedded::GlyphTextureManager)) => owner.wrapping_add(0x1E8),
        Ok(AppValue::Embedded(Embedded::TutorialRegistry)) => owner.wrapping_add(0x1CC),
        _ => unreachable!("bounded scalar getter set"),
    };
    shadow.methods[20 + index as usize].record(true, result != expected || before != after)
}

#[no_mangle]
unsafe extern "C" fn fc4_app_shadow_compare(shadow: *mut Shadow, field: u32, bytes: *const u8, len: u32) -> u32 {
    if shadow.is_null() || field >= 18 {
        return 1;
    }
    // SAFETY: caller supplies exclusive live handle and len readable bytes.
    let shadow = unsafe { &mut *shadow };
    let captured = shadow.seeded && !bytes.is_null() && len <= 32768;
    let mismatch = if captured {
        let sample = unsafe { std::slice::from_raw_parts(bytes, len as usize) };
        expected(shadow, field).as_deref() != Some(sample)
    } else {
        true
    };
    // SPEC-GAP: SDK string getters return bool, but sections 2/9 do not define
    // that result (AP21). Comparing out alone cannot complete any string method.
    // SPEC-GAP: lazy-folder cache writes and base-getter calls are not captured;
    // matching their strings alone must never grant complete method credit.
    shadow.methods[field as usize].record(false, mismatch)
}

#[no_mangle]
unsafe extern "C" fn fc4_app_shadow_method_stats(shadow: *const Shadow, field: u32, out: *mut u64) -> u32 {
    if shadow.is_null() || out.is_null() || field as usize >= METHOD_COUNT {
        return 1;
    }
    // SAFETY: private static-link transport; caller owns the live handle and
    // three writable u64s. Diagnostics never execute or count another method.
    let evidence = unsafe { &*shadow }.methods[field as usize];
    unsafe {
        out.write(evidence.calls);
        out.add(1).write(evidence.complete);
        out.add(2).write(evidence.mismatches);
    }
    0
}

#[no_mangle]
unsafe extern "C" fn fc4_app_shadow_expected(shadow: *mut Shadow, field: u32, out: *mut u8, cap: u32) -> u32 {
    if shadow.is_null() || out.is_null() || cap == 0 {
        return 0;
    }
    // SAFETY: private diagnostic transport; caller owns the handle and cap output bytes.
    let Some(value) = expected(unsafe { &mut *shadow }, field) else {
        return 0;
    };
    let count = value.len().min(cap as usize - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(value.as_ptr(), out, count);
        *out.add(count) = 0;
    }
    count as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_shadow(name: &[u8]) -> *mut Shadow {
        let shadow = fc4_app_shadow_new();
        unsafe {
            for field in 0..8 {
                assert_eq!(fc4_app_shadow_input(shadow, field, b"base\\".as_ptr(), 5), 0);
            }
            assert_eq!(fc4_app_shadow_input(shadow, 8, name.as_ptr(), name.len() as u32), 0);
        }
        shadow
    }

    fn stats(shadow: *const Shadow, field: u32) -> [u64; 3] {
        let mut out = [0; 3];
        assert_eq!(
            unsafe { fc4_app_shadow_method_stats(shadow, field, out.as_mut_ptr()) },
            0
        );
        out
    }

    #[test]
    fn scalar_getters_compare_null_high_bit_and_embedded_identity_without_adopting_outputs() {
        let shadow = fc4_app_shadow_new();
        unsafe {
            for (index, slot) in SCALAR_SLOTS.iter().enumerate() {
                for before in [0, 1, 0xFF, 0x8000_0010, u32::MAX] {
                    let before = if matches!(slot, 21 | 23 | 50) {
                        before & 0xFF
                    } else {
                        before
                    };
                    let owner = 0x1234_0000;
                    let result = match slot {
                        21 | 23 => u32::from(before != 0),
                        45 | 48 => owner + 0x1E8,
                        49 => owner + 0x1CC,
                        _ => before,
                    };
                    assert_eq!(
                        fc4_app_shadow_scalar(shadow, index as u32, before, result, before, owner),
                        0
                    );
                    // Neither a corrupted output nor a forbidden field change
                    // may disappear into the next sample's captured input.
                    assert_eq!(
                        fc4_app_shadow_scalar(shadow, index as u32, before, result ^ 1, before, owner),
                        1
                    );
                    assert_eq!(
                        fc4_app_shadow_scalar(shadow, index as u32, before, result, before ^ 1, owner),
                        1
                    );
                }
                assert_eq!(stats(shadow, 20 + index as u32), [15, 15, 10]);
            }
            assert_eq!(fc4_app_shadow_scalar(shadow, 16, 0, 0, 0, 0), 1);
            fc4_app_shadow_free(shadow);
        }
    }

    #[test]
    fn every_delivery_counts_and_partial_outputs_never_complete_the_method() {
        let shadow = fc4_app_shadow_new();
        unsafe {
            for (kind, mode) in [(0xCBB5_BB45, 1), (0xEA8A_E29A, 2), (0x26D3_1EC2, 0), (0x8BB5_BB46, 0)] {
                fc4_app_shadow_message(shadow, kind);
                assert_eq!(fc4_app_shadow_delivery(shadow, kind, 1, mode), 0);
            }
            // A non-mode branch is retained even when its effects are missing.
            assert_eq!(fc4_app_shadow_delivery(shadow, 0xBA44_EB51, 1, 0), 0);
            assert_eq!(fc4_app_shadow_delivery(shadow, 0xFA0C_6390, 0, 0), 1);
            assert_eq!(fc4_app_shadow_delivery(shadow, 0x26D3_1EC2, 1, 2), 1);
            assert_eq!(stats(shadow, 19), [7, 0, 2]);
            assert_eq!(fc4_app_shadow_mode(shadow, 0, 1), 0);
            fc4_app_shadow_free(shadow);
        }
    }

    #[test]
    fn string_output_mismatch_is_cumulative_and_unobserved_returns_stay_incomplete() {
        let shadow = seeded_shadow(b"SimCity 4");
        unsafe {
            for field in 0..8 {
                assert_eq!(fc4_app_shadow_compare(shadow, field, b"base\\".as_ptr(), 5), 0);
                assert_eq!(stats(shadow, field), [1, 0, 0]);
            }
            assert_eq!(fc4_app_shadow_compare(shadow, 0, b"wrong".as_ptr(), 5), 1);
            assert_eq!(fc4_app_shadow_compare(shadow, 0, b"base\\".as_ptr(), 5), 0);
            let mut out = [0; 16];
            assert_eq!(fc4_app_shadow_expected(shadow, 0, out.as_mut_ptr(), 16), 5);
            assert_eq!(stats(shadow, 0), [3, 0, 1]);
            assert_eq!(stats(shadow, 1), [1, 0, 0]);
            assert_eq!(fc4_app_shadow_input(shadow, 0, b"wrong".as_ptr(), 5), 1);
            assert_eq!(stats(shadow, 0), [3, 0, 1]);
            fc4_app_shadow_free(shadow);
        }
    }

    #[test]
    fn missing_capture_never_counts_as_complete_even_when_default_bytes_match() {
        let shadow = fc4_app_shadow_new();
        unsafe {
            // A missing stored input cannot silently become an empty-base input.
            for field in 1..8 {
                assert_eq!(fc4_app_shadow_input(shadow, field, b"".as_ptr(), 0), 0);
            }
            assert_eq!(fc4_app_shadow_input(shadow, 8, b"".as_ptr(), 0), 0);
            assert_eq!(fc4_app_shadow_compare(shadow, 0, b"".as_ptr(), 0), 1);
            assert_eq!(stats(shadow, 0), [1, 0, 1]);
            assert_eq!(fc4_app_shadow_input(shadow, 0, b"".as_ptr(), 0), 0);
            assert_eq!(fc4_app_shadow_compare(shadow, 0, std::ptr::null(), 0), 1);
            assert_eq!(fc4_app_shadow_compare(shadow, 0, b"".as_ptr(), 32769), 1);
            assert_eq!(stats(shadow, 0), [3, 0, 3]);
            assert_eq!(fc4_app_shadow_compare(shadow, 0, b"".as_ptr(), 0), 0);
            assert_eq!(stats(shadow, 0), [4, 0, 3]);
            fc4_app_shadow_free(shadow);
        }
    }

    #[test]
    fn exact_cached_strings_and_missing_mode_inputs_do_not_earn_complete_credit() {
        let shadow = seeded_shadow(b"SimCity 4");
        unsafe {
            let path = b"base\\Plugins\\";
            assert_eq!(fc4_app_shadow_compare(shadow, 8, path.as_ptr(), path.len() as u32), 0);
            assert_eq!(stats(shadow, 8), [1, 0, 0]);
            assert_eq!(fc4_app_shadow_mode(shadow, 0, 0), 1);
            fc4_app_shadow_message(shadow, 0xEA8A_E29A);
            assert_eq!(fc4_app_shadow_mode(shadow, 2, 1), 0);
            assert_eq!(stats(shadow, 18), [2, 1, 1]);
            fc4_app_shadow_free(shadow);
        }
    }

    #[test]
    fn empty_filename_preserves_output_without_claiming_the_unobserved_return() {
        let shadow = seeded_shadow(b"");
        unsafe {
            for field in [16, 17] {
                let sentinel = b"app-shadow-sentinel";
                assert_eq!(
                    fc4_app_shadow_compare(shadow, field, sentinel.as_ptr(), sentinel.len() as u32),
                    0
                );
                assert_eq!(fc4_app_shadow_compare(shadow, field, b"".as_ptr(), 0), 1);
                assert_eq!(stats(shadow, field), [2, 0, 1]);
            }
            fc4_app_shadow_free(shadow);
        }
    }

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
            assert_eq!(fc4_app_shadow_mode(shadow, 1, 1), 0);
            fc4_app_shadow_message(shadow, 0xEA8A_E29A);
            assert_eq!(fc4_app_shadow_mode(shadow, 1, 1), 1);
            assert_eq!(fc4_app_shadow_mode(shadow, 2, 1), 0);
            fc4_app_shadow_message(shadow, 0x26D3_1EC2);
            assert_eq!(fc4_app_shadow_mode(shadow, 0, 1), 0);
            fc4_app_shadow_free(shadow);
        }
    }
}
