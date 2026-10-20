# protection: answered questions (archive)
<!-- Archive for [protection.md](../../protection.md). Lanes do not read this file. Open questions are in ../protection.md. -->

## From the protection glue and grid shadow (filed by the coordinator from the lane's report, 2026-10-14; merged)

About `docs/specs/protection.md` (fire and police) and its `docs/abi` rows. The glue's current answer is in brackets.

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR1 — §5 dispatch record:** "callID" is taken as the record's `+0x08` field (vehicle slot) and "active" as the
  `+0x10` flag (state 5). [inferred]~~ **ANSWERED 2026-10-14** from the exe (`GetDispatchInfo` `0x00469D40`,
  `InitDispatch` `0x00469E50`, the active setter `0x00469BE0`, `EndDispatch` `0x0046ACE0`; `UpdateUnadjusted`
  `0x006F5D94`; end handler `0x006F5A40`) and written into protection.md §5. **Confirmed; the bracket matches.**
  - **`+0x08`** is the unit's call ID (`base + i`), from the source's dispatch call or the pylon's
    `GetCallNumber`.
  - **`+0x10`** is a byte set to (dispatch state == 5) by `0x00469BE0`. It posts active or inactive on a change,
    and `EndDispatch` clears it.
  - **Also.** The record is 44 bytes, copied with no AddRef. `+0x14`–`+0x1C` is the position and `+0x20` the
    source station. `EndDispatch` already revokes the unit, so the simulator's own revoke on the end message fails
    harmlessly.

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR2:** the dispatch automaton is handed to the core by a `cISC4Occupant` (`0x26BB767E`) QueryInterface, treated
  as borrowed. Right identity? [as stated]~~ **ANSWERED 2026-10-14** from the exe (`UpdateUnadjusted` `0x006F5DB1`;
  `InitDispatch` `0x00469E50`) and the shipped exemplars, and written into protection.md §7.2 and §5.1.
  **Borrowed is right; "automaton" is the wrong identity.** The occupant passed to `GetDispatchEffect` is the
  record's `+0x20`, the **source station** the unit left from, already a `cISC4Occupant*`. The original does no QI
  and no AddRef. The dispatch effect properties (`0x29233E9A–C`, `0x48D71ED9–DB`) exist only on the shipped station
  exemplars. A QI for `0x26BB767E` returns the same object plus a reference, which the glue must release.

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR3:** `AddControllersFromOccupantGroup(name, occ, 0)` and its Remove: the name as an `SC4String` gives slots 10
  and 12. Right overload? [yes]~~ **ANSWERED 2026-10-14** from the exe (call sites `0x006ADD5E`/`0x006ADD65`,
  `0x006E19BD`/`0x006E19C4`, `0x006E1C46`; slots 10/12 `0x00517150`/`0x0051AB00`; group lookup `0x0051CFC0` →
  `0x0051C9F0`) and written into protection.md §6.3 and §14.
  - **The overload: yes, the bracket matches.** Slots 10 and 12 are the by-name overloads. Slots 9 and 11 take a
    `u32` group ID.
  - **Caveat: the string layout.** The lookup reads the argument's string directly as the game's `cRZString`
    layout: begin pointer at `+0x04`, end at `+0x08`. It makes no virtual call. An `SC4String` works only if it
    has exactly that layout. A class with an inline small-string buffer at `+0x04` makes the game dereference
    text as an address and crash.
  - **How to call it.** Build the name with the game's own `cRZString(const char*)` (`0x00408480`) and its
    destructor, or with a layout-identical struct, as the original does.
  - **Testing.** The path runs only when a station starts or ends a strike, or on a jailbreak start, so a quiet
    test city may never reach it.
  - **The glue is safe.** The coordinator checked the glue: its `SC4String` has exactly this layout (begin, end,
    capacity, refcount; 0x14 bytes; always a heap buffer). So there is no crash risk. The layout goes into
    `docs/abi/layouts.md`.

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR4:** the fire simulator getter is city slot 57, inferred from the SDK order; the shadow depends on it. §14 lists
  only police slot 65. [57]~~ **ANSWERED 2026-10-14** from the exe (slot 57 body `0x006DC690`; city new-city path
  `0x0045ADF7`; city `Read` `0x0045CEDB`; cache fill `0x00601E8A` → `0x00B43D78`) and written into protection.md §1.
  **Confirmed; the bracket matches.** Slot 57 (`+0xE4`) returns city `+0x174`. The city creates that member as the
  fire class `0x2990C05A` with IID `0x88F54C7B`, and loads it from component ref #25. The neighbouring getters follow
  the component order: slot 55 crime `+0x16C`, 56 demand, 57 fire, 58 flammability, 59 flora, 60 industrial, 61
  land value.

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR5:** is there a read-only way to get the original's unadjusted int16 grid? [no; only the adjusted grid is
  compared]~~ **ANSWERED 2026-10-14** from the exe (slot 5 `0x006EFA50`; fire `Init` `0x006AE304`, police `Init`
  `0x006E28A0`; `ProtectionInit` `0x006F6C18`–`0x006F6CDE`) and written into protection.md §2. **No interface returns
  it, but there is a read-only way.**
  - **Where it lives.** Both original classes keep the unadjusted `int16` grid pointer at primary-interface
    offset `+0x84`, next to the adjusted one at `+0x80`.
  - **How to read it.** A shadow that knows it is facing the original class can read that pointer, borrowed, and
    use the grid's read-only slots. Each save also holds the grid as its own record.
  - **Spec correction.** The original builds both grids for a new city directly (global heap, inlined
    constructors), not through GZCOM.

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR6:** city slot 97: interfaces.tsv says ST0, §14 lists only its outputs. [outputs used, ST0 discarded]~~
  **ANSWERED 2026-10-14** from the exe (body `0x00459440`; constant `0x00A84D2C` = 0.5; cell size at city
  `+0x108`/`+0x10C` = 16.0, set at `0x0045A13C`; all 83 indirect `+0x184` call sites) and written into protection.md
  §14. **The bracket matches in effect, but there is no ST0 to discard.**
  - **No return value.** The function returns nothing: both `FSTP`s pop, so the x87 stack is empty at `RET 0x10`.
    The `x87` return in interfaces.tsv is wrong, and no caller reads ST0.
  - **Arithmetic.** `px = (x + 0.5) × 16`, `pz = (z + 0.5) × 16`. Both steps are exact; for example, cell (10, 20)
    gives (168.0, 328.0).
  - *[choice]* Declare it `void`. A float return would pop an empty register, giving NaN and setting the
    invalid-operation flag.
  - **Slot 96** (`0x00459410`, the corner) is the same without the `+ 0.5`.

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR7:** which slots does the game call from its second thread, and are these fallbacks acceptable: lists return 0
  untouched; chance of strike, efficiency and radius return 0; strike length 0xFFFFFFFF with no date, else 0; scalars
  from the month-end snapshot? [as stated]~~ **ANSWERED 2026-10-14** from the exe (all 79 reads of the fire and police
  globals `0x00B43D78`/`0x00B43D98`, by containing function; the ambience condition evaluators
  `0x004E2AA0`–`0x004E2CB0`) and written into protection.md §13.1. **None is known; no original behaviour; the
  implementer's choice stands.** Every reader of the protection globals is main-thread code: simulator agents, view
  input controls, UI windows, automata, the disaster layer, the advisors. The measured off-main caller (`IsPaused`,
  foundations §19.2) fits the ambience sound system, whose conditions read only the simulator and view globals. The
  original returns live values with no locking. If the fallbacks ever fire, they differ from it. A `Mutex` that
  answers exactly from any thread would be closer than fallbacks. *[inferred: no thread trace; static reachability
  through the globals only]*

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR8:** slots 21/22/32, dispatch-source slot 6 and slot 42 (non-empty) clear the caller's list without releasing
  its elements, which leaks the caller's references as the original does. Intended? [reproduced]~~ **ANSWERED
  2026-10-14** from the exe (shared helper `0x006DFC70`; list clear `0x00413E60`; every original caller) and written
  into protection.md §15.4 and §16 quirk 7. **Confirmed; the bracket matches.**
  - **What the helper does.** It clears the list with the plain list clear, which frees the nodes and never calls
    `Release`. Then it appends the simulator's list, calling `AddRef` on each element.
  - **Original callers are not affected.** Every one passes a fresh empty list and calls `ReleaseOccupantList`
    afterwards: `0x00416189`, `0x004164B1`, `0x007A1A8E`, `0x007A1AF0`, and the dispatch manager at `0x0046AFA2` and
    `0x0046B116`. So the leak happens only for a plug-in caller that passes a non-empty list.
  - *[choice]* Reproduce it. Releasing instead would over-release a list of borrowed pointers.

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR9:** the property setters `0x005FE3C0` / `0x005FE6B0`: scalar u32 or a one-element vector? [open]~~
  **ANSWERED 2026-10-14** from the exe (setters `0x005FE3C0`/`0x005FE6B0`; variant slot 26 `0x0091587D` and slot 102
  `0x00915014`; every protection call site) and written into protection.md §14.
  - **`0x005FE3C0`** writes a **scalar** `Uint32` (variant type 6), not a one-element vector. It is used for the
    strike start, the availability masks, the base call numbers, the jail population and the air sources'
    properties.
  - **`0x005FE6B0`** writes a `Uint32` **array** of the vector's length (type `0x8006`, elements copied). It is
    used only for the station statistics `0x8A70A0B9`, always 4 elements.
  - **Both** add the property when it is missing and replace the variant's type.
  - **Quirk.** The lookup falls back to the occupant's exemplar, so an ID defined in a plug-in exemplar would be
    changed there. No shipped exemplar has any of these IDs. *[choice]* Write onto the occupant.

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR10 — §5.3:** the zone manager's slot-5 grid class is not in docs/abi; `GetZoneType(x, z)` (slot 12) is used
  instead, 0 outside the city. Same values? [slot 12]~~ **ANSWERED 2026-10-14** from the exe (slot 5 `0x00698CF0`
  returns `+0x48`; created in zone manager `Init` at `0x004DE26F`–`0x004DE2D0`; `GetZoneType` `0x004DC740`; tract
  test `0x006F6180`) and written into protection.md §5.3. **Same values; the bracket matches inside the city.**
  - **The grid.** It is a `cSC4SimGrid<int8>` (vtable `0x00A87950`, CLSID `0x49B9E603`, tract size 1).
  - **Same byte.** `GetZoneType` reads the same byte of the same grid, sign-extended. So "≠ 0" agrees for every
    in-grid cell.
  - **Outside the grid.** Neither the original's raw read nor `GetZoneType` checks bounds, so there is no original
    value. "0 outside" is the implementer's choice, and it never triggers for grids the game creates.

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR11 — §12:** "void1 = the simulator" posts the primary interface pointer. Right pointer? [primary]~~
  **ANSWERED 2026-10-14** from the exe (cycle-end posts fire `0x006AD3A3`, police `0x006E0616`; QIs fire
  `0x006ACEFE`, police `0x006E008E`; police posts `0x006E0B92`, `0x006E166F`, `0x006E1DD8`, `0x006E1BFA`,
  `0x006E1D55`; message slots 12 `0x00913322`, 33 `0x0091346B`) and written into protection.md §12. **Confirmed;
  the bracket matches.**
  - **The pointer.** Every "the simulator" field is the primary interface pointer (vtables `0x00AB1160` /
    `0x00AB2490`).
  - **How it is obtained.** At a cycle end it comes from `QueryInterface(1)`, which returns the primary itself and
    is released after the post. The station and jailbreak posts use `this` or `this − 8` directly.
  - **How the message holds it.** void1 is stored raw. IGZUnknown is a counted reference.

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR12 — §4.1:** the simulator, flammability and crime simulators are not held (never called); only reference
  counts differ. Acceptable? [not held]~~ **ANSWERED 2026-10-14** from the exe (fire `Init` `0x006AE2D4`–`0x006AE2ED`,
  police `Init` `0x006E285E`–`0x006E286F`; `Shutdown` fire `0x006AD222`, police `0x006E03AF`; `ProtectionInit`
  `0x006F6BC7`–`0x006F6E09`) and written into protection.md §4.1. **Acceptable; the bracket matches.**
  - **Flammability and crime.** The original holds each at `+0xCC` from `Init` to `Shutdown`. No other code in
    either class reads it, at any sub-object offset.
  - **The simulator.** The original does not hold it at all: it reads the cache global `0x00B43D5C` for
    `AddAgent`.
  - **Spec correction.** Base init holds the city, the **budget simulator** and the **zone manager**. It holds the
    message server and subscribes only when the server is not held yet.
  - **Effect of not holding.** Only the reference counts, and so the objects' lifetimes, differ. Neither class can
    observe that.

<!-- from: From the protection glue and grid shadow (2026-10-14) -->
- ~~**PR13 — §13.2:** the air sources are created through GZCOM and their Init/Shutdown called by address while our
  replacement classes are active. Intended reading? [yes; no air dispatch on an unidentified executable]~~
  **ANSWERED 2026-10-14** from the exe (fire `Init` `0x006AE323`–`0x006AE373`; police `0x006E28BE`–`0x006E290D`;
  factories `0x006ACEA0` / `0x006E0030`, registered at `0x00460B33`–`0x00460B52`; source QI `0x006AC550`) and
  written into protection.md §4.1, §8.4 and §13.2. **Yes; the GZCOM path gives the original's object.**
  - **Same object both ways.** The registered factories do exactly what `Init` does: `GlobalHeapAlloc(0x28)` plus
    the same constructor. The original itself creates the sources through the registry when it loads a save with
    a source ref. So the spec's "operator new" is corrected.
  - **Which IID to request.** Ask GZCOM for IID `0xC9C00435`, which gives the object start that `Init` and
    `Shutdown` need in ECX. The sources refuse IID 1 (`cIGZUnknown`).
  - **Unidentified executables.** Skipping air dispatch there has no original behaviour; the implementer's choice
    stands.
- Not implemented: the 14 extra message-target slots 4–17 (§15.3; no outside caller found).

## From applying the protection, demand and pollution answers (filed by the coordinator from the `apply3` lane's report, 2026-10-14)

About PR1–PR13, DQ3–DQ8 and PG4–PG13 as applied. The current answer is in brackets.

<!-- from: From applying the protection, demand and pollution answers (2026-10-14) -->
- ~~**PR14 — protection §5:** the dispatch active/inactive handler: which occupant does `GetDispatchEffect` read?
  [`info.station`, as UpdateUnadjusted; inferred]~~ **ANSWERED 2026-10-14** from the exe (handler `0x006F647F` →
  `DoDispatchActivated` `0x006F61F0`; `GetDispatchEffect` fire `0x006AD720`, police `0x006E0CB0`;
  `AdjustProtectionEffect` `0x006F56A0`) and written into protection.md §5 (the active / inactive handler) and §7.2.
  **`info.station` (record `+0x20`); the bracket matches, now confirmed.** Two corrections to protection.md came out
  of it:
  - **§7.2 had one caller; there are two.** `UpdateUnadjusted` and this handler both pass the record's `+0x20`.
  - **§7.2 said "no efficiency scaling"; the effect is scaled.** `GetDispatchEffect` fails (AL = 0) only for an absent
    or zero centre; otherwise it reads outer and radius (an absent one keeps the caller's value) and scales all three
    by `GetStationEfficiency(station)`.
  - **Handler quirk.** The handler ignores `GetDispatchEffect`'s AL and passes an uninitialised stack radius when it
    fails. *[choice]* FreeCity4 skips the update then.

<!-- from: From applying the protection, demand and pollution answers (2026-10-14) -->
- ~~**PR15 — PR12:** how long does the protection simulator hold the budget simulator and the zone manager? [taken in
  base init only if not already held, released at the end of Shutdown; the read-only shadow host asks the city each
  time]~~ **ANSWERED 2026-10-14** from the exe (`ProtectionInit` `0x006F6BC7`–`0x006F6C0B`; `ProtectionShutdown`
  `0x006F56F0`) and written into protection.md §4.2 (base shutdown, now traced in full). **Confirmed; the bracket
  matches.** Both are taken in base init only when not already held and released in the last step of the base shutdown
  (zone manager, then budget simulator, then city), which comes before the class releases its flammability or crime
  simulator and departments. Asking the city each time in a read-only shadow host cannot be told apart. The trace also
  fixed the base shutdown's order: dispatch unregister, station list, the nine unsubscriptions (in an order slightly
  different from the subscriptions), `RemoveAgent`, the grids, then the held services.

<!-- from: From applying the protection, demand and pollution answers (2026-10-14) -->
- ~~**PR16 — PR5:** how does a shadow recognise the original class before reading +0x84, and are unadjusted mismatches
  judged like the adjusted grid's? [by the primary vtable against the §15.1/§15.2 addresses (compat table, identified
  executable only; otherwise not read); logged, not judged]~~ **ANSWERED 2026-10-14** from the exe (constructors fire
  `0x006ADA67`, police `0x006E1F9D`; destructors `0x006ADB27`, `0x006E16ED`; base constructor `0x006F6986`;
  `ProtectionShutdown` `0x006F580D`; city getters `0x006DC690` / `0x004597C0`) and written into protection.md §2.
  **Recognition: the bracket matches. Judging: no original behaviour; the implementer's choice stands.**
  - **Recognition.** The city's getters return the object start, whose first dword is the primary vtable (`0x00AB1160`
    fire, `0x00AB2490` police). Only the class's own constructor and destructor write it, so a replacement or wrapper
    fails the test. The `+0x84` member is null before `Init` and again after `Shutdown`; the grid's `GetInstanceID()`
    (`0x89D5B91A` / `0x49D5BBCE`) is an optional extra check.
  - **Judging.** The original never compares the grids. The unadjusted grid has no copy lag, but its dispatch cones
    follow the automata, so tracts under an active dispatch can differ for reasons outside the simulator; logging only
    stays a sound choice.

<!-- from: From applying the residential, protection and save-reference answers (2026-10-14) -->
- ~~**PR17:** Protection §5.1: what does `UpdateUnadjusted` do when `GetDispatchEffect` fails? [the return value is
  ignored and the record is zeroed, so the centre adds 0]~~
  **ANSWERED 2026-10-14** from the exe (`UpdateUnadjusted` `0x006F5D83`–`0x006F5DFE`; `GetDispatchEffect` fire
  `0x006AD720`, police `0x006E0CB0`) and written into protection.md §5.1 with a worked example. The AL is ignored,
  as in the bracket, but **the record is not zeroed**. The dispatch loop reuses the station loop's stack record, and
  a failed call zeroes only x, z and centre. The cone is drawn with `c = 0` and the leftover outer and radius from
  the previous dispatch or the last station: an inverted cone, 0 at the centre rising towards the old outer.
  **The bracket differs**, but only for custom stations: every shipped fire and police station has a non-zero
  dispatch centre, so with shipped data the call never fails. *[choice]*: keep one record across both loops to
  match; the zeroed record is acceptable as a known difference.

<!-- from: From applying the residential, protection and save-reference answers (2026-10-14) -->
- ~~**PR18:** Protection §7.2: what is in the caller's record? [a zeroed record]~~
  **ANSWERED 2026-10-14** from the exe (`0x006F5C87`, `0x006F5DA9`; handler `0x006F6221`) and written into
  protection.md §7.2. **Never a zeroed record.** In `UpdateUnadjusted` it is the record shared by every station and
  dispatch of the rebuild: the previous dispatch's values, else the last station's, else uninitialised stack. In
  the active/inactive handler it is a fresh, uninitialised stack record. **The bracket differs** when the call
  fails, and on success when the station lacks the outer or radius property (that field keeps the leftover).

<!-- from: From applying the residential, protection and save-reference answers (2026-10-14) -->
- ~~**PR19:** The older "PR14/PR15" labels in the jail vectors, `protection/mod.rs` and `algo.rs` clash with the
  QUESTIONS numbering. [left as they are]~~
  **ANSWERED 2026-10-14**: glue-only, no original behaviour; the implementer's choice stands.

<!-- from: From applying the exemplar, graphics, protection, landfill and agent-order answers (2026-10-15) -->
- ~~**PR20:** PR18 (protection §5.1): the answer covers `UpdateUnadjusted`'s caller record. Does the separate
  dispatch-message handler also carry leftovers into its own record? [no: it keeps a zeroed record]~~ **ANSWERED 2026-10-15 (re-answers11):** **DIFFERS** in the letter, choice accepted (protection.md §7.2, PR18 table). The handler's record is in its own stack frame (`0x006F61F0` reserves 0x44 bytes and writes nothing to the effect record before the call at `0x006F6221`), so it does not carry `UpdateUnadjusted`'s values. It holds stale bytes from unrelated earlier calls. Only the radius is used; a successful call with the radius property writes it, and FreeCity4 skips the update on failure, so a zeroed record is equivalent on shipped data.
