# Landfill developer (`cSC4LandfillDeveloper`) — behaviour spec

Status: round 3, written from the Windows 1.1.641 executable (`bin/SimCity4.exe`, image base `0x00400000`). Names
come from the Mac build's symbols (`external/extracted/sc4-mac-symbol-analysis`); every fact was checked on
Windows. The classes covered are the developer itself, the landfill zones it owns (`cSC4LandfillZone`) and the
landfill tile occupant (`cSC4LandfillOccupant`).

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

Shared material cited and not repeated here: streams, refs and strings (foundations §2, §2.7), class store and
city component table (§3, §3.6), Init order (§3.7), response curves (§4), messages (§5), RNGs and the C `rand()`
(§6, §12), month order (§13), slot-order rule (§16), framework slot tables (§17), x87 precision (§18), property
holder records (§10.3); scheduler levels and agent semantics (simulator spec); garbage supply, garbage grid and
the landfill flag (pollution spec §3, §4.6, §8); departments and line items (budget spec §7).

---

## 1. Role and identity

The landfill developer manages **landfill zones** (zone type **14**). Garbage that the pollution simulator sends
toward landfills accumulates in pollution's **garbage supply** (pollution §4.6.1). Once a month the developer
moves garbage from that supply into the landfill zones of the city. Each zone stores garbage in **heaps** (one
per 2×2 block of zoned cells). The heaps decay, are spread evenly across the zone, and drive which landfill tile
models are shown. The developer also:

- prices zoning a landfill (terrain levelling cost) for the zone tool,
- levels the terrain under new landfill cells,
- removes landfill cells when the terrain is modified under them,
- books the landfill maintenance in the sanitation department (line item 30),
- tells pollution how much free capacity is reachable (`SetGarbageCapacity`) and sets/clears the active-landfill
  flag (bit 15 of pollution's garbage cell) under each tile model,
- places a "zot" (a not-connected warning prop) on a zone without road access to any developed building,
- runs the landfill's garbage-truck automata and seagull effect (rendering only, §4.10).

| Item | Value |
|---|---|
| Class ID (`GetGZCLSID`) | `0xEA09D4AE` |
| Primary interface | `cISC4LandfillDeveloper`, IID `0x8A09D502`, vtable `0x00AB157C`, **13 slots** (§7.1) |
| QI also answers | `0x00000001` (cIGZUnknown) → primary; `0x452294AA` `cIGZMessageTarget2` (vtable `0x00AB156C`); `0xE4FDA3D4` `cIGZSerializable` (vtable `0x00AB1554`). Anything else → false. |
| City component | slot **#12** (foundations §3.6); created with `GetClassObject(0xEA09D4AE, 0x8A09D502)` |
| City getter | `cISC4City` slot 78 (`+0x138`) `GetLandfillDeveloper`, cached at city init as a global service pointer (§9) |
| Init order | after the zone developer, before budget and pollution (foundations §3.7) |
| Simulator type | `GetSimulatorType()` returns `0x2A398ED6` |
| Lua | `landfillzone.lua` (game data) defines the `landfillzone_attract` attractor and `landfillzone_out` garbage-truck generator that the zone automata use (§4.10). No engine logic depends on it. |

All three interfaces are one object with one reference count. `AddRef` increments. `Release` with a count ≤ 1
calls `Shutdown()` (virtual), destroys the zone list and the object, and returns 0; otherwise it decrements and
returns the new count. *[compat]*

**Owned objects.**

| Class | Kind | Created by | Saved |
|---|---|---|---|
| `cSC4LandfillZone` | plain refcounted object, **not** a GZCOM class (no CLSID). Implements a `cIGZMessageTarget2`-shaped vtable (`0x00AB1610`: QI answers `1` and `0x452294AA`; `DoMessage` returns true and does nothing; slot 4 is the destructor). | the developer (`new`) | inline in the developer's record (§6.2) |
| `cSC4LandfillOccupant` | GZCOM class `0x8A22BEBE` (factory `0x004776B0`), an occupant | a zone, one per non-empty tile | by the occupant manager, as its own class record (§6.4) |

### 1.1 Init

`Init()` (slot 3), once per object (a second call returns true at once):

1. On the first `Init` of this object read the tuning exemplar (§4.1).
2. `simulator.AddAgent(messageTarget, 6, "cSC4LandfillDeveloper", 2)` (simulator spec §4; flags 2 = enabled once
   the city is established). Level 6 = month.
3. Acquire (with `AddRef`) the city, terrain, pollution simulator, budget simulator and message server.
4. Subscribe to `0x07699455`, `0x26D31EC1`, `0x0772FAD4`, `0x278E6003`, `0xC6CDBF1A` (§3.2).
5. Return true.

The sanitation department is **not** acquired here; it is fetched on PostCityInit (§3.2).

**Where the services come from** *[compat]* (LG15; `0x006B9D98`–`0x006B9E87`). Init reads them from the
global service pointers that the city fills, not from the city's getters:

| Service | Global | Held |
|---|---|---|
| simulator, for `AddAgent` | `0x00B43D5C` | **no** (LG18) |
| city | `0x00B43CEC` | yes (`+0x1C`) |
| terrain | `0x00B43CF4` | yes (`+0x24`) |
| pollution simulator | `0x00B43D9C` | yes (`+0x20`) |
| budget simulator | `0x00B43D64` | yes (`+0x28`) |
| message server | `0x00B43CCC` | yes (`+0x18`) |

Each held service is taken with `AddRef`, and only if its field is still null. The five subscriptions are made
only in the branch that takes the message server. With the server already held, there would be none.

`Shutdown` releases the five held services (§1.2). Elsewhere the classes also use globals directly:
- `getLandfillTileCost` takes the occupant manager from the global city (§4.9.1);
- `PopulateZone` takes the network map from the global city (§4.4.5);
- `AddZot` uses the terrain and prop manager globals `0x00B43CF4` and `0x00B43D10`.

**The simulator** (LG18). `AddAgent` runs **before** any service is taken (step 2). It goes through the global
`0x00B43D5C`, which `SL::OnCityInit` (`0x00601C20`) fills with `city.GetSimulator()` (city slot 50) and
`SL::OnCityShutdown` (`0x00601FF0`) clears. Init does not check the global for null or hold a reference to it
(`0x006B9D98`–`0x006B9DBA`). `Shutdown` fetches the simulator again from the global city's slot 50 for
`RemoveAgent` (`0x006B95D3`–`0x006B95ED`, §1.2 step 5). Both calls therefore reach the city's own simulator.

A held reference and a per-call fetch name the same object for the whole life of the city. So holding only the city
and the message server changes reference counts and nothing else, provided nothing calls a service after the city
has destroyed it. Nothing in §1.2 does.

### 1.2 Shutdown

`Shutdown()` (slot 4): unsubscribe the five messages, release the message server; for every zone: zone
`Shutdown`, `Release`; clear the zone list; release the department, budget, pollution, terrain and city;
`simulator.RemoveAgent(messageTarget)` (the simulator is taken from the city, `cISC4City` slot 50). Return true.

The exact order (LF3; `0x006B94B0`), all inside "if initialized" (otherwise the call just returns true):

1. Clear the initialized flag.
2. If the message server is held: unsubscribe `0x07699455`, `0x26D31EC1`, `0x0772FAD4`, `0x278E6003`,
   `0xC6CDBF1A` (in that order), release the server, null the pointer.
3. For each zone in list order: zone `Shutdown` (§4.4.9), zone `Release`, unlink and free the list node. The list
   ends empty.
4. Release, each only if held and then nulled: the department, the budget simulator, the pollution simulator, the
   terrain, the city.
5. Take the simulator from the **global** city pointer (`0x00B43CEC`, slot 50), not from the developer's own
   field (already released), and, if there is one, `RemoveAgent(messageTarget)`.

So the developer stops receiving messages **before** any zone is shut down. Zone `Shutdown` uses only the zone's own
service references, never the developer's, and posts nothing the developer subscribes to; so releasing the message
server together with the other services after the zones (one `release_services` call) is not observable. Matching
the original order costs nothing and is recommended.

**A separate release for the message server** (LF6): no original behaviour; the implementer's choice stands. The
message server is the framework's system service. Its global (`0x00B43CCC`) is set once, at application start
(`SL::OnAppInit1`, `0x006024BB`), and the service lives as long as the application. The developer's reference is
therefore never the last one, and when it is released cannot be seen. What could be seen is when the developer
**stops receiving** its five messages, which the original does before the first zone `Shutdown` (step 2). One
`release_services` call after the zones is equivalent provided no message reaches the developer between the start
of `Shutdown` and that call. The zones post none of the five while they shut down.

## 2. State

### 2.1 Developer

| Field | Initial | Saved | Meaning |
|---|---|---|---|
| `zones` | empty list | yes (§6.1) | landfill zones, in creation (or load) order |
| `maintenance` | 0 | no | int64; Σ zone maintenance, last `UpdateBudgetRequirements` |
| `capacityTotal` | 0 | no | u32; Σ heap capacity of **connected** zones, last `UpdateCapacityValues` |
| `usedTotal` | 0 | no | u32; Σ garbage contained in **connected** zones, last `UpdateCapacityValues` |
| `loaded` | false | no | set by `Read` (§6.1), consulted on PostCityInit |
| `department` | null | no | sanitation department `0x6A42A8F5`, from PostCityInit |
| service pointers | null | no | city, terrain, pollution, budget, message server |

`maintenance`, `capacityTotal` and `usedTotal` are recomputed on PostCityInit after a load (§3.2).

### 2.2 Zone

| Field | Initial | Saved | Meaning |
|---|---|---|---|
| `region` | from the zoning region | yes | `SC4CellRegion<long>`: inclusive cell rect `x0, z0, x1, z1` plus a bit mask of size `(x1−x0+1) × (z1−z0+1)`. A cell is part of the zone iff its mask bit is set ("mask cells"). |
| `W`, `H` | `x1−x0`, `z1−z0` | yes | heap grid size |
| `heaps` | `W × H` heaps, all valid | yes | heap `(hx, hz)` (local, 0-based) covers cells `x0+hx … x0+hx+1`, `z0+hz … z0+hz+1`; vector index `hz × W + hx` |
| `connected` | false | yes | result of the last `IsZoneConnected` (§4.7.2) |
| `zot` | null | yes (ref) | the not-connected prop (§4.7.5) |
| `levelAltitude` | 0.0 | **no** | target altitude for levelling and zoning cost; on `Init`, if exactly 0.0 it becomes the altitude at the centre cell (§4.4.9) |
| `completelyDezoned` | false | no | sticky cache (§4.4.7) |
| `cellCount` | dirty | no | cached number of mask cells |
| automata, seagull effect | none | no | rendering (§4.10) |

A **heap**:

| Field | Initial | Saved | Meaning |
|---|---|---|---|
| `hx`, `hz` | its local position | yes | local coordinates (relative to the zone origin **at the time the heap was made**, §4.4.4) |
| `valid` | true | yes | the heap's four cells are zoned (cleared by `InvalidateSurroundingHeaps`) |
| `active` | false | yes | the heap currently shows tile models (contained ≠ 0 at the last check) |
| `capacity` | 0 | yes | u32 |
| `contained` | 0 | yes | u32, garbage held |

### 2.3 Landfill occupant (`cSC4LandfillOccupant`)

An occupant of type **`0x6A11BD2B`** (set by the constructor) with:

| Field | Initial | Saved | Meaning |
|---|---|---|---|
| `tileID` | ctor argument (default `0x02F10000`) | yes | landfill tile exemplar instance (group `0x07BDDF1C`, type `0x6534284A`) |
| `orientation` | ctor argument (default 0) | yes | s32, model rotation |
| `position` | (0,0,0) | yes | cell centre, terrain altitude |
| `isEmpty` | false | yes | `IsEmptyLandfillTile()`; nothing in the shipped code sets it true *[inferred]* |
| bounding box | from position | no | `(x−8, y, z−8) … (x+8, y+16, z+8)` metres, recomputed on `Read` and `SetPosition` |
| base occupant state | | yes | `cSC4Occupant` record (§6.4) |

The constructor also calls `SetFlag(4)` (visible). That changes nothing: `cSC4Occupant`'s constructor already sets
flag 4 for every occupant (`0x005E9265`) (LG28).

## 3. Agents, messages and cadence

### 3.1 Agent

One month-level agent (level 6, message `0x66956816`), registered in `Init` with flags 2. `DoMessage` returns
**true** for every message (finished). The month handler is §4.7.

### 3.2 Messages handled

| Message | Handling |
|---|---|
| `0x07699455` zoning (zone manager after `PlaceZone`) | `DoZoneChange(region = GetVoid2, zoneType = GetData3)` (§4.4.1). Runs for every zone type. |
| `0x278E6003` terrain modified | Only if the zone list is non-empty. Cell rect: `x0 := max(0, (data1 >> 16) − 1)`, `x1 := min(data1 & 0xFFFF, cellCountX − 1)`, `z0 := max(0, (data2 >> 16) − 1)`, `z1 := min(data2 & 0xFFFF, cellCountZ − 1)` (cell counts from the city). Only if `data4 ∈ {1, 2, 3}`: `DoTerrainChanged(rect)` (§4.5.1). The landfill's own levelling uses reason 5, which is ignored (§4.5.3, verified). |

**Where `data4` comes from** *[compat]* (partly answers §10.2 question 3). `0x278E6003` has exactly **one** send
site in the whole executable: `cSTETerrain::RedisplayTerrain(redisplay, ?, rect*, data4)` (`0x00749850`, terrain
slot 68 / `+0x110`), which at `0x00749960` calls the standard "post a message with five arguments" helper
`0x00743380`. That helper (verified in assembly) builds a `cRZMessage2Standard`, sets the type through message
vtable `+0x14`, data1..data4 through `+0x2C`, `+0x40`, `+0x54`, `+0x68`, and then **posts** it with
`messageServer[+0x10](msg, 0)` = `MessagePost(msg, false)`. So `data4` is literally `RedisplayTerrain`'s fourth
argument, and the message is always **queued**, never synchronous.

There are exactly two kinds of caller, and they are the only sources of `data4`:

| Caller | `data4` |
|---|---|
| `cSTETerrain::ApplyBrush` (`0x0074F0E0`, terrain slot `+0x110` at decomp `cSTETerrain.c:9439`) | `tBrushParameters::terrainMessageData4` — the brush parameter struct's field at `+0x20` (the SDK's `cISTETerrain::tBrushParameters`, whose own `static_assert` puts that field at `0x20`) |
| `cSC4Demolition::ModifyTerrainHeight` (`0x00467BB0`, demolition slot 9; terrain slot `+0x110` at decomp `cSC4Demolition.c:689` and `:780`) | its `reason` argument, passed straight through |
| the remaining `+0x110` call sites in `cSTETerrain.c` / `cSTETerrainView3D.c` | either a literal `0` or `1`, or a passthrough argument of an enclosing terrain function (`cSTETerrain.c:2166` and `:3626`) whose own callers were not enumerated |

Note that the message is sent only from the `redisplay ≠ 0` branch of `RedisplayTerrain`, so the many internal
`(0, 0, rect, 0)` calls post nothing.

The lot developer's `AttemptAreaLevelling` calls `ModifyTerrainHeight` with reason **5** (§4.5.3), which is why
the landfill's own levelling does not trigger `DoTerrainChanged`. Still **unresolved**: which brush sets
`terrainMessageData4` to 1, 2 and 3 (that field is filled by the terraforming tools' UI code, which was not
traced) — so the "*[inferred: 1–3 are the terraforming tools]*" reading in §3.2 stays inferred. What is now
verified is that 5 is the levelling reason, and that the only routes to a `data4` of 1, 2 or 3 are a brush, a
`ModifyTerrainHeight` caller passing 1–3, or one of the two passthrough sites.
| `0x0772FAD4` FundsChanged | Only if `department` is set: `funded := *department.GetFundingPercentage() > 0.0` (strict float compare). For every zone with a truck automaton: enable/disable it with `funded` (§4.10). No simulation state changes. |
| `0x26D31EC1` PostCityInit | `department := budget.GetDepartmentBudget(0x6A42A8F5)`, `AddRef` (a previous department is **not** released *[quirk: leak on a second PostCityInit]*). If `loaded`: every zone `Init` (§4.4.9), then `UpdateBudgetRequirements` (§4.9.3) and `UpdateCapacityValues` (§4.6.5). |
| `0x66956816` SimNewMonth | `DoMonthlySimulation` (§4.7.1). |
| `0xC6CDBF1A` PreViewLevelChange | Only if the zone list is non-empty. `level := view3D.GetCurrentViewLevel()` (the 3D view stores the **new** level before sending this message). For each zone with a seagull effect: if `level = 1` and the effect is not started → `Start(0)`; if `level ≠ 1` and it is started → `Stop(1)`. Rendering only. |

**The 3D view** (LG16; `0x006BA07F`–`0x006BA0D5`). The PreViewLevelChange handler reads the view from the global
`0x00B43DC4` and calls its slot 22 (`GetCurrentViewLevel`). It does not check for null *[quirk]*: with landfill
zones and no 3D view, the message crashes.
- `cSC4View3DWin::Init` stores the view in that global through `SL::OnView3DCreate` (`0x00602120`) and gives it
  window ID `0x9A47B417`.
- `SL::OnView3DShutdown` clears the global.
- Per zone with a seagull effect, `level = 1` calls the effect's slot 3 with 0 when its slot 5 says it is not
  running; any other level calls slot 4 with 1 when it is.

The same object can be reached through public interfaces. `cISC4App::GetMainWindow()`, then
`GetChildWindowFromID(0x6104489A)`, then `GetChildAs(0x9A47B417, 0xFA47B3F9, &view)` gives the
`cISC4View3DWin`. The SDK's `SC4UI::GetView3DWin()` does exactly this. *[choice]* Glue can use that route, and should
skip the effect toggles when it returns null.

**Threads** (LG20). The original `DoMessage` (`0x006B9EA0`) takes **no lock**. It switches on `GetType()` and
handles every message it is given. All five subscriptions and the month agent are delivered on the main thread
(foundations §18.2):
- the zone manager and the terrain post their messages, and the message server delivers them on its main-thread
  tick (§3.2, §4.4.8);
- PostCityInit, FundsChanged and PreViewLevelChange come from main-thread code;
- no worker thread of the game was found to send any of them.

So the original never drops a message and never sees one from another thread. A lock in FreeCity4's glue has no
counterpart in the original. *[choice]* Dropping on contention matches the original only while it never happens:
a dropped `0x07699455` would leave a zone unextended, and a dropped `0x278E6003` would leave a terrain change
unhandled. Each drop should be logged. Waiting for the lock would be the safer choice.

### 3.3 Ordering within the month level (foundations §13)

The original runs month agents in object-address order. In the logged sessions the landfill developer has
registration 2 and **address rank 20**, after power (1), residential (2), **pollution (3)**, **budget (4)**,
plumbing (5), aura (6) … crime (19), and before the port developers (21, 22), neighbours (23) and ordinance (24).
Aura (registration 15, rank 6) has no data link with the landfill developer.

Data links on the 1st of a month:

| Other agent | Link | Original order (rank) | Effect *[compat]* |
|---|---|---|---|
| pollution (3) | its month agent records the `landfilled` counter and the sanitation cost, then zeroes the counter (pollution §4.9); the landfill month agent calls `GarbagePutInLandfill` | pollution **before** landfill | intake placed on the 1st of month M is reported in pollution's history and charged at the roll of the 1st of M+1 |
| budget (4) | the settlement sums department expenses (budget §5.1); the landfill month agent sets line 30's full expenses | budget **before** landfill | the settlement on the 1st of M+1 books the maintenance set on the 1st of M |
| neighbours (23), ordinance (24) | read `GetLandfillUsage` (neighbour landfill deals; landfill gas recovery) | landfill **before** them | they see this month's capacity |

**Requirement** *[compat]*: the landfill developer's month agent runs **after** pollution's and the budget's, and
**before** the neighbours' and the ordinance simulator's. This order holds in all eight logged sessions
(foundations §13.3). Registration order alone does not give it, so FreeCity4 orders this agent explicitly. FreeCity4's registration order (foundations §13.3, which
puts the landfill developer second because it initialises early, §3.7) does not satisfy the first half. See
§10.2 question 1.

**What a wrong order changes** (LG6). If the landfill month runs **before** pollution and the budget, everything
the landfill does on the 1st of a month reaches those two systems one month early. Nothing else changes: the
landfill's own state, and the supply and capacity it leaves for pollution's daily steps, are the same in both
orders.

Example: a connected 2 × 2 landfill zoned in February. Its maintenance is still 0, because zoning does not book it
(§4.4.1 step 3). On 1 March the landfill takes 10000 from the supply. Its one heap turns active, the four cells
get `0x034E0000` tiles (cost 10 each), and line 30 is set to 40.

| On the 1st of… | Original order (pollution, budget, then landfill) | Landfill first |
|---|---|---|
| March | pollution records and zeroes `landfilled` without the 10000. The budget settles line 30 at 0. | pollution's history `0xEA1101AA` and line 20's cost include the 10000. The budget settles line 30 at 40. |
| April | pollution's history and line 20's cost include the 10000. The budget settles line 30 at 40 (the 1 March value). | the 10000 is not counted again. The budget settles line 30 at the value the landfill has just set on 1 April. |

So every landfill entry in pollution's garbage history, every line 20 charge for landfilled garbage and every line 30
settlement moves one month earlier. The total over a long run is the same. A conformance vector should assert the
order, as §10.2 question 1 says.

**How to get the order.**

- With FreeCity4's simulator, use the rank of foundations §13.4. That section deferred the rank mechanism until a
  tail agent's glue lands. The landfill glue has now landed, so the condition is met.
- When the original simulator delivers the month, the order is the unsigned order of the `cIGZMessageTarget2`
  pointers passed to `AddAgent` (foundations §13.1). Glue registered there inherits whatever address its object
  gets. *[choice]* Such glue should, at registration, compare its pointer with pollution's and the budget's and log
  when it is not above both.

## 4. Behaviour, formulas and tuning

Float arithmetic follows foundations §18: each `+ − × ÷` rounds to float32. `round(v)` below means the
**magic-number round** used throughout this class: `bits(float32(v + 12582912.0)) − 0x4B400000`, which is
round-half-to-even for `0 ≤ v < 2^22`; for larger `v` the result is wrong (a value in `[2^22, 2^23)` comes out as
`2^22 + (v − 2^22)/2`) *[quirk]*. `float(u)` of a u32 is exact-as-int64 then rounded to float32 (values ≥ 2^31
get `+2^32`).

### 4.1 Tuning

Exemplar: type `0x6534284A`, group `0x67CD5FA1`, instance `0x6A2806AF` ("LandfillDeveloper"), read once per
object on the first `Init` through `cSCPropertyHelp::GetPropertyValue`. A missing property leaves the static
unchanged (the values are process-wide statics: a second developer object would overwrite them with the same
values). No response curve is loaded (foundations §4.1 does not apply).

| Property | Name (Mac symbols) | Read as | Code default | Shipped | Used in |
|---|---|---|---|---|---|
| `0x4A280510` | kLandfillDecayRate | float | 0.1 | 0.07 | §4.7.3 |
| `0x4A280513` | kLevelToleranceThreshold | float | 1.0 | 5.0 | §4.3 |
| `0x4A677E5B` | kMaxGarbageToTakePerMonth | u32 | 40000 | 327 680 000 | §4.7.1 |
| `0x4A677E5C` | kMaxGarbageToTakePerMonthPerTile | u32 | 5000 | 5000 | §4.7.2 |
| `0x4A677E5E` | kMaxGarbageToDecayPerMonthPerZone | u32 | 3000 | 1500 | §4.7.3 (applied **per heap** *[quirk]*) |
| `0x4A677E5F` | kDefaultLandfillTileCost | s64 | 0 | 5 | §4.9.1 |
| `0x2AA00FE7` | kCostPerCellHeightChange | float | 1.0 | 4 (stored as s64, converted to 4.0 by the variant) | §4.3 |
| `0x4A677E5D` | kbMoveWithTerrain | bool | true | true | §4.5.2 |

Other constants: default efficiency 1.0; empty tile `0x02F10000`; terrain-change threshold 0.25 m (§4.5.2);
zot height offset +3.0 m (§4.7.5).

Values used from other subsystems: landfill tile capacity `tileCap := pollution.GetLandfillTileCapacity()`
(pollution tuning `0x4A65267A`, shipped 30000, code default 32767), read each time it is needed.

**Tile exemplars** (type `0x6534284A`, group `0x07BDDF1C`); cost = first value of property `0xEA54D286` (budget
item cost, budget §3):

| Tile | Shown for | Cost |
|---|---|---|
| `0x02F10000` | empty (never instantiated, §4.8) | 1 (unused) |
| `0x034E0000` | one active heap at a corner | 10 |
| `0x02F90000` | two adjacent active heaps (edge) | 20 |
| `0x034F0000` | two diagonal active heaps | 20 |
| `0x02FB0000` | three active heaps | 30 |
| `0x02FA0000` | four active heaps (interior) | 40 |

### 4.2 Geometry helpers

- **Cell centre**: `cISC4City::CellCenterToPosition(x, z) → (px, pz)` (slot 97). *[compat]* (LG10;
  `0x00459440`, pop 16): `px := float32((x + 0.5) × cellSizeX)` and `pz := float32((z + 0.5) × cellSizeZ)`, from
  `FILD`, a `+ 0.5f` and a multiply by the city's float cell sizes (`+0x108`, `+0x10C`, 16.0 in shipped cities).
  Both are stored with `FSTP`, so the x87 stack is **empty** at `ret` and there is **no return value**. EAX holds
  the `px` pointer by accident. Example: cell (37, 52) → (600.0, 840.0).
- **Altitude at a cell**: `terrain.GetAltitude(px, pz)` (`cISTETerrain` slot 13) at the cell centre.
- **Inclusive intersection** of two rects: `a.x0 ≤ b.x1 ∧ a.x1 ≥ b.x0 ∧ a.z0 ≤ b.z1 ∧ a.z1 ≥ b.z0`.
- **Centre cell** of a rect: `((x0 + x1) / 2, (z0 + z1) / 2)` with C integer division (toward zero).

### 4.3 Altitude variance and zoning cost

**`GetAltitudeVariance(rect) → (variance, mean, max, min)`** over **every cell of the rect** (not the mask):

1. `max := min := altitude(centre cell)` (not sea-level adjusted).
2. `sea := terrain.GetSeaLevel()` (slot 29).
3. For `x` in `x0…x1` (outer), `z` in `z0…z1` (inner): `a := altitude(x, z)`; if `a ≤ sea`: `a := sea + 1.0`.
   If `a < min`: `min := a`; else if `a > max`: `max := a`. `sum := sum + a`; `count := count + 1.0` (both
   float32).
4. `mean := count > 0 ? sum / count : 0`. `variance := |max − min|` (float32).

**`GetZoningCost(region) → int64`** (slot 9):

1. `target :=` `levelAltitude` of the **first** zone (list order) whose rect intersects the region's rect
   (inclusive). No tolerance test in that case.
2. Otherwise: `(variance, mean) := GetAltitudeVariance(region rect)`. If `variance ≤ kLevelToleranceThreshold`:
   return 0. Else `target := mean`.
3. `s := 0.0`; for each **mask** cell of the region (x outer, z inner): `s := s + |target − altitude(x, z)|`
   (float32; raw altitude, no sea adjustment).
4. Return `round(kCostPerCellHeightChange × s)` (float32 product, then the magic round) as an int64 whose high
   dword is **always 0** *[compat]*.

The zone manager adds this to the zoning cost of a type-14 `PlaceZone` (int64 add, §9).

### 4.4 Zoning and dezoning

#### 4.4.1 `DoZoneChange(region, zoneType)`

1. `R :=` the region's rect inflated by 1 cell on each side. Collect, in list order, every zone whose rect
   intersects `R` (inclusive; diagonal neighbours count).
2. If at least one zone was collected, for each collected zone `Z` in order:
   - zone type 14: `Z.ShutdownZoneEffects()`. If `Z` is the **first** collected zone: `Z.AddRegionToZone(region)`
     (§4.4.4), `Z.PopulateZone()` (§4.4.5), `Z.RecalculateCapacity()` (§4.6.2), `Z.InitZoneEffects()`.
     Otherwise `HandleZoneOverlap(Z, region, changeZone = false, removeTexture = false)`. *[quirk]* The effects
     of the other collected zones are shut down and not restarted, including zones only adjacent to the region.
   - any other type: `HandleZoneOverlap(Z, region, false, true)`.
   Then `UpdateBudgetRequirements()`.
3. If no zone was collected and `zoneType = 14`: create a zone for the region, `AddRef`, `Init` (§4.4.9).
   `(variance, mean) := GetAltitudeVariance(region rect)`. If `variance ≤ kLevelToleranceThreshold`:
   `levelAltitude := mean`; else `LevelRect(rect, mean, false)` (§4.5.3). Then `PopulateZone`,
   `RecalculateCapacity`, append to the list. *[quirk]* `UpdateBudgetRequirements` is **not** called (the new
   cells are first booked at the next month or zoning change).
4. Always: `UpdateCapacityValues()` (§4.6.5).

Consequences *[compat]*: zoning landfill next to (or diagonally touching) an existing landfill zone **extends**
that zone instead of creating a new one; zones are never merged; dezoning or rezoning to another type over a
landfill removes cells from it.

#### 4.4.2 `HandleZoneOverlap(Z, region, changeZone, removeTexture)`

1. If `Z`'s rect intersects the region's rect: `I :=` the intersection rect;
   `DezoneArea(Z, I, region, changeZone, removeTexture)` (§4.4.3).
2. If `Z.IsCompletelyDezoned()` (§4.4.7): `RemoveZone(Z, true)` (§4.4.6); else `Z.RecalculateCapacity()`.
3. `terrain.RedisplayTerrain(1, 1, &I, 5)` (slot 68; display only).

#### 4.4.3 `DezoneArea(Z, I, region, changeZone, removeTexture)`

For each cell of `I` whose bit is set in the **region** mask: `Z.DezoneCell(x, z, changeZone, removeTexture)`.
Afterwards, if `Z.GetNumValidHeaps() < 1`: set `Z.completelyDezoned := true` (sticky, §4.4.7).

#### 4.4.4 `AddRegionToZone(region)`

1. If a zot exists: remove it through the prop manager (`RemovePropA`, slot 29) and release it.
2. New rect := union of the zone rect and the region rect. New mask := old mask cells ∪ region mask cells.
3. For each region mask cell (x outer, z inner): `LevelRect((x, z, x+1, z+1), levelAltitude, false)`. This
   includes cells already in the zone.

   *[compat]* (LF5; `0x006BC150`, the call follows the mask-bit set for that cell). The rect is an **inclusive**
   cell rect, like the other rect passed to `LevelRect` (§4.4.1 step 3 passes the new zone's `{x0, z0, x1, z1}`). So
   each region cell levels the **2 × 2-cell block** `(x..x+1, z..z+1)`:
   - the average is over the 3 × 3 vertices `x..x+2 × z..z+2`;
   - the slope test is over the same block;
   - the call is `ModifyTerrainHeight(1, 1, x, z, x+2, z+2, avg, null, 5, null)` (§4.5.3).

   This is what the original does, and it must be reproduced as written. Three consequences follow:
   - The levelling reaches **one cell past** the region's +x and +z edges, onto cells that may not be landfill.
   - The cells are levelled one after another. Each cell's average is taken **after** the earlier cells' blocks
     were levelled, so the order (x outer, z inner) matters.
   - Each block is tested for slope on its own, so some blocks may be skipped (slope > 40) while others are
     levelled.

   Example: the region is the single cell (5, 5). The vertices 5..7 × 5..7 are averaged. If the terrain is flat
   at 100 m except vertex (7, 7) at 109 m, then `avg = (8 × 100 + 109) / 9 = 101`. The call is
   `ModifyTerrainHeight(1, 1, 5, 5, 7, 7, 101.0, null, 5, null)`, whose vertex range covers cells (5..6, 5..6).
   `levelAltitude` is passed as `f` and is not read (§4.5.3).

   With a 2 × 1 region (5, 5), (6, 5), cell (5, 5) is levelled as above first. Then cell (6, 5) averages the
   vertices 6..8 × 5..7, which by then include the six vertices 6..7 × 5..7 written by the first call.
4. Rebuild the heap grid for the new `W × H`: all heaps new (valid, inactive, empty, positioned by their new
   local coordinates); then each **old** heap is copied, with all its fields, to index `old.hz × W + old.hx`.
   *[quirk]* The old local coordinates are not translated: when the zone grows toward smaller `x` or `z`, the
   old heaps' garbage, capacity and active state move by the growth amount (and their tile models stay where
   they were until the heap's active state next changes).
5. For every cell of the new rect that is **not** a mask cell: `InvalidateSurroundingHeaps(x, z)`.
6. `cellCount` dirty.

#### 4.4.5 `PopulateZone()`

For each cell of the rect (x outer, z inner):

- If it is a mask cell:
  - if `zoneManager.GetZoneType(x, z) ≠ 14`, or the traffic network map has a network tile there
    (`FindNetworkTile(x, z, 0x1F4F, true)` ≠ null; flag mask as in flora §6.4.1): **clear its mask bit**;
  - else, if a landfill occupant (`0x6A11BD2B`) or a base-texture occupant (`0x497F6D9D`) is at the cell: keep
    it as is;
  - else `RemoveIncompatibleOccupants(x, z)` (removes flora `0x74758926` in the cell, §4.8.4) and
    `SetEmptyCellTexture(x, z)` (inserts a `cSC4LotBaseTextureOccupant` with the landfill zone texture, §4.8.5).
- If it is **not** a mask cell at the start: `InvalidateSurroundingHeaps(x, z)`.

*[quirk]* A cell dropped from the mask in this pass does **not** invalidate its heaps: heaps that include a road
cell or a cell that failed to zone stay valid and receive capacity and garbage (until a later `PopulateZone` or
`AddRegionToZone` pass sees the cell outside the mask).

**The road test** (LG7; `0x006BCDF6`–`0x006BCE16`). The traffic network map is fetched for each cell, as
`GetTrafficNetwork()` (slot 40) of the **global** city `0x00B43CEC`. The zone's own city pointer is not used. The
returned tile is only tested for null and is not released. That is correct, because `FindNetworkTile` (`0x004B0B20`, pop 16)
returns a **borrowed** pointer and makes no `AddRef` call *[compat]*. It returns the first network occupant
listed at cell `(x, z)` for which `HasAnyNetworkFlag(0x1F4F)` (network occupant slot 9) is true. If there is none
and the fourth argument is true, as here, it searches the map's multi-cell index under the key `(x << 16) | z`
the same way. Otherwise it returns null. The zone type comes from the zone's own zone manager (slot 12).

The map is the one the city creates in `CreateCityComponents` (`0x0045B07D`) with GZCOM
`CreateInstance(0x6A0F82B2, 0xCA0F81B9, &city[+0x1E8])`. City slot 40 `GetTrafficNetwork` (`0x004596F0`) returns
that field as it is: a `cISC4TrafficNetworkMap*` (IID `0xCA0F81B9`, vtable `0x00A8FED8`), with no `AddRef`. The
SDK's `intptr_t` is that pointer. `PopulateZone` calls slot 8 on it directly, with no QI and no release (LG17).

#### 4.4.6 `RemoveZone(Z, dezoneCells)`

Remove `Z` from the list. If `dezoneCells`: for each mask cell, `Z.DezoneCell(x, z, true, true)`. Then
`Z.Shutdown()` and `Release`. No capacity or budget update here (the callers do it, or not, §4.5.1).

#### 4.4.7 `IsCompletelyDezoned()` (zone)

If `completelyDezoned` is already true, return true **without rescanning** *[quirk: sticky]*. Otherwise set it
to true, scan the mask; if any bit is set, set it back to false and return false; else return true.

Since `DezoneArea` sets the flag whenever a zone keeps no valid heap, a zone narrowed to strips one cell wide is
treated as completely dezoned: `RemoveZone(Z, true)` then rezones **all** its remaining cells to type 0 and
removes their tiles *[compat]*. A zone created one cell wide (no heaps at all) is not affected until it is dezoned
in part.

#### 4.4.8 `DezoneCell(x, z, changeZone, removeTexture)` (zone)

1. If a zot exists: remove it (`RemovePropA`) and release it.
2. If `removeTexture`: `RemoveLandfillTile(x, z)` (§4.8.3) and `RemoveBaseTexture(x, z)` (§4.8.4).
3. Clear the cell's mask bit. `InvalidateSurroundingHeaps(x, z)`.
4. If `changeZone`: `zoneManager.PlaceZone(1×1 region at (x, z), zoneType 0, placeZone = true,
   skipFundsCheck = true, checkAreaSlope = false, skipZoneSizeCheck = true, checkLotSize = false, null, null, 0)`
   (slot 11). The zone manager then **posts** `0x07699455` for that cell, which re-enters `DoZoneChange` with
   type 0 on a later message-server tick; the zone being removed is already out of the list.
   *[compat]* **Not synchronous**: `cSC4ZoneManager::PlaceZone` (`0x004DD780`) delivers the message with
   `messageServer->vtable[+0x10]`, i.e. `MessagePost(msg, false)` — slot 4 of `cIGZMessageServer2` (`MessageSend`
   is slot 3 at `+0x0C` and takes one argument; this call site passes two, `(msg, 0)`). The message server is the
   zone manager's field `+0x18` (global `0x00B43CCC`). So there is **no re-entrancy** inside `DezoneCell`: the
   whole `RemoveZone` / `DoTerrainChanged` pass completes before the type-0 `DoZoneChange` runs.
5. `cellCount` dirty.

**`InvalidateSurroundingHeaps(x, z)`**: with `lx = x − x0`, `lz = z − z0`, set `valid := false` on heaps
`(lx, lz)`, `(lx, lz−1)`, `(lx−1, lz)`, `(lx−1, lz−1)` that lie inside `0 ≤ hx < W`, `0 ≤ hz < H`. Nothing else
(capacity, garbage, active state and tiles are untouched).

#### 4.4.9 Zone `Init` / `Shutdown`

`Init` (once): acquire city, occupant manager, terrain, pollution, the city's effect service and automata
controller manager, and the zone manager; `InitZoneEffects()`; if `levelAltitude` is exactly 0.0,
`levelAltitude := altitude(centre cell)`. Because `levelAltitude` is not saved, a loaded zone gets the
**centre altitude**, not the mean it was levelled to *[quirk]* (§4.3 then prices extensions against it).

The test (LF2; `0x006BB6E2`–`0x006BB6F2`) is an x87 `FUCOMPP` of the float32 `levelAltitude` (`+0x38`) against
the constant +0.0 (`0x00A81054`), then `TEST AH, 0x44` / `JP` to skip: it recomputes only when the compare is
**ordered and equal**. So +0.0 and **−0.0** both count as zero (IEEE equality), a **NaN** does not (unordered: the
NaN is kept), and every other value, however small (a denormal included), is kept. The centre cell is
`cx := (x0 + x1) / 2`, `cz := (z0 + z1) / 2` of the region rect, by C integer division (truncating toward zero;
the rect is never negative, so this is the floor). Then `CellCenterToPosition(cx, cz)` (city slot 97) and
`terrain.GetAltitude(px, pz)` (slot 13), stored as float32. Example: a zone over x 3..6, z 10..10 that reaches
`Init` with `levelAltitude = −0.0` gets the altitude at the centre of cell (4, 10).
`Shutdown` (`0x006BB750`):

1. Stop and release the zone's effects (§4.10; this runs even when the zone was never initialized).
2. Only if initialized: clear the flag.
   - If a zot is held: remove it through the city's prop manager (`RemovePropA`, slot 29), then release it.
   - Destroy the zone's region (cell mask).
   - Release the services the zone acquired, the city last.

**The zot on Shutdown** *[compat]* (LG13; `0x006BB766`–`0x006BB798`). The zone **removes the prop and then
releases it**:

1. If the city is held, take its prop manager (city slot 35).
2. If there is one, call `RemovePropA(zot)` (prop manager slot 29).
3. In every case, release the zot and null the pointer.

In the shipped flows this removal is almost never the one that counts:
- `RemoveZone` is always called with `dezoneCells = true` (§4.4.2, §4.5.1). Each `DezoneCell` removes the zot first
  (§4.4.8 step 1), and a zone with no mask cell left already lost it to the `DezoneCell` that cleared its last cell.
- So a zone reaches `Shutdown` still holding a zot only through the developer's own `Shutdown` (§1.2), when the city
  closes.

"Release only" differs from the original only there. It leaves the prop in the prop manager during the city
teardown, which nothing saves. *[choice]* Matching the original costs one call and is recommended.

**A zone that was never initialised** *[compat]* (LG24; developer `Shutdown` `0x006B94B0`, zone `FinalRelease`
`0x006BB8C0`, zone destructor `0x006BBBC0`). Such a zone skips step 2, so its `Shutdown` leaves the zot alone. The
reference is still released, by the zone's destructor:

1. The developer's `Shutdown` (only if the developer is initialised) walks its zone list. For each zone it calls
   the zone's `Shutdown` (`0x006B9535`), then releases the list's reference through the zone's slot 2
   (`0x006B953F`), and frees the list node.
2. Slot 2 is `FinalRelease`. With more than one reference it only decrements the count. At the last reference it
   calls `Shutdown` once more and then the deleting destructor (slot 4).
3. The destructor releases the zot if one is held (`0x006BBBC9`–`0x006BBBD2`). It does not call `RemovePropA` and
   does not null the field, because the object is being freed.

So the original releases its zot reference at the developer's `Shutdown` without removing the prop, provided the
list held the zone's last reference. An initialised zone has already released and nulled the zot in its own
`Shutdown`, so the destructor's release matters only for a zone that never ran `Init`. That happens when a
saved zone that holds a zot (version 4 or later, §6.2) is read but the city closes before PostCityInit runs.

**The bracket matches**: releasing FreeCity4's reference at the developer's `Shutdown`, without `RemovePropA`, is
what the original does.

### 4.5 Terrain

#### 4.5.1 `DoTerrainChanged(rect)`

For each zone: `HandleTerrainChange(rect)`. Then every zone for which `IsCompletelyDezoned()` is true is removed
with `RemoveZone(Z, true)`. *[quirk]* Neither `RecalculateCapacity` nor `UpdateCapacityValues` nor
`UpdateBudgetRequirements` runs: invalidated heaps keep their capacity and garbage until the next zoning change,
and the totals stay stale until the next month.

#### 4.5.2 `HandleTerrainChange(rect)` and `HandleCellTerrainChange(x, z)` (zone)

For each mask cell of `rect ∩ zone rect`:

1. If no landfill occupant is at the cell: `DezoneCell(x, z, true, true)` *[compat: any terraforming over an
   empty landfill cell dezones it, whatever the height change]*.
2. Else, if the occupant answers `cISC4LandfillOccupant` (`0x2A15F262`):
   - `p := occupant position`; `a := terrain.GetAltitudeAtNearestGrid(p.x, p.z)` (slot 19);
     `δ := p.y − a` (float32).
   - If `|δ| ≥ 0.25`:
     - if `IsEmptyLandfillTile()`: `DezoneCell(x, z, true, true)`;
     - else if `kbMoveWithTerrain`: `p.y := (sea < a) ? a : sea` (sea = `GetSeaLevel`); `SetPosition(p)`;
       `occupantManager` slot 24 `(occupant, 1)` (re-index the moved occupant *[inferred]*); occupant slot 15
       `(0x99EF1145, 0)` (display notification *[inferred]*);
     - else nothing.

#### 4.5.3 `LevelRect(rect, altitude, direct)` (zone)

`levelAltitude := altitude`. Then:

- `direct = true` (never used by the shipped code): `terrain.SetAltitude(clamped rect, altitude, 1, 5)` (slot 49)
  with the rect clamped to `[0, cellCountX] × [0, cellCountZ]`.
- `direct = false`: `buildingDevelopment.GetLotDeveloper().AttemptAreaLevelling(rect, 0.0, true, true,
  FLT_MAX, null)` (lot developer slot 21; result ignored). The rect is the **unclamped** argument (`push ebx` at
  `0x006BA78B`): the clamp above is computed on both paths but used only by the direct one.

  *Which `true` is which* (LF1): the call site pushes the literals `1` (`0x006BA787`) and `1` (`0x006BA785`) as the
  third and fourth arguments, `withWater` and `waterFlag`. Inside `0x006C44F0` the third argument is the one that
  reaches `GetAverageAreaAltitude` and `GetAreaSlope` (steps 1 and 3 below); the fourth goes only to
  `ModifyTerrainHeight`, as its **second** bool (step 4; the first is the literal `1`). The landfill's second
  `true` is therefore `waterFlag`. Because both are `true`, swapping them changes nothing the landfill can
  observe.

**What `AttemptAreaLevelling(rect, f, withWater, waterFlag, maxAreaSlope, lot)` actually does** *[compat]*
(`0x006C44F0`, pop 24; verified from the assembly — Ghidra's decompilation of this function is unreliable, it
renders the float argument as a pointer and drops the vertex average):

1. `avg := GetAverageAreaAltitude(rect, withWater)` (lot developer slot 18, `0x006C5B70`): the arithmetic mean of
   `GetTerrainAltitudeByVertex(x, z, withWater)` over **vertices** `x = x1 … x2+1` (outer), `z = z1 … z2+1`
   (inner), i.e. `Σ altitude / count`, both float32, count from an `FILD`.
2. `m := maxAreaSlope`. If `m` compares **equal** to `FLT_MAX` (`0x00AB1B00` = `0x7F7FFFFF`; `FUCOMPP` then
   `TEST AH, 0x44` / `JP`, so an **unordered** `m` — a NaN — also takes this branch): `m :=
   maxAreaSlopeAllowedForLevelling`, the lot developer's tuning `0xE900EA2A` (field `+0xE8`, shipped **40**). The
   landfill always passes `FLT_MAX`, so `m = 40`.
3. If `m < 1000.0` (`0x00A8FAF0`; a NaN `m` skips this test and goes straight to step 4): `s := GetAreaSlope(rect,
   withWater)` (slot 29, lot-developer §11.4). If `s > m` — strictly, both ordered and unequal (`FCOMP` then
   `TEST AH, 0x41` / `JNZ`, so a NaN `s` does **not** block): **return false and change nothing**.
4. Otherwise: call slot 9 (`+0x24`) of the object at lot-developer `+0x20` (global `0x00B43D00`,
   `cISC4Demolition`), a ten-argument terrain call whose last two arguments are the literal `5` and a null
   pointer, and return its result. The SDK names this `cISC4Demolition::ModifyTerrainHeight(bool, bool, int, int,
   int, int, float, cISC4Lot*, uint32_t, int64_t*)`, i.e. **reason 5** and no cost output; the rect arguments are
   `x1, z1, x2 + 1, z2 + 1` and the float is the step-1 average. **The terrain is levelled to `avg`.** The full
   argument list is `(1, waterFlag, x1, z1, x2 + 1, z2 + 1, avg, lot, 5, null)` (`0x006C4552`–`0x006C457D`).

*[quirk]* **The float argument `f` is never read** — no instruction in `0x006C44F0` touches the second parameter's
stack slot. Passing `0.0` is therefore harmless *and* meaningless: the target altitude is always the vertex
average computed in step 1.

Consequences for the zone *[compat]*:

- `LevelRect(rect, altitude, false)` still sets `levelAltitude := altitude` (the **cell-centre** mean from
  §4.3, sea-clamped), but the terrain is levelled to the **vertex** mean, which is a different number. The two
  disagree by however much the cell-centre and vertex averages differ, and `levelAltitude` (not saved, §4.4.9) is
  what §4.3 later prices extensions against.
- Because the return value is ignored, a rect whose `GetAreaSlope` exceeds 40 is **not levelled at all**: the zone
  is still created and populated, and `levelAltitude` still records a mean the terrain does not have. Zoning a
  landfill on steep ground therefore leaves the slopes in place while the zoning cost was priced as if they were
  removed.
- `ModifyTerrainHeight` ends with `terrain.RedisplayTerrain(1, 1, rect, 5)`, which posts `0x278E6003` with
  `data4 = 5` (§3.2) — outside the `{1, 2, 3}` set, so the landfill ignores its own levelling. No feedback loop.

### 4.6 Capacity

#### 4.6.1 `CalculateGarbageHeapCapacity(v)` (zone)

`v ≤ 0` → 0. Otherwise `round(float(u32(tileCap × cellCount)) / v)`: the product is a 32-bit `imul` (wraps),
converted as unsigned, one float32 division by `v`, then the magic round. Total zone capacity is therefore about
`tileCap` per mask cell whenever at least one heap is valid, and **0** for a zone with no valid heap (e.g. one
cell wide).

#### 4.6.2 `RecalculateCapacity()` (zone)

`v := GetNumValidHeaps()`; `c := CalculateGarbageHeapCapacity(v)`. For each heap (vector order): `capacity :=
valid ? c : 0`; if `contained > capacity` (unsigned): `pollution.AddToGarbageSupply(contained − capacity)` and
`contained := capacity` (excess garbage goes back to the supply; the `landfilled` counter is not reduced).
`totalCapacity := v × c`.

#### 4.6.3 `GetCapacity()` → `(available, contained, capacity)` (zone)

Sums over **all** heaps (valid or not): `contained := Σ contained`, `capacity := Σ capacity`,
`available := Σ (capacity − contained)` (32-bit wrap). `GetAvailableCapacity()` returns `available` as int32.

#### 4.6.4 `GetCellCount()` (zone)

Number of mask cells; cached, recounted when dirty.

#### 4.6.5 `UpdateCapacityValues()` (developer)

`capacityTotal := usedTotal := 0`. For each zone whose `connected` flag is set: `(a, c, k) := GetCapacity()`;
`usedTotal += c`, `capacityTotal += k`, each a u32 add that **saturates** at `0xFFFFFFFF`. Disconnected zones
contribute nothing.

**`GetLandfillUsage(usage&, capacity&)`** (slot 10): `capacity := capacityTotal`;
`usage := min(usedTotal, capacityTotal)`. Pollution's dispose step uses `capacity − usage` as the room for new
supply (pollution §4.6.1 step 5); the landfill gas recovery ordinance uses `capacity` (ordinance spec); the
neighbours simulator uses `capacity − usage` for landfill deals (neighbors spec).

**`GetLandfillTileCount()`** (slot 11): `zoneManager.GetZoneCount(14)`: the number of zone-14 cells in the city
(not limited to connected zones or mask cells). Pollution feeds it to its transport curve `0x8C19C3E9`.

### 4.7 Monthly simulation

#### 4.7.1 `DoMonthlySimulation()` (developer)

If the zone list is **empty**: record history `0x2A8C4696 := capacityTotal` and
`0x2A8C4697 := usedTotal + max(0, pollution.GetTotalGarbageUncollected()) + pollution.GetGarbageSupply()` with
the stale totals (§5); `UpdateBudgetRequirements()`; `pollution.SetGarbageCapacity(0)`. Done.

Otherwise:

1. `current := GetCurrentBudget()` (int64, line 30's current expenses, §4.9.2); `maint := maintenance`.
2. `t := pollution.GetGarbageSupply()` (u32, already ≥ 0); if `t > kMaxGarbageToTakePerMonth` (unsigned):
   `t := kMaxGarbageToTakePerMonth`.
3. If `maint > 0` and `current < maint` (signed int64): `t := trunc(float32(float32(current / maint) × t))`:
   one float32 division of the exact int64 values, one float32 multiplication by the int32 `t`, then truncation
   toward zero (`_ftol`).
4. `avail := 0`. For each zone in list order: `UpdateZone(&t)` (§4.7.2); if the zone is connected:
   `avail += zone.GetAvailableCapacity()` (int32 wrap).
5. `pollution.SetGarbageCapacity(avail)`; `UpdateBudgetRequirements()`; `UpdateCapacityValues()`.
6. History: `0x2A8C4696 := capacityTotal`; `0x2A8C4697 := usedTotal + max(0, GetTotalGarbageUncollected()) +
   GetGarbageSupply()` (u32 wrap; values read after step 4).

`pollution.garbageCapacity` (the "valve", pollution §4.6.1–§4.6.2) is therefore the free capacity of the connected
zones **after** this month's intake.

**The supply never clamps during the month** *[compat]* (LG12). Let `s` be the supply at step 2:

- `t ≤ s`;
- each heap's `n` is at most the zone's `budget`, which is at most `remaining`, which starts at `t`;
- so the intakes sum to at most `t`.

Every `AddToGarbageSupply(−n)` therefore stays within pollution's no-clamp case (pollution §8.1, slot 29), and the
supply after the month is exactly `s − Σ n`. The month makes no positive adds; those come only from
`RecalculateCapacity` on zoning changes (§4.6.2). `GetGarbageSupply` returns `max(0, supply)` and **stores** that
clamp, so it changes the value only when a damaged save left the supply negative.

Example: supply 50000, one connected 2 × 2 zone with an empty heap. Then `t = 50000` and the zone's budget is
`min(50000, 4 × 5000) = 20000`, so `n = 20000` and the supply ends at exactly 30000 (test 9).

#### 4.7.2 `UpdateZone(&remaining)` (zone)

1. `connected := IsZoneConnected()` (stored). If not connected: `AddZot()` (§4.7.5). If connected and a zot
   exists: remove it (`RemovePropA`) and release it.
2. `budget := min(remaining, GetCellCount() × kMaxGarbageToTakePerMonthPerTile)` (signed compare).
3. For each **valid** heap in vector order:
   1. `DecayGarbage()` (§4.7.3).
   2. `room := capacity − contained` (u32). `n := 0`; if `room ≠ 0` and `connected` and `budget > 0`:
      `n := min(room, budget)` (unsigned compare).
   3. `contained += n`; `pollution.AddToGarbageSupply(−n)`; `pollution.GarbagePutInLandfill(n)`;
      `remaining −= n`; `budget −= n`. (These calls are made with `n = 0` too.)
   4. If `contained = 0` and `active`: `active := false`, `UpdateSurroundingTiles(heap)` (§4.8). If
      `contained ≠ 0` and not `active`: `active := true`, `UpdateSurroundingTiles(heap)`.
4. `RedistributeGarbage()` (§4.7.4).

**`IsZoneConnected()`**: dilate the zone's cell region by one cell in the four orthogonal directions (cells only,
not diagonals); `ids := traffic.GetSubnetworksInRegion(dilated)` (slot 41). For each developer type of the table
`0x1010, 0x1020, 0x1030, 0x3110, 0x3120, 0x3130, 0x3320, 0x3330, 0x4200, 0x4300, 0x4100, 0x4400` (R$, R$$, R$$$,
CS$, CS$$, CS$$$, CO$$, CO$$$, ID, IM, IR, IHT): `counts := traffic.GetOccupantCountForAllSubnetworks(type)`
(slot 42); connected as soon as some `id ∈ ids` has `id < len(counts)` and `counts[id] ≠ 0`. So a landfill is
connected iff a transport subnetwork touching it reaches at least one developed RCI building.

#### 4.7.3 `DecayGarbage()` (heap)

If `contained > 0`: `d := round(float(contained) × kLandfillDecayRate)` (one float32 product, magic round). If
`d = 0`: `d := 1`; else `d := min(d, kMaxGarbageToDecayPerMonthPerZone)`. `d := min(d, contained)`;
`contained −= d`. The decayed garbage disappears (no counter). *[quirk]* The "per zone" cap applies to each heap.

#### 4.7.4 `RedistributeGarbage()` (zone)

`(·, total, cap) := GetCapacity()`. If `total > 0` and `v := GetNumValidHeaps() > 0`: `share := cap / v` (signed
integer division), `rem := total`; for each **valid** heap in vector order: `x := min(share, rem)`;
`contained := x`; `rem −= x`. Invalid heaps are not changed. The active flags are **not** updated here (they
catch up at the next `UpdateZone`).

*[quirk]* After a terrain dezone (§4.5.1), invalid heaps still hold capacity and garbage: both are counted in
`cap` and `total`, the valid heaps receive copies of the invalid heaps' garbage (exceeding their own capacity),
and the invalid heaps keep theirs, so garbage is duplicated until the next `RecalculateCapacity` returns the
excess to the supply.

#### 4.7.5 `AddZot()` (zone)

If no zot: spiral outward from the centre cell (clipped to the zone rect, centre first) to the first mask cell;
if none, use the centre cell. Position := cell centre, `y := terrain.GetAltitudeAtNearestGrid(x, z) + 3.0`.

**The spiral** *[compat]* (LF4; `0x006BB3B0`–`0x006BB4DA`). It is the same `SpiralIterator`, in the same mode
(clockwise), as plumbing.md §7.2 step 4. The settings are:

- Start: the centre `(cx, cz) := ((x0 + x1) / 2, (z0 + z1) / 2)`, by C integer division.
- Box: the zone rect `[x0, x1] × [z0, z1]`.

The order is `(cx, cz)`, then ring by ring.

- Ring r starts at `(cx + r, cz − r + 1)` and runs **+z** up the column `x = cx + r` to `(cx + r, cz + r)`.
- It then runs **−x** along the row `z = cz + r`, **−z** down the column `x = cx − r`, and **+x** along the row
  `z = cz − r`.
- It ends at the corner `(cx + r, cz − r)`.

Ring 1 is therefore `(cx+1, cz)`, `(cx+1, cz+1)`, `(cx, cz+1)`, `(cx−1, cz+1)`, `(cx−1, cz)`, `(cx−1, cz−1)`,
`(cx, cz−1)`, `(cx+1, cz−1)`.

Clipping is as in plumbing §7.2:

- Cells outside the rect are skipped one by one.
- Every rect cell is visited exactly once.
- The walk ends after ring `max(cx − x0, x1 − cx, cz − z0, z1 − cz)`.

The test at each cell is the zone's mask bit (`(x − x0) < W` and `(z − z0) < H` unsigned, then the bit). The
code tests `(cx, cz)` **before** the first `GetNext`, and that first `GetNext` returns `(cx, cz)` again. So the
centre is tested twice, which has no effect. If no cell has its bit set, the centre cell is used.

The hit is the first mask cell in this order. It is **not** "clockwise from each ring's top-left corner": that
order starts ring 1 at `(cx−1, cz−1)`.

Example: a 3 × 3 zone over `(10..12, 20..22)` whose centre cell `(11, 21)` has been dezoned (mask bit clear), with
every other bit set. The centre is `(11, 21)`, its bit is clear, and the first ring-1 cell is `(12, 21)`, which is
set. **The zot goes to cell `(12, 21)`.** A top-left-corner order would give `(10, 20)`.

Example with clipping: a zone over `x 10..14, z 20..22` with only `(10, 20)` and `(14, 22)` set. The centre is
`(12, 21)`, and none of ring 1's eight cells is set. Ring 2 clipped to the rect is `(14, 20)`, `(14, 21)`,
`(14, 22)`, `(10, 22)`, `(10, 21)`, `(10, 20)`, because its `z = 23` and `z = 19` rows are skipped. **The zot goes
to `(14, 22)`.**
`zot := propManager.AddLotProp(0x107A0000, position, orientation)` (slot 26, the `(propID, pos, orientation)`
overload; `orientation` is read from a global view service) — `AddRef`, store, then `SetRemovalFlags(3)` (prop
occupant slot 12). *[quirk]* No null check: a missing prop exemplar crashes. The zot is saved by reference.

**The orientation** *[compat]* (LG5; `0x006BB510`–`0x006BB524`) is the **current view rotation**, 0 to 3. The code
reads field `+0x10` of the global `SC4ViewUtilities` object (`0x00B43DD8`). `cSC4WinRegionScreen` fills that global
from its renderer's `GetViewUtilities()` (`cISC43DRender` slot 8). `SC4ViewUtilities::UpdateCameraPosition`
copies the field from the camera control's rotation (`+0x10C`) and uses `rotation × π/2` as the camera yaw.
Nothing checks the global for null *[quirk]*. So the zot faces whichever way the player's view faced in the month
it was placed. The value is saved in the prop's record (lot-records.md §10.1, field 5), and it is the only
observable effect.

No local save holds a landfill zot, because every saved landfill zone is connected. All 66 `0x107A0000` props in the
saves are building zots placed through the same global (lot-developer.md §5.6), and all have orientation 0.

*[choice]* A FreeCity4 core with no view uses 0, which matches the original whenever the view is at its first
rotation. Glue running inside the game can read the same field.

**Reaching the rotation through interfaces** *[compat]* (LG21; `SL::OnView3DInit` `0x00602590`). The global is
filled from the city view by two interface getters, so glue can follow the same route:

1. The `cISC4View3DWin` (IID `0xFA47B3F9`), found as in §3.2 (main window, child `0x6104489A`, then
   `GetChildAs(0x9A47B417, 0xFA47B3F9, &view)`).
2. `view.GetRenderer()`: `cISC4View3DWin` slot 4 (`+0x10`, `0x0043FD00`, pop 0). It returns the
   `cISC43DRender*` the view holds, **borrowed** (no `AddRef`).
3. `renderer.GetViewUtilities()`: `cISC43DRender` slot 8 (`+0x20`, vtable `0x00ABAB60`, `0x007C9FB0`, pop 0). It
   returns a pointer to the view utilities object, also borrowed. That object is **not** a GZCOM object: it has no
   `QueryInterface` and must not be released.
4. The rotation is the 32-bit integer at `+0x10` of that object, 0 to 3.

When a city view opens, `SL::OnView3DInit` stores the result of step 2 in the global `0x00B43DD0` and the result
of step 3 in `0x00B43DD8`, the global that `AddZot` reads. `SL::OnView3DShutdown` (`0x00602130`) clears both. So
in a loaded city, steps 1–4 give exactly the value the original reads. Neither interface has a getter that
returns the rotation itself: `cISC43DRender` and `cISC4View3DWin` have only `SetRotation`, `RotateLeft` and
`RotateRight`. The field read in step 4 is the only route.

Example: the player has turned the view so that step 4 reads 1. A zot placed in that month is saved with
orientation 1. A core that uses 0 saves 0 for the same zot.

*[choice]* Glue should read the rotation by steps 1–4 at the moment it places the zot. If the view is null, it
should use 0 instead of crashing.

**When the rotation is read** (LG30). The global caches the **object**, not the rotation: `AddZot` loads the
pointer from `0x00B43DD8` and reads `+0x10` at placement (`0x006BB510`, `0x006BB51F`). So the original also uses
the rotation current when the zot is placed, and a read at placement through steps 1–4 gives the same value. Only
a view that is not open differs: the original then reads through a null pointer, the glue uses 0.

### 4.8 Tile models (`UpdateSurroundingTiles(heap)`) and the random draws

A heap's state change redraws the four cells it covers, in the order `(hx, hz)`, `(hx+1, hz)`, `(hx, hz+1)`,
`(hx+1, hz+1)` (absolute = zone origin + local). For a cell at local `(cx, cz)`:

`mask := 1·A(cx−1, cz−1) + 2·A(cx−1, cz) + 4·A(cx, cz) + 8·A(cx, cz−1)`, where `A(i, j)` = heap `(i, j)` is
active (0 outside `0 ≤ i < W`, `0 ≤ j < H`).

Rule table (mask → tile, orientation):

| Mask | Tile | Or. | Mask | Tile | Or. |
|---|---|---|---|---|---|
| 0 | `0x02F10000` | 0 | 8 | `0x034E0000` | 0 |
| 1 | `0x034E0000` | 3 | 9 | `0x02F90000` | 0 |
| 2 | `0x034E0000` | 2 | 10 | `0x034F0000` | 2 |
| 3 | `0x02F90000` | 3 | 11 | `0x02FB0000` | 0 |
| 4 | `0x034E0000` | 1 | 12 | `0x02F90000` | 1 |
| 5 | `0x034F0000` | 0 | 13 | `0x02FB0000` | 1 |
| 6 | `0x02F90000` | 2 | 14 | `0x02FB0000` | 2 |
| 7 | `0x02FB0000` | 3 | 15 | `0x02FA0000` | **2 × (rand() & 3)** |

For each cell: look the rule up; if its tile is `0x02FA0000` (mask 15 only), draw **one** C `rand()` (foundations
§12; the process-wide CRT generator) and set the orientation to `2 × (rand() & 3)` ∈ {0, 2, 4, 6} (the value is
written into the static rule table entry, harmless because it is rewritten before every use). Then
`RemoveLandfillTile(x, z)` and `CreateLandfillOccupant(x, z, tile, orientation)`.

**Random draws** *[compat]*: one `rand()` per redrawn cell whose four surrounding heaps are all active, so 0–4 per
heap state change, at most 4 × (number of heaps whose `active` flag changes) per zone per month. Nothing else in
these classes draws random numbers.

**Which generator** (LG19). `UpdateSurroundingTiles` (`0x006BCFC0`) calls the C runtime's `rand()` (`0x009EFB9F`) at
`0x006BD201`, `0x006BD271`, `0x006BD2DD` and `0x006BD34B`, one site per redrawn cell. It reduces the result with
`and 0x80000003` and a sign fix, which is `rand() mod 4`, the same as `& 3` because `rand() ≥ 0`, then doubles it.
That `rand()` is the per-thread CRT generator of foundations §12.1. The landfill month runs on the simulation thread,
so the draws share one sequence with every other `rand()` caller on that thread, and the values cannot be
reproduced from a save (foundations §12.2).

The rule-table entries are 12 bytes, `{mask, tile, orientation}`, at `0x00B0D2D0`. Before the first cell, the code
also tests entry 0's tile against `0x02FA0000` (`0x006BD1BF`–`0x006BD1E0`). Entry 0 holds `0x02F10000`, so that test
never draws.

*[choice]* FreeCity4's own `CrtRand` per subsystem, seeded 1 (foundations §12.6), is the only reproducible option.
It keeps the draw count and the formula. Only the values differ from a live game, and those cannot be matched
anyway.

#### 4.8.1 `CreateLandfillOccupant(x, z, tile, orientation)` → bool

1. `RemoveIncompatibleOccupants(x, z)`.
2. If `tile = 0x02F10000` (empty): if no base-texture occupant is at the cell, `SetEmptyCellTexture(x, z)`.
   Return true. No occupant is created; the landfill flag stays cleared.
3. Otherwise: load the tile exemplar `(0x6534284A, 0x07BDDF1C, tile)` (*[quirk]* a missing exemplar crashes, see
   below); create a `cSC4LandfillOccupant(tile, orientation)`, attach the exemplar to its property holder,
   `SetFlag(4)`, `SetRemovalFlags(2)`; `pollution.SetIsActiveLandfill(x, z, true)`; `isEmpty := false`; position
   := cell centre with `y = altitude(x, z)`; `SetPosition`; `Init`; insert into the occupant manager
   (slot 20, `(occupant, 0)`) and return its result.

**Step 3 in detail** *[compat]* (LG3; `0x006BC7CF`–`0x006BC92A`):

1. `ex := resourceManager.GetResource({0x6534284A, 0x07BDDF1C, tile}, 0xA52160F5, &ex, 0, 0)` (resource manager
   `0x00B43CA8`, slot 3; `0xA52160F5` is `cISCResExemplar`). If `ex` is null, the code jumps to step 8 with a null
   occupant and crashes there (`0x006BC911`) *[quirk]*.
2. `o := new cSC4LandfillOccupant(tile, orientation)` (`0x00477270`). This constructor stores the tile ID and the
   orientation, sets type `0x6A11BD2B` and calls `SetFlag(4)`. Then `o.AddRef()`.
3. Attach the exemplar: `SetDefaultExemplar(ex)` (slot 3, `0x00458E60`) on the occupant's
   `cISCExemplarPropertyHolder`, the sub-object at `+0x20`. The developer calls it **directly** on the object it has
   just constructed (`0x006BC845`–`0x006BC84C`), with no QueryInterface (LG27). It takes a reference to `ex`, stores
   it and releases any exemplar held before.
4. `SetFlag(4)` again, `SetRemovalFlags(2)`, `pollution.SetIsActiveLandfill(x, z, true)`, `isEmpty := false`.
5. `(px, pz) := CellCenterToPosition(x, z)`; `py := terrain.GetAltitude(px, pz)`, stored as float32;
   `SetPosition((px, py, pz))`.
6. `o.Init()`. `cSC4Occupant::Init` (`0x005E8C90`) posts `0x99EF1140`, and the queued message holds a counted
   reference to the occupant (flora §9.1).
7. Release `o` (the reference from step 2), then release `ex`. *[quirk]* Between here and step 8 the occupant is
   kept alive only by the reference in step 6's queued message. A replacement should keep its own reference until
   the insert.
8. Return `occupantManager.InsertOccupant(o.AsOccupant(), 0)` (slot 20).

**When the insert returns false** (LG29, occupant manager slot 20, `0x0049EF60`), in this order: (1) the occupant is
null; (2) its flag 1 (bit 0, in an occupant manager) is already set, tested before anything else (`0x0049EF8A`); (3)
after `AddRef` and `Init` (a no-op for an initialised occupant, `0x005E8C90`), `AsPropertyHolder()` (slot 6) returns
null, which no `cSC4Occupant` class does (`0x0049EFBB`); (4) the occupant is already in the list of the first cell it
would be listed in, its low x and low z cell (`0x0049F158`–`0x0049F182`). Nothing else fails: not the type (never
read), not the initialised state (the insert initialises), and not the box (a cell range that comes out empty succeeds
and lists nothing; for cells outside the index see flora §6.4.2). On success it sets flag 1 and posts `0x99EF1142`
with its second argument (`0x0049F200`–`0x0049F214`). After failure (3) or (4) its reference is released again, but
the `Init` and the stored cell rectangle remain. A saved record's flags byte keeps bit 0 (lot-records.md §2.3), so an
occupant read from one is refused by (2) unless the bit is cleared first; flora's obliteration save clears it around
its `Write` (flora §10.3).

**After a failed insert** *[compat]* (LG26; callers `0x006BD234`, `0x006BD2A4`, `0x006BD310`, `0x006BD37E`;
`cSC4Occupant::Shutdown` `0x005E8980`, `cSC4Occupant::Release` `0x005E8DF0`). The original does nothing more:

- **For a new tile the insert cannot fail.** The occupant is not null (a null exemplar crashes before this,
  step 1). Its flag 1 is clear, because the constructor sets only flag 4. Its property holder exists. It is in no
  cell list yet.
- **No caller tests the result.** `CreateLandfillOccupant` returns the insert's result, and all four calls in
  `UpdateSurroundingTiles` ignore it.
- **There is no `Shutdown`.** Had an insert failed, the occupant would be freed when the queued `0x99EF1140`
  message (step 6) dropped the last reference. `cSC4Occupant::Release` frees the object at count 0 without calling
  `Shutdown`, and so without the `0x99EF1141` message that `Shutdown` posts for an initialised occupant.
  Subscribers would see the tile created and never destroyed.
- **Nothing keeps the tile.** The landfill keeps no list of its tiles: it finds them through occupant manager
  slot 33 (§4.8.6). After a failed insert, that lookup finds no tile, so the cell costs the default 5 (§4.9.1)
  and terraforming dezones it (§4.5.2), until the heaps next change.

Example: if the occupant manager refused the tile at cell (12, 21), the original's monthly maintenance would
count 5 for that cell, not the tile's 10 to 40.

**The bracket**: "released without `Shutdown`" matches the original. "The tile is kept in FreeCity4" has no
counterpart, because the original keeps no tile of its own. The case never arises in the original, so it
cannot be compared. *[choice]* Keeping FreeCity4's tile and logging the failure is acceptable.

Dead branch: `0x006BC88D`–`0x006BC8B4` handles `tile = 0x02F10000`. It calls `SetRemovalFlags(1)` and
`SetIsActiveLandfill(x, z, false)`, sets `isEmpty := true`, then calls `SetEmptyCellTexture`. Step 2 of
`CreateLandfillOccupant` has already returned for that tile, so this branch never runs. It is the only code found that
sets `isEmpty` to true (§10.2 question 7).

**The tile's interfaces** (LG27, from the exe; constructors `0x00477270` and `0x004774C0`, base constructor
`0x005E9220`). The object is laid out as follows:

| Offset | Interface | vtable | QueryInterface answers |
|---|---|---|---|
| `+0x00` | `cISC4LandfillOccupant` | `0x00A8CE28` | `0x2A15F262` |
| `+0x04` | `cISC4Occupant` (the `cSC4Occupant` base) | `0x00A8CDA0` | `1`, `0x26BB767E` |
| `+0x08` | `cIGZSerializable` | `0x00A8CD84` | `0xE4FDA3D4` |
| `+0x0C` | `cISCPropertyHolder` (the base's exemplar property holder) | `0x00A8CD20` | none |
| `+0x20` | `cISCExemplarPropertyHolder` (the same holder) | `0x00A8CD08` | none |

Every sub-object's QueryInterface is a thunk (`0x00477210`, `0x00477230`, `0x00477240`, `0x00477260`) to the one at
`0x00477130`, which answers only the four IIDs above and returns false for any other without writing `*out`. So
**nothing answers `0x0AC2B5F7`** through QueryInterface: not the occupant, not the property holder that
`cISC4Occupant::AsPropertyHolder` (slot 6, `0x006C2040`: occupant + 8) returns, and not the holder itself. The holder
keeps the exemplar at its `+4` (object `+0x24`); slot 4 `GetDefaultExemplar` (`0x0060CE80`) returns it without a
reference. `cSC4Occupant`'s constructor fixes where the holder sits (property holder at occupant + 8, exemplar-holder
vtable at occupant + 0x1C), so it is the same in every occupant class built on it: the exemplar holder is
`AsPropertyHolder()` + 0x14 = `cISC4Occupant*` + 0x1C = `cISC4LandfillOccupant*` + 0x20.

**The holder's slots** *[compat]* (LG25). The `cISCExemplarPropertyHolder` vtable has **five** slots. The dword
after slot 4 (`0x00A8CD1C`) is 0, and the occupant's `cISCPropertyHolder` vtable starts right after it, at
`0x00A8CD20`. The building occupant's holder (`0x00A871A0`, building-development.md §4.9.1) has the same
slots 1–4 and its own `QueryInterface` thunk.

| Slot | Off | Address | Method | Pop | Return | Behaviour |
|---|---|---|---|---|---|---|
| 0 | +0x00 | `0x00477260` | `QueryInterface(iid, out)` | 8 | AL | `this − 0x20`, then the occupant's QI `0x00477130` (LG27) |
| 1 | +0x04 | `0x005E06A0` | `AddRef()` | 0 | EAX | `this − 0x20`, then the occupant's `AddRef` |
| 2 | +0x08 | `0x005E06B0` | `Release()` | 0 | EAX | `this − 0x20`, then the occupant's `Release` |
| 3 | +0x0C | `0x00458E60` | `void SetDefaultExemplar(cISCResExemplar*)` | 4 | none | `AddRef`s the new exemplar, stores it at `+4`, releases the old one; nothing if it is the same pointer |
| 4 | +0x10 | `0x0060CE80` | `cISCResExemplar* GetDefaultExemplar()` | 0 | EAX | returns `+4`, borrowed (no `AddRef`); null when none is attached |

The landfill developer calls only slot 3. A declaration that stops at slot 3 or 4 calls the game correctly.
An object that the game is to call as this interface must have all five slots.

**`SetTileID` is not enough** (LG3). `SetTileID` (slot 6, `0x00477450`) neither loads nor attaches an exemplar.
When `id` differs from the stored ID, it stores `id` and rewrites the instance in the key of the **attached**
exemplar (`GetKey` and `SetKey`, exemplar slots 11 and 12):

- With no exemplar attached, it dereferences null *[quirk]*.
- With another tile's exemplar attached, that exemplar keeps its own properties (cost `0xEA54D286`, models) under
  the new key. Every other holder of the cached exemplar sees the new key too (§7.3).

*[choice]* To build a tile from outside the class, run steps 1–8 in the same order:

1. Create the occupant with GZCOM `CreateInstance(0x8A22BEBE, 0x2A15F262)` (factory `0x004776B0`). This uses
   the default constructor `0x004774C0`: tile `0x02F10000`, orientation 0, the same type `0x6A11BD2B` (`0x00477553`)
   and, from the `cSC4Occupant` constructor, flag 4 (LG28). It differs from the tile constructor only in the tile and
   the orientation.
2. Attach the exemplar with `SetDefaultExemplar(ex)` (slot 3) on the holder at `cISC4LandfillOccupant*` + 0x20, as the
   original does (LG27). QueryInterface cannot reach it (above).
3. Call `SetTileID(tile)`. It is now harmless, because the key already has that instance.
4. Call `SetOrientation(orientation)` (slot 8), then continue from step 4 (its `SetFlag(4)` is a no-op, LG28).

The layout-free alternative is the load path: QI the new occupant for `cIGZSerializable` (`0xE4FDA3D4`) and `Read` a
§6.4 record from a memory stream, with the holder key (group `0x07BDDF1C`, type `0x6534284A`, instance `tile`) and a
flags byte with bit 0 clear (LG29). The holder's `Read` (`0x005FF420`) then fetches the exemplar from the resource
manager itself, as for a tile in a loaded city (foundations §10.3), and the record also sets tile, orientation,
position and `isEmpty`. It differs in one way: the base record marks the occupant as loaded, so its `Init` does not
post `CreateOccupant` (`0x99EF1140`) as a new tile's does (lot-records.md §2.4). The layout route is therefore
recommended.

**Who sees the tile occupants.** The pollution flag is what pollution reads. The tile occupants are also visible
elsewhere, so tiles kept only inside FreeCity4 differ from the original:

- The occupant manager saves them as class `0x8A22BEBE` (§6.4).
- The flora blocking filter accepts them (flora §6.4.1). Without them, flora may be planted on a tiled cell.
- `PlaceZone` asks the landfill occupant at a cell `CanRemove(2)` (zones.md §6).
- The landfill's own reads use them: `GetMaintenanceCost` reads each tile's cost from the tile occupant
  (§4.9.1), and `PopulateZone` and `HandleCellTerrainChange` test whether a tile occupant is there (§4.4.5,
  §4.5.2).
- The model maker draws them.

#### 4.8.2 Effect on pollution's landfill flag

Bit 15 of a cell's garbage value is set when a non-empty tile occupant is created there and cleared when the
tile is removed (§4.8.3). It is set only on cells of zone type 14 (pollution slot 50 ignores other cells). A
cell with mask 0 (no active neighbour heap) has no occupant and no flag.

#### 4.8.3 `RemoveLandfillTile(x, z)`

If a landfill occupant is at the cell: remove it (occupant manager slot 21, `(occupant, 1, 3)`). Then
`pollution.SetIsActiveLandfill(x, z, false)` (always).

#### 4.8.4 `RemoveIncompatibleOccupants(x, z)` and `RemoveBaseTexture(x, z)` (zone)

*[compat]* (LG1). Both list the occupants at the cell through occupant manager slot 41 (`+0xA4`). The query
passes the cell as both corners, a type filter and −1 as the last argument. Each listed occupant is then removed
with slot 21, `RemoveOccupant(occupant, true, 3)`, the same arguments as §4.8.3:

- `RemoveIncompatibleOccupants` (`0x006BC630`) lists type `0x74758926` (flora) and removes every entry, in list
  order (`0x006BC6D8`–`0x006BC6DD`). Slot 48 (`+0xC0`) then releases each entry and empties the list.
- `RemoveBaseTexture` (`0x006BC930`) lists type `0x497F6D9D` and asks each entry for
  `cISC4LotBaseTextureOccupant` (`0x097F6D98`). Only an entry whose `GetUsage()` (that interface's slot 4) is
  **1** is removed, through its `AsOccupant()` (slot 3) (`0x006BCA14`–`0x006BCA1F`). Usage 1 is what
  `SetEmptyCellTexture` gives the landfill texture. A base texture with any other usage, such as a lot's, stays.
  The function releases the interface and each listed occupant itself, then frees the list.

`RemoveOccupant(occ, true, 3)` (`0x0049F240`, pop 12, returns AL) does this, in order:

1. Take a reference to `occ`.
2. If `occ.IsFlagSet(1)` (the occupant is in the manager):
   - take it out of the cell buckets;
   - call `occ.PostOccupantMessage(0x99EF1143, 3)` (flora §9.1: posted, data3 = the occupant type,
     data4 = **3**);
   - clear flag 1;
   - release the manager's reference.
3. The second argument is true, so call `occ.Shutdown()` (slot 4). This also happens when step 2 did not run.
4. Release the reference from step 1.

It returns true if and only if step 2 removed the occupant. Subscribers see the removal with data4 = 3 on a
later message-server tick.

#### 4.8.5 `SetEmptyCellTexture(x, z)` (zone)

*[compat]* (LG2; `0x006BA470`, pop 8, no return value). The function gives one cell the landfill ground texture.
It does not check for an existing texture; its callers do (§4.4.5, §4.8.1). In order:

1. Build one texture record, the 14-byte record of lot-records.md §6:

   | Field | Value |
   |---|---|
   | texture instance | `0x1005` (4101), a constant in the code. It equals entry 14 of the zone manager's texture array (zones.md §4, `0xE94825B6`), but that array is not read. |
   | cell x, z | the low bytes of `x` and `z` |
   | orientation | 0 |
   | configuration object byte 3 | 0 |
   | five unnamed bytes | `FF FF FF`, then `zoneManager.GetZoneDisplayAlpha()` (zone manager slot 16; shipped 170 = `0xAA`), then `01` |
   | animation frame | 0, so `SetTextureSpecification` makes **no random draw** (lot-records.md §6 draws only for `0xFF`) |

2. `GZCOM().CreateInstance(0xC97F987C, 0x097F6D98, &tex)`. If that fails, construct a `cSC4LotBaseTextureOccupant`
   directly (`0x60` bytes, `0x005E05B0`). If that also fails, the code dereferences null *[quirk]*.
3. `tex.AsOccupant().Init()` (base-texture slot 3, then occupant slot 3).
4. `tex.SetUsage(1)` (slot 5). Usage 1 marks a landfill texture, which `RemoveBaseTexture` tests (§4.8.4); lots use
   2.
5. `tex.SetTextureSpecification(&record, 1, null)` (slot 8).
6. Let `occ` be `tex.AsOccupant()`. If `occ.IsFlagSet(1)`: occupant manager slot 24 `(occ, 0)`, then
   `occ.PostOccupantMessage(0x99EF1145, 0)`. Otherwise occupant manager slot 20 `InsertOccupant(occ, 0)`. A new
   occupant takes the second branch. The result is not checked.
7. Release `tex`.

The texture occupant is saved by the occupant manager as a class `0xC97F987C` record (lot-records.md §6), so it
is visible in a save. Every usage-1 base texture in the eleven local saves with buildings (94 in all, in
Konradshohe, Fulham and the Big City Tutorial) holds exactly one record, texture `0x1005`, with the bytes
`00 00 FF FF FF AA 01 00` after `x, z`.

Example: `SetEmptyCellTexture(37, 52)` with display alpha 170 gives the record
`05 10 00 00 | 25 | 34 | 00 | 00 | FF FF FF AA 01 | 00`.

**The record in memory** *[compat]* (LG22; `SetTextureSpecification` `0x005E0B20`, `Write` `0x005DF1E0`,
`SetEmptyCellTexture` `0x006BA470`). `SetTextureSpecification(records, n, …)` takes an array of `n` records with a
**stride of 16 bytes**. Each record holds the 14 saved bytes in the saved order, then two padding bytes:

| Offset | Size | Field |
|---|---|---|
| `+0x0` | 4 | texture instance |
| `+0x4` | 1 | cell x |
| `+0x5` | 1 | cell z |
| `+0x6` | 1 | orientation |
| `+0x7` | 1 | configuration object byte 3 |
| `+0x8` | 5 | the five unnamed bytes |
| `+0xD` | 1 | animation frame (`0xFF` = draw one, lot-records.md §6) |
| `+0xE` | 2 | padding |

- `SetTextureSpecification` steps through the array 16 bytes at a time, tests the frame byte at `+0xD`, and copies
  each record into the occupant's vector as four 32-bit words, padding included.
- `Write` writes `+0x0` as a u32 and `+0x4` to `+0xD` as ten single bytes. It never writes `+0xE` or `+0xF`.
- Nothing in the class reads `+0xE` or `+0xF`. The original's own record leaves them unset: `SetEmptyCellTexture`
  builds the record in a 16-byte stack buffer and writes only bytes `+0x0` to `+0xD`.

So any value in the padding gives the same result. The record above, as passed in memory:
`05 10 00 00 25 34 00 00 FF FF FF AA 01 00 ?? ??`, where `??` is anything.

*[choice]* One record in a 16-byte buffer with zeroed padding, as FreeCity4 does, is equivalent to the original.

#### 4.8.6 Cell lookups (occupant manager slot 33)

*[compat]* (LG4). "A landfill occupant (or base texture) is at the cell" in §4.4.5, §4.5.2, §4.8.1, §4.8.3 and
§4.9.1 means occupant manager slot 33 (`+0x84`, `0x0049DCE0`, pop 16), `GetOccupantAtCell(&out, x, z, type)`:

1. If the manager is not initialised, return false.
2. Otherwise store `type` in the manager's own type filter and tail-jump to slot 32 (`0x0049E690`) with that
   filter.
3. Slot 32 walks the occupants of cell `(x, z)`'s bucket in bucket order. It returns the **first** occupant that
   passes the filter and whose bounding box overlaps the cell's square in x and z.
   - The square is `16x + 0.1 … 16x + 16 − 0.1` by `16z + 0.1 … 16z + 16 − 0.1` (float32).
   - The overlap is four strict comparisons; y is not tested.

**Ownership.** On a hit, slot 32 writes `*out` and calls **`AddRef`** on it (`0x0049E7DD`), then returns true. The
caller owns that reference and must release it:

- `getLandfillTileCost` releases it at `0x006BA454`.
- `CreateLandfillOccupant` releases the base texture it found at `0x006BC7AB`.

On a miss, `*out` is **not written** and the function returns false. The landfill's callers set `*out` to null
first (for example `0x006BC794`).

The glue's bracket uses the slot 41 list instead. That is equivalent for the presence tests, provided the list is
released through slot 48 (§4.8.4). For the cost lookup (§4.9.1) it is equivalent while a cell holds at most one
landfill occupant, which `CreateLandfillOccupant` ensures by removing the old tile first (§4.8).

### 4.9 Costs and budget

#### 4.9.1 `GetMaintenanceCost()` (zone)

`Σ over mask cells of tileCost(x, z)` (int64), where `tileCost` is the first s64 value of property `0xEA54D286`
of the landfill occupant at the cell, or `kDefaultLandfillTileCost` when the cell has no landfill occupant (empty
cells, including cells whose tile is mask 0). Shipped: an empty cell costs 5, a tile 10–40 (§4.1).

**Where the tile cost comes from** *[compat]* (LG11; `getLandfillTileCost`, `0x006BA3C0`, pop 8, EDX:EAX). For
each mask cell:

1. Take the occupant manager from the **global** city (`0x00B43CEC`, slot 34) and hold it.
2. Look up the landfill occupant at the cell (slot 33, type `0x6A11BD2B`, §4.8.6).
3. If there is none, the cost is `kDefaultLandfillTileCost`.
4. Otherwise the cost is `cSCPropertyHelp::GetPropertyValue(occupant.AsPropertyHolder(), 0xEA54D286, &v)` (occupant
   slot 6), as an int64. It starts as `kDefaultLandfillTileCost` and is kept if the property is missing. The
   occupant's holder has no properties of its own (every saved tile has none, §6.4), so the value comes from its
   default exemplar, **the tile exemplar** attached in §4.8.1 step 3. Then release the occupant.
5. Release the occupant manager.

So the cost **is** read from the tile exemplar, through the occupant. A modded tile exemplar changes the
maintenance.

Examples with the shipped exemplars:
- a 2 × 2 zone whose one heap is active has four corner tiles `0x034E0000`, so 4 × 10 = **40** (test 6);
- a 3 × 3 zone with all four heaps active has four corners (10), four edges `0x02F90000` (20) and the centre
  `0x02FA0000` (40), so **160**.

With the default for every cell, those zones would cost 20 and 45.

#### 4.9.2 `GetCurrentBudget()` and `GetEfficiency()`

- `GetCurrentBudget()` (slot 8) = `department.GetLineItem(30).GetCurrentExpenses()` (int64). The department is
  not null-checked *[quirk: crash before PostCityInit]*. `current = trunc(full × fundingFraction)` (budget §7).
- `GetMaintenanceCosts()` (slot 7) = `maintenance`.
- `GetEfficiency()` (slot 5): if `maintenance > 0` (signed int64): `float32(current / maintenance)` (one float32
  division of the exact int64 values; not clamped, so up to the department's maximum funding, 1.2 for
  sanitation); else 1.0. Returned through a hidden result pointer (§7.1).

#### 4.9.3 `UpdateBudgetRequirements()`

`sum := Σ zone.GetMaintenanceCost()` (int64, saturating at `INT64_MAX`). `department.GetLineItem(30)` → `AddRef`,
`SetFullExpenses(sum)`, `Release` (no null checks). `maintenance := sum`. Line item 30 of the sanitation
department (created by the pollution simulator, pollution §7) is the landfill line.

### 4.10 Effects (rendering only, not saved)

`InitZoneEffects` creates two automata controllers through the city's automata controller manager (named by two
string statics, presumably the `landfillzone_attract` / `landfillzone_out` entries of `landfillzone.lua`
*[inferred]*), placed at the first road cell next to the zone (`GetFirstRoadTileNeighbor`), and a
"flock_of_seagulls" visual effect at the centre cell, 64 m above the terrain. `ShutdownZoneEffects` releases them.
FundsChanged enables or disables the truck automaton (§3.2); PreViewLevelChange starts/stops the seagulls.
FreeCity4 may implement these as it likes; they affect no simulation value.

## 5. Messages posted

| Message | When | Data |
|---|---|---|
| `0x89EFA536` history record (foundations §5, history warehouse) | each month (§4.7.1), sent synchronously (`MessageSend`) on a new `cRZMessage2Standard` | data1 = `0x2A8C4696` (landfill capacity), data2 = `capacityTotal`; then data1 = `0x2A8C4697` (total garbage: landfilled + uncollected + waiting), data2 = the sum (u32). Names unnamed in the symbols. |

Indirect: `DezoneCell(changeZone = true)` makes the zone manager send `0x07699455`; the occupant manager and prop
manager send their own insert/remove messages.

## 6. Persistence

### 6.1 Developer record (`cIGZSerializable`, CLSID `0xEA09D4AE`)

```
u16   version = 3
s32   zoneCount
zoneCount × zone record (§6.2)
```

- `Write` returns true always.
- `Read`: sets `loaded := true` **before** checking the version; version ≠ 3 → return false. Reads the count (on
  a stream error the count is uninitialised *[quirk]*). The zone list is cleared **without** releasing the old
  zones *[quirk: only matters on a second Read]*. For each zone: construct an empty zone, `AddRef`, read it; on
  success append it; on failure the zone is leaked and skipped. Returns **true** regardless of stream errors.
- Recomputed on load: everything in §2.1 except `zones`, on PostCityInit (§3.2), which also `Init`s each zone.

### 6.2 Zone record (written inline)

```
u16   version = 4
s32   x0, z0, x1, z1
(x1−x0) × (z1−z0) × u8  mask: for x in x0 … x1−1 (outer), z in z0 … z1−1 (inner): 1 if mask cell else 0
s32   W                  (= x1 − x0)
s32   H                  (= z1 − z0)
W×H × heap record        in vector order (index hz × W + hx)
u8    connected          (0/1)
ref   zot                (foundations §2.7; class cISC4Occupant 0x26BB767E on read)
```

Heap record: `u16 2; u32 hx; u32 hz; u16 valid (0/1); u16 active (0/1); u32 contained; u32 capacity`.

Read:

- The version is **not** checked (any value is accepted); version 3 reads `connected` and no zot; versions ≥ 4
  read `connected` (u8, true iff ≠ 0) and the zot reference; versions < 3 read neither.
- The region is created with **all cells set**, then the `(x1−x0) × (z1−z0)` bytes are applied (byte = 1 → set,
  else clear). *[quirk]* The last column (`x = x1`) and last row (`z = z1`) are never written, so they load as
  mask cells even if they had been dezoned. They then count for maintenance and per-zone intake; no texture or
  tile is created for them (no `PopulateZone` on load).
- `W × H` heap records follow; each must have version 2, else the zone `Read` returns false (the partially read
  heap is still appended with the previous heap's values *[quirk]*). `valid` and `active` are true iff the u16
  equals 1.
- `levelAltitude` is not saved (§4.4.9).
- Returns `!stream.GetError()`.

### 6.3 Byte example

A 2×2 zone at cells (10..11, 20..21), one heap (valid, active, 10000 of 120000), connected, no zot:

```
04 00 | 0A 00 00 00 | 14 00 00 00 | 0B 00 00 00 | 15 00 00 00 | 01 |
01 00 00 00 | 01 00 00 00 |
02 00 | 00 00 00 00 | 00 00 00 00 | 01 00 | 01 00 | 10 27 00 00 | C0 D4 01 00 |
01 | 00 00 00 00
```

### 6.4 Landfill occupant record (`cSC4LandfillOccupant`, CLSID `0x8A22BEBE`)

Saved by the occupant manager as a class-store object (foundations §3.3).

```
u16   version = 3
      cSC4Occupant base record:
        u16  version = 4
        u16  occupant word (shared base field)
        u8   highlight (GetHighlight; read back as (b << 4) | 2)
        u8   flags & 0xDD
        u32  occupant type (0x6A11BD2B)
        4 × u8
        exemplar property holder (foundations §10.3)
u32   tileID
s32   orientation
f32   position x, y, z
u8    isEmpty (0/1)
```

Read requires version 3 (and 4 for the base record), sets `isEmpty := (byte = 1)`, recomputes the bounding box,
returns `!GetError()`. The nested holder's rejection is **not** another rejection condition at the occupant
base layer; the exact order matters for malformed records (LG33).

**Reader layers and failure state** *[compat]* (LG33; instruction-checked, 2026-10-23):

| Layer | Calls and acceptance | State on rejection |
|---|---|---|
| Landfill `Read`, `0x00477560` | Read u16; require 3. Call base `Read`; **test AL**, stop if false (`0x00477588`–`0x0047758F`). Otherwise read u32 tile, s32 orientation, three f32 position components, u8 empty; store `empty == 1`; recompute bounds; then test stream `GetError()` (`0x00477591`–`0x004775EE`). | Wrong outer version: consumes 2 bytes and changes no object state. Base rejection: keeps any base changes but makes no landfill-field or bounds changes. Tail error: keeps successful field writes and still assigns empty and recomputes bounds; no rollback. |
| Occupant base `Read`, `0x005E8D60` | Read u16; require 4. Read removal word, highlight, flags, type, four rect bytes in that order. Store status as the **byte** `(highlight << 4) OR 2`. Call exemplar holder `0x005FF420`; **ignore its Boolean**; call stream `GetError()` and return whether it is zero (`0x005E8DD4`–`0x005E8DED`). | Wrong base version: consumes that version only and changes no base state. Later failure retains earlier writes. A holder version rejection with no stream error makes this layer return true. |
| Exemplar holder `Read`, `0x005FF420` | Require u16 2; call plain holder and **test AL** (`0x005FF437`–`0x005FF456`). If false, stop before the parent key. Otherwise read group, type, instance into a key initially all zero, process the key, then return `GetError() == 0` (`0x005FF45C`–`0x005FF501`). | Bad exemplar version: own properties and parent unchanged. Plain-holder rejection: parent unchanged, own properties as left by plain reader. Errors during the key do not bypass key processing: an all-zero partial key clears the parent; a nonzero key still attempts resource lookup, with the existing §10.3 found/missing rules. |
| Plain holder `Read`, `0x005FF280` | Require u16 2; read count, clear own entries, process count entries, return `GetError() == 0` (`0x005FF29B`–`0x005FF2CF`, `0x005FF39E`–`0x005FF3B6`). | Bad plain version: own entries unchanged. After accepted version, clearing is not rolled back on stream error. The parent is not part of this operation. |

The return contract is Boolean **AL** for all four readers. Successful full landfill reads return EAX = 1;
its base-rejected/error exits return EAX = 0; its bad-version exit guarantees AL = 0 only. Primitive read
Booleans are not tested individually by the landfill/base readers. Stream virtual offset `+0x48` is
`GetError`, not the preceding nested reader's result. A version mismatch itself does not set that error.
These facts apply to these concrete reader bodies; foundations §10.3's strict rule still describes the
exemplar holder itself, not every owner that embeds one.

There is **no local resynchronisation** after a rejected holder. If the stream is good, landfill reads
21 bytes immediately after the portion the holder consumed. For LG33's 57-byte input, changing only the
exemplar version at offset 16 to 1 leaves the cursor at 18. Base returns true, then landfill reads offsets
18–38 as its tail: tile = 2, orientation = 0, position words = `(0, 0, 0x00100000)`, empty = false (byte
`0xF1`). With round-to-nearest, bounds become `(-8,0,-8)` … `(8,16,8)`. Return true, cursor 39,
`GetError = 0`; bytes 39–56 remain unread. These state values are static deductions, **not** a newly
scored native state comparison. The class store's entry-size resynchronisation (foundations §3.5) is outside
this reader and does not repair the current object's misparsed state.

Failed primitive reads leave destinations unchanged under the segment-stream contract (foundations §2.2),
but some destinations here are reused/uninitialised stack storage, rather than object fields. In particular,
failed version reads do not supply a defined version; a failed plain count read does not supply a bounded
count; a failed highlight/empty read does not mean “keep the old highlight/empty”. The base still derives
status from its byte temporary, and landfill still derives empty from its byte temporary if it reached the
tail. Do not invent deterministic values or a safe count loop for those cases. A sticky stream error makes
base return false **if it returns**, preventing the landfill tail; an error first occurring in the tail is
tested only after its assignments and bounds update.

Literal proposed test vectors, their exact call sequences, byte offsets, defined state, and remaining
unknowns are in [landfill-occupant-read-vectors.md](landfill-occupant-read-vectors.md). The recorded native
LG33 observation is true / 39 of 57 / error 0 versus the existing Rust false. Outer/base bad versions and
final-byte truncation rejected on both sides. Own-output reload 4/4 and scalar/record comparison 8/9 do not
establish propagation of nested rejection. Keep that observed difference; this clarification is not an
implementation change or a new native acceptance result.

**Tiles across a save and load** *[compat]* (LG8). The original **saves every tile occupant** and **does not
rebuild tiles on load**:

- The occupant manager writes one record per tile, with its tile ID, its orientation (including the random 0, 2, 4
  or 6 of an interior tile), its position, and the exemplar key `(0x07BDDF1C, 0x6534284A, tile)` as the holder's
  parent. On load the occupant manager restores each tile from its record. The developer's PostCityInit (§3.2)
  runs zone `Init`, `UpdateBudgetRequirements` and `UpdateCapacityValues` only. There is no `PopulateZone`, no
  `UpdateSurroundingTiles` and no `rand()` draw.
- A tile is next redrawn only when the active state of a heap next to it changes (§4.8).

Checked against the local saves. The three saved landfills (Konradshohe, Fulham, Big City Tutorial) cover 142 zone
cells in all. Every cell's saved tile, or the absence of one, is exactly what the §4.8 rule table gives for the
saved heaps' `active` flags. The 19 interior tiles have orientations 0 (4), 2 (7), 4 (4) and 6 (5). Every saved tile
has flags `0x05` and `isEmpty = 0`.

**FreeCity4's current choice against this.** FreeCity4 does not save tiles. It rebuilds them at PostCityInit, with
interior orientation 0 and no draw.

- *Loading an original save*: the rebuilt tile IDs match, because the saved tiles always agree with the saved
  heaps. Each interior's orientation becomes 0 instead of its saved value, which is a visible difference only. The
  draw count matches the original (none). The tiles can differ in one case: after `AddRegionToZone` moved heaps
  without moving their tiles (§4.4.4 step 4), the original keeps the stale tiles until the heaps next change.
- *A FreeCity4 save loaded by the original game*: the save has no tile records, so the original finds no tile
  occupants. Until each heap next changes state, two things differ:
  - every zone cell costs `kDefaultLandfillTileCost` (5) instead of 10–40 (§4.9.1);
  - any terraforming (reasons 1–3) over a cell **dezones** it, because §4.5.2 step 1 sees no tile.

  This is a simulation difference, not only a visual one.

*[choice]* Recommended:
- Save real tile occupants, created as in §4.8.1.
- On loading an original save, take each tile's ID and orientation from its saved record, not from a rebuild.
- If the rebuild is kept, it is acceptable for FreeCity4's own saves only, and the difference above should be
  documented.

**A FreeCity4 save without tiles** (LG23). FreeCity4 saves made while tiles were kept only inside FreeCity4 have
no tile records. The original has no behaviour for this case, because it never rebuilds tiles on load: the
original game loading such a save shows no tiles until each heap next changes (above). What FreeCity4 does with
its own old saves is therefore a choice. The bracket rebuilds, at PostCityInit, only the cells that need a tile
and have none. With that rule:

- A cell **needs a tile and has none** when the §4.8 rule table gives a tile other than `0x02F10000` for the saved
  heaps, and occupant manager slot 33 (§4.8.6) finds no landfill occupant (type `0x6A11BD2B`) at the cell. Cells
  whose rule gives the empty tile are left alone, as the original's load leaves them.
- **On an original save it rebuilds nothing** while every saved tile agrees with its heaps. That held for all 142
  zone cells of the three local landfills. Each saved tile then keeps its saved ID and orientation, and no draw
  is made, exactly as in the original.
- **One case differs.** After `AddRegionToZone` moved heaps without moving their tiles (§4.4.4 step 4), a cell can
  need a tile and have none. The original leaves it without a tile until its heaps next change. The rebuild
  creates the tile at load. A cell that holds a stale tile with the wrong ID is kept as it is in both.
- A rebuilt interior tile (mask 15) has no original value to match. Orientation 0, or one draw from FreeCity4's
  own generator (§4.8), are both acceptable.

Example: an old FreeCity4 save holds one zone of 3×3 cells (2×2 heaps, all four active) and no tile occupants.
Its nine cells, local x outer and z inner, have masks 4, 12, 8, 6, 15, 9, 2, 3, 1 (§4.8). None is 0, so all
nine need a tile, have none, and are rebuilt. An original save of the same zone already holds those nine tiles, so
nothing is rebuilt.

*[choice]* The bracket is acceptable.

## 7. Interface slots

### 7.1 `cISC4LandfillDeveloper` (vtable `0x00AB157C`, 13 slots)

| Slot | Off | Address | Signature (as the game expects) | Pop | Return | Behaviour | SDK |
|---|---|---|---|---|---|---|---|
| 0 | +0x00 | `0x006B8B20` | `bool QueryInterface(uint32 iid, void** out)` | 8 | AL | §1 | ok |
| 1 | +0x04 | `0x004F7BA0` | `uint32 AddRef()` | 0 | EAX | | ok |
| 2 | +0x08 | `0x006B9D20` | `uint32 Release()` | 0 | EAX | §1 | ok |
| 3 | +0x0C | `0x006B9D70` | `bool Init()` | 0 | AL | §1.1 | ok |
| 4 | +0x10 | `0x006B94B0` | `bool Shutdown()` | 0 | AL | §1.2 | ok |
| 5 | +0x14 | `0x006B8D50` | `SC4Percentage* GetEfficiency(SC4Percentage* result)` | 4 | EAX = `result` | §4.9.2 | SDK `SC4Percentage GetEfficiency()`: matches only under MSVC's rule that member functions return class types through a hidden pointer (see §7.4) |
| 6 | +0x18 | `0x006B9840` | `uint32 GetSimulatorType()` | 0 | EAX | `0x2A398ED6` | ok |
| 7 | +0x1C | `0x006B9850` | `int64 GetMaintenanceCosts()` | 0 | EDX:EAX | §4.9.2 | ok |
| 8 | +0x20 | `0x006B8BA0` | `int64 GetCurrentBudget()` | 0 | EDX:EAX (tail jump into the line item) | §4.9.2 | ok |
| 9 | +0x24 | `0x006B9030` | `int64 GetZoningCost(SC4CellRegion<long> const*)` | 4 | **EDX:EAX** (EDX = 0) | §4.3 | **SDK ERROR**: SDK `int32`; the zone manager consumes EDX:EAX |
| 10 | +0x28 | `0x006B9820` | `void GetLandfillUsage(uint32& usage, uint32& capacity)` | 8 | none (EAX = capacity) | §4.6.5 | ok |
| 11 | +0x2C | `0x006B8F10` | `int32 GetLandfillTileCount()` | 0 | EAX (tail result) | §4.6.5 | ok |
| 12 | +0x30 | `0x006B9400` | `cSC4LandfillZone* GetLandfillZoneFromCell(int x, int z)` | 8 | EAX | first zone (list order) with `x0 ≤ x < x1` and `z0 ≤ z < z1` (rect only, mask ignored), else null. *[quirk]* half-open: the last row and column are never found. No `AddRef`. | SDK `intptr_t`: callers treat the result as a refcounted zone object (§9) |

### 7.2 `cIGZMessageTarget2` (vtable `0x00AB156C`) and `cIGZSerializable` (vtable `0x00AB1554`)

| Interface | Slot | Address | Member | Pop | Return |
|---|---|---|---|---|---|
| MessageTarget2 | 0–2 | `0x006B9860`, `0x0068D230`, `0x006BA100` | QI / AddRef / Release thunks to the primary | 8/0/0 | AL/EAX/EAX |
| MessageTarget2 | 3 | `0x006B9EA0` | `bool DoMessage(cIGZMessage2*)` (§3.2) | 4 | AL = 1 |
| Serializable | 0–2 | `0x006B9870`, `0x0068D250`, `0x006BA110` | thunks | | |
| Serializable | 3 | `0x006B9880` | `bool Write(cIGZOStream&)` (§6.1) | 4 | AL |
| Serializable | 4 | `0x006B9900` | `bool Read(cIGZIStream&)` (§6.1) | 4 | AL |
| Serializable | 5 | `0x006B8B10` | `uint32 GetGZCLSID()` → `0xEA09D4AE` | 0 | EAX |

### 7.3 `cISC4LandfillOccupant` (IID `0x2A15F262`, vtable `0x00A8CE28`, 10 slots)

The object also implements `cISC4Occupant` (at its second vtable; QI `0x26BB767E` and `1`) and
`cIGZSerializable` (vtable `0x00A8CD84`: Write `0x00477350`, Read `0x00477560`, `GetGZCLSID` `0x00477100` →
`0x8A22BEBE`). QI answers `0x2A15F262` with this interface.

| Slot | Off | Address | Signature | Pop | Return | Behaviour | SDK |
|---|---|---|---|---|---|---|---|
| 0 | +0x00 | `0x00477130` | QueryInterface | 8 | AL | | ok |
| 1 | +0x04 | `0x005E28F0` | AddRef | 0 | EAX | | ok |
| 2 | +0x08 | `0x005E2900` | Release | 0 | EAX | | ok |
| 3 | +0x0C | `0x004771B0` | `bool Init()` | 0 | AL | `cSC4Occupant::Init` | ok |
| 4 | +0x10 | `0x00619100` | `bool Shutdown()` | 0 | AL | `cSC4Occupant::Shutdown` | ok |
| 5 | +0x14 | `0x005FD7E0` | `cISC4Occupant* AsOccupant()` | 0 | EAX | the occupant interface or null | ok |
| 6 | +0x18 | `0x00477450` | `uint32 SetTileID(uint32 id)` | 4 | **EAX = previous tile ID** | if `id ≠ tileID`: store it and change the **attached exemplar's** key instance to `id` (exemplar `GetKey`/`SetKey`) *[quirk: mutates a shared exemplar]* | **SDK ERROR**: SDK `bool`. A replacement returns the previous ID (non-zero, so a bool-reading caller sees true). No caller found in the shipped code. |
| 7 | +0x1C | `0x00477110` | `int32 GetOrientation()` | 0 | EAX | (read by the model maker) | ok |
| 8 | +0x20 | `0x004771C0` | `bool SetOrientation(int32)` | 4 | AL = 1 | stores it | ok |
| 9 | +0x24 | `0x00477120` | `bool IsEmptyLandfillTile()` | 0 | AL | `isEmpty` | ok |

**Inherited position interface, concrete landfill body** *[compat]* (LG32). On the occupant view
(`cISC4Occupant`, object +4, vtable `0x00A8CDA0`), slot 8 / `+0x20` is GetPosition
`0x005B2B00`, slot 9 / `+0x24` is SetPosition `0x00477600`, and slot 10 / `+0x28` is
GetBoundingBox `0x00477420`. All use `__thiscall`; the position methods pop 4 bytes (one vector pointer),
GetBoundingBox pops 8 (two output pointers). The table's ported name `cSC4PowerLineOccupant::vftable_1`
does not change the landfill constructor's actual installation of that table (`0x0047728A`).

SetPosition copies the three input **32-bit float words** to position, then computes bounds as in §2.3.
Each bound arithmetic operation loads f32, adds/subtracts the f32 constant 8 or 16, and stores f32
(`0x00477615`–`0x00477654`; constants `0x00A8CCFC` = `0x41000000`, `0x00A8CD04` = `0x41800000`).
There is no registration/Init check, manager notification, or success/failure branch in this body.

Treat the operation as **void**: callers must not predicate success on the SDK's `bool`. Instructions
`0x0047760F`–`0x00477612` leave the z input's raw word in EAX, and no subsequent instruction changes EAX
before `ret 4` at `0x00477657`. This is a residual register value, not a Boolean acceptance channel.
The shipped creation caller invokes slot 9 at `0x006BC8F4` and proceeds to Init at `0x006BC8FB` without
testing a return. This establishes the concrete landfill contract, not a return rule for every occupant class.

For `(88.5,-4,120.25)`, position words are `(0x42B10000,0xC0800000,0x42F08000)` and bounds are
`(80.5,-4,112.25)` … `(96.5,12,128.25)`. EAX's residual word is `0x42F08000`, hence AL = 0.
The SDK Boolean false measured in both `vlfr4-r2-on-reload-2032-20261022a` and
`vlfr4-r2-normal-diagnostic-20261022b` is consistent with that instruction trace; it is not a rejected move.
Their exact position/bounds and all 57 record bytes remain the recorded effect agreement. SetOrientation(-123)
returning true and same-ID SetTileID returning its previous ID remain separate contracts. Score position
effects through the typed outputs; retain the historical SDK return observation as unscored.

### 7.4 Notes on the SDK comparison

- `GetEfficiency` (checked at `0x006B8D50`: `ret 4`, writes the float through the first stack argument and
  returns it in EAX). The SDK declares `SC4Percentage GetEfficiency()` with `SC4Percentage` a POD holding one
  float. MSVC returns class types from member functions through a hidden pointer, which matches the game; a
  compiler that returns a 4-byte POD in EAX would not. Not counted as an SDK error.
- `GetLandfillUsage`: the SDK parameter order `(usage, capacity)` is correct (checked at `0x006B9820`).
- The zone vtable `0x00AB1610` is not a public interface; callers rely on its first three slots (§9).

**Independent native zone verification boundary (LG31).** The published contracts provide **no supported
public-interface route** to invoke a native zone's queries, mutations, or record `Read`/`Write` on controlled
inputs. Developer slot 12 (`0x006B9400`, §7.1) supplies only a borrowed zone pointer: it adds no reference.
The known zone prefix (`0x00AB1610`, §1) permits QI for unknown/message-target identity, AddRef/Release and
the no-op message operation; it publishes neither zone methods nor a zone serializable interface. There is
no zone GZCOM CLSID for independent creation. A caller retaining a non-null result must acquire its own
reference and balance it with Release (the shipped auto-ref holder does this, §9); this protects lifetime,
but does not establish an independent controlled zone instance or authorize private method calls. The
zone lifecycle in §4.4.9, inline format in §6.2 and replacement layout requirement in §9 describe behavior
and compatibility, not additional public calling contracts. Therefore park independent native zone
verification at this boundary: do not call code addresses, inspect private fields, substitute developer
serialization as a zone-method oracle, or credit developer/occupant checks as zone verification. The zone
rules remain specified and can have implementation conformance tests; their independent native comparison
remains unsupported through the published public interfaces, with no native comparisons or coverage credit
claimed. This closes LG31's contract question without changing that verification limitation.

## 8. Dependencies

| Interface | Slot / off | Member | Pop | Return | Used for |
|---|---|---|---|---|---|
| `cISC4City` (`0x00A87620`) | 34 / +0x88 | GetOccupantManager | 0 | EAX | services |
| | 35 / +0x8C | GetPropManager | 0 | EAX | zot removal |
| | 36 / +0x90 | GetZoneManager | 0 | EAX | zone types, PlaceZone |
| | 40 / +0xA0 | GetTrafficNetwork → `cISC4TrafficNetworkMap*`, **borrowed** (`0x004596F0`: returns city `+0x1E8`, no `AddRef`; LG17) | 0 | EAX | road test in `PopulateZone` |
| | 47 / +0xBC | GetAutomataControllerManager | 0 | EAX | effects |
| | 50 / +0xC8 | GetSimulator | 0 | EAX | agents |
| | 93, 94 / +0x174, +0x178 | cell counts X, Z | 0 | EAX | §3.2, §4.5.3 |
| | 97 / +0x184 | CellCenterToPosition(x, z, &px, &pz) | 16 | **none** (x87 stack empty; §4.2) | everywhere |
| `cISC4Simulator` | 19 / +0x4C, 20 / +0x50 | AddAgent / RemoveAgent | 16 / 4 | AL | §1.1–1.2 |
| `cISTETerrain` (`0x00AB3D18`) | 13 / +0x34 | GetAltitude(x, z) | 8 | ST0 | altitudes |
| | 19 / +0x4C | GetAltitudeAtNearestGrid(x, z) | 8 | ST0 | §4.5.2, zot |
| | 29 / +0x74 | GetSeaLevel() | 0 | ST0 | §4.3, §4.5.2 |
| | 49 / +0xC4 | SetAltitude(rect*, alt, 1, 5) | 16 | — | unused path of §4.5.3 |
| | 68 / +0x110 | RedisplayTerrain(1, 1, rect*, 5) | 16 | — | §4.4.2 |
| `cISC4PollutionSimulator` | 20 / +0x50 | GetGarbageSupply | 0 | EAX | §4.7.1 |
| | 21 / +0x54 | GetLandfillTileCapacity | 0 | EAX | §4.6.1 |
| | 24 / +0x60 | GetTotalGarbageUncollected | 0 | EAX | history |
| | 29 / +0x74 | AddToGarbageSupply(int32) | 4 | — | intake, spill |
| | 30 / +0x78 | SetGarbageCapacity(int32) | 4 | — | §4.7.1 |
| | 50 / +0xC8 | SetIsActiveLandfill(x, z, bool) | 12 | — | §4.8 |
| | 51 / +0xCC | GarbagePutInLandfill(int32) | 4 | (ignored) | §4.7.2 |
| `cISC4BudgetSimulator` (`0x00AB0908`) | 28 / +0x70 | GetDepartmentBudget(id) | 4 | EAX | PostCityInit |
| `cISC4DepartmentBudget` (`0x00AB0EB0`) | 10 / +0x28 | GetFundingPercentage → `SC4Percentage*` | 0 | EAX | FundsChanged |
| | 20 / +0x50 | GetLineItem(id) | 4 | EAX | line 30 |
| `cISC4LineItem` (`0x00AB0F58`) | 11 / +0x2C | GetCurrentExpenses | 0 | EDX:EAX | §4.9.2 |
| | 12 / +0x30 | SetFullExpenses(int64) | 8 | — | §4.9.3 |
| `cISC4ZoneManager` (`0x00A92E00`) | 6 / +0x18 | GetZoneCount(type) | 4 | EAX | slot 11 |
| | 11 / +0x2C | PlaceZone(10 args) | 40 | AL | §4.4.8 |
| | 12 / +0x30 | GetZoneType(x, z) | 8 | EAX | §4.4.5 |
| `cISC4OccupantManager` (`0x00A8F7F0`) | 20 / +0x50 | insert(occupant, 0) | 8 | AL | §4.8.1 |
| | 21 / +0x54 | remove(occupant, 1, 3) | 12 | AL | §4.8.3 |
| | 24 / +0x60 | (occupant, 1) after a move | 8 | — | §4.5.2 |
| | 33 / +0x84 | GetOccupantAtCell(&occ, x, z, type): first hit, `AddRef`ed; `occ` untouched on a miss | 16 | AL | §4.8.6 |
| | 41 / +0xA4 | occupants at a cell by type | 20 | — | flora / texture removal |
| | 48 / +0xC0 | release each entry of a slot-41 list and empty it | 4 | — | §4.8.4 |
| `cISC4PropManager` (`0x00A8F928`) | 26 / +0x68 | AddLotProp(propID, pos, orientation) | 12 | EAX | zot |
| | 29 / +0x74 | RemovePropA(occupant) | 4 | AL | zot |
| `cISC4TrafficSimulator` (`0x00AB33D0`) | 41 / +0xA4 | GetSubnetworksInRegion(region, &ids) | 8 | — | §4.7.2 |
| | 42 / +0xA8 | GetOccupantCountForAllSubnetworks(type, &counts) | 8 | — | §4.7.2 |
| `cISC4TrafficNetworkMap` (`cSC4TrafficNetworkMap`, IID `0xCA0F81B9`, vtable **`0x00A8FED8`**, 52 slots, not in docs/abi) | 8 / +0x20 | FindNetworkTile(x, z, flagMask, alsoMultiCell) → borrowed tile or null (`0x004B0B20`) | 16 | EAX | §4.4.5 |
| `cISC4BuildingDevelopmentSimulator` (`0x00AB0AF8`) | 36 / +0x90 | GetLotDeveloper | 0 | EAX | §4.5.3 |
| `cISC4LotDeveloper` (`0x00AB1B98`) | 21 / +0x54 | AttemptAreaLevelling(rect, float, bool, bool, float, lot*) | 24 | AL (ignored) | §4.5.3 |
| `cISC4View3DWin` (`0x00ABCE40`) | 22 / +0x58 | GetCurrentViewLevel | 0 | EAX | §3.2 |
| | 4 / +0x10 | GetRenderer → borrowed `cISC43DRender*` (`0x0043FD00`) | 0 | EAX | §4.7.5 (glue route only; the original reads the global) |
| `cISC43DRender` (`cSC43DRender`, vtable `0x00ABAB60`) | 8 / +0x20 | GetViewUtilities → borrowed, not a GZCOM object (`0x007C9FB0`) | 0 | EAX | §4.7.5 (rotation at `+0x10`) |
| resource manager (`0x00B43CA8`) | 3 / +0x0C | GetResource(key, `0xA52160F5`, &out, 0, 0) | 20 | AL | §4.8.1 |
| `cISCExemplarPropertyHolder` (IID `0x0AC2B5F7`, 5 slots, §4.8.1) | 3 / +0x0C | SetDefaultExemplar(ex) | 4 | none (`0x00458E60` sets no result) | §4.8.1 |
| `cIGZCOM` | 3 / +0x0C | CreateInstance(clsid, iid, void**) | 12 | AL | §4.8.5 |
| `cISC4LotBaseTextureOccupant` (IID `0x097F6D98`) | 3, 4, 5, 8 | AsOccupant, GetUsage, SetUsage(1), SetTextureSpecification(rec*, 1, null) | | | §4.8.4, §4.8.5 |
| `cISC4ZoneManager` | 16 / +0x40 | GetZoneDisplayAlpha | 0 | AL | §4.8.5 |
| `cISC4Occupant` | 8, 9, 12, 15, 29 | GetPosition, SetPosition, SetRemovalFlags, (0x99EF1145, 0), SetFlag | | | tiles, zot |
| `cIGZMessageServer2` | +0x0C, +0x14, +0x18 | MessageSend, subscribe, unsubscribe | | | |

## 9. Replaceability

**Who reaches in.**

| Caller | Access | Requirement for a replacement |
|---|---|---|
| `SL::OnCityInit` (`0x00601C20`) | caches `GetLandfillDeveloper()` in a global; the cached pointer is used by the advisor variables, the query tool (DescribeCell, performance meter, debug info), the neighbours simulator (`simulate_deals`, `UpdateCommodityStats`), the automata behaviour template, the city situation manager and the landfill gas recovery ordinance (holds a reference) | the primary interface pointer must stay valid for the city's life |
| pollution simulator `Init` | stores the pointer **without** `AddRef` (pollution §4.1) | same |
| slot 8 `GetCurrentBudget` | query tool (`0x004C830A`) | int64 |
| slot 9 `GetZoningCost` | zone manager `PlaceZone` (`0x004DDCE2`, adds EDX:EAX to the int64 cost for zone type 14) | must return int64 |
| slot 10 `GetLandfillUsage` | advisors, query tool, neighbours, pollution, ordinance | as §4.6.5 |
| slot 11 `GetLandfillTileCount` | pollution, automata behaviour template | |
| slot 12 `GetLandfillZoneFromCell` | `cSC4CitySituationManager::SetTarget` (`0x00528B26`), and `GetDistanceToTarget` (`0x00526A20`) through the stored result | **blocker** if non-null. The result goes into an auto-ref holder, which calls `AddRef`/`Release` through slots 1 and 2 of the returned object. The dword at **+0xC** is then followed as a pointer to four **integers** `x0, z0, x1, z1`, which are read correctly. Null is handled (LG9, below). A replacement must return null or an object laid out this way. |
| landfill occupant type `0x6A11BD2B` | flora blocking filter (flora §6.4.1), hover query filter, query tool, zone manager `PlaceZone` (calls the occupant's slot 14 with 2), model maker (`GetOrientation`) | the occupant class and its type must stay |

**The city situation manager and a null zone** *[compat]* (LG9). `SetTarget` asks only when the new target occupant
is in occupant group `0x1406` and the landfill developer global exists (`0x00528AB9`–`0x00528ACD`). Group `0x1406` is
the landfill tiles' group: every tile exemplar's `0xAA1DD396` is `[0x1406]`. `SetTarget` then converts the target's
position to a cell (city slot 95) and calls slot 12 with that cell (`0x00528B26`). After that:

- **Non-null zone.** It copies `x0, z0, x1, z1` from `*(zone + 0xC)` and uses `(x0, z0, x1 + 1, z1 + 1)`
  (`0x00528BD8`–`0x00528C0A`, integer `inc`). It hands that to a marker object, which converts it with `FILD`
  (`0x0043FB60`). `GetDistanceToTarget` uses the same rect in metres, ×16 (integer `shl 4`, then `FILD`, from
  `0x00526B3A`). It returns 0 inside the rect and the distance to its edge outside.
- **Null zone**, or a null `+0xC` pointer. Both functions fall back to the target's lot (lot manager slot 14, then
  lot slot 14 `GetBoundingRect`). A tile has no lot, so no zone marker is made, and `GetDistanceToTarget` returns the
  plain 3D distance to the target occupant.

So null is safe. It is also what the original itself returns for a tile in the zone's last row or column (§7.1).
The only effect is on the situation's target area, a UI and mission feature, never on the simulation
*[inferred: the U-Drive-It mission target]*.

No other code reads the developer's or a zone's memory. The developer owns **no grid** (simgrid §6.4 does not
apply); it writes pollution's garbage grid only through `SetIsActiveLandfill` and `AddToGarbageSupply`.

**Replacement.** The developer is a city component created through GZCOM (`0xEA09D4AE`), so a registered
replacement class is used. It must: answer `0x8A09D502` with the 13-slot vtable of §7.1 (with the int64
`GetZoningCost` and hidden-pointer `GetEfficiency`), be serializable under the same CLSID with the §6 record if
saves are to stay compatible, keep `cSC4LandfillOccupant` (`0x8A22BEBE`) as the tile class (other systems test
its type), and honour the GetLandfillZoneFromCell layout above or return null. The zone class itself is private
and need not be reproduced beyond that.

## 10. Quirks and open questions

### 10.1 Quirks (reproduced; summary)

1. `GetLandfillZoneFromCell` misses the last row and column of a zone (§7.1).
2. The zone record omits the last row and column of the mask; they load as zoned (§6.2).
3. `levelAltitude` is not saved; loaded zones use the centre altitude (§4.4.9).
4. `AddRegionToZone` copies old heaps by their old local coordinates (§4.4.4).
5. Cells dropped by `PopulateZone` keep their heaps valid (§4.4.5).
6. `completelyDezoned` is sticky; a zone with no valid heap after a partial dezone is removed whole (§4.4.7).
7. Terrain dezoning skips `RecalculateCapacity`; `RedistributeGarbage` then duplicates garbage (§4.5.1, §4.7.4).
8. The per-zone decay cap applies per heap (§4.7.3).
9. A new zone does not update the budget until the next month (§4.4.1 step 3).
10. `LevelRect(…, false)` does not pass the target altitude (§4.5.3).
11. Any terraforming (reasons 1–3) over an empty landfill cell dezones it (§4.5.2).
12. PostCityInit leaks a previous department reference; `GetCurrentBudget` crashes before PostCityInit (§3.2,
    §4.9.2).
13. `SetTileID` mutates the shared exemplar's key (§7.3).
14. The magic round is wrong above 2^22 (§4).
15. *Withdrawn* (2026-10-14, LG9): the city situation manager reads the zone's cell coordinates as integers and
    converts them correctly (§9). There is no quirk.

### 10.2 Open questions

1. **Month order**: *Decided* — `foundations.md` §13.4 (2026-10-13). The landfill developer's month agent gets a
   fixed rank in the stable tail (landfill developer, port developers, neighbours, ordinance), which runs after the
   first 18 registration-order agents; that satisfies "after pollution and budget, before neighbours and ordinance"
   without depending on when the city calls `Init`. **The rank mechanism is deferred until the landfill glue
   actually lands** (§13.4 "Sequencing"): the merged simulator core uses plain registration order and is verified
   in-game, and no tail agent is wired up yet. So implement the landfill core against the ordering requirement in
   §3.3 as a *contract to be satisfied later*, and pin it with a conformance vector that asserts the relative order
   rather than assuming the simulator already ranks agents.
2. *Answered* (2026-10-13, verified from the assembly of `0x006C44F0`): `AttemptAreaLevelling` levels the rect to
   the **vertex average** altitude and can refuse — see §4.5.3. Two corrections to what this spec asserted: the
   float argument is **never read** (so passing `0.0` is inert, not a lost request), and the levelling **can be
   skipped entirely** when `GetAreaSlope(rect) > 40`, which the old text did not allow for.
3. *Partly answered* (2026-10-13): `0x278E6003` has a single send site, `cSTETerrain::RedisplayTerrain`, and
   `data4` is that function's fourth argument — verified (§3.2). The lot developer's levelling does use reason
   **5**, verified: `AttemptAreaLevelling` → `cSC4Demolition::ModifyTerrainHeight(…, 5, …)` →
   `RedisplayTerrain(1, 1, rect, 5)`. *Answered* (re-specs14a, 2026-10-15): 1 = god-mode terraforming, the edge
   reconciliation and `ChangeTerrainLevel`; 2 = mayor-mode terraforming and the `ApplyTerrainBrush` command; 3 =
   disasters (earthquake, sinkhole, volcano) and water. Every sender is listed in terrain.md §8.2.
4. *Answered* (2026-10-13, verified): **no**. `PlaceZone` `MessagePost`s `0x07699455`, so §4.4.8 does not
   re-enter `DoZoneChange`. See §4.4.8.
5. The meaning of occupant manager slot 24 and occupant slot 15 with `0x99EF1145` after a tile move (§4.5.2).
6. *Answered* (2026-10-14, LG5): the zot orientation is the current view rotation, `SC4ViewUtilities` (`0x00B43DD8`)
   `+0x10` (§4.7.5).
7. `isEmpty` is never set true by the shipped code as far as found; the `IsEmptyLandfillTile` branches are dead
   *[inferred]*. The only writer of `true` found is a dead branch of `CreateLandfillOccupant` (§4.8.1); a loaded
   record can still carry 1 (§6.4).
8. History IDs `0x2A8C4696` / `0x2A8C4697` have no names in the symbols; the advisor/graph users were not traced.

## 11. Conformance tests

Unless stated: shipped tuning (§4.1), `tileCap` = 30000, sea level 250, flat terrain at 270 m, sanitation
funding 100 %, zone cells all zone type 14, no roads inside.

1. **Heap capacity, 4×4**: one zone of 4×4 cells → 16 cells, `W = H = 3`, 9 valid heaps;
   `c = round(480000 / 9) = 53333`; `totalCapacity = 479997`. A 2×2 zone: 1 heap, capacity 120000. A 1×5 zone:
   `W = 0`, no heaps, capacity 0, `GetLandfillUsage` contributes 0.
2. **Zoning cost, flat**: 4×4 region on flat ground → variance 0 ≤ 5 → `GetZoningCost = 0` (EDX = 0).
3. **Zoning cost, slope**: 2×2 region, cell-centre altitudes 270, 272, 280, 290 → min 270, max 290, variance 20 >
   5, mean 278, `Σ|278 − a| = 28`, cost `round(4 × 28) = 112`.
4. **Zoning cost, extension**: region intersecting a zone with `levelAltitude` 275 over cells 270, 280 → no
   tolerance test, cost `round(4 × 10) = 40`.
5. **Sea level clamp**: 2×2 region, centre cell `(x0, z0)` at 251, `(x0+1, z0)` at 240 (below sea 250),
   `(x0, z0+1)` at 260, `(x0+1, z0+1)` at 262 → the low cell counts as 251 for variance and mean: min 251, max 262,
   variance 11 > 5, mean 256; the cost uses raw altitudes: `Σ|256 − a| = 5 + 16 + 4 + 6 = 31`, cost 124.
6. **First intake**: connected 2×2 zone created this month (maintenance still 0, §4.4.1 step 3, so no funding
   limit), supply 10000 → `t = 10000`; zone budget `min(10000, 4 × 5000) = 10000`; heap: contained 10000,
   `AddToGarbageSupply(−10000)`,
   `GarbagePutInLandfill(10000)`, active → four tiles: cell (0,0) mask 4 → `0x034E0000` or. 1; (1,0) mask 2 →
   or. 2; (0,1) mask 8 → or. 0; (1,1) mask 1 → or. 3; no `rand()` draw; `SetGarbageCapacity(110000)`;
   maintenance becomes 40; history `0x2A8C4696 = 120000`, `0x2A8C4697 = 10000 + uncollected + 0`.
7. **Decay**: heap with 10000 → `round(10000 × 0.07) = 700` → 9300. Heap with 5 → `round(0.35) = 0` → 1 → 4.
   Heap with 40000 → 2800 → capped to 1500 → 38500.
8. **Underfunding**: maintenance 40, current 20, supply 10000 → `t = trunc(0.5 × 10000) = 5000`.
9. **Per-tile cap**: connected 2×2 zone, supply 50000 → zone budget 20000 → contained 20000, supply 30000.
10. **Global cap**: code-default `kMaxGarbageToTakePerMonth` 40000 and supply 100000 → `t = 40000`.
11. **Disconnected**: zone not connected → no intake, decay still applies, a zot `0x107A0000` is placed at the
    first mask cell spiralling from the centre, 3.0 m above the terrain; the zone is excluded from
    `capacityTotal`, `usedTotal` and `avail`. Connecting it removes the zot at the next month.

    The spiral order is §4.7.5's: ring r starts at `(cx + r, cz − r + 1)` and runs +z first. Include both of
    §4.7.5's examples:
    - the 3 × 3 zone with its centre dezoned → `(12, 21)`;
    - the clipped ring 2 → `(14, 22)`.

    A vector built on another ring order (the second half of `lf11_11`) must be regenerated.
12. **Random draws**: a 3×3 zone (2×2 heaps) with heaps (0,0), (1,0), (0,1) active; heap (1,1) becomes active →
    its redraw visits cells (1,1), (2,1), (1,2), (2,2); only the centre cell (1,1) has mask 15 → exactly **one**
    `rand()` call, tile `0x02FA0000`, orientation `2 × (rand() & 3)`. With the CRT seeded to 1 and no earlier
    draws, `rand()` = 41 → orientation 2. The same zone with heap (0,0) becoming active first → no draw.
    (In play, heaps fill in vector order: with `kMaxGarbageToTakePerMonthPerTile` = 5000 a 3×3 zone takes at most
    45000 a month and a heap holds 67500, so only one heap turns active per month at the start.)
13. **Redistribution**: 3 valid heaps of capacity 100 each, contained 100, 50, 0 (total 150) → share 100 →
    100, 50, 0.
14. **Spill on dezone**: dezoning one corner cell of a 3×3 zone with heaps full → the corner heap becomes invalid,
    `RecalculateCapacity` with 3 valid heaps: `c = round(240000 / 3)` = 80000; the invalid heap's garbage and
    any excess return to the supply.
15. **Narrow remainder**: dezoning the middle column of a 3×3 zone leaves two 1×3 strips with no valid heap →
    sticky flag → the remaining 6 cells are rezoned to type 0 and the zone is removed.
16. **Adjacency merge**: zoning a 2×2 landfill diagonally touching an existing 2×2 zone → no new zone; the
    existing zone's rect becomes the union, its mask the 8 cells, and its heaps are rebuilt (§4.4.4).
17. **Save round trip**: §6.3 bytes reproduce the zone exactly, except that `levelAltitude` becomes the centre
    altitude and a dezoned cell in the last row/column comes back as zoned.
18. **Usage clamp**: `usedTotal` 150, `capacityTotal` 100 → `GetLandfillUsage` → (100, 100).
19. **GetLandfillZoneFromCell**: zone rect (10, 20, 12, 22) → (10, 20) and (11, 21) return the zone; (12, 21) and
    (11, 22) return null.
20. **Efficiency**: maintenance 40, current 48 → 1.2; maintenance 0 → 1.0.
21. **Terrain move**: a tile at y = 270 with terrain raised to 271 (reason 1) and `kbMoveWithTerrain` → occupant
    y := 271; raised by 0.2 → unchanged; an empty cell in the rect → dezoned and rezoned to 0.
22. **Levelling target and refusal** (§4.5.3, `direct = false`). Stub the lot developer's three slots so the
    terrain model is not involved: `GetAverageAreaAltitude(rect, true)` → 272.0, `GetAreaSlope(rect, true)` → 3.0,
    and let `ModifyTerrainHeight` record its arguments.
    - With `maxAreaSlope = FLT_MAX`: `ModifyTerrainHeight` is called once with
      `(1, true, x1, z1, x2 + 1, z2 + 1, 272.0f, null, 5, null)` — note the **vertex** average 272.0, not any
      cell-centre value — and the slot returns true.
    - The second argument is inert: repeating with `272.0`, `0.0` and `−1.0` in that position gives identical
      recorded arguments and identical results.
    - With `GetAreaSlope` → 41.0 and `maxAreaSlope = FLT_MAX` (so the limit is the tuning's 40): the slot returns
      **false**, `ModifyTerrainHeight` is **not** called, and the terrain is untouched. The zone is still created
      and populated by §4.4.1 step 3, and `levelAltitude` is still set to the caller's argument.
    - With `maxAreaSlope = 5000.0` (≥ 1000) and `GetAreaSlope` → 9999.0: the slope test is skipped entirely and
      the levelling happens.
    - With `maxAreaSlope` a finite value ≠ `FLT_MAX`, the lot developer's tuning `0xE900EA2A` is **not** used.
23. **Reason 5 is ignored** (§3.2): delivering `0x278E6003` with `data4 = 5` over a zone with tiles changes no
    zone, heap or occupant; with `data4 = 1`, `2` or `3` the same rect dezones the empty cells (§4.5.2). The
    landfill's own levelling must not feed back into `DoTerrainChanged`.

### 11.1 Public zone verification boundary vectors (LG31)

These are verification admission and accounting vectors, not newly measured native zone results.
They supplement the behavior vectors above; in particular, an implementation-only zone round trip does
not establish that a public native zone record operation was exercised.

| Vector | Controlled setup or request | Required outcome |
|---|---|---|
| LG31-B1: acquisition | Developer slot 12 yields null, or yields a non-null borrowed zone pointer. | Null provides no object to test. Non-null supplies identity only under the published prefix; retention requires a balanced AddRef/Release. Neither outcome admits independent zone query/mutation/record tests. No private access or native comparison is attempted. |
| LG31-B2: zone record | Independently authored zone bytes, including the §6.3 example, are available; request a native zone `Read` followed by `Write`. | Stop at the public-interface boundary: no published zone factory or serializable calling contract exists. Keep any implementation-only result separate; report native zone comparison as unsupported, not pass or mismatch. |
| LG31-B3: indirect evidence | A public developer record round trip contains inline zones, or public occupant checks agree. | Credit only the developer or occupant operations actually compared. Independent native zone queries, mutations and records remain unverified; indirect bytes do not become a zone-method oracle. |
| LG31-B4: closure accounting | LG31 is answered by §7.4 and all measured public developer/occupant cases pass. | Close the contract question while retaining partial landfill verification. Claim no new native zone comparisons or zone coverage credit; question closure alone does not change verification status. |
