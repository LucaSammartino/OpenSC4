# dispatch: answered questions (archive)

<!-- from: glue-dispatch-complete4 (2026-10-22) -->
- **DP12 - §4.2 BeginDispatch: complete native vehicle creation/retarget contract.** Publish the exact GetDispatchAutomataProperties argument meanings/layouts, source station-to-spawn position/heading algorithm, automata-manager acquisition and vehicle creation/path slots with all arguments/return channels/ownership, event-target setup, and failure cleanup order. The current text identifies manager slot 17 mode 2 and automaton state/event slots but does not define enough arguments to issue those native calls. [Do not guess an ABI or delegate BeginDispatch to the native dispatch manager; begin_vehicle stays unwired and an actual Dispatch cannot be credited as supported.]
  **ANSWERED 2026-10-22 (re-dispatch-completion4):** DIFFERS — dispatch §§4.2–4.2.1 define source property outputs, tagged descriptors, seven-argument creation slot18, five-argument routing slot17, minimum-entry low byte (new0/retarget2), creation priority255, borrowed manager/return and acquired vehicle/callback/view holds, pathfinder setter/status/cleanup order, station/lot route bounds, precise point-cell operation and first-road path placement/heading. The selector receives the shared cRZRandomSuper object, not a vector. Routing failure mutates flags/deque; wrapper releases creation reference before manager removal. Retarget tag0/null uses current vehicle position and preserves prior return source. Non-traffic rejection leaks an already acquired pathfinder; ordinary routing exits release it. Constants/instruction citations and independent vectors are in the spec. Actual providers and DP10 rendering completion are still required; no replacement promotion.

<!-- from: glue-dispatch-complete4 (2026-10-22) -->
- **DP11 - §§4.1, 4.2, 4.4, 4.5: native pylon construction and placement.** Publish the pylon factory/class ID and IID acquisition/initialization contract, complete callable slot prefix (the SDK omits spec slot 3 AsOccupant and consequently shifts its remaining methods), return channels, ownership and station lot-diagonal placement/return algorithm. Which lot interface supplies the diagonal endpoints, transform and offsets? The spec gives named pylon slots and start + step × (i + 1), but not a native construction path or exact start/step computation and station-return predicate. [No fabricated pylons, positions or undeclared native forwarders; leave unsupported construction/placement callbacks unwired and replacement incomplete.]
  **ANSWERED 2026-10-22 (re-dispatch-completion4):** DIFFERS — dispatch §§4.1.1–4.1.2 publish factory/class/IIDs, actual 21-slot primary prefix, acquired source/owner and borrowed occupant, occupant-cell geometry, single-unit shortcut and byte-wrapped return multiplier. Slot 8 takes a non-null cIGZString interface, copies its explicit length/bytes, and does not retain it. GetCallNumber returns AX only; drag returns AL only. Geometry is supplied by the station occupant and city, with no lot interface/transform. Native citations and independent vectors are in the spec.

<!-- from: glue-dispatch-complete4 (2026-10-22) -->
- **DP13 - §4.5 and protection §8.4: air-source GetStations ownership and unit-state contract.** Does each air source's slot 6 return borrowed station pointers or AddRef every element? Publish both air sources' slot 6/7 arguments, outputs and ownership explicitly. PR8 proves AddRef-on-copy for ground fire/police sources only. Run gdc4-original-20261022-a1 matched independently acquired sources and free counts (police 4, fire 2, air 0), created a real fire dispatch, then failed while releasing an air-source list element after repeated samples. [Only ground providers receive PR8's acquired-reference release; for air providers, do not Release a reference whose acquisition is not published. This conservative literal ownership choice is not complete replacement credit.]
  **ANSWERED 2026-10-22 (re-dispatch-completion4):** DIFFERS — both air sources AddRef every copied station through the same slot-6 function `0x006E0010` and copy helper `0x006DFC70`; balance each exactly once. AL means nonempty. Slot 7 takes `(station, mask*, base*, count*)`, reads/writes count, mask, base in that order and retains earlier writes on failure; exact properties and shift widths are in dispatch.md §4.5.1 and protection.md §8.4. Destination node clearing does not release old elements. Independent distinguishing vectors are specified in dispatch.md §12.

<!-- from: glue-dispatch-complete4 correction (2026-10-22) -->
DP13 evidence correction: a2 failed identically. Inspection of the allowed SDK SC4List implementation showed its destructor already releases non-null elements; the lane's explicit ground release therefore doubled that cleanup. Clearing nodes after audited cleanup fixes this implementation error. Neither failure proves an air-source ownership rule. The original replacement's SC4List destructor already balanced ground station references.
<!-- Archive for [dispatch.md](../../dispatch.md). Lanes do not read this file. Open questions are in ../dispatch.md. -->

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DP1:** dispatch types 2, 3, 4 and 7: no shipped source handles them. [unused in the shipped game; a plugin may
  use them, so FreeCity4 keeps them as plain numbers]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (dispatch.md §2). The four `cISC4DispatchSource` classes handle 0 (police `0x006E0210`), 1 (fire `0x006ACFA0`), 5 and 6 (air sources: slot 3 `0x006DF760` compares `+0x14`, set to 5 at `0x006ACE33` and 6 at `0x006DFFA3`). The Mac symbols list no other `CanHandleDispatchType`.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DP2:** 409 template saves hold `toolType` 4, while the shipped tool's "none" is 8. [written by an earlier build;
  every value loads the same and is overwritten when the tool opens. Nothing more to settle unless a value other
  than 4–8 turns up]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (dispatch.md §7). This build writes `toolType` only with 8 (`0x0046A116`, `0x0046A4F6`, `0x0046A56B`) or with a handled type in `BeginDispatchTool` (`0x0046B0AA`), so 4 can only come from an earlier build. All values load the same.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DP3:** dispatch states: 0 idle, 3 dispatched and 5 working are inferred names; who sets the others? [the automata
  controllers; only state 5 changes `active`. A trace of `UpdateDispatchState` while a fire engine answers a fire in
  a turbo run would list the states in order]~~ **ANSWERED 2026-10-16 (re-answers19):** MATCHES (the automata set
  them; only 5 changes `active`), with the names corrected (dispatch.md §4.3, table with every setter). 1 = route
  blocked (`0x0052F2B2`), 2 = finished at the scene (fire truck `0x00533679`, patrol car `0x00543C46`, helicopter
  `0x00535518`), 3 = on the way (`BeginDispatch`, re-path `0x0052FEDA`), 4 = airplane only (`0x00504621`), 5 = at
  the scene (arrival `0x0052F2A9`), 6 = going home (slot 29 `0x0052F330`, slot 198 `0x0052F570`). State 0 is set
  only by the helicopter and the airplane; a ground vehicle's record ends on event 3 when the vehicle is removed. The
  fire engine's sequence is 3 → 5 → 2 → 6 → event 3, so the §4.7 worked example (which said "state 0") is
  corrected. Setter: emergency vehicle `SetDispatchState` `0x0052FF60` (slot 25), getter `0x00505020` (slot 26).

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DP4:** (cores12a) clearing the available pylons shuts down and releases every pylon. Is that right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (dispatch.md §4.1, "Dropping pylons"). `0x00469E10` walks the vector from the back; each non-null pylon gets `Shutdown` (pylon slot 5, which also releases its source occupant when initialized), then `Release`, then the pop. A pylon that is also the active one is shut down too; the active pointer keeps its own reference.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DP5:** (cores12a) clearing the active pylon is `SetActivePylon(null)`: UI state 1, then release. Is that right?
  [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES for `SetActivePylon` (`0x00469630`: old pylon `SetUIState(1)` and `Release`, no `Shutdown`; new pylon AddRef and `SetUIState(2)`) and for `Init` when loaded. Two other paths do different things. The manager's `Shutdown` shuts the active pylon down and releases it, with no UI state. `EndDispatchTool` does not touch the active pylon at all. Station removal and `EndDispatch` clear it with `SetActivePylon(null)`.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DP6:** (cores12a) returning a pylon with no source occupant only shuts it down; `EndDispatch` releases the
  record's reference. Is that right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** in the letter; the choice is accepted (dispatch.md §4.4). `ReturnPylonToStation` (`0x0046AA20`) shuts such a pylon down **and releases it** (`0x0046ACBA`), and `EndDispatch` then releases it again: an over-release *[quirk]*. It is unreachable in shipped play *[inferred]*, so the single release is the safe *[choice]*. Separately, a null pylon or a null dispatch source makes the return do nothing at all.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DP7:** (cores12a) when a station is removed, its vehicles are untracked and despawned but not released. Is that
  right? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** **DIFFERS** (dispatch.md §3). For each record of the station, `DoMessage` (`0x0046A320`) untracks the vehicle in the vehicle view (when the view exists), calls automaton slot 13 (`Remove`, with 0) **and releases the record's automaton reference**. The pylon is cleared from active if needed, shut down and released; the station is released and the record erased. The available pylons of that station are then shut down, released, cleared from active and erased.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DP8:** (cores12a) does `RevokeDispatchByPylon` return the pylon to its station only when some source handles the
  pylon's dispatch type? [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES in effect (dispatch.md §4.4). `0x0046AE70` always calls `ReturnPylonToStation(GetDispatchSource(p.GetDispatchType()), p)`, also when no record holds `p`, and the return does nothing when that source is null. AL is `RevokeDispatchByID`'s result for the first record holding `p`, else 0. `RevokeDispatchByID` erases the record whatever `EndDispatch` returned.

<!-- from: From the city situation, demolition, dispatch, port, prop developer and city detail specs (2026-10-14) -->
- ~~**DP9:** (cores12a) a re-targeted vehicle that already has a view handle is not tracked again. Is that right?
  [yes]~~ **ANSWERED 2026-10-15 (re-answers15):** MATCHES (dispatch.md §4.2 step 5). On re-targeting, `BeginDispatch` (`0x004698F0`) calls automaton slot 25 (`SetDispatchState(3)`), automata manager slot 17 (a new path, mode 2) and automaton slot 39 (event target) again. It does not track again, so the record keeps its handle. It then sets the pylon's dispatch state from automaton slot 26 and posts **begin** again with the same ID.
