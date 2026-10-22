# Object layouts that original code may read directly

Interface facts (see [README.md](README.md)). They give field offsets of game objects whose memory the original code,
or code written in the same style, may read without going through a virtual call. A replacement object of one of
these kinds should use the same layout. If it doesn't, any direct read gets wrong data. The facts were read from the
target executable's constructor and accessor bodies; no logic is recorded here.

## Standard message (`cIGZMessage2Standard`, original class `cRZMessage2Standard`, 0x2C bytes)

Vtable `0x00ACEC00`. It has 38 slots: slot 37 is the scalar deleting destructor, and slots 0–36 follow the
compiled order in foundations §5.1.

| Offset | Size | Field |
|---|---|---|
| +0x00 | 4 | vtable |
| +0x04 | 4 | reference count. Release destroys the object when the count is ≤ 1, otherwise it decrements. |
| +0x08 | 1 | flags byte (bits below), followed by 3 bytes of padding |
| +0x0C | 4 | type |
| +0x10 | 4 | data1 / void1 |
| +0x14 | 4 | data2 / void2 |
| +0x18 | 4 | data3 / void3 |
| +0x1C | 4 | data4 / void4 |
| +0x20 | 4 | string (`cIGZString*`, counted reference) |
| +0x24 | 4 | IGZUnknown (counted reference) |
| +0x28 | 4 | extra (`cIGZSerializable*`, counted reference) |

Flags byte at +0x08:

| Bit | Mask | Meaning | Set by | Cleared by |
|---|---|---|---|---|
| 0 | 0x01 | has type | SetType with a non-zero type | SetType(0) |
| 1 | 0x02 | has data1 | SetData1 or SetVoid1 (any value, even 0) | never |
| 2 | 0x04 | has data2 | SetData2 or SetVoid2 | never |
| 3 | 0x08 | has data3 | SetData3 or SetVoid3 | never |
| 4 | 0x10 | has data4 | SetData4 or SetVoid4 | never |
| 5 | 0x20 | has extra | SetExtra(non-null) | SetExtra(null) |
| 6 | 0x40 | has string | SetString(non-null) | SetString(null) |
| 7 | 0x80 | has IGZUnknown | SetIGZUnknown(non-null) | SetIGZUnknown(null) |

The pointer setters (string, IGZUnknown, extra) AddRef the new value and Release the old one. A new object is
all zeros apart from the vtable.

## Dispatch record (`cISC4DispatchManager::DispatchInfo`, 0x2C bytes)

`cISC4DispatchManager::GetDispatchInfo(uint32_t id, DispatchInfo& info)` (slot 10, `0x00469D40`, pop 8) looks the
ID up in the manager's own list of records and copies the whole record, 11 dwords, into the caller's struct
(`rep movsd`). It returns AL = 1, or AL = 0 with the struct untouched when the ID is unknown. The copy takes **no
references**: the three pointers in it are borrowed from the manager's record, so the caller must not Release
them and must not keep them after the dispatch ends.

| Offset | Size | Field |
|---|---|---|
| +0x00 | 4 | dispatch type (`tDispatchType`). `GetDispatchCount` (slot 11) and `GetDispatchIDArray` (slot 12) compare against it. The air dispatch sources handle type 5 (fire plane) and 6 (police helicopter) |
| +0x04 | 4 | dispatch ID: the key of slot 10, handed out by the manager from a counter |
| +0x08 | 4 | the station's vehicle slot that serves the dispatch (`uint32_t` field; only the low 16 bits are used) |
| +0x0C | 4 | handle from the dispatch vehicle view's tracking of the vehicle (0 when not tracked) |
| +0x10 | 1 | bool: set while the dispatch is in state 5 (`UpdateDispatchState`), followed by 3 bytes of padding |
| +0x14 | 4 | target x (`float`) |
| +0x18 | 4 | target y (`float`, the terrain altitude at x, z) |
| +0x1C | 4 | target z (`float`) |
| +0x20 | 4 | station occupant (`cISC4Occupant*`); the manager's record holds one reference |
| +0x24 | 4 | vehicle automaton (`cISC4Automaton*`); the manager's record holds one reference |
| +0x28 | 4 | pylon occupant; the manager's record holds one reference |

Evidence: Windows `0x00469D40` (copy; ID at record +4), `0x00469D80` and `0x00469DA0` (type at +0),
`0x0046A630` (`Dispatch`: new record); the Mac build's `GetDispatchInfo` `0x001870CC` (same 11-dword copy),
`InitDispatch` `0x0018752A`, `Dispatch` `0x0018829C`, `BeginDispatch` `0x00187EAE`, `EndDispatch` `0x00188D4C`
(releases +0x20, +0x24 and +0x28), `UpdateDispatchState` `0x00186E70` (+0x10).

## Static flora record (`cSC4StaticFloraData`, 0x178 = 376 bytes)

One record per flora type, owned by the flora simulator and handed out by `GetStaticFloraData` (flora slot 7) as
a raw pointer. **It has no vtable**: +0 is the type ID. It is not a GZCOM object. Its reference count is a plain
field that holders change directly, and its release function is non-virtual.

| Offset | Size | Field |
|---|---|---|
| +0x000 | 4 | type ID (`uint32_t`) |
| +0x004 | 4 | next cluster type (`uint32_t`) |
| +0x008 | 4 | seed type (`uint32_t`) |
| +0x00C | 1 | water flora (`bool`), followed by 3 bytes of padding |
| +0x010 | 56 | 14 `float`s, the values of property `0x083DC35B` in order: +0x10 maturity age, +0x14 seeding age, +0x18 seeding interval, +0x1C seeds per cycle, +0x20 seed spread, +0x24 lifespan, +0x28 maximum age, +0x2C slope tolerance, +0x30 minimum propagation score, +0x34 seed probability, **+0x38 occupant box half-width**, +0x3C unused, +0x40 clearance radius, **+0x44 height** (names as in the flora spec §3.3) |
| +0x048 | 4 | cluster radius (`float`) |
| +0x04C | 4 | cluster height (`float`) |
| +0x050 | 256 | propagation table, 16 × 16 bytes, `[elevation index][moisture type]` (rows of 16 bytes) |
| +0x150 | 4 | `uint32_t` from property `0x083DE16F` |
| +0x154 | 12 | family IDs: `std::vector<uint32_t>` `{begin, end, capacity_end}`; the buffer is a **B** block, or null |
| +0x160 | 8 | plop cost (`int64_t`) |
| +0x168 | 4 | the exemplar (`cISCResExemplar*`), a counted reference, or null |
| +0x16C | 1 | flags byte, followed by 3 bytes of padding |
| +0x170 | 4 | reference count (`int32_t`) |
| +0x174 | 4 | no access found: tail padding (the `int64_t` makes the record 8-aligned) |

Defaults: `0x006B0220` writes +0x000 to +0x150 (not the flags byte, not +0x160). The inline constructors at
`0x006B2BBB` and `0x006B133F` zero +0x34, +0x154, +0x158, +0x15C, +0x168 and +0x170.

**Memory.**
- The simulator creates records with the game's `operator new(0x178)`, the **global heap (C, `0x005E55E0`)**:
  `0x006B2BAF` (the key-0 default record in `Init`) and `0x006B1333` (`AddStaticFloraData` from an exemplar). The
  creator then increments +0x170.
- There is no AddRef function. Holders increment +0x170 directly.
- **Release** is `0x00472C70`: non-virtual, `__thiscall`, `this` = the record, no arguments, `ret` with no pop.
  When the count is ≤ 1 on entry it destroys the record: it releases the exemplar at +0x168 (vtable slot 2) if
  non-null, frees the family buffer at +0x154 with **B** (`0x0090CF63`) if non-null, and frees the record with
  **C** (`0x005E5620`). Otherwise it decrements the count.
- **Code outside the simulator frees records.** `cSC4FloraOccupant` holds one counted reference to its record:
  its setter (`0x00472CD0`) increments the new record's count (`0x00472CE3`) and calls Release on the old one
  (`0x00472CF0`), and its destructor calls Release (`0x00472D5F`). When an occupant drops the last reference, the
  record is freed there with C. Occupants are not tied to one city, so they can outlive the simulator's own
  references.

**Raw readers outside the simulator's methods** (a record handed to the game must have these fields at these
offsets):

| Offset | Read by |
|---|---|
| +0x038, +0x044 | `cSC4FloraOccupant::GetBoundingBox` `0x00472AB0` (`0x00472AF0`, `0x00472AF3`) |
| +0x160 | the flora tool's `Init` `0x004BBA20` (`0x004BBA5C`, `0x004BBA65`, as two dwords) |
| +0x168 | the flora occupant's setter `0x00472CD0` (`0x00472D02`: passes the exemplar to the occupant's model) |
| +0x16C | the terrain flora brush `0x0074B5E0` (`0x0074B658`) |
| +0x170 | the flora occupant's setter (increment) and Release (`0x00472C70`) |
| +0x154 to +0x15C, +0x168 | Release (frees and releases) |

A record that the plugin builds and hands to the game must therefore be 0x178 bytes with this layout. Either the
record is a C block with a B (or null) family buffer and a counted (or null) exemplar, so that the game's Release
can free it, or the plugin keeps one reference for the life of the process, so that no game Release ever sees a
count of 1 or less.

## Game string (`cRZString`, 0x14 bytes; the SDK's `SC4String` matches it)

Some game code reads a `cIGZString` argument directly as a `cRZString`, without a virtual call. For example, the
automata controller manager's by-name group lookup (`AddControllersFromOccupantGroup` /
`RemoveControllersFromOccupantGroup`, slots 10/12; its string-keyed hash lookup `0x0051CFC0` → `0x0051C9F0`; QUESTIONS
PR3). An argument passed there must have this layout:

| Offset | Field |
|---|---|
| +0x00 | vtable (`cIGZString`) |
| +0x04 | `char*` begin (STLport `std::string` start) |
| +0x08 | `char*` end |
| +0x0C | `char*` capacity end |
| +0x10 | `uint32` reference count |

- The text is always in a separate buffer that begin points to. There is no inline small-string storage, so
  +0x04 is never text.
- The SDK's `SC4String` has exactly this layout (`static_assert(sizeof(SC4String) == 0x14)`, heap buffer). A
  string class with any other layout, such as an inline buffer or a different member order, must not be passed
  where the game reads it raw.
- The game's own constructor `cRZString(const char*)` is at `0x00408480`.

## Building profile (`cISC4BuildingOccupant::BuildingProfile`, 0x40 bytes)

There is one profile per **building type**, shared by every building of that type and by every city. Two calls
hand it out:
- `cISC4BuildingDevelopmentSimulator::GetBuildingProfile(uint32 type)` (compiled slot 39, `+0x9C`, `0x0069CA60`,
  pop 4) returns a reference in EAX. It inserts a zero profile for an unknown type.
- `GetBuildingProfilePtr(type)` (slot 40, `+0xA0`, `0x006985C0`, pop 4) returns a pointer, or null for an unknown
  type.

`cISC4BuildingOccupant::GetBuildingProfile` (compiled slot 11, `+0x2C`, `0x00459250`) forwards to slot 39 with the
occupant's type (occupant + 0x38). When the BDS global is null, it returns a static zero profile instead. **Read
only**: never free or modify a profile.

Both BDS calls ignore `this`. The profiles live in one process-wide STLport `hash_map<uint32_t, BuildingProfile>`
at `0x00B48584` (the 0x14-byte hash table of allocators.md *hash_set*: bucket vector at `0x00B48588`–`0x00B48590`,
count at `0x00B48594`, identity hash).
- Each entry is a **0x50-byte B node**: next pointer at +0x00, the key (the building type) at +0x08, and the
  profile at **+0x10** (the pair is 8-aligned because of the `int64` fields). Allocation `0x0069B683`; the profile
  is copied in by the copy constructor `0x004D1230`.
- A profile does not move while it exists: inserting other types only relinks nodes.
- When a type's exemplar is loaded again, `LoadBuildingProperties` replaces its profile:
  - with its last argument false, it first erases the entry (`0x0069BF29` → `0x0069B550`), so a new node is made;
  - otherwise it assigns over the old profile in place, which reallocates the capacity vector (`0x0069C098`).

  A held pointer or reference is therefore valid only until the type is reloaded *[inferred: reloads happen while
  the catalogue is built (`AddBuildingData` `0x0069D0FF`), not during a simulation tick]*.

| Offset | Size | Type | Field | Filled from (`LoadBuildingProperties` `0x0069B910`) | Read by (examples) |
|---|---|---|---|---|---|
| +0x00 | 4 | uint32 | `purpose` (`PurposeType`, 0..9; 0 = none) | property `0x27812833` (uint8, zero-extended); absent → 0 (`0x0069BB9F`) | `cSC4BudgetSimulator::ComputeTax` `0x00691F04`; demand `UpdateCapacity` `0x006A2C96` and `PerformCensus` `0x006A5A5D`; BDS `CalculateMaxCapacity` `0x0069C7B9` and `DoBuildingChanged` `0x0069CACC`; `cSC4Lot::SetState` `0x006C3157`; `cSC4Lot::UpdateNeedsPower` `0x006BFD74`; `cSC4GrowthDeveloper::UpdatePopulationCount` `0x006B51E0` |
| +0x04 | 4 | uint32 | `wealth` (`WealthType`, 0..3) | property `0x27812832` (uint8); outside 0..3 → 0 with a debug message; absent → 0 (`0x0069BAFF`) | `ComputeTax` `0x00691FC9`; `DoBuildingChanged` `0x0069CB47`; `UpdateNeedsPower` `0x006BFD7A` |
| +0x08 | 2 | uint16 | **capacity**: the entry for this building's own developer type, `GetDeveloperType(purpose, wealth)`, in the capacity map; 0 when that key is absent or the property is missing | `0x0069BCA7`–`0x0069BCAA`, `0x0069BCF2` | `CalculateMaxCapacity` `0x0069C7C6`; `cSC4Lot::SetState` `0x006C3160` (releases construction resources); tract developer `Build` (building-development.md §4.6) |
| +0x0A | 1 | uint8 | construction duration in days | property `0x499AFA38` (uint8); absent → 30 (`0x0069BD45`) | lot developer construction (lot-developer.md §6.7.4) |
| +0x0B | 1 | | padding | | |
| +0x0C | 4 | `map*` | capacity maps: vector `begin` | property `0x27812834` (below) | demand `UpdateCapacity` `0x006A2CB0`; `cSC4Lot::GetCapacity` `0x006C05D9`; `GetAvailableCapacity` `0x006C068C`; `GetOccupancy` `0x006C0D27`; `GetOccupancyByType` `0x006C1158`; `SetPopulation` `0x006C1980`; `SetState` `0x006C31F3` (copies each map into the lot) |
| +0x10 | 4 | `map*` | vector `end` | | the same |
| +0x14 | 4 | `map*` | vector `capacity_end` | | |
| +0x18 | 1 | bool | flag from property `0x27812836` (bool) *[meaning not traced]* | absent → false (`0x0069BD00`; stored at `0x0069BD1C`) | no reader found |
| +0x19 | 3 | | padding | | |
| +0x1C | 4 | uint32 | building category flags (occupant groups, building-development.md §4.7.1.1) | property `0xAA1DD396` via `SetBuildingCategoryFlags` `0x00696B80` (`0x00696B9A`, `0x00696BF4`) | |
| +0x20 | 12 | float[3] | occupant size (x, y, z) | property `0x27812810`; absent → 0, 0, 0 (`0x0069BD7E`–`0x0069BDAC`) | |
| +0x2C | 4 | | padding: never written. The copy constructor and the assignment skip it | | |
| +0x30 | 8 | int64 | plop cost | property `0x49CAC341` (`int64`); absent → 0 (`0x0069BDBB`) | `cSC4ViewInputControlPlaceLot::Init` `0x004C16A7` |
| +0x38 | 8 | int64 | the sum of the `cost` fields (`BudgetItem` + 0x10) of the building's budget items | budget simulator slot 50 `GetBudgetItemInfo(holder, vector&)` (`0x0069BE00`), summed at `0x0069BE20` | |

Total 0x40 bytes, 8-aligned. Evidence: the copy constructor `0x004D1230` (+0x00, +0x04, word +0x08, byte +0x0A,
vector at +0x0C, byte +0x18, +0x1C, 12 bytes at +0x20, +0x30..+0x3F), the zero profile built in `0x0069B720`, and
the field-by-field store at the end of `LoadBuildingProperties` (`0x0069C071`–`0x0069C0DF`). The Mac build's
constructor and copy constructor (Mac `0x0065093E`, `0x00658A5E`) have the same members in the same order. Without
the 8-byte alignment of `int64` there, the two `int64`s sit 4 bytes lower.

The SDK names only `purpose` and `wealth`. Both are `uint32_t` enums at these offsets.

### The capacity maps (+0x0C)

- **Element type**: `std::map<uint32_t, uint16_t>`, the STLport map of allocators.md *map and set*. The vector's
  elements are the 12-byte map objects (header pointer, count, comparator); `end − begin` is a multiple of 12.
  Each map has its own B header, and 0x18-byte B nodes: key at +0x10, `uint16_t` at +0x14.
- **Key**: a **developer type**, which is the demand ID of the local capacity node it feeds: 0x1010 / 0x1020 /
  0x1030 (R$ / R$$ / R$$$), 0x3110 / 0x3120 / 0x3130 (CS), 0x3320 / 0x3330 (CO), 0x4100 / 0x4200 / 0x4300 /
  0x4400 (I-R, I-D, I-M, I-HT) (demand.md §2).
- **Value**: capacity units of that type: residents for R, jobs for C and I.
- **Filled** by `LoadBuildingProperties` (`0x0069BBA9`–`0x0069BC83`), once per building exemplar, while the
  building catalogue is built (its only caller is `AddBuildingData`, `0x0069D0FF`):
  - property `0x27812834` is a `uint32` list read as pairs (key, amount). Each pair sets `map[key] := low 16 bits
    of amount` (`0x00699030`); a key repeated later overwrites the earlier one;
  - the map is then **pushed** into the vector (`0x0069BC59` / `0x0069BC7E`, B buffer).
  - Without the property, nothing is pushed and the vector stays empty (begin = end = null).
- **One element at most.** `LoadBuildingProperties` is the only code found that builds a catalogue profile's
  vector, and it pushes at most one map. So in the shipped game the vector holds **0 or 1** maps. The other
  profile copies the game makes copy the profile **out** into locals:
  - the occupant filters around `0x004E4260`–`0x004E49B0` (assignment `0x004E41F0` into a stack copy);
  - `cSC4ViewInputControlQuery::GetBuildingWealth` (`0x004D3260`).
- **Readers.** The type is a vector, so code must not assume one map:
  - the demand simulator's `UpdateCapacity` (`0x006A2C80`) **adds up** the entry for the developer type over
    every map;
  - `cSC4Lot::GetCapacity` (`0x006C05A0`) returns the entry of the **first** map that contains it.

  With 0 or 1 map the two agree.
- **Never modified afterwards.** `cSC4Lot::SetState` copies the maps into the lot's own vector when a building is
  set (`0x006C31F3`, copy constructor `0x004C9470`), and the lot's occupancy code works on those copies.

### How `UpdateCapacity` reads it (demand.md §7.2)

`0x006A2C80`, from the demand simulator's `kMessageStateChanged` handler:
1. `GetBuildingProfile(bt)` (slot 39). If `purpose` (+0x00) is 0, stop.
2. Get `devType := GetDeveloperType(purpose, wealth)` (BDS slot 30, `+0x78`). The wealth here is the message's
   `data4`, not the profile's +0x04.
3. For each 12-byte map from +0x0C to +0x10, look up `devType` (a lower-bound search from the map's root,
   `0x006A2CC0`–`0x006A2CEE`). When found, add the `uint16_t` at node + 0x14, zero-extended, to a 32-bit sum
   (`0x006A2CF2`).
4. If the demand node `devType` exists (the simulator's map at + 0x50), call its `AddToSupplyValue` (slot 16,
   `+0x40`) with `float(±sum)`: + when the lot becomes occupied, − when it stops being occupied
   (`0x006A2D34`–`0x006A2D4F`; the sum is negated as an integer, then converted).

**Worked example.** A low-wealth house whose exemplar has `0x27812834` = (0x1010, 12) has purpose 1 and wealth 1,
and its vector holds one map {0x1010 → 12}. With `data4` = 1, `GetDeveloperType(1, 1)` = 0x1010, the sum is 12,
and the R$ node's supply moves by +12.0 or −12.0. With `data4` = 2 the lookup is 0x1020, which is absent, so the
sum is 0 and node 0x1020 still gets a call with 0.0.

## Lot and occupant getters used by the property tax (verified 2026-10-12)

These were checked against `ComputeTax` (`0x00691E20`) and the plugin's `FC4Interfaces.h` declarations. All
match.

| Call | Compiled slot | Body | Returns |
|---|---|---|---|
| `cISC4Lot::GetLocation(int32&, int32&)` | 21 (`+0x54`) | `0x006BF2E0` | the lot's origin cell; `ComputeTax` passes both outputs, in order, to `cISC4LandValueSimulator::GetLandValue(x, z)` (slot 6) |
| `cISC4Lot::GetSize(uint8&, uint8&)` | 22 (`+0x58`) | `0x006BF300` | the lot size in cells |
| `cISC4Lot::IsInLivableState()` | 37 (`+0x94`) | `0x006C2150` | `al = (byte [lot+0x7F] == 3)`, with EAX zeroed first |
| `cISC4Lot::GetOccupantWealth()` | 41 (`+0xA4`) | `0x006C21F0` | `movzx eax, byte [lot+0x82]`: the lot's own occupant wealth, 0..3. This is not the building profile's wealth. `cSC4Lot::UpdatePrimaryOccupantWealth` (`0x006C1430`) maintains it. |
| `cISC4Lot::GetBuilding()` | 48 (`+0xC0`) | `0x006BFA30` | borrowed `cISC4BuildingOccupant*` or null |
| `cISC4BuildingOccupant::AsOccupant()` | 3 (`+0x0C`) | `0x005FD7E0` | `this + 4`, or null for null |
| `cISC4Occupant::AsPropertyHolder()` | 6 (`+0x18`) | `0x006C2040` | `occupant + 8`. `cISC4Occupant::QueryInterface(0x25216283)` (`0x005FD310` via `0x005E89C0`) returns the same pointer, with one reference added. `cISC4BuildingOccupant::QueryInterface` (`0x004587D0`) adds 4 and continues there, so a QI on the building pointer also gives `AsOccupant()->AsPropertyHolder()`. |

The building value `0xE91A0B5F` is read with `cSCPropertyHelp::GetPropertyValue(holder, id, int64&)`
(`0x005FD4B0`). It calls holder slot 9 `GetProperty`, then property slot 5 (the value variant), then variant
slot 34 (`+0x88`, `GetValSint64(int64&)`, `0x009177E4`). The variant read copies the value directly if its
type is `Sint64` (9) and otherwise converts through `0x00917350`. It returns false for a missing property, and
the caller's 0 then stays.

## Landfill tile occupant (`cSC4LandfillOccupant`, re-answers8 LG27/LG28)

| Offset | What |
|---|---|
| `+0x00` | object start; `QueryInterface` (`0x00477130`) answers only `0x2A15F262`, `1`, `0x26BB767E`, `0xE4FDA3D4`, never `0x0AC2B5F7` |
| `+0x04` | the `cISC4Occupant` sub-object (vtable `0x00A8CDA0`): `AsPropertyHolder()` (slot 6) returns `+0x0C` of the object, null for null; `GetType()` (slot 7) returns the dword at `+0x28` of the object (`0x6A11BD2B` for landfill tiles) |
| `+0x20` | the `cISCExemplarPropertyHolder` sub-object (vtable `0x00A8CD08`) = `cISC4Occupant* + 0x1C` = `AsPropertyHolder() + 0x14` |
| `+0x24` | the default exemplar pointer: `SetDefaultExemplar` (holder slot 3) stores it, `GetDefaultExemplar` (holder slot 4) returns it without an AddRef |

A plugin creating landfill tiles calls `SetDefaultExemplar` on the `+0x20` sub-object before `Init` and before
inserting the tile (landfill.md, LG27).

## Prop profile (`cISC4PropManager::PropProfile`, 0x1C bytes)

`cISC4PropManager::GetPropProfile(uint32_t propID)` (slot 13, `0x004A3750`, pop 4) returns a **pointer into the prop
manager's static profile hash map** (`0x00B2179C`; lookup `0x0065FAD0`). An ID that is not in the map is
**inserted** with an all-zero profile (7 zero dwords) and that new entry is returned, so the result is never null.
The caller borrows it and must not free it *[inferred: each entry is a separately allocated node, so the pointer
stays valid while the map lives]*.

| Offset | Size | Field |
|---|---|---|
| +0x00…+0x0C | 16 | not read by the prop developer, the wealth requester or the crime simulator (not mapped by this lane) |
| +0x10 | 4 | requester satisfaction ID (`uint32_t`, exemplar property `0x49A1E05A`); compared with `GetRequesterSatisfactionId()` (prop developer `0x006F16C3`, `0x006F1757`) |
| +0x14 | 1 | wealth (`uint8_t`, property `0xE9A316EB`, default 0); compared by the wealth requester's `SelectProp` (`0x006F1D74`) |
| +0x18 | 4 | crime incident type (`uint32_t`, property `0x8AA864D5`); read by the crime simulator's `SelectProp` (`0x006A016E`, crime.md) |

Hash map node: `+0` next, `+4` key (prop ID), `+8` the 0x1C-byte profile (the returned pointer).

## Wealth prop requester (`cSC4WealthRequester`, class `0x89C49986`, 0xC bytes)

| Offset | Size | Field |
|---|---|---|
| +0x00 | 4 | `cISC4PropRequester` vtable `0x00AB29B0` |
| +0x04 | 4 | `cIGZSerializable` vtable `0x00AB2998` |
| +0x08 | 4 | reference count; the last `Release` (`0x006F1040`) deletes it |

## City detail manager (`cSC4CityDetailManager`, class `0xEAD4BA36`, 0x10 bytes)

| Offset | Size | Field |
|---|---|---|
| +0x00 | 4 | `cISC4CityDetailManager` vtable `0x00A87888` |
| +0x04 | 4 | `cIGZSerializable` vtable `0x00A87870` |
| +0x08 | 4 | detail level the city was last initialised with (`0xFFFFFFFF` = none); saved; slot 5 returns it |
| +0x0C | 4 | reference count |

These two objects are not read directly by other game code; the tables are here because a plugin replacing them
must keep the sub-object offsets that `QueryInterface` hands out. The other re-specs10 components keep their
state layouts in their specs (§2 of each); no other game code reads those fields directly.

## Occupant manager (`cSC4OccupantManager`, CLSID `0x098F964D`, 0x9C bytes, re-specs13b)

Factory `0x004A18A0`. Full behaviour in [occupant-manager.md](../specs/occupant-manager.md).

| Offset | Size | Field |
|---|---|---|
| +0x00 | 4 | `cISC4OccupantManager` vtable `0x00A8F7F0` |
| +0x04 | 4 | `cIGZSerializable` vtable `0x00A8F7D4` |
| +0x08 | 4 | `cIGZMessageTarget2` vtable `0x00A8F7C4` |
| +0x0C | 1 | initialised |
| +0x0D | 1 | set by Read, never read |
| +0x10 | 4 | reference count |
| +0x14 | 8 | `cRZCriticalSection` (recursive) |
| +0x1C | 4 | `float` sizeX |
| +0x20 | 4 | `float` sizeZ |
| +0x24 | 4 | `int32_t` manager cell count X |
| +0x28 | 4 | `int32_t` manager cell count Z |
| +0x2C | 4 | `int32_t` standard cell count X |
| +0x30 | 4 | `int32_t` standard cell count Z |
| +0x34 | 4 | shared type filter pointer |
| +0x38 | 12 | STLport `vector<vector<Holder*>>` grid (rows = x; storage B) |
| +0x44 | | InternalOccupantHashSet (buckets A) |

- Cell holder (4 bytes, C): pointer to a 12-byte circular list head (B). Node (12 bytes, B):
  `{next, prev, value = occupant | singleCellBit}`.
- Shared type filter (16 bytes, C; CLSID `0xE9C75ED1`): +0x0 vtable `0x00A8F79C`; +0x4 `cIGZSerializable` vtable
  `0x00A8F784`; +0x8 `uint32_t` reference count; +0xC `uint32_t` type.
- `cISC4Occupant` manager box: 4 bytes `{x0, z0, x1, z1}`, world cells, inclusive, at occupant +0x28 in the base
  class; a first byte of 0xFF means no box (base `GetOccupantManagerBBox` `0x005E8BF0`).

## Lot configuration (`cISC4LotConfiguration`, original class `cSC4LotConfiguration`, 0xC0 bytes)

From re-specs15a (lot-configuration.md §2). Vtable `0x00A8CEB8`, 34 slots: slot 33 is the scalar deleting
destructor. The lot configuration manager writes +0x3E, +0x40, +0x80 and +0x8C directly (not through a slot) when it
receives `kSC4MessagePostCityInit` (`0x0048AD40`), so a replacement configuration that the original manager fills
must keep these four offsets.

| Offset | Size | Field |
|---|---|---|
| +0x00 | 4 | vtable |
| +0x04 | 4 | reference count |
| +0x08 | 4 | ID (exemplar instance) |
| +0x0C | 0x14 | name (`cRZString`) |
| +0x20 | 1 | width in cells |
| +0x21 | 1 | depth in cells |
| +0x24 | 0x0C | family IDs (`vector<uint32>`) |
| +0x30 | 4 | maximum slope (f32) |
| +0x34 | 4 | minimum slope (f32) |
| +0x38 | 4 | maximum slope before foundation (f32) |
| +0x3C | 1 | growth stage |
| +0x3E | 2 | minimum building capacity (u16; manager-written) |
| +0x40 | 2 | maximum building capacity (u16; manager-written) |
| +0x42 | 1 | water-compatible flag |
| +0x44 | 0x14 | zone types (`vector<bool>`, 16 bits) |
| +0x58 | 0x14 | wealths (`vector<bool>`, 4 bits) |
| +0x6C | 0x14 | purposes (`vector<bool>`, 9 bits) |
| +0x80 | 0x0C | styles / occupant groups (`vector<uint32>`, ascending; manager-written) |
| +0x8C | 1 | monopoly flag (manager-written; nothing reads it) |
| +0x8D | 1 | do-construction flag |
| +0x8E | 1 | required road sides (bits 0–3) |
| +0x90 | 0x0C | object records (`vector`, 0x34 bytes each: lot-developer.md §17.1) |
| +0x9C | 0x0C | retaining wall types (`vector<uint32>`) |
| +0xA8 | 0x0C | fence types (`vector<uint32>`) |
| +0xB4 | 0x0C | fence segment positions (`vector` of 4 signed bytes) |

## Land-value native progress (`cSC4LandValueSimulator`, 1.1.641 GOG, verified 2026-10-22)

All pointers and vtable entries are 4 bytes, little-endian x86. **L** is the successful
`QueryInterface(0xE7781D8C)` result (`cISC4LandValueSimulator`, vtable `0x00AB1670`);
unknown IID 1 returns this same base. **M** is the separately queried message-target IID
`0x452294AA` result (vtable `0x00AB1660`), and **S** is serializable IID `0xE4FDA3D4`
(vtable `0x00AB1648`). Native base relationships: M = L + 4; S = L + 8.
Offsets in the first table are relative to **L**, never M or S. These are a partial object layout,
not an object allocation size or permission to construct an instance.

| Offset | Width (bytes) | Native field / interpretation |
|---|---|---|
| +0x0C | 1 | initialized flag; constructor clears, Init sets only after registration/subscription, Shutdown clears before removal |
| +0x10 | 4 | native reference count; observation only, ownership must use QI/AddRef/Release |
| +0x38 | 4 | pipelineState, int32 (0 through 9) |
| +0x3C | 4 | pointer to tract ScanningIterator |
| +0x40 | 4 | pointer to slope ScanningIterator |
| +0x4C | 1 | ignoreZoning, zero / nonzero |
| +0x70 | 4 | developer cursor, node pointer; equals header at end |
| +0x8C | 4 | developer-map header pointer (map base is the address L + 0x8C) |

QueryInterface establishes these bases at `0x006BD4DF–0x006BD4F9`, `0x006BD523–0x006BD52B`;
its success paths AddRef the shared object. Message-target slot 0 adjusts by −4
(`0x006BDDB0–0x006BDDB3`). Step receives L: DoMessage adjusts M by −4 at
`0x006BF113` / `0x006BF15D`. State is read at `0x006BE7CD`; iterator allocation/stores
are `0x006BE5CF–0x006BE5E8` and `0x006BE630–0x006BE649`; slope iteration reads +0x40
at `0x006BECD5`. Byte access to ignoreZoning is `0x006BEBED`, `0x006BEC08`.
Cursor/header dword accesses are `0x006BEEB3–0x006BEEC3`, `0x006BEED9–0x006BEEE5`.

### ScanningIterator (0x20 bytes, no vtable)

Each iterator pointer above points directly to this layout. The native allocation requests 0x20 bytes
(`0x006BE5CF`); construction is `0x0060CE90–0x0060CED0`. Unlisted bytes +0x01..+0x03
are not a field available to the observer.

| Offset | Width (bytes) | Native field |
|---|---|---|
| +0x00 | 1 | state: 0 fresh, 1 yielding, 2 exhausted |
| +0x04 | 4 | x, int32 |
| +0x08 | 4 | z, int32 |
| +0x0C | 4 | step, int32 (constructor sets 1) |
| +0x10 | 4 | x0, int32, inclusive lower bound |
| +0x14 | 4 | z0, int32, inclusive lower bound |
| +0x18 | 4 | x1, int32, inclusive upper bound |
| +0x1C | 4 | z1, int32, inclusive upper bound |

Widths and signed bounds are established by construction, reset `0x0060C860–0x0060C888`
and next `0x0060C8D0–0x0060C92A`. x/z in state 1 identify the last yielded tract;
state 0 has not yielded that stored coordinate. Exhaustion can leave z beyond z1.

### Developer-map nodes (known fields only)

Node offsets are relative to the node pointer, not to L or the developer interface. Header is a
sentinel: its +0x08 supplies the first cursor; equality to header means no developer. Never
interpret header payload as a developer. Child links use null for absence. No node allocation size, colour field or padding
contract is asserted here.

| Offset | Width (bytes) | Native field |
|---|---|---|
| +0x04 | 4 | parent node pointer; header +0x04 is root |
| +0x08 | 4 | left child pointer; header +0x08 is first node |
| +0x0C | 4 | right child pointer (ordinary nodes) |
| +0x10 | 4 | key, borrowed `cISC4GrowthDeveloper*`, used as its own interface base |
| +0x14 | 4 | weight, IEEE-754 float32 |

Header/first/cursor: `0x006BEEB3–0x006BEEC0`; payload reads and developer slot 13 call:
`0x006BEEE7–0x006BEEFD`. The loaded weight bits go to the callback context at
`0x006BEEF3`; `0x006BD671` consumes them as float32. Links are used by the cursor successor
at `0x005D17C4–0x005D17FA` (despite its exported constructor label). Map lookup uses header root
and node links/key at `0x006A3179–0x006A3196`. Tuning stores the borrowed developer and weight
through that map at `0x006BE2DC–0x006BE2F9`. Developer slot 13 returns a borrowed
`cISC4SimGrid<int16>*`; getter `0x007C9FB0–0x007C9FB6` is a plain pointer read with no AddRef.
The observer must copy map contents while the city/developer is alive; these facts grant no ownership.

### Message-target forwarding ABI

M's slot 3 (+0x0C) is `DoMessage(cIGZMessage2*)`, target `0x006BF0D0`.
Calling convention: x86 thiscall, ECX = **M**, one 4-byte message-pointer argument,
callee pops 4, bool result in AL (native always 1). The saved native slot must receive M,
not L or a proxy pointer. The separate L reference supplies the progress offsets above.
Type is queried through message slot 4; SimIdle data3 through slot 19 (+0x4C), returning a
32-bit budget in EAX (`0x006BF0D6–0x006BF0DC`, `0x006BF155–0x006BF160`). SimNewDay supplies
constant 5 (`0x006BF111–0x006BF116`). DoMessage return sites include `0x006BF11C–0x006BF11F`,
`0x006BF166–0x006BF169`, `0x006BF177–0x006BF17B`.

These addresses/layouts apply only to the named native build. An observer must validate executable
identity and the expected native interface/slot targets before reading private fields; mismatches
disable capture. The measured main-checkout **working copy** SHA256 is
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
That fingerprint is an observation, not certification of a pristine executable or a shipped detector.
No approved hook installation, per-instance vtable mutation or agent re-registration ABI is implied.
