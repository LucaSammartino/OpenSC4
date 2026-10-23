// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::BTreeMap;

const H: u32 = 0x1000;
const P: u32 = 0x2200; // Full nonzero pointer, low byte zero.
const Q: u32 = 0x3300;
const VP: u32 = 0x4400; // Deliberately NOT P+4, also low byte zero.
const VQ: u32 = 0x5500;
const N: u32 = 0x6600;
const VN: u32 = 0x6604;
const R: u32 = 0x7700;
const D: u32 = 0x8800;
const B: u32 = 0x9900;
const POOL: u32 = 0xaa00;
const IA: u32 = 0x1357_9bdf;
const IB: u32 = 0xa246_8cef;
const O: u32 = 0xbb00;
const FULL: u32 = u32::MAX;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Call(&'static str, Vec<u32>);

fn c(name: &'static str, words: &[u32]) -> Call {
    Call(name, words.to_vec())
}

struct Raw {
    lookup_answer: u32,
    words: BTreeMap<u32, u32>,
    calls: Vec<Call>,
    install_bits: Residue,
    retain: bool,
    refs: BTreeMap<u32, u32>,
    allocator_count: u32,
    typed_bits: Residue,
    source: Vec<u8>,
    copied: Vec<u8>,
    tag: u32,
}

impl Raw {
    fn fixture(answer: u32, head: u32) -> Self {
        Self {
            lookup_answer: answer,
            words: BTreeMap::from([
                (0x00b0_a9ec, POOL),
                (POOL + 8, head),
                (POOL + 12, 41),
                (N, R),
                (D, B),
                (D + 4, B),
                (D + 8, 0xd00d_feed),
                (O, 0xcc33_aa55),
            ]),
            calls: Vec::new(),
            install_bits: Residue {
                eax: 0xaaaa_aa00,
                defined_mask: FULL,
            },
            retain: true,
            refs: BTreeMap::from([(P, 17), (Q, 29)]),
            allocator_count: 41,
            typed_bits: Residue {
                eax: 0x1357_2400,
                defined_mask: FULL,
            },
            source: Vec::new(),
            copied: Vec::new(),
            tag: 6,
        }
    }

    fn typed(&mut self, name: &'static str, words: &[u32]) -> Normal<Residue> {
        self.calls.push(c(name, words));
        Ok(self.typed_bits)
    }

    fn copy(&mut self, name: &'static str, args: [u32; 4], width: usize, tag: u32) -> Normal<Residue> {
        self.calls.push(c(name, &args));
        let bytes = args[3] as usize * width;
        if bytes != 0 {
            assert_eq!(args[2], B);
        }
        self.copied = self.source[..bytes].to_vec();
        self.tag = tag;
        Ok(self.typed_bits)
    }
}

macro_rules! raw_word {
    ($method:ident) => {
        fn $method(&mut self, v: u32, slot: u32, word: u32) -> Normal<Residue> {
            self.typed(stringify!($method), &[v, slot, word])
        }
    };
}

impl Primitives for Raw {
    fn lookup(&mut self, h: u32, slot: u32, id: u32) -> Normal<u32> {
        self.calls.push(c("lookup", &[h, slot, id]));
        Ok(self.lookup_answer)
    }
    fn read_word(&mut self, address: u32) -> Normal<u32> {
        self.calls.push(c("read", &[address]));
        self.words.get(&address).copied().ok_or(Refusal::Memory)
    }
    fn write_word(&mut self, address: u32, word: u32) -> Normal<()> {
        self.calls.push(c("write", &[address, word]));
        self.words.insert(address, word);
        Ok(())
    }
    fn allocate_block(&mut self, receiver: u32) -> Normal<u32> {
        self.calls.push(c("allocate", &[receiver]));
        // Independent raw side effect: tests can start at 7, allocator writes 41.
        self.words.insert(POOL + 12, self.allocator_count);
        self.words.insert(POOL + 8, 0xdead_0000);
        Ok(N)
    }
    fn construct_variant(&mut self, receiver: u32) -> Normal<()> {
        self.calls.push(c("construct", &[receiver]));
        // Explicit fixture effects, not a helper-owned count initialization.
        self.words.insert(receiver, 0xcafe_babe);
        self.refs.insert(N, 13);
        Ok(())
    }
    fn install(&mut self, h: u32, slot: u32, property: u32, zero: u32) -> Normal<Residue> {
        self.calls.push(c("install", &[h, slot, property, zero]));
        if self.retain {
            *self.refs.get_mut(&property).unwrap() += 1;
        }
        Ok(self.install_bits)
    }
    fn get_variant(&mut self, property: u32, slot: u32) -> Normal<u32> {
        self.calls.push(c("get_variant", &[property, slot]));
        match property {
            P => Ok(VP),
            Q => Ok(VQ),
            N => Ok(VN),
            _ => Err(Refusal::Primitive),
        }
    }
    raw_word!(set_bool);
    raw_word!(set_u8);
    raw_word!(set_s32);
    raw_word!(set_u32);
    raw_word!(set_f32);
    raw_word!(set_string);
    fn set_s64(&mut self, v: u32, slot: u32, low: u32, high: u32) -> Normal<Residue> {
        self.typed("set_s64", &[v, slot, low, high])
    }
    fn copy_u8(&mut self, v: u32, slot: u32, begin: u32, count: u32) -> Normal<Residue> {
        self.copy("copy_u8", [v, slot, begin, count], 1, 0x8002)
    }
    fn copy_u32(&mut self, v: u32, slot: u32, begin: u32, count: u32) -> Normal<Residue> {
        self.copy("copy_u32", [v, slot, begin, count], 4, 0x8006)
    }
    fn copy_f32(&mut self, v: u32, slot: u32, begin: u32, count: u32) -> Normal<Residue> {
        self.copy("copy_f32", [v, slot, begin, count], 4, 0x800a)
    }
    fn copy_s64(&mut self, v: u32, slot: u32, begin: u32, count: u32) -> Normal<Residue> {
        self.copy("copy_s64", [v, slot, begin, count], 8, 0x8009)
    }
}

// Literal ledger prefixes: no production constants, lookup logic or count calculation
// are reused to derive these expected calls.
fn e(property: u32, _variant: u32, id: u32) -> Vec<Call> {
    vec![c("lookup", &[H, 9, id]), c("get_variant", &[property, 5])]
}

fn missing(id: u32, allocation: bool, count: u32) -> Vec<Call> {
    let mut calls = vec![
        c("lookup", &[H, 9, id]),
        c("read", &[0x00b0_a9ec]),
        c("read", &[POOL + 8]),
    ];
    if allocation {
        calls.push(c("allocate", &[0x00b0_a9e8]));
    }
    calls.extend([
        c("read", &[N]),
        c("write", &[POOL + 8, R]),
        c("read", &[POOL + 12]),
        c("write", &[POOL + 12, count]),
        c("write", &[N, 0x00aa_6c54]),
        c("construct", &[0x6604]),
        c("write", &[N, 0x00aa_6988]),
        c("write", &[0x6604, 0x00aa_6788]),
        c("write", &[0x6618, id]),
        c("install", &[H, 15, N, 0]),
        c("get_variant", &[N, 5]),
    ]);
    calls
}

fn untouched(raw: &Raw) {
    assert_eq!(raw.words[&O], 0xcc33_aa55);
    assert_eq!(raw.words[&(D + 8)], 0xd00d_feed);
    assert_eq!(raw.refs[&P], 17);
    assert_eq!(raw.refs[&Q], 29);
}

type WordHelper = fn(&mut Raw, u32, u32, u32) -> Normal<Residue>;
type ScalarCase = (WordHelper, u32, u32, u32, u32, &'static str, u32, u32, u32);
type ArrayCase = (WordHelper, &'static str, u32, u32, &'static [u8], [(u32, u32); 3]);

#[test]
fn published_s1_through_s12_literal_scalar_traces() {
    // S1–S8, S10–S11: helper, raw lookup answer/head, ID, whole word,
    // primitive name, slot, raw final EAX and its defined mask.
    let rows: [ScalarCase; 10] = [
        (set_bool, P, N, IA, 0, "set_bool", 11, 0, 0xff),
        (set_bool, 0, N, IB, 1, "set_bool", 11, 1, 0xff),
        (set_u32, Q, N, IA, 0xfedc_ba98, "set_u32", 26, 0xfedc_ba98, FULL),
        (set_u32, 0, N, IB, 0, "set_u32", 26, 0, FULL),
        (set_f32, 0, 0, IB, 0x3fc0_0000, "set_f32", 38, 0x3fc0_0000, FULL),
        (set_u8, P, N, IB, 0x7b6c_00a5, "set_u8", 14, 0xa5, 0xff),
        (set_s32, 0, N, IA, 0x8123_4567, "set_s32", 29, 0x8123_4567, FULL),
        (set_string, 0, N, IA, 0x1234_5000, "set_string", 56, 0xabcd_ef00, FULL),
        (set_u32, P, N, IA, 0x89ab_cdef, "set_u32", 26, 0x89ab_cdef, FULL),
        (set_f32, P, N, IA, 0x8000_0000, "set_f32", 38, 0x8000_0000, FULL),
    ];
    for (helper, property, head, id, word, name, slot, eax, mask) in rows {
        let mut raw = Raw::fixture(property, head);
        raw.retain = word != 0;
        if slot == 11 && word == 1 {
            raw.install_bits.eax = 1;
        }
        if head == 0 {
            raw.words.insert(POOL + 12, 7);
        }
        raw.typed_bits = Residue {
            eax,
            defined_mask: mask,
        };
        let result = helper(&mut raw, H, id, word).unwrap();
        assert_eq!(
            result,
            Residue {
                eax,
                defined_mask: mask
            }
        );
        let (mut expected, v) = match property {
            P => (e(P, VP, id), VP),
            Q => (e(Q, VQ, id), VQ),
            _ => (missing(id, head == 0, 42), VN),
        };
        expected.push(c(name, &[v, slot, word]));
        assert_eq!(raw.calls, expected);
        if property == 0 {
            assert_eq!(raw.refs[&N], if raw.retain { 14 } else { 13 });
            assert_eq!(raw.words[&(POOL + 8)], R);
            assert_eq!(raw.words[&(POOL + 12)], 42);
        }
        untouched(&raw);
    }
    // S9: EAX high word, not a 64-bit return. S12: full false-looking
    // and true-looking raw bits pass through, with all earlier effects intact.
    for eax in [0xfedc_ba98, 0x1357_2400, 0x2468_1301] {
        let mut raw = Raw::fixture(P, N);
        raw.typed_bits = Residue {
            eax,
            defined_mask: FULL,
        };
        assert_eq!(set_s64(&mut raw, H, IB, 0x89ab_cdef, 0xfedc_ba98), Ok(raw.typed_bits));
        let mut expected = e(P, VP, IB);
        expected.push(c("set_s64", &[VP, 35, 0x89ab_cdef, 0xfedc_ba98]));
        assert_eq!(raw.calls, expected);
        untouched(&raw);
    }
}

#[test]
fn install_residue_is_independent_of_retention_and_count_wraps() {
    for eax in [0x9988_7700, 0x1122_3301] {
        for retain in [false, true] {
            let mut raw = Raw::fixture(0, N);
            raw.install_bits = Residue {
                eax,
                defined_mask: 0xff,
            };
            raw.retain = retain;
            raw.words.insert(POOL + 12, FULL);
            assert_eq!(set_u32(&mut raw, H, IA, 7), Ok(raw.typed_bits));
            let mut expected = missing(IA, false, 0);
            expected.push(c("set_u32", &[VN, 26, 7]));
            assert_eq!(raw.calls, expected);
            assert_eq!(raw.refs[&N], if retain { 14 } else { 13 });
            assert_eq!(raw.words[&(POOL + 12)], 0);
            untouched(&raw);
        }
    }
}

#[test]
fn every_overload_executes_existing_inherited_missing_and_allocator_paths() {
    let words: [(WordHelper, &str, u32, u32); 6] = [
        (set_bool, "set_bool", 11, 0x7b6c_0080),
        (set_u8, "set_u8", 14, 0xabcd_00a5),
        (set_s32, "set_s32", 29, 0x8123_4567),
        (set_u32, "set_u32", 26, 0xf123_4567),
        (set_f32, "set_f32", 38, 0x3fc0_0000),
        (set_string, "set_string", 56, 0x1234_5000),
    ];
    for (property, head, v) in [(P, N, VP), (Q, N, VQ), (0, N, VN), (0, 0, VN)] {
        for (helper, name, slot, word) in words {
            let mut raw = Raw::fixture(property, head);
            assert_eq!(helper(&mut raw, H, IB, word), Ok(raw.typed_bits));
            let mut expected = if property == 0 {
                missing(IB, head == 0, 42)
            } else {
                e(property, v, IB)
            };
            expected.push(c(name, &[v, slot, word]));
            assert_eq!(raw.calls, expected);
            untouched(&raw);
        }
        let mut raw = Raw::fixture(property, head);
        assert_eq!(set_s64(&mut raw, H, IA, 0x89ab_cdef, 0xfedc_ba98), Ok(raw.typed_bits));
        let mut expected = if property == 0 {
            missing(IA, head == 0, 42)
        } else {
            e(property, v, IA)
        };
        expected.push(c("set_s64", &[v, 35, 0x89ab_cdef, 0xfedc_ba98]));
        assert_eq!(raw.calls, expected);
    }
}

// Little-endian byte literals, independent of helper count calculations.
const U8_BYTES: &[u8] = &[0xa5, 0x00, 0x7f];
const U32_BYTES: &[u8] = &[0x67, 0x45, 0x23, 0x01, 0xef, 0xcd, 0xab, 0x89, 0x98, 0xba, 0xdc, 0xfe];
const F32_BYTES: &[u8] = &[0, 0, 0, 0x80, 0, 0, 0xc0, 0x3f, 0, 0, 0x20, 0xc0];
const S64_BYTES: &[u8] = &[
    0xef, 0xcd, 0xab, 0x89, 0x98, 0xba, 0xdc, 0xfe, 0x67, 0x45, 0x23, 0x01, 0, 0, 0, 0, 0x10, 0x32, 0x54, 0x76, 0xff,
    0xff, 0xff, 0xff,
];

#[test]
fn published_twelve_array_literals_expanded_to_24_existing_new_traces() {
    let rows: [ArrayCase; 4] = [
        (set_u8_vector, "copy_u8", 94, 0x8002, U8_BYTES, [(0, 0), (1, 1), (3, 3)]),
        (
            set_u32_vector,
            "copy_u32",
            102,
            0x8006,
            U32_BYTES,
            [(0, 0), (4, 1), (12, 3)],
        ),
        (
            set_f32_vector,
            "copy_f32",
            110,
            0x800a,
            F32_BYTES,
            [(0, 0), (4, 1), (12, 3)],
        ),
        (
            set_s64_vector,
            "copy_s64",
            108,
            0x8009,
            S64_BYTES,
            [(0, 0), (8, 1), (24, 3)],
        ),
    ];
    let mut traces = 0;
    for (helper, name, slot, tag, source, cases) in rows {
        for (extent, count) in cases {
            for (property, id, v) in [(P, IA, VP), (0, IB, VN)] {
                let mut raw = Raw::fixture(property, N);
                raw.words.insert(D + 4, B + extent);
                raw.source = source.to_vec();
                let words_before = [raw.words[&D], raw.words[&(D + 4)], raw.words[&(D + 8)]];
                assert_eq!(helper(&mut raw, H, id, D), Ok(raw.typed_bits));
                let mut expected = if property == 0 {
                    missing(id, false, 42)
                } else {
                    e(P, VP, id)
                };
                expected.extend([c("read", &[D]), c("read", &[D + 4]), c(name, &[v, slot, B, count])]);
                assert_eq!(raw.calls, expected);
                assert_eq!(raw.copied, source[..extent as usize]);
                assert_eq!(raw.source, source);
                assert_eq!(raw.tag, tag);
                assert_eq!([raw.words[&D], raw.words[&(D + 4)], raw.words[&(D + 8)]], words_before);
                untouched(&raw);
                traces += 1;
            }
        }
    }
    assert_eq!(traces, 24);
}

#[test]
fn eight_empty_begin_distinctions_plus_inherited_and_allocating_vector_paths() {
    let rows: [(WordHelper, &str, u32); 4] = [
        (set_u8_vector, "copy_u8", 94),
        (set_u32_vector, "copy_u32", 102),
        (set_f32_vector, "copy_f32", 110),
        (set_s64_vector, "copy_s64", 108),
    ];
    let mut empty_distinctions = 0;
    for (helper, name, slot) in rows {
        for begin in [B, 0] {
            for (property, head, v) in [(P, N, VP), (Q, N, VQ), (0, 0, VN)] {
                let mut raw = Raw::fixture(property, head);
                raw.words.insert(D, begin);
                raw.words.insert(D + 4, begin);
                assert_eq!(helper(&mut raw, H, IA, D), Ok(raw.typed_bits));
                let mut expected = if property == 0 {
                    missing(IA, true, 42)
                } else {
                    e(property, v, IA)
                };
                expected.extend([c("read", &[D]), c("read", &[D + 4]), c(name, &[v, slot, begin, 0])]);
                assert_eq!(raw.calls, expected);
                untouched(&raw);
            }
            empty_distinctions += 1;
        }
    }
    assert_eq!(empty_distinctions, 8);
}

#[test]
fn portable_refusal_preserves_completed_prefix_and_never_cleans_up() {
    for (begin, end) in [(B, B - 4), (B, B + 3), (0, 4), (8, 0x8000_0008)] {
        let mut raw = Raw::fixture(0, N);
        raw.words.insert(D, begin);
        raw.words.insert(D + 4, end);
        assert_eq!(set_u32_vector(&mut raw, H, IA, D), Err(Refusal::VectorDomain));
        let mut expected = missing(IA, false, 42);
        expected.extend([c("read", &[D]), c("read", &[D + 4])]);
        assert_eq!(raw.calls, expected);
        assert_eq!(raw.refs[&N], 14);
        assert_eq!(raw.words[&(N + 0x18)], IA);
    }
    let mut raw = Raw::fixture(0, N);
    raw.words.remove(&(POOL + 12));
    assert_eq!(set_u32(&mut raw, H, IA, 1), Err(Refusal::Memory));
    assert_eq!(raw.words[&(POOL + 8)], R);
    assert_eq!(raw.words[&N], R);
    assert!(!raw.refs.contains_key(&N));
}
