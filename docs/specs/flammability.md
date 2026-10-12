# Spec: the flammability simulator (`cISC4FlammabilitySimulator`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program (SimCity 4 Windows 1.1.641) and its data files.
Implementers must work from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must
**not** consult decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (streams §2, references §2.7, city components §3.6, Init order
§3.7, response curves §4, messages §5, random numbers §6, C `rand()` §12, slot order §16/§17),
[simulator.md](simulator.md) (agents), [simgrid.md](simgrid.md) (grid class §2, geometry §3, slots §4,
persistence §5, lifetime §6, iterators §7.4), [weather.md](weather.md) (season, random cell iterator §6.7),
[ordinance.md](ordinance.md) (`GetCumulativeFactor` §6).

## 1. Role

The flammability simulator keeps two per-tract byte maps of how easily the city burns, and starts fires:

1. a **raw** map: per tract, the flammability of what stands there (buildings first, else flora, else props),
   taken from occupant property `0x29244DB5`, raised for buildings and flora on abandoned lots. It is updated
   whenever an occupant is inserted, removed or changed;
2. an **adjusted** map (the one other subsystems see, `GetFlammabilityGrid`): raw × a city-wide multiplier
   (ordinances, summer) × a no-water multiplier, refreshed over a **40-day cycle** in random tract order;
3. at the end of each cycle: the **average** flammability of zoned tracts (also recorded in the history
   warehouse), the list of **hotspots** (clusters of very flammable tracts), and one **fire roll**: a random fire
   on a random lot, or a fire at a hotspot.

| Class | CLSID | Interfaces (IID) |
|---|---|---|
| flammability simulator | `0xE990C045` | `cISC4FlammabilitySimulator` (`0x48F41617`), `cIGZMessageTarget2` (`0x452294AA`), `cIGZSerializable` (`0xE4FDA3D4`), `cIGZUnknown` (`1`) |

`QueryInterface` answers exactly these four IIDs (with an `AddRef`). For any other IID it returns false and leaves
`*ppv` **unchanged**. It does not answer `cIGZMessageTarget` (`0xE98B2F57`).

The city creates it in `CreateCityComponents` with `GZCOM()->GetClassObject(0xE990C045, 0x48F41617, &p)` and
stores it in reference slot **26** of its save record (foundations §3.6, read with IID `0x48F41617`). Other code
reaches it through `cISC4City::GetFlammabilitySimulator()` (compiled city slot 58, `+0xE8`); the game caches that
pointer in a global when the city is initialised, and every consumer goes through the cached pointer (§13).

Init order (foundations §3.7): after budget, ordinance, power, **plumbing**, neighbours, building development,
tract/lot developers, demand, aura, commercial, crime, fire protection; **before flora and weather**. `Init` calls
no method of another simulator, so the order does not matter for `Init` itself.

## 2. Grids and coordinates

- Both maps are `cSC4SimGrid<uint8>` objects (class `0x49B9E602`, simgrid §2), x-outer (`grid[x][z]`).

  | Map | Instance ID | Exposed |
  |---|---|---|
  | adjusted | `0x49D5B953` | `GetFlammabilityGrid()` |
  | raw | `0x49D5B964` | not exposed |

- **Tract size** `S` is always **1 cell** *[compat]*: the field is set to 1 at every tuning read and no tuning
  property changes it (the earlier write-up's "tract size from tuning" is wrong). A grid created by `Init` gets
  `SetTractSize(S)`; a grid loaded from a save keeps its saved tract size. With `S = 1` a tract **is** a city
  cell, and this spec writes "tract" and "cell" interchangeably except where noted. Places where the original
  would misbehave with `S > 1` are listed in §14.
- `countX`, `countZ`: the **city's** `CellCountX/Z` (read at `Init`), used for bounds checks.
- The grid's own `GetTractCountX/Z` sizes the daily iterator and `tractsPerDay`.

## 3. State

| Field | Constructor | Saved | Meaning |
|---|---|---|---|
| initialized (bool) | false | no | §4 |
| refCount | 0 | no | `AddRef` / `Release` (§4.3) |
| rng: `cRZRandom` | `Seed(0)`, no cached Gaussian | no | §7 |
| cellIterator: random cell iterator | prime-step mode, state "running", **count 0**, step 1 | no | weather §6.7 / simgrid §7.4 |
| city, ordinance, weather, plumbing, occupant manager, lot manager, message server (held refs) | null | no | `Init` |
| countX, countZ (s32) | 0 | no | city cell counts |
| adjusted grid, raw grid | null | **by ref** | §2 |
| averageFlammability (u8) | 0 | yes | §6.6 |
| globalMultiplier (f32) | **0** | yes | §6.4 |
| zonedSum, zonedCount (s32) | 0 | no (zeroed by `Init`) | §6.5 |
| candidates: vector of `{s32 x, s32 z, s32 weight}` | empty | **no** | hot tracts found in the current cycle |
| hotspots: list of `{s32 x, s32 z, s32 weight}` | empty | yes | §6.7 |
| summerMultiplier, abandonmentMultiplier, noWaterMultiplier (f32) | 0 | no | tuning (§8) |
| tractSize (s32), cyclePeriod (s32) | 0 | no | set to 1 and 40 at each tuning read |
| tractsPerDay (s32) | 0 | no | §4.1 |
| hotspotRadius, hotspotThreshold, hotspotMinWeight (s32) | 0 | no | tuning |
| randomFireProbability (f32) | 0 | no | tuning |
| hotspotFireCurve: response curve or none | none | no | tuning |

Tuning fields are rewritten at every `Init` (§8), also after a load.

## 4. Lifecycle

### 4.1 `Init()` → true

If already initialized, return true. Otherwise:

1. Set initialized. Read the tuning (§8).
2. `simulator.AddAgent(this as cIGZMessageTarget2, level 4, "cSC4FlammabilitySimulator", flags 2)` (§5).
3. If no message server is held: hold it and `AddNotification` for, in this order, InsertOccupant `0x99EF1142`,
   RemoveOccupant `0x99EF1143`, ChangeOccupant `0x99EF1145`, SeasonChange `0x6716A4E5`, OrdinanceUpdated
   `0xC2C09554`, ConditionChanged `0xE99B3D4D`.
4. Hold the city (if not held); `countX := city.CellCountX()`, `countZ := city.CellCountZ()`.
5. Hold (each only if not yet held) the ordinance simulator, weather simulator, plumbing simulator, occupant
   manager and lot manager.
6. If there is no adjusted grid: create a `cSC4SimGrid<uint8>`, hold it, `Init()`, `SetTractSize(tractSize)`,
   `SetInstanceID(0x49D5B953)`, `SetTractValues(0)`. Same for the raw grid with `0x49D5B964`. (The original
   constructs the grid objects directly; a replacement must create them through GZCOM, class `0x49B9E602`, so that
   the class store can save and load them: simgrid §6.) After a load both grids exist and are kept as loaded.
7. `tractsPerDay := adjusted.GetTractCountZ() × adjusted.GetTractCountX() / cyclePeriod` (C integer division).
   64² → **102**, 128² → **409**, 256² → **1638**.
8. Cell iterator rectangle `(0, 0, adjusted.GetTractCountX() − 1, adjusted.GetTractCountZ() − 1)` (upper bounds
   raised to the lower bounds if smaller), marked dirty. The iterator is **not reset** *[compat]*: its count is
   still 0, so the first `next` after `Init` reports "exhausted" (§6.5, quirk §14.1).
9. `zonedSum := zonedCount := 0`.

`globalMultiplier` is **not** computed here. A new city keeps 0 until the first SeasonChange or OrdinanceUpdated
message; a loaded city keeps its saved value (quirk §14.2).

### 4.2 `Shutdown()` → true

If initialized: clear the flag; delete the hotspot-fire curve; for each grid `Shutdown()` then `Release()` and
forget it; release the ordinance, weather and plumbing simulators and the city; `RemoveNotification` for the six
message types of §4.1 and release the message server; release the occupant and lot managers; then
`city.GetSimulator().RemoveAgent(this)` (the one-argument overload, all levels). Hotspots, candidates and the
average are kept. Always returns true.

### 4.3 Reference counting and destruction

`AddRef` increments and returns the new count. `Release`: if the count is > 1, decrement and return the new
count; otherwise call `Shutdown()`, destroy the object and return 0 (the count is not decremented first). The
class's own virtual destructor sits in the slot after the interface (§11.1, slot 9).

## 5. Agent and messages handled

One agent registration: **level 4 (SimNewDay) only, flags 2** (runs only in an established city), name
`"cSC4FlammabilitySimulator"`. There is no month agent. Every message returns **true** (finished in one call).

`DoMessage(msg)` dispatches on `msg.GetType()`:

| Type | Handling |
|---|---|
| `0x66956814` SimNewDay | `DoDailySimulation` (§6.5). data1 (the Julian day) is read but not used. |
| `0x66956810` (level 0) | nothing (it is not subscribed). |
| `0x99EF1142` InsertOccupant | `occ := msg.GetVoid1()`. If `occ.AsPropertyHolder().HasProperty(0x029244DB)`: ignore. Else `OccupantUpdate(occ, add = true)` (§6.3). |
| `0x99EF1145` ChangeOccupant | same as InsertOccupant (**add = true**) *[compat]*. |
| `0x99EF1143` RemoveOccupant | same filter, then `OccupantUpdate(occ, add = false)`. |
| `0x6716A4E5` SeasonChange, `0xC2C09554` OrdinanceUpdated | `globalMultiplier := ComputeGlobalMultiplier()` (§6.4). The grids are not touched; the daily refresh spreads the change over the next cycle. |
| `0xE99B3D4D` ConditionChanged | `u := msg.GetIGZUnknown()`; if null, stop. `QueryInterface(u, 0xE818FCED cISC4Lot)`; if it fails, stop. `b := lot.GetBuilding()`; if non-null, `OccupantUpdate(b.AsOccupant(), add = false)` (a full recompute of the building's tracts). Release the lot. No `0x029244DB` check here. |
| anything else | nothing. |

Property `0x029244DB` is the **preview occupant** marker (foundations §5.2): the lot-placement and toll-booth tools
set it on the temporary occupants they show under the cursor, and every grid simulator skips those occupants.

Same-day ordering: the day agent reads the plumbing simulator's watered state (`IsCellWatered`) and the zone
manager's zone types as they stand when it runs. No result of another level-4 agent of the same day is needed
*[inferred]*; registration order is acceptable (foundations §13).

## 6. Algorithms

Rounding: `roundHE(x)` = round to nearest, ties to even (the original adds `1.5 × 2^23` on the x87 and keeps the
low bits; valid for `|x| < 2^22`). Products of an integer with a `float32` multiplier are formed exactly enough
that FreeCity4 should compute them in `f64` and round once *[choice]* (same convention as foundations §4.3).

### 6.1 Occupant raw value (`GetOccupantRawFlammability(occ)`) *[compat]*

```
t := occ.GetType()
if t == building 0x278128A0 or t == prop 0xA823821E:
    m := 1.0
    lot := lotManager.GetOccupantLot(occ)
    if lot ≠ null and lot.GetCondition() == 4:  m := abandonmentMultiplier      (4 = abandoned [inferred])
else if t == flora 0x74758926:  m := 1.0
else: return 0
p := integer value of occ's property 0x29244DB5 (0 if absent; read through the property holder)
return clamp(roundHE(p × m), 0, 255)
```

Values outside `|p × m| < 2^22` give garbage in the original; *[choice]* FreeCity4 clamps properly.

### 6.2 Tract raw value (`ComputeTractRawFlammability(tx, tz)`) *[compat]*

1. Cell rectangle `[tx·S, tx·S + S − 1] × [tz·S, tz·S + S − 1]` (S = the tuning tract size field, 1).
2. Ask the occupant manager for the occupants in it:
   `GetOccupantsByStandardCityCells(list, &{x0, z0}, &{x1, z1}, filter, 0xFFFFFFFF)` (compiled slot 40, the
   filter overload), where `filter` is a ref-counted occupant **type filter** accepting building `0x278128A0`,
   flora `0x74758926` and prop `0xA823821E`. Each returned occupant carries a reference, released after use.
3. `B` := max raw value (§6.1) over buildings, `P` := max over props (`0xA823821E`), `F` := max over the other
   occupants (flora). Result := `B` if `B > 0`, else `P` if `P > 0`, else `F` (clamped to 0..255; already in range).

A building or prop whose raw value is 0 counts as absent. (The original evaluates props only while no
non-zero building has been seen and flora only while neither has; the result is the same as the rule above.)
Erratum (M3): an earlier version of this section swapped the names of the prop and flora type IDs. The numeric
checks were right; prop is `0xA823821E` and flora is `0x74758926`.

### 6.3 Occupant update (`OccupantUpdate(occ, add)`) *[compat]*

```
if occ.GetType() ∉ {building, flora, prop}: return
v := GetOccupantRawFlammability(occ)
if add and v == 0: return
r := occ.GetBoundingCityCells()            (x0, z0, x1, z1), inclusive
r := r with every bound shifted LEFT by raw.GetTractShift()      [quirk §14.4; 0 with S = 1]
if x0 < 0 or x1 ≥ countX or z0 < 0 or z1 ≥ countZ:
    r := r ∩ (0, 0, countX − 1, countZ − 1)      (disjoint → (0, 0, 0, 0): tract (0,0) is updated, quirk)
for x in x0..x1, z in z0..z1:
    if add: raw[x][z] := max(raw[x][z], v);  w := v                     [quirk §14.3]
    else:   w := ComputeTractRawFlammability(x, z);  raw[x][z] := w
    adjusted[x][z] := clamp(roundHE(w × globalMultiplier), 0, 255)       (no no-water factor)
```

On removal the original recomputes from the occupant manager's current contents; whether the removed occupant is
still listed at that moment is the occupant manager's behaviour *[inferred: it is already gone]*.

### 6.4 Global multiplier (`ComputeGlobalMultiplier()`) *[compat]*

```
f := float32(ordinance.GetCumulativeFactor(0x28F42AA0, 0))        (ordinance §6; Smoke Detectors 0.95)
if weather.GetCurrentSeason() == 2 (summer):  f := f × summerMultiplier
globalMultiplier := float32(f)
```

Shipped: summer multiplier 1.0, so only Smoke Detectors matter (0.95). Code default summer 1.2.

### 6.5 Daily refresh (`DoDailySimulation`, SimNewDay) *[compat]*

```
n := tractsPerDay
while n ≠ 0:
    if not cellIterator.next(x, z):                 (x = outer grid index)
        EndOfCycle()                                (§6.6)
        cellIterator.reset(shuffle = true)          (one C rand() call, weather §6.7)
        continue                                    (n is not decremented)
    r := raw[x][z];  v := 0
    if r > 0:
        v := roundHE(r × globalMultiplier)
        if not plumbing.IsCellWatered(x, z):  v := roundHE(v × noWaterMultiplier)
        v := min(v, 255)                            (no lower clamp; v is stored as its low byte)
        adjusted[x][z] := v
        if v > hotspotThreshold:  candidates.append({x, z, 1})
    (r == 0: adjusted[x][z] is NOT written)                                     [quirk]
    if zoneManager.GetZoneType(x, z) ≠ 0:  zonedCount += 1;  zonedSum += v
    n -= 1
```

Each cycle visits every tract exactly once, `tractsPerDay` tracts per day; the wrap usually falls inside a day and
that day continues into the next cycle. Because the iterator count is 0 after `Init` (§4.1), the **first** call
after every `Init` (new city or load) wraps at once: an empty `EndOfCycle` runs on the first SimNewDay. For a
64 × 64 city `EndOfCycle` therefore runs on days 1 (empty), 41, 81, 121, 161, … (counting SimNewDays from `Init`);
128² and 256² cities also wrap on day 41 (409 × 40 = 16360 < 16384; 1638 × 40 = 65520 < 65536).

### 6.6 End of cycle (`EndOfCycle`) *[compat]*

1. `averageFlammability := zonedCount ≠ 0 ? (zonedSum / zonedCount) mod 256 : 0` (integer division);
   `zonedSum := zonedCount := 0`.
2. Post SimCycleComplete (§9).
3. `historyWarehouse.RecordHistoryData(0x2A11CB48, averageFlammability)`.
4. Rebuild the hotspot list from the candidates (§6.7), then clear the candidates.
5. Fire roll (§6.8).

### 6.7 Hotspot list (`ComputeHotspotList`) *[compat]*

```
hotspots := empty                                   (the old list is discarded)
for c in candidates, in discovery order:
    h := the FIRST hotspot in list order with |h.x − c.x| + |h.z − c.z| < hotspotRadius
    if h exists:
        w := h.weight
        h.x := (h.x · w + c.x) / (w + 1);  h.z := (h.z · w + c.z) / (w + 1)     (C integer division)
        h.weight := w + 1
    else: append {c.x, c.z, c.weight (= 1)}
remove every hotspot with weight < hotspotMinWeight
```

Coordinates are tract indices. The weight is the number of hot tracts merged into the hotspot during the cycle.

### 6.8 Fire roll (`EndOfCycle`, last step) *[compat]*

```
ts := city.GetTutorialSystem()
if ts ≠ null and ts.IsTutorial(): return                       (no random draw at all)
r := rng.RandomDoubleRangeUniform(0.0, 100.0)                  (one double-path draw, §7)
useProbability := true
if r < randomFireProbability:                                  (r compared before rounding to float32)
    lot := lotManager.GetRandomLot()
    b := lot ? lot.GetBuilding() : null
    if b ≠ null:
        pos := b.AsOccupant().GetPosition()
        (cx, cz) := city.PositionToCell(pos.x, pos.z)
        goto IGNITE
if hotspots is empty or hotspotFireCurve is none: return
order := a permutation of 0 .. |hotspots| − 1: identity, then std::random_shuffle (|hotspots| − 1 C rand() calls)
for i in order:
    h := hotspots[i]                                           (i-th in list order)
    if hotspotFireCurve(float(h.weight)) > float32(r):
        S' := adjusted.GetTractSize();  cx := h.x · S' + S' / 2;  cz := h.z · S' + S' / 2
        useProbability := false
        goto IGNITE
return
IGNITE:
    m := city.GetDisasterLayer();  if m == null: return
    fire := QueryInterface(m.GetDisasterManager(0x621CDA33), 0x49B43F57)   (cISC4DisasterManagerFire)
    fire.IgniteCell(cx, cz, useProbability, false, 0)
    fire.SetCause(5)
    release fire
```

- The same `r` serves both tests. A random fire is tried with probability `randomFireProbability %` per cycle;
  if it finds no lot or no building, the hotspot test still runs with that (small) `r`.
- With `useProbability = true` the fire manager applies its own ignition chance, a curve of the adjusted
  flammability at the cell and its own generator. A hotspot fire ignites without that check.
- The original does not check the fire manager for null before calling it. *[choice]* FreeCity4 skips the
  ignition if the layer, the manager or the interface is missing.
- The property descriptions call these probabilities "monthly". They are **per cycle** (about 40 days).
- Shipped odds per cycle: random fire 5 %; otherwise, with `M` = the largest curve value over the hotspots, a
  hotspot fire happens when `r < M`, i.e. with probability `(M − 5) / 95` for `M > 5` (weight 40 → 7 → 2.1 %;
  weight ≥ 100 → 25 → 21 %).

## 7. Random numbers

| Draw | Generator | Method | Count |
|---|---|---|---|
| fire roll | the simulator's own `cRZRandom`, seeded **0** at construction | `RandomDoubleRangeUniform(0, 100)` | 1 per `EndOfCycle` outside the tutorial; 0 in the tutorial |
| daily iterator start | C runtime `rand()` | `reset(shuffle)` in prime-step mode | 1 per wrap (after the `EndOfCycle`) |
| hotspot visiting order | C runtime `rand()` | `std::random_shuffle` of `0 … n−1` (foundations §12.3) | `n − 1` per `EndOfCycle` that reaches the hotspot test with a non-empty list and a curve |
| random lot | the lot manager's own generator | `GetRandomLot()` | only when `r < randomFireProbability` |

Only the double path of the generator is used. Seed 0 makes its integer path deterministic, but the double path
seeds itself from the process clock on first use (foundations §6.1), so **the fire roll is effectively
clock-seeded** *[compat]*. The generator is not saved. Per `EndOfCycle` the `rand()` calls come in this order:
the shuffle (if any), then the iterator reset. *[choice]* FreeCity4 seeds the generator from the host
(foundations §6.3) and uses one `CrtRand` per subsystem for the `rand()` users (foundations §12.6).

## 8. Tuning

Exemplar **`0x6534284A / 0xE7E2C2DB / 0xA8F41EE3`** "Flammability Simulator" (`SimCity_1.dat`) *[compat]*,
loaded through the resource manager as `cISCResExemplar` (`0xA52160F5`) and read through its property holder.
Code defaults are set first; a missing property keeps the default; a missing exemplar keeps all defaults.

| Property | Name (Maxis) | Type | Shipped | Code default | Field |
|---|---|---|---|---|---|
| `0xC9B40128` | Flammability multiplier, summer | f32 | 1 | 1.2 | summerMultiplier |
| `0x29B40134` | Flammability multiplier, abandonment | f32 | 1.5 | 1.1 | abandonmentMultiplier |
| `0x29B4013B` | Flammability multiplier, no water | f32 | 1.25 | 1.1 | noWaterMultiplier |
| `0x29B4013C` | Flammability, hotspot settings | u32[3] | 54, 8, 40 | 196, 8, 16 | {threshold, radius (tracts, Manhattan, exclusive), minimum weight}; used only if the array has **exactly 3** values |
| `0x29B4013D` | Flammability, hotspot fire probability | f32 curve | (0,0) (39,0) (40,7) (50,15) (70,20) (100,25) | none | weight → % per cycle (foundations §4); deleted and rebuilt at each read; absent → no hotspot fires |
| `0x29B4013E` | Flammability, random fire probability | f32 | 5 | 0.01 | % per cycle |
| — | (not tunable) | | | 1 / 40 | tract size / cycle period (days per full refresh) |

Shipped curve values: f(40) = 7, f(45) = 11, f(60) = 17.5, f(≥ 100) = 25.

## 9. Messages posted

| Type | When | Fields | Delivery |
|---|---|---|---|
| `0x08F415FE` SimCycleComplete | every `EndOfCycle` (§6.6 step 2), including the empty one on the first day | void1 = this (`cISC4FlammabilitySimulator*`), IGZUnknown = this | `MessagePost(msg, false)` |

No subscriber of `0x08F415FE` was found in the executable *[inferred]*. The fire manager calls of §6.8 post the
fire manager's own messages. History type `0x2A11CB48` has no other reference in the executable; graph
definitions in data files may read it *[inferred]*.

## 10. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0xE990C045`) *[compat]*

```
u16  version = 2
ref  adjusted grid           (WriteGZSerializable; on read IID 0xC9BD939A cISC4SimGrid; simgrid §5)
ref  raw grid
f32  globalMultiplier
u8   averageFlammability
u32  hotspotCount
hotspotCount × (s32 x, s32 z, s32 weight)        (list order)
```

- **Write** returns `GetError() == 0`. The two grids are written as their own objects through the reference
  mechanism (foundations §2.7): adjusted first, then raw.
- **Read**: the version must be exactly **2**; otherwise return false without reading further. Then read the two
  references into the adjusted and raw slots, the multiplier, the average, the count and the hotspots, appending
  to the (empty) list. Return `GetError() == 0`.
  *[quirk]* The original reads the count into an uninitialised variable, so a truncated record can make it loop
  over a garbage count; *[choice]* FreeCity4 initialises it to 0. If the version read itself fails, the original
  compares garbage (practically always rejects).
- Not saved: candidates, zoned sum/count, iterator, generator, tuning, service pointers, counts.
- Recomputed by `Init` after a load: tuning, `countX/Z`, `tractsPerDay`, iterator rectangle. The saved hotspots
  and average are **discarded on the first SimNewDay** by the empty `EndOfCycle` (§6.5, §14.1).

Example payload (after the version, the two references): `0.95, 50, one hotspot (11, 10, 3)` →
`33 33 73 3F | 32 | 01 00 00 00 | 0B 00 00 00 0A 00 00 00 03 00 00 00`.

## 11. Interfaces and slots

### 11.1 Compiled slot tables *[compat]* (see foundations §16, §17)

`cISC4FlammabilitySimulator`, original vtable `0x00AB128C`, 9 interface slots (3 + 6) plus the class's
destructor. No overloads. Every slot matches the SDK header in order, argument size and return type.

| Slot | Offset | Signature (as the game expects it) | Pops | Return | Behaviour |
|---|---|---|---|---|---|
| 0 | `+0x00` | `bool QueryInterface(uint32 iid, void** ppv)` | 8 | AL | §1 |
| 1 | `+0x04` | `uint32 AddRef()` | 0 | EAX | §4.3 |
| 2 | `+0x08` | `uint32 Release()` | 0 | EAX | §4.3 |
| 3 | `+0x0C` | `bool Init()` | 0 | AL | §4.1, always true |
| 4 | `+0x10` | `bool Shutdown()` | 0 | AL | §4.2, always true |
| 5 | `+0x14` | `cISC4SimGrid<uint8>* GetFlammabilityGrid()` | 0 | EAX (pointer) | the **adjusted** grid, no `AddRef`. The original body is shared (COMDAT) with `cSC4AuraSimulator::GetParkMap`: both just return their own member at the same offset; nothing else is implied |
| 6 | `+0x18` | `bool IsCellFlammable(int32 x, int32 z)` | 8 | AL | false unless `0 ≤ x < countX` and `0 ≤ z < countZ` (city cells); else `adjusted.GetCellValue(x, z) ≠ 0` |
| 7 | `+0x1C` | `bool GetFlammabilityHotspot(uint8* value, int32* x, int32* z)` | 12 | AL | below |
| 8 | `+0x20` | `uint8 GetAverageFlammability()` | 0 | AL | `averageFlammability` |
| 9 | `+0x24` | (class) scalar deleting destructor `(uint32 flags)` | 4 | EAX | not part of the interface; only the object's own `Release` calls it |

`GetFlammabilityHotspot`: pick the hotspot with the **largest weight** (strictly larger wins, so the first of
equal weights in list order; weights must be > 0). Let `v := adjusted.GetTractValue(h.x, h.z)` at that hotspot.
If the list is empty or `v == 0`, return false and **write nothing**. Otherwise write (each only if its pointer
is non-null) `*value := v`, `*x := adjusted.GetTractSize() × h.x`, `*z := adjusted.GetTractSize() × h.z` (cell
coordinates of the tract corner) and return true. The safety advisor calls it and uses the outputs without
looking at the result, after pre-setting some of them to 0 *[compat: keep "write nothing" on false]*.

`cIGZSerializable`, vtable `0x00AB1264`: 0 `QueryInterface` (thunk, 8), 1 `AddRef` (thunk, 0), 2 `Release`
(thunk, 0), 3 `bool Write(cIGZOStream&)` (4), 4 `bool Read(cIGZIStream&)` (4), 5 `uint32 GetGZCLSID()` (0, EAX =
`0xE990C045`). Matches the SDK.

`cIGZMessageTarget2`, vtable `0x00AB127C`: 0 `QueryInterface` (thunk, 8), 1 `AddRef` (thunk, 0), 2 `Release`
(thunk, 0), 3 `bool DoMessage(cIGZMessage2*)` (4, AL; always true). Matches the SDK.

### 11.2 SDK comparison

The gzcom-dll header `cISC4FlammabilitySimulator.h` is **correct**: same 6 methods, same order, same argument
sizes; the only pointer return (`GetFlammabilityGrid`) is a plain pointer in EAX; there are no float or int64
returns. `GetAverageFlammability` returns its byte in AL (upper EAX bits are garbage); `IsCellFlammable` and
`GetFlammabilityHotspot` return `bool` in AL (upper bits garbage on some paths). No method returns a meaningless
value. Parameter names `param_1..3` of `GetFlammabilityHotspot` are `value, x, z`.

## 12. Dependencies (calls on other subsystems)

| Interface (original vtable) | Compiled slot | Method | Pops | Return |
|---|---|---|---|---|
| `cISC4City` (`0x00A87620`) | 93 / 94 `+0x174/+0x178` | `CellCountX()` / `CellCountZ()` | 0 | EAX |
| | 95 `+0x17C` | `PositionToCell(float x, float z, int& cx, int& cz)` | 16 | – |
| | 82 `+0x148` | `GetTutorialSystem()` | 0 | EAX |
| | 72 `+0x120` | `GetDisasterLayer()` | 0 | EAX |
| | 50 `+0xC8` | `GetSimulator()` (used by `Shutdown`) | 0 | EAX |
| | 33, 34, 36, 64, 70 | `GetLotManager`, `GetOccupantManager`, `GetZoneManager`, `GetPlumbingSimulator`, `GetWeatherSimulator`; also `GetOrdinanceSimulator` and `GetHistoryWarehouse` (slots per docs/abi). The original reads cached globals filled from these at city init; a replacement should call the getters | 0 | EAX |
| `cISC4Simulator` (`0x00AB2FB8`) | 19 `+0x4C` / 20 `+0x50` | `AddAgent(target, level, name, flags)` / `RemoveAgent(target)` | 16 / 4 | AL |
| `cIGZMessageServer2` | 4 / 5 / 6 `+0x10/+0x14/+0x18` | `MessagePost(msg, false)` / `AddNotification(target, type)` / `RemoveNotification(target, type)` | 8 | AL |
| `cISC4OrdinanceSimulator` (`0x00ABEAC0`) | 25 `+0x64` | `float GetCumulativeFactor(uint32 prop, uint32 index)` | 8 | **ST0** |
| `cISC4WeatherSimulator` (`0x00AB3610`) | 14 `+0x38` | `GetCurrentSeason()` | 0 | EAX |
| `cISC4PlumbingSimulator` (`0x00AB2228`) | 11 `+0x2C` | `bool IsCellWatered(int32 x, int32 z)` | 8 | AL |
| `cISC4ZoneManager` (`0x00A92E00`) | 12 `+0x30` | `GetZoneType(int32 x, int32 z)` | 8 | EAX |
| `cISC4HistoryWarehouse` (`0x00A8CCB4`) | 12 `+0x30` | `RecordHistoryData(uint32 type, int32 value)` | 8 | AL |
| `cISC4OccupantManager` (`0x00A8F7F0`) | 40 `+0xA0` | `GetOccupantsByStandardCityCells(SC4List<cISC4Occupant*>&, SC4Point<int> const* nw, SC4Point<int> const* se, cISC4OccupantFilter*, uint32)` (the filter overload; slot 41 is the type overload and forwards to 40) | 20 | AL |
| `cISC4LotManager` (`0x00AB37C0`) | 13 `+0x34` / 14 `+0x38` | `GetRandomLot()` / `GetOccupantLot(cISC4Occupant*)` | 0 / 4 | EAX |
| `cISC4Lot` (`0x00AB19B0`) | 35 `+0x8C` / 48 `+0xC0` | `GetCondition()` (byte, zero-extended) / `GetBuilding()` (not ref-counted) | 0 | EAX |
| `cISC4BuildingOccupant` | 3 `+0x0C` | `AsOccupant()` (vtable not located; slot from the SDK and use) | 0 | EAX |
| `cISC4Occupant` (`0x00AA41D8`) | 6 / 7 / 8 / 11 | `AsPropertyHolder()` / `GetType()` / `GetPosition(cS3DVector3*)` (4) / `GetBoundingCityCells(SC4Rect<long>&)` (4) | | EAX/AL |
| `cISCPropertyHolder` | 3 `+0x0C` | `HasProperty(uint32)` | 4 | AL |
| `cISC4TutorialSystem` (`0x00A861B8`) | 8 `+0x20` | `IsTutorial()` | 0 | AL |
| `cISC4DisasterLayer` (`0x00A9E390`) | 7 `+0x1C` | `GetDisasterManager(uint32 type)` | 4 | EAX |
| `cISC4DisasterManagerFire` (IID `0x49B43F57`, vtable `0x00A9E4DC`) | 5 `+0x14` / 9 `+0x24` | `IgniteCell(uint32 x, uint32 z, bool useProbability, bool teleport, uint32 fireIID)` / `SetCause(uint32)` | 20 / 4 | AL / EAX |
| `cIGZPersistResourceManager` / `cISCResExemplar` | 3 / 7 | `GetResource(key, 0xA52160F5, &p)` / `AsISCPropertyHolder()` | | |
| `cISC4SimGrid<uint8>` (simgrid §4) | 3, 4, 6, 7, 9, 10, 11, 12, 13, 22, 24, 27 | `Init`, `Shutdown`, `SetInstanceID`, `GetCellValue`, `SetTractSize`, `GetTractSize`, `GetTractShift`, `GetTractCountX/Z`, `GetTractValue`, `GetGridData`, `SetTractValues` | | |

Game-internal helpers the original links statically (response curves, property helpers, iterators) must be
reimplemented from foundations §4, §12 and simgrid §7.4.

## 13. Consumers and replaceability

### 13.1 Consumers (all through the cached `GetFlammabilitySimulator()` pointer and virtual slots)

| Consumer | Uses |
|---|---|
| Safety advisor (`update_occupant_variables`, `update_variables`) | `GetAverageFlammability` (Lua variable `ga_flammability`), `GetFlammabilityHotspot` (`l_flammability_h…`, location) |
| Query tool (`DescribeCell`) | `GetFlammabilityGrid()` → `GetCellValue(x, z)` |
| Fire manager `IgniteCell` (probability mode) | `GetFlammabilityGrid()` → `GetCellValue(x, z)` → its own ignition curve |
| Volcano lava (`SetThingsOnFire`, `BurnSurroundingStuff`) | `IsCellFlammable(x, z)` before igniting cells (cause 3) |
| Map view (flammability layer) | `GetFlammabilityGrid()` → `GetTractWidthX/Z`, then **`GetGridData()`** and reads the rows' memory directly (`DisplayStandardUint8Grid`) |
| Fire protection simulator | holds a reference from its `Init`, releases it in `Shutdown`; no call found *[inferred]* |
| City | creates, `Init`/`Shutdown`, save/load (slot 26) |

### 13.2 Replaceability findings

- Created only through GZCOM (city `CreateCityComponents`; class store on load). The COM director's registration
  calls the original's `GetGZCLSID` directly, which concerns only the original factory. No original code calls
  any other non-virtual flammability function or touches the object's memory. **Replaceable** through the class
  registry.
- The grid returned by `GetFlammabilityGrid` **must be a real `cSC4SimGrid<uint8>`-compatible object** whose
  `GetGridData()` yields the `SC4Vector<SC4Vector<uint8>>` layout of simgrid §4/§5, valid while the city is loaded,
  because the map view walks its memory. Using the game's own grid class (GZCOM `0x49B9E602`) satisfies this and
  also lets the class store save it by reference (§10).
- The adjusted grid must keep tract size 1 and the city's cell counts: the query tool, the fire manager and the
  volcano index it with city cell coordinates.
- A replacement must keep the posting of the fire (§6.8) through the fire manager's interface, and should keep
  `GetFlammabilityHotspot`'s "write nothing on false".

## 14. Original quirks (reproduced)

1. **First day after every `Init`** *[compat]*: the cell iterator is never reset in `Init`, so the first SimNewDay
   runs an empty `EndOfCycle`: average := 0, history records 0, the (loaded) hotspot list is cleared, a fire roll
   happens (only a random fire is possible). Saved hotspots and average are therefore visible only until then.
2. **Global multiplier 0 in a new city** *[compat]*: until the first SeasonChange (weather posts one at its first
   month-end, weather §5.3) or OrdinanceUpdated, every adjusted value written is 0. So a new city has no
   flammability, no hotspots and `IsCellFlammable` false everywhere for its first month.
3. **Add path** *[compat]*: insert and change take `max(raw, v)` without the building > flora > prop priority,
   and write `adjusted` from the new occupant's `v`, not from the tract's raw value (a lower `v` lowers the
   adjusted value until the next daily visit). A change that lowers an occupant's flammability is not reflected
   in `raw` until a removal or condition change recomputes the tract.
4. **Tract shift** *[quirk]*: the occupant update shifts cell bounds **left** by the tract shift (it should
   shift right); `ComputeTractRawFlammability` uses the tuning tract size, not the grid's. Both are identities
   with S = 1 (always the case). *[choice]* FreeCity4 supports only S = 1 and rejects other loaded tract sizes.
5. Out-of-city occupant rectangles collapse to tract (0, 0) (§6.3).
6. The daily refresh does not write `adjusted` where `raw` is 0, and has no lower clamp (a negative global
   multiplier would wrap; impossible with shipped data). *[choice]* FreeCity4 clamps at 0.
7. Probabilities are per cycle, not per month (§6.8); the first fire test uses the unrounded `r`, the hotspot
   test the `float32`-rounded `r`.
8. ChangeOccupant is treated as an insertion; ConditionChanged recomputes only the lot's building tracts.

## 15. Open questions

1. *(resolved)* Lot condition 4 is an abandoned lot at its last decay stage (foundations §5.2): the lot developer
   raises an abandoned lot's condition 1 → 2 → 3 → 4 over time, and occupied lots never reach 4. A freshly
   abandoned building therefore gets the abandonment multiplier only after the decay delay. A save with a
   long-abandoned building should still confirm the raw value.
2. *(resolved)* Occupant property `0x029244DB` marks preview occupants (foundations §5.2). Fire cause 5 is still
   unknown.
3. Whether the occupant manager still lists an occupant while RemoveOccupant is delivered (§6.3).
4. Who, if anyone, reads SimCycleComplete `0x08F415FE` and history type `0x2A11CB48` (graph data files?).
5. The x87 precision mode in effect during the simulation (double rounding in `roundHE`); irrelevant for
   shipped values.

## 16. Conformance tests (must exist in `engine/conformance`)

1. **Occupant raw value**: building with `0x29244DB5 = 150` on a normal lot → 150; same on a condition-4 lot →
   165 (code default 1.1) / 225 (shipped 1.5); 200 abandoned with 1.5 → 255 (clamp); 3 × 1.5 → **4** and
   5 × 1.5 → **8** (ties to even); a prop on an abandoned lot → no multiplier; a network occupant → 0; property
   absent → 0.
2. **Tract priority**: building 0 + flora 80 + prop 120 → 80; building 30 + flora 200 → 30; props 10 and 40 only
   → 40; empty tract → 0.
3. **Daily value**: raw 150, global 1.2, unwatered, no-water 1.1 → `roundHE(180 × 1.1)` = **198**; watered →
   180; raw 220, global 1.0, unwatered 1.25 → 275 → **255**; Smoke Detectors (0.95) raw 100 unwatered 1.25 →
   95 → 118.75 → 119; raw 0 → adjusted cell unchanged.
4. **Candidates**: threshold 196: v = 197 → candidate, v = 196 → none; shipped threshold 54: v = 55 → candidate.
5. **Cadence**: `tractsPerDay` 102 / 409 / 1638 for 64² / 128² / 256²; on a 64² map `EndOfCycle` runs on
   SimNewDays 1, 41, 81, 121, 161 after `Init`; each tract is visited once between two of them; one `rand()` per
   wrap.
6. **Average**: zoned tracts visited with values 100, 50, 0 (and any number of unzoned ones) → 50; no zoned
   tract → 0; `RecordHistoryData(0x2A11CB48, avg)` and SimCycleComplete posted once per `EndOfCycle`.
7. **Hotspots**: radius 8, min weight 2, candidates (10,10), (12,13), (30,30), (11,10) in that order →
   (10,10,1); merge → (11,11,2); new (30,30,1); merge → (11,10,3); prune → **[(11,10,3)]**.
8. **`GetFlammabilityHotspot`**: hotspots [(5,5,40), (20,20,45)], adjusted[20][20] = 0 → false, outputs
   untouched; adjusted[20][20] = 90 → true, 90, x = 20, z = 20; equal weights → the first in list order.
9. **Global multiplier**: Smoke Detectors on, summer, summer multiplier 1.2 → `float32(0.95f × 1.2f)` ≈ 1.14;
   shipped summer 1.0 → 0.95; a new city before any message → 0 and every adjusted write is 0.
10. **Fire roll** (stub generator): prob 5, r = 3, a lot with a building at world (100, 200) → `IgniteCell(6, 12,
    true, false, 0)` then `SetCause(5)`; r = 3 but `GetRandomLot` null and one hotspot (7, 9, 40) → hotspot fire
    at (7, 9) with `useProbability = false`; r = 6 with weight 40 (curve 7) → fire at the hotspot; r = 7 → no fire
    (7 > 7 is false); tutorial running → no fire and no generator draw; no curve → only random fires.
11. **`rand()` count**: an `EndOfCycle` with 4 hotspots that reaches the hotspot test makes 3 shuffle calls, then
    the wrap makes 1; with r < prob and a building found, only the wrap's call.
12. **Occupant update**: tract raw 100 from a building; insert flora 120 → raw 120, adjusted `roundHE(120 × g)`;
    insert flora 50 → raw stays 100, adjusted `roundHE(50 × g)` (§14.3); remove the building → recompute (flora
    120 if still present). An occupant with property `0x029244DB` is ignored on insert/change/remove.
13. **ConditionChanged**: a message whose IGZUnknown is a lot with a building recomputes the building's tracts
    (abandonment applied); a non-lot IGZUnknown does nothing.
14. **Persistence**: write → read → write is byte-identical; the payload after the references for
    (0.95, 50, [(11,10,3)]) is `33 33 73 3F 32 01 00 00 00 0B 00 00 00 0A 00 00 00 03 00 00 00`; versions 1 and 3
    are rejected; after `Init` on a loaded city the first SimNewDay clears the hotspots and the average (§14.1).
15. **Slots**: through the vtable with the game's argument sizes: slot 6 pops 8, slot 7 pops 12, slot 8 returns
    the byte in AL, slot 5 returns the adjusted grid pointer (not the raw one) without `AddRef`;
    `QueryInterface(0xE98B2F57)` fails; `Release` from count 1 calls `Shutdown`.
