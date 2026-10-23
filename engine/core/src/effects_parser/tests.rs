// SPDX-License-Identifier: GPL-3.0-or-later
// Independently transcribed from effects-parser.md §§4,7. Native vectors are
// PROPOSED / UNEXECUTED; these tests execute only stipulated primitive fixtures.
use super::*;
include!("extra_tests.rs");
use std::collections::BTreeMap;

#[derive(Default)]
struct Fixture {
    hits: BTreeMap<&'static str, Vec<u32>>,
    floats: BTreeMap<u32, RawFloat>,
    uints: BTreeMap<u32, u32>,
    vectors: BTreeMap<u32, [u32; 3]>,
    calls: Vec<String>,
    selected: Vec<u32>,
    validation: u8,
    validation_reindex: Option<u32>,
    index: u32,
    kind: u32,
    qi: Option<Identity>,
}
impl OptionPrimitives for Fixture {
    fn find(&mut self, _: &[Token], key: &'static str, n: u8, scratch: &mut Vec<Index>) -> u8 {
        self.calls.push(format!("find:{key}:{n}"));
        if let Some(v) = self.hits.get(key) {
            *scratch = v.iter().map(|&i| Index(i)).collect();
            0x80
        } else {
            0
        }
    }
    fn float(&mut self, token: &Token) -> RawFloat {
        self.calls.push(format!("float:{}", token.id.0));
        self.floats[&token.id.0]
    }
    fn uint(&mut self, token: &Token) -> u32 {
        self.calls.push(format!("uint:{}", token.id.0));
        self.uints[&token.id.0]
    }
    fn vector(&mut self, token: &Token) -> [u32; 3] {
        self.calls.push(format!("vector:{}", token.id.0));
        self.vectors[&token.id.0]
    }
}
impl AppendPrimitives for Fixture {
    fn construct(&mut self) -> Result<(), Unsupported> {
        self.calls.push("ctor".into());
        Ok(())
    }
    fn validate(&mut self, _: Identity, name: &Text, kind: u32, scratch: &mut Vec<Index>) -> u8 {
        self.calls.push(format!("validate:{}:{kind}:null", name.0));
        if let Some(i) = self.validation_reindex {
            scratch[0] = Index(i);
        }
        self.validation
    }
    fn assign_name(&mut self, name: &Text) {
        self.calls.push(format!("name:{}", name.0));
    }
    fn register_anonymous(&mut self, _: Identity, object: Identity) -> u32 {
        self.calls.push(format!("register:null:{}", object.0));
        self.index
    }
    fn object_type(&mut self, object: Identity) -> u32 {
        self.calls.push(format!("type:{}", object.0));
        self.kind
    }
    fn copy_record(&mut self, _: &Record) -> Result<(), Unsupported> {
        self.calls.push("copy".into());
        Ok(())
    }
    fn insert_record(&mut self, _: &Record) -> Result<(), Unsupported> {
        self.calls.push("insert:1:1".into());
        Ok(())
    }
    fn cleanup_string(&mut self) {
        self.calls.push("cleanup".into());
    }
    fn find_non_switch(&mut self, _: &[Token], n: u8, scratch: &mut Vec<Index>) -> u8 {
        self.calls.push(format!("nonswitch:{n}"));
        *scratch = self.selected.iter().map(|&i| Index(i)).collect();
        0 // deliberately ignored by the sequenceEffect body
    }
}
impl References for Fixture {
    fn add_ref(&mut self, id: Identity) {
        self.calls.push(format!("{}.add", id.0));
    }
    fn release(&mut self, id: Identity) {
        self.calls.push(format!("{}.release", id.0));
    }
    fn store_h(&mut self, id: Option<Identity>) {
        self.calls.push(format!("H={:?}", id.map(|x| x.0)));
    }
    fn store_k(&mut self, id: Option<Identity>) {
        self.calls.push(format!("K={:?}", id.map(|x| x.0)));
    }
    fn query_manager(&mut self, id: Identity, iid: u32, out: &mut Option<Identity>) -> bool {
        assert_eq!(iid, 0xC97CF5CE);
        assert_eq!(*out, None);
        self.calls.push(format!("{}.qi", id.0));
        *out = self.qi;
        false
    }
    fn reset_byte(&mut self) {
        self.calls.push("reset".into());
    }
    fn store_query_output(&mut self, id: Identity) {
        self.calls.push(format!("out={}", id.0));
    }
    fn base_query(&mut self, iid: u32, out: &mut Option<Identity>) -> bool {
        self.calls.push(format!("base:{iid}"));
        *out = self.qi;
        true
    }
}
fn tokens(s: &[&str]) -> Vec<Token> {
    s.iter()
        .enumerate()
        .map(|(i, s)| Token {
            id: Index(i as u32),
            text: Text((*s).into()),
        })
        .collect()
}
fn asymmetric() -> Record {
    let mut r = Record::shared_constructor_projection();
    r.component.name = "retained".into();
    r.type_word = 0x12345678;
    r.component.flags = 0xA5A50001;
    r.component.placement[0] = 0xFE;
    r.component.placement[1] = 0xA7;
    for (at, bits) in [
        (40, 0x41100000u32),
        (44, 0x41000000),
        (48, 0x40E00000),
        (52, 0x40400000),
    ] {
        r.component.placement[at..at + 4].copy_from_slice(&bits.to_le_bytes());
    }
    r.component.lod_min = 7;
    r.component.lod_max = 9;
    r.component.render = [11., 12., 13., 14.];
    r.component.select_weight = 0x1234;
    r
}
const ORDER: &[&str] = &[
    "find:offset:1",
    "find:rotateX:1",
    "find:rotateY:1",
    "find:rotateZ:1",
    "find:rotateXYZ:3",
    "find:rotateZXY:3",
    "find:scale:1",
    "find:lodRange:2",
    "find:lod:1",
    "find:emitScale:1",
    "find:sizeScale:1",
    "find:prob:1",
    "find:ignoreLength:0",
    "find:respectLength:0",
];
fn run(f: &mut Fixture, s: &[&str], select: u32, r: &mut Record) -> Result<(), Unsupported> {
    parse_options(
        &tokens(s),
        &mut Vec::new(),
        select,
        r,
        NumericPolicy::NearestAtLeast53,
        f,
    )
}
fn bits(r: &Record, at: usize) -> u32 {
    u32::from_le_bytes(r.component.placement[at..at + 4].try_into().unwrap())
}

#[test]
fn p1_absent_retains_asymmetric_fields() {
    let mut f = Fixture::default();
    let mut r = asymmetric();
    let mut expected = r.clone();
    expected.component.lod_min = 1;
    expected.component.lod_max = 6;
    assert_eq!(run(&mut f, &[], 0, &mut r), Ok(()));
    assert_eq!(r, expected);
    assert_eq!(f.calls, ORDER);
}
#[test]
fn p2_exact_one_clears_only_scale_bit() {
    let mut f = Fixture::default();
    f.hits.insert("scale", vec![1]);
    f.floats.insert(1, RawFloat::Dyadic(1, 0));
    let mut r = asymmetric();
    run(&mut f, &["-scale", "1"], 0, &mut r).unwrap();
    assert_eq!(
        (bits(&r, 52), r.component.placement[0], r.component.placement[1]),
        (0x3F800000, 0xFF, 0xA5)
    );
}
#[test]
fn p3_offset_then_scale_wraps_count() {
    let mut f = Fixture::default();
    f.hits.insert("offset", vec![1]);
    f.hits.insert("scale", vec![3]);
    f.vectors.insert(1, [0, 0, 0]);
    f.floats.insert(3, RawFloat::Dyadic(2, 0));
    let mut r = asymmetric();
    run(&mut f, &["-offset", "0,0,0", "-scale", "2"], 0, &mut r).unwrap();
    assert_eq!(
        [bits(&r, 40), bits(&r, 44), bits(&r, 48), bits(&r, 52)],
        [0, 0, 0, 0x40000000]
    );
    assert_eq!(&r.component.placement[..2], &[0, 0xA7]);
    assert_eq!(
        f.calls
            .iter()
            .filter(|x| x.starts_with("vector:") || x.starts_with("float:"))
            .cloned()
            .collect::<Vec<_>>(),
        ["vector:1", "float:3"]
    );
}
#[test]
fn p4_range_skips_lod_and_wraps() {
    let mut f = Fixture::default();
    f.hits.insert("lodRange", vec![3, 4]);
    f.hits.insert("lod", vec![1]);
    f.uints.insert(3, 258);
    f.uints.insert(4, 511);
    let mut r = asymmetric();
    run(&mut f, &["-lod", "255", "-lodRange", "258", "511"], 0, &mut r).unwrap();
    assert_eq!((r.component.lod_min, r.component.lod_max), (2, 0));
    assert!(!f.calls.iter().any(|x| x == "find:lod:1"));
    assert_eq!(
        f.calls
            .iter()
            .filter(|x| x.starts_with("uint:"))
            .cloned()
            .collect::<Vec<_>>(),
        ["uint:3", "uint:4"]
    );
}
#[test]
fn p5_length_order_overrides_text_order() {
    let mut f = Fixture::default();
    f.hits.insert("ignoreLength", vec![]);
    f.hits.insert("respectLength", vec![]);
    let mut r = asymmetric();
    run(&mut f, &["-respectLength", "-ignoreLength"], 0, &mut r).unwrap();
    assert_eq!(r.component.flags, 0xA5A50000);
    assert_eq!(&f.calls[12..], &["find:ignoreLength:0", "find:respectLength:0"]);
}
#[test]
fn p6_actual_scratch_count_controls_second_conversion() {
    let mut f = Fixture::default();
    f.hits.insert("emitScale", vec![1]);
    f.hits.insert("sizeScale", vec![3, 4, 99]);
    for (i, n) in [(1, 2), (3, 3), (4, 4)] {
        f.floats.insert(i, RawFloat::Dyadic(n, 0));
    }
    let mut r = asymmetric();
    run(&mut f, &["-emitScale", "2", "-sizeScale", "3", "4"], 0, &mut r).unwrap();
    assert_eq!(r.component.render, [2., 2., 3., 4.]);
    assert_eq!(r.component.placement[0], 0xFE);
    assert_eq!(
        f.calls
            .iter()
            .filter(|x| x.starts_with("float:"))
            .cloned()
            .collect::<Vec<_>>(),
        ["float:1", "float:3", "float:4"]
    );
}
#[test]
fn p7_duplicate_alternatives_are_distinct_foreign_fixtures() {
    for (index, value, flag, store) in [(3, 1, 0xA5, 0x3F800000), (1, 2, 0xA7, 0x40000000)] {
        let mut f = Fixture::default();
        f.hits.insert("scale", vec![index]);
        f.floats.insert(index, RawFloat::Dyadic(value, 0));
        let mut r = asymmetric();
        run(&mut f, &["-scale", "2", "-scale", "1", "-unknown", ""], 0, &mut r).unwrap();
        assert_eq!(
            (bits(&r, 52), r.component.placement[1], r.component.placement[0]),
            (store, flag, 0xFF)
        );
        assert_eq!(f.calls.iter().filter(|x| x.starts_with("float:")).count(), 1);
    }
}
#[test]
fn p8_probability_raw_low16_not_cast() {
    for (value, weight) in [(1, 0xFFFF), (0, 0)] {
        let mut f = Fixture::default();
        f.hits.insert("prob", vec![1]);
        f.floats.insert(1, RawFloat::Dyadic(value, 0));
        let mut r = asymmetric();
        run(&mut f, &["-prob", "1"], 7, &mut r).unwrap();
        assert_eq!(r.component.select_weight, weight);
        assert_eq!((r.component.lod_min, r.component.lod_max), (1, 6));
        assert_eq!(r.component.placement[0], 0xFE);
    }
}
fn named_fixture() -> (Parser, Fixture, Vec<Token>) {
    let mut p = Parser::with_capacity(2);
    p.manager = Some(Identity(1));
    p.current_block = Text("parent".into());
    p.select = 0x12345678;
    let f = Fixture {
        validation: 1,
        selected: vec![1],
        ..Fixture::default()
    };
    (p, f, tokens(&["sequenceEffect", "loopA", "other"]))
}
#[test]
fn r1_sequence_named_repeat_append() {
    let (mut p, mut f, t) = named_fixture();
    for _ in 0..2 {
        p.sequence_effect(&t, NumericPolicy::NearestAtLeast53, &mut f).unwrap();
    }
    assert_eq!(p.components.len(), 2);
    for r in &p.components {
        assert_eq!(
            (
                &*r.component.name,
                r.type_word,
                r.component.select_group,
                r.component.flags,
                r.component.index
            ),
            ("loopA", 6, 0x5678, 0, u32::MAX)
        );
    }
    let mut expected = vec!["nonswitch:1", "ctor", "validate:loopA:6:null", "name:loopA"];
    expected.extend_from_slice(ORDER);
    expected.extend(["copy", "cleanup"]);
    assert_eq!(f.calls, expected.repeat(2));
    assert!(p.pending.is_none());
}
#[test]
fn r2_options_override_initial_clear() {
    let (mut p, mut f, t) = named_fixture();
    f.hits.insert("ignoreLength", vec![]);
    p.sequence_effect(&t, NumericPolicy::NearestAtLeast53, &mut f).unwrap();
    assert_eq!(p.components[0].component.flags, 1);
}
#[test]
fn r3_anonymous_sentinel_is_success_and_full_type_retained() {
    let mut p = Parser::with_capacity(2);
    p.manager = Some(Identity(1));
    let mut f = Fixture {
        index: u32::MAX,
        kind: 9,
        ..Fixture::default()
    };
    p.add_anonymous(
        &tokens(&["soundEffect", "bell"]),
        Identity(8),
        7,
        NumericPolicy::NearestAtLeast53,
        &mut f,
    )
    .unwrap();
    let r = &p.components[0];
    assert_eq!(
        (&*r.component.name, r.type_word, r.component.index, r.component.flags),
        ("", 9, u32::MAX, 1)
    );
    assert_eq!(&f.calls[..3], ["ctor", "register:null:8", "type:8"]);
    assert_eq!(&f.calls[f.calls.len() - 2..], ["copy", "cleanup"]);
    assert!(!f.calls.iter().any(|x| x.contains(".add") || x.contains(".release")));
}
#[test]
fn r4_anonymous_raw_index_has_no_local_counter() {
    let mut p = Parser::with_capacity(2);
    p.manager = Some(Identity(1));
    let mut f = Fixture {
        kind: 0x12345609,
        ..Fixture::default()
    };
    for index in [0, u32::MAX] {
        f.index = index;
        p.add_anonymous(&[], Identity(8), 0, NumericPolicy::NearestAtLeast53, &mut f)
            .unwrap();
    }
    assert_eq!(
        p.components.iter().map(|r| r.component.index).collect::<Vec<_>>(),
        [0, u32::MAX]
    );
    assert_eq!(p.components[1].type_word, 0x12345609);
}
#[test]
fn r5_null_manager_short_circuits_both_paths() {
    let mut p = Parser::with_capacity(0);
    let mut f = Fixture::default();
    p.add_named(&[], 7, NumericPolicy::Unqualified, &mut f).unwrap();
    p.add_anonymous(&[], Identity(8), 7, NumericPolicy::Unqualified, &mut f)
        .unwrap();
    assert!(f.calls.is_empty());
    assert!(p.pending.is_none());
    assert!(p.components.is_empty());
}
#[test]
fn r6_capacity_branch_is_after_mutation() {
    let (mut p, mut f, t) = named_fixture();
    p.components = Vec::new();
    p.scratch = vec![Index(1)];
    p.add_named(&t, 0, NumericPolicy::NearestAtLeast53, &mut f).unwrap();
    assert_eq!(
        &f.calls[f.calls.len() - 3..],
        ["find:respectLength:0", "insert:1:1", "cleanup"]
    );
    assert_eq!(
        (p.components[0].component.lod_min, p.components[0].component.lod_max),
        (1, 6)
    );
}
fn refs(h: Option<u32>, k: Option<u32>, m: Option<u32>, t: Option<u32>) -> (Parser, Vec<String>) {
    let mut p = Parser::with_capacity(0);
    p.manager = h.map(Identity);
    p.interface = k.map(Identity);
    p.reset = 9;
    let mut f = Fixture {
        qi: t.map(Identity),
        ..Fixture::default()
    };
    p.set_manager(m.map(Identity), &mut f);
    (p, f.calls)
}
#[test]
fn m1_distinct_manager_reference_order() {
    let (p, c) = refs(Some(1), Some(3), Some(2), Some(4));
    assert_eq!(
        c,
        [
            "2.add",
            "H=Some(2)",
            "1.release",
            "2.qi",
            "4.add",
            "4.release",
            "4.add",
            "K=Some(4)",
            "3.release",
            "4.release",
            "reset"
        ]
    );
    assert_eq!(
        (p.manager, p.interface, p.reset),
        (Some(Identity(2)), Some(Identity(4)), 0)
    );
}
#[test]
fn m2_same_manager_still_queries_same_interface() {
    let (_, c) = refs(Some(1), Some(3), Some(1), Some(3));
    assert_eq!(c, ["1.qi", "3.add", "3.release", "3.release", "reset"]);
}
#[test]
fn m3_null_manager_retains_k() {
    let (p, c) = refs(Some(1), Some(3), None, None);
    assert_eq!(c, ["H=None", "1.release", "reset"]);
    assert_eq!(p.interface, Some(Identity(3)));
}
#[test]
fn m4_nonnull_manager_null_qi_clears_k() {
    let (p, c) = refs(Some(1), Some(3), Some(1), None);
    assert_eq!(c, ["1.qi", "K=None", "3.release", "reset"]);
    assert_eq!(p.interface, None);
}
#[test]
fn q1_logical_adjusted_identity_and_nullout() {
    let mut f = Fixture::default();
    let mut out = None;
    let q = QueryIdentities {
        adjusted_interface: Identity(41),
        canonical_unknown: Identity(97),
    };
    assert!(!query_interface(0x6956BABC, None, Some(q), &mut f).unwrap());
    assert!(f.calls.is_empty());
    assert!(query_interface(0x6956BABC, Some(&mut out), Some(q), &mut f).unwrap());
    assert_eq!(out, Some(Identity(41)));
    assert_eq!(f.calls, ["out=41", "97.add"]);
}
