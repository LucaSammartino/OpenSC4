| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-gzfw-shadow24: GZ buffer and command differential | Codex; `.claude/worktrees/glue-gzfw-shadow24` | ready; measured subset | Coordinator profile/row review; remaining owners below |

Implemented default-off `gzfw_shadow`, private Rust static-link transports, monthly observer integration and focused tests.
Rows addressed: `cGZBuffer` and `cGZCommandServer`, within the bounded scopes below; this does not establish whole-owner coverage.
Buffer: five private native formats (2/8, 4/16, 5/16, 7/16, 9/32), varying padded widths, pre-init locks, zero/duplicate Init, geometry, stride, format, RGB, disabled key and QI outputs; 140 observations/month.
Commands: four spec-supplied registrations plus an unknown name; direct, uppercase, quoted, two-word and empty inputs; counted and NUL-terminated parse lengths; 75 results/month. No live registration or execution.
Monthly total: 215 comparisons, 15,480 across the run; no exclusions or resynchronization.
Exact result: `gzfw24-20261025-r1-2032 72/72` (every monthly mismatch count zero, 2026-02-01 through 2032-01-01).
Game: Konradshohe, turbo, clean through 2032-01-03; foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, ref_warnings=0; DLL SHA256 prefix `a9dce9c2`.
Full gate: workspace 7715 passed / 0 failed / 0 ignored; clippy zero diagnostics; native build 199/199 edges; CTest 100/100; qbuild PASS.
Focused tests: 2/2; each corrupted buffer observation is detected, and command one-word parsing witnesses pass. Explicit owned-file formatting passed.
Gate evidence: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`, `focused-gzfw-shadow.log`.
Game evidence/run record: `build-runs/game-results/gzfw24-20261025-r1-2032.verdict` and adjacent `.log`; unique request preserved by the queue.
ABI: no public DLL exports or ABI version changed; new functions are private static-library transports.
Still unmeasured: `cGZMessageQueueManager2`, `cGZCOM`, `cGZFramework`, `cGZFileSystem`, `cGZScriptTerp`, `nGZLog::cService`.
Still missing: queue dispatch/order, COM class/library routing, framework registry/lifecycle, filesystem resolution, script callbacks, logging routing, and broader buffer/command behavior outside these probes.
Unmeasured contracts include GZ11/GZ12 and GZF1-3/GZF6-9; no native signatures or hidden input state were guessed.
Questions filed: none; no specs edited, per the revised lane rules.
Coordinator: append `gzfw_shadow` to the existing shadows profile string and review the bounded `cGZBuffer`/`cGZCommandServer` row credit; coverage tables/profile remain untouched here.
