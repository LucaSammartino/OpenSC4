| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dbpf-sample10: rotate packed DBPF shadow coverage | Codex / glue-dbpf-sample10 | ready: DBPF 73/73 exact to 2032, clean; Rust 6988, clippy clean, CTest 80/80 | Coordinator profile edit and matched-host runtime check; 800 s target unmet even with DBPF off; EX56 open |

## Implementation

Required prerequisite merge `c17f37d71` retained both additive header declaration groups. Rotation implementation `30cdc0adf` keeps the full load sweep and sorts paths before choosing ceil(files/12) whole files monthly. Any twelve consecutive months cover a stable file set. Summary lines retain the dated DBPF prefix and final mismatches count, and add subset positions, available/completed files, cumulative comparisons and twelve-month unique coverage.

Corrected source `3c2f8c290` retains the independent Rust reader and next record between framework ticks, yielding after roughly 800 ms. Both readers compare each record together; no native record object or cursor is retained. Autotest quit drains the final dated sample through framework ticks. Other shutdowns explicitly emit non-exact pending samples and release acquired QIs. The rotation test checks rolling years for pools of 1..257 files and all 72 house months. Default remains off.

## Gate

Final `qbuild.py all`: PASS, Rust **6988 passed, 0 failed, 0 ignored**, clippy **0 diagnostics**. Queued native verify `qbuild-20261024-092912-build-ctest`: build **50/50** (2.8 s), CTest **80/80** (74.8 s), total 82.8 s. Collected to terminal exit 0. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`, and the native job verdict under `build-results/`. Earlier source also passed its gate; its `090545` verdict is retained. No source changed after the final gate.

## Game results

`gdbs10-r3-house2032-20261024c 73/73`

[Run 471](../runs/gdbs10-r3-house2032-20261024c-485.md): **clean**, all DBPF lines mismatches=0/refusals=0, **ref_warnings=0**; all other problem counters zero. Train187 house switches plus DBPF, Konradshohe, turbo request end_date=2032-01-01. Quit completed 2032-01-04 after draining January's sample. **863.406 s**, 2.51 days/s, **880 s wall**. Full load: 11 files, 111826 records. Every one of six rotation years reached **11/11 files and 111826/111826 records**. Total **83 completed file comparisons, 876020 record/raw/payload comparisons**, 89.27% fewer records than predecessor 8163298. Maximum DBPF slice **953 ms**. Twenty-six slow-file entries totaled 79543 ms active time, max 4734 ms per file spread across ticks.

[Run 472](../runs/gdbs10-r4-housebase2032-20261024d-486.md), `gdbs10-r4-housebase2032-20261024d`: identical DLL and house switches, DBPF off; **clean**, **863.609 s**, 2.50 days/s, **879 s wall**, date 2032-01-01; all problem counters zero, DBPF sample lines zero. **The 800-second target is unmet even in this DBPF-off baseline.** Different slots/CPU affinity and simulation paths prevent a precise overhead estimate. Both runs satisfy the 1800-second ceiling; neither supports an under-800 claim. Coordinator should repeat a matched-host benchmark before claiming it.

Failed evidence remains separate: [R1/run 469](../runs/gdbs10-r1-house2032-20261024a-483.md) stalled at July 2026, 7/7 exact, whole-file reads took >5 s; this led to record slicing. [R2/run 470](../runs/gdbs10-r2-house2032-20261024b-484.md) stalled at February 2029, 37/37 exact; own PDB snapshot resolved CRT formatting and Pollution AfterStep, main CPU +0. R3 retried the unchanged gated DLL. All waits collected their original operation to terminal result. No operation remains outstanding. Unique log/verdict files are in `build-runs/game-results/`.

## Scope and integration

Existing **EX56** remains open in [data-formats questions](../specs/questions/data-formats.md); no new question. Scope remains strictly resource-manager-packed. No all-open-files, city-streaming, native record lifetime or ReadRecord return contract is claimed. The prerequisite merge adds three C exports (packed_count, packed_keys, read_raw); this lane never bumps ABI version.

The clean-room prohibition on opening any part of `tools/` prevents this lane from editing `tools/regress.ps1`. Coordinator: append `dbpf_shadow=1` to the current single shadows string after reviewing the measured result and runtime limitation; no profile/coverage-map file was read or edited here. Publish the detached HEAD to the reserved branch and credit the exact run. Further runtime investigation belongs to a matched-host house benchmark, since the DBPF-off baseline also exceeds 800 s.
