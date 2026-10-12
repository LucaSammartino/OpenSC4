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

## Building profile header (`cISC4BuildingOccupant::BuildingProfile`)

`cISC4BuildingOccupant::GetBuildingProfile` (compiled slot 11, `+0x2C`, body `0x00459250`) returns a reference to
the building **type's** shared profile. It forwards to `cISC4BuildingDevelopmentSimulator` compiled slot 39
(`+0x9C`, `0x0069CA60`) with the building type. That call inserts a zero profile for an unknown type
(allocators.md). Read only; never free or modify it.

| Offset | Type | Field (SDK name) | Read by |
|---|---|---|---|
| +0x00 | uint32 | `purpose` (`PurposeType`, 0..9) | `cSC4BudgetSimulator::ComputeTax` `0x00691F04` (table row) |
| +0x04 | uint32 | `wealth` (`WealthType`, 0..3) | `ComputeTax` `0x00691FC9` (waning-wealth index) |

The SDK struct has these two fields at these offsets (both enums are `uint32_t`). Its later fields were not
checked here.

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
