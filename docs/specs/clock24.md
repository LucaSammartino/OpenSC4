# Spec: the 24-hour clock (`cSC424HourClock`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program and its data files. Implementers must work from
this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled
or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (components §3.6, load order §3.7: `Read` comes before `Init`,
messages §5, x87 precision §18), [simulator.md](simulator.md) (the simulation date §2, the simulator's own pause
counters §6 and the "Main Simulator" exemplar §7), [advisors.md](advisors.md) (the popup's hidden pause, "How the
popup stops the simulation"), [modal-dialogs.md](modal-dialogs.md), [lot-developer.md](lot-developer.md) §8.2
(building lights on the day / night message), [mysims.md](mysims.md) (the hour message),
[save-records.md](save-records.md).

## 1. Role

The 24-hour clock is the city's **time of day**. It has nothing to do with the simulation date (simulator.md §2):
it counts game seconds within one day, `0 … 86399`, and advances them from a **real-time timer** multiplied by a
magnification, from the framework's idle loop. The simulation speed and the player's pause button do not drive it
(CK1, checked in the bytes; §3.1): **the clock keeps running while the simulation is paused or at speed 0**.

It keeps two times:

- the **game time** (`seconds`), which advances, is saved, and drives the hour messages that MySims, props, audio
  and the clock display listen to;
- the **visual time** (`visualSeconds`), which either follows the game time (mode 0, the day / night cycle) or is
  pinned to noon (mode 1, "always day") or midnight (mode 2, "always night"). Lighting, building lights and the
  vehicle headlights follow the visual time.

It posts messages when the hour changes, every five game minutes, when the visual hour changes, when day turns to
night and back, and when its pause counters change.

| Class | CLSID | Interfaces (IID) | City component | City getter |
|---|---|---|---|---|
| 24-hour clock | `0xA7E7F929` | `cISC424HourClock` (`0x681BD53A`) and `cIGZUnknown` (`1`) at +0, `cIGZMessageTarget2` (`0x452294AA`) at +4, `cIGZSerializable` (`0xE4FDA3D4`) at +8, a `cRZSystemService` base at +0xC | 50 | `cISC4City` slot 86 (`+0x158`, `Get24HourClock`) |

`QueryInterface` answers the four IIDs above itself and passes any other IID to the system-service base (`this +
0xC`), which answers the system-service interface `0x287FB697` *[inferred]*. The system service has service ID
`0xA7E7F92A` and priority `0xFFE17B80` (−2,000,000). `SL::OnCityInit` (`0x00601C20`) caches the clock in the global
`0x00B43DBC`; `SL::OnCityShutdown` (`0x00601FF0`) clears it.

Factory `0x00445C80` (`GlobalHeapAlloc(0x90)`), constructor `0x00445940`, destructor `0x00445CA0`
(`GlobalHeapFree`). Vtables: primary `0x00A86588` (30 slots, §8), message target `0x00A86578` (`DoMessage`
`0x00445CC0`), serializable `0x00A86560` (Write `0x004452D0`, Read `0x00445340`, GetGZCLSID `0x00445A30`), system
service `0x00A8652C` (slot 8 `Init` thunk `0x00445C70`, slot 9 `Shutdown` thunk `0x00445B00`, slot 10 `OnTick` the
shared default, slot 11 `OnIdle` `0x00445820`) and `0x00A86514` (the reference-counted base of the system service;
slot 3 is the deleting destructor thunk).

## 2. State

| Offset | Field | Constructor | Saved |
|---|---|---|---|
| `+0x0C` … `+0x27` | system-service base: service ID, priority, running flag | `0xA7E7F92A`, `0xFFE17B80` | no |
| `+0x28` | `initialized` (u8) | 0 | no |
| `+0x2C` | reference count | 0 | no |
| `+0x30` | `pauseCount` (s32) | 0 | **yes** |
| `+0x34` | `hiddenPauseCount` (s32) | 0 | no |
| `+0x38` | `magnification` (f32): game seconds per real second | 100.0 | yes, but `Init` overrides it from the tuning (§6) |
| `+0x3C` | `minMagnification` (f32) | 0.05 (`0x3D4CCCCD`) | no |
| `+0x40` | `maxMagnification` (f32) | 2000.0 | yes |
| `+0x44` | `seconds` (u32): game time of day | 43200 (noon) | yes |
| `+0x48` | `visualSeconds` (u32) | a copy of `seconds` (43200) | **no** |
| `+0x50` | `timer` (`cRZTimer`, unit 3, §4.3) | stopped, elapsed 0 | no (version 1 saved a timer record, §7) |
| `+0x68` | `nightStart` (u32 hour) | 20 | yes, but `Init` overrides it from the tuning |
| `+0x6C` | `nightEnd` (u32 hour) | 4 | yes, but `Init` overrides it from the tuning |
| `+0x70` | `dayNightNotifications` (u8) | 1 | no |
| `+0x74` | `visualMode` (s32): 0 follow, 1 noon, 2 midnight | 0 | no |
| `+0x78` | an empty string member | empty | no |
| `+0x8C` | an object pointer | null | no |

Nothing in the clock ever sets `+0x8C`. `Shutdown` passes the string at `+0x78` to that object's slot 7 and releases
it, but only when the pointer is non-null: dead code (CK3, verified: the clock's only writes to `+0x8C` are the
constructor's 0 at `0x00445A15` and `Shutdown`'s clear at `0x0044516B`, and no accessor hands the field out).
*[choice]* FreeCity4 omits both fields.

## 3. Cadence and messages

### 3.1 The idle step

`Init` registers the clock as a framework **system service** and adds it to the framework's **on-idle** list. Every
pass of the framework's idle loop calls `OnIdle` (§4.3). The clock has **no simulator agents**: it does not depend
on the simulation date, speed or `Pause()`. It stops only for its own two pause counters (§4.7).

*[compat]* (CK1) The simulator's `Pause` (`0x007021E0`), `HiddenPause` (`0x00702200`), `SetSimSpeed`
(`0x007026D0`), `DoSetSimSpeed` (`0x00702430`) and `EmergencyPause` (`0x00702540`) never reach the clock. The only
callers of the clock's `HiddenPause` (`+0x68`) are `SC4GamePause` (through city slot 86, `0x005F92EA`) and the
scrubber (through the global `0x00B43DBC`, `0x005BD0D6` and `0x005BD39B`) (§9); its `Pause` (`+0x60`) is reached only
by the cheat (§4.8). The 3D view's handler of the simulation-pause message (around `0x007F5714`) only updates the
UI, and `DoTemporarySimHiddenPause` (`0x007E5B40`) pauses the simulator, not the clock. So the player's pause
button and speed 0 leave the time of day, the lighting and the hour messages running.

Since one pass advances the clock by at most 1200 game seconds (§4.3), the clock can move no further than
20 game minutes per idle pass, however much real time passes: a host that starves the idle loop slows the clock in
game time too (CK22). Nothing else advances it.

### 3.2 Cheat codes

`Init` registers two cheat codes with the cheat code manager (`0x00B43CAC`, when it exists):
`"WhatTimeIzIt"` (`0x281F7858`) and `"Stopwatch"` (`0xC9B73FBB`). It also subscribes its message target to both.
`DoMessage` handles only the cheat message `0x230E27AC` (§4.8) and returns true for every message.

### 3.3 Posted messages

Every message goes through one helper (`0x004454B0`). It creates a `cRZMessage2Standard` and sets the type, `void1`
= the clock, the `IGZUnknown` field = the clock, `data2` and `data3`. It then **posts** the message (queued:
`MessagePost(message, 0)` on the message server `0x00B43CCC`; foundations §5) and releases its reference. If there
is no message server, nothing is posted.

| Type | Name (FreeCity4's) | Posted when | `data2` | `data3` |
|---|---|---|---|---|
| `0x44778230` | HourChanged | the game hour changes (§4.3, `SetHour`) | new hour | 0 |
| `0x44778231` | TimeTick | together with HourChanged; also when the game minute crosses a multiple of 5 within the same hour | hour | 0 with HourChanged, else the minute |
| `0x44778232` | VisualHourChanged | the visual hour changes (§4.4) | new visual hour | 0 |
| `0x44778233` | VisualTimeTick | together with VisualHourChanged; also when the visual minute crosses a multiple of 5 within the same hour | visual hour | 0 or the minute, as for TimeTick |
| `0x44778234` | DayNightChanged | the visual hour crosses the night boundary (§4.4), or notifications are switched on (§4.6) | 1 = day begins, 0 = night begins | 0 |
| `0x44778235` | PauseChanged | `pauseCount` goes 0 → 1 or 1 → 0 | 1 paused, 0 resumed | 0 |
| `0x44778236` | HiddenPauseChanged | `hiddenPauseCount` goes 0 → 1 or 1 → 0 | 1 paused, 0 resumed | 0 |

The names are FreeCity4's *[inferred from the behaviour]*. Shipped subscribers are listed in §9. Nothing in the
shipped game subscribes to `0x44778232`, `0x44778233`, `0x44778235` or `0x44778236`; FreeCity4 must post them anyway
for plugins *[compat]*.

## 4. Behaviour

### 4.1 Derived values

All arithmetic is unsigned 32-bit.

```
hour(s)    = s / 3600
minute(s)  = (s − hour(s) × 3600) / 60
second(s)  = s − (hour(s) × 60 + minute(s)) × 60
GetHour / GetMinute / GetSeconds                  use seconds
GetVisualHour / GetVisualMinute / GetVisualSeconds use visualSeconds
GetTimePercentage(out)       *out := f32(seconds × 1.1574074e-05f)   (0x37422E45 = f32(1/86400); seconds unsigned)
GetVisualTimePercentage(out) *out := f32(visualSeconds × 1.1574074e-05f)
isNightHour(h) = ((h − nightStart + 24) mod 24) < ((nightEnd − nightStart + 24) mod 24)
IsNight()      = isNightHour(GetVisualHour())
GetEventHour(i) = nightStart if i = 0, nightEnd if i = 1, else 0
```

With the shipped hours (night 20 → 1) the night hours are 20, 21, 22, 23 and 0, and the day hours are 1 to 19. With
the constructor's hours (20 → 4) the night hours are 20 to 3. If `nightStart = nightEnd` no hour is night.

**The percentage factor** (CK7). The constant at `0x00A86508` is the float `0x37422E45` = 1.1574074e-05 =
f32(1/86400), shared by both getters (`0x00445420`, `0x00445450`); an earlier version of this table gave its bit
pattern as `0x37C22E45` (1/43200), which was a typo. The getter loads the u32 with the unsigned-to-float idiom (an
exact integer load, plus 2^32 when the top bit is set; `seconds` is always below 86,400, so the correction never
fires), multiplies once by the float constant under the session's precision and stores a float. The value is
therefore the product rounded once to float. Examples: noon (43,200) gives exactly **0.5** (`0x3F000000`); 06:00
gives 0.25; 18:00 gives 0.75; 23:59:59 (86,399) gives 0.99998844 (`0x3F7FFF3E`); one second gives the constant
itself.

### 4.2 `Init()` and `Shutdown()`

`Init`: if `initialized`, return true. Otherwise:

1. Add the system service to the framework (`AddSystemService`), then add it to the on-idle list (`AddToOnIdle`).
2. If the cheat code manager exists, handle the cheats one at a time (CK12): register `WhatTimeIzIt`
   (`RegisterCheatCode`, manager slot 3), subscribe the message target to it (manager slot 7, with the cheat ID),
   then register `Stopwatch` and subscribe to it. Without a manager, skip both.
3. If `pauseCount = 0` and `hiddenPauseCount = 0` (both **exactly** 0), start the timer. A negative saved count
   leaves the timer stopped (§4.7, CK14).
4. Read the tuning (§6).
5. Set `initialized` and return true.

When a city is loaded, `Read` runs **before** `Init` (foundations §3.7). So the tuning replaces the saved
magnification and night hours. Only `pauseCount`, `maxMagnification` and `seconds` survive from the save
*[quirk]*.

`Shutdown`: if not `initialized`, return true. Otherwise:

1. Reset the timer (stopped, elapsed 0) and set `seconds := 0`.
2. Handle the dead object at `+0x8C` (§2).
3. Only if the cheat code manager exists (CK13): for `WhatTimeIzIt` and then `Stopwatch`, remove the notification
   (manager slot 8, target and cheat ID), then unregister the cheat code (manager slot 4). Without a manager,
   nothing is unregistered and nothing is logged.
4. Remove the clock from the on-idle list, then remove the system service.
5. Clear `initialized` and return true.

### 4.3 `OnIdle` (the step)

```
if pauseCount > 0 or hiddenPauseCount > 0: return true
e    := timer.GetElapsedTime()                            u32, unit 3 (microseconds, see the timer model)
step := trunc( f32( f32(e × magnification) × 9.99999997e-07f ) )      e converted as unsigned
if step ≤ 60: return true                                 the timer keeps running; nothing changes
if step > 1200: step := 1200
oldHour := GetHour();  oldMinute := GetMinute()
seconds := (seconds + step) mod 86400
newHour := GetHour()
if newHour ≠ oldHour:
    post HourChanged (newHour, 0);  post TimeTick (newHour, 0)
else if GetMinute() / 5 ≠ oldMinute / 5:
    post TimeTick (newHour, GetMinute())
if visualMode = 0: ChangeVisualTime(seconds)              §4.4
timer.Restart()                                           elapsed := 0, running
return true
```

- The constant is the float `0x358637BD` (9.99999997e-07, at `0x00A86510`). The two products are rounded to float.
  Under the session's 24-bit x87 precision (foundations §18), `float32` arithmetic reproduces them. There is no
  float store between them: each multiplication is rounded by the precision control alone (`0x0044584F` …
  `0x00445864`).
- **Converting `e`** (CK9). `e` is loaded exactly as an integer; when its top bit is set (e ≥ 2^31, a timer that ran
  for more than about 36 minutes at unit 3 without a step) 2^32 is added, which rounds `e` to float at 24-bit
  precision. So `f32(e × magnification)` is the product of the **exact** `e` and the magnification, rounded once,
  for every `e` below 2^31, and the product of `f32(e)` above. Since every product above 1200 is clamped, the
  difference cannot change a step.
- **`trunc` and the step's type** (CK8). The truncation is the C runtime's `_ftol` (`0x009EEF04`, foundations §18.1):
  toward zero, to a 64-bit integer, of which the clock keeps the low 32 bits as an **unsigned** step. Both tests
  (`≤ 60` at `0x0044586B`, `> 1200` at `0x00445874`) are unsigned comparisons. Consequences, all reachable only
  with a negative or NaN magnification (a negative minimum through `SetTimeMagnificationMinMax`, or a NaN passed to
  `SetTimeMagnification`, §4.6):
  - a product of −1 or less wraps to a step of 2^32 − n, which is above 1200, so the clock advances by 1200;
  - a product between −1 and 0 truncates to 0: no step;
  - a NaN product, or one of 2^63 or more, gives 0: no step, ever, while the magnification stays NaN.
- Until more than 60 game seconds have built up, the timer is **not** restarted, so short idle passes add up.
- Each step discards the fraction of a game second left after truncation: less than 1 of at least 61, so the day
  runs up to about 1.6 % slower than the magnification says *[quirk]* (CK5). The time between crossing the threshold
  and the idle pass that notices it is not lost; it is part of `e`.
- A long stall (more than 1200 / magnification real seconds) advances the clock by **at most 20 game minutes**.

**Timer model.** The timer (`cRZTimer`, `0x008905C4` `Start`, `0x008905F0` `Stop`, `0x0089058F` `Restart`,
`0x0088FF0A` `Reset`, `0x00890198` `GetElapsedTime`) holds an accumulated elapsed time and a start tick; it is
**running** exactly when the start tick is non-zero.

- `Start` keeps the accumulated time and sets the start tick to now. It does not test whether the timer already
  runs: on a running timer it drops the interval since the last start. The clock never calls it on a running timer
  (§4.2 step 3 and §4.7 start it only after a stop, or on a fresh timer), so *[choice]* FreeCity4 may treat `Start`
  on a running timer as doing nothing.
- `Stop` adds the running interval to the accumulated time and clears the start tick; on a stopped timer it does
  nothing.
- `Restart` sets the accumulated time to 0 and the start tick to now, **whether or not the timer was running**: on
  a stopped timer it starts it (CK15). `OnIdle` calls it only after a step, when the timer is running.
- `Reset` stops it and sets it to 0.
- `GetElapsedTime` is the accumulated time plus, when running, the current interval, converted to the unit and
  truncated to 32 bits. A stopped timer reports its accumulated time unchanged (0 for a timer that never ran).

The original subtracts a calibrated call overhead per interval, and counts at least one tick per interval.
*[choice]* FreeCity4 may use any monotonic clock with microsecond resolution and skip the overhead correction (CK2).
The clock's unit is 3 (pushed by the clock's constructor, `0x0044596F`), which is **exactly microseconds** of
`QueryPerformanceCounter` time (CK2; below).

**Timer layout** (CK21; the clock's timer is at `+0x50`, so these are `+0x50` … `+0x67` of the clock):

| Offset | Field |
|---|---|
| `+0x00` | unit (u32; the constructor `0x0088FEDF(unit)` stores it) |
| `+0x08` | start tick (u64; 0 = stopped) |
| `+0x10` | accumulated ticks (u64) |

Unit 0 counts CPU cycles (`rdtsc`); every other unit counts `QueryPerformanceCounter` ticks (`Start` `0x008905C4`
chooses by `unit = 0`). `GetElapsedTime` (`0x00890198`) converts the 64-bit tick total by unit: 0 and 1 return its
low 32 bits; 2 to 6 multiply it, as float, by a factor computed once from the counter frequency `F`:
2 nanoseconds (`(1 / F) × 10^9`, `0x00B54360`), **3 microseconds** (`(1 / F) × 10^6`, `0x00B54364`, stored at
`0x0088FFFF`), 4 milliseconds (`0x00B54368`), 5 seconds (`1 / F`, `0x00B5436C`), 6 minutes
(`(1 / F) × 0.016666668`, `0x00B54370`), each stored as a float; then `_ftol`, low 32 bits. Any other unit returns 0.
The calibrated overhead (u64 at `0x00B54358`) is subtracted from each interval longer than it; a shorter interval
counts 1 tick.

**Where `F` comes from** (CK2). Nothing is measured for these units: `F` is
`QueryPerformanceFrequency`. A static initializer (`0x00A78F7F`) runs the timer set-up `0x0088FF4E` once at process
start: it raises the process and thread priority, calls `0x0088FDC0(&0x00B13420, &0x00B13428)` and restores the
priority. That routine always stores `QueryPerformanceFrequency` in `0x00B13420` (`0x0088FED4`), and in
`0x00B13428` the CPU cycle rate: the same frequency when the ratio of `rdtsc` to the performance counter is within
(0.98, 1.02) (`0x00AC50B8`, `0x00AC50B0`), otherwise one measured over `SleepEx(100)` loops lasting at least 0.33 s
(`0x00AC50A8`). The unit table above is built from `0x00B13420` (`0x0088FFB9`–`0x0089000D`); the second value only
feeds a separate table at `0x00B54390`–`0x00B543A0`, which the clock does not use. The only measured quantity the
clock sees is the overhead: the smallest difference of 8 back-to-back `QueryPerformanceCounter` pairs
(`0x00890051`–`0x0089009F`). `0x0088FF19` is a plain `QueryPerformanceCounter` call. So at magnification 135 a day
lasts 86400 / 135 = 640 real seconds, plus the truncation loss of §4.3 (an observed 598 steps in 330 s fits).

So a reader can recover the real time between two clock steps: after each step `Restart` leaves the accumulated
ticks at 0 and the start tick at the moment of the step (`0x0089058F`), and the next step's `e` is the counter
reading at that step minus that start tick, less the overhead, × `10^6 / F`. The counter value the step itself read
is not stored; the next start tick is taken a few instructions later, so consecutive start ticks bound `e` from above
by that gap.

### 4.4 `ChangeVisualTime(v)` (`0x00445680`)

```
if visualSeconds = v: return
oldH := GetVisualHour();  oldM := GetVisualMinute()
visualSeconds := v
newH := GetVisualHour()
if newH ≠ oldH:
    post VisualHourChanged (newH, 0);  post VisualTimeTick (newH, 0)
    if dayNightNotifications and isNightHour(oldH) ≠ isNightHour(newH):
        post DayNightChanged (isNightHour(newH) ? 0 : 1, 0)
else if GetVisualMinute() / 5 ≠ oldM / 5:
    post VisualTimeTick (newH, GetVisualMinute())
```

Only hour changes can post DayNightChanged. A step that crosses a 5-minute boundary inside the same hour posts only
the tick.

### 4.5 `SetVisualTimeMode(mode)` and `EnableDayNightTransitionNotifications(b)`

`SetVisualTimeMode(mode)`: if `mode = visualMode`, do nothing. Otherwise store it, then:

| Mode | Action |
|---|---|
| 0 | `ChangeVisualTime(seconds)`, and every later step follows the game time |
| 1 | `ChangeVisualTime(43200)` (noon) |
| 2 | `ChangeVisualTime(0)` (midnight) |
| other | stored only; the visual time stays where it is, and steps no longer move it |

`EnableDayNightTransitionNotifications(b)`: if `b` equals the current flag, do nothing. Otherwise store it. When
switching **on**, post `DayNightChanged(IsNight() ? 0 : 1, 0)`, which reports the current state. Switching off posts
nothing.

### 4.6 Setters

No setter changes the visual time or touches the timer. In mode 0 the visual time catches up at the next step.

| Method | Behaviour | Messages |
|---|---|---|
| `SetHour(h)` | `h ≥ 24` → `h mod 24`. If `h ≠ GetHour()`: `seconds := h × 3600` (minutes and seconds dropped); otherwise nothing | HourChanged (h, 0) and TimeTick (h, 0) when changed |
| `SetMinute(m)` | `m ≥ 60` → `m mod 60`. `seconds := (GetHour() × 60 + m) × 60` (seconds dropped) | none |
| `SetSeconds(s)` | `s ≥ 60` → `s mod 60`. `seconds := (GetHour() × 60 + GetMinute()) × 60 + s` | none |
| `SetTimePercentage(&p)` | `seconds := u32(trunc(f32(p × 86400.0f))) mod 86400`; a negative product wraps through two's complement *[quirk]* | none |
| `SetTimeMagnification(m)` | `magnification := m`; if `m < minMagnification` → min; then if above `maxMagnification` → max. A NaN is stored unchanged | none |
| `SetTimeMagnificationMinMax(min, max)` | store both; if `max < min` → `max := min`. The magnification is **not** clamped again | none |

### 4.7 Pause counters

`Pause(b)` and `HiddenPause(b)` follow the same pattern. Each works on its own counter and posts its own message.

| Call | Effect |
|---|---|
| `Pause(true)` | `pauseCount += 1`. If it is now exactly 1: stop the timer if `hiddenPauseCount = 0`, then post PauseChanged (1, 0) |
| `Pause(false)` | if `pauseCount > 0`: `pauseCount −= 1`. If it is now 0: start the timer if `hiddenPauseCount = 0`, then post PauseChanged (0, 0). At 0 or below, nothing |
| `HiddenPause(true / false)` | the same with `hiddenPauseCount`, the other counter as the guard, and HiddenPauseChanged |
| `IsPaused()` / `IsHiddenPaused()` | the counter > 0 (signed) |

All four return true. The timer runs only while both counters are 0. A pause keeps the elapsed time, so the first step
after a resume can come early. The game uses only the **hidden** pause (§9). The visible pause is reached only
through the Stopwatch cheat.

**Signed counters and exact tests** (CK14). Both counters are signed 32-bit. The tests are of three kinds, and the
difference matters only for a negative count, which can come only from a save (`pauseCount` is saved as an `s32`
with no range check; `hiddenPauseCount` is not saved):

| Test | Where | Kind |
|---|---|---|
| `OnIdle` returns early | §4.3 | the count is **> 0** (signed) |
| `IsPaused` / `IsHiddenPaused` | §8 slots 25, 27 | **> 0** (signed) |
| `Pause(false)` / `HiddenPause(false)` decrements | above | only when the count is **> 0** |
| `Pause(true)` / `HiddenPause(true)` stops the timer and posts | above | the count, after `+1`, is **exactly 1** |
| stop or start the timer "if the other counter is 0" | above | the other counter is **exactly 0** |
| `Init` starts the timer | §4.2 step 3 | both counters are **exactly 0** |

So a saved `pauseCount = −1` gives: `IsPaused` false, `Pause(false)` does nothing, `OnIdle` does not return early,
but `Init` **does not start the timer**. A stopped timer that never ran reports 0 (§4.3), so every `OnIdle` sees a
step of 0 and the **clock stays frozen** at the saved time, and in mode 0 the visual time stays at noon (§4.9). No
hidden pause cycle restarts it either, because `HiddenPause(false)` starts the timer only when `pauseCount` is
exactly 0. The clock resumes only after `Stopwatch pause` (−1 → 0, no message, the timer still stopped) followed by
either a hidden pause cycle (any popup or game pause: `HiddenPause(true)` then `(false)` starts the timer) or
`Stopwatch pause` and `Stopwatch resume` (0 → 1 posts PauseChanged (1, 0), 1 → 0 starts the timer and posts
PauseChanged (0, 0)). Any other negative count behaves the same, with one more `Stopwatch pause` per unit. The
corpus has no such save (§7).

### 4.8 Cheats (`DoMessage`, message `0x230E27AC`)

The cheat message's `data1` is the cheat ID and `void2` is the typed text (a string object). The text is split into
arguments with the framework's command-line parser; `argv[0]` is the cheat name.

| Cheat | Arguments | Action |
|---|---|---|
| `WhatTimeIzIt` (`0x281F7858`) | `argv[1]` | `SetHour(atol(argv[1]))`; fewer than 2 arguments: nothing |
| `Stopwatch` (`0xC9B73FBB`) | `argv[1]`, lower-cased | `"pause"` → `Pause(true)`; `"resume"` → `Pause(false)`; `"magnify"` with an `argv[2]` → `SetTimeMagnification(f32(atol(argv[2])))`; anything else, or a missing argument: nothing |

`atol` stops at the first non-digit and returns 0 for no digits: `WhatTimeIzIt abc` sets the hour to 0, and
`Stopwatch magnify 0.5` gives 0, which is clamped to 0.05.

**Splitting the text** (CK11; the framework's command-line object, `0x00914E14` → `0x00914C70`):

1. The separators are exactly **space, tab and newline** (`\r` and other characters are ordinary).
2. Separators at the start and the end of the text are removed. An empty result gives no arguments at all.
3. The text is scanned left to right with a count of `"` characters seen since the current argument began. A
   separator ends the argument only when that count is **even**; the count then restarts at 0, and any further
   separators that follow are skipped (a run of separators is one break). A separator inside an odd number of
   quotes is part of the argument.
4. The text after the last break is the last argument.
5. Each argument that both **starts and ends** with `"` loses its last character and then, if it is not empty, its
   first. No other quote is removed: `a"b c"d` is the one argument `a"b c"d`, `"x y"` is `x y`, `""` is an empty
   argument, a lone `"` becomes an empty argument, and `"abc` (no closing quote) keeps its quote and runs to the
   end of the text.

`argv[0]` is the cheat name as typed; only the argument count and `argv[1]`, `argv[2]` are used.

**`atol`** (CK10; the C runtime's, `0x009EF7C2`): skip leading white space (the C locale's `isspace`), then one
optional `+` or `-`, then accumulate decimal digits as `total := total × 10 + digit` in **32-bit arithmetic that
wraps** (no overflow check), stop at the first non-digit, and negate the total for `-`. So `4294967296` gives 0,
`3000000000` gives −1,294,967,296 and `-1` gives −1. The two uses:

- `WhatTimeIzIt`: the result goes to `SetHour` as a u32, so `WhatTimeIzIt -1` is `SetHour(4294967295)`, which is
  reduced mod 24 to **15** (§4.6).
- `Stopwatch magnify`: the result is converted as a **signed** 32-bit integer to float (exact load, then a float
  store: rounded to nearest float), then passed to `SetTimeMagnification`. A negative or wrapped-negative value is
  clamped to the minimum (0.05); `magnify 16777217` gives 16777216.0, clamped to 2000.

### 4.9 Worked examples

**Cadence (shipped magnification 135).** The step threshold is `e = 451,852 µs`: `f32(451,852 × 135) = 61,000,020`,
and times the constant that gives 61.00002, so the step is 61. One microsecond less gives 60, so no step. Two cases:

- **Idle passes every 16,667 µs.** After 27 passes `e = 450,009` gives 60 (no step). After 28 passes `e = 466,676`
  gives a step of 63, and the timer restarts. The clock therefore moves about 63 game seconds every 0.467 real
  seconds.
- **The real-time rate.** A game day takes `86400 / 135 ≈ 640` real seconds (10 min 40 s), plus the discarded
  fractions. The five night hours (20 → 1) take about 133 real seconds.

**An evening, mode 0, notifications on.** At 19:59:30 a step of 63 seconds gives 20:00:33. In order, the clock
posts:

1. HourChanged (20, 0);
2. TimeTick (20, 0);
3. VisualHourChanged (20, 0);
4. VisualTimeTick (20, 0);
5. DayNightChanged (0, 0): night begins.

At 00:59:30 a step of 63 gives 01:00:33 and the same sequence for hour 1, ending with DayNightChanged (1, 0): day
begins.

**Five-minute ticks.** 12:09:00 (43,740) plus 62 gives 12:10:02. Minute 9 / 5 = 1 and 10 / 5 = 2, so the clock
posts TimeTick (12, 10). 12:05:00 plus 61 gives 12:06:01: still in the same 5-minute bucket, so nothing is posted.

**Loading a city saved at night.** The save holds `seconds = 75,600` (21:00) and the visual time is not saved. After
`Read`, `seconds = 75,600` and `visualSeconds = 43,200`, from the constructor. `Init` starts the timer. The 3D view
then sets the mode from the player's preference (§9); mode 0 is unchanged, so nothing happens. About 0.45 s later
the first step gives 21:01:01: no hour message, and minute 0 → 1 is no 5-minute crossing. `ChangeVisualTime(75,661)`
moves the visual hour from 12 to 21 and posts VisualHourChanged (21, 0), VisualTimeTick (21, 0) and
DayNightChanged (0, 0). Until then the city is lit as at noon *[quirk]*.

**"Always day" at night.** Visual time 21:00, `SetVisualTimeMode(1)` posts VisualHourChanged (12, 0),
VisualTimeTick (12, 0) and DayNightChanged (1, 0). Later steps advance `seconds` and post the game-time messages,
but the visual time stays at noon.

**Cheats.** `WhatTimeIzIt 25` at 12:30 gives `SetHour(1)`: `seconds := 3600`, and the clock posts HourChanged (1, 0)
and TimeTick (1, 0). The visual time follows at the next step, which posts the visual hour messages and, from 12 to 1,
no day / night change: both hours are day. `Stopwatch magnify 5000` sets the magnification to 2000.

## 5. Random numbers

None.

## 6. Tuning *[compat]*

Exemplar "Main Simulator" (type `0x6534284A`, group `0xE7E2C2DB`, instance `0xE9590E52`, simulator.md §7), read with
IID `0xA52160F5` by `ReadSimulatorTunableValues` (`0x00445220`) at the end of `Init`:

| Property | Use | Missing | Shipped |
|---|---|---|---|
| `0x897E709F` | magnification (f32), applied through `SetTimeMagnification`, so it is clamped to [min, max] | keeps the current value, still passed through the setter | 135.0 |
| `0x09B73421` | `nightStart` (u32 hour) | keeps the current value | 20 |
| `0x09B73422` | `nightEnd` (u32 hour) | keeps the current value | 1 |

All three are read with the converting getters (CK20): the magnification with the Float32 one (`0x005FD4F0`,
`0x0044527E`), the hours with the Uint32 one (`0x005FD480`, `0x0044529C`, `0x004452AB`). So an array gives its
first value through the text round trip, an array of count 0 gives 0 (the getter succeeds), and a float hour is
truncated (data-formats §5.1).

If the exemplar is missing, nothing changes: the magnification is 100.0 (or the saved value) and the night hours
are 20 and 4 (or the saved ones). The hours are not range-checked; an hour of 24 or more only changes the modular
comparison of §4.1.

## 7. Persistence (`cIGZSerializable`, `GetGZCLSID() = 0xA7E7F929`) *[compat]*

```
u16  version                                   2 when written
s32  pauseCount
f32  magnification
f32  maxMagnification
u32  seconds
     version < 2 only: a timer record, 10 bytes (u8 units, u64 elapsed, u8 running),
     read into a temporary timer and discarded
u32  nightStart
u32  nightEnd
```

- `Write` writes version 2 (26 bytes) and returns true unless the stream reports an error.
- `Read` has **no version check**: any version of 2 or more reads as version 2 *[quirk]*. It returns true unless the
  stream reports an error.
- `Read` does not touch the timer, the hidden pause count, the minimum, the visual time, the mode or the
  notification flag. A saved `pauseCount > 0` therefore leaves the timer stopped at `Init` (CK4).

Example (version 2, noon, shipped values): `02 00 | 00 00 00 00 | 00 00 07 43 | 00 00 FA 44 | C0 A8 00 00 |
14 00 00 00 | 01 00 00 00`.

Corpus: all 421 distinct saves parse exactly.

- **Versions.** 409 records are version 1 (36 bytes) and 12 are version 2 (26 bytes).
- **Common values.** Every record has `pauseCount` 0, magnification 135.0, maximum 2000.0 and night hours 20 / 1.
- **Version 1 timer records.** All are `02 | 00×8 | 00`: units 2, elapsed 0, stopped.
- **`seconds`.** Always below 86,400. By hour: 5 h ×2, 10 h ×2, 12 h ×265 (17 of them exactly 43,200), 13 h ×102,
  14 h ×25, 15 h ×16, 16 h ×7, 17 h ×1, 20 h ×1. Most cities were saved soon after they were founded or loaded,
  within a few game hours of the noon start.

## 8. Interface slot table *[compat]*

`cISC424HourClock` (vtable `0x00A86588`). "Pops" is the callee's stack cleanup in bytes. The SDK header
(`cISC424HourClock.h`) declares slots 7, 9, 19 and 20 differently from the game. The game's form is binding.

| Slot | Offset | Method | Original | Args (dwords) | Pops | Return |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `QueryInterface(iid, void**)` | `0x00444E40` (other IIDs: tail call to the system-service base) | 2 | 8 | AL |
| 1 | `+0x04` | `AddRef()` | `0x00825D60` | 0 | 0 | EAX (new count) |
| 2 | `+0x08` | `Release()` | `0x004451F0` | 0 | 0 | EAX (new count; 0 when destroyed) |
| 3 | `+0x0C` | `Init()` | `0x00445B70` | 0 | 0 | AL (1) |
| 4 | `+0x10` | `Shutdown()` | `0x00445130` | 0 | 0 | AL (1) |
| 5 | `+0x14` | `SetVisualTimeMode(s32)` | `0x00445780` | 1 | 4 | none meaningful |
| 6 | `+0x18` | `GetVisualTimeMode()` | `0x00407AC0` (shared code) | 0 | 0 | EAX |
| 7 | `+0x1C` | `GetTimePercentage(float* out)` | `0x00445420` | 1 | 4 | none; the value goes to `*out`. **SDK: `float GetTimePercentage(void)` is wrong** |
| 8 | `+0x20` | `SetTimePercentage(float& p)` | `0x00445480` | 1 | 4 | none meaningful |
| 9 | `+0x24` | `GetVisualTimePercentage(float* out)` | `0x00445450` | 1 | 4 | none; `*out`. **SDK declares `float (void)`** |
| 10 | `+0x28` | `GetHour()` | `0x00444EC0` | 0 | 0 | EAX |
| 11 | `+0x2C` | `SetHour(u32)` | `0x004457C0` | 1 | 4 | none meaningful |
| 12 | `+0x30` | `GetVisualHour()` | `0x00444ED0` | 0 | 0 | EAX |
| 13 | `+0x34` | `GetMinute()` | `0x00444EE0` | 0 | 0 | EAX |
| 14 | `+0x38` | `SetMinute(u32)` | `0x00444F40` | 1 | 4 | EAX (the new `seconds`) |
| 15 | `+0x3C` | `GetVisualMinute()` | `0x00444F10` | 0 | 0 | EAX |
| 16 | `+0x40` | `GetSeconds()` | `0x00444F80` | 0 | 0 | EAX (second within the minute) |
| 17 | `+0x44` | `SetSeconds(u32)` | `0x00444FE0` | 1 | 4 | EAX = `GetMinute()` (not meaningful) |
| 18 | `+0x48` | `GetVisualSeconds()` | `0x00444FB0` | 0 | 0 | EAX |
| 19 | `+0x4C` | `GetEventHour(u32 index)` | `0x00445020` | 1 | 4 | EAX. The SDK's `bool` argument is a full dword index (0 night start, 1 night end) |
| 20 | `+0x50` | `GetTimeMagnification(float& out)` | `0x00445040` | 1 | 4 | EAX = the float's bit pattern (SDK: `uint32_t`) |
| 21 | `+0x54` | `SetTimeMagnification(float)` | `0x00445050` | 1 | 4 | AL (1) |
| 22 | `+0x58` | `GetTimeMagnificationMinMax(float& min, float& max)` | `0x00445090` | 2 | 8 | EAX = the bits of max |
| 23 | `+0x5C` | `SetTimeMagnificationMinMax(float, float)` | `0x004450B0` | 2 | 8 | AL (1) |
| 24 | `+0x60` | `Pause(bool)` | `0x00445540` | 1 | 4 | AL (1) |
| 25 | `+0x64` | `IsPaused()` | `0x00703D50` (shared with `cSC4Simulator`) | 0 | 0 | EAX (0 / 1) |
| 26 | `+0x68` | `HiddenPause(bool)` | `0x004455B0` | 1 | 4 | AL (1) |
| 27 | `+0x6C` | `IsHiddenPaused()` | `0x00703D60` (shared with `cSC4Simulator`) | 0 | 0 | EAX (0 / 1) |
| 28 | `+0x70` | `IsNight()` | `0x004453E0` | 0 | 0 | EAX (0 / 1) |
| 29 | `+0x74` | `EnableDayNightTransitionNotifications(bool)` | `0x00445620` | 1 | 4 | void |

`bool` arguments are read as one byte. `Release` destroys the object when the count is 1 or less, without
decrementing it first: it calls `Shutdown`, then the deleting destructor through the `+0x10` sub-object.

Message target: slot 3 `DoMessage(cIGZMessage2*)`, 1 arg, pops 4, AL = 1. Serializable: slot 3 `Write` and slot 4
`Read`, 1 arg, pop 4, AL; slot 5 `GetGZCLSID()`, EAX. System service: slot 11 `OnIdle(u32)`, 1 arg, pops 4, AL = 1.

**The system-service sub-object** (`+0x0C`, vtable `0x00A8652C`; CK16–CK18, re-answers11), in SDK order: slot 3
`GetServiceID` (`0x009CEDE1`, `0xA7E7F92A`), 4 `SetServiceID`, 5 `GetServicePriority` (`0x0099BE66`), 6
`IsServiceRunning`, 7 `SetServiceRunning`, 8 `Init` and 9 `Shutdown` (the clock's own, through thunks), 10 `OnTick`
(`0x0091559A`: **AL = 1**, pops 4, nothing else), 11 `OnIdle`, 12 `GetServiceTickPriority` (`0x009D4714`). The base
constructor (`0x0091DE0F`) stores its priority argument both as the priority and as the **tick priority**
(`0x0091DE34`, `0x0091DE37`), so both are −2,000,000. Slot 0 of this sub-object is a thunk to the clock's own
`QueryInterface` (`0x00445A80` → `0x00444E40`), so every interface pointer answers the same five IIDs: the four of
§1 and, through the base (`0x0091DD67`), `0x287FB697` with the `+0x0C` pointer. IID 1 is answered by the clock
itself (the primary pointer). Any other IID returns **false and leaves `*out` unwritten** (`0x009457DC`).

**Return values of slots 5, 8 and 11** (CK19). None is meaningful: slot 5 leaves whatever `ChangeVisualTime` left
in EAX, or the mode for an unknown mode; slot 8 leaves the quotient of its division by 86,400 (`0x0044549B`); slot
11 leaves the last message post's result, or the current hour when the hour does not change. No caller reads EAX
after them *[inferred]*; a replacement may return anything.

## 9. Who reaches in

| Caller | Uses |
|---|---|
| the city (component 50) | creation, `Read` (before `Init`), `Init`, `Write`, `Shutdown`; getter slot 86 |
| `SL::OnCityInit` / `OnCityShutdown` (`0x00601C20` / `0x00601FF0`) | fill and clear the global `0x00B43DBC` |
| `SC4GamePause` (`0x005F91C0`), when its clock flag is set | `HiddenPause(true / false)` through the city getter. Its callers: `cSC4App::FullGamePause` (`0x00449380`, all flags), `cSC4App::PartialCloseCity` (`0x00454A60`), `cGZMiscCommandSystemSupport::DoMessage` (`0x005C8B30`), `SC4WriteCityRegionViewThumbnail` (`0x005DDEC0`) and `cSC4AutoGamePause` (`0x005F9320` / `0x005F9360`; advisors.md, modal-dialogs.md) |
| `cSC4ScrubberDescription` methods `0x005BD070` / `0x005BD340` | `HiddenPause(true)` at start and `(false)` at end, when bit 6 of its description flags is set (bits 4 and 5 pause the simulator) |
| `cSC4View3DWin` | `DoPostCityInit` (`0x007ED0F0`): `EnableDayNightTransitionNotifications(true)`, `SetVisualTimeMode(preference byte)`, `GetMinute` and `GetHour` for the clock display. The day / night menu and buttons (`DoDayNightMenu` `0x007E70C0`, `ProcessCommand` `0x007F1C10`; commands `0xCA35CB74` / `0xCA35CB76` / `0xCA35CB78` → modes 0 / 1 / 2) call `EnableDayNight…(true)`, then `SetVisualTimeMode` and store `GetVisualTimeMode()` back into the preference. `UpdateDayNightButtons` (`0x007E6EE0`) reads the mode. `UpdateAmbientLightLevel` (`0x007E6E40`) reads the visual hour and minute. `Update24HourClock` (`0x007EC0B0`) and `Update24HourClockHover` (`0x007EA810`) read the game time; it subscribes to TimeTick |
| `cSC4TutorialUIWinProc::ReStartCurrentTask` (`0x004439F0`) | `SetVisualTimeMode(1)` |
| MySims: `cSC4MySim::DispatchCommute` (`0x00426CE0`); `cSC4MySimDispatch::Init` (`0x00433EB0`) | `GetHour`: a value of `(17 − hour) × 60` for hours 8 to 17 and 5 otherwise is passed to the dispatch (automata side); the dispatcher's `Init` only takes and drops a reference. mysims.md (HourChanged 8 / 17 / 22) |
| `cSC4AudioAmbienceManager` (`_UpdateAlarms` `0x004E8EA0`, `_AddAlarmOccupant` `0x004EA690`) | `GetHour`, `GetMinute` |
| `cSC4PropOccupant::SetTimeOfDayRange` (`0x005ED400`) | `GetHour`, `GetMinute` (lot-records.md: prop time-of-day ranges) |
| `cSC4PropDeveloper::DoMessage` (`0x006F4E90`) | `GetHour`, `GetMinute` on TimeTick; `GetVisualTimeMode` on DayNightChanged |
| `cSC4LotDeveloper::DoMessage` (`0x006CD940`) | `GetVisualTimeMode` on DayNightChanged (lot-developer.md §8.2) |
| `cSC4VehicleManager::DoMessage` (`0x00562620`) | `GetVisualTimeMode` (mode 2 test) on DayNightChanged |
| `cSC4NetworkManager::DoMessage` (`0x00613950`) | `IsNight` |

Message subscribers: HourChanged: the MySim agent simulator and the automata controller manager. TimeTick: the MySim
dispatcher, the prop developer and the 3D view. DayNightChanged: the vehicle manager, the lot developer and the prop
developer.

## 10. Original quirks (reproduced)

- **Visual time after a load.** The visual time is not saved and stays at noon until the first step; a city saved
  at night lights up as day for about half a second, then gets DayNightChanged (§4.9).
- **Tuning beats the save.** The tuning replaces the saved magnification and night hours, because `Read` comes before
  `Init` (§4.2). The minimum and the hidden pause count are not saved.
- **Truncated steps.**
  - Steps of 60 game seconds or less wait.
  - The fraction of each step is lost.
  - Steps are clamped to 1200 (§4.3).
  - HourChanged and TimeTick carry minute 0 even when the step lands later in the hour.
  - One step that crosses several 5-minute boundaries posts one TimeTick.
- **Silent setters.** The setters other than `SetHour` post nothing, and none of them moves the visual time (§4.6).
- **`SetTimePercentage` wraps.** It wraps negative or oversized values instead of clamping them.
- **Saved pause count.** A saved `pauseCount > 0` freezes the clock until `Stopwatch resume` (§7).
- **No version check.** `Read` does not check the version.
- **Shared pause getters.** `IsPaused` and `IsHiddenPaused` are the simulator's code, folded by the linker, reading
  the same offsets.
- **SDK mismatches.** The SDK declarations of slots 7, 9, 19 and 20 do not match the game (§8).

## 11. Open questions

See QUESTIONS.md, section "From the commercial, industrial, civic, park, history and clock specs" (CK1–CK6; CK1 and
CK2 are answered in §3.1 and §4.3). CK7–CK15
(section "From the history warehouse and 24-hour clock cores") are answered in §4.1 (CK7), §4.2 (CK12, CK13), §4.3
(CK8, CK9, CK15), §4.7 (CK14) and §4.8 (CK10, CK11).

## 12. Conformance tests (must exist in `engine/conformance`)

Use a fake timer whose elapsed microseconds the test sets.

1. **Construction**: `seconds = visualSeconds = 43200`, magnification 100.0, min 0.05, max 2000.0, night 20 / 4,
   notifications on, mode 0. `GetTimePercentage` writes 0.5, `GetHour` = 12, `IsNight` = false, `GetEventHour(2)` = 0.
2. **Tuning**:
   - the shipped exemplar gives magnification 135.0 and night hours 20 / 1;
   - with no exemplar the values are unchanged;
   - an exemplar magnification of 5000 gives 2000, and 0.01 gives 0.05.
3. **Step threshold**: with magnification 135:
   - `e = 451,851` changes nothing and does not restart the timer;
   - `e = 451,852` adds 61 seconds and restarts it;
   - `e = 10,000,000` adds 1200;
   - with either pause counter above 0, nothing happens.
4. **Hour step**: 46,799 (12:59:59) plus 61 gives 13:01:00 and posts, in order, HourChanged (13, 0), TimeTick (13, 0),
   VisualHourChanged (13, 0) and VisualTimeTick (13, 0), with no DayNightChanged.
5. **Five-minute tick**: 12:09:00 + 62 posts TimeTick (12, 10) and VisualTimeTick (12, 10); 12:05:00 + 61 posts
   nothing.
6. **Night boundary**:
   - with 20 / 1, visual hours 20 to 23 and 0 are night;
   - entering 20 posts DayNightChanged (0), and entering 1 posts (1);
   - with notifications off, nothing is posted;
   - with 20 / 4, hours 20 to 3 are night;
   - with 5 / 5, no hour is night.
7. **Modes**: at visual 21:00, `SetVisualTimeMode(1)` posts VisualHourChanged (12, 0), VisualTimeTick (12, 0) and
   DayNightChanged (1). Then:
   - steps leave the visual time at 43,200;
   - mode 2 moves it to 0;
   - mode 3 is only stored;
   - mode 0 calls `ChangeVisualTime(seconds)`;
   - setting the same mode again does nothing.
8. **Pause counters**:
   - `Pause(true)` twice posts one PauseChanged (1); `Pause(false)` twice posts one (0); `Pause(false)` at 0 posts
     nothing;
   - the same holds for `HiddenPause` and HiddenPauseChanged;
   - the timer runs only when both counters are 0, and its elapsed time survives a pause.
9. **Setters**:
   - `SetHour(25)` at 12:30 gives 3600 and posts the two hour messages; `SetHour(12)` at 12:30 does nothing;
   - `SetMinute(75)` at 13:20:30 gives 47,700;
   - `SetSeconds(61)` then gives 47,701;
   - `SetTimePercentage(−0.5)` gives 66,496, and `SetTimePercentage(0.999999)` gives 86,399;
   - `SetTimeMagnificationMinMax(10, 5)` gives min 10 and max 10, and leaves the magnification as it was;
   - no setter changes `visualSeconds`.
10. **Notifications**: `EnableDayNightTransitionNotifications(false)` posts nothing. Then `(true)` at visual 22:00
    with 20 / 1 posts DayNightChanged (0); `(true)` again posts nothing.
11. **Cheats**:
    - `WhatTimeIzIt 7` calls `SetHour(7)`, and `WhatTimeIzIt` alone does nothing;
    - `Stopwatch PAUSE` pauses, because the argument is lower-cased;
    - `Stopwatch resume` resumes;
    - `Stopwatch magnify 5000` gives 2000, and `Stopwatch magnify` with no number does nothing.
12. **Save**:
    - `Write` gives the 26-byte example of §7;
    - a version 1 record of 36 bytes reads correctly and skips the timer record;
    - a version 3 record reads as version 2;
    - all 421 corpus records read, and rewriting them gives version 2 with the same values.
13. **Load order**: a save with magnification 50, max 1500 and night 22 / 5, loaded with the shipped exemplar, ends with
    magnification 135.0, max 1500 and night 20 / 1. `visualSeconds` stays 43,200 until the first step.
14. **Percentage** (CK7): `GetTimePercentage` writes `0x3F000000` (0.5) at 43,200, 0.25 at 21,600 and `0x3F7FFF3E`
    at 86,399; the factor's bits are `0x37422E45`.
15. **Step conversion** (CK8, CK9): with min −10 and magnification −5 (set through `SetTimeMagnificationMinMax(−10,
    2000)` then `SetTimeMagnification(−5)`), `e = 1,000,000` adds 1200; with magnification −0.5, `e = 1,000,000`
    changes nothing; with a NaN magnification no `e` ever steps; `e = 0x80000000` with magnification 135 adds 1200.
16. **Negative pause count** (CK14): a save with `pauseCount = −1` loads with `IsPaused` false, and after `Init` no
    `e` steps the clock (the timer was never started); `Pause(false)` does nothing; `HiddenPause(true)` then
    `(false)` still steps nothing; `Pause(true)` (count 0, no message), then `HiddenPause(true)`, `(false)` starts
    the timer and the next large enough `e` steps.
17. **Timer** (CK15): `Restart` on a stopped timer leaves it running with elapsed 0; `Stop` on a stopped timer
    changes nothing.
18. **Cheat parsing** (CK10, CK11):
    - `  Stopwatch   pause  ` pauses (leading, trailing and repeated blanks);
    - `Stopwatch "pause"` pauses (outer quotes removed); `Stopwatch "pa use"` does nothing (`pa use`);
      `Stopwatch pa"use` does nothing;
    - `WhatTimeIzIt -1` sets 15:00; `WhatTimeIzIt 4294967303` (wraps to 7) sets 07:00;
    - `Stopwatch magnify 3000000000` (wraps negative) gives 0.05.
19. **Cheat registration** (CK12, CK13): `Init` with a manager calls register `0x281F7858`, subscribe `0x281F7858`,
    register `0xC9B73FBB`, subscribe `0xC9B73FBB`, in that order; `Shutdown` calls remove-notification then
    unregister for each, in the same cheat order; without a manager neither happens.
20. **Simulation pause** (CK1): with the simulator paused (`Pause`, or speed 0) and the clock's counters at 0,
    idle passes still advance `seconds` and post HourChanged; only the clock's own `HiddenPause(true)` stops it.
