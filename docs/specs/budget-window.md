# Budget window family: `cSC4BudgetUIWinProc` and the sub-budget dialog `cSC4SubBudgetWindow` (behaviour spec)

Status: first pass (re-panels4, 2026-10-21). Covers the **lifecycle, command routing and data contracts** of the two
largest SC4-specific panel owners (`cSC4SubBudgetWindow` 71 235 B, `cSC4BudgetUIWinProc` 17 952 B; ui-windows.md
§14.3). It does not re-specify the money model (budget.md) or generic widgets (the widget lane). The per-row layout
code (`initNeighborBudget` 13 933 B, `updateNeighborBudget` 14 988 B, `initOrdinanceBudget` 4 468 B, the `add_*`
helpers) is summarised by contract only; pixel layout is §8 open work.

Evidence: Ghidra export via `tools/re_query.py` (asm, callers, callees, vtable) and decomp/. REA was **not available**
in this round (no binary open in the shared session), so every claim below is from the export; the decompiler output
of `on_init` and `DoSubBudgetWindow` is stack-mangled and was checked against the instructions at the cited addresses.

## 1. Identities *[compat]*

| Item | Value |
|---|---|
| `cSC4SubBudgetWindow` primary vtable | `0x00AB5494`, 6 slots: 0 QueryInterface, 1 AddRef, 2 Release, 3 `cIGZWinProc::DoWinProcMessage` thunk, 4 `DoWinMsg` `0x0078BCA0`, 5 deleting dtor `0x0077E420` |
| `cSC4SubBudgetWindow` size | `0x140` bytes (operator new at `0x00768478` via `DoSubBudgetWindow`) |
| Dialog window object | created through GZCOM: class `0x4386D516`, interface `0x5386D516` (the generic window-generator class; the same IID is queried again in `DoSubBudgetWindow`) |
| Dialog window ID | `SetID(0x0423278F)` (`cIGZWin` slot 64, `+0x100`, at `0x00779F96`) |
| `cSC4BudgetUIWinProc` vtables | `0x00AB4A40`, `0x00AB4A2C`, `0x00AB4A1C` |
| Budget-panel tab windows | `0xAA3AC000` and `0xAA3AC001` (ShowWindow `0x00768670` shows one, hides the other by the flag at `+0x194`) |

## 2. Opening a sub-budget dialog (`cSC4BudgetUIWinProc::DoSubBudgetWindow`, `0x00768460`) *[compat]*

Input: one 32-bit **selector**. Only the budget panel's button handler calls it (9 call sites in `DoWinProcMessage`
`0x00768750`). Steps, in order:

1. If a sub-budget object is already held (`+0x18`), do **nothing** (no re-entry, no refresh).
2. Allocate a `cSC4SubBudgetWindow` (0x140 B), construct it (`0x007862B0`), hold one reference at `+0x18`.
3. `CreateSubBudgetWindow(out win, selector, this+0x198 value, 0)` (§3). On failure skip to step 5.
4. On success: `ShowWindow`; `SetFlag(0x800, 1)`; `PullToFront`; `SetFlag(0x800, 0)`; `SetFlag(0x100, 1)`; if the
   window answers interface `0x5386D516`, set generator flags `0x1000`, `0x2000`, `0x4000` to 1. Then
   `WinLocationSaver.RestoreWindowLocation(win, 1)` (app slot 43 service, `+0x28`), **run the window modally**
   (`DoModalWin`, window manager `+0xA4`, `0x00768598`), hand focus back to the 3D view window (manager `+0x94`), and
   `WinLocationSaver.SaveWindowLocation(win)` (`+0x18`). The dialog's position therefore persists per window ID.
5. Always: `DestroySubBudgetWindow` (§6), drop the held object (`+0x18 := null`, Release), then
   `UpdateBalanceSheetInfo` (`0x00767990`) so the main panel shows any changed funding.

### 2.1 Selector table (from the panel's command IDs) *[compat]*

| Panel command (`0x287259F6` button message, value) | Selector | Dialog mode |
|---|---|---|
| `0xAA3AC401`, `0xAA3AC409`, `0xAA3AC501`, `0xAA3AC509` | `1` | ordinances (§4.1) |
| `0xAA3AC402`, `0xAA3AC40A`, `0xAA3AC502`, `0xAA3AC50A` | `2` | neighbour deals (§4.2) |
| `0xAA3AC403`, `0xAA3AC503` | `0x0A5A72D1` | department group (civic group, civic-buildings.md) |
| `0xAA3AC404`, `0xAA3AC405`, `0xAA3AC504`, `0xAA3AC505` | `0xAA369059` | group: transportation |
| `0xAA3AC406`, `0xAA3AC506` | `0x4A357B40` | group: police/fire/jail (protection.md) |
| `0xAA3AC407`, `0xAA3AC507` | `0x6A357B7F` | group: education/health |
| `0xAA3AC408`, `0xAA3AC508` | `0x4A357EAF` | group: utilities (power, water, waste) |
| `0xAA3AC40B`, `0xAA3AC50B` | `0x6A357B96` | group: parks/landmarks |
| `0xAA3AC40C`, `0xAA3AC50C` | `0xEA597195` | group: the second civic group (civic-buildings.md, department `0xEA59717A`) |

The `0xAA3AC4xx` set is the compact panel, the `0xAA3AC5xx` set the expanded one; both map identically. Every
command first passes `nsSC4TutorialSystem::FilterUICommandMessage`; a filtered command is swallowed (returns 1).

Other panel commands seen in the same handler, for completeness: `0xAA3AC400`/`0xAA3AC500` → `DisplayTaxesWindow`;
`0xAA3AC40D`/`0xAA3AC50D` → `DisplayLoanWindow`; `0xAA3AC40E`/`0xAA3AC50E` and `0x4A8879CB` set `+0x18C := 1` then
call the proc's own slot 6 (`+0x18`); `0xAA3AC40F`/`0xAA3AC50F` and `0x4A8879DB` set `+0x18C := 0` then the same
slot; `0x0A4C3354` → `RemoveLoanWindow`; `0x2A4C33A6` / `0xAA4C339C` scroll the loan list by 2 (bounded below by 0 and
above by count − 4); `0xAA4C33E6` takes the loan (§5.2).

## 3. `CreateSubBudgetWindow` (`0x00779E50`, thiscall, 4 args, returns bool) *[compat]*

1. `*out := null`. Create the generator (`0x4386D516` / `0x5386D516`); failure → return false.
2. `*out :=` its `cIGZWin` (AddRef); also hold it at `this+0x0C`.
3. Generator flags, in this order: `1:=1`, `2:=0`, `0x10000:=0`, `0x20:=0`, `0x40:=0`, `0x1000:=1`, `0x2000:=0`,
   `0x4000:=0`; set the generator's window procedure to **this object** (generator `+0x1C`).
4. Window flags: `SetFlag(0x100,1)`, `SetFlag(0x10000,0)`, `SetFlag(4,1)`, `SetFlag(0x800,0)`; `SetID(0x0423278F)`.
5. Store `+0x20 := selector`, `+0x28 := arg3`, `+0x24 := arg4`. Return true. `arg4` is a **department ID**, not an
   occupant (re-budgetwindow4, `initIndividualBudget` compares `+0x24` with department IDs at `0x00786815`–`0x007869EE`
   and passes it to `GetDepartmentBudget`, budget slot 28 `+0x70`, at `0x007867F6`). The panel passes 0 (group,
   ordinance and neighbour modes); `DoIndividualBudgetWindow` (`0x00786530`) passes the clicked row's department ID
   with selector 0 and the parent's `+0x28` (`0x0078658B`–`0x0078659A`). Below, "individual mode" means `+0x24 ≠ 0`.

The window is empty until the window manager delivers the init message (§4).

## 4. Window procedure `DoWinMsg` (`0x0078BCA0`) *[compat]*

| Message | Action |
|---|---|
| `0xA2BF8AD0` (init) | `on_init` (`0x007876B0`) |
| `0xA2BF8AD1` (shutdown) | `on_shutdown` (`0x0077E440`) |
| 3 / `0x287259F6` (button) | `on_button_activated(win, buttonID)` (`0x0078B120`), §5 |
| 3 / `0x887113A4` (slider moved) | §4.4 |
| 3 / `0xE8711255` (combo box changed) | neighbour mode: rewrite the selected deal rows (vector `+0x50..+0x54`, 24-byte entries), then `updateNeighborBudget` |
| 5 (key down), key `0x1B` (Esc) | close: remove the child held at `+0x14` (window `+0x3C`), drop it, `end_dialog(win, 0x42B7C353)`, return 1 |
| 5, key `0x0D` or `0x73` | the same close, **only** when modifier bit `0x4` is set; otherwise ignored |

`on_init` first takes a **hidden simulator pause** (`cISC4Simulator::HiddenPause`, slot 10, at `0x007878C0`);
`DestroySubBudgetWindow` releases it (`HiddenResume`, slot 13). The date does not advance while any sub-budget dialog
is open. It then branches on the selector (`+0x20`, `0x00787BE7`):

- **1 → ordinances**: load the background image (type `0x856DDBAC`, group `0x46A006B0`, instance `0x140155F2`), then
  `initOrdinanceBudget` (`0x0077C660`): one row per ordinance with an on/off check, cost text; the original on/off
  state and the edited state are kept as byte pairs (`+0xA8`, count `+0xA0`).
- **2 → neighbour deals**: image instance `0x140155D2`, then `initNeighborBudget` (`0x0077E600`), which calls
  `NeighborsSimulator.SetUIDisableFlag(true)` and `ProposeDeals(list, true)` (neighbors.md §7).
- **`+0x24` non-null → one occupant** (`initIndividualBudget`, `0x00786690`).
- **otherwise → a department group**: `budget.GetDepartmentBudgetsInGroup(selector, list)` (budget slot 31,
  `+0x7C`), one slider row per department (`SliderInfo` = {slider window ID, department}, vector `+0x2C..+0x30`), the
  starting funding fraction of each department saved (vector `+0x38..+0x3C`, 8 bytes each), totals with
  `SC4MakeMoneyString`. Groups `0x6A357B96`, `0xEA597195`, `0x0A5A72D1` differ only in which department header rows
  are shown (§4.1).

### 4.1 Group mode: the two lists and the special groups *[compat]* (BW1, BW7a)

The group dialog has two scrolling lists, each showing at most **9** rows:

| List | Rows | Row count | Scroll position | Arrow buttons |
|---|---|---|---|---|
| A | **income** line items (`GetType() == 1`) | `+0x124` | `+0x11C` | `0x451` up, `0x452` down |
| B | **expense** line items (every other type) | `+0x128` | `+0x120` | `0x453` up, `0x454` down |

`on_init` counts the rows first (`0x00787FB0`–`0x007881D8`). For each department of the group, in
`GetDepartmentBudgetsInGroup` order, it fetches the department's line items (department slot 21 `+0x54`) and walks
them in order:

1. The **first** income item of a department adds one **header row** to A; the first expense item adds one header row
   to B. In the three special groups (`0x6A357B96` parks/landmarks, `0xEA597195`, `0x0A5A72D1` the civic groups) the A
   header is added only if the department's `GetTotalIncome()` (department slot 16 `+0x40`) is non-zero and the B
   header only if its `GetIdealMonthlyFunding()` (slot 14 `+0x38`) is non-zero (tests at `0x0078807C`–`0x007880AB`
   and `0x007880F7`–`0x00788123`). Other groups always add the header. A department without income items has no A
   header in any group.
2. Each item whose `GetDisplayFlags()` (line item slot 17 `+0x44`) has bit 0 (ShowLineItem) adds one row to its list
   if `GetID() > 100` (slot 3, unsigned compare with `0x64`) **or** its amount is non-zero: `GetIncome()` (slot 14
   `+0x38`) for A, `GetFullExpenses()` (slot 10 `+0x28`) for B. Items with ID ≤ 100 and a zero amount are hidden.

The same header predicate is applied again when the rows are built (`0x007884B8`, `0x00788A6E`) and on every refresh
in `UpdateValues` (`0x00789C04` for A, `0x0078A2E8` for B), so a header that was hidden stays hidden.

After counting: the background image is instance `0x140155D2` (as the neighbour mode). If **both** counts are 0, the
dialog becomes the empty window (`SetEmptyWindow`, `0x0077BEC0`, with the LTEXT group `0x6A3FF01C` instance
`0x4A5CB4B6`) and no list is built. Otherwise `SetupWindowBig(win, min(countA, 9), min(countB, 9))` (`0x0077A6F0`)
sizes the dialog, and at most 9 rows of each list are created; the rest are reached by scrolling (§5.3).

Worked example (group `0x6A357B96`): department X has an income item ID 5 with income 0 and an income item ID 200
with income 0, and `GetTotalIncome() = 0`; department Y has one expense item ID 7, full expenses 300, ideal funding
300. A: X adds no header (special group, total income 0); item 5 is hidden (ID ≤ 100, amount 0); item 200 is shown
(ID > 100). countA = 1. B: Y adds a header and item 7: countB = 2. In group `0x4A357EAF` the same data gives
countA = 2 (header always added).

### 4.4 Slider moved (`0x887113A4`)

Order (`0x0078C067`–`0x0078C17F`): get the slider child of the window by the message's control ID through the
window's interface lookup (window slot 37 `+0x94`, IID `0x21325207`). If that fails, nothing else happens. Then the
process-wide re-entry byte (`0x00B4DFC8`): if it is set, nothing happens; otherwise set it, do the work below, clear
it. The **position** is the slider's 32-bit signed integer value (slider slot 4 `+0x10`).

- Group mode (`+0x24 = 0`): for **every** `SliderInfo` whose window ID equals the control ID (no early exit), call
  that department's `SetFundingPercentage(&p, 0)` (department slot 11 `+0x2C`, by reference; budget.md §7.1, which
  redistributes) with `p = f32(position) × 0.01f` rounded **once** to f32 (`FILD`, `FMUL float [0x00A823A4]` =
  `0x3C23D70A`, `FSTP float`, `0x0078C12F`–`0x0078C142`). The product of a small integer and the f32 constant is
  exact on the x87, so this is the IEEE f32 product `position as f32 * 0.01f32`. It is **not** the f64 product
  rounded to f32: of positions 0–200, 59 differ (5, 9, 10, 15, 18, 20, …); position 30 gives `0.29999998f`
  (`0x3E999999`), not `0.3f` (`0x3E99999A`). Then `UpdateValues(win)` (`0x007898A0`) once.
- Individual mode: `item := get_window_indiv_item(win, controlID)`; if null, skip to clearing the byte. Otherwise
  `SetOccupantFundingLevel(item, which, level)` with `which = 0` for control IDs below `0x0ABCE500` and `1` from
  `0x0ABCE500` up (unsigned), and `level = f32(position)` **unscaled** (percent; the 0.01 is applied inside, §5.5);
  then `updateIndividualBudget(win, 0)` (`0x00785740`).

`get_window_indiv_item(win, id)` (`0x00779600`): the child window with that ID (window slot 35 `+0x8C`); its
property `0x4BFEE088` (window slot 113 `+0x1C4`); if the property's type (slot 3) is `0xF`, the object it holds
(slot 51 `+0xCC`), else null. The property is released; the returned object is not AddRef'd.

## 5. Buttons (`on_button_activated`, `0x0078B120`) *[compat]*

Signature `bool on_button_activated(cIGZWin* win, uint32 id)` (thiscall, `RET 8`). Dispatch order
(`0x0078B120`–`0x0078B67E`): `id ≤ 0x384` goes through a jump table on `id − 0x67` (bytes `0x0078BC28`, targets
`0x0078BC08`); above that the explicit compares; every ID not matched below returns **0 with no refresh**
(`0x0078BBF9`). "Close the child" means: if `+0x14` is non-null, `win.RemoveChild(+0x14)` (window slot 15 `+0x3C`),
then `+0x14 := null` and Release it. Every `end_dialog` call here passes `(win, 0x42B7C353)` (§6).

| Button ID | Action, in order | Returns |
|---|---|---|
| `0x67`, `0x42B7C353` | **OK**: `end_dialog` (keeps all edits) | 1 |
| `0x6D` | **Cancel**: `RestoreStartSettings(win)` (§5.1), then `end_dialog` | 1 |
| `0x68`, `0x384` | close the child (the ordinance info popup, §5.6); the dialog stays | 0 |
| `0xCC`, `0x385` | `end_dialog` | 1 |
| `0xCD` | ordinances **apply**: for `i` in `0..count(+0xA0)`, if the edited byte differs from the original byte, `OrdinanceSimulator.SetOrdinanceOn(ordinanceID[i] (+0xA4), edited)` (global `0x00B43D90`, slot 13 `+0x34`; the edited byte zero-extended); then close the child; `end_dialog` | 0 |
| `0xCE` | close the child; `end_dialog` | 0 |
| `0xCF` | neighbour **accept**: `ApplyDealEdits` (`0x0077AB40`, §5.4); close the child; `end_dialog` | 0 |
| `0x451`, `0x452`, `0x453`, `0x454` | scroll (§5.3) | 0 |
| `0x0ABCE600`–`0x0ABCE6FF` | `item := get_window_indiv_item(win, id)`; null → return 0. Else open the **nested** individual dialog for the department `item.GetID()` (slot 3) through `DoIndividualBudgetWindow` (`0x00786530`), which builds a second `cSC4SubBudgetWindow` held at `+0x1C` | 0 |
| `0x0ABCDE00`–`0x0ABCDEFF`, individual mode only | **camera jump** to the row's building (§5.7) | 0 |
| `300`–`499` (`0x12C`–`0x1F3`) | ordinance row check box: `edited[id − 300] :=` the check box state (§5.6) and `updateOrdinanceBudget(win)` | 0 |
| `500`–`699` (`0x1F4`–`0x2BB`) | open the ordinance info popup for ordinance `id − 500` (§5.6) | 0 |

There is **no** common refresh tail: only the scroll buttons refresh (§5.3); `0xCD`/`0xCE`/`0xCF` end the dialog without
a refresh, and `UpdateValues` runs from a button only on the group-mode scroll path (`0x0078BBF4`).

### 5.1 `RestoreStartSettings` (`0x0077C5E0`)

Takes the window (`RET 4`) but does not use it. The branch is on `+0x24` (individual mode), not on the selector.
Group mode (`+0x24 = 0`): for every saved {fraction f32, department} pair (8 bytes, vector `+0x38..+0x3C`), in vector
order, call that department's `SetFundingPercentage(&fraction, 0)` (slot 11). Individual mode: for every
`IndividualRestoreInfo` (12 bytes: occupant pointer, level0 **f32**, level1 **f32**; vector `+0x44..+0x48`) call
`SetOccupantFundingLevel(occupant, 0, level0)` then `(occupant, 1, level1)` (`0x0077C5F6`–`0x0077C611`); the levels
are passed as raw 4-byte floats, in percent like the slider path. Ordinance and neighbour edits are not applied
until `0xCD` / `0xCF`, so Cancel needs no restore for them. In ordinance and neighbour modes the group branch runs
over an empty vector.

### 5.2 Loan taken (`0xAA4C33E6`, budget panel)

Reads the amount chosen (`+0x188`), asks the budget simulator for the bond terms (slots `+0xC4`, `+0xBC`), builds a
bond record (amount, months/12 split) and hands it to `FUN_00692110`; then `+0x184 += amount`, `+0x17C += 1`,
`UpdateLoanDisplay`, refresh, `RemoveLoanWindow`. The bond arithmetic is budget.md §5.4.

### 5.3 Scrolling (`0x451`–`0x454`) *[compat]* (BW7)

`pos` is the list's position (A `+0x11C`, B `+0x120`), `len` its row count (A `+0x124`, B `+0x128`), all u32.
"Show/hide arrow X" is `win.GetChild(X)` (slot 35 `+0x8C`) then `SetFlag(1, on)` (window slot 68 `+0x110`; flag 1 =
visible), with no null check of the child.

| Button | Step 1 (move) | Step 2 | Step 3 |
|---|---|---|---|
| `0x451` A up | if `pos > 0`: `pos −= 2` | if `pos == 0`: hide `0x451` | if `pos + 9 < len`: show `0x452` |
| `0x452` A down | if `pos + 9 < len`: `pos += 2` | if `pos + 9 ≥ len`: hide `0x452` | show `0x451` **unconditionally** |
| `0x453` B up | as `0x451` on B | hide `0x453` at 0 | show `0x454` if `pos + 9 < len` |
| `0x454` B down | as `0x452` on B | hide `0x454` | show `0x453` unconditionally |

So the largest position is the first even value with `pos + 9 ≥ len` (for `len = 12`: 0 → 2 → 4, and 4 shows rows
4–12, one past the end; the row builders skip missing rows). A down press on a list that fits (`len ≤ 9`) does not move
but **shows** the up arrow. Nothing hides the arrows at open time in this routine; `on_init` sets their initial state.

Refresh after the move (no `UpdateValues` unless stated):

| Button | Selector 1 | Selector 2 | otherwise |
|---|---|---|---|
| `0x451`, `0x452` | `updateOrdinanceBudget(win)` | `updateNeighborBudget(win)` | `UpdateValues(win)` (also in individual mode, `0x0078B3EF`) |
| `0x453`, `0x454` | `updateOrdinanceBudget(win)` | `updateNeighborBudget(win)` | individual mode: `updateIndividualBudget(win, 0)`; group mode: `UpdateValues(win)` |

### 5.4 `ApplyDealEdits` (`FUN_0077AB40`, button `0xCF`) *[compat]* (BW2)

For each entry of the held-deal vector (`+0x68..+0x6C`, 8 bytes, the deal pointer at `+4`), in order, on the deal
`d` (deal slots per neighbors.md §5.4, SDK order; checked by body):

1. AddRef `d` (skipped if null; the calls below are not guarded).
2. If `d.GetFlags()` (slot 36) has **Active** (`0x2`):
   - if `d.GetLastAmount()` (slot 12) ≠ 0: `d.SetAmount(d.GetLastAmount())` (slot 11), then
     `d.SetIncAmount(d.GetLastIncAmount())` (slots 17, 14);
   - else: `d.Cancel()` (slot 34), `d.SetLastAmount(0)` (slot 13), `d.SetLastIncAmount(0)` (slot 15).
3. Then, reading the flags **again**: if **Cancelled** (`0x4`) is set and `d.GetLastAmount() ≠ 0`: `d.Activate()`
   (slot 35; its result is ignored), `d.SetAmount(d.GetLastAmount())`, `d.SetIncAmount(d.GetLastIncAmount())`.
4. Release `d`.

The dialog uses a deal's lastAmount / lastIncAmount as the **edited** amount: the combo-box handler writes them, and
this routine commits them. Order matters: an active deal edited to 0 is cancelled in step 2 and its lastAmount is
cleared, so step 3 does not revive it. `Activate` succeeds only for a deal that still has Proposed and no failure
flag (neighbors.md §5.4); for a plain cancelled deal it returns false, and `SetAmount` still runs (and is itself
ignored under Grace/Probation).

Worked cases: Active, last 50 → amount 50, inc := lastInc. Active, last 0 → Cancel (Cancelled set, Active cleared,
amount 0), last := 0, lastInc := 0. Proposed+Cancelled, last 30 → Activate succeeds (Active, not Proposed, not
Cancelled), amount 30. Proposed only (not Cancelled, not Active) → untouched here; `DestroySubBudgetWindow` then
cancels it (§6, bit 0 → slot 34).

### 5.5 `SetOccupantFundingLevel` (`0x0077BC90`) *[compat]* (BW3, BW8)

`bool SetOccupantFundingLevel(cIGZUnknown* occupant, bool which, f32 level)` (thiscall, `RET 0xC`). `level` is in
percent. Steps: QI `occupant` for the occupant IID `0x26BB767E` into a holder; failure → return false. Branch on the
dialog's **service kind** `+0x134`, which `initIndividualBudget` sets from the department ID `+0x24`
(`0x00786815`–`0x00786A16`; department IDs per budget.md):

| Department `+0x24` | Kind `+0x134` | `which = 0` | `which = 1` |
|---|---|---|---|
| `0x09188F42` health, `0xAA538CB3` | `0xCA565486` hospital | staff funding | ambulance funding |
| `0x09188F4C` education, `0x4A538CC6` schools | `0xEA5654B6` school | staff funding | bus funding |
| `0xA2963983` police, `0xA2963984` jail | `0x0A567BAA` police | station funding | return false |
| `0x28F55A9F` fire | `0xEA567BC3` fire | station funding | return false |
| `0x8910BC8A` power | `0xCA58E540` power | plant funding | return false |
| any other | 0 | return false | return false |

- **Hospital / school**: `q := ResidentialSimulator` (global `0x00B43DA4`) `.GetHospitalQueryData(occ, &rec)` (slot 27
  `+0x6C`) or `.GetSchoolQueryData` (slot 26 `+0x68`); false → return false. Overwrite one f32 of the record with
  `level × 0.01f` rounded once to f32: `which = 0` the staff funding (`rec+0x2C`), `which = 1` the vehicle funding
  (`rec+0x34`); the other keeps the value just read. Then `SetOccupantFundingPercentages(occ, &rec+0x2C, &rec+0x34,
  isSchool)` (slot 19 `+0x4C`; isSchool 0 for hospital, 1 for school). Return true. The record layout is
  residential-education-health.md (query data).
- **Police**: `PoliceSimulator` (`0x00B43D98`) `.SetStationFunding(occ, &p)` (slot 8 `+0x20`), `p = level × 0.01f`.
- **Fire**: `FireProtectionSimulator` (`0x00B43D78`) `.SetStationFunding(occ, &p)` (slot 9 `+0x24`).
- **Power**: `PowerSimulator` (`0x00B43DA0`) `.SetPowerPlantFundingPercentage(occ, &p)` (slot 51 `+0xCC`).

The setter's own result is ignored; the routine returns true after the call and releases the holder on every path.
`which ≠ 0` for police, fire or power returns false **before** any call. Funding level 0 is therefore "staff / station /
plant" and level 1 "vehicles" (bus, ambulance). Example: school, which 1, level 75.0 → bus funding `0.75f`
(`0x3F400000`), staff funding unchanged.

### 5.6 Ordinance rows and the info popup (child `+0x14`) *[compat]* (BW9)

The ordinance dialog keeps a row map `+0x74` (u32 per button, indexed by `id − 300` or `id − 500`) giving the index
`k` into the byte pairs `+0xA8` (`[2k]` original, `[2k+1]` edited) and the ordinance IDs `+0xA4`.

- **Check box `300 + r`** (`0x0078B600`–`0x0078B663`): `k := map[r]`; the child window `id` (slot 35) is asked for its
  check-box interface `0x8810` (slot 0); if it answers, `edited[k] :=` its state (slot 10 `+0x28`, low byte) and the
  interface is released; then **always** `updateOrdinanceBudget(win)`. Return 0.
- **Info `500 + r`** (`0x0078B67E`–`0x0078BB69`): close any open child first; `k := map[r]`; build a full-dialog
  generator window (class `0x4386D516`), store it at `+0x14`, ID `0x0423278D`, add it to `win`, then a second
  generator window (ID `0x0423278F`, background image instance `0x144161E4`); the popup content is two texts
  (`add_text` IDs `0x0ABCE000`, `0x0ABCE001`, from ordinance `k`) and two image buttons, `0x384` (image
  `0x14416242`) and `0x68` (image `0x140155B4`). Both buttons close the child (§5 table). Return 0. (Pixel layout
  is out of scope.)

`+0x14` is also set by `SetEmptyWindow` (`0x0077BEC0`, the empty-group message window of §4.1). Buttons `0x68` and
`0x384` call `win.RemoveChild(+0x14)` **without** a null check (`0x0078B287`–`0x0078B291`) and only then test the
pointer; the other "close the child" paths test first.

### 5.7 Camera jump (`0x0ABCDE00`–`0x0ABCDEFF`, individual mode) *[compat]* (BW3)

`item := get_window_indiv_item(win, id)`; null → return 0. QI `item` for the occupant IID `0x26BB767E`; if it answers:
`v := (z, z, z)` where `z` is the f32 global `0x00B4DFCC` (in uninitialised data, so 0.0 unless written), then
`occupant.GetPosition(&v)` (slot 8 `+0x20`), then the camera control of the renderer (`0x00B43DD0` slot 16 `+0x40`,
no null check) jumps to `v` (`0x007CD810`, the routine of the Lua `camera_jump`, advisors.md). Release the holder.
Return 0, no refresh. The dialog stays open.

## 6. Closing (`end_dialog` `0x00779850`, `DestroySubBudgetWindow` `0x0077A1A0`) *[compat]*

`end_dialog(win)`: null → false. If the window has flag `0x1000` (modal), ask its parent to end the modal loop
(parent `+0x24` with code `0xF` and the window twice); otherwise remove it from its parent (`+0x38`). Returns true.

`DestroySubBudgetWindow`: if mode 2, for each held neighbour deal (vector `+0x68..+0x6C`, 8 bytes each): AddRef; if
the deal's flags (slot 36) have bit 0 set, call deal slot 34; Release. Then `NeighborsSimulator.SetUIDisableFlag(false)`
and `MarkDealsDirty()`. In every mode: `Simulator.HiddenResume()`; then `end_dialog` on the held window if any.

Order matters *[compat]*: the pause ends **before** the window is ended, and the panel refreshes its balance sheet
only after both (§2 step 5).

## 7. Budget panel message hooks (`cSC4BudgetUIWinProc::DoMessage`, `0x00768620`) *[compat]*

Messages `0x08DAC3E7` and `0x08DAC3E8` set a dirty byte (`+0x192`); message `0x86AD10EE` (a frame/update tick), when
the byte is set, clears it and runs `UpdateBalanceSheetInfo`. So the panel refreshes at most once per tick however
many money messages arrive. `ShowWindow` (`0x00768670`) refreshes immediately and sets `+0x19D := 1`.

## 8. Open work (filed as questions BW1–BW5, docs/specs/questions/budget-window.md)

Row layout of the four populate routines; the special-case groups in §4; `FUN_0077AB40` (button `0xCF`);
the `0x0ABCDExx` occupant controls; the meaning of panel slot 6 (`+0x18`) toggled by `+0x18C`.

## 9. Conformance tests

1. Opening any selector while a sub-budget dialog is held does nothing.
2. While a dialog is open `IsHiddenPaused()` is true; after OK or Cancel it is false and the panel balance sheet was
   refreshed once.
3. Group mode, move one slider to 30, press Cancel: every department in the group is back to its starting fraction.
4. Ordinances: toggle two rows, press `0xCD`: exactly two `SetOrdinanceOn` calls; toggle and press `0x6D`: none.
5. Neighbour mode: `uiDisabled` is 1 while open, 0 after close, and deals are marked dirty on close.
6. Esc closes; Enter without modifier bit `0x4` does not.
