//! Executable rz-strings.md §9 vectors. IDs are kept in test names.

use std::sync::{Arc, Mutex};

use super::*;

fn s(bytes: &[u8]) -> RzString {
    RzString::from_bytes(bytes)
}

fn value(d: &mut Detokenizer, name: &[u8], bytes: &[u8]) {
    d.set_token_replacement_value(&s(name), &s(bytes)).unwrap();
}

fn parse(d: &mut impl DetokenizerDispatch, bytes: &[u8]) -> (bool, Vec<u8>) {
    let mut output = s(b"previous");
    let success = d.detokenize(&s(bytes), &mut output).unwrap();
    (success, output.bytes().to_vec())
}

fn expect_parse(d: &mut impl DetokenizerDispatch, input: &[u8], success: bool, output: &[u8]) {
    assert_eq!(parse(d, input), (success, output.to_vec()));
}

struct Numbers;
impl FormatPrimitives for Numbers {
    fn float_digits(&self, _: u8, _: f64, _: Option<u32>, _: bool) -> Result<Vec<u8>, StringFault> {
        panic!("integer/string vector must not call a floating primitive")
    }
    fn pointer_digits(&self, _: u32, _: Option<u32>) -> Result<Vec<u8>, StringFault> {
        panic!("integer/string vector must not call a pointer primitive")
    }
}

#[test]
fn rz01_default_storage_and_lifetime() {
    let mut text = RzString::new();
    assert_eq!(text.strlen(), 0);
    assert_eq!(text.to_char(), b"\0");
    assert_eq!(text.allocation_bytes(), 8);
    assert_eq!(text.ref_count(), 0);
    assert_eq!(text.release(), Released::Destroy);
    assert!(text.query_interface(IID_STRING));
    assert!(text.query_interface(IID_UNKNOWN));
    assert_eq!(text.ref_count(), 2);
    assert_eq!(text.release(), Released::Alive(1));
    assert_eq!(text.release(), Released::Destroy);
}

#[test]
fn rz02_independent_counted_copy_and_buffer() {
    let mut source = s(b"A\0B");
    let mut copy = RzString::new();
    copy.copy(&source);
    assert_eq!(source.to_char().as_ptr(), source.to_char_mut().as_ptr());
    source.to_char_mut()[0] = b'C';
    assert_eq!(copy.bytes(), b"A\0B");
    assert_eq!(copy.strlen(), 3);
    assert_eq!(copy.to_char(), b"A\0B\0");
}

#[test]
fn rz03_null_setter() {
    let mut text = s(b"nonempty");
    text.from_char(None, 123);
    assert_eq!(text.to_char(), b"\0");
    text.set_c_string(Some(b"ab\0cd"));
    assert_eq!(text.bytes(), b"ab");
    text.set_c_string(None);
    assert_eq!(text.strlen(), 0);
}

#[test]
fn rz04_resize_is_not_reserve() {
    let mut text = s(b"ab");
    text.resize(4);
    assert_eq!(text.bytes(), b"ab\0\0");
    assert_eq!(text.strlen(), 4);
    text.resize(1);
    assert_eq!(text.bytes(), b"a");
    assert_eq!(text.to_char(), b"a\0");
}

#[test]
fn rz05_counted_edits() {
    let mut text = s(b"ab");
    text.append(b"\0c");
    assert_eq!(text.bytes(), b"ab\0c");
    text.insert(1, b"X").unwrap();
    assert_eq!(text.bytes(), b"aXb\0c");
    text.erase(2, u32::MAX).unwrap();
    assert_eq!(text.bytes(), b"aX");
}

#[test]
fn rz06_replace_old_count_and_identity() {
    let mut text = s(b"abcdef");
    let identity = &text as *const RzString;
    assert_eq!(text.replace(1, b"XY", 3).unwrap() as *const RzString, identity);
    assert_eq!(text.bytes(), b"aXYef");
}

#[test]
fn rz07_replace_object_erases_count_but_scans_source() {
    let mut text = s(b"abcdef");
    text.replace_string(1, &s(b"X\0Y")).unwrap();
    assert_eq!(text.bytes(), b"aXef");
}

#[test]
fn rz08_sensitive_comparison_stops_at_nul() {
    let a = s(b"A\0X");
    let b = s(b"A\0Y");
    assert_eq!(a.is_equal_string(&b, true), Ok(true));
    assert_eq!(a.compare_string(&b, true), Ok(0));
    assert_eq!(a.is_equal_string(&b, false), Ok(false));
    assert_ne!(a.compare_string(&b, false), Ok(0));
    assert_eq!(a.is_equal(b"A\0Y", 3, true), Ok(true));
    assert_eq!(a.compare_to(b"A\0Y", 3, false), Ok(-1));
}

#[test]
fn rz09_original_length_difference() {
    assert_eq!(s(b"ab").compare_string(&s(b"abcd"), true), Ok(-2));
    assert_eq!(s(b"AB").compare_to(b"abcd\0ignored", u32::MAX, false), Ok(-2));
    assert_eq!(s(b"A\0X").is_equal(b"A\0X", u32::MAX, true), Ok(false));
}

#[test]
fn rz10_search_counted_haystack() {
    assert_eq!(s(b"a\0bc").find(b"bc", 0, true), Ok(2));
}

#[test]
fn rz11_search_object_needle_scans_to_nul() {
    assert_eq!(s(b"abcd").find_string(&s(b"b\0z"), 0, true), Ok(1));
    assert_eq!(s(b"abcd").rfind_string(&s(b"b\0z"), u32::MAX, true), Ok(1));
}

#[test]
fn rz12_search_positions_and_failure() {
    let text = s(b"ababa");
    assert_eq!(text.find(b"ba", 0, true), Ok(1));
    assert_eq!(text.rfind(b"ba", u32::MAX, true), Ok(3));
    assert_eq!(text.find(b"x", 0, true), Ok(NOT_FOUND));
    assert_eq!(text.find(b"BA", 2, false), Ok(3));
    assert_eq!(text.rfind(b"BA", 2, false), Ok(1));
    assert_eq!(text.rfind(b"ababaa", u32::MAX, true), Ok(NOT_FOUND));
}

#[test]
fn rz13_empty_needle() {
    let text = s(b"ab");
    assert_eq!(text.find(b"", 2, true), Ok(2));
    assert_eq!(text.find(b"", 3, true), Ok(NOT_FOUND));
    assert_eq!(text.rfind(b"", 99, true), Ok(2));
    assert_eq!(text.rfind(b"", 0, true), Ok(0));
}

#[test]
fn rz14_trim_space_and_tab_only() {
    let mut text = s(b" \tA\r\n \t");
    text.trim();
    assert_eq!(text.bytes(), b"A\r\n");
    let mut all = s(b" \t \t");
    all.trim();
    assert_eq!(all.bytes(), b"");
    RzString::new().trim();
}

#[test]
fn rz15_substrings_are_independent_bytes() {
    let text = s(b"abcd");
    assert_eq!(text.mid(2, 99).bytes(), b"cd");
    assert_eq!(text.mid(4, 1).bytes(), b"");
    assert_eq!(text.mid(99, 1).bytes(), b"");
    assert_eq!(text.left(99).bytes(), b"abcd");
    assert_eq!(text.right(2).bytes(), b"cd");
    let mut copy = text.right(99);
    copy.to_char_mut()[0] = b'X';
    assert_eq!(text.bytes(), b"abcd");
    assert_eq!(s("é".as_bytes()).left(1).bytes(), b"\xc3");
}

#[test]
fn rz16_literal_split() {
    let mut text = s(b"a::b");
    assert_eq!(text.split_token(Some(b"::")).bytes(), b"a");
    assert_eq!(text.bytes(), b"b");
    assert_eq!(text.split_token(Some(b"::")).bytes(), b"");
    assert_eq!(text.bytes(), b"b");
    assert_eq!(text.split_token(None).bytes(), b"b");
    assert_eq!(text.split_token(Some(b"")).bytes(), b"b");
    assert_eq!(text.bytes(), b"b");
}

#[test]
fn rz17_ascii_case_includes_interior_nul() {
    let mut text = s(b"Az\0B");
    text.make_lower().unwrap();
    assert_eq!(text.bytes(), b"az\0b");
    text.make_upper().unwrap();
    assert_eq!(text.bytes(), b"AZ\0B");
}

#[test]
fn rz18_sprintf_assignment_and_identity() {
    let mut text = s(b"nonempty");
    let identity = &text as *const RzString;
    assert_eq!(
        text.sprintf(b"x=%s %%", &[FormatArg::Bytes(b"ok")], &Numbers).unwrap() as *const RzString,
        identity
    );
    assert_eq!(text.bytes(), b"x=ok %");
    assert_eq!(text.sprintf(b"", &[], &Numbers).unwrap() as *const RzString, identity);
    assert_eq!(text.bytes(), b"");
}

#[test]
fn rz19_sprintf_padding() {
    let mut text = RzString::new();
    text.sprintf(b"%04u", &[FormatArg::Unsigned(7)], &Numbers).unwrap();
    assert_eq!(text.bytes(), b"0007");
    text.sprintf(b"%-4s", &[FormatArg::Bytes(b"ab")], &Numbers).unwrap();
    assert_eq!(text.bytes(), b"ab  ");
}

#[test]
fn rz20_prefix_suffix_counted() {
    let text = s(b"A\0X");
    assert!(text.begins_with(b"A\0"));
    assert!(!text.ends_with(b"\0Y"));
    assert!(text.ends_with(b"\0X"));
    assert!(text.begins_with(b""));
    assert!(text.ends_with(b""));
    assert!(!text.begins_with(b"A\0XX"));
}

#[test]
fn string_inventory_wrappers_and_range_errors() {
    let mut text = RzString::from_c_string(b"ab\0x");
    let identity = &text as *const RzString;
    assert_eq!(text.assign(&s(b"Q")) as *const RzString, identity);
    text.append_string(&s(b"\0R"));
    text.insert_string(3, &s(b"end")).unwrap();
    assert_eq!(text.bytes(), b"Q\0Rend");
    text.strcat(b"A\0B");
    assert_eq!(text.bytes(), b"Q\0RendA");
    text.strncpy(b"x\0y", 99);
    assert_eq!(text.bytes(), b"x");
    text.strncpy(b"abcdef", 3);
    assert_eq!(text.bytes(), b"abc");
    assert!(matches!(text.insert(4, b"x"), Err(StringFault::Range)));
    assert!(matches!(text.erase(4, 0), Err(StringFault::Range)));
    assert!(matches!(text.replace(4, b"x", 0), Err(StringFault::Range)));
    assert_eq!(text.bytes(), b"abc");
    text.erase(3, 99).unwrap();
    text.replace(3, b"x", 99).unwrap();
    assert_eq!(text.bytes(), b"abcx");
}

#[test]
fn dt01_base_literals_and_lifecycle() {
    let mut d = Detokenizer::new();
    assert_eq!(d.html_clean_mode(), 0);
    assert_eq!(d.ref_count(), 0);
    assert!(d.init());
    assert!(d.shutdown());
    expect_parse(&mut d, b"a<b", true, b"a<b");
    d.set_html_clean_mode(7);
    assert_eq!(d.html_clean_mode(), 7);
    assert!(d.init());
    assert!(d.shutdown());
    assert_eq!(d.html_clean_mode(), 7);
    expect_parse(&mut d, b"a<b", true, b"a<b");
    assert_eq!(d.release(), Released::Destroy);
    assert!(d.query_interface(IID_DETOKENIZER));
    assert!(d.query_interface(IID_UNKNOWN));
    assert_eq!(d.release(), Released::Alive(1));
    assert_eq!(d.release(), Released::Destroy);
}

#[test]
fn dt02_case_insensitive_trimmed_lookup() {
    let mut d = Detokenizer::new();
    value(&mut d, b"City", b"Oak");
    expect_parse(&mut d, b"Hi # city #!", true, b"Hi Oak!");
    value(&mut d, b" A ", b"not parsed");
    expect_parse(&mut d, b"# A #", false, b"A");
}

#[test]
fn dt03_values_win_and_replace_equivalent_keys() {
    let mut d = Detokenizer::new();
    value(&mut d, b"City", b"Oak");
    value(&mut d, b"CITY", b"Elm");
    d.set_token_replacement_method(
        &s(b"city"),
        ReplacementMethod::new(1, 0, |_, _, _| panic!("value wins")),
    )
    .unwrap();
    expect_parse(&mut d, b"#City#", true, b"Elm");
}

#[test]
fn dt04_unknown_and_unmatched_suffix() {
    expect_parse(&mut Detokenizer::new(), b"a# missing #b#tail", false, b"amissingb#tail");
}

#[test]
fn dt05_replacement_is_not_recursive() {
    let mut d = Detokenizer::new();
    value(&mut d, b"A", b"#B#");
    value(&mut d, b"B", b"done");
    expect_parse(&mut d, b"#A#", true, b"#B#");
}

#[test]
fn dt06_global_hash_protection() {
    let mut d = Detokenizer::new();
    for (input, success, output) in [
        (&b"##"[..], true, &b"#"[..]),
        (b"####", true, b"##"),
        (b"###", false, b""),
        (b"a#tail", true, b"a#tail"),
    ] {
        expect_parse(&mut d, input, success, output);
    }
    // Global pairs prevent interpreting adjacent #A##B# as two tokens.
    value(&mut d, b"A\0B", b"combined");
    expect_parse(&mut d, b"#A##B#", true, b"combined");
}

#[test]
fn dt07_false_callback_output_shared_and_named_method_replaced() {
    let mut d = Detokenizer::new();
    let trace = Arc::new(Mutex::new(Vec::new()));
    d.set_token_replacement_method(&s(b"N"), ReplacementMethod::new(1, 0, |_, _, _| panic!("replaced")))
        .unwrap();
    let seen = trace.clone();
    d.set_token_replacement_method(
        &s(b"n"),
        ReplacementMethod::new(2, 10, move |token, out, ctx| {
            assert_eq!(token.bytes(), b"N");
            assert_eq!(ctx, 10);
            seen.lock().unwrap().push("N");
            out.set_c_string(Some(b"x"));
            false
        }),
    )
    .unwrap();
    let seen = trace.clone();
    d.add_unknown_token_replacement_method(
        ReplacementMethod::new(3, 20, move |_, out, ctx| {
            assert_eq!(out.bytes(), b"x");
            assert_eq!(ctx, 20);
            seen.lock().unwrap().push("U");
            out.set_c_string(Some(b"y"));
            true
        }),
        true,
    );
    expect_parse(&mut d, b"#N#", true, b"y");
    assert_eq!(*trace.lock().unwrap(), ["N", "U"]);
}

#[test]
fn dt08_unknown_callbacks_follow_unsigned_identity_order() {
    let mut d = Detokenizer::new();
    d.add_unknown_token_replacement_method(
        ReplacementMethod::new(0x2000, 0, |_, _, _| panic!("second not reached")),
        true,
    );
    d.add_unknown_token_replacement_method(
        ReplacementMethod::new(0x1000, 0, |_, out, _| {
            out.set_c_string(Some(b"first"));
            true
        }),
        true,
    );
    expect_parse(&mut d, b"#X#", true, b"first");
}

#[test]
fn dt09_unknown_identity_keeps_first_context_and_removes_by_address() {
    let mut d = Detokenizer::new();
    let callback = |_: &RzString, out: &mut RzString, ctx| {
        assert_eq!(ctx, 1);
        out.set_c_string(Some(b"context1"));
        true
    };
    d.add_unknown_token_replacement_method(ReplacementMethod::new(42, 1, callback), true);
    d.add_unknown_token_replacement_method(ReplacementMethod::new(42, 2, callback), true);
    expect_parse(&mut d, b"#X#", true, b"context1");
    d.add_unknown_token_replacement_method(ReplacementMethod::new(42, 3, callback), false);
    expect_parse(&mut d, b"#X#", false, b"X");
}

#[test]
fn dt10_final_fallback_overwrites_failed_callback() {
    let mut d = Detokenizer::new();
    d.add_unknown_token_replacement_method(
        ReplacementMethod::new(1, 0, |_, out, _| {
            out.set_c_string(Some(b"junk"));
            false
        }),
        true,
    );
    expect_parse(&mut d, b"# X #", false, b"X");
}

#[test]
fn dt11_html_only_selected_replacements_and_single_pass() {
    let mut d = Detokenizer::new();
    d.set_html_clean_mode(1);
    value(&mut d, b"A", b"<&\"");
    expect_parse(&mut d, b"<b>#A#</b>", true, b"<b>&lt;&amp;&quot;</b>");
    value(&mut d, b"A", b"&lt;>'");
    expect_parse(&mut d, b"#A#", true, b"&amp;lt;&gt;'");
    d.add_unknown_token_replacement_method(
        ReplacementMethod::new(1, 0, |_, out, _| {
            out.set_c_string(Some(b"<"));
            true
        }),
        true,
    );
    expect_parse(&mut d, b"#B#", true, b"&lt;");
}

#[test]
fn dt12_html_unresolved_name_still_false() {
    let mut d = Detokenizer::new();
    d.set_html_clean_mode(255);
    expect_parse(&mut d, b"# <x> #", false, b"&lt;x&gt;");
}

#[test]
fn dt13_html_scans_nul_and_parser_restores_nuls() {
    let mut d = Detokenizer::new();
    value(&mut d, b"A", b"X\0Y");
    expect_parse(&mut d, b"#A#", true, b"X#Y");
    d.set_html_clean_mode(1);
    expect_parse(&mut d, b"#A#", true, b"X");
}

#[test]
fn dt14_nul_restoration_only_on_hash_path() {
    let mut d = Detokenizer::new();
    expect_parse(&mut d, b"A\0B", true, b"A\0B");
    expect_parse(&mut d, b"A\0B#", true, b"A#B#");
}

#[test]
fn dt15_aliased_input_output() {
    let mut d = Detokenizer::new();
    value(&mut d, b"A", b"v");
    let mut text = s(b"# A #");
    assert_eq!(d.detokenize_in_place(&mut text), Ok(true));
    assert_eq!(text.bytes(), b"v");
    text = s(b"A\0B");
    assert_eq!(d.detokenize_in_place(&mut text), Ok(true));
    assert_eq!(text.bytes(), b"A\0B");
}

struct Resources {
    success: bool,
    returned: Option<RzString>,
    trace: Vec<&'static str>,
}
impl ResourceStrings for Resources {
    type Key = [u32; 3];
    type Handle = RzString;
    fn retrieve(&mut self, key: &[u32; 3], iid: u32, a: u32, b: u32) -> StringRetrieval<RzString> {
        assert_eq!(*key, [1, 2, 3]);
        assert_eq!((iid, a, b), (IID_STRING, 0, 0));
        self.trace.push("retrieve");
        StringRetrieval {
            success: self.success,
            interface: self.returned.take(),
        }
    }
    fn release(&mut self, _: RzString) {
        self.trace.push("release");
    }
}

#[test]
fn dt16_resource_failure_preserves_output_and_releases_nonnull() {
    let mut d = Detokenizer::new();
    d.add_unknown_token_replacement_method(
        ReplacementMethod::new(1, 0, |_, _, _| panic!("no parser on failure")),
        true,
    );
    for returned in [None, Some(s(b"#X#"))] {
        let present = returned.is_some();
        let mut resources = Resources {
            success: false,
            returned,
            trace: Vec::new(),
        };
        let mut output = s(b"keep");
        assert_eq!(
            d.detokenize_resource(&mut resources, &[1, 2, 3], &mut output),
            Ok(false)
        );
        assert_eq!(output.bytes(), b"keep");
        assert_eq!(
            resources.trace,
            if present {
                vec!["retrieve", "release"]
            } else {
                vec!["retrieve"]
            }
        );
    }
}

#[test]
fn dt17_query_failure_preserves_sentinel() {
    let mut output = 123;
    let mut text = RzString::new();
    assert!(!text.query_interface_into(99, &mut output, 7));
    assert_eq!(output, 123);
    assert_eq!(text.ref_count(), 0);
    let mut d = Detokenizer::new();
    assert!(!d.query_interface_into(99, &mut output, 8));
    assert_eq!(output, 123);
    assert_eq!(d.ref_count(), 0);
    assert!(text.query_interface_into(IID_STRING, &mut output, 7));
    assert_eq!(output, 7);
    assert!(d.query_interface_into(IID_DETOKENIZER, &mut output, 8));
    assert_eq!(output, 8);
}

#[test]
fn dt18_all_tokens_processed_after_failure() {
    let mut d = Detokenizer::new();
    value(&mut d, b"A", b"value-of-A");
    d.set_token_replacement_method(&s(b"X"), ReplacementMethod::new(1, 0, |_, _, _| false))
        .unwrap();
    expect_parse(&mut d, b"#X# / #A#", false, b"X / value-of-A");
}

#[test]
fn dt19_first_protected_hash_remains_opener() {
    let mut d = Detokenizer::new();
    value(&mut d, b"A", b"v");
    expect_parse(&mut d, b"##A#", true, b"v");
}

#[test]
fn virtual_parser_converter_and_resource_dispatch() {
    struct Subclass {
        parser_calls: u32,
        converter_calls: u32,
    }
    impl DetokenizerDispatch for Subclass {
        fn perform_detokenize(&mut self, input: &RzString, out: &mut RzString) -> Result<bool, StringFault> {
            self.parser_calls += 1;
            perform_detokenize(self, input, out)
        }
        fn convert_token(&mut self, token: &RzString, out: &mut RzString) -> Result<bool, StringFault> {
            self.converter_calls += 1;
            out.copy(token);
            out.append(b"!");
            Ok(false)
        }
    }
    let mut d = Subclass {
        parser_calls: 0,
        converter_calls: 0,
    };
    expect_parse(&mut d, b"#A#", false, b"A!");
    let mut resources = Resources {
        success: true,
        returned: Some(s(b"#B#")),
        trace: Vec::new(),
    };
    let mut out = s(b"old");
    assert_eq!(d.detokenize_resource(&mut resources, &[1, 2, 3], &mut out), Ok(false));
    assert_eq!(out.bytes(), b"B!");
    assert_eq!((d.parser_calls, d.converter_calls), (2, 2));
    assert_eq!(resources.trace, ["retrieve", "release"]);
}

#[test]
fn registration_copies_names_and_values() {
    let mut d = Detokenizer::new();
    let mut name = s(b"A");
    let mut text = s(b"value");
    d.set_token_replacement_value(&name, &text).unwrap();
    name.to_char_mut()[0] = b'B';
    text.to_char_mut()[0] = b'X';
    expect_parse(&mut d, b"#A#", true, b"value");
    expect_parse(&mut d, b"#B#", false, b"B");
}

#[test]
fn settled_format_syntax_modifiers_and_multibyte_precision() {
    let mut text = RzString::new();
    text.sprintf(
        b"%hu %lu %llu %I64X %o %x %d %i %c",
        &[
            FormatArg::Unsigned(65537),
            FormatArg::Unsigned(4294967297),
            FormatArg::Unsigned(4294967297),
            FormatArg::Unsigned(4294967297),
            FormatArg::Unsigned(9),
            FormatArg::Unsigned(31),
            FormatArg::Signed(42),
            FormatArg::Signed(43),
            FormatArg::Byte(b'!'),
        ],
        &Numbers,
    )
    .unwrap();
    assert_eq!(text.bytes(), b"1 1 4294967297 100000001 11 1f 42 43 !");
    text.sprintf(
        b"%*.*s",
        &[
            FormatArg::Signed(4),
            FormatArg::Signed(1),
            FormatArg::Bytes("éx".as_bytes()),
        ],
        &Numbers,
    )
    .unwrap();
    assert_eq!(text.bytes(), "   é".as_bytes());
    text.sprintf(
        b"%-04u|%+ d|%#x|%#o|%.3u",
        &[
            FormatArg::Unsigned(7),
            FormatArg::Signed(8),
            FormatArg::Unsigned(31),
            FormatArg::Unsigned(9),
            FormatArg::Unsigned(7),
        ],
        &Numbers,
    )
    .unwrap();
    assert_eq!(text.bytes(), b"7   |+8|0x1f|011|007");
}

#[test]
fn formatting_numeric_primitives_are_unpadded_and_receiver_assigned_last() {
    struct Primitive;
    impl FormatPrimitives for Primitive {
        fn float_digits(&self, c: u8, v: f64, p: Option<u32>, alternate: bool) -> Result<Vec<u8>, StringFault> {
            assert!(matches!(c, b'e' | b'E' | b'f' | b'g' | b'G'));
            assert_eq!((v, p, alternate), (1.5, Some(2), true));
            Ok(b"1.50".to_vec())
        }
        fn pointer_digits(&self, v: u32, p: Option<u32>) -> Result<Vec<u8>, StringFault> {
            assert_eq!((v, p), (0x1234, None));
            Ok(b"1234".to_vec())
        }
    }
    let mut text = s(b"old");
    for c in *b"eEfgG" {
        let format = [b'%', b'#', b'+', b'0', b'8', b'.', b'2', c, b'|', b'%', b'6', b'p'];
        text.sprintf(
            &format,
            &[FormatArg::Float(1.5), FormatArg::Pointer(0x1234)],
            &Primitive,
        )
        .unwrap();
        assert_eq!(text.bytes(), b"+0001.50|  1234");
    }
}

#[test]
fn open_boundaries_are_explicit_and_do_not_fabricate_results() {
    let mut text = s(b"\x7fA");
    assert!(matches!(text.make_lower(), Err(StringFault::Unsupported("RZS2"))));
    assert_eq!(text.bytes(), b"\x7fA");
    assert_eq!(text.find(b"a", 0, false), Err(StringFault::Unsupported("RZS2")));
    assert!(matches!(
        text.split_token_delimited(),
        Err(StringFault::Unsupported("RZS4"))
    ));
    assert!(matches!(
        text.split_token_separated(),
        Err(StringFault::Unsupported("RZS4"))
    ));
    for (format, arg) in [
        (&b"%d"[..], FormatArg::Signed(-1)),
        (b"%ls", FormatArg::Bytes(b"x")),
        (b"%c", FormatArg::Byte(0)),
        (b"%n", FormatArg::Pointer(0)),
        (b"%*s", FormatArg::Signed(-1)),
        (b"%s", FormatArg::Bytes(b"\xff")),
    ] {
        assert!(matches!(
            text.sprintf(format, &[arg], &Numbers),
            Err(StringFault::Unsupported("RZS5"))
        ));
        assert_eq!(text.bytes(), b"\x7fA");
    }
    let mut d = Detokenizer::new();
    assert_eq!(
        d.set_token_replacement_value(&s(b"\xff"), &s(b"x")),
        Err(StringFault::Unsupported("RZS2"))
    );
}
