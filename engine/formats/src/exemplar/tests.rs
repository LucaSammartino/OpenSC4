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

#[test]
fn ordered_append_then_unsorted_duplicate_keeps_unsigned_order_and_last_value() {
    let props: Vec<_> = [0, 1, 0x8000_0000, u32::MAX, 1, 2, 0x8000_0000]
        .into_iter()
        .enumerate()
        .map(|(n, id)| u32_prop(id, n as u32))
        .collect();
    let refs: Vec<_> = props.iter().map(String::as_str).collect();
    let ex = parse_ex(&rec(&refs));
    assert_eq!(
        ex.properties().iter().map(|p| p.id).collect::<Vec<_>>(),
        [0, 1, 2, 0x8000_0000, u32::MAX]
    );
    assert_eq!(ex.get(1).unwrap().as_u32(), Some(4));
    assert_eq!(ex.get(0x8000_0000).unwrap().as_u32(), Some(6));
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

/// §5.3 `SC4GetExemplarProperty(instance, propertyID)`: exemplar (`0x6534284A`, group 0, instance).
#[test]
fn sc4_get_exemplar_property() {
    let mut store = ExemplarStore::new();
    store.insert(Tgi::new(TYPE_EXEMPLAR, 0, 0x55), parse_ex(&rec(&[T23])));
    store.insert(Tgi::new(TYPE_EXEMPLAR, 1, 0x66), parse_ex(&rec(&[T23])));
    assert_eq!(store.exemplar_property(0x55, 0x10).unwrap().as_u32(), Some(7));
    assert!(store.exemplar_property(0x55, 0x11).is_none(), "missing property");
    assert!(store.exemplar_property(0x66, 0x10).is_none(), "group 0 only");
    store.insert(
        Tgi::new(TYPE_EXEMPLAR, 0, 0x77),
        parse_ex(&record(b"EQZB1###", key_words(cohort_key(5)), &[T23])),
    );
    assert!(
        store.exemplar_property(0x77, 0x10).is_none(),
        "the exemplar does not load"
    );
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
    // EX16 (Plugin_002_TheLivingMall.dat): `Uint8:0:{}` is a scalar 0, GetCount() 0 like an empty array.
    let p = one("0x10:{\"x\"}=Uint8:0:{}").get(0x10).cloned().unwrap();
    assert_eq!((p.as_u8(), p.is_array, p.count()), (Some(0), false, 0));
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
    assert!(Exemplar::parse(&text_rec("ParentCohort=Key:{0,0,0", "PropCount=0", &[])).is_err());
    assert!(Exemplar::parse(&text_rec("PropCount=0", "ParentCohort=Key:{0,0,0}", &[])).is_err());
    // [choice] EX12: no cap in the original (it crashes); above 16 MiB of values the read fails.
    assert!(Exemplar::parse(&text_rec(
        NO_PARENT,
        "PropCount=1",
        &["0x10:{\"x\"}=Uint32:0xFFFFFFFF:{1}"]
    ))
    .is_err());
    let cap = MAX_TEXT_VALUE_BYTES / 4;
    let p = one(&format!("0x10:{{\"x\"}}=Uint32:{cap}:{{1}}"))
        .get(0x10)
        .cloned()
        .unwrap();
    assert_eq!((p.len(), p.u32s().unwrap()[0]), (cap as usize, 1));
    assert!(Exemplar::parse(&text_rec(
        NO_PARENT,
        "PropCount=1",
        &[&format!("0x10:{{\"x\"}}=Uint32:{}:{{1}}", cap + 1)]
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
    let st = |t: &str| Property::array(1, Values::String(t.as_bytes().to_vec()));
    // The text the string getter writes, and what it returns.
    let text = |p: Property| {
        let mut out = b"unchanged".to_vec();
        let ok = p.get_string_into(&mut out);
        (decode_text(&out), ok)
    };
    // Scalar → float64 (EX2): signed types and a Uint32 below 2³¹ exactly; a Uint32 from 2³¹ and every Uint64 keep
    // 24 significant bits, for a Float64 target too (§5.1 worked examples).
    assert_eq!(s(Values::Uint64(vec![16_777_217])).get_f64(), Some(16_777_216.0));
    assert_eq!(s(Values::Sint64(vec![16_777_217])).get_f64(), Some(16_777_217.0));
    assert_eq!(s(Values::Uint32(vec![16_777_217])).get_f64(), Some(16_777_217.0));
    assert_eq!(s(Values::Uint32(vec![0x8000_0001])).get_f64(), Some(2_147_483_648.0));
    assert_eq!(s(Values::Uint32(vec![0xFFFF_FFFF])).get_f64(), Some(4_294_967_296.0));
    assert_eq!(
        s(Values::Uint64(vec![u64::MAX])).get_f64(),
        Some(18_446_744_073_709_551_616.0)
    );
    assert_eq!(
        s(Values::Sint64(vec![(1 << 53) + 1])).get_f64(),
        Some(9_007_199_254_740_992.0),
        "Sint64 rounded to 53 bits"
    );
    assert_eq!(s(Values::Sint32(vec![-3])).get_f32(), Some(-3.0));
    assert_eq!(s(Values::Float64(vec![0.1])).get_f32(), Some(0.1_f32));
    assert_eq!(s(Values::Float32(vec![0.1])).get_f64(), Some(f64::from(0.1_f32)));
    assert_eq!(s(Values::Uint32(vec![5])).get_f32(), Some(5.0));
    // Scalar → integer (EX3): through int64, floats truncated toward zero by _ftol, then the low bits (wrapping).
    assert_eq!(s(Values::Float32(vec![2.9])).get_u32(), Some(2));
    assert_eq!(s(Values::Float32(vec![-1.5])).get_u32(), Some(0xFFFF_FFFF));
    assert_eq!(s(Values::Float32(vec![300.7])).get_u8(), Some(44));
    assert_eq!(s(Values::Float32(vec![3e9])).get_i32(), Some(-1_294_967_296));
    assert_eq!(s(Values::Float64(vec![5e9])).get_u64(), Some(5_000_000_000));
    for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e19, -1e19] {
        let p = s(Values::Float64(vec![x]));
        assert_eq!(p.get_i64(), Some(i64::MIN), "{x}");
        assert_eq!(p.get_u64(), Some(0x8000_0000_0000_0000), "{x}");
        assert_eq!((p.get_u32(), p.get_i8()), (Some(0), Some(0)), "{x}");
        assert_eq!(p.get_bool(), Some(true), "{x}");
    }
    assert_eq!(s(Values::Float32(vec![f32::NAN])).get_u32(), Some(0));
    assert_eq!(s(Values::Sint32(vec![-1])).get_u32(), Some(0xFFFF_FFFF));
    assert_eq!(s(Values::Sint32(vec![-1])).get_u64(), Some(u64::MAX));
    assert_eq!(s(Values::Uint32(vec![0x1234_5678])).get_u8(), Some(0x78));
    assert_eq!(s(Values::Uint32(vec![0x1234_5678])).get_i16(), Some(0x5678));
    assert_eq!(s(Values::Uint8(vec![200])).get_i8(), Some(-56));
    assert_eq!(s(Values::Sint64(vec![-2])).get_u16(), Some(0xFFFE));
    assert_eq!(s(Values::Uint64(vec![u64::MAX])).get_i64(), Some(-1));
    assert_eq!(s(Values::String(vec![b'H'])).get_u32(), Some(72), "an RZChar scalar");
    // An RZChar scalar widens as a signed byte (EX4).
    let e9 = s(Values::String(vec![0xE9]));
    assert_eq!(e9.get_u32(), Some(0xFFFF_FFE9));
    assert_eq!(e9.get_i16(), Some(-23));
    assert_eq!(e9.get_f32(), Some(-23.0));
    assert_eq!(e9.get_f64(), Some(-23.0));
    assert_eq!(e9.get_u8(), Some(0xE9));
    assert_eq!(e9.get_char(), Some(0xE9));
    assert_eq!(e9.get_unicode_char(), Some(0xFFE9));
    assert_eq!(e9.get_bool(), Some(true));
    // Bool = (int64 ≠ 0), after truncation.
    assert_eq!(s(Values::Float32(vec![0.5])).get_bool(), Some(false));
    assert_eq!(s(Values::Uint32(vec![2])).get_bool(), Some(true));
    assert_eq!(s(Values::Bool(vec![true])).get_f32(), Some(1.0));
    // Arrays and strings: the text round trip (EX6–EX9). Short binary fractions survive it, other floats need not.
    assert_eq!(a(Values::Float32(vec![2.75, 9.0])).get_f32(), Some(2.75));
    assert_eq!(a(Values::Float32(vec![0.1])).get_f32(), Some(0.1));
    assert_eq!(0.05_f32.to_bits(), 0x3D4C_CCCD);
    assert_eq!(
        a(Values::Float32(vec![0.05])).get_f32().map(f32::to_bits),
        Some(0x3D4C_CCCC)
    );
    assert_eq!(a(Values::Float64(vec![-2.5])).get_i32(), Some(-2));
    assert_eq!(a(Values::Sint32(vec![-7])).get_i64(), Some(-7));
    assert_eq!(a(Values::Bool(vec![true])).get_u32(), Some(1));
    assert_eq!(a(Values::Uint64(vec![u64::MAX])).get_u64(), Some(u64::MAX));
    assert_eq!(a(Values::Uint32(vec![])).get_f32(), Some(0.0));
    // Parsing (EX9): whole words case-sensitive, lower-case 0x only, the scan's flags (§5.1 worked examples).
    for (t, want) in [
        ("0x1F", 31),
        ("0X1F", 0),
        ("1A", 1),
        ("true", 1),
        ("t", 1),
        ("TRUE", 0),
        ("True", 0),
        ("false", 0),
        ("-5", 0xFFFF_FFFB),
        ("-3", 0xFFFF_FFFD),
        ("1-2", 1),
        ("e5", 0),
        ("1e3", 1000),
        (".5", 0),
        ("1.2.3", 1),
        ("12 apples", 12),
        ("2.5", 2),
        ("abc", 0),
        (" 7", 7),
        ("+7", 7),
        (" 0x10", 0),
        ("", 0),
    ] {
        assert_eq!(st(t).get_u32(), Some(want), "{t:?}");
    }
    assert_eq!(st("0x123456789ABCDEF012").get_u64(), Some(0x1234_5678_9ABC_DEF0));
    assert_eq!(st("18446744073709551617").get_u64(), Some(1), "64-bit wraparound");
    assert_eq!(st("t").get_bool(), Some(true));
    assert_eq!(st("0.5").get_bool(), Some(true), "Bool from text is value ≠ 0");
    assert_eq!(st("1.#QNAN").get_bool(), Some(true));
    assert_eq!(st(".5").get_f32(), Some(0.5));
    assert_eq!(st("12").get_f32(), Some(12.0));
    assert_eq!(st("0x1F").get_f64(), Some(31.0));
    // Unsigned text takes the 24-bit path for a float target, signed text is converted exactly.
    assert_eq!(st("16777217").get_f64(), Some(16_777_216.0));
    assert_eq!(st("-16777217").get_f64(), Some(-16_777_217.0));
    assert_eq!(st("-16777217").get_f32(), Some(-16_777_216.0));
    // EX25: signed text into a Float32 rounds once from the exact value; a Sint64 scalar rounds twice (to the double
    // first). −(2⁵⁴ + 2³⁰ + 1) gives −(2⁵⁴ + 2³¹) from text and −2⁵⁴ from the scalar.
    assert_eq!(st("-18014399583223809").get_f32(), Some(-18_014_400_656_965_632.0));
    assert_eq!(
        s(Values::Sint64(vec![-18_014_399_583_223_809])).get_f32(),
        Some(-18_014_398_509_481_984.0)
    );
    // Char targets always get 0 through the text, and succeed.
    assert_eq!(st("A").get_char(), Some(0));
    assert_eq!(st("65").get_unicode_char(), Some(0));
    assert_eq!(a(Values::Uint8(vec![65])).get_char(), Some(0));
    // Array texts (EX6): a Char array all its characters, an RZUnicodeChar array units below 0x80 (others `?`), a
    // void array nothing.
    assert_eq!(a(Values::Char(b"12".to_vec())).get_u32(), Some(12));
    assert_eq!(a(Values::Void(b"12".to_vec())).get_u32(), Some(0));
    assert_eq!(a(Values::UnicodeChar(vec![0xE9, 0x35])).get_u32(), Some(0));
    assert_eq!(a(Values::UnicodeChar(vec![0x31, 0x35])).get_u32(), Some(15));
    // The string getter (EX5, EX7, EX8): true only for a string; every other type writes its text and returns false.
    assert_eq!(text(st("Hello")), ("Hello".into(), true));
    assert_eq!(st("Hello").get_string().as_deref(), Some("Hello"));
    assert_eq!(text(s(Values::Uint32(vec![7]))), ("7".into(), false));
    assert_eq!(s(Values::Uint32(vec![7])).get_string(), None);
    assert_eq!(text(s(Values::Sint32(vec![-7]))), ("-7".into(), false));
    assert_eq!(text(s(Values::Bool(vec![true]))), ("1".into(), false));
    assert_eq!(text(s(Values::String(vec![b'A']))), ("A".into(), false));
    assert_eq!(text(s(Values::String(vec![0]))), ("\0".into(), false));
    assert_eq!(text(a(Values::Uint32(vec![]))), ("0".into(), false));
    assert_eq!(text(a(Values::Uint32(vec![3, 4]))), ("3".into(), false));
    assert_eq!(text(a(Values::Char(b"Hi".to_vec()))), ("Hi".into(), false));
    assert_eq!(text(a(Values::UnicodeChar(vec![0x48, 0x69]))), ("Hi".into(), false));
    assert_eq!(text(a(Values::UnicodeChar(vec![0xE9, 0x35]))), ("?5".into(), false));
    assert_eq!(text(a(Values::Void(vec![0x31, 0x32]))), (String::new(), false));
    // Floats: fixed-point, the first 17 significant digits rounded half-up, padded with zeros, trailing fraction
    // zeros dropped, cut at 63 characters (EX8 as corrected by EX29; the §5.1 examples).
    let float_text = |x: f64| text(s(Values::Float64(vec![x]))).0;
    let f32_text = |x: f32| text(a(Values::Float32(vec![x]))).0;
    assert_eq!(f32_text(2.75), "2.75");
    assert_eq!(f32_text(0.1), "0.10000000149011612");
    assert_eq!(f32_text(0.001), "0.0010000000474974513");
    assert_eq!(f32_text(0.05), "0.05000000074505806");
    assert_eq!(f32_text(1e30), "1000000015047466200000000000000");
    assert_eq!(f32_text(f32::from_bits(0x4049_0FD0)), "3.1415901184082031");
    // 2^-25 is exactly 0.0000000298023223876953125: a tie at the 18th digit rounds up (half-up, not to even).
    assert_eq!(f32_text(2f32.powi(-25)), "0.000000029802322387695313");
    assert_eq!(f32_text(16_777_216.0), "16777216");
    // Expected texts below were computed independently from the exact values (Python `decimal`, ROUND_HALF_UP).
    assert_eq!(float_text(1e30), "1000000000000000000000000000000");
    assert_eq!(float_text(1e-5), "0.000010000000000000001");
    assert_eq!(float_text(1e23), "99999999999999992000000");
    assert_eq!(float_text(100.0), "100");
    assert_eq!(float_text(0.5), "0.5");
    assert_eq!(float_text(-2.5), "-2.5");
    assert_eq!(float_text(1.0 / 3.0), "0.33333333333333331");
    assert_eq!(float_text(2.0 / 3.0), "0.66666666666666663");
    assert_eq!(float_text(1.0 - 2f64.powi(-53)), "0.99999999999999989");
    assert_eq!(float_text(2f64.powi(-65)), "0.000000000000000000027105054312137611");
    assert_eq!(f32_text(f32::from_bits(0x47F1_206D)), "123456.8515625");
    // −0.0 prints `0`: the runtime drops a zero's sign (EX24); a Float32 too.
    assert_eq!(float_text(-0.0), "0");
    assert_eq!(f32_text(-0.0), "0");
    assert_eq!(f32_text(0.0), "0");
    // Below 1e-64 nothing is left of the fraction (the printer still writes `-` for a negative value); a long
    // text is cut at 63 characters.
    assert_eq!(float_text(1e-70), "0");
    assert_eq!(float_text(-1e-70), "-0");
    assert_eq!(
        float_text(1e300),
        "100000000000000010000000000000000000000000000000000000000000000"
    );
    assert_eq!(
        f32_text(1e-40),
        "0.000000000000000000000000000000000000000099999461011147596"
    );
    // EX34: `_fcvt(x, 64)` rounds half-up a second time at the 64th place: 9.5e-65 becomes 1e-64, whose text
    // (`0.`, 63 zeros, `1`) is then cut among the leading zeros to `0.` and 61 zeros, nothing stripped.
    assert_eq!(float_text(9.5e-65), format!("0.{}", "0".repeat(61)));
    assert_eq!(float_text(-9.5e-65), format!("-0.{}", "0".repeat(60)));
    // EX34: the second rounding starts below 1e-48 (decpt <= -48). 2e-49 (1.99999999999999987e-49) keeps 16 digits
    // and rounds them up to `2`; one rounding would print `0.`, 48 zeros and `1999999999999` (cut).
    assert_eq!(float_text(2e-49), format!("0.{}2", "0".repeat(48)));
    assert_eq!(float_text(1e-48), format!("0.{}1", "0".repeat(47)));
    // 4e-65 and 5e-65 (4.99999999999999998e-65) round to no digit; a tiny negative value keeps its sign (`-0`,
    // unlike −0.0).
    assert_eq!(float_text(4e-65), "0");
    assert_eq!(float_text(5e-65), "0");
    assert_eq!(float_text(-4e-65), "-0");
    // EX34: a cut among the significant fraction digits strips trailing zeros: 1.0000000000000002e-47 needs 65
    // characters, the cut keeps `0.`, 46 zeros and `100000000000000`, the strip leaves 49 characters.
    let stripped = float_text(1.000_000_000_000_000_2e-47);
    assert_eq!(stripped, format!("0.{}1", "0".repeat(46)));
    assert_eq!(stripped.len(), 49);
    // A text of exactly 64 characters (`0.`, 45 zeros, `10000000000000002`) is cut, not stripped: the 63 kept
    // characters end in 15 zeros.
    let exact64 = float_text(1.000_000_000_000_000_2e-46);
    assert_eq!(exact64, format!("0.{}1000000000000000", "0".repeat(45)));
    assert_eq!(exact64.len(), 63);
    // EX29: the cut applies to a Float32 too. The seven negative subnormals 0x80000001..=0x80000007 are the only
    // Float32 texts above 63 characters (`-0.`, 44 zeros, 17 digits); the 17th digit is lost.
    let tiny = f32_text(f32::from_bits(0x8000_0001));
    assert_eq!(tiny, format!("-0.{}1401298464324817", "0".repeat(44)));
    assert_eq!(tiny.len(), 63);
    let positive = f32_text(f32::from_bits(0x0000_0001));
    assert_eq!(positive, format!("0.{}14012984643248171", "0".repeat(44)));
    assert_eq!(
        f32_text(f32::from_bits(0x8000_0007)),
        format!("-0.{}9809089250273719", "0".repeat(44))
    );
    // From 0x80000008 on there is one leading zero fewer, and the text fits.
    let next = f32_text(f32::from_bits(0x8000_0008));
    assert_eq!(next, format!("-0.{}11210387714598537", "0".repeat(43)));
    assert_eq!(next.len(), 63);
    // A Float64 reaches the cut more often: −1e62 (`-` and 63 digits) and 1e63 (64 digits).
    assert_eq!(float_text(-1e62), format!("-1{}", "0".repeat(61)));
    assert_eq!(float_text(1e63), format!("10000000000000001{}", "0".repeat(46)));
    // NaN and the infinities as the C runtime prints them.
    assert_eq!(float_text(f64::INFINITY), "1.#INF");
    assert_eq!(float_text(f64::NEG_INFINITY), "-1.#INF");
    assert_eq!(float_text(f64::from_bits(0x7FF8_0000_0000_0000)), "1.#QNAN");
    assert_eq!(float_text(f64::from_bits(0xFFF8_0000_0000_0000)), "-1.#IND");
    // EX28: a signalling NaN is quieted by the x87 load before printing, a Float64 as well as a Float32.
    assert_eq!(float_text(f64::from_bits(0x7FF0_0000_0000_0001)), "1.#QNAN");
    assert_eq!(float_text(f64::from_bits(0xFFF0_0000_0000_0001)), "-1.#QNAN");
    assert_eq!(f32_text(f32::from_bits(0x7F80_0001)), "1.#QNAN");
    assert_eq!(f32_text(f32::from_bits(0xFF80_0001)), "-1.#QNAN");
    assert_eq!(f32_text(f32::from_bits(0xFFC0_0000)), "-1.#IND");
    assert_eq!(st("1.#INF").get_f32(), Some(1.0));
    assert_eq!(st("-1.#IND").get_f32(), Some(-1.0));
    // Empty (EX5): every converting getter succeeds with 0; the string getter writes the empty text, false.
    let empty = s(Values::Empty);
    assert_eq!(empty.get_u32(), Some(0));
    assert_eq!(empty.get_i64(), Some(0));
    assert_eq!(empty.get_f32(), Some(0.0));
    assert_eq!(empty.get_f64(), Some(0.0));
    assert_eq!(empty.get_bool(), Some(false));
    assert_eq!(empty.get_char(), Some(0));
    assert_eq!(text(empty.clone()), (String::new(), false));
    assert_eq!(empty.get_string(), None);
    // [choice] An object value is a missing property for every getter: false, output unchanged.
    let object = Property::resource(1, Tgi::new(1, 2, 3));
    assert_eq!(object.get_f32(), None);
    assert_eq!(object.get_u32(), None);
    assert_eq!(object.get_bool(), None);
    assert_eq!(object.get_char(), None);
    assert_eq!(object.text(), None);
    assert_eq!(text(object), ("unchanged".into(), false));
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
    // Not a string: the getter writes `?` (a unit from 0x80) and returns false.
    assert_eq!(back.get(0x34).unwrap().get_string(), None);
    assert_eq!(back.get(0x34).unwrap().text().as_deref(), Some(&b"?"[..]));
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
    // Two ParentCohort fields read (EX10); the missing PropCount line fails.
    assert!(matches!(
        Exemplar::parse(b"EQZT1###\r\nParentCohort=Key:{1,2}\r\n"),
        Err(Error::Text { line: 3, .. })
    ));
    assert!(matches!(
        Exemplar::parse(b"EQZT1###\r\nParentCohort=Key:{1,2\r\n"),
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

    // EX15: an index entry alone does not make a kind-1 resource load; a factory for its type that reads it does.
    let other = Tgi::new(0x1234_5678, 0, 0);
    let mut k1 = Exemplar::new(Kind::Exemplar);
    k1.set(Property::resource(1, other));
    let k1 = Exemplar::parse(&k1.to_binary()).unwrap();
    store.insert(exemplar_key(50), k1);
    assert!(matches!(
        store.load(&exemplar_key(50)),
        Err(LoadError::Resource { cause, .. }) if *cause == LoadError::NotFound(other)
    ));
    // EX31 condition 3: a factory must apply to every package already registered, and the store keeps no package,
    // so a factory for a type it has already met is refused *[choice]*.
    assert!(!store.add_factory(0x1234_5678, |_| true));
    // A factory whose read fails does not make the resource load either.
    let mut fresh = ExemplarStore::new();
    assert!(fresh.add_factory(0x1234_5678, |d| d == b"something else"));
    fresh.load_package(&pkg);
    fresh.insert(exemplar_key(50), store.get(&exemplar_key(50)).unwrap().clone());
    assert!(fresh.load(&exemplar_key(50)).is_err());
    // One that reads it does.
    let mut fresh = ExemplarStore::new();
    assert!(fresh.add_factory(0x1234_5678, |d| d == b"other"));
    let rep = fresh.load_package(&pkg);
    assert_eq!((rep.exemplars, rep.cohorts, rep.failures.len()), (1, 2, 0));
    fresh.insert(exemplar_key(50), store.get(&exemplar_key(50)).unwrap().clone());
    assert!(fresh.load(&exemplar_key(50)).is_ok());
    // The exemplar and cohort types keep the store's own factories.
    assert!(!fresh.add_factory(TYPE_EXEMPLAR, |_| true));
    assert!(!fresh.add_factory(TYPE_COHORT, |_| true));
    // EX38: each refusal is logged.
    assert_eq!(fresh.errors().len(), 2);

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

/// §3 "When records are read" (EX31): the store reads eagerly, and gets what the lazy original gets because the three
/// conditions hold.
#[test]
fn ex31_eager_reads_follow_the_winning_record() {
    let key = exemplar_key(60);
    let with_a = |v: u32| {
        let mut ex = Exemplar::new(Kind::Exemplar);
        ex.set(Property::single(0xA, Values::Uint32(vec![v])));
        ex.to_binary()
    };
    let pkg_of = |records: &[&[u8]]| {
        let mut b = PackageBuilder::new();
        for d in records {
            b.add(key, d, Compression::None).unwrap();
        }
        Package::from_bytes(b.build().unwrap()).unwrap()
    };
    let a_of = |store: &ExemplarStore| store.property(&key, 0xA).and_then(Property::as_u32);

    // Inside one package the first entry with a key is the one read; a later duplicate never is (§2.3).
    let mut store = ExemplarStore::new();
    let rep = store.load_package(&pkg_of(&[&with_a(1), b"not an exemplar"]));
    assert_eq!((rep.exemplars, rep.failures.len()), (1, 0));
    assert_eq!(a_of(&store), Some(1));
    let mut other = ExemplarStore::new();
    let rep = other.load_package(&pkg_of(&[b"not an exemplar", &with_a(1)]));
    assert_eq!((rep.exemplars, rep.failures.len()), (0, 1));
    assert_eq!(other.load(&key).unwrap_err(), LoadError::Unreadable(key));

    // Condition 1: a later package's record replaces the earlier read, also when it fails to read. The original reads
    // only the winning segment's record, so there is no fallback to the earlier one.
    store.load_package(&pkg_of(&[&with_a(2)]));
    assert_eq!(a_of(&store), Some(2));
    let rep = store.load_package(&pkg_of(&[b"not an exemplar"]));
    assert_eq!(rep.failures.len(), 1);
    assert!(store.get(&key).is_none());
    assert_eq!(store.load(&key).unwrap_err(), LoadError::Unreadable(key));
    assert_eq!(a_of(&store), None);
    // Condition 2: a failure is not remembered; a still later record that reads replaces it.
    store.load_package(&pkg_of(&[&with_a(3)]));
    assert_eq!(a_of(&store), Some(3));

    // Condition 3: a factory registered after a package holding its type is refused [choice]; registered first, it
    // reads that package's records.
    let res = Tgi::new(0x1234_5678, 1, 2);
    let mut b = PackageBuilder::new();
    b.add(res, b"data", Compression::None).unwrap();
    let res_pkg = Package::from_bytes(b.build().unwrap()).unwrap();
    let mut user = Exemplar::new(Kind::Exemplar);
    user.set(Property::resource(1, res));
    let user = Exemplar::parse(&user.to_binary()).unwrap();
    let mut late = ExemplarStore::new();
    late.load_package(&res_pkg);
    assert!(late.errors().is_empty());
    assert!(!late.add_factory(0x1234_5678, |_| true));
    // EX38: the refusal stands as [choice], but it is logged as an error, never ignored silently.
    assert_eq!(late.errors().len(), 1);
    assert!(late.errors()[0].starts_with("error: factory for type 12345678 refused"));
    late.insert(exemplar_key(61), user.clone());
    assert!(late.load(&exemplar_key(61)).is_err());
    let mut early = ExemplarStore::new();
    assert!(early.add_factory(0x1234_5678, |d| d == b"data"));
    assert!(early.errors().is_empty());
    early.load_package(&res_pkg);
    early.insert(exemplar_key(61), user);
    assert!(early.load(&exemplar_key(61)).is_ok());
}

// ---------------------------------------------------------------------------------------------------------------
// §10.7 Text exemplar and getter details

/// A record shaped like test 37 with one property line.
fn text_one(line: &str) -> Result<Exemplar> {
    Exemplar::parse(&text_rec(NO_PARENT, "PropCount=1", &[line]))
}

/// Property `0x10` of [`text_one`].
fn text_prop(values: &str) -> Property {
    let ex = text_one(&format!("0x10:{{\"x\"}}={values}")).unwrap();
    ex.get(0x10).cloned().unwrap()
}

fn parent_of(line: &str) -> Tgi {
    parse_ex(&text_rec(line, "PropCount=0", &[])).parent
}

/// §11 row "text ParentCohort: exactly 3 fields, leading spaces trimmed" (EX10).
#[test]
fn t51_parent_cohort_fields() {
    let ex = parse_ex(&text_rec("ParentCohort=Key:{0,0}", "PropCount=0", &[]));
    assert_eq!(ex.parent, Tgi::default());
    assert!(!ex.has_parent());
    assert_eq!(
        parent_of("ParentCohort=Key:{0x0A64ED55,0x00000001,0x05342861,0x99}"),
        Tgi::new(0x0534_2861, 0x0A64_ED55, 1),
        "the fourth field is ignored"
    );
    assert_eq!(
        parent_of("ParentCohort=Key:{0x0A64ED55, 0x00000001, 0x05342861}"),
        Tgi::new(0, 0x0A64_ED55, 0),
        "nothing trimmed: ` 0x…` reads 0"
    );
    // §4.3 worked examples: a decimal field survives a space (strtoul skips it), a missing type reads 0.
    assert_eq!(parent_of("ParentCohort=Key:{0x1,0x2,0x3,0x4}"), Tgi::new(3, 1, 2));
    assert_eq!(parent_of("ParentCohort=Key:{0x1, 0x2, 0x3}"), Tgi::new(0, 1, 0));
    assert_eq!(parent_of("ParentCohort=Key:{1, 2, 3}"), Tgi::new(3, 1, 2));
    let two = parse_ex(&text_rec("ParentCohort=Key:{0x1,0x2}", "PropCount=0", &[]));
    assert_eq!(two.parent, Tgi::new(0, 1, 2));
    let mut store = ExemplarStore::new();
    store.insert(exemplar_key(1), two);
    assert!(matches!(store.load(&exemplar_key(1)), Err(LoadError::Parent { .. })));
    // One field, and none.
    assert_eq!(parent_of("ParentCohort=Key:{0x10}"), Tgi::new(0, 0x10, 0));
    assert_eq!(parent_of("ParentCohort=Key:{}"), Tgi::default());
    // The line itself is trimmed of spaces and tabs.
    assert_eq!(parent_of(" \tParentCohort = Key:{1,2,3}\t "), Tgi::new(3, 1, 2));
}

/// §11 row "text reps trimmed" (EX11).
#[test]
fn t52_reps_and_type_name() {
    let p = text_prop("Uint32: 0x2:{1,2}");
    assert_eq!((p.is_array, p.as_u32()), (false, Some(1)));
    assert_eq!(text_prop("Uint32:4:{1,2}").u32s(), Some(&[1, 2, 0, 0][..]));
    assert_eq!(text_prop("Uint32:1:{1,2}").u32s(), Some(&[1][..]));
    assert!(text_one("0x10:{\"x\"}=Uint32 :0:{1}").is_err());
    assert!(text_one("0x10:{\"x\"}=Uint32:0: {1}").is_err());
    // reps by rule N on the raw text: `1A` is 26, ` 2` is 2 (strtoul skips the space), `0x2` is 2.
    assert_eq!(text_prop("Uint8:1A:{1}").len(), 26);
    assert_eq!(text_prop("Uint8: 2:{1}").len(), 2);
    assert_eq!(text_prop("Uint8:0x2:{1}").len(), 2);
    assert_eq!(text_prop("Uint8: 3:{1}").len(), 3);
    assert_eq!(text_prop("Uint8: 0x3:{1}").count(), 0, "` 0x3` is 0, a scalar");
    // EX16: `Uint8:1:{}` is one value, 0.
    assert_eq!(text_prop("Uint8:1:{}").u8s(), Some(&[0][..]));
}

/// §11 rows "text reps …" (EX12, EX13): values trimmed only in arrays, 64-bit hex without a sign.
#[test]
fn t53_values() {
    let p = text_prop("Uint32:0:{ 0x10}");
    assert_eq!((p.is_array, p.as_u32()), (false, Some(0)));
    assert_eq!(text_prop("Uint32:1:{ 0x10}").u32s(), Some(&[16][..]));
    assert_eq!(text_prop("Uint32:1:{\t0x10}").u32s(), Some(&[16][..]));
    assert_eq!(text_prop("Uint64:0:{10}").as_u64(), Some(16));
    assert_eq!(text_prop("Sint64:0:{-1}").as_i64(), Some(0));
    assert_eq!(text_prop("Bool:0:{1}").as_bool(), Some(false));
    // At most 16 hex digits; later ones are ignored.
    assert_eq!(
        text_prop("Uint64:0:{0x123456789ABCDEF012}").as_u64(),
        Some(0x1234_5678_9ABC_DEF0)
    );
    assert_eq!(text_prop("Sint64:0:{FFFFFFFFFFFFFFFF}").as_i64(), Some(-1));
    // Bool: leading spaces trimmed in a scalar too, then a case-insensitive `true` prefix.
    assert_eq!(text_prop("Bool:0:{  TrUe}").as_bool(), Some(true));
    assert_eq!(text_prop("Bool:2:{true, true}").bools(), Some(&[true, true][..]));
    // EX26: the Bool trim is LTrim, whose set is space and tab, as for array values.
    assert_eq!(text_prop("Bool:0:{\ttrue}").as_bool(), Some(true));
    assert_eq!(text_prop("Bool:0:{ \t TRUE}").as_bool(), Some(true));
    // EX27: a String is trimmed of spaces and tabs at both ends, then one quote is removed at each end, so white
    // space inside the quotes stays.
    let string_of = |v: &str| text_prop(v).get_string();
    assert_eq!(string_of("String:0:{ \" a \" }").as_deref(), Some(" a "));
    assert_eq!(string_of("String:0:{\t\"\tb\t\"\t}").as_deref(), Some("\tb\t"));
    assert_eq!(string_of("String:0:{ c }").as_deref(), Some("c"));
    // A scalar is not trimmed: ` 16` is still 16 (strtoul skips the space), ` 1A` is 1 (base 10).
    assert_eq!(text_prop("Uint32:0:{ 16}").as_u32(), Some(16));
    assert_eq!(text_prop("Uint32:0:{ 1A}").as_u32(), Some(1));
    assert_eq!(text_prop("Uint32:1:{ 1A}").u32s(), Some(&[26][..]));
}

/// §11 row "Float32 text parsed as f64, then rounded to f32" (EX14).
#[test]
fn t54_float32_text() {
    let f = |v: &str| text_prop(&format!("Float32:0:{{{v}}}")).as_f32().unwrap();
    assert_eq!(f("0.05").to_bits(), 0x3D4C_CCCC);
    assert_eq!(f("1e10"), 1_410_065_408.0);
    assert_eq!(f("1e32"), 0.0);
    assert_eq!(f("inf"), 0.0);
    assert_eq!(f("0.001").to_bits(), 0x3A83_126E);
    assert_eq!(f("3.14159").to_bits(), 0x4049_0FCE);
    assert_eq!(f("1e12"), -727_379_968.0);
    assert_eq!(f("1e-32"), f32::INFINITY);
    assert_eq!(f("0e-32").to_bits(), 0xFFC0_0000);
    assert_eq!(f("e-32").to_bits(), 0xFFC0_0000, "EX23: no mantissa digit is 0 × 10ⁿ");
    assert_eq!(f("e5"), 0.0);
    assert_eq!(f("33573415"), 33_573_412.0, "EX22");
    assert_eq!(f("1.#INF"), 1.0);
    assert_eq!(f("-2.5"), -2.5);
    assert_eq!(f("80"), 80.0);
    // EX35: a negative exponent is a rounded reciprocal, then a multiply (two roundings, not one divide).
    assert_eq!(f("9e-1").to_bits(), 0x3F66_6667);
    assert_eq!(f("5e-2").to_bits(), 0x3D4C_CCCC);
    assert_eq!(f("2.5e-3").to_bits(), 0x3B23_D70B);
    // EX36: the sign is applied last and flips the NaN.
    assert_eq!(f("-e-32").to_bits(), 0x7FC0_0000);
    assert_eq!(f("-1e-32"), f32::NEG_INFINITY);
}

/// The game's float parser on its own (§4.3 worked examples): the 24-bit value before any Float32 store.
#[test]
fn parse_float_worked_examples() {
    let bits = |s: &str| crate::x87::store_f32(parse_float(s.as_bytes())).to_bits();
    assert_eq!(bits("0.05"), 0x3D4C_CCCC);
    assert_eq!(0.05_f32.to_bits(), 0x3D4C_CCCD, "a correctly rounded parse differs");
    assert_eq!(bits("0.001"), 0x3A83_126E);
    assert_eq!(bits("3.14159"), 0x4049_0FCE);
    assert_eq!(bits("3.141590118408203125"), 0x4049_0FCE);
    // §5.1 round trip (EX29): the printed texts of Float32 3.14159 and 0.05 read back one step low.
    assert_eq!(bits("3.1415901184082031"), 0x4049_0FCE);
    assert_eq!(bits("0.05000000074505806"), 0x3D4C_CCCC);
    // EX22: an integer digit rounds twice (× 10, then + digit); one rounding would give 33,573,416.
    assert_eq!(parse_float(b"33573415"), 33_573_412.0);
    // EX23: the exponent is read even with no mantissa digit, with one optional sign; no digits is exponent 0.
    assert_eq!(parse_float(b"e5"), 0.0);
    assert_eq!(parse_float(b".e5"), 0.0);
    let neg_zero = parse_float(b"-e5");
    assert_eq!((neg_zero, neg_zero.is_sign_negative()), (0.0, true));
    assert_eq!(parse_float(b"2.5e+3"), 2500.0);
    assert_eq!(parse_float(b"1e"), 1.0);
    assert_eq!(bits("e-32"), 0xFFC0_0000);
    assert_eq!(bits("0e-32"), 0xFFC0_0000);
    assert_eq!(parse_float(b"1e10"), 1_410_065_408.0);
    assert_eq!(parse_float(b"1e12"), -727_379_968.0);
    assert_eq!(parse_float(b"1e32"), 0.0);
    assert_eq!(parse_float(b"1e-32"), f64::INFINITY);
    assert!(parse_float(b"0e-32").is_nan());
    assert_eq!(parse_float(b"inf"), 0.0);
    assert_eq!(parse_float(b"1.#INF"), 1.0);
    // White space, a sign, a prefix read up to the first character that does not fit.
    assert_eq!(parse_float(b" \t-12.5xyz"), -12.5);
    assert_eq!(parse_float(b"+7"), 7.0);
    assert_eq!(parse_float(b"2.5e3"), 2500.0);
    assert_eq!(parse_float(b"25E-1"), 2.5);
    assert_eq!(parse_float(b""), 0.0);
    // EX35 (§4.3): a negative exponent rounds 1/10ⁿ first (1/10 is 0x3DCCCCCD), then multiplies it in, so the
    // result is rounded twice. One division would give 0x3F666666, 0x3D4CCCCD and 0x3B23D70A.
    assert_eq!(bits("9e-1"), 0x3F66_6667);
    assert_eq!(bits("5e-2"), 0x3D4C_CCCC);
    assert_eq!(bits("2.5e-3"), 0x3B23_D70B);
    assert_eq!(
        crate::x87::store_f32(crate::x87::div(9.0, 10.0)).to_bits(),
        0x3F66_6666,
        "one division differs"
    );
    // These come out the same either way (§4.3).
    for (text, one_div) in [
        ("1e-1", 0.1f32),
        ("3e-1", 0.3),
        ("7e-1", 0.7),
        ("1e-3", 0.001),
        ("1.5e-2", 0.015),
    ] {
        assert_eq!(bits(text), one_div.to_bits(), "{text}");
    }
    // 1e-10 is about 7.09e-10 (10¹⁰ wraps to 1,410,065,408), positive exponents stay one multiply.
    let wrapped = parse_float(b"1e-10");
    assert!((wrapped - 7.09e-10).abs() < 1e-12, "{wrapped}");
    // EX36: the sign is applied last (`fchs`), so it flips the default NaN: `-e-32` gives 0x7FC00000.
    assert_eq!(bits("-e-32"), 0x7FC0_0000);
    assert_eq!(parse_float(b"-1e-32"), f64::NEG_INFINITY);
    // Every result keeps a 24-bit significand, with the register's exponent range (no Float32 overflow yet).
    let big = parse_float(b"1e9e9");
    assert_eq!(big, 1e9);
    let wide = parse_float(b"3e9");
    assert_eq!(wide, 3e9);
    let x = parse_float(b"123456789");
    assert_eq!(x, 123_456_792.0);
}

/// §10.7 test 55: too few property lines fail the read.
#[test]
fn t55_too_few_lines() {
    let lines = ["0x10:{\"a\"}=Uint32:0:{1}", "0x11:{\"b\"}=Uint32:0:{2}"];
    assert!(matches!(
        Exemplar::parse(&text_rec(NO_PARENT, "PropCount=3", &lines)),
        Err(Error::Text { line: 6, .. })
    ));
    assert_eq!(
        parse_ex(&text_rec(NO_PARENT, "PropCount=2", &lines)).properties().len(),
        2
    );
}

/// §11 row "round-trip parse" (EX9), §10.7 test 56.
#[test]
fn t56_round_trip_parse() {
    let st = |t: &str| Property::array(1, Values::String(t.as_bytes().to_vec()));
    for (t, want) in [
        ("0X1F", 0),
        ("TRUE", 0),
        ("true", 1),
        ("12 apples", 12),
        ("-5", 0xFFFF_FFFB),
        ("1.2.3", 1),
        ("1e3", 1000),
        (".5", 0),
    ] {
        assert_eq!(st(t).get_u32(), Some(want), "{t:?}");
    }
    assert_eq!(st("0.5").get_bool(), Some(true));
    assert_eq!(st("A").get_char(), Some(0));
}

/// §11 rows on EX2–EX4, §10.7 test 57.
#[test]
fn t57_scalar_conversions() {
    let s = |v: Values| Property::single(1, v);
    assert_eq!(s(Values::Float32(vec![300.7])).get_u8(), Some(44));
    assert_eq!(s(Values::Float32(vec![f32::NAN])).get_u32(), Some(0));
    assert_eq!(s(Values::Float32(vec![f32::NAN])).get_bool(), Some(true));
    assert_eq!(s(Values::String(vec![0xE9])).get_u32(), Some(0xFFFF_FFE9));
    assert_eq!(s(Values::Uint64(vec![16_777_217])).get_f64(), Some(16_777_216.0));
}

/// §11 row "floats as text" (EX8 as corrected by EX29: 17 significant digits, rounded half-up), §10.7 test 58.
#[test]
fn t58_float_text_round_trip() {
    let p = Property::single(1, Values::Float32(vec![f32::from_bits(0x4049_0FD0)]));
    let mut out = Vec::new();
    assert!(!p.get_string_into(&mut out));
    assert_eq!(out, b"3.1415901184082031");
    let s = Property::array(1, Values::String(out));
    assert_eq!(s.get_f32().map(f32::to_bits), Some(0x4049_0FCE));
}

/// §11 rows "getters on Empty …" (EX1, EX5), "void array as text" (EX6) and "holder slot 5" (EX17), §10.7 test 59.
#[test]
fn t59_empty_and_void() {
    // A tag-0 property: no value bytes, HasProperty true, GetCount() 0 (EX1).
    let ex = parse_ex(&rec(&[
        "10 00 00 00 | 00 | 00 00 00 00",
        T23.replace("10 00", "11 00").as_str(),
    ]));
    let empty = ex.get(0x10).unwrap();
    assert_eq!((&empty.values, empty.count()), (&Values::Empty, 0));
    assert_eq!(ex.get(0x11).unwrap().as_u32(), Some(7), "nothing read after the tag");
    assert_eq!(empty.get_u32(), Some(0));
    let mut out = b"x".to_vec();
    assert!(!empty.get_string_into(&mut out));
    assert!(out.is_empty());
    // A void array `31 32`.
    let void = Property::array(1, Values::Void(vec![0x31, 0x32]));
    assert_eq!(void.get_u32(), Some(0));
    let mut buf = [0u8; 8];
    let mut length = 8;
    assert!(void.get_void(&mut buf, &mut length));
    assert_eq!((length, &buf[..3]), (2, &[0x31, 0x32, 0][..]));
    // A smaller length copies that much and stays.
    let mut buf = [0u8; 8];
    let mut length = 1;
    assert!(void.get_void(&mut buf, &mut length));
    assert_eq!((length, &buf[..2]), (1, &[0x31, 0][..]));
    // Every other type returns false and leaves both unchanged: no conversion.
    for p in [
        Property::array(1, Values::String(b"12".to_vec())),
        Property::array(1, Values::Uint8(vec![1, 2])),
        Property::single(1, Values::Uint32(vec![7])),
        Property::single(1, Values::Empty),
        Property::resource(1, Tgi::new(1, 2, 3)),
    ] {
        let mut buf = [0xAAu8; 8];
        let mut length = 8;
        assert!(!p.get_void(&mut buf, &mut length), "{p:?}");
        assert_eq!((length, buf), (8, [0xAA; 8]), "{p:?}");
    }
    // Through a loaded exemplar, the same.
    let key = exemplar_key(1);
    let mut store = ExemplarStore::new();
    let mut e = Exemplar::new(Kind::Exemplar);
    e.set(Property::array(0x20, Values::Void(vec![0x31, 0x32])));
    e.set(Property::array(0x21, Values::String(b"Hi".to_vec())));
    e.set(Property::single(0x22, Values::Uint32(vec![7])));
    store.insert(key, e);
    let l = store.load(&key).unwrap();
    let (mut buf, mut length) = ([0u8; 4], 4);
    assert!(l.get_void(0x20, &mut buf, &mut length));
    assert_eq!((length, buf), (2, [0x31, 0x32, 0, 0]));
    assert!(!l.get_void(0x99, &mut buf, &mut length), "missing");
    let mut out = Vec::new();
    assert!(l.get_string_into(0x21, &mut out));
    assert_eq!(out, b"Hi");
    assert!(!l.get_string_into(0x22, &mut out));
    assert_eq!(out, b"7");
    let mut out = b"same".to_vec();
    assert!(!l.get_string_into(0x99, &mut out), "missing");
    assert_eq!(out, b"same");
}

/// §11 row on raw accessors (EX20): a type mismatch is a missing value, never the bytes reinterpreted.
#[test]
fn raw_accessor_mismatch_is_missing() {
    let f = Property::single(1, Values::Float32(vec![2.75]));
    assert_eq!(f.as_u32(), None, "the original returns 0x40300000");
    assert_eq!(f.as_f32(), Some(2.75));
    let a = Property::array(1, Values::Uint32(vec![1, 2]));
    assert_eq!(a.as_u32(), None, "the original returns a heap address");
    assert_eq!(a.f32s(), None);
    assert_eq!(Property::single(1, Values::Empty).as_u32(), None);
}
