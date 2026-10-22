# dispatch: open questions

2026-10-22 round-2 evidence update (re-dispatch-completion4): DP11 and DP12 are answered
and archived; DP13 was answered in the prior round. DP10 construction, adjustments, tracking,
renderer Update argument/gates, refresh, texture keys and delayed-untrack completion are in
dispatch §4.1.3. Complete call-display texture generation, screen rectangles and draw submission
remain open. No complete-owner, supported replacement or exact-shadow credit is granted.
<!-- Append new questions at the end; IDs continue docs/specs/QUESTIONS.md. -->

<!-- from: glue-dispatch-complete4 (2026-10-22) -->
- **DP10 - §§2, 4.1, 4.2, 8 slot 22: native vehicle-view provider contract.** What are the complete construction/factory, IID/subobject, renderer registration/unregistration, refresh, TrackAutomaton and UntrackAutomaton contracts of cSC4DispatchVehicleView (argument layouts, ownership, return channels and slots/nonvirtual entry points)? The spec publishes size 0xA0 and layer 5/priority 1000, but no callable view interface; the public SDK has no such interface. A replacement must also serve external callers of manager slot 22. [Leave view callbacks unwired; no complete-owner or supported replacement credit. Do not delegate to the original dispatch manager or invent a view.]

