| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-viewutil-shadow11: view utility shadow | `Codex` / `glue-viewutil-shadow11` (engine lane) | in progress: merge current main and repeat gate; prior source 16bdc1dc8, ABI unchanged; run 510 clean, viewutil 73/73 exact, resyncs=0; VU16 filed. | 1. Preserve additive registrations while merging main. 2. Run final gate once. 3. Refresh ready queue with full HEAD. |

## Handoff

- Goal: repair the merge-train conflict only; no new owners, questions or game runs.
- Done: prior measured rotation-only work and report committed at 76760254f.
- Next: merge main d1f26eca7, preserve both additive module registrations, run qbuild all, commit report and refresh queue.
- Facts: run 510 (`viewutil11-r1-on2-20261024-1424`) remains valid, clean 73/73; partial scope 1/50, VU16 blocks other bodies; no outstanding operations.

## Changes and scope

Commit 16bdc1dc8 adds default-off `viewutil_shadow`, observed at city load and SimNewMonth, with one dated summary per observation. It compares native RotateVectorByViewRotation to the production Rust core through a private static-link word adapter. Configuration, observer hooks and build entries are additive. No public C ABI exports or ABI version changed.

Each observation compares 156 calls and 469 fields: seven fixed edge-case vectors and five vectors from live city view/camera state, each with twelve fixed rotations and the live rotation, plus a required-live-state/binding check. Float words are compared bit-exactly, including y, signed zeros, quiet NaN payloads, infinities, finite extrema and subnormals. Negative and extreme signed rotations are included. The shadow changes only local vectors, takes no native state writes and performs no resyncs. Missing live state is a mismatch rather than vacuous success.

This measures **1/50 family entries**, the explicit cdecl rotation helper (78/19748 family bytes). It does **not** cover the whole SC4ViewUtilities owner, MinOfFourFloats, MaxOfFourFloats, or the other 47 owner bodies. New question **VU16** in `docs/specs/questions/view-utilities.md` requests callable native signatures, supported bindings and safe live/isolated call contracts. Those entries remain uncalled; no native signature was guessed. Signaling NaNs remain excluded under existing VU12's numeric-mode question.

## Validation

Final source unchanged since 16bdc1dc8. Required `qbuild all` completed PASS: workspace tests **7141 passed, 0 failed, 0 ignored** across 45 result lines; clippy `-D warnings` clean, no diagnostics. Queued native gate `qbuild-20261024-131027-build-ctest` PASS: build 262/262 edges, **CTest 81/81**, zero failures. Explicit-file formatting check passed. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. Prior failed wrapper launch and canceled diagnostic operation are retained; neither was counted as a test pass. All operations reached terminal results; no build, game request or waiter remains outstanding.

Required measured line:

```text
viewutil11-r1-on2-20261024-1424 73/73
```

Run record: `docs/runs/viewutil11-r1-on2-20261024-1424-510.md`. House switches plus `viewutil_shadow=1`, Konradshohe, turbo through 2032-01-01. Clean verdict at 2032-01-03 after 1023.437 seconds. Every load/month row has `live=1 samples=156 fields=469 resyncs=0 mismatches=0`; final shutdown confirms 73/73 exact. Reference warnings, foreign/structured exceptions, watchdog stalls, Rust panics and other warnings all zero. DLL SHA256: `59bd57d0d0479e20a0ceb8e20fd53a5e498b74c26149d9cb629ce4152db4af11`.

All 52 existing house summary keys remain exact; enabled summaries have 53 total keys including viewutil. Key sets match earlier enabled and default-off runs. Comparison is over common observed periods: the default-off control failed early, so a clean full-period baseline was not established. No stronger claim of unchanged individual lines is made.

Earlier failures remain separate: run 508 (`viewutil11-r1-on-20261024-1255`) hit the short deadline and stalled during quit, with 59/59 exact; run 509 (`viewutil11-r1-off-20261024-1336`) stalled outside FreeCity4 in 2027, with no viewutil lines, confirming default-off behavior. The clean completed retry is preserved without rerunning it.

## Remaining work

Measured rotation-only work is ready. VU16 blocks the remaining 49 entries; resolve its native ABI and safety contracts before extending comparisons. Coordinator should review partial scope before assigning coverage credit, regenerate the question index, and handle any regression-profile addition. This clean-room lane did not read or edit `tools/`, merge main or change shared refs; the request to modify `tools/regress.ps1` is left to the coordinator under those restrictions.
