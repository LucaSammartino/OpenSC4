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
shortcut). With w or h ≤ 0 the sum is empty and the result is 0 / 0: integer grids then return the integer
"indefinite" value truncated to T (`0x80000000` for 32-bit grids, hence 0 for 8- and 16-bit grids), float grids
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

A new grid has count 0. Owners `AddRef` it. `Release` destroys it when the count is 1 or less (§4). Getters on the
simulators return their grid pointers **without** `AddRef`.

### 6.2 Creation by the original

The original simulators construct their grids **directly** (`new` + constructor + `AddRef` + `Init` +
`SetTractSize` + `SetInstanceID`), not through GZCOM. On load, the class-object store creates grids through the
COM director's factories (§2) and the owner receives them through `ReadGZSerializable` with IID `0xC9BD939A`.
The map view also constructs `uint16`/`int16` grids of its own.

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
6. **Lifetime.** Getters hand out the grid without `AddRef` (as the original does), so the grid must stay alive
   from `Init` until the owner's `Shutdown`, and the owner releases each grid exactly once in `Shutdown`.

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
- `c − d × k` is evaluated in extended precision and rounded once to float32 before rounding to an integer.
- The clamp uses the arithmetic sum `v + delta` (no wrap).
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

No bounds checks: the grids must cover the same city. The instantiation `<uint8, int16>` is used by land value
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
  x := x0 and z += step; if z > z1: state := done, return false. Return current.
- Order: x inner, z outer. Owners set the rectangle to a grid's tract range `[0, countX−1] × [0, countZ−1]`
  (an empty range is widened to one tract).

**Random cell iterator** (prime-step or table mode): specified in weather.md §6.7 and foundations §12.4. Facts
needed by grid simulators *[compat]*:
- A never-reset iterator has count 0: its first `GetNext` returns false.
- `Reset` in prime-step mode makes exactly **one** `rand()` call; in table mode it refills `0 … n−1` in order
  when the size changed and `random_shuffle`s it (n − 1 `rand()` calls).

### 7.5 Other utilities (not specified here)

`FloodFill` (seaport), `FloodFillDistance` (terrain water proximity), `GetTractsInRadius` (residential) and
`AddRadiusEffectWithCurves` (residential occupant grade) are not used by the M3 grid simulators; their users'
specs will describe them.

## 8. Original quirks (reproduced)

CLSID argument of the constructor ignored, `0x49B9E601` creates a `uint8` grid (§2); counts use the shift while
widths use S (§3); averages of float grids sum truncated values but return a fractional quotient (§4.1); `Read`
ignores stream errors and trusts stored geometry (§5); `SetTractSize` keeps old values in place (§4.2);
`AddRadiusEffect` uses a 3-decimal distance table for R ≤ 15 and round-half-even (§7.1).

## 9. Open questions

1. x87 precision control during simulation (24-bit vs 64-bit mantissa): it changes results only for values within
   about 1e-7 of a rounding boundary (§7.1 `c − d × k`, §7.2). Conformance tests avoid such values.
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
