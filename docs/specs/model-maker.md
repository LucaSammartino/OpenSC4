# `cSC4ModelMaker`: occupants to model instances (behaviour spec)

Scope: the whole `cSC4ModelMaker` owner as tools/coverage.py assigns it (88 functions, 32,418 bytes on 2026-10-23;
inventory in §20). The model-ID rules it calls (`cSC4ModelMakerUtility`) are graphics-formats §4.2 and are not
repeated. §14 specifies model making and §15 the whole background-loader owner `cSC4ModelBackgroundLoader` (22
functions, 7,977 bytes) (re-modelmaker, 2026-10-22; completed by re-modelmaker7, 2026-10-23).
The 3D renderer it feeds is render3d.md / view3d.md. Evidence: the GOG 1.1.641 exe through the Ghidra export
(`tools/re_query.py`) and decomp/; REA was unavailable for this lane ("No app is open"), so every branch and
constant below was read from the instructions at the cited addresses. re-modelmaker7 read the remaining bodies on
the instruction bytes of the Steam 1.1.641 exe (the same code as GOG, `docs/lanes/re-dates.md`).

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
start a transform record (§12.10) as identity rotation, zero translation, flags and counter 0
(`0x004938D3`–`0x00493950`); read the instance's current record into it (instance slot `+0x24`, so its rotation
is kept); set the translation to p, flag bit 0 and the change counter + 1; apply it (slot `+0x28`); and if the
renderer `+0x14` holds the instance (slot `+0x6C`) re-queue it (slot `+0x78`; render3d.md §5) (`0x0049395A`–
`0x004939B1`). Release the holder.

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

### 11.2 `RebuildDynamicTexture(k)` (`0x00492CC0`) *[compat]*

*Added 2026-10-23 (re-modelmaker7), read on the instructions; the export of this body is stack-mangled.*

1. Get two services:
   - **R**: class `0x056B906E`, interface `0x656B8EFC`;
   - **T**: the texture service of §12.4, class `0x1AC0E11A`, interface `0xFAC0E219`.
2. Load the texture `(type 0x7AB50E44, group T slot 11, instance I + +0x10C)` through R slot 3 with interface
   `0x1AB50C3F`. `I` is `0x294ECDD0` for k = 0–3 and `0x894FD650` for k = 4 (table `0x00A8EE0C`). `+0x10C` is the
   saved zoom mirror (§2). On failure the binding is left as it is and the function returns.
3. Read the texture's image descriptor (texture slot 3; words `+4` mip count, `+8` width, `+0x0C` height, `+0x14`
   format, `+0x18` sub-format, `+0x1C` mip-pointer array).
4. Choose a tint kernel and its bits per texel and block size (§11.3) from (format, sub-format):

   | Format | Kernel |
   |---|---|
   | (3, 1) | 8888, 32 bits |
   | (2, 1) | 888, 24 bits |
   | (3, 10) | 1555, 16 bits |
   | (3, 13) | 4444, 16 bits |
   | 7 or 8 | DXT1, 4 bits, block 4 |
   | 9 or 10 | DXT3/5, 8 bits, block 4 |
   | anything else | return without change |

5. Per mip: `w := max(w, block)` and `h := max(h, block)`; its size is `w × h` texels; then `w >>= 1`, `h >>= 1`.
   A buffer of `bits × total / 8` bytes holds every mip.
6. The colour is `0xFF000000 | c(+0xC4) << 16 | c(+0xC8) << 8 | c(+0xCC)`, the light colour of §11.1:
   - each channel is `c(f) = low8(v − (v >> 8))` with `v = bits(f + 49152.0f) − 0x47400000` (`0x00A8ED90`);
   - that is `round(256 f)`, with 256 giving 255.
7. Each mip is tinted from the source into its place in the buffer. The kernel gets a width of `size / block`, a
   height of `block` and zero row padding, so a whole mip is one run.
8. The new descriptor is the source's eight words with the mip-pointer array replaced. When binding `+0xD0 + 4k`
   exists it is updated in place (T slot 7, `(binding, descriptor)`); otherwise it is created (T slot 4,
   `(descriptor, &binding)`).
9. The buffers are then freed (T keeps its own copy *[inferred]*). The texture and the services are released.

### 11.3 The tint kernels (`0x008D08E0`–`0x008D0E10`) *[compat]*

Each multiplies a texel's channels by the colour's channels, rounding each product. Arguments: `(dst, dstPad,
src, srcPad, width, height, colour)`, with the pads in bytes after each row.

| Kernel | Address | Texel | Rule per channel |
|---|---|---|---|
| 8888 | `0x008D08E0` | 32-bit A,R,G,B | `(p × c + 128) >> 8` for all four bytes (alpha × the colour's `0xFF`) |
| 888 | `0x008D09C0` | three bytes | `(p × c + 128) >> 8` with colour bytes 0, 1, 2 |
| 4444 | `0x008D0A60` | 16-bit | each nibble `(p × c' + 8) >> 4`, `c'` the high nibble of the colour's matching byte |
| 1555 | `0x008D0B60` | 16-bit | the 5-bit fields scaled by the colour's top 5 bits with rounding; the alpha bit kept when the colour's alpha bit is set |
| DXT1 | `0x008D0C60` | 8-byte block | §11.3.1 |
| DXT3/5 | `0x008D0E10` | 16-byte block | the first 8 bytes (alpha) copied, then the colour half as DXT1 with the quirk below |

#### 11.3.1 DXT colour blocks

The two 565 endpoints `e0`, `e1` are tinted (5- and 6-bit fields scaled by the colour's top 5 bits, rounded) into
`t0`, `t1`, and the endpoint order and indices are adjusted:

| Source order | Tinted order | Stored endpoints | Indices |
|---|---|---|---|
| `e1 < e0` (four-colour mode) | `t0 < t1` | swapped (`t1`, `t0`) | every bit inverted (`~idx`) |
| `e1 < e0` | `t0 = t1` | (`t0`, `t1`) | 0 |
| `e1 < e0` | `t0 > t1` | (`t0`, `t1`) | unchanged |
| `e0 ≤ e1` (three-colour mode) | `t1 < t0` | swapped | `idx ^ (~idx >> 1 & 0x55555555)` |
| `e0 ≤ e1` | otherwise | (`t0`, `t1`) | unchanged |

*[quirk]* Three oddities:
- Inverting every index bit maps 0↔3 and 1↔2, not the 0↔1, 2↔3 that an endpoint swap needs.
- The DXT3/5 kernel computes the same swap, then stores the endpoints back in their unswapped order, keeping the
  remapped indices.
- The 16-bit kernels advance their rows by four bytes per texel. With the single-run calls of §11.2 this is never
  reached.

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

*§12.7–§12.12 added 2026-10-23 (re-modelmaker7), read on the instructions (the export of these bodies is partly
stack-mangled).*

### 12.7 `LoadModelsCommon(o, zoom, rot, type)` (`0x00490C70`, RET 0x10) *[compat]*

1. Inputs:
   - `ex` = o slot 6, the exemplar;
   - `p` = o's position (slot 8); for a foundation (`0x68FD0C69`) `p.y += 0.2f` (`0x00A8ED78`);
   - the occupant's orientation and flip byte from `cSC4ModelMakerUtility::GetOccupantOrientationAndState`.
2. `GetAllModelInstanceIDs` (graphics-formats §4.2) fills a vector of model instance ids, a parallel vector of
   12-byte offsets and the key's type and group.
3. For each id `i` in order (an id of 0 makes no model):
   - **make the model:** `AddModel(o, &(type, group, id), rot, orientation, flip, &p, offset)` (§12.11). The
     offset argument is null when `offset[i]` is exactly (0, 0, 0), else `&offset[i]`.
   - **result:** only the **first** id sets the function's result: true when it gave a model *[quirk]*.
   - **when a model was made, by type:**
     - building `0x278128A0`, **first model only**:
       - when o answers `0x87DDA3A9`: if building-development slot 50 (`+0xC8`) returns 0 for both `(2, a, b)` and
         `(1, a, b)` (two words of the key), and its slot 56 (`+0xE0`) is true for o, call its slot 59 (`+0xEC`)
         `(interface, 0)`; release the interface;
       - the instance flags (slot 13 → slot 14) gain `0x40`.
     - flora `0x74758926`: flags `| 0x80`.
     - prop `0xA823821E`: flags `| 0x20`; when ex has `0x4A70D491`, bit 0 is cleared and `0x2` set instead.
   - **then, any type:** when ex's boolean `0xE9F0FA86` is true, the instance's byte record (slot 15) gains bit
     `0x4` and is written back (slot 16).
4. Free both vectors and return the result.

### 12.8 `LoadFoundationModels(o, zoom, rot, type)` (`0x00491040`) *[compat]*

A foundation that uses building-style models (`FoundationOccupantUsesBuildingStyleModels`) goes to
`LoadModelsCommon`. Otherwise one `cSC4FoundationModel` is made:
1. Read from ex:
   - `a` = float `0xC911EDA0` and `b` = float `0xC911EDA1` (both default 16.0);
   - the 5-value arrays of `0x68FCFF37` (`P1`) and `0x68FCFF38` (`P2`), through the property's value data (slot
     103), or none;
   - `0x88FCC3E3`, which is read and never used *[quirk]*.
2. Allocate 0x278 bytes from the pool (`0x005D5500`) and construct with 1.
3. Set it up (§12.5 setters):
   - `0x005D5360(P1, P2)` and `0x005D53B0(a, b)`;
   - the box: o slot 10 `(&min, &max)` with max y + 0.2f, through `0x005D5A60`;
   - `AdjustHeightToTerrain(1)` and model slot 4 `(o)`.
4. Add the model to o's holder (slot 19, then holder slot 5) and release our reference. True.

### 12.9 `CreatePlaceHolderModelInstance(o, zoom, viewRot, rot, flip)` (`0x0048F920`, RET 0x14) *[compat]*

The placeholder for a missing model:
- **Base:** `s` = the larger of `x1 − x0` and `z1 − z0` of o's cell rect (slot 11, inclusive). The base is
  `0x2A0E0000` for `s ≤ 1`, `0x2A0C0000` for 2, `0x2A0B0000` for 3 and `0x2A0A0000` for 4 or more.
- **Rotation:** `r = (3 × viewRot + (flip ? 0 : rot)) % 4`, a C remainder (negative inputs give negative r).
- **Key:** `(0x5AD0E817, 0xBADB57F1, GetModelProperty1Instance(base, zoom, r))`.
- **Instance:** through the resource manager `+0x10` (slot 3, interface `0x9AD0E7E3`), then the resource's slot 5
  `(0, &instance)`, then the resource is released.

Returns the instance or null.

### 12.10 The transform record and `FindModelTransform(ex, flag, rot, p, offset, rec)` (`0x0048FA40`) *[compat]*

A transform record is 0x34 bytes:

| Offset | Field |
|---|---|
| `+0x00` | change counter (byte; each change adds 1) |
| `+0x01` | flags (byte): bit 0 translation set, bit 2 rotation set |
| `+0x04` | 3×3 rotation, row-major (`m[r][c]` at `+4 + 4(3r + c)`) |
| `+0x28` | translation (x, y, z) |

`FindModelTransform`:
1. **Yaw and position:**
   - `flag` false: `θ = −(the camera object +0x18's float at +0x1C)`. With an offset, the offset is rotated about
     the y axis by `−GetObjectYRotationAngle(rot)` (§12.5 `RotateAboutAxis`, centre 0) and added to `p`; its y is
     added unrotated.
   - `flag` true: `θ = GetObjectYRotationAngle(rot)` (view-utilities.md: `−rot × π/2`), and the offset is added
     **unrotated** *[quirk]*.
2. **Rotation:** a y rotation by θ, `[[cos θ, −0, sin θ], [0, 1, −0], [−sin θ, 0, cos θ]]`. It is computed with a
   tilt of 0.0 (`0x00A80990`), which gives the negative zeros; flag bit 2 is set and the counter + 1.
3. **Terrain tilt**, when ex's boolean `0x69F14D33` is true:
   1. Find the surface normal `n`. When there is an object at `p` (`0x00B43D54` slot 7 `(p, 0x1F4F, 1)`), `n` is
      that object's slot 57 `(x, z)` normalised. Otherwise, when the terrain's slot 77 is false at `(x, z)`, `n` is
      the terrain's slot 26 normal. With neither, nothing is tilted.
   2. When `n.y ≠ 1`, limit the tilt:
      - if `n.y < 0.866` (`0x00A8ED9C`) and `n.x > 0.5`: `n := (0.5, n.y, n.z) / sqrt(n.y² + n.z² + 0.25)`;
      - then, if still `n.y < 0.866` and `n.z > 0.5`: `n := (n.x, n.y, 0.5) / sqrt(n.y² + n.x² + 0.25)`.
      - Only positive x and z are limited *[quirk]*.
   3. `f` := (0, 0, 1) rotated about y by `−θ`; `r := normalise(n × f)`.
   4. Columns 0, 1, 2 of the rotation := `r`, `n`, `r × n`. Flag bit 2 set, counter + 1.
4. **Translation:** := the position of step 1; flag bit 0 set, counter + 1.

### 12.11 `AddModel(o, key, rot, orientation, flip, p, offset)` (`0x004902E0`, RET 0x1C) *[compat]*

1. When `GetOccupantModelClass(o)` is `0xEA123BE1` or `0xAA123BF9`, try an animated model:
   - the class `0xAA123BF9` forces `flip := 1`;
   - when `GetAnimatedModelFromResourceKey(key)` succeeds: the instance is its slot 0; then its slot 8
     `(+0x10C, +0x110)` (zoom and rotation mirrors) and slot 14 `(0.01f)`; when ex has `0x6A9EDD07`, its byte goes
     to slot 16;
   - no instance → the key's type and group become (`0x5AD0E817`, `0xBADB57F1`).
2. Otherwise, and after a failed animated try, load the S3D resource of the key (resource manager slot 3,
   interface `0x9AD0E7E3`). A failure with another type/group retries with (`0x5AD0E817`, `0xBADB57F1`) and the
   same instance.
3. **Size cut:** the resource's box (slot 7, from (1,1,1, 0,0,0)) gives the extent `max − min`. When the camera
   object's scale (`+0x18 +0x154`) × the extent's length is **below** `kRenderModelThreshold` (render-properties
   float `+0x184`), the resource is released and null returned. Otherwise the instance is the resource's slot 5
   `(0, &instance)`.
4. **Placeholder:** with no instance, unless `+0xC1` is set and ex lacks `0x6A871B82`:
   `CreatePlaceHolderModelInstance(o, +0x10C, +0x110, orientation, flip)` (§12.9). Still none → release, null.
5. **Transform:** `FindModelTransform(ex, flip, orientation, p, offset, rec)` from a fresh record, applied with
   instance slot 10.
6. When o's flag `0x10` (o slot 32) is set, the instance keeps only bit 0 of its flags (slot 13 → slot 14).
7. Instance slot 4 `(o)`, then add it to o's holder (slot 20 → holder slot 5; the holder is not null-checked).
   Release our reference and the resource, and return the instance.

`rot` is not used.

### 12.12 `CreateOccupantShadow(o, zoom, viewRot)` (`0x00491230`, RET 0xC) *[compat]*

**When there is no shadow** (false):
- ex has the boolean `0x029244DB` true;
- o is a building (`0x87DDA3A9`) and ex's boolean `0xEA3209F9` is true;
- o is a prop (`0xE9793A65`) and the render property `+0x1FC` (shadow detail) is below 3, or ex's `0x8A5E5DB8` is
  not true;
- o is none of these and not a power pole (`0x2890D4DE`).

**The shadow model:**
1. `rotation` = the building's slot 14, the prop's slot 9 or the flora's (`0xC83DD057`) slot 12; 0 for a pole.
2. For `z` from `zoom` down to 0: `id = GetModelInstanceID(o, z, (viewRot + 1) % 4, rotation, ex has 0xE83A081D,
   &type, &group)` (type and group default to `0x5AD0E817`, `0xBADB57F1`).
3. Load it (resource manager, `0x9AD0E7E3`) and make a model with the resource's slot 6 `(0, &model)`.
4. The model is usable when it has meshes (slot 3) and either the shadow detail is at least 4 or it has exactly
   one mesh.
5. When the resource's box height (`max y − min y`) × the camera scale is below the render property `+0x134`, the
   function returns false. The first usable model ends the search; none → false.

**The overlay:**
1. A record from `FindModelTransform(ex, ex has 0xE83A081D, rotation, o's position, none, rec)`, then rotated by
   **−π/2** about y. The double at `0x00A8EDD8` is `−π/2` rounded to binary32. Columns:
   `c0' = c0 cos A − c2 sin A` and `c2' = c0 sin A + c2 cos A`. Flag bit 2 set, counter + 1.
2. For each mesh `j` whose material state (`+0x0B`) is at least 1 and whose vertex count (header word `+6`) is at
   least 3:
   - copy its vertex positions (attribute 0) and texture coordinates (attribute 7, `0x008D0520`) into arrays;
   - ask the terrain view (terrain slot 66) for overlay manager 0 (view slot 18);
   - call its model-shadow add, slot 11 (overlay-manager.md §3.7), with the material's texture binding, the count,
     the positions, the coordinates, a world box and the record. The box is the occupant's (slot 10) below shadow
     detail 5; at 5 or more it is the model box transformed by the record (`0x007F8170`) *[inferred]*;
   - a returned id other than −1 is added to o's holder (slot 7).
3. True.

## 13. Terrain hiding

### 13.1 `DoTerrainHidingForOccupantCells(o, t, flag)` (`0x00492440`) *[compat]*

RET 0xC, returns a bool. r = o's cell rect (slot `+0x2C`: x0, z0, x1, z1, inclusive). If x0 > x1 or z0 > z1:
false. Clip r to the city cell rect `+0x30` with the intersection of §7.3 (`0x0048ED90`); empty: false.
mode = false, except when t = `0x68FD0C69` and flag is true and o answers `0xC8FCE101`: mode = (that interface's
slot `+0x24` = 2) (released after). If `DoTerrainHidingForCells(r, mode)` (§13.2) is false,
return false. Otherwise make r end-exclusive (x1 + 1, z1 + 1) and ask the terrain (`+0x70`) to refresh it: slot
`+0x110(0, 0, &r, 0)`; return true. `RemoveModels` (§12.1) passes flag 0, so removal always uses mode false.

### 13.2 `DoTerrainHidingForCells(r, mode)` (`0x00491CD0`, RET 8) *[compat]*

*Added 2026-10-23 (re-modelmaker7).* r is an inclusive cell rect. The terrain's cell flags are read with slot 82
(`+0x148(x, z)`) and written with slot 81 (`+0x144(x, z, flags)`). Their low bits: 0 and 1 hide the terrain
*[inferred: the two triangles of the cell]*, 2 is base-texture cover, 3 is a network bit.

1. **Mode:** when `mode` is set, every cell of r gets `(f & ~4) | 3`; only changed cells are written.
2. **Grid:** `g[(x − r.x0) × H + (z − r.z0)] = f & ~7` for every cell, with W × H the size of r (an empty r gives
   no grid). Bits 0–2 are rebuilt from scratch, so step 1's bits survive only through the change test of step 5.
3. **Occupants:** the occupant manager (`+0x54`) slot 40 lists the occupants between r's corners under the view
   filter `+0x58` (type −1). Each starts with `mask = 0xFFFFFFFF` and `b = 0`; by type (o slot 7):
   - **network** `0xC772BF98`:
     - when it answers `0xA821EF94` and its slot 8 is false for `0x4000`: `mask := ~8`;
     - slot 8 true for `0x1000000` → `b := 8`;
     - unless slot 8 is true for `0x8000000`: slot 8 false for `0x2000000` → `b |= 1`, false for `0x4000000` →
       `b |= 2`;
     - then, when it answers `0x4A10AFC2`, its slot 92 `(&u, &a, &c)`: `a` → `b |= 1`, `c` → `b |= 2` (`u` is
       unused).
   - **foundation** `0x68FD0C69`: when its `0xC8FCE101` slot 9 is 2 and the lot at its first cell (the lot manager
     `+0x68` slot 6 `(x0, z0, 0)`) has flags (slot 12) with bit 0 and either of bits 1 and 2: `b := 3`.
   - **base texture** `0x497F6D9D`: over the overlap with r, its `0x097F6D98` slot 7 `(i, j, &cover)` for each cell;
     true with `cover` set → `g |= 4`. The indices `i, j` are relative to r's corner, not to the occupant
     *[quirk]*. `mask` and `b` stay unchanged, so step 4 is skipped.
   - **pipe or subway connection** `0x2A183167`, `0x8A13918A`: `b := 3`.
4. **Apply:** unless `b = 0` and `mask = ~0`, every grid cell of the occupant's cell rect (o slot 11) clipped to r
   gets `g := (g & mask) | b`. Each occupant is released.
5. **Write back:** each cell whose terrain flags differ from `g` is written. Returns whether any was written.

## 14. Making models (`AddModels` `0x00494390`) *[compat]*

*Outlined 2026-10-22 (re-modelmaker); specified in full 2026-10-23 (re-modelmaker7).*

`AddModels(o, zoom, rot, type, shown)` (RET 0x14):
1. **Load rect:** false unless o's cell rect (slot 11) overlaps the load rectangle `+0x8C` (§7.2).
2. **Masks and holder:** `zoomMask := 1 << zoom`, `rotMask := 1 << rot`. h = o's holder (slot 20), AddRef'd when
   present.
3. **Models, by type.** "All" sets **both** masks to the "every zoom" and "every rotation" words `+0xB0` and
   `+0xB4`. The result `R` is the loader's result unless noted.

| Type | Steps | Masks |
|---|---|---|
| building `0x278128A0` | `0x004919F0` (§12.2); false skips the rest of the row; with `0x87DDA3A9`, `DoBuildingCompletionChanged` (§9.5); with a lot (`LotAtOccupant`, §12.5), `R :=` `ApplyStateEffect(o, type, lot, 0)` (§9.1) | zoom |
| flora `0x74758926` | `0x004919F0` | zoom |
| prop `0xA823821E` | `LoadPropModels` (§12.2); when its `0xE9793A65` slot 22 is 2 and it has a lot, `R :=` the state effect | all, unless `OccupantUsesBuildingStyleModels` |
| foundation `0x68FD0C69` | `LoadFoundationModels` (§12.8); false skips the rest of the row (masks included); with a lot, `ApplyStateEffectToModels(o, type, lot, 0)` (§9.3) | all, unless `FoundationOccupantUsesBuildingStyleModels` |
| network `0xC772BF98` | `LoadNetworkModels` (§12.6), `ApplyStateEffect(o, type, 0, 0)` | all when its `0xA821EF94` slot 32 is false |
| power pole `0x2890D4DE` | `LoadNetworkModels`, `CreateOccupantShadow` (§12.12), `ApplyStateEffect(o, type, 0, 0)` | zoom |
| pipe and subway connection `0x2A183167`, `0x8A13918A` | `LoadUndergroundConnectionModels` (§12.6), `ApplyStateEffect(o, type, 0, 0)` | zoom |
| retaining wall `0x895D1169` | `cSC4LotRetainingWallOccupant::AddRetainingWallModels(o, 1, 0)` | all |
| landfill `0x6A11BD2B` | `LoadModelsCommon` (§12.7) | zoom |
| base texture `0x497F6D9D` | `0x0048F490` (§12.3) | all |
| any other | nothing, `R` false | zoom |

4. **Shown:** when `shown`, o's slot 32 is true for 4 and the view filter (`+0x58` slot 3) accepts o:
   - when `+0x50` is clear and the hiding filter (`+0x60` slot 4) accepts the type:
     `DoTerrainHidingForOccupantCells(o, type, 1)` (§13.1);
   - h slot 8 `(1)` (made visible), then `SetOccupantAnimation(o, 1)` on the coordinator `+0x74`.
5. **Highlight:** when o's slot 16 is not 0, `DoOccupantHilighted(o, 0, that value)`.
6. **Masks:** h slot 13 `(zoomMask)`, h slot 15 `(rotMask)`; release h. Return `R`.

h is not null-checked in steps 4 and 6. A building's and a prop's `R` is the state effect's result when one runs
*[quirk]*.

Making the models of one occupant is §12.7 (`LoadModelsCommon`) with `AddModel` (§12.11); the transform is §12.10,
the placeholder §12.9.

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
- **Task record** (36 bytes, in the list `+0x74`, nodes of §15.6):

  | Offset | Field |
  |---|---|
  | `+0x00` | type: 0 loads (the only type ever made) |
  | `+0x04` | state: 0 new, 1 running, 2 done |
  | `+0x08` | progress (float) |
  | `+0x0C` | rectangle x0, z0, x1, z1 (cells, inclusive) |
  | `+0x1C` | centre x, z |

  The curves of §15.2 sit at `+0xF4` (`0x0B579C21`) to `+0x154` (`0x0B579C27`), 0x10 bytes apart; then the radii
  `+0x164` and factors `+0x170`, the cell limit `+0x17C`, the resume progress `+0x180` and the speed `+0x184`.
  `+0xF0` is `0x0B579C20`.
- **The spiral:** a `SpiralIterator` at `+0x98` (simgrid.md §7.4, order in plumbing.md §7.2), clockwise (mode
  byte `+0xB8` = 1 from the constructor), its box at `+0xA8` and ring counter at `+0xC4`. The waiting loaded
  cells are a vector of (x, z) pairs at `+0xC8`.
- **Start** (`StartLoadingForZoomAndRotation(zoom, rotation, &rect, &centre)`, `0x0048DAE0`, RET 0x10, §10.1),
  under the lock (`+0x6C`):
  1. Recompute the speed as in `Init`; when it changed, store it and re-read the tuning.
  2. Free every task. Then progress `+0x78` := 0, `ClearResourceSet` (`0x0048CC30`), and clear the waiting cells.
  3. Keep the zoom (`+0x8C`) and rotation (`+0x90`).
  4. `+0x94` := `trunc(w × f[zoom])`, with `w` the screen width from `0x00B43DD0` slot 11 (default 1024) and f =
     0.166015625, 0.0888671875, 0.044921875, 0.01953125, 0.009765625 (170, 91, 46, 20, 10 cells at 1024).
  5. Give the filter `+0xEC` the zoom and rotation (`SetZoomAndRotation`).
  6. When the global `0x00B43DD8` is set:
     - append the task {type 0, state 0, progress 0, rect, centre};
     - post 3 (§15.4);
     - when the thread is suspended (`+0x14` = 2), its slots 5 and 7, then `+0x14` := 1.
  7. Unlock.
- **Refocus** (`0x0048CB40(&rect, &centre)`, RET 8, from the model maker's refocus step §10.2), under the lock.
  Only the first task of **type 0** changes; with none, nothing does:
  - its progress := 0; its centre and rectangle := the new ones;
  - the waiting cells are cleared;
  - the spiral's box := the rectangle with `x1 := max(x1, x0)` and `z1 := max(z1, z0)`;
  - the spiral is constructed again at the centre (`0x0043FE10`).

  The task's state is kept, so a running task goes on from ring 0 of the new centre.
- **Cancel** (`CancelAllLoadTasks` `0x0048CD60`, §10.1 while saving): under the lock, every task is freed, the
  waiting cells are dropped and `ClearResourceSet` runs.
- **`ClearResourceSet`** (`0x0048CC30`): when the set `+0xD8` is not empty (count `+0xE8`), release every
  resource in it and clear it.

### 15.4 The worker thread (`ThreadFunction` `0x0048E460`) *[compat]*

*Specified 2026-10-23 (re-modelmaker7).* Each **slice**, until the stop byte `+0x68` is set:

1. **Lock and budget:** lock `+0x6C`, restart the slice timer `+0x30` and set `budget := C27(0)`.
2. **Run tasks** while there are tasks, the pause byte `+0x28` is clear and the last `ContinueLoadTask` returned
   true. Each pass takes the **first** task `t`:
   1. When `budget ≤ 0`: if the timer has passed `C21(t.progress)`, end the run; otherwise
      `budget := C27(t.progress)`.
   2. For a type-0 task:
      - **New task** (state 0): the spiral's box := t's rectangle with `x1 := max(x1, x0)` and
        `z1 := max(z1, z0)`; start the spiral at t's centre (inline constructor); state := 1.
      - Then `ContinueLoadTask(t, &budget)`.
      - **Progress:** `t.progress := trunc(sqrt((2 × ring)² − waiting)) / (+0x94 + 1)`, also stored in `+0x78`.
        `ring` is `+0xC4`, `waiting` the number of waiting cells, and the square root is taken of an int32.
   3. A type-1 task would set `+0x78` := 0; no code makes one, and such a task (or one of another type) would never
      finish.
   4. A task in state 2 is unlinked and freed.
3. **After the run:**
   - with no task left, post 2;
   - when the loader holds the simulator paused (`+0x29`) and `+0x78 > C2B`, post 4.

   Then unlock.
4. **Sleep** `trunc(min(C22(first task's progress), 5,000,000))`, or 1,000,000 with no task, through
   `cRZThread::SleepCurrentThread` *[inferred: µs; MM1]*.

On exit it locks, posts 4 and unlocks. Returns 0.

**`ContinueLoadTask(t, &budget)`** (`0x0048DCD0`, RET 8). While `budget > 0` and fewer than `C2A` (`+0x17C`) cells
wait, take the spiral's next cell. The end of the spiral sets `t.state := 2` and marks the task done. For each cell:
- **`+0xF0` set:** load the resources of every occupant at the cell under the filter `+0xEC` (occupant manager
  `+0x20` slot 40, the cell as both corners, type −1), then release them. The budget pays `C25(t.progress)` when
  there was at least one occupant, else `C26(t.progress)`.
- **`+0xF0` clear:** no occupants are queried and the budget pays **nothing** *[quirk]*. Only the cell limit or the
  spiral's end stops the walk.
- The cell is appended to the waiting cells.

Then `budget := −1`. Posts follow (`PostCommand` `0x0048DA50`, the vector append of §15.6; each post sets `+0x88`):
- 0 when any cell was appended;
- 1 and then 4 when the task finished.

Returns whether fewer than `C2A` cells wait.

**`LoadResourcesForOccupant(o, zoom, rot)`** (`0x0048D630`, RET 0xC):
1. Nothing when o's holder (slot 19) has both `1 << zoom` in its zoom mask (slot 12) and `1 << rot` in its rotation
   mask (slot 14).
2. Otherwise:
   - read ex's `0xE83A081D` and o's orientation and flip (`GetOccupantOrientationAndState`);
   - take the model ids of o for (zoom, rot) from `GetAllModelInstanceIDs`, starting from a 4-entry zero vector;
     key type and group default to (`0x5AD0E817`, `0xBADB57F1`).
3. For each id ≠ 0, the resource manager `+0x1C` slot 3 `(key, 0x456B8F1D, &res, 0, 0)`:
   - a resource already in the set `+0xD8` (a hash set by pointer, bucket `(res >> 2) % buckets`) is released;
   - otherwise it is inserted after a resize to count + 1, keeping the reference.

   The set holds one reference per resource until `ClearResourceSet`.

### 15.5 The main thread

- **`ProcessBackgroundThreadToMainThreadCommunications`** (`0x0048E170`, on the animation tick `0x86AD10EE`, §5):
  nothing unless `+0x88` is set. Under the lock, the commands in order:
  - 0: when cells wait (`+0xC8` ≠ `+0xCC`), `ProcessLoadedCells` (`0x0048D800`). The main-thread timer `+0x48`
    is restarted first. For each waiting cell in order:
    1. Each occupant at the cell under the filter `+0xEC` gets `LoadOccupantModelsForZoomAndRotation(o, +0x8C,
       +0x90)` (§10.4) and is released; `n := n + 1`.
    2. Read `limit = C23(p)` and `min = C24(p)`, with `p` the first task's progress, or 0 with no task.
    3. When `n > min` and the timer has passed `limit`, stop after this cell.

    The processed cells are erased from the front, and it returns whether none are left. When cells are left, the
    command list is kept as it is for the next tick (`0x0048E1D3`–`0x0048E1DB`).
  - 1: the model maker's "background load started" byte `+0xA4` is cleared (`0x0048EAD0`).
  - 2: when no task is left and the thread is not suspended: suspend it (slot 4), state `+0x14` := 2.
  - 3: unless already done, the simulator's `HiddenPause` (slot 10; budget-window.md §6) and `+0x29` := 1.
  - 4: when `+0x29` is set, `HiddenResume` (slot 13) and `+0x29` := 0.

  Then the list is emptied and `+0x88` cleared.
- **`Pause`** (`0x0048C960`) and **`Resume`** (`0x0048C970`) set and clear `+0x28` (§5, `0xCAD61E00` and
  `0xEAD61E16`); while it is set the worker ends its slice at once.

### 15.6 Container helpers *[compat]*

*Added 2026-10-23 (re-modelmaker7).*
- `0x0048CAE0(&task)` (RET 4): a 0x2C-byte task-list node with the 36-byte record copied whole to `+8`. The links
  at `+0` and `+4` are left for the caller, `StartLoadingForZoomAndRotation`, which appends the node at the list's
  end.
- `0x005BFE60(pos, &value, unused, n, atEnd)` (RET 0x14), named `BeginLoadTask` in the export: the grow path of a
  4-byte vector insert. It is shared with code outside the loader; the loader's posts use it when the command
  vector `+0x7C` is full.
  1. New capacity `size + max(size, n)`.
  2. Copy `[begin, pos)`, then `n` copies of `value`, then, unless `atEnd`, `[pos, end)`.
  3. Free the old array and set begin, end and capacity.

  The third argument is not read.

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

## 20. Function inventory (owners `cSC4ModelMaker` and `cSC4ModelBackgroundLoader`, tools/coverage.py --owners)

Both owners are specified in full (re-modelmaker7, 2026-10-23, on `5c0ff782`): `cSC4ModelMaker` 88 functions,
32,418 bytes; `cSC4ModelBackgroundLoader` 22 functions, 7,977 bytes. The round 3 remainder is closed:

- model loading `AddModels` (§14), `AddModel` (§12.11), `LoadModelsCommon` (§12.7), `LoadFoundationModels`
  (§12.8), `FindModelTransform` (§12.10), `CreatePlaceHolderModelInstance` (§12.9), `CreateOccupantShadow`
  (§12.12), `DoTerrainHidingForCells` (§13.2) and `RebuildDynamicTexture` (§11.2);
- the six out-of-class rows `0x008D08E0`..`0x008D0E10` are the tint kernels of §11.3;
- the ten rows `0x00445F10`..`0x00447B00` listed here before now belong to `cSC4AnimationCoordinator`
  (animation-coordinator.md) in the owner map and are no longer counted here;
- the loader's remaining bodies are §15.3–§15.6.

| address | bytes | name | section |
|---|---|---|---|
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
| `0x0048F920` | 283 | cSC4ModelMaker::CreatePlaceHolderModelInstance | §12.9 |
| `0x0048FA40` | 1295 | cSC4ModelMaker::FindModelTransform | §12.10 |
| `0x0048FF50` | 65 | GZCOM_Interface<cISC4EffectsResource,cISC4EffectsCollection> | §12.5 |
| `0x0048FFF0` | 730 | cSC4ModelMaker::DoBuildingCompletionChanged | §9.5 |
| `0x004902E0` | 968 | cSC4ModelMaker::AddModel | §12.11 |
| `0x004906B0` | 358 | FUN_004906b0 | §12.4 |
| `0x00490820` | 565 | FUN_00490820 | §12.4 |
| `0x00490A60` | 70 | FUN_00490a60 | §12.5 |
| `0x00490AB0` | 68 | FUN_00490ab0 | §12.5 |
| `0x00490B00` | 117 | FUN_00490b00 | §12.5 |
| `0x00490B80` | 139 | FUN_00490b80 | §12.5 |
| `0x00490C10` | 90 | FUN_00490c10 | §12.5 |
| `0x00490C70` | 965 | cSC4ModelMaker::LoadModelsCommon | §12.7 |
| `0x00491040` | 462 | cSC4ModelMaker::LoadFoundationModels | §12.8 |
| `0x00491230` | 1979 | cSC4ModelMaker::CreateOccupantShadow | §12.12 |
| `0x004919F0` | 63 | FUN_004919f0 | §12.2 |
| `0x00491A30` | 76 | cSC4ModelMaker::LoadPropModels | §12.2 |
| `0x00491A80` | 80 | FUN_00491a80 | §3.3 |
| `0x00491AD0` | 421 | cSC4ModelMaker::cSC4ModelMaker | §2 |
| `0x00491C80` | 8 | FUN_00491c80 | §1 |
| `0x00491C90` | 60 | cSC4ModelMaker::~cSC4ModelMaker | §3.4 |
| `0x00491CD0` | 1874 | cSC4ModelMaker::DoTerrainHidingForCells | §13.2 |
| `0x00492440` | 233 | cSC4ModelMaker::DoTerrainHidingForOccupantCells | §13.1 |
| `0x00492530` | 584 | cSC4ModelMaker::UnloadUnnecessaryModels | §7.4 |
| `0x00492780` | 311 | cSC4ModelMaker::RemoveModelsInappropriateForRotation | §7.5 |
| `0x004928C0` | 232 | FUN_004928c0 | §12.5 |
| `0x004929B0` | 27 | GZCOM_CreatecSC4ModelMaker | §1 |
| `0x004929D0` | 30 | cSC4ModelMaker::~cSC4ModelMaker (deleting destructor) | §3.4 |
| `0x004929F0` | 718 | cSC4ModelMaker::Shutdown | §3.3 |
| `0x00492CC0` | 1068 | cSC4ModelMaker::RebuildDynamicTexture | §11.2 |
| `0x004930F0` | 585 | cSC4ModelMaker::DoLightingChanged | §11.1 |
| `0x00493340` | 86 | cSC4ModelMaker::RemoveModels | §12.1 |
| `0x004933A0` | 729 | cSC4ModelMaker::ApplyStateEffect | §9.1 |
| `0x004936A0` | 96 | FUN_004936a0 | §8 |
| `0x00493750` | 124 | cSC4ModelMaker::DoOccupantDestroyed | §8.2 |
| `0x004937D0` | 536 | cSC4ModelMaker::DoOccupantMoved | §8.8 |
| `0x00493A00` | 386 | cSC4ModelMaker::DoOccupantHilighted | §8.9 |
| `0x00493B90` | 232 | cSC4ModelMaker::DoOccupantShown | §8.4 |
| `0x00493C80` | 221 | cSC4ModelMaker::ApplyStateEffectToModels | §9.3 |
| `0x00493D60` | 346 | cSC4ModelMaker::DoLotStateChanged | §9.2 |
| `0x00493EC0` | 37 | cSC4ModelMaker::FinalRelease | §1 |
| `0x00493EF0` | 924 | cSC4ModelMaker::CreateOccupantFilters | §6.1 |
| `0x00494290` | 113 | cSC4ModelMaker::DoOccupantInserted | §8.3 |
| `0x00494310` | 126 | cSC4ModelMaker::DoOccupantRemoved | §8.3 |
| `0x00494390` | 1112 | cSC4ModelMaker::AddModels | §14 |
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
| `0x008D08E0` | 219 | FUN_008d08e0 | §11.3 |
| `0x008D09C0` | 160 | FUN_008d09c0 | §11.3 |
| `0x008D0A60` | 244 | FUN_008d0a60 | §11.3 |
| `0x008D0B60` | 247 | FUN_008d0b60 | §11.3 |
| `0x008D0C60` | 417 | FUN_008d0c60 | §11.3.1 |
| `0x008D0E10` | 442 | FUN_008d0e10 | §11.3.1 |

`cSC4ModelBackgroundLoader`:

| address | bytes | name | section |
|---|---|---|---|
| `0x0048C960` | 5 | cSC4ModelBackgroundLoader::Pause | §15.5 |
| `0x0048C970` | 5 | cSC4ModelBackgroundLoader::Resume | §15.5 |
| `0x0048CA40` | 155 | cSC4ModelBackgroundLoader::ExpandLoadRect | §15.3 |
| `0x0048CAE0` | 86 | FUN_0048cae0 (task node) | §15.6 |
| `0x0048CB40` | 240 | FUN_0048cb40 (refocus) | §15.3 |
| `0x0048CC30` | 94 | cSC4ModelBackgroundLoader::ClearResourceSet | §15.3 |
| `0x0048CC90` | 193 | cSC4ModelBackgroundLoader::Shutdown | §15.1 |
| `0x0048CD60` | 128 | cSC4ModelBackgroundLoader::CancelAllLoadTasks | §15.3 |
| `0x0048CF10` | 1811 | cSC4ModelBackgroundLoader::ReadTunableValues | §15.2 |
| `0x0048D630` | 449 | cSC4ModelBackgroundLoader::LoadResourcesForOccupant | §15.4 |
| `0x0048D800` | 569 | cSC4ModelBackgroundLoader::ProcessLoadedCells | §15.5 |
| `0x0048DA50` | 77 | cSC4ModelBackgroundLoader::PostCommand | §15.4 |
| `0x0048DAE0` | 487 | cSC4ModelBackgroundLoader::StartLoadingForZoomAndRotation | §15.3 |
| `0x0048DCD0` | 1177 | cSC4ModelBackgroundLoader::ContinueLoadTask | §15.4 |
| `0x0048E170` | 295 | cSC4ModelBackgroundLoader::ProcessBackgroundThreadToMainThreadCommunications | §15.5 |
| `0x0048E2B0` | 432 | cSC4ModelBackgroundLoader::cSC4ModelBackgroundLoader | §15.1 |
| `0x0048E460` | 1164 | cSC4ModelBackgroundLoader::ThreadFunction | §15.4 |
| `0x0048E8F0` | 12 | cSC4ModelBackgroundLoader::StaticBackgroundThreadFunction | §15.1 |
| `0x0048E900` | 255 | cSC4ModelBackgroundLoader::Init | §15.1 |
| `0x0048EAD0` | 10 | FUN_0048ead0 (command 1) | §15.5 |
| `0x0048EAE0` | 106 | RefocusLoadToNewViewLocation (model-maker method) | §10.2 |
| `0x005BFE60` | 227 | cSC4ModelBackgroundLoader::BeginLoadTask (vector insert) | §15.6 |
