# `cSC4ModelMaker`: occupants to model instances (behaviour spec)

Scope: the whole `cSC4ModelMaker` owner as tools/coverage.py assigns it (105 functions, 34,705 bytes; inventory
in §20). The model-ID rules it calls (`cSC4ModelMakerUtility`) are graphics-formats §4.2 and are not repeated.
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
returns false and leaves the out pointer untouched. *[quirk]* A null `this` on the `0xE4FDA3D4` path writes null
and returns false (`0x0048EABA`).

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
| `+0x78`..`+0x88` | uint32 ×5 | per-kind model budgets, 0x100 each | 0x100 |
| `+0xB0` | uint32 | bit k set for each budget k = 0..4 (so 0x1F) | 0x1F |
| `+0xB4` | uint32 | | 0x0F |
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
2. If `+0xB8`: create the background loader (0x188 bytes), AddRef it, `Init` it with this object, store `+0xBC`.
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
| `0x2977AE2F` | building completion | `DoBuildingCompletionChanged(slot +0x28)` (§9.3) |
| `0xE9C86DDB` | prop state | `DoPropStateChanged(slot +0x28, low byte of slot +0x38)` (§9.4) |
| `0x06CDB65B` | view filter | `DoViewFilterChange(slot +0x28, slot +0x3C)` (§10.3) |
| `0x49D968C4` | defer occupant operations | `0x00495970(slot +0x24 != 0)` (§8.8) |
| `0xA6B79601` | zoom/rotation change | p = slot `+0x28`, q = slot `+0x3C` (pairs). If `+0x44` is clear: `UpdateModelLoadRect`, then `DoZoomAndRotationChange(p0, p1, q0, q1)` (§10.1). If set: store q into `+0x48`/`+0x4C` |
| `0xA6B79602`, `0xA6B79620` | view update, post view init | if `+0x44` is clear and the View3D window exists: q = slot `+0x3C`; `+0x10C` = `+0x20` = q0, `+0x110` = `+0x24` = q1 |
| `0xA6B79611` | view translated | `UpdateModelLoadRect`, `DoViewTranslationChange` (§10.2) |
| `0xCAD61E00` | translation start | `+0xC3` = 1; if `+0xB8` and the loader exists, loader `+0x28` = 1 (`0x0048C960`; *[inferred]* a pause flag) |
| `0xEAD61E16` | translation end | `+0xC3` = 0, `UpdateModelLoadRect`, loader `+0x28` = 0 under the same test, `DoViewTranslationChange` |
| `0xC9DA96EA` | lighting changed | `DoLightingChanged` (§11) |
| `0xBA44EB51` | game idle | `UnloadUnnecessaryModels` (§7.4) |
| `0xC9F775BB` | options changed | re-read `0x00B216FC` (option `0x16`, clamped 0..2) |
| `0x86AD10EE` | animation tick (from the tick manager) | if the loader exists, `ProcessBackgroundThreadToMainThreadCommunications` |
| `0x4AB04CBE`, `0x4AB04CBF` | | subscribed but not handled |

*[quirk]* In `0xA6B79601` with `+0x44` set, only q is stored; p is dropped.
