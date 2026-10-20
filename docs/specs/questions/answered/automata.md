# automata: answered questions (archive)
<!-- Archive for [automata.md](../../automata.md). Lanes do not read this file. Open questions are in ../automata.md. -->

<!-- from: From the occupant manager and automata cores (2026-10-16) -->
- ~~**AT2 (automata.md §13 Q2):** the meaning of the two `vec3`s and the `bool` of a controller-manager automaton
  set.~~ **ANSWERED 2026-10-16 (re-answers16):** it is an occupant's **queue**:
  - the line of automata, front first;
  - the queue point, 1 m outside the middle of the lot edge that touches a network (traffic network map slot 14);
    the lot's facing picks the first side;
  - the unit direction in which the line grows;
  - "point found".

  State 4 (`QUEUE`) appends, steers each automaton behind its leader, and removes (`0x0051BB60`–`0x0051BD50`; point
  search `0x00516070`). automata §3.3.

<!-- from: From the occupant manager and automata cores (2026-10-16) -->
- ~~**AT3 (Q3):** the type numbers of RandomWalk, PlopJump, Plummet, PullOver, Bumped and Collide, and the meaning
  of corpus state types 0, 2, 3, 7, `0x0D`.~~ **ANSWERED 2026-10-16 (re-answers16):**
  - `0x0B` RandomWalk, `0x0E` Plummet, `0x0F` PlopJump, `0x11` Bumped, `0x14` Collide, `0x15` PullOver: the Windows
    factory `0x0050B7F0` sizes match the Mac classes, and the game's Lua `automata_states` table agrees;
  - 0 DEFAULT, 2 BEE_LINE, 3 CROWD, 7 IDLE;
  - 13 is the traffic automaton's blocked wait, pushed at `0x0054E322` for (tuning `0x00B21C74`) × 1000 ms.

  automata §6.2.

<!-- from: From the occupant manager and automata cores (2026-10-16) -->
- ~~**AT4 (Q4):** the records of aircraft and watercraft automata.~~ **ANSWERED 2026-10-16 (re-answers16):**
  - aircraft base `0xC96E75DC`: `u16` 1 + automaton base (`0x004FB060`/`0x004FB080`);
  - airplane `0xCBEB4F62` and helicopter `0x8BA98EDA`: `u16` 1 + aircraft record + `vec3` home position
    (`0x004FD380`/`0x004FD3C0`, `0x005343C0`/`0x00534400`);
  - watercraft `0x096E75F5`: `u16` 2 + automaton base (`0x00566EE0`/`0x00566F00`).

  automata §6.4.

<!-- from: From the occupant manager and automata cores (2026-10-16) -->
- ~~**AT5 (Q5):** the generator's two trailing `u32`s, the automaton base's unnamed fields, `CreateAutomaton`'s mode
  and last three arguments.~~ **ANSWERED 2026-10-16 (re-answers16):**
  - the generator `u32`s are the current automata group ID (corpus `0x4122`–`0x4139`) and the count spawned from it;
  - the base fields are named in automata §6.1 (attractor, distance, strength and attraction vector; model
    instance; speed and opacity tweens; priority; group ID);
  - `CreateAutomaton` is (group, const vec3& position, u32 automata flags, const key3& model key,
    cIGZMessageTarget2* callback, u8 priority, const vec3* orientation). It returns a pointer without a reference
    for the caller.

  automata §3.5, §6.1, §7.2.

## From the automata layer core (filed by the coordinator from the `cores14a` lane's report, 2026-10-16; merged `44cf925`)

cores14a implemented automata.md (short form) as a Rust core (`engine/core/src/automata/`), a C ABI (`fc4_automata_*`;
ABI 62, reserved 65 on the lane) and conformance vectors (`am14_01`-`am14_14`); §14 test 12 is the formats save sweep
(`cargo test -p fc4-formats --release --test automata_records -- --nocapture`: 417 saves, every record of components
40, 41 and 43-46 and every referenced record re-encodes byte-identically). There is no plugin glue yet, so none of these
readings has been checked in game. The core predates re-answers16 (AT2-AT5 above), which a later lane applies. AM is the
prefix the code comments use; automata.md's own Q1-Q6 keep their numbers. The current choice is in brackets.

<!-- from: From the automata layer core (2026-10-16) -->
- ~~**AM1:** (versions on re-save) the original's writers always write the current version (traffic manager 3 / base 6,
  automaton base 8, state 3, aircraft 5, watercraft 6), but §14 test 4 expects a version-2 record to round-trip byte
  for byte. Must a re-save upgrade old versions, or keep them? [writes the version it read; raises it only when the data
  needs the newer layout]~~ **ANSWERED 2026-10-16 (re-answers20):** **DIFFERS** (automata.md §4.1). Every writer writes a constant, its current version: base 6 `0x0050CB80`, traffic manager 3 `0x00553570`, aircraft manager 5, watercraft manager 6, automaton base 8 `0x0050AC0D`, state 3 `0x00509D0A`, pedestrian 5, vehicle, emergency and trailer 2. It never writes the version it read, and the missing fields come from the live object: a version-2 traffic manager re-saves as 3 with its incident list, and a version-4 base as 6 with the source building counts that `PostCityInit` recounted. A live save must write the current versions. A record codec may keep the read version for byte round-trip tests *[choice]*; §14 test 4 is such a codec test, and test 18 is the re-save.

<!-- from: From the automata layer core (2026-10-16) -->
- ~~**AM2:** §6.4 gives no read check for the pedestrian (5), vehicle, emergency vehicle, patrol car and trailer (2)
  needs the newer layout]
  *Evidence added 2026-10-16 (re-answers29), answering the state record's half.* The **state base Write
  (`0x00509D00`) always emits version 3 and all seven fields**, whatever version was read, and the Read
  (`0x00505E50`) accepts ≤ 3 and reads the seventh field only at version ≥ 3. So for states the original
  **does upgrade on re-save**: a version-2 state comes back as version 3. That **DIFFERS** from the bracket.
  automata.md §6.2. **What the implementer must change:** a state record of version < 3 must not be expected to
  round-trip byte for byte — it comes back as version 3 with the extra field written (zeroed for a v2 read).
  This does **not** settle §14 test 4, which is the *vehicle manager's* version-2 record and is a different
  writer; the managers, automaton base, aircraft and watercraft versions are still open here.

<!-- from: From the automata layer core (2026-10-16) -->
- **AM2:** §6.4 gives no read check for the pedestrian (5), vehicle, emergency vehicle, patrol car and trailer (2)
  versions, nor for the incident list's `u16` 1. Do the original readers reject other versions? [any value accepted
  and kept]~~ **ANSWERED 2026-10-16 (re-answers20):** **DIFFERS** (automata.md §4.1 table, §6.4). The original requires exact values: pedestrian 5 (`0x005446D5`), vehicle 2 (`0x0055BCF0`), emergency 2 (`0x0052FAE0`), patrol car 2 (`0x00543E05`), trailer 2 (`0x00556C47`), and the incident list's `u16` 1 (`0x00553E90`). A mismatch makes that Read return false. The incident's ref is checked against the occupant IID `0x26BB767E`.

<!-- from: From the automata layer core (2026-10-16) -->
- ~~**AM3:** §5.1 field 4 (the ID at the traffic manager's list cursor): what are the IDs of the list nodes, and what
  should a new manager write? [the value read is kept; 0 for a new manager]~~ **ANSWERED 2026-10-16 (re-answers20):** Read value kept: MATCHES. New manager writes 0: **DIFFERS**, it writes 1 (automata.md §5.1). The list is the manager's fixed **layer list**, one node per traffic layer with the layer ID and an accumulated time. Vehicle `0x0055FB50`: 1, 2, 3, 5, 4, 7, 6, 8, 9, 10; pedestrian `0x00545500`: 1, 7, 8. The cursor starts on the first node, so a new manager writes 1, which every empty-city corpus record holds. Each population pass moves it on, wrapping and skipping layers disabled in the tuning mask. Write gives 0 only when the cursor is at the end, after a Read that found no node with the saved ID. Field 3 is the default path type: vehicle 1, pedestrian 2.

<!-- from: From the automata layer core (2026-10-16) -->
- ~~**AM4:** §7.3 SetMaxCount removes the automata "beyond" the new maximum: which ones? Does CreateAutomaton refuse at
  the hard limit? [the newest first; CreateAutomaton returns null at the limit]~~ **ANSWERED 2026-10-16 (re-answers20):** **DIFFERS** (automata.md §7.2, §7.3, §12 quirk 8). `SetMaxCount` (`0x0050AA60`): clamp to 0–5000; an unchanged n does nothing. Otherwise it walks the list from the **front** (oldest first) while n < count. Every automaton without flag `0x200000` and below priority `0xFF` goes to the unplace hook (slot 62, `0x00507430`), which takes it out of the renderer and the occupant manager. The hook does not erase it from the list or lower the count, so all eligible automata are unplaced and none leaves the list or the save. Then max = n and hard limit = trunc(n × 1.25). `CreateAutomaton` refuses above the **max** (count > max) unless make-room (`0x0050A740`) succeeds. Make-room needs count < hard limit and p > 0, and evicts the first automaton of lower priority with `Remove(false)`. Flag `0x80` and priority `0xFF` bypass even the hard limit.

<!-- from: From the automata layer core (2026-10-16) -->
- ~~**AM5:** (§3.1) how exactly do the controller manager's two times A and B advance with HourChanged, and when can A
  differ from B?~~ **ANSWERED 2026-10-16 (re-answers20):** Answered (automata.md §3.1). A is the current time (object `+0x34`), the one slot 24 `GetControllerTime` (`0x0051A310`) returns. §3.1 and §7.1 said B; **corrected**. B (`+0x3C`) is A at the previous animation tick.
  - `HourChanged` (`0x005193BF`) adds 1.0 h to A (`0x00515080`), unnormalised, so A can reach 24 or more. Each controller then gets slot 24 with (A, A − B).
  - Each animation tick (`0x00518B80`): t = hour + minute × (1/60); if t < A's hour, A's day + 1; A's hour = t; delta = A − B; A is normalised (NaN → 0, hour into [0, 24), day mod 7); then B = A.

  So A ≠ B only between an `HourChanged` and the next tick (A = B + 1 h). Both must round-trip.

<!-- from: From the automata layer core (2026-10-16) -->
- ~~**AM6:** (§7.1 slots 5, 6, 9, 10) what does the `extra` argument mean? [ignored]~~ **ANSWERED 2026-10-16 (re-answers20):** Mostly MATCHES (automata.md §7.1). `extra` is a `const vec3*` position. Slot 6 (`0x0051AD20`) uses it only when the occupant is null: it calls controller slot 17 (`+0x44`) with it at `0x0051B117`. With an occupant, slot 14 attaches the controller (`0x0051B091`) and `extra` is unused. Every known caller passes an occupant and 0, so ignoring it is correct there. A null-occupant call must place the controller at `*extra`.

<!-- from: From the automata layer core (2026-10-16) -->
- ~~**AM7:** (§7.1 slots 11 and 12) does the original track the controllers that an occupant group itself created?
  ["remove the controllers that the group added" is the occupant's controllers whose template is one of the group's
  templates, which also works after a load]~~ **ANSWERED 2026-10-16 (re-answers20):** MATCHES (automata.md §7.1). Nothing tracks which controllers a group created. The body (`0x0051A350`) removes the occupant's controllers whose template (controller slot 8) is in the group's template list. Each removed controller gets the message `0xEA298D50`, leaves the grid, and is shut down and released. When the list becomes empty, the occupant's queue entry and its list entry are dropped. The call returns true if anything was removed. Slot 14 is the same body with a null template, which matches all.

<!-- from: From the automata layer core (2026-10-16) -->
- ~~**AM8:** §6.4 gives the emergency vehicle record as `u16 2, vehicle record, …`, but §12 quirk 3 says the vehicle
  loader "reads the vehicle part and leaves the rest unread". With the §6.4 layout a vehicle reader meets the
  emergency's own `u16` first and misaligns, failing at the occupant base version. Which layout is correct?

<!-- from: From the automata layer core (2026-10-16) -->
- **AM9:** §5.3 gives no upper version check for the watercraft manager. [fails above 6, as §5.2 does for the aircraft
  manager above 5]

<!-- from: From the automata layer core (2026-10-16) -->
- **AM10:** §6.4 names no IID for the trailer's list of vehicle references. [read with the automaton IID
  `0xA9B40F05`]

<!-- from: From the automata layer core (2026-10-16) -->
- **AM11:** (§4, §12 item 4) the original base Read ignores stream errors. Should a truncated manager record load
  partially, as in the original? [a short or cut record is a failed, atomic read, so round trips can be checked]

<!-- from: From the automata layer core (2026-10-16) -->
- **AM12:** (§7.3 slot 31, TrackAutomataGroupInstances) what are the semantics? [adds delta, does not go below 0,
  appends a new group]

## From the return-channel audit (filed by the coordinator from the `voidbool` lane's report, 2026-10-16; merged `2501158`)

voidbool checked every place `engine/plugin` reads a game method's return value against that slot's `ret` column in
`docs/abi/interfaces.tsv` and the specs, fixed 10 call sites in 7 groups (F1-F7, each marked `[abi-ret]`) and added
the checker `engine/tests/check_ret_channels.py` (ctest `ret_channel_check`). Its `SPEC_FACTS` table marks each slot
bool, none, wide or spec-silent (`sdk`); the uses it trusts without a spec answer are listed by `--verbose` and asked
below. When a spec answers one, move its `SPEC_FACTS` entry from "sdk" to "bool" or "none" and fix any call the checker
then flags. The current reading is in brackets.

<!-- from: From the return-channel audit (2026-10-16) -->
- ~~**AM9:** §5.3 gives no upper version check for the watercraft manager. [fails above 6, as §5.2 does for the aircraft
  manager above 5]~~ **ANSWERED 2026-10-16 (re-answers20):** MATCHES. `0x00574A0B`: below 6 is a base-only record, above 6 fails (`0x00574A26`). automata.md §5.3.

<!-- from: From the return-channel audit (2026-10-16) -->
- ~~**AM10:** §6.4 names no IID for the trailer's list of vehicle references. [read with the automaton IID
  `0xA9B40F05`]~~ **ANSWERED 2026-10-16 (re-answers20):** **DIFFERS**: the IID is `0x896E714A` (`0x00556C8B`; automata.md §6.4). That is the vehicle's class ID, which the vehicle's `QueryInterface` (`0x0055B665`) answers. Refs to vehicles, emergency vehicles, patrol cars and trailers resolve; any other automaton reads as null. Null entries are dropped from the list.

<!-- from: From the return-channel audit (2026-10-16) -->
- ~~**AM11:** (§4, §12 item 4) the original base Read ignores stream errors. Should a truncated manager record load
  partially, as in the original? [a short or cut record is a failed, atomic read, so round trips can be checked]~~ **ANSWERED 2026-10-16 (re-answers20):** Atomic failure accepted as *[choice]* (automata.md §4.1). The original stores fields as it reads and never rolls back. The current-version wrappers return `GetError() == 0`: controller manager `0x0051B9B8`, traffic v3 through the incident reader, aircraft v5 `0x004FBFC3`, watercraft v6 `0x00575153`. A cut record of a current version therefore returns false, as in the core. Only a cut base-only old version (traffic 2, aircraft < 5, watercraft < 6) returns true in the original. That is a **DIFFERS** case that no save contains.

<!-- from: From the return-channel audit (2026-10-16) -->
- ~~**AM12:** (§7.3 slot 31, TrackAutomataGroupInstances) what are the semantics? [adds delta, does not go below 0,
  appends a new group]~~ **ANSWERED 2026-10-16 (re-answers20):** **DIFFERS** (automata.md §7.3, §4). Slot 31 `TrackAutomataGroupInstances(group, bool track)` (`0x00513C40`) is not a counter. When slot 30 says the group is known, `track` = true creates an unsaved **set of the group's automata**, with a reference each, filled from the list; an existing set is kept. `track` = false releases the set and erases it. Later adds and removals keep the sets current, and slot 32 reads them. §4 field 6 was mislabelled: it holds the **source building counts**, occupant group ID → number of complete buildings, which slots 36 and 37 read. They are seeded at `Init` from script slot 18 (`0x0050F930`) and recounted at `PostCityInit` for base versions below 6 (`0x0050F341`). The record bytes are unchanged; only the meaning changes.
