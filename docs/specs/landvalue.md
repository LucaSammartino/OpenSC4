# Spec: the land value subsystem (`cISC4LandValueSimulator`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program (Windows build 1.1.641) and its data files.
Implementers must work from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must
**not** consult decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [simgrid.md](simgrid.md) (grid objects §2–§6, distance effect §7.2, two-grid helper §7.3,
scanning iterator §7.4), [foundations.md](foundations.md) (streams §2, references §2.7, class store §3, city
component table §3.6, response curves §4, messages §5, slot order §16, framework slot tables §17),
[simulator.md](simulator.md) (agents, calendar messages, CPU-cycle budgets §5.5).

## 1. Role

The land value simulator keeps one byte of **land value** (0..255) per *tract* (a square of S × S cells, S = 2 in
the shipped game). The value is:

```
intrinsic(t) = clamp255( base(t) + waterBonus(t) + altitudeEffect(t) )              (§6, states 1–4)
total(t)     = intrinsic(t), then for each listed growth developer, in turn:
               total(t) = clamp[1,255]( total(t) + weight_dev × curve(desirability_dev(t)) )   (state 7)
```

It also publishes a per-8×8-cell **slope** grid for other subsystems, the city-wide **average land value**, and
the **mode altitude**. Pollution, crime, parks and other services reach land value only *indirectly*, through the
growth developers' desirability maps *[inferred from data flow]*.

The value is recomputed by a resumable 9-step pipeline, **at most once per month and only when something changed**
(occupants, terrain, water or zoning), plus once after every `Init` (new city or loaded city).

| Class | CLSID | Interfaces (IID) |
|---|---|---|
| land value simulator | `0xE990C18E` | `cISC4LandValueSimulator` (`0xE7781D8C`), `cIGZMessageTarget2` (`0x452294AA`), `cIGZSerializable` (`0xE4FDA3D4`), `cIGZUnknown` (`1`) |

`QueryInterface` answers exactly these four IIDs (the message-target and serializable answers are separate
sub-objects of the same object; all share one reference count) and `AddRef`s on success. `GetGZCLSID()` returns
`0xE990C18E`. The city creates it through GZCOM and stores it in reference slot 29 of its save record (foundations
§3.6); on load the class store creates it from its class record. Other code reaches it through the city
(`cISC4City::GetLandValueSimulator()`) or a process-wide pointer the game sets at city init (§12).

## 2. Grids and coordinates

| Grid | Element | Tract size | Instance ID | Saved | Getter |
|---|---|---|---|---|---|
| intrinsic | `uint8` | tuning `0x47E2C200` (S) | `0x89D5BB39` | yes (ref) | `GetIntrinsicLandValueMap` |
| total | `uint8` | S | `0x89D5BB51` | yes (ref) | `GetTotalLandValueMap` |
| scratch (working copy) | `uint8` | S | `0x89D5BB60` | **no** | — |
| slope | `float` | 8 (fixed) | `0x49D5BB6F` | yes (ref) | `GetSlopeMap` |

All four are `cSC4SimGrid<T>` objects (simgrid.md §2): class IDs `0x49B9E602` (uint8) and `0x49B9E60A` (float).
Grid geometry (tract shift, tract counts, widths) follows simgrid.md §3. With the shipped S = 2 a 256 × 256-cell
city has 128 × 128 land value tracts and 32 × 32 slope tracts.

Tract `t = (tx, tz)` covers cells `[tx·S, tx·S + S − 1] × [tz·S, tz·S + S − 1]`. Grids are indexed `[x][z]`.

Two per-tract predicates are used throughout:

- **`HasLand(t)`** *[compat]*: true if **any terrain vertex** `(vx, vz)` with `tx·S ≤ vx ≤ tx·S + S` and
  `tz·S ≤ vz ≤ tz·S + S` (inclusive, so (S+1)² vertices) satisfies terrain `IsLandVertex(vx, vz)`. Vertices
  outside the terrain are not land. (`IsLandVertex` is true when the vertex is not under water *[inferred]*.)
- **`IsZoned(t)`** *[compat]*: true if any **cell** `(x, z)` of the tract's S × S cells (exclusive upper bound) has
  zone manager `GetZoneType(x, z) ≠ 0`. The comparison uses the full 32-bit result.

Both scan in x-outer, z-inner order and stop at the first hit. Note the different extents (vertices inclusive,
cells exclusive) *[quirk]*.

**Tract altitude** `alt(t)` = terrain `GetAltitude(px, pz)` where `(px, pz)` = the intrinsic grid's
`TractCenterToPosition(tx, tz)` (simgrid.md §4), i.e. `((tx + 0.5)·W, (tz + 0.5)·W)` metres with W = S × cell width.
With S = 2 and 16 m cells the sample point is a terrain vertex.

## 3. State

| Field | Initial (constructor) | Persisted | Meaning |
|---|---|---|---|
| initialized | false | no | `Init` done |
| refCount | 0 | no | |
| city, terrain, zone manager, message server | null | no | services taken in `Init` (`AddRef`ed) |
| intrinsic, total, slope grids | null | **ref** (§10) | §2 |
| scratch grid | null | no | §2 |
| dirty | false | yes (combined, §10) | something changed since the last pass |
| pipelineState | **not initialised by the constructor**; `Init` sets 0 | no | 0..8 running, 9 finished |
| tract iterator, slope iterator | created by `Init` | no | scanning iterators (simgrid.md §7.4) |
| sum (int32), count (int32) | — | no | accumulators of states 3 and 8 |
| ignoreZoning (bool) | — | no | set when no zoned land tract exists (state 3) |
| histogram | empty; `Init` sizes it | no | altitude bins of 2^5 = 32 m |
| histShift | `Init` sets 5 | no | |
| minAlt, maxAlt (f32) | — | no | observed land altitude range (state 3) |
| meanAlt (f32) | 0 | **yes** | mean altitude of counted tracts |
| modeAlt (f32) | 0 | no | returned by `GetModeAltitude` |
| pivot, lowRange, highRange (f32), topIndex (int32) | — | no | altitude-effect parameters (§7.3) |
| averageLandValue (u8) | 0 | **yes** | returned by `GetAverageLandValue` |
| tuning values | code defaults (§8) | no | re-read by every `Init` |
| altitude curve, desirability curve | absent | no | response curves (foundations §4) |
| developer list | empty | no | ordered map *growth developer pointer → weight* (§6, state 7) |
| developer cursor | — | no | position in the developer list (state 7) |

## 4. Lifecycle

### 4.1 `Init()` → true

If already initialized, return true and do nothing else. Otherwise, in this order (there is no "no city" path:
the original takes the city, zone manager and terrain from the process-wide pointers that the city sets when it is
created, and `AddRef`s them without a null check, so `Init` without a city would crash. It only ever runs from the
city's own `Init` (foundations §3.7). *[choice]* FreeCity4 returns true and creates no grids when there is no
city; every other method must then tolerate absent grids):

1. Take the city, the zone manager and the terrain, each only if not already held (`AddRef`).
2. Read the tuning exemplar (§8). An absent tract-size property leaves S unchanged; the other byte-array
   properties are required by the original (§8).
3. For each of the intrinsic, total, scratch grids that is still null (i.e. not restored by `Read`): create a
   `uint8` grid, `AddRef`, `Init`, `SetTractSize(S)`, `SetInstanceID(id)` (§2). Same for the slope grid with type
   `float`, tract size 8. Grids restored by `Read` are **not** resized or re-initialised *[quirk]* (§13).
4. Set **pipelineState := 0** (the pipeline always reruns after `Init`, new or loaded city) *[compat]*.
5. Create the two scanning iterators if missing and set their rectangles: tract iterator over
   `[0, countX−1] × [0, countZ−1]` of the intrinsic grid, slope iterator over the slope grid's tract range.
6. histShift := 5; resize the histogram to `trunc(terrain.MaxAllowedHeight()) >> 5` bins, new bins 0 (78 bins
   with the shipped terrain tuning, max allowed height 2500 m).
7. History warehouse: `SetHistoryDataQueueLength(0x6A663AC8, 1)`.
8. If the simulator exists: `AddAgent(self, L, "cSC4LandValueSimulator", flags 2)` for L = 0, 10, 4, 6 in that
   order (flags 2 = enabled only once the city is established, simulator.md §4).
9. If no message server is held yet: take it and subscribe (`AddNotification`) to the five change messages of §5.
10. initialized := true; return true.

### 4.2 `Shutdown()` → true

If not initialized, return true. Otherwise: initialized := false; `RemoveAgent(self)` (all levels) on the city's
simulator; unsubscribe the five messages and release the message server; release terrain, zone manager and city;
for each of the four grids: `Shutdown()`, `Release()`, set null; delete both iterators and both curves. The
developer list, dirty flag, pipeline state and statistics are **not** cleared *[quirk]* (a later `Init` re-reads
tuning into the same developer list and resets the state to 0).

### 4.3 Release

`Release()` decrements the count while it is above 1 and returns the new count. At 1 (or below) it calls
`Shutdown()`, destroys the object and returns 0.

## 5. Schedule and messages *[compat]*

| Message | Delivered as | Action | Returns |
|---|---|---|---|
| SimBegin `0x66956810` | agent, level 0 | nothing | true |
| SimNewDay `0x66956814` | agent, level 4 | `Step(budget = 5 ms)` | true |
| SimNewMonth `0x66956816` | agent, level 6 | if dirty **and** pipelineState = 9: dirty := false, pipelineState := 0 | true |
| SimIdle `0x6695681A` | agent, level 10 | `Step(budget = data3 ms)` | true (ignored by the simulator) |
| InsertOccupant `0x99EF1142`, RemoveOccupant `0x99EF1143`, WaterHeightsChanged `0x678E6E3A`, TerrainHeightsChanged `0x278E6003`, ZoningChange `0x07699455` | subscription | dirty := true | true |
| anything else | — | nothing | true |

`DoMessage` always returns true, so land value never holds a calendar level open. Consequences:

- A recomputation starts only at the first SimNewMonth after a change (or at `Init`), and only if the previous pass
  has finished; a change during a pass keeps `dirty` set, so another pass follows at the next month.
- On the 1st of a month the day level runs before the month level (simulator.md §5.3), so a restarted pass makes
  its first step on a later SimIdle or SimNewDay.
- SimIdle is delivered while the game is paused (simulator.md §5.4), so a pass also progresses during a pause
  *[compat]*.

Order relative to other simulators: land value reads only the growth developers' desirability maps (state 7) and
terrain/zoning. It does not depend on any other simulator having run earlier in the same day or month.

## 6. The pipeline (`Step(budget)`) *[compat]*

`Step` performs work for the current `pipelineState` and returns. **At most one state completes per call**: states
0, 2 and 6 always complete in one call; state 7 handles one developer per call; states 1, 3, 4, 5 and 8 loop over
tracts with the scanning iterator (z outer, x inner, simgrid.md §7.4) until either the iteration ends (the state
completes) or the time budget runs out (the state stays and the iterator keeps its position for the next call).
State 9 (and any value above 8) does nothing.

The original's budget is CPU-cycle based (simulator.md §5.5); it checks the clock every *n* iterations with an
adaptive stride *n* (1..1024, initially 1024, shared per state across calls). *[choice]* FreeCity4 may use any
deterministic slicing (for example a fixed number of tracts per call, or a whole state per call), as long as each
call completes at most one state and the results equal those of an uninterrupted pass. Because the grids are
modified in place, consumers can observe a partially updated intrinsic or slope grid while states 4 and 5 run;
the total grid only changes at the end of state 7.

| State | Work | Next |
|---|---|---|
| 0 | Reset the tract iterator to the first tract. | 1 |
| 1 | For every tract: scratch(t) := `HasLand(t)` ? **base** : 0, base = tuning `0x47E2C300[0]`. | 2 |
| 2 | Water bonus (§7.1) on scratch. Then minAlt := terrain `MaxAllowedHeight()`, maxAlt := terrain `MinAllowedHeight()` (deliberately inverted start values); reset the tract iterator; sum := 0, count := 0, ignoreZoning := false; all histogram bins := 0. | 3 |
| 3 | Altitude statistics (§7.2). | 5 (or 3 again, see §7.2) |
| 5 | For every slope tract `(sx, sz)`: slope(sx, sz) := lot developer `GetAreaSlope(rect, true)` with the inclusive cell rectangle `{x1 = sx·8, z1 = sz·8, x2 = sx·8 + 7, z2 = sz·8 + 7}` (8 = the slope grid's tract size, shift 3). Then reset the tract iterator. | 4 |
| 4 | If the altitude curve exists: for every tract, intrinsic(t) := min(255, scratch(t) + `AltitudeEffect(alt(t))`) (§7.3). If the curve is **absent**, the intrinsic grid is **not written at all** and keeps its previous contents *[quirk]*. Then reset the tract iterator. | 6 |
| 6 | scratch := copy of intrinsic (whole grid, including its dimensions). Developer cursor := first entry of the developer list. | 7 |
| 7 | If the cursor is not at the end: apply that developer (§7.4), advance the cursor, return (stay in 7). At the end: total := copy of scratch; reset the tract iterator; sum := 0, count := 0. | 8 |
| 8 | Average (§7.5), history record, post LandValueMapChanged (§9). | 9 |

Note the order 3 → 5 → 4 *[compat]* (slope before the altitude effect). A full pass needs at least
9 + D calls, D = number of developers in the list (10 + D when the zoning fallback of state 3 triggers).

**`GetAreaSlope` is read-only** *[compat]* (QUESTIONS LV3; lot developer slot 29, `0x006C5DC0`). It reads, for every
terrain vertex `(vx, vz)` with `x1 ≤ vx ≤ x2 + 1` and `z1 ≤ vz ≤ z2 + 1`, the lot developer's own slot 15
(`GetTerrainAltitudeByVertex`, `0x006C4310`): terrain slot 12 `GetAltitude(index)` and, because state 5 passes
`true`, terrain slot 31 `GetWaterLevel(index)` (`0x007413C0`, `0x00744980`, both plain array reads). It keeps the
lowest and highest value and their vertex positions in locals, computes the 2-D distance between those two vertices
and returns the slope in ST0. It writes nothing outside its own stack frame: no member of the lot developer, the
terrain or any grid. Calling it from a shadow, or more often than state 5 does, has no side effect.

### 6.1 When each input is read *[compat]* (QUESTIONS LV1, from the exe)

A pass has **no single input instant**. Each state reads the world at the moment it handles a tract (or developer),
inside its own `Step` call, and never reads it again for that pass. The calls of one pass are spread over frames
and days (SimIdle every frame, paused or not, plus one 5 ms call per SimNewDay, §5), so two states, and two tracts
of one time-sliced state, can see different worlds. The original keeps no copy of any input: nothing is snapshotted
at the pass start, at SimNewMonth or at the pass end.

| State | Inputs read, per tract `t` or developer, at that tract's (developer's) call | Kept until |
|---|---|---|
| 1 | `HasLand(t)` (terrain altitude and water level of its (S+1)² vertices, §2) | scratch base; it also decides the water-bonus sources of state 2 |
| 2 | no world input except terrain `MaxAllowedHeight`, `MinAllowedHeight` (tuning constants) | state 3 |
| 3 | `HasLand(t)`, `alt(t)`, and `IsZoned(t)` unless ignoreZoning (or HasLand is false: `IsZoned` is evaluated first, so it is read for every tract while ignoreZoning is false) | minAlt, maxAlt, meanAlt, modeAlt, pivot, ranges, topIndex; ignoreZoning for state 8. `SetupAltitudeEffect` (end of state 3, same call as the last tract) reads `MaxAllowedHeight` again when highRange > 100 (§7.3) |
| 5 | per slope tract: terrain altitude and water level of its 9 × 9 vertices (`GetAreaSlope`, §6) | slope grid |
| 4 | `alt(t)` **again** (terrain `GetAltitude` at the tract centre); combined with state 1's scratch and state 3's parameters | intrinsic grid |
| 6 | nothing from the world (copies intrinsic) | |
| 7 | developer i's published desirability map D (slot 13), fetched and applied whole in that developer's call | total grid (written at the end of the list) |
| 8 | `IsZoned(t)` unless ignoreZoning, then `HasLand(t)` only if that passed (short-circuit) | averageLandValue |

So one tract's land/water is read up to three times (states 1, 3, 8), its altitude twice (3, 4), its zoning twice
(3, 8), and its intrinsic value combines a state-1 base, state-3 statistics of **every** tract and a state-4
altitude. A change between those reads gives a result that no single instant reproduces: for example a terrain
edit between state 3 and state 4 makes state 4 use the new altitude against the old pivot and ranges.

**Slicing within a call.** States 1, 3, 4, 5 and 8 loop: test the time budget (`LimitTimer::Expired`, every
iteration while the adaptive stride is 1024, otherwise every 1024/stride iterations; `0x006BE865`), then take the
next tract from the iterator (`0x0060C8D0`), then read and write. The budget test comes **before** `GetNext`, so no
tract is taken and dropped: the tracts read in one call are exactly those the iterator returns in that call, and a
call may read none. A state that finishes ends the call (at most one state per call, §6). Nothing else runs during
a `Step` call (single-threaded; `Step` writes only land value's own grids and members, and at state 8 the history
record and a *queued* LandValueMapChanged), so every input a call reads is the world as it was when that call
started.

**Where the progress is visible** (for a shadow; the original's object, `this` = the `cISC4LandValueSimulator`
interface pointer): pipelineState at `+0x38` (int32, 0..9); the tract iterator pointer at `+0x3C` (states 1, 3, 4,
8, and the reset at 0, 2, 3, 5, 4, 7) and the slope iterator pointer at `+0x40` (state 5). Iterator layout (simgrid
§7.4, `0x0060C8D0`): `+0x00` state byte (0 fresh, 1 running, 2 done), `+0x04` x, `+0x08` z, `+0x0C` step, `+0x10`
x0, `+0x14` z0, `+0x18` x1, `+0x1C` z1. The developer cursor is at `+0x70` (a map node; `+0x8C` holds the map's header
node, which is the end; the list begins at the header's `+0x08`; a node's `+0x10` is the developer pointer and
`+0x14` the weight). ignoreZoning is the byte at `+0x4C`.

**Faithful capture boundary.** The only boundary that gives an independent shadow the original's exact inputs is
**each delivery of SimIdle or SimNewDay to land value's agent** (`DoMessage`, `0x006BF0D0`, the only caller of
`Step`). Read the world, pipelineState and the active iterator immediately before the delivery; read pipelineState
and the iterator again after it. The tracts (states 1, 3, 4, 5, 8) or the developer (state 7) handled in that call
are those between the two positions (a state that changed handled every remaining tract of the old state; state 3's
zoning fallback restarts the scan). Their inputs are the world as read before the call. Any coarser boundary
(SimNewDay, the pass end, a month) is a sample, not the input: it can only be **proved** right when no relevant
input changed between it and the calls it stands for. A boundary is never chosen because its output matches.

Typed input record for one pass (what an implementation or shadow needs to recompute it bitwise):

| Part | Type | Taken at |
|---|---|---|
| land1[t] | bool per land value tract | the state-1 call that handled t |
| land3[t], alt3[t], zoned3[t] | bool, float32 (`GetAltitude` result stored to float32), bool | the state-3 call that handled t (the last scan if the fallback ran) |
| areaSlope[s] | float32 per slope tract | the state-5 call that handled s |
| alt4[t] | float32 | the state-4 call that handled t (absent when there is no altitude curve) |
| D[i] | int16 grid per developer, in list order | developer i's state-7 call |
| zoned8[t], land8[t] | bool (zoned8 not read when ignoreZoning; land8 not read when zoned8 is false) | the state-8 call that handled t |
| maxH, minH | float32 terrain limits | state 2 / end of state 3 |

Recomputing §7 from this record (state 1 from land1, state 2 from scratch only, state 3 from land3/alt3/zoned3, state
4 from alt4, state 5 = areaSlope, state 7 from D, state 8 from the total grid with zoned8/land8) gives the original's
outputs exactly; when all parts were taken at one instant it is the same as running §6 on that instant.

**Startup.** `Init` sets pipelineState 0 (§4.1), so pass 1 starts at the first SimIdle or SimNewDay delivered after
the city is established, and follows the same table. Its state-7 developer maps are what the developers have
published at that call. In a **loaded** city that is, until the developer's first state-17 call after the load, the
map restored by the developer's `Read` (the published map is saved; `Init` neither resizes nor clears a restored
grid and sets the developer's state to 0, growth-developer §1.2, §2). The building development simulator's level-0
SimBegin call starts a monthly update on every developer at once (building-development §4.1, "Recomputed on load"),
so in the first frames after a load both passes run together: land value reaches state 7 at its 8th call, the
developer state 17 at its 17th at the earliest. Usually pass 1 therefore reads the **restored** map and the
developer publishes its first new map before pass 1 ends, so D at the pass end is not the D that pass 1 used. The
previous-map candidate of §7.4 for pass 1 is the restored map: a snapshot of D taken at any time after the load and
before the developer's first state-17 call (slot 10 true and never seen false since the load) is exactly it. In a
new city the developers are created and initialised before the simulation starts and the same rule holds with
their freshly created map.

**Example (pass 1 after a load).** Land value pass 1 runs states 0–6 on idle calls 1–7 and reads D at call 8; the
developer, started by SimBegin, publishes at its 17th idle call. The pass ends at call 9 + (sliced extra calls).
Comparing at LandValueMapChanged with D-now uses the map published at developer call 17, which pass 1 never saw; the
snapshot taken at load (or at the first frame, slot 10 true) is the map it used. If slicing delays land value past
the developer's state 17 instead, D-now is right and the load snapshot is the wrong candidate; the rule "previous
= last snapshot while slot 10 was true" covers both.

### 6.1.1 Native capture ownership and delivery order (LV6, static proof)

The verified native field widths and canonical interface bases are recorded in
[layouts.md, Land-value native progress](../abi/layouts.md#land-value-native-progress-csc4landvaluesimulator-11641-gog-verified-2026-10-22).
That record already includes the iterator, developer nodes, initialization/reference fields and
message-target forwarding ABI; the earlier pending-publication description is superseded.
Publication establishes the field contract, not an installed interceptor or a full train gate verdict.

Use two separately queried interfaces, each held by its own successful QueryInterface reference:
L = `cISC4LandValueSimulator` (IID `0xE7781D8C`), M = `cIGZMessageTarget2` (IID `0x452294AA`).
For this native build M = L + 4; serializable is L + 8 (`0x006BD4DF–0x006BD4F9`).
Progress offsets in §6.1 are relative to L. Native slot-3 forwarding uses M as `this` and the
original message pointer, one 4-byte argument, callee pop 4, bool in AL. Passing L to that slot
reads the wrong fields: DoMessage itself subtracts 4 before Step (`0x006BF113`, `0x006BF15D`).
Only the named 1.1.641 GOG build is established. Executable identity, native vtable/slot identity
and expected bases must be validated before any private-field read; a failure disables capture.

**Proven synchronous boundary.** The two direct calls to Step (`0x006BE7B0`) are in DoMessage:
SimNewDay supplies 5 ms at `0x006BF111–0x006BF116`; SimIdle gets the message's 32-bit data3
through slot 19 and calls Step at `0x006BF155–0x006BF160`. Calendar dispatch invokes target
slot 3 at `0x007039CB`, then releases the message at `0x007039D4`, then tests the saved AL and
advances its cursor. Idle dispatch calls SendAgentMessage at `0x00702EE0`; that function invokes
target slot 3 at `0x00702520`, saves AL and releases the message at `0x00702529` before returning.
Thus the synchronous forwarder's required observation order, **if safely installed**, is:

1. At entry for the actual land-value SimIdle/SimNewDay delivery, copy L's state, ignoreZoning,
   relevant iterator contents and developer cursor/header, then copy the world inputs needed by
   that state's read table. Capture no speculative next-state work; at most one state executes.
2. Forward that same message exactly once to the saved native slot, with native M, in this delivery.
   Preserve the native budget and message fields. Save the native bool result.
3. Before returning to the caller, copy the after-state and progress, and associate the pre-call
   input copies with the work actually performed. State 8 can already have recorded history and
   queued LandValueMapChanged; a later message-server notification is not this boundary.
4. Return the native bool. Both paths retain message ownership in the dispatcher, which releases
   its reference only after the target returns. The observer borrows the message during the call;
   it keeps owned input copies rather than message, iterator, tree-node or grid-storage pointers.

For unchanged tract states, a fresh iterator's stored coordinate is included on its first yield;
a running iterator's stored coordinate has already been yielded and is excluded from new work.
Next advances x first, wraps x to x0 and increases z, with inclusive signed int32 bounds
(`0x0060C8D0–0x0060C92A`). Iterator state 2 is exhaustion, not a valid final tract. An unchanged fresh
iterator means no tract was read. On a pipeline transition attribute work to the old state under
§6's rules, including its skip cases; a newly reset iterator does not imply new-state work.
In the state-3 fallback, ignoreZoning changes to true, statistics
and the tract iterator reset, and state remains 3 (`0x006BEBED–0x006BEC2B`); discard the old
scan's inputs rather than interpreting the reset as ordinary forward progress.

State 6 establishes the first developer cursor without reading its desirability map
(`0x006BEEB3–0x006BEEC3`). A state-7 call with cursor unequal to header reads that one node's
borrowed developer interface, float32 weight and current int16 desirability map, then advances
the cursor and returns (`0x006BEEE7–0x006BEF2E`). Copy that developer's map **before** forwarding;
never substitute a later publication. A state-7 call already at the header processes no developer
and advances to state 8. Developer slot 13's map return is borrowed (plain pointer getter
`0x007C9FB0–0x007C9FB6`); copying it confers no reference on the underlying storage. Successful
L and M QueryInterface references must remain alive through observation and be balanced after
capture stops; they do not transfer ownership of developers, iterators or tree nodes.

**Still unproved: installation and startup.** QueryInterface obtains an interface, not an
interceptor installation. Init registers native M at levels 0, 10, 4 and 6
(`0x006BE6EB–0x006BE724`); AddAgent references the supplied target
(`0x007043C4–0x007043D7`). The proof above does not establish a way to put a proxy in those
existing deliveries while preserving target identity, registration order, shutdown removal and
all notifications. Removing/re-adding an agent or changing a vtable is not approved by this
proof. An additional independent subscription does not bracket the native call. The native
startup windows are established in §6.1.2; a complete safe adapter installation is still unproved.
LV6's field/forwarding contract is resolved in §6.1.3. Installation/lifetime/order evidence is a
separate adapter acceptance requirement; QI does not satisfy it. LV2 and LV7
still require actual per-Step/developer/startup captures in the preserved runtime traces;
static layout proof and output-exact pass-end samples do not resolve them.

### 6.1.2 Native installation window and lifetime (LV6, bounded static proof)

The original provides a synchronous **pre-city-init notification**, not a land-value
interceptor service. The table below distinguishes availability of that notification from
availability of the particular land-value object. These are native facts for the build in
§6.1.1; they do not establish that an adapter has registered or installed a hook.

| Boundary | Native ordering and consequence |
|---|---|
| Loaded city, before city Init | After successful application Read, LoadCity sends city message `0x26D31EC0` with queued=false at `0x00455886–0x00455898`, then calls city Init at `0x004558A2`. City Read requests the saved land-value interface IID `0xE7781D8C` at `0x0045CF23–0x0045CF30`. The loaded interface can therefore already exist before city Init; validate the actual queried result rather than assuming a new owner will be allocated. |
| New city, before city Init | NewCity constructs a city at `0x004548FF`, sends the same message synchronously at `0x00454926–0x00454937` (new-city data1=1), then calls city Init at `0x00454941`. This notification precedes CreateCityComponents: a land-value getter result is **not guaranteed to exist here**. |
| City creates missing components | City Init calls CreateCityComponents at `0x0045BEBF`. That function retains an existing land-value interface or, when absent, requests CLSID `0xE990C18E` / IID `0xE7781D8C` at `0x0045AE6C–0x0045AE8B`. Thus a pre-init object-only forwarder cannot cover the new-city path merely by querying once. |
| Land-value construction | Allocation helper `0x006BDEF0` requests `0xA4` bytes and enters constructor `0x006BDDF0`. The constructor installs L/M/S vtables at `0x006BDE04–0x006BDE11`, clears initialized and reference count at `0x006BDE18–0x006BDE1B`, and clears both iterator pointers at `0x006BDEBB–0x006BDEBE`. Construction alone does not initialize the pipeline or make its progress fields readable. |
| Land-value Init | City Init invokes land-value slot 3 at `0x0045C088`. Native Init checks its one-byte initialized flag at L+`0x0C`; when clear, it reads tuning and prepares the grids, sets pipelineState to 0 at `0x006BE5CA`, and prepares both iterators. It registers **M**, without substituting another target, at levels 0, 10, 4, 6 at `0x006BE6F1`, `0x006BE702`, `0x006BE713`, `0x006BE724`; then subscribes M to the five change notifications. It sets initialized=1 only at `0x006BE79D`. Do not read constructor-era progress or infer initialized from a successful QI. |
| Post-init is a different boundary | NewCity queues `0x26D31EC1` at `0x0045496B`; LoadCity queues it at `0x004558CE`, both after city Init. Simulator DoMessage handles that message, removes its subscription at `0x00703746` and calls HiddenResume at `0x0070374F` (simulator slot 13). A late subscriber or a callback ordered after that handler is not proof of interception before the first Step. |
| Before city Shutdown | PartialCloseCity calls SC4GamePause at `0x00454A7C`, performs application dispatch calls, sends `0x26D31EC2` synchronously at `0x00454AAE–0x00454ABF`, then calls city Shutdown at `0x00454AC9`. It queues `0x26D31EC3` at `0x00454ADD`, releases the city at `0x00454AE7` and clears the application city pointer at `0x00454AEC`. The post-shutdown notification is too late to inspect land-value grids. |
| Owner Shutdown and final release | City Shutdown calls land-value Shutdown at `0x0045B4D2`, releases its interface at `0x0045B4DD` and clears its pointer at `0x0045B4E0`. Native land-value Shutdown clears initialized first (`0x006BDF81`), removes M from all agent levels (`0x006BDF9E`), removes all five subscriptions, then releases dependencies and destroys grids/iterators (§4.2). Final Release calls Shutdown and the deleting destructor at `0x006BD8B5–0x006BD8BE`. A held QI reference can retain the allocation, **not** initialized grids or a usable city. |

The synchronous/queued distinction above is verified in the message sender, not inferred
from the name PostCityMessage. Its final queued argument selects message-server slot 4
(two arguments, including priority=false) at `0x00449B69–0x00449B6E`, or slot 3 (one message
argument) at `0x00449B81–0x00449B84`. Slot 3's body (`0x0092EA65`) collects referenced
notification targets and invokes each target's slot 3 before returning
(`0x0092EB00–0x0092EB20`). Slot 4's body (`0x0092D0BB`) appends to a message queue instead.
Synchronous delivery does not establish exclusion from framework ticks, other threads or
callbacks that pump dispatch. In particular the message server releases its collection lock
before invoking targets (`0x0092EAF1–0x0092EAF8`).
The copy at `0x0092EAD7` obtains a reference through target slot 1 (`0x005447C7`);
the references survive until vector cleanup at `0x0092EB15`, which releases each target
through slot 2 (`0x006C6EA8`).
Removing a subscription does not cancel a target already copied into that dispatch;
unsubscription alone therefore cannot justify reclaiming its callback/table or saved context.

**Earliest dispatch and pause.** The agent cannot receive its registered calendar/idle
deliveries before its registration in Init. There is no initialized-flag guard in native
land-value DoMessage or Step (Step reads state directly at `0x006BE7CD`), so registration
itself must not be treated as permission to call it during Init. OnTick's active/reentrancy
tests are at `0x00703EFE–0x00703F0A`; its
hidden-pause test skips the dispatch loop at `0x00703F25–0x00703F2A`. Ordinary pause skips
calendar work but still permits idle work (`0x00703F2C–0x00703F4E`). A paused city or a
pass-end callback is consequently not a demonstrated installation barrier.

**Native entry spans, not detour instructions.** For DoMessage entry `0x006BF0D0`, a
five-byte branch would need a **six-byte whole-instruction span**, ending before
`0x006BF0D6`: the two one-byte register saves are followed by a four-byte stack-argument
load. A displaced-instruction continuation starts at `0x006BF0D6`; none of those three
instructions uses a relative address. The load must see the native stack arrangement;
calling a displaced prefix with an extra unaccounted return address changes its argument.
The only exported data reference to the entry is native M-vtable slot 3 at `0x00AB166C`.
The export reports no data references to `0x006BF0D1` or `0x006BF0D2`; this is not proof
against arbitrary computed entry. Init's first five bytes are three whole instructions
(three-byte stack reservation, two one-byte register saves), continuation `0x006BE3B5`.
Shutdown requires seven bytes to cover complete instructions, continuation
`0x006BDF77` (two one-byte saves, a two-byte register copy, a three-byte flag load).
All three are x86 __thiscall entries: Init/Shutdown receive L and no stack
arguments; DoMessage receives M and the single four-byte message argument, returning AL
and removing four bytes. This span evidence supplies neither a branch displacement nor
permission to patch executable memory.

**Installation and restoration preconditions (adapter obligations, not native guarantees).**
LV8's distinction between a lifecycle notification and exclusion is specified in §6.1.4.

1. Register the observer for synchronous pre-init and pre-shutdown early enough to receive
   them for every supported city entry path. Validate executable identity, entry bytes and
   native interface identities under §6.1.1 before any private read or mutation. A missed
   pre-init means no startup/per-Step completeness claim for that city.
2. Before any land-value registration can dispatch, establish the interceptor at an actual
   exclusion point. A global entry interceptor installed during pre-init could cover both
   loaded and newly constructed owners without changing agent identity/order. An object
   vtable route additionally needs a proved callback after creation and before registration
   on the new-city branch, and a complete preserved native M-vtable, not a replacement
   object registered in the agent sets. QI alone satisfies neither route.
3. Prove that no thread/tick/notification can enter the affected span or vtable during its
   mutation, that the native function is forwarded once with its original stack/register
   contract, and that reentrant observation cannot pump simulation. Do not manufacture a
   first Step to initialize the owner. Begin progress reads only on actual initialized
   deliveries, and use the state-7 pre-call map copies of §6.1.1 even for the first loaded
   pass; a later post-init/pass-end map cannot replace them.
4. Before owner Shutdown destroys inputs, disable capture and finish outstanding observer
   calls. At a proved exclusion point restore only the entry bytes/vptr still owned by
   this interceptor, preserving any other installed owner. Keep continuation code and any
   cloned table alive while reachable; retire them only after no in-flight call can use
   them. Release the observer's L/M references after restoration, never by manipulating
   the native reference-count field. Clear all owner identity/progress at city teardown;
   an address reused by the next city is a new owner.

**Adapter installation evidence still required (LV8: host-owned).** The native lifecycle and the six-byte DoMessage span
are established, but a complete safe installation is not. The unproved branches are
(a) observer registration/executable-mutation availability before the synchronous calls
at `0x00454937` and `0x00455898`, (b) exclusion of OnTick/notification/in-flight entry
while installing or restoring, including during the pre-shutdown call `0x00454ABF`, and
(c), for an object-only route, the interval from CreateCityComponents's return to
land-value registration on the new-city path. No native callback into the observer has
been established in that interval. Section 6.1.4 closes the native-boundary question:
none of these lifecycle notifications provides a mutation/drain barrier. The host must
satisfy the explicit dispatch contract there; this is adapter acceptance work, not an
unpublished native installation API. Keep `input_capture=pass-end-sample; native_step_capture=0` until they,
the actual capture implementation and full train gate are satisfied. The field ABI is published;
it does not supply these host guarantees. LV2/LV7
still require runtime traces at the real read boundaries.

### 6.1.3 LV6 decision: published layout and capture acceptance

**LV6 answer.** The requested layouts are already recorded in `docs/abi/layouts.md` under
“Land-value native progress”; no additional field row or C ABI export is needed for the
described read-only observation. A version-gated forwarder around the **actual native M
delivery**, reading progress through separately held L, is the observation boundary of
§6.1.1. A QueryInterface-obtained proxy alone is **not** a supported installation: it receives
no existing agent deliveries and is not substituted for the native target by QI. Re-registering
the proxy is not equivalent: native Init registers M and native Shutdown removes M, not that
proxy (`0x006BE6D0–0x006BE724`, `0x006BDF98–0x006BDF9E`). The executable establishes no
observer installation API or callback exclusion guarantee. Consequently the bounded native
proof in §6.1.2 is a host acceptance checklist, not proof that the proposed interceptor has
been installed. Closing the layout question must not enable capture or remove that checklist.

The following is a compact lookup for the published record, not a new layout proposal. Every
offset uses the base in its first column. All addresses/pointers are native 32-bit values;
copy bytes at their stated widths, without reading padding as a field.

| Base | Progress needed by an observer | Width / interpretation |
|---|---|---|
| L (land-value QI result) | +0x0C initialized; +0x10 reference count | 1-byte flag; 4-byte count (observe only) |
| L | +0x38 pipelineState; +0x3C tract iterator; +0x40 slope iterator | int32; two 4-byte pointers |
| L | +0x4C ignoreZoning | 1-byte flag |
| L | +0x70 developer cursor; +0x8C developer header | two 4-byte node pointers; +0x8C is a pointer field, not an embedded node |
| iterator pointer | +0x00 state | 1 byte: fresh 0, yielding 1, exhausted 2 |
| iterator pointer | +0x04 x, +0x08 z, +0x0C step, +0x10 x0, +0x14 z0, +0x18 x1, +0x1C z1 | seven signed int32 fields; inclusive bounds; 0x20-byte allocation |
| developer node | +0x04 parent, +0x08 left, +0x0C right | three 4-byte pointers; header +0x04 is root, header +0x08 is first |
| developer node (not header) | +0x10 developer; +0x14 weight | borrowed interface pointer; IEEE-754 float32 |

M = L + 4 and S = L + 8 are consequences of this build's QI, not offsets to apply to an
arbitrary interface. The byte/dword accesses, inclusive signed iterator comparisons and
float32 weight consumption are cited in the published layout record. Rechecked anchors:
QI `0x006BD4DF–0x006BD4F9`; state dword `0x006BE7CD`; iterator state byte and signed bounds
`0x0060C8D0–0x0060C924`; header/cursor/node payload dwords `0x006BEEB3–0x006BEEFD`.
The native message-target slot expects M, removes one four-byte argument and returns AL;
both native calls to Step pass L. SimNewDay uses budget 5, SimIdle forwards message data3
unchanged (`0x006BF111–0x006BF116`, `0x006BF155–0x006BF160`).

**Progress is a work interval, not a second simulation clock.** A before/after pair describes
only that delivery. For a tract state, include a fresh iterator's starting coordinate only if
it was actually yielded; exclude a yielding iterator's already processed coordinate. After
exhaustion, include the remaining in-bounds coordinates only. A newly reset iterator belongs
to the next phase or the state-3 retry; it is not evidence that another scan ran in that call.
An unchanged yielding position means no further tract work. State 9 supplies no input interval.
The state-3 retry invalidates the *statistics contribution* of the first scan; retain its trace
as a discarded attempt if desired, but compute statistics from the subsequent ignoreZoning scan.

**Developer and startup records.** State 6 establishes a cursor, not a desirability read.
Record a state-7 developer only when the pre-call cursor differs from the header: copy its
key, float32 weight and int16 map for that call, then associate them with the old cursor even
if the cursor reaches the header afterward. The header call consumes no developer and
publishes the accumulated total. This remains true for an empty developer map. For a loaded
city, the state-7 pre-call copy is the map actually available there, whether restored or newly
published; the earlier restored-map snapshot remains useful provenance, not a substitute
chosen by which output matches (§6.1, §7.4). A reference to L/M keeps the owner allocation alive
but cannot retain grids destroyed by Shutdown; capture must end before their destruction.

**Acceptance and labels.** A capture report must identify the native build, city/owner lifetime,
delivery ordinal/type/budget, before/after progress and owned input copies for the work interval.
It must demonstrate interception of the native registrations without changing their identity
or ordering, coverage from the first initialized delivery, and safe installation/restoration
and callback lifetime under §6.1.2. The observer must not pump simulation between its copies
and the corresponding native reads; an intervening mutation or missed delivery prevents a
claim of complete input provenance. A partial trace may describe its observed intervals but
does not prove a complete pass. Until those conditions are demonstrated, existing output
comparisons remain `input_capture=pass-end-sample; native_step_capture=0`, even if exact.
LV2 and LV7 remain open runtime questions. No result in this static ABI answer changes their
algorithms, sampling, golden outputs or fidelity credit. Conformance examples are §16.19–31.

### 6.1.4 LV8: lifecycle ordering versus host dispatch exclusion *[compat]*

**Answer.** No native city lifecycle notification in §6.1.2 supplies the requested
guarantee of no tick, notification or in-flight entry during interceptor installation
or restoration. Synchronous pre-init orders the sender before city Init; synchronous
pre-shutdown orders it before city Shutdown. Neither operation acquires an exclusion
barrier for the observer. Ordinary pause still permits idle dispatch
(`0x00703F2C–0x00703F4E`); HiddenPause changes a count and posts a message, without waiting
for an existing delivery (`0x00702200–0x0070221B`). OnTick's reentrancy byte governs that
OnTick loop (`0x00703F05–0x00703F0C`, cleared at `0x00703F67`), not arbitrary message-server
dispatch or mutation by an observer. The message server releases its collection lock
before calling its retained target snapshot (`0x0092EAF1–0x0092EB15`). Consequently
LV8's proposed lifecycle-only installation guarantee **differs** from the original.
The contract below defines what a host must supply; it does not name an existing game
installation API or certify any particular plugin callback.

**Native closure table.** These answers replace any reading of §6.1.2's candidate
boundaries as an approved installer. All addresses refer to GOG 1.1.641.

| Obligation | Native behavior and conclusion |
|---|---|
| Subscribe before pre-init | NewCity sends pre-init at `0x00454937` and calls city Init at `0x00454941`; LoadCity sends it at `0x00455898` and calls Init at `0x004558A2`. A subscription already present observes that ordering. The sends do not install or register an observer, replay a missed notification, or prove when an external host registered it. |
| Both construction paths | NewCity's land-value owner is created inside city Init, after pre-init (§6.1.2). A QI of an owner during loaded pre-init therefore does not establish coverage for a new city. A global native M route can cover future owners, but only if the host safely establishes it before their first initialized delivery. No new per-object callback is implied. |
| Pause or post-init as installation point | Ordinary pause permits idle; HiddenPause does not drain. Post-init is queued and the simulator resumes on its handler (§6.1.2); subscription order does not certify observation before the first Step. None is a mutation barrier. |
| Pre-shutdown as restoration point | PartialCloseCity pauses and makes application dispatch calls at `0x00454A98` and `0x00454AAB`, then sends pre-shutdown at `0x00454ABF` and calls Shutdown at `0x00454AC9`. This is notice before destruction, without a wait for all observer entries or retained notification targets. |
| Unsubscribe and hold L/M | Shutdown clears initialized at `0x006BDF81`, removes M's agents at `0x006BDF9E`, then removes subscriptions and destroys inputs. A retained reference preserves allocation, not the grids. A target already copied by message-server dispatch remains callable after unsubscription. Neither operation retires a forwarder. |

**Required host dispatch contract.** This is a behavioral acceptance contract for an
adapter, separate from native simulation behavior. An installer must identify the host
mechanism satisfying each row, or leave native capture disabled. Calling an arbitrary
callback “pre-init,” “paused,” or “on the simulation thread” is insufficient evidence.

| Phase | Required observable guarantee |
|---|---|
| Admission and exclusion | Before changing a shared native M slot, affected vptr or executable span, prevent new affected calls from resolving/entering that route and let previously admitted calls finish. This covers calendar and idle deliveries, message-server notifications, nested dispatch and every thread allowed to use the route. For a global route it covers all owners, not only the city currently observed. A lock or counter entered only inside the forwarder cannot exclude a caller that has already fetched a destination or is executing the native prefix. |
| Publication | Validate identity and ownership under that exclusion, publish a complete usable forwarding route, and keep the native target, message identity, registration order, budget and result unchanged (§6.1.1). Validation failure leaves the native route intact and capture disabled. No byte sequence, memory-protection API, atomic-publication primitive or detour implementation is specified here. |
| Startup coverage | The route must be ready before the first initialized delivery on **each** new and loaded city path. Do not read progress during construction/Init; native Init registers M before its final initialized store at `0x006BE79D`. The host must exclude delivery in that interval, then observe actual deliveries after Init completes. A route discovered later may collect a partial diagnostic trace but cannot label the missed startup pass complete. |
| Observation interval | Keep owner inputs valid throughout the before/native/after interval and prevent relevant world mutation between the copies and native reads. Do not pump dispatch from observation. If nested or concurrent entry cannot be excluded, do not claim a faithful record for that interval. Every real native delivery still forwards exactly once; observation must not suppress a delivery, replay one or synthesize a Step. |
| Owner shutdown | Stop admitting private reads for that owner and complete any admitted before/native/after interval **before** native Shutdown destroys its inputs. Quiescence cannot be obtained by waiting for the currently executing callback to return from inside that same callback. A host that cannot arrange this ordering must not start capture. Clear city identity and borrowed pointers; retain only owned input records. A later city at the same address is a different owner. |
| Restoration and retirement | Under route-wide exclusion, restore only a route still owned by this installer. A foreign replacement must not be overwritten. Prove that no executing call, fetched destination, retained target or nested callback can still reach the observer's code, table, continuation or context before reclaiming them or unloading its module. A zero count of calls already inside the observer, an unsubscribe, or the queued post-shutdown message is not that proof. Separately held L/M references are released after their last use and any owner-specific restoration; their reference counts are never edited directly. |

**Global and owner lifetimes are distinct.** A host may keep a global forwarding route
installed across city changes, provided it remains valid and resident and obeys the
owner-read rules above. Pre-shutdown then ends that city's capture, not necessarily the
global route's lifetime. A surviving route forwards uncaptured messages without reading
retired owner state. Retaining code/table/continuation resources until process exit can
avoid reclaiming them between cities; it does not excuse unsafe initial publication,
lost startup deliveries, reads after Shutdown, or a module unload while still reachable.
Conversely, restoring the route does not by itself retire a callback already in flight.

**Acceptance result.** LV8 is closed as a native-contract question: the game supplies
lifecycle ordering, not the requested exclusion facility. A host implementation must
demonstrate the contract above with install/restore and lifetime evidence; there is no
further native field or signature to infer for this answer. Until that evidence and the
actual before/forward/after capture exist, retain
`input_capture=pass-end-sample; native_step_capture=0`. Closing LV8 does not close LV2 or
LV7, certify an installed adapter, or upgrade exact sampled output to input provenance.

## 7. Formulas *[compat]*

### 7.1 Water bonus (state 2)

`AddDistanceEffect` (simgrid.md §7.2) with:

| Argument | Value |
|---|---|
| target grid | scratch |
| grid dimensions | the **intrinsic** grid's tract counts (X, Z) |
| mask grid, mask value | scratch, 0 (tracts without land are the "sources") |
| radius | float(tuning `0x47E2C310[1]`) (shipped 10) |
| near value | float(tuning `0x47E2C310[0]`) (shipped 50) |
| far value | 0 |

The distance is the 4-neighbour (Manhattan) tract distance to the nearest tract without land. For a land tract at
distance d with 0 < d < radius: scratch(t) := clamp(trunc(scratch(t) + near − d · (near − far)/radius), 0, 255).
With the shipped values: `base + 50 − 5d` for d = 1..9, unchanged from d = 10 on. Tracts without land stay 0.
If base were 0, every tract would be a source and nothing would change *[quirk]*.

### 7.2 Altitude statistics (state 3)

For every tract t (iterator order), with a = alt(t):

1. If `HasLand(t)`: if a < minAlt then minAlt := a; if a > maxAlt then maxAlt := a.
2. If (`ignoreZoning` or `IsZoned(t)`) and `HasLand(t)`:
   - sum := trunc(sum + a) (int32 sum, added as a float and truncated toward zero at each step);
   - count := count + 1;
   - bin := trunc(a) >> histShift (arithmetic shift), clamped to `[0, bins − 1]`; histogram[bin] += 1.

At the end of the iteration:

- If count = 0 and not ignoreZoning: ignoreZoning := true; reset the iterator, sum, count and every bin to 0;
  **stay in state 3** (the next call recounts every land tract, zoned or not). minAlt/maxAlt are kept.
- Otherwise:
  - meanAlt := count > 0 ? float32(sum / count) : 0 (the division is done in floating point).
  - best := the **lowest** bin index with the strictly largest count (bin 0 if all are 0).
  - modeAlt := float(2^(histShift−1) + best · 2^histShift) = 16 + 32·best; then if modeAlt < minAlt,
    modeAlt := minAlt; then if modeAlt > maxAlt, modeAlt := maxAlt.
  - Altitude-effect setup (§7.3); reset the slope iterator; go to state 5.

If no tract has land at all, minAlt = 2500 and maxAlt = 20 stay inverted (shipped terrain tuning) and modeAlt ends
at maxAlt *[quirk]*.

### 7.3 Altitude effect

**Setup** (end of state 3), all in `float32` storage:

```
pivot     := min(modeAlt, meanAlt)               (meanAlt when equal)
lowRange  := pivot − minAlt ;  if not (lowRange  > 1.0): lowRange  := 1.0
highRange := maxAlt − pivot ;  if not (highRange > 1.0): highRange := 1.0
topIndex  := 100
if highRange > 100:  topIndex := min(150, trunc((highRange − 100) / (terrain.MaxAllowedHeight() − 100) × 100 + 100))
```

(Each operation is rounded to a 24-bit significand, like one `float32` operation, because the game runs the
x87 at 24-bit precision (foundations §18); the result is then truncated toward zero.)

**`AltitudeEffect(a)`** → byte:

```
if a < minAlt:          return 0
if a >= pivot:          x := (topIndex − 50) / highRange × (a − pivot) + 50
else:                   x := (a − minAlt) / lowRange × 50
i := trunc(x)                                     (toward zero)
return low 8 bits of trunc( altitudeCurve(float(i)) )
```

The curve input is an integer index: 0 at the lowest land, 50 at the pivot, `topIndex` (100..150) at the highest
land. The result is truncated toward zero and then **wraps** to 8 bits: a curve output of −10 adds 246, and 300
adds 44 *[quirk]* (the shipped curve stays in 0..50). State 4 adds this byte to scratch and clamps at 255.

Shipped altitude curve `0x47E2C320`: (0,0) (25,5) (50,10) (75,21) (100,50) (125,30) (150,0): low land gets
little, land slightly above the typical altitude gets up to +50, the highest land falls back to 0.

### 7.4 Desirability (state 7, one developer per call)

The developer list is built by `Init` (§8): for each ID i in tuning `0x47E2C330`, the growth developer pointer
`BuildingDevelopmentSimulator.GetDeveloper(ID_i)` is the key and `0x47E2C331[i]` the weight. Later duplicates
overwrite the weight of an earlier identical key.

**Order** *[compat, see note]*: the list is a map keyed by the developer pointer, so it is ordered by the
**developer object's address** (in practice heap-allocation order), not by tuning order.
Because each application clamps to [1, 255], the order matters when two or more developers are listed. With the
shipped tuning there is exactly one developer (ID `0x1030`), so order does not matter. *[choice]* FreeCity4 applies
developers in tuning order. A null developer (unknown ID) is not filtered: the original would crash in state 7
*[quirk]*; *[choice]* FreeCity4 skips it.

**What "address order" is** (QUESTIONS LV1, from the exe). The key is the pointer as an **unsigned 32-bit
integer** (`map<uint32, float>` with `less`, filled by the tuning reader `0x006BE110`), and state 7 walks the map
from its smallest key up. So the developers are applied in **ascending unsigned pointer value**. That value is
fixed by the allocator, not by any listing:
- each growth developer is a 0x13C-byte object allocated with the game's `operator new`;
- in a new city the building development simulator creates them in its `Init`, one per exemplar of the resource
  manager's enumeration (callback `0x00699E20`, creation `0x006991A0`), in that enumeration's order;
- in a loaded city the class store creates them while reading the building development simulator's record (class
  factory `0x006B7870`), and `Init` then only updates them.

So the address order equals neither the tuning order (`0x47E2C330`) nor, in general, the creation order. A shadow
of the original reproduces it exactly by sorting the pointers that `GetDeveloper(ID)` returns, unsigned and
ascending; ties cannot occur (duplicate IDs give the same pointer and share one entry, the later weight winning).
For FreeCity4's own developers the tuning-order *[choice]* above stands.

For the current developer, with D = its desirability map (`GetDesirabilityMap()`, an `int16` grid) the helper
`ProcessTwoGridsWithCallback` (simgrid.md §7.3) maps D onto scratch's tracts and calls, for every scratch tract,
the step below. The desirability map's tract size is the growth developer's tuning property `0x87CD6310`, which
is **4** in all twelve shipped developer exemplars (`0x67CD5FA1` / `0x1010` … `0x4400`). With the shipped
scratch tract size 2, D is the **coarser** grid: each desirability tract is applied unchanged to the 2 × 2
scratch tracts it covers (the "A finer" case of simgrid §7.3) *[compat]*.

**Coverage** *[compat]*. In that case the helper (`0x006BDA30`) loops over the **desirability** map's tracts and
calls the step only for the scratch tracts they cover. If the desirability map covers a smaller area than the
scratch grid, the scratch tracts outside it are **skipped**: the step does not run for them, so their value is
neither changed nor clamped to 1 (a value 0 stays 0). This is not the same as reading `d = 0`, which would clamp 0
to 1 and add `curve(0) × weight`. With equal tract sizes, or a scratch grid coarser than the desirability map, the
original instead loops over the scratch grid and reads the desirability map without bounds checks (out-of-range
reads; not reachable with shipped data, where both grids cover the same city). *[choice]* FreeCity4 then skips
every scratch tract whose source tracts are not all inside the desirability map, and also skips covered tracts
that fall outside the scratch grid when the desirability map is larger.

```
e := desirabilityCurve present ? desirabilityCurve(float(d)) : float(d)
v := clamp( trunc(v + e × weight), 1, 255 )                 (truncation toward zero, then clamp)
```

where `d` is the matched desirability value (int16), `v` the scratch byte. Note the lower clamp **1**, not 0
*[compat]*. With the shipped curve (−255,−255) (255,255) and weight 2.5: `v + 2.5·clamp(d, −255, 255)`.

**Which desirability map state 7 sees** *[compat]* (QUESTIONS LV2, from the exe). The map is the developer's
**published** grid D (growth developer slot 13, `0x007C9FB0`, the member at `+0x114`). The developer builds its
next map in a separate working grid W and copies W into D as a whole, in one call, at state 17 of its update
(growth-developer.md §3.2, §5.1). The building development simulator starts that update once a month (`StartMapUpdate`
on every developer from its month agent). Nothing else writes D except the developer's own `SetTractSize`, `Read`
and `Init`. State 7 fetches D through slot 13 and applies it in the same call. So:
- state 7 never sees a half-built map: it sees either the map published before this month's developer update, or
  the one published by it;
- between state 7 and the delivery of LandValueMapChanged (§9, queued) D changes **at most once** in practice, by
  that copy: both updates start at the month boundary, and a land value pass ends long before the next one (one
  state per day or idle call, idle calls every frame).

Both passes advance about one state per day or idle call. Land value reaches state 7 at its 8th call, the
developer reaches state 17 at its 17th call at the earliest (state 15 can take several calls), so land value
usually reads the **previous** month's map. Time-sliced states on either side can reverse that.

A shadow that compares at LandValueMapChanged therefore needs exactly two candidates per developer: D as it was
before the developer's last state-17 copy, and D now. The shadow's "rerun with the previous map and keep the better
result" covers every case, provided "previous" is the D published before the latest copy. A shadow can capture it
with the developer's slot 10, `GrowthAndDeclineListsAreValid` (`state < 17`): D changes only in a state-17 call,
and slot 10 is false from the moment state 17 is pending, so D cannot change while slot 10 is true. The last
snapshot of D taken while slot 10 was true is therefore the map from before the latest copy.

**A developer first seen while slot 10 is false** (QUESTIONS LV5). Slot 10 is false only in states 17 to 20
(growth-developer §3.2), from the call that ends state 16 to the call that ends state 20 and sets state 0. D changes
in the first call of that window, state 17's. A developer first seen inside it has therefore already published its new
map unless it is still in state 17, and no slot tells state 17 from states 18–20 (slot 28 is `state == 0`). The map
from before the copy cannot be recovered through the interface. The window is reached only by a shadow that starts
watching during a monthly update: the state is 0 after construction, `Init` and a load (growth-developer §2), so slot
10 is true until the developer's first update.

### 7.5 Average (state 8)

For every tract with (`ignoreZoning` or `IsZoned(t)`) and `HasLand(t)` (ignoreZoning as left by state 3):
sum += total(t); count += 1 (int32). averageLandValue := count > 0 ? low byte of (sum div count) (truncating
integer division) : 0. Then `HistoryWarehouse.RecordHistoryData(0x6A663AC8, averageLandValue)`.

### 7.6 Queries

- `GetLandValue(x, z)` = total grid `GetCellValue(x, z)` (**cell** coordinates, simgrid.md §4, no bounds check).
- `GetLandValueType(x, z)`: v := `GetLandValue(x, z)`; v < `0x47E2C301[0]` → 1 (low); else v < `0x47E2C301[1]` →
  2 (medium); else 3 (high). Unsigned byte comparisons. Shipped boundaries 70 and 120.
  **Uniform over a tract** *[compat]* (QUESTIONS LV4, `0x006BD700`): v is the total grid's `GetCellValue(x, z)`
  (grid slot 7), which reads `data[x >> shift][z >> shift]` (simgrid §4). Every cell of a tract therefore gives the
  same value and the same type, and no cell is preferred: sampling a tract at its first cell `(tx·S, tz·S)` gives
  exactly what the original answers for any of its S × S cells. Example (S = 2, total(3, 5) = 95): cells (6, 10),
  (7, 10), (6, 11) and (7, 11) all answer 95 and type 2. There is no bounds check (out-of-range cells read
  outside the rows).
- `GetAverageLandValue()` = averageLandValue; `GetModeAltitude()` = modeAlt (0 until the first state 3 after
  creation or load, because it is not saved).

## 8. Tuning *[compat]*

Exemplar TGI (type, group, instance) = `0x6534284A, 0xE7E2C2DB, 0xE7E2C8D8` ("Land Value Sim", SimCity_1.dat),
loaded through the resource manager as a property holder. If the exemplar is missing, nothing is read and every
value keeps its current value (code defaults on the first `Init`).

| Property | Name | Type | Code default | Shipped | Use |
|---|---|---|---|---|---|
| `0x47E2C200` | Tract Size | u8 | 2 | 2 | S for the three uint8 grids (only when created, §4.1) |
| `0x47E2C300` | Intrinsic Min/Max | u8[2] | 20, 120 | 1, 64 | [0] = base (state 1); [1] stored, never used |
| `0x47E2C301` | Wealth Boundaries | u8[2] | 50, 75 | 70, 120 | `GetLandValueType` thresholds |
| `0x47E2C310` | Water Effect | u8[2] | 100, 5 | 50, 10 | [0] = near value, [1] = radius in tracts (§7.1) |
| `0x47E2C320` | Altitude Effect | f32 curve | absent | see §7.3 | `AltitudeEffect`; absent → state 4 skipped |
| `0x47E2C330` | Desirability IDs | u32[] | — | `0x1030` | growth developer types (§7.4) |
| `0x47E2C331` | Desirability Factors | f32[] | — | 2.5 | weight per ID, same index |
| `0x47E2C332` | Desirability Curve | f32 curve | absent | (−255,−255) (255,255) | §7.4; absent → identity |

Details *[compat]*:
- `0x47E2C200` is read with the "optional u8" helper: absent → unchanged.
- `0x47E2C300`, `0x47E2C301`, `0x47E2C310`, `0x47E2C330`, `0x47E2C331` are read without a presence check: the
  original requires them (an absent one is a null dereference). *[choice]* FreeCity4 keeps the current values.
- `0x47E2C330`/`0x47E2C331`: the number of entries is the ID array's count; the weight array is indexed in
  parallel without a length check. *[choice]* FreeCity4 uses min(count of IDs, count of weights).
- Curves: a present property (re)builds the curve; an absent property deletes it (foundations §4.1).
- The terrain values used here come from the terrain tuning (not this exemplar): MaxAllowedHeight =
  `0x48CD7B26` (shipped 2500), MinAllowedHeight = `0x48CD7B27` (shipped 20), in exemplar
  `0x6534284A, 0x88CD66E9, 0x00000001`.
- `0x1030` is a residential growth developer type (high wealth *[inferred]*).

## 9. Messages posted *[compat]*

| ID | When | Fields |
|---|---|---|
| `0x87CC75B3` (LandValueMapChanged / kMessageTypeMapChanged) | end of state 8 (every completed pass) | type only; data fields 0; `MessagePost(msg, false)` |

## 10. Persistence (`cIGZSerializable`) *[compat]*

`GetGZCLSID()` = `0xE990C18E`. Little-endian, in this order:

| # | Type | Value |
|---|---|---|
| 1 | u16 | version = 1 |
| 2 | ref | intrinsic grid (foundations §2.7) |
| 3 | ref | total grid |
| 4 | ref | slope grid |
| 5 | u8 | 1 if dirty **or** pipelineState ≠ 9, else 0 |
| 6 | u8 | averageLandValue |
| 7 | f32 | meanAlt |

`Write` returns true iff the stream reports no error afterwards. The scratch grid, histogram, modeAlt, ranges and
tuning are not saved. The grids' own data is written by the class-object store in their own class records
(`0x49B9E602`, `0x49B9E60A`; simgrid.md §5).

`Read`: reads the version; if it is not exactly 1, returns false and reads nothing else. Otherwise it reads the
three references, each requesting IID `0xC9BD939A` (`cISC4SimGrid`) with deferred loading, then
dirty := (byte 5 ≠ 0), averageLandValue := byte 6, meanAlt := field 7, and returns true iff the stream reports no
error. `Read` does not touch the pipeline state; the following `Init` sets it to 0, so a loaded city always
recomputes land value once, and once more at the next month if the saved flag was 1.

Byte-exact example: section 16, test 12.

## 11. Interface slot tables *[compat]*

All methods are `__thiscall`; "pop" is the number of argument bytes the callee removes.

### 11.1 `cISC4LandValueSimulator` (original vtable `0x00AB1670`)

| Slot | Offset | Method | Pop | Return |
|---|---|---|---|---|
| 0 | +0x00 | `QueryInterface(uint32 iid, void** out)` | 8 | AL bool |
| 1 | +0x04 | `AddRef()` | 0 | EAX new count |
| 2 | +0x08 | `Release()` | 0 | EAX new count (0 when destroyed) |
| 3 | +0x0C | `Init()` | 0 | AL = 1 |
| 4 | +0x10 | `Shutdown()` | 0 | AL = 1 |
| 5 | +0x14 | `GetLandValueType(int32 x, int32 z)` | 8 | EAX = 1, 2 or 3 (full 32 bits) |
| 6 | +0x18 | `GetLandValue(int32 x, int32 z)` | 8 | AL (upper EAX bits undefined) |
| 7 | +0x1C | `GetIntrinsicLandValueMap()` | 0 | EAX `cISC4SimGrid<uint8>*` (no `AddRef`) |
| 8 | +0x20 | `GetTotalLandValueMap()` | 0 | EAX `cISC4SimGrid<uint8>*` (no `AddRef`) |
| 9 | +0x24 | `GetSlopeMap()` | 0 | EAX `cISC4SimGrid<float>*` (no `AddRef`) |
| 10 | +0x28 | `GetAverageLandValue()` | 0 | AL (upper EAX bits undefined) |
| 11 | +0x2C | `GetModeAltitude()` | 0 | **ST0** float |
| 12 | +0x30 | (class destructor, not part of the interface) | 4 | — |

**SDK check** (`cISC4LandValueSimulator.h`): order, argument sizes and return types all match, including the
float return of `GetModeAltitude` (verified: the value is loaded onto the x87 stack) and the `uint32` enum of
`GetLandValueType`. No SDK error. The original leaves the upper EAX bits of slots 6 and 10 undefined and its
callers read only AL, so declaring them `uint8_t` is correct.

### 11.2 `cIGZSerializable` (vtable `0x00AB1648`)

| Slot | Method | Pop | Return |
|---|---|---|---|
| 0–2 | QueryInterface / AddRef / Release (adjust to the main object) | 8 / 0 / 0 | as above |
| 3 | `Write(cIGZOStream&)` | 4 | AL bool (§10) |
| 4 | `Read(cIGZIStream&)` | 4 | AL bool (§10) |
| 5 | `GetGZCLSID()` | 0 | EAX `0xE990C18E` |

### 11.3 `cIGZMessageTarget2` (vtable `0x00AB1660`)

| Slot | Method | Pop | Return |
|---|---|---|---|
| 0–2 | QueryInterface / AddRef / Release | 8 / 0 / 0 | as above |
| 3 | `DoMessage(cIGZMessage2*)` | 4 | AL = 1 always (§5) |

This interface's native `this` is M = L + 4, where L is the separately queried land-value
interface. Slot 0's thunk subtracts 4 (`0x006BDDB0`); slot 3 expects M directly. The forwarding
contract and installation acceptance requirements are in §6.1.1–§6.1.3. Obtaining M by QueryInterface
does not install a forwarder or change the simulator's existing agent targets.

## 12. Dependencies (methods the original calls) *[compat]*

| Service (how obtained) | Interface, vtable | Slot | Method | Pop | Return |
|---|---|---|---|---|---|
| city (global at city init) | `cISC4City`, `0x00A87620` | 50 | `GetSimulator()` (in `Shutdown`) | 0 | EAX |
| simulator (global) | `cISC4Simulator`, `0x00AB2FB8` | 19 | `AddAgent(target, level, const cIGZString& name, flags)` | 16 | AL |
| | | 20 | `RemoveAgent(target)` (all levels) | 4 | AL |
| message server (global) | `cIGZMessageServer2`, `0x00AD1BF4` | 4 | `MessagePost(msg, bool highPriority)` | 8 | AL |
| | | 5 / 6 | `AddNotification` / `RemoveNotification(target, type)` | 8 | AL |
| zone manager (global) | `cISC4ZoneManager`, `0x00A92E00` | 12 | `GetZoneType(int32 x, int32 z)` | 8 | EAX |
| terrain (city `GetTerrain`) | `cISTETerrain`, `0x00AB3D18` | 13 | `GetAltitude(float x, float z)` | 8 | ST0 |
| | | 23 | `MinAllowedHeight()` | 0 | ST0 |
| | | 24 | `MaxAllowedHeight()` | 0 | ST0 |
| | | 36 | `IsLandVertex(uint32 x, uint32 z)` | 8 | AL |
| building development simulator (global) | `cISC4BuildingDevelopmentSimulator`, `0x00AB0AF8` | 32 | `GetDeveloper(DeveloperType)` | 4 | EAX |
| | | 36 | `GetLotDeveloper()` | 0 | EAX |
| lot developer | `cISC4LotDeveloper`, `0x00AB1B98` | 29 | `GetAreaSlope(const SC4Rect<int32>&, bool)` | 8 | ST0 |
| growth developer | `cISC4GrowthDeveloper`, `0x00AB1488` | 13 | `GetDesirabilityMap()` | 0 | EAX `cISC4SimGrid<int16>*` |
| history warehouse (global) | `cISC4HistoryWarehouse`, `0x00A8CCB4` | 12 | `RecordHistoryData(type, value)` | 8 | — |
| | | 13 | `SetHistoryDataQueueLength(type, length)` | 8 | — |
| resource manager | `cIGZPersistResourceManager` (foundations §17.6) | 3 | get the tuning exemplar | 20 | AL |
| property holder / variant | foundations §17.1, §17.2 | | `GetProperty(id)`, value arrays (u8, u32, f32), counts | | |
| own grids | `cISC4SimGrid<T>` | | simgrid.md §4 slots 3, 6, 7, 9–13, 21, 24/25 | | |

**Reference counting** *[compat]*: `cISC4City::GetBuildingDevelopmentSimulator` (compiled slot 53, a plain field
read), `GetDeveloper(DeveloperType)` (slot 32, a map lookup returning null when absent) and `GetLotDeveloper()`
(slot 36, a plain field read) all return **borrowed** pointers: none of them calls `AddRef`. Land value stores the
growth-developer pointers in its developer list without `AddRef` and never releases them; it holds no reference
to the lot developer either. A replacement must not `Release` what these getters return, and a replacement of
the building development simulator must keep the developers alive for the city's lifetime.

Terrain: the SDK's `cISTETerrainMap` declares `GetAltitude(float, float)` before `GetAltitude(int32)`; in the
compiled vtable the `(float, float)` overload is slot **13** and `(int32)` slot 12 (foundations §16). Slots 23, 24
and 36 match the SDK. The terrain interface rows are in `docs/abi/interfaces.tsv` after regeneration.

## 13. Consumers and replaceability

### 13.1 Consumers (all through the vtable; no direct calls to land value code were found)

| Consumer | Uses |
|---|---|
| budget (`ComputeTax`) | slot 6 `GetLandValue` (AL) per cell |
| `cSC4Lot::GetLandValue` | slot 6, summed over the lot's cells |
| growth developers (`DoIdle`, `EstimateDesirability`) | slots 7/8 (intrinsic or total, chosen by a developer flag) and 9; then grid `GetCellValue` **and** `GetGridData` with **raw reads of the grid's vector memory** |
| aura simulator | slot 5 `GetLandValueType` at half-resolution cell coordinates |
| query tool, cell description, debug info | slots 5, 7, 8, 9 + grid `GetCellValue` |
| ambience | slot 8 + grid `GetCellValue` |
| map view (land value layer) | slot 8 + grid `GetGridData` (raw vector reads) |
| budget, building development, tract developer, growth developer `Init` | hold a reference (`AddRef`) |

### 13.2 Replaceability findings

- Created only through GZCOM (city) and the class store (load). The class is **replaceable through the class
  registry** (register `0xE990C18E`).
- **The grids it hands out are read as raw memory** by the growth developers and the map view (simgrid.md §6.3):
  the objects returned by slots 7, 8 and 9 must expose, through `GetGridData`, an STLport-layout
  `vector<vector<uint8>>` / `vector<vector<float>>` (three pointers per vector, rows indexed by x). *[compat]*
  The simplest compliant replacement uses the **game's own** grid class: create the four grids with
  `GZCOM()->GetClassObject(0x49B9E602 / 0x49B9E60A, 0xC9BD939A, &p)` (registered by the game's COM director) and
  write cells with `SetTractValue` or through `GetGridData`. FreeCity4 must **not** register the SimGrid class IDs
  itself unless its grids are layout-compatible (simgrid.md §6.3).
- The original constructs its grids with `new` and direct constructor calls (bypassing GZCOM); on load they come
  from the class store via their class IDs. A replacement that uses the game's grid class therefore loads old saves
  unchanged.
- The original reads the growth developer's desirability grid through `cISC4SimGrid` slots 11–13 and
  `GetGridData` (slot 25) (simgrid.md §7.3), i.e. it reads that grid's raw memory as well; a replacement may use
  the interface's `GetTractValue` instead.
- No original code outside the class reads its members directly.

## 14. Original quirks (reproduced)

Missing altitude curve leaves intrinsic stale and discards the water/base work (§6 state 4); vertices-inclusive vs
cells-exclusive tract predicates (§2); 8-bit wrap of the altitude effect (§7.3); lower clamp 1 in the
desirability step (§7.4); inverted min/max start values (§7.2); restored grids keep their saved tract size even if
the tuning changed (§4.1; with a different size the scratch grid and the intrinsic grid disagree, which the
original does not guard against: *[choice]* FreeCity4 resizes restored grids that do not match S and then runs a
full pass); Intrinsic Max (`0x47E2C300[1]`) is never used; `GetModeAltitude` is 0 after load until state 3 runs.

## 15. Open questions

1. *(resolved)* x87 precision: the game sets 24-bit precision on the main thread at start-up and the whole
   simulation runs with it (foundations §18). Every step of §7.3 and §7.4 is therefore a chain of `float32`
   operations, each rounded to nearest-even, then truncated; for example 5 × float32(−0.8) + 50 is evaluated as
   two `float32` operations. Computing these steps in `f32` (as the implementation does) matches the original.
2. *(resolved)* The growth developers' desirability maps use tract size 4 (`0x87CD6310` = 4 in every shipped
   developer exemplar), so state 7 uses the "A finer" branch of simgrid §7.3 (§7.4).
3. Whether any shipped content lists more than one desirability developer (only then does the address-order
   question of §7.4 matter).

4. *(resolved)* LV1, native pass input capture and startup snapshots: §6.1. A pass reads its inputs per state
   and per call, not at one instant; the exact capture boundary is each land value `Step` call; pass 1 after a load
   usually reads the developers' restored maps. Matching an earlier day's sample is evidence, not a contract.

5. *(resolved)* LV6, native progress/forwarding ABI: §6.1.1–§6.1.3 and the published
   `docs/abi/layouts.md` record. QI alone does not install a forwarder. Host installation,
   lifetime and real input-provenance validation remain adapter acceptance work; LV2/LV7
   remain open. The existing pass-end sample earns no per-Step fidelity credit.

6. *(resolved)* LV8, installation/exclusion contract: §6.1.4. Native pre-init and
   pre-shutdown provide ordering only. The host must supply route admission, draining,
   safe publication and retirement; vectors 26–31 distinguish these from notifications.
   This static answer grants no native capture or LV2/LV7 runtime verification credit.

## 16. Conformance tests (must exist in `engine/conformance`)

Shipped tuning unless stated; S = 2.

1. `GetLandValueType` with boundaries (70, 120): total value 0 → 1, 69 → 1, 70 → 2, 119 → 2, 120 → 3, 255 → 3.
2. Code defaults with no exemplar: base 20, boundaries (50, 75), water (100, 5), S = 2.
3. Water bonus (§7.1), base 1, near 50, radius 10, a straight shoreline: tracts at Manhattan distance 1, 2, 5, 9,
   10, 15 from the nearest no-land tract get 46, 41, 26, 6, 1, 1; no-land tracts stay 0.
4. Same with radius 10 and near 100 (code default near, shipped radius): d = 1 → 1 + 100 − 10 = 91; d = 3 → 71.
5. Altitude statistics: five zoned land tracts with altitudes 270.5, 275, 290, 300, 301 (no other land) →
   sum 1436, count 5, meanAlt = 287.2 (float32 `0x438F999A`), bins 8,8,9,9,9 → best 9 → mode 16 + 288 = 304 →
   clamped to maxAlt 301; minAlt = 270.5; pivot = 287.2.
6. Mode tie: bin counts {8: 2, 9: 2} → best = 8 → mode 272 (before clamping).
7. Zoning fallback: no zoned tract, land tracts at 300 and 340 → first pass of state 3 counts nothing, state stays
   3; second pass counts both: meanAlt 320, mode = 16 + 32·9 = 304 (bins 9 and 10 tie → 9).
8. Altitude effect with minAlt 260, maxAlt 322, modeAlt 272, meanAlt 300 → pivot 272, lowRange 12, highRange 50,
   topIndex 100. AltitudeEffect: a = 259 → 0; 260 → curve(0) = 0; 263 → i = 12 → curve(12) = 2.4 → 2;
   266 → i = 25 → 5; 272 → i = 50 → 10; 297 → i = 75 → 21; 322 → i = 100 → 50.
9. topIndex: highRange 100 → 100; highRange 340 with MaxAllowedHeight 2500 → trunc(240/2400 × 100 + 100) = 110;
   highRange 3000 → 150 (clamped).
10. Desirability step, identity curve (shipped), weight 2.5: (v, d) = (100, 40) → 200; (200, 40) → 255;
    (100, −50) → 1; (10, −3) → trunc(2.5) = 2; (1, 0) → 1; (0, 0) → 1 (clamp from 0 up to 1); (100, 400) →
    curve clamps d to 255 → 100 + 637.5 → 255.
11. Average: total values 10, 20, 31 on three counted tracts → sum 61, average 20 (truncation); with no counted
    tract → 0. History record (0x6A663AC8, 20) and one LandValueMapChanged post per completed pass.
12. Save bytes with intrinsic objectID 0x11, total 0x12, slope 0x13, dirty false, state 9, average 42,
    meanAlt 287.2:
    `01 00 | 11 00 00 00 02 E6 B9 49 | 12 00 00 00 02 E6 B9 49 | 13 00 00 00 0A E6 B9 49 | 00 | 2A | 9A 99 8F 43`.
    With state 5 (mid-pass) and dirty false, byte 5 is `01`.
13. `Read` of version 2 → false, nothing else consumed. `Read` of the bytes of test 12 → dirty false, average 42,
    meanAlt 287.2, `GetModeAltitude()` = 0; after `Init`, pipelineState = 0.
14. Messages: InsertOccupant sets dirty; SimNewMonth with dirty and state 9 → state 0, dirty false; SimNewMonth with
    dirty and state 4 → no change; every `DoMessage` returns true.
15. Pipeline call count: with one developer and at least one zoned land tract, an unlimited budget completes a
    pass in exactly 10 `Step` calls (states 0, 1, 2, 3, 5, 4, 6, 7 (developer), 7 (end), 8); the 10th call posts
    LandValueMapChanged. With no zoned tract: 11 calls.
16. Missing altitude curve: intrinsic keeps its previous contents (all 0 in a new city) and total = clamp[1,255] of
    the desirability terms only.
17. Desirability mapping (shipped tract sizes: scratch 2, desirability 4), identity curve, weight 2.5: a
    desirability tract (1, 2) holding 40 adds 100 to each of the scratch tracts (2, 4), (3, 4), (2, 5), (3, 5)
    and to no other tract; a scratch tract value 100 there becomes 200.
18. Desirability map smaller than the scratch area (A-finer case): scratch 8 × 8 tracts (size 2), desirability
    map 2 × 2 tracts (size 4) holding 0 → scratch tracts x < 4 and z < 4 are processed (value 0 → 1), every
    other scratch tract keeps its value (0 stays 0).
19. **LV6 ABI/base vector (native adapter acceptance).** QI gives L and M = L + 4; each successful
    query owns a separate reference. Read the pipeline dword at L + 0x38, not M + 0x38; forward
    slot 3 with M and the original message pointer exactly once. SimNewDay budget is 5;
    SimIdle data3 = 17 gives budget 17. The native AL result is returned unchanged. A rejected
    executable/interface identity produces no private-field read and no capture claim.
    A standalone QI proxy without interception receives no claim of native delivery coverage.
20. **LV6 tract interval vector.** Use inclusive bounds x=0..2, z=0..1, step=1 in state 1.
    Fresh (0,0) → yielding (1,0) attributes (0,0),(1,0); yielding (1,0) → yielding (0,1)
    attributes (2,0),(0,1). Yielding (0,1) → exhausted attributes (1,1),(2,1), excluding the
    already processed (0,1) and any stored out-of-bounds coordinate. Each of these is a
    separate delivery; state transitions do not execute next-state tracts in that delivery.
    Fresh → same fresh or yielding → same yielding attributes no new tract.
21. **LV6 retry vector.** The first state-3 scan finds no eligible zoned tract. After that
    delivery state is still 3, ignoreZoning changes false → true and the tract iterator is
    fresh at its origin. No all-land scan has yet occurred. Discard that attempt's statistics;
    the following scan supplies the all-land statistics. Do not treat the reset as wraparound
    forward progress or reuse the rejected scan's count/sum/histogram.
22. **LV6 developer interval vector.** One covered scratch cell starts at 100, total at 77;
    identity curve, two developer nodes in native address order A then B, weights 2.5 and 1.
    State 6 establishes cursor A without reading either map. A's state-7 pre-call map value
    is 40: scratch becomes 200, cursor B, state remains 7, total remains 77. A later publication
    of A=0 must not replace that input. B's state-7 pre-call map value is −3: scratch becomes
    197, cursor header, state remains 7, total remains 77. The next header delivery reads no
    developer, copies scratch to total (197), and advances to state 8. This tests the input
    record and attribution as well as the existing §7.4 arithmetic.
23. **LV6 empty/dead progress vector.** Empty developer map: state 6 selects header, then
    state 7 performs no developer callback, publishes scratch as total and enters state 8.
    State 9 deliveries perform no Step work. An outstanding L/M reference after Shutdown
    does not permit dereferencing the former iterator/grid storage; the trace ends before
    destruction. A queued LandValueMapChanged callback is not a new Step capture.
24. **LV6 loaded-map vector.** One developer, identity curve, weight 2.5, scratch=100.
    Restored D=40; developer publishes D=0 after land value's state-7 call but before pass
    completion. The state-7 record contains 40 and produces 200, while the later pass-end
    map contains 0 and would produce 100. Reverse that publication order (D=0 published
    before state 7): the record contains 0 and produces 100. Select by the actual read
    interval, never by matching the expected output or always preferring the restored map.
25. **LV6 provenance vector.** Exact outputs with only a pass-end sample still report
    `input_capture=pass-end-sample; native_step_capture=0`. A missed startup delivery,
    install/restore without an exclusion proof, or an observer that changes agent registration
    identity cannot upgrade that label. Full per-Step coverage requires the §6.1.3 record and
    acceptance evidence; these static vectors alone do not certify an installed adapter or
    settle the LV2/LV7 runtime incidents.

26. **LV8 lifecycle-only rejection vector.** With no host exclusion mechanism, receive
    synchronous pre-init, obtain QI, ordinary-pause the simulator and request installation.
    Expected: installation is ineligible; capture stays off. Replace ordinary pause with
    HiddenPause or a queued post-init callback: still ineligible. None drains affected
    callers. Native deliveries retain their original identity/order and occur once;
    no test may manufacture a Step to make this case pass.
27. **LV8 startup coverage vector.** Test new and loaded cities separately. With proven
    host exclusion, publish the validated global route before city Init; allow no private
    reads while Init is running, including after agent registration and before the final
    initialized store. After successful Init, the first real delivery has exactly one
    before/native/after record. A loaded city's first state-7 record uses its actual
    pre-call D (§16.24). Repeat with installation after the first delivery: startup
    completeness is false even if all later outputs match. An object-only installation
    demonstrated for the loaded city does not satisfy the new-city case.
28. **LV8 fetched-destination vector.** Hold caller A after it has obtained the old
    route's destination but before entering the observer; its observer-entry count is 0.
    Caller B requests restoration/reclamation. Expected: B cannot reclaim on that count.
    Admission exclusion must account for A and let it finish before mutation/retirement;
    while excluded, caller C cannot begin an affected call. A's native message is
    forwarded exactly once and its native bool is returned unchanged.
29. **LV8 retained-notification vector.** Message dispatch copies referenced targets A
    and B, unlocks, and calls A. A removes B's subscription. Expected native behavior:
    B is still in this dispatch's snapshot and is invoked once before snapshot cleanup.
    B's observer code/context remains valid until that invocation and its cleanup finish.
    With no admitted capture interval left, owner Shutdown may invalidate grids; a late
    uncaptured callback must not read the retired owner's progress/iterators/maps. The
    dispatch's reference alone does not make those inputs valid.
30. **LV8 shutdown/reuse vector.** Hold an admitted capture between its native return
    and after-copy. Request city shutdown. Under the host contract, complete the copy
    and end that interval before native Shutdown; no new owner reads are admitted.
    After Shutdown, keep an L/M reference and reuse the same numerical owner address
    for a later city: old identity/progress/maps must not be reused. For a global route
    kept across both cities, no per-city executable/table restoration is required, but
    its resources remain resident and the new city requires its own initialization
    and complete first-delivery coverage. Never block inside a callback waiting for
    that callback itself to finish.
31. **LV8 ownership/failure vector.** Under exclusion, make executable/interface validation
    fail: leave the native route intact, capture off, no private reads. In a separate
    case validation succeeds, then another installer replaces the route: restoration
    must not overwrite that replacement, and resource retirement still requires proof
    that no caller can reach the former route. Exact pass-end comparisons in either
    case retain `input_capture=pass-end-sample; native_step_capture=0`; these acceptance
    fixtures do not settle LV2/LV7 or count as real-game native capture evidence.

Vectors 19–31 are specified here for an engine/adapter lane to implement; this RE lane has
not added executable vectors or run native capture. They do not change vectors 1–18.
