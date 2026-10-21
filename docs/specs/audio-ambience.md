# Audio ambience: the ambience manager (behaviour spec)

Status: behaviour spec, written 2026-10-21 (re-audio4 lane). Every fact was checked on the Windows executable
`bin/SimCity4.exe` (1.1.641, image base `0x00400000`); every address is a Windows address. Numeric details
were checked against the instructions (`tools/re_query.py asm`). The REA decompiler session was unavailable
for this lane ("no app open"), so the evidence is the Ghidra export and its instruction listings only.
Implementers work from this spec, the public gzcom-dll SDK headers and general knowledge only; they must
**not** consult decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Tags as in [audio.md](audio.md): *[compat]* observable, *[choice]* FreeCity4 decision, *[inferred]*
consistent with the evidence but not proven, *[quirk]* reproduced oddity, *[unsettled]* open (§9).

Scope. **`cSC4AudioAmbienceManager`** (CLSID `0x6A12F51C`, factory `0x004EB0C0`, service
`kGZGUID_AudioAmbienceManagerSystemServiceId` `0xAA12F911`; [audio.md](audio.md) §1.2, §1.3): the city-side
service that turns occupants into positional sounds, runs the occupant alarm clock, keeps the listener's
"attention rectangle", and drives the ambience layers once per tick. The layers themselves
(`cSC4AudioAmbienceLayer`) are specified here as the interface the manager calls (§8) plus their `Init`,
tuning and filter creation (§8.1–8.3); their score and loop algorithms (`_UpdateAmbienceScores` `0x004E5920`,
`_UpdateAmbienceLoops` `0x004E5D50`) are left to a later section (§9, **AA1**). The sound-system events the
manager posts are the ones `cSC4AudioSoundManager::DoEvent` handles ([audio.md](audio.md) §8.3).

---

## 1. Object

The object is `0x120` bytes. Four interface vtables at `+0x00` (main: `IsServiceRunning`, `Init`,
`Shutdown`, `OnTick`, `DoEvent`), `+0x04` (the `cIGZMessageTarget2` the message server delivers to;
`DoMessage` `0x004EAF20` is reached through it), `+0x08` and `+0x0C` (the sound-system event target
registered with `cIGZSndSys`). Constructor `0x004EA390`, destructor `0x004EA670`.

| Offset | Field | Initial | Meaning |
|---|---|---|---|
| `+0x10` | ref count | 0 | |
| `+0x14` | service id | `0xAA12F911` | |
| `+0x18` / `+0x19` | bytes | 0 / 1 | service-running flags *[inferred]* |
| `+0x1C` | audio scape manager | null | service `0x49E50BA1`, IID `0x29E39DB4` (`Init`) |
| `+0x20` | `cIGZSndSys*` | null | service `0x23CECBA0`, IID `0x106077D2`; every sound request goes here |
| `+0x24` | occupant manager | null | copied from the global `0x00B43D0C` at city init |
| `+0x28` | occupant filter | null | an 8-byte filter (vtable `0x00A93640`) made at city init; passed to the per-cell occupant walk |
| `+0x2C` | layer list | empty | intrusive circular list of ambience layers (node: next, prev, layer) |
| `+0x30` | zoom index | 0 | copied from the view-utilities global `0x00B43DD8` `+0x0C` |
| `+0x34..+0x3C` | listener position x, y, z | | f32, from the audio director's listener (§4) |
| `+0x40..+0x48` | listener vector | | f32, the listener's second vector (direction *[inferred]*) |
| `+0x4C` | alarm list | empty | circular list of alarm entries (§3) |
| `+0x50` | ticking enabled | 1 | byte; `OnTick` does nothing while 0 |
| `+0x54` | attention margins | null | f32[5], indexed by zoom (§4) |
| `+0x60..+0x68` | layer ids | empty | `std::vector<u32>`: the ambience-layer exemplar instances (§2.2) |
| `+0x6C..+0x78` | current cell rectangle | 0 | x0, z0, x1, z1 (even values, §4) |
| `+0x7C..+0x88` | previous cell rectangle | 0 | same layout |
| `+0x8C..+0xA8` | world rectangles | | the attention rectangle and the city bounds in world units *[inferred]* |
| `+0xAC` | city ready | 0 | set to 1 at city init |
| `+0xB0` | stage | 0 | `OnTick` state machine (§5) |
| `+0xB4..` | random iterator | | `RandomIterator` over 2×2-cell blocks (`+0xB4..+0xB6` bytes 1,1,1) |
| `+0xE8` | stage timer | | `cRZTimer`, unit 3 (microseconds) |
| `+0x100` | tick timer | | `cRZTimer`, unit 4 (milliseconds) |
| `+0x118` | tracked-occupant set | null | GZCOM object, CLSID `0x4A401BA7`, IID `0x4A40074C` (`Init`) |
| `+0x11C` | tracked count | 0 | s32; +1/−1 as occupants enter/leave the set |

## 2. Lifetime

### 2.1 `Init` (`0x004E84D0`)

In order; any failure returns 0 *[compat]*:

1. Resolve the framework (cached in the global `0x00B628AC`) and call its slot 3.
2. Get the audio scape manager service (`0x49E50BA1` / `0x29E39DB4`) into `+0x1C`. Absent → return 0.
3. Get `cIGZSndSys` (`0x23CECBA0` / `0x106077D2`) into `+0x20`. Absent → return 0. Register the `+0x0C`
   event target with it (sound-system slot `+0x3C`).
4. Get the message server (`0x4FA845B` / `0x652294C7`) and subscribe the `+0x04` target to these 12 types,
   in this order: `0x26D31EC1` PostCityInit, `0x26D31EC2` PreCityShutdown, `0x8A4A69C7` ListenerPosChanged,
   `0x99EF1142` InsertOccupant, `0x99EF1146` MoveOccupant, `0x99EF1143` RemoveOccupant, `0xE99B3D4D`
   ConditionChanged, `0xE8BDA2E7` StateChanged, `0x09D92FC3` AutomataEvent, `0xC53D10AA` (exemplar changed),
   `0x8A878385` AutomatonApproachingIntersection, `0x4348B111` AppGainLoseFocus. Unlike the sound manager
   ([audio.md](audio.md) §8.3), this service **does** subscribe `0xC53D10AA` *[compat]*.
5. Create the tracked-occupant set (GZCOM `0x4A401BA7` / `0x4A40074C`) into `+0x118`; on failure return 0.
6. Start the tick timer `+0x100`, set `+0x50 = 1`, return 1.

### 2.2 City init (`0x26D31EC1`, `_HandleInitMessage` `0x004EABF0`)

1. Read the tuning (§4.1).
2. Create one ambience layer per id in `+0x60` (`_CreateAmbienceLayersFromExemplarIds` `0x004E9C10`): allocate
   a `0x25C`-byte `cSC4AudioAmbienceLayer`, call its `Init` (slot 4). On failure log
   `"Error : cSC4AudioAmbienceManager::_CreateAmbienceLayers : Failed to init layer (%08x)\n"` and drop it.
   On success set its score master volume (slot 17) from the audio preferences (the object behind
   `0x00B43C94` slot `+0x98`): 0 when the preference byte `+4` is clear, the preference volume otherwise; then
   append it to `+0x2C`. With the shipped tuning that is **94 layers** (audio.md §10.8).
3. Copy the occupant manager into `+0x24`. Ask the city (`0x00B43CEC`) for its cell counts (slots `+0x174`,
   `+0x178`) and convert cell (0, 0) and (n−1, n−1) to world corners through occupant-manager slot `+0x48`
   (stored at `+0x9C..+0xA8`). Set `+0xAC = 1`.
4. Recompute the attention rectangle (§4.2).
5. Make the occupant filter `+0x28`, reset the random iterator, set stage `+0xB0 = 0`.
6. Add itself to the framework's tick list (framework slot `+0x24`).
7. Walk the existing occupants (§3.1, `_IterateOccupantsThatNeedToBeTracked` `0x004EAAE0`).
8. `+0x50 = 1`; return 1.

### 2.3 City shutdown (`0x26D31EC2`, `0x004E9810`)

`+0x50 = 0`, remove itself from the framework's tick list (slot `+0x28`), then release every layer and empty
the lists (`0x004E9660`). Returns that result.

### 2.4 `Shutdown` (`0x004E96A0`)

Unregister the event target from `cIGZSndSys` (slot `+0x40`), unsubscribe the 12 message types of §2.1 (the
order differs, the set is the same), release the layers (`0x004E9660`) and the held services.

## 3. Occupant sounds and alarms

Every occupant handler first skips **preview occupants**: an occupant whose property holder has `0x029244DB`
set to true is ignored (foundations §5.2) *[compat]*. The occupant's property holder is its slot `+0x18`.

Sound requests go to `cIGZSndSys` slot `+0x2C` as sound-system events ([audio.md](audio.md) §8.3):
`0xEA451903` start, `0x6A451908` stop, `0xCA45190F` set position. The **instance id** of an occupant sound is
the occupant's address plus the sound id *[compat for uniqueness; the value itself is internal]*. Positions
are the occupant's position (slot `+0x20`) truncated to integers.

| Property | Meaning to this manager |
|---|---|
| `0x6A36AFAC` | ambient sound id: a looping sound started at insert, moved on move, stopped on remove |
| `0xC9B93A56` | a second sound id started at insert (non-looping flag) |
| `0x6A36AFAB` | alarm time list (f32 hours); with `0x0A36AFA2` → one-shot alarm, with `0x2A53BE13` → on/off alarm |
| `0x0A36AFA2` | one-shot alarm sound id |
| `0x2A53BE13` | on/off alarm sound id |
| `0xCA19D7CA`, `0x0A4C13CB`, `0x4A4C132E` | condition-dependent sounds: any of them triggers the lot condition update (§3.4) |
| `0xAA87817A` | automaton crossing sound id (§3.5) |

### 3.1 Insert (`0x99EF1142`, `HandleInsertOccupant` `0x004EA7D0`) and init walk (`HandleInitOccupant` `0x004EA970`)

1. Add the occupant to the tracked set `+0x118` (its slot `+0x14`); if it was added, `+0x11C += 1`.
2. Insert only: if it has `0x6A36AFAC`, start that sound looping (`_AddOccupantSndInstance` `0x004E8AD0`).
3. If it has `0x6A36AFAB`: register a one-shot alarm when `0x0A36AFA2` is present and an on/off alarm when
   `0x2A53BE13` is present (`_AddAlarmOccupant` `0x004EA690`; both may be registered).
4. Insert only: if it has `0xC9B93A56`, start that sound once.
5. If it has any of `0xCA19D7CA`, `0x0A4C13CB`, `0x4A4C132E`: find its lot (lot manager `0x00B43D08` slot
   `+0x38`) and run the condition update for that lot (§3.4).

The init walk (city init step 7) visits every occupant of types `0x278128A0` (buildings), `0xA823821E` and
`0xC7CC75F1` (occupant-manager slot `+0xB0` with a type filter) and applies the init variant, which starts
**no** sound itself *[compat]*.

`_AddOccupantSndInstance` also links an automaton occupant (IID `0xA9B40F05`) back to this manager (its
slot `+0x9C`); the start event carries the instance id, the loop flag and the position, and is followed by a
set-position event.

### 3.2 Move (`0x99EF1146`, `HandleMoveOccupant` `0x004E8D40`)

Non-preview occupant with `0x6A36AFAC`: post set-position for its sound. If it has `0xCA19D7CA`: also post
set-position for the condition sound (`0x004E8C70`).

### 3.3 Remove (`0x99EF1143`, `HandleRemoveOccupant` `0x004EA270`)

Remove from the tracked set (`+0x11C −= 1` when it was present). Then for each of `0x0A4C13CB`,
`0xCA19D7CA`, `0x6A36AFAC` present: stop the sound (`_RemoveOccupantSndInstance` `0x004E8BF0`, which also
unlinks an automaton through its slot `+0xA0`); for each of `0x0A36AFA2`, `0x2A53BE13` present: remove the
alarm entry (`_RemoveAlarmOccupant` `0x004EA080`). The stop is posted even when nothing was started
*[compat]*; the sound manager ignores unknown instance ids (audio.md §8.3).

### 3.4 Condition and state changes

`0xE99B3D4D` ConditionChanged: if the message's object (void1) answers the lot IID `0xE818FCED`, run
`_UpdateLotOccupantsForConditionChange` (`0x004E9DA0`) for that lot with `old := data1` (message slot `+0x24`,
reads `+0x10`) and `new := data2` (slot `+0x38`, reads `+0x14`) (`0x004EAEC5`–`0x004EAEDE`). The poster is
`cSC4Lot::SetCondition` (`0x006C26D0`): void1 is a **copy** of the lot, data1 the previous condition byte,
data2 the new one, data3 the lot's `+0x3C` (unused here); it is posted only when the condition changes.
`0xE8BDA2E7` StateChanged is subscribed and routed to the occupant handler but has no case there: it does
nothing *[quirk]*.

**The per-lot update `U(lot, old, new)`** (`0x004E9DA0`, pop 12). Insert and the init walk (§3.1 step 5)
call it as `U(lot, −1, −1)`.

1. `lot` null → return. If `new < 0` (signed), `new := lot.GetCondition()` (lot slot `+0x8C`, byte
   zero-extended) (`0x004E9DBB`).
2. `L := lot.GetLotOccupants()` (lot slot `+0xC4`, `0x006BFF70`: the occupant manager's occupants inside the
   lot's cell rectangle, any type, through an inside-lot filter) into a fresh list.
3. For each occupant `o` of `L`, in list order: `o` qualifies when it answers `cISC4BuildingOccupant`
   (`0x87DDA3A9`) **or**, failing that, `cISC4PropOccupant` (`0xE9793A65`), and its property holder (slot
   `+0x18`) is non-null. Both interface references are released after the occupant (they gate only). No
   preview check is made here *[compat]*. For a qualifying `o`, with `A := 0x0A4C13CB`, `C := 0xCA19D7CA`,
   `R := 0x4A4C132E` (each read as u32, `0x005FD480`; "absent" means the read fails), apply in this order:

| `new` | step (only for properties present) |
|---|---|
| 0, 5 or 6 | stop `A`; start `C` looping |
| 2 or 3 | stop `C`; start `A` looping |
| 1 or 4 | stop `A`; stop `C` |
| 0, and `old` = 1 | additionally start `R` looping (after the row above) |
| any other value (7 and up) | nothing |

   "Start" is `_AddOccupantSndInstance(o, id, loop = 1)` (start event, set-position, automaton link, §3.1)
   and "stop" is `_RemoveOccupantSndInstance(o, id)` (§3.3); the instance id is `o` + the property value. No
   test is made whether the sound is already running: a repeated condition 0 re-posts the `C` start
   *[compat]*.
4. Release the list through the occupant manager (`0x00B43D0C` slot `+0xC0`, `ReleaseOccupantList`), then
   free its nodes and head.

Consequences *[compat]*: `R` (the "repowered" sound, by its trigger) starts only on a real condition change 1 →
0, never on insert or init (old = −1); it is never stopped by this function, only by `HandleRemoveOccupant`
— which stops `A`, `C` and `0x6A36AFAC` but **not** `R` (§3.3) *[quirk]*. The lot condition values are those
of lot-developer.md §5.5 (0 normal, 1 unpowered, 2–3 run down, abandoned lots step up to 4); 5 and 6 share
row 0 *[compat; no producer of 5 or 6 is specified]*.

Worked example: a lot with one building carrying `C = 0x1111` and `A = 0x2222`. Insert (lot condition 0):
start `0x1111` looping, instance `o + 0x1111`. Power is lost (0 → 1): stop `o + 0x2222` (not running; the
sound manager ignores it), stop `o + 0x1111`. Run down (1 → 2): stop `o + 0x1111`, start `o + 0x2222`.

### 3.5 Automata

`0x09D92FC3` (`_HandleAutomatonMessage` `0x004E8820`) reads the automaton event code (message slot `+0x38`):

- code **9**: if the automaton's occupant has `0x6A36AFAC`, post set-position for its sound;
- codes **12** and **13**: if the automaton's kind (message slot `+0x4C`) is `0x10`, `0x14` or `0x1E`, play
  the occupant's `0x6A36AFAC` sound (`_PlayOccupantSound` `0x004E8720`): **start** (non-looping, plus a
  set-position) when the code is 13 and, for kind `0x10`, the automaton's slot `+0xE4(0x10)` is 0; otherwise
  **stop** it. Other codes: nothing.

`0x8A878385` (`_HandleRailwayCrossingMessage` `0x004E8990`): the message object must answer the automaton
IID `0xA9B40F05` and its occupant must have `0xAA87817A`; then start that sound once at the automaton's
position and post a set-position.

### 3.6 The alarm clock (`_UpdateAlarms` `0x004E8EA0`, every tick)

The clock is the simulation clock global `0x00B43DBC` (absent → no alarms). Time of day
`t = float(GetHour()) + float(GetMinute() / 60)`, where the division is an **unsigned integer** division
(`0x004E8EDB`): with minutes 0–59 the fraction is always 0, so alarms fire on whole hours only *[quirk]*.

Each alarm entry holds the occupant, a sound id, the last time seen (`+0x10`, f32), the time list
(`+0x14` count, `+0x18` data, f32 hours), an "on" byte `+0x24` and the kind byte `+0x25` (0 one-shot, 1 on/off).
For each time `a[i]` (with `next` = `a[i+1]`, or `a[0]` for the last entry):

- **one-shot**: if `last < a[i] <= t`, start the sound once with a fresh instance id
  (`cSC4AudioSoundManager::GenerateNewInstanceId`, audio.md §8.4) through the sound system's extended post
  (slot `+0x34`, tag `0x5B523BA4`), then post set-position the same way;
- **on/off**: the times alternate on, off, on, … (even index = on). If `t` lies in `[a[i], next)` — or the
  interval wraps midnight (`next < a[i]` and `t >= a[i]`, or `t − 24 >= a[i]` and `t − 24 < next`) — read
  `0x2A53BE13` and: on an "on" index start the looping sound if `+0x24` is 0 and set it; on an "off" index
  stop it if `+0x24` is set and clear it.

Then `last := t`, a raw 32-bit copy of the f32 (`0x004E91B0`; no integer conversion) (stack trace: `t` is stored at `0x004E8F12` at depth 0x4C, i.e. frame slot
entry − 0x34; after `PUSH EBX` at `0x004E8F20` that slot is `[ESP+0x1C]`, which `0x004E91AC` reads; the
loop's `[ESP+0x18]` is the different slot entry − 0x38 holding `a[i]`). Because the "fire" tests are `last < a[i] <= t`, an alarm at time 0 never fires
after midnight (`last` = 23, `t` = 0) *[quirk]*.

## 4. Tuning and the attention rectangle

### 4.1 `_ReadTuningValues` (`0x004E91D0`; also on `0xC53D10AA`)

From the audio settings exemplar (`0x6534284A` / `0xCA25875D` / `0x6A359C5C`, audio.md §10.8): `0x0A36F410`
into the margins `+0x54` — when missing, all five margins are **70.0** (`0x428C0000`) *[compat]* — and
`0x2A3FF8CD` into the layer-id vector `+0x60`. Shipped margins: 0, 0, 188, 150, 73.

### 4.2 `_UpdateAttentionRect` (`0x004E92A0`; city init and `0x8A4A69C7`)

1. Copy the listener's position and vector from the audio director (`GetSC4AudioCOMDirector` `+0x64`
   object, words 9–11 and 3–5) into `+0x34` and `+0x40`.
2. `zoom := *(0x00B43DD8 + 0x0C)`, stored in `+0x30`; `m := margins[zoom]` (no bounds check).
3. World rectangle `(x − m, z − m) .. (x + m, z + m)`; both corners to cells through occupant-manager slot
   `+0x4C`.
4. Clamp each cell coordinate to `[0, n − 1]` (the city's cell counts), then clear bit 0 (round down to even).
5. The first call after start-up (global byte `0x00B09824` set) writes the result to both rectangles and
   clears the byte; later calls move current → previous and store the new current.

## 5. `OnTick` (`0x004EA150`)

Does nothing while `+0x50` is 0. Otherwise `e :=` tick-timer elapsed (ms), restart it, run the alarm clock
(§3.6), then:

| Stage | Action | Next |
|---|---|---|
| 0, 1 | Seed the random iterator with the current rectangle halved (2×2-cell blocks); reset it; call each layer's `BeginUpdate` (slot 25) with the attention rectangle | 2, and fall through |
| 2 | `_DoStageUpdate(e)` (below); when it reports the walk finished | 3, and fall through |
| 3 | Each layer's `EndUpdate` (slot 27) | 1 |

`_DoStageUpdate` (`0x004E94E0`): restart the stage timer (µs). While its elapsed time is **at most 999 µs**
take the next random block `(bx, bz)`; walk the occupants of cells `2bx..2bx+1 × 2bz..2bz+1` through
occupant-manager slot `+0xBC` with the filter `+0x28` (callback `0x004E8E60`, which feeds each occupant to
the layers), then call each layer's `HandleCell` (slot 20) for the four cells in the order
`(2bx, 2bz)`, `(2bx+1, 2bz)`, `(2bx, 2bz+1)`, `(2bx+1, 2bz+1)`. When the iterator is exhausted, report
finished. In every case end with each layer's `Update(e, &listenerPos, &listenerVector)` (slot 26).
The time budget makes the walk length machine-dependent; a replacement may use any budget *[choice]* but
must visit every block of the rectangle before `EndUpdate` *[compat]*.

## 6. Events and focus

`DoEvent` (`0x004E9870`), the sound-system event target:

- `0x2A5B69C0` (master volume): every layer's `SetScoreMasterVolume` (slot 17) with the event's value.
- Any other event: the event's data object (slot `+0x18`, then `+0x80`) must answer the freshness-player IID
  `0xDC1CF595` (`cFreshPlayer`); then

| Event | Freshness-player call |
|---|---|
| `0x4A2C0389` | `Stop` (slot 10), then `Shutdown` (slot 7) |
| `0x0A414C5C` | `SetPlayPercent(value)` (slot 16) |
| `0x2A2C0384` | load the score named by the payload's type/group/instance (payload IID `0x65297976`); on failure log `"Error: cSC4AudioAmbienceLayer::DoEvent : Failed to load freshness score (%08x:%08x:%08x)\n"` |
| `0x2A2C0393` | `FadeVolume(3000, 0, 1)` (slot 12): a 3-second fade to 0 |
| `0x4A3DA208` | `SetVolume(value)` (slot 11) |
| `0xEA2C0390` | `SetHeadRelative(value != 0)` (slot 28), then `Play` (slot 8) |
| `0xEA2D827C` | `SetMinMaxDistance(...)` (slot 29) with the payload's two values |

Returns 1 when handled, 0 otherwise.

`0x4348B111` AppGainLoseFocus (`0x004E8CF0`): every layer's `Pause` (slot 28) with the message's flag.

## 7. Message dispatch (`DoMessage` `0x004EAF20`)

The message must answer `0x65297976`; otherwise return 0.

| Type | Handler | Returns |
|---|---|---|
| `0x26D31EC1` | §2.2 | its result |
| `0x26D31EC2` | §2.3 | its result |
| `0x4348B111` | §6 | its result |
| `0x8A4A69C7` | §4.2 | 1 |
| `0xC53D10AA` | §4.1 | 1 |
| `0x09D92FC3` | §3.5 | 1 |
| `0x8A878385` | §3.5 | 1 |
| `0x99EF1142`, `0x99EF1143`, `0x99EF1146`, `0xE99B3D4D`, `0xE8BDA2E7` | §3 (`_HandleOccupantMessages` `0x004EAD30`; the occupant is QI'd with `0x26BB767E`) | 1 |
| other | — | 1 |

## 8. The ambience-layer interface (as called by the manager)

`cSC4AudioAmbienceLayer`, vtable `0x00A934A8`, IID `0x4A2835FE`; slots: 3 `DoMessage`, 4 `Init`,
5 `Shutdown`, 6/7 `GetName`/`SetName`, 8/9 `SetLayerId`/`GetLayerId`, 10 `GetAveragePosition`, 11 `Enable`,
12/13 `AddAmbienceLoop`/`RemoveAmbienceLoop`, 14 `LoadFreshnessScore`, 15/16 `Add`/`RemoveFreshnessScore`,
17 `SetScoreMasterVolume`, 18 `CanHandleOccupant`, 19 `CanHandle`, 20 `HandleCell`, 21 `HandleOccupant`,
22–24 `Insert`/`Move`/`RemoveOccupant`, 25 `BeginUpdate`, 26 `Update`, 27 `EndUpdate`, 28 `Pause`.
The manager uses 4, 17, 20, 25–28 directly and 21 through the cell walk.

### 8.1 Layer `Init` (`0x004E8180`)

Runs once per layer (byte `+0x04` set on the first call; later calls do nothing). A process-wide layer count
(`0x00B21944`) is incremented; the layer that takes it from 0 to 1 first runs `_StaticInit` (`0x004E4EB0`),
and if that fails `Init` returns its result without doing anything else. Then, in this order:
`_ReadTuningValues(layerId)` (layer id `+0x1F8`, §8.2; its result is ignored); subscribe this layer to
`0xC53D10AA` on the message server (`0x652294C7`); `_CreateFilters` (`0x004E78E0`); `_CreateLayers`
(`0x004E4F60`); own slot 14 `LoadFreshnessScore`; own slot 12 `AddAmbienceLoop(+0x1D0)` (the loop sound id
read from `0x6A3FBDDA`, §8.2).

### 8.2 Layer tuning (`_ReadTuningValues` `0x004E7A90`)

**Defaults, written first, before the exemplar is looked up** (so a missing exemplar leaves exactly these):
ease object `+0xE8` gets `+0x108 := 100 × 0.001 = 0.1` and `+0x10C := 4000 × 0.001 = 4.0` (setter
`0x004F7BF0`: each argument times the f32 constant 0.001 at `0x00A867A4`); name `+0x188 :=
"Noname_0x%08x"` of the layer id; `+0x1E1 := 0x1F`, `+0x1E0 := 0x1F`, `+0x1EC := 1024`, `+0x1E8 := 1024`,
`+0x1F0 := 100`, `+0x34 := 1`, `+0x38 := 5`, `+0x20 := 50.0`, `+0x24 := 50.0`, `+0x200 := 1.0`; `+0x204 :=` the
u32 at `+0x314` of the object at `[0x00B43CC4] + 0x18`; three response curves with two points each:

| Curve | Default points (x, y) | Tuning key |
|---|---|---|
| `+0x1A0` | (0, 100), (20, 100) | `0xCA4132E9` |
| `+0x1B0` | (0, 0), (20, 1024) | `0x6A4132F3` |
| `+0x1C0` | (0, 1024), (20, 1024) | `0x6A4132F6` |

**Exemplar**: type `0x6534284A`, group `0xCA25875D`, instance = the layer id, through the resource manager
(`0x00B21950` slot 3, IID `0xA52160F5`). Missing → log "Failed to get resource", return 0 (defaults stay).
Present → read, in this order (each reader leaves the destination unchanged when the property is absent, except
where "cleared" says the destination is zeroed just before the read):

| Key | Reader | Destination | Absent value |
|---|---|---|---|
| `0x00000020` | string (`0x005FD570`) | name `+0x188` | `Noname_0x…` |
| `0xEA3FBDD7` | vec<u32> (`0x005FF510`) | vector `+0x1D4` (emptied first) | empty |
| `0x2A3FBDE1` | vec<u32> | vector `+0x208` (emptied first) | empty |
| `0x2A3FBDE5` | vec<u32> | vector `+0x224` (emptied first) | empty |
| `0x0A410BA0` | vec<u32> | vector `+0x240` (emptied first) | empty |
| `0x6A3FBDDA` | u32 (`0x005FD480`) | `+0x1D0`, the layer's ambience-loop sound id (§8.1) | 0 (cleared) |
| `0x0A25896B` | u8 (`0x005FD3C0`) | `+0x1E0` | 0 (cleared; the `0x1F` default is lost) |
| `0xAA3FBDDE` | u8 | `+0x1E1` | 0 (cleared) |
| `0xEA3FC42A` | u32 | `+0x1EC` | 1024 |
| `0xAA3FC48E` | u32 | `+0x1E8` | 1024 |
| `0x6A3FC4C7` | u32 | `+0x1F0` | 100 |
| `0x8A552758` | u32 | `+0x34` | 1 |
| `0x8A70B1A2` | u32 | `+0x38` | 5 |
| `0x8A3FC5DD` | f32 (`0x005FD4F0`) | `+0x200` | 1.0 |
| `0xCA4132E9`, `0x6A4132F3`, `0x6A4132F6` | response curve (`0x005F4700`) | curves `+0x1A0`, `+0x1B0`, `+0x1C0` | the defaults above |
| `0x0A4168E1` | f32 | `+0x20` | 50.0 |
| `0x0A56212A` | f32 | `+0x24`, which is first set to the new `+0x20` | `+0x20` |
| `0x8AA039B7`, `0x4AA03DB1` | u32 into locals (defaults 100, 4000) | ease `+0x108`, `+0x10C` := float(u32) × 0.001 | 0.1, 4.0 |

The two ease values are converted **unsigned** (`FILD` plus 2^32 when negative, `0x004E8105`–`0x004E8130`);
the call is the same setter as the default, so a missing `0x8AA039B7` still yields 0.1 s. The function returns 1
on success. The roles of `+0x20`/`+0x24`, `+0x34`/`+0x38`, `+0x1E0`–`+0x1F0`, `+0x200`, `+0x204` and the
four vectors belong to the scoring and loop code (still AA1, §9).

Worked example: an exemplar with only `0x6A3FBDDA = 0x1234` and `0x0A4168E1 = 30.0` gives loop sound
`0x1234`, `+0x20 = +0x24 = 30.0`, `+0x1E0 = +0x1E1 = 0`, every other value its default.

### 8.3 Layer filters (`_CreateFilters` `0x004E78E0`, `_AddFilter` `0x004E7010`)

The three tuning id lists are **filter class ids** of three kinds: kind 1 from `0x2A3FBDE1` (`+0x208`), kind 2
from `0x2A3FBDE5` (`+0x224`), kind 3 from `0x0A410BA0` (`+0x240`). `_CreateFilters` first resets three bit
vectors (`+0x144`, `+0x158`, `+0x16C`), then for each kind in the order 1, 2, 3 sizes that kind's bit vector to
its id count (all false) and calls `_AddFilter(id, kind)` for each id in list order.

`_AddFilter(id, kind)` (pop 8):

1. Look `id` up in a **process-wide** filter cache (a hash map class id → filter, buckets `0x00B2195C`–
   `0x00B21960`, index `id mod bucket count`), shared by every layer. Found → use that filter.
2. Not found → create it through GZCOM `CreateInstance(id, IID 0x4A40074C)`. Failure → log "Failed to create
   class object filter", return (nothing counted). Then call the filter's `Init` (slot 3, `+0x0C`); false → log
   "Failed to init filter", release it, return. Success → the cache takes one reference and stores it.
3. Count the filter's interfaces on the layer: answers `0xCA400757` → `+0x138 += 1`; `0xCA400753` → `+0x13C
   += 1`; `0xEA40074F` → `+0x140 += 1` (the query references are released at the end).
4. Append the filter pointer (no reference of its own; the cache owns it) to the kind's list and bump its count:
   kind 1 → list `+0x214`, count `+0x220`; kind 2 → `+0x230`, `+0x23C`; kind 3 → `+0x24C`, `+0x258`
   (`0x004E71E0`–`0x004E7247`). Another kind value appends nothing.

The same class id listed twice, or by two layers, yields the same filter object in each list *[compat]*.
Which interface or kind a filter must have for `CanHandle`/`HandleCell`/`HandleOccupant` to use it is still
AA1 (§9).

## 9. Open items

| # | Question | What would settle it |
|---|---|---|
| **AA1** | The layer's own behaviour: scores, loops and filter use (`CanHandle`, `HandleCell`, `HandleOccupant`, `Update`; `0x004E5110`–`0x004E6910`). Init, tuning and filter creation are §8.1–8.3. | a follow-up section of this spec |
| **AA4** | Types `0xA823821E`, `0xC7CC75F1` of the init walk are unnamed. | the occupant type table |

## 10. Conformance tests

1. With no `0x0A36F410` in the tuning, every margin is 70.0.
2. `Init` subscribes exactly the 12 types of §2.1, including `0xC53D10AA`.
3. Alarm clock: hour 7, minute 59 gives `t = 7.0`; a one-shot alarm at 7.5 fires when the hour turns 8.
4. An on/off alarm with times (22, 6) starts its loop at 22:00, keeps it through 03:00 (no event) and stops it
   at 06:00.
5. A preview occupant (`0x029244DB` true) produces no sound event on insert, move or remove.
6. `_DoStageUpdate` visits the four cells of a block in the order of §5.
