//! Bounded normal-domain algorithms from `docs/specs/rz-sparse-matrix.md` §§2–6.
//!
//! [choice, SM3] `from_validated_state` is a portable fixture/API constructor,
//! not an implementation of an original constructor, defaults, resize or copy.
//! Rows are nonempty, coordinates unique, and bounds supplied explicitly.
//! Standard ordered maps provide storage only; selection, ordering and counts
//! are owned here. Allocation/eraser exceptions and payload lifetime (SM2/3)
//! are unsupported; no native failure, rollback or destruction claim is made.
//! Pointer words are opaque integers, never dereferenced or reference counted.
//! The ten-word domain has its own payload type. Shared portable algorithms do
//! not establish native specialization aliases (SM5).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

pub type PointerMatrix = SparseMatrix<1>;
pub type ExtraMatrix = SparseMatrix<10>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub column_min: i32,
    pub column_max: i32,
    pub row_min: i32,
    pub row_max: i32,
}

impl Bounds {
    // Keep the native check sequence, including short-circuiting before I/O.
    fn admits(self, column: i32, row: i32) -> bool {
        if row < self.row_min {
            return false;
        }
        if column < self.column_min {
            return false;
        }
        if row > self.row_max {
            return false;
        }
        if column > self.column_max {
            return false;
        }
        true
    }
}

/// Portable errors, never invented native AL=false or successful fallbacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainError {
    InvalidBounds,
    UnsupportedPayloadWidth,
    CoordinateOutsideBounds,
    DuplicateCoordinate,
    InvalidBuffer,
    InsufficientRangeCapacity,
    UnsupportedEndIncrement,
    IdentityExhausted,
}

/// Primitive independent word storage. Reads/writes must be valid and return
/// normally; callbacks supply no tree, selection, membership or outcome logic.
/// Tokens may alias each other, but cannot address live matrix storage (SM4).
pub trait WordIo {
    fn word_len(&self) -> usize;
    fn read_word(&mut self, index: usize) -> u32;
    fn write_word(&mut self, index: usize, value: u32);
}

impl WordIo for Vec<u32> {
    fn word_len(&self) -> usize {
        self.len()
    }
    fn read_word(&mut self, index: usize) -> u32 {
        self[index]
    }
    fn write_word(&mut self, index: usize, value: u32) {
        self[index] = value;
    }
}

/// Starting word of a contiguous independent buffer. Validated before I/O.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordToken(pub usize);

fn validate(io: &impl WordIo, token: WordToken, words: usize) -> Result<(), DomainError> {
    match token.0.checked_add(words) {
        Some(end) if end <= io.word_len() => Ok(()),
        _ => Err(DomainError::InvalidBuffer),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatrixIdentity(u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowIdentity(MatrixIdentity, u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellIdentity(MatrixIdentity, u64);

static NEXT_MATRIX: AtomicU64 = AtomicU64::new(1);

struct Cell<const W: usize> {
    identity: CellIdentity,
    payload: [u32; W],
}

struct Row<const W: usize> {
    identity: RowIdentity,
    cells: BTreeMap<i32, Cell<W>>,
    count: u32,
}

pub struct SparseMatrix<const W: usize> {
    bounds: Bounds,
    identity: MatrixIdentity,
    next_identity: u64,
    rows: BTreeMap<i32, Row<W>>,
    count: u32,
    #[cfg(test)]
    events: Vec<Event>,
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
enum Event {
    EraseCell(i32, i32),
    Count(u32),
    NextRow(i32, Option<i32>),
    EraseRow(i32),
}

impl<const W: usize> SparseMatrix<W> {
    /// [choice, SM3] Validated explicit normal state; no default dimensions,
    /// empty-row fixtures, native factory, deep-copy or lifetime semantics.
    /// Fixture entries are (column, row, payload); unsorted input is allowed.
    pub fn from_validated_state(
        bounds: Bounds,
        entries: impl IntoIterator<Item = (i32, i32, [u32; W])>,
    ) -> Result<Self, DomainError> {
        if W != 1 && W != 10 {
            return Err(DomainError::UnsupportedPayloadWidth);
        }
        if bounds.column_min > bounds.column_max || bounds.row_min > bounds.row_max {
            return Err(DomainError::InvalidBounds);
        }
        // Keep Rust 1.75 compatibility: try_update, the replacement for the
        // deprecated fetch_update, is newer than the workspace's MSRV.
        let mut next = NEXT_MATRIX.load(Ordering::Relaxed);
        let identity = loop {
            let successor = next.checked_add(1).ok_or(DomainError::IdentityExhausted)?;
            match NEXT_MATRIX.compare_exchange_weak(next, successor, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break MatrixIdentity(next),
                Err(observed) => next = observed,
            }
        };
        let mut matrix = Self {
            bounds,
            identity,
            next_identity: 1,
            rows: BTreeMap::new(),
            count: 0,
            #[cfg(test)]
            events: Vec::new(),
        };
        for (column, row, payload) in entries {
            if !bounds.admits(column, row) {
                return Err(DomainError::CoordinateOutsideBounds);
            }
            if matrix.rows.get(&row).is_some_and(|r| r.cells.contains_key(&column)) {
                return Err(DomainError::DuplicateCoordinate);
            }
            matrix.add_normal(column, row, payload)?;
        }
        Ok(matrix)
    }

    fn allocate_identity(&mut self) -> Result<u64, DomainError> {
        let id = self.next_identity;
        self.next_identity = id.checked_add(1).ok_or(DomainError::IdentityExhausted)?;
        Ok(id)
    }

    fn add_row_normal(&mut self, row: i32) -> Result<(), DomainError> {
        if !self.rows.contains_key(&row) {
            let identity = RowIdentity(self.identity, self.allocate_identity()?);
            self.rows.insert(
                row,
                Row {
                    identity,
                    cells: BTreeMap::new(),
                    count: 0,
                },
            );
        }
        Ok(())
    }

    fn add_normal(&mut self, column: i32, row: i32, payload: [u32; W]) -> Result<(), DomainError> {
        self.check_identity_capacity(row)?;
        self.add_row_normal(row)?;
        let identity = CellIdentity(self.identity, self.allocate_identity()?);
        let target = self.rows.get_mut(&row).expect("normal row inserted");
        target.cells.insert(column, Cell { identity, payload });
        target.count = target.count.wrapping_add(1);
        self.count = self.count.wrapping_add(1);
        Ok(())
    }

    fn check_identity_capacity(&self, row: i32) -> Result<(), DomainError> {
        let required = if self.rows.contains_key(&row) { 1 } else { 2 };
        self.next_identity
            .checked_add(required)
            .ok_or(DomainError::IdentityExhausted)?;
        Ok(())
    }

    pub fn matrix_identity(&self) -> MatrixIdentity {
        self.identity
    }

    pub fn cell_identity(&self, column: i32, row: i32) -> Option<CellIdentity> {
        Some(self.rows.get(&row)?.cells.get(&column)?.identity)
    }

    pub fn stored_count(&self) -> u32 {
        self.count
    }

    pub fn column_count_for_row(&self, row: i32) -> u32 {
        self.rows.get(&row).map_or(0, |r| r.count)
    }

    pub fn row_count_for_column(&self, column: i32) -> u32 {
        let mut count = 0u32;
        for row in self.rows.values() {
            if row.cells.contains_key(&column) {
                count = count.wrapping_add(1);
            }
        }
        count
    }

    /// Borrowed logical payload address, not a value copy. A miss leaves the
    /// optional output untouched; None requests existence only. Portable bool
    /// models the published AL result, with no upper-EAX assertion. For W=1
    /// this is an API choice, not proof of a native pointer lookup alias (SM5).
    pub fn lookup<'a>(&'a self, column: i32, row: i32, output: Option<&mut Option<&'a [u32; W]>>) -> bool {
        let Some(cell) = self.rows.get(&row).and_then(|r| r.cells.get(&column)) else {
            return false;
        };
        if let Some(output) = output {
            *output = Some(&cell.payload);
        }
        true
    }

    /// Normal valid-helper domain only. Independent input/output buffers may
    /// alias, including partial overlap; old forward-word stores precede all
    /// input loads. No input snapshot is taken before the old output copy.
    pub fn insert(
        &mut self,
        column: i32,
        row: i32,
        io: &mut impl WordIo,
        input: WordToken,
        old_output: Option<WordToken>,
    ) -> Result<bool, DomainError> {
        if !self.bounds.admits(column, row) {
            return Ok(false);
        }
        validate(io, input, W)?;
        if let Some(cell) = self.rows.get_mut(&row).and_then(|r| r.cells.get_mut(&column)) {
            if let Some(output) = old_output {
                validate(io, output, W)?;
                for word in 0..W {
                    io.write_word(output.0 + word, cell.payload[word]);
                }
            }
            for word in 0..W {
                cell.payload[word] = io.read_word(input.0 + word);
            }
        } else {
            // Missing-row creation precedes input access, per §4. No helper
            // failure result is translated into native success or rollback.
            self.check_identity_capacity(row)?;
            self.add_row_normal(row)?;
            let mut payload = [0; W]; // overwritten fully; not a default cell.
            for (word, value) in payload.iter_mut().enumerate() {
                *value = io.read_word(input.0 + word);
            }
            self.add_normal(column, row, payload)?;
        }
        Ok(true)
    }

    pub fn remove_cell(
        &mut self,
        column: i32,
        row: i32,
        io: &mut impl WordIo,
        old_output: Option<WordToken>,
    ) -> Result<bool, DomainError> {
        let Some(cell) = self.rows.get(&row).and_then(|r| r.cells.get(&column)) else {
            return Ok(false);
        };
        if let Some(output) = old_output {
            validate(io, output, W)?;
            for word in 0..W {
                io.write_word(output.0 + word, cell.payload[word]);
            }
        }
        self.erase_cell_normal(column, row);
        if self.rows[&row].count == 0 {
            self.erase_row_normal(row);
        }
        Ok(true)
    }

    fn erase_cell_normal(&mut self, column: i32, row: i32) {
        let target = self.rows.get_mut(&row).expect("selected row");
        target.cells.remove(&column);
        #[cfg(test)]
        self.events.push(Event::EraseCell(column, row));
        target.count = target.count.wrapping_sub(1);
        self.count = self.count.wrapping_sub(1);
        #[cfg(test)]
        self.events.push(Event::Count(self.count));
    }

    fn erase_row_normal(&mut self, row: i32) {
        self.rows.remove(&row);
        #[cfg(test)]
        self.events.push(Event::EraseRow(row));
    }

    /// Exact signed half-open key selection against a wrapped 32-bit endpoint.
    /// Collect existing keys safely: skipped holes have no observable effect.
    /// In particular a negative count can select keys after endpoint wrapping.
    pub fn remove_rows(&mut self, start: i32, count: i32) -> bool {
        let end = start.wrapping_add(count);
        let keys: Vec<_> = self.rows.keys().copied().filter(|k| *k >= start && *k < end).collect();
        for row in keys {
            self.count = self.count.wrapping_sub(self.rows[&row].count);
            #[cfg(test)]
            self.events.push(Event::Count(self.count));
            self.erase_row_normal(row);
        }
        true
    }

    pub fn remove_columns(&mut self, start: i32, count: i32) -> bool {
        let end = start.wrapping_add(count);
        let rows: Vec<_> = self.rows.keys().copied().collect();
        for (index, &row) in rows.iter().enumerate() {
            let keys: Vec<_> = self.rows[&row]
                .cells
                .keys()
                .copied()
                .filter(|k| *k >= start && *k < end)
                .collect();
            for column in keys {
                self.erase_cell_normal(column, row);
            }
            // Capture successor before erasing this row, never traverse an
            // erased node. Snapshot contains ascending original row keys.
            let next_row = rows.get(index + 1).copied();
            #[cfg(test)]
            self.events.push(Event::NextRow(row, next_row));
            let _ = next_row;
            if self.rows[&row].count == 0 {
                self.erase_row_normal(row);
            }
        }
        true
    }

    fn visit_range(&self, range: Bounds, mut hit: impl FnMut(i32, i32, &Cell<W>)) -> u32 {
        let mut count = 0u32;
        for (&row_key, row) in &self.rows {
            if row_key < range.row_min {
                continue;
            }
            if row_key > range.row_max {
                break;
            }
            for (&column, cell) in &row.cells {
                if column < range.column_min {
                    continue;
                }
                if column > range.column_max {
                    break;
                }
                hit(column, row_key, cell);
                count = count.wrapping_add(1);
            }
        }
        count
    }

    pub fn count_range(&self, range: Bounds) -> u32 {
        self.visit_range(range, |_, _, _| {})
    }

    pub fn min_max_columns_for_row(
        &self,
        row: i32,
        io: &mut impl WordIo,
        min: WordToken,
        max: WordToken,
    ) -> Result<bool, DomainError> {
        validate(io, min, 1)?;
        validate(io, max, 1)?;
        io.write_word(min.0, i32::MAX as u32);
        io.write_word(max.0, i32::MIN as u32);
        let Some(row) = self.rows.get(&row) else {
            return Ok(false);
        };
        let first = *row.cells.first_key_value().expect("nonempty row").0;
        let last = *row.cells.last_key_value().expect("nonempty row").0;
        io.write_word(min.0, first as u32);
        io.write_word(max.0, last as u32);
        Ok(true)
    }

    pub fn min_max_rows_for_column(
        &self,
        column: i32,
        io: &mut impl WordIo,
        min: WordToken,
        max: WordToken,
    ) -> Result<bool, DomainError> {
        validate(io, min, 1)?;
        validate(io, max, 1)?;
        io.write_word(min.0, i32::MAX as u32);
        io.write_word(max.0, i32::MIN as u32);
        let mut found = false;
        for (&row_key, row) in &self.rows {
            if row.cells.contains_key(&column) {
                io.write_word(min.0, row_key as u32);
                io.write_word(max.0, row_key as u32);
                found = true;
                break;
            }
        }
        if found {
            for (&row_key, row) in self.rows.iter().rev() {
                if row.cells.contains_key(&column) {
                    io.write_word(max.0, row_key as u32);
                    break;
                }
            }
        }
        Ok(found)
    }

    pub fn max_used_column(&self, io: &mut impl WordIo, row_out: WordToken) -> Result<i32, DomainError> {
        validate(io, row_out, 1)?;
        io.write_word(row_out.0, i32::MIN as u32);
        let mut maximum = i32::MIN;
        for (&row_key, row) in &self.rows {
            let column = *row.cells.last_key_value().expect("nonempty row").0;
            if column > maximum || maximum == i32::MIN {
                maximum = column;
                io.write_word(row_out.0, row_key as u32);
            }
        }
        Ok(maximum)
    }

    pub fn column_begin(&self, column: i32) -> ColumnIterator<'_, W> {
        let mut result = ColumnIterator {
            matrix: self,
            current: None,
        };
        for (&row_key, row) in &self.rows {
            // §6 scans even past larger column keys rather than lower_bound.
            for &key in row.cells.keys() {
                if key == column {
                    result.current = Some((row_key, column));
                    return result;
                }
            }
        }
        result
    }
}

/// Three logical 32-bit words (12 bytes), copied values, never addresses.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeRecord {
    pub column: i32,
    pub row: i32,
    pub payload: u32,
}

impl SparseMatrix<1> {
    /// Capacity validation is a portable precondition, not a native capacity
    /// error. None performs count-only selection without any output writes.
    pub fn range_records(&self, range: Bounds, output: Option<&mut [RangeRecord]>) -> Result<u32, DomainError> {
        let Some(output) = output else {
            return Ok(self.count_range(range));
        };
        // Use usize, not wrapped native count, for safe output sizing.
        let mut required = 0usize;
        self.visit_range(range, |_, _, _| required += 1);
        if output.len() < required {
            return Err(DomainError::InsufficientRangeCapacity);
        }
        let mut index = 0;
        Ok(self.visit_range(range, |column, row, cell| {
            output[index] = RangeRecord {
                column,
                row,
                payload: cell.payload[0],
            };
            index += 1;
        }))
    }
}

/// Opaque logical record identities simulate alias relationships, not native
/// addresses/layout. Retagging only models the ignored matrix tuple field.
#[derive(Clone, Copy, Debug)]
pub struct IteratorIdentity {
    matrix: MatrixIdentity,
    row: Option<RowIdentity>,
    cell: Option<CellIdentity>,
}

impl IteratorIdentity {
    pub fn with_matrix_identity(self, matrix: MatrixIdentity) -> Self {
        Self { matrix, ..self }
    }
    pub fn matrix_identity(self) -> MatrixIdentity {
        self.matrix
    }
}

impl PartialEq for IteratorIdentity {
    fn eq(&self, other: &Self) -> bool {
        if self.cell.is_none() && other.cell.is_none() {
            true
        } else {
            self.row == other.row && self.cell == other.cell
        }
    }
}
impl Eq for IteratorIdentity {}

/// Borrowing prevents mutation while iterating (SM4). No Iterator::next default
/// is supplied: incrementing end explicitly reports unsupported behavior.
pub struct ColumnIterator<'a, const W: usize> {
    matrix: &'a SparseMatrix<W>,
    current: Option<(i32, i32)>,
}

impl<const W: usize> ColumnIterator<'_, W> {
    pub fn identity(&self) -> IteratorIdentity {
        let (row, cell) = match self.current {
            Some((row, column)) => {
                let row = &self.matrix.rows[&row];
                (Some(row.identity), Some(row.cells[&column].identity))
            }
            None => (None, None),
        };
        IteratorIdentity {
            matrix: self.matrix.identity,
            row,
            cell,
        }
    }

    pub fn current(&self) -> Option<(i32, i32, &[u32; W])> {
        let (row, column) = self.current?;
        Some((column, row, &self.matrix.rows[&row].cells[&column].payload))
    }

    pub fn increment(&mut self) -> Result<(), DomainError> {
        let (previous_row, column) = self.current.ok_or(DomainError::UnsupportedEndIncrement)?;
        self.current = None;
        for (&row_key, row) in &self.matrix.rows {
            if row_key <= previous_row {
                continue;
            }
            for &key in row.cells.keys() {
                if key == column {
                    self.current = Some((row_key, column));
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
