# city-situation: answered questions (archive)
<!-- Archive for [city-situation.md](../../city-situation.md). Lanes do not read this file. Open questions are in ../city-situation.md. -->

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CS1:** what does 3D view slot 57 do when the situation popup is shown (`0xEBB67EF7`)? [hides the mission status
  panel]~~ **ANSWERED 2026-10-14** in this lane from the exe (3D view vtable slot 57, `+0xE4`,
  `HideMissionStatusPanel`) and written into city-situation.md §3.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CS2:** the argument meanings of city situation manager slots 22–25 (`InitiateSituationForOccupant`,
  `InitiateSituationByAutomataGroup`, `StartJoyride`, `TreatAutomatonAsGroup`). [the symbol port's names; the
  argument counts are verified from the pops (8, 12, 12, 8 bytes); nothing FreeCity4 replaces calls them. A call
  trace of the four slots while the shipped situations run (car chase, UFO, joyride) in a turbo run would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** Answered from the exe and the Mac build's symbols (city-situation.md §8, slots 22–25). Slot 22 `(cISC4Occupant*, cISC4CitySituationAdvice*)`: QI for the automaton, then `InitiateSituation(advice or the first matching active entry, automaton, true)`. Slot 23 `(uint32 group, bool, bool search)`: starts the active entry that accepts the group, else (only when `search`) looks for the group's automata. Slot 24 `(cISC4Automaton*, bool, cIGZMessageTarget2*)`: sets `justActivated`, posts `0xABBBCA64`, enters the drive view. Slot 25 `(cISC4Automaton*, uint32 group)`: sets the automaton's group and tracks it. MATCHES: nothing FreeCity4 replaces calls them.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CS3:** who posts `0x2C16BDE7` (the situation popup without an advice)? [no code in the executable; only a script
  or a plugin could. A turbo run that triggers every shipped situation should log 0 posts of `0x2C16BDE7`]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES. The ID's four bytes occur once in the whole executable, at the handler's compare (`0x0052EE1F`), so no code and no data table can post it (city-situation.md §3). Only a script or plugin could.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CS4:** (cores12a) tick step 4 runs only while the status is 3; elapsed time grows by `t × 0.001` s in `double`;
  a time limit above 0 gives status 2 when reached; the 30-second warning fires once. Is that right? [as stated]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** on three points (city-situation.md §4.4). (1) Step 4 runs on every tick with a mission held, at any status (also 4 and 5). It is skipped only in a tick that handles `justActivated`, decrements a countdown that stays > 0, or decrements a positive `endDelay`. It does run in the tick in which the countdown expires. Within it, the built-in success checks and the clock act only at status 3, but the evade-list check, the Lua check (`+0xAE4 ≠ −2`, called whatever the status), the warning and the pursuit update run at any status. (2) `elapsed := elapsed + t × 0.001f` is `float` arithmetic (`0x0052CA4A`, constant `0x00A867A4`; 24-bit x87), not `double`. (3) The limit fires only when `limit ≥ 1.0f` and `limit ≤ elapsed`, so a limit in (0, 1) never fires. The warning MATCHES: it fires at most once (the flag is cleared by `InitiateSituation` and `ActivateSituation`), when `limit ≥ 30.0f` and `limit − 30.0f ≤` the pre-increment `elapsed`, through active automaton slot 61 with no null test.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CS5:** (cores12a) does `InitiateSituation` return false only for "could not start" (status 5)? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** (city-situation.md §4.3). It also returns false, with nothing changed, when the advice is already the active one (null included). For a null advice it deactivates the current mission and returns false with no popup. And without `showBriefing`, a successful start still sets status 5 and posts `0xEBB67EF7(advice, 0)`, but returns **true** (`0x0052E8E3`) *[quirk]*. The slot-21 caller then activates (status 3), so the queued popup opens the briefing window *[inferred]*.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CS6:** (cores12a) `DeactivateSituation` sends `0xABBBCA64` (data1 0), releases the advice, resets to status 4, ends a
  running countdown and unfreezes the automata; `EndSituation` is the same call. Is that right? [as stated]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** (city-situation.md §4.3.1). `DeactivateSituation` (`0x005294C0`, only with a mission held) does the send (synchronous), the release and status 4, but it does **not** touch the countdown, `endDelay`, the time limit, `elapsed` or the indicators. The automata are unfrozen only when the countdown runs out on a later tick. `EndSituation` (`0x00529ED0`) is a separate call. It releases the `+0xA9C` effect, sets the time limit to 0, and stops and releases the countdown effect, though not the countdown value. With a mission held, it untracks the active vehicle from the advice's entry and clears it, then calls `DeactivateSituation`. It always calls `ShowAllIndicators(4, indicatorsVisible)`.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CS7:** (cores12a) with max CSI reached, a situation does not take over another entry's vehicle. Is that right?
  [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES in effect (city-situation.md §4.2). The original has a takeover path, used at the limit or with no candidate. It scans the entries **after** the new one for a holder of strictly lower advice priority (`0x00524A50`) and moves that holder's vehicle and handle across. But every insert re-sorts the list ascending by priority (`0x00527920`), so no later entry has a lower priority unless priorities changed since the sort. The path is dead in shipped play *[inferred]*.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CS8:** (cores12a) `failure_money` is deposited only when it is non-zero, like the success money. Is that right?
  [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (city-situation.md §4.5). The test is in the advice's getter: slots 48 and 51 (`0x004237B0`, `0x00423820`) return false for 0, and the manager deposits only on true (budget slot 7). A negative amount is a withdrawal that the budget refuses when funds are too low.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CS9:** (cores12a) the outcome aura position is the active vehicle's, else the current target's, else no aura. Is that
  right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** for success (city-situation.md §4.5). Failure matches: the active automaton (slot 7), else the target (occupant slot 8), and with neither it skips the effect, the aura **and the money** (`0x00526771` → `0x00526931`), running only the failure script. Success (`0x0052C190`) has no target fallback and no skip: the position is the active automaton's, else the zero vector `0x00B227E0` (0, 0, 0). The effect, the aura (when radius > 0.0f) and the money then all happen.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CS10:** (cores12a) automaton notify subtypes 0xB, and 0xC with data3 0x14, drop the automaton from tracking like a
  removed occupant. Is that right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** (city-situation.md §4.7). Both go to `HandleAutomatonDisabled` (`0x0052C020`; 0xC with any other data3 returns true). It first sets automaton flag `0x10000000` (slot 30), which also makes `CanPlayerDrive` refuse it. With a mission held, the active automaton gives status 2 whatever the current status. Otherwise the target's automaton gives "next target" (types 1 and 3; status 1 when none is left) or status 2 (other types), followed by `HandleStatusChanged`. Only then is a tracked automaton dropped as for a removed occupant.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**CS11:** (cores12a) slots 23–25 are left inert. Blockers: slot 23's search path (`0x00525640`) is not traced;
  slot 24 posts `0xABBBCA64` without naming the IGZUnknown (which advice?); slot 25 "tracks the automaton
  (`0x005291C0`)" without saying whether that is the §4.2 indicator tracking. [inert]~~ **ANSWERED 2026-10-15 (re-answers15):** all three traced (city-situation.md §8 rows 21–25). Slot 23: when no entry accepts the group and `search` is set, `0x00525640` collects the group's automata from the four automata managers (slot 30 tests the group, slot 32 collects). The first one `CanPlayerDrive` accepts is taken; otherwise `CreateAutomatonForPlayerDrive` (`0x005292F0`), and with none, false. Then `StartJoyride(automaton, true, null)` and true. Slot 24 **sends** `0xABBBCA64` with data1 = 1 and **no** IGZUnknown (a joyride names no advice), then enters the drive view; status and active advice are untouched. Slot 25 runs `HandleInsertAutomaton` (`0x005291C0`), the `0x99EF1142` handler's body: yes, the per-automaton side of the §4.2 indicator tracking (`TrackAutomaton`, kind 4, listener). Inert slots **DIFFER**; the three can now be implemented.

<!-- from: glue-citysit4 (2026-10-21) -->
- ~~**CS13 - §4.1 step 5 / §4.6: how the `sitmgr` table is published.** "Create the Lua table `sitmgr`" names seven functions and two variables bound to `+0xA5C` / `+0xA60`, and lua-host.md §8.5 lists `sc4game.sitmgr.*` as used by the shipped scripts. Is it published like the budget's (budget.md §4.4) through `cISC4GameDataRegistry` (a node `"sitmgr"` under `GetRootNodeEntry()` keyed by the manager, the functions as call entries with flag 1, the variables as exchange entries), or as a plain Lua table on the held `cISCLua`? For each of the nine names: entry kind, key, argument conversion and result (`get_current_target` / `get_active_auto`: the occupant as `cIGZUnknown` variant, or a number?; `show_CSI`: a setter, a getter, or both?), and what `Shutdown`'s "remove the `sitmgr` table" removes. A call entry writes its results into the output `cIGZCommandParameterSet`, whose slot table is not in docs/abi or any spec: please add it (at least the calls a call entry needs to write one result). [not published by the FreeCity4 replacement: `sc4game.sitmgr` is absent while `enable = citysit` is on]~~
  **ANSWERED 2026-10-27 (re-x-sims):** DIFFERS — `sitmgr` is a node of the game-data registry (service 0xEBE0E860, IID
  0xE898ED03), not a table made on the held Lua interpreter: node keyed by the manager as cIGZUnknown, seven call
  entries (key = callback address, flags 1, context = manager) and two read-only exchange entries (flags 2) for the
  fields at +0xA5C/+0xA60, removed on Shutdown with RemoveChildEntry(manager). Each call callback writes one variant
  to output parameter 1: success count uint16, get_current_target/get_active_auto cIGZUnknown (no manager test),
  get_target_name string, num_targets_remaining uint32 (1 without a manager), show_CSI a bool that is both setter and
  getter (returns the old value), activate_situation a bool (§4.6.1, §4.1 step 5). The output parameter set's
  SetParameter row already exists in docs/abi/interfaces.tsv.

<!-- from: glue-citysit4 (2026-10-21) -->
- ~~**CS12 - §2 `+0xA2C`, §4.1 step 7, §4.2, §4.3.1 step 4, slot 29: the indicator view's slots.** The manager holds the dispatch manager's slot-22 object (dispatch.md §8 names it `GetDispatchVehicleView`, a `cSC4DispatchVehicleView`) and calls `ShowAllIndicators(4, visible)` and `TrackAutomaton(automaton, kind 4, listener)` / untrack on it. Neither spec gives that view's interface: its IID (if `cISC4DispatchManager` slot 22 returns a class pointer rather than an interface), and the slot, argument count, callee pop and return channel of `ShowAllIndicators`, `TrackAutomaton` (and its untrack form: the same slot with `false`?) and of the handle it returns (the `trackList` "indicator handle"). Please add the vehicle view's slot table (at least those calls) to dispatch.md §8 or city-situation.md §8. [the FreeCity4 replacement does not call the view: `SetCSIVisible` and `Init` change only the stored flag, no indicator is drawn or hidden]~~
  **ANSWERED 2026-10-27 (re-x-sims):** DIFFERS — dispatch manager slot 22 returns the concrete view object, not an
  interface, and the city situation manager calls it directly rather than through slots: ShowAllIndicators 0x0046D3A0
  (kind, show; callee pops 8, no result used), TrackOccupant 0x0046F240 (occupant, kind 4, instance, null label;
  handle in EAX) and UntrackOccupant 0x0046EFB0 (handle, immediate false; callee pops 8, AL). TrackAutomaton takes the
  view fresh from the dispatch manager each time, while Init and SetCSIVisible use the held +0xA2C view (§4.2
  "Indicator arguments", dispatch.md §4.1.3).

<!-- from: advisors-exact33 (2026-10-28) -->
- ~~**CS16 - §4.2 indicator arguments.** Dispatch §4.1.3 now documents the direct native boundary (`TrackOccupant(occupant, kind, number, label)` and `UntrackOccupant(handle, immediate)`), but city-situation §4.2 only specifies kind 4. What number and optional label does citysit pass, and does its untrack use immediate removal? The replacement currently uses 0/null/false, marked SPEC-GAP. Tracking, advice enumeration, the four managers, callbacks and the tuning limit are now wired; these argument defaults still need a compatibility contract.~~
  **ANSWERED 2026-10-27 (re-x-sims):** DIFFERS — the indicator call is TrackOccupant(occupant = the automaton's slot-5
  object, kind 4, instance = the first non-zero indicator instance at +0x68 of the automaton's occupant-group template
  or its parents via +0x0C, label null), and the untrack is not immediate (UntrackOccupant(handle, false)); nothing is
  tracked when no instance is found (§4.2 "Indicator arguments").

<!-- from: advisors-exact33 (2026-10-28) -->
- ~~**CS18 - §4.6 `get_success_count` / `activate_situation`: string GUID conversion (CS13 follow-up).** The registry mirror converts Lua strings to RZCharArray variants (lua-host §9.3), and the first twelve actual `get_success_count` inputs in the Jinae fixed schedule all have that type (`0x800E`). Numeric conversion yielded GUIDs such as 0, 2, 4, 6 and 8, so it cannot identify the missions. What is the original string-to-GUID conversion: hexadecimal with an optional `0x` prefix, or another format; and what happens for malformed strings or numeric arguments? The implementation now chooses hexadecimal strings and truncates numeric Float64 arguments, marked SPEC-GAP. Please also confirm the success-count result's variant type and the optional briefing default. Evidence: qcheck-18.log, Jinae run compare-20261028-035655-2829c710; remaining advisor activation flags differ for GUIDs 4C142F8A, 4C14354A, 8C03EC4A, AC143545, AC1726C8 and CC1435A1. No callback for the original manager is forwarded.~~
  **ANSWERED 2026-10-27 (re-x-sims):** DIFFERS — a string argument (variant type 0x800E) is parsed as base-16 text
  with the C library's unsigned conversion (white space, sign and 0x prefix accepted, stops at the first non-digit, no
  digits give 0, overflow clamps to 0xFFFFFFFF); any other variant type is read as an unsigned 32-bit value and a
  failed read gives 0, so a numeric argument is the GUID itself. get_success_count answers a uint16 variant and 0 for
  GUID 0 or a missing argument. The briefing bool of activate_situation has no default in the callback (§4.6.1).
