# Growth developer (`cSC4GrowthDeveloper`) — behaviour spec

Status: M4 round 5, written from the Windows 1.1.641 executable. `docs/subsystems/development.md` §4 (mostly Mac)
was the starting point. Every fact below was checked on Windows; corrections to the earlier notes are in §10.3.

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

Shared material cited and not repeated: streams and references (foundations §2, §2.7), class store (§3), response
curves (§4, loading patterns §4.1, evaluation §4.3), messages (§5), random generators (§6), slot-order rule (§16),
framework slot tables (§17), x87 precision (§18); scheduler levels, finished semantics and the adaptive slicing
helper (`simulator.md` §4, §5.3, §5.5); grid classes, geometry, persistence, ownership rule and map utilities
(`simgrid.md` §2–§7). Specified in parallel and cited for their side of each contract: `building-development.md`
(BDS), `tract-developer.md`, `lot-developer.md`, `zones.md`.

Notation: `r24(e)` = one rounding to float32 (foundations §18); `ftol(e)` = the CRT float→int64 truncation toward
zero (`0x009EEF04`; NaN gives low bits 0); `low16(i)` = the low 16 bits of an integer, reinterpreted as `int16`;
`rhe(e)` = round half to even. "Tract" = an element of a grid; tract size S is in cells (simgrid §3).

---

## 1. Role and identity

One growth developer exists per **developer exemplar** (12 in the shipped game: R$, R$$, R$$$, CS$, CS$$, CS$$$,
CO$$, CO$$$, I-R, I-D, I-M, I-HT). Each one:

- computes a **desirability** map for its developer type from about twenty input maps (land value, slope, wealth
  proximity, parks, pollution, schools, hospitals, crime, commute length, commercial traffic, landmarks, special
  buildings, transient aura) through response curves, in a sliced state machine started once a month by the BDS;
- keeps per-tract counts of zoned cells that fit it and the **maximum capacity** they could hold (updated on every
  zoning change), and the **built capacity** of its lots (updated when lots become occupied or vacant);
- turns built capacity × desirability into a per-tract **population** map, and hands every tract to the tract
  developer (`UpdatePopulation`);
- produces the **growth list** (tracts above the growth threshold, most desirable first, each with a maximum growth
  stage taken from city-size curves) and the **decline list** (tracts below the decline threshold), which the BDS
  and the tract developer consume (`building-development.md`, `tract-developer.md`).

| Item | Value |
|---|---|
| Class ID | `0x0990C138` (`GetGZCLSID` `0x006B3790`) |
| `cISC4GrowthDeveloper` | IID `0xE7CDD662`, primary vtable `0x00AB1488` |
| `cIGZMessageTarget2` | IID `0x452294AA`, vtable `0x00AB1474` (sub-object at +4) |
| `cIGZSerializable` | IID `0xE4FDA3D4`, vtable `0x00AB145C` (sub-object at +8) |
| `cIGZUnknown` | IID `1` → primary |
| Object size | `0x13C` bytes, allocated with the global heap (C) |
| Tuning exemplar | its own developer exemplar: type `0x6534284A`, group `0x67CD5FA1`, instance = developer type, in `SimCity_1.dat` (one per type; §4.8) |

`QueryInterface` (`0x006B37A0`) answers the four IIDs above (AddRef, AL = 1) and otherwise returns false without
touching the out pointer. All three sub-objects share one reference count. `Release` (`0x006B3D00`): count > 1 →
decrement and return the new count; otherwise call `Shutdown` (slot 4) and the deleting destructor (class slot 31),
and return 0.

### 1.1 Creation *[compat]*

- **Not a city component.** The BDS owns the developers in a map keyed by developer type
  (`building-development.md`). For a new city its `CreateDeveloper` (`0x00699030`) allocates the object with the
  global heap and calls the constructor **directly** (`0x006B7310`, call at `0x00699207`), then `AddRef` and
  `Init(propertyHolder)` (slot 3) with the developer exemplar. If a developer of that type already exists it only
  calls `Init(propertyHolder)` on it.
- The COM director registers a factory for `0x0990C138` (`0x006B7870`, registered at `0x00460448`): the class
  store uses it when a save holds developer references (the BDS writes its developers by reference, IID
  `0xE7CDD662`, BDS `Read` `0x0069AD53`). After loading, the BDS's `CreateDeveloper` finds the loaded object and
  calls `Init` on it. A loaded developer has **two** references: the BDS map's and the load's object table's
  (foundations §2.7). Until the city is saved, the BDS `Shutdown`'s `Release` is therefore not its last one: the
  developer is destroyed when the loaded segment closes, after the whole city `Shutdown` (building-development §1.4,
  DG39).
- Consequence (§9): registering `0x0990C138` in a plugin replaces only the developers created from saves; a new
  city still gets original objects.

### 1.2 `Init(cISCPropertyHolder* holder)` → bool (slot 3, `0x006B8510`)

1. If already initialised → return true (nothing is re-read; `Init` is idempotent).
2. Take references (AddRef) to the services that are still null, in this order: city, BDS, occupant manager
   (city slot 34 `GetOccupantManager`), zone manager, land value simulator, traffic simulator. If any of them is
   null → return false. References already taken are kept, so a later `Init` resumes *[compat]*.
3. Store the holder (AddRef; **no null check**: null crashes *[quirk]*; *[choice]* FreeCity4 returns false) and
   read property `0x21` (u32) as the developer type if present.
4. Create each grid that is still null (a loaded object already has five of them), each through the game's grid
   class, `AddRef`, `Init`, `SetTractSize(current tract size)` and `SetInstanceID(developerType + offset)`:

   | Grid | Element | Instance ID | Saved | Getter (slot) |
   |---|---|---|---|---|
   | zoned-cell count | `uint8` | type + `0x01000000` | yes | none |
   | working desirability | `int16` | type + `0x02100000` | **no** | none |
   | desirability (published) | `int16` | type + `0x02000000` | yes | `GetDesirabilityMap` (13) |
   | population | `int16` | type + `0x03000000` | yes | `GetPopulationMap` (15) |
   | built capacity | `uint16` | type + `0x05000000` | yes | `GetCapacityMap` (14) |
   | maximum capacity | `uint16` | type + `0x06000000` | yes | `GetMaxCapacityMap` (16) |

   The "current tract size" is the constructor value 64 at this point; step 7 sets the tuned size.
5. Set the update state to 0 (**always**, also for a loaded object).
6. Create the tract scanning iterator if missing (simgrid §7.4; global heap, `0x20` bytes).
7. Read the tuning (§4.8; `ReadSimulatorTunableValues` `0x006B78B0`).
8. If the message server reference is null: take it, then `AddNotification(this as target, 0xE8BDA2E7)` and
   `AddNotification(…, 0x07699455)`. A null message server → return false.
9. Simulator null → return false. Otherwise `AddAgent(target, 10, "cSC4GrowthDeveloper", 2)` then
   `AddAgent(target, 4, "cSC4GrowthDeveloper", 2)` (flags 2: enabled only when the city is established,
   `simulator.md` §4).
10. Mark initialised; return true.

### 1.3 `Shutdown()` → true (slot 4, `0x006B59A0`)

If not initialised → true. Otherwise: clear the initialised flag; `RemoveAgent(target)` on the city's simulator
(city slot 50, then simulator slot 20, all levels); remove both notifications and release the message server;
free the iterator; for each of the six grids `Shutdown()` then `Release()`; release the holder, the source demand
node, the BDS, land value, traffic, zone manager, occupant manager and city; delete all 19 desirability curves.
The stage curves, the special-building list, the growth and decline lists and the maps stay until destruction.

---

## 2. State

| State | Initial (constructor `0x006B7310`) | Saved | Notes |
|---|---|---|---|
| initialised flag | false | no | §1.2 |
| developer type (`0x21`) | 0 | no | tuning |
| target zone (1 R, 2 C, 3 I) | 0 | no | tuning `0x87CD6300` |
| purpose, wealth | 0, 0 | no | tuning `0x87CD6332`, `0x87CD6330` |
| profile field 2 | 0 | no | never set; copied into `GetConstructionInfo` (§6.2) |
| land value range min, max (u8) | 0, 0 | no | tuning `0x87CD6333` |
| use intrinsic land value (bool) | false | no | tuning `0x87CD633F` |
| tract size (u8) | **64** | no | tuning `0x87CD6310` (§4.8) |
| 19 desirability curves | absent | no | §4.2 |
| stage curves (vector of curves) | empty | no | §5.5 |
| baseline, growth threshold, decline threshold | 80, 80, 80 | no | |
| phantom-demand flag | **true** | no | reset to false by every tuning read |
| caps: power, water, fire, park (u8) | 0, 3, 3, 6 | no | getters only |
| abandonment thresholds (map int → float) | empty | no | §4.8 |
| monopoly-piece use | 1 | no | |
| style percentages (map u32 → `SC4Percentage`) | empty | no | |
| subjective min / max (vector of int32 each) | empty | no | |
| **update state** | 0 | no (Init sets 0) | 0 idle, 1..20 (§3.2) |
| update start day | 0 | no | Julian day when state 1 ran |
| scanning iterator | none | no | over the working-desirability tract range |
| special-building list + cursor | empty | no | copy of the BDS list, one reference per element |
| growth list, decline list | empty | no | vectors of `GrowthItem` (§5) |
| last phantom sum (int32) | 0 | **yes** | §5.3 |
| total capacity (int32) | **not initialised** (DG1) | **yes** | §6.3. The constructor writes `+0xEC`…`+0x104` and `+0x10C` but skips this field at `+0x108`, so a new city's developers start from whatever the block held (an operator-new block of `0x13C` bytes, `0x005E55E0`, created at `0x006991F4`). A load overwrites it. *[choice]* 0 |
| six grids | null | five by reference | §1.2 |
| source demand node | null | no | tuning `0x87CD6301` |

After a load nothing is recomputed: the lists are empty and the state is 0, so `GrowthAndDeclineListsAreValid`
returns **true** with empty lists until the next monthly update finishes *[compat]*.

---

## 3. Scheduling and messages

### 3.1 Agent registration and messages handled

Levels 10 (idle) and 4 (day), flags 2 (§1.2). `DoMessage` (`0x006B7030`, target sub-object):

| Message | Action | Result |
|---|---|---|
| `0x6695681A` SimIdle | `DoIdle(data3)` (budget in ms, `simulator.md` §5.4) | true (ignored) |
| `0x66956814` NewDay | `DoIdle(5)`, then the "not finished" rule below | see below |
| `0x07699455` ZoningChange | `UpdateMaxCapacity(rect = void2)` (§6.1); data3 (zone type) is ignored | true |
| `0xE8BDA2E7` StateChanged | lot occupancy change (§6.3) | true |
| anything else | nothing | true |

**NewDay "not finished" rule** *[compat]*: after `DoIdle(5)`, with `elapsed = simulator.GetSimDateNumber() −
startDay` (unsigned 32-bit), return **false** when `state ≠ 0` and `state < elapsed` (unsigned compare);
otherwise true. The day level then repeats NewDay on this developer (each repeat runs `DoIdle(5)` again) until the
rule passes (`simulator.md` §5.3). So an update must advance at least one state per elapsed day: on the start day
(elapsed 0) it never blocks; from day 21 on it blocks the day until the update finishes. `development.md` states
the same comparison as `today − start > state`.

Cadence: the BDS calls `StartMapUpdate` on every developer once a month (`building-development.md`,
`DoMonthlySimulation`) and then waits (`IsMapUpdateFinished`) before planning. One `DoIdle` call advances exactly
one state, except states 15 and 18 which may stay (§3.2). Idle calls arrive every frame, so an update normally
finishes within a day.

Ordering with other simulators *[inferred]*: the update reads the source maps as they are at each state's call
(land value, residential proximity and grades, aura, pollution, crime, traffic, BDS special buildings); no
snapshot is taken. The population pass (state 18) runs in the same calls as other day/idle agents, so residential
(which copies the population maps daily, `residential.md` §5.5) sees whichever tracts were done.

### 3.2 `DoIdle(ms)` state machine (`0x006B5F20`, jump table `0x006B6FDC`)

States outside 1..20 do nothing. Each step below ends the call unless noted.

| State | Work | Next |
|---|---|---|
| 1 | if initialised: `startDay := simulator.GetSimDateNumber()` | 2 (else stay 1) |
| 2 | every working-desirability tract := `low16(baseline)` | 3 |
| 3–14, 16 | one input layer each (§4.3) | state + 1 (14 → 15 after copying the special-building list, §4.4) |
| 15 | special buildings (§4.4): apply at most **one** effect per call | 16 when the list is exhausted (in the same call), else stay |
| 17 | publish and build the growth list (§5.1) | 18 |
| 18 | population pass, time-sliced (§5.2) | 19 when all tracts are done, else stay |
| 19 | phantom demand (§5.3) | 20 |
| 20 | sort, decline list, filter, sizes and stages (§5.4–§5.5) | 0 |

`StartMapUpdate` (slot 27) sets state 1 only if it is 0. `IsMapUpdateFinished` (28) = `state == 0`.
`GrowthAndDeclineListsAreValid` (10) = `state < 17` (signed).

---

## 4. Desirability *[compat]*

### 4.1 Model

The working grid W (`int16`, tract size S = tuning `0x87CD6310`, shipped 4) starts at the baseline (state 2).
Each layer adds `contribution = curve(input)` per tract, rounded and wrapped as stated per layer. **No clamping
anywhere**: every addition wraps in 16 bits *[quirk]*. State 17 copies W into the published grid D.

### 4.2 Curves

All 19 desirability curves use the **delete** pattern (foundations §4.1): present property → create the curve if
needed (global heap, `0x10` bytes, constructor `0x007D0410`) and load it with the property-pointer loader
`0x005F4620`; missing property → delete the curve (its point buffer with B, the object with C). An absent curve
skips its layer. A present property with fewer than 2 values gives an empty curve, which the evaluator
(`0x005F4250`) cannot handle (foundations §4.3) *[quirk]*; *[choice]* treat an empty curve as absent.

Evaluation is foundations §4.3 (three f32 operations). Shipped: all 19 properties are present on all 12
developers (§4.8).

### 4.3 Layers (states 3–14, 16)

"Walker" = `ProcessTwoGridsWithCallback<int16, T>` (simgrid §7.3) with W as grid A and the source as grid B. The
callback adds to the W tract:

- integer sources (`uint8`, `int8`, `int16`): `W := low16(W + low16(ftol(curve(float(v)))))`, where v is the
  value the walker passes (in the "A coarser" case the floor mean `(sum >> d) >> d` over the covered source
  tracts, signed 32-bit sum);
- float sources: equal sizes → `curve(v)` with v unchanged; A coarser → a signed 32-bit accumulator, starting at
  0, updated as `acc := ftol(r24(float(acc) + v))` for each covered value (walker `0x006B4120`), then the floor
  mean `(acc >> d) >> d`, converted to float; A finer → `float(ftol(v))`, i.e. the
  source value is **truncated toward zero** before the curve *[quirk]*.
  **Visit order of the float accumulator** *[compat]* (`0x006B41ED`–`0x006B42B5`): for W tract `(x, z)` the
  covered source tracts are visited with source x **outer** (`x·2^d … x·2^d + 2^d − 1`) and source z **inner**
  (`z·2^d … z·2^d + 2^d − 1`), the same order as the integer walkers. The order matters here because every step
  truncates (test 17). W tracts themselves are visited x outer, z inner. No shipped float source has smaller
  tracts than W (slope 8, school and hospital 4), so this case only arises with non-shipped tract sizes.

| State | Curve (property) | Source (interface, slot) | Element | Shipped source S | Walk vs W (S = 4) |
|---|---|---|---|---|---|
| 3 | land value `0x87CD6340` | land value: `0x87CD633F` ? intrinsic map (slot 7) : total map (slot 8) | `uint8` | 2 | W coarser, 2×2 floor mean |
| 4 | slope `0x87CD6398` | land value slot 9 (slope map) | `float` | 8 | W finer: truncated slope |
| 5 | R$, R$$, R$$$ proximity `0x87CD6343/44/45` | residential slot 5 `GetProximityMap(1/2/3)`, in that order, each only if its curve exists | `uint8` | 1 | 4×4 floor mean |
| 6 | CO$$, CO$$$ proximity `0x87CD6391/92` | commercial simulator slot 5 `GetProximityMap(0x3320 / 0x3330)` | `uint8` | 1 | 4×4 floor mean |
| 7 | park `0x87CD6341` | aura slot 14 `GetParkMap` | `int16` | 4 | equal |
| 8 | pollution i = 0..3: `0x87CD634B/4C/4D/4E` | pollution slot 5 `GetPollutionGrid(i)`: air value, water value, garbage value, **radiation source** (pollution §2, §8) | `int16` | 2, 2, **1**, 2 | means over 2×2, 2×2, 4×4, 2×2 |
| 9 | school `0x87CD6348` | residential slot 39 (school grade map) | `float` | 4 | equal, float unchanged |
| 10 | hospital `0x87CD6349` | residential slot 40 (hospital grade map) | `float` | 4 | equal |
| 11 | crime `0x87CD634A` | crime slot 5 `GetCrimeGrid` | `uint8` | 2 | 2×2 floor mean |
| 12 | commute (trip length) `0x87CD6347` | traffic slot 13 `GetTripLengthMap` | `uint8` | cells | **own loop**, below |
| 13 | commercial traffic `0x87CD6346` | traffic slot 9 (SDK `GetCommercialTrafficMap`) | `uint8` | 4 | equal |
| 14 | landmark `0x87CD6399` | aura slot 15 `GetLandmarkMap` | `int16` | 4 | equal |
| 16 | transient aura `0x87CD6397` | aura slot 13 `GetTransientAuraGrid` | `int8` | 4 | equal |

Walkers: `<int16,uint8>` `0x006B3E40` with callback `0x006B3AB0`; `<int16,float>` `0x006B4120` with `0x006B3AE0`;
`<int16,int16>` `0x0068C270` (shared with the aura) with `0x006B3B00`; `<int16,int8>` `0x006B4430` with
`0x006B3B30`.

**Garbage** *[quirk]*: the garbage value grid stores the active-landfill flag in bit 15 (pollution §2), so a cell
on an active landfill reads as a large negative `int16`, and a 4×4 mean containing it is negative. The shipped
curves clamp below their first x (foundations §4.3), so the R$ garbage curve (`0x87CD634D`, first point
`(0, 25)`) gives **+25** for such a tract.

**State 12, commute** (only if the curve exists), for each W tract `(x, z)`: with `S = W.GetTractSize()` (the
size, not the shift),

```
m := 255
for i in 0..S−1, j in 0..S−1:  v := tripLengthMap.GetCellValue(x·S + i, z·S + j)    (cell coordinates)
                                 if v ≠ 0 and v < m: m := v
W(x, z) := low16( ftol( r24( float(W(x, z)) + curve(float(m)) ) ) )
```

Unlike every other layer, the sum is truncated, not the curve output *[compat]* (test 6). A tract with no
non-zero commute cell uses m = 255.

### 4.4 Special buildings (states 14 → 15)

At the end of state 14 the developer replaces its private list with a copy of the BDS's special-building list
(BDS slot 44 `GetSpecialBuildings`, `building-development.md`; each element `AddRef`ed, the old elements
released) and puts its cursor on the first element.

State 15, from the cursor, for each occupant (the cursor advances past each one):

1. Properties from the occupant's property holder (occupant slot 6): `0x4A5F7C4B` (u32 array, unnamed: developer
   types) and `0x4A5F7315` (f32 array, unnamed: three values per developer type). Let i = the first index with
   `types[i] == developerType`. If either property is missing, or no index matches, skip the occupant.
2. `r := r24(values[3i] / float(S))` (radius in tracts; S = W's tract size, an int converted exactly);
   `c := values[3i+1]`; `o := values[3i+2]`. There is **no bounds check** on the float array *[quirk]*;
   *[choice]* skip the occupant when the array is shorter than `3i + 3`.
3. The occupant's position (occupant slot 8, `GetPosition`) → cell `(cx, cz)` with city slot 95
   `PositionToCell(x, z, &cx, &cz)` → tract `(cx >> shift, cz >> shift)` with W's shift.
4. `AddRadiusEffect<int16>` (`0x005E4500`, simgrid §7.1) on W's data with W's counts, that centre tract, radius r
   (float, tracts), centre value c, outer value o. This helper **clamps** to the `int16` range.
5. Stop the call after the first applied effect (state stays 15) unless the cursor is now at the end, in which
   case go to 16 in the same call.

`development.md` called the triple "(magnitude/tractSize, radius, falloff)"; it is (radius in cells, centre value,
outer value).

### 4.5 Precision summary

- Curve inputs: `float(int)` for integer sources (exact for |v| < 2²⁴); float sources passed unchanged or truncated
  as in §4.3.
- Each curve output is truncated with `ftol` and added as a 16-bit integer, except state 12 (one f32 add, then
  `ftol`) and state 15 (the helper's own rounding, simgrid §7.1).

### 4.6 `EstimateDesirability(x, z)` → int16 (slot 29, `0x006B4DF0`) *[compat]*

A one-cell estimate used by other code (callers not identified *[inferred: query/placement tools]*):

```
acc := baseline                                         (int32)
for each present curve in this order, v := source.GetCellValue(x, z) (slot 7, cell coordinates):
    land value (intrinsic or total per 0x87CD633F), slope, R$, R$$, R$$$, CO$$, CO$$$, park,
    pollution 0..3, school, hospital, crime, commercial traffic, landmark, transient
    acc := acc + ftol(curve(input))                     (int32, wraps)
return low16(acc)   in AX
```

Differences from the update *[quirk]*, all reproduced:

- No commute layer, no special buildings.
- Single-cell values instead of the walker's tract means.
- The **input conversion** differs: land value, R/CO proximity, crime and traffic as unsigned bytes; slope,
  school and hospital as float (unchanged); pollution as **unsigned** 16-bit (`movzx`), so the landfill bit gives
  32768 + garbage; park, landmark (both `int16` grids) and transient (`int8`) as the **low byte, unsigned**
  (−1 reads as 255, 300 as 44).
- It also calls `GetTractShift` on D and ignores the result.

The upper half of EAX is garbage; callers must use AX only. *[choice]* A replacement returns the value
sign-extended to 32 bits.

### 4.7 Grids read and their owners

All source grids are read through `GetGridData` (the walkers, raw `vector<vector<T>>`, simgrid §6.3) or
`GetCellValue`. A FreeCity4 replacement of any source simulator must hand out game grids (simgrid §6.4) with the
element types above.

### 4.8 Tuning *[compat]*

Read by `ReadSimulatorTunableValues` (`0x006B78B0`) from the developer exemplar (the `Init` holder), in this
order. Helpers: property value getters `0x005FD480` (u32), `0x005FD3C0` (u8), `0x005FD390` (bool), `0x005FD450`
(s32), `0x005FFC00` (vector<int32>); "raw" = the property's data pointer.

| Property | Type | Meaning | Code default | Shipped |
|---|---|---|---|---|
| `0x00000021` | u32 | developer type | keep | = instance |
| `0x87CD6300` | u8 | target zone 1 R / 2 C / 3 I | keep | R 1, C 2, I 3 |
| `0x87CD6301` | u32 | source demand: release the old node, then `demandSim.GetDemand(id, 0x20000)` (city-total node), AddRef | keep | = developer type |
| `0x87CD6310` | u8 | tract size S; if it differs from the stored size: `SetTractSize(S)` on all six grids, then iterator rectangle `[0, max(countX−1, 0)] × [0, max(countZ−1, 0)]` of W | keep (64) | 4 |
| `0x87CD6330` | u8 | wealth | keep | 1/2/3 |
| `0x87CD6332` | u8 | purpose | keep | R 1, CS 2, CO 3, I 5..8 |
| `0x87CD6331` | u8 | **not read** | — | 1 |
| `0x87CD6333` | raw bytes 0, 1 | land value range min, max | keep | 0, 255 |
| `0x87CD633F` | bool | use the intrinsic (not total) land value map | false | false |
| 19 curves | f32 pairs | §4.2, §4.3 | delete | all present |
| `0x87CD6357` | s32 | baseline | **80** | §4.9 |
| `0x87CD6355` | s32 | growth threshold | **80** | 50 |
| `0x87CD6354` | s32 | decline threshold | = growth threshold | 50 |
| `0x87CD6356` | bool | phantom demand (§5.3) | **false** | absent |
| `0x87CD6360` | u8 | number of stage curves k (vector resized to k) | 0 | R/C 8, I 3 |
| `0x87CD6361 + i` (i < k) | f32 pairs | stage-i curve (§5.5): present → loaded in place (`0x005F4620`); missing → rebuilt from the single point (0, 0) (`0x005F4480`), i.e. constant 0 | (0, 0) | all present |
| `0x87CD6373` | u8 | power cap | keep (0) | 0 |
| `0x87CD6370` | u8 | water cap | keep (3) | 3, except R$$$/CS$$$/CO$$$/I-HT 0 and I-D/I-M 1 |
| `0x87CD6371` | u8 | fire cap | keep (3) | 9 |
| `0x87CD6372` | u8 | park cap | keep (6) | 9, except R$$$ 3 |
| `0x87CD6380` | raw f32 [0], [1], [2] | abandonment thresholds for keys 1, 2, 3 (first set to `0.2f` = `0x3E4CCCCD` each); **no count check** *[quirk]*. Keys are density types (§10.1 slot 24). Not saved; nothing in the shipped exe reads them | 0.2 | absent |
| `0x87CD6381` | u8 | monopoly-piece use (SDK enum) | 1 | 0 |
| `0x87CD63A0` | u32 pairs (group, pct) | style map: cleared, then `map[group] := r24(float(pct) × 0.01f)` for i = 0, 2, … while i < count (an odd count reads one value past the end *[quirk]*) | empty | I: `[0x3001, 50, 0x3002, 50]` → 0.5 each; others absent |
| `0xCAB1CE3A` | vector<int32> | subjective minimums | keep | 7 values |
| `0xCAB1CE3C` | vector<int32> | subjective maximums | keep | 7 values |

"keep" = the value stays when the property is missing. The tuning is read only from `Init`; the developer does
not subscribe to the exemplar-changed message.

If `0x87CD6310` is missing, the tract size stays 64 and the iterator rectangle is never set (its default is the
full `int32` range), so state 18 would read far outside the grids *[quirk]*. *[choice]* FreeCity4 always sets
the rectangle after creating the grids.

### 4.9 Shipped developer values (`SimCity_1.dat`, group `0x67CD5FA1`)

| Type | Name (`0x20`) | Zone | Purpose / wealth | Baseline `6357` | Stages | Styles |
|---|---|---|---|---|---|---|
| `0x1010` | Residential $ | 1 | 1 / 1 | 25 | 8 | — |
| `0x1020` | R$$ | 1 | 1 / 2 | −89 | 8 | — |
| `0x1030` | R$$$ | 1 | 1 / 3 | −104 | 8 | — |
| `0x3110` | CS$ | 2 | 2 / 1 | 68 | 8 | — |
| `0x3120` | CS$$ | 2 | 2 / 2 | −3 | 8 | — |
| `0x3130` | CS$$$ | 2 | 2 / 3 | 1 | 8 | — |
| `0x3320` | CO$$ | 2 | 3 / 2 | −27 | 8 | — |
| `0x3330` | CO$$$ | 2 | 3 / 3 | −65 | 8 | — |
| `0x4100` | I-R | 3 | 5 / 1 | −81 | 3 | yes |
| `0x4200` | I-D | 3 | 6 / 2 | 69 | 3 | yes |
| `0x4300` | I-M | 3 | 7 / 2 | 47 | 3 | yes |
| `0x4400` | I-HT | 3 | 8 / 3 | −94 | 3 | yes |

Curves of the R$ developer (points as (x, y)), to fix the scale; the other developers use the same layer set with
their own points (dump of all 12 in the exemplar data):

| Curve | Points (abridged) |
|---|---|
| land value `6340` | (0,10) (15,9) … (195,1) (210,0) … (255,0) |
| park `6341` | (−100,−10) (100,10) |
| R$ / R$$ / R$$$ proximity `6343/44/45` | (0,5) … (255,0); (0,−5) … (255,0); same as R$$ |
| traffic `6346` | (0,2) (256,−2) |
| commute `6347` | (0,10) (255,−10) |
| school `6348`, hospital `6349` | (0,−10) (1,10); (0,−5) (1,5) |
| crime `634A` | (0,25) (15,24) … (255,−25) |
| air `634B`, water `634C` | (0,25) (1000,−25); flat 0 |
| garbage `634D` | (0,25) (241,24) … (4096,−25) |
| radiation `634E` | (0,0) (1,−2000) … (255,−2000) |
| CO proximity `6391/92`, slope `6398`, landmark `6399` | flat 0 |
| transient `6397` | (−127,−100) (127,100) |
| stage 1 `6361` | (0,100) (300,95) (360,80) (432,60) (518,45) … |
| stage 2 `6362` | (299,0) (300,5) (360,20) (432,35) (518,40) … |
| stage 3..8 `6363`..`6368` | start at 0 below 432, 746, 1547, 3206, 6647, 9571 |
| subjective `CE3A` / `CE3C` | [14, 14, −3, −2, 14, 0, 1] / [21, 21, 7, 4, 21, 1, 9] |

---

## 5. Growth and decline lists, population

`GrowthItem` (12 bytes, *[compat]* layout): `int32 x` (tract), `int32 z` (tract), `int32 maxStage`.

### 5.1 State 17: publish and list

1. D := W for every tract (raw copy through `GetGridData`).
2. Clear the growth list (keeps capacity). For x = 0..countX−1 (outer), z = 0..countZ−1 (inner), counts of W: if
   the zoned-cell count of `(x, z)` is > 0 (unsigned byte), append `{x, z, 1}`.
3. State 18; reset the scanning iterator to (0, 0) (simgrid §7.4; done at once if (0, 0) is outside its
   rectangle).

### 5.2 State 18: population (time-sliced)

Slicing: the shared adaptive helper with budget `ms` and this class's process-wide step `0x00B49098` (miss counter
`0x00B4909C`, guard `0x00B490A0`) (`simulator.md` §5.5). The clock is the processor's time-stamp counter converted with
the simulator's `MillisecsToCycles` (§5.5); a replacement may use any monotonic real-time clock such as
`QueryPerformanceCounter` (DG2). For fixed inputs, slicing changes only how much of the pass runs per call.
In a running city, different slice boundaries can straddle other deliveries that change those inputs (§5.2.1).
The deadline check comes **before** each tract. The tract developer is fetched once per call (BDS slot 35
`GetTractDeveloper`).

For each `(x, z)` from the iterator (x inner, z outer) while time remains:

```
d   := r24(float(W(x, z)) × 0.01f)            (dword constant 0x3C23D70A)
if d < 0: d := 0        else if not (d ≤ 1): d := 1       (NaN would stay; W is an integer, so never)
p   := r24(float(capacity(x, z)) × d)          (capacity: uint16)
population(x, z) := low 16 bits of the float32 bit pattern of r24(p + 12582912.0f)
                  = rhe(p) for 0 ≤ p < 2^22, stored in the int16 grid (values > 32767 read back negative)
tractDeveloper.UpdatePopulation({this, x, z, stage = uninitialised})       (tract developer slot 7)
```

When the iterator is exhausted → state 19. Out of time → stay in 18 (the iterator keeps its position).

W, capacity and population are indexed with the same `(x, z)` (all six grids share S). The `GrowthProfile`'s
fourth field (`uint8` stage) is never written *[quirk]*; `UpdatePopulation` ignores it
(tract-developer.md §4.18). *[choice]* FreeCity4 passes 0.

#### 5.2.1 Capturing state-18 reads and synchronous effects (GD-GC29-1) *[compat]*

Use the original developer's primary `cISC4GrowthDeveloper` pointer as the base for the following Windows
1.1.641 fields. The message-target-2 pointer is base +4; `DoMessage` subtracts 4 before calling `DoIdle`
(`0x006B70B4`–`0x006B70B7`). Do not apply the primary offsets to the message sub-object unchanged.

| Offset | State / access contract |
|---|---|
| +`0xD4` | int32 update state; 18 during the population pass, 19 after exhaustion |
| +`0xDC` | pointer to the 0x20-byte scanning iterator; no public iterator getter |
| +`0x110` | W, working int16 desirability grid; no public W getter |
| +`0x114` | D, published int16 desirability grid; public slot 13 (`+0x34`) returns D, not W |
| +`0x118` | int16 population grid; public slot 15 (`+0x3C`) |
| +`0x11C` | uint16 built-capacity grid; public slot 14 (`+0x38`) |
| +`0x120` | uint16 maximum-capacity grid; public slot 16 (`+0x40`); not read by the state-18 population formula |

The iterator has a state byte at +`0x00` (0 fresh, 1 running, 2 done), int32 current x/z at +`0x04`/+`0x08`,
step at +`0x0C`, and inclusive rectangle x0/z0/x1/z1 at +`0x10`/+`0x14`/+`0x18`/+`0x1C`.
`GetNext` (`0x0060C8D0`) returns the current pair on the fresh call and advances **before returning** on
later successful calls; x runs inside z (simgrid.md §7.4). Thus an entry cursor is the last returned pair
when running, not an unconditionally next-to-process pair. State 17 resets it to fresh (0,0) at
`0x006B68BE`–`0x006B68F4`. A deadline return need not consume another pair.

**Per-tract boundary.** In state 18, `DoIdle` obtains the tract developer once and the grid-data objects
for capacity, population and W (`0x006B693D`–`0x006B6975`). Its successful iterator return at
`0x006B69C5` supplies the pair about to be processed. Capture that pair and the actual W **signed word**
and capacity **unsigned word** used at `0x006B69E4` and `0x006B6A27`. There are no intervening external calls
between these two loads and the population store at `0x006B6A4D`, so a capture just after the successful
iterator return can obtain both inputs for that tract. The raw grid data is a vector of column vectors:
outer element stride 12 bytes at x, word offset 2×z in that column (simgrid.md §6.3).
Retain developer subtype, delivery/date and pair order with each record. Do not substitute a capacity
snapshot taken after the original's whole delivery, or after the whole state-18 pass.

After computing and storing the population word, the original forms the profile {developer, x, z, unwritten
stage} and synchronously calls tract-developer slot 7 at `0x006B6A65`, **before** requesting another pair
(`0x006B6A68` returns to the timer/iterator loop). A shadow can replay the captured pairs in this order,
using their captured W/capacity regardless of its own adaptive clock. It may keep the callback read-only
with respect to the game; it must still model any per-lot effects it compares, from independent lot inputs.
Record the original population immediately after its store and after callback return to separate numeric
disagreement from subsequent mutations. Neither final D nor final capacity is a record of the earlier reads.

**What the native callback can change.** `cSC4TractDeveloper::UpdatePopulation` (`0x00706BC0`,
tract-developer.md §4.18) reads published D and the tract's intersecting lots. For each qualifying lot it
calls `SetPopulation` (slot 55). Native `cSC4Lot::SetPopulation` (`0x006C1940`) clamps to available lot
capacity, assigns one per-lot population-map entry and recomputes that lot's cached occupant wealth
(`0x006C1A10`–`0x006C1A1A`, lot-records.md §3.2.1–§3.2.2). It does **not** call the growth developer's
capacity/population updates or post a change message. The wealth recomputation (`0x006C1430`–`0x006C15CF`)
updates the lot's cache, without a grid-update callback. Therefore this native call does not synchronously
change a later tract's W, capacity or population grid, nor overwrite the just-stored tract population.
It can change the lot populations/wealth a later tract's **lot-level** callback sees, especially for a
lot intersecting more than one tract. Do not treat that as a rewrite of the later tract's grid capacity.
Other deliveries between slices may update the grids through the ordinary capacity/population contracts.

**Worked slice case.** Let tract (5,24) have W = 100 and capacity = 85 when the original consumes it;
the formula writes population 85. Between deliveries, the host adds capacity 141 with no population delta,
so capacity becomes 226. A shadow whose earlier slice stopped before (5,24) and reads live capacity on its
next slice writes 226. Both finish with D = 100 and capacity = 226, while populations differ 85/226.
Replaying the captured pair {W = 100, capacity = 85} writes 85 under any slice partition. This is a controlled
test of the boundary, not a reconstruction of the historical city's unrecorded inputs. A separate callback
case should verify that native lot-population/wealth changes leave the next tract's grid words unchanged.

**Remaining observation.** A clean run through the reported 2027-12-03 residential pass must record every
original successful state-18 pair, W/capacity at these loads, population before/after its store and callback,
and entry/exit cursor and state for each delivery. Follow any intervening grid-changing messages with their
date/order. This distinguishes changed inputs between unequal slices from arithmetic or callback drift at
(5,24). The corrected-precision 263/263 and isolated 105/105 exact lines end before that date; their later
corrupt/crash outcomes do not validate this case or prove it fixed.

### 5.3 State 19: phantom demand

```
sum := 0
if phantomFlag:
    for each growth item (all zoned tracts, order of §5.1):
        g := growthThreshold − W(x, z)                       (int32)
        if g > 0:
            f := r24(float(g) × 0.01f);  if not (f ≤ 1): f := 1
            sum := sum + ftol(r24(float(population(x, z)) × f))      (population read as int16)
delta := sum − lastPhantomSum
if delta ≠ 0:
    sourceDemand.AddToDemandValue(r24(float(delta)))      (cISC4Demand slot 17)
    lastPhantomSum := sum
```

The delta step runs **even when the flag is off** (sum = 0): a save with a non-zero `lastPhantomSum` gives one
correction of `−lastPhantomSum` *[compat]*. Shipped: the flag is off on all developers, so `lastPhantomSum` stays
0. `AddToDemandValue` (`cSC4Demand`, `0x005D4C70`) adds the amount to the node's float32 demand value and returns
**true in AL**; it leaves the x87 stack as it found it (DG1; an earlier version of this section said ST0). The
result is ignored.

### 5.4 State 20: sort, decline list, filter

1. **Sort** the growth list with the game's `std::sort` (`0x006B5EC0`; STLport introsort, threshold 16, median of
   first / middle / last−1, Hoare partition, depth limit 2·⌊log₂ n⌋ then heap sort, final insertion sort) and the
   comparator `less(a, b) := W(a) > W(b)` (signed `int16`, `0x006B4B50`, reading W through the static
   `0x00B49074`). Result: descending desirability. Ties *[compat]*: with n ≤ 16 the insertion sort keeps the
   §5.1 order; with n > 16 the tie order follows the introsort exactly, so FreeCity4 must use the same algorithm
   (`tract-developer.md` consumes the order).
2. **Decline list**: clear it; walk the sorted growth list from the **end** to the beginning and append each item
   with `W < declineThreshold` (signed). Result: ascending desirability.
3. **Industrial order**: if purpose ∈ 5..8, draw one salt (§7) and sort the decline list with
   `less(a, b) := key(b) < key(a)` (unsigned), i.e. by **descending key**, where
   `key(x, z) := NextRaw()` of a fresh `cRZRandom` seeded with `(((x << 16) + z) << 4) XOR salt` (32-bit
   arithmetic; comparator `0x006B3820`). This replaces the desirability order with a reproducible pseudo-random
   order for one update. The sort is the **same routine as step 1** *[compat]*: state 20 calls `0x006B5EC0` twice
   (`0x006B6BCE`, `0x006B6C53`), and the routine takes the comparator as a function-pointer argument, so the
   decline list gets the identical STLport introsort (threshold 16, depth limit 2·⌊log₂ n⌋, final insertion sort)
   with comparator `0x006B3820`. Its input order is the ascending-desirability order of step 2. The comparator
   re-seeds two generators per comparison, so the number of comparisons has no effect on any other generator.
4. **Filter**: remove every growth item with `W < growthThreshold` (signed), keeping the order of the rest.

### 5.5 State 20: city size and growth stages

City size by target zone (`demandSim.GetDemand(id, index)`, then `cISC4Demand::QuerySupplyValue`, slot 8, ST0;
demand keys per `demand.md` §2):

```
n1 := count of neighbours.GetConnectedCities(list, 0)       (slot 21)
n2 := count of neighbours.GetAllConnectedCities(list, 0)    (slot 22)
c  := supply(0x3001, 0)                                     ("C Census", local)
t  := n2 > 0 ? r24(supply(0x3301, 0x10000) × r24(float(n1) / float(n2))) : 0
x  := r24(r24(t × 0.5f) + c)
zone 1 (R):  s1 := supply(0x1001, 0);  size := (s1 < x) ? x : s1          (NaN → s1)
zone 2 (C):  size := x
zone 3 (I):  size := supply(0x4001, 0x20000)
otherwise:   size := 0
size := r24(bds.GetStagePopulationMultiplier() × size)      (BDS slot 53, ST0)
```

The two neighbour lists are temporary (constructed and destroyed here, allocator A per `allocators.md`). A null
demand node crashes *[quirk]*.

**Which queries are made** *[compat]* (`0x006B6CC2`–`0x006B6EA2`). The city-size block runs on **every** state-20
pass, after the filter, whether or not the growth list is empty (n = 0 still makes all the calls below). Calls
in order:

| Target zone | Calls |
|---|---|
| 1 (R) | neighbours slot 21, neighbours slot 22, `GetDemand(0x3001, 0)` + supply, then **only if n2 > 0** (signed) `GetDemand(0x3301, 0x10000)` + supply, then `GetDemand(0x1001, 0)` + supply |
| 2 (C) | the same without the `0x1001` query |
| 3 (I) | `GetDemand(0x4001, 0x20000)` + supply only (no neighbour calls) |
| other | none |

then BDS slot 53 for every zone. With n2 ≤ 0 the `0x3301` node is not looked up at all (`GetDemand` is not
called either), so t = 0 and x = r24(0 + c) = c; a missing `0x3301` node then does not crash.

Stages: with k stage curves (`0x87CD6360`) and n growth items, if k > 0 and n > 0 (when k > 0 and n = 0 the original
still evaluates `stageCurve[k−1](size)` once before testing n, with no effect):

```
acc := stageCurve[k−1](size);  s := k;  inv := r24(1.0f / float(n));  i := 0
for each growth item in order:
    p := r24(float(i) × inv)                       (percentile 0, 100/n, 200/n, … below 100)
    while not (p < acc) and s > 1:
        s := s − 1
        acc := r24(stageCurve[s−1](size) + acc)
    item.maxStage := s
    i := i + 100
```

Stage curve j (1-based) gives the percentage of growth tracts allowed to reach stage j; the most desirable tracts
get the highest stages. With k = 0 every item keeps maxStage 1. The test `p < acc` is the x87 C0 flag
(`0x006B6F62`, `0x006B6F9F`), which is also set for an unordered compare, so a NaN `acc` counts as "p < acc": the
loop stops and the item keeps the current s (corrected: an earlier revision said a NaN drives s to 1). Then
state := 0.

### 5.6 Consumers (contracts)

- `GrowthAndDeclineListsAreValid`, `GetGrowthList`, `GetDeclineList` (slots 10–12): the BDS and the tract
  developer read the vectors in place (begin/end pointers, 12-byte items) and do not modify them *[inferred from
  the BDS growth-planning code; see `building-development.md`, `tract-developer.md`]*.
- `GetDesirabilityThreshold` (23) returns the **growth** threshold. The decline threshold has no getter.

---

## 6. Capacity bookkeeping *[compat]*

### 6.1 `UpdateMaxCapacity(rect)` on ZoningChange (`0x006B4B90`)

`rect` = the message's void2: four `int32` `{x0, z0, x1, z1}` in cells (inclusive), from the zone manager
(`zones.md`).

```
s := maxCapacityGrid.GetTractShift()
tx0 := x0 >> s; tz0 := z0 >> s; tx1 := min(x1 >> s, countX − 1); tz1 := min(z1 >> s, countZ − 1)
         (the lower bounds are not clamped; an empty range does nothing)
profile := GetConstructionInfo()                     (§6.2; only its zone code is used)
for tz in tz0..tz1 (outer), tx in tx0..tx1 (inner):
    n := 0;  sum := 0.0f
    for cz in (tz << s)..((tz+1) << s) − 1 (outer), cx in (tx << s)..((tx+1) << s) − 1 (inner):
        zt := (int8) zoneManager.GetZoneType(cx, cz)                   (slot 12, sign-extended)
        if BuildingFitsInZone(profile.zoneCode, zt):                 (BDS static 0x00695390, cdecl)
            n := n + 1
            sum := r24(sum + bds.GetMaxCapacity(zt, purpose, wealth))    (BDS slot 45, ST0)
    zonedCount.SetTractValue(tx, tz, n)                    (uint8: n mod 256)
    maxCapacity.SetTractValue(tx, tz, low 16 bits of bits(r24(sum + 12582912.0f)))   (= rhe(sum) mod 65536)
```

The whole tract is recomputed from scratch, so the result does not depend on the order of zoning messages.

### 6.2 `GetConstructionInfo(ConstructionProfile&)` (slot 9, `0x006B38E0`)

Writes only the first 18 bytes of the caller's profile:

| Field (SDK name) | Value |
|---|---|
| `unknown1` (zone code) | target zone 1 → 1; 2 → 4; 3 → purpose 5 → 7, purpose 6..8 → 8, other purposes → **not written**; other zones → 0 |
| `unknown2` | the never-set profile field (0) |
| `wealth`, `purpose` | the developer's |
| `landValueRangeMin`, `Max` | `0x87CD6333` bytes 0, 1 |

The zone code is the lowest zone type of the developer's family *[inferred: 1 R low, 4 C low, 7 agriculture,
8 industry]*; `BuildingFitsInZone` interprets it (`building-development.md`).

### 6.3 Lot occupancy: StateChanged and `UpdatePopulationCount`

Message `0xE8BDA2E7` (fields per `demand.md` §7): IGZUnknown = the lot (used **as** a `cISC4Lot*` without
`QueryInterface` *[compat]*), data1 = old state, data2 = new state, data3 = building type, data4 = occupant wealth.
data4 is the **wealth**, never the vacate reason: `cSC4Lot::SetState(state, wealth, reason)` (`0x006C3100`) puts the
lot's occupant wealth byte in data4 (the `wealth` argument when the lot enters state 2/3 with a non-zero
`wealth`; the wealth the lot had **before** the call when it leaves 2/3), see `lot-developer.md` §3.2. So a
removal reaches this handler with the wealth the lot had while it was occupied, not a recomputed one. The
IGZUnknown is a copy of the lot made at post time (same section).

```
if data4 ≠ wealth: done
occOld := 2 ≤ data1 < 4;  occNew := 2 ≤ data2 < 4          (unsigned byte compares)
if occOld = occNew: done
if data3 = 0: lot.GetBuildingType(true); done             (result discarded: no update) [quirk]
UpdatePopulationCount(lot, data3, occNew)
```

`UpdatePopulationCount(lot, buildingType, add)` (`0x006B51C0`) updates **built capacity** (the name is the
original's):

```
if bds.GetBuildingProfile(buildingType).purpose ≠ purpose: done      (BDS slot 39; inserts a default profile
                                                                      for an unknown type) [compat]
cap   := lot.GetCapacity(developerType, true)             (lot slot 50, uint16)
delta := add ? cap : −cap
(w, d) := lot.GetSize();  n := w · d                      (lot slot 22, two uint8; n = 0 divides by zero [quirk])
q := delta / n  (C division, toward zero);  r := delta − q · n
rect := lot.GetBoundingRect()                            (lot slot 14, cells, inclusive; normalised so that
                                                          x1 ≥ x0, z1 ≥ z0)
for cz in z0..z1 (outer), cx in x0..x1 (inner):  addTo(cx >> s, cz >> s, q)
(lx, lz) := lot.GetLocation();  addTo(lx >> s, lz >> s, r)   (lot slot 21)
totalCapacity := totalCapacity + delta                    (int32, not clamped)

addTo(tx, tz, v):  if capacity(tx, tz) + v < 0 (int arithmetic): capacity(tx, tz) := 0
                   else capacity(tx, tz) := (capacity(tx, tz) + v) mod 65536
```

`s` = the capacity grid's shift; the grid is written through `GetGridData` without bounds checks. In the negative
branch the original also calls `lot.GetLotConfiguration()` (slot 9) and that object's slot 3 (`GetID`), and
discards the result (dead debug code); a replacement need not call them.

---

## 7. Random numbers *[compat]*

| # | When | Generator | Method | Draws |
|---|---|---|---|---|
| 1 | state 20, purpose 5..8 only, once per update, **before** sorting, even if the decline list is empty | file-local `cRZRandom` at `0x00B49080`, shared by all developers; only the growth developer uses it | `NextRaw()` (`RandomUint32Uniform()`, `0x00913696`) → salt | 1 |
| 2 | each comparison of the decline sort | two temporary `cRZRandom`s, seeded `(((x << 16) + z) << 4) XOR salt` | `Seed` + `NextRaw()` each | no lasting state |

- The file-local generator is seeded at static initialisation (`0x00A74D00`) with the `cRZTime` value XOR
  `0xAC09C0E8` (wall clock; not saved). *[choice]* FreeCity4 takes its seed from the host (foundations §6.3).
- Only the four industrial developers draw. Which salt each one gets depends on the order in which they reach
  state 20, i.e. on the agent order of the idle and day levels (`simulator.md` §4: the original orders agents by
  address) *[compat: draw count]*. *[compat-limit]* A seeded run cannot reproduce which industrial developer gets
  which salt, because the original's agent order follows object addresses; FreeCity4 uses agent registration order
  (simulator.md §4).
- A key seed equal to `0xFFFFFFFF` would make `Seed` use the clock (foundations §6.1) *[quirk]*; it needs
  `salt = (((x << 16) + z) << 4) XOR 0xFFFFFFFF`, probability 2⁻³² per tract and update. *[choice]* FreeCity4 treats it as a
  clock-seeded temporary of the growth developer: seed `derive(0x0990C138 XOR n)`, `n` the number of such clock
  reads by this developer object so far (not saved), foundations §6.3 (RN2).
- No other draws: no C `rand()`, no random iterators, no shuffles (the population pass uses the scanning
  iterator).

---

## 8. Messages posted

None.

---

## 9. Persistence (`cIGZSerializable`, class ID `0x0990C138`) *[compat]*

`Write` (`0x006B3960`) / `Read` (`0x006B39F0`):

| # | Type | Field |
|---|---|---|
| 1 | u16 | version = 1 |
| 2 | s32 | last phantom sum (§5.3) |
| 3 | u32 | total capacity (§6.3; garbage plus the built capacity for a city whose developers were created by the original's BDS, §2) |
| 4 | ref | zoned-cell count grid (`uint8`) |
| 5 | ref | desirability D (`int16`) |
| 6 | ref | population (`int16`) |
| 7 | ref | built capacity (`uint16`) |
| 8 | ref | maximum capacity (`uint16`) |

- References per foundations §2.7, read with IID `0xC9BD939A`; the grids are written by the class store (simgrid
  §5).
- `Read`: version ≠ 1 → false, nothing else read. Otherwise read all fields (each reference read nulls the member
  first, without releasing a previous grid) and return `GetError() == 0`. `Write` returns `GetError() == 0`.
- Not saved: tuning, state, start day, the lists, W (recreated by `Init` and rebuilt by the next update), the
  iterator, the special-building copy.
- After `Read`, `Init` keeps the loaded grids, creates W, and resizes all six grids to the tuned S (a no-op for
  loaded grids already at S, simgrid §4.2).

---

## 10. Interface slots *[compat]* (foundations §16, §17)

### 10.1 `cISC4GrowthDeveloper`, vtable `0x00AB1488`, 31 interface slots + class slot 31

All `__thiscall`. Pop = bytes removed by the callee (checked with `abi_extract.py --check`); returns read from the
epilogues.

| Slot | Off | Method as compiled | Pop | Return | Behaviour |
|---|---|---|---|---|---|
| 0 | `+0x00` | QueryInterface(iid, void**) | 8 | AL | §1 |
| 1 | `+0x04` | AddRef (`0x004F7BA0`, shared) | 0 | EAX count | |
| 2 | `+0x08` | Release (`0x006B3D00`) | 0 | EAX count | §1 |
| 3 | `+0x0C` | Init(cISCPropertyHolder*) | 4 | AL | §1.2 |
| 4 | `+0x10` | Shutdown() | 0 | AL (1) | §1.3 |
| 5 | `+0x14` | GetType() → int32 target zone | 0 | EAX | `+0xD0`: 1 R, 2 C, 3 I (tuning `0x87CD6300`). **Not** the developer type |
| 6 | `+0x18` | GetSubType() → developer type | 0 | EAX | `+0xCC`: `0x1010` … `0x4400`, the developer exemplar's instance; the key of the BDS developer map. A reader that takes slot 5 as the type sees `0x0001`–`0x0003` and finds no developer exemplar (manual-checks run 101) |
| 7 | `+0x1C` | GetPurpose() | 0 | EAX | |
| 8 | `+0x20` | GetWealth() | 0 | EAX | |
| 9 | `+0x24` | GetConstructionInfo(ConstructionProfile&) | 4 | none | §6.2 |
| 10 | `+0x28` | GrowthAndDeclineListsAreValid() | 0 | EAX 0/1 | `state < 17` |
| 11 | `+0x2C` | GetGrowthList() → vector& | 0 | EAX | address of the member vector |
| 12 | `+0x30` | GetDeclineList() → vector& | 0 | EAX | |
| 13 | `+0x34` | GetDesirabilityMap() → `cISC4SimGrid<int16>*` | 0 | EAX, no AddRef | D (not W) |
| 14 | `+0x38` | GetCapacityMap() → `cISC4SimGrid<uint16>*` | 0 | EAX | built capacity |
| 15 | `+0x3C` | GetPopulationMap() → `cISC4SimGrid<int16>*` | 0 | EAX | |
| 16 | `+0x40` | GetMaxCapacityMap() → `cISC4SimGrid<uint16>*` | 0 | EAX | |
| 17 | `+0x44` | GetTotalCapacity() | 0 | EAX | §6.3 (may be negative as int32); returns `+0x108`, uninitialised in a new city (§2). No caller found *[inferred]* |
| 18 | `+0x48` | GetSourceDemand() → `cISC4Demand*` | 0 | EAX, no AddRef | |
| 19–22 | `+0x4C`–`+0x58` | GetPowerSupplyCap / GetWaterSupplyCap / GetFireCoverageCap / GetParkCap | 0 | AL | |
| 23 | `+0x5C` | GetDesirabilityThreshold() | 0 | EAX | growth threshold |
| 24 | `+0x60` | GetAbandonmentThreshold(int32 densityType) → float | 4 | ST0 | map lookup that **inserts** 0.0 for an unknown key (B allocation). **No caller in the shipped exe** (§13.3 Q4) |
| 25 | `+0x64` | GetMonopolyUse() | 0 | EAX | |
| 26 | `+0x68` | GetStylePercentages() → `map<uint32, SC4Percentage>*` | 0 | EAX | address of the member map |
| 27 | `+0x6C` | StartMapUpdate() | 0 | none | state 0 → 1 |
| 28 | `+0x70` | IsMapUpdateFinished() | 0 | EAX 0/1 | |
| 29 | `+0x74` | EstimateDesirability(int32 x, int32 z) → int16 | 8 | **AX** | §4.6 |
| 30 | `+0x78` | GetSubjectiveThresholds(int32 i, int32& min, int32& max) | 12 | none | each output independently: `0 ≤ i < size` of its vector → element i; otherwise min := −4, max := 20 |
| 31 | `+0x7C` | scalar deleting destructor (class) | 4 | EAX this | frees with the global heap (C) |

SDK comparison (`cISC4GrowthDeveloper.h`): slot order, argument sizes and pointer returns are correct. Errors:

1. `GrowthItem` is `{int32 x, int32 z, int32 maxStage}`; the SDK's four `uint8` fields are one `int32` (written
   whole at `0x006B6FA8`).
2. `EstimateDesirability` returns `int16` in AX with garbage in the upper half of EAX (`0x006B51B3`), not `int32`.
3. `GetStylePercentages` returns a pointer to `std::map<uint32, SC4Percentage>` (STLport tree, nodes from B;
   values are floats), not "map<uint32, uint32>" (`0x006B7590`; destructor `0x004C56F0`; filled at §4.8).
4. `GetAbandonmentThreshold` is correct as declared (float, ST0, pop 4, `0x006B7620`) but is not const: it inserts.
   The Mac symbol types its argument as `cISC4BuildingOccupant::DensityType` (the SDK's parameter name is
   `densityType`).

### 10.2 `cIGZMessageTarget2`, vtable `0x00AB1474` (4 slots)

0 QueryInterface thunk `0x006B75E0` (pop 8, AL); 1 AddRef `0x0068D230` (pop 0); 2 Release thunk `0x006B7600`
(pop 0); 3 `DoMessage(cIGZMessage2*)` `0x006B7030` (pop 4, AL; §3.1).

### 10.3 `cIGZSerializable`, vtable `0x00AB145C` (6 slots)

0 QueryInterface thunk `0x006B75F0` (pop 8, AL); 1 AddRef `0x0068D250`; 2 Release thunk `0x006B7610`;
3 `Write(cIGZOStream&)` `0x006B3960` (pop 4, AL); 4 `Read(cIGZIStream&)` `0x006B39F0` (pop 4, AL);
5 `GetGZCLSID()` `0x006B3790` (pop 0, EAX `0x0990C138`).

---

## 11. Dependencies *[compat]*

| Interface (vtable) | Slot / Off | Method | Pop | Return | Where |
|---|---|---|---|---|---|
| `cISC4City` (`0x00A87620`) | 34 `+0x88` | GetOccupantManager | 0 | EAX | Init |
| | 50 `+0xC8` | GetSimulator | 0 | EAX | Shutdown |
| | 95 `+0x17C` | PositionToCell(float x, float z, int& cx, int& cz) | 16 | ignored | §4.4 |
| `cISC4Simulator` (`0x00AB2FB8`) | 8 `+0x20` | GetSimDateNumber | 0 | EAX (tail call into the date object) | state 1, NewDay |
| | 19 `+0x4C` | AddAgent(target, level, name, flags) | 16 | AL | Init |
| | 20 `+0x50` | RemoveAgent(target) | 4 | AL | Shutdown |
| `cIGZMessageServer2` (`0x00AD1BF4`) | 5 `+0x14` / 6 `+0x18` | AddNotification / RemoveNotification(target, type) | 8 | AL | |
| `cISC4BuildingDevelopmentSimulator` (`0x00AB0AF8`) | 35 `+0x8C` | GetTractDeveloper | 0 | EAX | state 18 |
| | 39 `+0x9C` | GetBuildingProfile(type) → profile& | 4 | EAX | §6.3 |
| | 44 `+0xB0` | GetSpecialBuildings → list* | 0 | EAX | §4.4 |
| | 45 `+0xB4` | GetMaxCapacity(zoneType, purpose, wealth) | 12 | **ST0 float** (SDK: int64) | §6.1 |
| | 53 `+0xD4` | GetStagePopulationMultiplier | 0 | ST0 | §5.5 |
| | static `0x00695390` | BuildingFitsInZone(zoneCode, zoneType) (cdecl) | caller | AL | §6.1 |
| `cISC4TractDeveloper` (`0x00AB3118`) | 7 `+0x1C` | UpdatePopulation(const GrowthProfile&) | 4 | ignored | §5.2 |
| `cISC4ZoneManager` (`0x00A92E00`) | 12 `+0x30` | GetZoneType(x, z) | 8 | EAX (int8 sign-extended to 32 bits, `0x004DC740`) | §6.1 |
| `cISC4LandValueSimulator` (`0x00AB1670`) | 7, 8, 9 | intrinsic / total (`uint8`), slope (`float`) maps | 0 | EAX | §4.3 |
| `cISC4ResidentialSimulator` (`0x00AB2C10`) | 5 `+0x14` | GetProximityMap(wealth 1..3) | 4 | EAX | |
| | 39 `+0x9C`, 40 `+0xA0` | school / hospital grade maps (`float`) | 0 | EAX | |
| `cISC4CommercialSimulator` (`0x00AB0CCC`) | 5 `+0x14` | GetProximityMap(developerType): `0x3320` / `0x3330` → its two `uint8` grids (S = 1), else null (`0x0069EAD0`) | 4 | EAX | |
| `cISC4AuraSimulator` (`0x00AB07C8`) | 13, 14, 15 | transient (`int8`), park, landmark (`int16`) | 0 | EAX | |
| `cISC4PollutionSimulator` (`0x00AB2630`) | 5 `+0x14` | GetPollutionGrid(type) | 4 | EAX | |
| `cISC4CrimeSimulator` (`0x00AB0DC8`) | 5 `+0x14` | GetCrimeGrid | 0 | EAX | |
| `cISC4TrafficSimulator` (`0x00AB33D0`) | 9 `+0x24`, 13 `+0x34` | commercial traffic map, trip-length map (`uint8`) | 0 | EAX | |
| `cISC4DemandSimulator` (`0x00AB0E48`) | 6 `+0x18` | GetDemand(id, index) | 8 | EAX (no AddRef) | tuning, §5.5 |
| `cISC4Demand` (`0x00AA2758`) | 8 `+0x20` | QuerySupplyValue | 0 | ST0 | §5.5 |
| | 17 `+0x44` | AddToDemandValue(float) | 4 | AL (true); nothing on the x87 stack | §5.3 |
| `cISC4NeighborsSimulator` (`0x00AB1EE0`) | 21 `+0x54`, 22 `+0x58` | GetConnectedCities / GetAllConnectedCities(list&, 0) | 8 | | §5.5 |
| `cISC4Lot` (`0x00AB19B0`) | 9, 14, 21, 22, 46, 50 | GetLotConfiguration, GetBoundingRect, GetLocation, GetSize, GetBuildingType(bool), GetCapacity(devType, bool) | 0, 4, 8, 8, 4, 8 | EAX / AX (50) | §6.3 |
| `cISC4Occupant` (`0x00AA41D8`) | 6 `+0x18`, 8 `+0x20` | AsPropertyHolder, GetPosition(cS3DVector3&) | 0, 4 | | §4.4 |
| `cISCPropertyHolder` (`0x00AA6CD8`) | 9 `+0x24` | GetProperty(id) | 4 | EAX | §4.4 |
| `cIGZMessage2Standard` | 4, 9, 14, 15, 19, 24, 32 | GetType, GetData1, GetData2, GetVoid2, GetData3, GetData4, GetIGZUnknown | 0 | EAX | §3.1 |
| `cISC4SimGrid<T>` | 3, 4, 6, 7, 9–13, 22, 24, 26 | simgrid §4 | per T | | |
| streams | `cIGZOStream` / `cIGZIStream` 7, 8, 9, 18 | Set/GetUint16, Set/GetSint32, Set/GetUint32, GetError | | | §9 |

Shared helpers: response curve constructor `0x007D0410`, loaders `0x005F4620` / `0x005F4480`, evaluator
`0x005F4250` (pop 4, ST0); walkers §4.3; `AddRadiusEffect<int16>` `0x005E4500`; scanning iterator `0x0060CE90` /
`GetNext` `0x0060C8D0`; slicing helper `0x0068CC20` / `0x0068BF80` / `0x0068B960`; `cRZRandom` `0x00913A5C` /
`0x00913696`; `std::sort` `0x006B5EC0`; `SC4WriteGZSerializable` `0x005D99A0`, `SC4ReadGZSerializable`
`0x005D9730`.

---

## 12. Replaceability

**Replacement unit** *[choice]*: the growth developer is replaced together with the building development simulator's
developer creation (building-development.md §9), because the BDS constructs growth developers directly for a new
city and only uses the GZCOM factory when loading.

- **Construction blocker.** The BDS constructs new developers directly (§1.1), so a GZCOM registration of
  `0x0990C138` affects only loaded saves and mixes original and replacement developers. Replacing the growth
  developer therefore requires replacing the BDS's developer creation too (`building-development.md`).
- No original code calls its non-virtual members or reads its memory (callers of every member function and
  references to the vtable checked: only `DoMessage`, the constructor's two callers and the destructor). Everything
  else goes through the three interfaces.
- **Grids** (simgrid §6.4 applies; create game grids, save by reference):
  - D (`GetDesirabilityMap`): read **raw** by land value (two-grid walker over `GetGridData`, `landvalue.md`
    §7.4); through the interface by demand (`GetValue`, `demand.md` §6) and the tract and lot developers
    *[inferred, see their specs]*.
  - Population: read by residential through `GetTractValue` at S = 4 (`residential.md` §5.5); residential also
    takes its own tract sizes from this grid's `GetTractSize` on a new city.
  - Built capacity, maximum capacity: through the interface by the BDS / tract / lot developers *[inferred]*.
  - Zoned-cell count and W: internal (W is published to nothing; the comparator reads it through a static).
- **Containers crossing the interface** (rows for `allocators.md` in the final report): the growth and decline
  vectors (returned by reference, B buffers, 12-byte items, read-only for callers *[inferred]*); the style map
  (returned by pointer, B nodes); the BDS special-building list (read, elements `AddRef`ed into a private copy).
  Internal only: the abandonment map, the private special-building list, the curve objects and point buffers, the
  stage-curve vector, the subjective vectors.
- A replacement must keep: the slot ABI of §10 (AX return of slot 29, ST0 of slot 24, AL of 19–22), the
  `GrowthItem` layout and STLport vector layout of the returned lists, the STLport map layout of slot 26, the
  agent levels and NewDay rule, the save record of §9, the RNG use of §7 and the sort algorithm of §5.4.

---

## 13. Quirks and open questions

### 13.1 Quirks (reproduced unless marked *[choice]*)

1. No clamping of W: 16-bit wrap on every layer (test 5).
2. Landfill garbage reads negative → the R$ garbage curve's maximum (§4.3).
3. Slope truncated toward zero before its curve (W finer than the slope grid).
4. Commute: sum truncated, not the curve output (test 6).
5. `EstimateDesirability` reads park, landmark, transient as unsigned low bytes and pollution as unsigned (test 7).
6. StateChanged with building type 0 does nothing (the looked-up type is discarded).
7. `UpdatePopulationCount` divides by the lot area without a zero check; *[choice]* skip when the area is 0.
8. Population above 32767 wraps negative in the `int16` grid (test 1).
9. Out-of-bounds reads: `0x4A5F7315` triples, `0x87CD6380` with fewer than 3 values, odd `0x87CD63A0`; *[choice]*
   treat missing values as absent.
10. Missing tract-size property → iterator rectangle unset; *[choice]* always set it.
11. *Withdrawn* (DG1): `AddToDemandValue` returns true in AL and leaves nothing on the x87 stack; there is no quirk
    to reproduce.
12. `UpdatePopulation` receives an uninitialised stage byte; *[choice]* 0.
13. `GetBuildingProfile` and `GetAbandonmentThreshold` insert defaults on lookup.
14. Lot pointer taken from the message without `QueryInterface`.
15. `GetConstructionInfo` leaves the zone code unwritten for industry with a purpose outside 5..8.

### 13.2 Corrections to earlier notes

- `development.md` §4: the special-building triple is (radius in cells, centre value, outer value), applied with
  `AddRadiusEffect<int16>`; the commute layer truncates the sum; W (scratch) is not saved, D is; the save holds
  the last phantom sum and total capacity (not unknown fields); EstimateDesirability skips commute and specials.
- `residential.md` §11.1 (dependency table line for `GetPopulationMap`): the grid is `cISC4SimGrid<int16>*` (class
  `0x49B9E605`), not `uint16`. Reading values with the `uint16` view gives the same bits.

### 13.3 Open questions

1. Who calls `EstimateDesirability` and `GetSubjectiveThresholds`, and what the 7 subjective indices mean.
2. Names of `0x4A5F7C4B` / `0x4A5F7315` and which buildings carry them.
3. Whether any caller modifies the growth or decline vectors (assumed read-only).
4. *Answered.* Keys 1..3 are `cISC4BuildingOccupant::DensityType` values (Mac symbol of slot 24), i.e. low, medium,
   high density *[inferred: the enum's numeric values are not in the binary]*. Slot 24 is **dead in the shipped
   exe**: its only reference is the vtable entry `0x00AB14E8`; no direct call exists; the map's `operator[]`
   (`0x006B5840`, identical-code-folded) is called only by slot 24, by `ReadSimulatorTunableValues`
   (`0x006B8336`–`0x006B83D6`) and by an unrelated map in `STETerrain` (`0x0074AF00`); of the 656
   `call [reg + 0x60]` sites in the image, the 26 that consume an x87 result all belong to other interfaces, and no
   caller of the BDS `GetDeveloper` calls `+0x60`. Only a plug-in could call it. FreeCity4 implements it as declared (with the
   inserting lookup) for interface compatibility; no other spec needs to cite it beyond `lot-developer.md` §12.3.

---

## 14. Conformance tests (must exist in `engine/conformance`)

1. **Population** (§5.2), W / capacity → population: 57 / 1000 → d = 0.57f (`0x3F11EB85`), 570; 50 / 3 → 2
   (1.5 rounds to even); 50 / 5 → 2 (2.5); 33 / 7 → d = `0x3EA8F5C2`, p = 2.31 → 2; 99 / 1 → 1; −20 / 100 → 0;
   150 / 40000 → d = 1, stored −25536.
2. **Max capacity** (§6.1): one tract (S = 4), 10 fitting cells each with `GetMaxCapacity` = 7.3f → sum 73.0 →
   max capacity 73, zoned count 10. 5 cells × 0.5f → 2.5 → 2.
3. **Built capacity** (§6.3), shift 2, lot rect x 3..4, z 0..2, size 2 × 3, location (3, 0), `GetCapacity` = 40,
   capacities initially 0: add → tract (0,0) = 3·6 + 4 = 22, tract (1,0) = 18, total +40. Then remove → both 0,
   total 0 (q = −6, r = −4). Remove again → both stay 0, total −40.
4. **Stage assignment** (§5.5), shipped R$ stage curves, size 400 after the multiplier, 4 growth items:
   curve values stage 1 = 68.888885, stage 2 = 28.333332, stages 3..8 = 0; inv = 0.25; p = 0, 25, 50, 75 →
   maxStage 2, 2, 1, 1 (acc 28.333332, then 97.22221).
5. **AddCurve wrap**: W = 32760, curve output 10.7 → `ftol` 10 → W = −32766.
6. **Commute**: curve (0,10)(255,−10), W = 25: m = 100 → curve 2.1568623 → W = 27; m = 200 → curve −5.6862755
   (`0xC0B5F5F8`) → r24(25 + c) = 19.313725 → W = 19 (a per-curve truncation would give 20); no non-zero cell →
   m = 255 → −10 → 15.
7. **EstimateDesirability vs update**: baseline 25, only the transient curve (−127,−100)(127,100) present,
   transient cell −1: estimate = 25 + `ftol`(curve(255) = 100) = **125**; the update gives curve(−1) =
   −0.7874069 → 0 → 25. Park curve (−100,−10)(100,10), park value 300: estimate uses 44 → 4.4000006 → +4;
   the update uses 300 → +10.
8. **Lists** (§5.4), tracts in list order (0,0) W 60, (0,1) 70, (1,0) 60, (1,1) 40; thresholds 50 / 50:
   sorted (0,1), (0,0), (1,0), (1,1); decline [(1,1)]; growth [(0,1), (0,0), (1,0)] with maxStage from §5.5.
9. **Industrial decline order (seeded RNG)**: file-local generator seeded 12345 → salt = `NextRaw()` =
   `0x0C63D3DC` (the next update's salt: `0xFED6A704`). Decline list (3,5), (4,5), (10,2), (0,0): seeds
   `0x0C53D38C`, `0x0C23D38C`, `0x0CC3D3FC`, `0x0C63D3DC`; keys `0xD552F46D`, `0x80243FFD`, `0x9C320AE0`,
   `0xF1CC6936`; sorted: (0,0), (3,5), (10,2), (4,5). The generator has advanced exactly once.
10. **NewDay rule**: start day 100; today 103: state 2 → false (2 < 3); state 3 → true; state 0 → true;
    today 100 → true for every state.
11. **Phantom demand**: flag on, growth threshold 50, item with W 30 and population 200: f = r24(20 × 0.01f) =
    `0x3E4CCCCC` (0.19999999; the exact product 0.199999995529651641845703125 lies exactly halfway between
    `0x3E4CCCCC` and `0x3E4CCCCD` and rounds to even, as protection test 25 has for 20), r24(200 × f) = 39.999996,
    term `ftol` = **39** (corrected: not 40); last 0 → `AddToDemandValue(39.0)`. Flag off with last = 39 →
    `AddToDemandValue(−39.0)`, last 0.
12. **Style map**: pairs [0x3001, 50, 0x3002, 50] → 0.5f each (`0x3F000000`); pct 120 → 1.1999999.
13. **Tuning defaults**: an exemplar with only `0x21` and `0x87CD6310`: baseline 80, thresholds 80 / 80, phantom
    off, caps 0/3/3/6 (power, water, fire, park), monopoly 1, abandonment 0.2 for keys 1..3 and 0.0 (inserted) for
    key 4, no curves (W = baseline everywhere after the update), no stage curves (all maxStage 1).
14. **Save round trip**: record = `01 00`, s32, u32, five references in the order of §9; version 2 → `Read`
    false.
15. **Subjective thresholds**: index 2 with the R$ vectors → (−3, 7); index 7 → (−4, 20); index −1 → (−4, 20).
16. **StateChanged**: wealth mismatch → no change; old 1 → new 2 with building type 0 → no change; old 3 → new 4
    with a matching type → capacity removed as in test 3.
17. **Float walker visit order** (§4.3, A coarser): W tract size 4, a float source grid with tract size 2 (d = 1),
    baseline 25, only the school curve, (0,0)(10,10). Source values for W tract (0,0): (x 0, z 0) = 0,
    (0, 1) = −0.5, (1, 0) = 4.5, (1, 1) = 0. Visit (0,0), (0,1), (1,0), (1,1): acc 0 → `ftol`(−0.5) = 0 → 4 → 4;
    mean (4 >> 1) >> 1 = 1 → curve 1.0 → **W = 26**. (A z-outer order would give acc 3, mean 0, W = 25.)
18. **City-size queries** (§5.5), mock demand simulator recording `GetDemand` calls, multiplier 1.0: zone 1,
    n1 = 0, n2 = 0, supply(0x3001) = 100, supply(0x1001) = 80 → calls `(0x3001, 0)`, `(0x1001, 0)` only (no
    `(0x3301, 0x10000)`), size 100. Same with n1 = 1, n2 = 2, supply(0x3301) = 400 → calls `(0x3001, 0)`,
    `(0x3301, 0x10000)`, `(0x1001, 0)`; t = 200, x = 200, size 200. Zone 3 → only `(0x4001, 0x20000)` and no
    neighbour calls. An empty growth list makes the same calls.
