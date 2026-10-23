| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vehicles-vh5b (vehicle owner behaviour) | Codex / `re-vehicles-vh5b` (reserved detached worktree) | READY: answered work can land; VH2/VH4/VH6 answered; questions check and doccheck pass | VH1/VH5/VH7/VH9 remain open; full owner closure is not claimed |

## Report

Round 2 delivers native behaviour specifications only. Commit `ef1e3e774` completes vehicles §12.5 for VH2
and adds partial inherited-state rules in §12.6 for VH9. The final report commit fixes the pathfinder section
cross-reference. No engine, plugin, capi, tools, coverage-map or ABI files changed. No ABI rows are proposed.

VH2 is archived in `docs/specs/questions/answered/vehicles.md`, marked DIFFERS on 2026-10-24, and the questions
index is regenerated. The neighbour selector ranks status-1 path results by route-entry count, preserves the
earlier tie, uses single-cell path requests, and returns a network occupant looked up at the winning cell.
The spec separates that result from the retained neighbour-connection candidate. Ordinary and explicit-route
look-ahead have distinct fallback and terminal behaviour. Explicit expansion consumes its first route triple
even when the subsequent connection check fails. The four requested slots now have input/output contracts;
the directional order supplied by the foreign network/simulator services is an explicit interface boundary.
Conformance vectors cover ranking, clamping, lookup failure, packed words, exit ordering, terminal fallback,
consumption on failure, look-ahead target count and SlowDown's duration.

VH9 remains open. Its partial delivery specifies strict timeout expiry, controller precedence, full-delta
dispatch after a pop, PLOP_JUMP's floor comparison and retained velocity, and ATTACH_ANIM's model acquisition,
property ramp, selector exclusions, frame completion and saved return bool. In particular, a timeout reaching
exactly zero does not expire later, and a failed attached-model creation leaves the pushed state present.
The hop's gravity applies at equality with the floor; clamping strictly below it leaves velocity unchanged.
Additional conformance vectors cover these boundaries. RANDOM_WALK's complete controller/model movement,
the requested ZOT contract and base-versus-emergency dispatch hooks still require further native evidence.

VH1 (full vehicle/train/tank driving), VH5 (train composition/following/trips), VH7 (route/geometry owner
contracts), and VH9 remain in `docs/specs/questions/vehicles.md`. No new question IDs were filed. This READY
entry means the answered documentation can land, as requested for Round 2; it does not close these owners.
Round 1's VH4/VH6 answers remain delivered in §12.3/§12.4, including the 20000 ms Train collision timeout.

Validation: `python tools/questions.py index`, `python tools/questions.py check`,
`python tools/doccheck.py check`, and `git diff --check` passed at the specification milestone. Questions and
doccheck are repeated after the final report/cross-reference edit. Cargo, clippy, CTest and game/shadow are n/a
for this specification-only lane. No game runs were requested or remain pending.

Evidence came from absolute-path queries against the main checkout's export and bounded reads of decomp/.
The executable's mapped data verified literals and existing vtable targets through the ignored PE helper.
Numeric operations were checked against `re_query.py asm --bytes` and cited in the spec. No REA query was
attempted because the brief explicitly says no binary is open. The safe `docs/lanes/core-vehicles5.md` report
was read first; its actual closure table lives under forbidden engine/ and was not opened. Git's shared
packed-refs.lock diagnostic recurred, but the detached commit succeeded; no shared ref/config repair occurred.

Broad native investigation ended at approximately 100000/150000 measured context (last pre-final-query
reading 97939), respecting the requested two-thirds investigation limit. The remainder was reserved for
specification text, conformance vectors, archive/index maintenance, checks and delivery.

## Handoff
- Land this answered work using the READY queue entry; do not describe the four remaining questions as closed.
- Done: Round 1 VH4/VH6; Round 2 VH2 and partial VH9 in `ef1e3e774`; final report/cross-reference commit follows.
- Open: VH1/VH5/VH7/VH9 in `docs/specs/questions/vehicles.md`.
- VH1 next: native `005603f0`, `0055ea20`, Train `005591a0`, Tank `0054b030`; missing pseudocode blocks need asm.
- VH5 next: Train `00558510`, slots 195–199, StartTrip `005588d0`; composition/following/trip details unresolved.
- VH7 next: path advance/placement and Read reconstruction; no general route-geometry closure claimed here.
- VH9 next: finish RandomWalk geometry/model/controller rules and dispatch accessors; identify requested ZOT precisely.
- Reuse §12.6's verified timeout/hop/attachment contracts; do not repeat the old floor-equality shorthand.
- Spec §12.5 closes selector/four named slots; service-provided directional ordering remains a foreign interface input.
- No REA binary open; use main checkout export/decomp by absolute path. Never read engine/.
- Worktree stays detached; supervisor publishes commits. No game requests or proposed ABI rows outstanding.
