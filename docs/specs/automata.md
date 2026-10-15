# Spec: the automata layer (saved records and game hooks)

Status: **behaviour spec, short form** (the layer is mostly animation; §0 says what that means), clean-room input.
This document describes *behaviour*, not code. It was written by the analysis team from observing the original
program and its data files. Implementers must work from this spec, the public LGPL gzcom-dll SDK headers and
general knowledge only. They must **not** consult decompiled or disassembled code (see
[CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (streams §2, the class-object store and refs §3.3–§3.6, messages
§5), [save-records.md](save-records.md) (§3.2: these six components are still original), [lot-records.md](lot-records.md)
(§2.3: the occupant base record), [disasters.md](disasters.md) (§5 fire, §6 riot), [civic-buildings.md](civic-buildings.md)
(the ape escape), [dispatch.md](dispatch.md) (crews), [city-situation.md](city-situation.md) (U-Drive-It),
[clock24.md](clock24.md) (the 24-hour clock and `HourChanged`), [crime.md](crime.md), [advisors.md](advisors.md).

## 0. Summary and scope

The automata layer puts cars, people, planes, boats, animals and props in motion. Almost all of it is decoration.
This short form specifies only what the simulation and the save file depend on:

- the records of the six saved components (§2–§5) and of every object they reference (§6), so that a save made by
  the original loads and re-saves byte-compatibly;
- the calls other specs make into the layer (§7) and the riot, fire and U-Drive-It hooks (§8, §9);
- the message subscriptions (§10) and the interface slot tables (§11).

It does **not** specify movement, path following, animation, sound, the Lua automata scripts (`lua-host.md`), or
how automata are chosen and spawned. A replacement may animate however it likes, or not at all, provided it keeps
the records and the hooks.

| # | Component | Interface (IID) | Class ID | City slot | Saved version |
|---|---|---|---|---|---|
| 40 | automata script system | `cISC4AutomataScriptSystem` `0x8A1B6647` | `0x4A1B664E` | 48 | 1 |
| 41 | automata controller manager | `cISC4AutomataControllerManager` `0xCA14C9FC` | `0xCA14CA03` | 47 | 1 |
| 43 | vehicle manager | `cISC4AutomataManager` `0x098F9BF4` | `0xA98F9D3B` | 43 | 3 (2 read) |
| 44 | pedestrian manager | `cISC4AutomataManager` `0x098F9BF4` | `0xC98F9D17` | 44 | 3 (2 read) |
| 45 | aircraft manager | `cISC4AutomataManager` `0x098F9BF4` | `0xC98F9D4A` | 45 | 5 (≤ 5 read) |
| 46 | watercraft manager | `cISC4AutomataManager` `0x098F9BF4` | `0xA98F9D28` | 46 | 6 (≤ 6 read) |

Every save in the corpus (421 cities) holds components 40, 41, 43 and 44. Only 12 hold 45 and 46.

## 1. Encodings used below *[compat]*

All values are little-endian (foundations §2).

| Name | Bytes |
|---|---|
| `u8`, `u16`, `s16`, `u32`, `s32`, `f32` | as named |
| `bool` | one byte, 0 or 1 |
| `vec3` | three `f32`: x, y, z |
| `ref` | `u32` object ID; when non-zero, followed by `u32` class ID (foundations §3.4). 0 is null and has no class. |
| `str` | `u32` length, then that many bytes, no terminator |
| `key3` | three `u32`: type, group, instance |
| `time` | a *controller time*: `f32` hour (0 ≤ h < 24), then `s16` day of week (0–6) |
| `curve` | `u32` n, then n × (`f32` x, `f32` y) |
| `bytes` | `u32` n, then n bytes |

"Read checks IID X" means the loader resolves the ref and requires that the object answers `QueryInterface(X)`.
The IIDs used are: occupant `0x26BB767E`, automaton `0xA9B40F05`, controller `0x6A1857F8`, lot `0xE818FCED`,
network occupant `0xA821EF94`, 24-hour clock `0x681BD53A`, model `0xA818EDD8`, callback target `0x452294AA`.

## 2. Component 40: the script system (`0x8A1B6647` / `0x4A1B664E`) *[compat]*

The script system holds the automata groups, occupant groups and controller templates that the Lua automata
scripts define. None of that is saved; it is rebuilt from the scripts at `Init`.

Record: `u16` 1. Nothing else.

- Write (`0x0042E440`, a body shared with the nation) writes the `u16` and returns `GetError() == 0`.
- Read (`0x006B8A70`, shared with the industrial simulator) reads the `u16` and fails when it is not 1.
- Corpus: 421 of 421 records are the 2 bytes `01 00`.

## 3. Component 41: the controller manager (`0xCA14C9FC` / `0xCA14CA03`) *[compat]*

A **controller** is attached to an occupant and makes automata near it: a *generator* spawns them (wildlife in
trees, construction vehicles at construction sites, dogs, crowds, strike pickets), an *attractor* pulls them in
(police at a crime scene, dogs to a hydrant). Controllers are made from named *templates* that the scripts define.

### 3.1 Record (Write `0x005174D0`, Read `0x0051B520`)

| # | Field | Notes |
|---|---|---|
| 1 | `u16` version = 1 | Read does not check it. |
| 2 | `ref` the 24-hour clock | Read checks IID `0x681BD53A`. Always class `0xA7E7F929` in the corpus. |
| 3 | `time` A | |
| 4 | `time` B | slot 24 `GetControllerTime` returns this one. A equals B in every corpus save. |
| 5 | `u32` n1, then n1 × occupant entry | the controllers, grouped by occupant (§3.2) |
| 6 | `u32` n2, then n2 × automaton set | §3.3 |

The two times are the controller manager's view of the in-game time of day: they advance with the clock's
`HourChanged` messages (clock24.md) [inferred].

### 3.2 Occupant entries

Only controllers without **flag 8** (controller slot 12 `CheckFlag(8)`, `0x00517E60`) are saved. Flag 8 marks a
transient controller; such controllers are never written anywhere (also not from automata, §6.1).

Each entry, for each occupant that has at least one controller without flag 8, in the order of the manager's
occupant hash map:

1. `ref` occupant (read checks `0x26BB767E`);
2. `u32` m = the number of its controllers without flag 8;
3. m × `ref` controller (read checks `0x6A1857F8`), in list order.

A replacement must write the entries in some fixed order and need not reproduce the hash order *[choice]*; the
original reads any order.

### 3.3 Automaton sets

Each entry: `ref` occupant (`0x26BB767E`), `u32` k, k × `ref` automaton (`0xA9B40F05`), `vec3`, `vec3`, `bool`.
Only one corpus save (Big City Tutorial) has an entry: a building with k = 0, the vectors (1256, 0, 1041) and
(−1, 0, 0), and `true`. The meaning of the vectors is open (Q2).

### 3.4 Load

Read sets a *loaded* flag. When the manager later handles `PostCityInit` (`0x26D31EC1`, `HandlePostCityInit`
`0x00518660`), and only when loaded, it visits every saved controller:

1. Look up the controller's template by name (script system slot 17, `GetControllerTemplate(name)`).
2. If there is none: log *Couldn't find automata controller template '<name>' in scripts. Removing controller.*,
   shut the controller down and release it.
3. Otherwise set the template (controller slot 15), `Init` it (slot 3) and add it to the controller grid.

So a save survives a changed script set: unknown templates are dropped, not fatal.

### 3.5 Controller records

Two controller classes occur in saves. Both start with the controller base record.

**Controller base, version 5** (Write `0x00517FD0`, Read `0x005180E0`; Read ignores the version):

| # | Field | Notes |
|---|---|---|
| 1 | `u16` 5 | |
| 2 | `ref` occupant | `0x26BB767E`; the occupant the controller is attached to |
| 3 | `str` template name | empty when the controller has no template |
| 4 | `vec3` position | |
| 5 | `f32`, `f32` | radius and a second extent [inferred]; 0 in all corpus examples |
| 6 | `u32` | 0 in the whole corpus |
| 7 | `u32` flags | mostly 7; `0x87` seen on `dog_make`; never contains 8 (§3.2) |
| 8 | `u32` n, then n × `ref` automaton | `0xA9B40F05`; the automata it currently controls |

**Generator** (class `0x4A1628C5`, Write `0x005183D0`, Read `0x00518450`): `u16` 4 (Read requires 4), the base
record, `time` (period), `time` (second period), `u32`, `u32`. Corpus: 291,346 records; names `wildlife_generator`
(291,095), `construction_vehicle_generator`, `dog_make`, `crowd_sm`, `transit_strike_gen_1`/`_2`/`_3` and others.
The first `u32` looks like a day stamp (16,674–16,697 seen), the second is 0–3. Both must round-trip.

**Attractor** (class `0x6A1628BA`, Write `0x00518230`, Read `0x00518270`): the base record, `u16` 1 (read and
ignored), then a `curve` (attraction against distance [inferred]). Corpus: 196 records; names `dog_pull`,
`police_attract` and others; curves like ((0, 60), (50, 60)).

Corpus result: all 291,346 generators and 196 attractors parse exactly to their ends. Occupant classes behind
the controllers: flora `0xA9C05C85` (291,095), prop `0x2977AA47` (104), construction `0xA97F909E` (43),
building `0xA9BD882D` (35), network `0xC9C05C6E` (1).

## 4. The automata manager base record (version 6) *[compat]*

All four managers start with this record (Write `0x0050CB80`; Read wrapper `0x005108E0`, body `0x0050FEB0`).

| # | Field | Notes |
|---|---|---|
| 1 | `u16` version = 6 | Read accepts ≤ 6 |
| 2 | `s32` max count | applied with slot 23 `SetMaxCount` (§7.3) |
| 3 | `bool` active | slot 5 `Activate` / slot 6 `IsActive` |
| 4 | `bool` | the time-of-day scaling flag [inferred] |
| 5 | `u32` n, then n × `ref` automaton | `0xA9B40F05`; the manager's automata |
| 6 | only when version ≥ 5: `u32` m, then m × (`u32` automata group ID, `u32` count) | the group instance counts (slot 31) |

Read sets the loaded flag, and an *old-version* flag that is true when the version is below 6. Write always
returns true, and Read returns true without checking the stream error *[quirk]*.

## 5. The four managers

### 5.1 Vehicle (43) and pedestrian (44): the traffic automata manager, version 3

Write `0x00553570`, Read `0x00554410` (accepts 2 or 3).

| # | Field | Notes |
|---|---|---|
| 1 | `u16` 3 | |
| 2 | base record (§4) | |
| 3 | `u32` | Read discards it. Vehicle manager 1, pedestrian manager 2 in every save. |
| 4 | `u32` | the ID at the manager's list cursor, 0 at the end; Read moves the cursor to the first node with that ID |
| 5 | only in version 3: `u16` 1, `u32` count, count × incident | the traffic incidents (accidents) |

An incident (Write helper `0x00552A80`, Read helper `0x00553E90`): `s32` x, `s32` z, `u32` type, 8 × `s32`, `ref`
network occupant, `bool`, `u32`. The single corpus incident is (x 47, z 22, type 2), on network occupant class
`0xC9C05C6E`, `bool` 1, last `u32` 6264.

Corpus: 410 of the 421 pairs are the 22-byte version-2 record with a version-4 base (an empty city's managers);
11 are version 3 with base 5 or 6. Max counts seen: vehicle 336, 199, 750, 168; pedestrian 384, 512, 256.

### 5.2 Aircraft (45), version 5

Write `0x004FB5B0`, Read `0x004FBE70`. Read: a version above 5 fails; a version below 5 means the whole record is a
base record of that version (the leading `u16` is the base's own).

1. `u16` 5;
2. base record;
3. `u32` count, count × (`ref` occupant `0x26BB767E`, `ref` automaton `0xA9B40F05`): the aircraft parked at or
   assigned to airports. The original keeps these in a map ordered by pointer, so its write order is arbitrary;
   a replacement may write any fixed order *[choice]*;
4. `u32` n, n × (`vec3`, `u8`).

Corpus: 12 records, 11 of version 5 and 1 of version 4.

### 5.3 Watercraft (46), version 6

Write `0x0056FA70`, Read `0x005749E0`. A version below 6 means the whole record is a base record of that version.

1. `u16` 6;
2. base record;
3. `s32` count, count × (`ref` lot `0xE818FCED`, `ref` occupant `0x26BB767E`);
4. `s32` count, count × `ref` occupant;
5. `s32` count, count × ferry entry: `ref` lot, `ref` occupant, `s32` n, n × route element.

A route element: `ref` lot, `ref` lot, `s32` kind (0, 1, otherwise −1), 4 × `u16`, `bool`, `s16`, `s16`,
`bytes`, `bytes`.

Corpus: 12 records: 7 of version 6, 4 of version 5, 1 of version 4. Worked example in §14.

## 6. Automaton records *[compat]*

The managers' lists, the controllers and the controller manager's sets reference automata by `ref`. These are
the automaton classes found in saves, with their records.

### 6.1 The automaton base record, version 8 (Write `0x0050AC00`, Read `0x0050DFD0`, accepts ≤ 8)

| # | Field | Notes |
|---|---|---|
| 1 | the occupant base record | lot-records §2.3; occupant type `0x496E5783` |
| 2 | 3 × `vec3` | |
| 3 | `f32`, `f32` | |
| 4 | `u32` automata flags, `u32` | |
| 5 | `ref` controller | `0x6A1857F8`; written as null when the controller has flag 8 |
| 6 | `f32`, `f32` | |
| 7 | `vec3` | |
| 8 | `ref` automaton | `0xA9B40F05`; the leader (slots 48/49) [inferred] |
| 9 | `u32` n, then n × (`u32` state type, state record) | the state stack (§6.2) |
| 10 | `u32` m, then m × `key3` | model keys |
| 11 | `key3` | the current model, e.g. type `0x5AD0E817` |
| 12 | `u32` | |
| 13 | `u32` k, then k × `ref` callback target | only callbacks whose target is serializable (`QueryInterface 0xE4FDA3D4`) are written; read checks `0x452294AA`. Seen: the controller manager and the city situation manager (`0x0BB14381`). |
| 14 | 8 × `f32` | |
| 15 | `u8` | |
| 16 | only in version 8: `u32` | |

Corpus: base version 7 in 79,509 automata, version 8 in the rest.

### 6.2 States

The state type selects the record. The state factory (automaton slot 94, `0x0050B7F0`) knows types 8 to `0x34`;
any other type gets the default state, whose record is the base record alone.

**State base, version 3** (Write `0x00509D00`, Read `0x00505E50`, accepts ≤ 3): `u16` 3, `ref` controller (null
when it has flag 8; read checks `0x6A1857F8`), `u32`, `f32`, `bool`, `u32`, `u32`, and in version 3 a final `u32`.

| Type | Name | After the base | Write / Read |
|---|---|---|---|
| 8 | RiotLeader | `s32`, `s32`, `u8` | `0x00509DE0` / `0x00505F50` |
| `0x0B` | | `vec3`, `f32`, `f32` | `0x00509D90` / `0x00505F00` |
| `0x0E` | | 5 × `f32` | `0x00509E70` / `0x00505FE0` |
| `0x0F` | | `f32`, `f32` | `0x00509E30` / `0x00505FA0` |
| `0x10` | PlayerDrive [inferred] | `f32` | `0x00509EE0` / `0x00506090` |
| `0x11` | | 4 × `vec3` | `0x00509F20` / `0x00506100` |
| `0x13` | AttachAnim [inferred] | `ref` model (`0xA818EDD8`), `u16`, `bool` | `0x00509FB0` / `0x0050A020` |
| `0x14` | | none | `0x00509F80` / `0x00506160` |
| `0x15` | | `f32` | `0x00509EE0` / `0x00506050` |
| `0x34` | Chase [inferred] | `ref` automaton | `0x0050A0A0` / `0x0050A0E0` |
| any other | default | none | base only |

Other names the original uses for states are RandomWalk, PlopJump, Plummet, PullOver, Bumped and Collide; their
type numbers are not tied down (Q3). Corpus state types: `0x0B` 80,519; 0 811; 2 381; 7 107; `0x0D` 101; 3 32;
`0x14` 10. State versions: 2 in 80,080, 3 in 1,881.

### 6.3 Traffic automaton, version 2 (Write `0x00551DE0`, Read `0x00554D20`, requires 2)

`u16` 2, the automaton base record, `ref` automaton (`0xA9B40F05`), `u32`, `u8`, `ref` network occupant
(`0xA821EF94`; seen class `0xC9C05C6E`), `u32`, `u32`, `u32` n, n × route step (`u8`, `u8`, `u8`).

### 6.4 Concrete classes

| Class | ID | Record (all start with `u16` version) |
|---|---|---|
| pedestrian | `0x896E75AF` | `u16` 5, traffic record, `bool`, `vec3`, `f32`, `u32` (Write `0x00544650`, Read `0x005446C0`) |
| vehicle | `0x896E714A` | `u16` 2, traffic record, `u32` n + n × (`u32`, `s32`, `s32`) path entries, `u32` 0, `vec3`, `vec3` (−1, −2, −3), `f32` (Write `0x0055F640`, Read `0x0055BCF0`) |
| emergency vehicle | (`0x896E714A`, see §12) | `u16` 2, vehicle record, `vec3`, `ref` occupant, `u8`, `u16`, `u32` (Write `0x0052FA60`, Read `0x0052FAE0`) |
| patrol car | `0x4A222B98` | `u16` 2, emergency record, `ref` automaton (Write `0x00543DB0`, Read `0x00543DF0`) |
| trailer vehicle | `0x4A5B69ED` | `u16` 2, vehicle record, `u32` n + n × `ref` vehicle, `u32` m + m × `u32`, `vec3`, `ref` network occupant, `u32` (Write `0x005562A0`, Read `0x00556C30`) |

The vehicle's Read discards the path entries and the constant vector; a replacement must still write them
(writing an empty path list is allowed *[choice]*). Corpus: 81,787 pedestrians, 1,085 vehicles, 63 trailers and
3 patrol cars, all parsing exactly to their ends. No aircraft or watercraft automata occur in the corpus (Q4).

## 7. Calls other specs use *[compat]*

### 7.1 Controller manager (city slot 47)

| Slot | Call | Behaviour |
|---|---|---|
| 5 | `AddControllerFromTemplateName(occupant, name, extra)` (`0x00514CA0`) | Look up the template by name (script slot 17). None: return null. Otherwise return slot 6 `AddControllerFromTemplate(occupant, template, extra)`. |
| 9 | `AddControllersFromOccupantGroup(groupID, occupant, extra)` (`0x00517190`) | Look up the occupant group by **ID** (script slot 11). No script system or no group: return false. Otherwise add one controller per template in the group's list (slot 6 each) and return true. |
| 10 | `AddControllersFromOccupantGroup(name, occupant, extra)` (`0x00517150`) | The same, with the group looked up by **name** (script slot 12). |
| 11 / 12 | `RemoveControllersFromOccupantGroup` by ID / by name (`0x0051AB40` / `0x0051AB00`) | Remove the controllers that the group added to the occupant. |
| 14 | `RemoveControllers(occupant)` (`0x0051A520`) | Remove all controllers of the occupant. |
| 24 | `GetControllerTime` (`0x0051A310`) | Returns `time` B (§3.1). |

Callers: the fire occupant (slot 10, `fire_occupant`, §8.2), the civic ape escape (slot 10, `ape_escape`), the
police jailbreak (slot 10 to begin, slot 12 to end), the strike states of the fire, police and residential
simulators (slot 10, then slot 12), riots (slots 5 and 14, §8.1).

### 7.2 Pedestrian manager (city slot 44) slot 15: `CreateAutomaton`

Seven arguments: automata group ID, position (`vec3` pointer), a mode, direction (`vec3` pointer), and three
`u32`. It returns the new automaton or null. The riot passes (`0x4107`, cell position, 2, (0, 0, 0), 0, `0xFF`,
0). Only the group ID and position matter to the simulation; the rest select the creation path [inferred].

### 7.3 Manager slots that other specs use

- slot 5 `Activate(bool)`, slot 6 `IsActive`;
- slot 9 `Pause`, slot 10 `Resume`, slot 11 `IsPaused` (city-situation.md);
- slot 13 `IsDrivingMode`: true when the current view's ID is `0x2B709676` (the U-Drive-It view; advisors.md);
- slot 23 `SetMaxCount(n)` (`0x0050AA60`): clamp n to 0–5000, remove automata beyond it, and set the hard limit
  to `trunc(n × 1.25)` (a `double` multiply, constant at `0x00A955A0`).

### 7.4 Automaton slots that other specs use

slot 5 `AsOccupant`; slot 9 `SetVisible(bool)`; slot 13 `Remove` (dispatch despawn); slot 29
`ReturnToSourceOccupant` (dispatch "go home"); slot 51 `SetState(type, …6 more)`; slot 2 `Release`.

## 8. Riot and fire hooks *[compat]*

### 8.1 Riot leaders (disasters.md §6)

**Start** (`AddRiotLeader`, `0x0057DD00`), in this order:

1. find the nearest road tile; none → fail;
2. stop when the leader count has reached its cap;
3. `CreateAutomaton` on the pedestrian manager (city slot 44, slot 15) with group `0x4107` (§7.2); null → fail;
4. `AddRef` the automaton;
5. automaton slot 51 `SetState(8, 0, −1.0, 1, 0, 0, 0)`: the RiotLeader state;
6. automaton slot 9 `SetVisible(true)`;
7. register the leader's location with the disaster layer (city slot 72).

**Stage rise** (`Update`, `0x0057DFE0`): format the template name `riot_leader_<stage>` (the format
`riot_leader_%d`, `0x00A9E860`, with the leader's stage) and call controller manager slot 5
`AddControllerFromTemplateName(leader as occupant, name, 0)`.

**End**, in this order:

1. automaton slot 51 `SetState(1, 0, −1.0, 0, 0, 0, 0)`;
2. controller manager slot 14 `RemoveControllers(leader as occupant)`;
3. disaster layer (city slot 72) slot 15: unregister the location by occupant;
4. release the automaton and erase the leader.

### 8.2 Fire occupant (disasters.md §5)

`SetTarget` (`0x00471900`): when the target occupant's type is `0x278128A0` (a building), call controller
manager slot **10** `AddControllersFromOccupantGroup("fire_occupant", target, 0)`. `Shutdown` (`0x00470DC0`) calls
slot 14 `RemoveControllers(target)` and drops the target. (disasters.md §5.11 said slot 9; corrected, §13 Q1.)

## 9. U-Drive-It hooks

The U-Drive-It missions belong to the city situation manager (city-situation.md). The automata layer supplies:

- manager slot 9 `Pause` / slot 10 `Resume`: when a mission activates, the city situation manager pauses all four
  managers and resumes them when its start countdown ends (city-situation.md);
- manager slot 13 `IsDrivingMode` (§7.3);
- the player's automaton runs state `0x10` (PlayerDrive, §6.2), whose record adds one `f32`;
- `CreateAutomatonInternal` (`0x00510690`) asks the city situation manager (slots 19, 27, 28) whether a new
  automaton can start a mission, and the cheat handler (`0x0050DD30`) uses its slot 33.

None of this changes a save beyond the records above.

## 10. Messages *[compat]*

| Receiver | Messages handled |
|---|---|
| controller manager (`0x00519360`) | `0xA6B79602` view update, `0x86AD10EE` animation tick (drives `DoAnimationTick`), `0x09D92FC3` automaton notifications, `0x26D31EC1` PostCityInit (§3.4), `0x44778230` HourChanged, `0x89D3E9F3`, `0x99EF1142` InsertOccupant, `0x99EF1143` RemoveOccupant (drops the occupant's controllers [inferred]) |
| controller manager, cheats (`0x005187F0`) | `0x230E27AC` with `0xABA2B5BB`: a cheat command that toggles a debug flag |
| every manager (base, `0x00506970`) | `0x99EF1143`, `0x4A7FB807` emergency pause, `0x26D31EC2` PreCityShutdown, `0x06CDB65A`, `0x26D31EC1`, `0x2977AE2F` building completion changed, `0x86AD10EE`, `0x89D3E9F3`, `0x99EF1142`, `0xCAD61E00`, `0xA6B79602`, `0xA6B79612`, `0xC9F775BB`, `0xEA298D50`, `0xEAD61E16`, `0xEBFF18A3` |
| pedestrian manager (`0x00545450`) | adds `0x230E27AC` (cheats) and `0xA9E3B6E2` (crowd reaction), then the base |
| vehicle manager (`0x00562620`) | adds `0xA6B79611`, `0x0772FAD4` FundsChanged, `0x44778234` DayNightChanged, `0xA6B79601`, `0xC53D10AA` exemplar changed, `0xC9DA96E7`, `0xC9DA96E9` dusk, then the base |
| aircraft manager (`0x004FC710`) | adds `0x99EF1142` / `0x99EF1143` (`HandleOccupantRemoved` `0x004FB8D0` keeps §5.2 consistent), then the base |
| watercraft manager (`0x00575500`) | adds `0x2C06BA82` ferry pass finished, `0x99EF1142` / `0x99EF1143` (§5.3 lists), then the base |

The occupant insert and remove handlers keep the saved lists of §3.2, §5.2 and §5.3 consistent: an occupant that
leaves the city takes its controllers, airport pairs and ferry entries with it. A replacement must do the same,
or it will write refs to objects that are no longer saved.

## 11. Interface slot tables

`cISC4AutomataControllerManager` (vtable `0x00A96058`) is already in `docs/abi/interfaces.tsv`. The proposed rows
for the three below are in `build-runs/re-specs13a-abi-rows.tsv` of this lane.

**`cISC4AutomataScriptSystem`** (vtable `0x00A96BA8`): 0 QueryInterface, 1 AddRef, 2 Release, 3 Init, 4 Shutdown,
5 GetAutomataGroupMap, 6 GetAutomataGroup (2 args), 7 GetAutomataGroup (1 arg), 8 GetOccupantGroupNameMap,
9 GetOccupantGroupIDMap, 10 GetSpecialFilterOccupantGroups, 11 GetOccupantGroup(ID) `0x0051CFF0`,
12 GetOccupantGroup(name) `0x0051CFC0`, 13 CreateOccupantGroup, 14 GetDriveCommand, 15 GetControllerTemplateMap,
16 CreateControllerTemplate, 17 GetControllerTemplate(name) `0x0051CF90`, 18 GetTrackBuildingGroups.

**`cISC4AutomataManager`** (pedestrian `0x00A9A470`, vehicle `0x00A9D1E0`, aircraft `0x00A94850`, watercraft
`0x00A9DF38`): 0 QueryInterface, 1 AddRef, 2 Release, 3 Init, 4 Shutdown, 5 Activate, 6 IsActive, 7 AddAutomaton,
8 RemoveAutomaton, 9 Pause, 10 Resume, 11 IsPaused, 12 GetAutomataCount, 13 IsDrivingMode, 14 GetMinZoom,
15 CreateAutomaton (7 args), 16 GetRandomCreationPoint (7 args), 17–18 Dispatch, 19–20 TranslateDispatchLocation,
21–22 ChaseAutomaton, 23 SetMaxCount, 24 HandleTuningParametersChanged, 25 ChooseRandomExemplar,
26 ChooseRandomExemplarByFilter, 27 ChooseExemplarByCrimeType, 28 GetExemplarData, 29 GetExemplarDataMap,
30 CanCreateGroup, 31 TrackAutomataGroupInstances, 32 GetInstancesOfGroup, 33 ReleaseAutomatonSet,
34 SetCameraFollow, 35 IsCameraFollowUpdating, 36 GetSourceBuildingCount, 37 GetSourceBuildingCountForAutomataGroup.
The serializable views are at `0x00A9A448`, `0x00A9D1B4`, `0x00A94828` and `0x00A9DF10`.

**`cISC4Automaton`** (pedestrian primary vtable `0x00A9A718`), slots 0–69: 0–4 QueryInterface, AddRef, Release,
Init, Shutdown; 5–6 AsOccupant; 7 GetPosition; 8 SetPosition; 9 SetVisible; 10 SetDestination; 11 GetDestination;
12 GetTransform; 13 Remove; 14 SetOrientation; 15 GetOrientation; 16 MoveTo; 17 GetRadius; 18 GetDisabled;
19–20 PlayAnim; 21 PlayAnimByName; 22 AttachAnim; 23–28 call number, dispatch state and dispatch info ID (get/set
pairs); 29 ReturnToSourceOccupant; 30 SetAutomataFlag; 31 CheckAutomataFlag; 32–33 opacity; 34–38 speeds;
39–40 Add/RemoveCallback; 41–47 effects; 48 GetLeader; 49 SetLeader; 50 FindDrivingAutomaton; 51 SetState;
52 PushState; 53 PopState; 54 SetBehavior; 55 PushBehavior; 56 GetState; 57 FindState; 58 PrevState;
59 GetBehavior; 60 HandlePlayerDriveCommand; 61–62 Start/StopSFX; 63–64 priority; 65 HandleCollision;
66 GetCollidePosition; 67 GetAutomataGroup; 68–69 Register/UnregisterInGameDataRegistry.

Overload pairs (script 6/7 and 11/12, manager 17–22, automaton 5/6 and 19/20) are in the Windows order; the
argument types given above for script slots 11 and 12 are checked, the others are not.

## 12. Quirks *[quirk]*

1. **Flag-8 controllers are never saved**, and refs to them from automata and states are written as null (§3.2,
   §6.1, §6.2).
2. **Duplicate occupant keys in the controller manager record corrupt memory.** When Read meets an occupant it
   has already seen, it appends to a stale list pointer. The original never writes duplicates. A replacement must
   skip the repeated entry's controllers (read them, then drop them) *[choice]*.
3. **An emergency vehicle reloads as a plain vehicle.** Its class ID is the vehicle's (`0x896E714A`, the shared
   `GetGZCLSID` `0x00532240`), so the loader builds a vehicle, reads the vehicle part and leaves the rest of the
   record unread; the read of the next object then fails. The corpus has none, because emergency vehicles are
   not in the managers' saved lists [inferred]. A replacement must not save emergency vehicles *[choice]*.
4. **The manager base Read does not report stream errors** (§4).
5. **Discarded fields.** The traffic manager's `u32` (§5.1 field 3), the vehicle's path entries and constant vector
   (§6.4) and the controller manager's version are read and thrown away.
6. **Pointer-order maps.** The aircraft manager's pairs (§5.2) are written in pointer order, so two saves of the
   same city may differ in order.
7. **The controller manager's loaded flag gates `PostCityInit`.** A city that was not loaded (a new city) skips
   the template re-binding.

## 13. Open questions

- **Q1 (settled here).** disasters.md §5.11 step 4 named controller manager slot 9 for `fire_occupant`. The exe
  calls slot 10 (`+0x28`, at `0x00471A48`), the by-name variant; disasters.md is corrected in the same commit.
- **Q2.** The meaning of the two `vec3`s and the `bool` of a controller-manager automaton set (§3.3).
- **Q3.** The type numbers of the RandomWalk, PlopJump, Plummet, PullOver, Bumped and Collide states, and the
  meaning of corpus state types 0, 2, 3, 7 and `0x0D` (all use the default record).
- **Q4.** The records of aircraft and watercraft automata (helicopter, plane, boat). None occur in the corpus,
  and their Write and Read functions are not mapped here.
- **Q5.** The meaning of the generator's two trailing `u32`s, the automaton base's unnamed fields, and
  `CreateAutomaton`'s mode and last three arguments.
- **Q6.** Messages `0x89D3E9F3`, `0x06CDB65A`, `0xCAD61E00`, `0xEA298D50`, `0xEAD61E16`, `0xEBFF18A3`,
  `0xA6B79611`, `0xC9DA96E7` are handled but not named in other specs.

## 14. Conformance tests (must exist in `engine/conformance`)

Each test reads the record, checks the decoded fields, writes it back and compares bytes.

1. **Script system.** `01 00` → version 1. A record `02 00` must fail to load.
2. **Controller manager, empty** (30 bytes, New City (11)):
   `01 00 | 18 93 01 0A 29 F9 E7 A7 | 00 00 00 00 00 00 | 00 00 00 00 00 00 | 00 00 00 00 | 00 00 00 00`
   → version 1; clock ref ID `0x0A019318`, class `0xA7E7F929`; times (0.0 h, day 0) twice; no controllers; no sets.
3. **Controller manager times** (Getting Started Tutorial): both times are bytes `77 77 5F 41 03 00` = (13.9667 h,
   day 3).
4. **Vehicle manager, empty** (22 bytes): `02 00 | 04 00 | 50 01 00 00 | 00 | 00 | 00 00 00 00 | 01 00 00 00 |
   01 00 00 00` → version 2; base version 4, max 336, inactive, flag 0, no automata, no group map (version < 5);
   `u32` 1; cursor 1. The **pedestrian** record is the same with max `80 01 00 00` (384) and `u32` 2.
5. **Attractor** (82 bytes, Konradshohe): `05 00 | 2C 21 89 17 2D 88 BD A9 | 0E 00 00 00 "police_attract" |
   12 × 00 (position) | 8 × 00 | 00 00 00 00 | 07 00 00 00 | 00 00 00 00 | 01 00 | 02 00 00 00 | 00 00 00 00
   00 00 70 42 | 00 00 48 42 00 00 70 42` → base version 5, building `0x1789212C`, flags 7, no automata; curve
   ((0, 60), (50, 60)).
6. **Generator** (76 bytes, Tegel): `04 00 | 05 00 | 54 6E BD 09 47 AA 77 29 | 08 00 00 00 "dog_make" | 20 × 00 |
   00 00 00 00 | 87 00 00 00 | 00 00 00 00 | 00 00 20 41 00 00 | AB AA 2A 3E 00 00 | 22 41 00 00 | 00 00 00 00`
   → occupant ID `0x09BD6E54` (a prop, class `0x2977AA47`), flags `0x87`, times (10.0, 0) and (0.16667, 0), `u32`s 16674 and 0.
7. **Watercraft, version 6** (126 bytes, Konradshohe):
   `06 00 | 06 00 14 00 00 00 01 00 00 00 00 00 0C 00 00 00 05 13 00 00 00 00 00 00 06 13 00 00 00 00 00 00 07 13
   00 00 00 00 00 00 08 13 00 00 00 00 00 00 09 13 00 00 00 00 00 00 39 19 00 00 00 00 00 00 41 19 00 00 00 00
   00 00 10 31 01 00 00 00 00 00 20 31 01 00 01 00 00 00 30 31 01 00 02 00 00 00 0A 15 00 00 00 00 00 00 0F 15
   00 00 00 00 00 00 | 00 00 00 00 | 00 00 00 00 | 00 00 00 00`
   → base version 6, max 20, active, flag 0, no automata, 12 group counts: `0x1305`–`0x1309`, `0x1939`, `0x1941`,
   `0x13110` with 0, `0x13120` with 1, `0x13130` with 2, `0x150A` and `0x150F` with 0; then three empty lists.
8. **Riot order.** With a fake pedestrian manager and controller manager, `AddRiotLeader` must call, in order,
   `CreateAutomaton(0x4107, …)`, `SetState(8, …)`, `SetVisible(true)`; a stage rise to 2 must call
   `AddControllerFromTemplateName(leader, "riot_leader_2", 0)`; the end must call `SetState(1, …)` before
   `RemoveControllers(leader)`.
9. **Fire.** `SetTarget` on a building calls controller manager slot 10 with `"fire_occupant"`; on a non-building
   it calls nothing; `Shutdown` calls slot 14.
10. **Flag 8.** A controller manager holding one controller with flag 8 and one without writes n1 = 1, m = 1, and
    an automaton whose controller has flag 8 writes a null controller ref.
11. **Unknown template.** Loading a controller whose template name the scripts do not define drops it at
    `PostCityInit` and keeps the rest.
12. **Corpus round trip.** Every component 40, 41, 43–46 record and every referenced object record in the corpus
    parses to its exact end.
