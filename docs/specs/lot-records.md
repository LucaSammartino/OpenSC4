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
- `Read` returns false (and stops) on a version it does not accept; otherwise it returns `GetError() = 0`. The class
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
`0x00AB1930` (sub-object +8), `cISC4TrafficSource` `0x00AB1914` (sub-object +0x1C). It zeroes every field.

The lot does **not** save its occupants. `GetBuilding` and `GetLotOccupants` query the occupant manager over the
lot's bounding rect (with an "inside lot" filter) each time *[compat]*. The occupants' only link back is the
**lot tag** field of props and flora (§11, §12) and the lot reference of walls and constructions.

### 3.2 Record *[compat]*

| # | Field | Type | Meaning, observed values (12,986 lots) |
|---|---|---|---|
| 1 | version | u16 | 8 |
| 2 | lot configuration ID | u32 | `GetID()` (configuration slot 3) of the lot's configuration object; 0 when it has none (692 lots: the 691 empty zoned lots and one more) |
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

1. If the lot is **not** initialised (a new lot): travel jobs := 0, 0, 0; `altitude := terrain.GetAltitude(center)`
   via `SetAltitude` (which also sets both slopes); initialised := true. A loaded lot skips this (step 1 of §3.4),
   so its saved altitude and slopes are kept.
2. If the strategy byte is `0xFF` (constructor value): `RandomUint32Uniform(100)` on the lot file's `cRZRandomSuper`
   `0x00B49120` (foundations §6.2), one draw. Saved lots have 0…99, so no draw on load.
3. Release the current configuration object. If the configuration ID is non-zero: look it up with the lot
   configuration manager (`0x00B43D18`, slot 8 `GetLotConfiguration(id)`) and keep the result (AddRef). Then set
   the stored ID to 0. If the configuration is missing from the data (for example a removed plug-in), the lot has no
   configuration and its next save writes 0 *[compat]*.
4. Return true.

`PostCityInit` (`0x006BF200`, slot 8): `f := flags; flags := 0; SetFlags(f)`. `SetFlags` acts on a change of bit 0:
when bit 0 becomes set, each cell of the rect is locked in the terrain view at the lot's altitude; when it becomes
clear, each cell is unlocked. So a loaded lot with bit 0 re-locks its terrain *[compat]*.

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
| 14 | unnamed | s8, 5 × u32, 2 × f32, 5 × u32, s8, 2 × f32, bool, 2 × u32, 2 × f32, u8 | construction / crane animation and daily progress state *[inferred]* |
| 15 | crew | u32 n + n × {bool, bool, ref} | ref IID `0xA9B40F05` (`cISC4Automaton`; class `0x896E75AF` `cSC4Pedestrian`, record in `save-records.md`); n 0…5 |

`Read`: versions < 4 → false; versions ≥ 4 are read with this layout, version 4 using the legacy transform reader
(`0x005DA930`). Versions > 5 are read as 5 *[quirk]*.

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
| `cSC4TripData` `0x0070E0B0` / `0x0070E160` (non-virtual, `this` = the trip) | EAX = 0/1 | EAX = 0/1 |

A replacement returns a clean 0/1 in EAX; callers in the original test AL only (`bool`).

### 13.2 Slots of other interfaces used by these records

| Interface | Slot (offset) | Use | Pop | Return |
|---|---|---|---|---|
| `cISC4Lot` (vtable `0x00AB19B0`) | 5 (+0x14) `Init` `0x006BF490` | §3.5 | 0 | AL = 1 |
| | 7 (+0x1C) `Shutdown` `0x006C3060` | §3.5 | 0 | AL = 1 |
| | 8 (+0x20) `PostCityInit` `0x006BF200` | §3.5 | 0 | none defined (tail of SetFlags) |
| | 12 (+0x30) `GetFlags` `0x006C2030` | §3.2 | 0 | EAX = flag byte, zero-extended |
| | 13 (+0x34) `SetFlags(u32)` `0x006BF220` | §3.5 | 4 | none defined (SDK: bool) |
| | 15 (+0x3C) `GetCenter(out)` `0x006BF560` | Init | 4 | out-parameter (abi_extract also reports x87) |
| | 17 (+0x44) `SetAltitude(float)` `0x006BF5C0` | Init | 4 | none defined |
| | 34 (+0x88) `SetState(state, u, reason)` `0x006C3100` | §3.2 | 12 | none defined |
| | 44 (+0xB0) `SetHistorical(bool)` `0x006C2230` | Read v < 6 | 4 | none |
| | 54 (+0xD8) `GetPopulations` `0x006C21E0` | Write | 0 | EAX = pointer to the vector |
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

- **Class registry**: the loader creates every object by class ID through GZCOM (foundations §2.7). A replacement
  must register all classes of §1 (including the prop subclasses and `cSC4Occupant`) or loading a city fails for
  those classes (foundations §3.5).
- **Non-virtual entry points used across classes**: `cSC4TripData::Write/Read` (called from the lot and from
  neighbour connections, `neighbors.md`); `cSC4LotRetainingWallOccupant::AddRetainingWallOccupant` and
  `UpdateSurroundingRetainingWalls` (called by the lot developer); `GetPropTypeIndex` and the process-wide prop type
  table (used by every prop). Replacing the lot developer or traffic simulator alone therefore keeps these
  original helpers, or replaces them together.
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
configuration ID is dropped when the configuration is missing (§3.5); the crime-only lot (§14).

Open questions:

- **Q1** The exact rule for the occupant manager's cell rectangle (§2.3 field 6) belongs to the occupant manager;
  the `floor(x / 64) + 64` approximation misses edge and flora-model cases.
- **Q2** Where do vacate-reason values > 6 come from? Every observed caller passes 0…6 (`UpdateLot` 0…4, tract
  developer 5 and 6); all vacant lots in saves hold 1…6, so the large values were written in an earlier state or
  build. A writer copies the byte.
- **Q3** Names of the 19 scalar fields of the construction occupant (§9 #14).
- **Q4** Meaning of the five `0xFF`-default bytes of a base-texture record (§6) and of the effect prop's two bytes
  and u32 (§10.2).
- **Q5** Who sets lot flag bit 7 (§3.2)?
- **Q6** Layouts of the burnt-building, terrain-decal and automata-controller prop subclasses (absent from saves).

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
