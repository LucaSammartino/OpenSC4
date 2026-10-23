# Spec: the prop developer (`cSC4PropDeveloper`) and its requesters

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from observing the original program and its data files. Implementers must work from this spec, the
public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled or disassembled
code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (§2.7 references, §3.6 components and read-before-init, §6 random
numbers, §12 `rand()` and `random_shuffle`, §18 x87 precision), [simulator.md](simulator.md) (§2 levels, §4
`AddAgent`, §5.4 idle time), [simgrid.md](simgrid.md) (§7.4 scanning iterator), [lot-developer.md](lot-developer.md)
(§6.5 the lot's own props, §6.7.8 the animation-tick manager, §8 night lights, slot 63 `CreateLotProp`),
[lot-records.md](lot-records.md) (§10.1 and §10.2 the prop records, timer bits), [crime.md](crime.md) (§11.6 the
crime prop requester), [pollution.md](pollution.md) (§4.11 the garbage prop requester), [clock24.md](clock24.md)
(TimeTick, DayNightChanged, visual time mode), [power.md](power.md) (`IsPowered`, `0x8A07AE00`).

## 0. Summary

The prop developer places and removes **optional props** on lots and street cells, and drives the **timers** that
switch props on and off. Most of what it does is presentation (night lights, time-of-day props, signs), but two parts
change the saved city and must match the original:

- **Optional props.** Every lot configuration object of type prop with the *optional* flag is a *slot*. The prop
  developer visits the map one 8×8-cell **block** at a time. For each block it sums the occupancy of its lots, sets
  a target number of filled slots, splits that target between its **requesters** by their requested shares, and
  then removes at most one prop per over-served requester and creates props for under-served ones in random free
  slots. There are three requesters in every city: **wealth** (its own, props matching the lot's wealth), **crime**
  (crime.md §11.6) and **pollution** (garbage props, pollution.md §4.11).
- **The record**, with every registered prop list and the block scanner.

The rest is per-prop state: night-lit props follow dusk and dawn (in batches, shuffled), time-of-day props follow the
24-hour clock, dated props follow the calendar, power-dependent props follow the power grid, global-trigger props
follow the 3D view's effect triggers, lot-state props follow their lot's state, and signposts follow the terrain.

Implementation size: about 350 lines for the optional-prop algorithm and requesters, 300 for prop registration and
the timer handlers, 120 for the record; one member generator plus the wealth requester's file generator.

## 1. Identity

| Item | Value |
|---|---|
| Class ID (`GetGZCLSID`, `0x006F0D70`) | `0x89C48F47` |
| Component | city component **10**, IID `0xE9A1F4B6`; `cISC4City` slot 41 (`+0xA4`) `GetPropDeveloper`; global `0x00B43DB8` (set in `OnCityInit`, `0x00601C20`) |
| Factory | `0x006F3600`: global heap, **0xA70** bytes, constructor `0x006F3410` |
| Interface at +0 | `cISC4PropDeveloper` (SDK header), vtable `0x00AB29F8` (12 slots, §8.1) |
| Message target at +4 | vtable `0x00AB29E8`, `DoMessage` `0x006F4E90` |
| Serializable at +8 | vtable `0x00AB29D0`, `Write` `0x006F1930`, `Read` `0x006F3620` |

`QueryInterface` (`0x006F0D80`): `0xE9A1F4B6` and `1` → +0; `cIGZMessageTarget2` `0x452294AA` → +4;
`cIGZSerializable` `0xE4FDA3D4` → +8; otherwise false. Reference count at +0x10.

**Wealth requester** (`cSC4WealthRequester`, class `0x89C49986`): a 0xC-byte object (global heap) with the
requester vtable `0x00AB29B0` at +0, the serializable vtable `0x00AB2998` at +4 and a reference count at +8. Factory
`0x006F1500`, registered with the class store by the COM director (`0x00460425`), so a saved reference to it can be
resolved. `QueryInterface` (`0x006F0E30`): `1` and `cISC4PropRequester` `0x89C498BE` → +0, `0xE4FDA3D4` → +4. The
last `Release` (`0x006F1040`) deletes it.

## 2. State

Offsets are object offsets.

| Offset | Field | Initial | Saved |
|---|---|---|---|
| `+0x0C` | initialised (byte) | 0 | no |
| `+0x14` | **scan flag** (s8): 0 = never set up, 1 = set up, 2 = scanning | 0 | **yes** |
| `+0x18` | **block scanner**: scanning iterator (simgrid §7.4): state u8 at `+0x18`; x, z, step, clip x0, z0, x1, z1 (s32) at `+0x1C`…`+0x34` | state fresh at (0, 0), step 1, clip the full int32 range | **yes** |
| `+0x38` | member generator (`cRZRandomSuper`), seeded in the constructor (§5) | | no |
| `+0xA00` | `wealthAdded` (byte): the wealth requester has been created | 0 | **yes** |
| `+0xA04` | **requesters**: vector of `cISC4PropRequester*` (held) | empty | **yes** |
| `+0xA10`, `+0xA14`, `+0xA1C` | lot manager, prop manager, network lot manager (held at `Init`) | null | **yes** (refs) |
| `+0xA18` | power simulator (held at `Init`) | null | no |
| `+0xA20` | **night list**: vector of `cISC4PropOccupant*` (held) | empty | **yes** |
| `+0xA2C` | night cursor (u32) | 0 | **yes** |
| `+0xA30` | night mode (u32): 0 idle, 1 switching on, 2 switching off | 0 | **yes** |
| `+0xA34` | night batch size (u32) | **not initialised** *[quirk]* | **yes** |
| `+0xA38` | night tick accumulator, ms (u32) | 0 | **yes** |
| `+0xA3C` | `isNight` (byte) | 0 | **yes** |
| `+0xA40` | time-of-day threshold, hours (float) | 0.4 | no |
| `+0xA44` | last time-of-day update, hours (float) | −100.0 | no |
| `+0xA48` | **trigger map**: hash map (100 buckets) from effect trigger ID to a list of props (held) | empty | **yes** |
| `+0xA5C` | **dated list**: list of props (held) | empty | **yes** |
| `+0xA60` | **time-of-day list**: list of props (held) | empty | **yes** |
| `+0xA64` | **power list**: list of props (held) | empty | **yes** |
| `+0xA68` | `signsVisible` (byte) | 1 | **yes** (v5) |
| `+0xA6C` | **sign list**: list of props, **not** held | empty | **yes** (v5) |

pollution.md calls the requester vector "+0x9FC"; that offset is relative to the serializable sub-object (+8).

The field at `+0xA34` is never set by the constructor. The shipped city templates hold the heap fill value
`0xBAADF00D` there (§7). It is read only while the night mode is non-zero, and every assignment of a non-zero mode
also sets it, so the value is never used (PV1). `0xBAADF00D` is the Windows heap's fill for a fresh block when the
debug heap is active *[inferred]*, so a new city saved by the original holds whatever its heap block held: there is
no fixed original value to match, and writing 0 is safe.

## 3. Agents, messages and cadence

### 3.1 `Init` (slot 3, `0x006F1F30`) → true

1. Hold the lot manager, the prop manager, the network lot manager (`cISC4City` slot 42) and the power simulator.
2. If `wealthAdded = 0`: create the wealth requester, `AddPropRequester` it (§4.1), `wealthAdded := 1`. A loaded
   city has `wealthAdded = 1` and its requesters come back from the record (§7).
3. If the scan flag is 0: scan flag := 1; clip := (0, 0, ⌊cellsX / 8⌋ − 1, ⌊cellsZ / 8⌋ − 1) with x1 and z1
   raised to at least 0 (`cISC4City` slots 93 and 94, the cell counts); scanner reset to (0, 0) (done if outside
   the clip). Shipped city sizes give clip x1 = z1 = 7, 15 or 31.
4. `AddAgent(target +4, level 10 Idle, "cSC4PropDeveloper", flags 2)` and the same for **level 4 Day** (simulator.md
   §4). Flags 2: enabled only while the city is established.
5. Subscribe to `0x99EF1142`, `0x99EF1143`, `0xC9DA96E8`, `0xC9DA96E9`, `0x44778231`, `0x44778234`, `0x26D31EC1`,
   `0xE8BDA2E7`, `0x8A07AE00`, `0xAA375FE8`, `0xCA5E5027`, `0x278E6003`, `0x678E6E3A`.
6. Register with the animation-tick manager at priority `0xC0` (lot-developer.md §6.7.8: ticked every frame).

### 3.2 `Shutdown` (slot 4, `0x006F2C60`) → true

Release every prop in the trigger map, the time-of-day list and the dated list, and clear them. Release every prop
in the power list **without clearing it** *[quirk]*: the list keeps dangling pointers until the object is destroyed.
*[choice]* FreeCity4 clears it. Release and clear the night list. Remove the agents, unsubscribe everything,
unregister from the animation-tick manager, set the scan flag to 0, release the managers. If `wealthAdded`: release
and clear all requesters, `wealthAdded := 0`.

### 3.3 Messages (`DoMessage`, sub-object +4; every handler returns true unless stated)

| Message | Handling |
|---|---|
| `0x6695681A` Idle (agent) | **block step** (§4.2). The message data is ignored |
| `0x66956814` Day (agent) | block step; then `date := simulator.GetSimDate()` and `TickSimulatorDate(date)` (prop slot 16) for every prop of the dated list |
| `0x86AD10EE` animation tick | night mode 0 → return **false**. Otherwise the night batch of §4.8 with `ms := data2 / 1000` (unsigned) |
| `0xC9DA96E9` | dusk: night mode := 1, `isNight := 1`, start a sweep (§4.8) |
| `0xC9DA96E8` | dawn: night mode := 2, `isNight := 0`, start a sweep (§4.8) |
| `0x44778234` DayNightChanged | if the 24-hour clock exists: data2 = 0 (night begins) and visual time mode = 2 → switch all night props on (§4.8); data2 = 1 (day begins) and mode = 1 → switch all off. Otherwise nothing |
| `0x26D31EC1` PostCityInit | `isNight := lightingManager.IsNight()` (slot 9 `+0x24`); switch all night props to `isNight` |
| `0x44778231` TimeTick | time-of-day update (§4.9) |
| `0x99EF1142` occupant inserted | data3 = `0xA823821E` (prop): register the prop (§4.6) |
| `0x99EF1143` occupant removed | data3 = `0xA823821E`: unregister the prop (§4.7). data3 = `0xC772BF98` (network): remove street-name signs on that cell (§4.11) |
| `0xAA375FE8` global effect trigger | data1 = trigger, data2 = on: §4.10 |
| `0xE8BDA2E7` lot state changed | §4.12 |
| `0xCA5E5027` lot-state props | own message, data1 = x, data2 = z: §4.12 |
| `0x8A07AE00` power changed | rect (data1 x1, data2 z1, data3 x2, data4 z2): §4.13 |
| `0x278E6003` terrain heights changed, `0x678E6E3A` water heights changed | re-seat signposts (§4.11) |

The lighting manager (global `0x00B43DDC`) posts `0xC9DA96E9` when its light level falls below its night threshold
and `0xC9DA96E8` when it rises above it (`SetTimeOfDay`, `0x007D9F0D`–`0x007D9F20`); its slot 9 reports which side
it is on *[inferred: name IsNight]*. In the forced visual time modes (1 always day, 2 always night) the
DayNightChanged handler also switches everything at once.

**Cadence of the block step.** One block per Idle delivery, plus one per sim day. Idle is delivered from the
leftover frame time (simulator.md §5.4), also while the game is paused, so the number of blocks per sim day depends
on the frame rate. *[choice]* FreeCity4 delivers Idle as its simulator does; the **order** of blocks and every
outcome for a given sequence of block steps are *[compat]*.

## 4. Behaviour

### 4.1 Requesters

`AddPropRequester(r)` (slot 5, `0x006F21C0`): AddRef and append, **with no duplicate check**.
`RemovePropRequester(r)` (slot 6, `0x006F1080`): find the first occurrence, erase it and Release; absent → nothing.

A requester's **index** is its position in the vector. Every city in the corpus has index 0 = wealth, 1 = crime,
2 = pollution (the order of their `Init` calls). Props carry their requester's index in property `0x89A1C16C`
(§4.4), and the property is saved with the prop (lot-records.md §10.1), so the order is *[compat]*.

The requester interface (`cISC4PropRequester`, IID `0x89C498BE`, no SDK header; crime.md §11.6):

| Slot | Method | Wealth | Crime | Pollution |
|---|---|---|---|---|
| 3 | `GetRequestedShare(rect)` → int32 | **10**, always (`0x006F0E10`) | 0…10 from the average crime (crime.md §11.6) | from the garbage average (pollution.md §4.11) |
| 4 | `GetRequesterSatisfactionId()` → u32 | `0x49A1E995` | `0x49A1E996` | `0xE9A4300B` |
| 5 | `UsesCustomPropSelector()` → bool | true | true | false |
| 6 | `SelectProp(ids, x, z)` → prop ID | below | crime.md §11.6 | 0 |

A prop **serves** a requester when the prop exemplar's requester-satisfaction property `0x49A1E05A` (prop profile
field, default 0) equals the requester's satisfaction ID.

**Wealth `SelectProp(ids, x, z)`** (`0x006F1CB0`, pop 12):
1. `lot := lotManager.GetLot(x, z, false)` (any cell of a lot).
2. With a lot: `w := lot.GetOccupantWealth()` (lot slot 41); if the zone type at `(x, z)` (zone manager slot 12) is
   11 (airport), 12 (seaport) or 15 (plopped), `w := 2`. Without a lot: `w := networkLotManager.GetWealthForNetwork(x,
   z)` (slot 10).
3. Keep the IDs, in order, whose prop profile wealth byte (exemplar property `0xE9A316EB`, default 0) equals `w`.
4. None → 0. Otherwise `kept[RandomUint32Uniform(n)]` from the **wealth requester's file generator** (§5).

The wealth requester's `Write` and `Read` return true and transfer nothing.

### 4.2 Block step (`0x006F4E10`)

- Scan flag 1: set it to 2 and stop. The first step after setting up (or after loading a city that was never
  scanned) does nothing.
- Scan flag 2: `GetNext` on the scanner. If it returns false (the scan is finished), reset it to (0, 0) (done if
  (0, 0) is outside the clip) and `GetNext` again. Then **update the block** `(bx, bz)` just returned (§4.3).
- Any other scan flag: nothing.

Blocks are visited with x inner and z outer: (0, 0), (1, 0), …, (x1, 0), (0, 1), … After (x1, z1) the next step
restarts at (0, 0) at once.

`UpdateAllOptionalProps` (slot 7, `0x006F4DB0`) updates every block of the clip with **x outer, z inner**. No caller
exists in the game: no direct call targets it, and it is reachable only through slot 7 of the vtable (PV3).

### 4.3 Block update (`UpdateTract`, `0x006F4080`) *[compat]*

The block covers cells `x ∈ [8bx, 8bx + 7]`, `z ∈ [8bz, 8bz + 7]`. Let `R` be the number of requesters.

**Step 1, shares.** `share[r] := requester[r].GetRequestedShare(block rect)` for every r, with the rect `(8bx, 8bz,
8bx + 7, 8bz + 7)`; `total := Σ share[r]` (u32).

**Step 2, cell walk.** Counters for the block: `lots`, `occSum` (float32), `slots`, `keptAll` (all 0); per
requester `kept[r]` (0) and a kept list; one *free* list. For each cell, **x outer, z inner**:

1. `lot := lotManager.GetLot(x, z, true)`, which answers only at a lot's origin cell.
   - A lot is used only if `IsInLivableState()` (slot 37) and `GetBuildingType(false)` (slot 46) is non-zero;
     otherwise the cell is skipped. Then `lots += 1` and `occSum := f32(occSum + lot.GetOccupancy())` (slot 61,
     compiled with a hidden result pointer; lot-developer.md §11.4). Then `cfg := lot.GetLotConfiguration()` (slot 9);
     none → skip the cell (the lot still counts).
   - No lot: `netLot := networkLotManager.GetNetworkLot(x, z)` (slot 5); none → skip. Its configuration objects
     come from its slot 5.
2. `objs :=` the configuration's object array (0x34-byte records, SDK `cSC4LotConfigurationObject`). Object `i` is
   **open** when its type byte is 1 (prop) and its flag byte has bit 0 set (optional). `opt :=` the number of open
   objects; `k := 0`.
3. `props := lot ? propManager.GetLotProps(lot) : propManager.GetStreetProps(x, z)` (prop manager slots 24 and 31).
   For each prop in list order:
   1. No lot tag (occupant slot 27 false) → next prop.
   2. `i :=` the first object whose object ID (record +0x24) equals the tag. None → **`k := k − 1`** *[quirk]*, next.
   3. Object `i` not open (not optional, or already taken by an earlier prop) → next.
   4. Close object `i`; `opt := opt − 1`.
   5. No `0x89A1C16C` property on the prop → next (the slot stays closed and the prop is not counted).
   6. `r :=` the property value. *[quirk]* The original does not check `r < R`. *[choice]* FreeCity4 treats
      `r ≥ R` like step 5.
   7. If requester r has no custom selector, **keep** the prop. Otherwise ask `requester[r].SelectProp([type], x, z)`
      with a one-element list holding the prop's type (prop slot 7); keep it if the answer equals the type. A
      custom selector check therefore draws from that requester's generator (§5).
   8. Keep: append `(x, z, objs, i, prop)` to kept list r (AddRef), `kept[r] += 1`, `k := k + 1`.
   9. Not kept: `propManager.RemovePropA(prop)` (slot 29), reopen object `i`, `opt := opt + 1`.
4. Append one free entry `(x, z, objs, i)` for every object that is still open, in ascending `i`.
5. `slots := slots + opt + k`; `keptAll := keptAll + k`.

**Step 3.** If `lots = 0`, stop. Nothing has been drawn after the cell walk, and nothing is created or removed.

**Step 4, target.** `N := ⌊f32(f32(occSum / lots) × slots)⌋` (24-bit precision, foundations §18; the original floors
with the 1.5·2²³ idiom, exact here). Split it: `q := N div total`, `m := N mod total` (unsigned).
`want[r] := share[r] × q`. Then hand out `m` single units: start with `idx := 0`; for each unit advance
`idx := (idx = R − 1) ? 0 : idx + 1` until `share[idx] ≠ 0`, then `share[idx] −= 1`, `want[idx] += 1`. The first unit
therefore goes to index 1 if its share is non-zero. The number of units handed out is `min(m, total)`
(`0x006F4A94`–`0x006F4AA6`; always `m` since `m < total`). *[quirk]* `total = 0` is an unsigned integer division by
zero (`div` at `0x006F4A69`), a processor fault that ends the original; the wealth share of 10 makes it impossible
while the wealth requester is listed. It can happen only after a `Read` of a record without it (PV10). *[choice]*
FreeCity4 stops after the cell walk (everything of step 2, including its selector draws and removals, has
happened; nothing of steps 4–6).

**Step 5, removals.** For r = 0 … R − 1: if `kept[r] > want[r]`: `j := RandomSint32RangeUniform(0, kept[r])` from
the **member generator**; the j-th entry of kept list r is removed (`RemovePropA`, Release), and its slot is appended
to the free list; `slots := slots + 1`. At most **one** removal per requester per block update. `kept[r]` is not
changed.

**Step 6, creations.** `F := slots − keptAll` (this equals the length of the free list). For r = 0 … R − 1 with
`want[r] > kept[r]` (unsigned): `need := want[r] − kept[r]`; while `need ≠ 0` and `F > 0`:
`j := RandomSint32RangeUniform(0, F)` (member generator); if **select and create** (§4.4) succeeds on free entry j
for requester r, `need −= 1`; erase entry j (whether or not it succeeded); `F −= 1`.

So a free slot that cannot hold a prop for requester r is used up for the rest of this block update, and requesters
with lower indexes get first pick of the free slots.

### 4.4 Select and create (`0x006F1620`, pop 8, AL)

For free entry `(x, z, objs, i)` and requester r:

1. `ids :=` object i's ID list (record +0x28: prop IDs or prop family IDs). Empty → false.
2. `sat := requester[r].GetRequesterSatisfactionId()`.
3. Candidates, in order: for each ID, if `propManager.IsPropTypeValid(id)` (slot 8), the ID itself when its profile
   (slot 13) has satisfaction `sat`; otherwise every member of `propManager.GetPropTypesByFamily(id)` (slot 9, in the
   order it returns them) whose profile has satisfaction `sat`. Duplicates are kept.
4. No candidate → false. No custom selector → `pick := candidates[RandomUint32Uniform(n)]` (member generator).
   Custom → `pick := requester[r].SelectProp(candidates, x, z)`. `pick = 0` → false.
5. Create: if `lotManager.GetLot(x, z, false)` exists, `occ := lotDeveloper.CreateLotProp(lot, i, pick)` (lot
   developer slot 63: places the prop at the object's position and tags it with the object ID). Otherwise, if
   `networkLotManager.GetNetworkLot(x, z)` exists, `occ := networkLotManager.CreateProp(netLot, i, pick)` (slot 8).
   Otherwise the original uses the free entry itself as the occupant *[quirk; unreachable: the entry came from a lot or
   network lot at that cell in the same call]*; *[choice]* FreeCity4 returns false.
6. If `occ` is null, return false (`0x006F18C6`; correction: the original does test it, PV8). Otherwise set the u32
   property `0x89A1C16C := r` on `occ` (held for the call); return true. A false result still uses up the free
   entry and leaves `need` unchanged (§4.3 step 6).

The new prop announces itself with `0x99EF1142` and is registered by §4.6 like any other prop.

### 4.5 Worked examples

**A. First visit.** A block holds one livable lot with occupancy 0.5 and four optional prop objects, no props yet.
Shares: wealth 10, crime 0, pollution 0 (total 10).

- Cell walk: lots 1, occSum 0.5, opt 4, k 0 → slots 4, keptAll 0; free = the four objects.
- N = ⌊0.5 × 4⌋ = 2; q = 0, m = 2; want = (0, 0, 0). Unit 1: idx 1 (share 0), 2 (0), 0 (10) → want[0] = 1,
  share[0] = 9. Unit 2: idx 1, 2, 0 → want[0] = 2. want = (2, 0, 0).
- No removals. F = 4. Requester 0 needs 2: `RS(0, 4)` picks a free entry; select and create filters its IDs to
  wealth props and asks the wealth requester (one draw from its file generator when a prop of the lot's wealth
  exists); success → need 1. `RS(0, 3)` → second prop. Member-generator draws: 2.

**B. Second visit, no change.** Kept: two wealth props whose wealth still matches (each check costs one wealth draw).
slots = opt 2 + k 2 = 4, keptAll 2 → N = 2, want = (2, 0, 0), nothing removed or created. F = 2 but no requester
needs anything, so no member draw.

**C. Occupancy falls to 0.25.** The two wealth props are kept (two wealth draws): slots 4, keptAll 2. N = 1,
want = (1, 0, 0), kept[0] = 2 → one removal with `RS(0, 2)`, slots 5. F = 5 − 2 = 3, but no requester is
under-served, so nothing is created.

**D. Split.** Shares (5, 3, 2), N = 13: q = 1, m = 3, want = (5, 3, 2); units go to 1, 2, 0 → want = (6, 4, 3).

**E. A lot's wealth changes.** A medium-wealth lot becomes high-wealth: on the next visit each wealth prop's check
returns 0 (no candidate, no draw), so every one is removed at once and its slot reopened; creations then fill the
slots with high-wealth props.

### 4.6 Registering a prop (`HandleInsertOccupant`)

Props with the preview marker `0x029244DB` are ignored. Otherwise, with `p` the prop occupant (IID `0xE9793A65`):

1. **Night-lit** (u8 property `0x49C9C93C` = 1): append to the night list (held). If `p.GetState() ≠ isNight`: by
   day `SetState(0)`; at night `SetState(1)` only if the prop's cell is powered (`power.IsPowered(⌊x / 16⌋, ⌊z / 16⌋)`,
   from the prop's position), else unchanged.
2. **Timed** (any of `0x6A3FCC37`, `0x4A149631`, `0x0A751675`, `0x6A959E71` present): `SetState(1)`;
   `SetRandomChance(u8 0x4A751AD5, default 100)`. Then the four timer bits (prop slot 21
   `SetPropTimerState(mask, on)`; lot-records.md §10.1 field 13):
   - mask 4, **global trigger** `0x6A3FCC37` (u32): non-zero and the 3D view is available (QueryInterface
     `0xFA47B3F9`) → add to `triggerMap[trigger]` (held) and set mask 4 to the view's current state of that trigger
     (view slot 46). Zero or absent → mask 4 on. Non-zero with no 3D view (global `0x00B43DC8` null, or the
     QueryInterface fails): **nothing**: mask 4 is left as it is and the prop joins no map (`0x006F3D24`,
     `0x006F3D3A` both jump past the step; PV9).
   - mask 1, **time of day** `0x4A149631` (float32 pair, hours): absent → mask 1 on. At least two values with
     start ≠ end → `SetTimeOfDayRange(start, end)` and append to the time-of-day list (held). Otherwise unchanged.
   - mask 2, **date** `0x0A751675` (u32): absent → mask 2 on. Present → `SetSimulatorDateRange(value, u32
     0x4A764564 (default 1), u8 0xCA7515CC[0] (default 0), 0)` and append to the dated list (held).
   - mask 8, **power** `0x6A959E71` (bool): false or absent → mask 8 on. True → mask 8 := powered at the prop's
     cell, and append to the power list (held).
3. **Signs**: prop type `0xAB72FBB3` (signpost) or `0xCB79919B` (street-name sign) → append to the sign list (not
   held).

A prop shows only when all four timer bits are set *[inferred]*.

### 4.7 Unregistering a prop (`HandleRemoveOccupant`)

The mirror of §4.6 (`0x006F2FF0`), with the preview-marker test first. Each list is searched **only when the prop
meets that list's test** (PV12) *[compat]*:

| List | Test before the lookup | Removal |
|---|---|---|
| night list | u8 `0x49C9C93C` present and = 1, and the prop answers `0xE9793A65` | erase (Release) |
| trigger map | u32 `0x6A3FCC37` present and non-zero, the 3D view present and answering `0xFA47B3F9` | erase from `triggerMap[trigger]` (Release); an emptied entry is erased |
| time-of-day list | `0x4A149631` **present** (value not read) | the first matching entry takes the last entry's value and the last node is popped (Release) |
| dated list | `0x0A751675` present | the same |
| power list | `0x6A959E71` present (its value not read) | the same |
| sign list | type `0xAB72FBB3` or `0xCB79919B` | erase (no Release) |

The tests are looser than §4.6's join conditions (the time-of-day test ignores the value count and start ≠ end, the
power test ignores the bool), so a prop that never joined is looked up and not found, which is harmless. The
observable difference from "look everywhere": a prop that joined the trigger map while the 3D view existed is **not**
removed from it when the view is gone at its removal, and stays held *[quirk]*.

### 4.8 Night lights

**Sweep start** (`0xC9DA96E9` / `0xC9DA96E8`): set the mode and `isNight` (§3.3); `random_shuffle` the night list
(C `rand()`, n − 1 calls, foundations §12.3); `batch := ⌈n / 8⌉`; cursor := 0.

**Batch** (animation tick, mode ≠ 0; `0x006F0EC0`): `acc := acc + ms`; if `acc ≤ 200` stop; `acc := 0`.
`on := (mode = 1)`, read **before** the end test (`0x006F0EF6`–`0x006F0F04`). `end := cursor + batch`; if `end > n`
(signed): `end := n` and mode := 0 (`0x006F0F53`). For each cursor value below `end`: if `on`, `SetState(powered at
the prop's cell)`; otherwise `SetState(0)`; advance the cursor. So the batch that ends a dusk sweep still switches
its props on (PV7). The cell is
`(trunc(x / trunc(cellWidthX)), trunc(z / trunc(cellWidthZ)))` (`cISC4City` slots 91 and 92).

So a sweep takes 8 batches of more than 200 ms each (fewer if props were removed). With n = 0 the batch is 0, the
test `0 > 0` fails, and the mode stays non-zero until the next switch *[quirk]*; this is why 183 of the 421 saves
hold mode 2.

**Switch all** (`0x006F10E0`, from PostCityInit and DayNightChanged): for each entry of the night list (null
entries are erased): `SetState(on and powered at the prop's cell)`, the cell from position × (1 / cell width),
truncated. Then cursor := 0 and mode := 0.

Example: 20 night props at dusk → 19 `rand()` calls, batch 3; batches cover 0–2, 3–5, …, 15–17; the seventh batch
covers 18–19 and ends the sweep (mode 0, cursor 20).

### 4.9 Time of day (`0x44778231`)

`t := f32(hour + f32(minute × (1/60)f))` from the 24-hour clock (`GetHour`, `GetMinute`). If `|last − t| < 0.4`
(or NaN), stop. Otherwise `last := t` and, for each prop of the time-of-day list with `(start, end) :=
GetTimeOfDayRange()`: on when `start ≤ t ≤ end` if `end ≥ start`, and when `t ≥ start or t ≤ end` if `end < start`;
`SetPropTimerState(1, on)`.

A null entry is replaced by the last element, which is popped, and the same position is examined again. *[quirk]*
If the null entry is itself the last element, the original continues from the freed node. No save holds a null
reference (§7); *[choice]* FreeCity4 stops at the end of the list.

`last` is not saved: it is −100 after loading, so the first tick updates every prop. Example: a prop with range
(18.0, 8.0) is on at 07:30 (7.5) and off at 12:00; ticks at 07:00 and 07:20 (Δ 0.33) update nothing in between.

### 4.10 Global effect triggers (`0xAA375FE8`)

For every prop in `triggerMap[data1]` (null entries erased): `SetPropTimerState(4, data2 ≠ 0)`. The 3D view **sends** (not
posts) the message from `AssertGlobalEffectTrigger` / `RetractGlobalEffectTrigger` (view3d-window.md §6k). Three saves hold trigger `0xCACBDFC6`.

### 4.11 Signs

- `ShowSignsAndLabels(show)` (slot 8, `0x006F1210`): if `show = signsVisible`, nothing. Otherwise store it and, for
  every sign, set (show) or clear (hide) occupant flag 2 (occupant slots 29 `SetFlag` / 31 `ClearFlag`).
  `ToggleSignsAndLabels` (slot 9) calls slot 8 with the opposite value; `AreSignsAndLabelsVisible` (slot 10) returns
  `signsVisible`.
- A removed network occupant: every street-name sign (type `0xCB79919B`) whose cell (`cISC4City` slot 95
  `PositionToCell`) equals the first cell of the network occupant's bounding cells is removed (`RemovePropA`).
- Terrain or water changed: for every signpost (type `0xAB72FBB3`), `y := max(terrain.GetAltitude(x, z),
  terrain.GetWaterLevel(x, z))` (terrain slots 13 and 30, float overloads); if `|pos.y − y| > 0.01`, move it there
  and notify the occupant manager (slot 24 `+0x60`, flag 1). The message rectangle is ignored: every signpost is
  checked.

### 4.12 Lot state

**`0xE8BDA2E7`** (data1 old state, data2 new state): the message's IGZUnknown is used as the lot **without a
QueryInterface** *[quirk]*. `(x, z) := lot.GetLocation()`; `L2 := lotManager.GetLot(x, z, true)`; continue only if
`L2` exists with the same size (slot 22). If the message lot is not livable, remove the optional props of `L2`
(every lot prop whose tag's first matching configuration object has the optional flag bit, `RemovePropA`; the
type byte is not checked). Then post
`0xCA5E5027` with data1 = x, data2 = z, data3 = (old = 2 and new = 3) (data3 is not read).

**`0xCA5E5027`**: `lot := lotManager.GetLot(data1, data2, false)`; none → nothing. For every lot prop with u8
property `0x6A5E0727`: `SetState(lot.GetState() ≠ value)` (state 0 while the lot is in the named state, 1
otherwise).

### 4.13 Power changed (`0x8A07AE00`)

1. Street props in the rectangle (prop manager slot 30) with `0x49C9C93C = 1`: unpowered and in state 1 →
   `SetState(0)`; powered, `isNight` and in state 0 → `SetState(1)`. "Powered" is `power.IsPowered(trunc(x /
   trunc(cellWidthX)), trunc(z / trunc(cellWidthZ)))` from the prop's position, the cell of the night batch (§4.8;
   `0x006F2491`–`0x006F25EE`, PV11). Night-lit **lot** props are not touched here
   *[quirk]*: they change only at the next sweep or switch.
2. Power list (null entries replaced by the last element and popped): props whose cell (`trunc(pos × 0.0625)`) lies
   in the rectangle get `SetPropTimerState(8, IsPowered(cell))`.

## 5. Random numbers *[compat]*

| Draw | Generator | Method | When |
|---|---|---|---|
| removal index | member (`+0x38`) | `RandomSint32RangeUniform(0, kept[r])` | §4.3 step 5, once per over-served requester |
| free-slot index | member | `RandomSint32RangeUniform(0, F)` | §4.3 step 6, per creation attempt |
| non-custom pick | member | `RandomUint32Uniform(n)` | §4.4 step 4 (pollution) |
| wealth pick | wealth requester's file generator `0x00B4A5C0` (`cRZRandomSuper`, K `0x4C09C51F`, foundations §6.3) | `RandomUint32Uniform(n)` | every wealth `SelectProp` with candidates: kept checks (§4.3 step 2) and creations |
| crime pick | crime's file generator | crime.md §11.6 | every crime `SelectProp` |
| night shuffle | C `rand()` | `random_shuffle`, n − 1 calls | each sweep start (§4.8) |

Order inside one block update: the selector checks of the cell walk (cell order, then list order), then the
removals (requester order), then the creations (requester order; per attempt the free-slot draw before the pick).

The member generator is seeded **in the constructor** (`0x006F34A3`–`0x006F34B1`) with the clock value × 28
(mod 2³²); foundations §6.2 lists it under "at Init". FreeCity4 seeds it with `derive(0x89C48F47) × 28` (foundations
§6.3).

### 5.1 Live random-state observation (PV14, answered 2026-10-23)

The save record is insufficient for a same-input shadow: it contains neither the member generator, the
process-wide wealth generator, nor the calling thread's C runtime generator. A fixed shadow seed is therefore
not a reconstruction of the original's current inputs. There is no serializable RNG interface to call here.
An observer may use the read-only snapshots below, or the return-value transcript in §5.3. These are observation
contracts *[choice]*, not new game interfaces or new record fields. Neither requires reseeding the original or
calling a requester a second time.

**Build guard.** These offsets and addresses apply to the 32-bit GOG 1.1.641 executable studied here
(SHA-256 `3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`). Addresses below use preferred
image base `0x00400000`; translate them by the actual module-base displacement. Verify executable identity
before interpreting memory. If identity, expected interface pointers or snapshot invariants fail, report the
observation as unavailable; never try a guessed layout. The prop developer base is its primary
`cISC4PropDeveloper*`, with vtable `0x00AB29F8`, not its message-target pointer (+4) or serializable pointer (+8).
The constructor locates the member at primary base +`0x38` (`0x006F34AE`–`0x006F34B1`).

For a live, fully constructed `cRZRandomSuper` at address G:

| Offset from G | Width | Meaning for an independent snapshot |
|---|---|---|
| `+0x000` through `+0x9BC` | 624 little-endian u32 words | Complete current, untempered table, in order |
| `+0x9C0` | 32-bit native pointer P | Next table word; export as index `k = (P − G) / 4` |
| `+0x9C4` | signed 32-bit count r | Number of unread words, before the next draw |

The object occupies `0x9C8` bytes and has no leading vtable. At a completed-call boundary require `0 ≤ r ≤ 624`,
aligned P in the **inclusive** range `[G, G + 0x9C0]`, and `k + r = 624`. The endpoint `k = 624, r = 0` is valid:
the pointer is one past the table and must not be dereferenced. Reject a partially constructed object or a
snapshot taken during a draw/reload; negative r is not a valid quiescent snapshot. Copy all 624 words plus k and r
into shadow-owned storage. A shadow using pointers must reconstruct P from **its own** table base and k;
it must never retain the original's pointer. No native generator method is invoked to copy the state.

The next draw consumes table[k], advances k by one and decreases r by one when r is positive. With r = 0,
regenerate the table first, reset k = 0 and r = 624, then consume its first word. Regeneration and tempering
are the `cRZRandomSuper` rules of foundations §6.1, with 32-bit wrapping arithmetic and logical shifts. This is
not a seed import: do not seed, pre-draw, or regenerate an already readable table. Evidence: the constructor
`0x00913B7B`–`0x00913B8C`, reload tail `0x00913961`–`0x0091396B`, and word draw
`0x009139BD`–`0x00913A02`. The bounded draw uses the high 32 bits of an **unsigned** 64-bit product
(`0x00913A06`–`0x00913A11`); the signed range method subtracts the endpoints and adds the lower endpoint
(`0x00913A14`–`0x00913A26`). Even a bound of 1 consumes a word.

Snapshot sources and lifetimes:

| Source | G / state location | When the copy is valid |
|---|---|---|
| Prop developer member | primary developer base +`0x38` | Before the observed operation, outside any active member draw |
| Wealth selector | module address `0x00B4A5C0` | Before the corresponding real selector call, or before an operation with all intervening uses accounted for |
| Other custom selector | No generic RNG layout promised | Capture the actual selector return (§5.3); do not infer its generator from its satisfaction ID |
| Night shuffle C runtime state | Calling thread's runtime data +`0x14` (u32) | On that same thread, immediately before the shuffle's first draw, or before each actual draw |

Wealth selection draws from the file generator only after wealth filtering leaves a nonempty list
(`0x006F1DB9`–`0x006F1DD2`); an empty filtered list returns zero without a draw. It is process-wide, not owned by
the requester object and not reset by loading a city. A member snapshot can continue across operations only if
every member draw was simulated; a wealth snapshot can continue only if every intervening use of that shared
generator was accounted for. Otherwise take a fresh copy at the next observation boundary. Do not publish a
shadow's resulting state back into either native generator.

### 5.2 The thread-local C runtime state

`rand` is `0x009EFB9F`; its genuine call to `__getptd` returns the calling thread's runtime-data pointer in EAX.
At `0x009EFBA4`, before the state read, EAX +`0x14` holds the pre-draw state. An observer may read that word at
this point in the **original call**, then let the call finish once and record its EAX result. This provides a
concrete snapshot route without calling `__getptd` itself: that routine can allocate and initialize thread data
when absent (`0x009F761C`–`0x009F765D`), so it is not a read-only accessor. A pointer observed on one thread must
not be reused for another thread or after that thread exits. Never copy state from the plugin's own CRT.

In independent storage use foundations §12.1: the next state is `(s × 214013 + 2531011) mod 2^32` and the
result is `(nextState >> 16) & 0x7FFF` (`0x009EFBA4`–`0x009EFBBB`); `srand` writes the same +`0x14` field
(`0x009EFB92`–`0x009EFB9E`). Copying only the 15-bit result does not reconstruct the 32-bit state. A night
shuffle takes n − 1 draws for n > 1, visiting indexes 1 through n − 1 and swapping each with `rand() mod (i+1)`
(call at `0x006F53A6`, shuffle `0x006DB010`–`0x006DB058`). With n ≤ 1 it draws nothing; no runtime-state
snapshot is needed. Preserve the input list order and duplicate entries.

All other callers on the same thread share this state, including callers outside the simulation. A snapshot
at the previous dusk is not a valid starting state for the next dawn unless those intervening uses were also
observed. An operation-local snapshot suffices only when no unaccounted same-thread draw occurs inside it.
If an observer cannot establish that, capture the individual real draw results instead (§5.3). Reading while
the original is running concurrently is not a coherent snapshot: obtain a quiescent boundary on the invoking
thread, or use observation that suspends execution while copying. The observer itself must not draw, seed,
invoke native getters with side effects, or dispatch messages into the simulation.

### 5.3 Return-value transcripts and selectors

A documented alternative to cloning any generator is to record the random inputs actually consumed by the
original operation, then replay them into an independent shadow. Each event belongs to an operation sequence,
thread and nested call depth. Record the source identity (member address, shared generator address or thread's
CRT), the operation (raw/bounded/range/shuffle draw), bounds where applicable, and the actual return value.
Capture either a raw draw and apply its recorded bound in the shadow, **or** the bounded/range result; do not
consume both the wrapper result and its nested raw result as two draws. Event type, bounds, order and final
event count must match. A missing, extra or mismatched event makes the comparison invalid; never silently
fall back to a seed. Instrumentation must preserve the original call's arguments, return, registers and effects.

For custom selectors, capture the original `SelectProp` call and its return, including zero. Record requester
identity/index, call purpose (kept check or creation), ordered candidate IDs including duplicates, x and z, and
the u32 returned ID. The ABI is §8.3 slot 6: thiscall, requester in ECX, three arguments (vector reference, s32 x,
s32 z), callee pops 12 bytes, ID in EAX. The referenced vector is three 32-bit pointers: begin, end, storage-end;
copy the `[begin,end)` u32 IDs at entry, not the vector's stack address for later use. In a kept check the list
has just the current type; the call at `0x006F4677` returns to `0x006F467A`, where the ID is compared against that
type. In a creation the call at `0x006F1804` returns to `0x006F1807`; the preceding coordinate loads are unsigned
bytes (`0x006F17F4`–`0x006F17F8`), and zero means no creation. Capture the actual arguments rather than
reconstructing coordinates. This covers wealth, crime and plugin requesters without guessing their private state.

The shadow supplies the captured result only to the matching call; it must not call the live requester again,
substitute a candidate ID, clamp a result into the candidate list, or suppress a zero return. Recording an
already executed selector's return leaves its random draws and other side effects solely in the original.
If the shadow separately implements that selector, it may compare its result using an independent pre-call RNG
copy and the same environmental inputs; it must not both simulate its internal draws and consume them again
as developer-level transcript events. Reentrant events need explicit nesting, not one global FIFO.

An operation-level comparison requires all nonrandom inputs too (including ordered lists, requester replies
and creation success/failure). A matching saved record or matching list counts alone does not verify random
removal identities, selected types or creation attempts. Report separately what was compared. Resynchronization
after a mismatch does not turn that operation into a pass. An exact-through-date claim requires a complete
matched event stream and all claimed outputs throughout that interval; these RE findings alone make no such claim.

## 6. Tuning and data

No tuning exemplar. Data read:

| Source | Property | Use |
|---|---|---|
| lot configuration object | type byte 1, flag bit 0, object ID (+0x24), ID list (+0x28) | slots (§4.3), candidates (§4.4) |
| prop profile (prop manager) | `0x49A1E05A` requester satisfaction (u32); `0xE9A316EB` wealth (u8); `0x8AA864D5` crime incident | §4.1, §4.4 |
| prop occupant | `0x89A1C16C` requester index (u32, written here) | §4.3 |
| prop exemplar | `0x49C9C93C` night-lit (u8 = 1); `0x6A3FCC37` trigger; `0x4A149631` time of day (float ×2); `0x0A751675`, `0x4A764564`, `0xCA7515CC` date range; `0x6A959E71` needs power; `0x4A751AD5` chance; `0x6A5E0727` lot state; `0x029244DB` preview marker | §4.6, §4.12 |
| prop types | `0xAB72FBB3` signpost, `0xCB79919B` street-name sign | §4.11 |

Hard-coded: block size 8, requested share 10 (wealth), wealth 2 for zone types 11, 12 and 15, 200 ms batch
interval, 8 batches per sweep, 0.4 h time-of-day threshold, 0.01 signpost tolerance.

## 7. Persistence *[compat]*

Record class `0x89C48F47`. `Write` (`0x006F1930`, sub-object +8) always writes version 5:

| # | Field | Encoding |
|---|---|---|
| 1 | version | u16 = **5** |
| 2 | scan flag | s8 |
| 3 | scanner | u8 state (0 fresh, 1 running, 2 done); s32 x, z, step, x0, z0, x1, z1 |
| 4 | `wealthAdded` | u8 |
| 5 | requesters | u32 n; n refs (foundations §2.7) |
| 6 | lot manager, prop manager, network lot manager | 3 refs |
| 7 | night cursor, night mode, night batch, accumulator | 4 × u32 |
| 8 | `isNight` | u8 |
| 9 | night list | u32 n; n refs |
| 10 | trigger map | u32 entry count; per entry u32 trigger, u32 count, count refs (map iteration order) |
| 11 | time-of-day list, dated list, power list | each u32 n; n refs (list order) |
| 12 | `signsVisible` (v5) | u8 |
| 13 | sign list (v5) | u32 n; n refs |

`Read` (`0x006F3620`): version < 4 → false. The scanner (field 3) is read by `0x005DA410` with no check: the state
byte is stored as read (PV13). The scanning iterator's `GetNext` (`0x0060C8D0`) treats state 2 as done, state 0 as
fresh and **every other value (1 and 3…255) as running**, and it never rewrites a running state, so a state above 2
behaves exactly like 1 and is written back unchanged until the scan finishes (state 2) or the block step resets it
(state 0). *[compat]* FreeCity4 keeps the byte and treats it as running; `Read` does not fail. Version 4 stops after field 11 (`signsVisible` keeps 1, no signs);
version ≥ 5 reads fields 12 and 13 (a later version is read as 5). References are resolved with IID `0x89C498BE`
(requesters), `0x681905BD`, `0x28326F3E`, `0x89A5BA3F` (managers) and `0xE9793A65` (props). The requester vector
and the night list are resized to the count; the lists and the map are appended to. `Init` re-acquires the managers,
so *[choice]* FreeCity4 writes their references and ignores them on read.

**Corpus** (all 421 saves under `Regions\`): every record parses exactly.

- 409 are version 4, **122 bytes**: the shipped city templates, never run (scan flag 1, scanner fresh at (0, 0)),
  all lists empty. 225 hold night mode 0 with batch `0xBAADF00D` and accumulator 0; 181 hold mode 2 with batch 0
  (a dawn sweep on an empty list, §4.8); 3 hold mode 0, batch 0.
- 12 are version 5 (cities that ran): sizes 127 to 47,055 bytes; up to 1,253 night props, 5,296 time-of-day props,
  87 dated props, 483 power props, no signs.
- Requesters: always 3, classes `0x89C49986`, `0x8990C09A`, `0x8990C065` in that order; managers `0x498F9B01`,
  `0xA990BD10`, `0xE9A6CD3B`. Clip x1 = z1 ∈ {7, 15, 31}. `isNight` is 0 in all. No null reference anywhere.
- Prop classes: `0x2977AA47` (`cSC4PropOccupant`) and `0x2977AA49` (the visual-effect prop) in every list; the
  trigger map holds only effect props. 980 props are in both the time-of-day and the power lists.
- In finished sweeps the cursor equals the night-list length at the time (for example batch 157, cursor 1,253 = n);
  the list may have shrunk since (cursor 174 with n = 160).

Example, the 127-byte record of `Timbuktu\City - Getting Started Tutorial.sc4`:

```
05 00 | 02 | 01 07 00 00 00 0E 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 0F 00 00 00 0F 00 00 00 | 01
03 00 00 00 | C0 2F B5 13 86 99 C4 89 | B8 2B B4 13 9A C0 90 89 | 90 76 34 11 65 C0 90 89
38 00 31 13 01 9B 8F 49 | 6C 87 37 11 10 BD 90 A9 | FC 35 B4 13 3B CD A6 E9
00 00 00 00 | 02 00 00 00 | 00 00 00 00 | B8 00 00 00 | 00 | 00 00 00 00 | 00 00 00 00
00 00 00 00 | 00 00 00 00 | 00 00 00 00 | 01 | 00 00 00 00
```

Scanning, next block after (7, 14) of a 16 × 16 clip; three requesters; night mode 2 with an empty list, accumulator
184 ms; no props. A shipped template is the same without the last five bytes, with version 4, scan flag 1 and
state 0 at (0, 0).

A new FreeCity4 city writes batch 0 *[choice]* (the original writes whatever the heap held); a loaded value is
written back unchanged.

## 8. Interface slot tables *[compat]*

### 8.1 `cISC4PropDeveloper` (`0x00AB29F8`)

| Slot | Offset | Method | Address | Pops | Return |
|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface` | `0x006F0D80` | 8 | AL |
| 1 | `+0x04` | `AddRef` | `0x004F7BA0` | 0 | EAX |
| 2 | `+0x08` | `Release` | `0x006F0EA0` | 0 | EAX |
| 3 | `+0x0C` | `Init` | `0x006F1F30` | 0 | AL (true) |
| 4 | `+0x10` | `Shutdown` | `0x006F2C60` | 0 | AL (true) |
| 5 | `+0x14` | `AddPropRequester(r)` | `0x006F21C0` | 4 | none |
| 6 | `+0x18` | `RemovePropRequester(r)` | `0x006F1080` | 4 | none |
| 7 | `+0x1C` | `UpdateAllOptionalProps()` | `0x006F4DB0` | 0 | none |
| 8 | `+0x20` | `ShowSignsAndLabels(bool)` | `0x006F1210` | 4 | none |
| 9 | `+0x24` | `ToggleSignsAndLabels()` | `0x006F2BF0` | 0 | none |
| 10 | `+0x28` | `AreSignsAndLabelsVisible()` | `0x006F2C10` | 0 | AL |
| 11 | `+0x2C` | deleting destructor (not in the SDK) | `0x006F35B0` | 4 | EAX |

### 8.2 Message target (`0x00AB29E8`, +4) and serializable (`0x00AB29D0`, +8)

Message target: 0–2 thunks (`0x006F2C20`, `0x0068D230`, `0x006F2C40`), 3 `DoMessage` `0x006F4E90` (pop 4, AL;
false only for an animation tick in mode 0). Serializable: 0–2 thunks (`0x006F2C30`, `0x0068D250`, `0x006F2C50`),
3 `Write` `0x006F1930` (pop 4, AL), 4 `Read` `0x006F3620` (pop 4, AL), 5 `GetGZCLSID` `0x006F0D70` (pop 0, EAX).

### 8.3 Wealth requester (`cISC4PropRequester` `0x00AB29B0`, `cIGZSerializable` `0x00AB2998`)

| Slot | Offset | Method | Address | Pops | Return |
|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface` | `0x006F0E30` | 8 | AL |
| 1 | `+0x04` | `AddRef` | `0x004BDF00` | 0 | EAX |
| 2 | `+0x08` | `Release` | `0x006F1040` | 0 | EAX |
| 3 | `+0x0C` | `GetRequestedShare(rect const&)` | `0x006F0E10` | 4 | EAX = 10 |
| 4 | `+0x10` | `GetRequesterSatisfactionId()` | `0x006F0E20` | 0 | EAX = `0x49A1E995` |
| 5 | `+0x14` | `UsesCustomPropSelector()` | `0x00648F00` | 0 | AL = 1 |
| 6 | `+0x18` | `SelectProp(vector<u32> const&, int32 x, int32 z)` | `0x006F1CB0` | 12 | EAX |

Serializable: 0–2 thunks (`0x006F0E90`, `0x005C0500`, `0x006F1070`), 3 `Write` and 4 `Read` `0x004FC900` (pop 4,
AL true, nothing transferred), 5 `GetGZCLSID` `0x006F0E00` → `0x89C49986`.

### 8.4 Slots this class calls on others

| Interface | Slots |
|---|---|
| `cISC4LotManager` | 6 `GetLot(x, z, originOnly)` |
| `cISC4Lot` | 9 `GetLotConfiguration`, 21 `GetLocation`, 22 `GetSize`, 33 `GetState`, 37 `IsInLivableState`, 41 `GetOccupantWealth`, 46 `GetBuildingType(bool)`, 61 `GetOccupancy` (hidden result pointer, pop 4) |
| `cISC4LotConfiguration` | 23 `GetLotConfigurationObjectArray` |
| `cISC4NetworkLotManager` | 5 `GetNetworkLot(x, z)`, 8 `CreateProp(netLot, index, id)`, 10 `GetWealthForNetwork(x, z)`; network lot slot 5 (object array) |
| `cISC4PropManager` (Windows order) | 8 `IsPropTypeValid`, 9 `GetPropTypesByFamily`, 13 `GetPropProfile`, 24 `GetLotProps(list, lot)`, 29 `RemovePropA`, 30 `GetStreetProps(list, rect)`, 31 `GetStreetProps(list, x, z)` |
| `cISC4LotDeveloper` | 63 `CreateLotProp(lot, index, id)` → `cISC4Occupant*` in **EAX** |
| `cISC4PropOccupant` | 3 `AsOccupant`, 7 `GetPropType`, 11 `SetState`, 12 `GetState`, 13 `SetTimeOfDayRange`, 14 `GetTimeOfDayRange`, 15 `SetSimulatorDateRange`, 16 `TickSimulatorDate`, 17 `SetRandomChance`, 21 `SetPropTimerState` |
| `cISC4Occupant` | 6 `AsPropertyHolder`, 8 `GetPosition`, 9 `SetPosition`, 11 `GetBoundingCityCells`, 27 `GetLotTag`, 29 `SetFlag`, 31 `ClearFlag` |
| `cISC4City` | 41, 42, 91–95 |
| others | power simulator 37 `IsPowered(x, z)`; simulator 7 `GetSimDate`, `AddAgent`/`RemoveAgent`; zone manager 12; 24-hour clock `GetHour`, `GetMinute`, slot 6 `GetVisualTimeMode`; lighting manager 9; terrain 13 and 30; occupant manager 24 |

## 9. Who reaches in

| Caller | Use |
|---|---|
| City | `Read`/`Write` (before `Init`), `Init`, `Shutdown`; slot 41 hands it out |
| Crime simulator `Init` (`0x006A1819`) | `AddPropRequester`, new cities only (crime.md §3) |
| Pollution simulator `Init` (`0x006E7416`) / `Shutdown` | `AddPropRequester` (new cities only) / `RemovePropRequester` (pollution.md §3) |
| Label tool, sign-label demolish tool, sign tool (`Init`) | `ShowSignsAndLabels(true)` |
| Signpost and text-decal occupants, the 3D view | `AreSignsAndLabelsVisible` |
| 3D view `ProcessCommand` | `ToggleSignsAndLabels` |
| Lot developer, network lot manager | create the props it asks for (§4.4) |

## 10. Quirks (reproduced unless marked *[choice]*)

1. A tagged prop whose tag names no object lowers the block's slot count by one (§4.3 step 2.3.2), which lowers the
   target. Where such props come from (PV2): a lot tag is the configuration object's ID (`+0x24`), written when the
   prop is created (lot-developer.md §6.5, `CreateLotProp`, the city detail increase) and saved with the prop. A
   configuration of version < 2 gets **generated** IDs instead: a per-process serial (`0x00B2166C`, seeded through
   the GZCOM framework on first use and incremented per object, `ConvertPropertiesToObjectArray` `0x00479D10`), so
   its props' saved tags match nothing after the next load. No shipped configuration is of version < 2 (all 1908
   lot configurations and all 543 network lot configurations are version 2), so with shipped data only these remain:
   a prop whose object the current detail level filtered out of the configuration, which the city detail manager's
   reduce removes at `Init` (city-detail.md §3.2), so only until then; and a prop of an earlier configuration of the
   lot, if one survives a change of configuration *[not traced]*. `GetLotProps` selects by the lot's **rectangle**
   (prop-lotconfig.md A7.2), not by owner, so any tagged prop inside it counts. Plugin lots of version < 2 hit the
   rule with every tagged prop after each load *[inferred]*.
   **Corpus count (re-answers27, 2026-10-16).** The count is now made, with `re/prop_tag_corpus.py` over the 421
   distinct saves under `Regions\`: **40,830 of 147,162 tagged lot props are unmatched (27.7 %)**, so the rule
   fires on more than a quarter of the tagged lot props in the corpus. 40,794 sit on a lot that has a
   configuration whose objects do not include the tag; 36 sit on a lot with no configuration. 38,933 of the tags
   name no object in **any** shipped configuration (and `Plugins` holds no plugin `.dat`), so they are leftovers
   of a configuration the install no longer has. Effect props are clean (844 tagged, 0 unmatched). Full figures,
   the per-region breakdown and the parser's validation numbers are in QUESTIONS.md under PV2.
2. At most one removal per requester per block update; creations are not limited.
3. The remainder of the split starts at requester index 1 (§4.3 step 4).
4. A failed creation still consumes the free slot for the rest of the update.
5. The first block step after setting up does nothing.
6. Night batch size is uninitialised in the constructor; `0xBAADF00D` in 225 shipped records.
7. A sweep over an empty night list never returns the mode to 0.
8. `Shutdown` does not clear the power list (*[choice]*: FreeCity4 clears it).
9. Power changes update night-lit street props at once but night-lit lot props only at the next sweep.
10. The lot-state handler uses the message object as a lot without `QueryInterface`.
11. No bounds check on the requester index (*[choice]*: treated as no index); the null-lot creation path and the
    null last element of the time-of-day list are undefined (*[choice]* in §4.4 and §4.9).
12. Unregistering tests each list's own condition first; a trigger-map prop is not removed when the 3D view has gone
    (§4.7).
13. A scanner state byte above 2 is read as is and behaves as running (§7).
14. A total requested share of 0 faults the original (*[choice]*: stop after the cell walk, §4.3).

## 11. Open questions

See QUESTIONS.md, section "From the re-specs10 specs" (PV1–PV6). PV2 answered by re-answers19 (§10 quirk 1).
PV7–PV13 answered by re-answers22 (§4.3, §4.4, §4.6, §4.7, §4.8, §4.13, §7).
PV14 answered by re-propdev-pv14 (§5.1–§5.3; conformance vectors §12.15–§12.22).

## 12. Conformance tests

1. **Empty record**: a new 128 × 128 city after `Init` writes version 5 with scan flag 1, scanner fresh at (0, 0),
   clip (0, 0, 15, 15), `wealthAdded` 1, three requesters, batch 0, `signsVisible` 1, all lists empty: 127 bytes.
2. **Template read**: the 122-byte version-4 record of `Berlin\City - New City (10).sc4` reads, keeps batch
   `0xBAADF00D`, and writes back as version 5 (127 bytes) with the batch unchanged and `signsVisible` 1. Version 3
   is rejected.
3. **Round trip** of every corpus record: re-serialising a parsed version-5 record gives the same bytes.
4. **Block order**: on a 64 × 64 city (clip 7, 7), the first block step after `Init` updates nothing; the next 64
   update (0, 0), (1, 0), …, (7, 7); the 66th updates (0, 0) again.
5. **Worked example A** (§4.5): with a fixed member seed, two wealth props are created, in the slots chosen by
   `RS(0, 4)` then `RS(0, 3)`; exactly two member draws.
6. **Example C**: one removal, the entry chosen by `RS(0, 2)`; no creation.
7. **Split**: shares (5, 3, 2), N = 13 → want (6, 4, 3); shares (10, 0, 0), N = 2 → (2, 0, 0); shares (0, 4, 0), N = 3
   → (0, 3, 0).
8. **No lots**: a block of street cells only (network lots with optional props) changes nothing and draws nothing.
9. **Wealth selection**: candidates with wealth (1, 2, 3, 2) on a lot of occupant wealth 2 → one draw `U(2)` from
   the wealth generator, result the second or fourth ID; on a seaport lot (zone 12) of wealth 3 the same; no
   candidate → 0 and no draw.
10. **Night sweep**: 20 night props, dusk: 19 `rand()` calls, batch 3; ticks of 150 ms do nothing until the
    accumulator passes 200; the sweep ends after the seventh batch with mode 0 and cursor 20; unpowered props get
    state 0 in mode 1.
11. **Time of day**, each value checked as the first tick after a load: range (18.0, 8.0): 07:30 on, 12:00 off,
    18:00 on; range (8.0, 18.0): 08:00 on, 18:00 on, 18:06 off. In one session, an update at 07:00 followed by a
    tick at 07:20 changes nothing; a tick at 07:30 updates.
12. **Registration**: a prop with `0x0A751675` present and no other timed property ends with masks 1, 4 and 8 on, is
    in the dated list only, and has state 1 and chance 100.
13. **Signs**: `ShowSignsAndLabels(false)` twice clears flag 2 once per sign; `ToggleSignsAndLabels` restores it;
    `AreSignsAndLabelsVisible` follows.
14. **Lot state**: a lot going 2 → 3 posts `0xCA5E5027` with data3 = 1; a prop with `0x6A5E0727 = 3` on it ends in
    state 0, and in state 1 once the lot leaves state 3.

The following observation vectors are specification fixtures, not measurements of a live game shadow. Synthetic
tables exercise copying and cursor boundaries independently of clock seeding. All native source bytes must be
unchanged by every observation; only the original's own subsequent calls advance its state.

15. **PV14-01, relocated table:** synthetic G = `0x10000000`, table words all zero except word 623 = 1,
    P = `0x100009BC`, r = 1. Import at shadow base `0x20000000`: k = 623, r = 1, relocated pointer
    `0x200009BC`. First raw output = `0x00400091`, then k = 624, r = 0. Native source remains byte-identical.
16. **PV14-02, exhausted table:** continue PV14-01: another raw draw reloads before reading, returns 0, leaves
    k = 1, r = 623. In the input table the first reloaded word is zero (old words 0, 1 and 397 were zero).
    In contrast P = G +`0x9C0`, r = 1 is invalid and must be rejected, as must an unaligned P or r = −1.
17. **PV14-03, integer scaling:** supplied raw word `0xFFFFFFFF`, bound 4 → 3; raw `0x80000000`, bound 3 → 1;
    raw `0xFFFFFFFF`, range [−2, 2) → 1. Bound 1 yields 0 and consumes one word. A transcript with bound 3
    where the shadow requests bound 4 is invalid even if both recorded and computed results happen to be 1.
18. **PV14-04, night inputs:** ordered list [A, B, C, D], thread's pre-draw CRT state 1 → returns
    [41, 18467, 6334], swap indexes [1, 2, 2], final list [A, B, D, C]. Three draws. Lists [] and [A] remain
    unchanged with zero draws. A different thread's state is irrelevant. Advance the real thread once between
    two operations: reusing the first operation's snapshot is invalid; a fresh pre-call snapshot or real-return
    transcript supplies the second operation's inputs.
19. **PV14-05, shared wealth:** filtered wealth candidates [A, B], live file generator snapshot with next raw
    word `0x80000000` → bounded index 1, returns B, one shared-generator word consumed by the original call.
    The independently copied generator predicts B without changing the original. Empty filtered candidates →
    return 0, no draw. An unrelated intervening use invalidates continuation from the earlier snapshot.
20. **PV14-06, kept selector:** original selector called once for requester R, singleton [A], x = 7, z = 9,
    returns 0 → current A fails the kept check and is removed. Transcript replay returns 0, makes zero native
    selector calls, and removes the same prop. Returning A as a placeholder incorrectly keeps it. A return of A
    instead is the keep case; both consume precisely the one original selector invocation.
21. **PV14-07, creation selector:** original receives ordered [A, B, A], returns B → shadow uses B at the
    matching creation call. The same inputs with actual return 0 fail selection, consume the already selected
    free slot for this update and leave need unchanged (§4.3/§4.4). Missing return, reordered [A, A, B], wrong
    requester, wrong coordinates, or a leftover event at operation end → invalid comparison, not a pass.
22. **PV14-08, unavailable observation:** wrong executable identity, a torn/invalid table, or lost nested
    selector events → observation unavailable/invalid. Preserve native state; do not seed or call a selector
    to fill the gap. Equal serialized records after such an operation do not certify random-output equivalence.

## 13. How `CreateProp` actually gets an occupant (GLUE21-2 — DIFFERS, answered 2026-10-16 by re-answers31)

**Question.** With `enable = propmgr` the game reads `0x7C` from a null (`esi = 0`) at `SimCity 4.exe+0x2F44EF`
on the prop path that calls prop manager slot 15 `CreateProp`. The replacement's `create_prop_instance`
returns 0 when GZCOM does not answer `GetClassObject(cls, 0xE9793A65, &p)`, and hands the game a null. Should
`CreateProp` / `Add*Prop` **fail the call** instead?

**Answer: neither.** The original never returns a null from `CreateProp`. When GZCOM does not answer it
**fabricates the occupant itself**. Failing the call is a *[choice]* you may adopt as an interim, but it is not
what the original does, and it will not by itself stop the fault: the observed `+0x7C` read shows the caller
takes the out parameter as an occupant without a null check, so a null — whether returned with `true` or with
`false` — is still dereferenced.

### 13.1 What the original does (`cSC4PropManager::CreateProp`, `0x004A3040`)

1. **Validate the type first.** `vtable+0x20` (slot 8) `IsPropTypeValid(id)`. If it answers false the call fails
   immediately — that is the one real "fail the call" exit.
2. **Pick the class.** Default `cls = 0x2977AA47` (`cSC4PropOccupant`, `re/class_id_table.tsv:400`). Then slot 13
   `GetPropProfile(id)` (`0x004A3750`) and the dword at `profile+0` may override it (that is how
   `0x2977AA48` `cSC4PropOccupantTerrainDecal`, `0x2977AA49` `cSC4PropOccupantVisualEffect` and the burnt
   building `0xC9EFB016` are reached).
3. **Try GZCOM.** `CreateObject(cls, 0xE9793A65, &out)` — `vtable+0x0C` on the class factory, three stack args
   `(clsid, iid 0xE9793A65, out)`. If it succeeds, use that object.
4. **Otherwise allocate and construct it by hand**, dispatching on the class id:

   | `cls` | allocated size | path |
   |---|---|---|
   | `0x2977AA48` (terrain decal) | `0xA0` | `0x4A30D3` |
   | `0x2977AA47` (prop occupant) | `0x94` | `0x4A30B1` |
   | anything else | `0x68` | `0x4A309A` |

   Each path is `alloc` → construct → test → `add eax, 4` → store. **The out parameter receives the object at
   `+4`**, i.e. `CreateProp` returns the *second* sub-object, not the allocation base. Then `vtable+0x04`
   `AddRef`, `vtable+0x20` with the id, and a call into the manager (`0x4A310C`) that registers it; on success a
   further `AddRef`.
5. Only if the allocation itself returns null does the function leave a null in the out parameter.

So the compat behaviour is: **validate, try GZCOM, fall back to allocate-and-construct, and only then give up.**
This is the same pattern as the lot configuration manager's helper (lot-configuration.md §1.1).

### 13.2 The change to make

Implement the fallback in `create_prop_instance` (and any `Add*Prop` that can reach it):

1. If `IsPropTypeValid` is false, **fail the call** (this is the original's only clean failure).
2. Try `GetClassObject(cls, 0xE9793A65, &p)`; if it answers, use it.
3. Otherwise allocate `0x94` / `0xA0` / `0x68` by class-id family and construct a `cISC4PropOccupant`-shaped
   object, returning the **`+4`** sub-object pointer as the original does. If FreeCity4 cannot own that layout
   yet, do not hand back a null the caller will dereference — return `false` **and** make sure the out parameter
   is left untouched rather than zeroed, and treat the remaining `+0x7C` fault as expected until the occupant
   exists.

`0xE9793A65` is the IID asked for at create time; it is an interface IID, not a class ID. The same
IID-versus-CLSID mistake that broke the automata classes (automata §15) and the effects manager (effects §18)
applies here: the class is `0x2977AA47` and the interface is `0xE9793A65`, and a `QueryInterface` that answers
only the CLSID will leave the out parameter null.
