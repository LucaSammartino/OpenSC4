# The budget income and expense dialogs (behaviour spec)

Scope: the two window procedures the budget simulator's `ShowBudgetWindow` (slot 24, `0x00690330`) opens:
`cSC4BudgetUIIncomeWinProc` (tax rates and income) and `cSC4BudgetUIExpenseWinProc` (department funding and
expenses). They are separate from the budget panel of budget-window.md and budget-window-bodies.md. The budget
simulator is budget.md; its interface slots are named in the SDK order (budget.md, docs/abi/interfaces.tsv).

Evidence: static RE only, on the instruction and data bytes of the Steam 1.1.641 exe (the same code as GOG,
`docs/lanes/re-dates.md`). Every function was read in the disassembly; the row table and constants were read in the
PE. Written 2026-10-25 (re-budgetdlg8).

Notation. "Money text `id` := v" is the helper (`0x00764150` income, `0x007639D0` expense; int64 v, pop 12): the
script window's `cIGZWin` (slot 3), its child `id` (slot 34 `GetChildWindowFromID`), the child's text interface
(QI `0x212CDC1F`) gets `SC4MakeMoneyString(v)` through its slot 4; a missing window, child or interface writes
nothing. Integer arithmetic is signed 64-bit (`__allmul`, `__alldiv`, truncating). B is the held budget simulator.

---

## 1. Opening and closing *[compat]*

`ShowBudgetWindow`: when the budget's panel flag (`+0x850`, set by message `0x29222F02`, budget.md §5) is 1, create
the expense dialog (`0x28` bytes, `0x007638E0`), else the income dialog (`0x60` bytes, `0x00764030`); AddRef it and
tail-call its Release (so the dialog lives through the references the script window holds).

Both constructors build the window from the UI script (type 0, group `0x96A006B0`, instance `0xE91A0FA3` income,
`0x691A0FB4` expense; no parent, id `0x10000000`, IID `0x5386D516`) into the script-window member and, on success,
install the window-procedure sub-object (`+0x08`) through the script window's slot 7.

**Close** (`0x00763FD0` income, `0x00763880` expense): AddRef itself; destroy the window (the window's slot 6 gives the
window manager, its slot 23 `DestroyWindow(window)`); release and clear the budget and the script window; tail-call
its own Release.

| | Income (`0x60` bytes) | Expense (`0x28` bytes) |
|---|---|---|
| vtables | `0x00AB49F4`; window proc `+0x08` `0x00AB49E0`; `+0x0C` `0x00AB49D0` | `0x00AB49B0`; `+0x08` `0x00AB499C`; `+0x0C` `0x00AB498C` |
| script window | `+0x14` | `+0x10` |
| budget B | `+0x18` | `+0x14` |
| other | `+0x10` byte 1 (unused here); `+0x20…+0x5C` the amounts below | `+0x18` byte 1 (unused here); `+0x20` the ordinance expense (int64) |
| destructor | `0x00764100`: release `+0x18`, `+0x14` | `0x00763980`: release `+0x14`, `+0x10` |

## 2. The income dialog *[compat]*

### 2.1 State and rows

`+0x20` the ordinance income (int64), `+0x28` it × 12; per tax row a monthly and a yearly amount (int64):

| Tax type t | Slider id (IID `0x612CE0C3`) | Monthly text, store | Yearly text, store |
|---|---|---|---|
| 1 | `0x501` | `0x15`, `+0x30` | `0x16`, `+0x38` |
| 4 | `0x502` | `0x17`, `+0x40` | `0x18`, `+0x48` |
| 10 | `0x503` | `0x19`, `+0x50` | `0x20`, `+0x58` |

### 2.2 Messages (`DoWinProcMessage`, `0x00764700`, on `+0x08`)

Only messages for the script window count; the result is false except where noted.

- `0xA2BF8AD5` (the window is set up): disable child `0x102` (slot 68 `SetFlag(2, false)`); B := the city's budget
  (city slot 52, held); **Refresh** (§2.3).
- `0x682876C9` (a slider moved) with data1 in `0x501…0x503`: **SliderMoved(data1)** (§2.4).
- A command (type 3) with data1 `0x287259F6` (a button) and data2: `0x101` posts `0x29222F02` with data1
  `0x892230F2` through the message server `0x00B43CCC` (slot 3), so the expense dialog opens next, then Close, true;
  `0x107` Close, true; `0x102` nothing.

### 2.3 Refresh (`0x00764600`)

1. Zero `+0x20…+0x5C`. **RefreshRow** for t = 1, 4, 10 (`0x007644B0`): the slider := `ftol(GetTaxRate(t))` (B slot 36,
   slider slot 9); with `w := GetWeightedAssessedTaxValue(t)` (slot 38, int64, converted to float), monthly text :=
   `ftol(w · rate · 0.01)` and yearly text := `ftol(w · rate · 0.12)` (`0x00A823A4`, `0x00AB4A0C`); then **UpdateTotals**.
   RefreshRow writes only the texts, not the stores of §2.1.
2. With B: money text `0x38` := `GetTotalFunds()`, `0x34` := `GetTotalMonthlyExpense()`, `0x35` :=
   `GetTotalYearlyExpense()` (slots 6, 14, 15); without B, all three 0.
3. With the ordinance simulator `0x00B43D90`: `+0x20` := its `GetOrdinanceMonthlyIncome()` (slot 23), `+0x28` :=
   `+0x20 × 12`. Money texts `0x21` := `+0x20`, `0x22` := `+0x28`. UpdateTotals.

### 2.4 SliderMoved(id) (`0x00764300`)

The slider's value v (QI `0x612CE0C3`, slot 8, unsigned) for the row of id (another id: nothing): `SetTaxRate(t, v)`
(slot 37, v as a float); `w := GetWeightedAssessedTaxValue(t)`; monthly := `w · v / 12 / 100`, yearly := `w · v / 100`
(64-bit integer steps); store both, show both, UpdateTotals. So a row's monthly figure after a slider move is a
twelfth of the one Refresh showed for the same rate *[quirk]*.

### 2.5 UpdateTotals (`0x007641F0`)

`M := +0x20 + +0x30 + +0x40 + +0x50`, `Y := 12·M`. With B: `SetTotalMonthlyIncome(M)`, `SetTotalYearlyIncome(Y)`
(slots 22, 23). Money texts: `0x32` := M, `0x33` := Y, `0x36` := `GetTotalMonthlyIncome() − GetTotalMonthlyExpense()`,
`0x37` := `GetTotalYearlyIncome() − GetTotalYearlyExpense()` and `0x39` := `GetTotalFunds() + (the 0x37 value)`
(slots 16, 14, 17, 15, 6; each 0 without B). Because Refresh leaves the row stores at zero, opening the dialog sets
the budget's monthly income to the ordinance income alone until a slider moves *[quirk]*.

## 3. The expense dialog *[compat]*

### 3.1 The row table

Seven process-global rows of 40 bytes at `0x00B0D8B8` (shared by every expense dialog):

| Department | Slider (IID `0x21325207`) | Monthly text | Yearly text |
|---|---|---|---|
| `0x09188F4C` education | `0x301` | `0x215` | `0x216` |
| `0x09188F42` health | `0x302` | `0x217` | `0x218` |
| `0xA2963983` police | `0x303` | `0x219` | `0x220` |
| `0x28F55A9F` fire | `0x304` | `0x221` | `0x222` |
| `0x8910BC8A` power | `0x305` | `0x223` | `0x224` |
| `0xC921CEEA` tolls | `0x306` | `0x225` | `0x226` |
| `0xC921CEEB` transit | `0x307` | `0x227` | `0x228` |

Row fields: `+0x00` department, `+0x04` slider, `+0x08`/`+0x0C` text ids, `+0x10` the funding percent, `+0x18` the
monthly cost (int64), `+0x20` byte changed.

### 3.2 Messages (`DoWinProcMessage`, `0x00763EE0`, on `+0x08`)

- `0xA2BF8AD5`: **Init** (§3.3).
- Command `0x887113A3` with data2 = a control id: **Changed(id)** (§3.4).
- Button `0x287259F6`: `0x102` posts `0x29222F02` with data1 `0xC92230EE` (the income dialog next), **Apply**, Close,
  true; `0x111` Apply, Close, true; `0x101` nothing.

### 3.3 Init (`0x00763D50`)

1. B := the city's budget (held). `+0x20` := the ordinance simulator's `GetOrdinanceMonthlyExpense()` (slot 22; the
   global is not null-tested here *[quirk]*).
2. For each row: `d := B.GetDepartmentBudget(dept)` (slot 28; not null-tested *[quirk]*); `p := *d.GetFundingPercentage()`
   (slot 10); changed := 0; percent := `ftol(p · 100)`; monthly := `ftol(GetIdealMonthlyFunding() · p)` (slot 14, the
   int64 as a float); **RefreshRow** (`0x00763CB0`): the slider (slot 5) := percent, money texts monthly and
   `monthly × 12`, UpdateTotals.
3. Money texts `0x246` := `GetTotalFunds()`, `0x240` := `GetTotalMonthlyIncome()`, `0x241` := `GetTotalYearlyIncome()`
   (0 without B); `0x229` := `+0x20`, `0x230` := `12 × +0x20`. UpdateTotals. Disable button `0x101`.

### 3.4 Changed(id) (`0x00763B90`)

The control's slider interface (QI `0x21325207`; none: nothing) gives v (slot 4). The row whose slider is id (none:
nothing): when the department exists, changed := 1, percent := v, monthly := `GetIdealMonthlyFunding() · v / 100`;
money texts monthly and `monthly × 12`; UpdateTotals.

### 3.5 UpdateTotals (`0x00763A70`) and Apply (`0x007637F0`)

**UpdateTotals**: `M := Σ rows' monthly + +0x20`, `Y := 12·M`; money texts `0x242` := M, `0x243` := Y, `0x244` :=
`GetTotalMonthlyIncome() − M`, `0x245` := `GetTotalYearlyIncome() − Y`, `0x247` := `GetTotalFunds() + (the 0x245
value)` (0 without B). It does not write the budget.

**Apply**: for each changed row whose department exists, `SetFundingPercentage(&q, 0)` (slot 11) with
`q := percent · 0.01` as a float. Then `S := Σ rows' monthly + +0x20`; with B, `SetTotalMonthlyExpense(S)` and
`SetTotalYearlyExpense(12·S)` (slots 20, 21).

## 4. Quirks

1. Refresh and a slider move compute the income rows on different scales (§2.4), and opening the income dialog sets
   the monthly income to the ordinance income alone (§2.5).
2. The expense dialog's row table is process-global; two dialogs would share it.
3. The expense Init dereferences the ordinance simulator and each department without a null test.
4. The yearly text of income row 10 is `0x20`, not `0x1A`.

## 5. Conformance tests

1. Income, weighted value 120,000 at rate 9: Refresh shows 10,800 monthly and 129,600 yearly; moving the slider to 9
   shows 900 monthly and 10,800 yearly.
2. Income, ordinance income 50 and no slider moved: `0x32` = 50, `0x33` = 600, and the budget's monthly income
   becomes 50.
3. Expense, ideal funding 1,000 at 80 %: percent 80, monthly 800, yearly 9,600; moving to 50 gives 500 and 6,000;
   OK sets the department to 0.5 and the budget's monthly expense to Σ + ordinance expense.
4. Expense button `0x102` posts `0x29222F02` with `0xC92230EE`; income button `0x101` posts it with `0x892230F2`.
