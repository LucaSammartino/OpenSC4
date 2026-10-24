| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-vehicles-vh11: vehicle owner contracts | `Codex` / `re-vehicles-vh11` (RE lane) | ready: partial answers may land; VH9/VH10/VH11 answered; VH1/VH5/VH7 remain open | Land accumulated spec answers; resolve narrowed VH7 load mapping, then VH5/VH1 |

## Report

Final round, 2026-10-24: **ready to land the partial specification**, as requested.
VH9/VH10/VH11 remain answered and archived from earlier commits. This round adds
188 lines to vehicles.md §12.9, with 16 additional conformance vectors, and
narrows VH7 to specific unresolved evidence. **VH7 is not fully answered; VH5
was not attempted in this round; VH1 remains open with a precise missing-evidence
note.** No full engine-owner closure is asserted.

New measured contracts: route X/Z handling and preserved third byte; live path
key fields; masked equality OR for leader lane matching; reattachment failure
and placement protocols; Read's absence of geometry queries; endpoint and
leader-gap thresholds; route-end removal/hold behavior; network, turn and signed
slope speed factors. Signed downhill asin with ordinary positive tunings clamps
to zero rather than producing a downhill bonus. Record receiver offsets are
reported separately from live layout: the network reference is serialized at
receiver +0x1E4, while live placement uses +0x1EC. The Read thunk subtracts four,
and Vehicle::Read forwards its receiver unchanged. These observations alone do
not establish a save defect or an eight-byte layout adjustment.

Verification: questions index regenerated; `python tools/questions.py check`,
`python tools/doccheck.py check`, and `git diff --check` all passed (terminal
exit 0). Vectors are requirements in prose, not executed engine tests.
Cargo/clippy/CTest n/a; game runs/shadows n/a. ABI exports unchanged; no proposed
ABI rows. No engine/plugin/capi/coverage-map edits. Sources were the local export,
bounded decomp windows and read-only working-PE displacement checks; no REA
request was made because the session has no binary open.

Research stopped at measured context 95449/150000, within the requested
two-thirds bound. The final third is reserved for validation and handoff. Source
lookup failures were corrected: q.py could not start Git Bash (Win32 error 5),
so subsequent commands used PowerShell; source-window initially received a 0x
prefix it does not accept; re_query search does not support --skip. These were
lookup failures, separate from the passing documentation gates.

Queue destination: `build-runs/merge-queue/re-vehicles-vh11.json`, kind `re`,
state `ready`, actual full detached-HEAD tip. The supervisor publishes that tip
to the reserved lane branch; this round does not switch branches or update refs.

## Previous continuation report

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

- Goal: land the ready partial answers; remaining research must not delay this queue entry.
- Done: `28fe893ed` VH10/VH11; `944898267` VH9; `024f179a7` initial geometry; final round extends §12.9 and narrows VH7/VH1.
- Next exact research step: trace serialization interface receivers into 005552E0/00554D20 and compare their semantic field ownership with live 00550760; locate later load-to-live reattachment.
- Verified: Read body does not adjust receiver; known thunk subtracts 4; Vehicle::Read passes receiver unchanged. Read/Write access +1E4 for network ref and +224 for deque, live placement +1EC and +22C. Do not invent a +8 correction or claim a save defect.
- Route cells are X/Z plus preserved byte; inspected geometry ignores byte 3; producer-side meaning still missing.
- VH5 source pointers: cSC4Train.c animation line 402, StartTrip line 679; unnamed region_00550000.c helper 00558510 line 4103. TrainCar placement override 00557320 is in that unnamed file line 3864.
- VH1 source pointers: cSC4Vehicle.c speed line 1321, off-road 1406, full tick 2003; cSC4Train.c derail 208/tick 911; cSC4Tank.c commands 330. Confirm UTF-8 line numbering and instruction widths.
- Source root C:/Users/tlori/FreeCity4; helper build-runs/source-window.py expects address without 0x, UTF-8, capped windows. No REA binary open.
- Manifest C:/Users/tlori/FreeCity4/build-runs/run4-re-vehicles-vh11-r3-manifest.json; research stopped at measured 95449/150000.
- Detached HEAD: commit normally, never switch branches/update refs. Queue must use actual full HEAD and state ready, kind re, ABI false, game_runs [].
- Questions/doccheck/diff PASS; no engine tests or game runs; no ABI changes/proposals.
