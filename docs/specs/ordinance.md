# Spec: the ordinance subsystem (`cISC4OrdinanceSimulator`, `cISC4Ordinance`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program and its data files. Implementers must work from
this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose. "The original returns no meaningful value" means the SDK declares a return value that the
original never sets; callers in the shipped game ignore it.

Related specs: [foundations.md](foundations.md) (streams §2, object references §2.7, message objects §5),
[simulator.md](simulator.md) (agents, calendar messages), [budget.md](budget.md) (books the ordinance totals),
[demand.md](demand.md) (consumes ordinance tax factors).

## 1. Role

Ordinances are city-wide policies the player can enact or repeal. The subsystem:

1. creates one **ordinance object** per ordinance exemplar shipped with the game (or added by plugins);
2. decides each month which ordinances are **available** (offered to the player);
3. keeps which ordinances are **on**, charges a one-time enactment or repeal amount, and computes each on
   ordinance's **monthly** income or cost;
4. gives the budget the month's ordinance income and expense totals;
5. answers other subsystems' questions about the combined **effect** of all on ordinances (a product of factors or
   a sum of offsets stored as exemplar properties);
6. persists all of this in the city save file.

| Class | CLSID | Interfaces (IID) |
|---|---|---|
| ordinance simulator | `0xA990C08E` | `cISC4OrdinanceSimulator` (`0x41193C3A`), `cIGZMessageTarget2` (`0x452294AA`), `cIGZSerializable` (`0xE4FDA3D4`), `cIGZUnknown` (`1`) |
| standard ordinance (`cSC4OrdinanceSimple`) and its 20 subclasses (§3.6) | one per ordinance (§3.6) | `cISC4Ordinance` (`0xAA5E2247`), `cIGZSerializable` (`0xE4FDA3D4`), `cIGZUnknown` (`1`), the class's own ID `0x8BD4FC74` (§11.2), and the property-holder interfaces of its "misc properties" holder |

The city obtains the simulator with `GZCOM()->GetClassObject(0xA990C08E, 0x41193C3A, &p)` and stores it in
reference slot 31 of its save record (foundations §3.6). Other subsystems reach it through
`cISC4City::GetOrdinanceSimulator()`.

## 2. Terms

- **Ordinance ID**: a 32-bit value; by convention equal to the class ID of the ordinance's class and to the
  instance ID of its exemplar.
- **Available**: offered to the player. **On flag**: the player enacted it. **IsOn** = available **and** on flag.
- **Enabled**: a master switch (set to true by `Init`); a disabled ordinance never becomes available (§3.4).
- **Adjusted monthly income** (`adjusted`): the ordinance's signed monthly amount for the current month.
  Positive = income, negative = cost.
- **Pending one-time amount** (`pending`): the simulator's sum of enactment and repeal amounts since the last
  month-end (signed; negative = cost).
- Money values are signed 64-bit Simoleons.

## 3. Ordinance objects (`cISC4Ordinance`)

### 3.1 State

| Field | Initial | Source property (§3.2) | Saved |
|---|---|---|---|
| initialized | false | | yes |
| key (type, group, instance) | 0 | the exemplar's resource key | via the property holder |
| id | 0 | `0x28EBBC30` | yes |
| name (string) | empty | §3.2 | yes |
| description (string) | empty | §3.2 | yes |
| yearFirstAvailable (u32) | 0 | `0x28EBBC35` | yes |
| chanceAvailability (float fraction) | 0 | `0x28EBBC36` | yes |
| enactmentIncome (s64) | 0 | `0x28EBBC37` | yes |
| retracmentIncome (s64) | 0 | `0x28EBBC38` | yes |
| monthlyConstantIncome (s64) | 0 | `0x28EBBC39` | yes |
| monthlyIncomeFactor (float) | 0 | `0x28EBBC3A` | yes |
| advisorID (u32) | 0 | `0x28EBBC3B` | yes |
| isIncomeOrdinance (bool) | false | `0x28EBBC3C` | yes |
| available, on, enabled (bool) | false | | yes |
| adjusted (s64) | 0 | | yes |
| misc properties | empty | the whole exemplar (§3.2) | yes |

### 3.2 Loading from the exemplar (`FromData(key)`)

1. Load the exemplar `key` through the resource manager as `cISCResExemplar` (IID `0xA52160F5`). If it cannot be
   loaded, return false and change nothing.
2. Store `key`. Make the exemplar the parent of the misc-properties holder, so that `GetMiscProperties()` answers
   every property of the exemplar *[inferred: parent link, not a copy]*.
3. Read the properties of §3.1. A missing property leaves the field unchanged.
4. **Name**: if property `0x6A416B21` (an LTEXT resource key, three `u32`) is present and its first word is
   non-zero, the name is that localized string. If the name is still empty, use the string property
   `0x28EBBC32`; if that is missing too, use `sprintf("0x%08x", id)`.
5. **Description**: the same with `0xCA416B2A`, then `0x28EBBC34`, then `"0x%08x"`.
6. Return true.

### 3.3 Monthly amount (`GetCurrentMonthlyIncome`)

The standard rule *[compat]*, computed in single precision and truncated toward zero to `s64`:

```
GetCurrentMonthlyIncome = trunc_s64( float(monthlyConstantIncome) + float(population) × monthlyIncomeFactor )
population := cISC4ResidentialSimulator::GetPopulation()
```

Subclasses replace the population with another base (§3.6). With the shipped data almost every ordinance has a
negative constant and a negative factor, so the result is a cost that grows with the city.

### 3.4 Availability test (`CheckConditions`)

Standard rule: true iff `enabled`, the simulator service exists, and the current simulation year
(`cISC4Simulator::GetSimDate`) is **≥ yearFirstAvailable**. `chanceAvailability` is **not** used here or anywhere
else in the shipped simulation *[inferred: no reader found]*. Subclasses add conditions (§3.6).

### 3.5 Methods (SDK declaration order)

| Method | Behaviour |
|---|---|
| `Init()` | If not initialized: set initialized, set **enabled := true**, then `FromData(key)`. Returns true. Subclasses first acquire the subsystems they use (§3.6). |
| `Shutdown()` | If initialized: clear initialized and **enabled**. Subclasses release their subsystem references. Returns true. |
| `GetCurrentMonthlyIncome()` | §3.3 / §3.6. Does not change state. |
| `GetID()` | `id`. |
| `GetName()` / `GetDescription()` | A pointer to the object's own string; **no AddRef** *[compat]*: callers must not release it. |
| `GetYearFirstAvailable()` | `yearFirstAvailable`. |
| `GetChanceAvailability()` | `chanceAvailability`. |
| `GetEnactmentIncome()` / `GetRetracmentIncome()` / `GetMonthlyConstantIncome()` / `GetMonthlyIncomeFactor()` | the stored values. |
| `GetAdvisorID()` | `advisorID`. |
| `GetMiscProperties()` | the misc-properties holder (§3.2), no AddRef. |
| `IsAvailable()` | `available`. |
| `IsOn()` | `available && on`. |
| `IsEnabled()` | `enabled`. |
| `GetMonthlyAdjustedIncome()` | `adjusted`. |
| `CheckConditions()` | §3.4 / §3.6. |
| `IsIncomeOrdinance()` | `isIncomeOrdinance`. |
| `Simulate()` | `adjusted := GetCurrentMonthlyIncome()`. Returns true. |
| `SetAvailable(b)` | `ForceAvailable(b)` (through the object's own method table), then `adjusted := 0`. Returns true. |
| `SetOn(b)` | If `b ≠ on`: when switching on, `adjusted := trunc_s64(|float(enactmentIncome)| + float(adjusted))`; when switching off, `adjusted := trunc_s64(float(adjusted) − |float(retracmentIncome)|)`; then `on := b`. Returns true. *[quirk]* This preview adjustment is overwritten by `Simulate` at the next month-end for on ordinances. |
| `SetEnabled(b)` | `enabled := b`. Returns true. |
| `ForceAvailable(b)` / `ForceOn(b)` | set the flag only. Return true. |
| `ForceMonthlyAdjustedIncome(v)` | `adjusted := v`. Returns true. |

Two further methods exist on the concrete class only (not in the SDK interface): `SetKey(key)` stores the key,
and `FromData(key)` (§3.2). See §11.2 for why they matter.

### 3.6 Ordinance classes

There is one GZCOM class per shipped ordinance. All derive their behaviour from the standard ordinance; the
"Overrides" column lists what differs. Class IDs *[compat]*:

| CLSID (= ID) | Class | Overrides |
|---|---|---|
| `0x8BD4FC74` | `cSC4OrdinanceSimple` (base class; no shipped exemplar uses it) | — |
| `0xA2BF1DDC` | CarpoolIncentive | — (the traffic simulator tests `IsOrdinanceOn(0xA2BF1DDC)` directly, §6.3) |
| `0xA2BF1E43` | CleanAir | — |
| `0x00D0723D` | CommunityCPRTraining | — |
| `0x40D07236` | FreeClinics | monthly base = full expenses of line item `0x03110000` of department `0x09188F42` (health), 0 if missing |
| `0x815B4CEF` | JuniorSportsLeague | monthly base = total expenses of department `0x8A4260A6`; condition adds "that department's total expenses ≠ 0" |
| `0x22F6E80C` | LandfillGasRecovery | monthly base = the **second** output of `cISC4LandfillDeveloper::GetLandfillUsage` (SDK name: capacity): the summed heap capacity of the connected landfill zones (landfill.md §4.6.5); condition adds "that value ≠ 0 and `cISC4PowerSimulator::GetPowerPlantCount(3) > 0`" |
| `0xA0D07129` | LegalizeGambling | `GetCurrentMonthlyIncome` = `monthlyConstantIncome` only (no factor); `SetOn(false)` side effects (below) |
| `0x62BF1DAA` | MandatorySmokeDetectors | — |
| `0x62BF1DB9` | NeighborhoodWatch | condition adds `cISC4PoliceSimulator::GetPoliceStationCount() ≠ 0` |
| `0xE0D0722E` | NuclearFreeZone | — |
| `0x22F6E81B` | PaperReductionAct | — |
| `0x82B9999B` | PowerConservation | — |
| `0xE0D07233` | ProReadingCampaign | monthly base = sum of full expenses of line items `0x03190000` and `0x0034A000` of department `0x09188F4C` (education); 0 if the department is missing |
| `0xA2BF1DE5` | ShuttleService | — |
| `0xC2F6E81F` | TireRecycling | — |
| `0x62F6E7CF` | TouristPromotion | monthly base = total expenses of department `0x8A416368` (0 if missing) |
| `0x42BF1E18` | TrashPresort | monthly base = total expenses of department `0x6A42A8F5` (sanitation); if the budget simulator is missing the result is `monthlyConstantIncome`; condition adds `*cISC4PollutionSimulator::GetRecyclingCenterEffect() × 100 > 0`, i.e. "recycling effect > 0". The getter (pollution compiled slot 33, `+0x84`) returns a **pointer to an `f32` fraction** (0.0 … about 1.0) in EAX, not the SDK's `int32` (pollution §8, §4.6.3); the value is computed by the pollution simulator each cycle, not read from tuning |
| `0xC2BF1E04` | VehicleEmissionStandard ("Car Smogging") | — |
| `0x02BF1DFA` | WaterConservation | — |
| `0xC2BF1DC5` | YouthCurfew | — |

"Monthly base" replaces `population` in §3.3: `trunc_s64(float(constant) + float(base) × factor)`. "Condition
adds" means the standard test of §3.4 **and** the extra condition. Department and line-item lookups go through
`cISC4BudgetSimulator::GetDepartmentBudget`, `cISC4DepartmentBudget::GetLineItem` /
`GetTotalExpenses` and `cISC4LineItem::GetFullExpenses` (see [budget.md](budget.md)).

**LegalizeGambling `SetOn(false)`** *[compat]*, after the standard `SetOn`:
1. collect every lot whose `GetBuildingType(false)` is `0x033A0000` (the casino) and demolish them all through the
   demolition utility in one call;
2. set the civic building simulator's conditional-building status of `0x033A0000` to **Hidden** (1), keeping its
   other fields;
3. if the 3D view's current input control has ID `0x88F154FB`, remove it.

*[quirk]* This runs on **every** `SetOn(false)`, including the internal off-then-on re-announcement of
§4.2 / §4.3.

### 3.7 Shipped ordinances (exemplars, group `0xA9C2C209`, `SimCity_1.dat`)

Every shipped ordinance has `0x28EBBC36` chance = 0.005 and advisor 0, and its exemplar type (`0x10`) is 14.

| ID | Exemplar name | Year | Enact | Repeal | Constant | Factor | Effect properties (§6) |
|---|---|---|---|---|---|---|---|
| `0x00D0723D` | CPR Training | 0 | −10 | −10 | −50 | −0.005 | `0xE91B3AEE`=110 |
| `0x02BF1DFA` | Water Conservation | 0 | −10 | −10 | −50 | −0.005 | `0xA8F4EB0C`=0.9; `0x2A654100`=0.9917, `0x2A654200`=0.995, `0x2A654300`=0.9967, `0x2A654400`=0.9983 |
| `0x22F6E80C` | Landfill Gas Recovery | 0 | −20 | −10 | −20 | −0.001 | `0x08F79B8E`=0.97 |
| `0x22F6E81B` | Paper Reduction | 0 | −20 | −10 | −20 | −0.005 | `0xE8F79C90`=0.9; `0x2A653320`=0.9967, `0x2A653330`=0.9983 |
| `0x40D07236` | Free Clinics | 0 | −10 | −10 | −50 | −0.005 | `0xE91B3AEE`=110 |
| `0x42BF1E18` | Trash Presorting | 0 | −20 | −10 | −20 | −0.003 | `0xE8F79C90`=0.97 |
| `0x62BF1DAA` | Smoke Detectors | 0 | −10 | −10 | 0 | −0.001 | `0x28F42AA0`=0.95 |
| `0x62BF1DB9` | Neighborhood Watch | 0 | −10 | −10 | −10 | −0.001 | `0x28ED0380`=0.95 |
| `0x62F6E7CF` | Tourist Promotion | 0 | −10 | −10 | −50 | −0.005 | `0x2A653110`, `0x2A653120`, `0x2A653130` = 1.0033 |
| `0x815B4CEF` | Junior Sports | 0 | −20 | −10 | −100 | −0.005 | `0x28ED0380`=0.95 |
| `0x82B9999B` | Power Conservation | **1970** | −20 | −10 | 0 | −0.005 | `0x0911E117`=0.9; `0x2A654200`, `0x2A654300`, `0x2A654400` = 0.995; `0xAA5B8407`=−10 (s32) |
| `0xA0D07129` | Gambling | 0 | 0 | −20 | +100 | 1 | `0x28EBBC3C`=true; `0x28ED0380`=1.2 |
| `0xA2BF1DDC` | Carpool Incentive | 0 | −10 | −10 | −50 | −0.01 | — |
| `0xA2BF1DE5` | Shuttle Service | 0 | −20 | −10 | −20 | −0.005 | `0x08F79B8E`=0.98; `0x8A612FEE` = s32[5,−5,0, 5,0,−5, 5,−5,0] |
| `0xA2BF1E43` | Clean Air | 0 | −20 | −10 | −20 | −0.005 | `0x08F79B8E`=0.9; `0x2A654200`=0.9967 |
| `0xC2BF1DC5` | Youth Curfew | 0 | −10 | −10 | −40 | −0.002 | `0x28ED0380`=0.95; `0xAA5B8407`=−10 |
| `0xC2BF1E04` | Car Smogging | 0 | −20 | −10 | −20 | −0.005 | `0x08F79B8E`=0.9; `0xAA5B8407`=−5 |
| `0xC2F6E81F` | Tire Recycling | 0 | −20 | −10 | −20 | −0.001 | `0xE8F79C90`=0.97 |
| `0xE0D0722E` | Nuclear Free Zone | 0 | −10 | −10 | −50 | −0.002 | `0x2A654100`=1.0033, `0x2A654400`=0.9983; `0xAA5B8407`=+5 |
| `0xE0D07233` | Pro-reading campaign | 0 | −10 | −10 | −50 | −0.005 | `0xA92D9D7A`=110 |

Enact/Repeal/Constant are stored as `s32` in the exemplars and widened to `s64`. Name and description come from
LTEXT keys `(0x2026960B, 0x6A231EAA, instance)` in properties `0x6A416B21` / `0xCA416B2A`.

## 4. The ordinance simulator

### 4.1 State

| State | Initial | Saved |
|---|---|---|
| initialized | false | no |
| loadedFromSave | false (set by `Read`) | no |
| ordinances: map ID → ordinance (strong references), iterated in **ascending key** order | empty | yes |
| monthlyExpense (s64, ≥ 0 by construction) | 0 | yes |
| monthlyIncome (s64) | 0 | yes |
| pending (s64, signed) | 0 | yes |

### 4.2 `cISC4OrdinanceSimulator` methods (SDK declaration order)

`find(id)` means a map lookup by key.

| Method | Behaviour |
|---|---|
| `Init()` | §8.1. |
| `Shutdown()` | §8.2. |
| `AddOrdinance(o)` | Null → false. `RemoveOrdinance(o)` first (replaces an entry with the same ID). Insert `o` under `o.GetID()` (AddRef). Then: if `o.IsAvailable()`: `o.SetAvailable(false)`; `SetOrdinanceAvailable(id, true)`. If now `o.IsOn()`: `o.SetOn(false)`; `SetOrdinanceOn(id, true)`. If `!o.IsAvailable()` and `o.CheckConditions()`: `SetOrdinanceAvailable(id, true)`. Returns true. *[quirk]* An ordinance added while available **and** on is enacted twice (two enactment charges and two OrdinanceUpdated messages), because `SetOrdinanceAvailable` already re-enacts it. |
| `RemoveOrdinance(o)` | Null or `find(o.GetID())` fails → false. Otherwise, for the stored entry: if `IsOn()` → `SetOrdinanceOn(id, false)` (charges the repeal amount); if `IsAvailable()` → `SetOrdinanceAvailable(id, false)`; erase the entry. Returns true. |
| `IsOrdinanceAvailable(id)` | found and `IsAvailable()`. |
| `IsOrdinanceOn(id)` | found and `IsOn()`. |
| `IsOrdinanceAvailableButOff(id)` | found, `IsAvailable()` and not `IsOn()`. |
| `SetOrdinanceAvailable(id, b)` | §4.3. |
| `GetOrdinanceAvailableCount()` | number of entries with `IsAvailable()`. |
| `GetOrdinanceAvailableArray(out, &n)` | Walk in key order; write the IDs of available ordinances while fewer than the caller's `n` have been written. Set `n :=` the number written. Return the total number available. |
| `SetOrdinanceOn(id, b)` | §4.4. |
| `GetOrdinanceOnCount()` / `GetOrdinanceOnArray(out, &n)` | the same as the "available" pair with `IsOn()`. |
| `IsOrdinanceEnabled(id)` | found and `IsEnabled()`. |
| `SetOrdinanceEnabled(id, b)` | if found: `o.SetEnabled(b)`. Returns whether it was found. |
| `GetOrdinanceIDArray(out, &n)` | Walk all entries in key order, writing IDs (non-null entries) up to the caller's `n`. Set `n :=` the number written. Return the number of entries. |
| `GetOrdinanceIDByName(name)` | `GetOrdinanceByName(name)` → its `GetID()`, else 0. |
| `GetOrdinanceByID(id)` | the entry or null; **no AddRef** *[compat]*. |
| `GetOrdinanceByName(name)` | The first entry in key order whose `GetName()` equals `name` ignoring ASCII case; null if none. No AddRef. |
| `GetOrdinanceMonthlyExpense()` | `monthlyExpense` (the last computed value, §5). |
| `GetOrdinanceMonthlyIncome()` | `monthlyIncome`. |
| `GetCumulativeOffset(prop, index)` | §6.1. |
| `GetCumulativeFactor(prop, index)` | §6.1. |

### 4.3 `SetOrdinanceAvailable(id, b)` *[compat]*

```
o := find(id);  if none: return false
if o.IsAvailable() ≠ b:
    if not b and o.IsOn(): SetOrdinanceOn(id, false)          # repeal (charges retraction)
    o.SetAvailable(b)                                          # also adjusted := 0
    post OrdinanceAvailabilityChanged 0xA2D1C5B9 (void1 = IGZUnknown = this simulator,
                                                  data2 = id, data3 = o.IsAvailable())
    if b and o.IsOn():                                         # on flag survived while unavailable
        o.SetOn(false);  SetOrdinanceOn(id, true)              # re-enact: charges enactment, posts 0xC2C09554
return true
```

### 4.4 `SetOrdinanceOn(id, b)` *[compat]*

```
o := find(id);  if none: return false
if o.IsOn() ≠ b:
    o.SetOn(b)
    pending += b ? o.GetEnactmentIncome() : o.GetRetracmentIncome()     # signed s64 sum
    post OrdinanceUpdated 0xC2C09554 (void1 = IGZUnknown = this simulator, data2 = id, data3 = o.IsOn())
return true
```

*[quirk]* For an **unavailable** ordinance `IsOn()` stays false, so `SetOrdinanceOn(id, true)` charges the
enactment amount and posts data3 = 0 every time it is called, and sets the on flag; when the ordinance later
becomes available it is enacted (and charged) again by §4.3.

## 5. Monthly behaviour

The simulator registers one agent at the **month** level (§8.1). Its `DoMessage`:

| Message | Action |
|---|---|
| SimNewMonth `0x66956816` | `DoMonthlySimulation` |
| SimBegin `0x66956810` | `SimulationBegin` (below). *[inferred]* Unreachable in the shipped game: the simulator only sends this type to level-0 agents. Implement it anyway. |
| anything else | ignored |

It always returns true (the agent always finishes in one call).

**DoMonthlySimulation** *[compat]*:

```
monthlyExpense := 0;  monthlyIncome := 0
if pending ≤ 0: monthlyExpense := −pending else: monthlyIncome := pending
pending := 0
for o in ordinances (ascending ID):
    if o.IsOn() and o.Simulate():
        a := o.GetMonthlyAdjustedIncome()
        if a ≤ 0: monthlyExpense −= a  else: monthlyIncome += a
    if not o.IsAvailable() and o.CheckConditions():   SetOrdinanceAvailable(o.id, true)
    elif o.IsAvailable() and not o.CheckConditions(): SetOrdinanceAvailable(o.id, false)
post OrdinanceMonthlyDone 0xC8EA5018 (void1 = IGZUnknown = this simulator)
```

- The availability check runs after the ordinance's own booking, so an ordinance that loses availability this
  month is still booked this month, and its repeal amount goes into `pending` for **next** month.
- **SimulationBegin** is the same booking pass without the `pending` step and without the closing message: it
  zeroes both totals, re-evaluates availability for each ordinance (first), then books every on ordinance's
  **current** `adjusted` value (it does not call `Simulate`).
- The budget reads `GetOrdinanceMonthlyIncome/Expense` in its own month agent. Whether it sees this month's or
  last month's totals depends on the order of month agents (simulator spec §4, budget spec §14 question 2).

## 6. Effects

### 6.1 Cumulative queries *[compat]*

Ordinance effects are exemplar properties of the ordinance (read through `GetMiscProperties()`), combined over
all ordinances whose `IsOn()` is true, in ascending ID order:

```
GetCumulativeFactor(prop, index) -> float:               GetCumulativeOffset(prop, index) -> int:
  f := 1.0                                                   s := 0
  for each on ordinance o having property prop:              for each on ordinance o having property prop:
     v := o's value of prop                                     v := o's value of prop
     Float32 scalar:          f *= v                            Uint32 or Sint32 scalar: s += v (as int32)
     Float32 array:           f *= (index < count ? v[index] : 1.0)
                                                                Uint32/Sint32 array: s += (index < count ? v[index] : 0)
     any other type:          f *= 1.0                          any other type: s += 0
  return f   (float32 products)                              return s
```

- The scalar cases ignore `index`.
- *[compat]* The SDK declares `GetCumulativeOffset` as returning `int64_t`. The original produces a 32-bit sum
  only; callers use the low 32 bits. *[choice]* Return the sum sign-extended to 64 bits.
- Property values are found through the holder's normal lookup (own properties, then the parent exemplar).

### 6.2 Who asks for which effect

| Property | Kind | Asked by | Shipped users (§3.7) |
|---|---|---|---|
| `0x2A650000 \| demandID` (e.g. `0x2A651010` … `0x2A654400`) | factor | demand simulator, monthly tax/economy modifiers (demand spec §8.1) | Water Conservation, Paper Reduction, Tourist Promotion, Power Conservation, Clean Air, Nuclear Free Zone |
| `0x0911E117` | factor | power simulator (tuning read and monthly): power consumption factor | Power Conservation 0.9 |
| `0xA8F4EB0C` | factor | plumbing simulator (`DoIdle` state 0): water consumption factor | Water Conservation 0.9 |
| `0x28ED0380` | factor | crime simulator: crime multiplier (re-read on OrdinanceUpdated) | Neighborhood Watch, Junior Sports, Youth Curfew 0.95; Gambling 1.2 |
| `0x28F42AA0` | factor | flammability simulator `ComputeGlobalMultiplier` | Smoke Detectors 0.95 |
| `0x08F79B8E`, `0xE8F79C8B`, `0xE8F79C90`; `0x8A67E378`; `0x8A67E373/74/76` (index = per-type row) | factor | pollution simulator daily pass (meanings belong to the pollution spec) | `0x08F79B8E`: Landfill Gas 0.97, Shuttle 0.98, Clean Air 0.9, Car Smogging 0.9; `0xE8F79C90`: Paper Reduction 0.9, Trash Presort 0.97, Tire Recycling 0.97 |
| `0xAA5B8407` | **offset** | aura simulator `ComputeGlobalEffect` (re-run on OrdinanceUpdated) | Power Conservation −10, Youth Curfew −10, Car Smogging −5, Nuclear Free Zone +5 |

### 6.3 Direct readers

These consumers do not use the cumulative queries:

- **Residential simulator** (`GetOrdinanceEffects`): enumerates `GetOrdinanceOnArray`, takes each ordinance by ID
  and reads health/education effect properties straight from `GetMiscProperties()`: `0xA91B3AF4`, `0xA92D9D7A`,
  `0x692EF65A`, `0x892D9D02` (percent values × 0.01), curves `0xA91B3AFA` and `0xC91B3B02`, and the related
  `0x491B3AD5`, `0x891B3AE6`, `0xE91B3AEE`, `0xE92D9DB4`, `0xC92D9C7A`, `0x092D909B`. Shipped users: CPR Training
  and Free Clinics (`0xE91B3AEE` = 110), Pro-reading (`0xA92D9D7A` = 110). Semantics belong to the residential spec.
- **Traffic simulator** (`UpdateTravelStrategies`, run on OrdinanceUpdated): adds the 9-value `s32` array
  `0x8A612FEE` of every on ordinance to the travel-strategy percentages (clamped 0..100); and multiplies car
  trips by 0.8 when `IsOrdinanceOn(0xA2BF1DDC)` (Carpool) is true (traffic spec).
- **Budget window, LUA extensions, advisors**: names, descriptions, availability, on state, enact/repeal and
  adjusted amounts through the interfaces of §3.5 / §4.2.

## 7. Messages

### 7.1 Posted (all with `MessagePost(msg, false)`)

| Type | When | Fields |
|---|---|---|
| `0xC2C09554` OrdinanceUpdated | `SetOrdinanceOn` changed `IsOn` | void1 = IGZUnknown = simulator, data2 = ordinance ID, data3 = `IsOn()` |
| `0xA2D1C5B9` OrdinanceAvailabilityChanged *[name inferred]* | `SetOrdinanceAvailable` changed availability | void1 = IGZUnknown = simulator, data2 = ID, data3 = `IsAvailable()` |
| `0xC8EA5018` OrdinanceMonthlyDone *[name inferred]* | end of `DoMonthlySimulation` | void1 = IGZUnknown = simulator |

Subscribers to `0xC2C09554` in the shipped game: traffic, crime, flammability, power and aura simulators and the
advisor base class. No subscriber of the other two was found.

### 7.2 Handled

Only the agent messages of §5. The simulator subscribes to nothing on the message server.

## 8. Lifecycle

### 8.1 `Init()` → true

If not initialized:
1. set initialized;
2. register the agent `(this as cIGZMessageTarget2, level 6 = Month, name "cSC4OrdinanceSimulator", flags 2)`
   with the simulator (simulator spec §4; flags 2 = runs only in an established city);
3. if **not** `loadedFromSave`: zero `monthlyExpense`, `monthlyIncome` and `pending`, then **load all standard
   ordinances** (below);
4. clear `loadedFromSave`.

**Loading standard ordinances** *[compat]*: enumerate every exemplar resource (type `0x6534284A`, any group,
any instance) known to the resource manager. For each one whose property `0x10` (exemplar type) is **14**:

```
o := GZCOM()->GetClassObject(key.instance, 0x8BD4FC74)     # the class is chosen by the INSTANCE id
if created: o.SetKey(key); o.Init(); insert o under o.GetID() unless that ID is already present
```

- An exemplar whose instance has no registered class produces no ordinance. Adding a new ordinance therefore
  needs a class (a DLL); an exemplar alone is not enough *[compat]*.
- Duplicate IDs: the first one inserted wins. The enumeration order of the resource manager is not specified
  (open question 2).
- Ordinances are **not** made available here; that happens at the first month-end (§5).
- Saved cities keep their own ordinance set (§9.1): standard ordinances added by later plugins do not appear in
  an existing city unless something calls `AddOrdinance` *[compat]*.

### 8.2 `Shutdown()` → true

If initialized: clear it and remove the agent from the simulator (`RemoveAgent(this)`). The ordinance map is
kept; ordinances are not shut down. They are released when the simulator is destroyed.

## 9. Persistence (`cIGZSerializable`)

### 9.1 Ordinance simulator (`GetGZCLSID() = 0xA990C08E`) *[compat]*

```
u16   version = 4
s64   monthlyExpense
s64   monthlyIncome
u32   count                      (number of map entries)
count × ref ordinance            (foundations §2.7; in ascending key order)
s64   pending
```

**Read**: fail (return false) unless `version == 4`. Read the two totals and the count, **clear the map**, then
for each reference: resolve it as `cIGZSerializable` (`0xE4FDA3D4`); if that succeeds and the object answers
`cISC4Ordinance` (`0xAA5E2247`), store it under the key **`GetGZCLSID()` of the object** (not `GetID()`).
Unlike creation (`insert_unique`, the first entry wins), a later duplicate key here replaces the earlier entry.
Then read `pending`, and set `loadedFromSave`.

*Why the class ID* *[compat]*: the class store reads classes in ascending class-ID order (foundations §2.7,
§3.5). When this `Read` runs (class `0xA990C08E`), the ordinance objects with higher class IDs (shipped:
`0xC2BF1DC5`, `0xC2BF1E04`, `0xC2F6E81F`, `0xE0D0722E`, `0xE0D07233`) exist but have not been read yet, so their
`GetID()` would still be 0. `GetID()` cannot be used inside `Read`.

**Keys after a load** *[quirk]*: the original **never re-keys**. `Init` (which runs after the whole class store has
been read, foundations §3.7) leaves existing entries alone; it only adds missing standard ordinances when the city
was not loaded. Entries keep class-ID keys for the life of the city. For every shipped class the class ID equals
the ordinance ID, so nothing is visible. A plugin ordinance whose class ID differs from its ordinance ID cannot be
found by `IsOrdinanceOn(id)` / `RemoveOrdinance(id)` after a load, and `AddOrdinance` would add a second entry.
*[choice]* FreeCity4 keys by class ID in `Read` (as the original) and **re-keys by `GetID()` in `Init`**, when every
ordinance has been read. This is a documented difference from the original; when two entries end with the same
ID, the one first in class-ID order wins (as creation would keep the first).

**Who else holds a loaded ordinance** *[compat]* (SV2). An ordinance object that the load created (shipped or
plugin class alike) has exactly two holders in the original:

1. The simulator's map: +1 from the typed reference read (QI to `cISC4Ordinance`, foundations §2.7).
2. The **object table of the city segment's serializer** (foundations §2.7, "References the load keeps"): +1 until
   the next save, or, if the city is never saved, until the segment is closed at city close, which comes after
   the city's `Shutdown` and after the city is released.

Nothing else in the exe holds one:

- The only `QueryInterface` for `cISC4Ordinance` (`0xAA5E2247`) outside the ordinance class's own
  (`0x0080C924`) is this `Read` (`0x0080E96B`). No other class reads an ordinance reference from the save.
- The messages of §7.1 carry the simulator, not the ordinance.
- `GetOrdinanceByID` and `GetOrdinanceByName` do no AddRef, and every caller in the exe uses the pointer at once,
  without an AddRef and without storing it: the LUA extensions (`0x0040B3B1`, `0x0040B4F1`), the residential
  simulator (`0x006FE27E`, `0x0071192B`, `0x00767458`) and the budget window (`0x0077AE7E`, `0x0077AE9E`,
  `0x0077AED9`, `0x0077C70E`, `0x0077C726`).

So the reference that the simulator takes in `Read` is its own, and it releases it when it is destroyed (§8.2).
In a loaded city that has been saved, that release is the last one and destroys the ordinance. In a loaded city
that has not been saved, the ordinance outlives the simulator until the segment closes, and the table's release
destroys it. Nothing can observe the difference: no code reaches the ordinance after the simulator is gone. A
plugin DLL that keeps a pointer from `GetOrdinanceByID` without an AddRef is outside this contract in the
original too.

### 9.2 Ordinance (`GetGZCLSID()` = the class ID of §3.6) *[compat]*

```
u16   version = 4
u8    initialized
u32   id
str   name                          (foundations §2.3)
str   description
u32   yearFirstAvailable
f32   chanceAvailability            (fraction)
s64   enactmentIncome
s64   retracmentIncome
s64   monthlyConstantIncome
f32   monthlyIncomeFactor
u32   advisorID
u8    available
u8    on
u8    enabled
s64   adjusted
u8    isIncomeOrdinance
...   exemplar property holder sub-record (§9.3)
```

**Read** accepts any version:
- `version < 4`: the chance is stored as a percentage (`f32`) and is multiplied by 0.01.
- `version < 2`: there is no `isIncomeOrdinance` byte; it is set to `id == 0xA0D07129` (Gambling).
- After the holder sub-record: if the holder has `0x6A416B21` (resp. `0xCA416B2A`) and the LTEXT string is not
  empty, it replaces the name (resp. description). Then the key is taken from the holder's parent exemplar (if
  any) and **`FromData(key)` is run again**: when the exemplar still exists, every exemplar-derived field of §3.1
  is refreshed from the current data files; the saved values are used only when it is missing *[compat]*.
  `initialized`, `available`, `on`, `enabled` and `adjusted` always come from the save.
- `Init` is **not** called on loaded ordinances (their `initialized` byte is restored).

### 9.3 Exemplar property holder sub-record

`u16 version = 2`, then the plain property-holder record (`u16 version = 2`, `u32 count`, `count ×
{u32 propertyID, property value record}`), then the parent exemplar's key as `u32 group, u32 type, u32 instance`
(all zero without a parent). When writing, if the parent exemplar has a non-zero property `0x6A871B82`, the
original also stores that exemplar as a resource in the save package *[inferred]*. The byte format of a property
value record is shared with other classes and is not yet specified (open question 1).

## 10. Interactions and replaceability

### 10.1 Calls into this subsystem

| Caller | Uses |
|---|---|
| City | creates the simulator through GZCOM, `Init`/`Shutdown`, saves and loads it, `GetOrdinanceSimulator()`. |
| Simulator | delivers SimNewMonth (§5). |
| Budget simulator | `GetOrdinanceMonthlyIncome`, `GetOrdinanceMonthlyExpense` (monthly settlement and live totals). |
| Demand, power, plumbing, crime, flammability, pollution simulators | `GetCumulativeFactor` (§6.2). |
| Aura simulator | `GetCumulativeOffset(0xAA5B8407, 0)`. |
| Residential simulator | `GetOrdinanceOnCount/OnArray`, `GetOrdinanceByID`, `cISC4Ordinance::GetMiscProperties` (§6.3). |
| Traffic simulator | `IsOrdinanceOn(0xA2BF1DDC)`, the on ordinances' `0x8A612FEE` (§6.3). |
| Budget window (`cSC4SubBudgetWindow`), budget UI, LUA `ordinance_*` functions, advisors | list, query, `SetOrdinanceOn`, names and amounts through §3.5 / §4.2. |
| DLL plugins (gzcom-dll) | `AddOrdinance` / `RemoveOrdinance` with their own `cISC4Ordinance` implementations. |

### 10.2 Replaceability findings

From the caller data of the Windows build (direct calls) and the call-site survey:

- **Ordinance simulator**: created only through GZCOM (`GetClassObject(0xA990C08E, 0x41193C3A)` in the city's
  `Init`; the class store on load). No original code calls its methods directly or depends on its layout; the
  only reference outside the class is the COM director's class table. **Replaceable** through the class registry.
- **Ordinance objects**: created through GZCOM only (§8.1 by instance ID; the class store on load by class ID).
  Other code uses only `cISC4Ordinance` (and the property-holder interfaces of `GetMiscProperties()`).
  **However**, the original simulator's loader requests IID `0x8BD4FC74` and then calls `SetKey` through the
  **concrete** method table of `cSC4OrdinanceSimple` (Windows build: slot at byte offset `+0x78`; `FromData` is at
  `+0x7C`) *[compat]*. Consequences:
  - Replacing the simulator and the ordinance classes **together** is safe; this is the recommended path.
  - Replacing ordinance classes while the original simulator runs would require mimicking the private method
    table; FreeCity4 must not do this.
  - A FreeCity4 simulator must accept foreign `cISC4Ordinance` objects (DLL plugins, or original classes when
    not replaced) and use only the interface methods.
- No original code constructs ordinance objects with `new` outside the class factories.

## 11. Original quirks (reproduced)

1. Double enactment in `AddOrdinance` (§4.2); enacting an unavailable ordinance charges every call (§4.4).
2. `SetOn` preview adjustment of `adjusted` (§3.5).
3. Gambling's casino demolition on every `SetOn(false)` (§3.6).
4. Load keys the map by class ID (§9.1).
5. `chanceAvailability` is loaded and saved but never used (§3.4).
6. `GetCumulativeOffset` is 32-bit (§6.1).

## 12. Open questions

1. Byte format of the property value record inside the property-holder sub-record (§9.3). Needed to write saves
   the original can read; shared with every class that saves a property holder.
2. The enumeration order of exemplar keys at load (§8.1), which only matters for duplicate ordinance IDs.
3. *(resolved)* Float behaviour of §3.3: the game runs the x87 at 24-bit precision (foundations §18), so each
   operation rounds like a `float32` operation before the truncation (no extended precision). Strict `float32`
   arithmetic matches, except where an operand is a `double` constant or an integer above 2^24 (foundations
   §18.3); the ±1 tolerance can be removed for tests whose operands avoid that case.
4. Who calls `SetOrdinanceEnabled` / `cISC4Ordinance::SetEnabled` in the shipped game (tutorials, scenarios?).
5. The meaning of the pollution effect properties (`0x08F79B8E`, `0xE8F79C8B`, `0xE8F79C90`, `0x8A67E37x`) and of the
   residential effect properties; they belong to those subsystems' specs.
6. Whether anything listens for `0xA2D1C5B9` and `0xC8EA5018` (UI only?).
7. The meaning of `GetLandfillUsage`'s second output used by Landfill Gas Recovery.

## 13. Conformance tests (must exist in `engine/conformance`)

1. **Loading**: with stub exemplars (type 14 and type 5) and registered classes for some instances only, exactly
   the type-14 exemplars with a registered class are created, keyed by ID, `Init`ed (enabled = true), and
   **unavailable**; names fall back from LTEXT to `0x28EBBC32` to `"0x%08x"`.
2. **Monthly amount**: constant −50, factor −0.005, population 20,000 → −150; Gambling (constant 100, factor 1)
   → 100 regardless of population; FreeClinics with a health line item of full expenses 4,000 → −50 − 20 = −70;
   TrashPresort without a budget simulator → constant.
3. **Availability**: Power Conservation becomes available at the first month-end in year ≥ 1970 and not before;
   NeighborhoodWatch only with a police station; a disabled ordinance never becomes available; losing
   availability while on posts OrdinanceUpdated (off) then OrdinanceAvailabilityChanged and charges the repeal
   amount into `pending`.
4. **Enact/repeal accounting**: enacting (enact −20) then month-end gives expense 20 + the ordinance's monthly
   cost; repealing (repeal −10) adds 10 to the next month's expense; an income ordinance (+100) adds to income;
   `pending > 0` is booked as income.
5. **Messages**: `SetOrdinanceOn` posts `0xC2C09554` only on change with data2 = ID and data3 = new `IsOn`;
   month-end posts `0xC8EA5018` once.
6. **Quirks**: `AddOrdinance` of an available+on ordinance charges enactment twice; `SetOrdinanceOn(id, true)` on
   an unavailable ordinance charges every call and reports data3 = 0; Gambling `SetOn(false)` demolishes all
   lots of building type `0x033A0000` and hides that conditional building.
7. **Cumulative queries**: with on ordinances holding `0x28ED0380` = 0.95 and 1.2 → factor 1.14 (float32); an off
   ordinance is ignored; a Float32 array with index ≥ count contributes 1.0; offset `0xAA5B8407` with −10, −5,
   +5 → −10; a Sint32 array with index ≥ count contributes 0; a wrong type contributes the neutral value.
8. **Array queries**: `GetOrdinanceOnArray` with a buffer of 2 and 3 on ordinances writes 2 IDs (ascending), sets
   n = 2 and returns 3; `GetOrdinanceByName` is case-insensitive.
9. **Save**: write → read → write is byte-identical for the simulator and for an ordinance record; simulator
   version 3 and 5 are rejected; an ordinance record of version 3 reads the chance as a percentage; version 1
   derives `isIncomeOrdinance` from the ID; after reading, exemplar-derived fields come from the current exemplar
   when it exists and from the record otherwise; `Init` after a load does not reload standard ordinances.

## Compiled slot tables (round 2) *[compat]* (see foundations §16, §17)

Verified slot by slot against the original Windows vtables (`ret` operand = stack-argument bytes, SDK header
compiled under the MSVC overload rule):

| Interface | Original vtable | Interface slots | Result |
|---|---|---|---|
| `cISC4OrdinanceSimulator` | `0x00ABEAC0` | 26 (3 + 23) | no overloads; order and argument sizes match the SDK |
| `cISC4Ordinance` | `0x00ABDD38` (Free Clinics; all ordinance classes share the layout) | 30 (3 + 27) | no overloads; order and argument sizes match the SDK |

The SDK declaration order can be used as is. Ordinance code also calls the property-holder interface of its
"misc properties". That interface has overloads and several SDK signature errors (foundations §17.2).
