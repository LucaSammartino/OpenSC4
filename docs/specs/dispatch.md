# Spec: the dispatch manager (`cSC4DispatchManager`)

Status: **behaviour spec, short form** (the component is mostly the emergency-dispatch tool and its vehicles; this
spec covers the dispatch records, the messages the simulators react to, and the save record), clean-room input.
This document describes *behaviour*, not code. It was written by the analysis team from observing the original
program and its data files. Implementers must work from this spec, the public LGPL gzcom-dll SDK headers and general
knowledge only. They must **not** consult decompiled or disassembled code (see
[CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (components §3.6, streams §2, object references §2.7, messages §5),
[protection.md](protection.md) (§8 the dispatch sources, units and call IDs; §5 the begin/active/inactive/end
messages; §15.4 the dispatch-source interface), [advisors.md](advisors.md) (`0xAA1B2002` DisasterPeriodEnded),
[city-situation.md](city-situation.md) (uses the vehicle view), [docs/abi/layouts.md](../abi/layouts.md) (the
0x2C-byte `DispatchInfo`).

## 0. Summary

The dispatch manager keeps the list of **emergency dispatches**: fire engines, police cars, fire planes and police
helicopters that the **player** sends with the dispatch tool. It owns no simulator agent and no timer; it changes
state only when

- the dispatch tool calls it (`BeginDispatchTool`, `Dispatch`, `RevokeDispatchByPylon`, `EndDispatchTool`),
- a dispatched vehicle (automaton) reports an event,
- an occupant is removed, or a disaster period ends.

The game never dispatches by itself: no shipped code path calls `Dispatch` except the dispatch tool
(`0x004BA1B0`). In an unattended run the list stays empty and the record is the 22-byte empty form of §7.

The simulation side is small: the dispatch **sources** (the fire and police simulators and their air sources,
protection.md §8) hand out units; the manager posts four messages that the protection simulators use for their
dispatch cones (protection.md §5). Everything else (pylons, vehicles, the vehicle view) is presentation and
automata.

Implementation size: the record, the list and the messages are small (about 300 lines); the pylon layout and the
automata hand-off are UI/automata work outside the simulation milestone.

## 1. Identity

| Item | Value |
|---|---|
| Class | `cSC4DispatchManager`, class ID `0x4990BD93` |
| Component | city component **3**, interface `cISC4DispatchManager` (`0x694028EC`); `cISC4City` slot 39 (`+0x9C`, `GetDispatchManager`) |
| Global | `0x00B43D04` |
| Factory | `0x0046A7B0`, registered by the game COM director; global-heap allocation of **0x4C** bytes, constructor `0x0046A510` |
| QI (`0x004695B0`) | `1`, `0x694028EC` → `+0`; `cIGZMessageTarget2` `0x452294AA` → `+4`; `cIGZSerializable` `0xE4FDA3D4` → `+8`; else false |
| Vtables | primary `0x00A88108` (24 slots, §8); message target `0x00A880F8` (`DoMessage` `0x0046A320`); serializable `0x00A880E0` (`Write` `0x0046A250`, `Read` `0x0046A820`, `GetGZCLSID` `0x004695A0`) |
| Reference count | `+0x10`; `AddRef` shared (`0x004F7BA0`); `Release` (`0x00469760`) calls `Shutdown` and the deleting destructor (slot 23) when the count would drop from 1 |

## 2. State

| Offset | Field | Initial | Saved |
|---|---|---|---|
| `+0x0C` | `initialized` (byte) | 0 | no |
| `+0x10` | reference count | 0 | no |
| `+0x14` | `loaded` (byte): set by `Read` (before the version check) | 0 | no |
| `+0x18` | message server | null | no |
| `+0x1C` | city | null | no |
| `+0x20` | **dispatch list**: STLport `list<DispatchInfo>` (nodes of 0x34 bytes from the STL node allocator, the record at node `+8`) | empty | **yes** |
| `+0x24` | `nextID`: the last dispatch ID handed out | 0 | **yes** |
| `+0x28` | the dispatch **vehicle view** (`cSC4DispatchVehicleView`, 0xA0 bytes, registered with the renderer `0x00B43DD0` at layer 5, priority 1000) | null | no |
| `+0x2C` | the **active pylon** (the one the player is dragging) | null | **yes** (ref) |
| `+0x30`–`+0x38` | **available pylons**: vector of `cISC4PylonOccupant*` (one per free unit of the open tool's type) | empty | **yes** (refs) |
| `+0x3C` | `toolType`: the dispatch type whose tool is open; **8 = none** | 8 | **yes** |
| `+0x40`–`+0x48` | registered dispatch **sources**: vector of `cRZAutoRefCount<cISC4DispatchSource>` | empty | no (sources register again at their `Init`) |

The record `DispatchInfo` (0x2C bytes) is specified in docs/abi/layouts.md: type, ID, call ID (the station's unit
number), vehicle-view handle, `active` byte, target x/y/z, station occupant, automaton, pylon.

**Dispatch types** *[inferred]* from the sources: **0** police car, **1** fire engine, **5** fire plane, **6**
police helicopter; **8** means "no tool". No shipped source handles 2, 3, 4 or 7.

## 3. Agents, messages and cadence

No simulator agent, no `AddAgent` name, no animation tick.

Subscriptions (`Init`; removed in `Shutdown`):

| Message | Handling |
|---|---|
| `0x99EF1143` RemoveOccupant (data1 = occupant) | every record whose **station** is that occupant is dropped **without** `EndDispatch`: untrack and despawn its vehicle (automaton slot 13 with 0), shut down and release its pylon (clearing the active pylon if it is that one), release the station, erase the record. **No end message, no inactive message, no unit returned** *[quirk]*. Then every available pylon whose source occupant is that occupant is shut down, released and removed (clearing the active pylon if needed) |
| `0xAA1B2002` DisasterPeriodEnded | every record's vehicle is told to go home (automaton slot 29); records stay until the vehicles report back |
| `0xA6B79602` view changed | the vehicle view refreshes (presentation) |

Direct deliveries (no subscription): each vehicle automaton the manager creates gets the manager as its event
target, so the automaton's **`0x09D92FC3`** events arrive at `DoMessage`. data1 = the automaton, data2 = event
code:

- **3** (the vehicle is gone): `RevokeDispatchByID` of its record;
- **7** (state change), data3 = the new dispatch state: `UpdateDispatchState(record, state)` (§4.3); if the new
  state is **0**, also `RevokeDispatchByID`.

Other event codes, or a null automaton, return false; an automaton with no record returns true.

Posted (`MessagePost`, queued, priority 0; IGZUnknown = the manager, data1 = dispatch type, data2 = dispatch ID):

| Message | When |
|---|---|
| `0xE94671EB` begin | `BeginDispatch` succeeded (§4.2) |
| `0xE94671EC` active | the record's state becomes 5 |
| `0xE94671ED` inactive | the record's state leaves 5 |
| `0x894671F6` end | `EndDispatch` (§4.4) |

The protection simulators react to them (protection.md §5 table, §8.3). Because they are **queued**, a handler that
calls `GetDispatchInfo(data2)` may find the record already gone.

## 4. Behaviour

### 4.1 `Init` (`0x0046A050`), `Shutdown` (`0x0046A1A0`)

`Init` (once): hold the message server and subscribe (§3); create the vehicle view and register it with the
renderer; hold the city. If `loaded`: `toolType := 8`, `SetActivePylon(null)`, clear the available pylons, and
re-track every record's vehicle in the vehicle view (types 0 and 6 as kind 1, types 1 and 5 as kind 0, other
types untracked; the new handle goes to record `+0x0C`). Return AL = 1.

`Shutdown`: unsubscribe and release the message server; shut down and release the active pylon; clear the available
pylons; unregister and release the vehicle view; release the city. Records are **kept** (they are released by the
destructor).

### 4.2 `Dispatch(type, x, z)` (`0x0046A630`, slot 6) → ID *[compat]*

1. `source := GetDispatchSource(type)` (the first registered source whose `CanHandleDispatchType(type)` is true).
   If none, or `source.CanDispatch(x, z, &allBusy)` is false (protection.md §8.2): return **0**.
2. `pylon := activePylon`. If null and `allBusy`: return 0. If non-null and a record already holds this pylon
   (the player dragged a unit that is out): set that record's target x and z and its y := terrain altitude at
   (x, z), and go to step 5 with that record.
3. New record, all zero: `type`; `ID := ++nextID` (consumed even if the dispatch then fails); target `(x,
   altitude(x, z), z)`.
4. `InitDispatch(record)`; if false, return 0 (the record is discarded). Otherwise append it to the list.
   - Without a pylon: `station := source.DispatchFromNearestStation(x, z, &record.callID)`; null → false. Hold
     the station. If `type = toolType`, the available pylon whose call number equals the call ID is taken out of
     the list and becomes the record's pylon. Otherwise a new pylon is created, initialised and given the station
     as its source, **but not stored** — the record's pylon stays null and the new pylon is leaked *[quirk]*.
   - With a pylon: `station := pylon's source occupant` (held); the record holds the pylon; `callID := pylon's
     call number` (low 16 bits); `source.DispatchFromStation(station, &callID)` (result ignored); the pylon leaves
     the available list.
   - If the record has a pylon: move it to the target and set its dispatch type. Return true.
5. `BeginDispatch(record)`: find the automata manager for the source's vehicle group
   (`GetDispatchAutomataProperties`, protection.md §8.2), create the vehicle (or re-target the existing one), set
   its dispatch ID, call ID and state 3, track it in the vehicle view, and post **begin**. On success return the
   record's ID; on failure `RevokeDispatchByID(ID)` and return 0.

### 4.3 `UpdateDispatchState(record, state)`

If `(state = 5) ≠ record.active`: `record.active := (state = 5)` and post **active** or **inactive**. Then, if the
record has a pylon, set the pylon's dispatch state. States used: 0 idle (back home), 3 dispatched, 5 working at the
scene *[inferred names]*.

### 4.4 Ending and revoking

- **`EndDispatch(record)`** (`0x0046ACE0`): `record.active := false` **without** posting inactive *[quirk]*;
  `source := GetDispatchSource(type)`, null → return false. `source.RevokeDispatchFromStation(station, callID)`
  (returns the unit, protection.md §8.2). If the record has a pylon: clear the active pylon if it is this one; if
  `type = toolType`, return the pylon to its station (below), else shut it down; release it. Untrack and release the
  vehicle; release the station; post **end**; return true.
- **`RevokeDispatchByID(id)`** (slot 7): `EndDispatch` on the record with that ID, then erase it; AL = the
  `EndDispatch` result, or false when no record has the ID.
- **`RevokeDispatchByPylon(p)`** (slot 8): if a record holds `p`, `RevokeDispatchByID(its ID)`; in every case
  then return `p` to its station via the source of `p`'s dispatch type. AL = the revoke result, false when no record.
- **`RevokeAllDispatches(type)`** (slot 9): revokes **every** record, whatever its type *[quirk]*; AL = 1.
- **Return a pylon to its station**: a pylon without a source occupant is shut down and released. Otherwise it is
  placed back at its parking spot next to the station (the same layout as §4.5), its dispatch state set to 0 and UI
  state to 1, and it is added to the available list (with a reference) unless already there.

A revoke therefore never posts **inactive**: a dispatch revoked while active leaves its cone to the protection
simulators' next full rebuild (protection.md §8.3 assumes inactive comes first).

### 4.5 The dispatch tool *[compat for the slots; presentation for the layout]*

- **`BeginDispatchTool(type)`** (slot 16): if `type = toolType`, AL = 1. If there is no source for `type`, AL = 0.
  Otherwise `toolType := type`, clear the available pylons, and for each of the source's stations (`GetStations`)
  with unit state `(mask, base, count)`: lay the station's units out along its lot diagonal (a station with fewer
  than two units uses its own position), and create one pylon per **free** unit i (mask bit i set) with call number
  `base + i`, dispatch state 0, the tool's type, UI state 1, the station as source, at position `start + step ×
  (i + 1)`. AL = 1.
- **`EndDispatchTool(type)`** (slot 17): if `type = toolType`, clear the available pylons, `toolType := 8`, AL = 1;
  otherwise AL = 0.
- **`SetActivePylon(p)`** (slot 19): when it changes, the old pylon gets UI state 1 and is released; the new one is
  held and gets UI state 2.
- **`GetAvailablePylonCount(type)`** (slot 21): the number of free units (mask bits below `count`) summed over the
  source's stations; 0 without a source.
- The tool itself (`cSC4ViewInputControlDispatch`): opening calls `BeginDispatchTool`, closing `EndDispatchTool`;
  dropping the active pylon on its own station's lot calls `RevokeDispatchByPylon`, elsewhere `CanDispatch` then
  `Dispatch`; afterwards `SetActivePylon(null)`.

### 4.6 Queries

`CanDispatch(type, x, z)` (slot 5): the source's `CanDispatch`, or false without a source. `GetDispatchInfo(id,
&info)` (slot 10): docs/abi/layouts.md (the SDK names the first parameter `type`; it is the **ID**).
`GetDispatchCount(type)` (slot 11): records of that type. `GetDispatchIDArray(type, buf, &n)` (slot 12): writes the
IDs of that type in list order and returns the number written in EAX. It stops when that number equals `n`; the
comparison follows each write, so `n = 0` means no limit (docs/abi/allocators.md, *Dispatch ID array*). `n` is read,
never written. `GetAutomatonFromPylon(p)` (slot 20): the vehicle of the record that holds
`p` (borrowed), or null. `GetDispatchSource(type)` (slot 15): borrowed, may be null.

### 4.7 Worked example

Nothing dispatched, a fire station with 3 units (base 1). The player opens the fire tool: three pylons, call
numbers 1–3. They drop pylon 2 on a burning building: `CanDispatch(1, x, z)` is true; `Dispatch(1, x, z)` → ID 1
(`nextID` 0 → 1); the record takes pylon 2, `DispatchFromStation(station, 2)` clears unit 2's bit, a fire engine
automaton is created; **begin** (1, 1) is posted. On arrival the automaton reports state 5: **active** (1, 1);
the protection simulator adds the dispatch cone. When the fire is out it reports state 0: **inactive** (1, 1), then
`RevokeDispatchByID(1)`: unit 2 returns, pylon 2 goes back to the station (the tool is still open), **end**
(1, 1). Saved now, the record is `02 00 | 00000000 | 01000000 | 00000000 | 03000000 + three pylon refs | 01000000`
(tool still open, type 1).

## 5. Random numbers

None.

## 6. Tuning

None of its own. Unit counts, call-number bases and the automata groups come from the dispatch sources
(protection.md §8).

## 7. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0x4990BD93`) *[compat]*

| Field | Encoding |
|---|---|
| version | u16, **2** |
| record count n | u32 |
| n records | each: `type` s32; `ID` u32; `callID` u32; `active` u8 (0/1); target x, y, z float32 ×3; station ref (IID `0x26BB767E`); automaton ref (`0xA9B40F05`); pylon ref (`0xC9B6D7D6`) — refs per foundations §2.7 (4 bytes when null, else 8). The vehicle-view handle is **not** saved |
| `nextID` | u32 |
| active pylon | ref (`0xC9B6D7D6`) |
| available pylon count m | u32 |
| m pylons | refs (`0xC9B6D7D6`) |
| `toolType` | u32 |

`Write` returns `GetError() = 0` (EAX 0/1). `Read` sets `loaded` first; a version other than 2 returns false; the
records are **appended** to the current list (it is not cleared), the pylon refs pushed onto the current vector;
returns `GetError() = 0`. `Init` then resets `toolType` to 8 and drops the active and available pylons (§4.1), so
the saved tool state has no effect.

**Corpus** (421 saves): every record has no dispatch and no pylon, so it is 22 bytes:
`02 00 | 00 00 00 00 | nextID | 00 00 00 00 | 00 00 00 00 | toolType`.

| `nextID` | `toolType` | Saves |
|---|---|---|
| 0 | **4** | 409 (the bundled regions' cities) |
| 0 | 8 | 8 (Berlin Spandau and Tegel, FC4 Test Spandau and Tegel, London Fulham and Kensington, Big City and Rush Hour tutorials) |
| 1 | 8 | 2 (Berlin Konradshohe; Making Money Tutorial): one dispatch was made and ended |
| 0 | 5 | 1 (Getting Started Tutorial) |
| 0 | 7 | 1 (New City (32)) |

`toolType` 4, 5 and 7 cannot be produced by the shipped tool's "none" (8) unless the tool was open; the 409 saves
with 4 were presumably written by an earlier build whose "none" was 4 *[inferred]*. All values load identically.

## 8. Interface slot table *[compat]*

`cISC4DispatchManager` (vtable `0x00A88108`).

| Slot | Offset | Method | Original | Args | Pops | Return |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface` | `0x004695B0` | 2 | 8 | AL |
| 1 | `+0x04` | `AddRef` | `0x004F7BA0` | 0 | 0 | EAX |
| 2 | `+0x08` | `Release` | `0x00469760` | 0 | 0 | EAX |
| 3 | `+0x0C` | `Init` | `0x0046A050` | 0 | 0 | AL (1) |
| 4 | `+0x10` | `Shutdown` | `0x0046A1A0` | 0 | 0 | AL (1) |
| 5 | `+0x14` | `CanDispatch(type, float x, float z)` | `0x00469680` | 3 | 12 | AL |
| 6 | `+0x18` | `Dispatch(type, float x, float z)` → ID | `0x0046A630` | 3 | 12 | EAX (0 = failed) |
| 7 | `+0x1C` | `RevokeDispatchByID(id)` | `0x0046AEC0` | 1 | 4 | AL |
| 8 | `+0x20` | `RevokeDispatchByPylon(pylon)` | `0x0046AE70` | 1 | 4 | AL |
| 9 | `+0x24` | `RevokeAllDispatches(type)` — **SDK error**: the SDK declares no argument | `0x00469D10` | 1 | 4 | AL (1) |
| 10 | `+0x28` | `GetDispatchInfo(id, DispatchInfo&)` | `0x00469D40` | 2 | 8 | AL |
| 11 | `+0x2C` | `GetDispatchCount(type)` | `0x00469D80` | 1 | 4 | EAX |
| 12 | `+0x30` | `GetDispatchIDArray(type, u32* buf, u32& n)` | `0x00469DA0` | 3 | 12 | EAX (number written) |
| 13 | `+0x34` | `RegisterDispatchSource(src)` (holds a reference; no duplicate check) | `0x0046AE00` | 1 | 4 | none |
| 14 | `+0x38` | `UnregisterDispatchSource(src)` (first match; releases) | `0x0046A5D0` | 1 | 4 | none |
| 15 | `+0x3C` | `GetDispatchSource(type)` | `0x004698B0` | 1 | 4 | EAX (borrowed) |
| 16 | `+0x40` | `BeginDispatchTool(type)` | `0x0046B060` | 1 | 4 | AL |
| 17 | `+0x44` | `EndDispatchTool(type)` | `0x0046A4E0` | 1 | 4 | AL |
| 18 | `+0x48` | `GetActivePylon()` | `0x004ABAD0` (shared getter of `+0x2C`) | 0 | 0 | EAX (borrowed) |
| 19 | `+0x4C` | `SetActivePylon(pylon)` | `0x00469630` | 1 | 4 | none |
| 20 | `+0x50` | `GetAutomatonFromPylon(pylon)` | `0x00469DE0` | 1 | 4 | EAX (borrowed, 0 if none) |
| 21 | `+0x54` | `GetAvailablePylonCount(type)` | `0x0046AF10` | 1 | 4 | EAX |
| 22 | `+0x58` | `GetDispatchVehicleView()` | `0x0060FB80` (shared getter of `+0x28`) | 0 | 0 | EAX (borrowed) |
| 23 | `+0x5C` | deleting destructor `(flags)` (not in the SDK) | `0x0046A7D0` | 1 | 4 | EAX |

## 9. Who reaches in

| Caller | Slots |
|---|---|
| fire and police simulators (protection.md) | 13/14 at `Init`/`Shutdown`; 10 on the active/inactive/end messages; 11 inside their `CanDispatch` |
| fire-plane and police-helicopter sources (protection.md §8.4) | 13/14, 10 |
| the dispatch tool `cSC4ViewInputControlDispatch` (`0x004BA1B0`, `0x004BA510`) | 5, 6, 8, 16, 17, 18, 19 |
| MySims, the My Street sim, the player-drive control, the city situation manager, the automata manager | 22 (track and untrack vehicles in the vehicle view) |

## 10. Quirks (reproduced)

1. A station removed while it has dispatches drops them silently: no end message, no unit returned (§3).
2. `EndDispatch` clears `active` without posting inactive (§4.4).
3. `RevokeAllDispatches(type)` ignores `type` (§4.4).
4. `InitDispatch` without a pylon and outside the open tool creates a pylon it never stores (§4.2).
5. `nextID` advances even when the dispatch fails.
6. `Read` appends to the existing list and sets `loaded` before checking the version (§7).
7. `GetDispatchIDArray` with `n = 0` writes every matching ID: the limit is checked only after a write.

## 11. Open questions

See QUESTIONS.md, section "From the re-specs10 specs" (DP1–DP3).

## 12. Conformance tests

1. Empty record: a fresh manager writes `02 00 00000000 00000000 00000000 00000000 08000000`.
2. Round trip of the 22-byte corpus forms, including `toolType` 4; after `Init`, `toolType` is 8.
3. `Read` of version 1 or 3 fails; `loaded` is still set.
4. `Dispatch` with no registered source returns 0 and `nextID` is unchanged; with a source whose `CanDispatch`
   is true and a free unit, returns `nextID + 1`, appends one record and posts begin once.
5. A failed `InitDispatch` (no station found) still advances `nextID`.
6. Automaton event 7 with state 5 posts active once; a second state 5 posts nothing; state 0 posts inactive, then
   end, and removes the record.
7. `RevokeDispatchByID` of an active record posts end only (no inactive) and returns the unit
   (`RevokeDispatchFromStation` called once).
8. `RevokeAllDispatches(1)` also removes type-0 records.
9. RemoveOccupant of a station with two records removes both, posts nothing and never calls
   `RevokeDispatchFromStation`.
10. `BeginDispatchTool(1)` with one station of 3 units, mask `0b101`: two pylons (call numbers base and base + 2);
    `GetAvailablePylonCount(1)` = 2; `EndDispatchTool(0)` → false, `EndDispatchTool(1)` → true and the list is empty.
