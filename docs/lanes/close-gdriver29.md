| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| close-gdriver29 / renderer-driver, engine | Codex / .claude/worktrees/close-gdriver29 / detached HEAD; branch reserved close-gdriver29 | continue, round time guard; code b4280f930, c09a497ca atop 0e6b755a6, e75bc6af8; ABI unchanged; cargo 7888/0; final gate pending; no method credit | 1. Collect existing gate. 2. Test startup selection. 3. Complete device/texture/surface/remaining slots and house/action 2032 runs. |

Gate: ONE final-source qbuild all is still running as exec session **61006**; collect that same operation, do not duplicate it unless fixing a failure.
Rust **7888 passed / 0 failed / 0 ignored**, including the new pure-forward/blit rejection test: build-runs/qbuild-test.log. Current clippy was checking fc4-capi; no final verdict. CMake/CTest unverified.
An earlier completed clippy log was stale and is NOT a final-source pass. Earlier focused compile failed on a missing import, fixed before all; session 72128 was interrupted to release its build lock.
Formatting: 2 owned Rust files, 0 diff lines; diff check passed. All engine edits committed before the guard stopped this round.
Changes: compare vertex scalar returns (6,8-10), unchanged Boolean/float output bytes (47,49), unsupported blit errors (68-73), and zero return (82); complete counts comparisons independently of mismatches.
Diagnosis reports active identity/device/stages/platform error once per city. Opt-in gdriver_shadow_select=1 plus gdriver_shadow=1 selects DX7 at ObserverStart/PostAppInit, before city resources; this final startup path is UNTESTED.
AC1: device stream is still partial: texture/surface creation/lock formats, primitive queue prestate, host memory and lifecycle effects remain missing. No whole-row credit.
AC2: 52/83 slots hooked; below is code scope, with **0 credited complete calls in a qualifying native 2032 run** in every group.
| Slots | Comparison scope | Qualified credit |
|---|---|---|
| 6,8-10,26,28,40,46-51,54-57,68-73,78,81-82 | full outputs modeled for supported sampled inputs | 0 |
| 44-45 | complete only for caps 5/7 | 0 |
| 11-23,27,41-43,52-53,62,67,74-76 | partial; Flush counted only | 0 |
| 0-5,7,24-25,29-39,58-61,63-66,77,79-80 | not hooked | 0 |
AC3: monthly method evidence retained; 13 new hooked methods use spec addresses. No qualifying native monthly run completed: do not flip method or nSGLDX7 row credit.
Logs: MAIN/build-runs/game-results/. gdriver29-r2-diagnose-2032-20261026a: clean, 2032-01-05, 36.672s, ref_warnings=0; log 1145 proves active software table 0x00ACC4D8, inactive DX7 device=0/stages=0. No DX7 transitions checked.
Intermediate gdriver29-r2-select-2032-20261026a (DLL e72f0e1c): exited -1 without verdict after IN-CITY selection created device=0x099C9D20/stages=8 on 2026-01-28. Final code removes that script syntax and selects before city resources; this is not final-source verification.
AC4: sampling stays one/slot/32 frames, 128 device calls. Active-DX7 <=1.5x timing NOT measured. No game request outstanding.
Questions: none filed; no attributable native/core difference yet. RD1/RD3/RD4 remain open. No ABI version/exports, coverage map, generated ABI entries or shared Git refs changed.
Next: collect session 61006 and final verdict; after rebuild, submit ONE shared-queue startup/action run with build-runs/gdriver29-r2-select-actions.txt and both shadow/select switches. Inspect streams/state/method counts, file a renderer question for unexplained differences. Finish remaining slots and zoom/rotate/screenshot/display probes, full house to 2032 and matched active-DX7 timing control.
