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

- **`startTutorial`** (slot 5, `0x0043F080`): when there is no tutorial window, create one (100 bytes), take a
  reference and initialise it. Then is-tutorial := 1, end-pending := 0, and tell the registry that the GUID is
  active (`SetActive(GUID, true)`, `0x004B5D80`). AL is whatever the registry call left *[quirk]*.
- **`endTutorial(endRegistry)`** (slot 6, `0x0043F110`): when a window exists, close it, release it and clear
  +0x40. When `endRegistry`, tell the registry the GUID is inactive. Call the 3D view (global `0x00B43DC4`, when
  present) slots 60 and 59 (restore the camera controls *[inferred]*). Then is-tutorial := 0 and end-pending :=
  `!endRegistry`. AL = `endRegistry`.

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
object would still write is-tutorial.

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

- **TU1:** what the registry entry's byte +0x10 means (enabled, unlocked, completed?) and what `SetActive` changes.
  This belongs to a spec of the tutorial registry (`cISC4App` slot 49) with the advisors' Lua host.
- **TU2:** the 3D view slots 60 and 59 called by `endTutorial`.
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
