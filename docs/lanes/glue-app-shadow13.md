| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-app-shadow13 (application shadow) | Codex / `glue-app-shadow13` (engine) | ready, partial scope; cargo 7407/0/0; clippy PASS; ctest 81/81; runs 631-634 clean; final app 72/72, app-startup 1/1; AP17/AP18 open | 1. Integrate and append app_shadow to current regress profile. 2. Resolve AP17/AP18 and extend observation contracts. |

## Report

Added default-off `app_shadow` using a persistent Rust application model and native read-only observations. The supported scope is 17 fields: eight stored-path getters, six lazy user folders, two filename getters, and message-derived game mode. Stored inputs are captured once; the model receives the four specified mode messages and never resynchronizes from comparison outputs. Monthly observations use `app`; the first dated city quiet tick uses `app-startup` (2026-01-27 in the successful runs).

Source milestones: `5ee6bc317` introduced the observer and private transport; `ae3cc0a3c` registered the ini key; `f9e8cee74` added expected/input diagnostics and fixed startup date scheduling; `d0cad6574` separated the unresolved caches. No subsequent engine source changes.

AP17 leaves startup command-line/INI replay, pause/speed, native frame/tick counters and idle/message-pump dispatch decisions open because independent capture contracts are missing. AP18 records the slot65 Exception Reports cache retaining the ordinary Documents base despite the captured slot58 base pointing at the harness slot, and slot66 returning `Scripts\TestScripts\` rather than the literal model's `Scripts\`. The literal core rules remain unchanged. These two fields are excluded from supported labels and available only under additional default-off `app_shadow_path_cache=1`, reported as `app-path-cache`. This is partial verification; no whole-owner cSC4App credit is claimed.

Seven private static-link C transport symbols were added. The ABI flag is set for those C-linkage additions; public `fc4.h`, the DLL export contract and `FC4_ABI_VERSION` are unchanged.

## Validation

Final stable-source `qbuild.py --wt <WT> all` passed with `RUST_TEST_THREADS=2`: cargo **7407 passed, 0 failed, 0 ignored**, summed over 51 result lines; clippy **0 diagnostics** with `-D warnings`; native **96/96 build edges**, **81/81 CTests**. Native queue label: `qbuild-20261024-210908-build-ctest`; the full operation was collected to exit=0. Formatting passed for the owned `capi/src/app_shadow.rs`.

Evidence: `build-runs/app-closure-gate-console.log`, `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`, and `app-fmt-final.log`. Earlier interrupted/diagnostic evidence remains in `qbuild-test-transport-interrupted.log`, `app-recovery-gate/`, `app-diagnostic-gate/`, and `app-final-gate-console.log`. The earlier q.py attempt failed in Git Bash before Cargo; its log remains `build-runs/q/20261024-184945-32108.log`. Recovery and diagnostic full gates also passed before the final stable-source gate.

## Real-city measurements

- [Run 631](../runs/app-shadow13-2032-r2a-631.md), `app-shadow13-2032-r2a`: clean turbo to 2032-01-05, app 0/72 and app-startup 0/1. Exactly the two AP18 caches differed; the initial startup label also used an unset date, subsequently fixed.
- [Run 632](../runs/app-shadow13-2032-r2normal-632.md), `app-shadow13-2032-r2normal`: clean normal speed to 2032-01-01, **app 72/72**, **app-startup 1/1**. Explicit AP18 diagnostic **app-path-cache 0/73**, confirming the same two differences without turbo.
- [Run 633](../runs/app-shadow13-2032-r2exact-633.md), `app-shadow13-2032-r2exact`: clean turbo to 2032-01-20, **app 72/72**, **app-startup 1/1**, unresolved diagnostic off.
- [Run 634](../runs/app-shadow13-2032-r2final-634.md), `app-shadow13-2032-r2final`: final-gate DLL clean turbo to 2032-01-06, **app 72/72**, **app-startup 1/1**, every supported shadow line mismatches=0 and resyncs=0. DLL SHA256 `8c0ae2a45ff8fedab6e841d0b8740441a317d21b3a27ed78029d9996da802867`.

All four verdicts are clean with `ref_warnings=0`, foreign exceptions=0, watchdog stalls=0 and rtc=0. No game or build request remains outstanding. The exact measured labels for the final DLL are:

```text
app 72/72
app-startup 1/1
```

## Questions and integration

AP17 and AP18 are in [questions/application.md](../specs/questions/application.md). All unresolved measured differences belong to AP18; its diagnostic stays default-off. No analysis sources, tools/ files, game binaries or game assets were read or committed.

The coordinator should integrate the measured partial scope, append `app_shadow` to the existing single regress shadows string after integration, regenerate the question index, and review coverage credit for these labels. This lane did not edit tools/ or coverage maps, merge main, move refs or publish; the reserved worktree uses detached HEAD. Extend the shadow after AP17/AP18 receive observation contracts and path-cache semantics.
