# SimCity 4 simulation engine — reverse engineering notes

Target: `SimCity 4.exe` 1.1.641 (GOG), SHA256 `3BC5C7FE807FE5AA24A3ABB06E40F13486670BC1150612C09BAF4ECEDAFE1A4B`, image base `0x00400000`.
Tooling: REA 6.4.0 + Ghidra 12.1.4 (headless), plus small Python/capstone scans.

Status legend: **[confirmed]** read directly from disassembly/decompilation · **[inferred]** consistent with evidence, not yet proven.

## 1. Inventory

RTTI is stripped except for STL/exception classes. Names come from a debug **ID→name table** at
`0x00B07FD8–0x00B09418` (648 × `{uint32 id; const char* name}`), dumped to [`re/class_id_table.tsv`](../re/class_id_table.tsv).
It covers GZCOM class IDs, message type IDs, system service IDs and command IDs. **[confirmed]**

### Simulators (GZCOM class IDs)

| Class | CLSID |
|---|---|
| cSC4Simulator (orchestrator / clock) | `0x2990C1E5` |
| cSC4AuraSimulator | `0x0990BDD8` |
| cSC4BudgetSimulator | `0xE990BE01` |
| cSC4BuildingDevelopmentSimulator | `0x8990BE7A` |
| cSC4CivicBuildingSimulator | `0x0A413481` |
| cSC4CommercialSimulator | `0x8990C080` |
| cSC4CrimeSimulator | `0x8990C09A` |
| cSC4DemandSimulator | `0x2990BFF2` |
| cSC4FireProtectionSimulator | `0x2990C05A` (same ID listed for cSC4ProtectionSimulator) |
| cSC4FlammabilitySimulator | `0xE990C045` |
| cSC4FloraSimulator | `0xC990C034` |
| cSC4IndustrialSimulator | `0xA990C022` |
| cSC4LandValueSimulator | `0xE990C18E` |
| cSC4MySimAgentSimulator | `0x29E37DEC` |
| cSC4NeighborsSimulator | `0xC990C0A4` |
| cSC4OrdinanceSimulator | `0xA990C08E` |
| cSC4PlumbingSimulator | `0x0990C075` |
| cSC4PoliceSimulator | `0x6990C04F` |
| cSC4PollutionSimulator | `0x8990C065` |
| cSC4PowerSimulator | `0x0990C005` |
| cSC4ResidentialSimulator | `0x4990C013` |
| cSC4TrafficSimulator | `0x6990C1AA` |
| cSC4WeatherSimulator | `0x2990C1BC` |
| cSC4SimGrid (+ Bool8/Uint8/Sint8/…/Float64 variants) | `0x49B9E600` … `0x49B9E60B` |
| kSC4SimulatorSysServiceId | `0x46956659` |
| kSC424HourClockSysServiceId | `0xA7E7F92A` |

### Simulation clock messages

| Message | ID |
|---|---|
| kSC4MessageSimBegin / SimOneShot | `0x66956810` |
| kSC4MessageSimEmergencyTime | `0x66956811` |
| kSC4MessageSimNewMinute | `0x66956812` |
| kSC4MessageSimNewHour | `0x66956813` |
| kSC4MessageSimNewDay | `0x66956814` |
| kSC4MessageSimNewWeek | `0x66956815` |
| kSC4MessageSimNewMonth | `0x66956816` |
| kSC4MessageSimNewYear | `0x66956817` |
| kSC4MessageSimSpecificDay / SpecificTime | `0x66956818` / `0x66956819` |
| kSC4MessageSimIdle | `0x6695681A` |
| kSC4MessageSimPauseChange / HiddenPause / EmergencyPause / SpeedChange | `0xAA7FB7E0` / `0x4A7FB7E2` / `0x4A7FB807` / `0x4A7FB7E9` |
| kMessageTypeSimCycleComplete (4 distinct IDs) | `0x68F54CE0`, `0x08F415FE`, `0xA8DAF510`, `0xC8EA5018` |

Code placement: cSC4Simulator core is at `~0x00702000–0x00704200`; sub-simulators cluster in `0x00690000–0x00730000`
(e.g. Budget `0x693xxx`, BuildingDevelopment `0x69Dxxx`, Commercial `0x69Exxx`, Crime `0x6A1xxx`, Demand `0x6A8xxx`,
LandValue `0x6BExxx`, Pollution `0x6E7xxx`, Power `0x6F0xxx`, Residential `0x6FExxx`, Traffic `0x71Exxx`). **[inferred from name-string pushes]**

## 2. cSC4Simulator: a cooperative, time-sliced agent scheduler

Sources: Windows disassembly (addresses below), cross-checked against the named Mac Rev A Intel pseudocode from
[KillaVolt/SimCity-4-Deluxe-GOG-Decompiled](https://github.com/KillaVolt/SimCity-4-Deluxe-GOG-Decompiled) (Mac address in brackets).
The full Windows↔Mac map is in [`re/symbols_windows.tsv`](../re/symbols_windows.tsv).

Vtables (installed by the ctor at ~`0x703C62` and dtor at ~`0x704157`):
`0x00AB2FB8` cISC4Simulator (primary, 34 slots), `0x00AB2FA0` cIGZSerializable, `0x00AB2F6C` cIGZSystemService,
`0x00AB2F44` cIGZMessageTarget2, `0x00AB2F54` (cheat-code handler, **inferred**).

### Agents

Every subsystem registers itself in its `Init()` through `cISC4Simulator::AddAgent(target, level, name, flags)`
(`0x704360`, vtable `+0x4C`). Each of the 11 levels holds a `std::set<AgentInfo>` (`level*0x18 + 0x104` on Mac), and
`AgentInfo = { cRZAutoRefCount<cIGZMessageTarget2> target; bool enabled; cRZString name; uint32 flags }`.

| Level | Message (`0x66956810 + level`) | Meaning |
|---|---|---|
| 0 | SimBegin / SimOneShot | runs once, then the agent is erased from the set |
| 1 | SimEmergencyTime | |
| 2 | SimNewMinute | no agent registers here, so the level is skipped |
| 3 | SimNewHour | no agent registers here, so the level is skipped |
| 4 | SimNewDay | |
| 5 | SimNewWeek | |
| 6 | SimNewMonth | |
| 7 | SimNewYear | |
| 8 | SimSpecificDay | |
| 9 | SimSpecificTime | |
| 10 | SimIdle | gets leftover frame time; data3 = ms it may spend |

`flags`: bit 0 means the agent is enabled before the city is established, bit 1 means enabled after
(`SetCityEstablished` chooses which bit applies).

`SendAgentMessage` (`0x7024C0`) builds a `cRZMessage2Standard{type, data1 = julianDay, data2 = 0, data3 = levelValue}`
and calls the agent's `DoMessage` **synchronously**. The bool it returns means "finished". If the agent returns false,
the scheduler keeps its cursor on that agent and calls it again on a later frame. This is how heavy jobs, such as
traffic routing, get spread across frames.

### Agent schedule (from each subsystem's `Init`, Mac pseudocode)

| Agent | 0 once | 4 day | 5 week | 6 month | 7 year | 10 idle | flags |
|---|:-:|:-:|:-:|:-:|:-:|:-:|---|
| Traffic | ✓ | ✓ | | ✓ | | | 2 |
| Residential, Commercial, Demand, BuildingDevelopment | ✓ | ✓ | | ✓ | | ✓ | 2 |
| Aura, LandValue | ✓ | ✓ | | ✓ | | ✓ | 2 |
| Budget, LotDeveloper | ✓ | ✓ | | ✓ | | | 2 |
| Power, Plumbing, Crime, Neighbors | | ✓ | | ✓ | | ✓ | 2 |
| Pollution, Protection (fire/police) | | ✓ | | ✓ | | | 2 |
| Weather | | ✓ | ✓ | ✓ | | | 2 |
| Flora | | ✓ | | ✓ | | | 3 |
| HistoryWarehouse | | ✓ | | | ✓ | | 3 |
| TractDeveloper | | | | ✓ | ✓ | | 2 |
| GrowthDeveloper, PropDeveloper | | ✓ | | | | ✓ | 2 |
| STESurfaceWater | | ✓ | | | | | 2 |

### Frame loop: `OnTick` `0x703E70` [Mac `0016928e`]

```text
calibrate cycle timer; dt = frameTimer.elapsed(); frameTimer.restart()
target = paused ? 33 ms : 1000 / ticksPerSecond            // ticksPerSecond is a member
budget = max(15, target - dt)                               // this+0x70
if (simEnabled && !reentrant):
    sliceTimer.restart()
    do:
        if emergencyPaused: break
        if !paused: SimulateTimePassage(); SimulateAgents()
        if !paused-hard: SimulateIdleTime()
    while sliceTimer.elapsed() < budget
```

### `SimulateTimePassage` `0x702A80` [Mac `0016801e`]
1. Clear the per-level "fired" flags. Return early if paused.
2. `ready = intervalTimer.elapsed() >= msPerInterval` (the sim speed). The granularity member (2 = minute, 3 = hour,
   4 = day) chooses which level waits on the real-time interval. In practice this is the day level.
3. `done[level] = (cursor[level] == set[level].end())`: a level can only roll after every agent at that level has
   finished the previous interval.
4. Minute → hour → day counters roll at 60 and 24. Levels with no agents jump straight to their limit.
5. Tomorrow's date is computed first. If it starts a new week, month or year, the day can't roll until those levels'
   agents are also done.
6. On rollover: `date.AddDays(1)`, then post `SimNewDay(julian, 0, dayOfYear)` and, when applicable, `SimNewWeek(…,
   (dayOfYear-1)/7+1)`, `SimNewMonth(…, month)` and `SimNewYear(…, year)` via `PostMessageServerMessage` (`0x7020E0`).
   These go to the broadcast message server. The scheduler messages to agents are separate.

### `SimulateAgents` `0x7038B0` [Mac `00169124`]
- For each level that fired this frame, `BeginNewInterval(level)` (`0x702F10`) first **synchronously flushes** any
  agent still unfinished from the previous interval, calling it until it returns true. It then resets the level's
  cursor to `begin()`.
- It then round-robins levels 0…9, sending `0x66956810+level` to the agent under each cursor and advancing the
  cursor when that agent returns true. It stops as soon as the frame budget is spent. One-shot (level 0) agents are
  erased after they run.

### `SimulateIdleTime` `0x702DF0` [Mac `00168358`]
Cycles through the idle-level agents and sends `SimIdle(julian, isPaused, min(remaining, budget/3))` until the frame
budget is spent.

### Consequences
- Game time is **demand-driven**. A day only advances once every daily agent has finished, so on slow hardware the
  game slows down rather than skipping simulation work.
- Day is the real atomic step. Monthly work (most "big" recomputations) is spread across the month's daily frames
  through the cursor mechanism, but it is forced to finish before the next month starts.

## 3. Open questions / next steps

- Read `DoSetSimSpeed` / `ReadSimulatorTunableValues` to get the ms-per-day table for each speed.
- Go through each sub-simulator's `DoMessage` to see what it does on once/day/month/idle and what it reads and writes.
- Priority: Traffic, then Demand and RCI, then development (Building/Lot/Tract/Growth), then the grid simulators.
