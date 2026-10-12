# Clock, budget, neighbors and weather: reverse engineering notes

Target: `SimCity 4.exe` 1.1.641 (GOG), base `0x00400000`. This document extends [`docs/sim-engine.md`](../sim-engine.md)
(agent scheduler) and does not repeat it. Windows addresses verified in this pass are in
[`re/symbols/clock-budget-neighbors.tsv`](../../re/symbols/clock-budget-neighbors.tsv).

Sources: named Mac Rev A Intel pseudocode (Mac address in brackets), the unnamed Windows Ghidra pseudocode, and the
Windows disassembly. Labels: **[confirmed]** read directly from code; **[inferred]** consistent with the evidence but not proven.

Field offsets are given for the Windows build relative to the primary (`cISC4Simulator`) `this`, unless they are marked Mac.
Windows offsets differ from Mac offsets after `+0x44`, because the class layouts differ slightly.

Stream conventions (gzcom `cIGZOStream`/`cIGZIStream` vtable): `+0x14` u8, `+0x1C` u16, `+0x20` s32, `+0x24` u32,
`+0x30` float32, `+0x48` GetError. This slot order is **[inferred]** from the gzcom-dll headers, and it is consistent
with every Read/Write function in this document. All values are little-endian. A `cRZDate` is written as one u32
Julian day number. A `bool` is written as a u8.

---

## A. cSC4Simulator: speed, tunables, persistence and the 24-hour clock

### A.1 Fields used by speed and time (Windows offsets)

| Win off | Mac off | Type | Meaning |
|---|---|---|---|
| `+0x2C` | `+0x2C` | u8 | initialized (set by `Init`) **[confirmed]** |
| `+0x2D` | `+0x2D` | u8 | "inside OnTick" reentrancy flag (persisted, harmless) **[confirmed]** |
| `+0x2E` | `+0x2E` | u8 | unknown (persisted) |
| `+0x30` | `+0x30` | s32 | pause count (`Pause`/`Resume`); values > 0 mean paused **[confirmed]** |
| `+0x34` | `+0x34` | s32 | hidden-pause count; `Init` sets it to 1, so the game starts hidden-paused until the city view resumes it **[confirmed]** |
| `+0x38` | `+0x38` | s32 | emergency-pause count. `EmergencyPause` also calls `Pause()`, clears `+0x44` (it locks the speed) and posts `0x4A7FB807` on 0→1 **[confirmed Mac]** |
| `+0x3C` | `+0x3C` | s32 | sim speed: 0/1/2. 3 means "unset" (ctor value) **[confirmed]** |
| `+0x40` | `+0x40` | s32 | "SimTime" (`GetSimTime`/`SetSimTime` only; no other reader was found) **[confirmed accessor]** |
| `+0x44` | `+0x44` | u8 | speed change enabled (`SetSpeedChangeEnabledStatus`); `SetSimSpeed` is refused while it is 0 **[confirmed]** |
| `+0x78` | `+0x70` | u32 | frame budget in ms, recomputed each tick (persisted) |
| `+0x7C` | `+0x74` | s32 | gate level = 4 (SimNewDay): the level whose rollover waits for the ms-per-day timer **[confirmed ctor]** |
| `+0x80` | `+0x78` | u32 | current ms per sim day (ctor value 100) **[confirmed]** |
| `+0x84` | `+0x7C` | s32 | target frame rate (fps) for the speed **[confirmed]** |
| `+0xB0` | `+0xA0` | u32[11] | per-level interval counters (minute/hour counters live here) |
| `+0xDC` | `+0xCC` | u8[11] | per-level "fired this frame" flags |
| `+0xE8` | `+0xD8` | cGZDate | current sim date (Julian day at Win `+0xEC`) |
| `+0xFC` | `+0xEC` | u32[3] | ms-per-day table, indexed by speed |
| `+0x108` | `+0xF8` | vector<float>(5) | animation time-dilation table |

### A.2 SetSimSpeed / DoSetSimSpeed **[confirmed]**

`SetSimSpeed` `0x7026D0` [Mac `00167612`] and `DoSetSimSpeed` `0x702430` [Mac `00167598`]:

```
bool SetSimSpeed(s):
    if !speedChangeEnabled: return false
    if speed != s: DoSetSimSpeed(s)
    return true

DoSetSimSpeed(s):
    switch s: 0 -> targetFps = 30; 1 -> targetFps = 20; 2 -> targetFps = 15; default -> return
    speed = s
    msPerDay = msPerDayTable[s]
    dayTimer.Restart()
    PostMessageServerMessage(kSC4MessageSimSpeedChange 0x4A7FB7E9, data1 = s)
```

`Init` calls `SetSimSpeed(0)` when speed is 3 (a new city), or else `DoSetSimSpeed(speed)` (a loaded city).

Pause is not a speed. It is the separate pause counter (`Pause`/`Resume`, message `0xAA7FB7E0`). The UI's
turtle/llama/cheetah map to speeds 0/1/2 **[inferred]**.

How the speed paces the sim **[confirmed]**, from `OnTick` and `SimulateTimePassage`:
- The frame budget is `max(15, 1000/targetFps - msSinceLastTick)` ms, or `max(15, 33 - msSinceLastTick)` while any
  pause is active. A higher speed therefore gives the simulator *more* CPU per frame: 33, 50 or 66 ms.
- The day rollover (gate level 4) happens only when both conditions hold: every agent at levels 4 to 7 has finished
  its previous interval, **and** `dayTimer.elapsed >= msPerDay`. The timer restarts on each rollover. So `msPerDay`
  is a *minimum* real time per game day. On a slow machine, days take longer.
- Minutes and hours have no agents. The counters `+0xB0[2]`/`[3]` are forced to 60 and 24, so they never hold back the day.

### A.3 ReadSimulatorTunableValues **[confirmed]**

`0x7025B0` [Mac `001678EC`], called from `Init`. It reads the simulator tuning exemplar, TGI
`{type 0x6534284A (Exemplar), group 0xE7E2C2DB, instance 0xE9590E52}`, through the resource manager (`0xA52160F5` = cISCPropertyHolder IID).

| Property | Type | Code default | Use |
|---|---|---|---|
| `0x89590E6D` | u32[3] | `{5000, 1000, 100}` | ms per sim day for speed 0/1/2. Used only if the property has exactly 3 values |
| `0x89590E6E` | float[5] | `{0, 1, 1.5, 2, 1}` | animation time dilation (see A.5) |

**Actual values in the game data** **[confirmed]**: the exemplar is uncompressed in `SimCity_1.dat` (not overridden by `EP1.dat`).
It is named "Main Simulator" and also holds the clock tunables (A.8). It was dumped with `python tools/dbpf.py exemplar <SimCity_1.dat> 6534284A E7E2C2DB E9590E52`:

| Speed | UI | targetFps / frame budget | ms per sim day (data) | code default | 30-day month (data) | 365-day year (data) |
|---|---|---|---|---|---|---|
| pause | ‖ | — / 33 ms | no day advance | — | — | — |
| 0 | turtle | 30 / 33 ms | **2000** | 5000 | 60 s | 12.2 min |
| 1 | llama | 20 / 50 ms | **1000** | 1000 | 30 s | 6.1 min |
| 2 | cheetah | 15 / 66 ms | **50** | 100 | 1.5 s | 18.3 s |

These are minimum real times. Days take longer when agents need more frames. On cheetah, real cities are always agent-bound.

Animation dilation in the data: `0x89590E6E = [1, 1, 1.5, 3, 1]`. Paused = 1 (the code default is 0), turtle 1,
llama 1.5, cheetah 3, emergency 1.

### A.4 Start date **[confirmed]**

`GetSimStartDate` `0x7024A0` [Mac `001695C2`] and the ctor both call `date.Set(month=1, day=1, year=2000)`.
`cRZDate::Jday` [Mac `00028496`] is the classic RogueWave algorithm:

```
Jday(m, d, y): if m < 3: m += 9; y -= 1 else m -= 3
               return d + 1721119 + (y/100*146097 >> 2) + (y%100*1461 >> 2) + (153*m + 2)/5
```

So the start date is **Julian day 2451545** (2000-01-01, a Saturday). This is not 2451544. Agent messages carry
`data1 = julian day`.

### A.5 GetAnimationTimeDilation **[confirmed]**

`0x7026F0` [Mac `0016781C`], slot 32. It returns a factor used by animation and automata:

```
if IsEmergencyPaused() or VehicleManager->(vt+0x34)():  return dil[4]   # emergency / disaster-mode case
if IsPaused():                                         return dil[0]   # 0 -> frozen
switch speed: 1 -> dil[2]; 2 -> dil[3]; else -> dil[1]
```

With the shipped data: paused 1, turtle 1, llama 1.5, cheetah 3, and 1 in emergency mode. With the code defaults, paused is 0
and cheetah is 2. So animation only speeds up 3×, while simulation time speeds up 40× (2000 → 50 ms/day). Visual
animation is therefore deliberately decoupled from sim days.

### A.6 Persistence (savegame record of cSC4Simulator, CLSID `0x2990C1E5`) **[confirmed]**

`Write` `0x702290` / `Read` `0x702340` (the cIGZSerializable subobject; `this` = primary + 4):

```
u16  version            = 4      (Read rejects version >= 5)
u8   inOnTick                     (+0x2D)
u8   unknown2E                    (+0x2E)
s32  pauseCount                   (+0x30)
s32  simSpeed                     (+0x3C)  0..2, 3 = unset
s32  simTime                      (+0x40)
u32  frameBudgetMs                (Win +0x78)
11 × { u32 levelCounter; u8 levelFiredFlag }   (levels 0..10)
[version < 4 only: a serialized cRZTimer, read and discarded]
u32  currentJulianDay
```

`msPerDay` and `targetFps` are not saved. `Init` recomputes them from `simSpeed` through `DoSetSimSpeed`.
The hidden-pause and emergency counts are not saved either.

### A.7 Init and DoMessage **[confirmed]**

`Init` `0x703250` [Mac `00167D80`]: sets the initialized flag, `hiddenPause = 1` and calls `ReadSimulatorTunableValues`.
It subscribes to `0x26D31EC1` and `0x8BAC4B43` on the message server, registers the command `0x8BAC4B43`
"GetSimulationDate" with the command server, and resets the 11 level cursors to `begin()`. It sets the speed
(A.2), starts both timers and registers the service for ticks. It also registers the cheats `0x0AC07E24`
(SetTargetFrameRate) and `0x8A78BEFF` (`SimDate`); the second one only when the app's debug flag (vt+0x54) is set.

`DoMessage` `0x703540` [Mac `00168C12`]:
- `0x26D31EC1`: unsubscribe from it and call `Shutdown` (vt+0x34) **[confirmed]**. This is presumably app shutdown **[inferred]**.
- `0x8BAC4B43` "GetSimulationDate" command: for each of 6 output slots, matches the names "year", "month",
  "day of month", "day of week", "day of year" and "number" (Julian), and returns the value as a variant.
- `0x230E27AC` (cheat): `DoCheatCode` `0x703420`. `SetTargetFrameRate n` sets `targetFps = n`.
  `SimDate m d [y]` sets the date and runs `SimulateTimePassage` once.

### A.8 The 24-hour clock (cSC424HourClock, CLSID `0xA7E7F929`, service `0xA7E7F92A`)

The clock is an independent system service for time of day: lighting, night lights and automata schedules. It is
**not** derived from the sim date, and it **ignores sim speed**. It runs on real time scaled by a magnification
factor **[confirmed]**.

Fields (Mac offsets; Windows is the same up to `+0x40` relative to its primary vtable):

| Mac off | Type | Init | Meaning |
|---|---|---|---|
| `+0x2C` | s32 | 0 | pause count (persisted) |
| `+0x30` | s32 | 0 | hidden-pause count |
| `+0x34` | float | 100.0 | time magnification (game seconds per real second) |
| `+0x38` | float | 0.05 | magnification min |
| `+0x3C` | float | 2000.0 | magnification max |
| `+0x40` | u32 | 43200 | seconds since midnight (starts at noon) |
| `+0x44` | u32 | = `+0x40` | "visual" seconds (what the renderer shows) |
| `+0x48` | cRZTimer | | real-time timer (resolution 3, presumably microseconds **[inferred]**) |
| `+0x5C` | u32 | 20 | night start hour (event hour 0) |
| `+0x60` | u32 | 4 | night end hour (event hour 1) |
| `+0x64` | u8 | 1 | day/night transition notifications enabled |
| `+0x68` | s32 | 0 | visual time mode (0 = follow the clock; 1, 2 = override, e.g. forced day or night **[inferred]**) |

`OnIdle` `0x445820` [Mac `001222CC`] **[confirmed]**:

```
if pauseCount < 1 and hiddenPauseCount < 1:
    dt = uint(timer.elapsed * 1e-6 * magnification)      # whole game seconds
    if dt > 60:                                           # accumulate at least one game minute
        dt = min(dt, 1200)                                # at most 20 game minutes per step
        oldHour, oldMinute = GetHour(), GetMinute()
        seconds = (seconds + dt) % 86400
        if GetHour() != oldHour:  DoHourChanged -> post 0x44778230(hour), 0x44778231(hour, 0)
        elif oldMinute/5 != GetMinute()/5: post 0x44778231(hour, minute)    # 5-minute tick
        if visualMode == 0: ChangeVisualTime(seconds)
        timer.Restart()
```

With the code default magnification of 100, one 24-hour cycle takes 864 real seconds. The shipped value is 135 (see below).

`ChangeVisualTime(s)` `0x445680`: if the visual hour changes, it posts `0x44778232`(visual hour) and
`0x44778233`(visual hour, 0). When notifications are enabled and `IsNightHour` flips, it also posts
`0x44778234`(isDay = !night). If only the visual 5-minute bucket changes, it posts `0x44778233`(hour, minute).

`IsNightHour(h)` = `(h - nightStart + 24) % 24 < (nightEnd - nightStart + 24) % 24`. That is 20:00–03:59 with the
code defaults and 20:00–00:59 with the shipped data. `IsNight()` uses the *visual* hour.

`Pause(bool)`/`HiddenPause(bool)` are counters that stop and start the timer. The 0→1 and 1→0 transitions post
`0x44778235`(paused).

Clock messages (names are **[inferred]** from usage): `0x44778230` HourChanged, `0x44778231` TimeChanged (5-min),
`0x44778232` VisualHourChanged, `0x44778233` VisualTimeChanged, `0x44778234` DayNightTransition,
`0x44778235` ClockPauseChanged.

Tunables (`ReadSimulatorTunableValues` `0x445220`, same exemplar `E9590E52`) **[confirmed]**:

| Property | Type | Code default | Field |
|---|---|---|---|
| `0x897E709F` | float | 100.0 | time magnification (applied through `SetTimeMagnification`, which clamps it to [0.05, 2000]). **Data: 135** |
| `0x09B73421` | u32 | 20 | night start hour. **Data: 20** |
| `0x09B73422` | u32 | 4 | night end hour. **Data: 1** (night is 20:00–00:59) |

With the shipped magnification of 135, a 24-hour cycle takes 86400/135 = **640 real seconds (10.7 min)**, whatever the sim speed.

Persistence (`Write` `0x4452D0` / `Read` `0x445340`) **[confirmed]**:
`u16 version=2; s32 pauseCount; f32 magnification; f32 magnificationMax; u32 seconds; [v<2: cRZTimer]; u32 nightStart; u32 nightEnd`.

Cheats **[confirmed]**: `WhatTimeIzIt <hour>` (`0x281F7858`) calls `SetHour(hour)`.
`Stopwatch pause|resume|magnify <n>` (`0xC9B73FBB`) calls `Pause(true)`, `Pause(false)` or
`SetTimeMagnification(n)`; the keyword is lower-cased before matching.

---

## B. Budget (cSC4BudgetSimulator, cSC4DepartmentBudget, cSC4LineItem)

_In progress._

## C. Neighbors and weather

_In progress._
