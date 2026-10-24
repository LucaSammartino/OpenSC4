| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-weather-boot8: timing-independent weather shadow bootstrap | Codex, `glue-weather-boot8` | in progress | Reproduce train183, fix native date stepping, validate to 2032 |

## Handoff
- Goal: weather shadow exact 73/73 through 2032 with scres_file_shadow on and off.
- Worktree reused, detached HEAD; initial tree clean.
- Baseline verify: build PASS 249/249, CTest PASS 78/78, logs in build-runs/fc4verify-{build,ctest}.log.
- Reproduced weather-boot8-before-on-02 2/27 (run 439): February exact steps=4; March mismatches=54102 byte=108. Clean ref_warnings=0, time limit ended 2028-03-24.
- Next: collect diagnostic verify session 99589, request diagnostic run with seconds=900, inspect first weekly CRT/date log, fix bootstrap/date stepping.
- Diagnostic source logs first actual record difference even if CRT counters already differ; logs first 48 weekly delivery/native dates and CRT states.
- No game request outstanding. Diagnostic verify qbuild-20261024-065309-build-ctest running; session 99589 must be collected.
- weather-boot8-before-on-01 rejected (submitted before DLL existed); no questions filed.
- Do not read or edit tools/; coordinator must handle any regress profile update.
