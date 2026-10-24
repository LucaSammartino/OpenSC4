| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-modelinst-shadow12 (MI14 fixture fix and live destination observer) | `Codex` / `glue-modelinst-shadow12` (engine lane) | in progress: b5f3bb118 merge passed cargo 7408/0, clippy clean, build 62/62, CTest 81/81 and exact 145/145 to 2032; origin/main advanced during gate to 625733fc6, adding conflicts at both registration sites | (1) Preserve new view-utility and model-instance registrations. (2) Gate changed merged inputs and confirm in game. (3) Refresh READY full tip. |

## Handoff

- Prior tested merge a907efa2d, report commit 63285ec8d: all gate PASS; modelinst-mi14-merge-2032-r3 145/145 clean to 2032-01-10.
- Diagnostic: final ancestry check found origin/main advanced to 625733fc6 during the long gate; diff adds viewutil_shadow at both exact conflict sites. Merge this advancement to avoid another skipped tip.
- Run 632 was concurrently taken by train206; this lane's r3 record is now run 640 (free on current main).
- Merge 95e87ebc114834e11af350d88dd8a0ead67276ce of 625733fc6 resolves five additive conflicts: CMake, capi module, ini key/docs, observer shutdown. Both viewutil and modelinst preserved.
- Final all PASS on stable engine source 95e87ebc1: session 40602 collected to terminal, output build-runs/modelinst-r4-final-all.log. Prior gate full logs archived in build-runs/modelinst-r3-gate-evidence/.
- Cargo 7,466 passed / 0 failed / 0 ignored, 51 summaries; clippy exit=0, diagnostics=0. Native qbuild-20261024-215641-build-ctest build 110/110 (190.7 s), CTest 81/81 (273.4 s), total 466.3 s.
- Next: one confirmation modelinst-mi14-merge-2032-r4; DLL SHA256 0e7a5d9eb4af327798a12637112d5af9a7303134dffe93a97bea285702169e28. Runs 640/641 were taken concurrently: use free 650 for r3 and 651 for r4 after checking current main. No new owners/questions; no public ABI changes.

## Current-main refresh

The skipped READY tip `25e148fcdf7722d1ffabbaa69a950a6acb3f5f93` is refreshed by merge `a907efa2d77f809ad96645dba321ab661ef5f06f` of current `origin/main` (`b5f3bb118b66601e8898bfb4545716c287cc27f3`). Resolved both additive conflicts: `engine/capi/src/lib.rs` retains construction_shadow and modelinst_shadow, and `FC4Config.cpp` retains all main ini keys plus default-off modelinst_shadow at the end. No new owner, spec question, public ABI export, or coverage claim.

Final stable-source `qbuild.py all`: **PASS**, session 53110 collected to terminal. Cargo **7,408 passed / 0 failed / 0 ignored**, 51 summaries; clippy **exit=0, diagnostics=0**. Queued native `qbuild-20261024-201508-build-ctest`: build **62/62 PASS** (192.7 s), CTest **81/81 PASS** (261.8 s), total 456.7 s. Logs: `build-runs/modelinst-r3-final-all.log`, `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. Engine source stayed at a907efa2d throughout; the tree-change annotation reflects lane-record commits only. No duplicate gate or native verify was launched.

[Run 640](../runs/modelinst-mi14-merge-2032-r3-640.md): **`modelinst-mi14-merge-2032-r3 145/145`**, comprising **73/73 (modelinst)** and **72/72 (modelinst-live)**. Every dated line has mismatches=0 and resyncs=0. Clean to **2032-01-10** in 27.953 s, ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0. Live observer ON: 90,579 natural calls encountered, 4,096 captured, 86,483 dropped; 12,288 unique counter/padding bytes compared. First live line captured 1,427 calls / 4,281 fields. DLL SHA256 `8aa62a855f8d2d8f6994b976b46e2e4a509b5dafda1784f0896663b8c855607f`.

Konradshohe, turbo, target 2032-01-01, default replacements and only modelinst_shadow=1, matching the previous exact run. Evidence: `build-runs/game-results/modelinst-mi14-merge-2032-r3.verdict` and `.log`; bounded counter `build-runs/modelinst-summary.py`. The single blocking file-watcher wait (session 9863) was collected to terminal; no request or check remains outstanding. No normal-speed rerun is required because no shadow differs. Coverage remains scoped as described below. Final queue entry uses the actual detached HEAD's full 40-character hash; the supervisor publishes it to the reserved lane branch.

## Change and load diagnosis

Merged shadow11 in `2b8dc5bcf`, preserving main's lighting integration. Its diff has no model-instance native calls during plugin initialization or pre-city shadow registration: only default-off ini-key registration precedes city load. The fixture starts in the city-loaded notification. The earlier menu-only observations do not establish a particular pre-load native call as their cause.

MI14 supplied two concrete fixture ABI corrections (`e8703c22d`): the standalone constructor at `0x00802950` takes construction flag **1** and returns the receiver; its instance interface is at allocation **+0xA0**, while +0 is the mesh container. Omitting the argument violates the constructor's four-byte callee-pop contract. Cleanup uses the receiver-only nondeleting body on the allocation start, never Release or pool deletion of foreign storage. The first corrected-DLL request loaded the city and reached 2032 cleanly.

MI14 was read from `re-modelinst-mi14:docs/specs/model-instances.md` and its answered-question file. No new question or duplicate open MI14 was filed. Land that answer branch alongside this implementation if it is not already landed.

## Scoped live observer

Engine source `939c4bfa5` observes the published static instance vtable's slot 9, preserving its receiver-adjusting original thunk. Each natural game call is forwarded exactly once. It copies the caller's 56-byte destination before and after the call and compares **only** the destination counter increment and preservation of two padding bytes against the production Rust core. The baseline is independent of the native output. Source words/kind are unmeasured.

The first 4,096 eligible calls per city are sampled, an explicit MI14 instrumentation choice, with encountered/captured/dropped counts. Only copied values and aggregate counts survive the call; no instance pointer is retained. There are no added native getter/mutator/refcount calls or live bounds-cache refreshes. Fixture calls are excluded from live counts. Counters use a mutex, enable is atomic, and the slot is restored at shutdown. The final run logged observer ON and no installation/restoration refusal. Both shadows remain default-off under `modelinst_shadow`.

## Measured game results

- [Run 520](../runs/modelinst-mi14-loadfix-2032-r1-520.md): **`modelinst-mi14-loadfix-2032-r1 73/73`**, static fixture exact, clean to **2032-01-02** in 32.015 s; ref_warnings=0. DLL prefix `fa30f030`.
- [Run 521](../runs/modelinst-mi14-live-2032-r2-521.md): **`modelinst-mi14-live-2032-r2 145/145`**, clean to **2032-01-01** in 30.125 s; ref_warnings=0, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0. DLL SHA256 `0cc2798c7d79336bf890d7d88ca5eb6a9636bd71ea5a7539bb6907f3aef71d2a`.
- Run 521 families: **`modelinst-mi14-live-2032-r2 73/73 (modelinst)`** and **`modelinst-mi14-live-2032-r2 72/72 (modelinst-live)`**. Every dated line has mismatches=0 and resyncs=0. Static: 144 fields/line, 10,512 comparisons. Live: 93,481 encountered, 4,096 captured, 89,385 dropped, 12,288 unique compared bytes. First live line: 1,426 calls / 4,278 fields on 2026-02-01, so the result is nonempty.

Both runs used Konradshohe, turbo, target 2032-01-01, default replacement switches and only `modelinst_shadow=1`. There were no turbo-only differences requiring a normal-speed run. Evidence: `build-runs/game-results/<label>.verdict` and `.log`. The current harness summary lists the static family; dated `modelinst-live` lines are independently counted from the same log by `build-runs/modelinst-summary.py`. No request or check remains outstanding.

## Gates and evidence

- Focused Rust tests **2/2 PASS**, including counter wrap and padding-damage detection. Owned-file fmt and `git diff --check`: PASS. Log: `build-runs/modelinst-focused.log`.
- Required final stable-source `qbuild.py all`: terminal **PASS**, session 36931 collected. Workspace **7,375 passed / 0 failed / 0 ignored**, all 48 summaries; clippy `-D warnings` **PASS**, diagnostics=0.
- Queued native `qbuild-20261024-182011-build-ctest`: build **58/58 PASS**, 58.3 s; CTest **81/81 PASS**, 157.6 s; total 220 s. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. The tree-change annotation reflects lane-record commits only: `git diff 939c4bfa5 HEAD -- engine` was empty. No source changed after the all gate started.
- Initial corrected-fixture native verify also passed: `qbuild-20261024-155017-build-ctest`, build 263/263, CTest 81/81. Final all followed the real live-observer source change.
- Preserved transport evidence: q.py Bash failure `build-runs/q/20261024-161127-6204.log`; relative fmt failed to locate the file, absolute-path fmt passed. Focused cargo took 34m43s and passed 2 tests; PowerShell's exit 1 came from stderr progress becoming NativeCommandError. The first file-watcher wait timed out despite a completed valid verdict; that verdict was retained without repeating the game. Final blocking wait completed normally. All asynchronous operations were collected to terminal results.

## Coverage limit and next steps

This is partial observed-operation coverage. Static constructor/default/transform/finite bounds fixtures and live static GetTransform destination counter/padding are measured. Live source words/kind, bounds, draw attributes, model/material binding, basic/animated owners, processor outputs and sprite resources are unmeasured. No complete-owner verification or replacement is claimed. Extend instrumentation through MI14's synchronous existing-call contracts, rather than monthly enumeration or additional live bounds queries. No new spec question blocks these measured channels.

No public C ABI export/version changed. No coverage maps were edited. `tools/` was not opened or edited under the clean-room prohibition: the coordinator must append `modelinst_shadow` to the existing single regression `shadows = ...` string after reviewing the exact run, rather than adding a second string. Credit only the scoped line families above; extend other operations in a subsequent lane.
