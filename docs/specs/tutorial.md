# Spec: the tutorial system (`cSC4TutorialSystem`), short form

Status: **behaviour spec**, clean-room input. This document describes *behaviour*, not code. It was written by the
analysis team from observing the original program and its data files. Implementers must work from this spec, the
public LGPL gzcom-dll SDK headers and general knowledge only. They must **not** consult decompiled or disassembled
code (see [CONTRIBUTING.md](../../CONTRIBUTING.md)).

Normative words and tags (*[compat]*, *[choice]*, *[inferred]*, *[quirk]*) as in [foundations.md](foundations.md).

Scope: what the **simulation and the save file** need from the tutorial system: its saved record (component 48),
the `IsTutorial` flag that suppresses fires, and the three messages it reacts to. The tutorial window, its Lua
scripts and its task objects are UI and are only outlined (§4). Related specs: [foundations.md](foundations.md)
(§3.6 component 48, §3.7 read before `Init`; §11 the region's `tutorialID`), [disasters.md](disasters.md) (§5.8 the
fire's no-burn-down flag), [flammability.md](flammability.md) (§6.8 the fire roll), [advisors.md](advisors.md)
(the shared Lua context and the tutorial registry), [city-situation.md](city-situation.md) (§7 the shipped tutorial
cities), [lua-host.md](lua-host.md).

## 1. Identity

| Item | Value |
|---|---|
| Class ID | `0xAA3836FB` |
| Component | city component **48**, IID `0xAA3836F6` (foundations §3.6); reached through `cISC4City` slot 82 (`+0x148`) |
| Constructor | `0x0043F5A0`: fields zero except GUID +0x24 = `0xFFFFFFFF` and +0x28 = 3 |
| Interface at +0 | `cISC4TutorialSystem` (SDK header), vtable `0x00A861B8` (21 slots, §6) |
| Message target at +4 | `DoMessage` `0x0043F1A0` |
| Cheat handler at +8 | `0xE98B2F57`, `DoMessage` `0x0043F870` |
| Serializable at +0x10 | `Write` `0x0043F260`, `Read` `0x0043F2B0` |

`QueryInterface` (`0x0043EFE0`): `0xAA3836F6` → +0, `cIGZMessageTarget2` `0x452294AA` → +4, `0xE98B2F57` → +8,
`cIGZSerializable` `0xE4FDA3D4` → +0x10; anything else goes to the base `cRZUnknown` lookup. `AddRef` and `Release`
are the shared ones of the advisor system (identical code): they work on a reference-count sub-object at +0x14
(vtable `0x00A86160`) whose count is at **+0x18**. The constructor also stores `this` in the global `0x00B20E88`.

**The last `Release`** (TU4; `0x00449310` → `0x0090D997`): a count above 1 is decremented. At 1 the sub-object's
slot 3 is called with 1 (`0x0043F6C0` → `0x0043F780`), the scalar deleting destructor, and the object is
**destroyed and freed** (0x44 bytes), not just marked. The destructor (`0x0043F6E0`): releases the tutorial window
(+0x40) **once**, without closing it (`0x0043F70F`); frees the buffer of the vector at +0x34; releases the Lua
context (+0x30); runs the base destructor of the +0x14 sub-object; and clears the global `0x00B20E88`. It does not
run `Shutdown`: no unsubscription, no cheat unregistration, no registry or is-tutorial change. A running tutorial
holds two references to its window (§4.2), so a window still open at this point is leaked with one reference
*[quirk]*. In the shipped game the city calls `Shutdown` before the last release, which closes the window first.

## 2. State

| Offset | Field | Saved |
|---|---|---|
| +0x1F | **is-tutorial** flag (u8) | yes |
| +0x20 | current task number (s32) | yes |
| +0x24 | tutorial GUID (u32; `0xFFFFFFFF` = none) | yes, mapped on read (§3) |
| +0x28 | 3 (constructor), not read anywhere in this class | no |
| +0x2C | "registry end pending" (u8) | no |
| +0x2D | "start already tried this session" (u8) | no |
| +0x30 | the shared Lua context, fetched lazily (slot 7) | no |
| +0x40 | the tutorial window (`cSC4TutorialUIWinProc`, 100 bytes), only while a tutorial runs | no |

## 3. Persistence *[compat]*

Record type `0xAA3836FB`, version **2**, 11 bytes:

| Field | Type | From |
|---|---|---|
| version | u16 = 2 | |
| is-tutorial | u8 | +0x1F |
| task number | u32 | +0x20 |
| tutorial GUID | u32 | +0x24 |

- **Write** (`0x0043F260`): when the stream already has an error, write nothing. Returns true when the stream has
  no error afterwards.
- **Read** (`0x0043F2B0`): read the version; **not 2 → false** (nothing else read). When the stream is then
  error-free, read the three fields into +0x1F, +0x20, +0x24, then replace the GUID by the **tutorial registry's ID
  mapping** (registry = `cISC4App` slot 49, `+0xC4`; mapping `0x004B5C20`):
  `0 → 0x8A5B7A6C`, `1 → 0xEA5D6DC8`, any other value unchanged. Return true when the stream has no error.
  The region's `tutorialID` (foundations §11, `cSC4RegionalCity::Read` at `0x004AC7F0`) goes through the same
  mapping.

Read happens before `Init` (foundations §3.7), so a loaded tutorial city has is-tutorial set from the moment its
simulators start.

**Sample saves.** 421 distinct saves: 416 hold `02 00 | 00 | 00 00 00 00 | FF FF FF FF` (not a tutorial, task 0,
no GUID). The five shipped tutorial cities hold is-tutorial 1, task 0 and the GUIDs `0xCC02D3A3`, `0xABFAD020`,
`0x0BFDB71B`, `0x2C12D0E9` and `1` (the last is read as `0xEA5D6DC8`).

## 4. Behaviour

### 4.1 `IsTutorial` (slot 8, `0x0043F650`) and the fire *[compat]*

Returns EAX = 1 when is-tutorial is non-zero, else 0. That is the only thing the simulation reads:

- the fire simulator copies it each fire step into its **no-burn-down** flag (disasters.md §5.8), so burning
  buildings in a tutorial city are not destroyed;
- the flammability simulator skips its end-of-cycle fire roll, **including the random draw**, while it is true
  (flammability.md §6.8);
- the advisors suppress one class of advice while it is true (advisors.md, condition d).

Is-tutorial is set by `startTutorial` and by `Read`, and cleared by `endTutorial`. Nothing else writes it.

### 4.2 Starting and ending

- **`startTutorial`** (slot 5, `0x0043F080`): when there is no tutorial window, create one (100 bytes,
  constructor `0x00442920`), take **two** references and initialise it. The two `AddRef`s are the one in the
  "replace the old window" step and the one just before `Init`; with no old window to release, the count ends at
  2. Then is-tutorial := 1, end-pending := 0, and tell the registry that the GUID is active
  (`SetActive(GUID, true)`, `0x004B5D80`, through `cISC4App` slot 49 on the global `0x00B43C94`). AL is whatever
  the registry call left *[quirk]*.
- **`endTutorial(endRegistry)`** (slot 6, `0x0043F110`): when a window exists, close it (`0x00442C80`), release
  it, clear `+0x40` and release it **again** — the same two-reference teardown as `Shutdown` (§4.3), because
  `startTutorial` took two references (§4.2 above). When `endRegistry`, tell the registry the GUID is inactive
  (`SetActive(GUID, 0)`, `0x004B5D80`). Call the 3D view (global `0x00B43DC4`, when present) **slot 60 then
  slot 59**, which close any open secondary and tertiary **catalog menu** (TU2). Then is-tutorial := 0 and
  end-pending := `!endRegistry`. AL = `endRegistry`.

  The 3D view calls are not about the camera. The class is `cSC4View3DWin` (interface `cISC4View3DWin`, vtable
  `0x00ABCE40`):
  - **slot 60 `CloseTertiaryMenu` (`0x007E5E90`)** — if the tertiary-menu window (`view + 0x1FC`) is present,
    write its two-dword catalog state back through `view + 0x208` when that pointer is set, then close and
    destroy it, and clear `+0x1FC` and `+0x204`.
  - **slot 59 `CloseSecondaryMenu` (`0x007E5D20`)** — if the secondary-menu window (`view + 0x1F8`) is present,
    **calls slot 60 first**, then closes and destroys it, and clears `+0x1F8` and `+0x200`.

  So a tutorial that ends mid-tool does not leave a detached catalog popup open after its window is gone. The
  counterparts that *open* those menus are `InvokeSecondaryMenu` (`0x007E5C10`) and `InvokeTertiaryMenu`
  (`0x007E5D80`), called from the 3D view's own catalog/tool-button handling. `endTutorial` does **not** restore
  the zoom (a task sets it through slot 8 `SetZoom`), does not re-add view input controls (a task removes the
  current one through slot 26), and does not clear the window callback the tutorial registered.

#### 4.2.1 Native registry boundary (TU6)

Both calls below are `__thiscall`. ECX is exactly the **borrowed result of `cISC4App` slot 49** on the
application pointer at `0x00B43C94`. Do not add `0x1C0` to that result: the getter already returns
`this+0x1C0` (`0x0044B510`–`0x0044B516`). The registry is not a COM interface and has no AddRef/Release.

| Entry | Stack arguments in declaration order | Callee pops | Return and lifetime |
|---|---|---|---|
| `GetItem`, `0x004B5D10` | GUID: one 32-bit value, not a pointer | 4 bytes | EAX = matching item pointer, or null; borrowed interior pointer into the registry's list node |
| `SetActive`, `0x004B5D80` | GUID: one 32-bit value; active: one four-byte slot, only the low byte read (normal calls pass 0 or 1) | 8 bytes | AL = 1 if a known item's byte changed; AL = 0 if absent or unchanged. Upper EAX bits are not a Boolean result |

`GetItem` scans the registered groups and their item lists, comparing the whole GUID dword; its successful
result is node+8 (`0x004B5D60`–`0x004B5D7C`). The item owns its contents; callers neither free nor release the
pointer. Read the active byte at result+0x10 before any registry teardown/repopulation. No independent lifetime
or synchronization is acquired. The city-init lookup consumes it immediately.

`SetActive` first looks up the item. On a change it writes the **supplied low byte verbatim** to item+0x10
(`0x004B5D90`–`0x004B5D99`), then obtains preferences through app slot 38. It scans the 256 dwords starting at
prefs+0xF34. A nonzero active byte removes the **first matching GUID** by zeroing its cell; zero active inserts
the GUID into the **first zero cell**. Only a cell modification calls app slot 17 (`SavePreferences`), synchronously.
Even if the scan finds no matching/free cell, the item byte has already changed and AL is still 1
(`0x004B5DB1`–`0x004B5E02`). For example, disabling a known active item with a full preferences array changes
the in-memory byte and returns true, but neither saves preferences nor creates a durable disabled entry.

The four exported call sites are tutorial start (`0x0043F0FE`), tutorial end (`0x0043F159`), nation tutorial-city
reset (`0x00499B99`) and regional tutorial-city reset (`0x004A5599`). The function itself
has no lock, thread handoff or deferred-save protocol; the item write, preferences edit and possible save happen
in that order on the calling thread. This establishes the contract for calls in the original tutorial/city
message flow, not a guarantee that concurrent registry mutation or saving on another thread is safe. A replacement
using the native registry must preserve that execution context. Start's AL is consequently this changed-state
result, rather than an unconditional success flag.

### 4.3 Messages (`DoMessage`, sub-object +4; every case returns true)

| Message | Handling |
|---|---|
| `0xEA8AE29A` (city init complete) | only the first time per session (+0x2D clear) and only when is-tutorial is set: look the GUID up in the registry (`0x004B5D10`); when the entry exists and its byte +0x10 is set, `startTutorial`. In all cases set +0x2D. So a loaded tutorial city reopens its window; a tutorial whose registry entry is not enabled keeps is-tutorial set without a window |
| `0xAA3840E6` (`Tutorial_EndTutorial`) | only with a window: `endTutorial(message data == 0)` (message slot 9) |
| `0x26C63343` (pre-save) | when end-pending is set, `endTutorial(true)`, which finishes a tutorial that was ended without its registry entry before the city is saved |

`Init` (slot 3, `0x0043F7A0`) subscribes to the three messages (message server slot 5) and, when the cheat manager
(`0x00B43CAC`) exists and `cISC4App` slot 21 (`+0x54`) is true, registers the cheat handler and the cheat
`0x6A382BAF` "Tutorial" (debug subcommands reset, save, step, next, quit, handled by `0x0043F870` on message
`0x230E27AC`). `Shutdown` (slot 4, `0x0043F330`) undoes both. Both return true; there is no initialised flag.

**`Shutdown` in detail** (TU5), in this order:

1. When the cheat manager (`0x00B43CAC`) exists, **whether or not `Init` registered anything** (the `cISC4App`
   slot 21 test is not repeated): cheat manager slot 6 with (the cheat handler +8, 0), which removes the handler's
   notification, then slot 4 with `0x6A382BAF`, which unregisters the cheat.
2. When a tutorial window exists (+0x40): close it (`0x00442C80`), release it, then clear +0x40 and release it
   again (the two references `startTutorial` took, §4.2). This is the same teardown as `endTutorial`, without its
   registry, 3D view and flag steps.
3. When the message server (`0x00B43CCC`) exists: unsubscribe (server slot 6) `0xAA3840E6`, `0xEA8AE29A` and
   `0x26C63343`, in that order.

Is-tutorial, end-pending, the task number, the GUID and the registry are not touched, so a later save of the same
object would still write is-tutorial. Neither are `+0x2D`, the Lua context at `+0x30`, the vector at `+0x34` or
the global `0x00B20E88`.

**Verified complete (re-answers29, 2026-10-16).** The three steps above match `0x0043F330` exactly, in that order,
with the two details an implementer must not "tidy up":
- the window teardown is **close, release, clear `+0x40`, release again** — the two releases are the two
  references `startTutorial` took (§4.2), so a replacement that releases once leaks one reference;
- the cheat unregistration runs whenever the cheat manager exists, **without** repeating the `cISC4App` slot 21
  test that `Init` uses to decide whether to register. So `Shutdown` can unregister a cheat that was never
  registered.

`Shutdown` returns true and has no initialised flag, so calling it twice repeats all three steps. Test 7 covers
the happy path; an apply lane can implement §4.3 as written.

### 4.4 Other slots (UI)

Each forwards to the tutorial window when there is one:

| Slot | Without a window | With a window |
|---|---|---|
| 7 `GetScriptingContext` | when +0x30 is null, fetch the Lua context from the city (`cISC4City` slot 81, then its slot 10), keep a reference; return +0x30 | same |
| 9 `IsEdgeScrollingAllowed` | true | the current task's answer (window slot 9, then task slot 25); true when there is no task |
| 10 `GetCurrentTask` | null | the window's current task (window slot 9) |
| 13 `FilterMessage(msg)` | true | for a message of kind 3 with ID `0x287259F6` or `0x287259F7`, pass the message's data field 2 to window slot 6; for ID `0x49D993D4`, its field 3. True |
| 14 `ShowUIHintsForUsers` | nothing | window slot 7 |
| 15 `HandleCatalogMenu(menu)` | true | window slot 8, true |
| 16 `FilterLeftMouseClick(x, z, n)` | true | window slot 10's result |
| 17 `ResetTutorialNumAndIndex(a, b)` | false | GUID := `a`, task number := window slot 11 `(a, b)`; true |
| 18 (not in the SDK) `OnTutorialEvent(e)` | true | passes `e` to the window (`0x00443970`); true. `e` is a 32-bit event code, not a pointer: the window compares the whole dword with 1 (`0x00443989`) and acts only on 1 (if the current task's slot 20 returns 1, it calls the task's slot 15, then refreshes the instruction text from task slot 9); any other value does nothing (TU3) |
| 19 (not in the SDK) `SetTutorialTaskNumber(n)` | task number := `n` (the whole dword into the s32 at +0x20, the field slot 11 returns) | same |
| 20 (not in the SDK) `IncrementTutorialTaskNumber()` | task number += 1 | same |

Slots 11 and 12 return the task number and the GUID. The SDK header stops at slot 17.

## 5. Quirks

1. `Read` maps saved GUIDs 0 and 1 to two fixed GUIDs; a save that stores 1 reloads as `0xEA5D6DC8`, and saving it
   again writes `0xEA5D6DC8`.
2. A tutorial city whose registry entry is disabled stays "in tutorial" (no fires) without any window, until
   something calls `endTutorial`.
3. `startTutorial`'s AL is not a defined result.
4. `Init` and `Shutdown` have no guard: a second `Init` subscribes again.

## 6. Interface slot table (`cISC4TutorialSystem`, vtable `0x00A861B8`) *[compat]*

| Slot | Offset | Method | Pop | Return | Address |
|---|---|---|---|---|---|
| 0 | +0x00 | `QueryInterface` | 8 | AL | `0x0043EFE0` |
| 1 | +0x04 | `AddRef` | 0 | EAX | `0x004186E0` (shared) |
| 2 | +0x08 | `Release` | 0 | EAX | `0x00449310` (shared) |
| 3 | +0x0C | `Init` | 0 | AL = 1 | `0x0043F7A0` |
| 4 | +0x10 | `Shutdown` | 0 | AL = 1 | `0x0043F330` |
| 5 | +0x14 | `startTutorial` | 0 | — | `0x0043F080` |
| 6 | +0x18 | `endTutorial(bool endRegistry)` | 4 | AL = argument | `0x0043F110` |
| 7 | +0x1C | `GetScriptingContext` | 0 | EAX | `0x0043F490` |
| 8 | +0x20 | `IsTutorial` | 0 | EAX 0/1 | `0x0043F650` |
| 9 | +0x24 | `IsEdgeScrollingAllowed` | 0 | AL | `0x0043F500` |
| 10 | +0x28 | `GetCurrentTask` | 0 | EAX | `0x0043F6D0` |
| 11 | +0x2C | `GetTutorialTaskNumber` | 0 | EAX | `0x004ED900` (shared getter of +0x20) |
| 12 | +0x30 | `GetTutorialGUID` | 0 | EAX | `0x006BDD80` (shared getter of +0x24) |
| 13 | +0x34 | `FilterMessage(msg)` | 4 | AL = 1 | `0x0043F3C0` |
| 14 | +0x38 | `ShowUIHintsForUsers` | 0 | — | `0x0043F440` |
| 15 | +0x3C | `HandleCatalogMenu(menu)` | 4 | AL = 1 | `0x0043F470` |
| 16 | +0x40 | `FilterLeftMouseClick(x&, z&, n)` | 12 | AL | `0x0043F450` |
| 17 | +0x44 | `ResetTutorialNumAndIndex(a, b)` | 8 | AL | `0x0043F410` |
| 18 | +0x48 | `OnTutorialEvent(e)` | 4 | AL = 1 | `0x0043F4E0` |
| 19 | +0x4C | `SetTutorialTaskNumber(n)` | 4 | — | `0x006CE100` (shared setter of +0x20) |
| 20 | +0x50 | `IncrementTutorialTaskNumber()` | 0 | — | `0x0043F660` |

Slots 18–20 are not in the SDK header; the names are ours. Argument types (TU3): slot 18's `e` is a `uint32_t`
event code (only 1 does anything); slot 19's `n` is the task number, the same s32 field that slot 11 returns as
`int32_t`, so `int32_t` is the consistent declaration (`uint32_t` passes the same dword; neither extends or tests a
sign). Proposed `docs/abi` rows (adding 18–20 and correcting
the shared-getter names of 11, 12): `build-runs/re-specs13d-abi-rows.tsv`.

## 7. Open questions

- ~~**TU1:** what the registry entry's byte +0x10 means (enabled, unlocked, completed?) and what `SetActive` changes.
  This belongs to a spec of the tutorial registry (`cISC4App` slot 49) with the advisors' Lua host.~~
  **ANSWERED 2026-10-16 (re-answers29). PARTLY against the three guesses: `+0x10` is `active`, meaning "this
  tutorial is still due to run", so it is closest to *not completed* rather than "enabled" or "unlocked".**

  **The registry** is `cSC4TutorialRegistry`, a **plain C++ object** (no vtable; every method is a direct call),
  **embedded by value in `cSC4App` at `+0x1C0`** and reached through `cISC4App` slot 49 (`0x0044B510`, which is
  just `lea eax,[ecx+0x1C0]; ret`). Note: `decomp/index.tsv` mislabels `0x0044B670` as `GetTutorialRegistry`;
  that address is `cSC4App::GetLuaInterpreter` (slot 47, `app+0x658`). Use `0x0044B510`.

  **A registry entry** is `cItem`, 0x28 bytes, held in a 0x30-byte circular doubly-linked list node
  (`_Next`, `_Prev`, then the `cItem`) inside a `std::map` keyed by region directory name. The lookup key is the
  GUID and is **global** — `GetItem` walks every region's list and ignores the region.

  | Offset | Name | Meaning |
  |---|---|---|
  | `+0x00` | `guid` | the tutorial GUID; the `long` of `GetItem` / `SetActive` |
  | `+0x04` | `order` | sort key; the list is kept ascending by it (Lua key `order`) |
  | `+0x08` | `task_first` | 1-based index of this tutorial's first task in the window's task vector (Lua `task_first`) |
  | `+0x0C` | `task_count` | how many tasks it owns (Lua `task_count`) |
  | `+0x10` | `active` | **the byte in question** (see below) |
  | `+0x14` | `region_dir` | a `cRZString` (0x14 bytes), the region directory name, lowercased and prefixed (Lua `region_dir`) |
  | `+0x24` | — | string internal, zeroed by the copy constructor, never read |

  **`+0x10` is `active`.** Set (1) = still due to run; clear (0) = completed / dismissed / not to be
  auto-started. It is initialised to 1 for every entry built from Lua, and read in four places: `SetActive`,
  the city-init message handler (which auto-`startTutorial`s only when it is non-zero), `GetNextActive` (which
  only returns entries with it set), and the region view's `filter_tutorial_mayor_name` / `RenderTile` (which
  clear a tutorial city's mayor name while it is set).

  **What `SetActive(guid, active)` (`0x004B5D80`) changes — two things, not one:**
  1. the entry's `active` byte, written only when it differs (returns 0 if the GUID is unknown or already in
     the requested state);
  2. a **persisted 256-entry `uint32` GUID array in the application preferences block** (`prefs+0xF34`, reached
     through `cISC4App` slot 38 `GetPreferences`): `active = false` **adds** the GUID to the first free slot,
     `active = true` **removes** it (zeroes the matching slot). Then `cISC4App` slot 17 `SavePreferences`
     (`0x0044BDB0`) is called.

  The prefs array is the durable record; the in-memory byte is **re-derived from it at every registry `Init`**
  (`0x004B7720`), which forces `active = 0` for each listed GUID. The array is exactly adjacent to a 64-entry
  one at `prefs+0x1334` that `cSC4CitySituationManager::ResetTutorials` (`0x005266E0`) manages the same way.

  `SetActive` is called by `startTutorial` (`(guid, 1)`), `endTutorial` (`(guid, 0)`, only when
  `endRegistry`), `cSC4Region::ResetTutorialCity` (`(guid, 1)`, re-enabling a tutorial when its city is
  rebuilt), and `cSC4Nation::ResetTutorialCities` (call at `0x00499B99`). The reset callers are outside this class.

  **Registration and task lookup.** The registry is populated from Lua: `cSC4AdvisorSystem::reload_data`
  (`0x0041A4F0`) calls `SetLua` (`0x004B7D70`) with the shared context, which runs `Init`. `Init` loads the
  resource script **`tutorial_registry.lua`** and requires a global table `tutorial_registry`, then reads the
  tables `tutorials`, `default_regions` and `regions` and the keys `order`, `task_first`, `task_count` and
  `region_dir`. The tutorial GUID is the **integer key of the `tutorials` table** *[inferred]*. Tasks are **not**
  in the registry: it stores only the `task_first` / `task_count` slice. The window builds the actual
  `cSC4TutorialTask` objects (0xFC bytes each) from a Lua task script into its own vector, and the index formula
  used everywhere is

  > window vector index = `taskNumber + (task_first − 1)`, valid while
  > `index < task_first − 1 + task_count`.

  `ResetTutorialNumAndIndex` (slot 17) returns `index − (task_first − 1)` — the relative task number — and the
  tutorial system stores that in `+0x20`. `moveToNext` increments `+0x20` (slot 20) and recomputes.

  **Who sets is-tutorial (`+0x1F`) — confirmed, with the constructor added.** Four writers, all inside
  `cSC4TutorialSystem`, and nothing outside the class writes it (`cISC4TutorialSystem` exposes no setter):

  | Writer | Value | Address |
  |---|---|---|
  | constructor | 0 | `0x0043F5E2` |
  | `startTutorial` | 1 (only when currently 0) | `0x0043F0D9` |
  | `endTutorial` | 0 (unconditional) | `0x0043F186` |
  | `Read` | the saved byte | `0x0043F2E0` (through the `cIGZSerializable` sub-object at `+0x10`) |

  Still open: `cItem+0x24`, and `GetNextActive`'s "next" semantics when tutorials are completed out of
  order. The Lua shape is **settled** (below), since `tutorial_registry.lua` is readable in the shipped scripts.

  **`tutorial_registry.lua` (the shipped script, 2026-10-16).** The global is a table with `default_regions`
  and `tutorials`, and three methods:

  | Method | Effect |
  |---|---|
  | `add_tutorial(self, guid, order, region_dir)` | `tutorials[guid] = { order, region_dir, task_first = 0, task_count = 0, task_guid = guid, set_task_range }` |
  | `add_default_region(self, exp_pack_id, region_dir)` | `default_regions[exp_pack_id] = { region_dir }` |
  | `set_task_range_and_print_info(guid, task_first, task_count)` | looks the tutorial up with `hex2dec(guid)` and sets its `task_first` / `task_count` |

  So the **GUID is the `tutorials` table key**, passed as a hex string and converted with `tonumber(s, 16)`.
  That confirms the `[inferred]` above. `task_first` / `task_count` start at 0 and are filled later by the task
  scripts calling `set_task_range_and_print_info`. The registered entries in the shipped file are exactly the
  GUIDs the saves show:

  | GUID | `order` | `region_dir` | Comment in the file |
  |---|---|---|---|
  | `0x8A5B7A6C` | 0 | `maxisland` | the old Mayor Tutorial (commented out) |
  | `0xABFAD020` | 0 | `timbuktu` | Getting Started |
  | `0xEA5D6DC8` | 1 | `timbuktu` | Terraforming |
  | `0x0BFDB71B` | 2 | `timbuktu` | Making Money |
  | `0xCC02D3A3` | 3 | `timbuktu` | Big City |

  `add_default_region(0, 'timbuktu')` and `add_default_region(1, 'timbuktu')` cover the main build. Note the
  save-side GUID mapping of §3 (`0 → 0x8A5B7A6C`, `1 → 0xEA5D6DC8`) is a *different* thing: it maps the two
  legacy IDs a save may store onto these registry GUIDs.

- ~~**TU2:** the 3D view slots 60 and 59 called by `endTutorial`.~~ **ANSWERED 2026-10-16 (re-answers29).
  DIFFERS from the spec's guess: they are not "restore the camera controls".** They close the 3D view's
  **secondary and tertiary catalog menus**. Full behaviour is in §4.2; in short, `cSC4View3DWin` (vtable
  `0x00ABCE40`) slot 60 `CloseTertiaryMenu` (`0x007E5E90`) and slot 59 `CloseSecondaryMenu` (`0x007E5D20`), the
  latter calling the former first. What a tutorial changes on the 3D view is the zoom (slot 8 `SetZoom`, set per
  task) and the current view input control (slot 26, removed per task); `endTutorial` undoes neither.
- ~~**TU3:** (abitidy) slots 18 and 19 have one dword each. What is `OnTutorialEvent`'s `e` (an event ID, a message
  ID or a pointer)? Is `SetTutorialTaskNumber`'s `n` signed? [both declared `uint32_t`]~~ **ANSWERED 2026-10-15
  (re-answers15):** MATCHES for `e` and for the ABI of `n`. Slot 18 (`0x0043F4E0`) passes the dword to the window's
  `0x00443970`, which compares the whole dword with 1 (`cmp` at `0x00443989`) and does nothing for any other value,
  so `e` is an event code, not a pointer and not a message ID. Slot 19 (`0x006CE100`, `ret 4`) stores the whole
  dword into the task number at +0x20, which slot 11 returns and slot 20 increments: it is the s32 task number.
  `uint32_t` passes the same bits; `int32_t` would match slot 11 (§4.4, §6). The Mac symbol table names both
  methods but gives no parameter types, and no caller of slots 18–19 was established statically.

## 8. Conformance tests

1. **Plain record.** Bytes `02 00 00 00 00 00 00 FF FF FF FF` → is-tutorial false, task 0, GUID `0xFFFFFFFF`;
   `IsTutorial` = 0; writing gives the same 11 bytes.
2. **Tutorial record.** `02 00 01 00 00 00 00 A3 D3 02 CC` → `IsTutorial` = 1, GUID `0xCC02D3A3`. A fire in this
   city leaves its building standing (disasters.md §5.8), and the flammability end of cycle draws no random number.
3. **GUID mapping.** `02 00 01 00 00 00 00 01 00 00 00` → GUID `0xEA5D6DC8`; `… 00 00 00 00` → `0x8A5B7A6C`;
   `… 02 00 00 00` → 2.
4. **Version.** `03 00 …` → `Read` false, fields unchanged (GUID stays `0xFFFFFFFF` on a fresh instance).
5. **End without registry, then save.** Tutorial running: `endTutorial(false)` → `IsTutorial` = 0, end-pending set,
   registry still active; pre-save message → `endTutorial(true)`: registry inactive, end-pending clear.
6. **Task counter.** Slot 19(4), slot 20, slot 20 → slot 11 = 6; the next record stores task 6.
7. **Shutdown** (TU5). Tutorial running with a window, cheat manager present: `Shutdown` closes the window, drops
   both references and clears +0x40, unregisters the cheat and its handler, unsubscribes the three messages, and
   leaves `IsTutorial` = 1. A message `0x26C63343` posted afterwards does nothing.
