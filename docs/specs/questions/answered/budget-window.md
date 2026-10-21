# budget-window: answered questions (archive)
<!-- Archive for [budget-window.md](../../budget-window.md). Lanes do not read this file. Open questions are in ../budget-window.md. -->

<!-- from: re-panels4 (2026-10-21) -->
- ~~**BW1 - section 4 group mode:** groups `0x6A357B96`, `0xEA597195`, `0x0A5A72D1` are tested at `0x0078807C`-`0x00788092` in `on_init` and take another row layout. What differs (one slider per department vs one per item, per-building rows through `0x0ABCE6xx`)?~~
  **ANSWERED 2026-10-21 (re-budgetwindow4):** DIFFERS — no other row layout. The three groups differ only in the
  department header rows: the income-list header needs `GetTotalIncome() != 0` and the expense-list header needs
  `GetIdealMonthlyFunding() != 0`; other groups always add the header. Lists A (income, count `+0x124`) and B
  (expense, `+0x128`), item visibility (ShowLineItem bit and `ID > 100` or non-zero amount), 9 visible rows each,
  empty-window fallback: budget-window.md §4.1 (with a worked example). Per-building rows `0x0ABCE6xx` open a nested
  individual dialog (§5).

<!-- from: re-panels4 (2026-10-21) -->
- ~~**BW2 - section 5 button 0xCF:** what does `FUN_0077AB40` do before the dialog closes (neighbour mode: accept the selected deals)?~~
  **ANSWERED 2026-10-21 (re-budgetwindow4):** `ApplyDealEdits` commits each held deal's lastAmount / lastIncAmount
  (the combo box's edited values): an Active deal takes them, or is cancelled when the edit is 0; a Cancelled deal
  with a non-zero edit is re-activated and takes them. Order and worked cases: budget-window.md §5.4.

<!-- from: re-panels4 (2026-10-21) -->
- ~~**BW3 - section 5 occupant controls:** what do buttons `0x0ABCDE00`-`0x0ABCDEFF` do in occupant mode, and what are funding levels 0 and 1 of `SetOccupantFundingLevel` (`0x0077BC90`)?~~
  **ANSWERED 2026-10-21 (re-budgetwindow4):** `+0x24` is a department ID, not an occupant (§3). `0x0ABCDExx` in
  individual mode is a camera jump to the row's building (§5.7). `which` 0/1 select the per-service funding pair
  chosen by the service kind `+0x134` (set from the department ID); slider IDs below `0x0ABCE500` give 0, from it up
  give 1 (§4.4, §5.5 table).

<!-- from: core-budgetwindow4 (2026-10-21) -->
- ~~**BW7 - section 5 scrolling and the button tail:** (a) upper bound of a list position, length field of list A, which mode fills which list; (b) the arrow rule; (c) which buttons take the "mode refresh + `UpdateValues`" tail. [core: `len - 9`; arrows set after each scroll; every button not returning 1 takes the tail]~~
  **ANSWERED 2026-10-21 (re-budgetwindow4):** DIFFERS — (a) steps of 2 while `pos + 9 < len`, so the largest
  position is the first even value with `pos + 9 >= len` (len 12: 0, 2, 4), not `len - 9`; list A length `+0x124`,
  A = income items, B = expense items, group mode. (b) up hides at 0 and shows down when `pos + 9 < len`; down hides
  itself at the end and shows up unconditionally (even on a list that fits). (c) there is no common tail: only the
  scroll buttons refresh (per-mode table §5.3); `0xCD`/`0xCE`/`0xCF` end the dialog with no refresh; unmatched IDs
  return 0 with no refresh. budget-window.md §5 table, §5.3.

<!-- from: core-budgetwindow4 (2026-10-21) -->
- ~~**BW8 - sections 4.4, 5.1 numeric types:** slider position type and precision of `position x 0.01`; type of `SetOccupantFundingLevel`'s level and of `IndividualRestoreInfo` level0/level1; when the restore records are filled. [core: double product stored as f32; levels i32]~~
  **ANSWERED 2026-10-21 (re-budgetwindow4):** DIFFERS — the position is an i32; the group fraction is the f32
  product `f32(position) * 0.01f` (constant `0x3C23D70A` at `0x00A823A4`, rounded once), not the f64 product rounded
  to f32: 59 of positions 0-200 differ, e.g. 30 gives `0x3E999999`. The level is **f32** in percent
  (`f32(position)`, unscaled), and `IndividualRestoreInfo` holds two f32 levels (12-byte records). §4.4, §5.1, §5.5.
  The fill point of the restore records (`initIndividualBudget`) is not re-checked here.
