# Spec: simulation grids (`cISC4SimGrid<T>`) and the shared map utilities

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program (Windows build 1.1.641) and its data files.
Implementers must work from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must
**not** consult decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (streams §2, references §2.7, class store §3, slot order §16,
random cell iterators §12.4), [weather.md](weather.md) §6.7 (random cell iterator). Users: [landvalue.md](landvalue.md),
[pollution.md](pollution.md), [crime.md](crime.md), [flammability.md](flammability.md), [aura.md](aura.md).

## 1. Role

Almost every "field" simulator (land value, pollution, crime, aura, flammability, fire and police coverage,
power, water, flora, the growth developers) stores its per-area values in **simulation grids**: a 2-D array of one
numeric type over the city, at a resolution of one value per *tract* of S × S cells. The grids are GZCOM objects
of their own, saved in their own class records. A small set of free helper functions (`cSC4MapUtilities`, §7)
applies distance-based effects ("auras", "cones") to grid data.

## 2. Grid classes *[compat]*

One template class `cSC4SimGrid<T>`, instantiated for ten element types. All instantiations answer the **same**
interface ID for `cISC4SimGrid` (the IID does not encode T; callers must know the type).

| T | CLSID (stored in the object, returned by `GetGZCLSID`) | grid vtable | serializable vtable |
|---|---|---|---|
| `uint8` | `0x49B9E602` | `0x00A878C0` | `0x00A878A4` |
| `int8` | `0x49B9E603` | `0x00A87950` | `0x00A87934` |
| `uint16` | `0x49B9E604` | `0x00A87D40` | `0x00A87D24` |
| `int16` | `0x49B9E605` | `0x00A879E0` | `0x00A879C4` |
| `uint32` | `0x49B9E606` | `0x00A87DD0` | `0x00A87DB4` |
| `int32` | `0x49B9E607` | `0x00A87A70` | `0x00A87A54` |
| `uint64` | `0x49B9E608` | `0x00A87B00` | `0x00A87AE4` |
| `int64` | `0x49B9E609` | `0x00A87B90` | `0x00A87B74` |
| `float` | `0x49B9E60A` | `0x00A87C20` | `0x00A87C04` |
| `double` | `0x49B9E60B` | `0x00A87CB0` | `0x00A87C94` |

| Interface | IID |
|---|---|
| `cIGZUnknown` | `1` |
| `cISC4SimGrid<T>` | `0xC9BD939A` (all T) |
| `cIGZSerializable` | `0xE4FDA3D4` (a second sub-object of the same object; one shared reference count) |

**Factories.** The game's COM director registers class factories for `0x49B9E601`..`0x49B9E60B`. `0x49B9E601`
("bool") has no class of its own: it creates a **`uint8` grid** (whose CLSID field then says `0x49B9E602`). The
generic ID `0x49B9E600` is **not** registered. The grid constructor takes a CLSID argument but ignores it and
stores its own type's CLSID (callers pass `0x49B9E600`) *[quirk]*.

## 3. Geometry *[compat]*

| Field | Set by | Meaning |
|---|---|---|
| S (tract size, int32) | `SetTractSize` | cells per tract side (nominal) |
| shift (int32) | `SetTractSize` | number of times S can be halved (integer division) while it is > 1: 1→0, 2→1, 3→1, 4→2, 5→2, 8→3; S ≤ 1 → 0 |
| countX, countZ (int32) | `SetTractSize` | city `CellCountX() >> shift`, `CellCountZ() >> shift` |
| widthX, widthZ (float32) | `SetTractSize` | city `CellWidthX() × S`, `CellWidthZ() × S` (metres; 16 × S in SC4) |
| invWidthX, invWidthZ (float32) | `SetTractSize` | 1 / width |
| data | `SetTractSize`, `Read` | countX rows (indexed by x), each of countZ values (indexed by z) |

- **Cell → tract**: `tx = cx >> shift`, `tz = cz >> shift` (arithmetic shift).
- **Position → tract**: `tx = trunc(px × invWidthX)`, `tz = trunc(pz × invWidthZ)` (truncation toward zero).
- **Tract → position**: corner `(tx × widthX, tz × widthZ)`; centre `((tx + 0.5) × widthX, (tz + 0.5) × widthZ)`.
- For S not a power of two, counts use the shift while widths use S, so they disagree *[quirk]*. All shipped
  tract sizes are powers of two.
- The grid reads the city through the game's global city pointer, not through a parameter, so a grid can only
  be sized while a city exists.

## 4. `cISC4SimGrid<T>` slot behaviour *[compat]*

All slots are `__thiscall`. "Pop" is the number of argument bytes the callee removes; it depends on `sizeof(T)`
only for slots 26 and 27. **No method checks bounds** unless stated; out-of-range indices read or write outside the
arrays.

| Slot | Offset | Method | Pop | Return channel | Behaviour |
|---|---|---|---|---|---|
| 0 | +0x00 | `QueryInterface(iid, void**)` | 8 | AL | IIDs `1`, `0xC9BD939A` → the grid; `0xE4FDA3D4` → the serializable sub-object; `AddRef`s; others false |
| 1 | +0x04 | `AddRef()` | 0 | EAX new count | |
| 2 | +0x08 | `Release()` | 0 | EAX | count > 1: decrement, return it. Else destroy (no `Shutdown`), return 0 |
| 3 | +0x0C | `Init()` | 0 | AL = 1 | initFlag := true |
| 4 | +0x10 | `Shutdown()` | 0 | AL = 1 | initFlag := false (data kept) |
| 5 | +0x14 | `GetInstanceID()` | 0 | EAX | |
| 6 | +0x18 | `SetInstanceID(uint32)` | 4 | (EAX, not meaningful) | |
| 7 | +0x1C | `GetCellValue(int32 cx, int32 cz)` | 8 | T channel (below) | `data[cx >> shift][cz >> shift]` |
| 8 | +0x20 | `GetAverageValueInCellRect(int32 x, int32 z, int32 w, int32 h)` | 16 | T channel | §4.1 |
| 9 | +0x24 | `SetTractSize(int32 S)` | 4 | **none meaningful** | §4.2 |
| 10 | +0x28 | `GetTractSize()` | 0 | EAX | S |
| 11 | +0x2C | `GetTractShift()` | 0 | EAX | shift |
| 12 | +0x30 | `GetTractCountX()` | 0 | EAX | |
| 13 | +0x34 | `GetTractCountZ()` | 0 | EAX | |
| 14 | +0x38 | `GetTractWidthX()` | 0 | ST0 | |
| 15 | +0x3C | `GetTractWidthZ()` | 0 | ST0 | |
| 16 | +0x40 | `GetOneOverTractWidthX()` | 0 | ST0 | |
| 17 | +0x44 | `GetOneOverTractWidthZ()` | 0 | ST0 | |
| 18 | +0x48 | `TractIsInBounds(uint32 tx, uint32 tz)` | 8 | EAX 0/1 | unsigned `tx < countX && tz < countZ` (negative → false) |
| 19 | +0x4C | `PositionToTract(float px, float pz, int32& tx, int32& tz)` | 16 | **none meaningful** (EAX = tz) | §3; no bounds check |
| 20 | +0x50 | `TractCornerToPosition(int32 tx, int32 tz, float& px, float& pz)` | 16 | **none meaningful** (EAX = address of px) | §3 |
| 21 | +0x54 | `TractCenterToPosition(int32 tx, int32 tz, float& px, float& pz)` | 16 | **none meaningful** (EAX = address of px) | §3 |
| 22 | +0x58 | `GetTractValue(int32 tx, int32 tz)` | 8 | T channel | `data[tx][tz]` |
| 23 | +0x5C | `GetAverageValueInTractRect(int32 tx, int32 tz, int32 w, int32 h)` | 16 | T channel | §4.1 |
| 24 | +0x60 | `GetGridData()` | 0 | EAX pointer | address of the grid's `vector<vector<T>>` (§6.3) |
| 25 | +0x64 | `GetGridData() const` | 0 | EAX pointer | same body as 24 |
| 26 | +0x68 | `SetTractValue(int32 tx, int32 tz, T v)` | 12, or 16 for 8-byte T | — | `data[tx][tz] := v` |
| 27 | +0x6C | `SetTractValues(T v)` | 4, or 8 for 8-byte T | — | every element := v |
| 28 | +0x70 | (class destructor; not part of the interface) | 4 | — | |

**T channel** (how a `T` is returned): `uint8`/`int8` in AL, `uint16`/`int16` in AX (upper EAX bits undefined),
32-bit types in EAX, 64-bit types in EDX:EAX, `float`/`double` in ST0. The `uint8`/`int8` grids share the same
code for slots 7, 22, 26, 27 (and `uint16`/`int16`, `uint32`/`int32`, `uint64`/`int64` likewise); the averages
are type-specific.

### 4.1 Averages *[compat]*

`GetAverageValueInCellRect(x, z, w, h)` (cell coordinates; the last two parameters are a **width and height**,
not a corner):

1. If `x >> shift = (x + w − 1) >> shift` and `z >> shift = (z + h − 1) >> shift` (the rectangle lies in one
   tract), return that tract's value unchanged.
2. Otherwise sum over every **cell** `(cx, cz)` with `x ≤ cx < x + w`, `z ≤ cz < z + h` (x outer, z inner) the value
   of the tract containing it, as a signed 64-bit integer: unsigned types zero-extended, signed types
   sign-extended, `float`/`double` values **truncated toward zero** to an integer first. Then divide by `w × h`
   (32-bit product) in floating point.
3. Integer grids return the quotient truncated toward zero (for 64-bit grids the full 64-bit result). `float` and
   `double` grids return the quotient itself (not truncated), even though the summed values were truncated
   *[quirk]*.

`GetAverageValueInTractRect(tx, tz, w, h)` is the same over tracts `[tx, tx + w) × [tz, tz + h)` (no one-tract
shortcut). With w or h ≤ 0 the sum is empty and the result is 0 / 0: integer grids then return the C
runtime conversion's NaN result truncated to T: the conversion (`_ftol`, foundations §18.1) yields the 64-bit
`0x8000000000000000`, so 8-, 16- and 32-bit grids get **0** and 64-bit grids `0x8000000000000000`; float grids
NaN *[quirk]*; *[choice]* FreeCity4 returns 0.

### 4.2 `SetTractSize(S)` *[compat]*

If S equals the current S, nothing happens. Otherwise compute shift, counts, widths and inverse widths (§3) from the
city, resize the outer array to countX and every row to countZ: **existing values are kept** where the indices
still exist and new elements are 0 (the grid is not cleared, and old values are not rescaled). The return value
is meaningless; *[choice]* a replacement returns true.

### 4.3 SDK errors *[compat]* (gzcom-dll `cISC4SimGrid.h`)

1. `PositionToTract`, `TractCornerToPosition`, `TractCenterToPosition` and `SetTractSize` are declared `bool`, but
   the original returns nothing meaningful (callers ignore the value). A replacement must return a non-zero AL
   to be safe; code that calls the **original** must ignore the result.
2. `GetAverageValueInCellRect` / `GetAverageValueInTractRect`: the SDK names the parameters
   `nTopLeftX, nTopLeftZ, nBottomRightX, nBottomRightZ`; they are **x, z, width, height**.
3. `GetGridData` is declared `intptr_t`; it returns a pointer to the internal `vector<vector<T>>` in the game's
   STLport layout (§6.3). Both overloads have the same body, so their relative order does not matter.
4. Slot 28 is the class destructor (absent from the SDK); nothing outside the class calls it.

All other slots match the SDK in order, argument size and return type.

## 5. Persistence *[compat]*

A grid is never written inline. Its owner writes a **reference** (foundations §2.7) and the class-object store
writes the grid once into the class record of its CLSID. `GetGZCLSID()` returns the CLSID field. The record layout
(little-endian):

| # | Type | Field |
|---|---|---|
| 1 | u16 | version = 1 |
| 2 | u8 | initFlag (1/0) |
| 3 | u32 | CLSID |
| 4 | u32 | instance ID |
| 5 | s32 | S |
| 6 | s32 | shift |
| 7 | s32 | countX |
| 8 | s32 | countZ |
| 9 | f32 | widthX |
| 10 | f32 | widthZ |
| 11 | f32 | invWidthX |
| 12 | f32 | invWidthZ |
| 13 | T × countX × countZ | values, x outer, z inner, each in T's native little-endian size (`uint8`/`int8` 1 byte, 16-bit 2, 32-bit and `float` 4, 64-bit and `double` 8) |

`Write` always returns true. `Read` returns false (reading nothing more) if the version is not exactly 1;
otherwise it reads every field **as stored** (including the CLSID, which overwrites the object's own) without
recomputing anything from the city, resizes the data to countX × countZ and reads the values, and returns true
regardless of stream errors *[quirk]*. *[choice]* FreeCity4 returns false on a stream error.

## 6. Lifetime, creation and replaceability

### 6.1 Reference counting

A new grid has count 0. Owners `AddRef` it. `Release` destroys it when the count is 1 or less (§4). Single-pointer
getters in §6.2.1 return borrowed pointers **without** `AddRef`. Residential's three-output getters are an
exception: they `AddRef` every output; the caller releases each one (§6.2.2).

### 6.2 Creation by the original

The original simulators construct their grids **directly** (`new` + constructor + `AddRef` + `Init` +
`SetTractSize` + `SetInstanceID`), not through GZCOM. On load, the class-object store creates grids through the
COM director's factories (§2) and the owner receives them through `ReadGZSerializable` with IID `0xC9BD939A`.
The map view also constructs `uint16`/`int16` grids of its own.

### 6.2.1 Owner/getter inventory for city record shadows (SG3) *[compat]*

**Scope [choice, coordinator contract]:** at PostCityInit (after the owners have handled city initialization)
and each monthly observation, sample the simulation-owned grids exposed by the getters below. This is an
exhaustive inventory **for this scope**, not an enumeration of every live SimGrid or every saved grid. There
is no global live-grid enumerator required by this contract. A class-store record walk or references observed
by a stream adapter do not establish this inventory: original owners also construct grids directly.

Obtain the current owners through the corresponding city getters (`GetLandValueSimulator`,
`GetPollutionSimulator`, `GetPowerSimulator`, `GetPlumbingSimulator`, `GetCrimeSimulator`,
`GetFireProtectionSimulator`, `GetPoliceSimulator`, `GetFlammabilitySimulator`, `GetAuraSimulator`,
`GetTrafficSimulator`, `GetBuildingDevelopmentSimulator`, `GetCommercialSimulator`, `GetResidentialSimulator`).
Slots below are zero-based slots of each owner's primary interface, including IUnknown. Each no-argument
getter returns `cISC4SimGrid<element>*` in EAX; its stack pop is zero. Selectors are 32-bit stack arguments
(pop 4). These are interface calls: identical executable addresses can serve unrelated methods in other
interfaces, and their exported function names alone do not identify the owner.

Let `C = (city.CellCountX(), city.CellCountZ())`, and `D(S) = (Cx >> shift(S), Cz >> shift(S))` from §3.
The dimensions column gives **new-city geometry**; the actual grid's `GetTractCountX/Z`, `GetTractSize`,
`GetTractShift`, widths and inverse widths are authoritative, especially after load. Do not resize an observed
grid to the table's default. The parenthesized counts are for a 256 × 256-cell city. A loaded grid's §5 record
can preserve geometry independently of current tuning. All entries cover the entire city, indexed `[x][z]`.

| Owner | Getter (slot; selector if any) | Element | Tract size S / dimensions | Instance ID or discriminator |
|---|---|---|---|---|
| land value | `GetIntrinsicLandValueMap` (7) | uint8 | tuning `0x47E2C200`, shipped 2 / D(S) (128 × 128) | `0x89D5BB39` |
| land value | `GetTotalLandValueMap` (8) | uint8 | same | `0x89D5BB51` |
| land value | `GetSlopeMap` (9) | float32 | 8 / D(8) (32 × 32) | `0x49D5BB6F` |
| pollution | `GetPollutionGrid` (5; 0 air, 1 water) | int16 | 2 / D(2) (128 × 128), each | `0x49D5BC40`, `0x49D5BC41` |
| pollution | `GetPollutionGrid` (5; 2 garbage) | int16 | 1 / C (256 × 256) | `0x49D5BC42`; preserve all bits, including landfill bit 15 |
| pollution | `GetPollutionGrid` (5; 3 radiation) | int16 | 2 / D(2) (128 × 128) | **source** `0x89D5BBF3`, not radiation-value `0x49D5BC43` |
| power | `GetPoweredGrid` (35 const / 36 mutable) | uint8 | 1 / C (256 × 256) | `0x49D5BC86`; both slots alias one grid |
| plumbing | `GetWateredGrid` (7) | uint8 | 1 / C (256 × 256) | published role; ID alternates `0x49D5BBA1` / `0x49D5BB8C` |
| crime | `GetCrimeGrid` (5) | uint8 | tuning `0x47E2C200`, default 2 / D(S) (128 × 128) | `0x89D5B895` |
| fire protection | `GetFireProtectionGrid` (5) | uint8 | 2 / D(2) (128 × 128) | adjusted `0x49D5B8F9` |
| police | `GetPolicePowerGrid` (5) | uint8 | 2 / D(2) (128 × 128) | adjusted `0x49D5BBB9` |
| flammability | `GetFlammabilityGrid` (5) | uint8 | 1 / C (256 × 256) | adjusted `0x49D5B953` |
| aura | `GetAuraGrid` (12) | int8 | 4 / D(4) (64 × 64) | `0x69D5B600` |
| aura | `GetParkMap` (14) | int16 | 4 / D(4) (64 × 64) | `0x49D5B602` |
| aura | `GetLandmarkMap` (15) | int16 | 4 / D(4) (64 × 64) | `0x8A674CAF` |
| traffic | `GetAirPollutingTrafficMap` (8) | uint8 | 1 / C (256 × 256) | `0x89D5C3A2` |
| traffic | `GetCommercialTrafficMap` (9) | uint8 | 4 / D(4) (64 × 64) | `0xCABD2925` |
| traffic | `GetCongestionMap` (10) | uint8 | 1 / C (256 × 256) | `0x69D5C402` |
| traffic | `GetTripLengthMap` (13) | uint8 | 1 / C (256 × 256) | `0x69D5C3BF` |
| building development (BDS) | `GetBuildingMap` (41) | uint8 | 1 / C (256 × 256) | `0x49D5B678` |
| commercial | `GetProximityMap` (5; `0x3320`, `0x3330`) | uint8 | 1 / C (256 × 256), each | `0x89D5B788`, `0x89D5B6EF` |
| residential | `GetProximityMap` (5; wealth 1, 2, 3) | uint8 | 1 / C (256 × 256), each | `0x49D5BC96`, `0x49D5BCA6`, `0x49D5BCB7` |
| residential | `GetPopulationGrids` (25; three outputs) | uint16 | P / D(P), shipped 4 (64 × 64), each | `0x69D5C2A0` through `0x69D5C2A2` |
| residential | `GetEQGrids` (23; three outputs) | float32 | E / D(E), shipped 4 (64 × 64), each | `0x69D5C2D0` through `0x69D5C2D2` |
| residential | `GetHQGrids` (24; three outputs) | float32 | E / D(E), shipped 4 (64 × 64), each | `0x69D5C2E0` through `0x69D5C2E2` |
| residential | `GetSchoolAverageGradeMap` (39) | float32 | P / D(P), shipped 4 (64 × 64) | `0x69D5C324` |
| residential | `GetHospitalAverageGradeMap` (40) | float32 | P / D(P), shipped 4 (64 × 64) | `0x69D5C330` |
| residential | `GetAverageAgeMap` (41) | uint8 | P / D(P), shipped 4 (64 × 64) | `0x69D5C303` |
| each growth developer | `GetDesirabilityMap` (13) | int16 | developer tuning `0x87CD6310`, shipped 4 / D(S) (64 × 64) | developer type + `0x02000000` |
| each growth developer | `GetCapacityMap` (14) | uint16 | same | type + `0x05000000` |
| each growth developer | `GetPopulationMap` (15) | **int16** | same | type + `0x03000000` |
| each growth developer | `GetMaxCapacityMap` (16) | uint16 | same | type + `0x06000000` |

Residential P and E are taken from the R$ developer population grid's tract size on a new city; on load they
come from the residential record. They are not constants to impose on restored grids (residential §2).
Growth developer population is signed int16; residential population is unsigned uint16. They are different
grids even if their current values and dimensions happen to match.

**Developer discovery:** BDS slot 34 `GetDevelopers()` returns a borrowed pointer in EAX to its internal
`std::map<uint32, cISC4GrowthDeveloper*>` (pop 0; the STLport map layout and allocator are documented in
[building-development.md](building-development.md) §9 and [allocators.md](../abi/allocators.md)). Read every
entry without modifying the map, and apply the four growth-getter rows to each current developer. Both map
and values are borrowed; enumeration itself adds no references. BDS slot 32
`GetDeveloper(uint32 developerType)` is a borrowed lookup (pop 4, null if absent), not an enumerator. Shipped
keys are `0x1010`, `0x1020`, `0x1030`, `0x3110`, `0x3120`, `0x3130`, `0x3320`, `0x3330`, `0x4100`,
`0x4200`, `0x4300`, `0x4400`. Use map keys or developer slot 6 `GetSubType()` for identity; slot 5 `GetType()`
is the target zone (1/2/3), not the developer key. Do not probe all integers or assume exactly twelve entries
when exemplars are modified. In the shipped set the inventory has **38 non-growth roles + 4 × 12 = 86 roles**.

**Exclusions:** the map view's grids; all newly created return/scratch grids (including residential
`GetAverageEQGrid`/`GetAverageHQGrid` slots 21/22); aura's explicitly transient grid (`GetTransientAuraGrid`,
13), even though it is saved; owner-private grids with no getter (crime effect, unadjusted protection, raw
flammability, pollution's other sources and unused radiation value, aura base/building/raw-park/department
grids, growth work/zoned-count grids, traffic's other grids, plumbing work). The weather moisture map and
power consumption map are raw vector objects, not `cISC4SimGrid`; no §5 record applies to them. Flora owns
no SimGrid. Landfill, industrial and civic simulators add no SimGrid getter to this inventory. These exclusions
mean a successful sample establishes the owner-getter scope only, not all persisted city grids.

**Evidence:** geometry uses signed shifts at `0x00461000`–`0x00461019` (S/shift at `0x00460FD3`–`0x00460FEB`);
the per-owner creation sizes and IDs are specified in landvalue §2, pollution §2, power §2, plumbing §2,
crime §2, protection §2, flammability §2, aura §2, traffic-simulator §1, building-development §1.3,
commercial-industrial §2.1, residential §2 and growth-developer §1.2/§4.8. Getter instruction checks:
land value `0x006BDD80`, `0x0060FB80`, `0x006AAC50`; aura `0x0068D1B0`, `0x0040D390`, `0x0068D1D0`;
crime `0x006A0050`; flammability `0x0040D390`; fire/police/power `0x006EFA50`; traffic `0x006AAC50`,
`0x0080BC00`, `0x00698CF0`, `0x00820250`; BDS map `0x00698CF0`, developer-map address `0x00698CD0`;
growth `0x007C9FB0`, `0x004E82A0`, `0x007C1910`, `0x007E56F0`; residential scalar-grid getters
`0x00809240`, `0x00407AC0`, `0x006AAEB0`. These bodies return pointers without reference-count calls.
Commercial selectors are checked at `0x0069EAD4`–`0x0069EAEE`; residential selectors at
`0x006F6F94`–`0x006F6FB1`. Pollution `0x006E3364`–`0x006E3373` selects radiation **source** for type 3;
there is no selector bounds check, so only 0..3 belong to this contract.

### 6.2.2 Sampling and pointer lifetime (SG3) *[compat]*

All single-pointer inventory getters are borrowed: do not release their results. The aliases
`power.GetSupplyValueMapPtr(grid*&)` (slot 20, pop 4, `0x006EAD60`) and
`plumbing.GetSupplyValueMapPtr(grid*&)` (slot 6, pop 4, `0x006DA450`) write the same borrowed grid object
pointer through the output argument, **not** a row-vector address; do not count an alias as another role.
Getters on residential slots 23/24/25 instead each take **three output pointer references** (pop 12), in
wealth order R$, R$$, R$$$. They write the owner-held grids and `AddRef` each; release all nine acquired
references after the sample. Their EAX is the third AddRef result, not a success flag
(`0x006F72E3`–`0x006F72FA`, `0x006F7326`–`0x006F733D`, `0x006F735D`–`0x006F7374`).

Acquire each role again at **each** observation on the simulation thread while the owners are alive, after
initialization and before shutdown. Read the grid and finish its capture within that observation; do not retain
borrowed row pointers, the developer map, or a previous role-to-object association across simulation work,
loads, resizing, owner replacement or city teardown. AddRef can keep an object alive but does not guarantee
it remains the current object for the role. A replaced owner is sampled through its public getters under the
same contract; no original-class field offsets are needed to discover this scope.

Plumbing is a deliberate identity quirk: when a pass publishes its result it **swaps pointers**, not the
contents of a fixed public object. Both creation IDs can therefore be the current published grid; select by
getter, never by an ID filter or a pointer retained at PostCityInit. The other getter results likewise describe
the current role, rather than a promise of permanent pointer identity. Owners release grids at shutdown;
neither a city-global cache nor a previous sample extends a borrowed pointer's lifetime.

For comparison, identify a role by owner + getter + selector (developer key/wealth where applicable), then
capture that grid's own §5 metadata and all `countX × countZ` elements in x-major order. Capture **all native
bits** (signed values, garbage flags, float32 bits), not a rendered overlay or a converted byte map. Record
instance IDs as metadata, not as an enumeration key: unrelated grids can share an ID. Do not call `Init`,
`SetTractSize`, or a writing slot on an observed grid. Serializing or copying it is observation, not a request
to advance its owner's update pipeline. Monthly fields may contain partially refreshed or previous-cycle
values; no forced refresh is part of this inventory.

Reporting policy *[choice]*: include an outcome for every expected role, with explicit null owner, null grid,
unavailable developer-map access, geometry/type mismatch or capture failure; do not silently reduce the
denominator and call the remaining subset complete. Keep absent/unsupported roles distinct from exact
comparisons. Re-enumerate developers each observation, so additions/removals change the expected set then.

### 6.3 Raw memory access by original code *[compat]*

Original code outside the grid class reads and writes grid contents **directly** through the pointer that
`GetGridData` returns, assuming the game's STLport layout:

- `vector<X>` = three pointers `{begin, end, end_of_storage}` (12 bytes); the grid data is a
  `vector<vector<T>>` whose element i is row x = i; row elements are contiguous T values indexed by z.
- Users: the map view (every grid layer), the growth developers (land value grids), the shared helpers of §7
  (which receive `GetGridData` results), and the simulators themselves.

Consequences:

1. Any grid object that original code can obtain (from a simulator's getter, or created by the class store on
   load) must expose a compatible `vector<vector<T>>` through slots 24/25.
2. **FreeCity4 must not register the SimGrid class IDs** in its DLL unless its grids are layout-compatible: the
   class store would then create our grids for the original simulators, which write grid memory directly.
3. Replacement simulators therefore use the **game's own** grid class (§6.4, normative), and access values
   through the interface or through `GetGridData` as described here. Old saves then load unchanged.

### 6.4 Grid ownership rule *[compat]* (normative)

This rule binds every FreeCity4 replacement of a subsystem that owns, creates or hands out `cISC4SimGrid<T>`
objects (land value, pollution, crime, aura, flammability, power, plumbing, fire and police protection, flora,
growth developers, and any later one).

1. **Create grids only through GZCOM, using the game's own grid classes.** A replacement must obtain every grid
   it owns with `GZCOM()->GetClassObject(clsid, 0xC9BD939A, &p)`, where `clsid` is the class ID of the element type
   in §2 (`0x49B9E602`..`0x49B9E60B`; never `0x49B9E600`, which is not registered). The returned object already
   holds one reference for the caller *[inferred: the factory returns a count-0 object and the GZCOM
   `QueryInterface` adds one]*. Then call `Init()`, `SetTractSize(S)` and `SetInstanceID(id)` in that order, with
   the S and instance ID the subsystem spec gives. A fresh grid has S = 0 and no data until `SetTractSize`.
2. **Never register the SimGrid class IDs.** The FreeCity4 DLL must not register a factory for any of
   `0x49B9E600`..`0x49B9E60B`. GZCOM gives DLL registrations precedence, so a registration would make the class
   store create FreeCity4 objects for **original** simulators, which read and write grid memory directly (§6.3).
3. **Never implement `cISC4SimGrid` in FreeCity4 code that original code can reach.** Any grid pointer returned
   from a replaced interface, or saved by reference, must be a game grid created under rule 1. The FreeCity4 core
   may keep private grids of its own type (for example a Rust array) only if it copies their values into the
   game grids before any original code can read them, and it must treat the game grid as the persistent copy
   (§5 is the save format of the game grid; FreeCity4 writes only references to it).
4. **Save and load through references.** A replacement writes its grids with `WriteGZSerializable` (foundations
   §2.7) and reads them with `ReadGZSerializable(…, 0xC9BD939A, …)`; the class store then saves and restores the
   grid objects with the game's own `Write`/`Read` (§5). A grid restored this way keeps its saved geometry; the
   subsystem spec states whether `Init` resizes it.
5. **Access.** Values may be read and written through the interface (slots 7, 22, 26, 27) or through the
   `GetGridData` pointer with the §6.3 layout. When writing through the pointer, never change the vectors'
   sizes or reallocate them (only `SetTractSize` and `Read` may); the game's allocator owns that memory.
6. **Lifetime.** Owners keep grids alive until `Shutdown` and release their owned references there. A getter
   can expose a different owned grid after a publication swap (§6.2.2). Single-pointer getters borrow;
   residential three-output getters add caller-owned references that must also be released (§6.2.2).

## 7. Shared map utilities (`cSC4MapUtilities`) *[compat]*

Free functions (`__cdecl`, caller pops). They operate on **grid data** (the `GetGridData` result) plus explicit
tract counts, in **tract** coordinates; converting cell or world coordinates is the caller's job (each simulator
spec states how).

### 7.1 `AddRadiusEffect` (linear cone)

Arguments: grid data, countX, countZ, centre tract `(cx, cz)`, radius `r` (float, in tracts), centre value `c`
(float), outer value `o` (float). Variants exist for `uint8` (clamp 0..255; used by crime), `int8` (clamp
−128..127; used by aura transient effects and occupant changes) and `int16` (clamp −32768..32767; used by aura,
growth developers, pollution, protection).

```
if not (r > 0): no change                      (also for NaN)
R  := roundHalfEven(r)                          (2.5 → 2, 3.5 → 4)
k  := float32((c − o) / r)
for every tract (x, z) with max(0, cx − R) ≤ x ≤ min(countX − 1, cx + R), same for z:
    dx := x − cx ;  dz := z − cz
    d  := R ≤ 15 ? TABLE(dx, dz) : sqrt(dx² + dz²)        (see below)
    if d < r:                                              (compared with the unrounded r)
        delta := roundHalfEven(float32(c − d × k))         (round to nearest, ties to even)
        v[x][z] := clamp(v[x][z] + delta, typeMin, typeMax)
```

- `TABLE(dx, dz)` *[compat]*: for |dx|, |dz| ≤ 15 the distance is `float32(sqrt(dx² + dz²))` **rounded half-up to
  three decimals** (1.414 for (1,1), 2.236 for (2,1), 2.828 for (2,2)). Exact distances are used only when R ≥ 16.
- `c − d × k` is two x87 operations at 24-bit precision (foundations §18): `d × k` is rounded to float32, then
  `c −` that is rounded to float32, before rounding to an integer (corrected: not one rounding in extended
  precision).
- The clamp uses the arithmetic sum `v + delta` (no wrap).
- *[compat]* Both `roundHalfEven` calls are the add-and-subtract `1.5 × 2^23` idiom (7 sites in each of the
  three variants, `0x005E3D80`, `0x005E4130`, `0x005E4500`). It is exact round-half-even only for |x| ≤ 2^22;
  beyond that it returns the result described for the same idiom in crime §6. Radii and effect values in the
  shipped data stay far below that bound.
- Negative effects use a negative `c` (callers remove an effect by applying the opposite values).

### 7.2 `AddDistanceEffect` (Manhattan falloff from a mask)

Arguments: target grid data (`uint8`), countX, countZ, mask grid data (`uint8`), mask value `m` (u8), radius
(float), near value, far value (floats). Target and mask may be the same grid. Only user: land value (§7.1 there).

```
n := trunc(radius)
dist(x, z) := Manhattan (4-neighbour) distance from (x, z) to the nearest tract whose mask value = m,
              if that distance ≤ n; otherwise "far" (255)          (computed before the target is modified)
k := float32((near − far) / radius)
for every tract with 0 < dist < radius:
    target := clamp( trunc(target + near − dist × k), 0, 255 )       (truncation toward zero)
```

Tracts at distance 0 (the mask tracts) and at distance ≥ radius are unchanged. The distance field is built by n
rounds of 4-neighbour expansion over a byte grid, clamped at the grid edges, so it is exact Manhattan distance up to
n; with n ≥ 255 the "far" marker collides with real distances *[quirk]*.

### 7.3 `ProcessTwoGridsWithCallback<A, B>`

Arguments: grid A (`cISC4SimGrid<A>*`, written), grid B (`cISC4SimGrid<B>*`, read), callback `f(A* cell, B value,
void* context)`, context. It reads both grids' shift and counts (slots 11–13) and their data through `GetGridData`
(slots 24/25), then visits **every tract of A exactly once**:

| Case | Visit |
|---|---|
| shiftA = shiftB | for x < countX(A), z < countZ(A): `f(&A[x][z], B[x][z])` (x outer) |
| shiftA > shiftB (A coarser, d = shiftA − shiftB) | for each A tract: sum of the 2^d × 2^d covered B values (signed 32-bit), then `(sum >> d) >> d` (arithmetic shifts, i.e. floor of the mean); `f(&A[x][z], thatMean)` |
| shiftA < shiftB (A finer, d = shiftB − shiftA) | for each B tract (x < countX(B), z < countZ(B)): `f(&A[x'][z'], B[x][z])` for every A tract (x', z') it covers |

No bounds checks: the grids must cover the same city. If they do not, the cases differ *[compat]*: the
equal-size and "A coarser" cases loop over A and read B out of range when B is smaller; the "A finer" case loops
over **B**, so A tracts outside B's area are simply not visited (and a B larger than A writes outside A). Land
value §7.4 states what a replacement does in each case. The instantiation `<uint8, int16>` is used by land value
(desirability) and aura. The aura and the growth developers use further instantiations (float, uint8, int8, int16
sources) and **residential-only** variants that call the callback only for tracts whose lot (lot manager
`GetLot(x, z, false)`) has zone type 1..3; see aura.md.

### 7.4 Cell iterators

**Scanning iterator** (rectangle scan; land value, demand, growth/lot/zone developers, plumbing, network lots):

| Field | Default |
|---|---|
| rectangle `[x0, x1] × [z0, z1]` (inclusive) | full int32 range |
| step | 1 |
| current `(x, z)`, state ∈ {fresh, running, done} | (0, 0), fresh |

- `Reset(x, z)`: current := (x, z); state := fresh, or done if (x, z) is outside the rectangle.
- `GetNext(&x, &z)`: done → false. Fresh → state := running, return current. Running → x += step; if x > x1:
  x := x0 and z += step; if z > z1: state := done, return false. Return current. A finished scan therefore rests
  at `(x0, z1 + step)` with state done (`0x0060C8D0`); savers that write the iterator (plumbing §9) store that
  position *[compat]*.
- Order: x inner, z outer. Owners set the rectangle to a grid's tract range `[0, countX−1] × [0, countZ−1]`
  (an empty range is widened to one tract).

**Random cell iterator** (prime-step or table mode): specified in weather.md §6.7 and foundations §12.4. Facts
needed by grid simulators *[compat]*:
- A never-reset iterator has count 0: its first `GetNext` returns false.
- Its save reader (`0x005DB280`, weather.md §10) misbehaves on a stream error: the table is resized to the stream
  object's address *[quirk]*; replacements fail the `Read` instead *[choice]*.
- `Reset` in prime-step mode makes exactly **one** `rand()` call; in table mode it refills the table with the
  rectangle's **absolute** packed coordinates `(x mod 256) << 8 | (z mod 256)` in natural order (x outer, z inner)
  at every reset, and `random_shuffle`s it when asked (n − 1 `rand()` calls). A table-mode reset leaves the
  prime-step `current` and `step` unchanged. Details: weather.md §6.7.

**Spiral iterator** (`SpiralIterator`, `0x0043FE10` / `0x00440850` / `0x00440420` / `0x0043FE80`): visits the
cells of a clipped box ring by ring around a centre. Cells outside the box are skipped one at a time (never
whole rings), the ring counter equals the Chebyshev distance of every visited cell, and the walk ends once the
spiral has passed all four box edges. Specified with its plumbing use in plumbing §7.2 (`NearestWater`).

### 7.5 Other utilities (not specified here)

`FloodFill` (seaport) and `FloodFillDistance` (terrain water proximity) are not used by the specified simulators
yet; their users' specs will describe them. The residential simulator's helpers are specified where they are used:

- `GetTractsInRadius` (`0x005E5200`, cdecl, 7 arguments; unsigned upper clamp, x-major order, tract centres in
  `float32`) and `AddRadiusEffectWithCurves` (`0x005E48F0`, cdecl, 13 arguments; signed clamp, distance fraction
  0..1 as the curve input, reads the population grid raw, *[quirk]* skips the z advance on tracts without
  population): residential-education-health.md, "Coverage maps" and "EQ and HQ boost".
- The L1 distance transform used for the proximity maps (`0x005E4FA0` seed from a building-key map, `0x005E4E80`
  one forward/backward row-then-column pass, `0x005E4D60` repeat until unchanged, `0x005E5130` seed plus full
  transform): residential.md, "Proximity maps". *[quirk]* The seed stores every value in element 0 of its row, so
  only the z = 0 edge is seeded (RG23). The commercial simulator's two CO proximity maps use `0x005E5130` too
  (`DoIdle`, `0x0069ED30`), seeding the maps themselves, so the same defect applies there.

## 8. Original quirks (reproduced)

CLSID argument of the constructor ignored, `0x49B9E601` creates a `uint8` grid (§2); counts use the shift while
widths use S (§3); averages of float grids sum truncated values but return a fractional quotient (§4.1); `Read`
ignores stream errors and trusts stored geometry (§5); `SetTractSize` keeps old values in place (§4.2);
`AddRadiusEffect` uses a 3-decimal distance table for R ≤ 15 and round-half-even (§7.1).

## 9. Open questions

1. *(resolved)* x87 precision control during simulation is 24-bit (foundations §18); §7.1 and §7.2 are chains of
   `float32` operations.
2. Whether any original caller ever uses the result of `PositionToTract` (none found).

## 10. Conformance tests (must exist in `engine/conformance`)

1. Geometry: CellCountX = CellCountZ = 256, cell width 16. S = 2 → shift 1, counts 128, width 32, inverse 0.03125;
   S = 8 → shift 3, counts 32, width 128; S = 3 → shift 1, counts 128, width 48 (quirk); S = 1 or 0 → shift 0,
   counts 256.
2. `PositionToTract(100.0, −10.0)` with S = 2 → (3, 0) (truncation toward zero, −0.3125 → 0).
   `TractCenterToPosition(3, 4)` with S = 2 → (112.0, 144.0); corner → (96.0, 128.0).
3. `TractIsInBounds(−1, 0)` → false; `(127, 127)` with counts 128 → true; `(128, 0)` → false.
4. `GetCellValue(5, 7)` with shift 1 reads `data[2][3]`.
5. Averages, `uint8`, shift 1, tracts (0,0) = 10, (1,0) = 20, (0,1) = 30, (1,1) = 41:
   `GetAverageValueInCellRect(0, 0, 2, 2)` → 10 (one tract); `(0, 0, 4, 4)` → (4·10 + 4·20 + 4·30 + 4·41)/16 =
   25.25 → 25; `GetAverageValueInTractRect(0, 0, 2, 2)` → 101/4 → 25.
6. Float grid, shift 0, cells 1.9 and 2.9: `GetAverageValueInTractRect(0, 0, 2, 1)` → (1 + 2)/2 = 1.5 (fractional
   result of truncated values).
7. `int8` grid value −3: `GetAverageValueInTractRect` over 2 tracts of −3 and 0 → −1.5 → −1 (truncation toward
   zero). `uint8` 255 averaged with 0 → 127.
8. `SetTractSize(2)` then `SetTractValue(1, 1, 7)`, then `SetTractSize(4)`: counts 64, value at (1, 1) still 7.
9. Persistence: `uint8` grid, init, instance `0x89D5BB39`, S = 2, 256-cell city, all values 0 →
   `01 00 | 01 | 02 E6 B9 49 | 39 BB D5 89 | 02 00 00 00 | 01 00 00 00 | 80 00 00 00 | 80 00 00 00 |
   00 00 00 42 | 00 00 00 42 | 00 00 00 3D | 00 00 00 3D` followed by 16384 zero bytes.
10. `Read` with version 2 → false. `Read` of a record whose CLSID field is `0x49B9E603` into a `uint8` grid object →
    `GetGZCLSID()` returns `0x49B9E603` afterwards.
11. `AddRadiusEffect` (`int16`, all 0) at centre (10, 10), r = 3.0, c = 100, o = 0: k = 33.333332;
    (10,10) → 100, (11,10) → 67, (11,11) → 53, (12,10) → 33, (12,11) → 25, (12,12) → 6, (13,10) → 0 (d = 3 is not
    < 3), (13,11) and beyond unchanged.
12. Same with r = 2.5: R = 2; k = 40; (10,10) → 100, (11,10) → 60, (11,11) → 43, (12,10) → 20, (12,11) → 11,
    (12,12) → 0 (2.828 ≥ 2.5).
13. Round half even: r = 1.0, c = 0.5, o = 0.5 → centre delta round(0.5) = 0; c = o = 1.5 → 2; c = o = 2.5 → 2.
14. Clamping: `uint8` cell 250 + delta 10 → 255; `int8` cell −120 + delta −20 → −128; `int16` −32760 − 100 →
    −32768. Edge: centre (0, 0), r = 3 touches only x, z ∈ [0, 3].
15. `AddDistanceEffect` on a 7 × 1 `uint8` grid: mask = target, mask value 0, target = [0, 10, 10, 10, 10, 10,
    10], radius 3, near 30, far 0 → k = 10 → [0, 30, 20, 10, 10, 10, 10].
16. `AddDistanceEffect`, land value shipped values (radius 10, near 50, far 0), target base 1: distances 1..9 →
    46, 41, 36, 31, 26, 21, 16, 11, 6; distance 10 → 1.
17. `ProcessTwoGridsWithCallback<uint8, int16>` with A shift 1 (2 × 2 tracts) and B shift 0 (4 × 4 cells),
    B block for A(0,0) = {1, 2, 3, −7} → sum −1 → (−1 >> 1) >> 1 = −1 passed to the callback; block {1, 1, 1, 2}
    → 5 → 1.
18. Scanning iterator over [0, 2] × [0, 1] after `Reset(0, 0)`: (0,0), (1,0), (2,0), (0,1), (1,1), (2,1), then false.
    `Reset(5, 0)` → first `GetNext` false.

Owner-inventory conformance vectors (SG3; requirements for a future shadow harness, not claims of executed tests):

19. **SG3-inventory-default:** initialized 256 × 256-cell shipped city, twelve BDS developer keys listed in
    §6.2.1, all owner getters present → 86 expected roles: 38 non-growth, 48 growth. Calling both power
    overloads and its supply-map alias still contributes one role; plumbing's alias likewise adds none.
    Aura transient, residential synthesized averages and private grids contribute zero expected roles.
20. **SG3-geometry:** 64 × 64-cell new city, shipped tuning → power/plumbing/flammability and proximity
    maps 64 × 64; crime/land value intrinsic/total/pollution air/water/radiation/protection 32 × 32;
    aura/traffic commercial/residential population-EQ-HQ-grade-age/growth 16 × 16; slope 8 × 8.
    A restored intrinsic grid with S = 4 and counts 32 × 32 in a 128-cell city is captured as 32 × 32,
    not resized to the shipped S = 2 (64 × 64). Metadata differences remain visible.
21. **SG3-selectors-types:** pollution source radiation at (0,0) = −17, unused radiation-value grid = 123;
    pollution selector 3 captures −17. Garbage raw `0x8001` captures bytes `01 80`, not amount 1 alone.
    Growth population raw `0xFFFF` is int16 −1, residential population raw `0xFFFF` is uint16 65535;
    their two-byte records agree at that position but the role/type identities remain different.
22. **SG3-reference-balance:** initial reference counts q for each EQ/HQ/population grid; acquire the three
    triples → each count q + 1; capture and release each output once → each count q. A borrowed proximity,
    grade, age, power or growth getter leaves its count unchanged. No synthesized average getter is invoked.
23. **SG3-publication:** plumbing at sample A returns object A, ID `0x49D5BBA1`, value (0,0) = 1; after a
    completed pass sample B returns object B, ID `0x49D5BB8C`, value (0,0) = 0. Expected: one published role
    in each sample, with the second object's metadata/value. Capturing object A twice fails this vector.
24. **SG3-discovery-change:** developer map adds key K after the first sample → next expected set adds K's
    four roles; removing K removes those four next time. A null crime grid in an otherwise complete shipped
    inventory → 86 expected roles, at most 85 captured and one explicit failure, never 85/85 complete.
    A new owner returned by the city is queried again; a stale pointer from the previous owner is not sampled.
