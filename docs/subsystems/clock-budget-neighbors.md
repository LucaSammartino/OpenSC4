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

CLSIDs: cSC4BudgetSimulator `0xE990BE01`, cSC4DepartmentBudget `0xE990BFFC`, cSC4LineItem `0xAA313C9F`.
Windows code is at `0x68FC80–0x6951A0` (budget sim) and `0x6A96B0–0x6AC3F0` (department budget). Offsets below
are **Mac** offsets unless they are marked Win. Windows offsets shift by `-0x14` inside the tax-group array
(the Win rate is at `+0x70 + 0x80*i`).

### B.1 Scheduling and messages **[confirmed]**

`Init` (Win `0x692FF0`, Mac `0018FF94`) calls `AddAgent(this, level, "cSC4BudgetSimulator", flags=2)` for levels
**0 (once), 6 (month), 4 (day)**. It subscribes to `0x26D31EC4` CityEstablished, `0x29222F02` ToggleBudgetDialog,
`0x08DAC3E8` IdealMonthlyFundingChanged and `0x08DAC3E7` FundingPercentageChanged; DoMessage ignores the last two.
It registers the cheats `RiskyMoney` (`0xCBF31943`) and `WeaknessPays` (`0x4AB87766`), and `Moolah` (`0x493F4FE6`)
only in debug builds. It also publishes the game-data registry values "budget", "total_funds" and "deposit_funds".
For a new city, it sets funds to `InitialFunds[difficulty]` and each tax group's rate to `InitialTaxRate[difficulty]`,
with a cap of MaxTaxRate. It clears the per-group history and seeds history series `0xAABAAA9F..0xAABAAAAA`
(365 entries) and `0x4A5BA3D0` (funds/1000).

`DoMessage` (Win `0x693450`, Mac `0019071E`):

| Message | Action |
|---|---|
| `0x66956810` SimBegin (once) | `PostFundsChangedMessage` (Win `0x68FE10`) → broadcasts `0x0772FAD4` kSC4MessageFundsChanged |
| `0x66956814` SimNewDay | `DoDailySimulation` (Win `0x6900F0`): for each of the 12 tax groups, pushes `round(rate*10)` into history series `0xAABAAA9F + i`. **There is no money movement on days.** |
| `0x66956816` SimNewMonth | `DoMonthlySimulation` (Win `0x692510`), see B.6 |
| `0x26D31EC4` CityEstablished | `ResetFundsToStartingLevel` (Win `0x690450`) + `ResetTaxRatesToDefault` (Win `0x690490`) |
| `0x29222F02` ToggleBudgetDialog | sets the budget-window-open flag (Mac `+0x85C`) from data1 (`0x892230F2` → 1, `0xC92230EE` → 0), then `ShowBudgetWindow` |
| `0x230E27AC` cheat | `WeaknessPays`: funds += 1000. `Moolah n`: SetTotalFunds(n). `RiskyMoney` (only if the city is established): `DepositFunds(10000)` and `n += 1`. With probability `1 - 1/(0.2n + 1)`, it triggers disaster `0x42963812` at a random point in the middle half of the map (`x = W*(3/8 + rand/4)`); if the disaster starts, `n = 0`. The disaster type is not identified. |

### B.2 Funds **[confirmed]**

- `+0x28` s64 **funds**. `+0x30` s64 is a **shadow copy**. Every legitimate setter (`SetTotalFunds`, `DepositFunds`,
  `WithdrawFunds`, `ResetFunds…`) writes both. `DoMonthlySimulation` starts with `if funds != shadow: funds = shadow`.
  This is an anti-memory-editing guard **[inferred purpose]**.
- `DepositFunds(x)`: a negative `x` becomes `WithdrawFunds(-x)`. Otherwise it saturates at `INT64_MAX` and posts FundsChanged.
- `WithdrawFunds(x)`: **fails, returning false, if `x > funds`**. So direct purchases (plopping, bulldozing) cannot
  overdraw. Only the monthly settlement can make funds negative.
- `GetEstIncome() = YTDIncome + lastMonthlyIncome * (13 - currentMonth)`, and `GetEstExpenses` likewise.

### B.3 Taxes (ComputeTax, Win `0x691E20`, Mac `0018E626`) **[confirmed]**

There are 12 tax groups. Each group is a 0x80-byte record at Mac `+0x80 + 0x80*i`: `+4` rate (float, percent),
`+8` max rate, `+0x0C..+0x6B` 12 × s64 (per-month history), `+0x6C` s64, `+0x74` s64 **weighted assessed value**,
`+0x7C` s64 **monthly tax income**.

| idx | group | idx | group |
|---|---|---|---|
| 0 | R$ | 6 | CO$$ |
| 1 | R$$ | 7 | CO$$$ |
| 2 | R$$$ | 8 | I-R (agriculture) |
| 3 | CS$ | 9 | I-D |
| 4 | CS$$ | 10 | I-M |
| 5 | CS$$$ | 11 | I-HT |

The tax type comes from `kTaxTypeByPurposeAndWealth[purpose*4 + wealth]` (s16[9][4], Win `0x00AB0840`, dumped from
the exe). Purpose 1 Residence → {-,0,1,2}; 2 Commerce-Services → {-,3,4,5}; 3 Office → {-,-,6,7}; 4 Tourism →
{-,3,4,5} (taxed as CS); 5 Agriculture → 8; 6 Processing (dirty) → 9; 7 Manufacturing → 10; 8 HighTech → 11.
Purpose 0 and wealth 0 are untaxed (-1). The purpose numbering follows `cISC4BuildingOccupant::PurposeType`
**[inferred]**.

ComputeTax runs **monthly only**. It walks every lot through the LotManager's safe list:

```
for lot in lots:
    bldg = lot.GetBuilding(); if !bldg or !lot.IsDeveloped(): continue     # lot vt+0xC0 / +0x94
    occ  = bldg.GetPurposeAndWealth()          # {purpose, buildingWealth}
    lotW = lot.GetWealth()                      # lot vt+0xA4
    t = kTaxTypeByPurposeAndWealth[occ.purpose][lotW]; if t == -1: continue
    value  = bldg.exemplar[0xE91A0B5F]          # s64 building value, default 0
    lv     = LandValueSim.Get(lot.x, lot.z)     # byte 0..255 at the lot origin cell
    land   = round(LVCurve(lv) * lot.sizeX * lot.sizeZ)      # response curve, property 0x4AC519B8
    assessed = int64((land + value) * OccWeight[t] * WaningWealth[occ.buildingWealth - lotW])
    group[t].weightedAssessed += assessed
    tax = rate[t] * DifficultyTaxMult[difficulty] * assessed / 1200       # rate in % → /100, per month → /12
    group[t].monthlyTax += tax (double); total += tax
return int64(total)
```

With the shipped data, `LVCurve` is `{(0,0),(255,0)}`. So land value contributes **nothing**, and tax is
`rate% × buildingValue / 12 × 1.2|1.1|1.0` (easy/medium/hard). Example: a $10,000 building at 9% on hard
yields $75/month.

`GetTaxIncome(t)` (UI) recomputes `DiffMult * rate * weightedAssessed / 1200` from the stored assessed value. The Aura
sim also reads tax rates (`ComputeTaxRateChangeEffect`). Demand and desirability effects of tax rates live in other
simulators.

### B.4 Expenses: departments and line items **[confirmed unless marked]**

- Buildings carry **budget item** properties. They are parallel arrays indexed by item: `0xEA54D283` department ID,
  `0xEA54D284` line ID, `0xEA54D285` purpose ID, `0xEA54D286` monthly cost (s64), `0xEA54D287` local funding %
  (float; default 100). The property names are **[inferred]**. `GetBudgetItemInfo` (Win `0x6937B0`) turns them into
  `BudgetItem{dept, line, purpose, cost:s64, index, localFunding = pct/100}`. Examples from the data: City Hall cost
  470 (dept `0xEA59717A`), Grand Rail Station 300 (dept `0xC921CEEB`, line 1). Only 80 exemplars in
  `SimCity_1.dat` carry these directly; the rest presumably inherit them from parent cohorts **[inferred]**.
- `AddBudgetItemToBudget(holder, i)` → `GetDepartmentBudget(dept).GetLineItem(line).AddToFullExpenses(cost)`.
  The civic, power, plumbing, traffic and other sims call it on lot activation; removal reverses it.
- Department = line items (`map<lineID, cSC4LineItem*>` at `+0x40`) plus a set of **locally funded objects**
  (`(propertyHolder, purpose)` at `+0x58`). Department fields: `+0x10` ID, `+0x14` name, `+0x28` budget group,
  `+0x2C` funding % (SC4Percentage, 0..1), `+0x30` max allowed %, `+0x34` s64 YTD spending, `+0x3C` fixed-funding flag.
- Line item fields: `+0x0C` name, `+0x24` s64 secondary info, `+0x2C` s64 **full expenses** (cost at 100%),
  `+0x34` s64 **current expenses**, `+0x3C` s64 income, `+0x48` ID, `+0x4C` display flags, `+0x50` is-local-funding
  item, `+0x54` owner department, `+0x58` type (type 1 is never scaled by the department slider).
- **Funding a non-local line**: `current = int64(full * dept.fundingPct)` (`SetFullExpenses`). `SetFundingPercentage`
  uses `max(0, int64(full*pct + 0.5))`.
- **Local (per-building) lines**: `current = Σ_objects cost * localPct` (`RecalculateLineItemCurrentExpenses`).
- `SetFundingPercentage(p)` (Mac `001AC31E`): clamps `p` to [0, maxAllowed]. It ignores fixed-funding departments.
  It rescales the non-local lines, then `DoDepartmentFundingChanged` **redistributes the department % across locally
  funded buildings** in whole dollars: the target is `round(Σcost * p)`, applied proportionally to each object's cost,
  with each object clamped to [0, round(cost*maxPct)]. It iterates until the remainder is 0. It writes each object's
  new local % back through `ChangeBudgetItemLocalFunding`. It then posts `0x08DAC3E7` FundingPercentageChanged or
  `0x08DAC3E8` IdealMonthlyFundingChanged, both via `SendBudgetChangeMessage` (data1 = department ID).
  `SetLocalFundingPercent` posts `0x6A5FD3FD` BudgetForOccupantChanged.
- **How funding affects services**: service sims ask for the building's `BudgetItem.localFunding` through
  `GetBudgetItemForPurpose(holder, purpose)` and map it through their own response curve. Example:
  `cSC4PowerSimulator::GetCurrentPlantEfficiencyPercent = curve(localFunding*100)/100 * condition`, purpose `0xCA58E540`.
  The curves are in each service sim's tunables (out of scope here).
- Department IDs seen in the data (names **[inferred]** from the building names): `0xA2963983` police,
  `0xA2963984` jail, `0x28F55A9F` fire, `0x09188F4C` education (+`0x4A538CC6` schools), `0x09188F42` health
  (+`0xAA538CB3`), `0x8910BC8A` power, `0xAA4014B4` water, `0x6A42A8F5` sanitation/landfill, `0xC921CEEB` transit,
  `0xC921CEEA` tolls, `0xEA59717A` civic/reward buildings (city hall, statues), `0x2A5A723F` "business deal" rewards
  (toxic dump, prison, army base), `0x8A416368` landmarks, `0x8A4260A6` cruise ship.

### B.5 Bonds **[confirmed]**

`Bond = {s32 dueMonth, s32 dueYear, s64 amount, s64 fullCost, s64 remaining, s64 paid, s64 monthlyPayment, s64 principalLeft}`,
stored in a `std::list` at Mac `+0x790`. `+0x798` is the count and `+0x38` s64 is the total borrowed.

```
IssueBond(amount):                                      # Win 0x6916F0, Mac 0018DE54
    require amount != 0 and bondCount < MaxBonds
    a = (amount / BondIncrement) * BondIncrement, or BondIncrement if that is 0
    require a <= MaxBondAmount and totalBorrowed + a <= MaxBonds*MaxBondAmount   # GetCurrentBorrowingLimit
    pay  = GetMonthlyPaymentForLoan(a) = a*(r/12) / (1 - (1 + r/12)^-n)          # r = BondInterestRate/100, n = BondPaybackMonths
    full = pay * n                                                               # GetFullCostOfLoan
    due  = today + n months; push Bond{due, a, full, full, 0, pay, a}
    totalBorrowed += a; DepositFunds(a); post 0xC772FADA BondIssued
```

With the shipped values (r = 8.5%, n = 120): $10,000 → $123/month, about $14,800 in total.

Each month, every bond pays `monthlyPayment` (added to expenses and to the YTD bond payments), and `remaining -= pay`.
On the due month: `totalBorrowed -= amount`, the bond is erased and `0xCA3FBDCE` BondFinished is posted. Otherwise
`principalLeft -= pay - principalLeft*r/12`, and in the last month `pay = principalLeft`.

### B.6 Monthly settlement (DoMonthlySimulation, Win `0x692510`, Mac `0018ED64`) **[confirmed]**

```
restore funds from shadow (B.2)
m = index of the month that just ended (0..11)
income  = ComputeTax()                       # also stored as last tax income and YTD tax
        + OrdinanceSim.GetMonthlyIncome()    # vt+0x5C; also per month [m] and YTD
        + NeighborsSim.GetMonthlyIncome()    # vt+0x80, deals where this city sells (section C)
        + Σ dept.GetTotalIncome()            # line-item income (e.g. tolls)
expense = OrdinanceSim.GetMonthlyExpense()   # vt+0x58; per month [m] and YTD
        + NeighborsSim.GetMonthlyExpense()   # vt+0x84, deals where this city buys
        + Σ dept: RecalculateAllLineItemCurrentExpenses(); c = dept.GetTotalExpenses(); dept.ytdSpending += c
        + Σ bond payments (B.5)
SetTotalFunds(funds + income - expense)      # the only place funds can go negative
YTDIncome += income; YTDExpense += expense; lastMonthIncome = income; lastMonthExpense = expense
history: 0x6A51178B ← income (u32-clamped), 0xCA5BA3FB ← expense, 0x4A5BA3D0 ← funds/1000
if m == 11 (December ended): zero every YTD accumulator and each dept.ytdSpending
if funds <= MinAllowableFunds:  post 0xAA5CF1E3 kSC4MessageEndGameForBudgetFailure
```

**Bankruptcy is immediate and has no grace period** in this function: the end-game message is posted at every
month-end on which `funds <= MinAllowableFunds`, which is **-100,000** in the data. It is handled by
`cSC4View3DWin::DoMessage` (Win `0x7F53C0`, which also references `0xAA5CF1E3`), presumably with the "you're fired"
sequence **[inferred]**. Earlier warnings come from the advisors and the UI, not from this simulator.

### B.7 Tunables (ReadSimulatorTunableValues, Win `0x692210`, Mac `0018F898`) **[confirmed]**

These come from the exemplar `{0x6534284A, 0xE7E2C2DB, 0x0BFF6824}` "Budget Tuning Parameters" in `SimCity_1.dat`.

| Property | Name (prior art) | Code default | Shipped value |
|---|---|---|---|
| `0x6A9C7718` | Initial Funds [easy, med, hard] (s64) | — | 500000, 200000, 100000 |
| `0x890E0A99` | MaxTaxRate | 25 | 25 |
| `0x090E0A9F` | MinAllowableFunds (s64) | -100000 | -100000 |
| `0x490E0AA6` | BondIncrement (s64) | 1000 | 5000 |
| `0x090E0AAC` | MaxBonds | 5 | 10 |
| `0x890E0AB4` | MaxBondAmount (s64) | 10000 | 200000 |
| `0x8A3FB98D` | BondInterestRate (%; ×0.01 on load) | 0.085 | 8.5 |
| `0x0A3FB9A8` | BondPaybackMonths | 120 | 120 |
| `0x0C032488` | Initial Tax Rate [difficulty] | — | 9, 9, 9 |
| `0xEBFF69E7` | Difficulty Tax Multiplier [difficulty] | 100000 ×3 (placeholder) | 1.2, 1.1, 1.0 |
| `0x8A480DF7` | Waning Wealth multiplier [Δwealth 0,1,2] | 1, 1, 1 | 1, 0.75, 0.5 |
| `0x4AC519B8` | Land Value → Simoleans per tile (response curve) | — | `[0,0, 255,0]` (flat zero) |
| `0xA9188EF5`, `0xE918C920`, `0x4918C925` | TaxRateOccWeight R$, R$$, R$$$ | 1 | 1 |
| `0xA918C929`, `0x4918C92C`, `0x4918C930` | TaxRateOccWeight CS$, CS$$, CS$$$ | 1 | 1 |
| `0x8918C937`, `0x4918C93A` | TaxRateOccWeight CO$$, CO$$$ | 1 | 1 |
| `0x69A36E40`, `0xA918C942`, `0x8918C93E`, `0xA9A36E47` | TaxRateOccWeight I-R, I-D, I-M, I-HT | 1 | 1 |
| `0x89170C6C`/`0x09170C74`/`0xC9170C78`, `0x49170C7C` | LV→R/C/I tax "transmogrifier", MinAverageLV (static members) | — | absent from the data; ComputeTax does not read them |

The MaxBonds × MaxBondAmount borrowing limit is therefore 10 × 200,000 = 2,000,000 with the shipped data.

**Bug, on both builds** **[confirmed]**: the CS$$$ weight `0x4918C930` is stored into the CO$$$ slot (Mac `+0x7F4`,
Win `+0x7E4`), and CO$$$ then overwrites it. So CS$$$ always uses the ctor default of 1.0. This does not matter with
the shipped data, where every weight is 1.

### B.8 Persistence **[confirmed]** (Mac `Write` `00190A50`; Win `Write` `0x690530` / `Read` `0x694B00`)

The budget simulator record is `u16 ver=5`. Then s64 funds, s64 shadowFunds, s64 totalBorrowed, s64 lastTaxIncome.
Then u32 deptCount + {u32 deptID, serialized cSC4DepartmentBudget}. Then 12 × tax group {f32 rate, f32 maxRate,
12 × s64 month history, s64, s64 weightedAssessed, s64 monthlyTax}. Then s64 YTD ordinance expense, s64 YTD
ordinance income, 12 × s64 ordinance expense by month, 12 × s64 ordinance income by month, 6 × s64 YTD accumulators.
Then u32 bondCount + bonds (2 × s32 + 6 × s64 each), s64 YTD bond payments, 4 × s64 last-month totals. Then
u32 groupCount + {u32 id, u32, string name, s64, s64, u32 n, n × u32 deptIDs}. s64 uses stream slot `+0x28`.

Department (`Write` Mac `001ACBBC`, Win `0x6A9D70`): `u16 ver=10; bool; u32 deptID; string name; u32 group;
f32 fundingPct; f32 maxPct; s64 ytdSpending; bool fixed; u32 nLines + n × serialized cSC4LineItem;
u32 nLocal + n × {serialized holder, u32 purpose}; u32; u32`.

Line item (`Write` Mac `001ACD36`): `u16 ver=4; string name; s64 secondary; s64 full; s64 current; s64 income;
u32 id; u32 displayFlags; bool isLocal; u16 type; serialized owner; u32; u32`.

## C. Neighbors and weather

### C.1 cSC4NeighborsSimulator (CLSID `0xC990C0A4`)

Windows primary vtable `0x00AB1EE0` (38 slots; slot order = `cISC4NeighborsSimulator` in `re/prior_art_vtables.md`).
Code is at `0x6CDEF0–0x6D4860`.

**Init** (Win `0x6D38F0`, Mac `00050FEA`) **[confirmed]**: loads the tuning exemplar
`{0x6534284A, 0x7A4A8458, 0x6A469021}` "Neighbor Deals" and builds the deal-price response curve from `0x0A469353`.
It registers agent levels **6 (month), 4 (day) and 10 (idle)** with flags 2. It subscribes to `0xAA7A014D`
PreDemolishNetworkOccupant, `0x278E6003` TerrainHeightsChanged, `0xEA418D8F` (deal-status change, unnamed),
`0x6AC284F3` ConnectionsReady, `0x99EF1143` RemoveOccupant, `0x26D31EC5` CityObliterated and `0x26D31EC4` CityEstablished.

**DoMessage** (Win `0x6D42B0`, Mac `0004FFFC`) **[confirmed]**:

| Message | Action |
|---|---|
| SimNewMonth `0x66956816` | `simulate_deals()` (Win `0x6D0E50`) |
| SimNewDay `0x66956814`, SimIdle `0x6695681A` | `update_deals_in_regional_cities(dealsDirty, statsDirty)` (Win `0x6D0A40`); does nothing unless a flag is set |
| CityEstablished / CityObliterated | clear the deal list, then push to the region |
| TerrainHeightsChanged | `DoTerrainChanged(rect)`, which revalidates the edge connections in the changed rectangle |
| RemoveOccupant (network occupant, type `0x2890D4DE`) / PreDemolishNetworkOccupant | `DeleteOccupantConnection(occupant)` |
| `0xEA418D8F` | data1 ∈ {0, 8} sets "stats dirty" (`+0x49`), otherwise "deals dirty" (`+0x48`); then `simulate_deals()` |
| ConnectionsReady `0x6AC284F3` | sets ready (`+0x0C`). `simulate_deals` does nothing until this is set; before that it re-posts `0xEA418D8F(8)` |

**Connections.** A `cSC4NeighborConnection` (CLSID `0x0A134F24`) is an edge cell where a network leaves the city:
`{u32 type, s32 cellX, s32 cellZ, s32 otherRegionX, s32 otherRegionZ, u8 active, propertyHolder}`. Its
`Write` (Mac `0004D8CC`) uses version 1. The neighbors sim's own savegame record (`Write` Mac `0004CDCC`, version 2) is
just `u32 count + count × serialized connection`. **Deals are not saved in the city.** They are kept in the region
(`cISC4RegionalCity::SetLocalDeals/GetLocalDeals`, vt `+0x114`/`+0x118`). `update_deals_in_regional_cities` pushes
the deal list (`+0x114`) and `UpdateImportExport` (`+0x11C`) to every connected regional city and to the current one.
Connection types **[partly inferred]**: 4 is used by the power sim for power-line connections. Types {1,2,3,8,10,12}
(bitmask `0x150E`) are counted as traffic connections by `cSC4DemandSimulator::EvaluateConnectionUtilization`.
Types 3 and 12 are queried separately there (road/highway-like).

**Effect on other sims** **[confirmed call sites, inferred semantics]**:
- *Traffic*: `cSC4TrafficSimulator` adds each neighbor connection as a trip source/sink
  (`AddNeighborConnection`, `RecalculateNeighborConnectionCapacities`, `WorkOnUpdateNeighborConnections`).
  `cSC4NeighborConnection::UpdateTrafficVolume` stores the per-connection volume.
- *Demand*: `cSC4DemandSimulator::EvaluateConnectionUtilization` sums the traffic sim's volume
  (`TrafficSim vt+0x54(connCell, 3)`) over the traffic-type connections, scales it, and counts the connected regional
  cities. The Commercial/Industrial "Connections" demand exemplars (`{0x6534284A, 0xC7BB4816, 0x3800 / 0x4800}`,
  props `0x47BB3F31/32 = [0x3B20, 100, 0x3B30, 100]`) link those indices into C/I demand. **This** is the
  "neighbor connections lift demand caps" path. It belongs to the demand simulator; see the demand docs.
- *Budget*: `GetTotalMonthlyNeighborDealIncome` (vt `+0x80`) and `…Expense` (vt `+0x84`) feed `DoMonthlySimulation` (B.6).
- *Utilities*: the power, plumbing and pollution (garbage) sims read the active deals of their commodity and count
  the deal amount as capacity (buy) or load (sell). The power sim spreads deal amounts over power connections (type 4).

**Deal types** **[confirmed]** (commodity index in brackets; the type → sim mapping comes from which simulator queries it):

| Type | Meaning | Price property (shipped) |
|---|---|---|
| `0x4773FFE0` / `0x4773FFE1` | power: buy / sell [1] | `0xEA46932C` = 1000 |
| `0x4773FFE2` / `0x4773FFE3` | water: buy / sell [2] | `0x6A469337` = 1000 |
| `0x4773FFE4` / `0x4773FFE5` | garbage: buy (export trash) / sell (accept trash) [3] | `0x8A46933D` = 1000 |

Even types mean this city pays; they are summed into the monthly expense. Odd types mean the neighbor pays.
For both sums, deals flagged `0x3D` (proposed or any failure) are excluded.

**Deal object** (`cSC4NeighborDeal`, CLSID `0x2A134F42`; `Write` Mac `0004D968` / Win `0x6CE400`, version 5)
**[confirmed]**: `type (+0x20)`, region cells on our side and the other side, `amount (+0x34, s32)`,
`monthly cost (+0x4C, s64)`, `amountUsed`, inc/last amounts, `flags (+0x74)`, start / expiration / suspension /
current dates, and a property holder.
Flags: `0x1` proposed, `0x2` active, `0x4` cancelled by the player, `0x8` no suitable connection, `0x10` payer cannot
afford, `0x20` not enough landfill, `0x40` expired (renewal pending), `0x80` belongs to another city (skipped),
`0x100` failure grace period, `0x200` post-failure probation.

`UpdateCost`: `cost = GetDealCost(type, amount, otherCity)`, with a minimum of 1. `UpdateState`: if this city pays and
`regionalCity.GetBudget() < cost` → flag `0x10`; else if `FindConnectionSuitableForDeal` fails → flag `0x08`.

**Deal price** `GetDealCost(type, amount, other)` (Win `0x6D3250`, Mac `00050692`) **[confirmed]**, using the
`cISC4RegionalCity` commodity accessors (Exported `+0xD8`, Imported `+0xE0`, Produced `+0xE8`, Demanded `+0xF0`,
CostPerUnit `+0xF8`):

```
if amount < 1e-8: return 0
buyer, seller = (us, other) if type is even else (other, us)
existing   = amount of an existing deal of this type with this city (or 0)
buyerNeed  = max(basePrice/0xEA4AD2FF + 1, buyer.Demanded + buyer.Exported)       # basePrice per commodity; 0xEA4AD2FF = 2.5
sellerHave = max(1e-8, seller.Imported + seller.Produced)
x1 = (1 - (buyerNeed - (buyer.Produced + buyer.Imported - existing))) / amount
x2 = ((sellerHave - amount) - (seller.Demanded + seller.Exported - existing)) / sellerHave
pMin = seller.CostPerUnit * (1 + 0x6A469342)      # 1.1
pMax = seller.CostPerUnit * (1 + 0xCA469346)      # 1.5
unit = clamp(pMin + (curve(x1) + curve(x2))/2 * (pMax - pMin), pMin, pMax)
return unit * amount                              # monthly
```

`curve` is the response curve at Win `this+0x2C` / Mac `this+0x38`. **Bug on both builds** **[confirmed]**: `Init`
loads `0x0A469353 = [(0,1),(0.1,0.3),(0.3,0.1),(1,0)]` into it, but `UpdateCommodityStats` (Win `0x6D2A20`, at
`0x6D3117`) **reloads the same object** with the garbage cost curve `0x4B61FFC9 = [(0,1e-5),(0.0075,0.0075),(1,0.075)]`.
`simulate_deals` calls `UpdateCommodityStats` every month, so from then on deal pricing uses the garbage curve.

`UpdateCommodityStats(city)` (vt `+0x88`) **[confirmed]**: for power [1], water [2] and garbage [3], it publishes to
the current regional city: Produced (power-sim / plumbing-sim capacity; for garbage, landfill free space /
(12 × `0xEA46934F` years) plus pollution-sim garbage terms), Demanded, Exported, Imported, and
`CostPerUnit = (department monthly expense + buy-deal costs) / (produced + imported + 1)`. The departments are
`0x8910BC8A` power and `0xAA4014B4` water. Garbage cost per unit is `curve0x4B61FFC9(…)`. It also writes
history series (`0xAA11F837/3A`, `0x0A14EBB5`/`0xCA14EBC6`, `0xEA11017B/7E`, `0xEB54F32E`, `0x4B54F333`), and pushes
the city budget and income (regional city `+0xCC`/`+0xD4`).

**Monthly deal state machine** `simulate_deals` (Mac `0004F0F0`) **[confirmed structure; meaning of the states inferred]**:

```
UpdateCommodityStats(city)
for deal in deals (skip flag 0x80):
    deal.UpdateState()
    if deal is garbage-sell (0x4773FFE5) and
       city.Imported[3] + city.Produced[3] + freeLandfill*(1 - 1/(12*0xEA46934F + 0.000107)) < exportBudget:
        exportBudget -= deal.amount; deal.flags |= 0x20
    if deal.flags & 0x3C:                         # some failure
        if 0x04: remove deal (status 2 "cancelled")
        elif not 0x200: flags |= 0x100; flags &= ~0x38; suspensionDate = today   # first strike → grace period
        else: remove deal (status 3 no connection / 4 funds / 5 landfill)        # failure during probation
    elif 0x01 (proposed): if Activate(): startDate = today; status 1; statsDirty   # Activate: flags = (flags | 2) & ~(1|4)
    elif 0x02 (active):
        if 0x100 and today >= suspension + 90d (0x6B588FAD): flags = flags & ~0x100 | 0x200
        elif 0x200 and today >= suspension + 90d + 90d (0x8B58C59E): flags &= ~0x200
        elif 0x40: flags &= ~0x40; if payer.GetBudget() < cost: flags |= 0x10
                   else renew: new expiration = today + (expiration - start); status 7
        elif today >= expiration: flags |= 0x40; status 6
dealsDirty = dealsDirty or anything removed; UpdateCommodityStats(city); update_deals_in_regional_cities(...)
```

`ProposeDeals` (Win `0x6D21A0`) and `create_deal_proposition` (Win `0x6D0520`) generate offers when a neighbor has a
surplus. The thresholds use `0xEA4AE9EF` = 5000 (minimum surplus), `0xCA46931F` = 60 and `0xCA469346`; the details are
not decoded **[open]**.

**Neighbor tuning** (exemplar `0x6A469021`, shipped values; meanings **[inferred]** from their use):

| Property | Value | Used in | Meaning |
|---|---|---|---|
| `0x0A469353` | [0,1, 0.1,0.3, 0.3,0.1, 1,0] | Init | deal-price scarcity curve (overwritten, see the bug above) |
| `0x4B61FFC9` | [0,1e-5, 0.0075,0.0075, 1,0.075] | UpdateCommodityStats | garbage cost-per-unit curve |
| `0xEA46932C` / `0x6A469337` / `0x8A46933D` | 1000 / 1000 / 1000 | cost, max amount, propose | base quantity for power / water / garbage |
| `0xEA4AD2FF` | 2.5 | cost, max amount | quantity divisor |
| `0x6A469342` | 0.1 | cost | minimum markup over the seller's cost per unit |
| `0xCA469346` | 0.5 | cost, propose | maximum markup |
| `0x6B588FAD` | 90 | simulate_deals | grace period in days after a failure |
| `0x8B58C59E` | 90 | simulate_deals | probation period in days after the grace period |
| `0xEA46934F` | 1 | stats, simulate_deals, propose | landfill horizon in years (×12 months) |
| `0xAA46934B` | 1 | create_deal_proposition | — |
| `0xCA46931F` | 60 | ProposeDeals | — |
| `0xEA4AE9EF` | 5000 | ProposeDeals | minimum surplus before a deal is proposed |

### C.2 cSC4WeatherSimulator (CLSID `0x2990C1BC`)

Windows: Init `0x729900`, DoMessage `0x729720`, Read/Write `0x728F40`/`0x725E80` (version 2),
ReadSimulatorTunableValues `0x725AF0`, DoWeeklySimulation `0x728A40`, DoMonthlySimulation `0x729630`.

**Grid** **[confirmed]**: one cell per 16 m (`size * 0.0625`), which is one city tile. It holds a byte temperature
grid, a moisture grid, a vector<vector<bool>> water-edge mask (`+0x80`) and a wind vector. Agent levels
**6 (month), 5 (week), 4 (day)**, flags 2. Initial moisture variation comes from `GenerateMoistureVariation`.

**Monthly** (`DoMonthlySimulation`, data3 = month) **[confirmed]**:
- In January (data3 == 1, once terrain is available), it rebuilds the list of water cells at the city edge (`MakeWaterAreaList` → `RemoveWaterThatIsNotAtCityEdge`) and computes the sea moderation mask.
- `GetNewCurrentAmbientValues(seasonMonth)`. `seasonMonth = month` in the northern hemisphere, `(month+5)%12+1` otherwise (city vt `+0x78`).
  - `ambientTemp = baseTemp(0xA7607D73) * (1 + cos(2π(m-8)/12) * 0xA7607D74 * 0xA7607D75)` (peaks in August).
  - `ambientMoisture = 0xA7607D7A * (1 + cos(2π(m-1)/12) * 0xA7607D7B * 0xA7607D7C)`.
  - Wind: the new speed is uniform in [2, 4]. The direction points away from the lowest-terrain edge: the four edges are sorted by the summed edge heights, plus Gaussian angular noise with σ = `tan(0xA7607D89° / 2) * 0.5`.
- If the season (vt `+0x38`) changed, it resets the wind to the defaults `0xA7607D86/87/88` and posts `0x6716A4E5` (season changed, data = season).

**Weekly** (`DoWeeklySimulation`) **[confirmed]**: it advances a random cell iterator by `cellCount/8` cells, running
`SimulateCellTemperature` on each. When a full pass finishes (about every 8 weeks), it posts
`0x6716A4E6` kSC4MessageWeatherCycleCompleted. It then runs `SimulateMoisture` (about 4.5 KB of code, not decoded) over the grid rectangle.

```
SimulateCellTemperature(x, z):
    T = ambient + 273
    if waterEdge[x][z]: T += (baseTemp - ambient) * 0xA7607D77     # sea moderation, 0.9
    T = T - terrainHeight(x,z) * 0xA7607D78 - vt5C(x,z) * 0xA7607D79 - 273     # 0.082 per m; vt+0x5C is probably moisture [inferred]
    T = 0 if T < -15 else min(T, 45) + 15                          # → 0..60
    temperatureGrid[x][z] = round(T * 4.25)                        # → 0..255
```

**Daily** **[confirmed]**: during the month stored at `+0x15C`, it interpolates the wind vector and speed from the
previous month's values to the current ones by `(dayOfMonth-1)/30`.

**Consumers** **[confirmed call sites]**: `cSC4FlammabilitySimulator` (subscribes to the weather messages),
`cSC4DisasterInstanceFire::ChooseSpreadCell` (wind), `cSC4FloraSimulator::Init`, `cSTETerrain` (terrain
textures/snow, which listens for the season message), `cSC4LightingManager::SetTimeOfDay`, `cSC4EffectsManager` and
the map view. Weather has **no** effect on the budget, demand or neighbors.

**Weather tuning** (`{0x6534284A, 0x7A4A8458, 0x1A2FDB6B}` "Weather Tuning Parameters"; field = Mac offset):

| Property | Shipped | Code default | Field / meaning **[meanings inferred]** |
|---|---|---|---|
| `0xA7607D70` / `71` | 1 / 1 | 8 / 8 | `+0x54/+0x58` grid subdivision X/Z (clamped ≥ 1) |
| `0xA7607D72` | 1 | 1 | `+0x5C` |
| `0xA7607D73` | 65 | — | `+0x64` base ambient temperature |
| `0xA7607D74` / `75` | 0.5 / 0.5 | 0.5 / 0.5 | seasonal temperature amplitude factors (`+0x68` is clamped to [0,1]) |
| `0xA7607D76` | 10 | 6 | `+0xCC` |
| `0xA7607D77` | 0.9 | 0.95 | sea moderation weight (≥ 0) |
| `0xA7607D78` | 0.082 | 0.0035 | temperature drop per unit of height |
| `0xA7607D79` | 0.25 | 0.25 | temperature drop per unit of moisture |
| `0xA7607D7A` / `7B` / `7C` | 0.2 / 0.2 / 1 | 0.5 / 0.5 / 0.4 | ambient moisture base and seasonal amplitude (`7B` clamped to [0,1]) |
| `0xA7607D7D` | 0.35 | 128 (as ×255) | moisture threshold (stored ×255) |
| `0xA7607D7E` / `7F` | 1 / 2.5 | 0.12 / 4 | moisture simulation rates |
| `0xA7607D80` / `81` | 40 / 16 | 64 / 15 | moisture simulation integer parameters |
| `0xA7607D86/87/88` | (absent) | 3, 0, 1 | default wind speed and direction |
| `0xA7607D89` | 60 | → 0.25 | wind direction spread in degrees (stored as `tan(v·π/360)·0.5`) |


---

## D. Interactions summary

```
cSC4Simulator ──SimNewDay──► Budget.DoDaily (tax-rate history only)   Neighbors (push dirty deals to the region)   Weather (wind lerp)
              ──SimNewWeek─► Weather (1/8 of the temperature grid + moisture; WeatherCycleCompleted every ~8 weeks)
              ──SimNewMonth► Neighbors.simulate_deals ─► deal flags and costs ─► GetTotalMonthlyNeighborDeal{Income,Expense}
                             Budget.DoMonthly: ComputeTax(lots × building value × rate) + ordinances + deals + dept line items + bonds
                                               → funds; funds <= -100000 → kSC4MessageEndGameForBudgetFailure
                             Weather.DoMonthly: ambient temp/moisture/wind, season-change message
24-hour clock (real time × 135) ─► lighting, automata, night lights. It is independent of sim speed and date.
Department funding % ─► per-building local funding % ─► service sims' response curves (effectiveness).
Neighbor connections ─► traffic trip sources ─► demand (connection utilization caps); deals ─► utility capacity.
```

Order caveat **[inferred]**: within a month rollover, agents run in set order inside level 6. So whether the
neighbors sim's `simulate_deals` runs before the budget's `DoMonthlySimulation` depends on the agent set ordering
(AgentInfo `operator<`). That ordering decides whether this month's deal costs or last month's are booked.

## E. Open questions

1. Who consumes `kSC4MessageEndGameForBudgetFailure` beyond `cSC4View3DWin` (Win `0x7F53C0`)? Is there any UI-side
   grace period (e.g. a mayor-rating gate) before the player is actually fired?
2. The ordering of agents inside a level (`AgentInfo::operator<`, which decides budget vs. neighbors order in a month).
3. The `cSC4LineItem` type values (0, 1, …) and display flags; how line *income* (tolls, `0xC921CEEA`) is accrued
   (`AddToIncome` callers).
4. The service sims' funding → effectiveness response curves (police, fire, health, education) and their property IDs.
5. `ProposeDeals` / `create_deal_proposition` / `GetDealMaxAmount` thresholds (`0xEA4AE9EF`, `0xCA46931F`,
   `0xAA46934B`) are only partly decoded. The unnamed message `0xEA418D8F`.
6. `SimulateMoisture` (Mac `000A91CE`, about 4.5 KB) is not decoded. The exact meaning of weather props
   `0xA7607D72`, `76`, `7E`–`81`.
7. Disaster `0x42963812` triggered by the `RiskyMoney` cheat.
8. The exact unit of `cRZTimer` resolution 3 vs. 4 (the clock assumes microseconds via `* 1e-6`; the simulator timer
   uses resolution 4, read as milliseconds). This should be checked against `cRZTimer::GetElapsedTime` (Win `0x890198`).
9. The connection type enum (`cISC4NeighborsSimulator::ConnectionType`) is only partly known (4 = power).
