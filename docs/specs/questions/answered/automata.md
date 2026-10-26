# automata: answered questions (archive)
<!-- Archive for [automata.md](../../automata.md). Lanes do not read this file. Open questions are in ../automata.md. -->

<!-- from: automata-actions29 (2026-10-26) -->
- **AT25 - §7.6 (NM17), native construction continuation after balanced worker slots:** The replacement
  manager returns its own primary pedestrian wrapper (class `0x896E75AF`); it does not invoke native
  CreateAutomatonInternal or register a concrete pedestrian factory. Its former argument-free placeholders
  at slots 30/14 left 8/4 bytes on the stack. Those calls now have NM17's signatures; slots 7/8 supply
  GetPosition(out vec3)/SetPosition(in vec3), each popping 4 bytes. A real-DLL slot test verifies balanced
  stack/x87 and the worked roof change `(12.5,7,34.5)` to `(12.5,9,34.5)` for both manager wrappers.
  The original `0x29A5D1EC` fault disappears, but the full-house generated grow acceptance is incomplete:
  `aa29-fix-house-s0-a` reaches 216/216 and 2028-01-28 (only the excluded native quit fault at +0x1D0F94);
  `aa29-fix-house-s1-a` reaches 27/216, creates three workers and then faults on another thread in
  ucrtbase.dll+0x973FE; its repeat `aa29-fix-house-s1-b` reaches 27/216 then faults in ntdll.dll+0x4E728.
  `aa29-fix-house-s2-a` reaches 75/216 then faults at SimCity 4.exe+0x1CC3B3, reading `0x4396001C`
  (`eax=0x43960000 ecx=0x18747A88 edx=0x001AED84`); its repeat reaches 84/216 then faults at +0x3529C0.
  All have ref_warnings=0. With only automata disabled, the same DLL/house/s1 script completes 216/216
  and quits cleanly (`aa29-control-noauto-a`, 2028-01-29, ref0). Logs are in main's build-runs/game-results/.
  What additional observable contract must these returned workers satisfy during construction/scaffold
  continuation? In particular, settle the return-register value of GetPosition (the ABI row says
  eax-or-void), whether callers consume a returned vector/reference, and any further called slots or
  concrete views despite replacement of the manager's own creation/removal bodies. The new failures do
  not establish a specific cause; no return value, concrete layout or native callback was guessed.
  [blocked item: full-house s1/s2; retain NM17's already-tested stack correction]
  **ANSWERED 2026-10-26 (re-at29):** DIFFERS — automata.md §§7.6.1, 7.6.4–7.6.5 correct the earlier
  no-return claim: primary slot 7 returns its caller-supplied output pointer in EAX after copying the
  three position words, with 4-byte cleanup and no x87 result. Construction ignores that EAX, reloads
  its saved primary A, replaces the local buffer's y, and invokes slot 8 on A. This occurs before
  SetLot returns and on subsequent rising/lifting/lowering animation ticks; daily phase transitions
  do not recreate workers. No additional concrete worker view or callback is introduced when both
  manager bodies are replaced. Retain the primary interface and construction-owned reference across
  ticks, then perform the specified manager removal and owner Release. The D0–D0+8 worked case checks
  call order, position words, clock inputs and lifetime. The +0x1CC3B3 report is dispatch through A's
  vtable before the getter runs, so a getter return-register correction alone cannot explain/fix it;
  no runtime root cause or passing action run is claimed.

<!-- from: coordinator train84 controlled saved-port union (2026-10-22) -->
- **AT24 - aircraft manager loaded-reference destruction:** On identical integration DLL c269a6ba5f, immutable replacement save SHA25638f234433b0c..., identical seven-action reload script and literal39 replacement keys, both +ports and ports-disabled control reach2033/ref_warnings3. All three warnings are native cSC4Airplane class0xCBEB4F62, vtable SimCity4.exe+0x694B38, at cFC4Automata.cpp HostRelease:53 during destruction. Source labels train84-ports-union-reload-20261022-a and train84-ports-union-control-20261022-a are preserved under build-runs/game-results. The normal candidate union and seven ports-only/reload runs are clean/ref0. Does original aircraft-manager Read0x004FBE70 retain owning references to its airport/airplane map, and which exact clear/shutdown/destructor sites release them? Prove reference acquisition, loaded-segment lifetime, release order and final-owner conditions before changing ownership-audit classification or release behavior. Is the existing generic HostRelease semantically wrong or missing a justified destructor-only ownership annotation? No warning suppression or ports replacement promotion is authorized by this observation. [unresolved; same three warnings in control establishes an existing automata-path issue, not a ports-caused difference]
  **ANSWERED 2026-10-22 (re-aircraft-ownedrefs4):** DIFFERS — automata.md §§5.2.1–5.2.2 prove the
  native ownership rule, not the observed helper call stack. Read retains both QI-acquired references
  across segment disposal. Initialized aircraft Shutdown `0x004FB7D0` releases airport then plane per
  map entry, clears the map, then calls base Shutdown. PreCityShutdown clears the separate base list,
  not this map; ordinary removal erases then releases plane before airport. Final manager Release
  calls Shutdown before deleting destruction; deleting destructor `0x004FBCF0` frees storage without
  releasing map payloads. A last plane Release is valid at the specified Shutdown event, including
  final-Release-entered Shutdown. Holding these references past Shutdown until member destruction
  differs from native; no destructor-only ownership annotation is justified by these warnings or the
  controller-manager precedent. Seven independent probe families and native call signatures are
  published. Residual: without phase/receiver/refcount evidence at HostRelease:53, the three existing
  warnings cannot be classified as correct Shutdown releases versus delayed destructor cleanup.
  The paired runs establish a pre-existing path only. No implementation or audit change authorized.

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

<!-- from: a fragment of AM8 that an old union merge left inside ZG42 (moved here 2026-10-20) -->
- (AM8, tail of the answer) emergency's own `u16` first and misaligns, failing at the occupant base version. Which layout is correct?~~ **ANSWERED 2026-10-16 (re-answers20):** The §6.4 layout is correct and **quirk 3 was wrong** (automata.md §12 quirk 3, §6.4). The emergency Write (`0x0052FA60`) writes `u16` 2, the full vehicle record, then its own fields. Reloaded under the vehicle class ID, the readers shift by one `u16`: vehicle (2 ok), traffic (2 ok), automaton base (2, ≤ 8 ok). The occupant base then reads 8 where it needs 4 (`0x005E8D75`) and fails, so that object's Read fails; it does not stop partway with success. The cores14a description is right.

## From the enable=/spec-contradiction wave (filed by the coordinator from the implementer lanes' reports, 2026-10-16; answered by re-answers31)

These were filed in the implementer lanes' own `docs/lanes/<lane>.md` files, which are not merged here, so each
entry is recorded with the lane it came from. Every answer is written into the spec named below.

<!-- from: From the enable=/spec-contradiction wave (filed by the coordinator from the implementer lanes' reports, 2026-10-16; answered by re-answers31) -->
- ~~**A4** (glue19): the city's Init phase faults at `SimCity 4.exe+0x0045BF1B` (`ACCESS_VIOLATION` reading 0,
  `eax=1 ecx=0 edx=0x3000 ebx=0x00A80810`) after the terrain/surface-water `AddAgent`, with all six components
  reporting `Read -> 1`. Two causes were already fixed (QueryInterface returning the primary for
  `cIGZSerializable`; stubs returning a zeroed container where an empty `SC4List` has sentinel nodes). Find the
  third.~~ **ANSWERED 2026-10-16 (re-answers31): DIFFERS.** The null is the component at `cSC4City + 0x1E4`, the
  automata **script system** (CLSID `0x4A1B664E`, IID `0x8A1B6647`). `QueryInterface` answers the **CLSID** but
  not the **IID** the city creates it with, so the out parameter is left null and `FUN_0045bd20` calls `Init` on
  it with no null check. Same IID gap on all six classes. Written into [automata.md](../../automata.md) **§15**.

<!-- from: From the enable=/spec-contradiction wave (filed by the coordinator from the implementer lanes' reports, 2026-10-16; answered by re-answers31) -->
- ~~**A1** (glue19 / re-integrate1 §14 test 29): the state stack index (`+0x0C`) must be recomputed **on Read**;
  the corpus contradicts it (recomputing changes byte 174 of a 342-byte pedestrian record and breaks byte-exact
  re-save). Which is right?~~ **ANSWERED 2026-10-16 (re-answers31): DIFFERS from test 29 as written.** The
  stream value is authoritative and must be stored unchanged; `tAutomataState::Read` (`0x00505E50`) reads it
  straight into `state+0x0C` (`0x505E99`-`0x505E9F`) and never derives it. Recompute only after a stack
  *mutation*. Corrected in [automata.md](../../automata.md) §12 quirk 10 and §14 test 29.

## From the automata state behaviours and the effect-directory measurements (re-integrate1 lane, 2026-10-16; answered by re-answers33, 2026-10-20)

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT6:** what is PlayerDrive's saved `f32` at `+0x28`? Set to 0 on enter and never touched by
  `cSC4Automaton::DoPlayerDriveTick`. [a drive-channel / throttle accumulator, inferred] **Probe:**
  `cSC4Vehicle::TranslatePlayerDriveCommands` (Mac `0x000a577c` family) and `DoPlayerDriveOffRoadTick`.~~
  **ANSWERED 2026-10-20 (re-answers33): DIFFERS.** `+0x28` IS touched - not by `DoPlayerDriveTick`, but by
  `cSC4Vehicle::TranslatePlayerDriveCommands` (Mac `0x000a336c`): it integrates the throttle/brake key state
  into it, `+0x28 += dt_s x accel` (params+0x10 normally, params+0x14 when the speed tween is running and
  channel 0 is idle), sign by forward/reverse, and zeroes it when both drive channels are inactive. Then
  `cSC4Vehicle::PlayerDriveUpdateSpeed` (Mac `0x000a3e1a`) integrates it into the speed at `+0x2C`
  (`vel += dt_s x +0x28`, clamped to params+8/params+0xC). So `+0x28` is a **drive command accumulator** in
  speed-per-second units; it is the only field of the PlayerDrive record that is saved. automata.md 6.2, 6.5.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT7:** what reads Bumped's 4 x `vec3`? Base `DoBumpedTick` is `return !CheckAutomataFlag(1)`
  (Mac `0x000bb14e`) and `cSC4Vehicle::DoBumpedTick` is `return 0` (Mac `0x000a083e`); `HandleCollision`
  (Mac `0x000a091e`) writes them then pushes COLLIDE. [a leftover bump-response record, inferred] **Probe:**
  xrefs to `tAutomataStateBumped::Read`'s output and `cSC4Train::HandleCollision` (Mac `0x00393a4a`).~~
  **ANSWERED 2026-10-20 (re-answers33): MATCHES** the leftover-record guess. **Nothing reads them.** On the
  Mac side `tAutomataStateBumped` appears only in its own ctor (`0x00638236`, zeroes the four vectors), `Read`
  (`0x000c4608`, fills them from the stream), `Write` (`0x000c3c62`) and the factory case `0x11`
  (`0x000c8f5a`). No code pushes state `0x11` anywhere (no `SetState`/`PushState` call passes 0x11 on either
  build), and `cSC4Vehicle::HandleCollision` (Mac `0x000a091e`, Windows `0x0055C8C0`) writes the *orientation*
  `+0x58..+0x60` and pushes COLLIDE (`0x14`); `cSC4Train::HandleCollision` (`0x00558E00`) copies the train's
  `+0x94` speed onto its cars and also pushes `0x14`. The four vectors round-trip through a save and are
  otherwise dead storage. automata.md 6.2, quirk 13.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT8:** PlopJump's units. `U(30,50) x 1/3600` and `kfGravityAccel = -5.5555563e-05` only make sense
  together if `dt` is ms and the result is m/ms. [inferred] **Probe:** one scripted plop and a position trace.~~
  **ANSWERED 2026-10-20 (re-answers33): MATCHES** (dt in ms, position in m) - with the unit of the draw named:
  the `U(30, 50)` draw is in **km/h**; `x 1/3600` converts km/h to m/ms (1 km/h = 1/3600 m/ms), so the vertical
  velocity is 0.0083-0.0139 m/ms (8.3-13.9 m/s), the gravity `-5.5555563e-05` is m/ms^2, and a hop peaks at
  v^2/2g = 0.62-1.7 m after 150-250 ms. The enter code is `cSC4Automaton::HandleEnterState` case `0x0F`
  (`0x005114C5`): `RandomDoubleRangeUniform(30.0, 50.0)` (doubles at `0x00A95708`/`0x00A95710`)
  `x 0.0002777778...` (double `0x00A95700` = 1/3600) -> `state+0x2C`. The gravity is lazy-initialised in
  `cSC4Automaton::DoPlopJumpTick` (`0x005055F0`): the guard byte `0x00B22744` bit 0 gates
  `MOV dword ptr [0x00B22740],0xB8690455` at `0x00505606`. automata.md 6.2.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT9:** what pops the `final` RiotLeader after the riot end's `SetState(1, ...)` fails (automata.md 8.1)?
  The call is a no-op because the state was pushed `final=1`, and the leader is then `Release`d and
  unregistered. **Probe:** trace `cSC4DisasterInstanceRiot::Update`'s removal path.~~
  **ANSWERED 2026-10-20 (re-answers33): DIFFERS** from the implied "something pops it". **Nothing pops it.**
  `cSC4DisasterInstanceRiot::Update` (`0x0057DFE0`) walks the leader list at `this+0x1C`; for each node it
  accumulates elapsed ticks (node+0xC) and derives the phase (elapsed / this+0x50). When the leader's occupant
  reports `IsFlagSet(1)` true (occupant slot 32 - the automata flag 1 "remove"), the end branch runs: occupant
  slot `0xCC` = `SetState(1, 0, -1.0, 0, 0, 0, 0)` (the no-op), the script system (city slot 47) unregisters
  the leader's occupant group (slot `0x38`), the disaster layer (city slot 72) unregisters the location by
  occupant (slot `0x3C`), the automaton's `Release` runs (slot 2), and the node is unlinked and freed. The
  automaton is released **while still in state 8 with `final` set**; DISAPPEAR never runs for a riot leader.
  automata.md 8.1.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT10:** `random_walk.turn`'s exact template offsets. `ImportBehaviorTable` writes the array/scalar into
  `+0x54`/`+0x58` and zeroes `+0x5C`, while `DoRandomWalkTick` reads amplitude at `+0x58` and bias at `+0x5C` -
  one is off by 4. **Probe:** a Lua table that sets `turn = {a, b}` and a breakpoint on `0x0050CF90`.~~
  **ANSWERED 2026-10-20 (re-answers33): DIFFERS** - there is **no off-by-4**; the `+0x54/+0x58` reading
  mis-simulated the stack frame. `cSC4AutomataScriptSystem::ImportBehaviorTable` (`0x00522860`) reads the Lua
  key `turn` (string `0x00A96DCC`) inside the `random_walk` subtable and writes the array/scalar into the
  template's `+0x58` (amplitude) and `+0x5C` (bias): the template local is built by
  `tBehaviorTemplate::tBehaviorTemplate` (`0x0050BD40`, whose defaults are `+0x58 = 0.1` `0x3DCCCCCD` and
  `+0x5C = 0.2` `0x3E4CCCCD` - the spec's "0.01, 0.2" default row was also wrong and is corrected) and copied
  into the vector by `FUN_005205D0`/`FUN_00521350` (0x7C bytes). `DoRandomWalkTick` (`0x0050CF90`) reads the
  same offsets (`FLD float ptr [EBX+0x58]` at `0x0050D139`, `FLD float ptr [EBX+0x5C]` at `0x0050D187`). A
  scalar `turn` fills the amplitude only (bias stays 0); an array of two or more fills both. The full turn
  math is now in automata.md 6.2's Lua-key table and the new countdown paragraph.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT11:** the mapping of emergency-vehicle states 100-103 to their ticks.
  `cSC4EmergencyVehicle::DoStateTick` `0x0052F490` switches into slots `0xC3`-`0xC6`; the names are by
  elimination, not from a name table. **Probe:** the emergency vehicle's vtable at slots 195-198.~~
  **ANSWERED 2026-10-20 (re-answers33): MATCHES** the slot mapping, with the naming evidence upgraded from
  "by elimination" to the exe's own strings plus the dispatch pairings. The dispatch states 0-6 are named by
  `cSC4EmergencyVehicle::GetDebugExtraText` (`0x00532290`, jump table `0x00532414`): `0 "[at station]"`,
  `1 "[route blocked]"`, `2 "[done]"`, `3 "[en route]"`, `4 "[no resources]"`, `5 "[handling event]"`,
  `6 "[returning home]"` (strings `0x00A98574`...`0x00A98514`). The state types are tied to the dispatch states
  by the code that pairs them: `SetDispatchState(3)` + `SetState(100)` (`0x0052FF60` case 3);
  `FUN_0052F210` = `NotifyCallbacks(5, 0)` + `SetDispatchState(5)` + `SetState(0x66=102)` on arrival (or
  `SetDispatchState(1)` + `PushState(0x65=101)` + `PushState(7, ...)` on a blocked route); `FUN_0052F330` =
  `SetDispatchState(6)` + `PushState(0x67=103)`. So 100 EN_ROUTE, 101 REPATHING, 102 HANDLING_EVENT,
  103 TELEPORT stand on those ties; there is still no name table for the state types themselves.
  automata.md 6.5.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT12:** the exact contract of `FUN_005096B0` (the per-tick behaviour refresh) on `state+0x24` vs
  `state+0x08`. **Probe:** `cSC4AutomataController::GetBehaviorTemplate` (`0x0051BAB0` / Mac `0x000b8ce0`).~~
  **ANSWERED 2026-10-20 (re-answers33):** `FUN_005096B0` is **automaton slot 105** (Windows `0x005096B0`;
  `cSC4Pedestrian::vftable_2` slot 146). The emergency family overrides the same class-vtable slot with
  `0x0052F730`, which tail-calls it only when flag `0x80` is clear and the top state is not `0x34`. Contract:
  (1) `CheckAutomataFlag(0x4000)` set -> return; (2) look up the behaviour template for the automaton's current
  occupant group (city slot `0xBC` -> controller-manager slot `0x58` with the occupant `this+4`, the attraction
  vector `this+0xA4` and an out point `this+0x9C`), storing the found template at `this+0x98` and releasing the
  old one; (3) if found, query the group's queue info (slot `+0x80`, position `this+0x40`), draw a uniform and
  compare it with the group's strength; (4) walk the top of the state stack: **`state+0x24`** (the stored
  behaviour template) equal to the found template -> stop, no push; the state's **controller** (`state+0x08`)
  null or its group differing -> the state is popped (slot 53); otherwise `PushBehavior` (slot 55,
  `vtable+0xDC`) pushes the found template. Both fields are read every refresh. automata.md 6.5 (new
  subsection).

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT13:** the fate of an aircraft or watercraft that finishes `CRASH_RUNAGROUND` / `CRASH_SINK` - neither
  transitions when its duration expires. [the manager later reaps it, inferred] **Probe:** trace
  `cSC4WatercraftManager` occupancy/cleanup.~~ **ANSWERED 2026-10-20 (re-answers33): DIFFERS** - nothing reaps
  it. `cSC4WatercraftManager::DoAnimationTick` (`0x005736A0`) has no state-70/71 cleanup, and a scan of all 41
  watercraft functions finds no other writer of the crash accumulators or a reaper. The beached or sunk boat
  stays in its final state until something else ends it: automata flag 1 removal, the occupant leaving the city
  (`cSC4WatercraftManager::HandleOccupantRemoved` `0x00574520`), or a new `SetState` (the player taking the
  wheel pushes state `0x10`). automata.md 6.6.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT14:** whether `field259_0x2BC`'s idle budget never being re-zeroed is intentional lifetime-based
  despawning of idle boats or a missed reset (a boat that has spent 60 s total in IDLE is slow-removed).
  **Probe:** every writer of that field.~~ **ANSWERED 2026-10-20 (re-answers33): MATCHES** the lifetime
  reading. Exactly two writers exist across all 41 watercraft functions: `cSC4Watercraft::Init`
  (`0x0056DE40`, `MOV dword ptr [ESI+0x2BC],0` at `0x0056E2A0`) and `cSC4Watercraft::DoStateTick` case 7
  (`0x0057336A`: `+= dt`, and `> 60000.0` at `0x00A9DA40` -> slot 51 `SetState(0x49 SLOW_REMOVE, ...)`). It is
  never re-zeroed on leaving IDLE, so the 60 s budget is **cumulative over the boat's life**. automata.md 6.6.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT15:** whether `Crash_SetStateInternal`'s slot-59 read is a "don't re-crash" guard - Ghidra dropped the
  comparison (Mac `0x003804a6`). [inferred] **Probe:** `x86dis` on the Windows inlined copy at `0x00568570`,
  or `0x00570270`'s callers.~~ **ANSWERED 2026-10-20 (re-answers33): DIFFERS** - the slot-59 read
  (`GetTopStateType`, `vtable+0xEC`) is not a re-crash guard; it is the **player-drive test**. In
  `Crash_DetermineCase` (`0x00568570`) both crash branches first call slot 102 (`NotifyAutomatonEvent(0xB, 0)`)
  and then read slot 59: if the top state is `0x10` PLAYER_DRIVE the boat is pushed into `0x46`/`0x47`
  (CRASH_RUNAGROUND/CRASH_SINK) with the duration `+0x398` and `final=1`; **any other boat is pushed into
  `0x49` SLOW_REMOVE instead**. The `== 0x10` comparison is present in the bytes; the older Ghidra decompile
  dropped it. automata.md 6.6.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT16:** whether the helicopter `AI_PursueGoal` `default:` really falls through into the `0x32` body.
  [a safety net, inferred] **Probe:** `x86dis` on the jump table at `0x0053BBA0`.~~
  **ANSWERED 2026-10-20 (re-answers33): DIFFERS** - there is **no fall-through**. The dispatch is a byte
  selector at `0x0053C5A0` over the jump table `0x0053C57C`: state `0x32` has its own handler at
  `0x0053C441` (slot 25 called with 0, then `PopState`), and every unlisted type 1-`0x31` plus anything above
  `0x38` jumps to a separate default block at `0x0053C437` that is **`PopState` only** (bytes `8B 06 8B CE
  FF 90 D4 00 00 00`). The decompiler's "default: ... case 0x32:" rendering put the two blocks visually
  adjacent because they share the PopState tail, but they are distinct entries. automata.md 6.6.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT17:** the semantics of automaton vtable slot 102 (Windows) / 109 (Mac), the `(0xB, 0)` crash effect/anim
  kill, and the names of slots 77 (`EnableMotion(bool)`) and 82 (`SetTransform`) - all absent from
  `docs/abi/interfaces.tsv`. [inferred names] **Probe:** dump those vtable entries.~~
  **ANSWERED 2026-10-20 (re-answers33): DIFFERS on two of the three names.**
  - **Slot 102** (Windows `0x0050B540`, Mac 109 = Windows + 7): 2 stack args, `ret 8`. Reads the callback
    target list at `this+0x188`; if empty does nothing; otherwise allocates a `cRZMessage2Standard`, sets the
    message type to **`0x9D92FC3`** (the automaton-notification message the controller manager handles, S10),
    the sender to `this` and the two payload dwords to the arguments, and broadcasts it to every target. The
    crash path's `(0xB, 0)` reading was wrong: in `cSC4Watercraft::HandleEnterState` (`0x0056E40B`) the second
    argument is the **crash duration** (the same value stored to `+0x2C0`), and `FUN_0052F210` passes `(5, 0)`.
    Proposed name: `NotifyAutomatonEvent(u32 type, u32 value)` *[inferred]*. Slot 80 (`0x00505860`) is a
    one-line wrapper that tail-calls slot 102 with `(6, 0)`.
  - **Slot 77** is **`cSC4Automaton::EnableActiveSound(bool)`** (Windows `0x0050C330`, named by the export at
    high confidence; Mac `0x000bfba6` carries the same name). It toggles the `+0x1DC` byte and, when enabling,
    fills the active-sound list `+0x1C8` from exemplar property `0x6C044BB5`. The guessed `EnableMotion` is
    wrong.
  - **Slot 82** (Windows `0x004FB340`) is **`GetFirstModelInstance`** *[inferred]*: 22 bytes returning
    `*(this+0x110)` when the model-instance vector `+0x110..+0x114` is non-empty, else null. The guessed
    `SetTransform` is wrong - `cSC4Automaton::SetTransform` is Mac `0x000c1c68`, a different body that applies
    a `cS3DTransform` to every instance.
  New slot table in automata.md S11.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT18:** what the city query in the blocked-wait exit path means (`DoStopSignTick` step 5:
  `spCity->vtable+0xA0` -> `vtable+0x84(netOccupant, automaton, routeInfo, &out)`; returning 0 pops the wait).
  Whether 0 means "intersection still blocked" or "release failed" is not determinable from the bytes.
  [inferred] **Probe:** name that interface method (likely `cISC4TrafficNetworkMap` / `cISC4TrafficSimulator`).~~
  **ANSWERED 2026-10-20 (re-answers33): DIFFERS** from both guesses. `spCity->vtable+0xA0` is **city interface
  slot 40 = `cSC4City::GetTrafficNetworkMap`** (already in interfaces.tsv), and the method called at
  `vtable+0x84` is **`cSC4TrafficNetworkMap::CheckIntersection`** (`cISC4TrafficNetworkMap` slot 33,
  `0x004B06F0`, named in the manual traffic list). It resolves the automaton's cell and facing (slot `0xEC`
  on the automaton), bounds-checks the cell, looks up the network tile, finds the tile's intersection entry,
  and selects the **approach lane opposite the facing** (`(facing + 2) & 3`); the return value is the **count
  of automata occupying that lane** (0 = free), and the out `f32` receives the wait delay (`FLT_MAX` when
  nothing was found). So returning 0 = the intersection is clear and the wait pops; non-zero = still occupied.
  automata.md 6.5.

<!-- from: From the automata state behaviours and the effect-directory measurements (2026-10-16) -->
- ~~**AT19:** whether `this+0x94` is the measured current speed (written as `distance/dt` in both `DoPathTick`s,
  zeroed at the end of the pull-over move tick), and whether Windows slot `0xA0` in `DoStopSignTick` step 3 is
  `ReturnToDefaultSpeed`. [inferred] **Probe:** `cSC4Automaton::GetSpeed`'s body (slot 34); dump the Windows
  vtable entry at `+0x280`.~~ **ANSWERED 2026-10-20 (re-answers33): PARTLY.**
  - `this+0x94`: written as `|newPos - oldPos| / dt` **only by `cSC4Vehicle::DoPathTick`**
    (`FDIV [ESP+0x50]`, `FSTP [ESI+0x94]` at `0x0055CFBC`) - the "both DoPathTicks" claim was wrong,
    `cSC4TrafficAutomaton::DoPathTick` (`0x0054FDC0`) has no `+0x94` store. `cSC4Vehicle::DoPullOverTick`
    zeroes it (`MOV [EDI+0x94],0` at `0x0055E95C`) as claimed. `GetSpeed` (slot 34, `0x004FB370`) reads the
    **speed tween** (`ADD ECX,0x1A0; JMP TweenStruct::GetCurrent`), not `+0x94`; no tick reads `+0x94`; the one
    consumer found is `cSC4Train::HandleCollision` (`0x00558E00`), which copies the train's `+0x94` onto each
    coupled car before spinning them *[inferred: a derived-speed debug field]*.
  - Windows slot `0xA0` (`+0x280`, `0x0054DA50`) **is `ReturnToDefaultSpeed`**: it reads the max speed
    (slot `+0x1F4` = 125 = `GetSpeedModifier`-adjacent family), computes the tween duration
    `1.0 / maxSpeed x 0.3` (constants `0x00A81228 = 1.0`, `0x00A97D74 = 0.3`; `0.1` when the max speed is 0),
    then `TweenSpeed(defaultSpeed, duration)` - slot `+0x8C` = 35 supplies the default speed and slot
    `+0x90` = 36 is `TweenSpeed`. This matches the Mac `ReturnToDefaultSpeed` (`0x000caa42`,
    `(defaultSpeed, 0.3/m)`).
  automata.md 6.5, S11.
