| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-netrules-finish5: finish network rules shadow measurement | Codex / `glue-netrules-finish5` (engine lane) | READY measured work: source4a3d87e3b; cargo6696/0, clippy0 diagnostics, native198/198, CTest70/70; run351 clean to2032-02-01/ref0, netrules74/74 explicit exact and mismatch-clean, 84 evaluations/24 tables; no new questions; two inherited C ABI exports, none added this round | (1) Coordinator add netrules_shadow=1 to tools/regress.ps1 shadows profile (lane explicitly forbidden to read tools/); (2) review/credit run351 and land detached HEAD. |

## Change and scope

Round2 starts at 94de13664 with the prior shadow5 implementation already merged.
Commit4a3d87e3b changes only reporting in engine/plugin/FC4NetRulesShadow.cpp.
Before the first ready snapshot, Report emits `netrules status (...)` and returns
before incrementing comparison counters. This status contains neither the shadow
comparison prefix nor mismatch fields, so it does not count as a comparison.
After comparisons start, every report remains visible, including subsequent loss
of readiness. Replay, captures, table comparisons, exactness predicates, evaluation
counters and mismatch counters are unchanged. Default configuration remains off.

The reporting change adds no C ABI exports. The inherited lane work adds two;
the queue therefore retains abi=true. FC4_ABI_VERSION was not changed.
Spec conformance42/49 readiness semantics are unchanged. No new spec ambiguity or
semantic difference was found; no new numbered question was needed.

## Full gate

One stable-source `qbuild.py --wt <WT> all` reached terminal PASS:

- Rust: 6696 passed, 0 failed, 0 ignored.
- Clippy with warnings denied: 0 diagnostics.
- Native build: 198/198 edges; sccache180 hits/1 miss; build13.0s.
- CTest: 70/70 passed; 90.0s.

The same operation queued native build/CTest as
`qbuild-20261024-010203-build-ctest`, completed in107.1s after queue delay.
No duplicate all/verify/native request was launched. Logs remain in build-runs/:
qbuild-test.log, qbuild-clippy.log, fc4verify-build.log and fc4verify-ctest.log.
Only records changed after validation. The gate milestone is d79c6301c.

## Real-city evidence: run351

```
netrules-finish5-konrad-2032-20261024e 74/74
```

This is both explicit exactness and mismatch-clean count, independently checked
against every comparison in the full plugin log. All73 monthly comparisons and
shutdown have ready=1 and exact=1; none has ready=0. The initial load notification
is one separate status line with epoch0/evaluations0. Shutdown confirms74 lines,
74 exact. The final monthly comparison is:

```
netrules shadow (month) 2032-02-01: mismatches=0 table_mismatches=0 incomplete=0 unavailable=0 evaluations=84 new_evaluations=0 tables=24 epoch=1 ready=1 exact=1 intersections=unobserved
```

Verdict clean at2032-02-01 in242.484s: ref_warnings=0, foreign_exceptions=0,
structured exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, dialogs_closed=0,
Rust panics=0, resynced=0. All19 scripted actions executed, including the
January2032 mark and digest. Intersections remain explicitly unobserved; there
was no turbo-only mismatch requiring a normal-speed control.

The Konradshohe request is identical to run345 apart from its unique label and
rebuilt DLL. It uses the same38 providers, native netmgr/netlot/nettool/demolition,
and build-runs/netrules-finish-peripheral-actions.txt through2032-02-01.
DLL SHA256: 94e16e4afe82425af2bc7bc9cda06c411082a926d32ee81b7117cc59e71b9224.
The event-based waiter reached terminal exit0 without qwait or duplicate requests.

Record: docs/runs/netrules-finish5-konrad-2032-20261024e-351.md (df6a1a412).
Main and local docs/runs were scanned before assignment;348 was the maximum.
Request: build-runs/game-queue/done/netrules-finish5-konrad-2032-20261024e.req.
Verdict/full log: build-runs/game-results/ under that label. No operation remains
outstanding. Prior controls342-345 remain separate in their existing run records;
the prior round's detailed diagnostic report is retained at94de13664.

## Profile admission and coordinator action

The evidence now meets the requested every-counted-line exactness condition.
The lane did not open, read or edit tools/regress.ps1: the clean-room rules
explicitly forbid opening any part of tools/, while this task requests a change
there. A narrow exception was requested asynchronously; no answer was received.
The coordinator should add `netrules_shadow = 1` to the shadows profile using
run351 as evidence. Coverage tables were not edited; credit remains the
coordinator's action. This READY entry contains the measured engine/reporting
work, with the profile edit explicitly left for the coordinator.
