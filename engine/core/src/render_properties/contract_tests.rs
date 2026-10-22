// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent public-contract discriminators; synthetic ctypes below test service
//! plumbing, not the behavior of a shipped language or a solution to RP6.
use super::*;
use std::sync::Mutex;

#[test]
fn nul_byte_queries_all_types_and_unchanged_caller_storage() {
    let p = RenderProperties::new();
    for (query, expected) in [
        (&b"RS\0ignored"[..], [9, -1, -1]),
        (&b"STCS\0ignored"[..], [-1, 23, -1]),
        (&b"PD\0ParticleDamping"[..], [-1, -1, 3]),
        (&b"\0RenderShadows"[..], [-1; 3]),
        (&b""[..], [-1; 3]),
        (&b"RS\0\xff\x80\xfe"[..], [9, -1, -1]),
        (&b"STCS\0\xff\x80\xfe"[..], [-1, 23, -1]),
        (&b"PD\0\xff\x80\xfe"[..], [-1, -1, 3]),
    ] {
        let snapshot = query.to_vec();
        assert_eq!(
            [p.find_bool(query), p.find_int(query), p.find_float(query)],
            expected
        );
        assert_eq!(query, snapshot);
    }
    assert_eq!(p.find_bool(b"rEnDeRsHaDoWs"), 9);
    assert_eq!(p.find_float(b"ParticleDamping"), 4);
    for query in [
        &b"RenderShadows "[..],
        &b" RenderShadows"[..],
        &b"RenderShadow*"[..],
        &b"kRenderShadows"[..],
    ] {
        assert_eq!([p.find_bool(query), p.find_int(query), p.find_float(query)], [-1; 3]);
    }
}

#[test]
fn all77_names_and_acronyms_ignore_bytes_after_nul() {
    let p = RenderProperties::new();
    // Expected indices/winners are independent spec vectors, not calculated from
    // the implementation registry's names or an original-owner Find result.
    for line in include_str!("registry-vectors.txt").lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split_whitespace().collect();
        let index: i32 = fields[1].parse().unwrap();
        let winner = match (fields[0], index) {
            ("bool", 22) => 11,
            ("bool", 28) => 9,
            ("int", 19) => 4,
            ("float", 4) => 3,
            _ => index,
        };
        for (spelling, expected) in [(fields[2], index), (fields[3], winner)] {
            let mut query = spelling.to_ascii_uppercase().into_bytes();
            query.extend_from_slice(b"\0\xffIGNORED");
            let found = match fields[0] {
                "bool" => p.find_bool(&query),
                "int" => p.find_int(&query),
                "float" => p.find_float(&query),
                _ => unreachable!(),
            };
            assert_eq!(found, expected, "{line}");
        }
    }
}

#[test]
fn endpoint_values_names_and_isolation_rp4() {
    let mut p = RenderProperties::new();
    let untouched = RenderProperties::new();
    for (index, name, default, value) in [
        (0, "forcesoftwaredriver", false, true),
        (37, "doautozoomandrotationchanges", true, false),
    ] {
        assert_eq!(p.bool_name(index), Some(name));
        assert_eq!(p.bool_name_bytes(index), Some(name.as_bytes()));
        assert_eq!(p.get_bool(index), Some(default));
        let other = p.get_bool(37 - index);
        assert!(p.set_bool(index, value));
        assert_eq!(p.get_bool(index), Some(value));
        assert_eq!(p.get_bool(37 - index), other);
        assert_eq!(untouched.get_bool(index), Some(default));
    }
    for (index, name, default, value) in [
        (0, "viewoffsety", 0, i32::MIN),
        (23, "stringtexturecachesize", 1024, i32::MAX),
    ] {
        assert_eq!(p.int_name(index), Some(name));
        assert_eq!(p.int_name_bytes(index), Some(name.as_bytes()));
        assert_eq!(p.get_int(index), Some(default));
        let other = p.get_int(23 - index);
        assert!(p.set_int(index, value));
        assert_eq!(p.get_int(index), Some(value));
        assert_eq!(p.get_int(23 - index), other);
        assert_eq!(untouched.get_int(index), Some(default));
    }
    for (index, name, default_bits, value) in [
        (0, "particleeffectslod", 0x40a0_0000, -2.0),
        (14, "opaqueuialphavalue", 0x3f0c_cccd, 0.5),
    ] {
        assert_eq!(p.float_name(index), Some(name));
        assert_eq!(p.float_name_bytes(index), Some(name.as_bytes()));
        assert_eq!(p.get_float(index).unwrap().to_bits(), default_bits);
        let other = p.get_float(14 - index);
        assert!(p.set_float(index, value));
        assert_eq!(p.get_float(index), Some(value));
        assert_eq!(p.get_float(14 - index), other);
        assert_eq!(untouched.get_float(index).unwrap().to_bits(), default_bits);
    }
    assert!(p.set_int(7, -1));
    assert_eq!(p.get_int(7), Some(-1));
    assert!(p.set_float(0, -2.0));
    assert_eq!(p.get_float(0), Some(-2.0));
}

struct RecordingCtype(Mutex<Vec<i32>>);

impl ByteCtype for RecordingCtype {
    fn tolower(&self, ch: i32) -> i32 {
        self.0.lock().unwrap().push(ch);
        // Synthetic mapping deliberately consumes signed high bytes. High return
        // bits prove that the owner stores only the callback's low byte.
        match ch {
            -128 => 0x172, // r
            -1 => 0x273,   // s
            _ => CLocale.tolower(ch),
        }
    }

    fn isupper(&self, ch: i32) -> bool {
        CLocale.isupper(ch)
    }
}

#[test]
fn signed_non_utf8_bytes_use_supplied_lowering_and_stop_before_nul() {
    let p = RenderProperties::new();
    let service = RecordingCtype(Mutex::new(Vec::new()));
    assert_eq!(p.find_bool_with_ctype(b"\x80\xff\0X\xfe", &service), 9);
    assert_eq!(*service.0.lock().unwrap(), [-128, -1]);
    // No unsigned promotion, UTF-8 validation, suffix folding or caller mutation.
    assert_eq!(p.find_bool(b"\x80\xff"), -1); // explicitly C locale only
    for ch in 0x80..=0xff {
        assert_eq!(CLocale.tolower(signed(ch)) as u8, ch);
    }
}

struct ConstructionProfile;

impl ByteCtype for ConstructionProfile {
    fn tolower(&self, ch: i32) -> i32 {
        if ch == i32::from(b'I') {
            0xfd // synthetic non-UTF-8 construction-time name
        } else {
            CLocale.tolower(ch)
        }
    }

    fn isupper(&self, ch: i32) -> bool {
        // Synthetic classifier intentionally excludes I and includes lowercase e.
        ch != i32::from(b'I') && (ch == i32::from(b'e') || CLocale.isupper(ch))
    }
}

#[test]
fn construction_uses_foreign_classification_and_keeps_names_across_locale_change() {
    let p = RenderProperties::with_ctype(&ConstructionProfile);
    assert_eq!(p.bool_name_bytes(23), Some(&b"renderu\xfd"[..]));
    assert_eq!(p.bool_name(23), None); // UTF-8 convenience view only
    assert_eq!(p.bools[23].short_name_bytes(), b"reeu");
    assert_eq!(p.find_bool_with_ctype(b"RenderUI", &ConstructionProfile), 23);
    assert_eq!(p.find_bool_with_ctype(b"RenderUI", &CLocale), -1);
    assert_eq!(p.find_bool_with_ctype(b"renderu\xfd", &CLocale), 23);
    assert_eq!(p.find_bool_with_ctype(b"reeu", &CLocale), 23);
    // Query-time locale changes do not rebuild or refold stored full/short names.
    assert_eq!(p.bool_name_bytes(23), Some(&b"renderu\xfd"[..]));
}
