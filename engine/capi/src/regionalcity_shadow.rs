// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link live-record differential. No public DLL exports.
use fc4_core::regional_city::{RegionalCity, TAX_TYPES};
use fc4_core::stream::{ByteReader, ByteWriter};
use std::ffi::c_void;

type Probe = unsafe extern "C" fn(*mut c_void, u32, u32, f64, *const u8, u32) -> bool;

fn compare(c: &mut RegionalCity, mut probe: impl FnMut(u32, u32, f64, &[u8]) -> bool) -> u64 {
    let mut fields = 0_u32;
    let mut differences = 0_u32;
    let mut check = |field, arg, value, bytes: &[u8]| {
        fields += 1;
        differences += u32::from(!probe(field, arg, value, bytes));
    };
    // IDs are private to FC4RegionalCityShadow.cpp, deliberately outside fc4.h.
    for (id, value) in [
        (0, c.position.0 as f64),
        (1, c.position.1 as f64),
        (2, c.size.0 as f64),
        (3, c.size.1 as f64),
        (4, c.get_total_population() as f64),
        (5, c.get_commercial_jobs() as f64),
        (6, c.get_industrial_jobs() as f64),
        (7, *c.get_workforce_percentage() as f64),
        (8, c.get_mayor_rating() as f64),
        (9, c.get_difficulty_level() as f64),
        (10, c.get_serial_number() as f64),
        (11, c.get_birth_date() as f64),
        (12, u32::from(c.get_established()) as f64),
        (13, c.world[0] as f64),
        (14, c.world[1] as f64),
        (15, c.world[2] as f64),
        (16, c.get_world_hemisphere() as f64),
        (17, c.get_budget()),
        (18, c.get_income()),
        (19, c.get_tutorial_guid() as u32 as f64),
        (20, u32::from(c.is_tutorial()) as f64),
        (21, c.original_locale.0 as f64),
        (22, c.original_locale.1 as f64),
        (23, c.last_locale.0 as f64),
        (24, c.last_locale.1 as f64),
        (25, c.connections.len() as f64),
        (26, c.local_deals.len() as f64),
    ] {
        check(id, 0, value, &[]);
    }
    for (id, text) in [
        (30, &c.city_name),
        (31, &c.mayor_name),
        (32, &c.utility_advisor_name),
        (33, &c.description),
    ] {
        check(id, 0, 0.0, text);
    }
    for t in TAX_TYPES.into_iter().chain([0]) {
        check(40, t, c.get_tax_rate(t) as f64, &[]);
    }
    // Query only present map keys; allowable extrapolation inserts entries in
    // the native object, so only compare it when BOTH entries already exist.
    let keys: std::collections::BTreeSet<_> = c
        .populations
        .keys()
        .chain(c.extrapolated.keys())
        .chain(c.potential.keys())
        .copied()
        .collect();
    for t in keys {
        check(41, t, c.get_population(t) as f64, &[]);
        check(42, t, c.get_extrapolated_population(t) as f64, &[]);
        if c.potential.contains_key(&t) && c.extrapolated.contains_key(&t) {
            check(43, t, c.get_allowable_extrapolation(t) as f64, &[]);
        }
    }
    let commodities: Vec<_> = c.commodities.keys().copied().collect();
    for t in commodities {
        for (id, value) in [
            (50, c.get_produced(t)),
            (51, c.get_demanded(t)),
            (52, c.get_exported(t)),
            (53, c.get_imported(t)),
            (54, c.get_cost_per_unit(t)),
            (55, c.get_commodity_balance(t)),
        ] {
            check(id, t as u32, value, &[]);
        }
    }
    // This independently exercises all decoded fields, including the pending
    // changes, connection/deal records, unknown strings and POIs without getters.
    let mut out = ByteWriter::new();
    if !c.write_record(&mut out) {
        check(100, 0, 0.0, &[]);
    } else {
        check(100, 0, 0.0, out.bytes());
    }
    (u64::from(fields) << 32) | u64::from(differences)
}

#[no_mangle]
unsafe extern "C" fn fc4_regionalcity_shadow_compare(
    data: *const u8,
    len: u32,
    probe: Option<Probe>,
    ctx: *mut c_void,
) -> u64 {
    let Some(probe) = probe else {
        return u64::MAX;
    };
    if data.is_null() || len > 16 * 1024 * 1024 {
        return u64::MAX;
    }
    // SAFETY: private caller supplies readable capture bytes for this call.
    let wire = unsafe { std::slice::from_raw_parts(data, len as usize) };
    let mut input = ByteReader::new(wire);
    let Ok(mut c) = RegionalCity::decode_record(&mut input, 0) else {
        return u64::MAX;
    };
    if input.remaining() != 0 {
        return u64::MAX;
    }
    compare(&mut c, |field, arg, value, bytes| {
        // SAFETY: callback and context live for this synchronous comparison.
        unsafe { probe(ctx, field, arg, value, bytes.as_ptr(), bytes.len() as u32) }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_getters_detect_altered_native_values() {
        let mut c = RegionalCity::new(2);
        c.populations.insert(0x1010, 37);
        c.extrapolated.insert(0x1010, 40);
        c.potential.insert(0x1010, 100);
        let mut out = ByteWriter::new();
        assert!(c.write_record(&mut out));
        let mut input = ByteReader::new(out.bytes());
        let mut decoded = RegionalCity::decode_record(&mut input, 2).unwrap();
        let result = compare(&mut decoded, |id, arg, value, bytes| {
            match id {
                41 => arg == 0x1010 && value == 37.0,
                42 => value == 40.0,
                43 => value == 60.0,
                100 => bytes == out.bytes(),
                10 | 52 => false, // altered serial and three exported values
                _ => true,
            }
        });
        assert_eq!(result as u32, 4);
        assert!(result >> 32 > 60);
    }

    #[test]
    fn malformed_or_trailing_record_is_not_an_exact_sample() {
        unsafe extern "C" fn accept(_: *mut c_void, _: u32, _: u32, _: f64, _: *const u8, _: u32) -> bool {
            true
        }
        let mut out = ByteWriter::new();
        assert!(RegionalCity::new(0).write_record(&mut out));
        let mut bytes = out.bytes().to_vec();
        bytes.push(0);
        // SAFETY: valid readable local bytes and a synchronous callback.
        unsafe {
            assert_eq!(
                fc4_regionalcity_shadow_compare(bytes.as_ptr(), bytes.len() as u32, Some(accept), std::ptr::null_mut(),),
                u64::MAX
            );
            assert_eq!(
                fc4_regionalcity_shadow_compare(bytes.as_ptr(), 1, Some(accept), std::ptr::null_mut(),),
                u64::MAX
            );
        }
    }
}
