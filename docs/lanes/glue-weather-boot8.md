| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-weather-boot8: timing-independent weather shadow bootstrap | Codex, `glue-weather-boot8` | in progress | Reproduce train183, fix native date stepping, validate to 2032 |

## Handoff
- Goal: weather shadow exact 73/73 through 2032 with scres_file_shadow on and off.
- Worktree reused, detached HEAD; initial tree clean.
- Baseline verify: build PASS 249/249, CTest PASS 78/78, logs in build-runs/fc4verify-{build,ctest}.log.
- Reproduced weather-boot8-before-on-02 2/27 (run 439): February exact steps=4; March mismatches=54102 byte=108. Clean ref_warnings=0, time limit ended 2028-03-24.
- Diagnostic verify PASS 47/47 build edges, CTest 78/78. Diagnostic run weather-boot8-diag-on-03 2/4 (run 440), clean/ref_warnings=0 to 2026-04-02, first difference step=7 byte=107. February steps=5 exact but still diverges, unlike the task's suggested cause.
- Next: collect diagnostic verify session 63866, request diagnostic run -04 to April; new unconditional delivery and observer-arm failure logs distinguish CRT observer failure from missing date steps.
- Diagnostic source logs first actual record difference even if CRT counters already differ; logs first 48 weekly delivery/native dates and CRT states.
- No game request outstanding. Diagnostic verify qbuild-20261024-065710-build-ctest running; session 63866 must be collected.
- weather-boot8-before-on-01 rejected (submitted before DLL existed); no questions filed.
- Do not read or edit tools/; coordinator must handle any regress profile update.
