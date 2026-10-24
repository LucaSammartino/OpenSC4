| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vehicles-vh11: vehicle owner contracts | `Codex` / `re-vehicles-vh11` (RE lane) | wip: VH10/VH11 answered, 13 vectors; doccheck/questions/diff checks pass; VH1/VH5/VH7/VH9 open | 1. Complete remaining four contracts; 2. implement verified Init/reference rules in core |

## Report

Partial result: this round closes **two of six requested questions**, VH10 and VH11.
It does not establish full closure of any of the four engine owners. The remaining
VH1/VH5/VH7/VH9 entries stay open in `docs/specs/questions/vehicles.md`.

Commit `28fe893ed` adds vehicles.md §12.7 and §12.8, corrects the misleading
drive-parameter description in §12.3, archives both answers, and regenerates
the questions index. Eight Init vectors and five reference/aliasing vectors
are specification requirements, not executed engine conformance tests.

Measured native contracts:

- Slot 158 (`0055B690`) is one double-range draw narrowed to lateral offset
  `+0x200`; it does not initialize drive parameters.
- Fresh Init slot 134 is `00550B60`, independently verified from the constructor's
  primary table `00A9C068` and table word `00A9C280`. It selects/project a path from
  current position and does not consume a route front. It is distinct from
  BeginRoute (`00554AB0`). Traffic Init ignores its false result.
- Train references resolve to shared runtime objects during Read, querying car
  IID `4A416254`. Missing refs are skipped; duplicate refs alias the same runtime
  object. There is no later train-specific identity binding phase.
- The generic writer queries the same object's serializable interface; a null
  object or failed interface query is passed to the shared stream as null.

Verification: `python tools/questions.py index` regenerated the index;
`python tools/questions.py check` **PASS**; `python tools/doccheck.py check`
**PASS**; `git diff --check` **PASS**. The single operation running these checks
and the milestone commit returned terminal exit 0 (session 54071).
Cargo/clippy/CTest: n/a, spec-only. Game labels and owner shadows: n/a.
No C ABI exports changed; no proposed ABI rows; no engine/plugin/capi edits.
REA was not invoked: no binary was open, as stated in the brief. Evidence came
from re_query, existing decomp, and read-only PE words/constants. No decompiled
code, assembly listings, binary material, or helper scripts are committed.

Failed edit attempts are separate from gate results: the first spec patch had
an incorrect final-row context and applied nothing; a later combined patch
used an unsupported delete/add pair and applied nothing. Corrected patches
produced the committed spec. An attempted question-file restoration was
unnecessary and failed context matching; inspection confirmed the original
four open entries already existed, unchanged. Git commits succeeded despite
the shared `packed-refs.lock` maintenance warning; no workaround was attempted.

The requested research bound reserved the final third of the lane budget for
validation/reporting. Source investigation stopped near measured 97k of 150k;
the broad physics/composition/geometry/state contracts remain unfinished.
The queue is therefore **wip**, rather than asserting all six were answered.

## Handoff

- Goal: finish VH1/VH5/VH7/VH9 without engine reads/edits; VH10/VH11 already closed.
- Done: `c7a38bbff` initial plan; `28fe893ed` two answers, 13 vectors, index, passing checks.
- Next exact step: query numeric instructions for RANDOM_WALK at 0050CF90, then finish VH9 including ZOT and base dispatch slots; §12.6 already covers countdown/PLOP_JUMP/ATTACH_ANIM.
- Main source root C:/Users/tlori/FreeCity4. Existing decomp `_unnamed/region_00500000.c` lines 4997–5162 holds RANDOM_WALK; ZOT candidate 00508E10 is at line 2617 (confirm state mapping before naming).
- VH5: Train animation source cSC4Train.c starts line 402, StartTrip line 679; unnamed region_00550000.c composition helper 00558510 starts line 4103. These were only inspected, not closed or numerically certified.
- VH1: cSC4Vehicle.c PlayerDriveUpdateSpeed line 1321, off-road 1406, full tick 2003; cSC4Train.c derail 208 and full tick 911; cSC4Tank.c commands 330. Numeric instruction verification remains essential.
- VH7: unnamed region_00550000.c slot 131 at line 5981 (0055D210), placement slot 132 at 6084 (0055D5A0), traffic path tick 00550760 at line 145; none closed this round.
- TrainCar placement override 00557320 is at unnamed region_00550000.c line 3864. cSC4TrainCar.c lacks its body.
- Initial capped output misleadingly appeared to show an orphaned VH9 tail; actual question file retains all four originals; do not rewrite them.
- Worktree starts detached at 0490d3c97; commit normally, never update refs.
- Source root: C:/Users/tlori/FreeCity4; manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-vehicles-vh11-r1-manifest.json.
- No ABI changes or game runs planned. Date for answers: 2026-10-24.
- Key evidence: slot 134 = 00550B60 (table A9C068 + 218); slot 158 = 0055B690. Shared car reference helpers 005D96C0/005D9730; append 0055AB50–0055ABB0.
- Ignored build-runs/pe-values.py reads PE words/constants; no source content generated or committed. All public spec evidence is address citations and prose.
