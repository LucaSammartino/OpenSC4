# Lot and growth-occupant save records — data-format spec

Status: M6, written from the Windows 1.1.641 executable (`bin/SimCity4.exe`, image base `0x00400000`) and checked
against **every entry of every class record in all 587 city saves** under `Documents\SimCity 4` (417 distinct
files by content). Every layout below was read in the class's `Write` **and** `Read`, and a scratch validator parsed
each entry with these layouts, requiring exact consumption, byte-identical re-encoding, a correct entry CRC and
resolvable references (§17).

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

Shared material cited and not repeated: stream primitives (foundations §2.2), strings (§2.3), inline nested objects
(§2.6), references (§2.7), version convention (§2.10), the class-object store, its entries, CRC and load order
(§3.3–§3.5), random generators (§6), variants (§9), plain and exemplar property holders (§10.2, §10.3). The occupant
manager's own record, the simulator records that refer to lots (crime, budget, traffic, …) and the classes reached
only through them (pedestrians, automata) are specified in `save-records.md`. The trip-data element is also given in
`traffic-simulator.md` §6.3; the zone developer's record in `zones.md` §10.2; the lot developer's record in
`lot-developer.md` §13; flora behaviour in `flora.md`.

---

## 1. Scope and identity

These are the classes that **growth** (zone developer, lot developer, tract developer, construction) and **traffic**
create or change on a lot. A standalone FreeCity4 that replaces growth writes all of them. Each one is stored in the
class-object store (foundations §3.3): record key `(classID, 0x299B2D1B, 0)`, one entry per object, written only
because some other saved object holds a reference to it (§14).

| Class | Class ID | Role | Write / Read | `cIGZSerializable` vtable | Version written / accepted |
|---|---|---|---|---|---|
| `cSC4Lot` | `0xC9BD5D4A` | the lot | `0x006C0080` / `0x006C35F0` | `0x00AB1994` (sub-object +4) | 8 / ≤ 8 |
| `cSC4BusinessUnit` | `0x4A232DA8` | group of lots that share a business | `0x006C0370` / `0x006C09E0` | `0x00AB18C4` | 1 / 1 |
| `cSC4ZoneDeveloper` (lot map) | `0x498F9B01` | cell → lot map | `zones.md` §10.2 | — | 1 / 1 |
| `cSC4Occupant` (base part; bare instances) | `0xA9BC9AB6` | occupant base record; bare = dummy automaton occupant | `0x005E8CD0` / `0x005E8D60` | `0x00A9ECBC` | 4 / 4 |
| `cSC4BuildingOccupant` | `0xA9BD882D` | the grown (or plopped) building | `0x00458A90` / `0x00458B10` | `0x00A8721C` | 2 / 2 |
| `cSC4LotBaseTextureOccupant` | `0xC97F987C` | lot ground textures | `0x005DF1E0` / `0x005E10B0` | `0x00AA2D74` | 2 / 2 |
| `cSC4LotRetainingWallOccupant` | `0x49C05C9F` | retaining walls around a sunk / raised lot | `0x005E2AB0` / `0x005E2B40` | `0x00AA2FDC` | 2 / 2 |
| `cSC4FoundationOccupant` | `0x49C05C8F` | building foundations on slopes | `0x005D6250` / `0x005D62D0` | `0x00AA28FC` | 2 / 2 |
| `cSC4ConstructionOccupant` | `0xA97F909E` | construction in progress (crane, crew, dust) | `0x005CD250` / `0x005CE770` | `0x00AA219C` | 5 / ≥ 4 |
| `cSC4PropOccupant` | `0x2977AA47` | props placed by the lot developer | `0x005ECE90` / `0x005EF360` | `0x00AA484C` | 6 / any |
| `cSC4PropOccupantVisualEffect` | `0x2977AA49` | effect props (lot and construction effects) | `0x005F03B0` / `0x005EF750` | `0x00AA4DC4` | 3 / 3 |
| `cSC4SignpostOccupant` | `0xAB72FBB3` | region-view signposts with editable text (§10.3) | `0x005F5080` / `0x005F50F0` | `0x00AA5364` | 2 / any |
| `cSC4TextDecalOccupant` | `0x8B79C707` | terrain text labels (§10.4) | `0x005F7DB0` / `0x005F7E50` | `0x00AA556C` | 3 / any |
| `cSC4FloraOccupant` | `0xA9C05C85` | trees and plants, on lots and wild | `0x00472990` / `0x00472A20` | `0x00A88564` | 3 / 3 |
| `cSC4TripData` | — (inline element) | one trip of a lot's trip list | `0x0070E0B0` / `0x0070E160` | non-virtual | 2 / any |

Three more prop subclasses exist but occur in no save: burnt building `0xC9EFB016` (Write `0x005EDB10` / Read
`0x005EFBA0`), terrain decal `0x2977AA48` (`0x005ED710` / `0x005EF4E0`), automata controller `0x2A4017A0`
(`0x005EDDB0` / `0x005EFBF0`). Their layouts are not specified here (§16 Q6).

Interfaces requested when these objects are read through references (foundations §2.7): `cISC4Lot` `0xE818FCED`,
`cISC4BusinessUnit` `0x6A233069`, `cISC4BuildingOccupant` `0x87DDA3A9`, `cISC4PropOccupant` `0xE9793A65`,
`cISC4Automaton` `0xA9B40F05`, and for effect props the class ID `0x2977AA49` itself (used as an IID). Objects are
created by the store through `GZCOM()->GetClassObject(classID, cIGZSerializable)`, so every class above must be
registered with the class registry (§15).

Agents, ticks and tuning: none of these records is an agent. No message is posted by any `Write` or `Read`. The
occupant base `Init` posts `CreateOccupant 0x99EF1140` only for occupants that were **not** loaded (§2.4).

---

## 2. Common pieces

### 2.1 Encodings used *[compat]*

All little-endian, no padding (foundations §2.2). Short names used in the tables:

| Name | Bytes | Encoding |
|---|---|---|
| `u8 / s8 / u16 / s16 / u32 / s32 / f32` | 1/1/2/2/4/4/4 | primitive |
| `bool` | 1 | written 0 / 1 (`operator<<` `0x005D9FD0`); read as byte ≠ 0 (`operator>>` `0x005D9FA0`) |
| `vec3` | 12 | 3 × f32 x, y, z (`0x005DA580` / `0x005DA550`) |
| `box` | 24 | 6 × f32: min x, y, z, max x, y, z (`0x005DA600`) |
| `day` | 4 | u32 day number of a `cRZDate` (`0x005DA1A0`; read `0x005DA170` sets the date from it). Observed values are Julian day numbers (2,451,545 = 1 Jan 2000) |
| `ref` | 4 or 8 | foundations §2.7: u32 objectID (0 = null, nothing follows), u32 classID |
| `bytes` | 4 + n | u32 n, then n bytes (`0x005DADA0`); read `0x005DB520`: if n ≥ 1,000,000 nothing more is read and the vector is left unchanged |
| `xform` | 1 + … | u8 mask; if mask & 4: 9 × f32 3×3 matrix (`0x005DA680`); if mask & 1: 3 × f32 translation; if mask & 2: 1 × f32 scale (`0x005DA8C0`; read `0x005DA710`) |
| `gzstr` | 4 + n | foundations §2.3 |
| `plain holder` | | foundations §10.2 (u16 2, u32 count, entries) |
| `exemplar holder` | | foundations §10.3 (u16 2, plain holder, u32 group, u32 type, u32 instance of the parent exemplar) |

### 2.2 Write and Read conventions *[compat]*

- `Write(stream)` and `Read(stream)` are `cIGZSerializable` slots 3 and 4, `__thiscall`, callee pops 4. `Write`
  returns true exactly when the stream has no error afterwards (`GetError() = 0`); it never returns false for any
  other reason (foundations §3.5 requires this).
- `Read` returns false (and stops) on its **own** unaccepted version; nested rejection propagates only where
  the containing body tests it (see §2.3 for the occupant-base exception). Otherwise its final acceptance uses
  `GetError() = 0`. The class
  store ignores the result and resynchronises on the entry size (foundations §3.5), so a short read never shifts the
  next entry; a read past the payload puts the stream into the sticky error state and fails the rest of the class
  *[compat]*.
- Subclass records start with their own u16 version, then the base record **inline** (its own version included),
  then their fields. A subclass `Read` that finds a bad subclass version returns false before reading the base.
- References read in `Read` are only stored; the referenced object may not have been read yet (foundations §2.7).
  Work that needs them is done in `Init` / `PostCityInit` (§3.5, §2.4).

### 2.3 Occupant base record (`cSC4Occupant`) *[compat]*

Every occupant record below contains this block (14 bytes + the holder):

| # | Field | Type | Meaning, observed values |
|---|---|---|---|
| 1 | version | u16 | 4. `Read`: anything else → false, nothing more read |
| 2 | removal counters | u16 | low byte = number of holders of removal flag 2, high byte = holders of removal flag 1 (`SetRemovalFlags` `0x005E8A60`, `CanRemove` `0x005E8AE0`). 0 in all records except props (0, 1, 256, 257) |
| 3 | highlight | u8 | `GetHighlight()` (`cISC4Occupant` slot 16): the highlight level. Always 0 in saves |
| 4 | occupant flags | u8 | the flag byte **with bits 1 and 5 cleared** (`& 0xDD`). Bit 0 = inserted in the occupant manager, bit 2 = visible (constructor default 4). Observed: 5 for every class; flora 13 (bit 3 also set); props also 0x85 (80×) and 0x15 (1×). Bits 3, 4, 7 unnamed |
| 5 | occupant type | u32 | `0x278128A0` building, `0x497F6D9D` base texture, `0x895D1169` retaining wall, `0x68FD0C69` foundation, `0x74758926` flora, `0xA823821E` prop (also effect props), `0x096E763C` construction, `0x74758925` bare occupant |
| 6 | manager cell rect | 4 × u8 | the occupant manager's spatial-index rectangle x1, z1, x2, z2 (first byte `0xFF` = not indexed). Set by the occupant manager on insertion. *[inferred]* `floor(coord / 64) + 64` of the occupant's world bounds (holds for 99.2 % of buildings, base textures, foundations and props; edge and model-extent cases differ). Never `0xFF` in saves |
| 7 | exemplar holder | | foundations §10.3; parent = the occupant's exemplar (type `0x6534284A`) or `(0, 0, 0)` (base textures, constructions, bare occupants, construction effect props). Entries in ascending ID, key = ID (all saves) |

`Read` (`0x005E8D60`) reads the same fields; the highlight byte becomes the occupant's status byte as
`(highlight << 4) | 2`, i.e. "not yet initialised, loaded from a save" (bit 1). Flags and type are stored as read.

**Nested-holder return (LG33):** this base reader ignores the exemplar holder's Boolean at `0x005E8DD8`
and immediately queries stream `GetError` at `0x005E8DE1`. It returns true when that error is zero, even
when the holder has just rejected its version or its nested plain-holder version. No cursor repair or rollback
occurs. This is the common **base** body; each derived reader decides whether to test the base return and
what follows. Landfill's tested base return and misaligned continuation are specified in landfill §6.4;
do not generalise its 21-byte tail or total return to other derived occupants. The exemplar holder itself
still tests the plain holder's return (foundations §10.3).

### 2.4 Occupant `Init` after a load (`0x005E8C90`) *[compat]*

If `IsInitialized()` (slot 5) is true: return true. Otherwise set status bit 0 (initialised). If status bit 1
(loaded) is **clear**, post `PostOccupantMessage(0x99EF1140 CreateOccupant, 0)` (slot 15); if it is set, call
`SetHighlight(0, false)` (slot 17) instead. Returns true. So loaded occupants do not announce themselves as
newly created.

---

## 3. The lot (`cSC4Lot`, `0xC9BD5D4A`)

### 3.1 Role, creation, interfaces

A lot is a rectangle of cells with a configuration (the lot plan), a state, a zone, an occupant wealth, a list of
populations and a trip list. The zone developer creates lots during parcellization and owns them through its cell
map (`zones.md` §10.2); the lot developer develops them. The constructor (`0x006C2340`) installs four vtables:
`cISC4Lot` primary `0x00AB19B0`, `cIGZSerializable` `0x00AB1994` (sub-object +4), the plain property holder
`0x00AB1930` (sub-object +8), `cISC4TrafficSource` `0x00AB1914` (sub-object +0x1C). It initializes the record
fields, constructs the property holder and allocates the empty containers; it does not zero the whole object.

**View access (LR9).** The offsets above are correct. On the primary `cISC4Lot` view, **slot 3**
(`0x006C2040`) is `AsPropertyHolder`, returning object +8, and **slot 4** (`0x006C2050`) is
`AsTrafficSource`, returning object +0x1C. Both are non-owning casts (`__thiscall`, no stack arguments,
EAX pointer, no AddRef); both return null for a null ECX. The names attached to these shared functions in
the export and the old ABI rows are not reliable interface names: the same address serves different classes.
In particular, the old `cISC4Lot` slot-4 name `AsISCPropertyHolder` is wrong; its +0x1C result is right.

Primary `QueryInterface` (`0x006BF190`, callee pops 8, result AL) accepts exactly IID 1 and lot IID
`0xE818FCED` (the primary pointer), and Serializable IID `0xE4FDA3D4` (object +4). Each success AddRefs
the primary object once. Any other IID returns false **without writing the destination**. Neither secondary
view is acquired through this QI. QI through the property-holder view first subtracts 8
(`0x006C2280`), through Serializable subtracts 4 (`0x006C2260`), and through the traffic-source view
subtracts 0x1C (`0x006C22B0`); all delegate to the same restricted primary QI. The property-holder
AddRef/Release thunks (`0x006C2290` / `0x006C2310`) subtract 8; the traffic-source thunks
(`0x006C22C0` / `0x006C2320`) subtract 0x1C. All views share the lot's count at primary +0xC
(AddRef `0x004664A0`, Release `0x006BF460`).

Native source/target evidence, checked with instruction bytes:

| Source | Target/result | Discrimination |
|---|---|---|
| Constructor `0x006C2345`, `0x006C2357`, `0x006C2372`, `0x006C2378` | construct property holder at +8; install `0x00AB1930` there and `0x00AB1914` at +0x1C | The views are not exchanged. The parameterized constructor repeats this at `0x006C1E55`–`0x006C1E88`. |
| Primary vtable entries `0x00AB19BC`, `0x00AB19C0` | slots 3/4 target `0x006C2040` / `0x006C2050` | Slot numbers are zero-based. |
| `0x006C2044` / `0x006C2054` | EAX = ECX +8 / +0x1C, respectively | Neither function calls AddRef. |
| Traffic source iterator `GetNext`, `0x0071083D`–`0x0071084E` | call lot slot 4 at `0x00710841`, store its result in source output, then AddRef the **lot** | A native caller consumes +0x1C as a traffic source, not a property holder. The iterator supplies the reference; the cast does not. |
| QI `0x006BF194`–`0x006BF1F2`; property-view QI `0x006C2280` | only the three IIDs above; property-view QI adjusts back to primary | The vtable's presence does not imply QI accepts that secondary view's IID. |

The lot does **not** save its occupants. `GetBuilding` and `GetLotOccupants` query the occupant manager over the
lot's bounding rect (with an "inside lot" filter) each time *[compat]*. The occupants' only link back is the
**lot tag** field of props and flora (§11, §12) and the lot reference of walls and constructions.

### 3.2 Record *[compat]*

| # | Field | Type | Meaning, observed values (12,986 lots) |
|---|---|---|---|
| 1 | version | u16 | 8 |
| 2 | lot configuration ID | u32 | `GetID()` (configuration slot 3) of the lot's configuration object; 0 when it has none (692 lots: the 691 empty zoned lots and one more). A lot read but never `Init`ed has no object and writes 0, not the ID it read (§3.5) |
| 3 | lot flags | u8 | bit 0 terrain locked flat at the lot's altitude; bits 0+1 together = foundation lot (lot developer `AddLotFoundation` sets `(f & ~4) | 3`); bit 2 retaining walls present *[inferred: set by `AddRetainingWallOccupant`; cleared with bits 0–1 by `StartConstruction`]*; bit 3 needs power; bit 4 has power; bit 5 historical; bit 6 vertices locked; bit 7 traffic mark, cleared by the traffic simulator's `FinishTripMap` (`traffic-simulator.md` §4.7), setter not identified. Observed: 0x50, 0x58, 0x5B, 0x10, 0xD8, 0x5F, 0x53, 0xDB, 0xDF, 0xD0, 0x57 |
| 4 | bounding rect | 4 × u8 | x1, z1, x2, z2 in cells, inclusive; x1 ≤ x2, z1 ≤ z2 always |
| 5 | location | 2 × u8 | x, z: always one of the four corners of the rect |
| 6 | altitude | f32 | lot altitude in metres (observed 256.79 … 391.05) |
| 7 | slope at altitude change | f32 | terrain slope stored when the altitude was last set (`SetAltitude`) |
| 8 | current slope | f32 | terrain slope at the last `UpdateSlope` (0 … 30.53) |
| 9 | size x, size z | 2 × u8 | lot size in cells (1…6, 1…8); always the rect's extent, possibly swapped by facing |
| 10 | facing | u8 | 0…3 |
| 11 | state | u8 | `cISC4Lot::HabitationState` (`lot-developer.md` conventions): observed 1 (691), 2 (31), 3 (12,158), 4 (106) |
| 12 | condition | u8 | observed 0…4 |
| 13 | zone type | u8 | observed 1, 2, 3, 4, 5, 7, 8, 9, 15 |
| 14 | occupant wealth | u8 | 0…3 |
| 15 | day of last state change | day | written by `SetState`; never 0 in saves |
| 16 | building type | u32 | the building's type; 0 for 691 lots. The multiset of non-zero values equals the multiset of the building occupants' building types (§5) |
| 17 | vacate reason | u8 | written only by `SetState(4, …)` (state 4 = vacant) and kept afterwards. Every vacant lot has 1…6; non-vacant lots carry stale values, 98 distinct ones including > 6 (e.g. 186 in 4,646 lots) *[quirk]* (§16 Q2) |
| 18 | business unit | ref | `cSC4BusinessUnit` (read IID `0x6A233069`); 7,746 lots |
| 19 | travel designate | ref | another lot (read IID `0xE818FCED`) whose trips stand in for this one; 6,110 lots |
| 20 | population map count | u8 | P (observed 0, 1) |
| 21 | P × population map | | u8 n (1…3), then n × {u32 developer type, u16 count}: the vector of maps returned by `GetPopulations` (slot 54). Keys observed: `0x1010/0x1020/0x1030` (R$…R$$$), `0x3110/0x3120/0x3130` (CS), `0x3320/0x3330` (CO), `0x4100/0x4200/0x4300` (I-R, I-D, I-M); counts 0…2,337 |
| 22 | travel-population map | | u8 n (0…3), then n × {u32 developer type, u16 count}; same keys |
| 23 | travel jobs | 3 × f32 | `GetTravelJobs`; observed [0] 0…450, [1] 0…73.8, [2] 0…8.2 |
| 24 | properties | plain holder | foundations §10.2 (§3.6) |
| 25 | trip count | u32 | T (0…31) |
| 26 | T × trip | | §3.3 |
| 27 | traffic strategy byte | u8 | 0…99 (`traffic-simulator.md` §4.1) |

Map entries are written in ascending key order (an ordered map); a map with more than 255 entries would be
truncated by the u8 count *[quirk; never observed]*. The trip list is written front to back.

`Write` returns false **without writing field 27** if a trip's `Write` fails (only on a stream error); the return
value is in **AL only** (§13).

### 3.2.1 Population mutation (LR10) *[compat]*

`SetPopulation(devType: u32, population: u16)` (slot 55, `0x006C1940`, pop 8) first calls
slot 52 `GetAvailableCapacity(devType, false)`. It compares the requested **unsigned word** with AX and
uses the smaller word (`0x006C1958–006C195F`). It then calls `GetBuildingType(true)` (slot 46),
and BDS slot 39 to obtain that type's profile. Let C be the profile's ordered vector of capacity maps,
and P the lot's vector of population maps. Search C in vector order for the first map containing devType
(`0x006C1980–006C19EC`). The key comparisons are unsigned u32 comparisons.

- No match, including C empty: return **AL = 0**, without writing P or recomputing wealth.
- First match at index i: assign the clamped u16 to key devType in **P[i]**, inserting the key there
  if absent; recompute occupant wealth (§3.2.2); return **AL = 1** (`0x006C19F8–006C1A28`).
- It does not append a map, search P for a suitable map, update all duplicate keys, or remove a zero entry.
  A zero assignment, unchanged assignment, and insertion all take the same successful path. It performs
  no state/day update, queued message, travel-population update or job-cache update.
- P must already contain the selected index. The native method indexes it without a size check; an empty
  P paired with a matching nonempty C is outside its valid container invariant, not a successful creation
  case. A profile pointer is dereferenced without a null check. Unknown profiles represented by a nonnull
  profile with an empty capacity vector take the no-match path. Native BDS slot 39 (`0x0069CA60`)
  uses its profile catalogue's indexing operation: an unknown type creates a default profile with
  purpose/wealth/resource count zero and an empty capacity vector (`0x0069B766–0069B7CA`).

The clamp's collaborator is `GetAvailableCapacity` (`0x006C0640`, slot 52). With its second argument
false, it returns AX=0 unless the state byte is 3. Otherwise it selects the same first C[i] containing
devType, or returns AX=0 if none. Start a float32 fraction F=1; for each **other** key q of C[i], in
ascending unsigned-key order, if P[i] contains q, replace F with
`r24(F − extended(P[i][q] / C[i][q]))`. The division uses exact u16-to-x87 conversion and no intervening
float32 store (`0x006C0733–006C0751`). Missing population keys are skipped, not inserted by this getter.
If F equals 1, return the requested key's capacity word. Otherwise, if the profile's mixed-occupancy byte
is zero, return AX=0; if nonzero, return the low word of
`bits32(r24(extended(capacity × F) + 12582912.0f)) − 0x4B400000`
(`0x006C079D–006C07F9`; constants `0x00A81228`, `0x00AB16B4`). There is no nonnegative clamp of F or
this result; zero denominators are not guarded. The equality test treats NaN as unequal. These rules
also explain why a requested u16 can be clamped to zero outside state 3 while the setter still succeeds.
The second argument true to slot 52 additionally permits state 4; SetPopulation always supplies false.

Only AL is defined as the boolean result; EAX's upper 24 bits are incidental. This is stronger than the old
`eax-or-void` annotation: the two explicit AL assignments prove a boolean result. ReoccupyBuilding passes
this AL through (lot-developer.md §6.4); tract UpdatePopulation ignores it (tract-developer.md §4.18).
Caller pass-through alone would not establish a boolean contract.

### 3.2.2 State mutation and queued snapshot (LR11) *[compat]*

`SetState(state: u32, wealth: u32, reason: u32)` (slot 34, `0x006C3100`, pop 12) compares the
zero-extended current state byte with the **whole** state argument. Equality does nothing. Otherwise it
stores the low state byte, then obtains the current city day and stores it as the state-change day
(`0x006C3106–006C3133`). The following actions precede the message copy, in this order:

1. When the old state was 2, the stored building type is nonzero, and its BDS profile purpose is nonzero,
   release construction resources via BDS slot 51 with the zero-extended profile u16 resource field.
   This happens even if the new state is not 3. If the new state is 3, obtain the developer type for the
   profile purpose and native wealth, call lot slot 50 `GetCapacity(devType, true)`, then slot 55 with
   that result (`0x006C3136–006C3198`). That nested setter can recompute wealth before step 4.
   Slot 50's true argument bypasses its state gate; it returns the capacity word from the first profile
   map containing the key, or zero when no such key/type exists (`0x006C05A0–006C0633`). Resource
   release is called even for resource count zero: native BDS `0x00695980` sends float32 −R to its
   construction-resource counter's slot 17, then invokes counter slot 30 (`0x00695980–006959A3`).
2. On entering state 2, obtain `GetBuildingType(true)`. If nonzero, destroy all existing population maps
   (retaining vector storage), then copy each profile capacity map into P in vector order and set every
   copied count to zero (`0x006C31C0–006C32F0`, clear helper `0x006C2E30`). Empty capacity maps remain
   empty maps. No building type means this rebuild is skipped. On entering state 4, store low byte of
   reason as the vacate reason (`0x006C31B1`); other states retain the previous reason.
3. If the new state byte is outside {2,3}, set all existing population counts in every map to zero,
   retaining the maps and their keys (`0x006C32F6–006C33A7`). Travel populations, travel jobs and trips
   are not reset by these loops.
4. Save the current occupant wealth as Wpre, then call `UpdatePrimaryOccupantWealth` (`0x006C33A9`).
   With no stored building type (`GetBuildingType(false) = 0`) it sets wealth to zero; with a nonzero
   type whose nonnull profile has purpose zero it leaves wealth unchanged (`0x006C143B–006C1466`,
   `0x006C15C3`). The profile/occupancy selection for a known profile is detailed below.
5. Let Wcomputed be the resulting wealth; initially message data4 = Wcomputed. If old/new state bytes
   differ in membership of {2,3}: leaving uses Wpre for data4 while keeping Wcomputed in the lot;
   entering with whole wealth argument nonzero stores its low byte in the lot and data4. Entering with
   wealth argument zero keeps Wcomputed. Within or outside the group, the argument is ignored
   (`0x006C33BA–006C3411`). Wpre is sampled here, not necessarily at method entry.
6. Call `UpdateNeedsPower` (`0x006C3417`), then construct the independent lot copy with `0x006C2450`
   (`0x006C3449`). This is the snapshot point: after all preceding mutation and synchronous calls,
   before message allocation and posting. Message type `0xE8BDA2E7` has data1 = old byte, data2 = new
   byte, data3 = stored building type, data4 as above. Post via MessagePost(message, 0), then release
   the caller's message reference, then its copy reference (`0x006C3473–006C34DB`).

There is no defined success return. The equal-state path happens to leave AL = current state; the changed
path leaves the final copy Release's EAX (normally 1 while the queued message owns the copy).
Neither is a SetState boolean. A caller forwarding AL does not
make either value a meaningful transition result.

**Known-profile wealth selection.** For nonzero stored building type and profile purpose, build the
occupancy-by-type map O through lot slot 62 (`GetOccupancyByType`, `0x006C10F0`), before choosing wealth.
It uses `GetBuildingType(true)` to obtain its own profile, clears its output, and walks every capacity
map C[i] in vector order and every key in ascending unsigned order. Each capacity key is indexed in
P[i], inserting a zero count if missing, **even before any floating-point occupancy calculation**
(`0x006C11E0–006C1209`). Thus the wealth update may mutate multiple population maps and keys after
SetPopulation's single selected assignment.

For each map compute the signed-int32 sum of its u16 capacities, then divide by the number of keys
with signed integer division, truncating toward zero; call this integer mean A[i]. Sum these integer
means into signed-int32 S (`0x006C1250–006C126A`). Native arithmetic wraps at the int32 additions.
C empty produces empty O. Any empty map **inside** nonempty C reaches integer division by zero;
there is no empty-map skip (`0x006C1258`). If S≤0 the output remains empty. Otherwise for each capacity
entry (q,c) in each map, in the same order, compute
`r24(extended(P[i][q] / c) × extended(A[i] / r24(S)))` as its contribution. The two quotients and
their product stay in x87 precision until the one float32 store (`0x006C12DE–006C1336`). The first
contribution initializes O[q]; subsequent contributions to that key are accumulated with a float32
store after each addition (`0x006C136E–006C1386`). Zero c is not guarded; nonfinite contributions are
kept. Neither travel populations nor travel designates participate.

Wealth zero is first replaced by the low byte of the profile's native wealth. Nonzero wealth is retained
as an initial candidate only if the developer type obtained from BDS slot 30 for (purpose, current wealth)
occurs in O; otherwise it too falls back to native wealth (`0x006C14A8–006C14F0`). Now start the best
occupancy at **0.0f** (zero-filled native datum `0x00B49114`, read at `0x006C14FC`). Visit wealth values
from the profile's signed-int32 native wealth down through 1, obtaining each developer type through
BDS slot 30. For each type present in O, replace the lot wealth and best occupancy only if its float32
occupancy is **strictly greater** than the best. NaN, zero, negative values and equal scores do not win
(`0x006C1557–006C1574`). Consequently a positive tie chooses the higher wealth, whereas all-zero O
retains a supported nonzero current wealth, or the native-wealth fallback. The temporary O tree and
its sentinel are freed before return (`0x006C1576–006C15B8`); there is no wealth-change notification.

**Snapshot contents and lifetime.** The copy constructor gives population maps and the travel-population map independent tree storage
(`0x006C250F`, `0x006C251B`); the helper at `0x004C9470`, despite its exported template name,
constructs a tree copy, not a vector append. Scalar saved fields, travel-job floats, state/day/wealth/reason
and strategy byte are copied. Configuration is retained once (`0x006C24BF–006C24CC`). The runtime tree
at native +0x40 (*[inferred: occupant set]*) and trip list start empty. **Business unit and travel designate
both start null** (`0x006C2505`, `0x006C250C`; unit identity confirmed by SetUnit `0x006C20B8`).
The raw scalar at +0x30 is the unresolved configuration ID used by Init (`0x006BF515`), not a unit
reference. The property holder is copied into its own entry-vector storage; its property objects are
shared, with one AddRef per nonnull entry (`0x006C247A` → `0x005FF120` → `0x005FEA70` →
`0x0077AC37–0077AC47`).
This describes snapshot semantics, not a layout contract for forwarding original methods on replacement objects.

The constructor resets the lot reference count to zero through the property-holder constructor
(`0x005FF134`). SetState AddRefs the new copy once at `0x006C3454`. Assigning it to the message's
IGZUnknown AddRefs it once more (`0x0091346B` → `0x005447C7`); it does not transfer the caller's
reference. MessagePost with flag false appends to the queue (`0x0092D0CF–0092D0E2`).
PushBack temporarily retains the message and then retains it in the queue
node, releasing the temporary afterwards (`0x0092DDF8–0092DE0E`, `0x004F20CF`, `0x007E98E5`).
After SetState releases its message and copy references, the queue therefore owns one message reference
and that message owns one copy reference. When the last message reference is released, its destructor
releases the copy (`0x009133C0–009133C9`); additional references acquired by receivers can extend this
lifetime. The copy's final Release invokes Shutdown then destruction (`0x006BF471–006BF480`). If its
copied initialized byte is set, Shutdown destroys the population maps, clears the travel map and releases
and nulls configuration; the destructor consequently does not release configuration twice
(`0x006C3072–006C30D7`, `0x006C26B4`). If uninitialized, the destructor alone releases configuration.
The destructor releases each shared property once (`0x005FF150` → `0x005FF0D0–005FF0D9`) and frees
the independent containers. No unit/travel-designate reference was acquired, and neither is released.

**Power cache before snapshot.** `UpdateNeedsPower` changes only flag bit 3, directly, without SetFlags
or a message (`0x006BFE25`, `0x006BFE2F`). It clears the bit unless state is 3 or 4 and
GetBuildingType(false) is nonzero. In that case it obtains the building through lot slot 48 and the BDS
profile. Purpose zero skips the developer/configuration test below. Otherwise it obtains the developer
for (purpose,native wealth) through BDS slot 33 and the configuration through lot slot 9; a null result
clears the bit. It calls developer slot 19 and configuration slot 11, comparing their **low unsigned
bytes**; configuration's byte must be strictly greater to continue (`0x006BFDA8–006BFDBC`).
If the building is null, set the bit. If present, obtain its property holder via building slot 3, result
slot 6, then look up properties through slot 9. A present `0x27812852` whose value's slot 72 returns
nonzero suppresses power need; regardless, look up `0x27812854`, requiring that property to exist and
its value's slot 72 to return nonzero. Set bit 3 only if this latter test passes and the suppression test
did not; otherwise clear it (`0x006BFDC4–006BFE23`). All these calls finish before the snapshot.

### 3.3 Trip element (`cSC4TripData`) *[compat]*

Inline, not a reference. Matches `traffic-simulator.md` §6.3; the observed values:

| # | Field | Type | Observed (8,109 trips) |
|---|---|---|---|
| 1 | version | u32 | 2 (all) |
| 2 | route | bytes | 0…47 bytes |
| 3 | return route | bytes | 0…33 bytes |
| 4 | count | u32 | 1…148 |
| 5 | type | u8 | 0…3 |
| 6 | strategy | u8 | 0…2 |
| 7 | flags | u8 | 0…11 |
| 8 | status | u8 | 0…5 |
| 9 | destination x, z | 2 × s16 | include `0xCCCC` (−13,108) and other uninitialised values |
| 10 | trip length | f32 | 0 … 12.74 |
| 11 | travel types used | u32 | only if version > 1 (else 0 on read). Observed 1 (28), 2 (217) and **`0xCCCCCCCC` (126)**: the uninitialised field of probe / bulk trips (`traffic-simulator.md` §4.2 quirk) is saved as is |

`Read` accepts any version. A writer must reproduce uninitialised fields byte-for-byte to round-trip a loaded
city *[compat]*; for trips it creates itself FreeCity4 writes 0 *[choice]* (`traffic-simulator.md` §4.2).

### 3.4 `Read` (`0x006C35F0`) *[compat]*

1. Set the lot's **initialised** flag (not saved) to true first, before the version check.
2. Read the version. If > 8: return false (only the version consumed).
3. Read the fields in the order of §3.2. Versions < 6 use a legacy layout: the rect is 4 × s32, the location
   2 × s32, the size 2 × s32; after the wealth byte a `bool` → `SetVerticesAreLocked` (slot 69); after the building
   type three `bool`s → flag bit 3 (needs power), `SetHasPower` (slot 67), `SetHistorical` (slot 44). No version
   < 8 occurs in the saves.
4. Business unit: the destination is cleared and the old unit released, then the reference is read. Travel
   designate: the old one is released and the reference read.
5. Population maps: the vector is **resized** to P and each map's entries are inserted (`operator[]`, so an entry of
   a key already present overwrites it). The travel-population map is inserted into likewise (not cleared).
6. The plain holder is read; its result is ignored.
7. Only if version > 6: the trips; each read trip is **appended** to the existing list (not cleared first). A trip
   `Read` failure returns false at once.
8. Only if version > 7: the strategy byte.
9. Return `GetError() = 0` in **AL**.

### 3.5 What happens after the load *[compat]*

The zone developer's `Init` calls `lot.Init()` for every lot on its map, once per lot at its origin cell
(`zones.md` §1.2), and later `PostCityInit`.

`Init` (`0x006BF490`, slot 5):

1. If the lot is **not** initialised (a new lot): travel jobs := 0, 0, 0; `c := GetCenter()` (slot 15);
   `a := terrain.GetAltitude(c.x, c.z)` (terrain `0x00B43CF4`, `cISTETerrain` slot 13, terrain.md §4.1, result in
   ST0); `SetAltitude(a)` (slot 17, `a` stored to a float argument); initialised := true. A loaded lot skips this
   (step 1 of §3.4), so its saved altitude and slopes are kept.

   **`GetCenter(out vec3)`** (slot 15, `0x006BF560`, pop 4) — the centre of the rect in metres, cell size 16:
   `x := (f32(16·x1) + f32(16·(x2 + 1))) × 0.5`, `y := altitude` (the stored float, copied), `z :=
   (f32(16·z1) + f32(16·(z2 + 1))) × 0.5`. The rect bytes are zero-extended, the products are integers converted
   exactly (`FILD`), the sum is one x87 add and the factor is the float 0.5 at `0x00A84D2C`; every value is exact.
   Example: rect (10, 20, 12, 21) gives x = (160 + 208) / 2 = 184.0, z = (320 + 352) / 2 = 336.0.

   **`SetAltitude(a)`** (slot 17, `0x006BF5C0`, pop 4): if `|a − altitude| > 0.001` (float `a` minus the stored
   float, `FABS`, compared with the **double** 0.001 at `0x00AA2270`; equal or NaN → no change): `altitude := a`,
   then `s := lotDeveloper.GetLotSlope(lot, false)` (BDS `0x00B43D68` slot 36 `GetLotDeveloper`, then lot developer
   slot 30, which is `GetAreaSlope(rect, false)`, lot-developer.md §11.3; it runs after the altitude is stored and
   does not depend on it) and **both** slope fields := `f32(s)`: field 7 (slope at altitude change, `+0x2C`) and
   field 8 (current slope, `+0x28`) get the same value. Otherwise nothing changes, including the slopes. Because the
   constructor zeroes the altitude, a new lot whose terrain altitude is within 0.001 of 0 keeps altitude 0 and both
   slopes 0 *[quirk]*. `UpdateSlope` (slot 20, `0x006BF610`) sets only field 8 to `GetLotSlope(lot, false)`.
2. If the strategy byte is `0xFF` (constructor value): `RandomUint32Uniform(100)` on the lot file's `cRZRandomSuper`
   `0x00B49120` (foundations §6.2), one draw. Saved lots have 0…99, so no draw on load.
3. Release the current configuration object. If the configuration ID is non-zero: look it up with the lot
   configuration manager (`0x00B43D18`, slot 8 `GetLotConfiguration(id)`) and keep the result (AddRef). Then set
   the stored ID to 0. If the configuration is missing from the data (for example a removed plug-in), the lot has no
   configuration and its next save writes 0 *[compat]*.
4. Return true.

**The ID read is never written back.** The lot keeps two members: the configuration **ID** read by `Read` (`+0x30`,
the `Read` at `0x006C3621` stores into it) and the configuration **object** (`+0x34`). `Write` (`0x006C0080`)
writes only `object ? object.GetID() : 0` (`0x006C0093`); it never reads `+0x30`. So a lot that was read but never
`Init`ed (the crime-only lot of §14) saves field 2 as **0**, whatever ID it read, and a loaded lot that `Init`
resolved writes the ID of the object found *[compat]*. A writer that round-trips such a lot byte for byte only
does so when the ID read was 0 (the observed crime-only lot is an empty zoned lot, so it read 0).

`PostCityInit` (`0x006BF200`, slot 8): `f := flags; flags := 0; SetFlags(f)`. `SetFlags` acts on a change of bit 0:
when bit 0 becomes set, each cell of the rect is locked in the terrain view at the lot's altitude; when it becomes
clear, each cell is unlocked. So a loaded lot with bit 0 re-locks its terrain *[compat]*.

**`SetFlags(f)`** (slot 13, `0x006BF220`, pop 4), exactly: `old := flags; flags := f` (stored first, always). If
`(old XOR f) & 1 = 0`, return. `v := SL::TerrainView()` (`0x00602160`, terrain.md); if null, return (the flags stay
stored, no cell is touched). Then walk the rect **x outer, z inner**, both inclusive and ascending: for `x` from x1
to x2, for `z` from z1 to z2 (bytes zero-extended; an empty range when x1 > x2 or z1 > z2, which no lot has):
- bit 0 now set: `v.LevelCell(x, z, altitude)` (terrain view slot 16, terrain.md, pop 12; its return is ignored)
  with the lot's stored altitude float as given, re-read for every cell;
- bit 0 now clear: `v.RemoveCellLevelling(x, z)` (terrain view slot 17, pop 8).

Example: rect (10, 20, 11, 21), bit 0 set → LevelCell (10,20), (10,21), (11,20), (11,21) in that order. No other
bit has an effect here; nothing is returned.

`Shutdown` (`0x006C3060`): if initialised: initialised := false; clear the population vector and the
travel-population map; release the configuration and the travel designate; `unit.Remove(lot)` (business unit
slot 4) and drop the unit. Returns true.

Not saved: the configuration object (saved as its ID), the initialised flag, everything derived from the occupant
manager (buildings, props), road-access caches and other runtime members, which their owners recompute.

### 3.6 Lot properties (plain holder) — observed

| Property | Lots | Meaning (owner spec) |
|---|---|---|
| `0x8A0B4100` | 12,977 | uint16[12] trip statistics (`traffic-simulator.md`, `pathfinder.md`) |
| `0xE999B11F` | 12,329 | no-power counter (`lot-developer.md` conventions) |
| `0x4A611773` | 12,280 | traffic random value (`traffic-simulator.md` §4.1) |
| `0x691C863C` | 12,280 | traffic strategy bucket (`traffic-simulator.md`) |
| `0x8999B20B` | 12,177 | no-water counter |
| `0xC999E206` | 12,177 | low-desirability counter |
| `0x2A4F9C74` | 7,737 | network link IDs (`traffic-network.md`) |
| `0x8999B10B` | 1,610 | travel counter |
| `0xC999C45E` | 1,397 | current zot |
| `0xEA54D283…0xEA54D286` | 6 each | locally funded line items (`budget.md`) |

All holders are in ascending ID order with key = ID. No object-valued property occurs.

---

## 4. Business unit (`cSC4BusinessUnit`, `0x4A232DA8`) *[compat]*

| # | Field | Type | Meaning |
|---|---|---|---|
| 1 | version | u16 | 1 (`Read`: anything else → false) |
| 2 | n | u32 | number of member lots (observed 1…269) |
| 3 | n × lot | ref | read IID `0xE818FCED`, appended to the member list |

`Write` returns in AL, `Read` in AL. The lot developer creates units and adds lots (`lot-developer.md` §5.7); each
member lot refers back to its unit (§3.2 field 18). In every save the two directions agree: 7,746 lot → unit
references and 7,746 unit → lot references.

Creation during play is direct from native `PickUnitToJoin`, not through GZCOM (§15.1, LR8). Constructor
`0x006C0800` takes storage in ECX, no stack arguments, returns that storage in EAX, and begins with count 0:
the property-holder constructor `0x005FF100` called on unit +8 clears its +4, i.e. unit +0xC. Adding members
retains both directions: a list node retains its lot (`0x006C09AB`), and `lot.SetUnit` retains the unit
(`0x006C09CB` → `0x006C20D3`). A replacement developer using a factory that supplies one creation reference
must drop that reference after adding the members; the native direct constructor supplies no such reference.

**Membership slots** (`cISC4BusinessUnit`, vtable `0x00AB18DC`; the member list is a `std::list` of AddRef'd lot
pointers whose head is at unit `+0x1C`):

| Slot | Address | Behaviour | Return |
|---|---|---|---|
| 3 `Add(lot)` | `0x006C0940`, pop 4 | Search the list for the pointer `lot` (temporary AddRef/Release around the search). Only if absent: append it at the **back** (one AddRef held by the node). Then **always** `lot.SetUnit(this)` (lot slot 26), also when the lot was already a member | none defined: AL is whatever `SetUnit` left in EAX |
| 4 `Remove(lot)` | `0x006C08B0`, pop 4 | AddRef `lot` (if non-null); erase **every** node equal to `lot` (`0x006C0900`, each erase releases the node's reference; nothing happens when the lot is absent); Release the temporary reference. Then **always**, without a null test and whether or not the lot was a member: `lot.SetUnit(0)` (slot 26) and `lot.SetTravelDesignate(0)` (slot 60) | none defined: AL is the low byte of `SetTravelDesignate`'s EAX, i.e. 0 when the lot had no travel designate, else the low byte of the old designate's `Release()` result |
| 5 `GetMembers()` | `0x008085F0` | the address of the member list (unit `+0x1C`) | EAX |

`SetUnit(u)` (`0x006C20B0`): if `u` differs from the current unit, Release the old one **first**, store `u`, AddRef
it. So `Remove` on the unit's last member drops the unit's last reference inside `Remove` (`Release`
`0x006BFE40` destroys it at a count ≤ 1); `Remove` touches only the lot after that point. `Remove(lot)` on a unit
the lot does not belong to still clears the lot's unit and travel designate (releasing its real unit without
removing it from that unit's list) *[quirk; no caller does this]*. `SetTravelDesignate(0)` (`0x006BF660`) releases
the old designate if there was one.

Neither return value is used: `Shutdown` (`0x006C30F3`) and the lot developer's `RemoveLotFromItsBusinessUnit`
(`0x006C7320`: `lot.GetUnit().Remove(lot)` with no null test, then its unit-changed flag `+0x5E := 1`) ignore
AL. `Shutdown` releases the travel designate before it calls `Remove`, then stores 0 into the unit pointer without
a release (`Remove` already released it through `SetUnit(0)`); it returns AL = 1.

---

## 5. Building occupant (`cSC4BuildingOccupant`, `0xA9BD882D`) *[compat]*

| # | Field | Type | Meaning, observed (12,295) |
|---|---|---|---|
| 1 | version | u16 | 2 |
| 2 | occupant base | §2.3 | type `0x278128A0`; exemplar holder parent = the building exemplar |
| 3 | building type | u32 | the building's type ID (equals the lot's building type, §3.2) |
| 4 | building flags | u8 | bit 0 lit (night lights, `lot-developer.md` §8). 0 in all saves |
| 5 | bounds | box | world bounding box |
| 6 | orientation | s8 | 0…3 |
| 7 | completion | f32 | `SC4Percentage`, observed 0…1 (below 1 only while under construction *[inferred]*) |

`Read`: version must be 2 and the base `Read` must succeed, else false. Holder properties observed on buildings
include `0xEA54D284/7`, `0x08D9763D`, `0x2A58D350`, `0xC910B4CF`, `0x69B425DD/E` and others set by the utility,
budget and protection simulators (their specs).

---

## 6. Lot base-texture occupant (`cSC4LotBaseTextureOccupant`, `0xC97F987C`) *[compat]*

| # | Field | Type | Meaning, observed (13,079) |
|---|---|---|---|
| 1 | version | u16 | 2 |
| 2 | occupant base | §2.3 | type `0x497F6D9D`, holder parent `(0, 0, 0)` |
| 3 | bounds | box | |
| 4 | usage | s8 | 1 (94) or 2 (12,985) |
| 5 | n | u32 | number of texture records, 1…50 |
| 6 | n × texture record | 14 bytes | below |

Texture record (one per textured cell; `lot-developer.md` §6.6):

| Bytes | Field | Observed |
|---|---|---|
| u32 | texture instance | never 0 |
| u8, u8 | cell x, z | 0…126, 0…127 |
| u8 | orientation | 0…3 |
| u8 | configuration object byte 3 | 0, 1 |
| 5 × u8 | unnamed; constructor default `0xFF` (`0x005CA770`) | mostly `0xFF`; other values in up to 1,018 of the 28,815 records, e.g. `FF B2 33 AA 03` (T6) |
| u8 | animation frame | 0…7 |

`Read` resizes the record vector to n (no bound) and reads them. When textures are **set** (not on load), a frame
byte of `0xFF` becomes `RandomUint32Uniform(frames)` on the base-texture file's `cRZRandom` `0x00B23078` if the
texture has ≥ 2 frames (`GetFrameCountInLotBaseTexture` `0x005E08F0`), else 0: one draw per such record
(`SetTextureSpecification` `0x005E0B20`). Saved frames are kept on load.

---

## 7. Retaining-wall occupant (`cSC4LotRetainingWallOccupant`, `0x49C05C9F`) *[compat]*

| # | Field | Type | Meaning, observed (1,735) |
|---|---|---|---|
| 1 | version | u16 | 2 |
| 2 | occupant base | §2.3 | type `0x895D1169` |
| 3 | wall type | u32 | `0xC96D2137` (1,674), `0xC96D2136` (48), `0xC96D2135` (13) |
| 4 | edge mask | s32 | bit i = wall on edge i (0…15 observed) |
| 5 | edge hashes | 4 × s32 | one per edge, 0 = none *[inferred: model key of the edge's wall]* |
| 6 | position | vec3 | |
| 7 | size | vec3 | |
| 8 | lot | ref | read IID `0xE818FCED`; never null |

`Read` releases the old lot, then reads; while reading the hashes it **ORs bit i into the edge mask for every
non-zero hash i**. In 949 of the 1,735 saved walls the stored mask lacks bits of non-zero hashes, so the original
does not round-trip these records: after a load and save the mask grows *[quirk]* (test T7). The walls are created
by `AddRetainingWallOccupant(lot, …)`, a static member called by the lot developer (`AddLotFoundation`).

### 7.1 Building a wall: `AddRetainingWallOccupant(lot, add = true)` (`0x005E3070`, `__cdecl`, 8 bytes)

This is a direct cross-owner helper, not a factory or occupant-interface method. Its native lot-developer
call sites, direct wall constructor and replacement dependency are specified in §15.1 (LR8).

1. If `lot.GetFlags() & 1 = 0` (slot 12): return AL = 0, nothing created.
2. `R := lot.GetBoundingRect()` (slot 14, 4 × s32 x1, z1, x2, z2); `a := lot.GetAltitude()` (slot 16, stored as
   float); `top := 0`.
3. `found := GetLotWallEdgeFlags(lot, a, R, h[4], &top)` (`0x005E2CE0`, below). If false: return AL = 0, nothing
   created.
4. Create the wall (0x6C bytes, constructor `0x005E2F40`) and AddRef it. `t := GetRetainingWallType(lot)`
   (`0x005E2ED0`, lot-configuration.md: one uniform draw on `0x00B230C8` from the configuration's list, 0 when the
   lot has no configuration or the list is empty). If `t ≠ 0` and BDS (`0x00B43D68`) slot 29
   `LoadLotRetainingWallProperties(wall, t)` returns false: Release the wall, return AL = 0. (`t = 0` skips that
   call and builds the wall anyway.)
5. Edge mask (wall slot 8) := `(h[0] ≠ 0) | (h[1] ≠ 0) << 1 | (h[2] ≠ 0) << 2 | (h[3] ≠ 0) << 3`.
6. Position (wall interface at `+4`, slot 9) := `(f32(16·(x1 + x2 + 1)) × 0.5, a, f32(16·(z1 + z2 + 1)) × 0.5)`
   (the rect centre in metres, as `GetCenter`, §3.5). Size (wall slot 4) := `(16·(x2 − x1 + 1), top − a,
   16·(z2 − z1 + 1))`, the middle one a float subtraction of the two stored floats.
7. Wall slot 6 := `lot` (the lot reference); wall slot 10 := `h` (the four edge hashes); wall slot 12 := `t` (the wall
   type; 0 when `t = 0`); wall `+4` slot 3; insert into the occupant manager (`0x00B43D0C` slot 20, with the
   wall's slot 3 (0) result); `lot.SetFlags(lot.GetFlags() | 4)`; Release the creation reference.
8. Return AL = **0** even on success: the result byte (`[esp+0xF]` of the frame) is set to 0 at entry and never
   written again (`0x005E325B`) *[quirk; LD3 callers ignore it]*.

**`GetLotWallEdgeFlags(lot, a, R, h[4], &top)`** (`0x005E2CE0`): `top := f32(a + 0.1)` (float 0.1 at
`0x00A8C950`); `found := false`. For each edge `i` = 0, 1, 2, 3 (flag `1 << i`, sides as in zones.md slot 10):
1. `h[i] := 0`; `need := false`; `bits := 0`; `b := 0x80000000`.
2. If lot manager (`0x00B43D08`) slot 12 `GetNeighborCellsAtEdgeOfLot(R, 1 << i, C)` returns false: next edge.
3. For each cell of `C`, x outer, z inner, ascending inclusive: `L := lotManager.GetLot(x, z, false)` (slot 6). If
   `L` is null or `L.GetFlags() & 1 = 0`: `need := true`. If `L` is non-null and `L.GetState() ≠ 6` (slot 33):
   `bits |= b`. Then `b >>= 1`, and `b := 0x80000000` when it reaches 0 (so the 33rd cell reuses bit 31).
4. If `need`: `h[i] := bits`; `V :=` slot 11 `GetVerticesAtEdgeOfLot(R, 1 << i, V)` (its return is not tested); for
   each vertex of `V`, x outer, z inner, ascending inclusive: `found := true`; `y := terrain.GetAltitudeAtVertex(x,
   z)` (`0x00B43CF4` slot 14, terrain.md); if `y > top` (x87 compare; NaN no): `top := f32(y)`; then
   `h[i] += round(y) · x · z + 1` in 32-bit wrap-around arithmetic, where `round(y)` is
   `bits(f32(y + 12582912.0)) − 0x4B400000` (`0x005E2E6A`–`0x005E2E87`): round to nearest, ties to even, exact for
   |y| < 2^22.
5. Return `found` in AL (true when at least one edge needing a wall had a vertex).

So `h[i] = 0` means no wall on edge `i` (no neighbour row, or every neighbour cell is a terrain-locked lot), and
otherwise the hash mixes which neighbour cells hold non-state-6 lots with the rounded vertex altitudes. `top` is the
highest edge vertex (at least `a + 0.1`), so the wall is `top − a` high. A wall built with `t = 0` has the type the
constructor gives (not observed in saves: every saved wall has one of the three types).

**`AddRetainingWallOccupant(lot, add = false)`** (removal, `0x005E3270`–`0x005E32CB`): `(x, z) :=
lot.GetLocation()` (slot 21, the corner cell of §3.2 field 5); `w := null`; if occupant manager slot 33
`GetFirstOccupantByStandardCityCell(&w, x, z, 0x895D1169)` returns true and `w` is non-null: occupant manager slot
21 `RemoveOccupant(w, 1, 0)`, Release `w`, result := 1. Then **always** `lot.SetFlags(lot.GetFlags() & ~4)`. Return
AL = result (0 or 1). Only the first wall found at the location cell is removed; any other wall of the lot stays.

### 7.2 Refreshing walls: `UpdateRetainingWalls(lot)` (`0x005E37F0`, `__cdecl`, 4 bytes, no return)

Native lot-developer slot 62 wraps a direct call to this helper (§15.1); substituting wall factories does
not redirect that call. The lot argument is borrowed; temporary occupant/interface references are released
by the operations below.

Static RE (re_query.py asm; REA had no binary open). Callers: lot developer slot 62 `UpdateLotRetainingWalls`
(`0x006C4190`, which passes AL through) and `UpdateSurroundingRetainingWalls` (`0x005E39A8`). In this order:

1. If `lot.GetFlags() & 1 = 0` (slot 12): return; nothing else is called.
2. `w := null`; `(x, z) := lot.GetLocation()` (slot 21); occupant manager (`0x00B43D0C`) slot 33
   `GetFirstOccupantByStandardCityCell(&w, x, z, 0x895D1169)`. If false: `AddRetainingWallOccupant(lot, true)` (§7.1)
   and return.
3. `W := w.QueryInterface(0x295D1162)` (wall interface, building-development.md DG35). **`w` is not tested for
   null** before this call (the removal path of §7.1 does test it). If the QI fails:
   `AddRetainingWallOccupant(lot, true)`, Release `w`, return.
4. `R := lot.GetBoundingRect()` (slot 14); `top := 0`; `a := f32(lot.GetAltitude())` (slot 16);
   `old := W.GetEdgeHashes()` (slot 9 `0x005E2910`, copies object `+0x44..+0x50`).
5. `GetLotWallEdgeFlags(lot, a, R, new[4], &top)` (§7.1); its AL and `top` are **ignored**.
6. For `i` = 0, 1, 2, 3: if `old[i] ≠ new[i]` (32-bit integer compare): if `old[i] ≠ 0`,
   `AddRetainingWallModel(w, W, i, false, false)`; then if `new[i] ≠ 0`, `AddRetainingWallModel(w, W, i, true,
   true)`; `changed := true`. Equal hashes (zero or not) do nothing.
7. If `changed`: `W.SetEdgeHashes(new)` (slot 10 `0x005E2930`, a plain copy of the four words into `+0x44..+0x50`;
   it does **not** touch the edge mask at `+0x40`).
8. Release `W`, then Release `w`. Return (AL is whatever the last call left; slot 62 hands it on; LD3 callers
   ignore it).

What an update does **not** change on an existing wall: position, size (`top` is discarded, so a wall keeps the
height it was built with), wall type, lot reference, lot flag bit 2. A wall whose new hashes are all 0 is **kept**
(its models removed, the occupant stays). The edge mask changes only through `AddRetainingWallModel` below.

**`AddRetainingWallModel(occ, W, edge, add, insert)`** (`0x005E3400`, `__cdecl`, 20 bytes, AL = 1 on every path):
1. `P := occ.AsPropertyHolder()` (occupant slot 6, `occ + 8`); if null: return.
2. `G :=` occupant slot 20 `GetOrCreatePlaceableObject()` (`0x005E91C0`) when `add`, slot 19 (the existing one at
   `+0x30`, may be null) otherwise; if null: return; AddRef `G`.
3. Removal (`add = false`): `FindRetainingWallModel(G, edge, &m)` (`0x005E2FC0`: asks `G` slot 4 for at most 16
   attached objects; for each, QI `0xA9E2831E`, its slot 3 object `m`; match when the signed byte `m + 0x78` equals
   `edge`; first match is AddRef'd and returned true). If found: if renderer (`0x00B43DD0`) slot 27
   `IsModelInserted(m)`: slot 29 `RemoveModel(m)` (the renderer is not null-tested here); `G` slot 6 detach `m`;
   `m` slot 4 `SetOccupant(null)`; Release `m`. **The mask bit is left as it was**; `insert` is unused.
4. Add (`add = true`): create a model (`0xB0` bytes, constructor `0x005E17C0`, AddRef). Read properties
   `0x295961F2` (vector of u32, starts as five zeros) and `0x295961F3` (one word) from `P` when `P` has
   `0x295961F2` (slot 9), else from the resource `{type 0x6534284A, group 0x88CD66E9, instance 1}` (resource manager
   `0x00B43CA8` slot 3, IID `0xA52160F5`), Released after; the word is not initialised by this function when neither
   source has it. `occ` slot 10 gives the box (min starts (1,1,1), max (0,0,0); `0x005E2C30` derives it from the
   wall's stored position and size, so a stale size stays stale). `0x005E12C0(model, vector, word)`; `Init(model,
   box, edge, &ok)` (`0x005E2780`: stores `edge` at `+0x78`, builds the edge vertices; `ok` is written by its vertex
   setup `0x005E1BC0`).
   - `ok`: model slot 4 `SetOccupant(occ)`; `G` slot 5 attach the model; if `insert` and the renderer is non-null and
     not `IsModelInserted(model)`: `InsertModel(model)` (slot 28). Mask := `W.GetMask() | (1 << edge)` (slots 7/8,
     `+0x40`).
   - not `ok`: mask := `W.GetMask() & ~(1 << edge)`; the model is not attached.
   Release the model (destroyed when not attached), free the vector, Release `G`.

So after an update the saved record (§7 fields 4 and 5) holds the new hashes; mask bits are set for edges whose new
model built, cleared for edges whose new model failed, and **stay set for edges that went to hash 0**. A failed
build leaves a non-zero hash with a clear mask bit, one way to get the masks lacking hash bits that `Read` repairs
(T7) *[inferred: the cause of the 949 saved cases is not traced]*.

**`UpdateSurroundingRetainingWalls(lot)`** (`0x005E3960`, `__cdecl`, 4 bytes, no defined return): allocate an empty
list (12-byte head, `0x0090CF54`); `n :=` lot manager (`0x00B43D08`) slot 9 `GetLotsSurroundingLot(lot, list, 0xF)`
(zones.md §8.3 slot 10: the ring one cell outside the rect, row order, each lot once, one AddRef each; the lot itself
is excluded because it intersects the rect). If `n ≠ 0`: `UpdateRetainingWalls(L)` for each `L` in list order. Then
Release every non-null element, free the nodes and the head. The lot passed in is **not** updated itself. Callers:
`ClearLot` (`0x006C8A62`, after `AddRetainingWallOccupant(lot, false)`, `RemoveLotFoundation` and
`UpdateSurroundingLotFoundations`) and `AddLotFoundation` (`0x006C8C34`); see lot-developer.md.

---

## 8. Foundation occupant (`cSC4FoundationOccupant`, `0x49C05C8F`) *[compat]*

| # | Field | Type | Meaning, observed (8,981) |
|---|---|---|---|
| 1 | version | u16 | 2 |
| 2 | occupant base | §2.3 | type `0x68FD0C69` |
| 3 | foundation type | u32 | 34 distinct values, e.g. `0x194B1000` (4,637), `0x890B7313` (1,704) |
| 4 | maximum height | f32 | 0…32 |
| 5 | position | vec3 | |
| 6 | size | vec3 | |
| 7 | usage | s8 | 1, 2 |
| 8 | orientation | s8 | 0…3 |

---

## 9. Construction occupant (`cSC4ConstructionOccupant`, `0xA97F909E`) *[compat]*

A building under construction is a construction occupant on the lot plus the building occupant (completion < 1);
the lot is in state 2. Field names after #12 are not known; their grouping follows the Write order (§16 Q3).

| # | Field | Type | Meaning, observed (31) |
|---|---|---|---|
| 1 | version | u16 | 5 |
| 2 | occupant base | §2.3 | type `0x096E763C`, holder parent `(0, 0, 0)`, one property `0xAA1DD396` |
| 3 | lot | ref | IID `0xE818FCED` |
| 4 | building | ref | IID `0x87DDA3A9` (class `0xA9BD882D`) |
| 5 | transform | xform | mask 0 in all saves |
| 6 | world bounds | box | |
| 7 | local bounds | box | |
| 8 | has crane | bool | true when a crane model exists (4 of 31) |
| 9 | crane position | vec3 | |
| 10 | ground props | u32 n + n × ref | IID `0xE9793A65` (prop class `0x2977AA47`); 0…3 |
| 11 | roof props | u32 n + n × ref | same; 0…1 |
| 12 | effects | u32 n + n × ref | IID `0x2977AA49` (effect props), written as the effect-prop interface pointer; 1…3 |
| 13 | current effect | ref | IID `0x2977AA49` |
| 14 | unnamed | s8, 5 × u32, 2 × f32, 5 × u32, s8, 2 × f32, bool, 2 × u32, 2 × f32, u8 | construction / crane animation and daily progress state (decoded below) |
| 15 | crew | u32 n + n × {bool, bool, ref} | ref IID `0xA9B40F05` (`cISC4Automaton`; class `0x896E75AF` `cSC4Pedestrian`, record in `save-records.md`); n 0…5 |

`Read`: versions < 4 → false; versions ≥ 4 are read with this layout, version 4 using the legacy transform reader
(`0x005DA930`). Versions > 5 are read as 5 *[quirk]*.

### 9.1 Where the fields live (`0x005CD250` / `0x005CE770`)

The class embeds the occupant base at `+0x08` (constructor `0x005D00F0`: `cSC4Occupant::cSC4Occupant` `0x005E9220`
with ECX = this+8), so base fields are at object +0x2C onward. Object size `0x1A0`; GZCOM creator `0x005D0BC0`
passes argument 1, which selects the per-instance-table vtable pair (`+0x3C` = `0x00AA211C`, `+0x40` = `0x00AA2264`,
and `+0x19C` = `0x00AA1F38`); argument 0 leaves them shared.

**Three `this` frames** *[compat]* (answers OP13/OP14, 2026-10-21). The class's methods run on different
sub-objects, so the same field has three offsets in the binary. Every offset in this section and in §9.2 is given
in the **object frame** (the address the GZCOM creator returns, main vtable `0x00AA2250` at +0):

| Methods | `this` | Object offset = |
|---|---|---|
| `SetLot`, `DoDailySim`, `DoAnimationTick`, `AnimateCrane`, `RebuildModel`, `CreateSparks`, the `Attempt*Crane` steps | object + 0 | the offset as coded |
| `DoMessage` `0x005D1090` (message-target vtable `0x00AA2240` at +4) | object + 4 | coded + 4 |
| `Init` `0x005D1300`, `Shutdown`, `GetPosition`, `GetBoundingBox` (occupant vtable `0x00AA21B8` at +8) | object + 8 | coded + 8 |
| `Write` `0x005CD250`, `Read` `0x005CE770` (serializable vtable `0x00AA219C` at +0xC) | object + 0xC | coded + 0xC |

Earlier revisions of this table used the Write-frame offsets (12 lower than the object frame), which made the
saved lot / building references look like separate copies of the live ones and shifted every #14 role by three
fields. The saved fields, in Write order (object frame; the Write-coded offset in brackets):

| Offset [Write] | Type | Role, observed | Record field |
|---|---|---|---|
| `+0x68`/`+0x6C` [`+0x5C`/`+0x60`] | 2 × ref | the **live** lot / building references that `SetLot` holds (there is no separate saved copy); `Read` releases the old ones and reads into the same fields | 3, 4 |
| `+0x70` [`+0x64`] | xform | `operator<<` `0x005DA8C0`, read with `0x005DA930` (v4) or `0x005DA710` (v5) | 5 |
| `+0xA8` [`+0x9C`] | box | world bounds; the box `GetPosition` / `GetBoundingBox` read (their frame offset `+0xA0` = object `+0xA8`); `RebuildModel` writes it as the scaffold box `+0xC0` widened by sfPostWidth in x and z (`0x005CB76D`–`0x005CB799`; OP17, re-construction 2026-10-23) | 6 |
| `+0xC0` [`+0xB4`] | box | the scaffold box `RebuildModel` stores (x / z = building footprint ∓/± 1.0, constant `0x00B0A7D0`); used by `CreateConstructionCrew`, `AddRoofProps`, `CreateSparks` | 7 |
| `+0x104` [`+0xF8`] → `+0xD8` [`+0xCC`] | bool | written as (live crane model pointer `+0x104` != 0; `CreateCrane` sets it); read into the load-only flag `+0xD8`, never into `+0x104`. `Read` also sets `+0xD9` = 1 (refresh pending, §9.2) | 8 |
| `+0x108` [`+0xFC`] | vec3 | crane position (`0x005DA580`) | 9 |
| `+0x128`, `+0x12C` [`+0x11C`, `+0x120`] | ground / roof-prop lists | `std::list`, nodes at +8 hold the prop ref (IID `0xE9793A65`); count then refs | 10, 11 |
| `+0x130`/`+0x134` [`+0x124`/`+0x128`] | spark-effect vector | `vector<cRZAutoRefCount<…>>` begin/end; count then one ref per entry (pointer + 4 of each element, IID `0x2977AA49`) | 12 |
| `+0x13C` [`+0x130`] | scaffold effect | written as (pointer + 0x10) or 0 (IID `0x2977AA49`); `DoDailySim` switches it to removal (its slot 8, argument 1) | 13 |
| `+0x140` [`+0x134`] | s8 | **state** (ld §6.7.5, 0…4); stored as int32, written as its low byte, read sign-extended | 14a |
| `+0x144` [`+0x138`] | u32 | `T0` (sim day of `SetLot`) | 14b |
| `+0x148` [`+0x13C`] | u32 | unused: 0 from the constructor, only written and read *[inferred: no other access in the class]* | 14b |
| `+0x14C` [`+0x140`] | u32 | `nextDay` | 14b |
| `+0x150` [`+0x144`] | u32 | `N` (segments, ≥ 1; `RebuildModel`) | 14b |
| `+0x154` [`+0x148`] | u32 | `counter` (current segment) | 14b |
| `+0x158` [`+0x14C`] | f32 | scaffold span `N · segH` (`RebuildModel`) | 14c |
| `+0x15C` [`+0x150`] | f32 | `startY` (grown height + 0.1, `RebuildModel`) | 14c |
| `+0x160`…`+0x16C` [`+0x154`…`+0x160`] | 4 × u32 | `A`, `B`, `C`, `E` (ld §6.7.4) | 14d |
| `+0x170` [`+0x164`] | u32 | `msAcc` (animation milliseconds since the last phase change) | 14d |
| `+0x174` [`+0x168`] | s8 | crane state (0 swing, 1 pause, 2 pick target; ld §6.7.6); int32 in memory, read sign-extended | 14e |
| `+0x178`, `+0x17C` [`+0x16C`, `+0x170`] | 2 × f32 | crane swing limits min / max (radians, `AttemptAddOuterCrane`, ld §6.7.6) | 14f |
| `+0x180` [`+0x174`] | bool | unused: false from the constructor, only written and read *[inferred: no other access in the class]* | 14g |
| `+0x184`, `+0x188` [`+0x178`, `+0x17C`] | 2 × u32 | crane accumulator and duration (ms) | 14h |
| `+0x18C`, `+0x190` [`+0x180`, `+0x184`] | 2 × f32 | crane target angle and previous target angle (radians) | 14i |
| `+0x124` [`+0x118`] | u8 | crane type index (into the boom-radius table `0x00B22F70`; set at `0x005CDC32`, passed to `CreateCrane`) | 14j |
| `+0x194` [`+0x188`] | crew list | `std::list`, each node {u8 at +8, u8 at +9, 16-byte sub-object at +0xC…}; count then {bool, bool, ref} per entry | 15 |

The Write writes the crew count from the `+0x188` list, then per node reads two bytes (`operator>> bool`
`0x005D9FA0` on Read) and one `SC4ReadGZSerializable` reference with IID `0xA9B40F05`.

### 9.2 Life cycle (`0x005D1300` Init, `0x005D0890` SetLot, `0x005D0680` DoDailySim)

- **`Init`** (`0x005D1300`, occupant frame; answers OP15 / OP7, 2026-10-21), in this order: return true at once if
  the occupant's own `IsInitialized` (occupant slot 5) is true; run the occupant base `Init` (`0x005E8C90`, §2.4)
  **unconditionally**; `StaticInit` once per process (flag `0x00B22F60`); subscribe **four** messages through the
  message server `0x00B43CCC` slot 0x14, target = the message-target sub-object at object `+0x4`: `0x66956814`,
  `0xA6B79602`, `0x06CDB65B`, `0x26D31EC1` (`0x86AD10EE` is **not** subscribed: the animation-tick manager calls
  `DoMessage` directly, lot-developer §6.7.8); register with the animation-tick manager (service
  `0x00602600`, slot 0x14, priority `0x80`); fill 6 × 5 `cISC4SimGrid<float>*` slots from the framework
  (`0x1AC0E11A`/`0xFAC0E219` service, table `0x00AA1E30…0x00AA1E48`); pack the render state at object `+0xF8`
  (`0x007D2B50`); finally call **occupant slot 23 `AddOccupantGroup(0x5007)`** on itself (`CALL [EAX+0x5C]` at
  `0x005D1505`, `ECX` = the occupant sub-object, vtable `0x00AA21B8` slot 23 = `0x005E9350`, one stack argument,
  callee pops 4). Its compiled body (`0x005E9350`): read the occupant's u32-array property `0xAA1DD396` (the
  property of §9 #2), and if `0x5007` is not in it (`std::find` `0x004696C0`), append it, write the property back
  (`0x005FE6B0`) and post the queued message `0xEBFF18A3` (occupant slot 15 `0x005E9010`, carrying the occupant and
  the group ID); if it is already present nothing happens. The return value is not used. So a construction occupant
  always carries occupant group `0x5007` in property `0xAA1DD396`; a loaded one already has it from its base record
  and gets no second entry and no message.
- **`DoMessage`** (`0x005D1090`, object `+0x4` frame): `0x66956814` → `DoDailySim`; `0x86AD10EE` →
  `DoAnimationTick(ms)` with `ms` = the message's dword at `+0x14` (message slot 14 getter; the field the tick
  manager fills with the clamped **µs**, called data2 in lot-developer §6.7.8) divided by 1000, unsigned and
  truncating (`MUL 0x10624DD3; SHR 6`); `0x06CDB65B` → the common flag-2 renderer dance (§10.3); `0xA6B79602` → sets
  byte object `+0x100` = 1 when the dwords at +4 of the two objects returned by message slots 10 and 15 differ; `0x26D31EC1` → the refresh below. All
  return true.
- **Refresh after a load** (`0x26D31EC1`, `0x005D10C7…0x005D1155`): only when the refresh-pending byte `+0xD9` is set
  (`Read` sets it, nothing else does): `RebuildModel`; if the load-only has-crane byte `+0xD8` (record #8) is set,
  `CreateCrane(position x +0x108, position z +0x110, crane type +0x124)`; `AddToRenderer`; then `DoAnimationTick(0)`,
  with `state` temporarily set from 2 to 1 around the call when it is 2 (so a state-2 load redraws the scaffold at
  the state-1 height instead of re-setting the building completion), and clear `+0xD9`. The crane model therefore
  exists only after this refresh; a `Write` before it saves record #8 = false *[quirk]*.
- `Shutdown` (`0x005D0CE0`) unsubscribes the four, unregisters the tick, sends `SendConstructionMessage(0xC9E7B1E8, 0)`
  first, and releases the lot/building/props/effects/crew.
- **`SendConstructionMessage(type, data3)`** `0x005CBD20`: posts a `cRZMessage2Standard` (0x2C bytes) with type =
  arg1, void1/data2 = this, data3 byte = arg2, through the message server slot 0x10 (queued).
- **`SetLot(lot)`** `0x005D0890` (also reachable with the "already initialised" guard): releases the old lot and
  building references (`+0x68`/`+0x6C`), holds the new lot, and when the lot has a building (`lot` slot 0xC0/4=48,
  `GetBuilding`) drives the whole placement: `RebuildModel` `0x005CAD60`, `DivideLot` `0x005CB9F0`,
  `AttemptAddCrane` `0x005CE360`, `AddGroundProps` `0x005CF790`, `CreateConstructionCrew` `0x005CFA60`,
  `AddRoofProps` `0x005CFC40`, `CreateDustClouds` `0x005CBC10`, `CreateSparks` `0x005CEC20`,
  `CreateLotTexture` `0x005CF380`. It then reads the sim day (`0x00B43D5C` slot 0x20/4=8) into `+0x144`, computes
  the segment schedule into `+0x150…+0x164` from the lot's growth rate (`0x00B0A7D8`), resets state `+0x140` = 0,
  sets completion 0 (building slots 9/10 + a queued `0x2977AE2F` message), sets the scaffold height to
  ground + 0.001 (`0x005CC020`), and if a crane exists (`+0x104` != 0, `0x005D0B46`): target `+0x18C` := min
  `+0x178`, previous `+0x190` := max `+0x17C`, accumulator `+0x184` := 1, duration `+0x188` := 0, crane state
  `+0x174` := 2, and `FUN_005CBE10(min)` poses the boom at the min angle (`0x005D0B4E…0x005D0B87`). Finally `SendConstructionMessage(0x49E7B1E2, 1)` (segment-start), then
  `SendConstructionMessage(0xC9E7B1E8, 1)` and `AddToRenderer` `0x005CA960`.
- **`DoDailySim`** `0x005D0680` matches lot-developer.md §6.7.5 (segment state machine at `+0x140`; sends
  `SendConstructionMessage(0x49E7B1E2, 1)` at segment starts, completion at `0x5D0723`–`0x5D0790` fires the sparks
  iterator over the `+0x12C` list and the effect slot 9/7 dance). This class's copy adds the day-stamp
  bookkeeping: state 0/2 path at `0x005D0701`, and the completion path walks the effect list and drives
  `SendConstructionMessage(0xC9E7B1E8, …)`.
- **`GetPosition`** `0x005CA7D0` / **`GetBoundingBox`** `0x005CA820` (occupant frame) read the saved world box,
  record #6 at object `+0xA8…+0xBC` (their coded `+0xA0…+0xB4`); `GetPosition` returns the box centre (each
  coordinate `(min + max) * 0.5f`, constant `0x00A84D2C`).
- **`DoAnimationTick`** `0x005CE110` (lot-developer §6.7.6) adds `ms` to `msAcc` `+0x170` and interpolates by
  `state` `+0x140`; it is also called with 0 by the refresh above.
- The life-cycle fields (state, `T0`, `nextDay`, `N`, `counter`, span, `startY`, `A`, `B`, `C`, `E`, `msAcc`, the
  crane fields) all sit in record #14 as tabled in §9.1, so a loaded construction resumes its schedule from the
  saved values with no recomputation: `DoDailySim` reads only `state`, `nextDay`, `counter`, `N`, `B`, `C`, `E`;
  `RebuildModel` (refresh) recomputes `N`, span and `startY` from the building as before the save.

---

## 10. Prop occupants

### 10.1 `cSC4PropOccupant` (`0x2977AA47`) *[compat]*

| # | Field | Type | Meaning, observed (134,420 + 948 inside effect props) |
|---|---|---|---|
| 1 | version | u16 | 6 |
| 2 | occupant base | §2.3 | type `0xA823821E` |
| 3 | prop type ID | u32 | the prop exemplar's ID. In memory the prop keeps a u16 index into a process-wide ID table (`GetPropTypeIndex` `0x005EF2E0` appends unknown IDs on read) |
| 4 | bounds | box | |
| 5 | orientation | s8 | 0…3 |
| 6 | state | u8 | 0…2 |
| 7 | time of day start, end | 2 × u8 | tenths of an hour; (0, 0) = always (120,401); e.g. (180, 80) = 18:00–08:00 |
| 8 | has date range | bool | true for 174 props |
| 9 | date range (only if #8) | 4 × u32 | interval in days (365 in all saves), duration (4…28), start day, end day |
| 10 | chance | u8 | 1…100 (percent) |
| 11 | prop usage | u8 | 1, 2, 4 |
| 12 | lot tag | u32 | tag of the lot configuration object that placed it (`lot-developer.md` §6.3); `0xFFFFFFFF` = none (221) |
| 13 | timer bits | u8 | 0, 6, 7, 11, 13, 14, 15 |

`Read` does **not** check the version: any version is read with this layout; field 13 only if version > 5, else
it becomes `0x0F` *[compat]*. If the base `Read` fails, `Read` returns `GetError() = 0` without reading more (so it
can return true). Prop holders observed: `0x89A1C16C` (8,124, unnamed), `0x48E95539` (zot number,
`lot-developer.md`), `0xCAA46595`, `0xCAA45670`, `0xCAA45672` (`neighbors.md`).

### 10.2 `cSC4PropOccupantVisualEffect` (`0x2977AA49`) *[compat]*

| # | Field | Type | Meaning, observed (948) |
|---|---|---|---|
| 1 | version | u16 | 3 (`Read`: anything else → false) |
| 2 | prop record | §10.1 | complete, from its own version 6 |
| 3 | unnamed | u8, u8 | (1, 0) 876, (1, 1) 40, (0, 1) 32 |
| 4 | effect name | gzstr | |
| 5 | unnamed | u32 | 0 in all saves |

*[quirk]* `Write` writes an **empty name and 0** instead of #4 and #5 whenever the effect is live (either of its two
effect handles is non-null). So lot effect props (849, exemplar parent) are saved with empty names; only the
construction effects (99, parent `(0, 0, 0)`) carry names: `welding_sparks` (68), `large_dust` (20),
`medium_dust` (11). On load the name comes back from the exemplar *[inferred]*.

### 10.3 `cSC4SignpostOccupant` (`0xAB72FBB3`) *[compat]*

A region-view signpost: the user places it with the sign tool and edits its text on the region map. It embeds a
complete `cSC4PropOccupant` at `+0x04` (constructor `0x005F5519` calls the prop constructor with ECX = this+4), so
every prop field and the prop save record (§10.1) live at object +4 onward; the base occupant sits inside the prop
at +0x0C (prop-relative +8), and the prop's serializable Write/Read (`0x005ECE90`/`0x005EF360`, record §10.1) are
called inline by the signpost's own Write/Read. Object size `0x590` (the allocator pool at `0x00B0A96C`).

| # | Field | Type | Meaning |
|---|---|---|---|
| 1 | version | u16 | 2 |
| 2 | prop record | §10.1 | complete inline, from the prop's own version 6 (base occupant type `0xA823821E`) |
| 3 | text | string | the sign text (object `+0x98`), stream slot 15 (`+0x3C`, a string; OP2) |
| 4 | position | vec3 | the model's position, object `+0xDC/+0xE0/+0xE4` (`operator<<` `0x005DA580`) |

*Corrected 2026-10-22 (re-signposts): `Write` (`0x005F5080`) and `Read` run on the `+0x10` serialisable view, so their
`+0x88` and `+0xCC…+0xD4` are object `+0x98` (the text) and `+0xDC…+0xE4`; `SetPosition` runs on the `+0x0C`
occupant view, whose `+0xD0…+0xD8` are the same three object fields (`0x005F4E9F`–`0x005F4EAE`). The offsets in
the next paragraph are those views'.*

The vec3 is the signpost's own position: `SetPosition` (occupant view slot 9, `0x005F4E90`) adds the requested
delta into six 3×3 orientation matrices at `+0xDC…+0x118` (one per orientation), writes the absolute position into
`+0xCC/+0xD0/+0xD4`, sets byte `+0xA9` bit 0 (moved) and increments byte `+0xA8` (a change counter), translates the
box, and re-registers with the renderer (`0x00B43DD0` slot 0x7C) when flag 2 is set. `Read` restores the vec3,
increments `+0xA4` (a load counter) and sets `+0xA9` |= 1. Version 1 records additionally read one u32 (slot 0x24)
whose value is discarded *[compat]*.

State beyond the prop (measured on the object base):

| Offset | Type | Role |
|---|---|---|
| `+0x6C`/`+0x70` | 2 × ptr | embedded `cS3DModelInstanceBasic` (`0x00801CF0`) sub-object; `+0x70` gets vtable `0x00AA5298` (edit mode) or `0x00AA5498` (game mode) from the constructor's bool argument |
| `+0x88` | u32 | not saved (corrected: record field 3 is the text at `+0x98`) |
| `+0x98` | `cRZString` | the sign text; constructor installs the shared empty-string vtable `0x00A80810` |
| `+0x9C…+0xA4` | 3 × ptr + u8 | model-table back pointer, edit target, load counter (`+0xA4`) |
| `+0xA8`, `+0xA9` | 2 × u8 | change counter and moved flag (bit 0) |
| `+0xAC` | u8 | edit mode flag (`SetEditMode` arg) |
| `+0xAD` | u8 | blink flag (toggles every 500 ms of accumulated `+0xB0` time while editing) |
| `+0xB0` | u32 | accumulated tick milliseconds |
| `+0xB4`, `+0xB5` | 2 × u8 | model-dirty flags with change counters (RebuildModel clears 0xFD / 0xFB bits, sets bit 2) |
| `+0xB8…+0x118` | 4 × 3×3 f32 | four orientation matrices (identity when not loaded) |
| `+0x11C…+0x134`, `+0x138…+0x150` | 2 × 6 × f32 | per-orientation corner positions (FUN_005F4D10 fills them) |
| `+0x194` | 16-byte | message target sub-object |
| `+0x4BC` | `cRZString*` | owned string (freed in the destructor) |
| `+0x4C` | ptr | owned date-pair record (freed by `FUN_005EE160`) |
| `+0x4D4…+0x507` | 4 rows × 4 bytes | corner colours copied from the static tables at `0x00AA5204` (row 0 = static colours, row 1 = dynamic) |
| `+0x54C…+0x570` | 10 × ptr | the ten piece textures, one per entry of the RID table `0x00AA5214…0x00AA523C` (instances `0x8B4A6560`–`0x8B4A6565`, `0x8B4A6564`, `0x8B4A6565`, `0x8B4A6566`, `0x8B4A6567`; keys `0x7AB50E44` / `0x1ABE787D`); `Init` fills them through the `+0x0C` view (`+0x540` there) |
| `+0x574…+0x57C` | vector | the text textures `LoadTextures` makes (§10.3 below); `Shutdown` empties it through the `+0x0C` view (`+0x568` there). The element name in the export comes from a folded template (`0x00527180`) |
| `+0x580` | | packed render state (`cSC4PackedRenderState::PackRenderState` `0x007D2B50`; `Init`'s `+0x574` is the `+0x0C` view's, `0x005F744F`) |

Interfaces (sub-object map): `+0x00` the 8-slot class view `0x00AA5478` (`cSC4SignpostOccupant`: QI `0x005F4930`
answers `0x4B44FBE2` with this, otherwise `ADD ECX,4` and the prop QI; slot 3 `AsOccupant` `0x005F8400` returns
this+0xC; slot 4 `SetText` `0x005F7600`; slot 5 `GetText` `0x005F5920`; slot 6 `SetDynamic` `0x005F4BA0`; slot 7
`SetEditMode` `0x005F7320`); `+0x04` the prop's `cISC4PropOccupant` view (24 slots, `0x00AA5418`, QI-4 `0x005F5950`);
`+0x08` `cIGZMessageTarget2` (QI-8 `0x005F5960`, `DoMessage` `0x005F7510`); `+0x0C` the 34-slot `cISC4Occupant`
view `0x00AA5380` (QI-12 `0x005F5970`; overrides `Init` `0x005F73A0`, `Shutdown` `0x005F5E20`, `GetPosition`
`0x005ED1F0` = box centre from the prop box at object +0x40…+0x54, `SetPosition` `0x005F4E90`, slots 27/28 =
`cSC4PropOccupant::GetLotTag`/`SetLotTag` `0x005ECAC0`/`0x004302C0`, `SetAllFlags` `0x005F4A90`, everything else
the base); `+0x10` `cIGZSerializable` `0x00AA5364` (Write `0x005F5080`, Read `0x005F50F0`, `GetGZCLSID`
`0x005F5940`); `+0x14` the 26-slot `cSCExemplarPropertyHolder` view `0x00AA5300` (slots 3–21 shared holder
implementations; 22 `0x005F59A0` and 23/24 `0x005F5AC0`/`0x005F5AD0` are `SUB ECX,4` thunks into Write/Read);
`+0x28` `cISCExemplarPropertyHolder` `0x00AA52E8`; `+0x6C` a `cSCExemplarPropertyHolder`-base view `0x00AA52E4`
(`SUB ECX,0x6C` thunks; the holder base at prop-relative +0x68).

Behaviour:

- **Creation** — GZCOM creator `0x005F5CE0` allocates from pool `0x00B0A96C` and calls the constructor with
  argument 1. The sign tool (`cSC4ViewInputControlSignTool::OnMouseDownL` `0x004D81D0`, call at `0x004D82D9`)
  creates it with argument 1, takes a reference and selects it for editing. Registered in the COM director
  `0x00460090` at `0x0046083F` with `GetGZCLSID` `0x005F5940`.
- **`Init`** `0x005F73A0` — if already initialised return true. When not loaded (status bit 1 clear): zero and
  identity-fill the four matrices (`+0xE0…+0x118`, the three at `+0x40…+0x48` too), set the prop type to
  `0xAB72FBB3` (prop-view slot 8 `SetPropType` `0x005EF4C0`, which interns the ID in the process-wide table and
  writes the index — the saved record keeps the ID), set flag 0x10 (occupant view slot 30 `SetAllFlags`). Then the
  prop's `Init` (`0x005EC940`), `PackRenderState(+0x574; args 1,1,1,0,1,1,1,0,3,4,5,-1,1.0f,1,-1)`, texture
  loading, three message subscriptions (`0xA6B79602`, `0x06CDB65B`, then a third via slot 0x14), and
  `RebuildModel`. Returns 1.
- **`LoadTextures`** `0x005F5EF0` *(rewritten 2026-10-22, re-signposts)* — empties the text-texture vector (object
  `+0x574`); when the text (`+0x98`) is not empty, the text texture service (`0x00B43CE4`, its slot 4) renders it with
  font `0x2C1FD612` into that vector *[inferred: one texture per piece of text]* (`0x005F5EF7`–`0x005F5F74`). The ten
  piece textures are `Init`'s, not this function's: `Init` asks the resource service (`0x00B43CD4`) slot 6 for
  (`0x7AB50E44`, `0x1ABE787D`, RID) for each entry of `0x00AA5214…0x00AA523C` (`0x005F745A`–`0x005F7465`).
- **`Shutdown`** `0x005F5E20` unsubscribes, releases the ten bindings, empties the text-texture vector (object
  `+0x574`) and, when byte `+0xA0` says a model instance exists, unregisters from the animation tick manager (slot
  0x18), then frees the date pair and calls the base `Shutdown` (tail-jump).
- **`SetText(text)`** `0x005F7600` — compares against the current text (`cRZString` slot 0x30 `CompareTo(arg, 1)`);
  if equal **and** `+0x9C == +0xA0` the call is skipped; otherwise copies (slot 0x34) and calls `LoadTextures` and
  `RebuildModel`. `GetText` `0x005F5920` tail-jumps the string's slot 0x34 copy into the caller's string.
- **`SetDynamic(bool)`** `0x005F4BA0` — reads the current flag through `FUN_008018E0(&+0x9C)` (returns `+0x80` of
  the object at `+0x9C`, i.e. the model instance's byte); no change → return. Sets the occupant view's flag 2 test
  (`IsFlagSet(2)`, slot 32) result aside, drives the model instance (`+0x70` view slots 0x34/0x38) with the new
  state, copies the four-byte colour row `0x00AA5204 + 4·(arg != 0)` into the four corner rows `+0x4D4…+0x4D7`,
  `+0x4E4…+0x4E7`, `+0x4F4…+0x4F7`, `+0x504…+0x507`: row 0 (static) is (144, 172, 190, 255), row 1 (dynamic)
  (255, 255, 255, 204), in memory order *[inferred: R, G, B, A]* (`0x00AA5204` = `90 AC BE FF`, `FF FF FF CC`; OP3
  answered 2026-10-22, re-signposts), and — only when
  the object is in edit mode (`+0xAC`) and the argument is false — calls the class-view slot 7 (`SetEditMode`, via
  `[EAX + 0x1C]` with 0).
- **`SetEditMode(bool)`** `0x005F7320` — no change → return. Stores `+0xAC`. Turning on: class-view slot 6 with 1
  (QI-4 helper `[EAX + 0x18]`), registers with the animation tick manager (slot 0x14, tag `0x80`, target = the
  message target at `+0x8`), clears `+0xB0`, sets `+0xAD` = 1, `RebuildModel`. Turning off: unregister (slot 0x18),
  slot 6 with 0, `RebuildModel`.
- **`DoMessage`** `0x005F7510` — `0x06CDB65B` (visibility test): reads the message's void (slot 0x3C), QIs it
  against `0x4B44FBE2` through slot 0xC; when it answers and the city config (`0x00B43DB8`) slot 0x28 forces the
  hidden state, clears flag 2 (`ClearFlag(2)` via view slot 0x7C) unless already clear; then the prop's
  `DoMessage`. `0x86AD10EE` (time tick): takes data3 (slot 0x38) as milliseconds (÷1000 via the `0x10624DD3`/SHR 6
  pair) and calls `FUN_005F4960(this, ms)`: `+0xB0 += ms`; when `+0xB0 >= 500` (`0x1F4`) toggle `+0xAD` (SETZ of
  the inverted byte) and reset `+0xB0` — the blink. Then the prop's `DoMessage`. `0xA6B79602`: `RebuildModel`.
  Default: the prop's `DoMessage`.
- **`SetAllFlags`** `0x005F4A90` — the same flag-2 protection as the decal (§10.4): when the city config slot 0x28
  forces hidden, bit 2 is stripped from the new value before the base `SetAllFlags`. With bit 2 turning on it
  registers the model with the renderer (`0x00B43DD0` slot 0x6C test, slot 0x70 add; box pointer from the per-
  instance table at `+0x64`) — OP7.
- **`Draw`** `0x005F5200` — visual only (renderer thunk `__ZThn108_N20cSC4SignpostOccupant4DrawEP14SC4DrawContextb`
  `0x005F5CD0` adjusts `this` by −108 into the +0x6C sub-object). `RebuildModel` `0x005F5FB0` (4,964 bytes) builds
  the sign geometry from the text and matrices; it is not specified further here.

### 10.4 `cSC4TextDecalOccupant` (`0x8B79C707`) *[compat]*

The city-detail terrain text label embeds a prop at complete-object +4. Its corrected
state, record and owned behavior contract is [text-decal-occupant.md](text-decal-occupant.md),
with independent **UNEXECUTED** [vectors](text-decal-vectors.md).

**OP1 correction (2026-10-23):** occupant methods use O+12, serializable methods O+16,
constructor and internal rebuild O. Record +5C is O+6C text (GZStr); repeated +84/+88 is
O+94/+98 position (Float32), not IDs or repeated height. Complete-object +84/+88 are IDs;
Shutdown's +A8/+AC are O+B4/+B8 managers, while SetOrientation's +AC is O+AC orientation.
The former mixed-frame tables and geometry summary are withdrawn, not alternative behavior.
TextDecal-only OP2 is resolved by compiled stream-width/slot facts in the dedicated spec §7;
the signpost part of OP2 remains separate.

#### Contributor interface supplement (PR10, harvested 2026-10-23)

The dedicated TextDecal spec above remains authoritative for numeric comparisons,
geometry, resource ownership and persistence. The contributor independently agrees
on complete-object field roles and the version-3 record; its duplicate summaries
are superseded by that newer contract. The signpost portion of OP2 is now resolved
by §10.3: its serializable receiver is O+0x10 and stream slot 15 receives O+0x98 text.

| Complete-object view | Vtable | Interface facts |
|---|---|---|
| +0x00 | `0x00AA5680` | 11 slots; QI `0x005F7650` answers `0x2B3B7D86`; AsOccupant returns O+0x0C |
| +0x04 | `0x00AA5620` | embedded prop |
| +0x08 | `0x00AA5610` | message target; DoMessage `0x005F8030` |
| +0x0C | `0x00AA5588` | 34-slot occupant; Init `0x005F7F40`, Shutdown `0x005F76C0`, SetPosition `0x005F7AA0`, SetAllFlags `0x005F81F0` |
| +0x10 | `0x00AA556C` | serializable Read/Write |
| +0x14 | `0x00AA5508` | 26-slot holder; slots 23/24 Write/Read |
| +0x28 | `0x00AA54F0` | exemplar property holder |

Creator `0x005F84D0` uses pool `0x00B0A9B8`; registration is at `0x00460851`.
These are interface/provenance facts, not an alternative TextDecal behavior contract.

---

## 11. Flora occupant (`cSC4FloraOccupant`, `0xA9C05C85`) *[compat]*

Behaviour in `flora.md` §3.4 and §6. Record:

| # | Field | Type | Meaning, observed (744,617 in 568 saves) |
|---|---|---|---|
| 1 | version | u16 | 3 (`Read`: anything else → false) |
| 2 | occupant base | §2.3 | type `0x74758926`, flags 13 |
| 3 | flora type | u32 | the static flora record key (`flora.md` §3.2) |
| 4 | position | vec3 | |
| 5 | birth date | day | 2,451,545 … 2,462,350 |
| 6 | last seeding date | day | 2,451,545 … 2,466,676 |
| 7 | state | s8 | 0 (735,026), 1 (4,114), 2 (5,377), 3 (100) |
| 8 | orientation | u8 | 0…3 |
| 9 | lot tag | u32 | tag of the configuration object that planted it on a lot; 0 for wild flora (431,076) |

If the base `Read` fails, `Read` returns `GetError() = 0`. The static record pointer is not saved; it is looked up
from #3 *[inferred: on Init]*.

---

## 12. Bare occupant (`cSC4Occupant`, `0xA9BC9AB6`)

84 records (12 saves) consist of only the base block (§2.3): type `0x74758925`, holder parent `(0, 0, 0)`, no
properties. They are referenced only by the occupant manager; they are the dummy occupants of automata
*[inferred]* (`save-records.md`).

The class itself (behaviour, layout and the 34-slot `cISC4Occupant` interface) is specified in
occupant-manager.md §19. The construction signpost and text-decal classes above embed this base; the prop classes
do too (inside the prop). A replacement implements the base once and derives every occupant from it.

---

## 13. Interfaces *[compat]*

### 13.1 `cIGZSerializable` of each class

Every class above has a 6-slot `cIGZSerializable` view at the vtable listed in §1: slots 0–2 QueryInterface (pop 8),
AddRef, Release (pop 0); slot 3 `Write(cIGZOStream&)`; slot 4 `Read(cIGZIStream&)`; slot 5 `GetGZCLSID()` (EAX =
class ID). All 24 Write / Read functions were checked with `abi_extract --check`: `__thiscall`, callee pops 4.
Return channels from the epilogues:

| Class | Write returns | Read returns |
|---|---|---|
| lot `0x006C0080` / `0x006C35F0` | **AL** (`sbb al,al`; failure path `xor al,al`) | **AL** (version > 8: `xor al,al`) |
| business unit `0x006C0370` / `0x006C09E0` | **AL** | **AL** |
| occupant base, building, base texture, wall, foundation, flora, prop, effect prop, construction | EAX = 0/1 | EAX = 0/1 on the normal path; **AL** = 0 on the version-reject path |
| signpost `0x005F5080` / `0x005F50F0`, text decal `0x005F7DB0` / `0x005F7E50` | EAX = 0/1 (`NEG AL; SBB EAX,EAX; INC EAX`) | same epilogue form |
| `cSC4TripData` `0x0070E0B0` / `0x0070E160` (non-virtual, `this` = the trip) | EAX = 0/1 | EAX = 0/1 |

A replacement returns a clean 0/1 in EAX; callers in the original test AL only (`bool`).

### 13.2 Slots of other interfaces used by these records

| Interface | Slot (offset) | Use | Pop | Return |
|---|---|---|---|---|
| `cISC4Lot` (vtable `0x00AB19B0`) | 3 (+0xC) `AsPropertyHolder` `0x006C2040` | §3.1 (LR9) | 0 | EAX = this +8, or null; borrowed |
| | 4 (+0x10) `AsTrafficSource` `0x006C2050` | §3.1 (LR9) | 0 | EAX = this +0x1C, or null; borrowed |
| `cISC4Lot` (vtable `0x00AB19B0`) | 5 (+0x14) `Init` `0x006BF490` | §3.5 | 0 | AL = 1 |
| | 7 (+0x1C) `Shutdown` `0x006C3060` | §3.5 | 0 | AL = 1 |
| | 8 (+0x20) `PostCityInit` `0x006BF200` | §3.5 | 0 | none defined (tail of SetFlags) |
| | 12 (+0x30) `GetFlags` `0x006C2030` | §3.2 | 0 | EAX = flag byte, zero-extended |
| | 13 (+0x34) `SetFlags(u32)` `0x006BF220` | §3.5 | 4 | none defined (SDK: bool) |
| | 15 (+0x3C) `GetCenter(out)` `0x006BF560` | Init | 4 | out-parameter (abi_extract also reports x87) |
| | 17 (+0x44) `SetAltitude(float)` `0x006BF5C0` | Init | 4 | none defined |
| | 34 (+0x88) `SetState(state, wealth, reason)` `0x006C3100` | §3.2.2 (LR11) | 12 | none defined; caller AL pass-through is not success |
| | 44 (+0xB0) `SetHistorical(bool)` `0x006C2230` | Read v < 6 | 4 | none |
| | 54 (+0xD8) `GetPopulations` `0x006C21E0` | Write | 0 | EAX = pointer to the vector |
| | 55 (+0xDC) `SetPopulation(devType, u16)` `0x006C1940` | §3.2.1 (LR10) | 8 | AL = first capacity-map match; upper EAX undefined |
| | 67 (+0x10C) `SetHasPower(bool)` `0x006C21C0` | Read v < 6 | 4 | none |
| | 69 (+0x114) `SetVerticesAreLocked(bool)` `0x006C2130` | Read v < 6 | 4 | none |
| `cISC4LotConfiguration` | 3 (+0xC) GetID | lot Write | 0 | EAX |
| `cISC4LotConfigurationManager` (`0x00B43D18`) | 8 (+0x20) GetLotConfiguration(id) | lot Init | 4 | EAX |
| `cISC4BusinessUnit` | 4 (+0x10) Remove(lot) | lot Shutdown | 4 | — |
| `cISC4Occupant` | 5 (+0x14) IsInitialized, 15 (+0x3C) PostOccupantMessage, 16 (+0x40) GetHighlight, 17 (+0x44) SetHighlight | §2.3, §2.4 | 0, 8, 0, 8 | AL, —, EAX, — |
| `cIGZOStream` / `cIGZIStream` | +0x10 s8 … +0x3C GZStr, +0x48 GetError | all | foundations §2.2 | |

The `cISC4Lot` pops were checked with `abi_extract --check`; the pops of the other rows follow the pushes at the
call sites *[inferred]*.

### 13.3 SDK notes

- `cSC4TripData.h` (gzcom-dll): the four bytes the header calls `unknown4`, `transitType`, `flags1`, `unknown5` are,
  on Windows, type, strategy, flags and status in that order, and the header's `unknown6…unknown9` bytes at the end
  are one u32 **travel types used** (checked in `Write` `0x0070E0B0` and `Read` `0x0070E160`; pop 4, EAX).
- `cISC4DBSegmentIStream::ReadGZSerializable` takes two arguments (foundations §2.7); every `Read` here calls the
  helper `SC4ReadGZSerializable` `0x005D9730` (stream, IID, destination, loadNow = 0).

---

## 14. References and resolution on load *[compat]*

| From | Field | To class | Read IID | Observed count | Resolved |
|---|---|---|---|---|---|
| zone developer | cell map | lot | `0xE818FCED` | 21,179 cells → 12,985 lots | at read (dedup, `zones.md` §10.2) |
| lot | business unit | business unit | `0x6A233069` | 7,746 | stored; used by Shutdown and the lot developer |
| lot | travel designate | lot | `0xE818FCED` | 6,110 | stored |
| business unit | members | lot | `0xE818FCED` | 7,746 | stored |
| retaining wall | lot | lot | `0xE818FCED` | 1,735 | stored |
| construction | lot, building | lot, building | `0xE818FCED`, `0x87DDA3A9` | 31 + 31 | stored |
| construction | ground / roof props | prop | `0xE9793A65` | 12 | stored |
| construction | effects, current effect | effect prop | `0x2977AA49` | 99 | stored |
| construction | crew | pedestrian | `0xA9B40F05` | 54 | stored |
| lot developer | night deque | building | `0x87DDA3A9` | `lot-developer.md` §13 | stored |
| occupant manager | all occupants | all occupant classes | `save-records.md` | — | inserts into the spatial index |
| crime simulator, budget, traffic, … | lots | lot | `save-records.md` | — | |

No reference in any save is dangling or names a class missing from the index.

Which references keep which objects alive (from all saves): lots are reached from the zone developer, other lots,
business units, walls, constructions and the crime simulator; buildings from the occupant manager, the lot
developer, constructions, automata and some simulators; base textures, walls and foundations **only** from the
occupant manager; flora from the occupant manager and automata; props from the occupant manager, the prop developer,
automata and constructions; business units only from lots; bare occupants only from the occupant manager. A writer
must therefore emit an object exactly when one of these owners references it (foundations §3.5).

*[quirk]* One lot (`City - Rush Hour Tutorial.sc4`, a 1×2 empty R$$ lot, state 1) is not on the zone developer's map
and is saved only because the crime simulator still references it. After a load it is read but never `Init`ed by
the zone developer (`zones.md` §1.2).

---

## 15. Replaceability

### 15.1 New-object paths bypass the class registry (LR8)

Registering lot, business-unit and retaining-wall class factories covers serialized-object creation, not
creation during play. With native developers retained, the following are fixed direct edges, not directory
lookups or replaceable owner vtable slots. Addresses in this table were checked against native instruction bytes.

| Native source | Direct target | ABI/ownership at the edge |
|---|---|---|
| Zone developer CreateLot body `0x0072E520` (lot-manager vtable `0x00AB37C0`, slot 23): `0x0072E530`, `0x0072E557` | pool allocation `0x006BF430`, then parameterized lot constructor `0x006C1E50` | allocation called with one caller-cleaned 4-byte size (0x8C); constructor ECX = allocated storage, five stack words in logical order x, z, width, depth, facing, callee pops 20, EAX = same lot. At `0x0072E564` store output; `0x0072E56B` AddRef; `0x0072E573` Init. No GZCOM lookup. |
| PickUnitToJoin `0x006CB44C`–`0x006CB474` | allocate 0x20 via `0x005E55E0`, constructor `0x006C0800`, then unit slot 3 twice | allocation caller cleans 4; constructor `__thiscall`, no stack args, EAX = storage. Adds candidate then current lot; no creation AddRef or Release. Member lots acquire the unit through SetUnit (§4). |
| ClearLot `0x006C8A3D`; AddLotFoundation `0x006C8C23` | static wall helper `0x005E3070` | `__cdecl(lot*, add)`, two 4-byte stack slots, caller cleans 8; low byte is boolean. Clear uses false; foundation uses true. Return AL ignored. |
| ClearLot `0x006C8A62`; AddLotFoundation `0x006C8C34` | `0x005E3960` UpdateSurroundingRetainingWalls | `__cdecl(lot*)`, caller cleans 4; no defined return. |
| lot-developer slot 62 wrapper `0x006C4190`, direct call `0x006C4195` | `0x005E37F0` UpdateRetainingWalls | target is `__cdecl(lot*)`, not an owner virtual method. It can call `0x005E3070` itself (`0x005E392E`, `0x005E3947`). |
| AddRetainingWallOccupant `0x005E30E7`, `0x005E30F5` | wall pool allocation `0x005E2A50` (0x6C), constructor `0x005E2F40` | direct storage construction, not a class-factory call; helper AddRefs the new wall and later releases its creation reference (§7.1). |

The outer CreateLot call is `__thiscall(x, z, width, depth, facing, outLot**)`, callee pops 24,
AL = whether `*outLot` is non-null (`0x0072E67E`–`0x0072E68F`). Its constructor uses 4-byte argument
slots but stores cell bounds and facing as bytes (`0x006C1F69`–`0x006C1F7F`); this is not an ABI of
five packed bytes. The default constructor `0x006C2340` instead has no stack arguments; the
pool-allocation/default-construction helper `0x006C2FC0` tail-jumps to it at `0x006C2FEC`. Replacing
that helper does not affect the separate parameterized-constructor path above.

**Ownership across the boundary:**

- A fresh lot's count starts at 0 (property holder at +8 clears primary +0xC, `0x005FF10A`).
  CreateLot acquires one reference for the returned pointer (`0x0072E56B`) and a second for the master
  list (`0x0072E5D8`). The cell loop stores the same pointer in each cell (`0x0072E617`) without per-cell
  AddRefs; safe-iterator insertion (`0x0072E62E` → `0x0072C810`) and ordinary insertion
  (`0x0072E64D`–`0x0072E65C`) use that second reference without another AddRef. Caller and list must each
  release their own reference. Native Release at count ≤1 calls Shutdown then the property-view deleting
  destructor (`0x006BF471`–`0x006BF480`, `0x006C22A0` → `0x006C3000`); that destructor returns storage
  to the native lot pool (`0x006C3015`–`0x006C3026`). A native in-place constructor therefore requires
  its native pool storage, not a foreign replacement object or an arbitrary same-sized buffer.
- The business unit is not held by a creation reference in PickUnitToJoin. Its member list retains each lot
  and each member's back-pointer retains the unit (§4). Preserve both directions and break them through
  Remove/Shutdown; merely releasing an extra constructor reference is not membership removal.
- The static wall helpers borrow their input lot. The new wall's slot 6 SetLot (`0x005E2CB0`, pop 4)
  AddRefs a changed non-null new lot **before** storing it and releasing the old lot
  (`0x005E2CC8`–`0x005E2CD6`). The helper holds one wall reference from `0x005E3104`, hands its
  occupant view to occupant-manager slot 20 at `0x005E3246`, and releases its local wall reference at
  `0x005E3264` (also on the property-load failure path). The wall constructor `0x005E2F40` is
  `__thiscall`, no stack arguments, EAX = input storage (`0x005E2FB8`–`0x005E2FBB`). Native wall
  allocation uses the distinct pool at `0x00B0A8B0`; its free path is `0x005E32F0`. Update and surrounding
  helpers release acquired occupant/interface/list-element references (§7.2); they do not transfer ownership
  of their input lot to the caller.

**Supported replacement boundary:** use a replacement zone developer/lot manager together with replacement
lots, and a replacement lot developer together with replacement business units and retaining-wall creation/
update helpers. Their ordinary interface calls can then create the replacement objects and implement the
specified wall operations without entering the direct native edges above. This is a dependency requirement,
not evidence of a native plugin installation API. The city holds both zone-developer and lot-manager views
of one owner (foundations §3.6); replacing only one view is not this boundary.

No supported installation/restoration contract for independently intercepting these direct edges has been
established by these bytes or the class-object-store contract. Consequently **standalone complete-owner
activation with both native developers retained is unsupported by this spec**. Method addresses and factory
registration are insufficient. This does not claim machine-code interception is impossible; such an approach
would need a separately proved patch, calling convention, storage, lifetime, and restoration contract before
being used. No hook contract is implied here. Existing objects and saved references are not migrated by changing
a factory registration; restoration while such objects survive has no specified native mechanism.

### 15.2 Other coupling

- **Class registry**: the loader creates every object by class ID through GZCOM (foundations §2.7). A replacement
  must register all classes of §1 (including the prop subclasses and `cSC4Occupant`) or loading a city fails for
  those classes (foundations §3.5). This is the load path; the direct new-object paths of §15.1 bypass it.
- **Signpost and text-decal embed the prop**: their `cISC4PropOccupant` view at object +4 answers with `this+4`
  (`ADD ECX,4`), so a replacement must either reimplement the prop inline or forward — the two cannot be
  substituted independently.
- **Non-virtual entry points used across classes**: `cSC4TripData::Write/Read` (called from the lot and from
  neighbour connections, `neighbors.md`); `cSC4LotRetainingWallOccupant::AddRetainingWallOccupant`, `UpdateRetainingWalls` and
  `UpdateSurroundingRetainingWalls` (called by the lot developer); `GetPropTypeIndex` and the process-wide prop type
  table (used by every prop). Replacing the lot developer or traffic simulator alone therefore keeps these
  original helpers, or replaces them together.
- **Occupant-manager coupling**: `UpdateDecal` (§10.4) calls `MoveOccupant` (occupant manager slot 24) after
  changing the box; a replacement must call it too or the cell lists go stale.
- **Message subscriptions**: the construction, signpost and decal occupants subscribe through the message server
  slot 0x14 with their `cIGZMessageTarget2` view; a replacement must answer the same five message IDs with the
  same semantics, including the `0x06CDB65B` flag-2 dance shared with the prop (§10.3/§10.4).
- **Shared static RNGs**: the lot file's `0x00B49120` (lot Init, traffic) and the base-texture `0x00B23078`.
- **Spatial lookup**: lots find their occupants through the occupant manager (§3.1); a replacement lot must keep
  the occupants in the original occupant manager or replace both.
- **Raw memory access**: none found from outside these classes for the fields above; outside code uses `cISC4Lot`,
  `cISC4TrafficSource` and `cISC4Occupant` slots *[inferred: not exhaustively searched]*.
- No grid is owned by these classes (simgrid.md §6.4 does not apply).

---

## 16. Quirks and open questions

Quirks (reproduced): vacate reason kept after leaving state 4 (§3.2); uninitialised trip fields saved (§3.3);
lot trip list appended on read (§3.4); lot `Read` sets "initialised" before checking the version, so a rejected lot
skips Init's altitude step (§3.4); retaining-wall mask grows on load (§7); effect props lose their name while
live (§10.2); prop and construction readers accept unknown newer versions (§9, §10.1); u8 map counts (§3.2); the lot
configuration ID is dropped when the configuration is missing (§3.5); the crime-only lot (§14); the text-decal
record writes position twice and stores only the low orientation byte (§10.4, text-decal-occupant.md §7);
the signpost `SetText` skip requires both an equal string *and* an unchanged
capacity pair (§10.3); construction `+0xCC` (record #8) is written as `(+0xF8 != 0)`, i.e. from the live crane
pointer rather than a stored bool (§9.1).

Open questions:

- **Q1** The exact rule for the occupant manager's cell rectangle (§2.3 field 6) belongs to the occupant manager;
  the `floor(x / 64) + 64` approximation misses edge and flora-model cases.
- **Q2** Where do vacate-reason values > 6 come from? Every observed caller passes 0…6 (`UpdateLot` 0…4, tract
  developer 5 and 6); all vacant lots in saves hold 1…6, so the large values were written in an earlier state or
  build. A writer copies the byte. *Answered* in zones.md §8.5.1 *[inferred]* (cited by re-answers11, ZQ16): the
  lot constructor `0x006C1E50` writes the byte at `+0x80` (`0x006C1F19`) but never the vacate reason at `+0x83`, so a
  new lot keeps the byte left in its pool block by the lot that used it before, and a lot that was never vacant
  saves that stale value.
- **Q3** Names of the 19 scalar fields of the construction occupant (§9 #14). Partially decoded in §9.1/§9.2 from
  the state machine (`+0x140…+0x184`, `+0x118`); the remaining pure names and the meaning of `+0x118` are still
  open (filed as OP7 in questions/occupants-props.md).
- **Q4** Meaning of the five `0xFF`-default bytes of a base-texture record (§6) and of the effect prop's two bytes
  and u32 (§10.2).
- **Q5** Who sets lot flag bit 7 (§3.2)?
- **Q6** Layouts of the burnt-building, terrain-decal and automata-controller prop subclasses (absent from saves).
- **Q7** Answered statically 2026-10-23: the text-decal serializable receiver is O+16. Its
  extension writes position twice, height then size, RGB, alpha and orientation byte; no decal
  IDs or duplicate lot tag. See text-decal-occupant.md §§1–2,7; archived OP1 and OP2-TD.
- **Q8** TextDecal portion answered in text-decal-occupant.md §7: stream +3C is GZStr and
  receiver +5C is O+6C text. Signpost's independent receiver/field reconciliation remains OP2
  in questions/occupants-props.md; this correction makes no new signpost-owner claim.

---

## 17. Validation and conformance tests

### 17.1 Validation over all saves

587 saves, 417 distinct. For every distinct save, every entry of each class record was split (foundations §3.3),
its CRC checked, parsed with the layouts above, and re-encoded; all references were resolved against the save's
class records.

| Class | Saves | Entries (all saves) | Distinct entries checked | Parsed exactly, re-encoded identically | CRC failures | References / dangling |
|---|---|---|---|---|---|---|
| `cSC4Lot` | 14 | 23,912 | 12,986 | 12,986 | 0 | 13,856 / 0 |
| `cSC4BuildingOccupant` | 14 | 23,129 | 12,295 | 12,295 | 0 | — |
| `cSC4LotBaseTextureOccupant` | 14 | 24,037 | 13,079 | 13,079 | 0 | — |
| `cSC4LotRetainingWallOccupant` | 14 | 2,131 | 1,735 | 1,735 | 0 | 1,735 / 0 |
| `cSC4FoundationOccupant` | 14 | 17,385 | 8,981 | 8,981 | 0 | — |
| `cSC4FloraOccupant` | 568 | 1,059,847 | 744,617 | 744,617 | 0 | — |
| `cSC4PropOccupant` | 15 | 198,136 | 134,420 | 134,420 | 0 | — |
| `cSC4PropOccupantVisualEffect` | 14 | 1,558 | 948 | 948 | 0 | — |
| `cSC4ConstructionOccupant` | 11 | 69 | 31 | 31 | 0 | 227 / 0 |
| `cSC4BusinessUnit` | 14 | 552 | 328 | 328 | 0 | 7,746 / 0 |
| `cSC4ZoneDeveloper` | 587 | 587 | 417 | 417 | 0 | 21,179 / 0 |
| `cSC4Occupant` (bare) | 12 | 172 | 84 | 84 | 0 | — |

Versions seen: exactly the written versions of §1 (and trip version 2). Saves with lots: Konradshohe, Spandau,
Tegel, Fulham, Kensington, Big City Tutorial, Making Money Tutorial, Rush Hour Tutorial; `New City (32)` has props
only; flora is in 568 saves. Map sizes of the zone developer: 64² (157), 128² (250), 256² (10).

### 17.2 Byte-exact round-trip tests

Each test is a real entry from `City - Making Money Tutorial.sc4` (object ID, entry CRC = CRC-32/MPEG-2 over
object ID + payload, foundations §3.4). Required: (a) the reader consumes exactly the payload and yields the
values shown; (b) writing those values back gives the same bytes; (c) the entry header is
`u32 12 + len, u32 crc, u32 objectID`.

**T1 — lot (empty zoned lot)**, ID `0x0DFA3C64`, CRC `0x1C85D610`, 118 bytes:
```
08 00  00 00 00 00  10  49 64 49 64  49 64  4C FE 99 43  80 5F 03 40  80 5F 03 40
01 01 01 01 00 08 00  74 68 25 00  00 00 00 00  CD  00 00 00 00  00 00 00 00  00  00
00 00 00 00 00 00 00 00 00 00 00 00
02 00 01 00 00 00  00 41 0B 8A  00 41 0B 8A  00 00 00 00  02 80 00 00  0C 00 00 00  (24 × 00)
00 00 00 00  4A
```
→ version 8; configuration 0; flags 0x10 (has power); rect (73, 100)–(73, 100); location (73, 100); altitude
307.9867; both slopes 2.0527; size 1×1; facing 1; state 1; condition 0; zone 8; wealth 0; day 2,451,572; building
type 0; vacate 205; no unit; no designate; 0 population maps; empty travel map; travel jobs 0, 0, 0; one property
`0x8A0B4100` = uint16[12] all 0; 0 trips; strategy 74.

**T2 — business unit**, ID `0x10BDD698`, CRC `0x73D2BA7A`, 22 bytes:
`01 00 02 00 00 00 0C 40 DA 13 4A 5D BD C9 44 CA B1 11 4A 5D BD C9` → version 1; lots `0x13DA400C`, `0x11B1CA44`,
both class `0xC9BD5D4A`.

**T3 — flora**, ID `0x00947810`, CRC `0x1600BAD3`, 66 bytes:
```
03 00  04 00 00 00 00 0D 26 89 75 74 57 5F 57 5F  02 00 02 00 00 00 00 00 37 04 3E E8 4A 28 34 65 12 82 0C 2A
12 82 0C 2A  49 5F BE 44 12 C4 98 43 3A 56 FC 44  3C 74 25 00  5A 68 25 00  00  02  00 00 00 00
```
→ base: removal 0, highlight 0, flags 13, type `0x74758926`, cell rect (87, 95, 87, 95), no properties, parent
(`0xE83E0437`, `0x6534284A`, `0x2A0C8212`); flora type `0x2A0C8212`; position (1522.978, 305.532, 2018.695); birth
2,454,588; last seeding 2,451,546; state 0; orientation 2; lot tag 0.

**T4 — prop**, ID `0x009476C4`, CRC `0x892A416D`, 76 bytes:
```
06 00  04 00 00 00 00 05 1E 82 23 A8 57 5B 58 5B  02 00 02 00 00 00 00 00 36 C5 77 C9 4A 28 34 65 00 00 84 29
00 00 84 29  34 23 BF 44 C5 74 99 43 54 EB D8 44 00 20 C1 44 C5 74 9A 43 54 0B D9 44
03 00 00 00 00 64 02 24 FE A5 0A 00
```
→ prop type `0x29840000`; bounds (1529.1, 306.912, 1735.354)–(1545.0, 308.912, 1736.354); orientation 3; state 0;
time (0, 0); no date range; chance 100; usage 2; lot tag `0x0AA5FE24`; timer 0.

**T5 — building**, ID `0x00C3CF20`, CRC `0x9175D64A`, 70 bytes: version 2; base type `0x278128A0`, rect
(84, 79, 84, 79), parent (`0x67BDDF0C`, `0x6534284A`, `0x00730000`); building type `0x00730000`; flags 0; bounds
(1314.113, 306.227, 980.606)–(1320.113, 314.227, 988.606); orientation 2; completion 1.0 (`00 00 80 3F`).

**T6 — base texture**, ID `0x00BF7198`, CRC `0xB4A11718`, 79 bytes: version 2; base type `0x497F6D9D`, rect
(82, 91, 82, 91), parent (0, 0, 0); bounds (1184.1, 306.121, 1728.1)–(1199.9, 306.404, 1743.9); usage 2; 1 record:
`00 30 00 00 4A 6C 00 00 FF B2 33 AA 03 00` (texture `0x3000`, cell (74, 108), orientation 0, byte 0, unnamed
`FF B2 33 AA 03`, frame 0).

**T7 — retaining wall, mask growth**, ID `0x0D901200`, CRC `0x62B30E0F`, 92 bytes: wall type `0xC96D2135`; mask
`09 00 00 00`; hashes `0x00F439E9`, `0x00CFA447`, `0x010DEF29`, `0xE0C042C7` (all non-zero); position (1664,
299.696, 960); size (128, 0.2437, 96); lot `0x11DEAF54`. After `Read` the mask is **15**; the original's next save
writes `0F 00 00 00` at payload offset 40. A conforming writer of an unmodified loaded wall writes 15.

**T8 — foundation**, ID `0x0DA31268`, CRC `0xB39E1107`, 70 bytes: type `0x890B7313`; maximum height 32.0; position
(1656, 304.792, 1560); size (16, 4.1668, 48); usage 2; orientation 2.

### 17.3 Behaviour tests

- **T9** Lot `Read` of a payload whose version is 9: returns false after consuming 2 bytes; the lot is marked
  initialised (a later `Init` keeps altitude 0).
- **T10** Lot `Read` into a lot that already has one trip, payload with 2 trips: the list has 3 trips.
- **T11** Prop `Read` of a version-5 payload (T4 without its last byte, first byte 05): consumes 75 bytes, timer bits
  = `0x0F`.
- **T12** Effect prop `Write` while its effect is live and named `large_dust`: the name is written as
  `00 00 00 00` and the trailing u32 as 0.
- **T13** Lot `Init` after T1: no RNG draw (strategy 74 ≠ `0xFF`); configuration stays null (ID 0); altitude stays
  307.9867. With strategy `0xFF` and the RNG `0x00B49120` in a known state: exactly one `RandomUint32Uniform(100)`.
- **T14** Lot `PostCityInit` with saved flags `0x5B`: `SetFlags(0x5B)` from 0 locks every rect cell in the terrain
  view at the lot altitude (bit 0 changed); with flags `0x50` nothing is locked.
- **T15** Trip `Read` of version 1: no travel-types field is read and it becomes 0.
- **T16** `bytes` read with count `0x000F4240`: no further bytes consumed, the route keeps its previous contents.
- **T17** New lot (not read) with rect (10, 20, 12, 21), terrain altitude 300.0 at (184.0, 336.0) and
  `GetAreaSlope(rect, false)` = 2.5: after `Init`, altitude 300.0, fields 7 and 8 both 2.5, travel jobs 0, 0, 0.
  With terrain altitude 0.0005 instead: altitude stays 0 and no slope query is made (§3.5 `SetAltitude`).
- **T18** `SetFlags(0x01)` from 0 on rect (10, 20, 11, 21), altitude 310.5: the terrain view sees exactly
  `LevelCell(10,20,310.5)`, `(10,21)`, `(11,20)`, `(11,21)`; then `SetFlags(0x00)` gives `RemoveCellLevelling` on the
  same four cells in the same order; `SetFlags(0x03)` from `0x01` makes no terrain-view call.
- **T19** Unit with members [A, B]: `Remove(C)` for a lot C not in the list leaves [A, B] and still calls
  `C.SetUnit(0)` and `C.SetTravelDesignate(0)`; `Add(A)` leaves [A, B] and calls `A.SetUnit(unit)`; `Add(C)` gives
  [A, B, C].
- **T20** Lot read with configuration ID `0x12345678` and never `Init`ed: `Write` emits field 2 as `00 00 00 00`.
  After `Init` with the configuration manager answering that ID: `Write` emits `78 56 34 12`; answering null: 0.
- **T21** `AddRetainingWallOccupant(lot, true)` for a lot with flags bit 0 clear: no wall, no RNG draw, AL = 0.
  With bit 0 set, rect (10, 20, 10, 20), altitude 100.0, edge 0 neighbour row = one cell holding no lot and its
  edge vertices (10, 20) and (11, 20) at 102.4 and 103.6: the cell has no lot, so `bits = 0` and `h[0] = (102·10·20 + 1) + (104·11·20 + 1) = 43,282`; `top = 103.6`; mask bit 0 set; size y = 3.6
  (float); AL = 0.
- **T22** `UpdateRetainingWalls(lot)`, lot flags bit 0 set, wall at the location cell with hashes (5, 0, 7, 9) and mask
  `0b1101`; the edge pass gives (5, 4, 0, 8) and every model build succeeds. Calls in order: edge 1 add(1), edge 2
  remove, edge 3 remove then add. Result: hashes (5, 4, 0, 8), mask `0b1111` (bit 2 stays), position, size and type
  unchanged, the wall is not removed.
- **T23** Same wall, edge pass equal to the stored hashes: no `AddRetainingWallModel` call, `SetEdgeHashes` not
  called, the record is byte-identical. Lot flags bit 0 clear: no occupant query at all. No wall at the location
  cell: exactly one `AddRetainingWallOccupant(lot, true)`.
- **T24** Edge 0 changes from 0 to 6 and the model's vertex setup fails: hash 0 := 6, mask bit 0 cleared; after a
  save and load the mask has bit 0 again (§7 `Read`).
- **T25** `UpdateSurroundingRetainingWalls(B)` with lot C touching only ring row z1 − 1 and lot A touching only
  column x1 − 1 at rows z1..z2 (so C comes first in the zones.md ring order): `UpdateRetainingWalls(C)`, then `(A)`; never `(B)`; each of A and C ends with its reference count
  unchanged.

### 17.4 Native boundary discriminators (LR8/LR9)

These are conformance vectors derived from native bytes, not reported game executions.

- **T26** For a live lot at symbolic address P = `0x10000000`, slot 3 returns `0x10000008`, slot 4 returns
  `0x1000001C`; neither changes the reference count. Null ECX gives null for both. A slot-4 property-holder
  interpretation fails this vector. Through the traffic source iterator, the same slot-4 result is written to
  the source output, with one additional reference acquired through the primary lot at `0x0071084E`.
- **T27** Starting with a destination sentinel and a live lot P, QI with lot IID `0xE818FCED` returns P,
  Serializable IID `0xE4FDA3D4` returns P+4, and IID 1 returns P; each success increments the owner's count once.
  An unsupported IID leaves the sentinel unchanged and returns AL = 0. Property-view QI on P+8 behaves the
  same way after adjusting to P; it does not expose more IIDs.
- **T28** Observe class-factory invocations in two separate scenarios: resolving an unseen serialized lot
  reference uses the class store's GZCOM creation (foundations §2.7), while native zone-developer CreateLot
  follows `0x0072E530` → `0x006BF430` and `0x0072E557` → `0x006C1E50` without a factory lookup.
  Factory-only substitution therefore cannot claim all lots are replacement lots.
- **T29** PickUnitToJoin with exactly one eligible candidate lacking a unit creates one native 0x20-byte
  business unit at `0x006CB45C`, calls Add(candidate), then Add(current lot). No creation reference is acquired
  by PickUnitToJoin. The two member back-pointers retain two references to the unit (§4); a replacement
  factory returning one owned reference must release that extra reference after membership is established.
  With a candidate already in a unit, no constructor is called and only the current lot is added.
- **T30** ClearLot's trace includes `0x006C8A3D` → `0x005E3070(lot, false)` and `0x006C8A62` →
  `0x005E3960(lot)`. AddLotFoundation on its wall-creation branch includes `0x006C8C23` →
  `0x005E3070(lot, true)` and `0x006C8C34` → `0x005E3960(lot)`. Merely registering a replacement wall
  factory changes neither trace. Where edge flags require a new wall, `0x005E30F5` still constructs the
  native wall; the AL = 0 success quirk remains (§7.1).

### 17.5 Native mutation discriminators (LR10/LR11)

These independent vectors follow the cited native branches; they are not game executions. C is the
building profile capacity-vector, P the lot population-vector, k = `0x1010`, j = `0x1020`.
Where a slot-52 result is supplied, it is an explicit collaborator input, not an assumed capacity formula.
For T31–T36, GetBuildingType(true) supplies the capacity profile via configuration fallback, whereas
GetBuildingType(false) is zero: the successful wealth helper sets wealth to zero and does not insert
additional keys. T44 onwards separately exercise a stored building and the helper's insertion effects.

| Vector | Inputs | Required result / order |
|---|---|---|
| T31 | SetPopulation(k, 12); slot 52 returns 100; C=[], P=[] | AL=0; P remains []; no wealth update. Capacity query still occurs before the profile query. |
| T32 | SetPopulation(k, 120); slot 52 returns 100; C=[{k:100},{k:200}], P=[{k:7},{k:9}] | P=[{k:100},{k:9}]; one wealth update; AL=1. The second occurrence is untouched. |
| T33 | SetPopulation(k, 0); slot 52 returns 100; C=[{k:100}], P=[{k:7}] | P=[{k:0}], retaining k; wealth recomputed; AL=1. Repeating the call still recomputes wealth and returns AL=1. |
| T34 | SetPopulation(k, 3); slot 52 returns 100; C=[{j:20},{k:100}], P=[{k:9},{}] | P=[{k:9},{k:3}]; insertion is at the capacity-selected index, irrespective of the existing key in P[0]. |
| T35 | SetPopulation(k, 3); slot 52 returns 0; C=[{k:100}], P=[{}] | P=[{k:0}], AL=1: clamping to zero does not mean failure. |
| T36 | C=[{k:100}], P=[] | Selected P[0] does not exist. Native has no bounds/creation branch; do not turn this invalid invariant into a successful append. |
| T37 | State 3, day D0, wealth 2, P=[{k:7}]; SetState(3,1,6) | No calls, mutations, day update or message; wealth stays 2 and population stays 7. There is no defined boolean return. |
| T38 | No stored building type and no configuration fallback; state 1, wealth 2, P=[{k:7}]; day service returns D; SetState(2,0,6) | State=2, day=D, P unchanged (no rebuild), wealth=0, reason unchanged; power update before copy; data=(1,2,0,0). |
| T39 | Same as T38, but SetState(2,0x100,6) | Whole wealth is nonzero: final wealth=0 and data4=0 (low byte). Recompute occurs before override. |
| T40 | Stored nonzero type B with nonnull purpose-zero profile; state 3, wealth 2, P=[{k:7},{k:9}]; SetState(4,0,0x106) | Every count becomes zero; wealth remains 2; reason=6; no construction-resource release; data=(3,4,B,2). |
| T41 | Old state 2; stored type B with known purpose/native wealth and resource value R; SetState(3,0,0) | Day store precedes ReleaseConstructionResources(R), developer-type lookup, GetCapacity(devType,true), SetPopulation, outer wealth update, power update, copy, post, message Release, copy Release. Do not replace this with a synthetic successful transition. |
| T42 | State 1 → 2, GetBuildingType(true) supplies known B; C=[{k:100,j:50},{k:200}], previous P arbitrary | Rebuild produces P=[{k:0,j:0},{k:0}] without changing C; copy observes that rebuilt P. Original map nodes are destroyed; no per-entry SetPopulation calls. |
| T43 | Snapshot of lot with nonempty P, travel map, trips, runtime tree and configuration Q | P and travel-map entries copy independently; snapshot trips/runtime tree empty; Q gains one constructor reference; travel designate and unit null, neither retained. Subsequent live-lot P/travel-map edits do not alter the snapshot. |
| T44 | Stored residence profile, native wealth 2; C=[{k:100,j:100}], P=[{}], current wealth 1; call wealth update | P becomes [{k:0,j:0}], O={k:0.0,j:0.0}; wealth remains 1. With current wealth 0 or unsupported 3 instead, wealth becomes 2. |
| T45 | Same profile as T44, P=[{k:50,j:50}], current wealth 1 | O={k:0.5,j:0.5}; wealth=2, because descending positive ties favor the higher wealth. With P=[{k:75,j:25}], wealth=1. |
| T46 | Same profile, native wealth 2; C=[{k:100},{k:300,j:300}], P=[{k:100},{k:0,j:300}] | A=[100,300], S=400; O={k:0.25,j:0.75}; wealth=2. Duplicate k contributions are added, not overwritten. |
| T47 | Known stored residence profile, native wealth 2, C=[], P=[], current wealth 3 | Empty O; unsupported current wealth falls back to 2; no divide by zero. Current wealth 0 also becomes 2. |
| T48 | Same known profile but C=[{}], P=[{}] | GetOccupancyByType reaches signed integer division by zero at 006C1258; no successful wealth result. If reached during SetState, preceding state/day mutations have already happened and no copy/post is reached. |
| T49 | Unknown stored nonzero type B absent from catalogue, state 1, wealth 2, nonempty P; SetState(2,0,0) | First profile lookup creates the default profile (purpose=0, native wealth=0, C=[]); rebuild clears P to []; wealth stays 2; data=(1,2,B,2). No construction-resource release. |
| T50 | State 1, no stored/fallback building, wealth 2; SetState(0x101,0,0), day service D | Full argument differs from old byte: state byte still 1 but day becomes D, populations zeroed, wealth becomes 0, message data=(1,1,0,0). This is not the equal-argument no-op. |
| T51 | Native state 3, mixed-occupancy profile enabled, C=[{k:100,j:100}], P=[{k:0,j:25}], requested SetPopulation(k,90) | Slot 52 computes F=0.75, AX=75; assigned k=75, then wealth recomputed. With mixed occupancy disabled, AX=0 and assigned k=0; AL is 1 in both cases. |
| T52 | Old state 2, known stored profile resource count 0, new state 4 | Resource-counter slot 17 receives 0.0f, then slot 30 is called; no population-fill slot-50/55 pair. Every P count becomes zero, reason byte updates, and outer wealth/power/copy/post still follow. |
| T53 | One initialized live lot owns configuration Q and one nonnull property V, with no receiver retaining the queued snapshot; refcounts before call Q=q, V=v | Copy constructor: Q=q+1, V=v+1, copy refs=0. SetState copy AddRef:1; message IGZUnknown assignment:2. After post and local releases: copy refs=1, message refs=1 in queue, Q=q+1, V=v+1. Final message destruction releases copy; copy Shutdown/destruction return Q to q and V to v. Unit/travel-designate refcounts never change; live-lot refcount never changes. |
| T54 | Stored residence profile with native wealth 0, C=[{k:100}], P=[{k:50}] | O={k:0.5}; with initial wealth 0, wealth stays 0 (descending loop has no iterations). With initial wealth 1, its developer type k is present, so wealth stays 1. There is no forced minimum wealth of 1. |
| T55 | Stored unknown type B (default purpose-zero profile), state 4, wealth 1, P=[]; SetState(3,3,0) | State/day update; no rebuild/fill or resource release; computed wealth remains 1, then entering override stores 3; copy wealth=3 and data=(4,3,B,3). With wealth argument 0, copy wealth=1 and data4=1. |
| T56 | Stored residence B, native wealth 2, C=[{k:100,j:100}], P=[{k:0,j:0}], state 2, wealth 1; SetState(3,1,0) | Release resources, GetCapacity(j,true)=100, SetPopulation(j,100) clamps to 100; P=[{k:0,j:100}], nested wealth=2; outer wealth=2; explicit wealth argument 1 ignored within {2,3}; copy wealth=2, data=(2,3,B,2). |
| T57 | No stored/fallback building, state 3, wealth 2, P=[{k:7}]; SetState(4,3,6) | Counts zeroed; computed wealth=0, leaving argument 3 ignored; copy wealth=0 and reason=6, but data4=2. Power bit cleared before copy. Copy contents and message data4 intentionally differ. |

T44–T46 use normal positive capacities; zero capacity denominators retain native x87 nonfinite behavior
rather than a repaired fraction. For any matching C index without a corresponding P map, no portable
successful result is specified: the native unchecked index has no defined container semantics. These
invalid shapes do not justify an inferred map-creation rule. No remaining LR10/LR11 gap is needed for
valid native container invariants; these vectors specify behavior, not a runtime or coverage verdict.
