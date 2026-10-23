# Spec: the demolition utility (`cSC4Demolition`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was written by
the analysis team from observing the original program and its data files. Implementers must work from this spec,
the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled or
disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (components §3.6, the `Init` order §3.7, messages §5, streams §2),
[zones.md](zones.md) (`PlaceZone` and `OnDezone` call this utility; zone types §2), [lot-developer.md](lot-developer.md)
(`StartDemolishLot` / `EndDemolishLot` §7, `AttemptAreaLevelling`), [landfill.md](landfill.md) (§4.5.3, the
`ModifyTerrainHeight` call), [tract-developer.md](tract-developer.md), [power.md](power.md),
[building-development.md](building-development.md) (`GetCostOfOccupantDemolition`), [neighbors.md](neighbors.md)
(`0xAA7A014D`, `DeleteOccupantConnection`), [flora.md](flora.md).

## 0. Summary

The demolition utility is a **stateless service** with no simulation agent. Other components (the bulldozer tool,
the zone manager, the lot and tract developers, the power simulator, the building-development simulator, the
network tool and one ordinance) ask it to price or perform demolition:

- `DemolishRegion` walks a cell region, decides which lots and loose occupants may go, prices them, and (when
  committing) removes them, clears or explodes the lots, dezones or re-parcels them, and removes networks in one
  batch.
- `DemolishLots` turns a list of lots into a region and calls `DemolishRegion`.
- `DemolishOccupant` prices or removes one occupant, by occupant type.
- `ModifyTerrainHeight` levels terrain unless immovable occupants or locked lots are in the way.
- Two effect methods start the dust and debris visual effects and the demolition sounds.

It **never touches the funds**. It returns a cost (int64) through an out pointer; the caller decides whether to
charge it (the bulldozer charges the *preview* cost, §9). Its save record is two bytes (`u16` version 1) and holds
nothing. It draws **no random numbers**.

Implementation size: small to medium. The record and slots 0–5 are trivial; `DemolishRegion` with its filter is the
bulk (about 250 lines of logic), `DemolishOccupant` about 100, `ModifyTerrainHeight` about 80, the effects about 80
(presentation only).

## 1. Identity

| Item | Value |
|---|---|
| Class | `cSC4Demolition`, class ID `0x0990C38A` |
| Component | city component **2** (foundations §3.6), interface `cISC4Demolition` (`0x26DDE2EA`); `cISC4City` keeps it at `+0x120` and creates it in `CreateCityComponents` (`0x0045B03F`) |
| Global | `0x00B43D00` (the `SL` cache, set at city init) |
| Factory | `0x00466E40`, registered by the game COM director (`cSC4COMDirector`); allocates **0x34** bytes from the global heap (`0x005E55E0`, allocators.md) and constructs in place |
| QI (`0x004664B0`) | `1` and `0x26DDE2EA` → `+0`; `cIGZMessageTarget2` (`0x452294AA`) → `+4`; `cIGZSerializable` (`0xE4FDA3D4`) → `+8`; anything else → false. No `cIGZMessageTarget` (`0xE98B2F57`) |
| Vtables | primary `0x00A88004` (13 slots, §8); message target `0x00A87FF4` (`DoMessage` `0x00466FF0`); serializable `0x00A87FDC` (`Write` `0x0042E440`, `Read` `0x006B8A70`, `GetGZCLSID` `0x00466490`) |
| Reference count | `+0xC`; `AddRef` (`0x004664A0`) increments and returns the new count; `Release` (`0x00466950`) decrements while the count is > 1, otherwise calls `Shutdown` and the deleting destructor (slot 12) and returns 0 |

`Write` and `Read` are shared code: `0x0042E440` and `0x006B8A70` serve several classes with a version-1-only record.

## 2. State

| Offset | Field | Initial | Saved |
|---|---|---|---|
| `+0x0C` | reference count | 0 | no |
| `+0x10` | `initialized` (byte) | 0 | no |
| `+0x14`, `+0x18` | city cell counts X, Z (city slots 93, 94, read at `Init`) | 0 | no |
| `+0x1C` | the **demolition filter** Φ (a `cSC4DemolitionOccupantFilter`, §4.2), created at `Init` | null | no |
| `+0x20` | city | null | no |
| `+0x24` | lot developer (`buildingDevelopment.GetLotDeveloper()`, slot 36) | null | no |
| `+0x28` | occupant manager (`0x00B43D0C`) | null | no |
| `+0x2C` | terrain (`0x00B43CF4`) | null | no |
| `+0x30` | the region of the `DemolishRegion` call in progress (cleared on exit) | null | no |

**Cell counts are city slots 93 and 94 (DQ2 — MATCHES, confirmed 2026-10-16 by re-answers31).** The wording above
is correct and is **not** contradicted by `docs/abi/interfaces.tsv`, whose rows for `cISC4City` slots 93 and 94 are
*misnamed* (`GetLightingManager` / `GetDrawContext` — those names were picked up from byte-identical folded getter
bodies in other classes). The compiled evidence:

| Slot | Vtable entry | Body | Returns |
|---|---|---|---|
| 88 | `SetSize` `0x004593B0` | stores `param_2`/`param_3` to `+0x100`/`+0x104`, then `round(p × k)` to `+0x118`/`+0x11C` | — |
| 89 | `SizeX` `0x0045A370` | `FLD dword ptr [ECX+0x100]` | `f32` |
| 90 | `SizeZ` `0x0045A380` | `FLD dword ptr [ECX+0x104]` | `f32` |
| 91 | `CellWidthX` `0x0045A390` | `FLD dword ptr [ECX+0x108]` | `f32` |
| 92 | `CellWidthZ` `0x0045A3A0` | `FLD dword ptr [ECX+0x10C]` | `f32` |
| **93** | `0x007C1910` | `MOV EAX, dword ptr [ECX+0x118]` | `u32` **cell count X** |
| **94** | `0x004E82A0` | `MOV EAX, dword ptr [ECX+0x11C]` | `u32` **cell count Z** |

Three independent confirmations that `+0x118`/`+0x11C` are counts, not object pointers:

1. The public SDK header `cISC4City.h:218-219` declares `CellCountX` and `CellCountZ` immediately after
   `CellWidthX` / `CellWidthZ`, i.e. exactly slots 93 and 94.
2. `cSC4City::GetCitySizeType` (`0x00459390`, slot 87) discriminates on `+0x118` against `0x40` and `0x80` —
   64 and 128, the two SC4 city cell counts — and `cSC4City::SetSize` derives them as `round(size × k)`.
3. The cell-bounds tests (`CellIsInBounds` / `CellCornerIsInBounds`, `decomp/cSC4City.c:167`, `:187`) compare the
   cell coordinate against `*(int*)(city+0x118)` / `*(int*)(city+0x11C)`.

So the in-game reading of **128 × 128 is correct**, and this spec's "city slots 93, 94" is correct. A held ABI row
correcting the two `interfaces.tsv` names is in `build-runs/pending-abi/re-answers31-rows.tsv`; that file is not
merged here.

Module globals (tuning, §6): `0x00B07F40` small/medium model threshold, `0x00B07F44` medium/large model threshold,
`0x00B07F48` small/medium group threshold, `0x00B07F4C` medium/large group threshold, `0x00B07F50` cost to demolish
a destroyed lot tile. All `u32`.

Nothing persists between calls except the cached pointers and the tuning. Φ's per-call fields are reset at the end
of every `DemolishRegion` (§4.4 step 9).

## 3. Agents, messages and cadence

No simulator agent, no `AddAgent` name, no animation tick. Everything runs synchronously inside the caller's call.

Subscriptions (made in `Init`, removed in `Shutdown`):

| Message | Handling |
|---|---|
| `0xC53D10AA` (exemplar changed, foundations §5 table) | `ReadProperties()` (§6) **unconditionally**: data1/data2 are not compared with the tuning exemplars *[quirk]* |
| `0x26D31EC1` (PostCityInit) | subscribed, ignored (returns true) |
| `0xA9DED179` (deferred lot demolition) | data1 points to a heap record `{cISC4Lot* lot, bool dezone}`. If `lot` is non-null and `lot.IsInitialized()` (lot slot 6): `lotDeveloper.EndDemolishLot(lot)` (slot 13); if `dezone`: `zoneManager.PlaceZone(lot rect, 0, 1, 1, 0, 1, 1, 0, 0, 0)` (dezone the lot's rectangle, zones.md); release the lot. The record is then freed by the handler. **Nothing in the executable posts this message** (no immediate `0xA9DED179` outside `Init`/`Shutdown`/`DoMessage`; the ID's bytes occur only at `0x00466A03`,
`0x00466F29` and `0x00467002` in the whole file, so no data table names it either, DM3) *[quirk]*; FreeCity4 must keep the subscription and the handling for plugins. The record owns one reference to the lot, released only on the initialized path (`0x004670A0`): a null lot or one whose `IsInitialized()` is false jumps straight to freeing the record (`0x004670A7`, heap free `0x005E5620`), so an uninitialized lot's reference leaks *[quirk]* (DM10). FreeCity4 may release it on both paths *[choice]*: nothing posts the message, so the leak is unobservable. `EndDemolishLot` is lot developer slot 13 (`+0x34`); the dezone is zone manager slot 11 (`0x0046708C`, the manager through `0x00B43D14`) |

Sent (synchronously, `MessageSend`):

| Message | When | Data |
|---|---|---|
| `0xAA7A014D` (pre-demolish network occupant; the neighbours simulator answers with `DeleteOccupantConnection(void1)`, neighbors.md §6.5) | before a power pole is removed (§4.3, §4.6) | void1 = the pole occupant; IGZUnknown = the same occupant |

The removals themselves post the occupant manager's own messages (`RemoveOccupant` posts `0x99EF1143`); the lot
developer and the zone manager post theirs. The terrain levelling posts `0x278E6003` through `RedisplayTerrain`
(landfill.md §4.5.3).

## 4. Behaviour

### 4.1 `Init` (`0x00466EF0`), `Shutdown` (`0x00466980`)

`Init`: if already initialized, return true. Otherwise set `initialized`; subscribe to the three messages of §3;
cache and AddRef the city (and read its cell counts with city slots 93 and 94), the occupant manager, the lot
developer, the terrain; create Φ (0x34 bytes, §4.2) and AddRef it; `ReadProperties()`; return true (AL = 1).
`Shutdown`: if initialized, release Φ, the terrain, the lot developer, the occupant manager and the city (in that
order), unsubscribe the three messages and clear `initialized`. Returns AL = 1 either way.

### 4.2 The demolition filter Φ *[compat]*

Φ decides which occupants a demolition may touch. It is a `cISC4OccupantFilter`; its per-call fields are:

| Field | Meaning | Set by |
|---|---|---|
| `boundsActive`, `x0, z0, x1, z1` (floats, world metres) | restrict to occupants whose **position** lies in the box (inclusive) | the per-cell query of §4.4 step 4 (set before the query, cleared after) |
| `privilegeActive`, `level` | require `occupant.CanRemove(level)` (occupant slot 14) | `DemolishRegion` step 2 |
| `callerFilter` | an extra `cISC4OccupantFilter` from the caller (may be null) | `DemolishRegion` |
| `viewFilter` | the 3D view's current view filter (may be null) | `DemolishRegion` |

`Φ.IsOccupantIncluded(o)` is the conjunction, evaluated in this order (the first failing test returns false):

1. **Realism.** If the "realistic occupant" test rejects `o` while the default occupant filter accepts it, reject.
   (Both are shared filter classes. The default filter rejects occupants that expose interface `0x4A10AFC2` or
   `0x8A139184` or belong to occupant group `0x4217`, `0x4208` or `0x421D`; the realistic filter additionally
   rejects occupants of group `0x5001`, base-texture occupants on empty lots and props that are not "real" props.
   *[inferred]* summary; the two classes belong to the occupant-filter spec.)
2. `callerFilter` non-null and it rejects `o` → reject.
3. `boundsActive` and `o.GetPosition()` outside `[x0, x1] × [z0, z1]` (x and z only) → reject. All four sides are
   inclusive (`0x0046679D`–`0x004667C5`).
4. `privilegeActive` and not `o.CanRemove(level)` → reject. The base `CanRemove(level)`: level bit 1 → true unless
   the occupant's "system lock" byte is set; level bit 2 → true unless its "player lock" byte is set; otherwise
   true.
5. **Type.** foundation `0x68FD0C69`: accept only if its `cISC4FoundationOccupant` (`0xC8FCE101`) slot 9 returns 1
   (and stop: steps 6–7 do not apply). The original does not test the QueryInterface result (`0x00466817`–
   `0x00466835`: the out pointer starts null and slot 9 is called through it), so a foundation-type occupant without
   the interface crashes it. Every shipped foundation has the interface; FreeCity4 rejects such an occupant
   *[choice]* (DM5). building `0x278128A0`, power pole `0x2890D4DE`, construction `0x096E763C`,
   prop `0xA823821E`, network `0xC772BF98`, flora `0x74758926`: continue. **Any other type → reject.**
6. `viewFilter` non-null and it rejects `o` → reject.
7. Network `0xC772BF98` while the 3D view's level (`GetCurrentViewLevel`, view slot 22) is **not 1**: reject if
   its `cISC4NetworkOccupant` reports `HasAnyNetworkFlag(0x1F4F)` (slot 9) — in the underground view only the
   networks outside that mask can be demolished. `0x1F4F` is every network type bit except 4 (water pipe), 5
   (power line) and 7 (subway) (DM1), so a network occupant passes step 7 in another view level only when it carries
   none of the other nine types. Power poles are a separate occupant type and are not tested here.
8. Accept.

### 4.3 `DemolishOccupant(doIt, o, privilege, flags, int64* cost, bool skipNetworkUpdate, list* orphans)` (`0x004691F0`, slot 8) *[compat]*

Returns a bool (AL). Writes `*cost` (if non-null) on every path **except** the first.

1. If `o.IsFlagSet(0x40)` (occupant slot 32) and `flags & 0x100`: return false (cost untouched).
2. By `o.GetType()`; `c := 0` unless stated:

| Type | Price `c` | When `doIt` | Result |
|---|---|---|---|
| foundation `0x68FD0C69` | 0 | if `o.IsFlagSet(1)`: `occupantManager.RemoveOccupant(o, true, 0)` (slot 21) | the removal's result if attempted, else true |
| construction `0x096E763C` | 0 | `RemoveOccupant(o, true, 0)` | the removal's result when `doIt`, else true |
| building `0x278128A0` | `buildingDevelopment.GetCostOfOccupantDemolition(building, 1)` (city slot 53, then slot 10; int64 property `0x099AFACD`, building-development.md) | **nothing**: buildings go with their lot (§4.4 step 7) | true; **false** (c = 0) if the city has no building-development simulator |
| power pole `0x2890D4DE` (needs `cISC4PowerPoleOccupant` `0xA8C86142`; otherwise as "other") | pole slot 10 (int64) | remember the pole's connected poles (slot 4 count, slot 7 by index); **send `0xAA7A014D`** (§3); `RemoveOccupant(o, false, 0)`; if removed: pole slot 6 (`BreakAllConnectionsAtDestination(true)`), `o.Shutdown()` (occupant slot 4), and every remembered pole whose connection count (slot 4) is now 0 is appended to `orphans` (as `cISC4Occupant`, IID `0x26BB767E`) or released if `orphans` is null | true if not `doIt`, else the removal's result |
| prop `0xA823821E` | 0 | `propManager.RemovePropA(o)` (prop manager slot 29); if it succeeded, `o.Shutdown()` | true, or the removal's result when `doIt`; **false** if there is no prop manager |
| network `0xC772BF98` (needs `cISC4NetworkOccupant` `0xA821EF94`; otherwise as "other") | network occupant slot 63 (int64) | if not `skipNetworkUpdate` and `o.GetFlags() & 1`: `UpdateNetworks({o})` (§4.6) | true |
| flora `0x74758926` | `flora.GetCostOfOccupantDemolition(o, 0)` (city slot 59, flora slot 22) | `RemoveOccupant(o, true, 0)` (result ignored) | true; **false** if there is no flora simulator |
| anything else | 0 | — | false |

   **Pole references** (DM11). The remembered neighbour poles are held: each one from pole slot 7 is wrapped in an
   auto reference (`0x004F20C0`, AddRef) and pushed into a local vector (`0x00442E40`), which releases them all at
   the end (`0x006C6E90`). An orphan's `QueryInterface(0x26BB767E)` gives a new reference; that reference is what is
   appended to `orphans` (`0x00467B50`, no further AddRef), or it is released at once when `orphans` is null
   (`0x0046937B`). So every orphan in the list carries one reference that the list's owner must release, which
   `DemolishRegion` step 8 does. The `0xAA7A014D` send goes through the helper `0x00466C50`.

3. `*cost := c` (int64) if `cost` is non-null; return the result.

### 4.4 `DemolishRegion(doIt, R, privilege, flags, dezone, callerFilter, int64* cost, set* demolished, clicked, sx, sy)` (`0x00468410`, slot 6) *[compat]*

`R` is an `SC4CellRegion<long>` (a rect plus a cell mask). The result is a bool (AL).

1. `effects := doIt and (flags & 0xFF) ≠ 0`. Store `R` at `+0x30`.
2. Ask the 3D view (`0x00B43DC4`, **no null test** in the original) for `viewFilter := GetCurrentViewFilterInfo()`
   (slot 18, AddRef'd, may be null) and call `GetCurrentViewLevel()` (slot 22, result unused). Set up Φ:
   `level := 1` if `privilege = 0`, else `2`; `useView := privilege ≠ 0 and (flags & 0x2000) = 0`;
   `Φ.callerFilter := callerFilter`; `Φ.viewFilter := viewFilter` if `useView` (else it stays null);
   `Φ.privilegeActive := true`, `Φ.level := level`. *[choice]* With no 3D view, FreeCity4 must behave as if
   `viewFilter` were null and the view level were 1.
   **`useView` gates only Φ** (DM12). The local `viewFilter` is fetched whatever `useView` is (`0x00468473`;
   `useView` is computed after it, `0x0046847D`–`0x00468490`), and the two tests below that name `viewFilter`
   directly use it **also** with `privilege = 0` or `flags & 0x2000`: the empty-lot qualification of step 4
   (`0x00468960`–`0x00468974`, view filter slot 4) and the effect gate of step 5 (`0x00468D02`–`0x00468D12`, view
   filter slot 3).
3. `result := true`; `occs :=` an ordered set of occupants; `lots :=` a list (new entries at the **front**);
   `seen :=` a set of lots; `count := 0`.
4. For `x := R.x0 … R.x1` (outer), `z := R.z0 … R.z1` (inner), for each cell in the mask:
   - `lot := lotManager.GetLot(x, z, false)` (**slot 6**; see the DQ4 note below). If `lot` is non-null and not in `seen`, add it to `seen` and
     decide whether the whole lot qualifies:
     - `b := lot.GetBuilding()` (lot slot 48).
     - If `b` is null: the lot qualifies when `lot.GetState()` (slot 33) is **1 or 6** and either `viewFilter` is
       null or it includes the building type (`viewFilter` slot 4 with `0x278128A0`).
     - Otherwise: the lot qualifies when `lot.IsInDemolishableState()` (slot 39) and `Φ` accepts `b`.
     - If it qualifies: list every occupant of the lot (`GetLotOccupants`, slot 49). With `Φ.viewFilter` cleared for
       the test, if any **building or prop** on the lot is rejected by `Φ`, the lot is **blocked**: `result :=
       false`, nothing of the lot is taken. Otherwise push the lot at the front of `lots` and insert every lot
       occupant that `Φ` accepts (with the view filter restored if `useView`) into `occs`; `count += 1` per
       occupant newly inserted.
     - If it does not qualify: if `privilege = 1` and `b` is non-null and not `b.CanRemove(2)`, `result := false`.
   - Then, for this cell whether or not it has a lot: set Φ's bounds to the cell's world square (`CellCornerToPosition`
     of `(x, z)` and of `(x+1, z+1)`, city slot 96), query the occupant manager for the occupants in the cell
     accepted by Φ (slot 41, max −1), clear the bounds. For each such occupant `o`: `inc :=` the **default**
     occupant filter's verdict on `o`; insert `o` into `occs` (and `count += 1` if new) when
     `(lot is null or not inc) and ((flags & 0x2000) = 0 or inc)`.

   **DQ4 — which `GetLot`? (DIFFERS from the earlier wording, corrected 2026-10-16 by re-answers31.)** The
   previous text said `GetLot(x, z)` "(slot 5)". That is wrong twice over: the cell lookup is **slot 6**, and it
   takes **three** arguments. The compiled `cISC4LotManager` vtable (`0x00AB37C0`) is:

   | Slot | Offset | Compiled | Stack args |
   |---|---|---|---|
   | 5 | `+0x14` | `0x0072A1C0` | 1 (pop 4) — takes a pointer and reads `+0x00`/`+0x08` from it, i.e. `cS3DVector3 const&` |
   | 6 | `+0x18` | `0x0072B410` | 3 (pop 12, `ret 0x0C`) — bounds-checks `0 ≤ x < this+0x24` and `0 ≤ z < this+0x28`, then a 2-D grid lookup |

   So **slot 5 = `GetLot(cS3DVector3 const& sPoint)`** and **slot 6 = `GetLot(int32 x, int32 z, bool)`**. The
   SDK header `cISC4LotManager.h:42-43` declares them in the opposite order and is wrong; this is the same
   finding already written down in [crime.md](crime.md) §"the SDK's own slot order" and used by
   [aura.md](aura.md) §9 (`GetLot(x, z, false)`), [crime.md](crime.md) (`GetLot(cellX, cellZ, false)`),
   [demand.md](demand.md) and [building-development.md](building-development.md) (`GetLot(x, z, true)`), all at
   slot 6. **The change to make:** call **slot 6** with the third `bool` argument. Use `false` here — the
   `bool` is a containment re-check: when it is true the body additionally asks the lot (slot 21) for its bounds
   and returns null unless those bounds really contain `(x, z)`; when false it returns the lot occupying the cell
   without that re-check. For a cell walk that is already inside the demolition rectangle, `false` is the right
   choice and matches the other callers that walk cells.

5. Price and remove the occupants. `total := 0` (int64); `orphans :=` an empty list. For each `o` in `occs` (set
   order):
   - `ok := DemolishOccupant(doIt and not effects, o, privilege, flags, &c, true, orphans)`.
   - If `ok`: if `effects`: when `viewFilter` is null or includes `o`, start the occupant effect
     (`DoDemolishOccupantEffect(o, flags, lotAt(o), o = clicked, sx, sy)`, §4.8, where `lotAt` is the lot under
     `o.GetPosition()` via `PositionToCell`, city slot 95); then `DemolishOccupant(true, o, privilege, flags, &c,
     true, orphans)` for real. `total += c`; insert `o` into `*demolished` if given.
   - A false `ok` does **not** change `result`.
6. If `effects` and `count > 0`: `DoDemolishGroupEffect(flags, 0, count, R)` (§4.8).
7. If `doIt`: `networkManager.QueueRemovals(true)` (slot 7). For each lot in `lots` (front to back, i.e. the
   **reverse** of discovery order):
   - If `lot.GetState() = 6` (destroyed): `total += sizeX × sizeZ × kCostToDemolishDestroyedLotTile` (sizes from
     `GetSize`, slot 22, as bytes). This is added **whether or not** `doIt`.
   - If `doIt`:
     - if (the lot configuration (lot slot 9) is null or not `IsCompatibleWithWater()` (configuration slot 14)) and
       `flags & 0x300`: `lotDeveloper.ExplodeLot(lot, (flags & 0x1000) = 0)` (slot 14);
     - otherwise: `zone := lot.GetZoneType()` (slot 31); `lotDeveloper.StartDemolishLot(lot)`;
       `lotDeveloper.EndDemolishLot(lot)` (slots 12, 13; lot-developer §7); then
       - if `dezone` or `zone ∈ {11, 12, 15}` (airport, seaport, plopped): dezone the lot's bounding rect with
         `zoneManager.PlaceZone(region(rect), 0, 1, 1, 0, 1, 1, 0, 0, 0)` (zone manager slot 11);
       - else if `(flags & 0x800) = 0` and `zone ∈ {7, 8, 9}` (agriculture, industrial): **re-parcel**: remember
         the lot's bounding rect and facing (slots 14, 23), `lotManager.DeleteLot(lot)` (slot 24), then for every
         cell of the rect (x outer) whose zone (`zoneManager` slot 12) still equals `zone`:
         `lotManager.CreateLotIfPossible(x, z, 1, 1, facing, &new, 0, 0, 0)` (slot 22) and release `new`;
       - else nothing more (an RCI lot stays as an empty zoned lot in state 1, lot-developer §7.2).
8. Drain `orphans` from the back: insert each into `occs` and into `*demolished` if given. If `doIt`:
   `UpdateNetworks(occs)` (§4.6), then `networkManager.QueueRemovals(false)`. (`0x00468F70`–`0x00469035`: each
   orphan is popped and its node freed; the two sets take their own references on insert, and the list's reference
   is then released at `0x00469031`. Net, the drain neither leaks nor over-releases: DM11.)
9. `*cost := total` if `cost` is non-null. Clear Φ's caller filter, view filter and `privilegeActive`; release
   `viewFilter`; clear `+0x30`. Return `result`.

The orphan poles found in step 5 reach `UpdateNetworks` only through step 8, so they are removed in the same batch.

### 4.5 `DemolishLots(doIt, lots, privilege, flags, dezone, callerFilter, int64* cost, set* demolished)` (`0x004670C0`, slot 7) *[compat]*

- An **empty list** returns true at once and writes nothing (`*cost` is untouched; power.md and
  building-development.md rely on this).
- Otherwise: the rect is the union of the lots' bounding rects (lot slot 14); the cell mask starts empty and gets
  every cell of every lot for which `IsInDemolishableState()` is true. Lots not in a demolishable state are
  **silently skipped** (their cells are not in the region). Return `DemolishRegion(doIt, region, privilege,
  flags | 0x2000, dezone, callerFilter, cost, demolished, null, 0, 0)`.
- A non-empty list whose lots are **all** not demolishable still calls `DemolishRegion`, with the union rect and an
  empty mask: it returns true, writes `*cost := 0` and, when `doIt`, still calls `QueueRemovals(true)` and
  `QueueRemovals(false)`.

Because `0x2000` is set, the per-cell pass of §4.4 step 4 adds nothing on lot cells (a lot occupant is inserted only
when it is also in the lot's own occupant list, which it always is), and Φ has no view filter. The 3D view's filter
is still used by the empty-lot qualification and the effect gate (§4.4 step 2, DM12).

A null entry in the list (power.md's nuclear-free-zone quirk) is dereferenced by the original: it calls lot slot 14
on the first entry (`0x004670F9`) and on every later one, with no null test. FreeCity4 must skip null entries
*[choice]*: the union covers the non-null entries only, and a list holding **only** null entries behaves like the
empty list (returns true, `*cost` untouched). The original has no defined result there (it crashes), so this is the
natural reading.

### 4.6 `UpdateNetworks(set)` (`0x004681A0`, not in the interface)

For each occupant `o` in the set (set order) with `o.IsFlagSet(1)`:

- power pole (type `0x2890D4DE`): send `0xAA7A014D` (§3), then `occupantManager.RemoveOccupant(o, true, 0)`;
- network occupant (type `0xC772BF98`): append to a list (AddRef) and call `neighbors.DeleteOccupantConnection(o)`
  (neighbours simulator `0x00B43D8C`, slot 8; neighbors.md §6.5). The test is the occupant type alone: no
  `QueryInterface` and no network-type filter, so every network kind is queued (roads, rails, pipes, power-line
  pieces, subways and the rest) (DM9);
- any other type: skipped.

If the list is non-empty: `networkManager.DestroyNetworkOccupants(list)` (slot 6, `0x00468396`; the manager from the
getter `0x00602680`), then release the list (occupant manager slot 48).

Occupant flag 1 means "currently in the occupant manager" (DM4, verified): `InsertOccupant` returns at once when it
is set (`0x0049EF8E`) and sets it (`SetFlag(1)`, `0x0049F202`) just before posting `0x99EF1142`; `RemoveOccupant`
goes on only when it is set (`0x0049F264`) and clears it (`ClearFlag(1)`, `0x0049F382`) after posting `0x99EF1143`.
So poles already removed in §4.3 are skipped.

### 4.7 `ModifyTerrainHeight(doIt, perVertex, x0, z0, x1, z1, float h, cISC4Lot* exempt, uint32 reason, int64* cost)` (`0x00467BB0`, slot 9) *[compat]*

Returns an int (EAX), the number of vertices that differ from `h` (area mode) or that were (or would be) set
(per-vertex mode) *[inferred]*. `cost` is **never written** (DM2, verified): the function (`0x00467BB0`–
`0x00468191`, `ret 0x28`) never loads its tenth argument; the only late argument it reads is the ninth, `reason`, for
the redisplay calls (`0x00467F59`, `0x00468157`). No shipped caller passes a cost pointer.

Let `Q := (max(x0 − 1, 0), max(z0 − 1, 0), min(x1, cellsX − 1), min(z1, cellsZ − 1))`. Query the immovable occupants
in `Q` (occupant manager slot 40, at most 1 result in area mode, all in per-vertex mode) with the immovability
filter: buildings, props, power poles and networks whose bool property `0x29F013E6` is absent or false.

**Area mode** (`perVertex = false`):

1. If any immovable occupant was found: return 0, change nothing.
2. `n :=` the number of vertices `(x, z)`, `x ∈ [x0, x1]`, `z ∈ [z0, z1]`, with `|GetAltitudeAtVertex(x, z) − h| >
   0.05` (terrain slot 14). If `n = 0`: return 0.
3. List the lots in `Q` (lot manager slot 7). The area is locked if any lot other than `exempt` reports
   `GetVerticesAreLocked()` (lot slot 68).
4. If not locked and `doIt`: `terrain.SetAltitude(rect, h, 0, reason)` (slot 49, `0x00468172`) with `rect =
   (max(x0, 0), max(z0, 0), min(x1, cellsX), min(z1, cellsZ))`, then `terrain.RedisplayTerrain(1, 1, rect, reason)`
   (slot 68, `0x00468187`): both calls are given the **same** local rectangle (DM7).
5. Return `n` (also when the area is locked or `doIt` is false).

**The 0.05 tolerance is a `float` comparison** *[compat]* (DM6). In both modes the altitude (terrain slot 14,
returned in ST0) minus `h` (loaded as `float`) is taken in absolute value and compared with the `float` constant at
`0x00A880D8` (`0x3D4CCCCD`, 0.05000000074505806). Area mode counts the vertex when `|d| > 0.05f`
(`0x0046802A`–`0x0046803E`); per-vertex mode skips it when `|d| < 0.05f` (`0x00467D45`–`0x00467D59`), so a vertex
at exactly 0.05f is counted in neither. The main thread runs the x87 at 24-bit precision (docs/ops/lessons.md §5.2), so the
subtraction rounds to `float`: FreeCity4 must compute `abs(alt - h)` in `f32` and compare with `0.05f32`. Widening
to `f64` and comparing with 0.05 differs for differences within one `float` step of the threshold.

**Per-vertex mode** (`perVertex = true`, the only mode the shipped callers use — landfill.md §4.5.3):

1. For `x := x0 … x1` (outer), `z := z0 … z1`: skip water vertices (`IsWaterVertex`, slot 37) and vertices with
   `|altitude − h| < 0.05`. Skip the vertex if any lot of the 2×2 cells around it (`(x−1, z−1)…(x, z)`) other than
   `exempt` has locked vertices, or if any immovable occupant's bounding box overlaps the square
   `[16x − 14.5, 16x + 14.5] × [16z − 14.5, 16z + 14.5]` (x and z, closed; the 14.5 is `16 − 1.5`, the
   `float` 1.5 at `0x00A880D4`). Otherwise count it, and if `doIt` set it:
   `terrain.SetAltitude(x, z, h)` (slot 51).
2. If the count is > 0: `RedisplayTerrain(1, 1, (Q.x0, Q.z0, Q.x1 + 1, Q.z1 + 1), reason)` — **also in a dry
   run** *[quirk]*, so a dry run posts the terrain message `0x278E6003` with `data4 = reason`.

The shipped `reason` is 5 (landfill.md §4.5.3); it is passed through to the terrain untouched.

### 4.8 Effects (`DoDemolishOccupantEffect` slot 10, `0x004673B0`; `DoDemolishGroupEffect` slot 11, `0x00467530`)

Presentation only; no simulation state changes. A headless or replaced utility may do nothing here *[choice]*.

**Occupant effect** `(o, flags, lot, isClicked, sx, sy)`: compute the largest extent `s` of `o`'s bounding box; give
up if `s` scaled to the screen is below the render-quality cutoff. Build the effect key `k := (flags & 0xFF) << 8`
plus a type nibble: building or power pole `0x10`; flora `0x20`; prop `0x20` if in occupant group `0x5004`, else
`0x30`, and **no effect at all** unless the prop's bool property `0x8A5E5DB8` is true; network `0x50` if the network
has type 1 but not type 2 (network slot 22), else `0x40` (also `0x40` when the network interface is missing). Any
other type (foundation, construction) keeps nibble **0** and the effect still goes ahead with key
`(flags & 0xFF) << 8 | size` (DM13, `0x00466A90`: the nibble helper returns true for them; only the prop test can
return false). Play the occupant's sound (u32 property `0x0A93036F`, if present) through the audio system. Add a
size class: `+2` if `s > kMediumLargeModelThreshold`, else `+1` if `s > kSmallMediumModelThreshold` (both strict:
`0x0046740E`, `0x0046743C`). Look the key up in effect group `0x6A3589A0`; when absent, try `k − 1`, `k − 2`, …
down to the key whose low nibble is 0, then give up. Attach it to `o`; if `isClicked`, aim it along the ray through
screen point `(sx, sy)`; if `lot` exists and its condition (lot slot 35) is neither 0 nor 1, tint it 50 % grey;
start it.

**Group effect** `(flags, n, count, R)`: at the centre cell of `R` start effect `0x6A3589A0`, instance
`(flags & 0xFF) << 8 | size`, where `size` compares **`n`** with the group thresholds. `DemolishRegion` passes
`n = 0` and puts the count in the third argument, so the group size is **always the small variant** *[quirk]*
(both platforms). The effect's altitude is sampled at the cell *indices* used as world coordinates *[quirk]*
(visual only; terrain slot 13, `0x004675A2`).

Details (DM8): the centre cell is `((x0 + x1) / 2, (z0 + z1) / 2)` in signed integer division, truncating toward
zero (`0x00467555`, `0x00467568`), as C and Rust `/` do. The size compares unsigned and strictly: `+2` if `n >
kMediumLargeGroupThreshold` (`0x00B07F4C`, `jbe` at `0x004675BB`), else `+1` if `n > kSmallMediumGroupThreshold`
(`0x00B07F48`), else `+0`.

### 4.9 Flag bits (the `flags` argument)

| Bits | Meaning |
|---|---|
| `0x000000FF` | the "cause" byte: selects the effect family; 0 means **no effects** |
| `0x00000100` | skip occupants with occupant flag `0x40` (DemolishOccupant returns false for them) |
| `0x00000300` | explode the lots (`ExplodeLot`) instead of the normal clear, unless the lot's configuration is water-compatible |
| `0x00000800` | do not re-parcel agriculture and industrial lots (developers pass it) |
| `0x00001000` | with `0x300`: `ExplodeLot(lot, false)` instead of `true` |
| `0x00002000` | set by `DemolishLots` (and passed by the network tool): Φ gets no view filter, and loose occupants of lot cells are not added. The 3D view's filter still gates empty lots and effects (§4.4 step 2) |

`privilege`: 0 = the system (developers, simulators; `CanRemove(1)`), 1 = the player (tools; `CanRemove(2)` and the
extra lot test of §4.4 step 4). Shipped callers use only 0 and 1.

### 4.10 Worked examples

1. **Bulldozer preview over a 2×1 empty RCI lot** (state 1, no building) on cells (10, 20)–(11, 20), with one tree
   (flora, demolition cost 5) on cell (12, 20) inside the drag rectangle (10, 20)–(12, 20), view filter null:
   `DemolishRegion(false, R, 1, 1, false, null, &c, null, null, 0, 0)`. The lot qualifies (state 1); its occupants
   (base texture, props) are tested by Φ: base textures are not in the type list, so only props and flora survive.
   The tree on (12, 20) comes from the per-cell pass (no lot there). Each is priced: props 0, tree 5. Result true,
   `*c = 5`. Nothing changes. `effects` is false (dry run).
2. **Destroyed lot**: a 3×2 lot in state 6 with no building, flags 1, privilege 1. It qualifies; its remaining
   occupants are priced, and `3 × 2 × 2 = 12` is added (`kCostToDemolishDestroyedLotTile` = 2 shipped). Committed,
   it is cleared with `StartDemolishLot` / `EndDemolishLot`.
3. **Power plant hard failure** (power.md): `DemolishLots(true, [lot], 0, 0x303, false, null, null, 0)`. Cause 3
   effects play; the lot is not water-compatible, so `ExplodeLot(lot, true)`; nothing is dezoned or re-parcelled.
4. **Zone change over part of a farm**: `PlaceZone` prices with `DemolishLots(false, …, 1, …)`, writes the zone
   grid, then commits with `(true, …, 0x2000, …)` (zones.md §6). The farm lot (zone 7) is cleared and, because
   `0x800` is not set, deleted and **re-parcelled into 1×1 lots** on the cells that still hold zone 7 — the farm
   cells outside the rezoned area. Rezoning the whole farm leaves no 1×1 lots.

## 5. Random numbers

None. The demolition utility owns no generator and calls none.

## 6. Tuning *[compat]*

`ReadProperties` (`0x00466CD0`) runs at `Init` and on every `0xC53D10AA`. Missing exemplars or properties keep the
current values.

| Exemplar (type, group, instance) | Property | Global | Code default | Shipped (`SimCity_1.dat`) |
|---|---|---|---|---|
| `0x6534284A, 0x690F693F, 0x2A8A09CA` "Demolition Thresholds" | `0x6A8A09D2` small/medium model | `0x00B07F40` | 64 | 20 |
| | `0x6A8A09D3` medium/large model | `0x00B07F44` | 128 | 64 |
| | `0x6A8A09D4` small/medium group | `0x00B07F48` | 20 | 20 |
| | `0x6A8A09D5` medium/large group | `0x00B07F4C` | 100 | 100 |
| `0x6534284A, 0xE7E2C2DB, 0xE8DA7677` "Building Development" | `0x2AB845E7` cost to demolish a destroyed lot tile | `0x00B07F50` | 3 | **2** |

`EP1.dat` does not override either exemplar. Only `0x2AB845E7` affects the simulation; the thresholds are visual.

## 7. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0x0990C38A`) *[compat]*

| Offset | Size | Field |
|---|---|---|
| 0 | u16 | version, always **1** |

`Write` writes `u16 1` and returns `GetError() = 0` (EAX 0/1). `Read` reads a `u16`; any value other than 1 returns
**false** (AL = 0, the rest of EAX undefined); otherwise it returns `GetError() = 0`. No other state is read or
written. Validated: the class-object entry is `01 00` in **421 of 421** distinct saves under
`Documents\SimCity 4\Regions\` (all bundled regions plus the test region).

## 8. Interface slot table *[compat]*

`cISC4Demolition` (vtable `0x00A88004`).

| Slot | Offset | Method | Original | Args (dwords) | Callee pops | Return |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface(iid, void**)` | `0x004664B0` | 2 | 8 | AL |
| 1 | `+0x04` | `AddRef()` | `0x004664A0` | 0 | 0 | EAX (new count) |
| 2 | `+0x08` | `Release()` | `0x00466950` | 0 | 0 | EAX |
| 3 | `+0x0C` | `Init()` | `0x00466EF0` | 0 | 0 | AL (1) |
| 4 | `+0x10` | `Shutdown()` | `0x00466980` | 0 | 0 | AL (1) |
| 5 | `+0x14` | `SetDefaultOccupantFilter(filter)` — does nothing | `0x00648F10` (shared `ret 4`) | 1 | 4 | none (EAX undefined) |
| 6 | `+0x18` | `DemolishRegion(bool, const SC4CellRegion<long>&, PrivilegeType, uint32, bool, cISC4OccupantFilter*, int64*, set<occupant>*, cISC4Occupant*, long, long)` | `0x00468410` | 11 | 44 | AL |
| 7 | `+0x1C` | `DemolishLots(bool, const list<lot>&, PrivilegeType, uint32, bool, cISC4OccupantFilter*, int64*, set<occupant>*)` | `0x004670C0` | 8 | 32 | AL |
| 8 | `+0x20` | `DemolishOccupant(bool, cISC4Occupant*, PrivilegeType, uint32, int64*, bool, list<occupant>*)` | `0x004691F0` | 7 | 28 | AL |
| 9 | `+0x24` | `ModifyTerrainHeight(bool, bool, int, int, int, int, float, cISC4Lot*, uint32, int64*)` | `0x00467BB0` | 10 | 40 | EAX (count) |
| 10 | `+0x28` | `DoDemolishOccupantEffect(cISC4Occupant*, uint32, cISC4Lot*, bool, long, long)` | `0x004673B0` | 6 | 24 | none |
| 11 | `+0x2C` | `DoDemolishGroupEffect(uint32, int, int, const SC4Rect<long>&)` | `0x00467530` | 4 | 16 | none |
| 12 | `+0x30` | deleting destructor `(flags)` | `0x00466920` | 1 | 4 | EAX (this) |

The decomp's name for `0x004673B0` (`DoDemolishGroupEffect`) is wrong: slot 10 is the **occupant** effect (6
arguments) and slot 11 the group effect (4 arguments), matching the order of the SDK header and the call sites in
`DemolishRegion` (`0x00468D7C`, `0x00468E66`).

Container arguments are B-side STLport containers (allocators.md): the `set` arguments are
`std::set<cRZAutoRefCount<cISC4Occupant>>`, `DemolishLots`' list is `std::list<cRZAutoRefCount<cISC4Lot>>`, and
`DemolishOccupant`'s list is the allocator-service `list<cISC4Occupant*>` (an `ilist`), which the callee appends
to and the caller releases.

## 9. Who reaches in

| Caller | Call |
|---|---|
| bulldozer tool, preview (`cSC4ViewInputControlDemolish::UpdateSelectedRegion`, `0x004B93B0`) | `DemolishRegion(false, R, 1, 1, false, null, &cost, null, null, 0, 0)`, or `DemolishOccupant(false, o, 1, 1, &cost, …)` when one occupant is picked |
| bulldozer tool, release (`OnMouseUpL`, `0x004B9C50`) | if the preview returned false: show `0x495A6D86` and stop. Else **`budget.SetTotalFunds(GetTotalFunds() − previewCost)`** (budget slots 6 and 5: no expense line), then `DemolishOccupant(true, o, 1, 1, &cost, false, null)` or `DemolishRegion(true, R, 1, 1, false, null, &cost, null, clicked, x, y)`; if the real cost is 0, show `0x8A5EC5B8` |
| zone manager `PlaceZone` (zones.md §6) | `DemolishLots(false, list, checkSlope, 1, 0, 0, &c, 0)` to price; `DemolishLots(true, list, checkSlope, 0x2000, 0, 0, 0, 0)` to commit |
| zone developer `OnDezone` (zones.md §7.9) | `DemolishRegion(true, region, 1, 1, 0, 0, 0, 0, 0, 0, 0)` |
| lot developer `TestCoffeeBar`, flora clean-up (lot-developer.md) | `DemolishLots(true, [lot], 0, 0x800, 0, 0, 0, 0)`; `DemolishOccupant(true, flora, 0, 1, null, 0, null)` |
| lot developer `AttemptAreaLevelling` (landfill.md) | `ModifyTerrainHeight(true, waterFlag, x1, z1, x2 + 1, z2 + 1, avg, lot, 5, null)` |
| tract developer (tract-developer.md) | `DemolishLots(true, [lot], 0, 0x800, false, null, null, 0)` |
| power simulator (power.md) | nuclear-free zone `DemolishLots(true, list, 0, 1, false, null, null, 0)`; hard failure `(true, [lot], 0, destroy | 0x300, false, null, null, 0)` |
| building development (building-development.md) | `DemolishLots(true, queued, 0, 1, true, null, &cost, null)` |
| legalize-gambling ordinance `SetOn` (`0x0080C483`) | `DemolishLots(true, list, 0, 1, false, null, null, null)` |
| network placement helper (`0x0061FB87`) | `DemolishRegion(false, region, 1, 0x2001, false, temporaryFilter, cost, demolished, null, 0, 0)`; selected bitmap cells and separate placement/demolition filters: [network-routines.md](network-routines.md) §§4–6 |
| network tool `ClearNetworkOccsFromCell` (`0x0062746A`) | `DemolishOccupant(true, o, 1, 0x2001, null, false, null)` |

## 10. Quirks (reproduced)

1. `0xC53D10AA` re-reads the tuning for any exemplar (§3).
2. `0xA9DED179` has a handler but no poster (§3).
3. The group effect always uses the small variant (§4.8).
4. `ModifyTerrainHeight` redisplays (and posts `0x278E6003`) in a dry run when vertices would change (§4.7).
5. `DemolishOccupant` leaves `*cost` untouched when it refuses a flag-`0x40` occupant (§4.3 step 1).
6. A failed `DemolishOccupant` inside `DemolishRegion` does not make the region call fail (§4.4 step 5); only a
   blocked lot or the player-privilege test does.
7. Lots are cleared in reverse discovery order (§4.4 step 7); agriculture and industrial lots are re-parcelled into
   1×1 lots unless `0x800` is set.
8. The destroyed-lot surcharge is computed from the lot size bytes and is added in dry runs too.
9. The original dereferences the 3D view without a null test (§4.4 step 2).
10. The deferred-lot handler leaks the lot reference when the lot is not initialized (§3, DM10).
11. The 3D view's filter gates empty lots and effects even when `0x2000` or privilege 0 keeps it out of Φ (§4.4
    step 2, DM12).

## 11. Open questions

See QUESTIONS.md, section "From the re-specs10 specs" (DM1–DM4), and the cores12a questions DM5–DM13 and DM14 (the
all-null `DemolishLots` list) filed in the same section; all answered.

## 12. Conformance tests (must exist in `engine/conformance` if the class is replaced)

1. Record: `Write` emits exactly `01 00`; `Read` of `01 00` succeeds, of `02 00` fails.
2. Empty list: `DemolishLots(true, [], …, &c, …)` returns true and leaves `c` unchanged.
3. Building price: a lot with a building whose `0x099AFACD` is 1234 and nothing else priced, `DemolishLots(false,
   [lot], 0, 1, …, &c, …)` → true, `c = 1234`, nothing removed.
4. Destroyed lot: a 3×2 lot in state 6 with no occupants → `c = 12` with the shipped tuning, also with `doIt =
   false`.
5. Not demolishable: a lot whose `IsInDemolishableState()` is false in `DemolishLots` → true, `c = 0`, untouched.
6. Blocked lot: a lot whose building has the player lock, `DemolishRegion(…, privilege 1, …)` → false; the other
   lots of the region are still demolished when `doIt`.
7. Re-parcel: committing over a 2×2 zone-8 lot with flags 1 deletes it and creates four 1×1 lots; with `0x801` it
   keeps the lot in state 1.
8. Dezone: `dezone = true` (or a zone-15 lot) leaves zone 0 on the lot's cells.
9. Explode: flags `0x303` call `ExplodeLot(lot, true)` and never `StartDemolishLot`; `0x1303` passes `false`.
10. Power pole: demolishing a pole sends `0xAA7A014D` once before the removal; an orphaned neighbour pole is removed
    in the same call.
11. Networks: three road occupants in a region are destroyed by **one** `DestroyNetworkOccupants` call between
    `QueueRemovals(true)` and `QueueRemovals(false)`.
12. `ModifyTerrainHeight(true, true, …)` does not change a vertex under a building's bounding box or on a locked
    lot, and returns the number of changed vertices.
13. Effects: with cause byte 0 no effect is created; a dry run never creates one.
14. Tolerance: with `h = 0` and a vertex altitude whose `f32` difference from `h` is exactly `0.05f32`, the vertex
    is counted by neither mode (§4.7); the next `f32` above is counted in area mode and set in per-vertex mode.
15. All-null list: `DemolishLots(true, [null, null], …, &c, …)` returns true and leaves `c` unchanged (§4.5).
