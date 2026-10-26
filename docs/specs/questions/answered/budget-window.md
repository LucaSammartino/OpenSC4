# budget-window: answered questions (archive)

<!-- from: core-budgetwindow-ownership4 (2026-10-22); narrowed by re-budget-window-bw22 -->
- **BW17 - sections 1, 4, 4.1, 5.2, 5.6 and 8 whole-owner inventory / omitted bodies:** please publish the full method/slot inventory for `cSC4SubBudgetWindow` and the owned budget-panel paths, and the non-layout algorithms still omitted: ordinance enumeration/filtering, A/B counts and row-map construction in `initOrdinanceBudget`; proposed-deal enumeration, section/count rules and `+0x50` selection edits in `initNeighborBudget`/combo handling; occupant enumeration and the exact creation/ownership of `+0x44` individual restore records; all four per-mode refreshes (including group row reuse on scroll), empty/popup window creation and failure reference rules, and the panel loan-display/balance-sheet bodies. Which public external interface calls and arguments supply their raw inputs? [unimplemented: existing same-class populate/refresh host callbacks remain explicit gaps, not core coverage; no native own-class result may count as an external input. BW14/BW15 ownership and BW16 visibility are implemented from their settled text; the documented BW13 null-content divergence is retained. No whole-owner completion claimed.]

  **ANSWERED 2026-10-24 (re-budget-window-bw22):** DIFFERS — partial closure, with remaining paths split into BW22–BW25, not a whole-owner proof. Companion §1 gives the method inventory; §2 now adds period scale (zero→1, any nonzero→12), ordinance refresh, positional edit/map reuse, signed B formatting and vectors. Existing §3 settles six neighbour scans/counts/edits; §5 settles occupant enumeration and +0x44 record acquisition/references, leaving per-service float production BW23; §6 proves the bounded group-A slice, leaving BW24. Panel balance-sheet/formatting/loan bodies are §4.2–§4.3 and §8–§10, with their summary gaps still BW18. BW22 covers neighbour remainder; BW25 covers setup/property/popup/widget ownership. No same-owner populated row/refresh result becomes an external input through this split. Main §8 and companion §7 carry this boundary.

  **ANSWERED 2026-10-26 (re-bw30):** DIFFERS — the duplicate open parent was stale. Companion §12 now maps
  every requested part to settled rules and its outstanding child question, and lists public raw-input calls
  with worked balance-sheet and loan cases. BW18's former panel summaries are already settled in §11; no
  omitted cSC4BudgetUIWinProc body remains under BW17. New §§1.2–1.3 settle non-owning occupant properties,
  segment ownership, setup-tail generator references, and empty-window construction including the second
  factory's null-output fault. BW22–BW25 still cover static sub-budget body gaps; no runtime observation can
  replace those algorithms. This remains a partial parent closure, not whole-cSC4SubBudgetWindow completion
  or acceptance of same-owner result callbacks as external inputs.

<!-- from: re-budget-window-bw22 (2026-10-24); BW21 supplied in lane brief, absent from open file at this base. -->
- **BW21 — constructor ABI, allocation, nonmodal population and observation:** settle the constructor arguments, matching allocation/free service, the minimum lifecycle computing group rows/totals/strings without a modal window, and the exposed state.

  **ANSWERED 2026-10-24 (re-budget-window-bw22):** DIFFERS — §1.1 specifies a no-explicit-argument thiscall constructor, ECX storage/EAX same pointer, GlobalHeapAlloc(0x140)/GlobalHeapFree, and the deleting-destructor flags word. Population can avoid DoModalWin but still needs generated windows, normal init, providers/resources, a valid parent and balanced pause/teardown; there is no window-free native calculation API. Counts, offsets and reference vectors are fields; +0x90/+0x94 is reused section scratch and strings live in child controls, with no primary-vtable result getter. Static lifecycle vectors are in §9; no native off-screen run is claimed.

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

<!-- from: re-panels4 (2026-10-21) -->
- ~~**BW4 - section 2.1 panel slot 6:** commands `0xAA3AC40E`/`0xAA3AC40F` (and `0x4A8879CB`/`0x4A8879DB`) set `+0x18C` to 1/0 and call the budget proc's slot 6 (`+0x18`). Which method is it?~~
  **ANSWERED 2026-10-21 (re-budgetwindow4):** slot 6 is `ShowWindow` (`0x00768670`). The byte is primary `+0x194`
  (the handler runs on the sub-object at `+8`), the tab flag `ShowWindow` reads: 1 shows tab `0xAA3AC001` and hides
  `0xAA3AC000`, 0 the reverse; then the balance sheet is refreshed; the command returns 0. Which tab is compact is
  not in the bytes. budget-window.md §2.1.

<!-- from: core-budgetwindow4 (2026-10-21) -->
- ~~**BW6 - sections 2, 3, 6 references and pause balance:** (a) who releases `*out` and `+0x0C`? (b) is `HiddenResume` guarded when `CreateSubBudgetWindow` failed? [core implements: no Release of either; resume always]~~
  **ANSWERED 2026-10-21 (re-budgetwindow4):** (a) DIFFERS — the panel Releases `*out` right after the modal run and
  location save (`0x007685DB`); `+0x0C` is released by `on_shutdown` (nulled first) when the window was initialised,
  else by the destructor's base cleanup (`0x0077C4C0`); `DestroySubBudgetWindow` releases neither. (b) MATCHES —
  the resume is unguarded (`0x0077A22C`); the pause is a counter with no floor, so a failed create leaves it one
  lower (worked case and tests in budget-window.md §6, §9 tests 7-8). The nested individual dialog has its own
  pause/resume pair.

<!-- from: core-budgetwindow4 (2026-10-21) -->
- ~~**BW9 - sections 4, 5 unspecified routing:** `on_shutdown`; `DoWinMsg` return values; `end_dialog`'s second argument; who creates the child at `+0x14`; ordinance check IDs; `SetOrdinanceOn` arguments. [core takes child and toggles as typed host input and returns "unspecified"]~~
  **ANSWERED 2026-10-21 (re-budgetwindow4):** DIFFERS (core left these unspecified) — `on_shutdown` empties the
  dialog's vectors, frees the ordinance arrays, releases `+0x0C` and `+0x10`, returns 1, and does nothing when not
  initialised (§4). Returns: init/shutdown/button pass through their handler's result; slider, combo, ignored keys
  and other messages 0; close keys 1 (§4). `end_dialog`'s second argument is the modal result code passed to the
  parent's end-modal call, ignored by the panel (§6). `+0x14` is set by the ordinance info popup (`500 + r`) and by
  `SetEmptyWindow` (§5.6). Check boxes are IDs 300-499, info buttons 500-699 (§5 table, §5.6). `SetOrdinanceOn`
  takes (ordinance ID from `+0xA4`, edited byte zero-extended) only for rows whose edited byte differs (§5 table).

<!-- from: re-panels4 (2026-10-21) -->
- **BW5 - section 4 key close:** key messages `0x0D` and `0x73` close the dialog only when modifier bit `0x4` is set. Is `0x73` the virtual key F4 (Alt+F4) and bit `0x4` Alt, and does plain Enter really do nothing? [yes to F4 by key value; plain Enter returns 0 with no effect (byte-checked at `0x0078BCF0`, re-budgetwindow4); the meaning of bit `0x4` is not in the dialog code and needs a game check]
  **ANSWERED 2026-10-21 (re-budgetwindow-answers4):** MATCHES (static) — bit `0x4` is Alt: the GZ Win32 translator `FUN_0098D230` sends `WM_KEYDOWN` and `WM_SYSKEYDOWN` to the type-5 builder `FUN_0098C400`, whose mask (`FUN_0098B6E0`) sets `0x1` Shift, `0x2` Ctrl, `0x4` Alt (`GetKeyState(VK_MENU)`, `0x0098B702`), `0x10/0x20/0x40` mouse buttons, `0x40000000` repeat. So Alt+Enter and Alt+F4 close; plain Enter/F4 do nothing (spec §4.0). Whether the keystrokes reach the dialog at runtime is the new BW11.

<!-- from: core-budgetwindow-fix4 (2026-10-21) -->
- **BW10 - sections 4, 4.1 remaining on_init contracts:** (a) who sets `+0x10` (released by `on_shutdown` and the destructor)? (b) group mode with both counts 0 (empty window): are the `SliderInfo` rows (`+0x2C`) and the saved fractions (`+0x38`) still built, so a later Cancel restores them? (c) which state does `on_init` give the four scroll arrows `0x451`-`0x454` ("on_init sets their initial state", §5.3)? [core: (a) holds nothing at `+0x10`; (b) builds neither; (c) sets no arrow at init]
  **ANSWERED 2026-10-21 (re-budgetwindow-answers4):** DIFFERS — (a) `on_init` creates a GZCOM window (class `0x4386D516`), stores its `cIGZWin` with a reference at `+0x10`, sets flags, `SetID(0x0423278E)` and adds it as a child of the dialog, in every mode, before the selector branch (`0x00787A9E`–`0x00787BD5`, spec §4 "Content window"). (b) `+0x2C` is not built, but `+0x38` **is**: one {fraction, department} pair per group department, filled in the counting loop before the empty test (`0x00787FDB`–`0x00788045`, spec §4 "Order and the empty group"). (c) group mode: `0x451`/`0x453` hidden always, `0x452`/`0x454` hidden when their count ≤ 9 (`0x007896AB`–`0x0078973A`); the empty branch creates no arrows (spec §5.3 "Initial arrow state").

<!-- from: coordinator review (2026-10-21) -->
- **BW12 - section 5.3, selector-specific initial arrow bounds:** the general group on_init bounds at 0x007896C9/0x00789714 are proven, but initOrdinanceBudget (0x0077D620-0x0077D780) and initNeighborBudget (0x00781AD1-0x00781C4D) were not byte-checked. Which count fields and exact comparisons control each initial down arrow, and do these helpers differ from general group mode? Do not extend the general <=9 rule to these copies without proof. [open: prevents whole-owner credit for guessed selector-specific initialization]
  **ANSWERED 2026-10-21 (re-budget-arrow4):** DIFFERS — both helpers hide `0x451`/`0x453` always and test the unsigned u32 counts `+0x124` (A, drives `0x452`) and `+0x128` (B, drives `0x454`) against 9, but the ordinance helper (selector 1) hides a down arrow only when its count is < 9 (`JAE` at 0x0077D740/0x0077D782), so with exactly 9 rows its down arrow stays as created; the neighbour helper (selector 2) hides when count <= 9 (`JA` at 0x00781BEC/0x00781C2E), same as group mode. Spec section 5.3 "Selector-specific initial arrow state", test 18.

<!-- from: re-budget-core-final4 (2026-10-21) -->
- **BW13 - section 4 content window `+0x10` create failure:** "If that fails, `+0x10` stays null and nothing below runs." Does "nothing below" mean only the flag / `SetID` / `ChildAdd` / generator-release steps of that block (on_init then still takes the selector branch, builds the mode, sets `+0x08 := 1` and returns 1), or does on_init skip the mode branch too (and then: is `+0x08` set, what is returned, is the hidden pause still held)? Core implements the first reading (core-budgetwindow-final4, `create_content` in engine/core/src/budget_window/dialog.rs). [open]
  **ANSWERED 2026-10-21 (re-budget-core-final4):** MATCHES — a failed `CreateInstance` (`JZ` 0x00787AD2) skips only the rest of the content block (no `+0x10`, flags, `SetID`, `ChildAdd`), releases the generator out-pointer if non-null (0x00787BDA) and falls into the selector branch (0x00787BE7); the hidden pause stays held, `+0x08 := 1`, return 1. DIFFERS downstream: the native passes the null `+0x10` to `add_text`, which dereferences it unconditionally (0x0077982B), so the original faults; the spec makes the no-fault path a *[divergence]* with child insertion into a missing `+0x10` a no-op. Spec section 4 "Failed content creation", test 19.
- **BW14 - section 4 (BW10b) the department reference in `+0x38`:** each saved pair holds "the department with a reference". Is that an AddRef on the department (`cISC4DepartmentBudget`), and if so where is it released: per element when `+0x38` is emptied (at `0x00787F8B` in on_init and in on_shutdown), in the destructor, or never? The core keeps the department as an opaque handle and makes no AddRef/Release call for it. [open]
  **ANSWERED 2026-10-21 (re-budget-core-final4):** DIFFERS — each `+0x38` pair owns one counted reference: temp pair AddRef (0x00787FF6), element AddRef on append (0x00788023, or copy-construct in grow path `FUN_0077D7E0`), temp Release at 0x007881BD. Released per element by the erase routine 0x0077C420 (on_init 0x00787F8B, on_shutdown 0x0077E46F) and by the destructor via 0x005FF0C0 (0x0077C592). Core's uncounted handle does not match. Spec section 4 "Department references in the saved pairs", test 20.

<!-- from: re-budget-slider-ownership4 (2026-10-22) -->
- **BW16 - section 4.1 row-visibility ID test direction (static):** §4.1 rule 2 says an item is shown if `GetID() > 100` or its amount is non-zero. The bytes at all four sites (count A `0x007880D3`, count B `0x00788147`, build A `0x007886ED`, build B `0x00788F33`) are `CMP EAX,0x64; JBE <shown>`, then for ID > 100 a 64-bit amount test (`OR EAX,EDX; JZ <hidden>`): i.e. ID ≤ 100 is always shown (subject to display flag bit 0) and ID > 100 is hidden when its amount is zero. Is slot 3 really `GetID`, and should §4.1 (and its worked example) be inverted? [open: needs a reviewer of §4.1; BW15 tests avoid the case]
  **ANSWERED 2026-10-22 (re-budget-slider-ownership4):** DIFFERS — slot 3 is GetID: line item primary vtable 0x00AB0F58 slot 3 = 0x006AAEB0 returns object +0x54, which the department line-item factory FUN_006AB040 sets from its first argument (0x006AB076) and uses as the key of the department map +0x4C (0x006AB0A8). At all four sites (0x007880D3, 0x00788147, 0x007886ED, 0x00788F33) CMP ID,0x64 / JBE goes to "shown"; only ID > 100 reaches the 64-bit amount test, zero hides. Rule: a flagged item is shown if ID <= 100 or its amount is non-zero. Spec section 4.1 rule 2 and worked example corrected; tests 26-27.

<!-- from: re-budgetwin (2026-10-22) -->
- ~~**BW18 - budget-window-bodies.md §8, §10, the panel's summary parts:** the construction (`0x007658A0`) and
  `InitBalanceSheet` (`0x00764DF0`) labels, the rest of `Init` (`0x00766E10`), the loan slider's limits in
  `DisplayLoanWindow` (`0x00765C30`) and the row drawing of `UpdateLoanDisplay` (`0x00766680`). [summary]~~
  **ANSWERED 2026-10-23 (re-budgetui7):** budget-window-bodies.md §11, read on the bytes. §11.1 gives the
  construction order and the 18 label captions with their monthly and yearly strings; the yearly set is never used,
  because construction is the only caller and runs in the monthly view *[quirk]*. §11.2 gives the 72 notification
  targets and the initial period toggles. §11.3 gives the slider's limits: range `(increment, min(room, bond limit))`,
  ties to the limit, 32-bit; zero room gives `(0, 0)` and Accept off. §11.4 gives the row cells, the payment total
  and the scroll-button tests. The list redraw drops the bond-limit cap *[quirk]*. §11.5 identifies `DoMessage`'s
  dirty byte with slot 8's `+0x19E`.

<!-- from: re-budgetwindow-answers4 (2026-10-21); narrowed by re-bw30 -->
- ~~**BW11 - section 4.0 Alt+Enter / Alt+F4 delivery (runtime):** statically, modifier bit `0x4` is Alt (`GetKeyState(VK_MENU)` in `FUN_0098B6E0`) and `WM_SYSKEYDOWN` reaches the same type-5 builder as `WM_KEYDOWN`, so the dialog closes on Alt+Enter / Alt+F4. In the running game, with a sub-budget dialog open: does Alt+Enter close it (and nothing else happen), and does Alt+F4 close only the dialog or also start the application quit (`cSC4App::RequestQuit`) or a system close? [open: needs a game run; the translator's hook loop at `0x0098D2B6` and keyboard accelerators were not traced]~~
  **ANSWERED 2026-10-26 (re-bw30):** DIFFERS — §4.0 now distinguishes delivered type-5 behavior from live
  input routing. Hooks receive an explicit consumed out-byte; only that byte stops traversal. Unconsumed
  system key-down calls the builder and returns 0 without default-window processing, even when the builder
  suppresses input. The dialog close body itself neither switches display mode nor requests quit. A separate
  WM_CLOSE submits type 10. The remaining runtime observation is specified in §4.0: separate fresh-dialog
  Alt+Enter/Alt+F4 trials, display/popup state, visible close/quit effects, and ordered hook/type-5/system-close
  plus RequestQuit entry observations. No in-game verdict is asserted; the runtime part is narrowed, not measured.
