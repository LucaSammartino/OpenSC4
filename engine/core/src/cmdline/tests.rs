//! Tests from docs/specs/command-line.md §8 (conformance) plus edge cases from §2-§6.

use super::*;

fn cl(args: &[&str]) -> CmdLine {
    CmdLine::from_parts(b"", args.iter().map(|a| a.as_bytes()))
}

fn strs(v: &[Vec<u8>]) -> Vec<String> {
    v.iter().map(|a| String::from_utf8(a.clone()).unwrap()).collect()
}

// §8.1
#[test]
fn split_conformance() {
    let t = b"  \"C:\\Games\\SimCity 4.exe\"  -intro:off\t-w ";
    assert_eq!(strs(&split(t)), ["C:\\Games\\SimCity 4.exe", "-intro:off", "-w"]);
    assert_eq!(strs(&split(b"a\"b c\"d e")), ["a\"b c\"d", "e"]);
    assert_eq!(strs(&split(b"\"\"")), [""]);
    assert_eq!(strs(&split(b"x\ry")), ["x\ry"]);
    assert!(split(b"").is_empty());
    assert!(split(b" \t\n ").is_empty());
}

#[test]
fn split_quote_rules() {
    assert_eq!(strs(&split(b"\"x y\"")), ["x y"]);
    assert_eq!(strs(&split(b"\"")), [""]);
    assert_eq!(strs(&split(b"\"abc")), ["\"abc"]);
    // Odd quote count: the separator is part of the argument to the end.
    assert_eq!(strs(&split(b"a\"b c d")), ["a\"b c d"]);
    // A separator run is one break; the count restarts per argument.
    assert_eq!(strs(&split(b"a\"\"   \"b c\"")), ["a\"\"", "b c"]);
    // Only a leading and trailing quote pair is removed.
    assert_eq!(strs(&split(b"\"a\"b\"")), ["a\"b"]);
    // \r is ordinary even at the ends.
    assert_eq!(strs(&split(b"\rx\r")), ["\rx\r"]);
}

#[test]
fn new_keeps_text_verbatim() {
    let c = CmdLine::new(b"  a  b ");
    assert_eq!(c.get_command_line(), b"  a  b ");
    assert_eq!(c.argc(), 2);
}

// §8.2
#[test]
fn join_conformance() {
    let mut c = cl(&["p", "-w"]);
    c.insert_argument(b"a b", 99);
    assert_eq!(c.get_command_line(), b"p -w \"a b\"");
    c.erase_argument(99).unwrap();
    assert_eq!(strs(c.args()), ["p", "-w"]);
    assert_eq!(c.get_command_line(), b"p -w");
    let mut x = cl(&["x"]);
    x.insert_argument(b"", 0);
    assert_eq!(x.get_command_line(), b" x");
}

#[test]
fn join_edges() {
    assert_eq!(
        join(&[b"a\"b".to_vec(), b"t\tu".to_vec(), b"n\nl".to_vec()]),
        b"a\"b \"t\tu\" \"n\nl\""
    );
    assert_eq!(join(&[b"a".to_vec(), Vec::new(), b"b".to_vec()]), b"a  b");
    assert_eq!(join(&[b"c\rr".to_vec()]), b"c\rr");
    assert!(join(&[]).is_empty());
}

#[test]
fn insert_clamps() {
    let mut c = cl(&[]);
    c.insert_argument(b"a", 5);
    assert_eq!(strs(c.args()), ["a"]);
    c.insert_argument(b"b", -3);
    assert_eq!(strs(c.args()), ["b", "a"]);
    c.insert_argument(b"c", 1);
    assert_eq!(strs(c.args()), ["b", "c", "a"]);
    c.insert_argument(b"d", 3);
    assert_eq!(strs(c.args()), ["b", "c", "a", "d"]);
}

#[test]
fn erase_safe_errors() {
    let mut e = cl(&[]);
    assert_eq!(e.erase_argument(0), Err(CmdLineError::EraseFromEmpty));
    assert_eq!(e.erase_argument(-1), Err(CmdLineError::EraseFromEmpty));
    let mut c = CmdLine::new(b"a  b");
    assert_eq!(c.erase_argument(-1), Err(CmdLineError::NegativeIndex(-1)));
    assert_eq!(strs(c.args()), ["a", "b"]);
    assert_eq!(c.get_command_line(), b"a  b"); // unchanged, not rejoined
    c.erase_argument(0).unwrap();
    assert_eq!(c.get_command_line(), b"b");
    c.erase_argument(7).unwrap();
    assert_eq!(c.argc(), 0);
    assert!(c.get_command_line().is_empty());
}

// §8.3
#[test]
fn value_switch_conformance() {
    let c = cl(&["p", "-INTRO:Off", "/size:\"1 2\"", "-q:'z\""]);
    let mut v = Vec::new();
    assert!(c.is_switch_present_value(b"intro", &mut v, true));
    assert_eq!(v, b"Off");
    assert!(c.is_switch_present_value(b"size", &mut v, true));
    assert_eq!(v, b"1 2");
    assert!(c.is_switch_present_value(b"size", &mut v, false));
    assert_eq!(v, b"\"1 2\"");
    assert!(c.is_switch_present_value(b"q", &mut v, true));
    assert_eq!(v, b"z");
    v = b"keep".to_vec();
    assert!(!c.is_switch_present_value(b"intro:", &mut v, true));
    assert_eq!(v, b"keep");
    let d = cl(&["-intro:"]);
    assert!(!d.is_switch_present_value(b"intro", &mut v, true));
    assert_eq!(v, b"keep");
}

#[test]
fn value_switch_edges() {
    let mut v = b"keep".to_vec();
    // Empty name: false.
    assert!(!cl(&["-:x"]).is_switch_present_value(b"", &mut v, true));
    // One quote character: empty value (the original throws), no crash.
    assert!(cl(&["-n:\""]).is_switch_present_value(b"n", &mut v, true));
    assert!(v.is_empty());
    assert!(cl(&["-n:'"]).is_switch_present_value(b"n", &mut v, false));
    assert_eq!(v, b"'");
    // Backslash prefix; not a switch without a prefix; first match wins.
    let c = cl(&["n:a", "\\N:b", "-n:c"]);
    assert!(c.is_switch_present_value(b"n", &mut v, true));
    assert_eq!(v, b"b");
    // Quote only at one end stays.
    assert!(cl(&["-n:\"ab"]).is_switch_present_value(b"n", &mut v, true));
    assert_eq!(v, b"\"ab");
}

// §8.4
#[test]
fn bare_switch_conformance() {
    let c = cl(&["p", "-Intro", "/w", "-width"]);
    assert_eq!(c.get_index_of_switch(b"intro", 0), 1);
    assert_eq!(c.get_index_of_switch(b"intro", 2), -1);
    assert_eq!(c.get_index_of_switch_char(b'W', 0), 2);
    assert_eq!(c.get_index_of_switch_char(b'w', 3), -1);
    assert_eq!(c.get_index_of_switch(b"width", -5), 3);
    assert!(c.is_switch_present(b"intro"));
}

#[test]
fn bare_switch_edges() {
    let c = cl(&["-intro:off", "-x"]);
    assert_eq!(c.get_index_of_switch(b"intro", 0), -1);
    assert_eq!(c.get_index_of_switch(b"x", 99), -1);
    assert_eq!(c.get_index_of_switch_char(b'X', -1), 1);
    // Slot 9 falls back to the value form.
    assert!(c.is_switch_present(b"intro"));
    assert!(!c.is_switch_present(b"other"));
    // Slot 9 on a one-quote value does not crash.
    assert!(cl(&["-n:\""]).is_switch_present(b"n"));
    assert!(!cl(&["-n:"]).is_switch_present(b"n"));
    // Empty name, literal reading (CL2): a lone prefix matches.
    assert_eq!(cl(&["a", "-"]).get_index_of_switch(b"", 0), 1);
}

// §8.5
#[test]
fn letter_value_conformance() {
    let mut v = b"keep".to_vec();
    assert!(cl(&["-w"]).is_switch_present_char(b'w', &mut v, true));
    assert_eq!(v, b"keep");
    assert!(cl(&["-W1024"]).is_switch_present_char(b'w', &mut v, true));
    assert_eq!(v, b"1024");
    assert!(cl(&["-w\"x y\""]).is_switch_present_char(b'w', &mut v, true));
    assert_eq!(v, b"x y");
    assert!(cl(&["-w\"\""]).is_switch_present_char(b'w', &mut v, true));
    assert!(v.is_empty());
    assert!(cl(&["-intro"]).is_switch_present_char(b'i', &mut v, false));
    assert_eq!(v, b"ntro");
    v = b"keep".to_vec();
    assert!(!cl(&["w"]).is_switch_present_char(b'w', &mut v, true));
    assert_eq!(v, b"keep");
}

#[test]
fn letter_value_edges() {
    let mut v = Vec::new();
    // Single quotes are not removed by slot 10; unquote false keeps double quotes.
    assert!(cl(&["-w'a'"]).is_switch_present_char(b'w', &mut v, true));
    assert_eq!(v, b"'a'");
    assert!(cl(&["-w\"a\""]).is_switch_present_char(b'w', &mut v, false));
    assert_eq!(v, b"\"a\"");
    // Three characters with a quote: too short to unquote.
    assert!(cl(&["-w\""]).is_switch_present_char(b'w', &mut v, true));
    assert_eq!(v, b"\"");
    assert!(cl(&["-intro:off"]).is_switch_present_char(b'I', &mut v, true));
    assert_eq!(v, b"ntro:off");
    // A lone prefix is too short.
    assert!(!cl(&["-"]).is_switch_present_char(b'-', &mut v, true));
}

// §8.6
#[test]
fn find_conformance() {
    let c = cl(&["p", "-Intro:Off"]);
    assert_eq!(c.find(b"-intro:off", false), 1);
    assert_eq!(c.find(b"-intro:off", true), -1);
    assert_eq!(c.find_sub_string(b"ntro", true), 1);
    assert_eq!(c.find_sub_string(b"", true), 0);
}

#[test]
fn find_edges() {
    let c = cl(&["", "Ab", "xAB"]);
    assert_eq!(c.find(b"", true), 0);
    assert_eq!(c.find_sub_string(b"", false), 1);
    assert_eq!(c.find_sub_string(b"ab", true), -1);
    assert_eq!(c.find_sub_string(b"ab", false), 1);
    assert_eq!(c.find_sub_string(b"AB", true), 2);
    assert_eq!(c.find_sub_string(b"xABy", true), -1);
    assert_eq!(cl(&[]).find_sub_string(b"", true), -1);
    assert_eq!(cl(&[""]).find_sub_string(b"", true), -1);
}

// §8.7
#[test]
fn bounds_conformance() {
    let c = cl(&["a", "b"]);
    assert_eq!(c.argv(2), Ok(&b""[..]));
    assert_eq!(c.argv(1000), Ok(&b""[..]));
    assert_eq!(c.argv(1), Ok(&b"b"[..]));
    assert_eq!(c.get_argument(0), Ok(&b"a"[..]));
    assert_eq!(c.argv(-1), Err(CmdLineError::NegativeIndex(-1)));
    assert_eq!(c.get_argument(i32::MIN), Err(CmdLineError::NegativeIndex(i32::MIN)));
    assert_eq!(CmdLine::new(b"   ").argc(), 0);
}

// §8.8
#[test]
fn refcount_conformance() {
    let mut c = CmdLine::new(b"x");
    assert_eq!(c.ref_count(), 0);
    assert_eq!(c.add_ref(), 1);
    assert_eq!(c.release(), 0); // the caller now destroys it (slot 15)
    let mut c = CmdLine::new(b"x");
    assert!(c.query_interface(IID_IGZCMDLINE));
    assert!(c.query_interface(IID_IGZUNKNOWN));
    assert!(!c.query_interface(0x1234));
    assert_eq!(c.ref_count(), 2);
    assert_eq!(c.release(), 1);
}

#[test]
fn copy_does_not_resplit() {
    let mut src = CmdLine::new(b"a b");
    src.add_ref();
    let dst = CmdLine::from_parts(b"not split", src.args().iter().map(|a| a.as_slice()));
    assert_eq!(strs(dst.args()), ["a", "b"]);
    assert_eq!(dst.get_command_line(), b"not split");
    let cp = CmdLine::copy_of(&src);
    assert_eq!(cp.ref_count(), 0);
    assert_eq!(cp.get_command_line(), b"a b");
    assert_eq!(cp.args(), src.args());
}
