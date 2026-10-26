# Spec: the aura (mayor rating) subsystem (`cISC4AuraSimulator`)

Status: **implementation spec**, clean-room input. This document describes *behaviour*, not code. It was
written by the analysis team from observing the original program (Windows 1.1.641) and its data files.
Implementers must work from this spec, the public LGPL gzcom-dll SDK headers and general knowledge only. They must
**not** consult decompiled or disassembled code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words: **must**, **should**, **may**. Tagged notes: *[compat]* is required for compatibility with the
original game and its plugins; *[choice]* is a behaviour we chose where the original was arbitrary or undefined;
*[inferred]* is consistent with all evidence but not proven; *[quirk]* is surprising original behaviour that is
kept on purpose.

Related specs: [foundations.md](foundations.md) (streams §2, references §2.7, city components §3.6/§3.7, response
curves §4, messages §5, random numbers §6, slot-order rule §16, framework slot tables §17),
[simulator.md](simulator.md) (agents, levels, finished/unfinished, CPU-cycle timer §5.5),
[simgrid.md](simgrid.md) (grid classes §2, geometry §3, grid slots §4, grid persistence §5, lifetime §6, shared
helpers §7), [budget.md](budget.md) (departments, tax rates, game-data registry §4.4),
[ordinance.md](ordinance.md) (cumulative offsets §6.1).

## 1. Role

The aura simulator computes the **mayor rating** ("aura"): a signed per-tract opinion value in −127..127, and a
single city-wide rating derived from it. Once per game month it runs an 18-state pipeline (§6), spread over many
calls, that

1. stamps a **base value** for every residential lot (from residential tax rates and from the mismatch between land
   value and resident wealth),
2. adds the contributions of parks, landmarks, pollution, schools, hospitals, crime, commute length, congestion and
   building "mayor rating effects" through **response curves**,
3. smooths the result over time per tract (long-term decay) and adds a city-wide ordinance offset,
4. adds the short-lived **transient** effects (fires, tree planting, civic plops, mayor's limo, city situations),
5. publishes the aura grid, the population-weighted **mayor rating** (history series `0x0A5CBF37`), the best and
   worst tracts, and may start a **riot** in a cluster of very unhappy tracts.

It also keeps the **park map** and **landmark map** (radius cones stamped when park or landmark buildings appear,
scaled by department funding) that the growth and tract developers read for desirability, and it can end
service strikes (`BustStrike`).

| Class | CLSID | Interfaces (IID) |
|---|---|---|
| aura simulator | `0x0990BDD8` | `cISC4AuraSimulator` (`0xC773E880`), `cIGZMessageTarget2` (`0x452294AA`), `cIGZSerializable` (`0xE4FDA3D4`), `cIGZUnknown` (`1`) |

The city creates it through GZCOM (class `0x0990BDD8`, IID `0xC773E880`) and stores it in reference **#19** of
its save record (foundations §3.6). The city's `Init` initialises it after the demand simulator and before the
commercial, crime, land value, traffic, pollution and residential simulators (foundations §3.7); the aura's own
`Init` calls none of those, so the order does not matter for `Init`. Other code reaches it through
`cISC4City::GetAuraSimulator()` (compiled city slot 51, `+0xCC`).

**QueryInterface** *[compat]*: `0xC773E880` and `1` → the object (primary pointer); `0x452294AA` → its
`cIGZMessageTarget2` sub-object; `0xE4FDA3D4` → its `cIGZSerializable` sub-object; each success AddRefs and returns
true. Any other IID returns false. **Release**: when the count is above 1, decrement and return the new count;
otherwise call `Shutdown()`, destroy the object and return 0.

## 2. Grids

All grids are `cSC4SimGrid<T>` objects (simgrid §2) created by the aura simulator itself at `Init` (simgrid §6)
over the whole city. **Tract size S = 4 cells** unless stated (simgrid §3; a 256-cell city has 64 × 64 tracts).

| Grid | Element | CLSID | Instance ID | Tract size | Fill at creation | Saved | Meaning |
|---|---|---|---|---|---|---|---|
| aura | int8 | `0x49B9E603` | `0x69D5B600` | 4 | 0 | yes | published result: base + transient (§6.15) |
| base | int8 | `0x49B9E603` | `0x69D5B604` | 4 | −128 | yes | long-term smoothed value; −128 = "no data" |
| transient | int8 | `0x49B9E603` | `0x69D5B605` | 4 | 0 | yes | short-lived effects (§8), decays monthly |
| work | int8 | `0x49B9E603` | `0x69D5B606` | **1** (per cell) | none | no | per-cell scratch of one cycle; −128 = "no lot" |
| building effect | int8 | `0x49B9E603` | `0x49D5B607` | 4 | 0 | yes | cones of property `0xCA5B9305` (§7.3) |
| park map | int16 | `0x49B9E605` | `0x49D5B602` | 4 | none | yes | raw park map × park funding (§6.2); read by developers |
| raw park | int16 | `0x49B9E605` | `0x2A674CA5` | 4 | 0 | yes | cones of property `0x27812850` (§7.1) |
| landmark map | int16 | `0x49B9E605` | `0x8A674CAF` | 4 | 0 | yes | Σ department landmark grids × funding (§6.3); read by developers |
| landmark work | int16 | `0x49B9E605` | `0x8A674CAF` *[quirk: same ID as the landmark map]* | 4 | 0 | no | scratch of §6.3 |
| department landmark grids | int16 | `0x49B9E605` | the department ID | 4 | 0 | yes | one per budget department that owns landmarks (§7.2) |

"Fill at creation: none" means the grid keeps whatever a new grid holds (simgrid §6). `Init` only creates grids
that are still missing, so after a load the restored grids are kept as they are.

The grids form the data flow below (one monthly cycle; arrows mean "is read by"):

```
occupant cones ─> raw park ─(park funding)─> park map ──(park curve)──┐
occupant cones ─> building effect ─────────(added as is)──────────────┤
pollution, school, hospital, crime, trip, congestion ──(curves)───────┤
residential lots (tax, value/wealth) ─> work (per cell) <─────────────┘
work ─(4×4 average, ordinance offset, long decay)─> base ─(+ transient)─> aura ─> mayor rating, hotspots, riot
occupant cones ─> department landmark grids ─(funding)─> landmark map   (read by developers only)
```

The landmark map does **not** feed the aura; only the developers read it.

## 3. State

| Field | Constructor value | Saved | Meaning |
|---|---|---|---|
| reference count | 0 | — | |
| initialized | false | no | set by `Init`, cleared by `Shutdown` |
| rng: `cRZRandom` | seeded from the wall clock (`0xFFFFFFFF`) | no | §9 |
| the grids of §2 | none | see §2 | |
| department map (department ID → landmark grid) | empty | yes | §7.2; iterated in ascending ID |
| state (s32) | 0 | no | pipeline state 0..17 (§6); `Init` sets 1 |
| cycleStartDay (u32) | — | no | `GetSimDateNumber()` when the cycle started (§6.1) |
| departmentCursor (u32) | −1 (none) | no | §6.3 |
| lot iterator | none | no | the safe lot iterator of §6.4/§6.5 |
| mayorRating (s32) | 0 | yes | §6.15 |
| unusedRating (s32) | 0 | yes | set to 0 at the end of every cycle and saved; never read *[inferred]* |
| globalEffect (s32) | 0 | no | ordinance offset `0xAA5B8407` (§5.2, §6.14) |
| previousTaxRate[12] (f32) | −1.0 each | no | baseline tax rates for §6.5.2 |
| longDecay, shortDecay (f32) | 0.6, 0.4 | no | tuning (§10) |
| baseAura[3] (s32: R$, R$$, R$$$) | 0 | no | tuning (§10) |
| effectiveBaseAura[3] (s32) | — | no | base + tax-change effect (§6.5.2); kept in the process-wide table of §10, not in the object |
| hotspotThreshold, hotspotRadius, hotspotMinWeight (s32) | 0, 0, 0 (tuning sets 196, 8, 16 or the shipped values) | no | §10 |
| transient magnitude[4], radius[4] (f32) | 0 | no | per transient effect type (§8.1) |
| curves: park, air, water, garbage, radiation, school, hospital, crime, trip length, congestion, tax change, value/wealth, riot | none | no | §10 |
| maxRating (s16), maxLocation (s32 x, z) | 0, (0, 0) | yes | best tract (§6.15) |
| minRating (s16), minLocation (s32 x, z) | 0, (0, 0) | yes | worst tract |
| low-tract list (tract x, z, 1) | empty | no | §6.15 |
| hotspot list (s32 x, z, weight) | empty | yes | §6.16 |
| riotRoll (f32) | — | no | last random roll (§6.16) |
| reAddLandmarks (bool) | false | no | set by a version-8 load (§11.3) |

## 4. Lifecycle

### 4.1 `Init()` → true *[compat]*

If not initialized:
1. Prepare four empty pollution-curve entries (air, water, garbage, radiation); state := 0; drop any lot iterator.
2. Hold the city (`cISC4City`) and the building-development simulator (held but never used *[inferred]*).
3. Create every missing grid of §2 (CLSID, instance ID, tract size and fill as listed; geometry from the city,
   simgrid §3/§6).
4. state := **1** (a cycle starts at the next day or idle call).
5. Read the tuning (§10).
6. Subscribe on the message server to: occupant inserted `0x99EF1142`, occupant removed `0x99EF1143`, ordinance
   updated `0xC2C09554`, exemplar changed `0xC53D10AA` (foundations §5.2), PostCityInit `0x26D31EC1`.
7. Register its `cIGZMessageTarget2` with the simulator (`AddAgent(target, level, "cSC4AuraSimulator", flags 2)`)
   at levels **0, 10, 4, 6**, in that order (simulator §4). Flags 2: enabled only when the city is established.
8. Publish the `"aura"` node in the game-data registry (§4.3).
9. Set initialized.

### 4.2 `Shutdown()` → true

If initialized: clear initialized; remove the registry node; delete the riot curve; remove the agent from every
level (`RemoveAgent(target)`, simulator §5.6); unsubscribe the five messages and release the message server;
`Shutdown` and release every grid and every department grid, and clear the department map; release the
building-development simulator, the city and the lot iterator; delete all curves. (Unlike the other curves, the
riot curve is deleted before the agents are removed; this has no visible effect.)

### 4.3 The `"aura"` node in the game-data registry *[compat]*

Same mechanism as budget §4.4 (registry service `0xEBE0E860`, IID `0xE898ED03`, slots in foundations §17.7):

| Entry | Call | Arguments |
|---|---|---|
| node | `AddChildEntry(cIGZUnknown* key, name, parent, flags, context)` (slot 5) | key = the aura simulator; `"aura"`; root node; flags 0; context null |
| command | `AddChildEntry(void* key, name, callCallback, parent, flags, context)` (slot 3) | key = the callback's own address; `"add_transient_effect"`; parent = the `"aura"` node; flags 1 (call) *[inferred]*; context = the aura simulator |
| command | same | `"end_strike"`; same |

Callbacks (each returns true):
- `add_transient_effect(x, z, radius, magnitude)`: read input parameters 0..3 as Float32 (`cIGZVariant` slot 37,
  `GetValFloat32(float&)`, a converting read) and call `AddTransientEffect(x, z, radius, magnitude)` (§8.2).
- `end_strike(type)`: read parameter 0 as Uint32 (variant slot 25, `GetValUint32(uint32&)`) and call
  `BustStrike(type)` (§8.4).

The shipped base scripts only declare empty placeholders `sc4game.aura.add_transient_effect(x, z, radius,
magnitude)` and `sc4game.aura.end_strike(department)` (`SimCity_1.dat`, Lua `0xCA63E2A3 / 0x4A5E8EF6 /
0xFFBF9CCF`) and define constants `END_STRIKE_ALL = 0`, `POLICE = 1`, `FIRE = 2`, `HEALTH = 3`, `EDUCATION = 4`
(`0xFFA9DAF4`). No shipped file calls either command and none uses the constants: every decompressed entry of
`SimCity_1.dat` to `SimCity_5.dat` and `EP1.dat` was searched. So the node matters only to mods. *[choice]* Publish
the node when the registry is available; failure is not an error. A stub (no node) behaves the same with the
shipped data.

`Shutdown` removes the node (`RemoveChildEntry(cIGZUnknown*)`, slot 7); its children go with it.

## 5. Agents and messages handled

`DoMessage` returns **true** ("finished", simulator §4) for every message except the late SimNewDay case below.

### 5.1 Calendar agents

| Message | Level | Behaviour |
|---|---|---|
| SimBegin `0x66956810` | 0 | nothing; returns true, so the simulator removes the level-0 registration after the first delivery (simulator §5.3). |
| SimNewDay `0x66956814` | 4 | `DoIdle(5 ms)` (§6), which performs one step. Then, with the state **after** that step: if `state ≠ 0` and `state < GetSimDateNumber() − cycleStartDay` (unsigned compare) → return **false**; else true. |
| SimNewMonth `0x66956816` | 6 | if `state == 0`: state := 1 (start a cycle). True. A month that begins while a cycle is still running does **not** restart it. |
| SimIdle `0x6695681A` | 10 | `DoIdle(data3 ms)` (the time share the simulator offers, simulator §5.4). True (ignored anyway). |

**Cadence *[compat]***: a cycle starts on the first day or idle call after `Init` and after every month change
that finds the pipeline idle. Each `DoIdle` call performs **one** pipeline step (§6) except state 3 (one department
per call) and state 5 (as many lots as the time budget allows). "Late" means: the cycle has run for more whole days
than the state number it has reached. Then the day agent reports unfinished, the simulator keeps calling it in the
same day (each call advancing one step), and the calendar cannot advance until the state number catches up with the
elapsed days or the cycle ends (state 0). In practice idle calls finish a cycle within the first days of a month.
The day-number source is `cISC4Simulator::GetSimDateNumber()` (compiled slot 8).

**Order of step and comparison *[compat]***: the handler (`DoMessage`, `0x0068FA60`) calls `DoIdle` first and
reads the state afterwards. The simulator advances the date **before** it delivers SimNewDay (simulator §5.2 step
8), so on the first delivery of day `D + k` the elapsed value is `k`. Example, cycle started on day D (state 1 ran
on day D), delivery on day D + 5:

| State before the call | Step performed | State compared | `state < 5` | SimNewDay returns |
|---|---|---|---|---|
| 3 (departments left) | one department | 3 | yes | false |
| 3 (every department already processed) | publish | 4 | yes | false |
| 4 (lots exist) | prepare | 5 | no | true |
| 4 (no lots) | prepare | 0 (cycle ends, §6.4) | — | true |
| 6 (park curve present) | parks | 7 | no | true |
| 0 | none | 0 | — | true |

On day D + 6, state 5 with lots left after the 5 ms budget stays 5; `5 < 6` holds, so SimNewDay returns false. A "false" makes the
simulator deliver SimNewDay again in the same day (simulator §5.3), each delivery running one more step.

The day state 1 runs is normally the first of the month: on that day the day agent (level 4) runs before the month
agent (level 6) sets state 1, so state 1 runs on the next `DoIdle` (usually the same day's idle call).

Data read from other simulators in the same cycle (residential population and education/health grids, crime,
pollution, traffic, land value) is whatever they hold at the moment of the step; no ordering with their monthly
work is guaranteed (foundations §13). §5.3 lists which state reads what.

### 5.2 Subscribed messages

| Message | Behaviour |
|---|---|
| PostCityInit `0x26D31EC1` | Only if `reAddLandmarks` (§11.3): for every lot of the lot manager's safe lot iterator whose building's occupant has property `0x2781284F` (landmark): `OccupantChanged(occupant, remove)` then `OccupantChanged(occupant, add)` (§7). The flag is never cleared *[quirk]*; it only matters once per load. |
| occupant inserted `0x99EF1142` / removed `0x99EF1143` | `occupant := void1`. If its property holder **has** property `0x029244DB` (presence test; the preview-occupant marker, foundations §5.2) → ignore. Otherwise `OccupantChanged(occupant, add = (type == 0x99EF1142))` (§7). |
| ordinance updated `0xC2C09554` | `globalEffect := ordinance.GetCumulativeOffset(0xAA5B8407, 0)` (ordinance §6.1; int32). |
| `0xC53D10AA` | if `data1 == 0xE7E2C2DB` and `data2 == 0xAA023079` (the tuning exemplar's group and instance) → re-read the tuning (§10). |

### 5.3 What each state reads, and when *[compat]*

Each `DoIdle` call runs **one** step (§5.1). So each input is read at the moment its state runs, not at the end of
the cycle. The inputs, by state:

| State | Reads | Owner of the data |
|---|---|---|
| 1 | `GetSimDateNumber()` | simulator |
| 2 | Parks funding (`GetDepartmentBudget(0x8A4260A6)`, `GetFundingPercentage`); the raw park grid | budget; own grid, changed by §7 |
| 3 | the department map; one department's landmark grid and its funding per call | own grids (§7); budget |
| 4 | `GetLotCount()`; the 12 `GetTaxRate(t)` (only when the tax curve exists) | lot manager; budget |
| 5 | per lot: `GetBuilding`, `GetPopulations`, `GetBoundingRect`, `GetOccupantWealth`, and `GetLandValueType` at the lot centre | lots (growth, lot developer); land value simulator |
| 6 | the park map (state 2's result) | own |
| 7 | the four pollution grids | pollution simulator |
| 8 / 9 | the school / hospital grade map; for rule R, `GetLot(x, z, false)` and `GetZoneType` per cell | residential simulator; lots |
| 10 | the crime grid | crime simulator |
| 11 / 12 | the trip-length / congestion map; rule R as in 8 | traffic simulator; lots |
| 13 | the building-effect grid | own, changed by §7 |
| 14 | `GetCumulativeOffset(0xAA5B8407, 0)` | ordinance simulator |
| 15 | the base and transient grids; the three population grids (`GetPopulationGrids`) | own; residential simulator |
| 16 | the random generator; the riot manager | own; disaster layer |
| 17 | the transient grid | own |

**When.** State 1 runs at the first idle pass after the month agent (§5.1), normally on the 1st of the month. Every
later step is one `SimIdle` delivery (one per idle pass) or one `SimNewDay` delivery. Between two aura steps, every
other idle agent gets its own call in the same pass: growth developers, building development, commercial, crime,
land value, residential, the prop developer, power, plumbing, neighbours and demand (the order logged in run
r7c3f). When a day boundary falls inside the cycle, that day's day agents also run between two steps. These include
the traffic simulator, which rewrites the trip length of each source it finishes (traffic-simulator.md §4.6) and the
congestion map in its state 4, and pollution's day pass. So the grids read by the later states can be newer than
those read by the earlier ones.

**Occupant changes arrive late.** The occupant inserted and removed messages are **posted** (queued, flora.md §9).
The cones of §7 therefore reach the raw park grid, the department landmark grids and the building-effect grid only
at a later message-server tick (foundations §5). A cycle that runs in the same frame as the insertion uses the
grids from before it in states 2, 3 and 13. The park and landmark maps then catch up one cycle later.

**The end message arrives late too.** `0x2A09B21A` is posted at state 17 (§11.1). It reaches its subscribers at a
later message-server tick, after any occupant messages posted before it. At that moment the inputs can differ from
what the states read. In run r7c3f the cycle that starts on the 1st was delivered on day 1 to day 5 of the month.

**Arithmetic.** State 5's lot value (`0x0068CE00`), the rule A and rule R callbacks (`0x0068BBB0`, `0x0068BDA0`,
`0x0068BE10`, `0x0068BE70`, `0x0068BEE0`), state 14 (`0x0068F6FE`–`0x0068F94E`) and state 15 (`0x0068DA80`) were
checked against the instructions. They have no rounding step beyond §6:
- the lot value is an integer division then an integer clamp, then `trunc(curve)` added and clamped again;
- each rule A stage is one `_ftol` of `work + curve(source)`, summed at 24-bit precision;
- the base is an integer division, `+ globalEffect`, the decay step of §6.14 and one clamp.

A comparison taken at the end message, rather than when each state runs, therefore shows timing differences, not
rounding differences. §14.3 gives a way to read the inputs at the original's own moments.

**Worked examples** (the first in-game aura shadow, run r7c3f, Spandau, 32 × 32 tracts).
- *Park and landmark maps, cycles 2 and 3.* A landfill tile occupant (group `0x07BDDF1C`) carries both a park and a
  landmark effect of {−100, 32}. Its cone has radius 32 / 4 = 8 tracts and covers exactly **193** tracts (simgrid
  §7.1). Two tiles in one tract give 193 tracts with a centre of −200. Two tiles in neighbouring tracts give **208**
  tracts with a maximum of −100 + round(−100 + 1 × 12.5) = **−188**. These match the reported 193 / 200 and 208 /
  188.
  The landfill developer creates tiles in its month agent on the 1st (landfill.md §4.8). Their posted insert
  messages arrive after the original's states 2 and 3 of that cycle, but before the end message.
- *Single tracts near 180.* `|d| = 182` in the base grid with `|d| = 54` in the aura grid means one side holds
  **−128** (no data) and the other holds 54: 54 − (−128) = 182, and the aura of a −128 tract is 0 (§6.15). The same
  holds for 188 / 60 and 175 / 47. In such a tract, one side saw a residential lot with a building and a non-zero
  count, and the other did not. A tract that was −128 takes `clamp(new)` at once (§6.14), so the gap is the whole
  value, not one decay step.
- *Differences of 1 to 3.* A base near its equilibrium has `d` close to 0, and any non-zero `d` moves it by at least
  one (§6.14). So a block average that is one unit off, which one cell's work value can cause, already changes the
  base by 1. A `d` of +1 on one side against −1 on the other changes it by 2.

## 6. The monthly pipeline *[compat]*

`DoIdle(ms)` executes the step of the current state, then moves to the next state as listed. State 0 does nothing.
"Clamp" means clamp to **−127..127** (−128 is reserved for "no data"). `trunc` is truncation toward zero. Curves
are `cSC4ResponseCurve` lookups (foundations §4); a step whose curve is missing does nothing but still advances,
**except state 6** (§6.6). "The curve exists" means the curve **object** exists: the original tests only that, never
the number of points. An **empty** curve (a present property with fewer than 2 values, §10) therefore runs its stage
and is evaluated *[quirk]*, which reads the first segment of empty storage: an access violation (`0x005F4289`) when
the object was created by this tuning read, or the stale first `y` of the previous curve after an in-place reload
(§10). *[choice]* FreeCity4 treats an empty curve exactly like an absent one (the stage is skipped; for the park
curve §6.6's choice applies).

**Integer stores**: every float → integer conversion here is the CRT `_ftol` (`0x009EEF04`): a `fistp` to a
**64-bit** integer in the current rounding mode (round to nearest), then a one-unit correction toward zero when that
rounded away from zero, so the result is truncation toward zero as an int64 in EDX:EAX. NaN, ±∞ and magnitudes
≥ 2^63 give the integer indefinite `0x8000000000000000`, whose low 32 and low 16 bits are **0**. Callers keep only
the low bits they need:
- int16 grid stores (§6.2, §6.3) store the **low 16 bits**: values wrap modulo 65536 (no clamp), NaN → 0 *[compat]*;
- int8 stores with "clamp" (§6.13, rule A) clamp the **low 32 bits** to −127..127, so NaN → 0 *[compat]*.

### 6.1 State 1 — start
If initialized: `cycleStartDay := simulator.GetSimDateNumber()`; → 2. (If not initialized the state stays 1.)

### 6.2 State 2 — park map
`f := *budget.GetDepartmentBudget(0x8A4260A6).GetFundingPercentage()` (department `0x8A4260A6`, Parks and
Recreation, foundations §5.2; funding as a fraction, 1.0 = 100 %).
For every tract: `park := rawPark > 0 ? low16(trunc(float32(rawPark × f))) : rawPark` (one x87 multiply of the
int16 value by the `float32` fraction, rounded to `float32` per foundations §18; then the int16 store of §6: wraps,
NaN → 0). → 3.

*[compat]* The wrap is reachable only when `rawPark × f > 32767` (for example raw 30000 at 120 % funding gives
36000, stored as −29536); shipped cone sizes do not get near it.

**No Parks department**: the original does not test the department pointer. When `GetDepartmentBudget(0x8A4260A6)`
returns null it dereferences null (`0x0068F1E7`, reading the vtable of the null department) and the game crashes.
This cannot happen with the shipped game, because the park manager creates the department at its `Init` when it is
missing (foundations §5.2) and saves restore it. *[choice]* A replacement uses `f := 1.0` when the department (or the
budget simulator) is missing, the same rule state 3 applies to landmark departments (§6.3).

### 6.3 State 3 — landmark map (one department per call)
If the department map is empty → 4. Otherwise process the department after `departmentCursor` in ascending ID
order (the first one when the cursor is −1):

- `f := 1.0` if the department ID is 0 or the budget has no such department; else `*GetFundingPercentage()`.
- scaled value of a tract: `v > 0 ? v × f : v` (float, not truncated yet).
- the **first** department: `landmarkWork := trunc(scaled)`; every later one: `landmarkWork := trunc(landmarkWork +
  scaled)` (the sum is one `float32` add; int16 store of §6: the low 16 bits, so a sum above 32767 wraps, for
  example 20000 + 20000 → −25536 *[compat]*).
- `departmentCursor := ID`. If no department follows: copy landmark work into the landmark map, cursor := −1, → 4.

So N departments take N + 1 calls. If the cursor's department has disappeared from the map, the cursor is reset to
−1 and the state moves to 4 **without** publishing *[quirk]*.

**When the park and landmark maps change *[compat]*.** Only states 2 and 3 write them, once per cycle:
- The park map is rebuilt in **every** state 2. Every tract is written from the raw park grid, so nothing needs
  clearing first.
- The landmark map is replaced in state 3, when the last department is done: landmark work is copied over it. The
  first department overwrites landmark work, so nothing is cleared first either. With an empty department map the
  landmark map is never written, so it keeps what it held (the loaded grid, or 0 from creation).
- A load restores both maps from the save (§11.3). Nothing recomputes them at load or at `Init`.
- Inserting or removing an occupant changes only the raw park grid, a department grid or the building-effect grid
  (§7), and only when the posted message is delivered (§5.3). The park and landmark maps follow at the next state 2
  or 3.

So a park whose insert message is delivered after state 2 (or 3) of a cycle reaches the map one cycle later. §5.3
has the worked example: the landfill tiles created on the 1st are in the maps of the following cycle.

### 6.4 State 4 — prepare
Fill the work grid with −128. If the object has no lot iterator yet, get one from the lot manager (slot 15) and keep
it; otherwise **rewind** the one it has to the first lot (`0x0068F32F`–`0x0068F353`). If `lotManager.GetLotCount()
== 0` → state 0: the cycle ends **without** updating the rating or posting the end message *[quirk]*. Otherwise run
the tax-change computation (§6.5.2) and → 5.

*[compat]* (corrected: the iterator is not fresh each cycle.) One iterator serves every cycle of the object's life.
It stays registered with the lot manager between cycles, including after an empty-city cycle, and is released only
by `Shutdown` (§4.2). Nothing observable depends on this: a registered iterator only follows lot removals, and state
4 rewinds it before use.

### 6.5 State 5 — residential lots (time-sliced)

Uses the CPU-cycle limit timer (simulator §5.5) with a budget of `ms` milliseconds. For each lot from the iterator:
`b := LotBaseAura(lot)` (§6.5.1); for every cell in the lot's bounding rectangle (inclusive, cell units):
`work := max(work, b)` (signed). When the iterator is exhausted → 6; otherwise the state stays 5 and the next call
continues.

**The safe lot iterator (ABI) *[compat]*.** Lot manager slot 15 (`0x00734850`, which moves to the lot list object at
lot manager + 0x5C and continues at `0x00734860`) allocates an 8-byte block from the global heap (allocator C,
`0x005E55E0`): {+0 current list node, +4 the lot list object}. It points it at the first lot and registers it in the
list object's iterator list (list object + 4, node insert `0x0072BA10`), so a lot removal moves it past the removed
lot. The lot list is a doubly linked list of 12-byte nodes {next, prev, `cISC4Lot*`} with a head node; the list
object's first word points to the head. Taking the next lot (`0x0045D3F0`): if the current node is the head, the
walk is over; else the lot is the node's value and current := next. The block is released by a non-virtual
function (`0x0045D5B0`): unregister (`std::list::remove`, `0x00809580`, on list object + 4), then free with C
(`0x005E5620`). docs/abi/allocators.md (table 1, `GetSafeLotIterator`) has the same facts. `GetLotCount` (slot 16,
`0x00809250`) returns the count kept at lot manager + 0x64.

*[choice]* A replacement need not use the iterator. Walking the cells with `GetLot(x, z, false)` and stamping each
lot once gives the same work grid, because stamping takes the maximum and is order-independent (§6.5). Only the
moment each lot is read differs, as with the time slicing.

**Which lots, and where the clamps are *[compat]*.** State 5 is the only state that reads lots for their value. It
takes each lot as the iterator reaches it, between the end of state 4 and the end of state 5. A lot built,
demolished or filled with residents outside that window counts in the next cycle. Rules R (states 8, 9, 11, 12) look
up lots again, but only to choose cells. The clamps come in this order:
1. A lot's value is clamped to −127..127 twice (§6.5.1: after the average, and after the value/wealth term).
2. Stamping takes the maximum of the clamped values, cell by cell.
3. Each of states 6–13 clamps every cell it changes (rule A).
4. State 14 averages the cells of the tract that are not −128, adds `globalEffect` and clamps once more.

A lot's value is never added to the base. When the old base is −128 (no lot until now), the base takes the clamped
new value at once, not one decay step. So a residential lot that one run saw and the other did not, alone in its
tract, makes the whole value differ: base 54 against −128 (difference 182) and aura 54 against 0 (§5.3).

**Time slicing** (shared timer helper: constructor `0x0068CC20`, deadline `0x0068BF50`, check `0x0068BF80`,
adaptation `0x0068B960`):
- Deadline: `TSC now + trunc(float32(ms × MillisecsToCycles))` (simulator §5.5).
- `step` and a signed `miss` counter are a **process-wide static** of the aura code (`.bss` `0x00B47B60` /
  `0x00B47B64`, initialised once per process to 1024 and 0 on first use, guard `0x00B47B68`; referenced only by the
  aura's `DoIdle`). It is not in the object, not saved, not reset by `Init`, `Shutdown` or a load, so it carries
  over between cities. Every other user of the helper (growth developer, land value, plumbing, residential, …)
  passes its **own** static.
- Per call: `count := 0` and a local copy of `step`. For each iteration: `count := count + step` (uint32); if
  `count mod 1024 == 0` and the TSC has passed the deadline → stop (state stays 5; no lot is taken in this
  iteration); else take the next lot (none → state 6, stop) and stamp it. With `step` a power of two in 1..1024 the
  clock is read once every `1024 / step` lots; `step = 1024` reads it before every lot.
- At the end of **every** state-5 call (both exits): `n := count >> 10`, i.e. `n = step × (lots stamped + 1) / 1024`
  (unsigned; the number of clock reads), **not** lots / 1024. Then:
  - `n == 0`: nothing;
  - `n > 20`: `miss += 1`; if `miss ≥ 16`: `miss := 0` and, if `step > 1`, `step := step / 2` (read the clock less
    often);
  - `1 ≤ n < 5`: `miss −= 1`; if `miss ≤ −4`: `miss := 0` and, if `step < 1024`, `step := step × 2`;
  - `5 ≤ n ≤ 20`: nothing.

None of this changes results: the stamping (`max`) is order-independent and the original's slicing already depends
on the CPU clock. *[choice]* FreeCity4 may keep `step` and `miss` per simulator instead of per process (the only
difference is the starting step after a city change). The clock is also free: the original reads the processor's time-stamp counter, but a
replacement may use `QueryPerformanceCounter` or any monotonic clock *[choice]*. The clock only decides how many
lots one call stamps.

#### 6.5.1 `LotBaseAura(lot)`

- No building on the lot → −128.
- Walk the lot's population entries (`cISC4Lot::GetPopulations()`, compiled slot 54: (developer type, count)
  pairs). Residential types: `0x1010` R$ → wealth 0, `0x1020` R$$ → 1, `0x1030` R$$$ → 2.
  `sum += effectiveBaseAura[wealth] × count` for residential entries; `total += count` for **every** entry.
  *[quirk]* Non-residential entries add 0 to the sum but still count in the total, diluting the average.
- If no residential entry was seen, or `total ≤ 0` → −128.
- `avg := clamp(trunc(sum / total))` (integer division).
- If the value/wealth curve exists: `cx := trunc((x1 + x2) / 2)`, `cz` likewise (lot bounding rectangle, cells);
  `d := landValue.GetLandValueType(cx, cz) − lot.GetOccupantWealth()` (both 1..3, so d ∈ −2..2);
  result `:= clamp(avg + trunc(curve(d)))`. Otherwise the result is `avg`.
- **No land value simulator**: the original tests only the curve. It then calls the land value simulator through its
  global pointer with no null check (`0x0068CFB7`), so a city without one crashes at the first residential lot.
  This cannot happen in the shipped game, which always creates it. *[choice]* FreeCity4 skips the value/wealth term
  (result `avg`), as if the curve were absent. Using land value type 0 instead would give `d = −wealth` and bias
  every lot by `+3` to `+5` with the shipped curve.

**The population container *[compat]*.** `GetPopulations()` returns a pointer to the lot's own
`std::vector<std::map<uint32_t, uint16_t>>` (the lot + 0x54, `0x006C21E0`). It is an STLport vector {begin, end,
capacity_end} of 12-byte map objects; each map node is 0x18 bytes with the developer type at +0x10 and the count
(uint16) at +0x14 (docs/abi/allocators.md, *map and set*; allocator B). Saved lots hold 0 or 1 maps (lot-records.md
rows 20–21). The walk (`0x0068CE60`–`0x0068CF23`) takes every map in vector order and every entry in ascending key
order. An entry counts as residential when its key is one of the three types of the base-aura table (§10), **even
when its count is 0**. `sum` and `total` are int32.

`GetPopulation(type)` (slot 53, `0x006C04A0`, pops 4, returns uint16 in AX) looks the type up in each map without
inserting and returns the sum of the counts, wrapped to 16 bits. A loop over the 12 developer types with slot 53
gives the same result as the walk except in three cases:
- a residential type present with count 0: the walk has seen a residential entry, the loop has not (0 and absent
  both read 0);
- a key outside the 12 types: its count is in the walk's `total` only;
- a type whose counts in several maps sum above 65535: the loop wraps.

Saved lots do hold counts of 0 (lot-records.md row 21), so the first case happens in play. Example with effective
R$ = −5: a lot with {R$ 0, CS$ 40} gives `trunc(0 / 40) = 0`, then the value/wealth term (0 when d = 0), so **0**;
the slot 53 loop gives **−128**. A lot with {R$ 30, an unknown type 10} gives `trunc(−150 / 40) = −3` against
`trunc(−150 / 30) = −5`. FreeCity4 reads the container through slot 54.

#### 6.5.2 Tax-change effect (state 4)
`effectiveBaseAura[i] := baseAura[i]` for the three wealths. If the tax-change curve exists, for every tax index
`t` in 0..11:
- if `previousTaxRate[t] == −1.0`: `previousTaxRate[t] := budget.GetTaxRate(t)` (no effect this time);
- else `d := GetTaxRate(t) − previousTaxRate[t]`; if `d ≠ 0`: `e := trunc(curve(d))` and every wealth whose
  tax index equals `t` (R$ → 0, R$$ → 1, R$$$ → 2) gets `effectiveBaseAura := baseAura + e`.

*[quirk]* `previousTaxRate` is set only the first time and is not saved: the baseline is the tax rate seen in the
first cycle after the object was created (new city or load). Indices 3..11 are read but match no wealth.

### 6.6 State 6 — parks
If the park curve exists: combine the work grid with the park map (§6.13 rule A). → 7.
*[quirk]* If the park curve is **missing** the state is not advanced: the pipeline stays in state 6 forever, and
after enough days the SimNewDay agent reports unfinished forever, freezing the calendar. *[choice]* FreeCity4 must
advance to 7 instead (the shipped exemplar always has the curve).

### 6.7 State 7 — pollution
For i = 0..3 (air, water, garbage, radiation) with a curve: rule A with `pollution.GetPollutionGrid(i)` (compiled
slot 5). → 8.

### 6.8 State 8 — schools
If the school curve exists: rule A **restricted to residential cells** (rule R) with the residential simulator's
school grade map (compiled slot 39, `cISC4SimGrid<float>*`). → 9.

### 6.9 State 9 — hospitals
Same with the hospital curve and the hospital grade map (compiled slot 40). → 10.

### 6.10 State 10 — crime
If the crime curve exists: rule A with `crime.GetCrimeGrid()` (uint8). → 11.

### 6.11 State 11 — commute length
If the trip curve exists: rule A + R with `traffic.GetTripLengthMap()` (uint8, compiled slot 13); a source value of
**0 is treated as 255** *[quirk]*. → 12.

### 6.12 State 12 — congestion
If the congestion curve exists: rule A + R with `traffic.GetCongestionMap()` (uint8, compiled slot 10). → 13.

### 6.13 State 13 — building effects, and the combination rules
`work := clamp(work + buildingEffect)` (no curve), rule A. → 14.

**Rule A** (all of states 6–13): only work cells ≠ −128 change. The source grid is mapped onto the per-cell work
grid with simgrid §7.3 (`ProcessTwoGridsWithCallback`; the source is coarser, so each source tract's value applies
to every cell of the tract). New value: `clamp(trunc(work + curve(source)))` with the curve evaluated on the source
value as a float (`clamp(work + source)` without a curve).
**Rule R**: additionally only cells whose lot (`lotManager.GetLot(x, z, false)`) exists and has zone type 1, 2 or
3 (residential low/medium/high, `cISC4Lot::GetZoneType`) change.

### 6.14 State 14 — base grid (long-term smoothing)
`globalEffect := ordinance.GetCumulativeOffset(0xAA5B8407, 0)` again. `m := float32(1.0 − longDecay)`. For every
base tract: average the work cells of its S × S block that are ≠ −128 (`avg := trunc(sum / count)`):
- no such cell → `base := −128`;
- else `new := avg + globalEffect`; if `base == −128` → `base := clamp(new)`; else `d := new − base` and
  `base := clamp(base + step)` with `step = ceil(d × m)` for d > 0, `floor(d × m)` for d < 0, 0 for d = 0.

The `ceil` and `floor` are the 1.5·2²³ idiom, not `_ftol` (`0x0068F881`, `0x0068F8B6`): with `p := f32(d × m)`
and `rhe` as in crime.md §6, `floor(p) = rhe(f32(p − 0.5f))`, plus 1 when `float(that + 1) = p`; `ceil(p) =
rhe(f32(p + 0.5f))`, minus 1 when `float(that − 1) = p`. That is exact here (|p| < 255 ≪ 2²²).

*[compat]* `m` is a 32-bit float: with the shipped `longDecay` 0.9, `m = 0.100000024`, so `d = 10` gives
`d × m = 1.00000024` and a step of **2**, not 1. Any non-zero d moves at least one unit. → 15.

### 6.15 State 15 — aura grid and mayor rating
Get the three residential population grids (`GetPopulationGrids(&r1, &r2, &r3)`, compiled slot 25, uint16 grids;
released afterwards). Clear the low-tract list; `maxRating := −128`, `minRating := 127`. For each tract (x outer, z
inner) of the **aura grid's** counts:

**Population geometry *[compat]***: the population values are read raw (`GetGridData` rows, simgrid §4) at the
**aura tract indices** `(x, z)`, with no bounds check and no coordinate conversion: `r_w := popGrid_w[x][z]`. The
population grids' tract size is P (residential spec, Grids: the R$ growth developer's population-map tract size,
shipped **4**, or the saved value on a load), the aura's is S = 4, and both grids cover the whole city, so with the
shipped data the geometries are identical and the mapping is the identity:

| | Aura tract (x, z) | Population tract used |
|---|---|---|
| tract size | S = 4 | P = 4 |
| cells covered | `[4x, 4x + 3] × [4z, 4z + 3]` | the same cells |
| counts | `cellCountX / 4 × cellCountZ / 4` (64 × 64 in a 256-cell city) | the same |

If P ≠ S (a modded developer exemplar), the original reads population tract `(x, z)` anyway: for P > S it reads past
the end of the population rows (garbage or a crash), for P < S it reads only the low-index part of the population
grid. *[choice]* FreeCity4 uses the population tract that contains the aura tract's centre cell, `(⌊(x·S + S/2) /
P⌋, ⌊(z·S + S/2) / P⌋)`, which is the identity when P = S.

For each tract:
- `base == −128` → `aura := 0`; the tract is skipped for everything else.
- else `a := clamp(base + transient)`; `aura := a`;
  if `a < minRating` → minRating := a, minLocation := (x × S + S/2, z × S + S/2) (cells); if `a > maxRating` → the
  same for the maximum (strict comparisons, so the first tract wins ties);
  if `a < hotspotThreshold` → append (x, z, 1) to the low-tract list;
  `pop := r1 + r2 + r3`; `total += pop`; `weighted += a × pop` (32-bit).
`mayorRating := total > 0 ? trunc(weighted / total) : 0`. Record `history.RecordHistoryData(0x0A5CBF37,
mayorRating)` (compiled slot 12). → 16.

### 6.16 State 16 — hotspots and riots
**Hotspots**: clear the hotspot list; for each low tract in list order: find the first hotspot with
`|hx − x| + |hz − z| < hotspotRadius` (tract units); if found: `hx := trunc((hx × w + x) / (w + 1))`, `hz`
likewise, `w := w + 1`; else append (x, z, 1). Then remove every hotspot with `w < hotspotMinWeight`.

**Riot**: `riotRoll := float32(rng.RandomDoubleRangeUniform(0, 100))` (always drawn). If the hotspot list is not
empty and the riot curve exists: `k := rng.RandomUint32Uniform(n)` (n = number of hotspots), take the k-th hotspot,
`p := riotCurve(float(w))`; if `p > riotRoll`: `city.GetDisasterLayer().GetDisasterManager(0xCA380B06)` (riot) and,
if it exists, `AttemptDisasterStart(hx × S + S/2, hz × S + S/2)` (its result is ignored). → 17.

**Units *[compat]***: hotspot coordinates are tract indices; the riot position is in **cells** (int32 cell indices,
not tracts and not metres), `S` being the aura grid's `GetTractSize()` (4), so hotspot (9, 10) starts a riot at cell
(38, 42). The riot manager (`cSC4DisasterManagerRiot::AttemptDisasterStart`, `0x0057EA10`) reads them as cells: it
returns without effect when its active instance count has reached its limit, otherwise it creates a riot instance
and tries to add a riot leader there: the leader searches outward from that cell (spiral, clamped to
`CellCountX/Z − 1`) for the nearest road tile and converts the found cell to a world position with
`cISC4City::CellCornerToPosition` (compiled slot 96) plus `16.0 × RandomDoubleUniform()` per axis (a point inside
the 16 m cell). The riot's own limits and draws belong to the disaster specs.

### 6.17 State 17 — transient decay and end of cycle
For every transient tract ≠ 0: `transient := trunc(transient × shortDecay)`. Then `unusedRating := 0` and post the
cycle-completed message (§11.1). State := 0.

## 7. Occupant effects *[compat]*

`OccupantChanged(occupant, add)` (from §5.2). `sign := add ? +1 : −1`. Values come from Sint32-array properties
`v` of the occupant's property holder: `v[0]` = magnitude, `v[1]` = radius (no type or length check *[quirk]*, see
"How the values are read" below). The radius is divided by the grid's tract size **in cells** (4), so it acts as a
radius in **cells** (city tiles) *[compat]*, unlike the transient radius (metres, §8.2). Shipped examples
(`SimCity_1.dat`): Big Ben landmark effect {40, 20} (5 tracts), its mayor-rating effect `0xCA5B9305` {7, 256} (64
tracts: the whole of a large city), cemetery {2, 220}, prison {−7, 440}; landfill park and landmark effects
{−100, 32}.

1. **Network piece**: if `add` and the occupant type is `0xC772BF98` (network occupant *[inferred]*):
   `AddTransientEffect(position.x, position.z, 3)` (§8.1; shipped 0/0, so nothing happens).
2. **Centre cell** for parks and landmarks: `city.PositionToCell(position)` (compiled slot 95). If
   `lotManager.GetLot(cell, false)` exists, use the lot's centre instead, per axis with `width = x2 − x1 + 1`:
   odd width → `x1 + width/2`; even width → `x1 + width/2` when the position is in the right half or exactly in the
   middle (distance to the lot's far edge ≤ distance to its near edge, edges at `x1 × CellWidthX` and `(x1 +
   width) × CellWidthX`), else `x1 + width/2 − 1`. Same for z with `CellWidthZ`.
3. Effects (each applies independently when the property is present):

| Property | Target grid | Centre (tract) | Radius (tracts, float) | Centre value | Outer value |
|---|---|---|---|---|---|
| `0x27812850` park | raw park (int16) | centre cell >> tract shift | `v[1] / rawPark.GetTractSize()` | `sign × v[0]` | 0 |
| `0x2781284F` landmark | the department landmark grid (int16) | same | `v[1] / grid.GetTractSize()` | `sign × v[0]` | 0 |
| `0xCA5B9305` building effect | building effect (int8) | `buildingEffect.PositionToTract(position.x, position.z)` (not the lot centre) | `v[1] / buildingEffect.GetTractSize()` | `sign × v[0]` | 0 |

Each is one call of `AddRadiusEffect` (simgrid §7.1; int16 variant for park/landmark, int8 for the building
effect) with arguments (grid data, countX, countZ, tx, tz, radius, centre value, 0).

**How the values are read *[compat]*.** For each property: holder `GetProperty(id)` (slot 9), the property's value
(slot 5), then the variant's `RefSint32()` (slot 105, `+0x1A4`). That slot's body (`0x0060CE80`) is shared by every
`RefX` slot of the variant class (vtable `0x00AA6788`): it returns the word at variant + 4 whatever the type. For an
array that word is the data pointer; for a scalar it is the value itself (`GetValSint32` reads it there,
`0x00917721`). There is no type or count check, so the original does this:

| Property holds | Original |
|---|---|
| Sint32 or Uint32 array of 2 or more values (shipped: Sint32) | `v[0]`, `v[1]` as int32 (a Uint32 bit for bit) |
| an array of 1 value | `v[1]` read past the end of the array: garbage radius |
| an array of another element type | the raw bytes read as int32: meaningless values (Float32 −100.0 reads as −1 027 080 192), and past the end for 8- and 16-bit elements |
| a scalar | the value used as a pointer: a crash for small values, garbage otherwise |

*[choice]* FreeCity4 takes a Sint32 or Uint32 array (bitwise) with at least 2 values; anything else counts as absent
(no cone). This equals the original for every Sint32 or Uint32 array of 2 or more values, the only form that gives
meaningful values.

**Landmark department** (§7.2): `dept := budget.GetBudgetItemForPurpose(holder, 0xAA59670C, &item) ?
item.departmentID : 0`. If the map has no grid for `dept`: on add create one (int16, CLSID `0x49B9E605`, instance
ID = dept, tract size 4, filled with 0) and insert it; on remove skip the landmark cone. *[quirk]* A landmark whose
budget department changed between insert and remove is subtracted from the wrong grid.

## 8. `cISC4AuraSimulator` methods (behaviour)

### 8.1 `AddTransientEffect(float x, float z, uint32 type)`
If `magnitude[type] ≠ 0` and `radius[type] ≠ 0` → `AddTransientEffect(x, z, radius[type], magnitude[type])`. No
bound check on `type` *[quirk]*; FreeCity4 must ignore `type ≥ 4` *[choice]*. Types: 0 fire, 1 flora (tree
planting), 2 civic plop, 3 network plop (names from the tuning, §10).

### 8.2 `AddTransientEffect(float x, float z, float radius, float magnitude)`
`(tx, tz) := transient.PositionToTract(x, z)`; `AddRadiusEffect<int8>(transient, countX, countZ, tx, tz, radius /
transient.GetTractWidthX(), magnitude, 0)` (simgrid §7.1). x, z, radius are world units (metres).

### 8.3 Getters
- `GetAuraValue(x, z)` = `aura.GetCellValue(x, z)` (cell coordinates, simgrid §4).
- `GetMayorRating()` = mayorRating (low 8 bits).
- `GetMaxMayorRating(SC4Point<long>& p)` / `GetMinMayorRating(p)`: write max/min location (cells) to `p`, return the
  rating.
- `GetAuraGrid`, `GetTransientAuraGrid`, `GetParkMap`, `GetLandmarkMap`: the grid pointers, **no AddRef**.

### 8.4 `BustStrike(uint32 type)`
type 1 or 0 → police `EndStrike()`; 2 or 0 → fire protection `EndStrike()`; 3 or 0 → residential
`EndHealthStrike()`; 4 or 0 → residential `EndSchoolStrike()` (in this order). Others: nothing.

## 9. Random numbers

The aura owns one `cRZRandom` (foundations §6.1, §6.2), seeded from the wall clock at construction and **not
saved**. Draws happen only in state 16, once per cycle:
- always one `RandomDoubleRangeUniform(0, 100)` (double path, one `mul` step);
- when the hotspot list is non-empty and the riot curve exists, one `RandomUint32Uniform(n)` (integer path).

## 10. Tuning

Exemplar `0x6534284A / 0xE7E2C2DB / 0xAA023079` "Aura Simulator" (`SimCity_1.dat`), loaded through the resource
manager as a property holder at `Init` and on message `0xC53D10AA` (§5.2). If it is missing, nothing changes.
Curves are rebuilt from float pairs (x, y) (foundations §4); a missing property leaves the curve **absent** (the
stage is skipped, §6). Shipped values are from re/tuning/simulators.md.

**Curve loading *[compat]*** (foundations §4.1, pattern "delete", with two variants):
- The twelve stage curves (`0xEA023270`, `75`, `76`, `77`, `78`, `79`, `7F`, `80` and the four pollution curves
  `71`–`74`) go through one helper (`0x0068D700`): property **present** → if the curve object exists it is
  reloaded **in place**, otherwise a new object is constructed (`0x007D0410`); then the property-pointer loader
  (`0x005F4620`) loads it. Property **missing** → the object is deleted (absent).
- The riot curve `0xEA0232A1` is always deleted first; if the holder has the property a new object is constructed
  and loaded with the key-based loader (`0x005F4700`).
- The loader pairs the values: `n = count / 2` points (unsigned shift). A property with 0 or 1 values (including a
  single scalar) therefore gives a curve object with **no** points, which the original treats as existing (§6:
  crash when the object is new, stale first `y` after an in-place reload). Any non-empty list keeps at least its
  first point (foundations §4.2), so "empty" means exactly "fewer than 2 values". *[choice]* FreeCity4 treats such a
  property like a missing one (absent curve).

| Property | Type | Meaning | Shipped | Code default |
|---|---|---|---|---|
| `0xEA023270` | curve | park map → aura | (0,0) (50,2) (100,3) | none |
| `0xEA023271` / `72` | curve | air / water pollution → aura | (0,10) (200,7) (400,0) (600,−10) (800,−20) (1024,−35) | none |
| `0xEA023273` | curve | garbage → aura | (0,10) (200,7) (400,3) (1800,−4) (2400,−16) (4096,−35) | none |
| `0xEA023274` | curve | radiation → aura | (0,0) (1,−50) | none |
| `0xEA023275` / `76` | curve | school / hospital grade → aura (R cells) | (0,−7) (1,7) | none |
| `0xEA023277` | curve | crime → aura | (0,10) (40,5) (60,2) (80,−3) (100,−5) (255,−50) | none |
| `0xEA023278` / `79` | curve | trip length / congestion → aura (R cells) | (0,7) (255,−7) | none |
| `0xEA02327A` | f32 | longDecay | 0.9 | 0.6 (reset to 0.6 before each read) |
| `0xEA02327B` | f32 | shortDecay | 0.75 | 0.4 (reset before each read) |
| `0xEA02327C` / `7D` / `7E` | s32 | baseAura R$ / R$$ / R$$$ | 0 / 0 / 0 | 0 (kept if absent) |
| `0xEA02327F` | curve | tax-rate change (points) → aura | (−20,25) (−10,20) (−5,10) (−2,7) (−1,5) (1,−5) (2,−7) (5,−10) (10,−20) (20,−25) | none |
| `0xEA023280` | curve | land value type − wealth → aura | (−2,5) (−1,3) (0,0) (1,−3) (2,−5) | none |
| `0xEA023290` | f32[2] | transient fire {magnitude, radius m} | {−20, 128} | {0, 0} (reset before each read) |
| `0xEA023291` | f32[2] | transient flora | {3, 32} | {0, 0} |
| `0xEA023292` | f32[2] | transient civic plop | {25, 128} | {0, 0} |
| `0xEA023293` | f32[2] | transient network plop | {0, 0} | {0, 0} |
| `0xEA0232A0` | s32[3] | {hotspotThreshold, hotspotRadius, hotspotMinWeight} | {−50, 3, 4} | {196, 8, 16}, used unless the property has **exactly 3** values |
| `0xEA0232A1` | curve | hotspot weight → riot chance (%) | (0,0) (5,10) (10,60) (20,100) | none (deleted before each read) |

After reading, the three base aura values are copied into a process-wide table keyed by developer type (`.data`
`0x00B0D150`; see below). The transient arrays: magnitude := `v[0]`, radius := `v[1]`. They are shipped as f32[2] and copied **raw** into a
float vector (`0x005FFCC0`, at `0x0068D94E`) with no type check or conversion (corrected: not s32 converted to
float; a property stored with another type would be reinterpreted bit for bit, data-formats.md §5).

**The base-aura table *[compat]*.** A static table of three 20-byte entries, ended by an entry whose developer type
is 0:

| Entry | Developer type | Tax index | Wealth | Tuned base | Effective base |
|---|---|---|---|---|---|
| +0x00 | `0x1010` R$ | 0 | 0 | +0x0C | +0x10 |
| +0x14 | `0x1020` R$$ | 1 | 1 | +0x20 | +0x24 |
| +0x28 | `0x1030` R$$$ | 2 | 2 | +0x34 | +0x38 |

The tuning read copies `baseAura` R$, R$$, R$$$ into the entries' tuned bases (`0x0068D8E2`–`0x0068D91B`). State 4 copies
tuned into effective and adds the tax-change effect by tax index (`0x0068C000`, §6.5.2). `LotBaseAura` finds a
population key in the table and reads the effective base (§6.5.1). No other code refers to the table: every
instruction in the executable that names an address inside it is in one of those three places. Since the table is per process, the effective values
survive a city change, but state 4 always rewrites them before state 5 reads them. So nothing observable depends
on where they are kept, and a replacement may keep them in its object *[choice]*.

**Transient tuning in detail *[compat]*.** For each of the four properties, magnitude and radius are first set to 0
and a local float vector is emptied. The copy helper (`0x005FFCC0`) returns false only when the property is missing.
Otherwise it sizes the vector to the variant's count (slot 4) and copies that many raw 32-bit words from the
variant's data (slot 111), and returns **true** even when the count is 0. The aura then reads `v[0]` and `v[1]`
with no size check (`0x0068D958`–`0x0068D966`):

| Property holds | Original |
|---|---|
| a Float32 array of 2 or more values (shipped) | magnitude `v[0]`, radius `v[1]`; extra values ignored |
| a Float32 array of 1 value | magnitude `v[0]`; radius read past the end of the one-value block (garbage) |
| a scalar (count 0) or an empty array | the vector is empty, so the read goes through a null pointer: **crash** |
| a 32-bit integer array | the integer bits taken as floats (for example −20 becomes a NaN, 128 a denormal near 1.8e−43) |
| an array of 8-, 16- or 64-bit values | `count` 32-bit words copied from a smaller (or larger) buffer: garbage |

*[choice]* FreeCity4 uses `v[0]` and `v[1]` only for a Float32 array of at least 2 values; in every other case the
type keeps {0, 0}, as if the property were missing. Reading the shipped values through the property's Float32 values
gives exactly the raw copy's numbers.

## 11. Messages posted and persistence

### 11.1 Posted

| Message | When | Fields |
|---|---|---|
| aura cycle completed `0x2A09B21A` | end of every complete cycle (state 17) | void1 = the aura simulator (primary pointer); IGZUnknown = the same (counted). Posted with `MessagePost(msg, false)` (queued). The map view subscribes (it refreshes its aura layer). |

### 11.2 Save record (`cIGZSerializable`, `GetGZCLSID() = 0x0990BDD8`) *[compat]*

```
u16  version = 9
ref  aura grid                       (refs: foundations §2.7; each grid is its own object, simgrid §5)
ref  park map
ref  raw park
ref  base grid
ref  transient grid
ref  building effect grid
ref  landmark map
u32  departmentCount
departmentCount × { u32 departmentID; ref grid }        (ascending ID)
s32  unusedRating
s32  mayorRating
s32  maxLocation.x, s32 maxLocation.z, s16 maxRating
s32  minLocation.x, s32 minLocation.z, s16 minRating
u32  hotspotCount
hotspotCount × { s32 x, s32 z, s32 weight }              (list order)
```

`Write` returns `GetError() == 0`.

### 11.3 Read
- `version < 8` → return false. There is **no upper limit**: any version ≥ 9 is read as 9 *[quirk]*. *[choice]*
  FreeCity4 rejects versions above 9.
- The seven grid references are resolved with IID `0xC9BD939A` (`cISC4SimGrid`); a null reference leaves the grid
  missing and `Init` recreates it.
- Version 8: after the landmark map comes **one** extra reference (the old single landmark grid). Read it, then
  `Shutdown` and release it; set `reAddLandmarks` (§5.2), so all landmarks are stamped again into department grids
  at PostCityInit. No department block.
- Version ≥ 9: the department block.
- Then the remaining fields; return `GetError() == 0`.

What `Read` (`0x0068E340`) does with state already in the object *[compat]*:

- **Grid references** (the seven grids, and each department grid): the reference helper sets the destination to
  null first and then stores the resolved grid; the grid it held before is **not released** (foundations §2.7
  item 3). A failed reference leaves null.
- **Department map: not cleared, merged.** For each saved entry, `map[departmentID] := grid` (map `operator[]`
  `0x00497A00`, called at `0x0068E45F`): a new ID is inserted; an ID already present has its grid pointer
  **replaced** without releasing the old grid (a leak); entries whose IDs are not in the record stay untouched. A
  version-8 record does not touch the map.
- **Hotspot list: appended.** Each saved hotspot is appended at the end of the current list in record order (a node
  linked in before the list's end, `0x0068E548`–`0x0068E552`); the list is not cleared first, so hotspots already
  held stay in front (like flammability §10's hotspot read).
- **Scalars** (`unusedRating`, `mayorRating`, the max/min locations and ratings) are overwritten.
- The game calls `Read` on the object the class store has just created for the load (foundations §2.7), whose
  map and list are empty, so merge/append and replace give the same result in play *[inferred: no other caller
  of this `Read` slot is known]*. A replacement must merge and append, which is what a `Read` on a used object
  observes. `Read` touches no other state (the low-tract list, cursors, tuning and the grids' contents are kept).
- *[quirk]* Both counts (`departmentCount`, `hotspotCount`) are read into uninitialised locals, so a record that
  fails before a count loops over a garbage count (each pass with failed reads: a department entry with a
  leftover ID and a null grid, or a hotspot node with leftover fields). *[choice]* FreeCity4 treats a failed
  count as 0.
- *[quirk]* Version 8: the old landmark grid reference is used without a null check (`Shutdown`, then `Release`);
  a null or failed reference crashes the original. *[choice]* FreeCity4 skips the shutdown and release for null.

Not saved, rebuilt by the next cycle: work and landmark-work grids, state (set to 1 by `Init`), cycle start,
department cursor, previous tax rates, effective base auras, globalEffect (recomputed in state 14), the low-tract
list, the RNG and all tuning.

## 12. Compiled slot tables *[compat]* (see foundations §16, §17)

### 12.1 `cISC4AuraSimulator`, original vtable `0x00AB07C8` (16 interface slots + destructor)

| Slot | Offset | Signature as the game expects it | Callee pops | Return | Address |
|---|---|---|---|---|---|
| 0 | `+0x00` | `bool QueryInterface(uint32 iid, void** out)` | 8 | EAX (bool) | `0x0068B9C0` |
| 1 | `+0x04` | `uint32 AddRef()` | 0 | EAX | `0x004F7BA0` (shared) |
| 2 | `+0x08` | `uint32 Release()` | 0 | EAX | `0x0068BFB0` |
| 3 | `+0x0C` | `bool Init()` | 0 | EAX, always true | `0x0068E6B0` |
| 4 | `+0x10` | `bool Shutdown()` | 0 | EAX, always true | `0x0068DEC0` |
| 5 | `+0x14` | `int8 GetAuraValue(int32 x, int32 z)` (cells) | 8 | AL | `0x0068BAB0` |
| 6 | `+0x18` | `void AddTransientEffect(float x, float z, uint32 type)` | 12 | none | `0x0068BAD0` |
| 7 | `+0x1C` | `void AddTransientEffect(float x, float z, float radius, float magnitude)` | 16 | none (nothing left in ST0) | `0x0068BB30` |
| 8 | `+0x20` | `int8 GetMayorRating() const` | 0 | AL | `0x0068BAC0` |
| 9 | `+0x24` | `int8 GetMaxMayorRating(SC4Point<long>& location) const` | 4 | AL | `0x0068D1E0` |
| 10 | `+0x28` | `int8 GetMinMayorRating(SC4Point<long>& location) const` | 4 | AL | `0x0068D200` |
| 11 | `+0x2C` | `void BustStrike(uint32 type)` | 4 | none | `0x0068C0D0` |
| 12 | `+0x30` | `cISC4SimGrid<int8>* GetAuraGrid()` | 0 | EAX pointer | `0x0068D1B0` (body shared with an unrelated getter) |
| 13 | `+0x34` | `cISC4SimGrid<int8>* GetTransientAuraGrid()` | 0 | EAX pointer | `0x0068D1C0` |
| 14 | `+0x38` | `cISC4SimGrid<int16>* GetParkMap() const` | 0 | EAX pointer | `0x0040D390` (body shared with flammability's `GetFlammabilityGrid`) |
| 15 | `+0x3C` | `cISC4SimGrid<int16>* GetLandmarkMap() const` | 0 | EAX pointer | `0x0068D1D0` |
| 16 | `+0x40` | class destructor (scalar deleting, flag argument) | 4 | EAX | `0x0068D490` |

**SDK comparison** (`cISC4AuraSimulator.h`):
- **Overloads (slots 6/7)**: the SDK declares the 4-float overload first and the `(float, float, uint32)` overload
  second. MSVC reverses adjacent overload groups (foundations §16), which yields exactly the original layout (slot 6
  = 3-argument, slot 7 = 4-float). The SDK header is therefore **correct for MSVC**. A compiler that does not
  reverse overloads (GCC/Clang, or a Rust vtable written in source order) must place them as in the table above.
- **Parameter meaning (SDK naming error, not an ABI error)**: the 4-float overload's parameters
  `(unknown1, param_2, param_3, param_4)` are **(x, z, radius in metres, centre magnitude)**; the 3-argument
  overload's are `(x, z, type)` where `type` is 0..3 (§8.1), not an enum in the SDK.
- Return types checked against the epilogues: every getter returns in EAX/AL; no float, int64 or by-value struct
  return exists in this interface. `GetAuraValue`, `GetMayorRating`, `GetMax/MinMayorRating` return int8 in AL
  (callers sign-extend AL). The three `void` slots leave EAX undefined (no meaningful value); a replacement may
  return anything there. `eStrikeBuster` values match §8.4.
- No other SDK error was found in this interface.

### 12.2 `cIGZSerializable`, vtable `0x00AB07A0` (sub-object)

| Slot | Offset | Method | Pops | Return |
|---|---|---|---|---|
| 0–2 | | QueryInterface / AddRef / Release thunks to the primary object | 8 / 0 / 0 | EAX |
| 3 | `+0x0C` | `bool Write(cIGZOStream&)` (`0x0068CC50`) | 4 | EAX |
| 4 | `+0x10` | `bool Read(cIGZIStream&)` (`0x0068E340`) | 4 | EAX |
| 5 | `+0x14` | `uint32 GetGZCLSID()` → `0x0990BDD8` (`0x0068D1A0`) | 0 | EAX |

### 12.3 `cIGZMessageTarget2`, vtable `0x00AB07B8` (sub-object)

| Slot | Offset | Method | Pops | Return |
|---|---|---|---|---|
| 0–2 | | thunks to the primary object | 8 / 0 / 0 | EAX |
| 3 | `+0x0C` | `bool DoMessage(cIGZMessage2*)` (`0x0068FA60`) | 4 | EAX (bool; false only in §5.1) |

The class registers with the simulator and the message server through this sub-object.

## 13. Dependencies (methods the original calls on other subsystems) *[compat]*

All are `__thiscall`; "pops" = callee-pop bytes.

| Interface (vtable used) | Slot / offset | Method | Pops | Return |
|---|---|---|---|---|
| `cISC4City` (`0x00A87620`) | 51 `+0xCC` | `GetAuraSimulator()` (callers) | 0 | EAX |
| | 72 `+0x120` | `GetDisasterLayer()` | 0 | EAX |
| | 91 / 92 `+0x16C/+0x170` | `CellWidthX()` / `CellWidthZ()` | 0 | **ST0** float |
| | 95 `+0x17C` | `PositionToCell(float x, float z, int32& cx, int32& cz)` | 16 | EAX |
| `cISC4Simulator` (`0x00AB2FB8`) | 8 `+0x20` | `GetSimDateNumber()` | 0 | EAX |
| | 19 `+0x4C` | `AddAgent(target, level, name, flags)` | 16 | EAX |
| | 20 `+0x50` | `RemoveAgent(target)` | 4 | EAX |
| `cISC4BudgetSimulator` (`0x00AB0908`) | 28 `+0x70` | `GetDepartmentBudget(uint32)` | 4 | EAX |
| | 36 `+0x90` | `GetTaxRate(int32)` | 4 | **ST0** float |
| | 51 `+0xCC` | `GetBudgetItemForPurpose(holder, purpose, item&)` | 12 | EAX |
| `cISC4DepartmentBudget` (`0x00AB0EB0`) | 10 `+0x28` | `GetFundingPercentage()` → **pointer** to f32 (budget §7) | 0 | EAX |
| `cISC4OrdinanceSimulator` (`0x00ABEAC0`) | 24 `+0x60` | `GetCumulativeOffset(uint32 property, uint32)` → int32 | 8 | EAX |
| `cISC4LotManager` (`0x00AB37C0`) | 6 `+0x18` | `GetLot(int32 x, int32 z, bool)` | 12 | EAX |
| | 15 `+0x3C` | `GetSafeLotIterator()` | 0 | EAX |
| | 16 `+0x40` | `GetLotCount()` | 0 | EAX |
| `cISC4Lot` (`0x00AB19B0`) | 14 `+0x38` | `GetBoundingRect(SC4Rect<long>&)` | 4 | EAX |
| | 31 `+0x7C` | `GetZoneType()` | 0 | EAX |
| | 41 `+0xA4` | `GetOccupantWealth()` | 0 | EAX |
| | 48 `+0xC0` | `GetBuilding()` | 0 | EAX |
| | 54 `+0xD8` | `GetPopulations()` → pointer to the lot's `vector<map<uint32, uint16>>` (lot + 0x54, §6.5.1) | 0 | EAX |
| `cISC4LandValueSimulator` (`0x00AB1670`) | 5 `+0x14` | `GetLandValueType(int32 x, int32 z)` | 8 | EAX |
| `cISC4CrimeSimulator` (`0x00AB0DC8`) | 5 `+0x14` | `GetCrimeGrid()` (uint8) | 0 | EAX |
| `cISC4PollutionSimulator` (`0x00AB2630`) | 5 `+0x14` | `GetPollutionGrid(int32 type)` | 4 | EAX |
| `cISC4ResidentialSimulator` (`0x00AB2C10`) | 8 `+0x20` | `EndSchoolStrike()` | 0 | EAX |
| | 9 `+0x24` | `EndHealthStrike()` | 0 | EAX |
| | 25 `+0x64` | `GetPopulationGrids(grid*&, grid*&, grid*&)` (uint16) | 12 | EAX |
| | 39 `+0x9C` | school grade map → `cISC4SimGrid<float>*` | 0 | EAX |
| | 40 `+0xA0` | hospital grade map → `cISC4SimGrid<float>*` | 0 | EAX |
| `cISC4TrafficSimulator` (`0x00AB33D0`) | 10 `+0x28` | `GetCongestionMap()` (uint8) | 0 | EAX |
| | 13 `+0x34` | `GetTripLengthMap()` (uint8) | 0 | EAX |
| `cISC4PoliceSimulator` (`0x00AB2490`) | 13 `+0x34` | `EndStrike()` | 0 | EAX |
| `cISC4FireProtectionSimulator` (`0x00AB1160`) | 14 `+0x38` | `EndStrike()` | 0 | EAX |
| `cISC4DisasterLayer` (`0x00A9E390`) | 7 `+0x1C` | `GetDisasterManager(uint32 type)` | 4 | EAX |
| `cISC4DisasterManager` (riot, `0x00A9E798`) | 6 `+0x18` | `AttemptDisasterStart(int32 x, int32 z)` (MSVC-reversed overload, as in the SDK) | 8 | EAX (ignored) |
| `cISC4HistoryWarehouse` (`0x00A8CCB4`) | 12 `+0x30` | `RecordHistoryData(uint32 id, int32 value)` *[inferred name]* | 8 | — |
| `cISC4Occupant` | 3 / 6 / 7 / 8 | (building) `GetOccupant`, `AsPropertyHolder`, `GetType`, `GetPosition` *[inferred names]* | — | EAX |
| `cISCPropertyHolder` | 3 `+0x0C` / 9 `+0x24` | `HasProperty(id)` / `GetProperty(id)` (foundations §17.2) | 4 | EAX |
| `cIGZVariant` | 105 `+0x1A4` | `RefSint32()` (array pointer); 25 / 37 in §4.3 (foundations §17.1) | 0 | EAX |
| grids (`cISC4SimGrid<T>`, simgrid §4) | 6, 7, 9–14, 19, 24/25, 27 | `SetInstanceID`, `GetCellValue`, tract size/shift, counts, widths, `PositionToTract`, grid data | | |

**SDK errors in dependencies** (for the owners of those specs): residential `GetPopulationGrids` — all three
parameters are outputs (`cISC4SimGrid<uint16>*&`), the SDK declares only the first as a reference; residential slots
39/40 (SDK `int32 GetSchoolAverageGradeMap()` / `GetHospitalAverageGradeMap()`) return `cISC4SimGrid<float>*`
(same register, so callers can cast). Ordinance slot 24 `GetCumulativeOffset` (`0x0080DD70`): the SDK declares an int64
return, but the original sums int32 values into EAX and returns with `ret 8`. EDX is never set for the caller (it
holds whatever the last call left there), so a caller declared with int64 reads a garbage high half. Declare it
int32.

## 14. Interactions and replaceability

### 14.1 Consumers (all through the virtual interface, via `GetAuraSimulator()`)

| Consumer | Uses |
|---|---|
| City advisors (city planning) | `GetMayorRating`, `GetMaxMayorRating`, `GetMinMayorRating` → Lua `game` variables (advisors.md §7.3); the Lua reward conditions read the rating and set conditional-building status (advisors.md §6.6) |
| Regional city cache (`cSC4RegionalCity::UpdateCityCache`) | `GetMayorRating`, stored as a signed byte in the regional-city record (foundations §11) |
| 3D view (`UpdateMayorRatingBar`, `DoPostCityInit`) | `GetMayorRating` (the latter uses rating + 100) |
| Query tool | `GetAuraValue` (lot average of cells, mayor-rating percent), `GetMayorRating`; park/landmark/transient maps for its desirability readout |
| Growth developer, tract developer | `GetParkMap`, `GetLandmarkMap`, `GetTransientAuraGrid` → `GetCellValue` (desirability) |
| Map view | `GetAuraGrid` and `GetParkMap` → `GetGridData` (raw rows, simgrid §4) for its overlays; subscribes to `0x2A09B21A` |
| MySim / My Street sim | `GetAuraGrid` (trip variables) |
| Fire disaster (ignite) | `AddTransientEffect(x, z, 0)` |
| Flora planting tool, a terrain tool | `AddTransientEffect(x, z, 1)` |
| A lot-placement routine *[inferred: civic plop]* | `AddTransientEffect(x, z, 2)` |
| Mayor's limo | `AddTransientEffect(x, z, 48.0, 10.0)` |
| City situation manager | `AddTransientEffect(x, z, radius, magnitude)` with the situation's success/failure aura |
| 3D view, history UI | history series `0x0A5CBF37` |
| Lua (`sc4game.aura`) | §4.3 |

### 14.2 Replaceability findings
- Created only through GZCOM (class store on load, `GetClassObject` in the city's `Init`); no original code
  constructs it or calls one of its methods directly. The non-virtual helpers located near its code (the CPU-cycle
  timer start/check, the registry helper, the int16 two-grid helper, the time-slice helper) are shared library code
  called by many subsystems; they do not touch the aura object. **Replaceable** through the class registry.
- Consumers read the four grids it hands out, including **raw grid memory** (map view `GetGridData`). A replacement
  must return genuine `cISC4SimGrid<int8>` / `<int16>` objects (simgrid §2, or the game's own grid class) with the
  tract size and geometry of §2, valid until `Shutdown`, and must not hand out new references (callers do not
  release them).
- Its own save record references its grids (simgrid §5): a replacement that writes the same record must save its
  grids as `cSC4SimGrid` objects so the original can load them.
- `GetParkMap`/`GetLandmarkMap` are inputs to the growth and tract developers' desirability: their scale (park cone
  magnitudes × funding fraction) must match.

### 14.3 Running beside the original (shadow comparison) *[choice]*

A shadow that runs a whole cycle when `0x2A09B21A` arrives reads every input later than the original did (§5.3). It
therefore shows timing differences, which hide real ones. Tracking the original **step by step** aligns pipeline
reads. Matching occupant inputs additionally requires the recipient boundary in §14.3.1:

- After each delivery of SimIdle or SimNewDay to the original's agent, read the original's **state** (s32). When it
  has moved from s to s′, run our steps s, s+1, …, s′−1 at once, before any other agent runs. If the original went
  back to 0 from state 4 (empty lot list) or from state 17, run up to the end of the cycle.
- State 5 is spread over several calls. The simplest match is to run our whole state 5 in the call where the original
  leaves it. A lot added or removed between two of the original's slices can still differ; the work grid comparison
  below shows such lots.
- Apply occupant inserted and removed to our raw park, department and building-effect grids using the inputs
  read by the original recipient (§14.3.1). Include the direct PostCityInit rebuild calls; a separate subscriber
  to insert/remove broadcasts alone is insufficient.
- For a per-step diagnostic, compare our work grid with the original's after each step from 5 to 13. The original's
  work grid keeps its values from state 13 until the next state 4, so at the end message it still holds the inputs
  of state 14.
- Tax baseline: the original takes `previousTaxRate` in the first state 4 after its object was created, so in the
  first cycle after a load (§6.5.2). A shadow that starts at the first cycle after the load and reads the tax rates
  in our state 4 has the same baseline; one that starts later, or reads them at the end message, can differ.

The original object, as offsets from its `cISC4AuraSimulator` pointer (constructor `0x0068D110`, `Init`
`0x0068E6B0`): `cIGZMessageTarget2` sub-object +0x04 (the agent registered with the simulator), `cIGZSerializable`
+0x08; grids: aura +0x40, work +0x44, base +0x48, transient +0x4C, building effect +0x50, raw park +0x54, park map
+0x58, department map +0x5C, landmark map +0x68, landmark work +0x6C; state (s32) +0x70; cycleStartDay +0x74; lot
iterator +0x78; departmentCursor +0x7C; mayorRating +0x84; globalEffect +0x88; previousTaxRate[12] (f32) +0x94.

#### 14.3.1 Live occupant inputs and direct cone paths (AU-GC29-1) *[compat]*

An occupant message contains a **live pointer**, not a snapshot of its property holder, position or lot.
The original aura recipient obtains `void1`, calls the occupant's property-holder getter and tests **presence**
of preview marker `0x029244DB` at delivery time (`0x0068FBEB`–`0x0068FC09`).
The marker's value is irrelevant. If absent, it calls `DoOccupantChanged` (`0x0068ECF0`) at `0x0068FC19`.
That helper reads the live effect properties and position and performs a live `GetLot(cell, false)` lookup
(`0x0068ED31`–`0x0068EDAE`). The message does not retain an earlier lot association or effect values.
There is no guarantee that another subscriber's reads occur with the same values, even for the same message
and occupant pointer. A later read is equivalent only when these inputs have remained unchanged.

**Native observation boundary.** Use the original `DoMessage` (`0x0068FA60`), reached through slot 3 of
its `cIGZMessageTarget2` pointer (primary aura interface +`0x04`), to capture the preview test as it is made.
Also observe every original `DoOccupantChanged` entry, whose `this` is the primary aura pointer and whose
arguments are occupant and add/remove. This is a native recipient/helper boundary, **not** an additional
public observer API: no aura slot supplies an immutable occupant-event record. Preserve each original call
and its order. For inputs read within the helper, retain the results of the original getter/lookup calls
as they occur, rather than dereferencing the occupant again after the helper returns:

| Input | What must be retained |
|---|---|
| Broadcast gate | insert/remove type, occupant identity, marker presence result; an ignored event has no helper call |
| Occupant | type ID; the float32 position components actually returned for each use (network transient, park/landmark and building-effect paths) |
| Effect holder | presence and the two raw int32 elements for each of park `0x27812850`, landmark `0x2781284F`, building effect `0xCA5B9305`; apply §7's malformed-value choice separately |
| Park/landmark centre | position-to-cell inputs/results, whether `GetLot(cell, false)` found a lot, its cell bounds and the city cell widths used by §7's even-width rule |
| Landmark routing | result of `GetBudgetItemForPurpose(holder, 0xAA59670C, &item)` and the returned department ID on success; failure routes to 0 |
| Grid geometry | actual tract sizes/counts for the three target kinds; original department-grid presence before a removal |

These are service inputs to the cone rules, not the original's computed cone output. Capturing them at a
separate subscriber is a diagnostic only until compared with the recipient's actual reads. Do not defer
the lookup until the cycle-completed message or infer it from the final published park/landmark maps.

**Paths without occupant broadcasts.** The exported direct callers of `DoOccupantChanged` are exactly the
ordinary message call above and two calls in the PostCityInit landmark rebuild, at `0x0068FB1F` (remove) and
`0x0068FB30` (add). The rebuild (§5.2, §11.3) invokes the helper directly for each qualifying landmark;
it does not post an insert/remove message per lot, and does not pass through the preview-marker test.
The helper is not a public aura-interface slot. Separately, public `AddTransientEffect` slots 6/7 (§8)
write the transient grid synchronously without occupant broadcasts; the network-add branch of the helper
also calls slot 6 with type 3 (`0x0068ED04`–`0x0068ED2E`). Those calls affect the transient grid, not the
three permanent cone grids. Loading grid references (§11.3) is another source of initial grid state,
not a sequence of cone notifications; seed a shadow from the same boundary before following events.

**Worked cases** (isolated effect, zero initial grids, tract size 4):

| Original-recipient inputs | Expected result and discriminator |
|---|---|
| Insert, park effect {100, 4}, preview marker present with value 0 | ignored; raw park stays zero. A subscriber reading after marker removal would incorrectly stamp a centre value of 100 |
| Park effect {100, 4}, position at cell (16,16), lot bounds [16,24] on both axes at insert and remove | odd-width lot centre (20,20), tract (5,5), radius 1; add then remove cancels exactly. If a later subscriber sees no lot at removal, it subtracts at tract (4,4), leaving +100 at (5,5) and −100 at (4,4) |
| PostCityInit with reAddLandmarks true, landmark effect {40,20}, no grid for its department | remove skips the missing department grid, then add creates it and stamps centre 40. A follower listening only for occupant broadcasts sees neither call |

**Observation still needed for the grown-Lua first cycle.** Capture the above records at both boundaries
for the first plop/demolish sequence. Compare initial raw park (+`0x54`), the ordered department map
(+`0x5C`, each ID and grid), and building effect (+`0x50`) before events; then compare raw park immediately
before state 2, each consumed department before its state-3 call, park map (+`0x58`) before state 6, and
building effect before state 13. Retain funding inputs and the state/department cursor with these snapshots.
A mismatch before the consuming step identifies seed/event-input drift; matching source grids followed
by differing publication identifies pipeline timing or arithmetic instead. State-5 agreement alone does
not test the park source consumed in state 6. Static analysis does not identify which event or grid differed
in either reported grown run. Their first-cycle failure and exact later cycles remain distinct evidence;
the two exact empty-Lua/reach29-views cycles do not validate those grown first cycles.

## 15. Original quirks (reproduced)

Non-residential population dilutes the lot average (§6.5.1); the tax-change baseline is never updated (§6.5.2);
empty-city cycles end silently (§6.4); a missing landmark-department cursor skips publishing (§6.3); trip length 0 is
treated as 255 (§6.11); float rounding of `1 − longDecay` makes steps larger (§6.14); `reAddLandmarks` is never
cleared (§5.2); the landmark-work grid reuses the landmark map's instance ID (§2); Read accepts any version ≥ 8
(§11.3, FreeCity4 rejects > 9 *[choice]*); no bound check on transient type (§8.1, FreeCity4 ignores ≥ 4
*[choice]*); a missing park curve stalls the pipeline (§6.6, FreeCity4 advances *[choice]*); int16 park and landmark
stores wrap modulo 65536 (§6, §6.2, §6.3); the time-slice step is a process-wide static (§6.5). Not reproduced
(crashes or garbage in the original): a missing Parks department (§6.2, FreeCity4 uses 1.0 *[choice]*), an empty
curve (§6, §10, FreeCity4 treats it as absent *[choice]*), population grids with a tract size other than 4 (§6.15,
FreeCity4 maps by centre cell *[choice]*).

## 16. Open questions

1. *(resolved)* Property `0x029244DB` marks preview occupants (foundations §5.2).
2. *(resolved)* Message `0xC53D10AA` is the "exemplar changed" notification posted by the exemplar resource code
   (foundations §5.2).
3. *(resolved)* Department `0x8A4260A6` is Parks and Recreation (foundations §5.2); budget §12 is corrected.
4. *(resolved)* The residential population grids take the R$ growth developer's tract size, shipped 4, the same as
   the aura's S (residential.md, Grids). The aura reads them raw at its own tract indices; the mapping, and the
   behaviour when the sizes differ, are in §6.15.
5. *(resolved)* The time-slice helper's code (`0x0068CC20`, `0x0068BF80`, `0x0068B960`) is shared, but each caller
   passes its own static step; the aura's static and the exact constants are in §6.5. (simulator.md §5.5 may cite
   §6.5 for the helper's adaptation rule.)
6. *(resolved)* The riot start position is in **cells**, and the riot manager reads it as cells (§6.16), unlike the
   earthquake's `AttemptDisasterStart`, which takes world units (budget errata item 6).
7. *(resolved)* The process-wide base-aura table (`0x00B0D150`) is read and written only by the aura's own code; it
   holds the tuned and effective base auras (§10).
8. Whether the 0x99EF1142 handler's "has property" test also applies to network occupants (the network transient is
   reached only when the marker is absent).
9. Occupant effect radii are divided by the tract size in cells (§7), the transient radius by the tract width in
   metres (§8.2). The behaviour is certain; whether the shipped mayor-rating radii (110–440) were meant as metres
   (which would make them 16 times smaller) is a design question only. FreeCity4 reproduces the cell reading.
10. *(resolved)* SimNewDay compares the state **after** its `DoIdle` step (§5.1, test 14).
11. *(resolved)* An empty curve is evaluated by the original (crash or stale value); FreeCity4 treats it as absent
    (§6, §10). int16 stores wrap and NaN stores 0 (§6). A missing Parks department crashes the original; FreeCity4
    uses funding 1.0 (§6.2).
12. *(resolved)* `Read` merges into the department map (new IDs inserted, existing IDs' grid pointers replaced
    without release, other entries kept) and appends to the hotspot list; it clears neither (§11.3).

## 17. Conformance tests (must exist in `engine/conformance`)

Shipped tuning unless stated; S = 4; curves per foundations §4.

1. **ABI**: slot 6 pops 12 bytes and slot 7 pops 16; calling slot 6 with `(x, z, 2)` behaves like slot 7 with
   `(x, z, 128.0, 25.0)`; slot 6 with type 3 changes nothing; `GetMaxMayorRating(p)` writes `p` and returns the value
   in AL; `GetGZCLSID` = `0x0990BDD8`; QI for `0x452294AA` / `0xE4FDA3D4` returns the sub-objects; unknown IID →
   false.
2. **Park funding**: raw park 100 and funding 0.8 → park map 80; raw park −20 → −20; raw 7 at 0.5 → 3; raw 30000 at
   funding 1.2 (`float32` 1.20000005) → `trunc(36000.0) = 36000` stored as **−29536** (wrap); a stub budget whose
   funding is NaN with raw 100 → 0.
3. **Landmark funding**: departments A (ID 0x10, funding 0.5, value 30) and B (ID 0x20, funding 1.2, value 10) →
   calls 1 and 2 build 15 and `trunc(15 + 12.0) = 27`, call 3 publishes 27 and moves to state 4 (3 calls for 2
   departments). Department ID 0 uses factor 1.0. Two departments with value 20000 each at funding 1.0 → landmark
   map **−25536** (wrap).
4. **Tax effect**: first cycle reads rate 9 for index 0 → no effect; rate later 10 → `effectiveBaseAura[R$] =
   trunc(curve(1)) = −5`; rate 7 → `+7`; rate 9 again → 0 (no change, effective = base).
5. **Lot base aura**: effective R$ = −5, R$$ = 0; lot with R$ 30, R$$ 10 → `trunc(−150 / 40) = −3`; with an extra
   commercial entry of 40 → `trunc(−150 / 80) = −1`; land value type 2 and wealth 1 → `−1 + trunc(curve(1)) = −4`;
   no residential entry → −128; {R$ 0, CS$ 40} (a residential entry with count 0) → **0**, not −128 (§6.5.1); two
   maps {R$ 30} and {R$ 10} → `trunc(−200 / 40) = −5`.
6. **Work stamping**: two overlapping lots with values −4 and 3 → the shared cells hold 3; cells outside any lot
   stay −128 and are unaffected by every later stage.
7. **Curves on cells**: work 10, park map 50 → 12; park 100 → 13; crime 40 → 15; crime 255 → `clamp(10 − 50) =
   −40`; residential cell with trip length 128 → `trunc(10 − 0.0275) = 9`; trip length 0 → `trunc(10 − 7) = 3`;
   non-residential cell → unchanged by school, hospital, trip and congestion; work 120 + air curve(0) = 10 → 127.
8. **Long decay**: old base 10, block average 50, globalEffect 0 → step `ceil(40 × 0.100000024) = 5` → 15; old 0,
   new 10 → 2; old 0, new −10 → −2; old 0, new 1 → 1; old −128 → `clamp(new)`; no valid cell → −128. With code
   default longDecay 0.6: old 0, new 10 → `ceil(3.9999998) = 4`.
9. **Global effect**: ordinances Youth Curfew (−10) and Nuclear Free Zone (+5) active → globalEffect −5 is added to
   every block average before the decay.
10. **Aura and rating**: tracts (aura 20, pop 300) and (−10, pop 100) → mayor rating `trunc(5000 / 400) = 12`;
    (−25, pop 1) and (0, pop 1) → `trunc(−25 / 2) = −12`; total population 0 → 0; base −128 → aura 0 and not
    counted; best tract (3, 5) → `GetMaxMayorRating` location (14, 22).
11. **Transient**: `AddTransientEffect(x, z, 48.0, 10.0)` with tract width 64 m → radius 0.75 tracts, centre tract
    +10 (simgrid §7.1); monthly decay with 0.75: 10 → 7 → 5 → 3 → 2 → 1 → 0; −10 → −7.
12. **Hotspots** (threshold −50, radius 3, min weight 4): low tracts in order (10,10), (11,10), (10,12), (20,20),
    (9,11) → one hotspot (9, 10, weight 4); the (20,20) hotspot (weight 1) is removed.
13. **Riot**: hotspot weight 4 → chance `curve(4) = 8`; with a stub generator returning roll 7.5 the riot manager
    receives `AttemptDisasterStart(38, 42)` for hotspot (9, 10) (cell indices, not tracts or metres); roll 8.0 → no
    riot; one double draw per cycle, plus one integer draw only when there is a hotspot.
14. **Cadence** (state 1 ran on day D, so cycleStartDay = D; SimNewDay delivered with `GetSimDateNumber() = D + 5`;
    the comparison uses the state **after** the step, §5.1): state 3 with a department left → one department, state
    3, returns **false**; state 3 with every department processed → publish, state 4, **false**; state 4 with lots
    → state 5, returns **true** (comparing before the step would give false); state 6 with the park curve → 7,
    true; state 0 → no step, true. On day D + 6, state 5 with lots left after the 5 ms budget → stays 5, returns
    false, and the next delivery on the same day continues the lots. SimNewMonth during a running cycle does not
    reset the state. With an empty lot list the step in state 4 ends the cycle (state 0, SimNewDay returns true)
    with no history record and no `0x2A09B21A`.
15. **Occupant cones**: park with `{50, 22}` inserted on an even-width 2×2 lot at x1 = 10 (cells 10..11, edges at
    160 m and 192 m) with the position x = 170 m (left half) → centre cell 10, tract 2; at x = 176 m (exact middle)
    → cell 11; radius `22 / 4 = 5.5` tracts, centre value +50, outer 0; removal subtracts the same cone; a 3-wide lot
    at x1 = 10 → cell 11 regardless of position; no lot → the position's own cell. A landmark whose budget item has
    department `0x8A416368` creates a department grid with instance ID `0x8A416368` on insert; a removal with no
    grid changes nothing. Mayor-rating effect {7, 256} → radius 64 tracts on the building-effect grid, centre tract
    from the occupant position (not the lot centre). The park property as Uint32 {50, 22} → the same cone as
    Sint32; as Float32 {50, 22}, as Sint32 {50} or as a Sint32 scalar → no cone *[choice]* (§7).
16. **Persistence**: write → read → write is byte-identical (v9, two departments, three hotspots); version 7 →
    false; a version-8 record with one extra grid reference loads, has no departments, and re-stamps landmarks at
    PostCityInit; version 10 is rejected by FreeCity4 *[choice]*. **Read into a used object** (§11.3): an object
    holding departments {0x10 → G1, 0x30 → G3} and hotspots [(1, 2, 5)] reads a record with departments
    {0x10 → H1, 0x20 → H2} and hotspots [(9, 10, 4)] → departments {0x10 → H1, 0x20 → H2, 0x30 → G3} (G1 is not
    released) and hotspots [(1, 2, 5), (9, 10, 4)]. A record truncated before `hotspotCount` → FreeCity4 reads 0
    hotspots *[choice]*; a version-8 record whose extra grid reference is 0 → FreeCity4 loads it without the
    shutdown *[choice]*.
17. **BustStrike(0)** calls police, fire, residential health, residential school `EndStrike` in that order;
    `BustStrike(3)` only the health strike; `BustStrike(5)` nothing.
18. **Registry**: `sc4game.aura.add_transient_effect(x, z, 48, 10)` equals test 11; `end_strike(1)` equals
    `BustStrike(1)`.
19. **No Parks department** *[choice]*: a budget without department `0x8A4260A6`, raw park 100 → park map 100
    (factor 1.0) and the pipeline moves to state 3 (the original crashes).
20. **Empty curves** *[choice]*: crime curve property `0xEA023277` with one value (5.0), or with none → the crime
    stage is skipped (work 10 stays 10, crime 255 has no effect); with three values (0, 10, 255) → one point (0, 10),
    a constant 10 → work 10 becomes 20 for any crime value. An empty park curve property → state 6 advances as in
    §6.6's choice.
21. **Population mapping**: shipped P = S = 4; aura tract (3, 5) with population 300 / 50 / 0 in tract (3, 5) of the
    three grids → `pop = 350` for that tract. FreeCity4 with a population tract size P = 8 *[choice]* reads
    population tract `(⌊14 / 8⌋, ⌊22 / 8⌋) = (1, 2)` for aura tract (3, 5).
22. **Time-slice step** (§6.5, run with a stub clock that never expires): process static step 1024, miss 0; a call
    that stamps 30 lots and exhausts the iterator → `count = 1024 × 31`, `n = 31` → miss 1; after 16 such calls
    step = 512 and miss = 0. With step 512 a call that stamps 2 lots → `n = 1536 >> 10 = 1` → miss −1; with step
    256 and 2 lots → `count = 768`, `n = 0` → nothing. With step 256 the clock is read before lots 4, 8, 12, …
    (iterations whose `count` is a multiple of 1024). Stamping results are the same for every step value.
