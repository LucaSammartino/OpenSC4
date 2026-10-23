// SPDX-License-Identifier: GPL-3.0-or-later
use super::{check_count, utf8_is_string_valid, walk, UnsupportedCount, MAX_COUNT};

struct Case {
    id: &'static str,
    backing: &'static [u8],
    count: usize,
    expected: bool,
}

// Literal expectations transcribed independently from PUBLIC §5, before any
// production comparison. No decoder or execution of our validator is an oracle.
macro_rules! case {
    ($id:literal, [$($byte:literal),*], $expected:literal) => {
        Case { id: $id, backing: &[$($byte),*], count: <[u8]>::len(&[$($byte),*]), expected: $expected }
    };
    ($id:literal, [$($byte:literal),*], $count:literal, $expected:literal) => {
        Case { id: $id, backing: &[$($byte),*], count: $count, expected: $expected }
    };
}

// 92 published rows, 107 independent cases (B01–B15 each have two cases).
const CASES: &[Case] = &[
    case!("V01", [], true),
    case!("V02", [0x00], true),
    case!("V03", [0x01, 0x41, 0x7f], true),
    case!("V04", [0x80], false),
    case!("V05", [0xbf], false),
    case!("V06", [0xc0], false),
    case!("V07", [0xc1], false),
    case!("V08", [0xc2, 0x80], true),
    case!("V09", [0xdf, 0xbf], true),
    case!("V10", [0xe0, 0x9f, 0xbf], false),
    case!("V11", [0xe0, 0xa0, 0x80], true),
    case!("V12", [0xe0, 0xbf, 0xbf], true),
    case!("V13", [0xe1, 0x80, 0x80], true),
    case!("V14", [0xef, 0xbf, 0xbf], true),
    case!("V15", [0xf0, 0x8f, 0xbf, 0xbf], false),
    case!("V16", [0xf0, 0x90, 0x80, 0x80], true),
    case!("V17", [0xf0, 0xbf, 0xbf, 0xbf], true),
    case!("V18", [0xf1, 0x80, 0x80, 0x80], true),
    case!("V19", [0xf4, 0x8f, 0xbf, 0xbf], true),
    case!("V20", [0xf4, 0x90, 0x80, 0x80], true),
    case!("V21", [0xf5, 0x80, 0x80, 0x80], true),
    case!("V22", [0xf7, 0xbf, 0xbf, 0xbf], true),
    case!("V23", [0xf8, 0x87, 0xbf, 0xbf, 0xbf], false),
    case!("V24", [0xf8, 0x88, 0x80, 0x80, 0x80], true),
    case!("V25", [0xf8, 0xbf, 0xbf, 0xbf, 0xbf], true),
    case!("V26", [0xf9, 0x80, 0x80, 0x80, 0x80], true),
    case!("V27", [0xfb, 0xbf, 0xbf, 0xbf, 0xbf], true),
    case!("V28", [0xfc, 0x83, 0xbf, 0xbf, 0xbf, 0xbf], false),
    case!("V29", [0xfc, 0x84, 0x80, 0x80, 0x80, 0x80], true),
    case!("V30", [0xfc, 0xbf, 0xbf, 0xbf, 0xbf, 0xbf], true),
    case!("V31", [0xfd, 0x80, 0x80, 0x80, 0x80, 0x80], true),
    case!("V32", [0xfd, 0xbf, 0xbf, 0xbf, 0xbf, 0xbf], true),
    case!("V33", [0xfe], false),
    case!("V34", [0xff], false),
    case!("V35", [0xed, 0x9f, 0xbf], true),
    case!("V36", [0xed, 0xa0, 0x80], true),
    case!("V37", [0xed, 0xbf, 0xbf], true),
    case!("V38", [0xee, 0x80, 0x80], true),
    case!("V39", [0xc0, 0x80], false),
    case!("V40", [0xc1, 0xbf], false),
    case!("V41", [0xe0, 0x80, 0x80], false),
    case!("V42", [0xf0, 0x80, 0x80, 0x80], false),
    case!("V43", [0xf8, 0x80, 0x80, 0x80, 0x80], false),
    case!("V44", [0xfc, 0x80, 0x80, 0x80, 0x80, 0x80], false),
    case!("T01", [0xc2], false),
    case!("T02", [0xe0], false),
    case!("T03", [0xe0, 0xa0], false),
    case!("T04", [0xf0], false),
    case!("T05", [0xf0, 0x90], false),
    case!("T06", [0xf0, 0x90, 0x80], false),
    case!("T07", [0xf8], false),
    case!("T08", [0xf8, 0x88], false),
    case!("T09", [0xf8, 0x88, 0x80], false),
    case!("T10", [0xf8, 0x88, 0x80, 0x80], false),
    case!("T11", [0xfc], false),
    case!("T12", [0xfc, 0x84], false),
    case!("T13", [0xfc, 0x84, 0x80], false),
    case!("T14", [0xfc, 0x84, 0x80, 0x80], false),
    case!("T15", [0xfc, 0x84, 0x80, 0x80, 0x80], false),
    case!("B01/7F", [0xc2, 0x7f], false),
    case!("B01/C0", [0xc2, 0xc0], false),
    case!("B02/7F", [0xe1, 0x7f, 0x80], false),
    case!("B02/C0", [0xe1, 0xc0, 0x80], false),
    case!("B03/7F", [0xe1, 0x80, 0x7f], false),
    case!("B03/C0", [0xe1, 0x80, 0xc0], false),
    case!("B04/7F", [0xf1, 0x7f, 0x80, 0x80], false),
    case!("B04/C0", [0xf1, 0xc0, 0x80, 0x80], false),
    case!("B05/7F", [0xf1, 0x80, 0x7f, 0x80], false),
    case!("B05/C0", [0xf1, 0x80, 0xc0, 0x80], false),
    case!("B06/7F", [0xf1, 0x80, 0x80, 0x7f], false),
    case!("B06/C0", [0xf1, 0x80, 0x80, 0xc0], false),
    case!("B07/7F", [0xf9, 0x7f, 0x80, 0x80, 0x80], false),
    case!("B07/C0", [0xf9, 0xc0, 0x80, 0x80, 0x80], false),
    case!("B08/7F", [0xf9, 0x80, 0x7f, 0x80, 0x80], false),
    case!("B08/C0", [0xf9, 0x80, 0xc0, 0x80, 0x80], false),
    case!("B09/7F", [0xf9, 0x80, 0x80, 0x7f, 0x80], false),
    case!("B09/C0", [0xf9, 0x80, 0x80, 0xc0, 0x80], false),
    case!("B10/7F", [0xf9, 0x80, 0x80, 0x80, 0x7f], false),
    case!("B10/C0", [0xf9, 0x80, 0x80, 0x80, 0xc0], false),
    case!("B11/7F", [0xfd, 0x7f, 0x80, 0x80, 0x80, 0x80], false),
    case!("B11/C0", [0xfd, 0xc0, 0x80, 0x80, 0x80, 0x80], false),
    case!("B12/7F", [0xfd, 0x80, 0x7f, 0x80, 0x80, 0x80], false),
    case!("B12/C0", [0xfd, 0x80, 0xc0, 0x80, 0x80, 0x80], false),
    case!("B13/7F", [0xfd, 0x80, 0x80, 0x7f, 0x80, 0x80], false),
    case!("B13/C0", [0xfd, 0x80, 0x80, 0xc0, 0x80, 0x80], false),
    case!("B14/7F", [0xfd, 0x80, 0x80, 0x80, 0x7f, 0x80], false),
    case!("B14/C0", [0xfd, 0x80, 0x80, 0x80, 0xc0, 0x80], false),
    case!("B15/7F", [0xfd, 0x80, 0x80, 0x80, 0x80, 0x7f], false),
    case!("B15/C0", [0xfd, 0x80, 0x80, 0x80, 0x80, 0xc0], false),
    case!("S01", [0xff], 0, true),
    case!("S02", [0x41, 0xff], 1, true),
    case!("S03", [0x00, 0x41], 2, true),
    case!("S04", [0x00, 0xff], 2, false),
    case!("S05", [0xc2, 0x00], 2, false),
    case!("S06", [0xc2, 0x80, 0x00], 3, true),
    case!("S07", [0xc2, 0x80], 1, false),
    case!("S08", [0x41, 0x80, 0x41], 3, false),
    case!("S09", [0x41, 0x80, 0xff], 3, false),
    case!("S10", [0xe1, 0x7f, 0x41, 0x41], 4, false),
    case!("S11", [0xe1, 0x7f, 0xff, 0xff], 4, false),
    case!("S12", [0xe0, 0x9f, 0x80, 0x41], 4, false),
    case!("S13", [0xe0, 0x9f, 0x7f, 0x41], 4, false),
    case!("S14", [0xf0, 0x8f, 0x80, 0x80, 0x41], 5, false),
    case!("S15", [0xf8, 0x87, 0x80, 0x80, 0x80, 0x41], 6, false),
    case!("S16", [0xfc, 0x83, 0x80, 0x80, 0x80, 0x80, 0x41], 7, false),
    case!(
        "S17",
        [
            0x41, 0xc2, 0x80, 0xe0, 0xa0, 0x80, 0xf0, 0x90, 0x80, 0x80, 0xf8, 0x88, 0x80, 0x80, 0x80, 0xfc, 0x84, 0x80,
            0x80, 0x80, 0x80, 0x00
        ],
        22,
        true
    ),
    case!("S18", [0xc2, 0x80, 0xbf], 3, false),
];

#[test]
fn public_107_literal_cases() {
    assert_eq!(CASES.len(), 107);
    for case in CASES {
        assert_eq!(
            utf8_is_string_valid(&case.backing[..case.count]),
            Ok(case.expected),
            "{}",
            case.id
        );
    }
}

fn assert_reads(id: &str, expected_reads: &[usize]) {
    let case = CASES.iter().find(|case| case.id == id).unwrap();
    let mut reads = Vec::new();
    check_count(case.count).unwrap();
    let result = walk(case.count, |offset| {
        assert!(offset < case.count, "{id}: read outside counted span");
        reads.push(offset);
        case.backing[offset]
    });
    assert_eq!(result, case.expected, "{id}");
    assert_eq!(reads, expected_reads, "{id}");
}

#[test]
fn no_reads_on_empty_and_no_continuation_reads_on_all_15_truncations() {
    assert_reads("V01", &[]);
    assert_reads("S01", &[]);
    for case in CASES.iter().filter(|case| case.id.starts_with('T')) {
        assert_reads(case.id, &[0]);
    }
    assert_reads("S07", &[0]);
}

#[test]
fn all_30_bad_continuations_stop_at_first_bad_position() {
    // Literal expected offset lists for the fifteen positions, each tested on
    // both sides. They are not calculated by the production classification.
    let rows: &[(&str, &str, &[usize])] = &[
        ("B01/7F", "B01/C0", &[0, 1]),
        ("B02/7F", "B02/C0", &[0, 1]),
        ("B03/7F", "B03/C0", &[0, 1, 2]),
        ("B04/7F", "B04/C0", &[0, 1]),
        ("B05/7F", "B05/C0", &[0, 1, 2]),
        ("B06/7F", "B06/C0", &[0, 1, 2, 3]),
        ("B07/7F", "B07/C0", &[0, 1]),
        ("B08/7F", "B08/C0", &[0, 1, 2]),
        ("B09/7F", "B09/C0", &[0, 1, 2, 3]),
        ("B10/7F", "B10/C0", &[0, 1, 2, 3, 4]),
        ("B11/7F", "B11/C0", &[0, 1]),
        ("B12/7F", "B12/C0", &[0, 1, 2]),
        ("B13/7F", "B13/C0", &[0, 1, 2, 3]),
        ("B14/7F", "B14/C0", &[0, 1, 2, 3, 4]),
        ("B15/7F", "B15/C0", &[0, 1, 2, 3, 4, 5]),
    ];
    for &(low, high, reads) in rows {
        assert_reads(low, reads);
        assert_reads(high, reads);
    }
}

#[test]
fn public_stopping_and_lower_bound_read_order() {
    let traces: &[(&str, &[usize])] = &[
        ("S02", &[0]),
        ("S03", &[0, 1]),
        ("S04", &[0, 1]),
        ("S05", &[0, 1]),
        ("S06", &[0, 1, 2]),
        ("S08", &[0, 1]),
        ("S09", &[0, 1]),
        ("S10", &[0, 1]),
        ("S11", &[0, 1]),
        ("S12", &[0, 1, 2, 0]),
        ("S13", &[0, 1, 2]),
        ("S14", &[0, 1, 2, 3, 0]),
        ("S15", &[0, 1, 2, 3, 4, 0]),
        ("S16", &[0, 1, 2, 3, 4, 5, 0]),
        ("S18", &[0, 1, 2]),
    ];
    for &(id, reads) in traces {
        assert_reads(id, reads);
    }
}

#[test]
fn count_domain_is_distinct_from_false_without_allocating_gigabytes() {
    assert_eq!(check_count(0), Ok(()));
    assert_eq!(check_count(MAX_COUNT), Ok(()));
    assert_eq!(check_count(MAX_COUNT + 1), Err(UnsupportedCount));
    assert_eq!(check_count(usize::MAX), Err(UnsupportedCount));
    assert_eq!(utf8_is_string_valid(&[0xff]), Ok(false));
}
