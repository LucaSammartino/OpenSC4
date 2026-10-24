| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-dbpf-close8: DBPF owner verification | Codex, `glue-dbpf-close8` | ready | Land measured default-off shadow; retain EX56; allow 1800 s for house regressions |

## Changes

Implemented answered EX54/EX55 in the Rust packed-file projection: raw-read return and in/out-size behavior, null-buffer queries, missing-key rejection, type-only DIR exclusion, key ordering/counts and empty-list Boolean behavior. Four focused tests exercise all 23 vectors. Added three C ABI exports; the ABI version is unchanged.

The default-off `dbpf_shadow=1` follows public resource-manager and multi-packed roots. It compares held game segments against independently opened Rust packages: indexes/key lists/counts, metadata, raw return/size/bytes, null queries, missing raw keys, logical payload bytes/size and untouched buffer tails.

Commit `4609b87fa` spreads dated samples across framework OnTick callbacks, outside simulator OnTick. Explicit state retains the date, acquired QIs, files, next-file index and counters. Both readers compare together per file; processing returns between files after at least 800 ms elapsed. Shutdown releases retained QIs and explicitly reports every unfinished sample. Sample dates identify root acquisition, with file comparisons on later ticks. Round 2 changed only documentation and request duration after gating this implementation.

## Gate

One `qbuild.py all` on the final engine source: **PASS**. Rust **6985 passed, 0 failed, 0 ignored**; clippy **0 diagnostics**; native build **49/49 edges**; full CTest **79/79**, 83.1 s. Native job `qbuild-20261024-074334-build-ctest` was collected to terminal PASS. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`, and the job's verdict under `build-runs/build-results/`. Later commits contain documentation only. No build or game operation remains outstanding.

Earlier compile failure evidence remains at `build-runs/dbpf-r2-failed-native-build.log`; its switch-local braces fix passed before the incremental change. Earlier stalled R1/R2 evidence remains in their existing run records and result files.

## Real-game measurements

Both round-2 requests copied the switches from the explicitly permitted main `build-runs/regress-train186-shadows.ini`, added `dbpf_shadow=1`, used Konradshohe and turbo, and targeted `2032-01-27`. The harness supplies the city path. Both used DLL SHA256 prefix `04a4cd9c`. Neither watchdog nor simulation clock implementation changed.

- [R3: gdbc8-r3-house2032-20261024b](../runs/gdbc8-r3-house2032-20261024b-474.md): **clean**, stopped by its 1200 s limit at **2031-01-06**, 1201.860 s. **dbpf 60/61 clean**. All 60 complete samples matched; the final January 2031 sample was explicitly unfinished (8/11 files, 107537 records, refusals=1/mismatches=1). Foreign exceptions, watchdog stalls, reference warnings, Rust panics and structured exceptions were zero. This exercised incomplete-sample accounting and QI cleanup; it earns no 2032 credit. Max logged file time 5125 ms.
- [R4: gdbc8-r4-house2032-20261024c](../runs/gdbc8-r4-house2032-20261024c-475.md): same DLL/switches, limit 1800 s, timeout 2100 s. **Clean at 2032-01-27**, **1585.844 s**, 1.38 days/s. **dbpf 73/73 clean**: load plus 72 monthly samples dated 2026-01-27 through 2032-01-01. Every sample: files/opened/indexes=11, records/raw/payload=111826, compressed=88769, empty=0, refusals=0, mismatches=0. **8163298** completed record comparisons. Max logged file time **4875 ms**. Foreign exceptions, watchdog stalls, RTC failures, overruns, reference warnings, Rust panics, structured exceptions and harness INCOMPLETE lines were all **zero**.

Full evidence: `build-runs/game-results/<label>.log` and `.verdict`. Unique labels were collected without qwait or duplicate requests. R3's non-clean line was an explicit deadline refusal; all compared record values matched. R4 completed every queued sample. Per-record splitting was unnecessary for this measured zero-stall result; file-level scheduling does not promise sub-800-ms callbacks on other hosts.

## Scope and questions

**EX56 remains open in [docs/specs/questions/data-formats.md](../specs/questions/data-formats.md).** Scope is strictly **resource-manager-packed**. Completeness for every open packed file, including city-save backing files, is unclaimed. Indexed pointers are treated as borrowed; only acquired QI references are released. Public `ReadRecord` noninterference/lifetime and exact return contracts remain unproved; only payload bytes and in/out size are compared. The clean run does not answer EX56.

Writes, saves, flush/delete/create/copy/compact, space allocation, compression setters, game segment open/close/Init/Shutdown, city streaming ABI and record-object cursor/lifetime methods remain uncompared. Only independent Rust handles are opened/closed; no lifecycle or write method is called on game-held segments.

## Next steps

Land the measured work with the shadow default-off. The coordinator can add `dbpf_shadow=1` to the house profile; allow at least **1800 s** for this six-year workload (measured 1585.844 s) and a larger harness timeout. Resolve EX56 before expanding coverage claims. The worktree remains detached as instructed; the supervisor publishes its actual committed HEAD to the reserved lane branch.
