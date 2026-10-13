# Spec: the residential simulator (`cISC4ResidentialSimulator`)

Status: **implementation spec**, clean-room input, round 3. This document describes *behaviour*, not code. It was
written by the analysis team from the original program (Windows build 1.1.641) and its data files. The earlier
write-up in `docs/subsystems/demand-rci.md` §5 (partly from the Mac build) was the starting point. Every fact below
was checked against the Windows executable; corrections to the earlier write-ups are listed in §14.
Implementers must work from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must
**not** consult decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* consistent with all evidence
but not proven; *[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must
survive).

**Scope.** The class has two halves. This spec covers the **population half** and the whole class frame
(identity, lifecycle, the daily state machine, tuning, persistence, every interface slot, dependencies and
replaceability). The **education and health half** (school and hospital building lists, coverage maps, grading,
strikes, ordinance effects, auto-budget, query data, system ratings, the EQ/HQ blend and decay formulas,
`GetCellEQ/HQ`, global EQ/HQ, min/max searches and the meaning of the department blocks) is specified in
[residential-education-health.md](residential-education-health.md), cited below by section title.

Related specs, cited and not repeated: [foundations.md](foundations.md) (streams §2, references §2.7, class store
§3, city component table §3.6, Init order §3.7, response curves §4 and the four missing-property patterns §4.1,
messages §5, random generators §6, month order §13, slot-order rule §16, framework slot tables §17, **x87
precision §18**); [simulator.md](simulator.md) (agents §4, levels §2.2, finished semantics §5.3, CPU-cycle timer
§5.5, `GetSimDate` slot order §5.6); [simgrid.md](simgrid.md) (grid classes §2, grid slots §4, grid persistence
§5, raw access §6.3, **grid ownership rule §6.4**, `ProcessTwoGridsWithCallback` §7.3, scanning iterator §7.4);
[pollution.md](pollution.md) (`GetPollutionValue`, slot 6); [budget.md](budget.md) (departments);
[aura.md](aura.md), [crime.md](crime.md), [demand.md](demand.md) (consumers).

---

## 1. Role and identity

The residential simulator keeps, per **population tract** (normally 4 × 4 cells), the residents of each wealth
level (R$, R$$, R$$$), their education quotient (EQ), health quotient (HQ) and a single **average age**. Once per
month it:

- rebuilds three **proximity maps**: for each wealth level, the Manhattan distance (in cells, capped at 255) from
  every cell to the nearest residential building of that wealth (§5.3);
- copies the residential population from the growth developers' population maps, decays EQ and HQ (HQ also by
  pollution), blends in newcomers, ages the tract and applies the **generational reset** when the average age
  exceeds the tract's life expectancy; it may spawn a **buzzards** effect over sick tracts (§5.5);
- builds an age histogram and publishes the population, life expectancy, age-group and coverage history (§5.6,
  §5.7, §8).

It does **not** compute population: the population comes from the tract and lot developers through the growth
developers' population maps.

| Item | Value |
|---|---|
| Class ID (`GetGZCLSID`, `cIGZSerializable`) | `0x4990C013` |
| City component | slot **36** (foundations §3.6); `cISC4City::GetResidentialSimulator()` = city slot 68 (`+0x110`) |
| Global service pointer | `SL::spResidentialSim` (`0x00B43DA4`), set at city init |
| Factory | allocates the object and constructs it (refcount 0) |
| Init order | after pollution, before weather (foundations §3.7) |

Interfaces (`QueryInterface`, primary vtable slot 0):

| IID | Interface | Sub-object |
|---|---|---|
| `0x00000001` (`cIGZUnknown`), `0x077AC1EE` | `cISC4ResidentialSimulator` | primary |
| `0x452294AA` | `cIGZMessageTarget2` | second |
| `0xE4FDA3D4` | `cIGZSerializable` | third |

- `QueryInterface(iid, out)`: for a known IID, writes the sub-object pointer, `AddRef`s and returns true; otherwise
  returns false and leaves `*out` untouched. *[quirk]* (irrelevant in practice) for `0x452294AA` the cast is
  null-checked: a null object would write 0 and still `AddRef`.
- `AddRef`: increments the count and returns it.
- `Release`: if the count is > 1, decrements it and returns the new count; otherwise calls its own `Shutdown`
  (slot 4), destroys the object and returns 0.

## 2. Grids

All grids are game `cISC4SimGrid<T>` objects (simgrid §2). The original constructs them directly at `Init`
(construct, `AddRef`, `Init`, `SetTractSize`, `SetInstanceID`). *[choice]* FreeCity4 creates them through GZCOM
per simgrid §6.4 with the same element type, tract size and instance ID.

| Grid | Type (class) | Tract size | Instance ID |
|---|---|---|---|
| proximity R$, R$$, R$$$ | `uint8` (`0x49B9E602`) | proximity size = **1** | `0x49D5BC96`, `0x49D5BCA6`, `0x49D5BCB7` |
| population R$, R$$, R$$$ | `uint16` (`0x49B9E604`) | population size P | `0x69D5C2A0`, `0x69D5C2A1`, `0x69D5C2A2` |
| average age | `uint8` | P | `0x69D5C303` |
| school coverage (7 grids) | `uint8` | P | `0xAA8E1DC1`, `0x8A8DF4B6`, `0xCA8E449E`, `0xAA8DF4BD`, `0x8A8DF4C4`, `0xAA8DF4D4`, `0x69D5C30D` |
| hospital coverage (3 grids) | `uint8` | P | `0xAA8E1DE4`, `0x8A8DF6FC`, `0x69D5C317` |
| school grade map, hospital grade map | `float` (`0x49B9E60A`) | P | `0x69D5C324`, `0x69D5C330` |
| EQ R$, R$$, R$$$ | `float` | EQ size E | `0x69D5C2D0`, `0x69D5C2D1`, `0x69D5C2D2` |
| HQ R$, R$$, R$$$ | `float` | E | `0x69D5C2E0`, `0x69D5C2E1`, `0x69D5C2E2` |
| temporary proximity work grid | `uint8` (constructed with `0x49B9E600`) | proximity size | none; exists only during states 13 → 3 (§5.3), not saved |

- **P and E** *[compat]*: on a **new** city `Init` sets both to `GetTractSize()` of the R$ growth developer's
  population map (developer `0x1010`, growth-developer slot 15 `GetPopulationMap`). The shipped developer
  exemplars give **4** (property `0x87CD6310` of developers `0x1010`, `0x1020`, `0x1030`). On a loaded city both
  come from the save (§9) and the grids keep their saved geometry. The coverage, grade and age grids use P; the
  EQ and HQ grids use E.
- The population, EQ, HQ and age grids are indexed by the same tract coordinates (x, z); the code assumes P = E.
- Grid contents are the model: population grid w at tract (x, z) is `pop_w`, and so on. Coverage and grade grids
  belong to the other spec.

## 3. State

| Field | Initial (constructor / new-city `Init`) | Persisted | Meaning |
|---|---|---|---|
| initialised | false | no | `Init` completed; cleared by `Shutdown` |
| cityStarted | false | no (`Read` sets it) | the new-city part of `Init` has run (§4.1 step 6) |
| services | null | no | city, building development simulator (BDS), occupant manager, growth developers R$/R$$/R$$$ (`0x1010/0x1020/0x1030`), demand node `0x1000`, message server, budget simulator, departments |
| proximity size, P, E | 1, 4, 4 (P and E then reset at new-city `Init`) | yes | §2 |
| the grids of §2 | created at `Init` if absent | **ref** | §2 |
| state (u32) | 0 | yes | daily state machine (§5.1) |
| dirty flag (u8) | 1 | yes (written as `dirty ∨ state ≠ 10`) | set by funding and building changes and by an overrunning month; never read *[inferred]* |
| population (s32) | 0 (new city) | yes | Σ population of the last completed pass; `GetPopulation` |
| running population (s32) | 0 (new city) | yes (version ≥ 4) | sum being built by the current pass (§5.5) |
| leSum (u64) | 0 | yes, but **reset by every `Init`** | Σ LE × tract population of the current pass |
| ageCount[20], ageEqSum[20] (int32) | **not initialised** *[inferred]* | no | age histogram (§5.4–§5.6) |
| school covered, hospital covered (s32) | — | no | counters filled by the coverage states (other spec); used by state 9 |
| two unused floats | uninitialised | yes | never written except by `Read`; written to the save as they are |
| scanning iterator | rectangle full int range, step 1 | yes, but **reset by every `Init`** | tract scan of state 4 (simgrid §7.4) |
| tuningLoaded (u8) | — | no (`Read` clears it) | population tuning loaded for this pass (§7.1) |
| population tuning T (§7.1) | partly uninitialised (see §7.1) | no | |
| simulator tuning (§7.2), department blocks | §7.2 | department blocks: yes | the blocks' scalars are **overwritten** at the next `Init` (§9.3) |
| ordinance-effect lists (health, education) | empty | no | other spec, ordinance effects |
| occupant lists (pending and per-type school and hospital lists, one other list) | empty | yes | other spec, building lists |
| school / hospital global auto-budget (u8) | 0 | yes (version ≥ 7) | slots 52–55 |
| random generator: one `cRZRandom` | seed `0xFFFFFFFF` (wall clock) at construction | no | buzzards (§5.5) and the monthly strike rolls (education and health spec, Strikes): one stream shared by both |

## 4. Lifecycle

### 4.1 `Init()` → AL

1. Acquire and `AddRef`, each only if not already held: the city, the BDS, the occupant manager; the three growth
   developers through `BDS.GetDeveloper(0x1010 / 0x1020 / 0x1030)`; the demand node `GetDemand(0x1000, 0)` from the
   demand simulator. **If the demand node is null, return false.** The node is only held (released in
   `Shutdown`); nothing reads it.
2. If no message server is held, take it (return **false** if there is none) and subscribe the message target to
   `0x99EF1142` and `0x99EF1143` (occupant inserted / removed).
3. Take the budget simulator (`city.GetBudgetSimulator()`) if not held.
4. Take the simulator; if there is none, return **false**. Register the agent (§4.3).
5. History warehouse: `SetHistoryDataQueueLength(id, 2)` for `0xCA545CAB + g` and `0xCA545CCA + g`
   (g = 0..8, interleaved per g), then `0x4A8A3FE2` and `0x4A8A3FE5`.
6. **New city only** (cityStarted false): P := E := R$ developer's population-map tract size; population := 0;
   running population := 0; state := 0; cityStarted := true.
7. If not initialised: read the simulator tuning (§7.2), then build the two ordinance-effect lists (other spec,
   ordinance effects).
8. Create every absent grid of §2 (for the coverage vectors: if the first entry is null, every entry is
   created).
9. Create the departments if absent (health `0x09188F42` and `0xAA538CB3`, education `0x09188F4C` and
   `0x4A538CC6`, group `0x6A357B7F`; other spec and budget.md).
10. **Always**: set the iterator rectangle to the population R$ grid's tract range `[0, countX − 1] × [0,
    countZ − 1]` (an empty range widened to one tract) and `Reset(0, 0)`; read the population tuning (§7.1);
    leSum := 0; initialised := true; return **true**.

*[compat]* Consequences of step 10 on a loaded city: the saved iterator position and leSum are discarded, and the
population tuning is read once at `Init` and again at the next state 4.

### 4.2 `Shutdown()` → AL = 1

If initialised: initialised := false; `RemoveAgent(target)` on the simulator; unsubscribe both messages; release
the message server, the budget simulator and the four departments; for every grid `Shutdown`, `Release` and
forget it; release the demand node, the temporary grid (if any), the city, the BDS, the occupant manager and the
developers; hand every occupant list to the occupant manager (slot 48, `+0xC0`, *[inferred]*: releases the list's
references) and clear it. Returns true in every case.

### 4.3 Agent registration and cadence *[compat]*

`AddAgent(messageTarget, L, "cSC4ResidentialSimulator", flags 2)` for L = **0, 10, 6, 4**, in that order.

| Message (level) | Action | Result |
|---|---|---|
| `0x66956814` SimNewDay (4) | `DoDailySimulation(5 ms)`: one step of §5 | the step's "finished" value |
| `0x66956816` SimNewMonth (6) | if state = 10: state := 0 and dirty := 0; else dirty := 1 (the running cycle continues) | true |
| `0x66956810` SimBegin (0), `0x6695681A` SimIdle (10) | nothing | true (the level-0 agent is then removed) |
| `0x99EF1142` / `0x99EF1143` | if data3 = `0x278128A0`: query void1 for `cISC4BuildingOccupant` (`0x87DDA3A9`); if it is one, `DoBuildingChanged(building, inserted = (type = 0x99EF1142))` (other spec, building lists) | true |
| anything else | nothing | true |

**Finished semantics**: a NewDay step returns false only when a time-sliced state (1, 2, 3, 4) runs out of time.
The simulator then delivers the same NewDay again on later frames, and the day cannot advance until the step
completes (simulator §5.3).

**Monthly timing** *[inferred from the scheduler]*: the cycle has 16 states (§5.1) and normally completes one
state per day. On the 1st of a month the day agent runs before the month agent (foundations §13), so the NewDay
of the 1st finds state 10 (nothing to do) and NewMonth then restarts the cycle at state 0. State 0 runs on the
2nd, and state 9 (history) on the **16th**, if no state overruns. If the cycle has not reached state 10 by the
next NewMonth, it simply continues; that month's cycle starts only at the following NewMonth.

**Ordering within a day** *[inferred]*: the pass reads the growth developers' population maps, the pollution
source grids and the BDS building map as they are when its day step runs; the order of the residential day agent
relative to the developers, pollution and the BDS follows pointer order in the original and was not analysed.
*[choice]* FreeCity4 uses registration order (simulator §4).

## 5. The monthly cycle (`DoDailySimulation`) *[compat]*

### 5.1 States

One call performs at most one state. Order: **0 → 13 → 1 → 14 → 2 → 15 → 3 → 11 → 4 → 12 → 5 → 6 → 7 → 8 → 9 →
10**.

| State | Work | Next |
|---|---|---|
| 0 | strike checks, ordinance-effect refresh, pending building lists (other spec) | 13 |
| 13 | create the temporary grid; seed it for R$ (§5.3) | 1 |
| 1 | distance passes; copy into proximity R$ (time-sliced) | 14 |
| 14 | seed for R$$ (reusing the temporary grid) | 2 |
| 2 | distance passes; copy into proximity R$$ (time-sliced) | 15 |
| 15 | seed for R$$$ **and** run the full transform | 3 |
| 3 | distance passes (none left to do); copy into proximity R$$$; `Shutdown` and release the temporary grid (time-sliced) | 11 |
| 11 | histogram reset (§5.4); tuningLoaded := false | 4 |
| 4 | `DoPopulationChange` for every tract (§5.5) (time-sliced) | 12 |
| 12 | iterator `Reset(0, 0)`; `PostPopulationChange` (§5.6) | 5 |
| 5–8 | school coverage (5), school grading (6), hospital coverage (7), hospital grading (8); state 5 first zeroes the school-covered counter, state 7 the hospital-covered counter (other spec) | next |
| 9 | history (§5.7) | 10 |
| 10 | nothing (waits for NewMonth) | — |

### 5.2 Time slicing

States 1–4 loop until their work ends or the CPU-cycle budget of 5 ms runs out (simulator §5.5); each loop has its
own adaptive check stride (as in landvalue §6). When time runs out the call returns **false** and the state stays;
the temporary grid (states 1–3) or the iterator (state 4) keeps its position. *[choice]* FreeCity4 may use any
deterministic slicing, provided each call completes at most one state and the results equal those of an
uninterrupted pass. Consumers can observe a partially updated population/EQ/HQ/age grid while state 4 runs.

### 5.3 Proximity maps (states 13–15, 1–3)

For wealth w ∈ {1, 2, 3} (R$, R$$, R$$$):

1. **Seed**. key := BDS `GetBuildingMapKey(purpose 1, w)` (slot 42) = `(purpose << 2) + w + 1` = **6, 7, 8**. map :=
   BDS `GetBuildingMap()` (slot 41; a `uint8` grid of building keys). The temporary grid (tract size = proximity
   size) is filled per tract t:
   - equal tract shifts: t := 0 if `map[t] = key`, else 255;
   - temporary grid finer than the map (d = shift difference): t := 0 if `map[x >> d][z >> d] = key`, else 255;
   - temporary grid coarser: t := 0 if **any** map tract it covers equals key, else 255.
2. **Distance passes**. One pass over the `countX × countZ` temporary grid:
   - for each x (row): sweep z = 0 → countZ − 1 with a running d starting at 255: `d := min(d + 1, v)`; store d;
     then the same sweep from z = countZ − 1 down to 0;
   - then for each z (column): the same forward and backward sweeps along x;
   - the pass reports "changed" if any stored value differs from the value it replaced.

   Passes repeat until one reports no change. The result is the **Manhattan distance** (in proximity tracts =
   cells) to the nearest key tract, **capped at 255**; with no key tract every value is 255.

   *[compat]* (RS5, from the exe; one pass is `0x005E4E80`) **The running `d` is a 32-bit register** (`EDX`). It
   is set to 255 at the start of every sweep, and the step `d + 1` is a 32-bit `inc`, so it cannot wrap. The
   minimum is a signed 32-bit compare against the zero-extended byte `v`, and the store writes the low byte of `d`.
   After `d := min(d + 1, v)` with `v ≤ 255`, `d` is never above 255, so the store never truncates. A
   reimplementation that widens `d` and clamps to 0..255 gives the same results; the clamp never acts. "Changed"
   is the OR of `old XOR new` over the pass.

   Example: one row seeded `[255, 0, 255, 255]`.
   - The forward sweep gives `[255, 0, 1, 2]`. The first step is `min(256, 255) = 255`.
   - The backward sweep then gives `[1, 0, 1, 2]`.

   A byte-wide `d` would wrap `255 + 1` to 0 and store 0 in the first cell, which is wrong.
3. **Copy** into proximity grid w with `ProcessTwoGridsWithCallback<uint8, uint8>` (simgrid §7.3; equal shifts, so a
   plain copy).

States 13 and 14 seed only; states 1 and 2 run the passes (time-sliced, one pass per loop iteration) and copy.
State 15 seeds R$$$ and runs the full transform at once (not time-sliced); state 3 then runs one pass (no change),
copies and frees the temporary grid. States 13, 14 and 15 create the temporary grid only if it does not exist,
so R$$ and R$$$ reuse the R$ grid (the seed overwrites every tract).

### 5.4 Histogram reset (state 11)

ageCount[b] := 0 and ageEqSum[b] := 0 for b = 0..19; leSum := 0; running population := 0.

### 5.5 `DoPopulationChange(T, x, z)` (state 4, per tract)

State 4 first reads the population tuning (§7.1) if tuningLoaded is false. Then for every tract (x, z) from the
scanning iterator (x inner, z outer; simgrid §7.4), with T = the tuning and `S = P` (population tract size). All
float steps are float32 operations in the order given (foundations §18).

1. **Pollution**. For type q = 0 (air), 1 (water), 2 (garbage):
   `avg_q = (Σ over the S × S cells (S·x + i, S·z + j), i outer, j inner, of pollution.GetPollutionValue(q, cx, cz))
   ÷ (S·S)` (unsigned integer division; the pollution slot returns 16 bits, AX; it reads the **source** grids,
   pollution.md). The caller keeps only the **low byte**: `p_q = avg_q mod 256` *[quirk]*.

   *[compat]* (RS3, from the exe; helper `0x006F7FF0`):
   - The sum is a **32-bit unsigned accumulator that wraps**. It is one stack dword set to 0, and each 16-bit result
     is zero-extended and added with a 32-bit `add`.
   - The divisor `S·S` is a 32-bit `imul`, and the division is an unsigned 32-bit `div`.
   - The loop bounds `S·x … S·x + S` and `S·z … S·z + S` are compared unsigned.
   - The helper's arguments are `(q, S·x, S·z, S)`. Its outer loop runs over the first coordinate, and each call is
     `GetPollutionValue(q, cx, cz)`, which is the i-outer, j-inner order above.

   A wrap needs `S² · 65535 ≥ 2³²`, that is S ≥ 257, so it cannot happen with any real tract size. Example: with
   S = 4 and every cell at 300, the sum is 4800, the average is 300, and the low byte is `p_q = 44`.
   `hqLoss = ((HQdecay + air(p_0)) + water(p_1)) + garbage(p_2)` with the three tuning curves of §7.1 evaluated at
   `float(p_q)`.
2. **Decay and buzzards**, for w = R$, R$$, R$$$ (one flag `buzzard := false` per tract):
   - `EQ_w := EQ_w − EQdecay`; if the result is < 0, EQ_w := 0. Store.
   - `HQ_w := HQ_w − hqLoss`; if < 0, HQ_w := 0. Store.
   - If **HQ_w < deathsDoorHQ** (strict) and not `buzzard` and the **current** pop_w (before this pass's update) ≠ 0:
     `r := rng.RandomDoubleRangeUniform(0.0, 100.0)` (first draw); if **r < chance**:
     - `CreateVisualEffect("buzzards", &effect)` on the effects manager (slot 7). If it returns no effect, nothing
       else happens (and a later wealth level of the same tract may roll again).
     - Otherwise: `wx = float(EQgrid_w.GetTractWidthX() × x) + GetTractWidthX() × 0.5`, same for wz with Z (each a
       float32 step); `y = (terrain.GetAltitude(wx, wz) + height) + rng.RandomDoubleRangeUniform(−delta, +delta)`
       (second draw; `RandomDoubleRangeUniform(a, b) = a + u × (b − a)`, foundations §6.1); place the effect with an
       identity rotation, translation (wx, y, wz) and scale 1 (`SetSourceTransform`), `Start(0)`, release it;
       `buzzard := true`.

   Draws: 0, 1 or 2 per wealth level, at most 2 effects' worth per tract (one effect).
3. **Population**: read `new_w := developer_w.GetPopulationMap().GetTractValue(x, z)` for all three
   levels before any write. The grid is a `cISC4SimGrid<int16>` (class `0x49B9E605`, growth-developer.md §1);
   this read uses the 16 bits as unsigned (corrected: the grid type is not uint16; the values read the same).
4. For w = R$, R$$, R$$$ (flag `arrived := false`, `total := 0`):
   - total += new_w; old_w := pop_w; pop_w := new_w; running population += new_w.
   - If old_w < new_w: nc_w := new_w − old_w (16-bit); blend EQ_w, then HQ_w, for the newcomers (formula: other
     spec, EQ and HQ updates); arrived := true. Otherwise nc_w := 0.
   - If new_w = 0: EQ_w := 0 and HQ_w := 0.
5. **Ageing**. age := the age grid (uint8). If total ≠ 0: read the month (simulator `GetSimDate`, slot 6, month
   only); if month = 12, age := (age + 1) mod 256. If total = 0: age := 0.

   *[compat]* (RS2, from the exe) The month is read **per tract and nothing is latched**:
   - `DoPopulationChange` calls `GetSimDate` itself (`0x006FAA95`, through the cached simulator pointer) for each
     tract with total ≠ 0, and compares the month with 12 at once (`0x006FAA98`);
   - a tract with total = 0 does not call it at all.

   In the original, state 4 always runs within one day, so every tract of a pass sees the same month. A host that
   lets the date change in the middle of a pass must still read the date per tract, as the original would.
6. **Newcomers' age**: if arrived and Σ new_w ≠ 0:
   `age := (Σ_w old_w × age + Σ_w nc_w × newAge_w) ÷ Σ_w new_w` (signed 32-bit, truncated), with newAge_w the
   tuning ages (§7.1). *[quirk]* The numerator uses the **old** populations even of levels that shrank, so a
   tract whose residents changed wealth level gets older (test 6).
7. LE := low byte of own slot 29 `GetCellLifeExpectancy(S·x, S·z)` (§6.4), evaluated **after** the EQ/HQ updates.
8. **Generational reset**: if age > LE (signed): for every w, `EQ_w := EQ_w × EQreset`, `HQ_w := HQ_w × HQreset`
   (stored); then age := resetAge.
9. Store the low byte of age in the age grid.
10. leSum += LE × total (32-bit product, sign-extended to 64 bits).
11. **Histogram**: `b := (age − 20) ÷ 3` (signed, truncated toward zero); if b, read as unsigned, is ≥ 20, b := 19.
    So ages 18..22 → 0, 23..25 → 1, …, 74..76 → 18, and **ages 0..17 and ≥ 77 → 19** *[quirk]*. For each w:
    ageCount[b] += pop_w; ageEqSum[b] += trunc(EQ_w) × pop_w (EQ after step 8).

The pass ends when the iterator is done; state 12 follows.

### 5.6 `PostPopulationChange` (state 12)

1. population := running population.
2. History `0x2A5E7009` := population > 0 ? leSum ÷ population (unsigned 64-bit) : 0. This is the
   **population-weighted average life expectancy**.
3. For each bucket b: `avg_b := ageCount[b] > 0 ? float(ageEqSum[b]) / float(ageCount[b]) : 0` (both read as
   unsigned).
4. For each age group g = 0..8 (curves `0xEA540621 + g`, §7.2), with popSum := 0 and eqSum := 0:
   for b = 0..19 with ageCount[b] > 0:
   `n := trunc((curve_g(float(21 + 3b)) × float(ageCount[b])) × 0.01f)`; popSum += n;
   `eqSum := trunc(float(n) × avg_b + float(eqSum))`.
   Then history `0xCA545CAB + g` := popSum and `0xCA545CCA + g` := popSum > 0 ? eqSum ÷ popSum (integer) : 0.

   *[compat]* Precision of `n` (RS1, verified in the bytes at `0x006F814F`–`0x006F8163`): `0.01f` is the
   **float32** constant `0x3C23D70A` = 0.0099999998 (`0x00A823A4`), not a double. The two products stay in the x87
   register with no store between them, so at the game's 24-bit precision (foundations §18.3) each is rounded to
   float32 and `_ftol` truncates the rounded value. Worked example (test 10): curve 6 × count 200 = 1200.0; the exact
   product 1200 × 0.0099999998 = 11.99999973 lies 0.28 ulp below 12, so it **rounds to 12.0** and n = 12. An
   implementation that keeps the product unrounded (double arithmetic with the float constant) gets 11.99999973 →
   **11**, which is wrong. Because `0.01f` is 2.2 × 10⁻⁸ below 0.01 relatively (less than half an ulp), `100k × 0.01f`
   rounds to exactly k for every integer k up to 2^24.

### 5.7 History (state 9)

In this order: `0xAA1A2864` := trunc(own slot 32 `GetGlobalEQ()`); `0xAA1A2866` := trunc(own slot 33
`GetGlobalHQ()`) (both REH §4.8, walked x outer, z inner, RS4); `0xAA1A2CCA` := population. Then, if population > 0:
`h := float(hospitalCovered) / float(population)` (computed first), `0x4A8A3FE5` := rhe(float(schoolCovered) /
float(population) × 1000), then `0x4A8A3FE2` := rhe(h × 1000), where rhe rounds half to even (the original adds
12582912 and takes the float's low bits). If population ≤ 0: `0x4A8A3FE2` := 0, then `0x4A8A3FE5` := 0.

## 6. Queries (population half)

### 6.1 `GetTractQuotient(values[3], pops[3], x, z)` (internal, used by slots 35–38, 43, 44)

`Σp := (p0 + p1 + p2) mod 65536` *[quirk: the sum is kept in 16 bits]*, with p_w the population grids' values
at tract (x, z). If Σp = 0 → 0.0. Otherwise `((v2·p2 + v1·p1) + v0·p0) / Σp` in float32 steps (each product is a
float × int step), v_w the value grids' values at (x, z).

### 6.2 `GetAverageEQGrid` / `GetAverageHQGrid` (slots 21, 22)

Each call **creates a new float grid** (class `0x49B9E60A`), stores it in `*out`, `AddRef`s it for the caller
(the caller must `Release` it), `Init`, `SetTractSize(E)`, `SetInstanceID(0x69D5C351` (EQ) / `0x69D5C360`
(HQ)). Then for every tract of that grid: value := `float(trunc(GetTractQuotient(EQ or HQ, population, x, z)))`;
min starts at 255.0, max at 0.0 and track the stored values; `*min`, `*max` are written if non-null. Returns the
**max pointer** in EAX.

*[compat]* (RS6, from the exe; shared helper `AverageGrids`, `0x006F70E0`) **min and max are updated with every
tract, populated or not**, as REH §4.8 says, and that text is normative:
- An unpopulated tract (16-bit population sum 0) stores 0.0, and that 0.0 goes through the same compare as any
  other value (`0x006F722D`–`0x006F7263`).
- The compares are strict: `min := v` only when `min > v`, and `max := v` only when `max < v`. A NaN updates
  neither.
- The walk is x outer, z inner over the new grid's own `GetTractCountX/Z`.

Results:
- An empty city returns `min = max = 0.0`.
- Only a grid with no tracts leaves 255.0 / 0.0.

Example: a 1 × 2 grid where tract (0, 0) has R$ population 10 at EQ 100.0 and tract (0, 1) is empty.
- The values are 100.0 and 0.0.
- `min` goes 255.0 → 100.0 → 0.0, and `max` goes 0.0 → 100.0.
- The result is `min = 0.0`, `max = 100.0`.

### 6.3 `GetEQGrids` / `GetHQGrids` / `GetPopulationGrids` (slots 23, 24, 25)

Write the three grids (R$, R$$, R$$$) to the three **out-references** and `AddRef` each. EAX is the result of the
third `AddRef` (a reference count). *[choice]* A replacement returns 1 (true) or the count; no caller tests it.

### 6.4 Life expectancy and workforce

Each call fetches the tuning exemplar (§7) and builds a temporary curve.

| Slot | Result |
|---|---|
| 29 `GetCellLifeExpectancy(x, z)` | curve `0x6941A67F` (code default (0,0),(200,200), then loaded) at own slot 36 `GetCellHQ(x, z)`; truncated; **AL** (uint8). No exemplar → 0 |
| 34 `GetGlobalLE()` | the same curve at own slot 33 `GetGlobalHQ()`; **AL** |
| 30 `GetCellWorkforcePercent(x, z)` | curve `0x4941A92B` (default (0,0),(200,100), then loaded) at `float(uint8 slot 29(x, z))`, × **0.01f** (float32 constant, one float32 rounding, `0x006FC81E`; RS1); written through a **hidden result pointer** (SC4Percentage). No exemplar → a static float that is never written (0.0 *[inferred]*) |
| 31 `GetGlobalWorkforcePercent()` | the same with slot 34 (× 0.01f at `0x006FC997`) |

x, z are cell coordinates (the other spec converts them to tracts in `GetCellHQ`).

### 6.5 `GetAverageNewAgeByWealth(w)` (slot 42)

Builds `{25, 30, 25}` (code defaults; *[quirk]* the third should be 35), overwrites them with properties
`0xC929C652`, `0x0929CC30`, `0x0929CC36` (read as uint8) when the exemplar has them, and returns
`ages[(w − 1) mod 256]` in **AL**. No bounds check: w ∉ {1, 2, 3} reads outside the array (undefined;
*[choice]* FreeCity4 returns 0).

### 6.6 Other population-half slots

- Slot 5 `GetProximityMap(w)`: w = 1/2/3 → proximity R$/R$$/R$$$; anything else → null. No `AddRef`.
- Slot 16 `GetPopulation()`: population (§3).
- Slots 39 / 40 / 41: the school grade map, hospital grade map and **age grid** pointers (no `AddRef`).
- Slots 56 / 57 `GetAutoBudget(occupant)` / `SetAutoBudget(occupant, bool)`: the occupant's bool property
  `0x6C05FC4D` (default false) through its property holder (occupant slot 6).

## 7. Tuning *[compat]*

One exemplar, key **type `0x6534284A`, group `0xE7E2C2DB`, instance `0x69189C84`**, fetched through the persistent
resource manager (slot 3, IID `0xA52160F5`). Names below are from `re/tuning/simulators.md`.

### 7.1 Population tuning T (read at `Init` and at the first state-4 call of each cycle)

First the ordinance products: `eduProd := Π` of the decay multipliers in the education ordinance-effect list and
`healthProd := Π` over the health list (other spec, ordinance effects; 1.0 for empty lists). Then the defaults
below are set **unconditionally**, the exemplar is read, and finally `EQdecay := EQdecay × eduProd`,
`HQdecay := HQdecay × healthProd` (float32). tuningLoaded := true.

| Property | Name | Type | Code default | Shipped | Use |
|---|---|---|---|---|---|
| `0xC929C433` | EQ Decay | f32 | 1.0 | 0.2 | EQdecay |
| `0x0929C439` | HQ Decay | f32 | 1.0 | 0.2 | HQdecay |
| `0xC929C652` / `0x0929CC30` / `0x0929CC36` | Low / Medium / High Wealth Average Age | s16 | 25 / 30 / 35 | 25 / 30 / 35 | newAge_w (§5.5 step 6) |
| `0xCA027307` | Average Age reset value | u8 | 40 | 40 | resetAge |
| `0xCA027325` | EQ reset percent | pct (% × 0.01f) | 0.7 (`0x3F333333`) | 80 → **0.79999995** (`0x3F4CCCCC`) | EQreset |
| `0xCA027328` | HQ reset percent | pct (% × 0.01f) | 0.7 (`0x3F333333`) | 80 → **0.79999995** (`0x3F4CCCCC`) | HQreset |
| `0xC929CC58` / `0x0929CC65` / `0x2929CC6B` | Low / Medium / High Wealth EQ | f32 | 4 / 6 / 8 | 20 / 40 / 60 | newcomer EQ bonus (other spec) |
| `0xC929CC8D` / `0x6929CC92` / `0x4929CC97` | Low / Medium / High Wealth HQ | f32 | **none** *[quirk]* (uninitialised) | 20 / 40 / 60 | newcomer HQ bonus |
| `0x6AB70DA7` | Death's door HQ | f32 | 0 | 2 | deathsDoorHQ |
| `0x6AB71654` | Death's door effect height | f32 | 0 | 50 | height |
| `0x4AB71659` | Death's door effect height delta | f32 | 0 | 10 | delta |
| `0x4AB70DAC` | Death's door effect chance | f32 | 0 | 1.5 | chance |
| `0x6941B022` | Air Pollution to HQ Decay curve | curve | (0,0),(32000,100) | (0,0),(512,0.1),(1024,50) | air |
| `0xA941B028` | Water Pollution to HQ Decay curve | curve | (0,0),(32000,100) | (0,0),(512,0.1),(1024,50) | water |
| `0xC941B02E` | Garbage to HQ Decay curve | curve | (0,0),(32000,100) | (0,0),(512,0.2),(1024,20) | garbage |

**Curve pattern** (foundations §4.1): the three pollution curves are rebuilt from the code default **only when
the exemplar exists**, then loaded by key: exemplar present → "default"; exemplar missing → "keep" (empty on the
first read; *[choice]* an empty curve evaluates to 0.0). *[choice]* FreeCity4 uses `0.0` for the uninitialised HQ
bonuses when the exemplar or property is missing.

**Percentages (RS1)** *[compat]*: "pct" is the shared percentage property getter `0x005FDAC0` (called at
`0x006FC233`/`0x006FC242` for the two reset percents). It reads the property as a number of any type, converts it
to the x87 register exactly, multiplies by the **float32** constant `0.01f` = `0x3C23D70A` (`0x00A823A4`) and stores
a float32. Every arm of the getter uses that one constant; no `double` 0.01 appears in it or anywhere in this class
(protection.md §6.2 found the same for its three classes). So the shipped 80 becomes **0.79999995**
(`0x3F4CCCCC`), not 0.8f (`0x3F4CCCCD`): 80 × 0.0099999998 is exactly halfway between the two floats and rounds to
the even one. This changes the generational reset (§5.5 step 8; test 8).

### 7.2 Simulator tuning (read at `Init` when not initialised)

Code defaults are set first, unconditionally: health block (f0 0.6, max funding 1.2, strike efficiency 0.2,
strike duration 10, min grade 0.9, ID `0x077ACCD2`, init funding 0.25, a 100.0 field, strike flag 0, strike start
0) and the same values for the education block (ID `0x077ACCD0`); the funding curves and the nine age-group curves
are rebuilt from the point lists L1 = (0,0),(100,100),(120,110) and L2 = (0,20),(100,100). Then, **only if the
exemplar exists**:

| Property | Name | Type (reader) | Code default | Shipped | Target / pattern |
|---|---|---|---|---|---|
| `0xE918946F` | (OBS) Health Min Acceptable Funding % (not shipped) | f32 × 0.01 | 0.6 | — | health f0; never read afterwards *[inferred]* |
| `0x2918949E` | Health Funding To Efficiency Curve | curve | L1 | 0,0,100,100,120,110 | default |
| `0x0A53B2F9` | Health Funding to Coverage Curve | curve | L2 | 0,50,100,100 | default |
| `0xA9189479` | Health Init/Max Allowed Funding | vector × 0.01 | 0.25, 1.2 | 25, 120 | [0] init, [1] max |
| `0x89189482` | Health Strike Efficiency % | × 0.01 | 0.2 | 20 | |
| `0x8918948B` | Health Strike Duration | s32 | 10 | 500 | |
| `0x89189495` | Health Min Grade to End Strike | % × 0.01 | 0.9 | 90 | |
| `0x691894A4` | Health Strike Response Curve | curve | L2 | 0,100,80,0 | default |
| `0xC91894A8` | (OBS) School Min Acceptable Funding % (not shipped) | f32 × 0.01 | 0.6 | — | education f0; never read afterwards *[inferred]* |
| `0x691894C6` | School Funding To Efficiency Curve | curve | L1 | 0,0,100,100,120,110 | default |
| `0xEA53B30F` | School Funding to Coverage Curve | curve | L2 | 0,50,100,100,120,110 | default |
| `0x891894AD` | School Init/Max Allowed Funding | vector × 0.01 | 0.25, 1.2 | 25, 120 | |
| `0xE91894B3` | School Strike Efficiency % | × 0.01 | 0.2 | 20 | |
| `0xA91894BC` | School Strike Duration | s32 | 10 | 500 | |
| `0x691894C0` | School Min Grade to End Strike | % × 0.01 | 0.9 | 90 | |
| `0xC91894CB` | School Strike Response Curve | curve | L2 | 0,100,80,0 | default |
| `0xEA540621` … `0xEA540629` | Avg. Age to 1-10 … 81-90 Age Group | curve | L1 | `re/tuning/simulators.md` | §5.6; default |
| `0x8A891421` | School Capacity To Staff Divisor | u32 | 30 (set in this branch only) | 30 | other spec |
| `0x8A89142A` | Hospital Capacity To Staff Divisor | u32 | 30 (this branch only) | 30 | other spec |
| `0x6A9F6EDD` | Coverage radius slop multiplier | f32 | 1.1 (this branch only) | 1.1 | other spec |
| `0x4AA84161` | Maximum EQ Boost | f32 | 10.0 (this branch only) | 1.3 | other spec |
| `0x2AA84164` | Maximum HQ Boost | f32 | 10.0 (this branch only) | 1.1 | other spec |
| `0xEC43826B` | School Capacity Chunk Size | u32 vector | none | 500, 750, 1000, 1000, 10000, 300, 1200 | other spec |
| `0xAC43826D` | Hospital Capacity Chunk Size | u32 vector | none | 500, 500, 2000 | other spec |

- "default" = rebuilt from the code default list, then loaded by key (missing property → code default; missing
  exemplar → code default, since the rebuild is unconditional).
- Every "× 0.01" in this table is a float32 multiplication by `0.01f` (`0x00A823A4`), either through the pct getter
  (`0x006FBE71`, `0x006FBF69`) or inline after a float read (`0x006FBDB5` to `0x006FBF3E`), each stored as float32
  (RS1). Shipped values therefore become 25 → 0.25, 120 → **1.1999999** (`0x3F999999`), 20 → **0.19999999**
  (`0x3E4CCCCC`), 90 → 0.9f (`0x3F666666`, the nearest float to 0.9, so no visible change).
- *[compat]* Without the exemplar, the staff divisors, slop multiplier, boosts and chunk sizes are **never set**
  (uninitialised). *[choice]* FreeCity4 uses the "this branch" defaults and empty chunk lists.
- *[quirk]* The original releases the exemplar before its last property reads (the holder stays valid).
- The meaning of every value except the age-group curves belongs to the other spec.

### 7.3 Per-call curves

| Curve | Where | Pattern |
|---|---|---|
| `0x6941A67F` Health Quotient to Life Expectancy | slots 29, 34; fresh temporary curve per call | default (0,0),(200,200); missing exemplar → the slot returns 0 |
| `0x4941A92B` Life Expectancy to Workforce % | slots 30, 31; fresh temporary curve per call | default (0,0),(200,100); missing exemplar → static 0.0 |
| per-building curves `0xC9299662`, `0xC92D7EC0`, `0xE921A936` (schools) / `0xA92AE3BD`, `0xA92D9FCD`, `0x2921A9CC` (hospitals) | `GetPropertyHolderCurveData` (other spec) | property present → property-pointer loader; absent → (0,100),(100,100) |

## 8. Messages posted

**History** (`RecordHistoryData(id, value)`, internal): if a message server is held, create a
`cRZMessage2Standard`, type **`0x89EFA536`**, data1 = id, data2 = value, and **send** it synchronously
(`MessageSend`), then release it. The history warehouse stores (data1, data2); the win trend bar also listens.

| ID | When | Value |
|---|---|---|
| `0x2A5E7009` | state 12 | average life expectancy (§5.6) |
| `0xCA545CAB` + g (g = 0..8) | state 12 | age-group population |
| `0xCA545CCA` + g | state 12 | age-group average EQ (integer) |
| `0xAA1A2864` | state 9 | trunc(global EQ) |
| `0xAA1A2866` | state 9 | trunc(global HQ) |
| `0xAA1A2CCA` | state 9 | population |
| `0x4A8A3FE5` | state 9 | school-covered per mille of population *[inferred name]* |
| `0x4A8A3FE2` | state 9 | hospital-covered per mille *[inferred name]* |

Queue lengths: §4.1 step 5. The coverage counters are filled by the other spec's coverage states.

## 9. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0x4990C013`) *[compat]*

### 9.1 Record (version 7)

```
u16   version = 7
s32   proximity tract size
ref   proximity R$, R$$, R$$$
s32   population tract size P
ref   population R$, R$$, R$$$
ref   age grid
u32 n, n × ref   school coverage grids
u32 n, n × ref   hospital coverage grids
ref   school grade map
ref   hospital grade map
s32   EQ tract size E
ref   EQ R$, R$$, R$$$
ref   HQ R$, R$$, R$$$
f32   unused float 1
f32   unused float 2
u32   state
u8    dirty ∨ state ≠ 10
s32   population
department block (health)
department block (education)
list  pending school list
list  pending hospital list
list  other list
u32 n, n × list   school building lists
u32 n, n × list   hospital building lists
list  list (health-affecting buildings)
iterator: s8 iterState (0 fresh, 1 running, 2 done), s32 x, s32 z, u32 step, s32 x0, s32 z0, s32 x1, s32 z1
u64   leSum
s32   running population
u8    school global auto-budget
u8    hospital global auto-budget

department block:
f32 f0, f32 max funding, f32 strike efficiency, s32 strike duration, f32 min grade, u8 strike flag,
s32 strike start, ref department 1, ref department 2, f32 init funding, f32 100.0 field, u32 ID

list: u32 count, count × ref occupant
```

Grid references are read with IID `0xC9BD939A`, department references with `0x223E2B67`, occupant references with
`0x26BB767E` (foundations §2.7). The ordinance-effect lists, the histograms, the coverage counters, the tuning and
the temporary grid are not saved. Which list is which belongs to the other spec (building lists).

### 9.2 `Write` / `Read`

- `Write` always returns true (AL = 1).
- `Read` never rejects a version and always returns true (AL = 1, `0x00700E4E`), ignoring stream errors. It calls
  no `GetError` and tests no stream result. The loader ignores the result anyway (foundations §3.5).
  - **Versions** (RS9): no version is unreadable to the original.
    - ≥ 7: read with the version-7 layout, so a newer layout would be misread.
    - 3–6: the conditionals below.
    - 0–2: read as version 2.
  - **After a stream error** (RS9, from the exe). Reading goes on, and every later call fails and leaves its
    destination unchanged (foundations §2.9). So:
    - members read directly keep their constructed values: the three tract sizes, the unused floats, dirty,
      population, the iterator, leSum and the running population;
    - every later reference becomes **null**, because `SC4ReadGZSerializable` (`0x005D9730`) clears its
      destination first;
    - values that pass through a local take stale stack contents:
      - the **state** (an uninitialised local, `0x00700BF7`–`0x00700C05`);
      - the department-block scalars (the block reader's argument slot, `0x006F7BB4`);
      - the version itself, if its read fails;
    - each list's count is an uninitialised local (`0x005DB3D7`). If it lies in 1…9,999,999, that many **null**
      entries are appended;
    - the school coverage count is preset to 1 (`0x00700A44`). The hospital coverage count and the two
      building-list-vector counts reuse whatever count was read last (the same stack slot).

    The next `Init` creates the null grids and departments (§4.1 steps 8–9) and overwrites the department scalars
    (§9.3). It does **not** repair the state or the lists.
  - *[choice]* FreeCity4 does not reproduce the stale values:
    - On a stream error, `Read` returns **false** and leaves the simulator **as constructed**. It releases what it
      acquired, restores every field it wrote and leaves cityStarted false, so the next `Init` sets the city up
      as new (§4.1 step 6, state 0), as crime.md §12 does. The return value is free, since the loader ignores it.
    - Every version is accepted as in the original (test 20). A newer layout then ends in a stream error (handled
      as above) or is misread, as in the original.
  - **Replaced entries are not released** (RS10, from the exe):
    - a list is cleared by freeing its nodes only (`0x00413E60`);
    - a shrinking vector of lists frees nodes only (`0x00700490`);
    - a shrinking coverage vector drops its tail without `Release` (`0x00700A6F`–`0x00700A87`);
    - every reference read nulls its destination without `Release` (`0x005D9742`).

    On the original's only path this changes nothing. `Read` runs on an object the class-object store has just
    constructed (foundations §3.5), whose references are all null and whose lists are empty: the constructor sizes
    the coverage vectors to 7 and 3 nulls and the building-list vectors to 7 and 3 empty lists (`0x0070087C`,
    `0x007008FB`). *[choice]* FreeCity4 may release what it replaces, which the core already does
    (`release_department`). This cannot be observed.
  - Lists: cleared first; a count of 0 or ≥ 10,000,000 reads nothing more; null references are appended.
  - Department blocks: version < 5 → f0, max funding, strike efficiency and init funding are × 0.01f (stored as
    percentages); version < 6 → min grade × 0.01f (float32 constant, one float32 rounding each; the block reader
    at `0x006F7BA0`, RS1).
  - Version ≥ 3: read the iterator and leSum; version ≥ 4: read the running population (version 3 keeps the
    constructed value *[inferred]*).
  - Version < 3: iterator at (0, 0), fresh (done if (0, 0) is outside its rectangle); leSum := 0; running
    population := 0; if state = 4 → state := 11.
  - Version ≥ 7: read the two auto-budget bytes (non-zero → 1); otherwise both 0.
  - Then state 1 → 13, 2 → 14, 3 → 15 (the temporary grid is not saved); tuningLoaded := false; cityStarted := true.

### 9.3 What `Init` changes after a load *[compat]*

- The iterator is reset to (0, 0) and leSum := 0 (§4.1 step 10).
- Because initialised is false after a load, `Init` re-reads the simulator tuning (§7.2), whose unconditional
  defaults **overwrite the loaded department-block scalars, including the strike flags and strike start**:
  strikes do not survive save and load *[quirk]*.
- *[quirk]* A save taken during state 4 restarts the pass at (0, 0) after loading, but the loaded running
  population is kept, so the tracts already visited are counted twice in that month's population; the histograms
  hold whatever memory contained (state 11 did not run in this session) and their history for that month is
  meaningless. *[choice]* FreeCity4 zeroes the histograms at construction (only that month's age-group history
  differs).

## 10. Interface slot tables *[compat]*

All methods are `__thiscall`; "pop" is the number of argument bytes the callee removes (checked with
`tools/abi_extract.py --check`), return channels read from the epilogues. "SDK" compares with gzcom-dll
`cISC4ResidentialSimulator.h`; the slot **order** matches the header (60 interface slots, no overloads).

### 10.1 `cISC4ResidentialSimulator` (vtable `0x00AB2C10`, 60 interface slots + 4 class slots)

| Slot | Off | Method (as compiled) | Address | Pop | Return | Behaviour / SDK |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | QueryInterface(iid, void**) | `006F6E60` | 8 | AL | §1 |
| 1 | `+0x04` | AddRef | `004F7BA0` | 0 | EAX count | |
| 2 | `+0x08` | Release | `006F7DD0` | 0 | EAX count | §1 |
| 3 | `+0x0C` | Init | `006FEAD0` | 0 | AL | §4.1 |
| 4 | `+0x10` | Shutdown | `006F97F0` | 0 | AL = 1 | §4.2 |
| 5 | `+0x14` | GetProximityMap(WealthType) | `006F6F90` | 4 | EAX ptr | §6.6 |
| 6 | `+0x18` | SchoolIsOnStrike() | `006F6F10` | 0 | AL 0/1 | education strike flag |
| 7 | `+0x1C` | HealthIsOnStrike() | `006F6F20` | 0 | AL 0/1 | health strike flag |
| 8 | `+0x20` | EndSchoolStrike() | `006F9470` | 0 | AL | if slot 6 → end the strike (other spec), else false |
| 9 | `+0x24` | EndHealthStrike() | `006F94B0` | 0 | AL | same with slot 7 |
| 10 | `+0x28` | **ChanceOfSchoolStrike(SC4Percentage* ret)** | `006F9670` | **4** | EAX = ret | hidden result pointer. **SDK error**: `float (void)` |
| 11 | `+0x2C` | **ChanceOfHealthStrike(ret)** | `006F9730` | **4** | EAX = ret | **SDK error** (as 10) |
| 12 | `+0x30` | **GetSchoolSystemRating(ret)** | `006F9F20` | **4** | EAX = ret | **SDK error** (as 10) |
| 13 | `+0x34` | **GetHealthSystemRating(ret)** | `006F9DD0` | **4** | EAX = ret | **SDK error** (as 10) |
| 14 | `+0x38` | **GetSchoolSystemTotals(queryData, groups*)** | `006FA240` | 8 | **EAX = count** | number of buildings counted. **SDK error**: bool. Other spec |
| 15 | `+0x3C` | **GetHospitalSystemTotals(queryData, groups*)** | `006FA260` | 8 | **EAX = count** | **SDK error** (as 14) |
| 16 | `+0x40` | GetPopulation() | `006F6F70` | 0 | EAX | §6.6 |
| 17 | `+0x44` | GetTotalCityEducationUpkeepCost() | `004E13E0` | 0 | EAX = 0 | shared "return 0" body |
| 18 | `+0x48` | GetTotalCityHealthUpkeepCost() | `004E13E0` | 0 | EAX = 0 | |
| 19 | `+0x4C` | SetOccupantFundingPercentages(occ, const&, const&, bool) | `006F8290` | 16 | AL incidental | other spec; also sets the dirty flag. SDK says bool: *[choice]* a replacement returns true |
| 20 | `+0x50` | GetOccupantFundingPercentages(occ, &, &, bool) | `006F84B0` (jump to `006F8360`) | 16 | EAX incidental (a stored value or pointer on every path) | other spec; SDK says bool: *[choice]* a replacement returns true |
| 21 | `+0x54` | **GetAverageEQGrid(grid*&, float* min, float* max)** | `006FCF30` | 12 | **EAX = max pointer** | §6.2. **SDK error**: bool; a new grid per call, owned by the caller |
| 22 | `+0x58` | **GetAverageHQGrid(grid*&, min*, max*)** | `006FCFD0` | 12 | EAX = max pointer | **SDK error** (as 21) |
| 23 | `+0x5C` | **GetEQGrids(grid*&, grid*&, grid*&)** | `006F72C0` | 12 | EAX = refcount | §6.3. **SDK error**: the 2nd and 3rd parameters are out-references too, and all three grids are `AddRef`ed |
| 24 | `+0x60` | **GetHQGrids(&, &, &)** | `006F7300` | 12 | EAX = refcount | **SDK error** (as 23) |
| 25 | `+0x64` | **GetPopulationGrids(&, &, &)** | `006F7340` | 12 | EAX = refcount | **SDK error** (as 23) |
| 26 | `+0x68` | GetSchoolQueryData(occ, data) | `006F91C0` | 8 | AL | other spec |
| 27 | `+0x6C` | GetHospitalQueryData(occ, data) | `006F7E80` | 8 | AL | other spec |
| 28 | `+0x70` | EstimateCurrentOccupantCapacity(occ, &, &) | `006FD510` | 12 | EAX garbage on the main path | other spec; SDK says bool: *[choice]* return true |
| 29 | `+0x74` | **GetCellLifeExpectancy(x, z)** | `006FC570` | 8 | **AL** (uint8) | §6.4. **SDK error**: int32 |
| 30 | `+0x78` | **GetCellWorkforcePercent(SC4Percentage* ret, x, z)** | `006FC6E0` | **12** | EAX = ret | §6.4. **SDK error**: `float (x, z)` |
| 31 | `+0x7C` | **GetGlobalWorkforcePercent(ret)** | `006FC860` | **4** | EAX = ret | **SDK error**: `float (void)` |
| 32 | `+0x80` | GetGlobalEQ() | `006F7720` | 0 | ST0 | other spec |
| 33 | `+0x84` | GetGlobalHQ() | `006F7870` | 0 | ST0 | other spec |
| 34 | `+0x88` | **GetGlobalLE()** | `006FC9E0` | 0 | **AL** (uint8) | §6.4. **SDK error**: float |
| 35 | `+0x8C` | GetCellEQ(x, z) | `006F79C0` | 8 | ST0 | other spec |
| 36 | `+0x90` | GetCellHQ(x, z) | `006F7A10` | 8 | ST0 | other spec |
| 37 | `+0x94` | GetCellEQByWealth(x, z, w) | `006F7A60` | 12 | ST0 (tail of the grid's `GetTractValue`) | EQ grid w at (x ÷ E, z ÷ E) (unsigned division), no bounds check on w |
| 38 | `+0x98` | GetCellHQByWealth(x, z, w) | `006F7AB0` | 12 | ST0 | same with HQ |
| 39 | `+0x9C` | **GetSchoolAverageGradeMap()** | `00809240` | 0 | **EAX grid ptr** | `cISC4SimGrid<float>*`. **SDK error**: int32 |
| 40 | `+0xA0` | **GetHospitalAverageGradeMap()** | `00407AC0` | 0 | EAX grid ptr | **SDK error** (as 39) |
| 41 | `+0xA4` | **GetAverageAgeMap()** | `006AAEB0` | 0 | EAX grid ptr | `cISC4SimGrid<uint8>*`. **SDK error**: int32 |
| 42 | `+0xA8` | **GetAverageNewAgeByWealth(w)** | `006F88B0` | 4 | **AL** (uint8) | §6.5. **SDK error**: int32 |
| 43 | `+0xAC` | GetEQMinAndMaxCellCoords(6 refs) | `006F7380` | 24 | AL | other spec |
| 44 | `+0xB0` | GetHQMinAndMaxCellCoords(6 refs) | `006F7550` | 24 | AL | other spec |
| 45 | `+0xB4` | GetOccupantCoverage(occ, const pct&, float&, float&) | `006FD260` | 16 | AL | other spec |
| 46 | `+0xB8` | GetSchoolBuildingCount() | `006F9590` | 0 | EAX | other spec |
| 47 | `+0xBC` | GetSchoolBuildings(list&, groups*) | `006FADE0` | 8 | incidental | other spec |
| 48 | `+0xC0` | GetHospitalBuildingCount() | `006F9600` | 0 | EAX | other spec |
| 49 | `+0xC4` | GetHospitalBuildings(list&, groups*) | `006FAE50` | 8 | incidental | other spec |
| 50 | `+0xC8` | GetMaxEQ() | `006FEA40` | 0 | EAX = 200 | shared body |
| 51 | `+0xCC` | GetMaxHQ() | `006FEA40` | 0 | EAX = 200 | |
| 52 | `+0xD0` | GetGlobalAutoBudgetForSchools() | `006FEA50` | 0 | AL | |
| 53 | `+0xD4` | SetGlobalAutoBudgetForSchools(bool) | `006FEA60` | 4 | AL = the value | |
| 54 | `+0xD8` | GetGlobalAutoBudgetForHospitals() | `006FEA70` | 0 | AL | |
| 55 | `+0xDC` | SetGlobalAutoBudgetForHospitals(bool) | `006FEA80` | 4 | AL = the value | |
| 56 | `+0xE0` | **GetAutoBudget(cISC4Occupant*)** | `006F7D70` | **4** | AL | §6.6. **SDK error**: `(void)` |
| 57 | `+0xE4` | **SetAutoBudget(cISC4Occupant*, bool)** | `006F7DA0` | **8** | EAX = the property helper's result | §6.6. **SDK error**: `(bool)` |
| 58 | `+0xE8` | EstimateIdealFunding(occ, SC4Percentage&) | `006FD680` | 8 | AL | other spec |
| 59 | `+0xEC` | ToggleTractTracking(int32, int32) | `006F7DC0` | 8 | none | no-op |
| 60 | `+0xF0` | scalar deleting destructor(flags) | `00700330` | 4 | EAX = this | class only |
| 61 | `+0xF4` | GetHealthAffectingBuildings(list&) | `006FAEC0` | 4 | — | class only: appends the health-affecting list and the other list, each occupant `AddRef`ed |
| 62 | `+0xF8` | TrackTracts() | `00407990` | 0 | none | class only: no-op |
| 63 | `+0xFC` | OutputTractInfo(…) | `006F7DC0` | 8 | none | class only: no-op (shared with 59) |

Replacement values where the original returns garbage or an unexpected channel: slots 19, 28 → true; 47, 49 →
true; 21/22 may return the max pointer (no caller tests it); 23–25 → 1; AL-only slots (29, 34, 42) → the value
zero-extended in EAX.

### 10.2 `cIGZSerializable` (vtable `0x00AB2BE4`, 6 slots)

| Slot | Off | Method | Address | Pop | Return |
|---|---|---|---|---|---|
| 0 | `+0x00` | QueryInterface (thunk to slot 0 of §10.1) | `006FEAA0` | 8 | AL |
| 1 | `+0x04` | AddRef (thunk) | `0068D250` | 0 | EAX |
| 2 | `+0x08` | Release (thunk) | `006FEAC0` | 0 | EAX |
| 3 | `+0x0C` | Write(stream) | `006F8C40` | 4 | AL = 1 |
| 4 | `+0x10` | Read(stream) | `007009A0` | 4 | AL = 1 |
| 5 | `+0x14` | GetGZCLSID() | `006F6E50` | 0 | EAX = `0x4990C013` |

### 10.3 `cIGZMessageTarget2` (vtable `0x00AB2BFC`, 4 slots)

| Slot | Off | Method | Address | Pop | Return |
|---|---|---|---|---|---|
| 0 | `+0x00` | QueryInterface (thunk) | `006FEA90` | 8 | AL |
| 1 | `+0x04` | AddRef (thunk) | `0068D230` | 0 | EAX |
| 2 | `+0x08` | Release (thunk) | `006FEAB0` | 0 | EAX |
| 3 | `+0x0C` | DoMessage(msg) | `00700350` | 4 | AL (§4.3) |

## 11. Dependencies and replaceability

### 11.1 Methods the original calls (population half and frame) *[compat]*

| Interface (vtable) | Slot | Off | Method | Pop | Return used |
|---|---|---|---|---|---|
| `cISC4City` | 34 / 52 | `+0x88/+0xD0` | GetOccupantManager / GetBudgetSimulator | 0 | EAX ptr |
| `cISC4BuildingDevelopmentSimulator` (`0x00AB0AF8`) | 32 | `+0x80` | GetDeveloper(DeveloperType) | 4 | EAX ptr |
| | 41 | `+0xA4` | GetBuildingMap() | 0 | EAX `cISC4SimGrid<uint8>*` |
| | 42 | `+0xA8` | GetBuildingMapKey(PurposeType, WealthType) | 8 | AL |
| `cISC4GrowthDeveloper` | 15 | `+0x3C` | GetPopulationMap() | 0 | EAX `cISC4SimGrid<int16>*` (class `0x49B9E605`) |
| `cISC4DemandSimulator` | 6 | `+0x18` | GetDemand(0x1000, 0) | 8 | EAX ptr |
| `cISC4Simulator` (`0x00AB2FB8`) | 6 | `+0x18` | GetSimDate(year*, month*, day*, dayOfYear*, weekDay*) (only month) | 20 | — |
| | 19 / 20 | `+0x4C/+0x50` | AddAgent(target, level, name, flags) / RemoveAgent(target) | 16 / 4 | AL |
| `cIGZMessageServer2` | 3 / 5 / 6 | `+0x0C/+0x14/+0x18` | MessageSend(msg) / AddNotification(target, type) / RemoveNotification(target, type) | 4 / 8 / 8 | — |
| `cIGZMessage2Standard` | 4 / 5 / 10 / 11 / 16 / 19 | `+0x10/+0x14/+0x28/+0x2C/+0x40/+0x4C` | GetType / SetType / GetVoid1 / SetData1 / SetData2 / GetData3 | 0 / 4 / 0 / 4 / 4 / 0 | EAX |
| `cISC4HistoryWarehouse` (`0x00A8CCB4`) | 13 | `+0x34` | SetHistoryDataQueueLength(id, n) | 8 | — |
| `cISC4PollutionSimulator` | 6 | `+0x18` | GetPollutionValue(type, x, z) | 12 | **AX** |
| `cISC4EffectsManager` (`0x00A9F264`) | 7 | `+0x1C` | CreateVisualEffect(const char* name, effect*&) | 8 | AL (the out value is tested) |
| `cISC4VisualEffect` (vtable not located) | 3 / 8 / 2 | `+0x0C/+0x20/+0x08` | Start(0) / SetSourceTransform(const transform&) / Release | 4 / 4 / 0 | — |
| `cISTETerrain` (`0x00AB3D18`) | 13 | `+0x34` | GetAltitude(x, z) | 8 | ST0 |
| `cIGZPersistResourceManager` (`0x00AD89D0`) | 3 | `+0x0C` | GetResource(key, iid, out, 0, 0) | 20 | AL |
| `cISCResExemplar` / `cISCPropertyHolder` | 7 / 9 | `+0x1C/+0x24` | AsPropertyHolder / GetProperty(id) | 0 / 4 | EAX |
| `cISC4Occupant` | 6 | `+0x18` | AsPropertyHolder | 0 | EAX |
| `cISC4OccupantManager` | 48 | `+0xC0` | (occupant list) in `Shutdown` | 4 | — |
| `cISC4SimGrid<T>` | 3, 4, 6, 9, 10, 12–15, 22, 24/25, 26, 27 | simgrid §4 | Init, Shutdown, SetInstanceID, SetTractSize, GetTractSize, counts, widths, GetTractValue, GetGridData, SetTractValue, SetTractValues | | |
| `cISC4BudgetSimulator` / departments | | | department creation and funding | | other spec, budget.md |
| `cRZRandom` (non-virtual) | — | — | RandomDoubleRangeUniform(a, b) (`0x00913761`) | 16 | ST0 |
| shared helpers (non-virtual) | — | — | `ProcessTwoGridsWithCallback<uint8, uint8>` (`0x006F8970`), scanning iterator `Reset`/`GetNext` (`0x0060C860` / `0x0060C8D0`), map utilities `0x005E4FA0` (seed), `0x005E4E80` (one distance pass), `0x005E4D60` (passes to convergence), `0x005E5130` (seed + convergence) | | |

### 11.2 Consumers (through `SL::spResidentialSim` or `cISC4City` slot 68)

| Caller | Slots |
|---|---|
| advisors | HQ: 34; HQ occupant variables: 49, 20, 27; safety: 10, 11, 16 |
| MySim / MyStreetSim | 21 and 22 (grid released after reading), 37, 41, 42, 47, 49 |
| MySim change-job input control | 37 |
| tutorial, audio ambience, string detokenizer, other UI | 16 |
| query tool | 39/40 (debug), 19/26/27/28 (funding UI), 5 (desirability equation, `GetCellValue`), 36 (DescribePick), 59, 26/27 (tips) |
| place-lot tool (coverage preview) | 45 |
| sub-budget window | 19, 26, 27 |
| automata generator | 26, 27 |
| vehicle manager (funds changed) | 49 |
| `cSC4Lot::SetCondition` | 30 (hidden pointer) |
| aura simulator | 8, 9 (strike busting); 25 (**raw** `GetGridData` of the population grids); 39, 40 (**raw** walker) |
| crime simulator (census) | 23, 25 (**raw** `GetGridData`) |
| demand simulator | stores the pointer at `Init`; census calls 30 (hidden pointer) and 35 |
| growth developer | 5, 39, 40 (`GetCellValue` in `EstimateDesirability`; in `DoIdle` the **proximity grids are passed raw** to a two-grid walker) |

No original code outside the class reads its members directly or calls its non-virtual functions; it is reached
only through the interface (search of every reference to the global and to city slot 68).

### 11.3 Replaceability

- The class is replaceable through the GZCOM class registry (class ID `0x4990C013`); the city creates it as
  component 36 and the class store restores it from saves.
- **Raw grid readers** (simgrid §6.3): the population grids (slot 25: aura, crime), the EQ grids (slot 23: crime),
  the grade maps (slots 39/40: aura) and the proximity grids (slot 5: growth developer) are read through
  `GetGridData`. A replacement must hand out game grids created per simgrid **§6.4**, with the §2 element types
  and tract sizes; the aura and crime walkers assume the population tract size P (4) matches their own geometry.
- A replacement must keep: the slot ABI of §10 (hidden-pointer returns, AL-only returns, the out-reference
  triples of 23–25 with `AddRef`, the new-grid-per-call of 21/22), the save record of §9 (grids by reference),
  the agent levels and the NewDay finished semantics, the history IDs of §8, and the `0x1000` demand node check
  at `Init` (the original fails `Init` without it).

## 12. Original quirks (reproduced)

1. Pollution averages are truncated to their low byte before the HQ-decay curves (§5.5 step 1).
2. The newcomers' age uses the old populations of all levels (§5.5 step 6).
3. Ages 0..17 fall into the oldest histogram bucket (§5.5 step 11).
4. `GetTractQuotient` sums the populations in 16 bits (§6.1).
5. `GetAverageNewAgeByWealth` defaults the high-wealth age to 25 (§6.5), unlike the population tuning (35).
6. The newcomer HQ bonuses have no code default (§7.1); several simulator-tuning values are set only when the
   exemplar exists (§7.2).
7. Strike state does not survive save/load (§9.3); a save during state 4 double-counts population (§9.3).
8. History `0x4A8A3FE2`/`0x4A8A3FE5` are written in opposite orders in the two branches of state 9 (§5.7).
9. Slots 21/22 create a new grid on every call (callers must release it).

## 13. Open questions

1. The `cISC4VisualEffect` vtable and the exact names of its slots 3 and 8 (only the buzzards path uses them).
2. Whether any caller tests the results of slots 19, 21–25, 28, 47, 49 *[inferred: none]*.
3. *Closed as posed, but it points at a real dependency* (§4.3): the day-level order of residential relative to the
   growth developers, pollution and the BDS. There is no original order to match — the original orders agents by
   target-pointer value at every level, which is a heap-layout artifact and not stable between sessions
   (`foundations.md` §13.5). Registration order is the *[choice]*, and §4.3's existing text is confirmed correct.
   **However**, this is not a case with no coupling at all: §5 state 1 reads the BDS building map (slot 41) and the
   pollution **source** grids, and §4.3 already notes the pass reads the growth developers' population maps. So a
   same-pass ordering dependency *may* exist, and it is not resolved. What is resolved is that it cannot be settled by
   reproducing the original's pointer order. If a differential later shows residential reading a building map,
   pollution source grid or population map that another agent updates earlier **in the same day**, that is a derived
   *[compat]* requirement: record it in `foundations.md` §13.4's rank table with evidence, rather than reordering
   registrations ad hoc. Until such a differential exists, registration order stands and no vector may assume
   otherwise.
4. Version 3 saves: the running population keeps its constructed value (not verified at run time).
5. The dirty flag has no reader; its purpose is unknown.

## 14. Corrections to earlier documents

- `docs/subsystems/demand-rci.md` §5:
  - `0x2A5E7009` is the population-weighted **average life expectancy**, not the average age; the accumulator is
    Σ LE × population, not Σ age × population (also `re/names_windows.tsv` comment for `006F8070`).
  - The pollution values are truncated to a byte and read from the pollution **source** grids.
  - The histogram bucket is `(age − 20) / 3` truncated toward zero with every out-of-range value (including ages
    below 18) mapped to 19, not `min(19, …)`.
  - The reset percentages' code defaults are 0.7, but the shipped values are 0.8.
  - The state is at the object's offset `+0x94` on Windows; `+0x90` is relative to the message-target sub-object.
  - Life expectancy is evaluated at cell coordinates (tract × P), after the EQ/HQ updates.
- This spec, round 3 (RS1, 2026-10-13): every `× 0.01` is the float32 `0.01f`, so the shipped reset percent is
  0.79999995, not 0.8, and §15 test 8's EQ/HQ results were one float below what round 3 stated. Test 10's values
  were right, but only with the float32 rounding of each product, which §5.6 step 4 now states.
- `docs/specs/pollution.md` §10.1: slot 6 `GetPollutionValue` is also called by residential `DoPopulationChange`
  (every tract, every month, three types), not only by the hospital query data.
- `docs/specs/aura.md` open question on population geometry: the residential population grids use tract size 4
  (the developers' population maps, `0x87CD6310` = 4), matching the aura's S = 4.
- `docs/subsystems/development.md` §4: the Mac offsets used there for the grade maps correspond to Windows slots
  39/40.

## 15. Conformance tests (must exist in `engine/conformance`)

Unless stated: shipped tuning, ordinance products 1.0, P = E = 4, no pollution, month ≠ 12.

1. **Proximity, one source**: 5 × 5 building map with key 6 only at (2, 2) → proximity R$ = |x − 2| + |z − 2|
   ((0, 0) = 4, (4, 2) = 2); proximity R$$ (no key 7) = 255 everywhere.
2. **Proximity, cap**: 300 × 1 map with key 7 at (0, 0) → value at x = 200 is 200, at x = 280 is 255.
3. **Proximity, coarser target**: proximity size 2, building map size 1, key at map cell (3, 1) → target tract
   (1, 0) = 0, (0, 0) = 1.
4. **Decay**: EQ 10, HQ 10 → 9.8 and 9.8 (EQdecay = HQdecay = 0.2). EQ 0.1 → 0.
5. **Newcomers' age**: pops (100, 0, 0) → (150, 0, 0), age 40, LE 100 → age **35**.
6. **Age quirk**: (100, 0, 0) → (50, 10, 0), age 40, LE 100 → age **71** ((100·40 + 10·30) ÷ 60).
7. **December**: total > 0, month 12, age 40, no arrivals → 41. Age 255 → 0. Total 0 → age 0.
8. **Generational reset**: age 70, pop (200, 0, 0) unchanged, EQ_R$ and HQ_R$ stored as 50.2 and 60.2
   (`0x4248CCCD`, `0x4270CCCD`). Decay by 0.2f leaves exactly 50.0 and 60.0. Step 7: GetCellHQ = 60.0, and the
   shipped curve `0x6941A67F` = (0,50) (20,55) (40,60) (60,65) (200,100) is exact at its point (60, 65), so
   **LE = 65** (RS7). leSum += 65 × 200 = 13000. Since 70 > 65 → age 40; EQ 50 → **39.999996** (`0x421FFFFF`),
   HQ 60 → **47.999996** (`0x423FFFFF`), each × EQreset = HQreset = `0x3F4CCCCC` (§7.1, RS1) with one float32
   rounding. The histogram of step 11 then adds trunc(EQ) = **39**, not 40, per resident: with pop (200, 0, 0) at age 40 (bucket 6),
   ageEqSum[6] += 39 × 200 = 7800. *(Corrected: round 3 of this spec gave 40 and 48, which assumed a double 0.01.
   Corrected again (RS7): the test said LE 61, which the shipped curve cannot give at HQ 60. The reset and every
   other number are unchanged. LE 61 needs GetCellHQ = 44 (60 + 4 × 0.25, exact); test 11's LE 61 and LE 75 come
   from HQ 44 and HQ 100.)*
9. **Buckets**: age 35 → 5; 17 → 19; 18 → 0; 22 → 0; 76 → 18; 77 → 19; 0 → 19.
10. **Age groups**: one bucket, age 30 (b = 3), ageCount 200, EQ 50 (ageEqSum 10000) → `0xCA545CAB + g` =
    12, 48, 54, 40, 24, 14, 8, 0, 0 and `0xCA545CCA + g` = 50 for g = 0..6, 0 for g = 7, 8. Each n needs the
    float32 rounding of §5.6 step 4: 1200 × 0.01f rounds to 12.0 (unrounded it is 11.99999973 → 11); likewise
    4800 → 48.0, 5400 → 54.0, 4000 → 40.0, 2400 → 24.0, 1400 → 14.0, 800 → 8.0.
11. **Average LE**: tracts (total 100, LE 61) and (total 300, LE 75) → `0x2A5E7009` = 28600 ÷ 400 = **71**.
12. **Per mille**: population 2000, hospital covered 1 → `0x4A8A3FE2` = **0** (0.5 rounds to even); school
    covered 3 → `0x4A8A3FE5` = **2** (1.5 → 2). Population 0 → both 0, `0x4A8A3FE2` recorded first.
13. **Quotient wrap**: pops 40000, 30000, 0, values 10, 10, 10 → Σp = 4464 → 700000 / 4464 ≈ **156.810**.
14. **Pollution byte**: 16 cells of air source 300 → p_0 = 300 mod 256 = **44** → air term ≈ 0.0085938, so
    hqLoss ≈ 0.2085938.
15. **Life expectancy**: GetCellHQ = 100 → slot 29 = 75; HQ 22 → 55 (55.5 truncated).
16. **Workforce**: LE 75 → slot 30 writes **0.5** through the hidden pointer and pops 12 (50 × 0.01f rounds to
    exactly 0.5); LE 61 → ≈ 0.444.
24. **Percentage getter** (RS1): the reset percents with the shipped 80 read as `0x3F4CCCCC` (0.79999995); with the
    property absent they stay 0.7 (`0x3F333333`). Strike efficiency 20 → `0x3E4CCCCC`, max funding 120 →
    `0x3F999999`.
17. **New age**: slot 42(3) with the exemplar → 35; without → **25**; returns AL only.
18. **NewMonth**: state 10 → state 0, dirty 0, true. State 5 → state 5, dirty 1, true.
19. **NewDay unfinished**: a state-4 step whose time budget runs out returns false and resumes at the next tract.
20. **Read**: version 2 record with state 4 → state 11, leSum 0, running population 0, iterator fresh at (0, 0);
    version 7 with state 1 → 13, 2 → 14, 3 → 15. Any version with a complete record → `Read` returns true. A
    record cut short (stream error) → false, with the simulator left as constructed and cityStarted false (RS9,
    *[choice]*).
21. **Init after load**: a loaded department block with the strike flag set → after `Init` the flag is 0.
22. **Slot ABI**: slot 10 pops 4 and returns the pointer in EAX; slot 23 sets three out-references and raises each
    grid's count by one; slot 21 returns a new grid per call (two calls → two distinct objects).
23. **Buzzards**: HQ after decay 1.5 (< 2), pop > 0 → exactly one `RandomDoubleRangeUniform(0, 100)` draw on the
    member generator; HQ exactly 2.0 → no draw; pop 0 → no draw.
