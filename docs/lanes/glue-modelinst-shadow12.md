| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-modelinst-shadow12 (MI14 fixture fix and live destination observer) | `Codex` / `glue-modelinst-shadow12` (engine lane) | ready: merged shadow11 2b8dc5bcf; engine source 939c4bfa5; cargo 7375/0, clippy clean, CTest 81/81; run 520 clean static 73/73 to 2032-01-02; run 521 clean 145/145 (static 73/73, live 72/72) to 2032-01-01; ref_warnings=0; ABI unchanged; no new questions | (1) Review scoped coverage and append modelinst_shadow to existing regression shadows profile. (2) Extend unmeasured operations separately; no whole-owner credit. |

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
