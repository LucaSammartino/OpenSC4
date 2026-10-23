| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-watercraft-answers5 (watercraft owner answers) | Codex / `re-watercraft-answers5` (RE lane) | ready to land spec work; WC6–WC8 answered, WC1–WC5 open; 90/90 unique spec vectors, 32 added in round 3; diff check passed; no ABI/game runs | 1. Land the spec commits. 2. Trace the remaining WC1–WC5 gaps in watercraft §10.2 before certifying whole-owner closure. |

## Report

Ready to land the completed behavior specs. **The measured goal of complete
cSC4Watercraft owner closure was not achieved.** WC6–WC8 are answered; WC1–WC5
remain open, with the exact outstanding work recorded in watercraft §10.2 and
their original question entries. Ready describes reviewable spec work, not a
claim that an engine lane can close every owner row.

### Changes and milestones

- Prior rounds: `94facf3f` answered WC8 command/steering arithmetic; `1b572698`
  covered Init/Read, docking/undocking boundaries and the attribution inventory;
  `7ecb671b` corrected tuning labels; `7f2c873e` answered WC6 command effects;
  `1f86fb4d` answered WC7 routes/reservations/following; `478e6759` specified
  crash dispatch, double elapsed accumulation and missing-effect failure paths.
- Round 3 `b00cbc6b`: sink/heel angle ramps and strict endpoint behavior,
  distinct runtime/transform sink heights, bump rollback; 8 spec vectors.
- Round 3 `05d3d416`: maximum-extent boat distance gate, SAT contact equality,
  size tags, equal-size double losses, AI victims after player wins, exact bump
  speed transfer and detector returns; 10 spec vectors.
- Round 3 `91eab585`: constructor's eleven populated travel points and allocation
  failure, destructor release ordering, all ten interface adjustors, query/record
  forwarding, sign and vector erase contracts; 6 spec vectors.
- Round 3 `327aae9e`: straight/curved position equations, radius/signed yaw,
  reverse and zero/negative time behavior, scalar banking/bow/model lift and
  publication ordering; 8 spec vectors. Parameter-use labels expanded.
- Final report records partial progress on WC2–WC5 in the open-question file and
  adds the explicit owner-closure boundary table. No partially resolved question
  was moved to answered.

### Measured results and verification

- Spec-side vector IDs: **90/90 unique**, including **32** added this round
  (8 crash scalar + 10 collision + 6 lifecycle + 8 movement).
- Read-only inventory measurement retained: **75 functions / 38073 bytes**,
  not the historical brief's 79 / 38556; the discrepancy remains explicit.
- `python build-runs/validate-watercraft-spec-r3.py`: passed vector counts,
  duplicate check, open-question set WC1–WC5 and single four-column lane row.
- `python tools/questions.py index`: succeeded; regenerated index is unchanged.
- `git diff --check`: passed. No cargo, clippy, ctest or game runs: documentation
  only, with no engine files read or changed. Vectors are prose tables for an
  engine lane to encode; no runnable engine conformance vectors were added.
- Numeric work was checked against export instructions with byte/address
  citations and executable constants. Local export/decomp evidence sufficed;
  no REA request or REA failure occurred. No decompiled output/evidence scripts
  are committed. No ABI rows proposed or C ABI exports changed.

### Questions and next steps

WC6–WC8 remain in `docs/specs/questions/answered/watercraft.md` with their prior
answer markers. No new question IDs were filed. WC1 needs external idle-byte
writers. WC2 needs the animation-false caller and remaining parameter uses.
WC3 needs the remaining method signatures/failures and inherited contracts.
WC4 needs full AI selection, inherited steps, attitude composition, effect/model
details and far approach. WC5 needs model-to-footprint construction, inanimate
detection and complete sink/heel rotation/contact transforms. These were left
open because their behavior was not fully verified in the allotted final round.

Land this lane's spec work, then assign the remaining RE work from §10.2 before
certifying owner closure. The queue entry is `build-runs/merge-queue/re-watercraft-answers5.json`
with kind `re`, state `ready`, no game runs and no ABI change, as requested.
