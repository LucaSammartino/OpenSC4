# Flora simulator (`cSC4FloraSimulator`) — behaviour spec

Status: round 2 (M3), written from the Windows 1.1.641 executable (code `0x006AFBD0–0x006B3790`). The earlier
write-up in `docs/subsystems/grid-sims.md` §10 was the starting point. Every fact below was checked against the
Windows code; the Mac pseudocode was used only for names. Corrections to the earlier write-up are in §14.

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

Shared material cited and not repeated here: streams, refs (foundations §2, §2.7), class store (§3), city
component table (§3.6), Init order (§3.7), messages (§5), random generators (§6), C runtime `rand()` and the
random cell iterator (§12, weather §6.7), month order (§13), slot-order rule (§16), framework slot tables (§17);
scheduler levels and agent semantics (simulator spec).

---

## 1. Role and identity

The flora simulator owns **trees and other vegetation**. Flora is **occupant-based**: every tree is a
`cSC4FloraOccupant` in the occupant manager (occupant type `0x74758926`). The simulator keeps no density grid.
It:

- keeps the process-wide **flora type registry** (one static record per flora exemplar, §3.2);
- **plants** flora for the user tool, the lot developer, the lot editor and its own seeding (§6.5);
- **ages** existing flora: young → maturing → mature, then rebirth at the end of its lifespan (§6.7);
- **spreads** flora: mature flora drops seeds around itself, and "wild" seeds fall anywhere (§6.7.3), both only
  while propagation is on (god mode, or the `propogation` cheat);
- removes flora under new buildings and transport networks (§6.10), and moves or removes it when the terrain
  changes (§6.10.2);
- turns flora into **burnt trees** when a fire burns it, and clears burnt trees at a daily rate (§6.8, §6.9);
- saves the flora of an obliterated city and restores it in the replacement city (§10.3).

**Seasons are not simulated here.** The simulator subscribes to no season message and has no per-season state.
Seasonal tree appearance is a model/rendering matter of the occupant *[inferred]*.

| Item | Value |
|---|---|
| Class | `cSC4FloraSimulator`, object size 184 bytes, created by `GZCOM_Create` (factory `0x006B2940`) |
| CLSID | `0xC990C034` (`GetGZCLSID` `0x006AFBD0`) |
| IIDs answered | `0x00000001` and `0x06C66177` (`cISC4FloraSimulator`) → primary; `0x452294AA` (`cIGZMessageTarget2`); `0xE4FDA3D4` (`cIGZSerializable`) |
| City component | slot 27 of the component table (foundations §3.6); `cISC4City::GetFloraSimulator` is slot 59 (`+0xEC`) |
| Init order | after flammability, before industrial (foundations §3.7) |
| Global pointer | `SL::OnCityInit` stores `GetFloraSimulator()` in the global service pointer at `0x00B43D80`; `SL::OnCityShutdown` clears it. Most consumers use that global (§13.1). |

`QueryInterface` *[compat]*: for the four IIDs above it stores the matching interface pointer, `AddRef`s it and
returns AL = 1. *[quirk]* With a null `this` it stores null and still returns true. Any other IID: AL = 0, the
out pointer is left untouched.

`Release`: if the count is > 1, decrement and return it. Otherwise call `Shutdown`, then the class destructor
(slot 25) with "delete" = 1, and return 0.

---

## 2. Grids

None. The flora simulator creates, owns and reads **no** `cISC4SimGrid`. simgrid.md §6.4 (grid ownership) does
not apply, and no other code reads flora data through a grid. The only "map" structure is the random cell
iterator over 2×2-cell blocks (§6.7.1).

---

## 3. Data model

### 3.1 Simulator state

| Field | Type | Initial (constructor) | Set by | Saved |
|---|---|---|---|---|
| initialized | bool | false | Init / Shutdown | no |
| enabled | bool | **true** | cheat `on`/`off` (§5.2) | yes |
| propagation | bool | **true** | Init, CityEstablished / CityObliterated, cheat | yes (overwritten by Init) |
| propagationForced | bool | false | cheat `propogation` | yes |
| preserve | bool | false | cheat `preserve` | yes |
| sizeX, sizeZ | f32 (metres) | 1.0, 1.0 | Init (city `SizeX`/`SizeZ`) | yes (overwritten by Init) |
| cellsX, cellsZ | s32 | 1, 1 | Init (city cell counts) | yes (overwritten by Init) |
| today | u32 (day number) | 0 | data1 of each SimNewMonth | yes |
| period (months per full pass) | u32 | **6** | tuning `0x8761ABD0` | yes (overwritten if the tuning exemplar exists) |
| blocksPerMonth | u32 | 1 | Init, tuning read (§6.7.1) | yes (recomputed) |
| wildSeedsPerMonth | u32 | **5** | tuning `0x085489C7`, scaled (§7) | yes (recomputed, see §14 quirk 11) |
| keepIteratorFlag | u8 | 0 | **never set** (only Read) | yes |
| block iterator | random cell iterator, table mode | prime-step mode, dirty | Init, monthly | yes (but reset by Init) |
| rng | `cRZRandom` | seeded with `0xFFFFFFFF` (clock) | | **yes** (state) |
| elevationScale | f32 | 0.08 | tuning `0xC9DBDD02` (`16 / v`) | no |
| burntList | list of occupant refs | empty | KillFloraOccupant; daily removal | yes (refs) |
| burnAccumulator | f32 | 0 | daily | yes |
| burntRemovalAccumulator | f32 | 0 | daily | yes |
| burnRate | f32 | 1.0 | tuning `0xC9DBDD00` (`1 / v`) | no |
| burntRemovalStep | f32 | 4.0 | tuning `0xC9DBDD01` | no |
| held services | occupant manager, weather (may be null), simulator, terrain, lot manager, blocking filter | null | Init / Shutdown | no |

### 3.2 Flora type registry (static, one per process)

Three containers are **static** (shared by every city of the process): a map *type ID → static flora record*
(ordered by unsigned type ID), a vector of type IDs in registration order (the **type array**), and a map
*family ID → vector of type IDs*. They are filled at the first `Init` of the process (§4.1 step 2) and kept
across cities. An application-shutdown callback releases each record, empties the map and the type array (not
the family map).

**The shutdown callback** (verified, FQ2). `0x006B1070`, registered once per process by §4.1 step 2 with data 0.
If the "types loaded" flag is set it clears the flag, then for each map entry drops the map's reference to the
record (a count of 1 destroys it: the held exemplar is released, the family vector and the record are freed;
a higher count is only decremented, so records still held by flora occupants survive), erases every map node
and empties the type array; it returns true. The app keeps the callbacks in a list in registration order and runs
them inside its own `Shutdown` (`0x0044A633`–`0x0044A650`), each called with its data. The framework calls that
`Shutdown` (`0x0087AB54`) after it has sent PreAppShutdown to every framework hook (`0x0087A376`, hook slot 6)
and before PostAppShutdown (`0x0087A3C6`, slot 7) and PostSystemServiceShutdown (`0x0087A416`, slot 8), so plugin
DLLs are still loaded when the callbacks run. Nothing after it reads the registry, and it posts no message and
writes nothing: it only returns memory at process exit. **No observable behaviour depends on it**; *[choice]*
FreeCity4 need not register it and may keep its records until the process ends (§13.2).

Every `Init` also inserts a **default record** under key **0** into the map (only the map, not the type array).
It is a fresh record with the defaults of §3.3 and an **uninitialised flags byte**. If key 0 is already present
the new record leaks *[quirk]*. Consequences: `IsFloraTypeValid(0)` is true, `GetStaticFloraData(0)` is
non-null, and the default record takes part in `GetBestFloraTypeForLocation` and `GetFloraTypesByFilter`
*[quirk]* (§14). **After an exemplar-change reload** (§5) key 0 is absent: the reload clears the map
(`0x006B353A`), key 0 included, and only loads the exemplars, so until the next `Init` `IsFloraTypeValid(0)` is
false, `GetStaticFloraData(0)` is null and the default record takes part in nothing (verified, FQ14). The next
`Init` inserts a fresh default record, which then does not leak.

**The flags byte of the default record** (`Init`, `0x006B2BAA`–`0x006B2BF2`): the record is 376 bytes from the
game's global heap (`operator new`, the 512-byte pool, `docs/abi/allocators.md` instance C). The inline
constructor clears the reference count, the held exemplar, the family vector and one float; the defaults
routine (`0x006B0220`) writes the type, next type, seed type, water flag, the 14 floats, the cluster values, the
propagation table and the unnamed u32. Nothing writes the flags byte (nor the int64 plop cost). The pool hands
out recycled blocks without clearing them, so the byte holds whatever the previous user of that memory left
there: it is not a property of the game and cannot be reproduced. Bit 0 decides whether the default record can
win `GetBestFloraTypeForLocation` (§6.3); bit 1 is never consulted for it (no flora occupant has type 0, §6.5
step 1); the brush and wild seeding never see it (they use the type array). *[choice]* FreeCity4 gives the
default record flags **`0x01`** (bit 0 set: never chosen automatically), so type 0 never shadows a real type in
`GetBestFloraTypeForLocation` or the `blast` cheat. Exemplar records have the same uninitialised byte, but
`AddStaticFloraData` clears bit 0 and sets bits 1 and 2 before applying the properties (§3.3), so only bits 3–7
are left over, and no code reads them; FreeCity4 clears them *[choice]*.

**Registration order** (the type array, §6.7.3 and §6.11): the order of the resource manager's key list for type
`0x6534284A`. That list is built segment by segment (the resource manager's segments in their registration order;
a multi-file segment walks its files in order), and each package file lists its index **hash table** in bucket
order: bucket = `instance mod B` ascending (`B` = the table's prime bucket count, at least 53), most recently
indexed entry first within a bucket (`0x0096F9AB`, `0x0097C081`, `0x0097C1EA`; hash = the instance,
`0x0097C0C5`). Flora instances are large hash-like IDs, so the registration order is **not** ascending type ID
*[compat]*; it decides wild-seed draws (§6.7.3) and seed types (§6.11, quirk 12).

### 3.3 Static flora record (`cSC4StaticFloraData`)

One reference-counted record per flora type, built from a flora exemplar (resource type `0x6534284A`,
property `0x10` = `0x0F`). The names below describe how the code uses each value; the properties have no names
in our tuning tables. The 14 floats of property `0x083DC35B` fill fields 1–14 in order.

| # | Field (use) | Source | Code default |
|---|---|---|---|
| — | type ID | property `0x21` (u32) | 0 |
| — | next cluster type | `0x2A0348BA` (u32) | 0 |
| — | seed type (type planted when this flora seeds) | computed at registration (§6.11) | own type |
| — | water flora | `0xEA346888` **present** → true | false |
| 1 | maturity age (days) | `0x083DC35B[0]` | 730 |
| 2 | seeding age (days) | `[1]` | 730 |
| 3 | seeding interval (days) | `[2]` | 365 |
| 4 | seeds per cycle | `[3]` | 2 |
| 5 | seed spread (metres) | `[4]` | 128 |
| 6 | lifespan (days; rebirth age) | `[5]` | 1460 |
| 7 | maximum age (days) | `[6]` | 1560 |
| 8 | slope tolerance | `[7]` | 0.5 |
| 9 | minimum propagation score | `[8]` | 0.3 |
| 10 | seed probability | `[9]` | 0.5 |
| 11 | occupant box half-width (read by the occupant, not the simulator) | `[10]` | 1.0 |
| 12 | unused here | `[11]` | 4.0 |
| 13 | clearance radius (metres) | `[12]` | 3.0 |
| 14 | height (metres) | `[13]` | 3.0 |
| — | cluster radius / cluster height (metres) | `0x2A0348BB` / `0x2A0348BC` (f32) | 0 / 0 |
| — | propagation table: 16 × 16 bytes, `[elevation index][moisture type]` | `0x083DC487`: 256 floats, byte := low 8 bits of `trunc(v × 255)` | `0x99` (153) where both indices are in 5..12, else 0 |
| — | unnamed u32 (not used here) | `0x083DE16F` | 0 |
| — | flags: bit 0 "never chosen automatically", bit 1, bit 2 | default bit 0 clear, bits 1 and 2 set; `0xE9D117C3` (bool) sets bit 0 to its value; `0x6A37EBB6` (bool) present: true clears bit 2, false clears bit 1 | |
| — | family IDs | `0xA8F149C5` (u32 list) | empty |
| — | plop cost (int64, read by the flora tool) *[inferred name]* | `0x49CAC341` | 0 |
| — | the exemplar (held) | | |

*[quirk]* Property `0x083DC35B` is copied as 14 floats without a length check, and table values above 1.0 wrap
in the byte conversion (1.2 → 306 → 50).

Flag use: bit 0 set → the type is never picked by `GetBestFloraTypeForLocation` or by wild seeding (§6.3,
§6.7.3); bit 1 set → while propagation is off, the type skips the score test (§6.7.2); the terrain's god-mode
flora brush lists only types with bit 0 clear and bit 1 set *[inferred meaning: "wild/brush flora"]*. Bit 2 is
not read by any code found.

*[compat]* **Parent-cohort properties count** (verified, FQ13). Every value in the table is read through the
exemplar's property-holder calls (slot 3 `HasProperty`, slot 8 `GetProperty(id, u32&)`, slot 9 `GetProperty(id)`
and the typed helpers; `AddStaticFloraData` `0x006B12D0`, reads `0x006B13A6`–`0x006B159E`), and so does the
`0x10 = 0x0F` test that admits an exemplar at all (§4.1 step 2). All of these use the **effective** property
list (data-formats §4.4): a property the exemplar inherits from its parent cohort (or a grandparent) is used
exactly as if it were its own, and an own property overrides the inherited one. The 22 shipped flora exemplars
(all in `SimCity_1.dat`; none in `SimCity_2`–`5.dat` or `EP1.dat`) have no parent cohort, so on shipped data
including or leaving out inherited properties gives the same registry. They differ only for a plugin.
*Example:* a plugin exemplar with own properties `0x10 = 0x0F` and `0x21 = T` whose cohort holds `0x083DC35B` and
`0x2A0348BA = N`: the original's record for T has the cohort's 14 floats and next type N; a reader that leaves the
cohort out gives the code defaults (730, 730, 365, …) and next type 0. If `0x10 = 0x0F` itself is in the cohort,
the original registers the type and such a reader skips it. *[choice]* Read flora exemplars with inheritance.

Shipped example, **Burnt Tree** `0x29CFF3CB` (group `0xE83E0437`): floats {100, 100, 200, 3, 128, 15000, 15050,
0.6, 0, 0, 1, 5, 1.5, 5}, `0xE9D117C3` = true, a propagation table that is mostly 0–0.9.

### 3.4 The flora occupant (parts the simulator relies on)

`cSC4FloraOccupant` (CLSID `0xA9C05C85`) answers `cISC4FloraOccupant` (IID `0xC83DD057`) and `cISC4Occupant`
(`0x26BB767E`); its constructor sets occupant type `0x74758926`. The simulator uses:

| Value | Meaning |
|---|---|
| flora type | the static record key |
| birth date, last seeding date | day numbers (same scale as the simulator's day number, simulator spec) |
| state | **signed 8-bit**: 0 young, 1 maturing, 2 mature, 3 burnt, 4 dying (§6.7.2) |
| orientation | 0..3, chosen at planting |
| static record | a counted reference |

Occupant flag 1 (`IsFlagSet(1)`) means "inserted in the occupant manager" *[inferred]*: the occupant manager sets
it on insert, the monthly visit and `KillFloraOccupant` skip occupants without it, and the obliteration save
clears it around the write (§10.3). Flora changes are announced with the occupant's own
`PostOccupantMessage(0x99EF1145 ChangeOccupant, 0)` (slot 15).

The occupant's save record is version 3 and includes a lot tag (0 for wild flora); its byte layout is in
lot-records.md §11 (Write `0x00472990`, Read `0x00472A20`; validated on 744,617 saved entries).

---

## 4. Lifecycle

### 4.1 `Init()` → true

1. If already initialized, return true (AL = 1, the rest of EAX is garbage). Set initialized.
2. Only the first time in the process: register the application shutdown callback (`cISC4App` slot 69 `+0x114`
   `RegisterShutdownCallbackFunction(cb, 0)`), then **load all flora types**: enumerate the available resources
   of type `0x6534284A` (resource manager slot 18 `+0x48`, key-list `EnumKeys` slot 7 `+0x1C`); for each key
   whose exemplar has property `0x10` = `0x0F`, call own slot 5 `AddStaticFloraData(key, exemplar)`. Then post
   FloraTypesUpdated (§9).

   *[compat]* **What the enumeration amounts to** (verified, FQ1). Slot 18 (`0x00973280`) only wraps a key filter
   that accepts a key exactly when its **type** equals the given type (filter slot 3, `0x00973194`; group and
   instance are not looked at) and calls slot 17 `GetAvailableResourceList(list**, filter)` (`0x009739CB`), which
   adds the matching keys of every segment, segment by segment (§3.2). So slot 17 with any filter that accepts
   exactly the keys of type `0x6534284A` builds the same list in the same order. `EnumKeys` (slot 7,
   `0x009749CB`) hands the keys to the callback from first to last; key-list slot 9 `Size` and slot 10 `GetKey(i)`
   (`0x00974741`) index the same sequence, so reading keys 0 … `Size` − 1 is equivalent. Per key (`0x006B03B0`):
   `GetResource(key, 0xA52160F5)` (the exemplar interface, data-formats §4); if that succeeds and property `0x10`,
   read with the u32 converting getter (`0x005FD480`, data-formats §5.1, so through the parent cohort, §4.4), is
   `0x0F`, call own slot 5 with the key and the exemplar; the exemplar reference is released either way. A key
   whose resource does not load is skipped silently.
3. Subscribe on the message server (`+0x14`) to `0x278E6003`, `0x99EF1142`, `0x99EF1143`, `0xC53D10AA`,
   `0x26D31EC4`, `0x26D31EC5`, `0xAB99A64B`, in that order. Register command `0xAB99A64B` with the command server
   (slot 6 `+0x18`) under the name `"PlaceFlora"` and parameter info null.

   *[compat]* **The registration call** (verified, FQ12). Both halves of this step run only when the message server
   global (`0x00B43CCC`) is non-null (`0x006B29AA`); the command server global (`0x00B43CB0`) is then used without
   a null test, and the result is ignored (`0x006B2A0C`–`0x006B2A1F`). The arguments are (`0xAB99A64B`, a plain
   `const char*` to `"PlaceFlora"` at `0x00AB1428`, **null**), so null parameter info is the original's. The
   server's slot 6 (`0x0092B075`, pops 12, AL) refuses an ID of 0, a null name or an ID already registered;
   otherwise it stores the command, passes the info to its own slot 8 `RegisterCommandParameterInfo(id, info)`
   (`0x0092AD10`, pops 8) and announces the registration. Slot 8 copies a non-empty info string as the command's
   help text; for a null or empty one it writes `"Format: <name>\nOutput: <none>"` (format at `0x00AD1738`)
   instead, but only when the server's own help flag (`+0x48`) is set, and otherwise leaves the text empty.
   *Example:* with the flag set, the help text of `0xAB99A64B` is `"Format: PlaceFlora\nOutput: <none>"`.
4. If no simulator is held yet: hold it and `AddAgent(messageTarget, 4, "cSC4FloraSimulator", 3)` then the same
   at level **6**. Flags 3 = enabled before and after the city is established.
5. Hold the occupant manager, the weather simulator (may be null), the terrain and the lot manager. Create the
   **blocking filter** (§6.4.1) and hold it.
6. If the city exists: sizeX, sizeZ := city slots 89/90 (`SizeX`/`SizeZ`, float in ST0); cellsX, cellsZ := city
   slots 93/94 (cell counts, EAX); `blocksPerMonth := ((cellsX × cellsZ) div period) >> 2` (32-bit product,
   unsigned division). This uses the period **before** the tuning read of step 9: the constructor's 6 in a new
   city, the saved value in a loaded one. Period 0 → integer divide fault (§7).
7. If keepIteratorFlag = 0 (always, §14): set the iterator rectangle to `(0, 0, cellsX/2 − 1, cellsZ/2 − 1)`
   (halving truncates toward zero; the upper corner is clamped to ≥ the lower corner), force **table mode**,
   then `reset(shuffle = true)` (foundations §12.4: table refilled in natural order, then `random_shuffle`d,
   `n − 1` C `rand()` calls).
8. Insert the default record under key 0 (§3.2).
9. Read the tuning (§7).
10. `propagation := not city.GetEstablished()` (city slot 22, AL); then if propagationForced, propagation := true.
11. Register the cheat code `0x095BDC9C` with the string `"Flora"` (cheat manager slot 3 `+0xC`) and subscribe the
    message target to it (slot 7 `+0x1C`, `AddNotification2`).

    *[compat]* **The notification ID** (verified, FQ11). The call is `AddNotification2(target, 0x095BDC9C)`
    (`0x006B2C77`), and `Shutdown` removes the same pair (`RemoveNotification2(target, 0x095BDC9C)`,
    `0x006AFDDD`). The cheat manager keeps a list of (target, ID) pairs (`0x009212A6`: a pair already present is
    refused) and, when a cheat is entered, sends `0x230E27AC` with data1 = the cheat's ID to every pair whose ID
    **equals that cheat's ID or is 0** (`0x00920F34`–`0x00920F3E`). With the original's ID only "Flora …"
    lines reach flora, so the "another cheat" branch of §5 is reachable only by a message sent to flora
    directly; with 0, every cheat would reach it and each non-Flora cheat would take the exemplar-changed path.
    The original budget (`0xCBF31943`, `0x4AB87766`, `0x493F4FE6`; `0x00693135` …) and simulator (`0x0AC07E24`,
    `0x8A78BEFF`; `0x007033B1` …) likewise subscribe with their own cheat IDs, never 0.
12. `spec := app.GetNewCitySpecification()` (`cISC4App` slot 39 `+0x9C`). If
    `spec.obliteratedCityDBSegment` (SDK `SC4NewCitySpecification`, offset `0x204`) is non-null, restore the
    obliterated flora from it (§10.3).
13. Return true.

### 4.2 `Shutdown()` → true

If not initialized, return true. Clear initialized. If the burnt list is non-empty, hand it to the occupant
manager's `ReleaseOccupantList` (slot 48 `+0xC0`). Unsubscribe from the cheat (`RemoveNotification2`, slot 8) and
unregister cheat `0x095BDC9C` (slot 4). Release the filter, lot manager, terrain, occupant manager and weather.
`RemoveAgent(messageTarget)` (all levels), release the simulator. Unsubscribe `0x26D31EC5`, `0x26D31EC4`,
`0xC53D10AA`, `0x278E6003`, `0x99EF1142`, `0x99EF1143`, `0xAB99A64B`; unregister command `0xAB99A64B` (slot 9).
Return true. The static registry is not touched.

---

## 5. Scheduling and messages handled

Agents: one message target registered at levels 4 (day) and 6 (month), name `"cSC4FloraSimulator"`, flags 3.
`DoMessage` **always returns true** (finished) *[compat]*; nothing is spread over frames.

Order: the month agent reads the **weather moisture** (`GetMoistureType`, refreshed weekly by the weather, so
same-month order does not matter), the terrain and the lot manager. Nothing else in the month reads flora
results, so flora's position in the month list (foundations §13.2: registration 19) has no observable effect
*[inferred]*.

| Message | Action |
|---|---|
| SimNewDay `0x66956814` (level 4) | Daily burnt-flora removal (§6.8). |
| SimNewMonth `0x66956816` (level 6) | `today := data1`; monthly simulation (§6.7) if enabled. |
| InsertOccupant `0x99EF1142` | `occ := void1`. If its property holder **has** property `0x029244DB` (preview occupant, foundations §5.2): ignore. Else §6.10.1 with `type := data3` (the occupant type, §9.1). |
| RemoveOccupant `0x99EF1143` | Same preview test. If `data3` = flora type and the burnt list holds `occ`: remove it from the list and release the list's reference. |
| TerrainHeightsChanged `0x278E6003` | Only if the terrain is initialized (terrain slot 5): cell rectangle `x0 = max(0, (data1 >> 16) − 1)`, `x1 = min(data1 & 0xFFFF, cellsX − 1)`, `z0 = max(0, (data2 >> 16) − 1)`, `z1 = min(data2 & 0xFFFF, cellsZ − 1)`; §6.10.2. |
| CityEstablished `0x26D31EC4` | `propagation := propagationForced`. |
| CityObliterated `0x26D31EC5` | `propagation := true`. |
| PlaceFlora command `0xAB99A64B` | `params := void1` (command parameter set). type := param 0 as u32; `p1, p2` := params 1 and 2 as floats; `AddNewFloraOccupant(type, {p1, p2, −1}, false)`. *[quirk]* p2 lands in **y** and z = −1, so the location check always fails and nothing is planted (§14). Reads (verified, FQ10; `0x006B3497`–`0x006B34F6`): parameter-set slot 5 `operator[](0)`, `(1)`, `(2)` in that order, then variant slot 24 `GetValUint32()` for 0 and slot 36 `GetValFloat32()` for 1 and 2 (§12); a missing parameter reads the set's empty variant; the result of own slot 17 is ignored. |
| Cheat `0x230E27AC` | If data1 = `0x095BDC9C` and the command line has ≥ 2 words: §5.2. Otherwise (another cheat, or a bare "Flora"): handled **exactly like `0xC53D10AA`** below *[quirk]*. With the original subscription (§4.1 step 11) only "Flora …" cheats are delivered, so "another cheat" arrives only when sent to flora directly. |
| Exemplar changed `0xC53D10AA` | `inst := data2` (data1 is read and ignored). If `GetStaticFloraData(inst)` is non-null: clear the type array, the type map and the family map (records are **not** released, *[quirk]* leak) and reload all flora types (§4.1 step 2, including the FloraTypesUpdated post). Then, if `inst = 0x1A2FDB6A`, re-read the tuning (§7). The group is never checked *[quirk]*. |
| anything else | ignored |

### 5.2 Cheat `Flora …` *[compat]*

The cheat line is split into words (`cRZCmdLine`); word 1 is lower-cased and compared:

| Word 1 | Effect |
|---|---|
| `on` | enabled := true (flora already removed is not restored). |
| `off` | **EnableFlora(false)**: if enabled was true, set it false and demolish **every** flora occupant in the city (§6.10.4). |
| `preserve` [`off`] | preserve := true; if word 2 (lower-cased) is `off`, preserve := false. |
| `propogation` [`off`] (sic) | propagationForced := propagation := true; with `off`, both := false. |
| `blast` n m s | n, m, s := words 2, 3, 4 as unsigned integers (not checked against the word count). Repeat n times: centre `cx := u·sizeX`, `cz := u·sizeZ` (two `RandomDoubleUniform`); repeat m times: `x := g·s + cx`, `z := g·s + cz` (two `RandomDoubleGaussian`); if `x ≥ 0`, `z ≥ 0`, `x < sizeX` and `z ≤ sizeZ` (*[quirk]* inclusive in z): `t := GetBestFloraTypeForLocation(x, z)`, `DropSeed(t, x, z)`. |
| other | nothing |

---

## 6. Algorithms *[compat]*

All positions are world metres (x, z); y is altitude.

### 6.1 Elevation index

```
ElevationIndex(x, z, water):
    d := terrain.GetAltitude(x, z) − terrain.GetSeaLevel()     (water types: GetSeaLevel − GetAltitude)
    return clamp(floor(d × elevationScale), 0, 15)
```

`floor` is computed with the 1.5·2²³ float trick on a 32-bit float (`0x006B031C`, `0x006B034F`): with `rhe` as in
crime.md §6 and `v := d × elevationScale`, `n := rhe(f32(v − 0.5f))`, then `n := n + 1` if `float(n + 1) = v`
(exact floor for |v| ≤ 2²²). For 2²² < |v| < 1.5·2²³ the wrong `n` stays on the same side of 0..15, so the clamp
hides it; `v < −1.5·2²³` (not reachable with shipped terrain and scale) would give a large positive `n` and index
15 *[compat]*. `d`, the
product and the floor's steps are `float32` operations (foundations §18): the product is rounded before the
floor, so d = 50 with scale 0.02f gives exactly 1.0 and index 1. Shipped
elevationScale = 16 / 800 = 0.02, so one index step is 50 m; the code default 0.08 gives 12.5 m.

### 6.2 Propagation score

```
Score(type, x, z):
    unknown type → 0.0
    m := weather.GetMoistureType(x, z)                (0..15, all of EAX is used as an index)
    e := ElevationIndex(x, z, record.water)
    return record.table[e][m] × (1/255 as f32)        (x87 result in ST0)
```

*[compat]* **The moisture value's width** (verified, FQ5). The weather's `GetMoistureType` (slot 19, `0x007256D0`)
returns its own `GetMoistureValue(x, z)` (slot 17, `0x00726150`: a zero-extended byte, 0 off the grid) shifted
right by 4, so EAX is always 0..15 with the upper 24 bits zero. `Score` (`0x006B0939`–`0x006B095B`) and
`GetBestFloraTypeForLocation` add the whole EAX to `16·e` as the byte offset into the 256-byte table. Against the
original weather, reading only AL therefore gives the same index; a FreeCity4 weather must still return the
clean 32-bit value (§13.2), because original callers use all of EAX.

**No weather simulator.** `Init` takes the weather from the game's cached service pointer and holds it only if
it is non-null (`Shutdown` releases it only if non-null), but neither `Score` (`0x006B0900`) nor
`GetBestFloraTypeForLocation` (`0x006B07B0`) tests it: both call `GetMoistureType` through it, so without a
weather simulator the original reads a vtable through a null pointer and crashes at the first score: the first
monthly visit of a flora occupant that is not reborn, the first seed drop, or the first type-0 / `blast` planting.
The shipped city always has its weather component (foundations §3.6, slot 38). *[choice]* FreeCity4 uses
moisture type **0** when there is no weather simulator, the value an uninitialised weather simulator returns
(`GetMoistureValue` is 0 until `Init`, weather §4).

### 6.3 Best type for a location (`GetBestFloraTypeForLocation(x, z, &score)`)

Walk the type map in ascending type-ID order (including the key-0 default record, §3.2, which FreeCity4's flags
choice always skips). Skip records with flag bit 0. Compute the score with `m := GetMoistureType(x, z)` and the elevation index for the record's water flag.
Keep the record if its score is **strictly greater** than the best so far (start: type 0, score 0). Store the
best score, return the best type (0 if none scores above 0).

### 6.4 Location check (`LocationIsOKForNewFlora(type, x, z, exclude)`)

```
record := map[type]; none → false
(h, onLot) := lotManager.GetLotSavvyTerrainAltitude(x, z)
if not onLot:
    if h < seaLevel and not record.water: return false
    n := terrain.GetFaceNormal(x, z)            (output initialised to 0)
    if n.y ≤ 1 − record.slopeTolerance: return false
box := min (x − r, h − 16, z − r), max (x + r, h + record.height, z + r),  r = record.clearanceRadius
if min.x < 0 or min.z < 0 or city.SizeX() < max.x or city.SizeZ() < max.z: return false
for each occupant in occupantManager.GetOccupantsByBBox(min, max, blockingFilter):
    if occupant ≠ exclude and Intersects(box, occupant.GetBoundingBox()): return false
                                    (remaining occupants are still released)
return true
```

On a lot the sea-level and slope tests are skipped. Return in AL only.

`Intersects` is `SC4GeometryUtilities::BoundingBoxesIntersect(aMin, aMax, bMin, bMax, outMin, outMax)`
(`0x005D6D80`, cdecl, result in AL) called with a = the clearance box, b = the occupant's box and two **null
output pointers** (the earlier text called them "flags 0, 0"; they receive the overlap box when the first is
non-null). *[compat]* The test is **strict** on all three axes, in the order x, y, z:
`aMin < bMax` and `bMin < aMax` for each axis (`float32` compares). Boxes that only touch, for example
`box.max.x = occupant.min.x`, do **not** intersect, so an occupant whose box ends exactly at the clearance
radius does not block. An unordered compare (NaN) counts as overlapping on that axis.

#### 6.4.1 Blocking filter

`cSC4FloraBlockingOccupantFilter` accepts occupant types flora `0x74758926`, building `0x278128A0`, landfill
`0x6A11BD2B` (the landfill occupant type, landfill.md §2.3) and prop `0xA823821E`, plus network occupants (`0xC772BF98`) whose
`cISC4NetworkOccupant::GetNetworkFlag()` & `0x1F4F` ≠ 0 (road, rail, highway, street, avenue, light rail,
monorail, one-way road, dirt road, ground highway; not pipes, power lines or subway *[inferred from the SDK
enum]*). Everything else does not block flora.

*[compat]* **The filter object and how the query uses it** (verified, FQ4). `Init` builds it on the game heap
(12 bytes, two vtables); the simulator holds the `cISC4OccupantFilter` part, vtable `0x00AB1374`: slots 0–2
QI / AddRef / Release, **slot 3 `+0x0C` `bool IsOccupantIncluded(cISC4Occupant*)`** = the test above
(`0x006AFD10`, 4 bytes popped, AL), slots 4 and 5 `+0x10` / `+0x14` take one argument and return true
(`0x004FC900`). For a network occupant the test asks the occupant for `cISC4NetworkOccupant` (IID `0xA821EF94`);
if it does not answer, the occupant does not block; the interface is released after `GetNetworkFlag()`. The
box query (occupant manager slot 36) calls **only slot 3**, once per candidate occupant, before its box test.
The typed query (slot 37, `0x0049DD40`) is slot 36 with the manager's own one-type filter (type 0 = no
filter; an uninitialized manager returns false). So one typed query per accepted type (flora, building,
landfill, prop) plus a network query keeping flag & `0x1F4F` ≠ 0 returns **the same occupants**, grouped by type
instead of in the manager's cell order. `LocationIsOKForNewFlora` does not depend on the order: it fails if any
of them overlaps, and every reference is released either way.

#### 6.4.2 What an occupant-manager box query returns *[compat]*

Every flora query ends in occupant manager slot 36 (`0x004A0670`): the box queries of §6.4 and §6.5, the typed
query (slot 37) and the cell queries (slot 41 → 40 → 36, §6.10.4, used by the monthly visit). Slot 36 does not
test every occupant; it reads a coarse index (verified, FQ6):

- **The index.** 192 × 192 cells of 64 m; city (0, 0) is cell (64, 64). A coordinate `p` maps to
  `cell(p) := rhe(f32(f32(p + 4096) × (1/64)) − 0.499f)` (slot 15, `0x0049E0E0`; `rhe` = round half to even, crime.md
  §6; `float32` constants 4096, 0.015625, 0.499 at `0x00A8F7C0`, `0x00A8F7B8`, `0x00A8F7BC`). That is
  `floor((p + 4096) / 64)`, except that a coordinate **less than 64·(0.5 − 0.499f) ≈ 0.06399918 m below a 64 m
  line maps to the cell above** the line.
- **Listing** (`InsertOccupant` slot 20 `0x0049EF60`, `MoveOccupant` slot 24 `0x0049F3C0`): take the occupant's
  box at that moment; on x and on z separately, if `max − min > 1.0` (`0x00A81228`), move both ends inwards by
  `0.1f` (`0x00A8F768`, `float32` add and subtract). The occupant is listed in every cell from
  `(cell(min.x), cell(min.z))` to `(cell(max.x), cell(max.z))`. y plays no part.
- **Query**: map the query box's two corners to cells, clamped to 0..191 (`0x0049DF00`). The candidates are the
  occupants listed in that cell rectangle, each taken once. A candidate is returned if the filter (if any)
  accepts it (§6.4.1) and its **current** box overlaps the query box **strictly** on x, y and z:
  `occMin < qMax` and `qMin < occMax` (`float32` compares; an unordered compare, NaN, counts as overlapping), the
  same rule as §6.4's `Intersects`. Boxes that only touch are **not** returned. The last argument is a count
  limit (−1 = none; 0 returns false at once); an uninitialized manager returns false.

So "in the box" means: listed in one of the query's 64 m cells **and** strictly overlapping. The two differ for
a box wider than 1 m whose low edge lies in `[X − 0.164, X − 0.1)` below a 64 m line `X`: its listing starts at
`edge + 0.1 ≥ X − 0.064`, in the cell above `X`, so a query box that ends at `X − 0.1` (for example the monthly
block just below `X`, §6.7.1) overlaps it strictly but never returns it. Example: a box z 319.844 … 337.844 is
listed from 319.944 (0.056 below 320, so z cell 69, city z 320 … 384); the block query z 288.1 … 319.9 reads z cell
68 only and does not return it, although 319.844 < 319.9. Lines at odd multiples of 32 m lie inside a cell and do
not matter.

### 6.5 Planting (`AddNewFloraOccupant(type, pos, force)` → occupant or null)

`pos` is a caller-owned float[3] **that the function rewrites** *[quirk]*.

1. If `type` is not registered and ≠ 0: return null. If `type` = 0: `type := GetBestFloraTypeForLocation(pos.x,
   pos.z)`; 0 → return null. (The `enabled` flag is not consulted.)
2. Unless `force`: **cluster merging**, if the type has a record (otherwise go to step 3 with `pos` unchanged):

   ```
   loop:
       rec := map[type]; none → leave the loop
       n := 1;  S := 2·pos
       h := terrain.GetAltitude(pos.x, pos.z)
       for each flora occupant o in occupantManager.GetOccupantsByBBox(
               (pos.x − rec.clusterRadius, h − rec.clusterHeight, pos.z − rec.clusterRadius),
               (pos.x + rec.clusterRadius, h + rec.clusterHeight, pos.z + rec.clusterRadius), type 0x74758926):
           t' := AdvanceClusterType(o.floraType, type)        (§6.11)
           if t' ≠ 0:
               c := o.boundingBox.min + o.boundingBox.max     (twice its centre)
               if LocationIsOKForNewFlora(t', (S.x + c.x)/(2(n+1)), (S.z + c.z)/(2(n+1)), exclude = o):
                   occupantManager.RemoveOccupant(o, true, 0)
                   S.x += c.x;  S.z += c.z;  type := t';  n += 1
       pos := (S.x / (2n), −1, S.z / (2n))
       if n ≤ 1: leave the loop
   ```

   Then if `LocationIsOKForNewFlora(type, pos.x, pos.z, null)` fails, return null (*[quirk]* merged occupants are
   already gone).
3. Create a `cSC4FloraOccupant`; set its type; **orientation := `rng.RandomSint32RangeUniform(0, 4)`** (0..3).
4. If `pos.y ≤ 0`: `pos.y := lotManager.GetLotSavvyTerrainAltitude(pos.x, pos.z)`, raised to the sea level for
   non-water types. If `pos.y < 0` now: release the occupant, return null.
5. Set its static record (if the type has one), its position (`cISC4Occupant::SetPosition(pos)`), birth date and
   last seeding date := `simulator.GetSimDateNumber()`; `cISC4Occupant::Init()`.
6. Own slot 18 → `occupantManager.InsertOccupant(occ, 0)`. On success the result is the occupant's
   `cISC4Occupant` pointer (**borrowed**: the occupant manager holds the reference). The local reference is
   released either way; failure returns null.

With `force = true` steps 2 and the location test are skipped and a positive `pos.y` is used as given.

### 6.6 Seed drop (`DropSeed(type, x, z)` → bool)

```
if not LocationIsOKForNewFlora(type, x, z, null): return false
s := Score(type, x, z)
if s < rec.minScore: return false
u := rng.RandomDoubleUniform()
if (s − rec.minScore) × u < (1 − rec.minScore) × rec.seedProbability:
    AddNewFloraOccupant(type, {x, 0, z}, false);  return true     (true even if planting fails)
return false
```

*[quirk]* The acceptance probability **falls** as the score rises: `min(1, (1 − min)·p / (s − min))`.

### 6.7 Monthly simulation

Runs on SimNewMonth only while `enabled`.

#### 6.7.1 Existing flora

The iterator walks 2×2-cell blocks `(bx, bz)`, `0 ≤ bx < cellsX/2`, `0 ≤ bz < cellsZ/2`, in a shuffled order
(table mode; packed `(bx << 8) | bz` 16-bit entries, so block coordinates must stay ≤ 255).

```
k := blocksPerMonth                       (= ((cellsX·cellsZ) div period) >> 2)
while k > 0:
    if not iterator.next(bx, bz): iterator.reset(shuffle = true); continue      (k unchanged)
    for each flora occupant o in GetOccupantsByStandardCityCells((2bx, 2bz), (2bx+1, 2bz+1), type flora):
        if o.IsFlagSet(1) and o answers cISC4FloraOccupant: SimulateExisting(o)
    k −= 1
```

A full pass therefore takes about `period` months (24 shipped).

*[compat]* **Which blocks visit a plant** (verified, FQ15). A block's query is the city-cell box
`(32bx + 0.1, 32bz + 0.1) … (32bx + 31.9, 32bz + 31.9)` (§6.10.4), answered by the occupant manager's index
(§6.4.2). A plant whose box spans several blocks is visited once by each block that returns it, at that block's
place in the shuffled order. So a plant straddling a block edge is normally visited twice (or four times at a
corner) per pass. **The exception is §6.4.2's window:** if the box is wider than 1 m on an axis and its low edge
`m` on that axis lies in `[X − 0.164, X − 0.1)` below a 64 m line `X` (`X` a multiple of 64 in city metres),
the block that ends at `X` overlaps the box but does not return it, and only the blocks from `X` up visit it.
Testing every flora box directly against the block box adds those extra visits. With the shipped wild clumps
the extra visits are visible, because their lifespans are 2–42 days against a visit every
~`period` months: every visit by the block below finds the plant past its lifespan and gives it a rebirth
(birth := that day, state 0) that the original does not perform. The shipped first in-game shadow (2048 m city,
4096 blocks, 170 per month, propagation off) had six such trees, and they are the only plants that differed
(23 differing tree-months in 94 months):

| Occupant | Type | Position (x, z) | Box half-width | Axis | Line `X` | `X − m` | Months differing |
|---|---|---|---|---|---|---|---|
| `0x14C279F4` | `0x2A0C8216` Oak Tree clump 5 | (1726.9807, 328.84396) | 9 | z | 320 | 0.1560 | 7 |
| `0x14C21600` | `0x2A0C8214` Oak Tree clump 3 | (1605.8865, 560.6062) | 6 | x | 1600 | 0.1135 | 4 |
| `0x1492F0F8` | `0x2A0C8215` Oak Tree clump 4 | (327.88672, 151.14545) | 8 | x | 320 | 0.1133 | 3 |
| `0x14BFBB08` | `0x16800000` LotFlora5x5x10_Pines1 | (1024.876, 1371.5037) | 1.0 | x | 1024 | 0.1240 | 1 |
| `0x14C2F268` | `0x2A0C8216` Oak Tree clump 5 | (1736.8553, 401.34277) | 9 | x | 1728 | 0.1447 | 4 |
| `0x14BEA68C` | `0x2A0C8213` Oak Tree clump 2 | (977.7621, 131.8909) | 4 | z | 128 | 0.1091 | 4 |

*Example:* Oak Tree clump 5 `0x14C279F4` has the box x 1717.98 … 1735.98, z 319.84 … 337.84. It is listed in
manager cells x 90–91, z 69 (from z 319.944, §6.4.2). Blocks (53, 10) and (54, 10) (z 320 … 352) visit it; blocks
(53, 9) and (54, 9) (z 288 … 320) query z 288.1 … 319.9, which reads manager z cell 68 only, so they do not, although
319.84 < 319.9. A tree 0.2 m below the line (box z 319.8 … 337.8, listed from 319.9 in z cell 68) is visited by all
four blocks.

#### 6.7.2 One flora occupant (`SimulateExistingFloraOccupant`)

```
age := today − birthDate                  (u32; used as an unsigned→float value)
rec := map[floraType]; none → return
state := GetState()
if state ≥ 3: stage := reset
else:
    stage := 4 if age ≥ rec.maxAge, 3 if age ≥ rec.lifespan, 2 if age ≥ rec.maturity,
             1 if age ≥ 0.5·rec.maturity, else 0
if stage ∈ {3, 4} or state ≥ 3 ("reset"):
    birthDate := today;  state := 0;  post 0x99EF1145;  return            (rebirth as a young plant)
s := Score(floraType, position.x, position.z)
canSeed := false
if propagation or flag bit 1 is clear:
    if s < rec.minScore: stage := 4
    else if age ≥ rec.seedingAge: canSeed := true
if stage ≠ state:
    state := stage
    stage 1, 2 → post 0x99EF1145
    stage 0, 4 → occupantManager.RemoveOccupant(o, true, 0)
if propagation and canSeed:
    now := simulator.GetSimDateNumber()
    if lastSeedingDate + rec.seedingInterval < now:          (float compare)
        repeat rhe(rec.seedsPerCycle) times:                  (crime.md §6 rhe, 0x006B0C8A; none if ≤ 0)
            x := rng.RandomDoubleRangeUniform(pos.x − rec.spread, pos.x + rec.spread)
            z := rng.RandomDoubleRangeUniform(pos.z − rec.spread, pos.z + rec.spread)
            if 0 ≤ x < sizeX and 0 ≤ z < sizeZ: DropSeed(rec.seedType, x, z)
            lastSeedingDate := now
```

Notes *[compat]*: the dying stage (4) comes **only** from a low score; old age always leads to rebirth. Burnt
flora (state 3) that is still standing is reborn as a young burnt tree on its next visit *[quirk]*. Stage 0
after a non-zero state removes the plant (only reachable if the birth date moved forward). The seed type is the
record's computed seed type (§6.11), not necessarily the plant's own type.

#### 6.7.3 Wild seeds

After the block visits, if `propagation` and the terrain is initialized (terrain slot 5) and
`wildSeedsPerMonth > 0`:

```
k := wildSeedsPerMonth
while k > 0:
    t := typeArray[rng.RandomUint32Uniform(typeArray.size)]
    if map[t].flags bit 0: continue                   (k unchanged)
    x := rng.RandomUint32Uniform(trunc(sizeX));  z := rng.RandomUint32Uniform(trunc(sizeZ))
    DropSeed(t, x, z);  k −= 1
```

*[quirk]* The loop never ends if every registered type has flag bit 0; with an empty type array it reads
outside the array. Shipped tuning sets wildSeedsPerMonth to 0, so wild seeding is off in the shipped game.

### 6.8 Daily burnt-flora removal

```
burnAccumulator += burnRate
if burnAccumulator ≥ 1: burnAccumulator −= 1;  burntRemovalAccumulator += burntRemovalStep
if burntRemovalAccumulator ≥ 1:
    n := floor(burntRemovalAccumulator);  burntRemovalAccumulator −= n      (the §6.1 floor sequence, 0x006B1736)
    repeat n times while the burnt list is not empty:
        o := pop front;  occupantManager.RemoveOccupant(o, true, 0);  release o
```

At most one step per day *[quirk]*: with `burnRate > 1` the first accumulator grows without bound. Shipped
values remove **one burnt tree per day**; the code defaults (no tuning exemplar) remove four.

### 6.9 Wildfire (`KillFloraOccupant(occ)` → bool)

Called by the fire disaster (`cSC4DisasterInstanceFire::ToastOccupant`) for each burning occupant. If `occ` does
not answer `cISC4FloraOccupant`, or `IsFlagSet(1)` is false: return false. If its state is already 3: return
true. Otherwise: state := 3, flora type := **`0x29CFF3CB`** (Burnt Tree), static record := that type's record
(null if absent), post `0x99EF1145`, `AddRef` and append to the burnt list; return true. The burnt tree stays
until the daily removal (§6.8) or another removal path.

### 6.10 Removal and relocation

#### 6.10.1 Buildings and networks (`InsertOccupant`)

- Building (`0x278128A0`): `DemolishFloraInBoundingBox(min − (2, 2, 2) with min.y := −2, max + (2, 0, 2) with
  max.y := 2 + 1.7e38, 0)`: every flora within 2 m of the building's box, at any height, is removed.
- Network occupant accepted by the blocking filter (§6.4.1): the same with margin 0 (y from 0 upwards).
- Other types: nothing. Zoning alone removes nothing; lots remove flora only through their buildings
  (and the network-lot manager, §13.1).

#### 6.10.2 Terrain changes (`DoTerrainAltitudeChange(rect)`)

For each flora occupant in the cell rectangle (`GetOccupantsByStandardCityCells`):
`h := GetLotSavvyTerrainAltitude(pos.x, pos.z)`. If `preserve`, or `|h − pos.y| < 0.001`: **re-seat**; otherwise
`RemoveOccupant(o, true, 0)`.

Re-seat: `h' := GetLotSavvyTerrainAltitude(pos.x, pos.z)`, raised to the sea level for non-water types; if
`|h' − pos.y| ≥ 0.001`: `pos.y := h'`, `SetPosition`, `occupantManager.MoveOccupant(o, true)`.

So without the `preserve` cheat, any flora whose ground moved is removed.

*[compat]* Arithmetic (foundations §18): the tolerance is the **`float32`** constant `0.001f` (`0x3A83126F` =
0.0010000000474974513, `.rdata 0x00A867A4`, loaded with `fcomp dword`). `h` and `h'` arrive in ST0 (lot manager
slot 18); `h − pos.y` (`DoTerrainAltitudeChange`, `0x006B1B9F`) and `h' − pos.y` (re-seat, `0x006B1288`) are one
24-bit subtraction each, then `fabs`, then the compare. Removal/move happens when `|d| ≥ 0.001f` or `d` is NaN
(re-seat: a NaN `d` does **not** move). Because `d` is itself a `float32` value and no `float32` lies strictly
between the double 0.001 and `0.001f` (the next value below is `0x3A83126E` = 0.00099999993), "`|d| < 0.001f`"
and "`|d| < 0.001`" give the same result for every input; an implementation may use either constant.

#### 6.10.3 Demolition functions

`DemolishFloraOccupant(occ, flags)` = `occupantManager.RemoveOccupant(occ, true, 0)` for **any** occupant (no
type check; `flags` ignored). `DemolishFloraInCellArea(rect, flags)` and `DemolishFloraInBoundingBox(min, max,
flags)` query flora occupants (cell rectangle / box) and call slot 19 on each **with flags 0** (the argument is
ignored *[quirk]*). They return true if every call returned true (true for an empty area).

#### 6.10.4 `EnableFlora(false)`

Only on a true → false change: every flora occupant of the city (`GetOccupantsByStandardCityCells(null, null,
type flora)`) is passed to slot 19 with flags 0.

**Null corners** *[compat]* (occupant manager, original code): `GetOccupantsByStandardCityCells` (slot 41,
`0x0049DDC0`) sets its type filter (type 0 → no filter) and continues in the filter version (slot 40,
`0x0049E250`). That returns false at once if the occupant manager is not initialized. A null **min** corner
means cell (0, 0); a null **max** corner means cell `(nx, nz)`, the manager's standard-cell counts (not
`n − 1`). Given corners are clamped to `[0, n − 1]` per axis. The cell box becomes the world box
`min = (16·x0 + 0.1f, −FLT_MAX, 16·z0 + 0.1f)`, `max = (16·(x1 + 1) − 0.1f, +FLT_MAX, 16·(z1 + 1) − 0.1f)`
(each a `float32` add or subtract) and the call goes on as `GetOccupantsByBBox` (slot 36). With null corners the
box therefore covers the whole city and one cell beyond its far edges: **every** flora occupant of the city.
`GetOccupantsByOccupantManagerCells` (slot 39, `0x0049DD80` → slot 38 `0x004A0A70`, used by the obliteration save,
§10.3) treats null corners the same way on its own 64 m grid: a null min is that grid's first city cell
(64, 64) and a null max its last (`count + 63`); given corners are clamped to 0..191 *[inferred: the 192-cell grid
has a 64-cell margin around the city]*. Both therefore mean "the whole city".

### 6.11 Cluster chains and seed types

Each type may name a **next cluster type** (e.g. single tree → small cluster → large cluster).

`AdvanceClusterType(a, b)`: if `b` is reachable from `a` by following next links (including `a = b`), return
`next(b)`; else if `a` is reachable from `b`, return `next(a)`; else 0. Unknown types stop the walk.
*[quirk]* A cycle in the chain loops forever.

Seed type, computed in `AddStaticFloraData(record)`: `record.seedType := record.type`; then **every** registered
record whose seed type equals the new record's next type gets the new type as seed type. The original also
compares "other.next = new type" and then assigns the same value (a no-op, also in the Mac build). Result
*[quirk]*: clusters seed the head of their chain only if the head was registered after them; otherwise they
seed their own type.

*[compat]* **Next type 0** (verified, FQ14; `AddStaticFloraData` slot 6 `0x006B3570`, loop `0x006B3725`–
`0x006B3776`). There is no special case for 0: the comparison is "other's seed type = new record's next type",
whatever the value, over every map entry including key 0. The only record whose seed type is 0 is the key-0
default record (§3.2), so a new record with next type 0 sets key 0's seed type to its own type, if key 0 is in
the map at that moment. **In the game it never is:** the first load (§4.1 step 2) runs before `Init` inserts
key 0 (step 8), and the exemplar-change reload (§5) clears the map, key 0 included, before loading and does not
put key 0 back. So in the game key 0 keeps seed type 0. Only a registration made after `Init` and outside those
loads (a plugin calling slot 5 or 6, or a test) can change it. *Example:* after `Init`, register A (next 0)
then B (next 0): A's seed type is A and key 0's becomes A; B's seed type is B and key 0's stays A (it no longer
equals 0). Nothing reads key 0's seed type, since no flora occupant has type 0 (§6.5 step 1).

---

## 7. Tuning *[compat]*

Exemplar `0x6534284A / 0x7A4A8458 / 0x1A2FDB6A` "Flora Tuning Parameters" (SimCity_1.dat; not in
`re/tuning/simulators.md`, read with `tools/dbpf.py`). If the exemplar is missing nothing changes. Each property
is read only if present (getters `cSCPropertyHelp` float / u32).

| Property | Type | Use | Code default | Shipped |
|---|---|---|---|---|
| `0xC9DBDD00` | f32 | burnRate := 1 / v | rate 1.0 | 1 → 1.0 |
| `0xC9DBDD01` | f32 | burntRemovalStep := v | 4.0 | 1 |
| `0x8761ABD0` | u32 | period := v (months per pass) | 6 | **24** |
| `0x085489C7` | u32 | wildSeedsPerMonth := v (then scaled) | 5 | **0** |
| `0xC9DBDD02` | f32 | elevationScale := 16 / v | 0.08 (= 16 / 200) | 800 → 0.02 |

All properties are unnamed. After reading (only when the exemplar exists):

```
blocksPerMonth := ((cellsZ × cellsX) div period) >> 2            (unsigned; period 0 → divide fault)
wildSeedsPerMonth := u32( trunc( float(u32(trunc(float(wildSeedsPerMonth) × sizeX))) × sizeZ ) ) >> 20
```

i.e. seeds per 2²⁰ m² (one 1024 m × 1024 m city). The scaling rewrites the stored value; see §14 quirk 11.

**Elevation scale** (verified, FQ7; tuning read `0x006B00F0`). The constructor stores `0.08f` (`0x3DA3D70A`,
`0x006B1941`). The tuning read sets `elevationScale := 16.0f / v` (constant `0x00A8D45C`, one `float32` division,
`0x006B01B0`) only when the exemplar loads **and** has `0xC9DBDD02`; otherwise the value stays. The read runs at
every `Init` (§4.1 step 9) and when the exemplar `0x1A2FDB6A` changes (§5). The value is **not** in the save
record (§10.1: neither `Write` `0x006AFEE0` nor `Read` `0x006AFFE0` touches it), so a loaded city always uses the
running game's tuning. Shipped: 800 → `16 / 800` = `0x3CA3D70A` (0.0199999996).

**Period 0** *[quirk]*: `blocksPerMonth` is an x86 `div` by `period` in two places, `Init` step 6
(`0x006B2B53`, with the constructor's or the **saved** period) and this tuning read (`0x006B01CB`, with the value
just read). A period of 0 (tuning `0x8761ABD0 = 0`, or a save that holds 0) raises the integer divide-by-zero
exception there and the game crashes; it is not reachable with shipped data (6 by code, 24 shipped). No other
code divides by the period. *[choice]* FreeCity4 computes `blocksPerMonth` with a divisor of 1 when the period is
0 (a full pass every month) and keeps and saves the period as 0.

**No response curves** are loaded (foundations §4.1 patterns do not apply).

Per-type data comes from the flora exemplars (§3.3), loaded once per process.

---

## 8. Random numbers *[compat]*

One generator: the simulator's own `cRZRandom`, seeded from the clock at construction and **saved and
restored** with the city (like the weather, unlike foundations §6.2's general rule).

| Draw | Method | Count |
|---|---|---|
| orientation of every created flora occupant | `RandomSint32RangeUniform(0, 4)` | 1 integer step per occupant created |
| seed acceptance | `RandomDoubleUniform` | 1, only when the location is OK and the score ≥ minimum |
| seed position around a plant | `RandomDoubleRangeUniform(p − spread, p + spread)` | 2 per seed (x then z), drawn even when out of bounds |
| wild seed type | `RandomUint32Uniform(typeCount)` | 1 per round, also for skipped types |
| wild seed position | `RandomUint32Uniform(trunc(sizeX))`, then `(trunc(sizeZ))` | 2 per accepted round |
| `blast` cheat centre / offsets | `RandomDoubleUniform` × 2 per centre; `RandomDoubleGaussian` × 2 per seed | |
| block order | C runtime `rand()` through `random_shuffle` | `n − 1` per iterator reset (Init and each completed pass) |

---

## 9. Messages posted

| ID | When | Fields |
|---|---|---|
| `0x87A6277F` FloraTypesUpdated | after every (re)load of all flora types | `MessagePost` (queued, normal priority); void1 = IGZUnknown = the simulator (primary interface); data2 = number of map entries |
| `0x99EF1145` ChangeOccupant | rebirth, stage 1/2 changes, burning | through the occupant's `PostOccupantMessage(0x99EF1145, 0)` (§9.1) |

Insert and remove messages for flora come from the occupant manager (`InsertOccupant`/`RemoveOccupant`).

### 9.1 Occupant messages (`cISC4Occupant::PostOccupantMessage`, slot 15) *[compat]*

The flora occupant uses the base implementation `0x005E9010` (its `cISC4Occupant` vtable `0x00A88580`, slot 15;
the overrides of other occupant classes, `0x00458D10`, `0x0065A980`, `0x00654180`, call it first and then post a
second, class-specific message). `PostOccupantMessage(type, value)` builds one standard message:

| Field | Value |
|---|---|
| type | `type` |
| void1 | the occupant (`cISC4Occupant*`) |
| IGZUnknown | the occupant (counted) |
| data2 | `IsFlagSet(4)` as 0/1 |
| data3 | the occupant's **type** (`GetType()`, slot 7): `0x74758926` for flora |
| data4 | `value` |

It is **posted** (`MessagePost(msg, false)`, queued, normal priority) for every type, except `0x99EF1146`, which
is sent synchronously when a member of the occupant (offset `+0x30` of the interface) is non-null. So the
`0x99EF1145` of §6.7.2 and §6.9 reach flammability and the other subscribers on a later message-server tick,
after the simulator has finished the current visit.

The occupant manager's `InsertOccupant(occ, v)` (slot 20, `0x0049EF60`) sets the occupant's flag 1 and then calls
`occ.PostOccupantMessage(0x99EF1142, v)`; `RemoveOccupant(occ, …, v)` (slot 21, `0x0049F240`) calls
`occ.PostOccupantMessage(0x99EF1143, v)` and then clears flag 1. **data3 of InsertOccupant / RemoveOccupant is
therefore the occupant type** (the handlers of §5 read it with `GetData3`), and both messages are delivered
after the fact, when flag 1 is already set (insert) or already clear (remove).

---

## 10. Persistence

### 10.1 `cIGZSerializable` record, `GetGZCLSID() = 0xC990C034` *[compat]*

| # | Type | Field |
|---|---|---|
| 1 | u16 | version = **2** |
| 2–5 | u8 × 4 | enabled, propagation, propagationForced, preserve (bool as u8) |
| 6–7 | f32 × 2 | sizeX, sizeZ |
| 8–9 | s32 × 2 | cellsX, cellsZ |
| 10–13 | u32 × 4 | today, period, blocksPerMonth, wildSeedsPerMonth |
| 14 | u8 | keepIteratorFlag |
| 15 | iterator | `s8 state, s8 mode, u8 dirty, s32 x0, z0, x1, z1, s32 count, u32 n, n × u16 table, s32 index, s32 current, s32 step` (weather §10) |
| 16 | rng | `u32 lcg, u32 mul, s32 cachedFlag, f64 cachedGaussian` |
| 17 | list | `u32 count`, then `count` occupant refs (foundations §2.7) |
| 18–19 | f32 × 2 | burnAccumulator, burntRemovalAccumulator |

Write returns true iff the stream has no error (EAX 0/1). **Read** rejects any version ≠ 2 (AL = 0, nothing more
read). Bools read as "non-zero → true". The list reader clears the list, reads the count, and reads nothing more
if the count is 0 or ≥ 10,000,000; each ref is resolved with IID `0x26BB767E`. Read returns `GetError() == 0`.

Size for a new 64 × 64 city with no burnt flora: 39 + (39 + 2 × 1024) + 20 + 4 + 8 = **2158 bytes** (128²:
8302; 256²: 32878).

### 10.2 What `Init` recomputes on a loaded city *[compat]*

`Init` runs after `Read`. It overwrites sizes, cell counts and blocksPerMonth from the city, re-reads the tuning
(period, wild seeds), recomputes `propagation` from the city's established state, and — because
keepIteratorFlag is never non-zero — **resets the iterator** (new rectangle, refill, reshuffle). Effective saved
state: enabled, propagationForced, preserve, today, the generator, the burnt list and the two accumulators. The
static registry and the elevation/burn tuning are not saved.

### 10.3 Obliterated-city flora

God-mode "Obliterate City": the application calls `cSC4City::SaveObliterated(segment)`, which calls the terrain
and then flora slot 24 `SaveObliterated(segment)`:

- record key (type `0xC9BB3FC6`, group `0xC9BB3FC7`, instance 1): `u32 count` of all flora occupants
  (`GetOccupantsByOccupantManagerCells(null, null, type flora)`);
- for each occupant `i` (list order): record (`0xC9BB3FC6`, `0x4A664F83`, `i`): the occupant clears its flag 1, writes
  its own serialized form (`cIGZSerializable::Write`), then sets flag 1 again. All three happen only when the record
  opened (AQ4; test `0x006B2886`, calls `0x006B2890`–`0x006B28AB`), so an occupant whose record does not open keeps
  its flags. Its list reference is released either way (`0x006B28B2`).

The replacement city's `Init` finds the segment in `SC4NewCitySpecification` (§4.1 step 12) and, if the occupant
manager exists: reads the count; for each `i < count` whose record opens: create a `cSC4FloraOccupant`, `Read`
it (result ignored), `cISC4Occupant::Init()`, `InsertOccupant(occ, 0)`, release. Returns true (false only
without an occupant manager). So **trees survive obliteration** *[compat]*.

**Record layout and calls** (verified, FQ3; save `0x006B2760`, restore `0x006B0660`, the game's record-stream
helpers `0x005D9F00` / `0x005D9DB0` (write) and `0x005D9E70` / `0x005D9C90` (read)). *Correction:* every record
starts with a **u16 record version, 0**, written by the helper before the payload:

| Record | Bytes |
|---|---|
| count (`0xC9BB3FC6`, `0xC9BB3FC7`, 1) | `u16 0`, `u32 count` (6 bytes) |
| occupant `i` (`0xC9BB3FC6`, `0x4A664F83`, `i`) | `u16 0`, then the flora occupant's own record (lot-records.md §11, which starts with its own `u16` version 3) |

Opening a record for writing: query the given `cIGZPersistDBSegment` for `cISC4DBSegment` (IID `0x499899DA`);
`OpenOStream(key, &stream, true)` (slot 8); write `u16 0` (stream slot 7); if `GetError()` (stream slot 18) is
non-zero, close it again (`CloseOStream`, slot 9) and treat the record as not open. Closing: `CloseOStream`
(slot 9), then both references are released. Reading is the same with `OpenIStream(key, &stream)` (slot 5),
`GetUint16` (slot 7; the value is kept by the helper and **never checked** by flora) and `CloseIStream` (slot 6).
A segment that does not answer the IID, or a key that does not open, gives "not open": nothing is written or read.

- Save: the count record is written even when the count is 0. `i` is the occupant's position in the list and
  advances for every occupant, also when its record does not open. The count is written with `PutUint32`
  (stream slot 9) and the occupant record by the occupant's `cIGZSerializable::Write` on the same stream (the
  segment streams begin with the `cIGZOStream` / `cIGZIStream` slots). Returns true (AL = 1) whatever happened.
- Restore: `count` starts at 0 and is read with `GetUint32` (stream slot 9) only if the count record opens; the count
  is read **before** the occupant-manager test, so without an occupant manager the function returns false after
  reading it. Each occupant: pooled `new` (92 bytes) and constructor, `AddRef`, `Read` from the stream, `Init`,
  `InsertOccupant(occ, 0)` (slot 20), `Release`; records that do not open are skipped. Each occupant record is closed
  only after all of that (AQ5): its helper is destroyed at the end of the iteration (`0x006B073D`), so the record is
  still open during `Init`, `InsertOccupant` and `Release`. Neither touches the segment, so the order cannot be seen.
  The count record, by contrast, is closed right after its `GetUint32` (`0x006B06A6`). There is no upper bound on
  `count` (a damaged count means as many open attempts).

Example: three flora occupants → count record `00 00 03 00 00 00`; records `i` = 0, 1, 2, each `00 00` followed by
that occupant's version-3 record.

---

## 11. Interfaces (compiled slot tables) *[compat]*

### 11.1 `cISC4FloraSimulator`, vtable `0x00AB13C0`, 26 slots (3 + 22 interface + destructor)

Pops from `tools/abi_extract.py --check`; returns from reading each epilogue. **The SDK header
`cISC4FloraSimulator.h` is correct**: the three overload pairs are declared in source order and compile
reversed exactly as foundations §16 predicts, and every argument size and return type matches.

| Slot | Off | Method (as the game expects it) | Pop | Return | Behaviour |
|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface(u32 iid, void** out)` | 8 | AL | §1 |
| 1 | `+0x04` | `AddRef()` | 0 | EAX count | |
| 2 | `+0x08` | `Release()` | 0 | EAX count | §1 |
| 3 | `+0x0C` | `bool Init()` | 0 | AL = 1 | §4.1 |
| 4 | `+0x10` | `bool Shutdown()` | 0 | AL = 1 | §4.2 |
| 5 | `+0x14` | `bool AddStaticFloraData(cGZPersistResourceKey*, cISCResExemplar*)` | 8 | AL | Uses the given exemplar or loads it by key; null → false. Builds a record (§3.3) and returns slot 6's result. |
| 6 | `+0x18` | `bool AddStaticFloraData(cSC4StaticFloraData*)` | 4 | AL | Type already registered → false (no AddRef). Else insert in the map, append to the type array, `AddRef`, add to each family's list (creating families), compute seed types (§6.11); true. |
| 7 | `+0x1C` | `cSC4StaticFloraData* GetStaticFloraData(u32 type)` | 4 | EAX ptr, borrowed (null if unknown) | |
| 8 | `+0x20` | `void GetAllFloraTypes(SC4Vector<u32>&)` | 4 | none (garbage) | Clears the vector, appends the type array (registration order). |
| 9 | `+0x24` | `u32 GetAllFloraTypes(u32* buffer, u32& count)` | 8 | EAX | Null buffer: `count := N`, return N. Else `count := min(count, N)`, copy that many, return `count`. N = type-array size (excludes key 0). |
| 10 | `+0x28` | `void GetAllFloraFamilyTypes(SC4Vector<u32>&)` | 4 | none | Clears, appends every family ID (ascending). |
| 11 | `+0x2C` | `bool IsFloraTypeValid(u32)` | 4 | EAX 0/1 | In the map (true for 0, §3.2). |
| 12 | `+0x30` | `bool IsFloraFamilyTypeValid(u32 id)` | 4 | AL | A family ID, **or** a registered type ID. |
| 13 | `+0x34` | `u32 GetFloraTypesByFilter(hash_set<u32>&, cISC4FloraTypeFilter*)` | 8 | EAX | Every map key (including 0) accepted by the filter's slot 3 (null filter = all) is inserted; returns the number of insert attempts. |
| 14 | `+0x38` | `u32 GetFloraTypesByFamily(hash_set<u32>&, u32 family)` | 8 | EAX | Inserts the family's types; returns their count (0 for an unknown family). |
| 15 | `+0x3C` | `bool LocationIsOKForNewFlora(u32 type, f32 x, f32 z, cISC4Occupant* exclude)` | 16 | AL only | §6.4 |
| 16 | `+0x40` | `u32 GetBestFloraTypeForLocation(f32 x, f32 z, f32& score)` | 12 | EAX type | §6.3 |
| 17 | `+0x44` | `cISC4Occupant* AddNewFloraOccupant(u32 type, f32* pos, bool force)` | 12 | EAX ptr, borrowed | §6.5 |
| 18 | `+0x48` | `bool AddNewFloraOccupant(cISC4Occupant*)` | 4 | AL | `InsertOccupant(occ, 0)`'s result; false without an occupant manager. |
| 19 | `+0x4C` | `bool DemolishFloraOccupant(cISC4Occupant*, s32)` | 8 | EAX (callee's) | §6.10.3; no null check of the occupant manager. |
| 20 | `+0x50` | `bool DemolishFloraInCellArea(const SC4Rect<s32>&, s32)` | 8 | AL only | §6.10.3; rect = {x0, z0, x1, z1} cells. |
| 21 | `+0x54` | `bool DemolishFloraInBoundingBox(const f32* min, const f32* max, s32)` | 12 | AL only | §6.10.3 |
| 22 | `+0x58` | `int64 GetCostOfOccupantDemolition(cISC4Occupant*, s32)` | 8 | EDX:EAX = 0 | Always 0. |
| 23 | `+0x5C` | `bool KillFloraOccupant(cISC4Occupant*)` | 4 | AL only | §6.9 |
| 24 | `+0x60` | `bool SaveObliterated(cIGZPersistDBSegment*)` | 4 | AL = 1 | §10.3 |
| 25 | `+0x64` | scalar deleting destructor (class, not in the SDK) | 4 | EAX this | Called by `Release`. |

"AL only": the upper 24 bits of EAX are garbage; callers test AL. A replacement should return a clean 0/1 in EAX
*[choice]*. Functions 8 and 10 return nothing.

**Slot 13 with a filter** (verified, FQ9; `0x006B23A0`). The map is walked in ascending type order (key 0
included while it is present, §3.2). For each key, a non-null filter is asked through **`cISC4FloraTypeFilter`
slot 3 `+0x0C`, `bool (u32 floraType)`** (4 bytes popped by the callee, result in AL; no other filter slot is
called); a null filter accepts every key. Each accepted key is inserted into the caller's set (the set is first
grown to hold one more element, `0x004F0940`) and counted, also when the set already held it; the count is
returned (0 for an empty map). Example: map {0, A, B, C}, a filter accepting A and C, a set already holding A →
the set is {A, C}, the result 2. No caller in the exe passes a filter (the two set users, `0x0048B860` and
`0x006CD010`, call slot 14), so only plugins can see the difference.

### 11.2 `cIGZMessageTarget2`, vtable `0x00AB13AC`

Slots 0–2: thunks (`this − 4`) to QI / AddRef / Release. Slot 3 `+0x0C` `bool DoMessage(cIGZMessage2*)`, pop 4,
AL = 1 always (§5).

### 11.3 `cIGZSerializable`, vtable `0x00AB1394`

Slots 0–2: thunks (`this − 8`). Slot 3 `+0x0C` `bool Write(cIGZOStream&)`, pop 4, EAX 0/1. Slot 4 `+0x10`
`bool Read(cIGZIStream&)`, pop 4, EAX 0/1 (AL only on the version-reject path). Slot 5 `+0x14`
`u32 GetGZCLSID()`, pop 0, EAX `0xC990C034`. Write and Read are not thunks: they run on the serializable
sub-object directly.

### 11.4 `cISC4FloraOccupant`, vtable `0x00A88608` (the parts used; 15 slots)

| Slot | Off | Method | Pop | Return |
|---|---|---|---|---|
| 3 | `+0x0C` | `cISC4Occupant* AsOccupant()` (null for a null `this`) | 0 | EAX ptr |
| 4 / 5 | `+0x10` / `+0x14` | `GetFloraType()` / `SetFloraType(u32)` | 0 / 4 | EAX / garbage |
| 6 / 7 | `+0x18` / `+0x1C` | `GetBirthDate()` / `SetBirthDate(u32)` | 0 / 4 | EAX / garbage |
| 8 / 9 | `+0x20` / `+0x24` | `GetLastSeedingDate()` / `SetLastSeedingDate(u32)` | 0 / 4 | EAX / garbage |
| 10 / 11 | `+0x28` / `+0x2C` | `GetState()` (sign-extended byte) / `SetState(s32)` (stores a byte) | 0 / 4 | EAX / garbage |
| 12 / 13 | `+0x30` / `+0x34` | `GetOrientation()` (zero-extended byte) / `SetOrientation(u32)` | 0 / 4 | EAX / garbage |
| 14 | `+0x38` | `SetStaticFloraData(cSC4StaticFloraData*)` | 4 | (tail call) |

SDK `cISC4FloraOccupant.h` matches the slot order. *[SDK note]* the setters are declared `bool` but return
no meaningful bool: slots 5, 7, 9 leave EAX = the argument (`0x006CE240`, `0x004302D0`); slots 11 and 13 load only
AL with the argument's low byte (`0x005D6150`, `0x00472BB0`). Callers ignore the result; a replacement returns true.

---

## 12. Dependencies

Callee-pop and return channel checked with `tools/abi_extract.py --check` and the epilogues.

| Interface (vtable used) | Slot | Off | Method | Bytes | Return |
|---|---|---|---|---|---|
| `cISC4City` (`0x00A87620`) | 22 | `+0x58` | GetEstablished | 0 | AL |
| | 89 / 90 | `+0x164` / `+0x168` | SizeX / SizeZ (metres) | 0 | ST0 float |
| | 93 / 94 | `+0x174` / `+0x178` | cell count X / Z | 0 | EAX |
| `cISC4Simulator` (`0x00AB2FB8`) | 8 | `+0x20` | GetSimDateNumber | 0 | EAX |
| | 19 | `+0x4C` | AddAgent(target, level, name, flags) | 16 | bool |
| | 20 | `+0x50` | RemoveAgent(target) | 4 | bool |
| `cIGZMessageServer2` (`0x00AD1BF4`) | 4 / 5 / 6 | `+0x10/+0x14/+0x18` | MessagePost(msg, false) / AddNotification / RemoveNotification | 8 each | bool |
| `cIGZCommandServer` (`0x00AD17A0`) | 6 | `+0x18` | RegisterCommand(u32 id, const char* name, const char* paramInfo) (`0x0092B075`; false for id 0, null name or an existing id) | 12 | AL |
| | 8 | `+0x20` | RegisterCommandParameterInfo(u32 id, const char* info) (`0x0092AD10`; null or empty → default text when the server's help flag is set, §4.1 step 3) | 8 | AL |
| | 9 | `+0x24` | UnregisterCommand(id) | 4 | bool |
| `cIGZCheatCodeManager` (`0x00ACFCD0`) | 3 / 4 | `+0x0C` / `+0x10` | RegisterCheatCode(id, const cIGZString&) / UnregisterCheatCode(id) | 8 / 4 | bool |
| | 7 / 8 | `+0x1C` / `+0x20` | AddNotification2 / RemoveNotification2(target, id) | 8 / 8 | bool |
| `cISC4App` (global `0x00B43C94`) | 39 | `+0x9C` | GetNewCitySpecification → `SC4NewCitySpecification*` (field `obliteratedCityDBSegment` read) | 0 | EAX ptr |
| | 69 | `+0x114` | RegisterShutdownCallbackFunction(cb, data) | 8 | bool |
| `cISC4DBSegment` (IID `0x499899DA`, vtable `0x00AA2654`; queried from the `cIGZPersistDBSegment`, §10.3) | 5 / 6 | `+0x14` / `+0x18` | OpenIStream(const key&, IStream**) / CloseIStream(IStream*) | 8 / 4 | AL / ignored |
| | 8 / 9 | `+0x20` / `+0x24` | OpenOStream(const key&, OStream**, bool = true) / CloseOStream(OStream*) | 12 / 4 | AL / ignored |
| `cISC4DBSegmentIStream` (`0x00AA2280`) / `…OStream` (`0x00AA2310`) | 7 / 9 / 18 | `+0x1C` / `+0x24` / `+0x48` | Get-/PutUint16, Get-/PutUint32 (4 bytes each, EAX 0/1); GetError() (EAX 1 = error, sticky) | 4 / 4 / 0 | EAX |
| `cIGZPersistResourceManager` (`0x00AD89D0`) | 3 | `+0x0C` | GetResource(key, `0xA52160F5`, out, 0, 0) | 20 | AL |
| | 17 | `+0x44` | GetAvailableResourceList(list**, key filter) (slot 18 = slot 17 with a type filter, §4.1 step 2) | 8 | EAX count |
| | 18 | `+0x48` | GetAvailableResourceListForType(list**, `0x6534284A`) | 8 | EAX |
| `cIGZPersistResourceKeyList` (`0x00AADA64`) | 7 | `+0x1C` | EnumKeys(callback, context) | 8 | none |
| | 9 / 10 | `+0x24` / `+0x28` | Size() / GetKey(i) → pointer to the i-th key (no bounds check) (FQ1 equivalent) | 0 / 4 | EAX |
| `cISCResExemplar` (`0x00AAF0D0`) / `cISCPropertyHolder` (`0x00AA6CD8`) | foundations §17.2, §17.5 | | AsPropertyHolder; HasProperty (slot 3); GetPropertyValue (slot 8); GetProperty (slot 9) | | |
| `cISTETerrain` (`0x00AB3D18`) | 5 | `+0x14` | initialized flag | 0 | AL |
| | 13 | `+0x34` | GetAltitude(x, z) | 8 | ST0 |
| | 26 | `+0x68` | GetFaceNormal(x, z, cS3DVector3&) (output used) | 12 | ignored |
| | 29 | `+0x74` | GetSeaLevel | 0 | ST0 |
| `cISC4LotManager` (`0x00AB37C0`) | 18 | `+0x48` | **`f32 GetLotSavvyTerrainAltitude(f32 x, f32 z, bool* onLot)`** | 12 | **ST0 float** |
| `cISC4WeatherSimulator` (`0x00AB3610`) | 19 | `+0x4C` | GetMoistureType(x, z) → 0..15 | 8 | **full EAX** used |
| `cISC4OccupantManager` (`0x00A8F7F0`) | 20 | `+0x50` | InsertOccupant(occ, 0) | 8 | AL |
| | 21 | `+0x54` | RemoveOccupant(occ, true, 0) | 12 | AL |
| | 24 | `+0x60` | MoveOccupant(occ, true) | 8 | ignored |
| | 36 | `+0x90` | GetOccupantsByBBox(list, min, max, **filter**, −1) | 20 | AL |
| | 37 | `+0x94` | GetOccupantsByBBox(list, min, max, **type**, −1) | 20 | AL |
| | 39 | `+0x9C` | GetOccupantsByOccupantManagerCells(list, null, null, **type**, −1) | 20 | AL ignored |
| | 41 | `+0xA4` | GetOccupantsByStandardCityCells(list, &min, &max, **type**, −1) (null corners = whole city, §6.10.4) | 20 | AL |
| | 48 | `+0xC0` | ReleaseOccupantList(list) | 4 | ignored |
| `cISC4Occupant` (`0x00AA41D8`) | 3 | `+0x0C` | Init | 0 | ignored |
| | 6 | `+0x18` | AsPropertyHolder (then HasProperty `0x029244DB`) | 0 | EAX ptr |
| | 7 | `+0x1C` | GetType (blocking filter) | 0 | EAX |
| | 8 / 9 | `+0x20` / `+0x24` | GetPosition(cS3DVector3*) / SetPosition(const cS3DVector3*) | 4 | ignored |
| | 10 | `+0x28` | GetBoundingBox(min*, max*) | 8 | ignored |
| | 15 | `+0x3C` | PostOccupantMessage(`0x99EF1145`, 0) | 8 | ignored |
| | 29 / 31 / 32 | `+0x74/+0x7C/+0x80` | SetFlag(1) / ClearFlag(1) / IsFlagSet(1) | 4 | AL (32) |
| `cISC4NetworkOccupant` (IID `0xA821EF94`) | 12 | `+0x30` | GetNetworkFlag | 0 | EAX |
| `cISC4FloraOccupant` (`0x00A88608`) | 3–14 | | §11.4 | | |
| `cIGZCommandParameterSet` (game class `cGZCommandParameterSet`, vtable `0x00AD16C8`) | 5 | `+0x14` | `cIGZVariant* operator[](u32 index)` (`0x0092A162`): never null, an index the set lacks gives the set's own empty variant (slot 6 `GetParameter`, `0x0092A186`, would return null instead) | 4 | EAX ptr, borrowed |
| `cIGZVariant` (`0x00AA6788`) | 24 / 36 | `+0x60` / `+0x90` | `u32 GetValUint32()` / `f32 GetValFloat32()`: call the converting getter of slot 25 / 37 (data-formats §5.1) on a local preset to 0 / 0.0 and return it (`0x0091560F`, `0x0091567A`) | 0 | EAX / ST0 |
| non-virtual | | | `cSC4FloraOccupant` constructor (`0x00472B30`) with the pooled `operator new` (`0x00472960`); `cRZRandom` methods (foundations §6.1); `SC4GeometryUtilities::BoundingBoxesIntersect` (`0x005D6D80`, cdecl, 24 bytes of arguments popped by the caller, AL; §6.4); random cell iterator (`0x0049C820` SetRect, `0x0049CB10` GetNext, `0x0049D1C0` Reset); `cRZCmdLine` | | |

---

## 13. Effects on other subsystems and replaceability

### 13.1 Who reads flora data

| Consumer | How |
|---|---|
| **Flammability** | Reads flora occupants (type `0x74758926`) through the occupant manager and their property `0x29244DB5`, triggered by Insert/Change/Remove occupant messages (flammability §6). Flora's `0x99EF1145` posts on aging/burning reach it. |
| **Aura** | The **flora planting tool** (not the simulator) calls `AddTransientEffect(x, z, 1)` (aura §8). |
| Air pollution, land value, desirability | **No effect.** No code in those simulators reads flora occupants or the flora simulator (no reference to type `0x74758926` or to the flora global). The idea that trees reduce pollution or raise land value is not in the original *[compat]*. |
| Weather | Read by flora (moisture), not the other way round. |
| `cSC4ViewInputControlFlora` (mayor/god tree tool) | Global pointer: `GetStaticFloraData(type)` for the plop cost (int64), `AddNewFloraOccupant(type, pos, false)`; charges the budget and plays a sound. |
| `LUAExtension_tool_plop_flora` | through the generic plop tool *[inferred]* |
| `cSTETerrain::InitFloraBrushInfo` (god-mode forest brush) | `GetAllFloraTypes(buffer, count)` twice, then reads each record's flags byte **directly** (§13.2). |
| `cSC4LotDeveloper` (`AddLotFlora`, `PickFloraTypeForLot`) | `GetFloraTypesByFamily`, `IsFloraTypeValid`, `AddNewFloraOccupant(type, pos, false)` for lot flora. |
| `cSC4NetworkLotManager` (`RedevelopLot`, `DoNetworkOccupantRemoved`, `CreateLotObjects`) | `DemolishFloraInCellArea`. |
| `cSC4DisasterInstanceFire::ToastOccupant` | `KillFloraOccupant(occ)` (wildfire). |
| `cSC4City::SaveObliterated` | slot 24 through the city's own member. |
| `cSC4FloraOccupant::GetBoundingBox` | if it has no record, `GetStaticFloraData(type)` through the global, then reads the record's half-width and height **directly**. |
| Lot editor (`cSC4WinLotConfigurationEditor`), lot configuration manager | families, `AddNewFloraOccupant`, `DemolishFloraOccupant`. |

### 13.2 Replaceability findings

- **Replaceable** through the GZCOM class registry (CLSID `0xC990C034`): the city creates it through GZCOM, and
  every outside caller goes through the vtable (global `0x00B43D80` or city slot 59). No outside code reads the
  simulator object's memory and no grid is involved.
- **Blocker: the static flora record is a shared memory layout** *[compat]*. `GetStaticFloraData` returns a
  `cSC4StaticFloraData*` that original code reads raw and owns references to:

  | Byte offset | Field | Raw reader |
  |---|---|---|
  | `+0x00` | type ID | |
  | `+0x38`, `+0x44` | occupant half-width, height | `cSC4FloraOccupant::GetBoundingBox` |
  | `+0x160` | plop cost, int64 | flora tool `Init` |
  | `+0x168` | exemplar (held) | flora occupant (model), `Release` |
  | `+0x16C` | flags byte | terrain flora brush |
  | `+0x170` | reference count | `cSC4FloraOccupant::SetStaticFloraData` (increments directly), `cSC4StaticFloraData::Release` (`0x00472C70`, non-virtual) |

  The record is 376 bytes, allocated from the game's pooled allocator and freed by the game's `Release` with
  the matching pool free (and the family vector with the game's `operator delete`). A replacement must
  therefore either reuse the original records (build them with the full original layout from the game's
  allocator) or never let a record's count reach zero (keep its own reference for the process lifetime)
  *[choice: keep a permanent reference]*. The flora occupant also holds these records across cities.

  **Never freeing them** (FQ8). The original gives up the registry's references only in its application-shutdown
  callback (§3.2), after which nothing reads a record; its reload on an exemplar change (§5) already drops the
  old records without releasing them. So never freeing the record blocks and never clearing the registry at
  application shutdown has no observable effect; the implementer's choice stands. Records that flora occupants
  still reference must stay valid in any case, because occupants release them through the game's
  `cSC4StaticFloraData::Release`.
- **Flora occupants are original objects.** The simulator creates `cSC4FloraOccupant` directly (constructor +
  pooled new, not GZCOM) and its obliteration save assumes every type-`0x74758926` occupant **is** one (it steps
  from the `cISC4Occupant` pointer to the object base). A replacement should create flora occupants through
  `GZCOM_CreatecSC4FloraOccupant` / class `0xA9C05C85` and use only their interfaces.
- `SC4NewCitySpecification.obliteratedCityDBSegment` (offset `0x204`, documented by the SDK) is the only way to
  find the obliteration segment; a replacement must read it the same way.
- The weather replacement must return `GetMoistureType` as a clean 32-bit value (0..15): flora uses all of EAX
  as a table index.
- The lot manager's `GetLotSavvyTerrainAltitude` returns a float in ST0 (SDK error, §11 / report); a replacement
  flora simulator must declare it that way.
- The PlaceFlora command, cheat ID `0x095BDC9C` / string `"Flora"` and the command name are part of the
  observable surface (cheat console, Lua) *[compat]*.

---

## 14. Original quirks (reproduced) and corrections to the earlier write-up

Quirks *[quirk]*:

1. The key-0 default record (uninitialised flags) sits in the type map: `IsFloraTypeValid(0)` is true; it can
   win `GetBestFloraTypeForLocation` (score 0.6 in elevation and moisture 5..12) when its flag bit 0 happens to
   be clear, which makes type-0 planting requests fail there; a second city leaks a new record. The byte is heap
   leftovers; FreeCity4 fixes it at `0x01` *[choice]* (§3.2).
2. Exemplar-changed handling reloads all types when `data2` is any registered type (including 0), ignores the
   group, leaks the old records, and the cheat message falls into the same handler for any non-flora cheat (sent directly; the subscription delivers only Flora cheats, §4.1 step 11).
3. PlaceFlora puts parameter 2 into `y` and `z = −1`, so it never plants.
4. `AddNewFloraOccupant` rewrites the caller's position array and removes merged flora before its final
   location check.
5. Seed acceptance probability decreases with the score (§6.6).
6. Old flora is always reborn (birth := today, state 0) instead of dying; burnt flora is reborn as a young burnt
   tree if visited before the daily removal.
7. `DemolishFloraInCellArea` / `DemolishFloraInBoundingBox` ignore their flags argument;
   `DemolishFloraOccupant` removes any occupant type.
8. The daily removal takes at most one accumulator step per day.
9. Wild seeding never ends when every type has flag bit 0; it misbehaves with an empty type array.
10. The saved iterator is always discarded by `Init` (keepIteratorFlag is never set).
11. If the tuning exemplar exists but lacks `0x085489C7`, the stored (already scaled, or loaded) seed count is
    scaled again on every tuning read.
12. Seed types depend on exemplar load order (§6.11).
13. The `blast` cheat accepts `z = sizeZ` (inclusive) but not `x = sizeX`.
14. A period of 0 is an integer divide fault in `Init` and the tuning read (§7); FreeCity4 divides by 1.
15. Without a weather simulator the first score computation dereferences null (§6.2); FreeCity4 uses moisture 0.

Corrections to `docs/subsystems/grid-sims.md` §10: the daily removal uses `floor`, not a loop "while acc ≥ 1"
(one step per day); the monthly pass visits **2×2-cell blocks** from a shuffled table, `blocksPerMonth` per
month; old age causes **rebirth**, not death (death comes only from a low score); seeds of mature flora are of
the record's seed type; wild seeding is off in the shipped tuning (0 seeds) and is scaled per 2²⁰ m².

---

## 15. Open questions

1. Meaning of flag bit 2 (cleared by `0x6A37EBB6 = true`) and of the unused floats/u32 (`0x083DC35B[11]`,
   `0x083DE16F`); no reader found.
2. *(resolved)* data3 of Insert/RemoveOccupant is the occupant type (`PostOccupantMessage`, §9.1).
3. *(resolved)* `0x99EF1145` from `PostOccupantMessage` is **posted** (queued), §9.1.
4. The god-mode forest brush planting path (terrain) was not traced beyond its type list *[inferred: it plants
   through `AddNewFloraOccupant`]*.
5. *(resolved)* Null corners mean the whole city for both occupant-manager queries (§6.10.4).
6. *(resolved)* The default record's flags byte is never written (heap leftovers, §3.2; FreeCity4 `0x01`); a
   missing weather simulator crashes the original (§6.2); period 0 is an integer divide fault (§7);
   `BoundingBoxesIntersect` is strict (§6.4); the terrain tolerance is `0.001f` (§6.10.2).

---

## 16. Conformance tests (must exist in `engine/conformance`)

Unless stated: record defaults of §3.3, shipped tuning (§7), a 64 × 64-cell city (1024 m), sea level 250 m.

1. **Stage**: maturity 730, lifespan 1460, max age 1560: age 364 → 0; 365 → 1; 729 → 1; 730 → 2; 1459 → 2;
   1460 → rebirth (birth := today, state 0, one `0x99EF1145`); 2000 → rebirth.
2. **Negative age**: birth = today + 10 → age is 2³² − 10 as unsigned → rebirth.
3. **Elevation index** (scale 0.02): altitude − sea = 75 → 1; 310 → 6; 1000 → 15 (clamped); −30 → 0; **50 → 1, 100 → 2,
   150 → 3**: the product d × 0.02f (exactly 0.99999998 for d = 50) rounds to the whole number at 24 bits
   (foundations §18), and the floor of §6.1 is exact for it. Water
   type 120 m below sea → 2. Code-default scale 0.08: 70 → 5.
4. **Score**: table byte 153 → 0.6 (153 × f32(1/255)); byte 25 → 0.0980392; unknown type → 0.0.
5. **Exemplar table conversion**: 0.6 → 153; 0.1 → 25; 0.9 → 229; 0.4 → 102; 1.0 → 255; 1.2 → 50.
6. **Best type**: types A (score 0.4), B (0.6), C (0.6, flag bit 0) at a cell → B (first strictly greater, map
   order); all scores 0 → type 0, score 0. The key-0 default record (flags `0x01`, FreeCity4 *[choice]*, §3.2) at
   elevation and moisture index 8 (its table byte 153 → 0.6) with only type A (0.4) registered → A, score 0.4.
7. **blocksPerMonth** (period 24): 64² → 42; 128² → 170; 256² → 682. Period 6: 64² → 170.
8. **Wild seed scaling**: value 5, size 1024 m → 5; 2048 m → 20; 4096 m → 80; shipped value 0 → 0 (no wild
   seeding, no draws).
9. **Daily** (shipped 1, 1): five burnt trees → one removed per day, oldest first. Code defaults (1.0, 4.0) → four
   per day. Tuning (4, 2.5): removal on day 4 (2 trees, acc 0.5), day 8 (3 trees, acc 0).
10. **Daily, rate > 1** (`0xC9DBDD00 = 0.5`, step 1): first accumulator 1, 2, 3 … after days 1, 2, 3 (it gains 2
    and loses 1 per day); exactly one removal per day.
11. **DropSeed**: min 0.3, probability 0.5: score 0.6 → accepted for any u (one `RandomDoubleUniform` drawn);
    score 1.0 → accepted iff u < 0.5; score 0.2 → rejected, **no** draw; location not OK → rejected, no draw.
12. **Location bounds**: radius 3: x = 2.9 → false (min.x < 0); x = 3 → passes the bound; x + 3 = 1024 → passes;
    x + 3 = 1024.5 → false.
13. **Slope**: tolerance 0.5, off-lot: normal.y 0.5 → false; 0.51 → passes.
14. **Under water**: land type at ground 240 m (sea 250, off-lot) → false; water type → passes this test.
15. **Blocking**: a prop's box overlapping the clearance box → false; a power pole (network flag bit 5 only) →
    not blocking; a street (bit 3) → blocking; the excluded occupant itself → ignored. Strict test (§6.4): flora
    at x = 100, radius 3 (clearance box x 97..103) and a prop box with min.x = 103.0 → **not** blocking (touching);
    min.x = 102.99999237 (`0x42CDFFFF`) → blocking; the same on the y axis with the prop's min.y equal to `h + height`.
16. **Planting altitude**: land type, `pos.y = 0`, ground 240 m (on a lot, so allowed) → y raised to 250; ground
    260 → 260. Orientation = `RandomSint32RangeUniform(0, 4)` (one integer step).
17. **Cluster merge**: type A (next B, cluster radius 10) planted at (100, 100) with an A tree centred at
    (105, 100) → the tree is removed, the result is one B occupant at (102.5, 100) (if B's checks pass).
18. **PlaceFlora**: command (valid type, 500, 600) → `AddNewFloraOccupant(type, {500, 600, −1})` → null, nothing
    planted.
19. **Seeding**: seedsPerCycle 2.5 → 2 seeds; 3.5 → 4; 0.4 → 0 (and lastSeedingDate unchanged). Each seed draws
    two `RandomDoubleRangeUniform`, even when it falls outside the city.
20. **Seeding interval**: lastSeed 1000, interval 365, now 1365 → no seeding; now 1366 → seeding.
21. **Low score**: score < minimum with propagation on → stage 4 → occupant removed. Same with propagation off
    and flag bit 1 set → no change.
22. **Kill**: flora state 1 → state 3, type `0x29CFF3CB`, record = Burnt Tree, one `0x99EF1145`, in the burnt list,
    returns true; again → true, no change; a building → false.
23. **Building insert**: building box (100..120, 0..30, 200..220) → flora in x 98..122, z 198..222 removed; flora
    at x 97 kept.
24. **Terrain change**: data1 = (10 << 16) | 20, data2 = (0 << 16) | 70 → cells x 9..20, z 0..63. Flora with
    unchanged ground → kept (re-seat no-op); ground moved by 2 m → removed; with `preserve` → moved to the new
    ground, `MoveOccupant` called. Tolerance `0.001f` (§6.10.2): flora at y = 250, new ground 250.0009765625
    (250 + 2⁻¹⁰) → `|d|` = 0.0009765625 → kept, no `MoveOccupant`; new ground 250.001007080078125
    (250 + 2⁻¹⁰ + 2⁻¹⁵) → `|d|` = 0.00100708 → removed (with `preserve`: moved).
25. **Cheat**: `Flora off` with 10 flora → all removed, enabled = false, monthly does nothing; `Flora on` →
    enabled, nothing restored; `Flora propogation` → forced and propagation true; `Flora propogation off` →
    both false; `Flora preserve` → preserve true.
26. **Established**: CityEstablished with forced false → propagation false; CityObliterated → true; Init in an
    established city with forced true → true.
27. **Persistence**: new 64² city, no burnt flora → 2158 bytes, version 2; round trip preserves enabled, forced,
    preserve, today, the generator (the next `RandomDoubleUniform` matches), the burnt list and accumulators;
    version 3 → Read returns false after 2 bytes; burnt count 10,000,000 → list left empty.
28. **Init after load**: saved period 6 with shipped tuning → period 24, blocksPerMonth 42, iterator reshuffled.
29. **GetAllFloraTypes(buffer, count)**: N = 7, null buffer → count 7, returns 7; count 3 → 3 copied, returns 3;
    count 10 → count := 7.
30. **Obliteration**: 3 flora saved → record (`0xC9BB3FC6`, `0xC9BB3FC7`, 1) = `00 00 03 00 00 00` (u16 0, u32 3)
    and records (`0xC9BB3FC6`, `0x4A664F83`, 0..2), each `00 00` then the occupant's version-3 record; the new
    city's `Init` re-creates and inserts 3 flora occupants. A count record whose u16 is 7 restores the same
    (the version is not checked); no count record → nothing restored.
31. **AdvanceClusterType**: chain S → M → L: (S, S) → M; (S, M) → L; (M, S) → L; (L, S) → 0 (next(L) = 0); unrelated
    types → 0.
32. **Seed types**: chain S → C registered C then S → C seeds S; registered S then C → C seeds C.
33. **Occupant messages** (§9.1): `InsertOccupant(flora, 0)` → one **queued** message, delivered after the call
    returns: type `0x99EF1142`, void1 = IGZUnknown = the occupant, data3 = `0x74758926`, data4 = 0, data2 = the
    occupant's flag 4 (0/1); the flora handler sees the occupant with flag 1 already set. A rebirth during the
    monthly visit posts `0x99EF1145` with data3 = `0x74758926`, data4 = 0; no subscriber receives it before
    `DoMessage` returns.
34. **Null corners** (§6.10.4): flora at (1, 1), (1023, 1), (1, 1023) and (1023, 1023) in a 64² city →
    `Flora off` removes all four. Corners (10, 10)..(11, 11) → world box (160.1, −FLT_MAX, 160.1) ..
    (191.9, +FLT_MAX, 191.9); corners (−5, 3)..(70, 3) → cells x 0..63, z 3..3.
35. **Period 0** *[choice]*: tuning `0x8761ABD0 = 0`, 64² city → FreeCity4 `blocksPerMonth` = 4096 >> 2 = 1024,
    period saved as 0 (the original faults on the division).
36. **No weather** *[choice]*: no weather simulator, a type whose table byte at `[e][0]` is 153 → score 0.6
    (moisture 0); the original crashes.
37. **Box query** (§6.4.2): `cell(0)` = 64, `cell(63.93)` = 64, `cell(63.94)` = 65, `cell(64)` = 65, `cell(−0.5)`
    = 63, `cell(319.936)` = 68, `cell(319.937)` = 69. A box x 100 … 101 (width 1.0) is listed from `cell(100)`; a
    box x 100 … 102 from `cell(100.1)` to `cell(101.9)`. Strict: an occupant box x 10 … 20 is not returned by a
    query whose max.x is 10 or whose min.x is 20, and is returned when max.x is 10.000001. Listing: an occupant
    box (1717.9807, ·, 319.84396) … (1735.9807, ·, 337.84396) (x cells 90–91, z cell 69) is **not** returned by
    the query (1696.1, −FLT_MAX, 288.1) … (1727.9, +FLT_MAX, 319.9) and is returned by (1696.1, −FLT_MAX, 320.1) …
    (1727.9, +FLT_MAX, 351.9).
38. **Monthly visits across a 64 m line** (§6.7.1): 2048² city, a flora occupant of a type with box half-width 9 at
    (1726.9807, 328.84396) → the block queries of blocks (53, 10) and (54, 10) return it; those of (53, 9) and
    (54, 9) do not. The same type at (1726.9807, 328.8) (box z from 319.8) → returned by all four blocks. A type
    with half-width 0.4 (box 0.8 m wide, so no inset) at (1726.9807, 320.26) (box z 319.86 … 320.66, listed from
    z cell 68) → returned by blocks (53, 9) and (53, 10). A month in which only block (53, 9) is visited leaves the
    first occupant's birth date and state unchanged.
