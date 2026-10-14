# Spec: the commercial and industrial simulators (`cISC4CommercialSimulator`, `cISC4IndustrialSimulator`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program and its data files. Implementers must work from
this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (streams §2, references §2.7, the city's component list §3.6, load
order §3.7, messages §5, agent order §13), [simulator.md](simulator.md) (levels and agent flags),
[simgrid.md](simgrid.md) (the `uint8` grid class and its record), [building-development.md](building-development.md)
(the building map, slots 41 and 42), [residential.md](residential.md) §5.3 (the proximity-map algorithm, shared),
[growth-developer.md](growth-developer.md) §4.3 (the only simulation reader).

The two simulators are small. The commercial simulator keeps two proximity grids for office buildings and
recomputes them lazily. The industrial simulator does nothing at all: it exists so the city has an object to save
and to hand out. They are specified together because their shells are identical; §5 tables the differences.

## 1. Role

**Commercial.** The commercial simulator owns two `uint8` grids, the **CO$$ and CO$$$ proximity maps**. They are
meant to hold, for every city tile, the distance to the nearest office building of that wealth. They are rebuilt
from the building development simulator's building map whenever an office building is inserted or removed. The
growth developers read them as a desirability input (growth-developer.md §4.3, state 6), and the query tool's
desirability readout shows them.

*[compat]* Because the shared proximity algorithm seeds only the z = 0 edge (residential.md §5.3, QUESTIONS RG23),
the grids hold 1 (or 0) on that edge and 0 everywhere else. After the growth developer's 4 × 4 floor mean every
growth tract reads 0, and the shipped curves give 0 at distance 0 (§7). The commercial simulator has **no effect**
on the shipped game's growth *[inferred from the curves and the corpus, §8.3]*.

**Industrial.** The industrial simulator registers no agent, subscribes to nothing, and keeps no state beyond its
`initialized` flag. Its record is a version number. Industrial demand, pollution and freight are handled by the
demand, pollution and traffic simulators.

| Class | CLSID | Interfaces (IID) | City component (foundations §3.6) | City getter |
|---|---|---|---|---|
| commercial simulator | `0x8990C080` | `cISC4CommercialSimulator` (`0x077ABEB2`) and `cIGZUnknown` (`1`) at +0, `cIGZMessageTarget2` (`0x452294AA`) at +4, `cIGZSerializable` (`0xE4FDA3D4`) at +8 | 22 | `cISC4City` slot 54 (`+0xD8`) |
| industrial simulator | `0xA990C022` | `cISC4IndustrialSimulator` (`0x477A9E7D`) and `1` at +0, `0x452294AA` at +4, `0xE4FDA3D4` at +8 | 28 | `cISC4City` slot 60 (`+0xF0`) |

`QueryInterface` for any other IID returns false and leaves the output untouched. A successful query `AddRef`s.

The city creates both through GZCOM in `CreateCityComponents` and stores them in its record's reference list.
`SL::OnCityInit` caches the commercial simulator in the global `0x00B43D6C`, which the growth developers and the
query tool read.

| | Commercial | Industrial |
|---|---|---|
| Factory | `0x0069EF70`: `GlobalHeapAlloc(0x3C)` + constructor `0x0069EE50` | `0x006B8AD0`: `GlobalHeapAlloc(0x14)`, fields set inline |
| Primary vtable | `0x00AB0CCC` (6 interface slots + deleting destructor) | `0x00AB1530` (5 interface slots + deleting destructor) |
| Message target vtable | `0x00AB0CBC`, `DoMessage` `0x0069EEB0` | `0x00AB1520`, `DoMessage` `0x006B8AA0` |
| Serializable vtable | `0x00AB0CA4`: Write `0x0069EC50`, Read `0x0069ECB0`, GetGZCLSID `0x0069EA40` | `0x00AB1508`: Write `0x0042E440` (shared by identical-code folding), Read `0x006B8A70`, GetGZCLSID `0x006B8960` |

## 2. State

### 2.1 Commercial

Offsets are from the primary interface pointer. They are given for orientation; a replacement need not copy them.

| Offset | Field | Constructor | Saved |
|---|---|---|---|
| `+0x0C` | `initialized` (u8) | 0 | no |
| `+0x10` | reference count | 0 | no |
| `+0x14` | building development simulator, held | null | no |
| `+0x18` | city, held | null | no |
| `+0x1C` | message server, held | null | no |
| `+0x20` | `dirty` (u8) | **1** | yes, folded into the flag byte (§8) |
| `+0x24` | `state` (s32, 0..3) | **3** | no (only through the flag byte) |
| `+0x28`, `+0x2C` | demand nodes `0x3000` and `0x3800`, held | null | no |
| `+0x30` | grid tract size (u8) | **1** | no |
| `+0x34` | CO$$ proximity grid (`uint8`, instance `0x89D5B788`), held | null | yes (reference) |
| `+0x38` | CO$$$ proximity grid (`uint8`, instance `0x89D5B6EF`), held | null | yes (reference) |

The two demand nodes (`GetDemand(0x3000, 0)` "commercial" sum and `GetDemand(0x3800, 0)` "commercial
connections", demand.md §4) are fetched, `AddRef`ed and released at `Shutdown`, and never read *[quirk]*. Their
only effect is that `Init` fails when either is missing (§4.1).

### 2.2 Industrial

| Offset | Field | Constructor | Saved |
|---|---|---|---|
| `+0x0C` | `initialized` (u8) | 0 | no |
| `+0x10` | reference count | 0 | no |

`Init` also sets the static byte `0x00B490A4` to 1. Nothing reads it.

### 2.3 Reference counting (both)

`AddRef` increments. `Release` decrements when the count is above 1; when it is 1 or less it calls `Shutdown`,
then the deleting destructor, and returns 0 (the count is not decremented first) *[compat]*. The industrial
`Release` (`0x006D5140`) is the same code as the park manager's (identical-code folding).

## 3. Agents, messages and cadence (commercial)

`Init` registers **four** agents on the simulator (simulator.md §4, `AddAgent(target, level, name, flags)` at
`+0x4C`), all on the message-target interface (+4), all named `"cSC4CommercialSimulator"` (string `0x00A89920`)
and all with flags **2** (enabled only once the city is established, simulator.md §4), in this order:

| Order | Level | Message | Handling |
|---|---|---|---|
| 1 | 0 (one-shot) | `0x66956810` | ignored |
| 2 | 10 (idle) | `0x6695681A` | one `DoIdle` step (§4.3) |
| 3 | 4 (day) | `0x66956814` | one `DoIdle` step |
| 4 | 6 (month) | `0x66956816` | ignored |

The month and one-shot registrations still count for foundations §13 (agent order): the commercial simulator is
one of the month-level agents even though it does nothing there (QUESTIONS AO1: its AddAgent name is
`cSC4CommercialSimulator`).

It subscribes on the message server to:

| Message | Handling |
|---|---|
| `0x99EF1142` occupant inserted, `0x99EF1143` occupant removed | if data3 (the occupant type) is `0x278128A0` (building) and void1 answers `QueryInterface(0x87DDA3A9)` (`cISC4BuildingOccupant`), read the building profile (building slot 11 `GetBuildingProfile()`, first u32 = purpose). If the purpose is **3** (commercial office): `dirty := true`. Release the building. |

*[quirk]* There is **no** preview-occupant test (foundations §5.2): the temporary building under the lot-placement
cursor also marks the maps dirty. This costs one recomputation and changes nothing.

`DoMessage` returns true for every message.

The industrial simulator registers no agents and subscribes to nothing. Its `DoMessage` reads the type and
returns true.

## 4. Behaviour (commercial)

### 4.1 `Init()`

If `initialized`, return true. Otherwise:

1. `initialized := true`; set the write-only static `0x00B48614` to 1.
2. Register the four agents (§3).
3. If no message server is held: hold the cached message server and subscribe to `0x99EF1142`, then
   `0x99EF1143`.
4. If no city is held, hold the cached city. If no building development simulator is held, hold the cached one.
5. `demand := city.GetDemandSimulator()` (city slot 56, `+0xE0`). If it is null, or
   `GetDemand(0x3000, 0)` is null, or `GetDemand(0x3800, 0)` is null: **return false** without creating the grids.
   *[compat]* Each node found is `AddRef`ed and held.
6. If the CO$$$ grid is absent, create it: a new `uint8` grid (CLSID `0x49B9E602`, `0x44` bytes from the global
   heap), `AddRef`,
   `Init`, `SetTractSize(1)`, `SetInstanceID(0x89D5B6EF)` (simgrid.md §6.2). Then the same for the CO$$ grid
   with instance `0x89D5B788`. A grid loaded from the save is kept (foundations §3.7: `Read` runs before `Init`).
7. Return true.

`SetTractSize` sizes the grid from the city (simgrid.md §4.2): 64, 128 or 256 tracts square for the three city
sizes, all cells 0.

### 4.2 `Shutdown()`

If `initialized`: clear it; `Shutdown` and release both grids; release the city and the building development
simulator; unsubscribe `0x99EF1142` and `0x99EF1143` and release the message server; release both demand nodes;
remove the agent through the city's simulator (`RemoveAgent(+4)`). Return true.

### 4.3 `DoIdle` (`0x0069ED30`)

One call performs one step of a four-state machine. A `cRZTimer` is started at entry but never read; the step is
not time-sliced.

| State | Step |
|---|---|
| 3 (idle) | if `dirty`: `dirty := false`, `state := 0`. Otherwise nothing. |
| 0 | `state := 1` and continue at once with state 1 (same call) |
| 1 | rebuild CO$$ (§4.4, wealth 2), `state := 2` |
| 2 | rebuild CO$$$ (§4.4, wealth 3), `state := 3` |

So a change is noticed on one call and both maps are rebuilt on the next two calls. The day agent calls it once
per simulated day; the idle agent calls it on every idle tick while the simulator is running.

### 4.4 Rebuilding one map *[compat]*

For wealth w ∈ {2, 3}:

1. `key := BDS.GetBuildingMapKey(purpose 3, w)` (slot 42). With `(purpose << 2) + w + 1` this is **15** for CO$$
   and **16** for CO$$$. Only the low byte matters (building-development.md §7.1).
2. `map := BDS.GetBuildingMap()` (slot 41).
3. Run the shared **seed-and-converge** transform of residential.md §5.3 (helper `0x005E5130`: seed `0x005E4FA0`,
   then distance passes until no change, `0x005E4D60`) **directly on the proximity grid**, with the building map
   as the source and `key` as the target value. There is no temporary grid and no copy step, and the whole
   transform runs in one call.

Consequences *[compat]*, with all tract sizes 1 (the shipped case):

- the seed writes only the cells (x, 0): 0 where `map[x][0] = key`, otherwise 255;
- every other cell keeps the value it had. A new grid is all 0, and a pass can only lower a value, so these
  cells stay 0;
- after the passes, (x, 0) is 0 where an office building of that wealth occupies (x, 0), and 1 elsewhere on the
  edge (its neighbour (x, 1) is 0).

If a loaded grid holds other values off the edge (for example a grid written by a corrected FreeCity4 build), they
are only ever lowered by later rebuilds, never raised *[compat]*.

### 4.5 Worked example

A 64 × 64 city. A CO$$ tower is placed whose footprint covers tiles (10..12, 30..32). Nothing else is on the
z = 0 edge.

1. The occupant manager sends `0x99EF1142` with data3 = `0x278128A0` and void1 = the building. Its purpose is 3,
   so `dirty := true`.
2. Next `DoIdle` (state 3): `dirty := false`, `state := 0`.
3. Next `DoIdle`: state 0 → 1, rebuild CO$$ with key 15. No (x, 0) tile holds 15, so every (x, 0) is seeded 255
   and converges to 1; all other cells stay 0. `state := 2`.
4. Next `DoIdle`: rebuild CO$$$ with key 16, the same result. `state := 3`.
5. `GetProximityMap(0x3320)` now returns a grid with 64 cells of value 1 on the z = 0 edge and 4,032 cells of 0.

Had the tower stood at (5, 0), cell (5, 0) of the CO$$ grid would be 0 and every other edge cell 1.

### 4.6 `GetProximityMap(developerType)` (slot 5)

`0x3320` → the CO$$ grid; `0x3330` → the CO$$$ grid; anything else → null. No `AddRef`. The pointer is the
grid's `cISC4SimGrid<uint8>` interface; callers read its memory directly (simgrid.md §6.3).

## 5. Commercial and industrial compared

| | Commercial | Industrial |
|---|---|---|
| Interface slots beyond the base five | 1 (`GetProximityMap`) | 0 |
| Object size | `0x3C` | `0x14` |
| Agents | levels 0, 10, 4, 6; flags 2 | none |
| Subscriptions | `0x99EF1142`, `0x99EF1143` | none |
| Owned grids | two `uint8` proximity grids | none |
| Holds | city, BDS, message server, two demand nodes | nothing |
| `Init` can fail | yes (missing demand nodes) | no |
| Record | 19 bytes, version 1 | 2 bytes, version 1 |
| Random numbers | none | none |
| Tuning | none | none |

## 6. Random numbers

Neither simulator draws random numbers.

## 7. Tuning

Neither simulator reads an exemplar. The values that matter are constants: the building-map keys (15, 16), the
grid instances (`0x89D5B788`, `0x89D5B6EF`), the tract size (1) and the demand IDs (`0x3000`, `0x3800`).

The proximity maps are consumed through the growth developers' curves `0x87CD6391` (CO$$) and `0x87CD6392` (CO$$$)
(growth-developer.md §4.3). In the shipped exemplars (group `0x67CD5FA1`):

| Growth developer | `0x87CD6391` (CO$$ proximity) | `0x87CD6392` (CO$$$ proximity) |
|---|---|---|
| CS$$ `0x3120`, CO$$ `0x3320` | 18 points, 0 at distance 0 falling to −10 at 255 | flat 0 |
| CS$$$ `0x3130` | flat 0 | 18 points, 0 at 0 falling to −20 at 255 |
| CO$$$ `0x3330` | flat 0 | 18 points, 0 at 0 falling to −10 at 255 |
| the other eight | flat 0 (`(0, 0), (255, 0)`) | flat 0 |

With the maps of §4.4 every growth tract reads 0 (a 4 × 4 floor mean of at most four 1s), and every curve gives 0
at 0.

## 8. Persistence (`cIGZSerializable`) *[compat]*

### 8.1 Commercial (`GetGZCLSID() = 0x8990C080`)

```
u16  version = 1
u8   flag = (dirty || state ≠ 3) ? 1 : 0
ref  CO$$ grid    (foundations §2.7: u32 objID, then u32 classID 0x49B9E602 when objID ≠ 0)
ref  CO$$$ grid
```

19 bytes when both grids exist. `Write` returns true unless the stream reports an error.

`Read`: read the version; if it is not exactly 1, return false. Read the flag and set `dirty := (flag ≠ 0)`. Read
both references with `ReadGZSerializable(IID 0xC9BD939A)` into the CO$$ and CO$$$ fields. `state` is not read; it
keeps the constructor's 3. Return true unless the stream reports an error. So a city saved in the middle of a
rebuild restarts the rebuild from the beginning on the first `DoIdle` after loading.

*[quirk]* `Read` overwrites the grid fields without releasing what they held. On a freshly constructed object they
are null, so this matters only for a second `Read` on the same object. *[choice]* FreeCity4 releases old grids
before replacing them.

### 8.2 Industrial (`GetGZCLSID() = 0xA990C022`)

```
u16  version = 1
```

`Read` fails unless the version is exactly 1.

### 8.3 Corpus validation

421 distinct saves (all of `Documents\SimCity 4\Regions`), every record parsed exactly:

| Records | Count |
|---|---|
| commercial: 19 bytes, flag 1, both grids all 0 (64, 128 or 256 square) | 410 (157 / 243 / 10 by size) |
| commercial: 19 bytes, flag 0, both grids 128 × 128 with (x, 0) = 1 for every x and all other cells 0 | 11 (the played Berlin cities, e.g. `City - Konradshohe.sc4`) |
| industrial: 2 bytes `01 00` | 421 |

The 11 played cities confirm §4.4: their CO grids are exactly the edge pattern, with no office building on the
z = 0 edge. The 410 others never ran a rebuild (`dirty` is still the constructor's 1).

## 9. Interface slot tables *[compat]*

`cISC4CommercialSimulator` (vtable `0x00AB0CCC`):

| Slot | Offset | Method | Original | Args (dwords) | Callee pops | Return |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface(iid, void**)` | `0x0069EA50` | 2 | 8 | AL |
| 1 | `+0x04` | `AddRef()` | `0x004F7BA0` (shared) | 0 | 0 | EAX (count) |
| 2 | `+0x08` | `Release()` | `0x0069EB50` | 0 | 0 | EAX (count) |
| 3 | `+0x0C` | `Init()` | `0x0069EF90` | 0 | 0 | AL |
| 4 | `+0x10` | `Shutdown()` | `0x0069EB80` | 0 | 0 | AL (1) |
| 5 | `+0x14` | `GetProximityMap(uint32 developerType)` → `cISC4SimGrid<uint8>*` | `0x0069EAD0` | 1 | 4 | EAX |
| 6 | `+0x18` | deleting destructor (not an interface method) | `0x0069EB20` | 1 | 4 | EAX |

`cISC4IndustrialSimulator` (vtable `0x00AB1530`):

| Slot | Offset | Method | Original | Args | Callee pops | Return |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface` | `0x006B8970` | 2 | 8 | AL |
| 1 | `+0x04` | `AddRef` | `0x004F7BA0` (shared) | 0 | 0 | EAX |
| 2 | `+0x08` | `Release` | `0x006D5140` (shared with the park manager) | 0 | 0 | EAX |
| 3 | `+0x0C` | `Init` | `0x006B89F0` | 0 | 0 | AL (1) |
| 4 | `+0x10` | `Shutdown` | `0x006B8A10` | 0 | 0 | AL (1) |
| 5 | `+0x14` | deleting destructor | `0x006B8A40` | 1 | 4 | EAX |

Message target (both): slot 3 `DoMessage(cIGZMessage2*)`, 1 arg, pops 4, AL = 1. Serializable (both): slot 3
`Write(cIGZOStream&)` and slot 4 `Read(cIGZIStream&)`, 1 arg, pop 4, AL; slot 5 `GetGZCLSID()`, EAX.

## 10. Who reaches in

| Caller | Uses |
|---|---|
| Growth developers (`EstimateDesirability` `0x006B4DF0`, `DoIdle` `0x006B5F20`) | commercial slot 5 `GetProximityMap(0x3320 / 0x3330)` through the global `0x00B43D6C` (growth-developer.md §4.3, state 6) |
| Query tool (`cSC4ViewInputControlQuery::WriteDesirabilityEquation` `0x004C9520`) | the same, for the debug desirability readout |
| City | creation, `Init`, `Shutdown`, save and load; getter slots 54 and 60 |
| `SL::OnCityInit` / `OnCityShutdown` | fill and clear the global `0x00B43D6C` |

No code calls the industrial simulator beyond the city's lifecycle calls *[inferred: no other reference to its
interface or class ID was found]*.

## 11. Original quirks (reproduced)

- The proximity seed covers only the z = 0 edge (residential RG23); the maps are effectively constant (§4.4).
- No preview-occupant test on the occupant messages (§3).
- The demand nodes are held but never read, and their absence makes `Init` fail (§2.1, §4.1).
- One-shot and month agents that do nothing (§3).
- `Read` does not restore `state`; a saved mid-rebuild restarts (§8.1).

## 12. Open questions

See QUESTIONS.md, section "From the commercial, industrial, civic, park, history and clock specs" (CM1–CM3, IN1).

## 13. Conformance tests (must exist in `engine/conformance`)

1. **Identity**: both classes answer `QueryInterface` for their IIDs, `1`, `0x452294AA` and `0xE4FDA3D4`, and
   refuse others; `GetGZCLSID` returns `0x8990C080` / `0xA990C022`.
2. **Commercial Init**: a city whose demand simulator lacks node `0x3000` makes `Init` return false and creates no
   grid; with both nodes it creates two `uint8` grids of tract size 1 with instances `0x89D5B788` and
   `0x89D5B6EF`, registers agents at levels 0, 10, 4, 6 (flags 2, in that order) and subscribes `0x99EF1142`,
   `0x99EF1143`.
3. **Dirty marking**: an insert message with data3 = `0x278128A0` and a building of purpose 3 sets `dirty`;
   purpose 1 does not; data3 = `0xC772BF98` (network) does not; a preview occupant of purpose 3 does.
4. **State machine**: from (dirty, state 3) the next three `DoIdle` calls give states 0, 2, 3, and the maps
   change only on the second and third calls; with `dirty` false, `DoIdle` in state 3 changes nothing.
5. **Map content** (§4.5): 8 × 8 building map with key 15 at (3, 0) and at (5, 5) → CO$$ grid (3, 0) = 0, (x, 0) = 1
   for x ≠ 3, all other cells 0; CO$$$ grid edge all 1.
6. **Loaded grids**: a grid preloaded with 9 at (4, 4) and 0 elsewhere off the edge holds 1 at (4, 4) after a
   rebuild (its neighbours are 0); a preloaded 0 at (2, 0) becomes 1 when map (2, 0) is not the key (the seed
   writes 255 there first).
7. **GetProximityMap**: `0x3320` and `0x3330` return the two grids; `0x3110`, `0x3310` and 0 return null; no
   reference is added.
8. **Save**: write → read → write is byte-identical; flag = 1 when `dirty` or `state ≠ 3`; version 2 is rejected;
   after reading flag 1, the first `DoIdle` starts a rebuild. Industrial: writes `01 00`, rejects version 2.
9. **Corpus**: every commercial and industrial record of the 421-save corpus reads and re-writes byte-identically.
