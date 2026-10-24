| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-mysimdispatch-shadow9: dispatch replay | Codex / `glue-mysimdispatch-shadow9` (engine, detached HEAD) | **READY for measured default-off scope.** Source `315da8cc7`; cargo 7017/0, clippy clean, CTest 80/80; house run 471 clean to 2032, dispatch 73/73 exact constructor/header samples. No public C ABI change (98). | Coordinator: land measured scope. MS21/MS22 remain open; live trip behavior and the remaining owner behavior are uncompared. |

## What changed

The core implements the specified dispatch decisions: 42 ordinary travel pairs, same-mode fallback, FE/FF handling, strict departure delay, stuck notices, retry returning false, deterministic models, constructor defaults, idle snap/recovery, callbacks 1/2/9 and mission exit. Synchronous notices expose the change counter in order; missing-stop failure uses the state setter; failed idle double-snap applies the common clear-2002 tail. Unsupported partition/skip decisions remain explicit rather than guessed.

The plugin adds `mysimdispatch_shadow`, off by default, using public factory/serialization interfaces. It predicts constructor state independently and compares 208 guaranteed serialized prefix bytes plus seven class/header/scalar values. Unpromised constructor fields and variable reference encoding are excluded. Live references, when present, permit class/writer identity comparison only. No native snapshot seeds an expected request, and no dispatch replacement is enabled. Both comparators are private static-link transports; no public C ABI export changed and ABI remains 98.

Milestones: spec merge `16354d1ef`; core `ffa8d7194`; core/shadow `140680449`, `abbe101a0`; ordering/recovery `f2a48d7e8`; final formatting `315da8cc7`; gate record `555181f2f`.

## Final verification

- Explicit owned-file formatting: eight Rust files, exit 0, zero differences.
- Full `qbuild.py --wt <WT> all` on stable final source `315da8cc7`, collected to terminal as session 80433: **PASS**. Workspace test totals summed over all result lines: **7017 passed, 0 failed, 0 ignored**; dispatch integration **39/39**. Clippy: **0 diagnostics**, warnings denied. Native build: **161/161**, 83.0 s. Full CTest: **80/80**, 152.9 s. No edits or commits occurred during this gate.
- Stale ABI smoke resolved: the prior train-integration object had compiled header 97 while linked Rust was 98. Current header/Rust are both 98. Final native build ran `gen_abi_entries.py` (185 interfaces, unchanged), recompiled and relinked `capi_smoke`, and its test passed. No generated file was edited and no ABI version was bumped.
- Earlier failed gates remain distinct in `build-runs/r2-failed-discovery/` and `build-runs/r2-failed-abi/` (CTest 79/80). Final logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`.
- Focused session 44572 could not be collected in round 3: runner returned `Unknown process id`. It was not resubmitted or claimed to pass. The completed final workspace gate includes the dispatch suite, 39/39.

## Real-game measurement

[House run 471](../runs/mysimdispatch-shadow9-r2-prefix208-2032-20261024a-496.md), label `mysimdispatch-shadow9-r2-prefix208-2032-20261024a`, was enqueued once after checking for an existing verdict. The prepared request ran in slot 2 with turbo, house shadows and `mysimdispatch_shadow = 1`. The existing wait operation (session 84462, no qwait) was collected to terminal. No request remains outstanding.

Verdict **clean**, date **2032-01-01**, **810.656 s** simulation wall time (830 s harness wall), **ref_warnings=0**. Foreign exceptions, watchdog stalls, RTC, overruns, Rust panics, structured exceptions and incomplete shadow lines were all zero. Exact verdict summary and shutdown line (without timestamp/pid prefix):

```text
mysimdispatch    73 lines, 73 with every mismatch count 0
mysimdispatch resynced lines: 0
mysimdispatch: measured 73/73 exact assigned-constructor/header samples; live trip owner remains unverified (MS20 input-boundary limitation)
```

All 73 samples had `constructor=1 live_headers=0 fields=215 errors=0 mismatches=0`. This independently verifies the promised constructor fields; it does not exercise live header comparison. Existing `mysims` shadow also reported 73/73 clean lines. Evidence: `build-runs/game-results/<label>.verdict` and `<label>.log`; final DLL SHA256 prefix `7b48476f`.

## Questions and uncompared scope

**MS21 and MS22 remain open** in [mysims questions](../specs/questions/mysims.md). MS21 requests exact route-index/partition/current-marker and network-snap rules; failed-leg skip and callback-5 progression remain unsupported. MS22 requests omitted auxiliary constructor defaults; those bytes remain excluded. No new question was filed in round 3.

**Uncompared:** native live queue, clock, waits, setters, requests, stop behavior, serialized trip state, route partitioning, callback 5, dispatch-tool lifecycle, unpromised constructor fields and variable reference encoding. No live dispatch headers were observed. MS20 establishes that the public dispatch IID has only nine slots and supplies no complete read-only observer for direct-call trip inputs. Whole-owner comparison requires a supported captured input boundary; no whole-owner equivalence is claimed.

Next: coordinator may land this measured default-off work; spec writers resolve MS21/MS22, and a future lane can add an independently supported live input boundary. The supervisor publishes this detached HEAD to the reserved lane branch; this lane did not modify shared refs, merge main or push.
