# dispatch: open questions

2026-10-22 final round 3/3 (re-dispatch-completion4): DP11, DP12 and DP13 remain answered
and archived. DP10's construction, adjustments, process-global handles, tracking, Update gates,
refresh, texture keys and delayed-untrack completion are specified in dispatch §4.1.3.
This round adds the number-texture slot/ownership/failure protocol, intermediate number/icon
quads, borrowed screen-rectangle getter/storage, exact conflict/relocation/fallback algorithm,
and final 2D draw submission/order with independent distinguishing vectors.
**PARK DP10; no fourth round.** The precise remaining addresses/scope are at §4.1.3's
“DP10 final-round residual”: final camera/2D vertex values, full visible-entry preparation,
kinds 3/4 generation and alternate world-space/3D branches. Preserve the worktree.
No complete-owner, complete view-provider, supported replacement or exact-shadow credit.
<!-- Append new questions at the end; IDs continue docs/specs/QUESTIONS.md. -->

<!-- from: glue-dispatch-complete4 (2026-10-22) -->
- **DP10 - §§2, 4.1, 4.2, 8 slot 22: native vehicle-view provider contract.** What are the complete construction/factory, IID/subobject, renderer registration/unregistration, refresh, TrackAutomaton and UntrackAutomaton contracts of cSC4DispatchVehicleView (argument layouts, ownership, return channels and slots/nonvirtual entry points)? The spec publishes size 0xA0 and layer 5/priority 1000, but no callable view interface; the public SDK has no such interface. A replacement must also serve external callers of manager slot 22. [Leave view callbacks unwired; no complete-owner or supported replacement credit. Do not delegate to the original dispatch manager or invent a view.]

