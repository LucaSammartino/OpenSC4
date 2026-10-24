| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-nettool-nt32 (NT32–NT34 behavior) | Codex / `re-nettool-nt32` (RE lane, detached HEAD) | ready; NT32–NT34 answered; doccheck PASS; questions check PASS; cargo/clippy/ctest n/a; no game runs; no C ABI changes | 1. Integrate callable observations and straight-prefix semantics in glue-nettool-exact6. 2. Compare nonflat solve inputs and grading writes; retain city mismatches. |

## Report

Answered the three questions copied from the unmerged glue-nettool-exact6
question file into [the answered archive](../specs/questions/answered/network-tools.md).
The source lane's file has Report and Handoff sections but no Owner closure
table or `open:` rows. Its Handoff and the complete NT32–NT34 question entries
defined the closure scope.

- NT32: [network-tools §8.3.1](../specs/network-tools.md)
  specifies saved pricing counts surviving structural record release. Added
  test 38 for repeated release and next-placement reset. Corrected the older
  current-vector-size equation and stale float32 rounding in network-geometry §11.
- NT33: network-tools §8.3.1.1 specifies renderer PickTerrain slot 61,
  primary IID, pop20, AL, origin/direction/hit XYZ storage, literal zero
  arguments, false/unavailable distinction and finite-segment examples.
  The final classifier probe is lot-manager GetLot(x,z,false), slot 6,
  pop12, borrowed pointer EAX, not a generic terrain blocked method.
  +0x74 is the straight-step prefix, 13 for the fresh 13-step axis fixture;
  corrected the stale power-line-tool terminology. Added tests 39–40.
- NT34: network-tools §8.1.3 supplies nonflat initial terrain and piece/range
  constraints, ordered network and terrain solutions, grading writes, refresh
  corners, getter-time terrain and earthwork contributions 12/4/0. Added test 41,
  infeasible-support and fixed-and-placed variations. These are export-derived
  constraint-boundary fixtures, not measured native city snapshots.

Verification: `python tools/doccheck.py check` PASS and
`python tools/questions.py check` PASS. Regenerated docs/specs/QUESTIONS.md.
Independent arithmetic/hash-order assertions for the nonflat fixture PASS;
`git diff --check` PASS (only Git's standard LF/CRLF notice).
No Rust/native checks or game runs are appropriate to this spec-only change.
No REA calls were attempted because the brief states there is no binary open;
all evidence came from main-checkout re_query export and bounded decomp reads,
with numeric/ABI claims checked against instruction bytes and cited in the spec.
No engine, plugin, capi or coverage-map files were read or edited.
Existing docs/abi rows were read for interface identity/cross-checking only;
none was edited.

Proposed ABI rows are in `build-runs/abi-rows-re-nettool-nt32.tsv` (local,
gitignored): renderer slot 61 clarification and lot-manager slot 6 clarification.
The latter notes that its optional true validation checks the lot origin,
not general bounds containment. No C ABI export changed.

The source run `nettool-exact6-r3-20261024-7c9f` remains the engine lane's
reported clean 8/14 result, not a run performed here. Preserve native/core
110/113, 170/192, 90/98 and 55/61. The representative replay target 273.388092
versus native getter-time terrain 271.987152 cannot identify a unique missing
native constraint. The spec gives a stage-by-stage diagnostic boundary and
does not fit targets, suppress repeated corners or assert city exactness.

## Handoff

- Goal: close NT32, NT33, NT34 from glue-nettool-exact6 with behavior and conformance vectors.
- Done: eaa21326a NT32; 91e68e9a3 NT33/NT34 and cross-spec corrections.
- Done: doccheck and questions check PASS; index regenerated; final report written.
- Next: coordinator can land this detached commit and copy proposed ABI rows if desired.
- Then implementer can wire renderer/lot reads and compare the explicit nonflat stages.
- Preserve four city mismatches and missing model/placement scopes; no game request outstanding.
- Proposed ABI rows: build-runs/abi-rows-re-nettool-nt32.tsv (not tracked).
