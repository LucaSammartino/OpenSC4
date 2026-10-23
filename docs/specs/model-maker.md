# `cSC4ModelMaker`: occupants to model instances (behaviour spec)

Scope: the whole `cSC4ModelMaker` owner as tools/coverage.py assigns it (105 functions, 34,705 bytes; inventory
in §20). The model-ID rules it calls (`cSC4ModelMakerUtility`) are graphics-formats §4.2 and are not repeated.
§14 outlines model making and §15 specifies the background loader `cSC4ModelBackgroundLoader`, an owner of its
own (re-modelmaker, 2026-10-22).
The 3D renderer it feeds is render3d.md / view3d.md. Evidence: the GOG 1.1.641 exe through the Ghidra export
(`tools/re_query.py`) and decomp/; REA was unavailable for this lane ("No app is open"), so every branch and
constant below was read from the instructions at the cited addresses.

Offsets are from the start of the object (the primary interface pointer). The save interface sits at `+0x04`, so
its methods see `this + 4`; offsets quoted for `Read`/`Write` are converted to object offsets here.

## 1. Identity, interfaces and lifetime *[compat]*

- Class ID and own interface ID `0xA9C741B3`. Created by the COM director factory `GZCOM_CreatecSC4ModelMaker`
  (`0x004929B0`): allocate 0x114 bytes, run the constructor, return the object (null when the allocation fails).
  The director registers it (`cSC4COMDirector` ctor `0x004602D3` calls the class-ID getter).
- Two vtables: primary `0x00A8EDF8` (5 slots) at `+0x00`, save interface `0x00A8EDE0` (6 slots) at `+0x04`.

| vtable | slot | address | behaviour |
|---|---|---|---|
| primary | 0 | `0x0048EA70` | QueryInterface (below) |
| primary | 1 | `0x004664A0` | AddRef: `+0x0C` += 1 (shared helper) |
| primary | 2 | `0x00493EC0` | Release: if `+0x0C` > 1, decrement and return the new count; else `Shutdown` (§3.3), then the deleting destructor (slot 4, flag 1), return 0 |
| primary | 3 | `0x00495F10` | DoMessage (§5) |
| primary | 4 | `0x004929D0` | destructor (§3.4); frees when bit 0 of the flag is set |
| save | 0 | `0x00491C80` | QueryInterface thunk (`this - 4`) |
| save | 1 | `0x006C2270` | AddRef thunk |
| save | 2 | `0x004947F0` | Release thunk |
| save | 3 | `0x0048EF60` | Write (§4) |
| save | 4 | `0x0048F090` | Read (§4) |
| save | 5 | `0x0048EA60` | GetGZCLSID: `0xA9C741B3` |

**QueryInterface** (`0x0048EA70`): IIDs `0xA9C741B3`, `1` (cIGZUnknown) and `0x452294AA` give the primary
pointer; `0xE4FDA3D4` (cIGZSerializable) gives `this + 4`; each success AddRefs and returns true. Any other IID
returns false and leaves the out pointer untouched. *[quirk]* On the `0xE4FDA3D4` path a null `this` stores null
and then still calls AddRef through it (`0x0048EABA`–`0x0048EAC4`), a crash; unreachable through a live object.

## 2. State *[compat]*

Constructor `0x00491AD0` (all other bytes zero):

| offset | type | meaning | initial |
|---|---|---|---|
| `+0x08` | byte | initialised (set by `Init`, cleared by `Shutdown`) | 0 |
| `+0x09` | byte | subscribed to the city messages (set by `DoPostCityInit`) | 0 |
| `+0x0A` | byte | loaded from a save (set at the start of `Read`) | 0 |
| `+0x0C` | uint32 | reference count | 0 |
| `+0x10` | ref | resource manager (global `0x00B43CA8`), from `Init` | null |
| `+0x14` | ref | the 3D renderer (`0x681BD529`, View3D window slot `+0x10`), from `Init` or `Read` | null |
| `+0x18` | ptr | renderer slot `+0x20` result (not counted) | null |
| `+0x1C` | ref | animation tick manager (service `0x86AD10ED`/`0x86AD10EC`), from `DoPostCityInit` | null |
| `+0x20`, `+0x24` | int32 | current zoom and rotation (View3D window `+0x0C`/`+0x10`) | -1, -1 |
| `+0x28`, `+0x2C` | int32 | terrain cell counts x, z (terrain slots `+0x1C`, `+0x20`) | 0 |
| `+0x30`..`+0x3C` | int32 | city cell rect x0, z0, x1, z1 (0, 0, x-1, z-1 at city init) | 0 |
| `+0x40` | uint32 | saved, see §4 | 0 |
| `+0x44` | byte | view-change suppression (§5) | 0 |
| `+0x48`, `+0x4C` | int32 | zoom/rotation pair last seen in `0xA6B79601` | 0 |
| `+0x50` | byte | | 0 |
| `+0x54` | ref | occupant manager (global `0x00B43D0C`) | null |
| `+0x58` | ref | View3D window slot `+0x48` result | null |
| `+0x5C`, `+0x60`, `+0x64` | ref | occupant filters (§6) | null |
| `+0x68` | ref | lot manager (global `0x00B43D08`) | null |
| `+0x6C` | ref | building development simulator (`0x00B43D68`) slot `+0x90` result | null |
| `+0x70` | ref | terrain (global `0x00B43CF4`) | null |
| `+0x74` | ref | animation coordinator (`cSC4AnimationCoordinator`, 0x5C bytes) | null |
| `+0x50` | byte | bulk reload in progress: per-occupant terrain hiding is skipped (§6.3, §10.1) | 0 |
| `+0x78`..`+0x88` | int32 ×5 | load radius in cells for zoom 0..4 (§7.1; corrected in round 2, earlier text called these budgets) | 0x100 |
| `+0x8C`..`+0x98` | int32 ×4 | load rect x0, z0, x1, z1 in 16 m city cells (§7.2) | 0 |
| `+0x9C`, `+0xA0` | int32 | load focus cell x, z (§7.2) | 0 |
| `+0xA4` | byte | background load started (§10) | 0 |
| `+0xA8`, `+0xAC` | int32 | focus cell at the last background (re)start (§10.2) | 0 |
| `+0xB0` | uint32 | all-zooms mask, bit k for zoom k = 0..4 (0x1F); never changed | 0x1F |
| `+0xB4` | uint32 | all-rotations mask, bit k for rotation k = 0..3 (0x0F); never changed | 0x0F |
| `+0xC2` | byte | unload pending: set by view changes (§10), consumed by `UnloadUnnecessaryModels` (§7.4) | 0 |
| `+0xB8` | byte | background loader enabled (command line, §3.2) | 1 |
| `+0xBC` | ptr | background loader (`cSC4ModelBackgroundLoader`, 0x188 bytes) | null |
| `+0xC0` | byte | save in progress (`0x26C63343` sets, `0x26C63345` clears) | 0 |
| `+0xC1` | byte | IgnoreMissingModelDataBugs (command line) | 0 |
| `+0xC3` | byte | view translation in progress (`0xCAD61E00` sets, `0xEAD61E16` clears) | 0 |
| `+0xC4`..`+0xCC` | float ×3 | the global `0x00B216F8` (all three), zeroed by `Init` | |
| `+0xD0`..`+0xE0` | ref ×5 | texture bindings (released by the destructor, nulled by `Shutdown`) | null |
| `+0xE4`..`+0xEC` | vector | 8-byte entries {counted object, value}; `Shutdown` releases every first word and empties it | empty |
| `+0xF4`..`+0x100` | float ×4 | saved, see §4 | 0 |
| `+0x104`..`+0x110` | int32 ×4 | saved; `+0x10C`/`+0x110` mirror zoom/rotation | -1 |

## 3. Lifecycle *[compat]*

### 3.1 `Init` (`0x00494800`)

If `+0x08` is set, return true. Otherwise: take `+0x58` from the View3D window (global `0x00B43DC4`, slot
`+0x48`) when it is null; build the occupant filters (§6); subscribe on message server 2 (`0x00B43CCC`) to
`0x26D31EC1` PostCityInit, `0x26C63343` PreSave and `0x26C63345` (post save); take the resource manager into
`+0x10` and the renderer into `+0x14` (AddRef each) when null, and `+0x18` from renderer slot `+0x20`; zero
`+0xC4`..`+0xCC`; read the model detail global `0x00B216FC` (graphics-formats §4.2: option `0x16` through
`0x00B43CC4` slot `+0x1C`, clamped to 0..2); compute the load radius (§7.1); set `+0x08`; return true.

### 3.2 `DoPostCityInit` (`0x00494DA0`, message `0x26D31EC1`)

1. Command line (framework slot `+0x54`, option lookup slot `+0x20`), each value lowercased:
   `backgroundLoader`: a value of exactly "off" clears `+0xB8`, a value equal to "on" sets it;
   `IgnoreMissingModelDataBugs`: "off" clears `+0xC1`, "on" sets it. Any other value leaves the default.
2. If `+0xB8`: create the background loader (0x188 bytes), AddRef it, `Init` it with this object (§15.1), store `+0xBC`.
   *[quirk]* It is not tested for an existing loader, so a second PostCityInit leaks the first.
3. If `+0x74` is null: create, AddRef and `Init` the animation coordinator.
4. Fill each null reference: `+0x54` occupant manager, `+0x68` lot manager, `+0x6C` (`0x00B43D68` slot `+0x90`),
   `+0x70` terrain; when the terrain is taken, also `+0x28`/`+0x2C` = its cell counts and the rect `+0x30`..`+0x3C`
   = (0, 0, x-1, z-1). `+0x1C` = the animation tick manager when null.
5. If the View3D window (`0x00B43DD8`) exists: `+0x20`/`+0x10C` = its zoom (`+0x0C`), `+0x24`/`+0x110` = its
   rotation (`+0x10`).
6. `UpdateModelLoadRect` (§7.2). Release `+0x58` and take it again from View3D slot `+0x48`.
7. Enumerate the occupant manager (slot `+0x98`: out list, 0, 0, filter `+0x5C`, -1). For each occupant o, in list
   order: `DoOccupantCreated(o, o.IsFlagSet(4), o.GetType())`, then `DoOccupantShown` with the same arguments, then
   release o.
8. Subscribe to the 25 messages of §5 (all but the three of `Init`), register with the tick manager (slot `+0x14`,
   value `0xC0`), set `+0x09`.

### 3.3 `Shutdown` (`0x004929F0`)

If `+0x08` is clear, return true. Otherwise in this order: shut down and release the animation coordinator, then
the background loader; release `+0x1C`, `+0x70`, `+0x6C`, `+0x68`, `+0x54`; zero `+0x18` and release `+0x14`;
release `+0x10`; null the five texture bindings. If `+0x09` is set: clear it, unregister from the tick manager
(`0xC0`) and from the 25 city messages. In every case unregister the three `Init` messages. Release `+0x58`,
`+0x64`, `+0x60`, `+0x5C`; empty the `+0xE4` vector (releasing each entry); clear `+0x08`; return true.

### 3.4 Destructor (`0x00491C90`, deleting wrapper `0x004929D0`)

Restore both vtables, destroy the `+0xE4` vector (`0x007B3BD0`), release the five texture bindings from `+0xE0`
down to `+0xD0`. Nothing else is released: the destructor relies on `Shutdown` having run (Release runs it).

## 4. Save record (`Write` `0x0048EF60`, `Read` `0x0048F090`) *[compat]*

Version uint16 2, then in order:

| # | field | encoding |
|---|---|---|
| 1 | `+0x14` renderer | serialisable reference, IID `0x681BD529` |
| 2 | `+0x20`..`+0x3C` | 8 × sint32 |
| 3 | `+0x40` | uint32 |
| 4 | `+0x54` occupant manager | reference, IID `0xC6BB8334` |
| 5 | `+0x68` lot manager | reference, IID `0x681905BD` |
| 6 | `+0x6C` | reference, IID `0x4818E5D2` |
| 7 | `+0x70` terrain | reference, IID `0xC70BD37E` |
| 8 | `+0xF4`..`+0x100` | 4 × float32 |
| 9 | `+0x104`..`+0x110` | 4 × sint32 |

`Read` sets `+0x0A` first, returns false when the version is not 2 (nothing else read), and otherwise returns true
when the stream reports no error. `Write` returns true when the stream reports no error. The references read here
are the ones `Init`/`DoPostCityInit` then keep (they fill only null fields).

## 5. Messages (`DoMessage` `0x00495F10`) *[compat]*

Every message returns true. The occupant messages pass (occupant = message slot `+0x28`, flag = message slot `+0x38`
!= 0, value = message slot `+0x4C`); occupant-manager §5.5 names these the occupant, data2 (`IsFlagSet(4)`) and
data4.

| ID | name | action |
|---|---|---|
| `0x26D31EC1` | PostCityInit | §3.2 |
| `0x26C63343` | PreSave | `+0xC0` = 1 |
| `0x26C63345` | post save | `+0xC0` = 0 |
| `0x99EF1140` | occupant created | if occupant slot `+0x14` is true: `DoOccupantCreated` (§8.1) |
| `0x99EF1141` | occupant destroyed | `DoOccupantDestroyed` (§8.2) |
| `0x99EF1142` | occupant inserted | `0x00494290` (§8.3) |
| `0x99EF1143` | occupant removed | `0x00494310` (§8.3) |
| `0x99EF1144` | occupant shown | `DoOccupantShown` (§8.4) |
| `0x99EF1145` | occupant changed | `DoOccupantChanged` (§8.5) |
| `0x99EF1146` | occupant moved | `DoOccupantMoved` (§8.6) |
| `0xE6E85114` | occupant highlighted | `DoOccupantHilighted(occupant, slot +0x38 value, value)` (§8.7) |
| `0xE99B3D4D` | lot state | if the message object (slot `+0x80`) answers `0xE818FCED`: `DoLotStateChanged(lot, slot +0x24, slot +0x38)`, release the lot (§9.2) |
| `0x09D8E0B6` | building visual state | §9.1 |
| `0x2977AE2F` | building completion | `DoBuildingCompletionChanged(slot +0x28)` (§9.5) |
| `0xE9C86DDB` | prop state | `DoPropStateChanged(slot +0x28, low byte of slot +0x38)` (§9.4) |
| `0x06CDB65B` | view filter | `DoViewFilterChange(slot +0x28, slot +0x3C)` (§10.3) |
| `0x49D968C4` | defer occupant operations | `0x00495970(slot +0x24 != 0)` (§8.8) |
| `0xA6B79601` | zoom/rotation change | p = slot `+0x28`, q = slot `+0x3C` (pairs). If `+0x44` is clear: `UpdateModelLoadRect`, then `DoZoomAndRotationChange(p0, p1, q0, q1)` (§10.1). If set: store q into `+0x48`/`+0x4C` |
| `0xA6B79602`, `0xA6B79620` | view update, post view init | if `+0x44` is clear and the View3D window exists: q = slot `+0x3C`; `+0x10C` = `+0x20` = q0, `+0x110` = `+0x24` = q1 |
| `0xA6B79611` | view translated | `UpdateModelLoadRect`, `DoViewTranslationChange` (§10.2) |
| `0xCAD61E00` | translation start | `+0xC3` = 1; if `+0xB8` and the loader exists, loader `Pause` (`0x0048C960`, §15.5: `+0x28` = 1, the worker ends its slice at once) |
| `0xEAD61E16` | translation end | `+0xC3` = 0, `UpdateModelLoadRect`, loader `Resume` (`0x0048C970`, `+0x28` = 0) under the same test, `DoViewTranslationChange` |
| `0xC9DA96EA` | lighting changed | `DoLightingChanged` (§11) |
| `0xBA44EB51` | game idle | `UnloadUnnecessaryModels` (§7.4) |
| `0xC9F775BB` | options changed | re-read `0x00B216FC` (option `0x16`, clamped 0..2) |
| `0x86AD10EE` | animation tick (from the tick manager) | if the loader exists, `ProcessBackgroundThreadToMainThreadCommunications` (§15.5) |
| `0x4AB04CBE`, `0x4AB04CBF` | | subscribed but not handled |

*[quirk]* In `0xA6B79601` with `+0x44` set, only q is stored; p is dropped.

## 6. Occupant filters (`CreateOccupantFilters` `0x00493EF0`) *[compat]*

### 6.1 Construction

Called from `Init`. Each of the three slots is filled only when it is null, and the new filter is AddRef'd once
(slot `+0x04`) as the model maker's reference.

| slot | class | accepted occupant types (inserted in this order) |
|---|---|---|
| `+0x5C` type filter | occupant types filter (0x20 bytes, vtables `0x00A8EE38` / `0x00A8EE20`, hash set of 100 buckets) | `0x278128A0` building, `0x68FD0C69` foundation, `0x895D1169` wall, `0x74758926` flora, `0xA823821E` prop, `0x497F6D9D` base texture, `0xC772BF98` network, `0x2890D4DE` power pole, `0x2A183167` pipe connection, `0x8A13918A` subway connection, `0x6A11BD2B` landfill |
| `+0x60` terrain-hiding filter | same class | `0x68FD0C69`, `0x497F6D9D`, `0xC772BF98`, `0x2A183167`, `0x8A13918A` |
| `+0x64` zoom/rotation filter | `cSC4ZoomRotationModelChangeOccupantFilter` (0x10 bytes, ctor `0x00496790`, vtable `0x00A8EEA0`) | §6.4 |

### 6.2 The occupant types filter (`0x00A8EE38`)

The set of type IDs lives at object `+0x0C` (element count at `+0x1C`). Slots: `+0x0C` IsOccupantIncluded(o) =
slot `+0x10` of o's type (occupant slot `+0x1C`); `+0x10` IsOccupantTypeIncluded(t) (`0x00493700`) = t is in the
set; `+0x14` (property) always true (`0x004FC900`); `+0x18` deleting destructor (`0x00493720`: destroy the set, base
destructor `0x005E95C0`, free when bit 0 of the argument is set).

Serialisation, through the second interface at object `+0x04` (vtable `0x00A8EE20`):

- **Write** (`0x005EA370`, slot 3): PutUint16(3) twice; then, if the stream has no error, PutUint32(count) and
  PutUint32 of each type in hash-set iteration order. Returns true iff the stream has no error.
- **Read** (`0x005EA550`, slot 4): the first GetUint16 must be 3, else return false at once. If the second is 3
  and the stream has no error: GetUint32 count, then count × GetUint32, each inserted into the set (the set is not
  cleared first). Returns true iff the stream has no error.

*[quirk]* A second version word other than 3 skips the body but still returns "no error" (normally true).

### 6.3 Per-occupant refresh (`OccupantFilterIterationFunction` `0x00495360`)

Argument o; always returns true. Same body as §8.4 with f computed rather than passed:

1. f = the View3D object `+0x58` slot `+0x0C(o)` (o is visible). f set: occupant SetFlag(2) (slot `+0x74`); clear:
   ClearFlag(2) (slot `+0x7C`).
2. t = o's type (slot `+0x1C`). If `+0x50` is clear and the terrain-hiding filter `+0x60` accepts t:
   `DoTerrainHidingForOccupantCells(o, t, f)` (§13).
3. Model holder h = occupant slot `+0x4C`. If present: AddRef; when f is set and h slot `+0x0C` returns 0 (no
   models), `AddModels(o, +0x20, +0x24, t, false)` (§12); h slot `+0x20(f)`; Release at the end.
4. `SetOccupantAnimation(o, f)` on the animation coordinator `+0x74`.

Its one caller is `DoViewFilterChange` (`0x00495A96`, §10.3).

### 6.4 The zoom/rotation filter

`SetZoomAndRotation(z, r)` (`0x004967B0`) stores mask = (1 << (z + 2)) + (1 << (r + 7)) at `+0x0C`. Its
IsOccupantIncluded (`0x004967E0`): h = occupant slot `+0x4C`; when h is null → true; otherwise m = h's word `+0x2C`
AND mask; included unless m has a bit in `0x7C` (the zoom bits) **and** a bit in `0x780` (the rotation bits). So
the holder's `+0x2C` word records loaded zooms in bits 2..6 and rotations in bits 7..10, and the filter selects the
occupants still lacking the current zoom or rotation. Slots `+0x10`/`+0x14` return true; `+0x18` is the shared
default destructor `0x006C4A00`. (This class is not owned by `cSC4ModelMaker`; described here for its contract.)

## 7. Load radius, load rect and unloading *[compat]*

Fast rounding used below (the float32 `1.5·2^23` trick): **F(x)** = n, the nearest integer to x − 0.5 (ties to
even), raised to n + 1 only if n + 1 equals x − 0.5 exactly; **C(x)** = n, the nearest integer to x + 0.5 (ties to
even), lowered to n − 1 only if n − 1 equals x + 0.5. F is floor and C is ceiling except at exact odd integers v,
where F(v) = v − 1 and C(v) = v + 1 (even integers are exact).

### 7.1 `DetermineLoadRadiusAroundVisibleArea` (`0x0048EB50`)

From `Init`. m = total physical memory in MiB as `cRZVersion::GetSystemMemory` (`0x0089157C`) reports it:
GlobalMemoryStatus total physical >> 20, rounded up to a multiple of 4. If that call fails the radii stay 0x100.
Otherwise, with m as an unsigned value:

| zoom | slot | radius | minimum |
|---|---|---|---|
| 0 | `+0x78` | F(m × 0.5) | 8 |
| 1 | `+0x7C` | F(m × 0.25) | 8 |
| 2 | `+0x80` | F(m × 0.125) | 6 |
| 3 | `+0x84` | F(m × 0.0625) | 6 |
| 4 | `+0x88` | F(m × 0.03125) | 6 |

Example: m = 512 → 256, 128, 64, 32, 16. m = 4096 (the cap of the 32-bit status call) → 2048, 1024, 512, 256, 128.

### 7.2 `UpdateModelLoadRect` (`0x0048F660`)

Does nothing unless both the View3D window (`0x00B43DD8`) and the renderer (`0x00B43DD0`) exist. With z = the
window's zoom (`+0x0C`) and its visible rect r0..r3 (floats at window `+0x3EC`..`+0x3F8`, metres):

1. Load rect: `+0x8C` = F(r0/16), `+0x94` = C(r2/16), `+0x90` = F(r1/16), `+0x98` = C(r3/16).
2. The renderer's camera control (slot `+0x40`) is fetched; its result is unused.
3. g = the float `0x00B216F8`; focus `+0x9C` = F((2g + window `+0x3DC`)/16), `+0xA0` = F((2g + window `+0x3E4`)/16).
4. If the background loader is enabled (`+0xB8`) and exists (`+0xBC`): loader `ExpandLoadRect(&+0x8C, z)`
   (`0x0048CA40`, not this owner). Otherwise grow the rect by the radius of zoom z (`+0x78 + 4z`) on all four sides.
5. Clamp: x0, z0 ≥ 0; x1 ≤ `+0x28` − 1; z1 ≤ `+0x2C` − 1.

### 7.3 `ComputeCityExclusionRects(L, out)` (`0x0048ED20`) and rect intersection (`0x0048ED90`)

With X = `+0x28`, Z = `+0x2C` and L = (x0, z0, x1, z1), out receives four inclusive rects (x0, z0, x1, z1), the
city minus L: (0, 0, X−1, L.z0−1); (0, L.z1+1, X−1, Z−1); (0, L.z0, L.x0−1, L.z1); (L.x1+1, L.z0, X−1, L.z1).
A rect with x0 > x1 or z0 > z1 is empty.

`0x0048ED90(out, a, b)` (thiscall on out): if a and b overlap (inclusive: a.x1 ≥ b.x0, a.x0 ≤ b.x1, a.z1 ≥ b.z0,
a.z0 ≤ b.z1) out = the per-edge max of the mins and min of the maxes and it returns true; otherwise out = 0, 0, 0, 0
and false. It has 32 callers outside this owner too.

### 7.4 `UnloadUnnecessaryModels` (`0x00492530`, message `0xBA44EB51`)

1. If `+0xC2` is clear: return.
2. If `GetSystemMemory` succeeds with m > 32: clear `+0xC2` and return. *[quirk]* So the body below only runs on
   machines reporting at most 32 MiB (or when the memory query fails).
3. Clear `+0xC2`; build the four exclusion rects of the load rect (§7.3). For each non-empty rect, in the order
   above: list the occupants by standard city cells (manager slot `+0xA0`: list, min (x0, z0), max (x1, z1), filter
   `+0x5C`, -1). For each listed occupant with a model holder whose slot `+0x0C` is non-zero: if holder slot `+0x30`
   ≠ `+0xB0` or holder slot `+0x38` ≠ `+0xB4`, call holder slot `+0x2C` (unload). Release each occupant; empty the
   list.
4. Resource manager `+0x10` slot `+0xAC`.

### 7.5 `RemoveModelsInappropriateForRotation(r)` (`0x00492780`)

bit = 1 << r. For every occupant accepted by the type filter `+0x5C` (manager slot `+0x98` with no rect): when it
has a model holder with slot `+0x0C` ≠ 0 and holder slot `+0x38` has no bit in common with bit, holder slot
`+0x2C`. Each occupant is released.

### 7.6 Mask helpers

`0x0048EA00(mask)`: index of the lowest set bit among bits 0..4, else 5 (also for mask 0). `0x0048EA30(mask)`: the
same over bits 0..3, else 4. Both called from `LoadOccupantModelsForZoomAndRotation` (§10.4).

## 8. Occupant handlers and the deferred queue *[compat]*

All handlers take (occupant o, flag f, type t) as built in §5. Each first asks the type filter `+0x5C` (slot
`+0x10`, argument t); a rejected type does nothing. While the defer count `+0xF0` is non-zero, a handler does not
act: it appends {o (AddRef), op} to the queue `+0xE4` (`0x004936A0`) and returns. Op codes:

| op | handler | address |
|---|---|---|
| 1 | created | `DoOccupantCreated` `0x00494970` |
| 2 | destroyed | `DoOccupantDestroyed` `0x00493750` |
| 3 | shown | `DoOccupantShown` `0x00493B90` |
| 4 | changed | `DoOccupantChanged` `0x00494A30` |
| 5 | moved | `DoOccupantMoved` `0x004937D0` |
| 6 | highlighted (entry bytes +5, +6 keep its two arguments) | `DoOccupantHilighted` `0x00493A00` |
| 7 | inserted (queued only when f is set) | `0x00494290` |
| 8 | removed | `0x00494310` |

Entry layout: {counted occupant, op byte at +4, two argument bytes at +5/+6}; op 0 marks a cancelled entry.

### 8.1 Created (op 1)

Not deferring: call occupant slot `+0x50`, copy `+0x20` to `+0x10C`, then if the occupant has no model holder
(occupant slot `+0x4C` null) or the holder's slot `+0x0C` returns 0, `AddModels(o, zoom +0x20, rotation +0x24,
t, false)` (§12).

### 8.2 Destroyed (op 2)

Not deferring: `RemoveModels(o, t)` (§12).

### 8.3 Inserted (op 7) and removed (op 8)

Inserted: only when f is set; not deferring, it is `DoOccupantShown(o, f, t)`. Removed: not deferring, it is
`DoOccupantShown(o, false, t)`.

### 8.4 Shown (op 3)

Not deferring: when f is set, ask the View3D object `+0x58` (slot `+0x0C`, argument o) whether o is visible and
clear f when it is not, then set occupant flag 2 (slot `+0x74`); when f is clear, clear occupant flag 2 (slot
`+0x7C`). If the terrain-hiding filter `+0x60` (slot `+0x10`, t) accepts, `DoTerrainHidingForOccupantCells(o, t,
f)` (§13). Then the occupant's model holder (slot `+0x4C`), when present, gets slot `+0x20(f)` and is released.
Finally `SetOccupantAnimation(o, f)` on the animation coordinator `+0x74`.

### 8.5 Changed (op 4)

Not deferring, and only when occupant flag 1 is set (slot `+0x80(1)`): when f is clear, the model holder (if any)
gets slot `+0x20(0)` (hidden); when f is set, `RemoveModels(o, type)`, then if `+0x58` slot `+0x0C(o)` is true,
`AddModels(o, zoom, rotation, type, true)`.

### 8.6 Defer count (`0x00495970`, message `0x49D968C4`)

The message's flag (slot `+0x24` != 0) increments `+0xF0` when true and decrements it when false. When the count
goes from positive to zero or below and the queue is not empty, `EnableDeferredOccupantOperations` runs.
*[quirk]* Unbalanced decrements make the count negative; handlers then queue again until it returns to 0.

### 8.7 Replaying the queue (`EnableDeferredOccupantOperations` `0x00494B30`)

1. **Destroy cancels earlier work.** For each entry D with op 2, from the last entry backwards, every earlier entry
   with the same occupant is cancelled (op 0, reference released); if one of those was a Create (op 1), D is
   cancelled too (created and destroyed while deferred: nothing to do).
2. **Insert covers a change.** For each op 4 entry C, if an earlier entry with the same occupant has op 7, C is
   cancelled.
3. Replay the remaining entries in queue order: f = occupant `IsFlagSet(4)` (slot `+0x80`), t = `GetType()`
   (slot `+0x1C`), each calling the op's handler (op 6 with the two stored bytes instead).
4. Release every entry and empty the queue.

*[inferred]* that steps 1 and 2 visit every index (the decompiled loop heads are merged); the replacement visits
all indices.

### 8.8 Moved (op 5, `0x004937D0`)

No type filter: only occupant types `0x74758926`, `0xA823821E` and `0x278128A0` are handled; others do nothing.
Not deferring: p = occupant position (slot `+0x20`, three floats). For each model instance of the occupant's
model holder (holder slot `+0x0C` count, slot `+0x10` fills a list; at most 10 on the stack, more on the heap):
build a transform with an identity 3×3 rotation and translation p, read the instance's current transform record
(instance slot `+0x24`), apply the new one (slot `+0x28`) with bit 0 of the record's flag byte set and its
change counter incremented, and if the renderer `+0x14` holds the instance (slot `+0x6C`) re-queue it (slot
`+0x78`; render3d.md §5). Release the holder. *[outline]* the exact transform-record layout is in the remainder
(§20); the identity-plus-translation content is read from `0x0049388E`–`0x00493920`.

### 8.9 Highlighted (op 6, `0x00493A00`)

Arguments (o, h, c): h = message slot `+0x38`, c = message slot `+0x4C` (§5). No type filter. If h == c
(compared as 32-bit values), nothing happens. While deferring: queue op 6 with h and c kept as the entry's bytes
+5 and +6 (low bytes), and return. Otherwise, for each model instance of o's model holder (slot `+0x0C` count,
slot `+0x10` list): read the instance's highlight record (instance slot `+0x3C`), set bit 0 of its first byte to
(h != 0) keeping the other bits, set its second byte to the low byte of c, write it back (slot `+0x40`), and if the
renderer `+0x14` holds the instance (slot `+0x6C`), call renderer slot `+0x7C(instance, 1)`. Release the holder.
*[inferred naming]* bit 0 = highlighted, second byte = highlight colour/kind. *[quirk]* A replayed op 6 passes the
two stored bytes, so h and c above 255 replay truncated.

## 9. State-change handlers (partial)

### 9.1 State effect on one occupant (`0x004933A0(o, t, x, y)`, message `0x09D8E0B6`) *[compat]*

Returns a bool (RET 0x10) and always returns true; t and y are not read. Steps:

1. h = o's model holder (slot `+0x4C`), AddRef'd when present (h may be null; every later use of h passes it on
   unchecked, see the quirk below).
2. If o's exemplar (slot `+0x18`) has property `0x09F00E59` true (read through `0x005FD3C0`; missing = false):
   release h and return true, nothing else.
3. Otherwise dispatch on x through the 7-entry table at `0x0049367C`; x > 6 (unsigned) takes the default.

| x | target | effect |
|---|---|---|
| 0 | `0x004934C8` | "lit" test, below; on success swap to the +0x8000 textures, else default |
| 1 | `0x00493647` | default |
| 2 | `0x00493406` | dynamic texture 0 (`+0xD0`) |
| 3 | `0x0049345E` | dynamic texture 1 (`+0xD4`) |
| 4 | `0x004934AC` | dynamic texture 4 (`+0xE0`) |
| 5 | `0x00493477` | dynamic texture 2 (`+0xD8`) |
| 6 | `0x00493490` | dynamic texture 3 (`+0xDC`) |

- **Dynamic texture k** (x = 2..6): if the binding at `+0xD0 + 4k` is null, `RebuildDynamicTexture(k)` (§11). If it
  is still null, take the default. Otherwise `ApplyTextureToModels(h, binding, 2, 0)` (`0x004906B0`, §12.4), then
  `ChangeModels(o, 0x278128A0, 2)` (§12.4), release h, return true.
- **Lit test** (x = 0): o is asked for three interfaces in order and the first that answers decides:
  1. `0x87DDA3A9`: the answer is its slot `+0x40` (bool).
  2. else `0xA821EF94`: the answer is its slot `+0xF0` (bool).
  3. else `0xE9793A65` (a lot-attached occupant): the answer is false unless o's exemplar has property
     `0x4A9F188B` whose value (property slot `+0x14`, then value slot `+0xFC`) points to a non-zero byte. Then
     the interface's slot `+0x0C` gives an object whose slot `+0x2C` fills a cell rect (x1, z1, x2, z2); the lot
     manager (`+0x68`) slot `+0x18((x1 + x2) >> 1, (z1 + z2) >> 1, 0)` gives the lot at the rect centre
     (arithmetic shift, so it rounds towards minus infinity); null lot = false; the lot's slot `+0xC0` gives an
     object (null = false) whose slot `+0x40` is the answer.
  4. none of the three: false.
  When the answer is true, `SwapTexturesByOffset(h, 0x8000, 2, 1)` (`0x00490820`, §12.4); if that reports a
  change, `ChangeModels(o, 0x278128A0, 2)`. Then the queried interfaces are released and the function releases h
  and returns true **without** running the default. When the answer is false (or no interface answers), the
  interfaces are released and the default runs.
- **Default** (x = 1, x > 6, failed lit test, missing dynamic texture): `RestoreStaticTextures(h)` (`0x0048F380`,
  §12.4); if it reports a change, `ChangeModels(o, 0x278128A0, 2)`. Release h, return true.

Quirk: with a null h the dispatch still calls the texture helpers on it (they call through h's vtable), so an
occupant without a model holder must not reach steps 3+ in a conforming caller; the original never guards it.

### 9.2 Lot state (`DoLotStateChanged(lot, a, b)` `0x00493D60`, message `0xE99B3D4D`) *[compat]*

a = message slot `+0x24`, b = message slot `+0x38` (§5). Take the lot's cell rect (lot slot `+0x38`, four int32)
and list the occupants in it by standard city cells (manager slot `+0xA0`: list, min, max, **no filter**, -1). For
each listed occupant o that has a model holder (slot `+0x4C` non-null; it is not released here):
`ApplyStateEffectToModels(o, o.GetType(), b, a)`. Release each occupant; free the list. Returns nothing.

*[quirk]* The two state arguments reach `ApplyStateEffectToModels` swapped (pushed a first, so the callee's third
argument is b and its fourth is a).

### 9.3 `ApplyStateEffectToModels(o, t, x, y)` (`0x00493C80`) *[compat]*

Returns a bool.

| t | action |
|---|---|
| `0x278128A0` building, `0xA823821E` prop, `0xC772BF98` network, `0x2A183167` pipe connection, `0x8A13918A` subway connection | return `0x004933A0(o, t, x, y)` (§9.1) |
| `0x68FD0C69` foundation | QueryInterface o for `0xC8FCE101` (the foundation interface); if its slot `+0x24` returns 1: r = `0x004933A0(o, 0x68FD0C69, x, y)`, release, return r; otherwise release and return true |
| any other | return true |

*[quirk]* The foundation interface is used without a null check: a foundation-typed occupant that fails the
QueryInterface faults (`0x00493CDC`).

### 9.4 Prop state (`DoPropStateChanged` `0x00494900`, message `0xE9C86DDB`) *[compat]*

Let o = (message object) slot `+0x0C` (its occupant). If o's exemplar (o slot `+0x18`) has property
`0x4A70D491`: `ChangeModels(o, 0, 2)` and return. Otherwise `RemoveModels(o, t)` with t = o.GetType(), then
`AddModels(o, zoom +0x20, rotation +0x24, t, true)`. *[quirk]* The state byte the message carries (second
argument) is never read (`0x00494900`–`0x00494960`); `ChangeModels` always gets the constants 0 and 2.

### 9.5 Building completion (`DoBuildingCompletionChanged(msg)` `0x0048FFF0`, message `0x2977AE2F`) *[compat]*

Lowers a building under construction into the ground in proportion to how much is left to build. RET 4.

1. o = msg slot `+0x0C`; h = o's model holder (slot `+0x4C`); null: return. AddRef h; n = h slot `+0x0C`.
2. c = the float at msg slot `+0x24` (completion, 1.0 = complete). Tell the holder h slot `+0x24(c >= 1.0)`
   (a NaN gives false).
3. If n > 0: fill the instance array (h slot `+0x10`, inline below 10, else heap); p = o's position (o slot
   `+0x20`, three floats). For each instance:
   - Bounding box: a record (min x, y, z, max x, y, z) pre-set to (1, 1, 1, 0, 0, 0) filled by instance slot
     `+0x20`.
   - y = p.y; only when c < 1.0 (not for NaN): y = p.y - ((maxY - minY) * (1.0 - c)) * 1.05f, evaluated in that
     order (`0x00A8EDA0` = 1.05f, `0x00A81228` = 1.0f), rounded to float once.
   - Read the instance's transform record (slot `+0x24`, record pre-set to identity with header bytes 0), set its
     translation to (p.x, y, p.z), OR 1 into header byte 1, increment header byte 0, write it back (slot `+0x28`).
     The 3×3 part read back is kept.
   - Flags m = instance slot `+0x34`. When c < 1.0 and bit 0 of m is clear: instance slot `+0x38(m | 1)`. When
     c >= 1.0 (or NaN) and bit 0 is set: slot `+0x38(m & ~1)`. In both changing cases, if the renderer (`+0x14`)
     slot `+0x6C(inst)` is true, renderer slot `+0x7C(inst, 5)`.
   - Always: if renderer slot `+0x6C(inst)`, renderer slot `+0x78(inst)`.
4. Free the array, release h.

Example: a building whose model is 20 units tall at 40 % completion: c = 0.4, every instance is placed at
p.y - 20 × 0.6 × 1.05 = p.y - 12.6, bit 0 is set on each instance; at c = 1.0 the instances return to p.y and
bit 0 is cleared.

## 10. View changes *[compat]*

"Reload pass over list L" below means: if L is non-empty, set `+0x50`; run the per-occupant step on each occupant
in list order, releasing each; clear `+0x50`; `DoTerrainHidingForCells((0, 0, X−1, Z−1), 0)` over the whole city
(§13; X = `+0x28`, Z = `+0x2C`); terrain `+0x70` slot `+0x110(0, 0, &city rect +0x30, 0)`. An empty L skips all of
it. Lists come from manager slot `+0x98` (GetOccupantsByOccupantManagerCells: list, min, max, filter, -1); "whole
city" passes null min and max.

### 10.1 `DoZoomAndRotationChange(oldZoom, oldRot, newZoom, newRot)` (`0x00495B00`)

1. Hold an input-queue auto-flusher (1000) for the whole call.
2. If oldZoom ≠ newZoom: null the five texture bindings `+0xD0`..`+0xE0` (`0x0048EEA0`, release each). Else if
   oldRot = newRot: return.
3. `+0xC2` = 1. If the occupant manager `+0x54` is null: return.
4. Wait cursor (id 0xF) and a second flusher (1000). `+0x104` = oldZoom, `+0x108` = oldRot, `+0x10C` = newZoom,
   `+0x110` = newRot. `SC4ViewUtilities::GetCameraViewAnglesForRotation(newRot, newZoom, &+0xF4, &+0xFC, &+0x100)`
   (`0x007F7780`); `+0xF8` = (`+0x18` object) `+0x1C`.
5. Background loader enabled (`+0xB8`):
   - not saving (`+0xC0` clear): if oldRot ≠ newRot, `RemoveModelsInappropriateForRotation(newRot)` (§7.5); `+0xA4`
     = 1; loader `StartLoadingForZoomAndRotation(newZoom, newRot, &+0x8C, &+0x9C)` (`0x0048DAE0`, §15.3); `+0xA8`/`+0xAC`
     = `+0x9C`/`+0xA0`; go to step 7.
   - saving: loader `CancelAllLoadTasks` (`0x0048CD60`, §15.3), `+0xA4` = 0, then step 6.
6. Loader disabled (or saving): the zoom/rotation filter `+0x64` gets `SetZoomAndRotation(newZoom, newRot)`
   (§6.4); reload pass over the whole-city list for filter `+0x64`, the step being
   `LoadOccupantModelsForZoomAndRotation(o, newZoom, newRot)` (§10.4).
7. `SC4FlushInputMessageQueue` (`0x005F8610`), then drop the cursor and flushers.

### 10.2 `DoViewTranslationChange` (`0x00495D90`)

If a translation is in progress (`+0xC3`): return. Set `+0xC2`. Then:

- Loader enabled and not saving: if `+0xA4` is set, the refocus step `0x0048EAE0` (a model-maker method despite
  its export name `cSC4ModelBackgroundLoader::RefocusLoadToNewViewLocation`): when `+0xB8` is set and
  |`+0xA0` − `+0xAC`| + |`+0x9C` − `+0xA8`| > 4, loader `0x0048CB40(&+0x8C, &+0x9C)` (the loader's refocus, §15.3) and `+0xA8`/`+0xAC` =
  `+0x9C`/`+0xA0`. If `+0xA4` is clear: `StartLoadingForZoomAndRotation(+0x10C, +0x110, &+0x8C, &+0x9C)`, copy
  the focus to `+0xA8`/`+0xAC`, `+0xA4` = 1.
- Otherwise: list the occupants in the load rect (min (`+0x8C`, `+0x90`), max (`+0x94`, `+0x98`), filter `+0x5C`)
  and call `LoadOccupantModelsForZoomAndRotation(o, +0x10C, +0x110)` on each, releasing each. No `+0x50`, no
  terrain pass.

*[quirk]* The load rect is in 16 m city cells but goes to the manager's occupant-manager-cell query (slot
`+0x98`, occupant-manager §3), not the city-cell one (slot `+0xA0`) that §7.4 uses.

### 10.3 `DoViewFilterChange(arg1, filter)` (`0x004959C0`, message `0x06CDB65B`)

Only the second argument is read. If filter = `+0x58`: return. Otherwise AddRef filter, release the old one, store
it in `+0x58`. Then, if `+0x44` is clear and the occupant manager exists: reload pass over the whole-city list for
the type filter `+0x5C`, the step being `OccupantFilterIterationFunction(o)` (§6.3).

### 10.4 `LoadOccupantModelsForZoomAndRotation(o, zoom, rot)` (`0x00495430`) *[compat]*

Traced from the instructions (the decompiler output is unusable). RET 0xC, no result. zbit = 1 << zoom,
rbit = 1 << rot; ex = o's exemplar (slot `+0x18`).

1. h = o's model holder (slot `+0x4C`), AddRef'd when present. zmask = h slot `+0x30`, rmask = h slot `+0x38`
   (both 0 without h). If (zmask & zbit) and (rmask & rbit): already loaded, release h, return.
2. t = o slot `+0x1C`; n = h slot `+0x0C` (0 without h). If n = 0: `AddModels(o, zoom, rot, t, 1)` (§12),
   release h, return.
3. If `cSC4ModelMakerUtility::GetOccupantModelClass(o)` = `0xAA123BF9`: release h, return (nothing reloads).
4. fixed = ex has property `0xE83A081D` (ex slot `+0x0C`, existence only). lz = `0x0048EA00(zmask)` and
   lr = `0x0048EA30(rmask)` (§7.6: the loaded zoom, 5 when none; the loaded rotation, 4 when none).
5. Two id lists, each created with 4 zero entries: N for the request and L for what is loaded.
   `GetOccupantOrientationAndState(o, &a, &b)` (`0x00496670`), then the occupant form of
   `GetAllModelInstanceIDs` (`0x00497120`, graphics-formats GF7) fills N for (zoom, rot) and, unless lz = 5 or
   lr = 4, fills L for (lz, lr); both calls pass a, fixed, b and no offset/key outputs. (So the "fixed" byte is
   the flag argument of GF7 here, not the occupant's flag `0x80`.)
6. Decision, in this order:
   - N ≠ L (element-wise list comparison): **reload**.
   - lr ≠ rot and ex has property `0x4835C6A6` (ex slot `+0x24` non-null): **reload**.
   - fixed, or lr = rot: **nothing** (the masks are not updated either; a pure zoom change whose ids are equal
     leaves zmask without zbit, so the next call re-runs this decision).
   - otherwise: **rotate in place**.
7. Reload = `RemoveModels(o, t)` (§12.1) then `AddModels(o, zoom, rot, t, 1)`.
8. Rotate in place: θ = -(`+0xF8`) (the camera yaw saved by §10.1 step 4, float). For each of the holder's n
   instances (array filled by h slot `+0x10`): read its transform record (instance slot `+0x24` into a record
   pre-set to identity: two header bytes 0, 3×3 identity, translation 0, then a 1.0), overwrite the 3×3 part
   with the Y rotation rows (cos θ, -0, sin θ), (0, 1, -0), (-sin θ, 0, cos θ) (cos θ and sin θ rounded to float;
   the zero entries come from multiplying by the cached cos 0 = 1 and sin 0 = 0 of the shared double constant at
   `0x00A80990`, so the -0 signs are as listed), increment header byte 0 and OR 4 into header byte 1, and write it
   back (instance slot `+0x28`). After the loop (also when n = 0) set the holder's masks: h slot `+0x34(zbit)`,
   h slot `+0x3C(rbit)`.
9. Free both lists and the instance array, release h.

Example: a building loaded at (zoom 4, rot 0), requested (4, 1); same ids, no `0x4835C6A6`, no `0xE83A081D`:
every instance keeps its position, gets the yaw rotation of the current camera, and the holder's masks become
zoom bit 4 and rotation bit 1 (h slot `+0x34`/`+0x3C` are setters; whether they replace or OR is the model holder's
contract).

## 11. Lighting and dynamic textures (partial)

### 11.1 `DoLightingChanged` (`0x004930F0`) *[compat]*

1. c = (v, v, v) with v the dword at `0x00B216F8`, then the lighting object at global `0x00B43DDC` slot
   `+0x28(&c)` fills the three floats.
2. Compare with the stored colour `+0xC4..+0xCC`: if |c[k] - stored[k]| < 1e-4f (`0x00A8ED94`, float, strict) for
   all three k, return. A difference of exactly 1e-4f, or a NaN, counts as changed.
3. Store c into `+0xC4..+0xCC`, then `RebuildDynamicTexture(k)` (§11.2) for k = 0, 1, 2, 3, 4 in order.
4. If the occupant manager `+0x54` exists: build a counted occupant-type filter (0x18 bytes, vtables
   `0x00A8EDBC`/`0x00A8EDA4`, a type list at `+0x0C..+0x14`) holding the single type `0x278128A0`, get a fresh
   list (12-byte self-linked head from the list allocator service, global `0x00B207E4`, created on first use
   from framework class `0x3AE4BEA3`/interface `0x3AE4BEAB`), and fill it with occupant manager slot
   `+0x98(&list, 0, 0, filter, -1)`. For each listed occupant: AddRef and release its model holder (no other
   effect) and release the occupant (the list's reference). Release the filter, free the list nodes
   (`0x00413E60`) and the head (allocator slot `+0x10`).

Net observable effect: the five dynamic texture bindings are rebuilt when the light colour moves by at least
1e-4 in any channel; no model is re-flagged or re-sent to the renderer by this function. Whether materials bound
earlier by §12.4 see the new texels depends on whether §11.2 rebuilds the binding in place (not traced).

### 11.2 `RebuildDynamicTexture(k)` (`0x00492CC0`) — remainder

Builds texture binding `+0xD0 + 4k` (k = 0..4) from the light colour; not traced (1068 bytes, §20).

## 12. Model loading (partial)

### 12.1 `RemoveModels(o, t)` (`0x00493340`) *[compat]*

If `+0x50` is clear and the terrain-hiding filter `+0x60` accepts t: `DoTerrainHidingForOccupantCells(o, t, 0)`.
Then the occupant's model holder (slot `+0x4C`), when present, is AddRef'd, cleared (slot `+0x2C`) and released.

### 12.2 `LoadPropModels` (`0x00491A30`) and the shadow wrapper (`0x004919F0`) *[compat]*

`LoadPropModels(o, a, b, c)`: only when `OccupantUsesBuildingStyleModels(o)` (graphics-formats §4.2):
`LoadModelsCommon(o, a, b, c)`, and when that succeeds `CreateOccupantShadow(o, a, b)`; returns the
`LoadModelsCommon` result (false when the style test fails). `0x004919F0` is the same without the style test.

### 12.3 Small helpers *[compat]*

- `0x0048EE10`: assignment of a counted texture-binding reference (AddRef new, then release old; self-assignment
  and same-pointer do nothing).
- `0x0048F490`: if the object answers `0x097F6D98` (lot base texture occupant), call its slot `+0x28(0)`; release.

### 12.4 Model change notification and material texture swaps *[compat]*

All four walk the instances of a model holder h the same way: n = h slot `+0x0C`; when n > 0 an array of n
pointers (inline for n < 10, else `GlobalHeapAlloc(4n)`, freed at the end) is filled by h slot `+0x10(array, &n)`.
For each instance the material list is obtained by asking the instance for `0x26A68448` (skipped when it does not
answer; released after use); its slot `+0x0C` is the material count and slot `+0x18(j)` returns material j.
Material fields used: `+0x0B` texture state byte (0 none, 1 static, 2 swapped), `+0x10` texture name id, `+0x20`
swapped texture instance, `+0x24` counted texture binding, `+0x28..+0x2B` four sampler bytes, `+0x2C`, `+0x2D`,
`+0x2E` bytes, `+0x30` a dword.

- `ChangeModels(o, flag, mode)` (`0x0048F290`): mode 0 means 3. If flag is 0, call o slot `+0x1C` first (result
  ignored). h = o slot `+0x4C`; null: return false. Otherwise AddRef h, and for every instance for which the
  renderer (`+0x14`) slot `+0x6C(inst)` is true call renderer slot `+0x7C(inst, mode)`; release h; return true
  (also when h has no instances). Every caller in this owner passes flag `0x278128A0` and mode 2.
- `RestoreStaticTextures(h)` (`0x0048F380`, EAX = h): every material whose state is > 1 is set to state 1 and its
  binding `+0x24` is cleared (released when non-null). Returns true iff at least one material changed state.
- `ApplyTextureToModels(h, tex, a, b)` (`0x004906B0`, EAX = h, EBX = tex): for every material of every instance,
  unconditionally: state = 2; binding = tex (AddRef new, release old, nothing when equal); `+0x2C` = a; `+0x2E` = 1;
  `+0x30` = the current frame counter (`[[0x00B43CC4] + 0x24] + 0x44`, read once at entry); sampler bytes
  `+0x28..+0x2B` = (2, 2, 0, 0) when b is true, else (3, 3, 1, 1). No return value. The only caller passes a = 2,
  b = 0.
- `SwapTexturesByOffset(h, delta, a, b)` (`0x00490820`, EAX = h): needs the resource service (framework slot
  `+0x14`, class `0x1AC0E11A`, interface `0xFAC0E219`); without it returns false at once. For each material with
  state > 0 and name id `+0x10` != 0: state = 1 and the binding is released; the service's slot
  `+0x20(nameId, &pair)` maps the name to (group, instance); when the instance is 0 the material stays static.
  Otherwise the texture key (type `0x7AB50E44`, group, instance + delta) is loaded through service slot
  `+0x18(&key, &tex)`. On success: `+0x20` = instance + delta, state = 2, binding = tex (counted assignment
  `0x0048EE10`), `+0x2C` = a, `+0x2D` = 0, `+0x2E` = 0, `+0x30` = 1.0f, sampler bytes as above by b. The loaded
  reference is released afterwards. Returns true iff at least one load succeeded (a material whose load fails is
  left static with no texture). The only caller passes delta `0x8000`, a = 2, b = 1.

### 12.5 Lookup, geometry and container helpers *[compat]*

- `LotAtOccupant(o, &out)` (`0x0048F200`, RET 8): p = o's position (slot `+0x20`); the global `0x00B43CEC` slot
  `+0x17C(p.x, p.z, &cx, &cz)` converts it to a cell. When cx is in [`+0x30`, `+0x38`] and cz in
  [`+0x34`, `+0x3C`] (the city cell rect, inclusive, signed) and the lot manager (`+0x68`) slot
  `+0x18(cx, cz, 0)` returns a lot: out = that lot's slot `+0x8C`, return true. Otherwise out = null, false.
- `0x0048FF50(out, src, iid)`: interface pointer constructor. out = src's QueryInterface(iid) result (null on
  failure); the result ends up held once by out (the temporary's AddRef/Release pair cancels). out's previous
  value is not released (it is a constructor). Returns out.
- `RotateAboutAxis(c, p, angle, axis, out)` (`0x005D7710`, cdecl, used by `FindModelTransform`): picks the plane
  (i, j) = (1, 2) for axis 0, (0, 2) for axis 1, (0, 1) otherwise; du = p[i] - c[i], dv = p[j] - c[j];
  out[i] = float(float(cos a·du - sin a·dv) + c[i]); out[j] = float((sin a·du + cos a·dv) + c[j]) (the second
  sum is not rounded before the add). The third component of out is not written.
- `0x005D53B0(a, b)`: this `+0x58` = a, `+0x1F4` = b. `0x005D5360(a, b)`: for k = 0..4, this `+0x5C + 4k` =
  a[k] when a is non-null, this `+0x1F8 + 4k` = b[k] when b is non-null. `0x005D5A60(src)`: this `+0x30..+0x38`
  = src[0..2], `+0x3C..+0x44` = src[3..5] (a box). These three are setters of the object built by
  `LoadFoundationModels` (§12).
- `0x005D5500`: take one node from the free-list pool at `0x00B0A824` (pool header `0x00B0A828`: `+0x8` head,
  `+0xC` use count; an empty pool is refilled through `0x0090CF9A`), pop it and increment the count; returns it.
- Containers of 8-byte entries {counted reference, 3 bytes} (the `+0xE4` vector, §2): `0x00490A60` copy-construct
  a range, `0x00490AB0` construct n copies, `0x00490B00` assign a range (counted assignment, then the 3 bytes),
  `0x004928C0` grow-and-insert n copies (new capacity = old size + max(old size, n); prefix, the copies, suffix;
  a single copy for n = 1). `0x00490C10(n)`: vector of n zeroed 8-byte pairs. `0x00490B80(n)`: vector of n
  12-byte entries each (v, v, v) with v the dword at `0x00B216F8`. `0x00491210`: destructor freeing the buffer at
  `+0x0C`, then the base destructor `0x005E95C0`.

### 12.6 `LoadNetworkModels(o, zoom, rot)` (`0x0048F4D0`) and `LoadUndergroundConnectionModels(o, zoom, rot)` (`0x0048F5B0`) *[compat]*

Both RET 0xC and share one tail. A local list of up to 10 model instances and a count (0) are filled by the
occupant itself, with the arguments (zoom, rot, list, 10, &count):

| function | occupant type (o slot `+0x1C`) | filled by |
|---|---|---|
| network | `0x2890D4DE` | the object at o - 8 (the occupant's primary base), slot `+0x44` |
| network | any other | o's interface `0xA821EF94` (when o answers), slot `+0x84`; the interface is released |
| network | other, no `0xA821EF94` | nothing (count 0) |
| underground | `0x2A183167` or `0x8A13918A` | o slot `+0x90` |
| underground | any other | nothing (count 0) |

Tail: h = o's model holder (slot `+0x4C`, AddRef'd when present); for each filled instance in order: h slot
`+0x14(inst)` (add it to the holder), then release the list's reference. Release h. Returns count > 0. (A
holder-less occupant that fills instances is not guarded in the original.)

## 13. Terrain hiding (partial)

### 13.1 `DoTerrainHidingForOccupantCells(o, t, flag)` (`0x00492440`) *[compat]*

RET 0xC, returns a bool. r = o's cell rect (slot `+0x2C`: x0, z0, x1, z1, inclusive). If x0 > x1 or z0 > z1:
false. Clip r to the city cell rect `+0x30` with the intersection of §7.3 (`0x0048ED90`); empty: false.
mode = false, except when t = `0x68FD0C69` and flag is true and o answers `0xC8FCE101`: mode = (that interface's
slot `+0x24` = 2) (released after). If `DoTerrainHidingForCells(r, mode)` (`0x00491CD0`, remainder) is false,
return false. Otherwise make r end-exclusive (x1 + 1, z1 + 1) and ask the terrain (`+0x70`) to refresh it: slot
`+0x110(0, 0, &r, 0)`; return true. `RemoveModels` (§12.1) passes flag 0, so removal always uses mode false.

## 14. Making models (`AddModels` `0x00494390`, `AddModel` `0x004902E0`) *[outline; MM2]*

*Added 2026-10-22 (re-modelmaker).* The two bodies stay in the §20 remainder; this is what they do.

`AddModels(o, zoom, rotation, type, shown)`: an occupant outside the load rectangle (§7.2) gets no models.
Otherwise, by occupant type:

| Type | Loader | Visible at |
|---|---|---|
| building `0x278128A0` | building models; then completion (§9.5) and the state effect | the zoom |
| flora `0x74758926` | the same models path | the zoom |
| prop `0xA823821E` | prop models (§12.2); the state effect when its state is 2 | every zoom unless it uses building-style models |
| foundation `0x68FD0C69` | foundation models (`LoadFoundationModels` `0x00491040`), the state effect | every zoom unless building style |
| network `0xC772BF98` | network models (§12.6), the state effect | every zoom when the network occupant's slot 32 is false |
| power pole `0x2890D4DE` | network models, a shadow (`CreateOccupantShadow` `0x00491230`), the state effect | the zoom |
| pipe and subway connection `0x2A183167`, `0x8A13918A` | underground connection models (§12.6), the state effect | the zoom |
| retaining wall `0x895D1169` | the wall's own `AddRetainingWallModels` | every zoom |
| landfill `0x6A11BD2B`, base texture `0x497F6D9D` | the common loader (`LoadModelsCommon` `0x00490C70`) / the base-texture loader | the zoom / every zoom |

"Every zoom" is the mask `+0xB0`; "the zoom" is `1 << zoom`. The occupant's model handle (occupant slot 19) gets the
zoom mask (slot 13) and the rotation mask (slot 15).

`AddModel` makes one model instance: an animated model when the key names one, else the S3D resource of the key
(graphics-formats.md §4.2) through the resource manager; a missing key falls back to (`0x5AD0E817`, `0xBADB57F1`,
instance). A model whose extent × the scene's scale is below `kRenderModelThreshold` (render-properties float 9,
read at the table's `+0x184`) is dropped. Without a resource, a **placeholder** (`CreatePlaceHolderModelInstance`
`0x0048F920`) is made unless `+0xC1` (`-IgnoreMissingModelDataBugs:on`, §3.2) is set and the exemplar lacks
property `0x6A871B82`. The instance gets its transform (`FindModelTransform` `0x0048FA40`), the occupant's flag
`0x10` keeps only bit 0 of its flags, and it is added to the occupant's handle. Buildings set instance flag `0x40`,
flora `0x80`, props `0x20` (`0x22` without bit 0 when the exemplar has `0x4A70D491`).

When the occupant is shown and passes the view filter: the terrain under it is hidden (the hiding filter `+0x60`,
§13), its handle is made visible and its animation started.

## 15. The background loader (`cSC4ModelBackgroundLoader`) *[compat]*

*Added 2026-10-22 (re-modelmaker).* With `+0xB8` set (§3.2) the model maker owns one loader (0x188 bytes, `+0xBC`).
It loads the model resources of the load rectangle on a worker thread, in a spiral around the view, paced by
response curves from a tuning exemplar; the main thread turns the loaded cells into models a few at a time.

### 15.1 Lifetime

- **Constructor** (`0x0048E2B0`): the default curves of §15.2; vtable `0x00A8ED5C`. **Destructor** (`0x0048CDE0`):
  `Shutdown` when still initialised, then frees its curve arrays and containers.
- **`Init(modelMaker)`** (`0x0048E900`, once, byte `+0x04`): the speed `+0x184` := 3 when the global `0x00B43D44`
  slot 13 answers true, else the option `0x15` (`kBuildingLoaderSpeed`, `0x00B43CC4` slot 7); `ReadTunableValues`
  (§15.2); hold the model maker (`+0x18`), a new `cSC4ZoomRotationModelChangeOccupantFilter` (`+0xEC`, §6.4), the
  resource manager (`0x00B43CA8`, `+0x1C`), the occupant manager (`0x00B43D0C`, `+0x20`) and the simulator
  (`0x00B43D5C`, `+0x24`); `+0x0C`/`+0x10` := the city's `CellCountX` / `CellCountZ` − 1 (slots 93, 94); when the thread object `+0x60` is not
  running (its slot 11), clear the stop byte `+0x68`, give it `StaticBackgroundThreadFunction` (`0x0048E8F0`) with
  this object (slot 2) and call its slot 10 with 3 *[inferred: start]*. True.
- **`Shutdown`** (`0x0048CC90`, when initialised): clear `+0x04`; with a running thread: set the stop byte `+0x68`,
  wake it when suspended (state `+0x14` = 2: slots 5 and 7, state 1) and wait for it (slot 8 with 2,000,000); when
  the loader holds the simulator paused (`+0x29`), resume it (slot 13) and clear `+0x29`; `ClearResourceSet`;
  release `+0x1C`, `+0x20`, `+0x24`, `+0xEC`, `+0x18`. True.

### 15.2 Tuning (`ReadTunableValues` `0x0048CF10`)

The exemplar `0x6534284A / 0x7A4A8458 / 0x0B579C20 + s`, s = the speed of §15.1. Shipped (`SimCity_1.dat`):

| Property | Meaning | s = 0 "Background Loader" | s = 1, 2 "Aggressive" | s = 3 |
|---|---|---|---|---|
| `0x0B579C20` | query occupants per cell (`+0xF0`) | true | true | true |
| `0x0B579C21` | thread slice time limit vs. progress *[inferred: µs; MM1]* | 30000 … 1500 | 15000 … 1000 | 40000 … 1500 |
| `0x0B579C22` | sleep between slices vs. progress | 40000 … 400000 | 40000 … 50000 | 50000 … 400000 |
| `0x0B579C23` | main-thread time limit per step | 400000 … 50 | 2000000 … 10000 / 3000000 … 50000 | 50000 … 50 |
| `0x0B579C24` | main-thread minimum cells per step | 100 … 30 | 100 … 30 | 20 … 10 |
| `0x0B579C25`, `0x0B579C26` | cost of a cell with / without occupants | 1, 0.1 | 1, 0.1 | 1, 0.1 |
| `0x0B579C27` | budget per slice | 8 | 8 | 8 |
| `0x0B579C28` | minimum radius per zoom | 40, 30, 20, 10, 5 | 40, 30, 20, 10, 5 | 32, 16, 8, 4, 2 |
| `0x0B579C29` | radius factor per zoom | 0.2 … 0.0125 | 0.4 … 0.025 | 0.1 … 0.01 |
| `0x0B579C2A` | maximum loaded cells waiting | 1000 | 4000 / 5000 | 1000 |
| `0x0B579C2B` | progress at which the pause ends | 0.9 | 0.9 | 1.0 |

The curves are read at the task's progress (0 … 2) *[inferred: piecewise linear; MM1]*. Without the exemplar the
defaults are 16384, 65536, 400000, 100, 1, 0.1, 8, radii 8, 8, 6, 6, 4, factors 0, 1000 cells and 0.7.

### 15.3 Radius and tasks

- **Radius** (`ExpandLoadRect(rect, zoom)`, `0x0048CA40`): with m the system memory in MiB (§7.1; 0 when the call
  fails), `r := trunc(m × factor[zoom])`, then `r := max(r, minimum[zoom])` (unsigned); the rectangle grows by `r`
  on every side. §7.2 uses it in place of the model maker's own radius while the loader is on.
- **Start** (`StartLoadingForZoomAndRotation(zoom, rotation, &rect, &centre)`, `0x0048DAE0`, §10.1): re-read the
  tuning if the speed changed, drop the tasks and the loaded resources (`ClearResourceSet` `0x0048CC30`), keep the
  zoom and rotation, estimate the cell count from the screen size and a per-zoom factor, add one task (the
  rectangle and the centre) and post "pause" (§15.4); restart the thread if it had stopped.
- **Refocus** (`0x0048CB40(&rect, &centre)`, from the model maker's refocus step §10.2): under the lock (`+0x6C`),
  the first unfinished task gets progress 0, the new centre and rectangle; the waiting loaded cells are dropped; the
  spiral restarts at the centre, clamped into the rectangle. Without an unfinished task nothing changes.
- **Cancel** (`CancelAllLoadTasks` `0x0048CD60`, §10.1 while saving): under the lock, every task is freed, the
  waiting cells are dropped and `ClearResourceSet` runs.

### 15.4 The worker thread (`ThreadFunction` `0x0048E460`)

Under the loader's lock the worker runs the tasks: for each, while the slice time (curve `0x0B579C21` at the task's
progress) is not used up, `ContinueLoadTask` (`0x0048DCD0`) walks the next cells of a spiral around the centre inside
the rectangle, spending the budget (`0x0B579C27`) at 1 per cell with occupants and 0.1 per empty cell, loading the
resources of each occupant that passes the zoom/rotation filter (`LoadResourcesForOccupant` `0x0048D630`), and stops
when 1000 cells (`0x0B579C2A`) wait. It then posts commands (`PostCommand` `0x0048DA50`: append to `+0x7C`, set
`+0x88`): 0 cells ready, 1 task done, 2 all done, 3 pause, 4 resume (once progress exceeds `0x0B579C2B`). It sleeps
for curve `0x0B579C22`. The stop byte `+0x68` ends it.

### 15.5 The main thread

- **`ProcessBackgroundThreadToMainThreadCommunications`** (`0x0048E170`, on the animation tick `0x86AD10EE`, §5):
  nothing unless `+0x88` is set. Under the lock, the commands in order:
  - 0: when cells wait (`+0xC8` ≠ `+0xCC`), `ProcessLoadedCells` (`0x0048D800`): for each waiting cell,
    `LoadOccupantModelsForZoomAndRotation` (§10.4) for its occupants; after the minimum number of cells
    (`0x0B579C24`) it stops once the time limit (`0x0B579C23`) is passed. When it stops with cells left, the
    command list is kept as it is for the next tick (`0x0048E1D3`–`0x0048E1DB`).
  - 1: the model maker's "background load started" byte `+0xA4` is cleared (`0x0048EAD0`).
  - 2: when no task is left and the thread is not suspended: suspend it (slot 4), state `+0x14` := 2.
  - 3: unless already done, the simulator's `HiddenPause` (slot 10; budget-window.md §6) and `+0x29` := 1.
  - 4: when `+0x29` is set, `HiddenResume` (slot 13) and `+0x29` := 0.

  Then the list is emptied and `+0x88` cleared.
- **`Pause`** (`0x0048C960`) and **`Resume`** (`0x0048C970`) set and clear `+0x28` (§5, `0xCAD61E00` and
  `0xEAD61E16`); while it is set the worker ends its slice at once.

## 19. Conformance cases

| case | setup | expected |
|---|---|---|
| M1 QI | ask for `0xA9C741B3`, `1`, `0x452294AA`, `0xE4FDA3D4`, `0x12345678` | primary, primary, primary, primary+4, false (out untouched); count +1 per success |
| M2 Release | count 2, Release twice | first returns 1; second runs Shutdown and frees, returns 0 |
| M3 Read version | stream with version 1 | false; `+0x0A` set; nothing else read |
| M4 Save round trip | fields of §4 set, Write then Read into a fresh object | every field of the table equal; references resolved by IID |
| M5 Command line | `-backgroundLoader:OFF` / `:on` / `:maybe` | `+0xB8` = 0 / 1 / 1 (default kept) |
| M6 Defer count | flags true, true, false, false with 2 queued entries | replay once, on the last message only |
| M7 Created+destroyed | defer; queue Create(o), Shown(o), Destroy(o); release defer | nothing replayed for o |
| M8 Destroy only | defer; queue Shown(o), Destroy(o) | only Destroy replayed |
| M9 Insert covers change | defer; queue Insert(o, f=1), Changed(o) | only Insert replayed (as Shown) |
| M10 Move filter | Move message for a type outside the three of §8.8 | no instance touched |
| M11 Messages | each ID of §5 | DoMessage returns true; `0x4AB04CBE` has no effect |
| M12 Detail option | option `0x16` returns 5, then -3, then 1 | `0x00B216FC` = 2, 0, 1 |

Round 2 cases (§6, §7, §9, §10):

- M13 (§7.1): memory 512 MiB → radii 256, 128, 64, 32, 16; memory 8 MiB → 8, 8, 6, 6, 6 (minimums).
- M14 (§7): F(2.5) = 2, F(3.0) = 2 (odd integer), F(4.0) = 4; C(2.5) = 3, C(3.0) = 4, C(4.0) = 4.
- M15 (§7.2): visible rect (100, 40, 300, 200) m, zoom 2, radius 64, city 256 × 256, loader off → before growth
  (6, 2, 19, 13) (100/16 = 6.25, 40/16 = 2.5, 300/16 = 18.75, 200/16 = 12.5); after growth and clamping
  (0, 0, 83, 77).
- M16 (§7.3): city 64 × 64, L = (10, 20, 30, 40) → (0, 0, 63, 19), (0, 41, 63, 63), (0, 20, 9, 40),
  (31, 20, 63, 40).
- M17 (§6.4): zoom 2, rotation 1 → mask 0x110; a holder word 0x110 (or 0x7FC) is excluded, 0x10 or 0x100 is
  included; no holder → included.
- M18 (§9.2): lot state message with slot `+0x24` = A and `+0x38` = B → `ApplyStateEffectToModels(o, t, B, A)`.
- M19 (§10.1): oldZoom = newZoom and oldRot = newRot → no state change at all (`+0xC2` stays as it was).
- M20 (§9.1): x = 4 uses binding `+0xE0` (k = 4) and x = 5 uses `+0xD8` (k = 2), not k = x - 2; x = 1 and
  x = 7 both restore static textures; the result is true in every case.
- M21 (§9.5): c = 0.4, model height 20 → instances at p.y - 12.6 with bit 0 set; c = 1.0 → p.y, bit 0 cleared;
  c = NaN → treated as complete (p.y, bit 0 cleared, holder told false).
- M22 (§11.1): stored colour (0.5, 0.5, 0.5), new (0.5, 0.5, 0.50009) → no rebuild; new (0.5, 0.5, 0.5002) →
  all five dynamic textures rebuilt in order 0..4.
- M23 (§10.4): loaded (4, 0), requested (4, 0) with zoom bit and rotation bit present → returns at step 1;
  requested (3, 0) with equal id lists → nothing happens and the masks stay without bit 3.

re-modelmaker cases (§14, §15):

- M24 (§15.3): m = 2048 MiB, speed 0, zoom 4 → radius max(5, trunc(2048 × 0.0125)) = 25.
- M25 (§15.5): commands 3, 3, 4 in one tick → one `HiddenPause` and one `HiddenResume`; `+0x29` ends at 0.
- M26 (§15.5): command 0 with `ProcessLoadedCells` stopping on its time limit with cells left → the command list
  is unchanged and processed again on the next tick.
- M27 (§3.2, §15): `-backgroundLoader:off` → no loader is made and a zoom change reloads synchronously (§10.1
  step 6).
- M28 (§14): an occupant outside the load rectangle gets no models; a prop without building-style models gets the
  zoom mask `0x1F`, a building `1 << zoom`.

## 20. Function inventory (owner `cSC4ModelMaker`, tools/coverage.py --owners)

Specified: 20607 bytes; remainder (not yet specified or only outlined): 14098 bytes. The remainder is **not empty**,
so this spec does not yet claim the whole owner.

Remainder after round 3 (lane parked, 26 rows): model loading `AddModels` 0x494390, `AddModel` 0x4902E0,
`LoadModelsCommon` 0x490C70, `LoadFoundationModels` 0x491040, `FindModelTransform` 0x48FA40,
`CreatePlaceHolderModelInstance` 0x48F920; `CreateOccupantShadow` 0x491230; `DoTerrainHidingForCells` 0x491CD0;
`RebuildDynamicTexture` 0x492CC0; and the out-of-class helpers 0x445F10..0x447B00 (10 rows) and
0x8D08E0..0x8D0E10 (6 rows), not yet identified. re-modelmaker (2026-10-22) outlines `AddModels` and `AddModel`
in §14 (they stay in the remainder, MM2), names thirteen functions in the table below and moves `0x0048C960` and
`0x0048C970` to the loader owner (§15).

| address | bytes | name | section |
|---|---|---|---|
| `0x00445F10` | 134 | FUN_00445f10 | remainder |
| `0x004465A0` | 44 | FUN_004465a0 | remainder |
| `0x004465D0` | 78 | FUN_004465d0 | remainder |
| `0x00446760` | 52 | FUN_00446760 | remainder |
| `0x004469B0` | 578 | FUN_004469b0 | remainder |
| `0x00446D20` | 287 | FUN_00446d20 | remainder |
| `0x00446EB0` | 399 | FUN_00446eb0 | remainder |
| `0x00447750` | 86 | FUN_00447750 | remainder |
| `0x00447A00` | 139 | FUN_00447a00 | remainder |
| `0x00447B00` | 30 | FUN_00447b00 | remainder |
| `0x0048C960` | 5 | cSC4ModelBackgroundLoader::Pause | §15.5 (loader owner) |
| `0x0048C970` | 5 | cSC4ModelBackgroundLoader::Resume | §15.5 (loader owner) |
| `0x0048EA00` | 36 | FUN_0048ea00 | §7.6 |
| `0x0048EA30` | 36 | FUN_0048ea30 | §7.6 |
| `0x0048EA60` | 6 | cSC4ModelMaker::GetGZCLSID | §1 |
| `0x0048EA70` | 92 | cSC4ModelMaker::QueryInterface | §1 |
| `0x0048EB50` | 462 | cSC4ModelMaker::DetermineLoadRadiusAroundVisibleArea | §7.1 |
| `0x0048ED20` | 107 | cSC4ModelMaker::ComputeCityExclusionRects | §7.3 |
| `0x0048ED90` | 123 | FUN_0048ed90 | §7.3 |
| `0x0048EE10` | 59 | FUN_0048ee10 | §12.3 |
| `0x0048EF20` | 55 | FUN_0048ef20 | §8.3 |
| `0x0048EF60` | 301 | cSC4ModelMaker::Write | §4 |
| `0x0048F090` | 360 | cSC4ModelMaker::Read | §4 |
| `0x0048F200` | 142 | FUN_0048f200 | §12.5 |
| `0x0048F290` | 229 | cSC4ModelMaker::ChangeModels | §12.4 |
| `0x0048F380` | 268 | FUN_0048f380 | §12.4 |
| `0x0048F490` | 58 | FUN_0048f490 | §12.3 |
| `0x0048F4D0` | 217 | cSC4ModelMaker::LoadNetworkModels | §12.6 |
| `0x0048F5B0` | 169 | cSC4ModelMaker::LoadUndergroundConnectionModels | §12.6 |
| `0x0048F660` | 689 | cSC4ModelMaker::UpdateModelLoadRect | §7.2 |
| `0x0048F920` | 283 | cSC4ModelMaker::CreatePlaceHolderModelInstance | remainder |
| `0x0048FA40` | 1295 | cSC4ModelMaker::FindModelTransform | remainder |
| `0x0048FF50` | 65 | GZCOM_Interface<cISC4EffectsResource,cISC4EffectsCollection> | §12.5 |
| `0x0048FFF0` | 730 | cSC4ModelMaker::DoBuildingCompletionChanged | §9.5 |
| `0x004902E0` | 968 | cSC4ModelMaker::AddModel | remainder (outline §14) |
| `0x004906B0` | 358 | FUN_004906b0 | §12.4 |
| `0x00490820` | 565 | FUN_00490820 | §12.4 |
| `0x00490A60` | 70 | FUN_00490a60 | §12.5 |
| `0x00490AB0` | 68 | FUN_00490ab0 | §12.5 |
| `0x00490B00` | 117 | FUN_00490b00 | §12.5 |
| `0x00490B80` | 139 | FUN_00490b80 | §12.5 |
| `0x00490C10` | 90 | FUN_00490c10 | §12.5 |
| `0x00490C70` | 965 | cSC4ModelMaker::LoadModelsCommon | remainder |
| `0x00491040` | 462 | cSC4ModelMaker::LoadFoundationModels | remainder |
| `0x00491210` | 27 | FUN_00491210 | §12.5 |
| `0x00491230` | 1979 | cSC4ModelMaker::CreateOccupantShadow | remainder |
| `0x004919F0` | 63 | FUN_004919f0 | §12.2 |
| `0x00491A30` | 76 | cSC4ModelMaker::LoadPropModels | §12.2 |
| `0x00491A80` | 80 | FUN_00491a80 | §3.3 |
| `0x00491AD0` | 421 | cSC4ModelMaker::cSC4ModelMaker | §2 |
| `0x00491C80` | 8 | FUN_00491c80 | §1 |
| `0x00491C90` | 60 | cSC4ModelMaker::~cSC4ModelMaker | §3.4 |
| `0x00491CD0` | 1874 | cSC4ModelMaker::DoTerrainHidingForCells | remainder |
| `0x00492440` | 233 | cSC4ModelMaker::DoTerrainHidingForOccupantCells | §13.1 |
| `0x00492530` | 584 | cSC4ModelMaker::UnloadUnnecessaryModels | §7.4 |
| `0x00492780` | 311 | cSC4ModelMaker::RemoveModelsInappropriateForRotation | §7.5 |
| `0x004928C0` | 232 | FUN_004928c0 | §12.5 |
| `0x004929B0` | 27 | GZCOM_CreatecSC4ModelMaker | §1 |
| `0x004929D0` | 30 | cSC4ModelMaker::~cSC4ModelMaker (deleting destructor) | §3.4 |
| `0x004929F0` | 718 | cSC4ModelMaker::Shutdown | §3.3 |
| `0x00492CC0` | 1068 | cSC4ModelMaker::RebuildDynamicTexture | remainder |
| `0x004930F0` | 585 | cSC4ModelMaker::DoLightingChanged | §11.1 |
| `0x00493340` | 86 | cSC4ModelMaker::RemoveModels | §12.1 |
| `0x004933A0` | 729 | cSC4ModelMaker::ApplyStateEffect | §9.1 |
| `0x004936A0` | 96 | FUN_004936a0 | §8 |
| `0x00493700` | 25 | FUN_00493700 | §6.2 |
| `0x00493720` | 40 | FUN_00493720 | §6.2 |
| `0x00493750` | 124 | cSC4ModelMaker::DoOccupantDestroyed | §8.2 |
| `0x004937D0` | 536 | cSC4ModelMaker::DoOccupantMoved | §8.8 (outline) |
| `0x00493A00` | 386 | cSC4ModelMaker::DoOccupantHilighted | §8.9 |
| `0x00493B90` | 232 | cSC4ModelMaker::DoOccupantShown | §8.4 |
| `0x00493C80` | 221 | cSC4ModelMaker::ApplyStateEffectToModels | §9.3 |
| `0x00493D60` | 346 | cSC4ModelMaker::DoLotStateChanged | §9.2 |
| `0x00493EC0` | 37 | cSC4ModelMaker::FinalRelease | §1 |
| `0x00493EF0` | 924 | cSC4ModelMaker::CreateOccupantFilters | §6.1 |
| `0x00494290` | 113 | cSC4ModelMaker::DoOccupantInserted | §8.3 |
| `0x00494310` | 126 | cSC4ModelMaker::DoOccupantRemoved | §8.3 |
| `0x00494390` | 1112 | cSC4ModelMaker::AddModels | remainder (outline §14) |
| `0x004947F0` | 8 | cSC4ModelMaker::FinalRelease (thunk) | §1 |
| `0x00494800` | 248 | cSC4ModelMaker::Init | §3.1 |
| `0x00494900` | 99 | cSC4ModelMaker::DoPropStateChanged | §9.4 |
| `0x00494970` | 177 | cSC4ModelMaker::DoOccupantCreated | §8.1 |
| `0x00494A30` | 241 | cSC4ModelMaker::DoOccupantChanged | §8.5 |
| `0x00494B30` | 584 | cSC4ModelMaker::EnableDeferredOccupantOperations | §8.7 |
| `0x00494DA0` | 1463 | cSC4ModelMaker::DoPostCityInit | §3.2 |
| `0x00495360` | 193 | cSC4ModelMaker::OccupantFilterIterationFunction | §6.3 |
| `0x00495430` | 1329 | cSC4ModelMaker::LoadOccupantModelsForZoomAndRotation | §10.4 |
| `0x00495970` | 66 | cSC4ModelMaker::DoDeferOccupantOperations | §8.6 |
| `0x004959C0` | 313 | cSC4ModelMaker::DoViewFilterChange | §10.3 |
| `0x00495B00` | 642 | cSC4ModelMaker::DoZoomAndRotationChange | §10.1 |
| `0x00495D90` | 369 | cSC4ModelMaker::DoViewTranslationChange | §10.2 |
| `0x00495F10` | 1653 | cSC4ModelMaker::DoMessage | §5 |
| `0x005D5360` | 69 | FUN_005d5360 | §12.5 |
| `0x005D53B0` | 20 | FUN_005d53b0 | §12.5 |
| `0x005D5500` | 43 | FUN_005d5500 | §12.5 |
| `0x005D5A60` | 42 | FUN_005d5a60 | §12.5 |
| `0x005D7710` | 129 | FUN_005d7710 | §12.5 |
| `0x005EA370` | 174 | FUN_005ea370 | §6.2 |
| `0x005EA550` | 184 | FUN_005ea550 | §6.2 |
| `0x008D08E0` | 219 | FUN_008d08e0 | remainder |
| `0x008D09C0` | 160 | FUN_008d09c0 | remainder |
| `0x008D0A60` | 244 | FUN_008d0a60 | remainder |
| `0x008D0B60` | 247 | FUN_008d0b60 | remainder |
| `0x008D0C60` | 417 | FUN_008d0c60 | remainder |
| `0x008D0E10` | 442 | FUN_008d0e10 | remainder |
