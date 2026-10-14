# Spec: the city situation manager (`cSC4CitySituationManager`)

Status: **behaviour spec, short form** (the component is mostly gameplay UI; §0 says what that means), clean-room
input. This document describes *behaviour*, not code. It was written by the analysis team from observing the
original program and its data files. Implementers must work from this spec, the public LGPL gzcom-dll SDK headers
and general knowledge only. They must **not** consult decompiled or disassembled code (see
[CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (streams §2, components §3.6, the `Init` order §3.7, messages §5,
random numbers §6), [advisors.md](advisors.md) (the advice messages §6.4 and §10, the CitySituations advisor 29,
city-situation advices `0x8BE3753B`), [modal-dialogs.md](modal-dialogs.md) (§4: the situation popup and
bankruptcy), [budget.md](budget.md) (§5.6 bankruptcy, `DepositFunds` §6), [aura.md](aura.md) (§8.2 transient
effects), [lua-host.md](lua-host.md) (script tables), [dispatch.md](dispatch.md) (the indicator view).

## 0. Summary for the harness and for planning

**This component does not end cities and does not watch the city's finances.** It runs the Rush Hour "U-Drive-It"
missions (Maxis calls them *city situations*): a mission is a city-situation advice of the CitySituations advisor
(advisor 29); the manager marks vehicles that can start a mission with a floating indicator, starts the mission the
player picks, follows it on the real-time animation tick, and on success or failure pays the mission's money,
adds its aura effect, counts the success and shows a result popup.

- **Bankruptcy is the budget's and the 3D view's business, not this one's.** At every month-end with funds ≤
  `MinAllowableFunds` (−100,000) the budget posts `0xAA5CF1E3`, and the 3D view runs `DoBudgetFailure`
  (`0x007E6E00`): a dialog when popups are on, then `cISC4App::RequestGoToRegionView(false)`, which closes the city
  without saving (budget.md §5.6, modal-dialogs.md §4.1). Run 69 (funds −103,047 in 2057-10) and runs 71–79 are this
  path. The only users of `0xAA5CF1E3` in the executable are the budget's post (`0x00692B6E`) and the 3D view's
  handler (`0x007F56FE`).
- **No other automatic city-ending path was found.** The callers of `RequestGoToRegionView` (`cISC4App` slot 12)
  outside the region screen are `DoBudgetFailure`, the 3D view's command handler (the player's "go to region"
  command, `0x007F2D92`) and the tutorial window's button handler (`0x00444373`, the player leaving a tutorial).
  This manager calls none of the `cISC4App` close, quit or region requests.
- **Its popup never opens by itself in unattended play.** The situation popup (window procedure `0x0052CAB0`,
  posted as `0xEBB67EF7` only by the manager itself) opens only after a mission was started by the player (the
  vehicle menu, the catalog, the MySim "U-Drive-It" entry), by the debug cheat `csitrigger` (§4.6), or by the Lua
  function `sc4game.sitmgr.activate_situation`, which no shipped script calls (the only occurrence is the stub
  declaration in the `4A5E8EF6/FFBF9CCF` script). With no mission active its animation tick does nothing.
- **Switches:** `popup_dialogs = 0` (`cISC4App` slot 23) does **not** suppress the situation popup, but nothing opens
  it unattended. The cheat `csitrigger` exists only when the debug functionality flag (`cISC4App` slot 21) is set.
  The tutorial state does not change it, except that two shipped tutorial cities save the indicators hidden (§7).

A FreeCity4 replacement is not planned; the original object can stay. What another FreeCity4 component must keep
for it: the city slot (component 52), the save record (§7), the advice messages it listens to, the budget's
`DepositFunds` and the aura simulator's `AddTransientEffect(x, z, radius, magnitude)` that it calls (§4.5), and the
dispatch manager's indicator view (slot 22). An implementation that only keeps the record and answers the
interface with "no situation" (§8, "minimal replacement") is about 200 lines; a full one is a gameplay feature of
several thousand lines (targets, pursuit, coverage areas, the player-drive camera) and is out of scope here.

## 1. Identity

| Class | CLSID | Interfaces (IID) | City component | City getter |
|---|---|---|---|---|
| city situation manager | `0x0BB14381` | `cISC4CitySituationManager` (`0x6BB1437A`) at +0; `cIGZUnknown` (`1`) and `cIGZSerializable` (`0xE4FDA3D4`) at **+0xC**; `cIGZMessageTarget2` (`0x452294AA`) at +4; `cIGZMessageTarget` (`0xE98B2F57`) at +8 | 52 (city version ≥ 11) | `cISC4City` slot 49 (`+0xC4`, `GetCitySituationManager`) |

`QueryInterface` (`0x005246B0`) answers only those five IIDs; any other returns false and leaves the out pointer
untouched. *[quirk]* `cIGZUnknown` returns the serializable sub-object (+0xC), not +0. A sub-object at +0x10 (a
window procedure, vtable `0x00A97E08`, `DoWinProcMessage` `0x0052CAB0`) is not reachable through `QueryInterface`.

The class is registered by the **automata** COM director (`cSC4AutomataCOMDirector`, `0x00514A00`), not the
simulation director. Factory `0x005148C0`: `GlobalHeapAlloc(0xB00)` (`0x005E55E0`), constructor `0x00529FD0`.
Vtables: primary `0x00A97E58` (40 slots, §8), message target 2 `0x00A97E44` (`DoMessage` `0x0052EB80`), message
target `0x00A97E34` (`DoMessage` `0x005DEB40`, the shared "do nothing"), serializable `0x00A97E1C` (Write
`0x00527040`, Read `0x00529010`, GetGZCLSID `0x005274E0`), window procedure `0x00A97E08`.

`AddRef` (`0x00524750`) increments the count at +0x18. `Release` (`0x00525520`): if the count is 1 or less, call
`Shutdown` and the deleting destructor (`0x00527AA0`, which frees with `GlobalHeapFree`) and return 0, without
decrementing first; otherwise decrement and return the new count.

## 2. State

| Offset | Field | Constructor | Saved |
|---|---|---|---|
| `+0x14` | `initialized` (u8) | 0 | no |
| `+0x18` | reference count | 0 | no |
| `+0x1C` | a date object (wall-clock default) | | no |
| `+0x24` | held Lua interpreter (`cISC4App` slot 47) | null | no |
| `+0x28` | generator: `cRZRandomSuper` member seeded with −1 (wall clock) | | no |
| `+0x9F0` | `indicatorsVisible` (u8; "show CSI") | **1** | **yes** |
| `+0x9F1` | `loadedFromSave` (u8) | 0 | no (set by `Read`) |
| `+0x9F4` | `activeSituation`: the city-situation advice being played, held | null | no |
| `+0x9F8` | the four automata managers (vehicle, aircraft, pedestrian, watercraft; city slots 43, 45, 44, 46), held | empty | no |
| `+0xA04` | `trackList`: list of `{advice, tracked automaton, indicator handle}` | empty | no |
| `+0xA08` | set of tracked automata (its element count at `+0xA18`) | empty | no |
| `+0xA20` | held lot manager | null | no |
| `+0xA24` | held MySim agent simulator (city slot 71, `+0x11C`) | null | no |
| `+0xA2C` | held dispatch indicator view (dispatch manager slot 22) | null | no |
| `+0xA34` | `successCounts`: hash map u32 situation GUID → u16 count (element count at `+0xA44`) | empty | **yes** |
| `+0xA48` | `status`: 1 success, 2 failure, 3 running, 4 idle / briefing, 5 could not start | **4** | no |
| `+0xA4C` | active automaton (the player's vehicle) | null | no |
| `+0xA50` | current target occupant | null | no |
| `+0xA5C`, `+0xA60` | target count, current target index | 0 | no |
| `+0xA88` | `endDelay` (float, seconds) | 0 | no |
| `+0xA8C` | elapsed mission time (float, seconds) | 0 | no |
| `+0xA90` | start countdown (float, milliseconds) | 0 | no |
| `+0xA94` | time limit (float, seconds; 0 = none) | 0 | no |
| `+0xA98` | `justActivated` (u8) | 0 | no |
| `+0xA99` | `thirtySecondWarningGiven` (u8) | 0 | no |
| `+0xADC` … `+0xAF4` | seven Lua function references | −2 (none) | no |
| `+0xAF8` | held window | null | no |

The process-wide **maximum number of indicators** ("max CSI") is a static byte (`0x00B21D34`), not a member: the
automata manager loads it from its tuning (§6), `SetMaxCSI` (slot 26) overwrites it, and `GetMaxCSI` (slot 27)
returns it.

## 3. Agents, messages and cadence

The manager registers **no simulator agent**. It runs on the real-time **animation tick**: `Init` registers the +4
sub-object with the animation tick manager service (`cISC4AnimationTickManager`, slot 5, priority `0xFF`); each
tick arrives as message `0x86AD10EE` whose data3 holds the elapsed time in milliseconds as a `float` bit pattern.
Mission timing is therefore wall-clock time and independent of the simulation speed (and of pause).

Subscriptions (message server, all on the +4 sub-object):

| Message | Handling |
|---|---|
| `0x99EF1142` occupant inserted | if the occupant has no preview property `0x029244DB` and is an automaton (`QueryInterface 0xA9B40F05`): note it as a candidate vehicle for waiting situations |
| `0x99EF1143` occupant removed | if a mission is active: a removed target or active vehicle while `status = 3` fails the mission (§4.4); an automaton is dropped from tracking and its situation looks for another vehicle |
| `0x8A09FA64` AdviceAdded | IGZUnknown is a city-situation advice (`QueryInterface 0x6BEB1E8A`): **situation triggered** (§4.2) |
| `0x8A09FA65` AdviceRemoved | the same test: **situation untriggered** (§4.2) |
| `0x8A09FA67` AdviceRemovedAll | ignored (returns true) *[quirk]* |
| `0x6BB350E4` | posted by its own popup's close button after a mission ended: leave the player-drive mode and `EndSituation` if the message's advice is the active one |
| `0xEBB67EF7` show situation popup | IGZUnknown = advice, void1 = vehicle (optional): call the 3D view's slot 57 (`+0xE4`; the view pointer cached at `0x00B43DC4`), end an open MySim dialog, then show the popup (§4.3) |
| `0xCC0D905F` | if the active advice has situation flag 8: remember IGZUnknown as a window |
| `0xAC1045EF` initiate by GUID | `InitiateSituationByGuid(data1, data2 ≠ 0, data3)` (slot 21) |
| `0x26D31EC1` PostCityInit | if `loadedFromSave`: for each advice of advisor 29 that is a city-situation advice, **situation triggered** (§4.2) |

Delivered without a subscription: `0x86AD10EE` (animation tick, §4.4), `0x09D92FC3` (automaton notifications from
the tracked vehicles: subtype 9 "moved" re-checks success; 2 "removed" of the active vehicle fails a running
mission; 0xB, and 0xC with data3 = 0x14, "disabled"), `0x0BB324BD` (posted by the player-drive control when the
player leaves the vehicle: `EndSituation` unless the status is 1 or 2), `0x230E27AC` (the cheat code manager, §4.6),
and `0x2C16BDE7` (shows the popup with no advice; **no code in the executable posts it**, which answers
modal-dialogs.md §7 question 3 as far as static analysis can).

Messages it sends or posts:

| Message | When | Fields |
|---|---|---|
| `0xEBB67EF7` (posted to itself) | a mission was initiated with a briefing, could not start, or has ended (§4.3, §4.4) | IGZUnknown = advice, void1 = vehicle or 0 |
| `0xABBBCA64` (sent) | a mission becomes active (data1 = 1) or is deactivated (data1 = 0) | IGZUnknown = advice |
| `0xAC083D7D` (sent) | a target in a target sequence is reached | IGZUnknown = advice, data1 = target index |
| `0x6BB350E4` (posted to itself) | the popup's close button after the end of a mission | IGZUnknown = advice |
| `0xAC1045EF` (posted) | by the cheat and the Lua function (§4.6) | data1 = GUID, data2 = show briefing, data3 = 0 |

## 4. Behaviour

### 4.1 `Init()` (`0x0052DC40`) and `Shutdown()` (`0x00527AC0`)

`Init`, if `initialized` is clear:

1. If the debug functionality flag is set and a cheat code manager exists: register +4 as a cheat target and the
   cheat `csitrigger` (ID `0x8C4136DB`).
2. Subscribe the ten messages of §3; hold the dispatch manager's indicator view (dispatch manager slot 22, `+0x58`);
   register with the animation tick manager.
3. Hold the four automata managers (city slots 43, 45, 44 and 46, in that order) in the vector at `+0x9F8`.
4. Hold the lot manager, the MySim agent simulator and the Lua interpreter.
5. Create the Lua table `sitmgr` with `get_success_count`, `activate_situation`, `get_current_target`,
   `get_active_auto`, `get_target_name`, `show_CSI`, `num_targets_remaining` (functions) and `num_targets`,
   `cur_target_index` (variables bound to `+0xA5C` and `+0xA60`).
6. `HandleTuningParametersChanged` (slot 31): `SetMaxCSI(GetMaxCSI())`.
7. `indicatorView.ShowAllIndicators(4, indicatorsVisible)`.
8. **Rewards from the profile.** For each non-zero GUID at the start of the user preferences' 64-entry reward list
   (preferences `+0x1334`, `cISC4App` slot 38), until the first zero: `successCounts[guid] := 1`. *[quirk]* This
   **overwrites** a loaded count with 1.
9. Release the held window; set `initialized`; return true.

`Shutdown`, if `initialized`: release the held window, MySim simulator, Lua interpreter, lot manager and effect handles;
free the coverage cells; release the four automata managers; unregister from the animation tick manager; release the
indicator view; unsubscribe the ten messages; `DeactivateSituation` (slot 38); clear the Lua references and the track
list; remove the `sitmgr` table; unregister the cheat (when the debug flag is set); clear `initialized`. Return true.
`successCounts` and `indicatorsVisible` are kept.

### 4.2 Triggered situations and indicators

A mission becomes available when the CitySituations advisor activates its advice (the advice's Lua trigger,
advisors.md); the advisor posts AdviceAdded. On **situation triggered** (`0x0052A380`): if the advice is not in
`trackList`, append it. If its entry has no tracked vehicle, look for one: an automaton of one of the advice's
automata groups that the player can drive (`CanPlayerDrive`, `0x005245F0`: automaton flag 2 set, flags 1,
`0x8000000` and `0x10000000` clear, automaton type not `0x34` and not `0x14`) and that is not tracked yet. If the
number of tracked vehicles is below `GetMaxCSI()`, track it: the dispatch indicator view shows an indicator over it
(`TrackOccupant(…, 4, …)`), and the vehicle gets the manager as a listener. Otherwise the advice may take over the
vehicle of another entry. On **situation untriggered** (`0x0052A650`): untrack its vehicle, drop the entry, and
give free indicator slots to entries that have none.

Nothing here touches the simulation: it only marks vehicles.

### 4.3 Starting a mission, the popup

`InitiateSituation(advice, vehicle, showBriefing)` (slot 36, `0x0052E750`): if `advice` is already active, return
false. Otherwise `DeactivateSituation`; hold `advice` as `activeSituation`; `status := 4`; `endDelay := 0`; make
`vehicle` the active automaton and bind the mission script. A coverage mission (type 5) builds its coverage cells; a
target mission (types other than 2 and 5) finds or creates its first target. If that fails: `status := 5` and post
`0xEBB67EF7` without a vehicle (the "could not start" popup). Otherwise, if `showBriefing`, post `0xEBB67EF7` with
the vehicle (the briefing popup). Without a briefing the caller activates the mission at once (slot 37).

`ShowSituationPopup` (`0x0052E910`) creates the scripted window `0x2BB16D50`/`0xEBB16D71` (briefing),
`0x0BBC06B6`/`0xEBBC081E` (success, and "could not start") or `0x4BBC080F`/`0xEBBC081E` (failure), takes a
`cSC4AutoGamePause(simulator, clock, sound)` and runs it **modally**. It tests neither the popup-dialog flag nor
"safe" (modal-dialogs.md §4). Buttons (`0x287259F6` in data1, button in data2): 6 = start the mission
(`ActivateSituation`), 7 = joyride, 1 = close (after a finished mission: post `0x6BB350E4`; otherwise cancel the
mission); any other button only closes the window.

`ActivateSituation` (slot 37, `0x005276A0`): `status := 3`, run the advice's start script, read the time limit
from the script (`time_limit`, seconds, 0 = none), `elapsed := 0`, send `0xABBBCA64` (data1 = 1), enter the
player-drive mode, `justActivated := 1`.

### 4.4 The animation tick (`0x0052E300`)

With `t` the tick's milliseconds:

1. If `justActivated`: call slot 9 on each of the four automata managers (it sets their flag `+0x4D`; *[inferred]*
   it freezes traffic), clear `justActivated`, start the 4000 ms countdown (`+0xA90 := 4000.0`) and its
   "count_down" effect.
2. Else if the countdown is > 0: subtract `t`; when it reaches ≤ 0, end the countdown effect and call slot 10 on
   each automata manager (clears `+0x4D`). The mission clock does not run during the countdown.
3. Else if a mission is active and `endDelay > 0`: `endDelay -= t × 0.001`; when it reaches ≤ 0: run the
   **success** (status 1) or **failure** (status 2) effects (§4.5), then post `0xEBB67EF7` (the result popup).
4. If a mission is active and the countdown is not running: at 30 s before the time limit (once), the active
   vehicle plays sound `0x6C32871F`; then `CheckForSuccess(t)` gives the new status (`HandleStatusChanged`), and the
   pursuit logic runs.

`HandleStatusChanged(s)` (`0x00524C40`): if `s` differs from `status`, store it; if a mission is active and `s` is 1
or 2: `endDelay :=` the advice's end delay if the active vehicle has flag `0x8000000` and that delay is > 1.0,
otherwise 1.0. A removed active vehicle during `status = 3`, or the time limit, give status 2.

### 4.5 Mission outcome *[compat]*

**Success** (`0x0052C190`), only for the active advice:

1. `successCounts[guid] += 1` (a new GUID starts at 1; a count of `0xFFFF` stays).
2. If the advice has situation flag `0x40`: add its GUID to the first free entry of the preferences' reward list (64
   entries; nothing if already present or full) and save the preferences (`cISC4App` slot 17).
3. The success effect at the vehicle's position.
4. If the advice has a success aura (`success_aura_radius`, `success_aura_magnitude`, radius > 0):
   `aura.AddTransientEffect(x, z, radius, magnitude)` (aura.md §8.2) at the vehicle's (or target's) position.
5. If `success_money ≠ 0`: `budget.DepositFunds(success_money)` (int64, sign-extended from the advice's int32).
6. Call the advice's success script; retire the advice from advisor 29 (`RetireAdvice`, advisor slot 20); then
   **situation untriggered** for it.

**Failure** (`0x00526720`): the failure effect, the failure aura, `budget.DepositFunds(failure_money)` (shipped
values are negative, so this is a withdrawal, which `WithdrawFunds` refuses when funds are too low; budget.md §6),
and the failure script.

Shipped amounts (`sit_constants`, Lua): success 5,000 / 10,000 / 20,000 / 40,000 / 70,000; failure −2,000 …
−10,000; aura radii 32 … 512.

### 4.6 Debug and script entry points

- Cheat `csitrigger <hex GUID>` (only with debug functionality): posts `0xAC1045EF` (data1 = GUID, data2 = 1).
- Lua `sc4game.sitmgr.activate_situation(guid [, showBriefing])` (`0x00525550`): the same post, data2 = the
  boolean; returns whether it posted.
- Lua `get_success_count(guid)` returns `successCounts[guid]` (0 when absent).
- A debug cheat of the automata manager (`0x0050DD30`) calls slot 33 (ported name `ResetTutorials`,
  `0x005266E0`): clear `successCounts`, zero all 64 reward entries in the preferences and save them.

## 5. Random numbers

Only the member `cRZRandomSuper` at +0x28 (wall-clock seed): `RandomUint32Uniform(n)` to pick a random occupant
(`ChooseRandomOccupant`, `0x00526380`) when a mission creates or chooses a target, and `RandomSint32RangeUniform`
and `RandomUint32Uniform(4)` when a coverage mission builds its cells. **No draw happens while no mission is active.**
*[choice]* A FreeCity4 replacement seeds it with `derive(0x0BB14381)` (foundations §6.3).

## 6. Tuning

| Source | Value | Use |
|---|---|---|
| automata tuning exemplar (`0x6534284A`, group `0xA998D30B`, instance 1), property `0xEBFB4A0A` (u8), read by the automata manager | shipped **5** (0 when missing) | max CSI: the number of vehicles that carry an indicator |
| the mission advices (Lua tables of advisor 29) | `time_limit`, `success_money`, `failure_money`, `success_aura_radius`/`_magnitude`, `failure_aura_…`, effects, texts | §4.3–§4.5 |
| exemplar `0x6534284A`/`0x690F693F`/(advice image), property `0xEC1B2361` | | whether the popup plays its sound |

## 7. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0x0BB14381`) *[compat]*

```
u16  version = 2
u32  count                         number of entries in successCounts
count × { u32 guid; u16 successCount }     hash-map order (any order is valid on read)
u8   indicatorsVisible             written always; read only when version ≥ 2
```

`Write` (`0x00527040`) returns true unless the stream is in error. `Read` (`0x00529010`): read the version; if it is
**greater than 2** return false (versions 0–2 are accepted); set `loadedFromSave`; clear `successCounts`; read the
count and each entry (a GUID read twice keeps the **first** value); if version ≥ 2, read `indicatorsVisible` (any
non-zero byte is true); return true unless the stream is in error. Nothing else is saved: an active mission is lost
on save (the advice survives in the advisor's record), the tracking is rebuilt at PostCityInit (§3).

Corpus: 421 distinct saves; the record exists in **11** (cities saved by builds that write city version ≥ 11; the
other 410 have no component 52 and the city creates a fresh one). All 11 parse exactly with no trailing bytes:

| Saves | Payload (hex) | Meaning |
|---|---|---|
| 8 (Berlin: Konradshohe, Spandau, Tegel; FC4 Test copies; London: Fulham, Kensington; Timbuktu: New City (32)) | `02 00 00 00 00 00 01` | no successes, indicators shown |
| Timbuktu: Big City Tutorial, Rush Hour Tutorial | `02 00 00 00 00 00 00` | indicators hidden |
| Timbuktu: Making Money Tutorial | `02 00 01 00 00 00 FD 1E 15 8C 01 00 01` | one success of mission `0x8C151EFD` |

## 8. Interface slot table *[compat]*

`cISC4CitySituationManager` (vtable `0x00A97E58`). "x87" = `float` in ST0.

| Slot | Offset | Method | Original | Args (dwords) | Callee pops | Return |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface(iid, void**)` | `0x005246B0` | 2 | 8 | AL |
| 1 | `+0x04` | `AddRef()` | `0x00524750` | 0 | 0 | EAX |
| 2 | `+0x08` | `Release()` | `0x00525520` | 0 | 0 | EAX |
| 3 | `+0x0C` | `Init()` | `0x0052DC40` | 0 | 0 | AL (1) |
| 4 | `+0x10` | `Shutdown()` | `0x00527AC0` | 0 | 0 | AL (1) |
| 5 | `+0x14` | `GetMissionSuccessCount(guid)` → u16 | `0x00526320` | 1 | 4 | AX (upper half of EAX undefined) |
| 6 | `+0x18` | `GetActiveSituation()` → advice | `0x00527620` | 0 | 0 | EAX |
| 7 | `+0x1C` | `GetCurrentStatus()` → int (1–5) | `0x005274F0` | 0 | 0 | EAX |
| 8 | `+0x20` | `GetActiveAuto()` | `0x00527630` | 0 | 0 | EAX |
| 9 | `+0x24` | `GetCurrentTarget()` | `0x00527640` | 0 | 0 | EAX |
| 10 | `+0x28` | `GetCurrentTargetIndex()` | `0x00527510` | 0 | 0 | EAX |
| 11 | `+0x2C` | `GetNumTargets()` | `0x00527520` | 0 | 0 | EAX |
| 12 | `+0x30` | `GetNumTargetsRemaining()` = slot 11 − slot 10 | `0x00527530` | 0 | 0 | EAX |
| 13 | `+0x34` | `GetCurrentTimeLimitSeconds()` → float | `0x00527550` | 0 | 0 | **x87** |
| 14 | `+0x38` | `GetCurrentSituationTime()` → float | `0x00527560` | 0 | 0 | **x87** |
| 15 | `+0x3C` | `GetCurrentTimeRemainingSeconds()` → float (= 13 − 14) | `0x00527570` | 0 | 0 | **x87** |
| 16 | `+0x40` | `GetCurrentTargetName(cIGZString&)` | `0x005290F0` | 1 | 4 | EAX (the argument) |
| 17 | `+0x44` | `GetSituationAura(float& radius, float& magnitude)` | `0x00524B70` | 2 | 8 | AL (false when no mission) |
| 18 | `+0x48` | `GetSituationMoney(int64&)` | `0x00524BB0` | 1 | 4 | AL |
| 19 | `+0x4C` | `IsSituationAvailable(automaton)` | `0x00525CB0` | 1 | 4 | AL |
| 20 | `+0x50` | `IsSituationAvailable(occupant)` (overload) | `0x00525BC0` | 1 | 4 | AL |
| 21 | `+0x54` | `InitiateSituationByGuid(guid, bool showBriefing, data3)` | `0x0052A8B0` | 3 | 12 | AL |
| 22 | `+0x58` | `InitiateSituationForOccupant(occupant, …)` | `0x00525DC0` | 2 | 8 | AL |
| 23 | `+0x5C` | `InitiateSituationByAutomataGroup(group, …)` | `0x0052AB00` | 3 | 12 | AL |
| 24 | `+0x60` | `StartJoyride(…)` | `0x00524E20` | 3 | 12 | via call (`0x00506850`) |
| 25 | `+0x64` | `TreatAutomatonAsGroup(…)` | `0x0052A7A0` | 2 | 8 | none |
| 26 | `+0x68` | `SetMaxCSI(u8)` (process-wide) | `0x00524BF0` | 1 | 4 | none |
| 27 | `+0x6C` | `GetMaxCSI()` → u8 | `0x005245C0` | 0 | 0 | AL |
| 28 | `+0x70` | `GetTrackedAutomataCount()` | `0x00527690` | 0 | 0 | EAX |
| 29 | `+0x74` | `SetCSIVisible(bool)` | `0x00524C20` | 1 | 4 | none |
| 30 | `+0x78` | `GetCSIVisible()` | `0x00527500` | 0 | 0 | AL |
| 31 | `+0x7C` | `HandleTuningParametersChanged()` | `0x005245D0` | 0 | 0 | AL (1) |
| 32 | `+0x80` | `CountdownIsInProgress()` (countdown > 0) | `0x00527580` | 0 | 0 | EAX (0 or 1) |
| 33 | `+0x84` | reset mission progress (ported name `ResetTutorials`) | `0x005266E0` | 0 | 0 | none (tail call to `cISC4App` slot 17) |
| 34 | `+0x88` | deleting destructor | `0x00527AA0` | 1 | 4 | EAX |
| 35 | `+0x8C` | `InitiatePlayerDriveMode(bool)` | `0x005252D0` | 1 | 4 | none |
| 36 | `+0x90` | `InitiateSituation(advice, automaton, bool showBriefing)` | `0x0052E750` | 3 | 12 | AL |
| 37 | `+0x94` | `ActivateSituation()` | `0x005276A0` | 0 | 0 | none |
| 38 | `+0x98` | `DeactivateSituation()` | `0x005294C0` | 0 | 0 | none |
| 39 | `+0x9C` | `EndSituation()` | `0x00529ED0` | 0 | 0 | none |

The slot names are the symbol port's; the argument meanings of slots 22–25 are *[inferred]*. Slots 13–15 return
`float` in ST0 (verified: each is a single `fld` of a member or a difference of two); declaring them `void` or
`int` leaves a value on the x87 stack (docs/abi/README.md).

**Minimal replacement** *[choice]*, if FreeCity4 ever replaces the class: keep the record (§7) byte for byte,
answer slot 6 with null, slot 7 with 4, slots 8–12 and 28 with 0, slots 13–15 with 0.0, slots 16–25 and 32 with
false, slot 27 and 30 with the stored values, and keep `successCounts` for Lua `get_success_count`.

## 9. Who reaches in

| Caller | Uses |
|---|---|
| the city (component 52) | creates it (city version ≥ 11), `Init`, `Shutdown`, save and load |
| automata manager (`0x00510690` `CreateAutomatonInternal`; `0x0050DD30` `DoMessage`) | slots 19, 27, 28 when a new automaton might carry an indicator; slot 33 from a debug cheat |
| vehicle manager (`0x00562620`) | slot 31 on a tuning change |
| 3D view (`InvokeCatalogItemCommand`, `DoVehicleMenu`, the mission status panel) | slots 23, 30 (and the menu), 6, 13, 15 |
| player-drive control (`cSC4ViewInputControlPlayerDrive`) | slots 5, 6, 22, 32; posts `0x0BB324BD` |
| MySim dispatch (`EnterUDriveIt`, `0x00437340`) | slots 19, 24 |
| getaway van (`cSC4GetawayVan::PostInit`) | slots 5, 7, 9 |
| minimap | slots 6, 8, 9 |
| advice detokenizer (`gAdviceDetokenizerFunction`, `0x00417AF0`) | slots 6, 10–12, 15–18 for mission text tokens |
| obliterate-city tool (`0x0049D390`), two functions near `0x00440220` | slots 29, 30 |
| CitySituations advisor | posts AdviceAdded / AdviceRemoved; the manager calls its `RetireAdvice` |

No caller reads its fields by offset.

## 10. Quirks

- `QueryInterface(1)` returns the serializable sub-object (§1).
- `Init` overwrites saved success counts with 1 for every GUID in the user's reward list (§4.1 step 8): the count is
  a property of the profile as much as of the city.
- AdviceRemovedAll is subscribed but ignored, so indicators of a removed advisor's advices stay until the vehicles
  go (§3).
- The situation popup ignores the popup-dialog switch (§4.3).
- `0x2C16BDE7` has a handler and no sender (§3).

## 11. Open questions

See QUESTIONS.md, section "From the re-specs10 specs" (CS1–CS3).

## 12. Conformance tests (must exist in `engine/conformance` if the class is replaced)

1. **Record round trip**: the three corpus payloads of §7 read and write back byte-identical; version 3 is
   rejected; version 1 reads the map and leaves `indicatorsVisible` at its constructor value 1.
2. **Duplicate GUID**: a record with GUID `g` twice (counts 2, then 5) reads as `g → 2`.
3. **Profile rewards**: a loaded count `g → 7` with `g` in the preferences' reward list reads 1 after `Init`.
4. **Idle tick**: with no active mission, 10,000 animation ticks post nothing, draw nothing and change no state.
5. **Success payout**: an active advice with `success_money = 10000`, aura (256, m) and a vehicle at (x, z); status
   1 and 1.0 s of ticks → `DepositFunds(10000)`, `AddTransientEffect(x, z, 256, m)`, `successCounts[g] = 1`, and one
   `0xEBB67EF7` post.
6. **Failure penalty with no money**: funds 1,000 and `failure_money = −3000` → the withdrawal is refused (funds stay
   1,000), the popup is still posted.
7. **No city end**: funds forced below −100,000 with a mission active → this component posts no `0xAA5CF1E3` and
   calls no `cISC4App` close or region request (the budget does, budget.md §5.6).
8. **Interface ABI**: slots 13–15 return in ST0; every other slot leaves the x87 stack unchanged.
