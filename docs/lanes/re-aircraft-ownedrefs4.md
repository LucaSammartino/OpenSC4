| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-aircraft-ownedrefs4: AT24 aircraft retained references | Codex / `re-aircraft-ownedrefs4` (RE docs; detached worktree) | ready; AT24 native contract answered; diff check passed; no ABI/game/code changes | Coordinator: apply native phase contract to existing warning evidence; runtime classification remains bounded below. |

## Report

Resolved only the native AT24 contract in `docs/specs/automata.md` §§5.2.1–5.2.2. Moved the original
question and dated DIFFERS answer into `docs/specs/questions/answered/automata.md`, and regenerated
`docs/specs/QUESTIONS.md`. AD3/AD4/AT22/AT23 were not answered. No engine, tools, maps, ABI-table,
audit-macro, coverage or default edits; no implementation work or runtime promotion.

### Proven behaviour

- Version-5 Read retains one QI-acquired airport and one QI-acquired automaton reference per valid
  unique map pair. The helper releases temporary segment/serializable interfaces, not those results.
- A plane also in the base list has a separate retained reference. PreCityShutdown shuts down and
  releases that list entry while leaving the map pair alive after segment/table disposal.
- Aircraft Shutdown `0x004FB7D0`: ascending unsigned pointer-key order, airport then airplane per
  entry, free/reset map, clear runways, release filter, base Shutdown (which clears the base list).
- Airport removal: plane slot 13(false), erase, plane Release then airport Release. It does not run
  plane Shutdown at that site and contributes no later map release.
- Final manager Release `0x00507B50` invokes virtual Shutdown before deleting destruction. Deleting
  destructor `0x004FBCF0` frees storage without releasing map payloads. Explicit repeated Shutdown
  and final Release cannot repeat the payload pass after the initialized byte has been cleared.
- City calls aircraft Shutdown/Release at `0x0045B272`/`0x0045B27D`, before occupant-manager Shutdown
  at `0x0045B833`. Its map cleanup does not depend on later occupant shutdown notifications.

### Evidence and validation

Used main-checkout `tools/re_query.py` export-first queries, paginated instruction bytes, vtable data
references and bounded decomp reads. Checked the actual constructor, Init, Read/helper/QI chain,
map comparator/node destruction, Shutdown, final Release, deleting/base destructors, base clear,
message dispatch, runtime insertion/removal and city teardown. Pseudocode stack recovery for Read
and insert is broken; ownership and receiver offsets are taken from instructions instead. No REA
query was necessary. q.py Bash execution failed with E_ACCESSDENIED; bounded PowerShell/Python
reads worked. Git emitted a packed-refs.lock permission diagnostic while detached commits still
succeeded; shared refs/config were not changed.

Seven independent probe families are specified, not executed: segment disposal; pre-shutdown list
clear; Shutdown without PreCityShutdown; external plane owner; two-key traversal; removal/miss;
explicit versus final-Release-entered Shutdown. Their event signatures discriminate retention and
phase/order errors independently of the three-warning trace. Documentation diff check passed and
question index regenerated. Cargo/clippy/ctest and new game/shadow runs: n/a for this docs-only lane.

### Runtime residual and coordinator next step

Provided controls `train84-ports-union-reload-20261022-a` and
`train84-ports-union-control-20261022-a` both reach 2033 with three airplane warnings. They establish
an existing common path only. A zero-count airplane Release during the specified manager Shutdown
is legitimate. Retaining its map reference until member destruction after Shutdown differs from
native, and a destructor-only ownership annotation is not established by these controls or the
controller-manager precedent. The existing generic HostRelease:53 cannot be adjudicated from its
location alone. Needed evidence is its manager phase, exact airport/plane receiver identities,
base-list/map membership and before/after counts with segment/table/other owners accounted for.
No warning suppression or promotion is authorized. This lane did not read implementation files.

Milestones: `674ee4c0` initial lane handoff; `f88bc7cf` bounded source/spec/question answer.
Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-aircraft-ownedrefs4-r1-manifest.json`.
No ABI proposal file; C ABI unchanged. No external communications, subagents, merges or pushes.
