# Spec: the U-Drive-It tool (`cSC4ViewInputControlPlayerDrive`)

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from the Windows 1.1.641 executable (read in the instruction bytes and in a Ghidra 12.1.4 project of
the Steam build, whose code equals the GOG build's; evidence in `docs/lanes/re-dates.md`). Implementers must work
from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose; *[compat-limit]* depends on something FreeCity4 cannot reproduce, so only the rule is matched.

Related specs: [city-tools-input.md](city-tools-input.md) (§2 the base control), [automata.md](automata.md)
(§6.2 state `0x10` PlayerDrive, §9 the U-Drive-It hooks, the manager slots 9 `Pause` and 10 `Resume`),
[ui-windows.md](ui-windows.md) (the city view slots), [network-overlay-views.md](network-overlay-views.md)
(`SC4TrainJunctionView`).

## 0. Summary

The tool has two modes. In **choosing** mode the simulation and the automata are paused and the player clicks a
vehicle (or a rail junction to switch it). In **driving** mode the clicked automaton runs the player-drive state, the
camera tracks it, the keys and mouse buttons become drive commands, a dashboard window shows its gauges, and the
simulation runs again. Escape, or the vehicle's removal, ends the tool.

## 1. Making it *[compat]*

- **Construction** (`0x00565480`): id `0x2B709676`, cursor `0xC16C4E1B`; no automaton; the camera speed value
  (`+0x40`) `0x3D2C0831`; the two toggles (`+0x44`, `+0x45`) off; a timer; an empty callback list (`+0x64`); the
  cell size and its inverse 1.
- **`Init`** (`0x00565A10`, once): the base `Init`; animation ticks requested (priority `0xFF`); the terrain held, with
  its cell width and inverse; two pick filters (an automaton filter, vtable `0x00A9D894`, and a network-occupant
  filter, vtable `0x00A8FE14`); four interface objects registered with the resource system *[summary]*; the four
  automata managers (city slots 43–46: vehicles, pedestrians, aircraft, watercraft) held; a junction view
  (`SC4TrainJunctionView`, network-overlay-views.md §2: constructor `0x005649D0`, its `Shutdown` `0x00563730`
  releasing its pictures and models) added to the renderer at layer 5, priority 1000; the dispatch vehicle view
  (`0x00B43D04` slot 22) with all its indicators shown (`ShowAllIndicators(4, 0)`); subscriptions to `0xA6B79602`
  and the three messages of the table `0x00B0A110`; the network manager (`0x00B43D54`), the simulator
  (`0x00B43D5C`) and the automata script system (city slot 48) held; with a city view, its key accelerator (slot 31)
  is saved and a new one made from resource `0xA2E3D533` (GZCOM `0x42E967BE`) and installed (slot 32); the three
  command IDs of the table are registered with the framework service `0xEB903A32` (its slot 7).
  The table holds exactly three command/message IDs, in this order: `0xAC1F0449`, `0xCC1F0457`, `0xAC1F045D` (the
  panel messages of §7); the tool subscribes `0xA6B79602` first and then these three, and registers the same three
  with the framework service as its last step. The four interface objects are small message-target objects created
  in order and registered with the resource system in the order fourth, first, third, second, before the four
  managers are fetched. The tool holds no automaton and no hovered automaton when `Init` ends, and `Init` does
  nothing when the tool is already initialised *[compat]*. The initial reference counts of the created objects are not
  determined here (DR8).
- **`Shutdown`** (`0x00563970`, only when initialised; clears the initialised flag first) undoes these in this
  order: the three command IDs unregistered from the framework service; with a city view, the saved key accelerator
  reinstalled and the tool's own released; the dispatch vehicle view's indicators shown again (`ShowAllIndicators(4,
  x)`, where x is that of the city situation manager's slot 30 when it exists, else true); the callback targets
  released and the list emptied; `StopTracking`; the hovered automaton dropped (cursor text cleared); the simulator
  resumed when paused and the four managers resumed (slot 10); `HideUI` (both dashboard windows); the city view's
  UI restored (slot 55 with 0, the inverse of the minimize call of §3); the simulator and network manager released; engine sound event 0 stopped; the subscriptions removed (`0xA6B79602`,
  then the three table IDs); the junction view removed from the renderer (slot 33), its `Shutdown` run and released;
  the four interface objects unregistered and released in the order fourth, first, third, second; the cursor text
  cleared again; the two pick filters, the terrain and the animation tick subscription (priority `0xFF`) released;
  the four managers, the driven automaton, the dispatch vehicle view and the script system released; the base
  `Shutdown` last *[compat]*.

## 2. Choosing mode *[compat]*

- **`EnterChoosingMode`** (`0x00564450`): `StopTracking` (§4); the hovered automaton is dropped and the cursor text
  `0xAB73510C` cleared; the simulator is paused when it is not (its slots 15 and 9) and the four managers paused
  (slot 9).
- **Hover** (`OnMouseMove`, `0x005631C0`): a terrain pick (city view slot 36) gives the junction view's hover cell
  (city `PositionToCell`; none on a miss). While driving (an automaton without flags `0x8000000` and 1, its slot 31),
  the mouse position is written into the drive state (state `0x10`, `+0x214`, `+0x218`). Otherwise the automaton under
  the mouse (`PickAutomaton`, `0x00563140`: renderer slot 65 `Pick` with the automaton filter, the instance's
  occupant answering `cISC4Automaton` `0xA9B40F05`) becomes the hovered one; none → cleared, with the cursor text.
- **Click** (`OnMouseDownL`, `0x005655D0`):
  1. A network occupant under the mouse (`PickNetworkOccupant`, `0x005630C0`, IID `0xA821EF94`) that the network
     manager accepts (its slot 42) plays sound `0x8BE8D9AC` at the occupant's position and becomes the junction
     view's selected cell *[inferred: a rail junction whose route the player's train takes]*.
  2. Otherwise, with no automaton driven, the automaton under the mouse enters driving mode (§3).
  3. While driving, unless the city situation manager (city slot 49) reports its slot 32 *[inferred: a mission's
     start countdown]*, an automaton in group `0x4006` gets command 7 pressed (§5).

## 3. Driving mode (`EnterDrivingMode(a)`, `0x00565160`) *[compat]*

1. The disaster layer (city slot 72) is switched off (its slot 5 with 0) *[inferred]*.
2. The automaton's drivable form (its slot 50) must exist; it is the automaton used below. `StopTracking`; the hover
   is cleared.
3. The city view's UI is minimized (slot 55) and its window `0x43` shown; the dashboard (`ShowUI`, §6) opens with the
   automaton's property `0x2BE8FBF3`, or by default `0x6BE8846C` for an aircraft (type `0x480E6151`) and `0x0BE8824F`
   for anything else.
4. Audio: the occupant's sound property `0xCBBBA52F` starts (AEMS events slot 3), engine event 0, and the three
   values of `0x00B229A8` *[summary]*.
5. The automaton becomes the driven one (reference held); it is given state `0x10` (its slot 52 with
   `0x10, 0, −1.0, 0, 0, 0, 0`); the vehicle manager tracks it (slot 34).
6. The camera: from the automaton's drive settings (its slot 67, then `+0x64` or its AEMS data) the zoom range and
   the camera speed (default zoom 1, level 5, speed `0x3D2C0831`); the city view's `SetZoomLimits(zoom − 1, 4 or 5)`
   (5 when the application preference at `+0xF10` is above 0) and `SetZoom(level − 1)`; the dashboard's zoom buttons
   `0x4992046A` / `0xE9920494` enabled by `CanZoomIn` / `CanZoomOut`.
7. The toggles: `+0x44` on, `+0x45` off; the dispatch vehicle view tracks the occupant; the automaton gets the tool as
   a callback target (its slot 39); the timer is reset; `PauseEverything(false)` resumes the simulator when it is
   paused (its slots 15 and 12 `Resume`) and the managers (slot 10).

## 4. Leaving *[compat]*

- **`StopTracking`** (`0x00562FD0`): with a driven automaton: its callback target removed (slot 40); the dispatch
  vehicle view untracks it; the vehicle manager tracks nothing; the zoom limits back to `(0, 5)`; the AEMS engine
  event stopped and the occupant's sound `0xCBBBA52F` stopped. Only when the automaton is in state `0x10` (its slot 59
  reports the current state): if it is not flagged `0x8000000` and not in group `0x4003` it gets state 0 (its
  slot 13 with 0); otherwise (flagged, or in group `0x4003`) its slot 53 runs. In any other state neither happens
  and the automaton keeps its state. The automaton is then released *[compat]* (DR8).
- **`EndInput`** (`0x005650E0`): the callback message `0x0BB324BD` (§7) with code 1; an automaton for which its slot
  18 is true gets slot 13 with 0; `StopTracking`; the hover cleared; the disaster layer switched on again (slot 5
  with 1); the base `EndInput`.
- **Escape** (`OnKeyDown`, key `0x1B`) ends the input. **`AutomatonRemoved(a)`** (`0x00563310`): when `a` is the driven
  one, `StopTracking` and `EndInput`.

## 5. Drive commands *[compat]*

**Key path** (`OnKeyDown` `0x00566C60`, `OnKeyUp` `0x00566CF0`), in this order:

1. Key down only: a key with flag `0x40000000` (auto-repeat) is consumed and does nothing. Key up has no such test.
2. Key down only: key `0x1B` (Escape) ends the input (§4) and is consumed. Escape never acts on key up.
3. When the city situation manager's slot 32 is true, the key is consumed and does nothing (both down and up).
4. Otherwise, with the automata script system present, the key map (its slot 14) turns the key into a command 0–12.
   Result 13 (none), or no script system, leaves the key **unconsumed** (the tool answers "not handled").
5. The key effects of the automaton's occupant group template (`HandleKeyEffects`, `0x00566A70`, below) get the
   command first; if they do not consume it, the command table runs. The key is consumed whether or not either
   did anything.

**Key effects** (`HandleKeyEffects(c, isDown)`): nothing happens, and the key is not consumed, unless there is a
driven automaton without flags `0x8000000` and 1, the drive state (state `0x10`) exists, the automaton has an
occupant group template (its slot 67), and that template has a key-effect record for command `c`. The drive state
keeps one byte per command (at `+0x205 + c`; the "held" value). With the record's flags:

| Record flag | Effect on the held byte |
|---|---|
| `0x02` (pulse) | the byte is set to 0 and a pulse is signalled with the value `isDown` |
| else `0x01` (momentary) | the byte becomes `isDown` |
| neither (toggle) | on key down the byte is negated (0 becomes 1, anything else becomes 0); on key up it is unchanged |

The effect "changed" when the byte's new value differs from its old one or a pulse is signalled. Only when changed:
flag `0x10` starts or stops a named effect on the automaton, with the argument bit taken from flag `0x04` (a pulse
starts it once through the automaton's slot 47, a held-on byte starts it through slot 44, a held-off byte stops it
through slot 45); flag `0x20` finds the automaton's controller in the manager of city slot 47 (looked up by the
automaton's slot-5 key), creates it when none exists and the byte is on or a pulse was signalled, and then sets it active when the byte
is on, inactive when the byte is off and no pulse was signalled, and leaves it alone on a pulse. Independently of
"changed", flag `0x08` makes the key consumed, so the command table below is skipped. The held byte is written
before the effects run (DR5) *[compat]*.

| Command | Down | Up | Sound event (AEMS start / stop) |
|---|---|---|---|
| 0, 1 | `SetDriveCommand(c, 1)` | `(c, 0)` | 1, 2 |
| 2, 3 | the dashboard light `0x8BCB93FC` (2) or `0x6BCB93F5` (3) on, then `(c, 1)` | light off, `(c, 0)` | 3 |
| 4, 10 | toggles `+0x44` (4) or `+0x45` (10), then `(c, toggle)` | nothing | — |
| 5 | `(5, 1)` | `(5, 0)` | 6 |
| 6, 11, 12 | `(c, 1)` | `(c, 0)` | — |
| 7 | `(7, 1)` | `(7, 0)` | 4 |
| 9 | `(9, 1)`, then the drive state's byte `+0x229` is negated (0 becomes 1, anything else becomes 0) | `(9, 0)`, byte untouched | — |

`SetDriveCommand` is the automaton's slot 60; every command needs a driven automaton without flags `0x8000000` and 1
(`IsDriveAutoValid`, `0x00562ED0`). Details the table does not show *[compat]*:

- The two toggles (commands 4 and 10) and the dashboard lights (commands 2 and 3, on both down and up) change
  **before** the validity test, so they change even with no valid automaton; only the `SetDriveCommand` and the
  sound events are conditional. The toggle value sent is the new value of the toggle (1 or 0).
- Command 9's byte write needs the drive state to exist (when it is absent only `SetDriveCommand` is sent), and
  happens after `SetDriveCommand`. Releasing 9 never touches the byte.
- Command 8 has no keyboard action (the command table does not list it: unknown commands are ignored). It is
  driven only by the right mouse button (down 1, up 0). Keyboard commands outside 0–12 do nothing.
- Mouse buttons share the situation-manager gate of step 3 and the automaton validity test: the right button down
  and up send `(8, 1)` / `(8, 0)` only when a driven automaton without flags `0x8000000` and 1 exists and the
  situation manager's slot 32 is false. The left button's release always clears the junction view's selected cell
  first (ungated); it then turns command 7 off (`(7, 0)`, sound event 4 stopped) under the same gate.

## 6. The dashboard *[compat]*

`ShowUI(id)` (`0x00564570`) does nothing when the dashboard already exists. Otherwise it loads the dashboard window
`id` into the city view and centres it horizontally, then loads a second, smaller panel from resource `0xEC1A5CBF`,
hides it and centres it too. The first window is the **full** panel and the second the **compact** one (§7 swaps
them). Its buttons (`DoWinProcMessage`, `0x00562E20`; only for a button-click record from window `0x287259F6`,
anything else answers "not handled") post commands through the command server (slot 20): `0x4C1A5CC9` →
`0xCC1F0457`, `0x2C16C9CB` → `0xAC1F0449`, `0x4992046A` → `0x6A935CB7`, `0xE9920494` → `0x6A935CBA` (the last two
are the zoom buttons).

On every animation tick `Update` (`0x00566000`) does nothing unless there is a driven automaton without flags
`0x8000000` and 1. Then:

- **Cursor:** with a drive state, the cursor is the state's cursor ID (`+0x114`) unless the state's `+0x9C` is -1.0
  or the cursor ID is 0, in which case it is the default cursor `0xC16C4E1B`; it is applied through the base
  control's set-cursor call. Without a drive state it is left alone.
- **Gauges** (only when the full panel exists; each gauge is a control of that panel found by its ID, and each
  is skipped silently when missing; a gauge is set as "minimum, maximum, value"):

| Control | Value |
|---|---|
| `0xEBCB9403` | range 0 to the tool's camera speed value (`+0x40`); value is the automaton's speed (its slot 34) |
| `0x2BCB940B` | range from the state's `+0x30` to `+0x34`; value is the state's `+0x2C`, held between those limits |
| `0x2BCBCE68` | height over ground (below); this dial runs 0–100 and shows the height modulo 100 (100 subtracted repeatedly while above 100) |
| `0xEC0EA996` (text) | the same height, printed with no decimals |
| `0x6BCBCE73` | range ±0.52359879 (30 degrees) with the state's `+0x238` as value |
| `0x6C0AE7B1` | the state's three values `+0x23C`, `+0x240`, `+0x244` as minimum, maximum, value |
| `0x2BF98D69` | the state's `+0x248`, `+0x24C`, `+0x250` likewise |
| `0xEBF98D37` | the state's `+0x254`, `+0x258`, `+0x25C` likewise |
| `0x2C0C1C8C` | the state's `+0x260`, `+0x264`, `+0x268` likewise |
| `0x0C16C9B9` (button) | button state index 1 set from the toggle `+0x44` |
| `0xAC0EA9DB` (text) | the money string of the budget object's (city slot 52) value at its slot 6 (the cash) |
| `0x8C3923AE` (text) | the integer `state +0x144 − state +0x178` printed as `%d` |
| `0x6C3A0A6F` (button) | state set from the byte at drive-state `+0x1FF` |

  Height over ground: the automaton's position (its slot 7) is looked up in the terrain; for an automaton in group
  `0x4004` (an aircraft) the ground height is the terrain altitude at the position (a fixed fallback when the position
  is off the map); for any other automaton the position is converted to a cell and the ground is the altitude from
  the second terrain pointer (`0x00B43CF8`, slot 18) there, with the same fallback. The gauge value is the
  automaton's altitude minus that ground height. The city view is refreshed afterwards (its slot 58).
- **Engine sound volumes:** the first AEMS volume (its slot 7) is set to the automaton's absolute speed divided by
  the camera speed value (0 when that value is not positive); the second (its slot 8) to the state's `+0x2C`
  normalised across the `+0x30`..`+0x34` range (0 when `+0x34` is not positive; when `+0x34` is below `+0x30` the
  ratio is measured from `+0x30`).
- **Timer:** independent of the above, when the tool's timer is running (§7) and has reached 9000 ms the
  input ends (§4).

Exact semantic names of the drive-state fields are not known; they are listed by offset (DR1).

## 7. Messages and callbacks *[compat]*

- `DoMessage` (`0x00566890`) answers "handled" to every message, known or not:
  - the animation tick `0x86AD10EE` runs `Update`;
  - `0xA6B79602`, when the dashboard exists, **re-queries** the city view's `CanZoomIn` and `CanZoomOut` and sets
    the two zoom buttons `0x4992046A` and `0xE9920494` enabled or disabled from those answers, exactly as on entry
    (§3); it is not an unconditional re-enable. Without a dashboard it does nothing;
  - `0xAC1F0449` hides the full panel and shows the compact one, `0xCC1F0457` shows the full panel and hides the
    compact one, and `0xAC1F045D` toggles on the full panel's visibility (visible: hide it, show compact; hidden:
    show it, hide compact). All three do nothing without a dashboard;
  - the automaton notification `0x09D92FC3` carries a code and the automaton; it is **not** filtered against the
    driven automaton except in code 3. Code 3 → `AutomatonRemoved` (acts only if that is the driven one). Code `0xB`
    (for any automaton) → engine sound event 0 stopped, the dispatch vehicle view told to use the alternate model for
    the tracked occupant (`UseAlternateModel(view, occupant, true)`), and the tool's timer set to units 4 and
    restarted. Code `0xD` → when the notification's state kind is `0x10` and that automaton no longer has a drive
    state, engine sound event 0 is stopped (otherwise nothing). Other codes do nothing.
  - **Timer:** only started by code `0xB`, in milliseconds (framework timer unit 4). `Update` (§6) ends the input
    once the running timer reaches 9000 ms (9 seconds, a float comparison); the timer is reset when driving starts
    (§3 step 7).
- **Callbacks:** `AddCallbackTarget` (`0x00565FA0`) adds a message target; `SendCallbackMessage` (`0x005644E0`)
  sends message `0x0BB324BD` (with the tool and two values) to each target.

## 8. Quirks

1. The simulation is paused while choosing and runs while driving (§2, §3).
2. A left click on a network occupant is taken before the automaton pick (§2).

## 9. Open questions

See [questions/drive-tool.md](questions/drive-tool.md): DR1 (the semantic names of the drive-state fields behind the
gauges), DR2 (the meaning of commands 0–12 and of the automaton slots used), DR8 (initial reference counts).

## 10. Conformance tests

1. **Toggle:** two presses of command 4 send `SetDriveCommand(4, 0)` then `(4, 1)` (starting from on).
2. **Repeat:** a key down with flag `0x40000000` does nothing.
3. **Escape:** key `0x1B` ends the input and switches the disaster layer back on.
4. **Pause:** entering choosing mode pauses the four automata managers; entering driving mode resumes them.
