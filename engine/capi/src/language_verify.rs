// SPDX-License-Identifier: GPL-3.0-or-later
//! Private transport to the production utility. Inputs and initial output bytes
//! are constructed independently of native results. Host callbacks are foreign
//! LocaleHost services, never calls to the native utility under comparison.
use fc4_core::language::{LanguageUtility, LocaleHost, Released};
use std::cmp::Ordering;

type HostFn = unsafe extern "C" fn(u32, *const u8, usize, *const u8, usize, f64, *mut u8, usize) -> i32;

struct ForeignHost(HostFn);
impl ForeignHost {
    fn bytes(&self, op: u32, a: &[u8], b: &[u8], value: f64) -> Vec<u8> {
        let mut out = vec![0; 8192];
        let n = unsafe {
            (self.0)(
                op,
                a.as_ptr(),
                a.len(),
                b.as_ptr(),
                b.len(),
                value,
                out.as_mut_ptr(),
                out.len(),
            )
        };
        assert!(n >= 0 && n as usize <= out.len(), "foreign LocaleHost refused");
        out.truncate(n as usize);
        out
    }
}
impl LocaleHost for ForeignHost {
    fn sprintf_double(&self, f: &[u8], v: f64) -> Vec<u8> {
        self.bytes(0, f, &[], v)
    }
    fn utf8_to_ansi(&self, s: &[u8]) -> Vec<u8> {
        self.bytes(1, s, &[], 0.0)
    }
    fn ansi_to_utf8(&self, s: &[u8]) -> Vec<u8> {
        self.bytes(2, s, &[], 0.0)
    }
    fn char_lower_buff(&self, s: &mut [u8]) {
        s.copy_from_slice(&self.bytes(3, s, &[], 0.0));
    }
    fn char_upper_buff(&self, s: &mut [u8]) {
        s.copy_from_slice(&self.bytes(4, s, &[], 0.0));
    }
    fn compare_ansi(&self, a: &[u8], b: &[u8], ignore: bool) -> Option<Ordering> {
        let r = unsafe {
            (self.0)(
                if ignore { 6 } else { 5 },
                a.as_ptr(),
                a.len(),
                b.as_ptr(),
                b.len(),
                0.0,
                std::ptr::null_mut(),
                0,
            )
        };
        match r {
            -1 => Some(Ordering::Less),
            0 => Some(Ordering::Equal),
            1 => Some(Ordering::Greater),
            _ => None,
        }
    }
    // Manager locale mutation is outside this verification's authorized scope.
    fn set_locale_all(&self, _: &[u8]) -> bool {
        false
    }
    fn set_mbcp_ansi(&self) {}
    fn user_default_lcid(&self) -> u32 {
        0
    }
}

#[repr(C)]
pub struct Request {
    op: u32,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    integer: i64,
    value: f64,
    text: *const u8,
    text_len: usize,
    other: *const u8,
    other_len: usize,
    out: *mut u8,
    capacity: usize,
    length: usize,
    host: HostFn,
}

/// Caller owns this Box and supplies it only to this module's functions.
#[no_mangle]
pub extern "C" fn fc4_language_verify_new(id: u32) -> *mut LanguageUtility {
    let mut core = LanguageUtility::new();
    core.add_ref();
    core.set_language(id);
    Box::into_raw(Box::new(core))
}

/// # Safety
/// `core` must be a live pointer from new, used exclusively on this thread;
/// request and all byte ranges must be valid for their stated sizes.
#[no_mangle]
pub unsafe extern "C" fn fc4_language_verify_call(core: *mut LanguageUtility, r: *mut Request) -> i32 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let u = &mut *core;
        let r = &mut *r;
        let text = std::slice::from_raw_parts(r.text, r.text_len);
        let other = std::slice::from_raw_parts(r.other, r.other_len);
        let mut out = std::slice::from_raw_parts(r.out, r.length).to_vec();
        let host = ForeignHost(r.host);
        let symbol = if r.a == 0 { None } else { Some(other) };
        let result = match r.op {
            0 => u.query_interface(r.a) as i32,
            1 => u.add_ref() as i32,
            2 => match u.release() {
                Released::Alive(n) => n as i32,
                Released::Destroy => 0,
            },
            3 => u.init() as i32,
            4 => u.shutdown() as i32,
            5 => u.day_name(r.a, &mut out) as i32,
            6 => u.abbr_day_name(r.a, &mut out) as i32,
            7 => u.month_name(r.a, &mut out) as i32,
            8 => u.abbr_month_name(r.a, &mut out) as i32,
            9 => u.make_time_string(r.a as i32, r.b as i32, r.c as i32, &mut out) as i32,
            10 => u.make_date_string(r.a, r.b, r.c as i32, &mut out, r.d) as i32,
            11 => u.currency_symbol(&mut out) as i32,
            12 => u.currency_symbol_precedes_amount() as i32,
            13 => u.space_between_currency_symbol_and_amount() as i32,
            14 => u.thousand_separator(&mut out) as i32,
            15 => u.decimal_separator(&mut out) as i32,
            16 => u.make_money_string(r.integer, &mut out, symbol) as i32,
            17 => u.make_formatted_money_string(&host, r.value, &mut out, text, symbol) as i32,
            18 => u.make_number_string(r.integer, &mut out) as i32,
            19 => u.make_formatted_number_string(&host, r.value, &mut out, text) as i32,
            20 => u.convert_to_lower_case(&host, text, &mut out) as i32,
            21 => u.convert_to_upper_case(&host, text, &mut out) as i32,
            22 => u.compare_strings(&host, text, other, r.a != 0),
            23 => u.character_set_from_language() as i32,
            24 => u.uses_character_set(r.a) as i32,
            25 => u.road_driving_side() as i32,
            26 => u.measurement_system() as i32,
            27 => u.are_first_and_last_names_reversed() as i32,
            28 => u.uses_input_method_editing() as i32,
            29 => u.uses_multibyte_characters() as i32,
            31 => {
                u.set_language(r.a);
                0
            }
            32 => {
                u.set_country(r.a);
                0
            }
            _ => return -999,
        };
        if out.len() > r.capacity {
            return -998;
        }
        std::ptr::copy_nonoverlapping(out.as_ptr(), r.out, out.len());
        r.length = out.len();
        result
    }))
    .unwrap_or(-997)
}

/// # Safety
/// `core` is the unfreed Box returned by new, after its last Release returned 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_language_verify_free(core: *mut LanguageUtility) {
    drop(Box::from_raw(core));
}
