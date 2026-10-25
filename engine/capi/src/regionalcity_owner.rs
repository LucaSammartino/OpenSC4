// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link RegionalCity owner. No native object layout crosses FFI.
use fc4_core::regional_city::RegionalCity;
use std::sync::Mutex;

pub struct Owner(Mutex<RegionalCity>);

// All pointers must come from create and remain live until destroy. The caller
// excludes destroy from concurrent calls; every state access takes the mutex.
#[no_mangle]
pub extern "C" fn fc4_regionalcity_owner_create(difficulty: u8) -> *mut Owner {
    Box::into_raw(Box::new(Owner(Mutex::new(RegionalCity::new(difficulty)))))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_destroy(owner: *mut Owner) {
    if !owner.is_null() {
        drop(Box::from_raw(owner));
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_get(
    owner: *const Owner,
    field: u32,
    arg: u32,
    out: *mut f64,
) -> bool {
    let (Some(owner), Some(out)) = (owner.as_ref(), out.as_mut()) else { return false };
    let Ok(mut c) = owner.0.lock() else { return false };
    *out = match field {
        0 => c.position.0 as f64,
        1 => c.position.1 as f64,
        2 => c.size.0 as f64,
        3 => c.size.1 as f64,
        4 => c.get_total_population() as f64,
        5 => c.get_commercial_jobs() as f64,
        6 => c.get_industrial_jobs() as f64,
        7 => *c.get_workforce_percentage() as f64,
        8 => c.get_mayor_rating() as f64,
        9 => c.get_difficulty_level() as f64,
        10 => c.get_serial_number() as f64,
        11 => c.get_birth_date() as f64,
        12 => u32::from(c.get_established()) as f64,
        13..=15 => c.world[(field - 13) as usize] as f64,
        16 => c.get_world_hemisphere() as f64,
        17 => c.get_budget(),
        18 => c.get_income(),
        19 => c.get_tutorial_guid() as f64,
        20 => u32::from(c.is_tutorial()) as f64,
        21 => c.original_locale.0 as f64,
        22 => c.original_locale.1 as f64,
        23 => c.last_locale.0 as f64,
        24 => c.last_locale.1 as f64,
        25 => c.connections.len() as f64,
        26 => c.local_deals.len() as f64,
        40 => c.get_tax_rate(arg) as f64,
        41 => c.get_population(arg) as f64,
        42 => c.get_extrapolated_population(arg) as f64,
        43 => c.get_allowable_extrapolation(arg) as f64,
        50 => c.get_produced(arg as i32),
        51 => c.get_demanded(arg as i32),
        52 => c.get_exported(arg as i32),
        53 => c.get_imported(arg as i32),
        54 => c.get_cost_per_unit(arg as i32),
        55 => c.get_commodity_balance(arg as i32),
        _ => return false,
    };
    true
}

#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_set(
    owner: *mut Owner,
    field: u32,
    arg: u32,
    value: f64,
) -> bool {
    let Some(owner) = owner.as_ref() else { return false };
    let Ok(mut c) = owner.0.lock() else { return false };
    match field {
        10 => c.set_serial_number(value as u32),
        11 => c.set_birth_date(value as u32),
        12 => c.set_established(value != 0.0),
        13..=15 => c.world[(field - 13) as usize] = value as f32,
        17 => c.set_budget(value),
        18 => c.set_income(value),
        19 => c.set_tutorial_guid(value as i32),
        42 => c.extrapolate_growth(arg, value as f32),
        50 => c.set_produced(arg as i32, value),
        51 => c.set_demanded(arg as i32, value),
        52 => c.set_exported(arg as i32, value),
        53 => c.set_imported(arg as i32, value),
        54 => c.set_cost_per_unit(arg as i32, value),
        _ => return false,
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_position(
    owner: *mut Owner,
    x: i32,
    z: i32,
    move_connections: bool,
) -> bool {
    let Some(owner) = owner.as_ref() else { return false };
    let Ok(mut c) = owner.0.lock() else { return false };
    c.set_position(x, z, move_connections);
    true
}

#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_size(owner: *mut Owner, x: i32, z: i32) -> bool {
    let Some(owner) = owner.as_ref() else { return false };
    let Ok(mut c) = owner.0.lock() else { return false };
    c.set_city_size((x, z));
    true
}

fn text(c: &mut RegionalCity, field: u32) -> Option<&mut Vec<u8>> {
    Some(match field {
        30 => &mut c.city_name,
        31 => &mut c.mayor_name,
        32 => &mut c.utility_advisor_name,
        33 => &mut c.description,
        34 => &mut c.file_path,
        _ => return None,
    })
}

// Returns the byte count, excluding any terminator. No partial copy: a caller
// may query with null/zero, allocate, then retry; UINT32_MAX denotes failure.
#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_text(
    owner: *const Owner,
    field: u32,
    out: *mut u8,
    capacity: u32,
) -> u32 {
    let Some(owner) = owner.as_ref() else { return u32::MAX };
    let Ok(mut c) = owner.0.lock() else { return u32::MAX };
    let Some(bytes) = text(&mut c, field) else { return u32::MAX };
    if !out.is_null() && capacity as usize >= bytes.len() {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
    }
    bytes.len() as u32
}

#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_set_text(
    owner: *mut Owner,
    field: u32,
    bytes: *const u8,
    len: u32,
) -> bool {
    let Some(owner) = owner.as_ref() else { return false };
    if bytes.is_null() && len != 0 {
        return false;
    }
    let Ok(mut c) = owner.0.lock() else { return false };
    let Some(dst) = text(&mut c, field) else { return false };
    let src = if len == 0 { &[] } else { std::slice::from_raw_parts(bytes, len as usize) };
    *dst = src.to_vec();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_preserves_defaults_setter_semantics_and_isolation() {
        unsafe {
            let a = fc4_regionalcity_owner_create(2);
            let b = fc4_regionalcity_owner_create(1);
            let get = |owner, field, arg| {
                let mut value = -999.0;
                assert!(fc4_regionalcity_owner_get(owner, field, arg, &mut value));
                value
            };
            assert_eq!(get(a, 9, 0), 2.0);
            assert_eq!(get(b, 9, 0), 1.0);
            assert_eq!(get(a, 7, 0), 0.5);
            assert_eq!(get(a, 40, 0x1010), 7.0);
            assert_eq!(get(a, 19, 0), -1.0);
            assert!(fc4_regionalcity_owner_set(a, 19, 0, 42.0));
            assert_eq!(get(a, 20, 0), 1.0);
            assert_eq!(get(b, 20, 0), 0.0);
            assert!(fc4_regionalcity_owner_set(a, 50, 99, -12.0));
            assert_eq!(get(a, 50, 99), 0.0);
            assert!(fc4_regionalcity_owner_set(a, 51, 99, 12.0));
            assert_eq!(get(a, 55, 99), -12.0);
            assert!(fc4_regionalcity_owner_position(a, -4, 8, false));
            assert_eq!(get(a, 0, 0), -4.0);
            assert!(fc4_regionalcity_owner_size(a, 2, 4));
            assert_eq!(get(a, 3, 0), 4.0);
            assert!(!fc4_regionalcity_owner_set(a, 4, 0, 1.0));
            fc4_regionalcity_owner_destroy(a);
            fc4_regionalcity_owner_destroy(b);
        }
    }

    #[test]
    fn strings_are_copied_and_bad_inputs_do_not_mutate() {
        unsafe {
            let owner = fc4_regionalcity_owner_create(0);
            let bytes = b"A\0B";
            assert!(fc4_regionalcity_owner_set_text(owner, 30, bytes.as_ptr(), 3));
            let mut out = [99_u8; 4];
            assert_eq!(fc4_regionalcity_owner_text(owner, 30, out.as_mut_ptr(), 2), 3);
            assert_eq!(out, [99; 4]);
            assert_eq!(fc4_regionalcity_owner_text(owner, 30, out.as_mut_ptr(), 4), 3);
            assert_eq!(&out[..3], bytes);
            assert!(!fc4_regionalcity_owner_set_text(owner, 30, std::ptr::null(), 1));
            assert!(fc4_regionalcity_owner_set_text(owner, 30, std::ptr::null(), 0));
            assert_eq!(fc4_regionalcity_owner_text(owner, 30, std::ptr::null_mut(), 0), 0);
            assert_eq!(fc4_regionalcity_owner_text(owner, 999, std::ptr::null_mut(), 0), u32::MAX);
            assert!(!fc4_regionalcity_owner_get(owner, 0, 0, std::ptr::null_mut()));
            assert!(!fc4_regionalcity_owner_get(std::ptr::null(), 0, 0, &mut 0.0));
            fc4_regionalcity_owner_destroy(owner);
            fc4_regionalcity_owner_destroy(std::ptr::null_mut());
        }
    }
}
