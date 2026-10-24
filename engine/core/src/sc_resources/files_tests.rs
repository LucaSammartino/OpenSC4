// SPDX-License-Identifier: GPL-3.0-or-later
//! SCR11 section 12.3: routing, physical counts and invocation-local state.
use super::*;
use std::collections::{BTreeMap, VecDeque};

#[derive(Default)]
struct Source {
    files: BTreeMap<String, Vec<Vec<u8>>>,
    opened: Vec<String>,
    finished: Vec<(String, u32, Result<u32>)>,
}
struct Input(VecDeque<Vec<u8>>);
impl Lines for Input {
    fn read_line(&mut self) -> Option<Vec<u8>> {
        self.0.pop_front()
    }
}
impl Files for Source {
    fn open(&mut self, path: &str) -> Option<Box<dyn Lines>> {
        self.opened.push(path.into());
        self.files
            .get(path)
            .map(|v| Box::new(Input(v.clone().into())) as Box<dyn Lines>)
    }
    fn watch(&mut self, _: &str, _: u32, _: u32) -> Option<Box<dyn Watcher>> {
        None
    }
    fn finished(&mut self, path: &str, physical: u32, result: &Result<u32>) {
        self.finished.push((path.into(), physical, result.clone()));
    }
}
impl Source {
    fn add(&mut self, path: &str, lines: &[&str]) {
        self.files
            .insert(path.into(), lines.iter().map(|s| s.as_bytes().to_vec()).collect());
    }
}

fn routing(lines: &[&str], expected: &[(u32, &str)]) {
    let mut source = Source::default();
    source.add("a", lines);
    let mut actual = Vec::new();
    let count = parse_file_with(&mut source, "a", &mut |_, physical, payload, _| {
        actual.push((physical, payload.to_vec()));
        Ok(())
    });
    assert_eq!(count, Ok(lines.len() as u32));
    assert_eq!(
        actual,
        expected
            .iter()
            .map(|(n, s)| (*n, s.as_bytes().to_vec()))
            .collect::<Vec<_>>()
    );
    assert_eq!(source.opened, ["a"]);
    assert_eq!(source.finished, [("a".into(), lines.len() as u32, count)]);
}

#[test]
fn scr_v93_basic() {
    routing(&["#<", "plain explanation", "#> set x y"], &[(3, " set x y")]);
}
#[test]
fn scr_v94_opener_erases_later_closer_and_command_prefix() {
    routing(&["set x y #< #> set z w", "set a b", "#>set c d"], &[(3, "set c d")]);
}
#[test]
fn scr_v95_closer_before_opener_activates_then_closes() {
    routing(&["#>set x y #< tail", "set a b"], &[(1, "set x y "), (2, "set a b")]);
}
#[test]
fn scr_v96_no_nested_depth() {
    routing(&["#<", "#<", "#>set x y", "set a b"], &[(3, "set x y"), (4, "set a b")]);
}
#[test]
fn scr_v97_repeated_opener_discards_its_closer() {
    routing(&["#<", "#< #>", "set x y", "#>", "set a b"], &[(4, ""), (5, "set a b")]);
}
#[test]
fn scr_v98_suppressed_include_bad_syntax_blank_and_unclosed_eof() {
    routing(&["#<", "include missing.sgr", "\"(", ""], &[]);
}
#[test]
fn scr_v99_spaced_opener_stray_closer_and_blank() {
    routing(
        &["  # <", "set x y #>", ""],
        &[(1, "  # <"), (2, "set x y #>"), (3, "")],
    );
}
#[test]
fn scr_v100_quotes_do_not_protect_markers() {
    routing(&["set x \"#<\"", "#>set a b"], &[(2, "set a b")]);
}
#[test]
fn scr_v101_line_comments_do_not_protect_markers() {
    routing(&["# note #<", "include missing.sgr", "#>set a b"], &[(3, "set a b")]);
}
#[test]
fn scr_v102_child_block_state_and_inclusive_counts() {
    let mut source = Source::default();
    source.add("d/a", &["#<", "#>include child.sgr", "set a b"]);
    source.add("d/child.sgr", &["set c d", "#<", "ignored"]);
    let mut actual = Vec::new();
    let result = parse_file_with(&mut source, "d/a", &mut |path, n, line, _| {
        actual.push((path.to_owned(), n, line.to_vec()));
        Ok(())
    });
    assert_eq!(result, Ok(6));
    assert_eq!(
        actual,
        [
            ("d/child.sgr".into(), 1, b"set c d".to_vec()),
            ("d/a".into(), 3, b"set a b".to_vec())
        ]
    );
    assert_eq!(source.opened, ["d/a", "d/child.sgr"]);
    assert_eq!(
        source.finished,
        [("d/child.sgr".into(), 3, Ok(3)), ("d/a".into(), 3, Ok(6))]
    );
}
#[test]
fn scr_v103_closing_suffix_optional_include() {
    let mut source = Source::default();
    source.add("a", &["#<", " #>sinclude missing.sgr", "set a b"]);
    let mut actual = Vec::new();
    assert_eq!(
        parse_file_with(&mut source, "a", &mut |_, n, line, _| {
            actual.push((n, line.to_vec()));
            Ok(())
        }),
        Ok(3)
    );
    assert_eq!(actual, [(3, b"set a b".to_vec())]);
    assert_eq!(source.opened, ["a", "missing.sgr"]);
    assert!(matches!(source.finished[0].2, Err(Error::Parse { code: 2, .. })));
    assert_eq!(source.finished[1], ("a".into(), 3, Ok(3)));
}
#[test]
fn scr_v104_empty_closer_suffix_and_ordinary_empty_dispatch() {
    routing(&["#<", "#>", "", "# comment"], &[(2, ""), (3, ""), (4, "# comment")]);
}
#[test]
fn scr_v105_only_first_closer_removed() {
    routing(&["#<", "prefix #>set x y #>set z w"], &[(2, "set x y #>set z w")]);
}
#[test]
fn scr_v106_closer_survives_before_repeated_opener() {
    routing(
        &["#<", "#>set x y #< tail", "set a b"],
        &[(2, "set x y "), (3, "set a b")],
    );
}
#[test]
fn scr_v107_groups_do_not_protect_opener_and_spaced_closer() {
    routing(&["set x (#<)", "# >", "#>set a b"], &[(3, "set a b")]);
}
#[test]
fn scr_v108_registered_files_start_inactive() {
    let mut source = Source::default();
    source.add("a", &["#<", "ignored"]);
    source.add("b", &["set x y"]);
    let mut actual = Vec::new();
    let mut sink = |path: &str, n, line: &[u8], _| {
        actual.push((path.to_owned(), n, line.to_vec()));
        Ok(())
    };
    assert_eq!(parse_file_with(&mut source, "a", &mut sink), Ok(2));
    assert_eq!(parse_file_with(&mut source, "b", &mut sink), Ok(1));
    assert_eq!(actual, [("b".into(), 1, b"set x y".to_vec())]);
    assert_eq!(source.finished, [("a".into(), 2, Ok(2)), ("b".into(), 1, Ok(1))]);
}
