// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// Independent, language-neutral snapshots transcribed from spec §3. Kept within
/// this module because this lane's edit scope excludes the conformance crate.
#[test]
fn all_77_registry_vectors() {
    let properties = RenderProperties::new();
    let mut counts = [0; 3];
    for line in include_str!("registry-vectors.txt").lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 7, "{line}");
        let index: i32 = fields[1].parse().unwrap();
        match fields[0] {
            "bool" => {
                assert_eq!(index, counts[0]);
                counts[0] += 1;
                check_entry(&properties.bool_properties()[index as usize], &fields);
                assert_eq!(properties.get_bool(index), Some(fields[4].parse().unwrap()));
                assert_eq!(properties.bool_name(index), Some(fields[2]));
                assert_eq!(properties.find_bool(&fields[2].to_ascii_uppercase()), index);
                let winner = match index {
                    22 => 11,
                    28 => 9,
                    _ => index,
                };
                assert_eq!(properties.find_bool(&fields[3].to_ascii_uppercase()), winner);
            }
            "int" => {
                assert_eq!(index, counts[1]);
                counts[1] += 1;
                check_entry(&properties.int_properties()[index as usize], &fields);
                assert_eq!(properties.get_int(index), Some(fields[4].parse().unwrap()));
                assert_eq!(properties.int_name(index), Some(fields[2]));
                assert_eq!(properties.find_int(&fields[2].to_ascii_uppercase()), index);
                let winner = if index == 19 { 4 } else { index };
                assert_eq!(properties.find_int(&fields[3].to_ascii_uppercase()), winner);
            }
            "float" => {
                assert_eq!(index, counts[2]);
                counts[2] += 1;
                check_entry(&properties.float_properties()[index as usize], &fields);
                assert_eq!(properties.get_float(index), Some(fields[4].parse().unwrap()));
                assert_eq!(properties.float_name(index), Some(fields[2]));
                assert_eq!(properties.find_float(&fields[2].to_ascii_uppercase()), index);
                let winner = if index == 4 { 3 } else { index };
                assert_eq!(properties.find_float(&fields[3].to_ascii_uppercase()), winner);
            }
            kind => panic!("unrecognised vector type: {kind}"),
        }
    }
    assert_eq!(counts, [38, 24, 15]);
}

fn check_entry<T>(entry: &Property<T>, fields: &[&str])
where
    T: std::str::FromStr + std::fmt::Debug + PartialEq,
    T::Err: std::fmt::Debug,
{
    assert_eq!(entry.index(), fields[1].parse().unwrap());
    assert_eq!(entry.name(), Some(fields[2]));
    assert_eq!(entry.short_name(), Some(fields[3]));
    assert_eq!(entry.name_bytes(), fields[2].as_bytes());
    assert_eq!(entry.short_name_bytes(), fields[3].as_bytes());
    assert_eq!(entry.value(), &fields[4].parse::<T>().unwrap());
    assert_eq!(
        entry.extra_values(),
        &[fields[5].parse::<T>().unwrap(), fields[6].parse::<T>().unwrap()]
    );
}

#[test]
fn spec_section_7_conformance() {
    let mut p = RenderProperties::new();
    assert_eq!(p.num_bool_properties(), 38);
    assert_eq!(p.num_int_properties(), 24);
    assert_eq!(p.num_float_properties(), 15);
    assert_eq!(p.get_int(23), Some(1024));
    assert_eq!(p.get_float(2), Some(2.0));
    assert_eq!(p.get_bool(28), Some(false));
    assert_eq!(p.get_bool(9), Some(true));
    for query in ["RenderShadows", "rendershadows", "rs"] {
        assert_eq!(p.find_bool(query), 9);
    }
    assert_eq!(p.find_bool("kRenderShadows"), -1);
    assert_eq!(p.find_int("sd"), 4);
    assert_eq!(p.find_float("pd"), 3);
    assert_eq!(p.find_float("ParticleDamping"), 4);
    assert!(p.set_int(23, 4096));
    assert_eq!(p.get_int(23), Some(4096));
    assert_eq!(p.bool_name(37), Some("doautozoomandrotationchanges"));
}

#[test]
fn construction_and_clone_own_separate_tables_and_strings() {
    let original = RenderProperties::new();
    let mut copy = original.clone();
    let fresh = RenderProperties::default();
    assert_eq!(original, copy);
    assert_eq!(original, fresh);
    assert_ne!(original.bools.as_ptr(), copy.bools.as_ptr());
    assert_ne!(original.ints.as_ptr(), copy.ints.as_ptr());
    assert_ne!(original.floats.as_ptr(), copy.floats.as_ptr());
    assert_ne!(original.bools[0].name.as_ptr(), copy.bools[0].name.as_ptr());
    assert!(copy.set_bool(9, false));
    assert!(copy.set_int(23, -1));
    assert!(copy.set_float(6, 123.0));
    assert_eq!(original, fresh);
    // Dropping one object cannot invalidate another object's tables or name storage.
    drop(original);
    drop(copy);
    assert_eq!(fresh.bool_name(9), Some("rendershadows"));
    assert_eq!(fresh.get_int(23), Some(1024));
    assert_eq!(fresh.get_float(6), Some(0.0001));
    assert_eq!(RenderProperties::new(), fresh);
}

#[test]
fn every_setter_changes_only_its_own_value() {
    let baseline = RenderProperties::new();
    let mut p = baseline.clone();
    for i in 0..38 {
        let value = !baseline.get_bool(i).unwrap();
        assert!(p.set_bool(i, value));
        assert_eq!(p.get_bool(i), Some(value));
        p.bools[i as usize].value = baseline.bools[i as usize].value;
        assert_eq!(p, baseline);
    }
    for i in 0..24 {
        assert!(p.set_int(i, i32::MIN + i));
        assert_eq!(p.get_int(i), Some(i32::MIN + i));
        p.ints[i as usize].value = baseline.ints[i as usize].value;
        assert_eq!(p, baseline);
    }
    for i in 0..15 {
        assert!(p.set_float(i, -100.0 - i as f32));
        assert_eq!(p.get_float(i), Some(-100.0 - i as f32));
        p.floats[i as usize].value = baseline.floats[i as usize].value;
        assert_eq!(p, baseline);
    }
}

#[test]
fn ranges_are_metadata_and_floats_are_stored_without_arithmetic() {
    let mut p = RenderProperties::new();
    for i in 7..=11 {
        let metadata = *p.ints[i as usize].extra_values();
        for value in [i32::MIN, -1, 0, 6, i32::MAX] {
            assert!(p.set_int(i, value));
            assert_eq!(p.get_int(i), Some(value));
            assert_eq!(p.ints[i as usize].extra_values(), &metadata);
        }
    }
    for i in 0..3 {
        let metadata = *p.floats[i as usize].extra_values();
        // Core storage extension: raw bits, not a native NaN/x87 conformance claim.
        for bits in [0x8000_0000, 0xc000_0000, 0x7f80_0000, 0xff80_0000, 0x7fc1_2345] {
            assert!(p.set_float(i, f32::from_bits(bits)));
            assert_eq!(p.get_float(i).unwrap().to_bits(), bits);
            assert_eq!(p.floats[i as usize].extra_values(), &metadata);
        }
    }
}

#[test]
fn core_safety_only_invalid_indices_fail_without_mutation_rp4() {
    let mut p = RenderProperties::new();
    let baseline = p.clone();
    for index in [i32::MIN, -1, 38, 39, i32::MAX] {
        assert_eq!(p.get_bool(index), None);
        assert_eq!(p.bool_name(index), None);
        assert!(!p.set_bool(index, true));
    }
    for index in [i32::MIN, -1, 24, 25, i32::MAX] {
        assert_eq!(p.get_int(index), None);
        assert_eq!(p.int_name(index), None);
        assert!(!p.set_int(index, 99));
    }
    for index in [i32::MIN, -1, 15, 16, i32::MAX] {
        assert_eq!(p.get_float(index), None);
        assert_eq!(p.float_name(index), None);
        assert!(!p.set_float(index, 99.0));
    }
    assert_eq!(p, baseline);
}

#[test]
fn find_is_exact_typed_and_first_match_wins() {
    let p = RenderProperties::new();
    for query in [
        "",
        "rs*",
        " rendershadows",
        "rendershadows ",
        "render",
        "kRenderShadows",
    ] {
        assert_eq!(p.find_bool(query), -1);
    }
    assert_eq!(p.find_bool("uSs"), 11);
    assert_eq!(p.find_bool("UseScreenShake"), 22);
    assert_eq!(p.find_bool("RenderSnapshot"), 28);
    assert_eq!(p.find_int("SoundDetail"), 19);
    assert_eq!(p.find_float("ParticleDamping"), 4);
    assert_eq!(p.find_int("RenderShadows"), -1);
    assert_eq!(p.find_float("sd"), -1);
    // The match order is entry-first, rather than a full-name pass then an acronym pass.
    let mut table = p.bools.clone();
    table[0].short_name = "latername".into();
    table[1].name = "latername".into();
    assert_eq!(find(&table, b"LaterName", &CLocale), 0);
}

#[test]
fn naming_helper_handles_capital_runs() {
    assert_eq!(short_name(b"RenderUI", &CLocale), b"rui");
    assert_eq!(short_name(b"LODOffset", &CLocale), b"lodo");
    assert_eq!(short_name(b"ParticleLODOffset", &CLocale), b"plodo");
    assert_eq!(short_name(b"OpaqueUIAlphaValue", &CLocale), b"ouiav");
    assert_eq!(short_name(b"NoAddSigned", &CLocale), b"nas");
}

#[test]
fn c_locale_string_boundary_policy_rp5() {
    let p = RenderProperties::new();
    assert_eq!(p.find_bool("rendershadows\0suffix"), 9);
    for query in ["renderſhadows", "réndershadows"] {
        assert_eq!(p.find_bool(query), -1);
    }
}

#[test]
fn immutable_registry_is_send_and_sync_for_second_thread_getters() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<RenderProperties>();
    let p = std::sync::Arc::new(RenderProperties::new());
    let reader = std::sync::Arc::clone(&p);
    let result = std::thread::spawn(move || (reader.get_int(23), reader.find_bool("rs"), reader.get_float(6)));
    assert_eq!(p.get_bool(28), Some(false));
    assert_eq!(result.join().unwrap(), (Some(1024), 9, Some(0.0001)));
}
