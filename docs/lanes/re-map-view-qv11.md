| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-map-view-qv11: map-view owner questions | `Codex` / `re-map-view-qv11` (RE lane, detached HEAD) | ready; round 1 ac706d691, c04bcb9b0; round 2 QV4/QV6/QV9 answered; doccheck passed; ABI unchanged; no game runs | 1. Land docs. 2. Implement the six answered closure rows using the spec vectors. |

## Report

Round 2 closes QV4, QV6 and QV9 in map-view.md §§1.1, 2.1, 4.1, 4.3 and 7.1. Added ten conformance
vectors (24–33), moved the three questions to questions/answered/map-view.md marked DIFFERS, and
regenerated QUESTIONS.md. Together with round 1, all requested QV4/QV6/QV7/QV8/QV9/QV10 are answered.
No new questions or ABI proposals.

Measured native behavior:

- E8 selects lot types with a mask. Inclusive byte means round halfway upward and never output zero
  for selected lots. Mask zero still forces interpolation.
- Crime incidents draw ordered native-size icons with rotation, clipping and integer alpha blending
  divided by 256. Incident visuals flags do not filter them.
- Layer 5 computes signed population divided by unsigned built capacity, multiplied by 100; zero
  denominator yields zero. Geometry converts before the ratio, and stored results narrow to a word.
- Missing fields reset or retain individually. Absent ramp retains palettes; absent travel list retains
  count/bytes. Travel count narrows to a byte before the eight-entry cap; entries retain low bytes.
  Invalid terrain values retain the entering mode. Resource identity addition wraps at 32 bits.
- Property access is one holder lookup with unchecked variant aliases. Wrong types/allocation faults
  do not create a universal false/rollback protocol. Serialized duplicate resolution belongs to the
  holder/resource contract; the map consumes the returned property once.
- Initial view is none. Init's early latch prevents repair on a second call after missing-server failure;
  repeated Hook calls accumulate message IDs and subscription attempts.
- CRT clock is wall-time milliseconds, low 32 bits; eligibility uses unsigned wrapping subtraction.
  Active Update clears dirty at entry. Missing child advances clock, missing developer does not;
  ordinary empty sources replace with a cleared bitmap, none with null. Partial normal returns do not
  restore dirty/clock. Notifications compare supplied old/new record words, without a stored baseline.

Evidence: main export queried first with re_query.py, then bounded decomp reads and instruction-byte
checks. Owners/helpers: 0079FEF0, 0079E600, 007A2080, 0079DDE0/0079DDF0, 0068C270, 006B4710,
007A0D50, 0079E400, 007A17B0, 007A49B0, 007A2F60, 007A54D0 and CRT 009F361F. Read constants
A81054=0.0f and A80AA0=100.0f from the main executable. No decompilation, assembly listings or game
assets committed. REA was not invoked because the brief says no binary is open; no REA error/fallback.
The only engine read was the explicitly requested owner closure README.

Validation: questions index regenerated; `python tools/doccheck.py check` passed
(`build-runs/doccheck-r2.log`); `git diff --check` passed. Cargo/clippy/ctest n/a for spec-only changes.
No engine/plugin/capi edits, C ABI exports, game requests or game runs; no shadow equivalence claimed.
Unrelated questions QV1/QV2/QV3/QV5/QV11/QV14 remain open.

## Handoff

- Goal: six requested owner questions; all six now answered with rules and vectors.
- Done round 1: ac706d691 and c04bcb9b0, QV7/QV8/QV10, vectors 17–23.
- Done round 2: QV4/QV6/QV9, vectors 24–33; questions moved, index regenerated; doccheck passed.
- Next: coordinator lands detached-HEAD work; implementers consume the updated spec.
- No outstanding game requests, ABI proposals or implementation changes.
