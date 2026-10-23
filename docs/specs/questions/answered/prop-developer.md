# prop-developer: answered questions (archive)
<!-- Archive for [prop-developer.md](../../prop-developer.md). Lanes do not read this file. Open questions are in ../prop-developer.md. -->

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**PV1:** the prop developer's night batch size (`+0xA34`) is not initialised; 225 shipped records hold
  `0xBAADF00D`. [never read while the night mode is 0; *[choice]* FreeCity4 writes 0 for a new city and writes a
  loaded value back unchanged. A new city saved by the original would show what the GOG build writes]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice confirmed; prop-developer.md §2). The constructor never writes `+0xA34`, so the original saves whatever the heap block held (`0xBAADF00D` is the debug heap's fill *[inferred]*); there is no fixed value to match, and the field is never read while the mode is 0.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**PV2:** a tagged prop whose tag names no object of its lot's configuration lowers the block's slot count by one.
  Which props have such tags in practice? [props left over from a different configuration (a lot redeveloped with a
  new configuration, or a lower detail level before the city detail manager's reduce); reproduced. Counting, in a
  corpus city, lot props whose tag matches no object of their lot's configuration would show how common it is]~~ **ANSWERED 2026-10-16 (re-answers19):** MATCHES (prop-developer.md §10 quirk
  1). The tag is the configuration object's ID (`+0x24`), saved with the prop. The one source the bracket lacks is a
  configuration of version < 2: its objects get generated IDs from a per-process serial (`0x00B2166C`,
  `ConvertPropertiesToObjectArray` `0x00479D10`), so its props' tags match nothing after the next load. That cannot
  happen with shipped data: all 1908 lot configurations and all 543 network lot configurations are version 2 (a scan
  of `SimCity_1.dat`…`SimCity_5.dat` and `EP1.dat`). So the bracket's sources are the shipped ones; the detail-level
  case lasts only until the city detail manager's reduce at `Init`. The corpus count was not made (it needs the
  saved prop records with their tags).

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**PV3:** `UpdateAllOptionalProps` (slot 7) has no caller. [none in the exe; FreeCity4 implements it as specified
  (every block, x outer). A plugin scan or a call trace in a long turbo run would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (prop-developer.md §4.2). No direct call targets `0x006F4DB0`; it is reachable only through vtable slot 7. Implementing it as specified is harmless.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**PV4:** the block step runs on every Idle delivery, also while paused, so props develop at a frame-rate-dependent
  speed. Does the original really grow optional props while paused? [yes, from simulator.md §5.4. In a turbo run,
  pause a fresh city with zoned growth for 60 s of real time and log `0x99EF1142` for props carrying `0x89A1C16C`:
  posts while paused confirm it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES. The simulator delivers Idle whenever `hiddenPauseCount ≤ 0`, with data2 = (pauseCount > 0) (simulator.md §5.4), and the prop developer's Idle handler ignores the message data (prop-developer.md §3), so player pause does not stop the block step.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**PV5:** the requester index stored on a prop is not bounds-checked, and `RemovePropRequester` shifts later
  indexes. [only pollution removes its requester, at `Shutdown`; an index ≥ the requester count cannot occur in the
  shipped game. *[choice]* FreeCity4 treats it as "no index". Nothing to settle in game]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (choice confirmed). Static: only pollution removes its requester, at Shutdown; nothing to settle in game.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**PV6:** a power change switches night-lit **street** props at once but leaves night-lit **lot** props until the
  next dusk, dawn or switch. [reproduced, from the exe. At night in a turbo run, cut the power line to a lit block and
  watch a lit lot prop and a lit street lamp: the lamp goes dark at once, the lot prop at dawn]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (reproduced from the exe, prop-developer.md). What remains is a visual check, not a behaviour question.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PV7:** a night batch decides on or off with the mode it started with, so the batch that ends a dusk sweep (mode
  becomes 0) still switches its props on. Does the original read the mode before the end-of-sweep reset?~~ **ANSWERED 2026-10-16 (re-answers22):** MATCHES (prop-developer.md §4.8). The batch (`0x006F0EC0`) reads `mode = 1` into a flag at `0x006F0EF6`–`0x006F0F04`, before the end test sets the mode to 0 (`0x006F0F53`), and decides on or off from that flag; the batch that ends a dusk sweep switches its props on.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PV8:** §4.4 step 6 does not test the created occupant for null. [a null creation returns false: the slot is
  consumed and need is not decremented]~~ **ANSWERED 2026-10-16 (re-answers22):** MATCHES the bracket; the spec was wrong (prop-developer.md §4.4). The original does test the created occupant: null → return false (`0x006F18C6`), so the free entry is consumed and `need` is not decremented.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PV9:** a timed prop with a non-zero global trigger while the 3D view is missing: the spec covers only the
  view-present and zero cases. [mask 4 unchanged: neither on nor added to the trigger map]~~ **ANSWERED 2026-10-16 (re-answers22):** MATCHES (prop-developer.md §4.6). With a non-zero trigger and no 3D view (global `0x00B43DC8` null, or QueryInterface `0xFA47B3F9` failing) the code jumps past the step (`0x006F3D24`, `0x006F3D3A`): mask 4 unchanged, no trigger-map entry.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PV10:** a total share of 0 would divide by zero (§4.3 step 4). It cannot happen with the wealth requester present,
  but can after a Read of a record without it. [stop after the cell walk]~~ **ANSWERED 2026-10-16 (re-answers22):** **DIFFERS** from the original, *[choice]* confirmed (prop-developer.md §4.3). The split is an unsigned `div` by the total (`0x006F4A69`); a total of 0 is a divide fault that ends the original. Stopping after the cell walk (its selector draws and removals done, nothing of steps 4–6) is a safe *[choice]*.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PV11:** §4.13 step 1 (night-lit street props in the power-change rectangle) does not say which cell IsPowered is
  asked for. [trunc(position / cell width), as in the batch]~~ **ANSWERED 2026-10-16 (re-answers22):** MATCHES (prop-developer.md §4.13). The power-change handler asks `IsPowered(trunc(x / trunc(cellWidthX)), trunc(z / trunc(cellWidthZ)))` from the prop's position (`0x006F2491`–`0x006F25EE`), the same cell as the night batch.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PV12:** unregistering (§4.7) looks the prop up in every list whatever its properties, and Shutdown clears the
  power list ([choice] per §3.2). Does the original test each list's join condition instead of looking everywhere?~~ **ANSWERED 2026-10-16 (re-answers22):** **DIFFERS** for unregistering (prop-developer.md §4.7); the Shutdown *[choice]* is confirmed (the original releases the power list without clearing it). `HandleRemoveOccupant` (`0x006F2FF0`) searches each list only when the prop passes that list's test: night-lit = 1 and a prop occupant; trigger present, non-zero and the 3D view present; the time-of-day, date and power properties present (values not read); the two sign types. The visible difference from looking everywhere: a trigger-map prop is not removed (and stays held) when the 3D view is gone at its removal.

<!-- from: From the city detail, port developer and prop developer cores (2026-10-16) -->
- ~~**PV13:** a scanner state byte above 2 in the record: what does the original do? No corpus record has one. [Read
  returns false and nothing changes]~~ **ANSWERED 2026-10-16 (re-answers22):** **DIFFERS** (prop-developer.md §7). The original stores the state byte as read (`0x005DA410`, no check), and `GetNext` (`0x0060C8D0`) treats every value other than 0 and 2 as running without rewriting it; so a state above 2 behaves as 1 and is written back unchanged until the scan finishes or resets. `Read` must not fail on it.

<!-- from: From the enable=/spec-contradiction wave (filed by the coordinator from the implementer lanes' reports, 2026-10-16; answered by re-answers31) -->
- ~~**GLUE21-2** (`enable = propmgr`): the game reads `0x7C` from a null (`esi = 0`) at
  `SimCity 4.exe+0x2F44EF` on the prop path (slot 15 `CreateProp`). Should `CreateProp` / `Add*Prop` fail the
  call instead of returning a null?~~ **ANSWERED 2026-10-16 (re-answers31): DIFFERS.** Neither. The original
  never returns a null: `cSC4PropManager::CreateProp` (`0x004A3040`) validates the type, tries
  `CreateObject(cls, 0xE9793A65, &out)`, and on failure **allocates and constructs the occupant itself**
  (`0x94` / `0xA0` / `0x68` by class family), returning the `+4` sub-object. Written into
  [prop-developer.md](../../prop-developer.md) **§13**.

<!-- from: glue-weather-propdev-shadow5 (2026-10-23) -->
- ~~**PV14 - sections 2, 5, 7: record-only same-input shadow RNG contract:** How may a record-only observer copy the original prop developer's live member `cRZRandomSuper` state (`+0x38`), the wealth requester's file generator, and the C `rand()` state used by night shuffles into independent shadow generators without advancing or changing the original? Section 7 saves none of these states; section 5 documents clock-based seeding and shared generators, and foundations section 6.4 suggests forcing seeds, which changes the original and does not meet this lane's record-only requirement. Please specify a version-guarded, read-only snapshot layout/API (including table/cursor positions and shared-generator timing), or a documented alternative that supplies the same random inputs without native writes or duplicate requester calls. The existing `FC4PropDevShadow.cpp` uses fixed `kShadowSeed`, deliberately does not compare creations or the identity of random removals, and its `HookSelect` substitutes candidate IDs instead of calling stateful custom selectors. Clarify how those selector results can also be captured without drawing twice. [Blocked: retain existing default-off shadow; no guessed native layouts/signatures, no exact-through-2032 claim.]~~
  **ANSWERED 2026-10-23 (re-propdev-pv14):** DIFFERS — a serialized-record copy and fixed seed cannot supply the live random inputs. [prop-developer.md](../../prop-developer.md) §5.1 specifies the guarded `0x9C8`-byte generator layout, full 624-word copy, relocated next-word index and remaining count; §5.2 identifies the original calling thread's CRT state at runtime-data +`0x14`, observable during its genuine `rand` call. Shared generators require operation-local snapshots or complete intervening-use accounting. §5.3 permits actual random-return transcripts instead and records each real custom selector's arguments/return once at both kept-check and creation call sites, replaying it without another native invocation. Eight conformance vectors are in §12.15–§12.22. These are observation contracts, not a claim that the blocked shadow has implemented or measured exact equivalence.
