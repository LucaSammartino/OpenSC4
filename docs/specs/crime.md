# Crime simulator (`cSC4CrimeSimulator`) — behaviour spec

Status: round 1, written from the Windows 1.1.641 executable. The earlier write-up in
`docs/subsystems/grid-sims.md` §6 was the starting point. Every fact below was checked against the Windows code,
and the corrections are listed in §15.

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

Shared material cited and not repeated here: streams, refs and strings (foundations §2), class store (§3),
city component table (§3.6), Init order (§3.7), response curves (§4), messages (§5), random generators (§6),
month order (§13), slot-order rule (§16), framework slot tables (§17); scheduler levels and agent semantics
(simulator spec); grid classes, geometry, grid slots, grid persistence, lifetime and the map utilities
(`docs/specs/simgrid.md` §2–§7).

---

## 1. Role and identity

The crime simulator turns census data (residential population by wealth and education, joblessness) into a
monthly number of crimes. It places each crime on a random lot. Police coverage at the lot, minus the "crime
effect" of nearby buildings, can stop it. Each crime that happens:

- adds the crime type's **severity** to the crime grid at the lot,
- is recorded as an **incident** (used by police, crime props, traffic effects and the map),
- can be **arson**, which sets the building on fire.

The crime grid fades each month. The simulator also requests **crime props** (graffiti etc.) from the prop
developer, weighted by local crime. It exposes the nearest crime automaton or crime prop to police vehicles.

| Item | Value |
|---|---|
| Class ID (`GetGZCLSID`) | `0x8990C09A` |
| Primary interface | `cISC4CrimeSimulator`, IID `0x67659555`; QI also answers `0x00000001` (cIGZUnknown) with it |
| `cIGZMessageTarget2` | IID `0x452294AA` |
| Prop requester | `cISC4PropRequester`, IID `0x89C498BE` (name from the Mac build) |
| `cIGZSerializable` | IID `0xE4FDA3D4` |
| Not answered | `cIGZMessageTarget` `0xE98B2F57`, everything else → false, out pointer null |
| City component | slot **#23** (foundations §3.6); the city gets it with `GetClassObject(0x8990C09A, 0x67659555)` |
| City getter | `cISC4City` slot 55 (`+0xDC`) `GetCrimeSimulator`, cached by the game as `SL::spCrimeSim` |
| Init order | after commercial, before fire protection (foundations §3.7) |
| Extra class slot | slot 15 of the primary vtable (after the destructor) returns the simulator type `0x27567222` |

All four interfaces are one object (one reference count). `AddRef` increments, `Release` decrements. When
`Release` is called with a count ≤ 1, it calls `Shutdown()` and then destroys the object. It returns 0 then,
otherwise the new count. *[compat]*

## 2. Grids

Two `uint8` grids (simgrid §2, CLSID `0x49B9E602`). Both use the tuning tract size `S` (property `0x47E2C200`,
default **2**) at creation and the geometry of simgrid §3. Later code reads the size back from the grid
(`GetTractSize`/`GetTractShift`), never from the tuning (§8).

| Grid | Instance ID | Exposed | Meaning |
|---|---|---|---|
| crime grid | `0x89D5B895` | `GetCrimeGrid()` | accumulated crime: severities stamped by crimes, decayed monthly |
| crime-effect grid | `0x2A5BA1B3` | no | sum of building "crime effect" cones (property `0xCA5B9306`) |

- Created in `Init` when absent (a new city, or a save that did not provide them). Steps: create, `Init()`,
  `SetTractSize(S)`, `SetInstanceID(id)`, `SetTractValues(0)`. *[compat]* The original builds them with its own
  allocator and constructor, not through the class registry. *[choice]* FreeCity4 creates them with
  `GetClassObject(0x49B9E602, 0xC9BD939A)` (see §14.2, replaceability).
- Saved as **refs** (simgrid §5), so each grid is its own object in the class store.
- `GetCrimeGrid()` returns the pointer **without** `AddRef`. Callers must not release it. *[compat]*
- The simulator itself reads and writes cells **directly through `GetGridData`** (row vectors indexed `[x][z]`,
  simgrid §4). It does not use `SetTractValue`.

## 3. State

| State | Initial | Saved | Notes |
|---|---|---|---|
| crime grid, crime-effect grid | absent | yes (refs) | §2 |
| random cell iterator | rect 0, step 1, state/mode/dirty = 1 | yes | set to the whole crime grid in `Init` of a **new** city; never advanced (unused) |
| two unused `u32` values | 0 | yes | never set *[inferred]* |
| average crime (`f32`) | 0 | yes | recomputed monthly (§6.9) |
| ordinance crime factor (`f32`) | 1.0 | yes, but `Init` resets it to 1.0 | §4.2, §15 |
| monthly crime count (`u32`) | 0 | yes | crimes that happened last run (§6.6) |
| hotspot tract x, z, value (`s32` ×3) | 0 | yes, but `Init` resets them to 0 | §6.5, §6.6 |
| incident list | empty | yes | one record per crime of the last run (§6.8) |
| pipeline state (0–6) | **6** | no | §5 |
| criminal base, crimes to commit | 0 | no | §6.3, §6.4 |
| census table `pop[wealth 0..2][bucket 0..3]` (`u32`) | 0 | no | §6.1 |
| jobless factor per wealth (`f32` ×3) | 0 | no | §6.2 |
| tuning values and curves | §8 | no | |
| visuals-created flag per incident | false | no (false after load) | §11 |
| `initialized`, `loadedFromSave` flags | false | no | `loadedFromSave` is set by `Read` once the version is accepted, even on a stream error (§12) |
| crime type list | — | no | **process-wide**, shared by all cities (§7) |

## 4. Lifecycle

### 4.1 `Init()` → true

If already initialized, return true and do nothing. Otherwise, in order:

1. Set `initialized`.
2. Only the first time in the process: register an application shutdown callback (`cIGZApp` slot `+0x114`)
   that frees the static crime type list *[inferred: purpose]*, then load all crime types (§7).
3. Read the tuning (§8). This runs on every `Init`, including after a load.
4. Register as an agent of the simulator (`cISC4Simulator::AddAgent`, slot `+0x4C`) on the
   `cIGZMessageTarget2` sub-object, name `"cSC4CrimeSimulator"`, flags **2**, at levels **4** (day), **6**
   (month) and **10** (idle), in that order.
5. If it has no message server yet: hold the message server and subscribe to `0x99EF1142`, `0x99EF1143`,
   `0xC2C09554`, `0xC53D10AA`, in that order.
6. Hold (AddRef) the city, the occupant manager (`cISC4City +0x88`), the ordinance simulator (`+0xFC`) and the
   police simulator (`+0x104`). The city, ordinance and police pointers are taken from the game's cached service
   pointers (filled from those city getters at city init) and are AddRef'd **without a null check**: with no
   ordinance or police simulator the original crashes here. *[choice]* FreeCity4 holds null, treats a missing
   ordinance simulator as "factor unchanged" (§5), a missing police simulator as 0 jailed and 0 released (§6.4)
   and a missing police grid as 0 coverage (§6.6).
7. Create each missing grid (§2).
8. If **not** `loadedFromSave`: set the iterator rectangle to `(0, 0, countX − 1, countZ − 1)` of the crime
   grid, and register the prop-requester sub-object with the prop developer (`AddPropRequester`, §11.6). In a
   loaded city the prop developer restores the registration itself from its own save (§14.2).
9. Reset the hotspot to (0, 0, value 0) and the ordinance factor to **1.0**, for new and loaded cities alike.
   *[compat, quirk]* After a load nothing re-posts OrdinanceUpdated, so the factor stays 1.0 until some
   ordinance's on flag changes (§16 Q1).

### 4.2 `Shutdown()` → true

If not initialized, return true. Otherwise clear `initialized`. Shut down and release both grids. Release the
police, ordinance, occupant manager and city references. Clear the incident list (releasing each incident's lot).
Unsubscribe `0xC2C09554`, `0xC53D10AA`, `0x99EF1142`, `0x99EF1143`, then release the message server. Remove the
agent from all levels (`cISC4Simulator` slot `+0x50`, the single-argument overload).
*[compat]* It does **not** unregister from the prop developer (`0x006A1860`; no prop-developer call, unlike
pollution, which calls `RemovePropRequester`). The registration ends on the prop developer's side:
`AddPropRequester` (`0x006F21C0`) AddRefs the requester and appends it with no duplicate check, and the prop
developer's own `Shutdown` (`0x006F2F70`–`0x006F2FCE`) releases each listed requester once and empties the
list. So the shut-down crime object stays alive, held by the prop developer, until that `Shutdown`. An `Init`
on a new city that is not preceded by the prop developer's `Shutdown` would register the requester a second
time.

## 5. Scheduling and messages handled

`DoMessage` always returns **true** (finished, simulator spec §4).

| Message | Action |
|---|---|
| `0x66956814` SimNewDay (level 4) | `DoIdle()` (one pipeline step) |
| `0x6695681A` SimIdle (level 10) | `DoIdle()` (data ignored) |
| `0x66956816` SimNewMonth (level 6) | if state = 6 then state := 0 |
| `0x99EF1142` occupant inserted | occupant = `void1`. Skip it if it has property `0x029244DB`. Otherwise apply its crime effect with sign + (§6.10) |
| `0x99EF1143` occupant removed | the same, with sign − |
| `0xC2C09554` OrdinanceUpdated | ordinance factor := `GetCumulativeFactor(0x28ED0380, 0)` of the held ordinance simulator, stored as `float32`. No null check (with no ordinance simulator the original has already crashed in `Init`, §4.1). *[choice]* FreeCity4: no ordinance simulator → the factor is left unchanged |
| `0xC53D10AA` exemplar changed (foundations §5.2) | data1 = group, data2 = instance. Group `0xE7E2C2DB` and instance `0xC8E8F6C5`: re-read the tuning (§8). Group `0xEA12F32C`: reload all crime types (§7) |

**Pipeline** (`DoIdle`, one step per call):

| State | Step | Next |
|---|---|---|
| 0 | wait for `initialized` (always true after `Init`) | 1 |
| 1 | census (§6.1), then economic factors (§6.2) | 2 |
| 2 | criminal base (§6.3) | 3 |
| 3 | crimes to commit (§6.4) | 4 |
| 4 | decay the crime grid (§6.5); clear the incident list; commit crimes (§6.6) and store the result as the monthly crime count; police `ProcessMonthlyCrimeStats()`; average crime (§6.9) | 5 |
| 5 | post `0x88EA4DFF` (§10) | 6 |
| 6 | idle | (month message → 0) |

- A new simulator starts in state 6. The first run starts after the **first month message** and takes the next
  six day/idle deliveries. *[compat]* The state is not saved, so the same holds after a load.
- The month handler only resets the state. All work happens on later day/idle deliveries. The crime simulator's
  rank in the month order (foundations §13) therefore does not matter. It reads police, residential and demand
  values as they are at the time of each step.

## 6. Algorithms *[compat]*

Notation: `trunc` is conversion toward zero (the CRT `_ftol`: it converts to a 64-bit integer, truncating, and
the callers keep the low 32 bits; NaN and out-of-range values give the int64 indefinite `0x8000000000000000`,
whose low 32 bits are **0**). `rhe` is the original's "add 1.5·2²³ and store as `f32`" idiom. Exactly *[compat]*:

```
rhe(x) := int32( bits(f32(f32(x) + 12582912.0f)) − 0x4B400000 )      (32-bit wrapping subtraction)
```

For −2²² ≤ x ≤ 2²² this is round-half-to-even of the `float32` `x`. Outside that range the sum's spacing is
2 or more and the bit pattern no longer encodes `x`: for sums in [2²⁴, 2²⁵) (2²² ≤ x < 20,971,520) the result is
`2²² + (s − 2²⁴)/2` with `s` the sum rounded to an even integer, about `x/2 + 2²¹` (5,000,000 → 4,597,152;
8,388,608 → 6,291,456); each further power of two halves the slope again (100,000,000 → 26,655,776;
4,294,967,296 → 71,327,744). Below −2²² it is about `2x + 2²²` (−5,000,000 → −5,805,696). A replacement must
use the formula, not a saturating conversion. §6.4 and §11.6 say where such inputs can arise.

Products of a `float32` with an integer
or another `float32` are rounded to a 24-bit significand before truncation, because the game runs the x87 at
24-bit precision (foundations §18; corrected, the earlier text said "exact, extended precision"). FreeCity4 must
compute such a product in `f64` and round it to `f32` once (this equals the x87 result, since an `int32` operand
is not rounded first and the `f64` product is exact for integers below 2^29), then truncate.

### 6.1 Census (`DoCensus`)

Get the residential simulator's three **population** grids (`uint16`, R$, R$$, R$$$) and three **EQ** grids
(`float`). Both getters fill three out-pointers and AddRef them (§13). For each wealth `w`:

```
pop[w][0..3] := 0
for every tract (x, z) of population grid w (x < countX, z < countZ):
    b := trunc(EQ_w[x][z] × 0.02f)          (0.02f = 0.0199999995529651641845703125)
    pop[w][b] += population_w[x][z]
```

Release the six grids. Both grids are read through `GetGridData`, at the same tract indices.

- **Bucket boundaries** *[compat]*: the constant is a `float32` (`fmul dword`, 0.02f). The product is rounded to
  `float32` before the truncation (§6 notation, foundations §18), and every multiple of 50 rounds up to the whole
  number: the exact product 50 × 0.02f = 0.99999997765 rounds to 1.0. Buckets: 0 = EQ < 50, 1 = [50, 100),
  2 = [100, 150), 3 = [150, 200), and **EQ = 200 gives bucket 4**. EQ in (−50, 0] also gives 0. *(Corrected
  for foundations §18: the earlier text used the unrounded product, which put each multiple of 50 in the
  bucket below.)*
- **EQ ≥ 200** *[quirk]*: the original has no bounds check. Buckets ≥ 4 write past the row, into storage that
  is cleared or overwritten before use. With extreme values (R$$$ EQ ≥ 350, R$$ ≥ 550, R$ ≥ 750) the write
  corrupts the tuning tract size, and with larger values the response curves. *[choice]* FreeCity4 ignores
  populations whose bucket is outside 0..3. This matches the original for every value that does not corrupt
  memory. Consequence *[compat]*: population in tracts at EQ exactly 200 (the EQ cap) is not counted by the
  census at all.

### 6.2 Economic factors (`CalculateEconomicFactors`)

For each wealth, sum three values over a fixed list of demand IDs, using `cISC4DemandSimulator::GetDemand(id, 0)`
(the result is not null-checked):

| Wealth | Demand IDs |
|---|---|
| R$ | `0x4100`, `0x4200`, `0x4300`, `0x3110`, `0x3120` |
| R$$ | `0x4400`, `0x3320`, `0x3130` |
| R$$$ | `0x3330` |

`A += QueryActiveDemandValue()`, `B += QueryActiveDemandMax()`, `C += QueryActiveDemandMin()` (`cISC4Demand`
slots `+0x30`, `+0x38`, `+0x3C`; demand spec §3), accumulated in `float32`.

```
jobless[w] := 2 · (A − C) / (B − C) − 1
```

*[quirk]* When `B = C`, this is ±∞ or NaN. The curve of §6.3 then gives its end value for ±∞. For NaN the
original's comparison order yields the last segment's y *[inferred]*. *[choice]* FreeCity4 uses 0 for the
factor when `B = C`.

### 6.3 Criminal base (`ComputeCriminalBase`)

```
for w in R$, R$$, R$$$:
    acc    := Σ_{b=0..3} EQcurve_w(25 · b) × pop[w][b]        (float32 accumulator; inputs 0, 25, 50, 75)
    popSum := Σ_b pop[w][b]
    crim_w := trunc(joblessCurve_w(jobless[w]) × popSum + acc)
criminalBase := Σ_w crim_w                                     (32-bit integer)
```

*[compat, quirk]* The EQ curve is sampled at `25·b`, not at the bucket centres, so the shipped curves (defined
to 200) are only ever read between 0 and 75. The jobless term can be negative, so `crim_w` and the base can be
negative.

### 6.4 Crimes to commit (state 3)

```
jailed   := police.GetJailPopulation()          (slot +0x78, u32: Σ over jails of the rounded jail population
                                                 property 0x48D71EE1; protection §9, §15.2)
released := police.ResetUnjailedCriminals()      (slot +0x6C, s32; has side effects, called second)
if criminalBase > jailed (unsigned compare):
    crimes := trunc(crimesCurve(float(int32(criminalBase − jailed)))) + trunc(released × reoffend)
else:
    crimes := trunc(released × reoffend)
crimesToCommit := rhe(f32(float(uint32 crimes) × ordinanceFactor))
```

- `reoffend` is property `0x8A0CBAAD` (shipped name "Criminal Grid Decay Factor", 0.66).
- *[compat]* Rounding of the last line (foundations §18): the factor and the rounding constant 1.5·2²³ are both
  `float32` operands. The product is rounded to `float32` first; adding 1.5·2²³ then rounds half to even at the
  integer. So 10 × 0.95f (exact 9.49999988) becomes 9.5 and then **10**, not 9. Likewise `trunc(released ×
  reoffend)` truncates the `float32`-rounded product. *(Corrected; the earlier test values assumed one rounding
  of the exact product.)*
- *[compat]* The two police calls must be made in this order, once per month. Slot `+0x6C` returns the number of
  criminals released from jails (the jailed count above the current capacity, plus any pending releases) and
  resets that counter (police spec).
- A negative base compares as a huge unsigned number, so the first branch runs. The curve input is then
  negative, and the curve clamps it to its first y (0 with shipped data).
- **Range at `rhe`** *[compat]*. `crimes` is an `int32` (the sum of two `trunc` low halves) converted to
  `float32` as **unsigned**: a negative value has 2³² added in `float32` (so −1 becomes 4,294,967,296.0). With
  shipped data the input stays far below 2²²: the crimes curve is clamped to at most 65, and `released` counts
  last month's unarrested crimes plus the jail overflow (protection §9.3), both bounded by the crimes of earlier
  months (a few hundred). Values above 2²² are reachable only with modded tuning: a crimes curve with a y above
  2²² or **below 0** (the unsigned conversion then gives about 4.29·10⁹ → `rhe` 71,327,744 at factor 1), a
  negative or huge reoffend factor, or a huge ordinance factor. The exact formula of §6 is the *[compat]* result;
  `AddCrimes` then runs that many attempts.

### 6.5 Decay (`DecayCrimeGrid`)

For every **non-zero** cell: `v := low 8 bits of trunc(v × decay)` (`decay` = property `0x8A0CBAAC`, default 0.5,
shipped 0.8). Then `hotspotValue := crimeGrid[hotspotX][hotspotZ]` (the hotspot tract is kept, its value
follows the decay).

- 200 → 100 at 0.5, 200 → 160 and 255 → 204 at 0.8f (the product 204.000003 rounds to 204.0 in `float32`), 1 → 0.
- *[quirk]* A decay > 1 wraps modulo 256 (200 × 1.5 = 300 → 44). *[choice]* FreeCity4 reproduces the wrap.

### 6.6 Committing crimes (`AddCrimes(n)`)

If `n ≤ 0`, the result is 0. Otherwise repeat `n` times:

1. `lot := lotManager.GetRandomLot()` (slot `+0x34`). If null, this attempt ends. (The lot manager picks random
   cells with its own generator, up to 999 tries. It is not part of this spec. §9.)
2. `(x1, z1, x2, z2) := lot.GetBoundingRect()`, inclusive city cells.
3. `pol := policeGrid.GetAverageValueInCellRect(x1, z1, x2 − x1 + 1, z2 − z1 + 1)`. The police grid comes from
   police slot `+0x14`.
   `eff := crimeEffectGrid.GetAverageValueInCellRect(same)`. Both are `uint8` (simgrid §4).
4. `p := attenuationCurve(float(pol − eff))` (property `0x8A0CBAAB`), stored as `float32`.
5. `u := RandomDoubleUniform()` on the crime generator (§9).
6. If `p > u`: the crime is **stopped**, and this attempt ends. Otherwise `s := AddCrimeEvent(lot)` (§6.7,
   §6.8). If `s = 0`, the attempt ends.
7. Stamp: shift the bounding rect right by the crime grid's tract shift (arithmetic shift). For every tract
   `(x, z)` in it, x-outer and z-inner: `cell := min(255, cell + s)`. If `cell > hotspotValue` (strict), the
   hotspot becomes `(x, z, cell)`.
8. `count += 1`.

The result `count` (crimes that happened) becomes the monthly crime count.

So `p` is the probability that police **stop** an attempt. The chance that an attempt on a found lot becomes a
crime is `1 − p`: a crime happens when `u ≥ p`. `u` is in [0, **1**] (foundations §6.1: 1.0 is reachable), so
a draw of exactly 1.0 always commits whenever `p ≤ 1` (every curve with y ≤ 1, as shipped).

### 6.7 Lot bits and crime type selection

The lot is classified into four bit sets (`GetLotBits`):

| Set | Source | Value → bits |
|---|---|---|
| zone | `zoneManager.GetZoneType(cellX, cellZ)` at `lot.GetLocation()` | 1–3 → `0x1`, 4–6 → `0x2`, 7–9 → `0x4`, other → `0x80000000` |
| wealth | `lot.GetBuilding()` → `GetBuildingProfile().wealth` | `w ≥ 1` → `1 << (w − 1)`; 0 or no building → `0x80000000` |
| purpose | same profile, `.purpose` | 0 or no building → `0x100`; `p ≥ 1` → `1 << (p − 1)` |
| condition | `lot.GetCondition()` (byte) | 0, 1 → `0x1`; 2, 3, 5, 6 → `0x2`; 4 → `0x4`; > 6 → `0x80000000` |

A crime type is a **candidate** when each of its four masks (§7) shares a bit with the lot's set. Candidates
keep the order of the type list. With `total := Σ weight` over the candidates (`float32`):

- **No candidates**: no crime, result 0, no random draw.
- Otherwise `r := RandomDoubleRangeUniform(0, total)` (double). Walk the candidates in order:
  `r := r − weight`, and stop at the first candidate after which `r < 0`. If the walk ends without that
  (for example, all weights 0), the **last** candidate is chosen. *[compat]*

### 6.8 The crime event (`AddCrimeEvent`)

For the chosen type:

1. Append an incident `{lot (AddRef), type ID, name (copy), icon key, visuals = false}`.
2. If the type has the **arson** flag: `building := lot.GetBuilding()`. If it is non-null and the city's disaster
   layer (`cISC4City +0x120`) is non-null, then:
   - get the fire manager with `GetDisasterManager(0x621CDA33)` and QI it for `cISC4DisasterManagerFire`
     `0x49B43F57`,
   - `IgniteOccupant(building as cISC4Occupant)`, then `SetCause(2)` (arson *[inferred]*),
   - release the manager.

   *[quirk]* The original checks neither the manager nor the QI result for null.
3. Post `0x0A159D95` (§10).
4. Return the type's **severity** byte. A type with severity 0 still records and posts the incident, but
   §6.6 counts nothing and stamps nothing. *[compat]*

### 6.9 Average crime (`ComputeAverageCrime`)

Over all crime-grid tracts `(x, z)` (x < the grid's `GetTractCountX`, z < `GetTractCountZ`) whose corner cell
`(x·S, z·S)` has a zone type ≠ 0 (`zoneManager.GetZoneType`): `average := Σ value / count`, with 64-bit integer
sums converted to float, one division, stored as `float32`. Here `S` is the **crime grid's** `GetTractSize()`,
not the tuning value *[compat]*: after a reload message changes the tuning tract size, the existing grid keeps
its own size (§8) and this pass keeps using it.

*[compat, quirk]* With no zoned tract this is 0/0 = NaN: the stored value is the x87 default NaN (`float32` bits
`0xFFC00000`), and it is saved as such (§12). `GetAverageCrime()` returns `trunc(average)` as `uint32` (a tail
call of `_ftol`, §6 notation); for NaN that is the low half of the int64 indefinite, **0**. *(Corrected: the
earlier text said `0x80000000`, which only an inline 32-bit `fistp` would give, and chose to store 0; storing
the NaN keeps saves byte-identical and the getter result is 0 either way.)*

### 6.10 Building crime effect (`DoOccupantChanged(occ, sign)`)

Runs if the occupant (as a property holder) has property **`0xCA5B9306`**. The property is read as a byte
array `{m, r}`: magnitude and radius in **city cells**. *[quirk]* The original does not check the length, so
fewer than 2 bytes reads past the data. *[choice]* FreeCity4 skips arrays shorter than 2.

```
value  := +m when inserted, −m when removed
radius := float(r) / float(crimeEffectGrid.GetTractSize())          (tracts)
(tx, tz) := crimeEffectGrid.PositionToTract(position.x, position.z)  (occupant GetPosition)
AddRadiusEffect<uint8>(crimeEffectGrid data, countX, countZ, tx, tz, radius, centre = float(value), outer = 0)
```

The helper is simgrid §7.1, `uint8` variant `0x005E3D80`. Summary: the centre is in tracts. The box is
`±rhe(radius)` clamped to the grid. The slope is `(centre − outer) / radius`, and the distance is Euclidean in
tract units. A cell changes only if `d < radius` (strict), by `rhe(centre − d · slope)`, with the result clamped
to 0..255. Removal subtracts the same cone. Because of clamping, an insert/remove pair does not always restore
the grid. *[compat]*

## 7. Crime types *[compat]*

All exemplars of type `0x6534284A`, group `0xEA12F32C`, in the resource manager's key-list order. This list is
**static** (one per process), loaded at the first `Init` and reloaded by message `0xC53D10AA`. A reload frees
the old list first.

**Order of the list** *[compat]* (`LoadAllCrimeTypesFromExemplars` `0x006A1450`, `LoadCrimeTypeFromExemplar`
`0x006A0BA0`): the loader asks the resource manager for the key list of (type, group) (slot 17
`GetAvailableResourceList`, `0x009739CB`) and loads key 0, 1, 2, … in list order. A key whose instance is already
in the list updates that entry **in place** (keeping its position); a new instance is appended. The key list is a
plain sequence without de-duplication, built segment by segment in the resource manager's segment order (a
multi-file segment walks its files in order, `0x0097E43D`); each package file contributes its index **hash
table** in bucket order (`0x0096F9AB`): bucket `instance mod B` ascending, where `B` is the table's prime bucket
count (SGI prime list, at least 53), and within one bucket the most recently indexed key first (the hash is the
instance itself, `0x0097C0C5`; iteration `0x0097C081`, `0x0097C1EA`). It is **not** sorted by key.

Consequence for shipped data: every crime type is in `SimCity_1.dat`, each in its own bucket, and every instance
(0x01–0x13) is below 53 ≤ `B`, so the list is in **ascending instance order**: 0x01, 0x03, 0x04, 0x05, 0x06,
0x07, 0x08, 0x09, 0x0A, 0x0B, 0x10, 0x11, 0x12, 0x13 (the table below). Plug-in crime types with larger instances
land at `instance mod B` in their own package's table, and a plug-in package's types come before or after the
shipped ones according to segment order. *[choice]* FreeCity4 orders the list by ascending instance; this equals
the original for the shipped types (and for any set of types in one package whose instances are all below its
bucket count).

The order is **observable**: it is the walk order of the weighted selection (§6.7), so it decides which type a
given random draw commits (test 9). It is not saved (incidents store the type ID) and nothing else reads it.

Names in quotes are the official property names (community property list, re/prior_art_exemplar_props.md).

| Property | Type | Default | Meaning |
|---|---|---|---|
| (exemplar instance) | — | — | crime type ID |
| `0x8A416B2E` | u32 ×3 LTEXT key | — | display name, loaded with `SC4LoadExemplarString` (below) |
| `0xAA12FCB0` | **string** | empty | "Crime Name": an inline name, read **only when** the LTEXT load fails (no key property, or no such LTEXT). If it is absent too, the name stays empty |
| `0xAA12FCB1` | u32 ×3 | `(0, 0, 0)` | "Icon Resource Key" (type, group, instance) |
| `0xAA12FCB2` | u8 array | all bits | "Zone Filter" (zone mask) |
| `0xAA12FCB3` | u8 array | all bits | "Wealth Filter" |
| `0xAA12FCB4` | u8 array | all bits | "Zone Purpose Filter" |
| `0xAA12FCB5` | u8 | 64 | "Crime Grid Value": severity, added to the crime grid |
| `0xAA12FCB6` | u8 | 0 | "Trigger Fire": arson flag (non-zero = start a fire) |
| `0xAA12FCB7` | u8 array | all bits | "Lot Condition Filter" |
| `0xAA12FCB8` | f32 | 10 | "Relative Occurrance": selection weight |

**Name** *[compat]* (`LoadCrimeTypeFromExemplar` `0x006A0D12`; `SC4LoadExemplarString` `0x00603270` →
`0x00603220`; `SC4LoadString` `0x00603040`):

1. Read property `0x8A416B2E` as three `u32` from the exemplar that the resource manager returns for the crime
   type's own key. If it is missing, the load fails. Value [0] (the key's type) is **ignored**: the LTEXT type is
   always `0x2026960B`. Value [1] is the LTEXT group `G` and value [2] the instance `I`.
2. Look up (`0x2026960B`, `G` + current language ID, `I`). If that is missing and the language ID is not 0, look up
   (`0x2026960B`, `G`, `I`) (data-formats §6.2). If both are missing, the load fails and the name is untouched.
3. Copy the text into the name. Run the game's string detokenizer on it in place (the application's
   `GetStringDetokenizer`, `cISCStringDetokenizer` slot 9). If it returns false, the load fails. With no
   detokenizer the step is skipped.
4. Replace every U+2022 by U+200B (data-formats §6.1). The load succeeds.

If the load fails, read the string property `0xAA12FCB0` into the name. If that is missing too, the name stays
as it was (empty for a new entry).

**Mask parsing**: each byte `b` sets bit `b − 1`. A byte 0 makes the mask `0xFFFFFFFF` (any). *[quirk]* A present
but **empty** array gives mask 0, which never matches. A missing property gives `0xFFFFFFFF`. Byte values above
32 wrap the shift count modulo 32 *[compat]*.

Shipped types (`SimCity_1.dat`; no other shipped file defines any). Masks not listed are "any" (`[0]`). Icon
keys are type `0x7AB50E44`, group `0x1ABE787D` and the instance below. Name keys are type `0x2026960B`, group
`0x0A554AE0` and the instance below. No shipped exemplar has `0xAA12FCB0`. The "Name" column is the exemplar
name (property `0x20`), which the simulator does not read.

| ID | Name | Severity | Weight | Arson | Masks | Icon instance | Name key instance |
|---|---|---|---|---|---|---|---|
| 0x01 | Generic | 40 | 0 | — | — | `0x8A1DC833` | `0xEA5EA698` |
| 0x03 | Fighting | 60 | 3 | — | — | `0xCA63A970` | `0xDA5EA698` |
| 0x04 | Narcotic Deals | 80 | 2 | — | purpose [4, 2, 3] | `0x0A639636` | `0xEA5EA697` |
| 0x05 | Arson | 100 | 0.5 | yes | wealth [2, 1]; purpose [9, 1, 2, 3, 4, 6, 7, 8]; condition [3] | `0x2A6395EF` | `0xCA5EA698` |
| 0x06 | Money Snatcher | 60 | 3 | — | purpose [4, 2] | `0x4A639681` | `0xAA5EA698` |
| 0x07 | Stickup | 80 | 2 | — | purpose [3, 2] | `0x2A639653` | `0x2A5EA698` |
| 0x08 | Car Theft | 60 | 3 | — | purpose [9, 1, 2, 3, 4, 6, 7, 8] | `0xAA63A98D` | `0x3A5EA698` |
| 0x09 | Solicitation | 60 | 3 | — | purpose [4, 2, 3] | `0x6A63A9A4` | `0x4A5EA698` |
| 0x0A | Mugging | 80 | 2 | — | purpose [9, 1, 2, 3, 4, 6, 7, 8] | `0xAA63966B` | `0x0A5EA698` |
| 0x0B | Bank Robbery | 80 | 2 | — | purpose [3, 2] | `0xAA63966B` | `0x0A5EA698` |
| 0x10 | Flashing | 40 | 4 | — | purpose [9, 1, 2, 3, 4, 6, 7, 8] | `0x8A63A9BA` | `0x5A5EA698` |
| 0x11 | Graffiti | 40 | 4 | — | purpose [9, 1, 2, 3, 4, 6, 7, 8] | `0xCA63A9D1` | `0x6A5EA698` |
| 0x12 | Pie Throwing | 40 | 4 | — | purpose [9, 1, 2, 3, 4, 6, 7, 8] | `0x4A63A9E4` | `0x7A5EA698` |
| 0x13 | TPing | 40 | 4 | — | zone [1]; wealth [3, 2]; purpose [1] | `0x2A63A9F9` | `0x8A5EA698` |

*[compat]* Shipped data quirks: Bank Robbery has the same icon and name key as Mugging. In every one of the 13
shipped language packages, `0xEA5EA698` is the only name key present in group `0x0A554AE0`, and its text is an
unrelated building description. None of the other name keys exists in any shipped package, and no package has a
group `0x0A554AE0` + language ID. Only the current language's package is loaded (data-formats §6.2), so another
language cannot supply a name. In every language, the other 13 types therefore load **empty** (no `0xAA12FCB0`
fallback), and Generic's name is that language's unrelated text, detokenized. The name only feeds the incident
record, the `0x0A159D95` message string and the save. *[choice]* FreeCity4 reproduces the four steps above.
Inside the game it may instead call the game's `SC4LoadString` (`0x00603040`, `cdecl`, arguments `(cIGZString&
out, G, I, detokenizer or null)`), which gives the same text, detokenizer included.

Purpose byte 9 is bit 8 (`0x100`), the value used for "no purpose / no building" (§6.7). "Condition [3]" is bit 2,
which matches lot condition 4 only: an abandoned lot at its last decay stage (foundations §5.2).

## 8. Tuning *[compat]*

Exemplar type `0x6534284A`, group `0xE7E2C2DB`, instance **`0xC8E8F6C5`** ("Crime Simulator", `SimCity_1.dat`).
Read on every `Init` and on the reload message. First the scalar defaults are set (tract size 2, decay 0.5,
reoffend 1.0). If the exemplar exists, each curve is then set to its code default (when it has one) and
replaced by the property if that is present. A missing curve property **keeps** the default; the curve loader
used here leaves the curve unchanged when the property is absent.

| Property | Name / role | Code default | Shipped |
|---|---|---|---|
| `0x47E2C200` | crime grid tract size (u32) | 2 | absent → 2 |
| `0x49A34301` | Crime Prop Request Curve (avg crime/255 → share) | none | (0,0) (0.1,2) (0.2,4) (0.4,6) (1,10) |
| `0x8A0CBAA4` | R$: EQ → base criminality | (0,0.1) (255,0) | (0,0.4) (100,0.3) (200,0) |
| `0x8A0CBAA5` | R$$: EQ → base criminality | (0,0.1) (255,0) | (0,0.15) (100,0.11) (200,0) |
| `0x8A0CBAA6` | R$$$: EQ → base criminality | (0,0.1) (255,0) | (0,0.1) (100,0.07) (200,0) |
| `0x8A0CBAA7` | R$: jobless ratio → criminality | (1,0) (2,0.1) (5,0.5) | (−1,0.1) (−0.7,0.07) (0,0) (0.7,−0.07) (1,−0.1) |
| `0x8A0CBAA8` | R$$: jobless ratio → criminality | same | (−1,0.07) (−0.7,0.05) (0,0) (0.7,−0.05) (1,−0.07) |
| `0x8A0CBAA9` | R$$$: jobless ratio → criminality | same | (−1,0.05) (−0.7,0.03) (0,0) (0.7,−0.03) (1,−0.05) |
| `0x8A0CBAAA` | criminality → crimes | (0,0) (1e6,50000) | (0,0) (200,1) (5000,5) (10000,10) (20000,25) (200000,40) (400000,55) (600000,60) (900000,65) |
| `0x8A0CBAAB` | (police − crime effect) → attenuation | (0,0) (255,1) | (0,0) (100,0.05) (200,0.2) (255,0.5) |
| `0x8A0CBAAC` | crime grid decay factor (f32) | 0.5 | 0.8 |
| `0x8A0CBAAD` | "Criminal Grid Decay Factor": reoffend multiplier for released criminals (f32) | 1.0 | 0.66 |

Other properties used: building `0xCA5B9306` (crime effect `{m, r}`, §6.10); prop `0x8AA864D5` (the prop's
crime incident type, read by the prop manager into `PropProfile.crimeIncidentID`; 0 = generic crime prop);
occupant property `0x029244DB` (preview occupants, foundations §5.2; occupants carrying it are ignored by §6.10).

- *[quirk]* If the exemplar is **missing**, only the three scalars are reset. The curves keep their previous
  contents, which are empty in a fresh process, and an empty curve is undefined (foundations §4.3). *[choice]*
  FreeCity4 applies the code-default curves in that case. The prop request curve, which has no default,
  evaluates to 0.
- *[quirk]* A tract-size change by the reload message affects only later grid creation. Existing grids keep
  their size, and every later use (§6.6 stamping, §6.9, §6.10, §11.2) reads the size from the grid itself.
  *(Corrected: the hotspot scaling also uses the grid's size, not the tuning value.)*

## 9. Random numbers *[compat]*

**Generator**: one file-local `cRZRandomSuper` (MT19937, foundations §6.1), shared by all crime simulator
instances. It is seeded once per process at static initialization (`0x00A74B70`) with the `cRZTime` value for
"now" XOR `0x6C09C3F6`, it is not saved, and only this class uses it.

**Seed value.** `cRZTime()` (`0x00470B50`) is not the raw local wall clock. It takes `time()`, splits it with
`localtime`, and rebuilds an **unsigned 32-bit count of seconds since 1901-01-01 00:00 UTC** (Julian day
2415386) from the local calendar fields: local date and time, plus the CRT `_timezone`, minus 3600 when the
library's own daylight rule says the local time is daylight time. That rule applies only when the CRT
`_daylight` is non-zero, and it is the fixed pre-2007 US rule whatever the host zone: daylight time runs from
the Sunday on or before April 8 at 02:00 to the Sunday on or before October 31 at 01:00 standard time. So:

- when the host's own rule agrees with that one, the value is `time() + 2177452800` (2 177 452 800 = 25 202
  days, 1901-01-01 to 1970-01-01), truncated to 32 bits;
- where the two rules disagree (modern US dates between the second Sunday of March and the library's April
  start, or between its October end and the first Sunday of November; any non-US zone with daylight time), it
  is one hour off that value;
- a local time that falls in the library's spring-forward hour is invalid and gives **0**, so the seed is then
  `0x6C09C3F6` itself.

Worked example: 2026-10-14 12:00:00 local in US Central (`_timezone` = 21600, `_daylight` = 1). The library rule
runs from 2026-04-05 to 2026-10-25, so it is daylight time: 12:00 + 6 h − 1 h = 17:00 UTC, which is 3 969 450 000
(`0xEC990010`) seconds after 1901-01-01, the same as `time() + 2177452800`. The seed is `0xEC990010 XOR
0x6C09C3F6` = `0x8090C3E6`. The same instant on 2026-10-28 (still daylight time on the host, standard time by
the library rule) gives 3600 more.

*[choice]* The seed comes from the clock, so no run can observe the difference. FreeCity4 seeds with
`(uint32)(time(NULL) + 2177452800) XOR 0x6C09C3F6`, which is the original's value whenever the two daylight rules
agree, and skips the library's time-zone arithmetic. The other file-local generators seeded "with `cRZTime()`"
(foundations §6) use the same value.

| Operation | Method | Draws |
|---|---|---|
| each crime attempt with a lot found (§6.6 step 5) | `RandomDoubleUniform()` | 1 |
| type choice (§6.7), only when the attempt is not stopped and there is ≥ 1 candidate | `RandomDoubleRangeUniform(0, total)` | 1 |
| `SelectProp` (§11.6), when its chosen list is non-empty | `RandomUint32Uniform(n)` | 1 |

`GetRandomLot` uses the **lot manager's** generator (two `RandomUint32Uniform` draws per try, a file-local
`cRZRandom` of the lot manager code *[inferred: instance]*), not this one. `SelectProp` runs when the prop
developer calls it, so its draws interleave with the monthly draws in prop-developer timing.

## 10. Messages posted

Both are posted (`MessagePost(msg, false)`, queued) as `cRZMessage2Standard`:

| ID | When | Fields |
|---|---|---|
| `0x0A159D95` crime committed | each incident (§6.8 step 3), after an arson fire is started | data1 = crime type ID; IGZUnknown = the lot; string = a new copy of the type name |
| `0x88EA4DFF` crime map updated | pipeline state 5 | void1 = the simulator (primary interface pointer, no AddRef); IGZUnknown = the simulator |

Subscribers in the shipped game: the police simulator (`0x0A159D95`, station crime statistics) and the map view
(`0x88EA4DFF`).

## 11. Interface behaviour

### 11.1 `GetCrimeGrid()` → `cISC4SimGrid<uint8>*` (no AddRef; §2)

### 11.2 `GetCrimeHotspot(uint8* value, int32* x, int32* z)` → bool

If `hotspotValue > 0` (signed): write the value's low byte, `S · hotspotX` and `S · hotspotZ` (city cells of
the tract corner, with `S` the **crime grid's** `GetTractSize()`) through each non-null pointer, and return
true. Otherwise return false and write nothing. *(Corrected: the earlier text said the tuning tract size.)*

### 11.3 `GetNearestCrimeSim(const cS3DVector3& pos, float radius)` / `GetNearestCrimeProp(...)`

Query the occupant manager (`GetOccupantsByBBox`, filter overload) with the box `(pos.x − r, −FLT_MAX, pos.z − r)` to
`(pos.x + r, +FLT_MAX, pos.z + r)`, and with:

- for sims: occupants with `GetType() = 0x496E5783` that are in occupant group `0x4102`;
- for props: occupants with `GetType() = 0xA823821E` that have property `0x8AA864D5`.

How the original queries *[compat for the result]* (`0x006A10F0` sims, `0x006A0E50` props): it allocates a
12-byte filter object per call (global heap C; vtables `0x00AB0D4C`/`0x00AB0D34` for sims,
`0x00AB0D18`/`0x00AB0D00` for props) whose slot 3 `IsOccupantIncluded` is exactly the test above (`0x0069F4D0`,
`0x0069F490`). It calls occupant-manager **slot 36** `(list, &min, &max, filter, 0xFFFFFFFF)` on a fresh `ilist`
(head from the framework service, A). When that returns true it walks the list and then calls slot 48
`ReleaseOccupantList`. Finally it frees the nodes and the head and releases the filter. The manager calls only
the filter's slot 3, once per occupant in the box, and appends the accepted ones in its own walk order.

*[choice]* Equivalent: the typed overload, slot 37 `(list, &min, &max, type, 0xFFFFFFFF)` with type
`0x496E5783` (sims) or `0xA823821E` (props), followed by the group test (sims) or the property test (props) on
each listed occupant **before** it can become a candidate. Slot 37 stores the type in the manager's own filter,
whose test is `GetType() == type` (`0x004BDF90` → `0x005E9740`), and forwards to slot 36. The walk is the same, so
the qualifying occupants and their order are the same. The extra AddRef/Release of the non-qualifying ones is not
observable.

Start with `best := radius`. For each result, `d :=` its 3-D distance to `pos` (`GetPosition`), computed as
`float32` operations in this order *[compat]* (foundations §18): `dx := occ.x − pos.x`, `dy := occ.y − pos.y`,
`dz := occ.z − pos.z`, then `d := sqrt((dy·dy + dz·dz) + dx·dx)`, every subtraction, product, sum and the square
root rounded to `float32`. The query box corners `pos.x ∓ r`, `pos.z ∓ r` are `float32` too. The test is strict
(`d < best`; equal or NaN distances never win). If `d < best`, QI it (sim:
`0xA9B40F05`; prop: `cISC4PropOccupant` `0xE9793A65`, which must also have `GetState() = 0`). When that
succeeds, it becomes the candidate and `best := d`. Return the candidate as a **borrowed** pointer (the QI
reference is released before returning), or null.

### 11.4 `GetCrimeIncidentArray(SC4Vector<tCrimeEvent>& out)` → bool

Resize `out` to the incident count. For each incident that has a lot:
`out[i] = {cellX, cellZ from city.PositionToCell(lot.GetCenter()), type ID, visuals flag, icon key}`.
Elements of incidents without a lot (possible after a load) are not written. Return `count > 0` (signed).

**Container and allocator** *[compat]* (`0x006A1960`; resize `0x006A1380`, growth `0x0069FE90`):

- `out` is a `std::vector<tCrimeEvent>` with the default allocator, so its buffer is a **B** block (the STL node
  allocator, `docs/abi/allocators.md`): allocated with `0x0090CF54`, freed with `0x0090CF63`.
- Growing beyond the capacity allocates a new B buffer of `size + max(size, added)` elements (exactly the count
  when `out` is empty), copies the old elements and frees the old buffer with B. Shrinking only moves `end`, and
  a resize within the capacity keeps the buffer.
- The only caller (the traffic effect manager, `0x00553370`) passes a zeroed vector (no buffer) on every call. It
  frees the buffer with `0x0090CF63` afterwards, whatever the return value.
- A replacement must allocate the buffer with B and free any buffer it replaces with B. `SC4Vector` (allocator A)
  is not compatible.
- New elements are copies of the resize's fill value. That is a stack temporary whose icon key (+0x10..+0x1B) is
  zeroed and whose first 16 bytes are uninitialised (`0x006A198B`–`0x006A1997`). So an incident without a lot
  yields an element with icon key (0, 0, 0) and garbage cell, type and visuals fields. *[choice]* FreeCity4
  zero-fills new elements.

`tCrimeEvent` is **28 bytes**: `u32 cellX; u32 cellZ; u32 typeID; u8 visualsCreated; 3 bytes padding;
cGZPersistResourceKey iconKey (type, group, instance)`. *[compat]* The traffic effect manager walks the array
with this stride (§14.1).

### 11.5 `SetVisualsCreated(uint32 i, bool b)` / `GetVisualsCreated(uint32 i)`

Index < count: set and return true / return the flag. Otherwise return false.

### 11.6 Prop requester (`cISC4PropRequester`)

| Slot | Method | Behaviour |
|---|---|---|
| 3 | `GetRequestedShare(const SC4Rect<long>& r)` → int32 | `a := crimeGrid.GetAverageValueInCellRect(r.x1, r.z1, r.x2 − r.x1, r.z2 − r.z1)` (no +1, unlike §6.6); return `rhe(propRequestCurve(f32(a × k)))`, `k` the `float32` constant 1/255. The curve output lies between its smallest and largest y (clamped at both ends), at most 10 with shipped data; `rhe` leaves the exact range only for a modded curve with \|y\| > 2²² (§6 notation gives the result) |
| 4 | `GetRequesterSatisfactionId()` → u32 | `0x49A1E996` |
| 5 | `UsesCustomPropSelector()` → bool | true |
| 6 | `SelectProp(const vector<u32>& props, int32 cellX, int32 cellZ)` → u32 | see below |

**`SelectProp`**: `lot := lotManager.GetLot(cellX, cellZ, false)` (may be null). Split `props` in order:

- props whose `PropProfile.crimeIncidentID` (prop manager `GetPropProfile`) is 0 → generic list. The field is
  the `u32` at **+0x18** (24) of the profile that `GetPropProfile` returns (`0x006A016E`), the prop's property
  `0x8AA864D5` *[compat]*;
- props with a non-zero ID that matches the type of an incident whose lot is `lot` → matching list (each
  prop at most once). The lot test compares the pointers, so when the cell has **no lot** (`lot` null),
  incidents whose lot is null (after a load with a null lot ref, §12) match *[compat]*;
- other props are dropped.

If the matching list is non-empty, return `matching[RandomUint32Uniform(size)]`. Otherwise, if the generic list
is non-empty, return `generic[RandomUint32Uniform(size)]`. Otherwise return 0.

*[quirk]* With a 1-wide rect, `GetRequestedShare` asks for a zero-width average. It gets the containing tract's
value if the cell is not on a tract's first column or row, and 0 otherwise (simgrid §4).

## 12. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0x8990C09A`) *[compat]*

```
u16  version = 6
ref  crimeGrid                       (read requests IID 0xC9BD939A)
ref  crimeEffectGrid                 (same)
iterator                             (weather.md §10 format: s8 state, s8 mode, u8 dirty, s32 x0, z0, x1, z1,
                                      s32 count, u32 n, n × u16, s32 index, s32 current, s32 step)
u32  unused0 = 0
u32  unused1 = 0
f32  averageCrime
f32  ordinanceFactor
u32  monthlyCrimeCount
s32  hotspotTractX
s32  hotspotTractZ
s32  hotspotValue
s32  incidentCount
incidentCount × {
    ref  lot                         (read requests cISC4Lot IID 0xE818FCED; may be null)
    u32  crimeTypeID
    GZStr name                       (foundations §2)
    ResKey iconKey                   (u32 type, u32 group, u32 instance)
}
```

- `Write` returns `!stream.GetError()`.
- `Read` rejects any version ≠ 6 and returns false after consuming only the version (`loadedFromSave` stays
  clear). Otherwise it reads all fields, appends the incidents to the current list (it does not clear it), sets
  every incident's visuals flag to false, sets `loadedFromSave`, and returns `!GetError()`.
- **Stream error** (foundations §2.1, §2.9; the loader ignores the result, foundations §3.5; `Read`
  `0x006A05E0`, whose only early exit is the version test). The original does not stop at the first failure: it keeps calling the stream (each failed read leaves its destination
  unchanged), **sets `loadedFromSave` anyway** and returns false. Fields read before the failure hold the loaded
  values. The exceptions after it: a grid ref becomes **null** (the ref helper clears the destination first, without
  releasing an old grid); the iterator's table is resized to *(the stream object's address)* entries (the count
  is read into the argument slot that held the stream pointer), a multi-hundred-MB allocation or an out-of-memory
  failure; if `incidentCount` fails it is an uninitialised local, and each of that many loop passes allocates an
  incident with a null lot, an uninitialised type ID and empty name and icon key. With null grids the next
  `Init` (which skips new-city setup, §4.1 step 8) leaves the simulator unusable. *[choice]* FreeCity4: on a
  stream error `Read` returns false, does **not** set `loadedFromSave`, releases any refs it acquired and leaves
  the simulator as before the call, so the next `Init` sets the city up as new (grids, iterator, prop-requester
  registration).
- On the following `Init`, the hotspot and the ordinance factor are reset (§4.1 step 9), and the tuning is
  re-read. The pipeline state restarts at 6.
- *[choice]* FreeCity4 writes the same record, with the two unused values as 0.

## 13. Dependencies

Callee-pop and return channel checked with `tools/abi_extract.py --check`. "Bytes" is callee-pop.

| Interface (vtable used) | Slot | Off | Method | Bytes | Return |
|---|---|---|---|---|---|
| `cISC4City` (`0x00A87620`) | 33 | `+0x84` | GetLotManager | 0 | EAX ptr |
| | 34 | `+0x88` | GetOccupantManager | 0 | EAX ptr |
| | 36 | `+0x90` | GetZoneManager | 0 | EAX ptr |
| | 41 | `+0xA4` | GetPropDeveloper | 0 | EAX ptr |
| | 50 | `+0xC8` | GetSimulator | 0 | EAX ptr |
| | 56 | `+0xE0` | GetDemandSimulator | 0 | EAX ptr |
| | 63 | `+0xFC` | GetOrdinanceSimulator | 0 | EAX ptr |
| | 65 | `+0x104` | GetPoliceSimulator | 0 | EAX ptr |
| | 68 | `+0x110` | GetResidentialSimulator | 0 | EAX ptr |
| | 72 | `+0x120` | GetDisasterLayer | 0 | EAX ptr |
| | 95 | `+0x17C` | PositionToCell(f32 x, f32 z, int& cx, int& cz) | 16 | bool |
| `cISC4Simulator` (`0x00AB2FB8`) | 19 | `+0x4C` | AddAgent(target, level, name, flags) | 16 | bool |
| | 20 | `+0x50` | RemoveAgent(target) (all levels) | 4 | bool |
| `cISC4PoliceSimulator` (`0x00AB2490`) | 5 | `+0x14` | GetPolicePowerGrid → `cISC4SimGrid<uint8>*` | 0 | EAX ptr |
| | 27 | `+0x6C` | ResetUnjailedCriminals → s32 released count | 0 | EAX |
| | 30 | `+0x78` | GetJailPopulation → u32 (Σ rounded per-jail populations; protection §15.2) | 0 | EAX |
| | 38 | `+0x98` | ProcessMonthlyCrimeStats (result ignored) | 0 | none used |
| `cISC4ResidentialSimulator` (`0x00AB2C10`) | 23 | `+0x5C` | GetEQGrids(g0*&, g1*&, g2*&) | 12 | none (garbage) |
| | 25 | `+0x64` | GetPopulationGrids(g0*&, g1*&, g2*&) | 12 | none (garbage) |
| `cISC4DemandSimulator` (`0x00AB0E48`) | 6 | `+0x18` | GetDemand(id, 0) | 8 | EAX ptr |
| `cISC4Demand` (`0x00AA2758`) | 12, 14, 15 | `+0x30/+0x38/+0x3C` | active value / max / min (demand spec §3) | 0 | ST0 float |
| `cISC4OrdinanceSimulator` (`0x00ABEAC0`) | 25 | `+0x64` | GetCumulativeFactor(id, 0) | 8 | ST0 float |
| `cISC4LotManager` (`0x00AB37C0`) | 6 | `+0x18` | GetLot(int x, int z, bool) | 12 | EAX ptr |
| | 13 | `+0x34` | GetRandomLot() | 0 | EAX ptr |
| `cISC4Lot` (`0x00AB19B0`) | 14 | `+0x38` | GetBoundingRect(SC4Rect<long>&) | 4 | EAX |
| | 15 | `+0x3C` | GetCenter(cS3DVector3& out) → `&out`: writes x = 8·(x1 + x2 + 1), y = the lot's altitude, z = 8·(z1 + z2 + 1) for the bounding cells of slot 14 (exact in `float32`). The **x87 stack is empty** on return (`0x006BF5B6` pops the last value); the `x87` tag in docs/abi is a heuristic miss. A caller must not pop ST0 | 4 | EAX = `&out` |
| | 21 | `+0x54` | GetLocation(int& x, int& z) | 8 | bool |
| | 35 | `+0x8C` | GetCondition → u8 | 0 | AL |
| | 48 | `+0xC0` | GetBuilding → `cISC4BuildingOccupant*` | 0 | EAX ptr |
| `cISC4BuildingOccupant` | 3 | `+0x0C` | AsOccupant *[inferred name]* | 0 | EAX ptr |
| | 11 | `+0x2C` | GetBuildingProfile (purpose at +0, wealth at +4) | — | EAX ptr |
| `cISC4ZoneManager` (`0x00A92E00`) | 12 | `+0x30` | GetZoneType(x, z) | 8 | EAX enum |
| `cISC4OccupantManager` (`0x00A8F7F0`) | 36 | `+0x90` | GetOccupantsByBBox(list&, const float min[3], const float max[3], filter*, flags) — the original passes its own filter and flags `0xFFFFFFFF` (`0x006A121D`, `0x006A0F7D`); §11.3 | 20 | AL |
| | 37 | `+0x94` | the typed overload `(list&, min, max, type, flags)`: FreeCity4's equivalent (§11.3) | 20 | AL |
| | 48 | `+0xC0` | ReleaseOccupantList (one Release per element, nodes freed with A; allocators.md) | 4 | — |
| `cISC4Occupant` (`0x00AA41D8`) | 6 | `+0x18` | AsPropertyHolder | 0 | EAX ptr |
| | 7 | `+0x1C` | GetType | 0 | EAX |
| | 8 | `+0x20` | GetPosition(cS3DVector3&) | 4 | EAX |
| | 22 | `+0x58` | IsOccupantGroup(id) | 4 | bool |
| `cISC4PropOccupant` (`0x00AA4900`) | 12 | `+0x30` | GetState | 0 | EAX |
| `cISCPropertyHolder` (`0x00AA6CD8`) | 3 | `+0x0C` | HasProperty(id) | 4 | bool |
| | 9 | `+0x24` | GetProperty(id) | 4 | EAX ptr |
| `cIGZVariant` (`0x00AA6788`) | 95 | `+0x17C` | RefUint8 (pointer to the u8 data) | 0 | EAX ptr |
| `cISC4PropDeveloper` (`0x00AB29F8`) | 5 | `+0x14` | AddPropRequester(requester) | 4 | — |
| `cISC4PropManager` (`0x00A8F928`) | 13 | `+0x34` | GetPropProfile(id) → `PropProfile&` (crime reads only `crimeIncidentID`, `u32` at +0x18) | 4 | EAX ptr |
| `cISC4DisasterLayer` (`0x00A9E390`) | 7 | `+0x1C` | GetDisasterManager(type) | 4 | EAX ptr |
| `cISC4DisasterManagerFire` (`0x00A9E4DC`, sub-object of `cSC4DisasterManagerFire`) | 4 | `+0x10` | IgniteOccupant(occ) | 4 | bool |
| | 9 | `+0x24` | SetCause(cause) | 4 | EAX |
| `cISC4SimGrid<T>` | — | — | simgrid §4: GetCellValue, GetAverageValueInCellRect, SetTractSize, Get/SetInstanceID, GetTractSize, GetTractShift, GetTractCountX/Z, PositionToTract, GetGridData, SetTractValues, Init, Shutdown | | |
| message server / `cIGZMessage2` | | | MessagePost `+0x10`, Add/RemoveNotification `+0x14/+0x18`; SetType `+0x14`, GetData1 `+0x24`, GetVoid1 `+0x28`, SetData1 `+0x2C`, SetVoid1 `+0x30`, GetData2 `+0x38`, GetData3 `+0x4C`, SetString `+0x78`, SetIGZUnknown `+0x84` (foundations §5) | | |
| resource manager | | | exemplar lookup and key lists (foundations §3) | | |

The disaster-manager QI for `0x49B43F57` returns the sub-object at `+0x24` of `cSC4DisasterManagerFire`, whose
vtable is `0x00A9E4DC`.

## 14. Interactions and replaceability

### 14.1 Consumers (all through `SL::spCrimeSim` or `cISC4City +0xDC`; complete for the global)

| Consumer | Uses |
|---|---|
| Safety advisor (`update_variables`) | `GetAverageCrime`, `GetMonthlyCrimeCount`, `GetCrimeHotspot`; it is the only writer of history `0xAA662693` (GA_CRIME), recording the low byte of `GetAverageCrime` (advisors.md §7.6) |
| Aura simulator `DoIdle` | `GetCrimeGrid` → two-grid walker (simgrid §7.3) that reads the **grid memory** through `GetGridData` |
| Growth developer `DoIdle` / `EstimateDesirability` | `GetCrimeGrid` → a two-grid walker (raw memory); `GetCellValue` → desirability curve |
| Map view `Update` / `UpdateHighlights` | `GetCrimeGrid` → `GetGridData` (raw rows) for the crime layer; `GetCrimeHotspot` |
| Query tool | `GetCrimeGrid` → `GetCellValue` |
| MySim / MyStreetSim trip variables | `GetCrimeGrid` → cell values |
| Ambience filter | `GetCrimeGrid` → `GetCellValue ≥ 200` (crime ambience) |
| Getaway van `PostInit` | `GetCrimeGrid` |
| Helicopter, patrol car | `GetNearestCrimeSim` (`+0x24`), `GetCrimeHotspot` |
| Traffic effect manager `StopEffect` | `GetCrimeIncidentArray` with a 28-byte stride, then `SetVisualsCreated(i, true)` |
| Police simulator | subscribes to `0x0A159D95`; called by state 4 (`ProcessMonthlyCrimeStats`) |
| Prop developer | calls the prop-requester slots; saves the requester by ref |
| City / class store | creates, `Init`/`Shutdown`, save/load |

No caller of `GetNearestCrimeProp` or `GetVisualsCreated` was found. No code outside the class calls its
functions directly or reads its members. The only outside references are the COM director's class table
(factory and `GetGZCLSID`).

### 14.2 Replaceability findings

- **Replaceable** through the GZCOM class registry (CLSID `0x8990C09A`): the city and the class store create it
  only through GZCOM.
- **The crime grid must be a real `cSC4SimGrid<uint8>` memory layout** *[compat]*. The aura simulator, the growth
  developer and the map view read its rows through `GetGridData` (vector of row vectors, three pointers each,
  indexed `[x][z]`). A replacement should obtain both grids from the original grid class
  (`GetClassObject(0x49B9E602, 0xC9BD939A)`) or reproduce the layout exactly. The crime-effect grid has no outside
  reader, but it is saved by ref, so it must be a serializable grid object as well.
- **Prop requester identity** *[compat]*: the prop developer saves its requesters by ref and restores them on
  load, with a QI for `0x89C498BE`. `Init` registers only in new cities. A replacement must answer
  `0x89C498BE` on the **same object** that the class store saves for CLSID `0x8990C09A`. Otherwise loaded cities
  lose crime props, or the load fails.
- **`GetCrimeIncidentArray` resizes the caller's vector** *[compat, blocker for a naive port]*. The storage is
  owned and later freed by game code, so a replacement must allocate it with the game's **STL node allocator**
  (B: `0x0090CF54` / `0x0090CF63`, §11.4), not its own CRT heap or the framework service. The element layout
  must be exactly the 28-byte `tCrimeEvent` of §11.4.
- `GetCrimeGrid` and `GetNearestCrimeSim/Prop` return borrowed pointers. Callers do not release them.
- The police simulator depends on `0x0A159D95` with the fields of §10, and on being called once per month
  through `ProcessMonthlyCrimeStats` after `ResetUnjailedCriminals`. A replacement must keep both.

## 15. Original quirks (reproduced) and corrections to the earlier write-up

Quirks: hotspot and ordinance factor reset by `Init` after load (§4.1); first run only after the first month
(§5); EQ bucket boundaries and the EQ > 200 overflow (§6.1); `B = C` in the jobless factor (§6.2); EQ curve
sampled at 0/25/50/75 (§6.3); decay wrap (§6.5); last-candidate fallback (§6.7); severity-0 types (§6.8);
NaN average in an unzoned city, read back as 0 (§6.9); `rhe` beyond 2²² (§6, §6.4); grid tract size used after
a tuning reload (§6.9, §11.2); null-lot incidents matching lot-less cells (§11.6); unchecked effect array
length (§6.10); empty mask arrays (§7); missing exemplar leaves the curves empty (§8); zero-width share rect
(§11.6); no prop-developer unregistration on `Shutdown` (§4.2); `Read` setting `loadedFromSave` after a
stream error (§12).

Corrections to `docs/subsystems/grid-sims.md` §6:

- The value subtracted is police slot `+0x78`, not `+0x74`. Erratum (M3): an earlier version of this spec called
  slot `+0x78` `GetCriminalCount`. It is `GetJailPopulation`, the sum of the rounded per-jail populations
  (protection §9). The jailed-criminal counter is slot `+0x7C`, and crime does not read it. The slot crime
  calls was right.
- `p` is the probability that an attempt is **stopped**. A crime happens when `u ≥ p`.
- EQ buckets are "≤ 50, ≤ 100, ≤ 150, ≤ 200", not "0–49, 50–99, …". The criminal base is `trunc(jobless term ×
  popSum + EQ term)` per wealth, with the EQ curve sampled at `25·b`.
- The fourth lot bit set is the lot **condition** (`GetCondition`), not density.

## 16. Open questions

1. *(resolved)* Nothing posts OrdinanceUpdated `0xC2C09554` after a load. Its only poster is `SetOrdinanceOn`
   (the message is built at `0x0080D714`; ordinance §4.4), and it posts only when an ordinance's on flag
   changes. The ordinance simulator's `Read` and its `Init` after a load call neither `SetOrdinanceOn` nor
   `AddOrdinance` (ordinance §8.1, §9.1). A loaded city therefore runs at factor 1.0 (§4.1 step 9) until some
   ordinance's on flag changes, by any route: a player or LUA toggle, `AddOrdinance` / `RemoveOrdinance`, or a
   month-end availability change of an ordinance that is on (ordinance §4.3, §5). The handler (`0x006A22D5`)
   ignores the message data and re-reads the cumulative factor over **all** on ordinances, so one toggle of any
   ordinance, even one with no crime effect, restores the full factor. Example: a city saved with Gambling
   (1.2) on loads with factor 1.0, so 10 crimes stay 10 instead of 12. After the player enacts Free Clinics,
   which has no crime effect, the factor is `1.2f` again and 10 crimes become `rhe(f32(10 × 1.2f))` = 12.
2. Lot condition values: *(partly resolved)* condition 4, the only one arson can hit, is an abandoned lot at its
   last decay stage; occupied lots get 0..3 (foundations §5.2). Conditions 5 and 6 (accepted by
   `CalculateEconomicFactors`) and building purposes 1–9 (purpose 5 is the one excluded from most masks) are
   still unknown.
3. *(partly resolved)* Occupant property `0x029244DB` marks preview occupants (foundations §5.2). The meaning of
   occupant group `0x4102` is unknown. The IID `0xA9B40F05` is presumably `cISC4Automaton`.
4. *(resolved)* The crime type list is in the resource manager's key-list order, which for the shipped types is
   ascending instance (§7); it is observable through §6.7.
5. *(resolved, §11.3 and §13)* The last argument of `GetOccupantsByBBox` is the flags word, `0xFFFFFFFF`; slot
   48 is `ReleaseOccupantList`.
6. The meaning of the two unused saved `u32` values.
7. The fire cause value 2 (arson) and what `SetCause` changes in the fire's behaviour.
8. *(resolved, §7)* Crime type names: the LTEXT key is (`0x2026960B`, `G` + language ID, `I`), then
   (`0x2026960B`, `G`, `I`), from values [1] and [2] of `0x8A416B2E`; the fallback is the inline string
   `0xAA12FCB0`, and then the name stays empty. With shipped data only Generic gets a (wrong) name.

## 17. Conformance tests (must exist in `engine/conformance`)

1. **Decay**: decay 0.5: 200 → 100, 255 → 127, 1 → 0, 0 stays 0. Decay 0.8f: 200 → 160, 255 → 204, 5 → 4.
   Decay 1.5: 200 → 44 (wrap). After the decay, the hotspot value equals the decayed value at the hotspot tract.
2. **Census buckets** (product rounded to `float32`): EQ 0 → 0; 49.99 → 0; 49.999996 (the float below 50) → 0;
   **50.0 → 1**; 99.99999 → 1; **100.0 → 2**; 149.99998 → 2; **150.0 → 3**; 199.99998 → 3. **EQ 200.0 → bucket 4**:
   the tract contributes to no bucket, and neither does a tract with EQ 250 (FreeCity4 *[choice]*, same result
   as the original).
3. **Jobless factor**: A = 0, B = 1000, C = −1000 → 0; A = 500 → 0.5; A = −1000 → −1. R$ shipped curve at
   0.5 → −0.05; at −1 → 0.1.
4. **Criminal base** (shipped curves): R$ pop {1000, 0, 0, 0}, jobless −1 → 500. R$ pop {1234, 0, 0, 0},
   jobless 0 → 493. EQ bucket 1 is evaluated at input 25 (R$ curve 0.375).
5. **Crimes to commit** (shipped): base − jailed = 200 → 1; 7 500 → 7; 15 000 → 17; 100 → 0; 1 000 000 → 65.
   With released = 10, reoffend 0.66f → +6. Base ≤ jailed → only the released term. Base 7 500, jailed 0,
   released 10, factor 1.0 → 13.
6. **Ordinance rounding** (product rounded to `float32`, then round half to even): 10 crimes × 0.95f → **10**
   (9.49999988 → 9.5 → 10); 50 × 0.95f → **48** (47.4999994 → 47.5 → 48); 30 × 0.95f → 28 (28.5, tie to even);
   15 × 1.2f → 18; 2 × 0.95f → 2.
7. **Attenuation** (shipped): pol 150, eff 0 → p = 0.125 (±1e-6), so a draw u = 0.12 stops the attempt and
   u = 0.13 commits it; equality (u = p exactly) commits; u = 1.0 commits even at pol 255 (p = 0.5). pol 255 → 0.5. pol 0, eff 50 → input −50 → p = 0 → always commits. Default curve: p = (pol − eff)/255.
8. **Stamp and hotspot**: S = 2, lot cells (10, 10)–(13, 11) → tracts x 5..6, z 5. Severity 60 on cells
   {0, 200} → {60, 255}. The hotspot becomes (tract 6, 5, value 255) if 255 is the first strict maximum.
   `GetCrimeHotspot` → (255, 12, 10). Hotspot value 0 → returns false, writes nothing.
9. **Type selection** (shipped types; the list is in ascending instance order, §7). Residential zone (type 1),
   wealth $, purpose 1, condition 1: candidates Generic 0, Fighting 3, Car Theft 3, Mugging 2, Flashing 4, Graffiti
   4, Pie Throwing 4 (total 20). u = 0.5 → r = 10 → Flashing (0x10, severity 40); u = 0.1 → r = 2 → Fighting; u =
   0.0 → r = 0 → Fighting. TPing needs wealth $$ or $$$. Arson needs condition 4. All weights 0 → the last
   candidate. A lot with no building → wealth `0x80000000` and purpose `0x100`. Order is observable: with the list
   reversed, u = 0.1 would commit Pie Throwing instead of Fighting.
10. **Mask parsing**: [0] → all; [] → 0 (never matches); [1, 3] → `0x5`; [9] → `0x100`; missing → all.
11. **Crime-effect cone**: `0xCA5B9306 = {100, 8}`, S = 2 → radius 4 tracts, slope 25. The centre +100;
    d = 1 → +75; d = 2 → +50; d = 3 → +25; d = √2 → +65 (`rhe(64.64)`); d = 4 → unchanged. Removal subtracts and
    clamps: cell 30 at the centre → 0. A 1-byte property is skipped by FreeCity4.
12. **Average crime**: zoned tracts with values {10, 20, 0} → 10.0 → `GetAverageCrime` = 10; {1, 2} → 1.5 → 1.
    Unzoned city → the stored average is NaN (`0xFFC00000`, saved as such) and `GetAverageCrime` = **0**. A grid
    created with S = 2, then a tuning reload to S = 4: §6.9 still samples zone types at `(2x, 2z)`, and
    `GetCrimeHotspot` reports `2 · hotspotTract`.
13. **Pipeline**: a new simulator does nothing before the first month message. After it, exactly one step per
    day/idle delivery. `0x88EA4DFF` is posted once per month, at the 6th delivery. A second month message during
    the run does not restart it.
14. **RNG draw count**: 5 attempts, 4 lots found, 1 stopped, 3 with candidates → 4 `RandomDoubleUniform` +
    3 `RandomDoubleRangeUniform` on the crime generator, in attempt order.
15. **Incident array**: 3 incidents → the vector has 3 elements of 28 bytes, with cell coordinates from the lot
    centres, the type IDs, visuals false and the icon keys. Returns true. 0 incidents → false.
    `SetVisualsCreated(1, true)` → true, then `GetVisualsCreated(1)` → true; index 3 → false.
16. **Prop requester**: average crime 51 over the rect → input 0.2 → share 4; 255 → 10; 0 → 0.
    `SelectProp` with props {A (crime ID 0), B (ID 0x11)} on a lot with a Graffiti incident → B. On a lot without
    one → A. With no props → 0.
17. **Save**: write → read → write is byte-identical (v6, with 0, 1 and 3 incidents and a null lot). Version 5
    or 7 → `Read` false. After load and `Init`: hotspot 0, ordinance factor 1.0, visuals flags false,
    pipeline state 6.
18. **Lifecycle**: `Init` twice → one set of agent registrations and subscriptions. `Init` on a new city calls
    `AddPropRequester` once; on a loaded city never. `Shutdown` removes the agent and the four subscriptions.
19. **`rhe` beyond 2²²** (§6 notation): x = 4,194,304 → 4,194,304; 4,194,305 → 4,194,304; 4,194,307 →
    4,194,306; 5,000,000 → 4,597,152; 8,388,608 → 6,291,456; 100,000,000 → 26,655,776; −5,000,000 →
    −5,805,696. Crimes to commit with a modded crimes curve giving `crimes` = −1 and factor 1.0 → 71,327,744.
20. **Read with a stream error** (FreeCity4 *[choice]*): a v6 record truncated inside the incident list →
    `Read` false, `loadedFromSave` clear, no grid or incident replaced; the next `Init` creates both grids and
    calls `AddPropRequester`.
21. **SelectProp, no lot**: an incident of type Graffiti (0x11) with a null lot (loaded from a save with a null
    lot ref); `SelectProp({A (ID 0), B (ID 0x11)}, x, z)` on a cell with no lot → B; on a cell whose lot has no
    incident → A.
22. **Nearest crime sim distance** (`float32`, order of §11.3): `pos = (0, 0, 0)`, a qualifying sim at
    `(2109, 1812, 3693)` → `d = 4622.7124` (`0x459075B3`); the order `(dx² + dy²) + dz²` would give 4622.7129
    (`0x459075B4`). With radius 4622.712890625 (`0x459075B4`) the sim is returned. A sim at `(3, 4, 0)` with
    radius 5 is not returned (`d = 5`, strict test).

## Compiled slot tables *[compat]* (see foundations §16, §17)

**`cISC4CrimeSimulator`**, vtable `0x00AB0DC8`, 14 interface slots (3 + 11) + destructor + one class slot.
No overloads. The SDK order, argument sizes and return types are **correct**. Every pointer return is EAX.
`GetAverageCrime` returns `uint32` in EAX: the original tail-calls the float-to-int helper, which also leaves the
high half in EDX. That is harmless, and docs/abi's "edx:eax?" tag is a heuristic artefact. The SDK's
`tCrimeEvent::unknown1` is the visuals-created flag.

| Slot | Off | Method | Pop | Return |
|---|---|---|---|---|
| 0 | `+0x00` | QueryInterface(iid, void**) | 8 | bool |
| 1 | `+0x04` | AddRef | 0 | u32 |
| 2 | `+0x08` | Release (≤ 1 → Shutdown, delete, return 0) | 0 | u32 |
| 3 | `+0x0C` | Init | 0 | bool (true) |
| 4 | `+0x10` | Shutdown | 0 | bool (true) |
| 5 | `+0x14` | GetCrimeGrid | 0 | EAX ptr (borrowed) |
| 6 | `+0x18` | GetAverageCrime | 0 | EAX u32 |
| 7 | `+0x1C` | GetMonthlyCrimeCount | 0 | EAX u32 |
| 8 | `+0x20` | GetCrimeHotspot(u8*, s32*, s32*) | 12 | bool |
| 9 | `+0x24` | GetNearestCrimeSim(const cS3DVector3&, float) | 8 | EAX ptr (borrowed) |
| 10 | `+0x28` | GetNearestCrimeProp(const cS3DVector3&, float) | 8 | EAX ptr (borrowed) |
| 11 | `+0x2C` | GetCrimeIncidentArray(SC4Vector<tCrimeEvent>&) | 4 | bool |
| 12 | `+0x30` | SetVisualsCreated(u32, bool) | 8 | bool |
| 13 | `+0x34` | GetVisualsCreated(u32) | 4 | bool |
| 14 | `+0x38` | scalar deleting destructor (class) | 4 | — |
| 15 | `+0x3C` | GetSimulatorType → `0x27567222` (class) | 0 | EAX |

**`cIGZMessageTarget2`**, vtable `0x00AB0DB8`: slots 0–2 thunks; slot 3 `DoMessage(cIGZMessage2*)`, pop 4,
returns true (AL = 1).

**`cISC4PropRequester`**, vtable `0x00AB0D9C` (no SDK header): slots 0–2 thunks; 3 `GetRequestedShare(const
SC4Rect<long>&)` pop 4, EAX int32; 4 `GetRequesterSatisfactionId()` pop 0, EAX; 5 `UsesCustomPropSelector()`
pop 0, AL (a shared one-instruction function returning true); 6 `SelectProp(const vector<u32>&, int32, int32)`
pop 12, EAX u32.

**`cIGZSerializable`**, vtable `0x00AB0D84`: slots 0–2 thunks; 3 `Write(cIGZOStream&)` pop 4, bool; 4
`Read(cIGZIStream&)` pop 4, bool; 5 `GetGZCLSID()` pop 0, EAX `0x8990C09A`.

**SDK errors found in interfaces this class calls** (for the owners of those specs):

- `cISC4PoliceSimulator::GetPolicePowerGrid` returns `cISC4SimGrid<uint8>*` (crime reads a byte average), not
  `<short>`.
- `cISC4PoliceSimulator::ResetUnjailedCriminals` (slot 27) returns an **int32 count** of released criminals, not
  bool. It also has side effects (resets the pending count, trims the jailed count to capacity).
- `cISC4ResidentialSimulator::GetEQGrids` / `GetPopulationGrids`: **all three** parameters are out pointers
  (`cISC4SimGrid<T>*&`), each AddRef'd. The SDK declares the 2nd and 3rd as plain pointers. The return value is
  garbage.
- `cISC4OccupantManager::GetOccupantsByBBox`: the compiled order is the **filter** overload at slot 36 (`+0x90`)
  and the type overload at slot 37 (it forwards to 36). The SDK declares them the other way round. The two
  float pointers are the 3-float **min and max corners**, not "X cells / Z cells".
- `cISC4LotManager::GetLot`: the compiled order is `GetLot(const cS3DVector3&)` at slot 5 (pop 4) and
  `GetLot(int, int, bool)` at slot 6 (pop 12). The SDK declares them the other way round.
