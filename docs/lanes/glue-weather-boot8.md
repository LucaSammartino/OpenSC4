| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-weather-boot8: timing-independent weather shadow bootstrap | Codex, `glue-weather-boot8` | in progress | Reproduce train183, fix native date stepping, validate to 2032 |

## Handoff
- Final measurement: weather-boot8-fixed-on-05 73/73; clean to 2032-01-01, ref_warnings=0; run record docs/runs/weather-boot8-fixed-on-05-442.md.
- Goal: weather shadow exact 73/73 through 2032 with scres_file_shadow on and off.
- Worktree reused, detached HEAD; initial tree clean.
- Baseline verify: build PASS 249/249, CTest PASS 78/78, logs in build-runs/fc4verify-{build,ctest}.log.
- Reproduced weather-boot8-before-on-02 2/27 (run 439): February exact steps=4; March mismatches=54102 byte=108. Clean ref_warnings=0, time limit ended 2028-03-24.
- Diagnostic verify PASS 47/47 build edges, CTest 78/78. Diagnostic run weather-boot8-diag-on-03 2/4 (run 440), clean/ref_warnings=0 to 2026-04-02, first difference step=7 byte=107. February steps=5 exact but still diverges, unlike the task's suggested cause.
- Diagnostic verify -065710 PASS 47/47 build edges, CTest 78/78; run weather-boot8-diag-on-04 2/4 (run 441), clean/ref_warnings=0 through 2026-04-02.
- Root cause measured: every delivered/native date matches; weekly BeginRand refuses DR7=0x55 (all SCRes probes armed), then independent iterator diverges. Not a startup date-step loss.
- Fix: SCRes lends its idle DR0 solely around weather's synchronous weekly call (weather 5.2: no parser calls, queued notifications); existing native observation saves/restores all DR0/DR6/DR7 state. Refuse active parser frames, foreign occupied DR0 or non-execution slot.
- Next: collect full all gate session 99595, then house-profile runs on/off to 2032 with seconds=1500. No date catch-up invented: measured native dates agree and extra steps would not replay native behavior.
- Diagnostic source logs first actual record difference even if CRT counters already differ; logs first 48 weekly delivery/native dates and CRT states.
- Full final gate PASS: Rust 6960/6960, clippy zero diagnostics; native build and CTest PASS (build-results/qbuild-20261024-070503-build-ctest.verdict). Session 99595 collected to terminal.
- Next: final house-profile run weather-boot8-fixed-on-05, then weather-boot8-fixed-off-06, both to 2032 with seconds=1500.
- weather-boot8-before-on-01 rejected (submitted before DLL existed); no questions filed.
- Do not read or edit tools/; coordinator must handle any regress profile update.
