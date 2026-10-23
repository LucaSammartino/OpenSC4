//! Literal deductions from SM-V1..13, not native measurements or roundtrips.
use super::*;

const ALL: Bounds = Bounds {
    column_min: i32::MIN,
    column_max: i32::MAX,
    row_min: i32::MIN,
    row_max: i32::MAX,
};

fn fixture() -> PointerMatrix {
    PointerMatrix::from_validated_state(ALL, [(5, 3, [0x44]), (2, 0, [0]), (2, -1, [0x11]), (-3, 0, [0x22])]).unwrap()
}

#[derive(Debug, PartialEq, Eq)]
enum Access {
    Read(usize, u32),
    Write(usize, u32),
}

struct Words {
    values: Vec<u32>,
    events: Vec<Access>,
}

impl Words {
    fn new(values: Vec<u32>) -> Self {
        Self {
            values,
            events: Vec::new(),
        }
    }
}

impl WordIo for Words {
    fn word_len(&self) -> usize {
        self.values.len()
    }
    fn read_word(&mut self, index: usize) -> u32 {
        let value = self.values[index];
        self.events.push(Access::Read(index, value));
        value
    }
    fn write_word(&mut self, index: usize, value: u32) {
        self.events.push(Access::Write(index, value));
        self.values[index] = value;
    }
}

#[test]
fn sm_v1_signed_inclusive_occupancy_zero_and_inverted_ranges() {
    let matrix = fixture();
    let selected = Bounds {
        column_min: -3,
        column_max: 2,
        row_min: -1,
        row_max: 0,
    };
    assert_eq!(matrix.count_range(selected), 3);
    assert_eq!(
        matrix.count_range(Bounds {
            column_min: 2,
            column_max: -3,
            ..selected
        }),
        0
    );
    assert_eq!(
        matrix.count_range(Bounds {
            row_min: 0,
            row_max: -1,
            ..selected
        }),
        0
    );
    assert_eq!(matrix.column_count_for_row(0), 2);
    assert_eq!(matrix.column_count_for_row(9), 0);
    assert_eq!(matrix.row_count_for_column(2), 2);
    assert_eq!(matrix.row_count_for_column(4), 0);
    assert_eq!(matrix.stored_count(), 4);
}

#[test]
fn sm_v2_range_is_twelve_byte_value_copy_in_row_column_order() {
    let matrix = fixture();
    let untouched = RangeRecord {
        column: 99,
        row: 88,
        payload: 77,
    };
    let mut records = [untouched; 5];
    assert_eq!(std::mem::size_of::<RangeRecord>(), 12);
    assert_eq!(matrix.range_records(ALL, Some(&mut records)), Ok(4));
    assert_eq!(
        records,
        [
            RangeRecord {
                column: 2,
                row: -1,
                payload: 0x11
            },
            RangeRecord {
                column: -3,
                row: 0,
                payload: 0x22
            },
            RangeRecord {
                column: 2,
                row: 0,
                payload: 0
            },
            RangeRecord {
                column: 5,
                row: 3,
                payload: 0x44
            },
            untouched,
        ]
    );
    assert_eq!(matrix.range_records(ALL, None), Ok(4));
    let inverted = Bounds {
        column_min: 5,
        column_max: -3,
        ..ALL
    };
    assert_eq!(matrix.range_records(inverted, Some(&mut records)), Ok(0));
    assert_eq!(records[4], untouched);
    let mut too_small = [untouched; 3];
    assert_eq!(
        matrix.range_records(ALL, Some(&mut too_small)),
        Err(DomainError::InsufficientRangeCapacity)
    );
    assert_eq!(too_small, [untouched; 3]);
}

#[test]
fn sm_v3_fixed_column_holes_and_zero_payload() {
    let matrix = fixture();
    let mut cursor = matrix.column_begin(2);
    assert_eq!(cursor.current(), Some((2, -1, &[0x11])));
    let first_identity = cursor.identity();
    assert_eq!(first_identity, matrix.column_begin(2).identity());
    cursor.increment().unwrap();
    assert_eq!(cursor.current(), Some((2, 0, &[0])));
    assert_ne!(cursor.identity(), first_identity);
    cursor.increment().unwrap();
    assert_eq!(cursor.current(), None);
    assert_eq!(cursor.identity(), matrix.column_begin(4).identity());
    assert_eq!(cursor.increment(), Err(DomainError::UnsupportedEndIncrement));
}

#[test]
fn fixed_column_increment_skips_intermediate_nonmatching_rows() {
    let matrix =
        PointerMatrix::from_validated_state(ALL, [(2, -5, [7]), (1, -2, [8]), (3, -2, [9]), (2, 8, [0])]).unwrap();
    let mut cursor = matrix.column_begin(2);
    assert_eq!(cursor.current(), Some((2, -5, &[7])));
    cursor.increment().unwrap();
    assert_eq!(cursor.current(), Some((2, 8, &[0])));
    cursor.increment().unwrap();
    assert_eq!(cursor.current(), None);
}

#[test]
fn sm_v4_shared_independent_word_is_written_before_read() {
    let mut matrix = PointerMatrix::from_validated_state(ALL, [(2, 0, [0x11])]).unwrap();
    let identity = matrix.cell_identity(2, 0);
    let mut words = Words::new(vec![0x22]);
    assert_eq!(
        matrix.insert(2, 0, &mut words, WordToken(0), Some(WordToken(0))),
        Ok(true)
    );
    assert_eq!(words.events, [Access::Write(0, 0x11), Access::Read(0, 0x11)]);
    assert_eq!(words.values, [0x11]);
    let mut output = None;
    assert!(matrix.lookup(2, 0, Some(&mut output)));
    assert_eq!(output, Some(&[0x11]));
    assert_eq!(matrix.stored_count(), 1);
    assert_eq!(matrix.cell_identity(2, 0), identity);
}

#[test]
fn replacement_nonalias_and_new_cells_leave_old_output_untouched() {
    let mut matrix = PointerMatrix::from_validated_state(ALL, [(2, 0, [0x11])]).unwrap();
    let mut words = Words::new(vec![0x22, 0x55]);
    assert_eq!(
        matrix.insert(2, 0, &mut words, WordToken(0), Some(WordToken(1))),
        Ok(true)
    );
    assert_eq!(words.events, [Access::Write(1, 0x11), Access::Read(0, 0x22)]);
    words.events.clear();
    assert_eq!(
        matrix.insert(3, 0, &mut words, WordToken(0), Some(WordToken(1))),
        Ok(true)
    );
    assert_eq!(
        matrix.insert(-1, -2, &mut words, WordToken(0), Some(WordToken(1))),
        Ok(true)
    );
    assert_eq!(words.events, [Access::Read(0, 0x22), Access::Read(0, 0x22)]);
    assert_eq!(words.values[1], 0x11);
    assert_eq!(matrix.stored_count(), 3);
    assert_eq!(matrix.column_count_for_row(0), 2);
    assert_eq!(matrix.column_count_for_row(-2), 1);
    let mut cell = None;
    assert!(matrix.lookup(2, 0, Some(&mut cell)));
    assert_eq!(cell, Some(&[0x22]));
}

#[test]
fn sm_v5_all_bounds_reject_without_payload_access_and_boundary_accepts() {
    let bounds = Bounds {
        column_min: 0,
        column_max: 2,
        row_min: 0,
        row_max: 2,
    };
    let mut matrix = PointerMatrix::from_validated_state(bounds, []).unwrap();
    let mut words = Words::new(vec![0x22, 0x55]);
    for (column, row) in [(3, 1), (1, -1), (-1, 1), (1, 3)] {
        assert_eq!(
            matrix.insert(
                column,
                row,
                &mut words,
                WordToken(usize::MAX),
                Some(WordToken(usize::MAX))
            ),
            Ok(false)
        );
    }
    assert_eq!(words.events, []);
    assert_eq!(words.values, [0x22, 0x55]);
    assert_eq!(matrix.stored_count(), 0);
    assert_eq!(
        matrix.insert(2, 2, &mut words, WordToken(0), Some(WordToken(1))),
        Ok(true)
    );
    assert_eq!(words.events, [Access::Read(0, 0x22)]);
    assert_eq!(matrix.stored_count(), 1);
}

#[test]
fn sm_v6_rows_half_open_no_key_shift_count_before_erase() {
    let mut matrix =
        PointerMatrix::from_validated_state(ALL, [(0, 1, [0x11]), (0, 2, [0x22]), (0, 3, [0x33])]).unwrap();
    assert!(matrix.remove_rows(1, 2));
    assert_eq!(matrix.stored_count(), 1);
    assert_eq!(
        matrix.events,
        [Event::Count(2), Event::EraseRow(1), Event::Count(1), Event::EraseRow(2)]
    );
    assert!(matrix.lookup(0, 3, None));
    assert!(!matrix.lookup(0, 1, None));
    matrix.events.clear();
    assert!(matrix.remove_rows(3, 0));
    assert!(matrix.events.is_empty());
    assert_eq!(matrix.stored_count(), 1);
}

#[test]
fn sm_v7_wrapped_endpoint_and_negative_count_are_signed_key_rules() {
    let mut matrix =
        PointerMatrix::from_validated_state(ALL, [(0, i32::MAX, [1]), (0, i32::MIN, [2]), (0, -1, [3]), (0, 0, [4])])
            .unwrap();
    assert!(matrix.remove_rows(i32::MAX, 1));
    assert_eq!(matrix.stored_count(), 4);
    assert!(matrix.events.is_empty());
    // MIN + (-1) wraps to MAX; a count<=0 shortcut would wrongly skip.
    assert!(matrix.remove_rows(i32::MIN, -1));
    assert_eq!(
        matrix.events,
        [
            Event::Count(3),
            Event::EraseRow(i32::MIN),
            Event::Count(2),
            Event::EraseRow(-1),
            Event::Count(1),
            Event::EraseRow(0),
        ]
    );
    assert_eq!(matrix.stored_count(), 1);
    assert!(matrix.lookup(0, i32::MAX, None));
}

#[test]
fn sm_v8_columns_count_empty_row_erase_and_successor_order() {
    let mut matrix =
        PointerMatrix::from_validated_state(ALL, [(1, 0, [0x11]), (2, 0, [0x22]), (1, 3, [0x33])]).unwrap();
    assert!(matrix.remove_columns(1, 1));
    assert_eq!(
        matrix.events,
        [
            Event::EraseCell(1, 0),
            Event::Count(2),
            Event::NextRow(0, Some(3)),
            Event::EraseCell(1, 3),
            Event::Count(1),
            Event::NextRow(3, None),
            Event::EraseRow(3),
        ]
    );
    assert!(matrix.lookup(2, 0, None));
    assert_eq!(matrix.column_count_for_row(3), 0);
    assert_eq!(matrix.stored_count(), 1);
    assert!(matrix.remove_columns(2, 1));
    assert_eq!(matrix.stored_count(), 0);
    assert!(matrix.rows.is_empty());
    assert!(matrix.remove_columns(0, 0));
}

#[test]
fn columns_wrapped_negative_count_order_and_next_row_before_first_erase() {
    let mut matrix =
        PointerMatrix::from_validated_state(ALL, [(i32::MIN, -1, [1]), (0, 2, [2]), (i32::MAX, 2, [3])]).unwrap();
    assert!(matrix.remove_columns(i32::MAX, 1));
    assert_eq!(matrix.stored_count(), 3);
    matrix.events.clear();
    assert!(matrix.remove_columns(i32::MIN, -1));
    assert_eq!(
        matrix.events,
        [
            Event::EraseCell(i32::MIN, -1),
            Event::Count(2),
            Event::NextRow(-1, Some(2)),
            Event::EraseRow(-1),
            Event::EraseCell(0, 2),
            Event::Count(1),
            Event::NextRow(2, None),
        ]
    );
    assert!(matrix.lookup(i32::MAX, 2, None));
}

#[test]
fn sm_v9_missing_extrema_initialize_outputs_and_alias_last_store() {
    let matrix = PointerMatrix::from_validated_state(ALL, []).unwrap();
    let mut words = Words::new(vec![55, 66]);
    assert_eq!(
        matrix.min_max_columns_for_row(7, &mut words, WordToken(0), WordToken(1)),
        Ok(false)
    );
    assert_eq!(
        words.events,
        [Access::Write(0, 0x7fff_ffff), Access::Write(1, 0x8000_0000)]
    );
    words.events.clear();
    assert_eq!(
        matrix.min_max_rows_for_column(7, &mut words, WordToken(0), WordToken(1)),
        Ok(false)
    );
    assert_eq!(
        words.events,
        [Access::Write(0, 0x7fff_ffff), Access::Write(1, 0x8000_0000)]
    );
    assert_eq!(matrix.max_used_column(&mut words, WordToken(0)), Ok(i32::MIN));
    assert_eq!(words.values, [0x8000_0000, 0x8000_0000]);
    assert_eq!(
        matrix.min_max_rows_for_column(7, &mut words, WordToken(0), WordToken(0)),
        Ok(false)
    );
    assert_eq!(words.values[0], 0x8000_0000);
    assert_eq!(
        matrix.min_max_columns_for_row(7, &mut words, WordToken(0), WordToken(0)),
        Ok(false)
    );
    assert_eq!(words.values[0], 0x8000_0000);
}

#[test]
fn existing_extrema_alias_writes_max_last_in_both_directions() {
    let matrix = fixture();
    let mut words = Words::new(vec![9]);
    assert_eq!(
        matrix.min_max_columns_for_row(0, &mut words, WordToken(0), WordToken(0)),
        Ok(true)
    );
    assert_eq!(
        words.events,
        [
            Access::Write(0, 0x7fff_ffff),
            Access::Write(0, 0x8000_0000),
            Access::Write(0, (-3i32) as u32),
            Access::Write(0, 2)
        ]
    );
    words.events.clear();
    assert_eq!(
        matrix.min_max_rows_for_column(2, &mut words, WordToken(0), WordToken(0)),
        Ok(true)
    );
    assert_eq!(
        words.events,
        [
            Access::Write(0, 0x7fff_ffff),
            Access::Write(0, 0x8000_0000),
            Access::Write(0, u32::MAX),
            Access::Write(0, u32::MAX),
            Access::Write(0, 0)
        ]
    );
    assert_eq!(words.values, [0]);
}

#[test]
fn sm_v10_int_min_sentinel_override_last_tie_normal_max_first_tie() {
    let sentinels = PointerMatrix::from_validated_state(ALL, [(i32::MIN, 1, [0x11]), (i32::MIN, 2, [0x22])]).unwrap();
    let normal = PointerMatrix::from_validated_state(ALL, [(5, 1, [0x11]), (5, 2, [0x22])]).unwrap();
    let mut words = Words::new(vec![0]);
    assert_eq!(sentinels.max_used_column(&mut words, WordToken(0)), Ok(i32::MIN));
    assert_eq!(
        words.events,
        [Access::Write(0, 0x8000_0000), Access::Write(0, 1), Access::Write(0, 2)]
    );
    words.events.clear();
    assert_eq!(normal.max_used_column(&mut words, WordToken(0)), Ok(5));
    assert_eq!(words.events, [Access::Write(0, 0x8000_0000), Access::Write(0, 1)]);
}

#[test]
fn sm_v11_equality_ignores_matrix_and_null_end_ignores_row() {
    let matrix = fixture();
    let other = fixture();
    let live = matrix.column_begin(2).identity();
    let retagged = live.with_matrix_identity(other.matrix_identity());
    assert_ne!(live.matrix_identity(), retagged.matrix_identity());
    assert_eq!(live, retagged);
    assert_ne!(live, other.column_begin(2).identity());
    let end = matrix.column_begin(4).identity();
    let different_end = IteratorIdentity {
        matrix: other.identity,
        row: Some(other.rows[&0].identity),
        cell: None,
    };
    assert_eq!(end, different_end);
    assert_ne!(live, end);
    let changed_row = IteratorIdentity {
        row: Some(matrix.rows[&0].identity),
        ..live
    };
    assert_ne!(live, changed_row);
    let changed_cell = IteratorIdentity {
        cell: matrix.cell_identity(-3, 0),
        ..live
    };
    assert_ne!(live, changed_cell);
}

#[test]
fn sm_v12_missing_remove_unchanged_hit_output_then_count_and_empty_row() {
    let mut matrix = PointerMatrix::from_validated_state(ALL, [(2, 0, [0x11])]).unwrap();
    let mut words = Words::new(vec![0x55]);
    assert_eq!(matrix.remove_cell(9, 9, &mut words, Some(WordToken(0))), Ok(false));
    assert_eq!(words.values, [0x55]);
    assert_eq!(words.events, []);
    assert_eq!(matrix.stored_count(), 1);
    assert_eq!(matrix.remove_cell(2, 0, &mut words, Some(WordToken(0))), Ok(true));
    assert_eq!(words.events, [Access::Write(0, 0x11)]);
    assert_eq!(
        matrix.events,
        [Event::EraseCell(2, 0), Event::Count(0), Event::EraseRow(0)]
    );
    assert!(matrix.rows.is_empty());
    // Missing output token is not dereferenced even if invalid.
    assert_eq!(
        matrix.remove_cell(2, 0, &mut words, Some(WordToken(usize::MAX))),
        Ok(false)
    );
}

#[test]
fn remove_cell_optional_null_and_nonempty_row_retains_identity() {
    let mut matrix = fixture();
    let surviving = matrix.cell_identity(2, 0);
    let mut words = Words::new(vec![]);
    assert_eq!(matrix.remove_cell(-3, 0, &mut words, None), Ok(true));
    assert!(words.events.is_empty());
    assert_eq!(matrix.column_count_for_row(0), 1);
    assert_eq!(matrix.stored_count(), 3);
    assert_eq!(matrix.cell_identity(2, 0), surviving);
}

#[test]
fn sm_v13_ten_word_shared_independent_buffer_old_copy_before_input() {
    let mut matrix = ExtraMatrix::from_validated_state(ALL, [(2, 0, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10])]).unwrap();
    let mut words = Words::new(vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
    assert_eq!(
        matrix.insert(2, 0, &mut words, WordToken(0), Some(WordToken(0))),
        Ok(true)
    );
    assert_eq!(
        words.events,
        [
            Access::Write(0, 1),
            Access::Write(1, 2),
            Access::Write(2, 3),
            Access::Write(3, 4),
            Access::Write(4, 5),
            Access::Write(5, 6),
            Access::Write(6, 7),
            Access::Write(7, 8),
            Access::Write(8, 9),
            Access::Write(9, 10),
            Access::Read(0, 1),
            Access::Read(1, 2),
            Access::Read(2, 3),
            Access::Read(3, 4),
            Access::Read(4, 5),
            Access::Read(5, 6),
            Access::Read(6, 7),
            Access::Read(7, 8),
            Access::Read(8, 9),
            Access::Read(9, 10),
        ]
    );
    assert_eq!(words.values, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let mut output = None;
    assert!(matrix.lookup(2, 0, Some(&mut output)));
    assert_eq!(output, Some(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
    assert_eq!(matrix.stored_count(), 1);
}

#[test]
fn extra_new_row_new_cell_and_rejection_without_input_reads() {
    let bounds = Bounds {
        column_min: 0,
        column_max: 2,
        row_min: 0,
        row_max: 2,
    };
    let mut matrix = ExtraMatrix::from_validated_state(bounds, []).unwrap();
    let mut words = Words::new(vec![
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64,
    ]);
    assert_eq!(
        matrix.insert(3, 1, &mut words, WordToken(99), Some(WordToken(99))),
        Ok(false)
    );
    assert!(words.events.is_empty());
    assert_eq!(
        matrix.insert(2, 2, &mut words, WordToken(0), Some(WordToken(10))),
        Ok(true)
    );
    assert_eq!(
        matrix.insert(1, 2, &mut words, WordToken(0), Some(WordToken(10))),
        Ok(true)
    );
    assert_eq!(words.values[10..], [55, 56, 57, 58, 59, 60, 61, 62, 63, 64]);
    assert_eq!(
        words.events,
        [
            Access::Read(0, 1),
            Access::Read(1, 2),
            Access::Read(2, 3),
            Access::Read(3, 4),
            Access::Read(4, 5),
            Access::Read(5, 6),
            Access::Read(6, 7),
            Access::Read(7, 8),
            Access::Read(8, 9),
            Access::Read(9, 10),
            Access::Read(0, 1),
            Access::Read(1, 2),
            Access::Read(2, 3),
            Access::Read(3, 4),
            Access::Read(4, 5),
            Access::Read(5, 6),
            Access::Read(6, 7),
            Access::Read(7, 8),
            Access::Read(8, 9),
            Access::Read(9, 10),
        ]
    );
    assert_eq!(matrix.stored_count(), 2);
}

#[test]
fn lookup_borrows_logical_payload_and_miss_leaves_output_unchanged() {
    let matrix =
        ExtraMatrix::from_validated_state(ALL, [(-3, -2, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]), (2, 0, [0; 10])]).unwrap();
    let sentinel = [99; 10];
    let mut output = Some(&sentinel);
    assert!(!matrix.lookup(99, 99, Some(&mut output)));
    assert!(std::ptr::eq(output.unwrap(), &sentinel));
    assert!(!matrix.lookup(99, -2, Some(&mut output)));
    assert!(std::ptr::eq(output.unwrap(), &sentinel));
    assert!(matrix.lookup(2, 0, None));
    assert!(!matrix.lookup(99, 99, None));
    assert!(matrix.lookup(-3, -2, Some(&mut output)));
    assert!(std::ptr::eq(output.unwrap(), &matrix.rows[&-2].cells[&-3].payload));
    assert_eq!(output, Some(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
}

#[test]
fn validated_fixture_errors_are_portable_domain_checks_not_native_defaults() {
    assert!(matches!(
        PointerMatrix::from_validated_state(
            Bounds {
                row_min: 2,
                row_max: 1,
                ..ALL
            },
            []
        ),
        Err(DomainError::InvalidBounds)
    ));
    assert!(matches!(
        PointerMatrix::from_validated_state(ALL, [(1, 2, [0]), (1, 2, [3])]),
        Err(DomainError::DuplicateCoordinate)
    ));
    assert!(matches!(
        PointerMatrix::from_validated_state(Bounds { column_min: 0, ..ALL }, [(-1, 2, [0])]),
        Err(DomainError::CoordinateOutsideBounds)
    ));
    assert!(matches!(
        SparseMatrix::<2>::from_validated_state(ALL, []),
        Err(DomainError::UnsupportedPayloadWidth)
    ));
    let mut matrix = fixture();
    let mut words = Words::new(vec![]);
    assert_eq!(
        matrix.insert(2, 0, &mut words, WordToken(0), None),
        Err(DomainError::InvalidBuffer)
    );
    assert_eq!(matrix.stored_count(), 4);
    assert!(words.events.is_empty());
}
