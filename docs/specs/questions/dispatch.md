# dispatch: open questions

2026-10-22 evidence update (re-dispatch-completion4): DP13 is answered and archived. DP10's
construction, subobject adjustments and direct tracking boundary are in dispatch §4.1.3;
renderer Update/refresh and delayed-untrack completion remain open. DP11 is answered in
§§4.1.1–4.1.2, including string-interface copying and the narrow getter return channels.
DP12's complete caller-side descriptor/slot argument layout and order are in §4.2, including
the null source occupant on retarget. Internal station-to-road spawn/heading and full route
failure/ownership protocol remain open. These partial contracts confer no replacement credit.
<!-- Append new questions at the end; IDs continue docs/specs/QUESTIONS.md. -->

<!-- from: glue-dispatch-complete4 (2026-10-22) -->
- **DP10 - §§2, 4.1, 4.2, 8 slot 22: native vehicle-view provider contract.** What are the complete construction/factory, IID/subobject, renderer registration/unregistration, refresh, TrackAutomaton and UntrackAutomaton contracts of cSC4DispatchVehicleView (argument layouts, ownership, return channels and slots/nonvirtual entry points)? The spec publishes size 0xA0 and layer 5/priority 1000, but no callable view interface; the public SDK has no such interface. A replacement must also serve external callers of manager slot 22. [Leave view callbacks unwired; no complete-owner or supported replacement credit. Do not delegate to the original dispatch manager or invent a view.]

<!-- from: glue-dispatch-complete4 (2026-10-22) -->
- **DP12 - §4.2 BeginDispatch: complete native vehicle creation/retarget contract.** Publish the exact GetDispatchAutomataProperties argument meanings/layouts, source station-to-spawn position/heading algorithm, automata-manager acquisition and vehicle creation/path slots with all arguments/return channels/ownership, event-target setup, and failure cleanup order. The current text identifies manager slot 17 mode 2 and automaton state/event slots but does not define enough arguments to issue those native calls. [Do not guess an ABI or delegate BeginDispatch to the native dispatch manager; begin_vehicle stays unwired and an actual Dispatch cannot be credited as supported.]

