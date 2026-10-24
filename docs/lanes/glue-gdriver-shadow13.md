| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-gdriver-shadow13: sampled DX7 transitions | `Codex` / `glue-gdriver-shadow13` (engine lane) | Ready, partial scope. cargo 7409 passed, clippy 0 diagnostics, CTest 81/81. Run 631 clean through 2032-01-15, ref_warnings=0; gdriver 72/72 exact, 35 transitions, 0 mismatches/refusals. No public ABI change or questions. | 1. Coordinator register the partial shadow and append gdriver_shadow to the current regress shadows profile. 2. Extend texture formats, device outputs and unexercised paths. |

## Changes

Commit 688e67255 adds the default-off `gdriver_shadow` observer, private static-link C transport and core transition comparator. The observer requires the specified GOG DX7 interface identity, acquires its driver reference, restores hooks before releasing it, and leaves native calls outside the observer mutex. It samples at most one transition per slot per 32-frame bucket and bounds mismatch details to eight lines. Configuration and build registration are additive. This resumed round validates that committed source without further engine edits.

Each replay starts from the captured native pre-state for that individual transition. This verifies sampled transitions, not construction-to-frame equivalence. Implemented comparisons cover clear values, row length, matrix-mode/projection cache, capability setters and selected queries, stage selection/environment/combine fields and viewport readback. Dirty/applied caches, texture creation/lock formats, general render-state/device transform outputs and complete lifecycle remain outside this measurement. No spec contract was guessed and no questions were needed.

## Gate

The single final-source `qbuild all` completed successfully. It automatically submitted native verification to the coordinator queue as `qbuild-20261024-204323-build-ctest`; no duplicate verify was launched.

- Cargo: **7409 passed, 0 failed, 0 ignored**, summed over every test-result line. All three shadow tests passed, including deliberately differing snapshots and viewport edge cases.
- Clippy with warnings denied: **exit 0, 0 diagnostics**.
- Native build: **PASS 265/265 edges**, 309.0 s.
- Full CTest: **PASS 81/81**, 401.9 s; native gate total 713.2 s.
- Explicit owned-file formatting: **2 files, 0 diff lines**. Diff check clean.
- Retained logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. Prior interrupted focused compile evidence remains in `build-runs/gdriver-focused-1.log`; it is not counted as a passing operation.

## Real-city measurement

[Run 631](../runs/gdriver13-r2-turbo-2032-20261024a-644.md): `gdriver13-r2-turbo-2032-20261024a`, Konradshohe, turbo, end date 2032-01-01, `gdriver_shadow=1`. Verdict **clean**, date **2032-01-15**, real_seconds **28.469**, **ref_warnings=0**, foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0. DLL SHA256 prefix `0cb3fd55`.

Measured summary: **`gdriver 72/72`**. Exact retained log lines:

```text
gdriver transitions 2032-01-01: samples=35 mismatches=0 refused=0 frames=109 scope=cached-transitions
gdriver: shutdown: 72/72 exact; samples=35 mismatches=0 refused=0 scope=cached-transitions prestate=per-transition
```

All 72 monthly lines report zero mismatches and zero refusals. Counts are cumulative; 72 monthly lines do not mean 72 distinct transition samples. The run exercised 35 transitions across 109 frames: slots 12=1, 13=1, 28=4, 40=4, 41=4, 44=4, 45=4, 51=4, 62=4, 74=1, 75=4. Hooked comparison paths 14, 26, 46, 48 and 54-57 were not naturally exercised and receive no measured credit. Slot 43 supplies the frame counter. No normal-speed follow-up was required because the turbo run had no differences.

Evidence is retained at `build-runs/game-results/gdriver13-r2-turbo-2032-20261024a.verdict` and the adjacent `.log`. No request remains outstanding.

## Integration and remaining work

Ready to land this measured partial observer. Keep it default-off. The coordinator must review/register its limited scope in coverage and append `gdriver_shadow` to the existing current regress shadows string if accepted. This clean-room lane did not open or edit `tools/`, merge main or change shared Git refs. No public C ABI header/exports or version changed; the comparator is private static-link transport.

Broader nSGLDX7 verification remains open: texture create/lock formats, device render-state and transform outputs, full lifecycle replay, and natural coverage of the unexercised hooks above. No coverage of those paths is claimed by this run.
