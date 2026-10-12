# Advisor system (`cSC4AdvisorSystem`, `cSC4Advisor`, the seven advisor classes) — behaviour spec

Status: round 3, written from the Windows 1.1.641 executable. The Mac named pseudocode was used for names and
intent only. Every fact below was checked on Windows unless it is tagged *[inferred]*. The game's own Lua scripts
(`SimCity_1.dat`, group `0x4A5E8EF6`) are data. This spec names their fields and functions but does not copy them.

Tags: *[compat]* observable, must match; *[choice]* FreeCity4 decision; *[inferred]* not proven;
*[quirk]* original behaviour that looks unintended but is reproduced (or that a replacement must survive).

Shared material cited and not repeated here: streams, refs and strings (foundations §2), class store (§3), city
component table (§3.6), Init order (§3.7), response curves (§4), messages (§5), random generators (§6), month order
(§13), slot-order rule (§16), framework slot tables (§17), x87 precision (§18); scheduler and pause semantics
(simulator spec §5–§6); grid ownership (simgrid §6.4). Subsystem getters are named as in their own specs (aura,
budget, crime, demand, neighbors, ordinance, plumbing, pollution, power, protection, residential, landvalue).

Scope: the simulation side. The advisor windows, the popup tracker, the news ticker and the head animations are UI.
They are named where the simulation calls them and are otherwise left out. MySim (`cSC4MySim`, the MySim advisors)
is out of scope; §14.3 lists every point where this system touches it. The generic Lua host (`cSCLua`, script
packages, the resource loader) is described only as far as the advisor system uses it (§4.2); it belongs in its own
spec.

---

## 1. Role and identity

The advisor system runs the **advice engine**. Advisors are objects created from the Lua `advisors` table. Each owns
the advices of one `advice_type` from the Lua `advices` table. Once per simulated day, each advisor:

1. publishes city statistics into the Lua table `game` (the "variables", §7), and
2. evaluates the Lua **trigger** of a slice of its advices. An advice whose trigger is true becomes **active**: it is
   announced to the UI by message, can raise a "global effect" in the 3D view and can carry a **subject** (a map
   location or an object). It stays active until it times out, its trigger turns false, or a script retires it.

Advices can also react to **events** (`PushEvent(eventID, subject)`) pushed by advisors, other subsystems and
scripts. Reward advices decide whether reward buildings are unlocked (§6.6). The system hosts the Lua state the
advisors, the tutorial and MySim share.

### 1.1 Classes

| Class | CLSID (`GetGZCLSID`) | Interfaces (IID) | Vtables (Windows) | Size |
|---|---|---|---|---|
| `cSC4AdvisorSystem` | `0x4A0DD616` | `cISC4AdvisorSystem` `0x4A0DD617` (primary), `cIGZMessageTarget2` `0x452294AA` (+4), `cIGZMessageTarget` `0xE98B2F57` (+8), `cIGZSerializable` `0xE4FDA3D4` (+0x10) | `0x00A82AC8` (29 slots), `0x00A82A70`, `0x00A82A88` | — |
| `cSC4Advisor` | `0xEAC937B4` | `cISC4Advisor` `0xCA09F544` (+8), `cIGZSerializable` (+0xC), `cIGZMessageTarget2` (+0x10) | class `0x00A81204` (9), iface `0x00A811B0` (21), ser. `0x00A81194`, msg `0x00A81184` | `0xB8` |
| `cSC4AdvisorBase` | `0xCA09F543` (factory only, §15 Q1) | as `cSC4Advisor` | class `0x00A812CC` (10), iface `0x00A81278`, ser. `0x00A81260`, msg `0x00A81250` | `0xC0` |
| `cSC4AdvisorCityPlanning` | `0x2A5F877D` | as above; QI also answers `0x2A5F877D` | class `0x00A81514`, iface `0x00A814C0`, ser. `0x00A814A8`, msg `0x00A81498` | `0xDC` |
| `cSC4AdvisorCitySituations` | `0x4BE372CD` | as above | class `0x00A81C9C` (11), iface `0x00A81C48`, ser. `0x00A81C30`, msg `0x00A81C20` | `0xC0` |
| `cSC4AdvisorFinance` | `0xAA5292D7` | as above | class `0x00A81D4C`, iface `0x00A81CF8`, ser. `0x00A81CE0`, msg `0x00A81CD0` | `0xC4` |
| `cSC4AdvisorHQ` (health and education) | `0x6A5F8755` | as above | class `0x00A81FDC`, iface `0x00A81F88`, ser. `0x00A81F70`, msg `0x00A81F60` | `0xC4` |
| `cSC4AdvisorSafety` | `0xEA19E825` | as above | class `0x00A8237C`, iface `0x00A82328`, ser. `0x00A82310`, msg `0x00A82300` | `0xC4` |
| `cSC4AdvisorTraffic` (transportation) | `0x4A3AD3E1` | as above | class `0x00A82FEC` (10), iface `0x00A82F98`, ser. `0x00A82F7C`, msg `0x00A82F6C` | — |
| `cSC4AdvisorUtility` | `0xCA2C2A2F` | as above | class `0x00A83B8C` (10), iface `0x00A83B38`, ser. `0x00A83B20`, msg `0x00A83B10` | `0xC4` |
| `cSC4Advice` | `0x8A09F5F4` | `cISC4Advice` `0x8A09F5F5` (primary), `cIGZSerializable` (+0xC) | `0x00A808B8` (31), ser. `0x00A80884` | — |
| `cSC4RewardAdvice` | `0xAA371C32` | as `cSC4Advice` | — | — |
| `cSC4AdviceNeghborDeal` | `0x6A3848DB` | as `cSC4Advice`, plus `0xEA3848BA` | `0x00A83BE8` (33) | — |
| `cSC4AdviceSubject` | `0x6A26D049` | `cISC4AdviceSubject` `0x6A26D050` (at +0xC), `cIGZSerializable` | `0x00A81090` (14), ser. `0x00A810C8` | `0x28` |

All interfaces of one object share one reference count. Every class above is registered with the game's COM
director (`cSC4COMDirector` constructor, `AddCls(clsid, factory)`), so `GZCOM()->CreateInstance(clsid, iid)` builds
it. The other advice classes (`cSC4CitySituationAdvice` `0x8BE3753B`, `cSC4MySimAdvice` `0x6A9335DE`,
`cSC4MySimBalloonAdvice` `0x6B70EFAD`) and the MySim advisors (`0x4A1DBBBF`, My Street sim `0x0BF606A5`) are
created through the same tables but belong to the city-situation and MySim specs.

### 1.2 Creation and Init order

- The **system** is city component **#47** (foundations §3.6). The city creates it with
  `GetClassObject(0x4A0DD616, 0x4A0DD617)` and reaches it with `cISC4City` `+0x144` `GetAdvisorSystem`, cached as
  `SL::spAdvisorSystem`. Its constructor also stores itself in a **private global** that every advisor, advice and
  Lua extension uses instead of the city getter (§14.2) *[compat]*.
- It is initialised after the vehicle managers and before the city situation manager, the tutorial system and the
  MySim agent simulator (foundations §3.7). So every simulator it reads is already initialised.
- **Advisors** are created by the system from the Lua `advisors` table (§4.3), or by the class store when a saved
  city is loaded (§11).
- **Advices** are created by their advisor from the Lua `advices` table (§4.5), or by the class store on load.
- **Subjects** are created by anyone through `GZCOM()->CreateInstance(0x6A26D049, 0x6A26D050)`.

---

## 2. Data model

### 2.1 The `advisors` table (script data)

The system reads the global Lua table `advisors` (an array, index 1..n). Fields read by the engine:

| Field | Read by | Meaning |
|---|---|---|
| `class_id` | system | C++ class to create (§1.1). Entries whose class is `cSC4AdvisorBase` use the base class |
| `id` | system, advisor | advisor ID, the key of the system's advisor map. **0 = ignored** (template entries) |
| `caption` | advisor | display name. Only the text before the first `|` is kept |
| `name` | advisor | the advisor's personal name |
| `advice_type` | advisor | which `advices` entries belong to this advisor |

Shipped advisors (adv_advisors.lua / adv_const.lua):

| id | Advisor | Class | advice_type |
|---|---|---|---|
| 1–5 | MySim 1–5 | `0x4A1DBBBF` (MySim, out of scope) | 1 |
| 21 | Utilities | Utility | 21 |
| 22 | Health and education | HQ | 22 |
| 23 | City planning | CityPlanning | 23 |
| 24 | Transportation | Traffic | 24 |
| 25 | Environment | `cSC4AdvisorBase` | 25 |
| 26 | Finances | Finance | 26 |
| 27 | Safety | Safety | 27 |
| 28 | Fluff news | `cSC4AdvisorBase` | 28 |
| 29 | City situations | CitySituations | 29 |
| 30 | My Street sim | `0x0BF606A5` (MySim, out of scope) | 1 |

### 2.2 The `advices` table (script data)

An advisor owns every `advices` entry whose `type` equals its `advice_type`, whose `guid` is non-zero and whose
`class_id` is non-zero. An advice keeps the **table index** of its entry. Fields read by `cSC4Advice::Init`:

| Field | Meaning | Stored as |
|---|---|---|
| `guid` | the advice's unique ID (an exemplar-style instance) | GUID; must equal the saved GUID (§4.5) |
| `class_id` | C++ advice class. A mismatch with the object's own class sets the **class mismatch** flag | — |
| `priority` | 0..100 (popup filter, §6.8) | int |
| `mood` | 0 alarm, 1 bad job, 3 neutral, 5 great job, 7 fluff | int |
| `frequency` | minimum days between two activations | int (days) |
| `timeout` | days an active advice stays active | int (days) |
| `trigger` | Lua **expression** (text). Missing or empty means `"0"` | compiled to a function (§4.5) |
| `once` | non-zero: the advice can activate once per city | flag `once` |
| `news_only` | non-zero: news ticker only | flag `news_only` |
| `event` | non-zero: an event ID. The advice is evaluated only when that event is pushed | eventID, flag `event` |
| `command` | non-zero: a UI command ID sent when the advice activates | commandID |
| `persist` | non-zero: stays active when its trigger turns false | flag `persist` |
| `no_timeout` | non-zero: never times out | flag `no_timeout` |
| `effects` | list of global-effect IDs asserted in the 3D view while active | effect list |
| `title`, `message`, `news` | text keys or text, detokenized for display | UI only |

Subclasses read extra fields: `condition` (a function, reward advices, §6.6) and `nd_type` (neighbour-deal advices,
§7.8).

### 2.3 Advice flags

| Flag | Name | Set by | Meaning |
|---|---|---|---|
| `0x1` | news_only | Init | |
| `0x2` | **active** | activation / deactivation (§6.4) | |
| `0x4` | once | Init | deactivation sets `nextDay := 0x7FFFFFFF` |
| `0x8` | expire request | `game.expire_advice()`, base DoMessage (§5.4) | the advisor deactivates it on its next day |
| `0x10` | text dirty | many | the text is rebuilt on the next day pass |
| `0x40` | event | Init | skipped by the daily evaluation; evaluated on events |
| `0x80` | no text | — | forces `GetTrigger` false (§6.1) *[inferred: set by subclasses]* |
| `0x100` | persist | Init | |
| `0x200` | debug show GUID | debug message `0xEA917750` | setting or clearing it also sets `0x10` |
| `0x400` | forced trigger | debug message `0x4A91DD6E` | `GetTrigger` returns true |
| `0x800` | class mismatch | Init | the advisor re-creates the advice (§4.4) |
| `0x1000` | reward | `cSC4RewardAdvice::Init` | |
| `0x2000`, `0x4000` | popup tracker | UI | `0x4000` = "queued for a popup". Persisted with the flags |
| `0x8000` | no_timeout | Init | |

### 2.4 Advice info (per advisor, per advice)

Each advisor keeps a vector of **advice info** records, one per advice it owns, in table-entry creation order:

| Field | Meaning |
|---|---|
| advice | the advice (owning reference) |
| nextDay | first day on which the daily evaluation may test it again |
| expiry | day on which an active advice times out |
| lastMonth | value of the advisor's month counter when it was last evaluated |

plus an **active map** (advice → index in the vector), an **event multimap** (event ID → index) and an **event queue**
of (event ID, subject) pairs.

### 2.5 Subjects

A subject is a small refcounted value: an optional **location** (x, z as float32, in **city cell** coordinates), an
optional **object** (an `IGZUnknown`, AddRef'd), and two `uint32` data words. `SetLocation(x, z)` stores the pair
and then clamps it to `[0, cellCountX] × [0, cellCountZ]` (`cISC4City` `+0x174`/`+0x178`) *[compat]*. Subjects
reach Lua as userdata of the advice-subject tag (§8.1).

---

## 3. State

### 3.1 System

| Item | Initial | Persisted | Notes |
|---|---|---|---|
| advisor map (advisor ID → advisor, ascending ID) | empty | yes (§11.1) | |
| event subscriptions (event ID → set of advisors, ordered by pointer) | empty | no | rebuilt by advisor Init |
| context-advice stack | empty | no | §6.7 |
| popup thresholds `[3]` (one per difficulty) | 100, 100, 100 | no | §6.8 |
| own `cRZRandom` | seeded with `0xFFFFFFFF` (wall clock) in the constructor | no | §9 |
| Lua context (`cISCLua`) | the app's Lua (`SL` global) | no | set in Init |
| two Lua userdata tags | — | no | §8.1 |
| lastDay | 0 | no | last simulation day number handled by the ticker |
| idle timer (`cRZTimer`) | — | no | 500 ms paused-update timer |
| loadedFromSave | false | no | true after a successful `Read`; cleared on PostCityInit |
| initialised | false | no | |
| reloadPending | false | no | never set (§16 Q2) |
| advisor window | null | no | UI |

### 3.2 Advisor (`cSC4Advisor`)

| Item | Initial | Persisted |
|---|---|---|
| table index (entry in `advisors`) | — | yes |
| caption, personal name | from script | yes (and re-read on Init) |
| month counter | 0 | yes |
| current day (simulation day number) | 0 | yes |
| advisor ID | — | yes |
| evaluation cursor (index in the advice vector) | 0 | yes |
| advice info vector, active map | empty | yes |
| event multimap | empty | rebuilt on Read and Init |
| event queue | empty | yes |
| head animation lists, current head animation | — | no (UI) |

### 3.3 Advice (`cSC4Advice`)

| Item | Initial | Persisted |
|---|---|---|
| table index, GUID | from the entry | yes |
| flags | 0 | yes |
| advisor ID | — | yes |
| priority, frequency, timeout, mood | from script | yes (and re-read on Init) |
| global-effect list | from script | yes |
| event subject | null | yes (ref) |
| event ID, command ID | from script | no (re-read on Init) |
| compiled trigger (Lua reference) | none (`−2`) | no |
| trigger text, title, message | from script | no |
| session ID | — | no |
| loaded | false | no (true between Read and Init) |

### 3.4 Subclass state

| Advisor | Item | Persisted |
|---|---|---|
| all subclasses (and the base) | **dirty** byte (variables need an update) | no. Set to 1 by the constructor and by Init |
| all subclasses (and the base) | pending-update flags (bits 1, 2, 4) | no |
| CityPlanning | building count, park count, recreation count, landmark map (exemplar instance → count) | **yes** (§11.5) |
| Traffic | set of lots with a car "zot" | no |
| Safety | fire-active, riot-active, user-disaster bytes | no |
| HQ | ten static building-group vectors (filled once) | no |

---

## 4. Lifecycle

### 4.1 System `Init()` → true *[compat]*

If already initialised, only the last step runs. Otherwise:

1. Take the app's Lua context. `Open(0x7E)` (`cISCLua` slot 3).
2. `register_api(global = true)` (§8.2).
3. Create two Lua tags with `NewTag`. For each, push the C function **tag-gc** and `SetTagMethod(tag, "gc")`. Tag 0 is
   the advice-subject tag, tag 1 the string tag (§8.1). `GetDataTag(i)` returns them.
4. Register `gAdviceDetokenizerFunction` with the string detokenizer service (`SL` global, slot `+0x1C(fn, this,
   1)`). The detokenizer expands `%token%` text in advice titles and messages (UI, §8.5).
5. `reload_data()` (§4.2).
6. If the advisor map is not empty and the system was **not** loaded from a save: `kill_advisors()` (§4.3).
7. Register the cheat codes `DollyLlama` (`0xAAB7F42A`) and, only when the app's debug functionality is enabled
   (`cISC4App` `+0x54`), `Adv` (`0xA919D25E`).
8. If not loaded from a save: `load_advisors()`. Then mark initialised, `update_advisors()` and the popup thresholds
   (§6.8).
9. Subscribe `PostCityInit` `0x26D31EC1`, `CityEstablished` `0x26D31EC4` and the **ticker** `0xCA5501F2`.
10. Release any advisor window and call slot 24 (`startAdvisorWindow`, UI).

### 4.2 Script loading and reload (`reload_data`) *[compat]*

1. Clear reloadPending. If the Lua global `_on_reload` is a function, call it (protected, no arguments).
2. Override existing advisor scripts from loose files: for `<data directory>\advisors\` and then
   `<plugin directory>\advisors\` (so **plugins win**), every `*.lua` file whose resource key `{0xCA63E2A3,
   0x4A5E8EF6, 0xFF000000 | CRC24(basename)}` already exists replaces that script's data. New names are ignored.
3. Load the script **packages** of group `0x4A5E8EF6` in ascending package ID order (package headers `--#-package:<hex>`
   in the first bytes of the script). Shipped packages: `0x00000001` (system), `0x00002000`, `0x00003000` (advisors),
   `0x00004000`, `0x01001000` (expansion).
4. `register_api(global = false)` (registers the `game.*` functions). If it fails, return false.
5. Hand the Lua context to the tutorial registry (`cISC4App` `+0xC4` → `SetLua`). Return true.

The resource key hash (CRC24, OpenPGP polynomial `0x864CFB`, initial `0xB704CE`), the package loader and
`_dofile_from_resource` are generic Lua host behaviour and are specified in the Lua host spec (§16 Q8).

`reload_data` runs from `Init` and from the debug cheat `Adv reload` (§5.2). The ticker also calls it when
reloadPending is set, which never happens (§16 Q2).

### 4.3 Advisor creation (`load_advisors`, `update_advisors`, `load_advisor`, `kill_advisors`)

- **`load_advisor(i)`**: read `advisors[i]`. If its `id` ≠ 0: `CreateInstance(class_id, 0xCA09F544)`. If that
  succeeds, `map[id] := advisor`, then `advisor.Init(i)`. If `Init` fails, `RemoveAdvisor(id)`. Otherwise post
  `AdvisorAdded` `0x8A09FA6A` (§10).
- **`load_advisors()`**: if the global `advisors` is a table, `load_advisor(i)` for i = 1..n. Return true; false if
  there is no table.
- **`update_advisors()`**: for each entry i with `id` ≠ 0: if `GetAdvisor(id)` is null, `load_advisor(i)`;
  otherwise `advisor.Init(i)` (re-initialise in place). This is how a loaded city's advisors pick up the current
  scripts.
- **`RemoveAdvisor(id)`** (slot 6): if not found, return false. Send `AdvisorRemoved` `0x8A09FA69`. Remove the advisor
  from every event subscription set (dropping empty sets), erase it from the map. Return true.
- **`AddAdvisor(a)`** (slot 5): `map[a.GetID()] := a` (replacing). Post `AdvisorAdded`. Return true. A null `a`
  returns true and does nothing.
- **`kill_advisors()`**: send `AdvisorRemovedAll` `0x8A09FA6B`, shut down and release every advisor, clear the map.

### 4.4 Advisor `Init(i)` / `Shutdown()` (`cSC4Advisor`)

`Init(i)` → bool:

1. Read `advisors[i]`. Its `id` must equal the advisor's ID when the advisor already has one; otherwise return false.
   Store the table index, caption (cut at `|`), personal name and `advice_type`. Post `AdvisorChanged` `0x8A09FA68`.
2. Collect the matching `advices` entries (§2.2) as a map GUID → (table index, class ID).
3. Clear the **active state** of every advice: `SetFlags(active, off)` only (no deactivation messages), clear the
   active map, send `AdviceRemovedAll` `0x8A09FA67`. *[quirk]* An advice that was active when the city was saved is
   inactive after loading, while its expiry stays as saved.
4. For each existing advice (processed back to front):
   - its GUID is no longer in the table → remove it;
   - `advice.Init(index)` fails → clear its class-mismatch flag and remove it (it is **not** re-created);
   - `Init` succeeds but sets class mismatch → remove it and create it again below;
   - otherwise it is kept, and its GUID is taken off the to-create list.
   Removing = `RetireAdvice(advice)` (slot 20) + `advice.Shutdown()` + erase.
5. Create the remaining entries in ascending GUID order: `CreateInstance(class_id, 0x8A09F5F5)`, `Init(index)`,
   `SetAdvisorID(id)`, append info `{nextDay = currentDay, expiry = 0, lastMonth = monthCounter − 1}`.
6. Rebuild the event multimap and subscribe this advisor (system slot 16) for every event advice's event ID.
7. Load the head animations (UI) and, once, subscribe the four debug messages (§5.4). Return true.

The **base** `Init` (`cSC4AdvisorBase`, also used by every subclass) first subscribes, once, `InsertOccupant`
`0x99EF1142`, `RemoveOccupant` `0x99EF1143`, `PostCityInit` `0x26D31EC1`, `PostCityInitComplete` `0xEA8AE29A` and
`OrdinanceUpdated` `0xC2C09554`, then calls `cSC4Advisor::Init`. Every subclass `Init` sets **dirty := 1** (so the
variables are published on the next day) and then calls the base `Init`. Finance and CitySituations share one `Init`
body; CityPlanning, Safety, Traffic and Utility also subscribe their own messages (§5.5).

`Shutdown()` → true: unsubscribe the debug messages; month counter := current day := 0; clear the active map; send
`AdviceRemovedAll`; unsubscribe all events (system slot 17); shut down and release every advice. The base and
subclass `Shutdown`s unsubscribe their messages first and clear dirty.

### 4.5 Advice `Init(i)` → bool (`cSC4Advice`)

1. Read `advices[i]`. Its `guid` must equal the saved GUID if the advice was loaded; otherwise return false.
2. Re-read priority, mood, frequency, timeout (§2.2).
3. **Compile the trigger**: take the `trigger` text (`"0"` if empty) and run the Lua chunk
   `_atrg_ = function () return (<trigger>); end`. If that succeeds and `_atrg_` is a function, keep a Lua
   reference to it. With the app's debug functionality enabled, the trigger text is also kept for display.
4. Set or clear `once`, `news_only`, `event` (and the event ID), `persist`, `no_timeout` from their fields. Read
   `command`. Compare `class_id` with the object's own class ID: different → set class mismatch.
5. Read `effects` (a list of IDs) and **append** it to the effect list.
6. If the advice is active and was not just loaded, rebuild its text. Clear the loaded byte. Return true.

`cSC4RewardAdvice::Init` also sets flag `0x1000`. `cSC4AdviceNeghborDeal::Init` reads `nd_type`.

### 4.6 System `Shutdown()` → true

Unregister the detokenizer; unsubscribe `0x26D31EC1`, `0x26D31EC4`, `0xCA5501F2`; remove the advisor window
(slot 28); `kill_advisors()`; unregister both cheat codes; set the Lua global `game` to nil and force a collection
(`SetGCThreshold(0)`); release the Lua context; give the tutorial registry a null Lua context; clear initialised.

---

## 5. Cadence and messages handled

The advisor system is **not** a simulator agent. It runs from its own self-posted **ticker** message, so it runs in
the message pump after the simulator has advanced the date *[inferred: one ticker per pump pass]*.

### 5.1 The ticker `0xCA5501F2` (`kAdvisorSystemTicker_MessageType`) *[compat]*

`tick_self()` posts a new ticker message (no target, `MessagePost(msg, 0)`). Handling one:

1. If a city exists and is **not established** (`cISC4City` `+0x58`): return **without** re-posting. The chain stops
   until `CityEstablished` or `PostCityInit` restarts it.
2. `d :=` the simulator date (`+0x1C` `GetSimDate`). If its day number > lastDay (signed):
   1. lastDay := d.
   2. If the day of the month is **28**: force a Lua garbage collection (`GetGCCount`, `GetGCThreshold`,
      `SetGCThreshold(0)`), then **monthly step**: `Simulate(1)` on every advisor in ascending ID order.
   3. Set the Lua variables `g_year`, `g_month`, `g_day`, `g_date` (the day number) and `g_year_count` (current year −
      the year of the simulator's start date, `+0x14`). Each value is converted from uint32 to double.
   4. Go to step 4.
3. Not a new day: if the simulator is paused (`IsAnyPaused`, `+0x48`) **and** the idle timer shows more than 500 ms,
   go to step 4. Otherwise go to step 5.
4. If reloadPending: `reload_data()` and `update_advisors()`. Then **daily step**: `Simulate(0)` on every advisor in
   ascending ID order. Restart the idle timer.
5. `tick_self()`.

Consequences:
- While running, the daily step runs once per simulated day and the monthly step on day 28 of each month, just before
  that day's daily step.
- *[quirk]* If the simulation advances more than one day between two tickers and skips day 28, that month has no
  monthly step *[inferred: depends on frame pacing]*.
- *[quirk]* While paused, the daily step repeats every 0.5 s on the same day. It re-runs the active-advice pass
  (§6.2 step 3) but cannot re-evaluate advices already evaluated this month.

### 5.2 Other system messages

| Message | Handling |
|---|---|
| `PostCityInit` `0x26D31EC1` | If the city (message void1) is established: `tick_self()`; if lastDay < 1, monthly step. Then clear loadedFromSave. |
| `CityEstablished` `0x26D31EC4` | `PushEvent(0xAA31A210 NEW_CITY, null)`; `tick_self()`; if lastDay < 1, monthly step. |
| Cheat `0x230E27AC`, data `0xAAB7F42A` `DollyLlama` | toggle a global byte read only by advisor UI code. |
| Cheat, data `0xA919D25E` `Adv <args>` | debug only. `guids on`/`guids off` → post `0xEA917750` (data1 1/0); `trigger <hex>` → post `0x4A91DD6E` (data1 = hex); `print` → post `0x4A92ECEF`; `refresh` → post `0x8A917759`; `reload` → `reload_data()` and `update_advisors()`. |

### 5.3 Advisor `Simulate(kind)` (iface slot 14) → true

- `Simulate(0)` (daily): current day := the simulator date's day number; call the class's **simulate_day**.
- `Simulate(1)` (monthly): month counter += 1; call class slot 8. For the base and every subclass, slot 8 calls
  **update_variables** (§7). For a plain `cSC4Advisor` it does nothing.
- Any other value: nothing.

The base `simulate_day`: if dirty, `update_variables()` and dirty := 0. Then the `cSC4Advisor` day pass (§6.2).

So the variables are published (a) on every monthly step, (b) on the first daily step after `Init` or after a
`PostCityInit`, and (c) in part (the "occupant variables") whenever buildings change (§5.4).

### 5.4 Base `DoMessage` (`cSC4AdvisorBase`) *[compat]*

| Message | Handling |
|---|---|
| `InsertOccupant` / `RemoveOccupant` | If the occupant is a building (`0x278128A0`) and its property `0xEA3209F8` (conditional building) is true and `BuildingDevelopmentSimulator` `+0x38` `GetBuildingKeyFromType(type, &key)` succeeds: post **UpdateSelf**(1, key instance). |
| `UpdateSelf` `0xCA8F3DB6`, data1 = 1 | `a := GetAdviceByGUID(data2)`. If found: `a.GetTrigger()` (this refreshes the reward status, §6.6). If the civic simulator's status for that building is missing or lacks bit 2: `a.SetFlags(expire request, on)` and dirty := 1. |
| `UpdateSelf`, data1 = 2 | `GetTrigger()` on every advice with the reward flag. |
| `UpdateSelf`, data1 = 0 | nothing here (subclasses use it). |
| `OrdinanceUpdated` | post UpdateSelf(2, 0). |
| `PostCityInitComplete` | `GetTrigger()` on every advice whose class is `cSC4RewardAdvice`. |
| `PostCityInit` | `update_variables()`. |
| anything else | `cSC4Advisor::DoMessage` (debug messages below). |

UpdateSelf(a, b) is posted to the advisor's own message target (`MessageServer2` `+0x1C`
`GeneralMessagePostToTarget`), data1 = a, data2 = b.

`cSC4Advisor::DoMessage` handles only debug messages. `0x4A91DD6E` (data1 = an advisor ID or an advice GUID): set
forced trigger (`0x400`, or `0x600` for a GUID) on matching inactive advices whose trigger text is not `"=(0)"`, then
post `0xEA917750`. `0x4A92ECEF` prints, `0x8A917759` refreshes texts, `0xEA917750` sets or clears `0x200` on the
active advices.

### 5.5 Subclass messages *[compat]*

All subclasses keep a set of pending-update bits. On a building insert or remove they post UpdateSelf(0, 0) if no bit
was set, then set their bits; on UpdateSelf they run the matching update and clear the bits.

| Advisor | Messages | Effect |
|---|---|---|
| Finance | Insert/RemoveOccupant (building) | post UpdateSelf(0,0) if idle; mark pending. UpdateSelf clears it. *[quirk]* Nothing is recomputed: the flag has no reader. Then the base. |
| CityPlanning | Insert/RemoveOccupant | Non-buildings: return **without** calling the base *[quirk]*. Buildings: building count ±1; if group `0x1006` park count ±1; if group `0x1006` again, recreation count ±1 *[quirk: same group]*; if group `0x150A` (landmark): `key := GetBuildingKeyFromType`, insert sets `landmarks[key.instance] := 1`, remove decrements it if present (never erased). Then post/mark, then the base. |
| CityPlanning | UpdateSelf | occupant variables if pending; then the base. |
| CityPlanning | `0xEA394B74`, `0xCB3BE815`, `0xEB3BE415`, `0x2B3BE44A`, `0x6B3BE6D8`, `0x0B3BE81A` (airport growth), `0x8A394DC4` (seaport growth), `0x4A490D33` (seaport bad location) | subscribed, but no handler exists *[quirk]*: they fall through to the base and do nothing. |
| HQ | Insert/RemoveOccupant (building), UpdateSelf | pending → occupant variables. The base is called for every message. |
| Safety | Insert/RemoveOccupant, UpdateSelf | as HQ. |
| Safety | strikes, jailbreak, disasters, pipe burst, blocked vehicles | set variables and push events (§7.9). No base call. |
| Traffic | `0x4C04DE69` `LotDeveloperNotification` | data1 = 1: add the lot (message IGZUnknown) to the zot set; 2: remove it. Then `g_car_zot_count` := set size. No base call. |
| Traffic | Insert/RemoveOccupant (building or network `0xC772BF98`) | pending → occupant variables. Then the base. |
| Utility | Insert/RemoveOccupant (building or network) | pending (bits 1, 2) → occupant variables. |
| Utility | `0xEA418D8F` (neighbour-deal status report, neighbors.md §5.3) | pending (bit 4) → deal variables. data1 3/4/5/6 → push event `0x3A383059` / `0x0A383053` / `0xAA383052` / `0x4A383049` to **this advisor** with a subject whose object is the deal (§6.3). |
| Utility | `0xCA358F85` / `0xCA358F9C` (power strike begin/end) | `g_power_strike` := 1/0, `PushEvent(0xEA31A30A / 0x2A31A30F)`. *[quirk]* No code posts these messages. |
| CitySituations | none of its own | |

### 5.6 Ordering

The daily step reads the other subsystems' **current** values at the moment the ticker runs, after the simulator's
day tick *[inferred]*. Values computed monthly by the simulators (foundations §13) are read as they are, whatever
their month position. Nothing in the simulation reads the advisor variables back except through Lua (§14.1).

---

## 6. Advice evaluation *[compat]*

### 6.1 Triggers

`get_raw_trigger()` (raw value):
1. `PushContextAdvice(this)` (§6.7).
2. No compiled trigger → false. Otherwise call the compiled function (protected, 0 arguments, 1 result).
   A Lua error → false (with debug functionality, the error is reported).
3. A **boolean** result is used as is. A **number** is true unless it equals 0.0 (NaN → true). Any other type → false.
4. Pop the result and `PopContextAdvice()`.

`GetTrigger()` (slot 8) → bool in AL: forced trigger (`0x400`) → true; otherwise `get_raw_trigger() && !no text
(0x80)`. Reward advices override it (§6.6).

### 6.2 The day pass (`cSC4Advisor::simulate_day`)

Let n = number of advices.

1. **Batch size** `b = trunc(f32(f32(n × 0.033333335f) + 1.0f))` (float32 per foundations §18). Evaluate b advices
   starting at the cursor, wrapping, and leave the cursor after the last one. So each advice is visited about once
   every 30 days.
2. For each visited advice that is neither active nor an event advice: if `nextDay ≤ currentDay` and `lastMonth <
   monthCounter`: lastMonth := monthCounter; if `GetTrigger()` → **activate** (§6.4).
3. Then for every **active** advice (in active-map order):
   - deactivate it if `currentDay ≥ expiry` and not no_timeout, **or** expire request is set, **or** it is not
     persist and `GetTrigger()` is false;
   - otherwise, if text dirty: rebuild its text and send `AdviceChanged` `0x8A09FA66`.
4. Process the event queue (§6.3).

Consequence *[compat]*: an advice is tested at most once per advisor month (monthCounter changes on the monthly step),
and not before `nextDay`.

### 6.3 Events

- **System `PushEvent(event, subject)`** (slot 15) → true: for every advisor subscribed to `event` (in pointer
  order), `advisor.PushEvent(event, subject)`.
- **Advisor `PushEvent(event, subject)`** (slot 18) → true: append (event, subject) to its queue (AddRef).
- `SubscribeToEvent` / `UnsubscribeFromEvent` (slots 16/17) maintain the system's event → advisor sets.
- **`simulate_day_events`** (end of the day pass): pop each queued (event, subject) in order. For each advice
  registered for `event`:
  - inactive: lastMonth := monthCounter; `SetEventSubject(subject)`; if `GetTrigger()` → activate. *[quirk]* No
    `nextDay`, `once` or month check: an event advice can fire on every event.
  - active: set text dirty; if `GetTrigger()` → `SetEventSubject(subject)`.

The **Utility** advisor overrides `simulate_day_events` for the four neighbour-deal events: for each advice of the
event, it reads the deal from the subject object (`cISC4NeighborDeal`, IID `0x0773FEF1`) and requires the deal's
type to equal the advice's `nd_type`. An inactive match is tested (`GetTrigger` before `SetEventSubject`) and
activated; an active advice for the same deal is refreshed; an active advice for **another** deal is cloned
(`Clone`, slot 5), and the clone is added to the vector and the event multimap and activated if its trigger is true.
*[quirk]* Clones are never removed. Other events follow the base logic, except that the trigger is tested before the
subject is set.

### 6.4 Activation and deactivation

**Activate** (index i): nextDay := 0; expiry := currentDay + timeout; active map[advice] := i; `SetFlags(active,
on)` (rebuilds the text if dirty and asserts the global effects); `SetFlags(expire request, off)`; post
`AdviceAdded` `0x8A09FA64` (IGZUnknown = advice). If the command ID ≠ 0, post `AdviceCommand` `0x8A09FA6C`
(IGZUnknown = advisor, data1 = command ID).

**Deactivate**: `SetFlags(active | expire request | forced trigger, off)` (sets text dirty, retracts the global
effects); nextDay := `0x7FFFFFFF` if once, else currentDay + frequency; expiry := 0; `SetEventSubject(null)`; **send**
`AdviceRemoved` `0x8A09FA65` (IGZUnknown = advice).

### 6.5 Retire, command test, lookups

- `RetireAdvice(a)` (slot 20) → bool: false if `a` is null or inactive. Otherwise erase it from the active map and
  deactivate it; true. *[quirk]* The base version does not check that `a` is in the active map; an active advice
  missing from the map would use a garbage index. The CitySituations override checks both and returns true even when
  the advice is not found.
- `CommandWouldTrigger(cmd)` (slot 19) → bool: scanning from the last advice, true if some **non-event** advice has
  `GetCommandID() == cmd` and a true `GetTrigger()`. Each tested trigger runs its Lua function.
- `GetAdviceByGUID(g)` / `GetAdviceBySessionID(s)` (slots 12/13): linear search, null if none.

### 6.6 Reward advices and conditional buildings

A reward advice (`cSC4RewardAdvice`, class `0xAA371C32`, used by the CityPlanning and Finance tables) belongs to a
reward building whose exemplar instance is the advice's GUID. Its `GetTrigger()`:

1. `PushContextAdvice(this)`. `st := CivicBuildingSimulator.GetConditionalBuildingStatus(guid)` (civic `+0x18`),
   copied (all zero if missing).
2. Call the entry's Lua function `condition` (protected). Its result sets the status:
   - a **string**: status &= ~3; the tooltip text := the detokenized string;
   - **number 0**: status := (status & ~2) | 1;
   - **number 1**: if status lacks bit 4: create a subject from the building type's data (BuildingDevelopmentSimulator
     `+0xA0`), `SetEventSubject` it and set trigger := true. Status := 6.
3. `CivicBuildingSimulator.UpdateConditionalBuildingStatus(guid, status)` (civic `+0x1C`). `PopContextAdvice()`.
4. Return trigger && !no text.

So **the civic building simulator does not compute reward availability**: the advice writes it. The shipped
conditions compare `game.g_population`, `game.ga_mayor_rating`, grades and similar variables with tuning constants.
Reward triggers are re-run on `PostCityInitComplete`, on `OrdinanceUpdated` and when a conditional building is
added or removed (§5.4). The Lua functions `get_reward_building_state`, `unlock_reward_building` and
`reward_instance_count` read and write the same status (§8.3).

### 6.7 Context advice

A stack of advices (each AddRef'd). `PushContextAdvice(a)` (slot 18) pushes; `PopContextAdvice()` (slot 19) removes
the top, releases it and returns the pointer (now unreferenced by the stack); `GetContextAdvice()` (slot 20) returns
the top or null. Most Lua functions act on the context advice (§8.3).

### 6.8 Popup priority thresholds

`init_advice_popup_priority_thresholds`: thresholds := 100, 100, 100, then read
`tuning_constants.ADVICE_POPUP_PRIORITIES_BY_SKILL_LEVEL` {`low`, `medium`, `high`} into difficulty 0, 1, 2 (a missing
key keeps 100). Shipped: 60, 80, 100. `GetAdvicePopupPriorityThreshold()` (slot 21) returns the value for
`cISC4City.GetDifficultyLevel()` (`+0x60`); any value other than 1 or 2 uses difficulty 0. Only the UI uses it.

### 6.9 `SetFlags`, text and global effects

`SetFlags(mask, on)` (slot 14) → true:
- on: flags |= mask. If mask contains active: rebuild the text if text dirty, and if the advice was not already active,
  notify the global effects (assert).
- off: flags &= ~mask. If mask contains active: set text dirty, and if the advice was active, notify the global
  effects (retract).
- Either way, mask containing `0x200` also sets text dirty.

`NotifyGlobalEffectTriggerChanged()` (slot 23, void): for each effect ID, View3D `+0xBC`
`AssertGlobalEffectTrigger(id)` if active, else `+0xC0` `RetractGlobalEffectTrigger(id)`.

`UpdateText()` (slot 28) calls the class's text rebuild (class slot 30). Text rebuilding reads `title`, `message` and
`news`, runs the detokenizer, and is UI only.

---

## 7. The Lua `game` variables *[compat]*

### 7.1 Conventions

- `SetGlobalValue(name, double)` (system slot 13) does `game[name] = value`. `SetGlobalValue(name, cIGZString*)`
  (slot 12) stores a string. `SetLocalValue(name, value, subject)` (slot 14) does `game[name] = value` and
  `game[name .. "_subject"] = subject` (subject-tag userdata, AddRef'd; a null subject gives a null userdata).
  All three assume `game` exists.
- **L** marks a local value (with subject). Subject "cell (x, z)" = a new subject with `SetLocation(float(x),
  float(z))`, city cells. Subject "object" = `SetObject(occupant)`.
- Conversions (foundations §18: every add, multiply and divide rounds to float32):
  - "×100" = `f32(v × 100.0f)`, then stored as double.
  - "u32" = the unsigned value converted exactly (the 2^32 correction add only applies above 2^31).
  - "int" = signed, exact. "i64" = signed 64-bit, exact.
  - "float" = the callee's float, widened.
- All values are recomputed in place; nothing is smoothed or averaged by the advisors.

### 7.2 System (ticker)

`g_year`, `g_month`, `g_day` (day of month), `g_date` (day number), `g_year_count` (years since the start date): u32.

### 7.3 CityPlanning `update_variables`

| Variable | Source | Conversion |
|---|---|---|
| `ga_mayor_rating` | Aura `+0x20` `GetMayorRating` (int8) | sign-extended int |
| `l_mayor_rating_l` / `l_mayor_rating_h` | Aura `+0x28` `GetMinMayorRating(&p)` / `+0x24` `GetMaxMayorRating(&p)` | int8 value; subject cell (p.x, p.z) |
| `g_city_r_population`, `_c_`, `_i_` | Demand `GetDemand(0x1001 / 0x3001 / 0x4001, 0)` → `QuerySupplyValue` | float |
| `g_city_rci_population` | `f32(f32(I + C) + R)` | float |
| `g_city_r1_population` … `_r3_` | supply of `0x1011`, `0x1021`, `0x1031` | float |
| `g_city_ir/id/im/iht/co3/co2/cs1/cs2/cs3_population` | Demand `+0x24` `GetJobsBySensus(0x4101, 0x4201, 0x4301, 0x4401, 0x3331, 0x3321, 0x3111, 0x3121, 0x3131)` | u32 |
| `g_region_r/c/i_population`, `g_region_rci_population` | as the city ones with region `0x20000` | float |
| `g_city_workforce_population` / `g_region_workforce_population` | supply of `0x2100` (city / region) | float |
| `g_r1_demand`, `g_r2_demand`, `g_r3_demand`, `g_co2_demand`, `g_co3_demand` | region nodes `0x1010`, `0x1020`, `0x1030`, `0x3320`, `0x3330` → `QueryDemandValue` | float |
| `g_workforce_demand` | region `0x2000` → `QueryDemandValue` | float |
| `g_*_active_demand` (r1, r2, r3, co2, co3, id, im, iht, workforce) | region nodes as above plus `0x4200`, `0x4300`, `0x4400`, `0x2000` → `QueryActiveDemandValue` (`+0x30`) | float |
| `g_current_{id,im,iht,co2,co3,r1,r2,r3}_cap` | region node → `GetDemandCap` (`+0x68`, a percentage) | ×100 |
| `g_tax_rate_neutral` | Demand simulator `+0x28` `GetNeutralTaxRate` | float |
| `g_num_rzone_{ld,md,hd}_tiles`, `g_num_czone_{ld,md,hd}_tiles`, `g_num_izone_{r,l,h}_tiles` | ZoneManager `+0x18` zone count of zone type 1..9 | int |
| `g_num_rail_neighbors`, `g_num_road_neighbors`, `g_num_avenue_neighbors` | Neighbors `+0x44` `GetConnectionsOfType(1 / 2 / 8)` list length | u32 |
| `g_num_cities_connected`, `_adjacent`, `_connected_indirectly` | Neighbors `+0x54` / `+0x50` / `+0x58` city set size | u32 |
| `g_city_has_sea` | Seaport developer `+0x3C` `CityHasSea` | 0/1 |
| `g_{r1,r2,r3,cs1,cs2,cs3,co2,co3,ir,id,im,ih}_demand_extrap` | RegionalCity `+0x44` `GetExtrapolatedPopulation(t)` − `+0x24` `GetPopulation(t)` for t = `0x1010`, `0x1020`, `0x1030`, `0x3110`, `0x3120`, `0x3130`, `0x3320`, `0x3330`, `0x4100`, `0x4200`, `0x4300`, `0x4400` | int (32-bit difference) |
| `g_r_demand_extrap`, `g_c_demand_extrap`, `g_i_demand_extrap` | sums of the r / c / i rows above | int |
| `g_city_tile_size` | `f32(SizeX × SizeZ)` (`cISC4City` `+0x164`, `+0x168`) | float, **square metres** (1 048 576 / 4 194 304 / 16 777 216) |
| `g_water_tile_count` | Terrain `+0xD8` (the count of water points) *[inferred name]* | int |

Then the occupant variables (also run on UpdateSelf):

| Variable | Source | Conversion |
|---|---|---|
| `g_parks_funding_p` / `g_landmarks_funding_p` | Budget `+0x70` `GetDepartmentBudget(0x8A4260A6 / 0x8A416368)` → `+0x28` funding percentage | ×100 |
| `g_num_buildings`, `g_num_parks`, `g_num_recreation` | the persisted counters (§5.5) | int |

### 7.4 Finance

| Variable | Source (Budget unless noted) | Conversion |
|---|---|---|
| `g_funds` | `+0x18` `GetTotalFunds` | i64 |
| `g_income_monthly` / `g_expense_monthly` | `+0x40` / `+0x38` | i64 |
| `g_tax_rate_{r_low,r_med,r_high,cs_low,cs_med,cs_high,co_med,co_high,i_resource,i_dirty,i_manufacturing,i_hightech}` | `+0x90` `GetTaxRate(0..11)` | float |
| `g_tax_income` | `+0x4C` | i64 |
| `g_borrowed`, `g_borrowing_limit`, `g_bond_payments_monthly` | `+0xA4`, `+0xA8`, `+0xB4` | i64 |
| `g_nd_income` / `g_nd_expense` | Neighbors `+0x80` / `+0x84` | i64 |

### 7.5 HQ (health and education)

`update_variables`:

| Variable | Source | Conversion |
|---|---|---|
| `g_health_funding_p` / `g_education_funding_p` | department `0x09188F42` / `0x09188F4C` → funding | ×100 |
| `ga_life_exp` | Residential `+0x88` `GetGlobalLE` (uint8 in AL) | int |
| `ga_health` / `ga_education` | Residential `+0x84` / `+0x80` | float |
| `l_tract_hq_l` / `l_tract_hq_h` | Residential `+0xB0` `GetHQMinAndMaxCellCoords` → min / max | float; subject cell of the min / max tract centre |
| `l_tract_life_exp_l` / `_h` | the **same** min / max HQ values and cells *[quirk]* | |
| `l_tract_eq_l` / `l_tract_eq_h` | Residential `+0xAC` `GetEQMinAndMaxCellCoords` | as above |
| `g_num_treated_patients`, `g_health_capacity` | **uninitialised** stack values *[quirk]* | u32 of garbage. FreeCity4 *[choice]*: 0 |

Occupant variables. Lists come from Residential `+0xC4` `GetHospitalBuildings(list, groups)` and `+0xBC`
`GetSchoolBuildings`. Groups: hospitals `0x1513`, clinics `0x1512`, all health {`0x1513`, `0x1512`}, elementary
`0x150F`, high `0x1510`, college `0x1504`, all schools {`0x150F`, `0x1510`, `0x1514`, `0x1504`}, museums `0x1506`,
libraries `0x1505`.

| Variable | Rule |
|---|---|
| `g_num_hospitals`, `g_num_clinics`, `g_num_schools`, `g_num_museums`, `g_num_libraries` | list length, u32 |
| `l_X_funding_l` (X = hospital, clinic, school, museum, library) | minimum over the list (walked **back to front**, strict `<`) of the building's funding fraction (Residential `+0x50`; health purpose for hospitals and clinics, school purpose otherwise), ×100; subject object = that building. Empty list: value `FLT_MAX × 100` = 3.4028234e40 and a null subject *[quirk]* |
| `l_X_grade_l` | minimum of `f32(grade × 100)` from the building's query data (Residential `+0x6C` hospital / `+0x68` school; grade = float property `0x092453CB` / `0x292453C5`). Empty: `FLT_MAX` |
| `g_health_coverage_p` | totals over all health (Residential `+0x3C`): `f32(f32(served / f32(population + 1)) × 100)` |
| `ga_health_grade` | `H = f32(hospitals + clinics)`; H > 0 ? `f32(f32(Σgrade / H) × 100)` : 0 |
| `ga_school_grade`, `ga_museum_grade`, `ga_library_grade` | Residential `+0x38` totals; count > 0 ? `f32(f32(Σgrade / count) × 100)` : 0 |
| `g_num_museum_served`, `g_num_library_served` | served total, u32 |
| `g_education_coverage_p` | as health coverage over all schools |
| `g_num_educational_buildings` | the all-schools count again *[quirk]* |
| `g_num_elem_schools`, `g_num_high_schools`, `g_num_colleges` | list lengths |

### 7.6 Safety

`update_variables` (the department `0x28F55A9F` is fetched first and discarded):

| Variable | Source | Conversion |
|---|---|---|
| `g_fire_strike_chance` | Fire `+0x30` `GetChanceOfStrike` | ×100 |
| `g_population` | Residential `+0x40` | int |
| `l_flammability_h` | Flammability `+0x1C` `GetFlammabilityHotspot(&v, &x, &z)` (v, x, z zeroed first) | int; subject cell (x, z) |
| `ga_crime` | Crime `+0x18` `GetAverageCrime`, **low byte only** *[quirk]* | int |
| — | HistoryWarehouse `+0x30` `RecordHistoryData(0xAA662693, trunc(ga_crime))`: the **only writer** of the GA_CRIME trend | |
| `l_crime_h` | Crime `+0x20` `GetCrimeHotspot` into the **same** v, x, z, not re-zeroed: with no hotspot it repeats the flammability hotspot *[quirk]* | int; subject cell |
| `g_inmate_count` | Police `+0x78` (jail population) | int |
| `g_criminal_count` | Police `+0x7C` (jailed count) | int |
| `g_crime_commited_count` | Crime `+0x1C` `GetMonthlyCrimeCount` | u32 |
| `g_arrest_count` | Police `+0x88` | int |
| `g_police_strike_chance` | Police `+0x2C` `GetChanceOfStrike` | ×100 |
| `g_school_strike_chance` / `g_health_strike_chance` | Residential `+0x28` / `+0x2C` | ×100 |

Occupant variables:

| Variable | Source |
|---|---|
| `g_fire_funding_p`, `g_police_funding_p` | department `0x28F55A9F` / `0xA2963983` funding ×100 |
| `g_fire_station_count`, `g_police_station_count`, `g_jail_count` | Fire `+0x5C`, Police `+0x60`, `+0x8C` (u32) |
| `ga_flammability` | Flammability `+0x20` (uint8) |
| `ga_fire_coverage` / `g_fire_coverage_p` | Fire `+0x4C` (float) / `+0x50` ×100 |
| `ga_police_coverage` / `g_police_coverage_p` | Police `+0x48` / `+0x4C` ×100 |
| `g_jail_capacity` | Police `+0x74` (int) |
| `l_fire_funding_p` / `l_police_funding_p` | minimum station funding (Fire `+0x58` list, `+0x20` per station; Police `+0x54`, `+0x1C`), walked front to back, strict `<`, starting from **100.0**: ×100, subject object = that station. No stations: 10000 and a null subject *[quirk]* |
| `l_fire_station_no_roads` / `l_police_no_roads` | 1 with subject = the first station whose lot exists and has no road access (Lot `+0x70`), else 0 |

Messages (§5.5): health strike `0x077ACCD2` and school strike `0x077ACCD0` set `g_health_strike` / `g_school_strike`
and push `0xAA60FACA`/`0x2A60FACF`, `0xEA60FABF`/`0x2A60FAC7`; jailbreak `0x2A614891`/`0x2A614892` → `g_jailbreak`,
events `0xCA616650`/`0x6A61665A`; fire strike `0x68F54CE1`/`0x68F54CE2` → `g_fire_strike`, events
`0x0A31A314`/`0x6A31A319`; police strike `0xA8DAF511`/`0xA8DAF512` → `g_police_strike`, events
`0x6A31A2FE`/`0x8A31A303`; transit strike `0x69247DC4`/`0x69247DC5` → `g_transit_strike`, event = the message ID;
nuclear explosion `0xAA552F77` → event `0xAA552F77`; zoo escape `0xAA667CE2` → event `0x2A667FC3`; pipe burst
`0xEA1F0DCD` → event `0x4A31CF15` (subject cell (data1, data2)) unless a user disaster is running; emergency route
blocked `0xCA8DCDB4` (data2 > 2) → `0xCA3FCEFA` (data1 0) or `0xCA3FCEF4` (data1 1).

`DisasterPeriodStarted` `0xCA1B1E64`: `g_disasters_in_progress` := 1; the disaster type from the instance
(`0x621CDA33` fire, `0xCA380B06` riot, `0x4A1332E8` pipe burst); `g_disasters_fire_started`, `_fire_ended` := 0,
`g_disasters_riot_started`, `_riot_ended`; for fires the cause (`+0x38`): 1, 3 user; 2 arson; 4 effects (and user);
5 flammability → `g_disasters_fire_cause_{arson,effects,flammability}` *[inferred: variable names from
adv_game_data.lua]*. Any type other than fire, riot or pipe burst counts as a user disaster. Event `0xCC3298DC XOR
type` for a user disaster, else `0xCA1B1E64 XOR type`, subject cell (data2, data3); then, if not a user disaster,
event `0xCA3C0412`.
`DisasterPeriodEnded` `0xAA1B2002`: `g_disasters_in_progress` := 0; from the disaster layer's statistics:
`g_disasters_damage_building_count` (u32), `g_disasters_damage_cost` (i64),
`g_disasters_{police,fire}_dispatch_effectiveness` (×100); fire / riot ended flags; event `0xAA31A2F7`.

### 7.7 Traffic

Most values come from the traffic simulator's statistics block (Traffic `+0x88` `GetTrafficStats`, a pointer into
the traffic simulator; traffic spec). Per network (road, rail, street, avenue, subway, elevated, monorail, bridge):
`ga_<net>_congestion` (average ×100), `g_<net>_congestion_tile_count` (u32), `l_<net>_congestion_h` (worst value
×100, **not** truncated, subject cell). Per station kind (bus, train, subway, elevated, monorail):
`ga_<kind>_station_utilization_p` (×100), `l_<kind>_station_utilization_pl` (`trunc(f32(v × 100))`, subject cell).
Also `g_road_funding_p`, `g_masstransit_funding_p` (×100), `g_transit_strike` (0/1), `ga_freight_trip_length` (u8).

| Variable | Source |
|---|---|
| `ga_airport_efficiency` / `ga_seaport_efficiency` | port developer `+0x14` `GetAvgEfficiency` ×100. With no port the average is 1.0 → 100 *[quirk]* |
| `l_airport_no_road_access` / `l_seaport_no_road_access` | 1 with subject cell of the first port lot without road access (scanning from the back), else 0 and a null subject |
| `l_road_connection_congestion_h` / `l_avenue_connection_congestion_h` | maximum (strict `>` from 0) of the congestion map (Traffic `+0x28`, `GetCellValue`) at each road / avenue connection's inner cell (Neighbors `+0x44`, connection `+0x1C`); subject cell. *[quirk]* The avenue pass does not reset the location: with no congested avenue cell its subject is the road maximum's cell |
| `g_rail_neighbor_connection_count` | rail connection count (u32) |

Occupant variables: station counts, `g_train_freight_depot_count`, `g_num_rapidtransit_transitions`, `g_pothole_count`,
`g_<net>_tile_count` (road, rail, subway, highway, street, avenue, elevated_rail, monorail, dirtroad,
groundhighway); `g_airport_count` and `g_{small,medium,large}_airport_count` and the `_s1`/`_s2`/`_s3` stage counts
from `GetNumPorts(sizeMask, stageMask)` (port `+0x24`; sizes 2/4/8, all = 0xE; stages 1/2/4, all = 0x1F);
`g_seaport_count`, `g_seaport_s1..s3_count`; `g_seaport_volume` (`+0x34`, u32); `g_car_zot_count` (§5.5).
*[quirk]* `ga_rail_congestion` and the elevated and monorail station values are written twice (same value).

### 7.8 Utility

| Variable | Source | Conversion |
|---|---|---|
| `g_power_funding_p` | Power `+0xC0` | ×100 |
| `g_power_production_capacity` | Power `+0x30` `GetMonthlyProduction` (last month's production, not capacity *[quirk]*) | int |
| `g_power_consumed`, `g_overworked_power_plant_count`, `g_unpowered_building_count` | Power `+0x44`, `+0x6C`, `+0x54` | int |
| `g_power_strike` / `g_power_strike_chance` | Power `+0x9C` (AL) / `+0xA0` (reads the argument through EAX) | 0/1, ×100 |
| `g_power_imported` / `g_power_exported` | RegionalCity `+0xE0` / `+0xD8` (commodity 1) | double as returned |
| `g_water_funding_p` | department `0xAA4014B4` (no null check) | ×100 |
| `g_water_production_capacity` / `g_water_consumed` | Plumbing `+0x38` / `+0x3C` (low 32 bits) | int |
| `g_water_imported` / `g_water_exported` | RegionalCity `+0xE0` / `+0xD8` (commodity 2) | double |
| `g_pollution_funding_p` | department `0x6A42A8F5` | ×100 |
| `g_incinerator_capacity_daily`, `g_waste_to_energy_capacity_daily` | Pollution `+0x7C` / `+0x80` × 30 (int32, wraps) *[quirk: monthly × 30]* | u32 |
| `g_recycling_center_effect_p` | Pollution `+0x84` | ×100 |
| `ga_air_pollution`, `ga_water_pollution`, `ga_garbage_pollution` | Pollution `+0x28`, `+0x2C`, `+0x30`, max(v, 0) | int |
| `g_uncollected_garbage` | Pollution `+0x60` | int |
| `g_landfill_capacity` / `g_available_landfill_capacity` | LandfillDeveloper `+0x28` `GetLandfillUsage(&u, &cap)`: cap / cap − u | u32 |
| `g_garbage_produced`, `_recycled`, `_to_energy`, `_to_landfill` | Pollution `+0x8C`, `+0x98`, `+0xA0`, `+0xA4` | i64 |
| `g_garbage_imported` / `g_garbage_exported` | RegionalCity **`+0xD8` / `+0xE0`** (commodity 3; the slots are swapped on purpose, neighbors.md §8.2) | double |
| `l_air_pollution_h`, `l_water_pollution_h`, `l_uncollected_garbage_h` | Pollution `+0x44` / `+0x48` / `+0x4C` worst tract, max(v, 0) | int; subject cell |

Occupant variables: `g_power_plant_count` (Power `+0x60`), `g_power_plant_count_{coal,gas,oil,wind,solar,nuclear,
waste,fusion}` (`+0x5C` type 1, 3, 6, 9, 7, 5, 8, 2), `g_aged_power_plant_count` (`+0x70`), `g_power_pole_count`
(`+0x68`); `l_power_plant_funding_pl` / `l_power_plant_age_h` (Power `+0x78` `FindPowerPlants`, funding preset 1.0,
age preset 0; subject object = the plant or null); `g_water_source_count` (Plumbing `+0x30`), `g_water_pipe_count`,
`g_water_distressed_pipe_count`, `g_water_burst_pipe_count` (`+0x48`, `+0x4C`, `+0x50`), `g_watered_building_count`
/ `g_unwatered_building_count` (`+0x54` / `+0x58`), `g_water_pollution_pump_shutdown_count` (`+0x34`);
`l_water_building_funding_pl` / `l_water_building_age_h` (Plumbing `+0x6C`); `g_incinerator_count`,
`g_waste_to_energy_building_count`, `g_water_treatment_plant_count`, `g_recycling_center_count` (Pollution `+0xA8`,
`+0xAC`, `+0xB4`, `+0xB0`).

Neighbour-deal variables:
- `g_nd_count_{buy_water,sell_water,buy_power,sell_power,export_garbage,import_garbage}`: Neighbors `+0x78`
  `FindDeals(type)` list length for `0x4773FFE2`, `E3`, `E0`, `E1`, `E4`, `E5` (u32).
- `g_nd_connection_present_{water,power,garbage}`: over `GetAllConnections` (`+0x40`), pipe → water, power pole →
  power, rail or road → garbage *[quirk: not highway or avenue]*.
- `g_nd_can_{buy_water,buy_power,export_garbage,sell_water,sell_power,import_garbage}`: from `ProposeDeals(list,
  false)` (`+0x64`). *[quirk]* This call has side effects: it refreshes the regional commodity statistics and
  **draws from the neighbours' process-wide random generator** once per proposal (neighbors.md §7.1).

### 7.9 Events pushed by the advisors

CityEstablished → `0xAA31A210`; Safety and Utility messages (§5.5, §7.6); the Utility deal events (§6.3). Other
pushers: the building development simulator (event `0x4A64A9CC` to the CityPlanning advisor on building changes
*[inferred details]*), the port developers (`SendAdvisorMessage`), the city situation manager, MySim, and Lua
`game.trigger_event` (§8.3).

---

## 8. Lua functions

### 8.1 Tags and the gc method

Tag 0 = advice-subject userdata, tag 1 = string userdata (`cIGZString`). Each userdata holds one AddRef'd pointer.
The gc method (shared by both tags) checks that the Lua state is the system's, the value is userdata and its tag is
tag 0 or 1, then releases the pointer.

### 8.2 Registration (`register_api(bool global)`)

Every extension is `{name, function, isGlobal}` in one list built by two static collectors (38 simulation entries
and 9 UI entries). `register_api(true)` registers the global entries as globals; `register_api(false)` registers the
others into the table `game` (which the scripts must have created; otherwise the protected-less `SetTable` raises a
Lua error). Registration leaves the global `_tf_` pointing at the last function *[quirk]*.

### 8.3 `game.*` functions by subsystem *[compat]*

"ctx" is the context advice (§6.7). Numbers are truncated toward zero where an integer is needed. A bad argument
usually returns no value.

| Group | Function | Behaviour |
|---|---|---|
| History | `trend_value(type, months)` | HistoryWarehouse `+0x20` value at day `now − 30·months`; 0 if missing |
| | `trend_slope(type, months)` | `+0x18` trend slope m over [now − 30·months, now]: `f32(atan(m) × 63.6618f)` (≈ ±100); 0 if missing |
| | `trend_count(type, months)` | `+0x14` count over [now − 30·months + 1, now] |
| Ordinance | `ordinance_is_on(id)`, `ordinance_is_available(id)` | Ordinance `+0x20` / `+0x1C`: 1 or nil |
| | `ordinance_set_on(id)`, `ordinance_set_off(id)` | `+0x34 SetOrdinanceOn(id, true/false)` |
| | `ordinance_enact_fee`, `ordinance_repeal_fee`, `ordinance_monthly_cost` | ordinance `+0x2C` / `+0x30` / `+0x50` (i64 as double); 0 if missing |
| | `ordinance_name(id)`, `ordinance_description(id)` | string userdata (tag 1) or nil |
| Advice | `event_feedback([n])` | post a message whose type is ctx's event ID, IGZUnknown = ctx's subject object, data1 = n (default 0) |
| | `expire_advice()` | ctx `SetFlags(expire request, on)` |
| | `retire_advice()` | ctx's advisor `RetireAdvice(ctx)` |
| | `advice_subject()` | a **clone** of ctx's subject (subject `Clone`, slot 13) as tag-0 userdata |
| | `trigger_event(guid)` | system `PushEvent(guid, null)` |
| | `difficulty_level()` | 0/1/2, or −1 |
| | `random_chance(p)` | §9: 1 if a uniform draw in [0, 100) is < p, else nil |
| | `is_tutorial([guid])` | RegionalCity `+0x104` tutorial GUID compare |
| | `advice_info()` | debug message box |
| Subjects | `building_is_group(subj, group)` | the subject object as occupant → `IsOccupantGroup` |
| | `event_building_distance()` | subject data1 (u32), only for event `0xCA4E8986` (MySim building added) |
| | `event_disaster_distance()` | subject data1 (u32) |
| | `nd_cancel(subj)` | the subject object as neighbour deal → `Cancel` (`+0x88`) |
| MySim | `mysim_distance_to_closest_building`, `…_landmark_or_reward`, `mysim_local_school_grade`, `mysim_local_hospital_grade`, `mysim_dispatch_tool` | ctx's advisor as a MySim (§14.3) |
| Rewards | `reward_instance_count(id)` | civic status building count; if no status, the CityPlanning landmark count for `id` |
| | `get_reward_building_state(id)` | 1 if no status, 2 if status bit 2; **otherwise returns its argument** *[quirk]* |
| | `unlock_reward_building(guid)` | status := 2 (whole byte), `UpdateConditionalBuildingStatus` |
| UI | `camera_jump`, `camera_zoom`, `tool_plop_*`, `tool_button`, `window_*`, `window_query` | UI only |

Globals: `_combine_guids(a, b)` = `a XOR b` as a signed int32 (−1 on bad arguments); `_dofile_from_resource(name)`
(Lua host); `print(...)` (no output).

### 8.4 What reads the variables

Only the Lua scripts: triggers, reward conditions, text tokens and the MySim scripts. Shipped reward conditions use
`ga_mayor_rating`, populations and grades (§6.6).

### 8.5 Detokenizer (UI)

`%token%` text in advice texts: `link_id`, `sit_link_id`, mission tokens (city situation data), `advisor`,
`building_initial_cost`, `building_monthly_cost`, and Lua expressions evaluated as `_dasd_=(expr)` and formatted by a
type prefix. Unknown tokens become `%%UNKNOWN TOKEN --<token>--%%`. Out of scope beyond this list.

---

## 9. Random numbers *[compat]*

| Draw | Generator | Draws |
|---|---|---|
| `game.random_chance(p)` | the system's own `cRZRandom`, seeded with `0xFFFFFFFF` (wall clock) in the constructor; `RandomDoubleRangeUniform(0, 100)` (foundations §6.1) | 1 per call: `u = f32(int32(mul)/2^32 + 0.5)`, `r = f32(u × 100)` |
| Utility deal variables | the neighbours' process-wide `cRZRandom` through `ProposeDeals` | 1 per proposal (neighbors.md §7.1) |

No other draw. Seed handling follows foundations §6.3 *[choice]*. `GetRandom()` (slot 9) returns the address of the
system's generator.

---

## 10. Messages posted *[compat]*

| ID | Name | How | Data |
|---|---|---|---|
| `0x8A09FA64` | AdviceAdded | post | IGZUnknown = advice |
| `0x8A09FA65` | AdviceRemoved | send | IGZUnknown = advice |
| `0x8A09FA66` | AdviceChanged | send | IGZUnknown = advice |
| `0x8A09FA67` | AdviceRemovedAll | send | IGZUnknown = advisor |
| `0x8A09FA6C` | AdviceCommand | post | IGZUnknown = advisor, data1 = command ID |
| `0x8A09FA68` | AdvisorChanged | post | IGZUnknown = advisor |
| `0x8A09FA69` | AdvisorRemoved | send | IGZUnknown = advisor |
| `0x8A09FA6A` | AdvisorAdded | post | IGZUnknown = advisor |
| `0x8A09FA6B` | AdvisorRemovedAll | send | — |
| `0xCA8F3DB6` | UpdateSelf | post to the advisor's own target | data1, data2 (§5.4) |
| `0xCA5501F2` | ticker | post (broadcast) | — |
| ctx event ID | `game.event_feedback` | post | IGZUnknown = subject object, data1 |
| `0xEA917750`, `0x4A91DD6E`, `0x4A92ECEF`, `0x8A917759` | debug | post | data1 |

"send" = `MessageSend` (synchronous), "post" = `MessagePost(msg, 0)` (foundations §5).

---

## 11. Persistence *[compat]*

Records use the stream methods and refs of foundations §2. Each object writes only its own record; references are
written with `SC4WriteGZSerializable` and read with `SC4ReadGZSerializable(stream, iid, &p, 0)`.

### 11.1 System (`0x4A0DD616`), version 1

```
u16  1
u32  advisorCount
advisorCount × { ref advisor ; s32 advisorID }      // map order (ascending ID)
```

`Read`: version must be 1, else false. Each advisor is read with IID `0xCA09F544` and stored as `map[id]`. Then
loadedFromSave := !error; returns that. Nothing else is saved: subscriptions, thresholds and the Lua state are
rebuilt by `Init`.

### 11.2 Advisor (`0xEAC937B4` and every subclass CLSID), version 3

```
u16  3
s32  tableIndex
str  caption                    // GZStr
str  personalName
s32  monthCounter
s32  currentDay
s32  advisorID
s32  cursor
u32  n
n × { ref advice ; s32 expiry ; s32 lastMonth ; s32 nextDay ; s8 active ; s32 eventID }
u32  m
m × { s32 eventID ; ref subject }                   // the event queue
```

`active` is `CheckFlags(2)`; `eventID` is the advice's `GetEventID()`. `Read`: version must be 3. Advices are read
with IID `0x8A09F5F5`; an `active` advice is put in the active map; a non-zero event ID goes into the event
multimap. Subjects are read with IID `0x6A26D050`. Returns `!GetError()`. On `Init` the active state is cleared
again (§4.4).

### 11.3 Advice (`0x8A09F5F4` and subclass CLSIDs), version 4

```
u16  4
s32  tableIndex
u32  guid
u32  flags
s32  advisorID
s32  priority
s32  frequency
s32  timeout
s32  mood
u32  effectCount
effectCount × u32 effectID
ref  eventSubject
```

`Read`: the version is **not checked**. The effect list is read only when version > 3, and **appended** to the
current list *[quirk]*. The subject is released first and read with IID `0x6A26D050`. Then text dirty is set, the
loaded byte := !error, and that is returned. Subclasses that add state (neighbour deal `nd_type`) do not save it.

### 11.4 Subject (`0x6A26D049`), version 4

```
u16  4
u8   hasLocation
[f32 x ; f32 z]                 // only if hasLocation
u32  data1
u32  data2
ref  object                     // read with IID 0x00000001
```

`Read` rejects any version other than 4.

### 11.5 CityPlanning (`0x2A5F877D`), version 3

```
u16  3
<advisor record §11.2>
s32  buildingCount
s32  parkCount
s32  recreationCount
s32  landmarkCount
landmarkCount × { s32 exemplarInstance ; s32 count }   // ascending signed key
```

`Read` accepts any version. The three counters are always read. The landmark map is cleared and read only if
version > 2. A failed base read does not stop it; it returns `!GetError()` *[quirk]*.

### 11.6 Base-class advisors *[quirk]*

`cSC4AdvisorBase`'s serializable vtable reports CLSID `0xEAC937B4` (the plain `cSC4Advisor`), not `0xCA09F543`. So
the Environment (25) and Fluff news (28) advisors are saved as `cSC4Advisor` and come back as plain `cSC4Advisor`
objects. After loading they have no base message subscriptions and no dirty handling. Neither uses reward advices or
variables, so nothing visible changes. A replacement must write `0xEAC937B4` for them and accept both classes.

---

## 12. Interface behaviour (compiled slot tables) *[compat]*

Pop = callee-pop bytes (`abi_extract.py --check`). Return channels read from the epilogues.

### 12.1 `cISC4AdvisorSystem`, vtable `0x00A82AC8`, 29 slots

| Slot | Off | Method (as the game expects it) | Pop | Return | Address |
|---|---|---|---|---|---|
| 0–2 | | QueryInterface / AddRef / Release | 8/0/0 | | |
| 3 | `+0x0C` | `bool Init()` | 0 | AL true | `0x41B570` |
| 4 | `+0x10` | `bool Shutdown()` | 0 | AL true | `0x41A3B0` |
| 5 | `+0x14` | `bool AddAdvisor(cISC4Advisor*)` | 4 | AL true | `0x4197E0` |
| 6 | `+0x18` | `bool RemoveAdvisor(int32 id)` | 4 | AL | `0x419BC0` |
| 7 | `+0x1C` | `cISC4Advisor* GetAdvisor(int32 id)` (no AddRef) | 4 | EAX | `0x419070` |
| 8 | `+0x20` | `std::map<int32, cRZAutoRefCount<cISC4Advisor>>& GetAdvisors()` | 0 | EAX | `0x520080` |
| 9 | `+0x24` | `cRZRandom* GetRandom()` | 0 | EAX (address of the member) | `0x800640` |
| 10 | `+0x28` | `cISCLua* GetScriptingContext()` | 0 | EAX | `0x40D390` |
| 11 | `+0x2C` | `bool DoLink(const cIGZString&)` | 4 | AL | `0x41B080` |
| 12 | `+0x30` | `bool SetGlobalValue(const char*, cIGZString*)` | 8 | AL | `0x418870` |
| 13 | `+0x34` | `bool SetGlobalValue(const char*, double)` | 12 | AL | `0x4188E0` |
| 14 | `+0x38` | `bool SetLocalValue(const char*, double, cISC4AdviceSubject*)` | 16 | AL | `0x419520` |
| 15 | `+0x3C` | `bool PushEvent(int32, cISC4AdviceSubject*)` | 8 | AL true | `0x4190B0` |
| 16 | `+0x40` | `bool SubscribeToEvent(int32, cISC4Advisor*)` | 8 | AL | `0x41B370` |
| 17 | `+0x44` | `bool UnsubscribeFromEvent(int32, cISC4Advisor*)` | 8 | AL | `0x419EB0` |
| 18 | `+0x48` | `void PushContextAdvice(cISC4Advice*)` | 4 | none (EAX garbage) | `0x419840` |
| 19 | `+0x4C` | `cISC4Advice* PopContextAdvice()` | 0 | EAX (already released) | `0x418CE0` |
| 20 | `+0x50` | `cISC4Advice* GetContextAdvice()` | 0 | EAX | `0x418C10` |
| 21 | `+0x54` | `int32 GetAdvicePopupPriorityThreshold()` | 0 | EAX | `0x418780` |
| 22 | `+0x58` | `int32 GetDataTag(int32 i)` (no bounds check) | 4 | EAX | `0x418C30` |
| 23 | `+0x5C` | `cSC4AdvisorUIAdvisorWinProc* GetAdvisorWinProc()` | 0 | EAX | `0x6A0050` (folded) |
| 24 | `+0x60` | `void StartAdvisorWindow()` | 0 | none | `0x4187B0` |
| 25 | `+0x64` | `void ShowAdvisorWindow()` *[inferred name]* | 0 | none | `0x418820` |
| 26 | `+0x68` | `void HideAdvisorWindow()` | 0 | none | `0x418830` |
| 27 | `+0x6C` | `bool IsAdvisorWindowVisible()` *[inferred name]* | 0 | AL | `0x418810` |
| 28 | `+0x70` | `void RemoveWinProcs()` | 0 | none | `0x418840` |

SDK errors (checked): slot 9 returns a pointer to the system's `cRZRandom` (SDK `intptr_t`, harmless); slot 18
returns nothing (SDK `bool`; a replacement should return true); slots 23–28 are missing from the SDK; slot 6 is spelled
`RemoveAdivsor` in the SDK. The overload pair 12/13 is in reverse declaration order, as foundations §16 predicts.
Slots 23–28 are UI.

### 12.2 `cISC4Advisor` (`cSC4Advisor` iface vtable `0x00A811B0`, 21 slots)

| Slot | Off | Method | Pop | Return | Address |
|---|---|---|---|---|---|
| 3 | `+0x0C` | `bool Init(int32 tableIndex)` | 4 | AL | `0x410100` (base `0x410E30`) |
| 4 | `+0x10` | `bool Shutdown()` | 0 | AL true | `0x40ED30` (base `0x40EF30`) |
| 5 | `+0x14` | `int32 GetID()` | 0 | EAX | `0x40D390` |
| 6 | `+0x18` | `cIGZString* GetName()` (caption object) | 0 | EAX | `0x40D3A0` |
| 7 | `+0x1C` | `bool SetName(const cIGZString&)` | 4 | (tail call into the string's assign) | `0x40D3B0` |
| 8 | `+0x20` | `cIGZString* GetPersonalName()` | 0 | EAX | `0x800640` |
| 9 | `+0x24` | `bool SetPersonalName(const cIGZString&)` | 4 | tail call | `0x40D3C0` |
| 10, 11 | `+0x28`, `+0x2C` | `GetCurrentAdvices()` (both overloads, one body) | 0 | EAX: `std::map<cRZAutoRefCount<cISC4Advice>, int32>&` (the active map) | `0x40D3D0` |
| 12 | `+0x30` | `cISC4Advice* GetAdviceByGUID(uint32)` | 4 | EAX | `0x40D7A0` |
| 13 | `+0x34` | `cISC4Advice* GetAdviceBySessionID(int32)` | 4 | EAX | `0x40D7E0` |
| 14 | `+0x38` | `bool Simulate(int32 kind)` | 4 | AL true | `0x40D820` |
| 15 | `+0x3C` | `int32 GetHeadAnimCount(bool)` | 4 | EAX | `0x40D870` |
| 16 | `+0x40` | `bool SetHeadAnim(bool, int32)` | 8 | AL | `0x40D3E0` |
| 17 | `+0x44` | `int32 GetHeadAnimID(int32)` | 4 | EAX | `0x40F3A0` |
| 18 | `+0x48` | `bool PushEvent(int32, cISC4AdviceSubject*)` | 8 | AL | `0x40EEC0` |
| 19 | `+0x4C` | `bool CommandWouldTrigger(uint32)` | 4 | AL | `0x40D8A0` |
| 20 | `+0x50` | `bool RetireAdvice(cISC4Advice*)` | 4 | AL | `0x40E9B0` (CitySituations `0x413730`) |

SDK error (checked at `0x40D3D0`): `GetCurrentAdvices` returns a `std::map<advice, index>&`, not an
`SC4List<pair<int32, advice>>&`. The subclasses override only slots 3, 4 and (CitySituations) 20.

**Class (non-interface) vtable** of `cSC4Advisor` and its subclasses: 0 QI, 1 AddRef, 2 Release, 3 destructor,
4–5 inherited `cRZBaseString` helpers, 6 **simulate_day**, 7 **simulate_day_events**, 8 **monthly hook** (calls 9
in the base), 9 **update_variables**, 10 (CitySituations only) **UpdateMySimAdvices**. These are not public, but
the original calls them virtually and a subclass replacement must keep the order if it reuses the base code.

### 12.3 `cISC4Advice` (`cSC4Advice` vtable `0x00A808B8`, 31 slots)

| Slot | Off | Method | Pop | Return | Address |
|---|---|---|---|---|---|
| 3 | `+0x0C` | `bool Init(int32 tableIndex)` | 4 | AL | `0x408C70` |
| 4 | `+0x10` | `bool Shutdown()` | 0 | AL true | `0x407E90` |
| 5 | `+0x14` | `bool Clone(cISC4Advice**)` | 4 | AL | `0x407C50` |
| 6 | `+0x18` | `int32 GetMood()` | 0 | EAX | `0x407AA0` |
| 7 | `+0x1C` | `int32 GetPriority()` | 0 | EAX | `0x407AB0` |
| 8 | `+0x20` | `bool GetTrigger()` | 0 | **AL** (upper EAX garbage) | `0x408970` |
| 9 | `+0x24` | `cIGZString* GetTriggerString()` | 0 | EAX (string object) | `0x477890` |
| 10 | `+0x28` | `uint32 GetGUID()` | 0 | EAX | `0x7C9FD0` |
| 11 | `+0x2C` | `int32 GetAdvisorID()` | 0 | EAX | `0x407AC0` |
| 12 | `+0x30` | `void SetAdvisorID(int32)` | 4 | EAX = the argument | `0x407AD0` |
| 13 | `+0x34` | `int32 GetSessionID()` | 0 | EAX | `0x4FAF50` |
| 14 | `+0x38` | `bool SetFlags(uint32 mask, bool on)` | 8 | AL true | `0x407AE0` |
| 15 | `+0x3C` | `bool CheckFlags(uint32 mask)` (all bits of mask) | 4 | AL | `0x407B70` |
| 16 | `+0x40` | `int32 GetFrequencyInDays()` | 0 | EAX | `0x407BA0` |
| 17 | `+0x44` | `int32 GetTimeOutInDays()` | 0 | EAX | `0x6EFA50` |
| 18 | `+0x48` | `cIGZString* GetTextTitle()` | 0 | EAX (string object) | `0x800640` |
| 19 | `+0x4C` | `uint32 GetTextTitleGUID()` | 0 | EAX | `0x407D70` |
| 20 | `+0x50` | `cIGZString* GetTextMessage()` | 0 | EAX (string object) | `0x407BB0` |
| 21 | `+0x54` | `uint32 GetTextMessageGUID()` | 0 | EAX | `0x407DA0` |
| 22 | `+0x58` | `bool DoLink(const cIGZString&)` | 4 | AL | `0x407ED0` |
| 23 | `+0x5C` | `void NotifyGlobalEffectTriggerChanged()` | 0 | none | `0x407F40` |
| 24 | `+0x60` | `bool SetEventSubject(cISC4AdviceSubject*)` (sets text dirty) | 4 | AL true | `0x407DD0` |
| 25 | `+0x64` | `cISC4AdviceSubject* GetEventSubject()` | 0 | EAX | `0x7FB1D0` |
| 26 | `+0x68` | `int32 GetEventID()` | 0 | EAX | `0x6E0130` |
| 27 | `+0x6C` | `int32 GetCommandID()` | 0 | EAX | `0x407C20` |
| 28 | `+0x70` | `bool UpdateText()` (tail call to slot 30) | 0 | AL | `0x407C30` |
| 29 | `+0x74` | destructor thunk | 0 | — | `0x407B90` |
| 30 | `+0x78` | text rebuild (class virtual) | 0 | — | `0x409300` |

SDK errors (checked): slot 8 returns `bool` in AL (SDK `intptr_t`: reading EAX gives garbage in the upper bytes);
slots 9, 18, 20 return string objects (SDK `char*`); slot 12 returns nothing meaningful (SDK `cISC4Advice*`; EAX
holds the argument, a replacement should return `this`); slot 23 is void (SDK `intptr_t`); slots 29–30 are missing
(class virtuals).

### 12.4 `cISC4AdviceSubject` (vtable `0x00A81090`, 14 slots)

| Slot | Off | Method | Pop | Return | Address |
|---|---|---|---|---|---|
| 3 | `+0x0C` | `bool SetLocation(float x, float z)` (clamps, §2.5) | 8 | AL | `0x40D110` |
| 4 | `+0x10` | `float GetLocationX()` (0 without a location) | 0 | ST0 | `0x40CE10` |
| 5 | `+0x14` | `float GetLocationZ()` | 0 | ST0 | `0x40CE30` |
| 6 | `+0x18` | `bool HasLocation()` | 0 | AL | `0x40CE50` |
| 7 | `+0x1C` | `bool SetObject(cIGZUnknown*)` (AddRef) | 4 | AL | `0x40D130` |
| 8 | `+0x20` | `cIGZUnknown* GetObjectA()` | 0 | EAX | `0x40CF10` |
| 9 | `+0x24` | `uint32 GetData1()` | 0 | EAX | `0x40CE60` |
| 10 | `+0x28` | `SetData1(uint32)` | 4 | EAX = the argument | `0x8B6DE0` |
| 11 | `+0x2C` | `uint32 GetData2()` | 0 | EAX | `0x40CE70` |
| 12 | `+0x30` | `SetData2(uint32)` | 4 | EAX | `0x40CE80` |
| 13 | `+0x34` | `bool Clone(cISC4AdviceSubject** out)` (new subject, AddRef'd) | 4 | AL | `0x40D180` |

SDK errors (checked): slot 13 `Clone` is missing; `SetData1`/`SetData2` return the argument, not a bool (a
replacement should return true).

### 12.5 `cISCLua` (`cSCLua`, vtable `0x00AC4148`, 78 slots)

The slot order matches the SDK. Semantic differences (reported by the Lua analysis, for the Lua host spec):
`ToUserTag` (`+0x70`) returns the boxed pointer of a userdata, not a tag; `PushUserTag` (`+0x8C`) and `PushUserData`
(`+0x114`) are the same function `(void*, int32 tag)`; `ToUint32`/`ToSint32` are one function (truncate to int32);
`Register` (`+0x110`) takes `(fn, name)`.

---

## 13. Dependencies

Callee-pop and return channels are as in each subsystem's own slot table, checked again here for the calls this spec
adds. Only the interfaces not yet in `docs/abi` carry a vtable address.

| Interface | Off | Method | Pop | Return | Used by |
|---|---|---|---|---|---|
| `cISC4City` | `+0x58` | GetEstablished | 0 | AL | ticker, PostCityInit |
| | `+0x60` | GetDifficultyLevel | 0 | EAX | thresholds, `difficulty_level` |
| | `+0x110` | GetResidentialSimulator | 0 | EAX | Safety |
| | `+0x11C` | GetMySimAgentSimulator | 0 | EAX | CitySituations |
| | `+0x120` | GetDisasterLayer | 0 | EAX | Safety |
| | `+0x130` / `+0x134` | GetSeaportDeveloper / GetAirportDeveloper | 0 | EAX | CityPlanning, Traffic |
| | `+0x150`, `+0x164`, `+0x168`, `+0x174`, `+0x178` | GetTerrain, SizeX, SizeZ, cell counts | 0 | EAX / ST0 | CityPlanning, subjects |
| `cISC4Simulator` | `+0x14`, `+0x1C`, `+0x48` | start date, GetSimDate, IsAnyPaused | 0 | EAX / AL | ticker, Simulate |
| `cISC4AuraSimulator` | `+0x20`, `+0x24`, `+0x28` | GetMayorRating, GetMax/MinMayorRating | 0, 4, 4 | AL int8 | CityPlanning |
| `cISC4DemandSimulator` | `+0x18`, `+0x24`, `+0x28` | GetDemand, GetJobsBySensus, GetNeutralTaxRate | 8, 4, 0 | EAX / EAX / ST0 | CityPlanning |
| `cISC4Demand` | `+0x20`, `+0x24`, `+0x30`, `+0x68` | QuerySupplyValue, QueryDemandValue, QueryActiveDemandValue, GetDemandCap | 0 | ST0 / EAX ptr | CityPlanning |
| `cISC4RegionalCity` | `+0x24`, `+0x44`, `+0x98`, `+0x104`, `+0xD8`, `+0xE0` | GetPopulation(t), GetExtrapolatedPopulation(t), set utility advisor name *[inferred]*, GetTutorialGUID, GetExported(c), GetImported(c) | 4,4,4,0,4,4 | EAX / ST0 double | CityPlanning, Utility, Lua |
| `cISC4BudgetSimulator`, `cISC4DepartmentBudget` | §7.4, `+0x70`, `+0x28` | | | | Finance, HQ, Safety, Utility, CityPlanning |
| `cISC4NeighborsSimulator` | `+0x40`, `+0x44`, `+0x50`, `+0x54`, `+0x58`, `+0x64`, `+0x78`, `+0x80`, `+0x84` | | | | CityPlanning, Finance, Utility, Traffic |
| `cISC4PowerSimulator`, `cISC4PlumbingSimulator`, `cISC4PollutionSimulator`, `cISC4LandfillDeveloper` | §7.8 | | | | Utility |
| `cISC4ResidentialSimulator`, `cISC4CrimeSimulator`, `cISC4PoliceSimulator`, `cISC4FireProtectionSimulator`, flammability | §7.5, §7.6 | | | | HQ, Safety |
| `cISC4TrafficSimulator` | `+0x28`, `+0x88` | GetCongestionMap, GetTrafficStats | 0 | EAX | Traffic |
| `cISC4AirportDeveloper` / `cISC4SeaportDeveloper` (`0x00AB0714`) | `+0x14`, `+0x20`, `+0x24`, `+0x28`, `+0x34`, `+0x3C` | GetAvgEfficiency (hidden pointer), GetAllPortLots, GetNumPorts, AsISC4PortDeveloper, GetNumSeaportTrips, CityHasSea | 4,4,8,0,0,0 | | Traffic, CityPlanning |
| `cISC4CivicBuildingSimulator` | `+0x18`, `+0x1C` | GetConditionalBuildingStatus, UpdateConditionalBuildingStatus | | | rewards |
| `cISC4BuildingDevelopmentSimulator` | `+0x38`, `+0xA0` | GetBuildingKeyFromType, building type data | 8, — | AL | base DoMessage, rewards |
| `cISC4HistoryWarehouse` | `+0x14`, `+0x18`, `+0x20`, `+0x30` | count, trend, point, RecordHistoryData | | | Lua, Safety |
| `cISC4OrdinanceSimulator` | `+0x1C`, `+0x20`, `+0x34`, `+0x50` | | | | Lua |
| `cISC4View3DWin` | `+0xBC`, `+0xC0` | Assert/RetractGlobalEffectTrigger | 4 | | advice effects |
| `cIGZMessageServer2` | `+0xC`, `+0x10`, `+0x14`, `+0x18`, `+0x1C` | Send, Post, Add/RemoveNotification, post to target | | | everywhere |
| `cISCLua` | §12.5 | | | | everywhere |

The detailed rows of the subsystem calls are in §7 with their slots. Power `+0xA0` `GetChanceOfStrike` is used by
reading `[EAX]` after the call: a replacement power simulator must return the argument pointer in EAX (see the power
spec correction in the report).

**Response curves**: the advisor classes load none. **Grids**: they own none; Traffic reads the congestion map only
through `GetCellValue` (simgrid §6.4 applies to the traffic simulator, not to this system). **Shared helpers**
(AddRadiusEffect, iterators, flood fills): not used.

---

## 14. Interactions and replaceability

### 14.1 Consumers

| Consumer | Uses |
|---|---|
| City situation manager | `GetAdvisor(29)` → `GetAdviceByGUID`, `RetireAdvice`; `PushEvent`; city-situation advices |
| MySim dispatch (`EnterUDriveIt`), My Street sim, View3D vehicle menu | `GetAdvisor(29)` / `GetAdvisor(30)` |
| MySim (`cSC4MySim`) | `GetScriptingContext`, `GetDataTag`, `SetLocalValue`-style table writes |
| Tutorial system / registry | shares the Lua context (`GetScriptingContext`, `SetLua`) |
| Building development simulator, port developers | `PushEvent` / advisor `PushEvent` |
| Civic building simulator | written by reward advices (§6.6); it does not call the advisors |
| UI (advisor windows, popup tracker, View3D panels, HTML link handler) | window slots 23–28, `DoLink`, `GetAdvicePopupPriorityThreshold`, the advice messages |
| Lua scripts | `game.*` (§7, §8) |

### 14.2 Replaceability findings

- **Blocker: private global.** The system's constructor stores `this` in a global (`0x00B207C0`). About 60 call sites
  in the advisor classes, the advice classes, the neighbour-deal advice and the Lua extensions use that global, not
  the city getter. A replacement system class must either replace every advisor class, advice class and Lua extension
  as well, or write its own pointer there (a compatibility-table address, *[choice]* only in the shim).
- **Blocker: Lua.** Advices, the tutorial and MySim share the system's `cISCLua`. A replacement must provide a
  compatible Lua 4-style state (tags, `gc` tag methods, `game` table) before the original advice classes can run.
- The advisor map, the active maps and the vectors are reached only through the interfaces, but `GetAdvisors` and
  `GetCurrentAdvices` hand out **references to the original containers** (`std::map` of the original STL). A
  replacement that keeps those slots must lay out compatible containers.
- The subclasses rely on the class (non-interface) vtable order of `cSC4Advisor` (§12.2) and on its members
  (advice info vector, maps) through inherited non-virtual code. A replacement advisor must replace the whole
  hierarchy or nothing.
- Through GZCOM: every class is created by the class registry; the system is replaceable through it only with the two
  blockers above handled. Advisor and advice classes are replaceable individually through the script `class_id`
  fields (a new CLSID registered by a plugin) without touching the system.
- Nobody outside reads advisor or advice memory directly *[inferred: no raw offsets found outside the classes]*.

### 14.3 MySim touch points (for the MySim spec)

- CitySituations `UpdateMySimAdvices` (class slot 10) reads `cISC4City` `+0x11C` → MySim agent simulator `+0x30` →
  `+0x110`; it activates or deactivates its advices that are city-situation advices (`0x6BEB1E8A`) with situation flag
  `0x20` (MySim mission) by their trigger. Its caller is not identified (§16 Q5).
- Lua: `mysim_*` functions QI the context advisor to `0x0A1DA42F` and call MySim slots `+0xC8`, `+0xCC`, `+0xD0`,
  `+0xD4`, `+0x10C` (the compiled slots `+0xC4..+0xD4` are SDK + 4).
- Event `0xCA4E8986` (MySim building added): MySim stores the distance in metres in the subject's data1 for
  `event_building_distance`.
- The system creates the MySim advisors (ids 1–5, class `0x4A1DBBBF`) and My Street sim (30, `0x0BF606A5`) from the
  table, and MySim advices (`0x6A9335DE`, `0x6B70EFAD`). MySim, the My Street sim and the MySim dispatch call
  `GetAdvisor`, `GetScriptingContext`, `GetDataTag` and the subject functions. Nothing is posted to MySim by message.

---

## 15. Original quirks (reproduced)

1. Active advices lose their active state on load (§4.4) while expiry and nextDay are kept.
2. Event advices ignore `nextDay`, `once` and the monthly gate (§6.3).
3. The monthly step is tied to day 28 of the ticker's view of the date and can be skipped (§5.1).
4. Paused updates every 0.5 s (§5.1).
5. Base-class advisors are saved as `cSC4Advisor` (§11.6).
6. CityPlanning: parks and recreation count the same group; non-building occupant messages skip the base; airport
   and seaport subscriptions have no handler (§5.5).
7. HQ: life expectancy tract values repeat the HQ ones; two variables are uninitialised (§7.5).
8. Safety: `ga_crime` keeps the low byte; the crime hotspot falls back to the flammability hotspot (§7.6).
9. Traffic: avenue connection location not reset; port efficiency 100 with no ports; values written twice (§7.7).
10. Utility: "daily" capacities are monthly × 30; garbage import/export slots swapped (intended); rail counts as a
    garbage connection; `ProposeDeals` side effects and a random draw; the power strike handlers are dead (§7.8).
11. `get_reward_building_state` returns its argument for an unexpected status (§8.3).
12. The advice effect list is appended on Read and on Init (§4.5, §11.3).
13. CityPlanning Read accepts any version (§11.5); the advice Read does not check its version (§11.3).
14. Finance's pending flag has no reader (§5.5).

## 16. Open questions

- Q1. `cSC4AdvisorBase` registers a factory for `0xCA09F543` but reports `0xEAC937B4`; whether this was deliberate is
  unknown. Behaviour is specified as observed.
- Q2. reloadPending is cleared in two places and never set in the Windows build. FreeCity4 *[choice]*: keep the flag
  and set it from a host "reload scripts" command.
- Q3. The exact pump position of the ticker relative to the simulator tick (§5.6).
- Q4. The meaning of the data words of the strike messages and of event `0xCA3C0412`.
- Q5. Who calls CitySituations class slot 10 (`UpdateMySimAdvices`).
- Q6. Terrain `+0xD8` (`g_water_tile_count`): cells or vertices.
- Q7. `0x4A64A9CC` (building development event) has no script name.
- Q8. The Lua host (script packages, the resource hash, the protected-call wrapper, `print`) belongs in a separate
  spec.

---

## 17. Conformance tests (must exist in `engine/conformance`)

1. **Batch size**: n = 0 → 1; 1 → 1; 29 → 1; **30 → 2**; 59 → 2; **60 → 3**; 100 → 4; 250 → 9 (float32 rule).
2. **Evaluation gate**: advice with frequency 35, timeout 45, trigger true, created on day 1000 in month 5 (lastMonth
   4). Day pass on day 1000 → active, expiry 1045. Day 1045 → deactivated, nextDay 1080, AdviceRemoved sent. Day 1080
   in month 6 when lastMonth is 6 → not evaluated; after the next monthly step → evaluated and activated.
3. **Once**: the same with `once` → after deactivation nextDay = `0x7FFFFFFF`; never activates again.
4. **Persist / no_timeout**: persist advice whose trigger turns false stays active until expiry; no_timeout advice
   with trigger false and not persist is deactivated the same day; no_timeout and persist stays forever.
5. **Trigger values**: `true` → true; `1` → true; `0` → false; `0/0` (NaN) → true; `"1"` → false; `nil` → false; a
   Lua error → false. Forced trigger (`0x400`) → true. No-text flag → false.
6. **Event advice**: event advice (event `0xAA31A210`) inactive; `PushEvent(0xAA31A210, s)` → on the next day pass the
   advice gets subject s and activates if its trigger is true; a second push the next day re-activates it after a
   deactivation, regardless of nextDay.
7. **Activation messages**: activation posts AdviceAdded (IGZUnknown = advice); command 0x1234 also posts
   AdviceCommand (data1 0x1234). Deactivation sends AdviceRemoved synchronously.
8. **Ticker**: dates 27→28 of a month → monthly then daily step; variables `g_day` 28. Jump from day 27 to day 29 in
   one ticker → no monthly step that month. City not established → the ticker chain stops.
9. **Paused**: paused, same day, 600 ms after the last update → one daily step; 300 ms → none.
10. **Mayor rating**: aura rating −5, min −20 at cell (14, 22), max 40 at (3, 5) → `ga_mayor_rating` −5,
    `l_mayor_rating_l` −20 with subject (14.0, 22.0), `l_mayor_rating_h` 40 with subject (3.0, 5.0).
11. **Subject clamp**: city 64 × 64 cells, `SetLocation(70.0, −3.0)` → (64.0, 0.0).
12. **Reward condition**: condition returns 1 with status 0 → status 6, subject set, trigger true; returns 0 with
    status 6 → status 5; returns a string with status 6 → status 4 and the tooltip set.
13. **Persistence round trip**: advisor with 2 advices (one active, expiry 1045, lastMonth 5, nextDay 0) and one queued
    event → record bytes as §11.2; after Read + Init the active map is empty and the advice's expiry is 1045.
14. **Advice record**: version 3 record (no effect list) reads the subject right after `mood`.
15. **Subject record**: hasLocation 0 → 1 + 4 + 4 + ref; version 3 → Read false.
16. **System record**: two advisors IDs 23 and 26 → `u16 1, u32 2, ref, s32 23, ref, s32 26`.
17. **CityPlanning record**: counters 10, 2, 2, landmarks {0x1234 → 1} → version 3 record as §11.5; a version 2
    record keeps the current landmark map.
18. **random_chance**: with `mul` = 1 before the call: `mul` becomes `0x278DDE6D`, u = f32(0x278DDE6D/2^32 + 0.5) =
    0.65450847, r = f32(u × 100) = 65.450844 → `random_chance(70)` = 1, `random_chance(60)` = nil (foundations §7
    test 13 gives the same draw in double: 0.6545084968674928).
19. **Utility counts**: incinerator capacity 1000 → `g_incinerator_capacity_daily` 30000. Garbage: regional
    exported(3) = 5 → `g_garbage_imported` 5.
20. **HQ empty lists**: no hospitals → `l_hospital_funding_l` = 3.4028234e40 (double), subject null.
21. **Popup thresholds**: shipped table, difficulty 1 → 80; difficulty 7 → 60; missing table → 100.
22. **Lua functions**: `_combine_guids(0x0F0F0000, 0x00F0F0F0)` = 268431600 (`0x0FFFF0F0`);
    `_combine_guids(0x80000000, 0)` = −2147483648 (signed); one argument missing → −1;
    `trend_slope` with slope 1.0 → `f32(atan(1) × 63.6618f)` = 49.999863 (float32).
