| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vehicles-vh11: vehicle owner contracts | `Codex` / `re-vehicles-vh11` (RE lane) | wip: VH9/VH10/VH11 answered; VH1/VH5/VH7 open; questions/doccheck/diff checks pass | 1. Complete VH7, VH5 and VH1; 2. validate and enqueue ready |

## Report

This continuation closes **VH9** at `944898267`, following VH10/VH11 at
`28fe893ed`. Vehicles.md §12.6 now specifies RANDOM_WALK, MAGNET/controller
attraction and base dispatch slots; VH9 is archived. Eight new vectors supplement
the existing nine inherited-state vectors. Section 12.9 adds verified partial
VH7 findings and four more vectors. **VH1/VH5/VH7 remain open.**

RANDOM_WALK uses signed walk/rest timing and repeated double-range turn draws.
Native controller attraction is MAGNET (18), with no distinct ZOT dispatcher
case, and displacement has no dt factor. Base dispatch setters discard call
number/state; getters return zero/3. Geometry findings cover route replacement
failure, signed steps, endpoint equality, overshoot, half-radius front anchors
and unnormalized direction averaging. No full engine owner closure is claimed.

The questions index is regenerated. Questions/doccheck/diff checks passed after
VH9 and again after §12.9 (terminal exit 0). Commit `024f179a7` contains the
partial geometry and regenerated index. All vectors are specification text,
not executed engine tests. Cargo/clippy/CTest, game runs and shadows n/a.
No C ABI changes, proposed ABI rows or engine/plugin/capi/coverage-map edits.
REA was not invoked; only local export, decomp and read-only PE evidence used.

Source lookup failures are distinct from gate results: qgrep hit a Unicode
encoding error; two GetSpeedModifier lookups used the wrong class file. Corrected
UTF-8 source windows found it in unnamed region_00550000.c. The milestone commit
succeeded despite a shared packed-refs.lock warning; no workaround was attempted.
Measured context reached 122177/150000 before final documentation. Research
stopped then, exceeding the requested two-thirds bound; incomplete answers are
left open explicitly. The merge queue remains wip pending those answers.

## Prior round report

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

- Goal: finish VH1/VH5/VH7 without engine reads/edits; VH9/VH10/VH11 closed.
- Done: `28fe893ed` VH10/VH11; `944898267` VH9; `024f179a7` partial VH7 §12.9 and regenerated index; questions/doccheck/diff PASS.
- Next exact step: establish Traffic Read's adjusted this pointer from 00554D20 entry before naming record fields; finish VH7 reconstruction, then VH5/VH1. Partial geometry is now §12.9.
- Main source root C:/Users/tlori/FreeCity4. Existing decomp `_unnamed/region_00500000.c` lines 4997–5162 holds RANDOM_WALK; ZOT candidate 00508E10 is at line 2617 (confirm state mapping before naming).
- VH5: Train animation source cSC4Train.c starts line 402, StartTrip line 679; unnamed region_00550000.c composition helper 00558510 starts line 4103. These were only inspected, not closed or numerically certified.
- VH1: cSC4Vehicle.c PlayerDriveUpdateSpeed line 1321, off-road 1406, full tick 2003; cSC4Train.c derail 208 and full tick 911; cSC4Tank.c commands 330. Numeric instruction verification remains essential.
- VH7: slot 131 Traffic 0054E920 / Vehicle 0055D210; slot 132 Traffic 00550760 / Vehicle 0055D5A0. Advancement 005504E0; tick 0054FDC0; gaps 00551300/0055D9D0; speed modifier 0055DF50. Remaining: record names, route third byte, lane equality, bridge/slope factors and complete transitions.
- Read uses EDI+1E4 for serialized network ref, while live methods use +1EC. Verify interface adjustment before inferring a contradiction. Helper build-runs/source-window.py uses UTF-8 function-relative windows, max 60 lines/4 KB; default PowerShell decoding shifted line counts.
- TrainCar placement override 00557320 is at unnamed region_00550000.c line 3864. cSC4TrainCar.c lacks its body.
- Initial capped output misleadingly appeared to show an orphaned VH9 tail; actual question file retains all four originals; do not rewrite them.
- Worktree starts detached at 0490d3c97; commit normally, never update refs.
- Source root: C:/Users/tlori/FreeCity4; manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-vehicles-vh11-r2-manifest.json.
- No ABI changes or game runs planned. Date for answers: 2026-10-24.
- Key evidence: slot 134 = 00550B60 (table A9C068 + 218); slot 158 = 0055B690. Shared car reference helpers 005D96C0/005D9730; append 0055AB50–0055ABB0.
- Ignored build-runs/pe-values.py reads PE words/constants; no source content generated or committed. All public spec evidence is address citations and prose.
