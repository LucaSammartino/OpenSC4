| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mysims-ms20: dispatch owner closure | Codex, `re-mysims-ms20`, detached HEAD (supervisor publishes) | ready; MS18/MS20 answered; doccheck passed; no ABI exports changed | Implement the specified handoffs and fixtures; live replay requires complete input capture. |

## Report

Extended `docs/specs/mysims.md` with §4.6.3–§4.6.5 route priority/mask decisions,
idle/stuck recovery, FE/FF and same-mode behavior, neighbour/model boundaries and
automaton/mission callbacks. §7.2.2 specifies input/pylon lifecycle; §7.2.3 publishes
the actual native primary interface, ownership, constructor guarantees and observation
limits. Added 21 prose conformance vectors in §12.29: 18 MS18 cases (one covers all
42 unequal ordinary-mode pairs) and three MS20 cases. Together with the five existing
MS18 vectors, these specify 23 MS18 cases. No engine files were read or changed.

MS20 was absent from this base; its entry and the blocked owner's uncompared-contract
list were read from the reserved `glue-mysimdispatch-shadow9` worktree. Both MS18 and
MS20 are archived as ANSWERED/DIFFERS in `docs/specs/questions/answered/mysims.md`;
`python tools/questions.py index` regenerated `docs/specs/QUESTIONS.md`.

Important measured facts: nine primary dispatch slots, not a virtual queue/clock
interface; no public observer of all requests or universal inactive-dispatch discovery.
Neighbour retry always fails due to an unpopulated list. Near FE arrival writes the
duration field. Same-mode continuation can still clear queued destinations via idle
fallback. These are documented original behaviors, not proposed corrections.

Evidence: local export queried first with `tools/re_query.py`; bounded reads of main
checkout decompilation; numeric/ABI quirks checked against instructions with bytes;
primary vtable words checked read-only in the working executable. REA was not used
(no binary open); no REA error/fallback occurred. No decompilation, executable data,
assembly listings or helper code is committed. No proposed ABI rows or C ABI changes.

Validation: `python tools/doccheck.py check` passed; `git diff --check` passed.
Cargo/clippy/CTest are n/a for this specification-only lane. No game runs requested
or performed; no shadow or whole-owner verification credit claimed. The engine lane's
constructor-fixture run remains separate evidence and does not verify these handoffs.

Milestones: `8d6458366` investigation record; `27070a0d4` route/recovery/callback spec;
final commit adds interface/lifecycle/vector answers, question archive/index and this report.

## Handoff

- RE task complete; next owner is coordinator/engine implementer.
- Use §12.29 as independent fixture contracts; capture request/service/callback inputs before live replay.
- Do not guess missing native slots or promote constructor-only shadow results to whole-owner verification.
- Detached HEAD is intentional; supervisor publishes commits to the reserved lane branch.
- Ignored helper/evidence paths: `build-runs/read-vtables.py`, commit messages in `build-runs/`.
- No pending game requests, newly filed questions, ABI proposals or engine changes.
