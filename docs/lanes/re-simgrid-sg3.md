| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-simgrid-sg3: simulator owner grid inventory | Codex / re-simgrid-sg3 (reserved detached worktree) | ready: SG3 answered; doccheck and questions checks pass | Land spec; resume glue-simgrid-lots-shadow5 against the bounded owner/getter scope. |

## Report

Answered **SG3** in [simgrid.md](../specs/simgrid.md) §6.2.1–§6.2.2. The owner/getter inventory specifies
element types, slots/selectors, new-city dimensions and IDs, loaded-geometry rules and sampling lifetime.
The shipped configuration has 38 non-growth roles plus four grids for each of twelve growth developers:
**86 expected roles**. BDS's borrowed developer map supplies discovery for the developer subset; no global
class-store or stream-probe enumeration is assumed. The coordinator's explicit scope excludes map-view,
transient, scratch/synthesized and owner-private grids. Current replacement owners remain in scope through
the same getters. A complete sample establishes only this scope, not every saved/live city grid.

Corrected §6.1 and §6.4's blanket borrowing statement: residential population/EQ/HQ triples AddRef all three
outputs. Plumbing's published object changes on a pointer swap, so every observation must fetch it again.
Pollution selector 3 returns radiation source, not its unused value grid; garbage keeps its packed high bit.
Weather moisture and power consumption are raw vector objects, and flora has no SimGrid. These do not add
§5 record roles. Fire/police coverage getters return uint8 despite SDK signed-16 declarations.

Added six prose conformance vectors (§10, 19–24): default role count/alias handling, geometry and loaded
geometry, selectors/signedness/packed bits, reference balance, plumbing publication swaps, and discovery
changes/missing-role reporting. These are implementation requirements, **not executed conformance tests**.
No engine, plugin, capi, tools/coverage-map.tsv, ABI tables or C ABI exports changed.

Evidence: queried the main checkout's local export with re_query first, then read relevant existing owner
spec sections. Verified instruction bytes for grid dimension shifts, owner getters, selector dispatch,
developer-map address, supply-map aliases and all three residential reference-counting getters. Citations
are in the spec. No REA call was made: the task states that session has no binary open; no REA result or
fallback failure is claimed. No original game or decompilation content was committed.

The requested engine lane report and SG3 question were absent from both this worktree and main, but found
in the reserved sibling worktree `glue-simgrid-lots-shadow5/docs/` (report milestone `375f8115`). Its report
contains no Owner closure table or `open:` rows; the documented discovery blocker is SG3 itself. The exact
question is archived in [answered/simgrid.md](../specs/questions/answered/simgrid.md), marked ANSWERED
2026-10-23 (re-simgrid-sg3), DIFFERS because the coordinator bounded the requested universe. Regenerated
QUESTIONS.md. SG1 and SG2 remain open/parked, unchanged. If the sibling lane's open SG3 is merged later,
retain the archived answer and remove that duplicate open entry before regenerating the index.

Validation: `python tools/doccheck.py check`, `python tools/doccheck.py links`, `python tools/questions.py
check`, and `git diff --check` passed. Cargo, clippy, CTest and game checks are n/a for this spec-only lane;
no game-run labels, shadow exactness result, or measured game coverage is claimed. No proposed ABI rows.
Milestones: initial investigation `62fce3f0`; SG3 answer/spec/index `196f1be9`; final report follows.

Next: coordinator lands this RE result and resumes the blocked shadow lane; implementers add the specified
inventory and six conformance vectors, then measure game comparisons at PostCityInit and monthly. This
lane does not implement or verify the shadow.

## Handoff

- Ready for landing; no RE question remains within SG3's bounded scope.
- No game requests outstanding. SG1/SG2 are outside this task.
- Resume implementation from simgrid §6.2.1–§6.2.2 and §10 vectors 19–24.
- Queue uses the actual detached HEAD, kind re, ABI false, no game runs.
