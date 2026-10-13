// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests for the exemplar reader: the `docs/specs/data-formats.md` §10.4 / §10.5 conformance cases (numbered as
//! there), the exemplar rows of §11, and the FreeCity4 choices.

use super::*;
use crate::dbpf::{Compression, PackageBuilder};

/// Bytes from hexadecimal text; spaces and `|` are ignored.
fn hex(s: &str) -> Vec<u8> {
    let digits: Vec<u8> = s.bytes().filter(u8::is_ascii_hexdigit).collect();
    assert_eq!(digits.len() % 2, 0, "odd hex string {s:?}");
    digits
        .chunks(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect()
}

/// A binary record: the signature, the parent key (type, group, instance), the count, then the properties.
fn record(sig: &[u8; 8], parent: [u32; 3], props: &[&str]) -> Vec<u8> {
    let mut d = sig.to_vec();
    for w in parent {
        d.extend_from_slice(&w.to_le_bytes());
    }
    d.extend_from_slice(&(props.len() as u32).to_le_bytes());
    for p in props {
        d.extend(hex(p));
    }
    d
}

/// §10.4 header: `EQZB1###`, parent `00`×12, then the count and the properties.
fn rec(props: &[&str]) -> Vec<u8> {
    record(b"EQZB1###", [0; 3], props)
}

fn parse_ex(data: &[u8]) -> Exemplar {
    Exemplar::parse_record(TYPE_EXEMPLAR, data).unwrap()
}

/// §10.4 test 23's property: `0x10` = Uint32 7.
const T23: &str = "10 00 00 00 | 00 | 03 00 00 00 | 07 00 00 00";

/// A Uint32 scalar property in binary form.
fn u32_prop(id: u32, v: u32) -> String {
    let mut s = String::new();
    for b in id.to_le_bytes() {
        s += &format!("{b:02X} ");
    }
    s += "00 03 00 00 00 ";
    for b in v.to_le_bytes() {
        s += &format!("{b:02X} ");
    }
    s
}

/// A text record with an empty parent, the given PropCount line and property lines.
fn text_rec(parent: &str, count: &str, props: &[&str]) -> Vec<u8> {
    let mut s = format!("EQZT1###\r\n{parent}\r\n{count}\r\n");
    for p in props {
        s += p;
        s += "\r\n";
    }
    s.into_bytes()
}

const NO_PARENT: &str = "ParentCohort=Key:{0x00000000,0x00000000,0x00000000}";

fn cohort_key(instance: u32) -> Tgi {
    Tgi::new(TYPE_COHORT, 0x1000, instance)
}

fn exemplar_key(instance: u32) -> Tgi {
    Tgi::new(TYPE_EXEMPLAR, 0x1000, instance)
}

fn key_words(k: Tgi) -> [u32; 3] {
    [k.type_id, k.group, k.instance]
}

// ---------------------------------------------------------------------------------------------------------------
// §10.4 Exemplars

#[test]
fn t23_scalar_uint32() {
    let ex = parse_ex(&rec(&[T23]));
    assert_eq!(ex.kind, Kind::Exemplar);
    assert_eq!(ex.encoding, Encoding::Binary);
    assert!(!ex.has_parent());
    let p = ex.get(0x10).unwrap();
    assert_eq!(p.values, Values::Uint32(vec![7]));
    assert!(!p.is_array);
    assert_eq!(p.as_u32(), Some(7));
    assert_eq!(p.memory_type(), 0x06);
    assert_eq!(p.disk_tag(), Some(0x03));
    assert_eq!(p.count(), 0);
}

#[test]
fn t24_uint32_array() {
    let ex = parse_ex(&rec(&[
        "20 00 00 00 | 00 | 03 80 00 00 | 02 00 00 00 | 05 00 00 00 06 00 00 00",
    ]));
    let p = ex.get(0x20).unwrap();
    assert!(p.is_array);
    assert_eq!(p.u32s(), Some(&[5, 6][..]));
    assert_eq!(p.count(), 2);
    assert_eq!(p.memory_type(), 0x8006);
}

#[test]
fn t25_string() {
    let ex = parse_ex(&rec(&["20 00 00 00 | 00 | 0C 80 00 00 | 05 00 00 00 | 48 65 6C 6C 6F"]));
    let p = ex.get(0x20).unwrap();
    assert_eq!(p.as_str(), Some("Hello"));
    assert_eq!(p.count(), 5);
    // A string is an RZChar array (foundations §9.1).
    assert_eq!(p.memory_type(), 0x800E);
}

#[test]
fn t26_utf16_string() {
    let ex = parse_ex(&rec(&["20 00 00 00 | 00 | 0C 80 00 00 | 02 00 00 10 | 48 00 69 00"]));
    assert_eq!(ex.get(0x20).unwrap().as_str(), Some("Hi"));
}

#[test]
fn t27_scalar_0x0c_is_one_rzchar() {
    const T27: &str = "20 00 00 00 | 00 | 0C 00 00 00 | 48";
    let data = rec(&[T27]);
    // 24-byte header + 4 (ID) + 1 (kind) + 4 (tag) + 1 (the character): a string reader would want 4 more bytes.
    assert_eq!(data.len(), 34);
    let ex = parse_ex(&data);
    let p = ex.get(0x20).unwrap();
    assert_eq!(p.as_char(), Some(b'H'));
    assert_eq!(p.memory_type(), 0x0E);
    assert_eq!(p.count(), 0);
    assert_eq!(p.string_bytes(), None, "not a string");
    // The next property starts right after the one byte.
    let ex = parse_ex(&rec(&[T27, T23]));
    assert_eq!(ex.get(0x20).unwrap().as_char(), Some(b'H'));
    assert_eq!(ex.get(0x10).unwrap().as_u32(), Some(7));
}

#[test]
fn t28_unused_byte_and_key_high_byte_must_be_zero() {
    // The "unused" byte set: tag 0x01008003.
    let bad = rec(&["10 00 00 00 | 00 | 03 80 00 01 | 01 00 00 00 | 07 00 00 00"]);
    assert!(matches!(Exemplar::parse(&bad), Err(Error::Corrupt { .. })));
    // The key type's high byte set: tag 0x00018003.
    let bad = rec(&["10 00 00 00 | 00 | 03 80 01 00 | 01 00 00 00 | 07 00 00 00"]);
    assert!(matches!(Exemplar::parse(&bad), Err(Error::Corrupt { .. })));
    // Value type 0x0D00 (tag 0x0D) is not in the reader set.
    let bad = rec(&["10 00 00 00 | 00 | 0D 00 00 00 | 07"]);
    assert!(matches!(Exemplar::parse(&bad), Err(Error::Corrupt { .. })));
}

#[test]
fn t29_unknown_kind() {
    let bad = rec(&["10 00 00 00 | 02 | 03 00 00 00 | 07 00 00 00"]);
    assert!(matches!(Exemplar::parse(&bad), Err(Error::Corrupt { .. })));
}

/// §11 row "exemplar signature requires byte 4 = '1'" → only the first 4 bytes are checked (§4.1).
#[test]
fn t30_signature_and_type() {
    for sig in [b"EQZB2###", b"EQZBabcd"] {
        let ex = parse_ex(&record(sig, [0; 3], &[T23]));
        assert_eq!(
            ex.get(0x10).unwrap().as_u32(),
            Some(7),
            "{:?}",
            std::str::from_utf8(sig)
        );
    }
    assert!(matches!(
        Exemplar::parse_record(TYPE_EXEMPLAR, &record(b"EQZA1###", [0; 3], &[T23])),
        Err(Error::BadMagic { .. })
    ));
    // A cohort record with an exemplar signature (= an exemplar stored under the cohort type) fails.
    let as_exemplar = rec(&[T23]);
    assert!(matches!(
        Exemplar::parse_record(TYPE_COHORT, &as_exemplar),
        Err(Error::BadMagic { what: "cohort", .. })
    ));
    // ... and the reverse.
    let as_cohort = record(b"CQZB1###", [0; 3], &[T23]);
    assert!(matches!(
        Exemplar::parse_record(TYPE_EXEMPLAR, &as_cohort),
        Err(Error::BadMagic { what: "exemplar", .. })
    ));
    assert_eq!(
        Exemplar::parse_record(TYPE_COHORT, &as_cohort).unwrap().kind,
        Kind::Cohort
    );
    // Any other type fails.
    assert!(matches!(
        Exemplar::parse_record(0x1234_5678, &as_exemplar),
        Err(Error::Unsupported { .. })
    ));
    // The same rules through a package, and in the text form.
    let mut b = PackageBuilder::new();
    b.add(cohort_key(1), &as_exemplar, Compression::None).unwrap();
    b.add(exemplar_key(2), &as_cohort, Compression::None).unwrap();
    b.add(exemplar_key(3), &as_exemplar, Compression::None).unwrap();
    let pkg = Package::from_bytes(b.build().unwrap()).unwrap();
    let mut store = ExemplarStore::new();
    let rep = store.load_package(&pkg);
    assert_eq!((rep.exemplars, rep.cohorts, rep.failures.len()), (1, 0, 2));
    let text = text_rec(NO_PARENT, "PropCount=0", &[]);
    assert!(Exemplar::parse_record(TYPE_COHORT, &text).is_err());
    let mut text2 = text.clone();
    text2[..4].copy_from_slice(b"EQZT");
    text2[4..8].copy_from_slice(b"9xyz");
    assert_eq!(
        Exemplar::parse_record(TYPE_EXEMPLAR, &text2).unwrap().encoding,
        Encoding::Text
    );
    // Fewer than the 8 signature bytes fails.
    assert!(matches!(Exemplar::parse(b"EQZB1##"), Err(Error::Truncated { .. })));
}

/// §11 row "lookups return the last duplicate property" → confirmed: the last one wins, in place (§4.5).
#[test]
fn t31_duplicate_ids_last_wins() {
    let ex = parse_ex(&rec(&[
        "10 00 00 00 00 03 00 00 00 07 00 00 00",
        "10 00 00 00 00 03 00 00 00 09 00 00 00",
    ]));
    assert_eq!(ex.properties().len(), 1);
    assert_eq!(ex.get(0x10).unwrap().as_u32(), Some(9));
    // Also across types and in the text form.
    let ex = parse_ex(&rec(&[T23, "10 00 00 00 | 00 | 09 00 00 00 | 00 00 C0 40"]));
    assert_eq!(ex.get(0x10).unwrap().as_f32(), Some(6.0));
    let ex = parse_ex(&text_rec(
        NO_PARENT,
        "PropCount=2",
        &["0x10:{\"a\"}=Uint32:0:{7}", "0x10:{\"b\"}=Uint32:0:{9}"],
    ));
    assert_eq!(ex.properties().len(), 1);
    assert_eq!(ex.get(0x10).unwrap().as_u32(), Some(9));
}

/// §11 row "missing parent: chain resolution tolerant" → the exemplar fails to load (§4.2).
#[test]
fn t32_missing_parent_fails_the_exemplar() {
    let data = record(b"EQZB1###", [0x0534_2861, 1, 2], &[T23]);
    assert_eq!(&data[8..20], &hex("61 28 34 05 | 01 00 00 00 | 02 00 00 00")[..]);
    let ex = parse_ex(&data);
    assert_eq!(ex.parent, Tgi::new(TYPE_COHORT, 1, 2));
    let key = exemplar_key(7);
    let mut store = ExemplarStore::new();
    store.insert(key, ex);
    assert_eq!(
        store.load(&key),
        Err(LoadError::Parent {
            key,
            parent: Tgi::new(TYPE_COHORT, 1, 2),
            cause: Box::new(LoadError::NotFound(Tgi::new(TYPE_COHORT, 1, 2))),
        })
    );
    // Lookups behave as if it did not exist, own properties included.
    assert!(store.property(&key, 0x10).is_none());
    let all = store.load_all();
    assert!(all.exemplars.is_empty());
    assert_eq!(all.failures.len(), 1);
    // Once the cohort is there, it loads.
    store.insert(Tgi::new(TYPE_COHORT, 1, 2), Exemplar::new(Kind::Cohort));
    assert_eq!(store.property(&key, 0x10).unwrap().as_u32(), Some(7));
}

#[test]
fn t33_parent_of_exemplar_type_fails() {
    let parent = Tgi::new(TYPE_EXEMPLAR, 1, 2);
    let key = exemplar_key(7);
    let mut store = ExemplarStore::new();
    store.insert(parent, parse_ex(&rec(&[T23])));
    store.insert(key, parse_ex(&record(b"EQZB1###", key_words(parent), &[T23])));
    assert!(store.load(&parent).is_ok(), "the parent itself loads, as an exemplar");
    assert_eq!(
        store.load(&key),
        Err(LoadError::Parent {
            key,
            parent,
            cause: Box::new(LoadError::NotCohort(parent)),
        })
    );
    // A cohort-type key holding an exemplar record fails the factory's type check instead.
    let wrong = Tgi::new(TYPE_COHORT, 1, 3);
    store.insert(wrong, parse_ex(&rec(&[T23])));
    let child = exemplar_key(8);
    store.insert(child, parse_ex(&record(b"EQZB1###", key_words(wrong), &[])));
    assert!(matches!(
        store.load(&child),
        Err(LoadError::Parent { cause, .. }) if *cause == LoadError::WrongType(wrong)
    ));
}

#[test]
fn t34_inheritance() {
    let c = cohort_key(1);
    let e = exemplar_key(2);
    let mut store = ExemplarStore::new();
    store.insert(
        c,
        Exemplar::parse_record(
            TYPE_COHORT,
            &record(b"CQZB1###", [0; 3], &[&u32_prop(0x10, 1), &u32_prop(0x11, 2)]),
        )
        .unwrap(),
    );
    store.insert(
        e,
        parse_ex(&record(
            b"EQZB1###",
            key_words(c),
            &[&u32_prop(0x11, 5), &u32_prop(0x12, 6)],
        )),
    );
    let l = store.load(&e).unwrap();
    assert_eq!(l.kind(), Kind::Exemplar);
    assert_eq!(l.parent(), c);
    assert_eq!(l.get_u32(0x10), Some(1));
    assert_eq!(l.get_u32(0x11), Some(5));
    assert_eq!(l.get_u32(0x12), Some(6));
    assert!(!l.is_property_local(0x10));
    assert!(l.is_property_local(0x11));
    assert!(l.has_property(0x10));
    let ids: Vec<u32> = l.properties().iter().map(|p| p.id).collect();
    assert_eq!(ids, [0x10, 0x11, 0x12]);
    let own: Vec<u32> = l.own_properties().iter().map(|p| p.id).collect();
    assert_eq!(own, [0x11, 0x12]);
    // The store lookups agree.
    assert_eq!(store.property(&e, 0x10).unwrap().as_u32(), Some(1));
    assert_eq!(store.property(&e, 0x11).unwrap().as_u32(), Some(5));
}

#[test]
fn t35_three_levels() {
    let a = cohort_key(1);
    let b = cohort_key(2);
    let e = exemplar_key(3);
    let mut store = ExemplarStore::new();
    store.insert(
        a,
        Exemplar::parse_record(TYPE_COHORT, &record(b"CQZB1###", [0; 3], &[&u32_prop(0x10, 1)])).unwrap(),
    );
    store.insert(
        b,
        Exemplar::parse_record(TYPE_COHORT, &record(b"CQZB1###", key_words(a), &[&u32_prop(0x11, 2)])).unwrap(),
    );
    store.insert(e, parse_ex(&record(b"EQZB1###", key_words(b), &[&u32_prop(0x10, 3)])));
    let l = store.load(&e).unwrap();
    assert_eq!(l.get_u32(0x10), Some(3));
    assert_eq!(l.get_u32(0x11), Some(2));
    assert_eq!(store.load(&b).unwrap().get_u32(0x10), Some(1), "B inherits from A");
}

#[test]
fn t36_trailing_bytes_ignored() {
    let mut data = rec(&[T23]);
    data.extend_from_slice(&[0xFF, 0xFF]);
    assert_eq!(parse_ex(&data), parse_ex(&rec(&[T23])));
}

#[test]
fn t37_text() {
    let data = b"EQZT1###\r\nParentCohort=Key:{0x00000000,0x00000000,0x00000000}\r\nPropCount=0x00000003\r\n\
0x00000010:{\"A\"}=Uint32:0:{0x00000007}\r\n\
0x00000011:{\"B\"}=Float32:2:{1.5,2}\r\n\
0x00000012:{\"C\"}=String:0:{\"Hi\"}\r\n";
    let ex = parse_ex(data);
    assert_eq!(ex.encoding, Encoding::Text);
    assert!(!ex.has_parent());
    assert_eq!(ex.properties().len(), 3);
    let a = ex.get(0x10).unwrap();
    assert_eq!((a.as_u32(), a.is_array), (Some(7), false));
    let b = ex.get(0x11).unwrap();
    assert_eq!(b.f32s(), Some(&[1.5, 2.0][..]));
    assert_eq!(ex.get(0x12).unwrap().as_str(), Some("Hi"));
}

/// §11 row "text parent key order group, instance, type" → confirmed (§4.3).
#[test]
fn t38_text_variants() {
    let line = |s: &str| text_rec(NO_PARENT, "PropCount=1", &[s]);
    let one = |s: &str| parse_ex(&line(s)).get(0x10).cloned().unwrap();
    // PropCount=3 (decimal).
    let props3 = [
        "0x10:{\"a\"}=Uint32:0:{1}",
        "0x11:{\"b\"}=Uint32:0:{2}",
        "0x12:{\"c\"}=Uint32:0:{3}",
    ];
    assert_eq!(
        parse_ex(&text_rec(NO_PARENT, "PropCount=3", &props3))
            .properties()
            .len(),
        3
    );
    // PropCount=10 is ten (decimal), PropCount=1A is 26 (hexadecimal by rule N).
    let lines: Vec<String> = (0..26)
        .map(|i| format!("0x{:x}:{{\"p\"}}=Uint32:0:{{{i}}}", 0x100 + i))
        .collect();
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_eq!(
        parse_ex(&text_rec(NO_PARENT, "PropCount=10", &lines[..10]))
            .properties()
            .len(),
        10
    );
    assert_eq!(
        parse_ex(&text_rec(NO_PARENT, "PropCount=1A", &lines))
            .properties()
            .len(),
        26
    );
    assert!(parse_ex(&text_rec(NO_PARENT, "PropCount=1A", &lines))
        .get(0x119)
        .is_some());
    // Type names are case-sensitive.
    assert!(matches!(
        Exemplar::parse(&line("0x10:{\"a\"}=uint32:0:{7}")),
        Err(Error::Text { line: 4, .. })
    ));
    // reps 1 is a one-element array.
    let p = one("0x10:{\"a\"}=Uint32:1:{7}");
    assert!(p.is_array);
    assert_eq!(p.u32s(), Some(&[7][..]));
    assert_eq!(p.count(), 1);
    // 64-bit values are hexadecimal, with or without 0x.
    assert_eq!(one("0x10:{\"a\"}=Sint64:0:{1356}").as_i64(), Some(0x1356));
    assert_eq!(one("0x10:{\"a\"}=Uint64:0:{0x1356}").as_u64(), Some(0x1356));
    // Bool: true exactly when the lower-cased value begins with "true".
    assert_eq!(one("0x10:{\"a\"}=Bool:0:{TRUE}").as_bool(), Some(true));
    assert_eq!(one("0x10:{\"a\"}=Bool:0:{yes}").as_bool(), Some(false));
    assert_eq!(one("0x10:{\"a\"}=Bool:0:{1}").as_bool(), Some(false));
    // The parent key is group, instance, type.
    let data = text_rec(
        "ParentCohort=Key:{0x0A64ED55,0x00000001,0x05342861}",
        "PropCount=0",
        &[],
    );
    let ex = parse_ex(&data);
    let want = Tgi::new(0x0534_2861, 0x0A64_ED55, 1);
    assert_eq!(ex.parent, want);
    // ... and the load asks for that cohort.
    let key = exemplar_key(9);
    let mut store = ExemplarStore::new();
    store.insert(key, ex);
    assert!(matches!(store.load(&key), Err(LoadError::Parent { parent, .. }) if parent == want));
    store.insert(want, Exemplar::new(Kind::Cohort));
    assert_eq!(store.load(&key).unwrap().parent(), want);
}

/// §11 row "text: 2 bytes after the signature" → skipped unchecked (§4.3).
#[test]
fn t39_text_two_bytes_skipped_unchecked() {
    let err = Exemplar::parse(b"EQZT1###ParentCohort=Key:{0x00000000,0x00000000,0x00000000}\r\nPropCount=0\r\n");
    match err {
        Err(Error::Text { line: 2, detail }) => assert!(detail.contains("rentCohort"), "{detail}"),
        other => panic!("{other:?}"),
    }
    // Any 2 bytes are skipped: the line break is not checked.
    let ex = parse_ex(
        b"EQZT1###XYParentCohort=Key:{0x00000000,0x00000000,0x00000000}\nPropCount=1\n0x10:{\"a\"}=Uint8:0:{5}",
    );
    assert_eq!(ex.get(0x10).unwrap().as_u8(), Some(5));
}

// ---------------------------------------------------------------------------------------------------------------
// §10.5 Getters

#[test]
fn t40_get_property_uint32() {
    let u32_array = Property::array(1, Values::Uint32(vec![5, 6]));
    assert_eq!(u32_array.get_u32(), Some(5));
    assert_eq!(Property::array(1, Values::Float32(vec![2.75])).get_u32(), Some(2));
    assert_eq!(
        Property::single(1, Values::Float32(vec![-1.5])).get_u32(),
        Some(0xFFFF_FFFF)
    );
    assert_eq!(Property::single(1, Values::Bool(vec![true])).get_u32(), Some(1));
    assert_eq!(Property::array(1, Values::String(b"0x1F".to_vec())).get_u32(), Some(31));
    assert_eq!(Property::array(1, Values::String(b"12".to_vec())).get_u32(), Some(12));
    assert_eq!(Property::array(1, Values::Uint32(vec![])).get_u32(), Some(0));
}

#[test]
fn t41_float32_rounding() {
    let p = Property::single(1, Values::Uint32(vec![16_777_217]));
    assert_eq!(p.get_f32(), Some(16_777_216.0));
}

#[test]
fn t42_raw_accessors() {
    let scalar = Property::single(1, Values::Uint32(vec![7]));
    assert_eq!(scalar.as_u32(), Some(7));
    assert_eq!(scalar.count(), 0);
    let array = Property::array(1, Values::Uint32(vec![9]));
    assert_eq!(array.count(), 1);
    assert_eq!(array.u32s().unwrap()[0], 9);
    // [choice] §5.2: a type mismatch (scalar against array included) is a missing value, no reinterpretation.
    assert_eq!(array.as_u32(), None);
    assert_eq!(scalar.u32s(), None);
    assert_eq!(scalar.as_f32(), None);
    assert_eq!(scalar.as_i32(), None);
}

#[test]
fn t43_missing_property() {
    let key = exemplar_key(1);
    let mut store = ExemplarStore::new();
    store.insert(key, parse_ex(&rec(&[T23])));
    let l = store.load(&key).unwrap();
    let mut out = 0xDEAD_BEEF_u32;
    if let Some(v) = l.get_u32(0x99) {
        out = v;
    }
    assert_eq!(out, 0xDEAD_BEEF, "output unchanged");
    assert_eq!(l.get_string(0x99), None);
    assert_eq!(l.get_u32(0x10), Some(7));
}

// ---------------------------------------------------------------------------------------------------------------
// §11 rows and §4 / §5 details

/// §11 row "value type u16 + key type u16 + unused byte" → kind byte + u32 tag; every tag of the reader set.
#[test]
fn reader_tag_set() {
    let ok = |tag: u32| ValueType::from_disk_tag(tag).is_some();
    for tag in (0x00..=0x0C).chain(0x8000..=0x800C).chain([0x8010, 0x8011]) {
        assert!(ok(tag), "{tag:#06X} must be read");
    }
    for tag in [
        0x0D,
        0x0E,
        0x0F,
        0x10,
        0x11,
        0x12,
        0x800D,
        0x800E,
        0x800F,
        0x8012,
        0x0100,
        0x0001_0003,
        0x8000_0003,
    ] {
        assert!(!ok(tag), "{tag:#X} must fail");
    }
    // Byte-compatible with the usual description: value type 0x0300 + key type 0x0080 + unused 0.
    assert_eq!(ValueType::from_disk_tag(0x8003), Some((ValueType::Uint32, true)));
    // The value types the usual description does not list are valid.
    let ex = parse_ex(&rec(&[
        "01 00 00 00 | 00 | 04 00 00 00 | 01 02 03 04 05 06 07 08",
        "02 00 00 00 | 00 | 05 00 00 00 | FF",
        "03 00 00 00 | 00 | 06 00 00 00 | FE FF",
        "04 00 00 00 | 00 | 0A 00 00 00 | 00 00 00 00 00 00 F8 3F",
        "05 00 00 00 | 00 | 00 00 00 00",
        "06 00 00 00 | 00 | 00 80 00 00 | 03 00 00 00 | AA BB CC",
        "07 00 00 00 | 00 | 10 80 00 00 | 02 00 00 00 | 48 00 69 00",
        "08 00 00 00 | 00 | 11 80 00 00 | 02 00 00 00 | 48 69",
        "09 00 00 00 | 00 | 01 80 00 00 | 00 00 00 00",
        "0A 00 00 00 | 00 | 0B 00 00 00 | 02",
    ]));
    assert_eq!(ex.get(1).unwrap().as_u64(), Some(0x0807_0605_0403_0201));
    assert_eq!(ex.get(2).unwrap().as_i8(), Some(-1));
    assert_eq!(ex.get(3).unwrap().as_i16(), Some(-2));
    assert_eq!(ex.get(4).unwrap().as_f64(), Some(1.5));
    let empty = ex.get(5).unwrap();
    assert_eq!(
        (&empty.values, empty.memory_type(), empty.count()),
        (&Values::Empty, 0x00, 0)
    );
    let void = ex.get(6).unwrap();
    assert_eq!(
        (&void.values, void.memory_type()),
        (&Values::Void(vec![0xAA, 0xBB, 0xCC]), 0x8000)
    );
    let wide = ex.get(7).unwrap();
    assert_eq!(
        (&wide.values, wide.memory_type()),
        (&Values::UnicodeChar(vec![0x48, 0x69]), 0x800D)
    );
    let chars = ex.get(8).unwrap();
    assert_eq!(
        (&chars.values, chars.memory_type()),
        (&Values::Char(b"Hi".to_vec()), 0x800C)
    );
    assert_eq!(ex.get(9).unwrap().u8s(), Some(&[][..]), "arrays may have count 0");
    assert_eq!(ex.get(10).unwrap().as_bool(), Some(true), "any non-zero byte is true");
    // Scalar Char / RZUnicodeChar tags are written by the original but not read (foundations §9.2).
    assert!(Exemplar::parse(&rec(&["10 00 00 00 | 00 | 11 00 00 00 | 41"])).is_err());
    assert!(Exemplar::parse(&rec(&["10 00 00 00 | 00 | 10 00 00 00 | 41 00"])).is_err());
    // Arrays whose elements would run past the record fail.
    assert!(Exemplar::parse(&rec(&["10 00 00 00 | 00 | 03 80 00 00 | FF FF FF 0F"])).is_err());
}

/// §11 row "kind 1 = resource key" (§4.6): read, and the resource must load.
#[test]
fn kind1_resource_key() {
    const K1: &str = "10 00 00 00 | 01 | 78 56 34 12 | 01 00 00 00 | 02 00 00 00";
    let res = Tgi::new(0x1234_5678, 1, 2);
    let ex = parse_ex(&rec(&[K1]));
    let p = ex.get(0x10).unwrap();
    assert_eq!(p.resource_key(), Some(res));
    assert_eq!(p.memory_type(), 0x10);
    assert_eq!(p.get_u32(), None, "[choice] an object value does not convert");
    assert_eq!(ex.resource_keys(), &[(0x10, res)]);
    assert_eq!(Exemplar::parse(&ex.to_binary()).unwrap(), ex);

    let key = exemplar_key(1);
    let mut store = ExemplarStore::new();
    store.insert(key, ex.clone());
    assert_eq!(
        store.load(&key),
        Err(LoadError::Resource {
            key,
            property: 0x10,
            resource: res,
            cause: Box::new(LoadError::NotFound(res)),
        })
    );
    store.add_resource(res);
    assert!(store.load(&key).is_ok());

    // A kind-1 property later replaced by a value still had its resource loaded during the read.
    let replaced = parse_ex(&rec(&[K1, T23]));
    assert_eq!(replaced.get(0x10).unwrap().as_u32(), Some(7));
    let mut store2 = ExemplarStore::new();
    store2.insert(key, replaced);
    assert!(matches!(store2.load(&key), Err(LoadError::Resource { .. })));

    // A resource that is an exemplar record must itself load.
    let target = exemplar_key(2);
    let k1 = format!("10 00 00 00 | 01 | {}", hex_words(key_words(target)));
    let mut store3 = ExemplarStore::new();
    store3.insert(key, parse_ex(&rec(&[&k1])));
    store3.insert(target, parse_ex(&record(b"EQZB1###", key_words(cohort_key(99)), &[])));
    assert!(matches!(store3.load(&key), Err(LoadError::Resource { resource, .. }) if resource == target));
    store3.insert(cohort_key(99), Exemplar::new(Kind::Cohort));
    assert!(store3.load(&key).is_ok());
}

fn hex_words(w: [u32; 3]) -> String {
    w.iter()
        .flat_map(|x| x.to_le_bytes())
        .map(|b| format!("{b:02X} "))
        .collect()
}

/// §11 row "MAX_PARENT_DEPTH = 32, cycles end the walk" → kept as a [choice], treated as a load failure (§4.4).
#[test]
fn depth_limit_and_cycles_fail_the_load() {
    // A chain of 40 cohorts: instance 100 + i has parent 100 + i - 1; cohort 100 has none. Instances are numbered
    // so that the deepest records come first in key order, which load_all uses: a record first met deep inside
    // another's load must still load on its own.
    let mut store = ExemplarStore::new();
    for i in 0..40u32 {
        let mut c = Exemplar::new(Kind::Cohort);
        if i > 0 {
            c.parent = cohort_key(1000 - (i - 1));
        }
        c.set(Property::single(i, Values::Uint32(vec![i])));
        store.insert(cohort_key(1000 - i), c);
    }
    // 32 parents load; 33 do not.
    let l = store.load(&cohort_key(1000 - 32)).unwrap();
    assert_eq!(l.properties().len(), 33);
    assert_eq!(l.get_u32(0), Some(0));
    assert!(store.load(&cohort_key(1000 - 33)).unwrap_err().too_deep());
    let all = store.load_all();
    let loaded: Vec<u32> = all.exemplars.keys().map(|k| 1000 - k.instance).collect();
    assert_eq!(loaded, (0..=32).rev().collect::<Vec<_>>());
    let failed: Vec<u32> = all.failures.iter().map(|(k, _)| 1000 - k.instance).collect();
    assert_eq!(failed, (33..40).rev().collect::<Vec<_>>());
    assert_eq!(
        store.chain(store.get(&cohort_key(1000 - 39)).unwrap()).1,
        ChainEnd::TooDeep
    );

    // Cycles: a cohort that is its own parent, and two that are each other's.
    let mut store = ExemplarStore::new();
    let mut own = Exemplar::new(Kind::Cohort);
    own.parent = cohort_key(1);
    store.insert(cohort_key(1), own);
    let (a, b) = (cohort_key(2), cohort_key(3));
    let mut ca = Exemplar::new(Kind::Cohort);
    ca.parent = b;
    let mut cb = Exemplar::new(Kind::Cohort);
    cb.parent = a;
    store.insert(a, ca);
    store.insert(b, cb);
    let mut child = Exemplar::new(Kind::Exemplar);
    child.parent = a;
    store.insert(exemplar_key(4), child);
    for k in [cohort_key(1), a, b, exemplar_key(4)] {
        let mut e = store.load(&k).unwrap_err();
        while let LoadError::Parent { cause, .. } = e {
            e = *cause;
        }
        assert!(matches!(e, LoadError::Cycle(_)), "{k}: {e}");
    }
    assert_eq!(store.load_all().failures.len(), 4);
    assert!(matches!(store.chain(store.get(&a).unwrap()).1, ChainEnd::Cycle(_)));
}

/// §4.4: the effective list is a snapshot taken when the parent is set.
#[test]
fn effective_list_is_a_snapshot() {
    let c = cohort_key(1);
    let e = exemplar_key(2);
    let mut store = ExemplarStore::new();
    let mut cohort = Exemplar::new(Kind::Cohort);
    cohort.set(Property::single(0x10, Values::Uint32(vec![1])));
    store.insert(c, cohort);
    let mut ex = Exemplar::new(Kind::Exemplar);
    ex.parent = c;
    ex.set(Property::single(0x11, Values::Uint32(vec![2])));
    store.insert(e, ex);
    let mut loaded_c = store.load(&c).unwrap();
    let mut loaded_e = store.load(&e).unwrap();

    // Changing the cohort afterwards does not change the exemplar built from it.
    loaded_c.set(Property::single(0x10, Values::Uint32(vec![100])));
    loaded_c.set(Property::single(0x12, Values::Uint32(vec![3])));
    assert_eq!(loaded_e.get_u32(0x10), Some(1));
    assert!(!loaded_e.has_property(0x12));
    // Neither does replacing the record in the store.
    let mut c2 = Exemplar::new(Kind::Cohort);
    c2.set(Property::single(0x10, Values::Uint32(vec![50])));
    store.insert(c, c2);
    assert_eq!(loaded_e.get_u32(0x10), Some(1));
    assert_eq!(
        store.load(&e).unwrap().get_u32(0x10),
        Some(50),
        "a new load sees the new record"
    );

    // Adding updates both lists; setting the cohort again rebuilds the effective list.
    loaded_e.set(Property::single(0x13, Values::Uint32(vec![4])));
    assert!(loaded_e.is_property_local(0x13));
    assert_eq!(loaded_e.get_u32(0x13), Some(4));
    assert!(loaded_e.set_cohort(Some(&loaded_c)));
    assert_eq!(loaded_e.get_u32(0x10), Some(100));
    assert_eq!(loaded_e.get_u32(0x12), Some(3));
    let ids: Vec<u32> = loaded_e.properties().iter().map(|p| p.id).collect();
    assert_eq!(ids, [0x10, 0x11, 0x12, 0x13]);
    // Only a cohort can be set.
    let other = loaded_e.clone();
    assert!(!loaded_e.set_cohort(Some(&other)));
    assert!(loaded_e.set_cohort(None));
    assert_eq!(loaded_e.parent(), Tgi::default());
    assert_eq!(loaded_e.properties(), loaded_e.own_properties());
}

/// §4.4: both lists are sorted by ascending **unsigned** ID.
#[test]
fn lists_sorted_by_unsigned_id() {
    let ex = parse_ex(&rec(&[
        &u32_prop(0x8000_0001, 1),
        &u32_prop(0x10, 2),
        &u32_prop(0x7FFF_FFFF, 3),
    ]));
    let ids: Vec<u32> = ex.properties().iter().map(|p| p.id).collect();
    assert_eq!(ids, [0x10, 0x7FFF_FFFF, 0x8000_0001]);
    // The writer emits them in that order.
    let back = Exemplar::parse(&ex.to_binary()).unwrap();
    assert_eq!(back, ex);
}

#[test]
fn rule_n() {
    assert_eq!(super::rule_n(b"16"), 16);
    assert_eq!(super::rule_n(b"1A"), 26);
    assert_eq!(super::rule_n(b"0x10"), 16);
    assert_eq!(super::rule_n(b"0X10"), 16);
    assert_eq!(super::rule_n(b"-1"), 0xFFFF_FFFF);
    assert_eq!(super::rule_n(b"0x00000010:{\"Exemplar Type\"}"), 0x10);
    assert_eq!(
        super::rule_n(b"10:{\"x\"}"),
        10,
        "decimal: no letter before the first non-hex character"
    );
    assert_eq!(super::rule_n(b"ff"), 0xFF);
    assert_eq!(super::rule_n(b""), 0);
    assert_eq!(super::rule_n(b"0x123456789"), 0xFFFF_FFFF, "strtoul saturates");
}

#[test]
fn text_details() {
    let one = |s: &str| parse_ex(&text_rec(NO_PARENT, "PropCount=1", &[s]));
    // The ID is rule N on the text left of the first '='; the name is ignored.
    assert!(one("0x10:{\"x\"}=Uint32:0:{1}").get(0x10).is_some());
    assert!(one("16=Uint32:0:{1}").get(16).is_some());
    // A name containing '=' breaks the line [quirk].
    assert!(Exemplar::parse(&text_rec(NO_PARENT, "PropCount=1", &["0x10:{\"a=b\"}=Uint32:0:{1}"])).is_err());
    // Missing values read as 0; extra values are not read.
    let p = one("0x10:{\"x\"}=Uint32:3:{7}").get(0x10).cloned().unwrap();
    assert_eq!(p.u32s(), Some(&[7, 0, 0][..]));
    let p = one("0x10:{\"x\"}=Uint8:2:{1,2,3}").get(0x10).cloned().unwrap();
    assert_eq!(p.u8s(), Some(&[1, 2][..]));
    let p = one("0x10:{\"x\"}=Uint8:0:{}").get(0x10).cloned().unwrap();
    assert_eq!((p.as_u8(), p.is_array), (Some(0), false));
    // Integers are truncated to the type's width; leading spaces trimmed.
    let p = one("0x10:{\"x\"}=Sint8:3:{-1, 0x1FF,  200}")
        .get(0x10)
        .cloned()
        .unwrap();
    assert_eq!(p.i8s(), Some(&[-1, -1, -56][..]));
    let p = one("0x10:{\"x\"}=Sint32:2:{-5,1A}").get(0x10).cloned().unwrap();
    assert_eq!(p.i32s(), Some(&[-5, 26][..]));
    // Float32: decimal, 80 gives 80.0.
    let p = one("0x27812810:{\"Occupant Size\"}=Float32:3:{80,52.5,-1e2}")
        .get(0x2781_2810)
        .cloned()
        .unwrap();
    assert_eq!(p.f32s(), Some(&[80.0, 52.5, -100.0][..]));
    // String: up to the first '}', trimmed, one quote removed at each end; reps ignored.
    let p = one("0x20:{\"x\"}=String:7:{ \"Fire, Station {Small\" }")
        .get(0x20)
        .cloned()
        .unwrap();
    assert_eq!(p.as_str(), Some("Fire, Station {Small"));
    let p = one("0x20:{\"x\"}=String:0:{\"a}b\"}").get(0x20).cloned().unwrap();
    assert_eq!(p.as_str(), Some("a"), "a string cannot contain '}}' [quirk]");
    let p = one("0x20:{\"x\"}=String:0:{\"with: colons\"}")
        .get(0x20)
        .cloned()
        .unwrap();
    assert_eq!(p.as_str(), Some("with: colons"));
    // No Float64 in text; the values must start with '{'.
    assert!(Exemplar::parse(&text_rec(NO_PARENT, "PropCount=1", &["0x10:{\"x\"}=Float64:0:{1}"])).is_err());
    assert!(Exemplar::parse(&text_rec(NO_PARENT, "PropCount=1", &["0x10:{\"x\"}=Uint32:0:7"])).is_err());
    // Exactly PropCount property lines: fewer fails, lines after them are not read; empty lines are skipped.
    assert!(matches!(
        Exemplar::parse(&text_rec(NO_PARENT, "PropCount=2", &["0x10:{\"x\"}=Uint32:0:{1}"])),
        Err(Error::Text { .. })
    ));
    let ex = parse_ex(&text_rec(
        NO_PARENT,
        "PropCount=1",
        &["", "0x10:{\"x\"}=Uint32:0:{1}", "", "garbage"],
    ));
    assert_eq!(ex.properties().len(), 1);
    // ParentCohort and PropCount names are case-insensitive, the Key:{ prefix is case-sensitive.
    assert!(Exemplar::parse(&text_rec("parentcohort = Key:{0,0,0}", "propcount = 0", &[])).is_ok());
    assert!(Exemplar::parse(&text_rec("ParentCohort=key:{0,0,0}", "PropCount=0", &[])).is_err());
    assert!(Exemplar::parse(&text_rec("ParentCohort=Key:{0,0}", "PropCount=0", &[])).is_err());
    assert!(Exemplar::parse(&text_rec("PropCount=0", "ParentCohort=Key:{0,0,0}", &[])).is_err());
    // [choice] a reps count larger than the record is rejected rather than allocated.
    assert!(Exemplar::parse(&text_rec(
        NO_PARENT,
        "PropCount=1",
        &["0x10:{\"x\"}=Uint32:0xFFFFFFFF:{1}"]
    ))
    .is_err());
    // A text cohort.
    let mut c = text_rec(NO_PARENT, "PropCount=0", &[]);
    c[0] = b'C';
    assert_eq!(Exemplar::parse_record(TYPE_COHORT, &c).unwrap().kind, Kind::Cohort);
}

/// The first shipped text exemplar's opening lines (§4.3).
#[test]
fn text_shipped_style() {
    let data = b"EQZT1###\r\n\
ParentCohort=Key:{0xca386e22,0x0a64ed55,0x05342861}\r\n\
PropCount=0x00000003\r\n\
0x00000010:{\"Exemplar Type\"}=Uint32:0:{0x00000002}\r\n\
0x099afacd:{\"Bulldoze Cost\"}=Sint64:0:{0x0000000000001356}\r\n\
0x27812810:{\"Occupant Size\"}=Float32:3:{80,52,80}\r\n";
    let ex = parse_ex(data);
    assert_eq!(ex.parent, Tgi::new(0x0534_2861, 0xCA38_6E22, 0x0A64_ED55));
    assert_eq!(ex.get(0x10).unwrap().as_u32(), Some(2));
    assert_eq!(ex.get(0x099A_FACD).unwrap().as_i64(), Some(0x1356));
    assert_eq!(ex.get(0x2781_2810).unwrap().f32s(), Some(&[80.0, 52.0, 80.0][..]));
}

#[test]
fn converting_getters() {
    let s = |v: Values| Property::single(1, v);
    let a = |v: Values| Property::array(1, v);
    // Scalar → float: through float64; Uint32 ≥ 2³¹ unsigned.
    assert_eq!(s(Values::Uint32(vec![0xFFFF_FFFF])).get_f64(), Some(4_294_967_295.0));
    assert_eq!(s(Values::Sint32(vec![-3])).get_f32(), Some(-3.0));
    assert_eq!(s(Values::Float64(vec![0.1])).get_f32(), Some(0.1_f32));
    assert_eq!(s(Values::Float32(vec![0.1])).get_f64(), Some(f64::from(0.1_f32)));
    assert_eq!(s(Values::Uint32(vec![5])).get_f32(), Some(5.0));
    // Scalar → integer: through int64, floats truncated toward zero, then the low bits.
    assert_eq!(s(Values::Float32(vec![2.9])).get_u32(), Some(2));
    assert_eq!(s(Values::Sint32(vec![-1])).get_u32(), Some(0xFFFF_FFFF));
    assert_eq!(s(Values::Sint32(vec![-1])).get_u64(), Some(u64::MAX));
    assert_eq!(s(Values::Uint32(vec![0x1234_5678])).get_u8(), Some(0x78));
    assert_eq!(s(Values::Uint32(vec![0x1234_5678])).get_i16(), Some(0x5678));
    assert_eq!(s(Values::Uint8(vec![200])).get_i8(), Some(-56));
    assert_eq!(s(Values::Sint64(vec![-2])).get_u16(), Some(0xFFFE));
    assert_eq!(s(Values::Uint64(vec![u64::MAX])).get_i64(), Some(-1));
    assert_eq!(s(Values::String(vec![b'H'])).get_u32(), Some(72), "an RZChar scalar");
    // Bool = (int64 ≠ 0), after truncation.
    assert_eq!(s(Values::Float32(vec![0.5])).get_bool(), Some(false));
    assert_eq!(s(Values::Uint32(vec![2])).get_bool(), Some(true));
    assert_eq!(s(Values::Bool(vec![true])).get_f32(), Some(1.0));
    // Arrays and strings: text round trip of the first element.
    assert_eq!(a(Values::Float32(vec![2.75, 9.0])).get_f32(), Some(2.75));
    assert_eq!(
        a(Values::Float32(vec![0.1])).get_f32(),
        Some(0.1),
        "floats survive the text exactly"
    );
    assert_eq!(a(Values::Float64(vec![-2.5])).get_i32(), Some(-2));
    assert_eq!(a(Values::Sint32(vec![-7])).get_i64(), Some(-7));
    assert_eq!(a(Values::Bool(vec![true])).get_u32(), Some(1));
    assert_eq!(a(Values::Uint64(vec![u64::MAX])).get_u64(), Some(u64::MAX));
    assert_eq!(a(Values::String(b"true".to_vec())).get_u32(), Some(1));
    assert_eq!(a(Values::String(b"t".to_vec())).get_bool(), Some(true));
    assert_eq!(a(Values::String(b"false".to_vec())).get_u32(), Some(0));
    assert_eq!(a(Values::String(b"True".to_vec())).get_u32(), Some(0), "case-sensitive");
    assert_eq!(a(Values::String(b"2.5".to_vec())).get_u32(), Some(2));
    assert_eq!(a(Values::String(b"1e3".to_vec())).get_u32(), Some(1000));
    assert_eq!(a(Values::String(b"-3".to_vec())).get_u32(), Some(0xFFFF_FFFD));
    assert_eq!(a(Values::String(b"abc".to_vec())).get_u32(), Some(0));
    assert_eq!(a(Values::String(b"12".to_vec())).get_f32(), Some(12.0));
    assert_eq!(a(Values::String(b"0x1F".to_vec())).get_f64(), Some(31.0));
    assert_eq!(a(Values::Uint32(vec![])).get_f32(), Some(0.0));
    // The string getter: a string as is, anything else through its text.
    assert_eq!(
        a(Values::String(b"Hello".to_vec())).get_string().as_deref(),
        Some("Hello")
    );
    assert_eq!(s(Values::Uint32(vec![7])).get_string().as_deref(), Some("7"));
    assert_eq!(s(Values::Sint32(vec![-7])).get_string().as_deref(), Some("-7"));
    assert_eq!(s(Values::Bool(vec![true])).get_string().as_deref(), Some("1"));
    assert_eq!(a(Values::Float32(vec![2.75])).get_string().as_deref(), Some("2.75"));
    assert_eq!(
        a(Values::Float32(vec![0.1])).get_string().as_deref(),
        Some("0.100000001490116119384766")
    );
    assert_eq!(
        s(Values::Float64(vec![1e30])).get_string().as_deref(),
        Some("1.00000000000000001988462e+30")
    );
    assert_eq!(
        s(Values::Float64(vec![1e-5])).get_string().as_deref(),
        Some("1.00000000000000008180305e-05")
    );
    assert_eq!(s(Values::Float64(vec![100.0])).get_string().as_deref(), Some("100"));
    assert_eq!(a(Values::Uint32(vec![])).get_string().as_deref(), Some("0"));
    assert_eq!(
        a(Values::UnicodeChar(vec![0x48, 0x69])).get_string().as_deref(),
        Some("Hi")
    );
    // [choice] Empty and object values do not convert.
    assert_eq!(s(Values::Empty).get_u32(), None);
    assert_eq!(s(Values::Empty).get_string(), None);
    assert_eq!(Property::resource(1, Tgi::new(1, 2, 3)).get_f32(), None);
}

#[test]
fn binary_layout() {
    let mut ex = Exemplar::new(Kind::Cohort);
    ex.set(Property::array(0x20, Values::String(b"ab".to_vec())));
    ex.set(Property::single(0x10, Values::Uint32(vec![0x2A])));
    let b = ex.to_binary();
    let mut want = b"CQZB1###".to_vec();
    want.extend_from_slice(&[0; 12]);
    want.extend_from_slice(&[2, 0, 0, 0]);
    want.extend(hex("10 00 00 00 | 00 | 03 00 00 00 | 2A 00 00 00"));
    want.extend(hex("20 00 00 00 | 00 | 0C 80 00 00 | 02 00 00 00 | 61 62"));
    assert_eq!(b, want);
    let back = Exemplar::parse_record(TYPE_COHORT, &b).unwrap();
    assert_eq!(back.kind, Kind::Cohort);
    assert_eq!(back.get(0x10).unwrap().as_u32(), Some(0x2A));
    assert_eq!(back.get(0x20).unwrap().as_str(), Some("ab"));
    // A one-character RZChar scalar, Empty and a resource key.
    let mut ex = Exemplar::new(Kind::Exemplar);
    ex.set(Property::single(1, Values::String(vec![b'H'])));
    ex.set(Property::single(2, Values::Empty));
    ex.set(Property::resource(3, Tgi::new(0x1234_5678, 1, 2)));
    let mut want = record(b"EQZB1###", [0; 3], &[]);
    want[20] = 3;
    want.extend(hex("01 00 00 00 | 00 | 0C 00 00 00 | 48"));
    want.extend(hex("02 00 00 00 | 00 | 00 00 00 00"));
    want.extend(hex("03 00 00 00 | 01 | 78 56 34 12 01 00 00 00 02 00 00 00"));
    assert_eq!(ex.to_binary(), want);
}

fn sample() -> Exemplar {
    let mut ex = Exemplar::new(Kind::Exemplar);
    ex.parent = Tgi::new(TYPE_COHORT, 0x1234, 0x5678);
    for p in [
        Property::single(0x10, Values::Uint32(vec![2])),
        Property::array(0x20, Values::String(b"Test Exemplar".to_vec())),
        Property::single(0x21, Values::Uint8(vec![200])),
        Property::array(0x22, Values::Uint16(vec![1, 65535])),
        Property::array(0x23, Values::Sint32(vec![-5, 7, i32::MIN])),
        Property::single(0x24, Values::Sint64(vec![-1_234_567_890_123])),
        Property::array(0x25, Values::Float32(vec![1.5, -0.25, 1e-3, 1e-7])),
        Property::single(0x26, Values::Bool(vec![true])),
        Property::array(0x27, Values::Bool(vec![false, true])),
        Property::array(0x29, Values::Uint32(vec![7])),
        Property::array(0x2A, Values::Sint8(vec![-128, 127])),
        Property::single(0x2B, Values::Sint16(vec![-300])),
        Property::array(0x2C, Values::Uint64(vec![u64::MAX, 1])),
        Property::array(0x2D, Values::String(b"Caf\xE9 \"quoted\"".to_vec())),
    ] {
        ex.set(p);
    }
    ex
}

#[test]
fn binary_round_trip_and_typed_api() {
    let mut ex = sample();
    for p in [
        Property::array(0x28, Values::Uint32(vec![])),
        Property::single(0x30, Values::Float64(vec![-0.5])),
        Property::single(0x31, Values::Empty),
        Property::array(0x32, Values::Void(vec![1, 2])),
        Property::array(0x33, Values::Char(b"ab".to_vec())),
        Property::array(0x34, Values::UnicodeChar(vec![0x263A])),
        Property::single(0x35, Values::String(vec![b'Z'])),
    ] {
        ex.set(p);
    }
    let back = Exemplar::parse(&ex.to_binary()).unwrap();
    assert_eq!(back, ex);
    assert_eq!(back.get(0x10).unwrap().as_u32(), Some(2));
    assert_eq!(back.get(0x10).unwrap().as_f32(), None);
    assert_eq!(back.get(0x20).unwrap().as_str(), Some("Test Exemplar"));
    assert_eq!(back.get(0x21).unwrap().as_u8(), Some(200));
    assert_eq!(back.get(0x22).unwrap().u16s(), Some(&[1, 65535][..]));
    assert_eq!(back.get(0x23).unwrap().i32s().unwrap()[2], i32::MIN);
    assert_eq!(back.get(0x24).unwrap().as_i64(), Some(-1_234_567_890_123));
    assert_eq!(back.get(0x25).unwrap().f32s().unwrap().len(), 4);
    assert_eq!(back.get(0x26).unwrap().as_bool(), Some(true));
    assert_eq!(back.get(0x27).unwrap().bools(), Some(&[false, true][..]));
    assert!(back.get(0x28).unwrap().is_empty());
    assert!(back.get(0x29).unwrap().is_array);
    assert_eq!(back.get(0x23).unwrap().get_i32(), Some(-5));
    assert_eq!(back.get(0x25).unwrap().get_f32(), Some(1.5));
    assert_eq!(
        back.get(0x2D).unwrap().string_lossy().as_deref(),
        Some("Café \"quoted\"")
    );
    assert_eq!(back.get(0x34).unwrap().get_string().as_deref(), Some("\u{263A}"));
    assert_eq!(back.get(0x35).unwrap().as_char(), Some(b'Z'));
    // The text form cannot express some of these.
    assert!(matches!(ex.to_text(), Err(Error::Unsupported { .. })));
}

#[test]
fn text_round_trip() {
    let ex = sample();
    let text = ex.to_text().unwrap();
    let back = Exemplar::parse(&text).unwrap();
    assert_eq!(back.encoding, Encoding::Text);
    assert_eq!(back.parent, ex.parent);
    assert_eq!(back.properties(), ex.properties());
    // The writer refuses what the reader cannot read back.
    for p in [
        Property::array(1, Values::Uint32(vec![])),
        Property::single(1, Values::Float64(vec![1.0])),
        Property::single(1, Values::Empty),
        Property::single(1, Values::String(vec![b'x'])),
        Property::array(1, Values::String(b"a}b".to_vec())),
        Property::array(1, Values::String(b"a\nb".to_vec())),
        Property::single(1, Values::Float32(vec![f32::NAN])),
        Property::resource(1, Tgi::new(1, 2, 3)),
    ] {
        let mut e = Exemplar::new(Kind::Exemplar);
        e.set(p.clone());
        assert!(e.to_text().is_err(), "{p:?}");
    }
}

#[test]
fn bad_exemplars_are_errors() {
    assert!(matches!(Exemplar::parse(b"XQZB1###"), Err(Error::BadMagic { .. })));
    let good = sample().to_binary();
    for n in 0..good.len() {
        assert!(Exemplar::parse(&good[..n]).is_err(), "prefix {n}");
    }
    let mut huge = good.clone();
    huge[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Exemplar::parse(&huge).is_err());
    // Offset 28 is the first property's kind byte.
    let mut bad_kind = good.clone();
    bad_kind[28] = 0x0D;
    assert!(matches!(Exemplar::parse(&bad_kind), Err(Error::Corrupt { .. })));
    // Text cut before the last property line fails. (A cut inside the last line may still read, since missing
    // values are 0, and so may a cut inside the PropCount number, which then reads as a smaller count.)
    let text = sample().to_text().unwrap();
    let last_line = text[..text.len() - 2].iter().rposition(|&b| b == b'\n').unwrap();
    let count_at = text.windows(10).position(|w| w == b"PropCount=").unwrap() + 10;
    let count_end = count_at + text[count_at..].iter().position(|&b| b == b'\r').unwrap();
    for n in (0..last_line).filter(|n| !(count_at..=count_end).contains(n)) {
        assert!(Exemplar::parse(&text[..n]).is_err(), "text prefix {n}");
    }
    assert!(matches!(
        Exemplar::parse(b"EQZT1###\r\nParentCohort=Key:{1,2}\r\n"),
        Err(Error::Text { line: 2, .. })
    ));
}

#[test]
fn cohort_inheritance_through_a_package() {
    let grand = Tgi::new(TYPE_COHORT, 1, 1);
    let parent = Tgi::new(TYPE_COHORT, 1, 2);
    let child = Tgi::new(TYPE_EXEMPLAR, 1, 3);
    let mut g = Exemplar::new(Kind::Cohort);
    g.set(Property::single(0xA, Values::Uint32(vec![1])));
    g.set(Property::single(0xB, Values::Uint32(vec![1])));
    g.set(Property::single(0xC, Values::Uint32(vec![1])));
    let mut p = Exemplar::new(Kind::Cohort);
    p.parent = grand;
    p.set(Property::single(0xB, Values::Uint32(vec![2])));
    p.set(Property::single(0xC, Values::Uint32(vec![2])));
    let mut c = Exemplar::new(Kind::Exemplar);
    c.parent = parent;
    c.set(Property::single(0xC, Values::Uint32(vec![3])));

    let mut b = PackageBuilder::new();
    b.add(grand, &g.to_binary(), Compression::Auto).unwrap();
    b.add(parent, &p.to_text().unwrap(), Compression::Auto).unwrap();
    b.add(child, &c.to_binary(), Compression::Always).unwrap();
    b.add(Tgi::new(0x1234_5678, 0, 0), b"other", Compression::None).unwrap();
    let pkg = Package::from_bytes(b.build().unwrap()).unwrap();
    let mut store = ExemplarStore::new();
    let rep = store.load_package(&pkg);
    assert_eq!((rep.exemplars, rep.cohorts, rep.failures.len()), (1, 2, 0));
    assert_eq!(store.len(), 3);

    assert_eq!(store.property(&child, 0xA).unwrap().as_u32(), Some(1));
    assert_eq!(store.property(&child, 0xB).unwrap().as_u32(), Some(2));
    assert_eq!(store.property(&child, 0xC).unwrap().as_u32(), Some(3));
    assert!(store.property(&child, 0xD).is_none());
    let (chain, end) = store.chain(store.get(&child).unwrap());
    assert_eq!((chain.len(), end), (3, ChainEnd::Root));
    let all = store.resolved_properties(store.get(&child).unwrap());
    let summary: Vec<(u32, usize)> = all.iter().map(|(d, p)| (p.id, *d)).collect();
    assert_eq!(summary, vec![(0xA, 2), (0xB, 1), (0xC, 0)]);
    let set = store.load_all();
    assert_eq!((set.exemplars.len(), set.failures.len()), (3, 0));
    let l = &set.exemplars[&child];
    let values: Vec<(u32, Option<u32>)> = l.properties().iter().map(|p| (p.id, p.as_u32())).collect();
    assert_eq!(values, [(0xA, Some(1)), (0xB, Some(2)), (0xC, Some(3))]);

    // The package's other entries are registered as resources a kind-1 property may name.
    let mut k1 = Exemplar::new(Kind::Exemplar);
    k1.set(Property::resource(1, Tgi::new(0x1234_5678, 0, 0)));
    let k1 = Exemplar::parse(&k1.to_binary()).unwrap();
    store.insert(exemplar_key(50), k1);
    assert!(store.load(&exemplar_key(50)).is_ok());

    // A missing parent ends the diagnostic chain and fails the load.
    let mut orphan = Exemplar::new(Kind::Exemplar);
    orphan.parent = Tgi::new(TYPE_COHORT, 9, 9);
    assert_eq!(store.chain(&orphan).1, ChainEnd::Missing(orphan.parent));
    store.insert(exemplar_key(51), orphan);
    assert!(store.property(&exemplar_key(51), 0xA).is_none());
    // A parent key of the exemplar type ends the chain as "not a cohort".
    let mut wrong = Exemplar::new(Kind::Exemplar);
    wrong.parent = child;
    assert_eq!(store.chain(&wrong).1, ChainEnd::NotCohort(child));
}
