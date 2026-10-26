# Spec: the budget subsystem (`cISC4BudgetSimulator`, `cISC4DepartmentBudget`, `cISC4LineItem`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program. Implementers must work from this spec,
the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled or
disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined.
Where this spec says "the original returns no meaningful value", the original's method has no return statement
for a `bool` the SDK declares; callers in the shipped game ignore it.

Related specs: [simulator.md](simulator.md) (agent scheduling, calendar messages).

## 1. Role

The budget simulator owns the city treasury. It:

1. holds the **funds** and lets other subsystems deposit and withdraw money;
2. once a **month**, computes property-tax income, collects the ordinance, neighbour-deal and department income,
   pays ordinance, neighbour-deal, department and bond expenses, and books the result into the funds;
3. keeps **departments** (one per city service) made of **line items**, whose expenses scale with a
   **funding percentage**; for locally funded departments the percentage is kept per building;
4. issues and repays **bonds** (loans);
5. ends the game when the funds fall to the bankruptcy limit;
6. persists all of this in the city save file.

Three GZCOM classes take part:

| Class | CLSID | Interfaces (IID) | Owner |
|---|---|---|---|
| budget simulator | `0xE990BE01` | `cISC4BudgetSimulator` (`0x6772BD23` *[inferred]*), `cIGZMessageTarget2` (`0x452294AA`), `cIGZSerializable` (`0xE4FDA3D4`), `cIGZUnknown` (`1`) | the city |
| department budget | `0xE990BFFC` | `cISC4DepartmentBudget` (`0x223E2B67` *[inferred]*; the CLSID is also accepted as an IID), `cIGZSerializable`, `cIGZUnknown` | the budget simulator |
| line item | `0xAA313C9F` | `cISC4LineItem` (the CLSID doubles as the IID *[inferred]*), `cIGZSerializable`, `cIGZUnknown` | its department |

The city creates the budget simulator with `GZCOM()->GetClassObject(0xE990BE01, …)` and hands it out through
`cISC4City::GetBudgetSimulator()`. Other subsystems only use the interfaces listed here.

**Reference counting** *[compat]*: methods that return a single object (`CreateDepartmentBudget`,
`GetDepartmentBudget`, `CreateLineItem`, `CreateLineItemForBuildingType`, `GetLineItem`,
`GetLineItemFromBudgetItem`) return a **borrowed** pointer (no `AddRef`). Methods that fill a container
(`GetDepartmentBudgetsInGroup`, `GetAllLineItems`, `GetLocallyFundedItemsByPurpose`) **add one reference per
element**; the caller releases them.


## Verified slot order of `cISC4BudgetSimulator` overloads *[compat]* (see foundations §16)

| Slot | vtable offset | Method (as the game expects it) |
|---|---|---|
| 18 | `+0x48` | `int64 GetTaxIncome(int32 taxType)` |
| 19 | `+0x4C` | `int64 GetTaxIncome()` |
| 27 | `+0x6C` | `cISC4DepartmentBudget* GetDepartmentBudget(cIGZString const& name)` |
| 28 | `+0x70` | `cISC4DepartmentBudget* GetDepartmentBudget(uint32 departmentID)` |

This is the compiled layout; MSVC produces it from the SDK header's source order (foundations §16). A mismatch at slots 18/19 would crash the game as soon as
a city loads, because the per-type tax query is called continuously. All other slots (0–58) match the SDK
declaration order; slot 32/33 (`NeededFundingChanged`/`FundingPercentageChanged`) and 34/35 (`GetFunding`
overloads) share bodies in the original, and their SDK order is accepted as is. `cISC4DepartmentBudget` and
`cISC4LineItem` have no overloads.

### Erratum: `cISC4DepartmentBudget::GetLocalFundingPercent` returns a pointer *[compat]*

Slot 26 (`+0x68`), `GetLocalFundingPercent(holder, purpose)`, returns **`SC4Percentage const*`** (a pointer to
a 32-bit float) in EAX. The SDK header says it returns `float`. That is wrong. Game callers (for example the
vehicle manager's funds-changed handler, which runs every month and after any funding change) dereference
the returned value right away. A float returned in ST0 leaves garbage in EAX, and the game crashes with an
access violation (observed: a read of address `0x3F800000`, the bit pattern of 1.0f).

- Found (registered and the budget item exists): point to storage holding the item's local-funding fraction.
  Callers read it immediately and never keep it. The original points to a temporary that is only valid until
  the next call. The replacement must return a pointer that stays valid at least until the next call on the
  same object, for example to a member that is overwritten on each call *[choice]*.
- Not found: point to a constant 0.0f that is never written.

The other float-returning methods we implement are real floats in ST0: `cISC4BudgetSimulator::GetTaxRate`
(slot 36) and `cISC4Simulator::GetAnimationTimeDilation` (slot 32). `GetLocalFullFunding` (slot 27) returns
int64 in EDX:EAX, as declared.

## 2. Concepts and units

- **Money** is a signed 64-bit integer number of simoleons. Funds may be negative.
- A **percentage** (`SC4Percentage`) is a 32-bit float fraction: 1.0 = 100 %. Tax rates are floats in
  *percent* (9.0 = 9 %).
- **Difficulty** comes from `cISC4City::GetDifficultyLevel()`: 0 = easy, 1 = medium, 2 = hard. It indexes the
  per-difficulty tuning arrays (§9).
- **Tax groups**: 12 groups, indexed 0..11 *[compat]*:

  | idx | group | idx | group | idx | group |
  |---|---|---|---|---|---|
  | 0 | R$ | 4 | CS$$ | 8 | I-R (agriculture) |
  | 1 | R$$ | 5 | CS$$$ | 9 | I-D (dirty) |
  | 2 | R$$$ | 6 | CO$$ | 10 | I-M (manufacturing) |
  | 3 | CS$ | 7 | CO$$$ | 11 | I-HT (high tech) |

- **Tax type table** *[compat]*: a building's tax group is `TaxType[purpose][lotWealth]`, where `purpose` is the
  building profile's `PurposeType` (SDK `cISC4BuildingOccupant`) and `lotWealth` is the lot's occupant wealth
  (0..3). `-` means untaxed:

  | purpose | w0 | w1 ($) | w2 ($$) | w3 ($$$) |
  |---|---|---|---|---|
  | 0 None | - | - | - | - |
  | 1 Residence | - | 0 | 1 | 2 |
  | 2 Services | - | 3 | 4 | 5 |
  | 3 Office | - | - | 6 | 7 |
  | 4 Tourism | - | 3 | 4 | 5 |
  | 5 Agriculture | - | 8 | 8 | 8 |
  | 6 Processing | - | 9 | 9 | 9 |
  | 7 Manufacturing | - | 10 | 10 | 10 |
  | 8 HighTech | - | 11 | 11 | 11 |

  *[choice]* Purpose 9 (Other) and any purpose above 8 are untaxed. The original has no row for them and reads
  whatever data follows the table.

- **Budget items.** A building's exemplar (or its instance property holder) describes what the building costs or
  earns with five **parallel array properties**, one entry per item *[compat]*:

  | Property | Type | Meaning | Missing entry |
  |---|---|---|---|
  | `0xEA54D283` | uint32[] | department ID | — (defines the item count) |
  | `0xEA54D284` | uint32[] | line item ID (0 = "use the building type", see §12) | 0 |
  | `0xEA54D285` | uint32[] | purpose ID (what the item pays for) | 0 |
  | `0xEA54D286` | sint64[] | monthly cost (or income) at 100 % | 0 |
  | `0xEA54D287` | float[] | local funding in percent (100 = full) | 100 % |

  The item **index** is its position in these arrays. The SDK's `BudgetItem` struct reports
  `{departmentID, lineItem, purpose, cost, index, localFundingPercentage}`, the last as a fraction.
  Purposes seen in the game: `0xAA59670C` building maintenance (civic), `0x4A5A495E` building income (civic),
  `0xCA58E540` power plant.

- **Departments** group the expenses of one service (police, fire, schools, power, …). A department has an ID, a
  display name, a **budget group** ID, a funding percentage `p` with a ceiling `pMax`, a fixed-funding flag, a
  year-to-date spending total, a map of **line items** keyed by line ID, and a set of **locally funded objects**
  (pairs of property holder and purpose).
- **Line items** are rows of a department: typically one per building type. Each has a **full expense** (cost at
  100 %), a **current expense** (what is actually paid this month), an **income**, a type (0 = Expense,
  1 = Income), display flags, a "secondary info" number (the UI shows it, e.g. a building count) and a
  **locally funded** flag.
  - For a *non-local* line, current = full × the department's `p`.
  - For a *local* line, current = Σ over the department's locally funded buildings on that line of
    cost × the building's own local funding.
- **Budget groups** collect departments for the budget window. A group has an ID, a localised name
  (LTEXT group `0x6A3FF01C`, instance = group ID), cached total expense and income, and a set of department IDs.

## 3. State

### 3.1 Budget simulator

| State | Initial value | Persisted (§11) |
|---|---|---|
| initialized | false | no |
| loadedFromSave | false (set by `Read`) | no |
| agricultureRateFix | false (set by `Read` of version 4) | no |
| budgetPanelFlag (which budget panel `ShowBudgetWindow` opens) | 0 | no |
| funds, shadowFunds | 0, 0 | yes, both |
| totalBorrowed | 0 | yes |
| lastTaxIncome | 0 | yes |
| ytdIncome, ytdExpense | 0 | no (*[compat]* see §11.1 note) |
| ytdTaxIncome | 0 | yes |
| ytdReserved[5] (never accumulated, see §13) | 0 | yes |
| ytdOrdinanceIncome, ytdOrdinanceExpense | 0 | yes |
| ordinanceIncomeByMonth[12], ordinanceExpenseByMonth[12] | 0 | yes |
| ytdBondPayments | 0 | yes |
| lastMonthIncome, lastMonthExpense | 0 | yes |
| yearlyIncome, yearlyExpense (only set through the setters) | 0 | yes |
| taxGroup[12] = {rate, maxRate, history[12] (s64), reserved (s64), weightedAssessed (s64), monthlyTax (s64)} | all 0 | yes |
| departments: ordered map ID → department | empty | yes |
| groups: ordered map ID → group | empty | yes |
| bonds: list in issue order | empty | yes |
| bondCount | 0 | yes (as the list length) |
| riskyMoneyCounter | 0 | no |
| tuning values (§9) | code defaults until `Init` | no |

Note: `ytdIncome` and `ytdExpense` are **not** saved. They are reset to 0 when a city is loaded (*[compat]*,
see §11.1).

**Shadow funds** *[compat]*: every legitimate write of `funds` (§6 setters, deposits, withdrawals, resets,
settlement) writes the same value to `shadowFunds`. The monthly settlement first copies `shadowFunds` into
`funds` if they differ (an anti-tampering measure). Both must be stored and saved.

### 3.2 Department

| State | Initial value | Persisted |
|---|---|---|
| initialized | false (`Init` sets true) | yes |
| departmentID | 0 | yes |
| name (string), nameLtextGroup, nameLtextInstance | "", 0, 0 | yes |
| budgetGroup | 0 | yes |
| fundingPct `p` | 1.0 | yes |
| maxFundingPct `pMax` | 1.0 | yes |
| totalSpending (year to date) | 0 | yes |
| fixedFunding | false | yes |
| lineItems: ordered map lineID → line item | empty | yes |
| localObjects: set of (propertyHolder, purpose) | empty | yes |

### 3.3 Line item

| State | Initial value | Persisted |
|---|---|---|
| id | 0 | yes |
| name, nameLtextGroup, nameLtextInstance, nameNeedsResolve | "", 0, 0, false | yes (not the flag) |
| secondaryInfo (s64) | 0 | yes |
| fullExpenses, currentExpenses, income (s64) | 0 | yes |
| displayFlags | 5 (ShowLineItem \| bit 4) | yes |
| isLocal | false | yes |
| type | 0 (Expense) | yes |
| owner (department) | none | yes (as a reference) |

### 3.4 Bond

`{dueMonth (1..12), dueYear, amount, fullCost, remaining, paid, monthlyPayment, principalLeft}`; months and
years are 32-bit, the rest 64-bit. `remaining` and `paid` are bookkeeping only; `principalLeft` drives the
amortisation (§5.4).

## 4. Lifecycle, agents and messages handled

### 4.1 `Init()` → true
Does nothing if already initialized. Otherwise, in this order *[compat]*:

1. Set `initialized`.
2. Read the tuning values (§9).
3. Register itself with the simulator three times, name `"cSC4BudgetSimulator"`, **flags 2** (runs only while
   the city is established): level **0** (one-shot), level **6** (month), level **4** (day).
4. Subscribe on the message server to `0x26D31EC4` CityEstablished, `0x29222F02` ToggleBudgetDialog,
   `0x08DAC3E8` IdealMonthlyFundingChanged and `0x08DAC3E7` FundingPercentageChanged (the last two are ignored
   when they arrive).
5. Acquire the city, the land-value simulator and the ordinance simulator.
6. With the cheat-code manager, register and subscribe `RiskyMoney` (`0xCBF31943`) and `WeaknessPays`
   (`0x4AB87766`); if the application runs in debug mode also `Moolah` (`0x493F4FE6`).
7. If **not** `loadedFromSave` (a new city):
   - set funds and shadow funds to `InitialFunds[difficulty]` and post FundsChanged;
   - for each tax group: `rate := InitialTaxRate[difficulty]`, `maxRate := MaxTaxRate`, clear its 12-entry
     history, reserved value, weighted assessed value and monthly tax;
   - set the history-warehouse queue length of the 12 tax-rate series `0xAABAAA9F + i` to **365**;
   - record `funds / 1000` (integer division toward zero) into history series `0x4A5BA3D0`.
8. If `agricultureRateFix`: set tax group 8's rate to `InitialTaxRate[difficulty]`.
9. Publish the Lua table `sc4game.budget` in the **game-data registry** (§4.4): a node `"budget"` with a
   read/write value `"total_funds"` (read = funds, write = `SetTotalFunds`) and a command `"deposit_funds"`
   (parameter 0 read as a 64-bit integer → `DepositFunds`).

### 4.2 `Shutdown()` → true
Does nothing if not initialized. Otherwise: clear `initialized`; remove the `"budget"` registry node (§4.4); unregister
and unsubscribe all three cheats; release the city; unsubscribe the four notifications and release the message
server; release the land-value and ordinance simulators; shut down and release every department and empty the
department map; remove all of its agent registrations from the simulator. Funds, bonds, groups and tax data are
left as they are.

### 4.3 `DoMessage(msg)`

| Message | Action | Result |
|---|---|---|
| `0x66956810` SimBegin (one-shot agent) | post FundsChanged | true (the agent is finished) |
| `0x66956814` SimNewDay (day agent) | daily simulation (§5.7) | true |
| `0x66956816` SimNewMonth (month agent) | monthly settlement (§5.1) | true |
| `0x26D31EC4` CityEstablished | funds := shadow := `InitialFunds[difficulty]` (posts FundsChanged); every group's rate := `InitialTaxRate[difficulty]` (max rates unchanged) | true |
| `0x29222F02` ToggleBudgetDialog | data1 = `0x892230F2` → panel flag := 1, then `ShowBudgetWindow()`; data1 = `0xC92230EE` → flag := 0, then `ShowBudgetWindow()`; other data1 ignored | true |
| `0x230E27AC` cheat notification, data1 = cheat ID | `WeaknessPays`: `SetTotalFunds(funds + 1000)`. `Moolah <n>`: if an argument is present, `SetTotalFunds(n)` (64-bit parse), else nothing. `RiskyMoney`: see below | true for handled cheats; false for unknown cheat IDs and `Moolah` without argument |
| anything else | ignored | true |

Every agent message is finished in one call (the budget never asks to be called again).

**RiskyMoney** *[compat]*: only if the city is established. `DepositFunds(10000)`; `riskyMoneyCounter += 1`;
draw `u` uniform in [0,1); if `u < 1 − 1/(0.2·counter + 1)`, ask the disaster layer
(`cISC4DisasterLayer::GetDisasterManager`) for the **earthquake** manager (`0x42963812`); if it exists, call its
`AttemptDisasterStart(x, z)` (the two-integer overload, compiled slot 6, foundations §17) with

```
W := city.SizeX();  D := city.SizeZ()                 (world units, metres; 16 per cell)
x := trunc(float(W·0.375 + r1·W·0.25))                (r1, then r2: two further uniform draws)
z := trunc(float(D·0.375 + r2·D·0.25))
```

If the call returns an instance (non-null), reset the counter to 0. All three draws (`u`, `r1`, `r2`, in that
order) come from the budget source file's own `cRZRandomSuper` with `RandomDoubleUniform` (foundations §6.2).
The arguments are **world coordinates**, not cells, although the SDK names them `nCellX`/`nCellZ`: the earthquake
clamps them to `[0, W − 1]` × `[0, D − 1]` and converts to a cell itself with the city's `PositionToCell`. The
result is a point in the middle half of the map. The earthquake manager refuses to start while another
earthquake is active (returns null), so the counter then keeps growing.

### 4.4 The `"budget"` node in the game-data registry

**What the registry is.** `cISC4GameDataRegistry` (public SDK header) is an application **system service**: get
it with `RZGetFramework()->GetSystemService(0xEBE0E860, 0xE898ED03, &p)` (service ID `0xEBE0E860`, interface ID
`0xE898ED03`; the SDK calls the first `GZCLSID_cISC4GameDataRegistry`). It is a tree of named entries that the
Lua scripting layer exposes to scripts as the table `sc4game`: a node becomes a sub-table, a value entry becomes
a getter/setter function, a call entry becomes a function. Several subsystems publish nodes the same way
(automata, aura, civic buildings, ...). Overloads in this interface are compiled in reverse groups; the slots used
here are in foundations §17.

**What the budget publishes** *[compat]* (end of `Init`), giving the Lua functions `sc4game.budget.total_funds()`
and `sc4game.budget.deposit_funds(simoleons)`:

| Step | Call (compiled slot) | Arguments |
|---|---|---|
| node | `AddChildEntry(cIGZUnknown* key, name, parent, flags, context)` (slot 5) | key = the budget simulator as `cIGZUnknown`; name `"budget"`; parent = `GetRootNodeEntry()` (slot 21); flags 0; context null |
| value | `AddChildEntry(void* key, name, exchangeCallback, parent, flags, context)` (slot 4) | key = any address unique to this simulator (the original uses the address of its funds field); name `"total_funds"`; parent = `GetEntryID(budget as cIGZUnknown)` (slot 9); flags 0; context = the budget simulator |
| command | `AddChildEntry(void* key, name, callCallback, parent, flags, context)` (slot 3) | key = the callback's own address; name `"deposit_funds"`; same parent; flags **1** (`EntryFlagCallFunction`); context = the budget simulator |

Callbacks (each returns true):

- exchange(key, variant, set, registry): `sim := registry.GetEntryContext(registry.GetEntryID(key))` (slots 18 and
  8); if `set`: read the variant as Sint64 (`GetValSint64(int64&)`, a converting read; if it fails `v` stays 0,
  so the funds become 0) → `sim.SetTotalFunds(v)`; else `variant.SetValSint64(sim.GetTotalFunds())`.
- call(key, input, output, registry): `sim` as above; `p := input.GetParameter(0)`; if `p` exists, read it as
  Sint64 → `sim.DepositFunds(v)`. Nothing is written to `output`.

`Shutdown` removes the node with `RemoveChildEntry(cIGZUnknown* key = the budget simulator)` (slot 7); its
children go with it.

**Who reads it.** Only Lua scripts. The shipped scripts **never call** these functions: a script in
`SimCity_1.dat` only declares empty placeholder definitions of the two names (as it does for every registry
function), and no other shipped script (base game or Rush Hour, all `.dat` files searched) refers to them. No C++ code reads the
node. Therefore *[choice]*: omitting the node has **no observable effect with the shipped data**. FreeCity4
should still publish it when the registry service is available, because third-party Lua mods may call
`sc4game.budget.*`; failure to get the service is not an error.

## 5. Money flow

### 5.1 Monthly settlement (on SimNewMonth)

The date has already moved to the 1st of the new month. Let `(year, month)` be the new date and `m` the 0-based
index of the month that just ended (`m = month − 2`, or 11 when `month == 1`).

```
if funds != shadowFunds: funds := shadowFunds                     (no message)
tax      := ComputeTax()                                           (§5.2)
lastTaxIncome := tax;  ytdTaxIncome += tax
ordInc   := ordinance.GetOrdinanceMonthlyIncome()
ordinanceIncomeByMonth[m] := ordInc;  ytdOrdinanceIncome += ordInc
income   := tax + ordInc + neighbors.GetTotalMonthlyNeighborDealIncome()
            + Σ_departments dept.GetTotalIncome()
ordExp   := ordinance.GetOrdinanceMonthlyExpense()
ordinanceExpenseByMonth[m] := ordExp;  ytdOrdinanceExpense += ordExp
expense  := ordExp + neighbors.GetTotalMonthlyNeighborDealExpense()
for each department (ascending ID):
    dept.RecalculateAllLineItemCurrentExpenses()                   (§7.2)
    c := dept.GetTotalExpenses();  dept.totalSpending += c;  expense += c
for each bond (issue order): expense += bond payment               (§5.4)
SetTotalFunds(funds + income − expense)        (posts FundsChanged if the value changed)
ytdIncome += income;  ytdExpense += expense
lastMonthIncome := income;  lastMonthExpense := expense
record history: 0x6A51178B ← H(income), 0xCA5BA3FB ← H(expense), 0x4A5BA3D0 ← low32(funds / 1000)
shadowFunds := funds
if m == 11:  zero ytdTaxIncome, ytdReserved[*], ytdOrdinanceIncome, ytdOrdinanceExpense,
             ytdIncome, ytdExpense, ytdBondPayments, and every department's totalSpending
if funds <= MinAllowableFunds: post 0xAA5CF1E3 EndGameForBudgetFailure   (§5.6)
```

`H(v)` *[compat]*: if `v ≥ 2^32` the value recorded is `0xFFFFFFFF`; otherwise the low 32 bits of `v`
(the history warehouse takes a signed 32-bit value, so `0xFFFFFFFF` reads back as −1).

**Year end: what the YTD getters return on January 1st (BQ2, from the exe).** The order above is the
original's:
1. The month's amounts are added to `ytdIncome` (`+0x48`) and `ytdExpense` (`+0x40`) at `0x00692994`–`0x006929B3`.
2. The three history values are recorded at `0x00692A24`–`0x00692A4B`.
3. **Only then** does the `m == 11` test (`0x00692A4E`) zero the year-to-date fields, at `0x00692A65`–`0x00692AB9`.

So the reset comes **after** the month is added. December's income and expense, booked by the settlement on
January 1st, are added and wiped in the same call. They never appear in `GetYTDIncome()` (slot 10, `0x006AAF00`)
or `GetYTDExpenses()` (slot 12, `0x006AAEF0`):
- just before that settlement, both getters hold the sum of the eleven settlements from February 1st to
  December 1st;
- just after it, both return **0**.

The January 1st amounts can be observed in two ways:
- **History**: series `0x6A51178B` (income) and `0xCA5BA3FB` (expense). The same settlement writes them before the
  reset, as `H(income)` and `H(expense)`. This works for every month, so a shadow can compare all twelve.
- **Funds**: the change of `GetTotalFunds()` across the settlement is exactly `income − expense` (net only),
  provided nothing else moves funds between the two reads.

They **cannot** be observed through:
- `GetTotalMonthlyIncome`/`GetTotalMonthlyExpense`, which are live projections (§6);
- the stored `lastMonthIncome` (`+0x830`) and `lastMonthExpense` (`+0x840`). These have setters (§6) but no
  getter, and the only other code that reads them is `Write`.

Example: before the settlement on 2002-01-01, `GetYTDIncome() = 118,000` and `GetYTDExpenses() = 104,500`. The
settlement books income 10,700 and expense 9,900. Afterwards:
- both getters return 0;
- the history's newest points are 10,700 (`0x6A51178B`) and 9,900 (`0xCA5BA3FB`);
- funds rose by 800.

A YTD difference across that settlement would give `0 − 118,000`, which is meaningless.

The settlement is the **only** place where funds can become negative through normal play.

### 5.2 Property tax (`ComputeTax`)

Monthly only. First zero every group's `weightedAssessed`. Then for every lot of the city, **each exactly once**.
The original walks the lot manager's master lot list (`cISC4LotManager` compiled slot 15, `GetSafeLotIterator`,
opaque in the SDK), in its list order (corrected, zones.md): a new lot is inserted at the traffic simulator's
`GetDesiredLotInsertionPoint` (null in practice, so appended), and after a load the list is rebuilt in first-cell
order (x outer, z inner). It does **not**
query a cell rectangle. A reimplementation through public interfaces may use
`GetLotsInCellRect(list, 0, 0, CellCountX − 1, CellCountZ − 1, false)` (see Errata round 2, item 5): same set of
lots, different order; the order only affects float rounding of the sums *[choice]*.

```
building := lot.GetBuilding();  skip if none or if not lot.IsInLivableState()
(purpose, bWealth) := building profile;  lWealth := lot.GetOccupantWealth()
t := TaxType[purpose][lWealth];  skip if untaxed
value := building property 0xE91A0B5F (sint64, 0 if absent)
lv    := land value (0..255) at the lot's origin cell
land  := roundHalfEven(LVCurve(lv) × sizeX × sizeZ)                 (single precision)
assessed := trunc(float(land + value) × WaningWealth[bWealth − lWealth] × OccWeight[t])
weightedAssessed[t] += assessed
lotTax := float(assessed) × DifficultyTaxMultiplier[difficulty] × rate[t] × INV1200
groupTax[t] += lotTax;  total += lotTax
```

Afterwards `monthlyTax[t] := trunc(groupTax[t])` and the function returns `trunc(total)`.

- `INV1200` is the `float32` constant nearest to 1/1200 (`0.00083333335`); it converts a yearly percentage to a
  monthly fraction. The original multiplies by it; it never divides by 1200.
- *[compat]* **Operation order** (corrected, Errata round 3, item 2): every product above is evaluated **left to
  right, one `float32` rounding per multiplication** (foundations §18.3). `land + value` and `assessed` are 64-bit
  integers that enter their first multiplication **exactly** (not rounded to `float32` first; foundations §18.3
  point 1). The order matters: `(assessed × multiplier) × rate` and `(rate × multiplier) × assessed` differ in the
  last bit often enough to move a truncated result by 1 (§15 test 2 has a case). The running sums `groupTax[t]` and
  `total` are `float32` sums too: they are stored as doubles, but every addition rounds to 24 bits (BQ3 below).
  Totals then match the original to the simoleon for the same lots in the same order; tests allow ±1 per 1000 lots
  only for the lot-order freedom above (§15).
- `rate[t]` is the group's current rate (the original reads it through its own `GetTaxRate(t)`).
- `value` is read with a converting 64-bit integer read: a scalar of any numeric type converts (a float is
  truncated). All shipped exemplars store `0xE91A0B5F` as a scalar sint64. *[choice]* An array value uses its first
  element (the original's result for arrays was not verified). The holder is the building occupant's own property
  holder (`cISC4Occupant::AsPropertyHolder`, the same object that `QueryInterface(0x25216283)` returns), so instance
  overrides win over the exemplar.
- *[choice]* If `bWealth − lWealth` is outside 0..2, use multiplier 1.0 (the original indexes out of range).
- With the shipped data `LVCurve` is flat zero and every weight is 1, so tax = `rate% × buildingValue / 12 ×
  {1.2, 1.1, 1.0}` per difficulty. Example: a building worth 10,000 at 9 % on hard pays 75 per month.

**The lot set and its order (BQ1, from the exe).** `ComputeTax` (`0x00691E20`) works as follows:
- It gets a safe iterator from the lot manager (slot 15, call at `0x00691E99`) and walks the master list node by node
  until it returns to the head. At the end it unregisters and frees the iterator (`0x0069207C`–`0x006920AF`). The
  unregistering is `std::list::remove` (`0x00809580`) inlined: when the iterator block's owner (block `+4`) is not
  null, every node of the owner's iterator list (owner `+4`) whose value is the iterator is unlinked and freed with
  allocator B (`0x0090CF63`), and the scan runs to the end, so duplicates go too. The 8-byte block is then freed with
  allocator C (`0x005E5620`) **in every case**, also when no node matched or the owner is null (BQ4).
- For each lot it applies, in this order, the only three filters it has:
  - no building (lot slot 48, `+0xC0`, returns null);
  - not livable (lot slot 37, `+0x94`, `IsInLivableState`, returns false);
  - an untaxed purpose/wealth cell (−1 in the tax-type table).
- It has no bounds test, no other state test and no highlight check.
- It takes no references (BQ5). Each lot is the list node's value, used as it is, and lot slot 48
  (`cSC4Lot::GetBuilding`, `0x006BFA30`) releases its own `QueryInterface` reference before it returns the building.
  The loop (`0x00691EB0`–`0x00692077`) calls no `AddRef` or `Release` on either.
- It reaches the list only through slot 15 (BQ6). The original's slot 15 (`0x00734850`) registers the iterator on its
  member at `+0x5C`, which always exists, so it never fails. `ComputeTax` reads no lot manager field itself. It tests
  the block's owner (`0x00691EA1`, `0x00691EB3`): with a null owner it visits no lot and still frees the block. A null
  block would fault (`0x00691E9E`).

The master list and the cell → lot map that `GetLotsInCellRect` scans hold **the same lots** (zones.md §8):
- **Created and deleted together.** `CreateLot` writes every cell of the lot and adds it to the list in the same
  call (zones.md §8.5). `DeleteLot` clears every cell and removes it from the list (§8.6). A load rebuilds the list from the
  cell map (§10.2). Within the lot manager's slots, the only stores into the cell rows are in `CreateLot`
  (`0x0072E617`) and `DeleteLot` (`0x0072FA56`).
- **Mid-transition lots** (under construction, abandoned, burning, waiting for demolition) stay in both until
  `DeleteLot`. The three per-lot filters above decide whether they are taxed, so the same filters applied to either
  set select the same lots.
- **Preview lots are in neither set.** In highlight mode, `CreateLotIfPossible` and `DeleteLot` only record
  virtual lots and virtual deletions in separate maps (zones.md §8.5, §8.6).
- **No lot lies outside the city.** `CreateLot` writes its rectangle without clipping. Every creation path
  specified so far goes through `CreateLotIfPossible`, which needs code 8 on every cell, and a cell outside the
  city gives code 0 (zones.md §8.4).

The two sets can differ only in two edge cases. Neither is known to be reached by the shipped game.
- `CreateLot` (slot 23) has no check of its own. Called directly over an existing lot, it overwrites that lot's
  cells while the old lot keeps its list entry. An old lot that loses every cell is taxed by the original but
  missed by `GetLotsInCellRect`. A partly overwritten lot can be listed twice by `GetLotsInCellRect`. Direct
  callers of slot 23 were not enumerated, but every documented creation path checks for code 8 first.
- With a non-null insertion point whose list is null (zones.md §8.5 step 4 quirk), the lot is in the cells but
  not in the list. The traffic simulator never returns such a point.

**Order.** After a load, the list is in first-cell order, x outer and z inner. That is exactly the order of
`GetLotsInCellRect(list, 0, 0, CellCountX − 1, CellCountZ − 1, false)`, because both meet each rectangular lot
first at its minimum (x, z) cell, scanning x outer. So on a city with no lots created since the load, the shadow
walks the same lots **in the same order**, and every `float32` sum is bit-identical. Lots created later are appended
to the list in creation order (zones.md §8.5) but sit by position in the rect query. Deletions do not reorder the rest. Only
lots created since the load use the order freedom of §15 (the ±1 per 1000 lots tolerance).

**The total is not the sum of the lines (BQ3, from the exe).** `ComputeTax` keeps two kinds of running sum:
- `total`, one double on the stack, starting at 0.0 (`0x00A80990`);
- `groupTax[0..11]`, a vector of twelve doubles, all 0.0.

For each taxed lot, `lotTax` is the product chain above, left in ST0. It is added **first to `total`**, then to its
group (`0x0069205C`–`0x00692070`). Each `FADD` rounds to 24 bits on the main thread (foundations §18), and the result
is stored as a double, which is exact. So both sums hold exactly the values of `float32` running sums. At the end
(`0x006920B4`–`0x006920EB`):
- the return value is `_ftol(total)`, truncated toward zero to 64 bits;
- then, for t = 0..11, `monthlyTax[t] := _ftol(groupTax[t])`.

Both are kept at **64 bits** (BQ7). `_ftol` (`0x009EEF04`) returns EDX:EAX; `ComputeTax` returns both halves
(`0x006920FF`–`0x00692101`) and stores both halves of each `monthlyTax[t]` (`0x006920DC`–`0x006920DE`). Nothing
narrows them to 32 bits. Within ±2^63 the result is the exact truncation toward zero; at or beyond it, and for a NaN,
it is `0x8000000000000000` (the x87 integer indefinite, which `_ftol` passes through). Example: a total of 3.0e9
(`0x4F32D05E`) returns 3,000,000,000. Its low 32 bits, read as an int32, would be −1,294,967,296.

The total is never built from the lines or from the group sums. It keeps the fractions that the twelve truncations
drop, so it usually exceeds the sum of the lines by a few simoleons (by less than 12, up to rounding). BQ3's month
shows this: the lines sum to 4835 and the month's income is 4839. The settlement then adds this 64-bit total to
the other income (§5.1).

The total depends on the **lot order**, because its additions round. The lines can match while the total differs.
To match it to the simoleon, walk the lots in the master-list order above and add each lot's tax to `total` before
its group. A rect-query walk, or a total formed from the twelve group sums, matches only within the §15 tolerance.
Between 4096 and 8192 one `float32` step is 2^−11 ≈ 0.00049. So a reordering moves the truncated total only in
the months where the total lies within a few such steps of a whole number.

Worked example (medium, multiplier 1.1; rate 9 %; four lots whose assessed values are 104410 and 362120 in R$, then
258840 and 70630 in CS$). The lot taxes are 861.38251 (`0x4457587B`), 2987.4900 (`0x453AB7D7`), 2135.4299
(`0x450576E1`) and 582.69751 (`0x4411ACA4`). The exact sum is 796000 × 1.1 × 0.09 / 12 = 6567.

| Running `total` after lot | List order 1, 2, 3, 4 | Reverse order 4, 3, 2, 1 |
|---|---|---|
| first | 861.38251 | 582.69751 |
| second | 3848.8726 | 2718.1274 |
| third | 5984.3027 | 5705.6172 |
| fourth | **6567.0** (`0x45CD3800`) | **6566.9995** (`0x45CD37FF`) |
| returned | **6567** | **6566** |

Both orders give the same lines: R$ `trunc(3848.8726)` = 3848 and CS$ `trunc(2718.1274)` = 2718, which sum to
6566. Only the list order's total is the original's.

### 5.3 Department expenses and income

- Line-item **income** is added to `income` through `GetTotalIncome()` (e.g. civic buildings with purpose
  `0x4A5A495E` add their monthly income to an Income-type line on activation, §12).
- Line-item **current expenses** are recomputed (§7.2) and summed.
- **Funding → effectiveness**: the budget only stores each building's local funding fraction (property
  `0xEA54D287` × 0.01f, default 1.0; RG13: a float32 multiplication, not a division, so the round trip through
  `ChangeBudgetItemLocalFunding`, which stores float32(q × 100.0f), is inexact: 0.43 reads back as 0.42999998). Service simulators read it through `GetBudgetItemForPurpose(holder, purpose)`
  and map it through their own response curves (e.g. power-plant efficiency =
  `curve(localFunding × 100) / 100 × condition`). Those curves belong to the service-simulator specs.

### 5.4 Bonds

**Issue** (`IssueBond(amount)`, amount is uint32):

```
fail if amount == 0 or bondCount >= MaxBonds
a := (amount div BondIncrement) × BondIncrement;  if a == 0: a := BondIncrement
fail if a > MaxBondAmount or totalBorrowed + a > MaxBonds × MaxBondAmount
pay  := GetMonthlyPaymentForLoan(a);  full := GetFullCostOfLoan(a)
due  := (month + n mod 12, year + n div 12), carrying one year if the month exceeds 12   (n = BondPaybackMonths)
bondCount += 1; append {due, a, full, remaining = full, paid = 0, pay, principalLeft = a}
post 0xC772FADA BondIssued;  totalBorrowed += a;  DepositFunds(a)          → true
```

*[compat]* The original computes `a` and stores `fullCost` with 32-bit truncation (sign-extended). With the
shipped limits (≤ 200,000) this never matters. Funds are not checked: bonds can be issued while in debt.

**Payment formula** (annuity), `r = BondInterestRate` (fraction), `n = BondPaybackMonths`:

`GetMonthlyPaymentForLoan(L) = trunc( L · (r/12) / (1 − (1 + r/12)^(−n)) )`, with `r/12` taken from the float `r`.
The constants and intermediates are doubles, but the x87 runs at 24-bit precision (foundations §18), so every
operation, including the C runtime `pow`, rounds to a 24-bit significand: the result is only float-accurate.
*(Corrected: the earlier text said "computed in double precision".)* *[compat]* FreeCity4 evaluates the
expression as a sequence of `float32`-rounded operations in this order (q := r × 1/12; L × q; 1 + q; −(n + 0);
pow; 1 − that; divide); with `pow` from a correctly rounded `f64` routine rounded to `f32`, results may differ
from the original by 1 simoleon near integer boundaries *[inferred]*. `GetFullCostOfLoan(L) = GetMonthlyPaymentForLoan(L) × n`.
*[choice]* If `r == 0`, the payment is `L div n` (the original divides 0 by 0).

Shipped data (r = 0.085, n = 120): 10,000 → 123 per month, full cost 14,760.

**Monthly repayment** (inside §5.1, per bond in issue order). Let `(dy, dm)` = `(dueYear − year, dueMonth −
month)`, normalised so `dm` is in 0..11 (borrow one year when `dm < 0`):

```
p := bond.monthlyPayment
ytdBondPayments += p;  bond.remaining −= p;  bond.paid += p;  expense += p
if dy == 0 and dm == 0:                          (due month)
    totalBorrowed −= bond.amount; remove the bond; bondCount −= 1; post 0xCA3FBDCE BondFinished
else:
    interest := trunc(float(r/12) × float(principalLeft))         (single precision)
    principalLeft := principalLeft − (p − interest)
    if dy == 0 and dm == 1: bond.monthlyPayment := principalLeft  (the last payment clears the principal)
```

A bond issued in month M of year Y is paid on the 1st of each following month and removed on the 1st of month M
of year Y + n/12: n payments in total.

### 5.5 Neighbour deals

The budget does not own deals. It books whatever the neighbours simulator reports:
`GetTotalMonthlyNeighborDealIncome()` (deals where this city sells: odd deal types) and
`GetTotalMonthlyNeighborDealExpense()` (this city buys: even types); proposed or failed deals are excluded by
the neighbours simulator. Deal prices are defined in the neighbours spec, including the original curve bug
(§13.2). Whether this month's or last month's deal prices are booked depends on the order of the month agents
(§14, question 2).

### 5.6 Bankruptcy

*[compat]* At **every** month-end on which `funds ≤ MinAllowableFunds` (−100,000 in the shipped data), post
`0xAA5CF1E3` EndGameForBudgetFailure (no data fields). There is no grace period, counter or warning in this
subsystem. The 3D view handles the message: if modal dialogs can be shown it runs the budget-failure
("you're fired") sequence at once, otherwise it queues the message (once) and handles it when dialogs become safe.
The sequence shows a dialog (only if popup dialogs are enabled) and then **closes the city to the region view
without saving**, popups or not (modal-dialogs.md §4.1). Advisor warnings come from elsewhere. Direct purchases
cannot cause bankruptcy because `WithdrawFunds` refuses to overdraw (§6).

### 5.7 Daily simulation (on SimNewDay)

For each tax group i: record `roundHalfEven(rate[i] × 10)` (the product rounded to `float32` first, foundations §18) into history series `0xAABAAA9F + i`. No money
moves on days.

## 6. `cISC4BudgetSimulator` methods (SDK order)

| Method | Behaviour |
|---|---|
| `Init()` / `Shutdown()` | §4.1 / §4.2. |
| `SetTotalFunds(v)` | If `v ≠ funds`: `funds := shadowFunds := v`, post FundsChanged. *[choice]* return true (the original returns no meaningful value). |
| `GetTotalFunds()` | `funds`. |
| `DepositFunds(x)` | `x < 0` → return `WithdrawFunds(−x)`. `x == 0` → true, no change. Otherwise add, **saturating at INT64_MAX** when funds are non-negative; update shadow; post FundsChanged; true. |
| `WithdrawFunds(x)` | `x < 0` → return `DepositFunds(−x)`. `x == 0` → true. If `x > funds` → **false**, nothing changes (so with negative funds every purchase fails). Otherwise subtract, update shadow, post FundsChanged, true. |
| `GetMinAllowableFunds()` | tuning `MinAllowableFunds`. |
| `GetYTDIncome()` | `ytdIncome`. |
| `GetEstIncome()` | `ytdIncome + GetTotalMonthlyIncome() × (13 − currentMonth)`, currentMonth 1..12 from the simulator date. |
| `GetYTDExpenses()` | `ytdExpense`. |
| `GetEstExpenses()` | `ytdExpense + GetTotalMonthlyExpense() × (13 − currentMonth)`. |
| `GetTotalMonthlyExpense()` | **Live** projection, not the stored last-month value: ordinance monthly expense + neighbour deal expense + Σ departments' `GetTotalExpenses()` (current values, no recalculation) + Σ bonds' `monthlyPayment`. *[compat]* The value is 64-bit; return it in the 64-bit return registers so callers using the SDK's 32-bit declaration see its low 32 bits. |
| `GetTotalYearlyExpense()` | stored `yearlyExpense` (only changed by its setter). |
| `GetTotalMonthlyIncome()` | **Live**: Σ_t `GetTaxIncome(t)` + ordinance monthly income + neighbour deal income + Σ departments' `GetTotalIncome()`. 64-bit as above. |
| `GetTotalYearlyIncome()` | stored `yearlyIncome`. |
| `GetTaxIncome()` | `lastTaxIncome` (the last `ComputeTax` result). |
| `GetTaxIncome(t)` | `trunc(float(weightedAssessed[t]) × DifficultyTaxMultiplier[difficulty] × rate[t] × INV1200)`, left to right, one `float32` rounding per multiplication, `weightedAssessed[t]` entering exactly (§5.2; order corrected in Errata round 3, item 2). It uses the *current* rate with last month's assessed value, so the budget window reacts to the tax slider at once. *[choice]* 0 for t outside 0..11 (the original does not check). |
| `SetTotalMonthlyExpense(v)` / `SetTotalYearlyExpense(v)` / `SetTotalMonthlyIncome(v)` / `SetTotalYearlyIncome(v)` | Store into `lastMonthExpense` / `yearlyExpense` / `lastMonthIncome` / `yearlyIncome`. No other effect (the monthly ones are overwritten at the next settlement). Return `v`. |
| `ShowBudgetWindow()` | Open the budget window's sub-panel selected by the panel flag (the original builds one of two budget-UI window procedures, budget-dialogs.md). *[choice]* UI is outside the simulation; return true. |
| `CreateDepartmentBudget(id, group)` | If the department exists, return it unchanged (`group` ignored). Otherwise create it, `Init` it, store it under `id`, set its ID, call its `SetFundingPercentage(100 %, 0)` (a no-op since it starts at 100 %), find-or-create group `group` (name from LTEXT `0x6A3FF01C`/`group`, totals 0), add `id` to the group, set the department's budget group, then **group refresh** (§6.1). Return the borrowed department. |
| `RemoveDepartmentBudget(id)` | Not found → false. Otherwise remove `id` from its group's set and refresh that group, remove it from the map, shut it down and release it → true. |
| `GetDepartmentBudget(id)` | The department or null. |
| `GetDepartmentBudget(name)` | First department in ascending ID whose name **starts with** `name`, compared case-insensitively; null if none. An empty `name` matches the first department. |
| `GetAllGroups(vec)` | Clear `vec`; append `{groupID, name, totalExpense, totalIncome}` for each group in ascending ID (the cached totals of §6.1). |
| `SetGroupName(id, name)` | Find-or-create group `id`; set its name. The name is not kept across save/load (§11.1). |
| `GetDepartmentBudgetsInGroup(id, vec)` | Clear `vec`; if the group exists append each of its departments (ascending department ID) with one reference each. |
| `NeededFundingChanged(dept)` / `FundingPercentageChanged(dept)` | If `dept` is non-null, group refresh for `dept.GetBudgetGroup()`. |
| `GetFunding(id)` / `GetFunding(dept)` | Always 0 *[compat]*. |
| `GetTaxRate(t)` | `rate[t]` for 0 ≤ t < 12, else 0. |
| `SetTaxRate(t, r)` | For 0 ≤ t < 12 store `r` unclamped; no message. *[choice]* return true when stored, false otherwise. |
| `GetWeightedAssessedTaxValue(t)` | `weightedAssessed[t]` for 0 ≤ t < 12, else 0. |
| `GetBondIncrement()` | tuning `BondIncrement`. |
| `IssueBond(amount)` | §5.4. |
| `GetTotalBorrowed()` | `totalBorrowed` (sum of the principal amounts of open bonds). |
| `GetCurrentBorrowingLimit()` | `MaxBonds × MaxBondAmount`. |
| `GetCurrentBondLimit()` | `MaxBondAmount`. |
| `GetCurrentMaxOutstandingBondsLimit()` | `MaxBonds`. |
| `GetTotalMonthlyBondPayments()` | Σ bonds' `monthlyPayment`. |
| `GetAllLoans(vec)` | Clear `vec`; per bond in issue order: `{monthsRemaining, yearsRemaining}` = `(dueMonth − month, dueYear − year)` normalised so months is 0..11, `totalLoanAmount = amount`, `remainingPayment = principalLeft`, `monthlyPayment`. |
| `GetLoanTimeInMonths()` | `BondPaybackMonths`. |
| `GetMonthlyPaymentForLoan(L)` / `GetFullCostOfLoan(L)` | §5.4. |
| `GetBudgetItemInfo(holder, vec)` | If `holder` lacks `0xEA54D283` or it is empty → false, `vec` untouched. Otherwise clear `vec` and fill one `BudgetItem` per department entry, using the defaults of §2 for short arrays and `localFundingPercentage = float32(pct × 0.01f)` (RG13; not `pct/100`) → true. |
| `GetBudgetItemForPurpose(holder, purpose, item)` | The first item whose purpose matches → copy into `item`, true; else false. |
| `ChangeBudgetItemLine(holder, index, line)` | *(The SDK calls the 2nd argument "purpose"; it is the item index.)* If `index` is inside the line array, set entry `index` to `line` and write the whole array back as a property of `holder` (an instance override). No money moves. |
| `ChangeBudgetItemCost(holder, index, cost)` | The same for the cost array. No money moves. |
| `ChangeBudgetItemLocalFunding(holder, index, pct)` | Set entry `index` of the local-funding array to `pct × 100`, first **growing the array to index + 1 with 0.0** if it is shorter *[compat]* (so other items that had no entry become 0 % funded), and write it back. |
| `AddBudgetItemToBudget(holder, index)` | Read the holder's items; if `index` is valid and its department exists and has the item's line: `line.AddToFullExpenses(cost)`. Nothing is created. |
| `RemoveBudgetItemFromBudget(holder, index)` | The same with `−cost`. |
| `GetLineItemFromBudgetItem(item)` | `GetDepartmentBudget(item.departmentID)?.GetLineItem(item.lineItem)`, else null. |
| `CopyBudgetItemProperties(src, dst)` | Each of the five properties present on `src` is copied (whole array) onto `dst`. |

Methods the SDK declares `bool` and that are not given a result above return true *[choice]*; the original returns
no meaningful value.

### 6.1 Group refresh
For group `g` (if it exists): sum `GetTotalExpenses()` and `GetTotalIncome()` over its departments. If either sum
differs from the cached totals, store both and post `0x0A3E7624` BudgetGroupChanged with data1 = group ID,
data2 = low 32 bits of the expense total, data3 = low 32 bits of the income total.

## 7. `cISC4DepartmentBudget` methods (SDK order)

| Method | Behaviour |
|---|---|
| `GetDepartmentID()` | its ID. |
| `GetDepartmentName(str)` | copy the display name into `str`. |
| `SetDepartmentName(ltextGroup, ltextInstance)` | store both; name := the localised string. |
| `GetBudgetGroup()` / `SetBudgetGroup(g)` | plain accessors. *[compat]* `SetBudgetGroup` does **not** move the department between the budget simulator's group sets. |
| `GetIsFixedFunding()` / `SetFixedFunding(b)` | plain accessors. |
| `GetFundingPercentage()` | pointer to `p`. |
| `SetFundingPercentage(q, arg)` | **SetPct(q, arg, redistribute = true)** (§7.1). `arg` is only copied into the message. |
| `GetMaxAllowedFundingPercentage()` | pointer to `pMax`. |
| `SetMaxAllowedFundingPercentage(q)` | store; `p` is not re-clamped. |
| `GetIdealMonthlyFunding()` | Σ line items' full expenses. |
| `GetTotalExpenses()` | Σ line items' current expenses. |
| `GetTotalIncome()` | Σ line items' income. |
| `CreateLineItem(id, isLocal)` | Always creates a new line item (id, owner = this, isLocal, initialized) and stores it under `id`, **replacing** any existing entry *[compat]*. Returns it borrowed. |
| `CreateLineItemForBuildingType(iid, isLocal)` | `CreateLineItem(iid, isLocal)` then its `SetName(0, iid)` (the building type's name). |
| `RemoveLineItem(id)` | If present: detach it from this department, release it, remove it. |
| `GetLineItem(id)` | borrowed pointer or null. |
| `GetAllLineItems(vec)` | Clear `vec`; append every line item (ascending ID) with one reference each. |
| `AddLocallyFundedObject(holder, purpose)` | Precondition: `holder` has a budget item for `purpose` in this department whose line exists. Add (holder, purpose) to the set, holding a reference. Then `line.AddToFullExpenses(cost)`; `line.current := max(0, line.current + trunc(p × cost))`; `ChangeBudgetItemLocalFunding(holder, item.index, p)` (the new building starts at the department's percentage); `budgetSim.FundingPercentageChanged(this)`. |
| `RemoveLocallyFundedObject(holder, purpose)` | If registered: `SetLocalFullFunding(holder, 0, purpose)` (*[compat]* this sets the building's cost property to 0), release the reference, remove it. Then, if no local objects remain and `GetIdealMonthlyFunding() == 0`: `SetFundingPercentage(100 %, 0)`. |
| `SetLocalFundingPercent(holder, q, purpose)` | §7.3. |
| `SetLocalFullFunding(holder, cost, purpose)` | §7.4. |
| `GetLocalFundingPercent(holder, purpose)` | **Returns a pointer to a float, not a float** *[compat]* (see the erratum below): if registered and the item exists, a pointer to the item's local funding fraction; else a pointer to a constant 0.0. |
| `GetLocalFullFunding(holder, purpose)` | If registered and the item exists: the item's cost; else 0. |
| `IsLocallyFundedObjectInDepartment(holder, purpose)` | set membership. |
| `RecalculateAllLineItemCurrentExpenses()` | §7.2. |
| `GetTotalSpending()` / `SetTotalSpending(v)` | year-to-date spending accessors. |
| `GetLocallyFundedItemsByPurpose(purpose, list)` | **Append** (no clear) every registered holder with that purpose, one reference each. |
| `HasLocallyFundedItemsByPurpose(purpose)` | any registered pair with that purpose. |

**Department change message** `Notify(type, data3)` *[compat]*: if `type` is FundingPercentageChanged, first call
`budgetSim.FundingPercentageChanged(this)`; if IdealMonthlyFundingChanged, `budgetSim.NeededFundingChanged(this)`.
Then post `type` with void1 = this department, the IGZUnknown slot = this department, data2 = department ID,
data3 = `data3`.

### 7.1 SetPct(q, arg, redistribute)

```
q := clamp(q, 0, pMax)
if fixedFunding or q == p: return
p := q
for each line item that is not local and whose type is not Income (1):
    current := max(0, trunc(float(full) × q + 0.5))
if redistribute: DistributeToBuildings()
Notify(0x08DAC3E7 FundingPercentageChanged, arg)
```

**DistributeToBuildings** *[compat]* (only when the department has local objects). Work in whole simoleons:

```
for each registered (holder, purpose) whose item exists:
    c := low32(item.cost);  cur := floor(c × item.localFunding + 0.5);  hi := floor(c × pMax + 0.5)
C := Σc;  R := floor(C × p + 0.5) − Σcur                  (remainder to distribute)
active := all objects;  activeCost := C;  k := R / C (float)
while R ≠ 0 and active is not empty:
    for each active object in set order:
        if R < 0: d := max(floor(c × k), R); if cur + d < 0:  d := −cur; mark it done
        else:     d := min(ceil(c × k),  R); if cur + d > hi: d := hi − cur; mark it done
        if marked done now: remove from active, activeCost −= c
        cur += d;  R −= d;  stop the pass when R == 0
    k := R / activeCost
set every local line's current to 0
for each object: ChangeBudgetItemLocalFunding(holder, item.index, cur / c)
                 line(item.line).current := max(0, line.current + cur)
```

*[choice]* Objects are kept in **insertion order** (the original orders them by memory address, which changes
which building receives the rounding remainder). *[choice]* Objects with `c == 0` are skipped when computing
`cur / c` (their local funding is left unchanged) and do not stall the loop (the original can divide by zero or
loop forever). Example: three buildings costing 101 each, all at 100 %, `SetFundingPercentage(0.5)` → amounts
50, 50, 52 (target 152).

### 7.2 RecalculateAllLineItemCurrentExpenses

```
for each local line of type Expense: sum[line] := 0
n := number of non-local lines
for each registered (holder, purpose):                       (all of the holder's items, not only `purpose`)
    for each budget item of holder with department == this and a line in `sum`:
        sum[line] := trunc(float(cost) × localFunding × 100 + float(sum[line]));  n += 1
for each line in sum: line.current := max(0, sum[line] div 100)
C := Σ all lines' current;  F := Σ all lines' full
if F > 0:
    a := C × 100 / F                                           (float, percent)
    if |a − 100p| > 0.5 and |C − p × F| > n + 2:
        SetPct(clamp(a, 0, 100·pMax) / 100, 0, redistribute = true)
```

This re-synchronises the department slider with the actual per-building funding after individual buildings were
changed. *[compat]* A holder registered under two purposes of the same department is counted twice.

*[compat]* Precision (confirmed, Errata round 3, item 1): the `sum[line]` step is three `float32`-rounded
operations, left to right: `cost × localFunding`, then `× 100`, then `+ sum[line]`, each rounded to `float32`
(foundations §18; `cost` and `sum[line]` are 64-bit integers that enter exactly), then truncation toward zero.
`div 100` is a signed 64-bit integer division (truncating). Example: cost 300 at local funding `0.41f` (stored as
41.0 %) gives `300 × 0.41f = 122.999998927…`, rounded to `123.0`, so `12300` and a current expense of **123**;
at `0.35f`, **105**. Evaluating the same expression in `double` gives 122 and 104 instead, which is **wrong**.
The original game wrote exactly these values: a city saved by the unmodified game (Spandau, a 300-simoleon clinic
at 41 % and a 300-simoleon school at 35 %) holds year-to-date department spending of 369 = 3 × 123 and
315 = 3 × 105 after three settlements.

The single-line variant (used by §7.3) recomputes only one local Expense line the same way, without the
re-synchronisation step.

### 7.3 SetLocalFundingPercent(holder, q, purpose)

Ignored unless `0 ≤ q ≤ pMax`, the pair is registered and the item exists. Then:

```
newC := floor(q × 100 × cost + 0.5);  oldC := floor(cost × item.localFunding × 100 + 0.5)    (hundredths)
q' := p
if newC ≠ oldC:
    F := GetIdealMonthlyFunding()
    q' := (F == 0) ? q : ((newC − oldC) + floor(p × 100 × F + 0.5)) / F × 0.01
    if holder is an occupant: post 0x6A5FD3FD BudgetForOccupantChanged (IGZUnknown slot = the occupant)
ChangeBudgetItemLocalFunding(holder, item.index, q)
recompute the item's line (single-line variant of §7.2)
if q' ≠ p: SetPct(q', 0, redistribute = false)
```

### 7.4 SetLocalFullFunding(holder, cost', purpose)

Ignored unless registered and the item exists. Then: `ChangeBudgetItemCost(holder, item.index, cost')`;
`δ := cost' − item.cost`; `δC := floor(item.localFunding × 100 × δ + 0.5)`; `F := GetIdealMonthlyFunding()`
(before the change); new department percentage `q' := 100 %` if `F + δ == 0`, else
`(floor(F × p × 100 + 0.5) + δC) / (F + δ) × 0.01`. Then `line.AddToFullExpenses(δ)`;
`line.current := max(0, line.current + δC div 100)`; `Notify(0x08DAC3E8 IdealMonthlyFundingChanged, 0)`;
`SetPct(q', 0, redistribute = false)`.

*[compat]* Subscribers must tolerate IdealMonthlyFundingChanged arriving twice here (once from the line item,
once from the department); we may post it once *[choice]*.

## 8. `cISC4LineItem` methods (SDK order)

| Method | Behaviour |
|---|---|
| `GetID()` | its ID. |
| `GetType()` / `SetType(t)` | plain (0 Expense, 1 Income). |
| `GetName(name)` | If the name still needs resolving after a load, resolve it first (as `SetName`). Copy it out. *[compat]* The SDK warns that the original takes the game's internal string type. |
| `SetName(group, instance)` | Store both. `group == 0` → name := `cISC4BuildingDevelopmentSimulator::GetBuildingName(instance)` (compiled slot 38), the building type's **user-visible name** (rule in Errata round 2, item 2); otherwise the LTEXT string (`group`, `instance`). |
| `GetSecondaryInfoField()` / `SetSecondaryInfoField(v)` | plain. |
| `GetFullExpenses()` / `GetCurrentExpenses()` | plain. |
| `SetFullExpenses(v)` | `full := v`; if not local: `current := trunc(v × owner.p)` (no rounding, no clamp; Income lines too). Then the owner's `Notify(0x08DAC3E8, 0)`. |
| `AddToFullExpenses(v)` | `SetFullExpenses(full + v)`. |
| `GetIncome()` | plain. |
| `SetIncome(v)` | `income := max(0, v)`; owner `Notify(0x08DAC3E8, 0)`. |
| `AddToIncome(v)` | `income += v` (no clamp); owner `Notify(0x08DAC3E8, 0)`. |
| `GetDisplayFlags()` | flags (1 ShowLineItem, 2 ShowSecondaryInfoField, 4 no visible effect). |
| `SetDisplayFlag(f, on)` | set or clear the bits of `f`. |
| `IsLocalFundingItem()` | `isLocal`. |

## 9. Tuning

`Init` reads the exemplar with TGI type `0x6534284A`, group `0xE7E2C2DB`, instance `0x0BFF6824` ("Budget Tuning
Parameters", `SimCity_1.dat`) through the resource manager as a property holder. Code defaults are installed
first and kept for any missing property.

| Property | Name | Type (in data) | Code default | Shipped value |
|---|---|---|---|---|
| `0x6A9C7718` | Initial Funds [easy, med, hard] | sint64[3] | none (empty) *[choice]* use shipped | 500000, 200000, 100000 |
| `0x890E0A99` | MaxTaxRate | float | 25 | 25 |
| `0x090E0A9F` | MinAllowableFunds | sint64 | −100000 | −100000 |
| `0x490E0AA6` | BondIncrement | sint64 | 1000 | 5000 |
| `0x090E0AAC` | MaxBonds | uint8 (read as int32) | 5 | 10 |
| `0x890E0AB4` | MaxBondAmount | sint64 | 10000 | 200000 |
| `0x8A3FB98D` | BondInterestRate, in percent; **× 0.01 when present** | float | 0.085 (already a fraction) | 8.5 |
| `0x0A3FB9A8` | BondPaybackMonths | uint32 | 120 | 120 |
| `0x0C032488` | Initial Tax Rate [difficulty] | float[3] | none *[choice]* use shipped | 9, 9, 9 |
| `0xEBFF69E7` | Difficulty Tax Multiplier [difficulty] | float[3] | 100000 ×3 *[compat]* | 1.2, 1.1, 1.0 |
| `0x8A480DF7` | Waning Wealth multiplier [Δwealth 0, 1, 2] | float[3] | 1, 1, 1 | 1, 0.75, 0.5 |
| `0x4AC519B8` | Land Value → simoleons per tile (response curve, (x, y) pairs) | float[] | (0,0), (255,10) | (0,0), (255,0) |
| `0xA9188EF5`, `0xE918C920`, `0x4918C925` | TaxRateOccWeight R$, R$$, R$$$ | float | 1 | 1 |
| `0xA918C929`, `0x4918C92C`, `0x4918C930` | TaxRateOccWeight CS$, CS$$, CS$$$ (see §13.1) | float | 1 | 1 |
| `0x8918C937`, `0x4918C93A` | TaxRateOccWeight CO$$, CO$$$ | float | 1 | 1 |
| `0x69A36E40`, `0xA918C942`, `0x8918C93E`, `0xA9A36E47` | TaxRateOccWeight I-R, I-D, I-M, I-HT | float | 1 | 1 |
| `0x89170C6C`, `0x09170C74`, `0xC9170C78`, `0x49170C7C` | LV→R/C/I tax factors, MinAverageLV | float (`0x49170C7C`: read with the u8 getter `0x005FD3C0`, at `0x006923C4`) | — | absent | 

The last row's properties are read into global values but nothing uses them; they need not be implemented.
The exemplar also holds `0x4998C89D` (uint8 0), which the budget does not read.

The land-value curve is a piecewise-linear response curve over its (x, y) points, clamped at the end points
*[inferred]* (the shared response-curve spec governs). With the shipped flat curve, land value has no effect.

With the shipped data the borrowing limit is 10 × 200,000 = 2,000,000.

## 10. Messages

### 10.1 Posted (all through `MessagePost(msg, false)` on the message server)

| ID | Name | When | Data |
|---|---|---|---|
| `0x0772FAD4` | FundsChanged | funds changed (§6), SimBegin, resets | void1 and IGZUnknown = the budget simulator |
| `0xC772FADA` | BondIssued | `IssueBond` succeeded | void1 and IGZUnknown = the budget simulator |
| `0xCA3FBDCE` | BondFinished | a bond reached its due month | void1 and IGZUnknown = the budget simulator |
| `0xAA5CF1E3` | EndGameForBudgetFailure | month-end with funds ≤ MinAllowableFunds | none |
| `0x0A3E7624` | BudgetGroupChanged | group totals changed (§6.1) | data1 = group ID, data2/data3 = low 32 bits of expense/income |
| `0x08DAC3E7` | FundingPercentageChanged | department percentage changed (§7.1) | void1 and IGZUnknown = department, data2 = department ID, data3 = caller's argument |
| `0x08DAC3E8` | IdealMonthlyFundingChanged | a line's full expense or income changed, §7.4 | same layout, data3 = 0 |
| `0x6A5FD3FD` | BudgetForOccupantChanged | a building's local funding amount changed (§7.3) | IGZUnknown = the occupant |

FundsChanged, BondIssued and BondFinished are only posted once `Init` has acquired the message server.

"void1 = X" *[compat]*: `SetVoid1(X)` with X the sender's **primary interface pointer** (the
`cISC4BudgetSimulator*` or `cISC4DepartmentBudget*` that callers get from `GetBudgetSimulator()` /
`GetDepartmentBudget()`). Because void1 and data1 share one slot (foundations §5.1), data1 reads back as that
pointer's 32-bit value, not 0. The IGZUnknown slot holds a counted reference to the same object. No receiver in
the shipped game reads void1/data1 of these messages (details: Errata round 2, item 4); FreeCity4 sets it anyway
for plugins.

### 10.2 Handled
§4.3. History series written: `0xAABAAA9F`–`0xAABAAAAA` (tax rate × 10, daily), `0x6A51178B` (income),
`0xCA5BA3FB` (expense), `0x4A5BA3D0` (funds / 1000), all through `RecordHistoryData`.

## 11. Persistence

All records are little-endian *[compat]*. Types: `bool` = u8 (0/1), `pct` = float32 fraction, `str` = a string as
written by `cIGZOStream::SetGZStr`, `ref` = a serializable-object reference as written by the save-file segment
stream (the same mechanism other city objects use for references; see §14 question 1). Existing saves must load,
and saves we write must load in the original game.

### 11.1 Budget simulator (CLSID `0xE990BE01`)

```
u16   version = 5
s64   funds
s64   shadowFunds
s64   totalBorrowed
s64   lastTaxIncome
u32   departmentCount
      departmentCount × { u32 departmentID; ref department (CLSID 0xE990BFFC) }      ascending ID
12 ×  { f32 rate; f32 maxRate; 12 × s64 history; s64 reserved; s64 weightedAssessed; s64 monthlyTax }
s64   ytdOrdinanceExpense
s64   ytdOrdinanceIncome
12 ×  s64 ordinanceExpenseByMonth
12 ×  s64 ordinanceIncomeByMonth
s64   ytdTaxIncome
5 ×   s64 ytdReserved
u32   bondCount
      bondCount × { s32 dueMonth; s32 dueYear; s64 amount; s64 fullCost; s64 remaining; s64 paid;
                    s64 monthlyPayment; s64 principalLeft }                             issue order
s64   ytdBondPayments
s64   lastMonthIncome
s64   yearlyIncome
s64   lastMonthExpense
s64   yearlyExpense
u32   groupCount
      groupCount × { u32 groupID (map key); u32 groupID; str name; s64 totalExpense; s64 totalIncome;
                     u32 n; n × u32 departmentID (ascending) }                           ascending ID
```

Reading:
- `version < 4` → fail. *[choice]* `version > 5` → fail (the original has no upper check).
- Version 4 has the same layout. If version < 5 and tax group 8's rate read as exactly 0.0, set
  `agricultureRateFix` (Init then gives agriculture the default rate) *[compat]*.
- Set `loadedFromSave`, so `Init` skips the new-city setup.
- `bondCount` is taken from the file.
- Group names in the file are **discarded** and replaced by the LTEXT string (`0x6A3FF01C`, group ID) *[compat]*.
- `ytdIncome` and `ytdExpense` are not in the record and stay 0 after a load *[compat]*.
- The result is false if the stream reports an error.

### 11.2 Department (CLSID `0xE990BFFC`)

```
u16   version = 10
bool  initialized
u32   departmentID
str   name
u32   budgetGroup
pct   fundingPct
pct   maxFundingPct
s64   totalSpending
bool  fixedFunding
u32   lineCount;   lineCount × ref line item (CLSID 0xAA313C9F)                ascending line ID
u32   localCount;  localCount × { ref property holder (IID 0x25216283); u32 purpose }
u32   nameLtextInstance
u32   nameLtextGroup
```

Reading accepts versions 0..10; `version ≥ 11` → fail. For `version < 10` the two percentages are stored in
percent and are multiplied by 0.01. Line items are keyed by their own `GetID()`. If both name LTEXT IDs are
non-zero, the name is re-resolved from LTEXT; otherwise the stored string is kept.

### 11.3 Line item (CLSID `0xAA313C9F`)

```
u16   version = 4
str   name
s64   secondaryInfo
s64   fullExpenses
s64   currentExpenses
s64   income
u32   id
u32   displayFlags
bool  isLocal
u16   type
ref   owner department (CLSID 0xE990BFFC)
u32   nameLtextInstance
u32   nameLtextGroup
```

Reading requires `version == 4` exactly. After reading, the name is marked to be re-resolved on the next
`GetName`.

## 12. Interactions with other subsystems

| Caller | Uses |
|---|---|
| City | creates the budget simulator, `Init`/`Shutdown`, saves and loads it, `GetBudgetSimulator()`. |
| Simulator | delivers SimBegin, SimNewDay, SimNewMonth to the three agents (§4.1). |
| Service simulators (police, fire/protection, civic buildings incl. schools, hospitals and rewards, power, plumbing, pollution/landfill, traffic/transit, parks, port and landfill developers) | at `Init`: `CreateDepartmentBudget`, `SetDepartmentName`, `SetMaxAllowedFundingPercentage`. On lot activation: `GetBudgetItemForPurpose`; if the item's line is 0, `ChangeBudgetItemLine(holder, index, buildingType)`; `GetDepartmentBudget`; `GetLineItem` or `CreateLineItemForBuildingType`; then either `AddBudgetItemToBudget`/`AddToFullExpenses` (non-local), `AddLocallyFundedObject` (local; the power simulator also counts plants in the secondary info field and shows it), or `SetType(Income)` + `AddToIncome` (civic income purpose `0x4A5A495E`). Removal reverses this. For effectiveness they read `GetBudgetItemForPurpose(...).localFundingPercentage`. Police, protection and traffic subscribe to FundingPercentageChanged. |
| Building development simulator | `GetBudgetItemInfo` when loading building properties. |
| Neighbours simulator | the budget calls its deal income/expense totals monthly; it reads the power (`0x8910BC8A`) and water (`0xAA4014B4`) departments' expenses for commodity cost per unit. |
| Ordinance simulator | the budget calls `GetOrdinanceMonthlyIncome/Expense` monthly; some ordinances look up departments to compute their own cost. |
| Aura, demand | read tax rates (`GetTaxRate`) and the daily tax-rate history series. |
| Advisors | `GetDepartmentBudget` and funding queries. |
| Placement, bulldoze and network tools, missions | `GetTotalFunds`, `WithdrawFunds` (purchases), `DepositFunds` (mission rewards). |
| Budget window and query tools | groups, departments, line items, tax rates, `SetFundingPercentage`, `SetLocalFundingPercent`, `SetLocalFullFunding`, bonds, estimates; sends ToggleBudgetDialog. |
| 3D view | handles EndGameForBudgetFailure (§5.6). |

Known department IDs (names inferred from the buildings that use them): `0xA2963983` police, `0xA2963984` jail,
`0x28F55A9F` fire, `0x09188F4C` education (+`0x4A538CC6` schools), `0x09188F42` health (+`0xAA538CB3`),
`0x8910BC8A` power, `0xAA4014B4` water, `0x6A42A8F5` sanitation/landfill, `0xC921CEEB` transit, `0xC921CEEA`
tolls, `0xEA59717A` civic/reward buildings, `0x2A5A723F` business-deal rewards, `0x8A416368` landmarks,
`0x8A4260A6` Parks and Recreation (created by the park manager; the cruise-ship reward building is also charged to
it; foundations §5.2, which corrects the earlier "cruise ship" label).

## 13. Original bugs (reproduced)

### 13.1 CS$$$ tax weight is never loaded *[compat]*
The tuning reader stores property `0x4918C930` (TaxRateOccWeight CS$$$) into the **CO$$$** weight, and then
property `0x4918C93A` (CO$$$) overwrites it. Required behaviour:
- the CS$$$ weight (group 5) is always 1.0 regardless of data;
- the CO$$$ weight (group 7) is `0x4918C93A` if present, else `0x4918C930` if present, else 1.0.

With the shipped data every weight is 1, so nothing visible changes; it matters only for mods that edit these
weights. *[choice]* A configuration switch may enable the intended behaviour, off by default.

### 13.2 Neighbour deal prices use the wrong curve *[compat]*
The neighbours simulator builds its deal-price scarcity curve from `0x0A469353` at start-up, but its monthly
commodity update reloads the **same** curve object from the garbage cost curve `0x4B61FFC9`. From the first month
on, deal prices use the garbage curve. The budget books the resulting values unchanged (§5.5); the fix, if any,
belongs to the neighbours spec.

### 13.3 Minor original behaviours kept *[compat]*
History values above 2^32 are recorded as `0xFFFFFFFF` (§5.1); `ChangeBudgetItemLocalFunding` pads with 0 %
(§6); `RemoveLocallyFundedObject` zeroes the building's cost property (§7); `CreateLineItem` replaces an
existing line (§7); `SetBudgetGroup` does not move the department (§7); year-to-date income and expense are not
saved (§11.1). The per-group 12-month history, the per-group reserved value and `ytdReserved[5]` are never
updated by the original; they are kept only so that save records keep their layout.

## 14. Open questions

1. The exact byte format of a serializable reference (`ref` in §11) and of `SetGZStr` strings. This belongs to the
   savegame/segment-stream spec, which does not exist yet.
2. Agent order inside the month level decides whether the neighbours simulator prices this month's deals before
   or after the budget books them. Our simulator spec runs agents in registration order *[choice]*; the original
   orders them by object address. Which order did the shipped game normally produce?
3. ~~Disaster `0x42963812` is not identified.~~ Resolved: it is the earthquake (§4.3).
4. Response-curve evaluation (interpolation, clamping, empty curves) needs its own shared spec.
5. Who calls `SetTotalMonthlyIncome/Expense` and `SetTotalYearlyIncome/Expense`? Is `yearlyIncome`/`yearlyExpense`
   ever non-zero in the shipped game?
6. The per-group `maxRate` is stored and saved but nothing in the budget enforces it. Does the tax UI clamp to it?
7. *(resolved)* Exact floating-point results of the Windows tax computation: the x87 runs at 24-bit precision
   (foundations §18), so the tax computation is a sequence of `float32` operations; a strict float32
   implementation in the original order matches it.
8. Which departments are locally funded in the shipped game, and each department's `pMax` (set by the service
   simulators; out of scope here).
9. *(resolved, modal-dialogs.md §4.1)* The budget-failure sequence pauses, shows scripted dialog `0x0A5CF71D`
   when popups are enabled (its answer is ignored), resumes and requests the region view without saving. No other
   subscriber to `0xAA5CF1E3` was found.
10. Which of the two budget sub-panels each ToggleBudgetDialog value opens (UI only).
11. Behaviour of `ComputeTax` for a building whose wealth is below its lot's wealth (negative waning index) in the
    original; we use 1.0 *[choice]*.

## 15. Conformance tests (must exist in `engine/tests`)

1. **Funds**: deposit/withdraw of positive, zero and negative amounts; withdrawing more than the funds fails and
   changes nothing; withdrawing with negative funds fails; deposit saturates at INT64_MAX; FundsChanged posted only
   on change; shadow funds track every write; a tampered `funds ≠ shadow` is restored at the next month-end.
2. **Tax**: the tax-type table (all 36 cells, untaxed cells skipped); a 10,000 building at 9 % pays 90 / 82 / 75
   (easy / medium / hard; 82 = trunc(82.5)); waning-wealth multipliers 1 / 0.75 / 0.5; land-value curve applied as
   `roundHalfEven(curve(lv) × sizeX × sizeZ)`; `GetTaxIncome(t)` follows a rate change at once; totals within
   ±1 per 1000 lots of a float64 reference. **Operation order** (§5.2): on easy (multiplier 1.2) at 7 %, a group
   with `weightedAssessed` 998,857 gives `GetTaxIncome(t)` = **6992** (`(998857 × 1.2f) × 7 × INV1200` = 6992.0);
   the order `(7 × 1.2f) × 998857 × INV1200` gives 6991.9995 → 6991, which is wrong. A single lot assessed at
   998,857 in that group adds 6992.0 to `groupTax`.
3. **Settlement**: with stub ordinance (income 50, expense 30), neighbours (income 7, expense 11), a department with
   one non-local line (full 1000, p = 0.5) and one bond, funds change by exactly `income − expense`; YTD values,
   last-month values and the three history series are as in §5.1, including the `0xFFFFFFFF` clamp.
4. **Year end**: after the settlement on January 1st every YTD accumulator and department spending is 0, while
   the by-month ordinance arrays keep their values. The history series `0x6A51178B`/`0xCA5BA3FB` hold that
   settlement's income and expense, written before the reset. The funds changed by their difference (§5.1, BQ2).
5. **Bankruptcy**: EndGameForBudgetFailure is posted at month-end with funds = −100,000 and at every later
   month-end while still at or below it; not posted at −99,999; never posted by `WithdrawFunds`.
6. **Bonds**: rounding to the increment (7,499 → 5,000; 1 → 5,000 with increment 5,000); count, size and total
   limits; payment 123 for 10,000 at 8.5 %/120 months; exactly 120 payments, the last equal to the remaining
   principal; BondIssued/BondFinished messages; `totalBorrowed` and `GetAllLoans` values month by month.
7. **Department percentage**: clamping to [0, pMax]; fixed funding ignores changes; non-local Expense lines rescale
   with `trunc(full × q + 0.5)`, Income lines and local lines do not; FundingPercentageChanged carries data2 =
   department ID and data3 = the caller's argument.
8. **Redistribution**: {101, 101, 101} at 100 % → 50 % gives {50, 50, 52} in insertion order; buildings clamp at
   `pMax`; local funding properties are written as amount/cost × 100.
9. **Per-building funding**: `SetLocalFundingPercent` updates the building, its line and the department percentage
   (§7.3) and posts BudgetForOccupantChanged; `SetLocalFullFunding` and `RemoveLocallyFundedObject` behave as in
   §7.4 and §7, including the cost property set to 0 and the reset to 100 % when the department empties.
10. **Re-synchronisation**: `RecalculateAllLineItemCurrentExpenses` moves the department percentage only when both
    thresholds of §7.2 are exceeded. **Precision** (§7.2): one locally funded building with cost 300 on a local
    Expense line, local funding property 41.0 → current expense **123** (not 122); 35.0 → **105** (not 104); the
    department percentages 0.41 and 0.35 stay unchanged (no re-synchronisation).
11. **Budget items**: `GetBudgetItemInfo` with short arrays (defaults 0 and 100 %), missing department array →
    false; `GetBudgetItemForPurpose` returns the first match; `ChangeBudgetItemLocalFunding` pads with 0.
12. **Groups**: creating departments in a group, BudgetGroupChanged only when totals change, `GetAllGroups` and
    `GetDepartmentBudgetsInGroup` order and reference counts; prefix, case-insensitive `GetDepartmentBudget(name)`.
13. **Tuning**: code defaults with no exemplar; shipped values with the exemplar, including BondInterestRate × 0.01
    and the CS$$$/CO$$$ weight behaviour of §13.1.
14. **Daily**: history value `roundHalfEven(f32(rate × 10))`: the product is rounded to `float32` first
    (foundations §18), so most one-decimal rates become exact ties. 9.25 → 92, 9.35 → 94, **9.15 → 92**
    (9.15f × 10 = 91.4999962 rounds to 91.5, then to even), **9.05 → 90** (90.5000019 → 90.5 → 90).
15. **Estimates**: `GetEstIncome/Expenses = YTD + live monthly × (13 − month)`.
16. **Save**: write → read → write is byte-identical for all three records; budget version 3 and line-item
    version 3 or 5 are rejected; a version-4 budget with agriculture rate 0 gets the default rate at `Init`; a
    version-9 department converts percent to fraction; group names come from LTEXT after loading.

## Errata / clarifications (round 2)

Answers to the implementer's questions of round 2. The main text above has been corrected where it was wrong
(§4.1 step 9, §4.3 RiskyMoney, new §4.4, §5.2, §8 `SetName`, §10.1, §14 question 3).

1. **Null object values in property holders**: see foundations, Errata round 2, item 1 (a null-object entry is
   *dropped* on load, like every object-valued entry). It does not affect the budget's own records, which hold
   holders only by reference.

2. **Display name of a building type** (`cISC4LineItem::SetName(0, buildingType)`) *[compat]*. The
   building-development simulator keeps one name per building type, computed when it loads that building's
   exemplar (`LoadBuildingProperties`; a reload replaces the entry):

   ```
   if exemplar has 0x8A416A99 "User Visible Name Key" (uint32[3] = {type, group, instance})
      and the LTEXT (group, instance) loads in the current language:   name := that LTEXT string
   else if exemplar has 0x899AFBAD "Item Name" (string):             name := that string
   else:                                                             name := "Building"
   ```

   The type word of the key is ignored (it is the LTEXT type `0x2026960B` in the data). Inherited values count
   (the lookup goes through the exemplar's parent). A building type that was never loaded has the **empty**
   name. The budget obtains the name with `GetBuildingName(buildingType)` (compiled slot 38 of
   `cISC4BuildingDevelopmentSimulator`); the returned string belongs to the simulator, so copy it and do not
   release it. (Slot 37, `GetExemplarName`, is a different string: property `0x20` "Exemplar Name", else
   "Building".) A reimplementation may either call the game's `GetBuildingName` or apply the rule above to the
   exemplar itself; both give the same text.

3. **Game-data registry**: §4.4. Identity: system service `0xEBE0E860`, IID `0xE898ED03` (`cISC4GameDataRegistry`).
   It feeds the Lua table `sc4game.budget`. No shipped script or C++ code reads it, so leaving it out changes
   nothing in the shipped game. Publish it anyway for Lua mods *[choice]*.

4. **void1 of budget and department messages**: it holds the sending object (the budget simulator for
   FundsChanged / BondIssued / BondFinished, the department for FundingPercentageChanged /
   IdealMonthlyFundingChanged), as its primary interface pointer. The same object is in the IGZUnknown slot (a
   counted reference). Shipped receivers ignore it:
   - FundingPercentageChanged: police, fire protection and traffic compare **data2** (department ID) with their
     own department's ID; the budget window only sets a "dirty" flag.
   - FundsChanged: the 3D view, the vehicle manager and the landfill developer query the budget simulator (or
     their department) directly.
   - BudgetForOccupantChanged: the animation coordinator reads the **IGZUnknown** slot and queries it for
     `cISC4Occupant` (`0x26BB767E`).
   - BudgetGroupChanged and the bond messages: nothing reads their fields.
   *[compat]* FreeCity4 sets void1 (and therefore data1) as specified so that plugins see the same values.

5. **`GetLotsInCellRect`**: the budget does not call it (§5.2). For reference, as used by other subsystems:
   `GetLotsInCellRect(list, x1, z1, x2, z2, bool)` clears `list`, clamps the rectangle to
   `[0, CellCountX − 1] × [0, CellCountZ − 1]` (bounds **inclusive**, cell units), returns 0 if it is empty, and
   otherwise appends lots with one reference each and returns their number:
   - `bool = false`: every lot that **overlaps** the rectangle, each once, ordered by its first overlapped cell
     with x as the outer loop and z as the inner loop (both ascending);
   - `bool = true`: only lots whose **location cell** (`cISC4Lot::GetLocation`) is inside the rectangle, ordered
     with z outer and x inner.

   The whole-city query that matches the budget's lot set is therefore `(0, 0, CellCountX − 1, CellCountZ − 1,
   false)`.

6. **RiskyMoney**: §4.3. The disaster is the **earthquake** (`0x42963812`). The budget does not convert to
   cells. It passes integer **world** coordinates, truncated toward zero, to the earthquake manager's
   `AttemptDisasterStart(int32, int32)`. The earthquake clamps them to the city and converts them with
   `cISC4City::PositionToCell` (16 m cells, origin at the city's (0, 0) corner, floor).

7. **Stream errors**: confirmed. None of the three budget classes ever sets an error on a stream. Their `Read`
   returns `GetError() == 0` (compiled slot 18 of `cIGZIStream`) after reading everything. A rejected version
   returns false without consuming more and without changing the stream state. Every other failure (short
   record, unknown referenced class, unreadable string) is the stream's own sticky error state (foundations
   §2.1 and §2.9). Partial-failure handling needs nothing more.

8. **Slot table re-verified** *[compat]*. All 61 slots of the original budget vtable (`0x00AB0908`) were compared
   by `ret` operand with the SDK header compiled under the MSVC rule (foundations §16). All 59 interface slots
   match, and the table at the top of this spec is correct. Slots 27 and 28 have the same argument size. The
   body tells them apart: slot 28 is a map lookup on an integer, and slot 27 calls string methods on its
   argument. Also checked: the `FreeCity4.dll` deployed at 14:50 on 2026-10-12 (the one that crashed in-game)
   already had `GetTaxIncome(int32)` at slot 18 and `GetTaxIncome()` at slot 19, and all its readable `ret`
   operands match the game. **The in-game crash was therefore not caused by overload order** and its cause is
   still open.

   New tables related to the budget's own calls: `cISC4DisasterManager`, `cISC4GameDataRegistry`,
   `cISC4BuildingDevelopmentSimulator` and `cISC4LotManager` are in foundations §17.

## Errata / clarifications (round 3): first in-game differential test

The same save (Spandau, about 2,300 residents) was run once with the original budget (run 19) and once with
ours (run 20), the original simulator active in both. The first month-end (2001-10) starts from identical state.
Two differences were reported: health and education expense (122 / 104 in the original, 123 / 105 in ours) and
tax income by type (types 4, 6 and 9). Findings:

1. **Department expense: the spec and our implementation are right; the run-19 reference is not.** The current
   expense of a local line is recomputed at every settlement by §7.2, whose `sum[line]` step is
   `trunc(cost × localFunding × 100 + sum)`, each operation rounded to `float32` (the game sets 24-bit x87
   precision at start-up, foundations §18). Health and education here each hold one 300-simoleon building
   (local funding 41 % and 35 %), so the step gives `300 × 0.41f → 123.0` and current expense 123, and
   `300 × 0.35f → 105.0`, 105. The values 122 and 104 are what the same expression gives in 53- or 64-bit
   precision (`12299.99989…`, `10499.99982…`). The save itself was written by the unmodified game (before any
   FreeCity4 build existed) and records 123 / 105 as the current expenses and year-to-date spending of
   369 = 3 × 123 and 315 = 3 × 105. So the unmodified game produced 123 / 105 at three settlements in a row.
   In run 19 the main thread's x87 precision was therefore **not** 24-bit when the settlement ran. Something
   in the test process changed it. Until the harness shows 24-bit precision on the main thread, run-19-style
   references are not valid for float-sensitive values. §7.2 and §15 test 10 now state the precision and
   this case. No logic change.

2. **Operation order of the tax products (spec corrected).** §5.2 and `GetTaxIncome(t)` (§6) wrote the
   products in a different order from the original, and the implementation followed the text. The original
   evaluates `float(land + value) × WaningWealth × OccWeight` and `float(assessed) × DifficultyTaxMultiplier ×
   rate × INV1200` from left to right. Each multiplication is rounded to `float32`, and the 64-bit integer
   enters exactly. A different order can change a truncated result by 1 (§15 test 2). This is a rare ±1
   effect. It does **not** explain the reported per-type differences (+15, −1 and −4).

3. **Tax by type: no spec defect found.** The following were re-checked against the original and found to
   agree with §2 and §5.2:
   - the tax-type table (all 36 cells);
   - purpose and wealth: the first two 32-bit fields of the building's per-type profile;
   - lot wealth: the lot's occupant wealth, which is the lot's dominant occupant wealth level, at most the
     building's;
   - the livable test, the lot size, and the land value at the lot's location;
   - the building value: property `0xE91A0B5F` from the occupant's property holder, a scalar sint64 in all
     shipped data.

   The plugin's lot visitor reads the same values through the same interface slots. The per-type differences
   therefore come from **different inputs at the moment `ComputeTax` runs**, not from the formula. Candidates,
   still open:
   - (a) The month-level agent order. The original simulator orders agents by object address, so the
     replacement budget can settle before or after other month-level agents that change lot occupancy or wealth.
     In run 19 the original budget sometimes settled after the observer's month line (2001-12 and 2002-01
     show last month's values), and ours always settled before it.
   - (b) A different x87 precision between the two runs, which would make the game's own simulators diverge.
   - (c) A lot-set difference between the master lot list and `GetLotsInCellRect`. *Excluded* by BQ1 (§5.2):
     the two sets are equal on every known path, and their order is equal for lots that existed at the load.

   A per-lot dump was requested to decide between them. The same runs also differ by 7 simoleons in non-tax
   monthly income (35 against 28: ordinance, neighbour deals and department line-item income), which is still
   unexplained.
