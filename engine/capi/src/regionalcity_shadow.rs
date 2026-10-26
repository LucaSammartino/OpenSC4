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
            (
                55,
                c.get_commodity_balance_with_precision(t, crate::invocation::precision()),
            ),
        ] {
            check(id, t as u32, value, &[]);
        }
    }
    // regional-city §7.3: a present type replaces the caller's list, while an
    // absent type leaves it alone. The host seeds every output with this point.
    // Compare float bits so signed zero and NaN payloads survive this boundary.
    let sentinel = [0x3f80_0000_u32, 0xc000_0000, 0x4040_0000];
    for (&t, points) in &c.points {
        let bytes: Vec<_> = points
            .iter()
            .flatten()
            .flat_map(|value| value.to_bits().to_le_bytes())
            .collect();
        check(60, t, 0.0, &bytes);
    }
    // There is a free key among 0..=len, even when the saved types are sparse.
    let absent = (0..=c.points.len() as u32)
        .find(|t| !c.points.contains_key(t))
        .expect("one more candidate than present keys");
    let bytes: Vec<_> = sentinel.into_iter().flat_map(u32::to_le_bytes).collect();
    check(60, absent, 0.0, &bytes);
    // This independently exercises all decoded fields, including the pending
    // changes, connection/deal records and unknown strings without getters.
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
    // SAFETY: forwarded unchanged to the common capture decoder.
    unsafe { compare_capture(data, len, probe, ctx, None) }
}

// The host applies the same steps to an isolated, uninitialised COM object.
// Never mutate a live regional city: setters can affect its connections/deals.
fn mutate_fixture(c: &mut RegionalCity, step: u32) {
    match step {
        0 => {
            c.set_position(-3, 7, false);
            c.set_city_size((2, 4));
            c.set_serial_number(0x8123_4567);
            c.set_birth_date(0xf123_4567);
            c.set_established(true);
            c.set_world_position(12.5, -45.25);
            c.set_world_base_elevation(37.75);
            c.set_budget(-123456.125);
            c.set_income(98765.0625);
            c.set_tutorial_guid(0x1234_5678);
            c.set_city_name(b"FC4 isolated city");
            c.set_mayor_name(b"FC4 isolated mayor");
            c.set_utility_advisor_name(b"FC4 isolated utility");
            c.set_description(b"FC4 isolated description");
        }
        1 | 2 => {
            for commodity in [1, 2, 3, 97] {
                let value = if step == 1 { f64::from(commodity) + 0.125 } else { -1.0 };
                c.set_produced(commodity, value);
                c.set_demanded(commodity, value * 2.0);
                c.set_exported(commodity, value * 3.0);
                c.set_imported(commodity, value * 4.0);
                c.set_cost_per_unit(commodity, value * 5.0);
            }
        }
        3 => {
            c.set_established(false);
            c.set_tutorial_guid(-1);
            c.set_city_name(b"");
            c.set_mayor_name(b"");
            c.set_utility_advisor_name(b"");
            c.set_description(b"");
            c.extrapolate_growth(0x1010, 17.75);
            c.extrapolate_growth(0x1010, -2.5);
            c.get_allowable_extrapolation(0x1010);
        }
        _ => unreachable!("fixture step checked at the FFI boundary"),
    }
}

#[no_mangle]
unsafe extern "C" fn fc4_regionalcity_shadow_mutation_compare(
    data: *const u8,
    len: u32,
    step: u32,
    probe: Option<Probe>,
    ctx: *mut c_void,
) -> u64 {
    if step > 3 {
        return u64::MAX;
    }
    // SAFETY: same synchronous capture/probe contract as the getter entry.
    unsafe { compare_capture(data, len, probe, ctx, Some(step)) }
}

unsafe fn compare_capture(
    data: *const u8,
    len: u32,
    probe: Option<Probe>,
    ctx: *mut c_void,
    mutation: Option<u32>,
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
    if let Some(last_step) = mutation {
        for step in 0..=last_step {
            mutate_fixture(&mut c, step);
        }
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
    fn isolated_setters_cover_unknown_commodity_clamping_and_growth_insertion() {
        let mut c = RegionalCity::new(2);
        c.original_locale = (12, 34);
        mutate_fixture(&mut c, 0);
        assert_eq!(c.position, (-3, 7));
        assert_eq!(c.size, (2, 4));
        assert_eq!(c.serial, 0x8123_4567);
        assert_eq!(c.birth_date, 0xf123_4567);
        assert_eq!(c.budget, -123456.125);
        assert_eq!(c.income, 98765.0625);
        assert_eq!(c.city_name, b"FC4 isolated city");
        assert_eq!(c.original_locale, (12, 34));
        assert!(!c.moved, "non-rearranging position setter must not mark a move");
        mutate_fixture(&mut c, 1);
        assert_eq!(c.commodities.len(), 4);
        let extra = c.commodities[&97];
        assert_eq!(extra.produced, 97.125);
        assert_eq!(extra.demanded, 194.25);
        assert_eq!(extra.exported, 291.375);
        assert_eq!(extra.imported, 388.5);
        assert_eq!(extra.cost_per_unit, 485.625);
        mutate_fixture(&mut c, 2);
        assert!(c.commodities.values().all(|v| *v == Default::default()));
        mutate_fixture(&mut c, 3);
        assert!(!c.established);
        assert!(!c.is_tutorial());
        assert!(c.city_name.is_empty() && c.mayor_name.is_empty());
        assert_eq!(c.extrapolated[&0x1010], 14);
        assert_eq!(c.potential[&0x1010], 0);
        assert!(c.populations.is_empty());
    }

    #[test]
    fn mutation_boundary_rejects_invalid_step_and_compares_expected_not_captured_output() {
        unsafe extern "C" fn check(_: *mut c_void, id: u32, _: u32, value: f64, _: *const u8, _: u32) -> bool {
            // A simulated native setter that retained the old serial is a mismatch.
            id != 10 || value == 0.0
        }
        let mut out = ByteWriter::new();
        assert!(RegionalCity::new(2).write_record(&mut out));
        let baseline = out.bytes().to_vec();
        // SAFETY: local input and synchronous callback, no retained pointers.
        unsafe {
            let result = fc4_regionalcity_shadow_mutation_compare(
                baseline.as_ptr(),
                baseline.len() as u32,
                3,
                Some(check),
                std::ptr::null_mut(),
            );
            assert_eq!(result as u32, 1);
            assert!(result >> 32 > 60);
            assert_eq!(
                fc4_regionalcity_shadow_mutation_compare(
                    baseline.as_ptr(),
                    baseline.len() as u32,
                    4,
                    Some(check),
                    std::ptr::null_mut(),
                ),
                u64::MAX
            );
        }
        assert_eq!(baseline, out.bytes());
    }

    #[test]
    fn observation_skips_getters_that_would_insert_missing_map_entries() {
        let mut c = RegionalCity::new(2);
        c.commodities.clear();
        c.populations.insert(1, 17);
        c.extrapolated.insert(2, 23);
        c.potential.insert(3, 31);
        c.extrapolated.insert(4, 10);
        c.potential.insert(4, 40);
        let mut before = ByteWriter::new();
        assert!(c.write_record(&mut before));
        let mut allowable = Vec::new();
        let result = compare(&mut c, |id, t, value, _| {
            if id == 43 {
                allowable.push(t);
                assert_eq!(value, 30.0);
            }
            assert!(!(50..=55).contains(&id), "absent commodities must not be queried");
            true
        });
        assert_eq!(result as u32, 0);
        assert_eq!(allowable, [4]);
        let mut after = ByteWriter::new();
        assert!(c.write_record(&mut after));
        assert_eq!(before.bytes(), after.bytes());
    }

    #[test]
    fn points_compare_present_empty_and_absent_lists_without_changing_record() {
        let mut c = RegionalCity::new(2);
        c.points.insert(0, vec![[1.25, -0.0, f32::from_bits(0x7fc0_1234)]]);
        c.points.insert(7, vec![]);
        let mut before = ByteWriter::new();
        assert!(c.write_record(&mut before));
        let mut seen = Vec::new();
        let result = compare(&mut c, |id, t, _, bytes| {
            if id == 60 {
                seen.push(t);
                match t {
                    0 => assert_eq!(
                        bytes,
                        [0x3fa0_0000_u32, 0x8000_0000, 0x7fc0_1234]
                            .into_iter()
                            .flat_map(u32::to_le_bytes)
                            .collect::<Vec<_>>()
                    ),
                    7 => assert!(bytes.is_empty()),
                    1 => assert_eq!(
                        bytes,
                        [0x3f80_0000_u32, 0xc000_0000, 0x4040_0000]
                            .into_iter()
                            .flat_map(u32::to_le_bytes)
                            .collect::<Vec<_>>()
                    ),
                    _ => panic!("unexpected point type {t}"),
                }
            }
            if id == 100 {
                assert_eq!(bytes, before.bytes());
            }
            true
        });
        assert_eq!(result as u32, 0);
        assert_eq!(seen, [0, 7, 1]);
        assert_eq!(compare(&mut c, |id, _, _, _| id != 60) as u32, 3);
        let mut after = ByteWriter::new();
        assert!(c.write_record(&mut after));
        assert_eq!(before.bytes(), after.bytes());
    }

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
