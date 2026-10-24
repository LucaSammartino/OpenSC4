| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-weather-we1: weather rounding contract | Codex / `re-weather-we1` (RE, detached HEAD) | ready; WE1 answered; doccheck and question checks pass | Engine replay using exact wind and moisture rules |

## Report

Closed WE1, including its moisture continuation, in `docs/specs/weather.md` §5.1.1, §5.1.2 and §6.5.1.
Archived the engine lane's original question and all evidence continuations in
`docs/specs/questions/answered/weather.md`; regenerated `docs/specs/QUESTIONS.md` (weather: 0 open, 1 answered,
next WE2). The requested source question was imported into this worktree, not written into the main checkout,
in accordance with this lane's edit boundary.

The daily contract uses normal 24-bit x87 arithmetic, round-to-nearest-even at every arithmetic operation;
the fraction multiplies by binary64 1/30 without rounding the operand first. Normalization uses one rounded
reciprocal followed by two component products. Final speed is interpolated separately from vector magnitude.
Moisture has an additional contract omitted by the old summary: the accessor re-normalizes stored direction,
using speed/length when length exceeds 0.01f. Documented all local slope rounding and integer bias conversion.
No engine, plugin, capi, ABI table, coverage-map or tool changes; no proposed ABI rows.

Evidence: exported instruction bytes for `DoMessage` at `0x00729720`, `GetAmbientWind` at `0x00725A00`,
`ScaleVector2D` at `0x005D76D0`, and `SimulateMoisture` at `0x00726BF0`; addresses accompany each rule.
Constants were checked directly in the read-only original executable. Used existing decompilation only for
orientation; no REA request (the session has no binary), and no decompilation or assembly listing is committed.

Added instruction-derived bit-pattern vectors in §14 items 13–15: daily interpolation, reciprocal versus
division (day 2 x `3F7F64B5` versus `3F7F64B4`), single versus double 1/30 (day 8 t `3E6EEEEF` versus
`3E6EEEF0`), zero-vector interpolation, accessor threshold/rescaling and nearest-even moisture ties.
Local exact-rational calculator and PE constant checks are in ignored `build-runs/we1-arithmetic.py`.

Validation: `python tools/doccheck.py check` PASS; `python tools/questions.py check` PASS;
`git diff --check` PASS. Cargo, clippy and CTest are n/a for this specification-only lane.
No game runs were requested. Runs 374–377 are inherited question evidence (weather 1/73, not exact), not
validation of these rules. A corrected independent replay must determine whether all later moisture mismatches
are resolved; this lane makes no verified-coverage or byte-exact game claim.

## Handoff
- Goal complete: WE1 and moisture continuation have exact behavior and conformance cases.
- Milestones: `7a66701e0` lane initialized; `9625c2208` imported question; `204910d66` arithmetic spec and archive.
- Validation: doccheck and question checks pass; no engine gate or new game verdict.
- Next: coordinator lands docs; engine lane applies §5.1.1/§5.1.2/§6.5.1 and replays without resynchronization.
- No outstanding game request. Runs 374–377 remain inherited non-exact evidence only.
- Detached HEAD is intentional. Supervisor publishes commits to reserved lane branch; queue uses actual HEAD.
