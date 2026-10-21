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

## 9. State-change handlers (partial)

### 9.4 Prop state (`DoPropStateChanged` `0x00494900`, message `0xE9C86DDB`) *[compat]*

Let o = (message object) slot `+0x0C` (its occupant). If o's exemplar (o slot `+0x18`) has property
`0x4A70D491`: `ChangeModels(o, 0, 2)` and return. Otherwise `RemoveModels(o, t)` with t = o.GetType(), then
`AddModels(o, zoom +0x20, rotation +0x24, t, true)`. *[quirk]* The state byte the message carries (second
argument) is never read (`0x00494900`–`0x00494960`); `ChangeModels` always gets the constants 0 and 2.

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

## 20. Function inventory (owner `cSC4ModelMaker`, tools/coverage.py --owners)

Specified: 7777 bytes; remainder (not yet specified or only outlined): 26928 bytes. The remainder is **not empty**,
so this spec does not yet claim the whole owner.

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
| `0x0048C960` | 5 | FUN_0048c960 | §5 |
| `0x0048C970` | 5 | FUN_0048c970 | §5 |
| `0x0048EA00` | 36 | FUN_0048ea00 | remainder |
| `0x0048EA30` | 36 | FUN_0048ea30 | remainder |
| `0x0048EA60` | 6 | FUN_0048ea60 | §1 |
| `0x0048EA70` | 92 | FUN_0048ea70 | §1 |
| `0x0048EB50` | 462 | cSC4ModelMaker::DetermineLoadRadiusAroundVisibleArea | remainder |
| `0x0048ED20` | 107 | cSC4ModelMaker::ComputeCityExclusionRects | remainder |
| `0x0048ED90` | 123 | FUN_0048ed90 | remainder |
| `0x0048EE10` | 59 | FUN_0048ee10 | §12.3 |
| `0x0048EF20` | 55 | FUN_0048ef20 | §8.3 |
| `0x0048EF60` | 301 | FUN_0048ef60 | §4 |
| `0x0048F090` | 360 | cSC4ModelMaker::Read | §4 |
| `0x0048F200` | 142 | FUN_0048f200 | remainder |
| `0x0048F290` | 229 | cSC4ModelMaker::ChangeModels | remainder |
| `0x0048F380` | 268 | FUN_0048f380 | remainder |
| `0x0048F490` | 58 | FUN_0048f490 | §12.3 |
| `0x0048F4D0` | 217 | cSC4ModelMaker::LoadNetworkModels | remainder |
| `0x0048F5B0` | 169 | cSC4ModelMaker::LoadUndergroundConnectionModels | remainder |
| `0x0048F660` | 689 | cSC4ModelMaker::UpdateModelLoadRect | remainder |
| `0x0048F920` | 283 | cSC4ModelMaker::CreatePlaceHolderModelInstance | remainder |
| `0x0048FA40` | 1295 | cSC4ModelMaker::FindModelTransform | remainder |
| `0x0048FF50` | 65 | GZCOM_Interface<cISC4EffectsResource,cISC4EffectsCollection> | remainder |
| `0x0048FFF0` | 730 | cSC4ModelMaker::DoBuildingCompletionChanged | remainder |
| `0x004902E0` | 968 | cSC4ModelMaker::AddModel | remainder |
| `0x004906B0` | 358 | FUN_004906b0 | remainder |
| `0x00490820` | 565 | FUN_00490820 | remainder |
| `0x00490A60` | 70 | FUN_00490a60 | remainder |
| `0x00490AB0` | 68 | FUN_00490ab0 | remainder |
| `0x00490B00` | 117 | FUN_00490b00 | remainder |
| `0x00490B80` | 139 | FUN_00490b80 | remainder |
| `0x00490C10` | 90 | FUN_00490c10 | remainder |
| `0x00490C70` | 965 | cSC4ModelMaker::LoadModelsCommon | remainder |
| `0x00491040` | 462 | cSC4ModelMaker::LoadFoundationModels | remainder |
| `0x00491210` | 27 | FUN_00491210 | remainder |
| `0x00491230` | 1979 | cSC4ModelMaker::CreateOccupantShadow | remainder |
| `0x004919F0` | 63 | FUN_004919f0 | §12.2 |
| `0x00491A30` | 76 | cSC4ModelMaker::LoadPropModels | §12.2 |
| `0x00491A80` | 80 | FUN_00491a80 | §3.3 |
| `0x00491AD0` | 421 | cSC4ModelMaker::cSC4ModelMaker | §2 |
| `0x00491C80` | 8 | FUN_00491c80 | §1 |
| `0x00491C90` | 60 | FUN_00491c90 | §3.4 |
| `0x00491CD0` | 1874 | cSC4ModelMaker::DoTerrainHidingForCells | remainder |
| `0x00492440` | 233 | cSC4ModelMaker::DoTerrainHidingForOccupantCells | remainder |
| `0x00492530` | 584 | cSC4ModelMaker::UnloadUnnecessaryModels | remainder |
| `0x00492780` | 311 | cSC4ModelMaker::RemoveModelsInappropriateForRotation | remainder |
| `0x004928C0` | 232 | FUN_004928c0 | remainder |
| `0x004929B0` | 27 | GZCOM_CreatecSC4ModelMaker | §1 |
| `0x004929D0` | 30 | FUN_004929d0 | §3.4 |
| `0x004929F0` | 718 | cSC4ModelMaker::Shutdown | §3.3 |
| `0x00492CC0` | 1068 | cSC4ModelMaker::RebuildDynamicTexture | remainder |
| `0x004930F0` | 585 | cSC4ModelMaker::DoLightingChanged | remainder |
| `0x00493340` | 86 | cSC4ModelMaker::RemoveModels | §12.1 |
| `0x004933A0` | 729 | FUN_004933a0 | remainder |
| `0x004936A0` | 96 | FUN_004936a0 | §8 |
| `0x00493700` | 25 | FUN_00493700 | remainder |
| `0x00493720` | 40 | FUN_00493720 | remainder |
| `0x00493750` | 124 | cSC4ModelMaker::DoOccupantDestroyed | §8.2 |
| `0x004937D0` | 536 | cSC4ModelMaker::DoOccupantMoved | §8.8 (outline) |
| `0x00493A00` | 386 | cSC4ModelMaker::DoOccupantHilighted | remainder |
| `0x00493B90` | 232 | cSC4ModelMaker::DoOccupantShown | §8.4 |
| `0x00493C80` | 221 | cSC4ModelMaker::ApplyStateEffectToModels | remainder |
| `0x00493D60` | 346 | cSC4ModelMaker::DoLotStateChanged | remainder |
| `0x00493EC0` | 37 | cSC4ModelMaker::FinalRelease | §1 |
| `0x00493EF0` | 924 | cSC4ModelMaker::CreateOccupantFilters | remainder |
| `0x00494290` | 113 | FUN_00494290 | §8.3 |
| `0x00494310` | 126 | FUN_00494310 | §8.3 |
| `0x00494390` | 1112 | cSC4ModelMaker::AddModels | remainder |
| `0x004947F0` | 8 | FUN_004947f0 | §1 |
| `0x00494800` | 248 | cSC4ModelMaker::Init | §3.1 |
| `0x00494900` | 99 | cSC4ModelMaker::DoPropStateChanged | §9.4 |
| `0x00494970` | 177 | cSC4ModelMaker::DoOccupantCreated | §8.1 |
| `0x00494A30` | 241 | cSC4ModelMaker::DoOccupantChanged | §8.5 |
| `0x00494B30` | 584 | cSC4ModelMaker::EnableDeferredOccupantOperations | §8.7 |
| `0x00494DA0` | 1463 | cSC4ModelMaker::DoPostCityInit | §3.2 |
| `0x00495360` | 193 | cSC4ModelMaker::OccupantFilterIterationFunction | remainder |
| `0x00495430` | 1329 | cSC4ModelMaker::LoadOccupantModelsForZoomAndRotation | remainder |
| `0x00495970` | 66 | FUN_00495970 | §8.6 |
| `0x004959C0` | 313 | cSC4ModelMaker::DoViewFilterChange | remainder |
| `0x00495B00` | 642 | cSC4ModelMaker::DoZoomAndRotationChange | remainder |
| `0x00495D90` | 369 | cSC4ModelMaker::DoViewTranslationChange | remainder |
| `0x00495F10` | 1653 | cSC4ModelMaker::DoMessage | §5 |
| `0x005D5360` | 69 | FUN_005d5360 | remainder |
| `0x005D53B0` | 20 | FUN_005d53b0 | remainder |
| `0x005D5500` | 43 | FUN_005d5500 | remainder |
| `0x005D5A60` | 42 | FUN_005d5a60 | remainder |
| `0x005D7710` | 129 | FUN_005d7710 | remainder |
| `0x005EA370` | 174 | FUN_005ea370 | remainder |
| `0x005EA550` | 184 | FUN_005ea550 | remainder |
| `0x008D08E0` | 219 | FUN_008d08e0 | remainder |
| `0x008D09C0` | 160 | FUN_008d09c0 | remainder |
| `0x008D0A60` | 244 | FUN_008d0a60 | remainder |
| `0x008D0B60` | 247 | FUN_008d0b60 | remainder |
| `0x008D0C60` | 417 | FUN_008d0c60 | remainder |
| `0x008D0E10` | 442 | FUN_008d0e10 | remainder |
