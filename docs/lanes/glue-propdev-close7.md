| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-propdev-close7 (prop developer verification) | Codex / `glue-propdev-close7` | in progress | Apply PV16/PV17; gate and measure native shadow through 2032 |

## Handoff
- Goal: compare every native prop developer operation, monthly `propdev shadow` summaries, exact through 2032.
- Done: initial handoff `276e570b4`; PV16 scoped native slot-3 night observer, same-thread CRT pre-state/draw-count/final-state capture, independent ordered-list/callback-output comparison, restoration at city shutdown. Additive CRT C ABI; ABI version unchanged.
- Gate: focused observation tests 2/2 passed. Native `qbuild-20261024-063016-build-ctest` PASS: 249/249 edges, CTest 78/78, terminal session 77289. Queue says tree changed: only question/wait-helper files changed while native source remained stable.
- Game milestone: `gpc7-pv16-normal-2032-a` clean to 2032-01-01, foreign_exceptions/ref_warnings/rtc=0; source e5edd7566, DLL 489fe5e3, normal 146.891s. Exact coverage `propdev 0/72`, 0 resyncs; 24197/24197 selector events and 45465/45465 complete streams. Run record 434. Night callbacks=0: not exercised.
- Added final requester-tag projection comparison and ineligibility reporting for days without block work; do not promote these projections to full creation-effect exactness.
- Gate milestone: first `all` terminal PASS (session 26152): Rust 6939/0, clippy 0 diagnostics, native 50/50 edges, CTest 78/78, request `qbuild-20261024-064127-build-ctest`. Native source stable; report/run doc commits explain tree-changed note.
- Added bounded `clockmode 0|1|2` script action through borrowed city clock slot 86, clock slots 29/5, parser range/error tests, additive engine README documentation. `build-runs/propdev-night-actions.txt` selects mode 0 and enables native transition notifications on 2026-01-28.
- Clock action source `all` session 32462 TERMINAL FAIL: Rust 6939/0, clippy clean, native 55/55 edges, CTest 77/78; sole failure plugin_ini_script_test's stale unknown-action list at plugin_load_test.cpp:9488. Unique verdict `qbuild-20261024-064433-build-ctest.verdict`; complete failed CTest log preserved by rename to build-runs/fc4verify-ctest-clock-golden-failed.log.
- Corrected the native integration test's expected action-list string additively. Corrected-source `all` session 96482 is running; functions exec cell 70 collects it to terminal using 60s write_stdin waits. Collect cell 70, never launch a duplicate check.
- Next: collect final gate to actual terminal, then enqueue unique `gpc7-clock-normal-2032-b` normal-speed 2032 request with `script=build-runs/propdev-night-actions.txt`, `enable=simulator`, `propdev_shadow=1`; wait via wait-file.ps1, no qwait. Analyze with build-runs/analyze-propdev.py. No game request outstanding.
- Report unverified families explicitly: creation registration writes, hold/holder/tag callback ordering, dated timer/state/date advancement, occupant RNG, queued notification lifetime/order and model writes/releases. Night scope still unexercised until a run counts callbacks >0.
- PV18 filed for hidden dated-record snapshot/model-call contract; creation/dated operations remain explicitly ineligible, full-operations-exact=0.
- New owned header: engine/plugin/FC4PropDevNight.h. One dated monthly label now `propdev shadow YYYY-MM-DD`.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-glue-propdev-close7-r1-manifest.json`.
- Never inspect tools/ or analysis material; tools/regress.ps1 change requires coordinator follow-up under clean-room restriction.
