# Fire and police protection (`cSC4ProtectionSimulator`, `cSC4FireProtectionSimulator`, `cSC4PoliceSimulator`) — behaviour spec

Status: round 1, written from the Windows 1.1.641 executable. The earlier write-up in
`docs/subsystems/grid-sims.md` was the starting point. Every fact below was checked against the Windows code, and
the corrections are listed in §16. The implementer questions PR1–PR15 (`QUESTIONS.md`) were answered from the
exe and folded in (§4.1, §5–§9, §11, §15, §16, tests 1–5, 8, 9, 23–31).

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

Shared material cited and not repeated here: streams, refs and strings (foundations §2), class store (§3),
city component table (§3.6), Init order (§3.7), response curves (§4), messages (§5), random generators (§6),
C `rand` (§12), month order (§13), slot-order rule (§16), framework slot tables (§17); scheduler levels and agent
semantics (simulator spec); grid classes, geometry, grid slots, grid persistence, lifetime, the grid ownership
rule and the map utilities (`docs/specs/simgrid.md` §2–§7); budget items, departments and locally funded objects
(budget spec). How the crime simulator uses the police simulator is in `crime.md` §6.4, §6.6 and §10.

Naming: "the base" is `cSC4ProtectionSimulator`, the shared implementation. "Fire" and "police" are the two
concrete classes. Where both behave the same, the text says "the simulator". A **station** is a building that
the class's `GetStationEffect` accepts (§7.1). A **jail** is a police-only concept (§9).

---

## 1. Role and identity

Each simulator keeps a **coverage grid**: every station adds a linear cone (simgrid §7.1) whose strength depends
on the station's funding, road access and strike state. Vehicles out on a dispatch add a second, moving cone.
The grid is read by fires and riots, crime, the advisors, the map view and the query tools. Each simulator also:

- is a **dispatch source** for the dispatch manager (fire engines: dispatch type 1; police cars: type 0), handing
  out numbered units from each station (§8);
- runs **strikes**: under-funded stations can go on strike each month, which cuts their effect (§6.5);
- owns a budget department and books every station as a locally funded object (§6.6);
- owns an **air dispatch source** object (fire planes or police helicopters, §8.4).

The police simulator also runs **jails**: capacity, jailed criminals, arrests, rehabilitation, overflow
releases and jailbreaks (§9). Crime events from the crime simulator become arrests or unarrested crimes there.

| Item | Fire | Police |
|---|---|---|
| Class ID (`GetGZCLSID`) | `0x2990C05A` | `0x6990C04F` |
| Primary interface | `cISC4FireProtectionSimulator`, IID `0x88F54C7B` | `cISC4PoliceSimulator`, IID `0x67659895` |
| `cIGZUnknown` `0x00000001` | the primary interface | the primary interface |
| `cIGZMessageTarget2` `0x452294AA` | the base sub-object (§15.3) | same |
| Dispatch source `0xC9C00435` (no SDK header) | dispatch-source sub-object (§15.4) | same |
| `cIGZSerializable` `0xE4FDA3D4` | serializable sub-object (§15.5) | same |
| Not answered | everything else → AL = 0 | same |
| City component | slot **#25** (foundations §3.6) | slot **#33** |
| City getter | `cISC4City` slot 57 `+0xE4` (`GetFireProtectionSimulator`). Its body `0x006DC690`, a folded one-line getter, returns the city's `+0x174`. The new-city path creates that member as class `0x2990C05A` / IID `0x88F54C7B` (`0x0045ADF7`), and the city `Read` fills it from component ref #25 (`0x0045CEDB`). The game caches it in global `0x00B43D78` (`0x00601E8A`) | `cISC4City` slot 65 `+0x104` `GetPoliceSimulator`; cached in global `0x00B43D98` |
| Init order (foundations §3.7) | after crime, before flammability | after land value, before traffic |
| Object size | 0x108 bytes (factory `0x006ADB80`) | 0x168 bytes (factory `0x006E2720`) |

All four interfaces are one object with one reference count. The count starts at 0 (the GZCOM QI adds the
first reference). `AddRef` increments. `Release` with a count ≤ 1 calls `Shutdown()`, destroys the object and
returns 0; otherwise it decrements and returns the new count. `QueryInterface` with a null `this` sets the out
pointer to null and returns **true** *[quirk]*; otherwise it `AddRef`s on success. *[compat]*

The class-ID table also names `0x2990C05A` "cSC4ProtectionSimulator" (foundations §3.6); the class registered
under it is the fire simulator. There is no registered class for the base alone.

---

## 2. Grids

| Grid | Type | CLSID | Instance ID: fire / police | Tract size | Persisted |
|---|---|---|---|---|---|
| **adjusted** (the coverage grid every consumer reads) | `uint8` | `0x49B9E602` | `0x49D5B8F9` / `0x49D5BBB9` | 2 | yes, by `ref` |
| **unadjusted** (sum of all cones) | `int16` | `0x49B9E605` | `0x89D5B91A` / `0x49D5BBCE` | 2 | yes, by `ref` |

- Both grids are created at `Init` only when absent (a loaded city brings them back through the refs, with their
  saved geometry), with the class IDs above (how: below), followed by `SetTractSize(2)`. The class `Init` then
  sets the instance IDs above on both grids, also after a load. *[compat]*
- Geometry: simgrid §3 with S = 2 (a 256-cell city has 128 × 128 tracts, tract width 32 m).
- The **unadjusted** grid is rebuilt from scratch (§5.1) whenever a station, its funding, a strike or a dispatch
  changes. The **adjusted** grid follows it: `adjusted := clamp(roundHalfEven(unadjusted × factor), 0, 255)`,
  where `factor` is always 1.0 (§16 Q2). This copy is made in two ways: for the box around a station that was
  added, removed or changed (§5.2), and tract by tract by the daily iterator (§5.3). *[compat]*
- *(Corrected: not through GZCOM.)* For a new city, `ProtectionInit` creates both grids directly: 0x44 bytes from
  the global heap, the grid class's constructor inlined (vtables `0x00A878C0` `uint8`, `0x00A879E0` `int16`; the
  class ID written at `+0x10`), then its `Init`, at `0x006F6C18`–`0x006F6CDE`. A loaded city gets them from the
  class store, which does go through the class registry.
- The primary interface's slot 5 returns the **adjusted** grid, without `AddRef`.
- **Where the grids live** *[compat]* (PR5). Both classes keep the adjusted grid pointer at primary-interface
  offset `+0x80`, which slot 5 returns (`0x006EFA50`), and the **unadjusted** `int16` grid at **`+0x84`** (fire
  `Init` `0x006AE304`, police `Init` `0x006E28A0`; `ProtectionInit` runs on the sub-object at `+0x08`, so its
  `+0x78`/`+0x7C` are the same members). No interface slot returns the unadjusted grid. It can be read in three
  ways:
  1. the pointer at `+0x84` of the original object (borrowed, no AddRef), with the grid's own read-only slots
     (`GetCellValue`, `GetGridData`);
  2. its own record in a save (saved by ref; instance `0x89D5B91A` fire, `0x49D5BBCE` police);
  3. inferred from the adjusted grid, which equals `clamp(unadjusted, 0, 255)` only in tracts the daily iterator
     or a box update has copied since the last change.
- **Recognising the original class before reading `+0x84`** *[compat]* (PR16). Only on the identified executable
  (the build these addresses come from), and only for the pointer the city returns (`GetFireProtectionSimulator`, slot 57, and
  `GetPoliceSimulator`, slot 65; the city keeps them at `+0x174` / `+0x194` and the game caches them in
  `0x00B43D78` / `0x00B43D98`). That pointer is the object's start, the primary interface.
  - **Test**: its first dword equals the primary vtable, `0x00AB1160` (fire) or `0x00AB2490` (police) (§15.1,
    §15.2). Only the class's constructor and destructor write those values (fire `0x006ADA67` / `0x006ADB27`,
    police `0x006E1F9D` / `0x006E16ED`); during construction the base's vtable is there instead. A
    replacement registered under the same class ID, or a wrapper around the original, has its own vtable and
    fails the test, so its `+0x84` is never read.
  - **Lifetime**: the base constructor clears the member (`0x006F6986`); `ProtectionInit` or the load fills it;
    `ProtectionShutdown` releases the grid and clears the member again (`0x006F580D`, §4.2). So a null pointer
    means "no grid now", and a read is safe only between `Init` and `Shutdown`.
  - **Optional check of the grid itself** through its own interface: `GetInstanceID()` (slot 5) is `0x89D5B91A`
    (fire) or `0x49D5BBCE` (police) after `Init`, also on a loaded city, and `GetTractSize()` is 2.
- **Judging unadjusted mismatches** (PR16). No original behaviour bears on it: the original never compares the
  grids, so whether a shadow judges or only logs them is the implementer's choice. Facts that matter for the
  choice: the unadjusted grid is rebuilt from scratch on every station, funding, strike or dispatch change (§5.1),
  so it carries no lag, unlike the adjusted grid; but its dispatch cones (§5, active / inactive) follow the
  automata, which a shadow may not mirror, so tracts under an active dispatch can differ for reasons outside the
  simulator.
- Other code reads the adjusted grid through `GetCellValue`, `GetAverageValueInCellRect`, raw `GetGridData`
  (the map view) and `+0x20` (fires). See §13.

---

## 3. State

"Saved" refers to §11. Every count is a signed 32-bit integer unless stated.

**Base (both classes)**

| State | Initial | Saved | Notes |
|---|---|---|---|
| department (`cISC4DepartmentBudget*`) | null | yes (ref) | created at `Init` if null (§4) |
| purpose ID | fire `0xEA567BC3`, police `0x0A567BAA` | no | set at `Init`; the budget item purpose of a station |
| adjusted grid, unadjusted grid | null | yes (refs) | §2 |
| station list (ordered, ref-counted occupants) | empty | yes (refs) | insertion order |
| random cell iterator (prime-step mode) | rectangle empty, never reset | yes | weather §6.7; rectangle = all tracts, set at `Init` |
| tracts per day | — | no | `countX × countZ / period` (integer division) at `Init`; period fire **50**, police **40** |
| factor | 1.0 | no | reset to 1.0 by every daily call |
| cycle counters `zoned`, `sum`, `nonZero` | 0 | yes | §5.3 |
| average coverage `avg` (f32) | 0 | yes | §5.3 |
| percent coverage `pct` (f32, a fraction 0..1) | 0 | yes | §5.3 |
| strike flag (bool) | false | yes | city-level strike in progress |
| strike date (u32 day number) | 0 | yes | day the city-level strike began |
| next call number | 1 | yes | §8.2 |
| `cRZRandom` | seeded from the wall clock (`0xFFFFFFFF`) at construction | no | §10 |
| tuning (§6.7) | code defaults | no | read at `Init` and on the reload message |
| city, simulator, message server, budget simulator pointers | — | no | |
| initialised flag | false | no | `Init` does nothing a second time |

**Fire only**: the flammability simulator (held, never called), the **fire plane dispatch source** (ref, saved),
a "loaded" flag (set by `Read`, not otherwise used).

**Police only** (§9): jail department (ref, saved); jail list and locally-funded-jail list (refs, saved);
**jailbreak list** (refs, **not** saved); `capacity`, `population`, `jailed`, `arrestsLast`, `arrestsThisMonth`,
`unarrestedThisMonth`, `crimesLast`, `crimesThisMonth` (all saved); the crime simulator (held, never called);
the **police helicopter dispatch source** (ref, saved); a "loaded" flag.

---

## 4. Lifecycle

### 4.1 `Init()` → AL = 1

Does nothing (but still returns true) when already initialised. Otherwise marks itself initialised, then:

**Fire** (`0x006AE240`):
1. purpose := `0xEA567BC3`.
2. Read the tuning (§6.7).
3. Base init (below).
4. If there is no department: `budget.CreateDepartmentBudget(0x28F55A9F, 0x4A357B40)`, `AddRef` it,
   `SetDepartmentName(0x6A3FF01C, 0xAA4935FE)` (a localised string key, group and instance),
   `SetMaxAllowedFundingPercentage(maxFunding)`, `CreateLineItem(1, true)`, then
   `GetLineItem(1).SetDisplayFlag(1, false)`.
5. Get the flammability simulator (city `+0xE8`, through the cache global `0x00B43D7C`) and hold it at `+0xCC`
   (`AddRef`). It is never called (PR12, below).
6. `SetInstanceID` on both grids (§2).
7. If there is no plane source, create a `cSC4FirePlaneDispatchSource` (class ID `0x4C0AE114`) directly:
   `GlobalHeapAlloc(0x28)` and its constructor `0x006ACE00` (`0x006AE323`–`0x006AE362`). That is the same object
   its GZCOM factory builds (§8.4). Always call its `Init` (§8.4).

**Police** (`0x006E2740`):
1. Once per process, register a shutdown callback with the app (`cISC4App` slot 69 `+0x114`
   `RegisterShutdownCallbackFunction(cb, 0)`); the callback only clears the "registered" flag. *[choice]*
   FreeCity4 need not do this.
2. Read the tuning (§6.7), then base init.
3. purpose := `0x0A567BAA`.
4. If there is no department: police department `0xA2963983` (group `0x4A357B40`), named `(0x6A3FF01C,
   0xAA493855)`, max funding := police max funding, line item 1 created and hidden as for fire.
5. If there is no jail department: department `0xA2963984` (group `0x4A357B40`), named `(0x6A3FF01C,
   0x4A4FC955)`, max funding := the **same** police max funding, line item 1 as above.
6. Hold the crime simulator (city `+0xDC`, through the cache global `0x00B43D70`) at `+0xCC` (`AddRef`). It is never
   called (PR12, below).
7. Subscribe to the crime message `0x0A159D95` (§9.2).
8. `SetInstanceID` on both grids.
9. Create the `cSC4PoliceHelicopterDispatchSource` (`0x0C0B06E8`) if absent (same pattern as fire) and `Init` it.

**Base init** (`ProtectionInit`, `0x006F6BC0`) *[compat]*:
1. Hold the city (global `0x00B43CEC`), the budget simulator (city slot 52) and the zone manager (global
   `0x00B43D14`), each with `AddRef` and only if not already held (`0x006F6BC7`–`0x006F6C0B`). Create the grids if
   absent (§2). *(Corrected: the simulator is not held here, and the message server is held in step 4.)*
2. Iterator rectangle := `[0, countX − 1] × [0, countZ − 1]` of the adjusted grid (an empty range is widened
   to one tract). The iterator is not reset here, so a new city's first daily call ends a cycle at once
   (simgrid §7.4: a never-reset iterator yields nothing).
3. Register two agents with the simulator (`AddAgent`, simulator slot 19; the simulator is read from the cache
   global `0x00B43D5C` at `0x006F6D22`, not held, and the step is skipped when it is null): level **4** (day) and level **6**
   (month), both with flags **2** (only while the city is established) and the agent name
   **"cSC4ProtectionSimulator"** — the same name for fire and police.
4. Only if the message server is not held yet: hold it (global `0x00B43CCC`, `AddRef`) and subscribe to the nine
   messages of §5 (`AddNotification`, one call each; the whole step is skipped otherwise, `0x006F6D7A`) in this order: `0x99EF1142`,
   `0x99EF1143`, `0xC8DAC052`, `0x08DAC3E7`, `0xE94671EB`, `0x894671F6`, `0xE94671EC`, `0xE94671ED`, `0xC53D10AA`
   (`0x006F6D93`–`0x006F6E09`). The two day/month IDs are not subscribed; they arrive through the agents. Each ID has
   one handler in this class, so the order has no effect inside the class; it only fixes this target's position in
   the server's per-message notification lists, relative to targets that subscribe later. *[compat]*
5. Register itself (the dispatch-source sub-object) with the dispatch manager (`RegisterDispatchSource`).
6. tracts per day := `countX × countZ / period`.

**Held but never called** *[compat]* (PR12):

- **Fire's flammability simulator and police's crime simulator.** Each is held at `+0xCC` of its class. `Init`
  takes the reference (fire `0x006AE2D4`–`0x006AE2ED`, police `0x006E285E`–`0x006E286F`) and `Shutdown` releases
  it (fire `0x006AD222`, police `0x006E03AF`).
- **No other code reads them.** Apart from the constructors, which zero the member, there is no access to that
  member in either class. The check covered all offsets that reach it from the primary or from the `+0x08`,
  `+0x0C` and `+0x10` sub-objects.
- **The simulator (`cISC4Simulator`) is never held**; step 3 reads the cache global each time.
- **Not holding them** changes only those objects' reference counts. If the city dropped its own reference
  first, the object would also be destroyed earlier. Nothing in either class can observe that. *[choice]*
  FreeCity4 holds none of the three.

### 4.2 `Shutdown()` → AL = 1

Only when initialised (clears the flag). Fire: `Shutdown` and release the plane source; base shutdown; release
flammability and the department. Police: `Shutdown` and release the helicopter source; unsubscribe
`0x0A159D95`; base shutdown; release the jail list, the locally-funded-jail list and the jailbreak list
(occupant manager `ReleaseOccupantList`); release the crime simulator, the department and the jail department.

**Base shutdown** (`ProtectionShutdown`, `0x006F56F0`) *[compat]* (PR15; traced in full, offsets relative to the
base sub-object at `+0x08`), in this order:

1. Unregister the dispatch-source sub-object from the dispatch manager (city slot 39 `+0x9C`, fetched from the
   city global now; manager slot 14 `+0x38`).
2. Hand the station list (`+0x80`) to the occupant manager (city slot 34 `+0x88`; slot 48 `+0xC0`,
   `ReleaseOccupantList`).
3. If the message server (`+0x68`) is held: unsubscribe the nine messages (server slot 6 `+0x18`) in the order
   `0x99EF1142`, `0x99EF1143`, `0xC8DAC052`, `0x08DAC3E7`, `0xE94671EB`, `0xE94671EC`, `0xE94671ED`,
   `0x894671F6`, `0xC53D10AA` (`0x006F5744`–`0x006F57BA`; not the subscription order of §4.1, which has
   `0x894671F6` before the active / inactive pair), then release it and clear the member.
4. `RemoveAgent(this)` (slot 20 `+0x50`) on `city.GetSimulator()` (slot 50 `+0xC8`), skipped when null. The
   simulator is not held, so nothing is released.
5. For the adjusted grid (`+0x78`), then the unadjusted grid (`+0x7C`): `Shutdown` (`+0x10`), `Release`, clear
   the member.
6. Release the zone manager (`+0x70`), the budget simulator (`+0x6C`) and the city (`+0x64`), in that order, each
   cleared.

AL = 1. So the budget simulator and the zone manager are held from base init (taken only if not already held)
to step 6 of the **base** shutdown, which runs before the class releases flammability or crime and its
departments. A read-only shadow host that asks the city for them each time sees the same objects; it holds no
reference across the session and cannot observe the difference.

---

## 5. Scheduling, messages and grid updates *[compat]*

| Message | ID | Handling |
|---|---|---|
| level-0 sim message | `0x66956810` | finished (true); never delivered because no level-0 agent is registered |
| day (agent level 4) | `0x66956814` | `DoDaily` (§5.3) |
| month (agent level 6) | `0x66956816` | `DoMonthly` (§6.5; police §9.5) |
| occupant inserted | `0x99EF1142` | skipped if the occupant has property `0x029244DB` (preview occupant, foundations §5.2); else §5.2 insert |
| occupant removed | `0x99EF1143` | same skip; else §5.2 remove |
| occupant state changed | `0xC8DAC052` | same skip; else §5.2 change |
| funding percentage changed | `0x08DAC3E7` | if data2 == this department's ID and the station list is not empty: rebuild the **unadjusted** grid only (§5.1) *[quirk]* |
| dispatch begin | `0xE94671EB` | no effect (data1 is compared with the dispatch type, nothing follows) |
| dispatch active / inactive | `0xE94671EC` / `0xE94671ED` | if data1 == this class's dispatch type: `GetDispatchInfo(data2)`; if it returns exactly 1, `UpdateBothGridsByPosition` (§5.2) at the dispatch's position with the radius of its `GetDispatchEffect`. Active and inactive behave the same (the flag passed along is unused). Details below (PR14) |
| dispatch end | `0x894671F6` | if data1 == the dispatch type: `GetDispatchInfo(data2)`; if found, `RevokeDispatchFromStation(info.station, info.callID)` (§8.2); no grid update |
| reload tuning | `0xC53D10AA` | if data1 == `0xE7E2C2DB` and data2 == this class's tuning instance (fire `0x68F5524A`, police `0x28DFCF89`): re-read the tuning (§6.7) |
| police only: crime | `0x0A159D95` | §9.2 |
| police only: funding percentage changed | `0x08DAC3E7` | additionally, if data2 == the jail department's ID: recalculate jails (§9.3) |

**The dispatch record** *[compat]* (PR1). `GetDispatchInfo(id, &info)` (`0x00469D40`) copies the dispatch
manager's record: 11 `u32`, 44 bytes, **with no AddRef**. The simulator reads these fields:

| Offset | Field | Set by |
|---|---|---|
| `+0x00` | dispatch type (fire 1, police 0) | the dispatch request |
| `+0x04` | dispatch ID | the dispatch manager |
| `+0x08` | **call ID**, the unit number `base + i` | written by the source's `DispatchFromStation` / `DispatchFromNearestStation` (§8.2), or taken from the pylon's `GetCallNumber` (`InitDispatch` `0x00469E50`) |
| `+0x10` | **active** (byte) | `0x00469BE0`: active := (the dispatch state == 5). On a change it posts `0xE94671EC` (became active) or `0xE94671ED` (became inactive), data1 = type, data2 = ID. `EndDispatch` (`0x0046ACEE`) clears it |
| `+0x14`, `+0x18`, `+0x1C` | position x, y, z (`float32`) | the dispatch request |
| `+0x20` | the **source station** occupant (`cISC4Occupant*`, held by the record) | the station returned by `DispatchFromNearestStation`, or the pylon's source occupant |

`EndDispatch` itself calls the source's `RevokeDispatchFromStation(+0x20, +0x08)` before the end message reaches
the simulator. So the simulator's own revoke on `0x894671F6` finds the unit's bit already set, and fails with no
effect. *[compat]*

**The active / inactive handler** *[compat]* (PR14). The message handler (`0x006F647F`) calls
`DoDispatchActivated(id, active)` (`0x006F61F0`) with active = 1 for `0xE94671EC` and 0 for `0xE94671ED`; the
flag is never read. The function:

1. does nothing if the dispatch manager global (`0x00B43D04`) is null;
2. calls the manager's slot 10 (`+0x28`) `GetDispatchInfo(data2, &info)` and goes on only if AL is exactly 1;
3. calls its own `GetDispatchEffect(info.station, &effect)` (message-target slot 12, `+0x30`) with
   **`info.station` = the record's `+0x20`**, the same occupant `UpdateUnadjusted` passes (§7.2). The effect
   record is an uninitialised stack record, and the call's AL is **ignored**;
4. calls `UpdateBothGridsByPosition(info.x (+0x14), info.z (+0x1C), effect.radius)` (`0x006F6110`, §5.2).

The radius is therefore the station's dispatch radius scaled by its efficiency (§7.2). *[quirk]* When
`GetDispatchEffect` fails (the station has no dispatch centre value, or it is 0), it has not written the radius,
and the update uses a stack leftover. *[choice]* FreeCity4 skips the update when `GetDispatchEffect` fails.

The class `DoMessage` (message-target slot 3) always returns true (finished; simulator spec); the base handler
it calls returns false for messages it does not handle, but that value is not passed on. The simulator receives the day message once per
day and the month message once per month; its rank among other month agents does not matter for its own state,
but the fire `EndOfCycle` reads the police percent coverage (§5.4), and the crime simulator calls police slots 27,
31 and 38 from its own month agent (crime §6.4).

### 5.1 Rebuilding the unadjusted grid (`UpdateUnadjusted`)

```
unadjusted.SetTractValues(0)
for each station s in list order:
    e := GetStationEffect(s)                      (§7.1; its bool result is ignored *[quirk]*)
    AddRadiusEffect<int16>(unadjusted, countX, countZ,
                           trunc(e.x × invWidthX), trunc(e.z × invWidthZ),
                           e.radius × invWidthX,  float(e.centre), float(e.outer))
n := dispatchManager.GetDispatchCount(type)
ids := dispatchManager.GetDispatchIDArray(type)   (a stack buffer when n < 64)
for each id:
    if GetDispatchInfo(id, &info) and info.active:
        e := GetDispatchEffect(info.station)       (§7.2; the record's +0x20, §5)
        cone as above, centred on info's position (x, z)
```

The station loop uses **one** effect record for all stations; it is not initialised before the loop
(`0x006F5C30`). A station whose effect fails therefore draws a cone from whatever the record holds (§7.1 says which
fields a failed read leaves alone): for fire, the values read before the failure, unscaled; for police (centre 0
or absent), the **previous station's whole effect**, so that cone is drawn twice, or uninitialised stack data when
it is the first station *[quirk]*. *[choice]* FreeCity4 skips a station whose effect fails. Stations whose effect
fails cannot normally be in the list (insert requires success, §5.2), so this only matters when a station's
properties change after insertion. The radius is divided by the tract width (metres → tracts); simgrid §7.1
rounds it for the loop bounds and clamps to `int16`. *[compat]*

**When `GetDispatchEffect` fails** *[compat]* (PR17; `0x006F5D83`–`0x006F5DFE`). The dispatch loop passes the
**same** stack record as the station loop (`[esp+0x24]` at both `0x006F5C87` and `0x006F5DA9`). It ignores the
call's AL and always draws the cone. A failed call (the source station's dispatch centre is absent or 0, §7.2)
has written only x, z and centre (all 0). The radius and outer value are still in the record from the previous
effect read into it:

- the previous dispatch's `GetDispatchEffect`, when an earlier dispatch in this rebuild got that far;
- otherwise the last station's `GetStationEffect` (§7.1: its scaled values on success, or what a failed read left);
- otherwise, with no station in the list and no earlier dispatch, uninitialised stack data *[quirk]*.

The cone therefore has `c = 0` and the leftover `o` and `r`. Unless `o = 0` or `r ≤ 0`, it does not add 0: it adds
an **inverted cone**, 0 at the dispatch's tract and rising towards `o` near the leftover radius.

With shipped data the call never fails. The three fire station exemplars (group `0x07BDDF1C`, instances
`0x030B0000`, `0x038E0000`, `0x03100000`) all have dispatch centre 96, outer 32 and radius 128.0. The four police
stations (`0x03860000`, `0x03880000`, `0x030F0000`, `0x030C0000`) all have 200, 20 and 48.0. `EP1.dat` adds
none. Only a custom station without the dispatch centre reaches this case.

Example (tract width 32 m): the last station drawn had the scaled effect outer 10 and radius 150.0 m. A
dispatch whose station has no dispatch centre then draws `r = 150 / 32 = 4.6875` tracts with `c = 0`, `o = 10`,
so `k = float32(−10 / 4.6875) = −2.1333334`. Its tract gets `+0`; tracts at distance 1, 2, 3 and 4 get `+2`, `+4`,
`+6` and `+9`; the tract at table distance 4.472 gets `+10`. Tracts at distance ≥ 4.6875 get nothing.

*[choice]* To match the original, keep one effect record across both loops, zero only x, z and centre on a
failed dispatch read, and draw the cone. A zeroed record (nothing added) is acceptable as a known difference,
but it differs whenever a dispatch's source station lacks the dispatch centre. For the uninitialised case, zero
is the only reproducible value.

### 5.2 Station insert, remove and change

**Insert** (`0x006F6610`), for occupants of type building `0x278128A0` whose `GetStationEffect` succeeds:
1. `AddRef` and append to the station list.
2. n := the station's dispatch-unit count (§8.1), read through `GetDispatchesAtStation(s, null, null, &n)`;
   `SetStationAvailabilityFlags(s, (1 << n) − 1)` (all free; the shift count is taken mod 32, so n = 32 gives 0).
   *[quirk]* `n` is not initialised first: when the count property is absent it is stack garbage.
   *[choice]* FreeCity4 uses n = 0 then.
3. Choose the base call number (§8.2); `SetStationBaseCallNumber(s, base)`; nextCall := max(nextCall, base + n)
   (unsigned 32-bit compare; `base + n` wraps mod 2^32).
4. Budget (§6.1 terms): if `GetBudgetItemForPurpose(holder, purpose, item)` succeeds and
   `budget.GetDepartmentBudget(item.departmentID)` is not null, call `AddLocallyFundedObject(holder, purpose)` on
   that department. "Its department exists" always means this lookup by the item's department ID succeeds; the
   simulator's own department pointer is not used.
5. `UpdateBothGridsByPosition(e.x, e.z, e.radius)`: tract (tx, tz) := `adjusted.PositionToTract(x, z)`;
   r := trunc(radius × invWidthX); rebuild the unadjusted grid (§5.1); copy the box
   `[tx − r, tx + r] × [tz − r, tz + r]`, clipped to the grid, into the adjusted grid (§2).

**Remove** (`0x006F64F0`): if a building and in the station list: erase it; if it has a budget item and the
item's department exists (as in insert step 4), `RemoveLocallyFundedObject(holder, purpose)`; `UpdateBothGridsByPosition` with its effect;
`Release`. The availability and call-number properties stay on the occupant.

**Change** (`0x006F6250`): if a building and in the station list: `UpdateBothGridsByPosition` with its current
effect.

Police adds jail handling and a station message to all three (§9.4).

### 5.3 Daily cycle (`DoDaily`)

`DoDaily` sets factor := 1.0 and then runs the shared daily step (`0x006F6A70`):

```
n := tractsPerDay
while n > 0:
    if iterator.GetNext(tx, tz):
        adjusted[tx][tz] := clamp(roundHalfEven(unadjusted[tx][tz] × factor), 0, 255)
        ProcessCell(tx, tz)                                       (class slot; a no-op in both classes)
        if any cell of the zone grid in [tx·S, (tx+1)·S) × [tz·S, (tz+1)·S) is non-zero:
            zoned += 1;  sum += v;  if v ≠ 0: nonZero += 1        (v = the new adjusted value)
        n −= 1
    else:
        avg := zoned > 0 ? f32(sum / zoned) : 0                   (float division, see below)
        pct := zoned > 0 ? f32(nonZero / zoned) : 0
        zoned := sum := nonZero := 0
        EndOfCycle()                                              (§5.4)
        iterator.Reset(shuffle = true)                            (one C rand() call; simgrid §7.4)
```

The averages (`0x006F6B3A`) are **float** divisions: both integers are loaded exactly (`fild`, signed 32-bit, no
rounding even above 2^24) and divided once, rounded to float32 (foundations §18). `zoned > 0` is a signed test.
`sum` and `nonZero` are added as signed 32-bit integers (`sum` wraps only past 2^31, about 8.4 million tracts at
255). *[compat]*

The zone grid is the zone manager's slot-5 grid (`+0x14`), read raw through `GetGridData() const` as rows `[x][z]`;
"zoned" means any non-zero zone type. **Which grid** *[compat]* (PR10):

- **The grid.** Slot 5 returns the zone manager's member `+0x48`. That is a `cSC4SimGrid<int8>`: vtable
  `0x00A87950`, CLSID `0x49B9E603` (simgrid §2). It is created in the zone manager's `Init` (`0x004DE26F`–
  `0x004DE2D0`) with tract size 1, so it has one byte per city cell.
- **The read.** The daily step fetches the data pointer once per call (`0x006F6A9C`, `0x006F6AA3`). The tract test
  (`0x006F6180`) compares each byte with 0.
- **`GetZoneType(x, z)`** (slot 12, `0x004DC740`) reads **the same byte of the same grid**, sign-extended. So
  "GetZoneType ≠ 0" equals "byte ≠ 0" for every cell inside the grid, and a replacement may use slot 12. It
  costs one virtual call per cell instead of one per day.
- **Outside the grid.** Neither path checks bounds. Both read past the row vector, which is garbage or a crash.
  "0 outside the city" is FreeCity4's choice, the same as the skip below.

**S** is the simulator's own tract-size constant (2, set with the tuning,
§6.7, and the same value passed to `SetTractSize` when the grids are created), **not** the adjusted grid's tract
size, and it is not clamped (`0x006F6180`). The cells read are not bounds-checked. With a loaded adjusted grid of
another tract size (saved geometry, §2) the tract range would no longer match S = 2 and the original would read
the wrong cells or outside the zone grid. *[choice]* FreeCity4 uses S = 2 and skips cells outside the zone grid
(identical for every grid the game creates). Tracts per day: 64-cell city 1024 tracts → fire 20, police 25; 128 → 4096 tracts →
81 / 102; 256 → 16384 tracts → 327 / 409. A full cycle therefore takes about 51 (fire) or 41 (police) days.
*[compat]*

### 5.4 End of cycle

- **Fire**: history `0x2A11D203` := trunc(pct × 100); `0x2A11D204` := trunc(avg);
  `0xAAAD4D87` := trunc((pct × 100 + policePct × 100) × 0.5), where policePct is read through police slot 19
  (the city's police simulator). Then post `0x68F54CE0` (§12).
- **Police**: history `0x4A11D354` := trunc(pct × 100); `0x4A11D355` := trunc(avg). Then post `0xA8DAF510`.

History values are recorded with `cISC4HistoryWarehouse` slot 12 `RecordHistoryData(id, value)`. The constants
100 and 0.5 are float32 (`0x00A80AA0`, `0x00A84D2C`); each product and the sum round to float32, and `trunc` is the
CRT `_ftol` (foundations §18.1).

---

## 6. Formulas *[compat]*

### 6.1 Station funding fraction

```
fundingOf(station, purpose):
    if not budget.GetBudgetItemForPurpose(holder, purpose, item): 0
    dept := budget.GetDepartmentBudget(item.departmentID)
    if dept is null: 1.0
    else if dept.IsLocallyFundedObjectInDepartment(holder, purpose): item.localFundingPercentage
    else: dept.GetFundingPercentage()
```

"The item's department exists" (here, in §5.2 and §9.4) means: the lookup by the item's department ID returns a
non-null department. Whether the station is locally funded is asked of that department (slot 28), not read from the
item. *[compat]* (`0x006AD430`, `0x006E0E70`). The jail version (§9.1) differs only in its first line: **no jail
item gives 1.0**, not 0 (`0x006E06E0`).

### 6.2 Station efficiency (`GetStationEfficiency(s)`, fire slot 10, police slot 25)

```
e := curve(fundingOf(s, purpose) × 100) × 0.01
fire:   if the station's lot exists and has no road access: e × noRoadMultiplier
both:   if IsStationOnStrike(s): e × strikeEfficiency
```

Precision (PR3) *[compat]*: every `× 100` and `× 0.01` in this spec is a multiplication by a **float32** constant,
100.0f (`0x00A80AA0`) or 0.01f = 0x3C23D70A = 0.0099999998 (`0x00A823A4`); no `double` 100 or 0.01 is used
anywhere in the three classes or in the percentage getter `0x005FDAC0` (every x87 memory operand in
`0x006AC800`–`0x006AE400`, `0x006DF800`–`0x006E2C00` and `0x006F5000`–`0x006F7100` was listed). Each product is one float32 rounding (foundations §18). So
`curve × 0.01` is **not** `curve / 100`: for example 10 × 0.01f = 0.099999994 (0x3DCCCCCC, an exact tie rounded to
even), 20 × 0.01f = 0.19999999, 40 × 0.01f = 0.39999998, 25 × 0.01f = 0.25, 50 × 0.01f = 0.5, 100 × 0.01f = 1.0.
The only `double` constants are the fire strike draw bounds 0.0 and 100.0 (§6.3), passed as `double` arguments.
This was re-checked image-wide for RS1 (2026-10-13; foundations §18.3): the residential worked examples that assumed a
double 0.01 were the ones in error, and this section stands.

curve: fire `0x29233E8D`, police `0x68DDAE93`. The police version has no road factor. The lot comes from the lot
manager (`GetOccupantLot`), road access from `cISC4Lot::HasRoadAccess`.

### 6.3 Strike state of one station (monthly)

The station's strike start day is occupant property `0xEAA69F83` (u32; 0 or absent = not striking).
`IsStationOnStrike(s)` is true when that property exists and is non-zero. Let `now` = the simulator date's day
number and `f` = the station's budget item local funding fraction (1.0 if it has no item).

```
if striking (start ≠ 0):
    if (now − start) > maxStrikeDuration (unsigned): end
    else if f ≥ minFundingToEnd: end
else:
    fire:   draw := rng.RandomDoubleRangeUniform(0, 100);  start if draw < curve98(f × 100)
    police: draw := rng.RandomDoubleUniform();             start if draw < curve9E(f × 100) × 0.01
on start: property := now; automata controller manager AddControllersFromOccupantGroup(name, s, 0)
on end:   property := 0;   automata controller manager RemoveControllersFromOccupantGroup(name, s)
return changed (AL)
```

name: fire **"striking_fire"**, police **"striking_police"**. Exactly one draw per non-striking station.

**The controller calls** *[compat]* (PR3; fire `0x006ADD3E`–`0x006ADD65`, police `0x006E199D`–`0x006E19C4`;
the jailbreak start `0x006E1C2A`–`0x006E1C46` uses **"jailbreak"** the same way). The name is a temporary
`cRZString`, built from the `char*` global (`0x00B0D288`, `0x00B0D3F8`, `0x00B0D3F4`) and destroyed after the
call. It is passed by reference to slot 10 `AddControllersFromOccupantGroup(name, occ, 0)` (`0x00517150`) or slot
12 `RemoveControllersFromOccupantGroup(name, occ)` (`0x0051AB00`). These are the **by-name** overloads. Slots 9
and 11 are the by-ID overloads, which take a `u32` group ID (the script system's `GetOccupantGroup(u32)`,
`0x0051CFF0`).

The by-name overloads find the group through the script system's `GetOccupantGroup(name)` (slot 12,
`0x0051CFC0`). That is a string-keyed hash-table lookup (`0x0051C9F0`; hash `h := h·5 + signed char`), and it
reads the argument's **game-layout string directly**, with no virtual call: the `char*` begin at `+0x04` and the
end at `+0x08` (the game's `cRZString` is vtable, then the STLport string begin / end / capacity, then a
reference count, 0x14 bytes, `0x00408480`). The argument must therefore have that layout. Any other
`cIGZString` implementation is read as raw pointers. A string class whose `+0x04` holds inline characters
(a small-string buffer) makes the lookup dereference text as an address and **crash**. If the group is not
found, the call returns AL = 0 and does nothing.

### 6.4 Chance of strike (`GetChanceOfStrike`, fire slot 12, police slot 11)

1.0 when the city-level strike flag is set; 0 with no stations; otherwise, over all stations with
`c := curve(f × 100) × 0.01` (f as in §6.3; fire curve `0x29233E98`, police `0x68DDAE9E`):
`p := p + c − p·c` starting from p = 0 (the chance that at least one station strikes).

### 6.5 Monthly strike pass

Fire `DoMonthly` and the police `CheckForStrikeStartOrEnd` (`0x006E2570`):

```
changed := false; striking := 0
for each station s in list order:
    changed := changed or UpdateStrikingState(s)          (§6.3)
    if IsStationOnStrike(s):
        striking += 1
        if changed: BeginStrike(s)                        (no effect if the city strike flag is set)
if changed:
    if striking == 0 and IsOnStrike(): EndStrike()
    rebuild the unadjusted grid (§5.1)                    (the adjusted grid catches up through §5.3)
```

`BeginStrike(s)`: if the flag is clear: flag := true, date := now, post the strike-begin message with `s`
(§12), AL = 1; else AL = 0. `EndStrike()`: if the flag is set: flag := false, date := 0, post strike-end, AL =
1; else AL = 0. `GetStrikeLength()` = now − date, or −1 (0xFFFFFFFF) when date = 0.

*[quirk]* The city-level strike only ends in a month in which some station's state changed; a strike that is
already over at the station level keeps the flag if nothing changes. Which station is named in the begin message
depends on list order and the sticky `changed` flag.

### 6.6 Budget hooks

- `GetStationFunding(s, int32* cost, SC4Percentage* p)`: false if the station has no item for the purpose; else
  `*p := item.localFundingPercentage`, `*cost := low 32 bits of item.cost`, each only when its pointer is not
  null; true.
- `SetStationFunding(s, const SC4Percentage& p)`: item := `GetBudgetItemForPurpose(holder, purpose)` (the
  simulator's purpose); false if there is none. department := `GetDepartmentBudget(item.departmentID)`, i.e. the
  **item's** department, not the simulator's own (`0x006AD530`); false if null. Else
  `department.SetLocalFundingPercent(holder, p, purpose)`, true. The two differ only for a building whose budget
  item names another department.
- Message-target slot 11 (`ProtectionGetStationFunding(s, float* out)`): `*out := item.localFundingPercentage`,
  true; false without an item.

### 6.7 Tuning *[compat]*

Exemplar group `0xE7E2C2DB`, type `0x6534284A`; instance fire `0x68F5524A`, police `0x28DFCF89`; loaded through
the resource manager. Getters: "pct" (`0x005FDAC0`) reads any numeric property and multiplies it by the float32
0.01f (one float32 rounding, §6.2), so shipped percentages are **not** the decimal fractions: 10 → 0.099999994
(0x3DCCCCCC), 20 → 0.19999999 (0x3E4CCCCC), 50 → 0.5, 60 → 0.59999996, 80 → 0.79999995 (0x3F4CCCCC), 90 → 0.9f
(0x3F666666), 110 → 1.1f (0x3F8CCCCD), 120 → 1.1999999 (0x3F999999, not 1.2f = 0x3F99999A). "f32" and "s32" read
the value as is. Code defaults are float32 constants stored directly (1.2 = 0x3F99999A, 1.1 = 0x3F8CCCCD,
0.9 = 0x3F666666, 0.5, 0.01f). The "Shipped → used" columns give the exact float32 values. Every curve uses the **default** pattern of foundations §4.1: the code default is rebuilt from a
point list on every read, then the key loader replaces it only when the property exists.

**Fire**

| Property | Name (re/tuning) | Getter | Code default | Shipped → used |
|---|---|---|---|---|
| `0x29233E8D` | funding efficiency curve | curve | (0,0) (100,100) (120,110) | (0,10) (50,25) (75,45) (90,75) (100,100) (120,110) |
| `0x29233E98` | strike response curve | curve | (0,20) (100,100) | (0,100) (25,25) (50,10) (70,0) |
| `0x29233E8F` | max allowed funding | pct | 1.2 | 120 → 1.1999999 (0x3F999999) |
| `0x29233E94` | strike efficiency multiplier | f32 | 0.5 | 0.2f (0x3E4CCCCD, not scaled) |
| `0x29233E96` | min funding to end strikes | pct | 0.9 | 90 → 0.9f (0x3F666666) |
| `0x29233E95` | max strike duration (days) | s32 | 60 | 548 |
| `0x69B425DB` | station no-road multiplier | f32 | 0.8 | 0.8 |

**Police**

| Property | Name (re/tuning) | Getter | Code default | Shipped → used |
|---|---|---|---|---|
| `0x68DDAE93` | police funding → efficiency curve | curve | (0,0) (100,100) (120,110) | same |
| `0x68DDAEA0` | jail funding → efficiency curve | curve | (0,0) (100,100) (120,110) | (0,0) (10,50) (100,100) (120,110) |
| `0x68DDAE9E` | strike response curve | curve | (0,20) (100,100) | (0,100) (25,25) (50,10) (70,0) |
| `0xEA614F63` | jailbreak response curve | curve | (0,20) (100,100) | (0,0) (80,0) (90,10) (100,100) |
| `0x68DDAE95` | max allowed funding | pct | 1.2 | 120 → 1.1999999 (0x3F999999) |
| `0x68DDAE97` | monthly rehabilitation percent | pct | 0.01f | 10 → 0.099999994 (0x3DCCCCCC) |
| `0x68DDAE9A` | strike efficiency | pct | 0.5 | 20 → 0.19999999 (0x3E4CCCCC) |
| `0x68DDAE9B` | max strike duration (days) | s32 | 10 | 548 |
| `0x68DDAE9C` | min funding to end strikes | pct | 0.9 | 90 → 0.9f (0x3F666666) |
| `0x68DDAE9F` | arrest threshold | s32 | 16 | 20 |
| `0xEA614F64` | jailbreak escapee percentage (lo, hi) | f32 array, ≥ 2 values, each × 0.01 | **1.0, 5.0** (not scaled) *[quirk]* | 10, 50 → 0.099999994, 0.5 (each × 0.01f, `0x006E2518`) |
| `0xEA614F65` | end-jailbreak capacity | pct | 0.5 | 80 → 0.79999995 (0x3F4CCCCC, not 0.8f) |
| `0x68DDAE92` | (unnamed) jail overflow factor | pct | 1.1f | not shipped → 1.1f (0x3F8CCCCD) |

The tract size (2) and the period (fire 50, police 40) are code constants. *[quirk]* The code-default strike
curves rise with funding (more strikes at full funding); only the shipped curves make sense. A missing exemplar
leaves every value at its code default.

---

## 7. Station and dispatch effects *[compat]*

An effect is a record (position x, z in metres; radius in metres; centre value; outer value; original centre).

### 7.1 `GetStationEffect(s)` (message-target slot 14)

| | Fire (`0x006AD7C0`) | Police (`0x006E0D50`) |
|---|---|---|
| centre | s32 `0x08F55DC0`; **fails** when absent | s32 `0x48D71ED0`; **fails at once** (AL = 0) when absent or 0 |
| outer | s32 `0x08F55DC1`; absent → 0 | s32 `0x48D71ED1`; absent → field unchanged |
| radius | f32 `0x08F55DC2`; **fails** when absent | f32 `0x48D71ED2`; absent → field unchanged, **no failure** |
| position | occupant `GetPosition` (x, z) | same |
| efficiency e | prop occupants (QI `0xE9793A65`): curve8D(department funding × 100) × 0.01; else §6.2 | prop occupants: curve93(department funding × 100) × 0.01; else §6.2 |

Order and partial failure (PR2) *[compat]*:

- **Fire**: set centre, outer and radius to 0; read centre, outer, radius in that order (all three are always
  read); then write position and original centre := centre. Fail (AL = 0) if the centre **or** the radius was
  absent; the record then holds the raw values read and the position, unscaled. Otherwise compute e, scale, AL = 1.
- **Police**: set only centre to 0 and read it. If it is 0 (absent or zero), return AL = 0 at once: outer, radius,
  position and original centre keep the caller's previous values. Otherwise read outer and radius (their results
  are ignored; an absent one leaves the caller's value), write position and original centre, compute e, scale,
  AL = 1. *[quirk]* A police station without a radius property succeeds with a stale radius. *[choice]* FreeCity4
  uses 0 for an absent outer or radius.

On success the effect is scaled (`AdjustProtectionEffect`):
radius := radius × e; centre := trunc(centre × e); outer := trunc(outer × e); the original centre is kept. "Department funding" is the
department's `GetFundingPercentage()` fraction; prop stations ignore local funding, road access and strikes.
*[compat]*

### 7.2 `GetDispatchEffect(station)` (message-target slot 12)

centre / outer / radius from the occupant's properties: fire `0x29233E9A` / `0x29233E9B` /
`0x29233E9C`, police `0x48D71ED9` / `0x48D71EDA` / `0x48D71EDB`. Position 0 (the caller uses the dispatch's
position).

*(Corrected, PR14: the effect **is** scaled by the station's efficiency, and AL depends on the centre only.)*
Fire `0x006AD720`, police `0x006E0CB0`, the same steps:

1. x := z := 0 (`+0x00`, `+0x04`) and centre := 0 (`+0x0C`); read the centre (s32).
2. If the centre is absent or 0: AL = 0. Outer, radius and the original centre keep the caller's values.
3. Read outer (s32, `+0x10`), then radius (f32, `+0x08`). An absent one keeps the caller's value; neither read
   can fail the call.
4. Original centre (`+0x14`) := centre.
5. e := `GetStationEfficiency(station)` (fire slot 10 `+0x28`, police slot 25 `+0x64`; result through a hidden
   pointer; §6.2, so funding, road access and strikes count), then `AdjustProtectionEffect` (`0x006F56A0`, §7.1):
   radius := radius × e, centre := trunc(centre × e), outer := trunc(outer × e).
6. AL = 1.

Example: a fire station with dispatch centre 100, outer 20 and radius 300.0 at efficiency 0.5 gives centre 50,
outer 10, radius 150.0 and original centre 100.

**The caller's record** *[compat]* (PR18). Neither caller zeroes the record, and both ignore AL:

| Caller | Record passed | What a failed call leaves in radius, outer and original centre |
|---|---|---|
| `UpdateUnadjusted` (`0x006F5DB1`) | the station loop's record, shared by every station and dispatch of the rebuild | the previous dispatch's values, else the last station's (§5.1, PR17), else uninitialised stack data |
| `DoDispatchActivated` (`0x006F6221`) | a fresh, uninitialised stack record | uninitialised stack data; only the radius is used (§5) |

So "the caller's values" in steps 2 and 3 are never a zeroed record in the original. The handler's record (PR20)
does not inherit `UpdateUnadjusted`'s values: it is a different stack frame (`0x006F61F0` reserves 0x44 bytes and
writes nothing to the effect record at `+0x04` before the call at `0x006F6221`), so it holds whatever earlier,
unrelated calls left there. *[choice]* FreeCity4 may
pass a zeroed record. That reproduces the original when the call succeeds and the station has both the outer
and the radius property, which every shipped station has. An absent one keeps the leftover in the original
and 0 in a zeroed record. On failure, see §5.1 for the rebuild and §5 for the handler (skip the update).

*Which occupant* *[compat]* (PR2; corrected: an earlier version called it the automaton). There are **two**
callers (corrected, PR14): `UpdateUnadjusted` (`0x006F5DB1`) and the dispatch active / inactive handler
(`0x006F6221`, §5). Both pass the dispatch record's `+0x20`. That is the **source station** the unit
left from (§5), not the vehicle. The shipped data agrees: these six properties exist only on the station building
exemplars (group `0x07BDDF1C`: three fire stations, four police stations), never on a vehicle. The pointer is
already a `cISC4Occupant*`, and the original uses it **borrowed**: it does no QueryInterface and no AddRef, and
the dispatch record holds the reference. A replacement that QIs it for `0x26BB767E` gets the same object with one
more reference, which it must release.

### 7.3 Coverage queries

- `GetOccupantCoverage(s, const SC4Percentage& p, float& coverRadius, float& stationRadius)` (fire slot 21, police
  slot 20): AL = 0 if `GetStationEffect` fails. Else stationRadius := e.radius; t := trunc(originalCentre × p);
  if t > centre (unsigned): coverRadius := 0; else if t < outer: coverRadius := radius; else
  coverRadius := (centre − t) × radius / (centre − outer). AL = 1. *[quirk]* Fire sets both outputs to 0 first;
  police leaves them untouched when the effect fails.
- `GetStationDispatchRadius(s)`: the dispatch effect's radius (ST0), or 0.0 when it fails.
- `GetAverageCoverage()` = avg (ST0). `GetPercentCoverage()` returns a **pointer** to `pct` (a fraction).
- `GetTotalDispatchCount()`: Σ over stations of the dispatch-unit property (fire `0x08F55DC5`, police
  `0x48D71ED5`); a station without the property adds 0. Signed 32-bit sum, EAX.
- `GetMaxPossibleDispatchCount()` (dispatch-source slot 5, `0x006F5B00`, shared): one local `count := 0`, then
  for each station in list order: `GetDispatchesAtStation(s, null, null, &count)` through the dispatch-source
  vtable (slot 7), and `total += count`. The local is **not** reset per station and `GetDispatchesAtStation` only
  writes it when the property exists, so a station without the count property adds the **previous** station's
  count (0 before the first station that has one) *[quirk]*. With every station carrying the property (all shipped
  stations do *[inferred]*) it equals `GetTotalDispatchCount()`. Returns the signed 32-bit sum in EAX. *[compat]*

---

## 8. Dispatch *[compat]*

### 8.1 Per-station unit state

Each station has `count` units (fire `0x08F55DC5`, police `0x48D71ED5`), an **availability bit mask** (fire
`0x69B425DE`, police `0x48D71ED8`; bit i set = unit i free) and a **base call number** (fire `0x69B425DD`, police
`0x48D71ED7`). Unit i has call ID `base + i`. All three are occupant properties written by the simulator.
`GetDispatchesAtStation(s, u32* flags, u32* base, u32* count)` returns false when `count` is absent, or when a
requested `flags`/`base` is absent; `flags` is masked with `(1 << count) − 1` (shift count mod 32). Each
requested output is written as soon as its property is read, in the order count, flags, base, so a call that
fails on `base` has already written `count` and `flags`; an output whose property is absent is left unchanged.
*[compat]*

### 8.2 Allocation

- **Base call numbers**: on insert, mark the call IDs used by the other stations (`[base, base + count)`) in a bit
  vector of size nextCall, then take the first run of `count` free IDs starting at 1. Example: A (2 units) → base
  1; B (3) → base 3; nextCall := 6.

  Exact search (`0x006F6610`; PR10) *[compat]*. `nextCall`, `base`, `count` and the candidate are **unsigned
  32-bit**; there is no limit and no validation of a loaded `nextCall`.
  1. A `vector<bool>` of `nextCall` bits is allocated (all false) on **every** insert.
  2. For each other station (in list order, skipping the new one), `GetDispatchesAtStation(t, null, &base,
     &count)`; set bits `base … base + count − 1`. The two locals are not reset between stations, so a station
     without the properties re-marks the previous station's range *[quirk]*. Bit indices are not bounds-checked.
  3. If `nextCall ≤ 1`: base := 1. Otherwise c := 1 and repeat: look for the first set bit among `c … c + n − 1`
     (n = the new station's count); if none, stop with base := c; else, with j the first such offset,
     c := c + j + 1, and stop with base := c as soon as c ≥ nextCall. Bits at or beyond nextCall can be read (when
     `c + j ≥ nextCall`) without a bounds check: they are 0 inside the vector's last 32-bit word and heap data
     beyond it.
  4. nextCall := max(nextCall, base + n), unsigned, `base + n` wrapping mod 2^32.

  When every other station's range lies below nextCall and the out-of-range bits read as 0, the result is simply
  **the smallest c ≥ 1 such that no other station's call ID lies in `[c, c + n)`**.

  Huge `nextCall`: the game itself keeps `nextCall` at most one more than the highest call ID ever allocated, so a
  huge value only comes from a save (`u32`, §11). The original then allocates `nextCall / 8` bytes on every insert
  (512 MiB at 2^32 − 1; an allocation failure ends the game). A sum `base + n` past 2^32 wraps, and nextCall then
  stays put. *[choice]* FreeCity4 computes the smallest-free-run result above without the bit vector (call IDs at
  or beyond nextCall count as free), and saturates `base + n` at 2^32 − 1 instead of wrapping. The results are
  identical whenever the original neither runs out of memory nor reads a non-zero out-of-range bit, and no sum
  wraps.
- **`DispatchFromStation(s, u32* id)`** (dispatch-source slot 9): read the unit state. If `*id` > 0: the unit
  `*id − base` is taken (fails if that index > count; the bit is cleared **without checking that it was free**
  *[quirk]*). If `*id` = 0: take the lowest set bit i < count; with no free unit, i := count and no bit changes,
  and the call still succeeds with id = base + count *[quirk]*. Write the mask back, `*id := base + i`, AL = 1.
- **`RevokeDispatchFromStation(s, id)`** (slot 10): fails if the number of set bits ≥ count, or if the bit
  `id − base` is already set; else set it and write back, AL = 1.
- **`DispatchFromNearestStation(x, z, u32* id)`** (slot 8): among stations with a non-zero mask whose lot exists
  and has road access (a station without a lot is accepted), pick the one with the smallest 2D distance (strict
  `<`, list order breaks ties); `*id := 0`; if found, `DispatchFromStation(best, id)`. Returns the station
  pointer (EAX, borrowed) or null.
- **`CanDispatch(x, z, bool* allBusy)`** (slot 4): false if the total unit count is ≤ 0, or the terrain's sea
  level is not strictly below the altitude at (x, z). Else `*allBusy := dispatchManager.GetDispatchCount(type) ≥
  total`, AL = 1.
- `GetDispatchAutomataProperties(a, b*, c*, d*, e)` (slot 11, pop 20): fire `*b := 0x4202`; police
  `*b := 0x4207` while the disaster layer reports a disaster in progress, else `0x4228`; both `*c := (0, 0, 0)`,
  `*d := 0x11882`; AL = 1. *[inferred: the meanings of these values belong to the dispatch manager spec]*

### 8.3 Dispatch messages

The dispatch manager posts begin/active/inactive/end (§5). Active and inactive rebuild the unadjusted grid and
copy the box around the dispatch position into the adjusted grid; end returns the unit to its station without a
grid update; the cone is removed by the inactive message. The dispatch manager posts inactive only from
`UpdateDispatchState` (the unit leaves state 5, e.g. a fire put out; dispatch.md §4.3). `EndDispatch` posts **no**
inactive message (`0x0046ACE0`, dispatch.md §4.4): a dispatch revoked while still active, and a dispatch dropped
because its station was removed, leave their cone until the simulator's next full rebuild *[quirk]* (re-specs10).

### 8.4 Air dispatch sources

`cSC4FirePlaneDispatchSource` (`0x4C0AE114`) and `cSC4PoliceHelicopterDispatchSource` (`0x0C0B06E8`) are
GZCOM-registered classes, but the simulators create them directly and call their **non-virtual**
`Init` (`0x006DF8C0`) and `Shutdown` (`0x006AC860`). `Init` holds the city and the message server, subscribes to
`0x99EF1142`, `0x99EF1143`, `0xE94671EB`, `0x894671F6`, `0xE94671EC`, `0xE94671ED` and registers the source with
the dispatch manager. Their save record is `u16 1`, `u32 n` + n occupant refs, `u32` (meaning unknown) (`Write`
`0x006AC930`). The fire class's Read (`0x006AC970`) accepts version 1 only; the police class's Read (`0x006DF970`)
accepts 0 and 1 *[quirk]* (save-records.md §7.4).

**Air station lists and unit state (DP13).** Object-start slot 6 takes a native occupant-list
reference, replaces its nodes and AddRefs each copied station, returning AL = nonempty. Both
air tables use `0x006E0010` → `0x006DFC70`. Balance prior element references before reusing
the destination: the native node clear does not release them. Slot 7 takes
`(station, mask*, base*, count*)`, returns AL success, and writes count, masked mask, then base,
retaining partial writes on failure. The complete output/property/ownership and 32-bit shift
contracts are in dispatch.md §4.5.1; no borrowed-air-list exception applies.

**How they are created** *[compat]* (PR13):

- **Direct creation.** Each simulator's `Init` does `GlobalHeapAlloc(0x28)` and calls the constructor: fire
  `0x006ACE00`, police `0x006DFF70`. It keeps the object start and takes one reference.
- **The GZCOM factory builds the same object.** The factories are registered with `AddCls` at
  `0x00460B33`–`0x00460B52`: fire `0x006ACEA0`, police `0x006E0030`. Each is that same allocation and
  constructor, so `GetClassObject(0x4C0AE114 / 0x0C0B06E8, …)` gives an object identical to the direct one.
- **The original itself uses the factory path.** When a save holds a source ref, `Read` loads the ref through the
  class registry (IID `0xC9C00435`), and `Init` then runs on that object.
- **Ask for IID `0xC9C00435`.** Both sources answer `QueryInterface` (fire `0x006AC550`) for only three IIDs:
  `0xC9C00435` → the object start, `0x452294AA` (`cIGZMessageTarget2`) → `+4`, and `0xE4FDA3D4`
  (`cIGZSerializable`) → `+8`. **IID 1 (`cIGZUnknown`) is refused**, so a GZCOM request for it fails and the object
  is released.
- **Calling `Init` and `Shutdown`.** Both take the object start in ECX. The original pairs them with the reference:
  `Init` after creation or load, and `Shutdown`, then `Release`, in the simulator's `Shutdown`
  (fire `0x006AD1F0`–`0x006AD217`).

---

## 9. Police jails *[compat]*

### 9.1 Jail efficiency, capacity and population

```
jailEff(j)        := curveA0(jailFunding(j) × 100) × 0.01        (0x006E06E0)
jailFunding(j)    := §6.1 with the jail purpose 0xEA56768A, except: no jail budget item → 1.0 (not 0)
capacity(j)       := roundHalfEven(u32 property 0x48D71EE0 × jailEff(j))   (slot 28, 0x006E0AC0)
```

The property is read as `u32` (values ≥ 2^31 are converted as unsigned); an **absent** property reads as **0**,
so the jail has capacity 0 (PR14) *[compat]*. The product rounds to float32. `roundHalfEven` here and in
Recalculate, rehabilitation and escapes is the "add 1.5 × 2^23 (`12582912.0f`), store as float32, subtract the
bit pattern `0x4B400000`" trick: exact round-half-to-even for |v| < 2^22; above that it returns other values
(for 2^22 ≤ v < 2.5 × 2^23, `v/2 + 2^21`, rounded to an integer) *[quirk]*, which shipped capacities never reach.

**Recalculate** (`0x006E1240`): capacity := Σ capacity(j); if capacity ≤ 0: ratio := 0; else if jailed > capacity:
ratio := 1; else ratio := f32(jailed / capacity); each jail's property `0x48D71EE1` := roundHalfEven(capacity(j) ×
ratio); population := Σ of those values.

`GetJailedPercentage()` (slot 36, `0x006E0B30`), returned by value through the hidden pointer, has **no guard**:

| Case (signed compares) | Result |
|---|---|
| jailed > capacity | 1.0 (constant `0x00B0D3FC`) |
| otherwise | `fild jailed; fidiv capacity`, one float32 rounding |
| → jailed = capacity = 0 | 0/0 = the x87 default NaN (invalid operation is masked): stored as float32 `0xFFC00000` *[quirk]* |
| → capacity = 0, jailed < 0 (only from a corrupt save) | −∞ |

capacity = 0 with jailed > 0 gives 1.0, not ±∞, because the first case catches it. *[compat]*

### 9.2 Crime events (`0x0A159D95`)

```
lot := QI(message IGZUnknown, 0xE818FCED);  (x, z) := lot.GetLocation()      (cells)
v   := adjusted.GetCellValue(x, z)
s   := nearest police station to the cell centre (CellCenterToPosition; LengthOfLine2D; strict <)
if v > arrestThreshold: arrestsThisMonth += 1; station stats(s, arrest)
else:                   unarrestedThisMonth += 1; station stats(s, crime)
crimesThisMonth += 1
```

**Station statistics**: occupant property `0x8A70A0B9`, a u32 vector [crimesLast, arrestsLast, crimesThis,
arrestsThis]. crime: [2] += 1; arrest: [3] += 1 then [2] += 1; monthly shift: [1] := [3], [0] := [2],
[3] := [2] := 0. The vector is read, changed and written back (`0x006E2610` for crime/arrest, the same code
inline in `ProcessMonthlyCrimeStats` for the shift). Slot 44 returns [0]; slot 43 returns [1]; both return 0 when
the property is absent. With no stations the statistics update is skipped. *[inferred]*

Length handling (PR12) *[compat]*: only when the **read fails** (property absent) is the vector made exactly 4
zeros. A vector that **is** read is used at its length without any check: a longer one keeps its extra elements
(written back unchanged); a **shorter** one is indexed past its end, so the original writes into heap memory
beyond the vector (the out-of-range increments are lost when it is written back at its own length), and slots
43/44 read past its end (an empty vector has no storage: reading it crashes) *[quirk]*. Shipped content never
creates the property; only this code does, always with 4 elements. *[choice]* FreeCity4 pads a short vector with
zeros to 4 before updating or reading it, and writes back the padded vector.

### 9.3 Monthly crime bookkeeping

`ProcessMonthlyCrimeStats()` (slot 38, called by the crime simulator; crime §6.4):
1. If arrestsThisMonth > 0: jailed := max(0, jailed + arrestsThisMonth); Recalculate.
2. arrestsLast := arrestsThisMonth; arrestsThisMonth := 0; crimesLast := crimesThisMonth; crimesThisMonth := 0.
3. Shift every station's statistics vector.
4. History `0x0A5CC155` := crimesLast; `0x0A5CC156` := arrestsLast.

`ResetUnjailedCriminals()` (slot 27, also called by crime):
```
cap      := trunc(capacity × overflowFactor)          (1.1)
released := unarrestedThisMonth;  unarrestedThisMonth := 0
if jailed ≥ cap: released += jailed − cap; jailed := cap
return released                                        (EAX)
```
No Recalculate follows; the population is refreshed by the next Recalculate.

### 9.4 Jail and station occupants

- **Insert** (after the base insert): a police station → post `0xA949569A`; a jail (has `0x48D71EE0`) →
  `AddRef`, add to the jail list, add to the locally-funded-jail list when it has a jail-purpose item and that
  department exists, `AddLocallyFundedObject(holder, 0xEA56768A)`, Recalculate.
- **Remove** (`0x006E1570`), only for occupants of type building `0x278128A0`:
  1. **Locally-funded-jail list** (PR8): search it for the occupant pointer (first match); if found, erase that
     entry and release its reference. No other condition: not whether it is a jail, has a budget item, or is
     still locally funded. *[compat]*
  2. If the occupant is in the jail list: erase it (release); if `GetBudgetItemForPurpose(holder, 0xEA56768A)`
     succeeds, call `RemoveLocallyFundedObject(holder, 0xEA56768A)` on `GetDepartmentBudget(item.departmentID)`
     **without a null check** (a missing department would crash *[quirk]*; *[choice]* FreeCity4 skips it);
     Recalculate.
  3. The base removal (§5.2); then, for a police station (slot 22), post `0xA949569A`.

  *[quirk]* The jailbreak list is not touched: a demolished jail stays in it (holding a reference) until the end
  check (§9.5) removes it, which can take until a save and reload (§9.6).
- **Change**: base handling; a police station → post `0xA949569A`.

### 9.5 Police `DoMonthly` (`0x006E2A80`)

1. **Rehabilitation**: k := roundHalfEven(f32(jailed × rehabRate)); jailed := max(0, jailed − k); Recalculate
   if k ≠ 0. With the shipped rate 0.099999994 (§6.7) no product is an exact half (checked for every jailed < 5 000 000), and **jailed ≤ 5 never
   decreases** (5 × rate = 0.49999997 → 0) *[compat]*.
2. Strike pass (§6.5).
3. A loop that reads `0x48D71EE0`, `0x48D71EE1` and `0x48D71EE4` of every jail and discards them (no effect).
4. **Jailbreak end** (`0x006E11F0`): for each jail in the jailbreak list (the next node is taken before the
   call, so ending one is safe): p := `GetJailedPercentage()` (the **city-wide** value, through the vtable, slot
   36); if p < endThreshold (ordered compare, below): `EndJailbreak(j)` (slot 41, through the vtable).
5. **Jailbreak start** (`0x006E1190`): for each jail j: chance := `GetChanceOfJailbreak(j)`; if chance > 0 (ordered;
   NaN is skipped): draw := `rng.RandomDoubleUniform()`; if draw < chance (ordered): `BeginJailbreak(j)`.

**Comparisons** *[compat]*. Each `<` above is `fld a; fcomp b; fnstsw ax; test ah, 5; jp skip` (C0 = bit 0 and
C2 = bit 2 of AH). The FPU sets (C3, C2, C0) to (0,0,0) for a > b, (0,0,1) for a < b, (1,0,0) for a = b and
(1,1,1) for unordered (a NaN operand). `test ah, 5` keeps C0 and C2; the parity flag is set when an even number of
those bits is set, and `jp` skips the action. Only a < b leaves exactly one bit (C0), so the action runs **only for
an ordered a < b; NaN never passes**. The `chance > 0` test is `test ah, 0x41; jne skip` (C0 or C3 set → skip),
which also skips NaN. In FreeCity4: `if (a < b)` with IEEE semantics (false for NaN).

`GetChanceOfJailbreak(j)` := 0 if j is null, not a jail (slot 33), the **total** population ≤ 0, or j is already
in the jailbreak list;
else curveEA614F63(GetJailedPercentage() × 100) × 0.01.

`BeginJailbreak(j)` (slot 40): append to the jailbreak list (`AddRef`); r :=
`rng.RandomDoubleRangeUniform(lo, hi)`; escaped := roundHalfEven(capacity(j) × r); jailed := max(0, jailed −
escaped); Recalculate; post `0x2A614891`; add automata controllers **"jailbreak"** for j; AL = 1.

`EndJailbreak(j)` (slot 41): only when j is a jail and in the jailbreak list: remove the "jailbreak" controllers,
post `0x2A614892`, erase and release. Returns AL = **0 always** *[quirk]*.

`GetJailCurrentUpkeepCost(j)` (slot 37): trunc(float(item.cost) × item.localFundingPercentage) for the jail
purpose, as int64 in EDX:EAX; 0 without an item.

### 9.6 Jails stuck in a jailbreak (PR15) *[compat]*

The end check (§9.5 step 4) compares the **city-wide** jailed percentage, not anything about the jail itself. A
jail in the jailbreak list (demolished or not) therefore leaves it only when `GetJailedPercentage()` is an ordered
value below endThreshold (0.79999995 shipped). When the **total** capacity is 0 (the last jail demolished, or every
jail at capacity 0 through funding or an absent capacity property), §9.1 gives:

- jailed > 0 → 1.0, not below the threshold;
- jailed = 0 → NaN (`0xFFC00000`), and NaN is never below anything (§9.5 Comparisons).

Either way **the jail stays in the list**. With capacity 0, the crime simulator's monthly `ResetUnjailedCriminals`
(cap = trunc(0 × 1.1) = 0) sets jailed to 0, giving NaN; arrests booked by `ProcessMonthlyCrimeStats` raise it
again, giving 1.0 until the next reset. Which of the two the police month agent sees depends on the month order of
the crime and police agents (foundations §13); both fail the test. No other code removes the entry: `EndJailbreak` (`0x006E1C90`) has no direct caller and its
only reference is the vtable slot (`0x00AB2534`); the only virtual calls to slot 41 found are the end check's (scan
of every `call [reg + 0xA4]` near a police-simulator global `0x00B43D98` or `GetPoliceSimulator` call: none);
occupant removal never touches the list (`0x006E1570`). The entry goes away only when:

1. total capacity becomes positive again (a new or refunded jail) and jailed / capacity < endThreshold at a month
   end; `EndJailbreak(j)` then still accepts the demolished jail, because it checks only `IsJail(j)` (the
   occupant still has property `0x48D71EE0` *[inferred: the removed occupant object is kept alive by the list's
   reference and its properties are not cleared]*) and list membership; it removes the "jailbreak" controllers,
   posts `0x2A614892` naming the demolished occupant, erases and releases it;
2. the city is saved and loaded (the list is not saved, §11);
3. `Shutdown` releases the list.

While stuck, the entry is visible: `IsInJailbreak(j)` is true, `GetJailbreakBuildings` returns it, and its
"jailbreak" automata controllers stay attached. It does not block jailbreaks at other jails (`GetChanceOfJailbreak`
only tests the jail's own membership), but with capacity 0 the population is 0, so no jail can start one either.
A jail stuck with other jails present leaves at the first month end at which the city-wide percentage drops below
the threshold. *[choice]* FreeCity4 reproduces this exactly, including the NaN (IEEE `<` is false for NaN); it
must not divide-guard `GetJailedPercentage`.

---

## 10. Random numbers *[compat]*

Each simulator object owns a `cRZRandom` seeded from the wall clock at construction (foundations §6.2; the
police instance is missing from that table). It is not saved.

| Draw | Generator, method | Count |
|---|---|---|
| fire strike start (§6.3) | own `cRZRandom`, `RandomDoubleRangeUniform(0, 100)` (`0x00913761`) | 1 per non-striking station per month |
| police strike start | own `cRZRandom`, `RandomDoubleUniform()` (`0x0091372E`) | 1 per non-striking station per month |
| jailbreak start | police `cRZRandom`, `RandomDoubleUniform()` | 1 per jail with chance > 0 per month |
| escapees | police `cRZRandom`, `RandomDoubleRangeUniform(lo, hi)` | 1 per jailbreak |
| iterator reset (§5.3) | C runtime `rand()` (foundations §12) | 1 per cycle per simulator |

`re/names_windows.tsv` calls `0x00913761` `cSC4ParticlesEffect::RandomRange`; it is the `cRZRandom` method.

---

## 11. Persistence (`cIGZSerializable`) *[compat]*

`GetGZCLSID()` returns the class ID of §1. Refs and lists of refs as in foundations §2.7.

**Base record** (`0x006F5850` / `0x006F5920`):

```
u16  version = 3
ref  department                         (read with IID 0x223E2B67)
iterator                                (weather §10 layout; mode 1, table length 0)
ref  adjusted grid                      (IID 0xC9BD939A)
ref  unadjusted grid                    (IID 0xC9BD939A)
u32  station count; count × ref station
s32  zoned
s32  sum
s32  nonZero
f32  avg
f32  pct
u8   strike flag
u32  strike date
u32  nextCall
```

Read: version > 3 → false after reading only the u16. Version < 3: pct := f32(pct × 0.01f) (older saves stored
a percentage; 75.0 → 0.75). Returns `!stream.GetError()`.

**Versions Read accepts** (PR9) *[compat]*. The three `Read`s branch on their version only where shown below;
every accepted older version is read with the **same** field layout as the current one. No other layout is
handled, so if pre-release saves had another layout the original misreads them; none is known.

| Record | Accepted | Difference by version |
|---|---|---|
| base | 0–3 | 0–2: the `pct` field holds a percentage and is scaled by 0.01f; 3: a fraction |
| fire | 0–3 | 0–2: no plane-source ref after the base record (the record ends there); 3: the ref follows |
| police | 0–8 | 0–7: no helicopter-source ref (the record ends after `crimesThisMonth`); 8: the ref follows |

The outer version and the nested base version are independent (each record has its own `u16`). Rejection (version
too high) happens before anything else is read; the fire and police records reject before calling the base Read.

**Fire record** (`0x006AD250` / `0x006AD2A0`): `u16 3`, base record, `ref` plane source. Read: version > 3 →
false; sets the loaded flag; **ignores the base result** *[quirk]*; for version > 2, releases the current plane
source (pointer cleared, then `Release`) and reads the ref with IID `0xC9C00435`. Returns `!GetError()`.

**Police record** (`0x006E03F0` / `0x006E04E0`):

```
u16  version = 8
base record                              (the police fields below are written only if the base Write succeeded)
ref  jail department
list locally-funded jails                (u32 count + refs)
list jails
s32  capacity
s32  population
s32  jailed
s32  arrestsLast
s32  arrestsThisMonth
s32  unarrestedThisMonth
s32  crimesLast
s32  crimesThisMonth
ref  helicopter source                   (only read when version > 7)
```

Read: version > 8 → false. If the base Read fails, the police fields are skipped but the result is still
`!GetError()` *[quirk]*. Sets the loaded flag (also when the base Read failed). For version > 7 it **releases the
current helicopter source** first (pointer cleared, then `Release`), exactly like fire, then reads the ref with
IID `0xC9C00435` (PR13, `0x006E05BC`) *[compat]*. For version ≤ 7 the current source is kept. On a normal load
`Read` runs on a freshly constructed simulator before `Init`, so there is no source yet and the release does
nothing; `Init` then creates a source only if the ref left none (§4.1). Tuning re-reads (§6.7) never touch the
air dispatch sources.

Recomputed on load: tracts per day, the iterator rectangle (but not its position), the tuning, the jailbreak list
(empty), the agents and subscriptions. The adjusted grid is not rebuilt on load; it is whatever was saved.

---

## 12. Messages posted *[compat]*

All are posted (queued) through the message server with `cRZMessage2Standard`.

| ID | Class | When | Data |
|---|---|---|---|
| `0x68F54CE0` | fire | end of a coverage cycle (§5.4) | void1 = IGZUnknown = the simulator |
| `0x68F54CE1` | fire | `BeginStrike(s)` succeeds | void1 = IGZUnknown = the station |
| `0x68F54CE2` | fire | `EndStrike()` succeeds | none |
| `0xA8DAF510` | police | end of a coverage cycle | void1 = IGZUnknown = the simulator |
| `0xA8DAF511` | police | `BeginStrike(s)` succeeds | void1 = IGZUnknown = the station |
| `0xA8DAF512` | police | `EndStrike()` succeeds | none |
| `0x2A614891` | police | jailbreak begins | void1 = the simulator, IGZUnknown = the jail |
| `0x2A614892` | police | jailbreak ends | same |
| `0xA949569A` | police | a police station was inserted, removed or changed | IGZUnknown = the simulator |

**Which pointer is "the simulator"** *[compat]* (PR11). It is always the **primary interface pointer**: the object's
start, whose vtable is fire `0x00AB1160` or police `0x00AB2490`. That is the pointer `GetFireProtectionSimulator` /
`GetPoliceSimulator` return. The original gets it in two ways:

- **End of a coverage cycle** (fire `0x006AD3A3`, police `0x006E0616`). It calls `QueryInterface(1, &p)` on the
  primary. Both QIs answer IID 1 with the primary itself (fire `0x006ACEFE`, police `0x006E008E`). The reference
  is released after the post.
- **Station insert, remove or change, and jailbreaks** (police `0x006E0B92`, `0x006E166F`, `0x006E1DD8`,
  `0x006E1BFA`, `0x006E1D55`). It uses `this − 8` or `this` directly, which is the same pointer.

How the message stores the two fields (`cRZMessage2Standard`, vtable `0x00ACEC00`):

- **void1** (slot 12, `0x00913322`) is stored raw, without `AddRef`.
- **IGZUnknown** (slot 33, `0x0091346B`) is held by a counted reference.
- A station or jail pointer is the occupant pointer the caller passed in.

A replacement must post its own primary pointer. That is the same object its `QueryInterface(1)` returns, so a
listener can cast void1 to the simulator interface.

---

## 13. Consumers and replaceability

### 13.1 Consumers (all through the vtables via the globals `0x00B43D78` fire and `0x00B43D98` police)

| Consumer | Calls |
|---|---|
| Advisors (safety) | police slots 29, 24, 35, 18, 19 (pointer), 21, 30, 31, 34, 11; fire slots 23, 19, 20, 22, 12 |
| Query tools | fire 10; police 25, 36, 28, 43, 44, 7, 37; funding slider: fire 8 |
| Budget window | police 8, fire 9 |
| Aircraft manager | police 22, fire 7 |
| Disaster layer | fire 25, police 26 |
| Crime simulator | police 5 (grid average), 27, 31, 38 (crime §6.4, §6.6); see §16 for slot 30 vs 31 |
| Aura simulator (bust strike) | police 13, fire 14 |
| Riot, getaway van | police grid `GetCellValue`, `GetAverageValueInCellRect` (`uint8`) |
| Fire disaster (ignite, spread) | fire grid slot 8 `GetAverageValueInCellRect` (`uint8`) |
| Tract developer | fire grid `GetCellValue` |
| Coverage-radius helper | police 22 / fire 7, `GetPoliceStations` / `GetFireStations`, `GetOccupantCoverage` with p = 0.5 (fallback radius 32) |
| Map view (`cSC4WinMapView::Update`) | both adjusted grids **raw** through `GetGridData() const` |
| Map highlights | `GetFireStations`, `GetPoliceStations` |

**Threads** *[inferred]*. The original has no locking (foundations §19.1). Every one of the 79 reads of the two
globals sits in a function of one of these kinds:

- a simulator agent or message handler;
- a view input control (query, lot placement);
- a UI window (budget, map view);
- an automaton (fire truck, patrol car, getaway van);
- the disaster layer, the advisors, a MySim;
- the coverage-radius helper or the city-init/shutdown cache fill.

All of these are driven from the main thread. The one off-main caller measured so far, `IsPaused` on a game thread
(foundations §19.2), fits the ambience sound system. Its condition objects (evaluators `0x004E2AA0`–`0x004E2CB0`,
for example `0x004E2C20` = `IsPaused() and not IsEmergencyPaused()`) read only the simulator global `0x00B43D5C`
and view globals, never `0x00B43D78` or `0x00B43D98`. So no protection slot is known to be called off the main
thread, and a replacement's off-main fallbacks are not original behaviour.

### 13.2 Replaceability findings

- Both classes are created through the GZCOM class registry (city slot and class store), so a FreeCity4 class
  registered under `0x2990C05A` / `0x6990C04F` replaces them.
- **Grids**: the map view reads both adjusted grids through `GetGridData`; the grid ownership rule applies
  (simgrid §6.4). The replacement must create both grids as game grids with the class IDs, tract size and
  instance IDs of §2 and save them by ref.
- **Air dispatch sources**: the original creates them directly (§8.4) and calls non-virtual `Init`
  (`0x006DF8C0`) / `Shutdown` (`0x006AC860`). The same object can be made through GZCOM (§8.4, PR13), but the two
  non-virtual calls can only be made by address. *[choice]* FreeCity4
  either reimplements both sources (and must then save a ref to an object that answers `0xC9C00435` and
  `cIGZSerializable` under `0x4C0AE114` / `0x0C0B06E8`), or calls those two functions by address while the
  original classes are in use. No save of the corpus refers to these classes (fire v2 and police v7 have no
  source ref), but a 1.1.641 save may, so their class IDs must stay loadable (corrected, save-records.md §7.4).
- **The dispatch-source sub-interface** (IID `0xC9C00435`) is called by the dispatch manager; the replacement
  must implement it with the slot table of §15.4.
- **Message-target slots 5–17** are protection virtuals; no outside caller was found, but the vtable layout must
  stay if FreeCity4 keeps the original base and replaces only one class *[choice: replace both together]*.
- **SDK errors** (§15) must be honoured by any caller or replacement.
- **Agent name**: both simulators register agents named "cSC4ProtectionSimulator", so anything that enables or
  disables agents by name affects both.
- **Occupant properties** written by the simulator (`0xEAA69F83` strike start, the availability mask and base
  call number, `0x48D71EE1` jail population, `0x8A70A0B9` station statistics) are saved with the occupants and
  read by the original on load; a replacement must keep writing them in the same form.
- No original code reaches into the simulators' memory directly or calls their non-virtual members.

---

## 14. Dependencies *[compat]*

Callee-pop checked with `tools/abi_extract.py --check`, return channels from the epilogues. "Pop" is callee-pop.

| Interface (vtable) | Slot | Off | Method | Pop | Return |
|---|---|---|---|---|---|
| `cISC4City` (`0x00A87620`) | 33 | `+0x84` | GetLotManager | 0 | EAX ptr |
| | 34 | `+0x88` | GetOccupantManager | 0 | EAX ptr |
| | 36 | `+0x90` | GetZoneManager | 0 | EAX ptr |
| | 39 | `+0x9C` | GetDispatchManager | 0 | EAX ptr |
| | 47 | `+0xBC` | GetAutomataControllerManager | 0 | EAX ptr |
| | 50 | `+0xC8` | GetSimulator | 0 | EAX ptr |
| | 52 | `+0xD0` | GetBudgetSimulator | 0 | EAX ptr |
| | 55 | `+0xDC` | GetCrimeSimulator (police, held only) | 0 | EAX ptr |
| | 58 | `+0xE8` | GetFlammabilitySimulator (fire, held only) | 0 | EAX ptr |
| | 65 | `+0x104` | GetPoliceSimulator (fire end of cycle) | 0 | EAX ptr |
| | 72 | `+0x120` | GetDisasterLayer | 0 | EAX ptr |
| | 97 | `+0x184` | CellCenterToPosition(int x, int z, float& px, float& pz) (below) | 16 | none: x87 stack left empty |
| `cISC4Simulator` (`0x00AB2FB8`) | 7 | `+0x1C` | GetSimDate | 0 | EAX ptr |
| | 19 | `+0x4C` | AddAgent(target, level, name, flags) | 16 | AL |
| `cIGZDate` (`0x00ACEA38`) | 26 | `+0x68` | DayNumber | 0 | EAX |
| `cIGZMessageServer2` (`0x00AD1BF4`) | 4 | `+0x10` | MessagePost(msg, 0) | 8 | AL |
| | 5 / 6 | `+0x14` / `+0x18` | AddNotification / RemoveNotification(target, id) | 8 | AL |
| `cISC4BudgetSimulator` (`0x00AB0908`) | 25 | `+0x64` | CreateDepartmentBudget(id, group) | 8 | EAX ptr |
| | 28 | `+0x70` | GetDepartmentBudget(id) | 4 | EAX ptr |
| | 51 | `+0xCC` | GetBudgetItemForPurpose(holder, purpose, item&) | 12 | AL |
| `cISC4DepartmentBudget` (`0x00AB0EB0`) | 3 | `+0xC` | GetDepartmentID | 0 | EAX |
| | 5 | `+0x14` | SetDepartmentName(key) | 8 | — |
| | 10 | `+0x28` | GetFundingPercentage | 0 | EAX ptr |
| | 13 | `+0x34` | SetMaxAllowedFundingPercentage(const SC4Percentage&) | 4 | — |
| | 17 | `+0x44` | CreateLineItem(id, bool) | 8 | EAX ptr |
| | 20 | `+0x50` | GetLineItem(id) | 4 | EAX ptr |
| | 22 / 23 | `+0x58` / `+0x5C` | Add / RemoveLocallyFundedObject(holder, purpose) | 8 | AL |
| | 24 | `+0x60` | SetLocalFundingPercent(holder, const SC4Percentage&, purpose) | 12 | AL |
| | 28 | `+0x70` | IsLocallyFundedObjectInDepartment(holder, purpose) | 8 | AL |
| `cISC4LineItem` (`0x00AB0F58`) | 18 | `+0x48` | SetDisplayFlag(flag, bool) | 8 | — |
| `cISC4DispatchManager` (`0x00A88108`) | 10 | `+0x28` | GetDispatchInfo(id, info&) | 8 | AL |
| | 11 | `+0x2C` | GetDispatchCount(type) | 4 | EAX |
| | 12 | `+0x30` | GetDispatchIDArray(type, u32* buf, u32& n) | 12 | AL |
| | 13 / 14 | `+0x34` / `+0x38` | Register / UnregisterDispatchSource(src) | 4 | AL |
| `cISC4AutomataControllerManager` (`0x00A96058`) | 10 | `+0x28` | AddControllersFromOccupantGroup(const cRZString& name, occ, 0); the name must have the game's string layout (§6.3) | 12 | AL |
| | 12 | `+0x30` | RemoveControllersFromOccupantGroup(const cRZString& name, occ); same layout rule | 8 | AL |
| `cISC4OccupantManager` (`0x00A8F7F0`) | 48 | `+0xC0` | ReleaseOccupantList(list&) | 4 | none meaningful |
| `cISC4Occupant` (`0x00AA41D8`) | 6 | `+0x18` | AsPropertyHolder | 0 | EAX ptr |
| | 7 | `+0x1C` | GetType | 0 | EAX |
| | 8 | `+0x20` | GetPosition(cS3DVector3&) | 4 | — |
| `cISCPropertyHolder` (`0x00AA6CD8`) | 3 | `+0xC` | HasProperty(id) | 4 | AL |
| `cISC4LotManager` (`0x00AB37C0`) | 14 | `+0x38` | GetOccupantLot(occ) | 4 | EAX ptr |
| `cISC4Lot` (`0x00AB19B0`) | 21 | `+0x54` | GetLocation(int& x, int& z) | 8 | — |
| | 28 | `+0x70` | HasRoadAccess | 0 | AL |
| `cISTETerrain` (`0x00AB3D18`) | 13 | `+0x34` | GetAltitude(x, z) | 8 | ST0 |
| | 29 | `+0x74` | GetSeaLevel | 0 | ST0 |
| `cISC4ZoneManager` (`0x00A92E00`) | 5 | `+0x14` | zone grid getter | 0 | EAX ptr |
| `cISC4HistoryWarehouse` (`0x00A8CCB4`) | 12 | `+0x30` | RecordHistoryData(id, value) | 8 | none |
| `cISC4DisasterLayer` (`0x00A9E390`) | 10 | `+0x28` | IsADisasterOngoing (police slot 11 of §15.4) | 0 | AL |
| `cISC4PoliceSimulator` (`0x00AB2490`) | 19 | `+0x4C` | GetPercentCoverage (fire) | 0 | EAX ptr |
| `cISC4App` (`0x00B43C94` global) | 69 | `+0x114` | RegisterShutdownCallbackFunction(cb, 0) | 8 | AL |
| `cISC4SimGrid<T>` | | | slots 6, 9, 12, 13, 16, 17, 19, 24, 25, 27 (simgrid §4) | | |
| `cSC4MapUtilities::AddRadiusEffect<int16>` | | `0x005E4500` | simgrid §7.1 | | |
| `cRZRandom` | | `0x0091372E`, `0x00913761` | §10 | | |
| Property helpers | | `0x005FD450` s32, `0x005FD480` u32, `0x005FD4F0` f32, `0x005FDAC0` pct, `0x005FF510` vector, `0x005FE3C0` set scalar u32, `0x005FE6B0` set u32 array (below) | | | |
| Response curves | | `0x005F4480` build from points, `0x005F4700` key loader, `0x005F4250` GetOutputValue | foundations §4 | | |

**The two property setters** *[compat]*. Both take `(holder, id, value)` and start the same way:

1. Look the property up with the holder's `GetProperty(id)` (slot 9).
2. If it is missing, create a new property with that ID and an empty value, and add it to the holder (slot 15,
   second argument 0).
3. Set the value through the property's variant. This **replaces the variant's type and count**.

They differ in the value they write:

| Setter | Value written (variant call) | Stored as | Written by |
|---|---|---|---|
| `0x005FE3C0` | **scalar** `Uint32` (`SetValUint32`, slot 26, `0x0091587D`: type code 6, the value inline) | one `u32`, not an array | strike start `0xEAA69F83` (fire `0x006ADD22`, police `0x006E1981`); availability mask `0x69B425DE` / `0x48D71ED8` and base call number `0x69B425DD` / `0x48D71ED7` (`0x006AD1A4`/`0x006AD184`, `0x006E0304`/`0x006E02E4`); jail population `0x48D71EE1` (`0x006E135C`); the air sources' properties |
| `0x005FE6B0` | **`Uint32` array** with the vector's elements copied, count = vector length (slot 102, `0x00915014`: type `0x8006`; a length-0 vector gives an empty array) | `u32` array | station statistics `0x8A70A0B9` (`0x006E26F0`, `0x006E2C62`), always 4 elements from this code (§9.2) |

So the strike start, mask, base call number and jail population are scalars, and only the statistics are a
vector. In a saved occupant they appear as a scalar `Uint32` property and as a 4-element `Uint32` array.

*[quirk]* An occupant's holder falls back to its exemplar in `GetProperty` (`0x005FEDA0`). If a plug-in exemplar
defines one of these IDs, the setter therefore **changes the exemplar's property**, shared by every occupant of
that exemplar, instead of adding one to the occupant. None of the IDs exists in any shipped exemplar (all shipped
packages scanned). *[choice]* FreeCity4 writes onto the occupant itself.

**`CellCenterToPosition`** (slot 97, body `0x00459440`) *[compat]* (PR6). It returns nothing, despite the
`x87` return in `docs/abi/interfaces.tsv`. Both results go out through the pointers, and each `FSTP` pops its
value, so the x87 stack is empty at `RET 0x10`. EAX is left holding `&px`, which no caller uses.

- **Arithmetic**, at 24-bit precision: `px = f32(f32(x + 0.5) × cellX)`, and the same for `pz` with `cellZ`.
  `0.5` is the constant at `0x00A84D2C`. `cellX` and `cellZ` are city `+0x108` and `+0x10C`, both set to `16.0` at
  `0x0045A13C`. For every in-bounds cell both steps are exact.
- **Worked example.** Cell (10, 20): px = 10.5 × 16 = **168.0**, pz = 20.5 × 16 = **328.0**.
- **Callers.** None of the 83 indirect calls through `+0x184` in the exe reads ST0 afterwards. The next x87
  instruction is always a load. The police call is at `0x006E1426` (the nearest station, §9.2).
- **Calling it.** A caller that declares a float return would pop an empty register. It would get the x87
  "indefinite" NaN and set the sticky invalid-operation flag. *[choice]* Declare it `void` and use only the two
  outputs.
- **Slot 96.** `CellCornerToPosition` (`0x00459410`) is the same function without the `+ 0.5`, and also returns
  nothing.

---

## 15. Compiled slot tables *[compat]* (see foundations §16, §17)

"hidden ptr" = a by-value `SC4Percentage` return through a hidden first stack argument (included in Pop; EAX =
that pointer on return). "AL" = only AL is meaningful; "EAX 0/1" = the full register is 0 or 1.

### 15.1 `cISC4FireProtectionSimulator`, vtable `0x00AB1160`, 26 slots

| Slot | Off | Body | Method (as the game expects it) | Pop | Return | SDK |
|---|---|---|---|---|---|---|
| 0 | `+0x00` | `006ACED0` | QueryInterface(iid, void**) | 8 | AL | ok |
| 1 | `+0x04` | `0082BE80` | AddRef | 0 | EAX | ok |
| 2 | `+0x08` | `006AD1B0` | Release | 0 | EAX | ok |
| 3 | `+0x0C` | `006AE240` | Init | 0 | AL = 1 | ok |
| 4 | `+0x10` | `006AD1E0` | Shutdown | 0 | AL = 1 | ok |
| 5 | `+0x14` | `006EFA50` | GetFireProtectionGrid → `cISC4SimGrid<uint8>*` | 0 | EAX | **error**: SDK `<int16_t>` |
| 6 | `+0x18` | `00407BA0` | GetDepartmentBudget → `cISC4DepartmentBudget*` | 0 | EAX | ok |
| 7 | `+0x1C` | `006ACF80` | IsFireStation(occ): HasProperty `0x08F55DC2` | 4 | AL | ok |
| 8 | `+0x20` | `006E08D0` | GetStationFunding(occ, int32*, SC4Percentage*) (§6.6) | 12 | AL | ok (pointers null-checked) |
| 9 | `+0x24` | `006AD530` | SetStationFunding(occ, const SC4Percentage&) | 8 | AL | ok |
| 10 | `+0x28` | `006AD430` | SC4Percentage GetStationEfficiency(occ) | 8 | hidden ptr | **error**: SDK `float ()` |
| 11 | `+0x2C` | `006E0120` | IsOnStrike | 0 | AL | ok |
| 12 | `+0x30` | `006AD950` | SC4Percentage GetChanceOfStrike() | 4 | hidden ptr | **error**: SDK `float` |
| 13 | `+0x34` | `006AD060` | BeginStrike(occ) | 4 | AL | ok |
| 14 | `+0x38` | `006AD100` | EndStrike | 0 | AL | ok |
| 15 | `+0x3C` | `006E0130` | GetStrikeStartDate | 0 | EAX | ok |
| 16 | `+0x40` | `006E0140` | GetStrikeLength (−1 when no date) | 0 | EAX | ok |
| 17 | `+0x44` | `006AE170` | int32 GetStrikingStations(list&): **appends** every station with `IsStationOnStrike` (slot 18), `AddRef` each, no clear; returns the **number appended** (PR6), not the list size | 4 | EAX count | **error**: SDK `bool` |
| 18 | `+0x48` | `006F6F30` | IsStationOnStrike(occ) | 4 | AL (EAX 0/1 when the property exists) | ok |
| 19 | `+0x4C` | `006E0200` | GetAverageCoverage | 0 | ST0 | ok |
| 20 | `+0x50` | `004778A0` | const SC4Percentage& GetPercentCoverage() | 0 | EAX ptr | **error**: SDK `float` |
| 21 | `+0x54` | `006AD5E0` | GetOccupantCoverage(occ, const SC4Percentage&, float&, float&) | 16 | AL | names wrong: outputs are cover radius and station radius |
| 22 | `+0x58` | `006E0180` | GetFireStations(list&): clears, copies (AddRef) | 4 | EAX count | ok (`int32`; SDK uses `ilist`) |
| 23 | `+0x5C` | `006ADA30` | GetFireStationCount | 0 | EAX | ok |
| 24 | `+0x60` | `006E0F60` | GetStationDispatchRadius(occ) | 4 | ST0 | ok |
| 25 | `+0x64` | `006AD8F0` | GetTotalDispatchCount | 0 | EAX | ok |

### 15.2 `cISC4PoliceSimulator`, vtable `0x00AB2490`, 45 slots

| Slot | Off | Body | Method (as the game expects it) | Pop | Return | SDK |
|---|---|---|---|---|---|---|
| 0–4 | | `006E0060`, `0082BE80`, `006AD1B0`, `006E2740`, `006E0310` | QI, AddRef, Release, Init, Shutdown | 8,0,0,0,0 | AL, EAX, EAX, AL, AL | ok |
| 5 | `+0x14` | `006EFA50` | GetPolicePowerGrid → `cISC4SimGrid<uint8>*` | 0 | EAX | **error**: SDK `<short>` |
| 6 | `+0x18` | `00407BA0` | GetDepartmentBudget → `cISC4DepartmentBudget*` | 0 | EAX | type: SDK `intptr_t` |
| 7 | `+0x1C` | `006E08D0` | GetStationFunding (shared with fire) | 12 | AL | ok |
| 8 | `+0x20` | `006AD530` | SetStationFunding (shared) | 8 | AL | ok |
| 9 | `+0x24` | `00441080` | GetJailDepartmentBudget → `cISC4DepartmentBudget*` | 0 | EAX | type: SDK `uint32_t` |
| 10 | `+0x28` | `006E0120` | IsOnStrike | 0 | AL | ok |
| 11 | `+0x2C` | `006E0F90` | SC4Percentage GetChanceOfStrike() | 4 | hidden ptr | **error**: SDK `float` |
| 12 | `+0x30` | `006E0780` | BeginStrike(occ) | 4 | AL | ok |
| 13 | `+0x34` | `006E0830` | EndStrike | 0 | AL | ok |
| 14 | `+0x38` | `006E0130` | GetStrikeStartDate | 0 | EAX | ok |
| 15 | `+0x3C` | `006E0140` | GetStrikeLength | 0 | EAX | ok |
| 16 | `+0x40` | `006E1A00` | int32 GetStrikingStations(list&): 0 (list untouched) if not `IsOnStrike` (slot 10); else appends as fire (slot 17 test) and returns the number appended | 4 | EAX count | **error**: SDK `bool` |
| 17 | `+0x44` | `006F6F30` | IsStationOnStrike(occ) | 4 | AL | ok |
| 18 | `+0x48` | `006E0200` | GetAverageCoverage | 0 | ST0 | ok |
| 19 | `+0x4C` | `004778A0` | const SC4Percentage& GetPercentCoverage() | 0 | EAX ptr | **error**: SDK `float` |
| 20 | `+0x50` | `006E0940` | GetOccupantCoverage(occ, const SC4Percentage&, float&, float&) | 16 | AL | names wrong (as fire) |
| 21 | `+0x54` | `006E0180` | int32 GetPoliceStations(list&) | 4 | EAX count | **error**: SDK `bool` |
| 22 | `+0x58` | `006E01C0` | IsPoliceStation(occ): HasProperty `0x48D71ED2` | 4 | AL | ok |
| 23 | `+0x5C` | `006E0F60` | GetStationDispatchRadius(occ) | 4 | ST0 | ok |
| 24 | `+0x60` | `006ADA30` | GetPoliceStationCount | 0 | EAX | ok |
| 25 | `+0x64` | `006E0E70` | SC4Percentage GetStationEfficiency(occ) | 8 | hidden ptr | **error**: SDK `float` |
| 26 | `+0x68` | `006E1390` | GetTotalDispatchCount | 0 | EAX | ok |
| 27 | `+0x6C` | `006E0A00` | int32 ResetUnjailedCriminals() (§9.3) | 0 | EAX | **error**: SDK `bool` |
| 28 | `+0x70` | `006E0AC0` | uint32 GetJailCapacity(occ) | 4 | EAX | **error**: overload order (SDK slot 35) |
| 29 | `+0x74` | `0043FD10` | uint32 GetJailCapacity() (total) | 0 | EAX | **error**: SDK slot 28 |
| 30 | `+0x78` | `00762700` | GetJailPopulation (Σ rounded per-jail populations) | 0 | EAX | SDK slot 29 (shifted) |
| 31 | `+0x7C` | `00423650` | GetCriminalCount (jailed) | 0 | EAX | SDK slot 30 (shifted) |
| 32 | `+0x80` | `006E01A0` | int32 GetJails(list&): clears, copies | 4 | EAX count | SDK slot 31 (shifted) |
| 33 | `+0x84` | `006E01E0` | IsJail(occ): HasProperty `0x48D71EE0` | 4 | AL | SDK slot 32 (shifted) |
| 34 | `+0x88` | `0074C6E0` | GetArrestCount (last month) | 0 | EAX | SDK slot 33 (shifted) |
| 35 | `+0x8C` | `006E14B0` | GetJailCount | 0 | EAX | SDK slot 34 (shifted) |
| 36 | `+0x90` | `006E0B30` | SC4Percentage GetJailedPercentage() | 4 | hidden ptr | **error**: SDK `uint32_t` |
| 37 | `+0x94` | `006E0A40` | int64 GetJailCurrentUpkeepCost(occ jail) | 4 | EDX:EAX | **error**: SDK has no argument |
| 38 | `+0x98` | `006E2B50` | ProcessMonthlyCrimeStats() | 0 | **none meaningful** | SDK `bool`; *[choice]* return true |
| 39 | `+0x9C` | `006E1070` | IsInJailbreak(occ jail) | 4 | AL | **error**: SDK has no argument |
| 40 | `+0xA0` | `006E1AF0` | bool BeginJailbreak(occ jail) | 4 | AL | **error**: SDK `float` |
| 41 | `+0xA4` | `006E1C90` | EndJailbreak(occ jail) | 4 | AL = 0 always | ok (type); value always false |
| 42 | `+0xA8` | `006E08A0` | int32 GetJailbreakBuildings(list&): 0 **without clearing** when empty, else clears and copies | 4 | EAX count | **error**: SDK `bool` |
| 43 | `+0xAC` | `006E14D0` | GetPoliceStationArrestCount(occ) | 4 | EAX | ok |
| 44 | `+0xB0` | `006E1520` | GetPoliceStationCrimeCount(occ) | 4 | EAX | ok |

The two `GetJailCapacity` overloads are compiled together at the first declaration's position, in reverse
declaration order (foundations §16), which shifts SDK slots 28–34 down by one. Count returns declared `bool` put
the count in EAX, so a caller reading AL sees false for 256 stations or a multiple of 256. *[compat]*

### 15.3 `cIGZMessageTarget2` + protection virtuals, vtables fire `0x00AB1118`, police `0x00AB2448`, 18 slots

| Slot | Off | Fire | Police | Method | Pop | Return |
|---|---|---|---|---|---|---|
| 0–2 | | thunks | thunks | QI, AddRef, Release | 8, 0, 0 | |
| 3 | `+0x0C` | `006AE070` | `006E2920` | DoMessage(cIGZMessage2*) | 4 | AL = 1 |
| 4 | `+0x10` | `006ADAF0` | `006E17B0` | scalar deleting destructor | 4 | EAX |
| 5 | `+0x14` | `006ACF70` | `006ACF70` | DoDaily(msg): factor := 1, then the daily step | 4 | none meaningful |
| 6 | `+0x18` | `006AE0C0` | `006E2A80` | DoMonthly() | 0 | none meaningful |
| 7 | `+0x1C` | `006F6610` | `006E1DB0` | DoOccupantInserted(occ) | 4 | none meaningful |
| 8 | `+0x20` | `006F64F0` | `006E1570` | DoOccupantRemoved(occ) | 4 | none meaningful |
| 9 | `+0x24` | `006F6250` | `006E0B80` | DoOccupantChanged(occ) | 4 | none meaningful |
| 10 | `+0x28` | `006AD190` | `006E02F0` | SetStationAvailabilityFlags(occ, u32) | 8 | none meaningful |
| 11 | `+0x2C` | `006AD6B0` | `006E0C50` | ProtectionGetStationFunding(occ, float*) | 8 | AL |
| 12 | `+0x30` | `006AD720` | `006E0CB0` | GetDispatchEffect(occ station, effect&) | 8 | AL |
| 13 | `+0x34` | `008CD450` | `004E13E0` | GetDispatchType → fire 1, police 0 | 0 | EAX |
| 14 | `+0x38` | `006AD7C0` | `006E0D50` | GetStationEffect(occ, effect&) | 8 | AL |
| 15 | `+0x3C` | `006AD170` | `006E02D0` | SetStationBaseCallNumber(occ, u32) | 8 | none meaningful |
| 16 | `+0x40` | `00657730` | `00657730` | ProcessCell(x, z, ?) — no-op | 12 | none |
| 17 | `+0x44` | `006AD320` | `006E0610` | EndOfCycle() | 0 | none meaningful |

### 15.4 Dispatch source (IID `0xC9C00435`, no SDK header), vtables fire `0x00AB10E8`, police `0x00AB2414`, 12 slots

| Slot | Off | Fire | Police | Method | Pop | Return |
|---|---|---|---|---|---|---|
| 0–2 | | thunks | thunks | QI, AddRef, Release | 8, 0, 0 | |
| 3 | `+0x0C` | `006ACFA0` | `006E0210` | CanHandleDispatchType(t): fire t == 1, police t == 0 | 4 | EAX 0/1 |
| 4 | `+0x10` | `006F5A80` | same | CanDispatch(float x, float z, bool* allBusy) | 12 | AL |
| 5 | `+0x14` | `006F5B00` | same | GetMaxPossibleDispatchCount (§7.3) | 0 | EAX |
| 6 | `+0x18` | `006F6A50` | same | GetStations(list&): clears, copies | 4 | EAX 0/1 (count ≠ 0) |
| 7 | `+0x1C` | `006ACFB0` | `006E0220` | GetDispatchesAtStation(occ, u32* flags, u32* base, u32* count) | 16 | AL |
| 8 | `+0x20` | `006F5B40` | same | DispatchFromNearestStation(float x, float z, u32* id) | 12 | EAX ptr |
| 9 | `+0x24` | `006F5570` | same | DispatchFromStation(occ, u32* id) | 8 | AL |
| 10 | `+0x28` | `006F56E0` | same | RevokeDispatchFromStation(occ, u32 id) (thunk to `0x006F5620`) | 8 | AL |
| 11 | `+0x2C` | `006AD5B0` | `006E0C00` | GetDispatchAutomataProperties(…) (§8.2) | 20 | AL |

**How the list getters fill the caller's list** *[compat]* (PR8). Fire slot 22, police slots 21 and 32, police
slot 42 (when the jailbreak list is non-empty) and dispatch-source slot 6 all use one helper, `0x006DFC70`.

1. **Clear.** It empties the caller's list with the plain list clear `0x00413E60`. That returns each node to the
   list allocator (`0x00B207E4`, slot 4) and **does not `Release` the element**.
2. **Copy.** It appends the simulator's own list, calling `AddRef` (slot 1) on each element.
3. **Return** the number appended.

Effect on callers:

- **Original callers never lose a reference.** Every one builds a fresh empty list just before the call and
  hands it to `ReleaseOccupantList` (occupant manager slot 48) afterwards. The calls through the fire and police
  globals are at `0x00416189`, `0x004164B1`, `0x007A1A8E` and `0x007A1AF0`. The dispatch manager's calls to
  slot 6 are at `0x0046AFA2` and `0x0046B116`, released at `0x0046B03B` and `0x0046B47C`.
- **Only a non-empty list leaks.** If a caller passes one, the references its elements held are dropped without a
  `Release`.
- *[choice]* **Reproduce it.** No original caller can tell the difference. Releasing would over-release a list
  that holds borrowed pointers.

### 15.5 `cIGZSerializable`, vtables fire `0x00AB10D0`, police `0x00AB23FC`, 6 slots

Slots 0–2 thunks; 3 `Write(cIGZOStream&)` (fire `006AD250`, police `006E03F0`) pop 4, AL; 4
`Read(cIGZIStream&)` (fire `006AD2A0`, police `006E04E0`) pop 4, AL; 5 `GetGZCLSID()` (fire `006ACEC0`, police
`006E0050`) pop 0, EAX.

---

## 16. Original quirks (reproduced) and corrections to the earlier write-up

**Quirks** *[compat]*:
1. Funding changes and strike changes rebuild only the unadjusted grid; the adjusted grid (what everyone reads)
   catches up tract by tract over the next cycle (about 41–51 days).
2. `factor` is always 1.0; the "adjusted" grid is a clamped copy.
3. The station effect's result is ignored in §5.1; a failed effect can still draw a cone from values read before
   the failure.
4. Prop stations use the department funding only (§7.1).
5. City-level strikes end only in a month with a station change (§6.5).
6. `DispatchFromStation` succeeds with an out-of-range ID when no unit is free and clears a requested unit's
   bit without checking it (§8.2).
7. `GetStrikingStations` appends without clearing; `GetJailbreakBuildings` leaves the list as is when empty.
   The clearing getters drop the caller's old elements without `Release` (§15.4).
8. Demolished jails stay in the jailbreak list (§9.4).
9. Fire and police `Read` return true after the base record was rejected.
10. `GetJailedPercentage` returns NaN when both jailed and capacity are 0.
11. The escapee range code default (1.0, 5.0) is not scaled by 0.01; the code-default strike and jailbreak curves
    rise with funding / are inverted.
12. A new city's first daily call ends a cycle at once with zero counts (history 0) *[inferred]*.
13. `EndJailbreak` always returns false; `ProcessMonthlyCrimeStats` returns garbage.
14. A jail in a jailbreak never leaves the list while the city's total jail capacity is 0 (NaN or 1.0 jailed
    percentage, §9.6); a save and reload clears it.
15. All percentages are float32 products with 0.01f: shipped 120 → 1.1999999, 80 → 0.79999995, 20 → 0.19999999,
    10 → 0.099999994 (§6.7); jailed ≤ 5 is never rehabilitated (§9.5).
16. `GetMaxPossibleDispatchCount` re-adds the previous station's count for a station without the count property;
    the call-number search re-marks the previous range the same way (§7.3, §8.2).
17. The police station effect fails only on the centre; a missing outer or radius keeps the caller's stale value
    (§7.1). The shared effect record in §5.1 is never reset between stations.
18. Station statistics vectors are not length-checked once read (§9.2).
19. Jail efficiency treats "no jail budget item" as full funding (1.0), stations as 0 (§6.1).

**Corrections to `grid-sims.md` and other specs**:
- The coverage grid every consumer reads is `uint8`; the `int16` grid is internal.
- Police slots 28–35: see §15.2. Slot 30 (`+0x78`) is `GetJailPopulation`; the jailed count is slot 31 (`+0x7C`).
- `ResetUnjailedCriminals` releases this month's **unarrested** criminals plus the overflow above
  trunc(capacity × 1.1); it is not a list of "pending releases".
- *(applied)* foundations §4.1 now lists fire and police under the **default** pattern, and foundations §6.2 lists
  the police `cRZRandom`.
- The earlier version of this spec (corrected here): §6.1 tested "locally funded" on the item before the department lookup; the
  department lookup comes first and "locally funded" is asked of the department. The fire station effect also fails
  when the centre is absent; the police one does **not** fail when the radius is absent (§7.1). The statistics
  vector is not truncated to 4 when present (§9.2). Shipped percentages were given as decimal fractions (§6.7).

---

## 17. Open questions

1. The meaning of the dispatch-automata values `0x4202`, `0x4207`, `0x4228`, `0x11882` (dispatch manager spec).
2. Whether `factor` was ever meant to change (a funding or disaster multiplier); no code sets it to anything
   but 1.0.
3. The exact order of the releases in base `Shutdown` (no observable effect known).
4. Whether the automata "striking_*" and "jailbreak" controllers survive a save; the jailbreak list does not, so
   after a load a jail can be breaking out visually without being in the list *[inferred]*.
5. The police `0x48D71EE4` (overcapacity cost) read in `DoMonthly` is discarded; the jail cost may be booked
   elsewhere (budget spec).

---

## 18. Conformance tests (must exist in `engine/conformance`)

Curves are evaluated with foundations §4 (tolerance 1e-6 relative). Values given with a bit pattern are exact
float32 results under §6.2 (constants 100.0f and 0.01f) and must match bit for bit.

1. **Fire efficiency curve** (shipped `0x29233E8D`): funding 0.5 → e = 0.25; 0 → 0.099999994 (`0x3DCCCCCC`, not
   0.1f); 1.1f → 1.05f (`0x3F866666`); 1.2f → 1.1f (`0x3F8CCCCD`); the shipped maximum 1.1999999 → 1.1f.
2. **Fire efficiency modifiers**: funding 1.0, no road access, on strike → 1.0 × 0.8f × 0.2f = 0.16000001
   (`0x3E23D70B`). Same with a road → 0.2f (`0x3E4CCCCD`; the fire multiplier is a raw f32). Police at funding 1.0
   on strike, no road → 0.19999999 (`0x3E4CCCCC`; the police multiplier is the percentage 20 × 0.01f).
3. **Jail efficiency curve** (shipped `0x68DDAEA0`): funding 0.55 → 0.75; 0.05 → 0.25.
4. **Strike curve** (shipped): f = 0.2 → curve 40 (police chance 0.39999998, `0x3ECCCCCC`); f = 0.5 → 10 (police
   0.099999994); f ≥ 0.7 → 0.
5. **Chance of strike**: two stations with local funding 0.5 and 0.2 (in that list order), no city strike →
   c = 0.099999994, 0.39999998; p := (p + c) − p·c, each step float32 → **0.45999998** (`0x3EEB851E`). City strike
   flag set → 1.0. No stations → 0.
6. **Strike end**: station started day 1000, now day 1549, maxDuration 548 → ends (549 > 548). Now 1548 and
   funding 0.89 → stays; funding 0.9 → ends.
7. **Monthly strike pass**: two stations, A already striking, B starts this month (A first in the list) →
   BeginStrike(B), one `0xA8DAF511`/`0x68F54CE1` message with B; the unadjusted grid is rebuilt; the adjusted
   grid is unchanged until the iterator reaches each tract.
8. **Jailbreak curve** (shipped `0xEA614F63`): jailed percentage 0.95f → chance 0.55f (`0x3F0CCCCD`); 0.85f →
   0.049999997 (`0x3D4CCCCC`); 0.8f → 0.
9. **Rehabilitation** (shipped rate 10 → 0.099999994): jailed 600 → 540; 125 → 113 (12.499999 → 12); 135 → 122
   (13.499999 → 13, not 14); 6 → 5; 5 → 5 (0.49999997 → 0: stays).
10. **ResetUnjailedCriminals**: capacity 1000, jailed 1200, unarrested 7 → returns 107, jailed 1100,
    unarrested 0. Capacity 1000, jailed 900, unarrested 7 → returns 7, jailed 900.
11. **Recalculate**: two jails with capacity properties 400 and 600 at full funding (efficiency 1.0), jailed 600 →
    capacity 1000, per-jail populations 240 and 360, population 600. Jailed 1500 → 400 and 600, population 1000,
    `GetJailedPercentage` = 1.0.
12. **Escapees**: jail capacity 600, r = 0.25 → 150 escape; jailed 400 → 250.
13. **Crime event**: adjusted value 21 with threshold 20 → arrest (arrestsThisMonth +1, station vector [0,0,1,1]);
    value 20 → unarrested (+1, vector [0,0,1,0]). After `ProcessMonthlyCrimeStats` with 3 arrests and jailed 10:
    jailed 13; history `0x0A5CC156` = 3; vector shifted to [1,1,0,0] / [1,0,0,0].
14. **Occupant coverage**: centre 100, outer 20, radius 400 m, efficiency 1: p = 0.5 → (250, 400); p = 0.1 →
    (400, 400); p = 1.1 → (0, 400). Efficiency 0.5 (centre 50, outer 10, radius 200), p = 0.5 → (0, 200).
15. **Effect scaling**: centre 100, outer 21, radius 300, e = 0.75 → centre 75, outer 15, radius 225.
16. **Dispatch**: station with count 3, base 5, mask 0b111: `DispatchFromStation(s, id = 0)` → id 5, mask 0b110;
    again → 6, 0b100; `RevokeDispatchFromStation(s, 5)` → true, mask 0b101; revoke 5 again → false. Mask 0
    and id 0 → succeeds with id 8, mask 0.
17. **Call numbers**: insert A (2 units) → base 1; B (3) → base 3; nextCall 6. Remove A, insert C (2) → base 1.
18. **Cadence**: 256-cell city: fire 327 tracts per day, police 409; 64-cell city: 20 and 25.
19. **Cycle averages**: a cycle visiting 4 zoned tracts with adjusted values 0, 10, 20, 50 and any number of
    unzoned tracts → avg 20.0, pct 0.75; fire history `0x2A11D203` = 75, `0x2A11D204` = 20; with police pct 0.5,
    `0xAAAD4D87` = 62. Three zoned tracts 10, 10, 0 → avg = f32(20 / 3) = 6.6666665 (`0x40D55555`), pct = f32(2 / 3)
    = 0.6666667 (`0x3F2AAAAB`); history 66 and 6. No zoned tract → avg 0, pct 0.
20. **Adjusted copy**: unadjusted −5 → 0; 300 → 255; 128 → 128.
21. **Persistence round trip**: base record v3 of a city with two stations → identical bytes on re-save; a v2
    record with pct 75.0 loads as 0.75; version 4 is rejected after 2 bytes; the police record v7 loads without
    the helicopter ref; base versions 0, 1 and 2 load with the v3 layout (pct scaled); a fire v2 record ends
    after the base record; police v9 is rejected after 2 bytes; a police v8 Read on an object that already holds
    a helicopter source releases it before reading the ref.
22. **Slot ABI**: calling fire slot 10 with a hidden pointer and one occupant pops 8 and returns the pointer in
    EAX; police slot 37 returns the cost in EDX:EAX; police slot 41 returns AL = 0 after ending a jailbreak.
23. **Stuck jailbreak** (PR15): rehabilitation rate fixed to 0 (with the shipped rate 0.099999994 the first month
    end below rehabilitates 90 and the return becomes 810 + unarrested); one jail J (capacity property 1000, full
    funding), jailed 900, J in the jailbreak list. Demolish J → capacity 0, jailed 900 → `GetJailedPercentage` = 1.0 → month end: J stays, no
    `0x2A614892`. `ResetUnjailedCriminals` → returns 900 + unarrested, jailed 0 → percentage NaN (`0xFFC00000`) →
    month end: J stays. Save and reload → the list is empty. Without reloading: insert jail K (capacity 1000, full
    funding), jailed 0 → percentage 0 → at the next month end `EndJailbreak(J)`: one `0x2A614892` naming J, list
    empty, `IsInJailbreak(J)` false.
24. **Jailed percentage and the end test**: (jailed, capacity) = (0, 0) → `0xFFC00000`; (5, 0) → 1.0;
    (1200, 1000) → 1.0; (800, 1000) → 0.8f (`0x3F4CCCCD`), not below 0.79999995 → no end; (799, 1000) → 0.799
    (`0x3F4C8B44`) → ends. A NaN jailbreak chance makes no random draw; it arises whenever the city-wide percentage is NaN, because
    the jailbreak curve returns NaN for a NaN input (foundations §4.3).
25. **Percentages** (getter `0x005FDAC0`): 10 → `0x3DCCCCCC`, 20 → `0x3E4CCCCC`, 60 → `0x3F199999`, 80 →
    `0x3F4CCCCC`, 90 → `0x3F666666`, 110 → `0x3F8CCCCD`, 120 → `0x3F999999`. Escapee range (10, 50) →
    (`0x3DCCCCCC`, 0.5).
26. **GetMaxPossibleDispatchCount** (PR1): stations in list order A (count 3), B (no count property), C (2) → 3 + 3
    + 2 = 8, while `GetTotalDispatchCount` = 5. Order B, A, C → 0 + 3 + 2 = 5. No stations → 0.
27. **Station effect partial reads** (PR2): police, centre property 0 → AL = 0 and the record's outer, radius,
    position keep the caller's values; police with centre 100, outer 20 and no radius property, record radius
    previously 250, e = 1 → AL = 1, radius 250. Fire with no centre property, outer 20, radius 300 → AL = 0, record
    centre 0, outer 20, radius 300 (unscaled), position written.
28. **Call-number search** (PR10): stations A (base 1, 2 units) and C (base 5, 1 unit), nextCall 6. Insert D (2) →
    base 3, nextCall 6. Then insert E (3) → candidates 1, 2, 3, 4, 5 are each blocked, 6 ≥ nextCall → base 6,
    nextCall 9. No other station and a loaded nextCall of 0 → base 1, nextCall := n + 1.
29. **Station statistics** (PR12): property absent, crime → [0,0,1,0]. Present [5,6,7,8,9], arrest →
    [5,6,8,9,9]; monthly shift → [8,9,0,0,9]. *[choice]* Present [1,2], crime → [1,2,1,0] (padded).
30. **Jail capacity** (PR14, §9.1): jail with capacity property 500 and **no** jail budget item → efficiency 1.0,
    capacity 500. Capacity property absent → capacity 0; Recalculate with only that jail → ratio 0, population 0.
31. **Locally-funded-jail list** (PR8): removing a building that is in the locally-funded-jail list releases and
    erases that entry even when it is no longer in the jail list; removing a building not in it changes nothing.
